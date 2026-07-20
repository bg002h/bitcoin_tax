# SPEC — Conservative / Defensive Filing (Approach C: primitives first)

**Status:** DRAFT — awaiting the §2 independent review loop (author ≠ reviewer, to 0C/0I) then a plan.
**Branch:** `feat/conservative-filing` (off `main` @ `1f01184`). **Design of record:** `./DESIGN.md` (approved).
**Date:** 2026-07-20.

This SPEC resolves DESIGN §8's open questions and specifies the P1–P8 primitives. It is deliberately scoped
to the layered primitives; the guided wizard (Approach B) is a later SPEC.

---

## 1. Purpose (one line)

Let a poor-records holder file **maximally defensively** with least effort: sell what they can prove, hold
what they can't, report unprovable basis **conservatively** (default $0 = the IRS's own fallback = nothing to
adjust upward), and produce a **filing-ready** return + a **methodology disclosure**.

Guardrails (from DESIGN §2, restated as MUSTs):
- **G-1 Never omit a taxable event.** Every disposal (incl. private/P2P) is reported. The feature never
  helps hide income. Conservative *over*-reporting is the defense; omission is evasion.
- **G-2 $0 is the only unassailable basis.** $0 is the default filed basis for unprovable coins; any basis
  `> $0` is an explicit, warned opt-in (it asserts an acquisition window an examiner can question).

## 2. Resolved decisions (DESIGN §8)

- **D-1 Tranche representation → a tagged `Acquire`, not a new event.** A no-records tranche is an
  `Acquire { sat, usd_cost = <floor or 0>, basis_source: BasisSource::EstimatedConservative, utc_timestamp =
  <era-representative date> }`. Reusing `Acquire` means the tranche becomes a normal lot in the pool, so P2
  (steering) is **free** (HIFO already sorts `$0`/basis-pending lots last, `pools.rs:272`), and the whole
  disposal/matching machinery applies unchanged. The new `BasisSource` variant is the sole schema addition
  and drives P3 (dip warning), P7 (disclosure), and the 8949 "how acquired" column.
- **D-2 Floor = window-low; holding-period date = window-end.** The filer asserts an acquisition **window**
  `[start, end]`. If they choose a documented floor, `usd_cost` = the **lowest daily close in the window**
  (from the bundled BTC-USD data, P5) — the most conservative non-zero basis. `acquired_at` (holding period)
  = the window **end** (latest plausible date that is still > 1 yr before the earliest possible sale — long
  term for old coins, never overclaims a longer hold). `$0` is the default (no window-low needed; window
  still sets the holding-period date).
