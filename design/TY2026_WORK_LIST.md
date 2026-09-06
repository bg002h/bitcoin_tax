# TY2026 port work list — computed, not estimated

Generated 2026-09-05 by `xtask form-delta <2025-final> <2026-draft>`; **regenerated 2026-09-06**
after the x-aware label join changed the reader behind the "lines that moved" column (fold review
L3: four cells were reader artifacts — f1040s2 27→23, f1040s3 3→2, f1040sa 5→2, **f6251 1→0**, so
Form 6251 does NOT move a line for TY2026). **Regenerate when a final lands, OR when the label reader
changes** — the handoff is a diff, not a rebuild from memory. `xtask`'s
`the_committed_work_list_matches_form_delta_at_head` reds when a row here disagrees with the tool, when
an excused row below actually HAS a pair, or when a stem with a map in any bundled year has no row in either table.
**Regenerate with `cargo run -p xtask -- port-status 2025 2026-DRAFT`** (FR-50): it prints both tables from the
emitting surface; `port_status_prints_the_committed_work_list` holds this document to it. The shape column
is mechanical: **REBUILT** when added + removed exceed the common fields, **unchanged** when nothing was
added, removed or moved with labels compared, **port** otherwise.

★ Drafts are EVIDENCE ONLY. Nothing here is transcribed; these are counts of what CHANGED.

> ## ★★ CORRECTED 2026-09-05 — the first version of this table was WRONG
> Its `lines that moved` column was an artifact of a **draft cover-sheet page offset**. Every
> IRS draft carries a "DRAFT—NOT FOR FILING" cover as page 1, so the form starts on page 2 —
> while box pages come from the AcroForm FQN (`Page1[0]`), which still says 1. The join matched
> page-1 boxes against the cover sheet and produced **plausible** wrong labels.
>
> The first table claimed Form 1040 moved **31** line bindings and Form 6251 **28**, and a commit
> message called that "the trap, on the two forms that matter most". **The true figures are 0
> and 1.** The calibration tests did not catch it because they compare two FINALS, neither of
> which has a cover sheet. Found by an independent recon lens; now guarded by
> `no_committed_geometry_fixture_contains_a_draft_cover_sheet`.

| form | common | added | removed | lines that moved | shape |
|---|---|---|---|---|---|
| `f1040s1` | 72 | 1 | 1 | **UNWITNESSED** — the 2026 draft's label set cannot be read (bare sub-letter `a` at p1 y=120.9 has no numeric parent; FR-58), so the moved count is NOT a number here | port |
| `f1040s1a` | 10 | 175 | 44 | 1 | **REBUILT** |
| `f1040s2` | 44 | 24 | 19 | 23 | port |
| `f1040s3` | 35 | 3 | 2 | 2 | port |
| `f1040sa` | 14 | 33 | 19 | 2 | **REBUILT** |
| `f1040sb` | 72 | 0 | 0 | 0 | unchanged |
| `f1040sc` | 59 | 50 | 46 | 8 | **REBUILT** |
| `f1040sd` | 55 | 0 | 0 | 0 | unchanged |
| `f1040sse` | 25 | 2 | 2 | 0 | port |
| `f6251` | 62 | 0 | 0 | 0 | unchanged |
| `f8949` | 202 | 0 | 0 | 0 | unchanged |
| `f8959` | 26 | 0 | 0 | 0 | unchanged |
| `f8960` | 38 | 0 | 0 | 0 | unchanged |
| `f8995` | 22 | 14 | 11 | 8 | **REBUILT** |

## Not listed — stated PER CELL, never omitted (Fable plan review I8)

The table above enumerates from available fixture **pairs**. That is the wrong denominator: a form
the packet emits with no pair silently has no row — the same "clean verdict from zero comparisons"
shape the cover-sheet correction was about, one level up. Until `forms port-status <year>` exists
(it must enumerate from the emitting surface, `Stem` × year), the cells it would print are:

| form | emitted? | prior side (`--2025` final) | TY2026 side | cell |
|---|---|---|---|---|
| `f1040` | **yes** — the return itself | `f1040--2025` | **NO DRAFT** — the file archived as `f1040--2026-DRAFT.pdf` was the TY2025 form (found 2026-09-05); its fixture is gone and the "199 common, unchanged" row it produced was a reader artifact (removed 2026-09-06 by the work-list test) | **NO DRAFT** |
| `f8283` | yes — `packet.rs:223` | `f8283--2025` (Rev. 12-2025, periodic) | **NO DRAFT** — the draft URL served a 2025 document; archiver refused | **NO DRAFT** |
| `f8275` | yes — `packet.rs:218` | `f8275--2024` (Rev. 10-2024, periodic; aliased by hash for 2025) | **NO DRAFT** — the draft URL served a 2024 document; archiver refused | **NO DRAFT** — periodic; a new revision would be a hash change, not a year |
| `f8995a` | yes — `packet.rs:194` | **NO PRIOR SIDE** — bundled for 2024 only; no `f8995a--2025` authority or extract archived (the port report §5d's "keep" was a to-do, not a fact) | draft archived (`f8995a--2026-DRAFT`) | **NO PRIOR SIDE** → archive `f8995a--2025` + `i8995a--2025`, then diff |

Re-run the archiver when the two periodic forms post a 2026 revision; regenerate this list from the
emitting surface, not from `design/forms/geometry/` pairs.
