# Plan review r2 — architecture lens (Opus)

**Artifact:** `design/defensive-filing-wizard/IMPLEMENTATION_PLAN.md` @ 333f79b (post-fold)
**Contract:** `design/defensive-filing-wizard/SPEC.md` (GREEN, DFW-D1..D12)
**Prior:** `reviews/plan-architecture-fable-review-r1.md` (Fable, 3C/6I) — all folded per author.
**Reviewer:** Opus (software-architecture lens), independent, re-derived against source @ `feat/conservative-filing`. Different model from r1; not anchored.

## Verdict

**NOT GREEN** — **1 Critical / 2 Important / 6 Minor / 1 Nit**

The spine is sound and the C-1/C-2/C-3/I-1/I-3 folds genuinely hold against source. But the EXPORT trio (Task 3/10) asserts a `&Session`-parameterized shape the shipped code does not have — reintroducing the exact TUI `VaultLock` deadlock the chokepoint pattern exists to prevent — and two r1 Importants (I-2 byte-parity, I-4 fee/principal) are only partially folded: the concrete types cannot express what the prose promises.

---

## r1-resolution audit

- **C-1 (`&dyn TaxTables`) — RESOLVED.** File Map (`:55-58`), Task 6 sig (`:306-307`), `flagged_years` (`:206`), `plan_export` (`:211-212`) all take `&dyn TaxTables`. No `BundledTaxTables` in any core signature. Flavor gate = `tables.table_for(y).is_some()`. Matches the shipped core convention (`consent_terms`/`clamped_promote_year_saving`/`promote_prior_year_advisory`).
- **C-2 (core→cli inversion) — RESOLVED.** Verified against `cmd/tranche.rs`: `void_targets` (`:41`, returns `BTreeSet<EventId>`), `in_force_allocation_exists` (`:54`), `pre2025_tranche_exists` (`:71`) use only core types + `btctax_core::conventions::TRANSITION_DATE` (`tranche.rs:12` — genuinely core). They carry no `CliError`; the `CliError`-wrapping guards (`guard_tranche_vs_allocation :107`, `guard_allocation_vs_tranche :95`) stay in cli over them. Plan moves the three to core `tranche_guard.rs` (`:62-67`), cli keeps thin guards, `journey_view`/declare-flow read the CORE predicate (`:336,384`). Single gating authority preserved.
- **C-3 (KAT-G1 write-confinement) — RESOLVED IN SUBSTANCE (Minor residue).** Verified gate at `edit/persist.rs:~1897` (`kat_g1_mechanized_source_gate`), `persist_only_tokens = {conn(, save(, tax_profile::set, append_, donation_details::set, optimize_attest::set, restore(}`, `everywhere_tokens` bans `cmd::`, `edit/mod.rs:1-9` guarantee text. Plan adds `persist_declare_tranche`/`persist_promote_tranche`/`persist_defensive_export` wrappers, extends the allowlist with `apply_declare(`/`apply_promote(`/`apply_export(`, amends the guarantee text, plants a self-check token, and adds `persist.rs` to the File Map. Chokepoint reached via `btctax_cli::chokepoint::*` (module is crate-root, not `cmd::` — no leak). Residue → m-1, m-2.
- **I-1 (structured year-set) — RESOLVED.** `flagged_years(...) -> BTreeSet<i32>` in core `conservative.rs` (`:206`); `promoted_filing_years(state)` (`:209`) extracted from `admin.rs:84-98` (verified: the `None`-arm loop over `state.disposals` ∩ `promoted_origins`); `plan_export.years = {current} ∪ flagged_years`, strictly ⊇ `promoted_filing_years`. Single core source; no promote_id param so it unions all live promotes. (Task 6 Step 3 prose slip → m-4.)
- **I-2 (driver drops advisory) — NOT RESOLVED.** See Important-1.
- **I-3 (per-tranche `DidNotCover`) — RESOLVED.** `TrancheStatus = {DeclaredZero, Promoted}` (no `DidNotCover`); pool-level `PoolShort{pool, short_sat, live_tranche_sat}` (`:302`) on `DefensiveFilingView.still_short`; Task 6 derives one `PoolShort` per pool "no per-tranche attribution" (`:316-318`); KAT `:329`.
- **I-4 (fee/principal aggregate) — NOT RESOLVED.** See Important-2.
- **I-5 (four §5 KATs) — MOSTLY RESOLVED.** (a) pending-out→`UnmatchedOutflows` first (Task 5 `:280`); (c) cleared tranche removes the row (Task 2 `:195`); (d) clearance-shadow pseudo-off (Task 2 `:195`). (b) pool "still short" render has a view KAT (Task 6 `:329`) but no dashboard-render KAT (Task 7) → m-3.
- **I-6 (export trio placeholder) — types RESOLVED, executability NOT.** `ExportPlan{years,out_dir,forms}`, `plan_export`, `apply_export -> Vec<IrsPdfReport>` all defined — but the composition is unbuildable as written → Critical-1.

