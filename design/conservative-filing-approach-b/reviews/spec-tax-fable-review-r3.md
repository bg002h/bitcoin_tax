# Independent TAX review — Approach B sub-project 1 SPEC, round 3 (post r2-fold)

**Artifact:** `design/conservative-filing-approach-b/SPEC.md` (DRAFT, r1+r2 two-lens folds applied).
**Lens:** US federal tax correctness / completeness / honesty. Adversarial; every load-bearing claim
re-verified against statute/reg/case law AND current source (`project/{fold,pools,resolve}.rs`,
`tax/{return_1040,charitable,compute,printed}.rs`, `conservative.rs`, `forms.rs`, `state.rs`, `void.rs`,
`btctax-adapters/src/tax_tables.rs`, `btctax-forms/src/form8283.rs`, `btctax-cli/src/render.rs`).
**Provenance honored:** `DESIGN_PROVENANCE.md` + both arch reviews read; no adjudicated ruling re-litigated.
Parent-spec guarantees (amended Invariant, D-7 re-scope, D-8 exclusion) checked — no violation.
**Reviewer:** Fable (independent; not the author of the fold). **Date:** 2026-07-21.

**Verdict: NOT green. 0 Critical / 2 Important / 0 Minor / 2 Nit.**

All three r2 Importants are genuinely resolved — the removal-leg-builder ruling really does close every
§170(e)/8283/§1015 surface at one site (verified consumer-by-consumer, §V), the fee-draw evaporation is
complete and conservative, and the fold-diff re-key genuinely fires for the table-less/profile-less
audience years. The two new Importants are the last residue of the two r2 threads: the consent Σ's year
set, as re-worded, drops the CURRENT year's realized delta (the dropped half of the r2 M-2 fix), and the
fold-diff predicate is DISPOSAL-leg-scoped — blind to a promote that HIFO-reorders a prior filed year's
DONATION/GIFT removal legs (the fourth instance of the §170(e)/whole-surface class, and this one no other
gate catches). Both are one-clause fixes inside the existing BG-D6/BG-D9 structure.

---

## V — Verified resolved (r2 → fold, each checked against source/law)

- **I-1 (second §170(e) emitter) → RESOLVED.** BG-D11 now rules the decomposition at the
  **removal-leg builder** and I verified the single-site claim end-to-end against source: the ONLY
  place removal legs are born is `make_removal_legs` (`fold.rs`, `basis: c.gain_basis`), called from
  exactly the `Op::GiftOut` and `Op::Donate` arms; the only post-builder basis mutation is
  `rehome_onto_removal_leg` (documented fee cents — stays unclamped per BG-D4, correct). Every consumer
  reads the final `leg.basis`/`claimed_deduction` and therefore inherits: the fold's `claimed_deduction`
  (computed from FINAL legs, `Op::Donate` arm), **`crypto_charitable_gifts` (`return_1040.rs`,
  `short_basis += leg.fmv_at_transfer.min(leg.basis)` over `state.removals`) → `apply_170b`
  (`charitable.rs`) → `allowed_noncash` → Schedule A line 12 (`schedule_a_parts`,
  `charitable_noncash_12`) → taxable income**, the §170(d) `carryover_out` chain, the Form 8283
  `cost_basis` column (`forms.rs` `Form8283Row.cost_basis = leg.basis` → `printed.rs`
  `round_dollar(r.cost_basis)` → `btctax-forms/form8283.rs` `push_money(..., row.cost_basis, ...)` in
  BOTH the Section-A and Section-B writers → ST and LT rows), `removals.csv`'s `basis` column
  (`render.rs`), the §170(f)(11)(F) Section-A/B aggregate + qualified-appraisal blocker
  (`year_donation_deduction`, keyed on `claimed_deduction` — correctly shrinks with the claim), and the
  §1015 gift carryover (same builder). The `crypto_charitable_gifts` "reconcile" doc invariant is
  preserved (both sides now compute from the same final legs). §3 item 8 lists the chain; the §6 KAT
  asserts BOTH emitters including the computed 1040. **§6664(c) cite now correct:** (c)(2) is the
  exception removing the reasonable-cause defense for charitable-deduction-property valuation
  misstatements; (c)(3) is the qualified-appraisal special rule restoring it only for the *substantial*
  (not the Reg §1.6662-5(g) deemed-gross) case; (c)(4) definitions — matches current numbering.
