# Whole-branch review — tui-edit-save-rollback (Phase E, round 1)

**Branch:** `feat/tui-edit-save-rollback` @ `cc9fa4a` (diff `main..HEAD`, main == `45b9332`; 3 commits:
spec `c76652c`, Task 1 `33d0ed6`, Task 2+3 `cc9fa4a`).
**Spec:** `design/SPEC_tui_edit_save_rollback.md` (R0-GREEN, 2 rounds).
**Controller-verified full gate at HEAD:** 876 workspace tests, clippy `-D warnings` clean, fmt clean.

## Controller fold disposition
- **[M1] Minor** — `persist_void` post-append `clear(...)?` residue window → **FOLDED** (extracted a
  shared `rollback(session, pre, err)` helper; `persist_void`'s `clear` failure now returns
  `RolledBack`/`ResidueLive` instead of `NoChange`-without-rollback). Re-review below: GREEN.
- **[N1] Nit** — 3 silent headers lack the optional self-doc note → **no action** (module header
  documents the invariant; reviewer said no action required).

## Round 2 — M1 fold verification (independent) — GREEN

Re-reviewed by the same reviewer against the folded `edit/persist.rs`:
- Borrow/move of `pre` sound (no double-move; `&pre` borrow ends at the early `return`, `pre` moved
  into `save_or_rollback` only on the success path).
- `clear` failure now yields `RolledBack`/`ResidueLive` (not `NoChange`) — the residue window is
  closed; `NoChange`'s "no residue" contract now holds for `persist_void`.
- All 8 persist fns scanned: only `persist_void` had a fallible step AFTER a committed mutation; the
  other 7 leave nothing committed on failure, so `NoChange`-without-rollback is correct there.
- The void-rollback KAT still exercises the save-failure path; the shared `rollback` helper is used
  by both `save_or_rollback` and `persist_void` (not dead); KAT-G1 unaffected.

**Verdict: whole-diff GREEN — 0 Critical / 0 Important.** Ship-eligible.

Controller gate at the fold: `cargo test -p btctax-tui-edit` 145 pass; clippy `-D warnings` clean;
full workspace 876 pass at `cc9fa4a` (the M1 fold touches only `edit/persist.rs`, revalidated below).

---

## Reviewer output (verbatim)

All 17 mutation-surface fields are cleared by `close_all_mutation_surfaces` — exact match, nothing left open on `ResidueLive`. I have everything I need.

# Whole-diff review — tui-edit-save-rollback (Phase E)

**Verdict: 0 Critical / 0 Important / 1 Minor / 1 Nit** — the diff delivers the spec's guarantee. Ship-eligible; the two findings are non-blocking.

I verified the whole diff (`main..HEAD`, 3 commits) against the R0-GREEN spec, ran the 10 new/rewritten KATs (all green), and ran a fault-injection probe proving the new residue pins are load-bearing (restoring the tree byte-for-byte afterward — `git status` clean).

## What I confirmed sound

**1. The 4-KAT rewrite (highest-attention) — clean supersession, no silent drops.** For all four (`kat_s2_*classify_inbound` `main.rs:5079`, `kat_s2_ro_*reclassify_outflow` `:6588`, `kat_s2b_*set_fmv` `:8063`, `kat_s3a_*select_lots` `:9399`): every save-failure UX assertion is **preserved verbatim** — (1) modal closed, (2) form/flow still open with buffers intact, (3) status contains `"Save error"`, (4) byte-identical disk (`bytes_before == bytes_mid`). Each **adds** the `load_all_ordered == pre` residue pin (`:5169, :6690?, :8146, :9492`) and **inverts** retry from `pre+2`+`DecisionConflict` to `pre+1`+no-conflict. **Fault-injection probe:** neutering the `restore` in `save_or_rollback` (leaving residue) fails `kat_s2` at `main.rs:5169` and `kat_persist_void_*` at `persist.rs:1610`, exactly the new pins — so they are not coverage theatre. The genuine double-*success* tests `kat_e2e_fmv_repoint_*` (`:7645`) and `KAT-VOID-RETRY` (`:8696`) are outside every diff hunk — **correctly left alone**.

