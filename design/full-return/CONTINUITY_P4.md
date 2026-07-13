# CONTINUITY — Full-Return Expansion, Phase 4 (resume point)

**Written:** 2026-07-12. **Branch:** `full-return`. **HEAD:** `b61d595`.
**Read alongside:** the `full-return-expansion-roadmap` auto-memory (loaded each session),
`design/SPEC_full_return.md`, `design/IMPLEMENTATION_PLAN_full_return.md`, `design/full-return/FOLLOWUPS.md`,
`STANDARD_WORKFLOW.md`. This file is the fast on-ramp; those are authoritative.

## Where we are

- **Design GREEN** (spec + plan). **Phases 0/1/2/3 COMPLETE + GREEN-CERTIFIED.**
- **Phase 3 certified GREEN at `a64ff8f`** (Fable r1 1C/2I/4M → fold `695b1e6` → r2 GREEN 0C/0I).
- **Phase 4 IN PROGRESS** — the largest phase: credits + other taxes + the absolute-liability assembly
  (everything P2/P3 deferred) + the §6 delta-vs-absolute dual report.
  - **P4.0 DONE + committed green (`b61d595`):** the absolute WITH-crypto income assembly, SPEC §5 stages 1–2.
    New `AbsoluteReturn` struct + `assemble_absolute(ri, state, params, table, year)` in
    `crates/btctax-core/src/tax/return_1040.rs`. Produces income L1a–L9, adjustments L10, with-crypto AGI L11,
    and the Schedule SE/§6017 block (reuses the FROZEN `compute_se_tax`; `base < $400` ⇒ `se = None` ⇒ zeroes
    SE tax + ½-SE + 8959 L8). ½-SE = `SeTaxResult.deductible_half`. 2 KATs. Core lib 171/0; clippy clean.

## Operating contract (do NOT drift)

- **Pipeline (user-directed, standing):** **opus AUTHORS/implements, Fable REVIEWS to green** (0 Critical /
  0 Important) at every phase gate. Fable is pre-approved as reviewer for THIS project (overrides the
  ask-first Fable-escalation default for review steps only). Persist each review verbatim before folding;
  re-review after every fold incl. the last.
- **Standard workflow:** phased TDD; green = full suite passes AND 0C/0I. **Per-phase follow-up burndown by
  ownership** — on entering/closing a phase, reconcile FOLLOWUPS; do the ones this phase owns; only ownerless
  residue batches to the end.
- **FROZEN files — never edit (byte-pinned by `frozen_guard`):** `crates/btctax-core/src/tax/types.rs`,
  `compute.rs`, `se.rs`. The absolute assembly READS them (imports `compute_se_tax`, `net_1222`, etc.).
  Verify after any change: `git diff 059ec2a..HEAD -- <those three>` = 0 bytes.
- **Frozen seam:** `derive_tax_profile` = NON-crypto `TaxProfile` (engine adds the crypto DELTA). The absolute
  side (`assemble_absolute`) re-combines WITH crypto itself — never by un-delta-ing `compute_tax_year`.
- **Money** = `Usd`/`Decimal`, no floats; `round_dollar` half-up (lines), `round_cents` (SE components).
- **Licensing:** permissive (MIT OR Unlicense), clean-room; do NOT relicense to GPL.

## Next steps (Phase 4 remaining) — task list order

The session task list (P4.1…P4 review) mirrors the plan §Phase 4. Suggested build order:

1. **Absolute deductions L12–L15** (SPEC §5 stage 3; the critical-path unblocker):
   - Schedule A on **WITH-crypto AGI** (G7): medical `max(0, med − 7.5%·AGI)`, SALT, mortgage, **charitable at
     with-crypto AGI INCLUDING crypto donations from the ledger (§170(e) classes)** — this is the
     `p3-crypto-donation-delta-integration` P4 requirement (r1 §3.3 ruling: derive excludes; the ABSOLUTE Sch A
     includes them). Reuse `apply_170b`. **NB: non-50%-org classes are refused upstream (C1); a P4 caller must
     route through `screen_inputs` or add an `apply_170b` invariant guard — review-r2 N1 rider (iii).**
   - `deduction = max(std, Sch A)` with the **G21-completed dependent-floor earned income** = wages + Sch C
     net − ½SE (now computable — `p3-m3-dependent-floor-earned-income-G21`).
   - QBI/8995 → L13 (**task P4.1**: REIT box5×20%, TI-before-QBI limit, refuse above threshold/non-REIT QBI,
     REIT/PTP carryforward write-back + R3-M6 precedence KAT).
   - L14 = L12 + L13; **L15 = AGI − L14 (refuse if ≤ 0)**.
