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
//!                  "internal": "62000", "ots": "62000", "taxcalc": "62000",
//!                  "reconciled": true, "class": "agree-both" } ] }
//! ```
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
    round_leaf, stacking_ok, sum_round, taxcalc_methodology_class, usd, L16Operands,
};
use btctax_core::tax::FilingStatus;
use btctax_core::tax::packet::{assemble_printed_return, PrintedReturn};
use btctax_core::tax::return_1040::{
    assemble_absolute, screen_absolute, screen_compute_dependent, AbsoluteReturn,
};
use btctax_core::tax::return_refuse::screen_inputs;
use btctax_core::tax::testonly::{
    build_golden_return, ty2024_params, ty2024_table, GoldenHousehold, GoldenInputs,
};
use btctax_forms::testonly::{
    extract_lines, F1040_MAP_2024, F8283_MAP_2024, F8949_MAP_2024, F8959_MAP_2024, F8960_MAP_2024,
    F8995_MAP_2024, SCHEDULE_1_MAP_2024, SCHEDULE_2_MAP_2024, SCHEDULE_3_MAP_2024,
    SCHEDULE_A_MAP_2024, SCHEDULE_B_MAP_2024, SCHEDULE_C_MAP_2024, SCHEDULE_D_MAP_2024,
    SCHEDULE_SE_MAP_2024,
};
use btctax_forms::{fill_full_return, NamedForm};
use serde_json::{json, Map, Value};

const YEAR: i32 = 2024;

fn main() {
    let check = std::env::args().skip(1).any(|a| a == "--check");

    let mut stdin = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut stdin) {
        eprintln!("oracle_harness: cannot read stdin: {e}");
        std::process::exit(2);
    }

    let out = if check { run_check(&stdin) } else { run_default(&stdin) };
    // A single line of JSON — the drivers read one object per harness invocation.
    println!("{out}");
}

// ── DEFAULT mode ───────────────────────────────────────────────────────────────────────────────────

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

