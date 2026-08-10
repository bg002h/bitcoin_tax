//! `income scrub` — the SCRUB-OWNED refusal predicate (SPEC_income_scrub.md §2.2, §8 step 2).
//!
//! ★★★ WHY THIS FILE EXISTS. `income scrub` emits a copy of a real return and tells the filer it is
//! safe to hand to a stranger. It may emit ONLY when the ledger contributes nothing to any figure,
//! refusal, gate, watermark or advisory btctax produces for that year. The first draft reused the
//! 1040 digital-asset-box predicate — `digital_asset_activity(state, year)` alone — and a spec review
//! killed it on two counts:
//!
//! 1. **Year-scope is not artifact-scope.** `pseudo_active()` and `first_hard_blocker()` are
//!    PROJECTION-WIDE and year-blind. A vault whose only activity is an unclassified inbound has no
//!    in-year digital-asset activity at all, yet the FILER's export is DRAFT-watermarked and gated —
//!    and the recipient's copy would be clean and ungated. That filer ("btctax won't export my
//!    forms") is precisely the one most likely to run `income scrub`, handed the one file on which
//!    their problem vanishes.
//! 2. **It leaned on a don't-know.** That predicate's own doc says `false` leaves the box UNCHECKED,
//!    not answered "No". Reading the silence as "the ledger is empty" is widening an exemption.
//!
//! PRIVACY: synthetic values in tempdirs; no user file is read.

use btctax_cli::{cmd, Session};
use btctax_core::event::{Acquire, BasisSource, Dispose, DisposeKind, EventPayload, TransferIn};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::append_import_batch;
use btctax_core::state::Severity;
use btctax_core::tax::return_inputs::{Owner, ReturnInputs, W2};
use btctax_core::tax::types::FilingStatus;
use btctax_core::LedgerEvent;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::date;
use time::UtcOffset;

/// The year whose `ReturnInputs` every test here scrubs. Chosen so the ledger fixtures below sit
/// OUTSIDE it and outside `SCRUB_YEAR - 1`, isolating the disjunct each test is about.
const SCRUB_YEAR: i32 = 2024;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}

fn run_btctax(vault: &Path, args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let mut c = std::process::Command::new(bin);
    c.arg("--vault").arg(vault.to_str().unwrap());
    for a in args {
        c.arg(a);
    }
    c.env("BTCTAX_PASSPHRASE", "pw");
    let out = c.output().expect("run btctax");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn empty_vault(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    vault
}

/// Store a minimal, fully-answered `ReturnInputs` for [`SCRUB_YEAR`] — wages only, no crypto. The
/// return itself is deliberately ledger-free, so ANY refusal these tests see comes from the ledger
/// state and not from the return's own content.
fn store_wage_only_return(vault: &Path) {
    let mut s = Session::open(vault, &pp()).unwrap();
    btctax_cli::return_inputs::set(
        s.conn(),
        SCRUB_YEAR,
        &btctax_core::tax::testonly::answered(ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(80000),
                box3_ss_wages: dec!(80000),
                box5_medicare_wages: dec!(80000),
                ..Default::default()
            }],
            ..Default::default()
        }),
    )
    .unwrap();
    s.save().unwrap();
}

/// The shape a fixture must have for its test to isolate ONE disjunct. Every field is asserted, so a
/// fixture that quietly starts tripping a second disjunct fails loudly instead of passing for the
/// wrong reason.
struct Shape {
    in_year: bool,
    prior_year: bool,
    pseudo: bool,
    hard_blocker: bool,
}

/// Assert the fixture isolates the disjunct the caller names. Activity is computed here from the
/// PUBLIC state rather than from `digital_asset_activity`, which is crate-private to `btctax-core`.
fn assert_ledger_shape(vault: &Path, want: Shape) {
    let s = Session::open(vault, &pp()).unwrap();
    let (state, _cfg) = s.project().unwrap();

    let activity_in = |y: i32| {
        state.disposals.iter().any(|d| d.disposed_at.year() == y)
            || state
                .income_recognized
                .iter()
                .any(|i| i.recognized_at.year() == y)
            || state.removals.iter().any(|r| r.removed_at.year() == y)
    };
    assert_eq!(
        activity_in(SCRUB_YEAR),
        want.in_year,
        "fixture's IN-YEAR activity is not what this test is about"
    );
    assert_eq!(
        activity_in(SCRUB_YEAR - 1),
        want.prior_year,
        "fixture's PRIOR-YEAR activity is not what this test is about"
    );
    assert_eq!(
        state.pseudo_active(),
        want.pseudo,
        "fixture's pseudo_active() is not what this test is about"
    );
    assert_eq!(
        state
            .blockers
            .iter()
            .any(|b| b.kind.severity() == Severity::Hard),
        want.hard_blocker,
        "fixture's Hard-blocker presence is not what this test is about"
    );
}

