# Fable plan review — the year-transition plan, judged against "every year, N+1 cheap"

**Reviewer:** Fable 5.1, independent, design-level. **Date:** 2026-09-05.
**Read:** `design/ROADMAP_STATUS.md`, `design/TY2026_PORT_REPORT.md` (all of §1–§8), `design/LONG_RANGE_PLAN_filing.md`,
`design/TY2026_WORK_LIST.md`, `design/HARNESS.md`, `CLAUDE.md`; and the code the `FormAuthority` table would replace
(`cite_check.rs` `FormAuthority`/`emitted_form_years`, `pdf.rs`, `map.rs`, `packet.rs`, `tests/common/mod.rs`,
`tests/field_census.rs`, `tests/supported_years_cross_product.rs`, `line_coverage.rs`, `authority_manifest.rs::Entry`,
`tax_tables.rs`, `state.rs`, `admin.rs`, the TY2025 `f8949` extract, `RevProc_2025-32.txt`).
**Not re-derived:** §2's 32 sites, the field counts, the owner rulings, the draft-is-evidence rule, today's fixes.
No file other than this one was edited; no cargo/make/git was run.

## Verdict: **1 Critical / 8 Important / 4 Minor**

**The one question.** The plan serves the goal at the *instrument* layer — §2's FALLS-BACK taxonomy, the port
machine's stop points, and the B1-first build order are exactly the machinery the owner asked for, and the report is
right that the deliverable is machinery rather than a TY2026 packet. Its **shape** is wrong in four places, and each
one is a place where the plan optimises for the form-porting problem it has measured at the expense of the
year-filing problem it has not:

1. It models a "year" as a set of **forms**. A filed year has at least four per-year artifact classes — forms/maps,
   tables/params, the price dataset, and the **information-return regime** that routes Form 8949's boxes — and the
   fourth is where TY2026 differs most from TY2025 *for btctax's own filer profile*. Nothing in 27 risks, 24 steps or
   17 do-not-build rules mentions Form 1099-DA (C1).
2. The table it is about to build is specified as **one hand-written row per `(stem, year)`** — the F2 shape the repo
   finished deleting from three gates *today* (`emitted_form_years()`, `field_census.rs`, the cross-product test).
   A const table would put it back (I1).
3. It calls the `*Map` struct "the single thing that makes a year a code change" and then says two incompatible
   things about what to do with it, while silently reversing a `CLAUDE.md` rule the Schedule 1-A struct was reviewed
   green under (I2).
4. Its calendar has no critical path from final-forms-published to filed, and its rule 1 pushes statute-sourced
   work into the post-finals window for no reason (I4, I5).

---

## C1 — CRITICAL — the plan is silent on Form 1099-DA and TY2026 is the first year brokers report BASIS

**Where:** `TY2026_PORT_REPORT.md` — absent from every section (grep: zero hits in all three plan documents);
`LONG_RANGE_PLAN_filing.md` §1.2 (profile P1, "disposals → 8949/Sch D") and §3 P4.

