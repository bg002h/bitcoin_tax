//! **The §9 oracle-sweep harness** (T7) — a small UNPUBLISHED test-only binary that assembles, fills,
//! and reads a scenario BACK OFF THE PAPER, plus a `--check` mode that runs the `oracle_diff`
//! reproduction + classification in Rust.
//!
//! It exists so the two downstream drivers never re-implement btctax's printed arithmetic in Python:
//!   - the covering-array corpus generator's D-2 **refusal-free admission** (T10) pipes each candidate
//!     through DEFAULT mode and rejects any that `refused` (an AMT screen, an unmodeled input, a form
//!     that will not fill);
//!   - the live sweep (T12) drives `--check` for BOTH btctax's on-paper values AND the per-line
//!     verdict — so it never re-implements `round_dollar` (Python's `round()` is banker's and drifts on
//!     `.50`), the Tax Table, or the QDCGT worksheet.
//!
//! ## Contract
//!
//! **DEFAULT mode** — reads a `GoldenInputs`-shaped JSON on stdin. Assembles btctax's SAME return the
//! golden matrix fills (`build_golden_return`, so it is identical by construction), runs the fail-closed
//! refuse screens, fills the packet, and reads the whole line set back with `extract_lines`. Prints:
//!
//! ```json
//! { "refused": false, "lines": { "1040.line11": "62000", "schedule_se.line12": "..." } }
//! ```
//!
//! `"refused": true` (with no `lines`) ⇒ the scenario is out of the sweep's domain: a refuse screen
//! fired (AMT / unmodeled input / QBI-over-threshold), the identity would not print, or a member filler
//! refused. That is the D-2 signal.
//!
//! **`--check` mode** — reads a whole `GoldenHousehold`-shaped JSON (inputs + BOTH oracles' figures) on
//! stdin, assembles+fills+reads-back, then reproduces btctax's §3.1 printing on each oracle's figures
//! and classifies every divergence through the SAME `oracle_diff` helpers (`round_leaf`, `sum_round`,
//! `table_l16`, `stacking_ok`) the compute- and paper-level golden tests use. Prints:
//!
//! ```json
//! { "refused": false, "all_reconciled": true,
//!   "reproduced_ops": { "status": "Single", "ti": "47400", "qd_l3a": "0", "net_ltcg_qd_excl": "0" },
//!   "verdicts": [ { "line": "1040.line11", "label": "AGI (1040 L11)", "on_paper": "62000",
//!                  "internal": "62000", "oracles": { "OTS": "62000", "taxcalc": "62000" },
//!                  "ots": "62000", "taxcalc": "62000",
//!                  "reconciled": true, "class": "agree-both" } ] }
//! ```
//!
//! ★ **`oracles` is the authoritative witness field** (FR-234 D1): one entry per engine that actually
//! spoke about that line, keyed by engine name. A single-engine row carries exactly one entry and an
//! `engine` field naming it. The flat `ots` / `taxcalc` columns are a projection of that map kept for
//! `gen_goldens.py` and `sweep.py`; they cannot describe a third engine, and before FR-234 the `ots`
//! column carried EVERY single-engine figure regardless of which engine had spoken. A row whose
//! divergence was absorbed by the lawful Σround≠roundΣ methodology also carries
//! `rounding_order_residual` with the residual's exact size.
//!
//! ## Key convention (documented once, applied everywhere)
//!
//! A flattened key is `<form-segment>.<extract_lines key>`. The form segment is the packet's
//! [`NamedForm::name`] with a single leading `f` stripped **when it is followed by a digit**:
//! `f1040`→`1040`, `f8959`→`8959`, `f1040sa`→`1040sa`. `schedule_d`/`schedule_se` (no leading `f`)
//! pass through unchanged. This is what makes the plan's example key `"1040.line11"` resolve.

use std::collections::BTreeMap;
use std::io::Read;

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::tax::oracle_diff::{
    round_leaf, stacking_ok, sum_round, table_l16, taxcalc_methodology_class, usd, KnownDefect,
    L16Operands,
};
use btctax_core::tax::packet::{assemble_printed_return, PrintedReturn};
use btctax_core::tax::return_1040::{
    assemble_absolute, screen_absolute, screen_compute_dependent, AbsoluteReturn,
};
use btctax_core::tax::return_refuse::screen_inputs;
// The §1401(b)(2) 0.9% rate — the harness reproduces Form 8959's printed lines 7 and 13 from it
// (`other_taxes.rs:170,174`) so the line-18 cross-foot residual comes from the same constant the
// form does, not from a number retyped here.
use btctax_core::tax::tables::SE_RATE_ADDL_MEDICARE;
use btctax_core::tax::testonly::{
    build_golden_return, ty2024_params, ty2024_table, GoldenHousehold, GoldenInputs,
};
use btctax_core::tax::FilingStatus;
use btctax_forms::testonly::extract_lines;
use btctax_forms::{fill_full_return, NamedForm};
use serde_json::{json, Map, Value};

const YEAR: i32 = 2024;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    // ★★★ T11 — `--row-counts` reads a `GoldenInputs` on stdin and prints the DERIVED dependent
    //     counts both engines take (`XTOT`, `n24`, `nu18`, `n1820`, `n21`, `EIC`). It exists so
    //     `check_return.py` never has to re-derive them in Python: Schedule 8812's other-dependent
    //     leg is `ODC_c × max(0, XTOT − childnum − num)`, so a Python `XTOT` that drifted from the
    //     Rust one would silently change the $500-per-dependent credit that the whole line-19
    //     excuse is measured against. One derivation, in the language that owns the row.
    let row_counts = args.iter().any(|a| a == "--row-counts");
    // T7-m2: an OPTIONAL `--known-defect 1040.line16=<value>@<fu-id>` pin (only meaningful with --check).
    let known_defect = match parse_known_defect(&args) {
        Ok(kd) => kd,
        Err(msg) => {
            eprintln!("oracle_harness: {msg}");
            std::process::exit(2);
        }
    };

    let mut stdin = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut stdin) {
        eprintln!("oracle_harness: cannot read stdin: {e}");
        std::process::exit(2);
    }

    let out = if row_counts {
        run_row_counts(&stdin)
    } else if check {
        run_check(&stdin, known_defect.as_ref())
    } else {
        run_default(&stdin)
    };
    // A single line of JSON — the drivers read one object per harness invocation.
    println!("{out}");
}

