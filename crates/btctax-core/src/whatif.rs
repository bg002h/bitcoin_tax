//! btctax-core::whatif — READ-ONLY hypothetical-transaction tax planning (task #43, phase P1).
//!
//! Posit a NON-persisted transaction and read its MARGINAL federal tax effect on the current-year
//! position. Everything routes through the single audited tax engine (`compute_tax_year`); this module
//! invents NO tax authority. It reuses the proven clone-fold-discard synthetic-disposal seam
//! (`optimize::synthetic_state`) — the vault is NEVER written (KAT `whatif_never_persists`).
//!
//! **Marginal identity.** Every figure is `withhyp − baseline` where BOTH are full `TaxResult`s from
//! `compute_tax_year` (baseline on the UNMODIFIED timeline). Because `total_federal_tax_attributable`
//! is itself a `with_crypto − without_crypto` delta and the "without crypto" term is identical in the
//! two scenarios (same profile, same real events), the subtraction cancels it exactly — the marginal is
//! the hypothetical's own effect, not the whole-year figure (the bug `optimize consult` used to have).
//!
//! **Refusals** are inherited from the engine verbatim: a Hard blocker anywhere / a missing table /
//! a missing profile ⇒ `YearNotComputable`; a pre-2025 date ⇒ `PreTransitionYear`; a future/off-dataset
//! date with no `--price` ⇒ `Evaluate(ProceedsRequired)`; an empty as-of pool ⇒ `NoLots`.
use crate::conventions::{round_cents, Sat, TaxDate, Usd, SATS_PER_BTC, TRANSITION_DATE};
use crate::event::{DisposeKind, LedgerEvent, LotPick};
use crate::identity::{LotId, WalletId};
use crate::optimize::{fold_as_of, synthetic_state};
use crate::price::{fmv_of, PriceProvider};
use crate::project::pools::{method_order, pool_key};
use crate::project::{
    evaluate_disposal, project, CandidateDisposal, EvaluateError, LotMethod, ProjectionConfig,
};
use crate::state::{Blocker, Lot, Term};
use crate::tax::{compute_tax_year, TaxOutcome, TaxProfile, TaxResult, TaxTables};

/// The lot-selection choice for a hypothetical sale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SellMethod {
    /// Consume in a specific method order (FIFO/LIFO/HIFO) instead of the standing method.
    Method(LotMethod),
    /// Consume these EXACT lots (specific identification). Σ sat must equal `sell_sat`.
    Lots(Vec<LotPick>),
}

/// A hypothetical, NON-persisted sale to evaluate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellRequest {
    pub sell_sat: Sat,
    pub wallet: WalletId,
    pub at: TaxDate,
    /// USD per WHOLE BTC (proceeds = round_cents(price × sell_sat / 1e8)). `None` ⇒ use the bundled
    /// dataset daily-close FMV for `at` (a future/off-dataset date then needs an explicit price).
    pub price: Option<Usd>,
    /// `None` ⇒ consume by the STANDING method (`applicable_method` — reused, never re-implemented).
    pub method: Option<SellMethod>,
}

/// Which §1(h) preferential-rate zone the sale's preferential dollars land in (from `pref_split`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtcgBracket {
    Zero,
    Fifteen,
    Twenty,
}

/// One lot consumed by the hypothetical sale (the per-leg schedule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumedLot {
    pub lot_id: LotId,
    pub sat: Sat,
    pub basis: Usd,
    pub acquired_at: TaxDate,
    pub sold_at: TaxDate,
    pub term: Term,
    pub gain: Usd,
}

/// §1212(b) carryforward-out delta (SIGNED: `withhyp − baseline`, by character). A loss sale RAISES
/// the carryforward (positive); a gain sale that absorbs a carried loss LOWERS it (negative). Unlike
/// `Carryforward` (whose fields are non-negative magnitudes), these are signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarryforwardDelta {
    pub short: Usd,
    pub long: Usd,
}

/// Whether the hypothetical sale is a net gain or a net loss (drives the §1212 disclosure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellStatus {
    Gain,
    Loss,
}

