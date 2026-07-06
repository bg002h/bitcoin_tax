//! `what-if sell` (task #43) — §43 READ-ONLY hypothetical-sale tax planning. Opens the vault, runs the
//! pure deterministic projection, and calls `btctax_core::whatif::sell` with the synthetic sale.
//! Appends NOTHING, writes NO side-table row, calls no `session.save()` (the vault is byte-identical
//! after — the `whatif_never_persists` KAT). Tax decision-support (consequences), NOT buy/sell advice.
//!
//! **Ad-hoc profile.** With `--filing-status` + `--income` (+ optional `--magi`/`--carryforward-in`)
//! the plan runs against a NON-persisted `TaxProfile` so a user can plan without `tax-profile set`;
//! with no ad-hoc flags it falls back to the stored `Session::tax_profile(year)`. **[R0-M4] `--magi`
//! defaults to `--income`** (a floor — NEVER $0, which would silently suppress every NIIT disclosure);
//! the caller prints the caveat when the default is used.
use crate::{CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::whatif::{
    self, HarvestReport, HarvestRequest, HarvestTarget, SellMethod, SellReport, SellRequest,
};
use btctax_core::{
    Carryforward, EvaluateError, FilingStatus, LotMethod, TaxDate, TaxProfile, Usd, WalletId,
    WhatIfError,
};
use btctax_store::Passphrase;
use std::path::Path;
use std::str::FromStr;

/// A validated ad-hoc (non-persisted) profile spec built from the `what-if sell` flags. `filing_status`
/// + `income` are mandatory to enter ad-hoc mode; `magi` is optional (defaults to `income`).
#[derive(Debug, Clone)]
pub struct AdhocProfile {
    pub filing_status: FilingStatus,
    pub income: Usd,
    pub magi: Option<Usd>,
    pub cf_long: Usd,
}

/// Build a NON-persisted `TaxProfile` from the ad-hoc spec (mirrors `placeholder_tax_profile`, but
/// user-supplied). **[R0-M4]** `magi_excluding_crypto` defaults to `income` when `--magi` is omitted —
/// NEVER $0 (a $0 floor would silently suppress every NIIT disclosure).
pub fn adhoc_profile(a: &AdhocProfile) -> TaxProfile {
    TaxProfile {
        filing_status: a.filing_status,
        ordinary_taxable_income: a.income,
        magi_excluding_crypto: a.magi.unwrap_or(a.income), // [R0-M4] floor = income, never $0
        qualified_dividends_and_other_pref_income: Usd::ZERO,
        other_net_capital_gain: Usd::ZERO,
        capital_loss_carryforward_in: Carryforward {
            short: Usd::ZERO,
            long: a.cf_long,
        },
        w2_ss_wages: Usd::ZERO,
        w2_medicare_wages: Usd::ZERO,
        schedule_c_expenses: Usd::ZERO,
    }
}

/// The outcome of `what-if sell`: the report + whether the ad-hoc `--magi` defaulted to `--income`
/// (so the renderer/caller can print the "MAGI assumed = ordinary income" caveat).
#[derive(Debug)]
pub struct SellOutcome {
    pub report: SellReport,
    pub magi_caveat: bool,
}

/// `what-if sell` — READ-ONLY hypothetical sale. Resolves the profile (ad-hoc if supplied, else the
/// stored one for the sale year), builds the `SellRequest`, and calls the core engine. No `save()`.
#[allow(clippy::too_many_arguments)]
pub fn sell(
    vault: &Path,
    pp: &Passphrase,
    sell_sat: i64,
    wallet: WalletId,
    at: TaxDate,
    price: Option<Usd>,
    method: Option<LotMethod>,
    adhoc: Option<AdhocProfile>,
) -> Result<SellOutcome, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, _state, cfg) = s.load_events_and_project()?;
    let year = at.year();

    // Profile: ad-hoc (non-persisted) when supplied; otherwise the stored profile for the year.
    let (profile, magi_caveat) = match &adhoc {
        Some(a) => (Some(adhoc_profile(a)), a.magi.is_none()),
        None => (s.tax_profile(year)?, false),
    };

    let prices = s.prices();
    let tables = BundledTaxTables::load();
    let req = SellRequest {
        sell_sat,
        wallet,
        at,
        price,
        method: method.map(SellMethod::Method),
    };
    // whatif::sell is READ-ONLY (clone-fold-discard); no save() is ever called.
    let report = whatif::sell(&events, prices, &cfg, profile.as_ref(), &tables, &req)
        .map_err(map_whatif_err)?;
    Ok(SellOutcome {
        report,
        magi_caveat,
    })
}

/// The outcome of `what-if harvest`: the report + whether `--magi` defaulted to `--income`.
#[derive(Debug)]
pub struct HarvestOutcome {
    pub report: HarvestReport,
    pub magi_caveat: bool,
}

/// Parse `--target`: `zero-ltcg` | `fifteen-ltcg` | `gain=$X` | `tax=$X` (X >= 0; `$`/commas optional).
pub fn parse_harvest_target(s: &str) -> Result<HarvestTarget, CliError> {
    let lower = s.trim().to_ascii_lowercase();
    match lower.as_str() {
        "zero-ltcg" | "zero_ltcg" | "zeroltcg" => return Ok(HarvestTarget::ZeroLtcg),
        "fifteen-ltcg" | "fifteen_ltcg" | "fifteenltcg" => return Ok(HarvestTarget::FifteenLtcg),
        _ => {}
    }
    if let Some(v) = lower.strip_prefix("gain=") {
        return Ok(HarvestTarget::Gain(parse_target_amount(v)?));
    }
    if let Some(v) = lower.strip_prefix("tax=") {
        return Ok(HarvestTarget::Tax(parse_target_amount(v)?));
    }
    Err(CliError::Usage(format!(
        "bad --target {s:?}: expected zero-ltcg | fifteen-ltcg | gain=$X | tax=$X"
    )))
}
fn parse_target_amount(v: &str) -> Result<Usd, CliError> {
    let cleaned = v.trim().replace(['$', ','], "");
    Usd::from_str(&cleaned).map_err(|e| {
        CliError::Usage(format!(
            "bad --target amount {v:?}: expected a USD number: {e}"
        ))
    })
}

