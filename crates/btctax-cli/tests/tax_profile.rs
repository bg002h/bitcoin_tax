//! CLI integration tests for the `tax-profile` set/show command (Task 8).
//! Uses a temp vault + synthetic data only — PRIVACY: never reads ~/Documents/BitcoinTax/ReadOnly.
use btctax_cli::{cmd, tax_profile};
use btctax_core::{Carryforward, FilingStatus, TaxProfile};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn prof_2025() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Mfj,
        ordinary_taxable_income: dec!(120000),
        magi_excluding_crypto: dec!(130000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    }
}

/// Set a profile for 2025, then show it — must round-trip exactly.
#[test]
fn set_then_show_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // set
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();

    // show
    let got = cmd::tax::show_profile(&vault, &pp(), 2025).unwrap();
    assert_eq!(got, Some(prof_2025()));
}

/// show for a year with no stored profile → None.
#[test]
fn show_missing_year_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let got = cmd::tax::show_profile(&vault, &pp(), 2025).unwrap();
    assert_eq!(got, None);
}

/// Overwriting an existing profile upserts (replaces the old value).
#[test]
fn set_overwrites_previous_profile() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();

    let updated = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(80000),
        magi_excluding_crypto: dec!(85000),
        qualified_dividends_and_other_pref_income: dec!(500),
        other_net_capital_gain: dec!(1000),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(200),
            long: dec!(300),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, updated.clone(), false).unwrap();

    let got = cmd::tax::show_profile(&vault, &pp(), 2025).unwrap();
    assert_eq!(got, Some(updated));
}

/// Multiple years are stored independently.
#[test]
fn multiple_years_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let p24 = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(90000),
        magi_excluding_crypto: dec!(95000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2024, p24.clone(), false).unwrap();
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();

    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2024).unwrap(),
        Some(p24)
    );
    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2025).unwrap(),
        Some(prof_2025())
    );
    assert_eq!(cmd::tax::show_profile(&vault, &pp(), 2026).unwrap(), None);
}

/// `tax_profile::get` on a vault opened (not freshly created) still works (robust to missing DDL
/// call — the defensive `init_table` guard inside `get` creates the table if absent).
#[test]
fn get_on_open_vault_without_prior_init_table_call_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Open the vault and call the low-level `get` directly — no `Session::tax_profile` wrapper.
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    let result = tax_profile::get(s.conn(), 2025).unwrap();
    assert_eq!(result, None);
}

/// D-4 guard (SPEC §4.12): once full-return `ReturnInputs` exist for a year, a raw `tax-profile set`
/// for that year is REFUSED (would be silently ignored by `resolve_profile`) — unless `--force`.
/// `income clear` removes the inputs and re-opens the raw path. This is the two-sources-of-truth
/// guard end to end (import → refuse → force → clear → allow).
#[test]
fn set_profile_is_refused_while_return_inputs_exist_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Seed full-return inputs for 2024 via the TOML import path.
    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"82000\"\nbox2_fed_withheld = \"9100\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).unwrap();

    // A raw tax-profile for the SAME year is refused (Usage), and nothing is stored.
    let err = cmd::tax::set_profile(&vault, &pp(), 2024, prof_2025(), false).unwrap_err();
    assert!(matches!(err, btctax_cli::CliError::Usage(_)));
    assert_eq!(cmd::tax::show_profile(&vault, &pp(), 2024).unwrap(), None);

    // A DIFFERENT year is unaffected (the guard is per-year).
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();
    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2025).unwrap(),
        Some(prof_2025())
    );

    // --force overrides the guard and stores the raw profile anyway.
    cmd::tax::set_profile(&vault, &pp(), 2024, prof_2025(), true).unwrap();
    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2024).unwrap(),
        Some(prof_2025())
    );

    // `income clear` removes the inputs; afterward the un-forced path is allowed again.
    assert!(cmd::tax::clear_return_inputs(&vault, &pp(), 2024, false).unwrap());
    assert!(!cmd::tax::clear_return_inputs(&vault, &pp(), 2024, false).unwrap()); // idempotent
    cmd::tax::set_profile(&vault, &pp(), 2024, prof_2025(), false).unwrap();
}

