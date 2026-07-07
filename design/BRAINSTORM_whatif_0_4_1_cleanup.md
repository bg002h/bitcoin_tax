# BRAINSTORM — 0.4.1 what-if cleanup (parser dedup + BTC input) — task #48

**Status: DESIGN — one open UX question for the user.** Source baseline: `main` @ `2e89911`. Recon:
`cycle-prep-recon-whatif-sell-btc-input-tui-parser-dedup.md` (both citations ACCURATE, no drift). Combined
PATCH cycle; neither slug touches tax math (the verified `whatif::{sell,harvest}` core is unchanged) → standard
Opus R0, no Fable.

## Slug-2 (do FIRST — pure refactor): `whatif-tui-parser-dedup`
Today the harvest-target string (`zero-ltcg | fifteen-ltcg | gain=$X | tax=$X`) is parsed in TWO places:
`cmd::whatif::parse_harvest_target` (cli, whatif.rs:110) and a re-implementation in the TUI panel
(whatif_panel.rs — kept separate so the KAT-E10 source-gate, which forbids `cmd::` tokens in btctax-tui, stays
green).
**Design:** add **`impl FromStr for HarvestTarget`** in `btctax-core::whatif` (whatif.rs:371) — the single
source of truth. Its `Err` = a small dedicated `HarvestTargetParseError` (a core type with a `Display`
message), NOT `CliError` (keeps core dep-free of cli). Then:
- `cmd::whatif::parse_harvest_target` becomes `s.parse::<HarvestTarget>().map_err(|e| CliError::…(e.to_string()))`.
- the TUI panel calls `s.parse::<HarvestTarget>()` (it already depends on `btctax_core`, so KAT-E10 stays green —
  no `cmd::` token). The panel keeps only its own error→UI-string mapping.
Result: one parser, the duplication gone, KAT-E10 still satisfied. PATCH (no public CLI/behavior change — the
accepted strings are identical).

## Slug-1 (do SECOND): `whatif-sell-btc-input`
Today CLI `what-if sell --sell <SELL>` deserializes to a raw `i64` sat (cmd/whatif.rs:68) — so `0.05` (BTC) is
rejected; the user must type `5000000`. The **TUI panel already accepts a BTC decimal** (P3). This aligns the
CLI. **[★ the one open UX decision — see below]** the "how" affects both UX and SemVer.
Shared helper: lift the TUI's existing BTC-decimal→sat parse into one place (e.g. `btctax-core::whatif` or a cli
util) so the CLI and TUI use the SAME conversion (8-dp max, reject over-precision, HALF-?-free exact `× 1e8`).

## [★ DECIDED 2026-07-06: Option A] how the CLI accepts a BTC amount for `--sell`
**User chose A — smart parse on `--sell`** (`.` → BTC, bare integer → sat; non-breaking, PATCH, matches the TUI).
- **A — smart parse on `--sell` (PATCH, CHOSEN):** if the value contains a `.` → parse as BTC (`×1e8`→sat,
  8-dp max); a bare integer stays sat. Non-breaking (existing `--sell 5000000` callers unaffected); matches the
  TUI; the common inputs (`0.05` BTC / `5000000` sat) are unambiguous. Only oddity: `5000000.0` reads as 5M BTC
  (absurd → errors at the pool feasibility check, not silently).
- **B — separate `--btc <DECIMAL>` flag (MINOR):** keep `--sell <sat>`; add `--btc 0.05` (mutually exclusive,
  exactly one required). Fully explicit, zero ambiguity — but a NEW flag (MINOR, + man/README mirror).
- **C — unit suffix on `--sell` (PATCH):** `0.05btc` / `5000000sat` / bare = sat. Fully explicit, no ambiguity,
  but more typing + a custom value parser.

## Plan (TDD)
- **P1 (slug-2)** — `FromStr for HarvestTarget` + `HarvestTargetParseError` in core; rewire cmd + panel; KATs:
  `harvest_target_fromstr_matches_prior_parsers` (all forms incl. `$`/comma-optional, X<0 rejected), panel +
  cli both use it, KAT-E10 still green.
- **P2 (slug-1)** — the `--sell` BTC accept (per the chosen option) + the shared BTC→sat helper; KATs:
  `sell_0_05_btc_equals_5000000_sat`, `sell_bare_integer_stays_sat` (A), `sell_over_precision_rejected`
  (`0.000000001` → error), and the option-specific parse KATs; regenerate `docs/man/btctax-what-if-sell.1` +
  README.
- Whole-diff; ship in 0.4.1.

## Non-goals
No tax-logic change; no new harvest targets; the harvest `--target` help string stays identical.