---

## Critical

### C-1 (new) — Task 3/10: `apply_export` asserts a `&Session` export path that the shipped code does not have; composing as-is deadlocks the TUI

Task 3 (`:214-220`) specifies `apply_export(session: &mut Session, plan)` writing "each via the shipped gated `export_irs_pdf`/full-return path, **parameterized over `&Session`/state, no re-`Session::open`**." Verified against source:
- `export_full_return(session: &Session, state, events, …)` (`admin.rs:642`) **is** already `&Session`-parameterized. ✓
- `export_irs_pdf(vault_path: &Path, pp, …)` (`admin.rs:350`) **opens its own `Session`** (`:358`) and its **crypto-slice body is inline** (`:385-578`, incl. the `promote_export_gate` per-year gate, `form_8949`/`schedule_d`, and the `disclosure_8275` emit) — NOT extracted into a `&Session` fn. The full-vs-slice **dispatch** (`return_inputs::exists`, `:373`) also lives inside the self-opening fn.

So the "parameterized over `&Session`" slice path **does not exist**; Task 3 does not schedule extracting `export_irs_pdf`'s `Session::open` + slice body + dispatch into a `&Session` inner (mirroring `export_full_return`). Task 3 Step 3 (`:227-229`) mentions only "the export plan/apply degenerate trio" + year-set + `promoted_filing_years`. This is the SAME surgery Task 1 spends a whole task on for promote — here it is hand-waved as "degenerate" (degenerate refers only to the absent consent/ack, NOT the Session-extraction). Consequences: (a) `apply_export` cannot compile taking `&Session` over `export_irs_pdf`; (b) if the implementer instead re-derives `vault_path` and calls `export_irs_pdf(vault_path, pp, …)` inside `apply_export`, it **compiles but deadlocks the editor at Task 10** — the held `VaultLock` vs a second `Session::open` (r1's own verified-sound note: cmd fns opening their own Session deadlock the editor; the chokepoint is the only workable shape, `session.rs:662`). This silently reintroduces the deadlock the whole chokepoint pattern exists to prevent, and the export path gets no characterization test (Task 3 Step 4 only runs `promote_cli.rs`/census — behavior-preservation of the extracted slice is unproven).

**Fix:** add a Task-3 step (with a characterization test, like Task 1) extracting `export_irs_pdf`'s `Session::open` + crypto-slice body + the `return_inputs::exists` dispatch into a `&Session`-parameterized inner (e.g. `export_irs_pdf_from_session(session: &Session, state, events, out_dir, year, forms, attest)`), leaving the shipped `export_irs_pdf(vault_path, pp, …)` a thin opener over it; `apply_export` composes that inner + `export_full_return`, both over the already-open `&Session`. `apply_export` may take `&Session` (export mutates no events).

---

## Important

### I-1 (r1-I-2 NOT resolved) — Task 1: `PromotePlan` cannot reproduce the shipped filer-visible string; byte-parity (the P-A gate) unachievable

