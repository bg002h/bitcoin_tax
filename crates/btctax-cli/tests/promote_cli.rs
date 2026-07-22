//! Conservative-filing Task 8 (CLI wiring) — the VOID-direction BG-D9 prior-year fold-diff advisory
//! reaches the real `btctax reconcile void` verb. Voiding a live `PromoteTranche` reverts a filed floor
//! basis toward `$0`, which HIFO-reorders a PRIOR filed year's disposals (amend-to-PAY). This drives the
//! actual binary (`std::process::Command`) so the wiring — not just the core builder — is exercised: the
//! `Direction::Void` lines must PRINT before the void is recorded.
//!
//! Setup is hand-built via `persistence` (there is no CLI `promote` verb yet — that consent screen is
//! Task 10 — so the promote is appended directly, exactly as `declare_tranche_cli.rs` hand-crafts a raw
//! void). PRIVACY: synthetic values in a tempdir; no user file is read.

use btctax_cli::cli::FormArg;
use btctax_cli::cmd::promote::ProvenanceKind;
use btctax_cli::eventref::parse_event_id;
use btctax_cli::{cmd, return_inputs, CliError, Session, PROMOTE_ACK_PHRASE};
use btctax_core::conservative::Coverage;
use btctax_core::event::{
    Acknowledgment, Acquire, BasisSource, ConsentTerm, DeclareTranche, Dispose, DisposeKind,
    EventPayload, FloorMethod, PromoteTranche,
};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::{append_decision, append_import_batch, load_all};
use btctax_core::LedgerEvent;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime};
use time::UtcOffset;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026 - 01 - 01 0:00 UTC)
}
/// The single Exchange wallet the documented buy, the tranche, and the sell all share — so a promoted
/// tranche can HIFO-reorder the sell's draw.
fn wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload,
    }
}

