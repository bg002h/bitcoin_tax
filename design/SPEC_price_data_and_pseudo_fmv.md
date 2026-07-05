# SPEC — comprehensive price data + pseudo-FMV + a separate online updater

**Source baseline:** `main` @ `019ed3f` (branch `feat/price-data-fmv`). **Review status: R0 round 1 folded
(1C/4I/5M/3N — merged IN-PLACE; surgical) + Part C = a SEPARATE `btctax-update-prices` binary (user-directed
2026-07-05). Awaiting R0 round 2.** Review: `reviews/R0-spec-price-data-and-pseudo-fmv-round-1.md`. Tax-critical;
introduces the app's first (isolated) network binary. All-three-together; online = explicit dedicated binary →
local cache.

## Goal & decomposition (ship together; 3 phases)
- **A — Bundle the real daily-close dataset.** The shipped `btctax-adapters/data/btc_usd_daily_close.csv` is a
  **6-data-row STUB** (7 lines incl. header) [R0-N1]. Replace it with Quantoshi's `BitcoinPricesDaily.csv`
  (**5,802 daily closes, 2010-07-17 → 2026-06-03**). **A alone does NOT clear the vault-test 27** — income FMV
  is fixed at IMPORT (`fmv_status`), and the ingest is idempotent (`import.rs:2`, `normalize.rs:10-23`), so
  re-projecting with better data never back-fills an already-imported event's FMV [R0-I1]. A supplies the DATA
  that B (and disposal-proceeds auto-fill + display auto-FMV) consume at projection time.
