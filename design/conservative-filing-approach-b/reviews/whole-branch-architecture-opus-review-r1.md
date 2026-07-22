# Whole-branch ARCHITECTURE review — Approach-B Phase 1a (PromoteTranche)

Reviewer lens: software architecture. Author ≠ reviewer. Re-derived from first principles against
current source under `crates/` (not the diff alone); traced real call graphs. Range `dde890a..645bc20`.

## Verdict

**GREEN — 0 Critical / 0 Important / 4 Minor / 3 Nit**

The single most important architectural claim (BG-D1 by-construction) is TRUE in the realized code, and
the 4-site FoldCtx threading is genuinely complete. No blocking finding. The Minors/Nits below are
recorded per the workflow; none gates.

---

## Critical

none

## Important

none

The four load-bearing guarantees the brief asked me to verify all hold **in code**, not just by assertion:

- **BG-D1 rewrite is the SOLE mechanism.** `resolve` step-2 (`resolve.rs:1298-1306`) rewrites ONLY
  `Op::Acquire.usd_cost = entry.filed_basis`, gated on `a.basis_source == EstimatedConservative` and
  `promotes.get(&e.id)`. `build_op`'s `PromoteTranche` arm is `Op::Skip` (`resolve.rs:442`) — the promote
  moves nothing of its own. No new `BasisSource` (event.rs unchanged; the tag stays
  `EstimatedConservative`). The rewrite is INSIDE the timeline build, before the Eff is pushed, so
  `universal_snapshot` (step-3) and every downstream pass read the floor. I looked for a consumer that
  reads a promoted lot's basis around the rewrite: none — the lot's `usd_basis` becomes `filed_basis` at
  `fold_event` `Op::Acquire` (`fold.rs:670`), and the BG-D4 clamp decomposes it back from the SAME
  `PromoteSet`. The D-8 backstop (`estimated_conservative_remaining_sat`, tag-keyed), the Path-A seed and
  the self-transfer relocation carve (both key on the `EstimatedConservative` tag, `transition.rs:101`,
  `fold.rs:900`) hold unchanged. Confirmed by the genuinely-discriminating KAT
  `snapshot_timing_the_floor_is_visible_to_pass1_conservation` (a HIFO-reorder that only conserves when
  step-3 saw the floor).
- **All 4 FoldCtx sites receive the live PromoteSet** — `fold` (`fold.rs:492`), `pools_before`
  (`fold.rs:547`), `state_as_of` (`fold.rs:605`) all `res.promotes.clone()`; `universal_snapshot`
  (`transition.rs:67`) `promotes.clone()` forwarded from `resolve` (`resolve.rs:1499`). No
  `PromoteSet::new()` / phantom-empty at any site.
- **Single-site decompositions.** `clamped_leg_basis` (`conservative_promote.rs:165`) is the one clamp,
  called by BOTH `make_disposal_legs` (`fold.rs:209`) and `make_removal_legs` (`fold.rs:290`, with
  `net_proceeds_share = $0`). `make_removal_legs` is the sole removal-basis site; every downstream §170(e)
  consumer reads `leg.basis` by construction — `crypto_charitable_gifts` (`return_1040.rs:535`),
  `Form8283Row.cost_basis` (`forms.rs:154,213,440`), `printed.rs` (via `r.cost_basis`), `removals.csv`
  (`render.rs:795,848`). The prior-year advisory is keyed at one place (`promote_prior_year_advisory`,
  `conservative.rs:687`) over disposal ∪ removal LEG-SET diffs (Vec-eq, not Σ-gain). (One duplicated
  *formula* — not a divergent copy of the logic — is Minor-1 below.)
- **BG-D8 gate is a real CLI refuse-before-bytes.** `promote_export_gate` (`admin.rs:171`) runs FIRST in
  `export_snapshot`, `export_irs_pdf` (crypto-slice), and `export_full_return`, keyed on
  `disclosure_8275(...).incomplete`. The TUI cannot bypass it — `btctax-tui` is source-gated against
  `export_snapshot`/`write_csv_exports` (`export.rs` `e10_mechanized_source_gate`), so there is no second
  export surface on this branch. The T1 enum boxing (`Resolved::Accept(Box<EventPayload>, EventId)`,
  `resolve.rs:270`) is behavior-preserving (private enum, `(**payload).clone()` at the one use site,
  `resolve.rs:701`) and the headroom concern is real (`EventPayload` grew an `Acknowledgment`+`Vec`).

