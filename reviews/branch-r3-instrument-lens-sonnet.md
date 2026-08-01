# r3 — INSTRUMENT LENS (Sonnet)

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `d0aad6f..HEAD` (9 commits).

**Brief:** [`reviews/BRIEF-r3.md`](./BRIEF-r3.md). This lens was additionally tasked with **re-running
the mutation kills the commit messages claim** — an unverified mutation claim is exactly the false PASS
the B1 rule exists to prevent — and with checking whether the `GAPS`-ratchet census is blind.

**Result: 0 Critical / 1 Important / 1 Minor.**

★★★ **The two lenses converged independently on the SAME Important**, from opposite directions: the tax
lens reasoned from i1040gi to the advisory layer, this lens executed it. Where the tax lens's own
"what would make this wrong" said *"I did not build a `ReturnInputs` with the three conditions set and
call `advisories()` on it to watch both happen at once"*, this lens did exactly that and reports
`spouse_aged=true, spouse_blind=true, count()=2` **with the "forgone" advisory firing alongside**. That
is the confirmation the other lens flagged as missing.

★ **Every mutation claim in the range held.** All six §G-21 kills and the §G-20 `!= Some(false)`
relaxation red by name on execution. The `GAPS` census was verified non-blind by planted defect.

**Note:** the two lenses DISAGREE on seam S2. This lens verified the gate agrees with the Section A/B
split (it does — same helper). The tax lens asked the deeper question: *is the section split the right
predicate at all?* — and says no. Adjudicated in the disposition.

**Reviewer output reproduced VERBATIM below.**

---

VERDICT: fix-before-merge: crates/btctax-core/src/tax/advisories.rs:579-590

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/advisories.rs:579-590 (Advisory::Mfs63fSpouseBoxesForgone, enum at line 100, message at line 230)
CLAIM: The new `Mfs63fSpouseBoxesForgone` advisory (added in `2dc8b07`, before `fd9c15f` made the boxes claimable) was never updated for the S1 fix, so it fires — falsely claiming the box is NOT claimed and the return is understated — on exactly the returns where `fd9c15f`'s new gate correctly GRANTED the box.
FAILURE: MFS return, spouse present, spouse DOB 1955-03-02 (65+) and blind, with `spouse_had_no_income = Some(true)`, `spouse_not_filing_a_return = Some(true)`, `can_be_claimed_as_dependent_spouse = Some(false)` — i.e. all three i1040gi conditions affirmatively met. `AgedBlindBoxes::for_return` correctly counts BOTH boxes (verified by direct execution: `spouse_aged=true, spouse_blind=true, count()=2`). The filed 1040/standard deduction is CORRECT. But `advisories()` independently recomputes "aged"/"blind" straight from DOB/blind (lines 581-585) without ever consulting `spouse_63f_boxes_count`, so it still pushes `Mfs63fSpouseBoxesForgone{boxes:2}` (verified by direct execution: fires alongside the correctly-granted boxes). Its message (line 230-238) reads: "...so {they} not claimed... your tax is OVERSTATED by {total} and the boxes are yours to check by hand." A filer who trusts this and manually adds the standard-deduction addition on top of the software-computed (already-correct, already-claiming) figure signs a return that UNDERSTATES tax by double-counting the same boxes — the exact worst-case outcome this review is scoped to find.
EVIDENCE: `advisories.rs:579-590`:
    if ri.filing_status == FilingStatus::Mfs {
        if let Some(sp) = ri.header.spouse.as_ref() {
            let aged = sp.date_of_birth.is_some_and(|d| crate::tax::return_1040::born_early_enough(d, year));
            let blind = sp.blind == Some(true);
            let boxes = usize::from(aged) + usize::from(blind);
            if boxes > 0 { out.push(Advisory::Mfs63fSpouseBoxesForgone { per_box, boxes }); }
        }
    }
  contrast `packet.rs:296-300` (`AgedBlindBoxes::for_return`), which is the actual gate and DOES filter through `spouse_63f_boxes_count`. The message's own premise — "btctax counts a spouse's aged/blind boxes only on a JOINT return" (line 233-234) — is also simply false at HEAD; `fd9c15f`'s own doc comment says the opposite ("the spouse's boxes count only on MFJ HERE" was the PRE-fd9c15f state). No test exercises this interaction: `the_mfs_spouse_63f_advisory_fires_only_when_a_box_was_actually_forgone` (advisories.rs:975) never sets the three §63(f) fields, so it only probes the pre-fd9c15f behavior and passes unchanged even with this bug present.
```

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/qbi.rs:162
CLAIM: The doc comment "★ It is asked of nobody yet: `FOLLOWUPS.md` §G-22" is stale at HEAD — `64df404` (this same range) added `SectionId::Carryforwards`/`FieldId::QbiCarryforwardIn` to the TUI input form specifically to ask for it, and `LIMITATIONS.md` and `FOLLOWUPS.md` both say so ("Both QBI loss carryforwards are now ASKED").
FAILURE: No tax-outcome effect — purely a misleading internal doc comment that could misdirect a future reader auditing whether line 3 is collected.
EVIDENCE: `qbi.rs:162` vs `crates/btctax-input-form/src/spec/sections.rs` (`CARRYFORWARD_FIELDS`, `FieldId::QbiCarryforwardIn`) and `LIMITATIONS.md`'s "Both QBI loss carryforwards are now ASKED, in the TUI's 'Carryforwards from last year' section."
```

ALSO CHECKED, SOUND:

