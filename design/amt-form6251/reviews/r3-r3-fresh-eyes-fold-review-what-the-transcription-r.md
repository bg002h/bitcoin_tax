# r3 review — r3 fresh-eyes fold review — what the transcription restructure broke. Read PLAN.md as a new document, diffed its content against r1-ADJUDICATION.md and r2-ADJUDICATION.md for silently-vanished closures, attacked the two DECISIONs, and re-verified every in-tree citation and every repeated dollar figure. Tax law, arithmetic, §8 V1–V6, the tier split and §5's no-8801 argument treated as settled and not re-derived.

**Headline:** No Critical. The restructure holds: all 5 r1 Criticals and both r2 Criticals are genuinely closed, every §8 figure is internally consistent, and the r1/r2 file-map and mechanism folds check out against source. But the fold introduced three new blocking defects of its own — §2's new rounding rule contradicts §8's cent-precise vectors and §4's absolute chain; §11's `#[non_exhaustive]` decision hard-errors one crate and dissolves the documented no-wildcard exhaustiveness guarantee §4 leans on; and T3's `EXPECTED_REFUSED` instruction picked the one branch of r2 I-7 the corpus makes impossible. Eight Minors, mostly precision lost in compression.

## [Important] §2 ("Rounding") vs §8 vs §4 Tier-2 / T2 KAT list (prior r2 Minor ("Part III's rounding order is unstated"))

**Problem:** §2 now says "whole dollars **per line** … each band's tax is rounded at its own line" — an unambiguous statement about the *computation*, not the printout. §8 pins the same quantities to the cent: line 9 = 447,200.50 (V5) and 113,654.50 (V2); AMT = 15,818.50 (V4) and 1,722.50 (V6); regular tax 420,929.50; balance due 83,225.50. §4's Tier-2 row then requires the AMT to thread into `AbsoluteReturn.total_tax`/`amount_owed`, which is a cents chain (`ar.regular_tax` = 420,929.50). The two cannot both hold, and T2's "KATs: every §8 vector" is therefore unexecutable as written. Worked on V5: per-line rounding gives L31 = round(54,442.50), L38 = 447,200 or 447,201, L10 = round(420,929.50) = 420,930, L11 = **26,270 or 26,271** — §1's own headline exemplar ($26,271.00) moves by $1 depending on a rule the plan never states. The repo already has the answer and the plan does not cite it: `AbsoluteReturn` is exact, and `printed.rs:5-8` rounds each printed line and sums the *already-rounded* lines (Reading A, `design/full-return/ROUNDING_AUTHORITY.md`). This also silently decides V9: "line 7 strictly inside (L16 − FTC, L16]" is a ≤$600 window, so whether lines 7 and 10 are compared rounded or exact changes what V9 even is.

**Fix:** State which layer `Form6251` is. Recommended, matching the existing architecture: `Form6251` carries **exact** (cents) values and lives in the absolute chain alongside `AbsoluteReturn`; §8's figures are that layer and T2's KATs pin them at the cent; the **printed** Form 6251 rounds per line and cross-foots per `printed.rs`'s existing discipline, pinned by T7's byte-reproducible V5 golden. Say explicitly which representation the Who-Must-File line-7-vs-line-10 comparison uses (recommend exact, so the attach decision cannot flip on a rounding tie) and re-scope the "two orders differ by $1" KAT to the printed layer.

---

## [Important] §11 SemVer (prior r2 Minor ("§10's SemVer is wrong three ways"))

**Problem:** §11 chooses "Mark all three `#[non_exhaustive]` in the Tier-1 commit (cheap; `no-users-yet`)". It is not cheap, and for one of the three it is actively harmful. (a) `Schedule2Lines` is built with a **struct literal in another crate** — `crates/btctax-forms/tests/full_return_forms.rs:430` (`let lines = Schedule2Lines { line4, line11, line12, line21 }`). `#[non_exhaustive]` makes that E0639, a hard compile error, against §6's "`make check` green from the first commit." (b) `RefuseReason` is matched **exhaustively from another crate** at `crates/btctax-input-form/src/attribute.rs:24-26`, whose module header states the guarantee outright: "The `match` has **NO `_` wildcard arm**, so a newly-added `RefuseReason` variant is a compile error until it is placed." `#[non_exhaustive]` forces a wildcard arm there, converting exactly the compile-time totality that §4 cites as the reason attribute.rs is in the blast radius (and that r1 I-11 relied on) into a silent catch-all — leaving only the hand-written list at `:348`, which by construction cannot notice a new variant. This is a fold that trades a structural guarantee for a SemVer label, in a repo whose one named architectural fault line is exactly "held by convention, not construction".

