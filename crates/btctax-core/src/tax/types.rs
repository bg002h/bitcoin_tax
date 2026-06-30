use crate::conventions::Usd;
use serde::{Deserialize, Serialize};

/// IRS filing status (§1(a)–(d), §2(b), §2(a)). Governs both the ordinary-bracket schedule and
/// the §1(h) LTCG breakpoints (indexed, per-year) and the §1411 NIIT threshold and §1211(b) loss
/// limit (statutory, year-independent). `Qss` (Qualifying Surviving Spouse / §2(a)) uses the MFJ
/// schedule and thresholds for all rate lookups (§1(h)/§1/§1411 — MFJ treatment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FilingStatus {
    Single,
    Mfj,
    Mfs,
    HoH,
    Qss,
}

/// §1212(b) capital-loss carryforward, split by character. Magnitudes are always ≥ 0 (a carried
/// loss is stored as its positive amount; the sign is implied by the role). `Default` yields
/// zero (no carryforward). Used both for `carryforward_in` (profile field) and `carryforward_out`
/// (TaxResult field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Carryforward {
    pub short: Usd, // §1212(b)(1)(A): short-term character loss carryforward (≥ 0)
    pub long: Usd,  // §1212(b)(1)(B): long-term character loss carryforward (≥ 0)
}

/// Per-year tax context supplied by the user. Excludes all app-computed crypto items (B.1) so
/// that B can compute the incremental delta (I5). Persisted as JSON by the CLI side-table (Task 8);
/// `#[serde(default)]` on optional fields lets older/minimal stored profiles deserialize cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxProfile {
    pub filing_status: FilingStatus,
    /// Ordinary taxable income EXCLUDING all app-computed crypto items (mining/staking/etc. and
    /// net ST gains). This is the "base" income before B adds crypto on top (B.1 / I5).
    pub ordinary_taxable_income: Usd,
    /// Modified AGI excluding crypto, for the §1411 NIIT threshold comparison (B.1).
    pub magi_excluding_crypto: Usd,
    /// Qualified dividends + other preferential-rate income that shares the §1(h) 0/15/20 stack
    /// with net LTCG (B.1 / I9).
    pub qualified_dividends_and_other_pref_income: Usd,
    /// Non-crypto net LT-character capital gain already in the profile (optional; defaults to 0).
    /// B includes this in the §1222 LT stack when it is non-zero.
    #[serde(default)]
    pub other_net_capital_gain: Usd,
    /// Prior-year §1212(b) carryforward into this tax year, by character (optional; defaults to
    /// zero). The user is responsible for consistency with the prior year's `TaxResult` (M4 advisory
    /// is surfaced in Task 10).
    #[serde(default)]
    pub capital_loss_carryforward_in: Carryforward,
}

/// The marginal rates that apply to the user given their profile + the year's tax table. Reported
/// for informational purposes alongside the `TaxResult`. Not serde — internal to the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginalRates {
    pub ordinary: Usd, // marginal ordinary rate (Decimal, e.g. dec!(0.22))
    pub ltcg: Usd,     // marginal LTCG rate (0 / 0.15 / 0.20; Decimal)
    /// `true` when the crypto items *increased* NIIT (`niit_with > niit_without`).
    ///
    /// This is an **incremental** signal, not a raw MAGI-over-threshold flag: it is `false`
    /// when the taxpayer already pays NIIT without crypto and crypto does not raise NIIT further
    /// (e.g. when crypto adds only ordinary income while NII stays pinned by unchanged QD and
    /// MAGI is over the threshold both with and without crypto). Display-only — this field feeds
    /// no tax figure or delta. The NIIT delta itself is always `TaxResult::niit`.
    pub niit_applies: bool,
}

/// The computed result for a single tax year. All `Usd` fields are exact `Decimal`.
///
/// Delta fields (marked DELTA) = `with_crypto − without_crypto` (the incremental objective I5).
/// Level fields (marked level) = the WITH-crypto absolute value (e.g. carryforward for next year).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxResult {
    /// §1222 within-character net short-term WITH crypto (signed: gain > 0, loss < 0).
    pub st_net: Usd,
    /// §1222 within-character net long-term WITH crypto (signed).
    pub lt_net: Usd,
    /// Σ crypto ordinary income (mining/staking/etc.) recognized in the year. Added to the ordinary
    /// stack exactly once (double-count guard KAT in Task 5/7).
    pub ordinary_from_crypto: Usd,
    /// Crypto-attributable preferential-rate (§1(h)) tax (DELTA: with − without).
    pub ltcg_tax: Usd,
    /// Crypto-attributable §1411 NIIT (DELTA: with − without).
    pub niit: Usd,
    /// §1211(b) capital-loss ordinary offset actually used this year WITH crypto (level; ≥ 0).
    pub loss_deduction: Usd,
    /// §1212(b) carryforward out WITH crypto (level; feeds next year's `capital_loss_carryforward_in`).
    pub carryforward_out: Carryforward,
    /// THE objective (DELTA: with − without): `total_federal_tax_attributable =
    /// ordinary_delta + ltcg_tax + niit`. A wrong number must never be presented as authoritative
    /// (B.4 / I6 — hard blockers anywhere block this computation).
    pub total_federal_tax_attributable: Usd,
    pub marginal_rates: MarginalRates,
}

/// The outcome of a `compute_tax_year` call. `Computed` carries the full result; `NotComputable`
/// carries a `Blocker` whose `kind` is one of `{TaxYearNotComputable, TaxTableMissing,
/// TaxProfileMissing}` (all `Severity::Hard`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaxOutcome {
    Computed(TaxResult),
    NotComputable(crate::state::Blocker),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn tax_profile_serde_round_trips() {
        let p = TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: dec!(120000.00),
            magi_excluding_crypto: dec!(130000.00),
            qualified_dividends_and_other_pref_income: dec!(0.00),
            other_net_capital_gain: dec!(0.00),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(0.00),
                long: dec!(0.00),
            },
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: TaxProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn optional_profile_fields_default_to_zero() {
        // Older/minimal stored profiles omit the optional fields → serde-default to ZERO.
        let json = r#"{"filing_status":"Single","ordinary_taxable_income":"50000",
                       "magi_excluding_crypto":"50000","qualified_dividends_and_other_pref_income":"0"}"#;
        let p: TaxProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.other_net_capital_gain, Usd::ZERO);
        assert_eq!(p.capital_loss_carryforward_in, Carryforward::default());
    }
}
