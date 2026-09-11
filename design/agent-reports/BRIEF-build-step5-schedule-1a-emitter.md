# BRIEF — year-package table, design r2 §10 **step 5**: wire `f1040s1a/2025` (the Schedule 1-A emitter)

**Authority:** `design/FORM_AUTHORITY_TABLE_DESIGN.md` §10 step 5. Steps 1–4 are DONE. Step 5 is
**eight of ten wired**; two `Schema::Unwired` revisions remain. **You are wiring exactly one of them:
`f1040s1a/2025`.** The other (`f6251/2025`) is explicitly NOT in your scope — see §6.

Owner authorised this on 2026-09-11. Hard stop for the step: **2026-10-31**.

You are ONE opus builder in the **shared main tree**. Nothing is committed while you work; the
coordinator machine-checks and commits.

---

## 0. The instruction that outranks the rest

**Stop and report rather than build on any premise in this brief you can disprove.** Four briefs in this
arc have been refuted by measurement, two of them written by this coordinator — and **a fifth was refuted
while this brief was being written**: the coordinator told the owner that step 5's `f6251/2025` blocker was
"already cleared" because `Form6251Line1::Y2025 { line1a, line1b }` exists. That struct is in
**`btctax-core`** — the AMT *computation*. The forms-crate *transcription* struct `Form6251Map`
(`map.rs:302`) still carries the 2024 single `pub line1: MoneyCell`. **Two layers, conflated.** The
correction is why your scope is Schedule 1-A and not Form 6251.

So: if a fact in §2 is wrong, that is your most valuable finding. Report it and stop.

## 1. What you are delivering

`LineSet::F1040s1a_2025` moves from `Schema::Unwired` to a real schema, so the bundled TY2025 Schedule 1-A
map **parses into a transcription struct** and the form **fills and reads back**.

Concretely: a `Schedule1AMap` transcription struct, a filler, the `Schema` variant, the two pins updated,
and tests. Not a packet change — see §5.

## 2. Machine-verified facts — do not re-derive these

Measured by the coordinator at `b20f7772`:

| fact | evidence |
|---|---|
| `LineSet::F1040s1a_2025 => Schema::Unwired` | `line_set.rs:315` |
| the Unwired set is pinned as exactly two | `line_set.rs:357-363`, `the_unwired_set_is_exactly_the_two_ty2025_maps_step_5_could_not_wire`, asserting `["f1040s1a/2025", "f6251/2025"]` |
| no struct exists for this stem at all | `tests/line_set_wiring.rs`: `Stem::F1040s1a => return None, // no struct exists (Schedule1AMap: count 0)` |
| the map is on disk and complete | `crates/btctax-forms/forms/2025/f1040s1a.map.toml`, 231 lines, `[census]` at `:231` |
| its header is already correct | `form = "f1040s1a"`, `year = 2025`, `irs_stem = "f1040s1a"`, `versioning = "annual"`, `line_set = "f1040s1a/2025"`, `attachment_sequence = "1A"`, `template_sha256 = 64f97b38…` |
| the map, the archived authority and the geometry fixture are the same document | the map's own header block: one sha256 across `forms/2025/f1040s1a.pdf`, `design/forms/2025/f1040s1a--2025.pdf` and `design/forms/geometry/f1040s1a--2025.json` |
| **54 PDF fields, every one `text`** — no checkboxes, no radio groups, no ReadOnly widget | the map header, from `xtask dump-fields` |
| ★ **root subform is `form1[0]`**, as on Schedules 1 and 2 — NOT `topmostSubform[0]` | the map header's own warning. Read it per form; never assume |
| **nothing descends from a prior year** | Schedule 1-A was created by Pub. L. 119-21; `forms/2024/` holds no such map and `design/forms/extract/` no `f1040s1a--2024.txt`. Every field name was read from the blank |
| TY2025's year record **expects** this form | `forms/2025/YEAR.toml` `forms_expected` includes `"f1040s1a"` |
| TY2024 and TY2026 declare it absent **with reasons** | TY2024: "Schedule 1-A is TY2025+ (OBBBA)"; TY2026: "revision not released; draft archived (REBUILT: 175 added fields); January 2027 package" |
| the compute side already exists and is wired to the 1040 | `btctax_core::tax::schedule_1a::Schedule1A`, `Schedule1A::compute` called at `return_1040.rs:2435`, feeding `schedule_1a_additional` |
| core's printed line set is **48 distinct lines** | `line1, 2a–2e, 3, 4a–4c, 5–10, 11_steps, 12, 13, 14a–14c, 15–18, 19_steps, 20, 21, 22a, 22b, 23–27, 28_steps, 29–35, 36a, 36b, 37, 38` |
| `packet.rs` knows **nothing** about Schedule 1-A | zero hits for `F1040s1a`/`Schedule1A`/`schedule_1a` in `crates/btctax-forms/src/packet.rs` |

