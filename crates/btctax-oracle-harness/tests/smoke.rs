//! **T7 smoke test** — the oracle-sweep harness assembles + fills + reads a scenario BACK OFF THE PAPER.
//!
//! The floor case (`single_w2_only_standard`) is piped through the harness as a bare `GoldenInputs`
//! JSON on stdin; the harness assembles the SAME return the golden matrix fills, reads it back with
//! `extract_lines`, and prints the flattened `form.line → string` map. The assertion is that the AGI in
//! the box on the 1040 (`1040.line11`) is the AGI two independent oracles baked into the matrix — read
//! from the baked value, never hard-coded.
//!
//! A second case drives `--check` (the I4 reproduction/classification mode) over a whole golden
//! household and asserts every line reconciles, since the golden matrix is green by construction.

use std::io::Write;
use std::process::{Command, Stdio};

use btctax_core::tax::testonly::{golden_households, GOLDEN_RETURNS_JSON};

const HARNESS: &str = env!("CARGO_BIN_EXE_btctax-oracle-harness");

/// Run the harness with `args`, feeding `stdin_json`, and return its stdout as a parsed JSON value.
fn run(args: &[&str], stdin_json: &str) -> serde_json::Value {
    let mut child = Command::new(HARNESS)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the harness binary spawns");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin_json.as_bytes())
        .expect("write the scenario to the harness");
    let out = child.wait_with_output().expect("the harness runs to completion");
    assert!(
        out.status.success(),
        "the harness exited non-zero (args {args:?}):\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "the harness stdout is not JSON ({e}):\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

/// The raw JSON object for the named household, straight from the baked matrix (so the harness is fed
/// exactly the committed shape, no round-trip through a partial re-serialization).
fn raw_household(name: &str) -> serde_json::Value {
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    matrix["households"]
        .as_array()
        .expect("households is an array")
        .iter()
        .find(|h| h["name"] == serde_json::json!(name))
        .unwrap_or_else(|| panic!("{name} is in the golden matrix"))
        .clone()
}

#[test]
fn floor_case_reads_back_the_baked_ots_agi_off_the_paper() {
    let name = "single_w2_only_standard";

    // The baked AGI two independent oracles agreed on — read from the matrix, never guessed.
    let baked_agi = golden_households()
        .iter()
        .find(|h| h.name == name)
        .expect("the floor case is in the matrix")
        .expected_ots
        .adjusted_gross_income;
    let expected = format!("{}", baked_agi as i64); // whole dollars on the paper (SPEC §3.1)

    // Feed the bare GoldenInputs (the default-mode contract).
    let inputs = raw_household(name)["inputs"].clone();
    let out = run(&[], &serde_json::to_string(&inputs).unwrap());

    assert_eq!(
        out["refused"],
        serde_json::json!(false),
        "the floor case is refusal-free"
    );
    assert_eq!(
        out["lines"]["1040.line11"],
        serde_json::json!(expected),
        "the AGI in the box on the 1040 must be the baked oracle AGI ({expected})"
    );
}

/// btctax's conservative Form 6251 screening worksheet (`screen_absolute`) flags this high-income anchor
/// as "may owe AMT" and refuses it — even though BOTH oracles compute zero actual AMT and T6's paper
/// test fills it anyway (bypassing the screen). btctax's real export path refuses it too, so the harness
/// is right to report it out-of-domain: you cannot sweep on-paper values btctax will not produce. This
/// is exactly the D-2 refusal signal T10 rejects candidates on.
const EXPECTED_REFUSED: &[&str] = &["mfj_high_income_niit_and_addl_medicare"];

#[test]
fn the_amt_screen_anchor_is_reported_refused_in_default_mode() {
    let name = EXPECTED_REFUSED[0];
    let inputs = raw_household(name)["inputs"].clone();
    let out = run(&[], &serde_json::to_string(&inputs).unwrap());
    assert_eq!(
        out["refused"],
        serde_json::json!(true),
        "{name} trips btctax's Form 6251 AMT screen — the harness must report the D-2 refusal"
    );
    assert!(out.get("lines").is_none(), "a refused scenario carries no lines");
}

/// Drive `--check` over `households` and assert every ADMITTED one reconciles on every compared line
/// (the golden matrix is green by construction), and that exactly the AMT-screen anchor is refused.
/// Each call SPAWNS the harness subprocess per household, so this is inherently serial.
fn sweep_check_reconciliation(households: &[serde_json::Value]) {
    let mut refused: Vec<String> = Vec::new();
    let mut admitted = 0;
    for household in households {
        let name = household["name"].as_str().unwrap_or("<unnamed>").to_string();
        let out = run(&["--check"], &serde_json::to_string(household).unwrap());
        if out["refused"] == serde_json::json!(true) {
            refused.push(name);
            continue;
        }
        admitted += 1;
        assert_eq!(
            out["all_reconciled"],
            serde_json::json!(true),
            "{name}: every compared line must reconcile against BOTH oracles:\n{}",
            serde_json::to_string_pretty(&out["verdicts"]).unwrap()
        );
        // T7-m1: the Part-1 structural reproduction witness holds on every admitted household — btctax's
        // own `table_l16` reproduces its own filed L16 (true by construction; the sweep checks this field
        // on operand regions the anchors never reach).
        assert_eq!(
            out["reproduction_ok"],
            serde_json::json!(true),
            "{name}: table_l16(btctax operands) must reproduce btctax's own regular tax"
        );
    }
    assert!(admitted >= 10, "most anchors are admitted and swept, got {admitted}");
    assert_eq!(
        refused, EXPECTED_REFUSED,
        "exactly the known AMT-screen anchor should be refused; a change here means the AMT screen's \
         behavior moved — update EXPECTED_REFUSED deliberately, don't paper over it"
    );
}

/// ★ The make-check sweep: the 12 hand-audited ANCHORS + the 2 §5.1 PINNED cells (the non-`ca_` names).
/// Together they exercise EVERY reconciliation category `--check` implements — the L16 methodology class
/// (Table anchors), BOTH per-oracle provenance classes (the pinned cells), the C1 cross-foots (L24, SE
/// L12, 8959 L18), NIIT, the SALT cap and the deeper lines. The generated covering array is swept
/// differentially in `make check` by the sharded `golden_packet` (and by the `#[ignore]` twin below); a
/// serial subprocess-per-household loop over all ~104 here would blow the `make check` budget (§8).
#[test]
fn check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells() {
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    let households: Vec<serde_json::Value> = matrix["households"]
        .as_array()
        .expect("households array")
        .iter()
        .filter(|h| !h["name"].as_str().unwrap_or("").starts_with("ca_"))
        .cloned()
        .collect();
    assert!(
        households.len() >= 12,
        "the matrix carries the twelve anchors + the two pinned cells"
    );
    sweep_check_reconciliation(&households);
}

/// The whole-corpus twin (§8) — `--check` reconciles every line of ALL ~104 admitted households. A serial
/// subprocess-per-household loop, so `#[ignore]`d by default: run on demand / in CI.
#[test]
#[ignore = "full corpus (~104), serial subprocess per household — make-check sweeps the anchors + pinned cells; run in CI / on demand"]
fn check_mode_reconciles_every_line_of_every_admitted_golden_household() {
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    let households = matrix["households"].as_array().expect("households array");
    assert!(households.len() >= 100, "the whole T11 corpus");
    sweep_check_reconciliation(households);
}

/// ★ T7-m2: the `--known-defect` pass-through has TEETH. A pinned §10 known-defect is authoritative for
/// its line — a `--check` run reconciles L16 iff btctax still prints the pinned wrong value — and a STALE
/// pin FAILS, forcing the entry's removal. To exercise a divergence on the green corpus we INJECT a wrong
/// oracle L16 into the household (both oracles perturbed off btctax's on-paper figure), so without a pin
/// the line diverges; the pin at btctax's ACTUAL printed value then suppresses it, and a pin at any other
/// value stays red.
#[test]
fn known_defect_pin_suppresses_an_l16_divergence_and_a_stale_pin_stays_red() {
    let mut household = raw_household("single_w2_only_standard");

    // btctax's ACTUAL on-paper L16 (whole dollars) — read it back once, never guessed.
    let baseline = run(&["--check"], &serde_json::to_string(&household).unwrap());
    let l16 = baseline["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["line"] == serde_json::json!("1040.line16"))
        .expect("an L16 verdict");
    let on_paper: i64 = l16["on_paper"].as_str().unwrap().parse().unwrap();
    assert_eq!(l16["reconciled"], serde_json::json!(true), "green baseline");

    // Inject a wrong oracle L16 on BOTH oracles (off btctax's figure) so the line genuinely diverges.
    let wrong = (on_paper + 500) as f64;
    household["expected_ots"]["income_tax_before_credits"] = serde_json::json!(wrong);
    household["expected_taxcalc"]["income_tax_before_credits"] = serde_json::json!(wrong);
    let hj = serde_json::to_string(&household).unwrap();

    let l16_of = |out: &serde_json::Value| -> serde_json::Value {
        out["verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["line"] == serde_json::json!("1040.line16"))
            .cloned()
            .expect("an L16 verdict")
    };

    // No pin ⇒ the injected divergence surfaces (the anti-world guard, no class absorbs it above ceiling
    // — here below ceiling the methodology class would absorb a taxcalc-only dissent, but OTS is ALSO
    // wrong, so OTS's provenance conjunct-1 fails and the line stays red).
    let bare = run(&["--check"], &hj);
    assert_eq!(bare["all_reconciled"], serde_json::json!(false), "injected divergence must surface");
    assert_eq!(l16_of(&bare)["reconciled"], serde_json::json!(false));

    // Pin at btctax's ACTUAL value ⇒ the known defect is suppressed (reconciled, labelled `known-defect`).
    let pinned = run(
        &["--check", "--known-defect", &format!("1040.line16={on_paper}@FU-SMOKE")],
        &hj,
    );
    assert_eq!(l16_of(&pinned)["reconciled"], serde_json::json!(true), "the pin holds");
    assert_eq!(l16_of(&pinned)["class"], serde_json::json!("known-defect"));

    // STALE pin (btctax's value is not what was pinned) ⇒ stays red, forcing the entry's removal.
    let stale = run(
        &["--check", "--known-defect", &format!("1040.line16={}@FU-SMOKE", on_paper + 1)],
        &hj,
    );
    assert_eq!(l16_of(&stale)["reconciled"], serde_json::json!(false), "a stale pin must fail");
}
