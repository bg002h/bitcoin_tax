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

## Not listed

`f8275` and `f8283` have **no TY2026 draft** — the IRS draft URL served a 2024 and a 2025
document, and `scripts/archive_drafts.py` REFUSED both. Re-run the archiver when they post.
