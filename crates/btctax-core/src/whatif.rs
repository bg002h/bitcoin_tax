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
use rust_decimal::prelude::ToPrimitive;
use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

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
    /// A `harvest` target amount is ill-posed (a `Gain(X)`/`Tax(X)` with X < 0 ⇒ an EMPTY prefix
    /// feasible set — loss harvesting is a different problem; use `what-if sell`).
    InvalidTarget(String),
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

// ════════════════════════════════════════════════════════════════════════════════════════════════
// P2 — the harvest optimizer (task #43). The architect's lot-edge SEGMENT WALK (fable report §2/§5/§6):
// NOT global bisection (marginal-tax(N) is non-monotone — HIFO realizes losses first ⇒ a dip, the §1211
// $3k pin, and the §1212 carryforward-absorption plateau). The engine (`compute_tax_year`) is the ONLY
// oracle; the analytic solve is a SEED, sat-bisection is the DECIDER, and the returned N* is ALWAYS
// re-folded + verified true. Answer semantics for EVERY target: the max N such that the predicate holds
// on the ENTIRE PREFIX [0, N] (safe under partial fills + non-contiguous feasible sets under a FIFO/LIFO
// election). Writes NOTHING (clone-fold-discard, same seam as `sell`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The tolerance band (§4.3): N* is the maximum within τ sats. 1,024 sats is < $0.05 of tax at any
/// realistic BTC price — sub-materiality. Bisection stops here so the per-leg cent-rounding wiggle (up
/// to ⌈legs/2⌉ cents inside a segment) can never mislocate the boundary below the noise floor.
pub const HARVEST_TAU_SAT: Sat = 1_024;

/// The harvest optimization target. `Gain`/`Tax` amounts are USD and MUST be ≥ 0 (v1: a negative cap ⇒
/// an empty prefix set — loss harvesting is `what-if sell`, a different problem — rejected as
/// `InvalidTarget`). The two bracket targets are threshold-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarvestTarget {
    /// Max N with ZERO preferential dollars above `max_zero` (`pref_split.at_15 + at_20 == 0`) — sell
    /// as much long-term gain as fits entirely in the §1(h) 0% bracket.
    ZeroLtcg,
    /// Max N with ZERO 20%-bracket preferential dollars (`pref_split.at_20 == 0`) — stay at/under 15%.
    FifteenLtcg,
    /// Max N whose SALE-LOCAL realized net gain (`st_gain + lt_gain`, the synthetic disposal's own legs)
    /// is ≤ X. "Realize at most $X of gain WITH this sale." Requires X ≥ 0.
    Gain(Usd),
    /// Max N whose MARGINAL federal tax (`total(N) − total(0)`) is ≤ X. `tax=$0` is the flagship harvest
    /// primitive ("sell as much as possible adding zero federal tax"). Requires X ≥ 0.
    Tax(Usd),
}

/// Parse failure for [`HarvestTarget`]'s [`FromStr`] — the single source of truth shared by the CLI
/// `--target` parse and the TUI panel's target field. A PURE LEXER: it accepts/rejects exactly what the
/// legacy `parse_harvest_target` did. Note it does NOT reject negatives — `gain=-1`/`tax=-1` parse to
/// `Gain(-1)`/`Tax(-1)` and the ENGINE refuses them as `InvalidTarget` (a downstream refusal, not a
/// parse error), preserving the historical error class/path/message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarvestTargetParseError {
    /// The trimmed, lowercased string matched no alias and no `gain=`/`tax=` prefix. Carries the
    /// ORIGINAL (un-lowercased) input for the message.
    UnrecognizedTarget(String),
    /// A `gain=`/`tax=` amount that `Usd::from_str` rejected (e.g. `gain=abc`). Only `$` and `,` are
    /// stripped before parsing; `_` is left intact but `rust_decimal` accepts it as a digit separator,
    /// so `gain=1_000` is `Gain(1000)`, NOT a `BadAmount` — parity with the legacy lexer. Carries the
    /// offending amount substring.
    BadAmount(String),
}

impl fmt::Display for HarvestTargetParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarvestTargetParseError::UnrecognizedTarget(s) => write!(
                f,
                "bad --target {s:?}: expected zero-ltcg | fifteen-ltcg | gain=$X | tax=$X"
            ),
            HarvestTargetParseError::BadAmount(v) => {
                write!(f, "bad --target amount {v:?}: expected a USD number")
            }
        }
    }
}

