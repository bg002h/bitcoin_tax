//! Rate-application primitives (Sub-project B, Task 3): the exact-`Decimal` arithmetic core.
//!
//! Two pure functions:
//! - [`ordinary_tax_on`] — progressive marginal-bracket tax on ordinary taxable income.
//! - [`preferential_tax`] — §1(h) 0/15/20% preferential stacking (long-term gain + qualified
//!   dividends sit ON TOP of ordinary income for breakpoint placement), returning a [`PrefSplit`].
//!
//! **Exactness/determinism (NFR4/NFR5):** all math is `Decimal`; there is **no float** anywhere
//! (every rate is a `Decimal` literal). This is the **exact marginal-bracket formula method at cent
//! precision** — NOT the IRS binned Tax Tables and NOT whole-dollar rounding — with `ROUND_HALF_EVEN`
//! to cents applied at the END only (the project's canonical `round_cents`).
use crate::conventions::{round_cents, Usd};
use crate::event::LedgerEvent;
use crate::state::{Blocker, BlockerKind, LedgerState, Severity, Term};
use crate::tax::tables::{
    loss_limit, niit_threshold, LtcgBreakpoints, OrdinarySchedule, TaxTables, NIIT_RATE,
};
use crate::tax::types::{Carryforward, MarginalRates, TaxOutcome, TaxProfile, TaxResult};

/// Exact marginal-bracket tax on `taxable` (≥ 0). Sums (min(taxable, next_lower) − lower) × rate over each
/// bracket the income reaches; the open-ended top bracket has no upper bound. ROUND_HALF_EVEN to cents at
/// the END only (NFR5). NOT the IRS binned Tax Tables and NOT whole-dollar rounding — the exact formula
/// method at cent precision (deliberate determinism/exactness choice).
pub fn ordinary_tax_on(schedule: &OrdinarySchedule, taxable: Usd) -> Usd {
    if taxable <= Usd::ZERO {
        return Usd::ZERO;
    }
    let b = &schedule.brackets;
    let mut tax = Usd::ZERO;
    for (i, br) in b.iter().enumerate() {
        if taxable <= br.lower {
            break;
        }
        let upper = b.get(i + 1).map(|n| n.lower).unwrap_or(taxable); // open-ended top
        let span_top = if taxable < upper { taxable } else { upper };
        tax += (span_top - br.lower) * br.rate;
    }
    round_cents(tax)
}

/// The §1(h) preferential-rate split: how many preferential dollars land in each rate zone, plus the tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefSplit {
    pub at_0: Usd,
    pub at_15: Usd,
    pub at_20: Usd,
    pub tax: Usd,
}

/// §1(h) stacking: preferential income `pref` (= QD + net LT gain) sits ON TOP of `bottom` (ordinary
/// taxable income incl. net ST gain). Breakpoints are compared against TOTAL taxable income (bottom+pref);
/// ordinary income fills the bottom of the stack first. Exact Decimal; ROUND_HALF_EVEN at the end.
pub fn preferential_tax(bp: &LtcgBreakpoints, bottom: Usd, pref: Usd) -> PrefSplit {
    let z = Usd::ZERO;
    if pref <= z {
        return PrefSplit {
            at_0: z,
            at_15: z,
            at_20: z,
            tax: z,
        };
    }
    let bottom = if bottom < z { z } else { bottom };
    let top = bottom + pref;
    let clamp = |v: Usd| if v < z { z } else { v };
    // 0% zone: pref dollars below max_zero
    let at_0 = {
        let room = clamp(bp.max_zero - bottom);
        if room < pref {
            room
        } else {
            pref
        }
    };
    // 15% zone: (max_zero, max_fifteen]
    let lower15 = if bottom > bp.max_zero {
        bottom
    } else {
        bp.max_zero
    };
    let upper15 = if top < bp.max_fifteen {
        top
    } else {
        bp.max_fifteen
    };
    let at_15 = clamp(upper15 - lower15);
    let at_20 = pref - at_0 - at_15; // remainder above max_fifteen
    let tax = round_cents(at_15 * dec_15() + at_20 * dec_20());
    PrefSplit {
        at_0,
        at_15,
        at_20,
        tax,
    }
}
fn dec_15() -> Usd {
    rust_decimal_macros::dec!(0.15)
}
fn dec_20() -> Usd {
    rust_decimal_macros::dec!(0.20)
}

