# Plan review — US-federal-tax-correctness lens (Opus) — ROUND 3

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `1c69d35` (branch `feat/conservative-filing-b`)
**Against:** r2 tax (`plan-tax-fable-review-r2.md`, 0C/1I/3M/2N) + r2 arch (I-1 = 2nd `FoldCtx`) + SPEC (GREEN) + current source.
Fresh independent lens — re-derived rather than trusting prior verdicts. Source re-verified: `window_reference ->
Option<WindowRef{min: Usd/BTC, coverage}>` + `Coverage {Full,Partial}` has no serde derive today (conservative.rs:174-186,
T1 correct); `verify(vault_path, pp) -> Result<VerifyReport,_>` is vault+passphrase only, no `PriceProvider`
(inspect.rs:146 — T11-3b's threading is genuine work); `crypto_charitable_gifts` reads `leg.fmv_at_transfer.min(leg.basis)`
(return_1040.rs:535 — 2nd §170(e) emitter inherits T6); `make_disposal_legs` non-dual arm `basis = c.gain_basis`
(fold.rs:193-201); `rehome_onto_lot` does `lot.usd_basis += gain_basis` at the self-transfer relocation call site
(fold.rs:291/:845); disposal `net = proceeds − fee_usd` (fold.rs:634).

## Verdict

**NOT GREEN — 0 Critical / 1 Important / 1 Minor / 1 Nit.** All six r2 findings the fold claimed are genuinely
resolved. The one blocker is FRESH: the BG-D4 clamp bound (T4) omits `documented_share`, so a promoted disposal
leg that carries a documented fee carry files a small **estimate-enabled loss** — violating BG-1 and contradicting
the 8275's own "limited so as not to report a loss from the estimate" narrative. Meets the Critical "unmet
guarantee" bar; rated Important because the magnitude is bounded by the (tiny) documented fee and the flawed
formula is inherited verbatim from the GREEN SPEC (BG-D4) — the fix requires a SPEC amendment, not just a plan edit.

---

## Verified resolved (r2 → status)

- **tax I-1 (verify-drift false-cover) → RESOLVED.** Real now: T11 owns `promote_drift_advisory` (Produces
  :1015-1016), Step 3b (:1072-1078) recomputes `filed_basis_for` vs the STORED `filed_basis` and threads a
  `PriceProvider` + `drift: Vec<String>` into `verify`/`VerifyReport` (confirmed the current sig lacks prices, so
  it is owned work, not a gesture). KAT `verify_drift_advisory_is_direction_aware_and_the_fold_still_uses_the_stored_number`
  (:1042-1052) pins the tax-critical **stored-above-recomputed** direction (`12k > 9k → "void"+"re-promote"`, G-4)
  AND the fold-uses-stored assertion (`usd_basis == 12_000` under corrected prices). Direction logic is correct
  (stored > recomputed = overstated basis = the anti-overstatement case). Residuals → Minor-1.
- **tax M-1 (amend-direction sign) → RESOLVED.** T8 Step 3 (:793-797) now correct: PROMOTE raises basis → lowers
  tax → amend-to-**refund**/§6511; VOID reverts to $0 → amend-to-**pay**; "copy follows the SIGN of the year's Δ,
  not the direction" — and it correctly notes a promote-direction HIFO reorder can also raise tax (pay). KATs pin
  both: `undisposed_promote…` asserts `§6511 && !contains("additional tax")` on a basis-increase year (:734);
  `the_void_direction_fires_amend_to_pay` asserts `"additional tax"` (:774).
- **tax M-2 (consent-copy mandates) → RESOLVED.** T10 KAT `consent_copy_pins_the_deduction_exclusion_and_unrealized_labels`
  (:943-949) asserts both the tax-Δ-excludes-deduction sentence ("does NOT capture this charitable-deduction
  change") and the "hypothetical, not a filed figure" label; Step 3 (:979-982) mandates rendering both.
- **tax M-3 (never-ComputedTax{delta:0}) → RESOLVED.** T9 Step 3 (:884-890) reworded to permit
  `ComputedTax{delta_usd:0, deduction_delta_usd:Some(D≠0)}` for a donation-only computing year and forbid only
  the bare `{delta:0, None}`; fixture stated donation-only. Residual KAT tightness → Nit-1.
- **tax N-1 (PROMOTE_ACK_PHRASE sweep) → RESOLVED.** All three T10 snippets (:953/:960/:967) now pass
  `Some(PROMOTE_ACK_PHRASE)`; the distinct const is mandated in Reference (:911) and Step 3.
- **tax N-2 (file-map + T14 KAT-name) → RESOLVED.** `compute.rs`/`cmd/tax.rs` moved to a **READ-ONLY reference**
  note (:130-132); T14 KAT and mutation line both read `…_but_incomplete_8275_…` (:1245/:1275).
- **arch r2 I-1 (2nd `FoldCtx` in `universal_snapshot`) → RESOLVED (cross-lens).** T4 now lists
  `transition.rs` in Files + commit (:413-420/:497) and pins it with
  `the_pre2025_conservation_snapshot_sees_the_fee_evaporation_not_a_phantom_basis` (:468-478); mutation =
  `&PromoteSet::new()` reds it (:504-505). Tax-relevant (conservation adjudicated against evaporated residue).

---

## Important

### I-1 (T4, `clamped_leg_basis`) — the clamp bound omits `documented_share`, so a promoted leg carrying a documented fee carry files a small estimate-ENABLED loss (BG-1 violation; 8275↔return mismatch)

- **Defect.** T4 Step 3 (:487-489) and the Produces formula (:437-440) compute
  `reported_basis = documented_share + min(estimate_share, max(net_proceeds_share, 0))` — the estimate is clamped
  against the **whole-leg** `net_proceeds_share`, then `documented_share` is stacked on top UNCLAMPED. When a
  promoted lot has received a TP8(c) documented fee carry (`documented_share = usd_basis_share − estimate_share > 0`
  — reachable exactly as SPEC:124-128 describes and as T4's own `relocated_with_fee…` KAT constructs; confirmed
  in source: `rehome_onto_lot` fold.rs:291/:845 adds the fee basis to the relocated lot's `usd_basis`, which
  becomes `c.gain_basis = usd_basis_share`), the estimate can claim proceeds that the documented basis also needs,
  pushing the leg below zero.
- **Worked corner (tax failure).** Promote 1 BTC to a $12,000 floor; a self-transfer re-homes $30 documented fee
  basis onto the lot (`usd_basis = 12,030`); sell for net $8,000 (below floor). `estimate_share = 12,000`,
  `documented_share = 30`, `net = 8,000`. Plan: `reported = 30 + min(12,000, 8,000) = 8,030` → **gain = −$30**.
  But documented-only basis is $30 against $8,000 proceeds — a $7,970 *gain*, never a loss; the −$30 exists ONLY
  because the estimate absorbed the full $8,000 of proceeds, crowding the $30 documented basis below zero. The
  correct conservative result is **gain = $0** (documented basis claimed first, estimate fills the remaining room
  down to zero-gain but no further). The estimate is the but-for cause of the loss → BG-1 "never manufacture a
  loss off the estimate" is violated, and the T13 8275 narrative "limited so as not to report a loss from the
  estimate" is factually contradicted by the filed −$30 (the examiner-mismatch tax r1 M-4 exists to prevent). The
  same crowd-out bites the sold-just-above-floor band `estimate ≤ net < estimate + documented`. Magnitude is
  bounded by `documented_share` (on-chain fee basis, ~cents to low dollars), which is why this is Important not
  Critical — but it is a structural (`by construction`) guarantee with a hole, and the plan's own KAT
  `relocated_with_fee_then_promoted_keeps_documented_fee_unclamped` (:454-460) PINS the wrong behavior
  (`assert!(leg.gain < 0)` calling it "attribution intact"). Note: the SPEC's *prose* invariant ("estimate-
  attributable gain ≥ 0 by construction; any negative gain attributable solely to documented fee/rounding") is
  right — it is the **formula** `clamp(net, …)` that fails to implement it (the SPEC borrowed the v1 "fee carry can
  drive a leg negative" reasoning, which is valid only for a lot with real *purchase* basis, not for an
  estimate-dominated promoted lot).
- **In-plan fix (T4).** Clamp the estimate against the proceeds REMAINING after documented basis:
  `reported_basis = documented_share + min(estimate_share, max(net_proceeds_share − documented_share, 0))`.
  This is surgical: `documented_share = 0` reduces to the current formula (all plain-promote KATs unchanged); the
  `fee_usd > proceeds` corner (`net < 0`) still yields `estimate = 0`, basis = documented, gain = net − documented
  < 0 = a *genuine* documented §1001(b) loss (so `estimate_basis_never_goes_negative…` still passes). Then FLIP
  the mis-pinning KAT: `relocated_with_fee_then_promoted…` sold-below-floor must assert `leg.gain == Usd::ZERO`
  (not `< 0`), and ADD a KAT for the sold-just-above-floor band (`estimate ≤ net < estimate + documented`) asserting
  `leg.gain == 0`, with the mutation "bound = `net` (no `− documented_share`)" named. **Because this changes the
  SPEC's BG-D4 formula and its worked invariant, it requires a SPEC amendment (escalate to the SPEC owner) — the
  gate cannot close on a plan-only edit.**

---

## Minor

### M-1 (T11 Step 3b) — the drift advisory's filed/unfiled branch is undeterminable by the engine, and only one direction is pinned

Step 3b (:1074-1078) keys behavior on "**not-yet-filed** position → void+re-promote / **already-filed** →
advisory-only," but the engine has **no filed-year concept** (SPEC's repeated fact; BG-D9's copy is conditional
for exactly this reason). As written an implementer cannot branch on filed-status — the honest form is conditional
COPY ("if this position is not yet filed, consider void + re-promote to $X; if already filed, advisory only — the
filed number stands"), mirroring BG-D9's "if Y was already filed." Also: the self-review claims a "both-directions
KAT," but the KAT asserts only the stored-**above** direction; the stored-**below** branch ("understated-floor
advisory") has no assertion (it is tax-safe, so non-blocking). Fix: state the conditional-copy framing in Step 3b
and add a below-direction assertion.

## Nit

### N-1 (T9, `a_computing_removal_flagged_year_carries_the_deduction_delta`) — the KAT does not pin the M-3 permission it exists for

The fixture is donation-only so `delta_usd == 0` is the exercised path (:890), but the KAT (:863) asserts only
`ConsentTerm::ComputedTax { deduction_delta_usd: Some(d) if d != 0 }` — it never asserts `delta_usd == Usd::ZERO`,
so it does not actually pin "`{delta:0, deduction:Some}` is the emitted term" (a mutation emitting
`{delta: nonzero, deduction: Some}` survives). Add `&& delta_usd == Usd::ZERO` to the match guard.

---

*Fresh-pass items checked and NOT filed: T3 snapshot-timing / backstop tag-keying (correct — the floor reaches the
snapshot via the timeline rewrite); T5 fee-evaporation site (`consume_fee` TreatmentC sum, fold.rs:349-357 — a
promoted tranche's fee-sats are 100% estimate, so evaporation → 0 re-home is right); T6 documented-only removal
basis + the 2nd-emitter reachability (return_1040.rs:535 verified); T8 baseline = exclude the PromoteTranche EVENT
not its target (correct diff); T9 clamped-saving arithmetic (floor 12k / proceeds 8k → tax on 8k, correct); T13
Part I = as-filed 8949 col (e) not the floor; T14 refuse-before-bytes; the amend-direction §6511 citation. The
I-1 clamp defect is the sole blocker.*