- **I-2 (FIFO fee-draw back-channel) → RESOLVED.** BG-D4's new sub-bullet rules the estimate component
  of consumed fee-sats (fragment `origin_event_id` ∈ promote set; per-sat floor × fee-sats)
  **evaporates**; only documented fee basis re-homes. Verified complete against source: `consume_fee`
  (`fold.rs`) is the ONLY `FeeCarry` producer and `rehome_onto_{lot,disposal_leg,removal_leg}` the only
  three re-home sites (§3 item 8b lists all four); relocation preserves `origin_event_id`
  (arch r2 fact), so the promote-set keying survives; the pro-ration denominator (whole-tranche
  `tranche_sat`) means the evaporated share is never re-claimed by later legs (per-sat basis is
  invariant under `take_from`'s pro-rata debit, so documented-share ≥ 0 holds); the `TreatmentB`
  mini-disposition path needs no separate rule — its fee legs are ordinary `make_disposal_legs` output
  and the BG-D4 leg clamp covers them. Evaporation = basis forfeiture = conservative in every
  direction. §6 KAT pins the worked $1.20 corner.
- **I-3 (uncomputable-year blind spot, converged) → RESOLVED.** Both gates re-keyed to the
  profile/table/blocker-independent fold pair. Verified the premises: `BundledTaxTables::load()`
  inserts ONLY 2017/2024/2025/2026 (`tax_tables.rs` — 2018–2023 uncomputable forever);
  `compute_tax_year`'s three refusal doors (any Hard blocker / `table_for(year)` miss / no profile,
  `tax/compute.rs`) → `tax_total` `None` (`conservative.rs`); the fold itself needs none of those
  preconditions (legs/8949/removal content are produced regardless — blockers accumulate alongside), so
  the leg-set diff IS computable for every audience year. BG-D9 fires on the per-year leg-set diff with
  the tax-Δ clause only when Y computes; BG-D6's third bullet quotes tax-Δ only when both folds compute,
  else gain-Δ with the explicit "tax not computable" clause; the `Acknowledgment` records each term as
  computed-tax-Δ **or** gain-Δ-with-flag, and a genuinely-all-uncomputable promote never records a bare
  $0. §6 pins the table-less/profile-less advisory KAT and the uncomputable consent KAT. (Residue: the
  predicate's *surface* is disposal-only → new **I-2** below; that is a different hole, not a reopening
  of this one.)
- **M-1 (Form 8283 basis column) → RESOLVED** by the builder ruling — the column prints the documented
  component for ST and LT all the way to the AcroForm and `removals.csv` (chain verified above); no
  deduction-beside-floor mismatch survives; §3 item 8 carries the site chain.
- **M-2 (consent year-set scope) → PARTIALLY RESOLVED → new I-1.** The pre-promote reading is dead
  (BG-D6 now says "evaluated on the POST-promote fold", rejecting "already have disposed legs" read
  pre-promote — the reorder-created prior year is in). But the r2 fix direction was "every year the
  BG-D9 diff identifies, **plus the current year**" — and the fold dropped the second clause. See I-1.
- **N-1 (§6664(c) subsection) → RESOLVED** (verified above; the copy-seeding text now names (c)(2) for
  the deemed-gross removal and (c)(3) as the substantial-only carve-back).
- **N-2 (Jan-2013 figure) → RESOLVED.** §1 now reads "~$13/BTC — BTC's Jan-2013 close" (correct;
  BTC opened 2013 ≈$13.3) and keeps the Q4-2017 ≈$4.2k min-close (early-Oct 2017 — correct).

