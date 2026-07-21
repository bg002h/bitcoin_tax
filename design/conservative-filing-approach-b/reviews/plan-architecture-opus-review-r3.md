# Plan review — architecture lens (Opus), round 3

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `1c69d35` (branch `feat/conservative-filing-b`)
**Reviewer:** independent software-architecture lens (Opus 4.8), round 3 — a fresh model. Every r2 finding re-verified against the folded plan AND current source at HEAD; the `PromoteSet`/`FoldCtx` threading re-derived from scratch rather than trusting prior verdicts.

## Verdict

**NOT GREEN — 0 Critical / 1 Important / 1 Minor / 2 Nit.** The two r2 blockers (transition.rs:60 second `FoldCtx`; the verify-drift false cover) are genuinely and correctly folded, and every r2 Minor/Nit is resolved. But re-deriving the `FoldCtx` threading from source surfaces a NEW blocker of the **same shape as r2 I-1**: r2 (and the review charter) assumed `FoldCtx` has **two** construction sites (`fold` + `universal_snapshot`). It has **four** — `fold` (fold.rs:413), `universal_snapshot` (transition.rs:60), **AND `pools_before` (fold.rs:463) + `state_as_of` (fold.rs:520)**. T4 lists only the first two, so the plan as written does not compile, and the unguided fix at `state_as_of` silently diverges the optimizer. Fix I-1 and this lens goes green.

---

## Verified resolved (r2 findings — do not re-raise)

- **arch I-1 (second `FoldCtx` at transition.rs:60) — RESOLVED at the site it names.** Verified against source: `universal_snapshot` (transition.rs:37) builds a `FoldCtx { config, elections, selections }` at :60 and folds the pre-2025 residue through the shared `fold_event`; its **single** call site is resolve.rs:1286, inside `resolve`, in the `for (_seq, d) in &decisions` loop (:1260) — `promotes` (built before the step-2 timeline loop at :1076) is in scope there. The alloc_basis mechanism is real: for a fully-consumed tranche `snap.estimated_conservative_remaining_sat == 0` so the D-8 tag-arm (:1304) does not fire and `alloc_basis != snap.basis` (:1306) is the only guard — an empty-set snapshot would misadjudicate. T4 adds transition.rs to Files+commit, threads `promotes` into `universal_snapshot` + its FoldCtx, and pins the divergence KAT (empty set reds it). Correct — **but incomplete for the other two sites → I-1 below.**
- **arch I-2 (verify-drift false cover) — RESOLVED.** T11 **Step 3b** is now a real owned step: `promote_drift_advisory` in `conservative_promote.rs` + Files include `crates/btctax-cli/src/cmd/inspect.rs`. Verified `verify` (inspect.rs:146) has signature `(vault_path, pp) -> Result<VerifyReport, CliError>` — vault+passphrase only, exactly as the plan states, so threading a `PriceProvider` + a `drift` field is the right shape. KAT `verify_drift_advisory_is_direction_aware_and_the_fold_still_uses_the_stored_number` pins both directions AND `project(&ev, &hi)` still reads the STORED `dec!(12_000)`. Self-review/status corrected to "honest, now REALLY written." (Residual: the KAT hits the core fn, not the `verify`/`VerifyReport` surface → M-1.)
- **arch M-1 (`Printed8275` in T13 Files/commit) — RESOLVED.** T13 Files list `crates/btctax-core/src/tax/printed.rs`; the `git add` (:1216) includes it. `Printed8283Rows` (printed.rs:135) verified as the mirror; `Printed8275` does not yet exist (T13 creates it — no forward-consumption).
- **arch M-2 (`ATTEST_PHRASE`→`PROMOTE_ACK_PHRASE`) — RESOLVED.** All three T10 snippets (`empty_part_ii…`, `a_recorded_promote…`, `a_second_promote…`) now pass `Some(PROMOTE_ACK_PHRASE)`. `ATTEST_PHRASE` (lib.rs:197) survives only as the named PRECEDENT in T10's Reference — correct.
- **arch M-3 (`safe_harbor_residue` KAT) — RESOLVED.** T12 step 1 now carries `safe_harbor_residue_does_not_project_a_dangling_promote` (session.rs:713-716 drop filter).
- **arch M-4 (T3 refusal-KAT "both directions") — RESOLVED.** `a_promoted_tranche_still_refuses_a_safe_harbor_allocation_at_record_time` narrowed to the shared `pre2025_tranche_exists` predicate ("verified shared predicate"), asserting the one `guard_allocation_vs_tranche` path that covers both guards by construction.
- **arch N-1 (six→eight) — RESOLVED.** T4 says "the **eight** builder call sites." Verified complete against source: make_disposal_legs :362/:635, consume_fee :641/:832/:1122/:1199, make_removal_legs :1118/:1195 (2 disposal, 4 fee, 2 removal).
- **arch N-2 (file-map read-only) — RESOLVED.** `compute.rs`/`cmd/tax.rs` now listed as "**READ-ONLY reference** for T8's cascade quoting … these are NOT modified."
- **cross-lens tax M-1 (amend direction) — RESOLVED.** T8 step 3 states promote-raises-basis → amend-to-**refund**/§6511, void → amend-to-**pay**, copy following the Δ sign; `the_void_direction_fires_amend_to_pay` + the §6511/"additional tax" KATs pin it.