/// Vault-level `income import` → `income show` round trip (review M6): imports a TOML with a cleartext
/// SSN, then `income show` must return the stored inputs as JSON with the SSN REDACTED (I5) — cleartext
/// PII must never reach stdout. `show` on an unset year is `None`.
#[test]
fn income_import_then_show_redacts_pii_at_the_vault_level() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), 2024).unwrap(),
        None
    );

    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\n\n[header.taxpayer]\nfirst_name = \"Pat\"\nlast_name = \"Doe\"\nssn = \"123-45-6789\"\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"82000\"\nbox2_fed_withheld = \"9100\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).unwrap();

    let shown = cmd::tax::show_return_inputs(&vault, &pp(), 2024)
        .unwrap()
        .expect("inputs were imported");
    assert!(
        shown.contains("***-**-6789"),
        "SSN must be redacted: {shown}"
    );
    assert!(
        !shown.contains("123-45-6789"),
        "cleartext SSN must never appear: {shown}"
    );
    assert!(shown.contains("82000")); // non-PII figures are shown verbatim
                                      // M-1: with `serde_json` `preserve_order` enabled workspace-wide, `income show` renders fields in
                                      // the ReturnInputs STRUCT's declared order (`filing_status` first), NOT alphabetically (where
                                      // `capital_loss_carryforward_in` would precede it). A distinguishing pair: struct order puts
                                      // filing_status first, alphabetical order puts capital_loss_carryforward_in first.
    let fs = shown
        .find("\"filing_status\"")
        .expect("filing_status present");
    let cl = shown
        .find("\"capital_loss_carryforward_in\"")
        .expect("capital_loss_carryforward_in present");
    assert!(
        fs < cl,
        "preserve_order: curated struct order (filing_status before capital_loss_carryforward_in), \
         not alphabetical:\n{shown}"
    );
}

/// ★★★ **T11 / R13 — `income project` at the VAULT level: the oracle row, and no identity.**
///
/// The structural half is held in core (`oracle_projection.rs`), which plants distinctive tokens in
/// every identity field of a `ReturnInputs` and asserts none survives the projection. This is the
/// other half — the whole command, over a real vault, from an imported TOML that carries a cleartext
/// SSN — because the guarantee that matters to a filer is about what the COMMAND prints.
///
/// ★ It also pins the `not_carried` census, which is the thing that stops a truncated household
///   reading as a btctax defect: box 2 withholding is on the return and cannot reach either engine,
///   and the operator has to be told so.
#[test]
fn income_project_prints_the_oracle_row_and_no_identity() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    assert_eq!(
        cmd::tax::project_return_inputs(&vault, &pp(), 2024).unwrap(),
        None,
        "a year with no stored return projects nothing"
    );

    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\n\n\
         [header.taxpayer]\nfirst_name = \"Pat\"\nlast_name = \"Doe\"\nssn = \"123-45-6789\"\n\n\
         [[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"82000\"\n\
         box2_fed_withheld = \"9100\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).unwrap();

    let out = cmd::tax::project_return_inputs(&vault, &pp(), 2024)
        .unwrap()
        .expect("inputs were imported");
    // The FIGURES travel …
    assert!(
        out.contains("82000"),
        "the wages must reach the row:\n{out}"
    );
    assert!(out.contains("\"filing_status\": \"Single\""), "{out}");
    // … and NOTHING that identifies the filer does. Not redacted — absent by type.
    for token in ["Pat", "Doe", "123-45-6789", "***-**-6789", "ACME"] {
        assert!(
            !out.contains(token),
            "the oracle row leaked {token:?}:\n{out}"
        );
    }
    // The withholding is on the return and reaches neither engine; the census must say so.
    assert!(
        out.contains("box2_fed_withheld") && out.contains("PaymentOrWithholding"),
        "`not_carried` must name the withholding and its reason:\n{out}"
    );
    // And the row itself parses back as a `GoldenInputs`, which is what the harness and both oracle
    // drivers consume — a row nothing can read would be a screenshot with extra steps.
    let doc: serde_json::Value = serde_json::from_str(&out).expect("the projection is JSON");
    let _row: btctax_core::tax::testonly::GoldenInputs =
        serde_json::from_value(doc["row"].clone()).expect("the row round-trips as GoldenInputs");
}

