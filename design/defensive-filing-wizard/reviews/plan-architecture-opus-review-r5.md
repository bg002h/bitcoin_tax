# Plan review r5 — architecture lens (Opus)

**Artifact:** `design/defensive-filing-wizard/IMPLEMENTATION_PLAN.md` @ `de4c9cd` (post-r4-fold)
**Contract:** `design/defensive-filing-wizard/SPEC.md` (GREEN, DFW-D1..D12)
**Prior:** `reviews/plan-architecture-opus-review-r4.md` (Opus, NOT GREEN — 0C/1I/0M/2N).
**Reviewer:** Opus (software-architecture lens), independent. Every load-bearing claim re-derived against
CURRENT source on `feat/defensive-filing-wizard` @ `de4c9cd` — no reliance on the plan's self-citations or
line numbers; the complete consumer set of all three moved predicates re-grepped workspace-wide.

## Verdict

**GREEN — 0 Critical / 0 Important / 0 Minor / 2 Nit**

The r4 Important (I-new-2 — the C-2 `tranche_guard` predicate move deleted `pre2025_tranche_exists` from
`cmd::tranche` while a fifth, direct consumer at `session.rs:714` was neither in Task 5's Files header nor
rewired by any step, breaking the `btctax-cli` compile at Task 5 Step 3) is **RESOLVED**. The fold adds
`crates/btctax-cli/src/session.rs` to Task 5's Files header, adds a Step-2 clause rewiring `session.rs:714`
to `btctax_core::tranche_guard::pre2025_tranche_exists(&all)`, names it as the fifth direct consumer in File
Map C-2, and sweeps the stale `session.rs:692` locator in Step 3. n-new-2 (slice-body end-line) and n-new-3
(`void_targets` visibility) are both folded.

**The key check this round — the complete consumer set of all three moved predicates, re-grepped
workspace-wide — confirms NO sixth consumer exists.** The plan's Task 5 now rewires or preserves EVERY one.
The consumer-set enumeration is below. Two residual Nits (both descriptive, neither load-bearing, neither
breaks compile or behavior) are the only findings; they do not gate.

---

## r4-resolution audit

### I-new-2 — the fifth predicate consumer (`session.rs:714`) — RESOLVED

**Independent complete-consumer-set enumeration** (whole-repo grep, `--include='*.rs'`, all workspace
members incl. `xtask`, all build paths, `target/` excluded — `void_targets|in_force_allocation_exists|
pre2025_tranche_exists`):

**Definitions (all three, in `crates/btctax-cli/src/cmd/tranche.rs`, all move to core):**
- `void_targets` — `tranche.rs:40`, **private `fn`** (n-new-3 below).
- `in_force_allocation_exists` — `tranche.rs:54`, `pub`.
- `pre2025_tranche_exists` — `tranche.rs:71`, `pub`.

**Call sites (invocations) — the complete set is FIVE, all in `btctax-cli`:**

| # | Site | Callee | Enclosing fn | Disposition under Task 5 |
|---|------|--------|--------------|--------------------------|
| 1 | `tranche.rs:61` | `void_targets` | `in_force_allocation_exists` (a PREDICATE → **moves to core**) | Moves to core with its enclosing predicate; becomes an intra-module call. |
| 2 | `tranche.rs:72` | `void_targets` | `pre2025_tranche_exists` (a PREDICATE → **moves to core**) | Moves to core with its enclosing predicate; becomes an intra-module call. |
| 3 | `tranche.rs:94` | `pre2025_tranche_exists` | `guard_allocation_vs_tranche` (`:93`, **STAYS in cli**) | Rewire to `btctax_core::tranche_guard::pre2025_tranche_exists`. |
| 4 | `tranche.rs:111` | `in_force_allocation_exists` | `guard_tranche_vs_allocation` (`:107`, **STAYS in cli**) | Rewire to `btctax_core::tranche_guard::in_force_allocation_exists`. |
| 5 | **`session.rs:714`** | **`pre2025_tranche_exists`** | **`Session::safe_harbor_residue` (`:702`, the read-only allocate-opener precheck) — DIRECT `crate::cmd::tranche::…` call, not via a guard** | **Rewire to `btctax_core::tranche_guard::pre2025_tranche_exists(&all)` — NOW scheduled by the fold.** |

**Non-call-site references (test-name/comment only, no rewire required):**
`declare_tranche_cli.rs:289` (test name), `:320`, `:352`, `:358` (locator comments; `:358` cites the now-stale
`session.rs:692`). Task 5 Step 3 sweeps these.

**No re-export of any predicate at the cli crate root.** `lib.rs` re-exports ONLY the STAYING guard
(`pub use cmd::tranche::guard_allocation_vs_tranche;`, `lib.rs:27`) — not a predicate — so no other crate
reaches a predicate through a re-export. **There is NO sixth consumer** in any crate or build path.