/// The result of §1222 ST/LT netting + the §1211/§1212(b) loss limit, by character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapNet {
    /// §1222(5)/(6): within-character net short-term (signed; after `cf_short`).
    pub st_net: Usd,
    /// §1222(7)/(8): within-character net long-term (signed; after `other_lt` & `cf_long`).
    pub lt_net: Usd,
    /// Net short-term gain surviving cross-net (≥0) → ordinary rates.
    pub ordinary_gain: Usd,
    /// §1222(11) net capital gain surviving cross-net (≥0) → §1(h) preferential rates.
    pub preferential_gain: Usd,
    /// §1211(b) ordinary offset used this year (≥0).
    pub loss_deduction: Usd,
    /// §1212(b) short-term carryforward out (≥0).
    pub st_carry: Usd,
    /// §1212(b) long-term carryforward out (≥0).
    pub lt_carry: Usd,
}

/// §1222 ST/LT netting + the §1211/§1212(b) capital-loss limit.
///
/// Inputs are signed: gains positive, losses negative for `crypto_st`/`crypto_lt`/`other_lt`.
/// `cf_short`/`cf_long` are prior-year carryforward LOSS magnitudes (≥0) — they REDUCE the matching
/// character. `loss_limit` is the statutory §1211(b) cap ($3,000 / $1,500 MFS).
///
/// Steps: (1) §1222(5)–(8) within-character netting, treating each prior-year carryforward as a loss of
/// its own character; (2) cross-net a gain in one character against a loss in the other, the residual
/// loss retaining the character it survived in; (3) §1211(b) deduct up to `loss_limit` against ordinary
/// income in a net-loss year; (4) §1212(b) carry the remainder forward by character, the deduction
/// absorbed **short-term-first** (the §1212(b)(2) deemed-short-term-gain ordering).
pub fn net_1222(
    crypto_st: Usd,
    crypto_lt: Usd,
    other_lt: Usd,
    cf_short: Usd,
    cf_long: Usd,
    loss_limit: Usd,
) -> CapNet {
    let z = Usd::ZERO;
    // §1222(5)/(6): within-character net short-term (carryforward-in is a short-term loss → subtract).
    let st_net = crypto_st - cf_short;
    // §1222(7)/(8): within-character net long-term (other_net_capital_gain is LT-character; cf_long subtracts).
    let lt_net = crypto_lt + other_lt - cf_long;

    // Cross-net a gain in one character against a loss in the other (§1222 / Schedule D line 16).
    let (st2, lt2) = match (st_net >= z, lt_net >= z) {
        (true, true) | (false, false) => (st_net, lt_net), // both gains, or both losses: no cross-net
        (true, false) => {
            // ST gain, LT loss
            if -lt_net <= st_net {
                (st_net + lt_net, z)
            } else {
                (z, st_net + lt_net)
            }
        }
        (false, true) => {
            // ST loss, LT gain
            if -st_net <= lt_net {
                (z, lt_net + st_net)
            } else {
                (st_net + lt_net, z)
            }
        }
    };
    let ordinary_gain = if st2 > z { st2 } else { z };
    let preferential_gain = if lt2 > z { lt2 } else { z };
    let net_st_loss = if st2 < z { -st2 } else { z };
    let net_lt_loss = if lt2 < z { -lt2 } else { z };
    let net_loss = net_st_loss + net_lt_loss;

    // §1211(b) limit + §1212(b) ST-first absorption, character-preserving carryforward (M3).
    let loss_deduction = if net_loss < loss_limit {
        net_loss
    } else {
        loss_limit
    };
    let absorbed_st = if net_st_loss < loss_deduction {
        net_st_loss
    } else {
        loss_deduction
    };
    let absorbed_lt = loss_deduction - absorbed_st;
    CapNet {
        st_net,
        lt_net,
        ordinary_gain,
        preferential_gain,
        loss_deduction,
        st_carry: net_st_loss - absorbed_st,
        lt_carry: net_lt_loss - absorbed_lt,
    }
}

