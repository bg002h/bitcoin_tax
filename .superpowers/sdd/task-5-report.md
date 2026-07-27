# Task 5 report (oracle-sweep) — `golden_returns.rs` (compute level) onto the divergence-class machinery

**Status: DONE.** `golden_returns.rs` reworked off the hand-written `Divergence` / `DECLARED_DIVERGENCES`
array onto `tax::oracle_diff`'s class machinery + the full line set. All 12 households green; `make check`
green (1919 passed, 1 skipped; clippy `-D warnings` clean, EXIT=0).

Only `crates/btctax-core/tests/golden_returns.rs` changed (265 insertions, 313 deletions).
`crates/btctax-core/src/tax/oracle_diff.rs` was mutated for the guard check and **restored from a `cp`
backup** (verified `git diff` empty).

---

## 1. Baseline green first (TDD / characterize-green — Step 1)

```
$ cargo test -p btctax-core --test golden_returns
test every_golden_household_matches_the_independent_oracles ... ok
test result: ok. 1 passed; 0 failed
```

Backups taken before any edit: `scratchpad/gr.bak` (test) and `scratchpad/oracle_diff.bak` (machinery).

## 2. The reworked comparison (Step 2)

The old per-household `Divergence` struct + `DECLARED_DIVERGENCES` array and the reconciliation guard
(`:317-401`) are gone, replaced by the **two-level rule**:

- **Level 1 — the six cent-exact leaf totals** (QBI deduction, AGI, taxable income, SE tax, Additional
  Medicare, NIIT) stay **exact-vs-BOTH-oracles**, verbatim in spirit to HEAD: `round_dollar` both sides,
  `ours == round_leaf(OTS) && ours == round_leaf(taxcalc)`. No class escape (a mismatch is a hard fail).
- **Level 2a — L16 (tax): the §6.2 two-part.**
  - *Part 1 (structural, r2-I2 Table-semantics witness):* `assert_eq!(table_l16(reproduced_ops), ar.regular_tax)`
    — the reproduction machinery must reproduce btctax's own compute-engine L16 exactly, before any oracle
    is consulted.
  - *Part 2 (class stacking):* if the figure does not agree with both oracles, `stacking_ok(l16_ours,
    ots_l16, Some(taxcalc_l16), None, None, &reproduced_ops, None)` must absorb it, else it is btctax alone
    → `diffs`. The taxcalc Tax-Table dissent is absorbed by `taxcalc_methodology_class`.
- **Level 2b — L24 (TOTAL TAX): the phantom dissolves** (see §3).
- **Deeper-line rows** (deduction taken L12, Sch D→L7, SALT L5e via a `[(…); 3]` array; 8995 L12
  net-cap-gain OTS-single-witness) gate on `Option::Some` — no-ops now (all oracle leaves are `None` in the
  baked JSON), wired so they light up at T11 without a rewrite.

### How the L16 methodology operands were sourced (the ★ critical, non-obvious part)

`reproduced_ops: L16Operands` is built **entirely from btctax's own assembled return `ar`** — PRE-T11, no
oracle `Option` leaves:

| `L16Operands` field | source | why |
|---|---|---|
| `status` | `ri.filing_status` | the return's filing status |
| `ti` | `ar.taxable_income` | 1040 L15 |
| `qd_l3a` | `ar.qualified_dividends` | 1040 L3a |
| `net_ltcg_qd_excl` | `ar.net_ltcg` | §1(h) preferential net capital gain, **QD-EXCLUSIVE** |

These are exactly the three figures `assemble_absolute` passes to `qdcgt_line16` at
`return_1040.rs:1216-1222` (`taxable_income`, `qualified_dividends`, `net_ltcg`). The `net_ltcg` field's
doc (`return_1040.rs:897-899`) confirms it is "the §1(h) preferential net capital gain (QDCGT net-LTCG /
Form 8995 net-capital-gain), ≥ 0" — the QD-exclusive term `qdcgt_line16` adds to `qual_div` to form its L4.
So `taxcalc_methodology_class(reproduced_ops)` is **condition-only** (consults the Tax Table?) and fires
PRE-T11. The PROVENANCE class needs the oracle's own `Option` leaves (`None` now) ⇒ `provenance_class_fires`
returns `false` ⇒ provenance is correctly inert until T11 (`ots_ops`/`taxcalc_ops` passed as `None`).

## 3. The L24 change and the anchor it dissolves

OTS's L24 side changed from `round_dollar(e.total_tax)` (roundΣ of the exact total) to
`sum_round(&[e.income_tax_before_credits, e.se_tax, e.additional_medicare_tax, e.niit])` — Σround of OTS's
own **component totals** (the pre-T11 fallback per plan lines 68-72; the leg form
`sum_round([se_l10_oasdi, se_l11_medicare, f8959_l7, f8959_l13, …])` activates when the legs bake at T11).
This equals btctax's printed cross-foot `printed.f1040.line24` on all 12, so the comparison passes and
OTS's exact total is never consulted.