**The four allocation APPEND sites call the STAYING guard, preserved automatically** — independently
confirmed: `reconcile.rs:1015` and `:1258` call `crate::cmd::tranche::guard_allocation_vs_tranche`;
`edit/persist.rs:1032` and `:1105` call `btctax_cli::guard_allocation_vs_tranche` (the crate-root re-export).
None of the four calls a predicate directly, so the C-2 move does not touch them — the plan's "preserved
automatically (no rewire)" claim (`:377-378`) is TRUE. (The shipped `declare_tranche_cli.rs:352-358` doc
comment corroborates the whole set: "`guard_allocation_vs_tranche` is the ONE chokepoint for all four
allocation append sites … AND the TUI opener's pre-flight consult (`session.rs:692`, via the same
`pre2025_tranche_exists`)" — i.e. the test author already knew `session.rs` was a fifth, direct consumer and
recorded the now-stale `:692`, which Step 3 corrects to `:714`.)

**Fold verification against the plan text:**
- Task 5 Files header (`:332-334`) lists `crates/btctax-cli/src/session.rs` with the arch-I-new-2 note "rewire
  the FIFTH consumer `:714` — a DIRECT `pre2025_tranche_exists` call, not via a guard." ✓
- Step 2 (`:374-378`) rewires `session.rs:714`'s `crate::cmd::tranche::pre2025_tranche_exists(&all)` →
  `btctax_core::tranche_guard::pre2025_tranche_exists(&all)`, "do NOT leave a duplicate." ✓
- File Map C-2 (`:69-70`) names `session.rs:714` as "a FIFTH, DIRECT consumer … the safe-harbor
  allocate-opener precheck." ✓
- Step 3 (`:379-382`) sweeps the stale `declare_tranche_cli.rs` `:320/:352/:358` locator comments citing
  `session.rs:692`. ✓
- Self-review r4-fold note (`:597-599`) documents I-new-2 = tax-M-2 and asserts "grep-verified there is no
  sixth." My independent grep **confirms** that assertion.

With `session.rs:714` now scheduled, following Task 5 task-by-task no longer produces a RED crate: after the
move + the two guard-internal rewires + the `session.rs:714` rewire + deleting the cli copies, no path names
`crate::cmd::tranche::{void_targets,in_force_allocation_exists,pre2025_tranche_exists}` any longer, and every
invocation resolves (sites 1–2 intra-core, sites 3–5 via `btctax_core::tranche_guard::*`). The Important is
closed.

### n-new-2 (Task 3 slice-body end-line) — RESOLVED

The plan now cites the crypto-slice body as `admin.rs:385-599` (`:96`, `:251`). Re-derived from source:
`export_irs_pdf` (`admin.rs:350`) opens `Session::open` at `:358`, dispatches `return_inputs::exists` at
`:373`, and the slice arm's terminal `IrsPdfReport { … }` constructor opens at `:578` (`:385` =
`promote_export_gate(&state, &events, Some(tax_year))?;`, the first line of the slice body proper after the
full-return dispatch). The body runs to ~`:599`. `:385-599` is accurate; the r4 under-cite (`:385-583`) is
gone.

### n-new-3 (`void_targets` visibility widening) — RESOLVED (in prose)

Step 2 (`:366-369`) now reads: "`void_targets` (`:40`) moved too (★ arch-n-new-3: it is a private `fn`
today — keep it `pub(crate)`/module-private in core, a deliberate visibility choice, not a verbatim
signature)." Confirmed `void_targets` is private (`tranche.rs:40`, `fn`, no `pub`) with only two callers,
both of which move into the same core module, so `pub(crate)` (or plain module-private) is correct and no
consumer outside the module needs it. The deliberate visibility change is now documented, not silently
labeled "verbatim." *(Residual: the interface-block pseudo-signature still reads `pub fn void_targets` — see
Nit N-2.)*

---

## New findings

### N-1 (Nit) — Task 5 Step 2 mislabels two of the "4 sites inside the two guards"

**Source:** Plan `:372-376`: "4 sites inside the two guards `guard_tranche_vs_allocation`/
`guard_allocation_vs_tranche` (`:107,93` … only their internal predicate calls at `tranche.rs:61,72,94,111`
rewire)."

Re-derived from `tranche.rs`: of those four sites, only `:94` (inside `guard_allocation_vs_tranche`) and
`:111` (inside `guard_tranche_vs_allocation`) are actually **inside the two guards**. `:61` is inside
`in_force_allocation_exists` and `:72` is inside `pre2025_tranche_exists` — i.e. inside the two **predicates
that themselves MOVE to core**, not inside the guards. And "rewire" is the wrong verb for `:61,72`: those
`void_targets(events)` calls travel INTO core with their enclosing predicate and become plain intra-module
calls (no `btctax_core::tranche_guard::` external path needed), whereas `:94,111` genuinely rewire because
their enclosing guards stay in cli.