**What is wrong.** The repo's own legal research (`legal/research/REPORT_us_btc_tax_TY2025-2026.md` §9, confidence
HIGH) records that brokers report **gross proceeds** on Form 1099-DA for 2025 dispositions and **basis** for covered
digital assets from **2026-01-01**, and that Form 8949 routes each row from `{1099-DA received?, basis reported?}` to
**G/H/I** (short) and **J/K/L** (long), with **Box C/F never used for digital assets**. The TY2025 Form 8949 extract
in the tree confirms the box text verbatim (`design/forms/extract/f8949--2025.txt:28-33`: *"(I) Short-term digital
asset transactions not reported to you on Form 1099-DA or Form 1099-B"*) and carries the Note: *"If you checked Box A
or Box G above but the basis reported to the IRS was incorrect, enter in column (e) the basis as reported to the IRS,
and enter an adjustment in column (g)"*.

What btctax does today, verified in source:

- The emitter files **every** digital-asset row under Box I/L (`admin.rs:452` `broker_reporting_advisory`, doc:
  *"every row files under Box I/L"*). Box I is a checkbox that **asserts the filer received no 1099-DA**.
- On the **full-return** export path the advisory that would tell the filer this cannot fire:
  `admin.rs:1220` `broker_reported_rows: 0` (the crypto-slice path at `:862` computes it). The long-range plan
  noted this line (§3 P4, "one line; fix it in this phase"); the port report dropped it.
- For a post-2025 taxable disposition with no election, the engine files the **HIFO default** and discloses the
  dollar gap to acquisition order as an *Advisory* (`state.rs` `IdentificationDefaulted`, FR-34 — owner-mandated
  default; not re-litigated here). Under TY2026 basis reporting, the broker's 1099-DA carries **its** basis
  (acquisition order absent a timely identification to the broker); btctax's column (e) will carry a different
  number, with no code-B adjustment in (f)/(g), because there is no 1099-DA input to reconcile against.

**The journey.** February 2027: W-2 filer, bought BTC on an exchange in 2026, sold some on the same exchange in 2026,
card-reward stack in self-custody. They receive a 1099-DA showing proceeds *and basis*. btctax prints Form 8949 with
Box I checked (false — a 1099-DA was received), column (e) = HIFO basis ≠ the broker's figure, no (f)/(g), no
advisory. The IRS document-matches 1099-DA against Schedule D line 1b/8b (Box A/G totals) and finds nothing there.
**Direction: understatement whenever HIFO basis exceeds the broker's acquisition-order basis**, which is the ordinary
case in a rising year. The card rewards are handled correctly (`event.rs` `CardRewardRebate`, FR-45: rebate, basis =
FMV at receipt, no income line) — *provided* the price dataset covers every 2026 receipt date (see I3).

**Why it matters for year N+1 specifically.** The broker-reporting regime is a **per-year axis that moves every
year** (2025 proceeds → 2026 basis → whatever 2027 adds) and it has **no slot in the year package**. A year-package
model that carries forms and tables but not "which information returns exist this year and what they report" will
re-discover this class every January, on the filer's own return, after the forms are done.

**Minimal change to the plan.**
1. Add a fourth artifact class to §3's year package: the **information-return regime** for the year (which 1099s
   exist, what each reports, which 8949 boxes they drive). Put it in the year record (I3).
2. Add to the input surface, per disposal or per account, `broker_reported: none | proceeds | basis` (a 1099-DA
   import is the natural carrier). This is `CLAUDE.md`'s own corollary — *"if the form asks something our input
   surface cannot answer, collect it"* — because Form 8949 literally instructs *"Before you check Box … see whether
   you received any Form(s) 1099-DA"*. Route the box from it; when `basis` and btctax's basis differs, (e) = reported
   basis, (g) = adjustment, (f) = `B`, exactly as the form's Note says.
3. Fix `admin.rs:1220` now; it is one line and it is the only disclosure the full-return filer has.
4. Register it as **R28 / LIVE / CRITICAL** and add a §6 rule: *do not ship a TY2026 Form 8949 whose box is chosen
   without a 1099-DA answer from the filer.*

---

## I1 — IMPORTANT — a `const` table of `(stem, year)` rows re-creates the hand-list three gates just deleted

**Where:** `TY2026_PORT_REPORT.md` §3 (a): *"one `FormAuthority`-shaped row per `(stem, year)` … derive `pdf.rs`,
`map.rs`, `CENSUS_KEYS` and `EMITTED_FORMS` from it"*; `ROADMAP_STATUS.md` §5.

