# Whole-branch PRE-PUBLISH review — Fable

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `main..HEAD`, all **24** commits.

**Why Fable, and why here.** House rule (global `CLAUDE.md`): Fable is never the default and is reserved
for **a single review immediately before a first irreversible or costly action**. crates.io versions are
**immutable** — they cannot be unpublished, replaced or corrected — so this was that moment, and the
escalation was deliberately not spent earlier in the session. Owner approved before dispatch.

**The brief's central move: scope it AWAY from what the two earlier rounds covered.** r1 reviewed
`7bde148..65270db`, r2 reviewed `afa0ffe..HEAD`; both returned 0C/0I. **Nobody had read the branch as a
whole**, so Fable was pointed at INTERACTION between commits made days apart — the three new advisories
now rendering together on a second command path, four questions moved class (A)→(B) across three
commits, the cumulative effect on one filer, and the one figure neither oracle can validate. It was told
plainly that "do not publish" is a successful outcome, and given the full settled-facts list (including
§G-19a / §G-20 / §G-22) so it would not rediscover them.

★★★ **It paid for itself: the Important it found lives in `b94508d`, which falls BEFORE r1's range start
and had therefore never been read by any reviewer.** The scoping decision is what surfaced it.

**Reviewer output reproduced VERBATIM below.**

---

`VERDICT: publish-after-fixing-X` — X = gate Schedule B's printed FBAR sub-question pair on `7a == Some(true)`, exactly as this branch already does for Schedule C line J.

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/printed.rs:1014 (constructor) + crates/btctax-forms/src/schedule_b.rs:152-158 (writer)
CLAIM: A stored FBAR sub-question answer is printed whenever it is Some, with no gate on 7a — so a filer who answers 7a Yes, answers the sub-question, then corrects 7a to No signs a Schedule B whose 7a box says "No" while the "If 'Yes,' are you required to file FinCEN Form 114…?" box is checked — a mark in a box the form did not ask, contradicting the answer above it, under §6065.
FAILURE: (1) `income answer`: 7a=Yes, sub-question=Yes; saved (whole-struct JSON, input_form_store.rs:74). (2) `income answer` again — "Every live declaration is asked in ONE pass — including ones already answered" (answer.rs:48) — filer corrects 7a to No; the sub-question is now non-live (questions.rs:760) so it is never re-asked, and the engine's own liveness gate makes it un-clearable (registries.rs:269-271, "a set/clear while 7a is not Yes is NoSuchRow"). `fbar_filing_required=Some(true)` persists. (3) $2,000 of interest keeps Schedule B printing (return_1040.rs:1713-1718). (4) No screen refuses (return_refuse.rs:252 binds it `_`); the one advisory built for this box is nested under `foreign_accounts == Some(true)` (advisories.rs:459-464), so it is silent exactly when the box prints wrongly. (5) `export-irs-pdf` writes 7a=No checked AND the FinCEN-114 sub-box checked. Same via a hand-edited `income import` TOML — the input path LIMITATIONS.md itself directs filers to for `qbi_carryforward_in`.
EVIDENCE: `schedule_b_lines` carries it verbatim — `fbar_filing_required: ri.fbar_filing_required,` (printed.rs:1014) — while the line directly beneath gates the sibling: `line7b_countries: if ri.foreign_accounts == Some(true) {…}` (printed.rs:1016). Three doc comments assert the unenforced invariant: "When 7a is 'No' the pair is likewise left unwritten, because the form does not ask it then" (printed.rs:929); "never even asked because 7a was 'No'" (schedule_b.rs:149). The form conditions the box: "If 'Yes,' are you required to file FinCEN Form 114 …" (design/forms/extract/f1040sb--2024.txt:69). And the branch's OWN later commit closes the identical hole on Schedule C: "an answer to J without a Yes on I is not a mark the form has a place for … belt-and-braces here so a TOML import cannot produce a J-without-I page" (printed.rs:1095-1101).
```

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/qbi.rs:153-154
CLAIM: The `Form8995Lines` struct doc still says "Line 3 (a prior-year trade/business QBI loss carryforward) has no v1 input and stays blank: a Schedule C LOSS refuses upstream, so v1 never carries one forward" — false as of this branch, and it describes the single highest-risk square inch (the line neither oracle can validate) as dead plumbing.
FAILURE: No filed-return failure; the hazard is the r1-finding-5 shape — a future author auditing line 3 against the struct doc concludes the field is vestigial. The field doc 19 lines below says the opposite, which contains the damage.
EVIDENCE: qbi.rs:172-178 ("★★ L3 … This field did not exist … Its absence INFLATED the deduction and UNDERSTATED the tax") directly contradicts the paragraph above it.
```

```
SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/questions.rs:609-611
CLAIM: The `SKIPPABLE_QUESTIONS` doc says "Seven of the eight are BENEFIT CLAIMS … The eighth, FbarFilingRequired, is here for a different reason" — the list is now twelve (correctly stated four lines up), and the Schedule C pair shares FbarFilingRequired's "different reason."
FAILURE: Doc-only; misstates the registry's own taxonomy after two growth commits.
EVIDENCE: The array (questions.rs:613-865) has 12 entries; ScheduleC1099Required/Filed are compliance boxes, not benefit claims (their own docs say "no figure on the return reads them").
```

INTERACTION FINDINGS:

The Important finding is itself the cross-commit one: `b94508d` (the earliest commit, in **neither** review range — r1 started at `7bde148`, r2 at `afa0ffe`) built the FBAR pair without the orphan gate; `de8ffd8`, nine commits later, discovered the right pattern for Schedule C line J and wrote down why — and nobody carried it back. The advisory (`cbe651d`) and the writer disagree about when the box can be non-blank, and each was reviewed only beside its own commit.

Also checked, sound: (1) the three new advisories are trigger-disjoint and cannot double-count — `AgedBoxForfeitedNoDob` (DOB absent) and `AgedBoxForfeitedDeathUnanswered` (DOB present, gate skipped) are per-person exclusive, and both firing on one MFJ return names two different persons correctly; the MFJ 2-box total fix (r2 Nit) survives. (2) Registry sweep across all 12 skippables: every question is askable (DodSpouse needs MFJ+spouse+gate-Yes, all settable in order; line J needs I=Yes, ditto); stale values under changed liveness are safe everywhere *except* the FBAR pair — `is_aged`'s arm order makes a stale date-of-death inert once the gate is answered "No", Schedule C's stale J is dropped by the printed.rs gate, an MFS stale `spouse_died_during_year` is inert because `AgedBlindBoxes` filters to MFJ. No refusal was removed that a compute path still leans on (`ScheduleBPart3Unanswered`, `DependentStatusUnanswered` both stand; `is_aged (None,None)` fails safe). (3) Cumulative one-filer walk, main→HEAD: returns main refused (death gates were class-A and always-live) now compute with the aged box *forgone* plus an advisory — overstate-or-equal, never understate; 8995 line 3 can only raise tax when supplied, the write-back carries `max(carry_in − QBI, 0)` with an atomic user-provenance guard (return_1040.rs:1652-1695); the line-7 box revert is complete (no code writes `c1_23[0]`; the census entry now records the honest reason); the export path renders the identical `advisories_for` list (main.rs:812). (4) I independently re-derived 8995 lines 3/4/16 from `f8995--2024.txt:37-55` — the combine-means-subtract reading is right, the emitter writes line 3 as a magnitude, gates it blank-when-zero, and `assert_paren_magnitudes` now covers it with a planted-defect test. (5) Noted and accepted: 7b's own text conditions the country list on being *required to file FinCEN 114* (f1040sb--2024.txt:73), while btctax gates it on 7a alone — pre-existing gate, surplus true disclosure, outside this diff.

THE ONE THING I WOULD FIX BEFORE PUBLISHING: gate the FBAR pair at the single constructor seam — `fbar_filing_required: if ri.foreign_accounts == Some(true) { ri.fbar_filing_required } else { None }` in `schedule_b_lines` (printed.rs:1014), mirroring `line7b_countries` beneath it and Schedule C's line J — with a B1 planted-orphan test (7a=No + stored `Some(true)` ⇒ unwritten pair) observed red first.

WHAT WOULD MAKE THIS REVIEW WRONG: it assumes a stored `fbar_filing_required` genuinely survives a 7a Yes→No correction into the persisted inputs — I traced the answer flow, the registry's NoSuchRow clear-refusal, and the whole-struct serde save, but did not execute the two-pass `income answer` sequence end-to-end; if some path I did not find re-derives or projects the struct through live fields on save, the orphan dies before the writer and the Important finding collapses to the doc-vs-code inconsistency alone.

---

## Disposition (author, same day)

**All three folded. The Important was CONFIRMED before fixing, not taken on trust.**

The reviewer's own stated uncertainty — whether the orphan survives a two-pass `income answer` — turns
out **not to matter**, because the state is reachable by a second, simpler route it also named: a
hand-edited `income import` TOML. `LIMITATIONS.md` now actively directs filers to hand-edit TOML for
`qbi_carryforward_in`, so that path is not hypothetical. I confirmed the defect by writing the failing
test against `schedule_b_lines` directly and **watching it go red** (`left: Some(true), right: None`)
before touching the fix.

| finding | disposition |
|---|---|
| **Important** — the orphaned FBAR answer | **FIXED** at the constructor seam exactly as recommended, mirroring `line7b_countries` one line below and Schedule C's line J. Held by `an_fbar_answer_orphaned_by_a_7a_correction_is_not_printed`, observed RED first. |
| Minor — stale `Form8995Lines` struct doc | **FIXED.** Corrected rather than deleted: line 3 is the one figure neither oracle can validate, so a struct doc calling it dead plumbing is precisely how a future audit concludes the field is vestigial. Now points at §G-22 (it is asked of nobody yet). |
| Nit — `SKIPPABLE_QUESTIONS` taxonomy | **FIXED.** Rewritten as two named species — benefit claims, and compliance boxes no figure reads — since "class (B) == forgone benefit" is the reading a future question would get wrong. |

### ★★★ The lesson, and it is about REVIEW SCOPE, not about the FBAR

Three independent reviews ran on this branch. **The Important defect sat in the earliest commit, and
both earlier rounds were scoped to ranges that excluded it** — r1 began at `7bde148`, r2 at `afa0ffe`,
and `b94508d` precedes both. Each round was diligent inside its window and the defect lived outside
every window.

**A per-range review is not a branch review, and a stack of them does not become one.** The fix that
would have caught it existed *in the branch already* — Schedule C's line J, written nine commits later
with the reasoning spelled out — and nobody carried it back, because no reviewer ever held both commits
at once. When a branch grows over days, the last pass must be scoped to the WHOLE of it, and pointed
explicitly at interaction rather than at correctness-per-commit.
