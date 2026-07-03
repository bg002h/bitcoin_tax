# Whole-diff review (Phase E) — fix/gemini-subsatoshi-round — round 1

**Scope:** `git diff main..HEAD` (main `719e9fe`; fix commit `67c9d7e`).
Source touched: `crates/btctax-adapters/src/{lib,parse,read}.rs`, `crates/btctax-adapters/tests/gemini.rs`
(plus the SPEC + R0 review artifacts). No `btctax-core` / cli / tui change.
**Contract:** `design/SPEC_gemini_subsatoshi_round.md` (R0-GREEN, 2 rounds).

## Verdict: 0 Critical / 0 Important / 0 Minor / 2 Nit — **SHIP**

The fix is correct, load-bearing-tested (fault-injection confirmed), and clean. Both Nits are
non-blocking (one is an accurate historical doc mention; one is the Task-2 FOLLOWUPS append that this
review is itself part of).

---

## Verification performed

### 1. The round is correct — PASS
`parse.rs:90` `let sats = (btc * Decimal::from(SATS_PER_BTC)).round();` then `sats.to_i64()`.
`SATS_PER_BTC = 100_000_000` (parse.rs:12). `rust_decimal::Decimal::round()` is Banker's rounding =
`MidpointNearestEven`, matching `round_cents` (`btctax-core/conventions.rs:13` `MONEY_ROUNDING`). The
product of an exact `Decimal::from_str(btc)` and `Decimal::from(1e8)` is exact for realistic magnitudes
(≤ ~2.1e15 sat = 16 int digits + ≤10 dp ≪ Decimal's 28-digit budget), so `.round()` yields the true
nearest sat and `.to_i64()` is exact on the integer-valued result. Independently recomputed all asserted
KAT values (true nearest-satoshi, half-even):

| input BTC | ×1e8 (sat) | nearest (half-even) | asserted |
|---|---|---|---|
| 0.0010216163 | 102161.63 | 102162 (.63↑) | 102162 ✓ |
| 0.0997506234 | 9975062.34 | 9975062 (.34↓) | 9975062 ✓ |
| 0.7674706206 | 76747062.06 | 76747062 (.06↓) | 76747062 ✓ |
| -0.1156442018 | -11564420.18 | -11564420 (toward 0) | -11564420 ✓ |
| 0.00076035204 | 76035.204 | 76035 (.204↓) | 76035 ✓ |
| 0.000000001 | 0.1 | 0 | 0 ✓ (sub-half dust) |
| 0.000000005 | 0.5 | **0** (even) | 0 ✓ (half-even, not 1) |
| 0.000000025 | 2.5 | **2** (even) | 2 ✓ (half-even, not 3) |

Sign correct (negative rounds toward the nearest sat, verified by the `-0.1156442018` KAT and the
pre-existing `-0.5 → -50_000_000` in `btc_to_sat_is_exact_integer`). The two ties (0.5→0, 2.5→2)
genuinely discriminate half-even from half-up (which would give 1 / 3).

### 2. [★ fault-injection] the KATs are load-bearing — CONFIRMED, tree restored clean
Edited `parse.rs:90` `.round()` → `.trunc()` and ran `cargo test -p btctax-adapters subsatoshi`:
- **Unit KAT** `parse::tests::subsatoshi_btc_rounds_to_nearest_satoshi` → **FAILED**
  `panicked at parse.rs:229 … left: 102161, right: 102162` (102161.63 truncates to 102161).
- **Integration KAT** `gemini_subsatoshi_btc_amount_rounds_and_imports` → **FAILED**
  `panicked at gemini.rs:580 … "…must round to nearest satoshi 102162" left: 102161, right: 102162`.

Both KATs are load-bearing. Restored via `git checkout -- crates/btctax-adapters/src/parse.rs`;
`.round()` back at line 90; `git status --porcelain` **empty (clean)**; on branch
`fix/gemini-subsatoshi-round` throughout (no `git checkout <branch>`). The second-probe suggestion
("neuter the integration path") was unnecessary: the trunc probe already drove the integration KAT RED
through the same numeric path, proving it independently load-bearing — so I did not run a redundant
second mutation.

### 3. The integration KAT exercises the REAL numeric path — PASS
`write_subsatoshi_fixture` (gemini.rs:512+) writes the row-1 `BTC Amount BTC` cell with
`ws.write_number(1, 4, 0.0010216163f64)` → calamine `Data::Float` → `read.rs:178` `format!("{f}")` →
`parse_btc_to_sat`. Row-2 writes the same amount as a `write_string` cell (`Data::String` path). The
KAT asserts `out.events.len() == 2` and, for **both** events, `Acquire.sat == 102_162`. The trunc probe
producing **exactly 102161** (not an f64-garbled value) empirically proves `format!("{f}")` recovers
`0.0010216163` exactly across the numeric path — the load-bearing read-layer claim (R0-I1) holds.
Pre-fix reproduction: pre-fix `parse_btc_to_sat("…","0.0010216163")` returned `FractionalSat`
(102161.63 has a nonzero fractional part), and the KAT `.unwrap()`s `gm.parse(&g)` — so pre-fix this
KAT would have panicked at exactly the user's abort point ("gemini row 2: fractional satoshi …").

### 4. `FractionalSat` removal is clean — PASS
Variant deleted from `lib.rs` (was lines 59-64). `cargo check --workspace --tests` → **Finished, no
errors/warnings** — definitive proof nothing downstream (core/cli/tui/tests) references it and no
exhaustive `match AdapterError` fails to compile. Grep for `FractionalSat` finds only: two **code
comments** (`parse.rs:249`, `gemini.rs:513` — permitted), the SPEC/R0-review artifacts, an old
superseded plan doc (`IMPLEMENTATION_PLAN_foundation_03_adapters.md`), and one RESOLVED historical
FOLLOWUPS note (see Nit N1). No live reference; not `Serialize`, not `#[non_exhaustive]`.

### 5. Docs updated — PASS
- `parse.rs:54-59` doc now says finer-than-satoshi precision is ROUNDED to nearest sat
  (`MidpointNearestEven`, matching `round_cents`), explicitly scoped to **BTC QUANTITY only** with
  "USD/tax VALUES are still never silently rounded." Inline comment `parse.rs:86-89` reinforces.
- `read.rs:169-173` no longer claims the "≤8-dp" bound; states >8-dp sub-satoshi amounts now flow
  through and are rounded downstream by `parse_btc_to_sat`, with `format!("{f}")` recovering any decimal
  in f64's clean range, and that USD/tax values are still parsed exactly.

### 6. No regression / no over-reach — PASS
Diff is `btctax-adapters` only (confirmed: no core/cli/tui path touched). Full crate suite green:
`cargo test -p btctax-adapters` → **61 passed / 0 failed** across all bins. `btc_to_sat_is_exact_integer`
(parse.rs:215 — the clean-8dp KAT: `1`, `0.00000001`, `0.12345678 BTC`, `-0.5`) is unchanged and passes;
round is a verified no-op on whole-sat values. NFR5 intact: the change rounds a **Decimal-parsed BTC
quantity**, never a USD value — `parse_usd` untouched, no USD path modified (read.rs change is a doc
comment only). `cargo clippy -p btctax-adapters --tests` → **clean, no warnings**.

### 7. Conservation (FR9) — PASS
Rounding is per-cell; the ledger tracks integer sats (no re-derivation from BTC strings), and a Gemini
trade's BTC leg is a single cell, so no two-legs-of-one-trade drift. Worst-case drift ≤ 0.5 sat/row
(≤ 4 sat on the user's 8 rows), inherent to sub-satoshi source data, < $0.001. No `btctax-core` change,
no downstream assumption broken.

---

## Findings

### [N1] NIT — FOLLOWUPS.md historical note names the removed `FractionalSat` variant
`FOLLOWUPS.md:1198` (a **RESOLVED** Task-0 note about the `source`→`adapter` field rename) lists
`MissingColumn`/`Parse`/`FractionalSat` as the variants that once had a `source` field. It accurately
describes the brief's stub at that point in history, so it is not wrong — but a future reader grepping
`FractionalSat` will land here. Optional: add a parenthetical "(`FractionalSat` since removed — see
gemini-subsatoshi-round)" if you want grep-cleanliness. Non-blocking; no code impact.

### [N2] NIT — Task-2 FOLLOWUPS append ("Gemini sub-sat now rounds") not yet in the diff
The plan's Task 2 calls for a FOLLOWUPS entry recording the reject→round behavior change. It is absent
from the reviewed commit — expected, since this whole-diff review *is* Task 2 and the append is its
closing step. Reminder to add it before ship (record: sub-satoshi BTC quantities now round to nearest
sat, ≤0.5 sat/row drift by design; USD/tax rounding unchanged). Not a code defect.

---

## Fault-injection log
- Probe A: `parse.rs:90` `.round()`→`.trunc()`; `cargo test -p btctax-adapters subsatoshi` → unit KAT
  RED (102161 vs 102162) **and** `--test gemini gemini_subsatoshi` → integration KAT RED (102161 vs
  102162). Restored `.round()` via `git checkout -- crates/btctax-adapters/src/parse.rs`.
- Post-probe: `git status --porcelain` empty; `.round()` present at parse.rs:90; workspace `cargo check`
  + `cargo clippy` clean; branch unchanged.

**SHIP.**
