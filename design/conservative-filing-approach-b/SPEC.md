# SPEC — Conservative / Defensive Filing, **Approach B** — sub-project 1: the basis-floor engine + Form 8275

**Status:** DRAFT — **r1–r4 two-lens reviews FOLDED** (r1: tax 1C/4I + arch 0C/6I; r2: tax 0C/3I + arch 0C/1I;
r3: tax 0C/2I + arch 0C/1I; r4: **arch GREEN 0C/0I** + tax 0C/1I — all persisted verbatim in
`reviews/spec-{tax,architecture}-fable-review-r{1,2,3,4}.md`); pending the **r5** re-review to 0C/0I per
`STANDARD_WORKFLOW.md`. The architecture lens has CONVERGED (r4 green). r4's tax Important was the terminal
ring of the whole-surface class — cross-YEAR propagation: a flagged year Y's change flows through the two
carryover chains the product models (§1212(b) capital-loss, §170(d) charitable) into Y+1's derived carryover-in
lines, which are unflagged (Y+1's legs unchanged). Folded as a NAMING clause (loud-uncomputable pattern, no new
machinery) in BG-D9/D6 + §3 census + §6 KAT; plus the gift-only quoting fix both lenses raised (§1015
carryover-Δ, not a bare $0). Four rounds of surface censuses (tax lens) now find no further emitter or
propagation path.
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
window yields a trivial floor (a 2013–2017 min daily close is ~$13/BTC — BTC's Jan-2013 close; the min daily close over all of
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
  - **★ Estimate basis EVAPORATES when it is consumed as a FEE, never re-homing onto another lot/leg (tax r2
    I-2).** `consume_fee` draws fee-sats acquisition-date **FIFO** (`consume_fifo`, `pools.rs` — independent of
    the elected method), so a promoted tranche (typically the OLDEST lot — the feature's audience) is drawn
    first, and today its per-sat floor basis re-homes undecomposed via `FeeCarry` onto the surviving lot
    (`rehome_onto_lot`), the last disposal leg, or the last removal leg — either landing on a non-promote-keyed
    recipient or hiding in BG-D4's `documented = usd_basis_share − estimate_share` residue, escaping the clamp
    AND the deduction ban (BG-D11). Worked corner: promote 1 BTC to a $12k floor, self-transfer paying a
    10,000-sat fee drawn from the tranche → the relocated lot's `usd_basis` picks up $1.20 of *floor* fee basis;
    a later below-window-low sale files a **$1.20 loss that is 100% estimate money**, falsifying the amended
    invariant's attribution claim. **Ruling:** decompose the consumed fee-sats the same way — the estimate
    component (fragment `origin_event_id` ∈ promote set; per-sat floor × fee-sats) **evaporates** (basis
    forfeiture is always conservative); only the documented fee basis re-homes via `FeeCarry`. This is the same
    single-site decomposition as the leg builder, applied at `consume_fee`/`FeeCarry`.
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
    = `Σ` of per-year clamped deltas over **every year the pre/post fold pair differs in filed content,
    INCLUDING the current year** (equivalently: the BG-D9 diff run WITHOUT its `< current` advisory filter — the
    advisory keeps `< current` only because already-filed years need the 1040-X copy, but the realized-saving Σ
    MUST include the current year, else the dominant term of the most common flow — sell earlier this year, then
    promote before filing — is silently dropped; tax r2 M-2 / tax r3 I-1). The per-year term ranges over BOTH
    surfaces the promote rewrites (tax r3 I-2): a disposal-flagged year quotes the clamped gain/tax-Δ; a
    **removal-flagged year** (a HIFO reorder changed a donation/gift draw) quotes the profile-free
    **deduction-Δ** for donation legs (`Σ claimed_deduction`) or the **§1015 carryover-basis-Δ** for gift legs
    (`Σ leg.basis` — gifts carry `claimed_deduction: None`; both directions per BG-D9's donation/gift
    distinction, tax r4 / arch r4 M-1), never a bare $0. Each flagged year also carries the **carryover-cascade
    note** (BG-D9's §1212(b)/§170(d) clause): the exposure of later filed years whose carryover-in lines derive
    from a flagged year is recorded in the `Acknowledgment` as a **named-unquantified** term when the machinery
    cannot price it (never silently absent; tax r4 I-1). PLUS, for sats not yet disposed, an
    explicit **unrealized** line — *"saving and exposure accrue at disposal; at today's price the floor would
    reduce reported gain by ~$X (hypothetical, not a filed figure)"* — **never a bare $0**; and when there is no
    bundled close for "today" (data ends at release; tax r3 N-2), fall back to the latest bundled close + its
    date, or state *"no current price data — the floor itself, $`filed_basis`, is the maximum gain reduction"*,
    never a silent $0 or a dropped line.
  - **★ Uncomputable years must be surfaced as uncomputable, NEVER a silent $0 term (tax r2 I-3 = arch r2 I-1,
    the CONVERGED r2 blocker).** Every `tax($0) − tax(floor)` term routes through `compute_tax_year`, which
    returns `None` (→ `overpayment_delta_one` yields `Usd::ZERO`) for any year with **no bundled tax table**
    (`BundledTaxTables::load()` ships ONLY 2017/2024/2025/2026 — so **2018–2023 are uncomputable forever**, and
    those *are* the feature's Mt. Gox/LocalBitcoins-era audience years), **no stored `TaxProfile`** (forms
    export needs none — a permitted flow), or ANY unrelated Hard blocker anywhere in the projection. A silent
    `$0` here re-enters the exact "bare $0" defect through the uncomputable door and poisons the recorded
    `Acknowledgment`. So the consent quantification is defined on the before/after **fold pair** the machinery
    already produces: quote the tax-Δ **only when both folds compute that year**; otherwise show the
    **gain-Δ (disposal-flagged) / deduction-Δ (removal-flagged) / filed-content delta** (all profile- and
    table-independent — and the deduction-Δ *must* use the fold-pair figure even when the year computes, because
    engine B's `compute_tax_year` excludes crypto donations by design, tax r3 I-2) with an explicit *"tax not
    computable for year Y (no bundled table / no tax profile / blocked) — the reported gain/deduction still
    changes by ~$G/~$D"* clause. The `Acknowledgment` snapshot records each term as computed-tax-Δ **or**
    gain/deduction-Δ-with-uncomputable-flag, so the §6664(c) artifact stays honest; a genuinely-all-uncomputable
    promote never records a bare $0.
  - Plus interest, the penalty statement (BG-D10), and the wide-window "this floor is trivial" note when it
    applies. A **typed acknowledgment** — the consent phrase + a snapshot of the exact figures shown (each
    flagged computed-tax vs gain-only) + the attested provenance — **is recorded ON the event** (BG-D1's
    `Acknowledgment` struct), so the recorded good-faith artifact cannot later be shown to have quoted wrong (or
    silently zero) numbers. No second `attest` verb — record-time consent + an export-time artifact gate
    (BG-D8) is the friction; there is no watermark (clean export — matches the standing full-return DRAFT-gate
    policy: attestation/DRAFT stays pseudo-only). The non-interactive path (scripted/non-TTY) uses an explicit
    `--i-acknowledge <phrase>` flag with the computed figures still printed to stdout (mirroring the shipped
    `require_attestation`/`ATTEST_PHRASE` precedent — chosen over a bare refusal; arch r1 M-4 / r2 N-1).
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
    via `would_conflict` (`project/mod.rs`); (ii) a **non-voided** `PromoteTranche` whose target is
    absent/wrong-type is a hard `DecisionConflict` (the pass-1d/1e validation pattern), never silently inert —
    scoped to non-voided promotes so the both-voids end state (promote dead + tranche voided) does NOT emit a
    spurious permanent Hard (arch r3 N-1); (iii) `PromoteTranche`
    is added to the `voidable_decisions`/`is_revocable_payload` set AND `DeclareTranche` is EXCLUDED from the
    bulk-void candidate set while it carries a live promote (`void.rs` — today `DeclareTranche` is
    unconditionally a candidate), so the bulk sweep cannot create the dangling shape either.
    - **★ "Has a live promote" is adjudicated against the FINAL non-voided-promote set — deferred, not inline
      (arch r2 M-1).** A promote-void applies unconditionally; a tranche-void DEFERS and is adjudicated after,
      mirroring `allocation_voids`' step-3 deferred adjudication. Otherwise an inline pass-1a classification
      against the incrementally-built `voided` set is order-dependent: a hand-crafted vault holding
      void-of-tranche (seq N) then void-of-promote (seq N+1) would classify the tranche-void first (promote not
      yet voided → inert Hard `DecisionConflict`), then kill the promote — leaving the tranche permanently
      un-voidable with no clearing move (the double-void refusal, `cmd/reconcile.rs`, blocks re-issuing). The
      two-stage evaluation is acyclic (promote-liveness depends only on promote-targeted voids), so it is
      order-independent. Product paths can only produce the void-promote-first order (`would_conflict` refuses
      recording a tranche-void while a promote is live), so this is hand-crafted-vault hardening — the P9/T15
      guard class; pinned by the both-voids-either-order KAT.
  - **Promote over an already-DISPOSED tranche is ALLOWED** (amending a filed `$0` year to claim the floor via
    Form 1040-X is a legitimate refund path; the engine has no filed-year concept to refuse on).
  - **★ The prior-year advisory triggers on a FOLD DIFF (leg/8949 change), not a computed-tax diff, and not
    "disposed" (arch r1 I-4 + tax r1 I-4, re-keyed for arch r2 I-1 / tax r2 I-3).** Gating on "promote over an
    already-disposed tranche" (evaluated pre-promote) MISSES the default-HIFO retroactivity: promoting an
    **undisposed** tranche whose per-sat floor exceeds a documented lot's per-sat basis re-orders a PRIOR year's
    HIFO draw (`pools.rs` — the promoted lot exits the sort-last `usd_basis==0` special-case and now outranks
    cheaper documented lots), silently rewriting that year's legs/8949 rows and creating a real later-year
    understatement (documented basis double-counted across filed years). But keying on a **computed-`tax_total`**
    diff (the r1 fix) is ALSO wrong: `tax_total` is `None` — and `None == None` reads as "no change" — for every
    year with no bundled table (**only 2017/2024/2025/2026 ship**, so 2018–2023 never compute), no `TaxProfile`,
    or any unrelated Hard blocker, i.e. exactly the feature's old-filed-year audience; the advisory would be
    structurally unable to fire while the legs rewrite. So the advisory fires for **any year `< current` whose
    per-year FILED-CONTENT set differs between the pre- and post-promote fold** — profile/table/blocker-
    independent, the fold pair the machinery already produces. The operative predicate is the **leg-SET diff, NOT
    a Σ-gain diff** (tax r3 N-1: a reorder swapping equal-basis different-date lots changes the filed 8949 rows
    and legs with Σ-gain unchanged — Σ-gain is a usually-visible consequence, not an equivalence). Crucially the
    filed-content set is **disposal legs (8949) AND removal legs (8283 / Schedule-A donation + §1015 gift
    carryover)** — because donations/gifts draw through the SAME method-elected `consume_principal` as disposals
    (`fold.rs` `Op::GiftOut`/`Op::Donate`), so the same HIFO reorder silently rewrites a prior DONATION-only
    year's deduction with ZERO disposal-leg / 8949 change, and no other gate catches it (a removal recognizes no
    gain, and engine B excludes crypto donations by design — tax r3 I-2 = arch r3 I-1, the CONVERGED r3 blocker).
    This also resolves the partially-disposed ambiguity ("disposed" = has any disposed leg). Copy: *"this promote
    changes year Y's reported gain by ~$G **and its charitable deduction by ~$D** [and computed tax by ~$Δ, when Y
    computes]; if Y was already filed, claiming it requires a Form 1040-X for Y with the 8275 attached"* — the
    tax-Δ clause appears only when Y computes, otherwise a *"tax not computable for Y (no table/profile/blocked)"*
    note; conditional on "if Y was already filed" (the engine has no filed-year concept, so it must not assert an
    amendment is required); and it notes **§6511** (a refund claim for an old year — e.g. 2019 — is likely
    time-barred: 3 years from filing / 2 from payment, tax r1 M-5). The tax-Δ figure must **NOT** be implied to
    capture the deduction effect — engine B can't price crypto donations.
    - **The removal-flagged term distinguishes DONATION legs from GIFT legs (tax r4 / arch r4 M-1).** A
      DONATION-flagged year quotes `Σ claimed_deduction` from the fold pair (profile-free, `Some(..)` on the
      `Removal`) as ~$D, with the 1040-X clause. A **GIFT**-flagged year has `claimed_deduction: None` (a gift is
      no deduction on the donor's 1040), so quoting `Σ claimed_deduction` would print a bare `$0` that the "never
      $0" rule forbids AND falsely imply a donor 1040-X; instead quote the **§1015 carryover-basis-Δ** (`Σ
      leg.basis` over the year's gift removal legs — equally profile-free) and say *"the recorded donee-basis
      (§1015 carryover) documentation for year Y changes; the donor's Form 1040 is unaffected (note the Form 709
      basis column where one was filed)"* — no 1040-X assertion. When BOTH Δs are `$0` for a flagged year (e.g. a
      reorder swapping equal-basis same-term lots), name the changed filed content (8283 acquisition dates /
      donee-basis records) rather than a bare `$0`.
    - **★ Cross-YEAR carryover cascade — name it, per the loud-uncomputable pattern (tax r4 I-1).** Year Y is not
      the whole amendment set: a change to Y's net capital gain/loss or its charitable deduction propagates
      through TWO return-level carryover chains the product itself models — **§1212(b)/§1211(b) capital-loss**
      (engine B's per-year `carryforward_out` feeds Y+1's `capital_loss_carryforward_in`,
      `carryforward_consistency`) and **§170(d) charitable** (`write_back_carryover`/`apply_carryover_writeback`
      stamps Y's computed `charitable_carryover_out` into Y+1's stored `ReturnInputs`, silently overwriting a
      Computed-provenance value) — into LATER filed years whose crypto legs are byte-identical between the folds
      (so they never flag, are absent from the consent Σ, and engine B's per-year tax-Δ is blind because it
      applies the same `capital_loss_carryforward_in` in both folds). Worked corner: a promote-reorder that
      absorbs a prior year's $6k loss into Y strands the filed $3k carryforward deduction on Y+1 (amend-to-PAY,
      silent). Quantifying the cascade is profile/AGI-gated, so — exactly like the uncomputable-year rule — the
      spec floor is **naming, not computing**: the advisory adds *"carryover-linked lines of later filed years
      (Schedule D capital-loss carryforward, §1212(b); Schedule A charitable carryover, §170(d)) derive from
      year Y and may also require amendment, even though those years' crypto transactions are unchanged"*, quoting
      the Δ only where the machinery computes it (the `carryforward_out` diff when both folds compute Y; the
      `charitable_carryover_out` diff when Y's absolute return computes) else named-unquantified. Both directions
      (the VOID direction is amend-to-refund, §6511-bounded).
  - **The VOID direction gets the SAME advisory (tax r1 M-5).** Voiding a promote over a year whose fold diff
    changes reverts the books to `$0` while a filed return still claims the floor — an amend-to-**pay** situation
    (1040-X owing), symmetric to the promote direction; the fold-diff trigger covers both directions.
- **BG-D10 §6662(e)/(h) risk is disclosed, not hidden.** If an exam determines the correct basis is `$0`
  (Cohan refused per *Vanicek*, no evidentiary predicate), any positive claimed basis is a **gross valuation
  misstatement** → a **40% penalty** under §6662(h) (the >$5k threshold and the penalty base are both measured
  against the **underpayment attributable to the misstatement**, Reg §1.6662-5(b) — NOT the disallowed basis),
  and adequate disclosure does **not** protect against §6662(b)(3) (*Woods*, 571 U.S. 31). The consent screen
  (BG-D6) and the 8275 copy must name the base correctly (tax r1 M-3 — overstating the penalty is a copy defect
  too): *"20% ordinary / 40% worst-case **of the resulting additional tax** (the underpayment attributable to
  the misstatement), plus interest; the 8275 and good-faith methodology mitigate, they do not eliminate."*
- **BG-D11 ★ The estimate reduces reported GAIN on a disposal — it NEVER funds a DEDUCTION or an outbound basis
  carry, enforced at ONE site (tax r1 C-1, the Critical; extended by tax r2 I-1/M-1).** BG-D4 clamps the
  estimate into disposal *gain*; but a promoted tranche's basis also flows to NON-disposal filed surfaces the
  disposal-scoped gates (BG-D6 consent, BG-D7 8275, BG-D8 packet gate — all keyed on a promoted *8949 disposal
  leg*) do NOT cover:
  - **§170(e)(1)(A) short-term charitable donations — via TWO independent emitters (tax r2 I-1).** The **fold**
    computes the ST-donation deduction as `min(FMV, leg.basis)` (`fold.rs` `Op::Donate` arm, `claimed_deduction`);
    **AND, independently, the full-return engine re-derives it** — `crypto_charitable_gifts`
    (`tax/return_1040.rs`) does `short_basis += leg.fmv_at_transfer.min(leg.basis)` per removal leg → `apply_170b`
    (`tax/charitable.rs`) → Schedule A line 12 → taxable income → computed tax, and it NEVER reads
    `claimed_deduction`. So a promoted tranche donated to charity **within one year of `window_end`** (still
    short-term) files a `>$0` estimated-basis deduction on Form 8283 / Schedule A — no consent, no 8275, no
    export gap (a donation is a `Removal`, not a "promoted leg"). Worst surface: if the basis is later disallowed
    to `$0`, Reg §1.6662-5(g) makes any positive claim an automatic **gross** valuation misstatement AND
    **§6664(c)(2) removes the reasonable-cause defense for charitable-deduction-property valuation misstatements**
    (with §6664(c)(3)'s qualified-appraisal special rule restoring it only for the *substantial* — not the
    deemed-gross — case) — so BG-3's "the 8275 mitigates" is *false here*, and a knowing-choice guarantee (BG-1)
    is unmet. **A fold-site-only fix ships the harm through the second emitter** (and breaks
    `crypto_charitable_gifts`' own documented "these reconcile" invariant).
  - **The Form 8283 "Donor's cost or adjusted basis" column + the §1015 gift carryover (tax r2 M-1 / carry-cite
    fix).** `Form8283Row.cost_basis = leg.basis` (`forms.rs`) prints the floor onto the printed packet
    (`tax/printed.rs`), the official filled AcroForm (`btctax-forms/src/form8283.rs`), and `removals.csv`
    (`render.rs`) — for BOTH ST and LT donations (an LT donation has no promoted 8949 leg, so BG-D8 never gates
    it, yet still prints an unsubstantiated estimate as "cost"). And the **principal** §1015 carryover basis on a
    gift is `make_removal_legs` (`fold.rs`, `basis: c.gain_basis`) — NOT `rehome_onto_removal_leg`, which carries
    only fee cents; a fix aimed at the latter alone misses the principal carry.
  - **★ Ruling — decompose at the REMOVAL-LEG BUILDER, one site, all consumers inherit by construction (tax r2
    I-1).** A removal leg drawn from a promoted lot carries its **documented component only** (estimate share
    decomposed exactly as BG-D4 from the stored `filed_basis`; the estimate share **evaporates**; the pool-side
    debit still conserves Σbasis — the §1015 NoGainNoLoss reported≠consumed precedent, `fold.rs`). Then the
    fold's `claimed_deduction`, `crypto_charitable_gifts`/Schedule A, the Form 8283 `cost_basis` column, the
    printed PDF, `removals.csv`, and the gift §1015 carryover ALL file the documented component — the estimate
    never funds a deduction or an outbound carry, on any surface, by construction. The LT-donation *deduction*
    path is already clean (deduction = FMV, basis uninvolved); this ruling additionally keeps the LT 8283 basis
    *column* honest. Filer-facing copy explains why (an estimate may lower a gain you owe, but funding a
    *deduction* off it is the position §6664(c)(2) punishes with no defense). This fixes the now-false `forms.rs`
    "$0" sentence (§3 item 8), adds `crypto_charitable_gifts` + `make_removal_legs` + the Form 8283 column chain
    to §3's census, and is pinned by a §6 KAT that asserts BOTH emitters (the fold's `claimed_deduction` / Form
    8283 **and** the full-return Schedule A line 12 / computed tax).

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
8. **The §170(e) / removal-leg surface (BG-D11 / tax r1 C-1 + tax r2 I-1/M-1) — the fix lands at ONE builder,
   these are the consumers that inherit it:** the decomposition rule lives in the **removal-leg builder**
   (`make_removal_legs`, `fold.rs`, `basis: c.gain_basis` — the principal §1015 carry) so a promoted-lot removal
   leg carries the documented component only. Downstream consumers that then file correctly by construction (all
   must be covered by the §6 KAT / verified, NOT independently patched): the fold's `claimed_deduction`
   (`Op::Donate` arm); **`crypto_charitable_gifts` (`tax/return_1040.rs`) → `apply_170b` (`tax/charitable.rs`) →
   Schedule A line 12 — the SECOND, independent §170(e) emitter**; the Form 8283 `cost_basis` column
   (`forms.rs` `Form8283Row.cost_basis` → `tax/printed.rs` → `btctax-forms/src/form8283.rs` → `removals.csv` in
   `render.rs`, ST **and** LT); and the now-false `forms.rs` "$0" doc sentence. **Verified NO-change forms sites
   (arch r1 N-3), listed so the plan doesn't re-derive them:** 8949 col (e) reads `leg.basis` (`forms.rs`) and
   Form 8283 `how_acquired_from` stays `Review` (`forms.rs`). **★ Prior-year-diff wiring (tax r3 I-2):** because
   removals draw by the elected method (HIFO default) exactly like disposals, the BG-D9 advisory / BG-D6 consent
   fold-diff must range over `state.removals` per prior year (`crypto_charitable_gifts` recomputes a prior year's
   Schedule A from the rewritten legs), NOT just `state.disposals` — both leg types derive `PartialEq`/`Eq`, so
   it is the same set comparison.
8b. **The FIFO fee-draw back-channel (BG-D4 fee-evaporation / tax r2 I-2):** `consume_fee` / `consume_fifo`
   (`pools.rs`, acquisition-date FIFO) + the three `FeeCarry` re-home sites (`rehome_onto_lot`,
   `rehome_onto_disposal_leg`, `rehome_onto_removal_leg`, `fold.rs`) must decompose the consumed fee-sats so the
   estimate component evaporates and only documented fee basis re-homes.
8c. **The cross-year carryover chains (BG-D9 cascade / tax r4 I-1) — promote-adjacent, at minimum enumerated so
   the silent-overwrite path is considered:** `carryforward_consistency` (`tax/compute.rs`, wired `cmd/tax.rs`)
   for §1212(b) capital-loss (its "verify your prior return" copy is promote-blind), and
   `write_back_carryover` / `apply_carryover_writeback` (`cmd/tax.rs` / `tax/return_1040.rs`) for §170(d)
   charitable — the latter STAMPS a flagged year's computed `charitable_carryover_out` into Y+1's stored
   `ReturnInputs`, silently overwriting a Computed-provenance value. The plan decides whether their copy becomes
   promote-aware; the census must list them so the cascade isn't rediscovered at implementation time.

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
    (NOTE the false lead, arch r2 N-2: `cli/main.rs` has a SECOND textually-identical `other => {other:?}` in
    `bulk_resolve_payload_summary` that correctly needs NO promote arm — it renders imported *conflict* payloads
    only, a Decision-id promote is unreachable there.)
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
  incl. the fold-diff (profile/table-independent) prior-year advisory + dangling-target impossibility with
  deferred void-adjudication (BG-D9); the export-time completeness gate as a REAL refusal (BG-D8).
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
- **Estimate-never-funds-a-deduction (BG-D11 / tax r1 C-1 + tax r2 I-1/M-1):** a promoted tranche donated to
  charity while SHORT-TERM files a `$0`/documented §170(e)(1)(A) deduction (NOT the floor) on **BOTH emitters** —
  the fold's `claimed_deduction` / Form 8283 **and** the full-return `crypto_charitable_gifts` → Schedule A
  line 12 / computed tax (the KAT asserts the computed 1040, not just the fold); the Form 8283 `cost_basis`
  column prints the documented component for ST **and** LT donations; the gift §1015 carryover
  (`make_removal_legs`) stays documented-only; the LT-donation *deduction* path (deduction = FMV) is unaffected.
- **Fee-draw evaporation (BG-D4 / tax r2 I-2):** a promoted tranche whose fee-sats are consumed FIFO (a
  self-transfer paying an on-chain fee drawn from the tranche) then sold below the window low files a `$0`
  estimate loss — the estimate component of the burned fee-sats evaporated, did not re-home onto the surviving
  lot/leg; only documented fee basis re-homed.
- **The gates:** promote refused without the provenance attestation, incl. a mined/earned/airdrop/fork filer
  (BG-D5); the consent quantification is CLAMPED (a below-window-low sale quotes the clamped saving, not an
  unclaimable loss — tax r1 I-3), NON-ZERO/unrealized-labeled for an undisposed tranche (never a bare $0 — tax
  r1 I-2), and for an **uncomputable year** (no bundled table / no profile / blocked) shows the gain-Δ /
  deduction-Δ with an explicit "tax not computable" flag rather than a silent $0 (tax r2 I-3 / arch r2 I-1);
  **the Σ INCLUDES the current year's realized delta** (a tranche disposed earlier this year, then promoted
  before filing — the dominant term of the most common flow — is quoted, not dropped; tax r3 I-1); the typed
  acknowledgment (phrase + shown figures, each flagged computed-tax vs gain/deduction-only + provenance)
  recorded on the event (BG-D6); an empty/scaffold-only Part II narrative is refused at record time (BG-D7); a
  packet with a promoted leg but no 8275 artifact is a REAL export REFUSAL, not a silent gap (BG-D8); clean
  export (no watermark).
- **Lifecycle (BG-D9), engine-adjudicated:** void → reverts to `$0`; second promote → `DecisionConflict`;
  **void-of-tranche-with-live-promote → resolver-inert + `DecisionConflict`** (a RAW/hand-crafted void, not just
  the CLI path, cannot dangle the target — mirrors void-of-effective-allocation); **both voids in EITHER order**
  (void-tranche then void-promote, and the reverse) converge to promote-dead + tranche-voided, never a bricked
  ledger (arch r2 M-1; and BG-D9-ii is scoped to non-voided promotes so the both-voids end state emits no
  spurious Hard — arch r3 N-1); a **non-voided promote** with an absent/wrong-type target → hard
  `DecisionConflict` (arch r4 N-1 word-order); `safe_harbor_residue` does not project a dangling promote; **the
  prior-year advisory fires on an UNDISPOSED-tranche promote that HIFO-reorders a prior year — INCLUDING a
  table-less/profile-less year AND a prior DONATION/GIFT-only year with NO disposal-leg change** (the fold-diff
  over disposal legs **AND removal legs**, quoting the deduction-Δ for a donation-reordered year; NOT
  computed-`tax_total`, NOT a Σ-gain diff, NOT "disposed" — arch r1 I-4 + tax r1 I-4, re-keyed arch r2 I-1 / tax
  r2 I-3, widened arch r3 I-1 / tax r3 I-2), in BOTH the promote and void directions; the copy is conditional
  ("if Y was already filed") and notes §6511.
- **Removal-flag quoting + cross-year cascade (BG-D9/D6 / tax r4 I-1 + M-1):** a **GIFT-only** prior-year
  reorder (no disposal leg, `claimed_deduction: None`) STILL fires the advisory, quoting the §1015
  carryover-basis-Δ (`Σ leg.basis`) — NOT a bare `$0` — with the "donee-basis documentation changes; the donor's
  1040 is unaffected" copy and NO 1040-X assertion; a both-Δs-zero flagged year names the changed 8283
  dates/donee records instead of `$0`. **The carryover cascade is NAMED:** a loss-stealing reorder (a promote
  that absorbs a prior year's capital loss into year Y) flags Y AND the advisory names the §1212(b) carryforward
  cascade into the later filed year whose crypto legs are unchanged; the §170(d) `write_back_carryover` direction
  is likewise named — both quoted where the machinery computes, else named-unquantified (loud, never silent).
- **Payload-side census (arch r1 I-6):** `PromoteTranche` appears in the bulk + TUI void candidate lists
  (`is_revocable_payload`), renders a real label (not `"?"`/Debug) in the bulk-void + void-flow summaries, and
  has the stock no-fingerprint KAT (`persistence.rs`); a promoted tranche's `DeclareTranche` is excluded from
  the bulk-void candidate set.
- **Copy:** the 8275/consent copy states the 20%/40% worst case **against the underpayment/additional tax**
  (not the disallowed basis — tax r1 M-3) and never says "safe harbor" (BG-D7/D10); the clamped-leg 8275
  narrative adds the "limited so as not to report a loss" sentence (tax r1 M-4); provenance-neutral;
  term-correct.
