# Controller's ledger — `2026-09-05-r2-step4-review.md` (0C/4I/6M+2N), checked at `5b073355`

| finding | claim | check | verdict |
|---|---|---|---|
| R1 | OTS 2025 is installed and `ots_direct.py` selects it; the TY2025 record said "archived? no" | `ls ~/OpenTaxSolver2025_23.06_linux64/bin` (present, beside the 2024 tree); `ots_direct.py:70-79` (`OTS_DIR` + `OTS_YEAR`) | HOLDS — record corrected: an EXTERNAL install, named, not archived in-repo |
| R2 | the binding parser dropped every `line = "…" # comment`; Schedule A/2025 contributed 0 of 19 joins | after taking the first quoted token: 2024 152 → **235** joins, 2025 116 → **193**; `2025/f1040sa` 0 → 19, `f1040sc` 2 → 7; per-map counts now printed and a zero-join map reds | HOLDS — and the widened join then exposed the READER's own defect: one label column + a purely vertical rule labelled the 1040's `2b` and `3b` boxes "2a"/"3a" (the map is right by geometry: "2b" prints at x 488.5–497.9, 6pt left of the `f1_43` box). Fixed with an x-aware in-row rule (raw numeric-label words, ≤12pt gap); a planted 2a/2b swap now reds both ways; floors raised 99→235, 82→193 |
| R2 | "the eight are wired on seven maps' evidence" | with the parser fixed, all eight TY2025 maps join with every numbered binding (sa 19/19, sc 7/7) | HOLDS as of the review; CLOSED by the fix — the wiring now rests on eight maps' evidence |
| R3 | no non-test reader of `YearReadiness` in the range | true at 162739e8; `c76adf6b` (FR-48) added the report line, the refusal sentence, the import note, the export stamp | HOLDS — closed before this fold |
| R4 | `unlock.rs:229 .unwrap_or(2025)` survived | true at 162739e8; `c76adf6b` derives it | HOLDS — closed before this fold |
| R5 | the TY2025 `f8275` reason cited a function no longer on the path | true | HOLDS — reworded to `periodic_template` |
| R6 | `glob_problems` and the record derive from the same glob | true by construction | HOLDS — per-year expected-count pin added (5/17/15), not from the glob |
| R7 | the census's "complete" became a declaration | true | HOLDS — a `filable` year's measured absences must obey a structural rule (TY2025+ schedule or periodic form) |
| R8 | `FiledPacket` literal/`Default` constructible from any crate; the compile-error claim overstated | true | HOLDS — `#[non_exhaustive]`; sentences say "held by test" |
| R9 | TY2017 `taxcalc = "none"` states an impossibility | taxcalc 6.8.2 `JSON_START_YEAR` 2013 covers 2017 | HOLDS — "none wired — … no slice harness path" |
| R10 | TY2025 `tables` cites Pub. L. 119-21 for figures not bundled | `tax_tables.rs` says OBBBA did not change the TY2025 thresholds | HOLDS — moved into the parenthetical |
| R11 | `Slice` arm never checks `table` | code read | HOLDS — checked, with a test |
| R12 | `BundledTaxTables` doc omits TY2017 | `tax_tables.rs:60` | HOLDS — corrected |
| values | all 27 other record values, the `TRANSITION_DATE` amendment, the `return_due` bridge, `[census]` and map ⊆ PDF for the eight | the reviewer's machine checks, stated | ACCEPTED |

All hold. Folded in the commits after this one. 1206 tests (forms + cli + xtask + adapters) green.
