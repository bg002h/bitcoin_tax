//! Approach-B experimental disclosure (`design/approach-b-experimental-notice`) — CLI wiring KATs.
//!
//! The notice is INTERFACE-ONLY: it gates on `btctax_core::experimental::uses_approach_b`, and appears
//! on stderr for `declare-tranche` / `promote-tranche` / `export-irs-pdf` (crypto slice + full-return) —
//! never stdout (stdout is parsed/piped). It is deliberately **never** written to disk: no sibling file,
//! no field, nothing — the export directory is what a filer mails or hands to a preparer, and a copy of
//! this notice riding along with it would be the same hazard as printing it on the 8275 itself.
//!
//! ★ THE GUARD (the most important test in this file — the failure mode that matters is not a missing
//! banner, it is the banner leaking into a filing package): for a vault where Approach-B is loudly in
//! use AND a promoted disposal leg makes `form_8275.txt` / `basis_methodology.txt` / the 8275 PDF all
//! non-empty at once, the notice text appears NOWHERE the export directory produces — walked file by
//! file, plus a decoded AcroForm field-value check on the 8275 PDF specifically.
//!
//! PRIVACY: synthetic values in tempdirs; no user file is read.

use btctax_cli::{cmd, Session};
use btctax_core::conservative::Coverage;
use btctax_core::event::{
    Acknowledgment, Acquire, BasisSource, DeclareTranche, Dispose, DisposeKind, EventPayload,
    FloorMethod, PromoteTranche,
};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::{append_decision, append_import_batch};
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

