//! Task 4 (Defensive Filing Wizard, sub-project-2 P-A gate) — the full-driver CONSENT-PARITY harness:
//! proves the chokepoint path a future TUI will drive (`plan_promote` → `render_consent` →
//! `apply_promote`) produces a rendered consent screen that is BYTE-IDENTICAL, and a recorded
//! `Acknowledgment` that is `Eq`-identical, to the shipped CLI verb (`cmd::promote::promote_tranche`,
//! driven end-to-end via the real `btctax` binary). This is the §6664(c) guarantee that the good-faith
//! artifact a filer acknowledges is the SAME regardless of which surface drove the record.
//!
//! Every KAT below builds TWO fresh, IDENTICALLY-constructed vaults — one driven by the shipped CLI verb
//! (a subprocess, so main.rs's own wiring is exercised, not just the library fn), the other driven
//! directly through `crate::chokepoint` the way a future TUI will — and compares their outputs to EACH
//! OTHER (never against a hardcoded golden string; `tests/chokepoint_promote.rs`'s Step-1 characterization
//! KAT already pins the shipped transcript verbatim). The fixture (`build_three_piece_vault`, duplicated
//! from `chokepoint_promote.rs`'s Task-1 KAT) exercises all THREE ordered `PromotePlan` pieces at once (a
//! non-empty synthetic-promote advisory, a gift-only prior year, and a wide (>1yr) window that fires
//! `wide_window_note`) — so byte-for-byte ORDERING (I-1: advisory → consent → note) is under test, not
//! just presence. PRIVACY: synthetic values in a tempdir; no real user file is ever read.

