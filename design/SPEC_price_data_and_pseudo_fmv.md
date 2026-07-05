# SPEC — comprehensive price data + pseudo-FMV + online price backup

**Source baseline:** `main` @ `019ed3f` (branch `feat/price-data-fmv`). **Review status: DRAFT — Part C revised
to a SEPARATE `btctax-update-prices` binary (user-directed 2026-07-05, so the tax binaries have zero network
deps); R0 round 1 was dispatched against the pre-revision single-binary Part C — reconcile its A/B findings +
map its C notes onto the separate-binary design. Awaiting R0 round 2.** Brainstormed + user-approved: all-three-
together, online = explicit-command-→-local-cache in a dedicated binary. Resolves the real-vault-test
`[FmvMissing]` finding (27 income events) at its root + adds gap-date coverage.

## Goal & decomposition (ship together; 3 phases)
- **A — Bundle the real daily-close dataset.** The shipped `btctax-adapters/data/btc_usd_daily_close.csv` is a
  **7-row STUB** — the sole reason the vault test hit `[FmvMissing]` everywhere. Replace it with Quantoshi's
  `BitcoinPricesDaily.csv` (**5,802 daily closes, 2010-07-17 → 2026-06-03**).
- **B — Pseudo-reconcile FMV default.** Native income FMV does NOT auto-fill from prices (resolve.rs:285-286 —
  `manual_fmv` OR the import's own FMV, no price fallback; unlike disposal *proceeds*, evaluate.rs:93). So under
  **pseudo mode**, synthesize the missing income FMV from the daily close — flagged `[PSEUDO]`, correctable,
  approve-able, attestation-gated — clearing the `[FmvMissing]` Hard blocker for the fast-estimate workflow.
- **C — a SEPARATE `btctax-update-prices` binary (explicit, opt-in, network).** For dates outside the bundled
  range (recent/future/gaps), fetch from Binance/CoinGecko into a LOCAL cache the provider reads. The network
  code lives in its OWN crate/binary so the **tax binaries carry NO network dependency at all** (verifiable) —
  determinism/offline/privacy preserved.

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

## Part C — SEPARATE `btctax-update-prices` binary (network isolated from the tax binaries) [user-directed 2026-07-05]
**The tax binaries (btctax-cli / btctax-tui / btctax-tui-edit) MUST have NO network dependency in their tree** —
a verifiable "the binary that touches your vault cannot open a socket" property. So the online fetch lives in a
**new dedicated crate/binary, `btctax-update-prices`**, the ONLY component that links an HTTP client. The local
cache CSV is the clean, auditable, offline hand-off between them.
- **Layered provider (in btctax-adapters, NO network) — read by the tax binaries:** `LayeredPrices { bundled:
  BundledPrices, cache: BundledPrices }` (or `BundledPrices::load_with_cache(cache_path)`): `usd_per_btc` =
  cache-over-bundled (both local reads → deterministic). Cache = same `date,usd_close` format at a per-user path
  (default `dirs::data_dir()/btctax/price_cache.csv`; a `--price-cache`/env override shared by both binaries).
  session.rs:449 loads the layered provider. Cache absent → byte-identical to bundled-only. **btctax-adapters
  gains NO network dep** (the parse/format/path/provider are pure).
- **New crate `crates/btctax-update-prices`** (binary; the ONLY crate with an HTTP client — **`ureq`,
  rustls-tls, blocking**): `btctax-update-prices [--from DATE] [--to DATE] [--lag N=8] [--dry-run]
  [--source auto|binance|coingecko] [--price-cache PATH]`. Fetches daily closes (Binance klines primary;
  CoinGecko `market_chart/range` fallback — mirror `update_prices.py`), skips the `--lag` most-recent days,
  APPENDs new rows to the cache (idempotent — skip present dates; never touches the bundled data), prints a
  summary. `--dry-run` previews. User-Agent; timeouts; graceful offline error. Depends on btctax-adapters (for
  the CSV format + cache path) + ureq only — NOT on btctax-core/cli.
- **btctax-cli stays network-free:** its "no price for {date}" surfaces (the FmvMissing hint, the Part-B
  no-price case) print `run \`btctax-update-prices\`` — a STRING pointer, no dep, no shell-out.
- **Verifiable isolation KAT:** a test/CI check asserting `ureq`/rustls does NOT appear in `cargo tree` for
  btctax-cli / btctax-tui / btctax-tui-edit / btctax-core / btctax-adapters (only btctax-update-prices).
- KATs: `layered_prices_cache_over_bundled` (cache date wins / fills a gap); `cache_absent_is_bundled_only`
  (byte-identical) — both in btctax-adapters. In btctax-update-prices: `update_prices_dry_run_writes_nothing`;
  `update_prices_appends_and_is_idempotent` (parse a CANNED Binance/CoinGecko JSON fixture — NO live network in
  tests); `update_prices_respects_lag`. A live-network smoke test is `#[ignore]` (opt-in, not in CI).

## Scope / SemVer / lockstep
`btctax-adapters` (data file + `LayeredPrices`, NO network) + `btctax-core` (the pseudo FMV `PseudoDefault`
flavor + threading) + `btctax-cli` (session wiring to `LayeredPrices` + the `btctax-update-prices` pointer
string — **NO `ureq`**) + **NEW crate `btctax-update-prices`** (the sole network-linked binary: `ureq`). The
workspace grows 7 → **8 crates**. MINOR (new binary + new pseudo default). Docs: new `btctax-update-prices`
man page (clap doc-comment → regenerate); README note on offline-by-default, the separate opt-in updater
binary, + the BSD-2 price-data attribution; FOLLOWUPS. **All 8 crates bump for the next release** (the data
change ships in btctax-adapters; the new crate is a first-time publish → the new-crate 5-burst rate limit
applies to it, per [[crate-publishing-state]]).

## Plan (TDD; phased — a phase may stop-at-green if budget-bound)
- **T1 (A)** — convert + commit the dataset + attribution; the A KATs; confirm existing price KATs green.
- **T2 (B)** — the pseudo income-FMV `PseudoDefault` (resolve injection + taint + approve + attest); the B KATs
  + the ★ fault-inject; re-verify against the real-vault fixture (the 27 clear under pseudo).
- **T3 (C)** — `LayeredPrices` + cache wiring in btctax-adapters/the tax binaries (NO ureq) + the "no price →
  run btctax-update-prices" pointer; the **NEW `crates/btctax-update-prices`** binary (ureq + Binance/CoinGecko
  fetch → cache); the **dep-tree isolation check** (ureq absent from the tax binaries); canned-JSON KATs; the
  man page + README; whole-diff + full suite + FOLLOWUPS.

## Gotchas
- **[B] income FMV has NO normal price fallback** (resolve.rs:285) — the pseudo default is the ONLY price-derived
  income FMV; it must be `[PSEUDO]`-flagged (not a silent real value), consistent with income-attestation conservatism.
- **[B] no price ⇒ stay blocked** — never fabricate an FMV when the date is genuinely absent (fault-inject this).
- **[C] the tax binaries have NO network dep in their tree** — network lives ONLY in the separate
  `btctax-update-prices` binary; the cache is a local read; btctax-core/cli/tui/tui-edit/adapters never link an
  HTTP client (verifiable via `cargo tree` — a CI/test check). Tests use canned JSON, never live network (live
  smoke is `#[ignore]`).
- **[A] determinism + precision** — `Decimal` (never float); decide + document the rounding; sorted, deduped.
- **[license]** carry the BSD-2 attribution for the bundled data (published crate).
- **[C] cache is user-augmented** — two users' caches differ; the BUNDLED data is the reproducible baseline, the
  cache extends it (and pseudo-FMV is already a flagged estimate) — acceptable; document it.