- **B — Pseudo-reconcile FMV default.** Native income FMV has NO price fallback (resolve.rs:285-286 —
  `manual_fmv` OR the import's own FMV; unlike disposal *proceeds*, evaluate.rs:93/129). So under **pseudo
  mode**, synthesize the missing income FMV from the daily close — flagged `[PSEUDO]`, correctable, approve-able,
  attestation-gated — clearing the `[FmvMissing]` Hard blocker for the fast-estimate workflow.
- **C — a SEPARATE `btctax-update-prices` binary (explicit, opt-in, network).** The network code lives in its
  OWN crate so the **tax binaries carry NO network dependency at all** (verifiable via `cargo tree`) — offline/
  deterministic/private preserved. Fetches gap/recent dates into a local cache the pure provider reads.

## Data facts (verified)
- Quantoshi `BitcoinPricesDaily.csv`: header `Date,Price`; `M/D/YY` dates; float = **daily close** (Binance
  klines primary 8-dp; CoinGecko fallback; `--lag 8` settle). **BSD-2-Clause, ©2026 bg002h** → redistributable
  WITH attribution.
- btctax format (price.rs:8-45): `date,usd_close`, ISO, `Decimal`; `BundledPrices` = `BTreeMap<TaxDate,Usd>`,
  EXACT-date lookup. `fmv_of` already `round_cents` [R0-M4] → **2dp source is fine.** Pure/deterministic (NFR4).
- Wiring: session.rs:449-450 `BundledPrices::load()` (hard-wired — see [R0-C1] seam). No HTTP client exists.

## Part A — bundle the dataset + [★ R0-C1] the test-migration (the Critical)
- Convert once (committed data file): `M/D/YY → YYYY-MM-DD`; price → `Decimal` **rounded to 2dp cents** [M4];
  header `date,usd_close`; sorted asc, one row/day, deduped. `from_csv_str` parses it unchanged.
- **[R0-I4] License:** ship a SEPARATE `data/BitcoinPricesDaily.NOTICE` (BSD-2 + ©2026 bg002h) — do NOT put a
  header comment in the CSV (the parser errors on any non-`date` line, price.rs:28-41). Packaging is fine
  (adapters has no include/exclude).
- **[R0-C1] Migrate the tests broken by real data (the plan OWNS this — "existing KATs stay green" was FALSE):**
  - *Exact-FMV pins computed from stub prices* — `adapters/tests/river.rs:54-56` ($6.75←67500),
    `adapters/tests/fmv_fr3.rs:57-61`, `cli/tests/reconcile.rs` `sti_fixture` ($84.00←84000@2025-03-01 + the
    $33.75/$27.00 floors, ~48 refs at ~1815/2168/2338/2461/2600/2614).
  - *"No bundled price" assumptions now COVERED by real data* — `2025-07-04` (fmv_fr3.rs), `2025-12-31`
    (`optimize_consult.rs:406-453`), `2025-04-01` (reconcile.rs `missing_price_count`/`excluded_missing_price`,
    ~1811/2164/2332/2445).
  - **Approach:** add a **test-only provider-injection seam** so these tests use a CONTROLLED synthetic
    `BundledPrices::from_csv_str(...)` instead of `BundledPrices::load()` — decoupling them from the bundled
    dataset (robust + refresh-proof). `session.rs:449` + the bulk plans hard-wire `load()`; add a
    `pub(crate)`/`#[cfg(test)]` constructor or a `project_with_prices` seam to inject `&dyn PriceProvider`. For
    the few genuinely testing bundled coverage, recompute expected values from the real data. A far-future
    unpriced date (>2026-06-03) is the fallback ONLY where injection is infeasible (mark it refresh-fragile).
  - Unaffected (confirmed): `normalize.rs:88` StaticPrices double, price.rs `from_csv_str` unit tests,
    integration.rs, exchange-provided-CSV tests.
- KATs: `bundled_dataset_covers_2010_to_2026` (spot-dates); `bundled_dataset_parses_sorted_deduped`;
  `bundled_dataset_row_count` (truncation guard); **[M4]** `real_date_fmv_is_exact` (pin ONE real date's FMV);
  **[M5]** a committed fixture reproducing the vault-test income event so T2's "27 clear" is source-verifiable.

## Part B — pseudo FMV default
- **[R0-I1] This REVERSES `SPEC_pseudo_reconcile_mode.md` lines 20/107** (which left native-income FmvMissing
  UNCLEARED). Now defensible because the price DATA exists (A). Update the pseudo "0 Hard classification
  blockers" contract: pseudo now ALSO clears native-income `FmvMissing` **when a local price exists** (else it
  stays — the residual C addresses). State this reversal explicitly.
- **Mechanism:** new `PseudoDefault` flavor. For an unresolved native-income event with `fmv == None` AND
  `prices.usd_per_btc(date).is_some()`, inject a synthetic FMV `= fmv_of(prices, date, sat)` at the resolve/`Eff`
  layer (resolve.rs `pseudo_decisions` seam) — like the existing self-transfer-$0 / classify-raw defaults.
- **[R0-I2] Taint MUST reach the income row (the ★ guard):** `IncomeRecord` (state.rs:211-218) has NO `pseudo`
  field and `fold.rs:689-696` pushes it UNFLAGGED (only the lot is tainted, fold.rs:722). Add `pub pseudo: bool`
  to `IncomeRecord`; set it from `eff.pseudo`; render the `[PSEUDO]` marker on the Income tab (render.rs:299-312
  + the sort-views Income render) — mirroring the Lot/leg taint. Without this a pseudo income shows a CLEAN
  dollar figure (guard violation).
- **[R0-M1] Approve target:** there is no `SetFmv` — the real payload is `ManualFmv` (event.rs:157; CLI
  `set-fmv` main.rs:590). `approve` promotes the pseudo FMV to a `ManualFmv` decision (the approve loop
  reconcile.rs:283-292 is already generic); add a new `PseudoKind` + `PseudoKindArg` (cli.rs:638,
  main.rs:1174-1178/1235-1237) + its `approve` filter + display label.
- Real supersedes (a user `ManualFmv`/import FMV → no synthetic); attestation-gated on export (sub-3); `off`
  reverts. Deterministic; Σsat conservation unaffected (FMV is a value, not a quantity).
- KATs: `pseudo_fills_income_fmv_from_daily_close` (missing-FMV income + bundled date → recognized at the close,
  `[PSEUDO]` on `IncomeRecord` + rendered); `pseudo_fmv_absent_when_no_price` (date absent → still `[FmvMissing]`,
  no synthetic); `real_manualfmv_supersedes_pseudo_fmv`; `approve_promotes_pseudo_fmv_to_manualfmv`;
  `pseudo_income_fmv_flagged_and_export_gated`; the vault-fixture "27 clear under pseudo" [M5]. **★ fault-inject:**
  force the synthetic regardless of price availability ⇒ `pseudo_fmv_absent_when_no_price` RED.

## Part C — SEPARATE `btctax-update-prices` binary (network isolated)
- **Layered provider (btctax-adapters, NO network):** `LayeredPrices { bundled, cache: BundledPrices }` (or
  `BundledPrices::load_with_cache(cache_path)`): `usd_per_btc` = cache-over-bundled (both local → deterministic).
  Cache = `date,usd_close` at `dirs::data_dir()/btctax/price_cache.csv` (a shared `--price-cache`/env override).
  Cache absent → byte-identical to bundled-only. **[R0-M2] `dirs` is NOT a dep today — add it** (path resolved
  in the cli, passed into adapters). btctax-adapters gains NO network dep.
- **[R0-I3] Cache-derived FMV provenance/reproducibility (NFR4):** a cache-sourced price feeds the REAL
  auto-FMV path (disposal proceeds evaluate.rs:93/129) UNFLAGGED, yet the cache isn't in the published crate →
  not reproducible from it. Resolution: **treat the cache as a documented LOCAL INPUT** (like the vault) — a
  projection is reproducible GIVEN (events + bundled + cache); the bundled-only projection is the
  published-reproducible baseline. Document this in the command + README; (a per-value cache-provenance marker is
  a FOLLOWUP, not this spec). Pseudo income FMV is separately `[PSEUDO]`-flagged (B), so the estimate path is
  already visible.
- **New crate `crates/btctax-update-prices`** (binary; the ONLY crate linking an HTTP client — **`ureq`,
  rustls-tls, blocking**): `btctax-update-prices [--from D][--to D][--lag N=8][--dry-run][--source auto|binance|
  coingecko][--price-cache PATH]`. Binance klines primary, CoinGecko `market_chart/range` fallback (mirror
  `update_prices.py`); skip `--lag` recent days; APPEND to the cache (idempotent — skip present dates; never
  touches bundled); summary + `--dry-run`. User-Agent; timeouts; graceful offline error. Deps: btctax-adapters
  (format/path) + ureq ONLY — NOT btctax-core/cli.
- **btctax-cli stays network-free:** the "no price for {date}" surfaces (the B no-price case + the FmvMissing
  hint) print `run \`btctax-update-prices\`` — a STRING, no dep, no shell-out.
- **[★] Verifiable isolation KAT/CI check:** `ureq`/rustls absent from `cargo tree` of btctax-cli / -tui /
  -tui-edit / -core / -adapters (present ONLY in btctax-update-prices).
- KATs: `layered_prices_cache_over_bundled`, `cache_absent_is_bundled_only` (btctax-adapters);
  `update_prices_dry_run_writes_nothing`, `update_prices_appends_and_is_idempotent` (CANNED Binance/CoinGecko
  JSON fixtures — NO live network), `update_prices_respects_lag` (btctax-update-prices). Live-network smoke =
  `#[ignore]`.

## Scope / SemVer / lockstep
`btctax-adapters` (data + `LayeredPrices` + `dirs`, no network) + `btctax-core` (`IncomeRecord.pseudo` + the
pseudo FMV `PseudoDefault` + render taint) + `btctax-cli` (session wiring + `PseudoKind` + the pointer string,
**no ureq**) + **NEW `btctax-update-prices`** (ureq). Workspace **8 → 9 members** (8 publishable + xtask). **[R0-M3]
current versions are 0.2.0 → target 0.3.0**; the new crate is a FIRST-time publish (the new-crate 5-burst rate
limit applies — [[crate-publishing-state]]). Docs: new `btctax-update-prices` man page (clap doc-comment →
regenerate); README (offline-by-default, the separate updater, BSD-2 attribution, the cache-as-local-input
note); FOLLOWUPS. Note: this touches the R0-GREEN'd `SPEC_pseudo_reconcile_mode.md` contract [I1] — record it.

## Plan (TDD; phased — a phase may stop-at-green if budget-bound)
- **T1 (A)** — convert + commit the dataset + the NOTICE; **the C1 test-migration + provider-injection seam**;
  the A KATs + the committed vault-income fixture [M5]; confirm the (now migrated) suite green.
- **T2 (B)** — `IncomeRecord.pseudo` + the pseudo income-FMV `PseudoDefault` (resolve injection + taint + render
  + `PseudoKind` + approve→`ManualFmv`); the B KATs + the ★ fault-inject; the "27 clear under pseudo" fixture gate.
- **T3 (C)** — `LayeredPrices` + cache wiring (`dirs`, no ureq) + the pointer; the NEW `btctax-update-prices`
  crate (ureq + fetch) + the dep-tree isolation check + canned-JSON KATs; man page + README; whole-diff + full
  suite + FOLLOWUPS.

## Gotchas
- **[C1] the stub swap breaks ~50 test assertions** — migrate via a provider-injection seam (synthetic prices),
  not by chasing recomputed constants; the plan owns it.
- **[I1] A-alone can't fix the 27** (FMV fixed at import; ingest idempotent) — B is required; and B REVERSES a
  sub-2 decision — state it + update the "0 blockers" contract.
- **[I2] the pseudo taint MUST reach `IncomeRecord` + its render** — else an unflagged pseudo dollar figure (★ guard).
- **[I3] the cache is a documented LOCAL INPUT** — bundled-only is the reproducible baseline; pseudo FMV is flagged.
- **[C] the tax binaries link NO HTTP client** (cargo-tree check); network ONLY in btctax-update-prices; canned-JSON tests.
- **[B] no price ⇒ stay blocked** (fault-inject); **[I4] separate NOTICE file** (CSV header comment breaks the parser).
- Deterministic; `Decimal` 2dp; conservation unaffected.