/// A vault with a documented 0.6-BTC lot ($5,000/BTC), a 0.4-BTC tranche PROMOTED to a $12,000 floor
/// ($30,000/BTC — higher per-sat, so HIFO draws it FIRST), and a 2018 sell of EXACTLY 0.4 BTC. WITH the
/// promote the sell drains the tranche (gain $8,000); voiding it reverts the tranche to $0 (sorted LAST),
/// so the sell instead drains the documented lot (gain $18,000) — the amend-to-PAY reorder the advisory
/// warns about. Returns (vault, promote decision id).
fn build_promoted_vault(dir: &Path) -> (PathBuf, EventId) {
    let vault = dir.join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();
    let buy = imp(
        "BUY",
        datetime!(2017-01-01 00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 60_000_000,
            usd_cost: dec!(3_000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = imp(
        "SELL",
        datetime!(2018-09-01 00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 40_000_000,
            usd_proceeds: dec!(20_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    append_import_batch(s.conn(), &[buy, sell]).unwrap();

    // DeclareTranche (decision 1) then PromoteTranche targeting it (decision 2).
    let tranche_id = append_decision(
        s.conn(),
        EventPayload::DeclareTranche(DeclareTranche {
            sat: 40_000_000,
            wallet: wallet(),
            window_start: time::macros::date!(2018 - 01 - 01),
            window_end: time::macros::date!(2018 - 03 - 31),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    let promote_id = append_decision(
        s.conn(),
        EventPayload::PromoteTranche(PromoteTranche {
            target: tranche_id,
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(12_000),
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I understand and accept the risk".into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    s.save().unwrap();
    (vault, promote_id)
}

/// Run `btctax --vault <vault> reconcile void <target>`; returns (exit, stdout, stderr).
fn run_void(vault: &Path, target: &str) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let out = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().unwrap())
        .arg("reconcile")
        .arg("void")
        .arg(target)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn decision_count(vault: &Path) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    load_all(s.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .count()
}

/// ★ Task 8 (§6 / arch r1 I-3): `reconcile void` on a live promote PRINTS the `Direction::Void`
/// prior-year advisory (amend-to-PAY) before recording, and still records the void. Dropping the wiring
/// leaves stdout without the 1040-X / additional-tax warning (surfacing mutation).
#[test]
fn voiding_a_promoted_tranche_prints_the_void_direction_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, promote_id) = build_promoted_vault(dir.path());
    let before = decision_count(&vault);

    let (code, stdout, stderr) = run_void(&vault, &promote_id.canonical());
    assert_eq!(code, 0, "the void must succeed; stderr: {stderr}");
    // The VOID-direction lines: the 2018 rewrite, its 1040-X implication, and the amend-to-PAY wording.
    assert!(
        stdout.contains("2018"),
        "the advisory names the affected filed year 2018: {stdout}"
    );
    assert!(
        stdout.contains("1040-X"),
        "the advisory names the Form 1040-X implication: {stdout}"
    );
    assert!(
        stdout.to_lowercase().contains("additional tax"),
        "voiding a promote over a filed floor-year is amend-to-PAY (additional tax): {stdout}"
    );
    // The void was still recorded (the advisory is non-gating).
    assert_eq!(
        decision_count(&vault),
        before + 1,
        "the void decision is still recorded after the warning"
    );
}

/// ★ arch r1 M-3 (Phase-1a fold): voiding the promoted **DeclareTranche** (NOT the promote) is REFUSED
/// upstream by `guard_decision_conflict`/`would_conflict` at record time — the engine holds the tranche in
/// force via its live promote (an inert `DecisionConflict`, `resolve.rs` tranche-void adjudication). The
/// refusal fires BEFORE `promote_void_advisory_lines`, so NO Void-direction amend-to-PAY advisory is ever
/// printed for this (inert) void. This regression-locks the guard so the removed `.or_else` DeclareTranche
/// branch (which would have wrongly printed a reversion + amend-to-PAY for a void that changes nothing)
/// stays unreachable. Green both before AND after the `.or_else` removal — it pins the reachable guardrail,
/// not a mutation of the dead branch (the behavior was already correct; this locks it).
#[test]
fn voiding_a_promoted_declare_tranche_is_refused_and_prints_no_amend_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _promote_id) = build_promoted_vault(dir.path());
    // The DeclareTranche decision the promote targets (build_promoted_vault appends it as decision 1).
    let tranche_id = {
        let s = Session::open(&vault, &pp()).unwrap();
        let id = load_all(s.conn())
            .unwrap()
            .into_iter()
            .find(|e| matches!(e.payload, EventPayload::DeclareTranche(_)))
            .map(|e| e.id)
            .expect("the promoted DeclareTranche is present");
        id
    };
    let before = decision_count(&vault);

    let (code, stdout, stderr) = run_void(&vault, &tranche_id.canonical());

    // Refused at record time with the inert-void `DecisionConflict` reason (the resolver == record-time
    // guard by construction).
    assert_ne!(
        code, 0,
        "voiding a promote-held DeclareTranche must be refused; stdout={stdout} stderr={stderr}"
    );
    assert!(
        stderr.contains("cannot record this decision")
            && stderr.contains("held in force by a live PromoteTranche"),
        "the refusal names the inert-void reason: {stderr}"
    );
    // The wrong guidance never prints: no Void-direction amend-to-PAY advisory reaches stdout.
    assert!(
        !stdout.to_lowercase().contains("additional tax") && !stdout.contains("1040-X"),
        "an inert (refused) void must print NO amend-to-PAY promote-void advisory: {stdout}"
    );
    // Fail-closed: nothing was recorded.
    assert_eq!(
        decision_count(&vault),
        before,
        "a refused void appends no decision"
    );
}

/// ★ Task 11 (BG-D3, arch r3 M-1): the verify-drift advisory is WIRED into `verify`/`build_verify`, not
/// just the core fn. `build_promoted_vault` files a $12,000 floor for a 0.4-BTC 2018-Q1 tranche
/// ($30,000/BTC — far ABOVE any 2018 daily close), so recomputing `filed_basis_for` against the CURRENT
/// bundled prices lands well below the stored floor ⇒ the OVERSTATED-basis drift advisory fires and rides
/// the `VerifyReport.drift` field. Threading a `PriceProvider` into `verify` is what makes this non-vacuous.
#[test]
fn verify_surfaces_the_promote_drift_advisory_for_a_drifted_promote() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _promote_id) = build_promoted_vault(dir.path());
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(
        !report.drift.is_empty(),
        "verify's VerifyReport.drift must be non-empty for a drifted promote (wired into build_verify): \
         {:?}",
        report.drift
    );
    assert!(
        report
            .drift
            .iter()
            .any(|l| l.contains("void") && l.contains("re-promote") && l.contains("not yet filed")),
        "the stored floor is OVERSTATED (recomputes lower) → the conditional void+re-promote copy: {:?}",
        report.drift
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 10 — the `promote-tranche` verb: BG-D5 provenance + BG-D6 consent recording + BG-D7 Part II.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The wallet MY OWN `declare_tranche` fixtures below use (distinct from `wallet()` above, which the T8
/// fixtures share with a documented buy/sell).
fn tranche_wallet() -> WalletId {
    WalletId::SelfCustody {
        label: "promote-t10".into(),
    }
}

fn count<P: Fn(&EventPayload) -> bool>(vault: &Path, pred: P) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    load_all(s.conn())
        .unwrap()
        .iter()
        .filter(|e| pred(&e.payload))
        .count()
}

/// The single recorded `PromoteTranche` payload (panics if there isn't exactly one).
fn only_promote(vault: &Path) -> PromoteTranche {
    let s = Session::open(vault, &pp()).unwrap();
    load_all(s.conn())
        .unwrap()
        .into_iter()
        .find_map(|e| match e.payload {
            EventPayload::PromoteTranche(p) => Some(p),
            _ => None,
        })
        .expect("exactly one PromoteTranche recorded")
}

/// A vault with a single declared (UNPROMOTED) tranche inside a fully price-covered window
/// (2020-01-01..2020-01-10 — the bundled daily-close dataset spans 2010-07-17..release with no gaps, so
/// `filed_basis_for` always succeeds here). Returns (vault, the tranche's canonical target ref).
fn vault_with_tranche(dir: &Path) -> (PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        40_000_000,
        tranche_wallet(),
        date!(2020 - 01 - 01),
        date!(2020 - 01 - 10),
        now(),
    )
    .unwrap();
    (vault, id.canonical())
}

/// A vault with a tranche that is ALREADY promoted (hand-crafted, mirroring `declare_tranche_cli.rs`'s
/// `promote_ev` style) — so a SECOND `promote_tranche` call on the same target must be refused by
/// `would_conflict` (BG-D9). Returns (vault, the ORIGINAL tranche's target ref).
fn vault_with_promoted_tranche(dir: &Path) -> (PathBuf, String) {
    let (vault, target_ref) = vault_with_tranche(dir);
    let target = parse_event_id(&target_ref).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    append_decision(
        s.conn(),
        EventPayload::PromoteTranche(PromoteTranche {
            target,
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(1_000),
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: PROMOTE_ACK_PHRASE.into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    s.save().unwrap();
    (vault, target_ref)
}

fn consent_terms_fixture() -> Vec<ConsentTerm> {
    vec![ConsentTerm::ComputedTax {
        year: 2020,
        delta_usd: dec!(500),
        deduction_delta_usd: None,
    }]
}

fn consent_terms_with_deduction_and_unrealized() -> Vec<ConsentTerm> {
    vec![
        ConsentTerm::ComputedTax {
            year: 2020,
            delta_usd: dec!(0),
            deduction_delta_usd: Some(dec!(300)),
        },
        ConsentTerm::Unrealized {
            sat: 10_000_000,
            hypothetical_reduction: Some(dec!(1_000)),
            as_of: Some(date!(2020 - 06 - 01)),
        },
    ]
}

/// §6 / tax r1 M-6: refuse Gift/Inheritance/Mining/Earned/Airdrop/Fork — not just Gift (BG-D5's closed
/// enumeration). Fail-closed: nothing is ever recorded across the whole sweep.
#[test]
fn every_non_purchase_provenance_is_refused_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_tranche(dir.path());
    for pk in [
        ProvenanceKind::Gift,
        ProvenanceKind::Inheritance,
        ProvenanceKind::Mining,
        ProvenanceKind::Earned,
        ProvenanceKind::Airdrop,
        ProvenanceKind::Fork,
    ] {
        let err =
            cmd::promote::promote_tranche(&vault, &pp(), &target, pk, "facts".into(), None, now())
                .unwrap_err();
        assert!(
            matches!(err, CliError::Usage(ref m) if m.contains("purchase") && m.contains("real acquisition")),
            "{pk:?} must be refused naming 'purchase' + 'real acquisition': {err}"
        );
    }
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::PromoteTranche(_))),
        0,
        "fail-closed: nothing recorded across the whole non-purchase sweep (BG-D5)"
    );
}

/// BG-D7: an empty/whitespace Part II narrative is refused AT RECORD TIME (present-by-construction) —
/// even with a valid provenance and a correct acknowledgment.
#[test]
fn empty_part_ii_narrative_is_refused_at_record_time() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_tranche(dir.path());
    let err = cmd::promote::promote_tranche(
        &vault,
        &pp(),
        &target,
        ProvenanceKind::Purchase,
        "  ".into(),
        Some(PROMOTE_ACK_PHRASE),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(ref m) if m.contains("Part II")),
        "an empty narrative must be refused naming 'Part II': {err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::PromoteTranche(_))),
        0
    );
}

/// A fully-valid promote records: the filed_basis floor is `>$0`, the acknowledgment phrase is stored
/// verbatim, and `provenance_attested` is `true`.
#[test]
fn a_recorded_promote_carries_the_acknowledgment_and_stored_floor() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_tranche(dir.path());
    cmd::promote::promote_tranche(
        &vault,
        &pp(),
        &target,
        ProvenanceKind::Purchase,
        "cash P2P, no records".into(),
        Some(PROMOTE_ACK_PHRASE),
        now(),
    )
    .unwrap();
    let p = only_promote(&vault);
    assert!(
        p.filed_basis > btctax_core::Usd::ZERO,
        "filed_basis must be > $0: {p:?}"
    );
    assert!(!p.acknowledgment.phrase.is_empty());
    assert_eq!(p.acknowledgment.phrase, PROMOTE_ACK_PHRASE);
    assert!(p.provenance_attested);
    assert_eq!(p.part_ii_narrative, "cash P2P, no records");
}

/// BG-D9: a second promote on an already-promoted target is refused via the `would_conflict` pre-check
/// (NOT last-wins) — the record-time UX-P4-3 layer over the engine's own `DecisionConflict`.
#[test]
fn a_second_promote_is_refused_by_would_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_promoted_tranche(dir.path());
    let err = cmd::promote::promote_tranche(
        &vault,
        &pp(),
        &target,
        ProvenanceKind::Purchase,
        "x".into(),
        Some(PROMOTE_ACK_PHRASE),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(ref m) if m.contains("conflict")),
        "a second promote must be refused naming 'conflict': {err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::PromoteTranche(_))),
        1,
        "still exactly the ORIGINAL promote — the second attempt appended nothing"
    );
}

