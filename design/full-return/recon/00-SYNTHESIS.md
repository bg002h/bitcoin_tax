# Full Return — Recon Synthesis (opus first pass)

**Date:** 2026-07-11 · **Status:** recon complete (opus pass); Fable second sweep pending; NOT yet specced.
**Scope (user-locked):** "Common W-2 household" US federal 1040 — W-2 wages, 1099-INT / 1099-DIV,
standard-vs-itemized (Schedule A: mortgage, SALT, charitable), Schedule B, Schedule 1 basics, wrapping
the existing crypto 8949/Schedule D engine. **PDF output only** (print-and-mail, extend `btctax-forms`).
**Manual/structured entry** (offline CLI/TOML). No e-file, no OCR.

Source reports (read them for detail + citations):
- `01-form-graph.md` — form set + line-flow DAG + 8-stage compute order
- `02-computation-worksheets.md` — the exact math + reuse-vs-new table
- `03-irs-fillable-pdfs.md` — PDF feasibility (all GREEN)
- `04-input-data-model.md` — `ReturnInputs` schema + CLI surface + migration
- `05-prior-art-verification.md` — prior art, layered test plan, legal posture, redaction guide

**Round-2 opus deep-dives (spec-grade; see §8):** `deep/01-tax-table-qdcgt-method.md`,
`deep/02-derivation-and-absolute-niit-medicare.md`, `deep/03-ty2024-field-maps.md`,
`deep/04-schedule-a-and-std-deduction-engine.md`.

---

## 1. TL;DR — the shape

1. **The engine today is a crypto *delta* engine, not a 1040 calculator.** It computes `tax(with crypto) −
   tax(without)` and collapses everything non-crypto into two opaque post-deduction scalars
   (`ordinary_taxable_income`, `magi_excluding_crypto`). A full return **inverts the objective**: build the
   base from line-items → produce **absolute** liability.
2. **The hard math already exists and is reusable nearly verbatim.** `ordinary_tax_on` (= Tax Computation
   Worksheet), `preferential_tax`/`PrefSplit` (= the §1(h) 0/15/20 block of the QDCGT worksheet, lines
   6–21), `net_1222`, the NIIT closure, and `se.rs` are the exact building blocks. **The work is assembly +
   data, not new rate math.**
3. **The expansion is mostly ADDITIVE.** New per-year `ReturnInputs` side-table (line-items + PII +
   payments) → pure `derive_tax_profile()` → the frozen engine. `TaxProfile`/`compute.rs` and their ~80
   construction sites stay untouched; today's hand-entered scalars become the "raw override" escape hatch
   for crypto-only / what-if users.
4. **All six new PDFs are GREEN.** 1040 + Sch 1/2/3/A/B × 2024/2025 are the same XFA-hybrid AcroForms the
   recipe already fills; `lopdf` + `NeedAppearances` + geometric read-back extend uniformly.
5. **Two things are genuinely new work:** (a) an **absolute-liability assembly** layer (AGI → deduction →
   taxable income → tax → other taxes → total → refund/owed), and (b) the **IRS Tax Table + whole-dollar
   rounding** method for the common under-$100k case — see the top risk below.

---

## 2. Top correctness risk (independently flagged by recon 02 AND 05) — "Layer 0"

The engine computes tax by the **exact marginal formula at cent precision** (`ROUND_HALF_EVEN`). That is
correct for a *delta* but **wrong for an absolute filed return**:

- For **taxable income < $100,000** (the common W-2 case is *usually here*), the IRS **requires the binned
  Tax Table** ($50-wide rows, tax at the bin midpoint, whole-dollar) — not the formula. The exact formula
  differs from the official Tax Table number by up to ~$8.
- 1040 lines use **whole-dollar rounding**; the engine rounds to cents.

**Consequence:** every end-to-end golden diff will show spurious $1–$15 deltas until this is built — we
won't be able to tell real bugs from rounding noise. **This must be built first.**

