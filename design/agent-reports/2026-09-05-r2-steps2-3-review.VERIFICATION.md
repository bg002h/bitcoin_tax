# Controller's ledger — `2026-09-05-r2-steps2-3-review.md` (0C/2I/4M), checked at `1841788e`

The reviewer could not run cargo (the tree carried step 4 in flight); every claim below was re-checked
here against the live tree, with the commands named.

| finding | claim | check | verdict |
|---|---|---|---|
| Q1 | `alias_is_licensed_by` was tautological as called (`periodic_template` can only hand it the 2024 bytes) and would refuse a year bundling its own newer 8275 | read of `periodic_template` + `Form8275Map::for_year` as of bc6dce35: the compare target was the constant `template(F8275, 2024)`, the input the same bytes | HOLDS — folded: the pairing inside `periodic_template` is the licence; a `debug_assert_eq!` states it; the fn and its sp4 plants stay as the documented rule |
| Q2 | no test calls `fill_full_return` at any year but 2024; deleting the sort reds nothing | `grep -rn 'fill_full_return(' crates/*/tests crates/*/src \| grep -c 2025` = **0** | HOLDS — folded: `FiledPacket::stapled` is the ONLY constructor (the fields cannot be named by `fill_full_return`); removing the sort no longer compiles |
| Q2 | `fill_form_8959` calls `for_year(year)?` before `must_file()` | `lib.rs` wrapper read; only 8959 (and Schedule D, already gated in `packet.rs:188`) carry a must-file predicate | HOLDS — folded: `must_file()` first |
| Q3 | eight of the ten `Unwired` maps would parse into their 2024 structs; only `f6251/2025` would not | the reviewer's `tomllib` key-tree measurement; my in-process probe printed nothing (nextest filter) and was deleted — NOT independently re-run | ACCEPTED on the reviewer's stated method; docs reworded to "not yet verified"; step 5 re-measures by actually parsing |
| Q4 | five stale prose sites (`lib.rs:80-89`, `cross_product.rs:14`, …) | read | HOLDS — corrected |
| Q5 | `BUNDLED_BUT_NOT_SUPPORTED` can no longer fire (SUPPORTED_YEARS derived from the same glob) | true by construction after step 3 | HOLDS — doc says what makes a new year loud now |
| Q6 | the "map but no pdf" build.rs arm never observed red; the cross-product reader lost its liveness check | planted `forms/2024/f9998.map.toml` → rc=101 "has a .map.toml but no .pdf"; liveness assertion added (`BUNDLED.len() >= 37`) | HOLDS — folded |
| disk | `target/` 498 GB | `du -sh target` = 498G; `/scratch` has 5.1 TB free | HOLDS — not a hazard here; noted in CONTINUITY |

Folded in the commit after this one. 1099 tests (forms + cli + xtask) green.