**Independent surface census (hunt (a), third pass over this class):** no OTHER surface a promoted
tranche's basis reaches that funds a deduction/credit/outbound carry beyond those the two rulings cover.
Verified: `RemovalKind = {Gift, Donation}` only (`state.rs`) — no §165 casualty/theft/abandonment path
exists; `PendingLeg` (`Op::PendingOut`, floor basis parked in `pending_reconciliation`) reaches NO filed
surface (no product consumer outside `state/fold`; the outflow is blocker-gated); the derive-side
Schedule A (`return_1040.rs` ~L762) is crypto-donation-free **by design** ("crypto donations belong to
the absolute return"); `schedule_d`/8949/capital-loss carryover consume clamped disposal legs; the
safe-harbor Σbasis conservation check (`resolve.rs`) is unreachable under D-8 mutual exclusion (backstop
KAT pinned); `whatif.rs`/optimize re-fold through `resolve` and inherit BG-D1 by construction; TUI/CSV
surfaces are projections of the same legs.

---

## NEW findings

### I-1 (BG-D6 / §6) — The consent Σ's year set, as re-worded, omits the CURRENT year's realized delta: the dominant term in the feature's most common flow (sell, then promote before filing).

**Defect:** BG-D6 defines the saving as "Σ of per-year clamped deltas over **every year the BG-D9
fold-diff flags**" — and BG-D9 defines its flag set as "any year **`< current`** whose per-year
DISPOSAL-LEG set differs". A tranche disposed earlier in the CURRENT year is neither flagged (not
`< current`) nor covered by the unrealized line ("for sats **not yet disposed**"). The r2 M-2 fix
direction said "every year the BG-D9 diff identifies, **plus the current year**"; the fold kept the
first half and dropped the second.

**Failure scenario:** filer sells the 1-BTC tranche in March 2026 below cost-of-living pressure,
runs P6, promotes in July 2026 before filing. The consent screen, implemented per the strict text:
prior-year flags (none), unrealized line (no undisposed sats) — the five-figure 2026 realized saving
and its matching §6662 exposure are absent, and the `Acknowledgment` snapshots figures that omit the
very number the filer is about to file. That is the r1 I-2 "bare $0 for a real position" defect
re-entered through the year-set door; the recorded §6664(c) artifact is again wrong-in-both-directions.
(The spec is internally inconsistent here: the §6 KAT "a below-window-low sale quotes the clamped
saving" presumes that sale's year IS in the Σ, but pins no year — an implementation with the sale in a
prior year passes the KAT while shipping the omission.)

**Authority/code fact:** BG-D6's own integrity requirement (the recorded artifact "cannot later be
shown to have quoted wrong (or silently zero) numbers"); the fold pair the machinery produces includes
the current year at zero extra cost.

**Fix (in-spec, one clause + one KAT word):** the Σ's year set = **every year the pre/post fold pair
differs — including the current year** (equivalently: the BG-D9 diff run WITHOUT its `< current`
advisory filter; the advisory keeps `< current` because only already-filed years need 1040-X copy).
Amend the §6 consent KAT to pin the disposed-CURRENT-year realized quote explicitly.

### I-2 (BG-D9 / BG-D6) — The fold-diff predicate is DISPOSAL-scoped: a promote that HIFO-reorders a prior filed year's DONATION or GIFT rewrites that year's Schedule A / Form 8283 / §1015 carryover with NO advisory — and the consent Σ is structurally blind to it even when the year computes.

