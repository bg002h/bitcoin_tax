# The year-transition report — what makes a new tax year a DATA change

**Date:** 2026-09-05 (rewritten around a changed target; supersedes the 2026-09-05 decision report
of the same name).
**Sources:** six scaffolding lenses, `design/agent-reports/2026-09-05-scaffolding-{seams,package,compute,instruments,refusal,machine}.md`,
plus the six port lenses they supersede in part (`…-ty2026-port-*.md`).
This document synthesizes; it does not restate. Every claim is cited to the lens that measured it.

---

## 1. The target, and what it changes

**Owner ruling (2026-09-05): TY2025 is PAUSED. TY2026 is the first filed year. TY2027 timing is
acceptable.** Verbatim: *"what we need most is the scaffolding … the year to year transition for
every form we intend to support, not necessarily a completed 2025."*

The previous edition of this document answered *"what does TY2026 cost, given that TY2025 is the
gate."* That framing is dead at the top and mostly alive in the details. The question is now:

> **What has to be true for a new tax year to be a data change?**

Four consequences reorder everything below.

1. **The gate argument is void.** `full_return_for(2025) -> Some` is no longer on anybody's critical
   path. Every item whose *only* justification was "TY2025 must land first" is re-marked MOOT in §5
   rather than deleted — a reader needs to know it was considered.
2. **The pre-forms window is now the product's main path, not an edge case.** TY2026's *numbers*
   are already bundled and KAT-pinned (`BundledTaxTables::load()` inserts 2017/2024/2025/2026,
   `btctax-adapters/src/tax_tables.rs:76-81`); `btctax report --tax-year 2026` exits 0 with a full
   crypto-delta block today. What is missing is the **forms**, which land Nov 2026 – Feb 2027. So
   "the tables are out, the forms are not" is where btctax now *lives*, and every refusal-surface
   defect in §2.5 becomes a live product defect rather than a hypothetical (refusal lens §3c).
3. **The jump is two revisions, not one.** The shipped emitter is TY2024. TY2025's assets exist but
   ten of fifteen are loaded by nothing. A TY2026 port that keys off "what changed last year" is
   keying off a year that was never wired.
4. **The scaffolding is the deliverable.** Not a TY2026 packet. The measure of success is that
   `xtask forms port-status 2027` is a command, and that every gate in §2 takes its year from an
   enumerable artifact rather than a literal.

**What did NOT change.** The IRS calendar (finals Nov 2026 – Feb 2027; `i1040gi--2026` and
`i6251--2026` last, ~Jan – Feb 2027), the OTS 2026 cadence (preliminary ~2027-01-27), and the rule
that a draft is evidence and never a transcription source.

---

## 2. ★ THE SCAFFOLDING GAP

**This section outranks everything else in the document.** It is the class that ships wrong numbers
silently, and it is the reason the target change does not make the work smaller.

### The two-sided finding, stated once

The seams lens and the instruments lens reported what look like opposite headlines, and they are the
same fact seen from either side (adjudicated in §7, D6):

> **The product fails CLOSED. The instruments fail OPEN.**

All 34 forms-layer lookups (17 `*_pdf(year)`, 17 `Map::for_year(year)`) return
`Err(FormsError::UnsupportedYear)` for an unenumerated year — so nothing hands a filer a TY2024
PDF under a TY2026 heading. But the gates that would *catch* a bad port do not skip the new year:
**they re-check TY2024, find it correct, and report success.** `field_census.rs`'s documented
*"★★★ THE GATE"* runs on `let year = 2024;`. `DEFAULT_ROW_YEAR = "2024"` binds 273 of 323
line-coverage rows. `shipped_tables_are_the_validated_tables.rs` validates `table_for(2024)` while
the binary ships four years.

And the refusals are excellent for a reason nobody chose: **adding a year is ~85 hand-edits of code
and 0 edits of data**, so TY2025 was never wired, so the refusals still fire. `map.rs:3-4` asserts
the opposite as fact — *"adding a year is a `forms/<year>/` directory (PDF + maps), never a code
change"* — while ten complete, audited, census-carrying TY2025 maps sit on disk reachable from
nothing (package F1, machine F6, seams F1).

### Classes

| class | meaning |
|---|---|
| **REFUSES** | the year is enumerated; an unenumerated year gets `Err` / `None` / a panic naming the fix |
| **ABSENT** | no year parameter exists, so the site is pinned and cannot move. The failure is a stale value or a missing gate — visible as a literal |
| **FALLS BACK** | an unprepared year silently receives another year's **data, label, or verdict** — including the verdict *"pass"*. **This is the class that ships wrong numbers** |

**Measured: 32 FALLS BACK sites.** They are the entire content of §2.1–§2.6. The ABSENT and REFUSES
rows are listed alongside because their *shape* is what turns into a fallback the moment a `2026 =>`
arm is typed — §2.7 names the four that convert.

---

### 2.1 Geometry and delta — the port's own instruments, blind on exactly the artifacts the port reads

Every TY2026 draft carries an IRS `Caution: DRAFT—NOT FOR FILING` cover sheet as physical page 1.

| # | site | class | what an unprepared year gets |
|---|---|---|---|
| **1** | `crates/xtask/src/form_geometry.rs:228-234` — `page_of` derives a box's page from the FQN segment `PageN[0]`; `:112-130` derives a *word's* page from the PDF ordinal | **FALLS BACK** | every box joined against the **previous page's** text. Measured: all 16 archived drafts have `max(box.page) == pages - 1`; all 33 finals have `boxmax == wordmax == pages`. `f6251--2026-DRAFT` yields `Page2[0].f2_2[0] -> label 1a` — page 2 is Part III, lines 12-40 (machine F1, compute C-2) |
| **2** | `crates/xtask/src/form_delta.rs:76-86` — `label_moved` skips any pair where either label is `"?"`; `labels_available` means only *a fixture exists on each side* | **FALLS BACK** | the **cleanest possible verdict from zero comparisons**. `form-delta f8959--2025 f8959--2026-DRAFT` prints *"no field changed the printed line it sits beside"* having resolved **0 of 26** labels; f1040sb 0 of 72; same for f8960, f8995, f1040s3 (machine F2) |
| **3** | `crates/xtask/src/label_reader.rs:456` — `stem.rsplit("--").next().unwrap_or("2025")` with **no** `.trim_end_matches("-DRAFT")`, unlike its sibling `form_geometry.rs:191-195` | **FALLS BACK** | a nonexistent directory, and an error blaming a gitignored missing PDF that is present. Measured: `label-proof` **OK=0 FAILED=16** on `design/forms/2026/*-DRAFT.pdf`; `label-proof f6251--2025` succeeds. This is the *human-in-the-loop* instrument whose banner reads *"a defect the machines could not see"* (seams F2) |
| **4** | `label_reader.rs:456` **and** `form_geometry.rs:194` — `unwrap_or("2025")` | **FALLS BACK (dead)** | nothing: `"f1040".rsplit("--").next()` is `Some("f1040")`, never `None`. It **reads as a deliberate fallback-to-2025 policy and is not one**, in the two places most likely to be copied when a new stem-consuming command is added (seams F2) |
| **5** | `label_reader.rs:1071-1079` (`unwitnessed` reported by `eprintln!`, never asserted) and `:1061` (`assert!(checked >= 151)`, one **global** count across all years) | **FALLS BACK** | a TY2026 whose geometry was never generated contributes **0 joins** and the gate stays green on 2024+2025 coverage (instruments I-2) |
| **6** | `label_reader::every_map()` `:1013` builds the stem as `format!("{form}--{y}")` — the *map* spelling — while the fixture is committed under the IRS spelling | **FALLS BACK** | **30 line→label joins silently unreachable** whose fixtures are on disk: `f1040sd--{2024,2025}.json`, `f1040sse--{2024,2025}.json` vs maps named `schedule_d`, `schedule_se` (3+3, 12+12 bindings). **Schedule D is where btctax's capital gain lands, and no committed test joins any of its lines to its printed label, in any year** (instruments I-2, I-5) |
| **7** | `scripts/archive_drafts.py:85` — `printed_year` takes `max()` of every year on pages 1-3 | **FALLS BACK** | **`design/forms/2026/f1040--2026-DRAFT.pdf` is the TY2025 Form 1040** (masthead *"1040 U.S. Individual Income Tax Return 2025"*, footer *"Form 1040 (2025) Created 9/5/25"*). The only "2026" in it is line 36, *"applied to your 2026 estimated tax"* — which every 1040 carries. The module's own docstring credits the refusal it did not make (compute C-1) |

**Consequence, stated plainly:** `design/TY2026_WORK_LIST.md`'s entire *"lines that moved"* column is
an artifact of #1, and its four *"unchanged / mechanical"* rows are false greens from #2. So are the
evidence sentences inside the two panics landed today —
`crates/btctax-forms/src/form1040.rs:34-58` and `schedule_se.rs:37-56` both cite *"the 2026 drafts
move 31 line bindings while renaming nothing."* **The panics are correct and must stay; their
justification must be recomputed** (machine F4).

### 2.2 The forms/asset layer

