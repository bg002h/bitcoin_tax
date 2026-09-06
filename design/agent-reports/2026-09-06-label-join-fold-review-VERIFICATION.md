# VERIFICATION ledger — label-join fold review (2026-09-06-label-join-fold-review.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `3b0a35a2`. Every measurable claim re-run
with `sed -n`, `grep -n`, and the xtask binaries at HEAD; nothing hand-counted.

| finding | claim | check | verdict |
|---|---|---|---|
| L1 | `label_reader.rs:1067` reads `*ly + 4.0 <= *bottom`; `Word::y2` exists and is unused | `sed -n 1055,1075p`: the filter is `*ly >= top - 2.0 && *ly + 4.0 <= *bottom && …`; `form_geometry.rs:37` `pub y2: f64`; `grep -c '\.y2' label_reader.rs` → **0** | **TRUE** (the four constructed false-PASSes are reproduced by the plant the fold commits, not re-derived here) |
| L2 | `per_map_zero` guard is `!bindings.is_empty() && map_joined == 0`; no kill exists | `:1477` verbatim; `grep -n per_map_zero` → declared `:1420`, pushed `:1479`, asserted `:1513` — no plant | **TRUE** |
| L3 | work list says 27/3/5/1; `form-delta` at HEAD says 23/2/2/0 | doc rows `:24-26, :31` = 27, 3, 5, 1; `xtask form-delta` at HEAD: f1040s3 "2 of 33 COMPARED … moved", f1040sa "2 of 14", f6251 "60 of 62 COMPARED; none of them changed" (f1040s2's count taken from the report; regenerated in the fold) | **TRUE** |
| L4 | inline-table bindings (`line1a/3/8a/10`) are dropped by `strip_prefix('"')` | `forms/2024/schedule_d.map.toml:20-24` binds `line1a`, `line8a`, `line3`, `line10` as `{ proceeds_d = "…", … }`; `forms/2025/schedule_d.map.toml:12,14` binds `line3`, `line10`; `line_bindings` `:1120-1123` takes only a leading `"` | **TRUE** |
| L5 | TY2025 comment says "Every TY2025 map contributes" | `:1338` verbatim | **TRUE** |
| L6 | `field_census.rs:620` is a three-stem hand-list ignoring `year` | verbatim: `stem == "f1040s1a" \|\| stem == "f8275" \|\| stem == "f8283"` | **TRUE** |
| L7 | `FiledPacket` still derives `Default` with `pub` fields | `packet.rs:52-57` `#[derive(Debug, Clone, Default)]`, both fields `pub`; no `FiledPacket::default()` caller in the workspace | **TRUE** |
| L8 | the 12pt gap and the "~9.6pt" comment | `:1057` "~9.6pt tall", `:1069` `left - *lx2 <= 12.0` | **TRUE** (the 11.30/12.30 separation is the reviewer's measurement over 49 fixtures; recorded in the comment as theirs) |
| kills | "planted 2a/2b swap" at `ROADMAP_STATUS.md:179` has no committed test | `:179` verbatim; `grep -c swap label_reader.rs` → **0** | **TRUE** |
| R1–R12 closure | as tabled | R6 pins 5/17/15 (`tests/year_record.rs:40`), R11's kill sets `r.table = false`, R12 `by_year.insert(2017, …)` — all seen in the fold commit | **consistent** |

**Verdict: 9/9 claims TRUE, 0 refuted.** The reviewer's Python replica numbers (235/193, 79
correct / 23 wrong, 1,425 firings) are not re-run; the fold's measurement is the Rust test's own
printed counts after the change, quoted in the fold commit.