/// §B.3/§B.4 ASSEMBLY: compute the crypto-attributable federal tax for `year`.
///
/// Returns `Computed(TaxResult)` or `NotComputable(Blocker)`. Refusal precedence is deterministic:
/// (1) ANY unresolved `severity()==Hard` blocker anywhere in the projection → `TaxYearNotComputable`;
/// (2) no bundled table for `year` → `TaxTableMissing`; (3) no `TaxProfile` for `year` → `TaxProfileMissing`.
///
/// **Incremental delta (I5).** The objective `total_federal_tax_attributable` is the federal tax the crypto
/// items *cause*: `tax(profile WITH app-computed crypto) − tax(profile WITHOUT)`, ceteris paribus on the
/// minimal profile. `net_1222` is run twice (with/without crypto), each scenario priced through the §1
/// ordinary stack + §1(h) preferential stack + §1411 NIIT, and the scenarios are subtracted. By field:
/// `ltcg_tax`, `niit`, and `total_federal_tax_attributable` are crypto-attributable **deltas**; `st_net`,
/// `lt_net`, `ordinary_from_crypto`, `loss_deduction`, `carryforward_out`, and `marginal_rates` describe the
/// **WITH-crypto** filing position (`carryforward_out` MUST be a level — it feeds next year's `carryforward_in`).
///
/// **Double-count guard (I5).** `profile.ordinary_taxable_income` EXCLUDES all app-computed crypto items;
/// crypto ordinary income (mining/staking/etc.) is added onto the ordinary stack exactly **once** (in the WITH
/// scenario's `bottom`) and is reported back as `ordinary_from_crypto`.
///
/// **§1411 (B-M1).** NII is `QD + surviving net capital gains (ST+LT) − the §1211(b)-allowed net capital
/// loss` (≤ $3,000 / $1,500 MFS — matching Form 8960 line 5a / §1.1411-4(d)). Crypto **ordinary** income
/// (mining/staking/airdrops/rewards) is in MAGI but correctly EXCLUDED from NII (SE income per §1411(c)(6),
/// or non-NII "other income"); the ONLY residual understatement is crypto-lending **interest**
/// (§1411(c)(1)(A)(i)), which the minimal model cannot yet isolate — a Phase-2 refinement. NIIT =
/// 3.8% × max(0, min(NII, MAGI − threshold)) (floored at $0; never negative). MAGI gets only the crypto
/// **delta** added (so the non-crypto QD/cap-gain already in `magi_excluding_crypto` is never double-counted).
///
/// The pinned identity `total == (ord_with − ord_without) + ltcg_tax + niit` holds exactly (all cent-rounded
/// Decimal sums; no float).
///
/// **`events` parameter:** accepted for projection-source symmetry with `score_assignment` (the
/// optimizer's clone-fold-score path), which passes the original event slice through to every
/// `compute_tax_year` call so callers share a single signature. The refusal gate reads
/// `state.blockers` (not `events`) — `events` is not consulted in this function.
pub fn compute_tax_year(
    events: &[LedgerEvent],
    state: &LedgerState,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
) -> TaxOutcome {
    let _ = events;

    // (1) §B.4 refusal (B-I1): ANY unresolved severity()==Hard blocker ANYWHERE in the projection gates
    // EVERY year — deliberately conservative. An out-of-year unresolved `ImportConflict`/`DecisionConflict`
    // can leave a disputed-basis lot that a later disposal consumes (the lot is not `basis_pending`, so it
    // never re-triggers an in-year `FmvMissing`); any open Hard blocker means the basis foundation is unsound,
    // so B refuses rather than present an authoritative-but-wrong number.
    if let Some(b) = first_hard_blocker(state) {
        let evt = b
            .event
            .as_ref()
            .map(|e| e.canonical())
            .unwrap_or_else(|| "-".into());
        return TaxOutcome::NotComputable(Blocker {
            kind: BlockerKind::TaxYearNotComputable,
            event: b.event.clone(), // B-N1: carry the structured offending EventId for downstream C
            detail: format!(
                "year {year} not computable: unresolved Hard blocker [{:?}] {} :: {}",
                b.kind, evt, b.detail
            ),
        });
    }
    // (2) §B.2 missing table.
    let Some(table) = tables.table_for(year) else {
        return TaxOutcome::NotComputable(Blocker {
            kind: BlockerKind::TaxTableMissing,
            event: None,
            detail: format!("no bundled tax table for {year}"),
        });
    };
    // (3) §B.1 missing profile.
    let Some(profile) = profile else {
        return TaxOutcome::NotComputable(Blocker {
            kind: BlockerKind::TaxProfileMissing,
            event: None,
            detail: format!("no tax_profile set for {year}"),
        });
    };

    let status = profile.filing_status;
    let limit = loss_limit(status);
    let sched = table.ordinary_for(status);
    let bp = *table.ltcg_for(status);
    let thr = niit_threshold(status);

    // ── crypto inputs for the year (this year's disposals, filtered by tax-year) ────────────────────
    let mut crypto_st = Usd::ZERO;
    let mut crypto_lt = Usd::ZERO;
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in &d.legs {
            match leg.term {
                Term::ShortTerm => crypto_st += leg.gain,
                Term::LongTerm => crypto_lt += leg.gain,
            }
        }
    }
    // Crypto ordinary income (mining/staking/interest/airdrop/reward): every IncomeKind is ordinary at FMV.
    let crypto_ord: Usd = state
        .income_recognized
        .iter()
        .filter(|i| i.recognized_at.year() == year)
        .map(|i| i.usd_fmv)
        .sum();

    let cf = profile.capital_loss_carryforward_in;
    // ── two scenarios: §1222 netting WITH and WITHOUT crypto ───────────────────────────────────────
    let with = net_1222(
        crypto_st,
        crypto_lt,
        profile.other_net_capital_gain,
        cf.short,
        cf.long,
        limit,
    );
    let without = net_1222(
        Usd::ZERO,
        Usd::ZERO,
        profile.other_net_capital_gain,
        cf.short,
        cf.long,
        limit,
    );

    let qd = profile.qualified_dividends_and_other_pref_income;
    // §1 ordinary stack: crypto ordinary income + surviving net ST gain on top of profile ordinary income,
    // less the §1211 loss deduction. Crypto ordinary income is added EXACTLY ONCE (WITH scenario only).
    let bottom_with =
        profile.ordinary_taxable_income + crypto_ord + with.ordinary_gain - with.loss_deduction;
    let bottom_without =
        profile.ordinary_taxable_income + without.ordinary_gain - without.loss_deduction;
    let ord_with = ordinary_tax_on(sched, bottom_with);
    let ord_without = ordinary_tax_on(sched, bottom_without);
    // §1(h): QD shares the 0/15/20 preferential stack with surviving net capital gain.
    let pref_with = preferential_tax(&bp, bottom_with, qd + with.preferential_gain).tax;
    let pref_without = preferential_tax(&bp, bottom_without, qd + without.preferential_gain).tax;

    // §1411 NIIT. NII = QD + surviving net capital gains (ST+LT) − the §1211(b)-allowed net capital loss
    // (B-M1). Per Form 8960 line 5a / §1.1411-4(d) (Example 1), a net capital loss reduces NII by ONLY the
    // §1211-limited amount (≤ $3,000 / $1,500 MFS) — not by wiping out other-category gains. In a gain year
    // `loss_deduction == 0`, so this is a no-op; in a net-loss year the surviving gains are 0, so NII becomes
    // `qd − loss_deduction`. Crypto ORDINARY income (mining/staking/airdrops/rewards) is correctly EXCLUDED
    // from NII — SE income per §1411(c)(6), or non-NII "other income"; the ONLY residual understatement is
    // crypto-lending INTEREST (NII under §1411(c)(1)(A)(i)), which the minimal model cannot yet isolate from
    // other `crypto_ord` — a Phase-2 refinement. NIIT = 3.8% × max(0, min(NII, MAGI − threshold)).
    let nii_with = qd + with.ordinary_gain + with.preferential_gain - with.loss_deduction;
    let nii_without =
        qd + without.ordinary_gain + without.preferential_gain - without.loss_deduction;
    // `magi_excluding_crypto` already includes QD + non-crypto cap gain; add ONLY the crypto AGI delta.
    let crypto_agi = (with.ordinary_gain + with.preferential_gain - with.loss_deduction)
        - (without.ordinary_gain + without.preferential_gain - without.loss_deduction)
        + crypto_ord;
    let magi_without = profile.magi_excluding_crypto;
    let magi_with = magi_without + crypto_agi;
    let niit = |nii: Usd, magi: Usd| -> Usd {
        let over = if magi > thr { magi - thr } else { Usd::ZERO };
        let capped = if nii < over { nii } else { over };
        // D2: floor the base at $0 — a negative NII (e.g. `qd < loss_deduction`) must NEVER produce a
        // negative/refundable NIIT. NIIT = 3.8% × max(0, min(NII, MAGI − threshold)).
        let base = if capped > Usd::ZERO {
            capped
        } else {
            Usd::ZERO
        };
        round_cents(base * NIIT_RATE)
    };
    let niit_with = niit(nii_with, magi_with);
    let niit_without = niit(nii_without, magi_without);

    // Incremental delta (I5): tax WITH crypto − tax WITHOUT. Exact (cent-rounded Decimal arithmetic).
    let total = (ord_with + pref_with + niit_with) - (ord_without + pref_without + niit_without);

    let top = bottom_with + qd + with.preferential_gain;
    let marginal_rates = MarginalRates {
        ordinary: marginal_ordinary_rate(sched, bottom_with),
        ltcg: if top <= bp.max_zero {
            Usd::ZERO
        } else if top <= bp.max_fifteen {
            dec_15()
        } else {
            dec_20()
        },
        niit_applies: niit_with > niit_without,
    };

    TaxOutcome::Computed(TaxResult {
        st_net: with.st_net,
        lt_net: with.lt_net,
        ordinary_from_crypto: crypto_ord,
        ltcg_tax: pref_with - pref_without, // crypto-attributable preferential tax (DELTA)
        niit: niit_with - niit_without,     // crypto-attributable §1411 NIIT (DELTA)
        loss_deduction: with.loss_deduction, // WITH-scenario level (drives carryforward_out)
        carryforward_out: Carryforward {
            short: with.st_carry,
            long: with.lt_carry,
        },
        total_federal_tax_attributable: total, // = (ord_with−ord_without) + ltcg_tax + niit
        marginal_rates,
    })
}

