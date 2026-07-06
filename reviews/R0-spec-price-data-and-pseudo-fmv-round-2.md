# R0 — SPEC_price_data_and_pseudo_fmv — round 2

**Artifact:** `design/SPEC_price_data_and_pseudo_fmv.md` (round-1 folded IN-PLACE + Part C rewritten to a
SEPARATE `btctax-update-prices` binary).
**Baseline reviewed against:** branch `feat/price-data-fmv` @ `8d2bf53` (spec commit); source verified against
this tree (main == `019ed3f`). Read-only architect review; no implementation performed.
**Round-1:** `reviews/R0-spec-price-data-and-pseudo-fmv-round-1.md` (BLOCKED — 1C/4I/5M/3N).
**Bar:** 0 Critical / 0 Important.

## Verdict: **BLOCKED — 0 Critical / 2 Important / 3 Minor / 2 Nit**

Not R0-GREEN. The round-1 Critical (C1) and the four Importants (I1–I4) are **materially improved and mostly
resolved** — every load-bearing claim I re-checked against source holds (see the fold-verification table). But
two residual Importants remain, both of the *same class the round-1 review flagged*: the cross-crate blast
radius is still understated. Specifically (I-A) the C1 test-migration enumeration misses an entire **third**
crate whose bulk-income KAT breaks in **both** failure modes and rides a Session path the proposed seam does
not reach; and (I-B) the I2 income-taint fix is under-specified on its cross-crate render/scope surface —
there are **two** `IncomeRecord` push sites, the named render helper is the wrong (unused) one, and
`btctax-tui` (11 `IncomeRecord` construction sites + the income-tab render) is absent from the
Scope/SemVer/lockstep list, leaving the on-screen `[PSEUDO]` guard undecided on the TUI income surface. Both are
tighten-the-spec folds, not design reversals — the mechanism seams (B into resolve, C as a separate ureq crate)
remain sound and the network isolation is architecturally clean.

---

## Fold verification (round-1 findings re-checked against `8d2bf53` source)

