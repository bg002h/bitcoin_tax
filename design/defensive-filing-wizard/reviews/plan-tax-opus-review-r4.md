# Defensive Filing Wizard — IMPLEMENTATION_PLAN tax-correctness review (Opus, r4)

Lens: US-federal-tax-correctness. Independent re-derivation from CURRENT source at plan HEAD `9ceede7`
(the plan's self-citations / line numbers were NOT trusted — every load-bearing claim below was
re-grepped). Sources verified this round:
`cmd/promote.rs:85-118` (`resolve_live_tranche`/`is_voided`), `:333-341` (`render_consent`),
`:346-357` (`require_promote_ack`), `:364-488` (pipeline: gate order, print order at `:443-456`,
`shown_terms = terms` at `:466-469`, `would_conflict` at `:477-483`);
`cmd/tranche.rs:40` (`void_targets`, private), `:54` (`in_force_allocation_exists`, pub), `:71`
(`pre2025_tranche_exists`, pub), `:93` (`guard_allocation_vs_tranche`), `:107` (`guard_tranche_vs_allocation`),
`:134-165` (declare gate set + phantom-wallet `eprintln!`);
`cmd/admin.rs:78-116` (`promote_export_gate`; `None`-arm disposal-leg enum `:85-98`), `:350-381` (the
`return_inputs::exists` dispatch at `:373`, full-return delegation `:375`/`forms_ignored` `:379`, slice
body `:383+`), `:642` (`export_full_return(session: &Session, …)`);
`conservative.rs:27` (`tranche_dip_advisory`), `:61` (`method_inversion_advisory`), `:689-761`
(`promote_prior_year_advisory`: fold pair `:701-707`, `< current` filter `:729`, per-year leg-set diff
`:755-761`);
`project/mod.rs:107-122` (`would_conflict` forces `cfg.pseudo_reconcile=false`);
`project/fold.rs:388,710,831,876,1196,1274` (the six sat-carrying `UncoveredDisposal` emit sites);
`pools.rs:15` (`pool_key`); `session.rs:662` (VaultLock deadlock doc), `:714` (`pre2025_tranche_exists`
caller); `lib.rs:27` (`guard_allocation_vs_tranche` crate-root re-export);
`reconcile.rs:1015,1258` + `edit/persist.rs:1032,1105` (the four allocation append sites).

## Verdict

**GREEN — 0 Critical / 0 Important / 2 Minor / 1 Nit**

The r4 fold preserves every binding SPEC decision (DFW-D1..D12) and adds NO new tax logic: every filed
number still flows through the characterization-pinned shipped primitives (`consent_terms`,
`filed_basis_for`, `promote_prior_year_advisory`, `append_decision`, `export_irs_pdf`/`export_full_return`).
All three mandated confirmations re-derive clean:

1. **The C-2 `tranche_guard` predicate move is genuinely tax-neutral.** The three moved predicates
   (`void_targets`/`in_force_allocation_exists`/`pre2025_tranche_exists`) are PURE event scans returning
   `BTreeSet<EventId>`/`bool` over core types only; the MUTATING gate `guard_tranche_vs_allocation`
   (`tranche.rs:107`) stays in the CLI, and `journey_view`'s `safe_harbor_blocked` is a read-only
   derivation over the same predicates, not a second gating authority. (One enumeration gap — a fifth
   caller — is Minor M-2 below; it is compiler-caught and filed-number-neutral.)
2. **`flagged_years`'s per-promote union is SPEC-faithful and errs to over-export.** It matches DFW-D11's
   mandated `promote_prior_year_advisory` fold-pair machinery (per-`promote_id` marginal diff), captures
   every prior year any single live promote changes, and unions with `{current}` — the safe direction (a
   spurious extra 1040-X is not a misfiling; a missing one is). (The r3-added two-promote KAT does not
   actually discriminate union from a whole-state diff — Minor M-1 below.)
3. **Nothing in the r4 fold introduced a wrong-filing path.** Dropping the dead `state` param from
   `plan_promote`/`plan_declare` is sound (the shipped pipeline rebuilds from `events` and projects
   internally — `promote.rs:364-488` never reads a passed `state`); the `return_inputs::exists`
   dispatch living once inside `export_irs_pdf_from_session` is behavior-preserving over both arms
   (moved verbatim, mirrors the already-`&Session` `export_full_return:642`); the `journey_view`
   `debug_assert!(!state.pseudo_active())` is a filed-number-neutral discovery guard backing the
   engine-real DFW-D6 entry gate.