impl std::error::Error for HarvestTargetParseError {}

impl FromStr for HarvestTarget {
    type Err = HarvestTargetParseError;

    /// Parse `--target`: `zero-ltcg` | `fifteen-ltcg` | `gain=$X` | `tax=$X` (`$`/commas optional,
    /// case-insensitive). BYTE-FOR-BYTE the legacy `parse_harvest_target` lexer — NO new checks, in
    /// particular no negative-rejection (see [`HarvestTargetParseError`]).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.trim().to_ascii_lowercase();
        match lower.as_str() {
            "zero-ltcg" | "zero_ltcg" | "zeroltcg" => return Ok(HarvestTarget::ZeroLtcg),
            "fifteen-ltcg" | "fifteen_ltcg" | "fifteenltcg" => {
                return Ok(HarvestTarget::FifteenLtcg)
            }
            _ => {}
        }
        if let Some(v) = lower.strip_prefix("gain=") {
            return Ok(HarvestTarget::Gain(parse_target_amount(v)?));
        }
        if let Some(v) = lower.strip_prefix("tax=") {
            return Ok(HarvestTarget::Tax(parse_target_amount(v)?));
        }
        Err(HarvestTargetParseError::UnrecognizedTarget(s.to_string()))
    }
}

/// Parse a `gain=`/`tax=` amount: strip `$` and `,` (NOT `_`), then `Usd::from_str`. Negatives parse
/// fine (the engine refuses them downstream — see [`HarvestTargetParseError`]).
fn parse_target_amount(v: &str) -> Result<Usd, HarvestTargetParseError> {
    let cleaned = v.trim().replace(['$', ','], "");
    Usd::from_str(&cleaned).map_err(|_| HarvestTargetParseError::BadAmount(v.to_string()))
}

/// A hypothetical, NON-persisted harvest question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestRequest {
    pub wallet: WalletId,
    pub at: TaxDate,
    /// USD per WHOLE BTC (each candidate N's proceeds = `round_cents(price × N / 1e8)`). `None` ⇒ the
    /// bundled dataset daily-close FMV prices each candidate N (a future/off-dataset date then needs
    /// an explicit price).
    pub price: Option<Usd>,
    pub target: HarvestTarget,
}

/// The OUTCOME CHARACTER of a computed harvest answer. The architect's full status taxonomy also names
/// the REFUSALS (`NoLots` / `ProceedsRequired` / `PreTransitionYear` / `YearNotComputable`); those are
/// surfaced through the shared `WhatIfError` (mirroring `whatif::sell`) and mapped back to this label by
/// [`HarvestStatus::of_refusal`], so the CLI can show a consistent status line either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarvestStatus {
    /// The target binds inside the pool: N* is the max prefix-feasible amount.
    Found,
    /// The target never binds over the available pool — the full position fits (answer = N_avail).
    NotBinding,
    /// The predicate already fails at N = 0 (e.g. baseline QD alone exceeds `max_zero`); answer = 0.
    AlreadyBreached,
    /// The wallet's as-of pool has no harvestable sats (empty, or the first lot in consumption order has
    /// pending basis so N_avail == 0). Answer = 0.
    NoLots,
    /// (refusal label) A future/off-dataset date with no `--price` and no dataset FMV.
    ProceedsRequired,
    /// (refusal label) A pre-2025 date — a restatement of a closed year, not a plan.
    PreTransitionYear,
    /// (refusal label) The engine refuses the year (any Hard blocker anywhere, or a missing table/profile).
    YearNotComputable(Blocker),
}

impl HarvestStatus {
    /// The status LABEL for a refusal `WhatIfError` (so the CLI can render a uniform "status:" line for
    /// both the `Ok` answers and the `Err` refusals). Never returns `Found`/`NotBinding`/`AlreadyBreached`.
    pub fn of_refusal(e: &WhatIfError) -> HarvestStatus {
        match e {
            WhatIfError::NoLots => HarvestStatus::NoLots,
            WhatIfError::Evaluate(_) => HarvestStatus::ProceedsRequired,
            WhatIfError::PreTransitionYear(_) => HarvestStatus::PreTransitionYear,
            WhatIfError::YearNotComputable(b) => HarvestStatus::YearNotComputable(b.clone()),
            WhatIfError::InvalidTarget(_) => HarvestStatus::NoLots,
        }
    }
}

