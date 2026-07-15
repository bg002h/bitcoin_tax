# Whole-branch review — plan 2 (persistence), independent Fable pass r1

*Persisted VERBATIM (STANDARD_WORKFLOW §2). Reviewer: Fable. Persisted 2026-07-15 against HEAD `9b47873`
(base `32e6e38`, 7 task commits, ~774 insertions). The plan-2 whole-branch gate.*

---

VERDICT: 0 Critical / 3 Important

All three Important findings are seams a per-task lens structurally could not see: two live at the plan-2/plan-3 **crate boundary** (btctax-tui-edit consumes btctax-cli as a library dependency — `crates/btctax-tui-edit/Cargo.toml:19` — so `pub` vs `pub(crate)` and "who can render stderr" only become visible when you look at the whole branch's future caller), and one is the gap between I-11's letter ("a *year* without params") and its implementation ("a *call* without params").

## Strengths

- The §6.2 M-1 placement is disciplined and uniformly correct: all four writers call `coherence_clear_or_refuse` immediately after `Session::open`, before any committed-row read (`answer.rs:108`, `tax.rs:66/90` import/clear, `tax.rs:416` write-back — on **year+1**, the row it actually writes, which is the right target). Each call site carries a reasoned comment naming *why* the ordering matters (the absent-row early-returns in `answer`/write-back that would otherwise shadow the parked-refuse).
- Atomicity in `commit`/`park_to_profile`/`discard_parked_draft` is genuinely whole-set: `Vault::snapshot`/`restore` serialize/replace the *entire* DB image (`vault.rs:251-262`), so a failed save rolls back committed-row + draft mutations together; park is stash-FIRST; every refuse path returns before the snapshot, so refusals mutate nothing.
- The `income_clear_refuses_a_parked_draft_and_preserves_it` test pins the coherence call into a *real writer* (mutation-check b), not just the helper in isolation — the wiring itself has a test holding it.
- The two new `CliError` messages match the SPEC §6.2 M-d wording verbatim, and the `coherence_clears_wip_but_refuses_parked` test asserts both exits are named.
- Every error path in `write_back_carryover`/`answer` that follows a coherence WIP-delete returns before `s.save()`, so an aborted command never persists the in-memory draft delete — disk is untouched on every failure.

## Issues

### Important

**I-1 — `load`'s §6.3 stale-discard note is inexpressible to its only future callers, and `eprintln!` from a store read fn will garble the TUI.**
`input_form_store.rs:149-152`. SPEC §6.3 requires the stale-WIP discard be "discarded-with-note." The note is an `eprintln!` inside a library fn whose **only production callers will ever be renderers** (`load` has zero CLI callers; plan 3's TUI — a raw-mode, alternate-screen `btctax-tui-edit` per §9A — is the consumer). Failure scenario: user resumes a year after an upgrade → TUI calls `load` mid-session → the note is written to a covered/raw-mode stderr (invisible at best, screen-corrupting at worst) → the form silently opens on the Committed/Fresh state and the user never learns their WIP was discarded. Plan 3 cannot even reconstruct the note: `Loaded` carries no discard signal, and `DraftRow`/`get_draft_row` are `pub(crate)` — invisible cross-crate — so the found/expected versions are unreachable. This is a plan-3-blocking gap: the API cannot express a spec-required state.
*Fix:* carry the fact in the return value (e.g. `Loaded::Committed { ri, discarded_stale_draft: Option<StaleNote> }`, or return `(Loaded, Option<StaleNote>)`) and move the printing to call sites. (`coherence_clear_or_refuse`'s eprintln at :192 is fine *today* — its callers are CLI commands — but note it inherits the same problem if plan 3 ever calls it.)

**I-2 — `delete_draft` is `pub`: an unguarded deleter of a `parked = 1` row is exported to the TUI crate.**
`input_form_store.rs:75`. `discard_parked_draft`'s doc (:343, "the ONLY deleter of a `parked = 1` row") and §6.2 ("the form's 'discard parked draft' is the **only** path that deletes a `parked = 1` row") are contradicted at the crate seam: `delete_draft` checks nothing and is visible to `btctax-tui-edit`. Failure scenario: any plan-3 (or later web-renderer) code path that reaches for "just remove the draft" — e.g. a cleanup on quit — deletes the sole copy of a screened return, SSNs included, with no gate and no confirm; C-1 would then be held purely by the TUI author's discipline. Every internal caller (`load`, `coherence_clear_or_refuse`, `commit`, tests) is in-crate.
*Fix:* one keyword — `pub(crate) fn delete_draft`. (`set_draft_row` is already correctly `pub(crate)`; `draft_exists`/`init_draft_table` are harmless reads/DDL.)

**I-3 — `commit` never checks `year` against `table.year`/`params.year` — I-11 is enforced per-*call*, not per-*year*, though both structs carry the year.**
`input_form_store.rs:231-252`. SPEC §6.1: "`commit` on a **year** without params returns `NoTables` … it never commits unscreened (which would poison the year at resolve)." The implementation gates only on the caller passing `None`. `TaxTable.year` and `FullReturnParams.year` both exist (`tables.rs:54, :283`). Failure scenario: a plan-3 caller holds the (only) 2024 tables and passes them with `year = 2025` — `screen_inputs` passes, a committed `return_inputs` row is written for a table-less year → at resolve that year fails closed → **uncomputable, and it shadows any stored `tax_profile`** — exactly the Fable P4.9 r1 I1 poisoning that `write_back_carryover`'s own comment (`tax.rs:455-459`) documents as the reason it refuses to fabricate a row. Not Critical because no shipped call site can produce the mismatch (the natural wiring `fr.full_return_for(year)` can't), but the guarantee is held by caller convention where construction costs two comparisons.
*Fix:* `if table.year != year || params.year != year { return Ok(CommitOutcome::NoTables); }` (or a typed error), plus a one-line test.

### Minor (recorded; non-gating)

- **M-1 — the stale-PARKED remedy chain is two-hop and the second hop may be unreachable.** A stale parked draft hit via any committed-row writer yields `ParkedDraftBlocksWrite` (coherence at :185 checks `parked` but not `schema_version`), whose two named exits are both unexecutable for the stale case: 'use full return' can't run (`load` refuses `StaleParkedDraft` first), and the 'X' discard affordance exists only inside a form that may refuse to open for that year. The chain does terminate honestly (attempting the form surfaces `StaleParkedDraft` with the real remedy), so this is not honesty-*broken* — but coherence could check the version and emit `StaleParkedDraft` directly, and **plan 3 must make the discard affordance reachable when `load` errors**, else a stale parked draft is undiscardable in-app.
- **M-2 — `draft_exists` swallows real DB errors** (`input_form_store.rs:80-85`): `.is_ok()` maps a genuine rusqlite failure to `false` instead of `Err`. Contrast `parked_flag` and `return_inputs::exists` (`.optional()?`). Consequence today is only a hidden TUI affordance; fix to `.optional()?`.
- **M-3 — `save_draft` silently overwrites a STALE parked draft** and "heals" its version stamp (`:107-112` — `parked_flag` ignores `schema_version`; `set_draft_row` stamps current). Unreachable through the intended flow (the form can't open — `load` refuses first), so held by caller convention; a version check on the parked path would hold §6.3 by construction.
- **M-4 — post-snapshot, pre-save errors propagate without `restore`** in `commit`/`park`/`discard` (e.g. `return_inputs::delete` failing at `:288` after the stash succeeded). Disk is safe (save never ran), but a caller that continues the session after the error — against the vault's latch rule — could later persist the half-applied set via an autosave's `sess.save()`; in park's case that materializes the committed-row+parked-draft coexistence state, and a subsequent park would overwrite the parked draft. Double-fault territory on an in-memory SQLite (essentially OOM-only), hence Minor; restoring on *any* post-snapshot `Err` makes the fns transactional.

### Nit

- Park's clean-state gate (`:279`, `== Some(false)`) could be `.is_some()` — refusing *any* existing draft closes the (currently unreachable) parked-overwrite corner for free.
- `sess.restore(&snap)?` on the failure paths replaces the original save error if restore itself fails; consider latching both.
- `discard_parked_draft`'s refuse message ("no parked draft to discard") is slightly off for the WIP case — a draft exists, it just isn't parked.
- `save_draft` omits snapshot/restore. This matches the plan's own Task-2 code block verbatim (plan-blessed, and behaviorally right — restoring would evict the autosave from memory while the next debounced autosave converges either way), but it deviates from the letter of Global-Constraint I-7's blanket list; a one-line comment saying why would stop a future reviewer "fixing" it.

## Whole-branch seams

1. **C-1 parked invariant: HELD in every shipped path** — traced all writers of `parked` (`set_draft_row` via `save_draft` read-modify-write [mutation-checked], via park [`true`]) and all deleters (`load` stale path gates on `!d.parked`; coherence refuses `Some(true)`; `discard` requires `Some(true)`; `commit`'s delete is the spec-blessed re-commit). No silent-delete path exists today; but the invariant is convention-held at the crate seam via `pub delete_draft` (I-2) and the stale-parked `save_draft` corner (M-3).
2. **resolve.rs invisibility: HELD by construction** — resolve.rs is untouched by the diff and contains zero references to the draft table/module; `commit` writes only `screen_inputs`-clean rows; a draft never enters resolver precedence.
3. **Atomicity: HELD** — snapshot precedes the first mutation in all three fns; park is stash-before-delete; `Vault::snapshot`/`restore` cover the full DB image so both tables roll back together; no committed-deleted-but-stash-lost split is constructible on the save-failure path (M-4 covers the exotic non-save-failure residue).
4. **§6.2 placement: HELD** — coherence runs right after `Session::open` in all four writers, ahead of every absent-row early-return (verified in full source, not just hunks); write-back targets year+1, the row it writes; the no-draft case is a pure no-op and regresses nothing; the parked-refuse is reachable and integration-pinned via `income clear`.
5. **Never-write-unscreened: HELD for every shipped call shape** — `Refused`/`NoTables` return before the snapshot and are test-pinned; the one unenforced edge is the year↔params mismatch (I-3).
6. **Plan-3 contract: complete for the §9/§9A flows** (`load`/`save_draft`/`commit`/`park_to_profile`/`discard_parked_draft`/`active_source`/`shadows_profile`; re-commit is load→commit per the plan, needing no new fn; internals correctly `pub(crate)`) — **except** the §6.3 discard-note is inexpressible (I-1), and M-1's discard-reachability-on-stale needs a plan-3 note.

## Fixes required to close

1. (I-1) Return the stale-discard fact from `load` (variant field or tuple); print at CLI call sites only.
2. (I-2) `delete_draft` → `pub(crate)`.
3. (I-3) Year-consistency check in `commit` against `table.year`/`params.year` → `NoTables`, with a pinning test.

All three are small, isolated to `input_form_store.rs`, and none disturbs the four writer call sites or any test's shape. Minors M-1..M-4 and the Nits should be filed to FOLLOWUPS with plan-3 as the owning phase for M-1's affordance-reachability note.