| # | site | class | what an unprepared year gets |
|---|---|---|---|
| — | 17 × `*_pdf(year)` (`pdf.rs:73…213`), 17 × `Map::for_year(year)` (`map.rs:186…2077`) | **REFUSES** | `Err(UnsupportedYear)`. 11 of 17 PDFs and 12 of 17 maps accept **2024 only** |
| **8** | `crates/btctax-forms/src/form8283.rs:70-77` — `sec_clusters(year, section)` ends `(_, Section::A) => SEC_A_CLUSTERS_2023`, `(_, Section::B) => SEC_B_CLUSTERS_2023` | **FALLS BACK (latent)** | the **third** geometry wildcard; `f1040_clusters` and `se_clusters` were enumerated today and this one was missed. Its refusal is **borrowed** from `Form8283Map::for_year` (`map.rs:880`) and `f8283_pdf` (`pdf.rs:191`) — exactly the property that made the other two safe right up until they were not. Adding `2026 =>` to those two enumerations (the first thing a year package does) hands the Rev. 12-2026 form the **Rev. 12-2023** column geometry. **The clusters are what `verify_flat` reads back against, so a wrong set does not fail the read-back — it redefines it** (seams F3) |
| **9** | `crates/btctax-forms/src/map.rs:1005-1014` — `Form8275Map::for_year` is the **only** one of 16 constructors that aliases rather than refuses: `2017 \| 2024 \| 2025 => { let mut m = Self::ty2024(); m.year = year; Ok(m) }` | **FALLS BACK (by design)** | TY2024's map, stamped with the requested year. Defensible for a Rev. 01-2021 non-year-dated form — but **the guard is a year list, so `\| 2026` is a one-token edit** and nothing contradicts it (instruments I-6) |
| **10** | `crates/btctax-forms/tests/sp4.rs:394-403` — iterates `SUPPORTED_YEARS` correctly, then compares **every** year against `fieldset(F8275_PDF_2024)` | **FALLS BACK** | a pass. A year whose Form 8275 revision changed cannot be seen; the test reads as year-general and is not (instruments I-6) |
| **11** | `crates/btctax-forms/src/map.rs` — **no `#[serde(deny_unknown_fields)]` anywhere** | **FALLS BACK** | silence. Machine-diffed `Form6251Map` vs `forms/2025/f6251.map.toml`: `line1` missing ⇒ parse error (loud), `line1a`/`line1b` present ⇒ **silently discarded**. Only a *vanished required* key is loud; a **renamed** line is dropped without a word. Still open from the 2026-09-05 map AUDIT, IMPORTANT-1 (package F4) |
| **12** | `crates/btctax-forms/tests/field_census.rs:105` `let year = 2024;` (also `:193` `.join("2024")`, `:261`) | **FALLS BACK** | **a pass.** The documented *"★★★ THE GATE"* — `(map FQNs) ∪ (census FQNs) == PDF field set` — runs on TY2024 only. The ten TY2025 `[census]` sections are checked by nothing; adding TY2026 makes it re-check 2024 and report green (package F3, R15) |
| **13** | `crates/xtask/src/cite_check.rs:756` — `authority_coverage_may_only_improve` collects `archived` as `BTreeSet<&str>` of `f.form`, **discarding `FormAuthority::year`** (`:654`) | **FALLS BACK** | discharge. `FORMS` holds one row, `f1040s1a` year 2025 — the one form the work list measures as most changed — so a TY2025 archive satisfies the TY2026 obligation and `cite-check` keeps verifying the spec against the TY2025 booklet (instruments I-4) |
| **14** | `crates/xtask/src/form_geometry.rs:67` `pub pdf_sha256` — written at generation time, compared to `MANIFEST.json` by **nothing** | **FALLS BACK** | a stale fixture surviving an IRS revision, which is precisely how a revised authority gets absorbed silently. Measured 48/48 matching today, and no test says so (package F7) |
| — | `crates/btctax-forms/forms/2025/` — 20 of 32 committed assets (10 forms) referenced by no `include_bytes!`/`include_str!` | **REFUSES — wrongly** | the INVERSE of the class: a **prepared** year gets `UnsupportedYear` while its correct map sits beside it. TY2017 and TY2024: zero unreferenced (seams F1, machine F6, package F1) |
| — | `crates/btctax-forms/tests/map_pdf_conformance.rs:193` `for unmapped in [2023, 2025, 2026]` | **ABSENT** | a hand-list covering 2 of 17 forms (`Form6251Map`, `Form8995AMap`). For the other fifteen a new year is neither required to refuse nor required to resolve. Its *direction* is right — wiring 2026 turns it red — so it is a tripwire, not a defect (package F10, instruments I-10) |
| — | `crates/btctax-forms/src/lib.rs:68` `SUPPORTED_YEARS = &[2017, 2024, 2025]` | **ABSENT** | a coarse gate that asserts more than it checks. TY2017 is a fully wired, shipped year with **0** notes, **0** manifest entries, **0** extracts, **0** geometry fixtures and **0** census sections. Nothing tests (SUPPORTED_YEARS × forms) (package F9) |
| — | the 17-form registry exists **four times** as independent hand-lists — `pdf.rs` consts, `map.rs` consts, `tests/common/mod.rs:16 CENSUS_KEYS` (17, map spelling, no `f1040s1a`), `cite_check.rs:678 EMITTED_FORMS` (16, IRS spelling, no `f8995a`) — none derived from another | **ABSENT** | drift. Direct cause of #6's 30 missing joins, and of R16 (package F11, instruments I-5) |
| — | `design/forms/extract/*.txt` (64 files) — read by `line_coverage_check.rs:565`, `capital_loss_carryover_check.rs:31`, `prompt_check.rs:56-99`, `label_reader.rs:648` | **ABSENT** | no integrity pin and no regeneration command. `Entry` (`authority_manifest.rs:91-107`) hashes the **PDF**; `verify()` at `:290` checks the extract merely *exists*. A hand edit moves every citation assertion with it, permanently (package F6) |
| — | `authority_manifest.rs:543-544` resolves an extract only at `design/forms/extract/<stem>.txt`, while `archive_drafts.py:127` writes the text layer to `design/forms/2026/<stem>--2026-DRAFT.pdf.txt` — the filename `design/forms/README.md` reserves for the provenance **note** | **ABSENT** | all 16 TY2026 manifest entries carry `"extract": ""`. A year whose text layer exists reads as *not extracted*; the same suffix means a 736-byte note in 2024/2025 and a 9,494-byte `pdftotext` dump in 2026 (package F5) |

### 2.3 The compute layer

