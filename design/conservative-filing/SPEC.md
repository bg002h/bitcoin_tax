# SPEC — Conservative / Defensive Filing (Approach C: primitives first) — v4

**Status:** ★ GREEN — both lenses 0 Critical / 0 Important. Tax r1 (2C+2I) → r2 GREEN; architecture r1
(1C+6I) → r2 (2I) → r3 (1I) → r4 GREEN (5 cosmetic nits folded). Reviews in `./reviews/`. Next: the
implementation plan.
**Branch:** `feat/conservative-filing` (off `main`). **Design of record:** `./DESIGN.md` (approved).
**Sequencing note:** the shipped **8949-box bug** (FOLLOWUPS ⚠) is fixed AFTER this SPEC greens; D-6 below
depends on that fix landing (it inherits the corrected box logic, does not reimplement it).

Scope = the layered primitives; the guided wizard (Approach B) is a later SPEC.

---

## 1. Purpose & guardrails

Let a poor-records holder file **maximally defensively** with least effort: sell what they can prove, hold
what they can't, report unprovable basis as **$0** (the IRS's own fallback), produce a **filing-ready**
return + a **mandatory methodology disclosure**.

- **G-1 Never omit a taxable event.** Every disposal (incl. private/P2P) is reported. Omission is evasion.
- **G-2 $0 is the only unassailable *basis*.** $0 is the v1 filed basis for unprovable coins. (Basis is the
  only unassailable component — *character/term and proceeds remain assailable*, hence G-4.)
- **G-3 The fairness ↔ attack-surface curve is the filer's to walk (owner framing).** v1 files the safe end
  ($0) and actively quantifies the other end ("reconstruct + import records to save ~$X"); the choice is
  informed and theirs, never made silently.
- **G-4 Never UNDERSTATE tax (tax-review C1).** Character (ST vs LT) and Part/box are **derived** from the
  computed holding period, NEVER assumed long-term. A conservative feature that ever files a short-term gain
  as long-term violates its own "nothing to adjust upward" promise. The engine already derives term per-leg
  (`fold.rs` `term_for` → `is_long_term`); this feature MUST use it.

## 2. Resolved decisions (folds r1 findings; supersedes v1's D-1..D-7)