**Recommendation (adopt, not a user question):** replicate the $50-bin midpoint Tax Table + a whole-dollar
rounding mode for the *absolute-return path*, selected by the <$100k threshold; keep the existing
cent-precision formula for the *crypto-delta / what-if path*. Applied independently to QDCGT worksheet
lines 22 and 24. A filed-return product must match the IRS's official number (mismatches trigger notices).

> **RESOLVED by `deep/01` (round 2):** method locked. Two refinements — (1) IRS rounding is **half-up
> (away-from-zero)**, NOT the engine's half-even → a new `round_dollar` is mandatory and distinct from the
> crypto path; (2) **no per-year Tax-Table data is needed** — the official value is
> `round_dollar(ordinary_tax_on(schedule, bin_midpoint))` on the *existing* per-year schedule.
> `line16 = round_dollar(min(L23, L24))`. Detail in §8.

---

## 3. Proposed architecture

```
                 ┌──────────────────────────────────────────────┐
   NEW input →   │ ReturnInputs (per-year side-table, in vault)  │
                 │  household header + PII, dependents           │
                 │  Vec<W2>, Vec<1099-INT>, Vec<1099-DIV>        │
                 │  Schedule A (opt), Schedule 1 basics,         │
                 │  payments (est. tax), carryforward            │
                 └───────────────┬──────────────────────────────┘
                                 │ derive_tax_profile(&tables, year)   (pure fn)
                                 ▼
   FROZEN →       TaxProfile (two scalars) ──▶ compute.rs (crypto DELTA engine, unchanged)
                                 │
   NEW compute →   absolute-liability assembly (reuses ordinary_tax_on / preferential_tax /
                    net_1222 / niit / se + NEW Tax-Table + whole-dollar):
                      AGI → std-vs-itemized → taxable income → QDCGT line 16 →
                      Sch 2 (absolute 8960 NIIT + 8959 Add'l-Medicare) → total tax →
                      payments → refund/owed
                                 │
   EXTEND forms →  btctax-forms: fill FULL 1040 + Sch 1/2/3/A/B  (per-year TOML maps,
                    geometric read-back, DRAFT watermark + attestation gate)
```

**Resolution order** at `report --tax-year` (must be *loud* about which source it used):
`ReturnInputs` (full return) → stored `TaxProfile` (raw override) → pseudo-placeholder → `Missing` blocker.

**Key seams / files** (from recon 04 appendix):
- NEW `btctax-core/src/tax/return_inputs.rs` — the structs + `derive_tax_profile()`.
- NEW absolute-liability assembly (new module in `btctax-core/src/tax/`) — reuses the primitives.
- NEW per-year **Tax Table** bins + `std_deduction` fields → `btctax-adapters` `TaxTable` (indexed home).
- NEW `btctax-cli/src/return_inputs.rs` side-table (mirror `tax_profile.rs`).
- EDIT `btctax-cli` CLI trees (`income`/`deductions`/`dependents`/`household`/`payments` + TOML import).
- EDIT `btctax-forms` — new schedule fillers; FROZEN: `TaxProfile`, `compute.rs`.

### The biggest engine change inside the new work
**NIIT (Form 8960) and Additional Medicare (Form 8959) go from crypto-deltas to whole-household absolute
computations.** They now need absolute AGI + interest + dividends (8960) and **W-2 box 5 Medicare wages /
box 6 withheld** (8959) — inputs not modeled today. 8959 spans two stages (tax → Sch 2 L11; withholding →
1040 L25c). The rate math (`NIIT_RATE`, `niit_threshold`, `se_addl_medicare_threshold`) is already correct.

---

## 4. Decisions (RESOLVED 2026-07-11)