Shipped stdout order (`promote.rs:437-458`): `for line in &advisory { println!(line) }` (synthetic-promote advisory, **pre-consent**) → `render_consent(&terms, &gift_only_years)` → `wide_window_note` (**post-consent**). Two structural facts the fold drops:
1. `render_consent`'s **real signature is `render_consent(terms: &[ConsentTerm], gift_only_years: &BTreeSet<i32>)`** (`promote.rs:333`) — the gift-only relabel is an INPUT that changes how terms render. `PromotePlan` (`:118-123`) has **no `gift_only_years` field**, so `render_consent(&plan)` (`:128`) cannot reproduce the relabeled consent.
2. `PromotePlan.advisory_lines: Vec<String>` is declared as "synthetic-promote advisory **+ `wide_window_note`**" (`:120,148`) — a single flat Vec conflating a **pre-consent** block and a **post-consent** note. `render_consent(&plan)` "renders the terms AND `plan.advisory_lines` in the shipped order" (`:146-148`) cannot place `terms` **between** them (advisory→terms→note); and Step 4's driver is only `println!(render_consent(&plan))` (`:151`), so everything must come from that one call. Neither `[advisory..note] then terms` nor `terms then [advisory..note]` equals the shipped `advisory → terms → note`.

The characterization/parity KAT (Task 1 Step 1, Task 4) WILL red, leaving the implementer to invent the split the plan was supposed to specify — which is exactly r1-I-2's fix, discarded in the fold.