- **D-3 Covered/noncovered (P4) — reuse the existing custody envelope, don't build transfer statements.**
  v1 fires the 2027+ custody warning using the ALREADY-MODELED envelope (`optimize.rs:453` /
  `ForbiddenBroker2027`): a specific-ID on an **Exchange**-wallet lot for a **≥2027** disposal. It does NOT
  model broker-to-broker transfer statements or covered/noncovered lot provenance in v1 — a documented
  simplification (a self-custody→broker re-deposit is treated by its wallet at disposal, which is correct for
  the warning's purpose). SelfCustody never warns.
- **D-4 Disclosure (P7) = a generated free-form methodology STATEMENT, not Form 8275.** A conservative
  $0/floor basis is the *fallback*, not a position *contrary* to a reg, so no §6662(d)/Form 8275 disclosure
  is required. v1 emits an exportable text statement ("basis for pre-20XX holdings was not reconstructable;
  treated conservatively as $0 [or the <window> low], reported long-term") the filer keeps/attaches. Advisory
  + written to the export dir on request.
- **D-5 Conservative tranche is FILING-READY, explicitly NOT pseudo.** A tranche is a REAL declared lot with
  a REAL (conservative) basis. It MUST NOT set `pseudo_active()`, MUST export CLEAN (no `[PSEUDO]` banner, no
  attestation gate), and is distinct from pseudo-reconcile (synthetic-estimate, resolve-before-filing). A KAT
  pins that a tranche year exports without the pseudo banner.

## 3. Primitives (P1–P8)

### P1 — No-records vintage tranche (core)
- **Schema:** add `BasisSource::EstimatedConservative` (`btctax-core/src/event.rs`). Tranche = `Acquire` with
  that source (D-1).
- **Input:** a CLI verb + (later) TUI flow to declare a tranche: quantity (sat), acquisition window
  `[start,end]`, and basis position (`$0` default | `window-low` floor). Emits the tagged `Acquire`.
- **Projection:** the tranche folds to a lot like any acquire; `basis_source` propagates to the disposal leg
  and the 8949 row (`forms.rs` "how acquired" → a conservative/"various" treatment — verify the 8949 box).
- **Tests:** tranche → lot in pool; basis + acquired_at as declared; long-term on disposal; 8949 character.

### P2 — Steered matching (EMERGENT — verify only)
- No new code: HIFO (the posture default) sorts `EstimatedConservative` `$0`-basis lots LAST (`pools.rs:272`).
- **Tests:** a sale with documented + tranche lots draws the documented lot first; the tranche is untouched
  until documented lots are exhausted. (Pins the emergent property against a future method-order change.)

### P3 — Dip-into-no-records warning (advisory)
- When a disposal's matched legs include an `EstimatedConservative` lot, emit an advisory blocker (NOT hard)
  naming the tranche, its basis ($0/floor), and the resulting gain ("this sale reaches your <window> tranche
  — basis $X, gain $Y").
- **Tests:** advisory fires iff a tranche lot is consumed; not on a pure documented-lot sale.

### P4 — Custody-aware compliance warning (scoped; refine existing)
- Reuse `ForbiddenBroker2027` (D-3). Ensure the select-lots / optimize surfaces phrase it as: a ≥2027
  specific-ID on an Exchange lot needs a broker-side selection or it defaults to FIFO. Never for SelfCustody.
- **Tests:** warning fires for a 2027 Exchange specific-ID; silent for SelfCustody and for ≤2026.

### P5 — Documented price-floor engine
- `fn window_low(prices, start, end) -> Option<Usd>` over the bundled daily BTC-USD closes; `None` if the
  window is outside the dataset (→ fall back to $0 + surface "no price data for that window").
- **Tests:** window-low is the min close in range; boundary/out-of-range cases.

### P6 — Overpayment-delta (informational)
- For a tranche, compute the tax difference between the $0 position and the window-low floor position
  ("reconstructing this tranche could save ~$X"). Reuses the report/optimize engine (no persisted change).
- **Tests:** delta = tax($0) − tax(floor) for a fixed profile; zero when window-low is $0/absent.

### P7 — Methodology disclosure output (D-4)
- Generate the text statement from the tranches present (windows, positions taken). Exportable.
- **Tests:** statement enumerates each tranche's window + position; present iff a tranche exists.

### P8 — Self-custody nudge (advisory)
- Advisory suggesting the oldest/no-records tranches be held in SelfCustody (own-books specific-ID never
  expires there). Surfaced where custody is visible.
- **Tests:** nudge present when a tranche sits in an Exchange wallet; absent for SelfCustody.

## 4. Non-goals (v1)
The guided wizard (B); ProRata auto cross-wallet split; AMT computation; non-BTC assets; broker
transfer-statement/covered-lot provenance modeling (D-3); Form 8275 generation (D-4).

## 5. Owner decisions still needed (flag before/at review)
- **O-1 The 8949 character/box** for an `EstimatedConservative` tranche disposal (e.g., "various" acquired +
  which box) — a tax-form-correctness detail to confirm against the 8949 instructions.
- **O-2 Whether the documented-floor opt-in is in v1 at all**, or v1 ships **$0-only** (simplest, purest
  defense) with the floor engine (P5/P6) as informational-only and the *filed* floor deferred to B. (Leaning
  v1 = $0 filed + floor shown as what-if; promote-to-filed in B.)

## 6. Test / green definition
Per STANDARD_WORKFLOW: every primitive TDD + mutation-proven; full suite + CI green; the SPEC and each
downstream artifact reviewed to 0C/0I. Tax-correctness assertions (holding period, $0-fallback, LT character,
no-pseudo export) get explicit KATs.