/// The read-only result of `whatif::sell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellReport {
    pub req: SellRequest,
    /// Resolved USD proceeds for the whole sale (from `--price` or dataset FMV).
    pub proceeds: Usd,
    /// The per-lot consumption schedule (standing method / requested method / explicit lots).
    pub lots: Vec<ConsumedLot>,
    pub st_gain: Usd,
    pub lt_gain: Usd,
    /// Which §1(h) bracket the preferential dollars land in, and how many MORE preferential dollars
    /// fit before crossing into the next bracket (`None` at 20% — the top bracket has no headroom).
    pub bracket: LtcgBracket,
    pub bracket_room: Option<Usd>,
    /// The EXACT marginal federal tax the sale causes (`withhyp.total − baseline.total`; the no-crypto
    /// term cancels). This — NOT the whole-year figure — is the sale's own effect.
    pub marginal_tax: Usd,
    /// `marginal_tax ÷ gain`, rounded to 4dp. `None` when the sale's total gain ≤ 0 (a loss/zero sale
    /// has no meaningful effective rate — its VALUE is the carryforward, not this-year tax).
    pub effective_rate: Option<Usd>,
    /// §1212(b) carryforward-out delta (signed; `withhyp − baseline`).
    pub carryforward_delta: CarryforwardDelta,
    /// [R0-I1] The THIS-YEAR §1211(b) ordinary offset the sale unlocks = `withhyp.loss_deduction −
    /// baseline.loss_deduction`. This is $0 (NOT $3,000) when the baseline already consumes the
    /// §1211(b) cap (pre-existing real losses / a carryforward-in) — the disclosure is delta-based.
    pub ordinary_offset_delta: Usd,
    /// [R0-I2] §1411 NIIT delta the sale causes = `withhyp.niit − baseline.niit` (NOT the raw
    /// `MarginalRates.niit_applies`, which is crypto-vs-no-crypto and misreports a NIIT-reducing
    /// loss harvest). Signed: a loss harvest can make this negative.
    pub niit_incremental: Usd,
    /// `niit_incremental != 0` (the sale actually moved NIIT).
    pub niit_applies: bool,
    pub status: SellStatus,
    /// The full baseline / with-hypothetical results (so a caller can show any marginal field).
    pub baseline: TaxResult,
    pub withhyp: TaxResult,
}

/// Errors from the what-if engine. Mirrors `OptimizeError` — the SAME refusal taxonomy as `consult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhatIfError {
    /// The engine refuses to compute the year (any Hard blocker anywhere, or a missing table/profile).
    YearNotComputable(Blocker),
    /// A synthetic disposal needs an explicit `--price` (no dataset FMV for a future/off-dataset date).
    Evaluate(EvaluateError),
    /// The wallet's as-of pool cannot cover `sell_sat`.
    NoLots,
    /// The date is pre-2025 — a restatement of a closed year, not a plan.
    PreTransitionYear(i32),
}

fn computed(o: TaxOutcome) -> Result<TaxResult, WhatIfError> {
    match o {
        TaxOutcome::Computed(r) => Ok(r),
        TaxOutcome::NotComputable(b) => Err(WhatIfError::YearNotComputable(b)),
    }
}

/// The shared injector: fold the UNMODIFIED timeline (baseline) and the timeline with the synthetic
/// disposal appended (withhyp), and compute BOTH years' `TaxResult`. `picks = None` ⇒ the withhyp fold
/// consumes by the STANDING method; `Some(picks)` ⇒ that exact selection. Baseline and withhyp differ
/// ONLY by the synthetic append (`project` == `fold(resolve(..))`, `synthetic_state` == the same plus
/// the appended `Op::Dispose`), so `withhyp.total − baseline.total` is the EXACT marginal.
#[allow(clippy::too_many_arguments)]
pub fn synthetic_year(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    candidate: &CandidateDisposal,
    picks: Option<&[LotPick]>,
) -> Result<(TaxResult, TaxResult), WhatIfError> {
    // Baseline: the real projection (exactly what `report --tax-year` computes).
    let baseline_state = project(events, prices, config);
    let baseline = computed(compute_tax_year(
        events,
        &baseline_state,
        year,
        profile,
        tables,
    ))?;
    // Withhyp: the same projection with the synthetic `Op::Dispose` appended (clone-fold-discard).
    let with_state =
        synthetic_state(events, prices, config, candidate, picks).map_err(WhatIfError::Evaluate)?;
    let withhyp = computed(compute_tax_year(events, &with_state, year, profile, tables))?;
    Ok((baseline, withhyp))
}

/// Build a specific-method selection over the as-of pool: `method_order` (reused — never a
/// re-implemented HIFO) gives the consumption ranking; a trivial greedy fill splits `sell_sat` across
/// lots (partial on the last). Σ sat == `sell_sat` whenever the pool covers it (checked by the caller).
fn method_selection(lots: &[Lot], method: LotMethod, sell_sat: Sat) -> Vec<LotPick> {
    let mut need = sell_sat;
    let mut picks = Vec::new();
    for i in method_order(lots, method) {
        if need <= 0 {
            break;
        }
        let take = need.min(lots[i].remaining_sat);
        if take > 0 {
            picks.push(LotPick {
                lot: lots[i].lot_id.clone(),
                sat: take,
            });
            need -= take;
        }
    }
    picks
}

