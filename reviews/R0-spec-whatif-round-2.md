# R0 — SPEC review, what-if / synthesize tax-planning tool (task #43), round 2 (delta)

- **Artifact under review:** `design/SPEC_synthesize_whatif.md` @ `df6be61` (branch `feat/whatif`; main == `283238f`).
- **Prior round:** `reviews/R0-spec-whatif-round-1.md` (0C/2I/5M/5N; algorithm rendered faithfully — no Fable re-consult).
- **Scope of this round:** verify ONLY that the round-1 folds (I1, I2, M1–M5) are captured correctly and
  introduced no new contradiction. The optimizer-algorithm faithfulness was settled in round 1 (§A there) and is
  NOT re-litigated. Read-only; no implementation.
- **Grounding:** every fold re-checked against current source at review time (file:line).

## VERDICT: **0 Critical / 0 Important / 2 Minor / 2 Nit** — **R0-GREEN.** Cleared to plan/implement, **P0 first.**

Both Importants and all five Minors are folded correctly and coherently. No residual raw-`niit_applies` *use* and no
residual hard-coded "$3,000" *report value* survive anywhere in the spec. The two Minor / two Nit items below are
packaging-note imprecisions (SemVer wording, module-visibility wording) — none blocks implementation; the P0→P3
plan is executable with 0 open blocking questions. Fold at author's discretion (or defer to Ship-time packaging).

---

## A. Fold verification (each round-1 finding, against current source)

### I1 — §1212 sell disclosure now DELTA-based ✅ CAPTURED, COHERENT
- SPEC:44–47 states the disclosure is *"DELTA-BASED, never a hard-coded '$3,000'"*; this-year ordinary offset =
  `withhyp.loss_deduction − baseline.loss_deduction`, *"$0 when the baseline already consumes the §1211(b) cap."*
- **Field confirmed:** `TaxResult.loss_deduction` exists at **types.rs:104**, documented *"§1211(b) capital-loss
  ordinary offset actually used this year WITH crypto (level; ≥ 0)"* and set from `with.loss_deduction`
  (**compute.rs:399**) — a WITH-scenario **level**, exactly as I1 requires (so the marginal offset must be the
  baseline subtraction). The `$0-when-capped` claim is sound: `loss_deduction = min(net_loss, loss_limit)`
  (net_1222, compute.rs) is a whole-year cap, so a baseline already at the cap leaves delta = 0.
- **KAT confirmed present:** `whatif_sell_offset_delta_is_zero_when_baseline_caps` (SPEC:108–109) — baseline consumes
  the cap ⇒ offset delta = $0, ALL carried, *"NOT '$3,000'."* The simple case
  (`whatif_sell_loss_reports_carryforward_delta`) and a `carryforward_in_consumed_first` variant are retained
  (SPEC:107–110). Coherent.

### I2 — `niit_incremental = withhyp.niit − baseline.niit` replaces the raw flag ✅ CAPTURED, COHERENT
- SPEC:49–52 (sell) + SPEC:73, 76–78 (harvest report + NIIT-kink disclosure) all use
  `niit_incremental = withhyp.niit − baseline.niit`, explicitly *"NOT the raw `MarginalRates.niit_applies`."*