2. **L16** = `method.rs::qdcgt_line16` on with-crypto TI + **§7.2 Schedule-D method routing** (the 4 paths:
   gain-both / ST-gain-LT-loss / net-loss-capped / zero → QDCGT vs Tax Table). Folds `p3-l16-absolute-P4`.
   Also apply the **`p2-pref-over-ti-clamp`** min-cap here (cap pref slice at TI, reduce `other` first).
3. **Sch 2 other taxes:** AMT screen (refuse-trigger, **P4.3**, KAT-14); SE tax → **Sch 2 L4 = ss + medicare
   ONLY** (unbundle the 0.9% — KAT-6); **Form 8959 Part I+II+V** (**P4.5**; Part II reads `se.addl`, Part
   I = 0.9%·max(0, Σbox5 − thr), Part V → 25c); **absolute Form 8960** (**P4.5**: NII rebuilt FROM LINE ITEMS
   incl. crypto lending interest on L7/R3-M5 — NOT the engine's `nii_with`; MAGI = AGI fail-closed).
4. **Credits:** §904(j) FTC → Sch 3 L1 (**P4.2**, KAT-16, refuse above $300/$600); CTC/ODC **L19 = 0 +
   loud advisory** (**P4.7**, KAT).
5. **Excess-SS + payments** (**P4.6**, KAT-11): per-employer clamp, per-person, ≥2 employers → Sch 3 L11;
   settle L33–L37 (**L36 apply-to-next pinned 0/blank** — `spec-s48-l36`).
6. **§6 dual report** (**P4.8**): render absolute-liability lines + crypto delta side-by-side, §6 labels,
   never reconciled; document delta-deduction approximate. **KAT on a fixture where
   `absolute_with − absolute_without ≠ delta`** — use a **medical-floor fixture** (per the r1 §3.3 ruling, the
   one anti-conservative channel). Folds `p2-provenance-printing` (§4.12) + `p2-r2-n4-pseudo-year-viewer-gap`.
7. **P4 whole-diff Fable review → GREEN** (fold to 0C/0I, re-review after every fold, persist verbatim).

**Acceptance (plan §Phase 4):** deep/02 **Ex.2 ($60k mining)** end-to-end other-taxes block to the cent;
**KAT-5/5b** reduce-to-delta (4 regimes; SE fixture NII-binding for 5, MAGI-binding documented `absolute<delta`
for 5b); **KAT-12** (25c composition); dual-report renders + labels; every credit/other-tax has a golden;
refuse rows (QBI / FTC-over-cap / single-employer-SS / AMT) KAT'd.

## Key files / entry points

- `crates/btctax-core/src/tax/return_1040.rs` — `derive_tax_profile` (frozen seam), **`assemble_absolute` +
  `AbsoluteReturn`** (new, P4.0), `screen_compute_dependent`, `crypto_income`, `capital_gain_line7`, the
  `sum_*` income helpers, `standard_deduction`/`schedule_a_deduction`/`choose_deduction`,
  `apply_170b` (in `tax/charitable.rs`).
- `crates/btctax-core/src/tax/other_taxes.rs` — **to CREATE** (plan): absolute 8960 + 8959 Part I/II/V.
- `crates/btctax-core/src/tax/se.rs` (FROZEN) — `compute_se_tax` → `SeTaxResult { net_se, base, ss, medicare,
  addl, total, deductible_half }`. `Sch 2 L4 = ss + medicare`; `8959 Part II = addl`; `½-SE = deductible_half`.
- `crates/btctax-core/src/tax/method.rs` — `qdcgt_line16`, Tax-Table/QDCGT kernel (for L16).
- `crates/btctax-cli/src/{resolve.rs,session.rs}` — `resolve_core`/`resolve_and_screen`/`ProfileOutcome`
  (the fail-closed resolver ladder; the dual report + provenance printing wire in here).
- Reviews: `design/full-return/reviews/IMPL-P{0..3}-fable-review-*.md`. FOLLOWUPS: `design/full-return/FOLLOWUPS.md`.

## Watch-outs a reviewer will hit

- **§6017** is on the SE `base` (0.9235-factored), not gross; `base ≥ $400` files. `assemble_absolute`
  applies it via `.filter(|r| r.base >= 400)`.
- **Two W-2 channels** (deep/02 C4): §1402(b)(1) SS cap uses the SE-earner's OWN box3+box7; 8959 uses
  HOUSEHOLD Σbox5. Don't conflate.
- **8960 NII ≠ engine `nii_with`** (deep/02 C2) — rebuild from line items for the absolute.
- **`SeTaxResult.total` bundles the 0.9%** (deep/02 C5) — unbundle for Sch 2 L4 or you double-count 8959.
- **Delta-vs-absolute never reconciles to the dollar** (§6) — different questions; label them.
