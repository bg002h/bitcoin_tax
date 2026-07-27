# Stale-Snapshot Latch — FOLLOWUPS

Phase-owned follow-ups discovered during Tasks 1-8's build/review cycles and the whole-branch fix wave
that closed the branch out. Burned down on the **owning phase's** schedule (STANDARD_WORKFLOW
"per-phase, by ownership"), not all-at-end. Reconciliation is a grep: on entering a phase, sweep this
file for that phase's items. Live per-task ledger state lives in the (gitignored) SDD ledger
`.superpowers/sdd/IMPLEMENTATION_PLAN/progress.md`.

This registry did not exist while Tasks 1-8 ran (MF-1 in the whole-branch fix-wave brief) — it is filed
retroactively, seeded from `progress.md`'s inline "minor (deferred)" notes plus the fix-wave brief's own
follow-up list. Line numbers below were verified against source at HEAD `d816d23` (immediately after the
fix wave's two blocking-Important commits); they will drift like any other citation — re-verify before
acting on an item, per this repo's own "stale self-citations" lesson (see the doc-precision entry below,
which is exactly that lesson recurring on this branch).

Legend: **[open]** not started · **[closed]** burned down (kept for provenance) · owning phase in **bold**.

> ## ⚠ SUPERSEDED 2026-07-27 — this registry is HISTORICAL
>
> The live registry is now **`FOLLOWUPS.md` §"APPROACH-B ARCHITECTURE DECISION + POST-WIZARD
> RECONCILIATION"** (repo root). Read it first; do not burn items down from this file.
>
> Why: **the latch subsystem this registry documents is superseded by `SnapshotGen`** — a typed
> generation pinned to every plan, refusing commit across a generation bump — under both paths of the
> open architecture decision, and it is already deleted on `arch/engine-keep-wizard-cut`. So the 10
> guard/latch items below (guard (b)'s function-scope presence test, `NESTED_EXEMPT_OPENERS`'s
> unasserted parent tuple, the column-anchored `#[cfg(test)]` detection, the unconfined
> `stale_after_write` clear sites, `ALL_25_OPENER_KEYS`/`KEYMAP-SYNC` desync, the write-tail-prefix
> citation drifts, the park-ordering substring assertion, and the T7 armed-dashboard clipping
> residual) are **pruned as superseded**, not deferred.
>
> **What did NOT get pruned, and was promoted to the root registry** — all of it general-editor code
> that predates Approach B and survives every branch outcome: `flush_tax_inputs_draft`'s silent
> residue refusal, `handle_tax_inputs_key`'s `session.as_mut().unwrap()` panic path, the
> pseudo-approve shared fixture's missing write assertion, the **`BTCTAX_PRICE_CACHE` cross-test
> race** (8 unsynchronized sites), `corrupt_cli_config`'s wrong doc + bare-`INSERT` panic hazard, and
> the Browse status band's measure/render newline mismatch. The cross-cutting lesson — stop
> hand-citing self-referential line numbers in doc comments — was promoted too.

## Closed at ship

- **[closed] DESIGN.md §4 / tax-r2-m-4 — pre-existing stale-derived `Acknowledgment`s.** Filed at the
  Non-Goals gate: "an existing vault may already hold a promote whose `Acknowledgment`/`filed_basis` was
  recorded off a stale image; nothing detects or surfaces it." Closed at ship: **no vault predates the
  latch** (`no-users-yet` — btctax has never had a user; v0.2.0-v0.10.0 shipped with no adopted vaults),
  and the two payload-construction guards this branch adds — the `stale_reason()` probes at
  `declare_flow_confirm` (`main.rs:4365`) and `promote_flow_review` (`main.rs:4582`) — prevent any NEW
  stale-derived `Acknowledgment`/`PromotePlan`/`DeclareTranche` from being recorded going forward.
  Exposure is nil. (Owner: **ship** — DONE. Pointer added at `DESIGN.md:392`.)

