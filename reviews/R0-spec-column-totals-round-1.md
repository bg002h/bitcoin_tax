# R0 — SPEC_column_totals.md — Round 1

**Reviewer:** independent architect (R0 round 1). **Artifact:** `design/SPEC_column_totals.md` (DRAFT).
**Baseline:** `feat/tui-column-totals`; spec baseline `main @ 6a78edb`. **Bar:** 0C / 0I.

## Verdict: 0 Critical / 3 Important / 3 Minor / 1 Nit — NOT GREEN

**Headline checks:**
- **#1 weighted-avg formula (Holdings):** `Σ usd_basis ÷ Σ sat × 1e8` is DIMENSIONALLY CORRECT
  (`(USD/sat)×(1e8 sat/BTC)=USD/BTC`). Accumulators + lot fields match source. Gaps: breaks an existing KAT
  [I1]; evaluation-order + rounding unpinned [I3].
- **#2 Disposals scroll-cap after moving TOTAL:** VERIFIED SAFE — the cap keys off `active_row_count` from
  the data model (viewer `lib.rs:286-317`, editor `main.rs:8445-8474`), NOT `rows.len()`. No cap change.

### [I1] IMPORTANT — the weighted-avg change silently breaks `holdings_renders_total_row`
`holdings.rs:68` currently renders SUMMED basis; the KAT `holdings_renders_total_row` (tests.rs:176-231)
asserts `"4000.00"` (tests.rs:227-230). After the change the cell shows `4000÷0.75 = 5333.33`; `4000.00`
appears nowhere → suite red. **Fix:** update the KAT to `"5333.33"` (keep `"0.75000000"`); the fixture (lot A
0.5 BTC/$2500, lot B 0.25 BTC/$1500) is a perfect weighted-avg fixture — re-point it to double as
`holdings_footer_weighted_average_basis`.

### [I2] IMPORTANT — the zero-sat guard KAT tests the wrong path
Empty holdings short-circuits to `"no holdings"` at holdings.rs:34-39 BEFORE any total. The `Σ sat == 0`
guard is reachable ONLY with a NON-empty `lots` whose `remaining_sat` sum to 0. **Fix:** seed ≥1 lot with
`remaining_sat = 0`; fix the "empty holdings/year" wording.

### [I3] IMPORTANT — pin the weighted-avg evaluation order + rounding
Divide-first truncates the intermediate; rounding mode unspecified. **Fix:** pin
`round_cents((Σbasis × 1e8) / Σsat)` (multiply-first; ROUND_HALF_EVEN via `btctax_core::conventions::round_cents`,
conventions.rs:22-23 — no core change, btctax-tui already depends on core). KAT computes expected identically.

### [M1] MINOR — spec #2 cites wrong cap mechanism
It's `active_row_count`, not `rows.len()`. Reword to the verified fact (no cap change; existing selection
KATs tests.rs:1075-1173 stay valid).

### [M2] MINOR — height gate measures the `area: Rect` (border-inclusive); reconcile KAT size
Gate compares `MIN_ROWS_FOR_TOTALS` against the render `area` (`chunks[1]`/`Min(0)`, draw.rs:108-134). Keep
fixed `10`. The `totals_footer_hidden_on_short_terminal` KAT must override the `≥120×40` default to a `<10`-row area.

### [M3] MINOR — `MIN_ROWS_FOR_TOTALS` in `tabs/utils.rs` is crate-internal (no `pub` needed). No change required.

### [N1] NIT — Forms: DEFER (confirmed)
forms.rs:57-61 is mixed (8949 `Table` + Schedule D `Paragraph`); the ST/LT totals are ALREADY surfaced as the
Schedule D summary (forms.rs:129-138) — a footer would duplicate. Do Holdings/Disposals/Income this cycle.

## Confirmations (no finding)
- ratatui 0.29 (`Cargo.lock:2107`); `Table::footer` (table.rs:416) reserves bottom height, non-scroll,
  non-selectable, compatible with `render_stateful_widget` (holdings.rs:96, disposals.rs:122, income.rs:93).
- Disposals accumulates totals (disposals.rs:30-53), TOTAL row at 77-86 with an EMPTY BTC cell (safe to add Σ BTC;
  `disposals_renders_total_row` tests.rs:384-424 doesn't assert the BTC cell).
- Income unchanged (sums); `income_renders_total_row` survives.
- Scope btctax-tui-only; editor inherits via `btctax_tui::tabs::*::render` (draw_edit.rs:114-146). No core/cli/serde change.

**Fold I1, I2, I3 (all block the bar) + M/N wording; re-review round 2.**
