# SPEC — comprehensive price data + pseudo-FMV + online price backup

**Source baseline:** `main` @ `019ed3f` (branch `feat/price-data-fmv`). **Review status: DRAFT — awaiting R0.**
Brainstormed + user-approved (2026-07-05): all-three-together, online = explicit-command-→-local-cache.
Resolves the real-vault-test `[FmvMissing]` finding (27 income events) at its root + adds gap-date coverage.

## Goal & decomposition (ship together; 3 phases)
- **A — Bundle the real daily-close dataset.** The shipped `btctax-adapters/data/btc_usd_daily_close.csv` is a
  **7-row STUB** — the sole reason the vault test hit `[FmvMissing]` everywhere. Replace it with Quantoshi's
  `BitcoinPricesDaily.csv` (**5,802 daily closes, 2010-07-17 → 2026-06-03**).
- **B — Pseudo-reconcile FMV default.** Native income FMV does NOT auto-fill from prices (resolve.rs:285-286 —
  `manual_fmv` OR the import's own FMV, no price fallback; unlike disposal *proceeds*, evaluate.rs:93). So under
  **pseudo mode**, synthesize the missing income FMV from the daily close — flagged `[PSEUDO]`, correctable,
  approve-able, attestation-gated — clearing the `[FmvMissing]` Hard blocker for the fast-estimate workflow.
- **C — `btctax update-prices` (explicit, opt-in, network).** For dates outside the bundled range (recent/
  future/gaps), fetch from Binance/CoinGecko into a LOCAL cache the provider reads. **The projection NEVER
  touches the network** — determinism/offline/privacy preserved.

## Data facts (verified)
- Quantoshi `BitcoinPricesDaily.csv`: header `Date,Price`; `M/D/YY` dates; float price = **daily close**
  (Binance klines primary 8-dp; CoinGecko `market_chart` fallback; `--lag 8` settle). **BSD-2-Clause, ©2026
  bg002h** (the user) → redistributable in the published crate WITH the BSD-2 attribution.
- btctax format (price.rs:8-45): `date,usd_close`, ISO `[year]-[month]-[day]`, `Decimal`; `BundledPrices` =
  `BTreeMap<TaxDate,Usd>`, EXACT-date lookup (`usd_per_btc`, price.rs:47-49). Pure/deterministic (NFR4).
- Provider wiring: session.rs:449-450 `let prices = BundledPrices::load()?; project(&events, &prices, &cfg)`.
  `fmv_of(prices, date, sat)` (price.rs:13) computes FMV. No HTTP client anywhere in the workspace today.

## Part A — bundle the comprehensive dataset
- Convert once (a committed data file, not runtime): `M/D/YY → YYYY-MM-DD`; price → `Decimal` (keep source
  precision, or round to a documented dp — **decide: round to 2dp cents** for a defensible daily close + smaller
  file; R0 to confirm). Header `date,usd_close`. Sorted ascending, one row/day, no dupes. Replace the stub;
  `BundledPrices::from_csv_str` parses it unchanged (already ISO/Decimal).
- Add BSD-2 attribution: a `data/BitcoinPricesDaily.LICENSE` (or header comment + a NOTICE) crediting
  Quantoshi/bg002h BSD-2. Verify the crate still publishes (size: ~5.8k rows ≈ 150KB — fine).
- **A alone does NOT clear the vault-test 27** — income FMV has no price fallback (that's B). A makes the DATA
  available so B (and disposal-proceeds auto-fill, and the display auto-FMV) resolve.
- KATs: `bundled_dataset_covers_2010_to_2026` (spot-dates present); `bundled_dataset_parses_and_is_sorted`;
  `bundled_dataset_row_count` (guards accidental truncation); existing price KATs stay green.

## Part B — pseudo FMV default (the pseudo-reconcile seam, sub-project-2 pattern)
- New `PseudoDefault` flavor: for an unresolved native-income event with `fmv == None` AND
  `prices.usd_per_btc(date).is_some()`, inject a synthetic FMV `= fmv_of(prices, date, sat)` at the resolve/`Eff`
  layer (resolve.rs `pseudo_decisions`, the seam that carries pseudo-taint into the fold — like the existing
  self-transfer-$0 / classify-raw defaults). The income recognizes at the daily-close FMV, flagged `[PSEUDO]`.
- If `prices.usd_per_btc(date) == None` (no local price even after A/C) → NO synthetic; the `[FmvMissing]` Hard
  blocker STAYS (honest — pseudo won't fabricate a price out of nothing). This is the residual C addresses.
- Real supersedes (a user `SetFmv`/import FMV → no synthetic); `approve` promotes to a real `SetFmv`;
  attestation-gated on export (sub-3); `off` reverts. Deterministic.
- KATs: `pseudo_fills_income_fmv_from_daily_close` (missing-FMV income + a bundled date → recognized at the
  close, `[PSEUDO]`); `pseudo_fmv_absent_when_no_price` (date not in data → still `[FmvMissing]`, no synthetic);
  `real_setfmv_supersedes_pseudo_fmv`; `approve_promotes_pseudo_fmv_to_real_setfmv`;
  `pseudo_fmv_flagged_and_export_gated`. **★ fault-inject:** force the synthetic FMV regardless of price
  availability ⇒ `pseudo_fmv_absent_when_no_price` RED.

## Part C — `btctax update-prices` (explicit online backup → local cache)
- **Layered provider** `btctax_adapters::LayeredPrices { bundled: BundledPrices, cache: BundledPrices }` (or
  `BundledPrices::load_with_cache(cache_path)`): `usd_per_btc` = cache-over-bundled (both local reads →
  deterministic). Cache file = same `date,usd_close` format at a per-user path (default
  `dirs::data_dir()/btctax/price_cache.csv`; `--price-cache <path>` override). session.rs:449 loads the layered
  provider. Cache absent → behaves exactly as bundled-only (byte-identical).
- **Command** `btctax update-prices [--from DATE] [--to DATE] [--lag N=8] [--dry-run] [--source auto|binance|
  coingecko]`: fetch daily closes (Binance klines primary; CoinGecko `market_chart/range` fallback — mirror
  `update_prices.py`), skip the `--lag` most-recent days, APPEND new rows to the cache (never overwrite bundled;
  idempotent — skip dates already present), print a summary (rows added, range, source). `--dry-run` previews.
- **Network isolation:** ONLY this command opens a socket. Add a sync HTTPS client — **`ureq` (rustls-tls,
  blocking)** — to btctax-cli. The core/projection/all other commands stay network-free (assert via no
  network dep leaking into btctax-core). Clear opt-in; a User-Agent; timeouts; graceful offline error.
- KATs: `layered_prices_cache_over_bundled` (cache date wins / fills a gap); `cache_absent_is_bundled_only`
  (byte-identical); `update_prices_dry_run_writes_nothing`; `update_prices_appends_and_is_idempotent` (parse a
  CANNED Binance/CoinGecko JSON fixture — NO live network in tests); `update_prices_respects_lag`. A live-network
  smoke test is `#[ignore]` (opt-in, not in CI).

## Scope / SemVer / lockstep
`btctax-adapters` (data file + `LayeredPrices`) + `btctax-core` (the pseudo FMV `PseudoDefault` flavor +
threading) + `btctax-cli` (the `update-prices` command + `ureq` dep + session wiring). MINOR (new command + new
pseudo default + a network-capable dep). Docs: new `btctax-update-prices` man page (clap doc-comment →
regenerate); README note on offline-by-default + the opt-in updater + BSD-2 price-data attribution; FOLLOWUPS.
**All 7 crates bump for the next release** (the data change ships in btctax-adapters).

## Plan (TDD; phased — a phase may stop-at-green if budget-bound)
- **T1 (A)** — convert + commit the dataset + attribution; the A KATs; confirm existing price KATs green.
- **T2 (B)** — the pseudo income-FMV `PseudoDefault` (resolve injection + taint + approve + attest); the B KATs
  + the ★ fault-inject; re-verify against the real-vault fixture (the 27 clear under pseudo).
- **T3 (C)** — `LayeredPrices` + cache wiring; the `update-prices` command + `ureq`; canned-JSON KATs; the
  man page + README; whole-diff + full suite + FOLLOWUPS.

## Gotchas
- **[B] income FMV has NO normal price fallback** (resolve.rs:285) — the pseudo default is the ONLY price-derived
  income FMV; it must be `[PSEUDO]`-flagged (not a silent real value), consistent with income-attestation conservatism.
- **[B] no price ⇒ stay blocked** — never fabricate an FMV when the date is genuinely absent (fault-inject this).
- **[C] projection stays offline/deterministic** — network ONLY in `update-prices`; the cache is a local read;
  core has NO network dep. Tests use canned JSON, never live network (live smoke is `#[ignore]`).
- **[A] determinism + precision** — `Decimal` (never float); decide + document the rounding; sorted, deduped.
- **[license]** carry the BSD-2 attribution for the bundled data (published crate).
- **[C] cache is user-augmented** — two users' caches differ; the BUNDLED data is the reproducible baseline, the
  cache extends it (and pseudo-FMV is already a flagged estimate) — acceptable; document it.