/// A COVERED disposal in `year`: a 2017 buy big enough to cover it, so the sell raises no
/// `UncoveredDisposal` (which is `Severity::Hard` and would trip a different disjunct entirely).
fn add_covered_disposal(vault: &Path, year: i32) {
    let mut s = Session::open(vault, &pp()).unwrap();
    let buy = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("BUY")),
        utc_timestamp: date!(2017 - 01 - 01).midnight().assume_utc(),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload: EventPayload::Acquire(Acquire {
            sat: 60_000_000,
            usd_cost: dec!(3_000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };
    let sell = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("SELL")),
        utc_timestamp: time::Date::from_calendar_date(year, time::Month::September, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload: EventPayload::Dispose(Dispose {
            sat: 40_000_000,
            usd_proceeds: dec!(20_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };
    append_import_batch(s.conn(), &[buy, sell]).unwrap();
    s.save().unwrap();
}

// ── disjuncts 1 and 2 — the year-scoped pair ─────────────────────────────────────────────────────
//
// ★★★ THESE TWO WERE ADDED AFTER MUTATION TESTING, NOT BEFORE. The first version of this file
// covered only the two disjuncts SPEC §8 step 2 names by test ("an unclassified-inbound pseudo
// vault, and an out-of-year Hard blocker"). Deleting either year-scoped disjunct from
// `ledger_contribution` then left ALL 2649 tests green — the spec listed the two the ORIGINAL
// predicate would have missed, and nobody noticed that made the other two unheld. A guarantee
// without a test that reds when it is removed does not exist.

#[test]
fn in_year_digital_asset_activity_refuses_to_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);
    add_covered_disposal(&vault, SCRUB_YEAR);
    assert_ledger_shape(
        &vault,
        Shape {
            in_year: true,
            prior_year: false,
            pseudo: false,
            hard_blocker: false,
        },
    );

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_ne!(
        code, 0,
        "a disposal IN the scrub year must refuse — the ledger plainly contributes. stdout={stdout:?}"
    );
    assert!(
        !stdout.contains("box1_wages"),
        "a refusal must emit NO scrubbed TOML: {stdout:?}"
    );
    assert!(
        stderr.to_lowercase().contains("that year"),
        "the refusal must name the mechanism: {stderr:?}"
    );
}

#[test]
fn prior_year_digital_asset_activity_refuses_to_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);
    add_covered_disposal(&vault, SCRUB_YEAR - 1);
    assert_ledger_shape(
        &vault,
        Shape {
            in_year: false,
            prior_year: true,
            pseudo: false,
            hard_blocker: false,
        },
    );

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_ne!(
        code, 0,
        "a disposal in the PRIOR year must refuse. This disjunct is deliberate OVER-refusal: it \
         secures no live read, and is kept because scrub declines to prove the negative that no \
         path reads the prior year's ledger. Two attempts to name a mechanism for it were both \
         wrong — do not delete it on the strength of a third. stdout={stdout:?}"
    );
    assert!(
        !stdout.contains("box1_wages"),
        "a refusal must emit NO scrubbed TOML: {stdout:?}"
    );
    assert!(
        stderr.to_lowercase().contains("prior year"),
        "the refusal must name the mechanism: {stderr:?}"
    );
}

// ── disjunct 3 — `pseudo_active()`, which is PROJECTION-WIDE and year-blind ──────────────────────