/// ★★★ **T11 seam review C-1 — a return btctax REFUSES must say so in the projection.**
///
/// `scripts/oracle/check_return.py` promised `Exit 2 = … a refused return` and could not deliver it:
/// it hands the projected ROW to the harness, and the harness rebuilds a household with
/// `build_golden_return` — a fixture builder that sets the tax year, answers every live declaration
/// and both Schedule B Part III questions. So the refusal was answered away in the round trip and a
/// return `btctax report` refuses came back *"Every compared line reconciles"* at exit 0. The
/// refusal has to be taken where the filer's own `ReturnInputs` are in hand, which is here.
///
/// ★ The row still PRINTS — a filer is entitled to see what the engines would be told — so the only
///   thing this test can hold is the block itself. Deleting it leaves every other assertion in this
///   file green, which is exactly why it is written down.
#[test]
fn income_project_reports_the_screen_that_refuses_the_filers_own_return() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // A return with an UNANSWERED Schedule B Part III (FBAR) gate: `btctax report` refuses it.
    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\n\n\
         [header.taxpayer]\nfirst_name = \"Pat\"\nlast_name = \"Doe\"\nssn = \"123-45-6789\"\n\n\
         [[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"82000\"\n\
         box2_fed_withheld = \"9100\"\n\n\
         [[int_1099]]\npayer = \"BANK\"\nbox1_interest = \"2000\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).unwrap();

    let out = cmd::tax::project_return_inputs(&vault, &pp(), 2024)
        .unwrap()
        .expect("inputs were imported");
    let doc: serde_json::Value = serde_json::from_str(&out).expect("the projection is JSON");

    // The premise, measured rather than assumed: the COMPUTING path really does refuse this return.
    let refusal = doc["refused"].as_object().unwrap_or_else(|| {
        panic!("this fixture must be a return btctax refuses, else the test proves nothing:\n{out}")
    });
    let screen = refusal["screen"].as_str().expect("the screen is named");
    assert!(
        screen.starts_with("ScheduleBPart3Unanswered"),
        "the block must name the screen that fired, not merely that one did: {screen}"
    );
    assert!(
        refusal["detail"]
            .as_str()
            .expect("the refusal carries its own sentence")
            .contains("foreign financial account"),
        "the filer needs the refusal's own words, not a code:\n{out}"
    );
    // …and the row is STILL printed, so the filer can see what the engines would have been told.
    assert!(doc["row"]["w2_income"].as_f64() == Some(82_000.0), "{out}");

    // …and the block is not a CONSTANT: the same command on a FULLY ANSWERED return carries none.
    // A refusal that never lifts is no more use to `check_return.py` than one that never fires.
    let answered = dir.path().join("answered.toml");
    std::fs::write(&answered, ANSWERED_MFJ_RETURN).unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &answered, false, true).unwrap();
    let clean = cmd::tax::project_return_inputs(&vault, &pp(), 2024)
        .unwrap()
        .expect("inputs were imported");
    let clean: serde_json::Value = serde_json::from_str(&clean).expect("the projection is JSON");
    assert!(
        clean["refused"].is_null(),
        "a return btctax computes must carry no refusal block: {}",
        clean["refused"]
    );
    // The premise of THAT half, measured: this household really does itemize and really does claim
    // a child — the two routing facts the T11 fold added to the row (C-2, and the dependents block).
    assert_eq!(clean["row"]["standard_or_itemized"], "Itemized");
    assert_eq!(clean["row"]["dependents"][0]["credit"], "child_tax_credit");
}

/// A COMPLETE MFJ return — every live declaration, the whole document census, all fifteen §152
/// dependent gates — so `income project` on it carries no refusal. Kept as a literal because the
/// point of the fixture is that nothing is left unanswered, and a builder that filled the blanks
/// would be answering for the filer, which is the defect this whole seam exists to prevent.
const ANSWERED_MFJ_RETURN: &str = r#"filing_status = "Mfj"
tax_year = 2024
charitable_cwa_obtained = true
foreign_accounts = false
foreign_trust = false
dual_status_alien = false
has_income_exclusion = false
other_out_of_scope_income = false
excluded_canceled_debt = false
filing_form_4952 = false
claiming_mortgage_interest_credit = false
digital_asset_activity = false
state_refund_without_1099g = false
interest_or_dividends_without_1099 = false
amt_carryover_same_as_regular = true
amt_depreciation_same_as_regular = true
carryover_includes_spouses_joint_loss = false

[sch1]
hsa_activity = false

