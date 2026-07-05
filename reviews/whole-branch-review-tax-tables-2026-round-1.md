# Whole-diff review (Phase E) — feat/tax-tables-2026 — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (f97adac)..HEAD` — 1 impl commit (`50cad0f`), 7 files, +923/−32.
Contract: `design/SPEC_tax_tables_2026.md` (R0-GREEN, 2 rounds). `btctax-adapters` + one test re-point + doc rewords.

## Tax-figure correctness (the crux)
Every 2026 figure was verified against the PRIMARY source (Rev. Proc. 2025-32 PDF) across BOTH R0 rounds; this
pass spot-confirmed the encoded values match: `dec!(201750)`/`dec!(256200)` (HoH 32%/35% — the trap), `dec!(384350)`
(MFS 37%), `dec!(19000)` (gift annual), `dec!(184500)` (SS wage base), `dec!(15_000_000)` (lifetime). ✅
- **[★ HoH trap] fault-injection CONFIRMED load-bearing.** Swapping HoH's 32% start `$201,750 → $201,775`
  (Single's value — the exact transcription trap) drove `ty2026_hoh_ordinary_brackets_match_rev_proc_2025_32`
  RED (panic tax_tables.rs:671, comment "TRAP: NOT Single's $201,775"). The most error-prone figure is pinned.

## Wiring + behavior change
- `ty2026()` mirrors `ty2025()`; wired via `by_year.insert(2026, ty2026())` in `load()` (R0-I1 — `table_for`
  unchanged). QSS→MFJ alias intact.
- **`NotComputable → Computed` flip owned** (R0-I2): `tax_report.rs carryforward_mismatch_advisory_rendered`
  re-pointed with a FULL year-shift to 2027 (CSV dates + both profiles + docstring — not a naive swap; the
  prior-year loss stays in the 2026 CSV so the mismatch is preserved, and 2027 stays unbundled). Stale docs
  (tax_tables.rs header/struct; optimize.rs:1309 rationale) reworded.
- Statutory constants untouched (I4); FOLLOWUPS updated (2026 DONE, 2027 deferred until IRS fall-2026 publish).

## Suite
`cargo test --workspace --locked` **1156 passed / 0 failed**; clippy -D + fmt clean. Additive data → PATCH.

**SHIP.**
