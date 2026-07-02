# Whole-Branch Review (Phase E, final gate) — `feat/tui-edit-chunk1`, Round 1

- **Branch reviewed:** `feat/tui-edit-chunk1` @ `7cdb69d` (4 commits over base `22cda75`).
- **Diff:** `.superpowers/sdd/review-22cda75..7cdb69d.diff` (331 KB — read in sections) + current
  source re-read file-by-file for every load-bearing claim.
- **Contract:** `design/SPEC_tui_edit_chunk1.md` (R0 GREEN, 2 rounds) +
  `reviews/R0-spec-tui-edit-chunk1-round-1.md`.
- **Task reports:** `.superpowers/sdd/tui-edit-t{1,2,3}-report.md`.
- **Reviewer role:** independent whole-branch reviewer (author ≠ reviewer). All empirical checks
  below were **re-run at THIS HEAD** by this reviewer, not taken from the reports.
- **Date:** 2026-07-02.

## Verdict

**READY TO MERGE — 0 Critical / 0 Important** (1 Minor, 4 Nit — none blocking; the Minor is a
five-minute test-strengthening follow-up).

Both guarantees hold, verified independently:

1. **The viewer stays write-free.** The lib split is a pure visibility refactor: the full viewer
   suite passes with pre-existing test content unchanged (75 → 76 tests; the +1 is the
   spec-required wrapper-consistency KAT); the E10 mechanized gate passes at HEAD (run
   explicitly); `export.rs` and `tabs/tests.rs` do not appear in the diff at all — the E10
   scanner, the guarantee wording, and the TestBackend suite are byte-untouched. No new
   write-capable pub surface exists (full pub-item sweep below).
2. **The editor's writes are confined and confirmed.** KAT-G1 (gate + self-check) green at HEAD;
   the independent grep layer confirms the mutation surface exists in exactly one non-test
   location (`edit/persist.rs:41–42`), the four vault-creating constructors appear nowhere in
   non-test code, and `persist_tax_profile` has exactly ONE non-test call site — the modal's
   Enter arm (`main.rs:165`). All three safety tests (KAT-P1, KAT-C1 + complement, KAT-S1
   **un-ignored**) pass and genuinely assert what the spec claims.

---

## 1. Empirical results (all re-run at `7cdb69d`)

| Check | Command / method | Result |
|---|---|---|
| Viewer full suite | `cargo test -p btctax-tui` | **76 passed / 0 failed / 0 ignored** |
| E10 gate explicitly | `cargo test -p btctax-tui --lib e10` | **`e10_mechanized_source_gate` ok** |
| Editor full suite | `cargo test -p btctax-tui-edit` | **49 passed / 0 failed / 0 ignored** (KAT-G1, KAT-P1, KAT-C1 + complement, KAT-S1, KAT-U1×3, lock-exclusivity, KAT-F1–F4, KAT-V1–V11, E2E tax-tab, dispatch regressions) |
| Core ordered read | `cargo test -p btctax-core load_all_ordered` | **2 passed** (empty-db; ordinal order) |
| Mutation tokens outside `edit/persist.rs`, non-test, comment-stripped (`conn(` / `save(` / `tax_profile::set` / `append_`) | awk pre-`#[cfg(test)]` extraction + grep | **zero hits** |
| Vault-creating constructors anywhere non-test (`Session::create` / `Session::repair` / `Vault::create` / `Vault::repair`) [R0-I1] | same | **zero hits** |
| `cmd::` non-test | same | **zero hits** |
| fs-write verbs non-test (14-token list) | same | **zero hits** |
| Grep pipeline non-vacuous | persist.rs non-test re-scanned | exactly the two sanctioned lines (persist.rs:41–42) detected — the extraction works |
| `persist_tax_profile` call sites | crate-wide grep | definition + doc mention + KAT-P1 test calls + **one** non-test call: `main.rs:165` (modal Enter arm) |
| Test-count reconciliation | 726 (post-T1 workspace) + 49 (editor) + 2 (core) | **= 777**, matching the reported full-gate count exactly |