**Fix:** Take r2's other branch: declare **Tier 1 MAJOR for `btctax-core`** and leave all three plain derives — `no-users-yet` makes the major bump free, and it costs nothing structural. If a `#[non_exhaustive]` is still wanted, restrict it to `AbsoluteReturn` (verified: every literal is in-crate — `return_1040.rs:1287`, `printed.rs:1270`, `printed.rs:2132/2358`) and add one sentence saying `RefuseReason` deliberately stays exhaustive because `attribute.rs`'s no-wildcard match is load-bearing, and `Schedule2Lines` stays plain because `btctax-forms` constructs it by literal.

---

## [Important] T3 bullet 5 ("Retarget `smoke.rs`'s `EXPECTED_REFUSED` … retarget, do not delete") (prior r2 I-7)

**Problem:** r2 offered two options — "`EXPECTED_REFUSED` becomes empty (**or** is retargeted to a household that still trips line 7 > line 10)" — and the fold kept only the branch the tree makes impossible. No baked household can have AMT > 0: `gen_goldens.py:259`'s `if amt or credits` rejects any `c09600 != 0` and the comment at :256-258 says the substance check "applies to EVERYONE (anchors included)"; the anchor exemption at :266-276 bypasses only the *harness refusal* gate. Nor can one be in the FTC attach window — `_taxcalc_row` has no FTC field and no corpus household sets foreign tax. And the sole current anchor becomes clean: `mfj_high_income_niit_and_addl_medicare` (`corpus.py:390-399`: MFJ, W-2 $300k, interest $5k, ord div $12k, QD $9k, LTCG $60k) trips the screen only on the breakpoint (worksheet line 11 = 243,700 > 232,600) while its Form 6251 line 7 ≈ $55.8k against line 10 ≈ $63k — so after T3 it proceeds, exactly as `smoke.rs:93-97`'s own comment predicts. So there is nothing to retarget to. Worse, `EXPECTED_REFUSED` cannot merely be emptied either: `the_amt_screen_anchor_is_reported_refused_in_default_mode` indexes `EXPECTED_REFUSED[0]` (`smoke.rs:101-102`) and panics on an empty slice.

**Fix:** Replace the bullet with the actual work: `EXPECTED_REFUSED = &[]`; **invert** `the_amt_screen_anchor_is_reported_refused_in_default_mode` so it asserts the anchor now *proceeds* and reconciles on every compared line — that inversion is the end-to-end proof that T3 un-refused the target population, and is worth strictly more than the test it replaces; `sweep_check_reconciliation`'s `refused == EXPECTED_REFUSED` then asserts emptiness and `admitted >= 10` moves by one. Keep r2's "do not delete" attached to the *test function*, not to the constant. (T5 already owns the now-dead `gen_goldens.py:266-276` anchor-exemption branch.)

---

## [Minor] §4 "Tier 1 — the two declarations" / T3 bullet 4 (prior r2 I-1)

**Problem:** The file map faithfully reproduces r2's list — including r2's omission. `crates/btctax-input-form/src/spec/registries.rs` is missing, and it is the actual site of the work: it holds both TOTAL maps (`question_to_field` at :251, documented "a new `QuestionId` is a compile error here"; `field_to_question` at :235) and generates the 7 delegating declaration `Field`s at **literal** `FORM_QUESTIONS` indices 0–6 with the mortgage dedup at index 7 (:150). Separately, r2 I-1's fix asked the plan to "say whether the leaves get form `Field`s or `EXEMPT` entries" and the fold dropped it: §4/T3 say only "Update `spec/mod.rs`'s `decl_count`" without saying to what, or whether the dwelling question becomes a `Decl*` section field or a Schedule-A-owned leaf deduped like `SaMortgageAllUsed`. That one choice fixes `decl_count` (7→8 vs 7→9), the adjacent `decls.fields.len() == 8` assert at `spec/mod.rs:116-119` (which §4 does not mention), and the `coverage.rs:460` path strings.

**Fix:** Add `btctax-input-form/src/spec/registries.rs` to §4 with its two total maps and the index-literal delegating fields named. Add one sentence to T3 deciding the placement of each new leaf (recommend: the dwelling question dedups to a Schedule-A leaf like its sibling; the carryover question becomes a `Decl*` field) and state the resulting `decl_count` and `decls.fields.len()` targets.

---

## [Minor] §3 ★ DECISION (the ternary) / §4 `return_refuse.rs` row / §11 (prior r2 C-B)

