# SPEC — Conservative / Defensive Filing (Approach C: primitives first) — v2

**Status:** v2 DRAFT — folds the r1 tax + architecture reviews (`./reviews/spec-*-fable-review-r1.md`,
combined 3 Critical + 8 Important). Awaiting re-review to 0C/0I, then a plan.
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
  Fields: `{ sat, wallet, window_start, window_end }`. It **folds like an acquire** into a lot:
  `basis_source = EstimatedConservative`, `usd_basis = 0`, `acquired_at = window_end`, `wallet = declared`.
  This homes the **window** (P5/P6/P7 need it), the **event identity** (a Decision-id payload — `Acquire`
  has no manual `Source`), and the **wallet** (the fold hard-requires it, `fold.rs:568`). v1 declares **$0
  basis only** (D-7); no floor field is filed.
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
- **D-7 v1 declares & files `$0` ONLY** (arch I-7). `DeclareTranche` carries no floor; nothing `> $0` is ever
  written to a filed 8949 by the conservative flow. The window-low reference (P5) feeds ONLY P6's
  informational delta. Filing a `> $0` floor moves to Approach B (with D-10).
- **D-8 Transition exemption — the tag MUST survive the 2025 seed (arch C1).** Pre-2025 tranches fold into
  `PoolKey::Universal`; at the 2025-01-01 seed, Path A currently overwrites `basis_source →
  ReconstructedPerWallet` (`transition.rs:82`). This feature **exempts `EstimatedConservative` from the
  Path-A overwrite** so the tag reaches 2025+ disposal legs (which is *every* disposal it serves). **Path B:**
  declaring a pre-2025 tranche in a vault with an effective `SafeHarborAllocation` changes the Universal
  residue → the conservation guard hard-blocks it (`SafeHarborUnconservable`). v1 **refuses** a pre-2025
  tranche declaration when a Path-B allocation is in force, with a clear message (amend the allocation first);
  it does not silently inert the allocation. KAT: tranche-through-transition preserves the tag + a 2025+
  disposal leg carries `EstimatedConservative`.
- **D-9 HIFO-posture mechanism (arch I-6).** Steering is emergent ONLY under HIFO (a $0 lot sorts last,
  `pools.rs:272`); under the FIFO default an old $0 tranche is consumed FIRST (gain-maximizing inversion).
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
  label; `tui-edit form.rs:1756` edit-ring label (off-ring, precedent `SelfTransferInbound`).
- **Input:** a CLI verb (quantity, wallet, window start/end); $0 basis only in v1. Forward-only vault
  compat note (new variant → older binaries can't read; no installed base, harmless).
- **Tests:** DeclareTranche → lot (`$0`, `EstimatedConservative`, `acquired_at=window_end`, declared wallet);
  disposal leg carries the tag (through the 2025 transition — D-8); **term derived** (LT iff window_end >1yr
  before disposal); refuses a pre-2025 declaration under an effective Path-B allocation.

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
  it is a *close-based reference*, caveated in D-2/P6 copy. Partial dataset overlap → min over the covered
  part **with a caveat** (or `None` if no overlap). Never filed in v1 (D-7).
- **Tests:** min-close over range; partial-overlap caveat; out-of-range → None.

### P6 — Overpayment-delta nudge (informational; the G-3 lever)
- Per tranche: tax(`$0`) − tax(window-reference), surfaced as "reconstructing this <window> tranche and
  importing the records could save ~$X — at the cost of a documented basis an examiner can question." For a
  tranche the filer knows is **inherited**, the nudge additionally notes basis is reconstructable **by law**
  from date-of-death FMV with no purchase records (§1014 — the cheapest reconstruction; tax min-8a). Reuses
  the clone-fold-discard what-if seam (`whatif.rs`); nothing `>$0` is filed.
- **Tests:** delta = tax($0) − tax(reference) for a fixed profile; $0 when reference is $0/absent; nudge
  present iff a filed $0 tranche has a non-zero recoverable delta. Specify year-scoping (filed legs this year
  + a future-consumption note).

### P7 — Methodology disclosure (D-4; REQUIRED)
- Emitted whenever a tranche is in the filed set (not opt-in): enumerates each tranche's window + $0 position
  + the "records unreconstructable → conservative" rationale, **provenance-neutral**, and **term-correct**
  (states LT/ST as computed, never hard-codes "long-term"). First-class export artifact.
- **Tests:** present iff a filed tranche exists; enumerates each tranche; a filed-tranche year without it is a
  hard gap (assert presence); no hard-coded "long-term".

### P8 — Self-custody nudge (advisory)
- Suggests holding oldest/no-records tranches in SelfCustody (own-books specific-ID never expires there);
  recommends a HIFO election (D-9). **Tests:** present for an Exchange tranche; absent for SelfCustody.

### Invariant KAT (tax min-7)
- **No-loss:** a $0-basis tranche disposal can never produce a loss (gain = proceeds − $0 ≥ 0) → no
  §1211/§1212/§1091 interaction from a v1 tranche. Assert it. (For B's floor path: never claim a loss off an
  estimated basis — a disallowed estimate flips a claimed loss into a gain.)

## 4. Non-goals (v1)
The guided wizard (B); filing a `>$0` floor + its Form 8275 (B, D-10); VARIOUS multi-date rows; the
shipped-box fix (its own project — a **prerequisite** for D-6's compliant output); ProRata auto-split; AMT
compute; non-BTC assets; broker transfer-statement/covered-lot modeling.

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
