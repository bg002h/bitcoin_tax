# SPEC — Conservative / Defensive Filing, **Approach B** — sub-project 1: the basis-floor engine + Form 8275

**Status:** DRAFT (author-written; pending the independent two-lens review to 0C/0I per `STANDARD_WORKFLOW.md`).
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
no payment records but a **tight, on-chain-boundable** acquisition window (on-chain receipt timestamps are
permanent evidence). For a "Q4-2017, ~$12k window-min" tranche the $0-vs-floor delta is five figures of real
tax, and the position is *Cohan*-plus-attested-window — rational and litigable. **The honest limit the product
MUST state:** a wide window yields a trivial floor (a 2013–2017 min daily close is ~$65/BTC) — for that filer,
file `$0` and skip the audit surface.

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
  MINTS NO NEW IDENTITY. ★ (the single most important ruling).** The payload (fields resolved across BG-D2..D6):
  `EventPayload::PromoteTranche { target: EventId /* the DeclareTranche decision */, method: FloorMethod,
  filed_basis: Usd /* computed at record time + STORED, BG-D3 */, coverage: Coverage /* snapshot, BG-D3 */,
  provenance_attested: bool /* BG-D5 */, acknowledgment: <recorded typed consent, BG-D6> }` — no `acquired_at`,
  no partial-sat (whole tranche only). A promoted tranche stays
  `BasisSource::EstimatedConservative` — same unprovable origin, same D-8 mutual-exclusion obligations, same
  disclosure obligations, same Form 8283 `Review`, same record-time refusal semantics. **Only its filed number
  changes.** The resolver, when a non-voided `PromoteTranche` targets a `DeclareTranche`, emits that target's
  `Op::Acquire` with `usd_cost = the stored filed basis` (`basis_source` UNCHANGED) — exactly the
  `overpayment_delta_one` what-if seam (`conservative.rs`), minus the discard. **No new `BasisSource` variant.**
  Consequence, verified against source: the D-8 backstop (`resolve.rs` `snap.estimated_conservative_remaining_sat`),
  the Path-A seed exemption (`transition.rs`), the self-transfer relocation carve (`fold.rs`), both record-time
  refusal directions (`cmd/tranche.rs`, `session.rs`), and the tranche⇄Rev-Proc-2024-28 mutual exclusion ALL
  keep holding **by construction, with zero change** — this kills the tag-invisibility hazard class rather than
  enumerating a sweep to patch it. HIFO is correct either way (a promoted lot leaves `hifo_cmp`'s `usd_basis==0`
  special-case and sorts by its real per-sat filed basis — HIFO applied to the as-filed basis, not a bug).
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
  `verify` recomputes and surfaces any drift as an advisory only.) **`Coverage::Full` is REQUIRED** — a
  `Coverage::Partial` covered-part min can EXCEED the true window min (overstate basis → understate gain →
  violate G-4); `Partial`/`None` are a HARD refusal ("narrow the window until it is fully covered, or file
  $0"). Because the number is mechanically reproducible, it is disclosable — which is the whole point. (The
  v1 "warn when `filed_basis > window_reference`" guardrail is now unrepresentable and dropped.)
  - **Substantiating a date narrows the WINDOW, it is never an `acquired_at` override.** Void + re-declare the
    tranche with a tighter window: that moves `filed_basis` (higher window-min) AND `window_end` (term)
    *coherently* from one declared fact. A date that moved term without moving the window is the exact
    incoherence G-4 forbids.
- **BG-D4 The loss clamp (fold-time) — the never-understate structural guarantee.** Per disposal leg, the
  **estimate-claimed basis is `min(floor_share, net_proceeds_share)`** — the estimate-attributable gain is
  clamped `≥ 0`; **unclaimed floor EVAPORATES** (it never shifts to another leg — that would be fabrication).
  The **documented** components stay UNCLAMPED exactly as the amended v1 Invariant KAT enumerates
  (`SPEC.md §3`): §1001(b) `fee_usd` netting, the TP8(c) fee-sat carry, and cent-scale pro-rata rounding can
  still drive a leg negative — those are documented, real amounts. Engineering: the leg builder decomposes
  basis into the estimate component (the lot's `usd_basis` share) vs. documented components (fee carry applied
  after) and clamps only the former; `leg.lot_id.origin_event_id` gives exact leg→tranche identity, so this is
  tractable, not heuristic. Amended invariant: *"a promoted-tranche leg's estimate-attributable gain is `≥ 0`
  by construction; any negative gain remains attributable solely to documented fee/rounding, never the
  estimate."*
