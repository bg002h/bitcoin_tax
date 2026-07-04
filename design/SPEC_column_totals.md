# SPEC — frozen column totals on the TUI output tabs (btctax-tui)

**Source baseline:** `main` @ `6a78edb` (branch `feat/tui-column-totals`). **Review status: R0 round 1 folded
(0C / 3I / 3M / 1N — all folded; the two headline checks PASSED: weighted-avg formula dimensionally correct,
Disposals scroll-cap verified safe). Review: `reviews/R0-spec-column-totals-round-1.md`. Awaiting R0 round 2.**
**Lineage:** parked item 2 (user-requested 2026-07-03; totals SEMANTICS settled with the user — Holdings
weighted-avg basis, Disposals summed basis; see below). Follows the shipped `?` help overlay (parked item 1).

## The feature
Render each tabular output tab's **column totals as a FROZEN footer row** — always visible even while the
data body scrolls — via ratatui's native **`Table::footer(Row)`** (ratatui 0.29, confirmed). Built once in
the shared `btctax-tui` tab renderers → the editor (`btctax-tui-edit`) inherits it for free (it delegates to
these renderers, `draw_edit.rs`).

## Totals semantics per tab [SETTLED with the user 2026-07-03]
- **Disposals** (`tabs/disposals.rs`) — **Σ BTC · Σ proceeds · Σ basis · Σ gain** (selected year). Basis is
  **SUMMED** (keeps the row additive: `Σ gain = Σ proceeds − Σ basis`; a weighted-average in one cell would
  read as a bug). It ALREADY accumulates `total_proceeds/basis/gain` and pushes a scrolling "TOTAL" row
  (disposals.rs:77-86) — **MOVE that row into `.footer()`** (frozen) and ADD the currently-missing **Σ BTC**.
- **Holdings** (`tabs/holdings.rs`) — **Σ BTC + weighted-average basis $/BTC**. **[R0-I3 — PINNED FORMULA]**
  compute as **`round_cents((Σ usd_basis × 1e8) / Σ sat)`** — MULTIPLY-FIRST (keeps the numerator exact,
  divides once) and round via **`btctax_core::conventions::round_cents`** (ROUND_HALF_EVEN, the house money
  rounding; btctax-tui already depends on core — NOT a core change). Dimensionally USD/BTC = avg cost of the
  stack ("brokerage avg cost"). WHY weighted-avg exists: you can't sum an unrealized gain, so a total-basis-$
  pairs with nothing. Accumulates `total_sat`+`total_basis` (holdings.rs:41-48) but doesn't surface them —
  add the footer. **Guard `Σ sat == 0` → `—`** — reachable ONLY with a NON-empty `lots` whose `remaining_sat`
  sum to 0 (empty `lots` short-circuits to "no holdings" at holdings.rs:34-39 BEFORE any total). **[R0-I1]
  this CHANGES the existing `holdings_renders_total_row` KAT** (tests.rs:227-230 currently asserts the SUMMED
  basis `4000.00`) → update it to the weighted average **`5333.33`** (= 4000 ÷ 0.75; keep the `0.75000000`
  Σ-BTC assertion). That fixture (lot A 0.5 BTC/$2500, lot B 0.25 BTC/$1500) is a perfect weighted-avg
  fixture — re-point it to double as `holdings_footer_weighted_average_basis`.
- **Income** (`tabs/income.rs`) — **Σ BTC (sat) + Σ income (FMV recognized)**. Both sums.
- **Forms** (`tabs/forms.rs`) — **DEFERRED to a followup [R0-N1].** The tab is MIXED (a Form 8949 `Table` +
  a free-text Schedule D `Paragraph`, forms.rs:57-61/112-175), AND the 8949 ST/LT proceeds/basis/gain are
  ALREADY surfaced as the Schedule D summary right below (forms.rs:129-138) — a footer would DUPLICATE an
  existing on-screen total (and a grand-total would fight the ST/LT split). **This cycle = Holdings +
  Disposals + Income only.**
- Only aggregate NUMERIC columns; blank the label/date/text columns; the first cell reads `TOTAL` (or `Σ`).