/// §6 copy bullet covers the CONSENT copy too (not just the 8275/T13 narrative): the penalty base names
/// "of the resulting additional tax" + "plus interest", and NEVER says "safe harbor" (not even to deny it).
#[test]
fn the_consent_copy_names_the_underpayment_base_and_never_says_safe_harbor() {
    let screen = cmd::promote::render_consent(&consent_terms_fixture(), &BTreeSet::new());
    assert!(!screen.to_lowercase().contains("safe harbor"));
    assert!(screen.contains("of the resulting additional tax") && screen.contains("plus interest"));
}

/// tax r2 M-2: a fixture with a `ComputedTax{deduction_delta: Some}` term AND an `Unrealized` term pins
/// BOTH labels the consent screen must carry.
#[test]
fn consent_copy_pins_the_deduction_exclusion_and_unrealized_labels() {
    let screen = cmd::promote::render_consent(
        &consent_terms_with_deduction_and_unrealized(),
        &BTreeSet::new(),
    );
    assert!(
        screen.contains("does NOT capture this charitable-deduction change"),
        "tax-Δ-excludes-deduction sentence: {screen}"
    );
    assert!(
        screen.contains("hypothetical, not a filed figure"),
        "unrealized label rendered: {screen}"
    );
}

/// T9 handoff (progress.md): `consent_terms`'s `deduction_delta_usd` sums the §170(e) charitable-deduction
/// change AND the §1015 gift-basis change into one figure. When the render is told (via
/// `gift_only_years`) that a flagged year's removal was GIFT-only, it must label that year's Δ as a
/// donee-basis (§1015) documentation change — the donor's 1040 is unaffected — NEVER a Schedule-A
/// deduction.
#[test]
fn consent_copy_labels_a_gift_only_year_as_donee_basis_not_schedule_a() {
    let terms = vec![ConsentTerm::ComputedTax {
        year: 2020,
        delta_usd: dec!(0),
        deduction_delta_usd: Some(dec!(400)),
    }];
    let mut gift_only_years = BTreeSet::new();
    gift_only_years.insert(2020);
    let screen = cmd::promote::render_consent(&terms, &gift_only_years);
    assert!(
        screen.contains("donee-basis (§1015)") && screen.contains("donor's 1040 is unaffected"),
        "a gift-only year must be labeled donee-basis, not Schedule-A: {screen}"
    );
    // The disambiguation is an explicit "NOT a Schedule-A deduction" qualifier (not a bare, unqualified
    // "Schedule-A deduction" claim) — assert the qualifier, not mere absence of the substring (which
    // would also fail the correct denial wording, unlike the SPEC's literal "never says safe harbor").
    assert!(
        screen.contains("NOT a Schedule-A deduction"),
        "the gift-only year's Δ must be explicitly denied as a Schedule-A deduction: {screen}"
    );
}

