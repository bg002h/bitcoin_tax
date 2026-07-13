# CONTINUITY — Full-Return Expansion — **PHASE 4 IS CLOSED (CERTIFIED GREEN)**

> **STATUS: Phase 4 COMPLETE + CERTIFIED at `6eeda51`.** Compute (SPEC §5 stages 1–9) + the §6 dual report
> (Fable IMPL-P4 r1 1C/4I → fold → **r2 GREEN 0C/0I**) **and** P4.9 carryover write-back (Fable IMPL-P4.9 r1
> 0C/3I → fold `6eeda51` → **r2 GREEN 0C/0I**). Reviews: `reviews/IMPL-P4-fable-review-r{1,2}.md` +
> `reviews/IMPL-P4.9-fable-review-r{1,2}.md`. All three CI gates green (test/clippy `-D warnings`/fmt);
> whole workspace 1554 tests pass; frozen files byte-identical.
>
> **NEXT: Phase 5** (LIMITATIONS doc + conservative-omission advisories), then P6 (PDF fillers), P7 (goldens).
> P5 entry-gate sweep MUST pick up: `p4-r1-m3-ctc-advisory-P5`, `p4-r1-n1-taxyearreport-struct`, and the
> **LIMITATIONS.md line-by-line pass** (drafted early at `6eeda51`; its "forms filled" line was corrected —
> it must be re-checked at the P5 gate and again in P6 when the fillers land).

---

## (historical) Phase-4 resume notes

**Written:** 2026-07-12 (updated after the P4 gate → GREEN). **Branch:** `full-return`. **HEAD:** past `018e199`.
**Read alongside:** the `full-return-expansion-roadmap` auto-memory (loaded each session),
`design/SPEC_full_return.md`, `design/IMPLEMENTATION_PLAN_full_return.md`, `design/full-return/FOLLOWUPS.md`,
`STANDARD_WORKFLOW.md`. This file is the fast on-ramp; those are authoritative.

## Where we are

- **Design GREEN** (spec + plan). **Phases 0/1/2/3 COMPLETE + GREEN-CERTIFIED.**
- **Phase 3 certified GREEN at `a64ff8f`** (Fable r1 1C/2I/4M → fold `695b1e6` → r2 GREEN 0C/0I).
- **Phase 4 COMPUTE + §6 DUAL REPORT are CERTIFIED GREEN** at `018e199` (Fable IMPL-P4 r1 1C/4I/4M/3N →
  fold `018e199` → **r2 GREEN 0C/0I**; reviews in `reviews/IMPL-P4-fable-review-r{1,2}.md`; r2 Nit folded
  after, non-gating). SPEC §5 stages 1–9 + the §6 dual report. core lib 227/0; adapters 58/0; cli
  tax_report 24/0; clippy clean; frozen files byte-identical; whole-workspace exit 0. **Remaining P4 work:
  only P4.9 (carryover write-back) — see Next steps.** Everything lives in
  `crates/btctax-core/src/tax/return_1040.rs` + the new sibling modules (`qbi`, `other_taxes`, `amt`) + the
  CLI render/report wiring.
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
  - **P4.8 (`dc744f1`) — §6 dual report RENDER (read-only):** `report --tax-year` now renders the absolute
    filed 1040 (income→total tax L24→refund/owed) side-by-side with the crypto delta, with the §6
    never-reconcile labels + the §4.12 provenance line (`render_dual_report` / `provenance_label` in
    `btctax-cli/src/render.rs`; wired into `report_tax_year` as a 7th `TaxYearReport` field, printed by
    main.rs). Fetches `ReturnInputs` + runs `assemble_absolute` + `screen_absolute` (refuse → "NOT
    COMPUTABLE" for the absolute side while the delta still shows). 1 integration KAT; all 23 report goldens
    unchanged. Folds `p2-provenance-printing` (full-return-output half).

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

Everything through the **§6 dual-report RENDER is DONE + committed** (compute stages 1–9 + the dual report +
provenance + reduce-to-delta / medical-floor KATs — see "Where we are"). **Two items remain:**

1. **Carryover write-back PERSISTENCE (P4.9)** — still-open half of `p3-carryover-writeback-P4` +
   `p1-carryover-writeback-P3P4` + the QBI REIT/PTP half + R3-M6 precedence. **Deliberately deferred from
   P4.8** (P4.8 is read-only): persisting is a distinct increment with a **model change + a write-on-report
   design decision**:
   - Persist `AbsoluteReturn.charitable_carryover_out` (`Vec<CharitableCarryItem>`) +
     `qbi_reit_ptp_carryforward_out` (`Usd`) as year **(Y+1)**'s `charitable_carryover_in` /
     `qbi.reit_ptp_carryforward_in` via the CLI side-table (`btctax-cli/src/return_inputs.rs` `get`/`set`,
     then `Session::save`). The compute already EXPOSES both on `AbsoluteReturn`.
   - **R3-M6 precedence:** a computed carryover-in overwrites a prior **computed** value but **refuses to
     overwrite a user-entered** one (warn + `--force`). Needs **carryover PROVENANCE** — `CharitableCarryItem`
     (and the QBI carryforward) have no computed-vs-user flag today, so this is a `return_inputs.rs` MODEL
     change (`#[serde(default)]` provenance, defaulting to User for back-compat).
   - **Design decision to confirm:** `report` currently is read-only; R3-M6 says write-back "at report time".
     Either add a `--write-carryover`/`--force` flag to `report`, or a dedicated `carryover apply` command.
   - No fail-open in deferring: nothing persists a wrong carryover today (the filer carries forward manually,
     the P1–P3 behavior). L36 apply-to-next stays pinned 0/blank (`spec-s48-l36`).
   - `p2-r2-n4-pseudo-year-viewer-gap` (viewer/CLI output-consistency) also folds around here.
2. **P4 whole-diff Fable review → GREEN** (fold to 0C/0I, re-review after every fold, persist verbatim). Fable
   is pre-approved as reviewer for this project. The whole P4 diff = `b61d595..HEAD` on `full-return`. (May be
   run BEFORE P4.9 on the compute+render if desired — the write-back is an isolated additive follow-on.)

**Compute + render entry points built:** `assemble_absolute(ri, state, params, table, year) -> AbsoluteReturn`
(income→refund/owed, every line a field); `screen_absolute(ri, &ar, params)` (QBI/AMT/TI≤0 refuses);
`render::render_dual_report` / `render::provenance_label` (CLI). All refuses are
`Refusal { reason: RefuseReason, detail }`.

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
