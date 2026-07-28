# r2 review — Buildability / executability of the r1-folded plan — could a competent engineer with no knowledge of this conversation execute it, and would its gates catch a mistake? (NOT tax law; r1 owns that.)

**Headline:** The fold is substantively right but not yet executable: T1's blocking oracle step is inoperable for the two vectors that discriminate the very rule r1 C-1 fixed (taxcalc's `c09600` is Form 6251 line 11, not line 7, and it is identically $0 on V2/V2b), T3 still proceeds on a screen CLEAR without applying the line-7-vs-line-10 boundary §3.4 says it must, and the two new §2 declarations have no plumbing — the tree holds declarations in a derived registry (`questions.rs::FORM_QUESTIONS`) with eight consumers, none of which appear in §4 or any task.

## [Critical] T1 (§6) + §8 + §9 row 1 (r1 C-5 (recurrence))

**Problem:** T1's blocking instruction is "Derive every §8 vector's TMT from `c09600`". That is not executable for the vectors it exists to protect. Verified in the installed oracle (`.venv/lib/python3*/site-packages/taxcalc/calcfunctions.py:2673`): `c09600 = max(0., line9 - max(0., taxbc - e07300 - c05700))` — c09600 IS Form 6251 line 11 (the AMT), and the `AMT()` function returns only `(c62100, c09600, c05800)`. `line7`/`line9` (the TMT) are internal locals, exposed by no Records variable. TMT is therefore recoverable ONLY when c09600 > 0. Of the six tabled vectors, V1/V2/V2b/V3 all have AMT = $0, so c09600 ≡ 0 under BOTH Part III readings; V4 and V6 are Part-III-insensitive (r1 verified their regular bottoms exceed $583,750). **V5 is the only §8 vector whose c09600 can discriminate the reading at all — and V5 has L17 = $250,000 > 0, so it never exercises the L16/L22 cap or the L32 skip.** V2b, the plan's own named "discriminating canary" and the only excess-<-gain exemplar, is exactly the vector the oracle cannot see. §9 row 1's early warning ("V2/V2b/V5 disagree with c09600") is unobservable for two of its three vectors. This is r1 C-5 reproduced one level down: the gate guarding the plan's one acknowledged unknown still cannot detect the wrong answer for the cap rule, and encoding the cap wrong overstates TMT (Tier 1 over-refuses; Tier 2 files an overstated Schedule 2 L2).

