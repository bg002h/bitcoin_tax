//! Parse CLI references back into core types. The primary case is the canonical `EventId` string the
//! engine prints (`EventId::canonical()`): `import|<src>|<source_ref…>`, `conflict|<src>|<source_ref…>|<fp>`,
//! `decision|<seq>`. `source_ref` itself may contain `|` (adapters mint direction-scoped refs like
//! `out|cb-send`), so import rejoins parts[2..] and conflict takes the LAST part as the fingerprint.
use crate::CliError;
use btctax_core::{
    EventId, Fingerprint, IncomeKind, LotId, LotPick, Source, SourceRef, TaxDate, Usd, WalletId,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use time::macros::format_description;
use time::Date;

fn source_of(tag: &str) -> Option<Source> {
    match tag {
        "swan" => Some(Source::Swan),
        "coinbase" => Some(Source::Coinbase),
        "gemini" => Some(Source::Gemini),
        "river" => Some(Source::River),
        _ => None,
    }
}

pub fn parse_event_id(s: &str) -> Result<EventId, CliError> {
    let bad = || CliError::BadEventRef(s.to_string());
    let parts: Vec<&str> = s.split('|').collect();
    match parts.first().copied() {
        Some("import") => {
            if parts.len() < 3 {
                return Err(bad());
            }
            let source = source_of(parts[1]).ok_or_else(bad)?;
            let source_ref = parts[2..].join("|"); // may contain '|'
            Ok(EventId::import(source, SourceRef::new(source_ref)))
        }
        Some("conflict") => {
            if parts.len() < 4 {
                return Err(bad());
            }
            let source = source_of(parts[1]).ok_or_else(bad)?;
            let fp = Fingerprint(parts[parts.len() - 1].to_string()); // fingerprint is the last segment
            let source_ref = parts[2..parts.len() - 1].join("|");
            Ok(EventId::conflict(source, SourceRef::new(source_ref), &fp))
        }
        Some("decision") => {
            if parts.len() != 2 {
                return Err(bad());
            }
            let seq = parts[1].parse::<u64>().map_err(|_| bad())?;
            Ok(EventId::decision(seq))
        }
        _ => Err(bad()),
    }
}

/// `exchange:PROVIDER:ACCOUNT` | `self:LABEL`.
pub fn parse_wallet_id(s: &str) -> Result<WalletId, CliError> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    match parts.as_slice() {
        ["exchange", provider, account] if !provider.is_empty() && !account.is_empty() => {
            Ok(WalletId::Exchange {
                provider: (*provider).to_string(),
                account: (*account).to_string(),
            })
        }
        ["self", label] if !label.is_empty() => Ok(WalletId::SelfCustody {
            label: (*label).to_string(),
        }),
        _ => Err(CliError::Usage(format!(
            "bad wallet {s:?}; use exchange:PROVIDER:ACCOUNT or self:LABEL"
        ))),
    }
}

/// Exact USD (NFR5): string → Decimal, never float.
pub fn parse_usd_arg(s: &str) -> Result<Usd, CliError> {
    Decimal::from_str(s.trim()).map_err(|e| CliError::Usage(format!("bad USD {s:?}: {e}")))
}

pub fn parse_date_arg(s: &str) -> Result<TaxDate, CliError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s.trim(), &fmt).map_err(|e| CliError::Usage(format!("bad date {s:?}: {e}")))
}

/// `<event-id-canonical>#<split-sequence>` → `LotId`.
///
/// Uses `rsplit_once('#')` so that a source_ref containing '#' (e.g. Conflict fingerprint or a
/// semantic key like `in|1234|credit|1#0`) is handled correctly: the split-sequence suffix is
/// always the LAST `#`-separated segment, and `EventId::canonical()` uses `|` as its separator
/// (never `#`), so peeling from the rightmost `#` is unambiguous.
pub fn parse_lot_id(s: &str) -> Result<LotId, CliError> {
    let (origin, split) = s
        .rsplit_once('#')
        .ok_or_else(|| CliError::Usage(format!("bad lot id {s:?}; expected <event-id>#<split>")))?;
    let origin_event_id = parse_event_id(origin)?;
    let split_sequence = split
        .trim()
        .parse::<u32>()
        .map_err(|_| CliError::Usage(format!("bad split_sequence in lot id {s:?}")))?;
    Ok(LotId {
        origin_event_id,
        split_sequence,
    })
}

/// `<lot-id>:<sat>` → `LotPick`.
///
/// Uses `rsplit_once(':')` so that the `<lot-id>` portion (which uses `|` and `#` as separators,
/// never `:`) stays intact. The trailing `:<sat>` is always unambiguous.
pub fn parse_lot_pick(s: &str) -> Result<LotPick, CliError> {
    let (lot_str, sat_str) = s
        .rsplit_once(':')
        .ok_or_else(|| CliError::Usage(format!("bad --from {s:?}; expected <lot-id>:<sat>")))?;
    let lot = parse_lot_id(lot_str)?;
    let sat = sat_str
        .trim()
        .parse::<i64>()
        .map_err(|e| CliError::Usage(format!("bad sat in --from {s:?}: {e}")))?;
    Ok(LotPick { lot, sat })
}

