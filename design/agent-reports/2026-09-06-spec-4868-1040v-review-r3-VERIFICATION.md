# VERIFICATION ledger — spec-4868-1040v r3 review (2026-09-06-spec-4868-1040v-review-r3.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `e1131fa6`.

| finding | claim | check | verdict |
|---|---|---|---|
| N3-I1 | Part I's labels never form a candidate column (`candidate_columns` buckets by x2 and needs ≥ 3 tokens); the band reading gives 12 wrong Schedule B bindings; the contains reading zeroes the 4868's joins | `label_reader.rs:125-131` buckets by `(t.x2 / 2.0).round()`; the corpus replays are the reviewer's (not re-run) — consistent with `label-boxes f4868--2025` (Part I → Part II labels) | **accepted** (the mechanism was mine; the diagnosis stands on the code) |
| N3-I2 | the walk calls `label_join` first and pushes `unwitnessed` on `Err` before `GRID_MAPS` is consulted | `every_mapped_line_lands_on_its_own_printed_label` (mine): `match label_join(&stem) { Err(why) => { r.unwitnessed.push(…); continue; } }` precedes the per-map reach | **TRUE** |
| N3-M1 | line 9 is a `[census]` `never` entry, not a `lineN` key; "+9" cannot be | the spec's own table | **TRUE** |
| N3-M2 | T3 still says "+2 months" | the T3 text (mine) | **TRUE** |
| N3-M3 | journey row 1 still says "not computable" for no stored inputs | the table (mine) | **TRUE** |
| N3-M4 | `PrintedReturn` carries no year; `fill_full_return(pr, year)` takes it | `crates/btctax-forms/src/packet.rs:100` | **TRUE** |
| N3-M5 | `admin.rs:621` is the slice branch; the packet path's gate is `admin.rs:975` | `sed -n 972,978p`: `promote_export_gate(state, events, Some(tax_year))?;` at :975 | **TRUE** |
| N3-M6 | "the recorded payment" wording survives in T2's kill and journey row 3 | the text (mine) | **TRUE** |
| N3-N1/N2 | two `packet.rs` files unqualified; FR-50(b)/FR-53 closures rode the fold commit | by reading; `git show ddd0df68 --stat` | **TRUE** |

**Verdict: 10/10 (9 TRUE, 1 accepted).**
