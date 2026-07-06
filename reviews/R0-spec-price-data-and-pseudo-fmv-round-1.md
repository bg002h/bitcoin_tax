# R0 — SPEC_price_data_and_pseudo_fmv — round 1

**Artifact:** `design/SPEC_price_data_and_pseudo_fmv.md` (DRAFT).
**Baseline reviewed against:** branch `feat/price-data-fmv` @ `4b232c0` (spec commit); source verified against
this tree (main == `019ed3f`). Read-only architect review; no implementation performed.
**Bar:** 0 Critical / 0 Important.

## Verdict: **BLOCKED — 1 Critical / 4 Important / 5 Minor / 3 Nit**

Not R0-GREEN. The Part-A test blast radius is materially larger than the spec states (the "existing price
KATs stay green" claim is false, and the T1 gate as written is unachievable), and Part B silently reverses a
prior R0-GREEN'd scope boundary while leaving a real on-screen taint gap for the recognized-income row. The
mechanism seams for B and C are sound and the network isolation is architecturally clean — the blockers are
about scope honesty, the pseudo trust-contract, and taint completeness, not about the core approach.

---

## CRITICAL

### C1 — Part A: the stub-replacement test blast radius is understated; "existing price KATs stay green" is FALSE, and the T1 gate is unachievable as written

**Spec claims (must be corrected):** line 38 "existing price KATs stay green"; line 81 (T1) "confirm existing
price KATs green."

**Evidence — the stub (`crates/btctax-adapters/data/btc_usd_daily_close.csv`) has exactly 6 data rows:**
`2024-01-15=42500.00, 2024-02-01=43100.50, 2025-01-10=91000.00, 2025-03-01=84000.00, 2025-03-02=84250.25,
2025-06-15=67500.00`. Tests hard-code these in **two** fragile ways, BOTH broken by the real 2010→2026-06-03
dataset:

**(a) Exact FMV values computed from stub prices** (real close ≠ stub close ⇒ value assertions fail):
- `crates/btctax-adapters/tests/river.rs:19,54-56` — `BundledPrices::load()`; asserts income `usd_fmv == "6.75"`
  and `PriceDataset` from `2025-06-15 @ 67500`. Value break.
- `crates/btctax-adapters/tests/fmv_fr3.rs:22,41,48,57-61` — `ingest_files_bundled`; asserts `usd_fmv ==
  dec!(6.75)` from `2025-06-15 @ 67500`, **and** `price_dataset_count == 1` / `missing_count == 1`. Both the
  value and the counts break (see (b)).
- `crates/btctax-cli/tests/reconcile.rs` — the `sti_fixture` (line 1946) + its siblings bake
  `2025-03-01 → $84.00` (100_000 sat @ stub 84000) and derive `$33.75`, `$27.00` floors from stub prices;
  asserted at e.g. `1815` (`dec!(84.00)+dec!(33.75)`), `2168` (`dec!(84.00)`), `2338/2342`
  (`$84.00 + $27.00`), `2461`, `2600/2602` (`bulk_income_apply_sets_autofmv` — `decision fmv == fmv_of ==
  $84.00`), `2614` (`projected Income.usd_fmv == 84.00`). `grep -c '84\.00|2025-04-01|missing_price_count|
  excluded_missing_price' reconcile.rs` = **48**. All the `$84.00`/derived-floor pins break.
- `crates/btctax-cli/tests/reclassify_income_cli.rs:22-26` uses `2025-03-01` but only tests `--business`
  arg-parse (no FMV value asserted) → SAFE; the `$84,000` comment merely goes stale (see N2).

**(b) Stub-ABSENCE assumptions** (a date deliberately picked because the stub lacks it — the real dataset now
covers it, so the "no bundled price" scenario ceases to exist):
- `crates/btctax-adapters/tests/fmv_fr3.rs:6-14` — treats `2025-07-04` as a NON-dataset date (→ `Missing`).
  Real dataset covers 2025-07-04 ⇒ it resolves to `PriceDataset`; `missing_count` → 0, `price_dataset_count`
  → 2. Whole test premise destroyed.
- `crates/btctax-cli/tests/optimize_consult.rs:406-453` (`consult_future_date_requires_proceeds`) —
  comment line 408 "The bundled dataset's last entry is 2025-06-15. `at = 2025-12-31` has no dataset price";
  line 422-431 `.unwrap_err()` on the `--fmv` path expecting `ProceedsRequired`. Real dataset covers
  2025-12-31 ⇒ the FMV path SUCCEEDS ⇒ `unwrap_err()` panics. Break.
- `crates/btctax-cli/tests/reconcile.rs:1606,1811,1943,2154,2164,2316,2332,2437,2445` — `2025-04-01` is the
  fixtures' "UNPRICED" date driving `missing_price_count`/`excluded_missing_price`. Real dataset covers
  2025-04-01 ⇒ those rows become priced ⇒ counts/floors/`included` lengths all shift. Multiple bulk tests
  break (`bulk_sti_plan_fmv_floor_when_price_missing`, `bulk_income_plan_excludes_missing_price`, the outflow
  plan test at ~1807, and the Σ-income test at ~2316-2342).
- `crates/btctax-tui-edit/src/main.rs:11324-11362` picks `1990-01-01` (pre-2010) → STILL outside the real
  range → SAFE. Good precedent for the migration.

**Architectural wrinkle that makes the migration harder (must be addressed):** the bulk plans and `project()`
**hard-wire** `BundledPrices::load()` — `session.rs:449` (`project`), and the bulk `fmv_of(&prices, …)` plans
consume that same `prices` (session.rs:52,100,142,318). These tests **cannot inject a synthetic
`StaticPrices` provider**; they exercise the real bundled data. So the "unpriced date" scenarios can only be
re-homed to a date genuinely outside the real range — i.e. **> 2026-06-03** (future) or pre-2010 — and a
future date is itself fragile to the next `update-prices`/data refresh.

**Not affected (confirmed, so the spec can scope precisely):**
- `crates/btctax-adapters/src/normalize.rs:88 fn prices()` is an in-memory `StaticPrices` double
  (`m.insert(2025-03-01, 84000.00)`), NOT the bundled CSV → independent of the swap. SAFE. (The prompt's
  specific worry is answered: it does not depend on stub values.)
- `btctax-adapters/src/price.rs` unit tests (lines 59-80) use `from_csv_str` with inline synthetic data, not
  `DATASET_CSV` → SAFE.
- `optimize_mode2.rs` / most core tests use `StaticPrices::default()` (empty) → SAFE.
- `integration.rs:99` (River interest @ 2025-06-15) asserts event existence/kind only, no FMV value → SAFE.
- Exchange-provided-FMV CSV tests (coinbase/gemini/swan, and Sell rows carrying `Price at Transaction`) →
  `ExchangeProvided`, independent of the dataset → SAFE.

**Why Critical:** the spec's plan rests on a false premise ("existing price KATs stay green") and omits a
tax-touching migration that spans ≥4 test files and dozens of assertions; T1's stop-at-green gate cannot be
met as written.

**Fix:**
1. Correct lines 38 & 81 — existing price KATs will NOT stay green; enumerate the migration as an explicit
   T1 deliverable.
2. Choose and state a strategy per test: **(i)** re-pin exact-FMV assertions to the real dataset's actual
   closes for those dates (and lock at least one with a KAT, see M4); **(ii)** move every "no bundled price"
   scenario to a date provably outside the real range and add a `bundled_dataset_max_date` guard KAT so a
   future data refresh that extends coverage fails loudly rather than silently changing behavior; **(iii)**
   for the fragile bulk-plan tests, consider refactoring the plan API to accept an injected `&dyn
   PriceProvider` (decoupling from `BundledPrices::load()`) so they can use `StaticPrices` — a larger change,
   flag as a scope decision.
3. Add a `T0` "migrate the impacted tests" step BEFORE the data lands (red→green), or land the data and the
   test migration in the same commit.

---

## IMPORTANT

### I1 — Part B silently reverses the prior R0-GREEN'd pseudo-spec decision to NOT clear native-Income `FmvMissing`, and does not update the "0 blockers" contract

`SPEC_pseudo_reconcile_mode.md` (R0-GREEN, 3 rounds, 0C/0I) **explicitly excludes** native-Income FmvMissing
from pseudo clearing: line 20 "It does NOT clear, and leaves SURFACED: … native-`Income` `FmvMissing` (pseudo
defaults only inbound TransferIns)"; line 107 "**… native-Income `FmvMissing` … are NOT cleared** — surfaced".
Part B reverses exactly this, but the new spec never mentions the prior boundary or the `[R0-I2]` "what 0
blockers means" contract it amends.

