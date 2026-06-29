//! Shared normalize helpers: FR3 ingest-time FMV resolution, §6.2 `source_ref` synthesis, wallet
//! construction, and `Unclassified` raw capture. Used by every parser so the policy is identical.
use crate::read::RawRow;
use btctax_core::price::fmv_of;
use btctax_core::{FmvStatus, PriceProvider, Sat, Source, SourceRef, TaxDate, Usd, WalletId};
use std::collections::HashMap;

/// FR3: prefer the export's own USD (`ExchangeProvided`); else the bundled daily-close dataset
/// (`PriceDataset`); else `Missing` (a hard blocker in core). `sat` is taken in magnitude.
pub fn resolve_fmv(
    export_usd: Option<Usd>,
    date: TaxDate,
    sat: Sat,
    prices: &dyn PriceProvider,
) -> (Option<Usd>, FmvStatus) {
    if let Some(u) = export_usd {
        return (Some(u), FmvStatus::ExchangeProvided);
    }
    match fmv_of(prices, date, sat.abs()) {
        Some(u) => (Some(u), FmvStatus::PriceDataset),
        None => (None, FmvStatus::Missing),
    }
}

/// (source, direction)-scoping for `source_ref` (§6.2) and the semantic key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
    Trade,
}
impl Direction {
    pub fn tag(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
            Direction::Trade => "trade",
        }
    }
}

/// Mints stable `SourceRef`s (§6.2). Native-id rows pass the id through, direction-scoped. Id-less rows
/// (River; Gemini transfer rows lacking a `Trade ID`) get the semantic key `dir|utc_ms|type|sat` plus a
/// deterministic per-key `occurrence_index` to disambiguate identical rows in file order (the file-order
/// fragility is the documented §6.2 / FOLLOWUPS limitation).
#[derive(Debug, Default)]
pub struct SourceRefMint {
    seen: HashMap<String, u32>,
}
impl SourceRefMint {
    pub fn native(&self, dir: Direction, id: &str) -> SourceRef {
        SourceRef::new(format!("{}|{}", dir.tag(), id))
    }
    pub fn semantic(&mut self, dir: Direction, utc_ms: i64, type_tag: &str, sat: Sat) -> SourceRef {
        let key = format!("{}|{}|{}|{}", dir.tag(), utc_ms, type_tag, sat);
        let occ = self.seen.entry(key.clone()).or_insert(0);
        let r = SourceRef::new(format!("{key}#{occ}"));
        *occ += 1;
        r
    }
}

/// Single-account exchange wallet for a source (multi-account is future — FOLLOWUPS).
pub fn exchange_wallet(source: Source) -> WalletId {
    WalletId::Exchange {
        provider: source.tag().to_string(),
        account: "default".to_string(),
    }
}

/// Deterministic raw capture for an `Unclassified` event (sorted keys via `BTreeMap`).
pub fn raw_of(row: &RawRow) -> String {
    row.cells
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::price::StaticPrices;
    use btctax_core::FmvStatus;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn prices() -> StaticPrices {
        let mut m = std::collections::BTreeMap::new();
        m.insert(date!(2025 - 03 - 01), dec!(84000.00));
        StaticPrices(m)
    }

    #[test]
    fn fr3_prefers_export_then_dataset_then_missing() {
        let p = prices();
        // export present → ExchangeProvided (verbatim value, dataset ignored)
        let (v, s) = resolve_fmv(Some(dec!(123.45)), date!(2025 - 03 - 01), 50_000_000, &p);
        assert_eq!((v, s), (Some(dec!(123.45)), FmvStatus::ExchangeProvided));
        // no export, dataset hit → PriceDataset (0.5 BTC @ 84000 = 42000.00)
        let (v, s) = resolve_fmv(None, date!(2025 - 03 - 01), 50_000_000, &p);
        assert_eq!((v, s), (Some(dec!(42000.00)), FmvStatus::PriceDataset));
        // no export, dataset miss → Missing
        let (v, s) = resolve_fmv(None, date!(2025 - 06 - 15), 50_000_000, &p);
        assert_eq!((v, s), (None, FmvStatus::Missing));
    }

    #[test]
    fn native_source_ref_is_direction_scoped_id() {
        let mint = SourceRefMint::default();
        assert_eq!(mint.native(Direction::Out, "TX-9").0, "out|TX-9");
    }

    #[test]
    fn semantic_source_ref_disambiguates_identical_rows_by_occurrence() {
        let mut mint = SourceRefMint::default();
        let a = mint.semantic(Direction::In, 1_700_000_000_000, "income", 1000);
        let b = mint.semantic(Direction::In, 1_700_000_000_000, "income", 1000); // identical row
        assert_eq!(a.0, "in|1700000000000|income|1000#0");
        assert_eq!(b.0, "in|1700000000000|income|1000#1"); // occurrence_index increments
        assert_ne!(a, b);
    }
}