Also re-verified good: the type-ownership chain has one owner per type with no forward-consumption (T1 payload → T2 `ComputedFloor`/`PromoteSet`/`PromoteEntry` → T3 `Resolution.promotes` + `live_promotes` → T4 `FoldCtx.promotes`/`clamped_leg_basis` → T5/T6 builder params → T13 `Disclosure8275`/`Printed8275` → T14/T15/T16). Cited symbols exist at/near the cited lines: `Coverage` (conservative.rs:174), `WindowRef{min,coverage}`/`window_reference` (:181/:193), `round_cents`/`SATS_PER_BTC` (conventions.rs:22/:15), `Consumed` (pools.rs:291), `FeeCarry` (fold.rs:274), `consume_fee` TreatmentC (:348), `LotId.origin_event_id` (identity.rs:118), `is_revocable_payload` (void.rs:20), `effective_alloc` closure (void.rs:72-81), `voidable_decisions` (void.rs:54), `would_conflict` (mod.rs:107), `build_op` `_ => Op::Skip` (resolve.rs:413), the admit branch (:1085-1114) with `build_op` at :1089 and `op` pushed at :1111, pass-1a `Some(_) => voided.insert` (:484-485), `Resolution` (:201, one field-set, no `promotes` yet). The T3→T4→T5/T6 task order is sound.

---

## Important

### I-1 (T4) — `FoldCtx` has FOUR construction sites; T4 threads only two. The plan does not compile as written, and the unguided fix silently diverges the optimizer's `state_as_of`.

- **Defect:** `FoldCtx` (fold.rs:21, `pub(crate)`, no `Default`) is constructed at **four** sites, not two:
  `fold` (fold.rs:413), `universal_snapshot` (transition.rs:60), **`pools_before` (fold.rs:463)**, and
  **`state_as_of` (fold.rs:520)**. T4's threading step names only `fold` (":376") and transition.rs:60;
  the plan mentions `pools_before`/`state_as_of` **nowhere** (grep: 0 hits). r2 I-1 closed transition.rs:60
  but its site-census — "FoldCtx is constructed in `fold` and transition.rs:60" — undercounted the fold.rs
  helpers. All three fold.rs sites take `res: Resolution`, so `res.promotes` (T3) is in scope at each.
