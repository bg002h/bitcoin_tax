# SPEC — Conservative / Defensive Filing, **Approach B** — sub-project 1: the basis-floor engine + Form 8275

**Status:** DRAFT — **r1 two-lens review FOLDED** (tax 1C/4I + arch 0C/6I, both persisted verbatim in
`reviews/spec-{tax,architecture}-fable-review-r1.md`); pending the **r2** re-review to 0C/0I per
`STANDARD_WORKFLOW.md`. The r1 Critical (tax C-1, the §170(e) charitable-deduction leak) is closed by the new
BG-D11; all 10 Importants folded into BG-D1/D4/D6/D7/D8/D9 + §3's two censuses.
**Branch:** `feat/conservative-filing-b` (off `main` @ the v0.8.0 release).
**Parent:** the shipped Conservative-Filing v1 (`design/conservative-filing/SPEC.md`, v0.8.0). This spec is the
first of Approach B's sub-projects; the guided **wizard** (sub-project 2) and **VARIOUS multi-date rows**
(sub-project 3) are later specs.
**Design provenance:** brainstormed 2026-07-21, then an independent Opus critique + a Fable-architect
adjudication (summarized in `reviews/DESIGN_PROVENANCE.md`). The adjudication overruled parts of both the
initial design and the Opus critique; this spec is the adjudicated design of record.

---

## 1. Purpose & guardrails

