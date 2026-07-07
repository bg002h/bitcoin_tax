# Whole-diff review (Phase E) — feat/whatif-tui STAGE P3 (TUI panel) — round 1

**Verdict: 0 Critical / 0 Important — SHIP. This COMPLETES task #43 (P0–P3).**

Diff `main (21f05ac)..90611c0` — 1 task commit (P3). Contract: `design/SPEC_whatif_tui.md` (R0-GREEN, 2 rounds).
UI slice reusing the VERIFIED `btctax-core::whatif::{sell,harvest}` unchanged — no new tax logic.

## ★ The read-only / non-persistence invariant — DOUBLE-LOCKED (my runs)
- **`whatif_panel_never_persists`** (drive open→sell→harvest→close; vault byte-identical) — passes.
- **`e10_mechanized_source_gate`** — the existing source-gate scans ALL btctax-tui/src incl. the new
  `whatif_panel.rs` (deliberately NOT in export.rs) and forbids write-class tokens (`conn(`/`save(`/`fs::write`)
  — passes. `handle_key_still_only_mutates_ui` confirms the allow-list. The panel cannot reach a writer.

## Verified by KAT (my runs)
- **[I1 prices] `build_snapshot_prices_parity`** — `Snapshot.prices: LayeredPrices` returns the SAME FMV as the
  session's own provider for a sample date (built via `load_with_cache(default_cache_path())`, mirroring
  `Session::default_prices()`). The panel's baseline matches the viewer's tabs.
- **[I2 editor sweep] btctax-tui-edit 266/0** — all Snapshot construction sites updated; the whole workspace
  builds.
- **[I3 at-date + panel]** `sell_renders_report`, `harvest_renders_report`, `harvest_target_parses_all_forms`,
  `btc_input_parses_to_sat` (0.05→5,000,000; over-precision rejected), `error_renders_refusal_{no_lots,pre_2025}`,
  `no_profile_shows_placeholder_caveat`, `toggle_sell_harvest`, `focus_cycles_and_char_routes_to_focused_field`,
  `whatif_panel_w_noop_before_snapshot` — all pass. Compute is Enter-gated (M2). btctax-tui: 124/0.

## Scope / suite
btctax-tui (+`whatif_panel.rs`, +`Snapshot.prices`, +the `w` key) + btctax-tui-edit (the sweep). btctax-core/cli
UNCHANGED (the core reused verbatim). Full close-out re-running. Part of the 0.4.0 cycle.

## FOLLOWUP (non-blocking, filed)
The panel re-implements the harvest-target parser + refusal formatter locally (to avoid the `cmd::` token
KAT-E10 forbids) — a small UI-layer duplication (NOT tax logic). Consider moving the target parser to
`btctax-core::whatif` so both `cmd::` and the panel share one source.

**SHIP P3 — the read-only what-if panel reuses the verified core, cannot write the vault (behavioral + source-
gate KATs), and the editor sweep keeps the workspace green. Task #43 is complete: what-if sell + harvest, CLI +
TUI.**
