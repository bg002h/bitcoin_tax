# CONTINUITY — bitcoin_tax (TaxApp)

_Last updated: **2026-07-30**. Written at a pause; safe to exit and restart._
_(Supersedes the 2026-06-28 edition, whose deep-research workflow completed long ago.)_

**Written for a reader with NO prior context.** Confirm the tree first:
`git log --oneline -5`, `git status`, `git branch --show-current`.

> **One line:** on **`main`** — the `feat/amt-e2-vector-population` work is **MERGED** (all five gates
> green throughout; the branch's §0 order ①–⑥ is **complete**). **The §G-13 FIELD-PROVENANCE CENSUS is
> DONE: 15 of 15 forms, 1158 AcroForm fields = 668 mapped + 490 censused, ZERO unaccounted**, and the
> `CENSUS_NOT_YET_WRITTEN` ratchet is closed to `is_empty()`. It surfaced **18 gap fields / 9 unasked
> items** — the register table is `FOLLOWUPS.md` §G-13. Two tracks remain open — **(A)** Schedule 1-A for
> TY2025 (spec + plan green, task **T1 built**, T2–T7 untouched); **(B)** the form-authority pipeline
> (steps 1–2 of 3 done for all 16 forms; the label reader, §5, is increment 1 built).
> **The largest ARCHITECTURAL open item is §G-11 (§4a)** — the emitter cannot express "no testimony".
> **Next work is USER-DIRECTED**; nothing below is auto-start.

---

## 0. ★★ THE ORDER OF WORK — **ALL SIX COMPLETE (2026-07-30)**

This section drove the merged branch. It is kept as the record of what was done and why, **not as a
queue** — there is nothing here left to start.

| # | do this | outcome |
|---|---|---|
| **①** | ~~Fable consult on the HARNESS~~ | **✅ DONE** — verdict `needs-changes`; it *did* change what we built. Verbatim: `reviews/harness-design-fable-r1.md`. See §0a. |
| **②** | ~~Build the harness: A1 → A2 → A3, then B1/B2~~ | **✅ DONE** — `design/HARNESS.md` r2. ★ It fired on its own author twice: it blocked a `core.hooksPath --unset`, and it exposed that A4 had **never run** because `mkdir -p` in Bash bypassed the Write-tool hook. Both holes are closed (`scripts/hooks/`). |
| **③** | ~~Reconcile the archives~~ | **✅ DONE** — owner chose **hybrid**: storage differs by document kind, provenance does not. One manifest spans both trees (`xtask authority-manifest`). Residue: duplicate documents, pinned and shrink-only, with a review-by date that **reds the suite when it passes** (`ARCHIVE_RECONCILIATION_REVIEW_BY`). |
| **④** | ~~Fable consult on the PARSING STRATEGY~~ | **✅ DONE** — `reviews/label-reader-strategy-fable-r1.md`. ★ One cited measurement was **fabricated** (a phantom `f1_02` name gap); verified false, the *conclusion* kept on principle, the *evidence* discarded. |
| **⑤** | ~~The label reader~~ | **increment 1 BUILT** (`form_geometry.rs`, `label_reader.rs`); increment 2 redirected into the census. See §5. |
| **⑥** | ~~Fable consult on FIELD PROVENANCE~~ | **✅ DONE** — `reviews/field-provenance-fable-r1.md`, plus `shred-and-year-fable-r2.md` and `resumability-vs-discovery-opus-r1.md`. Built out into the §G-13 census, now complete. |

### ✅ What ⑥ became: the field-provenance census, COMPLETE

`crates/btctax-forms/tests/field_census.rs` asserts, for **all 15 forms** `fill_full_return` can emit,
that `(map FQNs ∪ [census] FQNs) == the PDF's AcroForm field set`, **exactly**. Every one of the 1158
fields now carries a determinate provenance: filled, or recorded as `unmodeled` / `artifact` / `gap`
**with a reason**. `CENSUS_NOT_YET_WRITTEN` ran 15 → 0 and its bound is now emptiness, so a 16th form
cannot arrive uncensused — mutation-verified in both directions.

**It found 18 gap fields / 9 unasked items — the table is in `FOLLOWUPS.md` §G-13.** The one that
changes the tax is **Schedule C line G** (material participation): a non-materially-participating sole
proprietor's income is passive and therefore §1411 NII, but btctax's NII omits Schedule C income
unconditionally, so the NIIT is understated. Adjudicated against Form 8960 line 4a and i8960's own
What's New, not inferred — and finding it corrected an f8960 census entry written earlier in the same
burndown.

★ **The method that worked, for the next form:** run the field probe, read each line's meaning from the
form's **extracted text** (never from position alone), verify every claim about btctax's behaviour in
**source** before recording it as a reason, and let the test — not the author — count the gaps.
★ **Two traps hit repeatedly:** `cargo fmt` reflows a shrinking list and silently breaks a string
replace (caught only by `the_two_lists_partition_every_form` and a fixed-size `[&str; 15]`), and
Form 8283's bundled asset is **Rev. 12-2023** while the archive holds **Rev. 12-2025** — different
sha256, so the archived extract is the *wrong revision* to transcribe from.

---

## 0a. ✅ FABLE CONSULT #1 — the harness — **DONE 2026-07-30**

**Verdict `needs-changes`.** Verbatim output: [`reviews/harness-design-fable-r1.md`](reviews/harness-design-fable-r1.md),
folded into `design/HARNESS.md` **r2**. Three load-bearing claims were independently verified against the
tree before folding (table at the end of that file). What it changed:

- **F1–F5 are TWO classes, not five** — **(α)** acted without observing an available fact (F1, F3);
  **(β)** shipped an instrument never seen discriminating (F2, F4, **F5**). The r1 five-mechanism list was
  the excuse-list mistake the document itself warns against. r2 is organised around the classes.
- **★★ A new top item: the harness-is-installed gate (A1).** `scripts/pre-push` is a reviewed, hardened
  hook, executable, in-repo since 2026-07-02 — `core.hooksPath` is **unset** and `.git/hooks/` holds only
  `*.sample`, so **it has never run**, and its install command is written in its own header. Without A1,
  H1/H3 repeat F4 on day one.
- **H3 as drafted provably would NOT have caught F1** — `design/forms/` is depth-2 under a `design/`
  dating to 2026-06-28, so a "new top-level path" trigger walks past. Split into a *deny* (shape-detector
  at `Write` time, now folded into A3) and an *ask* (any new directory at any depth, A4).
- **H4's lint DROPPED** → **B1 "seen-red-once"**: no checker exists until observed red on a planted
  defect. Covers F2+F4 as one class; cannot be satisfied performatively.
- **H5's lint DROPPED as having no target** (verified: `.slice(`/`.substring(` appear only in the two
  places *describing the lint*, zero in code) → **B2 pass-by-path payloads**.
