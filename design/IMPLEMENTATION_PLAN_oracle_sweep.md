# Oracle Sweep — Implementation Plan (double-oracle differential testing, read from the filled PDF)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend btctax's existing paper-vs-oracle test (`golden_packet.rs`, already holding the filled 1040 against OpenTaxSolver for 12 households) into a scaled, deeper, **two-oracle** differential harness whose btctax side is read **off the filled IRS PDF**, held against both OTS and PSL Tax-Calculator, with a variable-strength generated corpus and a non-CI live sweep.

**Architecture:** New shared test-support in `btctax-core` (`tax/oracle_diff.rs`): the "reproduce btctax's §3.1 printing on the ORACLE's figures" helpers + the divergence-**class** machinery (per-oracle provenance classes, the taxcalc Tax-Table methodology class, class-stacking, class-liveness). `golden_returns.rs` (compute level) and `golden_packet.rs` (paper level, evolved) both consume it over the same baked `full_return_goldens.json`. The Python drivers (`ots_direct.py`/`gen_goldens.py`) gain the deeper lines + provenance leaves and a covering-array corpus generator with two bake-time-steered liveness cells; a test-only Rust harness binary assembles+fills+reads-back a scenario for the sweep and for refusal-free admission.

**Tech Stack:** Rust 2021 (`rust_decimal` `Usd`, `lopdf` read-back, `serde`/`serde_json`), Python 3 (offline only: OTS 2024 binaries + `taxcalc` venv). Reuses `btctax_core::tax::method::{qdcgt_line16, regular_tax, TAX_TABLE_CEILING}`, `testonly::{golden_households, build_golden_household, ty2024_table, ty2024_params}`, `btctax_forms::{fill_full_return, testonly::extract_lines, verify_flat}`.

**Spec:** `design/SPEC_oracle_sweep.md` (r5, GREEN 0C/0I). Section refs (§N) below are to that spec; review-finding tags (r4-I1, …) trace the reviewed rationale.

*Plan review: r1 (`…-r1.md`) 1C/4I → r2; r2 (`…-r2.md`) 0C/4I → r3; **r3 (`…-r3.md`) 0C/0I — GREEN, ready to execute** (its 3 Minors + 2 Nits folded post-green here). The caught-bug policy (`9fdb9cc`) is folded in. Review loop: r1 1C/4I → r2 0C/4I → r3 0C/0I.*

## Global Constraints

- **FROZEN — never edit:** `crates/btctax-core/src/tax/{types,compute,se}.rs`. **No change to the compute engine, the fillers, or the map TOMLs.** This plan READS and REPRODUCES btctax's printing via `method.rs::qdcgt_line16` + `ty2024_table()`; it never alters compute.
- **Hermetic gating CI:** fill + `extract_lines` + `verify_flat` are pure `lopdf`, offline; all oracle answers stay **baked** in `crates/btctax-core/tests/goldens/full_return_goldens.json`. Only the offline generator/sweep touch the OTS binary or the `taxcalc` venv — never `make check`.
- **`make check` (~6 s warm — `cargo nextest run --workspace` + clippy `-D warnings`, concurrent) is the gate and MUST stay green at every task boundary.** Respect the runtime budget: the differential loop is **sharded** across `#[test]`s; the byte-reproducibility and identity determinism loops stay on the **12 anchors** (§8).
- **Deeper-line and provenance assertions gate on `Option::Some`** — they are inert on the current baked JSON (fields default `None`) and activate only after the offline re-bake (T11). This is how each Rust task stays green before the corpus carries the data.
- **MFS deferred; AMT-triggering scenarios OUT; dependents/credits OUT; TY2024 only.** The domain is {Single, MFJ}, refusal-free (D-2), itemizing-wins when itemized (D-3).
- **Two-oracle common-mode limit is accepted** (an identical OTS+btctax Table bug is why taxcalc is in the design; not a defect to fix here).
- **Caught bugs file FOLLOWUPS — they are NOT fixed in this plan (user-mandated 2026-07-16).** If a corpus scenario (T11) or a sweep run (T12) surfaces a genuine btctax fill/compute bug, file a `FOLLOWUPS.md` entry (severity + owning phase) and pin the scenario as a **declared known-defect divergence** (btctax's current value asserted, labelled `KNOWN DEFECT → <FU-id>`, the oracles' correct figures beside it) so `make check` stays green with the bug tracked — never weaken/skip a test, never fix compute/fill here (§10).
- Reviews use **Fable** (standing user directive). Per task: TDD (write the failing test, watch it fail, implement, watch it pass, commit); mutation-check each new guard (delete it → a named test fails → restore via `cp` backup, never `git checkout` on uncommitted work). Fish shell: quote globs; `git commit -F -` via heredoc.

---

## The per-line reproduction table (§6.1 → §6.2 — the C1 spine)

**★ This table is load-bearing (Fable plan-r1 C1).** Each compared line is reproduced by pushing the
**oracle's** figures through btctax's own §3.1 printing (`printed.rs`/`other_taxes.rs`). Cross-footed and
rate-on-printed lines print `Σ round_dollar(component)` — **NOT** `round_dollar(exact_total)` — so the
oracle drivers must bake the **component leaves** those lines sum, or the reproduction degenerates to the
lawful Σround≠roundΣ residual with no class to absorb it (T11 goes red on ~¼ of SE cells). The executor
**derives each line's exact reproduction from source** (the "btctax prints" column points there); this
table fixes the pattern, the operands, and which oracle witnesses each line.

