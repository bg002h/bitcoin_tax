# CONTINUITY — Full-Return Expansion, Phase 4 (resume point)

**Written:** 2026-07-12 (updated after P4.6). **Branch:** `full-return`. **HEAD:** `1ecd06d`.
**Read alongside:** the `full-return-expansion-roadmap` auto-memory (loaded each session),
`design/SPEC_full_return.md`, `design/IMPLEMENTATION_PLAN_full_return.md`, `design/full-return/FOLLOWUPS.md`,
`STANDARD_WORKFLOW.md`. This file is the fast on-ramp; those are authoritative.

## Where we are

- **Design GREEN** (spec + plan). **Phases 0/1/2/3 COMPLETE + GREEN-CERTIFIED.**
- **Phase 3 certified GREEN at `a64ff8f`** (Fable r1 1C/2I/4M → fold `695b1e6` → r2 GREEN 0C/0I).
- **Phase 4 IN PROGRESS — the absolute-return COMPUTE is COMPLETE (SPEC §5 stages 1–9) and validated.**
  Remaining: the §6 dual-report RENDER + carryover write-back persistence, then the P4 Fable gate. All
  increments below are committed green (core lib 220/0; clippy clean; frozen files byte-identical; the
  P4.3 state was whole-workspace-validated exit 0). Everything lives in
  `crates/btctax-core/src/tax/return_1040.rs` + the new sibling modules.
  - **P4.0 (`b61d595`):** income L1a–L11 + Schedule SE/§6017 block. `AbsoluteReturn` + `assemble_absolute`.
  - **P4.1a (`9788520`):** deductions L12–L15 + QBI (new `tax/qbi.rs`, Form 8995 REIT path; `qbi_ti_threshold`
    added to `FullReturnParams`). Absolute Sch A on with-crypto AGI incl. ledger §170(e) crypto donations
    (`crypto_charitable_gifts`); G21 dependent-earned = wages+SchC−½SE (closes `p3-m3-…-G21`). `screen_absolute`
    (post-assembly compute-dependent refuses: QBI-over-threshold, TI≤0-with-carryforward).
  - **P4.1b (`91e897c`):** regular tax L16 = `qdcgt_line16` on with-crypto TI, all four §7.2 Sch-D paths,
    pref-over-TI cap (folds `p3-l16-absolute-P4`, `p2-pref-over-ti-clamp` absolute side).
  - **P4.2 (`0397875`):** new `tax/other_taxes.rs` — Sch 2 L4 SE unbundle (KAT-6), Form 8959 Part I/II/V,
    absolute Form 8960 (NII rebuilt from line items, deep/02 C2; `nonbusiness_lending_interest` split).
  - **P4.3 (`e0fd7cc`):** new `tax/amt.rs` — the 2024 "Should I fill in Form 6251?" worksheet as a
    refuse-trigger (primary-source verified; saved to scratchpad `amt_worksheet_2024.txt`). `AmtParams` in
    `FullReturnParams`. Wired into `screen_absolute`.
  - **P4.4 (`951c664`):** §904(j) FTC → Sch 3 L1; CTC L19=0 (conservative omission); Sch 2/3 assembly →
    total tax L24.
  - **P4.5 (`5c82323`):** §6413(c) excess-SS (per-person, not pooled; `EMPLOYEE_OASDI_RATE` promoted to a
    shared const in `tables.rs`); payments L25/L33 → refund L35a / owed L37.
  - **P4.6 (`de8346b`):** reduce-to-delta KAT-5 (NII-binding: absolute 8960 == frozen delta, $380 cent-exact)
    + KAT-5b (MAGI-binding SE: absolute $238.25 < delta $380, documented §6 divergence).
  - **medical-floor KAT (`1ecd06d`):** with-crypto AGI shrinks the absolute Sch A medical deduction (the one
    anti-conservative channel §6 documents).

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

Build-order items 1–5 (deductions L12–L15, L16, Sch 2 other taxes, credits, excess-SS+payments) + the
reduce-to-delta / medical-floor KATs are **DONE + committed** (see "Where we are"). What remains:

