//! **P7 / oracle-sweep T6 — the golden packet round-trip, at the PAPER level.** The last link in the
//! chain, and the only one that touches paper.
//!
//! ★ What this closes.
//!
//! `btctax-core`'s `golden_returns.rs` proves the NUMBERS are right: it diffs btctax against two
//! independent engines — OpenTaxSolver, driven directly, and the PSL Tax-Calculator — over twelve
//! households, adjudicating every divergence by a named CLASS. But an engine that computes a perfect
//! return and then prints it into the wrong box, or drops a form, or leaves a cell blank, files a
//! wrong return with a clean conscience. Every test between the tax and the paper was, until now,
//! checking btctax against btctax.
//!
//! So this fills the **actual PDFs** for the **same twelve households the oracles blessed**, reads the
//! full compared line set BACK OFF THE PAPER with the line-keyed inverse transcriber (`extract_lines`
//! and the §6.3 sign/blank read-back), and holds each figure against BOTH oracles through the SAME
//! `oracle_diff` reproduction helpers and divergence classes the compute level uses. Not btctax's
//! figures — the ORACLES'. The assertion is literally: *the number two independent engines arrived at
//! is the number in the box on the 1040.*
//!
//! ★ Three-way localization (§6.5). The test computes the internal chain anyway (to fill the PDF), so a
//! mismatch is reported three ways — `oracle / btctax-internal / btctax-on-paper`: internal matches the
//! oracle but paper does not ⇒ a FILL/transcription bug; both btctax values differ ⇒ a COMPUTE bug. A
//! fill bug and a compute bug send the next author to different files, so the failure says which.
//!
//! ★ The line set, per the §6.1 / C1 reproduction table.
//!   - **AGI L11, taxable income L15, QBI deduction L13** — cent-exact-equivalent, held against BOTH
//!     oracles (`round_leaf` both sides).
//!   - **Tax L16** — the §6.2 two-part: `table_l16` structurally reproduces btctax's own compute L16,
//!     then `stacking_ok` absorbs taxcalc's Tax-Table-vs-schedule dissent only through the methodology
//!     class. btctax alone against BOTH oracles with no class FAILS.
//!   - **TOTAL TAX L24, Schedule SE L12, Form 8959 L18** — OTS-single-witness CROSS-FOOTS (`sum_round`
//!     of the oracle's own printed legs; taxcalc bundles payroll / exposes only exact totals). Pre-T11
//!     the component legs are `None`, so each falls back to `round_leaf` of the baked per-line total —
//!     the HEAD shape, green on the anchors; the leg form activates when the operands bake at T11. L24
//!     INHERITS the SE-L12 / 8959-L18 reproductions (C1 table) so all three switch together at T11.
//!   - **Form 8960 L17 (NIIT)** — OTS-single-witness `round_leaf(oracle_niit)`, cent-exact on the
//!     anchors, carrying an inherent ±cents epsilon (OTS's L17 is on cents-MAGI) → §10 triage, not a class.
//!   - **deduction L12, SALT L5e, Sch-D→L7, 8995 L12** — deeper rows, read off the paper NOW but
//!     oracle-compared only when their `Option` leaf bakes (T11); inert on today's JSON.
//!
//! ★ Sharded. The differential loop is split across `diff_shard_0..N` `#[test]`s (dispatch by
//! `household_index % N`) so nextest parallelizes it (§8). The byte-reproducibility and identity sweeps
//! stay on the twelve anchors as single tests. N is small now (12 anchors); T11 (~80–120 households)
//! re-measures the wall-clock and may adjust it.
//!
//! ★ The households come from `btctax_core::tax::testonly`, via the shared `tests/common/` builder — one
//! fixture, one packet, so the round-trip fills the SAME taxpayer the oracle validated, never a drift.

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::tax::oracle_diff::{
    provenance_class_fires, round_leaf, stacking_ok, sum_round, table_l16, taxcalc_methodology_class,
    L16Operands, LivenessLedger,
};
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{
    build_golden_household, golden_households, ty2024_params, ty2024_table, GoldenHousehold,
    GoldenInputs,
};
use btctax_core::tax::FilingStatus;
use btctax_forms::testonly::{
    extract_lines, F1040_MAP_2024, F8959_MAP_2024, F8960_MAP_2024, F8995_MAP_2024,
    SCHEDULE_A_MAP_2024, SCHEDULE_C_MAP_2024, SCHEDULE_SE_MAP_2024,
};
use std::collections::{BTreeMap, BTreeSet};

// The `full_return`/`packet`/`form` builders and the §6.3 read-back (`on_paper_signed`/`cell_or_zero`)
// live in the shared `tests/common/mod.rs` — ONE builder, so the P7 round-trip and the oracle-sweep
// read-back fill the SAME households, never a drifting copy.
mod common;
use common::{cell_or_zero, form, full_return, on_paper_signed, packet, Blank, Sign};

fn usd(v: f64) -> Usd {
    Usd::try_from(v).expect("the oracles emit finite figures")
}

/// An oracle's OWN §1(h) line-16 operands, assembled from its baked provenance leaves (`qual_div_l3a`,
/// `net_ltcg_qd_exclusive`, and its exact-cents taxable income). `None` while either leaf is unbaked
/// (pre-T11), which keeps [`provenance_class_fires`] inert then; `Some` post-bake so the per-oracle
/// provenance class can witness the §5.1 pinned cells' paper-L16 dissent.
fn oracle_ops(
    status: FilingStatus,
    taxable_income: f64,
    qual_div_l3a: Option<f64>,
    net_ltcg_qd_exclusive: Option<f64>,
) -> Option<L16Operands> {
    match (qual_div_l3a, net_ltcg_qd_exclusive) {
        (Some(qd), Some(ltcg)) => Some(L16Operands {
            status,
            ti: usd(taxable_income),
            qd_l3a: usd(qd),
            net_ltcg_qd_excl: usd(ltcg),
        }),
        _ => None,
    }
}

/// A PRESENT unsigned money cell, as the SIGNED integer it prints (whole dollars, SPEC §3.1). Panics if
/// the cell is absent (a line the differential compares must be on the paper) or unparseable.
fn paper_money(cells: &BTreeMap<String, String>, key: &str) -> Usd {
    let raw = cells
        .get(key)
        .unwrap_or_else(|| panic!("paper_money: the compared cell {key:?} is absent from the paper"));
    // `parse::<i64>()` would silently accept a leading minus, so a SIGNED line misrouted here would pass
    // unnoticed. Fail loud: an unsigned reader must never read a signed cell (use `on_paper_signed`).
    assert!(
        !raw.starts_with('-'),
        "paper_money: cell {key:?} carries a leading minus ({raw:?}) — read a signed line with \
         on_paper_signed, not paper_money"
    );
    let n: i64 = raw
        .parse()
        .unwrap_or_else(|_| panic!("paper_money: cell {key:?} is present but not an integer: {raw:?}"));
    usd(n as f64)
}

