# PLAN-oracle-sweep — independent Fable plan re-review, r2

*Persisted VERBATIM (STANDARD_WORKFLOW §2) before folding. Reviewer: Fable (independent plan review).
Reviewed `design/IMPLEMENTATION_PLAN_oracle_sweep.md` @ HEAD `d7c9bb9` (r2 fold of my r1 1C/4I/4M/4Nit,
including the `9fdb9cc` caught-bug policy) against GREEN spec r5. All citations verified against current
source, and the key r2-fold claims verified **numerically** against the baked
`full_return_goldens.json`. Persisted 2026-07-16. NOT green (0C/4I) — fold to r3 follows.*

---

VERDICT: 0 Critical / 4 Important / 3 Minor / 5 Nit

**One-line dispositions of the r1 findings:**

- **C1 — FIXED.** The per-line reproduction table is present, correct where I could check it exactly, and the four OTS component legs are real: every "btctax prints" cell reproduces from source (SE L12 = `line10 + line11` over printed lines, `printed.rs:231-233`; 8959 L18 = `line7 + line13`, `other_taxes.rs:167,173,178`; L24 = `line22 + line23` with L23 = Sch2 L21 = SE L12 + 8959 L18 + NIIT L17, `printed.rs:620-627` + `:289-292` — the reproduction's dropped L17/L21 terms are exactly the D‑2 admission predicates and the present-and-zero guard T4 preserves from `golden_packet.rs:104-119`). The legs are in `_parse`'s reach (`ots_direct.py:164-171` captures every `Lxx`; the driver already reads `se.get("L12")/("L13")` and `f8959.get("L18")` from the same solver outputs, `:231,:240`). **Numerically verified from the baked JSON:** mfj_se legs must be (0.00, 2,142.52) (wage base consumed; 2.9% × 73,880 = 2,142.52 = baked `se_tax` exactly), miner legs (6,870.84, 1,606.89) (= `return_1040.rs:3478`'s 1,606.89), and the M1 "dissolving 7th divergence" reproduces: sum_round → 8,355 + 8,478 = **16,833** = btctax's printed L24, OTS's exact 16,832.32 never consulted. taxcalc genuinely has no leg split (`taxcalc_run` exposes only `setax`/`ptax_amc` totals, `gen_goldens.py:255-257`), so `None` legs + OTS-single-witness at the **paper** level is sound and spec-blessed (§6.2 "Where an oracle exposes only the total…"). The compute-level half of the witness question survives as r2-I4 below.
- **I1 — FIXED.** L12 marked single-witness/WEAK in T1's schema comment, the table row, and T8; §14.2 closure filed as a follow-up (spec §6.4 explicitly allows "ship it single-witness/weak until closed").
- **I2 — FIXED.** All four corpus-coupled tests named with exact citations (verified: `:161-185` `checked==3` vs `round(exact se_tax)`; `:519-568` `checked==3`; `:596-620` `checked==3`; `:574-586` fills every non-SE household); the migration mechanics work — every folded assertion rides a fill the differential loop already produces (schedule_se and f8995 are extracted for compared lines; Sch C line A costs one extra `extract_lines` on an existing fill; the no-8995-row check reads the derived form set), predicates derive from `inputs.self_employment_income > 0`, and byte-repro/identity/SALT/no-SE tests are all accounted for.
- **I3 — FIXED.** `tests/common/mod.rs` exists and is consumed via `mod common;` by kats/overflow/sp3/sp3b (verified); the plan moves `packet`/`form` + the sign/blank helpers there and bans `include!`.
- **I4 — FIXED.** T12 mandates the T7 `--check` mode; the Python re-implementation option is deleted; the banker's-rounding rationale is stated.
- **M1–M4, N1–N4 — all landed.** M1's 6+1 disposition verified numerically (above); M2's bin-crate reasoning restated; M3's cartesian-triples ∪ pairwise construction in T10; M4's fail-closed `Option` is a named mutation target; N1 verified (`testonly.rs` has no `#[cfg(test)]` module); N2 (`crate::tax::FilingStatus` re-export exists, `tax/mod.rs:47`); N4 (`dead(&self, &[...])` consistent between Interfaces and test).

**The two flagged residuals:**

- **(a) AGI/L15 reproduction from `GoldenInputs` — SOUND, with one operand imprecision (r2-M1).** It is not self-referential: the reproduction never consults btctax's output, so a btctax income-aggregation or half-SE bug diverges paper-vs-reproduction and stays red; the income terms are the shared ground truth all three engines were fed (integral, so `Σround = Σ`), and the only computed components are per-oracle (`se_tax/2`) or independently double-oracle-checked (`sch_d_to_l7`). The design is not merely acceptable but **necessary**: `round_leaf(oracle_agi)` would differ from btctax's printed L11 by $1 exactly when half-SE lands on .50 (round(N−h) ≠ N−round(h) at h≡.50 for integral N) — a lawful residual with no class. Conceptual common-mode blindness is backstopped by the oracle-TI leaf feeding L16 part (b). The imprecision: the capital term of "Σ whole-dollar income inputs" is NOT a raw input (see r2-M1).
- **(b) NIIT "OTS primary, taxcalc weak/optional" — NOT crisp, and the OTS-primary claim has a verified hole (r2-I3).**

---

## Important

### r2-I1 — T2 step 1's flip-pair instruction is unexecutable: the legs are not in the JSON before T11, and no flip pair exists among the anchors even after it

**Touches:** T2 step 1 (`se_l12_cross_foots_from_legs_not_the_exact_total` + its comment and footnote).

The test comment orders: *"Read the exact se_l10_oasdi / se_l11_medicare for a cents-carrying SE anchor from full_return_goldens.json (do NOT guess); pick a household where the legs' cents make round(L10)+round(L11) ≠ round(L10+L11)."* Both halves are impossible at T2: (i) the baked JSON gains those fields only at **T11** — at T2 they exist solely as `None`-defaulting schema (the plan's own global constraint says so), so there is nothing to read; (ii) even post-bake, **no anchor flips** — the three SE anchors' legs are derivable from the baked totals and provably non-flipping (mfj_se: 0.00/2,142.52 → 0+2,143 = 2,143 = round(2,142.52); both miners: 6,870.84/1,606.89 → 6,871+1,607 = 8,478 = round(8,477.73)), which is *necessarily* so, because the current paper test (`golden_packet.rs:172-177`) passes today comparing btctax's cross-footed L12 to `round(exact se_tax)`. Worse, the shown assertion `sum_round(&[l10,l11]) == round_leaf(l10)+round_leaf(l11)` is a tautology of `sum_round`'s definition unless the pair flips — with anchor values it kills no mutation, the exact untested-guard shape the plan's own discipline forbids. **Executor failure:** stalls at T2 step 1 on a contradictory instruction, then either violates task order (chasing T8/T11 data) or ships a toothless test. **Fix:** make it a synthetic-literal unit test with a flipping pair, mirroring the repo's own KAT (`other_taxes.rs:80-85`: 274.50 + 499.50 ⇒ `sum_round` = 775 ≠ `round(774.00)` = 774), assert both the equality and the `≠ round_leaf(l10+l11)` inequality, and drop the read-the-JSON instruction (keep it only for the T2 `table_l16`/`consulted_table` tests, whose figures ARE baked — I verified 253,942.94/47,031.31, 112,400/8,000/25,000, and 70,008.908/10,454.96 against the JSON).

### r2-I2 — The reproduction table omits a spec §6.1 **headline** line: QBI deduction (8995 L15 → 1040 L13)

**Touches:** the C1 table (§ "The per-line reproduction table"), T5 step 2, T6 step 2, self-review coverage map. Spec §6.1.

Spec §6.1's headline set is: AGI, TI, L16, SE tax, NIIT, Add'l Medicare, **"QBI deduction (8995 L15 → 1040 L13)"**, L24. The table has rows for the other seven plus the four deeper lines — QBI deduction has none, and it is compared **today** at compute level against both oracles (`golden_returns.rs:250-255`). The table is self-declared load-bearing ("fixes the pattern, the operands, and which oracle witnesses each line"), so an executor implementing "the full line set per the C1 table" silently drops an existing, spec-required comparison. The row also needs a stated disposition, because btctax's printed L13 is **not** a leaf: `qbi.rs:195-211` derives 8995 L15 as `min(round(20% × round(QBI)), round(20% × (round(TI_bq) − round(ncg))))` — a rounds-at-lines chain whose value can lawfully differ from `round_leaf(oracle exact qbi)` on exact-.50 landings and min-near-ties (measure-epsilon, the same shape as the spec's r5-M1 regime-crossing residual). **Fix:** add the row — `1040 L13 / 8995 L15 | printed 8995 chain (qbi.rs:195-211) | round_leaf(oracle_qbi_deduction) | OTS + taxcalc (both bake it today); epsilon residual (exact-.50 / min-near-tie) → §10 triage, not a class` — and keep the compute-level exact-vs-exact row (see r2-I4).

### r2-I3 — The NIIT row's witness is unsound as stated: the OTS driver feeds Form 8960 an un-§1211'd L5a, the "(OTS `niit` is the printed L17)" gloss is contradicted by the driver, and "taxcalc weak/optional" leaves the executor guessing

**Touches:** the table's NIIT row; T8; T9; T11 step 3. Spec §6.2 (rate-on-printed pattern), §10.

Verified against source: `ots_direct.py:341` feeds OTS's 8960 `L5a = max(ltcg,0) + max(stcg,0)` — it never applies the §1211 limitation and ignores the loss-year NII **reduction** btctax correctly applies (`other_taxes.rs:219-222,308`: L5a "may be NEGATIVE (a §1211-limited loss)… REDUCES NII", §1.1411-4(d)). On a corpus cell combining a capped loss with NIIT-firing income (capped-loss × high-W-2 is a **pairwise-guaranteed** combination; investment income co-occurs in most such rows), OTS's NII is $3,000 high, so when the NII arm binds the **primary — and under this row, only —** NIIT witness is wrong by $114 by construction. T11 then goes red, and T11 step 3's triage ("exactly two legitimate causes: a corpus/steering error… or a genuine btctax bug") names neither actual cause — the executor is steered toward filing a **false** btctax FOLLOWUP and pinning btctax's *correct* value as a `KNOWN DEFECT`, corrupting the artifact the §10 lifecycle exists to keep honest. Separately, `:342` feeds 8960 `L13 = p1.get("L11")` — the **cents-carrying** pass-1 AGI — so OTS's L17 is computed on exact-cents MAGI, not btctax's printed-chain operands: the table's "(OTS `niit` is the printed L17)" is unsupported, and `round_leaf(ots_niit)` carries a ~±2¢ rounding window vs btctax's `round(3.8% × integral L16)` when the MAGI arm binds (epsilon; anchors are integral — 988.0/2,926.0 — because no anchor mixes SE cents with NIIT). And "taxcalc `niit` is exact ⇒ weak/optional" is not an executable disposition: compare-and-hope (latent unclassed $1 reds) and skip (single-witness) are different tests. **Fix:** (i) T8 sets 8960 L5a to the §1211-limited net figure it already computes for `sch_d_to_l7` (and L13 to pass-1 L11 as today — note the gloss, don't claim "printed L17"); (ii) state the disposition: paper-level NIIT = OTS via `round_leaf`, epsilon window explicitly assigned to §10 triage; taxcalc's exact `niit` compared at the **compute level** (exact-vs-exact, passes today — see r2-I4), not on paper; (iii) T11 step 3 adds the third legitimate cause (r2-M3).

### r2-I4 — T5/T9 delete the compute-level exact-total, both-oracle comparisons that exist and pass today (SE tax, Additional Medicare; NIIT per r2-I3) — a witness regression the paper-level single-witness rule does not license

**Touches:** T5 step 2 ("taxcalc's totals are NOT compared there"), T9 ("do NOT compare taxcalc's `setax`/`ptax_amc` totals there"), the table's Witness column. Spec §7.

At the **paper** level the plan is right: printed L12/L18 are cross-foots, `round(exact_total)` is a different quantity, and §6.2 makes those lines OTS-single-witness. But `golden_returns.rs` compares **compute structs** — `ar.se_tax_sch2_l4` is the *exact* figure (`return_1040.rs:3151,3478`: `se.ss + se.medicare`, e.g. 1,606.89), and today it is held against `e.se_tax` **and** `t.se_tax` (`golden_returns.rs:274-285`), agreeing to the cent (verified: 8,477.73 = 8,477.73 on both engines; `additional_medicare_tax` 394.92 both; `niit` 988/2,926 both). Exact-vs-exact has **no** Σround residual, is valid corpus-wide, and is precisely what spec §7 describes ("btctax's compute structs **vs both oracles**"). The fold's blanket "taxcalc's totals are NOT compared" universalizes a paper-level truth into deleting a live independent-lineage witness: **Additional Medicare loses its taxcalc witness entirely** (nothing else consumes `ptax_amc`), SE keeps only an indirect one (via the AGI reproduction's `se_tax/2`), and NIIT's is left to the r2-I3 ambiguity — a strictly weaker harness than HEAD on lines where today's guarantee is cent-exact three-engine agreement. It also creates a needless **interim coverage dip**: from T5 until T11 the leg-gated SE/8959/L24 rows and the `deduction_taken`-gated TI row are inert, so existing passing comparisons vanish for six task boundaries. **Fix:** state the two-level rule in T5 — cross-footed lines get a row *pair*: compute-level **exact totals vs both oracles** (keep today's SE/AddlMed/NIIT/TI/AGI/QBI exact rows — they need no new fields and cure the interim dip) and paper-level printed reproductions per the C1 table (OTS-single-witness where legs are required). Scope: do **not** extend exact-vs-exact to L16/L24 (their exact chains lawfully differ in cents across engines — OTS total 49,568.75 vs btctax exact 49,568.43 on mfj_se; the two-part/cross-foot design is correct there).

---

## Minor

### r2-M1 — The AGI row's income-side capital operand is not a "whole-dollar income input"
The reproduction's income side must carry the **§1211-netted** Schedule-D figure (`printed.rs:533-538`: L7 = Sch D L16 or −L21, the capped amount), not raw `short/long_term_capital_gains`. A literal raw-input sum is wrong by $15,000 on `single_capital_loss_capped` (−18,000 vs −3,000) — caught at T5's own boundary, but the table claims to fix the operands. Say: capital term = `net = stcg+ltcg`, floored at −3,000 when negative (domain-exact), or `round_leaf(oracle_sch_d_to_l7)` once baked (post-T11).

### r2-M2 — T5's "Produces" names the wrong btctax-side figures for the cross-footed rows
"btctax's `assemble_absolute`/method figures" — but the SE L12 / 8959 L18 / L24 comparisons must take the **printed-chain** values (the exact `ar.se_tax_sch2_l4` vs `sum_round(legs)` re-opens the ±$1 flip at T11). The table's "btctax prints" column governs; make T5 say the cross-foot rows read the printed chain (`golden_returns.rs` already builds it for L24, `:243,297`).

### r2-M3 — T11 step 3's "exactly two legitimate causes" is still an under-count
Beyond generator error and genuine btctax bug there is (iii) an **oracle-driver/extraction error** (the r2-I3 L5a defect; a mis-named taxcalc variable — T9's names are only "verified at implementation"; a mis-parsed OTS line), which triages per §10 "an oracle wrong → record + exclude with a cite" / fix T8-T9, and (iv) an **epsilon residual** on rows whose oracle side is `round_leaf` of a *non-leaf* quantity (QBI L15, NIIT L17) → §10 triage, never a class widening. Name both so the executor is not misdirected at the moment of failure.

---

## Nit

- **r2-N1** (table, NIIT row): cite `other_taxes.rs:320` (the actual `line17 = round_dollar(NIIT_RATE * line16…)` derivation); `:236,288` are the closure comment and struct doc.
- **r2-N2** (T8 step 1): "the 1040sa parse for L5e" — the driver runs **no** separate Schedule A solver; A5a/A5b/A8a ride the `US_1040` input (`ots_direct.py:261-268`). Say: read A/L5e from the US_1040 solver's output if it prints it, else derive `salt_capped = min(5a+5b, 10000)` and mark it driver-derived.
- **r2-N3** (T8/T11/T12): every FOLLOWUPS entry the plan files (the §14.2 L12 closure; caught bugs) must carry an **owning phase** at filing time (STANDARD_WORKFLOW / CLAUDE.md burndown rule) — say so where the filings are mandated.
- **r2-N4** (table, AGI row): the L9/L11 derivation is `printed.rs:518-542` (`form_1040_income_lines`); `:341-357` is `schedule_1_lines` (fine for the L26 half).
- **r2-N5** (T3 step 1): the two provenance tests pass `&ops` where the signature takes `oracle_ops: Option<&L16Operands>` — write `Some(&ops)` (seconds-level compile fix, but the snippet should be right).

---

## Verified (r2 spot-checks that hold)

Green-preserving order re-confirmed end-to-end: T1's `#[serde(default)]` fields parse the current JSON; T2/T3's baked literals are exact (JSON dump); T5 stays green on the 12 anchors (the 6 Table-L16 divergences all satisfy `consulted_table` on btctax's operands — including single_miner's coincidental 8,355 = round(8,355.055) agreement; the L24 entry dissolves numerically; NIIT row live-and-green via the existing `niit` field; M4's fail-closed `Option` keeps un-baked provenance classes from firing); T6's folded assertions run on fills the loop already makes; T7's bin-crate reasoning holds (`testonly` is `pub mod`, `GoldenHousehold` fields pub, `serde_json` genuinely absent from `btctax-forms`); T8's legs/T9's `None`s/T10's construction/T12's `--check`/T13's KATs are internally consistent, and the §10 KnownDefect machinery matches the `9fdb9cc` policy (separate category, stale pin fails).

## Strengths

The C1 fold is genuinely load-bearing — the reproduction table is derived from source, not asserted, and the dissolving-divergence claim survives independent numeric re-derivation to the dollar. The fold also resolved all four r1 Importants exactly as specified, with the type/name consistency across T1/T2/T3/T5/T6/T8/T9/T11 holding under a full cross-check.

**NOT green — 0 Critical / 4 Important.** r2-I1/I2 are wording/table amendments; r2-I3 is a small T8 driver fix plus a stated disposition; r2-I4 is a scoping sentence in T5/T9 restoring rows that already exist. Fold and re-review.
