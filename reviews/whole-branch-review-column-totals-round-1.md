# Whole-diff review (Phase E) — feat/tui-column-totals — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review (reviewer ≠ author). Diff `980237d..HEAD` (Task 1 `e7dae45` Disposals + gate;
Task 2 `a82868b` Holdings + Income). Contract: `design/SPEC_column_totals.md` (R0-GREEN, 2 rounds). 6 files:
btctax-tui (tabs/{holdings,disposals,income}.rs, tabs/utils.rs, tabs/tests.rs) + btctax-tui-edit (main.rs —
one inherited-render test only, no editor production change). No btctax-core/cli change.

## Verification + fault-injection (all probes restored the tree byte-for-byte)

**1. [★ correctness] Holdings weighted-average basis — CONFIRMED.** `holdings.rs`: `if total_sat == 0 { "—" }
else { round_cents((total_basis × Decimal::from(100_000_000i64)) / Decimal::from(total_sat)) }` — the pinned
MULTIPLY-FIRST expression, rounded via `btctax_core::conventions::round_cents` (ROUND_HALF_EVEN). Dimensionally
USD/BTC.
- **[★ fault-inject]** replaced the `round_cents(...)` with `total_basis` (summed) →
  `holdings_footer_weighted_average_basis` RED ("footer basis must be the WEIGHTED AVERAGE 5333.33"). The
  weighted average is load-bearing. Restored.
- **Zero-sat guard** — `holdings_footer_zero_sat_shows_dash` (a non-empty lots summing to 0 sat → `—`) passes;
  the empty-lots short-circuit (holdings.rs:34-39) is upstream, so the guard is reached only via the KAT's path.

**2. [★ the height gate] — CONFIRMED.** Each tab: `let table = if area.height >= MIN_ROWS_FOR_TOTALS
{ table.footer(total_row) } else { table };` with `pub(crate) const MIN_ROWS_FOR_TOTALS: u16 = 10`
(utils.rs) — measured on the border-inclusive `area: Rect`.
- **[★ fault-inject]** defeated the gate (`if true`) on all three tabs → `totals_footer_hidden_on_short_terminal`
  RED ("footer must be hidden on a <10-row area (height 9)"). The boundary is pinned (9 → hidden, 10 → shown).
  Restored.

**3. Disposals — summed basis (gain identity) + freeze + Σ BTC — CORRECT.** The "TOTAL" row moved OUT of
`rows` into `Table::footer(total_row)` (frozen, non-scrolling, non-selectable); the previously-empty BTC cell
now carries `Σ BTC`; basis stays SUMMED (`Σ gain = Σ proceeds − Σ basis` — no weighted-average here, correct).
**No scroll/selection-cap change** — the cap keys off `active_row_count` (excludes TOTAL); the existing
selection KATs (`total_row_not_selectable_g_selects_last_data_row`,
`scroll_down_does_not_advance_past_last_data_row_to_total`) stay green (implementer-confirmed).
Pinned by `disposals_footer_shows_summed_totals` + `disposals_total_row_no_longer_scrolls`.

**4. Income — summed footer — CORRECT.** `Σ BTC` + `Σ FMV` (both sums); `income_renders_total_row` survived the
move-to-footer unchanged; `income_footer_sums_sat_and_fmv` added.

**5. Forms — correctly DEFERRED** (mixed Table + Schedule-D text; the 8949 totals are already surfaced as the
Schedule D summary — a footer would duplicate). Per spec §N1.

**6. Scope / SemVer / inherit** — btctax-tui-only; the editor inherits via the shared
`btctax_tui::tabs::{holdings,disposals,income}::render` (`editor_inherits_totals_footer`); no core/cli/serde
change; footer style is `Modifier::BOLD` (no new theme). PATCH-class.

## Full suite
`cargo test --workspace --locked` + `clippy -D warnings` + `fmt --check` — ship gate (see merge).

**SHIP.**
