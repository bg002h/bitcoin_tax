# Plan review r1 — architecture lens (Fable)

**Artifact:** `design/defensive-filing-wizard/IMPLEMENTATION_PLAN.md` @ 0c5e9f3
**Contract:** `design/defensive-filing-wizard/SPEC.md` (GREEN, DFW-D1..D12)
**Reviewer:** Fable (software-architecture lens), independent, re-derived against source @ `feat/defensive-filing-wizard`.

## Verdict

**NOT GREEN** — **3 Critical / 6 Important / 6 Minor / 3 Nit**

The plan's spine (plan/confirm/apply extraction with characterization-first sequencing, ack inside
`apply`, the parity harness as the P-A gate) is sound and matches the shipped promote pipeline
(`cmd/promote.rs:364-488` verified). The session-parameterized `apply_*` design is *confirmed correct*
against `session.rs:662` (cmd fns opening their own `Session` deadlock the editor's held `VaultLock` —
the chokepoint is the only workable shape). But three findings make tasks unexecutable or silently
weaken a shipped mechanized guarantee, and six more leave an implementer making contract-level
decisions the plan was supposed to make.

---

## Critical

### C-1 — Task 6: `journey_view(..., tables: &BundledTaxTables, ...)` cannot compile in `btctax-core`

`BundledTaxTables` is defined in **btctax-adapters** (`crates/btctax-adapters/src/tax_tables.rs:66`);
`TaxTables` is the trait in core (`crates/btctax-core/src/tax/tables.rs:113`). btctax-adapters depends
on btctax-core; core naming `BundledTaxTables` is a dependency cycle — the Task 6 interface **cannot
compile as written**. Every shipped core consumer takes the trait (`consent_terms`,
`clamped_promote_year_saving`, `promote_prior_year_advisory` all take `tables: &dyn TaxTables` —
verified `conservative_promote.rs:258-266,487-495`, `conservative.rs:689-697`).
**Fix:** `journey_view(..., tables: &dyn TaxTables, ...)`. The "table ∈ {2017,2024,2025,2026}" flavor
gate becomes `tables.table_for(y).is_some()` (the trait's only method). Callers (dashboard, tests)
pass `BundledTaxTables::load()` / the `BTreeMap` test double.

### C-2 — Task 6 (and Task 8): `journey_view` "composing `guard_tranche_vs_allocation`" is a core→cli dependency inversion

Task 6 Step 3 instructs core's `journey_view` to compose `guard_tranche_vs_allocation` for
`safe_harbor_blocked`. That fn is **private** in `crates/btctax-cli/src/cmd/tranche.rs:107`, in a
crate core cannot depend on (cli depends on core). Unexecutable as written; the naive resolution —
re-deriving the predicate in core — creates exactly the second gating authority DFW-D1 forbids.
Task 8's declare-flow safe-harbor precheck has the same problem from tui-edit: the underlying
predicates `in_force_allocation_exists`/`pre2025_tranche_exists` (`cmd/tranche.rs:54,71`) are pub but
only reachable via a `cmd::` path, which the tui-edit source gate bans, and they are NOT re-exported at
the cli crate root (`lib.rs:27` re-exports only `guard_allocation_vs_tranche`).
**Fix:** add a step (Task 5 or 6) moving the three pure event-scan predicates (`void_targets`,
`in_force_allocation_exists`, `pre2025_tranche_exists` — they use only core types +
`conventions::TRANSITION_DATE`) into btctax-core; `cmd/tranche.rs` keeps its thin `CliError`-wrapping
guards over them (single source preserved for all four allocation append sites + the tranche path).
`journey_view` and the Task 8 flow both read the core predicate; alternatively the flow derives entry
state from `plan_declare`'s `Refusal` per DFW-D1 ("step availability derived from the chokepoint's
plan/guard results") — but the dashboard's *entry-time* first-class state (DFW-D9) still needs the core
predicate.

### C-3 — Tasks 7/9/10: the editor's write-confinement guarantee (KAT-G1) is silently bypassed, and the plan never updates the gate

The tui-edit mechanized gate is **KAT-G1** (`edit/persist.rs:1897`, cloning btctax-tui's e10), whose
shipped guarantee is explicit: *"`persist` is the ONLY module permitted to name the mutation surface"*
(`edit/mod.rs:1-9`; allowlist = `edit/persist.rs` alone for `conn(`/`save(`/`append_`/…, and the
export fns `export_snapshot`/`write_csv_exports`/`write_form_csvs` forbidden EVERYWHERE incl. tests —
the editor currently has NO export surface). The plan routes `apply_declare`/`apply_promote` calls into
`edit/declare_flow.rs`/`edit/promote_flow.rs` (Task 9 Step 3: "route via the `btctax_cli::chokepoint::*`
re-export") and `apply_export` into `defensive_dashboard.rs` (Task 10). None of these tokens are in
G1's lists, so the gate stays green while three non-persist modules acquire append/save/file-write
capability — **the extraction moves a shipped guarantee's enforcement without moving the gate**. The
plan's own instruction "write-class only in permitted modules" (Task 7 Step 3) is unimplementable: the
permitted-module set is `edit/persist.rs`, which appears nowhere in the plan's File Structure Map, and
no G1 change is scheduled.
**Fix:** (a) follow the shipped flow→persist split — flows collect input; new
`persist_declare_tranche`/`persist_promote_tranche`/`persist_defensive_export` wrappers in
`edit/persist.rs` make the chokepoint calls; (b) extend KAT-G1's `persist_only_tokens` with
`chokepoint::apply` (or `apply_declare(`/`apply_promote(`/`apply_export(`) and amend the G1/`edit/mod.rs`
guarantee text for the editor's new (chokepoint-only) export surface; (c) plant one new token in the G1
self-check. Add `edit/persist.rs` to the File Structure Map and to Tasks 8–10's file lists.

---

## Important

### I-1 — Tasks 3/6: the fold-diff year-set has no structured API and two homes on opposite sides of the crate boundary

`promote_prior_year_advisory` returns `Vec<String>` — the year enumeration + per-year leg-set diff is
inline display logic (`conservative.rs:689-790`, verified). Task 3's `export_year_set` "enumerated via
the `promote_prior_year_advisory` fold-pair machinery" is implementable only by parsing years out of
display strings (the DFW-D7 anti-pattern) or by an **unscheduled refactor** of `conservative.rs` —
which is absent from Task 3's file list. Meanwhile Task 6 puts `export_years: BTreeSet<i32>` on
`DefensiveFilingView` (core) while Task 3 defines `export_year_set` in the cli chokepoint — core cannot
call it, so the implementer ships **two divergent enumerations** of the very set that gates 1040-X
packets (DFW-D11). Note also the shipped advisory filters `years.retain(|y| *y < current)` and takes
ONE `promote_id`; the export set must union across ALL live promotes ∪ {current} — unstated.
**Fix:** extract a structured `promote_flagged_years(events, prices, cfg, promote_id, current) ->
BTreeSet<i32>` in core `conservative.rs` (the existing leg-set-diff loop), rebuild the advisory on it;
`export_year_set` (cli, Task 3) and `journey_view.export_years` (core, Task 6) both derive from it
(per-promote union ∪ current). Add `crates/btctax-core/src/conservative.rs` to Task 3's files.

### I-2 — Task 1: the thin-driver recipe drops the advisory lines and mis-homes `wide_window_note` — byte-parity unachievable following the steps as written

Shipped stdout order (`promote.rs:437-458`): prior-year advisory lines → consent render →
`wide_window_note` → ack gate. Task 1's `PromotePlan.advisory_lines` conflates "synthetic-promote
advisory + wide_window_note" into one Vec, and Step 4's driver is specified as only
`println!(render_consent(&plan))` — the advisory lines are never printed and the note's post-consent
position is lost. The characterization test (Step 1) will red and leave the implementer to invent the
split the plan was supposed to specify.
**Fix:** `PromotePlan { advisory_lines: Vec<String> /* pre-consent */, post_consent_note:
Option<String> /* wide_window_note */ , ... }`; Step 4's driver prints advisory_lines →
`render_consent` → post_consent_note → collects ack. The parity KAT (Task 4) compares the full ordered
transcript.

### I-3 — Task 6: `TrancheStatus::DidNotCover` is the per-tranche attribution DFW-D5.3 forbids

SPEC DFW-D5.3 (didn't-cover): the **shortfall row** enters a combined **pool-level** "still short"
state — "Render as ONE pool state …, **never a per-tranche attribution**". The plan models it as a
variant of `TrancheStatus` on `TrancheRow` — a per-tranche attribution, contradicting the settled SPEC
(coverage is emergent; several tranches can share the pool).
**Fix:** move it off `TrancheRow`: e.g. `ShortfallCandidate`/pool row carries `still_short:
Option<{live_tranche_sat, short_sat}>` derived per `pool_key(window_end, wallet)`; render the SPEC's
one-pool-state copy. Update the Task 6/7 KATs accordingly (see I-5).

### I-4 — Tasks 5/6: the per-event `Shortfall` aggregate erases the principal/fee split that `FeeOnlyPromoteNoop` needs

Task 6's `Advisory::FeeOnlyPromoteNoop` (SPEC DFW-D3 fee-only carve) requires knowing a tranche covers
ONLY `consume_fee` fee-shorts. The fold sites distinguish them ("self-transfer/gift fee short…"
`fold.rs:~386-391` vs "dispose short…" `fold.rs:~708-714`), but Task 5's
`Shortfall{event,wallet,date,short_sat}` **sums them per event** (DFW-D7), so the structured signal
cannot express fee-only — the implementer's only routes back out are a `Blocker.detail` parse (banned)
or an ad-hoc re-derivation. No task states the derivation and no KAT covers the advisory's firing
condition (Task 7(e) only tests rendering).
**Fix:** carry components in the fold-populated record (e.g. `principal_sat`/`fee_sat`, still summed
into `short_sat` for clearance/DFW-D8), populated at the emit sites; derive `FeeOnlyPromoteNoop` iff
the covered shortfalls are all fee-component; add a Task 6 KAT for it.

### I-5 — SPEC §5 minimum KATs with no owning task

Missing from every task's KAT list, all SPEC-§5-mandated: (a) **DFW-D4:** "a `pending-out` short routes
through `UnmatchedOutflows` first"; (b) **DFW-D5:** the pool-level "still short — don't declare again"
render; (c) **DFW-D5:** "a cleared tranche removes the shortfall row"; (d) **DFW-D6:** the clearance
shadow's own pseudo-off (Task 2 Step 3 forces it; no KAT/mutation holds it — the untested-guard
pattern).
**Fix:** (a),(c),(d) → Task 5/2/6 KAT lists; (b) → Tasks 6/7 alongside the I-3 remodel.

### I-6 — Tasks 3/10: the export trio's interface is a placeholder

`plan_export(...)` is literally elided; `ExportPlan` has no fields anywhere; `apply_export(...) ->
Result<IrsPdfReport, CliError>` names a type that is per-call/per-year (`cmd/admin.rs:261`, produced by
the year-scoped `export_irs_pdfs`/`export_full_return`) while the dashboard's `x` drives a multi-year
set — the aggregation shape (N calls? a `Vec<IrsPdfReport>`? partial-failure semantics?) and which
shipped export fns the trio wraps (crypto-slice vs full-return vs snapshot), plus the TUI's `out_dir`
source, are all unspecified. This is the only trio whose Plan type is never defined — the self-review's
"Placeholders: none" is false here.
**Fix:** specify `ExportPlan { years: BTreeSet<i32>, out_dir: PathBuf, kind: … }`,
`plan_export(session/state, events, prices, cfg, out_dir, current_year) -> Result<ExportPlan, Refusal>`
(gates: `promote_export_gate` per year + pseudo refuse-and-route), `apply_export` iterating the year
set over the shipped per-year export fn, returning `Vec<IrsPdfReport>` (or a summary), fail-closed
before any bytes on the first refused year.

---

## Minor

- **m-1 (Tasks 7/9):** the plan calls the tui-edit gate "the e10 `mechanized_source_gate`" — e10 is
  **btctax-tui**'s (`export.rs:974`); the editor's is **KAT-G1** (`edit/persist.rs:1897`) with a
  different token table. Name the right gate so "run — PASS (incl. e10)" watches the right test.
- **m-2 (Task 1 Step 1):** "reuse `promote_cli.rs`'s `build_promoted_vault`" — separate integration-test
  binaries can't import from each other. Move it to `tests/common/mod.rs` or `src/testonly.rs` (the
  crate already has one) and have both tests use it.
- **m-3 (Task 6):** `journey_view` takes both `state` and `events`; DFW-D6 requires the **discovery**
  signal pseudo-off, but the passed `state` may be a pseudo-active projection whose `shortfalls` are
  synthetic-cleared. The `journey_view_forces_pseudo_off` KAT holds it, but the signature invites the
  bug — either drop `state` (re-project internally, pseudo-off) or document the precondition + assert.
- **m-4 (Task 7, P-B):** the dashboard's `d`/`p` key-dispatch "LAUNCHES the sibling flows (Tasks 8–9)"
  — those types don't exist until P-C, so P-B can't compile the dispatch as described. State the stub
  (keys inert/message until P-C) or move dispatch wiring into Tasks 8/9.
- **m-5 (Task 5):** the cited fold emit sites drift by ~2 (`:710,831,876,1196,1274` today, not
  `:712,833,878,1198,1276`) and there are **15** `UncoveredDisposal` emit sites total; "six
  sat-carrying" matches the SPEC's 5-without-wallet+4-degenerate arithmetic but the implementer must
  re-grep at implementation time, not trust the list.
- **m-6 (Task 1):** `plan_promote(state: &LedgerState, …)` — the shipped pipeline never consumes a
  pre-built state (it projects internally); drop the param or state its purpose. `Refusal` also lacks a
  variant for `resolve_live_tranche` failures (unknown/voided/already-promoted target) — say where they
  map (`Refusal::Conflict`? a new variant?) so the parity KATs can cover them.

## Nit

- **n-1 (Task 2):** `guard_tranche_vs_allocation` is private (`cmd/tranche.rs:107`); the chokepoint
  needs it `pub(crate)` — Task 1 says "(or `pub(crate)`)" for the promote helpers, Task 2 doesn't.
- **n-2 (Tasks 8/9):** shipped flow-state structs all live in `edit/form.rs` (`ClassifyInboundFlowState`
  at `form.rs:638`, etc.); new per-flow files deviate from the convention — acceptable, but say it's
  deliberate (module size) so review doesn't churn on it.
- **n-3 (Tasks 3/10):** `IrsPdfReport` lives at `cmd::admin` — tui-edit can't name it through a `cmd::`
  path; add it (and `Refusal`, the `*Plan` types) to the crate-root/chokepoint re-export list in the
  Task 1/3 lib.rs steps (the `pub use cmd::admin::promote_export_gate` precedent, `lib.rs:37`).

---

## What was verified sound (no finding)

- The gate ordering in Global Constraints matches `promote.rs:364-488` exactly (incl. consent printed
  before the ack — the N-2 contract); ack-inside-`apply` + `would_conflict`-in-`apply` match DFW-D2.
- `would_conflict`'s pseudo-off precedent is real (`project/mod.rs:118-120`); `ProjectionConfig` is
  `Copy`, so the "own copy" fix is mechanical. The DFW-D6 latent gap is real
  (`promote.rs:396` `session.config()?.to_projection()` feeds stored pseudo into `consent_terms`).
- `pool_key(date, wallet)` at `pools.rs:15`; `Blocker { event: Option<EventId>, detail, … }` supports
  event-level clearance; `LedgerState` extension is projection-only (not persisted).
- tui-edit already depends on btctax-cli (+ core, adapters) — Cargo.toml verified; `cmd::` is allowed
  in G1 test regions, so Task 9's parity tie-in can drive the CLI verb fn from tui-edit tests.
- `promote_cli.rs` has both fn-call and spawn-the-binary(+stdout capture) harness styles (`:126,551-560`)
  — Task 1/4's capture steps are executable.
- `ConsentTerm` (event.rs:331, `Eq`), `Acknowledgment` (event.rs:359), `Usd`, `TaxProfile`,
  `PriceProvider` all reachable at the paths the plan names; `chokepoint` module name unclaimed.
- Characterization-first (Task 1 Steps 1-2 PASS before refactor; Task 3 Steps 1-2 FAIL for new fns) is
  the right TDD polarity in both directions; phase gates are real review loops per the workflow.