/// The SAME gift-only relabeling applies to an `Uncomputable` term (its `deduction_delta_usd` is a bare
/// `Usd`, not `Option`, but the mislabeling risk — and the T9 handoff — is identical).
#[test]
fn consent_copy_labels_a_gift_only_uncomputable_year_as_donee_basis_not_schedule_a() {
    let terms = vec![ConsentTerm::Uncomputable {
        year: 2019,
        gain_delta_usd: dec!(0),
        deduction_delta_usd: dec!(250),
    }];
    let mut gift_only_years = BTreeSet::new();
    gift_only_years.insert(2019);
    let screen = cmd::promote::render_consent(&terms, &gift_only_years);
    assert!(
        screen.contains("donee-basis (§1015)") && screen.contains("NOT a Schedule-A deduction"),
        "a gift-only UNCOMPUTABLE year must also be labeled donee-basis, not Schedule-A: {screen}"
    );
}

/// Run `btctax --vault <vault> reconcile promote-tranche <target> --provenance purchase --part-ii-file
/// <path> [extra...]`; returns (exit, stdout, stderr). stdin is NOT a terminal under `Command::output()`,
/// so this always drives the NON-interactive main.rs branch.
fn run_promote(
    vault: &Path,
    target: &str,
    part_ii_path: &Path,
    extra: &[&str],
) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let mut c = std::process::Command::new(bin);
    c.arg("--vault")
        .arg(vault.to_str().unwrap())
        .arg("reconcile")
        .arg("promote-tranche")
        .arg(target)
        .arg("--provenance")
        .arg("purchase")
        .arg("--part-ii-file")
        .arg(part_ii_path.to_str().unwrap())
        .env("BTCTAX_PASSPHRASE", "pw");
    for a in extra {
        c.arg(a);
    }
    let out = c.output().expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// N-2 (BG-D6): the non-TTY path with no `--i-acknowledge` still REFUSES (exit != 0) but prints the
/// computed consent figures to stdout BEFORE refusing — a scripted caller can always see what it declined
/// to acknowledge. Drives the REAL binary so the main.rs wiring (not just the library fn) is exercised.
#[test]
fn non_tty_missing_acknowledge_still_prints_the_consent_screen_and_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_tranche(dir.path());
    let part_ii = dir.path().join("part_ii.txt");
    std::fs::write(
        &part_ii,
        "cash P2P purchase, no records; window bounded on-chain",
    )
    .unwrap();

    let (code, stdout, stderr) = run_promote(&vault, &target, &part_ii, &[]);
    assert_ne!(
        code, 0,
        "missing --i-acknowledge must refuse; stderr: {stderr}"
    );
    assert!(
        stdout.contains("of the resulting additional tax"),
        "the consent screen prints even on refusal (N-2): {stdout}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::PromoteTranche(_))),
        0,
        "the refused promote must NOT be appended (fail-closed)"
    );

    // The non-interactive success path: --i-acknowledge with the exact phrase records it.
    let (code2, stdout2, stderr2) = run_promote(
        &vault,
        &target,
        &part_ii,
        &["--i-acknowledge", PROMOTE_ACK_PHRASE],
    );
    assert_eq!(
        code2, 0,
        "a correct --i-acknowledge must succeed; stderr: {stderr2}"
    );
    assert!(stdout2.contains("of the resulting additional tax"));
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::PromoteTranche(_))),
        1
    );
}

/// A vault with a tranche declared over a WIDE (> 1 year) window — still fully price-covered.
fn vault_with_wide_window_tranche(dir: &Path) -> (PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        40_000_000,
        tranche_wallet(),
        date!(2015 - 01 - 01),
        date!(2018 - 01 - 01),
        now(),
    )
    .unwrap();
    (vault, id.canonical())
}

