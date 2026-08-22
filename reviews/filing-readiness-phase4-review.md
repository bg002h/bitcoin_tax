# Phase 4 review — attestation KATs + the LIMITATIONS rewrite, scoped `6cd9e073..HEAD`

_Opus, read-only (minor phase per the owner's model tiering). **Persisted VERBATIM before any fold.**
The agent could not write this file itself — the harness blocks subagents from writing report files —
so the controller is the scribe; text copied verbatim, in its own commit, before any fold._

★★★ **This reviewer RE-RAN the claimed mutation in an isolated worktree rather than trusting the
commit message, and the claimed kill did not reproduce.** That is the single most valuable thing any
reviewer did on this branch.

---

VERDICT: needs-changes

**One Important finding. Everything else is Minor/Nit, and the tax substance of all nine KATs checks out against the form text.**

## FINDINGS

### I-1 — KAT 8's headline claim, "THE NEGATIVE TEST FOR P4's LIVENESS", is vacuous; its stated mutation kill does not reproduce

**WHAT IS ASSERTED.** `crates/btctax-forms/tests/attestation.rs:259-291`: *"★★★ AND IT IS THE NEGATIVE TEST FOR P4. … This household never answers it, and files. … **If this KAT ever shows L7 being asked the acknowledgment question, that is a P4 liveness defect — the KAT is not to be adjusted to accept it.**"* Commit `b2cd2f24` records the kill: *"MUTATION A — the §63(e) election conjunct dropped from the P4 gate (`return_1040.rs:2447`) … `this fixture must FILE, but screen_absolute refused: Some(CharitableCwaUnresolved)`"*.

**WHAT IS ACTUALLY TRUE.** I re-ran that exact mutation in an isolated worktree. The KAT **passes**:

```
Experiment 1 (baseline):  6 tests run: 6 passed
Experiment 2 (drop `ar.deduction_is_itemized &&` at return_1040.rs:2447):
                          6 tests run: 6 passed      ← the plant, undetected
```

Instrumenting the fixture (`zero_inputs("Single")`, `w2_income = 30_000`, `crypto_gift_5k()`):

| | value |
|---|---|
| `ri.schedule_a.is_some()` | **false** |
| `ar.schedule_a.is_some()` | **false** ⇒ `cwa_claimed` = $0 |
| `ar.deduction_is_itemized` | false |
| `ar.charitable_carryover_out` | `[]` ⇒ `cwa_deferred_to_carryover` = $0 |

The gate is `ar.deduction_is_itemized && (cwa_claimed > 0 || cwa_deferred > 0)`. All three conjuncts are false. `build_golden_return` only creates `ri.schedule_a` when a GoldenInputs Schedule-A axis is non-zero, and this fixture has none; a ledger `Removal` does not create one (`return_1040.rs:453` — `schedule_a_parts` returns `None` on `ri.schedule_a == None`). And the $5,000 gift sits under the CapGainProp30 ceiling of `min(30%·30,000, 50%·30,000)` = $9,000, so `apply_170b` emits no carryover item. The `deduction_is_itemized` conjunct is never the binding one, so a P4 that became live for standard-deduction filers would leave this KAT green.

**HOW IT FAILS.** B1 in the strict sense: the checker was never observed discriminating. The quoted RED output is not reproducible from the described mutation.

**★★ AND THE TRAP WAS ALREADY WRITTEN DOWN, IN THIS BRANCH, BEFORE KAT 8 WAS AUTHORED.** `crates/btctax-core/src/tax/return_1040.rs:7268-7276`, phase 2's own scoping test:

> *"★★★ (1) THE STANDARD-DEDUCTION FILER, and the fixture has to be built with care or it proves nothing. **A return with NO `schedule_a` at all leaves `ar.schedule_a` = `None`, so lines 11/12 are $0 and the gate is already shut by the CLAIMED-amount conjunct — dropping `deduction_is_itemized` would then survive, and did (B1 caught it).** This fixture therefore HAS a Schedule A carrying a $5,000 cash gift …"*

KAT 8 built precisely the fixture that comment forbids. This is the B3 shape verbatim — the fix existed in the branch and nobody held both commits at once.

**MITIGATION (why Important, not Critical).** The production gate is correct and the guarantee *is* held: `the_cwa_question_is_never_posed_to_a_standard_deduction_or_small_gift_filer` case (1) is non-vacuous by construction. And KAT 8's *other* halves are real — the packet form-set assertion has a working discriminating twin, and `the_same_gift_on_an_itemizing_return_refuses_until_the_acknowledgment_is_answered` genuinely proves P4 fires for somebody. No filed figure is wrong.

**FIX (machine-verified).** Adding `i.state_income_tax = 1_000.0;` to the fixture makes `ar.schedule_a` Some with `charitable_noncash_12 = 5000`, leaves `deduction_is_itemized` false ($6,000 still loses to $14,600), and then the mutation *does* red: `screen_absolute` returns `Some(CharitableCwaUnresolved)`. Verify the `names == {"f1040"}` assertion still holds afterwards — I did not run that leg. Also correct the doc comment, and drop the mortgage-ceiling/line-9 sentence to what it actually is (this filer has neither input, so those two are not exercised here — they are non-vacuously covered by `the_mortgage_debt_limit_question_is_asked_on_inputs_and_refuses_on_the_deduction`, which is the model KAT 8 should have followed).

**SEVERITY: I**

### M-1 — the persisted corpus cell carries the PRE-RE-STEER §170(b) ceiling

`scripts/oracle/corpus.py:552` and the baked `full_return_goldens.json`: *"The gift is kept far under the §170(b)(1)(G) 60%-of-AGI ceiling **($264,000 here)**"*. The cell's AGI is $380,000 (240k + 40k + 100k), so the ceiling is **$228,000**. $264,000 is 60% of $440,000 — the steering that was abandoned. Commit `3146cb13`'s message says $228,000; only the artifact is stale. This is the note a future maintainer reads before changing the gift, and it names a headroom 36k larger than exists — the V2b oracle-disqualification the note exists to prevent. **SEVERITY: M**

### M-2 — same class, second instance, in the vector generator

`design/amt-form6251/gen_e2_vectors.py:218`: *"The gift stays far under the §170(b) 60%-of-AGI ceiling **($984,000 here)**"*. V30's AGI is $2,040,000 ⇒ **$1,224,000**. $984,000 is 60% of $1,640,000, i.e. an earlier `net_ltcg = 1,600,000` steering. **SEVERITY: M**

### M-3 — the new negative-AGI cell silently costs OTS its AMT witness, for a reason that is false on its face

Machine-reproduced:
```
_ots_amt_disqualified('Single', {}, {'L11': -3000.0}, {...}, year=2024)
→ "OTS 2024 applies no §170(b) 60%-of-AGI cash ceiling (gift 0 exceeds 60% of AGI -3,000), …"
```
`ots_direct.py:269-278` tests `cash_gift > 0.60 * agi` with no `cash_gift > 0` guard, and `0 > -1,800` is true. `single_loss_year_taxable_income_at_the_floor` has **no gift at all**; the sentence is baked into the golden and `expected_ots.amt` is `null`. The direction is fail-closed and both engines compute AMT $0 here, so nothing is mis-filed — but an instrument went quiet on the first household of a new shape, and `selftest_defect_years` (`ots_direct.py:282`) has no negative-AGI row. **SEVERITY: M**

### M-4 — `gen_e2_vectors.py`'s ABORT→SKIP loses the only SPEC↔fixture cross-check for committed ids

The regression guard re-derives committed vectors from **the fixture's own `inputs`**, never from the SPEC row. On the new skip path (`:283-286`) `wages`, `ltcg`, `salt` and `expect` are unpacked and discarded, so a SPEC row edited for an already-committed id is now silently ignored — and the routing `expect` dict and the §170(b)-ceiling ABORT are never re-applied to committed vectors. The commit's claim *"the safety it was protecting is intact"* holds for *silent rewriting* but not for *silent divergence*. Cheap fix: on skip, compare the row's inputs to `existing[vid]["inputs"]` and abort on mismatch. **SEVERITY: M**

### M-5 — `_triple_b_cell` did not get the `charitable_cash` fix that `_reconstruct_cell` did

`corpus.py:697` (fixed) and `corpus.py:645` (not) carry the *identical* itemized-by-components `any()` list. A future cell with `charitable_cash` and no `standard_or_itemized` flag would be labelled `itemized=False` by `_triple_b_cell` and credit triple-B coverage it does not provide — the same defect class the commit fixed one function over. Unreachable today (the cell carries the flag, and is off-grid at the wage axis anyway: 240,000 ∉ `W2`). **SEVERITY: M**

### M-6 — KAT 3's doc comment elides the exact clause where lines 20 and 27 DIFFER

From the committed text layer (`design/forms/extract/f6251--2024.txt:107,125`):
- L20: *"…line 5 of the Qualified Dividends and Capital Gain Tax Worksheet **or the amount from line 14 of the Schedule D Tax Worksheet**…"*
- L27: *"…line 5 of the Qualified Dividends and Capital Gain Tax Worksheet **or the amount from line 21 of the Schedule D Tax Worksheet**…"*

The KAT quotes L20 with an ellipsis and says *"Line 27 cites the same worksheet line"*. True of the QDCGTW alternative only. `line20 == line27` holds solely because v1 refuses every route to the Schedule D Tax Worksheet (`Form4952Required` / `Form4952DeclarationUnanswered`, and lines 18/19 refused upstream). That precondition is unnamed, and the assertion message — *"a divergence means one of them was re-derived"* — would misdirect if the SDTW path ever lands. This is the compression-hides-the-dropped-term shape CLAUDE.md names. **SEVERITY: M**

### N-1 — LIMITATIONS.md §1211/§1212 bullet names two write-back fields where the code writes three

*"`--write-carryover` stamps the **charitable** and **QBI-REIT/PTP** carryovers"*. `apply_carryover_writeback` (return_1040.rs:2858-2893) writes `charitable_carryover_in`, `qbi.reit_ptp_carryforward_in` **and** `qbi.qbi_carryforward_in`. Incomplete, not false — and the document's own earlier paragraph says *"the charitable and the two QBI carryovers"*, correctly. The load-bearing claim (capital-loss is **not** among them) is TRUE and is held by a KAT. **SEVERITY: N**

### N-2 — KAT 5's completeness check compares LENGTHS while its doc claims it compares SETS

*"the test asserts the set of cells checked equals `Form6251Map::money_cells()`"* — it asserts `expected.len() == map.money_cells().len()`. Adequate in practice (the exhaustive `let Self { … }` destructuring in `money_cells()` makes a silent map addition impossible), so this is an overclaim in the comment, not a hole. **SEVERITY: N**

### N-3 — KAT 9's `vanished` failure message gives a rationale false for 9 of its 32 keys

*"Each is a line the form's arithmetic reaches — 'add lines …' with nothing to add is still a computed zero"* is true of 1z/9/11/14/15/18/21/22/24/25d/32/33 but not of 1a/2a/2b/3a/3b/25a/25b/25c/26, which are entry lines. **SEVERITY: N**

### N-4 — the golden's own provenance string names LOW_END but not NON_INTERACTION

`gen_goldens.py:475` was updated in the LOW_END commit and not in the later one, so the baked `corpus` description is one list short of what `households()` now assembles. **SEVERITY: N**

## VACUITY CHECK

The ones I tried hardest to break, and what happened:

- **KAT 8 (the brief's ★ item) — BROKE IT.** Detailed above; the mutation was re-run and the KAT stayed green.
- **KAT 1's non-interaction** — attacked as "two absent forms are trivially equal". Genuinely guarded: the form must FILE (`expect` on the `Option`), `line17 > 0`, `ceiling_allowed > 0`, and `line16` must actually fall. Non-vacuous. It does *not* pin $264,000 — it asserts internal consistency plus the cross-foot — which is the right scope for a non-interaction KAT.
- **KAT 3's floor branch** — cannot pass by not being taken: `floor_must_bind` asserts `l4 > l1` *before* asserting `l5 == 0`.
- **KAT 6** — attacked as "a fixture helper answered the P7 question for the filer". Closed: half 1 sets `filing_form_4952 = None` and requires the refusal, and the $80,000 twin separates the 0% band from an engine that returns zero for everyone.
- **KAT 2** — `seen >= 3` floor holds it live across the corpus.
- **KAT 7** — the pair is asserted at both taxable-income branches, so a fix cannot overshoot in either direction. I re-derived both figures from the worksheet's own text (below); both correct.
- **KAT 9** — the assertion is an equality over the whole `line*` key set, so spurious/vanished/changed all red. All 32 values check out.
- **KAT 5** — the 41-cell pairing is bounded by the map's own count, which is bounded by an exhaustive destructure.
- **The re-steered corpus cell** — `line15 < line12` is asserted on *every* qualifying household, not just this one, so a future cell that loses the excess-MAGI leg reds.
- **KAT 4's counterfactual** — `without.line11 == 0` plus `with.line11 - without.line11 == with.line11`; if a future edit gives V30 AMT from another source, it reds rather than degrading.
- **The mortgage KAT** — genuinely non-vacuous: $4,000 mortgage, `!ar.deduction_is_itemized` asserted, `Some(false)` answered, no refusal; $25,000 twin refuses. This is what KAT 8 should look like.

**Machine checks I ran myself, not taken from the commit messages:**
`make check` → 2740 passed / 12 skipped. `cargo fmt --all --check` → clean. `corpus.py` selftest → 106 candidates, triple-A 12, triple-B 6, 181 feasible pairs all covered. `gen_e2_vectors.py` → regression guard OK, all 31 committed vectors reproduce. `verify_f6251.py` → **V30 mfj 2,000.00 / 2,000.00 AGREE · AMTI agrees**, 0 unexpected divergences (OTS skipped — `OTS_DIR` unset here). Golden JSON diff → the only removed lines are the `generated` date and the `corpus` string, so "regeneration is provably minimal" is verified from the diff itself. `_ots_amt_disqualified` misfire → reproduced directly.

**Brief items 2-5, verified sound:**

- **(2) the re-steer binds.** AGI $380,000 ⇒ line 15 = $130,000 < line 12 = $140,000, so line 16 takes the excess-MAGI leg; NIIT $4,940 = 3.8% × 130,000, and the planted MAGI defect would give $3,040. Both engines agree in the baked cell. The gift ($50,000) is under the real $228,000 ceiling, so OTS is not disqualified — only the *annotation* is stale (M-1).
- **(3) the two-level reasoning holds and G-6d is closable.** Level 1 reds Rust-vs-fixture, and the fixture's values come from `f6251_reference.py`, so it proves only that two same-team transcriptions agree — the reasoning is exactly right. Level 2 changes the figures the oracles score, and the enabling change is real: `verify_f6251.py` now hands the **raw** $25,000 to `A5a`/`e18400` so each engine applies its own §164(b)(6). I re-derived all 31 of V30's Form 6251 lines by hand from `f6251--2024.txt` — every one matches the fixture to the cent, regular tax $352,705, TMT $354,705, AMT $2,000 = 20% × the $10,000 add-back. taxcalc independently agrees on both the AMT and AMTI, and its known #3108 omission is the *standard-deduction* limb, so V30 (itemizing) is a case where taxcalc genuinely witnesses line 2a. Closable.
- **(4) KAT 1's corrected figures are right.** §170(b)(1)(G) is 60% for TY2024, and `charitable.rs:145` is `pct(dec!(0.60))`. AGI 440,000 ⇒ $264,000 deducted, $736,000 carried, cross-foot exact. The plan's $1,000,000 would indeed have been asserting a bug.
- **(5) KAT 9's line 12 is right, and so is KAT 7's pair.** `f1040--2024.txt:80-86`: nothing clamps line 12 to line 11; the floor is line 15's own instruction. And the Capital Loss Carryover Worksheet (`i1040sd--2025.txt:1820`) settles KAT 7 decisively — line 1 asks for the **unfloored** figure: *"If the amount would have been a loss if you could enter a negative number on that line, enclose the amount in parentheses."* Floor case: line 1 = (17,600), line 3 = 0, line 4 = 0, line 13 = **20,000**. Positive case: line 1 = 22,400, line 4 = 3,000, line 13 = **17,000**. Both as pinned.

**The six FINDINGS the phase reports:** the signed-AGI reader fix is correct and loses nothing (`paper_money`'s leading-minus guard is deliberately not applied to line 11 because line 11 is genuinely signed, and `on_paper_signed` still panics on an unparseable cell); `_reconstruct_cell`'s LT-loss→`None` changes no existing coverage (only one corpus household has a negative `long_term_capital_gains`, and it is the new one — selftest still covers all 181 pairs); the single-use generator fix is right but incomplete (M-4), and its sibling function was missed (M-5); the `F6251_MAP_2024` re-export is test-only. No existing test quietly changed what it proves.

## LIMITATIONS.md VERDICT — row by row

Every row verified TRUE against current code except one INCOMPLETE enumeration (N-1, three write-back
fields named as two, whose operative claim — that the capital-loss carryover is NOT among them — is
true and KAT-held). Rows verified: the three hand-mark classes and their conditions; the carryover
write-back; all four §163(h)(3)(B) rows (asked on an INPUT predicate so a standard-deduction filer IS
asked; adverse refuses only when itemizing; unanswered refuses; btctax collects interest not balance);
all three Form 4952 rows (asked on EVERY return; "yes" refuses, "no" computes via QDCGTW; the same
answer governs Schedule A line 9); the line-9 ceiling row and its formula; all four §170(f)(8) rows
(unanswered or "no" refuses; $250 is PER CONTRIBUTION; a standard-deduction filer is never asked; a
§170(b)-ceiling-zeroed year IS asked); all four §170(f)(11)(D) rows; the threshold-comparator row
("more than" vs "or more") against the statute; and all five §1211/§1212 rows including the 2025-
instructions provenance and the $20,000/$17,000 pair.

**No false row. One incomplete enumeration whose operative claim is true.**

## WHAT WOULD MAKE THIS REVIEW WRONG

If `screen_absolute` has a second, non-obvious path to `CharitableCwaUnresolved` that my worktree mutation happened not to disturb — then the commit's MUTATION-A output would be real and I-1 collapses to a wording nit; I checked and found only the one gate at `return_1040.rs:2447-2495`, and the mutated build was observed passing 6/6.