- **Concrete failure:** adding `promotes: PromoteSet` to `FoldCtx` (T4) and updating only the two named
  sites leaves fold.rs:463 and fold.rs:520 as `FoldCtx { config, elections, selections }` → **E0063
  (missing field `promotes`)** at T4 step 4 `make check`. So "buildable exactly as written" is false.
  The compiler-guided easy fix has two branches:
  - `pools_before` (→ `optimize::available_lots_before`, optimize.rs:316) returns the **pool residue** and
    discards LedgerState. The T4/T5/T6 decomposition only rewrites **legs** (the FeeCarry re-homes onto the
    disposal *leg* at fold.rs:654, not the pool; clamp/documented-only touch leg basis, not `take_from`
    debits), so the pool residue is `ctx.promotes`-invariant — an empty set here is behaviorally harmless
    but should still read `res.promotes` for consistency.
  - `state_as_of` (→ `optimize::consult_sale`, optimize.rs:1249) returns the **finalized LedgerState**
    including disposals/removals whose leg basis/gain DO depend on `ctx.promotes` (T4 clamp, T5 fee
    evaporation, T6 documented-only). An empty set here compiles, keeps **every plan KAT green** (no KAT
    exercises the optimizer over a promote), and hands the optimizer un-clamped/un-evaporated basis for a
    promoted disposal → wrong tax-minimization advice. This is the exact "shared `fold_event` so the
    pre-fold cannot diverge" invariant r2 I-1 protected, violated at a second surface.
- **In-plan fix (T4):** enumerate all THREE fold.rs sites (`fold`:413, `pools_before`:463, `state_as_of`:520)
  plus transition.rs:60, each populated from `res.promotes` (in scope at all three fold.rs sites; the field
  addition stays inside fold.rs + transition.rs — `optimize.rs` calls these fns by unchanged signature, so
  no new file). Add a KAT exercising the optimizer path (`consult_sale`/`state_as_of`) over a promoted tranche
  with a below-window-low disposal; mutation: an empty set at `state_as_of` reds it. (`pools_before` needs
  no behavioral KAT — pin only that it compiles with `res.promotes`.)

---

## Minor

### M-1 (T11 Step 3b) — the verify-drift wiring (`verify`/`VerifyReport`) is specified but unpinned; the only KAT hits the core fn.

The drift step mandates threading a `PriceProvider` into `verify` (inspect.rs:146) + a `drift: Vec<String>`
field on `VerifyReport`, but `verify_drift_advisory_is_direction_aware…` calls `promote_drift_advisory(&ev,
&hi)` directly — no test asserts the `verify`/`VerifyReport` surface actually invokes it. An implementer who
adds the fn but forgets to call it from `build_verify`/`verify` ships a green suite with the BG-D3 mandate
un-surfaced (the project's own "untested-guard" failure mode). Add a CLI/`build_verify`-level assertion that
`VerifyReport.drift` is non-empty for a drifted promote.

---

## Nit

### N-1 (T4/T5) — "pass `&ctx.promotes` to the eight builder call sites" is wrong for site :362.

Site :362 (`make_disposal_legs`) is **inside** `consume_fee`, whose signature takes `config: &ProjectionConfig`,
not `ctx: &FoldCtx` (fold.rs:323-334) — so `ctx` is out of scope there. The value passed at :362 must be
`consume_fee`'s forwarded `promotes` param (T5), not `&ctx.promotes`. Self-correcting (E0425 cannot-find-`ctx`),
and T4 Produces already says `consume_fee` gains the param, but the blanket phrasing invites a wrong first cut.

### N-2 (T4 self-review framing) — "thread the `PromoteSet` there too" / the charter's "BOTH `FoldCtx` sites" both presuppose a two-site `FoldCtx`.

Once I-1 is folded, correct the T4 prose and the self-review to name the true four-site count (or say "every
`FoldCtx` construction site"), so a future fold does not re-inherit the two-site mental model that let two
sites slip.

---

## Buildability statement

With I-1 folded (all four `FoldCtx` sites threaded from `res.promotes`, `state_as_of` KAT added), this lens
finds the plan buildable exactly as written: one owner per type, no forward-consumption, the `promotes` chain
(T2 defines → T3 `Resolution.promotes` + `live_promotes` → T4 `FoldCtx` all sites → T5/T6 builder params →
`clamped_leg_basis`/decomposition keyed by `leg.lot_id.origin_event_id`) closed end-to-end, the verify-drift
surface real, and every remaining cited symbol/region verified against HEAD (`1c69d35`). The single open
blocker is I-1.
