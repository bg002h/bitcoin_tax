# SPEC — attestation export gate (sub-project 3 of auto-pseudo-reconcile — FINAL)

**Source baseline:** `main` @ `afb0807` (branch `feat/attest-export-gate`). **Review status: DRAFT — awaiting R0
(2 rounds to 0C/0I).** Design of record: `design/BRAINSTORM_auto_pseudo_reconcile.md`; roadmap memory
`auto-pseudo-reconcile-roadmap`. **Cross-cutting decisions settled — do NOT re-brainstorm.** Sub-projects 1
(per-exchange method election, `514875b`) + 2 (pseudo-reconcile mode, `afb0807`) are SHIPPED.

## Goal
Replace sub-2's **interim [I3] export refusal** with the settled **typed-attestation gate**: producing
`export-snapshot` output (the snapshot SQLite + projection CSVs + the tax-year Form 8949 / Schedule D / 8283 /
Schedule SE forms) while the ledger is **pseudo-active** requires the user to type the exact phrase
**`I attest this is true`**. A ledger that is NOT pseudo-active exports with NO prompt (today's behavior — the
user has already attested via their real decisions). This is the deliberate, friction-ful acknowledgment that
lets a pseudo-reconciled draft be exported ON PURPOSE while making an accidental filing impossible.

## Settled decisions (from the brainstorm)
- **Trigger = pseudo-active ONLY** (`state.pseudo_active()`, state.rs:268 = `pseudo_synthetic_count > 0`). A
  fully-real ledger is never gated.
- **Exact phrase** `I attest this is true` — a `const ATTEST_PHRASE`, compared TRIMMED, case-SENSITIVE, exact
  (a near-miss or extra text is rejected).
- **Scope = `export-snapshot` only** — it is the sole path that writes data/form FILES. `report --tax-year` is
  on-screen (already `[PSEUDO]`-flagged by sub-2, never a file) — NOT gated.
- **Output stays clean** — sub-2's guard is unchanged: the written files carry NO `[PSEUDO]` marker; the
  attestation merely PERMITS the export. (The user attesting owns the numbers — README disclaimer alignment.)
- Not persisted — a one-shot per-invocation command gate; no event/side-table.

## Mechanism
- `cmd::admin::export_snapshot` (admin.rs) — REPLACE the `if state.pseudo_active() { return
  Err(PseudoActiveExport(..)) }` block (admin.rs:56-57) with: `if state.pseudo_active() {
  require_attestation(attest)?; }`. Still checked FIRST (before any bytes are written — a rejected attestation
  leaves `out_dir` untouched).
- `require_attestation(attest: Option<&str>) -> Result<(), CliError>`:
  - if `attest.map(str::trim) == Some(ATTEST_PHRASE)` → Ok.
  - else if stdin is a TTY → PROMPT (`"This snapshot includes pseudo-reconciled placeholder values, NOT real
    tax data. To export it, type exactly: I attest this is true\n> "`), read one line, trim, exact-compare →
    Ok / `Err(CliError::AttestationFailed)`.
  - else (non-interactive, no/wrong `--attest`) → `Err(CliError::AttestationRequired)` naming the phrase.
- Error variants: replace `PseudoActiveExport(usize)` with `AttestationRequired` (non-interactive, phrase
  missing) + `AttestationFailed` (phrase typed but wrong) — both name the exact phrase + that the state is
  pseudo-reconciled. main() maps to exit 2 (error), stderr.

## CLI
`btctax export-snapshot --out <dir> [--tax-year <Y>] [--attest "I attest this is true"]`. The `--attest` arg
(`Option<String>`) is the non-interactive path (scripts/tests); absent + pseudo-active + TTY → interactive
prompt; absent + pseudo-active + non-TTY → `AttestationRequired`. Threaded to `export_snapshot`.

## KATs (btctax-cli tests)
- `export_pseudo_active_with_correct_attest_writes_files` — pseudo-active + `--attest "I attest this is true"`
  → export succeeds, files present.
- `export_pseudo_active_missing_attest_refused_out_dir_untouched` — pseudo-active, no `--attest`, non-TTY →
  `AttestationRequired`; `out_dir` has NO files (checked FIRST).
- `export_pseudo_active_wrong_phrase_refused` — `--attest "i attest this is true"` / `--attest "I attest this
  is true!!"` / trailing-junk → `AttestationFailed` (exact, trimmed, case-sensitive).
- `export_not_pseudo_active_needs_no_attest` — a fully-real ledger exports with NO `--attest` (byte-identical
  to today).
- `attest_gate_supersedes_interim_i3_refusal` — the old `PseudoActiveExport` unconditional refusal is gone; a
  correct attestation now PERMITS the export (the [I3] behavior change).
- **[★ fault-inject target]** the phrase check is load-bearing — breaking the exact-compare (accept any string)
  ⇒ `export_pseudo_active_wrong_phrase_refused` goes RED.
- Output cleanliness still holds (sub-2's `pseudo_marker_...absent_from_every_export_file` — re-run, still green).

## Scope / SemVer / lockstep
btctax-cli only (admin.rs export path + CLI arg + error variants). No core change. Behavior change: pseudo-active
export goes from unconditional-refusal → attestable (MINOR — a new capability + a new flag). Lockstep: `make
docs` (regen `btctax-export-snapshot.1` — its inline `--help` gains `--attest` + the gate note), doc-comments.
No GUI schema_mirror (no GUI crate). Update the sub-2 FOLLOWUPS "interim [I3]" note to "replaced by sub-3".

## Plan (TDD)
- **T1** — `ATTEST_PHRASE` const + `require_attestation` + error variants; replace the admin.rs:56-57 [I3] block;
  the exact-phrase KATs (correct/missing/wrong) + out-dir-untouched + not-pseudo-active-no-attest.
- **T2** — CLI `--attest` arg threaded; the supersedes-[I3] KAT; the ★ fault-inject; re-run sub-2's
  output-cleanliness KAT. `make docs`; whole-diff review + full suite + FOLLOWUPS (program COMPLETE).

## Gotchas
- **Gate ONLY when pseudo-active** — a fully-real export must stay prompt-free + byte-identical (KAT).
- **Check FIRST, before any bytes** — a rejected attestation leaves `out_dir` untouched (mirror the [I3] order).
- **Exact phrase** — trimmed, case-sensitive; a near-miss is rejected (fault-inject the compare).
- **Output stays clean** — the attestation PERMITS export; it does NOT add markers to the files (sub-2 guard holds).
- **Non-interactive safety** — no TTY + no/wrong `--attest` ⇒ refuse (never silently export a pseudo draft in a script).