/// **§6.5 three-way localization.** Which of the three disagrees — the oracle, btctax's internal
/// compute, or btctax's paper. Called only on a mismatch (`on_paper != oracle`).
fn localize(on_paper: Usd, internal: Usd, oracle: Usd) -> &'static str {
    if internal == oracle {
        // btctax COMPUTED the oracle's figure and printed something else.
        "LOC=btctax-on-paper (FILL/transcription bug — internal matches the oracle, the paper does not)"
    } else if on_paper == internal {
        // The paper faithfully prints btctax's own differing figure ⇒ the compute is wrong.
        "LOC=btctax-internal (COMPUTE bug — the paper faithfully prints btctax's own differing figure)"
    } else {
        "LOC=btctax-internal+on-paper (both btctax values differ — from the oracle AND from each other)"
    }
}

/// A line held against BOTH oracles (`round_leaf` both sides) — AGI / taxable income / QBI deduction.
#[allow(clippy::too_many_arguments)]
fn check_both(
    wrong: &mut Vec<String>,
    name: &str,
    label: &str,
    why: &str,
    on_paper: Usd,
    internal: Usd,
    ots: f64,
    taxcalc: f64,
) {
    let o = round_leaf(ots);
    let tc = round_leaf(taxcalc);
    if on_paper == o && on_paper == tc {
        return; // both oracles agree with the paper
    }
    // Localize against whichever oracle the paper disagrees with (btctax + OTS agree by design).
    let target = if on_paper != o { o } else { tc };
    wrong.push(format!(
        "  {:<42} {:<26} paper {:>10}  internal {:>10}  OTS {:>10}  taxcalc {:>10}   {}   ({})",
        name,
        label,
        on_paper,
        internal,
        o,
        tc,
        localize(on_paper, internal, target),
        why
    ));
}

/// A line held against BOTH oracles at PRE-COMPUTED whole-dollar targets (not `round_leaf` of a total) —
/// used for 1040 L15, whose target is the C1 cross-foot [`ti_crossfoot`]. Localized three ways like
/// [`check_both`].
#[allow(clippy::too_many_arguments)]
fn check_both_targets(
    wrong: &mut Vec<String>,
    name: &str,
    label: &str,
    why: &str,
    on_paper: Usd,
    internal: Usd,
    ots: Usd,
    taxcalc: Usd,
) {
    if on_paper == ots && on_paper == taxcalc {
        return;
    }
    let target = if on_paper != ots { ots } else { taxcalc };
    wrong.push(format!(
        "  {:<42} {:<26} paper {:>10}  internal {:>10}  OTS {:>10}  taxcalc {:>10}   {}   ({})",
        name,
        label,
        on_paper,
        internal,
        ots,
        taxcalc,
        localize(on_paper, internal, target),
        why
    ));
}

/// Reproduce btctax's whole-dollar 1040 L15 from an oracle's OWN line-rounded component leaves (C1 table):
/// `round_leaf(AGI) − round_leaf(deduction_taken) − round_leaf(qbi_deduction)`. btctax prints L15 as
/// `L11 − L12 − L13` on whole-dollar lines, so this matches its paper L15 EXACTLY — the lawful
/// rounding-order residual (an oracle carrying cents through the 8995 chain, its exact L15 straddling a
/// dollar; r3-M1) never appears, with no tolerance. Pre-T11 the deduction leaf is `None` ⇒ HEAD fallback
/// `round_leaf(total)`.
fn ti_crossfoot(agi: f64, deduction_taken: Option<f64>, qbi_deduction: f64, total: f64) -> Usd {
    match deduction_taken {
        // 1040 L15 is floored at 0 ("if zero or less, enter -0-"): AGI − deduction can go negative for a
        // low-income filer whose deduction exceeds AGI, where btctax prints 0.
        Some(ded) => (round_leaf(agi) - round_leaf(ded) - round_leaf(qbi_deduction)).max(Usd::ZERO),
        None => round_leaf(total),
    }
}