/// Parse an optional `--known-defect <line>=<btctax_value>@<fu-id>` argument (§10 / T7-m2). It pins
/// btctax's CURRENT (caught-wrong) whole-dollar value on a line against an open `FOLLOWUPS.md` id, so a
/// `--check` run reconciles that line iff btctax still prints the pinned value — the SAME authoritative
/// [`KnownDefect`] the baked corpus declares at promotion (`stacking_ok`, `oracle_diff.rs`). This is the
/// channel the live sweep uses to SUPPRESS an already-filed known defect (so a re-discovery is labelled
/// `known-defect`, not a fresh alarm) instead of hardwiring `None`.
///
/// Only `1040.line16` is supported: it is the sole line with a class / `stacking_ok` path (every other
/// compared line is an exact-match cross-foot or leaf, so a non-L16 pin is a direct value assertion
/// declared in the golden test at promote-time, not a `--check` argument).
fn parse_known_defect(args: &[String]) -> Result<Option<KnownDefect>, String> {
    let Some(pos) = args.iter().position(|a| a == "--known-defect") else {
        return Ok(None);
    };
    let spec = args
        .get(pos + 1)
        .ok_or("--known-defect needs an argument: <line>=<value>@<fu-id>")?;
    let (line, rest) = spec.split_once('=').ok_or_else(|| {
        format!("malformed --known-defect {spec:?}: expected <line>=<value>@<fu-id>")
    })?;
    if line != "1040.line16" {
        return Err(format!(
            "--known-defect only supports 1040.line16 (the only class/stacking line); got {line:?}. \
             A non-L16 pin is declared in the golden test at promotion, not on --check."
        ));
    }
    let (value, fu_id) = rest
        .split_once('@')
        .ok_or_else(|| format!("malformed --known-defect {spec:?}: expected <value>@<fu-id>"))?;
    let value: i64 = value
        .parse()
        .map_err(|_| format!("--known-defect value {value:?} is not a whole-dollar integer"))?;
    if fu_id.is_empty() {
        return Err("--known-defect fu-id must be non-empty".into());
    }
    Ok(Some(KnownDefect {
        // A short-lived one-shot CLI process: leaking the id to `&'static str` (what `KnownDefect` holds
        // for the baked-corpus declarations) is fine and never accumulates.
        fu_id: Box::leak(fu_id.to_string().into_boxed_str()),
        btctax_value: Usd::from(value),
    }))
}

// ── DEFAULT mode ───────────────────────────────────────────────────────────────────────────────────

/// `--row-counts` — the oracle row's DERIVED dependent counts, straight off `GoldenInputs`'s own
/// accessors. See the flag's note in `main`.
fn run_row_counts(stdin: &str) -> Value {
    let i: GoldenInputs = match serde_json::from_str(stdin) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("oracle_harness --row-counts: stdin is not a GoldenInputs row: {e}");
            std::process::exit(2);
        }
    };
    json!({
        "XTOT": i.xtot(),
        "n24": i.n24(),
        "nu18": i.nu18(),
        "n1820": i.n1820(),
        "n21": i.n21(),
        "EIC": i.eic_qualifying_children(),
        // ★ The TAX-CALCULATOR-READY form, not the row's `Option`: taxcalc has no "declined" state
        //   and its aged tests are `age >= 65`, so a declined date of birth is 0 — which forgoes the
        //   §63(f) addition exactly as silence must. Emitting `null` here made the first live
        //   cross-check in `check_return.py` fail on a Single household, with Python saying `0` and
        //   Rust saying `None` about the same absent spouse.
        "age_head": i.age_head.unwrap_or(0),
        "age_spouse": i.age_spouse.unwrap_or(0),
        "blind_head": u8::from(i.blind_head),
        "blind_spouse": u8::from(i.blind_spouse),
    })
}

fn run_default(stdin: &str) -> Value {
    let inputs: GoldenInputs = match serde_json::from_str(stdin) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("oracle_harness: stdin is not a GoldenInputs scenario: {e}");
            std::process::exit(2);
        }
    };
    match assemble(&inputs) {
        None => json!({ "refused": true }),
        Some((_ar, _pr, forms)) => {
            let lines = read_back_lines(&forms);
            let mut map = Map::new();
            for (k, v) in lines {
                map.insert(k, Value::String(v));
            }
            json!({ "refused": false, "lines": Value::Object(map) })
        }
    }
}

// ── `--check` mode: the reproduction + classification, in Rust ───────────────────────────────────────

