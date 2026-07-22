# Plan review — architecture lens (Fable), round 2

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `50d7f3e` (branch `feat/conservative-filing-b`)
**Reviewer:** independent software-architecture lens (Fable), round 2. Every r1 finding re-verified against the folded plan AND current source at HEAD.

## Verdict

**NOT GREEN — 0 Critical / 2 Important / 4 Minor / 2 Nit.** The r1 wiring folds are genuinely in and
correct (the `PromoteSet` chain, the T7 insertion point, the advisory call sites, `Printed8275`, the
8275 year aliasing all check out against source). Two blockers remain, and both are of the same shape:
**the fold's self-report claims work the plan text does not contain.** (1) The `FoldCtx` threading
census missed the SECOND `FoldCtx` construction site (`transition.rs:60`, inside `universal_snapshot`)
— an unlisted compile break whose easy fix silently violates the pre-fold no-divergence invariant; (2)
the BG-D3 verify-drift advisory is still owned by NO task — the status header and self-review say
"→ T11 (added)", but T11 contains no drift step and no task touches `cmd/inspect.rs` (where `verify`
lives). Fix both in-plan and this lens can go green.

---

## Verified resolved (r1 findings — do not re-raise)

- **I-1 (PromoteSet threading) — RESOLVED.** T2's Produces owns `PromoteEntry { filed_basis,
  tranche_sat }` + `PromoteSet` in `conservative_promote.rs`; T3 produces
  `live_promotes(events, &voided, &mut blockers) -> PromoteSet` (the `&mut Vec<Blocker>` gives T7(c)
  its conflict-push channel) and adds `promotes: PromoteSet` to `Resolution` — which has exactly one
  construction site (resolve.rs:1418), so the field addition is contained; T4 threads it into `FoldCtx`
  (fold.rs:21), populates in `fold` (:376, `mut res: Resolution` — `res.promotes` is right there), and
  the listed call-site set :362/:635/:641/:832/:1118/:1122/:1195/:1199 is verified **complete** against
  source (`make_disposal_legs` :362/:635, `make_removal_legs` :1118/:1195, `consume_fee`
  :641/:832/:1122/:1199). No task consumes a type before its owner defines it; T2→T3→T4 ordering is
  sound. (One residual: the fold.rs-external `FoldCtx` site → new I-1 below; the count word → N-1.)
- **I-2 (void-adjudication insertion point) — RESOLVED.** Verified against source: pass-1a
  (:459-496) currently voids all revocable targets inline via the `Some(_)` arm; T7(b)'s carve
  (defer any void of a `DeclareTranche` with ANY promote in the ledger) + T7(d)'s insertion
  ("immediately after the pass-1a loop, BEFORE step 2") reproduces v1 inline semantics exactly for a
  promote-less tranche — every downstream `voided` read (:516/:580/:608/:668, the admit branch
  :1086-1087 `if voided.contains(&e.id) { continue }`, the LotSelection/MethodElection passes) runs
  after the insertion, so the admitted timeline never contains the voided tranche and
  `universal_snapshot` (called at resolve.rs:1286, step 3) never counts it. Acyclicity holds:
  promote-liveness depends only on promote-targeted voids, all applied inline in pass-1a; the
  adjudication is order-independent both across void order (the `both_voids` KAT) and relative to the
  `live_promotes` call (a tranche is only ever voided when no live promote targets it, so the map is
  unaffected either way).
- **I-3 (BG-D9 advisory wiring) — RESOLVED.** T8-3b names the real void surfaces —
  `cmd/reconcile.rs::void` (:243) and `apply_bulk_void` (:826) both exist — with a CLI-level test;
  T10 step 3 mandates the `Direction::Promote` call before the consent prompt and names itself the
  only promote-direction call site. (Residual file-map drift → N-2.)
- **I-4 → NOT resolved — re-raised as I-2 below.**
- **I-5 (`Printed8275` uncreated) — RESOLVED.** T13's Produces defines `Printed8275` +
  `printed_8275` in `tax/printed.rs` (mirroring `Printed8283Rows`, printed.rs:135 verified), created
  in Phase 1a before T15 (`fill_form_8275(printed: &Printed8275, …)`) and T16 consume it. (T13's
  Files/commit omit the file → M-1.)
- **I-6 (8275 year coverage) — RESOLVED.** T15 step 3 makes the aliasing MANDATORY across
  `SUPPORTED_YEARS = &[2017, 2024, 2025]` (lib.rs:61 verified); the `for_year(year)` constructor
  pattern (map.rs :163/:487/…) returns per-year `Result`, so a three-armed match onto one bundled
  asset is implementable exactly as written; step 1 pins per-year map+fill KATs and T16 adds the
  2025 end-to-end (`a_promoted_2025_export_fills_the_8275_and_the_gate_passes`).
- **M-1 (`Usd::from_dollars`) — RESOLVED.** Swept file-wide; the only remaining hit is the Global
  Constraints prohibition itself. All snippets use `dec!`.
- **M-2 (census items 6/7) — RESOLVED.** T4 owns the parent Invariant-KAT amendment
  (kat_conservative.rs in Files); T11 step 3's ★ block owns items 6+7 explicitly.
- **M-3(b)/(d) — RESOLVED** (T3 `relocated_promoted_tranche_keeps_tag_and_floor`; T10's
  six-provenance loop). **(a)/(c) partially unresolved → M-3/M-4 below.**
- **M-4 (`build_op` item 11) — RESOLVED.** T3 step 3 adds the explicit
  `EventPayload::PromoteTranche(_) => Op::Skip` arm + comment; the current `_ => Op::Skip` (:413)
  verified.
- **N-1 (`EventId` Display) — RESOLVED.** T12 renders via `p.target.canonical()`; the Global
  Constraints idiom note pins it plan-wide.

Also re-verified good this round: `Disposal`/`Removal` DO derive `PartialEq, Eq` (state.rs:166/:201)
as T8's leg-set diff requires; `compute_tax_year` (compute.rs:232), `carryforward_consistency`
(:436), `capital_loss_carryforward_in` (:317), `charitable_carryover_out` (return_1040.rs:1311);
`TaxTables` trait (tables.rs:113) / `TaxProfile` (types.rs:32); `persistence::fingerprint` (:25);
`EventId::decision` (identity.rs:82); admin.rs export fns :68/:247/:535 with the `pseudo_active()`
checked-first slots :80/:281/:535; `write_basis_methodology_txt` call sites render.rs:871/:911 +
admin.rs:304/:555; `FormArg` cli.rs:958-970; the no-`..` destructure packet.rs:36-58;
`CENSUS_KEYS: [&str; 14]` census.rs:29; void.rs `is_revocable_payload` :20 / `effective_alloc` :72.

---

## Important

### I-1 (T4) — `FoldCtx` has a SECOND construction site the threading step doesn't list; the easy fix silently breaks the pre-fold no-divergence invariant

- **Defect:** T4's threading step ("★ FIRST thread the `PromoteSet` … fold.rs only") misses that
  `FoldCtx` is also constructed in `transition.rs:60`, inside `universal_snapshot` — under the
  in-source comment "**Same FoldCtx the real fold uses, so the pre-2025 residue cannot diverge
  (I-1)**". `fold_event` has four callers: fold.rs:425/:478/:539 AND transition.rs:66.
- **Concrete failure:** adding `promotes: &PromoteSet` to `FoldCtx` breaks compile (E0063) at a file
  in NO task's Files list. The implementer's unguided easy fix — an empty set — compiles and keeps
  **every plan KAT green** while violating the named invariant: after T5, a pre-2025 fee draw from a
  promoted tranche re-homes the FULL `FeeCarry` basis in the snapshot pre-fold but EVAPORATES the
  estimate in the real fold, so `snap.basis` diverges and the §7.4 conservation adjudication
  (`alloc_basis != snap.basis`, resolve.rs:1305) judges against a phantom basis — precisely in the
  fully-consumed-tranche case where the D-8 backstop (`estimated_conservative_remaining_sat > 0`)
  does NOT fire and the basis equality is all that's left. T3's `snapshot_timing…` KAT does not hold
  this: the floor reaches the snapshot via the TIMELINE rewrite, not via `ctx.promotes`.
- **In-plan fix (T4):** add `crates/btctax-core/src/project/transition.rs` to Files + commit;
  `universal_snapshot` gains a `promotes: &PromoteSet` param, passed into the `FoldCtx` at
  transition.rs:60; update the single call site (resolve.rs:1286 — `promotes` is in scope there,
  built before step 2, so this stays acyclic). Name the KAT: promoted pre-2025 tranche, fee-sats
  FIFO-drawn pre-2025, tranche fully consumed, then a `SafeHarborAllocation` — conservation must
  adjudicate against the evaporated (documented-only) residue basis; mutation: passing
  `&PromoteSet::new()` in `universal_snapshot` reds it.

### I-2 (T11 / self-review) — the BG-D3 verify-drift advisory is STILL unowned, now under false cover (r1 I-4 re-raise)

- **Defect:** the fold's status header ("the verify-drift task (T11)") and self-review ("verify-drift
  advisory → **T11 (added**, plan-r1 I-2/I-4)") claim the r1 finding is folded. It is not. T11's
  steps are the five advisories + the `$0` copy sweep + items 6/7 — no drift recompute, no drift KAT;
  grep "drift" over the plan hits only the header (:10), the Global Constraints quote (:46), T16's
  unrelated anti-drift destructure (:1295), and the self-review claims (:1308/:1318) — **zero hits
  inside any task body**. The verify surface is `cmd/inspect.rs:146` (`pub fn verify(…) ->
  Result<VerifyReport, CliError>`) — a file in NO task's Files list.
- **Concrete failure:** SPEC §2 BG-D3 / §6 mandate "the stored number survives a price-data change
  (fold uses stored, **verify flags drift**, direction-aware)". Executed as written, the whole suite
  goes green with the mandate unimplemented — and the artifact's own self-report says it was done.
  This is the claimed-but-absent class (project memory: don't defer a spec mandate on false cover),
  which is worse than r1's honest gap because a gate reader would tick it off.
- **In-plan fix:** add the real step to T11 (Files += `crates/btctax-cli/src/cmd/inspect.rs`) or a
  dedicated task: in `verify`/`VerifyReport`, for each live promote recompute `filed_basis_for`
  (T2) against current price data, compare to the stored `filed_basis`, and emit the direction-aware
  advisory (recompute BELOW stored → understated-floor warning; recompute ABOVE stored on a
  not-yet-filed position → "void + re-promote to the corrected lower number" hint). Name the §6 KAT
  pair: stored-survives-price-change (fold unchanged) + drift advisory fires with the correct
  direction copy. Correct the self-review/status lines to match.

---

## Minor

### M-1 (T13) — `tax/printed.rs` missing from T13's Files and commit

T13's Produces defines `Printed8275`/`printed_8275` "in `crates/btctax-core/src/tax/printed.rs`", but
its Files list and `git add` cover only `tax/form8275.rs` + `tax/mod.rs` + the test. As scripted, the
printed.rs change is left uncommitted at T13's commit and out of the task's declared scope. Add the
file to both.

### M-2 (T10) — three test snippets gate on `ATTEST_PHRASE` where the task mandates the distinct `PROMOTE_ACK_PHRASE`

`empty_part_ii_narrative_is_refused_at_record_time`, `a_recorded_promote_carries_the_acknowledgment…`,
and `a_second_promote_is_refused_by_would_conflict` all pass `Some(ATTEST_PHRASE)` — but T10 step 3
mandates "a NEW distinct const `PROMOTE_ACK_PHRASE` … NOT the pseudo-attest phrase". TDD-verbatim,
the failing tests drive the pseudo-attest phrase into the promote gate (the exact conflation the
distinct-const ruling exists to prevent). Sweep the snippets to `PROMOTE_ACK_PHRASE`.

### M-3 (T12) — the `safe_harbor_residue` promote-filter still has no test (r1 M-3(c) carried)

T12 step 3 adds `| EventPayload::PromoteTranche(_)` to the drop filter (session.rs:713-716), but
step 1's tests are only the two render KATs — the §6-mandated "`safe_harbor_residue` does not project
a dangling promote" KAT is still absent. Add it (a promote layered on a dropped `DeclareTranche` must
not leak into the pre-2025 residue).

### M-4 (T3) — the record-time-refusal KAT claims "both directions" but asserts one (r1 M-3(a) residual)

`a_promoted_tranche_still_refuses_a_safe_harbor_allocation_at_record_time` cites both guards
(cmd/tranche.rs:93-97 AND session.rs:694 — the TUI allocate-opener's `pre2025_tranche_exists`
refusal, verified at session.rs:692-701) but calls only `guard_allocation_vs_tranche`. Either assert
the session-side opener refusal for a PROMOTED tranche too, or narrow the comment.

---

## Nit

### N-1 (T4) — "the six builder call sites" lists eight

The line-number list (:362/:635/:641/:832/:1118/:1122/:1195/:1199) is correct and COMPLETE — eight
call sites, not six (2× disposal, 2× removal, 4× fee). Fix the count so an implementer doesn't stop
early; the numbers themselves need no change.

### N-2 (File-Structure map) — compute.rs/cmd/tax.rs still listed under "Modified (by task) … (T8)"

T8's Reference block records the explicit no-change decision ("T8 quotes their diffs, it does not
make their existing copy promote-aware"), but the map still lists both files under **Modified**. Move
the entry to a read-only/reference note so the map and the task agree.

---

## Buildability statement

With I-1 and I-2 folded, this lens finds the plan buildable exactly as written: the type-ownership
chain (T1 payload → T2 shared types → T3 `Resolution.promotes` → T4 `FoldCtx`+builders →
T5/T6 params → T13 `Disclosure8275`/`Printed8275` → T15/T16) has one owner per type, no
forward-consumption, correct task order, and every remaining cited symbol/region verified against
HEAD (`50d7f3e`).
