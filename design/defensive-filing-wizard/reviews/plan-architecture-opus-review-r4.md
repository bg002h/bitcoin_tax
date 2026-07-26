# Plan review r4 — architecture lens (Opus)

**Artifact:** `design/defensive-filing-wizard/IMPLEMENTATION_PLAN.md` @ `9ceede7` (post-r3-fold)
**Contract:** `design/defensive-filing-wizard/SPEC.md` (GREEN, DFW-D1..D12)
**Prior:** `reviews/plan-architecture-opus-review-r3.md` (Opus, NOT GREEN — 0C/1I/4m/1n).
**Reviewer:** Opus (software-architecture lens), independent, every load-bearing claim re-derived against
CURRENT source on `feat/defensive-filing-wizard` — no reliance on the plan's self-citations or line numbers.

## Verdict

**NOT GREEN — 0 Critical / 1 Important / 0 Minor / 2 Nit**

The r3 fold is correct and complete on every item it set out to close: I-new-1's core defect (the C-2
predicate move was scheduled in NO task) is fixed — Task 5 now schedules the move as real characterization
→ cross-crate-move → behavior-preserving-refactor surgery, with `tranche_guard.rs` in its Files header, so
Task 6/Task 8's `tranche_guard::` references now resolve to a module a prior P-B task creates. m-new-1..4
and n-new-1 all hold against source. **But re-deriving the newly-scheduled move fresh surfaces one new
Important:** the move deletes `pre2025_tranche_exists` from `cmd::tranche` while a FIFTH direct consumer —
`session.rs:714`, `crate::cmd::tranche::pre2025_tranche_exists(&all)` — is neither in Task 5's Files header
nor rewired by any step, so the `btctax-cli` crate fails to compile at Task 5 Step 3. The plan's
blast-radius model ("single source preserved for all four allocation append sites") under-counts the
consumers by one; this is the residue of the same I-new-1 executability class, on a call site the fold
never enumerated.

---

## r3-resolution audit

### I-new-1 (C-2 predicate move scheduled in NO task; Task 6 cannot compile) — RESOLVED (as to the r3 defect)

The r3 Important was that the C-2 move was described only in the File Structure Map and scheduled in no
task, so Task 6 — its first consumer — composed `tranche_guard::in_force_allocation_exists` over a module
no step created. The fold closes exactly that:

- **Task 5 is retitled** "Core: the `tranche_guard` predicate move (C-2) + the structured shortfall signal
  + DFW-D4 triage" (`:320`).
- **Task 5 Files header** (`:322-327`) now lists `crates/btctax-core/src/tranche_guard.rs`, the
  `defensive/mod.rs` skeleton, `crates/btctax-core/src/lib.rs` (`pub mod tranche_guard; pub mod defensive;`),
  and `crates/btctax-cli/src/cmd/tranche.rs` (rewire).
- **Steps 1–3** (`:351-364`) schedule it as real, PASS-before-refactor surgery: Step 1 pins the shipped
  `declare_tranche_cli.rs` allocation-guard behavior as the baseline; Step 2 creates `tranche_guard.rs` with
  the three predicates moved verbatim, adds the `pub mod`s, rewires `cmd/tranche.rs`'s two guards over
  `btctax_core::tranche_guard::*`, and deletes the cli copies; Step 3 asserts the shipped KATs still green +
  `make check` + a `refactor(core)` commit.