/// SPEC §1 "two honest limits": a wide (> 1 year) declared window tends to yield a LOW ("trivial")
/// floor — the CONSENT flow must surface this caution so the filer can weigh whether promoting is even
/// worth the Form 8275 disclosure surface.
#[test]
fn a_wide_window_promote_prints_the_trivial_floor_caution() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_wide_window_tranche(dir.path());
    let part_ii = dir.path().join("part_ii.txt");
    std::fs::write(
        &part_ii,
        "cash P2P purchase, no records; wide multi-year window",
    )
    .unwrap();

    let (code, stdout, stderr) = run_promote(
        &vault,
        &target,
        &part_ii,
        &["--i-acknowledge", PROMOTE_ACK_PHRASE],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("trivial") && lower.contains("wide"),
        "a wide window must print the trivial-floor caution: {stdout}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 14 — BG-D8 the export-refusal COMPLETENESS gate: a promoted-basis DISPOSAL leg filed WITHOUT its
// complete Form 8275 is a HARD REFUSAL (Reg §1.6662-4(f): disclosure is adequate only on a COMPLETED
// Form 8275). A REAL refuse-before-bytes gate (the pseudo-export-block precedent), NOT the always-written
// basis_methodology.txt pattern; on SUCCESS the disclosure is emitted by its OWN name (form_8275.txt).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The tax year both Task-14 fixtures dispose in — a SHIPPED IRS-PDF year (this build bundles 2017/2024/
/// 2025) so the CLEAN export actually fills a packet; the tranche is declared pre-2025 so a promote is
/// meaningful.
const T14_YEAR: i32 = 2024;

/// A 2024 sell of exactly 0.4 BTC in `wallet()` — drains the 0.4-BTC tranche (its only lot), so the
/// resulting disposal leg is a PROMOTED leg filed in `T14_YEAR`.
fn t14_sell() -> LedgerEvent {
    imp(
        "T14-SELL",
        datetime!(2024 - 09 - 01 0:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 40_000_000,
            usd_proceeds: dec!(20_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}

/// ★ The raw-vault BYPASS: declare a tranche (CLI), then HAND-APPEND a `PromoteTranche` with an EMPTY
/// `part_ii_narrative` — the T10 CLI refuses an empty narrative at record time (BG-D7), so only a raw
/// `append_decision` can force `disclosure_8275().incomplete == true` — then import a sell that drains the
/// promoted tranche. Net effect: a promoted DISPOSAL leg filed in 2024 whose Form 8275 Part II is empty.
fn raw_vault_promote_with_empty_part_ii(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let tranche_id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        40_000_000,
        wallet(),
        date!(2024 - 01 - 01),
        date!(2024 - 03 - 31),
        now(),
    )
    .unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    append_decision(
        s.conn(),
        EventPayload::PromoteTranche(PromoteTranche {
            target: tranche_id,
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(12_000),
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: PROMOTE_ACK_PHRASE.into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: String::new(), // ★ EMPTY — the raw-vault bypass (the CLI refuses this)
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    append_import_batch(s.conn(), &[t14_sell()]).unwrap();
    s.save().unwrap();
    vault
}

/// The T10-path CLEAN fixture: declare a tranche (CLI) + promote it via the REAL `promote-tranche` verb
/// (which enforces a non-empty Part II — a COMPLETE Form 8275), then import a sell that drains it — a
/// promoted 2024 disposal leg with a complete disclosure.
fn vault_with_promoted_disposal_via_cli(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let tranche_id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        40_000_000,
        wallet(),
        date!(2024 - 01 - 01),
        date!(2024 - 03 - 31),
        now(),
    )
    .unwrap();
    cmd::promote::promote_tranche(
        &vault,
        &pp(),
        &tranche_id.canonical(),
        ProvenanceKind::Purchase,
        "cash P2P purchase, no records; window bounded on-chain".into(),
        Some(PROMOTE_ACK_PHRASE),
        now(),
    )
    .unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    append_import_batch(s.conn(), &[t14_sell()]).unwrap();
    s.save().unwrap();
    vault
}

/// ★ BG-D8: an export whose packet contains a promoted-basis leg but only an INCOMPLETE Form 8275 (empty
/// Part II) is REFUSED — and refused BEFORE any bytes are written (the out_dir is left untouched). The
/// refusing state is reached only via the raw-vault bypass (the T10 CLI can't record an empty narrative).
#[test]
fn export_with_a_promoted_leg_but_incomplete_8275_refuses_before_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let vault = raw_vault_promote_with_empty_part_ii(dir.path());
    let out = dir.path().join("export_out"); // deliberately NOT pre-created

    let err = cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(ref m) if m.contains("Form 8275")),
        "a promoted leg without a complete Form 8275 must be REFUSED naming 'Form 8275': {err}"
    );
    // Refuse-before-bytes: a refused export writes ZERO bytes — the out_dir was never even created (or is
    // empty). This is what makes it a REAL gate, not the always-writes basis_methodology.txt pattern.
    assert!(
        std::fs::read_dir(&out)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "a refused export leaves out_dir untouched (zero bytes written)"
    );
}

/// ★ BG-D8: a CLEAN promoted export (real ledger, complete Form 8275) SUCCEEDS and emits the disclosure by
/// its OWN name — `form_8275.txt`, NOT `form_8275.txt || basis_methodology.txt` (basis_methodology is
/// ALWAYS written, so the disjunction would be a vacuous assertion — tax r1 I-8). No DRAFT watermark.
#[test]
fn a_clean_promoted_export_writes_the_8275_by_name_no_watermark() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_promoted_disposal_via_cli(dir.path());
    let out = dir.path().join("export_out");

    let report = cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None).unwrap();
    assert!(
        out.join("form_8275.txt").exists(),
        "a clean promoted export emits the 8275 content by its OWN name (form_8275.txt)"
    );
    // Clean export — a real (not pseudo) ledger is never DRAFT-watermarked.
    assert!(
        !report.watermarked,
        "a real promoted ledger exports CLEAN (no DRAFT watermark)"
    );
}

/// ★ Whole-branch tax M-1: the Form 8275 is the MANDATORY disclosure — it must travel WITH the promoted
/// 8949 position, so it rides UNCONDITIONALLY even when `--forms f8949` narrows the slice to exclude it.
/// Otherwise the estimate position would export without its official disclosure PDF (Reg §1.6662-4(f)
/// makes disclosure adequate only on a COMPLETED Form 8275). Mutation-verified: restoring the
/// `wants(forms, FormArg::Form8275)` gate around the 8275 fill reds this.
#[test]
fn a_narrowed_forms_f8949_slice_still_emits_the_mandatory_8275_pdf_on_a_promoted_year() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_promoted_disposal_via_cli(dir.path());
    let out = dir.path().join("export_out");

    // `--forms f8949` ONLY — deliberately excludes f8275 from the requested slice.
    let report =
        cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[FormArg::F8949], None).unwrap();
    assert!(
        out.join("f8949.pdf").exists(),
        "the requested 8949 slice is written"
    );
    assert!(
        out.join("form_8275.pdf").exists(),
        "the MANDATORY 8275 disclosure PDF rides even though --forms excluded it (BG-D8)"
    );
    assert!(
        report.form_8275_path.is_some(),
        "the report records the mandatory 8275 PDF path"
    );
}