- **Field confirmed:** `TaxResult.niit` is the **delta** at **types.rs:102** (*"Crypto-attributable §1411 NIIT
  (DELTA: with − without)"*), computed `niit_with − niit_without` (**compute.rs:398**). The baseline-subtraction is
  the same cancellation as `total` (the no-crypto term is profile-only, compute.rs:378) — exact. ✅
- **Raw-flag semantics confirmed (why the fold is needed):** `MarginalRates.niit_applies = niit_with > niit_without`
  **within one `compute_tax_year`** (compute.rs:390); its own doc (types.rs:76–83) says it is crypto-vs-no-crypto and
  *"Display-only — this field feeds no tax figure or delta."* So reading it on a with-real-disposals year would
  misfire exactly as I2 warned.
- **KAT confirmed present:** `whatif_niit_incremental_not_raw_flag` (SPEC:112–114) — NIIT-reducing loss harvest on a
  year with real disposals ⇒ `niit_incremental < 0`, raw flag NOT used.
- **Residual-raw scan:** the only three `niit_applies` mentions left (SPEC:50, 51, 113) are the negative override
  ("NOT the raw flag"), the report field DEFINED as `niit_incremental != 0` (a delta, not the raw
  `MarginalRates` read), and the KAT asserting the raw flag is *not* used. **No live raw-flag read remains.** ✅

### M1 — proceeds scale with N ✅ CAPTURED
- SPEC:30–32: *"proceeds scale with the candidate N (unlike `ConsultRequest`'s fixed total): `--price` is per-BTC →
  each candidate's `proceeds = round_cents(price × N / 1e8)`; `price: None` (dataset date) → `fmv_of`."*
- Confirmed against seam: `ConsultRequest.proceeds: Option<Usd>` is a **total** (optimize.rs:111);
  `round_cents` exists (conventions.rs:22). The off-by-1e8 trap is now closed in the core-module body. ✅

### M2 — whatif in btctax-core for seam access ✅ CAPTURED (wording nit → N1)
- SPEC:28–30: *"`whatif` lives IN btctax-core … crate-internal access to the today-private
  synthetic_state/fold_as_of/pool_key/method_order/hifo_cmp (call directly, or lift to `pub(crate)` — no public API
  widening)."*
- Confirmed private: `fold_as_of` (optimize.rs:1214), `synthetic_state` (:1230), `score_synthetic` (:1274),
  `candidate_selections` (:327); `method_order`/`hifo_cmp` (pools.rs:249/274). Location-in-crate + the
  `pub(crate)`-lift escape hatch is the correct resolution. (Wording caveat → N1.)

### M3 — `#[non_exhaustive]` + PrefSplit re-export → "MINOR" ⚠️ CALL RIGHT, LABEL IMPRECISE (→ m1, m2)
- SPEC:129–133 recommends `#[non_exhaustive]` on `TaxResult` (+ on `PrefSplit`) and a crate-level re-export.
- Confirmed premises: `TaxResult` is `#[derive(Debug, Clone, PartialEq, Eq)]` with all-public fields and **no**
  `#[non_exhaustive]` (types.rs:90); `PrefSplit` likewise (compute.rs:42). **Version now 0.3.0** (btctax-core &
  btctax-cli Cargo.toml:3) — so the fold's *"bump 0.3.0→0.4.0"* is **correct against current source** (round 1's
  "0.2→0.3" was from the stale 0.2.0 memory note; the fold updated the base correctly — good).
- The *engineering call* (add `#[non_exhaustive]`, re-export) is right. Two imprecisions → **m1 / m2** below.

### M4 — `--magi` defaults to `--income`, never $0 ✅ CAPTURED, COHERENT
- SPEC:92–94: *"`--magi` defaults to `--income` (a floor — NEVER $0 …) with a printed caveat 'MAGI assumed =
  ordinary income; NIIT may be understated if you have other MAGI'."*
- Confirmed root: `placeholder_tax_profile` sets `magi_excluding_crypto: Usd::ZERO` (cmd/tax.rs:20); the NIIT closure
  fires only when `magi > thr` (compute.rs:363). The never-$0 floor + disclosed understatement matches the round-1
  fix. ✅

### M5 — Qss→Mfj KAT ✅ CAPTURED
- SPEC:123: `[R0-M5] Qss→Mfj status mapping inherited (the breakpoint/threshold lookup)`. Confirmed:
  `FilingStatus::Qss` uses the MFJ schedule/thresholds for all §1(h)/§1/§1411 lookups (types.rs:4–15). ✅

---

## B. Self-consistency + new-gap scan

- **Residual "$3,000" hard-code:** none as a report value. Remaining "$3k" mentions are legitimate: the negative
  framing (SPEC:44 *"never a hard-coded '$3,000'"*), a KAT expected-value for the specific baseline-no-loss case
  (SPEC:121), and the algorithmic $3k-**pin plateau** in the non-monotone gotcha (SPEC:147). (SPEC:121 wording → N2.)
- **Residual raw-`niit_applies` read:** none (see I2 above).
- **Sell vs harvest naming:** sell carries both `niit_incremental` (SPEC:49) and a derived `niit_applies` bool
  (`!= 0`, SPEC:51); harvest carries `niit_incremental` (SPEC:73). Both surface the delta — coherent, not
  contradictory.
