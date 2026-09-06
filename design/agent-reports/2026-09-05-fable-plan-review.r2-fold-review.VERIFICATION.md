# Controller's machine-check ledger — `…r2-fold-review.md` (0C/7I/5M), checked at `97911117`

| finding | claim | check | verdict |
|---|---|---|---|
| F1 | `TY2026_PORT_REPORT.md:300` still lists `schedule_1a.rs (56)` in the bad column; `:257` still says "per-YEAR artifact" | `sed -n 300p` / `257p` grep counts 1 / 1 | HOLDS |
| F2 | `:274-277` "registry to grow" paragraph unedited; `pub const FORMS` at `cite_check.rs:740`; `extract_stem` points at `crates/btctax-core/src/tax/fixtures/` | grep 1; line 740; 6 mentions of `fixtures/` in cite_check.rs | HOLDS |
| F3 | `LONG_RANGE_PLAN_filing.md:489` still commits TY2025-first; `:467` still "A — recommended" | grep 1 / 1 | HOLDS |
| F4 | `emitted_form_years()` keys on the IRS basename via `irs_basename` at `cite_check.rs:853` | line 853 contains `irs_basename` | HOLDS |
| F5 | `Form1040Map` has `ty2017/ty2024/ty2025` and the `line7a` doc "line 13 for 2017" | 27 `fn tyYYYY` in map.rs; doc string present | HOLDS |
| F7 | 31 of 37 bundled templates join `MANIFEST.json` by sha256; the six misses are all five TY2017 + `forms/2024/f8283.pdf` | python sha256 join: **31/37**, misses exactly those six | HOLDS — measured |
| F7 | `AUTHORITY_NOT_YET_ARCHIVED` at `cite_check.rs:883` | line 883; its doc: "form-year btctax can print while holding no archived, extracted primary source" — a DIFFERENT notion from the manifest join; 36 of 37 pairs excused | HOLDS+ — two "archived"s, both now in r2 §9 |
| F8 | the 1040 is pushed with `None` at `packet.rs:93-97` | grep 1 | HOLDS |
| F9 | FR-47 named a non-existent `legal/text/statute/` | already corrected in `58200a59` (§55 archived under `statute-irc/`) | HOLDS — closed before this fold |
| F10 | `ROADMAP_STATUS.md:185` "two emitted"; `:204-208` still describes the pre-r2 const shape | grep 1 / 1 | HOLDS |
| F11 | zero `1099-DA` hits in the port report | `grep -c` = 0 | HOLDS |
| F12 | M1 rows 5/20/21 absent from the FR-46..53 section | grep = 0 | HOLDS |
| §10 step 1 "NO" | follows from F4–F8 | — | ACCEPTED; step 1 rewritten |

All 12 hold. Folded in the commit after this one; F9 was closed earlier.