| # | site | class | what an unprepared year gets |
|---|---|---|---|
| **15** | `crates/btctax-core/src/tax/return_1040.rs:2493` — `line1_rule: Form6251Line1Rule::Y2024` is a **literal** inside `form6251_inputs_from_parts` (`:2476`), which takes neither `year` nor `params`; its only caller `assemble_absolute` (`:2277`) holds both and passes `&params.amt` two lines later | **FALLS BACK** | the TY2024 Part I production for any year. Wrong AMTI → wrong TMT → wrong 1040 L17/L24, silently. **The comment above it claims "the year lives at THIS call site" — false.** Zero tests in `crates/*/tests/` mention `line1_rule` (R1, compute C-10, machine F-list) |
| **16** | `crates/btctax-core/src/tax/tables.rs:1088-1089` — `schedule_1a_params` accepts `2025..=2028` and returns identical values; `schedule_1a.rs:379-382,416-551` bakes the TY2025 line numbers into field names and 52 emitted label literals | **FALLS BACK** | `Schedule1A::compute(2026, …)` yields lines `4a…38` where TY2026 is `4a…44`, and `line_13b` (`:1363`) returns `part6.line38` where TY2026 needs line 44. `year` is consumed **only** by the params lookup (R2, compute C-8) |
| **17** | `crates/btctax-core/src/tax/tables.rs:1147-1159` — the 2025..=2028 guard `assert_eq!`s `Schedule1aParams` field-identity | **FALLS BACK** | it **enforces** sameness rather than checking it, so it reds **on the correction, not on the defect** — and the one Schedule 1-A datum that genuinely moves (Part V *"born before January 2, 1961"* → *"1962"*) **has no field in the struct at all** (R10, compute C-9) |
| **18** | `crates/btctax-core/src/tax/amt.rs:129 dec!(0.25)`, `:141 dec!(0.26)` — inlined where `AmtParams::exemption_phaseout_rate` and `rate_26` already exist and are read correctly throughout `form6251.rs` | **FALLS BACK** | the TY2024 rate. The TY2026 draft Form 6251 line 5 table (exemption 70,100 MFS; phase-out start 500,000; zero point 640,200) implies **0.50** — the literal is **half** (R9, compute C-3) |
| **19** | `crates/btctax-core/src/tax/qbi.rs::Form8995Lines::line15`, read by `printed.rs:710` as 1040 line 13 | **FALLS BACK** | the **pre-minimum** §199A figure. TY2026 makes line 15 an intermediate, adds line 16 *"Minimum deduction for active qualified business income"* (uncollected — not on btctax's input surface), and moves the deduction to line 17 as `max(15,16)`. The carryforwards documented *"line 16"* / *"line 17"* (`return_1040.rs:1608,1611`) become 18/19 — **and they are written back to next year's return** (compute C-6) |
| **20** | `crates/btctax-core/src/tax/printed.rs::ScheduleALines.line17`, taken as the itemized total | **FALLS BACK** | TY2025's line number. TY2026 moves the total to **line 18** behind a new §68-style gate (*"Is 1040 line 11b minus 13a and 13b more than $384,350?"*) with an **Itemized Deductions Worksheet**, enumerates the former free-text line 16 as 17a–17k/17z, and moves charitable to a new **Charitable Contribution Limitation Worksheet** at line 13. `AbsoluteReturn.itemized_deduction` is documented *"Schedule A line 17"* (compute C-7) |
| **21** | `crates/btctax-core/src/tax/printed.rs` as a whole — 107 line-numbered fields across 7 structs, **zero references to `year` in non-test code** | **FALLS BACK** | the printed layer's line numbering is decided by nothing a year can change. This is the structural root of #19 and #20 (compute B2 row 26) |
| **22** | `crates/btctax-core/src/tax/capital_loss_carryover.rs:74-169` — 8 `pub const &'static str` hardcoding "2024"/"2025", quoted verbatim into two filer-facing prompts at `questions.rs:733,762`; `SOURCE_EXTRACT` pinned to `i1040sd--2025.txt` | **FALLS BACK** | a TY2026 filer is shown *"Enter the amount from your **2024** Form 1040 … line 15"* — **a year off by one in a sworn-testimony prompt.** The worksheet is genuinely year-relative, so the fix is a two-year format, not a second copy (seams F6) |
| — | `crates/btctax-core/src/tax/return_1040.rs:2073-2074` — `false, false` for taxpayer/spouse senior qualification | **ABSENT** | a **hardcoded answer to a question the filer was never asked**. Direction is safe (overstates tax). `born_early_enough(dob, year)` (`:90`) already exists and derives 1961/1962 correctly, so closing this **ports the TY2026 Part V cutoff in the same edit** (R13, compute C-5) |
| — | `crates/btctax-core/src/tax/qbi_a.rs` — 43 line-numbered fields across 4 structs; `design/forms/extract/` holds only `f8995a--2024.txt` | **ABSENT** | no comparison at all. Form 8995-A is **unmeasured for both 2025 and 2026** despite a TY2026 draft being archived (compute C-6) |
| — | `crates/btctax-core/src/conventions.rs:17,19` `TRANSITION_DATE`, `TY2025_RETURN_DUE` | **ABSENT** | one date each, no year parameter. Consumed at `resolve.rs:1668`. **Under the new target this is a required data change, not a latent one** (R14) |
| — | statutory constants verified year-independent and deliberately excluded: `MEDICAL_FLOOR_RATE`, `SE_6017_FLOOR`, `SCHEDULE_B_THRESHOLD`, `QBI_RATE`, `MEDICARE_EMPLOYEE_RATE`, `FORM_8283_THRESHOLD`, §1(h) 15/20 %, §170(b) 60/50/30 %, §24 5 % phase-out | **REFUSES / n-a** | a `dec!(≥1000)` scan over all non-test source found **no indexed value living outside a per-year table** (seams, compute B2) |

### 2.4 Conformance instruments outside the forms crate

| # | site | class | what an unprepared year gets |
|---|---|---|---|
| **23** | `crates/btctax-core/src/tax/line_coverage.rs:55` — `DEFAULT_ROW_YEAR = "2024"` | **FALLS BACK** | **a pass.** Measured: **exactly one** non-test `quoting_year(` call in the file (`:2993`, `"2025"`); **273 of 323** production `c.line(`/`c.exception(` rows fall through to the default, and `line_coverage_check.rs:560` resolves the authority as `format!("{}--{}", e.form, e.year)`. This is the direct enforcement of *"transcribe, never paraphrase"*, and 273 rows of it are verified against the TY2024 booklet. The constant's own doc records that this already happened once (instruments I-1, seams F4) |
| **24** | `crates/xtask/src/prompt_check.rs:56,62,68,74,80,86,92,99` — eight `Clause.extract` string literals: 5 × `i1040gi--2025.txt`, 3 × `i8615--2025.txt` | **FALLS BACK** | a pass, printed as *"OK — {passed} assertions, all verbatim"* (`:223`). **These are the strings the TUI shows a filer immediately before an answer becomes sworn testimony** (instruments I-8) |
| **25** | `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs:50,211,260` — `table_for(2024)` / `full_return_for(2024)` | **FALLS BACK** | a pass covering one of four shipped years. Measured: **260** references to `ty2024_params`/`ty2024_table` across `crates/`, **zero** to any other year; `testonly.rs` defines only `ty2024_params` (`:53`) and `ty2024_table` (`:111`). The file's own doc names the class it exists to close — *"an assurance surface that does not touch the artifact it is claimed to assure"* — and that sentence is now true of it for two of four years (instruments I-3) |
| **26** | `crates/btctax-forms/tests/kats.rs:382-389` — the Schedule D line-17 on-state expectation ends in `(_, true) => "1"`, `(_, false) => "2"` | **FALLS BACK (latent)** | the same wildcard shape as the two cluster fns fixed today, in a test. Never exercised for 2026, because the four loops at `:391/:452/:474/:487` hand-list `[2017, 2024, 2025]` (instruments I-10) |
| — | `crates/xtask/src/schedule_1a_membership.rs:134,157` — `printed_labels("f1040s1a--2025")` | **ABSENT** | the best-designed census in the repo (two witnesses adjudicating 50 printed labels into 48 entry lines + 2 headings, tied to `Schedule1A::leaves` by an exhaustive destructure) — **pinned to the one form that gets restructured.** It cannot become year-general until `Schedule1A` does (instruments I-9) |

### 2.5 The refusal surface — what a filer actually sees

Measured against the prebuilt binary on a scratch vault; exit codes pasted, not described.

```
TY2017  report=0  export-irs-pdf=0  export-snapshot=0
TY2024  report=0  export-irs-pdf=0  export-snapshot=0
TY2025  report=0  export-irs-pdf=0  export-snapshot=0
TY2026  report=0  export-irs-pdf=2  export-snapshot=0
TY2027  report=1  export-irs-pdf=2  export-snapshot=0
TY2099  report=1  export-irs-pdf=2  export-snapshot=0
```

Four year-sets govern four entry points and **no surface names more than one of them**:
L0 `BundledTaxTables` **{2017,2024,2025,2026}** · L1 `SUPPORTED_YEARS` **{2017,2024,2025}** ·
L2 maps on disk **2017:5, 2024:17, 2025:15, 2026:0** · L3 `full_return_for` **{2024}** ·
`export-snapshot` **unbounded**.

| # | site | class | what an unprepared year gets |
|---|---|---|---|
| **27** | `btctax-tui/src/app.rs:193 selected_year: 2025`; `btctax-tui/src/unlock.rs:229 .max().unwrap_or(2025)` | **FALLS BACK** | TY2025. A new user opening an empty vault in 2027 lands on a paused year (refusal F6) |
| — | `btctax-cli/src/cmd/tax.rs:199` — `income import` is a bare `return_inputs::set(...)` with no table/params lookup, where the TUI's writer returns `CommitOutcome::NoTables` (`input_form_store.rs:309-310`) and its doc at `:295` says the write must be skipped so a commit *"never poisons the year at `resolve`"* | **ABSENT (gate)** | **`income import --year 2026` exits 0 and takes `report --tax-year 2026` from exit 0 with a full figure block to exit 2 — then prescribes `income clear` (a bare `delete`, no draft fallback, `cmd/tax.rs:346-354`), i.e. deleting the filer's W-2s.** Under the new target this is the *normal* entry path, not an edge (refusal F1). **The single highest-cost defect on this surface** |
| — | `btctax-cli/src/cmd/admin.rs:129 export_snapshot` — has a BG-D8 gate and a pseudo-attestation gate and **no year gate**; `export-irs-pdf` got one at `admin.rs:678` | **ABSENT (gate)** | `--tax-year 2099` writes `form8949.csv` + `schedule_d.csv` and exits 0; `diff snap_2026/form8949.csv snap_2099/form8949.csv` is **empty**. The CSVs carry no `tax_year` column, so a TY2026 and a TY2099 preparer handoff are indistinguishable artifacts (refusal F2) |
| — | `btctax-core/src/state.rs:86,88` — `BlockerKind` holds only `TaxProfileMissing` and `TaxTableMissing`, both `Severity::Hard`, both meaning *no number* | **ABSENT** | **there is no reason code for "a number, but no form"**, so L1/L2/L3 readiness has nowhere to render on any surface. `report --tax-year 2026` exits 0 with a complete crypto-delta block structurally identical to TY2025's filable output; `verify` says *"Hard blockers: 0"* (refusal F3) |
| — | `btctax-tui/src/tabs/forms.rs:61-76,123` renders `Form 8949 — 2026`; `SUPPORTED_YEARS` appears nowhere in `btctax-tui/`; `export.rs:128,169` writes the CSVs ungated | **ABSENT (gate)** | on a tab literally named **Forms**, a year with no bundled forms shows a populated form (refusal F5) |
| — | five to six user-facing supported-year sentences as literals: `btctax-forms/src/error.rs:11` (*"2017, 2024 and 2025"*, **57 lines below** the constant), `btctax-cli/src/cli.rs:208` (clap help), `resolve.rs:222`, `cmd/tax.rs:822`, `cmd/tax.rs:896`, `cmd/admin.rs:970`, `form1040_full.rs:57`, `schedule_d_full.rs:77`, plus `LIMITATIONS.md:3` and `:304` — **already stale today** | **ABSENT** | the refusal fires correctly and names the wrong year. Grep for the message text returns exactly one hit: the definition. **No test reds when `SUPPORTED_YEARS` gains 2026.** Same shape as the six stale refusal descriptions folded in `3d01b5e3` (refusal F4, seams F5) |
| — | `btctax tax-profile --year 1899` → exit 0, *"Tax profile for 1899 saved."*; the TUI enumerates its year list from exactly that set (`unlock.rs:183`) | **ABSENT** | no bound at the input layer. No wrong number escapes (`report` refuses at L0), but the vault accumulates years no gate ever looked at (refusal F7) |

### 2.6 The oracle harness

| # | site | class | what an unprepared year gets |
|---|---|---|---|
| **28** | `scripts/oracle/gen_goldens.py:199,232,321` — `year: int = 2024` as a **Python default argument** on `_taxcalc_row`, `taxcalc_run`, `_taxcalc_amt_credits`; `:518` writes `"tax_year": 2024` as a literal | **FALLS BACK** | TY2024 taxcalc constants, from a regeneration that merely omitted an argument (instruments I-7) |
| **29** | `scripts/oracle/ots_direct.py:79` — `OTS_YEAR = int(os.environ.get("OTS_YEAR", "2024"))` | **FALLS BACK** | the TY2024 tree, from an unset env var (instruments I-7) |
| **30** | `scripts/oracle/verify_f6251.py:68` — `DEFAULT_FIXTURE_YEAR = 2024` | **FALLS BACK** | TY2024 for any vector with no `year` key. Documented and deliberate today — and every committed vector relies on it (instruments I-7) |
| **31** | `scripts/oracle/ots_direct.py:658-665 version()` returns `"OpenTaxSolver 2024 …"` regardless of `OTS_YEAR`; it feeds `gen_goldens.py:511 oracle_1_version`, and **SPEC §11 gates golden regeneration on that string** | **FALLS BACK** | *"no change"* for a genuine engine-year change. The version gate is blind to the exact transition it guards (R21) |
| **32** | `scripts/oracle/verify_schedule_1a.py::_rows()` returns the latest row **at or before** the requested year | **FALLS BACK** | an earlier year's parameters, printed as **OK**. Correct semantics for a step function; wrong as a year gate (R26) |
| — | `crates/btctax-core/tests/goldens/full_return_goldens.json` — `_provenance.tax_year: 2024` over 106 households | **ABSENT** | nothing. No code in `golden_returns.rs`, `golden_packet.rs` or `oracle_sweep_readback.rs` reads `tax_year` or `_provenance`; the corpus **cannot tell the Rust side which year it is**, while `golden_returns.rs:38` imports `ty2024_params, ty2024_table` as the basis (instruments I-7) |
| — | `scripts/oracle/sweep.py:89,92,96` — `OASDI_BASE = 168_600`, `STD_DEDUCTION = corpus.STD_DEDUCTION_2024`, the TY2024 §199A ceiling, all module-level | **ABSENT** | no year axis at all. The model to copy is two files away: `corpus.py:123-132 salt_for(year)` refuses an unknown year **and** asserts the axis still straddles that year's §164(b) cap (instruments I-7) |

### 2.7 The four ABSENT sites that become FALLS BACK on the first `2026 =>` arm

Named separately because they are the ones a year package *arms*, and each is one token:

1. **`sec_clusters`** (#8) — arm `Form8283Map::for_year` and `f8283_pdf` and the Rev. 12-2023
   geometry goes live against a Rev. 12-2026 form, with the read-back redefined rather than failed.
2. **`Form8275Map::for_year`** (#9) — `| 2026` in an enumerated list, with `sp4.rs` (#10) unable to
   contradict it and `archive_drafts.py` having correctly refused the f8275 draft as a wrong-year
   document, so there is **no TY2026 evidence in the repo to contradict it with**.
3. **`field_census.rs`** (#12) — extend the year literal and the gate re-checks 2024 and passes.
4. **`kats.rs`'s on-state wildcard** (#25) — add 2026 to any of the four loops and the wildcard
   answers for it.

---

## 3. What a YEAR PACKAGE is

**Nine artifacts per form. TY2025 built seven of them for ten forms and zero of the last two — and
the two it skipped are the only two the compiler can see.** Traced end to end on Form 8959, TY2025
(package §1), the easiest possible port:

| # | slot | artifact for (f8959, 2025) | committed? |
|---|---|---|---|
| 1 | authority PDF | `design/forms/2025/f8959--2025.pdf` (71,969 B) | no (gitignored, by design) |
| 2 | provenance note | `design/forms/2025/f8959--2025.pdf.txt` (736 B) | yes |
| 3 | manifest entry | `design/forms/MANIFEST.json` (31 entries for 2025) | yes |
| 4 | form text layer | `design/forms/extract/f8959--2025.txt` | yes |
| 4b | instructions text layer | `design/forms/extract/i8959--2025.txt` | yes |
| 5 | geometry fixture | `design/forms/geometry/f8959--2025.json` (1,063 words, 26 boxes) | yes |
| 6 | bundled template | `crates/btctax-forms/forms/2025/f8959.pdf` | yes |
| 7 | field map + census | `crates/btctax-forms/forms/2025/f8959.map.toml` (137 lines) | yes |
| **8** | **asset binding** — `include_bytes!` const + `f8959_pdf(2025)` arm | — | **ABSENT** |
| **9** | **map binding** — `include_str!` const + `ty2025()` + `for_year(2025)` arm | — | **ABSENT** |

**And the split is exactly inverse to census coverage.** Every TY2025 map that carries a `[census]`
carries no wiring; every wired TY2025 map carries no census (`2025/f1040.map.toml` is **10 lines**;
the 2024 one is 293). *"TY2025 is partly done"* is not a gradient — it is **two different
definitions of the artifact, and the complete definition is the one plugged in to nothing.**

### The runbook, in dependency order

**M** = a machine can do it from `(stem, year)` plus committed artifacts. **M\*** = mechanical by
nature, manual today (no committed command does it). **H** = someone must read the form.

| # | step | tag | today |
|---|---|---|---|
| 1 | Decide the year's form list: does this stem exist for this year, under this name? | **H** | `f1040s1a--2024` correctly does not exist (Pub. L. 119-21); `f8275` → `f8275r` for 2025. Only the IRS page distinguishes a gap from a rename |
| 2 | Fetch the authority PDF — `irs-prior/{stem}--{year}.pdf` | **M\*** | no committed script; `archive_drafts.py` covers drafts only |
| 3 | Write the `.pdf.txt` provenance note (URL, sha256, bytes) | **M\*** | hand-written; format fixed by `design/forms/README.md` |
| 4 | Add/refresh the `MANIFEST.json` entry | **M** | `xtask authority-manifest --regenerate` |
| 5 | Name the instructions document **and its page range** | **H** | mechanical for 12 of 17; `f1040sa` → `i1040sca` is a lookup, and the four `i1040gi`-hosted schedules need a range — `instr_pages: Some((101, 110))` was a person reading a 110-page booklet |
| 6 | Fetch + note + manifest the instructions | **M\*** | as 2-4 |
| 7 | Extract **both** text layers — `-layout` for a form, plain for 3-column instructions | **M\*** | rule at `cite_check.rs:484`; wired only for the 1-row `FORMS` registry, so the 64 committed extracts have **no regeneration command** |
| 8 | Hash the extract **into** the manifest | **M\*** | absent — §2.2 |
| 9 | Generate the geometry fixture | **M** | `xtask extract-geometry` — **broken on drafts**, §2.1 #1 |
| 10 | Assert `geometry.pdf_sha256 == MANIFEST.sha256` | **M\*** | 48/48 hold; no test says so |
| 11 | Copy the PDF byte-for-byte to `crates/btctax-forms/forms/{year}/{crate_stem}.pdf` | **M** | needs a 2-row alias table (`f1040sd`→`schedule_d`, `f1040sse`→`schedule_se`) |
| 12 | Dump the AcroForm field inventory | **M** | `xtask dump-fields` |
| 13 | Prior-year delta on **both** axes (names, line bindings) | **M** | `xtask form-delta` — label axis blind on drafts, §2.1 #2 |
| 14 | Enumerate the printed line set **from the extract** | **M** | `xtask label-census` |
| 15 | Join line → field with both witnesses, flag every disagreement | **M** | `label_reader.rs`; agreement is machine-checkable |
| 16 | **Adjudicate wherever the two witnesses disagree** | **H** | f8959 lines 5/9/15 sit at the bottom of three-row filing-status blocks; the nearest-label-above witness is off by one on exactly those three, silently and plausibly |
| 17 | **Decide whether a moved line is the SAME line** | **H** | f6251 `line1` → `line1a`/`line1b` is a split, not a rename; TY2026 Schedule 1-A keeps 10 of 219 fields |
| 18 | Diff the `.map.toml` key set against the `*Map` struct field set, **both directions** | **M\*** | the check that would have caught `line1a`/`line1b` at commit time instead of at wire-up time (§2.2 #11) |
| 19 | **Amend the `*Map` struct** when the line set changes; choose alias / new field / refusal | **H** | the struct is a **per-YEAR artifact wearing a per-FORM name** — this is the single thing that makes a year a code change |
| 20 | **Transcribe each mapped line's instruction text verbatim as its doc comment** | **H** | the *check* (comment ⊆ that line's extract text) is mechanical: 17/17 on f8959 |
| 21 | **Write the `[census]`** — `rule` + `reason` for every unmapped field | **H** | measured volume: **551 reasons / 125,331 B** at TY2024; **242 / 55,329 B** at TY2025 |
| 22 | Emit the five code bindings | **M\*** | `include_bytes!`, `*_pdf` arm, `include_str!`, `tyYYYY()`, `for_year` arm — **85 hand-edits for a 17-form year** |
| 23 | Run the gates: map ⊆ PDF, census union, doc ⊆ extract, geometry-sha = manifest-sha, `*_pdf`/`for_year` year-sets identical | **M** | **1 of 5 is year-generic today** (`map_pdf_conformance.rs`) |
| 24 | **Decide whether a line's MEANING changed while its number did not**, and whether that puts the year out of scope | **H** | no delta tool reports this axis, and it is the one that silently produces a wrong number |

**The honest ceiling: 8 of 24 steps require someone to read a form** (1, 5, 16, 17, 19, 20, 21, 24),
and they are load-bearing — steps 16, 17 and 24 are precisely where this repo's year-port defects
have actually come from. **Two structural changes turn most of the rest into data:** (a) make the
year package a **table**, one `FormAuthority`-shaped row per `(stem, year)` carrying crate-stem,
instructions stem, page range and the census/geometry paths, and derive `pdf.rs`, `map.rs`,
`CENSUS_KEYS` and `EMITTED_FORMS` from it instead of four hand-lists; (b) make every gate walk that
table rather than a literal, starting at `field_census.rs:105`.

**The registry to grow is already in the tree, holding one row** — `cite_check.rs:651-672`
`FormAuthority { form, year, instructions, instr_pages, extract_stem }`, the only year-as-a-**field**
table in the workspace, already driving path construction, with a doc comment that states the recipe:
*"adding a form is a table entry plus a transcription, never a bespoke project."*

### What "REBUILT" actually means — the part, not the form

The work list reads Schedule 1-A's 219→10 field survival as REBUILT. That is an **AcroForm naming**
fact. Read from the text layer instead (compute C-5), the form decomposes:

| part | TY2025 | TY2026 | verdict |
|---|---|---|---|
| I MAGI | 1, 2a–2e, 3 | same | **same** |
| II Tips | 4a–4c, 5–13 | 4a–4e × 5 cols, 6a–6e × 13 cols, 5,7,8…15 | **REBUILT — new collection tables** |
| III Overtime | 14a–14c, 15–21 | 16a–16e, 18a–18e, 17,19,20, 21–27 | **REBUILT collection, renumbered arithmetic** |
| IV Car loan | 22–30 | 28–36 | **+6 renumber** + 2 new Yes/No per VIN |
| V Seniors | 31–37 | 37–43 | **+6 renumber**, every constant unchanged; the *only* substantive move is the birth cutoff 1961 → 1962 |
| VI Total | 38 | 44 | **+6 renumber**; 1040 destination 13b → 13a (contested, §7 D5) |

**So the unit of the year decision is the PART.** A whole-form per-year struct duplicates Parts
I/IV/V/VI verbatim to express a `+6` — including their doc comments, which is a duplicated
instruction someone eventually edits on one side only. A whole-form label indirection cannot express
*"line 4 became a five-column table."* The three axes are separable and each has a right expression:

| axis | right expression | repo exemplar (good) | repo instance (bad) |
|---|---|---|---|
| **quantity** — *which number this is* | semantic field name, line number in the doc comment | `AbsoluteReturn` (`return_1040.rs:1530-1660`) — zero `lineNN` fields | `printed.rs` (107), `schedule_1a.rs` (56), `qbi.rs::Form8995Lines` (16) |
| **instrument** — *what arithmetic this line does* | variant enum **carried on the year's params bundle**, selected once at table construction | **`SaltLimitation` on `FullReturnParams`** (`tables.rs:323`, `:465`) | `Form6251Line1Rule` — right type, selection left at `return_1040.rs:2493` |
| **label** — *what number it prints as* | per-year `field → label` table read by emitter and census | `LineCoverage { form, year, line, field, instruction }` — the table already exists | `Schedule1A::leaves()` — 52 literal labels welded to TY2025 |

Decision procedure for a form in a new year: **arithmetic changed → new variant** (TY2026: Form 8995
line 17 `min`→`max`; Schedule A line 18's §68 gate; Schedule A line 13's charitable worksheet);
**only the printed number changed → new label row** (Schedule 1-A Parts IV/V/VI); **collection
surface changed → per-year struct for that part only** (Schedule 1-A Parts II/III; Schedule A
17a–17z; Part IV's two Yes/No per VIN).

**The discriminator between the second and third cases is not measured today.** `form-delta` reports
field-name churn and label drift; neither can answer *"is this the same sentence?"*. **A third axis —
whitespace-normalised instruction-text equality on corresponding lines — is what separates a
renumber from a rebuild, and every classification in this section had to be computed by hand.**

---

## 4. The port machine

One namespace, `xtask forms`, because **58 of 64 committed extracts already name it** (*"Regenerate:
`cargo run -p xtask -- forms extract`"*) and no `forms` subcommand exists (`main.rs:200-230`).

| cmd | in | out | refuses |
|---|---|---|---|
| **`forms fetch <year> [--drafts] [--stem s]`** | a year | `design/forms/<year>/<stem>--<year>[-DRAFT].pdf` (gitignored), a note carrying URL+sha256+bytes, a `MANIFEST.json` entry | the year **printed on the document** ≠ the year requested; a draft path serving a final. `archive_drafts.py` already is this — **promote it, replace `printed_year`'s `max()` with the masthead/footer year, and stop writing the text layer to `<stem>.pdf.txt`** (it collides with the note name) |
| **`forms extract <stem> \| --all`** | archived PDF + note | `design/forms/extract/<stem>.txt` with the existing generated header, using **that file's own recorded flags** (measured: 32 `-layout`, 27 none, 1 explicit) | PDF sha256 ≠ note's; a stem with no note. `--all` prints `written / unchanged / missing-pdf` |
| **`forms geometry <stem>`** | archived PDF | `design/forms/geometry/<stem>.json` **plus two new required fields**: `cover_pages: N`, derived by counting leading pages with zero widget annotations; and `page_src: "annot"\|"fqn"` **per box**, so a fixture built the weak way is greppable | `cover_pages > 0` together with any `page_src: "fqn"`; `max(box.page) != pages - cover_pages`; a word-bearing, box-free page not declared a cover page |
| **`forms delta <old> <new>`** | two fixtures + two extracts | **three axes**: field-name set diff (exists, calibrated both directions); line→label join; **coverage — `resolved a/N` per side plus the unresolved sets**; and the new **instruction-text `diff -b` per corresponding line** | `label_moved.is_empty()` may be reported as *"no line moved"* **only when both sides resolve the same field set**; otherwise print `LABEL AXIS BLIND ON n FIELDS` and **exit non-zero** |
| **`forms port <stem> --from y1 --to y2 [--allow-draft] [--emit-wiring]`** | the two year packages | a `.map.toml` **with its wiring patch**, plus `design/forms/port/<stem>--y1-to-y2.json`, the per-line worklist `carried \| moved \| orphan \| needs-transcription \| undecided`; **non-zero while any line is not `carried`** | see the pass table below |
| **`forms wire --check`** | `crates/btctax-forms/forms/*/` | asserts every `.map.toml` on disk is reachable from `map.rs` | **reds TODAY naming 10 stems.** A free B1 kill-test available *before* the generator it guards — build it first, exactly as A1 was built before its hooks |
| **`forms port-status <year>`** | the tree | per stem `carried / needs-human / absent`, plus totals | — this is what makes *"adding a year is a data change"* **a number**, and it regenerates `design/TY2026_WORK_LIST.md` from the tool instead of by hand |

**`forms port`'s passes, each of which can stop the run:**

| pass | what it does | refusal |
|---|---|---|
| **P0 provenance** | both PDFs present, sha256 = note, draft/final consulted | a draft `to` without `--allow-draft`. **With it, the emitted map carries `provisional = true` AND its generated `for_year` arm returns `UnsupportedYear`, so a provisional port cannot be loaded by construction** |
| **P1 field set** | `forms delta` axis 1 | any add/remove ⇒ names listed, every binding touching one is `undecided` |
| **P2 line→label** | re-derive every `lineN = field` from the **to** form's own fixed geometry | per binding `carried` / `moved` / `orphan`; any `moved` or `orphan` ⇒ non-zero exit |
| **P3 line text** | `diff -b` of that line's printed text between the two extracts | changed ⇒ **the doc comment is DELETED** and the line becomes `needs-transcription`. **A stale instruction quote is worse than none** |
| **P4 census carry** | a `rule`/`reason` carries **only if** name ∧ label ∧ line-text are all unchanged | otherwise `rule = "undecided"`, reason = the pasted diff |
| **P5 wiring** | emit the five edits per `(stem, year)` | a map with any non-`carried` line gets an `UnsupportedYear` arm **and a `#[test]` that reds until a human clears it** |

### Where it must stop and hand to a human

1. **Any `needs-transcription` line.** A changed instruction sentence is testimony signed under
   26 USC 6065, and every AMT defect in this repo's history was compressed instruction text.
2. **Any `undecided` census disposition.** *"unmodeled because btctax collects no RRTA income"* is a
   claim about the **engine**, not the PDF.
3. **Any `moved` binding.** The machine proposes the rebind and must not commit it. This is exactly
   the class that puts the AMT in line 10's box.
4. **A rebuilt part.** The machine says *"this is not a port"* rather than emitting a 10-line map and
   209 `undecided`s.
5. **Every figure.** The machine never writes a dollar amount. Drafts are evidence; the encodable
   sources are the final form and the Rev. Proc.
6. **Everything downstream of the map** — `Schedule1A`'s struct, the §164(b) worksheet, the Part I
   productions. The machine may emit an **empty enum variant** so the compiler enumerates every call
   site (free and exact, the enums are not `#[non_exhaustive]`) and must not fill one.
7. **The coverage floor per form.** The machine reports the number and refuses on a set mismatch; it
   does not pick a threshold.

### Build order, and why it is this order

1. **`forms wire --check`** — reds today on 10 stems, before the thing it guards exists (B1).
2. **The geometry page fix** (§2.1 #1) **paired with a kill-test that prepends a blank page to a
   known fixture and asserts the label join goes RED.** The planted defect is already on disk: the
   measurement is `label-census f8959--2026-DRAFT` yielding 24 `Amount` rows, not 24 `Heading` rows.
3. **`label_reader.rs:456`'s `-DRAFT` strip**, with a B1 test running `proof()` on a `-DRAFT` stem.
   Measurement: **0/16 → 16/16**.
4. **The map-stem ⟷ IRS-stem alias table**, recovering 30 joins that exist and are invisible.
5. **`forms delta`'s coverage axis and non-zero exit** — without it every later measurement can be a
   false green.
6. **`forms extract`** — the ② step; blocks pass P3 outright.
7. **Re-archive `f1040--2026`** and re-measure `TY2026_WORK_LIST.md` and the two panic messages.
8. Only then the `.map.toml` + wiring generator.

---

## 5. The risk register, R1–R27, re-marked for the new target

**State:** REFUSES · DORMANT (wrong code in the tree, a higher gate hides it — **the port is the
commit that opens that gate**) · UNGUARDED (no wrong code yet, and no instrument would catch it) ·
LIVE · **MOOT** (the risk was real and is retired by the pause — recorded, not deleted).

### 5a. Engine

| # | risk | old | **new** | why the mark moved |
|---|---|---|---|---|
| **R1** | `return_1040.rs:2493` hardcodes `Form6251Line1Rule::Y2024` in a fn with no `year` | DORMANT / CRITICAL | **DORMANT / CRITICAL — now #1 on the build list** | TY2026 is the first filed year, so the arming commit is the *next* one, not a later one. Confirmed independently by compute C-10 with the mechanism: the fix is a **parameter**, the caller holds `params` and `year`, and the compiler reds the site. **The end state is `SaltLimitation`'s: move the selector onto `FullReturnParams`** |
| **R2** | `schedule_1a_params(2026)` + TY2025 line numbers baked into `Schedule1A` | DORMANT / CRITICAL | **DORMANT / CRITICAL — shape now known** | Confirmed from the compute side (C-8) and **refined**: Parts IV/V/VI are a pure `+6`, Parts II/III are new instruments (§3). **Correction to the old entry:** it cited *"the TY2026 draft 6251 line 46"*; Form 6251 ends at line 40 in both years — the sentence is **line 1a** (compute C-3). And its conclusion is **overturned**: see §7 D3 |
| **R3** | no committed test joins a mapped line to the printed label beside that box | UNGUARDED / CRITICAL | **UNGUARDED / CRITICAL + the instrument is itself blind** | The join exists and was watched red-on-planted-defect — but §2.1 #1/#5/#6 mean it is off by a page on every draft, silent on unwitnessed years, and unreachable for Schedule D and Schedule SE in every year. **R3 is now two problems: the missing test, and the broken witness** |
| **R4** | 23 of 266 in-place renames are SILENT (same name, box 12–252 pt away) | UNGUARDED / IMPORTANT | **UNGUARDED / IMPORTANT — and the hop is now 2024→2026** | The shipped emitter is TY2024 and TY2025 is unwired, so the port crosses two revisions with no intermediate check |
| **R5** | Form 8949's crypto box moved C → I; TY2025's Box C reads *"other than digital asset transactions"* | UNGUARDED / IMPORTANT | **UNGUARDED / URGENT** | Was a TY2025 problem to be solved on the way. It is now the **shipped** emitter's problem: btctax's live map ticks a box that explicitly excludes crypto, on a return signed under 26 USC 6065, and the TY2025 fix is inert. f8949 measures 202/0/0 across 2025→2026, so the box set is stable — **the whole delta is the one btctax has not taken** |
| **R6** | Schedule C 27a/27b — the printed letters swapped while field names stayed pinned to content | UNGUARDED / IMPORTANT | **UNGUARDED / IMPORTANT** | Unchanged in kind, larger in size: f1040sc measures 59 common / **50 added / 46 removed** for 2025→2026. Both cells are `unmodeled` today, so no money moves; the mechanism is the risk |
| **R7** | f8949 `rows_per_page` 14 → 11, `table_token` renamed | UNGUARDED / IMPORTANT | **UNGUARDED / IMPORTANT** | Pagination and overflow are year-dependent and carried in the map; a two-revision hop carries it twice |
| **R8** | `f1040_clusters` / `se_clusters` wildcard `_ =>` | DORMANT / IMPORTANT | **CLOSED, with two successors** | Both now `panic!` naming the fix — **correct, keep**. Successor 1: **`sec_clusters` is the third wildcard and was missed** (§2.1 #8). Successor 2: **both panic messages cite a number produced by the geometry bug** and must be recomputed (§2.1) |
| **R9** | `amt.rs` `dec!(0.25)` / `dec!(0.26)` inline where `AmtParams` carries them | DORMANT / IMPORTANT | **DORMANT / IMPORTANT — magnitude now measured** | The TY2026 draft's line-5 table implies **0.50**; the literal is half. Still no production caller of `amt_should_file_6251`. The fix is mechanical and free: read the params |
| **R10** | the 2025..=2028 guard **enforces** field-identity rather than checking it | DORMANT / IMPORTANT | **DORMANT / IMPORTANT — and now with a known instance** | Compute C-9: the struct has **no birth-cutoff field**, so the one datum that moves (1961→1962) is invisible to it. The resolution is **not** to relax the test — keep the cutoff out of the struct and derive it with `born_early_enough`, because a year-general function has nothing to forget to add |
| **R11** | `ftc_ceiling` (statutory §904(j) $300/$600) inside the indexed struct with no invariant | DORMANT / MINOR | **DORMANT / MINOR** | Unchanged. One line, next to `qbi_phase_in_top()` which already guards its neighbour |
| **R12** | `dependent_std_earned_addon` was $450 in both TY2024 and TY2025 — *unchanged is not unindexed* | DORMANT / MINOR | **DORMANT / MINOR — sharper** | A TY2026 `testonly::ty2026_table()` must be transcribed **independently** of `tax_tables.rs::ty2026()` (instruments HUMAN-2), and this is exactly the cell a copy-forward pass gets wrong |
| **R13** | `return_1040.rs:2073-2074` passes `false, false` for senior qualification | LIVE (TY2025) / MINOR | **LIVE / do it now — it is one line and it is free** | `born_early_enough(dob, year)` (`:90`) already derives 1961 for 2025 and 1962 for 2026. **Closing R13 ports the TY2026 Part V cutoff in the same edit** (compute C-5) |
| **R14** | `TY2025_RETURN_DUE` is a singular constant, not a per-year function | DORMANT / MINOR | **REQUIRED / IMPORTANT** | TY2026 is the filed year. Its shape does not admit a TY2026 due date, and `resolve.rs:1668` consumes it |
| **new** | **Form 8995 line 15 → 17, with an uncollected new line 16** | — | **DORMANT / IMPORTANT** | `printed.rs:710` reads `q.line15` for 1040 line 13 — filing the **pre-minimum** figure, understating §199A and overstating tax. Carryforwards 16/17 → 18/19 are **written back to next year's return**. Line 16's *"active"* eligibility is not on the input surface (compute C-6) |
| **new** | **Schedule A total 17 → 18 behind a §68-style gate, plus two new worksheets** | — | **DORMANT / IMPORTANT** | `AbsoluteReturn.itemized_deduction` and `printed.rs` both name line 17. The $384,350 gate and the Itemized Deductions / Charitable Contribution Limitation worksheets are unmodelled (compute C-7) |
| **new** | **`printed.rs` has no `year` in scope at all** | — | **UNGUARDED / IMPORTANT** | 107 line-numbered fields across 7 structs whose numbering is decided by nothing a year can change — the structural root of the two rows above |
| **new** | **Form 8995-A is unmeasured for 2025 and 2026** | — | **UNGUARDED / IMPORTANT** | 43 line-numbered fields across 4 structs; only `f8995a--2024.txt` is on disk, and a TY2026 draft is archived. **`f8995a--2025` must be archived as the prior side of the delta** — this is the one piece of TY2025 archive work the pause does *not* make moot |

### 5b. Instruments

| # | risk | old | **new** | why |
|---|---|---|---|---|
| **R15** | `field_census.rs` year pin (`:105`, `:193`, `:261`) | LIVE / IMPORTANT | **LIVE / IMPORTANT — de-pin onto WIRED years, not all years** | Its remediation half is now **partly MOOT**: the 330 unaccounted TY2025 boxes are 330 readings for a year nobody will file. De-pinning onto the raw year set reds on **paused work rather than on a defect** (adjudicated §7 D6) |
| **R16** | `EMITTED_FORMS` omits `f8995a` | LIVE / IMPORTANT | **LIVE / IMPORTANT — fix by construction, not by adding a string** | Generalised twice: the ratchet is keyed on `form` not `(form, year)` (I-4), and there are **four** hand-lists in two spellings joined by nothing (I-5). One derived `(form, year)` inventory closes R16, the 30 missing joins and this row at once |
| **R17** | a census reason can go stale on a **zero-delta** form | UNGUARDED / IMPORTANT | **UNGUARDED / IMPORTANT — mechanically halved** | The port machine's P4 carry condition (name ∧ label ∧ text unchanged) is the machine-checkable half. The reason's claim about the **engine** stays human, permanently |
| **R18** | `FIELD_PROVENANCE.md` claims 496 unaccounted; measured 0 across 17 | LIVE / MINOR | **LIVE / MINOR** | Unchanged. ~20 lines of generator; the document's headline is wrong by its own magnitude |
| **R19** | the two moving-URL notes (`f8275r--2025`, `i8275r--2025`) point at `irs-pdf/` and flip to the TY2026 final | LIVE ON FLIP / IMPORTANT | **LIVE ON FLIP / URGENT** | **The flip is now the event we are waiting for.** `verify()` never touches the network — for `Storage::Note` it only asserts the note exists — so an operator re-fetching after the flip gets a different tax year's document with no alarm |
| **R20** | a TY2026 draft under a clean stem is indistinguishable from a final in `MANIFEST.json` | UNGUARDED / IMPORTANT | **LIVE-adjacent / IMPORTANT — with a measured instance** | 16 drafts are archived under `--DRAFT` stems, so the *filename* discriminates; the manifest still does not, all 16 carry `extract: ""`, **and the archiver admitted a wrong-year 1040** (§2.1 #7). The TY2026 Schedule 1-A draft was revised 2026-09-04 |
| **R21** | `ots_direct.py version()` returns *"OpenTaxSolver 2024"* regardless of `OTS_YEAR` | LIVE / IMPORTANT | **LIVE / IMPORTANT** | Unchanged; SPEC §11 gates golden regeneration on that string, so the version gate is blind to the exact transition it guards |
| **new** | **`DEFAULT_ROW_YEAR` binds 273 of 323 coverage rows to the TY2024 extract** | — | **LIVE / CRITICAL** | The direct enforcement of *transcribe, never paraphrase*, and **one** production call site names a year. Delete the default; make `year` required; the compiler names all 323 sites (I-1) |
| **new** | **`prompt_check`'s eight filer-facing clauses pinned to TY2025 by literal path** | — | **LIVE / IMPORTANT** | The strings shown before an answer becomes sworn testimony, verified against last year's booklet and printed as OK (I-8) |
| **new** | **`shipped_tables_are_the_validated_tables.rs` covers 1 of 4 shipped years** | — | **LIVE / IMPORTANT** | 260 references to `ty2024_params`/`ty2024_table`, zero to any other year. **The equality is vacuous until a second, independently transcribed TY2026 table exists** — copying `ty2026()` into `testonly` would satisfy the test and prove nothing (I-3) |
| **new** | **`Form8275Map::for_year` aliases by an enumerated year list, and `sp4.rs` cannot contradict it** | — | **DORMANT / IMPORTANT** | §2.7 item 2. Gate the alias on a committed `pdf_sha256`, not a year list (I-6) |
| **new** | **`schedule_1a_membership` is pinned to the one form that got restructured** | — | **DORMANT / IMPORTANT** | The best-designed census in the repo, blocked behind the `Schedule1A` shape decision (I-9) |

### 5c. Oracles

| # | risk | old | **new** | why |
|---|---|---|---|---|
| **R22** | taxcalc `AMT_em_pe` TY2026 = 639,200 vs the draft form's $640,200 | LIVE / IMPORTANT | **LIVE / IMPORTANT — file it** | Two independent witnesses in hand (the draft line-4 figure, read by two lenses; and taxcalc's own `AMT_em_ps + AMT_em/AMT_prt`). Clears the never-file-on-one-oracle bar |
| **R23** | installed taxcalc 6.7.2 understates TY2026 AMTI by the **whole standard deduction** | LIVE / CRITICAL as a witness | **LIVE / CRITICAL — now the gating oracle blocker** | TY2026 is the filed year; its second witness (OTS 2026) does not exist until ~2027-01-27; and TY2026 is a materially bigger AMT year for btctax's large-LTCG population (`AMT_prt` 0.25→0.5). **The 6.8.2 upgrade moves from Tier 3 to Tier 0** |
| **R24** | `AutoLoanInterestDed_ps` gives a QSS the MFJ threshold | LIVE (caught) / IMPORTANT | **LIVE (caught) / IMPORTANT** | Mitigated by a **computed** disqualification that re-derives itself for TY2026 with no mechanism change |
| **R25** | a year bump that does not extend `TAXCALC_EXACT_YEARS` makes taxcalc smooth the Schedule 1-A step (±$100/$200) | LIVE / IMPORTANT | **LIVE / URGENT** | A TY2026 census is now the **first** thing that runs, and the divergence will read as a btctax rounding defect |
| **R26** | `verify_schedule_1a._rows()` returns the latest row at or before the year and prints OK | LIVE / IMPORTANT | **LIVE / IMPORTANT** | Unchanged. Correct for a step function, wrong as a year gate |
| **R27** | `PT_qbid_taxinc_thd` MFS 2026 = 201,775 | LIVE (out of reach) / MINOR | **LIVE / MINOR** | Unchanged; encode as a computed disqualification, do not file on one oracle |
| **new** | **the golden corpus's own `tax_year` label is inert, and its generator defaults to 2024 in four places** | — | **LIVE / IMPORTANT** | `gen_goldens.py` × 3 defaults + one literal, `ots_direct.py`'s `OTS_YEAR` env default, `verify_f6251.py`'s `DEFAULT_FIXTURE_YEAR`; no Rust code reads `_provenance`. The model to copy is `corpus.py:123-132 salt_for(year)`, two files away (I-7) |

### 5d. What the pause makes MOOT — recorded, not deleted

| former item | why it was there | verdict |
|---|---|---|
| **`full_return_for(2025) -> Some` as *the gate*** | the old §1: no 1040 for 2026 until 2025 returns `Some` | **MOOT as a gate.** No sequencing argument survives. The *work* (a second bundled `FullReturnParams`) is now aimed at 2026 |
| **Wire the ten inert TY2025 maps** (old Tier-2 #12) | *"TY2025 is 15 of 17"* was an unexecuted claim | **MOOT as a deliverable; the TEST is not.** `forms wire --check` reds today on those ten and must ship regardless — either the orphans get wired, or their absence gets **recorded with a reason**. A third state (present, unreachable, unremarked) is the defect |
| **Fix `Form6251Map` `line1` → `line1a`/`line1b`; add `Schedule1AMap`** (old #13) | TY2025's f6251 map had no deserialization target | **MOOT for TY2025, LIVE for TY2026** — and the *general* defect (no `deny_unknown_fields`, §2.2 #11) is unchanged and is the reason it was silent |
| **Build the Schedule 1-A emitter** (old #14) | TY2025 computed Schedule 1-A and could not print it | **MOOT as TY2025 work; REQUIRED for TY2026** — and it must now be built against the **part-level** decomposition of §3, not the TY2025 whole-form shape |
| **Carry `f1040s1`, `f8275`, `f8995a` to TY2025** (old #15) | *"makes TY2026 a one-year hop"* | **MOOT for f1040s1 and f8275. NOT moot for `f8995a--2025`** — its extract is the prior side of a TY2026 delta and does not exist |
| **Archive the three missing TY2025 authorities** (old #16) | completeness of the TY2025 package | **PARTLY MOOT.** `f8995a--2025` + `i8995a--2025`: keep, per above. `f1040s1--2025`: moot unless the form is emitted |
| **The TY2025 oracle port** (old #21) | *"a TY2025 two-oracle baseline is what TY2026 gets diffed against"* | **DEMOTED, not moot.** OTS 2026 does not exist until ~2027-01-27, so **TY2025 is the only year where the harness's year axis can be exercised end to end with two live engines.** Do it as a *scaffolding* exercise — the deliverable is the parameterisation (I-7), not a TY2025 golden |
| **"Do NOT start TY2026 `FullReturnParams` before TY2025's"** (old §6 rule 10) | no gate skipped TY2025 | **REVERSED by the ruling.** Replaced in §6 |
| **The 330 unaccounted TY2025 census boxes** (old critical-path step 6) | TY2025 exact cover | **MOOT as a gate.** 330 human readings for a year nobody files. They become the reason `field_census` de-pins onto **wired** years (§7 D6) |
| **The measured TY2025 per-form floor (~40–65 lines of new justification even for a zero-diff form)** | the cost model | **Carried forward unchanged.** It is a *per-year-package* cost, not a TY2025 cost, and it is the honest floor for TY2026 and TY2027 alike |

---

## 6. What should NOT be built

1. **Do NOT encode any TY2026 figure from a draft.** Not the §55(d) resets, not `$640,200`, not the
   0.50 phase-out rate, not the Schedule 1-A amounts. Drafts are replaced in place and the TY2026
   Schedule 1-A draft was revised 2026-09-04. The encodable sources are Rev. Proc. 2025-32 §55(d),
   the amended statute, and the final form. **Knowable ≠ encodable.**
2. **Do NOT build a port machine that emits only `.map.toml`.** That reproduces exactly today's
   state — ten committed, correct, and loaded by nothing. The five wiring edits per `(stem, year)`
   are a pure function of the pair; generate them, and ship `forms wire --check` first.
3. **Do NOT build a per-binding name-existence carry-forward.** It is the intuitive relaxation of
   pass 1 and it ships **16 wrong bindings**, one of them the AMT into line 10's box.
4. **Do NOT make a geometry-only matcher the sole witness.** Schedule C's 27a/27b swap breaks
   precisely the port keyed off the printed form.
5. **★ NEW — do NOT trust `design/TY2026_WORK_LIST.md`'s "lines that moved" column, or the two panic
   messages that cite it**, until the geometry page fix lands and it is regenerated. Every number in
   that column measures a cover sheet.
6. **★ NEW — do NOT rename the sixteen `*--2026-DRAFT.json` geometry fixtures into the final slot.**
   Drafts are evidence; the final fixtures must be generated from the final PDFs.
7. **★ NEW — do NOT add `2026 =>` to `sec_clusters`, `Form8275Map::for_year`, or any `_ =>` year arm
   without opening the form.** §2.7 lists the four one-token edits that convert an ABSENT site into
   a silent substitution. One `git grep '_ =>'` over every `fn *(year: i32) -> &'static` closes the
   geometry half of the class.
8. **★ NEW — do NOT add a `Form6251Line1Rule::Y2026` variant.** Measured from the text layer, Form
   6251 does not renumber for TY2026 (§7 D3): a `Y2026` variant would be byte-identical to `Y2025`
   and would fork `amount_entering_line4`, `printed()`, `cover_form6251line1` and every exhaustive
   match for nothing. *(The empty-variant trick remains legitimate as throwaway compiler recon —
   just do not land it.)*
9. **★ NEW — do NOT de-pin `field_census.rs` onto a year set that includes unwired years.** It would
   red on 330 readings of paused work rather than on a defect. De-pin onto wired years and put the
   orphans behind `forms wire --check`.
10. **★ NEW — do NOT satisfy `shipped_tables_are_the_validated_tables.rs` by copying
    `tax_tables.rs::ty2026()` into `testonly.rs`.** A copied table makes the equality vacuous — the
    validated side must be transcribed independently from Rev. Proc. 2025-32 and Pub. L. 119-21.
11. **Do NOT edit `DUPLICATE_SOURCE_GROUPS` to get past an `f8275` red.** The red is the safe
    failure and the code comment names this exact moment.
12. **Do NOT widen a tolerance when `_amti_verdict` reds after the 6.8.2 upgrade.** Delete expected
    gap #1 and the `STANDARD_DEDUCTION` excuse table instead.
13. **Do NOT regenerate a golden to make a red green**, and **do not widen OTS's year-scoped excuse
    sets speculatively** — `{2024}` is correct until someone reads `taxsolve_US_1040_2026.c`. **No
    excuse list keyed by vector *name*;** compute the disqualification from the defect's mechanism.
14. **Do NOT re-wire the AMT screening worksheet.** `amt_should_file_6251` has no production caller.
    Fix its constants (R9) or delete it; do not re-enable a screen that can answer *"no AMT"* for a
    filer who owes it.
15. **★ REPLACES the old rule 10 — do NOT build TY2025 deliverables that only TY2025 consumes.**
    The pause is a scheduling ruling, not a licence to skip §5d: the *tests and parameterisations*
    TY2025 would have forced are exactly the scaffolding, and they ship anyway. What is dropped is
    the TY2025 **packet**, not the TY2025 **instrument work**.
16. **Do NOT chase TY2027 constants.** taxcalc extrapolates past `LAST_KNOWN_YEAR = 2026` from CPI
    growth factors, not law. TY2027 is a *scaffolding* target — `forms port-status 2027` should be a
    command; TY2027 numbers should not be in the tree.
17. **Do NOT file R24 or R27 as single-oracle upstream reports.** R22 has two independent witnesses
    and clears the bar; those two do not yet.

---

## 7. Where the lenses disagree — called, not averaged

**D1 — Is Schedule 1-A "REBUILT"? THE COMPUTE LENS WINS.**
`TY2026_WORK_LIST.md`, the machine lens and the instruments lens all treat Schedule 1-A as rebuilt on
the strength of *10 of 219 field names surviving*. The compute lens read both text layers and
measured **Parts IV, V and VI as a pure `+6` renumber of an identical instrument** — every constant
unchanged, one substantive move (the birth cutoff, which this repo already derives year-generally) —
**with Parts II and III genuinely rebuilt into multi-column collection tables.** The field-name axis
is an AcroForm *naming* fact and the text layer is the form; **the form wins.** Consequence: the unit
of the year decision is the **part**, not the form (§3), and a whole-form per-year struct is the
wrong default.

**D2 — Do Form 1040 and Form 6251 move 31 and 28 printed line bindings for TY2026? NO. THE MEASUREMENT
IS VOID.** The work list says yes; the machine lens and the compute lens independently found the
cause — `form_geometry`'s FQN-derived box page against the PDF-derived word page, off by the draft's
cover sheet. **100 % of labelled boxes "moving" is the signature of a systematic offset, not a
renumber**, and f8959/f8960 reporting *"0 moved"* came from **0 comparisons**. Called for machine +
compute. **Everything downstream is void until the fix lands**: the work list's column, its four
"mechanical" verdicts, and the evidence sentences in `form1040.rs:34-58` and `schedule_se.rs:37-56`.

**D3 — Is `Form6251Line1Rule::Y2025` reusable for TY2026? THE OLD R2 SAID NO; THE COMPUTE LENS SAYS
YES, AND IS RIGHT — with the old lens's underlying fact intact.** Read from the text layer, Form 6251
numbers identically in both years (Part I ends 4, Part II 5–11, Part III 12–40); only citations and
constants move — line 1a cites Schedule 1-A **line 43** where TY2025 cites **37**, line 7 cites 1040
line **7a**, and the line-4/line-5 figures change. So the **shape** is reusable and the *only* thing
blocking reuse is a Rust identifier that encodes another form's line number:
`Form6251Line1Rule::Y2025 { schedule_1a_l37 }`. **Ruling: reuse the variant, rename the field
semantically, and carry the cited label in the per-year label table.** Same for
`Schedule1A::line_13b`. Both are pure renames, and both name a number that moves.

**D4 — "The forms layer refuses everywhere" vs "the instruments silently re-check 2024." BOTH, AND
THEY ARE ONE FACT.** The seams lens is right that 34 of 34 lookups refuse and that the real cost is
85 hand-edits; the instruments lens is right that the worst sites do not skip the new year, they pass
it. They are the same phenomenon: **because a year is code, TY2025 was never wired; because TY2025
was never wired, the refusals still fire and the year-pinned gates have never been asked a question
they could get wrong.** Neither headline is the finding on its own. §2 is organised around the pair.

**D5 — Where does Schedule 1-A's total land on the TY2026 Form 1040, 13a or 13b? UNRESOLVED, AND NOT
RESOLVABLE HERE.** The TY2026 Schedule 1-A draft (line 44) says *"Enter here and on Form 1040 … line
**13a**"*; the TY2026 Schedule A draft (line 18) still subtracts *"lines **13a and 13b**"*. One draft
is stale, and the arbiter — a genuine TY2026 Form 1040 — **is the wrong-year document in §2.1 #7.**
`AbsoluteReturn`'s field is named semantically and survives either answer; `Schedule1A::line_13b` is
named for the 2025 answer and must be renamed regardless (D3). **Action: re-archive `f1040--2026`
first; this is one of the two things that unblock it.**

**D6 — De-pin `field_census.rs` now, or write TY2025's five missing censuses first? DE-PIN ONTO WIRED
YEARS.** The instruments lens correctly makes HUMAN-1 (write the five census-less TY2025 maps)
a precondition, because de-pinning onto the raw year set turns a green gate red on **missing work
rather than on a defect** — R15 measured 330 unaccounted TY2025 boxes. Under the pause, paying 330
human readings for a year nobody will file is exactly the sequencing the ruling retired. **Ruling:
the census gate's year set is the set of **wired** years; unwired assets on disk are caught by
`forms wire --check` instead, which reds today and names them.** That keeps both instruments honest
and neither blocked on the other.

**D7 — Should `report --tax-year 2026` refuse? NO — AND THE THREE ADJACENT ANSWERS DIFFER.**
Adjudicated per the refusal lens, from this repo's own doctrine rather than taste:
- **`report`: emit, and say so.** Nothing is signed, the figure comes from a KAT-pinned Rev. Proc.
  2025-32 table, and refusing pushes the filer to estimate by hand — the `era.rs`
  friction-toward-untruth shape. **What is missing is a readiness line, and that absence is the
  doctrine's own defect class: a blank that no input produced.**
- **`income import`: REFUSE the committed write.** Writing a committed full-return row for a year
  with no `FullReturnParams` is the poisoning `input_form_store.rs:295` says must never happen; the
  TUI already refuses the identical act and keeps a draft. This is the one place refusal is right and
  missing.
- **`export-snapshot`: gate it, and stamp the year into the artifact.** A file named `form8949.csv`
  is a filing artifact in everything but format, and today TY2026's and TY2099's are byte-identical.
- **The open product ruling, for the owner:** *may a table-only year emit a file named
  `form8949.csv` at all?* That is a product decision, not a code question.
**The shape of the fix is one type, not five messages:** a `YearReadiness` computed once from the
four sets (table / crypto-slice forms / full-return maps / full-return params), rendered on every
number-bearing surface **and used to build every refusal string**, so the strings cannot go stale.
Per B1 it lands paired with a planted-defect test — add 2026 to one set only, and a test reds. The
model is one crate away: `btctax-forms/tests/census.rs:419`.

**D8 — Carried from the previous edition, still standing.**
- *Is `i6251--2026` published?* **Settled: no.** `irs-dft/i6251--dft.pdf` still serves the TY2025
  revision (it contains `$900,350`). The §55(d)(3) MFS kicker rate and cap remain the one genuinely
  open `AmtParams` cell; the statutory reading is available today, the confirmation is ~Jan 2027.
- *Do TY2026 drafts exist?* **Settled: yes — 16 of 17 forms and 4 of 13 instructions — but now
  qualified twice:** one of the sixteen archived drafts is a TY2025 document (§2.1 #7), and all
  sixteen geometry fixtures are off by a page (§2.1 #1). *"Passes 1 and 2 can be exercised months
  early"* is true **after** those two fixes and not before.
- *Is the 0.50 phase-out rate encodable today?* **Knowable, not encodable.** The compute lens derived
  it independently (70,100 / (640,200 − 500,000)); that is corroboration, not authority. Encode from
  Rev. Proc. 2025-32 §55(d) or the final form.
- *The label-join count, 130 vs 177.* **Not a contradiction** — different denominators (mapped money
  lines across 9 forms vs `lineN` bindings across 10), both zero disagreements, both watched red on a
  planted defect. Recorded so nobody chases it.

---

## 8. Reading the lens files

| lens | file | own it for |
|---|---|---|
| year-seam inventory | `design/agent-reports/2026-09-05-scaffolding-seams.md` | the full class-(a)/(b)/(c) table; `sec_clusters`; the 20 unreachable assets; `DEFAULT_ROW_YEAR`'s call-site census |
| year package | `…-scaffolding-package.md` | the nine slots; the wired-vs-censused inversion; the 24-step runbook; the four hand-lists |
| compute layer | `…-scaffolding-compute.md` | C-1 (the wrong-year 1040), C-3 (6251 does not renumber), C-5 (Schedule 1-A part by part), C-6/C-7 (8995 and Schedule A), the three-axis pattern table, the 26-row year-shaped-site census |
| year-pinned instruments | `…-scaffolding-instruments.md` | I-1 `DEFAULT_ROW_YEAR`; I-2 the 30 missing joins; I-3 shipped-vs-validated; I-4/I-5 the ratchet and the registries; I-6 the f8275 alias; I-7 the oracle defaults |
| refusal surface | `…-scaffolding-refusal.md` | the measured exit-code matrix; the four year-sets; `income import`; `export-snapshot`; the `YearReadiness` argument |
| the port machine | `…-scaffolding-machine.md` | F1 the page derivation; F2 the clean-verdict-from-nothing; the `xtask forms` specification; the stop points |

**Superseded in part** by the above, still authoritative for what they measured:
`…-ty2026-port-{seams,constants,authority,maps,oracles,machine}.md` — the 36-stem IRS probe, the
measured publication cadence, the 266-rename classification, the f6251 blind-copy measurement, and
the taxcalc 6.8.2 / OTS-cadence readings.