| R1 | Claim to verify | Verified? | Evidence |
|----|-----------------|-----------|----------|
| **C1** | stub = 6 data rows; the enumerated exact-FMV pins + "no-price" dates really break | **YES (partial — see I-A)** | `crates/btctax-adapters/data/btc_usd_daily_close.csv` = 6 rows (42500/43100.50/91000/84000/84250.25/67500). Pins confirmed: `river.rs:55-56` (`"6.75"`←67500), `fmv_fr3.rs:52,59-60` (`dec!(6.75)`), `reconcile.rs` `$84.00`/`$33.75`/`$27.00` (1807/1815/2168/2338/2342/2461/2600-2625/3254-3279). "No-price" dates confirmed: `optimize_consult.rs:406-453` (2025-12-31, `.unwrap_err()` on `ProceedsRequired`), `fmv_fr3.rs:6-14` (2025-07-04), `reconcile.rs` 2025-04-01 (`excluded_missing_price`). |
| **C1 seam** | is `project()`/session able to take `&dyn PriceProvider`, or is `load()` hard-wired? | **YES — seam feasible but wider than stated (I-A)** | Core `project()` **already** takes `prices: &dyn PriceProvider` (`btctax-core/src/project/mod.rs:62,64`). The hard-wire is only at the **cli Session** layer: `session.rs:449` (`project`), plus **~14 more** `BundledPrices::load()` sites in `session.rs` (463/479/514/554/589/678/**776**/886/1046) and `cmd/reconcile.rs` (240/271/478/552) + `cmd/optimize.rs` + `ingest.rs:29`. A single `project_with_prices` free-fn does **not** cover the Session bulk-plan methods. |
| **I1** | `SPEC_pseudo_reconcile_mode.md` 20/107 left native-income FmvMissing uncleared; ingest idempotent; spec states reversal + updates contract | **YES** | `SPEC_pseudo_reconcile_mode.md:20` ("native-`Income` `FmvMissing` … pseudo defaults only inbound TransferIns"), `:107` ("native-Income `FmvMissing` … are NOT cleared — surfaced"). `import.rs:1-3` ("Idempotency … core's job (`append_import_batch`)"). `normalize.rs:10-23` `resolve_fmv` fixes FMV at ingest (export→ExchangeProvided; else `fmv_of`→PriceDataset; else Missing). `resolve.rs:282-299` Income arm has **no** `fmv_of` fallback. Spec §Part B lines 58-61 now name the reversal + amend the "0 blockers" contract. **Fold sound.** |
| **I2** | `IncomeRecord` has no `pseudo`; `fold.rs` pushes unflagged; income render has no marker; add-bool+render+KAT is the complete fix | **PARTIAL (see I-B)** | `state.rs:210-218` — `IncomeRecord{event,recognized_at,sat,usd_fmv,kind,business}`, no `pseudo`. `fold.rs:689-696` push unflagged (native `Op::Income`); lot tainted at `fold.rs:723` (`pseudo: ev_pseudo`). CLI income render `render.rs:300-317` shows only ` [business]`, no pseudo. **But the fix enumeration is incomplete — I-B.** |
| **I3** | cache-as-local-input framing sound | **YES** | Spec lines 88-94 treat the cache as a documented LOCAL INPUT; bundled-only = published-reproducible baseline; pseudo income separately `[PSEUDO]`-flagged. Consistent with `evaluate.rs:93,129` disposal-proceeds path being the unflagged consumer. Sound. |
| **M1** | `ManualFmv` is the real payload; `PseudoKind` wiring correct | **YES** | `event.rs:158-161` `struct ManualFmv{event,usd_fmv}` (no `SetFmv`). `cli.rs:641-648` `PseudoKindArg` = {SelfTransfer, Raw, Conflict} — a new `IncomeFmv` variant is genuinely needed. |
| **M2** | `dirs` genuinely absent | **YES (but a residual contradiction — M-A)** | No `dirs` in any `Cargo.toml`. Spec line 87 correctly puts it in the cli; **line 111 contradicts** (puts it in adapters). |
| **M4** | 2dp OK (`fmv_of` `round_cents`) | **YES** | `btctax-core/src/price.rs:13-19` — `fmv_of` maps `round_cents` over the final product; 2dp source is invisible after rounding. |
| **M5** | committed-fixture gate sensible | **YES** | Spec adds a committed vault-income fixture ([M5], lines 55/79/123) so "27 clear under pseudo" becomes a real gate. Reasonable; the "27" itself remains from the local vault (not committed) — the fixture substitutes for it. |
| **I4** | separate NOTICE file (not CSV header) | **YES** | `price.rs:28-41` `from_csv_str` skips only blank + the `i==0` `date`-prefixed header; any `#`/© line errors. Spec line 35 mandates a separate `data/BitcoinPricesDaily.NOTICE`. Correct. |
| **M3 / Part C** | new crate deps adapters+ureq (not core/cli); cargo-tree isolation; 0.3.0; 9 members | **YES (2 precision notes — M-B/M-C)** | No HTTP client anywhere today (`ureq`/`reqwest`/`hyper`/`tokio`/`rustls` all absent). Current members = **8** (`btctax, -store, -core, -adapters, -cli, -tui, -tui-edit, xtask`); +update-prices → **9** = 8 publishable + xtask. ✓ All crates at `0.2.0` → `0.3.0`. ✓ |

---

## IMPORTANT

### I-A — Part A / C1: the test blast radius is STILL understated — `btctax-tui-edit` breaks (unlisted), and the proposed seam does not reach the Session bulk-plan path

The C1 fold correctly owns the migration and adopts a provider-injection seam — a real improvement. But the
prompt's own instruction ("re-grep the stub values … is the approach complete, or are there OTHER stub-coupled
tests not yet listed?") surfaces a concrete miss.