/// Plant a minimally-valid `return_inputs` row for `year` so `export_irs_pdf` DISPATCHES to the
/// full-return pipeline (`return_inputs::exists` is the dispatch's own discriminator, admin.rs
/// `export_irs_pdf`). Mirrors `export_irs_pdf.rs`'s
/// `export_dispatches_a_full_return_year_to_the_full_packet` fixture — an identified Single filer with
/// every live declaration answered, so a real (non-refusing) full-return packet can assemble.
fn plant_full_return_ri(vault: &Path, year: i32) {
    use btctax_core::tax::return_inputs::{Person, ReturnInputs};
    use btctax_core::tax::types::FilingStatus;

    let mut s = Session::open(vault, &pp()).unwrap();
    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        header: btctax_core::tax::testonly::not_a_dependent(),
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "Pat".into(),
        last_name: "Roe".into(),
        ssn: "222-33-4444".into(),
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    return_inputs::set(s.conn(), year, &ri).unwrap();
    s.save().unwrap();
}

/// ★ Phase-1a T14 (BG-D8): the refuse-before-bytes gate at the FULL-RETURN placement
/// (`export_full_return`'s OWN `promote_export_gate` call, admin.rs) — distinct from the crypto-slice
/// call site `export_with_a_promoted_leg_but_incomplete_8275_refuses_before_bytes` pins above. Planting
/// a `return_inputs` row for `T14_YEAR` on the SAME raw-vault-incomplete fixture makes `export_irs_pdf`
/// DISPATCH to `export_full_return`, so this confirms that placement's gate call fires too — before any
/// bytes are written.
#[test]
fn export_full_return_refuses_before_bytes_on_incomplete_8275() {
    let dir = tempfile::tempdir().unwrap();
    let vault = raw_vault_promote_with_empty_part_ii(dir.path());
    plant_full_return_ri(&vault, T14_YEAR);
    let out = dir.path().join("export_out"); // deliberately NOT pre-created

    let err = cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(ref m) if m.contains("Form 8275")),
        "the FULL-RETURN placement must refuse a promoted leg without a complete Form 8275: {err}"
    );
    assert!(
        std::fs::read_dir(&out)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "a refused full-return export leaves out_dir untouched (zero bytes written)"
    );
}

/// ★ Phase-1a T14 (BG-D8): the SAME refuse-before-bytes gate at the `export_snapshot` placement — a
/// THIRD distinct call site (`export_snapshot`'s own `promote_export_gate` call, admin.rs) from both
/// `export_irs_pdf`'s crypto-slice and full-return placements.
#[test]
fn export_snapshot_refuses_before_bytes_on_incomplete_8275() {
    let dir = tempfile::tempdir().unwrap();
    let vault = raw_vault_promote_with_empty_part_ii(dir.path());
    let out = dir.path().join("export_out"); // deliberately NOT pre-created

    let err = cmd::admin::export_snapshot(&vault, &pp(), &out, Some(T14_YEAR), None).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(ref m) if m.contains("Form 8275")),
        "export_snapshot must refuse a promoted leg without a complete Form 8275: {err}"
    );
    assert!(
        std::fs::read_dir(&out)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "a refused export_snapshot leaves out_dir untouched (zero bytes written)"
    );
}

/// ★ Phase-1a T14 (BG-D8): the success-emit of `form_8275.txt` at the FULL-RETURN placement — a
/// COMPLETE promoted disclosure (T10-recorded, non-empty Part II), exported via `export_irs_pdf`
/// DISPATCHED to `export_full_return` (a `return_inputs` row planted for `T14_YEAR`), writes the
/// disclosure by its OWN name — exactly as `a_clean_promoted_export_writes_the_8275_by_name_no_
/// watermark` pins for the crypto-slice call site.
#[test]
fn export_full_return_writes_form_8275_txt_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_promoted_disposal_via_cli(dir.path());
    plant_full_return_ri(&vault, T14_YEAR);
    let out = dir.path().join("export_out");

    cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None)
        .expect("a complete promoted disclosure exports via the dispatched full-return path");
    assert!(
        out.join("form_8275.txt").exists(),
        "the full-return packet emits the 8275 content by its OWN name (form_8275.txt)"
    );
}

