# r2 review — r2 fold-integrity: read the folded PLAN.md as a fresh document and hunt for errors the fold itself introduced (numeric drift, dangling refs, §5-vs-task contradictions, V7–V10 ordering, overcorrection). Every code citation in the plan was resolved against current HEAD.

**Headline:** The arithmetic fold is clean — all 8 vectors, both crossover curves and both exposure peaks recompute exactly, all 17 r1 IDs land, and every cited path/line resolves. But the fold broke two things while fixing them: the two new §2 declarations refuse only on *unanswered* (the answered-adverse branch understates, and the exemplar it cites provably does not refuse there), and T1's "derive every TMT from c09600" inverts r1 C-5 into an unexecutable gate, because c09600 is the AMT, not the TMT, and is $0 for both discriminating vectors. 13 findings: 2 Critical, 5 Important.

## [Critical] §2 (line 2k / line 3 bullets), T3 bullet 2, §5 "Never understate" (r1 I-5, I-6)

**Problem:** All three places specify only the UNANSWERED branch of the two new declarations: §2 says "(equal-for-AMT? unknown ⇒ refuse)" and "(None ⇒ refuse)"; T3 says "as unanswered ⇒ refuse"; §5 says "each refuses when unknown." Nothing says what happens when the filer answers ADVERSELY — "my AMT capital-loss carryover differs" or "the dwelling is a houseboat." Built literally, those returns compute with no line 2k and no line 3 add-back and file an UNDERSTATED tax, contradicting §5's own "Never understate" invariant. Two things make this a live trap rather than a reading quibble: (a) §3.1's AMTI formula is `line1 + line2a + line2b` with no line-3 term at all, so there is nowhere to put the §56(b)(1)(C) add-back even if someone wanted to compute it; (b) the exemplar the plan tells the implementer to mirror does the OPPOSITE of refuse on the adverse answer — `mortgage_all_used_to_buy_build_improve` is destructured as `_` in the refuse screen (/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_refuse.rs:426) and the KAT at return_refuse.rs:993-1005 sets it to `Some(false)` and asserts `reason(&r) == None` with the comment "No brick: the screen does not refuse a truthfully-answered mixed-use return" — the adverse answer ZEROES the 8a deduction and proceeds. "Mirroring the existing pattern" therefore produces exactly the silent fall-through.

**Fix:** State the full ternary for both declarations in §2 and T3: unanswered ⇒ refuse; answered-adverse ⇒ refuse as well (btctax models neither line 2k's divergent AMT carryover nor line 3's non-qualified-dwelling interest); answered-equal/qualified ⇒ the term is 0. Delete or requalify the "mirroring `mortgage_all_used_to_buy_build_improve`" cross-reference — say it mirrors only the *unanswered⇒refuse* half and diverges on the adverse half, and say why (a zeroed 8a is conservative; a missing AMT add-back is not). Change §5's bullet from "each refuses when unknown" to "each refuses unless the answer is the AMT-neutral one." Add a T3 KAT per declaration pinning the `Some(adverse)` ⇒ refuse branch.

---

## [Critical] T1 bullet 2 (+ §8 header, §9 row 1, §6 Tier-1 gate) (r1 C-5)

**Problem:** The fold inverted r1's instruction and made the blocking gate unexecutable for the two vectors it names as the discriminators. r1 C-5 said: pin V5 and the TI-600,000 vector "to the cent from independently derived figures" and "cross-check §8 against c09600." The fold reads: "**Derive every §8 vector's TMT from `c09600`**, not by hand … Hand figures are the cross-check, not the source," echoed by §8's header and §6's gate. But `c09600` is not the TMT — it is the AMT, per the repo's own docstring: /scratch/code/bitcoin_tax/scripts/oracle/gen_goldens.py:215-216 ("→ [(AMT c09600, credits c07100), …]. The D-2 admission predicate reads these as oracle-2's 1040 L17 (AMT)") and :237 ("c09600 == 0 (AMT)"). c09600 = max(0, TMT − regular tax), so it is clamped to $0 whenever no AMT is owed, and TMT is unrecoverable from it. V2 (TMT 113,654.50 vs regular 129,397.50) and V2b (70,005.00 vs 87,918.50) both have AMT $0 under the correct reading AND under both wrong readings — the plan says so itself two bullets later ("V2's AMT is $0 under both readings, only its TMT discriminates") and §3.3 records that V2b's wrong-reading figure is $75,812.50, still below regular tax. So c09600 = 0 for every reading of both vectors and the canary is inert; §9 row 1's early warning ("V2/V2b/V5 disagree with c09600") is inert for two of its three vectors. This is the C-5 failure mode returning inside C-5's own fix, and T1 internally contradicts itself.

