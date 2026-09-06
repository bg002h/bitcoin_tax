# TY2026 port work list — computed, not estimated

Generated 2026-09-05 by `xtask form-delta <2025-final> <2026-draft>`. **Regenerate when a final
lands** — the handoff is a diff, not a rebuild from memory.

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
| `f1040` | 199 | 0 | 0 | 0 | unchanged |
| `f1040s1a` | 10 | 175 | 44 | 1 | **REBUILT** |
| `f1040s2` | 44 | 24 | 19 | 27 | port |
| `f1040s3` | 35 | 3 | 2 | 3 | port |
| `f1040sa` | 14 | 33 | 19 | 5 | port |
| `f1040sb` | 72 | 0 | 0 | 0 | unchanged |
| `f1040sc` | 59 | 50 | 46 | 8 | port |
| `f1040sd` | 55 | 0 | 0 | 0 | unchanged |
| `f1040sse` | 25 | 2 | 2 | 0 | port |
| `f6251` | 62 | 0 | 0 | 1 | port |
| `f8949` | 202 | 0 | 0 | 0 | unchanged |
| `f8959` | 26 | 0 | 0 | 0 | unchanged |
| `f8960` | 38 | 0 | 0 | 0 | unchanged |
| `f8995` | 22 | 14 | 11 | 8 | port |

## Not listed — stated PER CELL, never omitted (Fable plan review I8)

The table above enumerates from available fixture **pairs**. That is the wrong denominator: a form
the packet emits with no pair silently has no row — the same "clean verdict from zero comparisons"
shape the cover-sheet correction was about, one level up. Until `forms port-status <year>` exists
(it must enumerate from the emitting surface, `Stem` × year), the cells it would print are:

| form | emitted? | prior side (`--2025` final) | TY2026 side | cell |
|---|---|---|---|---|
| `f1040s1` | **yes** — `packet.rs:104`, whenever Schedule 1 has content (crypto ordinary income lands there) | **NO PRIOR SIDE** — bundled for 2024 only; no `f1040s1--2025` authority archived | draft archived (`f1040s1--2026-DRAFT`) | **NO PRIOR SIDE** → archive `f1040s1--2025`, then diff |
| `f8283` | yes — `packet.rs:223` | `f8283--2025` (Rev. 12-2025, periodic) | **NO DRAFT** — the draft URL served a 2025 document; archiver refused | **NO DRAFT** |
| `f8275` | yes — `packet.rs:218` | `f8275--2024` (Rev. 10-2024, periodic; aliased by hash for 2025) | **NO DRAFT** — the draft URL served a 2024 document; archiver refused | **NO DRAFT** — periodic; a new revision would be a hash change, not a year |
| `f8995a` | yes — `packet.rs:194` | **NO PRIOR SIDE** — bundled for 2024 only; no `f8995a--2025` authority or extract archived (the port report §5d's "keep" was a to-do, not a fact) | draft archived (`f8995a--2026-DRAFT`) | **NO PRIOR SIDE** → archive `f8995a--2025` + `i8995a--2025`, then diff |

Re-run the archiver when the two periodic forms post a 2026 revision; regenerate this list from the
emitting surface, not from `design/forms/geometry/` pairs.
