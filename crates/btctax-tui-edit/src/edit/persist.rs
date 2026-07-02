//! The ONLY module in btctax-tui-edit that touches the mutation surface:
//! conn() / save() / tax_profile::set / append_decision live here and nowhere else.
//!
//! Guarantee: "writes ONLY append-only events + typed side-table upserts via
//! edit/persist.rs, each behind an explicit payload-showing confirmation; the vault
//! file only via Vault::save's atomic path."
//!
//! # VaultLock note
//! The editor holds the vault's exclusive lock for its entire lifetime
//! (session.rs:53–58, vault.rs:137–142). The CLI or viewer cannot run concurrently
//! against the same vault. There is no concurrent-writer case.
//!
//! # Vault-creating constructors
//! `Session::create` / `Session::repair` / `Vault::create` / `Vault::repair`
//! are FORBIDDEN in non-test code of this crate (R0-I1). They create/overwrite a
//! vault file outside `Vault::save`'s atomic path, which violates the guarantee.
//! The mechanized gate (KAT-G1) enforces this crate-wide.

/// Upsert the tax profile for `year` and atomically save the vault.
///
/// Mirrors `cmd::tax::set_profile` (cmd/tax.rs:14–23) minus the open/drop —
/// the editor operates on its HELD session (the VaultLock must stay acquired).
///
/// # Called only from the mutation-confirmation modal
/// This is a `pub fn` freely callable; "the confirmation modal gates the ONLY
/// call site" is a procedural guarantee (enforced by KAT-G1's confinement of
/// the surface, the KATs, and whole-diff review), not a type-level proof.
/// A sealed confirmation-token type is a FOLLOWUP if the editor grows more flows.
///
/// # Failed-save semantics (R0-M1)
/// When `tax_profile::set` succeeds but `save` fails, the in-memory session
/// already carries the confirmed upsert while the on-disk vault remains
/// the pre-action state (the atomic path leaves the old image). This divergence
/// is intentional and safe — do NOT roll back the side-table. The upsert is
/// idempotent; a retry re-runs it on the next confirmed action.
pub fn persist_tax_profile(
    session: &mut btctax_cli::Session,
    year: i32,
    p: &btctax_core::TaxProfile,
) -> Result<(), btctax_cli::CliError> {
    btctax_cli::tax_profile::set(session.conn(), year, p)?; // typed side-table upsert
    session.save()?; // encrypt + atomic_write
    Ok(())
}