**What is wrong.** The repo already moved the other way, and did it correctly: `cite_check.rs::emitted_form_years()`
reads `(stem, year)` off `crates/btctax-forms/forms/<year>/` (its own doc: *"the reason this is a function and not a
`const`"*); `tests/field_census.rs` was de-pinned today by walking the same directory with a shrink-only
`UNCENSUSED` register; `tests/supported_years_cross_product.rs` derives both axes from the filesystem and joins to the
archive **by sha256** precisely to avoid *"a fifth hand-written stem table"*. `STEM_ALIASES` is a two-row, total alias
table. So of the four things §3 says to derive from the table, one (`EMITTED_FORMS`) is already derived from a
stronger source than a table — the filesystem is the fact; a table is a claim about the fact.

A year package under a const table is 17 hand-added rows. A row someone forgets is a form outside **every** gate that
walks the table, reporting nothing — FALLS BACK by omission, `HARNESS.md` F2. `field_census.rs`'s module doc says the
same thing about `let year = 2024;` and was fixed by *deriving*, not by a better literal.

**What the const table is actually good for** is the facts a glob cannot produce: instructions stem, page range,
revision, template hash, which schema deserialises the map. Those facts have the same author and the same lifecycle
as the map file — so they belong **in the map header**, not in a central table that can list a row whose map does not
exist or miss a map that does.

**Minimal change.** The row **set** is `glob(forms/<year>/*.map.toml)`; the row **facts** are the map header; the
compile-time **binding** is a `build.rs`; `FORMS` in `cite_check.rs` is retired into the headers. See the shape section.

---

## I2 — IMPORTANT — the fate of the `*Map` structs is undecided, the report contradicts itself about it, and it silently reverses a `CLAUDE.md` rule

**Where:** `TY2026_PORT_REPORT.md` §3 step 19 (**H**: *"the struct is a per-YEAR artifact wearing a per-FORM name —
the single thing that makes a year a code change"*) versus §3's three-axis table (*"only the printed number changed →
new label row"*, bad-example column listing `schedule_1a.rs (56)`) and §7 D3 (*"carry the cited label in the per-year
label table"*).

**What is wrong.**
- Verified in source, the `*Map` struct is not per-year; it is per **line-set revision**. `Form6251Map` has
  `line1..line40`, is destructured exhaustively in `money_cells()`, and the filler pairs it cell-by-cell with the
  printed struct — `(&map.lineN, f.lineN)` at `form6251.rs:136-189`. D3 measures Form 6251 as **not** renumbering
  2025→2026, so the same struct serves both years; it changed at 2024→2025 (1a/1b). Calling it per-year mis-sizes
  step 19 for every constants-only year and under-sizes it for every renumber.
- Step 19 says amending the struct is human work; the three-axis table says a renumber should be a **label row**. Both
  cannot be the plan. If the label axis becomes data, step 19 shrinks to "new struct only when the line SET changes
  shape"; if not, the label table is decoration.
- `CLAUDE.md` says *"one field per numbered line, **named for the line**, in the form's own numbering"*. The report's
  quantity axis says *"semantic field name, line number in the doc comment"* and lists `schedule_1a.rs` — built under
  the `CLAUDE.md` rule and reviewed to green — as the bad example. That is a **standing-doctrine reversal presented as
  a synthesis**, which the brief's severity rule makes Important on its own.

**The resolution, called rather than averaged.** Both are right at different layers and the mistake is asking one
struct to be both. A **transcription** (what a specific revision of a document says — the printed struct, the map, the
Form 6251 worksheet) is line-named and per-revision by nature; that is what the `CLAUDE.md` rule protects, and the
AMT history is why. A **quantity** that crosses years (what the 1040 consumes — `AbsoluteReturn`) is semantic. So:
keep line-named transcription structs, **one per line-set revision, selected by a `line_set` field in the map
header**; expose semantic accessors for consumers; generate a renumber-only revision's struct skeleton and its filler
pairing from the label table rather than hand-writing them, so the "+6 duplicates Parts I/IV/V/VI verbatim" cost is
paid by a generator, not a person. Write the amendment to the `CLAUDE.md` clause explicitly — *fields are named for
the line **within a transcription struct**; cross-year quantities are semantic* — so the next reviewer does not red
the build for following the old sentence.

**Why it matters for N+1.** Whichever way this is decided, the table's row shape depends on it (a row needs
`line_set`, not just a map path). Deciding it after the table is built is the expensive order.

---

## I3 — IMPORTANT — there is no year-level record; the year package is modelled as forms only

**Where:** `TY2026_PORT_REPORT.md` §3 (nine artifacts, all per-form), §2.5 (four year-sets *"and no surface names
more than one of them"*), §7 D7 (`YearReadiness` computed from four sets).

**What is wrong.** Step 1 — *"decide the year's form list"* — is a human decision whose **output has no home**: it is
expressed only as which files got committed. `FORMS_ABSENT_FROM_YEAR` (the per-year "this form correctly does not
exist" list) lives inside a test file. `TY2025_RETURN_DUE` and `TRANSITION_DATE` are singular constants
(`conventions.rs`, R14). `BundledPrices` is a compiled-in CSV whose `max_date()` must reach 2026-12-31 before a TY2026
return can value a December card reward — a per-year data refresh that appears in no runbook step. The oracle
availability for the year (OTS 2026 does not exist until ~2027-01-27) is a fact the two-oracle rule depends on and
nothing declares. And C1's information-return regime has nowhere to go. `YearReadiness` compares four *actual* sets;
nothing states the *intended* set to compare them against, so "a form present that was not expected" and "a form
expected that is absent" are both invisible — the report's own *"third state (present, unreachable, unremarked) is
the defect"*.

**Minimal change.** One committed year record per year (`forms/<year>/YEAR.toml`; fields in the shape section)
carrying: due date, expected forms, absent-with-reason, tables citation of record, oracle availability, price coverage
requirement, information-return regime. `YearReadiness` becomes declared-vs-actual; every user-facing "supported
years" sentence is built from it (closes §2.5's stale-literal row by construction).

---

## I4 — IMPORTANT — §6 rule 1 is misdrawn: three of its four named figures have a statutory or Rev. Proc. source in the tree today

**Where:** `TY2026_PORT_REPORT.md` §6 rule 1 (*"not the §55(d) resets, not `$640,200`, not the 0.50 phase-out rate,
not the Schedule 1-A amounts … Knowable ≠ encodable"*) and §7 D8; `tax_tables.rs::ty2026_full_return_must_stay_fail_closed`.

**What is wrong.** Verified in the repo:
- `legal/text/irs-guidance/RevProc_2025-32.txt` §2.10 publishes the TY2026 §55(d)(1) exemption amounts and the
  §55(d)(2)-related phase-out figures; its §.07 records that OBBBA **§70107** amended §55(d)(4) (the reset). The
  phase-out **rate** is in the amended statute, not in the draft form — the draft is a *witness* to it.
- `BundledTaxTables::ty2026()` is **already** transcribed from *"Rev. Proc. 2025-32 + OBBBA Pub. L. 119-21"* and
  KAT-pinned; `schedule_1a_params` already returns the statutory, unindexed Schedule 1-A amounts for 2025..=2028. The
  plan's own product ships rule-1-forbidden figures, correctly, because their source is not the draft.
- `ty2026_full_return_must_stay_fail_closed`'s reason 1 (*"the 2026 instructions are not published … the §55(d)(3)
  phase-out RATE, the zero-exemption thresholds … are all unknown"*) was written 2026-07-29 and is now stale for the
  rate and thresholds; reasons 2 (form transcription) and 3 (no oracle) still hold.

So rule 1 as written conflates *"never encode from a draft"* (right, keep) with *"never encode a TY2026 figure until
the final form"* (wrong). The genuinely draft-only datum is the one D8 names — the MFS §55(d)(3) kicker's exact cap
in the 2026 instructions — and even there the report says *"the statutory reading is available today"*.

**Why it matters for N+1.** Following rule 1 literally puts **all of `FullReturnParams` TY2026** onto the post-finals
window (Jan–Feb 2027), on the critical path, for constants that could be encoded, KAT-pinned and two-oracle-checked
against taxcalc's `AMT_em_*` parameters this month. Every future year has the same Rev. Proc.-in-October /
forms-in-January gap, so the rule's shape decides whether tables are ever off the critical path.

**Minimal change.** Rule 1 → *"Do NOT encode any figure whose ONLY source is a draft. A figure with a statutory or
Rev. Proc. source is encoded from that source when it publishes, with the draft as a corroborating KAT and the final
form as a later confirmation test."* Rewrite the TY2026 gate's doc to reasons 2 and 3. Encode `AmtParams` TY2026 now.
The owner ruling `CONTINUITY.md` asks for is therefore not "relax the rule" — it is "the rule was aimed at the wrong
thing".

---

## I5 — IMPORTANT — no critical path from finals-published to filed; the two-oracle rule alone makes ~Feb 2027 the earliest validation; the extension is not in the plan

**Where:** `TY2026_PORT_REPORT.md` §1 (*"What did NOT change"* — the calendar is stated, never sequenced);
`LONG_RANGE_PLAN_filing.md` §6 (still sequenced for TY2025); `ROADMAP_STATUS.md` §3.

**What is wrong.** The dates the plan itself gives: finals Nov 2026 – Jan 2027, `i1040gi--2026` and `i6251--2026`
last (~Jan–Feb 2027); OTS 2026 *preliminary* ~2027-01-27; season opens ~2027-01-26; due 2027-04-15. `CLAUDE.md`'s
two-oracle rule is absolute (*"never … call a figure validated, on one oracle"*), so no TY2026 AMT or Schedule 1-A
figure can be called validated before OTS 2026 exists and has been read (`{2024}` excuse sets *"until someone reads
`taxsolve_US_1040_2026.c`"*, rule 13). The measured review cadence on one form (Schedule 1-A: 37 days, five rounds)
and the packet-read exit gate then sit inside a ~10-week window with the instruction-dependent transcriptions
(Schedule A's two new worksheets, Form 8995 lines 16/17, Schedule 1-A Parts II/III, Form 6251's confirmed kicker).
Nothing in the plan says which work waits on which publication.

**Minimal change.**
1. Partition every open item into **NOW** (statute / Rev. Proc. / regime: `FullReturnParams` 2026 incl. `AmtParams`,
   due dates, the Form 8995 line-16/17 instrument variant, the Schedule A §68-style instrument variant, the 1099-DA
   input, price dataset, the table redesign, the port machine), **AFTER FINALS** (maps, censuses, label tables, doc
   comments, worksheet line-by-line transcription, Parts II/III rebuild), **AFTER OTS 2026** (two-oracle census,
   goldens). Only the last two are on the critical path; the plan currently lets the first leak into them.
2. State the critical path: *final `i1040gi`/`i6251` → worksheet transcriptions → review to 0C/0I → OTS-2026 census
   → packet read → filed*.
3. Adopt the **extension as the default plan**, not a fallback: Form 4868 with payment by 2027-04-15, file by
   2027-10-15. The owner has already said TY2027 timing is acceptable; this is the sequencing that lets every gate stay
   hard under April pressure — the pressure that produced option D ("delete the gate early") last time. Add Form 4868
   (one AcroForm, an estimate) and 1040-V to P4.

---

## I6 — IMPORTANT — step 24 is overstated as instrument-less, which points the build at the wrong tool

**Where:** `TY2026_PORT_REPORT.md` §3 step 24 (*"no delta tool reports this axis, and it is the one that silently
produces a wrong number"*); §3 closing (*"a third axis … had to be computed by hand"*).

**What is wrong.** Two partial instruments already exist, one of them in the report itself:
- `forms delta` / port pass **P3** — `diff -b` of each line's printed text between the two extracts, with the doc
  comment deleted on change. On the form face, "meaning changed, number did not" is almost always a **text**
  change: Schedule 2 line 10 → *"Reserved for future use"*, Form 8949 Box C → *"other than digital asset
  transactions"*, Form 8995 line 17 `smaller`→`greater`, Form 6251 1a's cited Schedule 1-A line 37→43. P3 catches
  every one of those.
- `line_coverage.rs::Coverage::quoting(year)` — every `LineCoverage.instruction` is a verbatim sentence re-verified
  against `<form>--<year>.txt`; its own doc says *"a sentence carried forward unchanged from the old booklet REDS
  instead of passing."* Re-pointing the year re-verifies every quoted sentence.

What is genuinely uncovered is the **instructions booklet's predicates that no row quotes** — who-must-file tests,
worksheet definitions, a threshold that lives in the instructions rather than on the form. That is the residual, and
it is narrower and more buildable than "a meaning axis".

**Minimal change.** Rewrite step 24 as: *M for form text (P3) and for every quoted instruction sentence (`quoting`);
H for the residual — instruction predicates not yet quoted as rows.* Build-order item: widen `LineCoverage` rows to
those predicates (they are already what *"transcribe, never paraphrase"* demands), so the re-verification covers them.
Do not build a separate "meaning" tool.

---

## I7 — IMPORTANT — the refusal-surface defects are named "live" in §2.5 and have no slot in §4's build order

**Where:** `TY2026_PORT_REPORT.md` §2.5 (`income import --year 2026` → committed write → `report` exit 2 → prescribed
`income clear` **deletes the filer's W-2s**, *"the single highest-cost defect on this surface"*; TUI `selected_year:
2025`; `export-snapshot` ungated and unstamped) versus §4 *"Build order"* (eight items, all port-machine).

**What is wrong.** The report's §1 says the pre-forms window is now *"where btctax lives"*, and §7 D7 gives the fix its
shape (`YearReadiness`, one type, rendered everywhere, strings built from it, B1-tested). Then the build order omits
it. In the journey, these are the **first three things** a February 2027 user touches — before any form.

**Minimal change.** Build-order item 1b: `YearReadiness` + `income import` refusing the committed write for a year
with no `FullReturnParams` (mirror the TUI's `CommitOutcome::NoTables`) + the TUI default year derived from
readiness. Cheap, B1-testable (*"add 2026 to one set only, and a test reds"*), and it closes §2.5's stale-literal
row for every future year.

---

## I8 — IMPORTANT — the computed work list omits an emitted form and reports nothing

**Where:** `TY2026_WORK_LIST.md` (14 rows); `TY2026_PORT_REPORT.md` §5d (*"`f1040s1--2025`: moot unless the form is
emitted"*).

**What is wrong.** `packet.rs:100-106` pushes `f1040s1` whenever `sch_1` is `Some` — Schedule 1 **is** emitted, and it
is where btctax's non-rebate crypto ordinary income lands (`printed.rs:431-455`, line 8v; a `Reward` sign-up bonus in
the journey filer's stack goes there). `MANIFEST.json` holds `f1040s1--2026-DRAFT.pdf`; no `f1040s1--2025` authority
exists; so `form-delta` had no pair and the work list has **no row** — the same *"clean verdict from zero
comparisons"* shape as §2.1 #2, one level up. §5d's "moot" is wrong on its own premise.

**Minimal change.** `forms port-status <year>` enumerates from the **emitting surface** (`Stem` × year, I1/shape),
never from available fixture pairs, and prints `NO PRIOR SIDE` / `NO DRAFT` / `NO FINAL` per cell rather than
omitting the row. Un-moot `f1040s1--2025` (it is the prior side of an emitted form's delta) and re-generate the list.

---

## M1 — MINOR — the human-step audit

Of the eight **H** steps, four are partly mechanical and one **M** step is partly judgment:

| step | as marked | finding |
|---|---|---|
| 1 form list | H | the IRS prior-year index enumerates `fNNNN--YYYY` mechanically; the human decides only the **diff** against last year (new / vanished / renamed, e.g. `f8275`→`f8275r`). And its output must be committed (I3). |
| 5 instructions stem + pages | H | the stem is `fNNNN`→`iNNNN` plus a two-row alias (`f1040sa`→`i1040sca`; Schedules 1-A/2/3 → `i1040gi`); the page range is a `pdftotext` search for the schedule's running header, confirmed by a person. M\* + confirm. |
| 20 doc comments | H | P3 already carries unchanged lines; for changed lines the machine can **propose** the extract's own line text (that is what transcription is) and the human confirms the sentence boundary. |
| 21 census | H | engine claims are irreducibly human; the `artifact` rule (header boxes, *"Reserved for future use"*, page-2 repeats) is mechanical by shape and should be pre-filled. |
| 11 copy the PDF | M | **which** revision to ship is a judgment when the IRS re-issues a final mid-season (the README's own *"a different hash means the IRS REVISED"*). Record the decision in the map header (`template_sha256`). |
| 2 / `forms fetch` | M\* | the refusal *"year printed on the document ≠ year requested"* is wrong for **periodic** forms (`f8275` Rev. 10-2024, `f8283` Rev. 12-2025), whose face carries a revision date, not a tax year. The row needs a `versioning` field so the check knows which question to ask. |

## M2 — MINOR — document consistency

`ROADMAP_STATUS.md` §3 still commits to *"TY2025 filed with btctax after 2026-10-15"* while §0a pauses TY2025.
`LONG_RANGE_PLAN_filing.md` §6.3 marks option C (*"Skip TY2025; target TY2026 only … saves nothing"*) as the trap,
and the owner has now chosen essentially option C — the plan's own argument (*"the TY2025 build IS the TY2026 build"*)
is exactly why the choice is cheap, and the section should say so rather than read as a contradiction.

## M3 — MINOR — the price dataset is a per-year artifact with no runbook step

`BundledPrices` is compiled in; `btctax-update-prices` appends to a local cache. A TY2026 return valuing a
December-2026 card reward needs a close for that date in the **shipped** dataset or the filer must run the updater
(network). Add *"dataset covers the filed year through 12-31"* to the year record (I3) and to `YearReadiness`.

## M4 — MINOR — in-season fetch URL

During the 2027 season the current-year final may be served only at `irs-pdf/<stem>.pdf` (moving URL) before
`irs-prior/<stem>--2026.pdf` exists. R19 covers moving URLs for the two periodic notes only; `forms fetch` should
record the URL form it used and the manifest should treat an `irs-pdf` URL as re-verify-on-next-fetch for every
form, not two.

---

## The `FormAuthority` table — recommended shape

**Principle: stem is code, year is data, and the filesystem is the registry.** A new *form* needs a filler and is
correctly a code change; a new *year* of an existing form must be files under a directory and nothing else. Every
gate walks the glob; nothing walks a list.

### Layer 0 — location is status

```
crates/btctax-forms/forms/<year>/<stem>.pdf          bundled template (final authority only)
crates/btctax-forms/forms/<year>/<stem>.map.toml     the ROW (header) + bindings + [identity] + [census]
crates/btctax-forms/forms/<year>/YEAR.toml           the year record (Layer 3)
crates/btctax-forms/forms-provisional/<year>/…       port-machine output against a draft; NEVER globbed by the build
```

A provisional map cannot be loaded *by construction* because the build never sees the directory — the same move as
`-DRAFT` in the filename and `irs-dft` in the URL. This replaces the report's `provisional = true` flag plus a
generated `UnsupportedYear` arm (two truths, one of them a runtime check).

### Layer 1 — the row is the map header

```toml
form               = "f6251"          # crate stem (exists)
year               = 2026             # (exists)
irs_stem           = "f6251"          # IRS basename; differs only for schedule_d/schedule_se — STEM_ALIASES retires into this
versioning         = "annual"         # or { periodic = "Rev. 10-2024" }: a periodic form may alias a prior revision BY HASH, never by year list (I-6)
template_sha256    = "…"              # of the bundled PDF; joined BY CONTENT to MANIFEST.json, whose entry must be is_authority()
instructions       = "i6251"          # "" only for self-instructing forms (f8275)
instr_pages        = [101, 110]       # only for i1040gi-hosted schedules (human step 5, recorded once)
line_set           = "f6251/2025"     # which transcription struct deserialises this map (I2); constants-only year ⇒ same line_set, renumber ⇒ new one
attachment_sequence = "32"            # read from the extract; today a literal per form in packet.rs
```

Derived by convention, **never stored**: form extract `design/forms/extract/<irs_stem>--<year>.txt`, instructions
extract `<instructions>--<year>.txt`, geometry `design/forms/geometry/<irs_stem>--<year>.json`; the census is the
`[census]` table in the same file (the report's "census path" is the map path).

B1 kills, each planted before the checker ships: a header missing a required field → parse refusal (extend the
existing `deny_unknown_fields` structs with required fields); `template_sha256` ≠ the file → red; the hash absent
from `MANIFEST.json`, or present on an `is_draft()` entry → red; `line_set` naming no schema → compile error via the
match in Layer 2.

### Layer 2 — the binding is a `build.rs`

`crates/btctax-forms/build.rs` globs `forms/<year>/` under `CARGO_MANIFEST_DIR` only, emits `bundled.rs`:

```rust
pub enum Stem { F1040, F1040s1, F1040s1a, F1040s2, /* … closed set; CODE */ }
pub fn template(stem: Stem, year: i32) -> Option<&'static [u8]>;   // include_bytes! per file found
pub fn map_text(stem: Stem, year: i32) -> Option<&'static str>;    // include_str!  per file found
pub fn bundled_years() -> &'static [i32];                          // replaces SUPPORTED_YEARS
// println!("cargo:rerun-if-changed=forms");
```

Every `Map::for_year` becomes one generic line — `map_text(Stem::F6251, y).ok_or(UnsupportedYear(y))?` then parse
into the struct named by the header's `line_set` (an exhaustive `match`, so an unknown `line_set` cannot compile).
Step 22's 85 hand-edits become 0; §2.7's four one-token arms cease to exist; `error.rs`'s supported-years sentence is
built from `bundled_years()`. Keep `supported_years_cross_product`'s `wired`/`dispatch` obligations as the B1 witness
that the build script binds every file on disk. Publishing trap (`crate-publishing-state`): the build script reads
only under the manifest dir, and a `cargo package --list` gate asserts every globbed file is in the tarball.

*Acceptable weaker alternative* if a build script is refused for the published crate: `forms port --emit-wiring`
writes a committed `bundled.rs`, and `forms wire --check` (built first, red today on ten stems — the report's order is
right) runs in `make check`. Weaker because drift is then a test failure rather than a non-event.

### Layer 3 — the year record `forms/<year>/YEAR.toml`

```toml
year            = 2026
status          = "preparing"        # preparing | filable — a DECLARATION; full_return_for(year) stays the compute gate
return_due      = 2027-04-15         # replaces TY2025_RETURN_DUE / TRANSITION_DATE (R14)
forms_expected  = ["f1040", "f1040s1", "f1040s1a", "f1040s2", "…"]      # step 1's OUTPUT, committed
forms_absent    = { f8275 = "Rev. 10-2024 aliased by hash from 2024", f1040s1 = "…" }   # FORMS_ABSENT_FROM_YEAR moves here
tables          = "Rev. Proc. 2025-32; Pub. L. 119-21"                  # citation of record for TaxTable + FullReturnParams
oracles         = { ots = "2026 — not published (expected ~2027-01-27)", taxcalc = ">= 6.8.2" }
prices_through  = 2026-12-31         # BundledPrices::max_date() must reach this before status = filable (M3)
information_returns = { f1099da = { proceeds = true, basis = true } }   # routes Form 8949 boxes (C1)
```

`YearReadiness` (D7) = declared (this file) versus actual (glob, tables, params, prices). B1 kills: a form in
`forms_expected` with no map → red; a map on disk in neither `forms_expected` nor `forms_absent` → red (the "third
state"); `status = "filable"` with `full_return_for(year).is_none()` → red; `prices_through` unmet → red.

### Layer 4 — the label table is the join, not a new artifact

`LineCoverage { form, year, line, field, instruction }` already exists and is already verified against the year's
extract by `xtask line-coverage`. For a **renumber-only** revision, generate the new `line_set`'s struct skeleton and
the filler's `(map cell, printed field)` pairing from it; the quote-verification then *is* the gate that the emitter's
placement matches the year's booklet. For a **rebuilt part**, the machine stops (report stop-point 4) and a person
writes the transcription struct under the `CLAUDE.md` rule.

### The gates, all walking the glob, each with its kill

map ⊆ PDF fields (exists) · census ∪ map == PDF (exists, register) · `template_sha256` == file == an
`is_authority()` manifest entry · `geometry.pdf_sha256 == template_sha256` (§2.2 #14) · doc comment ⊆ that line's
extract text (step 20's check, 17/17 on f8959) · `for_year(y)` Ok ⇔ file on disk (tautological under Layer 2; keep as
witness) · `forms_expected` == present ∪ absent-with-reason · every `Stem` has an arm in `fill_full_return`'s
exhaustive `PrintedForms` destructure (exists).

### What the table does not decide, and must not pretend to

Step 1's diff, step 16's adjudications, step 17's same-line judgments, step 21's engine claims, step 24's residual
predicates, and every dollar figure. Each of those is recorded **in** the map or year file with a reason, so a
future porter reads the decision beside the thing it decided.

---

*Severity summary:* Critical — C1. Important — I1, I2, I3, I4, I5, I6, I7, I8. Minor — M1, M2, M3, M4.
No finding here re-opens an owner ruling; C1 does not re-litigate the HIFO default, only the plan's silence on what
TY2026 broker basis reporting does to it.
