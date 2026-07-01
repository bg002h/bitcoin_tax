# Whole-slug review — P2-A §170(e) charitable-deduction computation (round 1)

**Scope:** task-review + whole-diff gate, single commit `1b76d48` (base `f619b11`).
**Artifacts read:** `design/SPEC_p2a_170e_deduction.md`, `.superpowers/sdd/p2a-report.md`,
`.superpowers/sdd/review-f619b11..1b76d48.diff`, and current source
(`state.rs`, `project/fold.rs`, `tax/compute.rs`, `cli/src/render.rs`, `tests/kat_tax.rs`,
`tests/verify_report.rs`, `tax/tables.rs`).
**Reviewer stance:** independent; verified source at HEAD against the diff (they match verbatim).

**Verdict: READY TO MERGE — 0 Critical / 0 Important.**
Tax-correctness, no-regression, and standalone (no double-count into B) all verified.
Findings below are 2 Minor + 1 Nit; none blocks merge.

---

## 1. Deduction rule tax-correct (highest priority) — PASS

`fold.rs:1078-1087`:
```
Σ over final legs: term==LongTerm ? fmv_at_transfer : fmv_at_transfer.min(basis)
```
`leg.basis` = `c.gain_basis` and `leg.fmv_at_transfer` = pro-rata FMV (verified in
`make_removal_legs`, `fold.rs:228-235`). Four cases re-derived:

| Case | Rule branch | Result | §170(e) correct? |
|---|---|---|---|
| LT appreciated | `fmv` | FMV | ✓ would-be gain is LTCG → no reduction |
| LT depreciated | `fmv` | FMV | ✓ would-be loss is not a reduction; capped at FMV |
| ST appreciated (basis<fmv) | `min(fmv,basis)` | basis | ✓ ST gain reduces FMV to basis |
| ST depreciated (basis>fmv) | `min(fmv,basis)` | FMV | ✓ no would-be gain → no reduction |

- **Deduction ≤ FMV in every branch** (LT = fmv; ST = min(fmv,basis) ≤ fmv). No path exceeds
  FMV, and each branch equals the exact §170(e) amount for the modeled universe. **No `ST→basis`
  over-claim path remains.**
- **Dual-basis robustness:** using `gain_basis` (donor carryover for received-gift lots) is the
  correct basis for the would-be-gain test; the no-gain/no-loss middle zone yields `fmv < gain_basis`
  → `min` = fmv = FMV (correct, no reduction). No over-claim from dual-basis lots.
- **ST-depreciated KAT genuine:** `claimed_deduction_st_depreciated_equals_fmv_not_basis`
  (buy 2025-06-01 → donate 2025-12-01 = ST; basis $8k / fmv $3k) asserts `Some(3000.00)` AND that
  `QualifiedAppraisalNote` does NOT fire. The helper `donate_single` routes `cost→basis`,
  `fmv→FMV` through the real fold, so the path is exercised for real. The old `basis`-proxy ($8k)
  would have fired — this is the corrected false positive.

## 2. Trigger reframe + no regression — PASS

- Trigger now fires off the stored `claimed_deduction` (`fold.rs:1088`, strict `>`
  `QUALIFIED_APPRAISAL_THRESHOLD = dec!(5000)`; boundary $5,000.00 not / $5,000.01 flagged —
  matches §170(f)(11)(C) "more than $5,000").
- **No shipped KAT changed meaning.** I inspected every pre-existing appraisal KAT: `lt_60k`,
  `st_10k_fmv_2k` (ST **appreciated** basis $2k<fmv $10k), `mixed_legs_above` (ST leg basis
  $2k / fmv $50k — appreciated), `mixed_legs_below` (ST leg basis $450 / fmv $900 — appreciated),
  both boundaries (LT, fmv-based), `proxy_over_5k_flag_false`, `proxy_under_5k_flag_true`
  (ST appreciated basis $2k / fmv $60k), and the CLI `$510` test (ST **appreciated** basis $510 /
  fmv $1,000). For all APPRECIATED-ST/LT cases `min(fmv,basis)`=basis and LT=fmv, so results are
  identical to the old proxy. **No pre-existing KAT is ST-depreciated**, so the only behavior
  change (ST-depreciated → fmv) has no shipped-KAT overlap; the new lock KAT adds the coverage.
  The `kat_tax.rs` and `verify_report.rs` diffs are **purely additive (0 deletions)** — confirmed.
- **Detail text:** exact `"Claimed deduction $X"`; retains all three caveats + citations —
  dealer/inventory §1221(a)(1); donee-type private foundation §170(e)(1)(B)(ii) + "crypto is not
  qualified appreciated stock"; §170(f)(11)(F) aggregation; CCA 202302012; §170(f)(11)(C); §170(e).
  No "estimated proxy" and no "mining=basis" language. KAT
  `appraisal_detail_named_claimed_deduction_with_all_caveats` asserts each substring.

## 3. Gift → None — PASS

Gift arm `fold.rs:1001` sets `claimed_deduction: None`; only the Donation push (`fold.rs:1123`)
carries `Some(claimed_deduction)`. `gift_claimed_deduction_is_none` locks it.

## 4. Standalone — NOT fed to B (no double-count) — PASS

- `crates/btctax-core/src/tax/{compute.rs,types.rs,tables.rs}` are **NOT in the change set**
  (`git diff --name-only` confirms). The diff touches only 5 files.
