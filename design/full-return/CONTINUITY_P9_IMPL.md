# CONTINUITY — P9 IMPLEMENTATION (the FORM QUESTION REGISTRY)

*Written 2026-07-14 at a deliberate context boundary. Branch `main`, HEAD `db04781`, tree CLEAN,
pushed to `origin/main`. `make check` GREEN (1736 passed, 1 skipped).*

---

## ⚡ THE ONE COMMAND TO RESUME

```
Read design/full-return/CONTINUITY_P9_IMPL.md and continue the P9 implementation from step 6.
```

---

## 1. Where we are, in one paragraph

The **P9 spec is GREEN** (`design/SPEC_form_questions.md`, r9, 0C/0I — 8 Fable reviews persisted at
`design/full-return/reviews/P9-SPEC-fable-r{1..8}.md`). We are **implementing the spec's §5 build order**,
TDD, **one step per commit**, each red→green, **mutation-checking every guard**
([[untested-guard-pattern]]). **Steps 1–5 are DONE, committed, and pushed. Resume at step 6.**

## 2. What P9 IS (do not re-derive)

The answered-ness invariant — *"an input with no safe default must be answered before it reaches compute or
print"* — was btctax's one load-bearing invariant held by **convention, not construction**. P9 makes it
structural via a `FORM_QUESTIONS` **registry** that `screen_inputs`, `income answer`, and
`ReturnHeader::build` all **derive from** (one liveness predicate, never written twice). See
[[answeredness-invariant]]. The class is **anything that can silently answer for the filer** — a value, a
**liveness predicate** (the §2.9 shipped bug), or a **mis-scoped question** (the HSA Critical).

## 3. Steps DONE (committed on `main`)

| step | commit | what |
|---|---|---|
| 1 | `f8c1f4c` | fields: `blind`/`salt_use_sales_tax`→`Option<bool>`; `hsa_present`→`hsa_activity: Option<bool>` (RENAME); NEW `dual_status_alien`, `mortgage_all_used_to_buy_build_improve`; `RefuseReason::HsaPresent`→`HsaActivityUnsupported` |
| 2 | `a3ec138` | `SCHEMA_VERSION=2`; a stale row REFUSES (`StaleReturnInputs`, names clear→import→write-carryover); deleted the dead D-8 unlaunder |
| 3 | `bb72ae6` | `crates/btctax-core/src/tax/questions.rs` — `FormQuestion`, `QuestionId`(+`ALL`), `FORM_QUESTIONS` (8 entries); completeness anchor |
| 4 | `0ac80e2` | `screen_inputs` + `income answer` DERIVE from the registry; deleted 4 hand-written unanswered blocks + `schedule_b_part3_unanswered`; `foreign_trust` added to `schedule_b_files`. **§2.9 + P8a I1 now GREEN by named test.** `income answer` refactored (`Question`→`Skippable`(DOB) + new `Ask` enum; `live_questions -> Vec<Ask>`) |
| 5 | `db04781` | `ReturnHeader::build` → `HeaderError { Ssn, Unanswered(QuestionId), MfjWithoutSpouse }`; the fail-closed PRINT boundary (P8a I3 + r3 M-6); neutralizes `printed.rs:936/943 unwrap_or(false)`; `admin.rs` uses HeaderError Display |

**Two shipped-code bugs fixed + held by mutation-checked tests:** §2.9 (circular Schedule B liveness silently
omitting the FBAR surface) and P8a I1 (MFJ-no-spouse dependent box).

**★ STEPS 1–5 WERE INDEPENDENTLY REVIEWED AND ARE GREEN (0C/0I).** Fable reviewed the core mechanism
(`reviews/P9-IMPL-fable-r1.md`, 0C/2I/1M/3Nit): the mechanism is faithful and 8 guards are mutation-held, but
**2 guards were held by NOTHING** (the [[untested-guard-pattern]], proven by mutations that survived the full
suite): the mortgage question could be dropped from `income answer`; `income import` could silently swallow a
stale row (dropping a Computed carryover — understating tax). **BOTH FOLDED** (`8b46b2b`, test-only + 3
comment fixes) and **the fold was re-reviewed GREEN** (`reviews/P9-IMPL-fable-r2.md`, 0C/0I): each new test
was confirmed to kill the reviewer's exact mutation. The production code never changed — only the tests that
hold it. **Lesson for steps 6–12: `make check` green ≠ guard held. Mutation-check every guard, AND make the
"answered/asked/refused" assertions registry-DERIVED so a new question is covered with zero edits.**
*(One recorded, non-blocking: IMPL r1 Nit-3 — the interim window where `dual_status_alien`/`foreign`/`salt`
value-refusals are not yet live is spec-conformant sequencing; they land at step 8. Keep the release gate
closed until step 8.)*