The reversal is *defensible* — the prior exclusion was implicitly because there was no honest price source;
Part A now supplies one (the daily close), and the "no price ⇒ stay blocked" fault-inject
(`pseudo_fmv_absent_when_no_price`, spec line 52-53) preserves the honesty principle. But the spec must say so.

The rationale is also verified sound at the resolve layer: `build_op`'s Income arm has **no** price fallback —
`fmv = manual_fmv.get(id) OR x.usd_fmv.filter(|_| status != Missing)` (`resolve.rs:282-299`), no `fmv_of` call.
And import is idempotent — `cmd/import.rs:2` "Idempotency + ImportConflict detection are core's job
(`append_import_batch`)" — so a re-import will NOT re-resolve an already-stored `Missing` income event. That
is genuinely *why* A alone cannot clear the stored-Missing events (they were imported when the price was
absent; ingest-time `resolve_fmv`, `normalize.rs:10-23`, would have filled them as `PriceDataset` had the price
existed then). State this chain explicitly — it is the load-bearing justification for Part B existing at all.

**Fix:** add a paragraph that (a) names the prior spec's exclusion and declares this an intentional amendment
now that a price source exists; (b) updates the `[R0-I2]`/"0 Hard blockers" enumeration so native-Income
FmvMissing is "cleared under pseudo iff a price exists, else surfaced"; (c) records the ingest-idempotency
chain as the reason re-import doesn't fix the 27.