**Fix:** Restore r1's actual ordering in T1: hand-derive V1–V10 line by line and record them BEFORE any code (§8 already carries them, r1-verified to the cent), then use c09600 as a one-way cross-check on the vectors where it carries signal — V4 ($15,818.50), V5 ($26,271.00), V6 ($1,722.50) — and say explicitly that c09600 is inert for V1/V2/V2b/V3 because it is the clamped AMT, not the TMT. Rewrite §9 row 1's early warning to "V5 disagrees with c09600, or V2/V2b disagree with the hand-derived PART_III.md walk," and rewrite §6's Tier-1 gate the same way. If a true TMT oracle is wanted, name a second source (a hand-filled Part III in PART_III.md) rather than a field that does not exist.

---

## [Important] T3 bullet 4 (blast radius), §4 file map (r1 I-11)

**Problem:** T3's blast-radius sweep enumerates report / harvest / what-if / conservative-promote / TUI and `attribute.rs`, but misses the oracle harness's own test, which reds the moment T3 lands. /scratch/code/bitcoin_tax/crates/btctax-oracle-harness/tests/smoke.rs:98 pins `const EXPECTED_REFUSED: &[&str] = &["mfj_high_income_niit_and_addl_medicare"]`; the test at :101-113 asserts that household is refused, and :153-156 asserts the swept refusal set equals it exactly, with the comment "a change here means the AMT screen's behavior moved — update EXPECTED_REFUSED deliberately, don't paper over it." That anchor (corpus.py:390-399: MFJ, W-2 300,000, interest 5,000, ord div 12,000, qual div 9,000, LTCG 60,000) trips the screen but is a Tier-1 no-attachment case: AMTI 377,000, exemption 133,300, L12 243,700, L16 69,000, L17 174,700 → L38 55,772 vs regular L16 63,347, so line 7 ≤ line 10 and T3 makes it PROCEED. All three assertions flip. The anchor-exemption branch at gen_goldens.py:266-276 also becomes dead code, and its printed rationale ("btctax's conservative Form 6251 AMT screen; actual AMT $0 on both oracles") becomes false. §5 requires `make check` green from the first commit, so this is a red suite with no owning task.

**Fix:** Add `crates/btctax-oracle-harness/tests/smoke.rs` to §4 (Tier 1) and give T3 a bullet: after the refusal narrows, `EXPECTED_REFUSED` becomes empty (or is replaced by a household that still trips line 7 > line 10) and `the_amt_screen_anchor_is_reported_refused_in_default_mode` is retargeted, not deleted. Add gen_goldens.py's anchor-exemption branch (:266-276) to T5 — decide whether it is removed or retargeted.

---

## [Important] T5 bullet 2, §4 (`gen_goldens.py:257`, `corpus.py`), §9 row 5 (r1 I-7)

**Problem:** T5 says "Lift `gen_goldens.py:257`'s `c09600 != 0` rejection and widen `corpus.py`'s caps — these are the binding exclusions." Three problems, all verifiable in-tree. (a) The predicate is at :259 and is `if amt or credits:` — a COMBINED check over c09600 AND c07100, exactly the hazard the fold correctly fixed for main.rs:704 one bullet earlier; lifting it wholesale admits nonrefundable-credit households the corpus deliberately excludes. (b) It is not the only binding exclusion: a SECOND, btctax-side gate at gen_goldens.py:279-282 (`if l17 or l21:`) rejects any household whose paper 1040 L17 ≠ 0 — and L17 is precisely where AMT lands (Sch 2 L3 → 1040 L17), so lifting :259 alone cannot admit a single AMT-bearing household. (c) The stated rationale for both gates is not "btctax can't do AMT" — gen_goldens.py:255-257 says "a scenario the oracles see AMT/credits on is not L24-comparable, full stop," because ots_direct "is frozen at T8 and surfaces no AMT line of its own" (:234-236). btctax gaining AMT does not make an AMT household L24-comparable against OTS. Net: in Tier 1 (where T5 sits) btctax still refuses every AMT-owed return at the harness `refused` gate :266/:277, so lifting :259 is a no-op at best and admits non-comparable households at worst.