/// Append a `ClassifyInbound` decision event and atomically save the vault.
///
/// `payload` is the **fully-validated** `EventPayload::ClassifyInbound(…)` built by
/// the classify-inbound form.  `now` is the caller-supplied `OffsetDateTime`
/// (injected at Enter-press for test determinism; never derived inside this fn).
///
/// # Strict-append semantics
/// Calls `append_decision(conn, payload, now, UTC, None)` → the event is assigned
/// `decision_seq = MAX(existing) + 1`.  After `session.save()` the vault image on
/// disk contains the new event at the tail.  The KAT-P2a strict-prefix test
/// verifies this invariant.
///
/// # Called only from the classify-inbound confirmation modal
/// Same procedural guarantee as `persist_tax_profile` (see doc there).
pub fn persist_classify_inbound(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, btctax_cli::CliError> {
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    session.save()?;
    Ok(id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── KAT-P1 — append-only prefix test (side-table form) ───────────────────
    //
    // For chunk 1: a tax-profile set is a SIDE-TABLE upsert, NOT an event append.
    // The degenerate strong form: event log UNCHANGED by a profile set.
    // In-memory AND after drop+reopen, plus:
    //   - mutation-actually-happened guard (profile round-trips)
    //   - second differing upsert still leaves log == pre

    fn fixture_profile() -> btctax_core::TaxProfile {
        use btctax_core::{Carryforward, FilingStatus, TaxProfile};
        use rust_decimal_macros::dec;
        TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: dec!(120000),
            magi_excluding_crypto: dec!(130000),
            qualified_dividends_and_other_pref_income: dec!(5000),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(0),
                long: dec!(0),
            },
            w2_ss_wages: dec!(80000),
            w2_medicare_wages: dec!(85000),
            schedule_c_expenses: dec!(3000),
        }
    }

    #[test]
    fn kat_p1_append_only_prefix_side_table_form() {
        use btctax_core::event::{EventPayload, MethodElection};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{LotMethod, TaxProfile};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p1-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed ≥ 2 decision events via append_decision (fixture setup — test-region exception)
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let now = OffsetDateTime::now_utc();
            let tz = UtcOffset::UTC;
            let p1 = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: LotMethod::Fifo,
            });
            let p2 = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2025 - 01 - 01),
                method: LotMethod::Hifo,
            });
            append_decision(session.conn(), p1, now, tz, None).unwrap();
            append_decision(session.conn(), p2, now, tz, None).unwrap();
            session.save().unwrap();
        }

        // Open the editor's session and capture the pre-state
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 2, "should have seeded exactly 2 events");

        let p = fixture_profile();
        persist_tax_profile(&mut session, 2025, &p).unwrap();

        // In-memory: log unchanged
        let post_inmem = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post_inmem, pre,
            "event log must be UNCHANGED in-memory after profile set (side-table upsert)"
        );

        // Drop + reopen: persisted image also unchanged
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, pre,
            "event log must be UNCHANGED on disk after profile set"
        );

        // Mutation-actually-happened guard (test cannot vacuously pass on a no-op)
        let stored = session2.tax_profile(2025).unwrap().unwrap();
        assert_eq!(
            stored, p,
            "profile must be readable after persist_tax_profile"
        );

        // Second differing upsert: log still == pre
        let p2 = TaxProfile {
            ordinary_taxable_income: dec!(200000),
            ..p.clone()
        };
        drop(session2);
        let mut session3 =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        persist_tax_profile(&mut session3, 2025, &p2).unwrap();
        let post3 = load_all_ordered(session3.conn()).unwrap();
        assert_eq!(
            post3, pre,
            "log still unchanged after second (differing) upsert"
        );
        let stored2 = session3.tax_profile(2025).unwrap().unwrap();
        assert_eq!(stored2, p2, "second upsert value is readable");
    }

    // ── KAT-P2a — append-only strict prefix test (classify-inbound append form) ──
    //
    // Invariant: persist_classify_inbound appends EXACTLY one decision event
    // to the tail of the event log.
    //
    // Strict-prefix formula (spec §D5):
    //   post == pre ++ [new_event]
    //   post[pre.len()].decision_seq == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
    //
    // Also asserts: payload round-trips, returned EventId matches appended row.

    #[test]
    fn kat_p2a_append_only_strict_prefix_classify_inbound() {
        use btctax_core::event::{ClassifyInbound, EventPayload, InboundClass, IncomeKind};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2a-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed 1 import TransferIn event + 1 decision event to create a non-trivial pre-state.
        // The import event is used as the ClassifyInbound target.
        let import_event_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2a"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![btctax_core::event::LedgerEvent {
                id: import_event_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: None,
                payload: btctax_core::event::EventPayload::TransferIn(
                    btctax_core::event::TransferIn {
                        sat: 100_000,
                        src_addr: None,
                        txid: None,
                    },
                ),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            // Seed one decision so MAX(decision_seq) == 1 in pre.
            let now = OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        };

        // Open editor session, capture pre-state.
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            pre.len(),
            2,
            "pre must have exactly 2 events (1 import + 1 decision)"
        );
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Build ClassifyInbound payload.
        let payload = EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: import_event_id.clone(),
            as_: InboundClass::Income {
                kind: IncomeKind::Mining,
                fmv: Some(dec!(30000.00)),
                business: false,
            },
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let returned_id = persist_classify_inbound(&mut session, payload.clone(), now).unwrap();

        // ── Strict-prefix assertion ───────────────────────────────────────────
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 1,
            "post must be exactly pre.len()+1"
        );

        // Pre-prefix is byte-for-byte identical.
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows must be unchanged (strict prefix)"
        );

        // New tail row: decision_seq == pre_max + 1.
        let tail = &post[pre.len()];
        let tail_seq = tail
            .decision_seq
            .expect("new tail row must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "tail decision_seq must be pre_max+1 (spec KAT-P2a formula)"
        );

        // Returned EventId matches the tail row's event_id.
        let tail_event_id = EventId::Decision {
            seq: tail_seq as u64,
        };
        assert_eq!(
            returned_id, tail_event_id,
            "returned EventId must equal Decision {{ seq: tail_seq }}"
        );

        // Payload round-trips: deserialise tail row and compare.
        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail payload_json must deserialise");
        assert_eq!(
            stored_payload, payload,
            "stored payload must round-trip equal to the one we appended"
        );

        // Drop + reopen: same strict-prefix holds on disk.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, post,
            "on-disk image must equal in-memory post after save"
        );
    }

    // ── KAT-G1 — the editor's mechanized source gate ─────────────────────────
    //
    // Clones the E10 scanner structure (export.rs:690–919 in btctax-tui):
    //   - src-walk via CARGO_MANIFEST_DIR
    //   - non-test/test region split at first #[cfg(test)]
    //   - // comment stripping before matching
    //   - file:line failure output
    //   - plant-a-token self-check with runtime-constructed strings
    //
    // Allowlist: edit/persist.rs is the ONLY file permitted to use the write-mutation
    // tokens (conn( / save( / tax_profile::set / append_) in non-test code.
    //
    // R0-I1: Session::create / Session::repair / Vault::create / Vault::repair are
    // FORBIDDEN everywhere in non-test code (they create/overwrite a vault file
    // outside Vault::save's atomic path). One of these (Session::create) is planted
    // in the self-check so the gate cannot silently drop R0-I1 enforcement.

    #[test]
    fn kat_g1_mechanized_source_gate() {
        use std::io::{BufRead, BufReader};

        // ── Locate this crate's src/ directory ────────────────────────────────
        let src_dir = {
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR must be set in tests");
            std::path::PathBuf::from(manifest).join("src")
        };
        assert!(
            src_dir.exists(),
            "btctax-tui-edit/src must exist at {:?}",
            src_dir
        );

        // ── Token lists ───────────────────────────────────────────────────────

        // Non-test forbidden everywhere (no allowlist):
        //   cmd:: — cmd fns open/drop their own sessions (wrong lifecycle, deadlocks held lock)
        //   Session::create / Session::repair / Vault::create / Vault::repair — R0-I1:
        //     these constructors create/overwrite a vault file outside Vault::save's atomic path
        //   export_snapshot / write_csv_exports / write_form_csvs — viewer-only export surface
        let everywhere_tokens: &[&str] = &[
            "cmd::",
            "Session::create",
            "Session::repair",
            "Vault::create",
            "Vault::repair",
            "export_snapshot",
            "write_csv_exports",
            "write_form_csvs",
        ];

        // Non-test FS-write tokens forbidden everywhere (editor performs NO direct fs writes;
        // vault writes go only via Vault::save's atomic path inside btctax-store):
        let fs_write_tokens: &[&str] = &[
            "fsperms",
            "open_owner_only",
            "mkdir_owner_only",
            "File::create",
            "File::options",
            "OpenOptions",
            "fs::write",
            "write_owner_only",
            "create_dir",
            "DirBuilder",
            "set_permissions",
            "fs::copy",
            "fs::rename",
            "fs::remove_",
        ];

        // Non-test write-mutation tokens — FORBIDDEN outside edit/persist.rs:
        let persist_only_tokens: &[&str] = &["conn(", "save(", "tax_profile::set", "append_"];

        // Test-region forbidden everywhere (no viewer export surface in the editor):
        let test_region_tokens: &[&str] =
            &["export_snapshot", "write_csv_exports", "write_form_csvs"];

        // ── Comment stripping [M-R2-1] ────────────────────────────────────────
        /// Strip // comment suffix (covers // and /// doc-comments).
        fn strip_comment(line: &str) -> &str {
            if let Some(idx) = line.find("//") {
                &line[..idx]
            } else {
                line
            }
        }

        // ── Scan helper: non-test region ──────────────────────────────────────
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

        // ── Scan helper: test region ──────────────────────────────────────────
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
        assert!(
            !rs_files.is_empty(),
            "must find at least one .rs file in src/"
        );

        // ── Classify each file ────────────────────────────────────────────────
        // edit/persist.rs is the allowlisted file for write-mutation tokens and is
        // excluded from the test-region scan (mirrors viewer's exclusion of export.rs).
        fn is_persist_rs(path: &std::path::Path) -> bool {
            let fname = path.file_name().map(|n| n == "persist.rs").unwrap_or(false);
            let in_edit = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|d| d == "edit")
                .unwrap_or(false);
            fname && in_edit
        }

        // ── Scan each file ────────────────────────────────────────────────────
        let mut violations: Vec<String> = Vec::new();

        for path in &rs_files {
            let is_persist = is_persist_rs(path);

            // (1) everywhere_tokens in non-test region of ALL files.
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

            // (2) fs_write_tokens in non-test region of ALL files.
            {
                let hits = scan_non_test(path, fs_write_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden FS-write token {:?} in non-test region",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // (3) write-mutation tokens in non-test of ALL files EXCEPT edit/persist.rs.
            if !is_persist {
                let hits = scan_non_test(path, persist_only_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden write-mutation token {:?} outside edit/persist.rs",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // (4) test-region forbidden tokens in ALL files EXCEPT edit/persist.rs.
            // (edit/persist.rs excluded: its test region contains this gate + self-check.)
            if !is_persist {
                let hits = scan_test_region(path, test_region_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden viewer-export token {:?} in test region",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }
        }

        // ── Self-check: verify the scanner catches planted tokens ─────────────
        //
        // All tokens are runtime-constructed so NO literal forbidden token appears
        // in this source file (avoids false positives when edit/persist.rs is scanned).
        {
            let tmpdir = tempfile::tempdir().unwrap();
            let planted_path = tmpdir.path().join("planted_test.rs");

            // Construct forbidden tokens at runtime — never appear literally in source.
            let tok_save = format!("{}(", "save"); // "save("
            let tok_conn = format!("{}(", "conn"); // "conn("
            let tok_tax_set = format!("{}::{}", "tax_profile", "set"); // "tax_profile::set"
            let tok_session_create = format!("{}::{}", "Session", "create"); // "Session::create" [R0-I1]

            let content = format!(
                "// planted self-check file\n\
                 pub fn bad() {{\n\
                 \tlet _ = {tok_save});\n\
                 \tlet _ = {tok_conn});\n\
                 \tlet _ = {tok_tax_set}(conn, 2025, &p);\n\
                 \tlet _ = {tok_session_create}(&path, &pp);\n\
                 }}\n"
            );
            std::fs::write(&planted_path, &content).unwrap();

            // Verify scanner catches persist-only tokens.
            let hits_persist = scan_non_test(&planted_path, persist_only_tokens);
            assert!(
                hits_persist.iter().any(|(t, _)| t == "save("),
                "self-check FAILED: scanner did not detect planted write-mutation token — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "conn("),
                "self-check FAILED: scanner did not detect planted write-mutation token — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "tax_profile::set"),
                "self-check FAILED: scanner did not detect planted write-mutation token — gate is broken"
            );

            // Verify scanner catches the R0-I1 vault-creating constructor.
            let hits_everywhere = scan_non_test(&planted_path, everywhere_tokens);
            assert!(
                hits_everywhere.iter().any(|(t, _)| t == "Session::create"),
                "self-check FAILED: scanner did not detect planted Session::create [R0-I1] — gate is broken"
            );
        }

        // ── Assert clean ──────────────────────────────────────────────────────
        assert!(
            violations.is_empty(),
            "KAT-G1 source gate violations found:\n{}",
            violations.join("\n")
        );
    }
}