**Problem:** The ternary's *behaviour* is now stated correctly, but its *mechanism* is not assigned. The registry loop at `return_refuse.rs:543` fires only on `(q.live)(ri) && (q.get)(ri).is_none()` — unanswered only. A `Some(adverse)` refusal is a **value-refusal**, which in this codebase is a separate hand-written block plus its own `RefuseReason` variant (the `ForeignTrust` / `DualStatusAlienUnsupported` pattern at `return_refuse.rs:552-568`). §4's `return_refuse.rs` row names only the kept `AmtScreenTriggered` and "the registry loop at :543"; nothing names the two new variants, their detail strings, or their `attribute()` arms. Two knock-ons: §11's disclosure ("now **refuse** until the new declarations are answered") is false for this branch — an answered-adverse return refuses permanently, so its detail text must not tell the filer to answer a question they already answered; and the polarity of each question is unstated, which matters because `testonly.rs`'s prescribed "answer `true` for both" is neutral only if both are phrased affirmatively. §3's line-2k cell quotes the form as *"…carryover that is **different** for the AMT"*, whose natural phrasing inverts the polarity and would make `true` the adverse answer — refusing the entire baked corpus.

**Fix:** Add to T3: two new `RefuseReason` variants (one per declaration) raised by value-refusal blocks in the `ForeignTrust`/`DualStatusAlien` style, with their `attribute()` arms; write each question in its affirmative form ("is this a qualified dwelling for AMT?", "is your AMT capital-loss carryover the same as the regular one?") so `Some(true)` is unambiguously neutral and `testonly.rs`'s `true` is right; and split §11's sentence into the two behaviour changes (unanswered ⇒ refuse until answered; answered-adverse ⇒ refuse, unsupported in v1).

---

## [Minor] §8 table header vs §1 / T3 / T9 (prior r2 Nit (line 7 vs line 9 naming))

**Problem:** §2 fixes the naming correctly and §8's column is now "line 9 (TMT)". But the Tier-1/Tier-2 boundary is **line 7 > line 10**, and §8 carries a column for neither. For V1–V6 the table is complete only by accident (FTC = 0 ⇒ line 7 = line 9, and line 10 = the "Regular tax" column). V9 is defined by FTC > 0 — precisely where both identities break — and V9 is the vector that pins the attach test and T3's loosening mutation. As the schema stands there is no cell to record V9's discriminating figure.

**Fix:** Add `line 7` and `line 10` columns to §8 (identical to line 9 / regular tax on V1–V6, so the existing rows are unchanged) so V9's row can be recorded, and so T3's `line7 > line10` mutation has a pinned operand to read.

---

## [Minor] §1 "Out of scope" list vs §6 exhaustiveness guard (ii) (prior r2 I-6)

**Problem:** §6's input-surface guard is written against "the **eleven** uncapturable items", and §1 supplies exactly eleven (ISO, §1202, §4952, depreciation, NOL/ATNOL, K-1, §56(a)(6), depletion, IDC, long-term contracts, pre-1987 installments). Form 6251 Part I has twenty adjustment lines plus line 3; that list leaves **2m (passive activities), 2n (loss limitations), 2o (circulation costs), 2q (mining costs) and 2r (research & experimental costs)** unnamed. This does not dispute r1's settled line-by-line walk — nothing is missing from the tax — but a guard enumerated from §1's prose list will not cover those five, and under §0's own thesis an exhaustiveness guard is precisely the place where a prose list should not stand in for the numbered lines.

**Fix:** Re-key §6's guard (ii) to the form's numbering — "every Part I line 2c–2t and line 3 either refuses or has no `ReturnInputs` leaf" — enumerated by line number in `PART_III.md` (which T1 already produces line-by-line), and drop the count "eleven" from §6.

---

## [Minor] T4 bullet 2 (`testonly.rs:48-51,58-59`) (prior r1 I-10 / r2 I-10)

**Problem:** Two different files share the basename. §4 establishes `tax/testonly.rs` = `crates/btctax-core/src/tax/testonly.rs` (`:33-39` `answer_all_live_declarations` — correct). T4 then cites bare `testonly.rs:48-51,58-59` for "every bundled journey is deliberately sized under the screen"; those lines in `tax/testonly.rs` are `ty2024_params()`'s `std_deduction` table. The journey warnings actually live in **`crates/btctax-cli/src/testonly.rs`** (J6 River: "the kitchen-sink household clears the 2024 Form-6251 AMT-screen worksheet by only a thin margin — a corpus editor who enlarges the sale, income, or donation must keep the household on the computable side of that screen"; J6 Coinbase: "Amounts kept small so the return stays under the AMT screen"). Both comments go stale the moment T3 narrows the trigger, and no task owns them. Same class, unowned: `crates/btctax-forms/tests/full_return_forms.rs:425-429`'s rationale ("the return is refused outright if the Form 6251 screen trips. A 0 printed there would be a lie") is the justification for `schedule_2_fills_part_ii_and_leaves_part_i_blank`, which Tier 2 must retarget.