Re-verified this is a genuine, compilable, behavior-preserving move: the three predicates today are
`void_targets` (`tranche.rs:40`, private `fn`), `in_force_allocation_exists` (`:54`, `pub`),
`pre2025_tranche_exists` (`:71`, `pub`), each taking only `events: &[LedgerEvent]` and using
`btctax_core::conventions::TRANSITION_DATE` (confirmed `TRANSITION_DATE` is `conventions.rs:17`) — all core
types, so they move to core with no core→cli inversion. `guard_tranche_vs_allocation` (`:107`, private) and
`guard_allocation_vs_tranche` (`:93`, `pub`) wrap them and correctly STAY in cli; the crate-root re-export
`pub use cmd::tranche::guard_allocation_vs_tranche` (`lib.rs:27`) and its live consumers
(`reconcile.rs:1015,1258`; the tui-edit re-export callers `persist.rs:1032,1105`) all reference the GUARD,
which does not move, so they survive. **Task 5 (P-B) precedes Task 6 (P-B) and Task 8 (P-C)**, so the
`tranche_guard::` references in Task 6 Step 3 (`:442-444`) and Task 8 Step 1(d) (`:496`) now resolve to a
module a prior step creates. The specific r3 executability gap is closed.

*(But the newly-scheduled move's blast radius is under-counted by one consumer — see I-new-2. I-new-1's
"does Task 6 compile" question is answered YES; the new finding is a different call site the fold missed.)*

### m-new-1 (export dispatch factoring internally inconsistent) — RESOLVED

The r3 Minor was that the interface prose + Step 6 described `apply_export` as itself branching
slice-vs-full while Step 3 + the thin-opener put the dispatch inside `_from_session` — contradictory
factorings, one of which silently drops the full-return dispatch untested. The fold reconciles everything
to the correct (Step-3) reading:

- Interface (`:263-267`): "`apply_export` writes ONE packet per year by calling
  `export_irs_pdf_from_session(session, …, year, …)` per year; the full-vs-slice `return_inputs::exists`
  dispatch lives ONCE INSIDE `_from_session` (exactly as shipped `admin.rs:373`) — `apply_export` itself
  does not branch."
- Step 6 (`:290-295`): "`apply_export` calls `export_irs_pdf_from_session(session, …, year, …)` per year
  (the `return_inputs::exists` slice-vs-full dispatch lives ONCE inside `_from_session`)".
- No residual "`apply_export` dispatches / `_from_session` = slice-only" phrasing remains.

Re-verified against source: shipped `export_irs_pdf` (`admin.rs:350`) opens its own `Session` (`:358`),
projects (`:359`), then `if crate::return_inputs::exists(session.conn(), tax_year)?` (`:373`) delegates the
full-return arm to `export_full_return(&session, &state, &events, …)` (`:375`) and otherwise runs the
crypto-slice body. Extracting everything after `:358` into a `&Session` `_from_session` keeps that single
dispatch intact, so `export_irs_pdf` (thin opener) and `apply_export` both compose over it with no path
dropping the full-return branch. **Step 1 now pins BOTH arms** (`:269-274`): case (a) a promoted-disposal
vault with NO `return_inputs` (slice arm) and case (b) a vault WITH `return_inputs::exists(year)` (full
arm), so the retained dispatch is characterization-pinned and a slice-only `_from_session` would red case
(b). The m-new-1 hazard is closed.

### m-new-2 (Files-header manifest omissions on cross-crate files) — RESOLVED

Task 3 Files (`:232-233`) now list `crates/btctax-core/src/conservative.rs` (`flagged_years`) and
`crates/btctax-cli/src/lib.rs` (`IrsPdfReport` re-export). Task 5 Files (`:322-327`) now list
`crates/btctax-core/src/project/fold.rs` (populate `state.shortfalls`) and the `defensive/mod.rs` skeleton.
Both additions match the Steps that edit those files. *(The same manifest-completeness discipline was NOT
extended to `session.rs`, which the C-2 move also touches — that is I-new-2, filed as Important because it
is a compile break, not merely a grep-reconciliation gap.)*

### m-new-3 (`plan_promote`/`plan_declare` dead `state` param) — RESOLVED

