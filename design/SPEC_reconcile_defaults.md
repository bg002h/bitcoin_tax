# SPEC — cleaner reconciliation defaults: HIFO global default + long-term self-transfer-in acquisition

**Source baseline:** `main` @ `b976621` (branch `feat/reconcile-defaults`). **Review status: R0 round 1 folded
(2C/4I/3M/2N — merged IN-PLACE; surgical). Awaiting R0 round 2.** Review:
`reviews/R0-spec-reconcile-defaults-round-1.md`. User-mandated tax-policy default change (2026-07-05), updating
[[self-transfer-completion-policy]]. Two ~1-line changes, TAX-CRITICAL, LARGE test blast radius (42 tests).

## Goal & rationale (user-mandated)
The auto-reconcile estimate was needlessly punitive: unknown-basis deposits defaulted to **short-term**
(acquired = receipt) under **FIFO**. But most received BTC is a **long-held cold-storage deposit** and **HIFO
is the most common elected method**. So make both defaults realistic:
1. **Default method FIFO → HIFO** — GLOBAL (real + auto-reconcile), the fallback when no election is on file.
2. **Self-transfer-in acquisition default = 1 year + 1 day before receipt** (→ always long-term), for BOTH the
   pseudo default AND the manual `classify-inbound-self-transfer` (when no explicit `--acquired` supplied).

**Tax direction (honest):** both REDUCE the estimate (HIFO minimizes gain; long-term lowers the rate); basis
stays $0 (still conservative on the AMOUNT). Revises the prior conservative short-term policy — intended.

## Change 1 — HIFO as the global default [R0-C1: CORRECTED site set]
Flip these FOUR explicit `LotMethod::Fifo` DEFAULT literals → `LotMethod::Hifo`:
- **`fold.rs:41`** `.unwrap_or(LotMethod::Fifo)` — **the ONLY method-resolution path in the fold** (its own
  doc; the post-2025 default). **This is the load-bearing computation default the round-1 sketch MISSED.**
- **`config.rs:26`** `CliConfig::default()` `pre2025_method` — **the real CLI persisted default**
  (`ProjectionConfig::default()` at mod.rs:54 is SHADOWED on the CLI path via `read_config → to_projection`,
  so config.rs:26 is what real vaults actually use). Also round-1-MISSED.
- **`mod.rs:54`** `ProjectionConfig::default().pre2025_method` (the core default; kept in sync with config.rs).
- **`mod.rs:125`** `in_force_methods` display helper (so the UI reports HIFO consistently with the computation).
- **[R0-C1] DROP the round-1 sites** `event.rs:487` + `persistence.rs:434` — they are inside `#[cfg(test)] mod
  tests` (fixture literals), NOT defaults.
- **[R0-C2] Do NOT touch the enum `#[default]` (mod.rs:27) — keep `= Fifo`.** The only `#[serde(default)]
  pre2025_method` is on the **IMMUTABLE `SafeHarborAllocation` (event.rs:183-188)**; flipping the enum default
  would silently REWRITE pre-A.7 irrevocable allocations to HIFO (transition.rs:29-32 conserves under the
  RECORDED method). The four explicit literals achieve the intent with ZERO serde effect. There is no
  serde-default on the config path (SQLite key-value, explicit tag matching) — the round-1 serde concern was
  mis-targeted.
- **Keep the deliberate FIFO MECHANIC `pools.rs:63`** (`consume(.., Fifo, None)` — acquisition-date order for a
  relocation/removal mechanic, not the electable tax method).
- **[compliance]** HIFO needs specific-ID/records; the default stays `attested: false` (config already surfaces
  this) so the user is prompted to affirm it per exchange. Keep that surface.

## Change 2 — long-term self-transfer-in acquisition (one line + disclosure)
- **`fold.rs:1019`** `let acq = acquired_at.unwrap_or(date);` → default to **1 year + 1 day before `date`**
  (the receipt/event date) when `acquired_at` is `None`. This single site is the common default for BOTH the
  pseudo `SelfTransferMine{$0}` synthetic AND the manual classify (both → `Op::SelfTransferInbound { acquired_at:
  None }`, resolve.rs:380-386).
- **[R0-I1] Leap-safe long-term (the round-1 `days(366)` was WRONG):** `date − 366d` fails on a leap-crossing
  (receipt 2020-03-01 → 2019-03-01; `one_year_after = 2020-03-01 = date`; `is_long_term` = `disposed >
  one_year_after` (conventions.rs:65) is FALSE for a same-day sale). Use **`replace_year(year−1)` then `− 1 day`
  (Feb-29 → Feb-28 handled)**, or `days(367)` — either GUARANTEES long-term for any sale on/after `date`. Add a
  **leap-crossing KAT**. Checked/saturating sub (BTC dates 2009+; no real underflow).
