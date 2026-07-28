# r2 review — r1-fold audit: for each of C-1..C-5 / I-1..I-12, is it CLOSED (plan states the correct thing AND a task checkbox owns making it true) or merely MENTIONED?

**Headline:** 14 of 17 genuinely CLOSED — C-1, C-2, C-4, I-2, I-3, I-4, I-5, I-6, I-7, I-8, I-9, I-10, I-11, I-12 (I-5/I-6 closed behaviorally in T3; see the file-map finding). PARTIAL: C-3, C-5, I-1 — all three fail on the same seam, the four new §8 vectors are empty dash rows that no checklist item constructs, and T1's replacement gate ("derive every TMT from c09600") is inoperative for V2/V2b (c09600 is the AMT, = 0 under both Part III readings) and unplumbed for gifts/MFS/refund/FTC. Plus one fold-introduced misquote: §3.1's line-1 comment states line 6's rule.

## [Important] §6 T1 (blocking checkbox 2), §8 header, §9 row 1, Tier-1 gate (r1 C-5)

**Problem:** The C-5 fold pins the right figures (§8 V2 = 113,654.50, V2b = 70,005.00, V5 = 26,271.00), asserts tentative_minimum_tax not amt, and strikes V3 as the canary — all closed. But the replacement independence mechanism does not work. T1 says "**Derive every §8 vector's TMT from `c09600`**, not by hand, and record them *before* any code exists. Hand figures are the cross-check, not the source", §8's header repeats it, §9 row 1's early warning is "V2/V2b/V5 disagree with `c09600`", and the Tier-1 gate is "§8 reconciles against `c09600`". `c09600` is Tax-Calculator's AMT liability, not the TMT: `scripts/oracle/gen_goldens.py:214-224` documents it as "(AMT c09600, credits c07100)" and `admit()` rejects on `if amt or credits`. AMT = max(0, TMT − regular), so on V2 (AMT 0) and V2b (AMT 0) — the two vectors the plan itself designates as the Part III discriminators — c09600 is 0 under BOTH the correct and the wrong reading and carries zero information about TMT. The re-specified gate therefore still cannot detect a wrong Part III on the only vectors that discriminate; it can only reconcile V4/V5/V6, and only as TMT = c09600 + regular tax. Worse, the derivation is unplumbed even where it could work: `_taxcalc_row` (gen_goldens.py:104-129) has no charitable-contribution field at all (V1/V2/V2b/V6 are entirely gift-driven), maps MARS to 1 or 2 only (MFS/V8 unreachable), and carries no state refund (V7) or foreign tax credit (V9); the only task that touches the oracle scripts is T5, which runs after T1. T1's BLOCKING checkbox is thus unsatisfiable as written — it will either stall the build or be waved through, and a builder who takes it literally could "derive" V2's TMT as c09600 + regular = 129,397.50 and pin the wrong Part III in the KAT.

**Fix:** Invert T1's item back to what r1 actually asked: pin V2/V2b (and V1/V3) from the line-by-line hand derivations already recorded in §8 and in `reviews/r1-8-worked-vector-recomputation-*.md` — recorded before any code exists, which is the independence C-5 required — and use c09600 as a CROSS-CHECK only where AMT > 0 (V4/V5/V6), reconciling TMT = c09600 + regular tax. State explicitly in T1 that reaching c09600 at all requires extending `_taxcalc_row` with cash charitable (e19800), MARS = 3, state refund and FTC fields, and either move that plumbing into T1 or make T5's oracle work a T1 prerequisite. Restate §9 row 1's early warning as "V4/V5/V6 disagree with c09600; V2/V2b are held by the pinned hand figures".

---

## [Important] §8 vector table (V7) + §6 T2 KAT list (r1 C-3)

