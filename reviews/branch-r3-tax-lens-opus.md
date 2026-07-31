# r3 — TAX LENS (Opus)

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `d0aad6f..HEAD` (9 commits).

**Brief:** [`reviews/BRIEF-r3.md`](./BRIEF-r3.md) — scoped by harness rule **B3**. The brief named five
seams (S1 the MFS §63(f) boxes, S2 the new donation refusal, S3 the shared predicate's mirror defect,
S4 the advisory surface, S5 Form 8995 line 3), listed seven settled facts not to re-derive, and
forbade fresh-audit scope creep.

**Result: 0 Critical / 4 Important / 3 Minor. The gate is RED.**

★★★ **Seam S3 paid.** The brief asked whether making `spouse_63f_boxes_count` one shared predicate
had created the mirror of the bug it fixed — *an answer counted that was never asked*. The reviewer's
answer is precise and better than the question: the mirror defect is **not** in the deduction path
(two independent guards prevent it) but **is** in the advisory layer, which was never migrated. That
is finding 1, and it is an understatement path.

**Reviewer output reproduced VERBATIM below.**

---

VERDICT: fix-before-merge: 4 Important (0 Critical), plus 3 Minor

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/advisories.rs:579-590 (and 463, 484, 503)
CLAIM: fd9c15f made `spouse_63f_boxes_count` the ONE predicate for the spouse's §63(f) boxes and
       migrated two consumers; the four §63(f) advisories are still keyed on
       `filing_status == Mfj` and were not touched, so on the exact returns fd9c15f exists to
       enable, one advisory is FALSE in the understatement direction and three go silent.
FAILURE: MFS return, spouse DOB 1955-03-02 and `blind = Some(true)`, with the three i1040gi
       conditions set via `income import` (`spouse_had_no_income = true`,
       `spouse_not_filing_a_return = true`, `can_be_claimed_as_dependent_spouse = false`).
       `AgedBlindBoxes::for_return` returns `count() == 2` — I ran
       `the_mfs_spouse_63f_boxes_need_all_three_conditions_and_fail_closed`, it passes, both
       boxes ARE checked and both are in L12.
       (a) FALSE POSITIVE, understatement direction. `advisories.rs:579` fires
       `Mfs63fSpouseBoxesForgone { boxes: 2 }` on that same return, and its message
       (advisories.rs:230-239) tells the filer: *"btctax counts a spouse's aged/blind boxes only on
       a JOINT return, so they are not claimed … three things btctax does not ask and cannot verify
       … your tax is OVERSTATED by $3,100 and the boxes are yours to check by hand."* Every clause
       is false at HEAD: btctax now asks, verifies, and has already claimed them. A filer who acts
       on it reduces a correct L12 by $3,100 on a return signed under §6065.
       (b) FALSE NEGATIVE, §3.4 breach. On the same qualifying MFS return, a spouse with no DOB on
       file, or an unanswered spouse death question, or `blind = None`, genuinely forfeits a box —
       `is_aged`'s `(None, None)` arm and `spouse_blind: s.blind == Some(true)` both fail closed —
       but `AgedBoxForfeitedNoDob` (:463), `AgedBoxForfeitedDeathUnanswered` (:484) and
       `BlindBoxForfeitedNotDeclared` (:503) all require `Mfj` for the spouse term, so nothing is
       said. §3.4 permits a conservative omission only if the filer is TOLD.
EVIDENCE: i1040gi--2024.txt:1433-1438 — *"If your filing status is married filing separately and
       your spouse was born before January 2, 1960, or was blind at the end of 2024, you can check
       the appropriate box(es) … if your spouse had no income, isn't filing a return, and can't be
       claimed as a dependent on another person's return."* The gate in
       `questions::spouse_63f_boxes_count` implements this correctly. The advisory layer does not
       consult it: `advisories.rs:579` is `if ri.filing_status == FilingStatus::Mfs { if let
       Some(sp) = ri.header.spouse.as_ref() { … } }` with no reference to the predicate.
       Three of the four still carry comments asserting the migrated behaviour — :470 *"spouse only
       on MFJ (mirrors `AgedBlindBoxes::for_return`, which is the derivation that actually decides
       the deduction)"* and :500 *"MFS never counts the spouse — mirrors `AgedBlindBoxes`"* — which
       is the precise claim fd9c15f falsified.
       The commit message asserts *"THE COUPLING IS RESOLVED BY MAKING IT ONE PREDICATE"* and names
       two consumers; there are six. `FOLLOWUPS.md` §G-20 carries both halves side by side
       (:1499-1501 *"never on MFJ, where a claim that the boxes were forgone would be flatly false"*
       and :1503 *"✅ CLOSED — the boxes are now CLAIMABLE on MFS"*) and never reconciles them, so
       this is a miss, not a filed deferral.
       The advisory's own test is named `the_mfs_spouse_63f_advisory_fires_only_when_a_box_was_
       actually_forgone` and pins the MFJ case with *"an advisory there would be flatly false"* —
       the identical reasoning now applies to a qualifying MFS return and no case was added.
```

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_1040.rs:675-702
CLAIM: The §G-21 restriction gate is keyed on the Form 8283 Section-B REPORTING threshold
       ($5,000), but the tax consequence it exists to prevent is a §170 substantive one that has
       nothing to do with $5,000 — so a filer who declares a restricted donation of $5,000 or less
       files a full-FMV deduction btctax's own code says is too large, with no refusal, no
       advisory, and no mark on the form.