**Diagnostic evidence (temporary test, since removed):**

```
single_miner_qbi_limited_by_net_capital_gain  L24 ours=16833  sumround=16833  roundtotal=16832  match=true
```

`sumround` (16833) matches btctax's printed 16833; `roundtotal` (`round_leaf(e.total_tax)` = 16832) is the
old phantom. The `single_miner_qbi_limited_by_net_capital_gain` divergence (old `agrees_with:"neither"`
entry, `:157-191`) **dissolves**. All 12 households show `match=true` on L24 (no leg flips).

## 4. Disposition of the old `DECLARED_DIVERGENCES` (Step 3 verification)

- The **6 taxcalc Table-L16 entries** (`:95-144` + `single_crypto_business_se` `:192-202`) are now absorbed
  by `taxcalc_methodology_class`. The diagnostic confirms `record_fire` on exactly these 6 anchors — the
  ones where taxcalc dissents on L16 (`tc_dissent=true, meth=true`):
  `single_w2_only_standard, single_w2_plus_crypto_ltcg, single_qdcgt_both_slices,
  single_short_term_crypto_gain, single_capital_loss_capped, single_crypto_business_se`.
  ★ `single_qdcgt_both_slices` (TI = 112,400 ≥ $100k, ordinary remainder < ceiling) fires `meth=true` — the
  case the old "TI < $100k" gloss wrongly excluded.
  (Two further households — `mfj_itemized_salt_over_the_cap`, `single_miner_qbi` — also *consult* the Table
  (`meth=true`) but taxcalc happens to agree on L16, so no dissent needs absorbing; they take the fast-path.)
- The **7th entry** (`single_miner_qbi` L24) **dissolves** via the L24 `sum_round` change (§3).

## 5. Mutation-check — the anti-world guard has teeth

Added a synthetic both-oracle-disagree test `stacking_ok_guards_golden_returns_against_btctax_alone` (none
of the 12 real households is a both-disagree, so the real loop never exercises the FAIL branch): btctax
47,030 alone against OTS + taxcalc 47,031, above the ceiling (methodology cannot fire), no pin.

Mutation (in `oracle_diff.rs`, backed up first): forced `stacking_ok` to `return true` at the top →

```
test stacking_ok_guards_golden_returns_against_btctax_alone ... FAILED
  btctax alone against BOTH oracles ... must be REJECTED — the anti-world guard is the whole point ...
```

Restored `oracle_diff.rs` from `scratchpad/oracle_diff.bak` (NOT `git checkout`); `git diff` on it is empty;
both tests pass again.

## 6. Per-household green + gate

```
$ cargo test -p btctax-core --test golden_returns
test stacking_ok_guards_golden_returns_against_btctax_alone ... ok
test every_golden_household_matches_the_independent_oracles ... ok
test result: ok. 2 passed; 0 failed

$ make check   →  EXIT=0,  Summary: 1919 tests run: 1919 passed, 1 skipped  (clippy -D warnings clean)
```

## 7. Liveness posture (per the two-level rule)

`LivenessLedger` registered; `record_fire("taxcalc_methodology")` on each absorbed L16 dissent. A plain
positive check (`assert!(methodology_class_fired, …)`) proves the live class engaged this run. **No
`LivenessLedger::dead()` assertion and no provenance-class liveness assertion** — both are commented as
"enabled in T11 with the pinned cells". The anti-world guarantee is carried by `stacking_ok` (mutation-
checked, §5).

## 8. Self-review / concerns

- **Structural witness is exact, not rounded:** `table_l16(reproduced_ops) == ar.regular_tax` (both whole
  dollars via `qdcgt_line16`); it uses `ty2024_table()`, the same schedules `assemble_absolute` is given.
  Passes on all 12.
- **Deeper-line rows are genuine no-ops now** (oracle leaves `None`); they compile against real `ar` sources
  (`ar.deduction`, `ar.capital_gain`, `ar.schedule_a…salt_5e`, `ar.printed_inputs.qbi_net_capital_gain`), so
  they will *activate and be validated at T11*. If a compute-vs-oracle mapping is subtly off (e.g. the signed
  L7 convention), T11 (with real baked data + review) is the designed catch point — that is the plan's
  `Option::Some`-gating contract, not a gap here.
- **`liveness` is written-only until T11** (only `record_fire`, read by the deferred `dead()` sweep). This is
  intentional (the seam T11 extends) and clean under clippy `-D warnings`.
- **No FROZEN files touched** (`types/compute/se.rs` untouched; `oracle_diff.rs` restored bit-for-bit).
- No new caught bugs (all 12 stay green with no known-defect pin needed).

**Files changed:** `crates/btctax-core/tests/golden_returns.rs`.