## r3-resolution audit — M-1 folded (with a residual), N-1 folded correctly

- **r3 M-1 (flagged_years multi-promote completeness) — union spec FOLDED; the added two-promote KAT is
  non-discriminating (carried forward as r4 M-1).** The plan now specifies `flagged_years` as the "UNION
  of per-promote fold-diffs (matching `promote_prior_year_advisory`'s per-`promote_id` semantics — NOT a
  single whole-state with-all-vs-without-all diff)" (plan:261-263, Task 3 Step 6 plan:290-291) and adds a
  two-live-promote KAT (Task 3 Step 4 plan:286-288). The UNION requirement — the substance of r3 M-1 — is
  folded correctly and is SPEC-faithful (DFW-D11: "enumerated via the `promote_prior_year_advisory`
  fold-pair machinery … across all live promotes"; re-verified `promote_prior_year_advisory` is
  inherently per-`promote_id`: `with = project(ALL events)`, `without = project(ALL \ {promote_id})`,
  `conservative.rs:701-707`). HOWEVER, the ACCOMPANYING mutation claim does not hold for the KAT as
  written — see r4 M-1. The union impl itself is safe; the residual is test-adequacy only, non-gating.

- **r3 N-1 (`Refusal::Target` doc) — FOLDED CORRECTLY.** The plan's `PromotePlan` comment now reads
  "`Target` covers the resolve-live gate — unknown/voided/WRONG-TYPE target only (`resolve_live_tranche`,
  promote.rs:95-118). Already-promoted (DOUBLE-promote) is NOT caught here; it is `would_conflict` at
  APPLY-time → CliError (promote.rs:475-483), so `would_conflict` is not a plan Refusal" (plan:141-143).
  Re-verified at source: `resolve_live_tranche` (`promote.rs:95-118`) checks only `is_voided`
  (`:106-108`) + wrong-type (`match … DeclareTranche` `:112-115`) + absent (`find(|e| e.id == *id)`
  `:109-116`) — a double-promote is NOT among them; `would_conflict` ("a second live promote on this
  target", `promote.rs:477-483`) catches it at apply-time → `CliError`. The corrected attribution is
  exact. **Correct.**

## Re-derived confirmation of the three mandated audit areas

### 1. The C-2 `tranche_guard` predicate move — tax-neutral read-only extraction

- **The three predicates are pure and buildable in core.** `void_targets` (`tranche.rs:40`) scans for
  `VoidDecisionEvent`; `in_force_allocation_exists` (`:54`) = non-voided `SafeHarborAllocation` present;
  `pre2025_tranche_exists` (`:71`) = non-voided `DeclareTranche` with `window_end < TRANSITION_DATE`. All
  three consume only `&[LedgerEvent]` + core types + `conventions::TRANSITION_DATE` — no `Session`, no
  I/O, no CLI symbol. Moving them verbatim to `btctax-core::tranche_guard` inverts no dependency.
- **The mutating gate stays in the CLI.** `guard_tranche_vs_allocation` (`tranche.rs:107`, the tranche
  record-path refusal) and `guard_allocation_vs_tranche` (`:93`, the allocation-side refusal) remain in
  `cmd/tranche.rs`, rewired to call `btctax_core::tranche_guard::*`. Refusal BEHAVIOR is unchanged (same
  predicate, same `CliError`), so the move is behavior-preserving — the declare/allocation refusals fire
  under exactly the same conditions. The shipped `declare_tranche_cli.rs` allocation-guard KATs are the
  behavior baseline (Task 5 Steps 1/3).
- **`safe_harbor_blocked` is a read-only derivation, not a gate.** `journey_view` composes
  `tranche_guard::in_force_allocation_exists`+`pre2025_tranche_exists` into the `safe_harbor_blocked: bool`
  display field (Task 6 Step 3, plan:442-444) — a first-class dashboard state (DFW-D9), NOT a second
  authority that refuses a filing. The only authority that refuses is the CLI `guard_*` (which the
  chokepoint drivers still traverse). No filed number moves. **Tax-neutral.**

### 2. `flagged_years`'s per-promote union — captures every prior-year leg a promote changes

- **The union is the right shape and errs safe.** For each live promote, `promote_prior_year_advisory`'s
  diff (`with = project(ALL)`, `without = project(ALL \ {promote_id})`) flags every year whose disposal
  ∪ donation ∪ gift LEG SET changes (`conservative.rs:755-761` — Vec-eq, which catches a HIFO reorder
  AND an equal-basis/different-date swap that a Σ-gain compare would miss). Unioning these over all live
  promotes captures every prior year any single promote marginally rewrites — the DFW-D11 guarantee ("a
  filed prior year silently keeps a now-wrong 8949/§170(e)/§1015 leg" is prevented). The set is then
  unioned with `{current}` and is a strict superset of `promoted_filing_years` (disposal-legs-only,
  `admin.rs:85-98`) — pinned by Task 3 Step 4's single-promote KAT (a donation-reordered prior year with
  NO promoted disposal leg is in the set; mutation to `promoted_filing_years` drops it → reds). That KAT
  is well-posed and discriminating.
- **Over-export is the only realistic error direction.** Where two promotes' per-year leg effects offset
  in a whole-state diff (net-zero → the year's CURRENT filed legs equal the no-promote baseline), the
  union still flags the year and over-exports a harmless spurious 1040-X. This is the SPEC-blessed safe
  direction. **Captures the guarantee.**

### 3. r4 fold introduces no wrong-filing path

- **Dropped `state` param (plan_promote/plan_declare).** Re-verified `promote_tranche` (`promote.rs:364-488`)
  takes NO `state` — it opens the session, `load_all`s events, and every downstream primitive
  (`consent_terms`, `promote_prior_year_advisory`, `gift_only_flagged_years`, `would_conflict`) re-projects
  from `events` under `cfg`. `declare_tranche` (`tranche.rs:134-165`) is likewise events-only on the
  `None` path. So `plan_promote(events,prices,cfg,target,provenance,part_ii,now)` /
  `plan_declare(events,prices,cfg,…,target_shortfall,now)` losing `state` changes nothing computed.
- **`export_irs_pdf_from_session` (dispatch once inside).** Re-verified the shipped dispatch lives at
  `admin.rs:373` (`return_inputs::exists(conn, tax_year)` → full-return `export_full_return(&session,…)`
  at `:375` with `forms_ignored_full_return` at `:379`, else the crypto-slice body `:383+`). Extracting
  everything after `Session::open` into a `&Session` inner (mirroring `export_full_return:642`, which is
  ALREADY `&Session` over projected state/events) and leaving `export_irs_pdf` a thin opener is a verbatim
  move — the Task 3 Step 1 characterization pins BOTH dispatch arms (the full-return arm proves the
  retained dispatch survives). `apply_export` composing per-year over the TUI's held `&Session` avoids the
  real second-`Session::open` VaultLock deadlock (`session.rs:662`). Export appends no events (`&Session`,
  not `&mut`), and the inner `require_attestation(None)` fail-closed backstop (`admin.rs:389-392`) survives
  the extraction — DRAFT-gate posture intact on both surfaces. **Behavior-preserving for the filed packet.**
- **`journey_view` `debug_assert!(!state.pseudo_active())`.** The DISCOVERY read `shortfalls(state)`
  consumes the passed state's `state.shortfalls`, which — if the state were projected pseudo-active — would
  omit pseudo-cleared shortfalls (a `SelfTransferMine{$0}` masking a real short, DFW-D6). The engine-real
  guarantee is the Task 7 dashboard entry gate (`!state.pseudo_active()`); the `debug_assert` documents/
  backstops that precondition. It is release-compiled-out, but that is acceptable: `journey_view` is a pure
  read-only view that FILES nothing, and every actual filing path independently forces pseudo off
  (`apply_promote`/`plan_promote` mirror `would_conflict`'s `cfg.pseudo_reconcile=false`,
  `project/mod.rs:121-122`) or refuses (`plan_export` on pseudo-active, DFW-D11). No wrong number can flow
  from the view. **Filed-number-neutral.**

## Findings

### Minor

- **M-1 (Task 3 Step 4, plan:286-288 — the r3 two-promote KAT does not kill the mutation it names).** The
  KAT is specified as "two promotes each flagging a DIFFERENT removal-reordered prior year; assert BOTH
  years ∈ `plan_export.years`", with the mutation "a single whole-state with-all-vs-without-all diff whose
  two promotes' per-year effects cancel → a year drops → reds." **The construction and the mutation are
  incompatible.** With two promotes targeting DISJOINT years (P1→Y1, P2→Y2, Y1≠Y2), no cancellation is
  possible: the whole-state diff (with-both vs without-both) still flags Y1 (P1's effect is present, P2
  does not touch Y1's legs to offset it) and still flags Y2 — so a `flagged_years` reimplemented as a
  single whole-state diff returns `{Y1,Y2}` exactly as the union does, the assertion still passes, and the
  mutation SURVIVES. Union and whole-state diverge ONLY on SAME-year interactions (offsetting → whole-state
  drops but the year is genuinely back to baseline; redundant coverage → union could drop), neither of
  which a different-years KAT exercises. **Failure scenario:** the plan claims a mutation-proof for the
  per-promote-union requirement that its own KAT cannot deliver — an untested-guard: the union code could
  be silently swapped for a whole-state diff and this KAT would stay green. **Non-gating:** the union
  IMPL is SPEC-mandated (DFW-D11) and safe-direction (over-export); the single-promote KAT already pins the
  `promoted_filing_years`-vs-fold-diff distinction that is the real DFW-D11 §5 requirement. **Fix:** either
  (a) restate the two-promote KAT as a plain positive multi-promote coverage test (drop the
  whole-state-mutation claim, which is unfalsifiable at this altitude with distinct floors), or (b) if the
  union-vs-whole-state distinction must be pinned, construct two promotes whose effects on the SAME prior
  year net to an identical leg set under a whole-state diff while each marginal is non-empty — and note
  that such a construction is hard to realize with leg-set equality, which is itself the reason (a) is the
  honest choice.

- **M-2 (Task 5 Step 2, plan:356-362 — the C-2 predicate move under-enumerates its callers; a fifth cli
  caller of `pre2025_tranche_exists` is unlisted).** The step says "rewire … `guard_tranche_vs_allocation`/
  `guard_allocation_vs_tranche` … call `btctax_core::tranche_guard::*`, and DELETE the cli copies (single
  source; all four allocation append sites preserved)." Re-grepping the moved predicates' callers: the
  four allocation append sites (`reconcile.rs:1015,1258`; `edit/persist.rs:1032,1105`) call the
  CLI-resident `guard_allocation_vs_tranche` (which STAYS) — preserved automatically — but there is a
  FIFTH, DIRECT caller of a MOVED predicate the plan does not mention: `session.rs:714`
  (`if crate::cmd::tranche::pre2025_tranche_exists(&all)`), the read-only refusal to OPEN the
  safe-harbor-allocate flow (prevents a misleading residue display). **Failure scenario:** an implementer
  who follows "DELETE the cli copies" literally leaves `session.rs:714` referencing a deleted symbol → a
  compile break (caught by the Step 3 `make check`, so it cannot ship, but it is an avoidable
  plan-completeness gap; worse, an implementer might "fix" it by leaving a duplicate CLI copy, defeating
  the single-source intent). **Non-gating:** tax-neutral (the predicate is pure; after rewiring to
  `btctax_core::tranche_guard::pre2025_tranche_exists` the refusal behaves identically) and
  compiler-caught. **Fix:** add `session.rs:714` to the rewire list (rewire, do not duplicate), and
  correct the parenthetical — the callers to preserve are "the four allocation append sites (all via
  `guard_allocation_vs_tranche`) PLUS the `session.rs` residue-opener's direct `pre2025_tranche_exists`
  call." (The `lib.rs:27` `pub use cmd::tranche::guard_allocation_vs_tranche` re-export is unaffected —
  that guard stays in the CLI.)

### Nit

- **N-1 (Task 3, plan:239-240/261 — `flagged_years` has no `< current` filter, unlike the advisory it
  mirrors).** `promote_prior_year_advisory` filters candidate years to `< current`
  (`conservative.rs:729`) precisely so a year still being AUTHORED (≥ current) is never told it needs a
  Form 1040-X. `flagged_years(events, state, prices, tables, cfg)` drops the `current` param, so it can
  return a year `> current` if any disposal/removal is dated in a future year; `plan_export.years =
  {current} ∪ flagged_years` would then emit a premature packet for a not-yet-filed future year.
  **Practically inert:** the audience is historical no-records reconciliation (disposals ≤ current), a
  year == current is harmlessly absorbed by the `{current}` union, and a future-dated disposal is
  out-of-scenario; even if it occurred, the result is over-export (safe), never a wrong number. **Fix
  (optional):** either bound `flagged_years` to `< current` internally (letting `{current}` supply the
  current year) or note in the plan that a `> current` year is intentionally out of scope because the
  audience has no future-dated legs.

## Confirmed correct (adversarially re-checked, no finding)

- **DFW-D6 pseudo-off remains the ONE intended behavior change, correctly scoped.** `plan_promote` forces
  `cfg.pseudo_reconcile=false` on its own copy before `consent_terms`/`promote_prior_year_advisory`/
  `gift_only_flagged_years` (plan:152-154), mirroring `would_conflict` (`project/mod.rs:121-122`). The
  shipped `promote_tranche:396` uses the stored un-forced `cfg`, so the fix is real, is a no-op on
  non-pseudo vaults, and changes no CORRECT filed number — it only replaces synthetic §6664(c) terms with
  the honest pseudo-off flavors (Task 1 Step 6 KAT).
- **The `PromotePlan` three-ordered-pieces reproduce the print order + §6664(c) artifact.** Re-verified the
  shipped sequence: advisory FIRST (`for line in &advisory`, `promote.rs:443-445`) → `render_consent(&terms,
  &gift_only_years)` (`:453`, the shipped `render_consent` at `:333-341`) → `wide_window_note` (`:454-456`);
  recorded `Acknowledgment.shown_terms = terms` ONLY (`:466-469`). `PromotePlan`'s `advisory_lines` /
  `gift_only_years` (an INPUT to `render_consent`, not a pre-rendered string) / `post_consent_note`
  (plan:132-140) re-emit in that exact order, byte-reproducible with the driver's single `println!`. The
  plan correctly forbids collapsing the three into one flat Vec.
- **The six sat-carrying `UncoveredDisposal` emit sites are exactly as listed.** Re-grepped: `:388`
  ("self-transfer/gift fee short", the fee-carry arm) → fee; `:710` ("dispose short"), `:876` ("self
  transfer short"), `:1196` ("gift out short"), `:1274` ("donate short") → principal (each preceded by
  `consume_principal` at `:707/:873/:1193/:1271`); `:831` ("pending out short") lumps `total_sat` at `:828`
  — the one non-partitioned record the plan flags (plan:78) and routes to `ResolveFirst`, so its ambiguous
  fee/principal split never drives a declare prefill or `FeeOnlyPromoteNoop`. The fee/principal split is
  buildable and tax-sound (carried from r3 M-1/r2 M-1).
- **`method_inversion_advisory`/`tranche_dip_advisory` surfaced with real signatures.**
  `tranche_dip_advisory(&Disposal)->Option<String>` (`conservative.rs:27`) and
  `method_inversion_advisory(&LedgerState,&WalletId,LotMethod)->Option<String>` (`:61`) are pure,
  provenance-neutral, "never instruct a tax-understating action" builders; carried VERBATIM into
  `Advisory::{TrancheDip,MethodInversion}(String)` — no filed-number path.
- **`Refusal` plan-time set is complete; apply-time gates stay inside `apply`.** All six shipped promote
  refuse points have a home: resolve-live→`Target`, provenance→`Provenance`, Part II→`PartII`,
  floor/coverage→`Coverage` (`filed_basis_for`'s `Coverage` is `{Full,Partial}`, no `None` — r3 verified),
  ack + `would_conflict`→inside `apply_promote`→`CliError` (`promote.rs:458`,`:477-483`). No tax-relevant
  gate is lost.
- **Over-coverage stays a derived advisory; the shared promote gate is untouched.** `OverCovered{by_sat}`
  is scoped to `covered_sat>0 ∧ live_sat>covered_sat` with the fully-undisposed carve, derived in
  `journey_view` (read-only) — no guard added to the chokepoint, so `mixed_vintage`/undisposed/
  correctly-sized promotes stay behavior-preserving (DFW-D5.3, ledger-identical to a legit reorder).
- **`Acknowledgment.shown_terms` is byte-identical CLI vs TUI.** Both surfaces drive the SAME `plan_promote`
  (pseudo-forced-off) → `render_consent` → `apply_promote`; the Task 4 full-driver parity harness compares
  the RECORDED artifacts across both driver paths (not two renderer calls), covering happy/refused-ack/each
  refusal. The §6664(c) artifact carries the honest pseudo-off terms on either surface.
