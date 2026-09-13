# RECON — the prior-year delta probe: runbook steps 16 & 17 on years where history already knows the answer

**Brief:** `design/agent-reports/BRIEF-prior-year-delta-probe.md` (`ca342ef5e`), read in full and followed.
**Date:** 2026-09-13. **Tier:** opus. **Isolation:** own worktree, `CARGO_TARGET_DIR=<wt>/target-probe`.
**Gate:** `make check` — **3650 tests run: 3650 passed, 12 skipped**, exit 0 (nextest + clippy, foreground).
No Rust touched, so `cargo fmt` is moot. **Nothing committed. Nothing bundled.**

---

## 0. The answer, in one paragraph

Handed eleven real consecutive prior-year pairs, `form-delta` **got every single verdict it printed
right** — 88 line moves across 11 pairs, every one I adjudicated against the two extracted documents
held, and both control pairs (Schedule D, Form 8949) correctly returned `Unchanged`. The failures are
**not wrong answers; they are missing axes and a silent identity key.** Three, in descending order:
the label axis is keyed to the full AcroForm FQN, so an intermediate container rename drops a box out
of it — **71 further real line moves, 45% of the total, never reached the output, and the `added`/
`removed` lists they fell into carry no printed-line label at all**; there is **no line-set axis**, so
**62 retired line numbers** across the corpus were reported as zero; and there is **no line-meaning
axis**, so of **331 surviving line numbers**, the nine on Schedule 3 TY2020→TY2021 that now name a
*different quantity* — the 37→43 shape, the dangerous one — were reported as nothing at all. Steps 16
and 17 are load-bearing, and step 13 as it stands hands a porter 88 labelled moves plus **624
unlabelled names, 137 of which it actually prints**, with 71 of the moves hiding inside them.

---

## 1. Stop-and-report: what I could and could not disprove

The brief invited refutation of three claims. **Two held exactly; one has a trap beside it.**

| claim | verdict |
|---|---|
| `irs-prior` serves the revisions needed | **HELD.** 24 of 24 probes HTTP 200 for `f1040`, `f1040s1`, `f1040s3`, `f1040sd`, `f8949` × 2020–2023. All 17 documents fetched, hashed, noted. |
| `form-delta` accepts arbitrary stems | **HELD.** 11 prior-year pairs ran, none of them a bundled year. |
| prior-year forms carry AcroForms | **HELD, strongly.** 17 of 17 carry one — 23 to 244 boxes — and `extract-geometry` printed **zero** `field(s) have no widget rect and were dropped` notes on any of them, so every AcroForm field the PDFs declare is in a fixture. |

★ **The trap the brief walked into without falling in.** The brief named "Schedule 8812" as a candidate.
`irs-prior/f1040s8812--<year>.pdf` is **404 for every year 2020–2024**; the IRS stem for Schedule 8812
is **`f1040s8`** (200 for all four). Nothing in the brief was wrong — it named the form, not the stem —
but **runbook step 2 is written as `irs-prior/{stem}--{year}.pdf` and offers no way to learn that**,
and step 1 ("does this stem exist for this year, under this name?") is tagged **H** for exactly this.
A porter who guesses the obvious stem gets a 404 that is indistinguishable from "this form did not
exist that year" — the same confusion `f1040s1a--2024` is documented for. **This is a fourth instance
of the class, and the first where the form *does* exist and only the spelling is wrong.**

---

## 2. The corpus, and what it cost

17 authority documents fetched from `irs.gov/pub/irs-prior`, each with its sha256 and byte count
recorded in a `.pdf.txt` note in the committed format, each extracted (`pdftotext -layout`, form feeds
→ LF, recipe **proved** by `forms extract --adopt`, not guessed) and each observed
(`xtask extract-geometry`).