/// ★ Phase-1a T14 (BG-D8): the SAME success-emit of `form_8275.txt` at the `export_snapshot`
/// placement.
#[test]
fn export_snapshot_writes_form_8275_txt_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_promoted_disposal_via_cli(dir.path());
    let out = dir.path().join("export_out");

    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(T14_YEAR), None)
        .expect("a complete promoted disclosure exports via export_snapshot");
    assert!(
        out.join("form_8275.txt").exists(),
        "export_snapshot emits the 8275 content by its OWN name (form_8275.txt)"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 16 — wiring the OFFICIAL Form 8275 fillable PDF into the export paths; BG-D8's gate re-pointed
// at it. Four KATs: year-2025/2017 end-to-end (arch r1 I-6 — the year-aliasing must reach the export
// layer, not just `fill_form_8275` itself, which `sp4.rs` already pins), the completeness gate still
// refuses before ANY byte (including the new PDF) when Part II is incomplete, a > 6-leg promoted year
// refuses CLEANLY (no panic, no half-written packet), and the all-years `export_snapshot` dump still
// co-emits one `form_8275_{year}.txt` per promoted year (M2).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Generalizes `vault_with_promoted_disposal_via_cli` (T14, fixed to 2024) to an ARBITRARY supported
/// `year` — T16's KAT needs 2025 and 2017, since Form 8275's bundled asset aliases every
/// `SUPPORTED_YEAR`. Same shape: one declared+promoted tranche (a Q1 `year` window), drained by a
/// single Q3 `year` sell — a promoted DISPOSAL leg filed in `year` with a COMPLETE Form 8275.
fn vault_with_promoted_disposal_via_cli_year(dir: &Path, year: i32) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let window_start = time::Date::from_calendar_date(year, time::Month::January, 1).unwrap();
    let window_end = time::Date::from_calendar_date(year, time::Month::March, 31).unwrap();
    let tranche_id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        40_000_000,
        wallet(),
        window_start,
        window_end,
        now(),
    )
    .unwrap();
    cmd::promote::promote_tranche(
        &vault,
        &pp(),
        &tranche_id.canonical(),
        ProvenanceKind::Purchase,
        "cash P2P purchase, no records; window bounded on-chain".into(),
        Some(PROMOTE_ACK_PHRASE),
        now(),
    )
    .unwrap();
    let sell_ts = time::Date::from_calendar_date(year, time::Month::September, 1)
        .unwrap()
        .with_hms(0, 0, 0)
        .unwrap()
        .assume_utc();
    let sell = imp(
        "T16-SELL",
        sell_ts,
        EventPayload::Dispose(Dispose {
            sat: 40_000_000,
            usd_proceeds: dec!(20_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let mut s = Session::open(&vault, &pp()).unwrap();
    append_import_batch(s.conn(), &[sell]).unwrap();
    s.save().unwrap();
    vault
}

/// ★ T16 KAT 1 (plan Step-1; arch r1 I-6 — the most important). A promoted disposal filed in a
/// NON-2024 supported year (2025, then 2017) exports the OFFICIAL Form 8275 PDF via the crypto-slice
/// `export_irs_pdf` — proving the bundled 8275 asset's year-aliasing ({2017, 2024, 2025}) works
/// END-TO-END at the export layer (`sp4.rs` already pins the aliasing inside `fill_form_8275` itself;
/// this is the wiring on top of it). A promoted CURRENT-year (2025) export must never be permanently
/// refused for want of a "2025 revision" that does not exist.
#[test]
fn a_promoted_2025_export_fills_the_8275_and_the_gate_passes() {
    for year in [2025, 2017] {
        let dir = tempfile::tempdir().unwrap();
        let vault = vault_with_promoted_disposal_via_cli_year(dir.path(), year);
        let out = dir.path().join("export_out");

        let report = cmd::admin::export_irs_pdf(&vault, &pp(), &out, year, &[], None)
            .unwrap_or_else(|e| panic!("year {year}: a complete promote must export cleanly: {e}"));
        assert!(
            out.join("form_8275.pdf").exists(),
            "year {year}: the crypto-slice export must fill the official Form 8275 PDF"
        );
        assert_eq!(
            report.form_8275_path,
            Some(out.join("form_8275.pdf")),
            "year {year}: the report names the written 8275 PDF"
        );
        assert!(
            !report.watermarked,
            "year {year}: a real (non-pseudo) ledger exports CLEAN"
        );
    }
}

/// ★ T16 KAT 2 (plan Step-1). The SAME BG-D8 completeness gate — now with the crypto-slice path ALSO
/// trying to fill the official PDF (not just `form_8275.txt`) — still refuses BEFORE any byte reaches
/// disk when a promoted leg's Part II is incomplete: neither the content file nor the new PDF partially
/// writes.
#[test]
fn export_gate_now_refuses_when_the_8275_pdf_is_absent() {
    let dir = tempfile::tempdir().unwrap();
    let vault = raw_vault_promote_with_empty_part_ii(dir.path());
    let out = dir.path().join("export_out"); // deliberately NOT pre-created

    let err = cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(ref m) if m.contains("Form 8275")),
        "an incomplete Form 8275 must still refuse the export post-T16: {err}"
    );
    assert!(
        std::fs::read_dir(&out)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "a refused export leaves out_dir untouched — not even a partial form_8275.pdf"
    );
    assert!(!out.join("form_8275.pdf").exists());
    assert!(!out.join("form_8275.txt").exists());
}

/// A vault with `n` DISTINCT promoted disposal legs, each in its OWN wallet (so its sell can only
/// drain its own tranche's lot — no cross-wallet lot-selection ordering to reason about), all filed in
/// `year`.
fn vault_with_n_promoted_disposal_legs(dir: &Path, year: i32, n: u32) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let window_start = time::Date::from_calendar_date(year, time::Month::January, 1).unwrap();
    let window_end = time::Date::from_calendar_date(year, time::Month::March, 31).unwrap();
    let sell_ts = time::Date::from_calendar_date(year, time::Month::September, 1)
        .unwrap()
        .with_hms(0, 0, 0)
        .unwrap()
        .assume_utc();
    for i in 0..n {
        let w = WalletId::Exchange {
            provider: "coinbase".into(),
            account: format!("overflow{i}"),
        };
        let tranche_id = cmd::tranche::declare_tranche(
            &vault,
            &pp(),
            40_000_000,
            w.clone(),
            window_start,
            window_end,
            now(),
        )
        .unwrap();
        cmd::promote::promote_tranche(
            &vault,
            &pp(),
            &tranche_id.canonical(),
            ProvenanceKind::Purchase,
            format!("cash P2P purchase #{i}, no records; window bounded on-chain"),
            Some(PROMOTE_ACK_PHRASE),
            now(),
        )
        .unwrap();
        let mut s = Session::open(&vault, &pp()).unwrap();
        append_import_batch(
            s.conn(),
            &[LedgerEvent {
                id: EventId::import(
                    Source::Coinbase,
                    SourceRef::new(format!("OVERFLOW-SELL-{i}")),
                ),
                utc_timestamp: sell_ts,
                original_tz: UtcOffset::UTC,
                wallet: Some(w),
                payload: EventPayload::Dispose(Dispose {
                    sat: 40_000_000,
                    usd_proceeds: dec!(20_000),
                    fee_usd: dec!(0),
                    kind: DisposeKind::Sell,
                }),
            }],
        )
        .unwrap();
        s.save().unwrap();
    }
    vault
}