**Problem:** Half of C-3 is closed: §3.1 now reads `line2b = − state_refund_taxable` with the i6251 p.5 authority, and T2 owns "§3.1 incl. the negative line 2b". The other half — r1's "Add one refund-bearing vector to §8 (all six have refund = 0)" — is folded as prose only: "| **V7** | — | — | — | — | — | — | — | — | ★ **state refund > 0** (line 2b) — derive in T1 |". No checklist item constructs it. T1's four boxes are PART_III.md, "derive every §8 vector's TMT from c09600" (which cannot express a state refund — `_taxcalc_row` has no such field), "KATs asserting `tentative_minimum_tax` … on V2, V2b and V5", and "watch them fail" — V7 is named nowhere. T2's KAT list is "every §8 vector" plus explicitly enumerated boundaries (phase-out, 26/28, MFS kicker, line 6 ≤ 0, line 1 ≤ 0) — no refund case. Because V7 has no inputs, "every §8 vector" is satisfiable with V7 still empty, so deleting the line-2b subtraction would leave the suite green — a direct violation of §5's "every guarantee ships with a test that reds when the guarantee is removed", which this project records as its recurring failure. (V10, the 0%-band vector, has the identical dash-row problem; it is non-blocking. V8/MFS survives only because T2 names $875,950/$1,142,550 explicitly.)

**Fix:** Fill V7's inputs and figures in §8 now — it needs no oracle, the refund is a pure subtraction from line 1 — and add an explicit T2 checkbox: "KAT: a return with `state_refund_taxable` > 0; mutation — delete the line-2b subtraction and it must red." Do the same for V10 while filling the table.

---

## [Important] §8 vector table (V9) + §6 T3 + §7 T9 (r1 I-1)

**Problem:** The rule side of I-1 is fully closed (§3.4 defines lines 8-11, `regular_tax` = line 10, the three consequences; T2 "§3.4 lines 8-11"; T3 narrows the trigger to line 7 > line 10; T7 "including lines 8/9/10"; T9 "Attach iff line 7 > line 10"). But the one NEW behavior r1 identified — the window `1040 L16 − FTC < line 7 ≤ 1040 L16` where AMT is $0 yet the form must be attached — has no vector and no test. V9 is a dash row ("★ **FTC > 0**, incl. the attach-with-$0-AMT window — derive in T1") that no checkbox constructs, and T1 cannot derive it (no FTC field in `_taxcalc_row`, and c09600 = 0 in the window by construction). T1's KATs cover V2/V2b/V5; T3's only mutation is "revert to the blanket refusal — the V1 KAT must red", which V1 (FTC = 0) cannot distinguish from the wrong rule; T9's skip KAT names no FTC case. Net: an implementation that gates on `AMT > 0` instead of `line 7 > line 10` passes every test the plan names, which is exactly the defect I-1 raised.

**Fix:** Fill V9 with inputs that put line 7 strictly inside `(1040 L16 − FTC, 1040 L16]` (v1 caps the §904(j) FTC at $600 — `box6_foreign_tax`, return_inputs.rs:75), and add two explicit checkboxes: T3 — "KAT: in-window return, AMT $0, must REFUSE in Tier 1; mutation — re-trigger on `AMT > 0` and it must red"; T9 — "KAT: the same return attaches a 6251 in Tier 2, with printed lines 8/9/10 each differing from the no-FTC values by the FTC."

---

## [Important] §3.1 (line 1) + §6 T2 KAT "line 1 ≤ 0" (r1 r1 non-blocking (§3.1 line-1 branch))

**Problem:** MIS-FOLD. The plan writes `line1 = taxable_income_L15   // "if zero or less, enter -0-"`. That quoted phrase is Form 6251 **line 6**'s instruction, not line 1's. Line 1 verbatim (2024 form, transcribed in `reviews/r1-form-6251-part-i-exhaustiveness-is-amti-taxable-income-the-l.md:59`): "Enter the amount from Form 1040 or 1040-SR, line 15, if more than zero. If Form 1040 or 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from line 11 of Form 1040 or 1040-SR and enter the result here. (If less than zero, enter as a negative amount.)" So a zero-taxable-income filer's line 1 is AGI − deduction, a NEGATIVE amount — the opposite of what the plan asserts, and reachable for anyone whose AGI is below the standard deduction. r1 filed this non-blocking asking the plan to "record line 1's zero-TI negative-amount branch and its one-line unreachability proof"; the fold instead states the wrong rule, and T2's checkbox "line 1 ≤ 0" will encode it as a KAT. Direction when wrong: overstates AMTI ⇒ over-refuses in Tier 1.