/// The read-only result of `whatif::harvest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestReport {
    pub req: HarvestRequest,
    /// The answer: the max prefix-feasible amount, in sats (0 for `NoLots`/`AlreadyBreached`).
    pub n_sat: Sat,
    /// `n_sat` in whole BTC (= `n_sat / 1e8`), for display.
    pub n_btc: Usd,
    pub status: HarvestStatus,
    /// Human-readable statement of the constraint that bound N* (or why it didn't bind).
    pub binding_constraint: String,
    /// The SALE-LOCAL realized gain at N* (Σ the synthetic disposal's leg gains), by character.
    pub st_gain: Usd,
    pub lt_gain: Usd,
    /// The ENGINE-VERIFIED with-N* `TaxResult` (== `baseline` when n_sat == 0).
    pub with_result: TaxResult,
    /// The baseline (N = 0) `TaxResult` — the reference for every marginal field / disclosure.
    pub baseline: TaxResult,
    /// The EXACT marginal federal tax at N* = `with_result.total − baseline.total`.
    pub marginal_tax: Usd,
    /// §1212(b) carryforward-out delta at N* (SIGNED, `with − baseline`, by character). A NEGATIVE value
    /// is the carryforward BURN — a gain that absorbed a carried loss for $0 current-year tax.
    pub carryforward_delta: CarryforwardDelta,
    /// §1411 NIIT delta at N* = `with_result.niit − baseline.niit` (NOT the raw `niit_applies` flag).
    pub niit_incremental: Usd,
    /// `niit_incremental != 0` (the harvest actually moved NIIT). On a bracket target a nonzero value is
    /// the NIIT-kink disclosure: a 0%/15% answer can still cost +3.8%.
    pub niit_applies: bool,
    /// N_avail was TRUNCATED at a basis-pending lot in consumption order (further lots not harvestable).
    pub pending_capped: bool,
    /// The MANDATORY §1212 disclosure: the carryforward BURN (a gain harvest spending a carried loss) or
    /// the §1211(b) $3k-pin plateau (an all-loss position — only the cap is deductible this year). `None`
    /// when neither applies.
    pub plateau_note: Option<String>,
}

/// One engine probe at a candidate N: the full with-N `TaxResult` plus the SALE-LOCAL ST/LT gain split
/// (from the synthetic disposal's own legs — the `gain=$X` predicate is sale-local per the architect).
#[derive(Debug, Clone)]
struct Probe {
    tax: TaxResult,
    st_gain: Usd,
    lt_gain: Usd,
}

/// The per-target predicate on a with-N engine scenario. Prefix semantics: the answer is the max N such
/// that THIS holds for every probed n in [0, N]. Bracket targets read the with-scenario `PrefSplit`
/// (`at_*` are UNROUNDED Decimals — exact) — NEVER `MarginalRates.ltcg` (which reports a rate off `top`
/// even with zero preferential dollars, disagreeing with the vacuous case).
fn predicate_holds(target: HarvestTarget, p: &Probe, baseline: &TaxResult) -> bool {
    match target {
        HarvestTarget::ZeroLtcg => p.tax.pref_split.at_15 + p.tax.pref_split.at_20 <= Usd::ZERO,
        HarvestTarget::FifteenLtcg => p.tax.pref_split.at_20 <= Usd::ZERO,
        HarvestTarget::Gain(x) => p.st_gain + p.lt_gain <= x,
        HarvestTarget::Tax(x) => {
            p.tax.total_federal_tax_attributable - baseline.total_federal_tax_attributable <= x
        }
    }
}

/// The analytic SEED (never the decider): a linear solve on the segment's exact-Decimal slope against the
/// numeric threshold, for the `Gain`/`Tax` targets only (the bracket targets bisect from the midpoint).
/// Returns a sat offset-into-`(lo, hi)` or `None` (degenerate slope ⇒ fall back to midpoint bisection).
fn analytic_seed(
    target: HarvestTarget,
    baseline: &TaxResult,
    lo: Sat,
    lo_p: &Probe,
    hi: Sat,
    hi_p: &Probe,
) -> Option<Sat> {
    let span = hi - lo;
    if span <= 1 {
        return None;
    }
    let (x, f_lo, f_hi) = match target {
        HarvestTarget::Tax(x) => (
            x,
            lo_p.tax.total_federal_tax_attributable - baseline.total_federal_tax_attributable,
            hi_p.tax.total_federal_tax_attributable - baseline.total_federal_tax_attributable,
        ),
        HarvestTarget::Gain(x) => (x, lo_p.st_gain + lo_p.lt_gain, hi_p.st_gain + hi_p.lt_gain),
        _ => return None,
    };
    if f_hi <= f_lo {
        return None; // non-increasing across the segment (a plateau) ⇒ midpoint is fine
    }
    // n ≈ lo + (x − f_lo)/(f_hi − f_lo) · span, floored to a sat, kept STRICTLY inside (lo, hi).
    let offset = ((x - f_lo) * Usd::from(span) / (f_hi - f_lo))
        .floor()
        .to_i64()?;
    Some((lo + offset).clamp(lo + 1, hi - 1))
}