| stem | form | revisions archived | boxes |
|---|---|---|---|
| `f1040` | Form 1040 | 2020, 2021, 2022, 2023 | 128 / 131 / 136 / 139 |
| `f1040s1` | Schedule 1 | 2020, 2021, 2022 | 29 / 62 / 67 |
| `f1040s3` | Schedule 3 | 2020, 2021, 2022 | 25 / 41 / 40 |
| `f1040s8` | Schedule 8812 | 2020, 2021, 2022 | 23 / 71 / 41 |
| `f1040sd` | Schedule D | 2020, 2021 | 55 / 55 |
| `f8949` | Form 8949 | 2020, 2021 | 244 / 244 |

**Both archive checkers pass over the whole enlarged archive:**
`extract-geometry --all --check` → *"87 committed fixture(s) — 87 reproduce byte-for-byte, 0 rewritten,
0 unresolved"*; `forms extract --all --check` → *"143 text layer(s) — 143 reproduce byte-for-byte"*.

### 2.1 The archive is NOT silently absorbed — and that is good news

Before the manifest was updated, `make check` was **3650 run / 3648 passed / 2 failed**, and the two
failures were exactly the right two, naming all 17 documents individually:

- `authority_manifest::tests::every_primary_source_is_in_the_manifest` — *"AUTHORITY MANIFEST — 17
  problem(s): … a primary source in an accounted-for tree with NO manifest entry"*
- `form_geometry::pdf_sha_tests::every_committed_geometry_fixture_matches_the_manifest` — *"17 of 87
  committed geometry fixtures are not observations of the document MANIFEST.json points at"*

**Adding a prior-year document to this archive reds two committed gates until the manifest records it.**
That is the instrument the README claims, observed discriminating on a real case rather than asserted.

### 2.2 FR-139 discharged step 4 exactly as its doc comment promises

`authority-manifest --regen` refused first, correctly: *"REFUSING to regenerate: it would drop 125
document(s) from MANIFEST.json without a word"* — this worktree held none of the ignored PDFs.
`forms_fetch.rs`'s own doc comment predicts this refusal verbatim and exists to close it. It did:

    forms fetch --restore: 142 document(s) — 17 already present and hash-correct, 125 restored, 0 failed

★★ **That line is also an authority-drift measurement nobody asked for: all 125 previously-archived
documents still hash to what their notes record, as of 2026-09-13.** Zero IRS revisions anywhere in
the archive. `authority-manifest --regen` then wrote **153 lines, all insertions** (17 entries × 9
lines); the 183 pre-existing entries were not touched. `make check` green thereafter.

---

## 3. Every pair, what the tool said, and whether it was right

`form-delta <old> <new>`, run once over 11 consecutive pairs, captured to a file and grepped.

| pair | name axis | label verdict | correct? |
|---|---|---|---|
| `f1040--2020 → 2021` | 83 common, 48 +, 45 − | **Moved** 5 of 51 compared | **yes**, all 5 |
| `f1040--2021 → 2022` | 93 common, 43 +, 38 − | **Moved** 15 of 47 | **yes**, all 15 |
| `f1040--2022 → 2023` | 72 common, 67 +, 64 − | **Moved** 12 of 56 | **yes**, all 12 |
| `f1040s1--2020 → 2021` | 24 common, 38 +, 5 − | **Moved** 14 of 22 | **yes**, all 14 |
| `f1040s1--2021 → 2022` | 58 common, 9 +, 4 − | **Moved** 3 of 56 | yes |
| `f1040s3--2020 → 2021` | 19 common, 22 +, 6 − | **Moved** 12 of 17 | **yes**, all 12 |
| `f1040s3--2021 → 2022` | **0 common**, 40 +, 41 − | **Unwitnessed**, exit **1** | honest, and **blind** — see F1 |
| `f1040s8--2020 → 2021` | 10 common, 61 +, 13 − | **Moved** 10 of 10 | **yes**, all 10 |
| `f1040s8--2021 → 2022` | 17 common, 24 +, 54 − | **Moved** 17 of 17 | **yes**, all 17 |
| `f1040sd--2020 → 2021` | 54 common, 1 +, 1 − | **Unchanged**, 50 compared | yes — control |
| `f8949--2020 → 2021` | 244 common, 0 +, 0 − | **Unchanged**, 234 compared | yes — control |