- **[R0-I2] Disclosure independent of `--basis`:** the backdating fires on `acquired_at.is_none()`, but today's
  only disclosure (the zero-basis advisory) fires on `basis.is_none()` — independent (cli.rs:290/292 are
  separate flags). `classify-inbound-self-transfer --basis 500` (no `--acquired`) would SILENTLY backdate to
  long-term. Emit an advisory whenever the acquired date is DEFAULTED (a "holding period assumed long-term —
  correct with `--acquired`" note), gated on `acquired_at.is_none()`, not on basis.
- **[R0-I3] Fix the now-stale "short-term / receipt-date" user text:** the advisory body `fold.rs:1025-1026` and
  the `--help` doc `cli.rs:285-286` still say the HP defaults to the receipt date (short-term) — update both.
- **[ordering]** `acquired_at` is the FIFO sort key + HIFO tie-break (pools.rs:254-285; $0 lots sort last, so
  backdating reorders within the $0 group). Intended; KAT the order. Pool/transition orthogonality holds (method
  keys on disposal date, pool on receipt date) — confirmed.

## [R0-I4] Test blast radius — empirical: 42 tests across 14 binaries (the plan OWNS the migration)
Not mechanical — the DOMINANT cluster is the **OPTIMIZER suite (~20 tests: optimize_accept/run/mode1/score/
wash_sale/safe_harbor_method + session)** whose fixtures are defined RELATIVE to a FIFO baseline (e.g.
`high_basis_pick_lowers_tax_below_fifo_baseline`, `hifo_beats_fifo_matches_oracle`) — these must set an EXPLICIT
FIFO baseline (not rely on the now-HIFO default), reasoning about each test's intent. Plus **tui/tui-edit (5)**,
**method_election(_scoped) (9)**, **transition (4)**, and the **2-3 Change-2 self-transfer term/acquired tests**
— including REPLACING (not duplicating) the inverted KAT `self_transfer_in_hp_defaults_to_receipt_date_short_term`
(kat_tax.rs:2972). **adapters = 0 failures** (the round-1 "rate-engine KATs" guess was empty). R0 round 2 to
confirm the list is complete before implementation.

## KATs
- `default_method_is_hifo` (no election → HIFO; `Hifo (attested: false)`); `explicit_fifo_election_still_fifo`;
  `pools_mechanic_stays_fifo`. `safe_harbor_allocation_serde_default_stays_fifo` [C2 guard — immutable records
  unaffected].
- `self_transfer_in_defaults_to_long_term` (unknown-basis inbound, no acquired date → acquired 1yr+1day before
  receipt, term long on any later sale); `self_transfer_long_term_leap_crossing` [I1]; `explicit_acquired_supersedes`;
  `manual_classify_inbound_self_transfer_also_long_term`; `classify_with_basis_no_acquired_discloses_long_term` [I2].
- **★ fault-inject:** revert `fold.rs:1019` → `unwrap_or(date)` ⇒ `self_transfer_in_defaults_to_long_term` RED;
  revert `fold.rs:41` → Fifo ⇒ `default_method_is_hifo` RED.

## Scope / SemVer / lockstep
btctax-core (`fold.rs:41` + `fold.rs:1019` + `mod.rs:54/125`) + btctax-cli (`config.rs:26` + `cli.rs:285-286`
help + the I2 disclosure) + the 42 migrated tests. **Behavior change** to default tax outcomes → MINOR + a
CHANGELOG/README note + update [[self-transfer-completion-policy]] memory. `config` already prints the method
(no new flag). Enum `#[default]` UNCHANGED (C2).

## Plan (TDD)
- **T1** — flip the 4 method-default literals (`fold.rs:41`, `config.rs:26`, `mod.rs:54/125`) + `fold.rs:1019`
  (leap-safe) + the I2 disclosure + the I3 stale-text fixes; the KATs + the ★ fault-injects; **migrate the 42
  enumerated tests** (explicit FIFO baselines for the optimizer/method-election clusters; replace the inverted
  KAT; update term/acquired expectations). Whole-diff + full suite + CHANGELOG + FOLLOWUPS + memory.

## Gotchas
- **[C1] flip `fold.rs:41` + `config.rs:26`** (the real computation/CLI defaults) — NOT the test literals; UI-only
  flips leave real vaults on FIFO.
- **[C2] never touch the enum `#[default]`** (immutable SafeHarborAllocation) — flip explicit literals only.
- **[I1] leap-safe long-term** (`replace_year−1day` / `days(367)`) + a leap KAT.
- **[I2] disclose the defaulted long-term acquisition independent of basis.**
- **[I3] update the stale short-term/receipt-date advisory + help text.**
- **[I4] the optimizer + tui + method-election tests are NOT a mechanical migration** — reason per test.
- HIFO reduces gain; long-term lowers rate — both cut the estimate (mandated, not a bug); `pools.rs:63` stays FIFO.