**v1 = TY2024 only** · Common W-2 household · standard-vs-itemized (Sch A) · Sch B · Sch 1 basics ·
wrapping crypto · **absolute liability** · PDF fill · **no CTC**.
- **D1 → TY2024 first** (TY2025 a fast-follow once IRS finals drop).
- **D2 → full Schedule 1-A support** for the TY2025 follow-on (senior/tips/overtime *with* MAGI phase-outs).
- **D3 → CTC/ODC out of v1** — capture dependents only; Schedule 8812 is the #1 follow-on cycle.

Original fork analysis (kept for rationale):

| # | Decision | Options | Recon recommendation |
|---|---|---|---|
| **D1** | **Tax year(s) for v1** | (A) TY2024 first, TY2025 fast-follow; (B) both together | **A** — TY2024 is final & fully verifiable now; TY2025 is an OBBBA moving target (renumbered 1040, +58 fields, and the **2025 Schedule A final form isn't even published** — recon could only get the 2026 rev). Locking TY2024 first de-risks the spec. |
| **D2** | **TY2025 OBBBA new deductions** (Schedule 1-A: senior $6k / tips / overtime / car-loan → new 1040 L13b) — *only if TY2025 is in scope* | (i) full support (phase-outs); (ii) capture + manual L13b override; (iii) skip | Senior + tips are **common in a W-2 household**; recommend **(ii)** for the first 2025 cut, (i) later. |
| **D3** | **Child Tax Credit / Credit for Other Dependents** (Schedule 8812) | (A) out of v1 (capture dependents only); (B) include | You chose "Common W-2 household," which *excluded* credits — but CTC is near-universal for households with kids. It's a **credit** (slots in cleanly after taxable income; doesn't touch the deduction math). Recommend **A for v1, first follow-on** — but worth confirming. |

Adopted without asking (sound defaults, flagged for the record): **Tax Table binning** = replicate (§2);
**QBI/§199A** (1099-DIV box 5 REIT dividends) = capture box 5 + a manual `qbi_deduction_override`, defer
auto-compute; **Schedule D Tax Worksheet** (28%/§1250) = out of scope with a fail-closed refuse-guard
(Bitcoin generates neither).

---

## 5. Rough build phasing (for the plan phase — not final)

0. **Layer 0 method fix** — `round_dollar` (half-up) in `conventions.rs` + a year-independent
   `tax/method.rs` (Tax-Table vs TCW selection + the QDCGT `min(L23,L24)`), reusing
   `ordinary_tax_on`/`preferential_tax`. **No per-year Tax-Table data** (deep/01). Delta path + `TaxTable`
   untouched.
1. **`ReturnInputs` data model + side-table + CLI/TOML entry** (additive; engine frozen).
2. **`derive_tax_profile()` + absolute-liability assembly** — AGI, std-vs-itemized, taxable income, tax.
3. **Standard deduction + Schedule A engine** — bundle std-deduction amounts (basic + §63(f) aged/blind +
   dependent floor) in per-year `TaxTable`; medical 7.5% floor; SALT cap (year-keyed); mortgage interest
   (user-input); **new pure `tax/charitable.rs`** §170(b) ceiling ordering (60→50→30→20; **ST crypto = 50%,
   not 60%**) + class-tagged 5-yr carryover (crypto 8283 plugs in by holding-period → ceiling class). [deep/04]