**Totals, computed from the log rather than counted by hand:** 674 common, 353 added, 271 removed, 114
unwitnessed, **88 moves printed**. Of the 624 added/removed names, **137 are actually printed**; the
rest sit behind `… and N more (not shown)` — which does at least say it truncated.

### 3.1 The three shapes the brief asked for, found and adjudicated

**(a) A field that MOVED.** `f1040--2022 → f1040--2023` is the tool's founding case reproduced on real
data, and the cleanest specimen in the corpus. TY2023 added a **fourth dependents row**
(`Table_Dependents[0].Row4[0].f1_28/29/30`), consuming three field indices, so every page-1 income box
shifted three places:

| box | TY2022 line | TY2023 line |
|---|---|---|
| `f1_31[0]` | 1d | **1a** |
| `f1_40[0]` | 3a | **1z** |
| `f1_56[0]` | 15 — *taxable income* | **12 — *standard deduction*** |

★ **And the printed line set is IDENTICAL** — 54 line numbers both years, 0 retired, 0 new. A
name-existence check passes with 0 of 72 common names absent; a line-set check passes with 0 diff; and
a TY2022 map carried forward writes **taxable income into the standard-deduction box**. Only the
line→label axis sees it. **Verdict: correct, and the axis earns its keep.**

**(b) A line that was RETIRED — and the retirement that is worse than a deletion.** Form 1040 **line
30, "Recovery rebate credit"**, present TY2020 and TY2021, is in TY2022 *"Reserved for future use"*
(`design/forms/extract/f1040--2022.txt:110`). The IRS **kept a live AcroForm box** beside it:
`f2_19[0]` at y 528.0–540.0, in the contiguous 12pt stack between `f2_18[0]` (line 29) and `f2_20[0]`
(line 31). `form-delta` reports `f2_19[0]: line 28 → 30`, which is **correct**. What it cannot say is
the consequence: TY2022 line 32 reads *"Add lines 27, 28, 29, and 31"* — **line 30 is excluded from the
sum**. So a carried-forward TY2021 map writes the refundable child tax credit into a box that prints on
the return, is never added to anything, and **silently vanishes from total payments**. A refundable
credit is lost; the figure still appears on the page; **both oracles are blind, because nothing about
the computation changed.** A second instance in the same corpus: Schedule 3 line **13c**, *"Health
coverage tax credit from Form 8885"* (TY2021) → *"Reserved for future use"* (TY2022).

**(c) A line number REUSED for a different quantity — the dangerous one.** **Schedule 3,
TY2020 → TY2021: 13 line numbers survive, and 9 of them name a different quantity.** Measured by
diffing each surviving number's printed sentence out of the two committed text layers:

| line | TY2020 | TY2021 | why it matters |
|---|---|---|---|
| **7** | *"Add lines 1 through 6. Enter here and on Form 1040 … line 20"* — the Part I **total** | *"Total other nonrefundable credits. Add lines 6a through 6z"* — a **subtotal** | TY2021 then adds lines 1–5 **to** line 7 at line 8, so writing the old total here **double-counts lines 1 through 5** — an overstated credit, i.e. an **understatement of tax** |
| **8** | *"Net premium tax credit. Attach Form 8962"* — refundable, Part **II** | *"Add lines 1 through 5 and 7 … line 20"* — the Part **I** total | different quantity, different part, different destination on the 1040 |
| **12** | *"Other payments or refundable credits:"* — a **heading** over 12a–12f | *"Credit for federal tax on fuels. Attach Form 4136"* — a **money line** | a heading became a money line |
| **13** | *"Add lines 8 through 12f … line 31"* — the Part II **total** | *"Other payments or refundable credits:"* — a **heading** over 13a–13z | the reverse |
| 6, 9, 10, 11 | — | — | each slid to a different quantity (fuels → SS/RRTA → extension → premium) |
| 2 | *"Credit for child and dependent care expenses. Attach Form 2441"* | *"… from Form 2441, line 11. Attach Form 2441"* | ★ **same quantity, refined wording — NOT a reuse.** A text diff cannot tell this from the eight above; a human must. |