**Fix:** Replace the comment with the real branch — `line1 = if L15 > 0 { L15 } else { agi_L11 − deduction_L14 }  // may be NEGATIVE, i6251 line 1` — and add the one-line unreachability note r1 asked for (when the deduction exceeds AGI, line 1 + the line-2a add-back cannot reach the $133,300 exemption, so line 6 = 0 either way). Restate T2's KAT as "1040 L15 = 0 with L11 < L14 ⇒ line 1 is the negative amount, not −0−".

---

## [Important] §4 file map (rows for `return_refuse.rs` / `attribute.rs`) + §6 T3 (r1 I-5)

**Problem:** I-5 and I-6 are closed behaviorally — §2 states both declarations with direction-if-ignored, T3 owns "Add the §2 declarations (AMT capital-loss twin; qualified dwelling) as unanswered ⇒ refuse", and §9 row 4 guards them. But §4's file map gives the work only `return_refuse.rs` ("+ the two §2 declarations"), `return_1040.rs` and `attribute.rs`. In this tree a "unknown ⇒ refuse" declaration spans far more — pattern-matched from the existing `mortgage_all_used_to_buy_build_improve`: `return_inputs.rs` (the field), `questions.rs` (the `QuestionId`), `classifier.rs` (must call `c.declaration(field, QuestionId::…)` — classifier.rs:308-324 destructures the inputs struct by name), `btctax-input-form/src/spec/sections.rs` and `spec/coverage.rs` (the `FieldId` ↔ path map), `apply.rs`, plus `advisories.rs` and the example fixtures/goldens. The classifier registration is the load-bearing one: this project's stated architectural defect class is answered-ness, and a declaration that is destructured but not registered as a `declaration` silently reads as answered — the refusal never fires and the understatement I-5/I-6 identified ships anyway. This is the same species of omission as I-11, which r1 rated Important.

**Fix:** Add `return_inputs.rs`, `questions.rs`, `classifier.rs`, `btctax-input-form/src/spec/sections.rs`, `spec/coverage.rs` and `apply.rs` as Tier-1 rows in §4, and add a T3 checkbox: "each new declaration is registered via `c.declaration(...)` in `classifier.rs` and appears in the input-form coverage map; a classifier/coverage test reds if it is not — mutation: drop the registration and the unanswered-⇒-refuse KAT must red."

---

## [Minor] §10 SemVer (r1 I-9)

**Problem:** §10 says "**Tier 1: MINOR** — new public `form6251` API (`#[non_exhaustive]` so Tier 2 is additive), new `RefuseReason` variants, and a behaviour change". `RefuseReason` is public API (`lib.rs:17 pub mod tax` → `tax/mod.rs:26 pub mod return_refuse` → `return_refuse.rs:34 pub enum RefuseReason`) and is NOT `#[non_exhaustive]` (its derive line is `#[derive(Debug, Clone, PartialEq, Eq)]`). Adding variants breaks any downstream exhaustive match, i.e. semver-major for `btctax-core`, which is published. The plan applies exactly the right remedy to `Amt6251` (I-8) and misses it here — and I-9's adjudication itself leaned on "§10's MINOR" as an argument, so the wrong premise propagated.

**Fix:** Either mark `RefuseReason` `#[non_exhaustive]` as a T3 checkbox (a one-time break, cheap while there are no users) and keep §10 at MINOR, or state in §10 that Tier 1 is MAJOR for `btctax-core` because of the new variants.

---

## [Nit] §5 (last two bullets) + §9 row 3 + §6 T2 ("the two source-scan guards") (r1 I-12)

**Problem:** I-12 is closed — both rules are now §5 bullets and T2 owns "The two source-scan guards from §5". But one of them is specified as a mechanism that cannot do the job: "a source-scan test asserting §2's out-of-scope list is still refused" — a text scan cannot assert a refusal, and a grep-shaped test over source is brittle. The tree already has two stronger executable anchors for exactly this: `classifier.rs`'s by-name struct destructure (:308) forces every newly added input field to be explicitly classified, and `attribute.rs`'s hand-enumerated `RefuseReason` test (:348) is the established closed-list pattern.

**Fix:** Name the mechanism in §5/T2: the exhaustiveness guard is the classifier's by-name destructure plus a hand-enumerated out-of-scope list test in the `attribute.rs:348` style; keep the literal source scan only for the "no AMT constant outside `AmtParams`" rule, where a text scan is the right tool.

---

