//! Export module — the ONLY module in `btctax-tui` permitted to perform write-class I/O.
//!
//! # Guarantee
//! "never writes the vault or any decrypted image of it; writes only the four form CSVs
//! via `export.rs` on explicit user confirmation."
//!
//! This module writes ONLY: the timestamped export directory (via
//! `fsperms::mkdir_owner_only_exclusive`) and the four named form CSVs (via
//! `btctax_cli::render::write_form_csvs`).  No other write-class I/O occurs anywhere in
//! `btctax-tui` source — the mechanized gate (KAT-E10) enforces this on every `cargo test`.

use crate::app::Snapshot;
use btctax_core::{compute_se_tax, TaxTables};
use btctax_store::fsperms;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

// ── Export directory computation ──────────────────────────────────────────────

/// Compute the export directory path from a vault path and a UTC timestamp.
///
/// Pure function — no filesystem access. Extracted for testability (KAT-E6).
///
/// Format: `{vault_parent}/btctax-export-{YYYYMMDD}-{HHMMSS}Z`
/// e.g. for 2025-10-24 14:30:22 UTC: `btctax-export-20251024-143022Z`.
///
/// Note on the bare-filename fallback: `Path::parent()` of a relative bare filename
/// returns `Some("")`; the `unwrap_or(".")` arm is nearly dead code and the result is a
/// cwd-relative export dir.  Behaviourally fine; stated so callers don't "fix" it.
pub fn export_dir_for(vault_path: &Path, export_now: OffsetDateTime) -> PathBuf {
    use time::macros::format_description;
    let parent = vault_path.parent().unwrap_or(Path::new("."));
    let ts = export_now
        .format(format_description!(
            "[year][month][day]-[hour][minute][second]Z"
        ))
        .expect("timestamp format is infallible");
    parent.join(format!("btctax-export-{ts}"))
}

// ── Confirmation state ────────────────────────────────────────────────────────

/// State frozen when the export confirmation modal opens.
///
/// The export directory and file list are computed at modal-open time (when `e` is pressed)
/// from the injected `export_now` timestamp.  `do_export` uses this state verbatim; it does
/// NOT re-compute the directory — the modal and the write operation are consistent.
///
/// [R0-N3] `ExportConfirmState` is freely constructible; the "modal gates the ONLY call
/// site of `do_export`" guarantee is procedural (KAT-E10 + whole-diff review), not
/// type-level.  Acceptable for this scope.
pub struct ExportConfirmState {
    pub year: i32,
    pub out_dir: PathBuf,
    /// Files that will be written (derived before the modal opens).
    pub files: Vec<&'static str>,
    /// Frozen at modal-open time.  Stored so the modal and the written dir are consistent;
    /// `do_export` uses the pre-computed `out_dir` rather than re-running `export_dir_for`.
    /// The binary-crate dead_code lint fires on this `pub` field because no code reads it
    /// back after construction; the suppression is intentional — the field is part of the
    /// public struct contract and may be read by future callers or tests.
    #[allow(dead_code)]
    pub export_now: OffsetDateTime,
}

// ── SE assembly helpers ───────────────────────────────────────────────────────

/// Compute the SE tax result for `year` from a snapshot.
///
/// PROFILE-GATED: mirrors `cmd/tax.rs:79–106` exactly — no profile → `None` (no SE CSV).
/// Used by the `e` keybinding to populate `ExportConfirmState::files`.
pub fn se_result_for(snap: &Snapshot, year: i32) -> Option<btctax_core::SeTaxResult> {
    let p = snap.profiles.get(&year)?;
    let table_opt = snap.tables.table_for(year);
    table_opt.and_then(|t| {
        compute_se_tax(
            &snap.state,
            year,
            p.filing_status,
            t,
            p.w2_ss_wages,
            p.w2_medicare_wages,
            p.schedule_c_expenses,
        )
    })
}

