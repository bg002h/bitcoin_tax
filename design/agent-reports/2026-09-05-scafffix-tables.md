# scafffix — `shipped_tables_are_the_validated_tables.rs` now covers every shipped year

**Date:** 2026-09-05
**File owned and changed:** `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs` (only this file)
**Result:** 9 tests, **8 PASS / 1 FAIL**. The one red is the finding, not a regression — see §3.

---

## 1. What was wrong, measured

The file exists to close *"an assurance surface that does not touch the artifact it is claimed to
assure."* It had become an instance of that class itself.

| measurement | value | how measured |
|---|---|---|
| years the binary ships tax tables for | **2017, 2024, 2025, 2026** | `BundledTaxTables::load()`, `crates/btctax-adapters/src/tax_tables.rs:76-81`; also printed at runtime by the new gate |
| years the binary ships full-return params for | **2024** | `BundledFullReturnTables::load()`, `tax_tables.rs:99-104` |
| years this file asserted on | **2024** (three `table_for(2024)` / `full_return_for(2024)` literals) | old file lines 50, 211, 260 |
| `tyNNNN_table` / `tyNNNN_params` fns in `btctax_core::tax::testonly` | **`ty2024_table`, `ty2024_params`, and nothing else** | `grep -rn "ty202[567]_table\|ty202[567]_params\|ty2017_table" crates/btctax-core/src/tax/testonly.rs` → **0 hits** |
| references to `ty2024_table`/`ty2024_params` across `crates/` | **267** | `grep -rn "ty2024_params\|ty2024_table" --include=*.rs crates/ \| wc -l` |

Two distinct defects fell out of that:

1. **Three of four shipped years were compared to nothing by this file**, and a fifth year added
   tomorrow would have been compared to nothing *while every test here reported success* — the
   FALLS-BACK class, `design/TY2026_PORT_REPORT.md` §2 row 25.
2. **Even for TY2024 the equality was incomplete.** `TaxTable::gift_annual_exclusion` and
   `gift_lifetime_exclusion` were never compared (only `year`, `ordinary`, `ltcg`, `ss_wage_base`
   were), and nothing forced a *future* field to be compared either.

### 1a. The three unwitnessed years are NOT equally exposed

Measured by matching every shipped ordinary-bracket `lower` literal against the assertions in
`tax_tables.rs`'s own `mod tests` (33 test fns). This is an **upper** bound on coverage — a literal can
be matched by another status's identical figure — so the true numbers are no better than these:

| year | bracket thresholds appearing in ANY assertion | full 28-edge KAT | `testonly` twin |
|---|---|---|---|
| TY2017 | 28/28 | yes — `ty2017_table_matches_rev_proc_2016_55` | no |
| TY2024 | 28/28 | yes — `ty2024_full_schedule_equality_all_28_edges_and_ltcg` | **yes** |
| **TY2025** | **8/28** | **no** | no |
| TY2026 | 24/28 | no (per-status spot KATs; the 4 misses are each schedule's `$0` floor) | no |

**TY2025 is the live exposure.** Twenty bracket thresholds the binary applies to a filer are asserted
by nothing anywhere: the entire interior of MFJ (`23850/96950/206700/394600/501050`), the entire HoH
schedule (`17000/64850/103350/197300/250500`), and Single/MFS `103350/197300/250525`. TY2017 and TY2026
are pinned, but by a KAT living in the same file as the data it checks — weaker independence than a
second artifact, and neither is bound to the corpus that computes returns.

---

## 2. What changed

Rewritten around three ideas. **No year appears in any assertion.**

**§1 — the shipped year set is derived from the shipping artifact, twice.** `by_year` is private, so
the file computes the set by two independent mechanisms that must agree:

- **A (what the public lookup answers to):** probe `table_for(y)` / `full_return_for(y)` over
  `1861..=2200`.
- **B (what the artifact advertises):** parse the map keys out of the artifact's own derived `Debug`
  rendering — exhaustive by construction, no window.

A disagreement is an **error naming the year**, so a year outside the probe window is *detected*
rather than missed, and a `Debug` that stops parsing yields an empty set → error, not a vacuously
green loop. A third reading falls out free: every value's own `year` field is cross-checked against
its map key, which catches the one-token year-port defect `by_year.insert(2026, ty2025())`.

**§2 — the validated side fails closed.** `validated_table_for` / `validated_params_for` are the one
written mapping (a test cannot enumerate a module's functions), written in the direction that cannot
go quiet: an unlisted year → `None` → a **named test failure**, never a skip.

**§3 — the comparisons.**
- Both sides are now **destructured exhaustively** (`let TaxTable { .. } = shipped;`), so adding a
  field to `TaxTable` or `FullReturnParams` fails to compile here (E0027) until someone decides
  whether it belongs in the equality. The compiler does the review.
- TY2024's assertions are unchanged in strength and **extended**: `gift_annual_exclusion` and
  `gift_lifetime_exclusion` are now compared. `source` is deliberately *not* compared — equal
  provenance strings would mean one side was copied from the other.
- The `MAX_UNVALIDATED = 2` count ratchet is replaced by the **mechanism**: the only lawful absence is
  a `Qss` schedule missing from *both* sides (§1(a)/§2(a) + `TaxTable::key` normalises `Qss → Mfj`).
  Strictly stronger — under the count, a `Single/ordinary` row missing from both sides passed; now it
  fails — and unlike a count it composes over a year *set*.
- The §199A published-*"Phase-in range amount"* invariant became a pure function
  (`qbi_phase_in_findings`) so it can be watched going red on a tampered clone, and a year with no
  primary-source figure is now **reported by name** instead of silently skipped.

**Test inventory (9):** 3 comparison/gate tests, 6 kills. `a_single_moved_bracket_is_detected` kept
verbatim (it is cited in `reviews/branch-r6-instrument-opus.md`).

---

## 3. ★ The red, and why it is the finding

`every_shipped_year_has_a_validated_counterpart` **FAILS**, printing:

```
the binary ships tax tables for {2017, 2024, 2025, 2026} and full-return params for {2024};
the validated corpus has NO counterpart for these:
  tables: [
    "TY2017: 4 ordinary schedule(s)/28 bracket(s) + 4 §1(h) breakpoint pair(s), ss_wage_base 127200 — source \"Rev. Proc. 2016-55 …\"",
    "TY2025: 4 ordinary schedule(s)/28 bracket(s) + 4 §1(h) breakpoint pair(s), ss_wage_base 176100 — source \"Rev. Proc. 2024-40 …\"",
    "TY2026: 4 ordinary schedule(s)/28 bracket(s) + 4 §1(h) breakpoint pair(s), ss_wage_base 184500 — source \"Rev. Proc. 2025-32 …\"",
  ]
  params: []
```

This is the state the brief predicted (*"a year that ships an unvalidated table is exactly what this
test should catch"*) and what the port report asked for (*"a shipped year with no validated
counterpart is a compile-or-test failure rather than a silence"*, instruments I-3 recommendation 4).
It is deliberately its **own** test, so the two comparison tests stay green and the red names exactly
one thing.

**It cannot be closed from inside this file, and must not be closed the cheap way.** Copying
`tax_tables.rs::ty2025()` into `testonly` would satisfy it by construction and prove nothing — two
artifacts agree only if derived independently (port report rule 10). Closing it means transcribing the
other side from Rev. Proc. 2016-55 / 2024-40 / 2025-32, which are **not on disk**: `legal/text/` holds
only `RevProc_2024-28.txt` (the digital-asset safe harbour). Priority order by exposure: **TY2025
first** (8/28), then TY2026, then TY2017.

---

## 4. Which test reds for which planted defect

Every check here was watched going red on a planted defect (B1). Plants marked *(temporary)* were
applied to my own file, run, and reverted; the file on disk is clean (`grep -c "PLANT P"` → **0**).

| # | planted defect | test that reds | observed message |
|---|---|---|---|
| P5 *(temporary)* | a validated counterpart appears for **TY2025** carrying TY2024's numbers | `every_shipped_tax_table_equals_the_one_the_corpus_validates` | `TY2025 Single ordinary bracket 1: shipped (11925, 0.12) vs validated (11600, 0.12)` — **proves the loop covers a non-2024 year** |
| P1 *(temporary)* | TY2024 Single bracket 2 `lower` +$1 on the validated side | same | `TY2024 Single ordinary bracket 2: shipped (47150, 0.22) vs validated (47151, 0.22)` |
| P2 *(temporary)* | validated side loses `Mfs/ordinary`; gains a `Qss/ltcg` row | same | `2 schedule(s) …: ["TY2024 Mfs/ordinary (shipped only — UNWITNESSED)", "TY2024 Qss/ltcg (validated only)"]` — and the *lawful* `Qss`-absent-from-both rows stay silent, so it discriminates |
| P4 *(temporary)* | validated side vanishes entirely | same | `no shipped year had a validated counterpart, so this test compared NOTHING while reporting success` |
| P3a *(temporary)* | TY2024 Single standard deduction +$1 | `every_shipped_full_return_params_equal_the_ones_the_corpus_validates` | `TY2024 Single: STANDARD DEDUCTION differs …  left: Some(14600)  right: Some(14601)` |
| P6 *(temporary)* | published MFJ §199A top off by $1 | same | `the §199A threshold + width does not reconcile … ["TY2024 Mfj: threshold + width = 483900, but the revenue procedure publishes … 483901"]` |
| committed kill | one bracket moved by $1, on clones | `a_single_moved_bracket_is_detected` | green (asserts inequality **and** that every other bracket still matches) |
| committed kill | a year outside the probe window (`PROBE_HI + 800`) | `the_year_derivation_reds_on_a_year_outside_the_probe_window` | derivation returns `Err` naming 3000; the clean bundle derives to `{2024}` |
| committed kill | `by_year.insert(2026, ty2024_table())` | `the_year_derivation_reds_when_a_table_is_filed_under_the_wrong_year` | `Err` naming both 2026 and 2024 |
| committed kill | a marker that does not occur / a `by_year:` false positive | `the_debug_year_parser_discriminates` | parser finds both keys and both self-declared years; finds **nothing** for an absent marker |
| committed kill | a shipped year with no witness | `the_counterpart_gate_discriminates` | `{2024}` → empty; `{2024, 2999}` → `[2999]` |
| committed kill | the published §199A **top** pasted into the **width** field | `the_qbi_phase_in_top_invariant_catches_the_published_amount_paste` | exactly one finding, naming `Mfj`; silent on clean params; a year with no published figure reports all 5 statuses |

Verification commands run: `cargo nextest run -p btctax-adapters --test shipped_tables_are_the_validated_tables`
(9 tests, 8 pass / 1 fail), `CARGO_TARGET_DIR=target-clippy cargo clippy -p btctax-adapters
--all-targets --all-features -- -D warnings` (clean), `rustfmt --edition 2021 --check <file>` (clean).

---

## 5. Found and NOT fixed

1. **★ The red itself (§3).** TY2017/TY2025/TY2026 ship tables with no independently derived
   counterpart. Closing it needs the revenue procedures, which are not in the repo, and edits to
   `btctax-core/src/tax/testonly.rs` — another agent's file. **TY2025's 8/28 is the number to act on.**
2. **`BundledTaxTables` exposes no year enumeration.** The port report's recommendation
   (*"iterate `BundledTaxTables::load().by_year.keys()`"*) is not reachable from an integration test —
   `by_year` is private. §1's Debug-parse + probe cross-check is a workaround. **A `pub fn years(&self)
   -> impl Iterator<Item = i32>` on both bundles would let this file (and every other year-general
   instrument) drop the parser.** That is a one-line change in `tax_tables.rs`, which I do not own.
3. **`FullReturnParams` for TY2025/TY2026 do not exist at all**, so the params test covers 100% of its
   shipping surface today and will red the moment a year is added without a counterpart — by design,
   and worth flagging to whoever adds them.
4. **The §199A published-top figure is TY2024-only.** `published_qbi_phase_in_top` returns `None` for
   every other year and the test reports that by name; the moment TY2025/TY2026 params land, somebody
   must read §.27 of the relevant Rev. Proc. and add the two columns.
5. **`tax_tables.rs`'s in-crate KATs check the data in the file they live in.** Not a defect I can fix
   from here, but the independence is weaker than the `testonly` twin, and the table in §1a should not
   be read as "TY2017 and TY2026 are as safe as TY2024".