#[test]
fn a_pseudo_active_vault_refuses_to_scrub_though_the_year_itself_is_ledger_quiet() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);

    let (code, _out, stderr) = run_btctax(&vault, &["reconcile", "pseudo", "on"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let inbound = LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("IN")),
            // ★ 2026 — OUTSIDE SCRUB_YEAR and SCRUB_YEAR-1, so `digital_asset_activity` is false for
            //   both and only the projection-wide disjunct can fire.
            utc_timestamp: date!(2026 - 03 - 01).midnight().assume_utc(),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet()),
            payload: EventPayload::TransferIn(TransferIn {
                sat: 10_000_000,
                src_addr: None,
                txid: None,
            }),
        };
        append_import_batch(s.conn(), &[inbound]).unwrap();
        s.save().unwrap();
    }
    assert_ledger_shape(
        &vault,
        Shape {
            in_year: false,
            prior_year: false,
            pseudo: true,
            hard_blocker: false,
        },
    );

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_ne!(
        code, 0,
        "a pseudo-active vault must REFUSE to scrub: the filer's own export carries a DRAFT \
         watermark and an attestation gate that the recipient's copy would not reproduce. \
         stdout={stdout:?}"
    );
    assert!(
        !stdout.contains("[filing_status]") && !stdout.contains("box1_wages"),
        "a refusal must emit NO scrubbed TOML on stdout: {stdout:?}"
    );
    assert!(
        stderr.to_lowercase().contains("pseudo"),
        "the refusal must name the mechanism that caused it: {stderr:?}"
    );
}

// ── disjunct 4 — `first_hard_blocker()`, likewise projection-wide and year-blind ─────────────────

#[test]
fn an_out_of_year_hard_blocker_refuses_to_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);

    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        // A Sell with no covering lot anywhere ⇒ `UncoveredDisposal`, whose severity is Hard.
        // ★ Dated 2020: OUTSIDE SCRUB_YEAR and SCRUB_YEAR-1. The filer's 2024 delta is
        //   NotComputable because of it, while the recipient's copy of the 2024 inputs computes
        //   cleanly — the exact asymmetry §2.2 exists to stop.
        let sell = LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("SELL")),
            utc_timestamp: date!(2020 - 06 - 01).midnight().assume_utc(),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet()),
            payload: EventPayload::Dispose(Dispose {
                sat: 50_000_000,
                usd_proceeds: dec!(1_000),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        };
        append_import_batch(s.conn(), &[sell]).unwrap();
        s.save().unwrap();
    }
    assert_ledger_shape(
        &vault,
        Shape {
            in_year: false,
            prior_year: false,
            pseudo: false,
            hard_blocker: true,
        },
    );

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_ne!(
        code, 0,
        "an out-of-year Hard blocker must REFUSE to scrub: it makes the filer's year \
         NotComputable and the recipient's computable. stdout={stdout:?}"
    );
    assert!(
        !stdout.contains("[filing_status]") && !stdout.contains("box1_wages"),
        "a refusal must emit NO scrubbed TOML on stdout: {stdout:?}"
    );
    assert!(
        stderr.to_lowercase().contains("blocker"),
        "the refusal must name the mechanism that caused it: {stderr:?}"
    );
}

// ── ★★★ THE POSITIVE CONTROL. Without it, `fn scrub() { refuse() }` passes both tests above. ─────

#[test]
fn a_ledger_free_vault_still_scrubs() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);
    assert_ledger_shape(
        &vault,
        Shape {
            in_year: false,
            prior_year: false,
            pseudo: false,
            hard_blocker: false,
        },
    );

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_eq!(
        code, 0,
        "a vault whose ledger contributes NOTHING must still scrub — a predicate that refuses \
         everything is not a safety gate, it is a broken command. stderr={stderr:?}"
    );
    assert!(
        stdout.contains("filing_status"),
        "the scrubbed TOML must actually be emitted: {stdout:?}"
    );
    assert!(
        !stdout.contains("111-22-3333"),
        "and it must be scrubbed: {stdout:?}"
    );
}

// ── §7 — a DRAFT shadows the committed row, and scrub must read what the filer is looking at ─────

