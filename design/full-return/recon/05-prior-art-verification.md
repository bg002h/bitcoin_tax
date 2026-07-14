# Recon 05 — Prior Art, Correctness/Verification Strategy, and Legal Positioning

**Scope:** Recon agent 5 of 5 for expanding `btctax` to fill a COMPLETE US individual
return (Common W-2 household: W-2, 1099-INT/DIV, Schedule A/B/1/2/3,
standard-vs-itemized, TY2024/2025, PDF output only, manual entry, offline).

**Status:** FIRST-PASS recon — solid, not exhaustive. Fable sweeps later. Sources cited
inline; uncertainty flagged explicitly. License hazards called out in bold.

**Bottom line up front:**
- The single highest correctness risk is **not** placement — it is **arithmetic method
  mismatch**: the current engine computes tax by the *exact marginal-formula method at cent
  precision*, but a real 1040 for taxable income **< $100,000** (the common-W-2 case) must
  use the **IRS Tax Table** (binned to $50 rows) and whole-dollar rounding. Diffs against any
  real/reference return will show multi-dollar deltas that are *correct for a crypto delta but
  wrong for an absolute return*. This must be resolved before shipping (see §2).
- **License:** the two best open-source references (OpenTaxSolver, HabuTax) are **GPL-2.0**,
  which is **incompatible** with our MIT OR Unlicense. We may learn architecture and
  re-derive the same IRS-dictated math **from the IRS instructions**, but must **not** copy,
  paste, or closely paraphrase their source, comments, or data tables.

---

## 1. Prior art / reference implementations

### 1.1 OpenTaxSolver (OTS) — GPL-2.0, C — architecture lessons only, DO NOT COPY

