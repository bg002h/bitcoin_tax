# SPEC — what-if TUI overlay (task #43, phase P3)

**Source baseline:** `main` @ `21f05ac` (branch `feat/whatif-tui`). **Review status: DRAFT — awaiting R0 (Opus —
UI slice reusing the VERIFIED core; low tax-risk).** Final slice of #43. Parent spec:
`design/SPEC_synthesize_whatif.md`. Reuses `btctax-core::whatif::{sell,harvest}` (shipped P0-P2) UNCHANGED — NO
new tax logic; tax correctness rides entirely on the already-verified core.

## Goal
An interactive what-if panel in the btctax-tui VIEWER: type a sell amount → live `SellReport` (marginal +
§1212 carryforward + §1(h) bracket + NIIT); or pick a harvest target → `HarvestReport`. Read-only; the viewer
never mutates the vault (existing invariant, app.rs:120 — `handle_key` mutates ONLY UI-nav fields).

## The one data gap + fix
The `Snapshot` (app.rs:104) already carries `events`, `state`, `cli_config`, `profiles: BTreeMap<i32,
TaxProfile>`, `tables: BundledTaxTables` — everything `whatif::{sell,harvest}` needs EXCEPT a `PriceProvider`.
**Fix:** add `prices` to the `Snapshot`, built once in `build_snapshot` (unlock.rs:170, which HAS the `Session`
→ `session.prices()`) as a concrete owned snapshot of the price data (the bundled daily-close + any local cache
the session already resolved) — NOT a re-open. The panel calls
`whatif::sell(&snap.events, &snap.prices, &snap.cli_config.to_projection(), year, profile, &snap.tables, …)`.

## The panel (viewer overlay — follows the export-modal / sort-overlay patterns)
- **Open:** a new keybinding **`w`** (What-if) from any output tab → `App.whatif: Option<WhatIfPanel>` (mirrors
  `export_modal: Option<…>`, app.rs:155). Esc closes. `handle_key` still mutates ONLY UI state + this panel
  field (extend the app.rs:120 doc-comment allow-list).
- **Mode toggle:** Tab/`s`/`h` switches Sell ⇄ Harvest within the panel.
- **Inputs (text-entry sub-fields, the unlock/export input pattern):**
  - Sell: amount (**accept BTC decimal**, e.g. `0.05`, parse→sat — resolving the `whatif-sell-btc-input`
    FOLLOWUP for the TUI at least); wallet (default = a picker over the pool's wallets); optional price
    (default = bundled FMV for the year, editable).
  - Harvest: target selector (`zero-ltcg | fifteen-ltcg | gain=$X | tax=$X`); wallet; optional price.
- **Profile:** default to the stored `snap.profiles[selected_year]` when present; else a clearly-labeled
  placeholder (single filer, `$0` ordinary) with an on-panel caveat "no stored tax profile — figures assume
  single / $0 other income; set one via `tax-profile set`". (Ad-hoc income entry is a nice-to-have; v1 may use
  the stored profile + the caveat.)
- **Output:** render the `SellReport`/`HarvestReport` live in the panel — lots consumed, ST/LT, the bracket +
  room, marginal tax + effective rate, the §1212 carryforward-delta line, NIIT, harvest status/binding
  constraint + disclosures. On `WhatIfError`, show the refusal reason (missing table/profile, pre-2025,
  ProceedsRequired, NoLots, YearNotComputable) verbatim. Recompute on input change (debounced-by-keystroke is
  fine — a fold is fast).
- **Year:** uses `App.selected_year` (the `[`/`]` year nav already present); a future/`at` beyond the dataset
  needs an explicit price (surfaced as ProceedsRequired).

## [★] Read-only / non-persistence invariant
The panel calls ONLY `whatif::{sell,harvest}` (clone-fold-discard) + reads `snap`. It NEVER touches the vault,
`conn()`, or any writer. KAT: the viewer's vault file is byte-identical after opening + driving the what-if
panel through sell + harvest. The App holds no `Session` (unlock.rs:92) — reinforced.

## KATs (btctax-tui)
- **★ `whatif_panel_never_persists`** — drive the panel (open, sell, harvest, close); the vault byte-identical.
- `whatif_panel_sell_renders_report` (a fixture snapshot → the panel shows the marginal/bracket/carryforward
  lines) + `…_harvest_renders_report` (status + binding constraint + disclosures).
- `whatif_panel_btc_input_parses_to_sat` (`0.05` → 5,000,000; reject ambiguous over-precision).
- `whatif_panel_error_renders_refusal` (no profile / pre-2025 / ProceedsRequired shown, not a crash).
- `whatif_panel_toggle_sell_harvest`; `handle_key_still_only_mutates_ui` (the app.rs:120 invariant KAT extended).
- Snapshot: `build_snapshot_populates_prices` (the new field is set).
- Regression: the existing viewer/editor suites stay green (the new `prices` field + `w` key break nothing;
  the editor uses `open_session`/`render`, not `App`).

## Scope / SemVer
btctax-tui only (+the `w` panel, +`Snapshot.prices`) + btctax-core/cli UNCHANGED. Man page/README note the
viewer's `w` what-if panel. Read-only; no persistence. Part of the 0.4.0 cycle (already breaking via P0).

## Plan (TDD)
- **P3** — add `Snapshot.prices` (build_snapshot) + the `WhatIfPanel` state + the `w` open + Sell/Harvest input
  + render (reuse the core reports) + the read-only/parse/render/refusal KATs; man page + README; whole-diff.

## Gotchas
- **[★ read-only]** the panel calls only the non-persisting core + reads the snapshot; extend the app.rs:120
  allow-list; the byte-identical KAT is the gate.
- **[prices]** the one missing snapshot input — retain it in `build_snapshot` (has the Session), do NOT re-open.
- **[BTC input]** accept a BTC decimal in the TUI (resolves the sat-vs-BTC FOLLOWUP for the panel).
- **[profile]** stored profile default + a loud caveat when absent (never silently assume single/$0 without saying).
- **[no new tax logic]** reuse `whatif::{sell,harvest}` verbatim; correctness is the verified core's.