/// `what-if harvest` — READ-ONLY harvest optimizer. Resolves the profile (ad-hoc if supplied, else the
/// stored one for the harvest year), builds the `HarvestRequest`, and calls the core segment-walk
/// optimizer. No `save()`.
#[allow(clippy::too_many_arguments)]
pub fn harvest(
    vault: &Path,
    pp: &Passphrase,
    wallet: WalletId,
    at: TaxDate,
    price: Option<Usd>,
    target: HarvestTarget,
    adhoc: Option<AdhocProfile>,
) -> Result<HarvestOutcome, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, _state, cfg) = s.load_events_and_project()?;
    let year = at.year();

    let (profile, magi_caveat) = match &adhoc {
        Some(a) => (Some(adhoc_profile(a)), a.magi.is_none()),
        None => (s.tax_profile(year)?, false),
    };

    let prices = s.prices();
    let tables = BundledTaxTables::load();
    let req = HarvestRequest {
        wallet,
        at,
        price,
        target,
    };
    // whatif::harvest is READ-ONLY (clone-fold-discard); no save() is ever called.
    let report = whatif::harvest(&events, prices, &cfg, profile.as_ref(), &tables, &req)
        .map_err(map_whatif_err)?;
    Ok(HarvestOutcome {
        report,
        magi_caveat,
    })
}

/// Map a `WhatIfError` to a user-facing `CliError` (mirrors `cmd::optimize::map_opt_err`).
pub fn map_whatif_err(e: WhatIfError) -> CliError {
    match e {
        WhatIfError::YearNotComputable(b) => CliError::Usage(format!(
            "year not computable — resolve the blocker or set a tax profile first: [{:?}] {}",
            b.kind, b.detail
        )),
        WhatIfError::PreTransitionYear(y) => CliError::Usage(format!(
            "{y} is pre-2025: a pre-2025 sale restates a closed year — not a plan"
        )),
        WhatIfError::NoLots => {
            CliError::Usage("no lots available to sell from that wallet as of that date".into())
        }
        WhatIfError::Evaluate(EvaluateError::ProceedsRequired) => CliError::Usage(
            "--price <usd-per-btc> is required for a future/off-dataset date with no bundled price"
                .into(),
        ),
        WhatIfError::Evaluate(ev) => CliError::Usage(format!("evaluate error: {ev:?}")),
        WhatIfError::InvalidTarget(msg) => CliError::Usage(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// [R0-M4] `--magi` defaults to `--income` (NEVER $0) when omitted; supplying it wins.
    #[test]
    fn adhoc_magi_defaults_to_income() {
        let defaulted = adhoc_profile(&AdhocProfile {
            filing_status: FilingStatus::Single,
            income: dec!(100000),
            magi: None,
            cf_long: Usd::ZERO,
        });
        assert_eq!(
            defaulted.magi_excluding_crypto,
            dec!(100000),
            "MAGI must FLOOR at income, never $0"
        );
        assert_ne!(defaulted.magi_excluding_crypto, Usd::ZERO);

        let explicit = adhoc_profile(&AdhocProfile {
            filing_status: FilingStatus::Single,
            income: dec!(100000),
            magi: Some(dec!(130000)),
            cf_long: Usd::ZERO,
        });
        assert_eq!(explicit.magi_excluding_crypto, dec!(130000));
    }

    /// `--target` parses the four forms; `$`/commas are optional; a bad form / negative is rejected.
    #[test]
    fn parse_harvest_target_forms() {
        assert_eq!(
            parse_harvest_target("zero-ltcg").unwrap(),
            HarvestTarget::ZeroLtcg
        );
        assert_eq!(
            parse_harvest_target("FIFTEEN-LTCG").unwrap(),
            HarvestTarget::FifteenLtcg
        );
        assert_eq!(
            parse_harvest_target("gain=$25,000").unwrap(),
            HarvestTarget::Gain(dec!(25000))
        );
        assert_eq!(
            parse_harvest_target("tax=$0").unwrap(),
            HarvestTarget::Tax(dec!(0))
        );
        assert_eq!(
            parse_harvest_target("tax=1500.50").unwrap(),
            HarvestTarget::Tax(dec!(1500.50))
        );
        assert!(parse_harvest_target("nonsense").is_err());
        assert!(parse_harvest_target("gain=abc").is_err());
    }

    /// The ad-hoc long-term carryforward-in flows into the profile (short stays $0).
    #[test]
    fn adhoc_carryforward_in_is_long_term() {
        let p = adhoc_profile(&AdhocProfile {
            filing_status: FilingStatus::Mfj,
            income: dec!(80000),
            magi: Some(dec!(90000)),
            cf_long: dec!(25000),
        });
        assert_eq!(p.capital_loss_carryforward_in.long, dec!(25000));
        assert_eq!(p.capital_loss_carryforward_in.short, Usd::ZERO);
    }
}