**Fix (r1's, restore it):** `PromotePlan { advisory_lines: Vec<String> /*pre-consent*/, gift_only_years: BTreeSet<i32>, post_consent_note: Option<String> /*wide_window_note*/, terms, target, payload }`. `render_consent` = advisory_lines → `render_consent(terms, gift_only_years)` → post_consent_note (or the driver prints the three in order). Task 4 compares the full ordered transcript.

### I-2 (r1-I-4 NOT resolved) — Tasks 5/6: `Shortfall` cannot express fee-only; two Advisory variants are declared but never derived or KAT-held

Two coupled gaps:
1. **Data model.** File Map (`:70-72`) says `state.shortfalls` "retain[s] the principal-vs-fee `kind` (★ I-4 — `FeeOnlyPromoteNoop` needs it)", but the authoritative type — Task 5 `Shortfall { event, wallet, date, short_sat }` (`:261`) — has **no** `principal_sat`/`fee_sat`/`kind` field. Prose and type contradict; `shortfalls()` (`:264`) yields only the per-event `short_sat` aggregate. Verified the fold DOES distinguish them (`consume_fee` fee-short at `fold.rs:388`; `consume_principal` shorts at `:710,876,1196,1274`), so the split is derivable at the emit sites — but Task 5's struct discards it, and DFW-D3's `FeeOnlyPromoteNoop` (`:299`) then has no structured source but a banned `Blocker.detail` parse.
2. **Derivation + coverage.** Task 6 Step 3 (`:334-336`) derives only `OverCovered`/`NowDisplacing`/`PoolShort`. Two Advisory variants — `FeeOnlyPromoteNoop` (DFW-D3 fee-only-suppress) **and** `MethodInversion(String)` (DFW-D3 tax-N-2 "surface the shipped `method_inversion_advisory`/`tranche_dip_advisory` on tranche rows"; the shipped fns exist, `conservative.rs:61,27`) — are **declared in the enum but no task populates `TrancheRow.advisories` with them and no KAT holds either** (Task 6 KATs `:322-331` omit both; Task 7(e) only *renders* fee-only). `tranche_dip_advisory` isn't represented at all. Dead variants + an unbuilt binding-decision surface.

**Fix:** carry `principal_sat`/`fee_sat` (summed into `short_sat` for DFW-D7/D8 clearance) in the fold-populated record; derive `FeeOnlyPromoteNoop` iff all covered shortfalls are fee-component; wire `MethodInversion` (and `tranche_dip`) from the shipped advisories onto tranche rows in Task 6 Step 3; add a Task-6 KAT for each firing condition.

---

## Minor

- **m-1 (Tasks 8/9/10):** r1-C-3 required adding `edit/persist.rs` to Tasks 8–10's **file lists**; the File Map has it but the per-task **Files headers** (`:371,395,421`) still omit it (only the Steps `:387,409,430` mention it). Since Tasks 5–10 have no explicit `git add` line, the Files header IS the manifest — add `persist.rs` to each.
- **m-2 (Task 7/9):** r1-m-1 not folded — Task 7 Step 4 (`:360`) and Task 9 Step 4 (`:410`) still call the gate "e10 `mechanized_source_gate`"; the editor's gate is **KAT-G1** (`kat_g1_mechanized_source_gate`). e10 is btctax-tui's. Rename so "run — PASS (incl. …)" watches the right test.
- **m-3 (Task 7):** I-5(b) pool "still short — don't declare again" has a view-level KAT (Task 6 `:329`) but no dashboard-**render** KAT (Task 7 `:350-354` don't cover it). Add one.
- **m-4 (Task 6 Step 3):** `:335` says compose `promote_prior_year_advisory` "(export years)", but the field is `flagged_years: BTreeSet<i32>` (`:304`) and I-1's whole point is the structured `flagged_years()` fn — Step 3 must call `flagged_years()`, not the `Vec<String>` advisory (else the banned string-parse re-enters). Prose slip; correct it.
- **m-5 (Task 5):** r1-m-5 not folded — the fold emit-site citations `:388,712,833,878,1198,1276` (File Map `:71`, Task 5 `:285`) drift from source `:388,710,831,876,1196,1274`. Off-by-2 except 388. Implementer must re-grep, not trust the list.
- **m-6 (Task 1):** r1-m-6 not folded — `Refusal = {Provenance, Coverage, PartII, Conflict}` (`:124`) has no variant for `resolve_live_tranche` failures (unknown/voided/already-promoted target — the FIRST gate, `promote.rs:377`). State where they map (`Conflict`?) so the parity KATs can cover the path.

## Nit

- **n-1 (Tasks 3/10):** r1-n-3 partially open — `apply_export -> Vec<IrsPdfReport>` names `IrsPdfReport`, which lives at `cmd::admin` (`admin.rs:261`); if `persist_defensive_export` returns it, `persist.rs` names a `cmd::` token (KAT-G1 `everywhere_tokens` bans `cmd::`). Resolve by re-exporting `IrsPdfReport` at the cli crate root (the `pub use cmd::admin::promote_export_gate` precedent, `lib.rs:37`) or having the wrapper return `()`/a count (the Task-10 KATs read the filesystem, not the return type).

---

## Verified sound (no finding)

- Gate ordering in Global Constraints matches `promote.rs:377-485` exactly (resolve-live → BG-D5 → BG-D7 → BG-D3 floor → `consent_terms` → synthetic-promote advisory → gift-only → consent render + `wide_window_note` → `require_promote_ack` → `would_conflict` → append).
- The DFW-D6 latent bug is real: `cmd/promote.rs:398` `session.config()?.to_projection()` feeds stored `pseudo_reconcile` into `consent_terms`/`promote_prior_year_advisory`/`gift_only_flagged_years`. `ProjectionConfig` `Copy`, so the `plan_promote` `pseudo_reconcile=false` own-copy fix is mechanical (mirrors `would_conflict`).
- `export_full_return(&Session, …)` already Session-parameterized (only the slice path needs C-1's extraction).
- `promoted_filing_years` extractable from the verified `admin.rs:84-98` `None`-arm loop; `promote_export_gate` at `admin.rs:78`.
- Characterization-first polarity (Task 1 PASS-before-refactor; Task 3 FAIL-for-new) correct; phase gates are real two-lens loops.
- Task right-sizing holds EXCEPT the export Session-extraction hidden inside Task 3 (C-1) — which is task-sized and should be surfaced as its own step.
