# Recon: what a TY2024→TY2025 form port actually costs

**Scope.** Measure, not guess, whether "one year done ⇒ the next year is easy" holds for the 12 forms
TY2025 is missing (`f1040s1`, `s2`, `s3`, `sa`, `sb`, `sc`, `f6251`, `f8275`, `f8959`, `f8960`, `f8995`,
`f8995-a`). Recon only; no tracked file was modified. All counts below are commands run in this session,
not hand counts.

**Bottom line.** The claim is **true for 8 of 12 forms** — same field layout, same line numbers, and for
one form (`f8275`) a **byte-identical PDF**, so the port is a copy-and-verify. It is **false for the two
forms the tax-law change actually touches**: Schedule A (new SALT cap + a brand-new phase-out worksheet)
and Form 6251 (Part I restructured around the new Schedule 1-A), plus ripple damage into Schedule 2. The
codebase's own `tax_tables.rs` already carries a fail-closed test that names this exact set as the
blocker — this recon's independent PDF-diff arrived at the same four items from the other direction.

---

## 1. Archive status — 9 of 12 ready to map, 3 need fetching

Checked `design/forms/2025/`, `design/forms/extract/`, and `design/forms/MANIFEST.json` (no entry for a
document = not archived):

| form | PDF in `design/forms/2025/` | extract in `design/forms/extract/` | verdict |
|---|---|---|---|
| Schedule 1 (`f1040s1`) | **absent** | **absent** | needs fetching |
| Schedule 2 (`f1040s2`) | present | present | ready |
| Schedule 3 (`f1040s3`) | present | present | ready |
| Schedule A (`f1040sa`) | present | present | ready |
| Schedule B (`f1040sb`) | present | present | ready |
| Schedule C (`f1040sc`) | present | present | ready |
| Form 6251 | present | present | ready |
| Form 8275 | **absent** | **absent** | needs fetching |
| Form 8959 | present | present | ready |
| Form 8960 | present | present | ready |
| Form 8995 | present | present | ready |
| Form 8995-A | **absent** | **absent** | needs fetching |

I fetched the 3 missing PDFs to scratchpad (not the repo) to complete the delta analysis below, and
checked cadence with a live HTTP probe:

| form | `irs-prior/{stem}--2025.pdf` | fallback used |
|---|---|---|
| `f1040s1--2025.pdf` | `200` | none needed |
| `f8275--2025.pdf` | **`404`** | `irs-pdf/f8275.pdf` → `200` (periodic cadence, exactly the pattern `design/forms/README.md` already documents for `f8275r`) |
| `f8995a--2025.pdf` | `200` | none needed |