The full gate (777 tests, both clippys, fmt, both PII scans) was reported green at this HEAD by
the Task-3 report and was NOT re-run wholesale here (per the review brief); the targeted re-runs
above independently confirm every load-bearing subset.

## 2. Guarantee (a): the viewer is still write-free — PASS

- **Pure visibility refactor, proven.** The viewer suite passes with zero content changes to
  pre-existing tests (`#[test]` count 75 → 76 across `crates/btctax-tui/src`; the +1 is
  `attempt_open_is_wrapper_consistent_with_open_session`, required by D1's acceptance).
  `main.rs` is now the spec's exact one-liner (`btctax_tui::run_viewer()`); the old `main` body,
  `handle_key`, scroll helpers, run loop, and its tests moved verbatim into `lib.rs` (diff
  inspected hunk-by-hunk; no logic edits found in any moved region).
- **E10 untouched and green.** `export.rs` is absent from the diff; the gate still walks
  `CARGO_MANIFEST_DIR/src` (so the new `lib.rs` is *inside* its coverage) and passes at HEAD.
  `export` is declared `pub(crate) mod` in lib.rs — not externally reachable.
- **No new write-capable pub surface.** Full sweep of externally-`pub` items in the lib:
  `app::{Screen, Tab, Snapshot}`, `unlock::{PASSPHRASE_CAP, UnlockState, OpenOutcome,
  SessionOpenOutcome, open_session, attempt_open, build_snapshot, latest_year}`,
  `draw::draw_unlock_screen`, six `tabs::*::render` fns, `restore_terminal` / `TerminalGuard` /
  `setup_panic_hook`, `run_viewer`. Every one is a read-only data type, renderer
  (`&Snapshot` + UI-only `&mut TableState`), or terminal-lifecycle helper. `App`,
  `draw::draw`, the tab `draw` wrappers, and `ExportConfirmState` are all `pub(crate)` (verified
  in source, matching the D1 "INTERNAL" row). `open_session` returns a `Session`, but that adds
  **no capability beyond the already-pub `btctax_cli::Session::open`** (btctax-cli exports
  `Session` publicly) — the seam only single-sources error mapping + snapshot building, exactly
  as R0 rated it.
- **[R0-M5] pinned ordering verified by inspection:** `open_session` (unlock.rs:118–136) calls
  `drop(pp)` immediately after `Session::open` succeeds and **before** `build_snapshot` —
  today's exact ordering, with the pin cited in both doc-comment and inline comment.
- **The viewer still never stores a Session:** `attempt_open` (unlock.rs:149–164) is a thin
  wrapper that explicitly `drop(session)`s on Success; `App::do_unlock` still calls
  `attempt_open`. The session binding inside `open_session` is immutable (`let session = …`),
  preserving the compile-level flavor of the old note. The tab `render` extractions consume
  exactly `snap`/`year`/`table_state` — the six wrapper `draw` fns keep the `snapshot == None`
  placeholder byte-identical, and `tabs/tests.rs` (untouched) keeps calling the wrappers.
- **Unlock rendering byte-identical:** `draw_unlock` delegates to the extracted
  `draw_unlock_screen` with the viewer's exact original title and note strings (diff-verified).

## 3. Guarantee (b): the editor's writes are confined and confirmed — PASS

- **Confinement (two independent layers, both green).** KAT-G1 (E10-clone: src-walk,
  first-`#[cfg(test)]` region split, `//`-stripping, file:line output, plant-a-token self-check
  with runtime-constructed tokens including `Session::create` per R0-I1) passes at HEAD. My own
  comment-stripped non-test greps (separate implementation from the gate) found: mutation
  tokens ONLY at persist.rs:41–42; zero vault-creating constructors; zero `cmd::`; zero fs-write
  verbs. The editor performs no direct filesystem writes; the vault is written only inside
  `btctax-store` via `Vault::save`'s atomic path.
- **The modal is the only path to `persist_tax_profile`.** One non-test call site
  (main.rs:165), inside `handle_modal_key`'s Enter arm. Dispatch order is pinned
  modal → form → screen (main.rs:79–89); the modal swallows every key except Enter/Esc
  (`q` swallowed — asserted in three tests); Esc closes the modal only and writes nothing.
  The wrapper name is not a banned token — confinement is of the raw surface, as designed and
  as R0 round 2 confirmed.
- **[R0-M1] failed-save semantics implemented exactly as pinned:** on `Err`, modal closes, form
  + buffers stay, `status = "Save error: {e}"`, **no re-projection, no side-table rollback**
  (main.rs:192–197; persist.rs doc-comment states the divergence). KAT-S1 tests the whole path
  including the idempotent retry.
- **Guarantee wording** is present verbatim in every module doc (main.rs, editor.rs,
  draw_edit.rs, edit/mod.rs, edit/persist.rs, form.rs) and already covers chunk 2's
  `append_decision` so it never needs rewording.

## 4. The three safety tests — all run, all genuinely assert the spec's claims

- **KAT-P1** (`edit/persist.rs::kat_p1_append_only_prefix_side_table_form`) — **PASS, non-vacuous.**
  Seeds 2 real decision events via `append_decision` + save (so log-unchanged is not
  empty-vs-empty); captures `pre` via `load_all_ordered`; asserts `post == pre` **in-memory**
  after `persist_tax_profile`, then **drops + reopens** and asserts the persisted image agrees;
  the round-trip guard (`session.tax_profile(2025) == fixture`) proves the mutation executed;
  a second **differing** upsert still leaves the log `== pre` while the read-back updates
  (upsert-not-append proven). `RawEventRow` carries **all 11 columns including `ordinal`**
  (persistence.rs:334–347 — the [N6] residual was adopted, strictly stronger than the R0-M2
  minimum), `ORDER BY ordinal`, read-only, not a projection input.
- **KAT-C1** (`main.rs::kat_c1_cancel_path_vault_bytes_unchanged`) — **PASS.** Drives the REAL
  `handle_key` end-to-end (`p` → type into buffers → Enter → modal → `q`-swallowed assert →
  Esc → form-still-open + no-status asserts → Esc → `q`), drops the session, and asserts the
  vault file **byte-identical**. The complement test proves a confirmed mutation DOES change
  the bytes (no trivially-green cancel test).
- **KAT-S1** (`main.rs::kat_s1_save_error_path_chmod_parent`) — **PASS, UN-IGNORED** (the
  pre-recorded `#[ignore]` fallback was not needed; 0 ignored in the suite). chmod-0o500-parent
  forces `atomic_write`'s `.tmp` creation to fail after a real key-driven confirm. All four
  Err-arm claims individually asserted: (1) modal closed, (2) form open with buffer content
  `"120000"` intact, (3) status contains `"Save error"`, (4) on-disk vault byte-identical.
  Perms restored → re-confirm → retry succeeds, profile round-trips, and the event log is
  re-asserted unchanged. The root-skip guard probes the actual denial (writes a probe file
  under 0o500) rather than checking uid, exactly as spec'd.

## 5. Validation parity, form, modal — PASS

- **Rules spot-checked 4-for-4 against main.rs:688–760** (and the other six read in passing —
  all ten line up): required-empty (`ordinary-taxable-income is required` ↔ main.rs:693–700);
  optional-default-0 (`carryforward_short` empty → `Usd::ZERO` ↔ main.rs:723–727, **no
  negativity check** — the parity pin); negative-reject (`w2-ss-wages` `is_sign_negative()` ↔
  main.rs:733–740); **KAT-V11 whitespace-only** — `is_empty()` = byte-len-0 checked BEFORE
  trimming (form.rs:54–57), so `"  "` takes the parse path and errors for BOTH optional and
  required fields (both KAT-V11 cases pass), matching `parse_usd_arg` exactly. Parse is
  `Usd::from_str(buf.trim())` where `Usd = Decimal` (conventions.rs:8) — identical semantics to
  `eventref::parse_usd_arg`'s `Decimal::from_str(s.trim())`. KAT-V8b pins negatives ACCEPTED
  for fields 2–7 (no invented rules). Construction mirrors main.rs:762–775 field-for-field.
- **FilingStatus cycling:** 5-variant declaration-order cycle (form.rs:121–129), KAT-V1 proves
  5 presses return to start and all variants reachable; Tab cycles on row 0, moves focus
  otherwise, and **never inserts text** (main.rs:239–249).
- **Pre-population:** `snapshot.profiles.get(&year)` → every buffer gets the field's `Decimal`
  `Display` string + stored filing status (main.rs:289–316); KAT-F1 asserts all 9 buffers + the
  status. `p` is a no-op when `snapshot.is_none()`.
- **Modal content:** `draw_mutation_modal` renders the year + all 10 leaf fields from the
  **validated** `TaxProfile` (format-arg mapping inspected — each `{arg}` maps to the right
  field), `filing_status` via the CLI's own `render::filing_status_tag`, plus the upsert note,
  atomic-path note, and the Enter/Esc legend. Single-spaced: 16 content lines + border = 18
  rows ≤ 24; longest line ~53 chars < the 62-char inner width. KAT-F2 renders at a **real
  80×24 TestBackend** and asserts the year, the fs tag, all 10 field NAMES, "WRITES THE VAULT",
  and "writes nothing" (the last content line — so bottom-clipping is genuinely excluded).
  See **[M1]** below for the one assertion gap (money VALUES not asserted).
- **[R0-N5] status semantics:** `app.status = None` lives ONLY in the Browse arm
  (main.rs:113–116); modal and form dispatch return before it — the modal's own Enter can never
  wipe the status it just set; the next Browse key clears it (mirrors the viewer's
  `export_status`).

## 6. Lifecycle — PASS

- **The live mut Session:** `EditorApp.session: Option<Session>` stored (unboxed) at unlock and
  held for the whole TUI session; `Some` iff `snapshot` is `Some`; mutably borrowed only inside
  the modal Enter arm's persist block. The `cmd::*` open/drop-per-call fns are bypassed (and
  gate-banned).