/// ★★★ `return_inputs::get` sees only the COMMITTED row, but §6.1's precedence is that a
/// version-current DRAFT shadows it. A filer who edited their return in the input form and has not
/// committed is **looking at the draft** — and the defect they are writing in to report lives in the
/// return on their screen, not in the one underneath it.
///
/// Scrubbing the committed row would hand a stranger a DIFFERENT return and send them after a bug
/// that is not there, which is the same harm §2.2 refuses ledger-bearing years to avoid.
#[test]
fn scrub_reads_the_draft_that_shadows_the_committed_row() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault); // the COMMITTED row: $80,000 of wages

    // …then an uncommitted draft the filer is actually looking at, with a different figure.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut ri = btctax_cli::return_inputs::get(s.conn(), SCRUB_YEAR)
            .unwrap()
            .unwrap();
        // All three wage boxes, so `80000` cannot survive anywhere for an innocent reason and the
        // negative assertion below means what it says.
        ri.w2s[0].box1_wages = dec!(12345);
        ri.w2s[0].box3_ss_wages = dec!(12345);
        ri.w2s[0].box5_medicare_wages = dec!(12345);
        btctax_cli::input_form_store::save_draft(&mut s, SCRUB_YEAR, &ri).unwrap();
        s.save().unwrap();
    }

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("12345"),
        "scrub emitted the COMMITTED row, not the draft the filer is looking at: {stdout}"
    );
    assert!(
        !stdout.contains("80000"),
        "the shadowed committed figure must not be what travels: {stdout}"
    );
}

// ── §4 — THE ARTIFACT MUST BE SAFE AS A FILE. Each item lands with its planted-defect test (B1). ──

/// §4.1 — the `--out` file is OWNER-ONLY.
///
/// ★ It was a bare `std::fs::write`, which lands 0644 under the default umask: **world-readable**.
/// The identity in this file is synthetic; every FIGURE in it is real, so on a shared machine a
/// filer's whole income was readable by every other account on the box.
#[cfg(unix)]
#[test]
fn the_out_file_is_owner_only_even_when_it_already_exists() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);
    let out = dir.path().join("scrubbed.toml");

    // ★ PRE-CREATE it world-readable. `write_owner_only` creates with 0600, but an EXISTING path
    //   keeps its old mode — and scrub is exactly the command a filer re-runs to the same path.
    std::fs::write(&out, b"stale").unwrap();
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o644)).unwrap();

    let (code, _o, stderr) = run_btctax(
        &vault,
        &[
            "income",
            "scrub",
            "--year",
            "2024",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");

    let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "a scrubbed return must be readable only by its owner, got {mode:o}"
    );
}