- **No new contradiction** introduced by the folds.
- **Plan implementable, 0 open blocking questions:** P0 (engine delta: `pref_split`/`bottom_with` + regression
  KAT) is self-contained and unblocked; P1 (sell + `synthetic_year` + ad-hoc profile + consult fix + §1212/NIIT/
  non-persistence KATs) has every seam identified; P2 (segment-walk harvest, all 4 predicates, prefix semantics,
  status enum, disclosures, trap battery) inherits the round-1-faithful algorithm with delta reporting now
  consistent; P3 (TUI) is a deferred spec-slice. The m1/m2 packaging items are Ship-time, not P0 blockers.

---

## FINDINGS (all sub-Important)

### [Minor m1] M3's "→ then MINOR" is inaccurate for the introduction release — this cycle is breaking regardless
**Where:** SPEC:129–133. The fold reads: *"OR (preferred) add `#[non_exhaustive]` … NOW so this + all future field
additions are non-breaking → **then MINOR**."* Adding `#[non_exhaustive]` to an **already-exposed** struct is itself
a **breaking** (major-category) change per the Cargo SemVer reference — so option (b) does **not** make *this*
release MINOR; it makes only *future* field additions free. Round-1 M3 stated the caveat correctly (*"add
`#[non_exhaustive]` to TaxResult now (**itself breaking → same bump**)"*); the fold dropped it. **Impact:** low
(pre-1.0; the correct answer — bump to **0.4.0**, and the leading bold *"adding pub fields … is a BREAKING change"*
— is already in the same sentence), but taken literally the "→ MINOR" could invite a compatible-version publish of a
breaking change. **Fix:** restore the caveat — *"this cycle is 0.4.0 regardless; `#[non_exhaustive]` buys
future-proofing (future additions become MINOR), not a MINOR label for this release."*

### [Minor m2] `PrefSplit` is already public API at `btctax_core::tax::PrefSplit` — "newly-exposed" mischaracterizes it
**Where:** SPEC:131 (*"the newly-exposed `PrefSplit`, which must also be `pub`-re-exported from btctax-core"*).
`PrefSplit` is **already** `pub use`-re-exported at the tax-module level (**tax/mod.rs:11**) and reachable as
`btctax_core::tax::PrefSplit` (`pub mod tax`, lib.rs:13). It is merely **absent from the crate-ROOT re-export list**
(lib.rs:36–42, which carries `TaxResult`). **Two consequences the fold should reflect:** (a) the action is a
crate-**root** re-export for parity with `TaxResult` (not a first exposure); (b) because `PrefSplit` is *already*
public, adding `#[non_exhaustive]` to it is *also* breaking — which **reinforces** m1 (this is unavoidably a
breaking cycle), not a counterexample. **Fix:** reword to *"re-export `PrefSplit` at the crate root (already public
via `tax::`) and add `#[non_exhaustive]` (itself breaking, same bump)."*

### [Nit N1] M2 "call directly" is loose — a sibling `whatif` module cannot call `optimize`/`pools` private fns
SPEC:29 offers *"call directly, or lift to `pub(crate)`."* Rust item privacy is per-**module**, not per-crate: a new
`btctax-core::whatif` sibling cannot "call directly" `synthetic_state`/`fold_as_of` (private to `optimize`) or
`method_order`/`hifo_cmp` (private to `project::pools`) without a `pub(crate)` lift (or being a submodule). The fold
already names `pub(crate)` as the mechanism, so this is wording only. **Fix:** drop "call directly, or"; keep "lift
the reused seams to `pub(crate)` (no public widening)."

### [Nit N2] SPEC:121 "`harvest_all_loss_notbinding` + $3k disclosure" shorthand sits in mild tension with I1
The value is correct **for that scenario** (all-loss pool, baseline with no other loss ⇒ offset delta = the full
$3,000 cap), so it is not a hard-code bug. But given I1's headline is *"never hard-code $3,000,"* the KAT label reads
cleaner as *"offset-**delta** disclosure ($3,000 here because the baseline has no other loss)"* to keep the
delta-based framing uniform across the KAT list.

---

## C. Gate

**R0-GREEN.** 0 Critical / 0 Important. Both round-1 Importants (I1 §1212 delta, I2 NIIT incremental) and all five
Minors (M1–M5) are folded correctly and verified against current source; no residual raw-flag use, no residual
hard-coded $3,000 report value, no new contradiction. The 2 Minor / 2 Nit items are Ship-time packaging-wording
polish and do not gate implementation. **Cleared to plan/implement — P0 (engine delta: `pref_split`/`bottom_with`
on `TaxResult` + the byte-identical-regression KAT) first.**