fn run_check(stdin: &str, known_defect: Option<&KnownDefect>) -> Value {
    let h: GoldenHousehold = match serde_json::from_str(stdin) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("oracle_harness --check: stdin is not a GoldenHousehold scenario: {e}");
            std::process::exit(2);
        }
    };
    let (ar, pr, forms) = match assemble(&h.inputs) {
        None => return json!({ "refused": true }),
        Some(ready) => ready,
    };
    let lines = read_back_lines(&forms);

    // btctax's OWN return figures — the operands `assemble_absolute` feeds `qdcgt_line16` (1040
    // L15 / L3a / QD-exclusive net LTCG). Filing status comes off the printed return.
    let reproduced = L16Operands {
        status: pr.filing_status,
        ti: ar.taxable_income,
        qd_l3a: ar.qualified_dividends,
        net_ltcg_qd_excl: ar.net_ltcg,
    };

    // ── T7-m1: the Part-1 STRUCTURAL witness (SPEC §6.2(b)) ──────────────────────────────────────────
    // `table_l16` on btctax's OWN operands must reproduce btctax's OWN filed regular tax. It holds by
    // construction on a correct build (both sides run `qdcgt_line16` on the same L15/L3a/net-LTCG), so
    // it is a self-consistency check that the `oracle_diff` reproduction seam and the `return_1040`
    // compute path agree — on operand regions the 12 anchors never reach, exactly where the live sweep
    // hunts. A `false` here is a genuine reproduction/Table-semantics signal the sweep must surface.
    let reproduction_ok = table_l16(
        reproduced.status,
        reproduced.ti,
        reproduced.qd_l3a,
        reproduced.net_ltcg_qd_excl,
    ) == ar.regular_tax;

    let e = &h.expected_ots;
    let t = &h.expected_taxcalc;
    // The oracles' OWN L16 operands (baked provenance leaves) — so the per-oracle provenance classes can
    // absorb the §5.1 pinned cells' L16 dissent (bin-edge ⇒ OTS, cents-flip ⇒ taxcalc). `None` pre-bake.
    let ots_ops = oracle_ops(
        pr.filing_status,
        e.taxable_income,
        e.qual_div_l3a,
        e.net_ltcg_qd_exclusive,
    );
    let taxcalc_ops = oracle_ops(
        pr.filing_status,
        t.taxable_income,
        t.qual_div_l3a,
        t.net_ltcg_qd_exclusive,
    );
    let mut verdicts: Vec<Value> = Vec::new();

    // Paper reader off the flattened line map (whole dollars, SPEC §3.1). `None` ⇒ the line is not on
    // this return; a present-but-unparseable cell is a filler/map bug and panics loudly.
    let paper = |key: &str| -> Option<i64> {
        lines.get(key).map(|raw| {
            raw.parse::<i64>().unwrap_or_else(|_| {
                panic!("oracle_harness --check: cell {key:?} is not an integer: {raw:?}")
            })
        })
    };

    // ── AGI L11 / taxable income L15 / QBI deduction L13 — held against BOTH oracles (exact-vs-both). ──
    verdicts.push(verdict_both(
        "1040.line11",
        "AGI (1040 L11)",
        paper("1040.line11"),
        round_dollar(ar.agi),
        e.adjusted_gross_income,
        t.adjusted_gross_income,
    ));
    // Taxable income (L15) — the C1 CROSS-FOOT (AGI − deduction − QBI, each line-rounded from the oracle's
    // own leaves), floored at 0: matches btctax's whole-dollar L15 and dissolves the 8995-chain
    // rounding-order residual. Both oracles stay exact witnesses.
    verdicts.push(verdict_both_targets(
        "1040.line15",
        "taxable income (L15)",
        paper("1040.line15"),
        round_dollar(ar.taxable_income),
        ti_crossfoot(
            e.adjusted_gross_income,
            e.deduction_taken,
            e.qbi_deduction,
            e.taxable_income,
        ),
        ti_crossfoot(
            t.adjusted_gross_income,
            t.deduction_taken,
            t.qbi_deduction,
            t.taxable_income,
        ),
    ));
    // L13 is on every return (0 when there is no QBI): absent-or-present-"0" both mean $0.
    verdicts.push(verdict_both(
        "1040.line13",
        "QBI deduction (L13)",
        Some(paper("1040.line13").unwrap_or(0)),
        round_dollar(ar.qbi_deduction),
        e.qbi_deduction,
        t.qbi_deduction,
    ));

    // ── Tax L16 — the §6.2 two-part: `stacking_ok` absorbs taxcalc's Tax-Table-vs-schedule dissent only
    //    through the methodology class; btctax alone against BOTH oracles with no class FAILS. ──────────
    verdicts.push(verdict_l16(
        "1040.line16",
        "tax (L16)",
        paper("1040.line16"),
        round_dollar(ar.regular_tax),
        e.income_tax_before_credits,
        t.income_tax_before_credits,
        ots_ops.as_ref(),
        taxcalc_ops.as_ref(),
        &reproduced,
        known_defect,
    ));

    // ── AMT L17 (G-6). btctax's figure is Form 6251 line 11; 1040 L17 ⊇ it (v1 has no other Sch 2
    //    Part I item). Both witnesses are OPTIONAL and for DIFFERENT reasons, so the class records
    //    which of them actually spoke:
    //      · OTS reports `None` when its own TY2024 defects reach the household (stale 2023 MFS
    //        §55(d)(3) constants; no §170(b) cash ceiling) — `ots_direct.py` gates and explains it.
    //      · taxcalc reports `None` on goldens predating this comparison, and is separately
    //        KNOWN-SUSPECT for standard-deduction filers (PSLmodels#3108: its AMTI omits Form 6251
    //        line 2a's standard-deduction add-back).
    //    A `None` is "not witnessed", NEVER $0 — coercing it would manufacture agreement.
    verdicts.push(verdict_amt(
        "1040.line17",
        "alternative minimum tax (L17)",
        paper("1040.line17"),
        round_dollar(ar.amt.amt()),
        e.amt,
        t.amt,
    ));

    // ── The C1 cross-foot reproductions, hoisted so L24 INHERITS them (pre-T11 the legs are `None`, so
    //    each falls back to `round_leaf` of the baked per-line total). ─────────────────────────────────
    // ★★★ FR-234 D3 — btctax's OWN exact-cents legs for the two cross-footed lines, so the
    //     rounding-order residual is computed from the mechanism rather than tolerated. Each
    //     constructor PROVES its reproduction against the figure btctax actually printed
    //     (`Crossfoot::reproducing` panics otherwise), so a leg list that drifts from the printed
    //     chain cannot quietly license an excuse.
    //   · Schedule SE L12 = "Add lines 10 and 11" over the PRINTED boxes, whose exact legs are
    //     `se.ss` (L10, §1401(a)) and `se.medicare` (L11, §1401(b)(1)) — `printed.rs:327-329`.
    //   · Form 8959 L18 = "add PRINTED 7 + 13", whose exact legs are 0.9% × the printed line 6 and
    //     0.9% × the printed line 12 — `other_taxes.rs:170,174`. Both operands are already whole
    //     dollars on the paper; the products are not.
    let se_crossfoot = ar.se.as_ref().map(|se| {
        Crossfoot::reproducing(
            &[se.ss, se.medicare],
            pr.forms
                .sch_se
                .as_ref()
                .expect("an SE-tax return prints a Schedule SE")
                .line12,
        )
    });
    let f8959 = &pr.forms.f8959;
    let f8959_crossfoot = Crossfoot::reproducing(
        &[
            SE_RATE_ADDL_MEDICARE * f8959.line6,
            SE_RATE_ADDL_MEDICARE * f8959.line12,
        ],
        f8959.line18,
    );

    // ★ The residual applies to a target built from an engine's exact TOTAL (necessarily roundΣ). Where
    //   the engine publishes its own printed LEGS the comparison is paper-to-paper — both sides Σround
    //   — so there is nothing to absorb and it stays STRICT (`Crossfoot::NONE`).
    let (se_l12_ots, se_l12_ots_xf) = match (e.se_l10_oasdi, e.se_l11_medicare) {
        (Some(l10), Some(l11)) => (sum_round(&[l10, l11]), Crossfoot::NONE),
        _ => (
            round_leaf(e.se_tax),
            se_crossfoot.unwrap_or(Crossfoot::NONE),
        ),
    };
    let (f8959_l18_ots, f8959_l18_ots_xf) = match (e.f8959_l7, e.f8959_l13) {
        (Some(l7), Some(l13)) => (sum_round(&[l7, l13]), Crossfoot::NONE),
        _ => (round_leaf(e.additional_medicare_tax), f8959_crossfoot),
    };

    // ── TOTAL TAX L24 — OTS single-witness cross-foot that inherits SE-L12 / 8959-L18:
    //    `round_leaf(L16) + SE-L12 + 8959-L18 + round_leaf(NIIT)`. line17 (AMT/APTC) and line21
    //    (credits) are read as the precondition (must be 0 for an admitted scenario) and echoed. ────────
    // L16 leg = btctax's OWN FILED L16 (the value summed into printed L24), not the oracle's L16 — the L16
    // VALUE is adjudicated separately by verdict_l16 (with its provenance/methodology class). Keeps L24
    // reconciled on the §5.1 pinned cells while still catching a real cross-foot / Sch-2-leg / L16 bug.
    let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
    let mut l24 = verdict_engine(
        "1040.line24",
        "TOTAL TAX (L24)",
        Engine::Ots,
        paper("1040.line24"),
        pr.forms.f1040.line24,
        l24_target,
        // The target already SUMS printed whole-dollar legs (L16 + SE-L12 + 8959-L18 + NIIT), so both
        // sides are Σround and no rounding-order residual exists here.
        Crossfoot::NONE,
    );
    if let Value::Object(m) = &mut l24 {
        m.insert(
            "precondition_line17".into(),
            json!(paper("1040.line17").unwrap_or(0)),
        );
        m.insert(
            "precondition_line21".into(),
            json!(paper("1040.line21").unwrap_or(0)),
        );
    }
    verdicts.push(l24);

    // ── Schedule SE line 12, Form 8959 line 18, Form 8960 line 17 — the cross-foot reproductions.
    //
    // ★★★ **T11 fold (I-3) — TAX-CALCULATOR IS A WITNESS ON ALL THREE, and used not to be asked.**
    //     Each of these lines was held against OTS alone, and `check_return.py`'s census then told
    //     the filer *"only one engine models this line"* — which is FALSE. `gen_goldens.taxcalc_run`
    //     bakes `se_tax` (`setax`), `additional_medicare_tax` (`ptax_amc`) and `niit`, the script
    //     already holds that dict, and `golden_returns.rs`'s cent-exact block has compared btctax
    //     against BOTH engines on all three across the whole corpus since T7. So the figure was in
    //     hand and the filer was told a divergence there was ambiguous — the inverse of the census's
    //     purpose, and the *"never enumerate the outcomes you happened to see"* rule pointed at the
    //     wrong half.
    //
    // ★ The OTS leg keeps its `sum_round(legs)` cross-foot (the lawful §6102 Σround≠roundΣ residual
    //   is dissolved by summing the oracle's OWN printed legs); taxcalc publishes no legs, so its
    //   leg is `round_leaf(total)` — the same target `golden_returns.rs` holds it to.
    if h.inputs.self_employment_income > 0.0 {
        if let Some(p) = paper("schedule_se.line12") {
            let internal = pr
                .forms
                .sch_se
                .as_ref()
                .expect("an SE household has a printed Schedule SE")
                .line12;
            verdicts.push(verdict_engine(
                "schedule_se.line12",
                "Sch SE L12 (SE tax) [OTS]",
                Engine::Ots,
                Some(p),
                internal,
                se_l12_ots,
                se_l12_ots_xf,
            ));
            verdicts.push(verdict_engine(
                "schedule_se.line12",
                "Sch SE L12 (SE tax) [taxcalc]",
                Engine::Taxcalc,
                Some(p),
                internal,
                round_leaf(t.se_tax),
                // ★★★ FR-234 — taxcalc publishes only the exact TOTAL (`setax`), so its figure is
                //     roundΣ while the filed L12 adds the printed L10/L11 boxes. On 21 of the 107
                //     corpus households those differ by exactly $1 and BOTH are right; this is the
                //     computed mechanism that says so, with no tolerance and no household list.
                se_crossfoot.unwrap_or(Crossfoot::NONE),
            ));
        }
    }

    if let Some(p) = paper("8959.line18") {
        verdicts.push(verdict_engine(
            "8959.line18",
            "8959 L18 (Add'l Medicare) [OTS]",
            Engine::Ots,
            Some(p),
            pr.forms.f8959.line18,
            f8959_l18_ots,
            f8959_l18_ots_xf,
        ));
        verdicts.push(verdict_engine(
            "8959.line18",
            "8959 L18 (Add'l Medicare) [taxcalc]",
            Engine::Taxcalc,
            Some(p),
            pr.forms.f8959.line18,
            round_leaf(t.additional_medicare_tax),
            // Identical two-leg structure to Schedule SE L12: `ptax_amc` is taxcalc's exact total, the
            // filed L18 adds the printed 7 and 13. No corpus household hits the residual there today —
            // which is exactly why it gets the mechanism now rather than after it bites.
            f8959_crossfoot,
        ));
    }

    if let Some(p) = paper("8960.line17") {
        let internal = pr
            .forms
            .f8960
            .as_ref()
            .expect("a NIIT household has a printed Form 8960")
            .line17;
        verdicts.push(verdict_engine(
            "8960.line17",
            "8960 L17 (NIIT) [OTS]",
            Engine::Ots,
            Some(p),
            internal,
            round_leaf(e.niit),
            // Form 8960 L17 is `round_dollar(3.8% x printed operands)` — ONE printed figure, no legs
            // summed on the paper, so no rounding-order residual exists and this stays STRICT.
            Crossfoot::NONE,
        ));
        verdicts.push(verdict_engine(
            "8960.line17",
            "8960 L17 (NIIT) [taxcalc]",
            Engine::Taxcalc,
            Some(p),
            internal,
            round_leaf(t.niit),
            Crossfoot::NONE,
        ));
    }

    // ── Deeper-line rows — oracle-compared only when their `Option` leaf bakes (T11). Each is a no-op on
    //    today's JSON (the leaves are `None`); they light up at the re-bake with no further rewrite. ────
    // Deduction taken (1040 L12) — both oracles.
    if let Some(p) = paper("1040.line12") {
        let internal = round_dollar(ar.deduction);
        if let Some(o) = e.deduction_taken {
            verdicts.push(verdict_engine(
                "1040.line12",
                "deduction (L12) [OTS]",
                Engine::Ots,
                Some(p),
                internal,
                round_leaf(o),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
        if let Some(tc) = t.deduction_taken {
            verdicts.push(verdict_engine(
                "1040.line12",
                "deduction (L12) [taxcalc]",
                Engine::Taxcalc,
                Some(p),
                internal,
                round_leaf(tc),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
    }
    // SALT cap (Schedule A L5e) — both oracles; only when Schedule A files.
    if let Some(p) = paper("1040sa.line5e") {
        let internal = pr
            .forms
            .sch_a
            .as_ref()
            .expect("a Schedule-A household has a printed Schedule A")
            .line5e;
        if let Some(o) = e.salt_capped {
            verdicts.push(verdict_engine(
                "1040sa.line5e",
                "SALT (Sch A L5e) [OTS]",
                Engine::Ots,
                Some(p),
                internal,
                round_leaf(o),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
        if let Some(tc) = t.salt_capped {
            verdicts.push(verdict_engine(
                "1040sa.line5e",
                "SALT (Sch A L5e) [taxcalc]",
                Engine::Taxcalc,
                Some(p),
                internal,
                round_leaf(tc),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
    }
    // Schedule D → 1040 L7 (SIGNED, leading minus) — both oracles; only when line 7 is present.
    if let Some(p) = paper("1040.line7a") {
        let internal = round_dollar(ar.capital_gain);
        if let Some(o) = e.sch_d_to_l7 {
            verdicts.push(verdict_engine(
                "1040.line7a",
                "Sch D -> L7 [OTS]",
                Engine::Ots,
                Some(p),
                internal,
                round_leaf(o),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
        if let Some(tc) = t.sch_d_to_l7 {
            verdicts.push(verdict_engine(
                "1040.line7a",
                "Sch D -> L7 [taxcalc]",
                Engine::Taxcalc,
                Some(p),
                internal,
                round_leaf(tc),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
    }
    // 8995 line 12 (net capital gain cap) — OTS single-witness / WEAK; only when Form 8995 files.
    if let Some(p) = paper("8995.line12") {
        let internal = pr
            .forms
            .f8995
            .as_ref()
            .expect("an 8995 household has a printed Form 8995")
            .line12;
        if let Some(o) = e.qbi_cap_l12 {
            verdicts.push(verdict_engine(
                "8995.line12",
                "8995 L12 net-cap-gain (WEAK)",
                Engine::Ots,
                Some(p),
                internal,
                round_leaf(o),
                // A single printed figure (`round_dollar` of one amount), not a sum of printed
                // legs — there is no rounding-order residual to absorb, so this stays STRICT.
                Crossfoot::NONE,
            ));
        }
    }

    let all_reconciled = verdicts.iter().all(|v| v["reconciled"] == json!(true));

    json!({
        "refused": false,
        "all_reconciled": all_reconciled,
        // T7-m1: the Part-1 structural reproduction witness (see above) — a separate field the sweep
        // checks alongside `all_reconciled`.
        "reproduction_ok": reproduction_ok,
        "reproduced_ops": {
            "status": format!("{:?}", reproduced.status),
            "ti": money(reproduced.ti),
            "qd_l3a": money(reproduced.qd_l3a),
            "net_ltcg_qd_excl": money(reproduced.net_ltcg_qd_excl),
        },
        "verdicts": verdicts,
    })
}

/// ★★★ **FR-234 D1 — the engine a verdict was compared against, as a TYPE.**
///
/// `verdict_engine` used to take the engine as a `&str` and then route its figure into the `ots`
/// JSON column **unconditionally**, ignoring that argument — so a taxcalc-compared row published
/// `"ots": <taxcalc's number>, "taxcalc": null, "engine": "taxcalc"`. That is a defect in what the
/// instrument *claims to have done*, and it is not theoretical: it misled this cycle's controller
/// into briefly concluding OTS was the outlier on Schedule SE line 12, when OTS in fact agrees with
/// btctax and taxcalc is the dissenter.
///
/// ★ **Why an enum and a MAP rather than two fixed columns.** Two hardcoded columns are the
/// [`CLAUDE.md`](../../../CLAUDE.md) *"derive the list, or make the compiler hold it"* shape: the
/// set of engines grows, the columns do not, and a third engine reintroduces exactly this bug. The
/// authoritative field is now `oracles` — a map keyed by the engine that actually spoke, so a third
/// engine appears in it with no further edit and the witness census counts it automatically. The
/// historical `ots` / `taxcalc` columns survive as a PROJECTION of that map through an `_`-free
/// match ([`legacy_columns`]), so they can never disagree with it and a third engine is a build
/// error at the projection rather than a silent omission.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Engine {
    /// OpenTaxSolver (oracle 1).
    Ots,
    /// Tax-Calculator (oracle 2).
    Taxcalc,
}

impl Engine {
    /// The engine's WITNESS name — the spelling the `engine` field carries, the key it takes in the
    /// `oracles` map, and therefore the name `check_return.py`'s per-line witness census counts.
    ///
    /// ★ Deliberately NOT the same spelling as the legacy `ots` column (see [`legacy_columns`]):
    /// the column names are a frozen wire format from before the census existed, and renaming them
    /// would be churn in `gen_goldens.py` / `sweep.py` for no gain.
    fn name(self) -> &'static str {
        match self {
            Engine::Ots => "OTS",
            Engine::Taxcalc => "taxcalc",
        }
    }

    /// The `agree-*` class this engine's single-witness agreement is labelled with. `_`-free, so a
    /// third engine cannot silently inherit `agree-ots` the way the old `if engine == "taxcalc"`
    /// string test did.
    fn agree_class(self) -> &'static str {
        match self {
            Engine::Ots => "agree-ots",
            Engine::Taxcalc => "agree-taxcalc",
        }
    }
}

/// The oracle figures behind ONE verdict, keyed by the engine that spoke. An engine ABSENT from the
/// map did not speak about this line — which is never the same as $0 (the [`verdict_amt`] rule,
/// generalized to every row).
type Oracles = BTreeMap<Engine, Usd>;

/// The historical two fixed columns (`ots`, `taxcalc`), PROJECTED from the authoritative `oracles`
/// map through an `_`-free match. Adding an `Engine` variant is a build error here, which forces a
/// deliberate decision about the legacy shape instead of dropping the new engine on the floor.
///
/// ★ **What this does NOT cover, stated rather than left silent:** a third engine has no column of
/// its own and would appear only in `oracles`. That is exactly why `oracles` is the authoritative
/// field and these two are a compatibility projection.
fn legacy_columns(oracles: &Oracles) -> (Option<Usd>, Option<Usd>) {
    let mut ots = None;
    let mut taxcalc = None;
    for (engine, value) in oracles {
        match engine {
            Engine::Ots => ots = Some(*value),
            Engine::Taxcalc => taxcalc = Some(*value),
        }
    }
    (ots, taxcalc)
}

/// ★★★ **FR-234 D3 — the Σround ≠ roundΣ residual, COMPUTED from the mechanism.**
///
/// A printed TOTAL whose form says *"add lines 10 and 11"* is `Σ round_dollar(exact leg)`: the IRS
/// whole-dollar rule (`design/forms/extract/f4868--2024.txt:217-223`, verbatim — *"If you do round
/// to whole dollars, you must round all amounts"*) puts whole dollars in the LEG boxes, and the
/// total adds the boxes. An engine that publishes only the exact TOTAL for that line necessarily
/// produces `round_dollar(Σ exact leg)` instead. Both are lawful; they differ by
/// `Σ round(leg) − round(Σ leg)`, which is at most a dollar per leg.
///
/// `CLAUDE.md` forbids the two shortcuts that would also make the corpus green: *"An excuse list
/// keyed by VECTOR NAME is a liability"* and *"state the mechanism, let it decide, never enumerate
/// the outcomes you happened to see."* So there is **no tolerance and no household list** here. The
/// residual is computed from btctax's own exact legs, and a divergence is absorbed **iff it equals
/// that residual exactly** — which reduces to *"the engine's rounded exact total equals btctax's
/// rounded exact total"*, a strict equality on a different lawful methodology.
///
/// [`Crossfoot::NONE`] is the honest statement for a line with **no leg structure**: its residual is
/// zero by construction, so [`Crossfoot::absorbs`] can never fire and the comparison stays STRICT
/// with no special case to remember.
#[derive(Clone, Copy, Debug)]
struct Crossfoot {
    /// `Σ round_dollar(leg)` — what the form prints, and therefore what is in the box on the paper.
    sigma_round: Usd,
    /// `round_dollar(Σ leg)` — what an engine publishing only the exact total necessarily produces.
    round_sigma: Usd,
}

impl Crossfoot {
    /// A line with no cross-foot: one printed figure, no legs, zero residual, strict comparison.
    const NONE: Crossfoot = Crossfoot {
        sigma_round: Usd::ZERO,
        round_sigma: Usd::ZERO,
    };

    /// Build the residual from btctax's own EXACT-cents legs, and prove the reproduction by
    /// requiring `Σ round(leg)` to equal the whole-dollar total btctax actually printed.
    ///
    /// ★ The panic is the point: a residual computed from legs that do not reproduce the filed line
    /// is an instrument reporting something other than what it measured, and it would be free to
    /// absorb a real defect. Fail loudly instead — the harness's caller (`smoke.rs`) asserts a
    /// zero exit status, so this cannot pass unnoticed.
    fn reproducing(legs: &[Usd], printed_total: Usd) -> Crossfoot {
        let xf = Crossfoot {
            sigma_round: legs.iter().copied().map(round_dollar).sum(),
            round_sigma: round_dollar(legs.iter().copied().sum()),
        };
        assert_eq!(
            xf.sigma_round, printed_total,
            "oracle_harness --check: the cross-foot legs {legs:?} sum-round to {} but btctax printed \
             {printed_total} — the leg reproduction has drifted from the printed chain, so its \
             rounding-order residual cannot be trusted",
            xf.sigma_round
        );
        xf
    }

    /// `Σ round(leg) − round(Σ leg)`. Zero whenever the line does not cross-foot.
    fn residual(self) -> Usd {
        self.sigma_round - self.round_sigma
    }

    /// Whether this line's rounding-order residual — and **nothing else** — explains the gap
    /// between the paper and a single-total engine's figure.
    ///
    /// Three conjuncts, each load-bearing:
    /// 1. `residual != 0` — a line with no cross-foot (or one whose legs happen to round cleanly)
    ///    absorbs nothing at all, so [`Crossfoot::NONE`] is inert.
    /// 2. `paper == sigma_round` — the FILED figure must be the form's own cross-foot. A filler bug
    ///    that dropped a dollar on the way to the PDF is therefore never absorbed.
    /// 3. `target == round_sigma` — the engine must land exactly on btctax's rounded exact total.
    ///    A divergence of any OTHER size fails, which is what keeps this from being a tolerance.
    fn absorbs(self, paper: Usd, target: Usd) -> bool {
        self.residual() != Usd::ZERO && paper == self.sigma_round && target == self.round_sigma
    }
}

/// A line held against BOTH oracles (`round_leaf` both sides) — AGI / QBI deduction. Reconciled iff the
/// on-paper whole dollars equal each oracle's `round_leaf`. No class absorbs a dissent here.
fn verdict_both(
    line: &str,
    label: &str,
    on_paper: Option<i64>,
    internal: Usd,
    ots: f64,
    taxcalc: f64,
) -> Value {
    let o = round_leaf(ots);
    let tc = round_leaf(taxcalc);
    let p = on_paper.map(Usd::from);
    let reconciled = p == Some(o) && p == Some(tc);
    verdict(
        line,
        label,
        on_paper,
        internal,
        &Oracles::from([(Engine::Ots, o), (Engine::Taxcalc, tc)]),
        reconciled,
        if reconciled { "agree-both" } else { "diverge" },
    )
}

/// A line held against BOTH oracles at PRE-COMPUTED whole-dollar targets (not `round_leaf` of a total) —
/// used for 1040 L15, whose target is the C1 cross-foot [`ti_crossfoot`].
fn verdict_both_targets(
    line: &str,
    label: &str,
    on_paper: Option<i64>,
    internal: Usd,
    ots: Usd,
    taxcalc: Usd,
) -> Value {
    let p = on_paper.map(Usd::from);
    let reconciled = p == Some(ots) && p == Some(taxcalc);
    verdict(
        line,
        label,
        on_paper,
        internal,
        &Oracles::from([(Engine::Ots, ots), (Engine::Taxcalc, taxcalc)]),
        reconciled,
        if reconciled { "agree-both" } else { "diverge" },
    )
}

/// Reproduce btctax's whole-dollar 1040 L15 from an oracle's OWN line-rounded component leaves (C1 table):
/// `round_leaf(AGI) − round_leaf(deduction) − round_leaf(QBI)`, floored at 0 (L15 "if zero or less, enter
/// -0-"). Matches btctax's whole-dollar `L11 − L12 − L13`, so the 8995-chain rounding-order residual never
/// appears. `None` deduction leaf (pre-T11) ⇒ HEAD fallback `round_leaf(total)`.
fn ti_crossfoot(agi: f64, deduction_taken: Option<f64>, qbi_deduction: f64, total: f64) -> Usd {
    match deduction_taken {
        Some(ded) => (round_leaf(agi) - round_leaf(ded) - round_leaf(qbi_deduction)).max(Usd::ZERO),
        None => round_leaf(total),
    }
}

/// An oracle's OWN §1(h) L16 operands, from its baked provenance leaves — `Some` post-T11 so the
/// per-oracle provenance class can witness the §5.1 pinned cells; `None` while a leaf is unbaked.
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

/// Tax L16 — reconciled iff `stacking_ok` accepts it (agree, or a per-oracle provenance / the taxcalc
/// methodology class explains the dissent). The class NAME is diagnostic; `reconciled` is `stacking_ok`'s
/// authoritative verdict, not a re-derivation. `ots_ops`/`taxcalc_ops` are each oracle's OWN baked L16
/// operands, so the provenance classes can witness the §5.1 pinned cells.
#[allow(clippy::too_many_arguments)]
fn verdict_l16(
    line: &str,
    label: &str,
    on_paper: Option<i64>,
    internal: Usd,
    ots16: f64,
    tc16: f64,
    ots_ops: Option<&L16Operands>,
    taxcalc_ops: Option<&L16Operands>,
    reproduced: &L16Operands,
    known_defect: Option<&KnownDefect>,
) -> Value {
    let o = round_leaf(ots16);
    let tc = round_leaf(tc16);
    let both = Oracles::from([(Engine::Ots, o), (Engine::Taxcalc, tc)]);
    let Some(pi) = on_paper else {
        return verdict(line, label, on_paper, internal, &both, false, "absent");
    };
    let p = Usd::from(pi);
    // T7-m2: `known_defect` is threaded through (was hardwired `None`) — a declared §10 pin is
    // authoritative, so a suppressed known defect reconciles while btctax still prints its wrong value.
    let reconciled = stacking_ok(
        p,
        ots16,
        Some(tc16),
        ots_ops,
        taxcalc_ops,
        reproduced,
        known_defect,
    );
    let class = if reconciled && known_defect.is_some_and(|kd| p == kd.btctax_value) {
        "known-defect"
    } else if p == o && p == tc {
        "agree-both"
    } else if reconciled {
        if taxcalc_methodology_class(reproduced) {
            "methodology-taxcalc"
        } else {
            "provenance"
        }
    } else {
        "diverge"
    };
    verdict(line, label, on_paper, internal, &both, reconciled, class)
}

/// The AMT verdict — both witnesses optional, and their absence is MEANINGFUL.
///
/// `class` distinguishes who spoke, so a sweep can tell "both agree" from "only one could look":
/// `agree-both` · `agree-ots` · `agree-taxcalc` · `unwitnessed` · `diverge`. A missing witness never
/// counts toward agreement — that is the whole point of gating OTS on its own known defects rather
/// than letting it vote with a number we know to be wrong.
fn verdict_amt(
    line: &str,
    label: &str,
    on_paper: Option<i64>,
    internal: Usd,
    ots: Option<f64>,
    taxcalc: Option<f64>,
) -> Value {
    let o = ots.map(round_leaf);
    let tc = taxcalc.map(round_leaf);
    let p = on_paper.map(Usd::from).unwrap_or(internal);
    let ots_ok = o.map(|v| v == p);
    let tc_ok = tc.map(|v| v == p);
    let class = match (ots_ok, tc_ok) {
        (Some(true), Some(true)) => "agree-both",
        (Some(true), None) => "agree-ots",
        (None, Some(true)) => "agree-taxcalc",
        (None, None) => "unwitnessed",
        _ => "diverge",
    };
    let reconciled = matches!(class, "agree-both" | "agree-ots" | "agree-taxcalc");
    // A witness that did not speak is ABSENT from the map, never a fabricated $0.
    let mut oracles = Oracles::new();
    if let Some(v) = o {
        oracles.insert(Engine::Ots, v);
    }
    if let Some(v) = tc {
        oracles.insert(Engine::Taxcalc, v);
    }
    verdict(line, label, on_paper, internal, &oracles, reconciled, class)
}

/// A line held against ONE named engine — a cross-foot, a WEAK/NIIT leaf, or one leg of a twin-row
/// pair. `target` is that engine's already-reproduced figure (a `Usd`). Reconciled iff the paper
/// matches.
///
/// ★ T11 fold (N-1) — the ENGINE is a parameter. Every twin-row `[taxcalc]` leg used to be built by
///   a function hardwired to `"agree-ots"`, so a taxcalc row printed `class: agree-ots` and the only
///   thing saying which engine had spoken was a substring of the human label. The verdict now
///   carries an `engine` field, and `check_return.py`'s witness census reads THAT rather than
///   parsing `[taxcalc]` out of a display string.
///
/// ★★★ **FR-234 D1 — the engine is now an [`Engine`], and its figure goes into the column named by
///   it.** The `engine` argument was a `&str` that only ever reached the `class` and the `engine`
///   field: the figure itself was published as `ots` unconditionally, so a taxcalc row read
///   `"ots": <taxcalc's number>, "taxcalc": null`. Fourteen call sites emitted that, and it cost a
///   controller a wrong conclusion about which oracle dissented on Schedule SE line 12.
///
/// ★★★ **FR-234 D3 — `crossfoot` is REQUIRED, not defaulted.** Every call site must state whether
///   its line cross-foots printed legs against an engine that publishes only the exact total, so a
///   NEW compared line cannot inherit either behaviour by silence — adding one without a decision
///   does not compile. [`Crossfoot::NONE`] is the explicit "no leg structure, stay strict".
fn verdict_engine(
    line: &str,
    label: &str,
    engine: Engine,
    on_paper: Option<i64>,
    internal: Usd,
    target: Usd,
    crossfoot: Crossfoot,
) -> Value {
    let p = on_paper.map(Usd::from);
    let exact = p == Some(target);
    // Only a gap that IS this line's rounding-order residual, to the dollar, is absorbed (D3).
    let absorbed = !exact && p.is_some_and(|paper| crossfoot.absorbs(paper, target));
    let reconciled = exact || absorbed;
    let class = if absorbed {
        "methodology-rounding-order"
    } else if reconciled {
        engine.agree_class()
    } else {
        "diverge"
    };
    let mut v = verdict(
        line,
        label,
        on_paper,
        internal,
        &Oracles::from([(engine, target)]),
        reconciled,
        class,
    );
    if let Value::Object(m) = &mut v {
        m.insert("engine".into(), json!(engine.name()));
        // ★ Name the residual's exact SIZE, not just the fact of an excuse (`CLAUDE.md`: taxcalc's
        //   computed excuses each name their omission's size, so a divergence of the wrong shape is
        //   unexpected even on a line expected to diverge).
        if absorbed {
            m.insert(
                "rounding_order_residual".into(),
                json!(money(crossfoot.residual())),
            );
        }
    }
    v
}

/// Assemble one verdict object. Money is emitted as exact whole-dollar TEXT (never a float), so the
/// Python sweep compares strings and never re-rounds.
///
/// `oracles` is the AUTHORITATIVE carrier: one entry per engine that actually spoke about this line,
/// keyed by [`Engine::name`]. The `ots` / `taxcalc` columns beside it are [`legacy_columns`]'
/// projection of that map — emitted for `gen_goldens.py` (which reads `l16["ots"]` / `["taxcalc"]`)
/// and `sweep.py`'s divergence report, and structurally incapable of disagreeing with it.
fn verdict(
    line: &str,
    label: &str,
    on_paper: Option<i64>,
    internal: Usd,
    oracles: &Oracles,
    reconciled: bool,
    class: &str,
) -> Value {
    let (ots, taxcalc) = legacy_columns(oracles);
    let mut witnesses = Map::new();
    for (engine, value) in oracles {
        witnesses.insert(engine.name().into(), json!(money(*value)));
    }
    json!({
        "line": line,
        "label": label,
        "on_paper": on_paper.map(|n| Usd::from(n).to_string()),
        "internal": money(internal),
        "oracles": Value::Object(witnesses),
        "ots": ots.map(money),
        "taxcalc": taxcalc.map(money),
        "reconciled": reconciled,
        "class": class,
    })
}

fn money(u: Usd) -> String {
    u.to_string()
}

// ── The shared assembly + refuse screens (identical to what the golden matrix fills) ─────────────────

/// One assembled return: the exact-cents compute, the §3.1 printed chain, and the filled packet.
type Ready = (AbsoluteReturn, PrintedReturn, Vec<NamedForm>);

/// Build btctax's return from a bare `GoldenInputs`, run the fail-closed refuse screens (input →
/// compute-dependent → absolute, the same chain the CLI export path composes), and fill the packet.
/// `None` ⇒ REFUSED: a refuse screen fired (AMT / unmodeled input / QBI-over-threshold), the identity
/// would not print, or a member filler refused — the D-2 signal. `Some` ⇒ the return btctax will file.
fn assemble(inputs: &GoldenInputs) -> Option<Ready> {
    let (ri, state) = build_golden_return(inputs);
    let params = ty2024_params();
    let table = ty2024_table();

    if screen_inputs(&ri, &table, &params).is_some()
        || screen_compute_dependent(&ri, &state, YEAR, &params).is_some()
    {
        return None;
    }
    let ar = assemble_absolute(&ri, &state, &params, &table, YEAR);
    if screen_absolute(
        &ri,
        &ar,
        &params,
        &state,
        YEAR,
        btctax_core::InformationReturnRegime::NONE /* TY2024 only: no Form 1099-DA regime (spec 1099-DA T0) */,
    )
    .is_some()
    {
        // ★ T5 (r2 I-7): this stays a COMBINED check, deliberately. The plan once said to narrow it to
        // the AMT reason; that was wrong twice over. (a) Deleting or narrowing it would admit
        // QBI-over-threshold and taxable-income≤0 returns the corpus excludes for unrelated reasons.
        // (b) After T3 it is already a no-op for the zero-AMT population — those returns now COMPUTE
        // and are swept. What it still excludes is a household that genuinely owes AMT or must attach
        // Form 6251, which belongs out of domain until Tier 2 can FILE the form.
        return None;
    }
    let pr = assemble_printed_return(
        &ri,
        &state,
        &BTreeMap::new(),
        &ar,
        &table,
        YEAR,
        &[],
        btctax_core::InformationReturnRegime::NONE,
    )
    .ok()?; // identity would not print (D-2)
            // ★ The harness reads PDFs back; statements carry no AcroForm and no compared line, so it takes
            //   the forms half. That is a deliberate narrowing, not an oversight: a statement is prose the
            //   filer attaches, and `read_back_lines` has nothing to read from it.
    let forms = fill_full_return(&pr, YEAR).ok()?.forms; // a member filler refused (overflow etc.)
    Some((ar, pr, forms))
}

// ── Read-back: the whole packet, flattened `<form-segment>.<line> → text` ─────────────────────────────

fn read_back_lines(forms: &[NamedForm]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for f in forms {
        let map = map_for(&f.name).unwrap_or_else(|| {
            // A form with no known map means a member was added to the packet without teaching the
            // harness to read it — fail loud rather than silently drop a line the sweep would compare.
            panic!("oracle_harness: no line-map for packet form {:?}", f.name)
        });
        let cells = extract_lines(&f.bytes, map)
            .unwrap_or_else(|e| panic!("oracle_harness: {} failed to transcribe: {e}", f.name));
        let seg = form_segment(&f.name);
        for (k, v) in cells {
            out.insert(format!("{seg}.{k}"), v);
        }
    }
    out
}

/// The form segment of a flattened key — [`NamedForm::name`] with a single leading `f` stripped when it
/// is followed by a digit (`f1040`→`1040`, `f8959`→`8959`, `f1040sa`→`1040sa`); `schedule_*` unchanged.
fn form_segment(name: &str) -> &str {
    let b = name.as_bytes();
    if b.first() == Some(&b'f') && b.get(1).is_some_and(u8::is_ascii_digit) {
        &name[1..]
    } else {
        name
    }
}

/// The committed 2024 line-map for a packet form (every name [`fill_full_return`] can emit).
fn map_for(name: &str) -> Option<&'static str> {
    Some(match name {
        "f1040" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap()
        }
        "f1040s1" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040s1, 2024).unwrap()
        }
        "f1040s2" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040s2, 2024).unwrap()
        }
        "f1040s3" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040s3, 2024).unwrap()
        }
        "f1040sa" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040sa, 2024).unwrap()
        }
        "f1040sb" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040sb, 2024).unwrap()
        }
        "f1040sc" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040sc, 2024).unwrap()
        }
        "schedule_d" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::ScheduleD, 2024).unwrap()
        }
        "f8949" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8949, 2024).unwrap()
        }
        "schedule_se" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::ScheduleSe, 2024).unwrap()
        }
        // ★ T16 — Form 8889. The harness reads its cells back like any other member; no compared
        //   line lives on it (neither oracle prints a Form 8889), but the read-back is what makes
        //   the packet's own cells visible to the sweep's transcription check.
        "f8889" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8889, 2024).unwrap()
        }
        "f8959" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8959, 2024).unwrap()
        }
        "f8960" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8960, 2024).unwrap()
        }
        "f8995" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8995, 2024).unwrap()
        }
        "f8283" => {
            btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8283, 2024).unwrap()
        }
        _ => return None,
    })
}