/// §4.2 — the marker rides on the EMITTED STRING, so stdout and `--out` carry it identically.
#[test]
fn the_scrubbed_output_carries_the_provenance_marker_on_both_paths() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);
    let marker = btctax_core::tax::scrub::SCRUB_PROVENANCE_MARKER;

    let (code, stdout, stderr) = run_btctax(&vault, &["income", "scrub", "--year", "2024"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains(marker), "stdout is unmarked: {stdout}");

    let out = dir.path().join("scrubbed.toml");
    let (code, _o, stderr) = run_btctax(
        &vault,
        &[
            "income",
            "scrub",
            "--year",
            "2024",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        std::fs::read_to_string(&out).unwrap().contains(marker),
        "the --out file is unmarked, so re-importing it would be silently accepted"
    );
}

/// §4.3 — a marked file is REFUSED by `income import`, and `--force` is the only way past.
///
/// ★★ The harm is data loss: a scrubbed file is schema-identical to a real one and import is an
/// unconfirmed whole-blob upsert, so this would overwrite the vault's real identity and IP PIN —
/// unrestorable — leaving a synthetic SSN well-formed enough to print on a filed 1040.
#[test]
fn importing_a_scrubbed_file_is_refused_without_force_and_accepted_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    store_wage_only_return(&vault);
    let out = dir.path().join("scrubbed.toml");
    let (code, _o, _e) = run_btctax(
        &vault,
        &[
            "income",
            "scrub",
            "--year",
            "2024",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0);

    let args = [
        "income",
        "import",
        "--year",
        "2024",
        "--file",
        out.to_str().unwrap(),
    ];
    let (code, _o, stderr) = run_btctax(&vault, &args);
    assert_ne!(code, 0, "a marked file must be REFUSED");
    assert!(
        stderr.to_lowercase().contains("scrub"),
        "the refusal must say WHY: {stderr}"
    );

    let mut forced = args.to_vec();
    forced.push("--force");
    let (code, _o, stderr) = run_btctax(&vault, &forced);
    assert_eq!(code, 0, "--force must get past the marker guard: {stderr}");
}

/// ★★★ THE NEGATIVE TEST, and it is the one that matters most: the guard must not become a wall.
/// Both committed example TOMLs — ordinary, unscrubbed returns — still import CLEAN with no flag.
/// A marker check that reds on real files would break `income import` for everyone.
#[test]
fn the_committed_example_tomls_still_import_without_force() {
    for fixture in ["fullreturn_inputs.toml", "nine_dependents_amt_inputs.toml"] {
        let dir = tempfile::tempdir().unwrap();
        let vault = empty_vault(dir.path());
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/examples")
            .join(fixture);
        assert!(src.exists(), "fixture missing: {}", src.display());

        let (code, _o, stderr) = run_btctax(
            &vault,
            &[
                "income",
                "import",
                "--year",
                "2024",
                "--file",
                src.to_str().unwrap(),
            ],
        );
        assert_eq!(
            code, 0,
            "{fixture}: an ordinary return must import with no --force. stderr: {stderr}"
        );
    }
}

// ── §8 step 8 — the TOML ROUND TRIP. The output is only useful if it can be loaded back. ─────────

/// ★★★ A scrubbed copy nobody can load is a screenshot with extra steps. The whole product is that
/// the recipient can `import` it and reproduce the filer's numbers — so the round trip is the
/// feature, not a nicety.
///
/// ★ This also pins the marker's shape from the other side: it must be a COMMENT, invisible to
/// deserialization. A marker emitted as a TOML KEY would be rejected by
/// `parse_return_inputs_toml`'s unknown-key check and the file would be unloadable — which is
/// exactly why §4.2 specifies a comment line.
#[test]
fn the_scrubbed_toml_round_trips_back_through_import() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());

    // ★★★ THE MAXIMAL SENTINEL, not the wage-only fixture. `store_wage_only_return` leaves every
    //     nested shape at its default — no payments, no Schedule A/C, no dependents, no 1099s, no
    //     box-12 entries — so an emitter that DROPPED a whole block still compares equal on both
    //     sides. Mutation-verified: dropping `payments` SURVIVED against the thin fixture.
    //
    //     `maximal_sentinel` is the fixture §3.3 already maintains: every `Option` `Some`, every
    //     `Vec` with two elements, every nested struct present. It is also the only thing in the repo
    //     that emits an array-of-tables inside an array-of-tables (`[[w2s]]` carrying `box12`) —
    //     the shape this repo documents TOML serialization as fragile about.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            SCRUB_YEAR,
            &btctax_core::tax::scrub_axis::maximal_sentinel(),
        )
        .unwrap();
        s.save().unwrap();
    }

    let out = dir.path().join("scrubbed.toml");
    let (code, _o, stderr) = run_btctax(
        &vault,
        &[
            "income",
            "scrub",
            "--year",
            "2024",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");

    // A FRESH vault, which is the real workflow: the recipient is not the filer.
    let dir2 = tempfile::tempdir().unwrap();
    let recipient = empty_vault(dir2.path());
    let (code, _o, stderr) = run_btctax(
        &recipient,
        &[
            "income",
            "import",
            "--year",
            "2024",
            "--file",
            out.to_str().unwrap(),
            "--force",
        ],
    );
    assert_eq!(code, 0, "the scrubbed file must load: {stderr}");

    // ★★★ COMPARE AGAINST THE IN-MEMORY SCRUB, NOT AGAINST THE FILE. Parsing the emitted file and
    //     comparing it to what the recipient stored puts the FILE on both sides of the assertion, so
    //     it tests the storage layer and not the emitter. Every `ReturnInputs` field is
    //     `#[serde(default)]`, so a field the emitter DROPS is simply absent from the file, defaults
    //     on both sides, and compares equal — losing, say, the filer's estimated-tax payments and
    //     their balance due, while the whole suite stays green.
    //
    //     The filer's own scrubbed return is the only expectation that can red on emitter loss.
    let sent: btctax_core::tax::return_inputs::ReturnInputs = {
        let s = Session::open(&vault, &pp()).unwrap();
        let stored = btctax_cli::return_inputs::get(s.conn(), 2024)
            .unwrap()
            .unwrap();
        btctax_core::tax::scrub::scrub_pii(&stored)
    };
    let landed = {
        let s = Session::open(&recipient, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2024)
            .unwrap()
            .unwrap()
    };
    assert_eq!(
        sent, landed,
        "the return that landed differs from the one that was sent"
    );
}