## Height gate [the user's specific requirement]
Show the frozen footer ONLY when the tab's content area is tall enough — a `const MIN_ROWS_FOR_TOTALS: u16 =
10` (the user's stated threshold = the minimum to fit the table's border + header + the frozen totals row +
**more than one data row**). Concretely: `if content_area.height >= MIN_ROWS_FOR_TOTALS { table.footer(totals) }
else { table /* no frozen totals — give the vertical space to data */ }`. On a standard ≥24-row terminal the
content pane is ~20 rows so the footer always shows; only a very short terminal drops it. **[R0-M2]** the
gate measures the **`area: Rect` passed to `render`** (border-INCLUSIVE — it's `chunks[1]`/`Min(0)`,
draw.rs:108-134); at 10, usable inner = 8 = header(1) + footer(1) + 6 data (≥2 data, comfortable). Keep the
fixed **10** (the user's number; border-inclusive gives margin — no need to compute from chrome).

## Rendering notes
- `Table::footer(Row)` renders a pinned bottom row inside the table's block; it does NOT scroll and is NOT
  selectable (so the scroll/selection helpers need no change once the Disposals "TOTAL" row leaves `rows`).
- For Disposals, removing the "TOTAL" row from `rows` needs **NO scroll/selection-cap change [R0-M1 —
  VERIFIED SAFE]**: the cap is computed by `active_row_count` from the DATA MODEL (viewer `lib.rs:286-317`,
  editor `main.rs:8445-8474`) — it already EXCLUDES TOTAL (`snap.state.lots.len()` / `Σ d.legs.len()` /
  income `.count()`), never `rows.len()`. The existing selection KATs
  (`total_row_not_selectable_g_selects_last_data_row`, `scroll_down_does_not_advance_past_last_data_row_to_total`,
  tests.rs:1075-1173) stay valid unchanged.
- Footer row style: dim/bold to distinguish from data (reuse an existing style; no new theme).

## Scope / SemVer
- **btctax-tui ONLY** (the shared tab renderers) — the editor inherits it. **No btctax-core / btctax-cli
  change; no persisted state; no serde.** PATCH-class (additive UI). No CLI/docs mirror.

## KATs (btctax-tui `tabs/tests.rs` — TestBackend, ≥`120×40`)
- `disposals_footer_shows_summed_totals` — the rendered frame's FOOTER row contains Σ BTC / Σ proceeds /
  Σ basis / Σ gain matching hand-summed legs (seed ≥2 legs of different values); and `Σ gain == Σ proceeds
  − Σ basis`.
- `disposals_total_row_no_longer_scrolls` — the "TOTAL" appears via the frozen footer, not as a body row
  (scrolling the body does not move it; and it's not selectable).
- **`holdings_footer_weighted_average_basis`** [I3] — 2 lots of DIFFERENT $/BTC (lot A 0.5 BTC/$2500, lot B
  0.25 BTC/$1500); footer basis == `round_cents((Σ usd_basis × 1e8) / Σ sat)` = **`5333.33`** (provably ≠
  lot A `5000` ≠ lot B `6000` ≠ simple-avg `5500` ≠ Σbasis `4000`), Σ BTC == `0.75000000`. The KAT computes
  its expected value with the SAME expression/rounding as the impl. **[I1] this is the UPDATED
  `holdings_renders_total_row`** (tests.rs:227-230, was `4000.00`) — re-pointed, not added.
- **`holdings_footer_zero_sat_shows_dash`** [I2] — seed a NON-empty `lots` whose `remaining_sat` sum to 0
  (e.g. one fully-consumed lot still in `state.lots`) → footer basis shows `—`, no panic. (NOT "empty
  holdings" — that short-circuits to "no holdings" before any total; the guard is only reachable this way.)
- `income_footer_sums_sat_and_fmv` (the existing `income_renders_total_row` survives the move-to-footer).
- **`totals_footer_hidden_on_short_terminal`** [M2] — this KAT OVERRIDES the default backend size: render
  into an area of height `< 10` (e.g. `TestBackend::new(120, 8)`) → NO "TOTAL" present (data-only); the
  paired `≥ 10` case → the footer present. (Pins the height gate; the `≥120×40` default is for the other KATs.)
- `editor_inherits_totals_footer` — a `btctax-tui-edit` Disposals render (via the shared
  `btctax_tui::tabs::disposals::render`, draw_edit.rs:114) shows the frozen footer (inherit-for-free).

## Plan (TDD)
- **Task 1** — Disposals: move the TOTAL row → `.footer()` + add Σ BTC + the height gate; the `MIN_ROWS_FOR_TOTALS`
  const (shared in `tabs/utils.rs`). KATs.
- **Task 2** — Holdings (weighted-avg basis footer, pinned `round_cents((Σbasis×1e8)/Σsat)`; **UPDATE the
  existing `holdings_renders_total_row` 4000.00→5333.33** [I1]) + Income (Σ footer). **Forms DEFERRED** [N1].
  KATs (weighted-avg, zero-sat-dash via a 0-remaining lot, income sums).
- **Task 3** — whole-diff review (Phase E) + full workspace suite + FOLLOWUPS (bulk-reconcile program + both
  parked TUI-polish items all shipped).

## Gotchas
- **Weighted-avg is Holdings-ONLY** — Disposals stays SUMMED basis (the gain identity); do not average the
  Disposals basis cell (reads as a bug).
- **Divide-by-zero** — Holdings weighted-avg guards `Σ sat == 0` → `—` (reachable ONLY with non-empty `lots`
  summing to 0 sat; empty `lots` short-circuit to "no holdings" BEFORE the total).
- **Height gate is the user's explicit requirement** — below the threshold, OMIT the frozen totals (data
  gets the space); do not shrink the data to <2 rows to force a footer.
- **Disposals scroll/selection** — moving "TOTAL" to `.footer()` needs NO cap change: the cap uses
  `active_row_count` from the data model (already excludes TOTAL — R0-verified); existing selection KATs stay green.
- **btctax-tui only** — no core/cli change; the editor inherits via the shared renderers.