**Why it does not gate:** the governing directive is unambiguous and correct — Step 2 (`:366-369`) explicitly
says "move `void_targets` too" and "DELETE the cli copies," and any implementer moving the three predicates
verbatim carries `:61,72`'s internal calls along automatically. Even a literal (mis-verbed) attempt to
"rewire `:61` to `btctax_core::tranche_guard::void_targets`" compiles (a crate may name itself; `void_targets`
is `pub(crate)`, so the within-crate path resolves). So the mischaracterization produces no wrong action, no
compile break, no behavior change — it is a descriptive imprecision only.

**Fix (optional):** reword to "2 guard-internal rewire sites (`:94,111`, inside the STAYING guards) + 2
intra-predicate `void_targets` calls (`:61,72`) that move to core with their enclosing predicate + the fifth
direct consumer `session.rs:714`."

### N-2 (Nit) — interface-block pseudo-signature for `void_targets` still says `pub fn` (residual n-new-3)

**Source:** Plan `:342`: `pub fn void_targets(events: &[LedgerEvent]) -> BTreeSet<EventId>;` — vs Step 2
(`:366-369`) which correctly directs `pub(crate)`/module-private.

The "Produces" interface block still declares `pub fn`, contradicting the (authoritative) Step-2 visibility
instruction folded for n-new-3. Benign — `void_targets` has no consumer outside the core module, so `pub` is
harmless and `pub(crate)` is what ships; the code compiles either way. But the interface sketch and the step
now disagree on one token. **Fix:** change `:342` to `pub(crate) fn void_targets` (or drop the `pub`) to
match Step 2.

---

## Verified sound (no finding) — re-derived at `de4c9cd`

- **Seam integrity / no core→cli inversion.** The three moved predicates use only core types
  (`LedgerEvent`, `EventId`, `BTreeSet`) and `btctax_core::conventions::TRANSITION_DATE` (`tranche.rs:74,111`
  reference `TRANSITION_DATE`) — no `btctax-cli` symbol — so they move to core cleanly. tui-edit reaches the
  chokepoint via `btctax_cli` crate-root re-exports only: the `guard_allocation_vs_tranche` (`lib.rs:27`) and
  `promote_export_gate` (`lib.rs:37`) precedents establish the pattern, and the planned
  `pub use crate::cmd::admin::IrsPdfReport;` (arch-n-1) follows it — no `cmd::` path leak into tui-edit.

- **Chokepoint deadlock hazard.** `apply_export(session: &Session, …)` and `export_irs_pdf_from_session(
  &Session, …)` compose over the editor's already-open session — no re-`Session::open`. The deadlock
  citation holds: `session.rs:661-662` — "a second open would deadlock on the held VaultLock, and
  `cmd::optimize::accept` is forbidden to the editor for the same reason." The C-1 extraction (thin
  `export_irs_pdf` keeps `Session::open` at `:358`; the `&Session` inner takes the dispatch at `:373` + slice
  body `:385-599` + the `export_full_return:642` delegation) is intact and the full-return arm is
  characterization-pinned (Task 3 Step 1 both arms).

- **Write confinement (KAT-G1).** `kat_g1_mechanized_source_gate` (`persist.rs:1897`) with `everywhere_tokens`
  (`:1919`) and `persist_only_tokens` (`:1955`) scans only `btctax-tui-edit/src`; the chokepoint's own
  `crate::cmd::admin::export_irs_pdf_from_session` (inside `btctax-cli`) is outside the scan and trips no
  `cmd::` violation. C-3's plan to extend `persist_only_tokens` with `apply_declare(`/`apply_promote(`/
  `apply_export(` and route every write through `persist.rs` wrappers is the correct shape.

- **Export re-export precedent + degenerate trio.** `pub struct IrsPdfReport` (`admin.rs:261`);
  `promote_export_gate` (`admin.rs:78`) is the year-enumeration source for `promoted_filing_years`
  (`admin.rs:84-98` region); `pub use cmd::admin::promote_export_gate` (`lib.rs:37`) is the exact re-export
  precedent for the planned `IrsPdfReport` crate-root re-export.

- **Compile-ability of the C-2 move.** With all five consumers now dispositioned (2 intra-core, 2
  guard-internal rewires, 1 `session.rs:714` rewire) and the cli copies deleted, the crate compiles at Task 5
  Step 3. This was the single un-surfaced edit r4 flagged; it is now surfaced.

- **Characterization polarity / task right-sizing.** Task 5 Steps 1–3 pin the shipped allocation-guard
  behavior PASS-before-move (behavior-preserving cross-crate refactor); Steps 4–8 FAIL-for-new (the
  structured shortfall signal). The C-2 move is bounded, task-sized surgery.