**Fix:** State the recovery formula and the witness construction in T1: (a) `line9 = c09600 + max(0, taxbc − e07300 − c05700)`, `line7 = line9 + line8` — and say explicitly that it yields nothing when c09600 = 0; (b) for each Part-III-sensitive geometry, pair the vector with a **witness** whose AMT > 0 so c09600 becomes informative (e.g. hold V2b's L12/L13/L20 geometry — excess < gain, L17 = 0, L32 skip — while lowering regular tax until AMT > 0), and record BOTH; (c) name the invocation: `.venv` already has taxcalc 6.7.2, but `gen_goldens.py` has no ad-hoc single-household entry point — say which script/mode T1 adds and where the derived figures are recorded (PART_III.md table? a committed JSON fixture the T2 KATs read?).

---

## [Important] T3 bullet 3 (§6) vs §3.4 consequence 3 (r1 I-1 (half-folded))

**Problem:** T3's decision procedure reads: "Screen clears ⇒ AMT $0. Screen trips ⇒ compute; line 7 ≤ line 10 ⇒ proceed; else refuse." On the CLEAR path nothing tests line 7 against line 10 — yet §3.4 consequence 3, three pages earlier, names that exact rule as wrong: "There is a window — `1040 L16 − FTC < line7 ≤ 1040 L16` — where AMT is $0 yet the form must still be attached. 'Proceed with no form' (T3) … [is] wrong there." The window is reachable on the clear path: `amt.rs::amt_should_file_6251` compares the worksheet's line 12/13 against `regular_tax_l16` with NO Sch-3-L1 subtraction (`return_1040.rs:1424-1431` passes `ar.regular_tax`), and its header argument that the FTC "cancels from both sides" is about the AMT-owed test, not the ATTACH test — where line 10 = L16 − FTC and line 7 is un-reduced, so it does not cancel. An FTC ≤ $600 is a live input (`ForeignTaxOverCeiling` only fires above the ceiling). §8 V9 is specified to exercise this window, but no task says what the CLEAR path does with it. (Honest note: the window is a pre-existing under-attachment, not a Tier-1 regression — but the plan claims to fix the boundary and does not.)

**Fix:** Make T3's rule uniform: compute Form 6251 lines 7–10 whenever Schedule 3 line 1 > 0 (or unconditionally, and drop the screen to a pure fast-path for FTC = 0), then branch on line 7 ≤ line 10 in BOTH arms. Alternatively state and prove the sufficiency "screen clear ∧ FTC = 0 ⇒ line 7 ≤ line 10" and route every FTC-bearing return through the computation. Pin V9 as a KAT on the clear path, not only the trip path.

---

## [Important] §4 file map, T3 bullet 1, T9 bullet 2

**Problem:** `RefuseReason::AmtOwed` does not exist. The tree's variant is `RefuseReason::AmtScreenTriggered` (`crates/btctax-core/src/tax/return_refuse.rs:161`, produced at `return_1040.rs:1433`, anchored at `btctax-input-form/src/attribute.rs:141-143`, enumerated by hand at `attribute.rs:348`). A repo-wide grep for `AmtOwed` returns only the design docs. Three places instruct "keep" / "do not delete" a symbol that isn't there, and the plan never says whether T3 renames `AmtScreenTriggered → AmtOwed` (a public-API break with a compile-error blast radius, and the same breakage r1 I-9 objected to for removal) or adds a second variant and keeps both (in which case §4/T3 must say which one fires when).

**Fix:** Name the real variant. Decide and state: keep `AmtScreenTriggered` and narrow its trigger to line 7 > line 10 (cheapest — the anchor note at `attribute.rs:142` "computed at `report`, not a v1 form field" stays valid and needs only a text refresh), OR rename and list the call sites. Also note `RefuseReason` is **not** `#[non_exhaustive]` (return_refuse.rs:33), so every variant added or renamed is a downstream compile break.

---

## [Important] §2 (the two new declarations), §4 file map, T3 bullet 2 (r1 I-5, I-6)

**Problem:** §2 adds two declarations "mirroring the existing unanswered-question pattern" / "mirroring `mortgage_all_used_to_buy_build_improve`", but §4 lists only `return_refuse.rs` and `attribute.rs`. The actual pattern is a derived registry with eight consumers, none named anywhere in the plan: `btctax-core/src/tax/questions.rs` (`FormQuestion` entry + a `QuestionId` variant + `QuestionId::ALL`), the `ReturnInputs` field itself (`return_inputs.rs`, `#[serde(default)]` ⇒ TOML surface) and its shape test at `return_inputs.rs:490-517`, the registry refusal loop `return_refuse.rs:543`, the print boundary `packet.rs:326`, `btctax-cli/src/cmd/answer.rs` (`income answer`, incl. the hand-listed id sets at `:262-266` and the per-id liveness forcer at `:342-344`), `btctax-input-form/src/seam.rs` (`FieldId`), and `btctax-input-form/src/spec/mod.rs` — whose test hard-codes `decl_count == 7` and `decls.fields.len() == 8` and asserts the `FieldId ↔ QuestionId` map is TOTAL, so two new questions are a guaranteed red until every one of those is updated. Separately unstated: both liveness predicates must be INPUT-level (the registry runs inside `screen_inputs`, before compute), which means every filer with a 1098 or a carryforward is asked an AMT question even when nowhere near the AMT — the same over-ask fork §2.7 deliberately resolved for the mortgage question.

**Fix:** Add a T3 sub-item enumerating the registry surface above, state the two `QuestionId` variants and their exact liveness predicates (`schedule_a.mortgage_interest_1098 > 0`; `capital_loss_carryforward_in.short > 0 ∨ .long > 0`), state the `ReturnInputs` field names and that they are `Option<bool>` defaulting to `None`, and add `questions.rs`, `return_inputs.rs`, `seam.rs`, `spec/mod.rs`, `answer.rs`, `packet.rs` to §4. Acknowledge the over-ask and say it is accepted (as §2.7 did).

---

## [Important] T3 (blast radius) — missing from every task

**Problem:** Two consequences of adding registry declarations are unaddressed. (1) `btctax_core::tax::testonly::answer_all_live_declarations` (`testonly.rs:33-39`) auto-answers every live registry question **`false`** — `let ans = matches!(q.id, QuestionId::MortgageAllUsedToBuyBuildImprove)` — for the whole fixture surface, including `build_golden_return` (`testonly.rs:611`), which is what the oracle harness assembles (`btctax-oracle-harness/src/main.rs:693`) and therefore the entire baked corpus. corpus.py gives every itemized household `MORTGAGE_ITEMIZED = 25_000`, so a new dwelling question silently answers "not a qualified dwelling" across the corpus (changing the AMT add-back) and a new carryover question silently answers "not equal for AMT" (refusing). This is precisely the silently-answering-for-the-filer class the answered-ness work exists to prevent, landing inside the test oracle. (2) Every **stored** `ReturnInputs` with a 1098 or a carryforward flips from computable to refused on upgrade (`#[serde(default)]` ⇒ `None`). §10 records only the opposite direction ("previously-refused returns now compute").

**Fix:** T3 must state the `answer_all_live_declarations` answer for each new id and why (the dwelling question's safe fixture answer is `true`, matching the mortgage precedent; the carryover twin's is `true`), and list the fixtures/goldens that move. §10 adds the second behaviour change: previously-computable stored returns with a 1098 or a carryforward now refuse until answered.

---

## [Important] T5 (§6), §4 file map row for gen_goldens.py (r1 I-7 (recurrence))

**Problem:** T5 calls `gen_goldens.py:257`'s `c09600 != 0` rejection "the **binding** AMT exclusions". For Tier 1 it is not binding and lifting it admits nothing: the Tier-1 population is exactly the line-7 ≤ line-10 population, whose AMT is $0, so c09600 was never what rejected them. What rejected them is `corpus.py`'s domain caps (W-2 $270k / LTCG $20k, far below the $1,218,700 phase-out start) plus the harness-refusal gate at `gen_goldens.py:277` — which T3 fixes on its own. Worse, there is a **third** admission gate the plan never mentions: `gen_goldens.py:280-283` rejects any household whose btctax paper `1040.line17 != 0` ("AMT/credit ⇒ not L24-comparable"), and the L24 cross-foot in `btctax-forms/tests/golden_packet.rs` is built on that precondition. This is the same class of error r1 I-7 found — the named edit is inert, and the real gate is elsewhere.

**Fix:** Rewrite T5: the Tier-1 win comes from widening `corpus.py`'s caps into the AMT region plus T3's un-refusal; the `c09600 != 0` lift belongs to **Tier 2** and must be paired with the `1040.line17 != 0` gate at :280-283 and with whatever the L24 cross-foot needs when L17 > 0. State what happens if a household is admitted where taxcalc reports c09600 > 0 and btctax computes line 7 ≤ line 10 (a real disagreement — is that a bake-time hard error?).

---

## [Important] §8 rows V7–V10 vs T1 bullet 2

**Problem:** T1 is BLOCKING and its instruction is "derive every §8 vector's TMT" — but four of the ten vectors have every input cell blank ("— — — —"). They cannot be derived; they must first be **constructed**, and the plan gives no recipe or acceptance test for any of them. V9 is the hard one: it must land TMT strictly inside `(L16 − FTC, L16]`, a band at most $600 wide (the §904(j) ceiling is $300/$600, `FullReturnParams::ftc_ceiling` = 300), which is a search problem, not a table fill. V8 (MFS) and V7 (state refund) are also not expressible through the existing oracle plumbing: `_taxcalc_row` (`gen_goldens.py:104-129`) maps `MARS` as `2 if Married/Joint else 1` (MFS is MARS 3), carries no state-refund field, no foreign-tax field, and — relevant to V1/V2/V2b/V6, which all have donations of $85k–$900k — **no charitable-contribution field at all**.

**Fix:** Give V7–V10 concrete inputs in §8 (or make constructing them an explicit, separately-checkable T1 deliverable with a stated acceptance test each), and list the `_taxcalc_row` additions T1 needs: `e19800`/`e19700` (charity), `e00700` (state refund), `e07300` (FTC), and MARS 3 for MFS. Note V9's search explicitly so it is not mistaken for a table fill.

---

## [Important] §5 mutation rule vs T3 / T9 (r1 I-1, I-10)

**Problem:** §5 requires every guarantee to ship with a test that reds when the guarantee is removed. The guarantee most likely to be reverted is the one r1 had to correct — the boundary is line 7 > line 10, **not** AMT > 0 — and no task carries a mutation for it. T3's only mutation is "revert to the blanket refusal — the V1 KAT must red", which is the opposite direction: it catches a *narrowing*, not the *loosening* to `AMT > 0` that would silently re-admit the V9 attach window. T9's mutation covers only the Tier-2 packet skip.

**Fix:** Add to T3: a V9 KAT asserting the return REFUSES (Tier 1) with the mutation stated — replace `line7 > line10` with `amt > 0` and the V9 KAT must red. Add the mirror to T9 for the attach decision. These two mutations, not the blanket-refusal revert, are what hold r1 I-1.

---

## [Important] §3.5 (last paragraph) — owned by no task; §9 row 5 (r1 I-12 (recurrence))

**Problem:** §3.5's no-new-Form-8801 argument is explicitly "**Conditional on** §2's two new declarations and §5's exhaustiveness guard", and discharges the condition with "Carry a `debug_assert` that every AMT adjustment applied is a §56(b)(1) exclusion item, plus an 8801-recompute KAT asserting $0." Neither artifact appears in T1–T9. §9 row 5's mitigation cites the same two artifacts. This is exactly r1 I-12's defect ("an unowned mitigation in a risk table is not a mitigation") recurring for a different item — §5's two source-scan guards were correctly given to T2, but this pair was not.

**Fix:** Add both as T2 checkbox items, next to the two source-scan guards.

---

## [Important] §4 file map row `btctax-cli/src/cmd/… (report)` — owned by no task; Tier 1 gate (r1 r1 Minor "Tier 1 gate / missing task")

**Problem:** The row that makes the AMT computation visible to the filer ("print AMTI / exemption / TMT / AMT so the filer sees the number that un-refused them") is Tier-1 scope in §4 but appears in no T1–T5 checkbox, is not in the Tier-1 gate, and has no golden — so Tier 1 can go green without it. Its path is also unresolvable: there is no `crates/btctax-cli/src/cmd/report.rs`; the report command lives in `crates/btctax-cli/src/cmd/tax.rs` with rendering in `crates/btctax-cli/src/render.rs`. Same for the TUI: §4 lists no TUI file at all, though T3 bullet 4 names "TUI" among the surfaces to enumerate and r1 asked for the tax-panel line (`btctax-tui-edit/src/edit/tax_inputs.rs:768 focus_refusal` is the anchor consumer).

**Fix:** Give the report/TUI line an owning task (T4 is the natural home, alongside the hand-built packet), name the real files, and add "the AMTI/exemption/TMT/AMT line renders with a golden" to the Tier-1 gate.

---

## [Important] §3.3 code block + T2 bullet 1 vs §5 rounding (r1 I-8)

**Problem:** §3.3 says `preferential_tax(bp, bottom, pref)` "already exists (`compute.rs:57`)" and the pseudocode composes TMT from it, while T2 must emit "the full line vector for Parts I–III" (r1 I-8: ~30 printed boxes) under §5's "Whole-dollar rounding per SPEC §3.1, **per line**". The helper cannot do both: it returns a 4-field aggregate (`PrefSplit { at_0, at_15, at_20, tax }`) and rounds **once, to cents**, at the end (`tax = round_cents(at_15 * 0.15 + at_20 * 0.20)`). Form 6251 prints lines 31, 34 and 37 as separate whole-dollar boxes summed at line 38, so `round(a) + round(b) ≠ round(a + b)` can put the emitted line 38 (and hence lines 40 → 7 → 11 → Sch 2 L2) $1 away from the value the helper returns. No task resolves which is authoritative.

**Fix:** State in T2 that Part III computes per-line whole-dollar values and that `preferential_tax` is reused (if at all) only for the band SPLIT (`at_0/at_15/at_20`), with the per-band tax rounded per line; or say TMT is the rounded per-line sum and the helper's `tax` field is not used. Add a KAT on a vector where the two orders differ.

---

## [Important] §4 file map (return_1040.rs is Tier 1 only) vs T8

**Problem:** Tier 2 must thread the AMT into the **compute-side** tax chain, not just the printed one: `AbsoluteReturn` (`return_1040.rs:896-935`) carries `regular_tax`, `tax_after_credits`, `schedule_2_other_taxes`, `total_tax`, `amount_owed`, all computed today with no AMT term. §4 assigns `return_1040.rs` to Tier 1 only and gives Tier 2 just `printed.rs`, the forms crate and the CLI. If Tier 2 fills the PDF but never adds the AMT to `AbsoluteReturn.total_tax`, `btctax report` understates the balance due by exactly the AMT while the emitted packet shows it. T8's "KAT: V5's balance due with AMT included" is the only thing that would catch it, and only if balance due is read from the absolute chain.

**Fix:** Add a Tier-2 `return_1040.rs` row to §4 (AMT → Sch 2 L2 → L3 → 1040 L17 → L18 → L24 → amount owed) and make T8's KAT assert both chains (absolute `amount_owed` and the printed L24) so a one-sided fix reds.

---

## [Minor] §4 row `return_1040.rs`

**Problem:** "`screen_absolute` calls the computation; `AbsoluteReturn.amt`" is self-contradictory: `screen_absolute(&ri, &ar, &params)` takes `&AbsoluteReturn` immutably (`return_1040.rs`, called at `btctax-oracle-harness/src/main.rs:704`), so it cannot populate a field on it. Either the computation runs twice or the ownership is different from what §4 says.

**Fix:** Say `assemble_absolute` computes Form 6251 and stores it on `AbsoluteReturn`; `screen_absolute` reads `ar.amt` (or the line vector) and branches. Note that line 10's operands are already on hand: `ar.regular_tax` (L16) and `ar.foreign_tax_credit` (Sch 3 L1).

---

## [Minor] T2 bullet 3 ("fix the existing screen's missing kicker in this task") (r1 C-4)

**Problem:** No insertion point is given, and the screen is not shaped like Form 6251. `amt_should_file_6251` (`amt.rs:83-120`) is written in the *screening worksheet's* line numbers — `line3`, `line5` (AMTI analogue), `line7`, `line11` — with no line-4 analogue, and the only authority the plan quotes is i6251 p.9 (a Form 6251 line). An implementer has to decide unaided whether the kicker attaches to the screen's `line5` before the exemption comparison (and hence also feeds the phase-out branch at `line5 > phaseout_start`). T2's KAT bullet lists the MFS boundaries once; only §9 row 2 says "in both the screen and the computation".

**Fix:** Name the variable and the ordering (`line5` after the state-refund subtraction, before the exemption test), cite the screening worksheet's own step if it has one, and say explicitly that the boundary KATs are required on BOTH `amt.rs` and `form6251.rs`.

---

## [Minor] §5 gate vs T1 bullet 4 ("Watch them fail")

**Problem:** §5 makes `make check` green "from the first commit", but T1 is a standalone BLOCKING task whose deliverable is KATs against a `tentative_minimum_tax` API that T2 has not yet created — those tests do not fail, they fail to compile, and the commit is red. Nothing else enforces T1's actual purpose (figures recorded BEFORE the implementer chooses a reading), so squashing T1 into T2 would quietly dissolve the discipline.

**Fix:** State the mechanism: T1 commits the derived figures as data (PART_III.md table plus a committed JSON/`const` fixture) with no code dependency — green — and T2's KATs read that fixture. The "recorded before" property is then enforced by git history, not by a red suite.

---

## [Minor] T1 bullet 2 ("Hand figures are the cross-check, not the source")

**Problem:** This inverts the authority hierarchy: it makes a third-party open-source engine the source of truth and the primary-source-verified r1 figures a mere cross-check, and it gives no tie-break rule for a disagreement. (For Part III specifically the risk is low — I read taxcalc's implementation and it matches the settled reading: `line20 = dwks14` regular-side, `line16 = min(line6, line15)`, `line22 = min(line6, line13)`, and the `line22 == line32` skip — but V7/V8/V9 exercise paths that were never checked against it.)

**Fix:** Add one sentence: the form and its instructions are the authority; a taxcalc disagreement is a finding to adjudicate against the PDF, never a figure to encode.

---

## [Minor] §1 ("The trigger rule: AMT is owed when …")

**Problem:** §1's prose defines the Tier-2 population as "AMT is owed", which §3.4 then corrects to line 7 > line 10 — a strictly larger set that includes the $0-AMT attach window (§8 V9). §1's table is right; only the prose beneath it survives with the old framing.

**Fix:** One clause: Tier 2 is every return with line 7 > line 10, which includes but is not limited to the AMT-owed population.

---

## [Minor] §10 SemVer (r1 I-9)

**Problem:** "Tier 1: MINOR — … new `RefuseReason` variants" uses the opposite convention from the one r1 I-9 applied to variant removal. `RefuseReason` is not `#[non_exhaustive]` (`return_refuse.rs:33`), so *adding* variants is as breaking as removing them for any downstream exhaustive match (which is why `attribute.rs:141` is a compile error, by design).

**Fix:** Either mark `RefuseReason` `#[non_exhaustive]` in T3 (and say so), or state the 0.x convention being used so §10 and r1 I-9 are consistent.

---

## [Nit] §3.3 pseudocode

**Problem:** `preferential_tax(REGULAR bottoms, pref)` is two arguments and plural; the real signature is `preferential_tax(bp: &LtcgBreakpoints, bottom: Usd, pref: Usd) -> PrefSplit` (`compute.rs:57`). "REGULAR bottoms" never resolves to a named quantity.

**Fix:** Write it out: `bottom` = Form 6251 line 20 = QDCG Worksheet line 5 as figured for the regular tax; `bp` = the regular-side `LtcgBreakpoints`.

---

## [Nit] §10 lineage / FOLLOWUPS.md §G-4 (lines 302-311)

**Problem:** The plan cites G-4 as its lineage, but G-4 still carries the pre-r1 framing an executor would read first: the Tier-1 boundary as "AMT is $0 / Who Must File is not met" and "the printed forms for a zero-AMT filer are byte-identical to today's" — both superseded (r1 I-1 and the r1 Minor T4 folded into §1/T4).

**Fix:** Add one line to G-4 pointing at PLAN §3.4 for the boundary and §1/T4 for the hand-built-packet framing.

---