**What `form-delta` says about any of this: nothing.** It correctly reports that the *box* which held
line 7 now sits at 6c (Adoption credit) — so a map keyed by field name is served. **A cross-reference
keyed by line number is not**, and this repo's transcription structs are keyed by line number
(`line20`, `line33`) by standing rule.

**A fourth shape the brief did not name, and the worst of the four: a move that is money-neutral and
testimony-false.** Schedule 1, TY2020 → TY2021, verified box-by-box with `label-boxes`:

- `f1_13[0]`: line **8** *"Other income. List type and amount"* → line **8c** *"Cancellation of debt"*.
  Both feed line 9, so **AGI is identical and both oracles agree**. The return now testifies to
  cancellation of debt the filer never had. Per this repo's own doctrine an entry is sworn testimony;
  this one is **fabricated**, and no figure-level instrument can see it.
- `f1_29[0]`: line **22** *"Add lines 10 through 21. These are your adjustments to income"* → line
  **8z** (other income, write-in). A **subtraction** from income written into an **addition** to it:
  AGI overstated by twice the adjustments. Sign-flipped, and taxpayer-adverse.

---

## 4. Findings

### F1 — Important. The label axis is keyed to the full AcroForm FQN, so a container rename silently removes a box from it. **71 of 159 real line moves (45%) never reached the output.**

`compute` builds `common = field_set(old) ∩ field_set(new)` over **fully-qualified** AcroForm names, and
`label_axis` iterates only `common`. An IRS revision that renames an *intermediate* container segment
therefore drops the box out of the label axis entirely, into `added`/`removed` — **and `print_capped`
prints those as bare names with no printed-line label**, even though `compute` is holding both label
maps at that moment. The drift is not merely unreported; it is **unrecoverable from the output**.

Re-pairing on `(page, leaf segment)` — the IRS's own positional index, e.g. `f2_22[0]` — over the same
11 pairs and the same `label-boxes` witness the tool itself uses:

| pair | tool printed | leaf-matched | **hidden** |
|---|---|---|---|
| `f1040--2020 → 2021` | 5 moved / 83 common | 20 moved / 123 | **15** |
| `f1040--2021 → 2022` | 15 / 93 | 42 / 126 | **27** |
| `f1040--2022 → 2023` | 12 / 72 | 26 / 113 | **14** |
| `f1040s1--2020 → 2021` | 14 / 24 | 19 / 29 | **5** |
| `f1040s1--2021 → 2022` | 3 / 58 | 5 / 62 | **2** |
| `f1040s3--2020 → 2021` | 12 / 19 | 15 / 22 | **3** |
| `f1040s3--2021 → 2022` | 0 / **0** | 0 / 40 | 0 |
| `f1040s8--2020 → 2021` | 10 / 10 | 12 / 12 | **2** |
| `f1040s8--2021 → 2022` | 17 / 17 | 20 / 20 | **3** |
| `f1040sd`, `f8949` (controls) | 0 | 0 | 0 |
| **total** | **88** | **159** | **71** |

The mechanism, at its most trivial: TY2022 → TY2023 renames `Lines4a-11_ReadOrder[0]` to
`Line4a-11_ReadOrder[0]`. **One character.** It costs the label axis 11 boxes. Others in the corpus:
`Lines27-32_ReadOrder[0].f2_16[0]` → bare `f2_16[0]`; `StandardDeductionBubble[0].f1_44[0]` ⇄ bare;
`RoutingNo[0].f2_25[0]` ⇄ bare.

★★ **`f1040s3--2021 → f1040s3--2022` is the extreme, and it inverts the tool's whole purpose.** The
*only* name-axis change is that the root subform was renamed `form1[0]` → `topmostSubform[0]`:

    boxes: 41 old, 40 new
    common with full FQN     : 0
    common with ROOT STRIPPED: 40
    root-stripped only in old: 1   ['Page2[0].Line13z_ReadOrder[0].f2_13[0]']
    root-stripped only in new: 0

