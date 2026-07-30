# CONTINUITY — bitcoin_tax (TaxApp)

_Last updated: **2026-07-30**. Written at a pause; safe to exit and restart._
_(Supersedes the 2026-06-28 edition, whose deep-research workflow completed long ago.)_

**Written for a reader with NO prior context.** Confirm the tree first:
`git log --oneline -5`, `git status`, `git branch --show-current`.

> **One line:** on branch `feat/amt-e2-vector-population` (**44 commits, NOT merged**, all five gates
> green throughout). Two tracks open — **(A)** Schedule 1-A for TY2025: spec + plan green, task T1 built;
> **(B)** a form-authority pipeline: steps 1-2 of 3 done for all 16 forms.
> **The next action is the label reader (track B step 3)** — characterised and deliberately NOT built;
> see §5. **The largest ARCHITECTURAL open item is §G-11 (§4a)** — the emitter cannot express "no
> testimony", so it constrains what B3 may emit. It does *not* block the label reader.

---

## 1. What this branch already did

- **★★ Fixed a LIVE DEFECT in shipped v0.14.0** — `FOLLOWUPS.md` **§G-9**. The §63(f) age-65 box on 1040
  line 12a was decided from the date of BIRTH alone, but i1040gi carves out a person who died in-year
  before reaching 65. A spouse who died at 64 got a $1,550 addition they were not entitled to —
  **understating tax on a signed return**, and invisible to both oracles (OTS takes a filer-answered
  `"You_65+Over?"` boolean; taxcalc has only `age_spouse`). Fixed with two class-(A) gates on
  `HouseholdHeader` plus two class-(B) dates on `Person`; **5 mutations killed**. Residue: **§G-9a**.
- **B1 + B2** (TY2025 groundwork) landed earlier: harness year seams, the `SaltLimitation` enum, per-year
  Form 6251 Part I, MAGI add-backs, 1040 L13b threaded, TY2025/TY2026 fail-closed gates.
- **T1 of B3** built: `Schedule1aParams`, `StepRounding`, `StairStepPhaseOut` in `tables.rs`. 8 tests,
  **8 mutations killed**.
- **Form-authority pipeline**: 66 documents archived as URL notes + 2.9 MiB committed text layer.

**Invariant held on every commit:** TY2024 provably unchanged — golden matrix md5
`c4e1853ed82d113ca5cd97ffd8abbf47` unmoved, both oracles exit 0, 2449 tests green.

---

## 2. Track A — Schedule 1-A (TY2025), branch B3

**Read:** `design/ty2025/SPEC_schedule_1a.md` (r3, 0C/0I) and
`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` (r3). Reviews in `design/ty2025/reviews/`.

- **T1 — DONE.** Rounding direction is a **parameter**: Parts II/III **floor** (lines 11/19), Part IV
  **ceils** (line 28) — statutory, because §163(h)(4)(B)(iii) says "or portion thereof". Three threshold
  pairs, three caps. `exhaustion_excess` is per-direction: Part IV exhausts at `threshold + $49,001`, not
  `+$50,000`.
- **T2-T7 — NOT started.**

★★ **Do NOT delete `ty2025_full_return_must_stay_fail_closed_until_complete`.** B3 satisfies its
**condition 4 only**. TY2025 `FullReturnParams` land LAST, after B4 — bundling early does not refuse, it
emits plausible wrong numbers.

### What the 13-agent provenance census found

(`design/ty2025/reviews/PROVENANCE_CENSUS_schedule_1a.md` — all re-verified against source)

1. **★★ Lines 5 and 14b have NO INPUT PATH.** `ReturnInputs` carries `w2s`, `int_1099`, `div_1099`,
   `g_1099` and nothing else, but both lines read from **1099-NEC / 1099-MISC / 1099-K**. They would be
   blank *because nothing can populate them* ⇒ they **REFUSE** (see §4.4 on why `0` is not an option).
2. **The line-5 ceiling is un-implementable as specified**, so it refuses rather than computing: it needs
   the deductible part of SE tax **plus** self-employed SEP/SIMPLE/qualified-plan contributions **plus**
   self-employed health insurance; printed Schedule 1 Part II carries lines 15/18/21 only.
3. **The four worksheets appear ZERO times in the FORM extract** — only in the *instructions* extract. A
   census driven off the form fixture alone could never red on a worksheet omission.
