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

#[test]
fn check_mode_reconciles_every_line_of_every_admitted_golden_household() {
    // The golden matrix is green by construction (the T6 paper differential passes on all twelve), so
    // `--check` must return `all_reconciled` for every household btctax ADMITS — exercising the
    // cross-foots (L24, SE L12, 8959 L18), the L16 methodology class, NIIT, and the SALT cap across the
    // whole matrix, not one anchor. The lone AMT-screen anchor is (correctly) refused instead.
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    let households = matrix["households"].as_array().expect("households array");
    assert!(households.len() >= 12, "the matrix carries the twelve anchors");

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
    }
    assert!(admitted >= 10, "most anchors are admitted and swept, got {admitted}");
    assert_eq!(
        refused, EXPECTED_REFUSED,
        "exactly the known AMT-screen anchor should be refused; a change here means the AMT screen's \
         behavior moved — update EXPECTED_REFUSED deliberately, don't paper over it"
    );
}