- **VaultLock exclusivity:** `lock_exclusivity_editor_session_blocks_concurrent_open` proves a
  held editor session makes a second `open_session` return `Locked`; KAT-U1's third case proves
  the converse (editor shows Locked when another holder exists). The concurrency story is
  documented at module, struct, and field level in editor.rs and in persist.rs.
- **Save-per-action, in order:** persist (`tax_profile::set` → `session.save()`) → on `Ok`
  re-project via `build_snapshot(&session)` → status; on `Err` **no re-projection**
  (main.rs:157–198). The re-projection-failed sibling path keeps the old snapshot and sets a
  restart-advising status (the save is already on disk). E2E test proves the re-projected
  snapshot actually feeds `compute_tax_year` (Tax tab flips from NOT COMPUTABLE to computed).
- **Early `drop(pp)`** in `open_session` — verified (§2). Unlock hygiene: `mem::take` →
  `Passphrase::new`, never cloned, masked rendering via the shared `draw_unlock_screen`.
- **80×24 modal fit** — KAT-F2 pins it (§5).
- **EDITOR markers:** unlock title + note line, tab-bar `[EDITOR]` badge title, footer badge —
  all present and TestBackend-asserted (unlock-screen and browse-screen marker tests).

## 7. Workspace hygiene / cross-cutting — PASS