4. Worksheet arity comes from the **worksheet** (it prints four 1099 columns), not its narrative (which
   says overflow begins at "more than three").

---

## 3. Track B — the form-authority pipeline

`design/forms/README.md` is the entry point. **Three steps; a form is not done until step 3:**

| step | state |
|---|---|
| 1. **archived** — URL note + sha256 | ✅ 66 documents |
| 2. **extracted** — committed text layer | ✅ 57 documents, in `design/forms/extract/` |
| 3. **conformance-tested** — label census, decisions derived from each line | ❌ **Schedule 1-A only** |

**Acquisition is mechanical** — the point, because forms change every year:

- **annual**: `https://www.irs.gov/pub/irs-prior/{stem}--{year}.pdf`
- **periodic** (Forms 8275, 8283 — "Rev. Month Year", no tax-year edition):
  `https://www.irs.gov/pub/irs-pdf/{stem}.pdf`
- instructions are the **identically-numbered** `iNNNN`. `i1040gi` carries the 1040-family schedules that
  get no standalone booklet (Schedule 1-A, Schedules 2 and 3) and is the only one needing page ranges.

★ **PDFs are gitignored, not committed.** Each has a `<name>.pdf.txt` note with its URL + sha256; the
committed **text layer** is what tests read, so they need no PDF and no network. **A changed hash means
the IRS REVISED the document** — review it, never silently absorb it.

★ **A missing year can be correct:** `f1040s1a--2024.pdf` does not exist because OBBBA created
Schedule 1-A for TY2025. Not a fetch failure.

Tooling: `cargo run -p xtask -- cite-check` (34+ quotations verified verbatim; also prints authority
coverage) and `-- extract-schedule-1a`.

---

## 4. The doctrine established this session — READ BEFORE WRITING CODE

From the user; it now governs the work. Detail in `CLAUDE.md` and in memory
(`the-answer-is-in-the-manual`, `blank-is-the-normal-case`, `an-entry-is-testimony`).

1. **★★★ Taxes are simple instructions anyone can follow, and every form has an identically-numbered
   instructions document.** If implementing a line feels hard, **you have stopped reading and started
   inventing.** Difficulty is a signal to go back to the page, not to think harder.
2. **"§X disagrees with §Y" is a lookup, not a review finding.** A document need not agree with *itself*;
   it must agree with *the form*. Two sections that each match the manual cannot disagree.
3. **★★ Most fields on a tax return are BLANK, intentionally.** Never assert non-blankness — assert
   **provenance**: collected / computed from named lines / a constant the form prints / refusal.
   *"Usually zero"* is not a provenance.
4. **★★★ Every entry is TESTIMONY from the filer against the filer.** A blank is *no testimony*; a printed
   `0` is an affirmative sworn statement that the amount IS zero. Writing `0` on an unasked line
   **fabricates testimony under someone else's signature.** Whether a blank is lawful turns on **intent**,
   which is not software's domain — so btctax has exactly three lawful moves: **collect, refuse, or leave
   genuinely blank.** It must equally never build the opposite thing (a heuristic flagging an omission as
   suspicious). Both directions are software deciding intent.
   - ★ Sharper than "fail closed": **does the silence ASSERT, or FORGO?** Class (A) declarations assert ⇒
     must be answered or refuse. Class (B) benefit claims forgo ⇒ silence is lawful (*New Colonial Ice*).
     That is why §G-9's fix is legitimate: forgoing a deduction swears to nothing.
   - **Verified defect, §G-11:** `btctax-forms/src/lib.rs` `fmt_money(d: Usd) -> String` is the entire
     money path, so **no line can express blank**. Whole-surface; needs its own spec.
5. **Derive the decision FROM the line; don't check prose about it.** Rounding direction and
   cross-references are read off the printed text and asserted against the code
   (`tables.rs::schedule_1a_conformance`). This is how the Form 6251 line-33 class — "Subtract line 32
   from line **22**", once transcribed as line 12 and worth $200,000 on one vector — becomes a test.

★★ **RECONCILE THE TWO ARCHIVES BEFORE THE LABEL-READER WORK.** Found at the very end of the session,
after `design/forms/` had already been built: **`legal/primary-sources/` already exists** and holds

    statute-irc/        16 × 26 USC sections (HTML)      ← rung 4, THE LAW
    regulations-cfr/     6 × 26 CFR regs (XML)           ← rung 3
    irs-guidance/       11 × Notices, CCAs
    irs-publications/    6 × Pubs
    irs-forms/           7 × forms + instructions        ← OVERLAPS design/forms/
    federal-register/    1 × TD 10000 (broker regs)