- `grep` confirms **zero** references to `removals` / `claimed_deduction` anywhere under
  `src/tax/`. `compute_tax_year` reads only `state.disposals` and `state.income_recognized`
  (`compute.rs:281,294`). `TaxProfile` / `TaxResult` unchanged; `ordinary_taxable_income` remains
  user-supplied post-deduction. The §170 figure is not wired into capital-gains math.
- No capital-gains / removal-basis / gain math touched (fold.rs change is confined to the new
  local `claimed_deduction` and the trigger comparison; the removal-leg basis/fmv/term computation
  is unchanged).

## 5. Surfacing correct — PASS

- Donation header appends `[claimed deduction $X]` (`render.rs:226-237`); gift shows none
  (`None → String::new()`). `render_report_donation_header_shows_claimed_deduction` locks both.
- `removals.csv` gains a trailing `claimed_deduction` column (index 8; existing columns 0-7
  unaffected → stable header); donation → `d.to_string()` (consistent with existing
  `basis`/`fmv` columns), gift → empty (`render.rs:100-136`).
- Per-year total (`render.rs:266-279`): `Σ claimed_deduction where kind==Donation &&
  yr(removed_at.year())` — **per-removal (filter_map over removals, not per-leg)** → correct;
  year filter matches disposals/income (`yr` closure `render.rs:156`, `None ⇒ all`). Label carries
  "Schedule A itemized" + "BEFORE §170(b) AGI limits / carryover".
  `render_report_charitable_total_year_filter_and_qualifier` locks 2026=$16k, 2025=$8k, and
  that the prior-year $24k combined sum does NOT appear. KATs are genuine (real fold / real CSV).

## 6. NFR — PASS

- **Determinism (NFR4):** `claimed_deduction` is a pure function of the deterministically-ordered
  final legs; Decimal addition is order-independent regardless.
- **Exact Decimal / no float (NFR5):** `Usd` = `Decimal`; `.min()` and `.sum()` on Decimal; no
  float introduced.
- **Additive field / no migration:** `Removal` derives only `Debug, Clone, PartialEq, Eq`
  (no serde) and is a projected type rebuilt from events → additive `Option<Usd>` field is safe.
- **Synthetic-only / PII:** all new tests use tempdirs + synthetic CSV; no ReadOnly access.
- **Advisory-only:** `QualifiedAppraisalNote` severity unchanged; never gates `compute_tax_year`.

---

## Findings

### Minor

- **M1 — `removals.csv` repeats the per-removal deduction on every leg row (export
  double-count footgun).** `render.rs:129-136` writes `deduction_str` on *each* leg row of a
  donation, with the full per-removal amount repeated. For a multi-leg donation (e.g. the mixed
  LT+ST case, deduction $52k over 2 legs) the CSV shows `52000.00` on **both** rows, so a naive
  `SUM(claimed_deduction)` over rows over-counts to $104k. The authoritative surfaced figure (the
  `render_report` per-year total) is per-removal and **correct**, and this is an export
  representation — not fed into any computed tax figure — so it is not the "feeding-B" double-count
  the gate treats as blocking. Two cheap fixes: emit the amount only on the first leg row (empty on
  subsequent legs), or add "(per-donation total, repeated per leg — do not sum across legs)" to the
  CSV header comment at `render.rs:101-102`. Note the only CSV KAT
  (`csv_removals_has_claimed_deduction_column`) uses a single-leg donation, so the multi-leg
  repetition is untested. Recommend a follow-up (not a merge blocker).

- **M2 — Spec Task 3 FOLLOWUPS deliverable not completed.** The diff does not touch
  `FOLLOWUPS.md`. The P2-A deferrals the spec Plan Task 3 enumerates — donee-type modeling
  (public charity vs private foundation, §170(e)(1)(B)); §170(b) AGI percentage limits + 5-yr
  carryover + OBBBA-2026 0.5% floor / 35% cap; and the explicit standalone / double-count trap
  note — are not recorded (only the earlier appraisal-trigger slug's §170(e)/aggregation deferrals
  are present). Per the standard workflow FOLLOWUPS is a required artifact. Mitigating: all three
  deferrals are already disclosed **in-product** (the trigger detail caveats and the surfaced
  "BEFORE §170(b) AGI limits / carryover" label), so there is no user-facing correctness gap.
  Recommend adding the three entries before ship; non-code, cheap.

### Nit

- **N1 — Legacy "proxy" terminology lingers in pre-existing tests.** Test names/comments
  (`qualified_appraisal_proxy_over_5k...`, `..._under_5k...`, and the CLI `$510` comment "ST proxy
  = basis" / "FMV doesn't matter for ST") still use the retired "proxy" framing. Cosmetic only —
  every assertion remains valid (those cases are ST-appreciated, where `min(fmv,basis)`=basis, so
  the "= basis" comments still describe the actual result). Optional cleanup.

---

## Gate confirmation

Full validation suite reported GREEN by the implementer (467 tests, clippy `-D warnings`, fmt,
release build, PII clean) — not re-run per instructions. This review independently verified the
three high-risk axes the gate exists to protect: **tax-correctness** (no over-claim path,
deduction ≤ FMV, all four §170(e) cases correct), **no regression** (no shipped KAT changed
meaning; behavior differs from the old proxy only for the previously-untested ST-depreciated case,
correctly reducing false flags), and **standalone** (compute.rs/types.rs unchanged; not wired into
B). **Ready to merge (0 Critical / 0 Important).**