**A THIRD crate breaks and is unlisted.** `crates/btctax-tui-edit/src/main.rs` — the `seed_income_inbounds`
bulk-income KAT and its consumer (~`21063-21266`) assert:
- `i2 = 2025-06-15, 50_000 sat` → **`$33.75`** and `i1 = 2025-03-01, 100_000 sat` → **`$84.00`**, `m.count == 2`,
  `m.total_income_usd == dec!(117.75)` (`main.rs:21066,21258-21264`) — **exact-FMV pins** computed from stub
  closes (67500/84000). Real closes differ ⇒ **break**.
- `i3 = 2025-04-01, 40_000 sat (UNPRICED)` → `m.excluded_missing_price == 1` (`main.rs:21065,21265-21266`) —
  the **stub-ABSENCE** assumption. The real dataset covers 2025-04-01 ⇒ i3 becomes priced ⇒ `count → 3`,
  `total` shifts, `excluded_missing_price → 0`. **Break.**

This test breaks in **both** C1 failure modes at once, in a crate the migration scope (spec lines 38-52,
`btctax-adapters` + `btctax-cli` only) never mentions. It routes through `Session::bulk_classify_income_plan`
(`session.rs:771`, which hard-wires `BundledPrices::load()` at `session.rs:776`) via `App`/`handle_key`, so it
cannot be re-homed by re-pinning a free `project()` call.

**The seam as written is under-scoped.** Spec line 48 proposes "a `pub(crate)`/`#[cfg(test)]` constructor **or**
a `project_with_prices` seam." Verified: core `project()` **already** accepts `&dyn PriceProvider`
(`mod.rs:62,64`), so injection is feasible — but the hard-wire lives at the **cli Session** layer across **~15
`load()` sites** (`session.rs:449,463,479,514,554,589,678,776,886,1046`; `cmd/reconcile.rs:240,271,478,552`;
`cmd/optimize.rs`; `ingest.rs:29`). A `project_with_prices` free function covers only the single `project()`
path; the bulk-plan methods and the tui-edit App path need an **instance-level** injected provider on `Session`
(the "constructor" alternative), which is a substantially larger refactor than "add a constructor" implies — or
those Session-driven tests fall back to the re-pin/far-future strategy (which the tui-edit case forces).

**Why Important (not Critical):** T1's gate is "full suite green", which is self-correcting — the tui-edit
failures *would* be caught during implementation (so the gate is achievable, unlike round-1's C1). But the
enumeration + seam design silently understate the migration in exactly the way round-1 C1 called out, and an
implementer following the spec's seam would hit an un-covered Session path mid-T1.

**Fix:**
1. Add `crates/btctax-tui-edit/src/main.rs` (the `seed_income_inbounds` bulk-income KAT ~21063-21266:
   `$84.00`/`$33.75`/`$117.75` pins + the 2025-04-01 `excluded_missing_price` sentinel) to the C1 enumeration
   and the T1 deliverable.
2. State the seam is an **instance-level** provider on `Session` (covering the ~15 `load()` sites incl. the
   bulk-plan methods), not just a `project_with_prices` free fn; OR state which Session-driven tests use the
   re-pin/far-future fallback and mark them refresh-fragile.
3. Note that core `project()` already takes `&dyn PriceProvider` — so the seam is purely a cli-layer refactor.

### I-B — Part B / I2: the income-taint fix is under-specified on its cross-crate render + scope surface (the on-screen `[PSEUDO]` guard is left undecided on the TUI income tab)

The core of I2 is folded correctly (add `IncomeRecord.pseudo` + render + KAT). But the enumeration is incomplete
on three axes, and the third is a genuine open guard question, not tidiness:

1. **Two `IncomeRecord` push sites, not one.** `fold.rs:689-696` (native `Op::Income`) **and** `fold.rs:877-884`
   (`Op::IncomeInbound`, the bulk-classified path). Adding a non-`Default` `pub pseudo: bool` is compiler-forced
   at both; the plan (line 66) names only `:689-696`. State both and each one's value (`:877` = `ev_pseudo`
   for consistency; today always real).

