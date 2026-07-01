# SPEC — minimal qualified-appraisal trigger (Slug 2, Phase-2)

**Source baseline:** `origin/main` @ `eae88df` (post pre-2025 reconciliation slug).
**Goal:** Emit an **advisory** on a charitable donation whose estimated **claimed deduction** exceeds
**$5,000**, signalling that a §170(f)(11)(C) **qualified appraisal is likely required** — computed by a
term-aware proxy that never misses the common case, WITHOUT the full §170(e) deduction computation
(deferred to the Phase-2 forms program). Decision-support only; changes no tax figure.

**SemVer:** additive `BlockerKind` variant (Advisory) + a statutory constant + emission logic ⇒ **MINOR**
(pre-1.0). Backward-compatible (new advisory only; no existing behavior changes).

## Legal grounding
- **§170(f)(11)(C):** a qualified appraisal is required when the **claimed deduction** for a property
  contribution exceeds **$5,000** (statutory, NOT inflation-indexed).
- **§170(e)(1)(A):** FMV is reduced by the gain that would not be long-term capital gain if sold — so the
  claimed deduction is **FMV for long-term capital-gain property** and **basis for short-term OR
  ordinary-income property** (ordinary-income = property by CHARACTER: crypto held as inventory/for sale
  in a trade or business (§1221(a)(1)), self-created property, or ≤1-yr lots — NOT investment-held mined
  BTC held >1yr, which is a capital asset deducted at FMV).
- **Crypto-specific (CCA 202302012):** the "readily-valued / exchange-price" exception to the appraisal
  requirement does NOT apply to cryptocurrency — a donation of crypto with a claimed deduction > $5,000
  requires a qualified appraisal. (Strengthens why this advisory matters for BTC donations.)
- **§170(f)(11)(F)** (aggregation of similar items across a year) — **deferred** (this slug flags
  per-donation-event; cross-donation aggregation is a Phase-2 refinement).

## The rule (chosen 2026-06-30)
For each `Donate` event, compute a **term-aware claimed-deduction proxy**:
```
deduction_proxy = Σ over the donation's removal legs of ( leg.term == LongTerm ? leg.fmv_at_transfer : leg.basis )
```
Emit an Advisory `QualifiedAppraisalNote` iff `deduction_proxy > QUALIFIED_APPRAISAL_THRESHOLD ($5,000)`.

Why this proxy (not "FMV>$5k AND basis>$5k", which under-flags the textbook LT-appreciated donation):
- LT legs contribute their **FMV** (the LT deduction) → the classic donate-appreciated-BTC case
  (FMV $60k, basis $5k, held >1yr) is flagged (proxy=$60k>$5k). The AND rule missed it.
- ST legs contribute their **basis** (the §170(e)-reduced deduction) → a ST high-FMV/low-basis donation
  is NOT over-flagged when its actual deduction (basis) ≤ $5k.
- **Deferred imprecision (safe / over-flag direction) — [R0-I1: the §170(e) reduction turns on asset
  CHARACTER, not holding period].** **Ordinary-income property** — crypto held as **inventory / for sale
  in a trade or business (§1221(a)(1))**, self-created property, or other property whose sale would yield
  ordinary income — is deducted at **basis** under §170(e)(1)(A) even when held >1yr, but this proxy
  treats all LT legs as FMV → it may OVER-flag such lots. **Do NOT describe this as "long-term-held
  property":** investment-held mined BTC held >1yr is a CAPITAL asset → LT capital-gain property →
  correctly deducted at FMV and correctly flagged (NOT an over-flag). Over-flagging an advisory is safe
  (it says "verify"); precisely detecting ordinary-income (character-based) property requires the §170(e)
  deduction computation (Phase-2 forms program). The advisory detail MUST disclose this over-flag caveat,
  framed as asset character (dealer/inventory), never as holding period.

## Current-state (recon @ eae88df — hook points)
- `Donate` folds to a `Removal { kind: Donation }` (`fold.rs` ~1004-1075) via `consume_principal` +
  `make_removal_legs` (`fold.rs:201-238`); each `RemovalLeg` carries `basis`, `fmv_at_transfer`, `term`
  (`state.rs:134-141`). Total FMV = `*fmv`; per-leg term/basis/fmv are all present after
  `make_removal_legs`.
- The user-supplied `appraisal_required: bool` (`state.rs:149`, set from the CLI `--appraisal` flag) is
  currently **recorded but consumed by nothing** — it is the taxpayer's manual assertion. This slug's
  computed advisory is an **independent cross-check**; the two are intentionally decoupled (the advisory
  fires on the computed proxy regardless of the manual bool — which is exactly when a manual `false` +
  proxy>$5k is most useful to surface).
- Advisory `BlockerKind`s (`Pre2025MethodNote`, etc.) are emitted via `st.add_blocker(kind, Some(ev),
  detail)` and already rendered in `verify` under "Advisory blockers" (`render.rs:982-989`) — **no render
  change needed**.
- Statutory constants live in `crates/btctax-core/src/tax/tables.rs` (`NIIT_RATE`, `loss_limit`, …) with
  a statute cite + "not inflation-indexed" + "never in a TaxTable" convention.

## Plan (TDD)