`plan_promote(events, prices, cfg, target, provenance, part_ii, now)` (`:145-146`) no longer carries
`state`; `plan_declare(events, prices, cfg, sat, wallet, ws, we, target_shortfall, now)` (`:205-207`) no
longer carries `state`. Re-verified the shipped promote pipeline is events-only: `promote_tranche`
(`promote.rs:364`) does `resolve_live_tranche(&events, …)` (`:378`), `session.config()?.to_projection()`
(`:396`), `consent_terms(…)` (`:410`), `promote_prior_year_advisory(…)` (`:433`),
`gift_only_flagged_years(…)` (`:449`), `would_conflict(&events, …)` (`:478`), `append_decision(…)` (`:485`)
— never a pre-built `LedgerState`. The declare `None` path is `tranche.rs:134-154` (events-only) and the
`Some` clearance re-projects from events. No dead param, no `unused_variables` clippy risk under `make check`.

### m-new-4 (`journey_view` discovery off a possibly-pseudo state) — RESOLVED

Task 6 (`:403-406`) now opens `journey_view` with `debug_assert!(!state.pseudo_active())` and documents it
as "the DFW-D6 precondition the Task 7 entry gate enforces … so the DISCOVERY read `shortfalls(state)` is
never taken off a pseudo-active state whose `state.shortfalls` are synthetic-cleared." This is exactly the
"state the precondition + assert it" resolution r3 offered. Confirmed `LedgerState::pseudo_active`
(`state.rs:290`) exists, so the assert compiles; the operative guard remains the dashboard's
`!state.pseudo_active()` entry gate (Task 7 Step 1(a)), with `journey_view_forces_pseudo_off` (Task 6 KAT)
holding the shadow-projection behavior. Precondition now explicit and checked.

### n-new-1 (Task 3 Step 3 "move Session::open into a &Session inner" wording) — RESOLVED

Step 3 (`:276-281`) now reads "the thin `export_irs_pdf(vault_path, pp, …)` KEEPS `Session::open` (`:358`)
and calls the inner; … everything AFTER the open — the … `return_inputs::exists` dispatch (`:373`), the
`export_full_return` delegation, AND the crypto-slice body (`:385-583`) — moves INTO the `&Session`
`_from_session`." The open now unambiguously stays in the thin opener; the impossible "move the open into
the fn that receives an already-open `&Session`" reading is gone.

---

## Important

### I-new-2 — Task 5's C-2 move deletes `pre2025_tranche_exists` from `cmd::tranche` but leaves a fifth, direct consumer (`session.rs:714`) unscheduled; the `btctax-cli` crate does not compile at Task 5 Step 3

Task 5 Step 2 (`:356-362`) instructs: "rewire `cmd/tranche.rs` so
`guard_tranche_vs_allocation`/`guard_allocation_vs_tranche` … call `btctax_core::tranche_guard::*`, and
DELETE the cli copies (single source; all four allocation append sites preserved)." The Files header
(`:322-327`) lists exactly one cli file to modify: `crates/btctax-cli/src/cmd/tranche.rs`.

Re-grep of every consumer of the three moved predicates shows the plan's "all four allocation append sites"
model is missing a consumer. `pre2025_tranche_exists` has THREE production call sites, not the
guard-mediated set the plan enumerates:

- `tranche.rs:94` — inside `guard_allocation_vs_tranche` (guard STAYS; rewired). OK.
- `session.rs:714` — **`if crate::cmd::tranche::pre2025_tranche_exists(&all) {`** — a DIRECT call, not
  through a guard, inside `Session::safe_harbor_residue` (the read-only allocate-flow opener that refuses
  to show a residue when a pre-2025 tranche exists, `session.rs:702-724`). **This is the fifth consumer,
  and it is neither in Task 5's Files header nor rewired by any step.**
- Test-name/comment mentions in `declare_tranche_cli.rs:289,320,352,358` — not call sites.

`in_force_allocation_exists` has one non-guard-internal consumer (`tranche.rs:111`, inside the retained
`guard_tranche_vs_allocation`); `void_targets` is tranche-internal only — both move cleanly. Only
`session.rs:714` is orphaned.