- **Scope answers:** (c) keep memory as principles, wire trigger-shaped ones into hook messages; (d) **no**
  session-shaping — a checkpoint cadence is the forbidden self-verification scaffolding.

<details>
<summary>The original dispatch brief (kept for provenance)</summary>

★ **Ask the user's approval before dispatching.** Fable is escalation, never autonomous.

**Paste this to kick it off:**

> Consult Fable on `design/HARNESS.md` — the harness meant to stop me violating doctrine I have written
> down. Give it the full context from CONTINUITY.md §0a, one question only, and let it say the design is
> the wrong shape.

**The brief the dispatched agent must carry:**

- **THE CONTEXT — state all of it, it is what makes the question answerable.**
  - **The project.** `btctax` emits a complete US federal 1040 that a human signs under **26 USC §6065
    penalties of perjury**. A wrong number is the worst outcome; an **understatement** of tax is worse
    than an overstatement. The codebase is Rust, ~2450 tests, five validation gates, heavy use of
    mutation-verification ("a guarantee without a test that reds when it is removed does not exist").
  - **The problem.** The *assistant* (me) reliably writes down correct doctrine — in `CLAUDE.md` and in a
    persistent memory directory — and then violates it, sometimes **the same day**. The precipitating
    example: a memory was written saying *"before deriving or building, grep for what already exists — I
    conclude from not having looked"*, and hours later a primary-source archive was built from scratch
    without checking that `legal/primary-sources/` already held the same material.
  - **The diagnosis so far.** `CLAUDE.md` and memory are **passive context** — read at session start,
    violated 40 tool calls later while executing rather than reflecting. This is the same defect the
    codebase itself has been fixing all session: **held by convention, not construction.**
  - **The five OBSERVED failures**, from one session, all mechanically detectable — F1 built-without-
    checking; F2 enumerated from a range or hand-list instead of the source (**three separate times**);
    F3 committed with the gate red (ran it, never read the output); F4 claimed a checker worked while it
    was blind to the exact case it existed to protect; F5 truncated a payload between sub-agents and then
    reported the artifact as a finding. Details in `design/HARNESS.md`.
  - **The proposal.** H1 pre-commit hook running the gates · H2 a test forbidding two primary-source
    archives · H3 a `PreToolUse` hook on `Write` for new top-level paths (fires at the decision point) ·
    H4 a lint for enumeration-from-a-literal · H5 a workflow-script lint on `.slice()` into agent prompts.
  - **The available surface.** Claude Code hooks (`PreToolUse`/`PostToolUse`, currently unused in this
    repo), git hooks (none installed), the Rust test suite, `xtask` dev tooling, and `CLAUDE.md` itself.

