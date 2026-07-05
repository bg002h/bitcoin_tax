# SPEC — attestation export gate (sub-project 3 of auto-pseudo-reconcile — FINAL)

**Source baseline:** `main` @ `afb0807` (branch `feat/attest-export-gate`). **Review status: R0 round 1 folded
(1C/3I/1M/1N — all merged IN-PLACE; surgical, no append). Awaiting R0 round 2.** Review:
`reviews/R0-spec-attest-export-gate-round-1.md`. Design of record: `design/BRAINSTORM_auto_pseudo_reconcile.md`.
**Cross-cutting decisions settled — do NOT re-brainstorm.** Sub-projects 1 (`514875b`) + 2 (`afb0807`) SHIPPED.

## Goal
Replace sub-2's **interim [I3] export refusal** with the settled **typed-attestation gate**: producing any
Form 8949 / Schedule D / 8283 / Schedule SE (or the snapshot SQLite + projection CSVs) while the ledger is
**pseudo-active** requires the user to affirm the exact phrase **`I attest this is true`**. A NOT-pseudo-active
ledger exports with no prompt (today's behavior). The deliberate friction that lets a fictional draft be
exported ON PURPOSE while making accidental filing impossible.

## [R0-C1] The gate covers BOTH form-writing paths (the "sole path" premise was FALSE)
Two shipped commands write the form CSVs; **both** must be gated:
1. **`btctax export-snapshot` (CLI)** — `cmd::admin::export_snapshot`, currently the interim [I3] refusal
   (admin.rs:56-57).
2. **`btctax-tui` VIEWER export** — `btctax-tui/src/export.rs:115 do_export` → `:143 write_form_csvs`, reached
   via `lib.rs:246` (`e` opens a modal) → `:169-181` (Enter → do_export). Currently a plain Enter/Esc confirm,
   **no pseudo check** — the exact bypass this sub-project exists to prevent (`pseudo on` → viewer `e`/Enter →
   fictional 8949 on disk). The viewer's projection honors the persisted pseudo flag (unlock.rs:173 →
   session.rs:461-464 → config.rs:125-126,40), so `pseudo_active()` is available there.
`btctax-tui-edit` is CLEAN — its source-gate (edit/persist.rs:1708-1757) forbids the form writers; NOT in scope.

## Settled decisions
- **Trigger = pseudo-active ONLY** (`state.pseudo_active()`, state.rs:268 = `pseudo_synthetic_count > 0` —
  complete, catches the basis-taint case). A fully-real ledger (even with the mode flag ON but 0 synthetics) is
  never gated. Gate ONLY when pseudo-active; not "always."
- **Exact phrase** `const ATTEST_PHRASE = "I attest this is true"` — compared TRIMMED, case-SENSITIVE, exact.
  **[R0-M1]** the prompt + error strings are BUILT from `ATTEST_PHRASE` (a KAT asserts they contain it — no drift).
- **Output stays clean** — sub-2's guard is unchanged: written files carry NO `[PSEUDO]` marker; the attestation
  merely PERMITS the export.
- Not persisted — a one-shot per-invocation gate; no event/side-table.

## Mechanism
- **[R0-I2] Pure library helper (no I/O):** `require_attestation(attest: Option<&str>) -> Result<(), CliError>`
  in btctax-cli — EXACT-COMPARE ONLY, no TTY read:
  - `attest.map(str::trim) == Some(ATTEST_PHRASE)` → `Ok(())`.
  - **[R0-I1]** `Some(_)` non-matching → `Err(AttestationFailed)` (a wrong phrase is FAILED regardless of env).
  - `None` → `Err(AttestationRequired)`.
  Keeps the library I/O-explicit (lib.rs:3, session.rs:2 invariant) and the KATs deterministic (no env-dependent
  TTY branch inside the fn — the round-1 hang risk).
- **CLI (`export-snapshot`):** `cmd::admin::export_snapshot` gains `attest: Option<&str>`; REPLACE admin.rs:56-57
  with `if state.pseudo_active() { require_attestation(attest)?; }` — still checked FIRST (before any bytes; a
  rejected attestation leaves `out_dir` untouched). The **TTY prompt** lives in the `ExportSnapshot` **main.rs
  arm** (main.rs:291-294, where every other prompt lives, e.g. :692-701): if pseudo-active + `--attest` absent
  + stdin is a TTY → prompt (built from `ATTEST_PHRASE`), read a line, pass as `Some(line)`; non-TTY + absent →
  the helper's `AttestationRequired`. (Mirrors plan→prompt→apply.)