### Task 1 — statutory constant + `QualifiedAppraisalNote` advisory kind
- **Files:** `crates/btctax-core/src/tax/tables.rs`; `crates/btctax-core/src/state.rs`.
- Add `pub const QUALIFIED_APPRAISAL_THRESHOLD: Usd = dec!(5000);` to `tables.rs` with a §170(f)(11)(C)
  cite + "statutory, NOT inflation-indexed" note + the existing "never belongs in a TaxTable" convention
  comment. Add `BlockerKind::QualifiedAppraisalNote` to `state.rs` and place it in the **Advisory** arm of
  `severity()` (must NEVER be Hard — it cannot gate `compute_tax_year`).
- KATs: `QUALIFIED_APPRAISAL_THRESHOLD == dec!(5000)`; `QualifiedAppraisalNote.severity() == Advisory`.

### Task 2 — emission in the `Donate` fold arm + KATs
- **Files:** `crates/btctax-core/src/project/fold.rs` (Donate arm).
- Compute the proxy from the FINAL persisted legs — **[R0-M1] AFTER both `make_removal_legs` AND
  `carry.rehome_onto_removal_leg` have run** (`rehome_onto_removal_leg`, `fold.rs:274-276`, mutates the
  LAST leg's `basis` AFTER `make_removal_legs`), i.e. immediately before `st.removals.push(...)`, so the
  proxy matches the legs actually stored (a re-homed ST fee-cent basis is then included). Compute
  `deduction_proxy = Σ (leg.term == Term::LongTerm ? leg.fmv_at_transfer : leg.basis)` over the final
  legs; if `> QUALIFIED_APPRAISAL_THRESHOLD`, `st.add_blocker(BlockerKind::QualifiedAppraisalNote,
  Some(<donate event id>), detail)`. Emit **per qualifying Donate event** (NOT once-per-projection — each
  donation is a separate §170 item; do not use the `note_pre2025_once` single-fire guard). Detail text:
  the estimated deduction proxy + the $5k threshold + §170(f)(11)(C) + the crypto-specific CCA 202302012
  point + the **character-framed** over-flag caveat ("this estimate treats all long-term legs at FMV;
  crypto held as inventory/for sale in a trade or business (§1221(a)(1)) or other ordinary-income
  property is deducted at basis under §170(e) REGARDLESS of holding period — the precise determination is
  deferred; verify") + a **§170(f)(11)(F) aggregation caveat** ("this flags a single donation; the $5,000
  test also aggregates similar donated items across the tax year — cross-donation aggregation is not
  considered here"). Do NOT read/modify the user's `appraisal_required` bool. Do NOT change any
  basis/gain/removal math.
- KATs (fixtures build a pre/post donation with known term/fmv/basis): (a) **LT $60k FMV / $5k basis →
  flagged** (proxy $60k; the case the AND rule missed); (b) **ST $10k FMV / $2k basis → NOT flagged**
  (proxy = basis $2k ≤ $5k); (c) **mixed legs** (one LT + one ST) whose term-weighted sum crosses $5k →
  flagged, and a mixed case summing ≤ $5k → not flagged; (d) **boundary:** exactly $5,000.00 → NOT
  flagged AND $5,000.01 → flagged (strictly `>`); (e) advisory is **Advisory** + a year whose only
  blocker is this note still yields `TaxOutcome::Computed(..)` (never gates); (f) **two qualifying
  donations → two `QualifiedAppraisalNote` blockers** (per-event, not single-fire); (g) a non-donation
  removal (GiftOut) never emits it; (h) **[R0-M2] decoupling from the manual bool:** proxy>$5k with
  `appraisal_required=false` STILL emits the advisory, and proxy≤$5k with `appraisal_required=true` does
  NOT emit it (locks the independent-cross-check decision).

### Task 3 — `verify` surfacing KAT + whole-diff review (Phase E gate)
- **Files:** `crates/btctax-cli/tests/verify_report.rs` (KAT); `render.rs` ONLY if a gap is found.
- KAT: a vault with a >$5k-proxy donation shows the `QualifiedAppraisalNote` under `verify`'s Advisory
  blockers with the deduction/threshold/§170 detail; a small donation shows none. Expect NO render change
  (advisories already render).
- Whole-diff: the advisory never gates `compute_tax_year`; no tax figure / removal-math change; the
  proxy is deterministic + exact (Decimal, no float); the over-flag caveat is disclosed (character-framed,
  not holding-period); per-event emission; backward-compat (pure addition). Record the deferred items in
  FOLLOWUPS: precise §170(e) ordinary-income (character-based) deduction (upgrades this proxy) +
  §170(f)(11)(F) cross-donation aggregation — both land in the Phase-2 forms & §170(e) program. **[R0-Nit]
  Also reconcile the FOLLOWUPS "Standing roadmap" line** that still describes this slug as the superseded
  `FMV>$5k AND basis>$5k` rule → the term-aware deduction proxy.

## Out of scope
- Precise §170(e) claimed-deduction computation (ordinary-income-property detection) — Phase-2 forms
  program; this proxy over-flags LT-held ordinary-income property in the safe direction until then.
- §170(f)(11)(F) aggregation of similar items across a tax year — per-donation-event flagging only here.
- Coupling to / validating the user's manual `appraisal_required` bool; Form 8283 generation; any change
  to donation basis/gain/removal math; 2026/2027 tax tables.