/// Highest ordinary bracket rate the income reaches (the rate on its last dollar). Display-only (B-M4):
/// uses `taxable > br.lower`, so exactly at a bracket boundary it reports the LOWER bracket's rate.
fn marginal_ordinary_rate(sched: &OrdinarySchedule, taxable: Usd) -> Usd {
    let mut r = Usd::ZERO;
    for br in &sched.brackets {
        if taxable > br.lower {
            r = br.rate;
        } else {
            break;
        }
    }
    r
}

/// M4: compare the declared carryforward-in for a year against the prior year's computed
/// carryforward-out. Returns a human warning when they differ; `None` when they match or the
/// prior year is unavailable. Non-gating advisory — caller must never use this to block
/// computation or change the exit code (Task 10 / B.5).
pub fn carryforward_consistency(
    prior_out: Option<&Carryforward>,
    this_in: &Carryforward,
) -> Option<String> {
    match prior_out {
        Some(p) if p != this_in => Some(format!(
            "carryforward_in (short {} / long {}) does not match prior-year carryforward_out \
             (short {} / long {}) — verify your prior return",
            this_in.short, this_in.long, p.short, p.long
        )),
        _ => None,
    }
}

/// §B.4 (B-I1): the projection-wide Hard-blocker gate. Returns the FIRST unresolved blocker whose
/// `severity() == Severity::Hard`, anywhere in `state.blockers`. `state.blockers` is in deterministic
/// projection order (NFR4) and `.find` returns the first, so the chosen blocker — hence the
/// `TaxYearNotComputable` detail/event — is deterministic.
fn first_hard_blocker(state: &LedgerState) -> Option<&Blocker> {
    state
        .blockers
        .iter()
        .find(|b| b.kind.severity() == Severity::Hard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::{LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule};
    use rust_decimal_macros::dec;

    fn sched() -> OrdinarySchedule {
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.10),
                },
                OrdinaryBracket {
                    lower: dec!(10000),
                    rate: dec!(0.20),
                },
                OrdinaryBracket {
                    lower: dec!(40000),
                    rate: dec!(0.30),
                },
            ],
        }
    }

    #[test]
    fn ordinary_tax_sums_marginal_brackets_exactly() {
        // 0 → 0
        assert_eq!(ordinary_tax_on(&sched(), dec!(0)), dec!(0.00));
        // exactly at a boundary: $10,000 all at 10% = $1,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(10000)), dec!(1000.00));
        // $25,000 = 10%·10,000 + 20%·15,000 = 1,000 + 3,000 = 4,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(25000)), dec!(4000.00));
        // into the open-ended top: $50,000 = 1,000 + 20%·30,000(=6,000) + 30%·10,000(=3,000) = 10,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(50000)), dec!(10000.00));
    }

    fn bp() -> LtcgBreakpoints {
        LtcgBreakpoints {
            max_zero: dec!(48350),
            max_fifteen: dec!(533400),
        }
    }

    #[test]
    fn preferential_zero_then_fifteen() {
        // bottom 40,000 ordinary, pref 20,000 LT → 8,350 @ 0%, 11,650 @ 15% = 1,747.50
        let s = preferential_tax(&bp(), dec!(40000), dec!(20000));
        assert_eq!(s.at_0, dec!(8350));
        assert_eq!(s.at_15, dec!(11650));
        assert_eq!(s.at_20, dec!(0));
        assert_eq!(s.tax, dec!(1747.50));
    }

    #[test]
    fn preferential_fifteen_then_twenty() {
        // bottom 500,000 ordinary, pref 100,000 → 33,400 @ 15% + 66,600 @ 20% = 5,010 + 13,320 = 18,330.00
        let s = preferential_tax(&bp(), dec!(500000), dec!(100000));
        assert_eq!(s.at_0, dec!(0));
        assert_eq!(s.at_15, dec!(33400));
        assert_eq!(s.at_20, dec!(66600));
        assert_eq!(s.tax, dec!(18330.00));
    }

    #[test]
    fn preferential_all_zero_when_under_max_zero() {
        // bottom 10,000, pref 20,000, top 30,000 < 48,350 → all 0%
        let s = preferential_tax(&bp(), dec!(10000), dec!(20000));
        assert_eq!(s.at_0, dec!(20000));
        assert_eq!(s.tax, dec!(0.00));
    }

    #[test]
    fn preferential_zero_pref_is_zero_tax() {
        assert_eq!(
            preferential_tax(&bp(), dec!(100000), dec!(0)).tax,
            dec!(0.00)
        );
    }
}