FAILURE: Itemizing filer donates BTC with a claimed deduction of $4,000, the donee's right to
       dispose is restricted, and the filer answers the new universal **Yes**
       (`donations_had_restrictions = Some(true)`). `year_donation_deduction == 4000`, not
       `> 5000`, so the whole match block at :678 is skipped — no refusal. `crypto_charitable_gifts`
       (return_1040.rs:613-626) still pushes `CharitableGift { class: CapGainProp30, amount:
       long_fmv }`, i.e. the FULL $4,000 at fair market value, onto Schedule A line 12. Section A
       carries no 5a/5b/5c, and `form_8283_printed` writes nothing because `no_restrictions !=
       Some(false)`. So btctax holds an explicit filer declaration that the deduction is overstated,
       discards it, and files the inflated number. There is no advisory:
       `donations_had_restrictions` appears nowhere in `advisories.rs`.
EVIDENCE: The commit's own reasoning, in `SkippableId::DonationsHadRestrictions`
       (questions.rs:586-591): *"A **Yes** to any of the three limbs shrinks or kills the §170
       deduction (Reg §1.170A-7), and btctax deducts at full FMV — so a Yes means the number is
       WRONG."* Nothing in that sentence is conditioned on $5,000.
       Reg §1.170A-7(a)(1) (no deduction for a contribution of less than the taxpayer's entire
       interest) and §170(f)(3)(A) apply at every dollar amount. The $5,000 line is
       §170(f)(11)(C)'s **appraisal / reporting** threshold — it decides which SECTION of Form 8283
       you file (forms.rs:398-404, *"§170(f)(11)(C): 'more than $5,000' — strict `>`"*), not whether
       the deduction is allowable.
       The scoping test `a_section_a_year_never_asks_the_restriction_questions`
       (return_1040.rs:1969-1989) only exercises `donations_had_restrictions: None`; the
       `Some(true)` case below the threshold is not covered, which is why the suite is green.
```

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_1040.rs:675-702
CLAIM: The same gate fires on years that file NO Form 8283 at all, because it reads the ledger
       aggregate rather than whether the return claims a §170 noncash deduction — so a `Some(true)`
       filer taking the standard deduction is permanently and unescapably blocked from a return
       that is entirely correct, and the `None` message asserts a falsehood.
FAILURE: Single filer, $6,000 of BTC donated, restricted, no other itemized deductions. `itemized =
       $6,000 < standard $14,600`, so `choose_deduction` (return_1040.rs:466 `Auto => standard.max
       (itemized)`) takes the standard deduction: **no §170 deduction is claimed and no Form 8283
       is attached**. The restriction is irrelevant to every figure on the return. Yet
       `year_donation_deduction == 6000 > 5000`, so:
       - `Some(true)` ⇒ `RefuseReason::DonationRestrictionsUnresolved`. Via `resolve_and_screen`
         (resolve.rs:199-203) the year becomes `Uncomputable`, which blocks report / optimize /
         what-if / export entirely. The only escapes are answering "No" (perjury) or deleting a
         real ledger event. The message's stated reason — *"the deduction it would compute is too
         large"* — is false: it computes none.
       - `None` ⇒ the same refusal, whose message opens *"this year files a Form 8283 SECTION B
         (donations over $5,000)"*. It does not; escapable, but the assertion is false.
       The same over-reach hits an itemizing filer whose §170(b) ceiling drops Schedule A line 12
       to $500 or less — `printed.rs` notes *"§170(b) ceilings legitimately make L12 smaller than
       the sum of the 8283's per-donation amounts"* — so no 8283 attaches there either.
EVIDENCE: The emitter's own scope, `packet.rs:606-610`: *"Form 8283 files only when the return
       ITEMIZES and its printed noncash gifts clear the $500 threshold printed on Schedule A line
       12 — **a standard-deduction year with donations files none**."* The gate at
       return_1040.rs:675 tests only `crate::forms::year_donation_deduction(state, year) >
       QUALIFIED_APPRAISAL_THRESHOLD`, with no itemization or L12 term. `FOLLOWUPS.md`:1434-1436
       states the intended scope as *"a year that genuinely files a Section B"*, which is the
       predicate the code does not implement. Both this and the finding above are fixed by one
       change: key the gate on whether the return actually claims the noncash §170 deduction, and
       on the presence of a `Some(true)` declaration rather than on the reporting threshold.
```

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_1040.rs:1739
CLAIM: `apply_carryover_writeback` stamps `capital_loss_carryforward_in_provenance = Computed` on
       a value it never writes and that `AbsoluteReturn` does not carry — a false claim of
       knowledge that silences the §1212(b) half of the advisory §G-20a was built to add.