Forty of forty-one leaf paths are byte-identical below the root. Re-running the tool's own label axis on
the root-stripped pairing: **38 of 40 compared, 0 moved.** The real TY2021→TY2022 Schedule 3 work list
is *(a) substitute one prefix in 40 map keys, (b) drop one field, (c) re-read line 13c, which became
"Reserved for future use."* What the tool printed instead was `0 common, 40 added, 41 removed`,
`★★ LINE->LABEL DRIFT UNWITNESSED`, and **exit 1** — the loudest possible output for the calmest
transition in the corpus, telling the porter to re-map all forty from scratch.

**The counter-argument, stated fairly.** The AcroForm FQN *is* the fill key: a map naming
`form1[0].Page1[0].f1_03[0]` genuinely cannot fill `topmostSubform[0].Page1[0].f1_03[0]`. So
`added`/`removed` is *true* on the name axis, and the label axis's advertised job is the case where the
name is unchanged. **Why it still stands:**

1. **The two signals are correlated, not independent.** A container is named for the rows it groups
   (`Lines1-11_ReadOrder`, `Lines4a-11_ReadOrder`), so it is renamed **because** lines were renumbered.
   The FQN key therefore deletes boxes from the label axis **preferentially in the pairs where that axis
   matters most.** This is the repo's own recurring shape: *the thing that decides was not the thing
   that knows.*
2. **The information is already loaded and simply not printed.** Attaching each added/removed name's
   printed label — which `compute` has — would let a porter see `f2_16[0]` go from line 27 to 27a. That
   is a print-side omission, not a missing capability.
3. **It is the FR-114 shape verbatim** — a moved line presented as a rename — at the port layer, which
   is precisely what the brief asked me to look for.

**Not fixed, per the brief** (FR-165 landed in this file hours ago). Location: `form_delta.rs::compute`
(the `common` construction), `label_axis`, and `run`'s `print_capped` calls for `added`/`removed`.

### F2 — Important. There is no line-set axis: **62 retired line numbers across the corpus, and `form-delta` reports zero of them.**

The tool's two axes are *field names* and *the line labels of surviving fields*. A **retired printed
line** is in neither. Measured by diffing `xtask label-census` between each pair:

| pair | retired | new | surviving | the retired numbers |
|---|---|---|---|---|
| `f1040--2020 → 2021` | 5 | 6 | 42 | 10a 10b 10c 12 27 |
| `f1040--2021 → 2022` | 7 | 13 | 41 | 1 12a 12b 12c 27a 27b 27c |
| `f1040--2022 → 2023` | 0 | 0 | 54 | — |
| `f1040s1--2020 → 2021` | 4 | 37 | 21 | 18a 18b 18c 19 |
| `f1040s1--2021 → 2022` | 0 | 5 | 58 | — |
| `f1040s3--2020 → 2021` | 6 | 24 | 13 | 12a–12f |
| `f1040s3--2021 → 2022` | 0 | 0 | 37 | — |
| `f1040s8--2020 → 2021` | 6 | 53 | 10 | 2 4 6a 6b 14 15 |
| `f1040s8--2021 → 2022` | **34** | 3 | 29 | 4a–4c 14a–14i 15a–15h … (the whole advance-CTC reconciliation) |
| `f1040sd`, `f8949` | 0 | 0 | 24 / 2 | — |
| **total** | **62** | **141** | **331** | |

Schedule 8812 TY2021 → TY2022 shows the cost: **34 printed lines died** — Parts I-B, I-C and III, the
whole advance-child-tax-credit reconciliation — and `form-delta`'s output for that is *"54 removed"*
field names, of which it prints eight, **none carrying a line label**.