## 4. Steps REMAINING (resume here — spec §5 has the detail)

- **Step 6 — Sch A line 8 value behaviour + advisory, ONE step** (§2.7/§3.4). `mortgage_all_used_to_buy_build_improve
  == Some(false)` ⇒ **8a = 0 AND the line-8 box CHECKED AND** `MixedUseMortgageNotAllocated` advisory fires.
  Advisory TEXT branches on the deduction actually taken (itemized vs standard-wins — do NOT tell a
  standard-deduction filer "your Schedule A claimed $0"); `forgone_interest` = full `mortgage_interest_1098`,
  documented as a CEILING ("up to"). Scoped to `mortgage_interest_1098 > 0`. **Value + advisory are ONE step**
  (do not ship the advisory a step before the value — r5 M-2). **Regression test: the mixed-use filer
  COMPUTES under BOTH `Auto`-standard-wins AND `ForceItemize`** (there is NO refusal — r4 I-2 deleted it).
- **Step 7 — the blind + SALT advisories (fire on `None`) AND their skippable PROMPTS in `income answer`.**
  *(★ The prompts were moved here from spec-step-4 so prompt+advisory ship together — see §6.)*
  `BlindBoxForfeitedNotDeclared { per_box, persons }` — `persons` = `[taxpayer.blind.is_none()]` +
  `[Mfj ∧ (spouse absent ∨ spouse.blind.is_none())]` (mirror `AgedBlindBoxes::for_return`; MFS never counts
  the spouse). `SalesTaxElectionNotAsked` — fires on `None` ∧ `schedule_a.is_some()` (NOT "itemizes" — r5
  Nit-3). Prompts: blind (taxpayer + spouse, gated `spouse.is_some()`), SALT (gated `schedule_a.is_some()`) —
  SKIPPABLE (empty ⇒ stays `None`). Mutation: drop either advisory ⇒ a named test fails.
- **Step 8 — the value-refusals the property test can't hold** (§3.5, r5 I-3), each a named test + named
  mutation: **`DualStatusAlienUnsupported`** (`dual_status_alien == Some(true)`), **`SalesTaxElectionWithoutAmount`**
  (`salt_use_sales_tax == Some(true)` ∧ `salt_sales_tax_amount == 0` ∧ any income-tax SALT input > 0),
  **`ScheduleBForeignCountryMissing`** (`foreign_accounts == Some(true)` ∧ `foreign_country_names.trim().is_empty()`;
  detail names `income import`, not `income answer` — `answer` can't capture strings). *(`HsaActivityUnsupported`
  already exists from step 1.)*
- **Step 9 — DELETE the 3 dead fields + `serde_ignored` unknown-key rejection, TOGETHER** (§2.3). Delete
  `Person.ssn_valid_for_employment` (×2: Person + Dependent) and `W2.box13_retirement_plan`; add
  `serde_ignored`-based unknown-key rejection to `income import` (names `hsa_present` + each deleted field).
  Named test: TOML with `hsa_present=false` + `box13_retirement_plan=true` ⇒ refuses naming both; mutation:
  bare `toml::from_str` ⇒ fails. **MUST precede step 10** (else the classifier must bind class-(D) fields).
  *(`serde_ignored` v0.1.14 confirmed in the offline cargo cache.)*
- **Step 10 — the classifier** (§3.3). A fn destructuring EVERY struct reachable from `ReturnInputs` (incl.
  `Box12Entry`, `Carryforward`) with no `..`; `#![deny(unused_variables)]`; `_`-and-`_`-prefix rule forbidden
  on structs/collections/classifiable leaves; the exemption register carrying class + statutory reason (incl.
  `itemize_election` = class C, §2.8). Folds the open `header: _` follow-up (`return_refuse.rs`).