/// A vault with a documented 0.6 BTC lot, a 0.4 BTC tranche PROMOTED to a $12,000 floor, and a 2024 sell
/// (2024 is a `btctax_forms::SUPPORTED_YEARS` PDF revision) of exactly the promoted 0.4 BTC — so the
/// 2024 export carries a PROMOTED disposal leg (a non-empty
/// `form_8275.txt` / `basis_methodology.txt` / 8275 PDF), exactly the co-occurrence the guard test needs.
/// Mirrors `promote_cli.rs::build_promoted_vault` (same figures). Returns the vault path.
fn build_promoted_vault(dir: &Path) -> PathBuf {
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
        datetime!(2024-09-01 00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 40_000_000,
            usd_proceeds: dec!(20_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    append_import_batch(s.conn(), &[buy, sell]).unwrap();

    let tranche_id = append_decision(
        s.conn(),
        EventPayload::DeclareTranche(DeclareTranche {
            sat: 40_000_000,
            wallet: wallet(),
            window_start: date!(2018 - 01 - 01),
            window_end: date!(2018 - 03 - 31),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    append_decision(
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
    vault
}

/// A plain vault (one real buy/sell, no Approach-B activity at all).
fn build_plain_vault(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    let buy = imp(
        "BUY",
        datetime!(2025-01-05 12:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 1_000_000,
            usd_cost: dec!(200),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = imp(
        "SELL",
        datetime!(2025-06-15 12:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 1_000_000,
            usd_proceeds: dec!(500),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    append_import_batch(s.conn(), &[buy, sell]).unwrap();
    s.save().unwrap();
    vault
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// § 1 — CLI stderr (never stdout), real binary — eprintln! cannot be intercepted in-process
// (mirrors declare_tranche_cli.rs's `run_declare` / promote_cli.rs's `run_promote` convention).
// ════════════════════════════════════════════════════════════════════════════════════════════════

const NOTICE_MARK: &str = "EXPERIMENTAL — DEFENSIVE FILING";
const NOTICE_FACT: &str = "heavy AI assistance";

/// Run `btctax --vault <vault> <args...>`; returns (exit, stdout, stderr).
fn run_btctax(vault: &Path, args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let mut c = std::process::Command::new(bin);
    c.arg("--vault").arg(vault.to_str().unwrap());
    for a in args {
        c.arg(a);
    }
    c.env("BTCTAX_PASSPHRASE", "pw");
    let out = c.output().expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A successful `declare-tranche` emits the notice on stderr, never stdout.
#[test]
fn declare_tranche_notice_reaches_stderr_not_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let (code, stdout, stderr) = run_btctax(
        &vault,
        &[
            "reconcile",
            "declare-tranche",
            "--amount",
            "0.5",
            "--wallet",
            "self:cold",
            "--window-start",
            "2020-01-01",
            "--window-end",
            "2020-12-31",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains(NOTICE_MARK),
        "the notice must reach stderr on a successful declare: {stderr:?}"
    );
    assert!(
        stderr.contains(NOTICE_FACT),
        "the AI-assistance fact must be present: {stderr:?}"
    );
    assert!(
        !stdout.contains(NOTICE_MARK),
        "the notice must NEVER reach stdout (stdout is parsed/piped): {stdout:?}"
    );
}

/// A REFUSED declare (non-positive amount) never emits the notice — the eprintln sits after the write
/// succeeds, mirroring the phantom-wallet warning's own "silent on refusal" contract.
#[test]
fn declare_tranche_notice_is_silent_on_a_refused_declare() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let (code, _stdout, stderr) = run_btctax(
        &vault,
        &[
            "reconcile",
            "declare-tranche",
            "--amount",
            "0",
            "--wallet",
            "self:cold",
            "--window-start",
            "2020-01-01",
            "--window-end",
            "2020-12-31",
        ],
    );
    assert_ne!(code, 0, "a non-positive amount must be refused");
    assert!(
        !stderr.contains(NOTICE_MARK),
        "a refused declare must never emit the notice: {stderr:?}"
    );
}

/// A tranche VOIDED before export (and never promoted) leaves Approach-B unused — no stderr notice on
/// `export-irs-pdf`. The load-bearing "don't show it to a filer who voided everything" case, at the CLI
/// layer.
#[test]
fn voided_only_tranche_never_triggers_the_notice_on_export_irs_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let tranche_id = {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let id = append_decision(
            s.conn(),
            EventPayload::DeclareTranche(DeclareTranche {
                sat: 10_000_000,
                wallet: wallet(),
                window_start: date!(2020 - 01 - 01),
                window_end: date!(2020 - 12 - 31),
            }),
            now(),
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        s.save().unwrap();
        id
    };
    cmd::reconcile::void(&vault, &pp(), &tranche_id.canonical(), now()).unwrap();

    let out = dir.path().join("out");
    let (code, _stdout, stderr) = run_btctax(
        &vault,
        &[
            "export-irs-pdf",
            "--out",
            out.to_str().unwrap(),
            "--tax-year",
            "2024",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        !stderr.contains(NOTICE_MARK),
        "a voided-only tranche must not trigger the notice: {stderr:?}"
    );
}

/// A successful `promote-tranche` emits the notice on stderr, never stdout.
#[test]
fn promote_tranche_notice_reaches_stderr_not_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_promoted_vault(dir.path()); // already-promoted; declare a SECOND tranche to promote

    // A second, fresh tranche (2019 window) to actually drive `reconcile promote-tranche` end-to-end
    // through the real binary (the fixture's own promote was hand-appended, not CLI-driven).
    let target = {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let id = append_decision(
            s.conn(),
            EventPayload::DeclareTranche(DeclareTranche {
                sat: 5_000_000,
                wallet: wallet(),
                window_start: date!(2019 - 01 - 01),
                window_end: date!(2019 - 06 - 30),
            }),
            now(),
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        s.save().unwrap();
        id
    };

    let part_ii = dir.path().join("part_ii.txt");
    std::fs::write(
        &part_ii,
        "cash P2P purchase, no records; window bounded on-chain",
    )
    .unwrap();

    let (code, stdout, stderr) = run_btctax(
        &vault,
        &[
            "reconcile",
            "promote-tranche",
            &target.canonical(),
            "--provenance",
            "purchase",
            "--part-ii-file",
            part_ii.to_str().unwrap(),
            "--i-acknowledge",
            btctax_cli::PROMOTE_ACK_PHRASE,
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains(NOTICE_MARK),
        "the notice must reach stderr on a successful promote: {stderr:?}"
    );
    assert!(
        !stdout.contains(NOTICE_MARK),
        "the notice must NEVER reach stdout: {stdout:?}"
    );
}

/// `export-irs-pdf` (crypto slice) on a vault with a live tranche/promote emits the notice on stderr,
/// never stdout; a plain vault (no Approach-B activity) emits nothing.
#[test]
fn export_irs_pdf_notice_reaches_stderr_not_stdout_and_is_absent_without_approach_b() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_promoted_vault(dir.path());
    let out = dir.path().join("out");

    let (code, stdout, stderr) = run_btctax(
        &vault,
        &[
            "export-irs-pdf",
            "--out",
            out.to_str().unwrap(),
            "--tax-year",
            "2024",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains(NOTICE_MARK),
        "the notice must reach stderr for an Approach-B vault: {stderr:?}"
    );
    assert!(
        !stdout.contains(NOTICE_MARK),
        "the notice must never reach stdout: {stdout:?}"
    );

    // A plain vault (no tranche/promote at all) never emits the notice.
    let dir2 = tempfile::tempdir().unwrap();
    let vault2 = build_plain_vault(dir2.path());
    let out2 = dir2.path().join("out");
    let (code2, _stdout2, stderr2) = run_btctax(
        &vault2,
        &[
            "export-irs-pdf",
            "--out",
            out2.to_str().unwrap(),
            "--tax-year",
            "2025",
        ],
    );
    assert_eq!(code2, 0, "stderr: {stderr2}");
    assert!(
        !stderr2.contains(NOTICE_MARK),
        "a non-Approach-B vault must never emit the notice: {stderr2:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// § 2 — ★ THE GUARD: the notice text reaches NOTHING the export directory produces, even on a vault
// where Approach-B is LOUDLY in use (stderr fires) AND a promoted disposal leg makes
// `form_8275.txt` / `basis_methodology.txt` / the 8275 PDF all non-empty at once — the exact
// co-occurrence the interface-only constraint exists for.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The full needle set: the notice's title plus each fact-bearing fragment. If ANY of these strings
/// appear anywhere in the export directory, the notice has leaked into the filing package.
fn notice_needles() -> Vec<&'static str> {
    vec![
        NOTICE_MARK,
        NOTICE_FACT,
        "137 characters",
        "no in-editor action will save until you quit",
        "check every figure",
    ]
}

/// Walk `dir` (non-recursive — every export this crate writes is a flat directory) and assert that NO
/// file's raw byte content contains any needle, and that no file is literally named `EXPERIMENTAL.txt`.
fn assert_directory_carries_no_notice(dir: &Path) {
    let entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
        .collect();
    assert!(
        !entries.is_empty(),
        "precondition: the export actually wrote files into {dir:?}"
    );
    for entry in entries {
        let path = entry.unwrap().path();
        assert_ne!(
            path.file_name().and_then(|n| n.to_str()),
            Some("EXPERIMENTAL.txt"),
            "no EXPERIMENTAL.txt sibling file — the notice is interface-only, never exported"
        );
        let bytes = std::fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for needle in notice_needles() {
            assert!(
                !text.contains(needle),
                "{path:?} must never carry the experimental notice ({needle:?})"
            );
        }
    }
}

#[test]
fn notice_text_is_absent_from_every_file_in_the_export_directory() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_promoted_vault(dir.path());
    let out = dir.path().join("out");
    let report = cmd::admin::export_irs_pdf(&vault, &pp(), &out, 2024, &[], None).unwrap();
    assert!(
        report.experimental_notice,
        "precondition: this vault IS Approach-B (a live promoted tranche) — CLI stderr fires"
    );

    // Preconditions: the co-occurrence this guard exists for — all three filed artifacts non-empty.
    let form_8275_txt = std::fs::read_to_string(out.join("form_8275.txt"))
        .expect("a promoted disposal leg files a non-empty form_8275.txt");
    assert!(
        !form_8275_txt.trim().is_empty(),
        "precondition: form_8275.txt is non-empty"
    );
    let basis_methodology_txt = std::fs::read_to_string(out.join("basis_methodology.txt"))
        .expect("a filed tranche writes a non-empty basis_methodology.txt");
    assert!(
        !basis_methodology_txt.trim().is_empty(),
        "precondition: basis_methodology.txt is non-empty"
    );
    let f8275_pdf_bytes =
        std::fs::read(out.join("form_8275.pdf")).expect("the 8275 PDF is written");

    // The blanket walk: every file in the export directory, raw bytes.
    assert_directory_carries_no_notice(&out);

    // The PDF's own AcroForm FIELD VALUES too — not just a raw-byte scan (a PDF's byte stream can
    // encode text as UTF-16BE, so a naive `contains` on raw bytes could miss an encoded leak; field
    // values are what a preparer/adjuster actually reads).
    let doc = btctax_forms::testonly::load(&f8275_pdf_bytes).expect("the 8275 PDF parses");
    let fields =
        btctax_forms::testonly::collect_fields(&doc).expect("the 8275 PDF has an AcroForm");
    for field in &fields {
        if let Some(value) = btctax_forms::testonly::text_value(&doc, field.id) {
            for needle in notice_needles() {
                assert!(
                    !value.contains(needle),
                    "8275 PDF field {:?} must never carry the experimental notice ({needle:?}): {value:?}",
                    field.fqn
                );
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// § 3 — the FULL-RETURN export dispatch (`export_full_return`, a SEPARATE function from the
// crypto-slice path above): same stderr + directory-wide guard coverage, driven through the SAME
// public `export-irs-pdf` entry point once `return_inputs` exist for the year (the dispatch in
// `export_irs_pdf_from_session`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Store minimal, answered `ReturnInputs` for `year` on an already-open vault — routes
/// `export-irs-pdf` to the full-return packet instead of the crypto slice. Mirrors
/// `export_irs_pdf.rs::export_dispatches_a_full_return_year_to_the_full_packet`'s own fixture.
fn give_full_return_inputs(vault: &Path, year: i32) {
    use btctax_cli::return_inputs;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let mut s = Session::open(vault, &pp()).unwrap();
    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        header: btctax_core::tax::testonly::not_a_dependent(),
        ..Default::default()
    };
    ri.header.taxpayer = btctax_core::tax::return_inputs::Person {
        first_name: "Pat".into(),
        last_name: "Roe".into(),
        ssn: "222-33-4444".into(),
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    return_inputs::set(s.conn(), year, &ri).unwrap();
    s.save().unwrap();
}

/// The full-return dispatch emits the notice on stderr, never stdout.
#[test]
fn full_return_export_notice_reaches_stderr_not_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_promoted_vault(dir.path());
    give_full_return_inputs(&vault, 2024);

    let out = dir.path().join("out");
    let (code, stdout, stderr) = run_btctax(
        &vault,
        &[
            "export-irs-pdf",
            "--out",
            out.to_str().unwrap(),
            "--tax-year",
            "2024",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        out.join("00_f1040.pdf").exists(),
        "this must be the FULL-RETURN dispatch, not the crypto slice: {out:?}"
    );
    assert!(
        stderr.contains(NOTICE_MARK),
        "the notice must reach stderr on the full-return dispatch: {stderr:?}"
    );
    assert!(
        !stdout.contains(NOTICE_MARK),
        "the notice must never reach stdout: {stdout:?}"
    );
}

/// The guard, restated for the full-return dispatch: the notice appears in NOTHING the export
/// directory produces — every `full_return_paths` PDF (byte scan; the exact sequence prefix is a
/// map/year detail this test does not need to know), the manifest, `form_8275.txt`, and
/// `basis_methodology.txt` — via the same whole-directory walk.
#[test]
fn full_return_export_notice_absent_from_every_file_in_the_export_directory() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_promoted_vault(dir.path());
    give_full_return_inputs(&vault, 2024);

    let out = dir.path().join("out");
    let rep = cmd::admin::export_irs_pdf(&vault, &pp(), &out, 2024, &[], None).unwrap();
    assert!(
        !rep.full_return_paths.is_empty(),
        "precondition: this is the full-return dispatch"
    );
    assert!(
        rep.experimental_notice,
        "precondition: this vault IS Approach-B (a live promoted tranche) — CLI stderr fires"
    );

    assert_directory_carries_no_notice(&out);

    // The AcroForm field-value check too, for every PDF in the packet.
    for path in &rep.full_return_paths {
        let bytes = std::fs::read(path).unwrap();
        let Ok(doc) = btctax_forms::testonly::load(&bytes) else {
            continue;
        };
        let Ok(fields) = btctax_forms::testonly::collect_fields(&doc) else {
            continue;
        };
        for field in &fields {
            if let Some(value) = btctax_forms::testonly::text_value(&doc, field.id) {
                for needle in notice_needles() {
                    assert!(
                        !value.contains(needle),
                        "{path:?} field {:?} must never carry the experimental notice ({needle:?}): {value:?}",
                        field.fqn
                    );
                }
            }
        }
    }
}