## 3. The rules that bind this transcription

`CLAUDE.md`, "Transcribe IRS forms — never paraphrase them", applies in full and is the highest-risk part
of this task:

- **One field per numbered line, named for the line, in the form's own numbering**, carrying the official
  instruction text **verbatim** as its doc comment. A derived or closed form needs a written equivalence
  proof naming the branch where it breaks, plus a KAT pinning that branch — absent both, transcribe.
- ★★ **Transcribe from the TEXT LAYER, never the rendered page.** Use
  `design/forms/extract/f1040s1a--2025.txt`. This exact rule was learned on Form 6251: a rendered `12` read
  as `22` and inflated the tentative minimum tax by $200,000 on one vector. **Then re-verify every
  cross-reference ("enter the amount from line N") against the extracted text.**
- **Per line-set revision.** `Schedule1AMap` describes what the **TY2025** revision prints. TY2026's
  Schedule 1-A is a REBUILD (175 added fields), so it will need its own struct — do not build one struct to
  straddle both, and do not name this one as though it were year-agnostic.
- **Blank is the normal case.** Most lines on a real Schedule 1-A are empty. Never assert
  non-blankness; the invariant is that every line has determinate **provenance**. A hardcoded `0` on a line
  nobody answered fabricates sworn testimony.
- **Derive the list, or make the compiler hold it.** Enumerate the expected line set **from the form's
  extracted text**, never from a range (`1..=38` is wrong here — the label set is larger) and never from a
  hand-written list.

## 4. The pattern to follow

- **Struct:** `Schedule1Map` (`map.rs:3509`) is the model — the `MapRow` header fields first
  (`form`, `year`, `irs_stem`, `versioning`, `template_sha256`, `line_set`, `attachment_sequence`, …), then
  the line cells. Required header fields **refuse** when missing.
- **Filler:** `schedule23.rs` holds the Schedules 1/2/3 fillers (`fill_*_with_map(map: &Schedule1Map, …)`).
  Note `schedule23.rs:74-83`/`:195-203` — every plan declares a column **per line**, which is the FR-102
  fix; follow that, do not reintroduce a shared column constant.
- **Tests:** `tests/f6251_map.rs` + `tests/f6251_fill.rs` (and the `f8995a` pair) are the models — a
  map-parse test and a fill test with **read-back verification** against the blank's field geometry.
- **Wiring:** add the `Schema` variant, map `LineSet::F1040s1a_2025` to it, and delete it from the
  `Unwired` arm. The exhaustive `schema()` match means a revision without an arm cannot compile.

## 5. The boundary you must STATE, not silently leave

`packet.rs` does not know Schedule 1-A, and **no currently computable year can exercise a packet
containing it**: TY2024 has no such schedule, and `full_return_for(2025)` is a tested `None` (the year is
paused), so there is no TY2025 full return to build a packet from. The emitter is therefore verifiable in
isolation (fill the blank, read back) but **not through a packet on any year the product can compute today.**

That is an honest boundary and it must be **visible in the source**, not implied — otherwise a later reader
assumes Schedule 1-A prints in a packet when nothing does that. So:

- say it in the struct's / filler's doc comment, naming what is and is not exercised;
- **file a follow-up in `FOLLOWUPS.md`** for packet inclusion + the `"1A"` attachment sequence, with its
  owning phase being the year that can actually print it (the TY2026 January-2027 package, or the S1 TY2025
  rehearsal if the owner rules it);
