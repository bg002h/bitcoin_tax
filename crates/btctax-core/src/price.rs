use crate::conventions::{round_cents, Sat, TaxDate, Usd, SATS_PER_BTC};

/// Daily-close BTC/USD provider (§9.2). Pure & deterministic; the projection borrows `&dyn PriceProvider`
/// so identical (events, prices) → identical ledger (NFR4). The bundled dataset lives in btctax-adapters.
pub trait PriceProvider {
    /// USD per WHOLE BTC at the daily close for `date`, or None if unknown.
    fn usd_per_btc(&self, date: TaxDate) -> Option<Usd>;
}

/// FMV (USD, cents) of `sat` satoshis at `date`, if a price exists.
/// §7.1 totality (M6): **checked** Decimal ops — an overflow yields `None` (treated as missing FMV → the
/// `fmv_missing` gating path), never a panic.
pub fn fmv_of(prices: &dyn PriceProvider, date: TaxDate, sat: Sat) -> Option<Usd> {
    let px = prices.usd_per_btc(date)?;
    px.checked_mul(Usd::from(sat))
        .and_then(|x| x.checked_div(Usd::from(SATS_PER_BTC)))
        .map(round_cents)
}

/// Test/CLI stub: an explicit date→price map (deterministic).
#[derive(Debug, Default, Clone)]
pub struct StaticPrices(pub std::collections::BTreeMap<TaxDate, Usd>);
impl PriceProvider for StaticPrices {
    fn usd_per_btc(&self, date: TaxDate) -> Option<Usd> {
        self.0.get(&date).copied()
    }
}