[header]
filer_tin_issued_by_due_date = true
nra_spouse_resident_election = false
can_be_claimed_as_dependent_taxpayer = false
can_be_claimed_as_dependent_spouse = false
taxpayer_died_during_year = false
spouse_died_during_year = false

[header.taxpayer]
first_name = "Robin"
last_name = "Hale"
ssn = "111-22-3333"
date_of_birth = "1979-04-04"

[header.spouse]
first_name = "Sam"
last_name = "Hale"
ssn = "222-33-4444"
date_of_birth = "1981-08-08"

[[header.dependents]]
name = "Kim Hale"
ssn = "333-44-5555"
relationship = "Daughter"
date_of_birth = "2015-03-03"
lived_with_you_over_half_year = true
lived_with_you_in_us = true
full_time_student = false
permanently_and_totally_disabled = false
qc_relationship = true
younger_than_you_or_spouse = true
provided_over_half_own_support = false
filing_joint_return = false
joint_return_only_to_claim_refund = false
qualifying_child_of_another_person = false
citizen_national_resident_or_canada_mexico = true
married = false
tin_issued_by_due_date = true
citizen_national_or_resident_alien = true
ssns_valid_for_employment_issued_by_due_date = true
qr_relationship_or_member_of_household = true
qualifying_child_of_any_taxpayer = false
gross_income_under_limit = true
you_provided_over_half_support = true
divorced_separated_multiple_support_or_kidnapped_rule_applies = false

[[w2s]]
owner = "taxpayer"
employer = "NORTHWIND"
box1_wages = "420000"
box3_ss_wages = "420000"
box5_medicare_wages = "420000"
box2_fed_withheld = "90000"
box17_state_tax_withheld = "6000"

[[int_1099]]
payer = "BIG BANK"
box1_interest = "9000"

[schedule_a]
mortgage_all_used_to_buy_build_improve = true
mortgage_dwelling_is_amt_qualified = true
mortgage_within_debt_limit = true
salt_use_sales_tax = false
salt_state_estimated_payments = "1000"
salt_real_estate = "5000"

[[schedule_a.charitable]]
class = "cash60"
amount = "2000"

[[form_1098]]
lender = "BIG BANK"
box1_interest = "30000"
other_borrower_paid_interest = false

[documents]
w2 = true
int_1099 = true
div_1099 = false
b_1099 = false
g_1099 = false
form_1098 = true
form_1098e = false
sa_1099 = false
sa_5498 = false
r_1099 = false
ssa_1099 = false
nec_misc_k_1099 = false
k1 = false
schedule_e_rental = false
s_1099 = false
oid_1099 = false
w2g = false
c_1099 = false
a_1095 = false
t_1098 = false

[home_sale]
sold_main_home = false
"#;