/// A line witnessed by OTS alone (a cross-foot or a WEAK leaf; taxcalc exposes no comparable figure).
fn check_ots(
    wrong: &mut Vec<String>,
    name: &str,
    label: &str,
    why: &str,
    on_paper: Usd,
    internal: Usd,
    ots: Usd,
) {
    if on_paper == ots {
        return;
    }
    wrong.push(format!(
        "  {:<42} {:<26} paper {:>10}  internal {:>10}  OTS {:>10}   {}   ({})",
        name,
        label,
        on_paper,
        internal,
        ots,
        localize(on_paper, internal, ots),
        why
    ));
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
//  The paper-level differential — the shared body, and the shards that parallelize it.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// How many `#[test]` shards the differential loop is split across (§8, r2-M1). The differential runs the
/// FULL corpus (all ~104 households, PDF fill + read-back) — nextest parallelizes the shards across cores,
/// so this is the cheap way to keep the whole-corpus oracle check inside the `make check` budget. **T11
/// raised N 4 → 8** after measuring the 104-household wall-clock (the un-parallelizable whole-corpus
/// property tests — byte-repro / identity / carries — take the §8 SAMPLE instead, since a single `#[test]`
/// cannot be split across cores; see `sample_households`).
///
/// ★ MUST equal `SHARD_WRAPPERS` (the count of `diff_shard_*` `#[test]`s below), enforced at COMPILE time
/// by the `[fn(); SHARDS]` array in `the_shards_partition_every_household` (T6-m1). Raise one and you must
/// raise the other, else the `idx % SHARDS == SHARDS-1` households are silently dropped.
const SHARDS: usize = 8;

/// The number of `diff_shard_*` wrappers that actually run — kept in lockstep with `SHARDS` by the
/// compile-time array guard.
const SHARD_WRAPPERS: usize = 8;

/// ★ **The figures two independent engines computed are the figures in the boxes on the paper.** The
/// per-household body every shard runs: read the full compared line set OFF THE FILLED PDF, hold each
/// line against both oracles via the `oracle_diff` reproduction + classes (§6.1 / C1 table), and FOLD
/// the per-household form-integrity checks (Schedule SE line 12, the 8995 Part-I business row, Schedule
/// C line A, the no-8995 rule, and Attachment Sequence order) onto the SAME fill so they cost nothing
/// extra. Every disagreement is localized three ways (§6.5) into `wrong`.
fn diff_household(h: &GoldenHousehold, wrong: &mut Vec<String>) {
    let a = full_return(h);
    let e = &h.expected_ots;
    let t = &h.expected_taxcalc;

    // Read every form we touch, once. Absent forms read as `None` (not on this return).
    let read = |name: &str, map: &str| -> Option<BTreeMap<String, String>> {
        a.forms.iter().find(|f| f.name == name).map(|f| {
            extract_lines(&f.bytes, map)
                .unwrap_or_else(|err| panic!("{}: the filled {name} must transcribe — {err}", h.name))
        })
    };
    let f1040 = read("f1040", F1040_MAP_2024).expect("every return has a 1040");
    let sch_se = read("schedule_se", SCHEDULE_SE_MAP_2024);
    let f8959 = read("f8959", F8959_MAP_2024);
    let f8960 = read("f8960", F8960_MAP_2024);
    let sch_a = read("f1040sa", SCHEDULE_A_MAP_2024);
    let f8995 = read("f8995", F8995_MAP_2024);
    let sch_c = read("f1040sc", SCHEDULE_C_MAP_2024);

    // ── AGI L11 / taxable income L15 / QBI deduction L13 — held against BOTH oracles ─────────────────
    check_both(
        wrong,
        &h.name,
        "AGI (1040 L11)",
        &h.why,
        paper_money(&f1040, "line11"),
        round_dollar(a.ar.agi),
        e.adjusted_gross_income,
        t.adjusted_gross_income,
    );
    // Taxable income (1040 L15) — the C1 CROSS-FOOT target (AGI − deduction − QBI, each line-rounded from
    // the oracle's own leaves), not `round_leaf(total)` — matches btctax's whole-dollar L15 exactly and
    // dissolves the lawful 8995-chain rounding-order residual. Both oracles stay exact witnesses.
    check_both_targets(
        wrong,
        &h.name,
        "taxable income (L15)",
        &h.why,
        paper_money(&f1040, "line15"),
        round_dollar(a.ar.taxable_income),
        ti_crossfoot(e.adjusted_gross_income, e.deduction_taken, e.qbi_deduction, e.taxable_income),
        ti_crossfoot(t.adjusted_gross_income, t.deduction_taken, t.qbi_deduction, t.taxable_income),
    );
    // QBI deduction (1040 L13 / 8995 L15). Line 13 is on every return (0 when there is no QBI), so read
    // it under AbsentIsZero — present-"0" or absent both mean $0.
    check_both(
        wrong,
        &h.name,
        "QBI deduction (L13)",
        &h.why,
        usd(cell_or_zero(&f1040, "line13", Blank::AbsentIsZero) as f64),
        round_dollar(a.ar.qbi_deduction),
        e.qbi_deduction,
        t.qbi_deduction,
    );

    // ── Tax L16 — the §6.2 two-part (structural reproduction + class stacking), as the compute level ─
    // The reproduced operands are btctax's OWN return figures (1040 L15 / L3a / QD-exclusive net LTCG) —
    // the three args `assemble_absolute` passes to `qdcgt_line16`. Filing status comes off the printed
    // return.
    let reproduced_ops = L16Operands {
        status: a.pr.filing_status,
        ti: a.ar.taxable_income,
        qd_l3a: a.ar.qualified_dividends,
        net_ltcg_qd_excl: a.ar.net_ltcg,
    };
    // Part 1 (structural, Table-semantics witness): the reproduction must recreate btctax's own
    // compute-engine L16 exactly — a drift breaks this before any oracle is consulted.
    assert_eq!(
        table_l16(
            reproduced_ops.status,
            reproduced_ops.ti,
            reproduced_ops.qd_l3a,
            reproduced_ops.net_ltcg_qd_excl,
        ),
        a.ar.regular_tax,
        "{}: oracle_diff::table_l16 must reproduce btctax's own compute-engine L16 exactly",
        h.name
    );
    // Part 2 (class stacking): the ON-PAPER L16 is the figure. Below the Tax-Table ceiling taxcalc's
    // continuous schedule dissents from btctax + OTS; `stacking_ok` absorbs it ONLY through the
    // methodology class (provenance leaves are `None` until T11). btctax alone against both oracles with
    // no class FAILS — the anti-world guard.
    let l16_paper = paper_money(&f1040, "line16");
    let l16_internal = round_dollar(a.ar.regular_tax);
    let ots16 = e.income_tax_before_credits;
    let tc16 = t.income_tax_before_credits;
    // The oracles' OWN L16 operands (baked provenance leaves) — LIVE at T11, so the per-oracle provenance
    // classes can absorb the §5.1 pinned cells' paper-L16 dissent (bin-edge ⇒ OTS, cents-flip ⇒ taxcalc).
    let ots_ops = oracle_ops(
        a.pr.filing_status,
        e.taxable_income,
        e.qual_div_l3a,
        e.net_ltcg_qd_exclusive,
    );
    let taxcalc_ops = oracle_ops(
        a.pr.filing_status,
        t.taxable_income,
        t.qual_div_l3a,
        t.net_ltcg_qd_exclusive,
    );
    if !(l16_paper == round_leaf(ots16) && l16_paper == round_leaf(tc16))
        && !stacking_ok(
            l16_paper,
            ots16,
            Some(tc16),
            ots_ops.as_ref(),
            taxcalc_ops.as_ref(),
            &reproduced_ops,
            None, // no known-defect pin
        )
    {
        let target = if l16_paper != round_leaf(ots16) {
            round_leaf(ots16)
        } else {
            round_leaf(tc16)
        };
        wrong.push(format!(
            "  {:<42} {:<26} paper {:>10}  internal {:>10}  OTS {:>10}  taxcalc {:>10}   {}   \
             (btctax alone — no lawful class absorbs it)",
            h.name,
            "tax (L16)",
            l16_paper,
            l16_internal,
            round_leaf(ots16),
            round_leaf(tc16),
            localize(l16_paper, l16_internal, target),
        ));
    }

    // ── The C1 cross-foot reproductions, hoisted so L24 INHERITS them (C1 table, plan line 47) ───────
    // Schedule SE L12 = `sum_round([L10 OASDI, L11 Medicare])`; Form 8959 L18 = `sum_round([L7, L13])`.
    // Pre-T11 the legs are `None`, so each falls back to `round_leaf` of the baked per-line total — the
    // HEAD shape, numerically identical to the leg form on the anchors. L24 (below) sums THESE SAME
    // reproductions, so when the legs bake at T11 all three switch to `sum_round(legs)` together and the
    // §6102 Σround≠roundΣ residual never resurrects on L24 (it would, with no class to absorb it, if L24
    // instead summed `round(exact total)`).
    let se_l12_ots = match (e.se_l10_oasdi, e.se_l11_medicare) {
        (Some(l10), Some(l11)) => sum_round(&[l10, l11]),
        _ => round_leaf(e.se_tax),
    };
    let f8959_l18_ots = match (e.f8959_l7, e.f8959_l13) {
        (Some(l7), Some(l13)) => sum_round(&[l7, l13]),
        _ => round_leaf(e.additional_medicare_tax),
    };

    // ── TOTAL TAX L24 — OTS-single-witness cross-foot that INHERITS the SE-L12 / 8959-L18 reproductions:
    //    `round_leaf(L16) + SE-L12 + 8959-L18 + round_leaf(NIIT)` (each an already-printed whole-dollar
    //    line, summed — the printed `round(L22)+round(L23)` shape). The precondition (no AMT, no credits)
    //    is read OFF THE PAPER: lines 17 and 21 must be PRESENT-and-"0" (a dropped line fails loudly) —
    //    else the formula would understate the total. ─────────────────────────────────────────────────
    let _ = cell_or_zero(&f1040, "line17", Blank::PresentZero); // Sch 2 L3 (AMT / excess APTC)
    let _ = cell_or_zero(&f1040, "line21", Blank::PresentZero); // L19 + L20 (nonrefundable credits)
    // The L16 leg is btctax's OWN printed L16 (`l16_paper`, read off the form above) — the value btctax
    // actually summed into L24 — NOT the oracle's L16. The L16 VALUE is separately adjudicated by the
    // two-part comparison above (with its provenance/methodology class), so L24 witnesses btctax's
    // cross-foot arithmetic + the SE-L12 / 8959-L18 / NIIT legs against OTS, and stays green on the §5.1
    // pinned OTS-provenance cell (whose L16 legitimately differs from OTS's by the Tax-Table bin the L16
    // class absorbs) while still reddening on any real btctax cross-foot / Sch-2-leg / L16 bug.
    check_ots(
        wrong,
        &h.name,
        "TOTAL TAX (L24)",
        &h.why,
        paper_money(&f1040, "line24"),
        a.pr.forms.f1040.line24, // btctax's printed Σround L24
        l16_paper + se_l12_ots + f8959_l18_ots + round_leaf(e.niit),
    );

    // ── Schedule SE line 12 — the C1 cross-foot reproduction hoisted above. This REPLACES the old
    //    `round(exact se_tax)` comparison. OTS single-witness. ─────────────────────────────────────────
    if h.inputs.self_employment_income > 0.0 {
        if let Some(se) = &sch_se {
            check_ots(
                wrong,
                &h.name,
                "Sch SE L12 (SE tax)",
                &h.why,
                paper_money(se, "line12"),
                a.pr
                    .forms
                    .sch_se
                    .as_ref()
                    .expect("an SE household has a printed Schedule SE")
                    .line12,
                se_l12_ots,
            );
        }
    }

    // ── Form 8959 line 18 — the C1 cross-foot reproduction hoisted above. OTS single-witness (taxcalc's
    //    `ptax_amc` is the exact total). ─────────────────────────────────────────────────────────────
    if let Some(f) = &f8959 {
        check_ots(
            wrong,
            &h.name,
            "8959 L18 (Add'l Medicare)",
            &h.why,
            paper_money(f, "line18"),
            a.pr.forms.f8959.line18,
            f8959_l18_ots,
        );
    }

    // ── Form 8960 line 17 — NIIT. Paper side is `round_leaf(oracle_niit)`, OTS single-witness. It carries
    //    a ±cents epsilon by nature (OTS computes L17 on cents-MAGI, not the printed operands btctax
    //    rounds) → §10 TRIAGE if a corpus scenario surfaces one, NEVER a divergence class. Cent-exact on
    //    the anchors today (btctax's NIIT equals `e.niit`), and the T8 OTS-8960-L5a §1211 fix (r2-I3)
    //    tightens OTS's own NII so the T11 corpus stays tight. ─────────────────────────────────────────
    if let Some(f) = &f8960 {
        check_ots(
            wrong,
            &h.name,
            "8960 L17 (NIIT)",
            &h.why,
            paper_money(f, "line17"),
            a.pr
                .forms
                .f8960
                .as_ref()
                .expect("a NIIT household has a printed Form 8960")
                .line17,
            round_leaf(e.niit),
        );
    }

    // ── Deeper-line rows — read OFF THE PAPER now, oracle-compared only when their `Option` leaf bakes
    //    (T11). Each `if let Some` is a no-op on today's JSON (the keys are absent ⇒ `None`); they light
    //    up at the re-bake without another rewrite. ───────────────────────────────────────────────────
    // Deduction taken (1040 L12) — both oracles.
    {
        let paper = paper_money(&f1040, "line12");
        let internal = round_dollar(a.ar.deduction);
        if let Some(o) = e.deduction_taken {
            check_ots(wrong, &h.name, "deduction (L12) [OTS]", &h.why, paper, internal, round_leaf(o));
        }
        if let Some(tc) = t.deduction_taken {
            check_ots(wrong, &h.name, "deduction (L12) [taxcalc]", &h.why, paper, internal, round_leaf(tc));
        }
    }
    // SALT cap (Schedule A L5e) — both oracles; only when Schedule A files.
    if let Some(sa) = &sch_a {
        let paper = paper_money(sa, "line5e");
        let internal = a
            .pr
            .forms
            .sch_a
            .as_ref()
            .expect("a Schedule-A household has a printed Schedule A")
            .line5e;
        if let Some(o) = e.salt_capped {
            check_ots(wrong, &h.name, "SALT (Sch A L5e) [OTS]", &h.why, paper, internal, round_leaf(o));
        }
        if let Some(tc) = t.salt_capped {
            check_ots(wrong, &h.name, "SALT (Sch A L5e) [taxcalc]", &h.why, paper, internal, round_leaf(tc));
        }
    }
    // Schedule D → 1040 L7 — SIGNED (leading minus, §6.3); both oracles; only when line 7 is present.
    if let Some(paper_signed) = on_paper_signed(&f1040, "line7a", Sign::Leading) {
        let paper = usd(paper_signed as f64);
        let internal = round_dollar(a.ar.capital_gain);
        if let Some(o) = e.sch_d_to_l7 {
            check_ots(wrong, &h.name, "Sch D → L7 [OTS]", &h.why, paper, internal, round_leaf(o));
        }
        if let Some(tc) = t.sch_d_to_l7 {
            check_ots(wrong, &h.name, "Sch D → L7 [taxcalc]", &h.why, paper, internal, round_leaf(tc));
        }
    }
    // 8995 line 12 (net capital gain cap) — OTS single-witness / WEAK (driver-hand-fed; §14.2 closure is
    // a follow-up); only when Form 8995 files.
    if let Some(f) = &f8995 {
        let paper = paper_money(f, "line12");
        let internal = a
            .pr
            .forms
            .f8995
            .as_ref()
            .expect("an 8995 household has a printed Form 8995")
            .line12;
        if let Some(o) = e.qbi_cap_l12 {
            check_ots(wrong, &h.name, "8995 L12 net-cap-gain (WEAK)", &h.why, paper, internal, round_leaf(o));
        }
    }

    // ── FOLD: Form 8995 Part I must NAME the business its line 2 totals (Fable P7 r1 I1) ──────────────
    // With ONE trade or business the column total IS the row, so 1i(c) must equal line 2 exactly, and
    // the business must be named and carry its TIN (the sole proprietor's hyphenated SSN).
    if h.inputs.self_employment_income > 0.0 {
        if let Some(f) = &f8995 {
            let cell = |k: &str| f.get(k).map(String::as_str).unwrap_or("<BLANK>");
            if cell("row1_business") != "Bitcoin mining" {
                wrong.push(format!(
                    "  {:<42} 8995 row 1i(a) must NAME the trade or business, got {:?}",
                    h.name,
                    cell("row1_business")
                ));
            }
            if cell("row1_tin") != "123-45-6789" {
                wrong.push(format!(
                    "  {:<42} 8995 row 1i(b) must be the business TIN (hyphenated SSN), got {:?}",
                    h.name,
                    cell("row1_tin")
                ));
            }
            if cell("row1_qbi") != cell("line2") {
                wrong.push(format!(
                    "  {:<42} 8995 line 2 must equal its one-business column total 1i(c) ({:?} vs {:?})",
                    h.name,
                    cell("line2"),
                    cell("row1_qbi")
                ));
            }
            if cell("line2") == "<BLANK>" {
                wrong.push(format!(
                    "  {:<42} 8995 line 2 must be printed — an SE household has business QBI",
                    h.name
                ));
            }
        }
    }

    // ── FOLD: a filed Schedule C must NAME its business on line A (Fable P7 r3 I1) ────────────────────
    if let Some(sc) = &sch_c {
        let got = sc.get("line_a_business").map(String::as_str);
        if got != Some("Bitcoin mining") {
            wrong.push(format!(
                "  {:<42} Schedule C line A must name the business, got {:?}",
                h.name, got
            ));
        }
    }

    // ── FOLD: a household with no business files NO Form 8995 at all ──────────────────────────────────
    if h.inputs.self_employment_income <= 0.0 && f8995.is_some() {
        wrong.push(format!(
            "  {:<42} no QBI of any kind ⇒ no Form 8995, but the packet carries one",
            h.name
        ));
    }

    // ── FOLD: the packet is stapled in IRS Attachment Sequence order, the 1040 first (r3-N1) ─────────
    if a.forms[0].name != "f1040" {
        wrong.push(format!(
            "  {:<42} the 1040 must sort first, got {:?}",
            h.name, a.forms[0].name
        ));
    } else if a.forms[0].attachment_sequence.is_some() {
        wrong.push(format!(
            "  {:<42} the 1040 carries no Attachment Sequence number of its own",
            h.name
        ));
    }
    let seqs: Vec<&str> = a.forms[1..]
        .iter()
        .map(|f| {
            f.attachment_sequence
                .unwrap_or_else(|| panic!("{}: {} has no attachment sequence", h.name, f.name))
        })
        .collect();
    let mut sorted = seqs.clone();
    sorted.sort_unstable();
    if seqs != sorted {
        wrong.push(format!(
            "  {:<42} packet out of Attachment Sequence order — got {seqs:?}",
            h.name
        ));
    }
}

/// Run the differential over the households this shard owns (`household_index % SHARDS == shard`).
fn run_shard(shard: usize) {
    let households = golden_households();
    let mut wrong: Vec<String> = Vec::new();
    for (idx, h) in households.iter().enumerate() {
        if idx % SHARDS != shard {
            continue;
        }
        diff_household(h, &mut wrong);
    }
    assert!(
        wrong.is_empty(),
        "shard {shard}: the filed packet disagrees with an INDEPENDENT tax engine on {} line(s).\n\
         The return computes correctly and prints something else — the one class of bug every other test \
         in this repo is blind to. Each line is localized three ways (§6.5): oracle / btctax-internal / \
         btctax-on-paper. Do not weaken this test to make it pass.\n\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn diff_shard_0() {
    run_shard(0);
}
#[test]
fn diff_shard_1() {
    run_shard(1);
}
#[test]
fn diff_shard_2() {
    run_shard(2);
}
#[test]
fn diff_shard_3() {
    run_shard(3);
}
#[test]
fn diff_shard_4() {
    run_shard(4);
}
#[test]
fn diff_shard_5() {
    run_shard(5);
}
#[test]
fn diff_shard_6() {
    run_shard(6);
}
#[test]
fn diff_shard_7() {
    run_shard(7);
}

/// ★ **The shards PARTITION every household exactly once.** `SHARDS` and the `diff_shard_*` wrapper
/// count are coupled only by convention; raising `SHARDS` to 5 without adding `diff_shard_4` would
/// silently drop every `idx % 5 == 4` household — the exact silent coverage loss the brief warns
/// against. This guard fails loudly instead: it pins `SHARDS == SHARD_WRAPPERS` AND that the running
/// wrappers cover each of the twelve household indices exactly once.
#[test]
fn the_shards_partition_every_household() {
    // ★ COMPILE-TIME coupling (T6-m1): this array must hold exactly `SHARDS` wrapper fns, so raising
    // `SHARDS` without adding a matching `diff_shard_*` wrapper is a COMPILE error — not merely a runtime
    // mismatch against the `SHARD_WRAPPERS` const. The wrappers are named directly, so the pairing cannot
    // silently decay (the `idx % SHARDS == SHARDS-1` households can never be dropped for want of a runner).
    let _wrappers: [fn(); SHARDS] = [
        diff_shard_0,
        diff_shard_1,
        diff_shard_2,
        diff_shard_3,
        diff_shard_4,
        diff_shard_5,
        diff_shard_6,
        diff_shard_7,
    ];

    assert_eq!(
        SHARDS, SHARD_WRAPPERS,
        "every shard index 0..SHARDS needs a diff_shard_* wrapper — raise SHARDS and SHARD_WRAPPERS \
         together, and add the wrapper"
    );
    let n = golden_households().len();
    let mut covered = vec![0u32; n];
    for shard in 0..SHARD_WRAPPERS {
        for (idx, c) in covered.iter_mut().enumerate() {
            if idx % SHARDS == shard {
                *c += 1;
            }
        }
    }
    assert!(
        covered.iter().all(|&c| c == 1),
        "the running shards do not partition the {n} households exactly once (each must be covered by \
         exactly one wrapper): {covered:?}"
    );
}

/// ★ **Every declared divergence class must ENGAGE — the full liveness sweep (T11).**
///
/// Positive liveness, mirroring the compute level: the taxcalc Tax-Table methodology class fires on the
/// Table anchors, and the two per-oracle L16 PROVENANCE classes fire on the §5.1 pinned cells (bin-edge ⇒
/// `ots_provenance`, cents-flip ⇒ `taxcalc_provenance`), whose oracle L16 leaves now bake. Cheap and
/// NON-sharded: it needs only btctax's own compute operands (`ar`) and the baked oracle leaves, never a
/// filled PDF, so a single test can see the whole corpus and no per-shard split can make it spuriously
/// miss a Table (or pinned) household. The `dead()` sweep deletes any class no household exercises.
#[test]
fn the_paper_differential_engages_every_divergence_class() {
    let params = ty2024_params();
    let table = ty2024_table();
    let mut liveness = LivenessLedger::default();
    for h in &golden_households() {
        let (ri, state) = build_golden_household(h);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        let ops = L16Operands {
            status: ri.filing_status,
            ti: ar.taxable_income,
            qd_l3a: ar.qualified_dividends,
            net_ltcg_qd_excl: ar.net_ltcg,
        };
        let e = &h.expected_ots;
        let t = &h.expected_taxcalc;
        let l16 = round_dollar(ar.regular_tax);
        let ots_ops = oracle_ops(
            ri.filing_status,
            e.taxable_income,
            e.qual_div_l3a,
            e.net_ltcg_qd_exclusive,
        );
        let taxcalc_ops = oracle_ops(
            ri.filing_status,
            t.taxable_income,
            t.qual_div_l3a,
            t.net_ltcg_qd_exclusive,
        );
        // OTS carries a provenance class only: a surviving OTS dissent it witnesses ⇒ fired.
        if l16 != round_leaf(e.income_tax_before_credits)
            && provenance_class_fires(ots_ops.as_ref(), &ops, e.income_tax_before_credits)
        {
            liveness.record_fire("ots_provenance");
        }
        // taxcalc: the methodology class takes precedence (as in `stacking_ok`), else its provenance class.
        if l16 != round_leaf(t.income_tax_before_credits) {
            if taxcalc_methodology_class(&ops) {
                liveness.record_fire("taxcalc_methodology");
            } else if provenance_class_fires(taxcalc_ops.as_ref(), &ops, t.income_tax_before_credits) {
                liveness.record_fire("taxcalc_provenance");
            }
        }
    }
    let dead = liveness.dead(&["taxcalc_methodology", "ots_provenance", "taxcalc_provenance"]);
    assert!(
        dead.is_empty(),
        "declared divergence class(es) never fired and are not pinned: {dead:?}. The Table anchors hold \
         the methodology class live; the two §5.1 pinned cells hold the per-oracle provenance classes live \
         (bin-edge ⇒ ots_provenance, cents-flip ⇒ taxcalc_provenance). A dead class is dead weight — \
         re-derive the pinned cell that should hold it (an engine bump can move the edge), or delete it."
    );
}

// ══════════════════════════ the form set the return requires — DERIVED from inputs ═════════════════

/// **The exact form set a return requires, DERIVED from its inputs alone** — the documented §7 triggers,
/// stated INDEPENDENTLY of the packet assembler and pinned against the twelve hand-audited anchors
/// (`derived_form_set_reproduces_the_twelve_anchors`). A generated corpus (T11) inherits this checked
/// derivation instead of a hand-written map that would go stale the moment a household is added.
///
/// Every clause is a claim about the LAW, mirroring `return_1040`'s own predicates:
///   * Schedule D + Form 8949 file with capital activity (`ScheduleDLines::must_file`).
///   * Schedule C / Schedule SE / Form 8995 (§199A) file for a crypto trade or business.
///   * Schedule B files above the **$1,500** interest OR dividend trigger (`schedule_b_files`; the
///     foreign-account flags are `false` for every golden household).
///   * Schedule A files when the filer itemizes (D-3: itemizing wins whenever there are itemized inputs).
///   * Form 8959 files above the Additional-Medicare threshold — $200k single / $250k MFJ — on Medicare
///     wages + SE income.
///   * Form 8960 (NIIT) files with net investment income AND MAGI over the same threshold (the NIIT and
///     Additional-Medicare thresholds coincide for {Single, MFJ} — the sweep's domain).
///   * Schedule 1 carries the SE household's business income (L3) and ½-SE adjustment (L15).
///   * Schedule 2 carries Part-II other taxes: SE tax, Additional Medicare (8959), or NIIT (8960).
///   * Schedule 3 (credits) is out of the domain (D-1: no credits) — never filed here.
///
/// The Form 8959/8960 bases are modeled at the documented threshold and are FAITHFUL on the anchors, but
/// APPROXIMATE: Form 8959 uses raw `w2 + SE` (it omits the 0.9235 net-SE factor and the SS-wage-base
/// interaction, so an SE-heavy household near $200k could derive a spurious `f8959`); Form 8960
/// approximates MAGI at the AGI level (the ½-SE adjustment, which flips no anchor, is omitted). Neither
/// flips any of the twelve anchors, and T11's whole-corpus `derive_form_set`-vs-assembler check
/// re-validates the derivation over the generated corpus — the safety net if a scenario lands on an edge.
fn derive_form_set(i: &GoldenInputs) -> BTreeSet<&'static str> {
    let mut set: BTreeSet<&'static str> = BTreeSet::new();
    set.insert("f1040"); // every return

    let mfj = i.filing_status == "Married/Joint";
    let se = i.self_employment_income > 0.0;

    // Capital activity ⇒ Schedule D + Form 8949 (the detail its lines 3/10 cite).
    if i.short_term_capital_gains != 0.0 || i.long_term_capital_gains != 0.0 {
        set.insert("schedule_d");
        set.insert("f8949");
    }
    // A crypto trade or business ⇒ Schedule C, Schedule SE, and the §199A Form 8995 + its carriers.
    if se {
        set.insert("f1040sc");
        set.insert("schedule_se");
        set.insert("f8995");
        set.insert("f1040s1"); // business income L3 + ½-SE adjustment L15
    }
    // Schedule B: interest OR dividends over $1,500.
    if i.taxable_interest > 1500.0 || i.ordinary_dividends > 1500.0 {
        set.insert("f1040sb");
    }
    // Schedule A when the filer itemizes (D-3: itemizing wins whenever there are itemized inputs).
    if i.itemized_deductions > 0.0
        || i.state_income_tax > 0.0
        || i.real_estate_tax > 0.0
        || i.mortgage_interest > 0.0
    {
        set.insert("f1040sa");
    }

    let threshold = if mfj { 250_000.0 } else { 200_000.0 };
    // Form 8959 — Additional Medicare Tax on Medicare wages + SE income over the threshold.
    let f8959 = i.w2_income + i.self_employment_income > threshold;
    if f8959 {
        set.insert("f8959");
    }
    // Form 8960 (NIIT) — net investment income AND MAGI over the threshold.
    let net_cap_gain = (i.short_term_capital_gains + i.long_term_capital_gains).max(0.0);
    let nii = i.taxable_interest + i.ordinary_dividends + net_cap_gain;
    let magi =
        i.w2_income + i.taxable_interest + i.ordinary_dividends + net_cap_gain + i.self_employment_income;
    let f8960 = nii > 0.0 && magi > threshold;
    if f8960 {
        set.insert("f8960");
    }
    // Schedule 2 — Part-II other taxes: SE tax, Additional Medicare, or NIIT.
    if se || f8959 || f8960 {
        set.insert("f1040s2");
    }
    set
}

/// ★ **The derivation reproduces the twelve hand-audited anchor sets** (r3-M1). The pinned sets are the
/// same ones the packet was proven to carry; `derive_form_set` must reproduce each from inputs ALONE.
#[test]
fn derived_form_set_reproduces_the_twelve_anchors() {
    // The hand-audited law, pinned per anchor.
    let pinned: BTreeMap<&str, &[&str]> = BTreeMap::from([
        ("single_w2_only_standard", &["f1040"][..]),
        ("mfj_two_w2_standard", &["f1040"][..]),
        ("single_w2_plus_crypto_ltcg", &["f1040", "schedule_d", "f8949"][..]),
        ("single_short_term_crypto_gain", &["f1040", "schedule_d", "f8949"][..]),
        ("single_capital_loss_capped", &["f1040", "schedule_d", "f8949"][..]),
        (
            "single_qdcgt_both_slices",
            &["f1040", "f1040sb", "schedule_d", "f8949"][..],
        ),
        (
            "mfj_itemized_over_100k",
            &["f1040", "f1040s2", "f1040sa", "f1040sb", "schedule_d", "f8949", "f8960"][..],
        ),
        (
            "mfj_high_income_niit_and_addl_medicare",
            &["f1040", "f1040s2", "f1040sb", "schedule_d", "f8949", "f8959", "f8960"][..],
        ),
        ("mfj_itemized_salt_over_the_cap", &["f1040", "f1040sa"][..]),
        (
            "single_crypto_business_se",
            &["f1040", "f1040s1", "f1040s2", "f1040sc", "schedule_se", "f8995"][..],
        ),
        (
            "single_miner_qbi_limited_by_net_capital_gain",
            &["f1040", "f1040s1", "f1040s2", "f1040sb", "f1040sc", "schedule_d", "f8949", "schedule_se", "f8995"][..],
        ),
        (
            "mfj_se_over_the_addl_medicare_threshold",
            &["f1040", "f1040s1", "f1040s2", "f1040sc", "schedule_se", "f8995", "f8959"][..],
        ),
    ]);

    let mut wrong = Vec::new();
    let mut checked = 0;
    for h in &golden_households() {
        // This test pins the 12 hand-audited ANCHOR sets as a regression check. The generated + §5.1
        // pinned corpus households (added at T11) are validated instead by the whole-corpus
        // `each_golden_packet_carries_exactly_the_forms_the_derived_law_requires`; skip anything not in
        // the anchor map rather than demanding a pinned set for all ~104 households.
        let Some(want_set) = pinned.get(h.name.as_str()) else {
            continue;
        };
        checked += 1;
        let want: BTreeSet<&str> = want_set.iter().copied().collect();
        let got: BTreeSet<&str> = derive_form_set(&h.inputs).into_iter().collect();
        if got != want {
            let missing: Vec<_> = want.difference(&got).collect();
            let spurious: Vec<_> = got.difference(&want).collect();
            wrong.push(format!(
                "  {:<42} derived MISSING {missing:?}  SPURIOUS {spurious:?}",
                h.name
            ));
        }
    }
    assert_eq!(
        checked,
        pinned.len(),
        "not every hand-audited anchor was found in the corpus — an anchor was renamed or dropped, so its \
         derivation is going unchecked"
    );
    assert!(
        wrong.is_empty(),
        "derive_form_set disagrees with the hand-audited anchor sets — fix the derivation, not the \
         pinned law:\n{}",
        wrong.join("\n")
    );
}

/// The §8 make-check SAMPLE stride: one in every `SAMPLE_STRIDE` GENERATED households is kept (all
/// anchors + both §5.1 pinned cells are ALWAYS kept). Sized so the un-parallelizable whole-corpus
/// property tests stay inside the `make check` budget; the full corpus runs in their `#[ignore]` twins.
const SAMPLE_STRIDE: usize = 15;

/// The bounded, DETERMINISTIC make-check SAMPLE (§8 runtime fallback). A single `#[test]` — byte
/// reproducibility, the identity sweep, the form-set check — cannot be split across cores the way the
/// sharded differential can, so re-filling all ~104 households in each would blow the `make check` budget.
/// The sample keeps EVERY anchor (they carry every deeper-line / form-set teeth scenario and the
/// methodology class) and BOTH §5.1 pinned cells (the provenance classes), plus every `SAMPLE_STRIDE`-th
/// generated covering-array row for breadth. The DIFFERENTIAL itself still runs the FULL corpus in
/// `make check` (it shards across cores); only these three property tests take the sample, with their
/// `#[ignore]`d `*_whole_corpus` twins covering all ~104 on demand / in CI.
fn sample_households() -> Vec<GoldenHousehold> {
    let mut generated = 0usize;
    golden_households()
        .into_iter()
        .filter(|h| {
            if h.name.starts_with("ca_") {
                let keep = generated.is_multiple_of(SAMPLE_STRIDE);
                generated += 1;
                keep
            } else {
                true // every hand-audited anchor + both §5.1 pinned cells
            }
        })
        .collect()
}

/// The 12 anchors + 2 §5.1 pinned cells (the non-generated households) — every filler and FORM TYPE, and
/// both provenance classes. The "every form type" property tests (byte-reproducibility, the identity
/// sweep) take THIS rather than the wider sample: the generated covering array adds no new serialization
/// path or form type, and re-filling it in a single un-parallelizable `#[test]` would cost budget for no
/// coverage. Their `#[ignore]` twins still run the whole corpus.
fn anchors_and_pinned() -> Vec<GoldenHousehold> {
    golden_households()
        .into_iter()
        .filter(|h| !h.name.starts_with("ca_"))
        .collect()
}

/// ★ **Exactly the forms the DERIVED law requires — no more, no fewer.** The whole-corpus check, now
/// against `derive_form_set` rather than a hand-written map, so it scales to the T11 corpus.
///
/// A DROPPED form understates the return (P6 found Schedule 3 missing its line 10, and a filer billed
/// twice for tax already paid). A SPURIOUS form makes an assertion the filer did not intend: an empty
/// Schedule SE stapled to a W-2 filer's return tells the IRS they had self-employment income.
fn assert_packets_carry_the_derived_forms(households: &[GoldenHousehold]) {
    let mut wrong = Vec::new();
    for h in households {
        let want: BTreeSet<String> = derive_form_set(&h.inputs).iter().map(|s| s.to_string()).collect();
        let got: BTreeSet<String> = packet(h).iter().map(|f| f.name.clone()).collect();
        let missing: Vec<_> = want.difference(&got).collect();
        let spurious: Vec<_> = got.difference(&want).collect();
        if !missing.is_empty() || !spurious.is_empty() {
            wrong.push(format!(
                "  {:<42} MISSING {missing:?}  SPURIOUS {spurious:?}",
                h.name
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "the assembled packet is not the return the DERIVED law requires:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn each_golden_packet_carries_exactly_the_forms_the_derived_law_requires() {
    assert_packets_carry_the_derived_forms(&sample_households());
}

/// The full-corpus twin (§8) — validates `derive_form_set` against the assembler over ALL ~104 households
/// (the safety net if a generated scenario lands on a form-set edge). `#[ignore]`d: run on demand / in CI.
#[test]
#[ignore = "full corpus (~104) — the make-check gate runs the sample; run in CI / on demand"]
fn each_golden_packet_carries_exactly_the_forms_whole_corpus() {
    assert_packets_carry_the_derived_forms(&golden_households());
}

// ══════════════════════════ the packet as an ARTIFACT: identity, determinism, one anchor ═══════════

/// A household with no self-employment must not be handed a Schedule SE — or a Schedule C — at all.
///
/// The packet is assembled from what the return NEEDS. Stapling an empty Schedule SE to a W-2 filer's
/// return is not cosmetic: it asserts to the IRS that they had self-employment income.
#[test]
fn a_w2_only_household_gets_no_schedule_se_and_no_schedule_c() {
    let households = golden_households();
    let h = households
        .iter()
        .find(|h| h.name == "single_w2_only_standard")
        .expect("the floor case is in the matrix");

    let pkt = packet(h);
    let names: Vec<&str> = pkt.iter().map(|f| f.name.as_str()).collect();

    assert!(names.contains(&"f1040"), "every return has a 1040");
    assert!(
        !names.contains(&"schedule_se"),
        "a W-2-only filer has no self-employment tax; the packet must not include Schedule SE. Got: {names:?}"
    );
    assert!(
        !names.contains(&"f1040sc"),
        "a W-2-only filer runs no business; the packet must not include Schedule C. Got: {names:?}"
    );
}

/// Every form in every golden packet carries the filer's name and SSN.
///
/// A schedule that arrives at the IRS without an SSN on it is a loose page. This iterates the WHOLE
/// packet for EVERY household rather than pinning one form — the P6 review found an unnamed Form 8949
/// precisely because a test that checked one form had promised to check all of them.
fn assert_every_form_carries_the_filers_identity(households: &[GoldenHousehold]) {
    // The map key under which each form carries its identity block, and the map to read it with.
    let maps: BTreeMap<&str, &str> = BTreeMap::from([
        ("f1040", F1040_MAP_2024),
        ("f1040s1", btctax_forms::testonly::SCHEDULE_1_MAP_2024),
        ("f1040s2", btctax_forms::testonly::SCHEDULE_2_MAP_2024),
        ("f1040s3", btctax_forms::testonly::SCHEDULE_3_MAP_2024),
        ("f1040sa", SCHEDULE_A_MAP_2024),
        ("f1040sb", btctax_forms::testonly::SCHEDULE_B_MAP_2024),
        ("f1040sc", SCHEDULE_C_MAP_2024),
        ("schedule_d", btctax_forms::testonly::SCHEDULE_D_MAP_2024),
        ("schedule_se", SCHEDULE_SE_MAP_2024),
        ("f8959", F8959_MAP_2024),
        ("f8960", btctax_forms::testonly::F8960_MAP_2024),
        ("f8995", F8995_MAP_2024),
        ("f8949", btctax_forms::testonly::F8949_MAP_2024),
    ]);

    let mut naked: Vec<String> = Vec::new();
    let mut seen = 0;

    for h in households {
        for f in &packet(h) {
            let Some(map) = maps.get(f.name.as_str()) else {
                panic!(
                    "{}: the packet contains {} but this test has no map for it — a new form was added \
                     and the identity check would have silently skipped it",
                    h.name, f.name
                );
            };
            let got = extract_lines(&f.bytes, map).unwrap();
            seen += 1;

            // Forms spell the identity block differently — the 1040 has `header.taxpayer_ssn`, the
            // schedules `identity.ssn`, the 8949 one per page (`identity_page1.ssn`). Match on the
            // SHAPE of the key rather than enumerating them, so a form that invents a fourth spelling
            // still gets checked instead of quietly passing. `extract_lines` only ever returns cells
            // that carry text, so a key being present IS the value being non-empty.
            let key_ends = |suffix: &str| got.keys().any(|k| k.ends_with(suffix));
            let has_ssn = key_ends("ssn");
            let has_name = key_ends("name") || (key_ends("_first") && key_ends("_last"));
            if !has_name || !has_ssn {
                naked.push(format!(
                    "  {:<42} {:<12} name={has_name} ssn={has_ssn}   keys: {:?}",
                    h.name,
                    f.name,
                    got.keys().take(6).collect::<Vec<_>>()
                ));
            }
        }
    }

    assert!(seen > 30, "the sweep must actually see forms, saw {seen}");
    assert!(
        naked.is_empty(),
        "{} form(s) would reach the IRS without a name or an SSN on them:\n{}",
        naked.len(),
        naked.join("\n")
    );
}

#[test]
fn every_form_of_every_golden_packet_carries_the_filers_identity() {
    // The identity property is per FORM TYPE; the anchors + pinned cells carry every form type, so this
    // runs them (not the wider sample) to stay cheap. The whole corpus runs in the `#[ignore]` twin.
    assert_every_form_carries_the_filers_identity(&anchors_and_pinned());
}

/// The full-corpus twin (§8) — the identity sweep over ALL ~104 households. `#[ignore]`d: on demand / CI.
#[test]
#[ignore = "full corpus (~104) — the make-check gate runs the sample; run in CI / on demand"]
fn every_form_carries_the_filers_identity_whole_corpus() {
    assert_every_form_carries_the_filers_identity(&golden_households());
}

/// ★ **The same return fills to the same bytes.** Twice, for every household.
///
/// Each individual filler already pins its own content hash, but nothing until now asserted the
/// property of the PACKET: that `fill_full_return` — which walks a form set, assembles an order, and
/// serializes a dozen documents — is reproducible end to end. Anything non-deterministic that leaked
/// into the output (a hash-map iteration order reaching a page tree, a timestamp, a fresh object id)
/// would show up here and nowhere else. A return you cannot reproduce is a return you cannot attest to.
fn assert_the_whole_packet_is_byte_reproducible(households: &[GoldenHousehold]) {
    for h in households {
        let a = packet(h);
        let b = packet(h);

        assert_eq!(
            a.len(),
            b.len(),
            "{}: the packet changed SIZE between two fills",
            h.name
        );
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name, "{}: form order is not stable", h.name);
            assert_eq!(
                x.attachment_sequence, y.attachment_sequence,
                "{}: {} attachment sequence is not stable",
                h.name, x.name
            );
            assert_eq!(
                x.bytes,
                y.bytes,
                "{}: {} does not fill to the same bytes twice ({} vs {} bytes)",
                h.name,
                x.name,
                x.bytes.len(),
                y.bytes.len()
            );
        }
    }
}

#[test]
fn the_whole_packet_is_byte_reproducible() {
    // Byte-reproducibility is a per-fill DETERMINISM property, and it fills each household TWICE — the
    // heaviest whole-corpus test. The anchors + both pinned cells already exercise every filler and form
    // type, so this runs THEM rather than the wider sample. The full corpus is covered by the `#[ignore]`
    // twin.
    assert_the_whole_packet_is_byte_reproducible(&anchors_and_pinned());
}

/// The full-corpus twin (§8) — byte reproducibility over ALL ~104 households. `#[ignore]`d: on demand / CI.
#[test]
#[ignore = "full corpus (~104) — the make-check gate runs the sample; run in CI / on demand"]
fn the_whole_packet_is_byte_reproducible_whole_corpus() {
    assert_the_whole_packet_is_byte_reproducible(&golden_households());
}

/// ★ **The §164(b)(5) SALT cap is applied ON THE PAPER, not just in the engine.**
///
/// Schedule A line 5e is "the smaller of line 5d or $10,000". This household's state income tax
/// ($1,068) and real estate tax ($10,509) add to **$11,577**, so the cap BINDS: line 5e must print
/// $10,000 and the filer loses $1,577 of deduction. Both independent oracles agree on the resulting
/// taxable income ($3,730), which is what makes this checkable at all.
///
/// The SALT figures are IRS ATS Test Scenario 2's — the only IRS-authored numbers in the matrix. (The
/// scenario itself is NOT a golden return: its 1040 is blank and even Schedule A's computed totals are
/// blank. It is a test-case specification, not an answer key. See FOLLOWUPS `p7-ats-scenario-2`.)
///
/// A cap that is computed but printed uncapped files a return claiming $1,577 of deduction the law
/// does not allow, and every arithmetic test in the repo would still be green.
#[test]
fn the_salt_cap_is_printed_onto_schedule_a() {
    let households = golden_households();
    let h = households
        .iter()
        .find(|h| h.name == "mfj_itemized_salt_over_the_cap")
        .expect("the SALT-cap household is in the matrix");

    let pkt = packet(h);
    let got = extract_lines(&form(&pkt, "f1040sa").bytes, SCHEDULE_A_MAP_2024).unwrap();

    let cell = |k: &str| got.get(k).map(String::as_str).unwrap_or("<BLANK>");

    assert_eq!(cell("line5a"), "1068", "state & local income tax");
    assert_eq!(cell("line5b"), "10509", "real estate tax");
    assert_eq!(cell("line5d"), "11577", "5a + 5b — the UNCAPPED total");
    assert_eq!(
        cell("line5e"),
        "10000",
        "★ the §164(b)(5) cap: 5e is the SMALLER of 5d ($11,577) and $10,000. Printing $11,577 here \
         would claim $1,577 of deduction the law does not allow — and every arithmetic test in this \
         repo would still be green."
    );
    assert_eq!(cell("line8a"), "25000", "mortgage interest");
    assert_eq!(
        cell("line17"),
        "35000",
        "total itemized = the CAPPED $10,000 + $25,000 mortgage. It beats the $29,200 standard \
         deduction, so the cap actually changes this filer's tax."
    );
}
