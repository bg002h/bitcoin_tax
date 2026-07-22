# Defensive Filing Wizard — IMPLEMENTATION_PLAN tax-correctness review (Opus, r3)

Lens: US-federal-tax-correctness. Independent re-derivation from CURRENT source at plan HEAD `89986ab`
(the plan self-citations/line numbers were NOT trusted — every claim below re-grepped). Sources verified:
`cmd/promote.rs:300-488` (pipeline, gate order, render_consent, ack, would_conflict),
`cmd/promote.rs:85-118` (`is_voided`/`resolve_live_tranche`), `cmd/promote.rs:153,172,216` (movable helpers),
`cmd/tranche.rs:40,54,71,107,125-175` (predicates + declare gate set + phantom-wallet eprintln),
`cmd/admin.rs:78-116,261,350-409,616-698` (export gate/enumeration/dispatch/`export_full_return` &Session),
`conservative.rs:27,61,689-758` (advisories + fold-diff year machinery),
`conservative_promote.rs:50-69,89,258-273,487` (`filed_basis_for`/Coverage/drift/consent_terms/clamped),
`project/mod.rs:107-119` (`would_conflict` pseudo-off), `project/fold.rs:388,710,831,876,1196,1274`
(the six `UncoveredDisposal` emit sites), `pools.rs:15` (`pool_key`), `session.rs:658-666` (VaultLock deadlock).

## Verdict

**GREEN — 0 Critical / 0 Important / 1 Minor / 1 Nit**

The plan implements the GREEN SPEC (DFW-D1..D12) without weakening any binding decision. It adds NO new
tax logic: every filed number still flows through the characterization-pinned shipped primitives
(`consent_terms`, `filed_basis_for`, `promote_prior_year_advisory`, `append_decision`,
`export_irs_pdf`/`export_full_return`). The DFW-D6 pseudo-off correction is the ONE intended behavior
change and is correctly scoped to the buggy pseudo-active path. Over-coverage is a derived dashboard
advisory, not a chokepoint gate — the shared promote gate stays behavior-preserving. All four flagged
audit areas re-derive clean; the two findings are non-gating.

## r2-resolution audit — all 3 Minor + 1 Nit folded correctly

- **r2 M-1 RESOLVED (fee/principal `kind` unbuildable → clean `fee_sat` field).** The old `Shortfall`
  had no `kind`, so the principal-vs-fee distinction `FeeOnlyPromoteNoop` needs was dropped. The current
  plan replaces `kind` with an explicit **`fee_sat: i64`** on `Shortfall` (plan:313-314), populated from
  raw `{…,principal_sat,fee_sat}` records, aggregated per event into
  `Shortfall{short_sat = Σ(principal+fee), fee_sat = Σ fee}` (plan:76-79, 323, 340-342). Re-verified
  buildable at source: the six emit sites cleanly partition — `fold.rs:388` is the **fee** short
  ("self-transfer/gift fee short", inside the `fee_sat<=0` fee-carry fn), while `:710`/`:876`/`:1196`/`:1274`
  are **principal** shorts (from `consume_principal`). So a pure-fee short → `fee_sat==short_sat`, a
  pure-principal short → `fee_sat==0`, and a gift/donate event that co-emits both (`:1196`/`:1274` principal
  + `:388` fee on the same `eff.id`) sums to `short_sat=principal+fee, fee_sat=fee`. Three new core KATs
  now pin exactly this: `fee_only_short_has_fee_sat_equal_short_sat`,
  `principal_plus_fee_short_on_one_event_aggregate_to_one_shortfall` (Task 5), and
  `fee_only_coverage_tranche_shows_fee_only_promote_noop` (Task 6). The r2 fix is cleaner than the `kind`
  enum I proposed. **Correct.**

- **r2 M-2 RESOLVED (`still_short` residual value now asserted).** Task 6's
  `a_live_tranche_not_clearing_its_pool_shows_pool_still_short` now reads "`assert_eq!` its `short_sat`
  AND `live_tranche_sat` (★ tax-M-2 — the residual value, not just the count)" (plan:388). A
  wrong-residual mutation now reds. **Correct.**