- New member wired (workspace Cargo.toml members list; Cargo.lock updated).
- Explicit dep pins match the viewer exactly (ratatui 0.29 / crossterm 0.28 / rust_decimal 1 /
  time 0.3); `edition`/`rust-version`/`license` workspace-inherited; `tempfile` +
  `rust_decimal_macros` dev-only. No `[workspace.dependencies]` table introduced.
- MSRV: workspace `rust-version = "1.88"` unchanged; stable toolchain (1.95.0) proxies —
  both clippys reported clean at this HEAD by the task gate.
- **Other lanes' property untouched:** the diff contains NO changes to `btctax-cli`,
  `btctax-store`, `btctax-adapters`, `export.rs`, `render_schedule_se`, or any CSV writer —
  only workspace manifests, `btctax-core/persistence.rs` (the additive read-only fn),
  `btctax-tui` (the split), the new crate, and design/review docs.
- Determinism / synthetic-only: all fixtures are synthetic (temp vaults, `kat-*-pass`
  passphrases, fixture Decimals); exact `Decimal` throughout (no float); PII scans reported
  clean at HEAD.
- Scanner conventions held by construction: `#[cfg(test)]` is single-and-last in every editor
  module (persist.rs's three later textual matches are string literals *inside* its test
  region — after the real region split at line 48; harmless); no `//` inside non-test string
  literals.