fn run_check(stdin: &str) -> Value {
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

    let e = &h.expected_ots;
    let t = &h.expected_taxcalc;
    // The oracles' OWN L16 operands (baked provenance leaves) — so the per-oracle provenance classes can
    // absorb the §5.1 pinned cells' L16 dissent (bin-edge ⇒ OTS, cents-flip ⇒ taxcalc). `None` pre-bake.
    let ots_ops = oracle_ops(pr.filing_status, e.taxable_income, e.qual_div_l3a, e.net_ltcg_qd_exclusive);
    let taxcalc_ops = oracle_ops(pr.filing_status, t.taxable_income, t.qual_div_l3a, t.net_ltcg_qd_exclusive);
    let mut verdicts: Vec<Value> = Vec::new();

    // Paper reader off the flattened line map (whole dollars, SPEC §3.1). `None` ⇒ the line is not on
    // this return; a present-but-unparseable cell is a filler/map bug and panics loudly.
    let paper = |key: &str| -> Option<i64> {
        lines.get(key).map(|raw| {
            raw.parse::<i64>()
                .unwrap_or_else(|_| panic!("oracle_harness --check: cell {key:?} is not an integer: {raw:?}"))
        })
    };

    // ── AGI L11 / taxable income L15 / QBI deduction L13 — held against BOTH oracles (exact-vs-both). ──
    verdicts.push(verdict_both(
        "1040.line11", "AGI (1040 L11)",
        paper("1040.line11"), round_dollar(ar.agi), e.adjusted_gross_income, t.adjusted_gross_income,
    ));
    // Taxable income (L15) — the C1 CROSS-FOOT (AGI − deduction − QBI, each line-rounded from the oracle's
    // own leaves), floored at 0: matches btctax's whole-dollar L15 and dissolves the 8995-chain
    // rounding-order residual. Both oracles stay exact witnesses.
    verdicts.push(verdict_both_targets(
        "1040.line15", "taxable income (L15)",
        paper("1040.line15"), round_dollar(ar.taxable_income),
        ti_crossfoot(e.adjusted_gross_income, e.deduction_taken, e.qbi_deduction, e.taxable_income),
        ti_crossfoot(t.adjusted_gross_income, t.deduction_taken, t.qbi_deduction, t.taxable_income),
    ));
    // L13 is on every return (0 when there is no QBI): absent-or-present-"0" both mean $0.
    verdicts.push(verdict_both(
        "1040.line13", "QBI deduction (L13)",
        Some(paper("1040.line13").unwrap_or(0)), round_dollar(ar.qbi_deduction),
        e.qbi_deduction, t.qbi_deduction,
    ));

    // ── Tax L16 — the §6.2 two-part: `stacking_ok` absorbs taxcalc's Tax-Table-vs-schedule dissent only
    //    through the methodology class; btctax alone against BOTH oracles with no class FAILS. ──────────
    verdicts.push(verdict_l16(
        "1040.line16", "tax (L16)",
        paper("1040.line16"), round_dollar(ar.regular_tax),
        e.income_tax_before_credits, t.income_tax_before_credits,
        ots_ops.as_ref(), taxcalc_ops.as_ref(), &reproduced,
    ));

    // ── The C1 cross-foot reproductions, hoisted so L24 INHERITS them (pre-T11 the legs are `None`, so
    //    each falls back to `round_leaf` of the baked per-line total). ─────────────────────────────────
    let se_l12_ots = match (e.se_l10_oasdi, e.se_l11_medicare) {
        (Some(l10), Some(l11)) => sum_round(&[l10, l11]),
        _ => round_leaf(e.se_tax),
    };
    let f8959_l18_ots = match (e.f8959_l7, e.f8959_l13) {
        (Some(l7), Some(l13)) => sum_round(&[l7, l13]),
        _ => round_leaf(e.additional_medicare_tax),
    };

    // ── TOTAL TAX L24 — OTS single-witness cross-foot that inherits SE-L12 / 8959-L18:
    //    `round_leaf(L16) + SE-L12 + 8959-L18 + round_leaf(NIIT)`. line17 (AMT/APTC) and line21
    //    (credits) are read as the precondition (must be 0 for an admitted scenario) and echoed. ────────
    // L16 leg = btctax's OWN FILED L16 (the value summed into printed L24), not the oracle's L16 — the L16
    // VALUE is adjudicated separately by verdict_l16 (with its provenance/methodology class). Keeps L24
    // reconciled on the §5.1 pinned cells while still catching a real cross-foot / Sch-2-leg / L16 bug.
    let l24_target =
        pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
    let mut l24 = verdict_ots(
        "1040.line24", "TOTAL TAX (L24)", paper("1040.line24"), pr.forms.f1040.line24, l24_target,
    );
    if let Value::Object(m) = &mut l24 {
        m.insert("precondition_line17".into(), json!(paper("1040.line17").unwrap_or(0)));
        m.insert("precondition_line21".into(), json!(paper("1040.line21").unwrap_or(0)));
    }
    verdicts.push(l24);

    // ── Schedule SE line 12 — the cross-foot reproduction. OTS single-witness. Only for an SE household. ─
    if h.inputs.self_employment_income > 0.0 {
        if let Some(p) = paper("schedule_se.line12") {
            let internal = pr
                .forms
                .sch_se
                .as_ref()
                .expect("an SE household has a printed Schedule SE")
                .line12;
            verdicts.push(verdict_ots("schedule_se.line12", "Sch SE L12 (SE tax)", Some(p), internal, se_l12_ots));
        }
    }

    // ── Form 8959 line 18 — the cross-foot reproduction. OTS single-witness. ──────────────────────────
    if let Some(p) = paper("8959.line18") {
        verdicts.push(verdict_ots("8959.line18", "8959 L18 (Add'l Medicare)", Some(p), pr.forms.f8959.line18, f8959_l18_ots));
    }

    // ── Form 8960 line 17 — NIIT. `round_leaf(oracle_niit)`, OTS single-witness (±cents epsilon by
    //    nature → §10 triage, never a class; cent-exact on the anchors today). ─────────────────────────
    if let Some(p) = paper("8960.line17") {
        let internal = pr
            .forms
            .f8960
            .as_ref()
            .expect("a NIIT household has a printed Form 8960")
            .line17;
        verdicts.push(verdict_ots("8960.line17", "8960 L17 (NIIT)", Some(p), internal, round_leaf(e.niit)));
    }

    // ── Deeper-line rows — oracle-compared only when their `Option` leaf bakes (T11). Each is a no-op on
    //    today's JSON (the leaves are `None`); they light up at the re-bake with no further rewrite. ────
    // Deduction taken (1040 L12) — both oracles.
    if let Some(p) = paper("1040.line12") {
        let internal = round_dollar(ar.deduction);
        if let Some(o) = e.deduction_taken {
            verdicts.push(verdict_ots("1040.line12", "deduction (L12) [OTS]", Some(p), internal, round_leaf(o)));
        }
        if let Some(tc) = t.deduction_taken {
            verdicts.push(verdict_ots("1040.line12", "deduction (L12) [taxcalc]", Some(p), internal, round_leaf(tc)));
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
            verdicts.push(verdict_ots("1040sa.line5e", "SALT (Sch A L5e) [OTS]", Some(p), internal, round_leaf(o)));
        }
        if let Some(tc) = t.salt_capped {
            verdicts.push(verdict_ots("1040sa.line5e", "SALT (Sch A L5e) [taxcalc]", Some(p), internal, round_leaf(tc)));
        }
    }
    // Schedule D → 1040 L7 (SIGNED, leading minus) — both oracles; only when line 7 is present.
    if let Some(p) = paper("1040.line7a") {
        let internal = round_dollar(ar.capital_gain);
        if let Some(o) = e.sch_d_to_l7 {
            verdicts.push(verdict_ots("1040.line7a", "Sch D -> L7 [OTS]", Some(p), internal, round_leaf(o)));
        }
        if let Some(tc) = t.sch_d_to_l7 {
            verdicts.push(verdict_ots("1040.line7a", "Sch D -> L7 [taxcalc]", Some(p), internal, round_leaf(tc)));
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
            verdicts.push(verdict_ots("8995.line12", "8995 L12 net-cap-gain (WEAK)", Some(p), internal, round_leaf(o)));
        }
    }

    let all_reconciled = verdicts
        .iter()
        .all(|v| v["reconciled"] == json!(true));

    json!({
        "refused": false,
        "all_reconciled": all_reconciled,
        "reproduced_ops": {
            "status": format!("{:?}", reproduced.status),
            "ti": money(reproduced.ti),
            "qd_l3a": money(reproduced.qd_l3a),
            "net_ltcg_qd_excl": money(reproduced.net_ltcg_qd_excl),
        },
        "verdicts": verdicts,
    })
}