### I2 — Pseudo income-FMV taint does not reach the `IncomeRecord` / Income-tab row (a clean-number leak)

The pseudo taint currently rides `Lot`/`Consumed`/`DisposalLeg`/`PendingLeg`/held-lot rows
(SPEC_pseudo_reconcile_mode.md lines 35-39). But **`IncomeRecord` has no `pseudo` field** — `state.rs:211-218`
(`event, recognized_at, sat, usd_fmv, kind, business`), and the Income-tab render iterates
`state.income_recognized` (`render.rs:299-312`) with no `[PSEUDO]` marker path. In `fold.rs` the Income arm
taints the income **lot** (`fold.rs:722 pseudo: ev_pseudo`) but pushes the `IncomeRecord` (`fold.rs:689-696`)
with no pseudo flag. So a pseudo-synthesized native income would recognize a **clean ordinary-income dollar
figure** on the Income tab while only the derived lot is flagged on Holdings — exactly the leak the pseudo
design's headline `[★]` guard forbids (SPEC_pseudo_reconcile_mode.md line 77: `[PSEUDO]` on-screen, provably
absent from exports). This surface is newly exposed by Part B (today no `IncomeRecord` is ever pseudo, because
`Op::IncomeInbound` bulk-classify produces REAL decisions, not synthetics).

**Fix:** thread `ev_pseudo` into `IncomeRecord` (new `pseudo: bool`, omitted by CSV/form writers per the
dedicated-bool discipline `render.rs:56-63`), mark the Income-tab row via `pseudo_marker`, and add a KAT:
pseudo income row carries `[PSEUDO]` on-screen AND is clean in every export CSV/form (mirror the C1 basis-taint
KAT). List the render/state touch-points in the plan.

### I3 — Cache-derived FMVs enter the ledger as REAL, unflagged, and non-reproducible (NFR4 / audit provenance)

Part C's cache extends the price provider, but a cache-only date's FMV flows through the **normal** paths as a
REAL value: native-income ingest resolves it as `FmvStatus::PriceDataset` (`normalize.rs:19-21`) and disposal
proceeds fall back to it (`evaluate.rs:93,129`) — **not** `[PSEUDO]`-flagged. Yet that value is **not
reproducible** from the published bundled baseline: two machines with different caches produce different
"authoritative" tax numbers for the same events, and an auditor with only the crate cannot reproduce them. The
spec's determinism note (lines 95-96, "bundled = reproducible baseline; cache extends") acknowledges divergence
but not that cache FMVs become **real, unflagged, ledger-affecting** numbers. NFR4 ("identical (events,
prices) → identical ledger") holds only per-provider once the provider is no longer canonical.

Mitigating (state it): the cache only matters for dates **outside** the bundled range (> 2026-06-03), i.e.
recent/in-progress periods that are estimates anyway; any past tax year fully within the bundled range stays
byte-reproducible.