**Fix:** Rewrite T5's second bullet: keep the `credits` conjunct, and split the AMT conjunct so only c09600 is relaxed. State the three real binding gates in order — the harness `refused` gate (:266, cleared for free by T3), the taxcalc AMT conjunct (:259), and the btctax paper-L17 gate (:282) — and say which of them T5 touches. Then say what Tier 1 actually gains: screen-tripping, zero-AMT, no-attachment households (which already pass :259 and :282 and only need T3 plus wider corpus.py caps). Defer admitting AMT-bearing households to Tier 2, and add the OTS-comparability question ("what does the L24 cross-foot compare against when AMT ≠ 0?") as an explicit open item rather than leaving it implied.

---

## [Important] §5 last bullet ("§3.1's exhaustiveness is guarded executably"), T2 bullet 5 (r1 I-12)

**Problem:** §5 specifies the load-bearing guard as "a source-scan test asserting §2's out-of-scope list is still refused." That is unimplementable for eleven of §2's twelve items, and §2 itself says so two sections earlier: the list is "refused upstream **or uncapturable**." Only §57(a)(5) PAB interest is actually refused — /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/amt.rs:38 ("PAB interest is already refused (INT box 9 / DIV box 13, `screen_inputs`)") and :39 ("ISO/§1202/§4952/deprec./NOL/8801 are all out-of-scope inputs v1 never captures"). There is no refusal to assert for ISO, §1202, investment interest, depreciation, NOL/ATNOL, K-1, §56(a)(6), depletion, IDC, long-term contracts or pre-1987 installment sales — the guarantee for those is that no INPUT exists, which is a different (and differently-shaped) test. As written T2's implementer either writes a test that cannot pass or quietly narrows it to the one item that can, silently gutting the guard §3.5 is conditioned on.

**Fix:** Split the §5 bullet into the two guarantees §2 already distinguishes: (i) a refusal test asserting §57(a)(5) PAB interest still refuses; (ii) an input-surface test asserting the uncapturable eleven still have no `ReturnInputs` leaf — the mutate-and-diff coverage KAT at crates/btctax-input-form/src/spec/coverage.rs is the existing mechanism for "a new leaf appeared," so state whether the guard extends that or stands alone. Mirror the split in T2's bullet and in §9 row 3's mitigation.

---

## [Important] §3.5 last paragraph, §9 row 5 mitigation (r1 I-12 (same class), CONFIRMED §3.4)

**Problem:** §3.5 closes with "Carry a `debug_assert` that every AMT adjustment applied is a §56(b)(1) exclusion item, plus an 8801-recompute KAT asserting $0," and §9's row 5 names the same pair as the mitigation for "§3.5 wrong ⇒ a Form 8801 obligation." Neither appears in T1–T9 — this is precisely the defect r1 I-12 raised, reproduced for a different item. Worse, the 8801-recompute KAT is not buildable as scoped: there is no Form 8801 code anywhere in the tree (grep for "8801" across crates/ and docs/ returns only prose mentions at amt.rs:29, :37, :39), and §2 declares Form 8801 out of scope. "Recompute 8801 and assert $0" therefore requires building the very thing §2 excludes.

**Fix:** Give both items an owning task (T2 is the natural home for the `debug_assert`). Replace the 8801-recompute KAT with something buildable that carries the same guarantee — e.g. a KAT asserting that for each §8 vector the set of applied AMT adjustments is exactly {line 2a, line 2b, MFS kicker} and that every member is a §56(b)(1) exclusion item, which is what makes Form 8801 Part I lines 18/21 zero. If a genuine 8801 recompute is wanted, say so and reconcile it with §2's out-of-scope declaration.

---

## [Important] §4 file map (`return_refuse.rs` row) (r1 I-11)

