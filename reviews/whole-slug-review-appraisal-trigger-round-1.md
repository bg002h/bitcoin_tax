# Whole-slug review — minimal qualified-appraisal trigger (round 1)

- **Role:** independent comprehensive reviewer — serves as BOTH task-review and the Phase-E whole-diff gate.
- **Slug:** `eae88df..c122dad` (2 commits: `6994b27` spec R0-GREEN; `c122dad` feat Tasks 1-3).
- **Contract:** `design/SPEC_appraisal_trigger_minimal.md`.
- **Verified against current source** (`HEAD == c122dad`): `crates/btctax-core/src/tax/tables.rs`,
  `src/state.rs`, `src/project/fold.rs`, `src/tax/compute.rs`, `src/tax/mod.rs`, `src/lib.rs`,
  `crates/btctax-cli/src/render.rs`, KATs in `tests/kat_tax.rs` + `crates/btctax-cli/tests/verify_report.rs`,
  and `FOLLOWUPS.md`.
- **Verdict:** **READY TO MERGE. 0 Critical / 0 Important.** 1 Minor, 1 Nit (neither blocks).

---

## 1. Proxy correctness (highest priority) — CORRECT

`fold.rs` Donate arm (read at source, lines 1067-1104):

```
let deduction_proxy: Usd = legs.iter()
    .map(|leg| if leg.term == Term::LongTerm { leg.fmv_at_transfer } else { leg.basis })
    .sum();
if deduction_proxy > crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD { … add_blocker … }
```

- **Computed from the FINAL persisted legs.** `make_removal_legs` (1041-1042) → `consume_fee` (1046-1056)
  → `carry.rehome_onto_removal_leg(last)` (1057-1058) all run BEFORE the proxy (1074-1083); the proxy
  reads the same `legs` that are then moved into `st.removals.push(Removal{…})` (1105-1112). A re-homed
  ST fee-cent basis is included (`rehome_onto_removal_leg` mutates `leg.basis`, fold.rs:274-276; the
  proxy uses `basis` for ST legs). Matches spec R0-M1 exactly.
- **Strictly `>` the threshold** (`QUALIFIED_APPRAISAL_THRESHOLD = dec!(5000)`, tables.rs). Correct
  ("more than $5,000"; Form 8283 Section A ≤ $5k / Section B > $5k → exactly $5,000 needs no appraisal).
- **`term` is holding-period-derived** (`term_for(c.gain_hp_start, removed)`, make_removal_legs:233),
  i.e. LT/ST at the donation date — the right axis for the §170(e) FMV-vs-basis proxy.
- **The proxy never UNDER-flags a single donation (never misses a required appraisal).** Per leg the
  actual §170 claimed deduction is: LT capital-gain → FMV; ST(appreciated)/ordinary → basis;
  depreciated → FMV (lesser-of). The proxy yields LT→FMV, ST→basis, which is **≥ the actual deduction in
  every case** (LT ordinary-income/inventory: proxy FMV ≥ actual basis = over-flag; depreciated ST:
  proxy basis > actual FMV = over-flag; all others exact). It is a conservative upper bound → misses
  nothing per-event; over-flags only in the safe "verify" direction. The only residual miss is
  cross-donation aggregation (§170(f)(11)(F)), explicitly deferred and disclosed.

**Headline KATs re-derived:**
- (a) LT $60k-FMV / $5k-basis → proxy = FMV **$60k > $5k → FLAGGED** (the case the rejected AND-rule
  missed, since basis $5k is not > $5k). ✓
- (b) ST $10k-FMV / $2k-basis → proxy = basis **$2k ≤ $5k → NOT flagged**. ✓
- (d) LT FMV=$5000.00 → proxy $5000.00 (not > $5000) → **NOT flagged**; LT FMV=$5000.01 → proxy
  $5000.01 → **FLAGGED**. ✓