---

## Findings

### Critical

None.

### Important

None.

### Minor

- **[M1] KAT-F2 asserts the 10 field NAMES but not the 9 money VALUES.** Spec D5: "the
  rendered modal buffer contains the year and all 10 leaf field names **with the validated
  values**." The test asserts `2025`, the `mfj` tag, all field names, and both warning strings —
  but none of the nine distinct fixture amounts (120000 / 130000 / 5000 / 1000 / 500 / 250 /
  80000 / 85000 / 3000, chosen distinct precisely so swaps are detectable). A value-swap or
  value-omission rendering regression would pass. Exposure is confirmation-display exactness
  only — what is *persisted* is the same validated `TaxProfile` struct held in
  `MutationModalState`, i.e. the user's own validated form input, so no unconfirmed foreign
  data can be written; the current format-arg mapping is verified correct by inspection.
  **Fix:** add nine `rendered.contains(…)` value asserts to KAT-F2 (five-minute change).
  Non-blocking; record as a FOLLOWUP if not folded before merge.

### Nit

- **[N1] Task-1 report drift (report only, not code):** its "authoritative" D1 surface listing
  says `Snapshot { …, profiles: Vec<TaxProfile>, … }` (actual: `BTreeMap<i32, TaxProfile>` +
  a `tables: BundledTaxTables` field it omits) and `Screen { …, Main }` (actual: `Viewer`).
  The code matches the spec; the report does not. Harmless now that this review re-verified
  the real surface, but worth knowing the t1 report's listing is not citable.
- **[N2] `cmd::` test-region wording tension (spec-internal):** D3 calls `cmd::init::run` "the
  sole exception" in test code, while D5's KAT-F4 itself mandates `cmd::tax::show_profile` in a
  test. The implemented gate follows the viewer's E10 structure exactly (`cmd::` unscanned in
  test regions), which R0 blessed; the tension is in the spec's phrasing, not the code.
- **[N3] `btctax-tui-edit/Cargo.toml` reuses the viewer spec's `[R0-M1]` tag** for the
  explicit-pins comment; in THIS spec's numbering R0-M1 is the failed-save pin. Cosmetic
  cross-spec tag collision.
- **[N4] `try_env_passphrase` duplicates `do_unlock`'s outcome-application match** (the viewer
  factors this as `apply_open_outcome`). Cosmetic duplication; candidate for chunk 2 cleanup.

---

## Gate disposition

**GREEN — 0 Critical / 0 Important. Ready to merge.** [M1] and the nits are non-blocking;
fold [M1]'s nine-assert strengthening either pre-merge or as a recorded FOLLOWUP entry
(do not drop it silently).