So the four-rung ladder in §5a is **already archived in this repo**, and I wrote that brief as though we
would have to go and find rungs 3-4. ★ This is the exact failure the `the-answer-is-in-the-manual` memory
describes — *concluding from not having looked* — committed on the same day I wrote it down.

**Two archives with different provenance conventions is the "which one is authoritative?" ambiguity this
session was spent eliminating.** Reconcile before building the label reader, since both would feed it:
`design/forms/` is URL-note + hash + extracted text, machine-checked by `xtask`; `legal/primary-sources/`
is committed binaries with no manifest. Decide one convention, and note that `irs-forms/` overlaps
`design/forms/` directly (Form 8949, Schedule D, Form 8283 and their instructions).

---

## 4a. ★★ §G-11 — the largest architectural open item, and what it does and does not block

**`FOLLOWUPS.md` §G-11.** `btctax-forms/src/lib.rs` — `fn fmt_money(d: Usd) -> String { d.to_string() }`
is the **entire** money path. Every money field on every emitted form is `Usd`, never `Option<Usd>`, so
**no line can express blank**; `Usd::ZERO` prints `"0"`. Zero-suppression exists only ad hoc and only for
whole *rows* (`schedule_d.rs`, `fill8949.rs`).

Under §4.4 that is not a formatting gap: writing `0` on a line the filer was never asked about
**fabricates sworn testimony under their signature**. It is invisible to every value-checking test and to
both oracles, because `0` is the correct *value* in the overwhelming majority of cases — **the defect is
in the act, not the arithmetic.**

**What it blocks — state this precisely, it was overstated once already:**

| | |
|---|---|
| **Constrains** | B3's emission choices. It is *why* T3a has lines 5 and 14b **refuse** rather than print `0` — refusing is the only lawful move left when the emitter cannot stay silent. |
| **Does NOT block** | the label reader (§5), the conformance census, or archiving/extraction. Those are independent. |
| **Blocks eventually** | any honest emission of a form with unasked lines — i.e. the whole surface, on a long enough horizon. |

**It needs its own spec, not a patch.** Sketch only: the emitter's money type grows a "not stated" state
that survives to the AcroForm write; computations may not manufacture a *stated* zero from *unstated*
inputs; and each line records which of the three lawful moves (collect / refuse / genuinely blank) it
takes, and why. The per-line decision then becomes a reviewable fact instead of an accident of
`Decimal::default()`.

★ **Scope bound, from §4.4 and easy to overshoot in both directions:** do not build a heuristic that flags
an omission as suspicious either. Intent is not software's domain, and *both* directions — assuming
silence, and policing it — are software deciding intent.

---

## 5. ★ THE NEXT ACTION — the label reader (track B, step 3)

**Read `design/forms/LABEL_READER.md` first.** Characterised and deliberately unbuilt: the obvious regex
gives **45** where Schedule 1-A's answer is **48**, and shipping a reader wrong on the one form whose
truth we know would manufacture exactly the false confidence the census exists to prevent.

**Three distinct sub-problems, not one to tune:**

1. **Whitespace** — lines 1 and 3 have *seven* spaces after the number where the pattern allowed six.
2. **Sub-letters have no parent** — `2b`-`2e`, `4a`-`4c`, `36b` appear as a bare `b`/`c` on their own
   line, so the reader is a small **state machine**, not a filter.
3. **Some numeric lines are HEADINGS** — lines 4, 14, 22 carry no amount box. `22a`/`22b` are a bare
   `a`/`b` with *nothing after them* and are missed entirely.

**And two of sixteen forms return ZERO** under the leading-number pattern: `f1040sa` and `f1040` put the
number in a second column beside a category label; `f8949` is a grid.

**Agreed design:**

- the reader **proposes**; a **human-established expected LIST** (not a count) is the authority;
- **zero labels is ALWAYS a hard failure**, whatever the layout;
- unanalysed layouts sit in a ratchet that **may only shrink**
  (`cite_check::AUTHORITY_NOT_YET_ARCHIVED` is the working model);