- **THE ONE QUESTION.** Is `design/HARNESS.md` the right shape for making an assistant actually follow
  doctrine it has already written down — and what would make it materially better? Concretely: **which of
  H1-H5 will actually fire, which will be muted or routed around, and what is missing entirely?**

- **★★ SAYING "THIS IS THE WRONG SHAPE" IS A SUCCESSFUL CONSULT.** Nothing is built yet. If the whole
  approach is misconceived — if the failure is not addressable by mechanism at all, or if there is a
  categorically better lever (different memory structure, different session shape, different division of
  labour between assistant and tests) — **say so plainly now**, while it costs nothing. Do not soften it
  into "consider also…".

- **EXPLICITLY IN SCOPE — what we suspect but have not evaluated.**
  (a) **H4 is the highest-value and least likely to work** — F2 is a reasoning failure with only a faint
  syntactic shadow. Is there a better lever for "enumerated from the wrong source"? (b) Do hooks that
  merely *ask* (H3) change behaviour, or do they become noise that gets muted — and is there evidence
  either way? (c) Is the **memory system itself** mis-shaped for this: should doctrine be phrased as
  triggers ("when creating a new top-level directory, …") rather than principles? (d) Should any of this
  be **session-shaped** instead — a required opening action, a checkpoint cadence — rather than
  tool-shaped? (e) What does the **failure data** suggest that we have not noticed: are F1-F5 five
  problems or one?