**2. Rollback correctness.** All 8 persist fns snapshot **before** the mutation (`persist_void` at `persist.rs:249`, before `load_all`/`append`/`clear`). Whole-DB restore reverts `persist_void`'s `optimize_attest::clear` side-table for free — `kat_persist_void_rollback_preserves_optimize_attest_on_failed_save` asserts the row survives, and I confirmed it fails when restore is neutered. `restore` is untouched-on-failure (`self.conn = db_from_bytes(image)?` — `?` before assignment). `decision_seq` reuse is correct by construction.

**3. ResidueLive/latch path — airtight.** `rollback_failed = true` appears in exactly **one** production site (`on_persist_error`, `main.rs:442`); `ResidueLive` is constructed in exactly one production site (`save_or_rollback`, `persist.rs:67`). **No `Display` impl** for `PersistError`. All **8** Enter arms delegate to `on_persist_error` (`:317,586,1041,1387,1454,1795,2645,2935`) — none formats inline. `residue_latch_status` reproduces the attest wording verbatim (`kat_e2e_attest_errlatch_chmod` green). All **9** openers guard on it. `close_all_mutation_surfaces` clears all **17** flow/modal fields (exact match to the struct).

**4. KAT-G1.** `"restore("` added to `persist_only_tokens` (`:854`) and the plant-a-token self-check (`:1025,:1055`). The only `restore(` substrings crate-wide are inside `edit/persist.rs`; `restore_terminal` (`main.rs:3955`, non-test) correctly does **not** match `restore(`. `snapshot(` left ungated (would false-positive on `build_snapshot`).

**5. Attest untouched.** `persist_safe_harbor_attest` still returns `CliError`, still `session.save()?` (no rollback), not in the diff. `attest_save_failed` set only by its own arm (`:3734`). Both `From<CliError>`/`From<CoreError>` target `NoChange`, never `ResidueLive`.

**6. Cross-crate.** store→cli→editor consistent; `StoreError`→`CliError` via existing `#[from]`. No dead code, no stale contradicting doc comments (the "do NOT roll back" tax-profile header was correctly rewritten).

### [M1] Minor — `persist_void`'s post-append `clear(...)?` can leave residue classified as `NoChange`
`crates/btctax-tui-edit/src/edit/persist.rs:270`

In `persist_void` the order after the snapshot is `append_decision(...)?` (`:261`, mutation) → `optimize_attest::clear(...)?` (`:270`, mutation) → `save_or_rollback` (`:273`). If `clear` returns `Err`, the `?` maps it `CliError → PersistError::NoChange` and the fn returns **before** `save_or_rollback` — while the `VoidDecisionEvent` append is already committed to the in-memory conn. `on_persist_error(NoChange)` then shows the benign "safe to retry" status, does **not** roll back, and does **not** latch, so a live void residue can piggy-back a later successful save — the exact hazard the cycle closes. This contradicts `NoChange`'s doc contract ("NOTHING was written and there is no residue").

**Why it does not block:** `clear` is a pure in-memory `DELETE` (`optimize_attest.rs:88`) over a handful of rows; it can only fail on OOM/corruption — the same astronomically-rare class as the `ResidueLive` restore-OOM. It is **pre-existing** (main's `persist_void` had the same append→clear→save ordering with *no* rollback at all, strictly worse), not introduced by this diff, and the spec scopes the mechanism explicitly to "when `session.save()` fails." Reachability is effectively nil.

**Fix (if you want the invariant fully airtight to match the spec's own paranoia about the restore-OOM):** treat any error after the append as a rollback trigger, e.g. handle the `clear` result rather than `?`-ing it — on `Err`, `session.restore(&pre)` and return `RolledBack`/`ResidueLive` per whether the restore succeeds. Otherwise, narrow the `NoChange` doc to note the post-append side-effect window.

### [N1] Nit — three "silent" persist headers didn't get the optional self-documenting note
`persist_classify_inbound` (`persist.rs:~106`), `persist_reclassify_income` (`~170`), `persist_set_fmv` (`~198`) still carry their pre-cycle headers with no "reverted on a failed save; retry is clean" line. The spec marked this **[R0-N1, optional]**, so this is purely cosmetic — the universal invariant is documented at the module header (`persist.rs:19-23`) and on the other five fns. No action required.
