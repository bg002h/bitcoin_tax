# Whole-diff review (Phase E) — feat/attest-export-gate — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review. Diff `main (afb0807)..HEAD` — 3 task commits (T1-T3), 17 files, +1004/−73.
Contract: `design/SPEC_attest_export_gate.md` (R0-GREEN, 2 rounds). Sub-project 3 (FINAL) of auto-pseudo-reconcile.

## The design + both-paths guard
Replaces sub-2's interim [I3] blanket export refusal with a typed-attestation gate. `require_attestation`
(lib.rs:89) is a PURE exact-compare (`Some(p) if p==ATTEST_PHRASE → Ok`; `Some(_) → AttestationFailed`;
`None → AttestationRequired`; trimmed, case-sensitive) — no TTY read (R0-I2, KAT-deterministic). Both
`ATTEST_PHRASE` + `require_attestation` are `pub`.
- **CLI** — `cmd::admin::export_snapshot` calls `require_attestation(attest)?` when `pseudo_active()`, checked
  FIRST (admin.rs:63, before any bytes → out_dir untouched on refuse). The TTY prompt lives in the main.rs arm
  (not the library).
- **[★ R0-C1] Viewer** — `btctax-tui/lib.rs:304` the `e` export modal validates the typed phrase via the SAME
  `btctax_cli::require_attestation` when pseudo-active (typed-word modal, mirrors safe-harbor-attest); plain
  Enter/Esc otherwise. This closes the round-1 Critical (viewer could write fictional 8949 ungated). No other
  form-writer exists (R0 gap-swept).

## Fault-injection (both restored byte-for-byte)
- **[★ shared exact-phrase guard] CONFIRMED load-bearing.** Flipping `Some(_) → AttestationFailed` to
  `Some(_) → Ok(())` (accept any phrase) drove `viewer_pseudo_active_export_requires_typed_phrase` RED — and
  since the CLI path (`admin.rs:63`) calls the IDENTICAL helper via `?`, both gates fall together. (The
  implementer independently confirmed the CLI ★ KATs `require_attestation_is_exact_trimmed_case_sensitive` +
  `export_pseudo_active_wrong_phrase_refused` go RED under an `eq_ignore_ascii_case` fault.) One helper, two
  callers — no divergent logic.
- Trigger = `pseudo_active()` (`pseudo_synthetic_count>0`) — a fully-real ledger exports prompt-free
  (`export_not_pseudo_active_ignores_attest`); a wrong phrase is `AttestationFailed` regardless of TTY (R0-I1);
  error/prompt strings are built from `ATTEST_PHRASE` (`attest_strings_contain_phrase`, R0-M1).
- The removed `PseudoActiveExport` variant's sub-2 KAT was correctly rewritten to
  `attest_gate_supersedes_interim_i3_refusal` (R0-I3); sub-2's `pseudo_marker_...absent_from_every_export_file`
  + the E10 source gate stay green (output still clean — attestation PERMITS, adds no markers).

## Full suite
`cargo test --workspace --locked` **1140 passed / 0 failed**; clippy -D + fmt clean (implementer, this tree);
`make docs` regenerated `btctax-export-snapshot.1` (`--attest`) + the viewer attest note.

## SemVer
btctax-cli + btctax-tui; new capability + `--attest` flag + a viewer modal. MINOR. No core change.

**SHIP — this completes the auto-pseudo-reconcile program (all 3 sub-projects).**
