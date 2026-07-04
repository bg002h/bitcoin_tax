# SPEC — frozen column totals on the TUI output tabs (btctax-tui)

**Source baseline:** `main` @ `6a78edb` (branch `feat/tui-column-totals`). **Review status: DRAFT — awaiting
R0 (2-round loop to 0C/0I).**
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
- **Holdings** (`tabs/holdings.rs`) — **Σ BTC + weighted-average basis $/BTC** (`= Σ usd_basis ÷ Σ sat ×
  1e8`, i.e. avg cost of the stack — the "brokerage avg cost"). This tab is WHY weighted-avg basis exists:
  you can't sum an unrealized gain, so a total-basis-$ figure pairs with nothing; the per-coin avg is the
  meaningful summary. It already accumulates `total_sat` + `total_basis` (holdings.rs:41-48) but does NOT
  surface them — add the footer. Guard `Σ sat == 0` → show `—` (no divide-by-zero).
- **Income** (`tabs/income.rs`) — **Σ BTC (sat) + Σ income (FMV recognized)**. Both sums.
- **Forms** (`tabs/forms.rs`) — per-form Σ where a column is meaningfully summable (e.g. Form 8949
  proceeds/basis/gain). [R0: confirm the Forms tab is a Table (not free text) before committing a footer;
  if it's mixed/text, DEFER Forms to a followup and do Holdings/Disposals/Income this cycle.]
- Only aggregate NUMERIC columns; blank the label/date/text columns; the first cell reads `TOTAL` (or `Σ`).

## Height gate [the user's specific requirement]
Show the frozen footer ONLY when the tab's content area is tall enough — a `const MIN_ROWS_FOR_TOTALS: u16 =
10` (the user's stated threshold = the minimum to fit the table's border + header + the frozen totals row +
**more than one data row**). Concretely: `if content_area.height >= MIN_ROWS_FOR_TOTALS { table.footer(totals) }
else { table /* no frozen totals — give the vertical space to data */ }`. On a standard ≥24-row terminal the
content pane is ~20 rows so the footer always shows; only a very short terminal drops it. [R0: pin the exact
arithmetic — border(2) + header(1) + footer(1) + ≥2 data ⟹ ≥6; the user chose the more generous 10, so the
totals never crowd. Use 10 unless R0 finds a reason to compute it from the real chrome.]

## Rendering notes
- `Table::footer(Row)` renders a pinned bottom row inside the table's block; it does NOT scroll and is NOT
  selectable (so the scroll/selection helpers need no change once the Disposals "TOTAL" row leaves `rows`).
- For Disposals, removing the "TOTAL" row from `rows` means the scroll-cap comment (disposals.rs:77
  "selection capped at data_rows-1") stays correct — verify the cap logic keys off `rows.len()` and adjust
  if it assumed the extra TOTAL row.
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
- **`holdings_footer_weighted_average_basis`** — with 2 lots of DIFFERENT $/BTC, the footer basis ==
  `round(Σ usd_basis ÷ Σ sat × 1e8)` (a true weighted average, provably ≠ either lot's rate and ≠ Σ basis);
  `holdings_footer_zero_sat_shows_dash` (empty holdings → no divide-by-zero).
- `income_footer_sums_sat_and_fmv`.
- **`totals_footer_hidden_on_short_terminal`** — render at height `< MIN_ROWS_FOR_TOTALS` → NO totals row
  present (data-only); render at `≥ MIN` → the footer present. (Pins the height gate.)
- `editor_inherits_totals_footer` — a `btctax-tui-edit` Disposals render (via the shared renderer) shows the
  frozen footer (the inherit-for-free guarantee). [or assert the shared fn is the sole renderer]

## Plan (TDD)
- **Task 1** — Disposals: move the TOTAL row → `.footer()` + add Σ BTC + the height gate; the `MIN_ROWS_FOR_TOTALS`
  const (shared in `tabs/utils.rs`). KATs.
- **Task 2** — Holdings (weighted-avg basis footer) + Income (Σ footer) + Forms (if a Table; else defer). KATs.
- **Task 3** — whole-diff review (Phase E) + full workspace suite + FOLLOWUPS (bulk-reconcile program + both
  parked TUI-polish items all shipped).

## Gotchas
- **Weighted-avg is Holdings-ONLY** — Disposals stays SUMMED basis (the gain identity); do not average the
  Disposals basis cell (reads as a bug).
- **Divide-by-zero** — Holdings weighted-avg guards `Σ sat == 0` (empty year → `—`).
- **Height gate is the user's explicit requirement** — below the threshold, OMIT the frozen totals (data
  gets the space); do not shrink the data to <2 rows to force a footer.
- **Disposals scroll/selection** — once the "TOTAL" body row moves to `.footer()`, re-verify the selection
  cap (it must key off the true data-row count, not the old `rows.len()` that included TOTAL).
- **btctax-tui only** — no core/cli change; the editor inherits via the shared renderers.