**License: GNU GPL v2.0.** Written in C. Hosted on SourceForge/GitHub, author Aston Roberts.
([opentaxsolver.sourceforge.net](https://opentaxsolver.sourceforge.net/),
[openhub.net/p/opentaxsolver](https://openhub.net/p/opentaxsolver)).

> **GPL ↔ MIT/Unlicense incompatibility (hard rule).** GPL-2.0 is a strong copyleft license.
> Incorporating GPL code (or a non-trivial derivative of it) into `btctax` would force the
> *combined work* to be distributed under GPL-2.0 — directly incompatible with our permissive
> `MIT OR Unlicense`. Therefore: **study the architecture and algorithms; never copy code,
> comments, constant tables, or distinctive expression.** The *tax computations themselves* are
> dictated by IRS forms/instructions (public-domain government procedures — see §1.4/§4) and may
> be independently re-implemented **from the IRS source**, not from OTS. Keep a clean-room
> discipline: cite the IRS line/worksheet, not the OTS file.

**Architecture lessons worth borrowing (ideas, not code):**
- **Per-year, per-form decomposition.** OTS ships a fresh package per tax year; the Federal
  forms/schedules are produced by **one main `US-1040` program** that covers 1040 + Schedules
  1–3, A–D and forms 6251/8949/8889, with **Schedule C split into a separate program**
  ([download2025](https://opentaxsolver.sourceforge.net/download2025.html),
  [forms.html](https://opentaxsolver.sourceforge.net/forms.html)). Lesson: **year is a
  first-class axis**; keep the common 1040 core together, split rarely-needed schedules out.
  This matches our existing `tables.rs` "indexed per-year table keyed by (year, FilingStatus)"
  design.
- **Plaintext-in / fill-PDF-out.** OTS uses a neat plaintext input file, computes, then an
  "Auto-Fillout" step writes answers onto the **actual government PDF** — a dual-stage
  *compute → fill* pipeline that mirrors what we already do (core computes, `btctax-forms`
  fills). It also separates a text engine from a GUI front-end
  ([opensource.com writeup](https://opensource.com/article/20/2/open-source-taxes)).
- **"How to add new forms" is a documented extension seam.** Forms are modular units; adding
  one shouldn't require understanding the whole engine. Good target for our form front-end.

### 1.2 HabuTax — GPL-2.0, Python — the single most instructive design

[github.com/habutax/habutax](https://github.com/habutax/habutax). **License: GPL-2.0** — same
copy prohibition as OTS.

**The key architectural idea (borrow the pattern, not the code):**
- Calculations and dependencies are specified **at the level of individual form fields**, and a
  **generic dependency solver** walks the graph and evaluates in the right order. Contributors
  can add a form without understanding the whole system.
- **"Fail loudly"** — it refuses to silently skip an unimplemented line/scenario rather than
  produce a wrong number. This is *exactly* the posture a high-stakes full-return filler needs,
  and it dovetails with btctax's existing `Blocker`/`Severity` gating in
  `crates/btctax-core/src/state.rs`.

**Design lessons for us:**
1. Model the return as a **field-level DAG** (`1040.L11 = 1040.L9 − Sch1.L10`, etc.). This makes
   cross-form flow explicit, testable, and auditable — and lets each line cite its IRS source.
2. **Fail closed on any unmodeled line** rather than defaulting it to 0 — a wrong full return is
   worse than a refusal (per the task's own stakes note).

### 1.3 IRS Direct File — the authoritative "simple return" scope to mirror

Direct File is the government's own line in the sand for what a "simple return" is. Its
**included/excluded scope is a ready-made, defensible feature boundary** for our v1.

**In scope (FS2024→FS2025):** W-2 wage income; SSA/pension/annuity income; interest; **standard
deduction only**; EITC, CTC, Credit for Other Dependents, Child & Dependent Care Credit, Premium
Tax Credit, Saver's Credit; adjustments for student-loan interest, educator expenses, HSA.
**Out of scope:** **itemized deductions**, gig/self-employment/business income, rental income;
wage caps (~$200k single / $250k MFJ)
([Treasury JY2629](https://home.treasury.gov/news/press-releases/jy2629),
[Money](https://money.com/irs-irect-file-eligibility-2025/),
[Eligibility PDF](https://home.treasury.gov/system/files/136/Direct-File-Eligibility-Information.pdf)).

> **Positioning note:** our target ("Common W-2 household … Schedule A/B, standard-vs-itemized")
> deliberately goes **one notch beyond Direct File** by adding **itemized deductions (Schedule A)
> and Schedule B**. That is fine, but it means we cannot simply cite "same scope as Direct File."
> Recommend we publish an explicit **supported-forms/limitations list** modeled on Direct File's
> eligibility page and Free File Fillable Forms' limitations page (below).

### 1.4 IRS Free File Fillable Forms (FFFF) — the closest analog to what we're building

FFFF is *the* closest precedent: **official IRS forms, filled manually, minimal calculation
assistance, "do the math yourself."** It confirms our form-first, manual-entry model is a
recognized pattern — and its published limitations list is a template for ours.

- Covers **100+ forms/schedules**, incl. 1040, Schedules 1/2/3/A/B, up to **50 W-2s**
  ([availability + limitations](https://www.irs.gov/e-file-providers/free-file-fillable-forms-program-limitations-and-available-forms),
  [user guide PDF](https://www.irs.gov/pub/irs-utl/free_file_fillable_forms_user_guide.pdf)).
- **"The forms offer very little calculation assistance; you are expected to do the math
  yourself."** Negative amounts use a leading `-`. No attachments beyond in-program forms.
- **Notable scoping quirks to mirror in our limits doc:** Schedule A line 8b allows only **one**
  person's info; Schedule B line 7b allows **up to five** menu selections; the only 1099 it makes
  you transcribe as a form is **1099-R** (others are entered as withholding). These are concrete
  precedents for how a form-filler bounds overflow/edge fields.

**Lesson:** publish a **"Program limitations and available forms" doc** as a first-class artifact
(man page + `--help` + a shipped LIMITATIONS file). It is both UX and legal cover (§3).

### 1.5 Commercial tools (TurboTax / FreeTaxUSA) — only the applicable slice

All legitimate US tax software shares the **same underlying IRS math**; the differentiation is
the **interview → form-field mapping** layer
([Intuit forms list](https://turbotax.intuit.com/personal-taxes/irs-forms/),
[comparison](https://claimyr.com/government-services/irs/FreeTaxUSA-vs-TurboTax-vs-HR-Block-Which-Tax-Software-Wins-for-2025/2025-04-11)).

Applicable to our offline, form-first, PDF-fill approach:
- **Two layers, cleanly separated:** (a) a *data-collection* layer (interview / structured
  input) and (b) a *form-computation* layer (IRS math → line values → PDF). We already have (b)
  in `btctax-forms`; the full-return work is mostly (a) plus an **income-aggregation + deduction
  engine** feeding a 1040 DAG.
- **FreeTaxUSA's noted weakness — "asks for IRS form numbers without explaining them, assumes tax
  literacy"** — is *acceptable and on-brand* for a CLI tool aimed at a technical user, and lets us
  skip building a hand-holding interview. Our input can be form-line-oriented.
- Do **not** attempt their credit/optimization breadth. Ship Direct-File-ish scope + Schedule A/B.

---

## 2. Correctness / verification strategy (stakes are high)

**Framing:** `btctax-forms::verify` (`crates/btctax-forms/src/verify.rs`) is a **geometric,
map-independent read-back** — it proves a value **landed in the correct PDF cell** and that no
unauthorized field was written. It says **nothing about whether the number is arithmetically
right.** A full return needs a **separate numeric-correctness layer**. Propose four layers.

### Layer 0 (PREREQUISITE) — resolve the arithmetic-method mismatch — TOP RISK

The current core deliberately computes a **marginal crypto *delta*** with the **exact
marginal-bracket formula at cent precision, `ROUND_HALF_EVEN`**, and explicitly documents that it
is **"NOT the IRS binned Tax Tables and NOT whole-dollar rounding"**
(`crates/btctax-core/src/tax/compute.rs`, `ordinary_tax_on` / `preferential_tax`). That choice is
*correct for a delta* but **wrong for an absolute return**, because:

1. **Tax Table requirement.** For **taxable income under $100,000**, the IRS *requires* the
   **Tax Table** (income binned to $50 rows, tax taken at the bin midpoint) — not the formula.
   For **≥ $100,000**, the **Tax Computation Worksheet** (a formula) applies. The
   **Qualified Dividends & Capital Gain Tax Worksheet (QDCGT)** and the **Schedule D Tax
   Worksheet** both call back into the Tax Table / Tax Computation Worksheet for their ordinary
   pieces ([i1040 instructions](https://www.irs.gov/instructions/i1040gi),
   [i1040sd](https://www.irs.gov/instructions/i1040sd)). A common-W-2 household is *usually
   under $100k*, so **the Tax Table path is the default, not the edge case.**
2. **Whole-dollar rounding.** 1040 lines are conventionally whole dollars: "drop under 50¢, round
   50–99¢ up; if you round, round *all* amounts; when summing, add with cents and round only the
   total" ([i1040](https://www.irs.gov/instructions/i1040gi)). Our engine rounds to **cents**.

**Action (Layer 0):** implement (a) the per-year **Tax Table** (or a faithful bin generator) and
**Tax Computation Worksheet**, (b) a **whole-dollar rounding mode** for 1040 line output, and
(c) select method by taxable-income threshold + presence of QD/LTCG. Keep the existing
cent-precision path for the *what-if crypto delta*; add an *absolute-return* path. Without this,
**every golden diff below will show spurious $1–$15 deltas** and we won't know real bugs from
rounding noise.

### Layer 1 — Per-worksheet Known-Answer Tests (KATs)

Small, hand-verified fixtures for each computational unit, extending the existing KAT pattern in
`crates/btctax-forms/tests/kats.rs`:
- **QDCGT worksheet** — the crown jewel; the §1(h) 0/15/20% stacking already lives in
  `preferential_tax`. KAT it *line-by-line against the worksheet's own line numbers* across:
  no QD/LTCG; QD only; LTCG only; income straddling each breakpoint; income both under and over
  $100k (to exercise Tax-Table vs Worksheet). Use the worked example in the 1040 instructions as
  one fixture.
- **Standard deduction** — per year × filing status × the "born before Jan 2, 1960 / blind"
  additions and the **dependent's limited standard deduction** worksheet. (The ATS scenario in
  §2.3 literally prints the $14,600 / $29,200 / $21,900 margin values — good anchors.)
- **Schedule A limits** — SALT **$10,000 cap** ($5,000 MFS), medical **7.5%-of-AGI floor**,
  investment-interest and gifts limits, and the **standard-vs-itemized choice**.
- **Schedule B** — interest/dividend totalization and the >$1,500 trigger.
- **Schedule 1/2/3 roll-ups** and the **1040 line flow** (AGI, taxable income, total tax,
  payments, refund/owed).
- **Tax Table / Tax Computation Worksheet** — KAT bin boundaries (e.g., the $50-bin midpoint
  rule) explicitly.

Each KAT should assert **against the IRS worksheet line, citing the instruction line number** in
the test — so the test doubles as the audit trail.

### Layer 2 — End-to-end golden returns (synthetic households)

Build a handful of **synthetic households** spanning the scope matrix (single vs MFJ; standard vs
itemized; with/without QD & LTCG; under/over $100k; multiple W-2s; child/dependent), run the full
pipeline, and **diff every 1040/Schedule line** against an **independently produced expected
return**. Sources for the "expected" side (independence is the point — don't grade the engine
with the engine):
- **Hand-worked** by a human following the instructions (highest trust, lowest volume).
- **A reference tool** for cross-check — but **only tools whose output we may legally observe as
  a black box.** *Observing another tool's numeric output for comparison is fine; copying its
  code is not.* Prefer FFFF-style manual or the IRS worksheets themselves. If OTS/HabuTax are
  used at all, use them **only as an independent numeric oracle** (run them, compare numbers) —
  **never read their source into our implementation** (keeps the clean-room story intact).

### Layer 3 — GOLDEN FIXTURES from IRS-authored material (assessment)

Three candidate public-domain fixture sources, ranked by usefulness:

1. **IRS worked examples in the 1040 / Pub instructions (BEST first fixtures).** The QDCGT worked
   example, Pub 17 examples, Schedule D examples, etc. Small, authoritative, come with the
   answer, public-domain. Use these to seed Layer 1 KATs.
2. > **★ CORRECTED AT P7 (2026-07-14) — THIS ENTRY IS WRONG. Read the correction before relying on it.**
   >
   > The ATS Scenario 2 PDF is **not a filled return and contains no answer key.** It is a test-case
   > **specification**: the IRS validates your MeF *XML submission* against their system, so the PDF
   > only has to state the FACTS. We fetched it, rendered it and looked at the pages:
   >
   > - The **1040 is BLANK** — watermarked "DRAFT — DO NOT FILE", with lines 1a–15 empty. Only the
   >   identity, filing-status and dependents blocks are filled.
   > - Even **Schedule A is only half-filled**: the IRS entered the INPUT lines (5a = 1,068,
   >   5b = 10,509, 8a = 16,854, 11 = 250, 12 = 735) and left every COMPUTED line blank — 5e (the SALT
   >   cap), 7, 8e, 10, 14 and 17 (total itemized).
   > - There are **zero AcroForm `/V` values** in the file, so the "budget a small fixture-ingest step
   >   to parse the `/V` fields" recommendation below has nothing to parse.
   > - The **form list below is also wrong**. The scenario's own cover page lists 1040, W-2 ×2,
   >   Schedule 1, Schedule A, Schedule C, Schedule EIC, Form 8283, Form 8867 — **not** Schedule D,
   >   8812, 8863, 8995 or 4972.
   >
   > **Consequence:** ATS Scenario 2 cannot serve as a golden return, and the P7 plan task that
   > depended on it ("ingest ATS Scenario 2 with a partial-line diff") is not achievable as written.
   > The plan's own alternative — "or a v1-envelope synthetic golden" — is what P7 shipped: eleven
   > households validated against **two** independent engines (OpenTaxSolver driven directly, and the
   > PSL Tax-Calculator), which is a stronger check than one IRS scenario would have been anyway,
   > because it covers the whole matrix rather than one taxpayer.
   >
   > What the scenario IS still good for, and what we took from it: a realistic, public-domain
   > household SHAPE. Its Schedule A SALT figures ($1,068 state income tax + $10,509 real estate =
   > $11,577, over the $10,000 cap) seeded the `mfj_itemized_salt_over_the_cap` golden, which closed a
   > real hole — **no golden exercised §164(b)(5) at all** until then. See FOLLOWUPS `p7-ats-scenario-2`.
   >
   > *(Amusingly, even the IRS's own scenario would take the standard deduction: its Schedule A totals
   > $28,289 against the $29,200 MFJ standard. ATS tests e-file schema, not whether itemizing wins.)*

   **IRS MeF ATS individual scenarios (VALUABLE, with caveats) — CONFIRMED usable.** These are
   **complete, IRS-authored, filled 1040 test returns**, published as PDFs on the MeF ATS page
   and **in the public domain** (US government work)
   ([MeF ATS](https://www.irs.gov/e-file-providers/modernized-e-file-mef-assurance-testing-system-ats),
   [TY2024 1040 ATS page](https://www.irs.gov/e-file-providers/tax-year-2024-form-1040-series-and-extensions-modernized-e-file-mef-assurance-testing-system-ats-information),
   [example scenario 2 PDF](https://www.irs.gov/pub/irs-efile/1040-mef-ats-scenario-2-10082024.pdf)).
   I fetched and parsed **1040 ATS Scenario 2 (TY2024, "Sean & Joan Jackson," MFJ)** locally: it
   contains **two W-2s (Speedway LLC wages $29,513 / Kroger $9,217 with SS/Medicare/withholding
   boxes), a dependent (DOB 2006), an Identity-Protection PIN, a paid-preparer block with PTIN,
   and the filled 1040 plus Schedule D / Schedule 8812 / Form 8863 / Form 8995 / Form 4972 /
   Form 8283 / Form 8867.** **Assessment:**
   - **Pro:** real, complete, free, IRS-authored end-to-end returns with source docs *and* the
     computed return in one file — exactly a golden-return shape. **PDF-only is not a problem**
     for us: we don't e-file, but the *numbers* are all we need, and the scenario doubles as a
     real fill-target to smoke-test `btctax-forms` placement too.
   - **Con / caveats to flag:** (a) scenarios are engineered to exercise **e-file schema/business
     rules**, so a single scenario typically pulls in **forms outside our scope** (8812/8863/8995/
     4972 above) — we can only consume **scenarios (or sub-parts) that fit the common-W-2 scope**,
     or accept a partial-line diff. (b) The scenario is distributed as a **fillable PDF whose
     computed line values live in AcroForm fields**; `pdftotext` reliably extracts the *drawn*
     W-2/preparer text but **not** all overlay field values — extracting the 1040 computed lines
     needs an AcroForm field reader (we already have `lopdf` in `btctax-forms`). Budget a small
     **fixture-ingest step** (parse the ATS PDF's `/V` field values into an expected-lines table).
     (c) There is **no separate clean "answer key"**; the filled return *is* the key. (d) These
     change yearly and IDs are dummy — fine.
3. **Publication 1436 / Pub 5078 (ATS guidelines).** Pub 1436 is the individual-1040 ATS
   guideline; note that for recent years **"the 1040 scenarios are no longer contained within the
   Test Package"** and now live on the MeF ATS *Updates* webpage as the standalone PDFs in (2)
   ([p1436](https://www.irs.gov/pub/irs-pdf/p1436.pdf), and the MeF ATS page). Treat Pub 1436 as
   the *index/instructions*; the fixtures themselves are the scenario PDFs.

### Layer 4 — Determinism / golden-PDF hashing (already have it — extend it)

`btctax-forms` already pins **SHA-256 of the rendered PDF** (`GOLDEN_F8949_SHA256` in
`tests/kats.rs`; per-form `GOLDEN_2024_*` hashes in `tests/sp3.rs`), with the toolchain
Cargo.lock-pinned (lopdf 0.36.0). **Extend the same discipline to the full 1040/Schedule set** so
byte-level output stays reproducible. Caveat: hashing catches *drift*, not *correctness* — it must
sit **on top of** Layers 0–3, never instead of them.

**Layered plan, one line:** *Layer 0 fixes the method (Tax Table + whole-dollar) → Layer 1 KATs
each worksheet against its IRS line → Layer 2 diffs synthetic end-to-end returns vs an independent
oracle → Layer 3 adds IRS-authored golden fixtures (instruction examples first, ATS scenarios
second) → Layer 4 hashes the PDFs for determinism.*

---

## 3. Legal / positioning posture

We are moving from "one Form 8949" to **filling an entire official return that states an absolute
tax liability.** The liability/UX posture must scale up accordingly.

### 3.1 Core principles (from precedent)
- **Not tax/legal advice; preparer responsibility stays with the user.** Professional tax-software
  agreements universally state the software **does not relieve the user of responsibility for the
  preparation, accuracy, content, and review** of the return, and that they must **not rely on the
  software for advice on appropriate tax treatment**
  ([TaxAct legal notice](https://www.taxact.com/professional/legal-notice),
  [JofA: tax-software liability](https://journalofaccountancy.com/issues/2014/sep/tax-software-risks-201410408.html)).
- **UPL / unauthorized practice.** Tax-prep software historically survives UPL challenge by
  positioning itself as a **tool that executes the user's own inputs and the IRS's own rules**,
  **not** a person rendering professional judgment. Keep the tool **mechanical**: it computes what
  the forms/instructions dictate from user-entered data; it must **not** recommend positions,
  choose filing status for the user, or opine on gray areas
  ([Hastings LJ, accountants & UPL](https://repository.uclawsf.edu/cgi/viewcontent.cgi?article=1782&context=hastings_law_journal)).
  The existing "consult a professional" advisories (e.g., the §170 qualified-appraisal advisory)
  are the right pattern — extend, don't dilute.
- **Disclaimers are necessary but not sufficient.** A boilerplate disclaimer alone doesn't immunize
  ([Kitces](https://www.kitces.com/blog/tax-advice-liability-risk-advisor-tax-planning-value-add-value-strategy-financial-planning-clients/)).
  Pair it with **real guardrails**: scope limits, fail-closed blockers, the DRAFT watermark, and
  the attestation gate.

### 3.2 Recommended guardrail posture (concrete)
- **Ship a published, versioned LIMITATIONS / supported-forms doc** (modeled on Direct File
  eligibility + FFFF limitations, §1.3–1.4). State exactly which forms/lines/credits are supported
  and which are **out of scope**, per tax year. This is both UX and the primary UPL/liability
  shield ("we told you what it does and doesn't do").
- **Force the DRAFT watermark + attestation gate whenever the return is a *full absolute
  liability*** — the stakes note says a wrong full return is worse than a wrong 8949, so the full
  return should default to the **same or stronger** gate than pseudo/incomplete crypto data does
  today. Recommend: **full-return output is always DRAFT** unless the user passes an explicit
  attestation flag affirming they've reviewed every line against the IRS instructions and/or a
  professional. Consider **never** offering a "final/clean" 1040 without that attestation.
- **Fail closed on any unmodeled line** (HabuTax's "fail loudly"): if any in-scope line can't be
  computed from inputs, **block and refuse** rather than emit a plausible-looking wrong number.
- **Data-flow honesty:** where a value is *derived* vs *user-entered*, keep it distinguishable so a
  reviewer can audit; never silently fabricate a line (e.g., don't guess withholding).

### 3.3 What ships, and where
- **On the PDF itself:** DRAFT watermark (existing infra, `crates/btctax-forms/src/watermark.rs`);
  a footer/first-page banner: *"Prepared with btctax — self-prepared draft. Not tax advice. Verify
  every line against IRS instructions before filing. No warranty."* Keep it out of official
  signature/preparer fields (we are **self-prepared**, not a paid preparer — do **not** populate
  the Paid Preparer/PTIN block).
- **In `--help` and man pages** (single-source clap doc-comments per the binary-docs infra):
  a short **NOT-TAX-ADVICE + limitations** stanza on the full-return subcommand, linking to the
  LIMITATIONS doc; state supported years and forms; state that output is DRAFT by default and how
  to attest.
- **In an accuracy/limitations statement** (LIMITATIONS.md + man page): "computes federal only;
  no state; standard *and* Schedule A itemized; W-2 / 1099-INT / 1099-DIV; TY2024–2025; uses IRS
  Tax Table/Tax Computation Worksheet and whole-dollar rounding; **no** self-employment, business,
  rental, or credits beyond the listed set; **no e-file**; you are responsible for review."

---

## 4. The redacted real return

The user offered an **old, redacted real return** to study structure. Two deliverables: a
**learning checklist** and a **redaction guide**. (Reminder: this is *structure* study — it is not
a fixture; treat it as reference, not ground truth.)

### 4.1 (a) CHECKLIST — what to learn from it
1. **Form inventory & assembly order.** Which forms/schedules are present and in what order
   (1040, Sch 1/2/3, Sch A, Sch B, W-2 copies, any 1099-R). Confirms our scope covers a real
   household and reveals anything we're missing.
2. **Inter-form number flow (the DAG).** Trace how a number moves: W-2 box 1 → 1040 line 1a;
   Sch B total interest → 1040 line 2b; Sch B dividends → 1040 line 3b; Sch 1 line 10 → 1040
   line 8; Sch A line 17 → 1040 line 12 (itemized) vs standard; AGI (line 11) → taxable income
   (line 15) → tax (line 16) → Sch 2 → total tax (line 24) → payments → refund/owed. Verify our
   field-level DAG against a real one.
3. **Standard-vs-itemized decision** as actually made, and how the chosen path zeroed the other.
4. **Rounding & formatting conventions in practice.** Whole-dollar vs cents on each line; how
   negatives are shown (parentheses vs leading `-`); blank vs `-0-`; comma grouping; how the
   left-margin standard-deduction reference amounts appear.
5. **Multi-instance & overflow handling.** Multiple W-2s; multiple interest/dividend payers on
   Sch B (the FFFF "up to five" precedent, §1.4); how a payer list too long for the form is
   continued/attached (statement vs continuation) — informs our `overflow.rs` strategy.
6. **Edge/unusual fields actually used.** Filing-status box; dependents grid; digital-asset
   question; "born before 1960 / blind" boxes; presidential campaign checkbox; IP PIN;
   direct-deposit routing/account (which we will likely *omit* or leave blank).
7. **Signature / preparer block treatment** — confirms our decision to leave Paid Preparer blank
   (self-prepared) and where the taxpayer signs.
8. **Any line we do NOT support** appearing on a real return → feeds the LIMITATIONS doc (§3.2).

### 4.2 (b) REDACTION GUIDE — remove/mask before sharing into this session

> **Goal:** preserve the **numeric structure and line-to-line flow** we need, while removing every
> piece of PII/identifier. When in doubt, redact — we need *shapes and amounts*, not identities.
> **Prefer masking over deletion** so field positions/relationships stay legible (e.g., replace a
> name with `REDACTED-NAME`, an amount you'd rather not share with a **plausible placeholder of the
> same magnitude**).

**MUST remove or mask (identity / account / contact):**
- **SSN / ITIN** (taxpayer, spouse, **every dependent**) — mask all digits (`XXX-XX-XXXX`).
- **Names** — taxpayer, spouse, dependents; and any **employer/payer names** if identifying
  (replace with `Employer A`, `Bank B`).
- **Full address** — street, city, ZIP (a real return prints these on 1040 + W-2s). Region/state
  can stay only if you're comfortable; not needed for structure.
- **EINs** of employers/payers (W-2 box b, 1099s) — mask.
- **Account numbers & routing numbers** — direct-deposit block, any 1099 account numbers.
- **Driver's-license / state-ID number** (some state returns / e-file records) — remove.
- **Phone, email**.
- **Preparer identifiers** — **PTIN, preparer name, firm name, firm EIN, firm address/phone**
  (the ATS example even shows a "Young's Tax Service / PTIN / Firm EIN" block — that class of data).
- **Identity Protection PIN (IP PIN)**, e-file **Self-Select PIN**, and any **DCN/submission ID**.
- **Barcodes / QR codes** — they can encode SSN/name; obscure them.
- **Signatures** (image) and **dates of birth** (mask DOB — keep only "born before 1/2/1960 Y/N"
  if a line depends on it).

**KEEP (this is the structure we need):**
- **All dollar amounts on every line** (or same-magnitude placeholders), and **which line each sits
  on** — this is the whole point.
- **Form/schedule identities and line numbers**; checkbox states (filing status, digital-asset Y/N,
  age/blind, dependents' credit-eligibility checkboxes **without** SSNs).
- **Number of W-2s / payers** and their **per-form box amounts** (wages, withholding — *values*, not
  names/EINs).
- **Tax year.** (We support TY2024/2025; an older year is still fine for *structure* study, but tell
  us the year so we map line numbers correctly — line numbering shifts across years.)

**Practical method:** share it **as text/values or a flattened image with boxes blacked out**, not
a live fillable PDF (a fillable PDF can leak redacted values in hidden AcroForm `/V` fields even
when the visible text is covered — mirror of the extraction trick we used in §2.3). If sending a
PDF, **flatten it first** and verify no `/V` field survives.

---

## Appendix — grounding in the current codebase

- `crates/btctax-core/src/tax/compute.rs` — `ordinary_tax_on` (exact marginal formula, cents,
  ROUND_HALF_EVEN) and `preferential_tax` (§1(h) 0/15/20 stacking). **Delta-oriented; needs a
  Tax-Table + whole-dollar absolute-return path (Layer 0).**
- `crates/btctax-core/src/tax/tables.rs` — indexed-per-(year,status) vs statutory-constant
  separation; a good home for the per-year **Tax Table** bins.
- `crates/btctax-core/src/tax/types.rs` — `FilingStatus`, `TaxProfile`, `TaxResult`. **No standard
  deduction / AGI aggregation / Schedule A model yet** — that is the front-end to build.
- `crates/btctax-core/src/state.rs` — `Blocker`/`Severity` gating → reuse for "fail closed on
  unmodeled line."
- `crates/btctax-forms/src/verify.rs` — geometric read-back = **placement only**, not arithmetic.
- `crates/btctax-forms/tests/kats.rs`, `tests/sp3.rs` — golden **SHA-256 PDF** hashing to extend.
- `crates/btctax-forms/src/watermark.rs` — DRAFT watermark infra to force on full returns.

**Sources:**
[OTS home](https://opentaxsolver.sourceforge.net/) ·
[OTS forms](https://opentaxsolver.sourceforge.net/forms.html) ·
[Open Hub OTS](https://openhub.net/p/opentaxsolver) ·
[HabuTax](https://github.com/habutax/habutax) ·
[Direct File / Treasury JY2629](https://home.treasury.gov/news/press-releases/jy2629) ·
[Direct File eligibility PDF](https://home.treasury.gov/system/files/136/Direct-File-Eligibility-Information.pdf) ·
[FFFF limitations & forms](https://www.irs.gov/e-file-providers/free-file-fillable-forms-program-limitations-and-available-forms) ·
[FFFF user guide](https://www.irs.gov/pub/irs-utl/free_file_fillable_forms_user_guide.pdf) ·
[1040 instructions](https://www.irs.gov/instructions/i1040gi) ·
[Schedule D instructions](https://www.irs.gov/instructions/i1040sd) ·
[MeF ATS](https://www.irs.gov/e-file-providers/modernized-e-file-mef-assurance-testing-system-ats) ·
[TY2024 1040 ATS page](https://www.irs.gov/e-file-providers/tax-year-2024-form-1040-series-and-extensions-modernized-e-file-mef-assurance-testing-system-ats-information) ·
[Example ATS scenario 2 PDF](https://www.irs.gov/pub/irs-efile/1040-mef-ats-scenario-2-10082024.pdf) ·
[Pub 1436](https://www.irs.gov/pub/irs-pdf/p1436.pdf) ·
[JofA tax-software liability](https://journalofaccountancy.com/issues/2014/sep/tax-software-risks-201410408.html) ·
[TaxAct legal notice](https://www.taxact.com/professional/legal-notice) ·
[Kitces on advice liability](https://www.kitces.com/blog/tax-advice-liability-risk-advisor-tax-planning-value-add-value-strategy-financial-planning-clients/) ·
[Hastings LJ UPL](https://repository.uclawsf.edu/cgi/viewcontent.cgi?article=1782&context=hastings_law_journal)