/// A line held against BOTH oracles (`round_leaf` both sides) — AGI / QBI deduction. Reconciled iff the
/// on-paper whole dollars equal each oracle's `round_leaf`. No class absorbs a dissent here.
fn verdict_both(line: &str, label: &str, on_paper: Option<i64>, internal: Usd, ots: f64, taxcalc: f64) -> Value {
    let o = round_leaf(ots);
    let tc = round_leaf(taxcalc);
    let p = on_paper.map(Usd::from);
    let reconciled = p == Some(o) && p == Some(tc);
    verdict(line, label, on_paper, internal, Some(o), Some(tc), reconciled, if reconciled { "agree-both" } else { "diverge" })
}

/// A line held against BOTH oracles at PRE-COMPUTED whole-dollar targets (not `round_leaf` of a total) —
/// used for 1040 L15, whose target is the C1 cross-foot [`ti_crossfoot`].
fn verdict_both_targets(line: &str, label: &str, on_paper: Option<i64>, internal: Usd, ots: Usd, taxcalc: Usd) -> Value {
    let p = on_paper.map(Usd::from);
    let reconciled = p == Some(ots) && p == Some(taxcalc);
    verdict(line, label, on_paper, internal, Some(ots), Some(taxcalc), reconciled, if reconciled { "agree-both" } else { "diverge" })
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
fn oracle_ops(status: FilingStatus, taxable_income: f64, qual_div_l3a: Option<f64>, net_ltcg_qd_exclusive: Option<f64>) -> Option<L16Operands> {
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
) -> Value {
    let o = round_leaf(ots16);
    let tc = round_leaf(tc16);
    let Some(pi) = on_paper else {
        return verdict(line, label, on_paper, internal, Some(o), Some(tc), false, "absent");
    };
    let p = Usd::from(pi);
    let reconciled = stacking_ok(p, ots16, Some(tc16), ots_ops, taxcalc_ops, reproduced, None);
    let class = if p == o && p == tc {
        "agree-both"
    } else if reconciled {
        if taxcalc_methodology_class(reproduced) { "methodology-taxcalc" } else { "provenance" }
    } else {
        "diverge"
    };
    verdict(line, label, on_paper, internal, Some(o), Some(tc), reconciled, class)
}

/// A line witnessed by OTS alone (a cross-foot or a WEAK/NIIT leaf; taxcalc exposes no comparable
/// figure). `target` is the already-reproduced OTS figure (a `Usd`). Reconciled iff the paper matches.
fn verdict_ots(line: &str, label: &str, on_paper: Option<i64>, internal: Usd, target: Usd) -> Value {
    let p = on_paper.map(Usd::from);
    let reconciled = p == Some(target);
    verdict(line, label, on_paper, internal, Some(target), None, reconciled, if reconciled { "agree-ots" } else { "diverge" })
}

/// Assemble one verdict object. Money is emitted as exact whole-dollar TEXT (never a float), so the
/// Python sweep compares strings and never re-rounds.
#[allow(clippy::too_many_arguments)]
fn verdict(line: &str, label: &str, on_paper: Option<i64>, internal: Usd, ots: Option<Usd>, taxcalc: Option<Usd>, reconciled: bool, class: &str) -> Value {
    json!({
        "line": line,
        "label": label,
        "on_paper": on_paper.map(|n| Usd::from(n).to_string()),
        "internal": money(internal),
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
    if screen_absolute(&ri, &ar, &params).is_some() {
        return None; // e.g. the Form 6251 AMT screen — out of the sweep's domain
    }
    let pr = assemble_printed_return(&ri, &state, &BTreeMap::new(), &ar, &table, YEAR).ok()?; // identity would not print (D-2)
    let forms = fill_full_return(&pr, YEAR).ok()?; // a member filler refused (overflow etc.)
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
        "f1040" => F1040_MAP_2024,
        "f1040s1" => SCHEDULE_1_MAP_2024,
        "f1040s2" => SCHEDULE_2_MAP_2024,
        "f1040s3" => SCHEDULE_3_MAP_2024,
        "f1040sa" => SCHEDULE_A_MAP_2024,
        "f1040sb" => SCHEDULE_B_MAP_2024,
        "f1040sc" => SCHEDULE_C_MAP_2024,
        "schedule_d" => SCHEDULE_D_MAP_2024,
        "f8949" => F8949_MAP_2024,
        "schedule_se" => SCHEDULE_SE_MAP_2024,
        "f8959" => F8959_MAP_2024,
        "f8960" => F8960_MAP_2024,
        "f8995" => F8995_MAP_2024,
        "f8283" => F8283_MAP_2024,
        _ => return None,
    })
}