/// READ-ONLY harvest optimizer. Finds the MAX N (in sats) sellable from `req.wallet`'s as-of pool such
/// that `req.target` holds on the entire prefix [0, N], computed ONLY through `compute_tax_year` via the
/// standing-method consumption schedule. Writes NOTHING (clone-fold-discard throughout). Refusals mirror
/// `sell` (the shared `WhatIfError` taxonomy).
#[allow(clippy::too_many_arguments)]
pub fn harvest(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    req: &HarvestRequest,
) -> Result<HarvestReport, WhatIfError> {
    let year = req.at.year();
    if year < TRANSITION_DATE.year() {
        return Err(WhatIfError::PreTransitionYear(year));
    }
    let kind = DisposeKind::Sell;

    // v1 well-posedness: a Gain/Tax cap of X < 0 makes the prefix set EMPTY (G(0)=0, marginal(0)=0).
    if let HarvestTarget::Gain(x) | HarvestTarget::Tax(x) = req.target {
        if x < Usd::ZERO {
            return Err(WhatIfError::InvalidTarget(format!(
                "target amount must be >= 0 (got {x}); to harvest LOSSES use `what-if sell` \
                 (a loss sale is not a max-N-under-a-cap problem)"
            )));
        }
    }

    // ── P0: baseline `total(0)` (also the refusal gate: missing table/profile / any Hard blocker). ──
    let baseline_state = project(events, prices, config);
    let baseline = computed(compute_tax_year(
        events,
        &baseline_state,
        year,
        profile,
        tables,
    ))?;

    // ── P0: the wallet's as-of pool (mirror `sell`): the NoLots gate + basis-pending detection. ──
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
    let total_all: Sat = lots.iter().map(|l| l.remaining_sat).sum();
    let pending_ids: BTreeSet<LotId> = lots
        .iter()
        .filter(|l| l.basis_pending)
        .map(|l| l.lot_id.clone())
        .collect();

    if total_all == 0 {
        return Ok(no_op_report(
            req,
            &baseline,
            HarvestStatus::NoLots,
            "no lots available to harvest from that wallet as of that date".into(),
            false,
        ));
    }
    // Proceeds availability (mirror `sell`): explicit `--price` OR a dataset FMV, else ProceedsRequired.
    if req.price.is_none() && fmv_of(prices, req.at, total_all).is_none() {
        return Err(WhatIfError::Evaluate(EvaluateError::ProceedsRequired));
    }

    // ── P1: ONE fold of the whole pool (NO injected selection ⇒ the STANDING method) gives the exact
    // consumption ORDER. The lot EDGES are the cumulative leg sats, truncated at the first basis-pending
    // lot (consuming one fires FmvMissing ⇒ the engine refuses). Proceeds don't affect the ORDER, but
    // `evaluate_disposal` needs some resolvable proceeds (guarded above).
    //
    // NOTE (belt-and-suspenders): in THIS engine every basis-pending origin (unknown-basis gift OR
    // FMV-missing income) also raises a RESTING Hard blocker at its origin event, so the `baseline`
    // compute above already refused with `YearNotComputable` before we get here — a pending lot in the
    // pool ⇒ the whole year is uncomputable (the conservative, correct behavior). The truncation is kept
    // per the architect's design so N_avail stays sound should a future NON-gating pending lot ever exist.
    let full_proceeds = req
        .price
        .map(|px| round_cents(px * Usd::from(total_all) / Usd::from(SATS_PER_BTC)));
    let full_candidate = CandidateDisposal {
        existing_event: None,
        wallet: req.wallet.clone(),
        date: req.at,
        sat: total_all,
        kind,
        proceeds: full_proceeds,
    };
    let full = evaluate_disposal(events, prices, config, &full_candidate, None)
        .map_err(WhatIfError::Evaluate)?;
    let mut edges: Vec<Sat> = Vec::new();
    let mut cum: Sat = 0;
    let mut pending_capped = false;
    for leg in &full.legs {
        if pending_ids.contains(&leg.lot_id) {
            pending_capped = true;
            break;
        }
        cum += leg.sat;
        edges.push(cum);
    }
    let n_avail = edges.last().copied().unwrap_or(0);
    if n_avail == 0 {
        return Ok(no_op_report(
            req,
            &baseline,
            HarvestStatus::NoLots,
            "N capped at 0 — the first lot in consumption order has pending basis".into(),
            true,
        ));
    }

    // The engine probe at a candidate N (clone-fold-discard): the with-N `TaxResult` + sale-local ST/LT.
    let probe = |n: Sat| -> Result<Probe, WhatIfError> {
        if n <= 0 {
            return Ok(Probe {
                tax: baseline.clone(),
                st_gain: Usd::ZERO,
                lt_gain: Usd::ZERO,
            });
        }
        let proceeds = req
            .price
            .map(|px| round_cents(px * Usd::from(n) / Usd::from(SATS_PER_BTC)));
        let cand = CandidateDisposal {
            existing_event: None,
            wallet: req.wallet.clone(),
            date: req.at,
            sat: n,
            kind,
            proceeds,
        };
        let out = evaluate_disposal(events, prices, config, &cand, None)
            .map_err(WhatIfError::Evaluate)?;
        let state =
            synthetic_state(events, prices, config, &cand, None).map_err(WhatIfError::Evaluate)?;
        let tax = computed(compute_tax_year(events, &state, year, profile, tables))?;
        Ok(Probe {
            tax,
            st_gain: out.st_gain,
            lt_gain: out.lt_gain,
        })
    };

    // ── P0: already breached at N = 0? (only the bracket targets can — Gain/Tax hold vacuously at 0.) ──
    let base_probe = Probe {
        tax: baseline.clone(),
        st_gain: Usd::ZERO,
        lt_gain: Usd::ZERO,
    };
    if !predicate_holds(req.target, &base_probe, &baseline) {
        return Ok(no_op_report(
            req,
            &baseline,
            HarvestStatus::AlreadyBreached,
            format!(
                "target already breached at N=0 ({})",
                binding_label(req.target)
            ),
            pending_capped,
        ));
    }

    // ── P2: lot-edge walk — first edge where the predicate goes true→false. Per T1 (tax(N) monotone
    // WITHIN a lot segment) the edge checks bound the interior, so the prefix condition is verified by
    // the walk itself.
    let mut lo_edge: Sat = 0;
    let mut lo_probe = base_probe;
    let mut break_edge: Option<(Sat, Probe)> = None;
    for &e in &edges {
        let p = probe(e)?;
        if predicate_holds(req.target, &p, &baseline) {
            lo_edge = e;
            lo_probe = p;
        } else {
            break_edge = Some((e, p));
            break;
        }
    }

    // ── P3: resolve the answer (NotBinding, or the boundary inside the first failing segment). ──
    let (n_star, star_probe, status) = match break_edge {
        None => (n_avail, lo_probe, HarvestStatus::NotBinding),
        Some((hi_edge, hi_probe)) => {
            // Boundary in (lo_edge, hi_edge]: predicate holds at lo_edge, fails at hi_edge. Within one
            // segment tax(N)/gain(N) are monotone (T1) ⇒ sat-bisection is SOUND. Analytic seed → bisect.
            // `lo` always holds the largest KNOWN-true N; the seed is only a first probe (never trusted).
            let mut lo = lo_edge;
            let mut hi = hi_edge;
            if let Some(seed) = analytic_seed(req.target, &baseline, lo, &lo_probe, hi, &hi_probe) {
                if predicate_holds(req.target, &probe(seed)?, &baseline) {
                    lo = seed;
                } else {
                    hi = seed;
                }
            }
            while hi - lo > HARVEST_TAU_SAT {
                let mid = lo + (hi - lo) / 2;
                if predicate_holds(req.target, &probe(mid)?, &baseline) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            // MANDATORY final engine-verify: re-fold N* and confirm the predicate holds (never return an
            // N the engine did not fold + verify true).
            let final_p = probe(lo)?;
            debug_assert!(
                predicate_holds(req.target, &final_p, &baseline),
                "harvest invariant: the returned N* must be engine-verified true"
            );
            (lo, final_p, HarvestStatus::Found)
        }
    };

    // ── P4: report + the mandatory disclosures. ──
    let with_result = star_probe.tax.clone();
    let marginal_tax =
        with_result.total_federal_tax_attributable - baseline.total_federal_tax_attributable;
    let carryforward_delta = CarryforwardDelta {
        short: with_result.carryforward_out.short - baseline.carryforward_out.short,
        long: with_result.carryforward_out.long - baseline.carryforward_out.long,
    };
    let niit_incremental = with_result.niit - baseline.niit;
    let plateau_note = plateau_note(&baseline, &with_result, carryforward_delta, pending_capped);
    let binding_constraint = match status {
        HarvestStatus::NotBinding => {
            "available pool exhausted — the target does not bind (the full position fits)"
                .to_string()
        }
        _ => binding_label(req.target),
    };

    Ok(HarvestReport {
        req: req.clone(),
        n_sat: n_star,
        n_btc: Usd::from(n_star) / Usd::from(SATS_PER_BTC),
        status,
        binding_constraint,
        st_gain: star_probe.st_gain,
        lt_gain: star_probe.lt_gain,
        with_result,
        baseline,
        marginal_tax,
        carryforward_delta,
        niit_incremental,
        niit_applies: niit_incremental != Usd::ZERO,
        pending_capped,
        plateau_note,
    })
}

/// A human label for the constraint a `Found`/`AlreadyBreached` answer is measured against.
fn binding_label(target: HarvestTarget) -> String {
    match target {
        HarvestTarget::ZeroLtcg => "0% LTCG bracket ceiling (\u{00a7}1(h) max_zero)".to_string(),
        HarvestTarget::FifteenLtcg => {
            "15% LTCG bracket ceiling (\u{00a7}1(h) max_fifteen)".to_string()
        }
        HarvestTarget::Gain(x) => format!("realized-gain cap {x}"),
        HarvestTarget::Tax(x) => format!("marginal federal-tax cap {x}"),
    }
}

/// The §1212 disclosure (mandatory when material): the carryforward BURN (a gain harvest spending a
/// carried loss for $0 current-year tax) or the §1211(b) $3k-pin plateau (an all-loss position). Also
/// carries the pending-basis-cap note. `None` when nothing material happened.
fn plateau_note(
    baseline: &TaxResult,
    with_result: &TaxResult,
    cf_delta: CarryforwardDelta,
    pending_capped: bool,
) -> Option<String> {
    let carried = cf_delta.short + cf_delta.long;
    let mut parts: Vec<String> = Vec::new();
    if carried < Usd::ZERO {
        // A gain absorbed a carried loss: the carryforward is BURNED (spent for $0 current-year tax).
        parts.push(format!(
            "this harvest absorbs {} of loss carryforward (short {} / long {}) \u{2014} the \
             carryforward is SPENT, not deductible again",
            -carried, -cf_delta.short, -cf_delta.long
        ));
    } else if carried > Usd::ZERO {
        // A net-loss position: only the §1211(b) cap is deductible this year; the rest is carried.
        let offset = with_result.loss_deduction - baseline.loss_deduction;
        parts.push(format!(
            "net loss: only {} is deductible against ordinary income this year (\u{00a7}1211(b) cap); \
             {} carried to next year (short {} / long {})",
            offset, carried, cf_delta.short, cf_delta.long
        ));
    }
    if pending_capped {
        parts.push(
            "N capped: further lots in consumption order have PENDING basis (resolve them to harvest more)"
                .to_string(),
        );
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

/// Build a zero-N report (NoLots / AlreadyBreached): the with-scenario == the baseline (no sale).
fn no_op_report(
    req: &HarvestRequest,
    baseline: &TaxResult,
    status: HarvestStatus,
    binding_constraint: String,
    pending_capped: bool,
) -> HarvestReport {
    HarvestReport {
        req: req.clone(),
        n_sat: 0,
        n_btc: Usd::ZERO,
        status,
        binding_constraint,
        st_gain: Usd::ZERO,
        lt_gain: Usd::ZERO,
        with_result: baseline.clone(),
        baseline: baseline.clone(),
        marginal_tax: Usd::ZERO,
        carryforward_delta: CarryforwardDelta {
            short: Usd::ZERO,
            long: Usd::ZERO,
        },
        niit_incremental: Usd::ZERO,
        niit_applies: false,
        pending_capped,
        plateau_note: None,
    }
}
