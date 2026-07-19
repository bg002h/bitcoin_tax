//! UX-P4-12(b): key reconcile args carry units + help — `classify-inbound-income`'s `--fmv` and
//! `set-fmv`'s `--fmv` disambiguate USD dollars vs sats (the exemplary `what-if sell --help` does), and
//! `--kind` lists its valid values instead of a blank field.

fn help(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let out = std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("btctax binary must execute");
    // clap prints --help to stdout; keep stderr too in case of a usage error.
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
fn classify_inbound_income_help_has_fmv_units_and_kind_values() {
    let h = help(&["reconcile", "classify-inbound-income", "--help"]);
    assert!(
        h.contains("USD dollars, NOT sats"),
        "--fmv must disambiguate the unit:\n{h}"
    );
    assert!(
        h.contains("mining") && h.contains("staking"),
        "--kind must list its valid values:\n{h}"
    );
    // fold r1-I1: the help must NOT claim a daily-close fallback this single-event command lacks
    // (omitting --fmv fires a Hard FMV-missing blocker).
    assert!(
        h.contains("FMV missing"),
        "--fmv help must state the real no-fmv behavior (a Hard FMV-missing blocker):\n{h}"
    );
    assert!(
        !h.contains("bundled daily close for the receipt date is used"),
        "the false daily-close fallback claim must be gone:\n{h}"
    );
    // fold r2-I-A: the remedy is VOID-then-reclassify (classify-inbound is first-wins, so a bare
    // re-classify with --fmv is refused by the record-time guard). Pin the exact command.
    assert!(
        h.contains("reconcile void") && h.contains("first-wins"),
        "--fmv help remedy must be `reconcile void` then re-classify (first-wins):\n{h}"
    );
}

#[test]
fn set_fmv_help_has_fmv_units() {
    let h = help(&["reconcile", "set-fmv", "--help"]);
    assert!(
        h.contains("USD dollars, NOT sats"),
        "--fmv must disambiguate the unit:\n{h}"
    );
}