**Failure scenario:** an implementer executing Task 5 task-by-task (the plan's declared
`subagent-driven-development` model) follows Step 2 literally — moves the three predicates to core, rewires
`cmd/tranche.rs`'s two guards, deletes the cli copies — and at Step 3 runs `make check`, which **fails to
compile**: `session.rs:714` names `crate::cmd::tranche::pre2025_tranche_exists`, which no longer exists at
that path (a plain `use` inside the rewired `tranche.rs` does not republish it as
`crate::cmd::tranche::pre2025_tranche_exists`, and the plan does not direct an explicit `pub use`
re-export). To proceed they must perform an unscheduled edit to a file not in the manifest — the exact
"hidden surgery / not independently compilable" class I-new-1 was raised to prevent, and the residue of the
same under-counted blast radius (the plan's self-review, `:562-563` area, and the File Map `:68` both assert
"single source … for all four allocation append sites," which is now false by one).

This is Important, not Minor: unlike m-new-2's grep-reconciliation omissions (whose edits the Steps made
unmissable so the crate still built), here the scheduled Step *actively deletes* a symbol a non-manifest
file still references, so following the plan produces a RED crate with no step covering the fix.

**Fix (either is sufficient, and correct the blast-radius claim):**
1. Add `crates/btctax-cli/src/session.rs` to Task 5's Files header and a Step-2 clause: rewire
   `session.rs:714` to `btctax_core::tranche_guard::pre2025_tranche_exists(&all)`. (Cleanest — completes
   the "single source in core" intent.) **OR**
2. Keep an explicit `pub use btctax_core::tranche_guard::pre2025_tranche_exists;` (and any other externally
   referenced predicate) in `cmd/tranche.rs` so `crate::cmd::tranche::pre2025_tranche_exists` still
   resolves, and note in Step 2 that `session.rs:714` depends on that re-export.

Then correct the File-Map (`:68`) / self-review (`:562-563`) wording from "all four allocation append
sites" to also count `session.rs`'s direct `safe_harbor_residue` opener as a consumer of the moved
predicate, so a future reconcile grep finds it.

---

## Nit

- **n-new-2 (Task 3 slice-body end-line under-cite).** The plan cites the crypto-slice body as
  `:385-583` (`:249,:280`). Re-grep: the slice body's terminal `IrsPdfReport { … }` constructor spans
  `admin.rs:578-599` (closing `}` at `:600`), so the body runs to ~`:599`, not `:583` (r3 itself cited
  `:385-599`). Not load-bearing — Step 3 says "everything AFTER the open … moves INTO `_from_session`," so
  the exact end line does not change the surgery — but tighten the citation for grep-accuracy.

- **n-new-3 (`void_targets` visibility widening, benign).** The plan's `tranche_guard.rs` interface
  (`:333`) declares `pub fn void_targets(events) -> BTreeSet<EventId>`, but the shipped fn is private
  (`tranche.rs:40`, `fn`). Moved into core beside its only two callers (`in_force_allocation_exists` /
  `pre2025_tranche_exists`) it can stay module-private; publishing it is harmless (no consumer outside the
  module), but note it is a deliberate visibility change, not a verbatim move, so "moved verbatim" (Step 2)
  is slightly imprecise for this one predicate.

---

## Verified sound (no finding)

- **Gate ordering** (Global Constraints `:25-28`) matches the shipped pipeline: resolve-live
  (`promote.rs:378`) → provenance/Part II/floor → `consent_terms` (`:410`) → synthetic-promote advisory
  (`for line in &advisory`, `:443`) → gift-only relabel (`gift_only_flagged_years`, `:449`) → consent
  render (`render_consent(&terms, &gift_only_years)`, `:453`) + `wide_window_note` (`:454`) →
  `require_promote_ack` (`:458`) → `would_conflict` (`:478`) → `append_decision` (`:485`). `render_consent`
  is `pub fn render_consent(terms: &[ConsentTerm], gift_only_years: &BTreeSet<i32>) -> String`
  (`promote.rs:333`); the `PromotePlan` three ordered pieces (`advisory_lines`/`gift_only_years`/
  `post_consent_note`, `:135-139`) reproduce this order byte-for-byte (I-1 from r2, still holds).

