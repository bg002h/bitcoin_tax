# SPEC — comprehensive price data + pseudo-FMV + a separate online updater

**Source baseline:** `main` @ `019ed3f` (branch `feat/price-data-fmv`). **Review status: R0-GREEN (3 rounds; 0C/0I).
Cleared to implement.** Reviews: `reviews/R0-spec-price-data-and-pseudo-fmv-round-{1,2,3}.md`. r1 1C/4I, r2
0C/2I (both = understated cross-crate blast radius), r3 0C/0I/3M/2N (M-D the kat_rate_engine construction site;
M-E/M-F/N-A/N-B opportunistic during T1/T2). Part C = a SEPARATE `btctax-update-prices` binary (user-directed
2026-07-05, so the tax binaries have zero network deps). Tax-critical;
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
  klines primary 8-dp; CoinGecko fallback; `--lag 8` settle). **The prices are public MARKET FACTS originally
  sourced from Binance/CoinGecko — factual data is not copyrightable, so NO license/attribution attaches** (the
  Quantoshi repo's BSD-2 covers its CODE, not these facts). Freely bundlable.
- btctax format (price.rs:8-45): `date,usd_close`, ISO, `Decimal`; `BundledPrices` = `BTreeMap<TaxDate,Usd>`,
  EXACT-date lookup. `fmv_of` already `round_cents` [R0-M4] → **2dp source is fine.** Pure/deterministic (NFR4).
- Wiring: session.rs:449-450 `BundledPrices::load()` (hard-wired — see [R0-C1] seam). No HTTP client exists.

## Part A — bundle the dataset + [★ R0-C1] the test-migration (the Critical)
- Convert once (committed data file): `M/D/YY → YYYY-MM-DD`; price → `Decimal` **rounded to 2dp cents** [M4];
  header `date,usd_close`; sorted asc, one row/day, deduped. `from_csv_str` parses it unchanged.
- **No attribution file needed** (user-corrected 2026-07-05): the data are public daily-close market FACTS
  (Binance/CoinGecko-sourced) — facts aren't copyrightable, so NO BSD-2/NOTICE. A one-line provenance note
  ("daily closes derived from public Binance/CoinGecko market data") may go in the README — **NOT** in the CSV
  ([R0-I4] the parser errors on any non-`date` line, price.rs:28-41). Packaging is fine (no include/exclude).
- **[R0-C1] Migrate the tests broken by real data (the plan OWNS this — "existing KATs stay green" was FALSE):**
  - *Exact-FMV pins computed from stub prices* — `adapters/tests/river.rs:54-56` ($6.75←67500),
    `adapters/tests/fmv_fr3.rs:57-61`, `cli/tests/reconcile.rs` `sti_fixture` ($84.00←84000@2025-03-01 + the
    $33.75/$27.00 floors, ~48 refs at ~1815/2168/2338/2461/2600/2614).
  - *"No bundled price" assumptions now COVERED by real data* — `2025-07-04` (fmv_fr3.rs), `2025-12-31`
    (`optimize_consult.rs:406-453`), `2025-04-01` (reconcile.rs `missing_price_count`/`excluded_missing_price`,
    ~1811/2164/2332/2445).
  - **[R0-r2 I-A] A THIRD crate also breaks:** `btctax-tui-edit/src/main.rs` `seed_income_inbounds` bulk-income
    KAT (~21063-21266) — BOTH modes at once: exact-FMV pins `$84.00`/`$33.75`/`$117.75` (from stub 84000/67500)
    AND the `2025-04-01 → excluded_missing_price==1` sentinel (real data covers it ⇒ 2→3). It routes through
    `Session::bulk_classify_income_plan` (session.rs:771/776).
  - **Approach — a Session-LEVEL injectable provider (not a free fn) [R0-r2 I-A]:** core `project()` ALREADY
    takes `&dyn PriceProvider` (mod.rs:62/64), so the seam is feasible — but the hard-wire is at the CLI
    `Session` layer across **~15 `BundledPrices::load()` sites** (incl. session.rs:449/771/776). Add an
    instance-level provider on `Session` (a `#[cfg(test)]`/`pub(crate)` constructor accepting `&dyn
    PriceProvider`, defaulting to the layered bundled+cache) so ALL these tests inject a CONTROLLED synthetic
    `from_csv_str(...)` — decoupled + refresh-proof. Recompute expected values only where a test genuinely
    asserts bundled coverage; a >2026-06-03 unpriced date is the fallback ONLY where injection is infeasible
    (mark refresh-fragile).
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
- **[R0-I2 + r2 I-B] Taint MUST reach the income row (the ★ guard):** `IncomeRecord` (state.rs:211-218) has NO
  `pseudo` field. Add `pub pseudo: bool`; set it from `eff.pseudo` at **BOTH push sites — fold.rs:689 (native
  `Op::Income`) AND fold.rs:877 (`Op::IncomeInbound`)** [R0-r2 I-B]; mirroring the Lot taint (fold.rs:722).
  - **CLI report** — flag via the real helper **`pseudo_tag` (render.rs:61, used at 239/353/365)** [R0-r2 I-B]
    (there is no `pseudo_marker` symbol [R0-r3 N-A]). This is the primary tax-output guard surface.
  - **[R0-r3 M-D] Also update `btctax-adapters/tests/kat_rate_engine.rs:167/190`** — it constructs `IncomeRecord`
    and needs a compiler-forced `pseudo: false` (no stub coupling, no recompute; caught by full-suite-green).
  - **TUI** — `Lot`/`DisposalLeg` already carry `pseudo` (holdings.rs:223/disposals.rs:254) and the TUI surfaces
    pseudo via the **mode BANNER** (draw_edit.rs:126 "[PSEUDO] rows are placeholders"), not per-row marks. So:
    thread the new `IncomeRecord.pseudo` through `btctax-tui` + `btctax-tui-edit` income construction (~11
    fixture sites + the projection) so they COMPILE, and surface income pseudo the SAME way the TUI already
    surfaces pseudo lots/disposals (the banner + the field). If the TUI viewer lacks any pseudo-row convention,
    a per-row TUI marker for ALL row types is a pre-existing sub-2 gap → FOLLOWUP, NOT #41's scope to retrofit.
  Without the field, a pseudo income shows a CLEAN dollar figure (guard violation).
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
  Cache = `date,usd_close` at `dirs::data_dir()/btctax/price_cache.csv` (a `--price-cache`/env override). Cache
  absent → byte-identical to bundled-only. **[R0-M2/r2 M-A] `dirs` is NOT a dep today; put it in `btctax-cli`
  AND `btctax-update-prices` (each resolves the default path) — NOT in `btctax-adapters`**, which takes an
  already-resolved `cache_path: Option<&Path>` and stays a pure format/provider crate (no `dirs`, no network).
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
  touches bundled); summary + `--dry-run`. User-Agent; timeouts; graceful offline error. Direct deps:
  btctax-adapters (format/path) + `dirs` + ureq ONLY — **no DIRECT dep on btctax-core/cli** [R0-r2 M-C]
  (btctax-core arrives TRANSITIVELY via adapters — fine: core is itself network-free).
