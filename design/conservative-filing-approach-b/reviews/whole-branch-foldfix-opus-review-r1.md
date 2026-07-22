# Whole-branch fold-fix re-review — Approach-B Phase-1a Minor burndown (opus, r1)

Scope: the 6-commit Minor-burndown delta `645bc20..949b763` (crates/ only) applied
AFTER the whole branch already passed two-lens review at 0C/0I. Both lenses applied
(tax-correctness + architecture). Author ≠ reviewer; each fix re-derived against
current source, not the fix author's claims. Pre-existing (untouched) issues out of
scope.

## Verdict

**GREEN** — 0 Critical / 0 Important / 0 Minor / 1 Nit

No Critical or Important titles: **none**.

---

## Per-fix verification

### 1. `eb07593` — explicit `Coverage::Full` arm in `filed_basis_for` — SAFE
`conservative.rs:214-218` confirms `Coverage` has EXACTLY two variants (`Full`,
`Partial`). So today the match arms are exhaustive over real variants:
`None → NoCoverage`, `Partial → PartialCoverage`, `Full → computed floor`. The new
`Some(_) => Err(PartialCoverage)` residue (conservative_promote.rs:68) is **statically
unreachable** for the two current variants — behavior is byte-identical to the former
catch-all for both real cases. The computed arm is unchanged operand-for-operand
(`round_cents(wr.min * Usd::from(sat) / Usd::from(SATS_PER_BTC))`, `coverage: Full`).
Error variant reused for the residue is `PartialCoverage` — correct: a future variant
shares the "not provably the TRUE window min" cause (same semantics as Partial), so it
refuses rather than silently overstating basis (G-4). A new variant becomes a
compile-forced decision, not a silent Full default. No behavior change today.

### 2. `2aafb12` — `estimate_share_of` extraction + fee-fragment `.max($0)` — SAFE (tax-critical)
Extracted helper `estimate_share_of` (conservative_promote.rs:150-152) reproduces the
EXACT prior expression: `round_cents(p.filed_basis * Usd::from(leg_sat) /
Usd::from(p.tranche_sat))`.
- `clamped_leg_basis` (line 188): former inline used
  `p.filed_basis * Usd::from(leg_sat) / Usd::from(p.tranche_sat)` → **identical**
  operands, order, and `round_cents`.
- `consume_fee` (fold.rs:419): former inline used
  `entry.filed_basis * Usd::from(c.sat) / Usd::from(entry.tranche_sat)` →
  `estimate_share_of(entry, c.sat)` is **identical**.
No value shift at either site ⇒ the BG-D4 clamp and the fee-evaporation withholding are
unchanged.

The new `.max(Usd::ZERO)` on the fee fragment: the mapped value is the RE-HOMED
documented remainder, `(c.gain_basis − estimate_share).max($0)`. Re-derivation:
effective withheld = `c.gain_basis − rehomed = min(estimate_share, c.gain_basis)
≤ estimate_share`, so `.max` can only **decrease** the amount withheld (caps it at the
fragment's own basis) — it CANNOT increase the estimate withheld. The re-homed value is
now `∈ [0, c.gain_basis]`, so it can never inject negative basis onto the survivor and
never re-homes more than the fragment holds ⇒ **cannot manufacture a loss** and never
funds a carry with estimate money (BG-D11 preserved). This is the principled
`documented-remainder = max(basis − estimate, 0)`; only a sub-cent rounding residue
between the pool's pro-rata `gain_basis` and `estimate_share_of` can trigger it.
All tax-critical characterization tests re-run GREEN:
`tranche_fee_draw_evaporates_estimate_then_sale_files_zero_loss`,
`estimate_basis_never_goes_negative_when_fee_exceeds_proceeds`,
`promoted_removal_evaporates_estimate_but_keeps_the_documented_fee_carry`,
`relocated_with_fee_then_promoted_sold_below_floor_files_zero_gain_not_an_estimate_enabled_loss`,
`the_pre2025_conservation_snapshot_sees_the_fee_evaporation_not_a_phantom_basis`,
`a_pre2025_promoted_disposal_below_floor_clamps_on_the_real_fold_path`,
`below_window_low_sale_quotes_the_clamped_saving_not_an_unclaimable_loss`,
`sold_just_above_floor_band_still_files_zero_gain`, `filed_basis_is_whole_tranche_scaled`,
`clamped_leg_basis_is_identity_when_not_promoted`.

### 3. `20b12be` — hoist `CONFLICT_HINT` to one module const — SAFE
`resolve.rs:455` holds the single `const CONFLICT_HINT`, value identical to both former
locals (`"see \`btctax events list\` for event refs + decision status"`). Both former
`live_promotes`/`resolve` locals are gone (replaced by pointer comments at 474/565);
all ~20 `{CONFLICT_HINT}` format sites now bind the one module const (same module → in
scope for both fns). No stray redefinition. Pure move, no behavior change.

### 4. `4546cba` — `#[command(hide = true)]` on `Reconcile::PromoteTranche` — SAFE
Flag present at cli.rs:910. The dispatch arm `Reconcile::PromoteTranche { … }` remains
wired at main.rs:1188 — hidden ≠ disabled; the verb still parses and dispatches. No
test/census asserts `promote-tranche` visibility in `--help` (grep clean), so hiding
breaks nothing. Man-page regen lives in the excluded docs commit (out of scope).

### 5. `949b763` — remove unreachable inert-void `.or_else` branch + pin the guard — SAFE
The removed `.or_else` handled a `DeclareTranche` target with a live non-voided promote.
That case is refused UPSTREAM: `void()` calls `guard_decision_conflict` (reconcile.rs:320)
BEFORE `promote_void_advisory_lines` (line 344); `would_conflict` (mod.rs:107) runs the
real projection, where a tranche-void whose target still carries a live promote yields a
NEW `DecisionConflict` "void targets a DeclareTranche held in force by a live
PromoteTranche" (resolve.rs:1244-1254). So the `?` at line 326 returns before the
advisory ⇒ the removed branch was genuinely dead. The REACHABLE-correct path (voiding a
`PromoteTranche` directly → the effective amend-to-PAY advisory) is untouched. Second
caller (bulk, reconcile.rs:901) can't feed a `DeclareTranche`-with-live-promote target
(`voidable_decisions`' `promoted_target` filter, void.rs:107-118); even if it did, the
new fn returns `Vec::new()` for a non-`PromoteTranche` target — no wrong advisory. For a
`DeclareTranche` with an ALREADY-VOIDED promote, both old and new return empty (void is
allowed, prints nothing) — no behavior change. `decision_is_voided` fully removed, zero
remaining refs. New test
`voiding_a_promoted_declare_tranche_is_refused_and_prints_no_amend_advisory` pins the
guard: asserts refusal (`code != 0`, `"cannot record this decision"` +
`"held in force by a live PromoteTranche"`), no `additional tax`/`1040-X` on stdout, and
zero decisions appended. The positive test
`voiding_a_promoted_tranche_prints_the_void_direction_advisory` still asserts the
effective-void advisory. Both GREEN. No reachable behavior lost.

## Nit (non-gating)
- fold.rs:416 — the comment calls the clamped value the fragment's "withheld
  contribution"; the value clamped is actually the RE-HOMED *documented remainder*
  (`gain_basis − estimate_share`), not the withheld estimate share. Wording only; the
  code is correct and the intent ("floor each fragment at $0") is accurate.

## Validation
`cargo test -p btctax-cli --test promote_cli voiding_a_promoted` → 2 passed. The
clamp/evaporation `btctax-core` characterization suite listed above → all passed. Delta
compiles clean.
