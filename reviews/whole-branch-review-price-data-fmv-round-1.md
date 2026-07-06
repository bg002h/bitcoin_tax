# Whole-diff review (Phase E) — feat/price-data-fmv — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (019ed3f)..HEAD` — 4 impl commits (T1-T3 + the BSD-2 fix), 55 files,
+8,596/−172 (the bulk = the 5,801-row dataset CSV). Contract: `design/SPEC_price_data_and_pseudo_fmv.md`
(R0-GREEN, 3 rounds). Tax-critical; introduces the app's first (isolated) network binary.

## Fault-injection of the two headline guards (restored byte-for-byte)
- **[★ no-price-stays-blocked — the tax-critical guard] CONFIRMED load-bearing.** resolve.rs:1009
  `let Some(synth) = fmv_of(prices, date, x.sat) else { continue }` — a pseudo income-FMV synthetic is injected
  ONLY when a local price exists; no price ⇒ the row stays Hard `FmvMissing` (pseudo never fabricates a value
  out of nothing). **Fault-inject:** replacing the guard with `.unwrap_or_default()` (fabricate a $0 synthetic
  on an unpriced date) drove `pseudo_fmv_absent_when_no_price` RED. The honesty invariant is guarded.
- **[★ network isolation — the user's reason for the separate binary] CONFIRMED.** `cargo tree` shows
  **`ureq`/`rustls` ABSENT from btctax-cli / -tui / -tui-edit / -core / -adapters (all 0)** and PRESENT only in
  `btctax-update-prices`. The vault-touching binaries provably cannot open a socket; also wired as an
  xtask/CI `check-isolation` job.

## Verified by inspection + named KATs
- **T1 (data):** the 6-row stub → **5,801 real daily closes** (2010-07-17 → 2026-06-03; ISO, Decimal 2dp
  HALF_EVEN, sorted/deduped). The R0-C1 test migration used the Session-level `set_prices` seam (inject the old
  stub → zero recompute) where possible, recompute-to-real / move-unpriced-beyond-dataset elsewhere. KATs
  `bundled_dataset_{row_count,parses_sorted_deduped,covers_2010_to_2026}`, `real_date_fmv_is_exact`.
- **T2 (pseudo FMV):** `IncomeRecord.pseudo` set at BOTH fold push sites (native + IncomeInbound);
  `PseudoKind::PseudoFmv`; synthetic `ManualFmv` at the daily close, `[PSEUDO]`-flagged (`pseudo_tag`),
  approve→real `ManualFmv`, export-gated; the reversed sub-2 "0-blockers" contract updated. KATs
  `pseudo_fills_income_fmv_from_daily_close`, `real_manualfmv_supersedes_pseudo_fmv`,
  `approve_promotes_pseudo_fmv_to_manualfmv`, `pseudo_income_fmv_flagged_on_render`,
  `pseudo_income_fmv_export_gated`, `income_fmv_27_clear_under_pseudo` (the vault-test fixture — the 27 clear).
- **T3 (updater):** `LayeredPrices` cache-over-bundled (adapters, no dirs/network, `cache_absent_is_bundled_only`
  byte-identical); the new `btctax-update-prices` crate (Binance primary / CoinGecko fallback, `--lag`/`--dry-run`,
  idempotent append) tested with CANNED JSON (no live network; `live_fetch_smoke` `#[ignore]`). All 8 crates
  0.2.0→0.3.0; the new crate is a first-time publish.
- **BSD-2 correction (af35fce):** the `BitcoinPricesDaily.NOTICE` is deleted; no BSD-2 in README/data — the data
  ship as public Binance/CoinGecko market facts (a README provenance note only). Verified gone.

## Suite
`cargo test --workspace --locked` (implementer 1215 passed / 0 failed / 1 ignored; re-run at merge — the ~1189
sort-views baseline + ~26 new KATs); clippy -D + fmt clean; the updater compiles under MSRV 1.88.

**SHIP — completes the price-data + pseudo-FMV program (A+B+C) and the dogfooding batch.**
