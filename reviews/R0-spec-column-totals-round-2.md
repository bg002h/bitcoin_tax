# R0 — SPEC_column_totals.md — Round 2 (verification of folds)

**Reviewer:** independent architect (R0 round 2). **Artifact:** `design/SPEC_column_totals.md` (folded).
**Baseline:** branch `feat/tui-column-totals` @ `841df4b` (main == `6a78edb`). **Bar:** 0C / 0I.
**Round 1:** 0C / 3I / 3M / 1N — all findings folded by the author. This round confirms each against CURRENT source.

## Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — **R0-GREEN**

Both round-1 headline checks remain valid (weighted-avg dimensionally correct; Disposals scroll-cap safe). All
three Important folds (I1, I2, I3) are correctly reflected in the current spec AND match current source. M1/M2/N1
folded. No regression. No new findings.

## Per-finding verification

### [I1] RESOLVED — existing KAT re-pointed, not duplicated
- **(a) confirmed** — `tests.rs:227-230` currently asserts the SUMMED basis: `buffer_has(&buf, "4000.00")`
  with message `"TOTAL USD basis must be the sum: 4000.00"`. The fixture is lot A (50M sat / $2500) + lot B
  (25M sat / $1500) at tests.rs:177-206. Post-change the cell renders the weighted average, so `4000.00` would
  vanish → the KAT would go red unless updated. The spec (lines 27-31, 74-75) correctly updates it to `5333.33`
  keeping the `0.75000000` Σ-BTC assertion, re-pointed to double as `holdings_footer_weighted_average_basis`
  ("re-pointed, not added").
- **(b) confirmed** — `4000 ÷ 0.75 = 5333.33`; pinned form `(4000 × 1e8) / 75_000_000 = 5333.3333… → 5333.33`.
  Provably distinct: lot A rate `5000` (2500/0.5), lot B rate `6000` (1500/0.25), simple-avg `5500`,
  Σbasis `4000` — all four ≠ `5333.33`. Strong discriminator; a broken impl can't accidentally pass.
- **(c) confirmed** — Plan Task 2 (spec lines 89-91) explicitly lists **"UPDATE the existing
  `holdings_renders_total_row` 4000.00→5333.33 [I1]"**. Coherence check on the re-point: the KAT's other
  assertions (`"TOTAL"` present — still true since the footer's first cell reads `TOTAL`; `0.50000000` /
  `0.25000000` data rows; `0.75000000` Σ BTC) survive the move-to-footer unchanged.

### [I2] RESOLVED — zero-sat KAT hits the reachable path
- **Short-circuit confirmed** at `holdings.rs:34-39`: `if lots.is_empty() { … "no holdings" … return; }` —
  fires BEFORE `total_sat`/`total_basis` are accumulated (holdings.rs:41-48). Empty `lots` therefore never
  reaches the total, so a "empty holdings" fixture could never exercise the `Σ sat == 0` guard.
- **Guard reachable ONLY via non-empty lots** — `remaining_sat` is a non-negative quantity (conventions.rs:5-6),
  so `Σ sat == 0` on a non-empty `lots` requires every lot's `remaining_sat == 0` (e.g. a fully-consumed lot
  still in `state.lots`). The spec's `holdings_footer_zero_sat_shows_dash` KAT (lines 76-78) seeds exactly this
  and asserts `—` with no panic. Path is correct and uniquely reachable.