/// ★ T16 KAT 3: Form 8275 v1 does not paginate — a year with MORE promoted disposal legs than the
/// revision's 6-row Part I capacity refuses CLEANLY (naming the year, the leg count, and a remedy),
/// never panics, and writes ZERO bytes (refuse-before-bytes) — pinning `export_full_return`'s Task-16 /
/// ADD-2 pre-check (admin.rs). Full-return tables are TY2024-only (v1), so the overflowing year must be
/// `T14_YEAR`.
#[test]
fn promoted_export_with_more_than_6_legs_refuses_cleanly_not_panics() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_n_promoted_disposal_legs(dir.path(), T14_YEAR, 7);
    plant_full_return_ri(&vault, T14_YEAR);
    let out = dir.path().join("export_out"); // deliberately NOT pre-created

    // This KAT's own job is to prove there is NO panic — assert it directly rather than relying on the
    // harness to merely report one.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None)
    }));
    let err = match result {
        Ok(Err(e)) => e,
        Ok(Ok(_)) => {
            panic!("a 7-leg promoted year (> the 6-row capacity) must refuse, not succeed")
        }
        Err(_) => panic!("a 7-leg promoted year must refuse CLEANLY, not panic"),
    };
    assert!(
        matches!(err, CliError::Usage(ref m)
            if m.contains(&T14_YEAR.to_string()) && m.contains("7 promoted disposal leg")),
        "the refusal names the year and the leg count: {err}"
    );
    assert!(
        err.to_string().contains("void one of the promotes"),
        "the refusal names a concrete remedy: {err}"
    );
    assert!(
        std::fs::read_dir(&out)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "an overflow refusal leaves out_dir untouched (refuse-before-bytes) — no half-written packet"
    );
}

/// ADD-2 twin of the above for the CRYPTO-SLICE pre-check (admin.rs, the `printed_8275` block) — the
/// sibling above plants a full-return RI and so exercises only the full-return pre-check. This one
/// omits `plant_full_return_ri`, so `export_irs_pdf` takes the crypto-slice path and the >6-row refusal
/// must come from the crypto-slice block. Deleting only that block (leaving the full-return copy) would
/// survive the sibling KAT but must RED here.
#[test]
fn promoted_crypto_slice_export_with_more_than_6_legs_refuses_cleanly_not_panics() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_n_promoted_disposal_legs(dir.path(), T14_YEAR, 7);
    // NB: no `plant_full_return_ri` — this drives the crypto-slice export path.
    let out = dir.path().join("export_out"); // deliberately NOT pre-created

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cmd::admin::export_irs_pdf(&vault, &pp(), &out, T14_YEAR, &[], None)
    }));
    let err = match result {
        Ok(Err(e)) => e,
        Ok(Ok(_)) => {
            panic!("a 7-leg promoted crypto-slice export (> the 6-row capacity) must refuse, not succeed")
        }
        Err(_) => panic!("a 7-leg promoted crypto-slice export must refuse CLEANLY, not panic"),
    };
    assert!(
        matches!(err, CliError::Usage(ref m)
            if m.contains(&T14_YEAR.to_string()) && m.contains("7 promoted disposal leg")),
        "the crypto-slice refusal names the year and the leg count: {err}"
    );
    assert!(
        err.to_string().contains("void one of the promotes"),
        "the crypto-slice refusal names a concrete remedy: {err}"
    );
    assert!(
        std::fs::read_dir(&out)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "a crypto-slice overflow refusal leaves out_dir untouched (refuse-before-bytes)"
    );
}

/// A vault with TWO promoted disposal legs filed in DIFFERENT years — each its own declared+promoted
/// tranche, in its own wallet, drained by its own sell.
fn vault_with_two_promoted_years(dir: &Path, year_a: i32, year_b: i32) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    for (i, year) in [year_a, year_b].into_iter().enumerate() {
        let w = WalletId::Exchange {
            provider: "coinbase".into(),
            account: format!("m2-{i}"),
        };
        let window_start = time::Date::from_calendar_date(year, time::Month::January, 1).unwrap();
        let window_end = time::Date::from_calendar_date(year, time::Month::March, 31).unwrap();
        let tranche_id = cmd::tranche::declare_tranche(
            &vault,
            &pp(),
            40_000_000,
            w.clone(),
            window_start,
            window_end,
            now(),
        )
        .unwrap();
        cmd::promote::promote_tranche(
            &vault,
            &pp(),
            &tranche_id.canonical(),
            ProvenanceKind::Purchase,
            format!("cash P2P purchase, no records; {year} window bounded on-chain"),
            Some(PROMOTE_ACK_PHRASE),
            now(),
        )
        .unwrap();
        let sell_ts = time::Date::from_calendar_date(year, time::Month::September, 1)
            .unwrap()
            .with_hms(0, 0, 0)
            .unwrap()
            .assume_utc();
        let mut s = Session::open(&vault, &pp()).unwrap();
        append_import_batch(
            s.conn(),
            &[LedgerEvent {
                id: EventId::import(Source::Coinbase, SourceRef::new(format!("M2-SELL-{i}"))),
                utc_timestamp: sell_ts,
                original_tz: UtcOffset::UTC,
                wallet: Some(w),
                payload: EventPayload::Dispose(Dispose {
                    sat: 40_000_000,
                    usd_proceeds: dec!(20_000),
                    fee_usd: dec!(0),
                    kind: DisposeKind::Sell,
                }),
            }],
        )
        .unwrap();
        s.save().unwrap();
    }
    vault
}

/// ★ T16 KAT 4 (M2): the all-years `export_snapshot` dump (`tax_year: None`) with TWO different
/// promoted years in range must co-emit BOTH `form_8275_{year}.txt` files — never a bare,
/// collision-prone `form_8275.txt` that a second promoted year would silently overwrite.
#[test]
fn all_years_snapshot_writes_one_8275_txt_per_promoted_year() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_two_promoted_years(dir.path(), 2024, 2017);
    let out = dir.path().join("export_out");

    cmd::admin::export_snapshot(&vault, &pp(), &out, None, None)
        .expect("an all-years snapshot with two complete promoted years must succeed");

    assert!(
        out.join("form_8275_2024.txt").exists(),
        "the 2024 promoted year's disclosure is co-emitted"
    );
    assert!(
        out.join("form_8275_2017.txt").exists(),
        "the 2017 promoted year's disclosure is co-emitted"
    );
    assert!(
        !out.join("form_8275.txt").exists(),
        "the all-years dump never uses the bare, collision-prone name"
    );
}