## T7 residual (open)

- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2] Owner: next cycle.** An armed dashboard with ≥~6 declare candidates plus the real composed arm
  status still clips `[x] export` off the non-scrolling `Paragraph` at 80×24 (18 rows needed + 6-row
  notice > 22 available) — inherent CONTENT OVERFLOW, not the presence-driven over-reservation Task 7
  closed, so content-sizing alone cannot reach it; needs a pinned or scrolling export row. Pre-existing
  and worse before this branch (unarmed main clipped the same line at 9 candidates with no status);
  strictly improved by Task 7's fix (cliff moved 5→6). Golden:
  `docs/examples-tui/btctax-tui-edit-defensive-filing-stale-armed-no-status.txt`.
  Related, same screen: `main.rs:493-521` (`EditorScreen::DefensiveFiling`'s key-dispatch match arm) has
  no `?` (help) arm and this screen has no footer keybinding row of its own (Browse's footer doesn't
  carry over) — a filer landing here cold has no in-screen way to see what the keys do.

## Doc-precision batch (open unless marked closed by Step 7 of the fix wave)

- **[closed — fix wave Step 7]** `draw_edit.rs`'s `STALE_MARKER_DEFENSIVE` said "figures **below**" but
  renders in the BOTTOM band (golden `...stale-armed-no-status.txt` rows 21-22) — Browse's own wording
  ("figures below" is correct there) doesn't transfer. Reworded to "the figures on this screen".
- **[closed — fix wave Step 7]** `main.rs:736-737`'s doc parenthetical cited a deleted
  `#[allow(dead_code)]` attribute (T1's `stale_reason` was the last one; gone since Task 5). Dropped.
- **[closed — fix wave Step 7]** `draw_edit.rs`'s PSEUDO-banner citation pointed at "`:214-217` below";
  the banner is actually further down (drifted across Task 7's two fix rounds). Corrected to point at the
  banner's current location.
- **[closed — fix wave Step 7]** `DESIGN.md:353` said "the 12 sites" where its own enumeration lists 15.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2] Owner: next cycle.** Three more self-citation drifts in the same class, NOT touched by Step 7
  (verified against HEAD `d816d23`): the three write-tail-prefix KATs' doc comments cite the wrong line
  for their own production prefix — `commit_tax_inputs`'s prefix is actually at `main.rs:1655` (comments
  say `:1598`), `confirm_park_to_profile`'s prefix is at `:1837` (comments say `:1780`), the
  safe-harbor-attest prefix is at `:7191` (comments say `:7088`). A fourth comment
  (`confirm_park_to_profile_after_write_runs_after_dirty_is_cleared`'s doc) describes "the block that
  clears `form.dirty` (`:1758-1763` precede `:1777`)" — the actual dirty-clearing block is now at
  `:1825-1830` and the `after_write` call at `:1836`. None of these are behavioral; all are pure
  citation rot, the exact recurring class this entry itself is an instance of. Recommend: stop
  hand-citing self-referential line numbers in doc comments for values that can drift (name the
  function/const instead where the citation isn't load-bearing to a specific line), or add a doc-lint
  that flags `` `:\d+` `` citations for staleness at merge time.
- **[promoted — LIVE in root FOLLOWUPS.md G-1] Owner: next cycle.** `draw_edit.rs`'s Browse status band MEASURES `Line::from(String)`
  (`:334`) but RENDERS `Paragraph::new(String)` (`:418`) — `Line::from` is newline-blind while
  `Paragraph::new(String)` newline-splits into `Text`, so a `\n`-bearing status would measure 1 row but
  need 2+. Unreachable today (Task 7's reviewer checked all ~110 `status = Some(...)` sites; none embeds
  `\n`), so the failure mode if it ever became reachable would be a truncated band, never a clipped
  export line (that class is Task 7's own residual above). Fix by sharing one `Vec<Line>` between the
  measuring and rendering pass, mirroring how `draw_defensive_filing`'s own notice already does it
  (`draw_edit.rs:119-135` builds `notice_lines` once, used by both the measure and the render below it).
- **[promoted — LIVE in root FOLLOWUPS.md G-1] Owner: next cycle.** `flush_tax_inputs_draft`'s residue refusal (`main.rs:1264-1278`, guard at
  `:1265-1267`) returns `None` with no status set and without clearing `dirty`, so
  `handle_tax_inputs_key`'s `q`/`Esc` arms (`:1407-1434`) could swallow the keystroke forever under a
  latch whose OWN status message says "Quit the editor NOW" — the flow would stay open, showing no error,
  looking like a dead key. Unreachable via the one-flow invariant (a live, dirty `tax_inputs_form` cannot
  coexist with either save-forbidding latch in production — see guard (c)'s own doc) — which is exactly
  what the check exists to survive if that invariant is ever weakened. Also update the "Returns" contract
  doc (`main.rs:1240-1243`) to name this silent-refusal case explicitly.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2] Owner: next cycle.** `PSEUDO_ACTIVE_DASHBOARD_NOTICE`'s first remedy, "'P' from Browse"
  (`defensive_dashboard.rs:65-70`), dead-ends in the ARMED instance that is its primary producer — while
  armed, `open_pseudo_approve_flow`'s own combined-latch check (`main.rs:8293-8296`) refuses `P` outright.
  The chain still leads out (the stale refusal is itself actionable: quit and reopen), so this is a
  wording gap, not a dead end in practice, but the notice text should say so.
- **[promoted — LIVE in root FOLLOWUPS.md G-1] Owner: next cycle.** `corrupt_cli_config`'s doc (`main.rs:30998-31002`) is wrong on two
  counts: (1) it implies the corrupted key (`pseudo_reconcile`) matters because the three write paths
  under test don't read it — but `read_config` rejects ANY recognized key with an unparseable value, so
  the key choice buys nothing; (2) it says the subsequent failure is in `session.config()`, but the
  failure is actually in `load_events_and_project`. Separately, its INSERT (`main.rs:31009`) is a bare
  `INSERT INTO cli_config(key, value) VALUES (...)` against a table with a `key` PRIMARY KEY — it will
  panic if a future fixture pre-sets `pseudo_reconcile` before calling it. The sibling at
  `main.rs:31408` already had to use `UPDATE ... WHERE key = ...` for exactly this reason; `corrupt_cli_config`
  should either match that pattern or gain `ON CONFLICT(key) DO UPDATE`.

## Test-surface gaps (open)

- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2] Owner: next cycle.** Six success strings are asserted by no test anywhere in the suite; they
  are provably byte-identical to what shipped (confirmed by literal diff during Task 4's review), but by
  diff, not by the suite — a future edit could silently change one with nothing going red.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2] Owner: next cycle.** The park-ordering guard's `!contains("entry screen")` assertion
  (`main.rs:31159`, inside `confirm_park_to_profile_after_write_runs_after_dirty_is_cleared`) is lethal
  only for TODAY's fixture/wording; it pins the absence of one substring rather than the composed status
  as a whole, so a future fact-2 reword could reintroduce the defect this KAT exists to catch while the
  substring-absence check keeps passing on the new wording.
- **[promoted — LIVE in root FOLLOWUPS.md G-1] Owner: next cycle.** `approve_all_pseudo_defaults_then_fail_reprojection` (`main.rs:31384`) is
  a shared TEST FIXTURE (used by multiple KATs to reach "approved, then re-projection failed"), not
  itself a write-regression guard — nothing asserts, on its own, that the approval write actually landed
  before the induced failure. Each caller currently re-derives that assertion for its own purposes; worth
  a single shared assertion inside the fixture so a future caller can't skip it.
- **[promoted — LIVE in root FOLLOWUPS.md G-1] Owner: next cycle.** `handle_tax_inputs_key`'s dispatch can still panic via
  `session.as_mut().unwrap()` (`main.rs:1278`, inside `flush_tax_inputs_draft`) if `app.tax_inputs_form`
  is ever `Some` while `app.session` is `None` — a combination the one-flow/one-session invariant rules
  out today but that nothing at this call site itself defends against structurally (the same
  "convention, not construction" class this repo's `answered-ness invariant` memory names).
- **[promoted — LIVE in root FOLLOWUPS.md G-1] Owner: next cycle.** Task-4 triage item (8) (`.superpowers/sdd/IMPLEMENTATION_PLAN/progress.md`),
  never registered here: commit `9796013` had added a doc note warning that pre-existing tests mutate the
  same process-global `BTCTAX_PRICE_CACHE` env var outside any lock (a cross-test-file race hazard);
  `8f84326` deleted that note wholesale with no FOLLOWUPS entry to replace it. The hazard itself is still
  live: 8 `std::env::set_var("BTCTAX_PRICE_CACHE", ...)` sites survive today at `main.rs:17969-18284`
  (re-verified against current source — progress.md's own citation, `:16702-17021`, has already drifted),
  none serialized against each other beyond whatever incidental ordering the test runner happens to give.

## Guard hardening (open, `next cycle` unless noted)

- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2]** Guard (b) direction 2 (the scanner that checks every opener calls the combined latch check)
  is a FUNCTION-SCOPE presence test: `let _ = app.stale_or_residue_latch_status();` sitting anywhere in
  an opener's body would satisfy the scanner without the return value ever gating anything. The scanner
  (`main.rs`, the region building `main_src`/`editor_src` around `:17505-17613`) greps for the call
  syntax, not for control flow consuming its result.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2]** `NESTED_EXEMPT_OPENERS`'s (`main.rs:17443`) parent-surface tuple element (the second `&str`
  in each `(&str, &str)` pair) is never asserted anywhere — only the first element (the opener name) is
  matched (`main.rs:17566`). If the parent-surface names drift from the real nesting, nothing catches it.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2]** Both scanners (guard (a)'s dispatch-subset check and guard (b)'s latch-call check) cover
  only `main.rs` + `editor.rs` (`main.rs:17505-17613`). A production `#[cfg(test)]` block that is
  INDENTED (rather than starting at column 0) would silently widen what `production_lines` (`main.rs:17191`)
  treats as production code, since the scanner's test-region detection is column-anchored.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2]** `stale_after_write` has exactly two `= None` assignment sites (`main.rs:931` inside
  `apply_reprojection`'s `Ok` arm, `main.rs:4853` inside `execute_defensive_export`'s post-rebuild path)
  with nothing structurally confining the count to two — a third clear site added elsewhere (bypassing
  D-4's documented single-clear-per-path discipline) would not be flagged by any scanner.
  `ALL_25_OPENER_KEYS` (`main.rs:22955`) is similarly hand-maintained with no tie to the `KEYMAP-SYNC`
  region (`main.rs:415-483`) it is supposed to mirror — a future keymap change would silently desync the
  two.
- **[pruned — superseded 2026-07-27, see root FOLLOWUPS.md G-2]** Note for whoever next touches guard (a): it fails **closed** on the `.as_mut()` dispatch
  variant (i.e., an unhandled dispatch shape reds the guard rather than silently passing) — this is
  intentional and load-bearing; do not "simplify" `close_all_mutation_surfaces`'s scan or the guard's own
  matcher to make that red go away without checking why it's red first.

## When triggered

- **[deferred, no owning phase yet]** Add `#[must_use]` to `close_all_mutation_surfaces` once a third
  production caller appears (still exactly 2 today: `on_persist_error`'s `ResidueLive` arm at `:775` and
  `arm_stale` at `:1038` — see the corrected mutation record on
  `residue_live_may_save_false_prevents_dirty_tax_inputs_draft_from_reaching_disk`). Not worth the
  annotation churn for 2 callers, both of which already use the return value.