- do **not** invent a fixture that pretends a year computes when it does not, and do **not** bundle any
  `FullReturnParams` — `ty2025_full_return_must_stay_fail_closed_until_complete` and
  `ty2026_full_return_must_stay_fail_closed` are deliberate gates and are outside your scope.

## 6. Explicitly NOT in scope

- **`f6251/2025`** — the other `Unwired` revision. §10 step 5 says write that transcription struct once for
  TY2026 "which shares the layout". ★ Record, but do not act on, one observation for the owner: TY2025's and
  TY2026's Form 6251 Part I are **not** identical (2025's line 2a cites 1040 line 12e; 2026's line 1a
  subtracts Schedule 1-A line 43), so "one struct for both" may itself be a premise worth refuting — and
  per-revision transcription implies two structs regardless. Put that in your report, change no code for it.
- Bundling any year's `FullReturnParams`; touching the TY2025/TY2026 fail-closed gates.
- TY2026 rows or `forms-provisional/2026/` (step 6, after finals).
- Anything transcribed from a DRAFT. `Entry::is_draft` enforces this; the TY2026 Schedule 1-A draft is
  archived but **must not** be transcribed.

## 7. Kills — B1: no checker exists until it has been seen RED on a planted defect

Every one of these must be **run against the pre-fix state and its red output pasted** into your report:

1. **The map parses.** `Schedule1AMap::parse(map_text(Stem::F1040s1a, 2025))` is `Ok` — and red before the
   struct exists.
2. **A required header field missing → parse refusal** (the step-1 kill, applied to this map).
3. **`template_sha256` ≠ the file → refusal.**
4. **Fill + read-back**: every cell the filler writes is read back from the filled PDF at the geometry the
   blank declares. Plant a wrong field name → red.
5. **The line set is derived from the extract**, and a line dropped from the struct reds. This is the
   conformance kill — it must enumerate from `design/forms/extract/f1040s1a--2025.txt`, not from a list.
6. **The shrink-only pin shrinks**: `the_unwired_set_is_exactly_the_two_…` now asserts `["f6251/2025"]`
   alone. Re-read its doc comment — it is a ratchet, and its name no longer matches its content, so rename
   it to say what it now pins.
7. **`tests/line_set_wiring.rs`**: `Stem::F1040s1a`'s `return None, // no struct exists` is now false.
   ★ Read that file's header before editing: its helper is named `parses_into_2024_struct` and Schedule 1-A
   has **no 2024 counterpart**, so the honest edit may be to the helper's shape, not just its arm. Say what
   you chose and why.

## 8. Validation

`make check` (baseline at `b20f7772`: **3571 passed / 12 skipped**, fmt clean) plus every instrument that
touches forms: `xtask line-coverage-check`, `census-join`, `box-census`, `form-geometry`, `cite-check`,
`prompt-check`, and the crate's own `year_record`, `map_rows`, `map_pdf_conformance`, `field_census`,
`full_return_forms`, `supported_years_cross_product` tests. **Report each as a number, not as "green".**
If an instrument goes green *without* having traversed the new form, that is a finding about the instrument —
say so (FR-114's shape).

## 9. Mechanics — not negotiable

- **Main tree. Do NOT commit, push, `git stash`, `git checkout`, or revert anything.** A builder ran
  `git stash` against a brief in this arc and it had to be recovered by `pop`. To revert a mutation, use a
  **cp backup**, never `git checkout --`.
- **Do not spawn subagents.**
- Never hand-count what a tool can count. Run it and paste the value.
- No `--no-verify` anywhere; a hook denies it.

## 10. Your report — final action

Write `design/agent-reports/REPORT-build-step5-schedule-1a-emitter.md`, then return only a short summary and
that path. It must contain: what you built and where (`file:line`); **the transcription's provenance** (which
extract lines became which fields, and how you re-verified every cross-reference); **kills** with pasted
red-then-green for each of §7; the §5 boundary as you stated it and the follow-up you filed; the §6
Form 6251 observation; **refuted premises**; and the literal validation numbers.