v1 files the safe end of the fairness↔attack-surface curve ($0 basis, maximum gain, no understatement risk)
and *quantifies* the other end (P6's "reconstructing this tranche could save ~$X"). **Approach B builds the
actuator G-3 promised:** it lets a filer, **knowingly and on the record**, promote a v1 `$0` tranche to a
filed **>$0 basis floor** — reducing the reported gain on money genuinely spent — backed by the mandatory
Form 8275 disclosure. This is a filer choice along the curve, never a silent default.

The real filer: a cash/P2P (LocalBitcoins-era) or **dead-exchange** (Mt. Gox, BTC-e, Cryptsy) purchaser with
no payment records but a **tight, on-chain-boundable** acquisition window. For a multi-BTC "December-2017,
~$12k window-min" tranche the $0-vs-floor delta is five figures of real tax, and the position is
*Cohan*-plus-attested-window — rational and litigable. **Two honest limits the product MUST state:** (i) a wide
window yields a trivial floor (a 2013–2017 min daily close is ~$65/BTC; the min daily close over all of
Q4-2017 is ≈$4.2k, not the ~$12k of a tight late-December window) — for that filer, file `$0` and skip the
audit surface; (ii) an **on-chain receipt timestamp bounds only `window_end`** (an exchange purchase precedes
the on-chain withdrawal), so the filer must set `window_start` early enough to actually cover the purchase —
which honestly *widens* the window and *lowers* the floor. Both are anti-overstatement guardrails (G-4).

- **G-1..G-4 (inherited from v1) hold.** Every disposal reported; character/box derived, never assumed.
- **BG-1 Never understate — as a KNOWING choice, enforced structurally (not by trust).** A floor may reduce
  a gain toward zero; it may **never manufacture a loss** off the estimate (BG-D4 clamp). It is filed only
  behind an explicit two-sided informed-consent acknowledgment (BG-D6) that states both the saving and the
  penalty exposure.
- **BG-2 Provenance-neutrality is preserved the ONLY way it can be: the ENGINE asserts nothing; the FILER
  attests.** A window-min-*close* floor is an estimate of *purchase* cost — legally baseless for coins that
  were gifted (donor-carryover) or received as unreported income. The engine never asserts provenance; the
  promote **requires the filer to attest purchase-in-window** (BG-D5) and refuses otherwise, pointing
  gift/inherited filers at real-acquisition modeling.
- **BG-3 The 8275 MITIGATES, it does not immunize.** A `>$0` estimated basis carries §6662 exposure. The 8275
  + a good-faith methodology support a §6664(c) reasonable-cause defense and rebut §6662(b)(1) negligence, and
  give the position a *Cohan* reasonable-basis footing — but they are **not a safe harbor**, and adequate
  disclosure does **not** protect against the §6662(e)/(h) valuation-misstatement penalty. The filer-facing
  copy must say so (BG-D7).

## 2. Resolved decisions (the adjudicated design)

- **BG-D1 `PromoteTranche` is a decision layered on a `DeclareTranche`, resolved as pass-2 Op-construction, and
  MINTS NO NEW IDENTITY. ★ (the single most important ruling).** The payload (fields resolved across BG-D2..D6),
  all fields typed (arch r1 M-4):
  `EventPayload::PromoteTranche { target: EventId /* the DeclareTranche decision */, method: FloorMethod,
  filed_basis: Usd /* WHOLE-TRANCHE basis = round_cents(window_min_close_price × sat / SATS_PER_BTC); computed
  at record time + STORED, BG-D3 — NOT a per-BTC price (arch r1 M-2) */, coverage: Coverage /* snapshot, BG-D3
  */, provenance_attested: bool /* BG-D5; precedent `timely_allocation_attested: bool`, `event.rs` */,
  acknowledgment: Acknowledgment /* BG-D6: the typed consent PHRASE + a snapshot of the saving/exposure figures
  the filer was shown + the attested provenance text/version — a struct, not a bare bool (tax r1 N-2), so the
  recorded artifact IS the §6664(c) good-faith evidence */, part_ii_narrative: String /* BG-D7 Reg
  §1.6662-4(f) filer facts, captured at promote time, present-by-construction (arch r1 I-5) */ }` — no
  `acquired_at`, no partial-sat (whole tranche only). A promoted tranche stays
  `BasisSource::EstimatedConservative` — same unprovable origin, same D-8 mutual-exclusion obligations, same
  disclosure obligations, same Form 8283 `Review`, same record-time refusal semantics. **Only its filed number
  changes.** The resolver, when a non-voided `PromoteTranche` targets a `DeclareTranche`, emits that target's
  `Op::Acquire` with `usd_cost = the stored filed basis` (`basis_source` UNCHANGED). **No new `BasisSource`
  variant.** ★ **The rewrite happens INSIDE the `resolve` timeline build** (the `Op::Acquire` admit site /
  `build_op`, `resolve.rs`), **NOT** as a post-`resolve` timeline mutation — the `overpayment_delta_one` what-if
  seam (`conservative.rs`) is the right *swap* but the wrong *timing*: it mutates after `resolve()` returns, so
  pass-1 §7.4 effectiveness + `universal_snapshot` never see the swapped basis (arch r1 M-3). The promote map
  MUST be applied so step-3's snapshot sees the floor, else allocation conservation is adjudicated against a
  different pre-2025 residue than the fold consumes (a promoted-basis HIFO re-order changes which lots remain).
  Consequence, verified against source: the D-8 backstop (`resolve.rs` `snap.estimated_conservative_remaining_sat`),
  the Path-A seed exemption (`transition.rs`), the self-transfer relocation carve (`fold.rs`), both record-time
  refusal directions (`cmd/tranche.rs`, `session.rs`), and the tranche⇄Rev-Proc-2024-28 mutual exclusion ALL
  keep holding **by construction, with zero change** — this kills the tag-invisibility hazard class rather than
  enumerating a sweep to patch it. HIFO is correct either way (a promoted lot leaves `hifo_cmp`'s `usd_basis==0`
  special-case and sorts by its real per-sat filed basis — HIFO applied to the as-filed basis, not a bug — but
  see BG-D9 for the prior-year retroactivity that same re-order creates). Vault-compat: the stock forward-only
  doc note (older binaries fail loudly on the new variant; precedent `ReclassifyIncome`/`DeclareTranche`,
  `event.rs`) + the stock no-fingerprint KAT (`persistence.rs`) apply (arch r1 N-2).
- **BG-D2 `FloorMethod = { WindowLowClose }` — ONLY.** The enum is kept for B-future extensibility, but v1
  has exactly one method.
  - **§1014 (inherited) is OUT** (→ a separate real-acquisition feature). It is a *statutory* basis (DoD FMV),
    not an estimate; routing it here makes the good-faith-estimate attestation false, **breaks BG-2** (§1014
    requires asserting inheritance), and forces a contradictory term rule (§1223(9) auto-long-term vs the
    conservative `window_end`). The v1 P6 §1014 note already points the inherited filer at the right
    destination (a real acquisition import with statutory basis + §1223(9)) — which this codebase does not yet
    model (`event.rs` has no `Inherited` `BasisSource`), so it is its own small feature.
  - **"PartialRecords" is OUT — redundant.** A filer with partial substantiation records a *real documented
    acquisition* (the existing import flow, zero new code); needs no 8275 and no estimate. The tranche path is
    *definitionally* for the cannot-substantiate case.
- **BG-D3 The filed basis is COMPUTED, not chosen, and STORED.** There is **no `--basis` flag** and **no
  `acquired_at` field.** `filed_basis` = the window-min daily *close* (`window_reference`, P5), computed at
  record time and **stored on the `PromoteTranche` event** together with the `Coverage` snapshot; the fold uses
  the stored number forever. (A later bundled-price-data update must never silently move an as-filed position;
  `verify` recomputes and surfaces any drift as an advisory only — **direction-aware (tax r1 N-3):** a stored
  floor now recomputing *above* the reference (the stored number would overstate basis) on a not-yet-filed
  position earns a "void + re-promote to the corrected lower number" hint (G-4 anti-overstatement); drift on an
  already-filed year stays advisory-only.) **`Coverage::Full` is REQUIRED** — a
  `Coverage::Partial` covered-part min can EXCEED the true window min (overstate basis → understate gain →
  violate G-4); `Partial`/`None` are a HARD refusal ("narrow the window until it is fully covered, or file
  $0"). Because the number is mechanically reproducible, it is disclosable — which is the whole point. (The
  v1 "warn when `filed_basis > window_reference`" guardrail is now unrepresentable and dropped.)
  - **Substantiating a date narrows the WINDOW, it is never an `acquired_at` override.** Void + re-declare the
    tranche with a tighter window: that moves `filed_basis` (higher window-min) AND `window_end` (term)
    *coherently* from one declared fact. A date that moved term without moving the window is the exact
    incoherence G-4 forbids.
- **BG-D4 The loss clamp (fold-time) — the never-understate structural guarantee.** Per disposal leg, the
  **estimate-claimed basis is `clamp(net_proceeds_share, $0, estimate_share)` = `min(estimate_share,
  max(net_proceeds_share, $0))`** — the estimate-attributable gain is clamped `≥ 0` AND the estimate basis is
  never negative (this closes both the parent-KAT `fee_usd > proceeds` corner (`fold.rs` netting) and the
  cent-scale negative-remainder corner — tax r1 I-1/M-2; a `min(floor, net)` alone would file a NEGATIVE
  estimate basis). **Unclaimed floor EVAPORATES** (it never shifts to another leg — fabrication).
  - **★ The estimate component is the STORED `filed_basis` pro-rated over the tranche's sats — NOT the lot's
    `usd_basis` (arch r1 I-2 / tax r1 I-1).** The TP8(c) SELF-TRANSFER fee carry re-homes documented fee basis
    *into* the surviving lot's `usd_basis` (`rehome_onto_lot`, `fold.rs`) — via the product's own P8-recommended
    Exchange→SelfCustody move — where it is indistinguishable from the estimate. So `estimate_share =
    filed_basis × leg.sat / tranche_sat` (keyed via `leg.lot_id.origin_event_id` → the promote set the builder
    is threaded); `documented_share = usd_basis_share − estimate_share`, and **only the estimate component is
    clamped** — the documented components stay UNCLAMPED exactly as the amended v1 Invariant enumerates
    (§1001(b) `fee_usd` netting, the TP8(c) fee-sat carry, cent-scale rounding can still drive a leg negative;
    those are documented, real amounts). Amended invariant: *"a promoted-tranche leg's estimate-attributable
    gain is `≥ 0` by construction and its estimate basis is `≥ $0`; any negative gain remains attributable
    solely to documented fee/rounding, never the estimate."* (Precedent that reported-basis ≠ pool-basis is
    tractable: the §1015 NoGainNoLoss zone already does it, `fold.rs`.)