- (c) mixed above: LT-leg FMV $50k + ST-leg basis $2k = **$52k > $5k → FLAGGED**; mixed below: LT-leg
  FMV $100 + ST-leg basis $450 = **$550 ≤ $5k → NOT flagged**. ✓

## 2. Detail text is tax-CORRECT — CONFIRMED (read verbatim from fold.rs:1088-1102)

Every required element is present and correctly stated:
- **§170(f)(11)(C) $5,000 claimed-deduction threshold** — "exceeds the §170(f)(11)(C) $5,000 threshold." ✓
- **CCA 202302012 crypto point** — "a crypto donation with a claimed deduction >$5,000 requires a
  qualified appraisal; the exchange-price/readily-valued exception does NOT apply to crypto." ✓
- **Over-flag caveat framed by CHARACTER, deducted at basis REGARDLESS of holding period** — "crypto held
  as inventory/for sale in a trade or business (§1221(a)(1)) or other ordinary-income property is
  deducted at basis under §170(e) REGARDLESS of holding period — the precise determination is deferred;
  verify." Character-framed (§1221(a)(1) inventory/for-sale), not holding-period-framed. ✓
- **§170(f)(11)(F) aggregation caveat** — "this flags a single donation; the $5,000 test also aggregates
  similar donated items across the tax year — cross-donation aggregation is not considered here." ✓
- **Does NOT contain the tax-INCORRECT "mining held >1yr is deducted at basis."** The substring
  "mining" does not appear anywhere in the emitted detail. This is the R0-I1 defect that was chased to
  green in the spec; it is absent from shipped output. Investment-held mined BTC >1yr is a capital
  asset → FMV → correctly flagged, and nothing in the text would induce a taxpayer to under-claim it. ✓

## 3. Advisory semantics — CORRECT

- **`QualifiedAppraisalNote` is `Severity::Advisory`** — placed in the Advisory arm of
  `BlockerKind::severity()` (state.rs), doc comment states "never gates `compute_tax_year`." ✓
- **Never gates compute.** `compute_tax_year` gates only via `first_hard_blocker` (compute.rs:239, 419-423
  → `.find(|b| b.kind.severity() == Severity::Hard)`). An Advisory categorically cannot gate. KAT (e)
  is genuine: it builds a donation-only state (0 Hard blockers, exactly 1 QualifiedAppraisalNote),
  supplies a synthetic 2026 `TaxTable` + `TaxProfile`, and asserts `compute_tax_year(..) ==
  TaxOutcome::Computed(_)`. ✓
- **Per-donation-event emission** (not single-fire) — emitted inside the Donate arm, NOT via the
  `note_pre2025_once` guard. KAT (f) proves two donations → two notes. ✓
- **Decoupled from the user's `appraisal_required` bool** — the arm never reads it for emission (only
  passes `*appraisal_required` through to the persisted `Removal`). KAT (h) locks both directions:
  proxy>$5k + bool=false → still emits; proxy≤$5k + bool=true → does not emit. ✓

## 4. No unintended change — CONFIRMED

- The Donate arm only **adds** the proxy+emission block; `make_removal_legs`, `consume_principal`,
  `consume_fee`, `rehome_onto_removal_leg` are untouched (diff shows only `+` lines). No basis/gain/
  removal math changed; no tax figure moves.
- Constant lives in `tables.rs` as a `pub const … : Usd = dec!(5000)` with §170(f)(11)(C) cite +
  "STATUTORY / NOT inflation-indexed / never in a TaxTable" convention — NOT a `TaxTable` field. ✓
- **No render.rs change** — advisories auto-render: `render_verify` buckets by `b.kind.severity()`
  (render.rs:411) and prints `[{:?}] {evt} :: {detail}` for each advisory (render.rs:983-989); the new
  variant renders via Debug as "QualifiedAppraisalNote". ✓