---

## Minor (recorded; do not gate)

### Minor-1 — the `estimate_share` decomposition FORMULA is duplicated (drift risk)
`consume_fee` (`fold.rs:415-418`) recomputes the estimate share INLINE —
`round_cents(entry.filed_basis * Usd::from(c.sat) / Usd::from(entry.tranche_sat))` — the byte-identical
expression that lives inside `clamped_leg_basis` (`conservative_promote.rs:174`). BG-D4 calls the fee
decomposition "the same single-site decomposition as the leg builder," but it is a second textual copy,
not a shared call. No current defect (identical formula, both `round_cents`), but a future change to the
pro-ration (e.g. rounding mode) in one and not the other would silently split the leg clamp from the fee
evaporation. **Fix:** extract `pub fn estimate_share_of(p: &PromoteEntry, leg_sat: Sat) -> Usd` in
`conservative_promote.rs` and call it from both `clamped_leg_basis` and `consume_fee`.

### Minor-2 — `filed_basis_for` success arm is a catch-all, not an explicit `Coverage::Full` match (T2)
`conservative_promote.rs:56-62`: the third match arm `Some(wr) => Ok(ComputedFloor { ...
coverage: Coverage::Full })` fires for any `wr` not caught by the `Partial` guard AND hard-codes
`coverage: Coverage::Full` regardless of `wr.coverage`. `Coverage` has exactly two variants today
(`Full`/`Partial`) and `window_reference` can only produce those, so it is currently harmless — but a
future third variant (e.g. a `Partial`-family) would silently take the Full path and file a floor over an
under-covered window (overstate basis → understate gain → violate G-4). **Fix:** `Some(wr) if wr.coverage
== Coverage::Full => Ok(...)` plus an explicit `Some(_) => Err(PromoteRefusal::PartialCoverage)` (or a new
refusal), so a new variant is a compile-forced decision, not a silent Full.

### Minor-3 — the void-direction advisory prints amend-to-PAY on an INERT tranche-void (T8)
Running `reconcile void <declare-tranche-with-a-live-promote>`: `void()`
(`reconcile.rs`) prints `promote_void_advisory_lines` BEFORE recording, and that helper finds the live
promote via its `.or_else` (`pt.target == target && !voided`, `reconcile.rs:1015-1023`) and prints the
`Direction::Void` advisory — "voiding reverts the floor to $0 … additional tax, plus interest, file
1040-X." But `void()` has no `would_conflict` pre-check; it records the void, and the engine adjudicates it
INERT (`resolve.rs:1243-1256` — the tranche keeps its live promote → `DecisionConflict`, the floor
stands). So the filer is told a reversion + amendment will happen when nothing changes. Non-gating, and
`verify` surfaces the `DecisionConflict`, but it prints materially-wrong tax guidance for a real user
action — exactly the "never print a misleading figure" posture BG-D6 spent five rounds enforcing. **Fix:**
gate the advisory to fire only when the void will be EFFECTIVE — i.e. when `target` IS the
`PromoteTranche` (the intended revert path), not when it is a `DeclareTranche` whose promote-void would be
inert; or soften to a conditional ("if this void takes effect …"). Recommend fixing in Phase 1a.