/// M-1 blast-radius tripwire (spec §4/§9.6, "pin that enumeration in the KAT"): the workspace-global
/// `preserve_order` flip is SAFE only because every PRODUCTION `serde_json::Value` construction is
/// DISPLAYED or PARSED, never serialized into PERSISTED/FINGERPRINTED bytes — those use typed serde
/// (`serde_json::to_string(&<typed>)`, field-ordered regardless) and hand-rolled Decimal bytes. This
/// scans EVERY line of `crates/*/src` for the Value-CONSTRUCTION-for-output idioms
/// (`serde_json::to_value` / `json!`) and asserts each hit's file is in the audited enumeration; a NEW
/// site FAILS here, forcing an audit of whether it feeds stored bytes.
/// (No `#[cfg(test)]` skip — a sticky file-level flag would blind production code after a mid-file
/// `mod tests`; scanning all lines is correct AND stricter, and no Value-output site is outside the
/// list today, test or otherwise.)
///
/// Known-safe sites: `btctax-cli/src/cmd/tax.rs` (`income show` display JSON, never parsed; M8);
/// `btctax-oracle-harness/src/main.rs` (`json!` → stdout, displayed/re-parsed, never stored/hashed);
/// `btctax-input-form/src/spec/coverage.rs` (coverage tooling, `#[cfg(test)]`-gated at its `mod`).
/// (update-prices is parse-only — `from_str` — and constructs no output `Value`; btctax-forms/xtask
/// are serde_json-free.)
#[test]
fn m1_preserve_order_value_output_sites_are_enumerated() {
    use std::path::{Path, PathBuf};
    // Files that construct a `Value` for OUTPUT and are audited display/parse-only (not stored/hashed).
    const ALLOWED: &[&str] = &[
        "btctax-cli/src/cmd/tax.rs",
        "btctax-oracle-harness/src/main.rs",
        "btctax-input-form/src/spec/coverage.rs",
        // ★ AUDITED 2026-08-09 (SPEC_income_scrub.md §3.3, §8 step 4). `replaced_paths` serializes a
        //   `ReturnInputs` and its scrubbed twin ONLY to diff them into a set of field PATHS — the
        //   derived axis §3.3's fixture matrix is driven by. Neither `Value` is written, stored,
        //   hashed or emitted: the function returns `BTreeSet<String>` and every caller is an
        //   assertion. So key order cannot reach persisted or fingerprinted bytes, which is the
        //   invariant this enumeration protects. (Deliberately NOT hidden behind `#[cfg(test)]` — the
        //   scan reads test regions too, by design, and gating it would evade the audit rather than
        //   answer it.)
        "btctax-core/src/tax/scrub_axis.rs",
        // ★ AUDITED 2026-09-06 (SPEC_interview.md R10 / task T1). `provenance.rs`'s `Value` use is
        //   the LEAF_SOURCE KAT and nothing else: it serializes a `ReturnInputs`, walks it into leaf
        //   PATHS, and re-deserializes per-leaf probes to classify each leaf's TYPE. Every `Value`
        //   dies inside the test — the function returns `BTreeSet<String>` and every caller is an
        //   assertion — so key order cannot reach persisted or fingerprinted bytes, which is the
        //   invariant this enumeration protects. Same audit and same conclusion as `scrub_axis.rs`.
        "btctax-core/src/tax/provenance.rs",
        // ★ AUDITED 2026-09-07 (SPEC_interview.md R10.4 / task T4b, seam review I-1).
        //   `open_next_year.rs`'s `Value` use is `leaves_the_seed_writes`: it serializes the seeded
        //   draft and a blank return for the same year ONLY to diff them into a set of leaf PATHS,
        //   which decides which sentences the opener's REPORT prints. Both `Value`s die inside the
        //   function — it returns `Vec<String>` and its only caller builds display text — so key
        //   order cannot reach persisted or fingerprinted bytes (the draft itself is written by typed
        //   serde through `save_draft`, untouched by this). Same audit, same conclusion, as
        //   `scrub_axis.rs` and `provenance.rs` above.
        "btctax-cli/src/open_next_year.rs",
        // ★ AUDITED 2026-09-07 (T16 seam review I-1). `testonly.rs`'s `Value` use is
        //   `every_money_leaf_household`: it serializes `maximal_sentinel`, overwrites the leaves
        //   `provenance::leaf_walk::money_leaves` classifies as money, and immediately
        //   re-deserializes into a typed `ReturnInputs`. The `Value` dies inside the function — it
        //   returns `(ReturnInputs, LedgerState)` and its only callers are the two-chain
        //   assertions in `packet.rs` — so key order cannot reach persisted or fingerprinted bytes,
        //   which is the invariant this enumeration protects. Same audit and same conclusion as
        //   `scrub_axis.rs` and `provenance.rs`, whose walks it is built on.
        "btctax-core/src/tax/testonly.rs",
    ];
    let crates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut hits: Vec<String> = Vec::new();
    fn walk(dir: &Path, hits: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "src")
                    || p.join("src").exists()
                    || is_src_descendant(&p)
                {
                    walk(&p, hits);
                }
                continue;
            }
            if p.extension().is_none_or(|x| x != "rs") || !is_src_descendant(&p) {
                continue;
            }
            // NOTE (r2-I1): we do NOT skip `#[cfg(test)]` regions — a file-level "in_test" flag is
            // sticky and blinds production code that follows a mid-file `mod tests` (e.g. render.rs's
            // `render_events_list` after its test mods). No production OR test Value-output site is
            // outside ALLOWED today, so scanning every line is both correct and stricter — a future
            // Value-output site anywhere (incl. a test mod in a non-allowed file) reds loudly.
            let text = std::fs::read_to_string(&p).unwrap();
            for (i, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                // Best-effort backstop, not a proof: catches the two idioms every current Value-output
                // site uses (a full-path `serde_json::to_value` call — incl. a `use serde_json::to_value;`
                // import line — and `json!`). KNOWN residual gaps (r3-M1, grep-verified absent today, so
                // NOT matched to avoid CI false-positives on innocent parse imports): a braced
                // `use serde_json::{to_value, …}` followed by a bare `to_value(` call, and hand-built
                // `Value::Object`/`Map::new`. The persisted-bytes invariant does NOT rest on this scan —
                // it holds by construction (no persisted struct serializes a `Value`; typed serde is
                // field-ordered); the scan just flags a NEW site for a fresh audit.
                if code.contains("serde_json::to_value") || code.contains("json!(") {
                    hits.push(format!("{}:{}", p.display(), i + 1));
                }
            }
        }
    }
    fn is_src_descendant(p: &Path) -> bool {
        p.components().any(|c| c.as_os_str() == "src")
    }
    walk(&crates_dir, &mut hits);
    for hit in &hits {
        let norm = hit.replace('\\', "/");
        assert!(
            ALLOWED.iter().any(|a| norm.contains(a)),
            "NEW production serde_json::Value output site: {hit}\n\
             The M-1 preserve_order flip is safe ONLY for display/parse-only Value sites. Audit whether \
             this one feeds PERSISTED or FINGERPRINTED bytes (it must NOT — those are typed serde / \
             hand-rolled). If display/parse-only, add its file to ALLOWED + update the spec §9.6 enumeration."
        );
    }
}