2. **Wrong render helper named.** The CLI per-row pseudo helper is **`pseudo_tag`** (`render.rs:61`, applied to
   Lot at `:239` and legs at `:353,:365`). `pseudo_marker` (`render.rs:56`) that round-1 named is **defined but
   has zero call sites**. The income fix = add `pseudo_tag(i.pseudo)` at `render.rs:300-317`, mirroring `:239`.
   Name the right helper so the plan mirrors a real precedent.

3. **`btctax-tui` is absent from Scope/lockstep, yet it must change — and the TUI on-screen guard is undecided.**
   Spec line 68 says render the marker on "the sort-views Income render", which is
   `crates/btctax-tui/src/tabs/income.rs:77-96` (`sorted_income`). But the Scope/SemVer/lockstep paragraph
   (line 111) lists only `btctax-adapters + btctax-core + btctax-cli + btctax-update-prices` — **`btctax-tui`
   and `btctax-tui-edit` are omitted.** Adding a field to the `btctax-core` `IncomeRecord` struct forces edits
   at **11 construction sites in `btctax-tui`** (`tabs/income.rs`×2, `export.rs`×1, `lib.rs`×1, `tabs/tests.rs`×7)
   plus recompiles in `btctax-tui-edit`. More substantively: `btctax-tui` has **no per-row pseudo convention
   today** — its only pseudo signal is the export-time typed-attest gate (`draw.rs:260`, `export.rs:65`), not an
   on-screen banner (that banner lives in `btctax-tui-edit/draw_edit.rs:126`). So under pseudo mode the
   `btctax-tui` income tab (and holdings) currently show clean numbers with no on-screen marker. The spec must
   **decide**: (a) add a per-row `[PSEUDO]` to the TUI income tab (a `btctax-tui` render change + scope entry),
   or (b) declare the TUI relies on the existing attest/banner gate and only the CLI report gets a per-row
   marker — then drop the "sort-views Income render" wording and list `btctax-tui`/`-tui-edit` for the
   mechanical constructor-update + version bump only. Leaving it ambiguous leaves the ★ on-screen guard
   (`SPEC_pseudo_reconcile_mode.md:77`) unspecified on a real income render surface.

**Fix:** enumerate both fold pushes; name `pseudo_tag`; add `btctax-tui` (+ `-tui-edit`) to the impacted/bumped
crate list with the 11 constructor sites; and make an explicit decision + KAT for the TUI income surface
(per-row marker vs. gate-only), so the guard is provably closed everywhere an `IncomeRecord` FMV renders.

---

## MINOR

### M-A — `dirs` location: line 87 contradicts line 111
Line 87 (correctly, per round-1 M2) resolves the cache path "in the cli, passed into adapters"; line 111 lists
"`btctax-adapters` (data + `LayeredPrices` + `dirs`, no network)" — putting `dirs` in adapters. Direct
contradiction. Keep `dirs` in `btctax-cli` (path-policy out of core/adapters, per M2); fix line 111.

### M-B — the cargo-tree isolation check is not a hermetic `#[test]`
Spec lines 103-104/135 call the `ureq`/rustls-absent-from-`cargo tree` isolation a "KAT/CI check." A per-crate
`#[test]` that shells out to `cargo tree` is non-hermetic (needs `cargo` on PATH, is offline-hostile and slow).
The isolation itself is sound and verifiable — specify it as an **xtask/CI step** parsing `cargo metadata`
(or a `Cargo.lock`/dep-graph assertion), not a unit test. (Matches the binary-docs xtask precedent
[[binary-docs-infra]].)