- **r2 M-3 RESOLVED (phantom-wallet stderr preservation stated).** Task 2 Step 4 now states the shipped
  `eprintln!` (`tranche.rs:159`) moves to the driver and is "kept emitted byte-for-byte on the `None`
  path (`declare_tranche_cli.rs` holds it); it is I/O, not gate logic, and must not migrate into the
  chokepoint" (plan:221-223). Re-verified at source: `tranche.rs:159` IS the phantom-wallet warning,
  emitted AFTER `guard_tranche_vs_allocation` at `:154` ("so a REFUSED declaration never emits the
  misleading stranded-lot note") — the driver, which runs `eprintln!` only on `Ok(plan)`, preserves that
  ordering. Tax-neutral ($0 tranche). **Correct.**

- **r2 N-1 RESOLVED (both halves).** (a) `Refusal::Conflict` is **gone** from the enum — it is now
  `{Target, Provenance, Coverage, PartII}` (plan:143) with an explicit comment that `would_conflict` is
  apply-time → `CliError`, not a plan `Refusal`. (b) The core predicate signature is corrected: the plan
  now says `pre2025_tranche_exists(events)` "takes only `events` — NO `we` arg" (plan:66-69), matching
  source `tranche.rs:71` (`pub fn pre2025_tranche_exists(events: &[LedgerEvent]) -> bool`),
  `:54` (`in_force_allocation_exists(events)`), `:40` (`void_targets(events)`, private). **Correct.**

## Re-derived audit of the four flagged areas

### 1. Fee/principal split + `FeeOnlyPromoteNoop`/`MethodInversion`/`TrancheDip` — tax-sound

- **The split is well-defined and buildable.** Re-verified the six `UncoveredDisposal` emit sites: five
  partition cleanly into principal-only or fee-only (above). The **one** wrinkle the plan itself flags is
  `fold.rs:831` (pending-out), which lumps `total_sat = *sat + fee_sat.unwrap_or(0)` into ONE
  `consume_fifo` shortfall (plan:78 notes this: "the split is per-record, not uniform across sites"). This
  is **harmless**: a pending-out short is routed to `ResolveFirst` (DFW-D4 exception, via its co-emitted
  `UnmatchedOutflows`), never a `DeclareCandidate`, so its ambiguous split never drives a declare prefill
  (`short_sat` magnitude) or a `FeeOnlyPromoteNoop` (which only fires on a tranche that COVERS a
  shortfall). The `pending_out_short_routes_through_unmatched_outflows_first` KAT (Task 5) pins the route.
- **A mis-derived/absent advisory misfiles nothing.** `FeeOnlyPromoteNoop` fires iff a covered shortfall
  has `short_sat == fee_sat` (all-fee). Per SPEC DFW-D3 (fee-only coverage) this is UX-only — fee-sats
  draw acquisition-date FIFO (method-independent) and BG-D4 fee-evaporation forfeits the estimate
  component, so promoting a fee-only-coverage tranche yields ~$0 regardless. The promote stays
  behavior-preserving either way, so a wrong advisory changes no filed number.
- **`method_inversion_advisory`/`tranche_dip_advisory` are surfaced with their REAL signatures.**
  Re-verified: `tranche_dip_advisory(disposal: &Disposal) -> Option<String>` (`conservative.rs:27`) and
  `method_inversion_advisory(state: &LedgerState, wallet: &WalletId, method: LotMethod) -> Option<String>`
  (`conservative.rs:61`) — the plan cites both at the correct lines (plan:355, 373-374). The plan's
  `Advisory::{MethodInversion(String), TrancheDip(String)}` carries the `Option<String>` output VERBATIM
  (SPEC DFW-D3 tax-N-2), and `journey_view` holds everything both need (`state.disposals` for the dip;
  `state` + `in_force_methods` for the wallet's method). Both are pure read-only advisories over
  already-projected state (the module doc: "PURE builders … no folding, no I/O … never instruct a
  tax-understating action") — no filed-number path. KATs
  `hifo_steered_promote_surfaces_method_inversion_advisory` / `tranche_dip_surfaces_on_tranche_row`
  (Task 6) assert present-verbatim + absent-when-not-inverting. **Tax-sound.**

### 2. `export_irs_pdf_from_session` (Task 3) — behavior-preserving for the filed packet

Re-verified `export_irs_pdf` (`admin.rs:350`): `Session::open` (`:358`) → `load_events_and_project`
(`:359`) → the `return_inputs::exists` dispatch (`:373`) → full path `export_full_return(&session,…)`
(`:375`, with `report.forms_ignored_full_return = !forms.is_empty()` at `:379`) OR crypto-slice body
(`:385` `promote_export_gate` → attest gate → `form_8949`/`schedule_d` → `disclosure_8275`/8275 →
write). The plan extracts the dispatch + slice into `export_irs_pdf_from_session(session: &Session,
state, events, out_dir, tax_year, forms, attest)` and leaves `export_irs_pdf` a thin opener (plan:247-249).
This exactly mirrors the already-`&Session` `export_full_return` (`admin.rs:642`: `fn
export_full_return(session: &Session, state, events, out_dir, tax_year, attest)` — takes `&Session` +
already-projected `state`/`events`, reads `session.conn()` for `return_inputs`, does NOT re-project). The
`forms` param is carried so `forms_ignored_full_return` survives. The deadlock rationale is REAL: verified
`session.rs:658-666` documents "a second open would deadlock on the held VaultLock" — so `apply_export`
composing over the TUI's already-open `&Session` is required, and the extraction is what makes it possible.
The filed packet (`form_8949`/`schedule_d`/`form_8275`) is produced by moved-verbatim code from the same
`(state, events, tax_year)`, and the Task-3 characterization KAT (Step 1/3) pins the packet + `8275`
presence + `IrsPdfReport` across the extraction. **Behavior-preserving.** The DRAFT-gate posture is intact
on both surfaces: `plan_export` refuses pseudo-active at plan time (DFW-D11 refuse+route), and the inner
`require_attestation(None)` at `admin.rs:391` remains a fail-CLOSED backstop should state ever go
pseudo-active between plan and apply (it refuses, writing zero bytes — never a fail-open fictional packet).

### 3. `PromotePlan` ordered fields (Task 1) — reproduce the §6664(c) artifact + print order

Re-verified the shipped print sequence in `promote_tranche`: (a) synthetic-promote advisory printed FIRST
(`for line in &advisory { println!("{line}") }`, `:443-445`); (b) `render_consent(&terms,
&gift_only_years)` (`:453`, the shipped `pub fn render_consent(terms, gift_only_years)` at `:333`); (c)
`wide_window_note` printed AFTER consent (`:454-456`). The recorded artifact is
`Acknowledgment.shown_terms = terms` (`:466-468`) — i.e. the `consent_terms` output ONLY, not the advisory
or note. The plan's `PromotePlan` captures exactly these three ordered pieces —
`advisory_lines`/`gift_only_years` (an INPUT to `render_consent`, NOT a pre-rendered string)/`post_consent_note`
(plan:132-140) — and `render_consent(&plan)` re-emits them in shipped order
(advisory_lines → `render_consent(&plan.terms,&plan.gift_only_years)` → post_consent_note), which is
byte-reproducible with the driver's single `println!` (the plan correctly forbids collapsing the three
into one flat Vec, since `terms` must sit BETWEEN the pre-advisory and the note — plan:168-172). The
characterization KAT (Task 1 Step 1) pins the full ordered transcript; the full-driver parity harness
(Task 4) asserts the recorded `shown_terms` structurally `Eq` across BOTH the CLI verb and the TUI persist
path, on happy/refused-ack/each-refusal. Since both surfaces drive the SAME `plan_promote` (which forces
`pseudo_reconcile=false` before `consent_terms`/`promote_prior_year_advisory`/`gift_only_flagged_years` —
re-verified all three consume `config` and `project` under it: `consent_terms` `conservative_promote.rs:264-273`,
`promote_prior_year_advisory` `conservative.rs:701/707`), the §6664(c) artifact is identical CLI↔TUI and
carries the honest pseudo-off terms. **Byte-parity story holds.**

### 4. `Refusal::Target` (resolve-live) / dropped `Conflict` (apply-time) — every tax-relevant gate covered

Enumerated all six shipped promote refuse points and confirmed each has a home:

| shipped gate | source | plan home |
|---|---|---|
| resolve-live (`resolve_live_tranche`) | `promote.rs:378`/`:95-118` | `Refusal::Target` (plan-time) |
| BG-D5 provenance | `:381` | `Refusal::Provenance` (plan-time) |
| BG-D7 Part II empty | `:386` | `Refusal::PartII` (plan-time) |
| BG-D3 floor/coverage (`filed_basis_for`) | `:397`→`refuse_no_floor` | `Refusal::Coverage` (plan-time) |
| BG-D6 ack (`require_promote_ack`) | `:458` | inside `apply_promote` → `CliError` |
| BG-D9 `would_conflict` | `:477` | inside `apply_promote` → `CliError` (dropped as a `Refusal`) |

No tax-relevant gate is lost: the two apply-time gates (ack + would_conflict) correctly stay INSIDE
`apply_promote`, matching the shipped enforcement point (ack single-sourced, would_conflict fail-closed
before append). `filed_basis_for`'s Coverage enum is `{Full, Partial}` with NO `None` variant
(re-verified `conservative_promote.rs:56-69`), so `Refusal::Coverage` is the correct single home for both
`Partial`/no-floor outcomes. **Plan-time refusal set is complete.**

## Findings

### Minor

- **M-1 (Task 3, `flagged_years` multi-promote completeness, DFW-D11) — the export-set superset is only
  KAT-pinned for a SINGLE live promote.** `flagged_years(events, state, prices, tables, cfg) -> BTreeSet<i32>`
  (plan:237-238) drops the `promote_id`/`direction` params of the shipped
  `promote_prior_year_advisory` (`conservative.rs:689-698`), so its per-promote iteration is implicit, and
  Task 3 Step 4's only KAT uses ONE promote (a 2016 tranche + one 2025 donation). DFW-D11's guarantee is
  "the fold-diff–flagged prior years **across all live promotes**" — the tax hazard being that a filed
  prior year silently keeps a now-wrong 8949/§170(e)/§1015 leg. A whole-state "with-all vs without-all
  promotes" single diff could theoretically drop a year if two promotes' per-year leg effects cancel;
  the safe implementation is the **union of per-promote diffs** (exactly `promote_prior_year_advisory`'s
  per-`promote_id` semantics, which the plan's own wording "the fold-diff enumeration via
  `promote_prior_year_advisory`" (plan:277) already points at). **Non-gating:** the export set is unioned
  with `{current}` and is already a strict superset of `promoted_filing_years`, and in the realistic
  basis-monotonic case (promotes only raise a floor) it errs toward OVER-export (a spurious extra 1040-X
  packet is not a misfiling; a missing one is) — and the audience's overwhelming shape is a single
  tranche. **Fix:** state that `flagged_years` unions the per-promote fold-diffs (not a single
  whole-state diff), and add a two-live-promote KAT where each promote flags a DIFFERENT removal-reordered
  prior year, asserting both years are in `plan_export.years`.

### Nit

- **N-1 (Task 1, `Refusal::Target` doc) — "already-promoted target" is mis-attributed to the resolve-live
  gate.** The plan comments that `Refusal::Target` "covers the FIRST gate (resolve-live: unknown/voided/
  **already-promoted** target, promote.rs:377)" (plan:141-142). Re-verified source: `resolve_live_tranche`
  (`promote.rs:95-118`) checks only `is_voided` + wrong-type + absent — a DOUBLE-promote (target already
  promoted) is NOT caught here; it is caught by `would_conflict` ("a second live promote on this target",
  `:475-483`) at APPLY time. Gate coverage is fully preserved (the double-promote still refuses, at
  `apply_promote` → `CliError`, exactly as shipped — and the shipped CLI also prints consent before that
  refusal, so no behavior differs); only the comment's attribution is wrong. **Fix:** reword to
  "unknown/voided/wrong-type target" and note already-promoted is the `would_conflict` apply-time refusal.

## Confirmed correct (adversarially checked, no finding)

- **DFW-D6 pseudo-off is the only behavior change, correctly scoped.** `plan_promote` forces
  `cfg.pseudo_reconcile=false` on its own copy before the three consumers; re-verified the shipped
  `promote_tranche:396` uses the stored `cfg` un-forced, so the fix is real and needed, and it is a no-op
  on non-pseudo vaults (where `pseudo_reconcile` is already false) — no CORRECT filed number changes.
  Mirrors `would_conflict` (`project/mod.rs:119` sets `cfg.pseudo_reconcile=false`).
- **Over-coverage stays a derived advisory, gate untouched.** `OverCovered{by_sat}` is derived in
  `journey_view` (core, read-only) scoped to `covered_sat>0 ∧ live_sat>covered_sat` with the
  fully-undisposed carve — no guard is added to the shared promote chokepoint; the `mixed_vintage` /
  undisposed / correctly-sized promotes stay behavior-preserving (Task 6 KATs + shipped `promote_cli.rs`
  green). This honors the SPEC r3/r4 adjudication (ledger-identical to a legitimate reorder → cannot
  hard-refuse). No filed-number change.
- **Declare `None`-path byte-for-byte.** `plan_declare(None)` replicates the shipped gate set
  (`sat>0`, `ws<=we`, `guard_tranche_vs_allocation` — re-verified `tranche.rs:135-154`); the `Some` path's
  clearance shadow forces pseudo off (arch-I-5) so a synthetic `SelfTransferMine{$0}` cannot falsely clear,
  and the checked candidate == the appended `plan.payload` (no plan/apply divergence).
- **Flavor gate consistency** (carried from r2): `journey_view`'s three-flavor `SavingFlavor` gate
  (table∈{2017,2024,2025,2026} ∧ stored `TaxProfile` ∧ no Hard blocker) matches the `consent_terms`
  discipline and `clamped_promote_year_saving`'s `_ => Usd::ZERO` fallback, so no bare `ComputedTax{$0}` /
  bare `$X` for a non-computing year (pinned by `table_year_with_no_TaxProfile_shows_uncomputable…`).
- **C-2 predicate move is a READ-only precheck.** Moving `void_targets`/`in_force_allocation_exists`/
  `pre2025_tranche_exists` (events-only) to `btctax-core::tranche_guard` leaves the mutating
  `guard_tranche_vs_allocation` gate in the CLI (`tranche.rs:107`); `journey_view`'s `safe_harbor_blocked`
  is a read-only derivation, not a second gating authority.
- **Write confinement is filed-number-neutral.** Routing every mutation through `persist_*` wrappers
  (KAT-G1) changes no filed number — every filed number still flows through the characterization-pinned
  `apply_*` composing the shipped verbs.