**Fix:** either (a) give cache-sourced FMVs a distinct provenance (`FmvStatus::PriceCache`, surfaced/markable)
so they are visibly non-canonical, or (b) explicitly document that cache-derived FMVs are for
not-yet-filable/in-progress estimation and that final filing must rest on bundled data or explicit user FMV —
and bound the determinism claim to "per fixed provider; bundled data is the canonical reproducible provider."

### I4 — The "header comment in the CSV" license option is incompatible with the parser

Spec line 33 offers "a `data/BitcoinPricesDaily.LICENSE` (or **header comment + a NOTICE**)". The `+ header
comment` path breaks parsing: `BundledPrices::from_csv_str` (`price.rs:28-41`) skips only blank lines and the
`i==0` line starting with `date`; **any** other line is parsed as `date,usd_close` and a `#`/copyright line
fails with `PriceDataset("line N: bad date …")`.

**Fix:** use the **separate-file** attribution only (`data/*.LICENSE`/`NOTICE` + a `//!` header comment in
`price.rs`), OR extend the parser to skip `#`-prefixed comment lines (and add a KAT for a commented dataset).
Packaging is fine for the separate-file route: `crates/btctax-adapters/Cargo.toml` has no `include`/`exclude`,
so committed files under the crate (incl. `data/*`) are published — but state this explicitly and add the
attribution file to the T1 deliverables.

---

## MINOR

### M1 — `SetFmv` is not a core payload; the FMV decision is `ManualFmv`
Spec lines 48,51 say approve "promotes to a real `SetFmv`". There is no `SetFmv` event payload; the core payload
is `ManualFmv` (`event.rs:157,313`), surfaced by the CLI `reconcile set-fmv` subcommand (`cli.rs:311`,
`main.rs:590` → `cmd::reconcile::set_fmv`). Good news: `PseudoDefault.decision` is already a generic
`EventPayload` and `apply_bulk_pseudo_approve` persists it verbatim (`reconcile.rs:283-292`), so a
`PseudoDefault { decision: EventPayload::ManualFmv{…}, kind: PseudoKind::IncomeFmv }` needs **no new approve
arm**. But a **new `PseudoKind` variant** is needed plus its CLI plumbing: `PseudoKindArg` (`cli.rs:638`), the
`main.rs` mappings (`1174-1178`, `1235-1237`), the render label, and the `filtered_pseudo_plan` filter. Rename
"SetFmv" → "a `ManualFmv` decision (CLI `set-fmv`)" and enumerate these touch-points in the plan.

### M2 — `dirs` is not currently a dependency; specify where it lives
No crate depends on `dirs` (verified: no `dirs` in any `Cargo.toml`). Part C assumes `dirs::data_dir()`
(spec line 60). Add `dirs` and state the layering: resolve the cache path in **btctax-cli** (where `dirs`
lives) and pass it into `btctax-adapters` `LayeredPrices::load_with_cache(path)`, keeping path-policy out of
core/adapters and making the cache temp-path-testable (the canned-JSON KATs need an injectable path).

### M3 — Version/scope: crates are at 0.2.0, and there are 8 workspace members
Workspace crates are `version = "0.2.0"` (e.g. `btctax-adapters/Cargo.toml`), not 0.1.0 — a MINOR bump →
**0.3.0**. `Cargo.toml members` lists **8** crates incl. `xtask` (not published); "all 7 crates bump" (spec
line 78) means the 7 published crates. State the concrete target version and that `xtask` is excluded.

### M4 — Precision: `fmv_of` already rounds to cents; 2dp source is fine but pin one real-date FMV
`fmv_of` applies `round_cents` to the FINAL product (`price.rs:13-18`), so 2dp source precision is defensible
(sub-cent source granularity is invisible after rounding for realistic sat quantities). After the C1 migration
no KAT needs sub-cent source precision. Document the 2dp decision AND add one KAT pinning a real-date FMV
(date + real close + sat → exact cents) so the round/precision choice is locked and a future re-fetch at
different precision fails loudly.