### M-C — "NOT btctax-core/cli" is true only for DIRECT deps
`btctax-update-prices` depends on `btctax-adapters` (for the `date,usd_close` CSV format / cache path helpers),
and `btctax-adapters` depends on `btctax-core` (`PriceProvider`, `FmvStatus`, `Usd`). So `btctax-core` is pulled
**transitively**. This is fine (core has no network dep, and the arrow points *from* update-prices), but state
"no *direct* core/cli dep; core arrives transitively via adapters" so the cargo-tree assertion is written
against the right expectation (core WILL appear in update-prices' tree; `ureq`/rustls will NOT appear in the tax
binaries' trees).

---

## NIT

- **N-A** — stale comments after the swap (same class as round-1 N2): `optimize_consult.rs:408`
  ("The bundled dataset's last entry is 2025-06-15") and `reclassify_income_cli.rs:22` (`$84,000`) go stale.
  Refresh during migration. (Round-1 N1's "6-row STUB" is now correctly stated at spec line 11 — good.)
- **N-B** — Scope granularity (round-1 N3, re-asked): one combined spec for A+B+C remains **defensible** — the
  three parts are genuinely coupled (B's honesty depends on A's data via the "no price ⇒ stay blocked"
  fault-inject; C extends the same provider), and the phased T1/T2/T3 plan with per-phase stop-at-green is a
  reasonable mitigation. **But** given A is now confirmed to span **three** crates (I-A) and B amends a prior
  R0-GREEN'd spec (I1) + touches two more render crates (I-B), keep **each phase's implementation plan on its
  own review gate**, and keep A's test-migration on its own commit so B's tax review isn't gated behind A's
  churn. Judgment call — noted, not required.

---

## Confirmations (verified sound — no change needed)

- **Part C network isolation is clean and buildable.** No HTTP client exists in the workspace today; a new
  `crates/btctax-update-prices` binary depending on `btctax-adapters` + `ureq` (rustls-tls, blocking) only is
  a sound first network dep, and nothing else depending on it keeps `ureq`/rustls out of every tax binary's
  tree. rustls avoids the system-OpenSSL cross-platform trap ([[cross-platform-ci]]). Canned-JSON fixtures +
  an `#[ignore]` live smoke test is the right offline-test posture.
- **B resolve-layer seam is right.** Native income has no price fallback (`resolve.rs:282-299`); injecting a
  synthetic FMV into the `manual_fmv` map during the pseudo phase flows through `build_op`'s Income arm and
  sets the `Eff.pseudo` taint — matching the existing pseudo seam. Do NOT push into fold. The "no price ⇒ no
  synthetic ⇒ stays blocked" fault-inject is the correct honesty guard.
- **Export discipline holds.** The `IncomeRecord` CSV writer (`render.rs:715-724`) writes
  event/kind/date/sat/usd_fmv/business and must continue to **omit** `pseudo` — consistent with the dedicated-
  bool "writers omit" discipline (`SPEC_pseudo_reconcile_mode.md:107` last bullet; existing `PendingLeg.pseudo`
  at `state.rs:225-227`).
- **Version/member arithmetic is correct.** 8 members → 9 (8 publishable + xtask); all crates `0.2.0` → `0.3.0`;
  new crate is a first-time publish (5-burst new-crate rate limit, [[crate-publishing-state]]).
- **Not-affected classifications hold.** `normalize.rs:88-100` `fn prices()` is a `StaticPrices` double (SAFE);
  `price.rs:59-80` `from_csv_str` unit tests use inline data (SAFE); `optimize_mode2.rs` +
  `btctax-core/tests/evaluate.rs` use `StaticPrices::default()` (empty → injectable, SAFE);
  exchange-provided-FMV CSV tests (coinbase/gemini/swan; `verify_report.rs` 42500/67500 are exchange Buy/Sell
  columns) are `ExchangeProvided`, independent of the swap (SAFE); the many `2025-04-01` uses in `kat_tax.rs`/
  `lot_selection.rs`/`method_election*.rs`/`properties.rs`/`fixtures.rs`/`fr9_exit_code.rs` are event dates
  carrying their own USD or over `StaticPrices` (SAFE); `tui-edit/main.rs:11324-11362` uses pre-2010
  `1990-01-01` (SAFE).

---

### Suggested fold order
I-A (add the tui-edit test + widen the seam to instance-level) and I-B (both fold pushes + `pseudo_tag` + add
`btctax-tui`/`-tui-edit` to scope + decide the TUI income marker) are the two substantive folds; M-A/M-B/M-C
tighten the dirs location, the isolation-check mechanism, and the transitive-dep wording; N-A/N-B are
comment/granularity. Re-review after the fold (including the last), per §2.