★ The repo **can** answer this: `xtask label-census` (runbook step 14) enumerates the printed line set
from the extract, and the diff above is two invocations and `comm`. **`form-delta` neither runs it nor
names it**, and step 13 sits *before* step 14, so a porter following the runbook in dependency order
meets the field-name churn with no line-set context at all. The cheap fix is a sentence in
`form-delta`'s output pointing at `label-census`; the real fix is a third axis.

### F3 — Important. There is no line-meaning axis, and the reuse set is **331 surviving line numbers**.

§3.1(c) measured 9 of 13 surviving Schedule 3 numbers naming a different quantity. Extrapolation is not
the point; the **structural** point is that a surviving line number is the *only* case in which every
carried-forward cross-reference — a `*Map` field named `line7`, a `LineCoverage` row, a doc comment
reading *"Enter the amount from Schedule 3, line 7"* — **keeps compiling, keeps passing, and now names a
different quantity.**

The runbook assigns this to **step 24** and tags its form-text half **M** (*"P3's `diff -b` of each
line's printed text"*). Two gaps the probe exposes:

1. **Step 24's mechanical half has nothing to bite on in a fresh port.** `Coverage::quoting(year)`
   re-verifies each `LineCoverage.instruction` against the new extract — but a year with no rows yet has
   no sentences to re-verify, so the "M" half is vacuous exactly when the port is new.
2. **Nothing in step 13's output says step 24 is outstanding for a specific line number.** The surviving
   set is computable (`label-census` ∩ `label-census`) and is the exact worklist for step 24. Nobody
   computes it.

★ A mechanical text diff over surviving numbers is a good **generator** and a bad **judge**: on
Schedule 3 it produced 9 candidates, of which **8 were quantity substitutions and 1 (line 2) was a
wording refinement of the same quantity.** That 1-in-9 is the irreducible human residue — and a far
better use of step 24's budget than reading a whole form.

### F4 — Minor. `form_delta.rs` claims twice that the missing-fixture error names the command. It does not.

`form_delta.rs:119-120`: *"so that state is a hard `Err` out of [`compute`] — naming the missing file and
the command that makes it"*. `form_delta.rs:402-403`: *"(A fixture that is MISSING is a hard error before
this point, naming the file and `xtask extract-geometry <stem>`.)"*

Observed:

    xtask form-delta: geometry fixture missing: …/design/forms/geometry/i1040sd--2020.json (No such file or directory (os error 2))

`form_geometry::load` (`form_geometry.rs:141-142`) formats `"geometry fixture missing: {} ({e})"` and
names no command. The file and the io error are there; the actionable half is not. Landed with FR-165,
hours old. **Either the message gains the command or the doc drops the claim** — the repo's own rule is
never to describe code from its doc comment.

### F5 — Minor, and already adjudicated in-repo. The label reader puts the Checking/Savings pair on line **35b**; the form prints it under **"c Type:"**, i.e. 35c.

Form 1040 prints *"▶b Routing number  ▶ c Type: ☐Checking ☐Savings"* on one row. `label-boxes` returns
`c2_05[0] → 35b` and `c2_05[1] → 35b` for TY2020, and the same mis-join for TY2021 (`c2_06`), TY2022
(`c2_05`) and TY2024 (`c2_5`). **★ Because the bias is stable year over year, `form-delta` cancels it and
reports nothing — a diff instrument is structurally unable to see a constant witness error.**

This is a **positive** result about the repo: a human already paid step 16 here. `2024/f1040.map.toml`
(T10 / §5.4) adjudicated it with a genuinely independent second witness — `dump-fields` on-states
(`c2_5[0] on=["1"]` at x=377.4, `c2_5[1] on=["2"]` at x=435.0) plus the printed row's left-to-right
order — and keys the block **semantically** (`checking`/`savings`), not by line number, so the wrong
label never reaches a return. **That is what step 16 costs and what discharges it: a second witness of a
different kind.** Note that `form-delta` carries **one** witness per side, so it can never generate
step-16 work; step 16 lives at step 15, against a map a fresh port does not yet have.

---

## 5. ★ The `labels_available` / zero-comparison fear, tested

The brief asked me to verify that a document with **no AcroForm at all** makes the label axis report its
unavailability rather than silently comparing nothing. **It does — by a stronger mechanism than
predicted, and the predicted arm turns out to be unreachable by this route.**

Two genuinely AcroForm-free prior-year documents (probed: `/AcroForm` absent, 0 `/Widget`):
`i1040sd--2020` (sha256 `6dc26fbfab554df0d00cc53b288e05cf87992c88db769e1391dde907da5b7e4c`) and
`i1040sd--2021` (sha256 `a5c20b717f0543a480c9e661b467004f2e36f8f5d64c9373302125c0439da609`).

1. `xtask extract-geometry i1040sd--2020` → **refuses**, exit 1:
   *"reading AcroForm fields: bundled PDF structure error: catalog has no AcroForm"*. **No fixture is
   written**, so no observation of a label-less document can enter the archive.
2. `xtask form-delta i1040sd--2020 i1040sd--2021` → **hard error**, exit 1: *"geometry fixture
   missing: …"*. It never reaches a verdict, let alone a clean one.

So `no_archived_pair_reports_a_clean_verdict_from_zero_comparisons`'s fear does not materialise, and it
is guarded twice over. ★ **Consequence worth recording:** because `extract-geometry` refuses a
no-AcroForm PDF outright, `LabelVerdict::NoFixture` / `labels_available == false` **cannot be reached by
a form having no AcroForm** — the only route left is a fixture that exists and whose words yield no
printed label column, which is what the `Unwitnessed::GeometryFixtureMissing` doc comment already says.
The brief's expectation was right about the *outcome* and wrong about the *arm*; the code's own doc is
right and the brief's phrasing (*"`labels_available` should be false"*) is not what happens.

★ The one pair that *did* reach an evidence-free verdict, `f1040s3--2021 → 2022`, exited **1** with
*"this run is NOT evidence that no line moved"* — the guard working exactly as designed. **Its honesty
is not in question; its blindness is F1.**

---

## 6. The number Fable's §6 asked for: what steps 16 and 17 actually cost

Per the brief: *"what a human had to do that the tool could not."* Across 11 pairs:

| what the machine produced | count | what a human must then do |
|---|---|---|
| line moves **printed**, each with old and new label | **88** | one step-17 "same line?" decision each. Cheap — the labels are right there. All 88 adjudicated correct. |
| line moves **not printed** (F1) | **71** | find them at all. Requires re-pairing on the leaf index by hand, or reading both forms. **Nothing in the repo does this today.** |
| added/removed field names, **no label attached** | **624** (137 printed) | the only place the 71 hide. Each is a step-17 candidate with zero line information. |
| retired line numbers (F2) | **62** | two `label-census` runs and a `comm`, which no step-13 output suggests |
| surviving line numbers (F3) | **331** | the step-24 worklist. A text diff narrows it: on Schedule 3, 13 → 9 candidates → **8 real**. |
| witness disagreements surfaced by `form-delta` | **0** | step 16 is not reachable from step 13 — one witness per side. The corpus's one real witness error (F5) was found by `dump-fields` on-states, not by any diff. |

**The honest summary of the cost.** Step 17 is **cheap where the tool speaks and unbounded where it is
silent** — 88 easy decisions, versus 71 moves and 62 retirements a human must go and find. Step 16 is
**not exercised by step 13 at all**; it needs a second witness, and the corpus shows the second witness
that works is `dump-fields` on-states plus the printed row, exactly as `2024/f1040.map.toml` records.
The `f6251/2025` 37→43 collision that took two review rounds was not an unlucky pair — it is the
**normal** shape of a year boundary, and it is the shape this corpus produces nine times on one schedule.

### 6.1 One bonus pair, because the archive already held the other side

`form-delta f1040--2023 f1040--2024` — the boundary into a **bundled** year — runs now that 2023 is
archived: 122 common, 19 added, 17 removed, **27 of 81 compared moved** (`f1_32[0]: 1b → 1a`,
`f1_52[0]: 8 → 7`, `c1_22[0]: 7 → 6c`, …). Not a defect (TY2023 is not bundled and never will be), but
it confirms the tool works across a bundled-year boundary and that TY2023→TY2024 was itself a 27-box
renumber.

---

## 7. Scope discipline

- **No year was bundled.** `crates/` is untouched — the porcelain working-tree status for `crates/` is
  empty. No `YEAR.toml`, no `Stem` variant, no params, no compute change, no `.map.toml`, no template PDF.
- **No defect was fixed.** F1–F5 are reported only; `form_delta.rs` and `form_geometry.rs` carry FR-165
  from hours ago and are not touched.
- **No oracle was run.** This probe is about documents.
- No subagents. Every command in the foreground. Target dir `<worktree>/target-probe`, never `/tmp`.

### Files added or changed (nothing committed)

**Modified (1):** `design/forms/MANIFEST.json` — +153 lines, 17 new entries, 0 deletions, 0 modifications.

**Added (1):** `design/agent-reports/RECON-prior-year-delta-probe.md` — this report.

**Added, 51 further committed-surface files** — for each of the 17 stems
`f1040--{2020,2021,2022,2023}`, `f1040s1--{2020,2021,2022}`, `f1040s3--{2020,2021,2022}`,
`f1040s8--{2020,2021,2022}`, `f1040sd--{2020,2021}`, `f8949--{2020,2021}`:

- `design/forms/<year>/<stem>.pdf.txt` — provenance note (URL, sha256, bytes), 17 files
- `design/forms/extract/<stem>.txt` — text layer, recipe proved by `--adopt`, 17 files
- `design/forms/geometry/<stem>.json` — geometry observation, 17 files

**Present but excluded from version control** (`design/forms/**/*.pdf`): the 17 fetched authority PDFs,
plus **125 restored** by `forms fetch --restore`. Two transient no-AcroForm probe PDFs
(`i1040sd--2020/2021`) were fetched, measured, and **deleted** — their hashes are in §5 so the
measurement is reproducible, and no note, extract or fixture was written for them.

### Reproduction

    cargo run -p xtask -- forms fetch --restore                    # if the ignored PDFs are absent
    cargo run -p xtask -- form-delta f1040s3--2021 f1040s3--2022   # F1's extreme case, exits 1
    cargo run -p xtask -- form-delta f1040--2022 f1040--2023       # 12 moves, identical line set
    cargo run -p xtask -- label-boxes f1040s1--2020 | grep f1_29   # F3's testimony case, TY2020: 22
    cargo run -p xtask -- label-boxes f1040s1--2021 | grep f1_29   #                       TY2021: 8z
    cargo run -p xtask -- label-census f1040s3--2020               # F2's line-set axis, by hand

---

## 8. Suggested follow-ups (for the controller to place, not filed by me)

1. **F1 — pair the label axis on something a container rename cannot break, or at minimum print the
   printed-line label beside every `added` and `removed` name.** Owning phase: whichever phase next
   touches `form_delta.rs`. The one finding with a measured magnitude: 71 of 159, 45%.
2. **F2 — give `form-delta` a line-set axis, or make its output name `label-census` as the missing
   step.** 62 retired lines reported as zero.
3. **F3 — compute the surviving-line set as step 24's explicit worklist**, and record that step 24's
   mechanical half is vacuous for a year with no `LineCoverage` rows yet.
4. **F4 — reconcile the two doc claims with `form_geometry::load`'s actual message.** One line either way.
5. **Runbook step 2 — record that the IRS stem is not derivable from the form name** (`f1040s8` is
   Schedule 8812; `f1040s8812` is 404), a fourth instance of the step-1 class.
6. **Retire-vs-reserve is its own shape.** A line retired *as a quantity* while keeping a live box and
   dropping out of its own total (1040 line 30, TY2022; Schedule 3 line 13c, TY2022) loses money with a
   figure still printed on the page and both oracles agreeing. Worth a named check.
