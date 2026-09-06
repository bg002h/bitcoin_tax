# Controller's ledger — `2026-09-05-r2-step1-review.md` (0C/2I/6M), checked at `2c3044e3`

| finding | claim | check | verdict |
|---|---|---|---|
| rows | all 37 rows correct on nine fields | the reviewer's method is stated (sha256 join, extract/template text for sequences, `Rev.` grep for periodic, manifest join for `authority`); I re-ran `tests/map_rows.rs` (all four kills green) and the xtask two-way test | ACCEPTED |
| P1 | kill 4 is gated on `authority`, so `2024/f8283` (extract prints 155) was never checked | the `if row.authority.is_none()` guard was in `check_rows` as written; after re-keying on extract existence, the new plant `a_planted_wrong_sequence_number_is_reported_even_on_a_manifest_excused_row` (999 on 2024/f8283) reports `SequenceMismatch{printed: 155}`; the five TY2017 rows surface as `SequenceUnverifiable`, pinned | HOLDS — folded |
| P2 | `fill_full_return` has no sort; f8283 is pushed after f8275 | `grep -n 'sort' packet.rs` had no match; the push order ends `f8275, f8283`; TY2024 order already ascending (no golden moved after the sort — 1088/1088) | HOLDS — folded (stable sort by `sequence_key`, `1A` after `01`, `12A` after `12`; the position test pins f8283 between 6251 and 8995 for TY2025) |
| P3 | `manifest_authority_hashes` reimplements `is_authority()` | true by construction (crate dependency direction) | HOLDS — an agreement test over every entry now lives in xtask |
| P4 | the two-way plant re-derives the check | it did (inline `difference`) | HOLDS — `two_way_diff` factored; plant runs it in both directions |
| P5 | `AnnualTag`'s typo guarantee untested | no test named it | HOLDS — two asserts added |
| P6 | `instructions`/`instr_pages` duplicate `FORMS` with nothing tying them | true | HOLDS — every archived row's instructions stem must be a manifest entry; `FORMS[0]` must equal its row |
| P7 | `instructions = ""` example stale (f8275 has `i8275`) | `forms/2024/f8275.map.toml` says `instructions = "i8275"` | HOLDS — doc corrected |
| P8 | commit message said 13/13; nextest ran 10 | the 13 was the filtered run's count including neighbours; the file counts are 7 + 3 | HOLDS — history; not rewritten |

All eight hold. Folded in the commit after this one.