### [I3] RESOLVED — formula pinned; no new dep / core change
- **`round_cents` confirmed** at `conventions.rs:21-24`: `pub fn round_cents(v: Usd) -> Usd {
  v.round_dp_with_strategy(2, MONEY_ROUNDING) }`, where `MONEY_ROUNDING = RoundingStrategy::MidpointNearestEven`
  (conventions.rs:13, doc'd "ROUND_HALF_EVEN (§6.1)"). Matches the spec's pinned rounding exactly.
- **Multiply-first pinned** — spec lines 20-24 pin `round_cents((Σ usd_basis × 1e8) / Σ sat)` (numerator exact,
  single divide, ties-to-even) and require the KAT to compute its expected value with the identical
  expression/rounding (lines 71-74). Correct.
- **No new dep / no core change confirmed** — `crates/btctax-tui/Cargo.toml:19` already declares
  `btctax-core = { path = "../btctax-core" }`, and btctax-tui already imports `btctax_core::…` across many
  modules (tags.rs, app.rs, tax.rs, forms.rs, export.rs, lib.rs). Calling the existing `round_cents` adds
  nothing to core. PATCH-class, btctax-tui-only holds.

### [M1] RESOLVED — cap wording now cites `active_row_count`
Spec lines 53-58 now attribute the scroll/selection cap to `active_row_count` (not `rows.len()`). Confirmed at
`lib.rs:286-317`: Holdings → `snap.state.lots.len()`, Disposals → `Σ d.legs.len()` (comment "no +1 for TOTAL"),
Income → `.count()` (comment "no +1 for TOTAL"). The cap keys off the data model and already excludes TOTAL, so
moving Disposals'/Holdings' TOTAL into `.footer()` needs no cap change; the selection KATs (tests.rs:1075-1173)
stay valid. Matches the fold.

### [M2] RESOLVED — gate measures the border-inclusive `area: Rect`
Spec lines 40-48 now state the gate compares `MIN_ROWS_FOR_TOTALS = 10` against the render `area` (border-
inclusive). Confirmed: the content pane is `chunks[1]` = `Constraint::Min(0)` (draw.rs:110-115) passed as
`content_area` (draw.rs:134) into each tab's `render(area: Rect, …)` (e.g. holdings.rs:27). At height 10, inner
usable = 8 = header(1) + footer(1) + 6 data (≥2 data, comfortable) — the spec's arithmetic (line 47) is right,
fixed `10` retained. The `totals_footer_hidden_on_short_terminal` KAT (lines 80-82) overrides the `≥120×40`
default to a `<10`-row backend, with a paired `≥10` case. Correctly folded.

### [N1] RESOLVED — Forms DEFERRED
Spec lines 33-37 defer Forms. Confirmed `forms.rs` is mixed (module doc line 1: "Form 8949 rows (selectable
table), Schedule D totals"; `Table` at forms.rs:112 + `Paragraph` at forms.rs:168), and the 8949 ST/LT
proceeds/basis/gain are ALREADY on-screen as the Schedule D summary (forms.rs:131-138, "Schedule D Part I (ST)"
/ "Part II (LT)"). A footer would duplicate an existing total and a grand-total would fight the ST/LT split.
Deferral is the right call; this cycle = Holdings + Disposals + Income.

## Spot-checks — no regression
- **Disposals summed basis (gain identity) unchanged** — disposals.rs:51-53 accumulates `total_proceeds`,
  `total_basis` (SUMMED), `total_gain`; the TOTAL row (disposals.rs:77-86) has an empty BTC cell (safe to add
  Σ BTC). `Σ gain = Σ proceeds − Σ basis` preserved. Spec keeps Disposals basis SUMMED (lines 17-19, 96-97).
- **`Table::footer(Row)` still the mechanism** — spec lines 11-13, 50-52 unchanged; round-1 already confirmed
  it exists in ratatui 0.29, non-scroll, non-selectable.
- **Scope btctax-tui-only** — spec lines 61-63; no core/cli/serde/persisted-state change (round_cents already
  exists; only called).
- **Editor inherits via shared renderers** — draw_edit.rs delegates to
  `btctax_tui::tabs::holdings::render` / `disposals::render` / `income::render` (lines 114/121/128); the frozen
  footer is inherited for free.

## Conclusion
All 3 Important + all Minor/Nit findings from round 1 are folded and verified against current source. No new
findings at any severity. **R0-GREEN — proceed to plan/implementation.**