- **D-1 Tranche = a first-class `EventPayload::DeclareTranche`, NOT a tagged `Acquire`** (arch I-4/I-5/min-9).
  Fields: `{ sat, wallet, window_start, window_end }`. It folds into a lot: `basis_source =
  EstimatedConservative`, `usd_basis = 0`, `acquired_at = window_end`, `wallet = declared`. This homes the
  **window** (P5/P6/P7 need it), the **event identity** (`EventId::Decision { seq }` — `Acquire` has no
  manual `Source`), and the **wallet** (the fold hard-requires it, `fold.rs:568`). v1 declares **$0 basis
  only** (D-7); no floor field is filed.
  - **D-1a Decision-event fold machinery (arch r2 New-1 — SPEC-owned, not a plan detail).** A `DeclareTranche`
    is the FIRST decision payload folded as a primary movement; today the pass-2 timeline builder
    (`resolve.rs:1054`) filters to `EventId::Import` only. P1 MUST extend it to admit `DeclareTranche`, and:
    (a) **Effective date = `window_end`, decoupled from the event timestamp.** The `LedgerEvent.utc_timestamp`
    keeps its documented CREATION-time meaning (no back-dating — preserves the events-list/audit convention);
    the fold builds this event's `Eff` with `date = window_end`, so `pool_key` (`pools.rs:15`) and the pass-1
    conservation snapshot (`transition.rs:44`) bucket it correctly (pre-2025 window ⇒ `PoolKey::Universal`).
    (b) **Canonical-order surrogate:** `sort_canonical` keys on `(utc, src_priority, src_ref)` which a Decision
    id lacks — order `DeclareTranche` Effs by `(window_end, decision_seq)` for determinism (NFR4); two
    same-window tranches tie-break on `decision_seq`. ★ Compare `decision_seq` **numerically** (it rides on
    `Eff.id`) — NOT lexicographically via a `src_ref` string, which would misorder seq 2 vs 10 (arch r3 N-4).
    (c) **No silent skip:** `build_op`'s `_ => Op::Skip` catch-all (`resolve.rs:393`) means an omitted arm
    vanishes the tranche silently — the new arm is REQUIRED and pinned by a KAT ("a DeclareTranche yields an
    Op, never Skip").
    (d) **Void/duplicate (arch r2 New-6):** voidable via the generic `VoidDecisionEvent` (revocable per the
    `Some(_)` catch-all, `resolve.rs:464`); two tranches are legitimately additive (no duplicate-conflict).
    (e) **Σ-conservation (FR9):** the new fold arm MUST bump `stats.sigma_in` like `Op::Acquire`
    (`fold.rs:596`) or the conservation KATs go red (arch r3 N-5).
- **D-2 Holding period date = window END (pin it).** `acquired_at := window_end` (arch/tax min-5 — never a
  "representative"/midpoint date, which would overclaim the hold). The window END is the latest plausible
  acquisition date → conservative for the holding period (never overclaims long-term).
- **D-3 Custody warning (P4) reuses the existing envelope** (`optimize.rs:453` `ForbiddenBroker2027` /
  `is_broker` / `persistability`) — verified TRUE by both lenses. No transfer-statement modeling in v1. A
  ≥2027 specific-ID on an Exchange lot needs a broker-side selection or defaults to FIFO; SelfCustody never
  warns; relief runs through 2026-12-31 (Notices 2025-7/2026-20).
- **D-4 Disclosure (P7) = a REQUIRED free-form methodology statement (not Form 8275 in v1).** For the **$0**
  position, a $0 basis cannot understate gain, so there is no §6662 basis exposure and no 8275 is required;
  the i8949 still asks for a basis explanation when actual cost isn't used, so P7 is **mandatory** (audit
  hygiene + i8949 compliance). **8275 belongs to Approach B's floor path** (D-10).
