# BRIEF — the SCENARIO coverage census: what we do not cover, and why

**Tier:** opus. **Isolation:** worktree. **READ-ONLY** — change no source file. No subagents.
**Owner ask, 2026-09-13:** *"can we dispatch an agent to look for tax scenarios that we do not cover so we
can decide if permanently, temporarily, or accidentally omitted?"*

## 0. The one question

> For every tax scenario a 1040 filer can be in, is btctax's position on it **recorded** — covered,
> refused, declared absent, or ruled out — or is the scenario simply **unaccounted for**?

Only the last is a defect. The owner's three buckets map onto the evidence like this, and **every candidate
must land in exactly one**:

| bucket | what makes it that | evidence required |
|---|---|---|
| **covered** | we handle it | ★ **name the test.** A "covered" claim with no test naming it is UNACCOUNTED |
| **permanently omitted** | a policy ceiling | a `RefuseReason`, or `LONG_RANGE_PLAN_filing.md` §7, or an owner ruling |
| **temporarily omitted** | scheduled | `YEAR.toml [forms_absent]` with a reason, or a `FOLLOWUPS.md` entry with an owning phase |
| **★ accidentally omitted** | nothing says anything | **the finding** — no refusal, no declaration, no ruling, no test |

★★ **This is `CLAUDE.md`'s "blank is the normal case" rule one level up.** There, a blank line is correct
when the inputs say so and a defect when nothing ever populated it, and the two are indistinguishable on the
page. Here a *scenario* we do not handle is fine when we refuse it by name and a defect when we silently
emit a return anyway. **Same invariant, scenario granularity: what must exist is determinate PROVENANCE,
not coverage.**

## 1. DERIVE the candidate set — do not imagine it

`CLAUDE.md`: *"Derive the list, or make the compiler hold it — never type one beside a set that grows."*
A hand-imagined list of tax situations is exactly the failure that rule exists to stop, and it is
unreviewable. Enumerate from primary sources already in the repo:

1. **Every numbered line of the 1040 and its schedules**, from `design/forms/extract/*.txt` (126 committed
   text layers — the authority, never the rendered page).
2. **Every schedule and form the 1040 and its instructions REFERENCE** — `i1040--*.txt` names the attachment
   for each line. A referenced form that btctax has no position on is a candidate.
3. **The instructions' own decision points** — *"Who Must File"*, *"You must file Schedule X if…"*,
   *"Check the box if…"*. These are the IRS's own enumeration of scenarios; they beat any list you invent.
4. `Stem::ALL` versus what each `YEAR.toml` bundles.

Say in the report **which source produced each candidate**, so a reader can re-derive the set.

## 2. Already measured — do not re-derive (controller, 2026-09-13)

| fact | value |
|---|---|
| `RefuseReason` variants | **140** (`return_refuse.rs`) |
| `YEAR.toml [forms_absent]` entries | **13**, each with a written reason |
| `LONG_RANGE_PLAN_filing.md` §7 | six subsections: 7.1 e-file, 7.2 state, 7.3 a second §24(b)/§32 in `btctax-forms`, 7.4 last-mile absences, 7.5 P2-profile "not now", 7.6 one thing not to "fix" |
| committed form text layers | **126** extracts, **70** geometry fixtures |
| EITC / §32 | **0** mentions in `btctax-core/src`, 0 in cli, 0 in forms, and **no `RefuseReason` names it** |
| Form 8606, RMD | **0** mentions anywhere in `crates/` |
| 1099-R | 12 core / 4 cli / 1 forms, **no `RefuseReason` names it** |
| IRA *deduction* | `RefuseReason::IraDeductionClaimed` — a recorded boundary ✓ |

★ **EITC is the exemplar to reason from, not a conclusion to repeat.** It is absent, and nothing refuses
it. So which bucket? §7.3 forbids *"a second §32 implementation in `btctax-forms`"*, which is a statement
about **where** it may live, not whether it exists. Resolve it, with evidence.

## 3. State the DIRECTION OF HARM for every unaccounted scenario

`STANDARD_WORKFLOW.md`: *an understatement of tax is worse than an overstatement.* So for each finding say
which way it errs:

- **understates tax** (we omit income, or claim a deduction/credit the filer is not owed) — the worse class;
- **overstates tax** (we omit a deduction or a refundable credit the filer IS owed) — still a wrong return,
  and for EITC it can be thousands of dollars the filer never receives;
- **refuses to print** — the FR-102 shape: an HSA filer could not print a single page, and twelve
  task-level reviews at 0C/0I missed it.

Rank findings by **(direction of harm × plausibility for a real filer)**, not by form number.

## 4. Scope

**IN:** the federal Form 1040 and every schedule/form it references, TY2024–TY2026.

**OUT — and these are settled, do not relitigate:**
- **Do NOT re-run the field provenance census.** That exists at BOX granularity (TY2024: 1158 fields, 662
  mapped, 496 undecided). This census is one level **up**, at *scenario* granularity. Cite it; do not redo it.
- §7's rulings: e-file/MeF, state returns, a second §24(b)/§32 in `btctax-forms`.
- Owner rulings: **no TY2025 return will ever be filed with this software**; aggregate 1099-B with
  adjustments is the permanent ceiling (FR-110); the filer may be told to google where to mail (FR-132).
- **Propose no implementation.** Classify and report. The owner decides what gets built.
- Do not edit any file except your own report.

## 5. Stop-and-report

**Six briefs in this arc were refuted by their implementer, three written by this controller, all today.**
If you can disprove anything in §2 — that EITC really has no refusal, that `[forms_absent]` has 13 entries,
that §7 says what I say it says — **stop and report it** rather than building on it. Check, don't trust.

## 6. Deliverable

Final action: write the report with a Bash heredoc (`cat > <path> <<'MARKER'` … `MARKER`), **not** the
`Write` tool, to:

    design/agent-reports/RECON-scenario-coverage-census.md

Return only a short summary plus that path. Structure: (a) how the candidate set was derived, with counts
per source; (b) the four-bucket table, every candidate in exactly one, with its evidence — test name,
`RefuseReason`, `[forms_absent]` line, or §7 subsection; (c) the **unaccounted** list ranked by harm ×
plausibility, each with direction of harm; (d) the handful you would put in front of the owner first, and
why. A table, not an essay.