**Fix:** Correct the citation to `btctax-cli/src/testonly.rs:48-51,58-59`, and add a T3 sub-item sweeping the doc comments whose stated rationale is "the AMT screen refuses this" — the two J6 corpus warnings and `full_return_forms.rs:425-429` — the way the whole-surface-sweep rule requires when a taxonomy changes.

---

## [Minor] §0 defect table (r1 C-1 row) / T1 bullet 1 / T2 mutations / §10 (prior r1 C-1)

**Problem:** §0 characterises r1 C-1 as "two rounds on a Part III question **line 20 answers in one sentence**" — now also normative in `CLAUDE.md` and `FOLLOWUPS.md` §G-5. r1 actually ruled the question a *false dichotomy* in which "both candidate figures are wrong, each half-right": line 20 settles the **positioning** half (which the original plan already had right); the half that was wrong needed line 16's and line 22's "smaller of", line 17's floor at zero, and the line-32 skip. T1's Part III bullet inherits the same asymmetry, naming only the 20/27-vs-13 parentheticals. Consequently the one defect r1 headlined as "THE ONE THING MOST LIKELY TO PRODUCE A WRONG FILED NUMBER" ($75,812.50 vs $70,005.00) has a pinned vector (V2b) but no mutation, while the two lesser defects (line 2b, MFS kicker) each got one, and §10's risk table has no row for it.

**Fix:** Extend T1's Part III bullet to require the cap lines verbatim (16, 17, 22 and the line-32 skip) alongside 20/27/13, add to T2's mutations "delete the line-16/line-22 `min` ⇒ the V2b KAT reds (75,812.50 against the pinned 70,005.00)", give §10 the matching risk row, and soften §0/§G-5's claim to "line 20 answers the *positioning* half in one sentence; lines 16/22/32 answer the cap half".

---

## [Minor] T1 ★ DECISION (the independence mechanism) → T2 (prior r2 C-A)

**Problem:** The decision is right in substance, but the link it exists to create is never drawn. T1 commits "a **JSON fixture**"; T2's KAT bullet says only "KATs: every §8 vector" and never says those KATs *read* the fixture, and nothing names the file's path, schema (per-line? per-vector?), or which crate deserializes it. The whole content of the mechanism is "the numbers T2 asserts are the numbers T1 recorded first", and that is the sentence that is missing — as written, T2 can retype figures and satisfy every bullet. Secondarily, T1 also owns the `_taxcalc_row` plumbing (`e19800`, `MARS = 3`, state refund, FTC), which is code; if it lands in the same commit as the hand-derivation, the git-history argument for independence is weakened by the plan's own ordering.

**Fix:** Add to T2: "the vector KATs deserialize T1's committed fixture; no §8 figure is retyped in test source", and name the fixture's path and shape in T1 (e.g. `crates/btctax-core/src/tax/fixtures/form6251_vectors.json`, one object per vector with every line 1–40). Say the fixture + `PART_III.md` commit lands **before** the taxcalc-plumbing commit.

---

## [Minor] §5 Discharge (the adjustment-set KAT) (prior r2 I-5)

**Problem:** The KAT is specified as "the set of applied AMT adjustments is exactly {line 2a, line 2b, MFS kicker} **and each is a §56(b)(1) exclusion item**". The line-4 MFS kicker is not a §56(b)(1) item at all — it is the §55(d)(3) exemption clawback, part of the rate/exemption structure rather than an adjustment or preference. A builder writing the assertion literally must either mis-classify it or quietly weaken the assertion that discharges the entire §5 argument. (The conclusion is unaffected: an exemption clawback creates no deferral item, so Form 8801 Part I lines 18/21 stay $0.)

**Fix:** Split the assertion: the applied **adjustment** set is exactly {line 2a, line 2b} and each is a §56(b)(1) exclusion item; the MFS line-4 kicker is asserted separately as a §55(d)(3) exemption phase-out that introduces no deferral item — which is the property the no-8801 argument actually needs.

---

