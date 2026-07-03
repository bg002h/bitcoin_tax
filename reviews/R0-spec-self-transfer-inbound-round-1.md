# R0 spec review — SPEC_self_transfer_inbound (Cycle A, inbound self-transfer-in) — round 1

**Artifact:** `design/SPEC_self_transfer_inbound.md`
**Reviewer:** independent adversarial architect (did NOT author).
**Baseline verified against current source:** branch `feat/self-transfer-inbound` @ `6fdb682`
(spec commit; `main` == `a740b3d`; the branch diff over main is spec-only, so `a740b3d`
source is the live source). All line numbers below re-grounded at this tree.

## Verdict

**0 Critical / 0 Important / 3 Minor / 3 Nit**

The spec is fundamentally **sound**. The tax rationale is correct and conservative in the
IRS-safe direction; the load-bearing invariant (`basis_pending: false`) is verified against
the real gating chain; non-taxability, conservation, the Advisory-not-Hard blocker, the
pool/HP orthogonality, and the reuse claims all check out against current source. No finding
gates implementation. The Minors are spec-accuracy / enumeration-completeness issues (every
one is *compile-forced or KAT-pinned*, i.e. mechanically surfaced — not a silent-behavior
risk), and are worth folding before Task 1/Task 3 so the implementer has the full site list.

Because R0's block bar is Critical/Important, **this gate is PASSED** — implementation may
proceed. The Minors should still be folded (cheap, and one corrects a self-contradiction in
the spec text).

---

## What was verified sound (the load-bearing claims)