- **Export `&Session` extraction (C-1) still holds.** `export_full_return` (`admin.rs:642`) is already
  `fn export_full_return(session: &Session, state, events, out_dir, tax_year, attest)`, so the
  `_from_session` slice branch mirrors it. Re-confirmed the slice body needs `&Session` beyond
  `state`/`events`: `session.resolve_screened(&state, tax_year, &tables)` (`admin.rs:460`, SE profile) and
  `session.donation_details()` (`admin.rs:508`, Form 8283). `apply_export(session: &Session, …)` composes
  over the TUI's already-open session — no second `Session::open`, avoiding the held-`VaultLock` deadlock
  (`session.rs:661-662`, "a second open would deadlock on the held VaultLock" — citation confirmed).

- **The six sat-carrying `UncoveredDisposal` fold sites** are EXACTLY `:388, 710, 831, 876, 1196, 1274`
  (re-grepped `fold.rs`); the other emits (`:691,742,819,864,935,1177,1225,1255,1303`) are the
  without-wallet/degenerate data-fix variants, correctly excluded (m-5 from r2, holds).

- **Every composed shipped fn exists and is core-derivable:** `method_inversion_advisory`
  (`conservative.rs:61`), `tranche_dip_advisory` (`conservative.rs:27`), `promote_prior_year_advisory`
  (`conservative.rs:689`, returns `Vec<String>`), `window_reference` (`conservative.rs:236`),
  `clamped_promote_year_saving` (`conservative_promote.rs:487`), `promote_drift_advisory`
  (`conservative_promote.rs:89`), `consent_terms` (`conservative_promote.rs:258`). `flagged_years` does not
  yet exist (Task 3 creates it); `state.shortfalls` does not yet exist (Task 5 creates it) — both correctly
  new.

- **Seam integrity / write confinement.** The KAT-G1 gate `kat_g1_mechanized_source_gate`
  (`persist.rs:1897`) roots at `CARGO_MANIFEST_DIR/src` — i.e. it scans **only `btctax-tui-edit/src`**, so
  the chokepoint's own `crate::cmd::admin::export_irs_pdf_from_session` reference (inside `btctax-cli`) is
  outside the scan and trips no `cmd::` violation. `everywhere_tokens` includes `"cmd::"` (`:1920`);
  `persist_only_tokens` (`:1955-1963`) is `conn(`/`save(`/`tax_profile::set`/`append_`/
  `donation_details::set`/`optimize_attest::set`/`restore(` — the plan's C-3 extension with
  `apply_declare(`/`apply_promote(`/`apply_export(` is the correct shape. tui-edit reaches the chokepoint via
  `btctax_cli::chokepoint::*` (crate root) + `btctax_cli::IrsPdfReport` (re-export `pub use
  crate::cmd::admin::IrsPdfReport;`, precedent `pub use cmd::admin::promote_export_gate` at `lib.rs:37`) —
  no `cmd::` path leak. `apply_export` taking `&Session` (export mutates no events, writes only the FS via
  `btctax-cli`'s `fsperms` calls, which are outside the tui-edit scan) is correct;
  `apply_declare`/`apply_promote` take `&mut Session`. Every core signature takes `&dyn TaxTables`, never
  `BundledTaxTables`; no core fn names a `btctax-cli` symbol.

- **Characterization polarity / task right-sizing.** Task 1/Task 3/Task 5 Steps 1–3 pin shipped output
  PASS-before-refactor; Task 5 Steps 4–8 / Task 6 KATs FAIL-for-new. The C-2 move is now surfaced as
  task-sized surgery (Task 5 Steps 1–3). The ONE remaining un-surfaced edit is `session.rs:714` (I-new-2).