#[cfg(test)]
mod net_tests {
    use super::*;
    use rust_decimal_macros::dec;
    fn lim() -> Usd {
        dec!(3000)
    }

    #[test]
    fn both_gains_no_crossnet() {
        let n = net_1222(dec!(5000), dec!(8000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.ordinary_gain, dec!(5000));
        assert_eq!(n.preferential_gain, dec!(8000));
        assert_eq!(n.loss_deduction, dec!(0));
    }

    #[test]
    fn within_character_then_crossnet_order() {
        // ST gain 10,000; LT loss 4,000 → LT loss offsets ST gain → net ST gain 6,000, no preferential.
        let n = net_1222(dec!(10000), dec!(-4000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.st_net, dec!(10000));
        assert_eq!(n.lt_net, dec!(-4000));
        assert_eq!(n.ordinary_gain, dec!(6000));
        assert_eq!(n.preferential_gain, dec!(0));
        assert_eq!(n.loss_deduction, dec!(0));
    }

    #[test]
    fn st_loss_offsets_lt_gain_to_preferential() {
        // ST loss 3,000; LT gain 9,000 → net capital gain 6,000 (preferential), no ordinary.
        let n = net_1222(dec!(-3000), dec!(9000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.ordinary_gain, dec!(0));
        assert_eq!(n.preferential_gain, dec!(6000));
    }

    #[test]
    fn loss_year_3k_limit_st_first_carryforward() {
        // ST loss 5,000; LT loss 2,000 → total loss 7,000; deduct 3,000 (ST-first); carry 2,000 ST + 2,000 LT.
        let n = net_1222(dec!(-5000), dec!(-2000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.loss_deduction, dec!(3000));
        assert_eq!(n.st_carry, dec!(2000)); // §1212(b): the $3k came out of ST loss first
        assert_eq!(n.lt_carry, dec!(2000));
    }

    #[test]
    fn loss_limit_is_mfs_1500() {
        let n = net_1222(dec!(-5000), dec!(0), dec!(0), dec!(0), dec!(0), dec!(1500));
        assert_eq!(n.loss_deduction, dec!(1500));
        assert_eq!(n.st_carry, dec!(3500));
        assert_eq!(n.lt_carry, dec!(0));
    }

    #[test]
    fn multi_year_carryforward_preserves_character() {
        // Year 1: ST loss 5,000 + LT loss 2,000 → carry {short:2000, long:2000} (from prior test).
        let y1 = net_1222(dec!(-5000), dec!(-2000), dec!(0), dec!(0), dec!(0), lim());
        // Year 2: LT gain 10,000, no crypto ST; carry-in {short:2000, long:2000}.
        // st_net = 0 - 2000 = -2000; lt_net = 10000 - 2000 = 8000; cross-net: ST loss offsets LT gain →
        // preferential 6,000, no loss.
        let y2 = net_1222(
            dec!(0),
            dec!(10000),
            dec!(0),
            y1.st_carry,
            y1.lt_carry,
            lim(),
        );
        assert_eq!(y2.preferential_gain, dec!(6000));
        assert_eq!(y2.ordinary_gain, dec!(0));
        assert_eq!(y2.loss_deduction, dec!(0));
    }

    #[test]
    fn st_loss_only_3k_all_st_character() {
        let n = net_1222(dec!(-10000), dec!(0), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.loss_deduction, dec!(3000));
        assert_eq!(n.st_carry, dec!(7000));
        assert_eq!(n.lt_carry, dec!(0));
    }
}

/// M4 carryforward-consistency unit tests (Task 10).
#[cfg(test)]
mod consistency_tests {
    use super::*;
    use crate::tax::types::Carryforward;
    use rust_decimal_macros::dec;

    /// A carryforward_in that matches prior carryforward_out → no warning (None).
    #[test]
    fn carryforward_match_is_silent() {
        let c = Carryforward {
            short: dec!(2000),
            long: dec!(2000),
        };
        assert_eq!(carryforward_consistency(Some(&c), &c), None);
    }

    /// A carryforward_in that differs from prior carryforward_out → warning containing "does not match".
    #[test]
    fn carryforward_mismatch_warns() {
        let prior = Carryforward {
            short: dec!(2000),
            long: dec!(2000),
        };
        let declared = Carryforward {
            short: dec!(0),
            long: dec!(2000),
        };
        assert!(carryforward_consistency(Some(&prior), &declared)
            .unwrap()
            .contains("does not match"));
    }

    /// No prior-year data (prior_out == None) → None regardless of this_in.
    #[test]
    fn no_prior_is_silent() {
        assert_eq!(
            carryforward_consistency(None, &Carryforward::default()),
            None
        );
    }
}