- ★ pin an observation **of the form** (which reds when the form changes), never the **reader's own
  output** (which would assert only that the reader still does what it did).

**Cost, honestly:** 16 forms × 2 years, each needing its label list read off the form once. That is the
same act as transcribing the form, done once and then held by a test forever.

---

## 5a. ★ CONSULT FABLE ON THE PARSING STRATEGY — do this BEFORE paying the 32-list cost

**Why here and why Fable.** House rule (global `CLAUDE.md`): Fable is never the default and is reserved
for **a single review immediately before a first irreversible or costly action**. This qualifies on both
counts — the label-reader design fixes the shape of conformance for **16 forms × 2 years**, the cost it
gates is ~32 label lists read off forms by hand, and the failure mode is *false confidence* (a reader that
quietly finds 45 of 48 reports a form conformant by having nothing to check). Reviewing the strategy
before paying is far cheaper than discovering it is wrong on form 12.

★ **Ask the user's approval before dispatching** — Fable is escalation, never autonomous.

**Paste this to kick it off:**

> Consult Fable on the parsing strategy in `design/forms/LABEL_READER.md` before we build it. One
> question only: **is "reader proposes, human-established expected LIST is the authority" the right
> strategy, or is there a materially better one we are missing?** Give it the settled facts so it does
> not re-derive them, forbid a fresh audit, and make it answer in the fixed format.

**The brief the dispatched agent must carry** (sharp brief matters more than the tier):

- **THE ONE QUESTION.** Is the design in `design/forms/LABEL_READER.md` the best available strategy for
  enumerating a form's labels, given that the *purpose* is to distinguish *"this line encodes no
  decision"* from *"we forgot this line"*? If a materially better strategy exists, name it and say what
  it costs.

- **★★ A VERDICT OF "START OVER" IS EXPLICITLY WELCOME, AND THIS IS THE MOMENT FOR IT.** Say so plainly
  if the whole approach is wrong. We have paid for the archive and the extracts; we have NOT paid for the
  ~32 hand-read label lists or for 15 forms of transcription. **If we should begin again differently, the
  cheapest possible time to learn that is now** — do not soften it into "consider also…". A recommendation
  to discard `LABEL_READER.md` entirely counts as a successful consult, not a failed one.

- **★ WHAT WE ARE ACTUALLY DOING, so the strategy is judged against the real goal.** We are **filling out
  forms**, and the answers are written down for us in a four-rung ladder we may climb whenever a rung is
  silent:

  | rung | source | standing |
  |---|---|---|
  | 1 | **the form's own embedded instructions** — captions, cautions, "enter the smaller of", skip routing | guidance |
  | 2 | **the numbered instructions document** `iNNNN` (`i1040gi` for the 1040-family schedules) | guidance |
  | 3 | **the regulations** — 26 CFR | the agency's **interpretation** — binding in practice, **capable of being wrong** |
  | 4 | **the statute** — 26 USC | ★★ **the only rung that is LAW** |

  ★★ **Only rung 4 is law.** A Treasury regulation is the executive's reading of the statute; it is
  routinely held invalid for exceeding or contradicting it, the more so since *Loper Bright* ended
  deference. **If we believe the statute disagrees with a regulation, it is our duty to push back** — the
  tax system even supplies the instrument, **Form 8275-R** (Regulation Disclosure Statement), as distinct
  from Form 8275 for positions contrary to everything else.

  ★ **And the honest part: that duty is routinely neglected because challenging is expensive.** Say so
  rather than pretending otherwise — but do not let it silently become "the reg settles it". btctax
  emits Form 8275 and **not** 8275-R (`FOLLOWUPS.md` §G-12), so today it cannot *do* the duty at all: it
  can only agree with the regs, or take a contrary position undisclosed.

  ★★ **A believed statute/reg disagreement is now RECORDED AND TICKLED, not remembered.**
  `AUTHORITY_CONFLICTS.md` is the register; `cargo run -p xtask -- authority-conflicts` is the check, and
  an entry past its `review-by` **fails the test suite**. Neglecting the duty stays a legitimate choice
  (cost) — but it must be *a choice*, dated, with a review date, revisited. It can never again be an
  omission nobody decided. Mutation-verified: an overdue entry reds the suite.

  **We are never without an authority** — only ever without having gone and read it. Judge the strategy on
  how directly it gets us from "this line exists" to "this is what the form tells the filer to do", not on
  parsing elegance.