4. **Absolute NIIT (8960) + Additional Medicare (8959)** — whole-household; **rebuild NII from line-items**
   (not the engine's `nii_with`); `W2` **owner-tag** for per-earner SS wages; Sch 2 L4 = SS+Medicare only
   (route the 0.9% to 8959→L11); **reduce-to-delta invariant** KAT. Wire to Schedule 2. [deep/02]
5. **PDF fillers** — full 1040 + Sch 1/2/3/A/B, per-year maps, geometric read-back, DRAFT/attestation gate.
6. **Follow-on cycles (post-v1):** (a) TY2025 form set + OBBBA tables + **full Schedule 1-A** (senior/tips/
   overtime phase-outs, per D2); (b) **CTC/ODC — Schedule 8812** (#1 follow-on, per D3).

Each phase is TDD + the §2 review-to-green loop per `STANDARD_WORKFLOW.md`.

---

## 6. Verification & legal posture (recon 05)

- **Layered tests:** Layer 0 method fix → Layer 1 per-worksheet KATs (QDCGT, std deduction, Sch A limits,
  Tax Table bins — each asserting against the cited IRS line) → Layer 2 synthetic end-to-end golden returns
  diffed vs an **independent** oracle → Layer 3 **IRS ATS scenarios** (public-domain, IRS-authored filled
  returns; Scenario 2 TY2024 MFJ confirmed usable — extract `/V` fields with `lopdf`) + instruction worked
  examples → Layer 4 golden-PDF SHA-256 hashing (already in place, extend).
- **Placement ≠ arithmetic:** the existing geometric read-back proves a value landed in the right cell, not
  that it's *right*. The numeric-correctness layer above is separate and mandatory.
- **Legal:** ship a **versioned LIMITATIONS / supported-forms doc** (mirror IRS Direct File eligibility +
  Free File Fillable Forms limitations). **Force DRAFT watermark + attestation on every full return**
  (higher stakes than one 8949). **Fail closed** on any unmodeled in-scope line. Stay **mechanical** (UPL):
  compute what the forms dictate from user inputs; never recommend positions or pick filing status. **Leave
  the Paid Preparer/PTIN block blank** (self-prepared). "Not tax advice."
- **License:** OpenTaxSolver + HabuTax are **GPL-2.0** → architecture lessons only; **clean-room re-derive**
  all math from the public-domain IRS instructions. Never copy their code/comments/tables.

---

## 7. For the Fable second pass to confirm (open flags)

- **2025 final forms** — pull the *final* 2025 Schedule A (recon only got the 2026 rev) to confirm 2025
  keeps the 2024 charitable/other-itemized structure (the 2026 rev adds a 0.5%-AGI charitable floor,
  gambling limit, and a §68-style itemized cap — those are **TY2026**, not 2025).
- **OBBBA dollar amounts** — confirm the exact statutory std-deduction ($15,750/$31,500/$23,625) and SALT
  ($40k / 30% phase-down >$500k) figures against OBBBA engrossed text / final 2025 Pub 501 + Schedule A
  instructions before bundling.
- **Root container FQN per (form, year)** — ✅ **TY2024 DONE** (deep/03: 1040/Sch3/A/B = `topmostSubform[0]`,
  Sch1/Sch2 = `form1[0]`); Fable to capture the **TY2025** prefixes (they flip again). Geometric read-back
  stays the fail-closed net.
- **Derivation traps to regression-test** — ✅ **locked** (deep/02): strip the pref slice once; box 2a shares
  the `other_net_capital_gain` channel; `magi = AGI` **proven exact** (8960 L13) with a fail-closed on
  §911/CFC/PFIC. Fable: adversarially KAT the reduce-to-delta invariant.
- **Deeper coverage** — Fable can go where this pass deliberately stayed shallow (per user's "not 100%
  thorough" steer): full Schedule 1-A phase-out math, QBI/§199A auto-compute, CTC/8812, excess-SS credit.

---

## 8. Round-2 opus deep-dive addendum (2026-07-11)

Four v1-gating deep-dives ran (reports in `deep/`). They confirm the first pass is directionally correct
and lock four areas to spec grade — with **six corrections/refinements (three dollar-affecting, marked [$])**.

**Corrections to the first pass (loud):**
1. **[$] Short-term donated crypto's charitable ceiling is 50%, not 60%.** 60% is **cash-only**
   (§170(b)(1)(G)); ST crypto (reduced to basis by §170(e)) is ordinary-income property → **50%**
   (§170(b)(1)(A)). Full 6-class ceiling table + ordering **60→50→30→20** (the 30%-capgain class also capped
   at `50%·AGI − cash`) + **class-tagged** 5-yr carryover. Cite **Rev. Proc. 2023-34 §3.15**. [deep/04]
2. **[$] IRS rounding is HALF-UP (away-from-zero), not half-even.** "$2.50 → $3." Reusing `round_cents`
   (`MidpointNearestEven`) mis-rounds real Tax-Table cells (proven). New **`round_dollar`
   (`MidpointAwayFromZero`)** in `conventions.rs`, distinct from the crypto cent path. [deep/01]
3. **[$ risk] Absolute NIIT ≠ the engine's `nii_with`.** The delta engine's NII omits whole-household
   interest + non-qualified dividends (they cancel in the delta), so the absolute **Form 8960 must rebuild
   NII from line-items**; only MAGI (=AGI) is reusable. `magi = AGI` **proven exact** (8960 L13); fail-closed
   on any §911/CFC/PFIC input. [deep/02]
4. **No per-year Tax-Table data needed** (supersedes §5 phase-0). The bin structure is a *year-independent*
   rule; value = `round_dollar(ordinary_tax_on(schedule, bin_midpoint))` on the existing per-year schedule.
   (Standard-deduction **amounts** are still indexed and still bundled in `TaxTable` — don't conflate them.) [deep/01]
5. **`W2` needs a taxpayer/spouse owner tag.** Box-5 Medicare wages aggregate household-wide (8959 Part II)
   but box-3 SS wages are **per-earner** (§1402(b), SE cap) — different aggregation. [deep/02]
6. **SE double-count trap.** `se.rs.total` bundles the 0.9% Additional Medicare; **Schedule 2 line 4 must
   use SS+Medicare only**, routing the 0.9% to Form 8959 → Sch 2 line 11. [deep/02]

**Newly locked (spec-grade) — new modules the plan will add:**
- **`tax/method.rs`** (year-independent) — `tax_table`, `tax_on_amount`, `qdcgt_line16`;
  `line16 = round_dollar(min(L23,L24))`, L22/L24 each pick Table(<$100k)/TCW(≥$100k) on their own amount,
  `PrefSplit` = QDCGT L6–21, cents carried and rounded once. 3 cent-exact worked examples verified. Delta
  path + `TaxTable` untouched. [deep/01]
- **`tax/charitable.rs`** (pure) — §170(b) ceiling ordering + class-tagged carryover; crypto donations flow
  from the ledger's computed §170(e) deduction (holding-period → ceiling class), never re-typed; runs after
  AGI, before taxable income. [deep/04]
- **Standard deduction** — basic + §63(f) aged/blind (married $1,550 / unmarried $1,950 per box, ≤4 joint) +
  dependent floor `min(regular, max($1,300, earned+$450))`, bundled in per-year `TaxTable`; MFS §63(c)(6)
  coupling + §63(e) election as flags. [deep/04]
- **Absolute 8960 / 8959** — new assembly-layer code reusing the statutory primitives, with a
  **reduce-to-delta invariant** KAT (must collapse to the engine's crypto-delta when non-crypto inputs are
  0). 8959 Part II already lives in `se.rs`; Part I/IV/V net-new. Worked MFJ: absolute NIIT **$2,242** vs
  crypto-delta **$1,596** (different questions, both right). [deep/02]
- **TY2024 field maps captured** (205 real field names) — six root FQNs (1040/Sch3/A/B `topmostSubform[0]`;
  **Sch1/Sch2 `form1[0]`**); no `/TU` tooltips (geometry-only mapping); filing status = 5 independent
  checkboxes (leaf `c1_3` reused, FQN disambiguates); 1040 per-line container boundaries (L4a–11 in
  `Line4a-11_ReadOrder[0]`); Sch B grids asymmetric (14 interest / 15 dividend rows); Sch 2 two-page split
  (`f1_`/`f2_`, L21 = `f2_25`). Starting skeleton for the `btctax-forms` TOML maps. [deep/03]
