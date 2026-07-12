# Fable Recon — Second Pass (seed / re-runnable brief)

**How to run this** (paste into a Claude Code session in this repo when Fable usage is available):

> Execute the Fable recon second pass: read `design/full-return/recon/FABLE_RECON.md` and follow it exactly.

This is the **second** recon sweep for the **full-return expansion** (btctax → complete US 1040). The
**opus first pass is already done** and lives in this directory:
`00-SYNTHESIS.md` (decision doc) + `01-form-graph.md`, `02-computation-worksheets.md`,
`03-irs-fillable-pdfs.md`, `04-input-data-model.md`, `05-prior-art-verification.md`.

**Locked scope (do not re-litigate):** v1 = **TY2024**, "Common W-2 household" (W-2 + 1099-INT/-DIV,
standard-vs-itemized / Schedule A, Schedule B, Schedule 1 basics), wrapping the existing crypto
8949/Schedule D engine, **absolute liability**, **PDF output only**, **manual/structured entry**, **no
CTC in v1**. Follow-ons: **TY2025 with FULL Schedule 1-A** (senior/tips/overtime phase-outs) and
**CTC/Schedule 8812**.

**Mandate for this pass:** the opus pass was deliberately "solid but not exhaustive." Your job is to
**VERIFY and DEEPEN** — confirm the opus findings against primary IRS sources, resolve every open flag in
`00-SYNTHESIS.md` §7, and produce **spec-grade** detail (exact numbers, worksheet line math, field maps)
for the deferred/uncertain items. **Do NOT redo settled findings from scratch** — cite the opus report,
then confirm or correct it. Flag any correction to an opus claim loudly.

---

## Orchestration instructions (for the executing assistant)

Fan out the agents below **in parallel**, each with `model: "fable"` (this pass is explicitly Fable per
the user; approval already given). Each agent:
- reads the relevant opus report(s) first and treats them as prior findings to verify, not gospel;
- cites **primary IRS sources** (forms, instructions, Pubs, Rev. Procs., OBBBA/Pub. L. 119-21 engrossed
  text) and prefers reading the actual PDFs;
- writes a full report to `design/full-return/recon/fable/NN-<name>.md`;
- returns a SHORT (≤300-word) summary noting especially any **correction** to an opus claim.

After all complete, **synthesize** into `design/full-return/recon/fable/00-SYNTHESIS-FABLE.md`: a
reconciliation that lists (a) confirmed opus findings, (b) corrections, (c) newly-resolved flags, (d) any
remaining unknowns for the spec. Then report the reconciliation to the user. `mkdir -p
design/full-return/recon/fable` first.

---

## Agent F1 — TY2025 final forms + OBBBA statutory figures (highest-value verification)
Resolve the two biggest opus flags. (1) Pull the **final** (or latest) 2025 **Schedule A** and confirm it
KEEPS the 2024 charitable/other-itemized structure — the opus pass could only get the **2026** rev, which
adds a 0.5%-AGI charitable floor, gambling-loss limit, and a §68-style itemized cap; confirm those are
**TY2026, not TY2025**. (2) Verify the exact OBBBA dollar amounts against **Pub. L. 119-21 engrossed text
+ final 2025 IRS instructions / Pub 501**: standard deduction **$15,750 / $31,500 / $23,625**; SALT cap
**$40,000 / $20,000 MFS, 30% phase-down over $500k / $250k MAGI, floor $10k**; enhanced senior deduction
**$6,000** (§70103) phase-out. Also re-verify the **2025 Form 1040 page-2 renumbering** (L7a, L11a/L11b,
L12e, L13a, L13b) and **Schedule 1-A** part/line structure on the latest draft. Output: a per-figure
"confirmed / corrected" table with citations, and a go/no-go on bundling TY2025 tables.

## Agent F2 — QDCGT worksheet + Tax Table method (spec-grade math lock)
Deep-verify the opus §4 claim that `preferential_tax`/`PrefSplit` maps EXACTLY onto QDCGT worksheet lines
6–21 and that line 16 = `min(ordinary_tax_on(B)+PrefSplit.tax, ordinary_tax_on(TI))`, **including** the
line-24 comparison and the `min(L23,L24)` — construct 2–3 worked numeric examples (straddling each
breakpoint, under/over $100k) and confirm to the cent. Then nail the **"Layer 0" Tax Table method**: the
exact **$50-bin midpoint** rule, the **sub-$3,000 / sub-$25** small-bin rules, and the IRS **whole-dollar
rounding** convention ("round the total, not each item"), with citations to the 2024 1040 instructions.
Deliver an unambiguous rounding/method spec the implementer can code against.

## Agent F3 — Deferred follow-on math: full Schedule 1-A, QBI/§199A, CTC/8812
Produce spec-grade math for the items the opus pass deferred (needed for the TY2025 + CTC follow-ons, and
to de-risk v1 data capture): (1) **Schedule 1-A** full phase-out math for Parts II/III/IV/V (tips $25k;
overtime $12,500/$25k MFJ; car-loan $10k; senior $6k) with their MAGI phase-out formulas and the Part I
MAGI definition. (2) **QBI / §199A** — Form 8995 simplified path (20% of REIT/§199A dividends from
1099-DIV box 5, taxable-income limit) and the 8995 vs 8995-A threshold. (3) **CTC/ODC — Schedule 8812**
($2,000/child <17 → $2,200 for 2025; $500 ODC; MAGI phase-out; ACTC refundable portion). Citations to
each form's instructions.

## Agent F4 — Derivation correctness + absolute NIIT/Add'l-Medicare
Deep-verify the `derive_tax_profile()` logic (opus report 04 §5): the **1099-DIV box 1a ⊃ 1b** strip-once
rule, the **box 2a → `other_net_capital_gain`** channel coupling with crypto Schedule D netting, and that
**`magi_excluding_crypto` = AGI** is correct precisely because tax-exempt interest is NOT a §1411 add-back
(confirm against Form 8960 instructions). Then lock the **absolute Form 8960 (NIIT)** and **Form 8959
(Additional Medicare)** whole-household computations: Part-I NII assembly (8960), the wages side of 8959
(**W-2 box 5 Medicare wages, box 6 withheld**), the two-stage 8959 split (tax → Sch 2 L11; withholding →
1040 L25c), and threshold coordination with SE income. Output: a worked MFJ example end-to-end.

## Agent F5 — Per-(form, TY2024) field-map skeletons + root FQN capture
Extend opus report 03: for **each** of Form 1040, Schedule 1, 2, 3, A, B for **TY2024**, download the
official PDF and capture (a) the **exact root container FQN** (`topmostSubform[0]` vs `form1[0]` — it
flips per form), and (b) a concrete **field-map skeleton** listing the money/checkbox leaf field names
(`f1_N[0]`/`c1_N[0]`) for the in-scope lines, keyed to the 1040 line numbers from report 01. Note Schedule
B overflow row capacity and the geometric-read-back cases flagged for Sch A (two amount columns) and Sch 2
(two-page, code boxes). This becomes the starting point for the `btctax-forms` TOML maps.

## Agent F6 — Completeness / adversarial critic
Read all opus reports + F1–F5 outputs. Ask: what in-scope line, worksheet, edge case, or data field is
still unaccounted for? Probe specifically — multi-W-2 excess-SS credit (box 4), the MFS-both-itemize
coupling, negative-number / loss-year formatting on the PDFs, the "which source produced the numbers"
precedence risk (two sources of truth for one year), and any 1040 line the assembly can't currently
produce. Output a prioritized gap list for the spec.
