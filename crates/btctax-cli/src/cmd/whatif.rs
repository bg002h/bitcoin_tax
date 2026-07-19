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
    let (events, state, cfg) = s.load_events_and_project()?;
    let year = at.year();
    let prices = s.prices();
    let tables = BundledTaxTables::load();

    // Profile: ad-hoc (non-persisted) when supplied; otherwise resolve (ReturnInputs-derived → stored → …),
    // fail-closed screened — the same source `report`/`optimize` use, so plans match the filed number.
    let (profile, magi_caveat) = match &adhoc {
        Some(a) => (Some(adhoc_profile(a)), a.magi.is_none()),
        None => (s.resolve_screened_profile(&state, year, &tables)?, false),
    };

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
///
/// A thin wrapper over the single source of truth, [`HarvestTarget`]'s `FromStr` in `btctax-core`
/// (shared with the TUI panel). The `Display` of the core error reproduces the historical `--target`
/// messages; we only re-wrap it in the same `CliError::Usage` variant this parser has always used, so
/// CLI error output stays stable.
pub fn parse_harvest_target(s: &str) -> Result<HarvestTarget, CliError> {
    s.parse::<HarvestTarget>()
        .map_err(|e| CliError::Usage(e.to_string()))
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
    let (events, state, cfg) = s.load_events_and_project()?;
    let year = at.year();
    let prices = s.prices();
    let tables = BundledTaxTables::load();

    let (profile, magi_caveat) = match &adhoc {
        Some(a) => (Some(adhoc_profile(a)), a.magi.is_none()),
        None => (s.resolve_screened_profile(&state, year, &tables)?, false),
    };

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
        WhatIfError::NoLots {
            wallet,
            at,
            available,
            requested,
        } => CliError::Usage(crate::render::no_lots_message(
            &wallet, at, available, requested,
        )),
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

    /// `--target` parses the four forms; `$`/commas are optional; an unrecognized form / bad amount is
    /// rejected. (Negatives are NOT rejected here — the pure lexer passes `gain=-1` through and the
    /// ENGINE refuses it as `InvalidTarget`; see `cmd_and_panel_share_fromstr` + the core KAT.)
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

    /// The CLI `--target` parse is now a thin wrapper over the shared core `HarvestTarget: FromStr`
    /// (the same parser the TUI panel calls — dedup, task #48). Prove the wrapper delegates: for every
    /// representative form the cmd parse's Ok value matches `s.parse::<HarvestTarget>()` and they agree
    /// on success/failure — including the negative the lexer passes through (NOT a parse error) and the
    /// `gain=1_000` underscore case (`Gain(1000)`, parity with the legacy lexer). (The cmd re-wraps any
    /// Err in `CliError::Usage`, which adds a `usage: ` prefix — so we compare the Ok VALUE + the
    /// err-ness, not the CLI-decorated message.) The panel shares the identical `from_str` (see
    /// `whatif_panel::parse_harvest_target`); KAT-E10 keeps the panel free of `cmd::` tokens.
    #[test]
    fn cmd_and_panel_share_fromstr() {
        for s in [
            "zero-ltcg",
            "FIFTEEN-LTCG",
            "gain=$25,000",
            "gain=1000",
            "tax=$0",
            "tax=1500.50",
            "gain=-1",    // the lexer passes negatives through; the engine refuses them
            "gain=1_000", // `_` accepted by rust_decimal → Gain(1000), parity with the legacy lexer
            "nonsense",
            "gain=abc",
        ] {
            let via_cmd = parse_harvest_target(s);
            let via_core = s.parse::<HarvestTarget>();
            assert_eq!(
                via_cmd.as_ref().ok(),
                via_core.as_ref().ok(),
                "cmd wrapper must yield FromStr's Ok value: {s:?}"
            );
            assert_eq!(
                via_cmd.is_err(),
                via_core.is_err(),
                "cmd wrapper must agree with FromStr on failure: {s:?}"
            );
        }
    }

    /// UX-P4-9: an INSUFFICIENT pool (lots exist but do not cover the sale) names the available
    /// balance, the wallet, the as-of date, and the requested amount — NOT the false "no lots
    /// available" wording (which reads as an empty wallet).
    #[test]
    fn no_lots_message_insufficient_names_available_wallet_date_requested() {
        use time::macros::date;
        let err = map_whatif_err(WhatIfError::NoLots {
            wallet: WalletId::SelfCustody {
                label: "cold".into(),
            },
            at: date!(2025 - 08 - 01),
            available: 50_000_000, // 0.5 BTC held
            requested: 60_000_000, // 0.6 BTC requested
        });
        let msg = err.to_string();
        assert!(msg.contains("only 0.50000000 BTC available"), "{msg}");
        assert!(msg.contains("self:cold"), "names the wallet: {msg}");
        assert!(msg.contains("2025-08-01"), "names the as-of date: {msg}");
        assert!(
            msg.contains("(requested 0.60000000 BTC)"),
            "names the request: {msg}"
        );
        assert!(
            !msg.contains("no lots available"),
            "the false 'no lots available' wording is gone when lots DO exist: {msg}"
        );
    }

    /// UX-P4-9: a GENUINELY EMPTY pool (available == 0) says "no BTC available" — the honest
    /// zero case, distinct from mere insufficiency, and never "only 0 BTC".
    #[test]
    fn no_lots_message_empty_pool_says_no_btc() {
        use time::macros::date;
        let err = map_whatif_err(WhatIfError::NoLots {
            wallet: WalletId::SelfCustody {
                label: "hot".into(),
            },
            at: date!(2025 - 08 - 01),
            available: 0,
            requested: 60_000_000,
        });
        let msg = err.to_string();
        assert!(
            msg.contains("no BTC available in self:hot as of 2025-08-01"),
            "empty pool names the wallet + date: {msg}"
        );
        assert!(
            !msg.contains("only"),
            "an empty pool must not say 'only <X>': {msg}"
        );
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