1. **§6 dual report RENDER (P4.8) — the last implementation piece.** Wire the report path
   (`crates/btctax-cli/src/{resolve.rs,session.rs}` + the report render) to run the fail-closed screen ladder
   (`screen_inputs` → `screen_compute_dependent` → `assemble_absolute` → `screen_absolute`, all → `NotComputable`
   on refuse) and render the **absolute-liability lines + the crypto delta side-by-side** with the §6 labels
   ("different questions"); document the delta deduction as **approximate**; NEVER reconcile to the dollar.
   - **KAT (render-level):** a **medical-floor fixture** where `absolute_with − absolute_without ≠ delta` —
     the compute-level pin already exists (`medical_floor_uses_with_crypto_agi_shrinking_the_deduction`,
     `1ecd06d`); the render KAT asserts both numbers print with the never-reconcile labels.
   - Folds **`p2-provenance-printing`** (§4.12 — print `Provenance` + a `provenance_label` on every output) +
     **`p2-r2-n4-pseudo-year-viewer-gap`**.
   - **Carryover write-back PERSISTENCE + R3-M6 precedence** (still-open half of `p3-carryover-writeback-P4`
     + the QBI REIT/PTP half): persist `AbsoluteReturn.charitable_carryover_out` +
     `qbi_reit_ptp_carryforward_out` as year (Y+1)'s `charitable_carryover_in` / `qbi.reit_ptp_carryforward_in`
     in the CLI side-table; computed overwrites computed, **refuses to overwrite a user-entered value w/o
     `--force`**. Needs `crates/btctax-cli/src/return_inputs.rs` (the side-table get/set). The compute already
     EXPOSES both carryover-outs on `AbsoluteReturn`.
   - L36 apply-to-next stays pinned 0/blank (`spec-s48-l36`).
2. **P4 whole-diff Fable review → GREEN** (fold to 0C/0I, re-review after every fold, persist verbatim). Fable
   is pre-approved as reviewer for this project. The whole P4 diff = `b61d595..HEAD` on `full-return`.

**Compute entry points already built** (for the render to consume): `assemble_absolute(ri, state, params,
table, year) -> AbsoluteReturn` (income→refund/owed, every line a field); `screen_absolute(ri, &ar, params)`
(QBI-over-threshold / AMT / TI≤0-carryforward refuses); `screen_compute_dependent`; `screen_inputs`
(return_refuse.rs). All refuses are `Refusal { reason: RefuseReason, detail }`.

**Acceptance (plan §Phase 4):** deep/02 **Ex.2 ($60k mining)** end-to-end other-taxes block to the cent;
**KAT-5/5b** reduce-to-delta (4 regimes; SE fixture NII-binding for 5, MAGI-binding documented `absolute<delta`
for 5b); **KAT-12** (25c composition); dual-report renders + labels; every credit/other-tax has a golden;
refuse rows (QBI / FTC-over-cap / single-employer-SS / AMT) KAT'd.

## Key files / entry points

- `crates/btctax-core/src/tax/return_1040.rs` — `derive_tax_profile` (frozen seam), **`assemble_absolute` +
  `AbsoluteReturn`** (income→refund/owed, every 1040 line a field), `screen_absolute`,
  `screen_compute_dependent`, `crypto_income` (+ `nonbusiness_lending_interest`), `capital_net`,
  `crypto_charitable_gifts`, `excess_social_security`, the `sum_*` helpers,
  `standard_deduction`/`schedule_a_deduction`/`choose_deduction`.
- `crates/btctax-core/src/tax/other_taxes.rs` — **DONE (P4.2):** `sch2_line4_se`, `form_8959`/`Form8959`,
  `form_8960`/`Form8960` (absolute NIIT, line-item rebuild).
- `crates/btctax-core/src/tax/qbi.rs` — **DONE (P4.1a):** `compute_8995`/`Qbi8995`, `qbi_over_threshold`.
- `crates/btctax-core/src/tax/amt.rs` — **DONE (P4.3):** `amt_should_file_6251` (2024 worksheet refuse-trigger).
- `crates/btctax-core/src/tax/tables.rs` — `FullReturnParams` now carries `qbi_ti_threshold_*`, `amt:
  AmtParams`; new pub `EMPLOYEE_OASDI_RATE`. Bundled TY2024 values in `btctax-adapters/src/tax_tables.rs`.
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
