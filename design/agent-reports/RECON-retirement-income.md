# RECON — retirement income (1040 lines 4a–6b): is the spec still right, what does it miss, what does it cost?

**Agent:** opus, read-only. **Worktree:** `.claude/worktrees/agent-a06cc89a2ebfcbdc8`, HEAD `45e4c5b16`.
**Brief:** `design/agent-reports/BRIEF-retirement-income-recon.md`. **Date:** 2026-09-13.
**Nothing was committed; no source file was changed.**

**Bottom line.** The spec is **sound where it quotes the form and stale where it quotes the code**. Its
transcription is byte-accurate — every extract citation I machine-checked verifies verbatim — but every
*code* citation has drifted (most by 1000+ lines) and **two are now wrong in substance**, one of them in
the tax-overstating direction. Meanwhile the ground moved under it in the filer's favour: the gap the spec
was written to close is now **refused twice over**, not silently zero, and the build already has a named
slot (**T14**) which the owner **closed as NOT NEEDED** on 2026-09-07.

---

## 0. Stop-and-report items — three of the brief's premises do not hold

The brief's stop condition was *"if the spec isn't at that path, or doesn't cover 4a–6b, or the probe
counts are wrong."* The spec **is** at that path and **does** cover 4a–6b (572 lines; §6 transcribes
4a/4b/4c/5a/5b/5c/6a/6b/6c/6d and line 9), so I did not stop. But three premises are wrong and one of
them matters.

### 0.1 The probe table does not reproduce (the `core` column does; `cli`/`forms` do not)

Six methods tried. Only case-insensitive line counts reproduce the `core` column:

| probe | brief | `grep -rIn` | `rg -c` | `grep -rIin` (matches brief's core) | files (`-l`) |
|---|---|---|---|---|---|
| `1099-R` | 12 / 4 / 1 | 10 / 5 / 0 | 10 / 5 / 0 | **12** / 7 / 2 | 4 / 3 / 0 |
| `pension` | 8 / 0 / 0 | 7 / 2 / 1 | 7 / 2 / 1 | **8** / 2 / 6 | 3 / 1 / 1 |
| `annuity` | 7 / 0 / 0 | 6 / 2 / 1 | 6 / 2 / 1 | **7** / 2 / 1 | 3 / 1 / 1 |
| `social_security` | 23 / 0 / 0 | **23** / 0 / 0 | **23** / 0 / 0 | **23** / 0 / 0 | 5 / 0 / 0 |
| `f8606` | 0 / 0 / 0 | **0 / 0 / 0** | **0 / 0 / 0** | **0 / 0 / 0** | 0 / 0 / 0 |
| `RMD` | 0 / 0 / 0 | **0 / 0 / 0** | **0 / 0 / 0** | **0 / 0 / 0** | 0 / 0 / 0 |

The two load-bearing zeros hold under every method. The `cli` and `forms` columns reproduce under none —
they read as typed rather than measured, and `pension` in particular is 6 in `forms` (not 0): the TY2024
AcroForm maps carry pension reasons.

### 0.2 ★ `social_security = 23` is a FALSE POSITIVE — there is no §86 code at all

All 23 core hits are **`excess_social_security`** — the Schedule 3 excess-*withholding* credit
(`return_1040.rs:1074`), which has nothing to do with lines 6a/6b benefits. The probe that looked like the
best-covered area is the one with **zero** relevant code. This is the shape CLAUDE.md warns about: an
instrument reporting something other than what it measured.

### 0.3 ★★ "Nothing names 1099-R income" is REFUTED — and this answers the brief's ★ question

The brief asked: refused, partially modelled, or silently zero? **Refused — twice over, and structurally.**

1. **A dedicated document-census row per family**, built by the interview arc (T3, merged 2026-09-11):
   `DocumentRow::R1099` and `DocumentRow::Ssa1099` (`crates/btctax-core/src/tax/document_census.rs:69,71`),
   each a mandatory tri-state on `ReturnInputs.documents` (`r_1099`, `ssa_1099`), each asked with a
   prompt citing the 1040's own instructions (`:396`, `:401`), each carrying a **verbatim §2.2 exit
   sentence** (`:179`, `:183`) that names this very spec's task:
   > *"btctax cannot take a Form 1099-R for this year: Form 1040 lines 4a–5b and the Simplified Method are
   > not built (T14, on S2). File with a preparer, or wait for T14."*

   `Some(true)` refuses as `RefuseReason::DocumentTypeUnsupported { kind }` (`return_refuse.rs:768`, fired
   `:2256`) — and the firing **walks `DocumentRow::ALL`**, so the refusal is derived, not listed. Held by a
   derived kill: `every_unsupported_census_row_refuses_with_its_own_exit_sentence` (`return_refuse.rs:4591`)
   iterates every row and asserts the sentence appears verbatim. `None` blocks commit; `row_is_live` is
   `true` for both rows unconditionally (`document_census.rs:733-738`).

2. **The spec's own OQ-1 has already landed.** The catch-all scope attestation now reads *"a PENSION,
   ANNUITY or IRA DISTRIBUTION (Form 1099-R), SOCIAL SECURITY or railroad retirement benefits (Form
   SSA-1099 or RRB-1099), …"* (`questions.rs:941-946`), guarded by a test asserting each of the five limbs
   `pension`/`annuity`/`1099-r`/`social security`/`ssa-1099` is present (`questions.rs:3747-3761`).

So the understatement path the spec was written against **is closed**. §2 of the spec — *"the only thing
between a pensioner and a filed return that omits their pension is `other_out_of_scope_income`"* — is now
historically true and currently stale.

### 0.4 ★ "The TY2026 1040 extract" does not exist

The brief asks for coverage *"against the TY2026 1040 extract."* There is **no `f1040--2026`** and **no
`i1040gi--2026`** — not as PDF, not as extract, not even as a DRAFT. `design/forms/2026/` holds 24
documents (Schedules 1, 1-A, 2, 3, A, B, C, D, SE, Form 6251, 8949, 8959, 8960, 8995, 8995-A as DRAFTs,
plus W-2 and four 1099s as finals) and the base **Form 1040 is not among them**. `MANIFEST.json` has
`f1040--` for 2024 and 2025 only.

That is decisive for this feature specifically: lines 4a–6d are printed on **page 1 of the 1040 itself**,
and both worksheets live in **i1040gi**. So 100% of this spec's quotes come from the two TY2026 documents
that are missing. Deliverable 1 below is therefore stated against TY2025 (with TY2024 deltas), which is all
the repo can support.

### 0.5 T14 already exists as the build slot, and the owner closed it

`SPEC_interview.md:1227` defines **T14** — *"(iff S2 confirms a 1099-R or SSA-1099) the two screens + the
Simplified Method and Social Security Benefits worksheets — from `design/ty2025/SPEC_retirement_income.md`
after its own one round."* `FOLLOWUPS.md:6542`:
> *"**T14 is CLOSED — not needed.** It was gated on 1099-R *or* SSA-1099 and the owner has ruled out both.
> No retirement screens, no Simplified Method worksheet, no Social Security Benefits worksheet."*

The spec itself is recorded as *"DRAFT r1, parked by D-C"* (`SPEC_interview.md:98`), D-C being *"Finish the
spec to 0C/0I; park the build until after the first filed return"* (`LONG_RANGE_PLAN_filing.md:652`).

★ **Minor, documentation:** `FOLLOWUPS.md` still carries the superseded partial at `:6576` — *"this closes
the SSA-1099 half only. The 1099-R half … is **still unanswered**, and T14 stands or falls on it."* The
later entry (*"I won't have a 1099R this year"*) rules out both halves. Two entries, same day, opposite
conclusions; the stale one reads as an open owner question. Worth one edit.

---

## 1. The spec's coverage, line by line

Against `design/forms/extract/f1040--2025.txt` (lines 74–84, machine-read). **Gross vs taxable is the whole
subtlety, and the spec gets it right by transcribing two instructions rather than reasoning:**

| line | what it is | does the spec COMPUTE it? | note |
|---|---|---|---|
| **4a** | IRA distributions, **gross** | **No — structurally never populated** | Not an omission. *"If the distribution from your IRA is fully taxable, enter the total distribution on line 4b; **don't make an entry on line 4a**"* (`i1040gi--2025.txt:2664-2667`, verified verbatim). v1's only computing branch **is** fully-taxable, so 4a is an `Option<Usd>` that is always `None`, with provenance *"the form instructs a blank here."* |
| **4b** | IRA distributions, **taxable** | **Yes** — Σ 1099-R box 1 over IRA-flagged documents, when the filer declares no Exception applies | ★ Deliberately **not** box 2a: the IRA instructions never mention box 2a (S-6). |
| 4c | three checkboxes (Rollover / QCD / write-in) | no field — **never checked, with a reason** | The boxes exist only to flag Exceptions 1/3/4, all of which refuse. 4c is the form's own list of what v1 refuses. |
| **5a** | Pensions and annuities, **gross** | **Conditionally** — `Some(Σ box 1)` when box 2a < box 1, `None` when fully taxable | Same instruction shape (`:2876-2880`, verified). |
| **5b** | Pensions and annuities, **taxable** | **Yes** — Σ box 2a | Permitted explicitly: *"If your Form 1099-R shows a taxable amount, you can report that amount on line 5b"* (`:2902-2904`, verified). |
| 5c | Rollover / PSO / write-in | no field | Rollover refuses; PSO is advisory A-1. |
| **6a** | Social security benefits, **gross** | **Yes** — Σ SSA-1099/RRB-1099 **box 5** (net), which is worksheet line 1 | ★ box 5, not box 3 — the one trap a careless read would get wrong; M-7 kills it. |
| **6b** | Social security, **taxable** | **Yes** — the Social Security Benefits Worksheet, all 18 lines, transcribed in §5.1 | The only line here with real arithmetic. |
| 6c | lump-sum election checkbox | no field — advisory A-2 | Can only reduce tax ⇒ advise, don't refuse (S-1). |
| **6d** | MFS lived-apart-all-year checkbox | **Yes** — `line6d_checked: bool`, from the R-8 declaration | ★ **TY2025+ only.** TY2024's form has 6a, 6c, then 7 — no 6d (`f1040--2024.txt:68-70`). |
| **9** | total income | **Yes** — gains three operands `+4b +5b +6b` | |

**So: of the three GROSS lines, only 6a is unconditionally computed.** 4a is structurally never populated
and 5a only on the partially-taxable branch. That is correct rather than a gap — but it means the feature
as specced fills **five** money cells (4b, 5a-sometimes, 5b, 6a, 6b) plus one checkbox, not eight.

**What the spec does not cover, by design** (§4.2, each mapped to a refusal): all four IRA *Exceptions*
(rollover, Form 8606 basis/Roth/conversion/recharacterization, QCD, HSA funding distribution); the
Simplified Method and General Rule; a 1099-R with box 2a blank or *"taxable amount not determined"*; the PSO
exclusion; disability pension before minimum retirement age (the instructions send it to line 1h, which
btctax has no field for); lump-sum/Form 4972; the worksheet's four *Exceptions*. Every one is either a
refusal (R-1…R-8) or an advisory (A-1…A-4), classified by **S-1** — refuse when an unanswered branch could
understate, advise when it can only overstate. That classifier is the spec's best feature: it makes the
refusal list *derived* rather than chosen.

---

## 2. The forms it drags in — archived? mapped? refused?

| form / worksheet | archived? | transcribed / mapped? | refused today? |
|---|---|---|---|
| **Form 8606** (nondeductible basis, Roth, conversions) | **No** — absent from `design/forms/*`, absent from `MANIFEST.json`, zero code hits | No | **Yes** — via the `R1099` census row today; via R-1/R-2 after the build |
| **Simplified Method Worksheet** | **Yes** — inside `i1040gi`, both years (`i1040gi--2025.txt:2973` *"Simplified Method Worksheet—Lines 5a and 5b"*, Table 1 at `:3037`, Table 2 at `:3061`) | No | **Yes** — R-3 / R-5 |
| **Social Security Benefits Worksheet** | **Yes** — `i1040gi--2025.txt:3364` *"Social Security Benefits Worksheet—Lines 6a and 6b"*, 18 lines at `:3396-3463` | **In the SPEC only** (§5.1, all 18 lines with verbatim instruction text). **No code.** | **Yes** — via the `Ssa1099` census row |
| **Form 5329** (the RMD / early-distribution penalty) | **No** | No. Named once, as a *reason*: the TY2024 map's line-4a entry says *"btctax models no IRA, matching Schedule 2 line 8 (Form 5329)"* | Indirectly — no RMD path exists (`RMD` = 0 hits everywhere) |
| **Form 4972** (lump-sum ten-year averaging) | **No** — appears only as text inside `f1040--2024.txt`, `f6251--*.txt`, `FIELD_PROVENANCE.md` | No | **Yes** — R-3 / R-4 |
| Pub. 915 / Pub. 939 (the General Rule; the barred-worksheet substitute) | No (publications, not forms) | No | Yes — R-6 / R-7 cite them as the exit |

**The 1040's own cells.** The **TY2024** `f1040.map.toml` already declares all seven as `unmodeled` with a
reason and a named refusal (`crates/btctax-forms/forms/2024/f1040.map.toml:326-332`), e.g. line 6a —
*"Social security benefits (gross) — btctax collects no SSA-1099 and runs no §86 provisional-income
worksheet"*, `covered_by = "RefuseReason::DocumentTypeUnsupported"`. So provenance is **recorded**, which is
the "blank with a determinate reason" state CLAUDE.md demands, not the defect.

★ The **TY2025** map has **no 4a–6d entries at all** (98 lines: `line7a`, the two Digital-Asset boxes, and
T8's dependents grid). Those cells sit on the `UNCENSUSED` register rather than carrying a declared reason —
a real asymmetry between the two years, though for a year that will never be filed.

**No retirement input struct exists:** `grep` for `struct Form1099R`, `struct FormSsa1099`, `Form1099RKind`
across `crates/` returns nothing.

---

## 3. ★ The TY2026 transfer justification

The spec's header says *"written 2026-09-04, against the archived TY2025 finals."* Under the owner's
2026-09-11 ruling that no TY2025 return is ever filed, here is what survives.

### Transfers to TY2026 unchanged

- **S-7 — the four §86 thresholds are statutory and unindexed. Machine-verified, not assumed:**
  `i1040gi--2025.txt:3421,3424,3453` print $32,000 / $25,000 / $12,000 / $9,000 and
  `i1040gi--2024.txt:3391,3394,3424` print the identical four. The spec's instruction *"do not build a
  per-year table"* is right, and is the most valuable single transfer here — §86(c) has never been indexed.
- **S-2 — the two "don't make an entry" instructions**, hence 4a/5a as `Option<Usd>`. Statutory in shape,
  not year-shaped.
- **S-6 — the IRA/pension box-2a asymmetry.** It is the form's asymmetry and has been stable for decades.
  Also the best defect-prevention line in the document: a shared *"taxable amount = box 2a"* helper is wrong
  on the IRA side.
- **S-1, S-3** (one class-(A) declaration per document, YES-conditions enumerated from the form's own
  Exception list), **S-4**, **T-1** (worksheet line 8's third bullet is a JUMP), **T-2** (4a blank by
  instruction and 6b `-0-` by instruction, on one page), the refusal set R-1…R-8, the advisories A-1…A-4,
  and the test matrix M-1…M-11. All doctrine, none year-keyed.
- The 18-line worksheet **structure**.

### Must be re-read from the TY2026 documents — which do not yet exist

- **Every extract line number in the document.** All ~40 point into `i1040gi--2025.txt` / `f1040--2025.txt`.
  TY2026's pagination will differ.
- **The line-9 operand list — already proven to move.** TY2024 prints *"…6b, **7**, and 8"*
  (`f1040--2024.txt:75`); TY2025 prints *"…6b, **7a**, and 8"* (`f1040--2025.txt:84`). S-10 flags this.
- **Worksheet line 3's operand list**, same reason: TY2025 reads *"lines 1z, 2b, 3b, 4b, 5b, 7a, and 8"*
  (`:3400`, verified).
- **★★ Worksheet line 6's operand list against TY2026's Schedule 1.** It reads *"Schedule 1, lines 11
  through 20, and 23 and 25"* — a **range**, so any TY2026 renumbering of Schedule 1 Part II silently
  changes which adjustments it covers. This repo has already been burned by exactly this: Schedule 1-A
  **37 → 43 was a COLLISION, not a renumber** — TY2026 refilled line 37 with the MAGI, and reusing the
  TY2025 cross-reference overstated the AMT base by ≈MAGI (`ROADMAP_STATUS.md:311`). Treat the range as
  hostile until re-read.
- **6d's existence.** New in TY2025; confirm it survives into TY2026 before R-8 depends on it.
- **The AcroForm field names** for all six cells (an `xtask dump-fields` job once the final ships).

### TY2025-only — drop it

- **OQ-2 is dead as written.** It recommends *"both years, since TY2024 is the year btctax can actually
  file"* and prescribes pinning the TY2025 cells. Neither year is filed now. The TY2025 f1040 map work it
  asks for is work for a year that will never be filed — compare `f1040s1a/2025`, ruled permanently
  `Unwired` for the same reason. Restate OQ-2 as a TY2026 question.
- **Every "for all of 2025" / "at any time in 2025" phrase** in the §5.1 transcription is year-stamped text,
  to be re-quoted rather than edited.
- The header's *"against the archived TY2025 finals"* provenance claim.

### ★★ The transfer problem the spec cannot fix, and it is the biggest one

**The line-coverage census is 100% TY2024-quoted.** Every collector in `line_coverage.rs` is built with
`Coverage::quoting("2024")` — 30+ sites, including both income collectors (`cover_form1040lines` at `:2924`,
quoting 2024 at `:2962`; `cover_form1040income` at `:3272`, quoting at `:3287`). So new 4b/5b/6b rows would
land quoting **TY2024**'s text — which exists and is checkable — while the **filed** year is TY2026, whose
form is not archived. (This also explains why both line-9 census rows carry *"…6b, 7, and 8"*: that is the
correct TY2024 wording, **not** a defect.) Nothing in this feature can close that; it is the year-package
table's job. Worth naming in any plan so a reviewer does not read it as this task's bug.

---

## 4. The input-surface delta

### Already built — needs nothing

The **declaration** half is done. `documents.r_1099` and `documents.ssa_1099` exist as mandatory tri-states,
with prompts, liveness, classifier entries (`classifier.rs:296`, `classify_document_census`), TUI rows
(`registries.rs:485` — *"The eighteen census rows, `FORM_QUESTIONS` indices 17..=34"*), and refusals.
**The build's first act is to make two `exit_sentence()` arms return `None`, not to add a question.**

### New, and answerable from paper the filer is holding

| new leaf | printed where on the document? |
|---|---|
| `Form1099R::box1_gross_distribution` | box 1 |
| `box2a_taxable_amount: Option<Usd>` | box 2a (blank is meaningful — routes to R-5) |
| `box2b_taxable_amount_not_determined` / `box2b_total_distribution` | the two box-2b checkboxes |
| `box4_fed_withheld` | box 4 → 1040 line 25b |
| `box7_distribution_codes: String` | box 7, captured verbatim, screened not interpreted |
| `FormSsa1099::box3_benefits_paid` / `box4_benefits_repaid` / `box5_net_benefits` | boxes 3, 4, 5 |
| `mfs_lived_apart_all_year` | the filer's own life; the worksheet's *Before you begin* asks it directly |
| `form_8815_or_adoption_exclusion` | ★ correctly scoped to the **residue** of `has_income_exclusion` (`return_inputs.rs:2471`), which already covers §911/§931/§933 — so only Form 8815 and adoption benefits are newly asked |

### ★ New, and NOT cleanly answerable from a document — the design problems

1. **`Form1099RKind` (IRA vs pension) is asked as a judgment when the form answers it.** The 1099-R prints
   an **IRA/SEP/SIMPLE checkbox** immediately right of box 7, and that box — not the filer's reading — is
   what routes 4a/4b vs 5a/5b. The spec captures box 7's codes verbatim and then asks `kind` separately.
   **Collect the checkbox and derive `kind` from it.** Asking the filer to classify their own document
   invites the one error that silently sends a figure to the wrong line pair, and S-6 makes that error a
   *money* error (4b takes box 1; 5b takes box 2a).

2. **`exception_applies` is one question standing for four unrelated determinations, and none is printed on
   the form.** A rollover is *sometimes* box 7 code G; a QCD is **never** marked (the payer cannot know);
   basis lives on a Form 8606 the filer may not have kept; an HSA funding distribution is an election. So it
   is correctly asked — but a filer who did an **ordinary 60-day rollover or a direct trustee-to-trustee
   transfer**, among the most common retirement events there is, answers *yes* truthfully and **the whole
   return refuses**. That is fail-closed and defensible for v1, and it is also the single biggest usability
   cost in this feature. ★ It deserves the journey walk §10 already calls for, with the owner, before a plan
   freezes — not a review round.

**Exact count:** 2 new transcription structs (11 boxes, +1 recommended), 2 new return-level declarations,
1 per-document declaration, **0 new census rows.**

---

## 5. Build estimate — five independently gateable tasks

**T14.0 must come first.** Not a judgment call — without it the three most important new rows are
unverifiable by construction.

### ★★ T14.0 — make the census able to tell 4b from 5b from 6b (the spec's S-9). MUST BE FIRST.

**Confirmed still open at HEAD, and measured rather than read.** All three lines print the identical two
words, *"Taxable amount"*. I reimplemented `label_precedes`'s bare-letter rule
(`crates/xtask/src/line_coverage_check.rs:335-393`) against the real extract. The three occurrences of
`"b Taxable amount"` in `f1040--2025.txt` are at byte offsets **8977 / 9281 / 9577** (extract lines
**74 / 76 / 78**) — about 300 bytes apart, well inside the function's 700-character run-up window (`:380`):

| a census row labelled… | at line 74 (4b) | at line 76 (5b) | at line 78 (6b) |
|---|---|---|---|
| `4b` | correct | **WRONG — accepted** | **WRONG — accepted** |
| `5b` | rejected | correct | **WRONG — accepted** |
| `6b` | rejected | rejected | correct |

So **a 4b→6b label swap passes today** — the spec's M-1 kill, and the Form 6251 line-33 class again. Note
r7 *did* harden this function — the stem-caption form was removed and a left word boundary added, with the
class measured at 71 accepted misattributions (`:353-370`) — but it hardened two other halves, **not** the
run-up window. The spec's proposed fix is right and my measurement confirms its premise: anchor to the
physical extract row whose first token is the stem label, **one row each** (74, 76, 78). Touches no tax
logic; gate is M-1 alone.

### T14.1 — the two documents, and the two census rows stop refusing

`Form1099R` + `FormSsa1099` as box-named transcription structs; the three declarations; classifier entries;
`decl_tristate!` rows; `exit_sentence()` returning `None` for `R1099`/`Ssa1099`. ★ The gate is already
written and derived: `every_unsupported_census_row_refuses_with_its_own_exit_sentence`
(`return_refuse.rs:4591`) walks `DocumentRow::ALL`, so it adapts for free — and `decl_tristate!` couples to
an **array index**, with an explicit warning that inserting mid-array silently repoints every later question
(`registries.rs:258-266`), so append.

### T14.2 — 4a/4b/5a/5b and line 9, in **both** income chains

★ **There are two, and a plan that names one will silently leave the other disagreeing.** `Form1040Lines`
(`printed.rs:587`) with `cover_form1040lines`, and `Form1040Income` (`printed.rs:725`) with
`cover_form1040income`. Two `total_income`/`adjustments` sites likewise: `derive_tax_profile`
(`return_1040.rs:1615`, adjustments at `:1730`) and `assemble_absolute` (`:2147`, total_income `:2326`,
adjustments `:2356`). This is the *"a figure with no reader"* shape — two chains ⇒ the test is the
comparison. Gate: M-2/M-3/M-4, plus flipping the seven TY2024 map cells from `unmodeled` to mapped.

### ★★ T14.3 — 6a/6b: the Social Security Benefits Worksheet. **S-5 MUST BE REWRITTEN FIRST.**

**This is the one place the spec is now wrong in a direction that moves money.**

S-5 prescribes worksheet line 6 as *"**printed Sch 1 L15 + printed L18**, transcribed as two operands"*, and
justifies it by saying btctax's `adjustments` is `early_wd + half_se + student_loan` (cited
`return_1040.rs:1715`). At HEAD that composition has changed:

    return_1040.rs:2356   let adjustments = early_wd + half_se + student_loan + hsa_deduction_13;

Schedule 1 **line 13 is the HSA deduction**, and worksheet line 6 — *"the total of the amounts from
Schedule 1, lines 11 through 20, and 23 and 25"* (`i1040gi--2025.txt:3403`, verified) — **includes** line 13
and excludes only 21 (student loan) and 22 (reserved). So the spec's two-operand list now **omits the HSA
deduction**: line 6 too small → line 7 (provisional income) too large → more benefits taxable → the filer's
tax **OVERSTATED**. (S-5's own reasoning for *not* taking `adjustments` wholesale remains correct — it still
wrongly includes line 21.)

★★ **And the fix already exists in the tree, with its reasoning written out.** The §221 student-loan MAGI
quotes the **identical sentence** and was corrected for exactly this at `return_1040.rs:2326-2340`:
> *"★★★ AND WITH THE HSA DEDUCTION, which is Schedule 1 LINE 13. … Lines 15 (½-SE) and 18 (early withdrawal)
> were the only two members btctax modelled until T16 added line 13; adding it to `adjustments` and not here
> inflated the MAGI and OVERSTATED the tax. Held by
> `form8889::tests::the_hsa_deduction_is_inside_the_section_221_magi`. ★ It is a BLOCK, not a list: a future
> Schedule 1 lines 11–20 adjustment belongs here the day it is added, and the worksheet's own sentence is
> the rule that says so."*

So **worksheet line 6 and `agi_before_student_loan`'s subtrahend are the same quantity**, derived from the
same sentence, and must share **one named accessor** (the T9 single-accessor pattern) rather than two
hand-typed operand lists. This is the B3 failure shape exactly: the fix is already in the branch and nobody
carried it back, because no one held both lines at once.

Gate: M-5 (assert the **branch taken**, not the dollar — T-1 shows the arithmetic coincides), M-6
(re-pointed: *adding an HSA deduction must move 6b*), M-7, M-8, M-10, M-11, plus both oracles on 6b across
all five filing statuses including the MFS lived-apart / lived-with pair.

### T14.4 — the Simplified Method Worksheet (optional; **its stated blocker is gone**)

The spec refuses it (R-3/R-5) because its line 6 / line 10 is a multi-year carryforward — *"a feature with a
persistence surface and a provenance flag … not a worksheet."* **Both now exist**: `open_next_year` carries
computed carryforwards across the year boundary (`crates/btctax-cli/src/open_next_year.rs:184`, `seed` at
`:449`) and `CarryProvenance::ComputedFromPriorReturn { year }` is the flag (`return_inputs.rs:1551-1560`).
What remains is four collected figures — cost at the annuity starting date, the ASD itself, age at ASD,
months paid — plus Tables 1 and 2 (`i1040gi--2025.txt:3037`, `:3061`). Still the right call to defer, but
**the spec's stated reason is obsolete and should be restated** rather than re-quoted.

**Sequencing:** T14.0 → T14.1 → T14.2 → T14.3 (depends on T14.2 for worksheet line 3's 4b/5b operands) →
T14.4. A journey walk with the owner belongs between T14.1 and T14.2, on the `exception_applies` refusal.

---

## 6. Spec verdict: citation audit

**Extract citations — 100% accurate.** Every one checked verifies verbatim: `2664-2667`, `2876-2880`,
`2902-2906`, `3396-3404`, the four thresholds at `3421/3424/3453` against TY2024's `3391/3394/3424`, and both
worksheet headers (`2973`, `3364`). The transcription discipline held.

**Code citations — all drifted; two wrong in substance.**

| spec cites | at HEAD | verdict |
|---|---|---|
| `printed.rs:501-585` `Form1040Lines` | `:587` | drift; claim *"there is no field"* **still true** (no `line4a/4b/5a/5b/6a/6b` on any 1040 struct) |
| `printed.rs:598-620` `Form1040Income` | `:725` | drift |
| `return_1040.rs:1697-1698` `total_income` | `:2326` (and a second chain at `:1615`) | drift + **a second chain the spec never mentions** |
| `return_1040.rs:1715` `adjustments` | `:2356`, now `+ hsa_deduction_13` | ★★ **wrong in substance — S-5 above** |
| `printed.rs:311-340` `ScheduleALines.line2: Option<Usd>` | **`ScheduleALines` has no `line2`** | ★ **wrong** — the precedent is real, the citation is not; use `Form1040Lines.line19/20/21` or `Schedule1ALines.line4a/4b` (`schedule_1a.rs:121,133`) |
| `line_coverage.rs:2265-2272` line-9 row | `:3033` **and** `:3361` | drift; two rows now (two chains) |
| `return_refuse.rs:248` / `:1273-1278` `IraDeductionClaimed` | `:531` / fired `:3820` | drift; **S-8's coupling still holds** |
| `return_refuse.rs:988-1001` `other_out_of_scope_income` | fired `:3071` | drift |
| `questions.rs:548-556` the prompt | `:941-946` | drift; **and the OQ-1 widening has landed** |
| `questions.rs:583-584` *"cannot answer no to a category they were never shown"* | `:1026-1027` | drift; rule intact |
| `return_inputs.rs:606-611` `hsa_activity` | `:1692` | drift |
| `return_inputs.rs:960-970` `has_income_exclusion` | `:2471` | drift |
| `return_inputs.rs:648-650` `qbi_carryforward_in_provenance` | `:1884` | drift |
| `line_coverage_check.rs:270-320` `label_precedes` | `:335-393` | drift; **S-9 still open (measured, §5)** |
| `advisories.rs:43` `Advisory` | `:43` | exact |
| `forms/2025/f1040.map.toml:1-10` *"a stub carrying only line 7a"* | 98 lines (T8 added the dependents grid) | drift; **still no 4a–6d cells, which is the point** |

For scale: `RefuseReason` now has **128** variants (T12 measured 126) with **no `ALL` const** — the recurring
*"list beside a set that grows"* exposure, unchanged.

---

## 7. What I did not do

No code written, nothing built, no file under `crates/` touched, no census re-run, nothing on EITC, state
returns or e-file. No commit, no push. The one script I wrote lives in the session scratchpad (`s9.py`) and
was used only to produce the §5 table.