pub fn parse_income_kind(s: &str) -> Result<IncomeKind, CliError> {
    match s.to_ascii_lowercase().as_str() {
        "mining" => Ok(IncomeKind::Mining),
        "staking" => Ok(IncomeKind::Staking),
        "interest" => Ok(IncomeKind::Interest),
        "airdrop" => Ok(IncomeKind::Airdrop),
        "reward" => Ok(IncomeKind::Reward),
        _ => Err(CliError::Usage(format!("bad income kind {s:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{EventId, Fingerprint, IncomeKind, LotId, Source, SourceRef, WalletId};
    use rust_decimal_macros::dec;
    use time::macros::date;

    #[test]
    fn import_eventref_round_trips_even_with_pipe_in_source_ref() {
        // Adapters mint direction-scoped source_refs that CONTAIN '|' (e.g. "out|cb-send").
        let id = EventId::import(Source::Coinbase, SourceRef::new("out|cb-send"));
        let s = id.canonical(); // "import|coinbase|out|cb-send"
        assert_eq!(parse_event_id(&s).unwrap(), id);
    }

    #[test]
    fn decision_and_conflict_eventrefs_round_trip() {
        let d = EventId::decision(7);
        assert_eq!(parse_event_id(&d.canonical()).unwrap(), d);

        let fp = Fingerprint::of_bytes(b"x");
        let c = EventId::conflict(Source::Gemini, SourceRef::new("in|99|credit|1#0"), &fp);
        assert_eq!(parse_event_id(&c.canonical()).unwrap(), c);
    }

    #[test]
    fn bad_eventref_is_a_typed_error() {
        assert!(matches!(
            parse_event_id("garbage"),
            Err(crate::CliError::BadEventRef(_))
        ));
        assert!(matches!(
            parse_event_id("import|nosuchsource|x"),
            Err(crate::CliError::BadEventRef(_))
        ));
    }

    #[test]
    fn wallet_usd_date_kind_parsers() {
        assert_eq!(
            parse_wallet_id("exchange:coinbase:main").unwrap(),
            WalletId::Exchange {
                provider: "coinbase".into(),
                account: "main".into()
            }
        );
        assert_eq!(
            parse_wallet_id("self:cold").unwrap(),
            WalletId::SelfCustody {
                label: "cold".into()
            }
        );
        assert_eq!(parse_usd_arg("1234.56").unwrap(), dec!(1234.56));
        assert_eq!(parse_date_arg("2025-01-01").unwrap(), date!(2025 - 01 - 01));
        assert_eq!(parse_income_kind("interest").unwrap(), IncomeKind::Interest);
        assert!(parse_wallet_id("bogus").is_err());
    }

    #[test]
    fn parse_lot_id_round_trips_all_three_origin_variants() {
        // Import origin (with a '|' in source_ref — adapters mint direction-scoped refs)
        let l_imp = LotId {
            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("out|cb-send")),
            split_sequence: 3,
        };
        let s = format!(
            "{}#{}",
            l_imp.origin_event_id.canonical(),
            l_imp.split_sequence
        );
        assert_eq!(parse_lot_id(&s).unwrap(), l_imp);

        // Decision origin (Path-B seed lots: origin = allocation Decision id, resolve.rs:570-574)
        let l_dec = LotId {
            origin_event_id: EventId::decision(7),
            split_sequence: 1,
        };
        assert_eq!(
            parse_lot_id(&format!("{}#1", EventId::decision(7).canonical())).unwrap(),
            l_dec
        );

        // Conflict origin
        let fp = Fingerprint::of_bytes(b"x");
        let cid = EventId::conflict(Source::Gemini, SourceRef::new("in|99|credit"), &fp);
        let l_con = LotId {
            origin_event_id: cid.clone(),
            split_sequence: 0,
        };
        assert_eq!(
            parse_lot_id(&format!("{}#0", cid.canonical())).unwrap(),
            l_con
        );

        // N1: a source_ref containing '#' must still round-trip — rsplit_once('#') splits on the
        // LAST '#' (the split-sequence suffix is always last); locks the rsplit choice
        // (cf. eventref.rs decision_and_conflict_eventrefs_round_trip: "in|99|credit|1#0").
        let l_hash = LotId {
            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("in|99|credit|1#0")),
            split_sequence: 2,
        };
        assert_eq!(
            parse_lot_id(&format!(
                "{}#{}",
                l_hash.origin_event_id.canonical(),
                l_hash.split_sequence
            ))
            .unwrap(),
            l_hash
        );
    }

    #[test]
    fn parse_lot_pick_splits_trailing_sat() {
        let pick = parse_lot_pick("import|coinbase|X#0:25000").unwrap();
        assert_eq!(
            pick.lot,
            LotId {
                origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("X")),
                split_sequence: 0,
            }
        );
        assert_eq!(pick.sat, 25_000);
    }

    #[test]
    fn parse_lot_id_rejects_missing_hash() {
        assert!(matches!(
            parse_lot_id("import|coinbase|X"),
            Err(crate::CliError::Usage(_))
        ));
    }

    #[test]
    fn parse_lot_pick_rejects_missing_colon() {
        assert!(matches!(
            parse_lot_pick("import|coinbase|X#0"),
            Err(crate::CliError::Usage(_))
        ));
    }
}