/// READ-ONLY hypothetical sale. Injects a synthetic `Op::Dispose` (standing method / requested method /
/// explicit lots), reads the per-lot schedule + ST/LT split via `evaluate_disposal`, and computes the
/// MARGINAL federal tax + §1212 carryforward delta + §1(h) bracket + §1411 NIIT delta — every dollar
/// straight from `compute_tax_year`. Writes NOTHING (clone-fold-discard throughout).
pub fn sell(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    req: &SellRequest,
) -> Result<SellReport, WhatIfError> {
    let year = req.at.year();
    if year < TRANSITION_DATE.year() {
        return Err(WhatIfError::PreTransitionYear(year));
    }

    // As-of pool for the wallet (mirrors `consult_sale`): the NoLots gate + the lot table for the legs.
    let pre = fold_as_of(events, prices, config, req.at);
    let want = pool_key(req.at, &req.wallet);
    let mut lots: Vec<Lot> = pre
        .lots
        .into_iter()
        .filter(|l| {
            l.remaining_sat > 0 && pool_key(req.at, &l.wallet) == want && l.acquired_at <= req.at
        })
        .collect();
    lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
    if lots.iter().map(|l| l.remaining_sat).sum::<Sat>() < req.sell_sat {
        return Err(WhatIfError::NoLots);
    }

    // Proceeds: explicit per-BTC price → total; else the dataset FMV is resolved downstream. Fail fast
    // when neither exists (a future/off-dataset date with no `--price`), mirroring `consult_sale`.
    let proceeds: Option<Usd> = req
        .price
        .map(|px| round_cents(px * Usd::from(req.sell_sat) / Usd::from(SATS_PER_BTC)));
    if proceeds.is_none() && fmv_of(prices, req.at, req.sell_sat).is_none() {
        return Err(WhatIfError::Evaluate(EvaluateError::ProceedsRequired));
    }
    let candidate = CandidateDisposal {
        existing_event: None,
        wallet: req.wallet.clone(),
        date: req.at,
        sat: req.sell_sat,
        kind: DisposeKind::Sell,
        proceeds,
    };

    // Selection: standing method (None) / requested method / explicit lots.
    let picks: Option<Vec<LotPick>> = match &req.method {
        None => None,
        Some(SellMethod::Method(m)) => Some(method_selection(&lots, *m, req.sell_sat)),
        Some(SellMethod::Lots(p)) => Some(p.clone()),
    };
    let picks_ref = picks.as_deref();

    // Per-lot schedule + ST/LT split (side-effect-free).
    let out = evaluate_disposal(events, prices, config, &candidate, picks_ref)
        .map_err(WhatIfError::Evaluate)?;
    // Baseline + with-hypothetical full results.
    let (baseline, withhyp) = synthetic_year(
        events, prices, config, year, profile, tables, &candidate, picks_ref,
    )?;

    // Resolved proceeds for display (guaranteed present by the fail-fast guard above).
    let resolved_proceeds = candidate
        .proceeds
        .or_else(|| fmv_of(prices, req.at, req.sell_sat))
        .unwrap_or(Usd::ZERO);

    let lots_consumed: Vec<ConsumedLot> = out
        .legs
        .iter()
        .map(|l| ConsumedLot {
            lot_id: l.lot_id.clone(),
            sat: l.sat,
            basis: l.basis,
            acquired_at: l.acquired_at,
            sold_at: req.at,
            term: l.term,
            gain: l.gain,
        })
        .collect();

    // §1(h) bracket + headroom, from the WITH-scenario `pref_split` (P0) and the year's breakpoints.
    let ps = withhyp.pref_split;
    let bracket = if ps.at_20 > Usd::ZERO {
        LtcgBracket::Twenty
    } else if ps.at_15 > Usd::ZERO {
        LtcgBracket::Fifteen
    } else {
        LtcgBracket::Zero
    };
    // `table_for(year)` + `profile` are both guaranteed `Some` here (withhyp computed successfully;
    // a missing table/profile would have refused as `YearNotComputable`).
    let bp = profile
        .map(|p| p.filing_status)
        .and_then(|fs| tables.table_for(year).map(|t| *t.ltcg_for(fs)));
    let top = withhyp.bottom_with + ps.at_0 + ps.at_15 + ps.at_20;
    let bracket_room = bp.and_then(|bp| match bracket {
        LtcgBracket::Zero => Some((bp.max_zero - top).max(Usd::ZERO)),
        LtcgBracket::Fifteen => Some((bp.max_fifteen - top).max(Usd::ZERO)),
        LtcgBracket::Twenty => None,
    });

    let st_gain = out.st_gain;
    let lt_gain = out.lt_gain;
    let gain = st_gain + lt_gain;
    let marginal_tax =
        withhyp.total_federal_tax_attributable - baseline.total_federal_tax_attributable;
    let effective_rate = if gain > Usd::ZERO {
        Some((marginal_tax / gain).round_dp(4))
    } else {
        None
    };
    let carryforward_delta = CarryforwardDelta {
        short: withhyp.carryforward_out.short - baseline.carryforward_out.short,
        long: withhyp.carryforward_out.long - baseline.carryforward_out.long,
    };
    let ordinary_offset_delta = withhyp.loss_deduction - baseline.loss_deduction;
    let niit_incremental = withhyp.niit - baseline.niit;
    let status = if gain < Usd::ZERO {
        SellStatus::Loss
    } else {
        SellStatus::Gain
    };

    Ok(SellReport {
        req: req.clone(),
        proceeds: resolved_proceeds,
        lots: lots_consumed,
        st_gain,
        lt_gain,
        bracket,
        bracket_room,
        marginal_tax,
        effective_rate,
        carryforward_delta,
        ordinary_offset_delta,
        niit_incremental,
        niit_applies: niit_incremental != Usd::ZERO,
        status,
        baseline,
        withhyp,
    })
}