### M5 — The "27 income events" and the dataset facts are not verifiable from committed source
No committed test asserts the "27" (grep found none), and the real dataset is not yet in-repo (only the stub).
So T2's "re-verify the 27 clear under pseudo" (spec line 83) is not a reproducible gate. Cite the source of the
27 (the local real vault), and add a **committed** fixture reproducing an N-income `FmvMissing` scenario that
clears under pseudo, so T2 has a real gate. Likewise state that `5,802 rows / 2010-07-17→2026-06-03` will be
asserted by `bundled_dataset_row_count` / `bundled_dataset_covers_*` (spec line 37-38) — good, but note the row
count must be a `>=` or exact-with-refresh-guard so `update-prices`-style refreshes of the *bundled* file don't
silently drift the KAT.

---

## NIT

- **N1** — "7-row STUB" (spec lines 8, 40): the file is **6 data rows** (7 lines incl. the `date,usd_close`
  header). Say "6 rows / 7 lines" for precision.
- **N2** — `reclassify_income_cli.rs:22` comment "`$84,000 → PriceDataset`" goes stale after the swap (2025-03-01
  stays in-range so the test remains SAFE, but the price differs). Refresh the comment during migration.
- **N3** — Scope granularity (spec asks): one combined spec is workable, but A is a large mechanical test
  migration (C1), B is a core tax-semantics change amending a prior spec (I1/I2), and C adds the first network
  dep. Consider splitting so B's tax review and the pseudo-contract amendment aren't gated behind A's churn;
  at minimum keep A's migration on its own commit/gate. Judgment call — noted, not required.

---

## Confirmations (verified sound — no change needed)

- **B mechanism fit:** the `PseudoDefault`/`Eff.pseudo` seam CAN carry a synthetic income FMV. Injecting into
  the `manual_fmv` map (built pass 1d, `resolve.rs:565-610`) during the pseudo phase 1f (`resolve.rs:929-992`,
  runs BEFORE the pass-2 timeline build) flows through `build_op`'s Income arm (`resolve.rs:283-299`) and sets
  `pseudo_ids` for the `Eff.pseudo` taint (`resolve.rs:1025`). `prices` is already a `resolve` parameter
  (`resolve.rs:401`), and the income date/sat are available from the `LedgerEvent`. **Resolve is the right
  layer** (matches the existing pseudo seam); do NOT push this into fold. The "no price ⇒ no synthetic ⇒ stays
  blocked" fault-inject is the correct honesty guard.
- **Native income truly has no resolve-layer price fallback** (`resolve.rs:282-299`), while disposal proceeds
  DO — but only on the **synthetic consult/optimize path** (`evaluate.rs:91-97,127-131`); native stored
  disposals carry their own `usd_proceeds`. The spec's B-rationale should scope the "disposal proceeds DO"
  claim to the synthetic path so it doesn't over-imply a general disposal price fallback.
- **Tax soundness of pseudo income at the daily close:** defensible and consistent. Recognizing ordinary income
  at FMV is the conservative (higher-tax, IRS-safe) direction; the daily close is the app's uniform FMV
  convention (`PriceDataset` at ingest, disposal proceeds, and bulk-inbound auto-FMV all use it —
  `reconcile.rs:464,483` confirms bulk-classify-inbound-income auto-values via `fmv_of(date, sat)`). The only
  asymmetry vs bulk-inbound is the **trust flag** (`[PSEUDO]` vs a real classify decision) — reasonable because
  pseudo is opt-in and promotable via approve; call it out per I1.
- **C network isolation:** no HTTP client exists today (no `ureq`/`reqwest`/`hyper`/`tokio`/`rustls` in any
  `Cargo.toml`) — `ureq` (blocking, rustls-tls) in **btctax-cli** is a sound first network dep; rustls avoids
  the system-OpenSSL cross-platform trap (consistent with the cross-platform-CI matrix). btctax-core has no
  network dep and the layered provider (a local-file `LayeredPrices` in btctax-adapters + cache reads) keeps
  the projection offline/deterministic — the isolation is architecturally clean and testable via a
  no-network-dep-in-core assertion. Canned-JSON fixtures with an `#[ignore]` live smoke test is the right
  approach. Security posture (TLS verify via rustls, no secrets, a User-Agent, timeouts, graceful offline
  error, opt-in privacy) is adequate as specified — keep the User-Agent/timeout/verify requirements as
  explicit acceptance criteria.

---

### Suggested fold order
C1 (scope-honesty + migration plan) and I1/I2 (pseudo contract + income taint) are the substantive folds; I3/I4
tighten determinism/provenance and license mechanics; M/N are wording/plan-completeness. Re-review after the
fold (including the last), per §2.
