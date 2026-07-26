//! Task 1 (Defensive Filing Wizard, sub-project-2 P-A) — characterization + behavior tests for the
//! extracted PROMOTE chokepoint (`btctax_cli::chokepoint`): `plan_promote` / `render_consent` /
//! `apply_promote`. Step 1 pins the SHIPPED `cmd::promote::promote_tranche` full ordered stdout
//! transcript (advisory → consent → note) BEFORE the refactor, exercising all THREE ordered
//! `PromotePlan` pieces at once (I-1): a non-empty `Direction::Promote` advisory line, a gift-only
//! flagged prior year, AND a wide (> 1 year) declared window — from ONE crafted gift event, so the
//! fixture stays small. PRIVACY: synthetic values in a tempdir; no real user file is ever read.

use btctax_cli::cmd::promote::ProvenanceKind;
use btctax_cli::{chokepoint, cmd, eventref, CliError, Session, PROMOTE_ACK_PHRASE};
use btctax_core::event::{
    Acquire, BasisSource, ConsentTerm, DeclareTranche, Dispose, DisposeKind, EventPayload,
    FmvStatus, Income, IncomeKind, OutflowClass, ReclassifyOutflow, TransferOut,
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
/// The single wallet the documented buy, the tranche, and the gift-out all share (so the tranche can
/// HIFO-outrank the documented buy once promoted).
fn wallet() -> WalletId {
    WalletId::SelfCustody {
        label: "chokepoint-t1".into(),
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

/// A vault exercising ALL THREE ordered `PromotePlan` pieces at once (I-1): a documented low-basis lot
/// (2012, $10 total — $10/BTC) + an UNPROMOTED 0.4-BTC tranche over a WIDE (731-day, > 1 year) window
/// [2020-01-01, 2021-12-31] whose window-low close ($5,053.95/BTC, the 2020-03-16 COVID low) FAR exceeds
/// the documented lot's $10/BTC — so once promoted the tranche outranks the documented lot under HIFO —
/// and a PRIOR-year (2022, < the injected `now()`'s 2026 tax year) 0.3-BTC gift-out that draws entirely
/// from the documented lot BEFORE the promote but entirely from the (BG-D11 documented-only, $0-basis)
/// tranche AFTER: one gift event that is simultaneously (a) a non-empty `Direction::Promote` advisory
/// line (the §1015 fragment) and (b) a GIFT-ONLY flagged year (no donation event exists at all in this
/// fixture). Returns (vault, the tranche's canonical target ref).
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

/// Run `btctax --vault <vault> reconcile promote-tranche <target> --provenance purchase --part-ii-file
/// <path> [extra...]`; returns (exit, stdout, stderr). Mirrors `promote_cli.rs`'s `run_promote`.
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

/// ★ Step 1 (Task 1 brief) — the characterization KAT: pins the SHIPPED `cmd::promote::promote_tranche`
/// full ordered stdout transcript (advisory → consent → `wide_window_note`, `promote.rs:443-455`) AND the
/// recorded `Acknowledgment.shown_terms`, captured verbatim from a real run of the CURRENT (pre-refactor)
/// verb against `build_three_piece_vault` (which exercises all THREE ordered `PromotePlan` pieces at
/// once). This must stay GREEN across the chokepoint extraction (Steps 3-5): the refactor is
/// behavior-PRESERVING except for the ONE intended DFW-D6 pseudo-off change (Step 6), which this fixture
/// does not touch (pseudo-reconcile is never enabled here).
#[test]
fn pins_shipped_promote_transcript_and_shown_terms() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = build_three_piece_vault(dir.path());
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

    let expected = "Promoting this tranche changes the §1015 carryover basis passed to the donee for year 2022's gift(s) by ~$3 — donee-basis documentation only; the donor's own Form 1040 for 2022 is unaffected, so no amended return is required.\nPromoting this tranche is a KNOWING choice to file a >$0 basis floor (the minimum daily closing price over the attested acquisition window) instead of the IRS-fallback $0. If an exam determines the correct basis is $0, the penalty is 20% ordinary / 40% worst-case of the resulting additional tax (the underpayment attributable to the misstatement), plus interest; the Form 8275 disclosure and the good-faith window-low-close methodology mitigate this exposure, but do not eliminate it and do not guarantee immunity from penalty.\nYear 2022: tax not computable here (no table/profile/blocked) — promoting changes the reported gain by ~$0 and the deduction/basis by ~$3. The deduction/basis figure is a donee-basis (§1015) documentation change; the donor's 1040 is unaffected — NOT a Schedule-A deduction.\n10000000 sat remain undisposed: at the 2026-01-01 close, promoting would reduce a future sale's reported gain by up to ~$505.40 (hypothetical, not a filed figure) — saving and exposure accrue only at disposal.\nnote: this tranche's declared window spans 730 days (over a year). A WIDE window tends to produce a LOW (\"trivial\") floor relative to a tight one — for some filers it may be simpler, and just as conservative, to leave this tranche at its filed $0 basis and skip the Form 8275 disclosure surface entirely.\nRecorded decision decision|3\n";
    assert_eq!(
        stdout, expected,
        "the full ordered transcript (advisory \\n consent \\n note \\n \"Recorded decision\") must be \
         byte-identical to the shipped verb"
    );

    // The recorded Acknowledgment.shown_terms — the exact ConsentTerm values captured from the same run.
    let s = Session::open(&vault, &pp()).unwrap();
    let promote = load_all(s.conn())
        .unwrap()
        .into_iter()
        .find_map(|e| match e.payload {
            EventPayload::PromoteTranche(p) => Some(p),
            _ => None,
        })
        .expect("exactly one PromoteTranche recorded");
    assert_eq!(promote.filed_basis, dec!(2021.58));
    assert_eq!(
        promote.acknowledgment.shown_terms,
        vec![
            ConsentTerm::Uncomputable {
                year: 2022,
                gain_delta_usd: dec!(0),
                deduction_delta_usd: dec!(3),
            },
            ConsentTerm::Unrealized {
                sat: 10_000_000,
                hypothetical_reduction: Some(dec!(505.40)),
                as_of: Some(date!(2026 - 01 - 01)),
            },
        ]
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Step 6 — DFW-D6, the ONE intended behavior change: `plan_promote` forces `pseudo_reconcile = false`
// on its own config copy before `consent_terms`/`promote_prior_year_advisory`/`gift_only_flagged_years`.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A vault where an unresolved native `Income` (a mining reward, no FMV) sits in the SAME wallet as the
/// tranche + a low-basis documented lot. Its pseudo-synthesized FMV (`fmv_of` at the 2021-11-01 bundled
/// close, $61,293.24/BTC × 0.3 BTC ≈ $18,388) FAR EXCEEDS the tranche's promoted floor (~$5,053.95/BTC-
/// equivalent, the 2020-03-16 COVID low over [2020-01-01, 2021-12-31]) — so IF pseudo mode were left
/// ACTIVE it would rank ABOVE the tranche in BOTH the pre- and post-promote fold, dominating a prior-year
/// (2022) sell sized to the tranche's own 0.2-BTC sat: the promote's effect would be MASKED entirely (the
/// year-2022 term vanishes, replaced by an `Unrealized` line for the never-touched tranche — verified
/// empirically by temporarily disabling the fix). Forced pseudo OFF (DFW-D6, the shipped fix), the
/// unresolved Income stays Hard-blocked/excluded, so the documented lot ($10/BTC) serves the BASELINE
/// sell and the promoted tranche serves the WITH-promote sell — a real, year-2022 `Uncomputable` term.
/// Returns (vault, the tranche's canonical target ref).
fn build_pseudo_off_vault(dir: &Path) -> (PathBuf, String) {
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
    let income = imp(
        "INCOME",
        datetime!(2021-11-01 00:00 UTC),
        EventPayload::Income(Income {
            sat: 30_000_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Mining,
            business: false,
        }),
    );
    let sell = imp(
        "SELL",
        datetime!(2022-06-01 00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 20_000_000,
            usd_proceeds: dec!(10_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    append_import_batch(s.conn(), &[buy, income, sell]).unwrap();

    let tranche_id = append_decision(
        s.conn(),
        EventPayload::DeclareTranche(DeclareTranche {
            sat: 20_000_000,
            wallet: wallet(),
            window_start: date!(2020 - 01 - 01),
            window_end: date!(2021 - 12 - 31),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    s.save().unwrap();
    (vault, tranche_id.canonical())
}

/// ★ Step 6 (DFW-D6, the ONE intended behavior change): on a vault with pseudo-reconcile ACTIVE
/// (`reconcile pseudo on`), the RECORDED `Acknowledgment.shown_terms` are the pseudo-OFF (honest) figures
/// — never whatever a synthetic default would fold in. Mutation-verified (dev-time): commenting out
/// `plan_promote`'s `honest_cfg.pseudo_reconcile = false;` line flips the recorded `shown_terms` to
/// `[Unrealized { sat: 20_000_000, hypothetical_reduction: Some(1010.79), as_of: Some(2026-01-01) }]` —
/// the year-2022 term disappears entirely because the pseudo income lot then masks the promote's effect.
#[test]
fn pseudo_active_promote_records_honest_terms_not_synthetic() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = build_pseudo_off_vault(dir.path());
    // Turn pseudo-reconcile ON in the vault's STORED config — this is what DFW-D6 must override.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let part_ii = dir.path().join("part_ii.txt");
    std::fs::write(&part_ii, "cash P2P purchase, no records; pseudo-off KAT").unwrap();

    cmd::promote::promote_tranche(
        &vault,
        &pp(),
        &target,
        ProvenanceKind::Purchase,
        std::fs::read_to_string(&part_ii).unwrap(),
        Some(PROMOTE_ACK_PHRASE),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let promote = load_all(s.conn())
        .unwrap()
        .into_iter()
        .find_map(|e| match e.payload {
            EventPayload::PromoteTranche(p) => Some(p),
            _ => None,
        })
        .expect("exactly one PromoteTranche recorded");
    assert_eq!(promote.filed_basis, dec!(1010.79));
    assert_eq!(
        promote.acknowledgment.shown_terms,
        vec![ConsentTerm::Uncomputable {
            year: 2022,
            gain_delta_usd: dec!(1008.79),
            deduction_delta_usd: dec!(0),
        }],
        "pseudo-reconcile is ACTIVE on this vault, but the recorded shown_terms must be the HONEST \
         (pseudo-forced-off) figures — never a synthetic-influenced number (DFW-D6)"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Step 7 — the acknowledgment gate lives INSIDE `apply_promote`, fail-closed, and distinguishes a
// MISSING phrase from a WRONG one (mirrors `require_attestation`'s precedent).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ Step 7: `apply_promote` refuses BOTH a missing (`None`) and a WRONG acknowledgment phrase, with
/// DISTINCT messages, and appends NOTHING on either refusal (fail-closed) — only the correct phrase
/// appends the `PromoteTranche` decision. Mutation-verified (dev-time): dropping the
/// `require_promote_ack(acknowledge)?;` line from `apply_promote` makes the `None` call succeed instead
/// of refusing, redding this KAT's first `unwrap_err()`.
#[test]
fn apply_promote_ack_gate_is_fail_closed_and_distinguishes_missing_from_wrong() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = build_three_piece_vault(dir.path());
    let target_id = eventref::parse_event_id(&target).unwrap();

    let mut session = Session::open(&vault, &pp()).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();
    let plan = chokepoint::plan_promote(
        &events,
        session.prices(),
        &cfg,
        &target_id,
        ProvenanceKind::Purchase,
        "cash P2P purchase, no records; wide multi-year window",
        now(),
    )
    .unwrap();

    let before = load_all(session.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .count();

    // No acknowledgment at all.
    let err_none = chokepoint::apply_promote(&mut session, plan.clone(), None, now()).unwrap_err();
    // A WRONG phrase — refused too, but for a DISTINCT reason.
    let err_wrong =
        chokepoint::apply_promote(&mut session, plan.clone(), Some("wrong phrase"), now())
            .unwrap_err();

    let msg = |e: &CliError| match e {
        CliError::Usage(m) => m.clone(),
        other => panic!("expected CliError::Usage: {other}"),
    };
    let (msg_none, msg_wrong) = (msg(&err_none), msg(&err_wrong));
    assert_ne!(
        msg_none, msg_wrong,
        "a missing ack and a WRONG ack must be DISTINCT refusals"
    );
    assert!(
        msg_none.contains("requires acknowledging"),
        "missing-ack message: {msg_none}"
    );
    assert!(
        msg_wrong.contains("did not match"),
        "wrong-ack message: {msg_wrong}"
    );

    // Fail-closed: neither refused attempt appended anything.
    let after_refusals = load_all(session.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .count();
    assert_eq!(
        after_refusals, before,
        "both refused acknowledgments appended NOTHING"
    );

    // The correct phrase succeeds and appends exactly one PromoteTranche decision.
    let id =
        chokepoint::apply_promote(&mut session, plan, Some(PROMOTE_ACK_PHRASE), now()).unwrap();
    assert!(matches!(id, EventId::Decision { .. }));
    let after_success = load_all(session.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .count();
    assert_eq!(after_success, before + 1);
}
