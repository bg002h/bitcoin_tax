# SPEC — 0.5.0 what-if cleanup (harvest-target parser dedup + `--sell` BTC input) — task #48

**Source baseline:** `main` @ `2e89911` (branch `feat/whatif-0.4.1`). **Review status: R0 round 1 folded
(1C/4I/3M/2N — Opus; behavior-preservation traps + SemVer; merged IN-PLACE). Awaiting R0 round 2.** Review:
`reviews/R0-spec-whatif-0.4.1-round-1.md`. **[R0-I2] SemVer corrected: MINOR → 0.5.0** (adding pub API —
`FromStr`/`HarvestTargetParseError`/the parse helpers — to published btctax-core is additive-public, NOT PATCH). Brainstorm: `design/BRAINSTORM_whatif_0_4_1_cleanup.md`. Recon:
`cycle-prep-recon-whatif-sell-btc-input-tui-parser-dedup.md`. Two FOLLOWUPS, one combined PATCH cycle. The
verified `whatif::{sell,harvest}` tax core is UNCHANGED.

## P1 — `whatif-tui-parser-dedup`: one harvest-target parser (`FromStr`)
Today `zero-ltcg | fifteen-ltcg | gain=$X | tax=$X` is parsed twice — `cmd::whatif::parse_harvest_target`
(cli, cmd/whatif.rs:110) + a re-implementation in the TUI panel (whatif_panel.rs), the latter kept separate so
the **KAT-E10 source-gate** (forbids `cmd::` tokens in btctax-tui/src) stays green.
- **Add `impl FromStr for HarvestTarget`** in `btctax-core::whatif` (next to the enum, whatif.rs:371) — the
  single source of truth. **[★ R0-C1 + R0-M2] Move `parse_harvest_target`'s logic BYTE-FOR-BYTE, adding NO new
  checks:** lowercase the WHOLE string (→ `GAIN=`/`TAX=` accepted, case-insensitive), the trim/double-trim, the
  aliases (`zero-ltcg`/`zero_ltcg`/`zeroltcg`, same for fifteen), `gain=X`/`tax=X` where the amount = `Usd::from_str`
  after stripping **`$` and `,`** (NOT `_`). **DO NOT reject negatives.** Today `Usd = Decimal` (conventions.rs:8)
  parses `-1` fine, so `gain=-1` → `Gain(-1)` and the ENGINE refuses it as `InvalidTarget` (harvest.rs:1143).
  Rejecting it in the parser would MOVE the refusal (different error class/path/message) and BREAK parity — the
  parser must stay a pure lexer. (No CLI test covers `gain=-1` today, so a parser-side reject would ship silently.)
- **Err type [R0-I4]:** a new `pub enum HarvestTargetParseError` in `btctax-core::whatif` with a `Display`
  covering the TWO current failure cases (cmd/whatif.rs:124 + :130): **`UnrecognizedTarget(String)`** ("expected
  zero-ltcg | fifteen-ltcg | gain=$X | tax=$X") and **`BadAmount(String)`** (the `Usd::from_str` failure — e.g.
  `gain=abc`). NO "must be ≥ 0" variant (per C1, negatives are NOT a parse error). CORE type — NOT `CliError`
  (keeps btctax-core cli-dep-free). (cli error messages aren't test-pinned, so the map-back is low-risk; keep
  the existing CliError variant.)
- **Rewire:** `cmd::whatif::parse_harvest_target` → `s.parse::<HarvestTarget>().map_err(|e|
  CliError::…(e.to_string()))` (keep the exact CliError variant it uses today so cli error messages are stable);
  the TUI panel's local parse → `s.parse::<HarvestTarget>()` mapping the Err to its UI string. **KAT-E10 stays
  green** — the panel already depends on `btctax_core`, so no `cmd::` token appears.
- NO behavior/surface change (identical accepted strings + identical rejections). PATCH.

## P2 — `whatif-sell-btc-input`: `--sell` accepts BTC (smart parse, Option A)
**[R0-M1 citation fix]** `WhatIf::Sell.sell` is ALREADY a `String` (cli.rs:337), manually parsed to sat at
main.rs:224 (NOT a bare `i64`); the smart parser replaces that manual parse. **[★ R0-I1] TWO helpers in
`btctax-core::whatif`** — the TUI field means BTC, the CLI `--sell` is smart, so they CANNOT share one parser:
- **`parse_btc_amount(s) -> Result<Sat, …>`** — BTC-ONLY (a bare `1` = **1 BTC** = 100,000,000 sat). Lift the
  TUI panel's EXISTING parse (whatif_panel.rs:417/572): strip **`_` and `,`** (NOT `$` — R0-M3), `Decimal::from_str`,
  **≤ 8 fractional digits** (reject over-precision `0.000000001`: after `× dec!(1e8)`, `sat.fract() != 0` ⇒ error;
  EXACT, no float), reject negative. **The TUI amount field calls THIS unchanged** — bare `1` stays 1 BTC (do
  NOT point it at the smart parser, or `1`→1 sat silently breaks the TUI, whatif_panel.rs:572).
- **`parse_sell_arg(s) -> Result<Sat, …>`** — the SMART CLI parser: trimmed value **contains `.`** →
  `parse_btc_amount(s)`; **else** parse as a **bare integer sat EXACTLY as today** (`i64::from_str`, main.rs:224).