FAILURE: Filer runs `btctax report --tax-year 2024 --write-carryover`. The function assigns
       `charitable_carryover_in` (:1724-1731), `qbi.reit_ptp_carryforward_in` (:1732) and
       `qbi.qbi_carryforward_in` (:1734) from real `AbsoluteReturn` fields, then at :1739 stamps
       `capital_loss_carryforward_in_provenance = Computed` **without assigning
       `capital_loss_carryforward_in`** — there is no such field on `AbsoluteReturn` (its
       carryover-out set is `charitable_carryover_out`, `qbi_reit_ptp_carryforward_out`,
       `qbi_carryforward_out`, return_1040.rs:1030-1035), and nothing anywhere else in the crates
       writes `ri.capital_loss_carryforward_in` from a prior year. 2025's row therefore keeps its
       default `Carryforward { 0, 0 }`, now flagged authoritative. The `cl` term at
       advisories.rs:595-597 (`… == Usd::ZERO && provenance == User`) goes false, so
       `BenefitCarryoversNotStated` stops naming the capital-loss carryover — for a filer who may
       genuinely have one, including one btctax's own 2024 run displayed as `carryover_out`
       (render.rs:1341). Direction is overstatement, so the omission is lawful; §3.4 permits it
       only if the filer is told, and this is the one thing that told them.
EVIDENCE: The field's own doc comment, added in the same commit (return_inputs.rs:516-524):
       *"`Computed` means btctax **derived it from a prior year it actually computed**; `User` (the
       default) means it is the filer's or nobody's."* And the advisory's shipped text
       (docs/examples/examples.md, `PRIOR-YEAR CARRYOVERS NOT STATED`): *"btctax has no capital-loss
       carryover … on file, and **it did not compute your prior year**, so it cannot tell 'you have
       none' from 'nobody asked'."* After the write-back the stamp asserts the opposite of both.
       The charitable sibling at :1740 is honest — :1724 writes the value. Only the capital-loss
       stamp is unfounded.
```

```
SEVERITY: Minor
WHERE: crates/btctax-forms/src/form8283.rs:511-537
CLAIM: The 5a/5b/5c "No" write sits outside the `match section` block, so a full return filing a
       Section **A** 8283 prints three marks in Section B Part II — the same "a mark the form has
       no place for" class this branch fixed in 3bcf3a0, one commit earlier.
FAILURE: Itemizing filer with $3,000 of crypto donations (Section A per forms.rs:400) who answered
       the always-offered universal "No". `fill_one`'s `match section` closes at :468; the
       `if let Some(header) = filer` block at :475 runs for BOTH sections, and at :518
       `no_restrictions == Some(false)` pushes checks into `Form8283[0].Page2[0].c2_1[1]` /
       `c2_2[1]` / `c2_3[1]`. Page 2 goes out with three answered Section B Part II questions on a
       return whose Section B Part I is empty. Not fabricated testimony — the answer is the filer's
       own and is true — which is why this is Minor rather than a repeat of 3bcf3a0.