/// Compute the files list for the modal from the current snapshot and year.
///
/// Always includes `form8949.csv`, `schedule_d.csv`, `form8283.csv`.
/// Includes `schedule_se.csv` iff `se_result_for(snap, year)` is `Some`.
pub fn compute_files(snap: &Snapshot, year: i32) -> Vec<&'static str> {
    let mut files = vec!["form8949.csv", "schedule_d.csv", "form8283.csv"];
    if se_result_for(snap, year).is_some() {
        files.push("schedule_se.csv");
    }
    files
}

// ── Export execution ──────────────────────────────────────────────────────────

/// Perform the export: create the exclusive directory and write the four form CSVs.
///
/// 1. Calls `fsperms::mkdir_owner_only_exclusive(out_dir)` [D1b, R0-I1] — FAILS with
///    `AlreadyExists` if the dir pre-exists; nothing is written in that case.
/// 2. Assembles the SE result PROFILE-GATED (mirrors `cmd/tax.rs:79–106`).
/// 3. Calls `btctax_cli::render::write_form_csvs` with the assembled inputs.
///
/// Returns the written dir path on success.  On `AlreadyExists` (same-second re-export
/// OR pre-created dir) returns an error with nothing written — the exclusive-create
/// contract [R0-I1] is satisfied.
///
/// [R0-N3] The confirmation modal in `main.rs` is the sole call site; this is a
/// procedural guarantee enforced by KAT-E10 and the whole-diff review.
pub fn do_export(
    snap: &Snapshot,
    state: &ExportConfirmState,
) -> Result<PathBuf, btctax_cli::CliError> {
    // EXCLUSIVE create — must precede write_form_csvs [R0-I1].
    // Fails with AlreadyExists on a pre-existing dir; nothing is written.
    fsperms::mkdir_owner_only_exclusive(&state.out_dir).map_err(btctax_cli::CliError::Store)?;

    // SE assembly — PROFILE-GATED, mirrors cmd/tax.rs:79–106 exactly.
    let year = state.year;
    let se_result = match snap.profiles.get(&year) {
        Some(p) => {
            let table_opt = snap.tables.table_for(year);
            table_opt.and_then(|t| {
                compute_se_tax(
                    &snap.state,
                    year,
                    p.filing_status,
                    t,
                    p.w2_ss_wages,
                    p.w2_medicare_wages,
                    p.schedule_c_expenses,
                )
            })
        }
        None => None, // no profile → no SE figure → no schedule_se.csv
    };

    btctax_cli::render::write_form_csvs(
        &state.out_dir,
        &snap.state,
        year,
        se_result.as_ref(),
        &snap.donation_details,
    )?;

    Ok(state.out_dir.clone())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_adapters::BundledTaxTables;
    use btctax_cli::CliConfig;
    use btctax_core::{
        event::IncomeKind,
        identity::{EventId, Source, SourceRef},
        state::{IncomeRecord, LedgerState},
        Carryforward, FilingStatus, TaxProfile,
    };
    use rust_decimal::Decimal;
    use std::collections::BTreeMap;
    use time::macros::datetime;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_event_id(tag: &str) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(tag))
    }

    fn make_date(y: i32, m: u8, d: u8) -> btctax_core::TaxDate {
        time::Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    /// Build a minimal Snapshot with the given state and profiles.
    fn make_snapshot(state: LedgerState, profiles: BTreeMap<i32, TaxProfile>) -> Snapshot {
        Snapshot {
            events: vec![],
            state,
            cli_config: CliConfig::default(),
            profiles,
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
        }
    }

    /// Build a `TaxProfile` for Single filer with the given SE-relevant fields.
    fn make_se_profile(
        w2_ss: Decimal,
        w2_medicare: Decimal,
        schedule_c_expenses: Decimal,
    ) -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: Decimal::from(50_000i64),
            magi_excluding_crypto: Decimal::from(50_000i64),
            qualified_dividends_and_other_pref_income: Decimal::ZERO,
            other_net_capital_gain: Decimal::ZERO,
            capital_loss_carryforward_in: Carryforward::default(),
            w2_ss_wages: w2_ss,
            w2_medicare_wages: w2_medicare,
            schedule_c_expenses,
        }
    }

    /// Add a business mining income record to a LedgerState.
    fn add_mining_income(state: &mut LedgerState, fmv: Decimal, year: i32) {
        state.income_recognized.push(IncomeRecord {
            event: make_event_id(&format!("mining-{year}")),
            recognized_at: make_date(year, 3, 1),
            sat: 100_000_000,
            usd_fmv: fmv,
            kind: IncomeKind::Mining,
            business: true,
        });
    }

    // ── KAT-E6 — Timestamped dir uniqueness / determinism ────────────────────

    /// KAT-E6: `export_dir_for` is a pure function; calling with a fixed timestamp gives
    /// a deterministic suffix; a different timestamp gives a different suffix.
    #[test]
    fn e6_export_dir_for_deterministic_and_unique() {
        let vault_path = std::path::Path::new("/tmp/test/vault.pgp");

        let ts1 = datetime!(2025-10-24 14:30:22 UTC);
        let dir1 = export_dir_for(vault_path, ts1);
        assert!(
            dir1.to_string_lossy()
                .ends_with("btctax-export-20251024-143022Z"),
            "dir1 must end with btctax-export-20251024-143022Z; got {:?}",
            dir1
        );

        let ts2 = datetime!(2026-01-15 09:05:07 UTC);
        let dir2 = export_dir_for(vault_path, ts2);
        assert!(
            dir2.to_string_lossy()
                .ends_with("btctax-export-20260115-090507Z"),
            "dir2 must end with btctax-export-20260115-090507Z; got {:?}",
            dir2
        );

        assert_ne!(dir1, dir2, "different timestamps must yield different dirs");
    }

    // ── KAT-E4 — Hard-coded golden figures (W-2 swap-catching) ───────────────

    /// KAT-E4: hard-coded golden figures for the swap-catching fixture.
    ///
    /// Fixture: TY2025, Single, mining $100,000 gross + $60,000 Schedule C expenses
    /// (→ net_se $40,000), w2_ss_wages $150,000, w2_medicare_wages $170,000.
    /// Both W-2 caps BIND and DIFFER — swapping them changes the answer.
    ///
    /// Swap-catching check (documented): swapping the W-2 params gives:
    ///   ss_cap = max(0, 176,100 − 170,000) = 6,100 → ss = 12.4% × 6,100 = $756.40
    ///   addl_threshold = max(0, 200,000 − 150,000) = 50,000 → addl = 0 (36,940 < 50,000)
    /// These differ from the goldens below — proving the test catches a swap.
    #[test]
    fn e4_golden_figures_swap_catching() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "e4-golden-pass";
        btctax_cli::cmd::init::run(&vault, &btctax_store::Passphrase::new(pp_str.into()), &key)
            .unwrap();

        let export_now = datetime!(2025-06-15 10:00:00 UTC);

        // Build Snapshot: mining $100,000, $60,000 expenses, w2_ss $150,000, w2_medicare $170,000.
        let mut state = LedgerState::default();
        add_mining_income(&mut state, Decimal::from(100_000i64), 2025);
        let mut profiles = BTreeMap::new();
        profiles.insert(
            2025,
            make_se_profile(
                Decimal::from(150_000i64),
                Decimal::from(170_000i64),
                Decimal::from(60_000i64),
            ),
        );
        let snap = make_snapshot(state, profiles);

        let out_dir = export_dir_for(&vault, export_now);
        let modal = ExportConfirmState {
            year: 2025,
            out_dir: out_dir.clone(),
            files: compute_files(&snap, 2025),
            export_now,
        };

        do_export(&snap, &modal).expect("export must succeed");

        // Read back schedule_se.csv and assert hard-coded goldens.
        // Parse as plain text (no csv crate dependency in btctax-tui).
        let se_path = out_dir.join("schedule_se.csv");
        assert!(se_path.exists(), "schedule_se.csv must exist");

        let csv_text = std::fs::read_to_string(&se_path).expect("must read schedule_se.csv");
        let lines: Vec<&str> = csv_text.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "schedule_se.csv must have header + 1 data row"
        );
        let row: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(row.len(), 7, "data row must have 7 fields");

        // Column order: net_se_earnings(0), se_base_9235(1), ss_component(2),
        //               medicare_component(3), additional_medicare_component(4),
        //               total_se_tax(5), deductible_half(6).
        //
        // Hand-verified goldens (see spec KAT-E4):
        //   net_se  = 100,000 − 60,000 = 40,000
        //   base    = round_cents(40,000 × 0.9235) = 36,940.00
        //   ss      = 12.4% × min(36,940, 176,100 − 150,000 = 26,100) = 12.4% × 26,100 = 3,236.40
        //   medicare= 2.9% × 36,940 = 1,071.26
        //   addl    = 0.9% × max(0, 36,940 − (200,000 − 170,000 = 30,000)) = 0.9% × 6,940 = 62.46
        //   total   = 3,236.40 + 1,071.26 + 62.46 = 4,370.12
        //   ded_half= round_cents((3,236.40 + 1,071.26) / 2) = 2,153.83
        //
        // Swap check: w2_ss=170k/w2_medicare=150k gives ss_cap=6,100→ss=756.40; addl_thr=50k→addl=0
        assert_eq!(row[0], "40000", "net_se_earnings golden");
        assert_eq!(row[1], "36940.00", "se_base_9235 golden");
        assert_eq!(row[2], "3236.40", "ss_component golden");
        assert_eq!(row[3], "1071.26", "medicare_component golden");
        assert_eq!(row[4], "62.46", "additional_medicare_component golden");
        assert_eq!(row[5], "4370.12", "total_se_tax golden");
        assert_eq!(row[6], "2153.83", "deductible_half golden");
    }

    // ── KAT-E5 — 0o600 file / 0o700 dir permissions (Unix only) ─────────────

    /// KAT-E5: all written CSVs are 0o600; the export dir is 0o700.
    #[cfg(unix)]
    #[test]
    fn e5_file_and_dir_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "e5-perms-pass";
        btctax_cli::cmd::init::run(&vault, &btctax_store::Passphrase::new(pp_str.into()), &key)
            .unwrap();

        let export_now = datetime!(2025-06-15 11:00:00 UTC);
        let mut state = LedgerState::default();
        add_mining_income(&mut state, Decimal::from(50_000i64), 2025);
        let mut profiles = BTreeMap::new();
        profiles.insert(
            2025,
            make_se_profile(Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
        );
        let snap = make_snapshot(state, profiles);

        let out_dir = export_dir_for(&vault, export_now);
        let modal = ExportConfirmState {
            year: 2025,
            out_dir: out_dir.clone(),
            files: compute_files(&snap, 2025),
            export_now,
        };

        do_export(&snap, &modal).expect("export must succeed");

        // Dir must be 0o700.
        let dir_mode = std::fs::metadata(&out_dir).unwrap().permissions().mode();
        assert_eq!(
            dir_mode & 0o777,
            0o700,
            "export dir must be 0o700, got {:#o}",
            dir_mode & 0o777
        );

        // All written CSVs must be 0o600.
        for name in [
            "form8949.csv",
            "schedule_d.csv",
            "form8283.csv",
            "schedule_se.csv",
        ] {
            let path = out_dir.join(name);
            if path.exists() {
                let mode = std::fs::metadata(&path).unwrap().permissions().mode();
                assert_eq!(
                    mode & 0o777,
                    0o600,
                    "{name} must be 0o600, got {:#o}",
                    mode & 0o777
                );
            }
        }
    }

    // ── KAT-E9 — schedule_se.csv absent when SE result is absent ─────────────

    /// KAT-E9(a): no business income → no schedule_se.csv.
    #[test]
    fn e9a_no_se_income_no_schedule_se_csv() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        btctax_cli::cmd::init::run(
            &vault,
            &btctax_store::Passphrase::new("e9a-pass".into()),
            &key,
        )
        .unwrap();

        let export_now = datetime!(2025-07-01 12:00:00 UTC);
        // No income records at all.
        let state = LedgerState::default();
        let mut profiles = BTreeMap::new();
        profiles.insert(
            2025,
            make_se_profile(Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
        );
        let snap = make_snapshot(state, profiles);

        let files = compute_files(&snap, 2025);
        assert!(
            !files.contains(&"schedule_se.csv"),
            "files must NOT include schedule_se.csv when no SE income"
        );

        let out_dir = export_dir_for(&vault, export_now);
        let modal = ExportConfirmState {
            year: 2025,
            out_dir: out_dir.clone(),
            files,
            export_now,
        };

        do_export(&snap, &modal).expect("export must succeed");
        assert!(
            !out_dir.join("schedule_se.csv").exists(),
            "schedule_se.csv must NOT exist when no SE income"
        );
    }

    /// KAT-E9(b): business income present but NO TaxProfile → no schedule_se.csv.
    /// Profile gate mirrors cmd/tax.rs:79–106.
    #[test]
    fn e9b_no_profile_no_schedule_se_csv() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        btctax_cli::cmd::init::run(
            &vault,
            &btctax_store::Passphrase::new("e9b-pass".into()),
            &key,
        )
        .unwrap();

        let export_now = datetime!(2025-07-02 12:00:00 UTC);
        let mut state = LedgerState::default();
        add_mining_income(&mut state, Decimal::from(50_000i64), 2025);
        // NO profile for 2025.
        let snap = make_snapshot(state, BTreeMap::new());

        let files = compute_files(&snap, 2025);
        assert!(
            !files.contains(&"schedule_se.csv"),
            "files must NOT include schedule_se.csv when no profile"
        );

        let out_dir = export_dir_for(&vault, export_now);
        let modal = ExportConfirmState {
            year: 2025,
            out_dir: out_dir.clone(),
            files,
            export_now,
        };

        do_export(&snap, &modal).expect("export must succeed");
        assert!(
            !out_dir.join("schedule_se.csv").exists(),
            "schedule_se.csv must NOT exist when no profile (profile-gated)"
        );
    }

    // ── KAT-E11 — Pre-created export dir → error, NOTHING written ────────────

    /// KAT-E11: if the export dir pre-exists, `do_export` returns `Err(AlreadyExists)`
    /// and the sentinel file inside the pre-created dir is untouched.
    #[test]
    fn e11_pre_created_dir_fails_nothing_written() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        btctax_cli::cmd::init::run(
            &vault,
            &btctax_store::Passphrase::new("e11-pass".into()),
            &key,
        )
        .unwrap();

        let export_now = datetime!(2025-08-01 08:00:00 UTC);
        let out_dir = export_dir_for(&vault, export_now);

        // Pre-create the exact export dir with a sentinel file.
        std::fs::create_dir_all(&out_dir).unwrap();
        let sentinel = out_dir.join("sentinel.txt");
        std::fs::write(&sentinel, b"sentinel content").unwrap();

        let mut state = LedgerState::default();
        add_mining_income(&mut state, Decimal::from(50_000i64), 2025);
        let mut profiles = BTreeMap::new();
        profiles.insert(
            2025,
            make_se_profile(Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
        );
        let snap = make_snapshot(state, profiles);

        let modal = ExportConfirmState {
            year: 2025,
            out_dir: out_dir.clone(),
            files: compute_files(&snap, 2025),
            export_now,
        };

        // Export must fail — pre-existing dir triggers AlreadyExists.
        let result = do_export(&snap, &modal);
        assert!(
            result.is_err(),
            "do_export must return Err when the export dir pre-exists"
        );

        // The form CSVs must NOT have been written.
        assert!(
            !out_dir.join("form8949.csv").exists(),
            "form8949.csv must NOT exist after AlreadyExists failure"
        );
        assert!(
            !out_dir.join("schedule_d.csv").exists(),
            "schedule_d.csv must NOT exist after AlreadyExists failure"
        );
        assert!(
            !out_dir.join("form8283.csv").exists(),
            "form8283.csv must NOT exist after AlreadyExists failure"
        );

        // Sentinel file must be untouched (no truncation of pre-existing files).
        let sentinel_content = std::fs::read(&sentinel).unwrap();
        assert_eq!(
            sentinel_content, b"sentinel content",
            "sentinel file must be byte-identical after failed export"
        );
    }

    // ── KAT-E10 — Mechanized source gate ─────────────────────────────────────

    /// KAT-E10: mechanized source gate for the D5 forbidden-token table.
    ///
    /// Walks `crates/btctax-tui/src/`, scans each file's **non-test region** (the portion
    /// before the first `#[cfg(test)]` marker) for forbidden write-class tokens, applies
    /// the two documented exceptions:
    ///   1. `export.rs` — permitted to use write-class I/O tokens + `write_form_csvs`.
    ///   2. Test regions — permitted to use `cmd::init::run` + fixture write verbs.
    ///
    /// Fails with `file:line` on any other hit.
    ///
    /// Self-check: the test plants a forbidden token in a temp file (written via runtime
    /// string construction so no literal forbidden token appears in this source file) and
    /// asserts the scanner detects it — tests the tester.
    ///
    /// [M-R2-1] The scanner strips `//` line comments (and `///` doc-comments, which also
    /// start with `//`) before matching, so guarantee doc-comments that NAME a forbidden
    /// token as documentation do NOT trigger a false positive.
    #[test]
    fn e10_mechanized_source_gate() {
        use std::io::{BufRead, BufReader};

        // Locate the btctax-tui/src directory from CARGO_MANIFEST_DIR.
        let src_dir = {
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR must be set in tests");
            std::path::PathBuf::from(manifest).join("src")
        };
        assert!(
            src_dir.exists(),
            "btctax-tui/src must exist at {:?}",
            src_dir
        );

        // ── Token lists ───────────────────────────────────────────────────────
        //
        // Tokens forbidden EVERYWHERE in btctax-tui (no exception even in export.rs):
        let everywhere_tokens: &[&str] = &[
            "save(",
            "append_",
            "cmd::",
            "conn(",
            "export_snapshot",
            "write_csv_exports",
        ];

        // Write-class tokens: forbidden outside export.rs in non-test code.
        let write_class_tokens: &[&str] = &[
            "write_form_csvs",
            "open_owner_only",
            "mkdir_owner_only",
            "mkdir_owner_only_exclusive",
            "fsperms",
            "File::create",
            "File::options",
            "OpenOptions",
            "fs::write",
            "write_owner_only",
            "create_dir",
            "create_dir_all",
            "DirBuilder",
            "set_permissions",
            "fs::copy",
            "fs::rename",
            "fs::remove_",
        ];

        // Five tokens forbidden everywhere (including test code; subset of everywhere_tokens;
        // excludes cmd:: which has a test-code exception).
        let test_region_tokens: &[&str] = &[
            "save(",
            "append_",
            "conn(",
            "export_snapshot",
            "write_csv_exports",
        ];

        // ── Comment stripping [M-R2-1] ────────────────────────────────────────
        /// Strip `//` comment suffix from a line (covers `//` and `///` doc-comments).
        fn strip_comment(line: &str) -> &str {
            if let Some(idx) = line.find("//") {
                &line[..idx]
            } else {
                line
            }
        }

        // ── Scan helper: non-test region ─────────────────────────────────────
        /// Scan a file's non-test region for the given tokens.
        /// Returns `(token, line_number)` pairs for any hits.
        fn scan_non_test(path: &std::path::Path, tokens: &[&str]) -> Vec<(String, usize)> {
            let file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => return vec![],
            };
            let reader = BufReader::new(file);
            let mut hits = Vec::new();
            let mut in_test = false;
            for (idx, line) in reader.lines().enumerate() {
                let line = line.unwrap_or_default();
                if line.trim_start().starts_with("#[cfg(test)]") {
                    in_test = true;
                }
                if !in_test {
                    let code = strip_comment(&line);
                    for &tok in tokens {
                        if code.contains(tok) {
                            hits.push((tok.to_string(), idx + 1));
                        }
                    }
                }
            }
            hits
        }

        // ── Scan helper: test region ─────────────────────────────────────────
        /// Scan a file's test region (after `#[cfg(test)]`) for the given tokens.
        fn scan_test_region(path: &std::path::Path, tokens: &[&str]) -> Vec<(String, usize)> {
            let content = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let test_start = match content.find("#[cfg(test)]") {
                Some(pos) => pos,
                None => return vec![],
            };
            let test_region = &content[test_start..];
            let prefix_line = content[..test_start].lines().count();
            let mut hits = Vec::new();
            for (idx, line) in test_region.lines().enumerate() {
                let code = strip_comment(line);
                for &tok in tokens {
                    if code.contains(tok) {
                        hits.push((tok.to_string(), prefix_line + idx + 1));
                    }
                }
            }
            hits
        }

        // ── Collect all .rs files under src/ ─────────────────────────────────
        let mut rs_files: Vec<std::path::PathBuf> = Vec::new();
        fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        collect_rs(&p, out);
                    } else if p.extension().is_some_and(|e| e == "rs") {
                        out.push(p);
                    }
                }
            }
        }
        collect_rs(&src_dir, &mut rs_files);
        assert!(!rs_files.is_empty(), "must find at least one .rs file");

        // ── Scan each file ────────────────────────────────────────────────────
        let mut violations: Vec<String> = Vec::new();

        for path in &rs_files {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_export = filename == "export.rs";

            // Check everywhere_tokens in non-test region of ALL files.
            // (export.rs is allowed to use write-class tokens but NOT the everywhere tokens.)
            {
                let hits = scan_non_test(path, everywhere_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden token {:?} (everywhere rule, non-test region)",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // Check write-class tokens in non-test region of non-export files.
            if !is_export {
                let hits = scan_non_test(path, write_class_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden write-class token {:?} (export.rs-only rule)",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // Check test-region-forbidden tokens in non-export test regions.
            // (export.rs test region excluded: it contains the self-check which uses
            //  runtime-constructed tokens, but we skip it to keep the scan simple.)
            if !is_export {
                let hits = scan_test_region(path, test_region_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden token {:?} found in test region",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }
        }

        // ── Self-check: verify the scanner catches planted tokens ─────────────
        // Use runtime string construction so no literal forbidden token appears in
        // this source file (avoids false positives when export.rs is scanned).
        {
            let tmpdir = tempfile::tempdir().unwrap();
            let planted_path = tmpdir.path().join("planted_test.rs");

            // Construct the forbidden token at runtime (never appears literally in source).
            let tok_save = format!("{}(", "save"); // "save("
            let tok_conn = format!("{}(", "conn"); // "conn("
            let tok_exp = "export_snapshot".to_string();

            let content = format!(
                "// planted self-check file\npub fn bad() {{\n    let _ = {tok_save});\n    let _ = {tok_conn});\n    unreachable!(\"{tok_exp}\");\n}}\n"
            );
            std::fs::write(&planted_path, &content).unwrap();

            let hits_everywhere = scan_non_test(&planted_path, everywhere_tokens);
            assert!(
                hits_everywhere.iter().any(|(t, _)| t == "save("),
                "self-check FAILED: scanner did not detect planted 'save(' — gate is broken"
            );
            assert!(
                hits_everywhere.iter().any(|(t, _)| t == "conn("),
                "self-check FAILED: scanner did not detect planted 'conn(' — gate is broken"
            );
            assert!(
                hits_everywhere.iter().any(|(t, _)| t == "export_snapshot"),
                "self-check FAILED: scanner did not detect planted 'export_snapshot' — gate is broken"
            );
        }

        assert!(
            violations.is_empty(),
            "Source gate violations found:\n{}",
            violations.join("\n")
        );
    }
}
