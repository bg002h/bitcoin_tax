# VERIFICATION ledger — label-join fold re-review r3 (2026-09-06-label-join-fold-review-r3.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `e8197309`.

| finding | claim | check | verdict |
|---|---|---|---|
| R2 | the "NO DRAFT with the draft on disk" plant uses `f6251`, whose pair computes, so the `(None, Ok)` arm fires and the `claims_no_draft` conjunct is never evaluated | by reading `check_work_list` and the plant (mine); `f6251--2025` and `f6251--2026-DRAFT` fixtures both exist | **TRUE** |
| R1 | `2017/f8283` has zero numbered keys and is not in `GRID_MAPS`; `2017/f8949` is listed but unreachable | `grep -cE '^(line[0-9]\|\[line[0-9])' forms/2017/f8283.map.toml` → 0; 2017 has no geometry | **TRUE** |
| R3 | the excuse check tests the geometry JSON while `compute` enters on the PDF | `form_delta.rs:51` `fn pdf_for(stem) -> Option<PathBuf>`; the `fixture` closure (mine) reads `design/forms/geometry/{stem}.json` | **TRUE** |
| R4 | the 2024 floor's `why` says "there is no PDF to extract geometry from" while `crates/btctax-forms/forms/2024/f8283.pdf` (181,414 bytes) exists and only the archived authority `design/forms/2024/f8283--2024.pdf` is missing | `label_reader.rs:1474`; `ls`: the bundled PDF exists, `design/forms/2024/` holds only `i8283--2024.pdf` | **TRUE** |
| R5 | the true-excuse control depends on `f1040--2026-DRAFT` staying unarchived | by reading the plant (mine) | **TRUE** |
| R6 | the `[[` guard is inert (`"[part1_rows"` never strips `line`); `[[line…]]` occurs in no map | by reading; `grep -c '^\[\[line' forms/*/*.map.toml` → 0 (the reviewer's measurement, consistent with the section grep earlier) | **TRUE** |
| R7 | the temp dir leaks on a failed assertion | by reading | **TRUE** |
| R8 | `FOLLOWUPS.md` ends without a newline | `tail -c 2 \| xxd` → `**` | **TRUE** |
| R9 | inside a section every quoted FQN on any line is attributed to the line | by reading | **TRUE** |
| gate | 261/215, 0 wrong, 13/5/18, 19/19; +12/+14 = the 26 section FQNs | the fold commit's own run | **consistent** |

**Verdict: 10/10 TRUE, 0 refuted. Gate verdict accepted: YES.**