**Problem:** The fold answered I-11's "the file map misses the refusal's blast radius" by adding `attribute.rs` only, but §2's two NEW declarations have a much wider radius, and the exemplar the plan names shows exactly how wide. `mortgage_all_used_to_buy_build_improve` appears in eleven places: crates/btctax-core/src/tax/return_inputs.rs (the field itself), classifier.rs:312,322 (the answered-ness classifier — this project's known architectural fault line), questions.rs:186,192, advisories.rs, return_refuse.rs, return_1040.rs, crates/btctax-input-form/src/spec/sections.rs, spec/coverage.rs:460, apply.rs, plus crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml and docs/examples/examples.md. §4 lists only return_refuse.rs ("+ the two §2 declarations") and attribute.rs — not even return_inputs.rs, where the field must be declared. coverage.rs is the sharp one: it derives the covered-leaf set BY OBSERVATION and goes red on any new in-scope `ReturnInputs` leaf that has no form `Field` ("That standing bite is the whole point of the task"), so T3 reds the suite the moment the fields land.

**Fix:** Expand §4's declaration row to the full set: return_inputs.rs, classifier.rs, questions.rs, advisories.rs, return_refuse.rs, input-form spec/sections.rs + spec/coverage.rs + apply.rs, attribute.rs, and the examples fixture + docs. Add a T3 bullet naming coverage.rs's mutate-and-diff KAT and sections.rs's form Field as required work, not incidental, and say whether the two new leaves get Fields or an `EXEMPT` entry.

---

## [Minor] §3.1 code block, line 1 (r1 r1 §5 non-blocking bullet 2)

**Problem:** `line1 = taxable_income_L15  // "if zero or less, enter -0-"` quotes 1040 line 15's own instruction and presents it as Form 6251 line 1's rule. Form 6251 line 1 has a negative-amount branch when 1040 L15 is zero — the exact branch r1's non-blocking item asked the plan to "record … and its one-line unreachability proof." The fold not only failed to record it, it inserted a quotation that erases it; T2 then lists a "line 1 ≤ 0" KAT with no stated rule to test against. Impact is bounded (`AbsoluteReturn.taxable_income` is already floored at 0, the error direction is overstated AMTI ⇒ fail-closed, and the branch cannot produce AMT: it requires deductions ≥ AGI, which caps the add-back at SALT ≤ $10,000, far under the $133,300 exemption) — hence Minor, not blocking.

