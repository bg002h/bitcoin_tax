# SPEC — 0.4.1 what-if cleanup (harvest-target parser dedup + `--sell` BTC input) — task #48

**Source baseline:** `main` @ `2e89911` (branch `feat/whatif-0.4.1`). **Review status: DRAFT — awaiting R0
(Opus; no tax-math change → no Fable).** Brainstorm: `design/BRAINSTORM_whatif_0_4_1_cleanup.md`. Recon:
`cycle-prep-recon-whatif-sell-btc-input-tui-parser-dedup.md`. Two FOLLOWUPS, one combined PATCH cycle. The
verified `whatif::{sell,harvest}` tax core is UNCHANGED.

## P1 — `whatif-tui-parser-dedup`: one harvest-target parser (`FromStr`)
Today `zero-ltcg | fifteen-ltcg | gain=$X | tax=$X` is parsed twice — `cmd::whatif::parse_harvest_target`
(cli, cmd/whatif.rs:110) + a re-implementation in the TUI panel (whatif_panel.rs), the latter kept separate so
the **KAT-E10 source-gate** (forbids `cmd::` tokens in btctax-tui/src) stays green.
- **Add `impl FromStr for HarvestTarget`** in `btctax-core::whatif` (next to the enum, whatif.rs:371) — the
  single source of truth. Move the EXACT current accept-logic into it: the three aliases each
  (`zero-ltcg`/`zero_ltcg`/`zeroltcg`, etc.), `gain=$X` / `tax=$X` with **`$` and thousands-commas optional**
  and **X ≥ 0 required** (reject negatives — preserve today's `cmd/whatif.rs:110-135` behavior EXACTLY).
- **Err type:** a new `pub enum HarvestTargetParseError` in `btctax-core::whatif` with a `Display` (e.g.
  "unrecognized target '<s>' (expected zero-ltcg | fifteen-ltcg | gain=$X | tax=$X)"; "gain must be ≥ 0"). It
  is a CORE type — NOT `CliError` (keeps btctax-core free of any cli dep).
- **Rewire:** `cmd::whatif::parse_harvest_target` → `s.parse::<HarvestTarget>().map_err(|e|
  CliError::…(e.to_string()))` (keep the exact CliError variant it uses today so cli error messages are stable);
  the TUI panel's local parse → `s.parse::<HarvestTarget>()` mapping the Err to its UI string. **KAT-E10 stays
  green** — the panel already depends on `btctax_core`, so no `cmd::` token appears.
- NO behavior/surface change (identical accepted strings + identical rejections). PATCH.

## P2 — `whatif-sell-btc-input`: `--sell` accepts BTC (smart parse, Option A)
Today `WhatIf::Sell { sell }` (cli.rs:334) deserializes to a raw `i64` sat → `cmd/whatif.rs:68 sell_sat: i64`.
- **[★ Option A] `--sell` is a STRING arg with a smart parser** (not a bare `i64`): if the trimmed value
  **contains `.`** → parse as **BTC decimal** → sat (`× 100_000_000`, **≤ 8 fractional digits**, EXACT integer
  sat — reject > 8 dp as over-precision, e.g. `0.000000001`); **else** parse as a **bare integer sat** (today's
  behavior — `--sell 5000000` UNCHANGED). Reject negatives + non-numeric with a clear message. `5000000.0` →
  5,000,000 BTC (huge) → not special-cased; fails downstream at the pool feasibility check (NoLots/insufficient),
  never silently.
- **Shared helper:** put the BTC-decimal→sat conversion in ONE place so the CLI and the TUI use the SAME exact
  logic. **Lift the TUI panel's existing BTC→sat parse** into `btctax-core::whatif` (e.g. `pub fn
  parse_btc_or_sat(s: &str) -> Result<Sat, …>` + a `parse_btc_amount`), and have BOTH the CLI `--sell` and the
  TUI amount field call it (dedups a SECOND parser as a bonus; keeps KAT-E10 green — core, not cmd).
- **No breakage:** existing `--sell <integer>` callers are byte-identical; only decimal inputs are newly
  accepted. PATCH.

## KATs
- **P1:** `harvest_target_fromstr_matches_prior_parsers` — every accepted form (3 aliases each, `gain=$1,000`,
  `gain=1000`, `tax=$0`) parses identically to the pre-refactor result, and every rejection (`gain=-1`, `foo`,
  empty) errors; `cmd_and_panel_share_fromstr` (both call `HarvestTarget::from_str`); **KAT-E10 still passes**
  (no `cmd::` in the panel).
- **P2:** `sell_0_05_btc_equals_5000000_sat`; `sell_bare_integer_stays_sat` (`--sell 5000000` == 5,000,000 sat,
  byte-identical to today); `sell_over_precision_rejected` (`0.000000001` → error, not truncation);
  `sell_negative_and_nonnumeric_rejected`; `parse_btc_or_sat_shared_by_cli_and_tui` (same helper, same result).
- Regression: the full whatif + cli + tui suites stay green; the harvest `--target` help string is byte-identical.

## Scope / SemVer / lockstep
btctax-core (+`FromStr`/`HarvestTargetParseError`, +`parse_btc_or_sat` helper) + btctax-cli (rewire target parse
+ the `--sell` smart parser) + btctax-tui (panel calls the shared parsers). **PATCH → 0.4.1** (no new public
surface, no breaking change; `--sell` decimal is additive/non-breaking). **Lockstep:** regenerate
`docs/man/btctax-what-if-sell.1` (the `--sell` doc-comment: "accepts either a sat integer or a BTC decimal,
e.g. `0.05` or `5000000`") + the README `what-if` note. No GUI/schema-mirror (this repo has none). Network
isolation unchanged.

## Plan (TDD)
- **T1 (P1 dedup)** — `FromStr`/error in core; rewire cmd + panel; the P1 KATs; confirm KAT-E10 green.
- **T2 (P2 BTC input)** — `parse_btc_or_sat` in core; the `--sell` smart parser (cli) + the TUI amount field
  both call it; the P2 KATs; man page + README; whole-diff; ship 0.4.1.

## Gotchas
- **[P1 exact parity]** the `FromStr` must accept/reject EXACTLY what `parse_harvest_target` does today (aliases,
  `$`/comma-optional, X≥0) — a golden-parity KAT; keep the cli's error VARIANT stable.
- **[P1 KAT-E10]** the panel calls `btctax_core` (`from_str`), never `cmd::` — the source-gate must stay green.
- **[P2 non-breaking]** bare-integer `--sell` is byte-identical to today; only `.`-containing values are new.
- **[P2 over-precision]** reject > 8 fractional digits (no silent truncation of sub-sat).
- **[shared helper]** ONE BTC→sat conversion used by cli + tui (don't fork a third).