| §6.1 line | btctax prints (source) | Reproduction on oracle leaves | Witness |
|---|---|---|---|
| **AGI** 1040 L11 | L9 = Σ printed income (`printed.rs:518-542`); L11 = L9 − Sch1 L26 (= `round(half-SE)`, `printed.rs:341-357`) | `sum_round(income legs) − round_leaf(oracle_se_tax/2)`, where the **capital leg is the §1211-netted Sch-D→L7** (`net = stcg+ltcg`, floored at −3,000 when negative — `printed.rs:533-538`; or `round_leaf(oracle_sch_d_to_l7)` post-bake), **NOT** the raw gain inputs (r2-M1/N4) | OTS + taxcalc (each with its own `se_tax`) |
| **Taxable income** L15 | `(L11 − L13 QBI − L12 ded).max(0)` (cross-foot of printed lines) | reproduced-AGI − `round_leaf(oracle_deduction_taken)` − `round_leaf(oracle_qbi_deduction)` | OTS + taxcalc |
| **Tax** L16 | Table/QDCGT on **printed** L15 (`method.rs:74-91`) | **two-part** (§6.2b): structural `table_l16(reproduced ops)` + the per-oracle provenance/methodology classes | two-part |
| **QBI deduction** 1040 L13 / 8995 L15 (r2-I2) | printed 8995 chain: `min(round(20%×round(QBI)), round(20%×(round(TI_bq)−round(ncg))))` (`qbi.rs:195-211`) — **not a leaf** | `round_leaf(oracle_qbi_deduction)` (reuse the existing `qbi_deduction` field — baked today on both structs) | OTS + taxcalc; epsilon residual (exact-.50 / min-near-tie) → §10 triage, **not** a class |
| **Sch SE L12** | `round(L10 OASDI) + round(L11 Medicare)` (`printed.rs:231-233`) | `sum_round([oracle_se_l10, oracle_se_l11])` | **OTS single-witness** — taxcalc exposes only `setax` (exact total), a different quantity (§6.4 `Option`) |
| **8959 L18** | `round(L7) + round(L13)` (`other_taxes.rs:167,173,178`) | `sum_round([oracle_8959_l7, oracle_8959_l13])` | **OTS single-witness** — taxcalc `ptax_amc` is the exact total |
| **NIIT** 8960 L17 | `round(3.8% × printed base)` (`other_taxes.rs:320`) | **paper:** `round_leaf(oracle_niit)` — OTS single-witness, with a ±cents epsilon (OTS's L17 is computed on cents-MAGI, not printed operands) → §10 triage, not a class. taxcalc's exact `niit` is compared at the **compute level** (T5, exact-vs-both — r2-I4), not on paper. **Needs the T8 fix: OTS 8960 L5a must be the §1211-limited net (r2-I3).** | OTS (paper) + taxcalc (compute) |
| **Total tax** L24 | `round(L22)+round(L23)`; L22 = L16 + Sch2 (SE L12 + 8959 L18 + NIIT) (`printed.rs:289-292,627`) | `sum_round([reproduced L16, reproduced SE-L12, reproduced 8959-L18, round_leaf(oracle_niit)])` — **inherits** the SE/8959 cross-foots | **OTS single-witness** (taxcalc bundles payroll — already excluded today) |
| **Deduction taken** L12 | `round(std or itemized)` (leaf) | `round_leaf(oracle_deduction_taken)` | OTS + taxcalc |
| **SALT** Sch A L5e | `min(round(5d), 10000)` (leaf) | `round_leaf(oracle_salt_capped)` | OTS + taxcalc |
| **Sch D → 1040 L7** | signed leading-minus (`printed.rs:387-390`) | `round_leaf(oracle_sch_d_to_l7)`, sign per §6.3 | OTS + taxcalc |
| **8995 L12** | net-cap-gain cap | `round_leaf(oracle_qbi_cap_l12)` — **OTS single-witness / WEAK** (OTS's L12 is driver-hand-fed; §14.2 closure deferred to a follow-up — I1) | weak |

The four component leaves the cross-foots consume — OTS **Sch SE L10 (OASDI)** + **L11 (Medicare)** and
**8959 L7** + **L13** — are all already in `ots_direct.py`'s `_parse` reach (it captures every `Lxx`);
T1 bakes them, T8 emits them, T2's `sum_round` consumes them. Taxcalc has no SE/8959 leg split, so those
lines are OTS-single-witness by §6.4's `Option` rule (T9 leaves them `None`).

**★ This table is the PAPER level (T6).** The single-witness dispositions above are paper-level — where
btctax prints a cross-foot (`Σround(legs)`) and taxcalc exposes only an exact total. At the **COMPUTE level
(T5, `golden_returns.rs`)** the leaf totals HEAD holds against both engines today **at §3.1 whole dollars**
(bit-equal on SE tax/NIIT/Add'l-Medicare; within ~5¢ on TI/AGI/QBI, where OTS line-rounds the 8995 chain
and taxcalc does not — so HEAD's `round_dollar`-both-sides is what passes, r3-M1) — **SE
tax, Additional Medicare, NIIT, taxable income, AGI, QBI deduction** — keep their **exact-vs-BOTH-oracles**
comparison (r2-I4; Add'l Medicare has no other taxcalc witness). Only **L16** (two-part) and **L24** (the
`sum_round` cross-foot, OTS-single-witness) use the reproduction at the compute level; their exact chains
lawfully differ in cents across engines, so exact-vs-exact is NOT forced there.

**Pre-T11 fallback (r3-M2).** A reproduction whose component leaves are still `None` (before the T11
re-bake) falls back to `round_leaf` of that component's baked per-line **total** — the HEAD shape
(`golden_packet.rs:120-123`), proven green on the anchors since no anchor's legs flip; the leg/leaf form
takes over when the operands bake at T11. So the T5/T6 step-3 dispositions ("L24 dissolves", "headline lines
match OTS as before") are literally true at every task boundary, not only post-bake.

---

### Task 1: Schema — deeper-line + provenance-leaf fields on the oracle structs

**Files:**
- Modify: `crates/btctax-core/src/tax/testonly.rs:396-423` (`ExpectedOts`, `ExpectedTaxcalc`)
- Test: `crates/btctax-core/src/tax/testonly.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces: `ExpectedOts`/`ExpectedTaxcalc` gain `Option<f64>` fields (all `#[serde(default)]`, so the current JSON parses unchanged): `deduction_taken`, `salt_capped` (Sch A L5e), `sch_d_to_l7` (1040 L7), `qbi_cap_l12` (8995 L12 — **OTS single-witness/WEAK**, I1: OTS's is driver-hand-fed; a schema comment must say so, and the §14.2 closure is filed as a follow-up); and the **provenance leaves** `qual_div_l3a`, `net_ltcg_qd_exclusive` (the §1(h) subterm `max(0, min(ltcg, ltcg+stcg))`, **QD-EXCLUSIVE** — r5-N2). **Cross-foot component legs (C1) — `ExpectedOts` only:** `se_l10_oasdi`, `se_l11_medicare` (Sch SE), `f8959_l7`, `f8959_l13` — the legs Sch SE L12 / 8959 L18 sum. Taxcalc has no leg split, so it leaves them `None` and those lines are OTS-single-witness. `taxable_income` (exact cents) is already present on both — reuse as the TI leaf. `ExpectedTaxcalc` additionally carries `total_tax: Option<f64>` (absent today — §6.4 M-4) and the shared deeper/provenance fields (NOT the SE/8959 legs).

- [ ] **Step 1: Write the failing test** (create a `#[cfg(test)] mod tests` in `testonly.rs` — none exists today, N1)

```rust
#[test]
fn current_goldens_parse_with_optional_deeper_fields_absent() {
    let hs = golden_households(); // parses GOLDEN_RETURNS_JSON
    let h = &hs[0];
    // The new deeper/provenance fields are absent in today's JSON ⇒ None, not a parse error.
    assert!(h.expected_ots.qbi_cap_l12.is_none());
    assert!(h.expected_ots.net_ltcg_qd_exclusive.is_none());
    assert!(h.expected_taxcalc.total_tax.is_none());
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test -p btctax-core --lib testonly::tests::current_goldens_parse_with_optional_deeper_fields_absent` → FAIL (fields don't exist).

- [ ] **Step 3: Add the fields.** In `ExpectedOts` and `ExpectedTaxcalc`, add (each preceded by `#[serde(default)]`):

```rust
    #[serde(default)] pub deduction_taken: Option<f64>,       // 1040 L12
    #[serde(default)] pub salt_capped: Option<f64>,           // Sch A L5e
    #[serde(default)] pub sch_d_to_l7: Option<f64>,           // 1040 L7 (signed)
    #[serde(default)] pub qbi_cap_l12: Option<f64>,           // 8995 L12 — OTS single-witness/WEAK (I1): driver-hand-fed, NOT an independent check; §14.2 closure = follow-up
    // provenance leaves for the §6.2(b) predicate (Table_btctax inputs):
    #[serde(default)] pub qual_div_l3a: Option<f64>,          // 1040 L3a
    #[serde(default)] pub net_ltcg_qd_exclusive: Option<f64>, // §1(h) term, QD-EXCLUSIVE (r5-N2)
```
and in `ExpectedOts` **only**, the C1 cross-foot legs (taxcalc has no split):
```rust
    #[serde(default)] pub se_l10_oasdi: Option<f64>,          // Sch SE L10 (OASDI leg)
    #[serde(default)] pub se_l11_medicare: Option<f64>,       // Sch SE L11 (Medicare leg)
    #[serde(default)] pub f8959_l7: Option<f64>,              // 8959 L7 leg
    #[serde(default)] pub f8959_l13: Option<f64>,             // 8959 L13 leg
```
and in `ExpectedTaxcalc` only: `#[serde(default)] pub total_tax: Option<f64>,` (OTS's is required `f64`; taxcalc's is optional — §6.4 M-4). `taxable_income` (exact cents) is already present on both; reuse it as the TI leaf.

- [ ] **Step 4: Run to verify it passes** — same command → PASS. Then `make check` → green (existing tests untouched; JSON parses).

- [ ] **Step 5: Commit** — `feat(oracle-sweep): optional deeper-line + provenance-leaf oracle fields (T1)`

---

### Task 2: `oracle_diff` — reproduce btctax's §3.1 printing on the oracle's figures

**Files:**
- Create: `crates/btctax-core/src/tax/oracle_diff.rs`
- Modify: `crates/btctax-core/src/tax/mod.rs` (add `pub mod oracle_diff;`)
- Test: in `oracle_diff.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `method::{qdcgt_line16, regular_tax, worksheet-ceiling TAX_TABLE_CEILING}`, `testonly::ty2024_table` (the `TaxTable` carrying the ordinary schedule + LTCG breakpoints per filing status), `conventions::{round_dollar, Usd}`.
- Produces (all `pub`, test-support): the **per-line reproduction** of the printed value on an oracle's leaves —
  - `round_leaf(oracle_line: f64) -> Usd` = `round_dollar(usd(x))` (Leaf pattern);
  - `sum_round(components: &[f64]) -> Usd` = `Σ round_dollar` (Cross-footed pattern — the `golden_packet.rs:120-123` L24 pattern generalized). **Its `components` are the oracle's per-line LEGS from the reproduction table (C1), never a single exact total** — SE L12 = `sum_round([se_l10_oasdi, se_l11_medicare])`, 8959 L18 = `sum_round([f8959_l7, f8959_l13])`, L24 = `sum_round` of the reproduced L16 + reproduced SE-L12 + reproduced 8959-L18 + `round_leaf(oracle_niit)`. `sum_round(&[exact_total])` is a bug — it degenerates to `round(exact)`, the lawful residual with no class;
  - `rate_on_printed(rate: Usd, printed_operand: Usd) -> Usd` = `round_dollar(rate * printed_operand)` (Rate-on-printed pattern — `other_taxes.rs` 8959 L7/L13, 8960 L17);
  - `table_l16(status, ti: Usd, qd_l3a: Usd, net_ltcg_qd_excl: Usd) -> Usd` = `qdcgt_line16(schedule, bp, ti, qd, net_ltcg)` (Tax-table pattern; `Table_btctax`).
  - `consulted_table(status, ti, qd_l3a, net_ltcg_qd_excl) -> bool` — true iff any worksheet operand (the ordinary remainder `L5 = max(0, ti − (qd+ltcg))` OR the full `ti`) is `< TAX_TABLE_CEILING` (r3-I1 methodology condition; computed from the same operands `qdcgt_line16` consumes — `method.rs:83-89`).

- [ ] **Step 1: Write the failing tests** (pin the reproduction against baked anchor figures)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::FilingStatus; // the re-export (N2), not crate::tax::return_inputs::

    // ★ C1: sum_round is Σround(legs), NOT round(exact_total). Pin the cross-foot with a SYNTHETIC
    // flipping pair (mirrors the repo's own KAT, other_taxes.rs:80-85): 274.50 → 275, 499.50 → 500,
    // so the legs sum to 775, but round(774.00) = 774. The legs are NOT baked until T11 and no anchor
    // flips anyway, so this uses literals — do NOT read the JSON here (r2-I1).
    #[test]
    fn sum_round_cross_foots_the_legs_not_the_exact_total() {
        assert_eq!(sum_round(&[274.50, 499.50]), dec!(775));                    // round(274.50)+round(499.50)
        assert_ne!(sum_round(&[274.50, 499.50]), round_leaf(274.50 + 499.50)); // ≠ round(774.00) = 774 — the whole point
    }

    // Table_btctax reproduces OTS's exact-cents L16 at whole dollars for the above-ceiling SE anchor.
    #[test]
    fn table_l16_reproduces_ots_above_ceiling() {
        // mfj_se_over_the_addl_medicare_threshold: OTS TI 253_942.94, L16 47_031.31 (baked).
        let got = table_l16(FilingStatus::Mfj, usd(253_942.94), usd(0.0), usd(0.0));
        assert_eq!(got, dec!(47031)); // round_dollar(47_031.31) — no preferential income ⇒ pure TCW
    }

    // consulted_table: true when the ordinary remainder is below the ceiling (single_qdcgt_both_slices),
    // false when every operand is at/above it (a pure high-TI ordinary household).
    #[test]
    fn consulted_table_tracks_the_worksheet_operands() {
        // TI 112_400, QD 8_000, net-LTCG(qd-excl) 25_000 ⇒ remainder 79_400 < ceiling ⇒ true.
        assert!(consulted_table(FilingStatus::Single, usd(112_400.0), usd(8_000.0), usd(25_000.0)));
        // TI 253_943, no preferential ⇒ remainder = TI ≥ ceiling ⇒ false.
        assert!(!consulted_table(FilingStatus::Mfj, usd(253_943.0), usd(0.0), usd(0.0)));
    }
}
```
*(The `table_l16` / `consulted_table` literals ARE baked figures — read the exact values from `full_return_goldens.json` during implementation; the `sum_round` test above is a synthetic-literal unit test and needs no JSON read.)*

- [ ] **Step 2: Run to verify they fail** — `cargo test -p btctax-core --lib oracle_diff` → FAIL (module absent).

- [ ] **Step 3: Implement `oracle_diff.rs`.** Header doc-comment marks it test-support ("Reproduces btctax's §3.1 printed chain on an independent oracle's figures — the seam the differential tests hold the paper against; see `design/SPEC_oracle_sweep.md` §6.2"). Implement the five reproductions + `consulted_table` using `qdcgt_line16` and `TAX_TABLE_CEILING`. `usd(f64)` = `Usd::try_from(x).expect("finite oracle figure")` (mirror `golden_usd`). Pull the per-status `OrdinarySchedule` + `LtcgBreakpoints` from `ty2024_table()`.

- [ ] **Step 4: Run to verify they pass**; then `make check` → green.

- [ ] **Step 5: Commit** — `feat(oracle-sweep): oracle_diff §3.1-printing-on-oracle-figures reproduction (T2)`

---

### Task 3: `oracle_diff` — the divergence-class machinery (the intricate core)

**Files:**
- Modify: `crates/btctax-core/src/tax/oracle_diff.rs`
- Test: same file

**Interfaces:**
- Produces:
  - `enum OracleId { Ots, Taxcalc }`;
  - `struct L16Operands { status: FilingStatus, ti: Usd, qd_l3a: Usd, net_ltcg_qd_excl: Usd }` (an oracle's own leaves, exact);
  - `fn taxcalc_methodology_class(reproduced_ops: &L16Operands) -> bool` — the class **condition**: `consulted_table(reproduced_ops)` (r3-I1). Condition-only, no value check (r4-N1 declined — a value check under-absorbs mixed methodology+provenance households; backstopped by the OTS provenance conjunct under stacking).
  - `fn provenance_class_fires(oracle_ops: Option<&L16Operands>, reproduced_ops: &L16Operands, oracle_l16: f64) -> bool` — the **per-oracle provenance predicate** (§6.2(b), r4-I1): `table_l16(oracle_ops) == round_leaf(oracle_l16)` **AND** `table_l16(reproduced_ops) != round_leaf(oracle_l16)`. The first conjunct is the falsifiable witness — a real `Table_btctax` semantics bug fails it and stays red. **`oracle_ops: Option` — absent (the L16 leaves are not yet baked, i.e. pre-T11) ⇒ returns `false`, the class CANNOT fire (M4)**; a default of `true` would make the anti-world guard vacuous — this is the named mutation-check target.
  - `fn stacking_ok(figure: Usd, ots_l16: f64, taxcalc_l16: Option<f64>, ots_ops: Option<&L16Operands>, taxcalc_ops: Option<&L16Operands>, reproduced_ops: &L16Operands, known_defect: Option<&KnownDefect>) -> bool` — the guard's class-form (r3-I2a). `figure` is btctax's value on the line — the on-paper value at the paper level (T6) or the compute figure at the compute level (T5) (r3-N2). If the figure agrees with an oracle that oracle needs no class; a both-oracle disagreement passes **only when each dissenting oracle's diff independently matches its own class** (taxcalc: methodology OR its provenance; OTS: its provenance). Replaces the old `agrees_with:"neither"` + `outlier_alt` stack (`golden_returns.rs:41-53,358-372`). **One sanctioned exception (§10, user-mandated):** a both-oracle disagreement also passes when a `KnownDefect { fu_id: &'static str, btctax_value: Usd }` is declared for that `(household, line)` and `figure == btctax_value` — pinning btctax's current WRONG value against an open `FOLLOWUPS.md` id. A known-defect is a **separate, loudly-named category, never a lawful class**, and a stale one (btctax's value moved — bug fixed or changed) fails, forcing the entry's removal.
  - `struct KnownDefect { fu_id: &'static str, btctax_value: Usd }` — the §10 caught-bug pin.
  - `struct LivenessLedger { fired: BTreeSet<&'static str>, pinned: BTreeSet<&'static str> }` with `fn record_fire(class)`, `fn declare_pinned(class)`, and `fn dead(&self, declared: &[&'static str]) -> Vec<&'static str>` (takes the declared-class list — N4) = declared classes neither fired nor pinned (r3-I2b, predicate analogue of `golden_returns.rs:388-401`).

- [ ] **Step 1: Write the failing tests** — pin fire/refute against the named anchors and the composition (§6.2(b)/§6.4):

```rust
// taxcalc methodology class fires on single_qdcgt_both_slices (remainder below ceiling), the anchor
// the old "TI < $100k" gloss wrongly excluded (r3-I1).
#[test]
fn methodology_class_fires_on_qdcgt_both_slices() {
    let ops = L16Operands { status: FilingStatus::Single, ti: usd(112_400.0), qd_l3a: usd(8_000.0), net_ltcg_qd_excl: usd(25_000.0) };
    assert!(taxcalc_methodology_class(&ops));
}

// Below the ceiling the taxcalc PROVENANCE conjunct-1 fails (Table_btctax bins; taxcalc uses the schedule)
// ⇒ the provenance class cannot fire/over-absorb there (§6.4 composition).
#[test]
fn taxcalc_provenance_cannot_fire_below_ceiling() {
    // single_crypto_business_se: taxcalc TI 70_008.908, L16 10_454.96 (baked); Table_btctax bins to 10_459.
    let ops = L16Operands { status: FilingStatus::Single, ti: usd(70_008.908), qd_l3a: usd(0.0), net_ltcg_qd_excl: usd(0.0) };
    assert!(!provenance_class_fires(Some(&ops), &ops, 10_454.96));
}

// A real Table_btctax semantics bug fails conjunct-1 ⇒ NOT absorbed (teeth). Simulated by feeding an
// oracle L16 that btctax's own lookup does NOT reproduce on the oracle's operands.
#[test]
fn provenance_class_keeps_teeth_against_a_semantics_mismatch() {
    let ops = L16Operands { status: FilingStatus::Mfj, ti: usd(253_942.94), qd_l3a: usd(0.0), net_ltcg_qd_excl: usd(0.0) };
    assert!(!provenance_class_fires(Some(&ops), &ops, 99_999.0)); // 99,999 ≠ Table_btctax(253,942.94)
}

// LivenessLedger: a declared-but-neither-fired-nor-pinned class is "dead".
#[test]
fn liveness_flags_a_dead_class() {
    let mut l = LivenessLedger::default();
    l.declare_pinned("ots_provenance");        // held by a §5.1 pinned cell
    l.record_fire("taxcalc_methodology");
    // "taxcalc_provenance" declared below but neither fired nor pinned ⇒ dead.
    assert_eq!(l.dead(&["taxcalc_methodology","ots_provenance","taxcalc_provenance"]), vec!["taxcalc_provenance"]);
}
```
*(Read the exact baked taxcalc/OTS figures from the JSON before finalizing the literals.)*

- [ ] **Step 2: Run to verify they fail** → FAIL (types absent).

- [ ] **Step 3: Implement** the enums, predicates, `stacking_ok`, and `LivenessLedger` per the Interfaces. Keep `provenance_class_fires` a pure two-conjunct function over `table_l16`/`round_leaf`. `stacking_ok` mirrors the guard at `golden_returns.rs:358-372` but consults the class predicates instead of `agrees_with`.

- [ ] **Step 4: Run to verify they pass; mutation-check** each predicate (e.g. drop conjunct-1 of `provenance_class_fires` → `provenance_class_keeps_teeth_against_a_semantics_mismatch` fails → restore). Then `make check` → green.

- [ ] **Step 5: Commit** — `feat(oracle-sweep): divergence-class machinery — methodology + per-oracle provenance + stacking + liveness (T3)`

---

### Task 4: Forms-side on-paper read-back — the sign table and blank regimes

**Files:**
- Modify/Create: `crates/btctax-forms/tests/common/mod.rs` — the repo's **existing** shared-test module (`#![allow(dead_code)]`, consumed via `mod common;` by `kats.rs`/`overflow.rs`/`sp3.rs`/`sp3b.rs`). Put `Sign`/`Blank`/`on_paper_signed`/`cell_or_zero` **and** a shared `packet(&GoldenHousehold) -> Vec<NamedForm>` / `form(&pkt, name)` here — moved out of `golden_packet.rs`'s private fns (`:40-55`) so both consumers share ONE packet builder, not a copy (the drift `golden_packet.rs:24-27` warns against). **I3:** a top-level `tests/*.rs` file is its own integration-test crate — do NOT `include!` it (that compiles + runs its tests twice under `--all-targets`) and it cannot reach `golden_packet.rs`'s privates; `common/` is the sanctioned share.
- Test: `crates/btctax-forms/tests/oracle_sweep_readback.rs` — `mod common;` then unit-tests the helpers.

**Interfaces:**
- Consumes: `btctax_forms::testonly::extract_lines`, `common::packet`.
- Produces:
  - `fn on_paper_signed(cells: &BTreeMap<String,String>, key: &str, sign: Sign) -> Option<i64>` — parse a filled cell to a signed integer per its sign convention (§6.3): `Sign::Leading` (1040 L7, a leading-minus cell — `printed.rs:387-390`), `Sign::ParenMagnitude` (Sch D L6/14/21 — magnitude in a pre-printed parenthesized box, negate), `Sign::Unsigned`. An unparseable present value returns `Err`-panics with the raw string (parse discipline, §6.3).
  - `enum Blank { PresentZero, AbsentIsZero }` and `fn cell_or_zero(cells, key, regime) -> i64` — `PresentZero` asserts the key is present-and-`"0"` (dropped-line detection, `golden_packet.rs:104-119`); `AbsentIsZero` reads an absent key as 0 (absent-form line).

- [ ] **Step 1: Write the failing test** — fill the capped-loss anchor and read 1040 L7 as −3000 (signed), and Sch D L21 as −3000 (paren-magnitude):

```rust
mod common; // the shared tests/common/mod.rs
use common::{packet, form, on_paper_signed, Sign};
use btctax_core::tax::testonly::golden_households;
use btctax_forms::testonly::{extract_lines, F1040_MAP_2024, SCHEDULE_D_MAP_2024};

#[test]
fn line7_is_signed_and_schedule_d_is_parenthesized_magnitude() {
    let h = golden_households().into_iter().find(|h| h.name == "single_capital_loss_capped").unwrap();
    let pkt = packet(&h);
    let f1040 = extract_lines(&form(&pkt,"f1040").bytes, F1040_MAP_2024).unwrap();
    assert_eq!(on_paper_signed(&f1040, "line7a", Sign::Leading), Some(-3000)); // f1040 map keys it line7a (r3-N1)
    let sd = extract_lines(&form(&pkt,"schedule_d").bytes, SCHEDULE_D_MAP_2024).unwrap();
    assert_eq!(on_paper_signed(&sd, "line21", Sign::ParenMagnitude), Some(-3000));
}
```

- [ ] **Step 2: Run to verify it fails** → FAIL.

- [ ] **Step 3: Implement** `on_paper_signed` / `cell_or_zero` / `Sign` / `Blank`. (Confirm the exact `line7`/`line21` map keys and the on-paper string form against the filled anchor while implementing.)

- [ ] **Step 4: Run to verify it passes**; `make check` → green.

- [ ] **Step 5: Commit** — `feat(oracle-sweep): on-paper sign table + blank regimes (T4)`

---

### Task 5: Rework `golden_returns.rs` (compute level) onto the class machinery + full line set

**Files:**
- Modify: `crates/btctax-core/tests/golden_returns.rs` (replace `Divergence`/`DECLARED_DIVERGENCES` :35-213 and the comparison loop :238-411)

**Interfaces:**
- Consumes: `oracle_diff::*` (T2/T3), the (still 12-household) `golden_households()`.
- Produces: the compute-level differential — btctax's compute + printed-chain figures vs the oracles across the full line set, adjudicated by the class machinery. This is the §6.2(b) **Table-semantics witness** (r2-I2). **Two-level rule (r2-I4):** the leaf totals HEAD holds against both engines today **at §3.1 whole dollars** (bit-equal on SE tax/NIIT; within ~5¢ on TI/AGI/QBI where the 8995 chain line-rounds — HEAD's `round_dollar`-both-sides is what passes, so keep those rows verbatim, r3-M1) — **SE tax, Additional Medicare, NIIT, taxable income, AGI, QBI deduction** — keep their **exact-vs-BOTH-oracles** comparison (as HEAD, `golden_returns.rs:249-297`; no new fields, and this preserves taxcalc's independent witness — Add'l Medicare has no other). Only **L16** (§6.2 two-part; taxcalc's exact-schedule divergence absorbed by the methodology class) and **L24** (the `sum_round` cross-foot of OTS's components, OTS-single-witness — taxcalc bundles payroll, excluded today) use the reproduction; do **NOT** force exact-vs-exact on L16/L24 (their exact chains lawfully differ in cents across engines). Deeper-line comparisons gate on `Option::Some` (inert until T11); provenance-class liveness is deferred to T11 (the methodology class is live via the 6 Table anchors).

- [ ] **Step 1: Characterize green first.** Run `cargo test -p btctax-core --test golden_returns` and note it passes today (baseline).
- [ ] **Step 2: Write the new comparison** — replace the per-household `DECLARED_DIVERGENCES` array with the class calls, **per the two-level rule** (Produces): the six cent-exact totals (SE tax, Add'l Medicare, NIIT, TI, AGI, QBI deduction) stay **exact-vs-both-oracles** (as HEAD); **L16** uses the §6.2 two-part (`table_l16(reproduced ops)` + `stacking_ok` over the methodology/provenance classes); **L24** compares btctax's **printed** L24 (`printed.f1040.line24` — `golden_returns.rs` already builds the printed chain, `:243,297`; r2-M2 — NOT the exact `ar` figure) against `sum_round` of OTS's components, OTS-single-witness. For each household×line: if the figure agrees with every opinionated oracle → continue; else require `stacking_ok(...)` and `LivenessLedger::record_fire` the class(es) that absorbed it. Register the methodology class live; **do not** yet assert provenance-class liveness (comment: "enabled in T11 with the pinned cells"). Keep the anti-world guarantee via `stacking_ok`. The deeper-line rows gate on `Option::Some` (`if let Some(x) = h.expected_ots.qbi_cap_l12 { … }`) — inert until T11.
- [ ] **Step 3: Run** `cargo test -p btctax-core --test golden_returns` → PASS on the 12 households. Disposition of today's `DECLARED_DIVERGENCES` (M1): the **6** taxcalc Table-L16 entries (`golden_returns.rs:95-144` + `single_crypto_business_se` `:192-202`) are now absorbed by `taxcalc_methodology_class`; the **7th** (`single_miner_qbi…` L24 `agrees_with:"neither"` `:157-191`) **dissolves** — the L24 `sum_round` reproduction equals btctax's printed L24 from OTS's own components, so OTS's exact total is never consulted (the §6102 rounding rationale now lives in the reproduction rule, not a per-household entry). Mutation-check: break `stacking_ok` (force `true`) → a synthetic both-disagree test fails → restore.
- [ ] **Step 4:** `make check` → green.
- [ ] **Step 5: Commit** — `refactor(oracle-sweep): golden_returns onto divergence classes + full line set, 12-household green (T5)`

---

### Task 6: Evolve `golden_packet.rs` — full line set off the PDF, both oracles, derived form-sets, sharding

**Files:**
- Modify: `crates/btctax-forms/tests/golden_packet.rs` — the differential test (`:68-153`); the hand-written form-set map (`:300-350`); the anchors-only determinism carve-out; **AND the four other corpus-coupled tests the plan must also migrate (I2):** `the_se_households_print_the_oracles_se_tax_onto_schedule_se` (`:161-185`, `checked==3`, and it compares printed SE L12 to `round(exact se_tax)` — the C1 residual, now fixed to `sum_round([se_l10_oasdi, se_l11_medicare])`), `the_se_households_name_their_business_in_form_8995s_part_i_table` (`:519-568`, `checked==3`), `every_filed_schedule_c_names_its_business_on_line_a` (`:596-620`, `checked==3`), and `a_household_with_no_business_files_no_form_8995_row` (`:574-586`, fills every non-SE household). Left as-is they break at T11 (a generated corpus has ≫3 SE households; the `checked==3` counts go red and the un-carved fills blow the budget).

**Interfaces:**
- Consumes: `oracle_diff::*` (T2/T3), `on_paper_signed`/`cell_or_zero` (T4), `extract_lines`, `verify_flat`/`no_unmapped_filled`.
- Produces: the paper-level differential — every headline + deeper line read off the filled packet via `extract_lines`, held against BOTH oracles via the reproduction helpers + classes; **three-way localization** (`oracle / btctax-internal / btctax-on-paper` in the failure line, §6.5); the form set **derived from inputs** vs the documented triggers with a **KAT that reproduces the 12 anchors' hand-written sets** (r3-M1); the differential loop **sharded** across N `#[test]`s (`#[test] fn diff_shard_0()`, …, dispatch by `household_index % N`) so nextest parallelizes (§8, r2-M1). Deeper-line rows gate on `Some` (inert until T11).

- [ ] **Step 1: Write the failing KAT** — `derived_form_set_reproduces_the_twelve_anchors`: for each of the 12 named anchors, `derive_form_set(&h.inputs)` equals the hand-written set (kept as pinned data). → FAIL until `derive_form_set` exists.
- [ ] **Step 2: Implement** `derive_form_set(inputs) -> BTreeSet<&str>` from the triggers (Sch B > $1,500; 8959 $200k/$250k; 8995 with QBI; Sch D with gains; Sch A when itemized; Sch 1/2/3/SE/C as their carriers require — mirror `golden_packet.rs:300-350`'s reasoning), the sign/blank read-back, and the two-oracle comparison via `oracle_diff` **per the C1 reproduction table** (SE L12 = `sum_round([se_l10_oasdi, se_l11_medicare])`, OTS-single-witness — this replaces the `:161` test's `round(exact se_tax)` comparison; 8959 L18 likewise). Move the whole-corpus `each_golden_packet_carries…` check to the derived set. **Migrate the four I2 tests:** derive their SE/Sch-C predicates from inputs (`inputs.self_employment_income > 0`) instead of the hard `checked==3`, and fold their per-household assertions (SE-L12-onto-Sch-SE, 8995 Part-I business name, Sch-C line-A, no-8995-row) **into the sharded differential loop's existing fills** so they cost no extra packet builds. The attachment-sequence-order check **rides the differential loop's fills** (r3-N1); byte-repro + identity stay on the 12 anchors.
- [ ] **Step 3: Run** the forms tests → PASS on the 12 households (headline lines match OTS as before; deeper rows inert). Mutation-check the derived-set KAT (perturb one trigger threshold → the anchor KAT fails → restore).
- [ ] **Step 4:** `make check` → green (measure the forms test wall-clock; confirm the shard count keeps it within budget — §8).
- [ ] **Step 5: Commit** — `refactor(oracle-sweep): golden_packet — full line set off the PDF, two oracles, derived form-sets, sharded (T6)`

---

### Task 7: The §9 test-only harness binary (assemble + fill + read-back a scenario)

**Files:**
- Create: **a small unpublished bin crate `crates/btctax-oracle-harness`** (`publish = false`), NOT `btctax-forms/src/bin/` (M2). Reason: the harness needs `serde_json`, which `btctax-forms` does **not** depend on (Cargo.toml verified) — a `src/bin` would add `serde_json` to the published library *and* ship an `oracle_harness` binary in `cargo install btctax-forms`, the very "unwanted deps" condition to avoid. The bin-crate depends on `btctax-forms` + `btctax-core` (`testonly` is a plain `pub mod` — `tax/mod.rs:24` — so reachable) + `serde_json`; add it to the workspace members.
- Test: `crates/btctax-oracle-harness/tests/smoke.rs`

**Interfaces:**
- Consumes: `testonly::{build_golden_household, ty2024_params, ty2024_table}`, `assemble_printed_return`, `fill_full_return`, `extract_lines`, and (for `--check`) `oracle_diff`.
- Produces: a CLI that reads a household JSON (the `GoldenInputs` shape) on stdin, assembles+fills the packet, reads back the full compared line set with `extract_lines`, and prints `{ "refused": bool, "lines": { "1040.line11": "...", ... } }` on stdout. Shared by the sweep (T12) and the Python D-2 refusal-free admission (T10). "Refused" ⇒ assembly returned a refusal (AMT screen etc.) — the D-2 signal.
- Also exposes a **`--check` mode (I4):** given a scenario + both oracles' figures as JSON on stdin, it runs the `oracle_diff` reproduction + classification **in Rust** and returns the per-line verdict, so the sweep (T12) never re-implements `round_dollar` (Python's `round()` is banker's — drifts on `.50`), the Table, or the QDCGT in Python.

- [ ] **Step 1: Write the smoke test** — pipe the `single_w2_only_standard` inputs JSON through the harness, assert `refused == false` and `lines["1040.line11"]` equals the baked OTS AGI. → FAIL.
- [ ] **Step 2: Implement** the bin: deserialize `GoldenInputs`, build via the same path `build_golden_household` uses, assemble (`assemble_printed_return` — on `Err` emit `{"refused":true}`), fill, `extract_lines` each form, emit the flattened `form.line → string` map.
- [ ] **Step 3: Run the smoke test → PASS**; `make check` → green.
- [ ] **Step 4: Commit** — `feat(oracle-sweep): oracle_harness test-only bin — assemble+fill+read-back a scenario (T7)`

---

### Task 8: Extend `ots_direct.py` — deeper lines + provenance leaves

**Files:**
- Modify: `scripts/oracle/ots_direct.py` (the `evaluate` return dict :348-359; it already `_parse`s every `Lxx` at :164-171)

**Interfaces:**
- Produces: `evaluate` returns, in addition to today's keys, `deduction_taken` (1040 L12), `salt_capped` (Sch A L5e when itemized), `sch_d_to_l7` (1040 L7, **signed**), `qbi_cap_l12` (marked **single-witness/WEAK** — it is the driver's hand-fed `round(net_capital_gain)`, I1; keep baking it but T5/T6 treat it single-witness, and file the §14.2 closure — derive OTS's L12 from OTS's own Schedule-D output — as a follow-up **with a severity + owning phase, STANDARD_WORKFLOW §4** — r2-N3), the provenance leaves `qual_div_l3a` (= `h["qualified_dividends"]`), **`net_ltcg_qd_exclusive` = `max(0.0, min(ltcg, ltcg+stcg))`** (QD-EXCLUSIVE subterm, NOT the QD-inclusive `net_capital_gain` at `:292-294` — r5-N2), and **the C1 cross-foot legs** `se_l10_oasdi` = `se.get("L10")`, `se_l11_medicare` = `se.get("L11")` (the SE solver's printed OASDI/Medicare lines), `f8959_l7` = `f8959.get("L7")`, `f8959_l13` = `f8959.get("L13")` (the 8959 solver's legs) — all already in `_parse`'s reach (`:164-171` captures every `Lxx`). `taxable_income` is already the exact-cents TI leaf.
- **★ r2-I3 — fix the OTS Form 8960 feed (a driver correctness bug, not just an emit):** `ots_direct.py:341` currently sets 8960 `L5a = max(ltcg,0) + max(stcg,0)`, which **never applies the §1211 loss limitation** btctax correctly applies (`other_taxes.rs:219-222,308`: a §1211-limited loss is NEGATIVE and REDUCES NII). On a capped-loss × NIIT-firing cell (pairwise-guaranteed) OTS's NII is $3,000 too high → OTS's NIIT is wrong **by construction**, and T11 would file a FALSE btctax bug + pin btctax's correct value as a KNOWN DEFECT. **Set 8960 `L5a` to the §1211-limited net figure the driver already computes for `sch_d_to_l7`** (the capped Sch-D→L7 value). Leave `L13 = p1.get("L11")` (pass-1 cents-MAGI) as today — hence NIIT is a paper-level ±cents epsilon → §10 triage, NOT "the printed L17" (the old gloss is dropped from the table). **Also widen the 8960 _trigger_ gate (r3-M3):** `ots_direct.py:325-328` gates the 8960 run on `investment = taxable_interest + ordinary_dividends + max(ltcg, 0)` — STCG never triggers it, so a net-STCG-only-over-threshold cell gets a false OTS `niit = 0`. Change it to `investment = interest + dividends + max(sch_d_net, 0)` (the same §1211-limited figure the L5a fix feeds).

- [ ] **Step 1** (offline, needs `OTS_DIR`): add the keys to the return dict, reading them from the already-parsed OTS output dicts (`se.get("L10")`/`se.get("L11")`, `f8959.get("L7")`/`f8959.get("L13")`, `final.get("L7")` for Sch-D→L7, `final.get("L12")` for the 1040 deduction, etc.). **For `salt_capped` (Sch A L5e): the driver runs NO separate Schedule-A solver — A5a/5b/8a ride the `US_1040` input (`:261-268`)** — so read L5e from the `US_1040` output if it prints it, else DERIVE `salt_capped = min(state_income_tax + real_estate_tax, 10000)` and mark it driver-derived (r2-N2). Keep `qbi_cap_l12` from the existing `round(net_capital_gain)` (weak, per Interfaces) and add the separate QD-exclusive leaf. Also apply the §1211 8960-L5a fix (Interfaces, r2-I3).
- [ ] **Step 2** run `python3 scripts/oracle/ots_direct.py`-driven `gen_goldens.py` for a single household locally and eyeball the new keys — **include one capped-loss × NIIT-firing scenario to confirm OTS's `f8960` binary accepts a NEGATIVE L5a** (the §1211 fix feeds one; `_parse` reads negatives fine, the solver's acceptance is the open question — r3-M3) (no CI impact — Python is offline).
- [ ] **Step 3: Commit** — `feat(oracle-sweep): ots_direct emits deeper lines + QD-exclusive provenance leaf (T8)`

---

### Task 9: Extend `gen_goldens.py` `taxcalc_run` — deeper lines + provenance leaves + Option keys

**Files:**
- Modify: `scripts/oracle/gen_goldens.py` (`taxcalc_run` :204-260; the JSON assembly :263-327)

**Interfaces:**
- Produces: `expected_taxcalc` gains the **leaf** deeper lines it can express — `deduction_taken` (`c04470`/`standard`), `salt_capped`, `sch_d_to_l7` — plus `total_tax` (taxcalc's L24-equivalent excluding W-2 payroll — reuse the existing exclusion note) and the provenance leaves (`qual_div_l3a = e00650`, `net_ltcg_qd_exclusive = max(0, min(p23250, p23250+p22250))`, `taxable_income` already `c04800` at exact cents). **It leaves `se_l10_oasdi`/`se_l11_medicare`/`f8959_l7`/`f8959_l13`/`qbi_cap_l12` ABSENT (`None`)** — taxcalc exposes no SE/8959 leg split and no 8995-L12-granular variable — so the **paper-level** SE L12 / 8959 L18 / L24 rows are **OTS-single-witness** (§6.4 `Option` rule — taxcalc's printed cross-foot legs don't exist). **This is PAPER-level only (r2-I4):** at the **compute level (T5)** taxcalc's exact `setax` / `ptax_amc` / `niit` totals ARE compared (exact-vs-both at §3.1 whole dollars — bit-equal on setax/ptax_amc/niit, TI/AGI/QBI within ~5¢ so HEAD's `round_dollar`-both-sides passes; r3-M1 — and Add'l Medicare has no other taxcalc witness). 8995 L12 stays OTS-single-witness/WEAK (I1). Names verified against the `taxcalc` variable set at implementation.

- [ ] **Step 1** (offline venv): add the `calc.array(...)` extractions; write the new keys into the `expected_taxcalc` dict.
- [ ] **Step 2** eyeball one household's JSON.
- [ ] **Step 3: Commit** — `feat(oracle-sweep): gen_goldens taxcalc emits deeper lines + provenance leaves (T9)`

---

### Task 10: The covering-array corpus generator + pinned liveness cells + D-2 admission

**Files:**
- Modify: `scripts/oracle/gen_goldens.py` (replace the hand-written `HOUSEHOLDS` :86-201 with a generated corpus; extend the `_provenance` block :291-322)
- Create: `scripts/oracle/corpus.py` (the axis definitions + the variable-strength constrained covering-array builder)

**Interfaces:**
- Produces: `corpus.households()` — the 12 anchors (verbatim, `why` preserved) + a **variable-strength** covering array (t=3 over the named triples {SE, LTCG, qual-div} and {itemized, SALT-over-cap, high-income}; t=2 elsewhere) with the **constraints** (SALT-position ⇒ itemized; itemized ⇒ itemizing-wins; no all-none row; D-1 no dependents), **plus the two bake-time-steered liveness cells** (§5.1, r3-I2b/r4-I1): a bin-edge cell (an L16 operand steered onto a $50 boundary — holds the OTS provenance class live) and a high-TI above-ceiling cents-flip cell (holds the taxcalc provenance class live), each **checked at generation time** to actually produce the intended flip using both engines' offline figures. **D-2 admission:** each candidate is piped through the T7 `oracle_harness` bin; a `refused` candidate is rejected/adjusted (never silently kept); admitted only if both oracles report zero AMT (`c09600`/OTS) and zero L21 credits. The `_provenance` block gains the engine-version-gated regeneration note (§11) and keeps `generated` as the only non-deterministic field (excluded from the §12 determinism claim — r5-M1/M-1).

- [ ] **Step 1** implement `corpus.py` as: the **full cartesian product over each named triple's axes** ({SE, LTCG, qual-div} and {itemized, SALT-over-cap, high-income} — ≈30 + ≈12 rows after the D-2/D-3 constraints) **∪ pairwise-with-constraints over the remaining axes**, deduplicated (M3). This is **t=3-complete on the two named triples by construction** — no variable-strength CA algorithm, and NOT `allpairspy`/PICT (pairwise-only, a mis-fit for t=3, and a global t=3 over all 8 axes blows the ~80-120 budget) — and t=2 elsewhere; a ~10-line coverage assertion verifies every named triple-combination appears. **No new *runtime* dependency** (pure-Python; an offline dev-only helper is acceptable but unnecessary). Include the two pinned liveness cells with `why` strings naming their role.
- [ ] **Step 2** wire `gen_goldens.py` to `corpus.households()`, the harness-binary admission loop, and the extended provenance block.
- [ ] **Step 3** (offline) generate to a scratch file and sanity-check counts (~80–120 + 12 anchors + 2 pinned cells), the two pinned cells' intended flips, and refusal-free admission. **Do not commit the regenerated JSON yet** (that is T11, its own reviewable step).
- [ ] **Step 4: Commit** the generator only — `feat(oracle-sweep): covering-array corpus generator + pinned liveness cells + D-2 admission (T10)`

---

### Task 11: Regenerate the baked corpus; activate deeper-line + provenance-class assertions

**Files:**
- Modify: `crates/btctax-core/tests/goldens/full_return_goldens.json` (regenerated, offline)
- Modify: `crates/btctax-core/tests/golden_returns.rs` + `crates/btctax-forms/tests/golden_packet.rs` (turn on the provenance-class **liveness** assertion; the deeper-line rows now have `Some` data and become live)
- Modify (only if bugs surface): `FOLLOWUPS.md` (one known-defect entry per caught btctax bug — §10, user-mandated) + a `KnownDefect` pin in the test

**Interfaces:**
- Produces: the full baked corpus at the new schema; every deeper-line comparison and both provenance classes now active and green; the class-liveness guard asserts each declared class fired ≥1 or is pinned (both pinned cells now present).

- [ ] **Step 1** (offline): `env OTS_DIR=… .venv/bin/python scripts/oracle/gen_goldens.py > crates/btctax-core/tests/goldens/full_return_goldens.json` (per the file header recipe).
- [ ] **Step 2** enable the `LivenessLedger::dead()` assertion in both tests (it was deferred in T5/T6); the two pinned cells make the provenance classes live.
- [ ] **Step 3** `make check` → green on the FULL corpus. Investigate any red — **four legitimate causes** (a lawful Σround≠roundΣ residual is **not** one — the C1 legs make every cross-foot exact; r2-M3): **(i) corpus/steering error** → fix the generator (T10); **(ii) genuine btctax fill/compute bug** → do **not** fix here, do **not** weaken — **file a `FOLLOWUPS.md` entry** (severity + owning phase, STANDARD_WORKFLOW §4) and pin a **declared known-defect divergence** (`KnownDefect { fu_id, btctax_value }`, `KNOWN DEFECT → <FU-id>`, oracle figures beside it) so `make check` goes green with the bug tracked (§10, user-mandated); **(iii) oracle-driver / extraction error** (a mis-named taxcalc variable, a mis-parsed OTS line, the r2-I3 8960-L5a shape) → fix T8/T9 (§10 "an oracle is wrong → record + cite"), **never** a false btctax KNOWN-DEFECT pin; **(iv) epsilon residual** on a row whose oracle side is `round_leaf` of a *non-leaf* quantity (QBI L15, NIIT L17 — exact-.50 / min-near-tie / cents-MAGI) → §10 triage, never a class-widening. Re-measure the runtime; adjust the T6 shard count if the budget is exceeded (§8 fallback: anchors + sample in `make check`, full corpus in a CI-only test).
- [ ] **Step 4: Commit** — `feat(oracle-sweep): regenerate baked corpus (~NN households); activate deeper lines + provenance classes (T11)`

---

### Task 12: `sweep.py` — the non-CI live sweep

**Files:**
- Create: `scripts/oracle/sweep.py`

**Interfaces:**
- Consumes: `corpus.py` (threshold-biased seeded sampling), `ots_direct.evaluate` + `taxcalc_run` (live oracles), and the T7 `oracle_harness` **`--check` mode** for BOTH the btctax on-paper values AND the reproduction + classification (**I4 — mandatory**: the sweep never re-implements `round_dollar`, the Tax Table, or the QDCGT worksheet in Python — Python's banker's `round()` drifts on `.50` and would either spam false divergences or silently absorb real ones, blinding the discovery mechanism §9 exists to provide).
- Produces: `sweep.py --seed N --count K` → for each seeded threshold-biased scenario (§5.2; honors D-2/D-3), diff the full line set live and emit a **divergence report** (the scenario as a paste-ready household dict, the disagreeing line, `oracle-1 / oracle-2 / btctax-on-paper`, the seed+index). A genuine btctax bug the sweep surfaces is triaged per §10 — **file a `FOLLOWUPS.md` entry** (severity + owning phase, STANDARD_WORKFLOW §4; don't fix here); promoting the scenario into the baked corpus makes it a `KnownDefect` pin there. Never in `make check`.

- [ ] **Step 1** implement the seeded generator (threshold-biased toward the $1,500 Sch B trigger, $10k SALT cap, $200k/$250k thresholds, the wage base, the standard-deduction crossover) and the per-scenario diff+report.
- [ ] **Step 2** (offline) run `sweep.py --seed 1 --count 50`; confirm a clean run prints "0 undeclared divergences" and that an injected wrong figure surfaces a report.
- [ ] **Step 3: Commit** — `feat(oracle-sweep): sweep.py live threshold-biased divergence sweep (T12)`

---

### Task 13: Validation KATs (§12) and the regime-crossing note

**Files:**
- Modify: `crates/btctax-forms/tests/golden_packet.rs` / `crates/btctax-core/tests/golden_returns.rs` (add the §12 KATs)

*(Note: the r5-M1 regime-crossing-straddle disposition is ALREADY folded into spec §6.4 at r5 — "the one out-of-class residual … falls to §10 triage if the sweep surfaces one … not fixed by widening the methodology condition." No spec edit is needed in this task.)*

**Interfaces:**
- Produces the §12 obligations as tests: **deeper-line teeth** (for each deeper line, a corpus scenario where dropping that line's logic changes the number — the t=3 triples guarantee the 8995-L12 case); **read-back fault-injection** (a `#[should_panic]` fixture that perturbs an on-paper value or swaps a map, proving the test reads the PDF not the struct); **anchor-derivation KAT** (already in T6 — reference it); **class-liveness** (already in T11 — reference it); **determinism** (regenerating yields identical `households` payload excluding `_provenance.generated` — a Python-side check, run offline); **runtime budget** (a note asserting the differential wall-clock stays within the §8 budget, measured).

- [ ] **Step 1** write the fault-injection `#[should_panic]` test and the deeper-line teeth tests; verify they fail-for-the-right-reason then pass.
- [ ] **Step 2** add the determinism KAT (Python-side, offline): regenerating the corpus yields an identical `households` payload, excluding the `_provenance.generated` timestamp (§12, r5-M1/M-1). (The r5-M1 regime-crossing note is already in spec §6.4 — no spec edit needed.)
- [ ] **Step 3** `make check` → green.
- [ ] **Step 4: Commit** — `test(oracle-sweep): §12 validation KATs — deeper-line teeth, read-back fault-injection, determinism (T13)`

---

## Self-review (done by the plan author)

- **Spec coverage:** §1–§3 (extend/unify/scale, read-off-PDF, two read-back layers, evolve `golden_packet.rs`) → T4/T6/T7; §4 domain D-1/D-2/D-3 → T10; §5.1 covering array + pinned cells → T10, §5.2 sweep → T12; §6.1 line set → T1/T5/T6/T8/T9; §6.2 comparison rule + two-part L16 → T2/T3/T5/T6; §6.3 sign/blank → T4; §6.4 classes/stacking/liveness → T3/T5/T6/T11; §6.5 three-way localization → T6; §7 corpus + `golden_returns` disposition + anchor pinning → T5/T6/T10/T11; §8 runtime budget/sharding → T6/T11; §9 harness binary → T7; §10 lifecycle + r5-M1 note → T12/T13; §11 version-drift → T10; §12 validation → T13. No section unmapped.
- **Placeholder scan:** the two former "decide at implementation" points are now **resolved** — T7 is a separate unpublished bin-crate (M2), T12 mandates the harness `--check` mode (I4); neither is a deferred choice. All oracle figure literals in test code are flagged "read the exact baked value, do not guess."
- **Type consistency:** `L16Operands`/`OracleId`/`LivenessLedger`/`KnownDefect`/`taxcalc_methodology_class`/`provenance_class_fires`/`stacking_ok` are named identically in T3 and consumed in T5/T6/T11; `table_l16`/`consulted_table`/`sum_round`/`round_leaf`/`rate_on_printed` from T2 are used unchanged downstream; the C1 component-leg field names (`se_l10_oasdi`/`se_l11_medicare`/`f8959_l7`/`f8959_l13`) match across T1 (schema), T8 (OTS emit), the per-line table, and the T2/T5/T6 reads; taxcalc leaves them `None` (T9). `oracle_ops: Option<&L16Operands>` (M4) and `dead(&self, &[…])` (N4) reconciled between T3's Interfaces and its tests.
- **r1 review (1C/4I/4M/4Nit) folded → r2:** C1 (per-line reproduction table + OTS component legs); I1 (L12 single-witness/weak + §14.2 follow-up); I2 (the four `checked==3` tests migrated); I3 (`tests/common/` share); I4 (`--check`); M1 (6+1 divergence disposition); M2 (bin-crate); M3 (cartesian-triples ∪ pairwise); M4 (`Option` fail-closed); N1–N4 (create-not-append, import path, comment, `dead` sig).
- **r2 review (0C/4I/3M/5Nit) folded → r3:** I1 (synthetic-literal `sum_round` flip test — the leg-flip is unbakeable pre-T11 and no anchor flips); I2 (QBI-deduction row added — the dropped §6.1 headline line, reuses the existing `qbi_deduction` field, epsilon→§10 triage); I3 (NIIT witness rebuilt — the T8 OTS 8960-L5a §1211 fix + paper=OTS-epsilon/compute=taxcalc disposition); I4 (the **two-level rule** — compute-level keeps exact-vs-both for SE/AddlMed/NIIT/TI/AGI/QBI, paper stays OTS-single-witness for the cross-foots); M1 (AGI capital leg = §1211-netted Sch-D→L7); M2 (L24 reads the printed chain, not the exact `ar`); M3 (T11 step 3 = four causes, incl. oracle-driver error ≠ false btctax pin); N1–N5 (NIIT cite, salt driver-derived, owning-phase at every FOLLOWUPS filing, AGI L9 cite, `Some(&ops)`).