**Fix:** Replace the comment with Form 6251 line 1's actual rule and add the one-line unreachability proof r1 asked for, then point T2's "line 1 ≤ 0" KAT at it explicitly (assert the plan's chosen behaviour AND assert AMT = 0 in that branch, so the shortcut is the thing under test).

---

## [Minor] §4 (`return_refuse.rs` row), T3 bullet 1, T9 bullet 2 (r1 I-9)

**Problem:** All three name `RefuseReason::AmtOwed`. That variant does not exist. The real one is `RefuseReason::AmtScreenTriggered` (crates/btctax-core/src/tax/return_refuse.rs, anchored at crates/btctax-input-form/src/attribute.rs:144 and hand-enumerated at :348 — the `:348` citation the fold added is correct, the identifier is not). r1 used the wrong name too, so the fold inherited it rather than inventing it, but the fold re-asserts it in three places including the file map, and T9's "do not delete the public variant" instruction cannot be followed against a name that isn't there. There is also a semantic point: T3 narrows the trigger from "the screening worksheet tripped" to "line 7 > line 10", after which `AmtScreenTriggered` is a misnomer for what the variant now means.

**Fix:** Replace `AmtOwed` with `AmtScreenTriggered` in §4, T3 and T9, and add one sentence deciding the naming: either keep the name and document that its meaning narrowed to Who-Must-File condition 1, or rename it (a rename is the breaking change T9 is trying to avoid, so say which you chose).

---

## [Minor] §5 ("No literal AMT constant outside `AmtParams`"), §4 (`tables.rs` row), T2 (r1 I-12)

**Problem:** §5 makes this a global constraint enforced by a T2 source-scan test, but §4 widens `AmtParams` by only two fields (`mfs_kicker_start`, `mfs_kicker_max`), and `AmtParams` (crates/btctax-core/src/tax/tables.rs:239-254) holds dollar thresholds only — no rates. The 26% / 28% rates, the 25% exemption phase-out rate and the 28%-bracket subtrahend ($4,652 / $2,326 MFS, needed by §3.3's `tax26_28`) have no home. And the file the plan explicitly keeps ("`amt.rs` | 1 | keep as cheap pre-filter") already carries `dec!(0.25)` at amt.rs:111 and `dec!(0.26)` at amt.rs:123, so the scan as worded reds against code T2 is required to preserve.

**Fix:** Scope the §5 rule (e.g. "no literal AMT *dollar* constant outside `AmtParams`") or add rate fields to `AmtParams` and say so in §4's tables.rs row. Either way note that the $4,652 subtrahend is derivable as 0.02 × `breakpoint_28pct` so it needn't become a new indexed constant, and confirm the scan's exemption for `#[cfg(test)]` boundary KATs, which by design contain the literals $232,600 / $1,218,700 / $1,751,900 / $875,950 / $1,142,550.

---

## [Minor] §10 SemVer vs §4 (`return_1040.rs`, `printed.rs` rows) (r1 I-8)

**Problem:** The fold answered I-8's breaking-change concern by putting `#[non_exhaustive]` on the new form6251 type (T2, §10) — but §4 also mutates two EXISTING public structs, and neither is `#[non_exhaustive]`: `AbsoluteReturn` gains `.amt` in Tier 1 (crates/btctax-core/src/tax/return_1040.rs:836-837 is a plain `#[derive(Debug, Clone, PartialEq, Eq)]`, and its own doc-comment stresses "No `Default`" precisely because "Every field is spelled out at the one construction site"), and `Schedule2Lines` gains `.line2/.line3` in Tier 2 (crates/btctax-core/src/tax/printed.rs:293-294, likewise plain). Both additions break struct-literal construction, so §10's "Tier 1: MINOR" and "Tier 2: MINOR" do not hold as stated — and T9 argues from §10's MINOR classification, so the inconsistency is load-bearing in at least one place.

**Fix:** Either mark both structs `#[non_exhaustive]` as part of the Tier-1 commit (a breaking change taken once, deliberately, before any Tier-2 field lands) or amend §10 to say Tier 1 is a MAJOR/breaking bump for `btctax-core`. Note in §4 that adding to `AbsoluteReturn` also touches every construction site and fixture, since it has no `Default`.

---

## [Minor] §8 rows V7–V10, T1 (r1 C-5, I-1)

**Problem:** V7–V10 have every input column set to "—": no wages, no LTCG, no deduction, no filing status. They are not vectors yet, they are briefs. T1's bullets say "Derive every §8 vector's TMT from c09600" — deriving presupposes a defined return, and no bullet says CONSTRUCT them. T2 then requires "KATs: every §8 vector," so T2 is blocked on work T1 does not name. V9 is the one that will actually bite: its defining property is the attach-with-$0-AMT window `1040 L16 − FTC < line 7 ≤ 1040 L16`, whose width is exactly the FTC — capped at $300, or $600 for MFJ (crates/btctax-core/src/tax/return_refuse.rs:212-215, `ftc_ceiling * 2` for MFJ, `ftc_ceiling: dec!(300)`). Landing a whole-dollar line 7 inside a ≤$600 window requires deliberate tuning and has no acceptance criterion anywhere in the plan.

**Fix:** Add a T1 bullet: "construct V7–V10 (inputs, not just purpose), record their full rows in §8, and only then derive/verify." Give V9 an explicit acceptance criterion — line 7 strictly inside `(L16 − FTC, L16]` with the FTC at its §904(j) ceiling — and state that this is the vector T3's proceed-vs-attach split and T9's skip KAT are both keyed to.

---

## [Nit] §3.4 (`line7 = TMT`), T1 bullet 3, §8 column header (r1 I-1)

**Problem:** §3.4 defines `line7 = TMT` and `line9 = line7 − line8`, but Form 6251 labels line 9 "Tentative minimum tax" — line 7 is the pre-AMTFTC tentative tax. The plan's usage is internally consistent and harmless across §8 as tabled (every current vector has FTC = 0, so lines 7 and 9 coincide), but T1's KAT is named `tentative_minimum_tax` and V9 is defined by FTC > 0, so exactly the new vector is the one where the field name becomes ambiguous against the form. Two smaller path nits in the same neighbourhood: §4's `oracle-harness/src/main.rs:704` should be `crates/btctax-oracle-harness/src/main.rs:704` (line 704 is correct), and `gen_goldens.py:215` is the docstring — the function opens at :214 and the c09600 read is at :223.

**Fix:** Pick a name that cannot collide — e.g. `line7_tentative_tax` and `line9_tentative_minimum_tax` — or state once in §3.4 that this plan uses "TMT" for line 7 and that lines 7 and 9 differ by the AMTFTC. Fix the two path/line citations.

---