use btctax_cli::cmd::promote::ProvenanceKind;
use btctax_cli::{chokepoint, cmd, eventref, CliError, Session, PROMOTE_ACK_PHRASE};
use btctax_core::event::{
    Acquire, BasisSource, DeclareTranche, EventPayload, OutflowClass, PromoteTranche,
    ReclassifyOutflow, TransferOut,
};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::{append_decision, append_import_batch, load_all};
use btctax_core::LedgerEvent;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime};
use time::UtcOffset;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026 - 01 - 01 0:00 UTC)
}
/// The single wallet all fixtures in this file use.
fn wallet() -> WalletId {
    WalletId::SelfCustody {
        label: "chokepoint-parity".into(),
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

/// Adapted from `chokepoint_promote.rs::build_three_piece_vault` (Task 1) — kept as a LOCAL copy (not
/// shared across test files; matches this suite's own convention of each file owning its vault-builders)
/// so this parity harness has no cross-file coupling. A vault exercising ALL THREE ordered `PromotePlan`
/// pieces at once (I-1): a documented low-basis lot (2012, $10 total — $10/BTC) + an UNPROMOTED 0.4-BTC
/// tranche over a WIDE (731-day, > 1 year) window [2020-01-01, 2021-12-31] whose window-low close
/// ($5,053.95/BTC, the 2020-03-16 COVID low) far exceeds the documented lot's $10/BTC, and a PRIOR-year
/// (2022, < the injected `now()`'s 2026 tax year) 0.3-BTC gift-out that draws entirely from the
/// documented lot BEFORE the promote but entirely from the tranche AFTER — one gift event that is
/// simultaneously (a) a non-empty `Direction::Promote` advisory line (the §1015 fragment) and (b) a
/// GIFT-ONLY flagged year (no donation event exists in this fixture). Returns (vault, the tranche's
/// canonical target ref).
fn build_three_piece_vault(dir: &Path) -> (PathBuf, String) {
    let vault = dir.join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();
    let buy = imp(
        "BUY",
        datetime!(2012-01-01 00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(10),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let gift_out = imp(
        "GIFTOUT",
        datetime!(2022-06-01 00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 30_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    append_import_batch(s.conn(), &[buy, gift_out]).unwrap();

    let tranche_id = append_decision(
        s.conn(),
        EventPayload::DeclareTranche(DeclareTranche {
            sat: 40_000_000,
            wallet: wallet(),
            window_start: date!(2020 - 01 - 01),
            window_end: date!(2021 - 12 - 31),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    append_decision(
        s.conn(),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("GIFTOUT")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(5_000),
            fee_usd: None,
            donee: None,
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    s.save().unwrap();
    (vault, tranche_id.canonical())
}

/// A vault with ONE `$0`-basis tranche over `[window_start, window_end]` — a minimal fixture for the
/// pre-consent refusal-parity KATs (BG-D5/BG-D3/BG-D7), which all fire INSIDE `plan_promote` before any
/// consent computation ever runs, so there is nothing to exercise here beyond `resolve_live_tranche`
/// succeeding. Returns (vault, the tranche's canonical target ref).
fn simple_tranche_vault(
    dir: &Path,
    window_start: time::Date,
    window_end: time::Date,
) -> (PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        40_000_000,
        wallet(),
        window_start,
        window_end,
        now(),
    )
    .unwrap();
    (vault, id.canonical())
}

/// Run `btctax --vault <vault> reconcile promote-tranche <target> --provenance <provenance>
/// --part-ii-file <path> [extra...]`; returns (exit, stdout, stderr). `extra` carries `--i-acknowledge
/// <phrase>` when the scenario wants it, and is empty otherwise — under `Command::output()`, stdin is
/// NEVER a terminal, so an omitted `--i-acknowledge` always takes main.rs's single-call `None` branch
/// (the N-2 non-interactive path), never the interactive prompt. Mirrors `promote_cli.rs`/
/// `chokepoint_promote.rs`'s own `run_promote`.
fn run_promote(
    vault: &Path,
    target: &str,
    provenance: &str,
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
        .arg(provenance)
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

/// Drive the chokepoint's PLANNING half exactly the way a future TUI will: `plan_promote` →
/// `render_consent`. Returns (the rendered consent string, the plan) — `apply_promote` is called
/// separately by each test since the acknowledgment varies per scenario.
fn plan_and_render(
    session: &Session,
    target: &str,
    provenance: ProvenanceKind,
    part_ii: &str,
) -> (String, chokepoint::PromotePlan) {
    let target_id = eventref::parse_event_id(target).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();
    let plan = chokepoint::plan_promote(
        &events,
        session.prices(),
        &cfg,
        &target_id,
        provenance,
        part_ii,
        now(),
    )
    .expect("this fixture's plan_promote must succeed");
    let rendered = chokepoint::render_consent(&plan);
    (rendered, plan)
}

fn count_promotes(vault: &Path) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    load_all(s.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::PromoteTranche(_)))
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

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Happy path — a fully-consented promote.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ Task 4 Step 1 (happy path): the CLI-driven transcript's consent portion is byte-identical to
/// `chokepoint::render_consent`'s own output, and the recorded `Acknowledgment` (the §6664(c) good-faith
/// artifact) is `Eq`-identical, across the two drivers.
#[test]
fn happy_path_consent_and_acknowledgment_are_driver_parity() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let (vault_a, target_a) = build_three_piece_vault(dir_a.path());
    let (vault_b, target_b) = build_three_piece_vault(dir_b.path());
    assert_eq!(
        target_a, target_b,
        "identical construction on two fresh vaults must yield identical decision refs"
    );

    let part_ii_text = "cash P2P purchase, no records; wide multi-year window";
    let part_ii_path = dir_a.path().join("part_ii.txt");
    std::fs::write(&part_ii_path, part_ii_text).unwrap();

    // (a) the shipped CLI verb, driven end-to-end via the real binary.
    let (code_a, stdout_a, stderr_a) = run_promote(
        &vault_a,
        &target_a,
        "purchase",
        &part_ii_path,
        &["--i-acknowledge", PROMOTE_ACK_PHRASE],
    );
    assert_eq!(code_a, 0, "stderr: {stderr_a}");

    // (b) the chokepoint, driven the way a future TUI will.
    let mut session_b = Session::open(&vault_b, &pp()).unwrap();
    let (rendered_b, plan_b) = plan_and_render(
        &session_b,
        &target_b,
        ProvenanceKind::Purchase,
        part_ii_text,
    );
    let id_b =
        chokepoint::apply_promote(&mut session_b, plan_b, Some(PROMOTE_ACK_PHRASE), now()).unwrap();
    drop(session_b); // release the vault lock before re-opening vault_b below

    // ★ Byte-identical consent copy (advisory → consent → note, I-1's order): stdout_a's consent portion
    // must equal `rendered_b` verbatim, modulo the ONE trailing `\n` `println!` always adds; only the CLI
    // DRIVER's own trailing "Recorded decision ..." line (main.rs, NOT `render_consent`) may follow.
    let expected_prefix = format!("{rendered_b}\n");
    assert!(
        stdout_a.starts_with(&expected_prefix),
        "the CLI-driven transcript must start with the chokepoint-rendered consent verbatim:\n\
         --- stdout_a ---\n{stdout_a}\n--- rendered_b ---\n{rendered_b}"
    );
    let tail = &stdout_a[expected_prefix.len()..];
    assert_eq!(
        tail,
        format!("Recorded decision {}\n", id_b.canonical()),
        "only the driver's own trailing line may follow — and it names the SAME decision ref on both \
         sides (identical construction ⇒ identical decision sequence numbers)"
    );

    // ★ Eq-identical recorded Acknowledgment.
    let promote_a = only_promote(&vault_a);
    let promote_b = only_promote(&vault_b);
    assert_eq!(
        promote_a.acknowledgment, promote_b.acknowledgment,
        "the recorded Acknowledgment (incl. shown_terms) must be Eq-identical across drivers"
    );
    assert!(
        !promote_a.acknowledgment.shown_terms.is_empty(),
        "sanity: the three-piece fixture's consent has real ConsentTerm rows, not a vacuous empty Vec"
    );
    assert_eq!(promote_a.filed_basis, promote_b.filed_basis);
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Refused-ack — the consent still surfaces (BG-D6 fail-closed, inside `apply_promote`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ Task 4 Step 1 (refused-ack): BOTH a MISSING and a WRONG acknowledgment phrase still print the FULL
/// consent transcript (byte-identical across drivers) before refusing, and append NOTHING on either side.
#[test]
fn refused_ack_still_surfaces_identical_consent_across_drivers() {
    for ack_case in [None, Some("the wrong phrase")] {
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let (vault_a, target_a) = build_three_piece_vault(dir_a.path());
        let (vault_b, target_b) = build_three_piece_vault(dir_b.path());

        let part_ii_text = "cash P2P purchase, no records; wide multi-year window";
        let part_ii_path = dir_a.path().join("part_ii.txt");
        std::fs::write(&part_ii_path, part_ii_text).unwrap();

        let mut extra: Vec<&str> = Vec::new();
        if let Some(phrase) = ack_case {
            extra.push("--i-acknowledge");
            extra.push(phrase);
        }
        let (code_a, stdout_a, stderr_a) =
            run_promote(&vault_a, &target_a, "purchase", &part_ii_path, &extra);
        assert_ne!(
            code_a, 0,
            "case {ack_case:?}: a refused ack must exit non-zero; stderr: {stderr_a}"
        );

        let mut session_b = Session::open(&vault_b, &pp()).unwrap();
        let (rendered_b, plan_b) = plan_and_render(
            &session_b,
            &target_b,
            ProvenanceKind::Purchase,
            part_ii_text,
        );
        let err_b: CliError =
            chokepoint::apply_promote(&mut session_b, plan_b, ack_case, now()).unwrap_err();
        drop(session_b); // release the vault lock before re-opening vault_b below

        // ★ Consent still surfaces, byte-identical, with NOTHING else following it — the refusal fires
        // INSIDE `apply_promote`, AFTER the driver's unconditional `render_consent` println but BEFORE
        // main.rs's trailing "Recorded decision" line (an early `?`-return never reaches it).
        assert_eq!(
            stdout_a,
            format!("{rendered_b}\n"),
            "case {ack_case:?}: a refused ack still prints the FULL consent transcript, byte-identical, \
             with nothing appended after it"
        );
        assert_eq!(
            stderr_a,
            format!("error: {err_b}\n"),
            "case {ack_case:?}: the refusal text itself must also be byte-identical across drivers"
        );

        assert_eq!(
            count_promotes(&vault_a),
            0,
            "case {ack_case:?}: nothing recorded via the CLI"
        );
        assert_eq!(
            count_promotes(&vault_b),
            0,
            "case {ack_case:?}: nothing recorded via the chokepoint"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Each refusal (BG-D5 / BG-D3 / BG-D7) — all fire INSIDE `plan_promote`, before any consent renders.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Build TWO fresh, identically-constructed `simple_tranche_vault`s and assert BOTH drivers refuse
/// IDENTICALLY for a gate that fires INSIDE `plan_promote`, before any consent is ever computed: (i) the
/// CLI verb refuses (non-zero exit) printing NOTHING to stdout (the render never runs); (ii)
/// `plan_promote` itself refuses with a `Refusal` whose `CliError` mapping is byte-identical to the CLI's
/// stderr line; (iii) nothing is recorded on either side (fail-closed).
fn assert_prerender_refusal_parity(
    window_start: time::Date,
    window_end: time::Date,
    provenance_flag: &str,
    provenance_kind: ProvenanceKind,
    part_ii_text: &str,
) {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let (vault_a, target_a) = simple_tranche_vault(dir_a.path(), window_start, window_end);
    let (vault_b, target_b) = simple_tranche_vault(dir_b.path(), window_start, window_end);

    let part_ii_path = dir_a.path().join("part_ii.txt");
    std::fs::write(&part_ii_path, part_ii_text).unwrap();

    let (code_a, stdout_a, stderr_a) = run_promote(
        &vault_a,
        &target_a,
        provenance_flag,
        &part_ii_path,
        &["--i-acknowledge", PROMOTE_ACK_PHRASE],
    );
    assert_ne!(
        code_a, 0,
        "this fixture must be refused; stdout={stdout_a} stderr={stderr_a}"
    );
    assert!(
        stdout_a.is_empty(),
        "a pre-consent refusal must print NOTHING to stdout (render_consent never runs): {stdout_a:?}"
    );

    let target_id_b = eventref::parse_event_id(&target_b).unwrap();
    let session_b = Session::open(&vault_b, &pp()).unwrap();
    let events_b = load_all(session_b.conn()).unwrap();
    let cfg_b = session_b.config().unwrap().to_projection();
    let refusal_b = chokepoint::plan_promote(
        &events_b,
        session_b.prices(),
        &cfg_b,
        &target_id_b,
        provenance_kind,
        part_ii_text,
        now(),
    )
    .expect_err("this fixture must refuse plan_promote directly too");
    let err_b: CliError = refusal_b.into();
    drop(session_b); // release the vault lock before re-opening vault_b below

    assert_eq!(
        stderr_a,
        format!("error: {err_b}\n"),
        "the CLI-driven refusal text must be byte-identical to plan_promote's Refusal, mapped through \
         the SAME From<Refusal> for CliError"
    );

    assert_eq!(count_promotes(&vault_a), 0);
    assert_eq!(count_promotes(&vault_b), 0);
}

/// ★ BG-D5: a non-`Purchase` provenance is refused identically across drivers.
#[test]
fn bg_d5_bad_provenance_refusal_is_identical_across_drivers() {
    assert_prerender_refusal_parity(
        date!(2020 - 01 - 01),
        date!(2020 - 01 - 10),
        "gift",
        ProvenanceKind::Gift,
        "cash P2P purchase, no records",
    );
}

/// ★ BG-D3: a window straddling the bundled dataset's first covered day (2010-07-17) has a genuine gap
/// (2010-07-10..16 uncovered, 2010-07-17..20 covered) — `Coverage::Partial`, hard-refused — refused
/// identically across drivers.
#[test]
fn bg_d3_partial_coverage_refusal_is_identical_across_drivers() {
    assert_prerender_refusal_parity(
        date!(2010 - 07 - 10),
        date!(2010 - 07 - 20),
        "purchase",
        ProvenanceKind::Purchase,
        "cash P2P purchase, no records; window straddles the bundled dataset's first covered day",
    );
}

/// ★ BG-D7: a whitespace-only Form 8275 Part II narrative is refused identically across drivers.
#[test]
fn bg_d7_empty_part_ii_refusal_is_identical_across_drivers() {
    assert_prerender_refusal_parity(
        date!(2020 - 01 - 01),
        date!(2020 - 01 - 10),
        "purchase",
        ProvenanceKind::Purchase,
        "   ",
    );
}