`f8275`'s fetched "TY2025" PDF hashes **identical** to the already-archived TY2024 PDF
(`sha256:9b4b82e3d0dd4ece…`, matches `design/forms/MANIFEST.json`'s TY2024 entry exactly). It is Rev.
10-2024 and the IRS has not re-revised it — this form's "port" is zero-diff.

## 2. The delta, form by form

Diffed `design/forms/extract/{stem}--2024.txt` against `design/forms/extract/{stem}--2025.txt` (for the
9 already archived) and against a freshly-extracted `pdftotext -layout` of the 3 fetched PDFs, with
`diff -b` to suppress whitespace-only reflow noise. AcroForm field geometry compared with
`cargo run -p xtask -- dump-fields <pdf>` on the real (gitignored, locally-present) PDFs.

| form | text lines 2024→2025 | AcroForm fields 2024→2025 | field diff | verdict | evidence |
|---|---|---|---|---|---|
| Schedule 1 | 97→102 | not compared (PDF only just fetched) | — | **SMALL** | same line numbers throughout; lines 4, 7, 14, 20 each gained a new checkbox/sub-entry (Form 4797/4684 checkboxes on L4; "repaid a 2025 overpayment, check + amount" on L7; storage-fee checkbox on L14; MFS-lived-apart checkbox on L20). No renumbering. |
| Schedule 2 | 131→131 | 60→63 | 55 differing lines | **SMALL–MODERATE** | L1e/L1f simplified (column-(n)/(o) sub-references removed); **L4 gained a 3-checkbox exemption block** (§4361/§4029 self-employment-tax exemption, new fields); **L10 "Repayment of first-time homebuyer credit" is retired**, replaced by "Reserved for future use" — same line number, different meaning. |
| Schedule 3 | 56→58 | 39→37 | 34 differing lines (mostly geometry reflow) | **TRIVIAL** | text-level diff is only year stamps + reflow; 2 fewer AcroForm fields is a minor internal consolidation with no line-number or content change observed. |
| **Schedule A** | 71→75 | 37→33, **root subform renamed** `topmostSubform[0]`→`form1[0]` | 70 differing lines | **STRUCTURAL** | SALT cap raised **$10,000/$5,000 → $40,000/$20,000**, with a **brand-new MAGI phase-out** ("If Form 1040 line 11b is more than $500,000 ($250,000 MFS)… see instructions") that does not exist on the 2024 form — this is new tax *logic*, not a threshold bump. L2 and L17 cross-references moved from Form 1040 "line 11"/"line 12" to "line 11b"/"line 12e" because Form 1040 itself restructured (§ below). |
| Schedule B | 80→80 | 72→72 | 4 differing lines | **TRIVIAL** | only year stamps changed; the 4 field-geometry diffs are one checkbox pair nested one subform level deeper — cosmetic. |
| Schedule C | 140→140 | 105→105 | 6 differing lines | **SMALL, with a gotcha** | same line numbers, but **line 27a/27b traded places**: 2024's 27a ("Other expenses") is 2025's 27b, and vice versa for "Energy efficient commercial buildings deduction". The AcroForm field **names** (`f1_39`, `f1_40`) stayed pinned to the *content*, not the letter — `f1_39` is "other expenses" in both years, just under a different letter. A port keyed to the printed letter (`line27a =`) would silently swap the two; a port keyed to the field name is unaffected. The existing 2024 map already keys by the descriptive line label (`"27a"`/`"27b"` in its census, tied to field name) — worth flagging explicitly when Schedule C is ported. |
| **Form 6251** | 147→148 | 61→**62** (one new field) | 125 differing lines | **STRUCTURAL** | Line 1 splits into **1a/1b**: *"1a Subtract Schedule 1-A (Form 1040), line 37, from Form 1040… line 14"* / *"1b Subtract line 1a from Form 1040… line 11b"* — directly wired to the new Schedule 1-A. Line 4's combine-range changes from "lines 1 through 3" to **"lines 1b through 3"**. All indexed thresholds moved (exemption $85,700→$88,100, phase-out start $609,350→$626,350, 26/28% breakpoint $232,600→$239,100, capital-gains breakpoints, etc. — inflation, not structural, but every one is a hardcoded constant somewhere). |
| Form 8275 | 85→79\* | not compared (byte-identical PDF) | — | **ZERO** | fetched "2025" edition is the *same PDF* as the archived 2024 one (sha256 match). No port work at all beyond copying the map and re-labelling the year, if even that. |
| Form 8959 | 69→69 | **26→26, 0 field-geometry differences** | 0 | **TRIVIAL** | text delta is year stamps only; AcroForm fields are byte-identical in name **and** coordinates. |
| Form 8960 | 69→69 | **38→38, 0 field-geometry differences** | 0 | **TRIVIAL** | same as 8959 — perfectly stable form. |
| Form 8995 | 59→60 | 33→33 (field *names* re-padded `f1_3`→`f1_03`, rows renamed `Ln1A_Row1`→`Row1i`, no content change) | 15/44\*\* | **SMALL** | pure inflation bump: TI-before-QBI threshold **$191,950/$383,900 → $197,300/$394,600**; cosmetic OMB-number change (1545-2294→1545-0074); same line numbers. |
| Form 8995-A | 103→103 | not compared (PDF only just fetched) | — | **SMALL** | same threshold bump as 8995, same 3 places, plus the Part III phase-out ceiling **$241,950/$483,900 → $247,300/$494,600**. Same line numbers throughout. |

\* Line-count drop for 8275 is `pdftotext` header/footer whitespace variance between my ad-hoc extract
and the committed one, not a content difference — the PDFs are bit-identical (verified by hash), which
is the stronger check.
\*\* `diff -b` shows 44 raw differing lines but only 15 are content (renamed subform/field labels); the
remainder is coordinate reflow from the renamed rows.

### Why Schedule A and Form 6251 are correctly classified STRUCTURAL, not SMALL

Both are corroborated **independently of this recon** by the codebase itself. `crates/btctax-adapters/src/tax_tables.rs`
carries a deliberate fail-closed test, `ty2025_full_return_must_stay_fail_closed_until_complete`, whose
doc comment lists the exact blockers before TY2025 full-return support may exist:

> "TY2025 computed with Form 6251 line 1 in 2024 numbering (2025 splits it into 1a/1b),
> `AbsoluteReturn.line14` as 'L12 + L13' when the 2025 form says 'Add lines 12e, 13a, and 13b', a scalar
> `salt_cap` against a §164(b) phase-down, and Schedule 1-A absent entirely."

Every clause in that sentence is a form I diffed and independently confirmed. `design/ty2025/SPEC.md`
§5.5 (already written, before this recon) transcribes the SALT phase-down worksheet line-by-line, so the
Schedule A logic gap is already scoped — it just is not yet built.

### The ripple: Form 1040 itself changed underneath everything

Form 1040 is one of the 5 forms TY2025 *already* ships, so it looked "done." Diffing its extract shows
it was restructured more than a glance suggests: the dependents block moved from row-per-dependent to
column-per-dependent, the filing-status block reworded, and — the part that matters here — **line 11
split into 11a/11b and line 12 exploded into 12a–12e** (with new sub-lines 13a "QBI deduction" and
**13b "Additional deductions from Schedule 1-A, line 38"**, feeding line 14 = "Add lines 12e, 13a, and
13b"). Every other form that used to say "Form 1040… line 11" or "line 12" (Schedule A, Form 6251) had
to update its own cross-reference to match. A change to the *anchor* form forces a re-check of every
dependent schedule's citations even when that schedule's own content barely moved — Schedule B/8959/8960
don't cite 1040 lines 11/12 and were untouched by this; Schedule A and 6251 do, and were.

## 3. What a `.map.toml` actually contains

Read `f8959.map.toml` (71 lines), `f6251.map.toml` (164 lines), `f1040sa.map.toml` (108 lines), and
`f8995.map.toml` (104 lines) from `crates/btctax-forms/forms/2024/`. Measured composition (grep, not
eyeballed):

| file | total lines | comment/prose lines | `lineN = "field"` mappings | `[census]` unmodeled-field entries |
|---|---|---|---|---|
| f8959.map.toml | 71 | 35 (49%) | 17 | 7 |
| f8995.map.toml | 104 | 59 (57%) | 16 | 12 |
| f1040sa.map.toml | 108 | 55 (51%) | 19 | 13 |
| f6251.map.toml | 164 | 94 (57%) | 41 | 18 |

So roughly **half of every map file is hand-written prose** (provenance reasoning, geometry notes, the
"why this field is unmodeled" justification the harness's answered-ness rule requires), a fifth to a
third is the actual `lineN = "fully.qualified.field[0]"` mapping, and the rest is the census/identity
scaffolding.

**Can (a) — field names — be generated? Yes, mechanically, today.** `cargo run -p xtask -- dump-fields
<pdf>` dumps every AcroForm field name, type, geometry and maxlen in one shot; I ran it against all 9
already-archived 2025 PDFs plus 6251 in this session with no code changes needed.

**Can (b) — the line→field correlation — be generated? Partially, and it already exists as a tool.**
`extract-geometry <stem>` + `label-census <stem>` (and the underlying `label-boxes <stem>`, which prints
a `field-name → line-label` TSV directly) derive the mapping geometrically: which printed line label sits
above which AcroForm box, by monotonic-label-column detection plus "last label at or above the box's
centre." I ran `extract-geometry f6251--2025` + `label-census f6251--2025` in this session (output not
committed) and it worked end-to-end on a form it had never seen — **but it silently dropped line "1a"**,
the exact new sub-line Pub. L. 119-21 added, going straight from nothing to "1b" in its output. This is
concrete, reproducible evidence that the tool is calibrated on stable forms and needs re-validation on
every *restructured* one — it has a B1-style planted-defect test only for Schedule 1-A (`design/forms/geometry/`
holds exactly 3 fixtures: `f1040--2024`, `f1040s1a--2025`, `f1040sa--2024`); none of the 12 target forms
have been run through it and committed yet.

**(c) — the hand-written census/provenance prose — is not automatable.** Deciding *why* a field is
`unmodeled` (btctax doesn't collect RRTA income; there's no expense-category breakdown for Schedule C)
is a domain judgment about what btctax's engine currently computes, not a fact the PDF or the tool can
derive.

## 4. The cost model

**What a tool derives:** the field inventory (`dump-fields`, exact and free), and — for forms whose line
layout didn't change — a strong first-draft line↔field correlation (`extract-geometry` + `label-boxes`).
For a **stable** form (8275, 8959, 8960, and likely 8995/8995-A/1040sb/1040s3/1040s1 given their small,
same-shaped diffs above) this makes the .map.toml port close to: copy the 2024 file, re-point the `year`
and PDF path, re-run `dump-fields` to confirm every field name is unchanged, and re-verify the sha256 in
the header note. That is a table entry, exactly as the owner expects.

**What a human must still decide, every time:** (1) whether any line's *meaning* changed even when its
number didn't (Schedule C's 27a/27b swap, Schedule 2's retired line 10); (2) whether a form's own
AcroForm authoring convention changed (Schedule A's `topmostSubform[0]`→`form1[0]`); (3) new
checkboxes/sub-fields that appeared on an otherwise-stable line (Schedule 1's four, Schedule 2's three)
— census them as unmodeled or model them, a product decision; (4) for the two forms actually rewritten
by law (Schedule A, Form 6251), the new computation itself — the SALT §164(b) phase-down worksheet and
the Schedule 1-A-aware AMTI line — which is tax-logic work in `btctax-adapters`, not PDF-mapping work at
all, and has its own dependency (6251 cannot be finished before Schedule 1-A is).

**Estimate for the 12, ranked by measured evidence, not intuition:**

| tier | forms | why |
|---|---|---|
| zero/near-zero | Form 8275 | byte-identical PDF; copy the map, done |
| mechanical (table entry) | 8959, 8960 | fields 100% identical; year-bump + hash re-verify |
| small (table entry + a few new census lines) | 8995, 8995-A, Schedule B, Schedule 3, Schedule 1 | same field/line layout, only threshold constants or new optional checkboxes |
| moderate (needs a human decision, not just a copy) | Schedule 2, Schedule C | Schedule 2's retired line 10 + new exemption checkboxes need a product call; Schedule C's letter-swap needs the port to key on field name, not printed letter, or it silently reverses two dollar amounts |
| **hardest — new tax logic, not just a map** | **Schedule A**, **Form 6251** | Schedule A needs the new SALT §164(b) worksheet built in the engine (already specced, not built) and its AcroForm root-subform convention changed; Form 6251 needs Schedule 1-A to exist first, a restructured Part I combine-range, and is the one thing the codebase itself currently refuses to enable for TY2025 (fail-closed test, cited above) |

**The three hardest, ranked:**

1. **Form 6251** — structural PDF change *and* a genuine cross-form dependency on Schedule 1-A *and* the
   project's own fail-closed gate names it as the blocker. Not shippable until Schedule 1-A is done, which
   this branch is already building.
2. **Schedule A** — the SALT cap increase is not a constant swap; it's a new phase-out worksheet (10 new
   lines of logic per `design/ty2025/SPEC.md` §5.5), plus the one AcroForm naming-convention change found
   in this recon.
3. **Schedule 2** — smallest of the three, but the retired "Repayment of first-time homebuyer credit"
   line and the new §4361/§4029 self-employment-tax-exemption checkboxes are the kind of "same line
   number, different meaning" trap that a diff-blind port (bump the year, keep the prose) would miss
   silently, since nothing forces a re-read of a line whose number didn't move.

## 5. What makes year-porting harder than "just a table entry" — summary of concrete findings

- **Schedule 1-A is genuinely new for TY2025**, created by Pub. L. 119-21 — confirmed in
  `design/forms/README.md`'s own note (*"`f1040s1a--2024.pdf` does not exist because Schedule 1-A was
  created by Pub. L. 119-21 for TY2025"*) and by there being no 2024 counterpart anywhere in the archive.
- **Form 6251's Part I was reworked specifically to integrate Schedule 1-A** — verified: line 1 splits
  into 1a ("Subtract Schedule 1-A… line 37, from… line 14") and 1b, and line 4's combine-range shifts to
  "lines 1b through 3." Independently corroborated by `tax_tables.rs`'s fail-closed test.
- **A PDF's own AcroForm field-naming convention can change between tax years on the same form** —
  measured on Schedule A only (`topmostSubform[0]` → `form1[0]`), not on any of the other 8 forms tested.
  This would silently break a path-prefix assumption baked into tooling rather than read per-form.
  Schedule 2 already uses `form1[0]` in *both* years, so this is a per-PDF authoring-tool artifact, not
  a blanket TY2025 convention — it must be checked per form, not assumed.
- **A line's printed letter can swap content while the underlying field name does not** (Schedule C
  27a/27b) — the trap runs the *opposite* direction from what's intuitive: keying a port off the printed
  form (safer-seeming) is what breaks here; keying off the field name is what survives.
- **A line can go quietly "Reserved for future use"** (Schedule 2 line 10, the expired first-time-
  homebuyer-credit repayment) — same line number, no signal to a numeric diff, only visible by reading
  the line text.
- **Thresholds are partially pre-seeded but not complete.** `tax_tables.rs` already carries TY2025
  bracket/AMT-phaseout constants (e.g. `$626,350` AMT phase-out start is already coded), so the
  computation engine is ahead of the PDF-mapping layer for some numbers — but the QBI TI-before-deduction
  threshold ($197,300) is not yet a labelled TY2025 field (only present, coincidentally, as a 32%-bracket
  boundary), and the SALT cap is explicitly called out in the fail-closed test as still a bare scalar
  needing to become a worksheet.
- **The label-reading automation is unproven outside Schedule 1-A.** Running it fresh against Form 6251
  TY2025 in this session dropped line "1a" — the exact newly-added line — without any code change; per
  this repo's own B1 rule ("no checker exists until it has been observed RED on a planted defect"), this
  tool needs a planted-defect test on a *restructured* form, not only on the stable anchors it has today,
  before its output can be trusted for the two forms (6251, Schedule A) that actually restructured.

## Files referenced

- `design/forms/2025/`, `design/forms/2024/`, `design/forms/extract/`, `design/forms/MANIFEST.json`,
  `design/forms/README.md`, `design/forms/LABEL_READER.md`
- `crates/btctax-forms/forms/2024/*.map.toml`, `crates/btctax-forms/forms/2025/*.map.toml`
- `crates/xtask/src/dump_fields.rs`, `crates/xtask/src/form_geometry.rs`, `crates/xtask/src/label_reader.rs`
- `crates/btctax-adapters/src/tax_tables.rs` (`ty2025_full_return_must_stay_fail_closed_until_complete`)
- `design/ty2025/SPEC.md` §5.4/§5.5, `design/ty2025/SPEC_schedule_1a.md`