- **TUI (`btctax-tui` viewer) [R0-C1]:** in the export flow, when `pseudo_active()`, the `e` modal becomes a
  TYPED-WORD modal (mirror the tui-edit safe-harbor-attest typed-word pattern) requiring `ATTEST_PHRASE` before
  `do_export`; Esc cancels; a wrong phrase does not export. When NOT pseudo-active → today's plain Enter/Esc
  confirm. Uses the shared `ATTEST_PHRASE`/exact-compare (btctax-tui depends on btctax-cli).
- **Errors:** replace `PseudoActiveExport(usize)` with `AttestationRequired` + `AttestationFailed` (both name
  `ATTEST_PHRASE` + that the state is pseudo-reconciled); main() → exit 2 / stderr (main.rs:42-43).

## [R0-I3] Sub-2 KAT to UPDATE (removing `PseudoActiveExport` won't compile otherwise)
`crates/btctax-cli/tests/pseudo_reconcile_cli.rs:138-162 export_snapshot_refused_while_pseudo_active` asserts
the removed `PseudoActiveExport` variant (:145-148) → rewrite it to the attestation behavior (missing/wrong →
refused; correct → permitted). Only references to the variant: admin.rs:57, lib.rs:61, that test — the plan
covers all three.

## KATs
CLI (pseudo_reconcile_cli.rs): `export_pseudo_active_correct_attest_writes_files`;
`export_pseudo_active_missing_attest_refused_out_dir_untouched` (`None`→`AttestationRequired`, no files);
`export_pseudo_active_wrong_phrase_refused` (`Some("i attest…")`/`Some("…!!")`/trailing-junk → `AttestationFailed`
— exact/trimmed/case-sensitive; **★ fault-inject target** — break the exact-compare ⇒ RED);
`export_not_pseudo_active_ignores_attest` **[R0-N1]** (fully-real ledger exports with no `--attest`; same file
SET, not byte-identical — sqlite embeds timestamps); the rewritten `export_snapshot_refused_...` →
`attest_gate_supersedes_interim_i3_refusal`; `attest_strings_contain_phrase` **[M1]**.
TUI (btctax-tui tests): `viewer_pseudo_active_export_requires_typed_phrase` (typing the phrase exports; wrong/
Esc does not); `viewer_not_pseudo_active_export_plain_confirm`. Re-run sub-2's
`pseudo_marker_...absent_from_every_export_file` (still green — output stays clean, both paths).

## Scope / SemVer / lockstep
**btctax-cli** (helper + admin.rs + main.rs arm + error variants) **+ btctax-tui** (viewer export modal) [R0-C1].
No core change. Behavior change: pseudo-active export refusal → attestable (MINOR — new capability + `--attest`
flag + a TUI modal). Lockstep: `make docs` (regen `btctax-export-snapshot.1` + the `?`-overlay note for the
viewer modal), doc-comments. No GUI schema_mirror (no GUI/tauri crate — verified). Update the sub-2 FOLLOWUPS
"interim [I3]" note → "replaced by sub-3 (attestation gate, both CLI + viewer)".

## Plan (TDD)
- **T1** — `ATTEST_PHRASE` + pure `require_attestation` + error variants; replace admin.rs:56-57; **rewrite the
  sub-2 `export_snapshot_refused_...` KAT** [I3]; the exact-phrase KATs (correct/missing/wrong/not-active) +
  out-dir-untouched + `attest_strings_contain_phrase` + the ★ fault-inject.
- **T2** — CLI `--attest` arg + main.rs-arm TTY prompt; the supersedes KAT.
- **T3** — btctax-tui viewer typed-word export modal (mirror safe-harbor-attest) + the 2 viewer KATs; re-run
  sub-2 output-cleanliness. `make docs`; whole-diff review + full suite + FOLLOWUPS (**program COMPLETE**).

## Gotchas
- **[C1] TWO paths** — CLI export-snapshot AND the btctax-tui viewer `e` export both gate; missing the viewer
  is the accidental-filing bypass.
- **[I2] pure helper** — no TTY read in `require_attestation` (env-dependent hang); prompt lives in the main.rs arm / TUI modal.
- **[I1] wrong phrase ⇒ `AttestationFailed`** regardless of TTY; only `None` ⇒ `AttestationRequired`.
- **Gate ONLY when pseudo-active**; check FIRST (out_dir untouched on refuse); exact/trimmed/case-sensitive phrase.
- **Output stays clean** — attestation permits; adds no markers (sub-2 guard holds).
