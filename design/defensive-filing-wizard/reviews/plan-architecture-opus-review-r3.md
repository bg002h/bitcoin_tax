# Plan review r3 — architecture lens (Opus)

**Artifact:** `design/defensive-filing-wizard/IMPLEMENTATION_PLAN.md` @ `89986ab` (post-r2-fold)
**Contract:** `design/defensive-filing-wizard/SPEC.md` (GREEN, DFW-D1..D12)
**Prior:** `reviews/plan-architecture-opus-review-r2.md` (Opus, NOT GREEN — 1C/2I/6m/1n).
**Reviewer:** Opus (software-architecture lens), independent, every load-bearing claim re-derived against
CURRENT source on `feat/defensive-filing-wizard` @ `89986ab` — no reliance on the plan's self-citations.

## Verdict

**NOT GREEN — 0 Critical / 1 Important / 4 Minor / 1 Nit**

The r2 fold is genuinely good: all nine r2 findings (C-1, I-1, I-2, m-1..m-6, n-1) are RESOLVED and hold
against source — the export `&Session` extraction is now real task-sized surgery with a characterization
test, the `PromotePlan` ordered fields reproduce the shipped stdout byte-for-byte, the `Shortfall`
fee/principal split and its three dependent advisories are derived + KAT-held, and every re-grepped line
citation (the six fold sites, `render_consent`'s signature, `IrsPdfReport`'s home, the KAT-G1 token
lists) matches. But one r1 finding regressed during the r1→r2 folds and is still open: **C-2's
core-predicate move (`tranche_guard.rs`) is described only in the File Structure Map and is scheduled in
NO task** — Task 6, its first consumer, composes `tranche_guard::in_force_allocation_exists` over a module
no step creates, so it cannot compile task-by-task, and the cross-crate surgery (move 3 fns cli→core +
rewire `cmd/tranche.rs`'s two guards over all four allocation append sites, behavior-preserving) is the
exact "hidden task-sized surgery" the lens exists to surface. This is the residue of r1's Critical C-2:
the File-Map half was folded, the task-scheduling half was not.

---

## r2-resolution audit

- **C-1 (export `&Session` extraction) — RESOLVED.** Verified `export_irs_pdf` (`admin.rs:350`) opens its
  own `Session` (`:358`), the full-vs-slice dispatch is `crate::return_inputs::exists(session.conn(), …)`
  (`:373`), and the crypto-slice body runs `:385-599` — using `session` for `session.resolve_screened`
  (`:460`, SE profile) and `session.donation_details()` (`:508`, Form 8283) beyond `state`/`events`, so a
  `&Session` inner is genuinely needed. Task 3 now schedules it as real surgery: Step 1-2 pin the shipped
  packet (characterization PASS-before-refactor), Step 3 (`:267-271`) extracts `Session::open` + dispatch
  + slice body into `export_irs_pdf_from_session(session: &Session, state, events, out_dir, tax_year,
  forms, attest) -> Result<IrsPdfReport, CliError>` (`:247-248`) mirroring the already-`&Session`
  `export_full_return` (`admin.rs:642`), leaving `export_irs_pdf(vault_path, pp, …)` a thin opener. The
  signature is coherent and compilable; `apply_export(session: &Session, …)` (`:253`) composes over the
  TUI's already-open session — no second `Session::open`, so the `VaultLock` deadlock
  (`session.rs:~660`, "a second open would deadlock on the held VaultLock") is avoided. `IrsPdfReport`
  re-export is present (n-1). *(New Minor m-new-1 + Nit on the dispatch-location prose — see below — but
  the deadlock hazard and the executability are genuinely fixed.)*

- **I-1 (`PromotePlan` ordered fields reproduce the shipped string) — RESOLVED.** Re-derived the shipped
  stdout order in `promote_tranche`: `for line in &advisory { println!(line) }` (`:443-445`, PRE-consent
  synthetic-promote advisory) → `println!("{}", render_consent(&terms, &gift_only_years))` (`:453`) →
  `if let Some(note) = wide_window_note(…) { println!(note) }` (`:454-456`, POST-consent) → ack (`:458`).
  `render_consent`'s real signature is `render_consent(terms: &[ConsentTerm], gift_only_years:
  &BTreeSet<i32>) -> String` (`:333`) and the gift-only relabel is applied per-term inside it
  (`render_term(term, gift_only_years)`, `:338`). The plan's `PromotePlan` carries the three ORDERED
  pieces — `advisory_lines: Vec<String>`, `gift_only_years: BTreeSet<i32>`, `post_consent_note:
  Option<String>` (`:135-139`) — and `render_consent(&plan)` re-emits `advisory_lines` → shipped
  `render_consent(&plan.terms, &plan.gift_only_years)` → `post_consent_note` (`:147`,:168-172), which is
  byte-identical to the shipped order. `gift_only_years` is honored as the shipped INPUT (not a
  pre-rendered string), and `gift_only_flagged_years(…) -> BTreeSet<i32>` (`:216`) confirms the type. The
  single-flat-Vec collapse r2 flagged is explicitly forbidden (`:170-172`). Byte-parity (the P-A gate) is
  achievable.

- **I-2 (`Shortfall` fee/principal split + three derived advisories) — RESOLVED.** (1) `Shortfall { event,
  wallet, date, short_sat, fee_sat }` (`:313-314`) now carries `fee_sat` (principal = `short_sat -
  fee_sat`); the raw fold records carry `{…, principal_sat, fee_sat}` (`:76`). Re-verified the
  decomposition is in scope at each of the six sites: the fee site is a pure fee short (`consume_fee` →
  `consume_fifo(key, fee_sat)`, `fold.rs:385-390`), the principal sites are pure principal
  (`consume_principal(… *sat …)` with `wallet`/`date`/`eff.id` in scope, e.g. `:706-712`), and the one
  lumped site (pending-out `total_sat = *sat + fee_sat.unwrap_or(0)`, `:827-828`) is routed to
  `ResolveFirst` via `UnmatchedOutflows` (DFW-D4), so it never becomes a tranche-covered shortfall and
  cannot corrupt `FeeOnlyPromoteNoop`. (2) All three advisories are now DERIVED in Task 6 Step 3 (`:399-401`)
  and KAT-held: `FeeOnlyPromoteNoop` (KAT `:389`), `MethodInversion` (KAT `:390`), `TrancheDip` (KAT
  `:391`). The shipped source fns exist and are pure/core-derivable: `method_inversion_advisory(state:
  &LedgerState, wallet: &WalletId, method: LotMethod) -> Option<String>` (`conservative.rs:61`),
  `tranche_dip_advisory(disposal: &Disposal) -> Option<String>` (`conservative.rs:27`). No dead variants.

- **m-1 (persist.rs in Tasks 8/9/10 Files headers) — RESOLVED.** Task 8 (`:441-443`), Task 9 (`:466-468`),
  Task 10 (`:493-494`) Files headers now each list `crates/btctax-tui-edit/src/edit/persist.rs` with the
  named wrapper + KAT-G1 allowlist note.

- **m-2 (e10 → KAT-G1) — RESOLVED.** Task 7 Step 3 (`:429`) explicitly corrects "the tui-edit gate is
  KAT-G1 (`persist.rs:1897`), NOT e10 (that gate is btctax-tui's)"; Task 7 Step 4 (`:430`) and Task 9
  Step 4 (`:482`) now say "incl. KAT-G1's `kat_g1_mechanized_source_gate`". Verified the real gate name is
  `kat_g1_mechanized_source_gate` (`persist.rs:1897`).

- **m-3 (PoolShort dashboard-render KAT) — RESOLVED.** Task 7 Step 1(f) (`:423`) adds the dashboard-render
  KAT for the `PoolShort` "still short by S — don't declare again" row, distinct from the Task-6 view-level
  KAT (`:388`).

- **m-4 (`flagged_years()` not the `Vec<String>` advisory) — RESOLVED.** Task 6 Step 3 (`:398`) now calls
  "`flagged_years()` (★ arch-m-4: the STRUCTURED `BTreeSet` fn from Task 3 … NOT the `Vec<String>`
  `promote_prior_year_advisory`, else the banned string-parse re-enters)". Confirmed `flagged_years` does
  not yet exist and `promote_prior_year_advisory` returns display strings (`conservative.rs:689`).

- **m-5 (emit-site line drift) — RESOLVED.** Re-grepped `fold.rs`: the six sat-carrying
  `BlockerKind::UncoveredDisposal` sites are EXACTLY `:388`(fee)`,710`(dispose)`,831`(pending-out)`,876`
  (self-transfer)`,1196`(gift-out)`,1274`(donate) — matching the plan (`:71-78`, `:323`, `:341`). The
  r2 off-by-2 drift (`:712,833,878,1198,1276`) is gone; those are the `"… short by …"` message lines ~2
  below each blocker, exactly as the plan notes. The other `UncoveredDisposal` emits (`:691,742,819,864,
  935,1177,1225,1255,1303`) are the without-wallet/degenerate (data-fix) variants — correctly excluded.

- **m-6 (`Refusal::Target` for the resolve-live gate) — RESOLVED.** `Refusal = { Target(String),
  Provenance(String), Coverage(String), PartII(String) }` (`:143`); `Target` maps the FIRST gate
  (`resolve_live_tranche`, `promote.rs:95`, which returns `CliError` for absent/wrong-type/voided). The
  plan correctly drops `would_conflict` as a plan `Refusal` because it is apply-time → `CliError`
  (verified `would_conflict` sits inside the pipeline at `promote.rs:477`, after the ack).

- **n-1 (`IrsPdfReport` crate-root re-export) — RESOLVED.** `IrsPdfReport` lives at `cmd::admin`
  (`admin.rs:261`); the plan adds `pub use crate::cmd::admin::IrsPdfReport;` at the cli crate root
  (`:252`), precedent `pub use cmd::admin::promote_export_gate;` (`lib.rs:37`). So `persist.rs`'s
  `persist_defensive_export` names `btctax_cli::IrsPdfReport` — no `cmd::` token (the KAT-G1
  `everywhere_tokens` ban at `persist.rs:1920`). *(Note: `FormArg`, which the TUI must construct for
  `ExportPlan.forms`, lives at `btctax_cli::cli::FormArg` (`cli.rs:982`) — a `cli::` path, not `cmd::`, so
  it is already gate-safe and needs no re-export.)*

---

## Important

### I-new-1 — C-2's core-predicate move (`tranche_guard.rs`) is specified only in the File Map and scheduled in NO task; Task 6 (its first consumer) cannot compile

The File Structure Map (`:62-70`) specifies the C-2 fix in full: **Create**
`crates/btctax-core/src/tranche_guard.rs` with `void_targets(events)` / `in_force_allocation_exists(events)`
/ `pre2025_tranche_exists(events)` moved out of `cmd/tranche.rs`, **Modify**
`crates/btctax-core/src/lib.rs` (`pub mod tranche_guard;`), and keep `cmd/tranche.rs`'s thin
`CliError`-wrapping guards over the core predicates ("single source preserved for all four allocation
append sites"). Verified this is a genuine cross-crate move: the three predicates today are
`void_targets` (`tranche.rs:40`, private), `in_force_allocation_exists` (`:54`, pub),
`pre2025_tranche_exists` (`:71`, pub), each taking only `events: &[LedgerEvent]` + using
`btctax_core::conventions::TRANSITION_DATE` (`:12`) — so they are core-movable — while
`guard_tranche_vs_allocation` (`:107`) / `guard_allocation_vs_tranche` (`:93`) wrap them and must stay in
cli.

The move is **consumed** but never **produced**:
- Task 6 Step 3 (`:402-403`) composes `tranche_guard::in_force_allocation_exists + pre2025_tranche_exists`
  in core `journey_view` "the CORE predicate, C-2, never the cli-private guard".
- Task 8 Step 1(d) (`:455`) reads "the CORE `tranche_guard::{pre2025_tranche_exists,
  in_force_allocation_exists}`, C-2".

But **no** task's Files header or Steps create `tranche_guard.rs`, rewire `cmd/tranche.rs`, or add
`pub mod tranche_guard`. Confirmed by grep: no `**Files:**` line names `tranche_guard.rs`; Task 5 Files
(`:308`) = "Create `discovery.rs`; Modify `state.rs`, `lib.rs`" (no tranche_guard, no `cmd/tranche.rs`);
Task 6 Files (`:348`) = "Create `defensive/mod.rs`" only. Task 5's steps build the shortfall signal; Task
6's steps build `journey_view` and merely *use* `tranche_guard::`.

**Failure scenario:** an implementer executing task-by-task (the plan's declared
`subagent-driven-development` model) reaches Task 6, writes `journey_view` composing
`tranche_guard::in_force_allocation_exists`, and it **does not compile** — the module does not exist and
core cannot reach the cli-private predicate (`btctax-cli` depends on `btctax-core`, not the reverse). To
proceed they must, mid-Task-6, perform an unscheduled cross-crate surgery: move three fns, rewire
`cmd/tranche.rs`'s two guards, preserve the shipped safe-harbor/allocation guard behavior across all four
allocation append sites, and add a behavior-preservation test — none of which is in Task 6's scope,
Files, or KAT list. This is precisely the hidden task-sized surgery the lens must surface. It is the
residue of r1's **Critical** C-2: the r1→r2 folds updated the File Map but never scheduled the work in a
task, so the executability gap that made it Critical in r1 is intact.

**Fix (r1-C-2's, finished this time):** add an explicit step — cleanest as a new sub-step in Task 5 (the
first CORE task, which already modifies core `lib.rs`) or a dedicated pre-P-B task — that: (1) creates
`crates/btctax-core/src/tranche_guard.rs` (the three predicates), (2) adds `pub mod tranche_guard;` to
core `lib.rs`, (3) rewires `cmd/tranche.rs` so `guard_tranche_vs_allocation`/`guard_allocation_vs_tranche`
call `btctax_core::tranche_guard::*` and deletes the cli copies, (4) pins behavior with a
characterization test that keeps the shipped allocation-guard KATs green, and (5) lists
`crates/btctax-core/src/tranche_guard.rs`, `crates/btctax-core/src/lib.rs`, and
`crates/btctax-cli/src/cmd/tranche.rs` in that task's Files header.

---

## Minor

- **m-new-1 (Task 3 — dispatch-location is internally inconsistent; one reading silently drops a shipped
  tax guard untested).** Step 3 (`:267-271`) + the interface (`:247-249`, "thin opener: Session::open →
  export_irs_pdf_from_session") put the `return_inputs::exists` dispatch INSIDE
  `export_irs_pdf_from_session` — correct and byte-parity-preserving (`export_irs_pdf` stays
  `open → _from_session`, which internally branches slice-vs-full exactly as shipped `admin.rs:373-381`).
  But the interface prose (`:256-260`) and Step 6 (`:280`) describe `apply_export` as itself "dispatching
  each year through `export_irs_pdf_from_session` (crypto slice) **or** `export_full_return` … via the
  `return_inputs::exists` check MOVED into the chokepoint" — the opposite factoring (dispatch in
  `apply_export`, `_from_session` = slice-only). If an implementer follows that prose *and* the literal
  `:249` thin-opener, `export_irs_pdf` loses its full-return dispatch and silently emits a crypto slice on
  a full-return year (the P5-C1 chimera the shipped dispatch prevents, `admin.rs:361-372`) — and the Step-1
  characterization vault (a promoted-disposal vault, `return_inputs::exists == false`) exercises only the
  slice path, so **no test catches it**. The Step-3 reading is correct; reconcile `:256-260`/`:280` to it
  (both `export_irs_pdf` and `apply_export` just call `_from_session` per year; the dispatch lives once,
  inside `_from_session`), and add a full-return-year case to the Step-1 characterization so the retained
  dispatch is pinned.

- **m-new-2 (Files-header manifest omissions — same class as the folded r2 m-1, not applied to the
  cross-crate files).** These tasks have no `git add` line, so the Files header *is* the manifest, yet:
  Task 3 Files (`:231`) omit `crates/btctax-core/src/conservative.rs` (where Step 6 creates
  `flagged_years`, interface `:236-238`) and `crates/btctax-cli/src/lib.rs` (the `IrsPdfReport` re-export,
  `:252`); Task 5 Files (`:308`) omit `crates/btctax-core/src/project/fold.rs` (where Step 3 populates
  `state.shortfalls` at the six sites, `:340-342`) and the `defensive/mod.rs` skeleton that
  `defensive/discovery.rs` needs to compile as `defensive::discovery` (the File Map assigns `mod.rs` to
  Task 6). The Steps make each edit unmissable, so this is Minor — but add the files to the headers for
  grep-able reconciliation, consistent with the r2 m-1 fold.

- **m-new-3 (`plan_promote`'s `state: &LedgerState` param is dead).** `plan_promote(events, state, prices,
  cfg, target, provenance, part_ii, now)` (`:144-146`) carries `state`, but the shipped promote pipeline
  consumes `events`/`prices`/`cfg` and never a pre-built `LedgerState` (re-verified `promote_tranche`,
  `promote.rs:364-488`: `resolve_live_tranche(events)`, `consent_terms(events,…)`,
  `promote_prior_year_advisory(with_events,…)`, `gift_only_flagged_years(…events, with_events)`,
  `would_conflict(events,…)` — no state). This is r1-m-6's dead-param half, unfolded (the r2 fold took only
  the `Refusal::Target` half). An unused param is a clippy `unused_variables` warning under `make check`
  and misleads (it invites reading a possibly pseudo-active state). Drop it, or keep `_state` for
  signature symmetry with `plan_declare`/`plan_export` and note it is intentionally unused. (Check
  `plan_declare`'s `state` (`:204`) the same way — its `None` path replicates `tranche.rs:134-154`, which
  is `events`-only, and the `Some` clearance re-projects, so it too may be dead.)

- **m-new-4 (`journey_view`'s `state` param invites a DFW-D6 discovery violation).** `journey_view(events,
  state, prices, tables, cfg)` (`:362-363`) derives discovery via `shortfalls(state)` (Task 5 `:317`,
  reads `state.shortfalls`). If the passed `state` is a pseudo-active projection, its `shortfalls` are
  synthetic-cleared (a Phase-B `SelfTransferMine{$0}` lot masks a real short — SPEC DFW-D6), silently
  under-reporting candidates. The dashboard's `!state.pseudo_active()` entry gate (Task 7 Step 1(a)) is
  the only caller and prevents the bad path, and `journey_view_forces_pseudo_off` (`:392`) holds the
  behavior — but the signature still invites it, and Task 6 Step 3's "all shadow projections force
  pseudo-off" does not obviously cover the *discovery* read off the passed state. Mandate that
  `journey_view` re-derives the shortfall signal from an internal `pseudo_reconcile=false` re-projection
  (not the passed `state.shortfalls`), or state the precondition + assert it (r1-m-3's fix).

## Nit

- **n-new-1 (Task 3 Step 3 wording).** "move `admin.rs`'s `Session::open` (`:358`) + … into a `&Session`
  inner" reads literally as moving the `open` INTO the fn that receives an already-open `&Session`
  (impossible). The interface (`:249`, "`export_irs_pdf(vault_path, pp, …)` becomes a THIN opener:
  `Session::open → export_irs_pdf_from_session`") disambiguates — the `open` stays in the thin opener.
  Reword Step 3 to "the thin `export_irs_pdf` keeps `Session::open` and calls the inner; the dispatch +
  slice body move into `_from_session`."

---

## Verified sound (no finding)

- **Gate ordering** in Global Constraints (`:25-28`) matches the shipped pipeline exactly: resolve-live
  (`promote.rs:378`) → BG-D5 provenance (`:381`) → BG-D7 Part II (`:386`) → BG-D3 floor (`:397`) → BG-D6
  `consent_terms` (`:410`) → synthetic-promote advisory (`:443`) → gift-only relabel (`:449`) → consent
  render + `wide_window_note` (`:453-456`) → `require_promote_ack` (`:458`) → `would_conflict` (`:477`) →
  append (`:485`).
- **Ack-inside-`apply` / `would_conflict`-in-`apply`** honored: both sit after the consent render in the
  shipped order, so extracting them into `apply_promote` (`:148-149`) preserves DFW-D2's residency. The
  consent-printed-before-ack contract (a refused ack still surfaces the figures, `promote.rs:451-456`) is
  what the refused-ack parity KAT (Task 4 `:293`) exercises.
- **Seam integrity:** every core signature takes `&dyn TaxTables` (`journey_view` `:363`, `flagged_years`
  `:237`, `plan_export` `:243`) — never `BundledTaxTables`; the flavor gate is `tables.table_for(y)`. No
  core fn names a `btctax-cli` symbol. The tui-edit drivers reach the chokepoint via
  `btctax_cli::chokepoint::*` (crate-root, module added at `lib.rs`) and `btctax_cli::IrsPdfReport`
  (re-export) — no `cmd::` path leak past the KAT-G1 `everywhere_tokens` ban (`persist.rs:1920`).
- **Write confinement (KAT-G1):** verified the gate at `persist.rs:1897`, `everywhere_tokens` (`:1919-1928`,
  incl. `cmd::`, `export_snapshot`/`write_csv_exports`/`write_form_csvs`), `fs_write_tokens` (`:1932-1947`),
  `persist_only_tokens` (`:1955-1963` = `conn(`/`save(`/`tax_profile::set`/`append_`/`donation_details::set`/
  `optimize_attest::set`/`restore(`), and the self-check plant region (`:2141-2190`). The plan's C-3 fold
  (adds the three `persist_*` wrappers, extends `persist_only_tokens` with `apply_declare(`/`apply_promote(`/
  `apply_export(`, amends the `edit/mod.rs:1-9` guarantee for the editor's new chokepoint-only export
  surface, plants a self-check token) is the correct shape — and because the PDF-writing `fsperms` calls
  live in `btctax-cli` (`admin.rs` `write_bytes_owner_only`/`mkdir_out`), not tui-edit, they are outside
  the gate's scan and the new export surface trips neither `fs_write_tokens` nor the `export_snapshot`
  family. `apply_export` taking `&Session` (not `&mut`) is correct: export reads the session and writes
  the filesystem, mutating no events; `apply_declare`/`apply_promote` correctly take `&mut Session`.
- **The DFW-D6 pseudo-off fix** is mechanical and real: `promote.rs:396` `session.config()?.to_projection()`
  feeds the stored `pseudo_reconcile` into `consent_terms`/`promote_prior_year_advisory`/
  `gift_only_flagged_years`; `ProjectionConfig` sets `pseudo_reconcile=false` as `would_conflict` does
  (`project/mod.rs:119`), and it is `Copy`, so `plan_promote`'s own-copy fix (`:151-153`) mirrors it.
- **Characterization polarity:** Task 1/Task 3 pin the shipped output PASS-before-refactor; Task 5/6 KATs
  FAIL-for-new. Phase gates are real two-lens loops. Task right-sizing holds — EXCEPT the C-2 predicate
  move, which is task-sized and un-surfaced (I-new-1).
- **Parity phasing:** Task 4 (P-A) drives the CLI verb + the raw chokepoint (the only full driver that
  exists in P-A); Task 9 Step 1(a) ties in the TUI persist path once it exists (P-C) — together they
  satisfy SPEC DFW-D2's "both full driver paths". Coherent, not a gap.
