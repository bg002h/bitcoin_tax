# VERIFICATION ledger — spec-4868-1040v r4 review (2026-09-06-spec-4868-1040v-review-r4.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `12fd965e`.

| finding | claim | check | verdict |
|---|---|---|---|
| R4-I1 | the walk has two `continue` arms above the join — the `m.stem` geometry join and `label_join` — and "counted first" reads as above both, which would divert a declared grid before its fixture is looked for | `every_mapped_line_lands_on_its_own_printed_label` (mine): `match &m.stem { Err(why) => { unwitnessed.push; continue } }` precedes `match label_join(&stem)` | **TRUE** |
| R4-I2 | line 8 (`c1_1`, `--out-of-country`) has no kill in T2 or T3 | the spec's T2/T3 text (mine) | **TRUE** |
| R4-M1 | the "fourth row gate … reds three times" sentence is false under r4's naming | the R1 text (mine) | **TRUE** |
| R4-M2 | binding Part I by name is a deviation from the transcription rule's naming clause, presented as compliance | `CLAUDE.md` "one field per numbered line, named for the line" + the scope amendment (transcription structs) | **TRUE** |
| R4-N1 | `return_inputs.rs` is a doubled basename | `crates/btctax-cli/src/return_inputs.rs` and `crates/btctax-core/src/tax/return_inputs.rs` | **TRUE** |
| R4-N2 | `candidate_columns` merges adjacent 2pt buckets before the `< 3` reject | `label_reader.rs:132-137` (reviewer's read; consistent with the bucket code at :125-131) | **accepted** |
| R4-N3 | the corpus spells the cells `taxpayer_ssn`, `address_street`… (`forms/2024/f1040.map.toml:117-127`) | the map | **TRUE** |
| R4-N4 | `label_reader.rs` IS edited (its test walk) | by design | **TRUE** |

**Verdict: 8/8 (7 TRUE, 1 accepted).**