/// M-1: the workspace-wide `serde_json` `preserve_order` flip is ACTIVE — a `Value` serializes in
/// INSERTION order, not sorted alphabetically. (Without the feature, this would emit
/// `{"apple":2,"zebra":1}`.) Removing the feature from ALL serde_json deps reds this.
#[test]
fn serde_json_preserve_order_is_enabled_workspace_wide() {
    let mut m = serde_json::Map::new();
    m.insert("zebra".to_string(), serde_json::json!(1));
    m.insert("apple".to_string(), serde_json::json!(2));
    let s = serde_json::to_string(&serde_json::Value::Object(m)).unwrap();
    assert_eq!(
        s, r#"{"zebra":1,"apple":2}"#,
        "preserve_order must keep insertion order, not sort keys"
    );
}

/// [P2 review M-r3-4 / N3] `resolve_all_screened` maps a corrupt side-table row to a per-year refusal
/// (Uncomputable), NOT a whole-vault brick — the read-only viewer must still open, with other years
/// resolving normally. Pins the availability behavior N3 introduced.
#[test]
fn resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick() {
    use btctax_cli::{resolve::ProfileOutcome, return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Valid full-return inputs for 2024 + a CORRUPT `return_inputs` blob for 2023 (one bad side-table row).
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                ..Default::default()
            }),
        )
        .unwrap();
        s.conn()
            .execute(
                "INSERT INTO return_inputs(year, inputs_json) VALUES (2023, 'not json')",
                [],
            )
            .unwrap();
        s.save().unwrap();
    }

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _cfg) = s.project().unwrap();
    let tables = btctax_adapters::BundledTaxTables::load();
    let resolved = s.resolve_all_screened(&state, &tables).unwrap();

    // 2023 (corrupt) → per-year Uncomputable; 2024 (valid) → Ready. The viewer is NOT bricked.
    assert!(
        matches!(
            resolved.get(&2023),
            Some(ProfileOutcome::Uncomputable { .. })
        ),
        "a corrupt 2023 blob must become a per-year refusal, not fail the whole enumeration"
    );
    assert!(
        matches!(resolved.get(&2024), Some(ProfileOutcome::Ready { .. })),
        "the valid 2024 year must still resolve"
    );
}

/// UX-P4-12(d): a bare `tax-profile --year Y` (no `--show`, no `--filing-status`) is a usage error
/// that POINTS at `--show` — so a user who meant to VIEW the profile isn't stranded on
/// "filing-status is required" with no hint of the read path.
#[test]
fn tax_profile_set_error_points_at_show() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let bin = env!("CARGO_BIN_EXE_btctax");
    let out = std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().unwrap(),
            "tax-profile",
            "--year",
            "2025",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    assert_eq!(
        out.status.code(),
        Some(2),
        "a bare set (no --show, no --filing-status) is a usage error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--filing-status is required"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("--show"),
        "the set-error points at --show for viewing: {stderr}"
    );
}
