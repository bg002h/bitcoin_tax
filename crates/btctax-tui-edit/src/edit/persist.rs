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
/// # Called only from the mutation-confirmation modal (Task 3)
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
// Task 2 skeleton: function body complete; call site wired in Task 3.
#[allow(dead_code)]
pub fn persist_tax_profile(
    session: &mut btctax_cli::Session,
    year: i32,
    p: &btctax_core::TaxProfile,
) -> Result<(), btctax_cli::CliError> {
    btctax_cli::tax_profile::set(session.conn(), year, p)?; // typed side-table upsert
    session.save()?; // encrypt + atomic_write
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
