# SPEC — what-if TUI overlay (task #43, phase P3)

**Source baseline:** `main` @ `21f05ac` (branch `feat/whatif-tui`). **Review status: R0 round 1 folded (0C/3I/6M/2N — Opus;
read-only invariant confirmed airtight; 3 Importants + minors merged IN-PLACE). Awaiting R0 round 2.** Review:
`reviews/R0-spec-whatif-tui-round-1.md`.** Final slice of #43. Parent spec:
`design/SPEC_synthesize_whatif.md`. Reuses `btctax-core::whatif::{sell,harvest}` (shipped P0-P2) UNCHANGED — NO
new tax logic; tax correctness rides entirely on the already-verified core.

## Goal
An interactive what-if panel in the btctax-tui VIEWER: type a sell amount → live `SellReport` (marginal +
§1212 carryforward + §1(h) bracket + NIIT); or pick a harvest target → `HarvestReport`. Read-only; the viewer
never mutates the vault (existing invariant, app.rs:120 — `handle_key` mutates ONLY UI-nav fields).

## The one data gap + fix
The `Snapshot` (app.rs:104) already carries `events`, `state`, `cli_config`, `profiles: BTreeMap<i32,
TaxProfile>`, `tables: BundledTaxTables` — everything `whatif::{sell,harvest}` needs EXCEPT a `PriceProvider`.
**[★ R0-I1] Fix (the mechanism, corrected):** `Session::prices()` returns a NON-Clone `&dyn PriceProvider`
borrow of a Session the viewer drops (unlock.rs:158) — NOT ownable. Instead type the field **`pub prices:
btctax_adapters::LayeredPrices`** (owned, `#[derive(Clone)]`, price.rs:69) and build it in `build_snapshot` via
**`LayeredPrices::load_with_cache(btctax_cli::price_cache::default_cache_path().as_deref())`** (both public +
pure) — byte-identical to the session's own provider (so the panel's baseline matches the Tax tab). **MUST pass
the real cache path — never `None`** (dropping the cache overlay would silently disagree with the viewer's tabs).
The panel calls `whatif::sell(&snap.events, &snap.prices, &snap.cli_config.to_projection(), year, profile,
&snap.tables, …)`. **[R0-I2] The new mandatory field breaks ~10 `Snapshot` construction sites in btctax-tui-edit
(draw_edit.rs:5306; main.rs:9169/9217/9299/9421/9578/9919/13566/13591) + btctax-tui test builders — a REQUIRED
P3 sweep to add `prices` at every site** (else the workspace won't build).

## The panel (viewer overlay — follows the export-modal / sort-overlay patterns)
- **Open:** a new keybinding **`w`** (What-if) from any output tab → `App.whatif: Option<WhatIfPanel>` (mirrors
  `export_modal: Option<…>`, app.rs:155). Esc closes. `handle_key` still mutates ONLY UI state + this panel
  field (extend the app.rs:120 doc-comment allow-list).
- **Mode toggle:** Tab/`s`/`h` switches Sell ⇄ Harvest within the panel.
- **Inputs (text-entry sub-fields, the unlock/export input pattern; [R0-M3] the panel takes focus + gets keys
  FIRST while open):**
  - **[★ R0-I3] `at: TaxDate`** — an EXPLICIT date field (FMV is strictly per-DATE, not per-year; sell/harvest
    key the as-of pool + ST/LT boundary + daily-close FMV on `req.at`, whatif.rs:216). Default convention:
    **today** when `selected_year` == the current year, else **the last day of `selected_year`** (stated on
    the panel). Editable.
  - Sell: amount (**accept BTC decimal**, e.g. `0.05`, parse→sat — resolves the `whatif-sell-btc-input`
    FOLLOWUP for the TUI); wallet (a picker over the pool's wallets); optional price (default = the daily-close
    FMV for `at`; a future/off-dataset `at` with no bundled price surfaces `ProceedsRequired` until entered).
  - Harvest: target selector (`zero-ltcg | fifteen-ltcg | gain=$X | tax=$X`); wallet; optional price.
- **[★ R0-M2] Compute is EXPLICIT (Enter), not per-keystroke** — harvest is a multi-fold segment walk (not
  "one fast fold"); recompute only on Enter (or an explicit key), so typing an amount/target/date doesn't
  refold on every character. Sell (one fold) uses the same gate for consistency.
- **Profile:** default to the stored **`snap.profiles.get(&selected_year)`** ([R0-M1] `.get`, NEVER `[year]`
  index — a missing year would panic) when present; else a clearly-labeled placeholder (single filer, `$0`
  ordinary) with an on-panel caveat "no stored tax profile — figures assume single / $0 other income; set one
  via `tax-profile set`" (the placeholder clears `TaxProfileMissing` exactly like the CLI ad-hoc path,
  compute.rs:269). (Ad-hoc income entry is a nice-to-have; v1 uses the stored profile + the caveat.)
- **Output:** render the `SellReport`/`HarvestReport` live in the panel — lots consumed, ST/LT, the bracket +
  room, marginal tax + effective rate, the §1212 carryforward-delta line, NIIT, harvest status/binding
  constraint + disclosures. On `WhatIfError`, show the refusal reason (missing table/profile, pre-2025,
  ProceedsRequired, NoLots, YearNotComputable) verbatim. Recompute on input change (debounced-by-keystroke is
  fine — a fold is fast).
- **Year:** the `[`/`]` year nav (already present) switches `selected_year`, which re-derives the profile and
  the `at`-date default; the `at` field remains editable independently.

## [★] Read-only / non-persistence invariant
The panel calls ONLY `whatif::{sell,harvest}` (clone-fold-discard) + reads `snap`. It NEVER touches the vault,
`conn()`, or any writer. R0 confirmed this is triple-locked (App holds no Session, unlock.rs:91-92; the core is
pure; `handle_key` mutates only UI fields). **[R0-M6] Put the panel in its OWN module (e.g. `whatif_panel.rs`)
or `app.rs` — NOT `export.rs`** — so the existing mechanized source-gate **KAT-E10** (export.rs:715-945, which
scans ALL of btctax-tui/src/ and forbids write-class tokens `conn(`/`save(`/`fs::write`) covers the new panel
FOR FREE. KAT: the viewer's vault file is byte-identical after opening + driving the panel through sell + harvest.

## KATs (btctax-tui)
- **★ `whatif_panel_never_persists`** — drive the panel (open, sell, harvest, close); the vault byte-identical.
- `whatif_panel_sell_renders_report` (a fixture snapshot → the panel shows the marginal/bracket/carryforward
  lines) + `…_harvest_renders_report` (status + binding constraint + disclosures).
- `whatif_panel_btc_input_parses_to_sat` (`0.05` → 5,000,000; reject ambiguous over-precision).
- `whatif_panel_error_renders_refusal` (no profile / pre-2025 / ProceedsRequired shown, not a crash).
- `whatif_panel_toggle_sell_harvest`; `handle_key_still_only_mutates_ui` (the app.rs:120 invariant KAT extended).
- **[R0-M5]** `whatif_panel_w_noop_before_snapshot` (pressing `w` on the unlock screen does nothing, no panic);
  `whatif_panel_no_profile_shows_placeholder_caveat`.
- **[R0-M4]** `build_snapshot_prices_parity` — the snapshot's `LayeredPrices` returns the SAME FMV as the
  session's own provider for a sample date (not merely "is set").
- Regression: **[R0-I2] the ~10 btctax-tui-edit `Snapshot` construction sites + tui test builders are all
  updated** and the viewer/editor suites stay green (the editor uses `open_session`/`render`, not `App`, but
  DOES build `Snapshot` literally — all sites swept).

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