- **[★ R0-I3 non-breaking, incl. negatives] the non-`.` (sat) path is BYTE-IDENTICAL to today** — `--sell -5`
  computes today's degenerate report (whatif.rs:232), so the sat path passes `-5` through as −5 sat; **NO
  sat-side negative check**. Only the `.`-BTC path rejects negatives/over-precision (those are NEW inputs).
  `5000000.0` → 5,000,000 BTC → fails at the pool feasibility check, never silently.
- **[R0-M1] apply `parse_sell_arg` to `optimize consult --sell` too** (main.rs:171 — the THIRD identical sat
  parser) so both `--sell` sites accept BTC consistently (same helper; cheap).
- **No breakage:** existing `--sell <integer>` callers (incl. `-5`) are byte-identical; only `.`-values are new.

## KATs
- **P1:** `harvest_target_fromstr_matches_prior_parsers` — every accepted form (3 aliases each incl.
  case-insensitive `GAIN=`, `gain=$1,000`==`gain=1000`, `tax=$0`) parses identically to the pre-refactor result;
  **[★ C1] `harvest_target_gain_negative_parses_not_rejected`** (`gain=-1` → `Gain(dec!(-1))`, NOT a parser
  error — the engine refuses it downstream); rejections limited to `foo`/empty/`gain=abc`;
  `cmd_and_panel_share_fromstr`; **KAT-E10 still passes** (no `cmd::` in the panel).
- **P2:** **[I1] `parse_btc_amount_bare_one_is_one_btc`** (`1`→100,000,000 sat — the TUI meaning);
  **`parse_sell_arg_dot_is_btc_int_is_sat`** (`0.05`→5,000,000; `5000000`→5,000,000);
  **[★ I3] `sell_arg_sat_path_byte_identical_incl_negative`** (`-5`→−5 sat, matches today's degenerate path);
  `sell_over_precision_rejected` (`0.000000001` → error, EXACT no truncation); `sell_btc_negative_rejected`
  (`-0.05` on the BTC path → error); **`tui_amount_field_uses_parse_btc_amount`** (unchanged — bare `1` = 1 BTC).
- Regression: the full whatif + cli + tui suites stay green (incl. the existing TUI BTC KAT + `optimize consult
  --sell`); the harvest `--target` help string byte-identical.

## Scope / SemVer / lockstep
btctax-core (+`FromStr`/`HarvestTargetParseError`, +`parse_btc_amount`/`parse_sell_arg` helpers) + btctax-cli
(rewire the target parse + the `what-if sell` AND `optimize consult` `--sell` smart parse) + btctax-tui (panel
calls `HarvestTarget::from_str` + keeps `parse_btc_amount` for the amount field). **[★ R0-I2] MINOR → 0.5.0**
(NOT 0.4.1 — the new `pub` `FromStr`/`HarvestTargetParseError`/`parse_btc_amount`/`parse_sell_arg` are additive
PUBLIC API on published btctax-core; behavior stays non-breaking, but the surface grows). (Branch name
`feat/whatif-0.4.1` is now a misnomer — the release is 0.5.0.) **Lockstep:** regenerate
`docs/man/btctax-what-if-sell.1` (+ `btctax-optimize-consult.1` if it shows `--sell`) from the cli.rs:335
doc-comment via `xtask docs` ("accepts a sat integer OR a BTC decimal, e.g. `0.05` or `5000000`") + the README
`what-if` note. No GUI/schema-mirror (this repo has none). Network isolation unchanged.

## Plan (TDD)
- **T1 (P1 dedup)** — `FromStr`/error in core; rewire cmd + panel; the P1 KATs; confirm KAT-E10 green.
- **T2 (P2 BTC input)** — `parse_btc_or_sat` in core; the `--sell` smart parser (cli) + the TUI amount field
  both call it; the P2 KATs; man page + README; whole-diff; ship 0.4.1.

## Gotchas
- **[★ P1-C1 pure lexer]** the `FromStr` accepts/rejects EXACTLY what `parse_harvest_target` does today —
  aliases (case-insensitive), `$`/comma-optional — and **does NOT reject negatives** (`gain=-1`→`Gain(-1)`, the
  ENGINE refuses it). No new checks; keep the cli's error VARIANT stable.
- **[P1 KAT-E10]** the panel calls `btctax_core` (`from_str`), never `cmd::` — the source-gate must stay green.
- **[★ P2-I1 two helpers]** `parse_btc_amount` (BTC-only, bare `1`=1 BTC) for the TUI amount field;
  `parse_sell_arg` (smart: `.`→BTC else sat) for the CLI `--sell`. NEVER point the TUI field at the smart parser.
- **[★ P2-I3 non-breaking incl. negatives]** the non-`.` sat path is byte-identical to today (`-5`→−5 sat,
  degenerate — no sat-side sign check); only the `.`-BTC path is new + rejects negatives/over-precision.
- **[P2 over-precision]** reject > 8 fractional digits on the BTC path (EXACT `sat.fract()!=0`, no float truncation).
- **[M1 third parser]** apply the smart parse to `optimize consult --sell` too (main.rs:171).