- **D-5 A tranche is FILING-READY, explicitly NOT pseudo** (arch verified TRUE: `pseudo_active()` counts only
  synthetic pseudo decisions; a real `DeclareTranche` lot has `pseudo=false` and can't trip the export gate).
  MUST export clean (no `[PSEUDO]` banner/attestation). KAT pins it.
- **D-6 Form 8949 mapping — TERM-AWARE + YEAR-AWARE, inherited from the corrected box logic** (tax C1+C2,
  arch I-2/I-3). Part **derived from the leg's term** (LT → Part II, ST → Part I) — never hard-coded. Box =
  the **year-aware digital-asset scheme** the shipped-box fix installs: for **TY2025+**, no-1099-DA →
  **L** (LT) / **I** (ST); broker-1099-DA-without-basis → **K** (LT) / **H** (ST). Pre-2025 tax years keep
  **C/F**. The conservative feature **does not reimplement boxes** — it emits a normal disposal row and
  inherits `forms.rs`'s (corrected) box selection; this SPEC therefore **depends on the shipped-box fix**.
  Date acquired (col b) = the **window-end date** the row already carries — a single-row tranche is i8949-
  compliant without "VARIOUS", which has no typed representation path (arch I-3; VARIOUS-multi-date deferred).
  Basis (col e) = $0. **No** adjustment code (you supply a missing basis, not correct a reported one).
- **D-7 an UNPROMOTED tranche declares & files `$0` ONLY** (arch I-7; re-scoped for Approach B, Task 11).
  A bare `DeclareTranche` carries no floor; nothing `> $0` is written to a filed 8949 by the conservative
  flow **for an unpromoted tranche**. The window-low reference (P5) feeds P6's informational delta.
  ★ Filing a `> $0` floor IS now available via **Approach B** — a `PromoteTranche` (BG-D1/D2/D3) that
  rewrites the tranche's `$0` to a filed window-low floor behind the D-10 consent/8275 gate. A PROMOTED
  tranche therefore DOES file a `> $0` basis; the `$0`-assuming advisories/copy are promote-aware
  (Task 11 tag-side census), and `promote_drift_advisory` (BG-D3) flags a stored floor that later
  recomputes away from current price data.
- **D-8 The tag MUST survive BOTH overwrite sites (arch C1 + r2 New-2).** The `EstimatedConservative` tag is
  hard-overwritten at exactly two sites (assignment-site sweep): the 2025 Path-A seed
  (`basis_source = ReconstructedPerWallet`, `transition.rs:83`) and the self-transfer RELOCATION arm
  (`basis_source = CarriedFromTransfer`, `fold.rs:812`). This feature **exempts `EstimatedConservative` from
  BOTH**:
  - **Path-A seed:** exempt so the tag reaches 2025+ disposal legs (every disposal it serves). Path A keeps
    `usd_basis`/`acquired_at` and routes to `PoolKey::Wallet(lot.wallet)`, so the exemption changes ONLY the
    tag — the per-wallet position is identical.
  - **Relocation (arch r2 New-2):** exempt so a tranche moved Exchange → SelfCustody — **exactly the move P8
    recommends**, and `SelfTransfer` is lot-selectable — keeps its tag; else the disposal leg silently loses
    P3's dip advisory and P7's MANDATORY disclosure (its own test calls a filed-tranche year without the
    disclosure "a hard gap"). Precedent: the pseudo taint already propagates through this same relocation
    struct (`fold.rs:818`). `usd_basis`/`acquired_at` already carry, so tax/term/HIFO are unaffected either
    way — only the tag-keyed advisory/disclosure layer is at stake.
  - **Tranche ⇄ Path-B allocation are MUTUALLY EXCLUSIVE — enforced structurally, not by record-time
    convention (arch r2 New-3 + r3 New-1).** A record-time refusal fires in BOTH directions (declaring a
    tranche when an allocation is present; declaring an allocation when a pre-2025 tranche is present),
    scoped to ANY **in-force (non-voided)** `SafeHarborAllocation` — effective OR **inert**. Scoping to
    "effective" alone is a bug: an inert (unconservable) allocation can be flipped effective by a
    later-declared tranche whose $0-basis sats complete its sat total (basis unchanged), at which point
    Path B silently discards the tranche (`transition.rs:88`) — the ordering r3-New-1 found. The friendly
    refusal HEDGES real-world irrevocability (tax r2 N-3) with a **DIRECTION-SPECIFIC** hint (the implemented
    split — `ALLOCATION_IS_FINAL_HINT` / `TRANCHE_IS_FINAL_HINT`, `cmd/tranche.rs`): the allocation-side
    refusal points at the allocation ("revisit the in-app safe-harbor allocation; if your filed allocation
    is already final, unallocated pre-2025 units are a facts-and-circumstances matter for a professional"),
    the tranche-side at voiding the tranche ("Void the tranche first (`reconcile void <decision-ref>`); if
    you have already filed the tranche's $0 basis, unallocated pre-2025 units are a facts-and-circumstances
    matter for a professional"). Both satisfy the normative hedge; each names the artifact the user is
    blocked BY. (The record-time predicate treats a `SafeHarborAllocation` as in force iff it is NON-voided;
    a VOIDED allocation is left to the engine — T16 review r2 / I-1 — so the record-time layer stays a pure
    event-scan and the engine's void semantics are the single source of truth. The r1 "effective-despite-a-
    void" mirror was removed: it coupled badly with the backstop's blocker retraction below.)
  - **Projection-time invariant backstop (the real guarantee).** Independent of declaration order, a
    `SafeHarborAllocation` is **denied effectiveness** (kept inert → Path A → the tag survives via the seed
    exemption), via a loud `SafeHarborUnconservable`-class blocker, whenever the pre-2025 Universal residue
    contains an `EstimatedConservative` lot **with `remaining_sat > 0`** (a fully-consumed tranche leaves
    nothing to discard — arch r4 Nit-2; the plan extends `UniversalSnapshot`, today totals-only, by one
    field). This makes "a tranche and an EFFECTIVE Path-B allocation can never coexist" a construction — no
    ordering (incl. inert-then-declare) reaches the silent discard. The record-time refusal is the friendly
    early error; this guard is the correctness invariant.
    - **Void retirement (T16 review r2 / I-1).** The backstop fires for EVERY allocation, voided or not; a
      VOIDED-inert allocation would otherwise carry a **permanent** Hard `SafeHarborUnconservable` (the void
      cannot be re-issued), bricking every year on the SUPPORTED flow "void an inert allocation, then declare
      a pre-2025 tranche". So the §7.4 irrevocability pass, when a void APPLIES (the allocation is not
      effective), **retracts** that Hard for the retired allocation — reason-agnostic (tranche-residue,
      re-keyed totals-mismatch, or timebar), so no blind spot. The allocation stays inert (Path A, tag
      survives); a NON-voided allocation keeps its Hard (the deny-effectiveness guarantee, still pinned by
      the Task-5 backstop KATs). A voided-but-would-be-EFFECTIVE allocation (only via a hand-crafted raw
      void — `reconcile void` refuses voiding an effective allocation) is likewise denied effectiveness and
      retired, so it too never seeds Path B: the tag always survives.
  - **Consequence (stated, not accidental — arch r4 Nit-4):** a filer with BOTH documented pre-2025 lots and
    $0 tranches forgoes Rev. Proc. 2024-28 *reallocation flexibility* (Path B), but loses NO basis — Path A
    carries documented lots' basis + `acquired_at` per-wallet unchanged (`transition.rs:80-85`). Coexistence
    (an allocation that itself accounts for tranche sats at $0) is a deliberate v1 **non-goal** (see §4), a B
    refinement. The reverse event-scan refusal is knowingly over-broad in one corner (it also refuses when a
    tranche was fully consumed pre-2025, a vault with no hazard) — acceptable for a "friendly early error"
    (arch r4 Nit-3).
  - **KATs:** tranche-through-Path-A-seed preserves the tag + a 2025+ disposal leg carries
    `EstimatedConservative`; a relocated tranche keeps the tag; the record-time refusal fires for ANY
    in-force allocation (effective OR inert) in BOTH directions; and the **projection-time backstop** — an
    allocation that WOULD conserve over a residue containing an `EstimatedConservative` lot is kept inert
    (Path A, tag survives), pinning the inert-then-declare ordering r3-New-1 found. Scope the plain
    transition KAT to Path-A / no-in-force-allocation vaults.
- **D-9 HIFO-posture mechanism (arch I-6).** Steering is emergent ONLY under HIFO (a $0 lot sorts last,
  `pools.rs:272`); under the FIFO default an old $0 tranche is consumed FIRST (a *gain*-maximizing inversion —
  not necessarily *tax*-maximizing once LT character is weighed, but never an understatement: correct
  application of the in-force method files the correct tax for that method, tax r2 N-2).
  v1 does **not** auto-emit a `MethodElection` (elections are ≥2025 + global/Exchange-scoped — heavy). Instead
  P3 is UPGRADED to fire a **method-inversion advisory** whenever the in-force method would consume a tranche
  lot while a documented lot remains available in the same wallet, and P8/product copy recommends a HIFO
  election. P2 states the method dependence explicitly. (Auto-election is an Approach-B candidate.)
- **D-10 §6662 scoping (tax I-4).** $0 → no §6662(d) basis exposure (nothing to understate). A filed **floor**
  (Approach B) is an estimated `>$0` basis → if disallowed, a §6662(d) substantial-understatement penalty is
  avoided only via substantial authority OR reasonable basis **+ adequate disclosure** = **Form 8275** (a
  free-form memo has no §6662 effect). So B's promote-to-filed-floor path MUST generate/recommend an 8275.

## 3. Primitives (P1–P8)

### P1 — `DeclareTranche` (core; D-1)
- **Schema:** new `EventPayload::DeclareTranche { sat, wallet, window_start, window_end }` + new
  `BasisSource::EstimatedConservative`. Fold arm: emit a lot as in D-1. **Exhaustive-`match` sweep** the new
  `BasisSource` compile-forces (scope in the plan): `forms.rs::how_acquired_from` (this is the **Form 8283**
  donor field, NOT an 8949 column — tax min-6; give it `Review`, and state §170(e): an LT tranche donation →
  FMV deduction, an ST-held tranche donation → deduction limited to basis = **$0**); `render.rs:44` CSV
  label; `tui-edit form.rs:1756` edit-ring + `form.rs:1771` `basis_source_display` — all 4 sites (off-ring,
  precedent `SelfTransferInbound`). Voidable via `VoidDecisionEvent` (D-1d).
- **Input:** a CLI verb (quantity, wallet, window start/end); $0 basis only in v1. Forward-only vault
  compat note (new variant → older binaries can't read; no installed base, harmless).
- **Tests:** DeclareTranche → lot (`$0`, `EstimatedConservative`, `acquired_at=window_end`, declared wallet);
  disposal leg carries the tag (through the 2025 transition — D-8); **term derived** (LT iff window_end >1yr
  before disposal); refuses a pre-2025 declaration under ANY in-force Path-B allocation (effective OR inert —
  D-8); a DeclareTranche yields an `Op`, never `Op::Skip` (D-1a-c); and a **VOIDED** DeclareTranche folds
  nothing — the new timeline admit must honor `voided` (arch r3 N-2).

### P2 — Steered matching (EMERGENT under HIFO — verify + state dependence; D-9)
- No new matching code; HIFO sorts `$0` lots last (`pools.rs:272`, verified). **P2 explicitly states this
  holds only under HIFO**; under FIFO it inverts (D-9's advisory covers that).
- **Tests:** under HIFO, a sale with documented + tranche lots draws the documented lot first; a KAT also
  pins the FIFO inversion so the dependence is not silently assumed.

### P3 — Dip + method-inversion advisory (D-9)
- Advisory (never hard) when a disposal's matched legs include an `EstimatedConservative` lot: names the
  tranche window, its $0 basis, and the resulting gain — **provenance-neutral** (tax min-8c: don't assert
  "purchases" for coins the filer knows were gifted/inherited). PLUS the D-9 method-inversion advisory.
- **Tests:** dip advisory iff a tranche leg is consumed; inversion advisory iff a non-HIFO method consumes a
  tranche while a documented lot remains.

### P4 — Custody-aware compliance warning (D-3; reuse)
- As D-3. **Tests:** fires for a ≥2027 Exchange specific-ID; silent for SelfCustody and ≤2026.

### P5 — Window reference-price engine (informational only in v1)
- `fn window_reference(prices, start, end) -> Option<Usd>` — the min **daily close** over the window from the
  bundled data (5,801 rows, 2010→2026, verified). **NOT a true floor** (tax I-3: intraday lows can be lower);
  it is a *close-based reference*, caveated here and in P6 copy. Partial dataset overlap → min over the covered
  part **with a caveat** (or `None` if no overlap). Never filed in v1 (D-7).
- **Tests:** min-close over range; partial-overlap caveat; out-of-range → None.

### P6 — Overpayment-delta nudge (informational; the G-3 lever)
- Per tranche: tax(`$0`) − tax(window-reference), surfaced as "reconstructing this <window> tranche and
  importing the records could save ~$X — at the cost of a documented basis an examiner can question." For a
  tranche the filer knows is **inherited**, the nudge additionally notes basis is reconstructable **by law**
  from date-of-death FMV with no purchase records (§1014 — the cheapest reconstruction; tax min-8a). Reuses
  the clone-fold-discard what-if seam (`whatif.rs`); nothing `>$0` is filed. **Surface (arch r2 New-4):** the
  nudge appears in the `report --tax-year` output (below the tranche figures) and the TUI Tax tab; year-scope
  = the tranche legs consumed in the report's year, plus a one-line note if undisposed tranche sats remain.
- **Tests:** delta = tax($0) − tax(reference) for a fixed profile; $0 when reference is $0/absent; nudge
  present iff a filed $0 tranche has a non-zero recoverable delta (this year's consumed legs).

### P7 — Methodology disclosure (D-4; REQUIRED)
- Emitted whenever a tranche is in the filed set (not opt-in): enumerates each tranche's window + $0 position
  + the "records unreconstructable → conservative" rationale, **provenance-neutral**, and **term-correct**
  (states LT/ST as computed, never hard-codes "long-term"). **Export (arch r2 New-4):** a text file
  (`basis_methodology.txt`) written alongside the form CSVs in the export dir, and surfaced in the TUI as a
  required artifact whenever a tranche is filed.
- **Tests:** present iff a filed tranche exists; enumerates each tranche; a filed-tranche year without it is a
  hard gap (assert presence); no hard-coded "long-term".

### P8 — Self-custody nudge (advisory)
- Suggests holding oldest/no-records tranches in SelfCustody (own-books specific-ID never expires there);
  recommends a HIFO election (D-9). **Tests:** present for an Exchange tranche; absent for SelfCustody.

### Invariant KAT (tax min-7)
- **No-loss-from-the-estimate (amended, plan-tax r1 I-1 + r2 NEW-1/M-5):** a tranche leg can never file a
  loss *attributable to the $0 estimate*. Absent fees and sub-cent rounding, `gain = proceeds − $0 ≥ 0`.
  The engine can still put a negative gain (or a `>$0` basis) on a tranche row through **documented, real**
  amounts — never the estimate: (a) USD-fee netting when `fee_usd > proceeds` (`fold.rs` `net = proceeds −
  fee_usd`) reduces the amount realized per §1001(b); (b) the shipped TP8(c) fee-sat flow re-homes a
  **documented** fee-sat basis onto the last disposal leg — which is the tranche leg when the tranche leg is
  last (the in-force order exhausts the documented lots ahead of it, or a specific-ID selection names it)
  *while a documented lot remains for the FIFO fee draw* (NB: under a pure-HIFO principal this can't happen —
  HIFO consumes every `>$0` lot before the `$0` tranche, so nothing documented is left to draw the fee from;
  the reachable stagings are FIFO exact-exhaustion or a named-lot sale — plan-tax r2 NEW-1); and (c) sub-cent
  pro-rata remainder rounding — `make_disposal_legs` gives the last leg `net − Σ round_cents(shares)`, which
  can be negative at **cent scale** on a multi-leg dust allocation with no fees at all (bounded by ≤ ½¢ per
  prior leg — plan-tax r3 N-5; Σ-conserving, shared by every multi-leg disposal, vanishes at 8949 whole-dollar
  rounding). All three are correct (§1001(b)/§1011) and none understates tax. So the invariant is scoped:
  *any negative tranche-leg gain is attributable solely to documented `fee_usd`/fee-sat basis or cent-scale
  pro-rata rounding, never to the estimate.* Assert the core (fee-free + single-leg ⇒ `≥ 0`) and characterize
  the corners. (For B's floor path: never claim a loss off
  an estimated basis — a disallowed estimate flips a claimed loss into a gain.)

## 4. Non-goals (v1)
The guided wizard (B); filing a `>$0` floor + its Form 8275 (B, D-10); VARIOUS multi-date rows; the
shipped-box fix (its own project — a **prerequisite** for D-6's compliant output); ProRata auto-split; AMT
compute; non-BTC assets; broker transfer-statement/covered-lot modeling. **Tranche ⇄ Path-B safe-harbor
allocation coexistence** — an allocation that itself accounts for the tranche's sats at $0 (D-8) — is a
deliberate v1 non-goal: v1 makes them mutually exclusive (Path A serves the mixed-records filer with no
basis loss); coexistence is a B refinement.

## 5. Owner decisions — RESOLVED
- **O-1 → D-6 (corrected).** 8949 is term-aware + year-aware (G–L from TY2025), inherited from the box fix,
  window-end date (not VARIOUS), $0 in col (e), no adjustment code, P7 mandatory.
- **O-2 → D-7 + G-3.** v1 files $0; the window reference is an informational nudge (P6), never a v1 filed
  position; floor-filing + 8275 → Approach B.

## 6. Test / green definition
Every primitive TDD + mutation-proven; full suite + CI green; SPEC + downstream artifacts reviewed to 0C/0I
under BOTH the tax and architecture lenses. Explicit KATs: tranche-through-2025-transition (tag preserved);
term-split (ST vs LT derived, never hard-LT); no-loss invariant; method-inversion advisory; clean (non-pseudo)
export; Path-B refusal.