- **SETTLED — do not re-derive.** The measured layout data (leading-number works for 6 forms;
  `f1040sa`/`f1040` use a second column and return **0**; `f8949` is a grid); the three sub-problems
  (whitespace, parentless sub-letters, headings-with-no-amount-box); Schedule 1-A's truth is **48**; the
  extracts are committed and PDFs are not; `pdftotext -layout` for forms, plain for 3-column instructions.
- **EXPLICITLY IN SCOPE — the alternatives we did NOT evaluate**, and this is the real value of the
  consult: (a) reading the **AcroForm field names** from the fillable PDF instead of the text layer —
  btctax already has `xtask dump-fields`, and a fillable form's field list *is* an enumeration of its
  boxes, which may make the whole text-parsing problem the wrong problem; (b) `pdftotext -bbox` /
  coordinate-based column detection instead of whitespace heuristics; (c) the IRS **MeF XML schemas**,
  which enumerate every line as a typed element; (d) accepting per-form hand-written lists as the
  *primary* artifact with the reader used only as a change-detector.
- **FORBIDDEN.** Re-auditing the spec or plan; restating the transcription doctrine back to us; style,
  naming, prose. Do not propose "add more tests" without naming the specific defect it catches.
- **OUTPUT FORMAT.** `RECOMMENDATION: <keep | replace-with-X | hybrid | START-OVER-with-X>`, then at most
  five bullets of justification, then `COST DELTA:` versus the ~32 hand-read lists, then
  `WHAT WOULD MAKE THIS WRONG:` — one sentence naming the assumption its advice depends on. If the verdict
  is `START-OVER`, add `WHAT WE KEEP:` — the archive and extracts are paid for and should not be discarded
  by reflex.

★ **`dump-fields` is the lead most likely to change the answer.** If the fillable PDFs carry usable
AcroForm names, label enumeration may be a *lookup* rather than a parse — and the emitter needs those
names anyway to fill the form (B4), so the work would serve two purposes instead of one.

---

## 6. Traps that have already cost time

- **`make check` ≠ CI.** Five gates: `make check` · `cargo fmt --all --check` ·
  `cargo +1.88 check --workspace --locked` · `cargo run -p xtask -- check-isolation` ·
  `bash scripts/pii-scan-generic.sh` (**scans HEAD — commit first**).
- **`.venv/bin/python`**, never bare `python3` (taxcalc/pandas live there). `sweep.py` needs
  `--seed N --count N`. `OTS_DIR=~/OpenTaxSolver2024_22.07_linux64`; OTS 2025 is at
  `~/OpenTaxSolver2025_23.06_linux64`.
- **`include_str!` must not escape its crate** — it ships a broken tarball with exit 0. Hence the
  Schedule 1-A fixture is in-crate while the other 57 extracts live in `design/`, read by `xtask`
  (`publish = false`).
- **A shrinking golden is a refusal, not a change.** Investigate before regenerating.
- **`rm -rf __pycache__` after restoring a mutated Python file** — a stale cache twice masked a restore.
- **One branch-mutating task in the shared tree at a time**; delegated agents must not spawn their own.
- ★ **Do not truncate large objects passed between workflow agents** — truncating classifications to
  14 KB made reviewers report labels as "omitted" that were merely unsent.

---

## 7. Open items

| id | what |
|---|---|
| **§G-11** | ★★ the emitter cannot express "no testimony" — **see §4a**; largest architectural item, needs its own spec |
| **§G-9a** | do the §63(f) **blind** boxes have a death interaction? |
| **§G-10** | residue: coverage — a checker that cannot tell "encodes no decision" from "we forgot this line" |
| §G-6c / §G-6d, E4-E6 | AMT Tier-2 items, parked behind the TY2025 pivot |
| B3 T2-T7, B4 | Schedule 1-A build; filing assets + corpus |
| — | **1 of 16 forms** has reached conformance (step 3) |
| **§G-12** | btctax emits Form 8275 but **not 8275-R**, so a position contrary to a REGULATION cannot be disclosed — the duty is unrepresentable |
| — | `AUTHORITY_CONFLICTS.md` is empty: we believe no reg governing our forms disagrees with the statute. **A statement about what we examined, not a guarantee.** |
| — | **crates.io temp token still needs revoking** (from the v0.14.0 publish) |