- **Conservative direction (Req #2/#3, pressure-test #1) — CORRECT.** `$0` basis ⇒ later
  Sell gain `= proceeds − 0 = proceeds` (maximum) ⇒ never under-reports gain. Checked the
  down-stream non-sell paths too: a `$0`-basis ST **donation** deducts `min(FMV,0)=0` and a LT
  donation deducts FMV — both are the *actual* §170(e) figures (`fold.rs:1107-1116`), so the
  default under-claims the deduction (taxpayer over-pays) rather than mis-reporting to the IRS.
  Gift-out recognizes zero gain regardless. Receipt-date HP default ⇒ ST ⇒ ordinary rates ⇒
  more tax ⇒ conservative. FMV-at-receipt is rightly rejected (it would manufacture basis and
  under-report). **No case under-states a gain or a loss.**
- **`basis_pending: false` (G1, pressure-test #2) — CORRECT and won't gate.** Confirmed the
  chain: `Lot.basis_pending` → `Consumed.basis_pending` (`pools.rs:222`, verbatim copy) →
  `make_disposal_legs` `if c.basis_pending { add FmvMissing }` (`fold.rs:138-145`). A `$0`
  basis is a computable value, so `false` is right and the later disposal computes a real gain
  with no `FmvMissing`. This is genuinely UNLIKE `Op::Income`/`IncomeInbound` FMV-missing
  (`fold.rs:677`, `:859` → `true`) and `GiftReceived` case-4 (`fold.rs:936` → `true`).
  Copy-pasting the `IncomeInbound` arm's `basis_pending: pending` would indeed silently gate —
  G1 is a real trap and the spec pins it correctly.
- **Non-taxable (G2, pressure-test #3) — CORRECT.** The `IncomeInbound` arm's only taxable
  side effect is the `IncomeRecord` push (`fold.rs:843-850`); omitting it yields empty
  `income_recognized` for the event ⇒ nothing on Schedule 1/SE. No other side effect of
  `IncomeInbound` must be replicated except `sigma_in += sat` (which the spec keeps). Its
  `FmvMissing`-on-missing-fmv branch must NOT be replicated (basis is always computable here) —
  the spec correctly drops it.
- **Outside FIFO / invariant #7 (Req #4, pressure-test #4) — CORRECT, and load-bearing.**
  `honoring_principal` (`resolve.rs:1008`) has a `_ => None` catch-all; the new op falls to
  `None` ⇒ absent from the `honoring` map (`resolve.rs:772-775`) ⇒ a `LotSelection` targeting
  it hits the `None` arm of `selections.retain` (`resolve.rs:802-810`) ⇒ `LotSelectionInvalid`.
  So invariant #7 holds *because* the catch-all returns `None` — no `honoring_principal` change
  is needed (see M1 on the spec's mischaracterization of this).
- **Conservation FR9 (Req #5, pressure-test #5) — CORRECT.** `sigma_in += sat` is the correct
  and sufficient entry: the new lot's `remaining_sat` enters `sigma_held` at `finalize`, matched
  by `sigma_in` (`conservation.rs:51,62`), exactly as `Acquire`/`Income`/`GiftReceived` do. A
  pre-2025 receipt routes to `PoolKey::Universal` and is picked up identically by
  `transition::universal_snapshot` (which reuses `fold_event`, `fold.rs:526-528`), so no §7.4
  breakage — the new op is conservation-isomorphic to `Income`.
- **pool_key vs acquired_at orthogonality (pressure-test #6) — CORRECT.** `pool_key(date,…)`
  (`pools.rs:15-21`) keys on the receipt date; `acquired_at` carries the supplied-or-receipt
  date; `donor_acquired_at: None` ⇒ `gain_hp_start() == acquired_at` (`state.rs:106-108`). A 2026
  receipt with a supplied 2013 date ⇒ 2026 Wallet pool AND long-term. Verified.
- **Advisory blocker (pressure-test #7) — CORRECT.** `severity()` (`state.rs:62-82`) is an
  exhaustive two-arm match with NO catch-all, so adding `SelfTransferInboundZeroBasis` to the
  Advisory arm is compile-forced and lands as Advisory. `compute_tax_year` gates only on
  `severity()==Hard` (`compute.rs:237`), so Advisory never gates. Blocker DISPLAY is
  severity-driven (`render.rs:472 match b.kind.severity()`), not per-kind — so "no render
  change" is CONFIRMED, and `severity()` is the ONLY exhaustive `BlockerKind` match in the tree
  (no other display/verify/CSV site needs an arm). Firing on `basis.is_none()` (G4) matches the
  user-mandated Req #2 (attested `Some(0)` stays silent).
- **Reuse (pressure-test #8) — CORRECT.** `ClassifyInbound` collection/duplicate-first-wins/
  bad-target (`resolve.rs:529-579`) matches on the TARGET's payload being `TransferIn`, not on
  the `InboundClass` variant — so `SelfTransferMine` rides it unchanged; a void re-exposes the
  inbound as `UnknownInbound` (build_op falls through to `resolve.rs:279`). `classify_inbound`
  (`reconcile.rs:38-52`) takes `class: InboundClass` opaquely — unchanged. `build_op`'s
  `inbound_class` match (`resolve.rs:256-277`) IS exhaustive (no catch-all) ⇒ the C3 arm is
  compile-forced.
- **SemVer / serde (pressure-test #9) — CORRECT.** Additive struct-variant on an
  externally-tagged enum round-trips (`Eq` holds: `Option<Usd>`/`Option<TaxDate>` are `Eq`);
  old-binary-fails-loud matches the `ReclassifyIncome` precedent documented at
  `event.rs:198-214`; no `docs/manual`/GUI mirror exists; no new `EventPayload` variant ⇒ no
  fingerprint KAT (Cycle A rides `ClassifyInbound`).

---

## Findings (most-severe first)

### [M1] MINOR — G6 mischaracterizes the catch-all `Op` sites as compile-forced, and omits one

`design/SPEC_self_transfer_inbound.md` G6 (and Req #4 / C3) claims that adding
`Op::SelfTransferInbound` "forces a NEW ARM in EVERY exhaustive `Op` match … `is_disposition_op`
→ false, `honoring_principal` → None … a missed arm is a compile error, not a silent bug."

**This is inaccurate.** Of the four non-fold sites that see an `Op`:

- `is_disposition_op` (`resolve.rs:996-1004`) — `_ => false`. **Catch-all**, not exhaustive.
- `honoring_principal` (`resolve.rs:1008-1016`) — `_ => None`. **Catch-all**, not exhaustive.
- `evaluate.rs::honoring_sat` (`evaluate.rs:76-84`) — `_ => None`. **Catch-all**, and the spec
  does NOT enumerate this site at all.
- `fold_event` (`fold.rs:538`) — the only genuinely exhaustive `Op` match; here the new arm IS
  compile-forced (correctly identified by the spec).

So three of the four sites take the new op **silently** through a catch-all. **The saving grace
(and why this is Minor, not Important): all three defaults are the CORRECT ones** —
`is_disposition_op`→false, `honoring_principal`→None (which is precisely what makes invariant
#7 hold), `honoring_sat`→None (a self-transfer-in is not a scorable disposal). There is no
behavioral defect today.

**Why it still gates a fold:** the false "compiler will force it" framing invites the
implementer to *add* explicit arms to `honoring_principal`/`is_disposition_op`; if anyone adds
`Op::SelfTransferInbound { sat, .. } => Some(*sat)` to `honoring_principal` (mis-reading it as
"selectable"), invariant #7 breaks silently. It's KAT-pinned (#7), so it'd be caught, but the
spec's own guidance should not be wrong about which sites are exhaustive.

**Fix:** In G6, state that `is_disposition_op`, `honoring_principal`, and `honoring_sat`
(add this third site) each have a `_ =>` catch-all whose default (`false`/`None`/`None`) is
already correct — so the requirement is to **leave the catch-all untouched (or, if adding an
explicit arm, it MUST be `=> false`/`=> None`)**. Only `fold_event` (and `severity()` for the
blocker, and `build_op` for `InboundClass`) is compile-forced.

### [M2] MINOR — wallet-missing corner: C4 and G5 contradict each other on the blocker kind

C4's pseudocode says the wallet-missing guard yields "Hard **UnknownBasisInbound** + return"
(spec lines ~120-121), but it cites "the `IncomeInbound` wallet-missing guard (`fold.rs:830`)",
and G5 says to "**reuse the `IncomeInbound` wallet guard**." The actual `IncomeInbound` guard
(`fold.rs:830-839`) emits `BlockerKind::FmvMissing` with detail `"income inbound without
wallet"`. So the two halves of the spec disagree, and "reuse it verbatim" produces `FmvMissing`
— the wrong kind AND a message that says "income" for a non-income event.

Not a tax defect (both `FmvMissing` and `UnknownBasisInbound` are Hard → the return is gated
either way → no silent under-report). But it is an internal contradiction an implementer would
resolve arbitrarily, and `FmvMissing` here is semantically wrong (there is no FMV question —
basis is `$0`/supplied). It also has a cosmetic downstream symptom via the incomplete status
function in M3 (a `FmvMissing`-attributed self-transfer would render "Classified as Income(?)
but FMV missing").

**Fix:** Make C4 and G5 agree: emit `UnknownBasisInbound` (matches the "no lot could be
created / basis can't be established" semantics of the existing `UnknownInbound` path) with a
self-transfer-specific detail (e.g. `"self-transfer-in has no destination wallet"`). Do NOT
copy the `IncomeInbound` guard. Add a one-line KAT pinning the wallet-missing kind/return so
the choice can't drift.

### [M3] MINOR — TUI `InboundClass`/`InboundVariant` render enumeration is incomplete; "modal already covers it" is inaccurate

The spec's TUI section enumerates: `InboundVariant` + `SelfTransferForm` step + validator +
"Draw arm in `draw_classify_inbound_form` (`draw_edit.rs:591`)", and asserts the confirm modal
"reuses `ClassifyInboundModalState` (`as_: InboundClass` already covers it)."

Adding a third variant is **compile-forced at more exhaustive sites than the spec lists** (all
verified at current source):

- `draw_classify_inbound_modal` (`draw_edit.rs:728-763`) — exhaustive `match &modal.as_ {
  Income, GiftReceived }`. The *state struct* reuses fine, but this **render fn needs a new arm**
  — the "already covers it" claim conflates state with rendering.
- classify-inbound status `cls_desc` (`btctax-tui-edit/src/main.rs:2193-2198`) — exhaustive
  `match as_ { Income, GiftReceived }`; needs a `SelfTransferMine` arm (else no build).
- `InboundVariant` Tab-cycle (`main.rs:769-772`) — exhaustive 2-cycle; needs the cycle order
  extended (Income → GiftReceived → SelfTransferMine → Income).
- variant→form transition (`main.rs:783-800`) — exhaustive `match variant`; needs a
  `SelfTransferMine => SelfTransferForm` arm.
- VariantPicker draw (`draw_edit.rs:604-607`) and step-index helper (`main.rs:698-701`) — both
  need arms (the former is inside `draw_classify_inbound_form`, so the spec's pointer covers it;
  the latter is not mentioned).

Note the `if let InboundClass::Income { fmv, .. }` sites (`form.rs:1975,1991`) and the
`FmvMissing`-branch `match as_ { Income, _ => "?" }` (`main.rs:2168-2171`) are NOT exhaustive
and need no change (the new variant simply never matches / hits the harmless `"?"`).

Every listed site is **compile-forced** (exhaustive, no catch-all) → the workspace-lockstep
rebuild surfaces them, so this is Minor, not a silent risk. But Task 3 should scope them
explicitly (and drop the inaccurate "already covers it").

**Fix:** Expand the TUI section's site list to include `draw_classify_inbound_modal`, the
`cls_desc` status match, the Tab-cycle, the variant→form transition, and the step-index helper;
reword the modal claim to "the STATE struct is reused; `draw_classify_inbound_modal` gains a
render arm."

### [N1] NIT — advisory message omits the `btctax reconcile` command prefix

The C5/C4 advisory text says `"… (classify-inbound-self-transfer --basis)"`. Sibling messages
use the full invocation (e.g. `main.rs:2185` `"btctax reconcile void decision|{seq}"`). Align to
`btctax reconcile classify-inbound-self-transfer --basis … --acquired …` for copy-paste parity.

### [N2] NIT — no KAT for the pre-2025-receipt-date path or the wallet-missing corner

The 8 invariants cover 2026 receipts. Two cheap KATs would harden the corners the design leans
on: (a) a `date < TRANSITION_DATE` self-transfer-in → lands in `PoolKey::Universal`, survives
the §7.4 seed, conservation balanced (proves the `Income`-isomorphism under transition); (b) the
wallet-missing corner (ties off M2). Both are low-cost insurance.

### [N3] NIT — `acquired_at` is not constrained to ≤ receipt date

The validator (mirrored on `validate_classify_inbound_gift`, `form.rs:439`) does not check that
a supplied `acquired_at` is on/before the receipt date. A typo'd future date is *conservative*
(`is_long_term(future, sale)` → short-term), so this is data-hygiene only — worth a one-line
range check or an explicit "not validated; future date ⇒ short-term (conservative)" note so a
later reviewer doesn't mistake it for a hole.

---

## Decomposition / scope (pressure-test #10)

Cycle A (inbound-only) is the right first slice: the unmatched-inbound case is self-contained,
rides the existing `ClassifyInbound` decision (no new `EventPayload`/fingerprint), and is the
largest concrete user pain (the 50 BTC example). No Cycle B logic (passthrough drop / matcher /
confirmed-not-automatic) leaks in — it is cleanly deferred to FOLLOWUPS in Task 4. The TDD
phasing (core heart first, then CLI, then TUI) puts the highest-risk arm under test first.
Good.