- **Backward-compatible / no latent exhaustiveness break.** The only match touching `BlockerKind` is
  `severity()` (updated) and render's match on `severity()` (a `Severity`, unaffected). No other
  exhaustive `match` on the enum exists; blockers are recomputed by projection (not persisted), so the
  new serde variant raises no deserialization concern. clippy `-D warnings` clean confirms.

## 5. KATs sufficient + genuine — YES

All 8 spec groups (a-h) present as 11 `qualified_appraisal_*` functions in `kat_tax.rs` plus 2
`verify_donation_*` end-to-end KATs in `verify_report.rs`. They drive real `project(...)` /
CLI (init→import→reclassify→verify) paths and inspect `st.blockers` / `report.advisory` — not
tautological. Mixed-leg cases (both directions) correct; GiftOut KAT (g) confirms only the Donate arm
emits (`RemovalKind::Gift` never produces the note); the verify KATs assert the detail carries
§170(f)(11)(C), CCA 202302012, §170(e), §170(f)(11)(F) and the proxy dollar amount, that `report.hard`
is empty, and that `render_verify` surfaces the note.

## 6. NFR4 / NFR5 — CONFIRMED

- **NFR5 exact Decimal / no float:** the proxy is `Usd` (rust_decimal) throughout — `.sum()` over
  Decimal, `>` against a Decimal constant, `{deduction_proxy:.2}` Decimal formatting. No `f64`.
- **NFR4 determinism:** pure function of the ordered legs; blockers appended in deterministic order.
- **Synthetic-only:** all KATs use `StaticPrices::default()` and synthetic CSVs/events; no network/PII.

---

## Findings

### Critical — none.
### Important — none.

### Minor

**MIN-1 — Spec Task-3 FOLLOWUPS deliverable not completed (documentation drift; not in the diff).**
Spec Task 3 mandates two `FOLLOWUPS.md` edits as part of this slug: (i) record the deferred items
(precise §170(e) ordinary-income/character-based deduction that upgrades this proxy; §170(f)(11)(F)
cross-donation aggregation) and (ii) **[R0-Nit] reconcile the "Standing roadmap" line that still
describes this slug as the superseded `FMV>$5k ∧ basis>$5k` rule → the term-aware deduction proxy.**
`FOLLOWUPS.md` is **not** in the whole-slug diff (9 files, none is FOLLOWUPS), and line 10 still reads
"…minimal appraisal-trigger **FMV>$5k∧basis>$5k**…" — i.e. it now describes the *shipped* feature by
the exact AND-rule the spec explicitly rejected (it under-flags the textbook LT-appreciated donation).
No tax-correctness or runtime impact (docs only), so it does not touch the 0C/0I code gate — but it is
a spec-mandated Phase-E deliverable and a stale roadmap line that could misdirect the next implementer.
**Fix:** land the two FOLLOWUPS edits (reconcile line 10 to the term-aware proxy; note the shipped
minimal advisory + the two Phase-2 deferrals) in this slug or as an immediate follow-up commit.

### Nit

**NIT-1 — Report KAT count off by one.** `appraisal-report.md` says "12 new test functions" in
`kat_tax.rs`; there are **11** (`grep -c "fn qualified_appraisal"` = 11). All 8 groups (a-h) are covered;
this is a report-accuracy nit only, no code impact.

---

## Verdict

**The slug is READY TO MERGE: 0 Critical / 0 Important.** The term-aware proxy is tax-correct and
conservative (never misses a required appraisal for a single donation; over-flags only in the safe
direction); the emitted detail text is tax-correct and free of the R0-I1 "mining held >1yr = basis"
error; the advisory is Advisory-severity and provably never gates `compute_tax_year`; emission is
per-event and decoupled from the manual bool; no basis/gain/removal math or tax figure changed; KATs
are genuine and cover all 8 groups. The single Minor (MIN-1, the un-landed FOLLOWUPS reconciliation) is
a documentation deliverable that should be completed with the merge but does not block the code gate.