EVIDENCE: f8283--2025.txt:83-84 — *"Complete lines 5a through 5c **if conditions were placed on a
       contribution listed in Section B, Part I**"*; the three questions are each scoped to *"the
       donated property"* listed there. Reachable and already exercised: the existing test
       `the_full_return_8283_names_the_filer_and_prints_whole_dollars`
       (full_return_forms.rs:2196) uses `Form8283Section::A` and was changed by d6ff290 to pass
       `Some(false)`; it passes, and asserts nothing about the page-2 marks. The new test's own doc
       comment says *"A SECTION B row — over $5,000 — because 5a/5b/5c live on page 2 and **only
       Section B prints them**"*, which is false of the code it guards.
```

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/packet.rs:271-278; questions.rs:543-546 and 846-848;
       advisories.rs:470 and 500
CLAIM: Five doc comments describing the MFS spouse-box behaviour were left asserting the
       pre-fd9c15f rule, and each states something false about the code it annotates.
FAILURE: A future reader (or a future "harmonisation") reads the authoritative-sounding text and
       reinstates the MFJ-only rule, or concludes the MFS answers are inert and drops the fields.
       This is the mechanism CLAUDE.md names for the Form 8995 line-3 doc ("a struct doc calling it
       dead plumbing is exactly how a future audit concludes the field is vestigial").
EVIDENCE: packet.rs:271-278, on `AgedBlindBoxes::for_return` itself: *"The spouse's boxes count
       only on MFJ HERE … btctax captures none of those three conditions, so it forgoes the boxes
       on MFS … Filed as `FOLLOWUPS.md` §G-20; the forfeit is currently UNADVISED"* — all four
       claims false at HEAD. questions.rs:543-546 on `SkippableId::SpouseDiedDuringYear`: *"Live
       only on MFJ … so on MFS this question's answer could never move a figure."* questions.rs:
       846-848, directly above the line fd9c15f changed: *"★ MFJ ONLY — `AgedBlindBoxes::
       for_return` counts a spouse box on no other status, so on MFS this answer is inert."*
       advisories.rs:470 and :500 as quoted in the first finding.
```

```
SEVERITY: Minor
WHERE: crates/btctax-input-form/src/spec/sections.rs:1029-1046; advisories.rs:558-573
CLAIM: The new Carryforwards fields stamp `CarryProvenance::User` on any entry including `0`, and
       the advisory defines `User + 0` as "unknown" — so a filer who uses the new form to state
       "I have none" is told btctax has nothing on file, which is now false.
FAILURE: Filer with a Schedule C opens the new "Carryforwards from last year" section and enters
       `0` for both lines (the honest answer for most). `set` at :1043 writes
       `qbi_carryforward_in_provenance = User`; `unknown(v, p) = v == ZERO && p == User` at :561-563
       is still true, so `QbiCarryforwardNotStated` fires, telling them *"btctax has no prior-year
       loss carryforward on file for it."* `CarryProvenance` has only two values and cannot express
       "the filer stated zero", so the distinction the commit claims — *"a zero that is merely the
       struct default is an UNKNOWN"* — is not actually made: a typed zero and the struct default
       are the same two bytes. Direction is safe (over-advising), and the message's closing
       sentence covers the case, which is why this is Minor. Same shape as the answered-ness
       invariant: the surface asks a question it cannot record the answer to.
EVIDENCE: sections.rs:1041-1044 (`ri.qbi.qbi_carryforward_in = m;` then unconditional `= User`);
       advisories.rs:561-563. The commit message's premise — *"Each stamps `CarryProvenance::User`,
       so the year-to-year write-back refuses to overwrite a filer's own figure without `--force`"*
       — also does not hold for a stated zero: the write-back guards are `> Usd::ZERO && == User`
       (return_1040.rs:1704, 1714), so a filer's typed `0` is neither protected nor recorded.
```

ALSO CHECKED, SOUND:

**S1 (`fd9c15f`) — the gate itself is correct and does fail closed.** `questions::spouse_63f_boxes_count` requires `spouse.is_some()` and, on MFS, all three of `spouse_had_no_income == Some(true)`, `spouse_not_filing_a_return == Some(true)`, `can_be_claimed_as_dependent_spouse == Some(false)`; every other status returns `false`. Verified against i1040gi--2024.txt:1433-1438 and the footnote at :3706-3707, both of which state exactly those three conditions. **The MFJ path is byte-identical to before** (`Mfj => true` reproduces the old `.filter(|_| filing_status == Mfj)`). I ran `the_mfs_spouse_63f_boxes_need_all_three_conditions_and_fail_closed` — passes, and its seven forgo rows cover each condition unanswered and each denied. Filing-status changes are safe in both directions (MFS→Single/HoH ⇒ `_ => false`; MFS→MFJ ⇒ `true`, correct). A hand-edited TOML cannot grant a box it is not entitled to: it can only set the same three `Option<bool>`s the gate reads, and the adverse value of the third (`Some(true)`) refuses upstream as `DependentSpouseUnsupported`. Not a finding.