- **BG-D5 Record-time provenance attestation (BG-2's load-bearing half).** The promote refuses unless the filer
  records an explicit **purchase-provenance** attestation. The affirmative clause is operative — *"these units
  were acquired **by purchase** within the declared window"* — and the negative enumeration is closed so it
  cannot be misread *expressio unius* (tax r1 M-6): *"…not by gift, inheritance, mining, staking/earning,
  airdrop, fork, or any acquisition other than purchase."* A miner/earner/airdrop/fork recipient has an FMV-at-
  receipt income basis documented from the return (Notice 2014-21; Rev. Rul. 2019-24) — a *real*-acquisition
  path, not an estimate — so any non-purchase filer is refused and pointed at real-acquisition modeling. The
  attested text (+ its version) is stored in the `acknowledgment` struct (BG-D1), not discarded.
- **BG-D6 Record-time informed-consent acknowledgment — ONE command, two-sided.** `promote-tranche` shows a
  consent screen quantifying BOTH sides using existing machinery, with two corrections the reused seam requires:
  - **The figures re-fold through the CLAMPED promoted path, not the raw swap (tax r1 I-3).** The saving/exposure
    is `Σ` over years of `tax($0) − tax(clamped floor)`, computed by threading a synthetic promote set into the
    what-if so the BG-D4 loss clamp engages. The raw `overpayment_delta_one` swap has no promote event, so its
    clamp never binds — on a sale below the window low it would quote a saving that includes a **loss the filed
    promote is structurally forbidden to claim**, overstating both the displayed saving and (since exposure =
    the same delta) the at-risk tax. Never advertise a number only true *reconstruction* (documented records,
    loss legitimately claimable) could deliver.
  - **The quantification is defined for the promote-BEFORE-disposal flow (tax r1 I-2), the planning case the
    product itself sells.** `overpayment_delta_one` is year-scoped to *realized* disposals (`conservative.rs`
    re-folds `compute_tax_year`), so for an undisposed tranche `tax($0) − tax(floor) = $0` in every year — a
    bare "$0 saving / $0 exposure" is FALSE in both directions for a five-figure latent position. So: the saving
    = `Σ` of per-year clamped deltas over all years that already have disposed tranche legs; PLUS, for sats not
    yet disposed, an explicit **unrealized** line — *"saving and exposure accrue at disposal; at today's price
    the floor would reduce reported gain by ~$X (hypothetical, not a filed figure)"* — **never a bare $0.**
  - Plus interest, the penalty statement (BG-D10), and the wide-window "this floor is trivial" note when it
    applies. A **typed acknowledgment** — the consent phrase + a snapshot of the exact figures shown + the
    attested provenance — **is recorded ON the event** (BG-D1's `Acknowledgment` struct), so the recorded
    good-faith artifact cannot later be shown to have quoted wrong numbers. No second `attest` verb —
    record-time consent + an export-time artifact gate (BG-D8) is the friction; there is no watermark (clean
    export — matches the standing full-return DRAFT-gate policy: attestation/DRAFT stays pseudo-only). The
    non-interactive path (scripted/non-TTY) either refuses or requires an explicit `--i-acknowledge <phrase>`
    form (arch r1 M-4).
- **BG-D7 Form 8275 — mandatory, honestly framed, phased.** Mandatory for EVERY promote (the §6662(d)
  understatement is measured RETURN-WIDE, so its ≥10%/$5k threshold is unknowable at promote time; the
  auto-generated 8275 costs nothing and buys threshold-independent optionality + negligence/good-faith
  evidence). Position framing: *"basis estimated at the minimum daily closing price over the attested
  acquisition window (Cohan; the bearing-heavily minimum)."* The **copy NEVER says "safe harbor" or promises
  penalty immunity**; it states the mitigation honestly and the **20% / 40% worst case** (below). The
  disclosure honestly says "minimum daily **closing** price" (intraday lows can be lower — P5's caveat). When
  the BG-D4 clamp bound a leg (filed basis = net proceeds < the floor), the generated narrative adds one
  sentence — *"limited so as not to report a loss from the estimate"* — so the disclosed method matches the
  filed amount and cannot read as an examiner mismatch (tax r1 M-4). Part I auto-filled (item = Form 8949
  col (e); form/line/description/amount).
  - **Part II narrative — REQUIRED filer facts, captured at promote time, present-by-construction (arch r1
    I-5).** Reg §1.6662-4(f) wants facts "in sufficient detail"; a pure template reads as one. The narrative's
    home is the `part_ii_narrative: String` field ON the `PromoteTranche` event (BG-D1), authored at
    `promote-tranche` time alongside the consent (consistent with BG-D6's "recorded ON the event"), edited via
    void + re-promote. Auto-scaffold prefills it; an empty/scaffold-only narrative is REFUSED at record time
    (not deferred) so the artifact is present-and-non-empty by construction — which is what makes BG-D8's gate
    real.
- **BG-D8 Export-time completeness gate — a REAL refusal, not the `basis_methodology.txt` pattern (arch r1
  I-5).** A form packet (CSV, `export-irs-pdf`, or full-return) that contains a promoted leg WITHOUT its 8275
  artifact is a HARD refusal on those surfaces. **The mechanism is the pseudo-export-block precedent**
  (`fold.rs` "export/forms are BLOCKED while this is active"), **NOT** the `basis_methodology.txt` presence
  "gate" — that artifact is auto-generated and unconditionally written on export (`render.rs`), so it can never
  actually be absent and has no refusal machinery to mirror. The 8275 is the first artifact that CAN be absent
  (the PDF, Phase 1b) or content-incomplete, so the gate must genuinely refuse. Clean export, no watermark.
- **BG-D9 Lifecycle — engine-adjudicated, not CLI-only (arch r1 I-3).** Revocable via `VoidDecisionEvent`
  (void → the tranche reverts to `$0`, the intact `DeclareTranche`). A second live `PromoteTranche` on one
  target → `DecisionConflict` (not last-wins).
  - **Dangling-target impossibility holds by CONSTRUCTION, at the engine.** "Never a dangling target" is not a
    CLI guard: the void classifier applies a void of any non-listed decision via the `Some(_)` catch-all
    (`resolve.rs`), and an internal consumer (`session.rs` `safe_harbor_residue`, which filters out
    `DeclareTranche` but would keep a `PromoteTranche`) can produce the dangling shape with no CLI in the loop.
    So: (i) voiding a `DeclareTranche` that has a live promote is **resolver-adjudicated** — the void is inert +
    `DecisionConflict`, mirroring void-of-effective-allocation (`resolve.rs`), surfacing at record time for free
    via `would_conflict` (`project/mod.rs`); (ii) a `PromoteTranche` whose target is absent/voided/wrong-type is
    a hard `DecisionConflict` (the pass-1d/1e validation pattern), never silently inert; (iii) `PromoteTranche`
    is added to the `voidable_decisions`/`is_revocable_payload` set AND `DeclareTranche` is EXCLUDED from the
    bulk-void candidate set while it carries a live promote (`void.rs` — today `DeclareTranche` is
    unconditionally a candidate), so the bulk sweep cannot create the dangling shape either.
  - **Promote over an already-DISPOSED tranche is ALLOWED** (amending a filed `$0` year to claim the floor via
    Form 1040-X is a legitimate refund path; the engine has no filed-year concept to refuse on).
  - **★ The prior-year-delta advisory triggers on the correct predicate — a computed-tax DIFF, not "disposed"
    (arch r1 I-4 / tax r1 I-4).** Gating on "promote over an already-disposed tranche" (evaluated pre-promote)
    MISSES the default-HIFO retroactivity: promoting an **undisposed** tranche whose per-sat floor exceeds a
    documented lot's per-sat basis re-orders a PRIOR year's HIFO draw (`pools.rs` — the promoted lot exits the
    sort-last `usd_basis==0` special-case and now outranks cheaper documented lots), silently rewriting that
    year's legs/8949/tax with NO advisory and creating a real later-year understatement (documented basis
    double-counted across filed years). So the advisory fires for **any year `< current` whose computed
    `tax_total` changes between the pre- and post-promote fold** (the same before/after machinery the consent
    screen uses) — which also resolves the partially-disposed ambiguity ("disposed" = has any disposed leg).
    Copy: *"this promote changes year Y's computed tax by ~$Δ; if Y was already filed, claiming it requires a
    Form 1040-X for Y with the 8275 attached"* — conditional on "if Y was already filed" (the engine has no
    filed-year concept, so it must not assert an amendment is required), and it notes **§6511** (a refund claim
    for an old year — e.g. 2019 — is likely time-barred: 3 years from filing / 2 from payment, tax r1 M-5).
  - **The VOID direction gets the SAME advisory (tax r1 M-5).** Voiding a promote over a year whose computed tax
    changes reverts the books to `$0` while a filed return still claims the floor — an amend-to-**pay** situation
    (1040-X owing), symmetric to the promote direction; the any-year-tax-diff trigger covers both directions.
- **BG-D10 §6662(e)/(h) risk is disclosed, not hidden.** If an exam determines the correct basis is `$0`
  (Cohan refused per *Vanicek*, no evidentiary predicate), any positive claimed basis is a **gross valuation
  misstatement** → a **40% penalty** under §6662(h) (the >$5k threshold and the penalty base are both measured
  against the **underpayment attributable to the misstatement**, Reg §1.6662-5(b) — NOT the disallowed basis),
  and adequate disclosure does **not** protect against §6662(b)(3) (*Woods*, 571 U.S. 31). The consent screen
  (BG-D6) and the 8275 copy must name the base correctly (tax r1 M-3 — overstating the penalty is a copy defect
  too): *"20% ordinary / 40% worst-case **of the resulting additional tax** (the underpayment attributable to
  the misstatement), plus interest; the 8275 and good-faith methodology mitigate, they do not eliminate."*
- **BG-D11 ★ The estimate reduces reported GAIN on a disposal — it NEVER funds a DEDUCTION or an outbound basis
  carry (tax r1 C-1, the Critical).** BG-D4 clamps the estimate into disposal *gain*; but a promoted tranche's
  `lot.usd_basis` also flows to two NON-disposal filed surfaces the disposal-scoped gates (BG-D6 consent,
  BG-D7 8275, BG-D8 packet gate — all keyed on a promoted *8949 disposal leg*) do NOT cover:
  - **§170(e)(1)(A) short-term charitable donations.** The fold computes the ST-donation deduction as
    `min(FMV, leg.basis)` (`fold.rs`), so a promoted tranche donated to charity **within one year of
    `window_end`** (still short-term) would file a `>$0` estimated-basis deduction on Form 8283 / Schedule A —
    with no consent line, no 8275, and no export gap (a donation is a `Removal`, not a "promoted leg"). This is
    the worst surface for it: if the basis is later disallowed to `$0`, Reg §1.6662-5(g) makes any positive
    claim an automatic **gross** valuation misstatement AND **§6664(c)(3) removes the reasonable-cause defense
    entirely for charitable-deduction property** — so BG-3's "the 8275 mitigates" is *false here*, and a
    knowing-choice guarantee (BG-1) is unmet.
  - **Gift `Removal` carryover.** The floor rides `rehome_onto_removal_leg` (`fold.rs`) into a donee's §1015
    carryover-basis records — a lesser outbound leak.
  - **Ruling (the cleanest close, matching the parent Invariant's spirit): the estimate NEVER funds a
    deduction.** A promoted-tranche leg's §170(e)(1)(A) ST-donation deduction stays limited to its **documented
    component** (i.e. `$0` absent a fee carry, decomposed exactly as BG-D4); the estimate floor does not flow to
    Schedule A / Form 8283, and the gift-removal carryover stays documented-only. The LT-donation path is
    already clean (deduction = FMV, basis uninvolved). Filer-facing copy explains why (an estimate may lower a
    gain you owe, but funding a *deduction* off it is the position §6664(c)(3) punishes with no defense). This
    fixes the now-false `forms.rs` "$0" sentence (§3 item 8) and is pinned by a §6 KAT (a promoted tranche
    donated ST files a `$0`/documented deduction, not the floor).

## 3. The enumerated semantic sweep — TWO censuses (tag-side + payload-side)

BG-D1 keeps the tag, so the `== EstimatedConservative` **tag-side** census is a small *semantic* residue (items
1–8: sites whose *copy or math* assumed `$0`). But there is **no compile-forced `EventPayload` match in product
code** — every consumer has a catch-all — so the new *payload* variant compile-forces almost nothing, and the
**payload-side** census (items 9–15) is a required hand sweep (arch r1 I-6), else a promote ships that void UIs
don't list and void flows render as `"?"` — the exact silent hazard BG-D1 claims to kill (on the tag side). All
sites verified against current source.

**Tag-side (copy/math assumed `$0`):**
1. `conservative.rs::basis_methodology` — its "a `>$0` amount reflects documented fee basis… **never the
   estimate**" sentence becomes FALSE once a promote lands; distinguish promoted legs (via
   `leg.lot_id.origin_event_id` against the promote set — the builder's signature gains the promote set).
2. `conservative.rs` P6 (`overpayment_delta_one` / `overpayment_nudge_lines`) — REWRITE: an unpromoted tranche
   → the existing nudge PLUS a promote-funnel line (`reconcile promote-tranche …`, which must quote the
   **clamped** promote delta or state the promote's saving can be lower on a sale below the window low — tax r1
   I-3); a promoted tranche → a status line ("promoted to $X window-reference floor; further savings only via
   true reconstruction"); the §1014 note unchanged.
3. `conservative.rs::method_inversion_advisory` — promoted-aware (a promoted lot exits `hifo_cmp`'s
   `usd_basis==0` special-case; the "$0-basis unit drawn first" copy/logic must reflect that).
4. `conservative.rs::self_custody_nudge` + `tranche_dip_advisory` — generalize "$0-basis" copy (the dip already
   prints basis-as-filed; copy only).
5. **Stale "$0" copy in the mutual-exclusion refusals + backstop blocker (arch r1 M-1 / tax r1 M-1):**
   `cmd/tranche.rs` `TRANCHE_IS_FINAL_HINT` ("already filed the tranche's **$0** basis"), `cmd/tranche.rs` ("(**$0**
   EstimatedConservative) is on file"), `cmd/tranche.rs` phantom-wallet warning ("it still files at **$0**"),
   `resolve.rs` `SafeHarborUnconservable` blocker detail ("(**$0** EstimatedConservative) remains"), and
   `session.rs` TUI opener ("a … tranche (**$0** EstimatedConservative) is on file") are all factually wrong once
   a promoted tranche exists — the predicates are correct, the copy is not.
6. SPEC amendments: parent D-7 ("nothing `>$0` ever filed") re-scoped to UNPROMOTED tranches; the parent
   Invariant KAT amended per BG-D4; the `event.rs` `DeclareTranche`/`EstimatedConservative` doc comments.
7. Every test pinning "$0-only" re-scoped to unpromoted. TUI/CSV labels UNCHANGED (`estimated_conservative`
   stays true); an optional "promoted" marker is a nit.
8. **`forms.rs` §170(e) "$0" sentence (BG-D11 / tax r1 C-1):** the "an ST-held tranche donation → deduction
   limited to basis = **$0**" copy becomes false unless BG-D11's "estimate never funds a deduction" is
   implemented; the §170(e)(1)(A) `min(FMV, leg.basis)` site (`fold.rs`) is where the fix lands (documented
   component only). **Verified NO-change forms sites (arch r1 N-3), listed so the plan doesn't re-derive them:**
   8949 col (e) reads `leg.basis` (`forms.rs`) and Form 8283 `how_acquired_from` stays `Review` (`forms.rs`).

**Payload-side (no compiler help — enumerate or ship silent):**
9. `void.rs` `is_revocable_payload` (`matches!`) — add `PromoteTranche`, else it is absent from the bulk + TUI
   void candidate lists (degrades BG-D9 revocability to the raw CLI path only). Plus the `voidable_decisions`
   exclusion of a promoted tranche's `DeclareTranche` target (BG-D9-iii).
10. `session.rs` `safe_harbor_residue` — filters OUT `DeclareTranche` but would KEEP a `PromoteTranche`,
    projecting a promote with no target (feeds the BG-D9 dangling shape); handle the variant.
11. `resolve.rs` `build_op` (`_ => Op::Skip`) and the `Some(_)` void classification — both happen to do the
    right thing for a promote but SILENTLY; the promote's real rewrite lands here (BG-D1), so make it explicit.
12. `persistence.rs` fingerprint (`_ => return None`) — correct as-is; add the stock no-fingerprint KAT (N-2).
13. `cli/main.rs` bulk-void summary (`other => {other:?}`) and `tui-edit/main.rs` void-flow summary
    (`_ => ("?", …)`) — a promote renders as Debug/"?" in the exact flows BG-D9 depends on; add real arms.
14. The record-time `would_conflict` (`project/mod.rs`) + pass-1d/1e target-validation arms (BG-D9-i/ii).
15. Serde: `Coverage` (`conservative.rs`) has no `Serialize/Deserialize` derive today (arch r1 N-1); storing it
    on the payload forces the derive — list it in the plan.

## 4. Phases (of ONE sub-project — a single ship gate)

`basis_methodology.txt`/plain-paper text is **NOT** a legitimate standalone MVP: Reg §1.6662-4(f) makes
disclosure adequate only on a properly completed Form 8275, and the parent D-4/D-10 say a memo "has no §6662
effect." So no release exposes `promote` without the official form. With **no installed base** (project fact),
internal phasing costs nothing:
- **Phase 1a** — the engine: `PromoteTranche` schema + **both §3 censuses** (the tag-side copy/math residue AND
  the payload-side hand census — there is no compile-forced match, arch r1 I-6); pass-2 Op-construction fold
  applied INSIDE `resolve` (BG-D1); the computed+stored `Coverage::Full` floor (BG-D3); the fold-time loss clamp
  decomposed from the stored `filed_basis` (BG-D4); the estimate-never-funds-a-deduction close (BG-D11); the
  provenance + informed-consent gates with the clamped/undisposed-aware quantification (BG-D5/D6); the
  `promote-tranche` CLI verb; the P6/P7/advisory rewrite (§3); the 8275 disclosure **content** generation
  (Part I + the REQUIRED, present-by-construction Part II narrative on the event); engine-adjudicated lifecycle
  incl. the any-year-tax-diff advisory + dangling-target impossibility (BG-D9); the export-time completeness
  gate as a REAL refusal (BG-D8).
- **Phase 1b** — the official **Form 8275 fillable PDF** in `btctax-forms` (per-year AcroForm map + geometry
  readback, same machinery as 8949/8283), wired into `export-irs-pdf` + the full-return packet + the DRAFT
  gate's exhaustive destructure; the completeness gate (BG-D8) points at the PDF.

Each phase is TDD + mutation-proven; the whole branch passes the independent two-lens (tax + architecture)
Fable review to 0C/0I before merge.

## 5. Non-goals (this sub-project)

Inherited/§1014 basis (a separate real-acquisition feature); "PartialRecords"/documented-basis floors (use the
existing import); filing a LOSS off an estimate; partial-sat promotion (whole-tranche only — split = void +
re-declare two tranches); the guided **wizard** (sub-project 2); **VARIOUS** multi-date 8949 rows
(sub-project 3); tranche ⇄ safe-harbor coexistence (a v1 non-goal, unchanged); AMT/non-BTC.

## 6. Test / green definition

Every primitive TDD + mutation-proven; full suite + all CI-only jobs green; SPEC + downstream plan reviewed to
0C/0I under BOTH the tax and architecture lenses before merge. Explicit KATs:
- **Promote fold (BG-D1):** a `PromoteTranche` re-homes its target's lot to `filed_basis` with `basis_source`
  STILL `EstimatedConservative`; **the promoted (pre-2025) tranche STILL trips the D-8 backstop** (a
  `SafeHarborAllocation` is denied effectiveness — the by-construction guarantee) and STILL fires both
  record-time refusal directions; a relocated promoted tranche keeps the tag + the floor. **Term-invariance
  (tax r1 M-7):** the promote rewrites ONLY `usd_cost` — `acquired_at`/term/holding-period are byte-identical
  pre/post promote (pins the "no silent LT/ST flip" claim; mutating the date must go red). **Snapshot-timing
  (arch r1 M-3):** the promoted basis is visible to the pass-1 §7.4 effectiveness snapshot (a promote that
  HIFO-reorders the pre-2025 residue does NOT open a checked-vs-folded divergence).
- **The floor is computed+stored (BG-D3):** window-min close as a WHOLE-tranche amount (per-BTC price × sat /
  SATS_PER_BTC — the units KAT, arch r1 M-2); `Coverage::Partial`/`None` HARD-refused; the stored number
  survives a price-data change (fold uses stored, verify flags drift, direction-aware per N-3).
- **The loss clamp (BG-D4):** a floor on a tranche sold BELOW the window low files `$0` gain, NOT a loss;
  the estimate basis never goes negative in the `fee_usd > proceeds` / cent-scale-negative-remainder corners
  (I-1/M-2); **a relocated-with-fee-then-promoted tranche sold below floor keeps its documented fee component
  UNCLAMPED** — the estimate share is decomposed from the stored `filed_basis`, not the merged `lot.usd_basis`
  (arch r1 I-2 / tax r1 I-1); the amended invariant KAT.
- **Estimate-never-funds-a-deduction (BG-D11 / tax r1 C-1):** a promoted tranche donated to charity while
  SHORT-TERM files a `$0`/documented §170(e)(1)(A) deduction (NOT the floor) on Form 8283 / Schedule A; the
  gift-`Removal` carryover stays documented-only; the LT-donation path (deduction = FMV) is unaffected.
- **The gates:** promote refused without the provenance attestation, incl. a mined/earned/airdrop/fork filer
  (BG-D5); the consent quantification is CLAMPED (a below-window-low sale quotes the clamped saving, not an
  unclaimable loss — tax r1 I-3) and NON-ZERO/unrealized-labeled for an undisposed tranche (never a bare $0 —
  tax r1 I-2); the typed acknowledgment (phrase + shown figures + provenance) recorded on the event (BG-D6); an
  empty/scaffold-only Part II narrative is refused at record time (BG-D7); a packet with a promoted leg but no
  8275 artifact is a REAL export REFUSAL, not a silent gap (BG-D8); clean export (no watermark).
- **Lifecycle (BG-D9), engine-adjudicated:** void → reverts to `$0`; second promote → `DecisionConflict`;
  **void-of-tranche-with-live-promote → resolver-inert + `DecisionConflict`** (a RAW/hand-crafted void, not just
  the CLI path, cannot dangle the target — mirrors void-of-effective-allocation); a promote with an
  absent/voided/wrong-type target → hard `DecisionConflict`; `safe_harbor_residue` does not project a dangling
  promote; **the prior-year advisory fires on an UNDISPOSED-tranche promote that HIFO-reorders a prior year's
  tax** (the any-year-`tax_total`-diff trigger, NOT "disposed" — arch r1 I-4 / tax r1 I-4), in BOTH the promote
  and void directions; the copy is conditional ("if Y was already filed") and notes §6511.
- **Payload-side census (arch r1 I-6):** `PromoteTranche` appears in the bulk + TUI void candidate lists
  (`is_revocable_payload`), renders a real label (not `"?"`/Debug) in the bulk-void + void-flow summaries, and
  has the stock no-fingerprint KAT (`persistence.rs`); a promoted tranche's `DeclareTranche` is excluded from
  the bulk-void candidate set.
- **Copy:** the 8275/consent copy states the 20%/40% worst case **against the underpayment/additional tax**
  (not the disallowed basis — tax r1 M-3) and never says "safe harbor" (BG-D7/D10); the clamped-leg 8275
  narrative adds the "limited so as not to report a loss" sentence (tax r1 M-4); provenance-neutral;
  term-correct.
