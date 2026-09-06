# VERIFICATION ledger — label-join fold re-review r2 (2026-09-06-label-join-fold-review-r2.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `2246f78c`.

| finding | claim | check | verdict |
|---|---|---|---|
| N1 | `[lineN]` table sections are invisible to `line_bindings` and `numbered_line_keys` (both key on `split_once('=')`) | `grep -cE '^\s*\[line[0-9]' forms/*/*.map.toml` → 2017/schedule_d 1, 2024/f8283 3, 2024/schedule_d 3, 2024/f1040sb 3, 2025/f1040sb 3, 2025/f1040s1a 2, 2025/schedule_d 1; a header has no `=` | **TRUE** |
| N2 | `GRID_MAPS` says f8283 has 0 numbered keys; `forms/2024/f8283.map.toml` binds `[line5a]/[line5b]/[line5c]` | `sed -n 73,82p`: the three sections, six FQNs | **TRUE** — the recorded reason was false for the 2024 map (the 2025 map has none) |
| N3 | `ROADMAP_STATUS.md` still says floors 235/193 | grep: the label-reader bullet in §2 | **TRUE** |
| N4 | the 2024 ratchet comment documents only 99 → 235 above `min_joins: 249` | `label_reader.rs:1394-1396` | **TRUE** |
| N5 | `(false, 19, 1, 1)` → `None` | the `_ => None` arm, by construction | **TRUE** |
| N6 | the work-list plant never runs the loop; an excuse is accepted on "no pair" alone | the test text (mine) | **TRUE** |
| N7–N9 | quote-split assumptions; `first_bundled_year` fails open if an earlier year loses a form; `pub` fields | by reading | **TRUE**, recorded |
| gate | 249/201/0 wrong; 13/5/18; L1 moved nothing (249−14 = 235, 201−8 = 193) | the fold commit's own captured run | **consistent** |

**Verdict: 9/9 TRUE, 0 refuted.**