### Minor-4 — the `promote-tranche` verb is exposed with only procedural release-gating until Phase 1b
`Reconcile::PromoteTranche` (`cli.rs:906`) is a visible, un-hidden, un-feature-flagged subcommand, and
export emits only a plain-text `form_8275.txt` (`render.rs::write_form_8275_txt`). Reg §1.6662-4(f) /
SPEC §4 deem plain text INADEQUATE disclosure — the official AcroForm is Phase 1b (Tasks 15-16, unbuilt on
this branch). The PLAN consciously handles this ("Do NOT release — `promote` is not exposed in a
*released* binary until Phase 1b", `IMPLEMENTATION_PLAN.md:1337`), so merging an *unreleased* `main` is
spec-and-plan-sanctioned and this is NOT an Important. BUT the guarantee rests ENTIRELY on procedural
discipline: `main` is not branch-protected and has no release gate, so one accidental tag/publish between
the 1a merge and 1b would ship a user-reachable inadequate-disclosure path. **Fix (defense-in-depth,
cheap):** `#[command(hide = true)]` or a `promote` feature flag on `Reconcile::PromoteTranche` until
Task 16 lands, converting the procedural "don't release" into a code-level guard.

---

## Nit (recorded)

- **Nit-1 (T7) — `CONFLICT_HINT` duplicated.** Identical const in `live_promotes` (`resolve.rs:465`) and
  `resolve` (`resolve.rs:564`), "kept in sync" by comment. Hoist to one module-level `const`.
- **Nit-2 — `with_synthetic_promote` duplicated.** Private in `conservative_promote.rs:422` and re-copied
  in `cmd/promote.rs:625` (documented, because the core one is not `pub` and the crates differ). Benign —
  both yield the same `promotes` set (the promote folds `Op::Skip`; only `target`/`filed_basis` are read,
  so the differing timestamp/`now` is inert) — but two copies of the seq-minting logic. Consider `pub` +
  reuse.
- **Nit-3 (T10) — interactive TTY prints the consent screen twice** (`main.rs:1290-1318`): a discarded
  ack=None preview call, then the real call, each re-running `consent_terms` + the advisory + the
  gift-classification fold pair. Correctness unaffected (deterministic, same events); wasteful + double
  output. Restructure to compute figures once, then prompt.

## Triage adjudication (each independently assessed)

- **[T2] Coverage catch-all** → Minor-2.  **[T7] CONFLICT_HINT dup** → Nit-1.  **[T8] inert-void copy** →
  Minor-3.
- **[T8] mixed gift+disposal contradictory message** → RESOLVED in current code, NOT a finding: the gift
  fragment (`conservative.rs:892-906`) scopes "no amended return" explicitly to "year Y's gift(s) … the
  donor's own Form 1040 for Y is unaffected," while the disposal/donation fragment scopes the 1040-X to
  disposals/donations — the two are correctly attributed, not contradictory.
- **[T5] `rehome_onto_*` per-fn docs say "full basis carries"** → doc Nit (ownerless residue). The
  `FeeCarry` struct doc now names the estimate withholding (`fold.rs:315-318`); the three `rehome_onto_*`
  method docs could add a one-line BG-D4 cross-ref. Non-gating.
- **[T11] verify prints "drift: 0" vs TUI hides empty; methodology header wording over-general** → cosmetic
  Nits (ownerless residue); pick one convention / narrow the wording. Non-gating.
- **[T6] partial-promoted-removal cent-residue KAT** → optional (more-correct-than-SPEC); the whole-tranche
  KATs + the `clamped_leg_basis` floor-at-$0 logic already cover the residue. Nice-to-have Nit.
- **[T10] `wide_window_note` 365-day threshold** → acceptable implementer judgment (SPEC silent);
  informational, non-gating, over-a-year is a defensible "wide" line. Confirmed, no change needed.

## Test architecture (rubric item 5)

The KATs pin guarantees at the right seam, not impl details:
`snapshot_timing_the_floor_is_visible_to_pass1_conservation` discriminates the load-bearing rewrite
PLACEMENT via a HIFO reorder (a bare presence-check could not);
`promoted_pre2025_tranche_still_trips_the_d8_backstop` crafts an allocation matching the PROMOTED residue
exactly (so the denial is proven tag-keyed, not a totals coincidence);
`the_pre2025_conservation_snapshot_sees_the_fee_evaporation_not_a_phantom_basis`,
`relocated_with_fee_then_promoted_sold_below_floor_files_zero_gain_not_an_estimate_enabled_loss`, and the
both-emitters donation KAT (`..._deducts_documented_only_on_both_emitters`, asserting the computed 1040
Schedule A line, not just the fold) each survive the mutation they name. No KAT asserts an internal detail
in place of the guarantee; no missing seam I could find would ship a wrong FILED number green.