**Defect:** BG-D9's trigger is "any year `< current` whose per-year **DISPOSAL-LEG set (equivalently
Σ-gain / 8949 content)** differs". Removals are not disposal legs and not 8949 content. But donations
and gifts draw through the SAME method-elected `consume_principal` (`fold.rs`: `Op::GiftOut` and
`Op::Donate` call it exactly as `Op::Dispose` does), so the same HIFO reorder BG-D9 exists to catch
(promoted lot exits `hifo_cmp`'s `usd_basis==0` sort-last, outranks cheaper documented lots) rewrites a
prior year's REMOVAL legs identically. And the consent Σ cannot compensate: its tax-Δ runs through
`compute_tax_year` (engine B), whose profile Schedule A is crypto-donation-free by design
(`return_1040.rs`: "crypto donations belong to the absolute return, not the frozen delta"), and its
gain-Δ fallback is $0 for a donation-only rewrite. Every gate reads $0/no-change while the filed
surface diverges.

**Failure scenario:** filer donated 0.5 BTC (ST) in 2025 from a wallet holding documented lots + the
$0 tranche; the 2025 donation drew documented lots (HIFO; tranche sorted last) → filed 2025 Schedule
A/8283 deducted `min(FMV, documented basis)` ≈ $30k. 2026: promotes the (undisposed) tranche whose
per-sat floor outranks those lots. Post-promote fold: the 2025 donation now draws the TRANCHE
(documented-only ≈ $0 deduction per BG-D11), the freed documented lots fund later-year 8949 basis —
the filed $30k deduction AND the documented basis are now both claimed across filed years, the exact
"documented basis double-counted" harm BG-D9 names, in the amend-to-PAY direction — and no year's
disposal-leg set, Σ-gain, or 8949 content changed in 2025, so nothing fires. The consent records $0.
The void direction mirrors it.

**Authority/code fact:** §170(e)(1)(A) + §170(d) (the collapsed deduction also corrupts the carryover
chain); §6662(d) (the silent later-year understatement); `consume_principal` call sites (`fold.rs`
Dispose/SelfTransfer/GiftOut/Donate — one draw mechanism, four leg surfaces);
`crypto_charitable_gifts` is year-scoped over `state.removals` (the prior year's Schedule A recomputes
from the rewritten legs); engine B's crypto-donation exclusion (`return_1040.rs` derive-side comment).

**Fix (in-spec, same stroke as the existing re-key):** widen the diff surface: the BG-D9 advisory (and
the BG-D6/I-1 year set) fires on any year whose **disposal-leg set OR removal-leg set** differs between
the folds — equivalently the year's filed content: 8949 rows + 8283/Schedule-A donation content +
`removals.csv` gift carryover. The copy gains a deduction clause: "changes year Y's reported gain by
~$G **and its charitable deduction by ~$D**" — $D is profile-free (Σ `claimed_deduction` per year from
the fold pair, exactly like the gain-Δ), so the uncomputable-year path stays honest. Add the
prior-year-donation-reorder KAT (advisory fires; consent quotes the deduction-Δ; both directions).
Add `crypto_charitable_gifts`-per-prior-year to the §3 item 8 note so the plan wires the diff there.

### N-1 (BG-D9) — "(equivalently Σ-gain / 8949 content)" is not an equivalence.

A reorder swapping equal-basis, different-date lots changes the leg set and the printed 8949 rows
(col (b)) with Σ-gain unchanged; an implementer taking the parenthetical literally and diffing Σ-gain
ships a predicate weaker than the one the KAT pins. Make the leg-set (filed-content) diff the operative
predicate and demote Σ-gain to "a consequence usually visible as".

### N-2 (BG-D6) — The unrealized line's "at today's price ~$X" has no defined behavior when today has no bundled close.

Bundled price data ends at release; "today" is typically after it. Under the spec's own loud-uncomputable
pattern (third bullet), state the fallback (latest bundled close + its date, or an explicit "no current
price data — the floor itself, $filed_basis, is the maximum gain reduction") rather than letting an
implementation print $0 or silently drop the line.

---

## Disposition

The r2 fold is genuinely complete on its own terms: the removal-leg-builder ruling is the right
single-site architecture and verifiably reaches every emitter (I re-derived the consumer set from
source rather than trusting the census), the evaporation rule closes the fee back-channel
conservatively, and the fold-diff re-key reaches the audience years. The two remaining Importants are
narrow: a dropped clause (current year) and a predicate one surface too narrow (removal legs) — both
fixable inside BG-D6/BG-D9 with the machinery the spec already mandates, plus two KAT pins. This is
the same lesson as every round of this review: when the taxonomy changes, the sweep must cover the
whole filed surface — disposals AND removals — in one pass. Re-review after fold.

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 2 |
| Minor | 0 |
| Nit | 2 |