- **FORBIDDEN.** Proposing more "be careful" instructions — that is exactly what already failed, and
  adding more is the null action in a costume. Proposing self-verification scaffolding ("add a final
  verification step") — the user's global config forbids it and it over-verifies with no quality gain.
  Proposing gates on *judgement* rather than on facts — they get routed around, which teaches that gates
  are routable. Re-auditing the tax logic, the spec, or the plan.

- **OUTPUT FORMAT.** `VERDICT: <sound | needs-changes | wrong-shape>`, then **per-item** `H1..H5:
  <keep | drop | change-to-X>` with one line of reasoning each, then `MISSING:` (up to three mechanisms
  we did not think of, most valuable first), then `WHAT WOULD MAKE THIS WRONG:` — one sentence naming the
  assumption the advice depends on.

★ **The measure of the harness is not that it exists.** It is whether a future session **fails a gate it
would otherwise have walked past.** Ask the reviewer to say which of its recommendations would actually
produce that, and which would merely look like rigour.

</details>

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

★★★ **CORRECTION 2026-07-30 — THERE ARE FOUR ARCHIVES, NOT TWO.** Everything below this box was
written from memory; `cargo run -p xtask -- archive-check` (harness A3) walked the tree on its first
run and found two more that had never been named anywhere:

★ **Refined after a full walk (the first pass sampled 4 PDFs and generalised — F2, again).** They are
not four peer archives. They are **TWO CONVENTIONS, each with two layers**, plus one directory of
legacy strays:

| | binaries | text layer | provenance |
|---|---|---|---|
| **(A) `design/forms/`** | PDFs **gitignored**; each has a `.pdf.txt` URL + sha256 note | `design/forms/extract/` — **57 committed** extracts (what tests read) | hashes + `MANIFEST.json`, machine-checked by `xtask cite-check`; a changed hash means the IRS REVISED it |
| **(B) `legal/primary-sources/`** | **47 binaries COMMITTED** | `legal/text/` — **25 committed** extracts | `legal/_scripts/fetch_*.sh`; **no hashes, no manifest, no revision detection** |
| strays | `design/amt-form6251/` — 8 duplicate notes, **2 unique** (`f6251--2026-DRAFT`) | — | older, terser note template |

★★ **The `.pdf.txt` files were never extracts** — they are provenance notes. They "diverged" only
because (A)'s template is richer (737 B vs 289 B). The real text layers are `design/forms/extract/`
and `legal/text/`.

★★ **So the reconciliation was ONE question, not four:** *commit the binary, or commit only its hash +
extract?* (A) keeps the repo small, makes an IRS revision detectable, and needs the network to
re-obtain. (B) is self-contained and offline, with no revision detection at all. **(B) holds material
(A) lacks — the statute and the regs — so neither tree can simply be deleted.**

### ✅ DECIDED 2026-07-30 — **hybrid**: storage differs by kind, provenance does not differ at all

Forms are re-fetchable from `irs.gov/pub/irs-prior` forever and are **revised annually**, so a hash is
exactly the alarm you want ⇒ note + sha256, binary gitignored. The statute and the regs are **law
as-of-a-date**, should be frozen in the repo, and their non-IRS URLs are less stable ⇒ committed. What
is now *identical* across both trees is the thing that was actually broken: a single manifest and a
single checker.

- **`cargo run -p xtask -- authority-manifest`** — **113 entries** (47 committed + 66 note-only;
  16 statute, 6 regulation, 33 instructions, 40 form, 12 guidance, 6 publication), each with kind,
  storage, sha256, URL and extract. `--regen` **derives** it from the trees — never hand-listed.
- **Two directions, because one is not enough.** *verify*: every entry resolves and every committed
  file still hashes true (a changed hash means the source was **REVISED** — review, never absorb).
  *census*: every primary source in an accounted tree **is in the manifest** — the shape detector
  pointed inward, catching "archived but never recorded".
- ★★★ **`MANIFEST.json` already existed with 66 entries and NOTHING read it.** A manifest nobody
  checks is F4 in its purest form. It has a reader now.
- **113 of 113 URLs recorded — `URL_NOT_RECOVERABLE` is EMPTY.** Getting to 110 required parsing what
  the fetch scripts actually use (`declare -A` map + `for` loop); a naive parse got 87 and silently
  dropped **every rung that is law**. The last 3 — CCA 202302012 and 26 USC **§61** / **§1223** — were
  found by web search and then **verified by sha256 against the committed bytes** (all three
  byte-identical) before being written into `legal/_scripts/fetch_remainder.sh`.
  ★ **Verification is the point, not ceremony:** a URL that merely *looks* right asserts a provenance
  we have not established — the same sin as a paraphrase presented as a quotation.

### Countdown: **15 → 7** duplicate groups, and **4 → 3** archives (2026-07-30)

★ **A correction first.** "All 15 are `design/amt-form6251/` strays" was wrong — generalised from the
4 groups that happened to be sampled. **F2 again**, in the note describing the F2 detector. A full
walk showed only **8** were strays.

**DONE — the 8 are retired.** `design/amt-form6251/` is now **purely a design directory**
(`PLAN.md`, `PART_III.md`, `reviews/`, the vector generator) and is **off the archive list**.

- ★★ **`crates/xtask/src/cite_check.rs` read `design/amt-form6251/{form}--{year}.pdf` in LIVE CODE** —
  deleting first would have broken the fixture regenerator. Repointed to `design/forms/{year}/`, and
  the proof is that re-extraction reproduced both fixtures with **only the `# Source:` line changed**
  (same sha256, same text). That also repaired the two provenance lines without hand-editing either.
- The 2 unique files (`f6251--2026-DRAFT`) moved to `design/forms/2026/`.

**Remaining 7 — two different things, neither a stray, both DECISIONS rather than cleanup:**

| # | groups | what it is |
|---|---|---|
| **3** | `design/forms/{year}/{f8275,i8275,f8283}` == `design/forms/periodic/*` | **By design.** These forms are "Rev. Month Year" with no tax-year edition, so the year-named path is an alias. Retiring means deciding whether year-indexed lookup may resolve through `periodic/`. |
| **4** | `design/forms/2025/*` == `legal/primary-sources/irs-forms/*` | The genuine **(A)/(B) overlap** — Form 8949, Schedule D + their instructions, under both conventions. Under the hybrid rule *forms* belong in (A) as note+hash, so (B)'s copies are the redundant ones — but they are **committed binaries with extracts in `legal/text/`**, so this is a deletion decision, not a tidy-up. |

`DUPLICATE_SOURCE_GROUPS = 7` pins it; the test reds if it rises **or** if it falls without the
constant coming down. **Neither remainder blocks ④.**

★ ~~`design/amt-form6251/` is a **design directory**, not an archive~~ — **done, see the countdown
above.** Original note: retire its form-notes, keep
`PLAN.md` / `PART_III.md` / `reviews/` / the vector generator, and repoint the provenance line in
`crates/btctax-core/src/tax/fixtures/schedule_1a_2025_form.txt`.

★ **That "two" was itself F2** — a count written from recollection instead of a walk, inside the very
note warning against enumerating from a hand-list. The number is now **measured and pinned**:
`archive_check::the_archive_count_may_only_shrink` reds if a fifth appears, and
`every_accounted_for_tree_still_exists` reds when one is retired, so step ③'s progress is a test result
rather than a claim.

★★ **RECONCILE THE ARCHIVES BEFORE THE LABEL-READER WORK.** Found at the very end of the session,
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

## 5. The label reader (track B, step 3) — ⑤ in the §0 order, AFTER the harness and the reconcile

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

## 5a. ★ FABLE CONSULT #2 — the parsing strategy (④ in the §0 order). Do it BEFORE paying the 32-list cost

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

  ★★ **CORRECTED 2026-07-30 — all four rungs are ALREADY ARCHIVED. Do not treat finding them as work.**
  The brief above was written as though rungs 3-4 had to be sourced. They are in the repo and now
  machine-verified: `cargo run -p xtask -- authority-manifest` lists **105 entries** — 16 × 26 USC
  (rung 4), 6 × 26 CFR (rung 3), 29 instructions (rung 2), 40 forms (rung 1), plus guidance and pubs —
  each with kind, storage, sha256, URL and extract, and **every URL recorded** (`URL_NOT_RECOVERABLE`
  is empty). The reviewer should assume the ladder is *available*, and judge only how directly a
  strategy climbs it.
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

★★★ **`dump-fields` WAS the lead most likely to change the answer — so it was MEASURED, not left as a
question.** Full data in `design/forms/LABEL_READER.md` §"MEASURED 2026-07-30". Summary for the brief:

- **The naive hope is FALSE.** Field names are sequential (`f1_01`…`f1_31`), and semantic naming is
  wildly inconsistent: Schedule 1-A names its line-22 table, f1040sa names 4 lines, f1040 names **1**,
  and **f6251 names ZERO**. A names-based strategy works on one form and collapses on the next.
- **But the GEOMETRY is universal and answers the question the text layer cannot.** An amount box is a
  field with coordinates; `pdftotext -bbox` gives every word coordinates; the two origins differ by a
  mechanical flip (~792 page height). Join on y and each row yields *its printed line number* **and**
  *whether it has an amount box* — which is sub-problem #3 (heading vs label) solved by construction.
  It also names 22a/22b outright, the case the regex misses entirely.
- **It is evidence, not an oracle:** the AcroForm enumerates **boxes**, the census asks about **lines**.
  Headings have no box but are still labels; one line can own several boxes. 54 fields ≠ 48 labels on
  the one form whose answer we know.

**So the live question is no longer "is there a better source?" but "what is the right ARBITER between
three imperfect signals — text layer, AcroForm geometry, and a hand-read list?"** That is what the
consult should answer.

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
| — | **1 of 16 forms** has reached conformance (step 3 of the *form-authority* pipeline — distinct from the field census, which is complete) |
| **§G-13 gaps** | ★★ **18 gap fields / 9 unasked items** from the completed census — table in `FOLLOWUPS.md` §G-13. Schedule C line G (material participation → NIIT) is the one that changes the tax |
| — | Schedule C carries ONE aggregate `expenses: Usd`, so line 28 prints a total whose addends (lines 8–27b) are all blank — recorded, deliberately not counted as a gap |
| **§G-12** | btctax emits Form 8275 but **not 8275-R**, so a position contrary to a REGULATION cannot be disclosed — the duty is unrepresentable |
| — | `AUTHORITY_CONFLICTS.md` is empty: we believe no reg governing our forms disagrees with the statute. **A statement about what we examined, not a guarantee.** |
| — | **crates.io temp token still needs revoking** (from the v0.14.0 publish) |