- **BG-D5 Record-time provenance attestation (BG-2's load-bearing half).** The promote refuses unless the filer
  records an explicit **purchase-provenance** attestation: *"these units were acquired by purchase within the
  declared window — not by gift, inheritance, or as unreported income."* A gift/inherited/income filer is
  refused and pointed at real-acquisition modeling.
- **BG-D6 Record-time informed-consent acknowledgment — ONE command, two-sided.** `promote-tranche` shows a
  consent screen quantifying BOTH sides using existing machinery: the **saving** = `overpayment_delta_one`
  reused with `reference = the floor`; the **exposure** = that same delta as the at-risk tax, plus interest,
  plus the penalty statement (below), plus the wide-window "this floor is trivial" note when it applies. A
  **typed acknowledgment is recorded ON the event.** No second `attest` verb — record-time consent + an
  export-time artifact gate (BG-D8) is the friction; there is no watermark (clean export — matches the
  standing full-return DRAFT-gate policy: attestation/DRAFT stays pseudo-only).
- **BG-D7 Form 8275 — mandatory, honestly framed, phased.** Mandatory for EVERY promote (the §6662(d)
  understatement is measured RETURN-WIDE, so its ≥10%/$5k threshold is unknowable at promote time; the
  auto-generated 8275 costs nothing and buys threshold-independent optionality + negligence/good-faith
  evidence). Position framing: *"basis estimated at the minimum daily closing price over the attested
  acquisition window (Cohan; the bearing-heavily minimum)."* The **copy NEVER says "safe harbor" or promises
  penalty immunity**; it states the mitigation honestly and the **20% / 40% worst case** (below). The
  disclosure honestly says "minimum daily **closing** price" (intraday lows can be lower — P5's caveat). Part I
  auto-filled (item = Form 8949 col (e); form/line/description/amount). **Part II = auto-scaffold + a REQUIRED
  filer-facts narrative** (Reg §1.6662-4(f) wants facts "in sufficient detail"; a pure template reads as one).
- **BG-D8 Export-time completeness gate.** A form packet (CSV, `export-irs-pdf`, or full-return) that contains
  a promoted leg WITHOUT its 8275 artifact is a HARD gap (mirrors v1's `basis_methodology.txt` presence gate).
  Clean export, no watermark.
- **BG-D9 Lifecycle.** Revocable via `VoidDecisionEvent` (void → the tranche reverts to `$0`, the intact
  `DeclareTranche`). A second live `PromoteTranche` on one target → `DecisionConflict` (not last-wins). Voiding
  a `DeclareTranche` that has a live promote → REFUSED ("void the promote first" — never a dangling target).
  A promote over an already-DISPOSED tranche is ALLOWED (amending a filed `$0` year to claim the floor via Form
  1040-X is a legitimate refund path; the engine has no filed-year concept to refuse on) — with a loud
  **prior-year-delta advisory**: *"this promote changes year Y's computed tax; filing it requires a Form 1040-X
  for Y, with the 8275 attached."*
- **BG-D10 §6662(e)/(h) risk is disclosed, not hidden.** If an exam determines the correct basis is `$0`
  (Cohan refused per *Vanicek*, no evidentiary predicate), any positive claimed basis is a **gross valuation
  misstatement** → a **40% penalty** under §6662(h) (>$5k threshold), and adequate disclosure does **not**
  protect against §6662(b)(3) (*Woods*, 571 U.S. 31). The consent screen (BG-D6) and the 8275 copy state:
  *"20% ordinary / 40% worst-case on the disallowed portion, plus interest; the 8275 and good-faith
  methodology mitigate, they do not eliminate."*

## 3. The enumerated semantic sweep (small, because BG-D1 keeps the tag)

The whole-surface sweep rule applies, but BG-D1 makes it a **semantic** residue only — sites whose *copy or
math* assumed `$0`, each a visible behavior change (no silent tag-invisibility holes). To do, all verified
against current source:
1. `conservative.rs::basis_methodology` — its "a `>$0` amount reflects documented fee basis… **never the
   estimate**" sentence becomes FALSE once a promote lands; distinguish promoted legs (via
   `leg.lot_id.origin_event_id` against the promote set — the builder's signature gains the promote set).
2. `conservative.rs` P6 (`overpayment_delta_one` / `overpayment_nudge_lines`) — REWRITE: an unpromoted tranche
   → the existing nudge PLUS a promote-funnel line (`reconcile promote-tranche …`); a promoted tranche → a
   status line ("promoted to $X window-reference floor; further savings only via true reconstruction"); the
   §1014 note unchanged.
3. `conservative.rs::method_inversion_advisory` — promoted-aware (a promoted lot exits `hifo_cmp`'s
   `usd_basis==0` special-case; the "$0-basis unit drawn first" copy/logic must reflect that).
4. `conservative.rs::self_custody_nudge` + `tranche_dip_advisory` — generalize "$0-basis" copy (the dip already
   prints basis-as-filed; copy only).
5. SPEC amendments: parent D-7 ("nothing `>$0` ever filed") re-scoped to UNPROMOTED tranches; the parent
   Invariant KAT amended per BG-D4; the `event.rs` `DeclareTranche`/`EstimatedConservative` doc comments.
6. Every test pinning "$0-only" re-scoped to unpromoted.
7. TUI/CSV labels UNCHANGED (`estimated_conservative` stays true); an optional "promoted" marker is a nit.

## 4. Phases (of ONE sub-project — a single ship gate)

`basis_methodology.txt`/plain-paper text is **NOT** a legitimate standalone MVP: Reg §1.6662-4(f) makes
disclosure adequate only on a properly completed Form 8275, and the parent D-4/D-10 say a memo "has no §6662
effect." So no release exposes `promote` without the official form. With **no installed base** (project fact),
internal phasing costs nothing:
- **Phase 1a** — the engine: `PromoteTranche` schema + the exhaustive-match sweep; pass-2 Op-construction fold
  (BG-D1); the computed+stored `Coverage::Full` floor (BG-D3); the fold-time loss clamp (BG-D4); the
  provenance + informed-consent gates (BG-D5/D6); the `promote-tranche` CLI verb; the P6/P7/advisory rewrite
  (§3); the 8275 disclosure **content** generation (Part I + Part II scaffold, as structured text); lifecycle
  (BG-D9); the export-time completeness gate keyed on the content artifact.
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
  record-time refusal directions; a relocated promoted tranche keeps the tag + the floor.
- **The floor is computed+stored (BG-D3):** window-min close; `Coverage::Partial`/`None` HARD-refused; the
  stored number survives a price-data change (fold uses stored, verify flags drift).
- **The loss clamp (BG-D4):** a floor on a tranche sold BELOW the window low files `$0` gain, NOT a loss; the
  documented fee corners still reach negative (attribution intact); the amended invariant KAT.
- **The gates:** promote refused without the provenance attestation (BG-D5); consent acknowledgment recorded
  on the event (BG-D6); a packet with a promoted leg but no 8275 artifact is a HARD gap (BG-D8); clean export
  (no watermark).
- **Lifecycle (BG-D9):** void → reverts to `$0`; second promote → `DecisionConflict`; void-tranche-with-live-
  promote → refused; promote-over-disposed → allowed + the 1040-X advisory.
- **Copy:** the 8275/consent copy states the 20%/40% worst case and never says "safe harbor" (BG-D7/D10);
  provenance-neutral; term-correct.
