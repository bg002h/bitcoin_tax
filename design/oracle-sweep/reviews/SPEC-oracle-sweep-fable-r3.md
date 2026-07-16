# SPEC-oracle-sweep — independent Fable architect re-review, r3

*Persisted VERBATIM (STANDARD_WORKFLOW §2) before folding. Reviewer: Fable (independent architect pass, r3
— re-review after the r2 fold). Reviewed against `main`, clean tree. Persisted 2026-07-15. NOT green
(0C/2I) — fold to r4 follows.*

---

VERDICT: 0 Critical / 2 Important / 1 Minor / 2 Nit

*Reviewer: Fable (independent architect pass, r3 — re-review after the r2 fold). Reviewed against `main`, clean tree, 2026-07-15. Every load-bearing claim was re-verified against current source — `printed.rs`, `other_taxes.rs`, `method.rs`, `golden_packet.rs`, `golden_returns.rs`, `testonly.rs`, `ots_direct.py`, `gen_goldens.py`, and the baked `full_return_goldens.json` figures — not against the spec's own account of it.*

**Disposition of r2-I1:** resolved as framed (the §6.2 contradiction is gone; the two-part rule restores the oracle's opinion of the tax and closes both horns on L24) — **but the fold's divergence-class predicates are mis-scoped in a way the repo's own anchor data refutes → new-problem (r3-I1, r3-I2).**
**Disposition of r2-I2:** **resolved** — `golden_returns.rs` is disposed coherently (stays, full corpus at compute level, no PDF fills, adopts the class mechanism, serves as the §6.2(b) witness), the witness is genuinely computable there, and a whole-tree grep confirms there is no third consumer. It inherits r3-I1's predicate defect on day one, but that defect is filed once, under r3-I1.

---

## Verification of the r2 fold

| r2 | Disposition | Evidence |
|---|---|---|
| I1 (Table row ambiguous: dead check vs undeclared reds) | **Resolved as framed; new seam in the class predicates** | §6.2 two-part rule verified against `printed.rs:607-618` (paper L16 *is* `qdcgt_line16` on the printed L15 + printed Sch-D figure) and `printed.rs:627` (L24 = printed L22 + L23). Part (a) is internally consistent (both sides btctax's own lookup, QDCGT on reproduced printed operands per r2-N1 — folded). Part (b) restores the oracle's L16 opinion that part (a) alone drops, so the "check that cannot fail" horn is closed. Pinning L24's tax component to the part-(a) figure closes the bin-straddle horn **on L24** — verified consistent with the `golden_packet.rs:81-131` cross-foot pattern. `Table_btctax` is reachable cross-crate as claimed (`method.rs:74` `pub fn qdcgt_line16`; `ty2024_table` via `testonly`, already imported by `golden_packet.rs:33`). The straddle predicate **is** computable from what the extraction carries: the baked OTS figures carry exact cents (`single_crypto_business_se` OTS TI = 70,008.94; `mfj_se_over…` OTS TI = 253,942.94 — `full_return_goldens.json`), and the reproduced printed TI is computed in-test. But the predicate is conditioned on the **wrong quantities** → r3-I1. |
| I2 (`golden_returns.rs` undisposed; bake-day red) | **Resolved** | §7's last bullet chooses option (i) from r2's fix direction and says why (it is the compute-side Table-semantics witness §6.2(b) needs). Coherent: compute level has `expected_ots.income_tax_before_credits` (`testonly.rs:397-410`, all-required `f64` today; the §6.4 `Option` change covers taxcalc's missing `total_tax` — confirmed absent from the baked JSON) and can call `ty2024_table` + `method` fns (it is a `btctax-core` integration test; it already imports both — `golden_returns.rs:29-31`). No PDF fills ⇒ the full corpus at compute level is microseconds-per-household, no §8 budget pressure. Whole-tree grep for `golden_households|GOLDEN_RETURNS_JSON|full_return_goldens` (excluding `target/`): definition `testonly.rs`, consumers `golden_returns.rs` + `golden_packet.rs`, writer `gen_goldens.py`, plus design docs. **No third consumer left to break.** |
| M1 (nextest model) | Resolved — §8 now states it correctly (per-test processes, parallel across the run, serial within one `#[test]`). |
| M2 (4th pattern; misfiled example) | Resolved — the rate-on-printed-operand row is accurate (`other_taxes.rs:167` L7 = round(0.9% × printed L6), `:173` L13, `:320` L17 = round(3.8% × line16)); the Leaf example is now 8960 L13 = round(exact AGI), matching `:316`; the taxonomy is declared illustrative with line-by-line derivation pushed to the plan. The reproduced-operand chains bottom out in oracle-exposed components or scenario inputs — coherent. |
| M3 (anchor form sets) | Resolved — §7 keeps the 12 hand-written sets (`golden_packet.rs:300-350`) as pinned data and obligates the derivation to reproduce them (but see r3-M1: §12 never received the obligation §7 attributes to it). |
| M4 (D-2 enforcement vehicle) | Resolved — §9 harness binary, invoked per candidate; drift-prone Python re-implementation explicitly ruled out. |
| M5 (credits scope) | Resolved — scoped to the L17/L21 band the `golden_packet.rs:104-119` precondition actually requires; childless EIC (L27, payments-side, touches no compared line) admitted; low-W-2 band floored above the EIC domain. Sound, if belt-and-suspenders. |
| M6 (class liveness) | Folded — but the rule as written is jointly unsatisfiable with the guard and the straddle class it polices → r3-I2. |
| N1 (QDCGT both sides) | Resolved — §6.2(a) states it. |

---

## Important

### r3-I1 — the L16-family divergence-class predicates are conditioned on the headline TI, but the lookup operates on **worksheet operands**; the repo's own anchor refutes the taxcalc predicate today, and the OTS witness class misses whole families of lawful $1 residuals

**Anchors:** spec §6.2(b) ("reproduced-TI and oracle-TI straddle a $50 Tax-Table bin boundary … computable from the two TIs"), §6.4 (taxcalc class: "Tax Table mandatory **i.e. TI < $100,000**", citing `golden_returns.rs:16-22, 102-104`); `crates/btctax-core/src/tax/method.rs:21` (`TAX_TABLE_CEILING`), `:47-56` (`worksheet_tax` — **each QDCGT operand chooses Table-vs-TCW independently**), `:84-90` (the ordinary remainder L5 and the 15/20% slices); `crates/btctax-core/tests/golden_returns.rs:116-126`; `crates/btctax-core/tests/goldens/full_return_goldens.json`.

Three concrete failures, in increasing subtlety:

1. **Certain, day one, no corpus growth needed.** `single_qdcgt_both_slices` has TI = **112,400** (≥ $100k) yet taxcalc diverges on L16 (17,471 vs btctax/OTS 17,477) — because the QDCGT worksheet looks its **ordinary remainder** up in the Table, and the remainder is below $100k. `golden_returns.rs:122-125` documents this exact mechanism twenty lines below the passage the spec cites. The moment §7's disposition converts the per-household entries to the §6.4 class, this **anchor household's** divergence matches no predicate (`TI < $100,000` is false) → undeclared red in `btctax-core` with the current 12 households. The correct behavior requires the implementer to contradict the spec's own "i.e." gloss.
2. The **OTS witness class** has the mirrored defect: for a QDCGT household the straddle that matters is on the reproduced-vs-oracle **remainder** (and pref-slice operands), not on "the two TIs" — a remainder straddle with same-bin TIs is an undeclared red; a TI-straddle test above $100k is meaningless (no bins).
3. **No class covers the non-bin rounding-provenance residuals**, which the corpus makes an expected event: (i) the QDCGT 15/20% slices are computed on printed operands by btctax (`method.rs:87-90`) and on exact cents by OTS — the baked figures prove OTS carries cents into L16 itself (`single_miner_qbi…` OTS L16 = **8,354.59**) — so `round_dollar` can flip $1 with no bin involved; (ii) above $100k, the TCW on the whole-dollar reproduced TI vs the oracle's tax on its exact-cents TI (`mfj_se_over…`: OTS TI 253,942.94, L16 47,031.31) differ by rate × δ where δ ≠ 0 for **every** cents-carrying household, flipping the rounded dollar with probability ≈ |rate × δ| per household per oracle. The §5.1 axes *mandate* high-income SE cells (SE over $250k; high W-2 band; the t=3 triple), so the baked corpus contains dozens of cents-carrying households above and below $100k. Expected undeclared reds ≥ 1 — the same magnitude of failure that made r2-I1 horn 2 gating; today's 3 comparable anchors pass by residual-position luck.

**Fix direction (one paragraph each in §6.2(b)/§6.4):** scope the predicates to what the lookup actually consumes, and make the witness class a **provenance** predicate rather than a geometric one. (a) taxcalc class: fires when btctax's L16 lookup consulted the Table for **any worksheet operand** (computable from the reproduced operands via the `worksheet_tax` branch, `method.rs:47-56`) — this admits `single_qdcgt_both_slices` and stays refutable above the ceiling. (b) OTS witness class: fires iff `Table_btctax(oracle's own exact operands) == round_dollar(oracle L16)` **and** `Table_btctax(reproduced printed operands) ≠ round_dollar(oracle L16)` — i.e. the disagreement is *fully explained* by operand provenance (printed-chain rounding), not by lookup semantics. This subsumes bin-straddle, remainder-straddle, slice-rounding, and TCW-cents in one computable, falsifiable predicate: a genuine Table-semantics bug in btctax fails the first equality, so the class cannot absorb it, and part (b) keeps its teeth.

### r3-I2 — the straddle class is jammed between two other r3 requirements: the retained anti-"btctax against the world" guard makes it unfireable, and mandatory class-liveness makes it unfulfillable in a deterministic corpus

**Anchors:** spec §6.4 ("The anti-'btctax against the world' guard **stays**: a line where btctax disagrees with **both** oracles is never silently classed" … "**Two L16 classes coexist** … (occasional)" … "**Class liveness (r2-M6):** every declared class must fire for ≥1 corpus household"); `crates/btctax-core/tests/golden_returns.rs:358-372` (the guard's current form: `matches_1 || matches_2 || agrees_with.starts_with("neither")`), `:41-53` (`outlier_alt` — the per-household model's explicit **stack** machinery); spec §5.1/§12 (no pinned-straddle obligation).

(a) **Every OTS-straddle household is necessarily a both-oracle disagreement.** A straddle puts the TIs (or operands) near a $50 bin **edge**; btctax then taxes the bin **midpoint** while taxcalc computes the exact schedule at the edge — a ≈ rate × $25 ≈ $3–6 difference that always survives rounding. So the straddle class can never fire on a household where btctax agrees with taxcalc on L16. The spec keeps the guard ("never silently classed") without defining class-**stacking**, and the per-household model it replaces handled this exact shape only via explicit `agrees_with: "neither"` + `outlier_alt`. Read the guard the way `golden_returns.rs:358-372` implements it today (agree-with-≥1-or-red), and {guard, a straddle household in the corpus, straddle-class liveness} are **jointly unsatisfiable**: the class §6.2(b) builds its witness on can never legally fire. (b) Even with stacking defined, a straddle is a low-single-digit-percent event per cents-carrying household, and the baked corpus is **deterministic** — plausibly zero straddles at bake time — while §6.4 makes liveness unconditional and §5.1/§12 obligate no pinned straddle cell. Either way the implementer meets a red guard with no specced resolution — the situation the gate exists to prevent.

**Fix direction:** (a) define the guard's class-form explicitly: a line where btctax disagrees with both oracles passes only when **each** oracle's diff matches its own declared, condition-bearing predicate (with r3-I1's falsifiable provenance predicate, this is *stronger* than the old `"neither"` prose — a btctax Table bug matches no predicate and stays red). (b) Make liveness satisfiable by construction: add a §5.1 pinned cell tuned at bake time to fire the witness class (the generator sees both engines' exact figures offline, so steering a household's reproduced/oracle operands onto a bin edge is a deterministic, checkable act), with the obligation recorded in §12 — or scope §6.4's liveness rule to "fires ≥1 household **or** carries a §5.1 pinned-cell obligation."

---

## Minor

- **r3-M1 — §7 attributes to §12 an obligation §12 does not contain.** §7: "**§12 obligates** the derivation to reproduce all twelve [hand-written form sets]" — §12's checklist (deeper-line teeth, fault injection, hermeticity, determinism, runtime, green) has no such item. The requirement exists (stated in §7), but the validation checklist is the artifact the implementation phase is checked against; a missing item there is how a stated requirement goes unverified. Add the derivation-reproduces-the-12-anchors KAT (and, per r3-I2(b), the class-liveness/pinned-cell obligation) to §12.

## Nit

- **r3-N1 —** §7's anchors-only carve-out names byte-reproducibility and the identity sweep, but `the_packet_is_stapled_in_irs_attachment_sequence_order` (`golden_packet.rs:383-414`) also loops the whole corpus with full packet fills. Its property is genuinely valuable on *generated* households (new form combinations ⇒ new orderings), so the right disposition is to ride the sharded differential loop's existing fills, not anchors-only — say which.
- **r3-N2 —** D-2's "zero **L18–L21-band** credits" mislabels the band (L18 is the L16+L17 sum, not a credit; the precondition lines are L17 and L19–L21 → L21). The `golden_packet.rs:104-119` citation carries the exact truth; tighten the prose to match it.

---

## Strengths (brief)

The fold is again real work: both r2 Importants were engaged on the merits with the right in-repo machinery (the two-part rule is verified accurate against `printed.rs`/`method.rs`, the `golden_returns.rs` disposition is the strongest of the three options r2 offered, and all six Minors + the Nit landed cleanly, including the corrected nextest model and the accurate fourth pattern row). The surviving defects are confined to the divergence-class *predicates* — the one part of the r2 fix direction that was under-specified, including by the r2 reviewer.

**NOT GREEN: 0 Critical / 2 Important blocks the gate.** Both findings are paragraph-scale predicate rewrites in §6.2(b)/§6.4 (plus two §12 checklist lines), not design changes — fold and re-review.