- **btctax-cli stays network-free:** the "no price for {date}" surfaces (the B no-price case + the FmvMissing
  hint) print `run \`btctax-update-prices\`` — a STRING, no dep, no shell-out.
- **[★] Verifiable isolation check [R0-r2 M-B]:** an **xtask/CI step** (NOT a non-hermetic `#[test]`) asserting
  `ureq`/rustls is absent from `cargo tree` of btctax-cli / -tui / -tui-edit / -core / -adapters (present ONLY
  in btctax-update-prices).
- KATs: `layered_prices_cache_over_bundled`, `cache_absent_is_bundled_only` (btctax-adapters);
  `update_prices_dry_run_writes_nothing`, `update_prices_appends_and_is_idempotent` (CANNED Binance/CoinGecko
  JSON fixtures — NO live network), `update_prices_respects_lag` (btctax-update-prices). Live-network smoke =
  `#[ignore]`.

## Scope / SemVer / lockstep
`btctax-adapters` (data + `LayeredPrices` taking a `cache_path`, **no `dirs`**, no network) + `btctax-core`
(`IncomeRecord.pseudo` at BOTH push sites + the pseudo FMV `PseudoDefault`) + `btctax-cli` (Session-level
provider seam + `dirs` cache-path + `PseudoKind` + `pseudo_tag` income flag + the pointer string, **no ureq**) +
**`btctax-tui` + `btctax-tui-edit`** (thread `IncomeRecord.pseudo` through income construction/render + the
`seed_income_inbounds` test migration) [R0-r2 I-A/I-B] + **NEW `btctax-update-prices`** (ureq + `dirs`). Workspace
**8 → 9 members** (8 publishable + xtask). **[R0-M3]
current versions are 0.2.0 → target 0.3.0**; the new crate is a FIRST-time publish (the new-crate 5-burst rate
limit applies — [[crate-publishing-state]]). Docs: new `btctax-update-prices` man page (clap doc-comment →
regenerate); README (offline-by-default, the separate updater, the price-data provenance note = public
Binance/CoinGecko market facts, the cache-as-local-input note); FOLLOWUPS. Note: this touches the R0-GREEN'd `SPEC_pseudo_reconcile_mode.md` contract [I1] — record it.

## Plan (TDD; phased — a phase may stop-at-green if budget-bound)
- **T1 (A)** — convert + commit the dataset (NO attribution file — public market facts); **the C1
  test-migration + provider-injection seam**;
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
- **[B] no price ⇒ stay blocked** (fault-inject); **the price data are public market FACTS — no
  BSD-2/attribution file**; any provenance note goes in the README, never a CSV header comment ([I4] breaks the parser).
- Deterministic; `Decimal` 2dp; conservation unaffected.
