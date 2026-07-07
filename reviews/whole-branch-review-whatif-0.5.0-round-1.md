# Whole-diff review (Phase E) — feat/whatif-0.4.1 (the 0.5.0 cleanup) — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Diff `main (2e89911)..ad084dc` — 2 task commits (T1 `dc6fc53` FromStr dedup, T2 `ad084dc` --sell BTC). Contract:
`design/SPEC_whatif_0_4_1_cleanup.md` (R0-GREEN, 2 rounds). NO tax-math change (verified whatif core untouched).

## Verified by KAT + a real run (my runs)
- **[★ C1 parity] `harvest_target_gain_negative_parses_not_rejected`** — `gain=-1`→`Gain(-1)` (parser is a pure
  lexer; the engine refuses it). `harvest_target_fromstr_matches_prior_parsers` — byte-for-byte with the old
  `parse_harvest_target` (case-insensitive aliases, `$`/`,` stripped). All 24 core whatif KATs pass.
- **[★ I3 non-breaking] `sell_arg_sat_path_byte_identical_incl_negative`** — `parse_sell_arg("-5")==Ok(-5)`,
  matches raw `i64::from_str`; the sat path is unchanged.
- **[I1 two helpers] `parse_btc_amount_bare_one_is_one_btc`** (bare `1`=1 BTC — the TUI meaning);
  `parse_sell_arg_dot_is_btc_int_is_sat` (`0.05`→5,000,000; `5000000`→5,000,000); `sell_over_precision_rejected`
  (`0.000000001`→error, exact); `sell_btc_negative_rejected`.
- **My smoke:** `what-if sell --sell 0.05` == `--sell 5000000` (identical: LT $4,514.75, 15% bracket, marginal
  $677.21). Both `--sell` sites (what-if sell + `optimize consult`) accept BTC (live-confirmed).
- **[dedup] KAT-E10 green** — `e10_mechanized_source_gate` passes; the TUI panel calls
  `btctax_core::whatif::HarvestTarget::from_str` + `parse_btc_amount`, never `cmd::`. Three duplicate sat parsers
  + two target parsers collapse to `parse_sell_arg` + `HarvestTarget::FromStr`.

## A spec-error the implementer correctly overrode (not a finding — a good catch)
The spec's separator golden claimed `gain=1_000`→`BadAmount`. Empirically `rust_decimal`'s `Usd::from_str("1_000")`
= `Ok(1000)` (it accepts `_` as a digit separator), so the LEGACY lexer already produced `Gain(1000)`. Per the
byte-for-byte-parity directive, the implementer PRESERVED parity (no new check) + corrected the KAT to assert
`Gain(1000)` — the right call (a `_`-reject would have broken parity, untested/silent). My spec + both R0 rounds
missed this; the empirical verification caught it.

## Scope / suite
btctax-core (+`FromStr`/`HarvestTargetParseError`/`parse_btc_amount`/`parse_sell_arg` — additive pub API →
MINOR) + btctax-cli (both `--sell` sites + the target rewire + doc-comments) + btctax-tui (panel calls the
shared parsers). Full close-out re-running (implementer: 0 failed; core whatif 24 / cli lib 101 / tui 126).
Incidental: `xtask docs` also refreshed a stale `btctax-update-prices.1` version stamp (v0.3.0→v0.4.0,
deterministic at the current crate version). Release = **0.5.0**.

**SHIP — the parser dedup + `--sell` BTC input are correct and non-breaking (sat path byte-identical, `gain=-1`
still parses, KAT-E10 green), resolving both FOLLOWUPS. Bump to 0.5.0 + publish next.**
