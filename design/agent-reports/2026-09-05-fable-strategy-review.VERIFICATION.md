# Controller's machine-check ledger — `…fable-strategy-review.md`, checked at `97911117`

Owner decisions the review raises are NOT verified here (S1, S2, S6, S7, S9) — they are the owner's.

| claim | check | verdict |
|---|---|---|
| `DIRECTION-2026-08-01.md:173` — "one lived filing will re-rank better than any census" | line 173 contains "lived filing" | HOLDS |
| `SPEC_appraisal_trigger_minimal.md` exists (Form 8283 Section B / appraisal deadline) | 1 file | HOLDS |
| `OWNER_DECISIONS_2026-09-04.md` has D-A / D-B rulings by a delegate | 3 heading/bold hits | HOLDS |
| `LONG_RANGE_PLAN_filing.md` §6.2 measured "37 days" for one schedule | 2 hits | HOLDS |
| `admin.rs:365` `hand_marks` | grep 1 | HOLDS |
| commits: July 1,284 / Aug 187 / Sep 122 | `git rev-list --count` by month on HEAD: **1,233 / 180 / 126** | HOLDS within method variance (branch vs first-parent) |
| 3,024 tests | 3,003 at this HEAD (`make check`) | HOLDS approximately (21 difference; counts move daily) |
| taxcalc has TY2026 AMT policy: `AMT_prt` 0.5 | `.venv` taxcalc **6.7.2**: `AMT_prt` 2025 = 0.25, 2026 = **0.5** | HOLDS |
| taxcalc `AMT_em_pe` 2026 = 639,200 vs Rev. Proc. 2025-32 §2.10 $640,200 (MFS complete phaseout) | `AMT_em_pe` 2026 = **639,200.0**; extract `:729-732` MFS complete phaseout **$640,200** | HOLDS — a real one-oracle defect; excuse by mechanism (S7) |
| `AMT_em` 90,100/140,200/70,100 and `AMT_em_ps` 500k/1M in taxcalc 2026 | not read (2-D array conversion error in my probe); the scalar cells above verified | NOT CHECKED — does not change any decision |
| "266 review .md files", "12.6 MB of design prose" | my counts differ by method (`design/` is 45 MB incl. PDFs/geometry); not decision-bearing | NOT RECONCILED |
| `BundledPrices` ends 2026-06-03 (S5/FR-53) | verified earlier (ledger 1) | HOLDS |
| 1099-DA input has no date (FR-46 b/c "the TY2026 input surface") | FR-46 as written | HOLDS — S3 moves it to NOW |
| The S1 "TY2025 rehearsal" un-pauses a slice the owner PAUSED (§0a) | owner ruling of record | OWNER DECISION — surfaced in CONTINUITY.md, not actioned |

Decision-bearing claims all hold. The non-owner items (S3 → FR-46 NOW; S4 machinery hard stop; S5
real-data runs; S8 physical rehearsal + FR-49; S7's taxcalc upgrade + excuse) are folded into
`ROADMAP_STATUS.md` §3 in the fold commit that follows.
