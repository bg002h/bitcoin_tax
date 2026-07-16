# PLAN-oracle-sweep — independent Fable plan review, r1

*Persisted VERBATIM (STANDARD_WORKFLOW §2) before folding. Reviewer: Fable (independent plan review).
Reviewed `design/IMPLEMENTATION_PLAN_oracle_sweep.md` @ HEAD `26d02ac` against GREEN spec r5. Persisted
2026-07-16. NOT green (1C/4I) — fold to r2 follows. (Note: the caught-bug policy amendment `9fdb9cc` landed
AFTER this review's HEAD; it is not assessed here and will be re-reviewed with the r2 fold.)*

---

VERDICT: 1 Critical / 4 Important / 4 Minor / 4 Nit

**Artifact:** `design/IMPLEMENTATION_PLAN_oracle_sweep.md` @ HEAD 26d02ac, against GREEN `design/SPEC_oracle_sweep.md` (r5). All citations below verified against current source.

---

## Critical

### C1 — The cross-footed reproductions have no component leaves to consume; T11 goes red on every SE split-flip cell, and no task owns the fix

**Touches:** T1 (schema), T2 (`sum_round`), T5/T6 (line set), T8/T9 (drivers), T11 step 3. Spec §6.2 (the sentence the plan drops), §6.1.

The spec places an explicit obligation **on the plan**: *"Each compared line's reproduction is derived line-by-line from `printed.rs`/`other_taxes.rs` in the plan; the patterns below are illustrative, not an exhaustive taxonomy (r2-M2)"* and *"Cross-footed and rate-on-printed lines require the oracle drivers to expose the component/operand lines they consume."* The plan contains no per-line table, and its schema/driver tasks (T1:32, T8:290, T9:304) add **only** the four deeper lines + the L16 provenance leaves. Nothing bakes the components the cross-footed reproductions need:

- **Sch SE L12**: btctax prints `round(L10) + round(L11)` (`printed.rs` — "★ Add lines 10 and 11 — over the PRINTED lines"). The only baked oracle figure is the exact total `se_tax`. `sum_round` over `[se_tax]` degenerates to `round_dollar(exact_total)` — precisely the lawful Σround≠roundΣ residual r1 C-1 refuted the comparison for.
- **8959 L18**: btctax prints `round(L7) + round(L13)`, and the repo's **own doc-comment** says this "may differ from `round_dollar(se.addl)` by a dollar, and that is correct, not a bug" (`crates/btctax-core/src/tax/other_taxes.rs:79-85`, KAT at `:473-510`: `275 + 500 = 775 ≠ round(774.00)`). Only the exact total `additional_medicare_tax` is baked — the reproduction cannot express the repo's own documented-correct behavior.
- These propagate: printed Sch 2 L21 sums the printed SE L12 and 8959 L18, so **1040 L24's reproduction inherits both**. The current `golden_packet.rs:120-123` formula (`round(tax)+round(se)+round(niit)+round(addl)`) passes today only because the 12 anchors' fractional parts happen not to flip — T2 generalizes that formula (`sum_round`) with the latent gap intact.

**Concrete failure:** on integer-input corpus cells, `ss = 12.4%·(0.9235·P)` and `medicare = 2.9%·(0.9235·P)` have quasi-uniform cents; `round(a)+round(b) ≠ round(a+b)` for roughly a quarter of them. A ~100-cell corpus with SE varied per §5.1 yields ~10+ households where the T11 re-bake turns `make check` red on SE L12 / 8959 L18 / L24 — divergences that are **lawful printing residuals with no declared class** (the spec declares provenance classes for the L16 family only, deliberately, because component exposure makes them unnecessary elsewhere). T11 step 3's triage list ("a genuine btctax fill/compute bug … or a corpus/steering error — do not weaken") names neither actual cause, so the executor is misdirected at the moment of failure, and the repair (new schema fields → driver changes → re-bake) reopens T1/T8/T9 mid-T11. The plan as written cannot close T11 green.

**Fix:** add to the plan the spec-mandated per-line table: for each §6.1 line, the pattern (Leaf / Cross-foot / Rate-on-printed / Table two-part) and the exact operand source. Extend T1/T8 with the component leaves the cross-foots need — at minimum OTS Sch SE L10/L11 (already in `_parse`'s reach: `ots_direct.py:164-171` captures every `Lxx` printed) and the 8959 L7/L13 legs (or derive them from the shared whole-dollar inputs + baked figures, stated explicitly); taxcalc-side components that don't exist go single-witness per §6.4's `Option` rule (T9). AGI's reproduction should also state its Sch-1 L26 source (`round(se_tax/2)` from the baked exact total works — say so).

---

## Important

### I1 — The §6.4 L12-closure mandate (r1 I-5, open decision §14.2) has no task; the new 8995-L12 "oracle check" bakes the driver's own arithmetic

**Touches:** T8 step 1 (:292), T1 (:32), T9 (:304). Spec §6.4 last bullet, §14.2.

The spec: OTS cannot infer net capital gain — the driver **hand-computes** 8995 L12 and feeds it in (`ots_direct.py:290-294`, verified: `net_capital_gain = qd + max(0, min(ltcg, ltcg+stcg))`, passed as `"L12": round(net_capital_gain)`), so "paper L12 vs OTS L12" is self-referential. The spec mandates the plan close the loop (derive OTS's L12 from OTS's own Schedule D output) and/or resolve a taxcalc L12-granular variable, and **until closed, mark L12 single-witness/weak**. The plan's T8 instead says *"Keep `qbi_cap_l12` from the existing `round(net_capital_gain)`"* — the self-referential value becomes a baked `ExpectedOts` field that T5/T6 will compare as if it were an oracle figure; T9 defers the taxcalc variable to implementation with no fallback; no task marks the line weak. The plan's self-review coverage map (:378) lists §6.4 → T3/T5/T6/T11, none of which touch this. **Fix:** in T8, derive `qbi_cap_l12` from OTS's Schedule D output lines (the `_parse` mechanism already captures them), or add the explicit "single-witness/weak until closed" disposition to T1's schema comment and T5/T6's line table.

### I2 — T6 evolves only part of `golden_packet.rs`; the three `checked == 3` counts and the other full-corpus fill loops break at T11

**Touches:** T6 Files (:252-254) and step 2 (:260); T11 step 3. Spec §3.4/§7 ("derive the form-set **and SE/Sch-C expectations** from inputs").

T6's file list names the differential test (:68-153), the form-set map (:300-350), and the determinism carve-out; step 2 disposes of the form-set test, the attachment-order check, and byte-repro/identity. But `golden_packet.rs` has four more corpus-coupled tests the plan never mentions: `the_se_households_print_the_oracles_se_tax_onto_schedule_se` (:161-185, `checked == 3` at :181-185, and it compares printed SE L12 to `round(exact se_tax)` — the C1 residual again), `the_se_households_name_their_business_in_form_8995s_part_i_table` (:519-568, `checked == 3`), `every_filed_schedule_c_names_its_business_on_line_a` (:596-620, `checked == 3`), and `a_household_with_no_business_files_no_form_8995_row` (:574-586, fills every non-SE household). At T11 the counts go red (a generated corpus has far more than 3 SE households) and the un-carved loops add hundreds of ~150-250 ms fills — a second, independent way T11 cannot close green as written. **Fix:** add these to T6's scope: derive the SE/Sch-C counts from inputs (spec §7), and either fold their per-household assertions into the sharded differential loop's existing fills or carve them to the 12 anchors.

### I3 — T4's sharing mechanism is unsound as named: a top-level `tests/` file is its own crate, and the shown test uses helpers it cannot reach

**Touches:** T4 Files (:200), Step 1 (:211-221).

`crates/btctax-forms/tests/oracle_sweep_readback.rs` would be auto-compiled as an independent integration-test crate; `include!`-ing it from `golden_packet.rs` (the plan's stated option) compiles and runs its tests **twice** under `cargo nextest --workspace` and clippy `--all-targets -D warnings`. Worse, T4's shown Step-1 test calls `packet(&h)` and `form(&pkt, …)` — private helpers of `golden_packet.rs` (:40-55) that a sibling test crate cannot import, so the test as written does not compile in the file the plan names without duplicating the packet builder (the exact drift `golden_packet.rs:24-27` warns against). The repo already has the correct pattern: `tests/common/mod.rs` with `#![allow(dead_code)]`, consumed via `mod common;` by four test crates (verified: `kats.rs`, `overflow.rs`, `sp3.rs`, `sp3b.rs`). **Fix:** put `Sign`/`Blank`/`on_paper_signed`/`cell_or_zero` (and a shared `packet()` helper) in `tests/common/`, consumed by both `golden_packet.rs` and any new test file; keep their unit tests in `common` or in one owning test crate.

### I4 — T12's classification default is self-contradictory, and the "simplest" route reimplements the Tax Table and half-up rounding in Python

**Touches:** T12 Interfaces (:350). Spec §6.2, r2-M4 rationale.

T12 says the harness `--check` mode is "(preferred)", then immediately: *"simplest is to reuse the harness for btctax values and re-run the reproduction in Python."* The Python route means re-implementing `round_dollar` (half-away-from-zero — Python's `round()` is banker's, a guaranteed drift on `.50` values), the $25/$50 bin-midpoint Table, and the QDCGT worksheet — exactly the cross-boundary re-implementation the spec rejected for the AMT screen (D-2/r2-M4: "not a Python re-implementation … that could drift"). A drifted sweep classifier either spams false divergences or, on Table bins, silently absorbs real ones — blinding the discovery mechanism §9 exists to provide. **Fix:** make `--check` the mandated mechanism (one JSON-in/JSON-out extension of the T7 bin); delete the "simplest" sentence.

---

## Minor

### M1 — T5's green-claim mis-counts and omits the seventh divergence's disposition
T5 step 3 says "the 5 taxcalc Table divergences now absorbed." `DECLARED_DIVERGENCES` has **6** Table L16 entries (`golden_returns.rs:95-144` plus `single_crypto_business_se` at :192-202 — the spec's "+four more" undercount propagated) **and** a 7th: `single_miner_qbi…` L24, `agrees_with: "neither"` (:157-191). That one is not absorbed by any class — it **dissolves** because the comparison becomes `sum_round` of OTS's components (verified: `round(8354.59)+round(8477.73) = 16,833 =` btctax's printed L24; OTS total 16,832.32 no longer consulted). State both dispositions, and note the §6102 rationale prose should survive somewhere (the reproduction rule now embodies it).

### M2 — T7's "prefer src/bin" default trips the plan's own escape condition
`btctax-forms` has **no `serde_json` dependency** (Cargo.toml verified); the harness bin needs one, and bins take regular deps — so the src/bin form adds serde_json to the published library crate and ships an `oracle_harness` binary in `cargo package`/`cargo install btctax-forms`. That is the "pulls unwanted deps" condition T7 itself names as the trigger for the fallback. Either accept explicitly, gate the bin with `required-features`, or take the separate unpublished bin-crate fallback. (`btctax_core::tax::testonly` is a plain `pub mod` — `tax/mod.rs:24` — so reachability from a bin is fine, as claimed.)

### M3 — T10's covering-array guidance: the named dev-dep doesn't fit variable strength; recommend pinned triples + pairwise
`allpairspy` is pairwise-oriented; neither it nor a naive hand-rolled t-wise builder gives *variable* strength (t=3 only over two named triples, t=2 elsewhere) out of the box, and global t=3 over 8 axes blows the ~80-120 budget. The simple, verifiable construction that satisfies §5.1 exactly: the **full cartesian product over each named triple's axes** (≈30 + ≈12 rows after constraints) ∪ pairwise-with-constraints over the rest, deduplicated — trivially t=3-complete on the triples, checkable by a 10-line coverage assertion. Say this in T10 step 1 so the executor doesn't burn time on a real CA algorithm or a mis-fitting library.

### M4 — Pre-T11, the oracle-side `L16Operands` cannot be constructed; the predicates need `Option` and a stated fail-closed default
T3's signatures take bare `&L16Operands` for oracle ops, but until T11 the leaves (`qual_div_l3a`, `net_ltcg_qd_exclusive`) are `None` (T1). `stacking_ok`/`provenance_class_fires` must accept `Option<L16Operands>` (or be gated by the caller) with **absent ⇒ the class cannot fire** — a default of "true" would make the anti-world guard vacuous. The global constraint (:18) implies this; make it explicit in T3's Interfaces so the mutation-check has a named target.

---

## Nit

- **N1** (T1 step 1, :34): `testonly.rs` has **no** `#[cfg(test)] mod tests` today (verified) — "append to" should be "create".
- **N2** (T2 step 1, :91): `use crate::tax::return_inputs::FilingStatus;` is a private import path; the real re-export is `crate::tax::FilingStatus` (`tax/mod.rs:47`) or `crate::tax::types::FilingStatus`.
- **N3** (T2 step 1, :97): the comment "whole-dollar inputs ⇒ integral" is false for the SE components (baked `se_tax` = 8,477.73); harmless since the figures are flagged as placeholders to replace from the JSON, but the parenthetical will confuse.
- **N4** (T3, :145 vs :182): `LivenessLedger::dead()` is declared no-arg in Interfaces but called as `dead(&[...])` in the shown test — pick one.

---

## Verified (the load-bearing claims that hold)

- **Green-preserving order, T1–T10:** adding `#[serde(default)] Option<f64>` fields parses the current JSON unchanged (no `deny_unknown_fields`; serde defaults missing `Option` to `None`). T5 stays green on the 12 households: all six Table L16 divergences satisfy `consulted_table` on the reproduced operands (spot-checked: `single_crypto_business_se` bins 70,008.908 → midpoint 70,025 → **10,459** vs taxcalc 10,454.96, exactly as the plan's T3 test states; `single_qdcgt_both_slices` remainder 79,400 < ceiling; MFJ 253,943 ≥ ceiling refutes), the L24 entry dissolves under `sum_round` (M1), and every other line agrees at whole dollars. No task consumes a later task's signature; T7 precedes its T10/T12 consumers.
- **Cited mechanisms exist as claimed:** `TAX_TABLE_CEILING`/`regular_tax`/`qdcgt_line16` (`method.rs:21,60,74-91` — and `qdcgt_line16` already returns whole dollars via its final `round_dollar`, so `table_l16 == dec!(47031)` is type- and value-consistent); the worksheet consults the Table per operand (`worksheet_tax`, `:47-56`); `ty2024_table()`, `build_golden_household`, `golden_households` (`testonly.rs:88,367,480`); `extract_lines`/`verify_flat`/`no_unmapped_filled` exported via forms `testonly` (`lib.rs:405-407`); L7 leading-minus vs Sch D paren-magnitude (`printed.rs` L7 routing + doc); 8959/8960 rate-on-printed (`other_taxes.rs:167,173,320`); `AmtScreenTriggered` (`return_refuse.rs:161`); Python citations (`HOUSEHOLDS` :86-201, `taxcalc_run` :204, `_provenance` :291, `generated` :306, `_parse` :164, QD-inclusive `net_capital_gain` :290-294) all exact.
- **Runtime/sharding posture is sound:** nextest parallelizes per-`#[test]` shards; the plan correctly acknowledges T6's measurement is provisional and re-measures at T11 with a stated §8 fallback. T8-T12's offline correctness riding on the T11 re-bake is the spec's own design (baked-oracle hermeticity), not a plan defect.

## Strengths

The task decomposition is genuinely TDD-shaped with named anchors and real baked figures behind almost every shown assertion, and the inert-until-rebake `Option` gating is a clean mechanism for keeping eleven of thirteen boundaries green. The T2/T3 class machinery is specified to the function-signature level with correct math (every spot-checked literal reproduces from source), and the self-review's coverage map made the two genuine coverage holes (C1, I1) fast to isolate.

**NOT green** — resolve C1 and I1–I4 (C1 requires a schema/driver amendment before T8/T10/T11 are executed; I2 and I3 are scope additions to T6/T4; I4 and the Minors are wording-level) and re-review.