**S3 — the mirror defect (an answer COUNTED that was never asked) is NOT present in the deduction path.** Two independent guards: `spouse_63f_boxes_count` is the single liveness predicate for `SpouseDiedDuringYear`/`DodSpouse` (pinned by `the_mfs_spouse_death_gate_is_asked_exactly_when_the_boxes_count`), and beneath it `is_aged`'s `(None, None)` arm (return_1040.rs:77) returns `false`, so an unasked or skipped death question forgoes the aged box rather than granting it. `spouse.blind` is asked on MFS already (`BlindSpouse` is live on `spouse.is_some()`), so no box is counted from a fact never posed. The mirror defect does exist — but in the advisory layer, which is finding 1, not here.

**S5 (`64df404`) — direction and write-back are right.** Form 8995 line 3 is *"Qualified business net (loss) carryforward from the prior year"*, parenthesized, and line 4 is *"Combine lines 2 and 3"*; line 7 is the REIT/PTP equivalent feeding line 8. The two new fields' help text says "A POSITIVE amount", subtracted at line 4 / reducing line 8, sourced from *"line 16 / line 17 of last year's Form 8995"* — all four cross-references match f8995--2024.txt:34-58 exactly. `parse.rs` rejects a negative money entry (`ParseError::Negative`), so the form path cannot produce the sign inversion that would inflate line 4. The write-back **assigns** (`next_year.qbi.qbi_carryforward_in = ar.qbi_carryforward_out`), never accumulates, so no double-count across years, and `qbi_carryforward_out = (in − business_qbi.max(0)).max(0) ≥ 0`, so no negative. Line 16's "If greater than zero, enter -0-" is correctly implemented as that clamp. Sound.

**S2(a), the escape route, for the `None` case.** `DonationsHadRestrictions` is `live: |_ri| true`, is wired into `SKIPPABLE_QUESTIONS` (index 12 in `registries.rs`), into the input form's Skippables section, and `attribute.rs:92` anchors the refusal to the field — so `btctax income answer`, the route the message names, does reach it. The `Some(false)` path files. That half is sound; my findings are about the `Some(true)` path and the gate's scope.

**S2(b), the "Section B page escapes the gate" direction.** Cannot happen: `form_8283` decides the section with the identical expression the gate uses (`year_donation_deduction(state, year) > QUALIFIED_APPRAISAL_THRESHOLD`, forms.rs:397-404), so every printed Section B implies the gate fired. The `>` / `>=` boundary is right — §170(f)(11)(C) is "more than $5,000", so a gift *at* $5,000 is Section A. The failure is entirely in the other direction (finding 3).

**The 5a/5b/5c prompt text and the emitter's on-states.** The prompt enumerates all three limbs in the form's own words; I diffed it clause by clause against f8283--2025.txt:99-104 and it is faithful, including 5b's full "income / possession / vote / acquire / designate" list. `None` writes nothing (blank ≠ "No"), which is the correct answer to the `3b22ca1` class. The dumped on-states `"1"`/`"2"` are not Schedule C's `"Yes"`/`"No"`.

**S4, the rest of the advisory surface.** `QbiCarryforwardNotStated` and `BenefitCarryoversNotStated` name opposite directions (UNDERSTATED vs OVERSTATED) and are pinned not to converge; neither double-counts the other. The QBI advisory is gated on §199A *activity inputs* rather than the computed deduction, which is right — the income limitation can zero the deduction while the carryforward is still wrong. The §G-20b two-unconditional-member budget is settled fact #4 and I did not relitigate it.

**Not re-derived, per the brief:** §G-19a, §G-12, §G-22's known partiality, §G-20b, `.pii-patterns`, the golden md5 / five gates, and the standing "no oracle validates a carryforward INPUT" condition.

WHAT WOULD MAKE THIS REVIEW WRONG:

1. **I did not execute the end-to-end paths for findings 1, 3 or 4 — I read them.** For finding 1 I ran the packet test to confirm the boxes are claimed on a qualifying MFS return, and I read `advisories.rs:579-590` directly to confirm it has no gate on `spouse_63f_boxes_count`; I did **not** build a `ReturnInputs` with the three conditions set and call `advisories()` on it to watch both happen at once. If some caller filters the advisory list by filing status or by the emitted box state before display, the false advisory never reaches the filer and finding 1 collapses to a latent defect. I saw no such filter in `advisories()` or in the export path added by `c2be5a2`, but I did not trace every consumer.
2. **For findings 3 and 4 I did not construct a vault and run `btctax report`.** I traced `resolve_and_screen` → `screen_compute_dependent` → `Uncomputable`, and `choose_deduction`'s `standard.max(itemized)`, and `packet.rs:606-610`'s own statement that a standard-deduction year files no 8283. If some earlier guard refuses a restricted-donation year for an unrelated reason before this gate is reached, the false-refusal half of finding 4 is unreachable in practice. The `Some(true)`-under-$5,000 understatement in finding 3 does not depend on that, since it is an *absence* of any guard.
3. **Finding 5 rests on a negative search.** I grepped every crate for writes to `capital_loss_carryforward_in` and found only test fixtures, `tax_profile.rs` (the separate user-entered `tax-profile set` command) and the read sites. If a §1212(b) carryover is persisted into next year's `ReturnInputs` by some path whose name does not contain that identifier, the stamp is honest and the finding is wrong.
4. **I took the brief's settled fact #6 (all five gates green at HEAD) on trust** rather than running `make check`. Every finding above is invisible to a green suite by construction — each is an uncovered case, not a broken assertion — so a green suite is consistent with all of them, but I have not independently confirmed the branch is green.
5. **Severity judgement on finding 3.** I rated it Important rather than Critical because btctax never claimed to handle restricted donations and the number was equally wrong before this range — the commit strictly improves the situation. If the standard is "a signed return understates tax on facts the software collected", it is arguably Critical.

---

## Disposition (author, same day) — ALL SEVEN FOLDED

**Every Important was CONFIRMED by a failing test written FIRST, then fixed, then mutation-verified.**
The reviewer flagged findings 1, 3 and 4 as read rather than executed; none was taken on trust.

| finding | disposition |
|---|---|
| **I-1** — the four §63(f) advisories not migrated | **CONFIRMED** — the test returned `(counted: true, fired: true)`, i.e. boxes claimed AND "forgone" advisory firing, exactly as claimed. **FIXED**, but not as recommended: it needed **two** predicates. `spouse_63f_boxes_count` (record + status) decides the deduction; a new `spouse_63f_status_permits` (status only) drives the advisories, because an ABSENT MFJ spouse record is itself a forgone box. My first attempt used one predicate and reddened `mfj_with_no_spouse_record_still_advises_the_aged_box_p5_m2` — that regression is the split's seen-red-once. All four guards + the split mutation-verified. |
| **I-2** — gate too narrow ($5,000 ≠ §170) | **FIXED.** A declared restriction now refuses at ANY amount. Reg §1.170A-7 has no dollar floor; $5,000 only picks the 8283 *section*. |
| **I-3** — gate too wide (standard-deduction false block) | **FIXED**, and this moved the gate: it now lives in `screen_absolute`, keyed on `ar.deduction_is_itemized` — the real §63(e) election, which is not derivable from the inputs. `screen_absolute` gained `state`/`year`; the compiler enumerated every call site. |
| **I-4** — unfounded `Computed` stamp | **FIXED by deletion.** Confirmed `AbsoluteReturn` has no capital-loss carryover-out to write, because the §1211/§1212 worksheet is unmodeled in v1. The charitable sibling is honest and stays. |
| Minor — 5a/5b/5c on a Section A page | **FIXED.** Guarded on `section == B`; the form scopes them to "a contribution listed in Section B, Part I". |
| Minor — five stale doc comments | **FIXED**, all five rewritten to what the code now does. |
| Minor — `CarryProvenance` can't express a stated zero | **FILED as §G-23** — the answered-ness invariant in a new field; needs a third state, not a predicate tweak. Direction is safe. |

**Adjudicated against the instrument lens on S2.** That lens verified the gate agrees with the
Section A/B split (it did) and called the scope sound. This lens asked whether the split is the right
predicate *at all* — and it was not. The deeper question won: I-2 and I-3 are both real, and both are
now pinned by tests that red on the old behavior.

★ **Severity note.** The reviewer offered that I-3 might be Critical. Moot — it is fixed — but the
bar it named, *"a signed return understates tax on facts the software collected"*, is the right one.