- **Steps 11–12 — LIMITATIONS.md + FOLLOWUPS.md.** LIMITATIONS: the new refusals; the 3 advisories; the two
  SCOPE-entailed software-answered boxes (**Schedule D QOF "No"** + **Form 8949 Box I/L**) and the Sch C
  G/H/I/J blanks; the `income clear`→`import`→(if carryover) `report --write-carryover` remedy; that
  `income show` refuses a stale row; that Sch B/MFS texts now name `income answer`; that `answer` can't
  capture strings. FOLLOWUPS (with owning phases): **(a) → P8** capture `mortgage_interest_deductible` (the
  Pub. 936 result — recovers the deduction step 6 zeroes); **(b) → the RELEASE GATE** refuse-and-reimport
  expires the moment real data exists (prior-year carryforwards are what a filer can't reconstruct).

## 5. Hard constraints (unchanged)

- **FROZEN**: `crates/btctax-core/src/tax/{types,compute,se}.rs`. Verify `git diff 059ec2a..HEAD -- <those>`
  is empty after each step.
- **Gate**: `make check` (~7s). **NOT** `cargo test --workspace` (402s).
- **Every step**: TDD (write the failing test, watch it fail, then implement); **mutation-check each guard**
  (delete it → a NAMED test fails → restore); commit; push at milestones.
- **Fixture churn helper**: `btctax_core::tax::testonly::answer_all_live_declarations(&mut ri)` /
  `answered(ri)` — registry-derived, so a new always-live question needs zero fixture edits. Answers "no"
  for all EXCEPT the mortgage question (answers "yes" → full 8a).
- Fish shell: quote globs (`rg ... --glob '*.rs'`); use a heredoc for `git commit -F -` (backticks
  shell-expand in `-m`).

## 6. Decisions to carry forward (do not re-litigate)

- **★ The class-(B) skippable prompts (blind/SALT) moved from spec-step-4 to step 7**, bundled with their
  advisories — same end state, mirrors the spec's step-6 "bundle the prompt with what makes it meaningful."
  **Flag this in the whole-diff implementation review.**
- **★ Review cadence**: the owner flagged review volume. Do NOT review every step. TDD + mutation-check per
  step; run **ONE whole-diff implementation review** at a larger milestone (after ~step 8, once the
  refusals/advisories land, or at the very end). The SPEC is already green 0C/0I across 8 rounds.
- **★ Sonnet 5 for mechanical fixture/test churn** (per owner 2026-07-14): two delegations so far, both
  correct on first review — zero corrections, no extra review cycles. Keep using Sonnet 5 for well-scoped
  mechanical work; reserve **Fable** for adversarial reviews. Dispatch subagents on files you are NOT
  concurrently editing (shared-worktree conflict — [[no-parallel-branch-tasks-shared-worktree]]).
- **Two reviewer fixes were overruled on the merits and upheld by the next reviewer**: the HSA-rename
  migration (old `true` → `None`, re-ask; NOT `Some(true)`) and the stale-row remedy (`clear`+`import`+
  `write-carryover`, NOT "treat as absent" which fails open on carryovers). Don't reopen.

## 7. The commit trail (implementation)

```
db04781  impl(P9 step 5): ReturnHeader::build -> HeaderError — the fail-closed print boundary   ← HEAD
0ac80e2  impl(P9 step 4): screen_inputs + income answer DERIVE from the registry
bb72ae6  impl(P9 step 3): questions.rs — the FORM QUESTION REGISTRY
a3ec138  impl(P9 step 2): SCHEMA_VERSION=2 — a stale row REFUSES, it is not migrated
f8c1f4c  impl(P9 step 1): field changes — the answered-ness fields become tri-state
4acb850  spec(P9 r9): fold r8's non-blocking residue (1M/3Nit) — spec is GREEN
```

## 8. Still parked (do not start without the owner)

- **P8** (`design/SPEC_input_surface.md`, r7) — the rest of the input surface. P9 changes its shape; re-read
  after P9 lands. (Step-6 follow-up (a) is owned by P8.)
- **P6.7** cleanup batch (12 Minors/Nits).
- **Release decision** — deferred until the input surface is drivable; also owns step-12 follow-up (b).
- **`p7-tenforty-upstream-followup`** — check for responses on issue #278 / PR #279.