- **S1 (fail-closed gate).** `spouse_63f_boxes_count` requires all three i1040gi conditions in the claiming direction (`Some(true)`, `Some(true)`, `Some(false)`); every one of the 7 pinned forgo cases in `the_mfs_spouse_63f_boxes_need_all_three_conditions_and_fail_closed` (packet.rs:787-838) holds by direct execution, and relaxing the first condition from `== Some(true)` to `!= Some(false)` reds it by name exactly as the commit message claims. The MFJ path is logically unchanged (`spouse_63f_boxes_count` reduces to `spouse.is_some()` on MFJ, identical to the pre-range predicate) — verified by inspection, not just by test. No stale-answer/filing-status-change leak: the predicate is a pure function of current `ReturnInputs`, recomputed every call, no caching.
- **S2 (donation-restriction gate, escapability + $5,000 scope).** Mutation-confirmed by direct execution: dropping the `None` arm, dropping the `Some(true)` arm, dropping the threshold comparison, and widening `>` to `>=` (reds on exactly $5,000) all red the named tests exactly as claimed. The threshold and the emitter both call the SAME `year_donation_deduction` helper that assigns `Form8283Section::B` in `forms.rs::form_8283` — structurally impossible for the refusal gate and the Section A/B split to disagree. `DonationsHadRestrictions` is `live: |_ri| true` unconditionally, so the unanswered-refusal escape route (`btctax income answer`) is always reachable; the emitter-side mutation claims ("write No on `None`", "delete the write") also both confirmed red by direct execution. `Some(true)` correctly has no software escape (btctax cannot compute a restricted gift), which is the correct — not a bug.
- **S3 (shared predicate, no mirror defect).** `spouse_63f_boxes_count` is the single definition consumed by both `AgedBlindBoxes::for_return` and the liveness of `SpouseDiedDuringYear`/`DodSpouse`; `the_mfs_spouse_death_gate_is_asked_exactly_when_the_boxes_count` directly asserts `(gate.live)(&ri) == spouse_63f_boxes_count(&ri)` for both claimable states, which structurally forecloses "counted but never asked."
- **field_census GAPS ratchet.** `census_accounts_for_every_field` enumerates from the ACTUAL AcroForm fields read out of the bundled PDF (`acroform_fqns` → `collect_fields`), not a range or hand-list. Confirmed by direct mutation: deleting the f8283 `[line5a]`/`[line5b]`/`[line5c]` map entries reproduces the "we forgot this line" failure and reds `census_accounts_for_every_field` by name, listing exactly the 6 dropped FQNs.
- **S5 (Form 8995 line 3).** The arithmetic (`line4 = max(line2-line3,0)`, `line16 = max(line3-line2,0)`) and the write-back guard (`apply_carryover_writeback`'s User-provenance `--force` gate) both pre-date this range at `d0aad6f` and are unchanged; this range only exposes `qbi_carryforward_in` as a collectible TUI field (`CARRYFORWARD_FIELDS`) stamping `CarryProvenance::User`, following the identical pattern already used for the REIT/PTP sibling. No negative or double-count path found.
- `make check` — 2533 passed, 0 failed, 12 skipped. Tree clean (`git status --short` shows only the pre-existing untracked `reviews/BRIEF-r3.md`) after every mutation was restored via `cp` (never `git checkout`).

WHAT WOULD MAKE THIS REVIEW WRONG:

- I did not execute a fresh probe test in the input-form/TUI crates for the S4 finding (only in `btctax-core`'s own advisory function) — if some other layer suppresses or re-filters `Mfs63fSpouseBoxesForgone` before it reaches the CLI's printed advisory list, the practical exposure would be smaller than described. I did not find such a filter by inspection, but I did not execute the full CLI `report` path with this exact fixture.
- I did not exhaustively re-derive every one of the 15 forms' `[census]` sections against their PDFs — only the f8283 mutation was executed. I'm relying on `census_accounts_for_every_field`'s own structure (verified to read the real PDF) to generalize to the other 14 forms in `CENSUSED`.
- I did not construct a full end-to-end fixture that both claims the MFS spouse's boxes (fd9c15f path) AND produces a filed PDF, to visually confirm the printed 1040 checkbox state agrees with `AgedBlindBoxes::for_return` — I relied on the existing `AgedBlindBoxes::for_return`/`return_1040.rs` wiring (pre-existing, settled) rather than re-verifying the PDF fill for this specific combination.

---

## Disposition (author, same day) — BOTH FOLDED

| finding | disposition |
|---|---|
| **Important** — `Mfs63fSpouseBoxesForgone` fires on returns that CLAIM the boxes | **FIXED.** Same defect the tax lens found; this lens's executed confirmation is what made it certain. See that review's disposition — the fix needed a predicate SPLIT rather than the single-predicate gate both lenses assumed. |
| Minor — stale `qbi.rs:162` "asked of nobody yet" | **FIXED.** It IS collected; §G-22's remaining gap is the other carryforward families. |

★★★ **This lens's mutation re-runs are the reason to keep it.** Every kill claimed in the range held
under independent execution — the six §G-21 kills and §G-20's seven forgo cases including
`!= Some(false)` redding by name — and the `GAPS` census was verified non-blind by planted defect
(deleting the three 8283 map entries reds it, listing exactly the six dropped FQNs). A review that
only reads cannot produce that.

★ **Where it was wrong, and it matters:** it called S2's scope sound because the refusal gate and the
Section A/B split use the same helper. True, and beside the point — the tax lens asked whether that
split is the right predicate at all, and it was not (I-2/I-3). *Agreeing with a neighbour is not the
same as being right.*
