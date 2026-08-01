# PRE-MERGE — whole branch, adversarially verified (workflow)

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `main..HEAD` (34 commits).

**Brief:** [`reviews/BRIEF-merge.md`](./BRIEF-merge.md) — scoped by harness rule **B3**: the last pass
before an irreversible action is scoped to the WHOLE branch and pointed at INTERACTION, because this
branch has now twice proved that a stack of range-scoped reviews does not add up to a branch review.

**Shape:** 5 independent lenses over `main..HEAD` (4 Opus, 1 Sonnet), then **2 independent skeptics
per blocking finding, each instructed to REFUTE it**; a finding died only if both killed it.
16 agents, ~2.37M tokens.

**Result: `fix-before-merge` — None surviving Important, None killed by refutation.**

★★★ **Ten skeptic votes, ZERO refutations.** Every surviving finding was independently re-executed by
at least one skeptic — none is an argument-only claim. After dedup the five reports are **three
distinct defects**; three separate lenses independently found the same stale advisory string.

★★★ **Finding 2 is a REGRESSION introduced by the r3 fold itself** (`5ab1258`, committed one hour
earlier). The pre-r3 gate refused that case; the re-key opened it. This is precisely what the
`r3-fixes` lens was created to look for, and the reason a fix must be reviewed like any other change.

★★ **Finding 3 punctures the r3 commit's own stated assurance** — *"the compiler enumerated every call
site."* The compiler enumerates a **signature change**. It does not enumerate a call that is
**deleted**: all three fail-closed screens on the filed-PDF export path can be removed and the entire
2536-test suite stays green, and a skeptic drove `export_irs_pdf` end-to-end to watch five real PDFs
land on disk with the mutation applied.

★ **Process note, recorded against myself:** 16 agents ran concurrently in ONE shared worktree and
several mutated files to verify kills. Three lenses independently report finding mutations they did
not plant. The final tree was verified clean (`git status` shows only the untracked brief, `git diff
HEAD` empty, `make check` 2536/2536, golden md5 `c4e1853` unmoved) — but **intermediate evidence
obtained inside that window is softer than it reads**, and the report says so. This is the
"no parallel branch tasks in a shared worktree" rule; verification agents needed `isolation: worktree`.

**Workflow output reproduced VERBATIM below.**

---

{
  "summary": "Whole-branch pre-merge review of feat/no-pen-deferrals, adversarially verified",
  "agentCount": 16,
  "logs": [
    "Refutation: 5 findings survived, 0 killed by 2/2 skeptics"
  ],
  "result": {
    "verdict": "VERDICT: fix-before-merge: 3 surviving Important — (1) the stale `Mfs63fSpouseBoxesForgone` advisory text, (2) a restricted-donation carryover laundered into next year's inputs stamped `Computed`, (3) the filed-PDF export path's three fail-closed screens held by no test.

Ten skeptic votes were cast across the five findings. **Zero refutations.** Every surviving finding was independently re-executed by at least one skeptic — none is an argument-only claim. After deduplication the five findings are **three distinct defects**: three separate lenses (`refusal-surface`, `testimony`, `one-filer-walk`) independently found the same stale advisory string, and six skeptics collectively failed to kill it.

---

## SURVIVING BLOCKING FINDINGS

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/advisories.rs:230-244 (message) and :88-100 (variant doc)
SKEPTICS: 3 lenses found it independently; 6 skeptics tried to kill it, 0 refuted.
          4 of the 6 built the fixture and observed the message print verbatim.

CLAIM: `Advisory::Mfs63fSpouseBoxesForgone` still asserts that btctax \"counts a spouse's
aged/blind boxes only on a JOINT return\" and that i1040gi's three MFS conditions are \"three
things btctax does not ask and cannot verify\" — both falsified by `fd9c15f`, which made the
boxes claimable and made those three answers the gate. r3 (`5ab1258`) fixed the firing
CONDITION and never touched the sentence that states the contradiction.

FAILURE: Two reachable shapes, both executed at HEAD.
 (a) The DEFAULT state. MFS, spouse DOB 1955-03-02 + blind, the three conditions unanswered —
     normal, because coverage.rs:328-329 deliberately exempts `spouse_had_no_income` and
     `spouse_not_filing_a_return` from the input form. The advisory fires and tells the filer
     \"your tax is OVERSTATED by $3,100 and the boxes are yours to check by hand.\" A filer who
     does that signs a 1040 whose §63(f) checkbox count claims $3,100 of additions line 12 does
     not contain — the exact box-count/amount drift `AgedBlindBoxes::count()` is the single
     source for. The action that actually works — set the two fields via `income import`,
     whereupon `AgedBlindBoxes::for_return` claims the boxes AND `standard_deduction` raises
     line 12 together — is never mentioned and is affirmatively contradicted.
 (b) WORSE, and the reason this is not cosmetic: the identical message fires when the filer has
     ANSWERED a condition adversely. With `spouse_had_no_income = Some(false)` — the filer has
     told btctax the spouse HAD income, so §151(b)/i1040gi disqualify the boxes and btctax
     correctly computes deduction $14,600, tax $18,339 — the advisory still prints \"three things
     btctax does not ask and cannot verify\" and offers a $3,100 hand-claim on a return signed
     under §6065. btctax DID ask, DID verify, and the filer's own recorded answer is why the
     boxes were declined. `spouse_not_filing_a_return = Some(false)` is the ORDINARY MFS case
     (the spouse files their own return) and has the same shape.

EVIDENCE: advisories.rs:230-244 (read at HEAD, verbatim): \"...but btctax counts a spouse's
aged/blind boxes only on a JOINT return, so {they} not claimed. The instructions allow them on
married-filing-separately \\\"if your spouse had no income, isn't filing a return, and can't be
claimed as a dependent on another person's return\\\" — three things btctax does not ask and
cannot verify, so it does not claim them for you.\" Contradicted by questions.rs:140-149
`spouse_63f_status_permits` (Mfs => all three conditions read) and by the advisory's own guard
at :588 `!spouse_63f_boxes_count(ri)`. \"does not ask\" is false on its face for the third
condition: questions.rs:269-277 makes `can_be_claimed_as_dependent_spouse` a MANDATORY
FORM_QUESTIONS entry (`RefuseReason::DependentSpouseStatusUnanswered`), and return_refuse.rs:724
refuses `Some(true)` — so every computable MFS-with-spouse return has it settled at `Some(false)`.
`git show --stat fd9c15f` does not list advisories.rs. `git show 5ab1258` touches the guard and
three SIBLING advisories' doc comments and never this variant's message or its doc at :88-100
(\"which btctax forgoes because it counts spouse boxes on MFJ only\", \"three conditions btctax
captures none of\"). FOLLOWUPS.md §G-20 rests its ✅ CLOSED on this exact message: \"already names
the cost, so nobody is left guessing.\" Authority transcribed correctly
(i1040gi--2024.txt:1433-1438, :3706-3707) — which is precisely what makes the prose false.
```

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_1040.rs:1602 (the re-keyed gate)
    → crates/btctax-cli/src/cmd/tax.rs:558-581 (write_back_carryover)
SKEPTICS: 2 tried to kill it, 0 refuted. BOTH built the fixture and executed it; one used an
          isolated worktree to avoid the contaminated shared tree.

CLAIM: r3's re-key of the §G-21 gate to `ar.deduction_is_itemized` means a year in which the
filer DECLARED a donation restriction no longer refuses when the return takes the standard
deduction — but `apply_170b` still runs and still produces a full-FMV `charitable_carryover_out`,
which `--write-carryover` persists into next year's `charitable_carryover_in` stamped
`CarryProvenance::Computed`, past every one of the write-back's own anti-laundering gates.

FAILURE: EXECUTED by both skeptics, identical results. TY2024, Single, AGI $30,000, LT BTC gift
FMV $50,000 to a public charity, `donations_had_restrictions = Some(true)` (truthful — retained
right). §170(b)'s 30% ceiling allows $9,000, which loses to the $14,600 standard deduction ⇒
`deduction_is_itemized == false` ⇒ `screen_absolute` skips the whole §G-21 block ⇒ the year
computes and exports CLEAN. Correct, so far: no §170 deduction is claimed. But
`charitable_carryover_out` = $41,000 at FULL FMV — the number btctax has just been told is too
large. `--write-carryover` then passes the pseudo-taint gate, the NotComputable-delta gate,
`screen_absolute` (returns None) and the user-provenance gate, and writes
`next_year.charitable_carryover_in = $41,000` with provenance `Computed` — a claim of knowledge
over a figure btctax knows is inflated. Next year that carryover deducts with NO gate anywhere:
`donations_had_restrictions` is `Durability::PerYear` so it is `None` on that row, and the §G-21
gate reads `year_donation_deduction(state, Y+1)` = $0 because the donation was in Y. The wrong
input is persisted TODAY and becomes a wrong figure on a signed return the moment TY2025 tables
land. Note the pre-r3 gate (`year_donation_deduction > $5,000` in `screen_compute_dependent`)
DID refuse this year — the branch closed the hole and then reopened it.

EVIDENCE: return_1040.rs:1601-1602 (read at HEAD): `let donated = ...year_donation_deduction(
state, year); if ar.deduction_is_itemized && donated > Usd::ZERO {`. The carryover is computed
regardless of the election — return_1040.rs:1263-1275, verbatim: \"apply_170b runs
UNCONDITIONALLY (even in a std-deduction year) so the carryover ages (Reg. §1.170A-10(a)(2), G8)
and carryover_out is the REAL filed carryover\". `write_back_carryover` (tax.rs:525-581, read at
HEAD) gates on pseudo-taint, a NotComputable delta, `screen_absolute` and user provenance — and
nothing else; the restriction declaration is not among them. The principle is already written
two gates above, at tax.rs:528-531: \"NEVER persist a carryover derived from a pseudo-tainted OR
hard-blocked ledger into year+1's stored inputs. Next year `pseudo_active()` is false and the
UX-P4-1 banner correctly does not fire — so an unflagged, deliberately-fictional (or
unanswerable) figure would ride into a real input. Fail-closed\" — which describes this case
exactly. And r3's OWN I-4 states the rule it breaks (return_1040.rs:1758-1762): \"A provenance
stamp is a claim of knowledge; do not make one the code cannot support.\" Authority for the
amount being wrong is the branch's own citation: Reg §1.170A-7 / §170(f)(3)(A), quoted at
return_1040.rs:1587-1590 and in the refusal's own detail string.
```

```
SEVERITY: Important
WHERE: crates/btctax-cli/src/cmd/admin.rs:772-792 (export_full_return)
SKEPTICS: 2 tried to kill it, 0 refuted. Both re-ran the mutation independently (2536/2536 and
          2537/2537 green). One went further and drove `cmd::admin::export_irs_pdf` end-to-end.
NOTE: filed Important by `one-filer-walk`, Minor by `r3-fixes`. Both skeptics graded it Important.

CLAIM: The three fail-closed refuse screens on the FILED-PDF path are held by no test — all
three can be deleted and the entire suite stays green.

FAILURE: MUTATION-VERIFIED four separate times, not argued. Wrapping `screen_inputs`,
`screen_compute_dependent` and `screen_absolute` in `if false { }` leaves `cargo nextest run
--workspace --no-fail-fast` fully green. One skeptic then built the actual failing input (Single,
W-2 pushing taxable income above the TY2024 §199A(e)(2) threshold, 1099-DIV box 5 > 0) and called
the public `export_irs_pdf` entry point: with the tree CLEAN it correctly refuses (\"the 2024
return is not computable [QbiAboveThreshold]: ... — no forms were written\"); with the mutation
applied it returns `Ok` and writes `00_f1040.pdf`, `55_f8995.pdf` (the SIMPLIFIED 8995, exactly
the wrong form once the 8995-A phase-in applies), `02_f1040s2.pdf`, `71_f8959.pdf`,
`72_f8960.pdf` and a manifest to disk. No downstream gate intervened. HEAD is CORRECT today;
what is missing is the GUARANTEE. This matters NOW specifically because `5ab1258` is the commit
that MOVED the §G-21 refusal into `screen_absolute` and changed its signature, touching this
exact call site, and the fold's stated assurance was \"`screen_absolute` gained `state`/`year`;
the compiler enumerated every call site.\" The compiler enumerates a signature change. It does
not enumerate a call that is deleted or reordered, and no test does either.

EVIDENCE: admin.rs:772-792 (read at HEAD), whose own comment states the guarantee it cannot
keep: `// Fail-closed screens, in the same order the report runs them. A refusal writes NO
bytes.` `grep -rn \"no forms were written\" crates/btctax-cli/tests/` returns ZERO hits (verified).
The REPORT path IS covered — report_exit_code.rs:201 asserts \"the ABSOLUTE return must be refused
by screen_absolute\" — but that exercises `cmd/tax.rs`'s independent call site, not this one. The
path that emits the SIGNED ARTIFACT is the uncovered one. Authority — CLAUDE.md, \"Tests for
conformance, reviews for judgment\": \"**A guarantee without a test that reds when it is removed
does not exist.** Mutation-verify.\" And HARNESS.md B1: \"The reviewable question is one sentence
with a factual answer — *'which test reds when this checker is removed?'*\" The factual answer
here is: none.
```

---

## MINORS AND NITS (deduplicated across all five lenses — recorded, not gating)

| # | Item | Found by |
|---|---|---|
| M1 | **13 sites still name `screen_compute_dependent` as the §G-21 refusal's home** — `5ab1258` moved it to `screen_absolute`. Verified by grep at HEAD: questions.rs:606,941; printed.rs:141; classifier.rs:149,**155** (a recorded census PROVENANCE REASON string, not a comment — the load-bearing one); packet.rs:618; return_refuse.rs:43; attribute.rs:24; registries.rs:305; form8283.rs:516; f8283.map.toml:57; full_return_forms.rs:2115; **FOLLOWUPS.md:1445 (§G-21's own \"As built\" table)**. Two of them (printed.rs:141, packet.rs:618) assert an unreachability INVARIANT on the stale mechanism — the invariant still holds, but through a different chain, so the recorded reason no longer supports the claim it is attached to. `5ab1258` edited §G-20's and §G-23's FOLLOWUPS rows and left §G-21's. | all 5 |
| M2 | **`full_return_forms.rs:2115-2116` cites a test name `5ab1258` itself deleted** (`a_section_b_year_refuses_until_the_restriction_questions_are_answered_no`). | testimony, consistency |
| M3 | **Stale \"MFJ only\" §63(f) doc cluster** — return_1040.rs:138-139 (byte-identical to `main`, sitting on `standard_deduction`, the function that computes the figure), advisories.rs:140, advisories.rs:1718, classifier.rs:270 (census reason string). Executed contrast: qualifying MFS → deduction $17,700 / tax $17,595; unanswered → $14,600 / $18,339. | refusal-surface, one-filer-walk |
| M4 | **`export_full_return`'s `donated > Usd::ZERO` term survives deletion** — mutation-verified green across the whole suite. Removing it refuses every itemizing filer who answered `Some(true)` with no crypto donation in the ledger; the exits are perjury or deleting a truthful answer — r3's own I-3 shape, one clause over. Sibling terms ARE held. | r3-fixes |
| M5 | **A declared restriction on a NON-crypto noncash gift never refuses** — `donated` reads only the crypto ledger, so Schedule A line 12 still prints the gift at full claimed value. Bounded at ~$500 of deduction (~$185 tax) by the `NonCryptoNoncashGift` row; btctax transcribes the filer's own stated amount here, which is the principled reason the gate keys on the ledger. Strictly narrower pre-r3. | r3-fixes |
| M6 | **8283 5a/5b/5c print three \"No\" marks where the form routes to BLANK** — i8283--2024.txt:1211: \"Complete lines 5a-5c **only if** you attached restrictions…\". The branch treats the identical \"if X\" construct the OPPOSITE way twice elsewhere (Schedule D 17-22, Schedule B 7b/FBAR). No figure moves and the filer did give the answer. | testimony |
| M7 | **The §G-21 prompt drops one of line 5b's YES-conditions** — questions.rs:930-931 stops at \"or to ACQUIRE it\"; the form adds \"**or to designate the person having such income, possession, or right to acquire**\". A filer retaining only a designation power answers \"No\" truthfully, btctax prints \"No\", and deducts at full FMV. Fails OPEN, contradicting the comment three lines above (\"every omission fails closed\"). Narrow; rests on a disputable legal reading. | testimony |
| M8 | **classifier.rs:392-393 binds two `Option<bool>` leaves with bare `_`** (`payments_requiring_1099`, `will_file_required_1099`) — the module's own r2 M-6 rule forbids exactly that on `Option<bool>`. The census silently under-reports two answered-ness decisions and nothing reds. The four other class-(B) leaves the branch added were all recorded via `c.exempt(..)`. | refusal-surface |
| M9 | **`fbar_filing_required`'s doc still calls it \"a class-(A) DECLARATION\"** — `cbe651d` reversed that two commits later; every other site now says class (B). The doc instructs a maintainer to restore the refusal the refusal review deliberately removed. Mitigated by `the_fbar_sub_question_does_not_refuse_a_return`. | refusal-surface, testimony |
| M10 | **`LIMITATIONS.md:269` never updated for the MFS widening** — still \"you (or, on a joint return, your spouse)\"; the new MFS entitlement is undocumented. No computational effect (the interview asks regardless). | consistency |
| M11 | **Crypto-slice Schedule D is partial and unstamped** — `3e16f85` established the \"the disclosure has to be ON the page, because the page is what travels\" doctrine and applied it only to the crypto-slice 1040; the equally-partial Schedule D (now printing line 17 = \"Yes\") gets no watermark. Pre-existing in substance. | testimony |
| N1 | **`d6ff290` inserted `donations_had_restrictions` between `dual_status_alien`'s doc comment and its declaration** — the dual-status prose is now rustdoc for the donations field, and `dual_status_alien` (a refusing class-(A) declaration) carries no doc at all. | r3-fixes, refusal-surface, testimony |
| N2 | **Two new comments misdescribe their code** — `kitchen_sink_household()`'s §G-21 comment claims a Section B files (its $5,000 is a `Cash60` Schedule A gift; `removals` is empty, so `year_donation_deduction` is $0 and the `Some(false)` is inert); and form8995.rs justifies blanking line 3 with \"the only line that is neither derived nor computed — it is testimony\", when line 7 is equally filer-supplied and still prints `0`. | one-filer-walk |

---

## COVERAGE — verified sound; do not redo

**r3's five unreviewed fixes, all cleared, most by executed mutation:**
- **The `screen_absolute` signature change.** All production `assemble_absolute` sites enumerated (`cmd/tax.rs:335`, `cmd/tax.rs:560`, `cmd/admin.rs:789`, plus `oracle-harness:752`); every one pairs `assemble_absolute` + `screen_absolute` on the SAME `(ri, state, year)` tuple, in order. No mismatched year/state.
- **The §G-21 re-key**, both directions mutation-killed: dropping `ar.deduction_is_itemized` reds `a_standard_deduction_year_is_never_blocked_by_the_restriction_questions` (r3's I-3 case); narrowing the `Some(true)` arm back to $5,000 reds `a_declared_restriction_refuses_at_any_amount_not_just_over_5000` (I-2's case). The unanswered arm's `> QUALIFIED_APPRAISAL_THRESHOLD` exactly matches `forms.rs:400`'s Section A/B split (both the year aggregate, both strict `>`), so the refusal's \"this year files a Section B\" is true whenever it fires.
- **The predicate split**, all 7 consumers checked and mutation-killed: collapsing `spouse_63f_status_permits` back to `spouse_63f_boxes_count` reds `mfj_with_no_spouse_record_still_advises_the_aged_box_p5_m2` AND `blind_advisory_counts_taxpayer_and_mfj_spouse_and_fires_on_none`. The deduction needs a record; the forgone-box advisories must fire when there is none. **No input claims a box and simultaneously advises it forgone** — r3's I-1 GATE is genuinely fixed (only the string is stale).
- **The deleted `CarryProvenance::Computed` stamp** is correct: `apply_carryover_writeback` never assigns `capital_loss_carryforward_in`, so the stamp was a claim about a value it does not produce. Only readers are `BenefitCarryoversNotStated` (which it was falsely silencing) and a `NoTaxDirection` census exempt. Post-fix the advisory fires — over-advises, safe.
- **The `section == Form8283Section::B` guard** mutation-killed in both directions (flipping to `::A` reds `the_8283_restriction_boxes_are_written_only_when_the_filer_answered_no`), and backed verbatim by f8283--2025.txt:83.

**§63(f) / MFS widening, executed across 16+ variants:** fail-closed in seven directions — all three conditions met + death gates answered → $17,700/$17,595; **any one unanswered or adverse → $14,600/$18,339. No combination grants a box `main` withheld.** §63(c)(6) coupling intact (`mfs_spouse_itemizes == Some(true)` zeroes the whole standard deduction including §63(f) additions). The class-(A)→(B) death-gate downgrade is direction-safe: `is_aged`'s `(None,None)` arm returns `false`, so an unanswered gate FORGOES the box; advisory and deduction share `born_early_enough` so they cannot use different cutoffs.

**Answered-ness / no fabricated testimony:** every branch `Option` conversion traced constructor→writer. Schedule B 7a/8 carried verbatim with no `unwrap_or`; schedule_b.rs writes via `filter_map(|(pair, answer)| answer.map(..))`; Schedule C I/J via `if let (Some(pair), Some(answer))`; 8283 5a/5b/5c only on `Some(false)`; 8995 line 3 blanked at zero. Orphan audit done on CHILD gates, not just parents (FBAR gated on `foreign_accounts == Some(true)`; line J on line I; the one new gate-narrowing — `SpouseDiedDuringYear` moving to `spouse_63f_boxes_count` — can strand a stored answer but every consumer traced, no orphan reaches a page).

**Census honesty:** `field_census.rs` enumerates from the REAL AcroForm (`collect_fields(&doc)` over the shipped PDF), asserts three directions, reds on a dropped field. GAPS 16→0 reconciles exactly (2+4+2+6+1+1); 15 closed by mapping, the 16th (Schedule SE line A) reclassified gap→unmodeled with LIMITATIONS.md:241-244 backing it.

**On-states:** all four new checkbox families dumped not analogized, KATs assert the literal string against the real PDF; `pdf.rs:433-452` now rejects any on-state the widget's own `/AP /N` does not declare, with a planted-defect kill (B1 satisfied honestly).

**Also cleared:** Form 8995 lines 3/4/16 line-by-line against f8995--2024.txt:37,38,55 — line 3 can only reduce, never create, a deduction (executed: $20k carryforward left a $5,423 deduction unchanged because the §199A(a)(2)(B) limit bound); `qbi_carryforward_out` IS written back, no leak. `schedule_d_line17` branch-for-branch identical to the routing it replaced. The SSN relaxation keeps its boundary (`ReturnHeader::build` rejects malformed SSNs for taxpayer, spouse and dependents). The §G-18 revert is complete. `DonationRestrictionsUnresolved`/unanswered is escapable (`live: |_ri| true`, `income answer` walks both registries, the gate is not in `screen_inputs`). `year_donation_deduction` and `crypto_charitable_gifts` are the same quantity by construction. `Some(true)` still cannot reach the 8283 printer. All 14 advisories checked for the r3 double-count shape — each forfeit predicate is the complement of its claim. ~48 new tests enumerated; none trivially true, zero-iteration, or asserting less than its name.

---

## RESIDUAL RISK

**★★★ THE SHARED BLIND SPOT — say it loudly: four of the five lenses worked entirely at the library seam. Nobody on the review side built a vault, ran `btctax export-irs-pdf`, and read the ink on the emitted PDF.** `refusal-surface` executed nothing at all. `one-filer-walk`, `testimony` and `consistency` drove `screen_*` / `assemble_*` / `advisories_for` directly. `r3-fixes` ran mutations but explicitly states it \"did NOT build a §G-21-refusing vault and run `btctax export-irs-pdf` against it to watch zero bytes land.\" The only end-to-end CLI execution in this entire report came from a *skeptic* verifying finding 3, and it found that the mutated export path really does write five real PDFs to disk. Consequence: **every \"the invariant still holds / `Some(true)` is unreachable at the writer\" conclusion in the COVERAGE section above is an inference from control flow, not an observation of the artifact.** If any surface reaches `btctax_forms::fill_*` through a path that `assemble_absolute` / `assemble_printed_return` / `assemble_printed_forms` / `fill_full_return` / `export_irs_pdf_from_session` / `write_form_csvs` does not name, no lens covered it — and finding 3 proves the suite could not have told them.

**★★ SECOND SHARED BLIND SPOT — the shared tree was contaminated during the review window, and three lenses say so independently.** `refusal-surface` found a planted mutation live in `return_1040.rs` plus an untracked `tests/zz_review_walk.rs` and chose to run nothing rather than compile someone else's mutation. `one-filer-walk` found `admin.rs` **already mutated** when it checked `git status` mid-review (it was clean at session start) and ran with an edit it did not plant. `consistency` reports unexplained transient divergence in `form8283.rs` and `advisories.rs` — including a `Form8283Section::A`-for-`B` swap it did not make — a `cargo test` failure on tests it had just seen pass, and evidence of its own context being compacted away. **Any single-point-in-time green obtained inside that window should be discounted.** The final states were each verified clean and I re-verified at HEAD just now (`git status` shows only the untracked brief; `git diff HEAD` empty), and `consistency`'s thrice-repeated final `make check` was 2536/2536 — but the intermediate evidence is softer than it reads.

**Nobody independently re-ran the five gates except `consistency`** (`make check` only — 2536/2536, matching settled fact 8). `testimony` took settled fact 8 wholly on trust and says so. `fmt`, `msrv`, `check-isolation` and `pii-scan` were not re-run by any lens; per the fast-validation-gate memory, green `make check` ≠ green CI.

**Not exercised by anyone:** the TUI, the interactive `income form`, and interactive `income answer` (escapability of the new refusal was established structurally, from the registry, not by driving a session). The persistence layer's `ReturnInputs` reconstruction path. The TY2025 / Schedule 1-A Part V surface. The `defensive` subcommand.

**Not verified by execution, per-finding:** finding 2's downstream half — that the persisted TY2025 row is the row a future TY2025 build would read — is an inference from `write_back_carryover`'s contract; both skeptics observed the *write*, neither observed the *read* (it is not observable in v1). Finding 1's harm requires a filer to ACT on the advisory; the message's remedy is conditional (\"If all three are true of your spouse\"), so a careful reader is partly protected by the conditional even though the two factual sentences preceding it are false — `one-filer-walk` flags this as the judgement that could drop it to Minor, and graded it Important because it is r3's own I-1 mechanism inverted and because FOLLOWUPS.md rests §G-20's closure on this message being accurate. I agree with Important.

**Verified only through the KATs, not independently:** the checkbox on-state STRINGS (`xtask dump-fields` was never re-run against the shipped PDFs by any lens). A KAT and a map wrong in the same direction would survive — mitigated, not eliminated, by the new `/AP /N` fail-closed check at fill time.

**One finding rests on a legal reading, not a fact:** M7's claim that retaining a power to *designate* the recipient of income is not \"giving anyone a right to its income.\" A tax practitioner could reasonably dispute it; if disputed, M7 collapses to nothing.

**Interaction combinations nobody built:** the MFS §63(f) widening crossed with a real Form 6251/AMT trigger, and with a §1211/§1212 capital-loss-carryforward TI≤0 case. The three `screen_absolute` rows below the new §G-21 row were not re-verified for order-dependent interaction beyond confirming that ordering changes WHICH message shows first, never WHETHER a return refuses.",
    "survivors": 5,
    "killed": 0
  },
  "workflowProgress": [
    {
      "type": "workflow_phase",
      "index": 1,
      "title": "Find"
    },
    {
      "type": "workflow_phase",
      "index": 2,
      "title": "Refute"
    },
    {
      "type": "workflow_phase",
      "index": 3,
      "title": "Verdict"
    },
    {
      "type": "workflow_agent",
      "index": 1,
      "label": "find:r3-fixes",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "a176ecfd2a1c91880",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785534512162,
      "queuedAt": 1785534512145,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "merge",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-merge.md
It defines the one question, the range (`main..HEAD`, 34 commits), the four prior rounds and what
they covered, the eight settled facts you must NOT re-derive, and the exact output format.

ALSO read /scratch/code/bitcoin_tax/CLAUDE…",
      "lastProgressAt": 1785535477243,
      "tokens": 211890,
      "toolCalls": 88,
      "durationMs": 965081,
      "resultPreview": "{\"verdict\":\"merge\",\"findings\":[{\"severity\":\"Minor\",\"where\":\"crates/btctax-cli/src/cmd/admin.rs:779-792\",\"claim\":\"The full-return export's fail-closed screen chain — the gate the branch's new §170 refusal now sits behind — is not held by any test: both screen calls can be deleted and the whole 2536-test suite stays green.\",\"failure\":\"Not a wrong figure today (the calls are present and correct). The…"
    },
    {
      "type": "workflow_agent",
      "index": 2,
      "label": "find:refusal-surface",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "a77a91231d70595b6",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785534512164,
      "queuedAt": 1785534512145,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "fix-before-merge: 2 Important — (1) `Advisory::Mfs63fSpouse…",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-merge.md
It defines the one question, the range (`main..HEAD`, 34 commits), the four prior rounds and what
they covered, the eight settled facts you must NOT re-derive, and the exact output format.

ALSO read /scratch/code/bitcoin_tax/CLAUDE…",
      "lastProgressAt": 1785535599423,
      "tokens": 304728,
      "toolCalls": 95,
      "durationMs": 1087259,
      "resultPreview": "{\"verdict\":\"fix-before-merge: 2 Important — (1) `Advisory::Mfs63fSpouseBoxesForgone`'s user-facing MESSAGE still asserts behaviour `fd9c15f` falsified two commits later and names the wrong remedy (r3 fixed the CONDITION, not the text); (2) the r3 re-key of the §G-21 gate lets a full-FMV charitable carryover from a year the filer DECLARED restricted be persisted into next year's inputs, stamped `Co…"
    },
    {
      "type": "workflow_agent",
      "index": 3,
      "label": "find:testimony",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "a872142aea454136f",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785534512164,
      "queuedAt": 1785534512145,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "ANSWERED-NESS / ORPHANS (the assigned lens), all sound. (1)…",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-merge.md
It defines the one question, the range (`main..HEAD`, 34 commits), the four prior rounds and what
they covered, the eight settled facts you must NOT re-derive, and the exact output format.

ALSO read /scratch/code/bitcoin_tax/CLAUDE…",
      "lastProgressAt": 1785535383217,
      "tokens": 258428,
      "toolCalls": 74,
      "durationMs": 871052,
      "resultPreview": "{\"findings\":[{\"severity\":\"Important\",\"where\":\"crates/btctax-core/src/tax/advisories.rs:230-244 (message) and :88-99 (variant doc)\",\"claim\":\"`Advisory::Mfs63fSpouseBoxesForgone` still tells the filer that btctax \\\"does not ask and cannot verify\\\" i1040gi's three MFS conditions and \\\"counts a spouse's aged/blind boxes only on a JOINT return\\\" — both became FALSE three commits later in `fd9c15f`, whi…"
    },
    {
      "type": "workflow_agent",
      "index": 4,
      "label": "find:one-filer-walk",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "aab969fcc725b6c6c",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785534512164,
      "queuedAt": 1785534512145,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "fix-before-merge: 2 Important",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-merge.md
It defines the one question, the range (`main..HEAD`, 34 commits), the four prior rounds and what
they covered, the eight settled facts you must NOT re-derive, and the exact output format.

ALSO read /scratch/code/bitcoin_tax/CLAUDE…",
      "lastProgressAt": 1785535591019,
      "tokens": 275212,
      "toolCalls": 99,
      "durationMs": 1078855,
      "resultPreview": "{\"verdict\":\"fix-before-merge: 2 Important\",\"findings\":[{\"severity\":\"Important\",\"where\":\"crates/btctax-cli/src/cmd/admin.rs:772-792 (export_full_return)\",\"claim\":\"The three fail-closed refuse screens on the FILED-PDF path are held by no test — I deleted all three and all 2536 tests passed, twice.\",\"failure\":\"MUTATION-VERIFIED, not argued. (1) Replaced the `screen_absolute` call with `if false { let…"
    },
    {
      "type": "workflow_agent",
      "index": 5,
      "label": "find:consistency",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "af2ba4421f5a9b1b5",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785534512165,
      "queuedAt": 1785534512145,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "merge (0 Critical, 0 Important — 2 Minor recorded, not bloc…",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-merge.md
It defines the one question, the range (`main..HEAD`, 34 commits), the four prior rounds and what
they covered, the eight settled facts you must NOT re-derive, and the exact output format.

ALSO read /scratch/code/bitcoin_tax/CLAUDE…",
      "lastProgressAt": 1785535395222,
      "tokens": 261330,
      "toolCalls": 123,
      "durationMs": 883057,
      "resultPreview": "{\"verdict\":\"merge (0 Critical, 0 Important — 2 Minor recorded, not blocking)\",\"findings\":[{\"severity\":\"Minor\",\"where\":\"crates/btctax-forms/src/form8283.rs:516-517 (also crates/btctax-core/src/tax/questions.rs:606,941; packet.rs:618; printed.rs:141; classifier.rs:149,155; return_refuse.rs:43; crates/btctax-forms/tests/full_return_forms.rs:2114-2116; crates/btctax-input-form/src/attribute.rs:24; cra…"
    },
    {
      "type": "workflow_agent",
      "index": 6,
      "label": "refute:testimony:0:0",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a30a4b967830adc95",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535383234,
      "queuedAt": 1785535383220,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535542848,
      "tokens": 89475,
      "toolCalls": 31,
      "durationMs": 159614,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Tried to kill this on reachability and could not — the finding holds.\
\
Traced every upstream guard:\
1. Gate logic (advisories.rs:588, fixed in 5ab1258): `ri.filing_status == Mfs && !spouse_63f_boxes_count(ri)`. With the three i1040gi conditions unanswered (the default — coverage.rs EXEMPT_LEAVES confirms `header.spouse_had_no_income…"
    },
    {
      "type": "workflow_agent",
      "index": 7,
      "label": "refute:testimony:0:1",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a9535c2d7ab0321fb",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535383234,
      "queuedAt": 1785535383220,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535619841,
      "tokens": 92922,
      "toolCalls": 40,
      "durationMs": 236607,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Verified by direct execution and source inspection; the finding stands as filed.\
\
1) Git history: `git show --stat fd9c15f` confirms it touched FOLLOWUPS.md, classifier.rs, packet.rs, questions.rs, return_inputs.rs, coverage.rs, examples.md — NOT advisories.rs. `fd9c15f` added `questions::spouse_63f_status_permits` (checks exactly th…"
    },
    {
      "type": "workflow_agent",
      "index": 8,
      "label": "refute:one-filer-walk:0:0",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a9fd0a453415f8a31",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535591032,
      "queuedAt": 1785535591029,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785536026513,
      "tokens": 117010,
      "toolCalls": 62,
      "durationMs": 435481,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Reproduced the mutation and went further to test reachability directly, rather than trusting the mutation-count alone.\
\
1. Reran the exact mutation (all three screen calls in export_full_return wrapped in `if false {}`, admin.rs:779-798) and ran `cargo nextest run --workspace --no-fail-fast`: 2536 tests run, 2536 passed, 0 failed — m…"
    },
    {
      "type": "workflow_agent",
      "index": 9,
      "label": "refute:one-filer-walk:0:1",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a5bbac6f0a9a840fe",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535591032,
      "queuedAt": 1785535591029,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Every factual claim in the finding checks out, and I indepe…",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535826604,
      "tokens": 88953,
      "toolCalls": 30,
      "durationMs": 235572,
      "resultPreview": "{\"refuted\":false,\"reasoning\":\"Every factual claim in the finding checks out, and I independently re-ran the mutation.\
\
1. Code re-read (admin.rs:772-792): the three calls (`screen_inputs`, `screen_compute_dependent`, `screen_absolute`) and the exact comment \\\"// Fail-closed screens, in the same order the report runs them. A refusal writes NO bytes.\\\" and the refusal-format string \\\"the {tax_year…"
    },
    {
      "type": "workflow_agent",
      "index": 10,
      "label": "refute:one-filer-walk:1:0",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "adc494aa80beb8050",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535591033,
      "queuedAt": 1785535591030,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535895002,
      "tokens": 94038,
      "toolCalls": 46,
      "durationMs": 303969,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Built and executed the exact failing input at HEAD (test appended to advisories.rs's own test module, run under cargo test, then reverted — tree left clean). MFS, spouse DOB 1950-03-01 + blind, with ri.header.spouse_had_no_income = Some(false), spouse_not_filing_a_return = Some(true), can_be_claimed_as_dependent_spouse = Some(false). R…"
    },
    {
      "type": "workflow_agent",
      "index": 11,
      "label": "refute:one-filer-walk:1:1",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "ae32cb056a76eaa65",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535591033,
      "queuedAt": 1785535591030,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535700778,
      "tokens": 67488,
      "toolCalls": 20,
      "durationMs": 109745,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Reproduced live with cargo test (crates/btctax-core/src/tax/advisories.rs): built a ReturnInputs with filing_status=Mfs, spouse DOB 1950-03-01 + blind=Some(true), and spouse_had_no_income=Some(false) (an ANSWERED, disqualifying value — not a skip), spouse_not_filing_a_return=Some(true), can_be_claimed_as_dependent_spouse=Some(false). R…"
    },
    {
      "type": "workflow_agent",
      "index": 12,
      "label": "refute:refusal-surface:0:0",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "ac52a174219dbcdd0",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535599426,
      "queuedAt": 1785535599426,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535813231,
      "tokens": 95916,
      "toolCalls": 40,
      "durationMs": 213805,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"REACHABILITY CONFIRMED by execution, not just reasoning. Built the exact scenario in a scratch #[test] in crates/btctax-core/src/tax/advisories.rs, ran it, and observed the stale message fire verbatim:\
\
MFS, spouse DOB 1955-03-02 (65+), blind=Some(true), can_be_claimed_as_dependent_spouse=Some(false), spouse_had_no_income=None, spous…"
    },
    {
      "type": "workflow_agent",
      "index": 13,
      "label": "refute:refusal-surface:0:1",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a68552ce5b7c02078",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535599426,
      "queuedAt": 1785535599426,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535842039,
      "tokens": 85625,
      "toolCalls": 32,
      "durationMs": 242613,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"The finding holds. Traced the full control flow: `Mfs63fSpouseBoxesForgone` fires at advisories.rs:588 exactly when `!spouse_63f_boxes_count(ri)` (packet.rs:299-303 gates the actual deduction/box-checking on the same predicate). `spouse_63f_status_permits` (questions.rs:140-149) requires all three i1040gi conditions (`spouse_had_no_inc…"
    },
    {
      "type": "workflow_agent",
      "index": 14,
      "label": "refute:refusal-surface:1:0",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "ac9de51c9c99e8c5b",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535599427,
      "queuedAt": 1785535599426,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "The finding is CONFIRMED reachable end-to-end at the btctax…",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785535835917,
      "tokens": 103394,
      "toolCalls": 35,
      "durationMs": 236490,
      "resultPreview": "{\"refuted\":false,\"reasoning\":\"The finding is CONFIRMED reachable end-to-end at the btctax-core level; I built and executed the failing input rather than just reasoning about it.\
\
Traced every upstream guard: (1) apply_170b (charitable.rs:106) runs unconditionally on gifts (crypto_charitable_gifts reads full FMV off LT legs) with NO reference to donations_had_restrictions anywhere in its body. (2…"
    },
    {
      "type": "workflow_agent",
      "index": 15,
      "label": "refute:refusal-surface:1:1",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a790de59e386b59e3",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785535599427,
      "queuedAt": 1785535599426,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

A reviewer filed this finding against `main..HEAD`. **Your job is to REFUTE it.** Assume it is wrong
until the code forces you to conclude otherwise. Reviewers routinely file defects that are
unreachable in practice, already guarded elsewhere, or based on a misread of the control flow.…",
      "lastProgressAt": 1785536001535,
      "tokens": 124134,
      "toolCalls": 50,
      "durationMs": 402108,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Verified the finding is CORRECT by direct code reading and by executing a scratch test reproducing the exact scenario (in an isolated git worktree to avoid a shared-tree collision with another concurrent agent working in this repo).\
\
Control-flow trace confirmed exactly as claimed:\
1. return_1040.rs:1601-1602 gate: `let donated = ..…"
    },
    {
      "type": "workflow_agent",
      "index": 16,
      "label": "verdict",
      "phaseIndex": 3,
      "phaseTitle": "Verdict",
      "agentId": "af484e7b85ea1f3a4",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785536026520,
      "queuedAt": 1785536026519,
      "attempt": 1,
      "lastToolName": "Bash",
      "lastToolSummary": "grep -n \"does not ask and cannot verify\" -r crates/ | wc -l…",
      "promptPreview": "You are writing the FINAL PRE-MERGE VERDICT for the btctax repo at /scratch/code/bitcoin_tax, branch
`feat/no-pen-deferrals` (34 commits ahead of main). Read the brief first: /scratch/code/bitcoin_tax/reviews/BRIEF-merge.md

Five independent lenses reviewed `main..HEAD`. Every blocking finding was then given to TWO
independent skeptics instructed to REFUTE it. A finding was KILLED if both skeptics…",
      "lastProgressAt": 1785536226758,
      "tokens": 100592,
      "toolCalls": 11,
      "durationMs": 200237,
      "resultPreview": "VERDICT: fix-before-merge: 3 surviving Important — (1) the stale `Mfs63fSpouseBoxesForgone` advisory text, (2) a restricted-donation carryover laundered into next year's inputs stamped `Computed`, (3) the filed-PDF export path's three fail-closed screens held by no test.

Ten skeptic votes were cast across the five findings. **Zero refutations.** Every surviving finding was independently re-execut…"
    }
  ],
  "totalTokens": 2371135,
  "totalToolCalls": 876
}

---

## Disposition (author, same day) — ALL THREE FOLDED

**Every finding was confirmed by a test written FIRST, then fixed, then mutation-verified.**
Thirteen mutations observed red. Suite 2536 → **2541**; all five gates green; TY2024 golden matrix
md5 `c4e1853` unchanged.

| finding | disposition |
|---|---|
| **I-1** — the stale `Mfs63fSpouseBoxesForgone` message | **FIXED, in BOTH halves.** The message no longer claims btctax "counts spouse boxes only on a JOINT return" or that the three conditions are "things btctax does not ask and cannot verify", and no longer sends the filer to check the boxes BY HAND — that drifts the §63(f) box count away from the line-12 amount `AgedBlindBoxes::count()` is the single source for. It now names the action that works. ★ And the FIRING condition was narrowed: an **adversely answered** condition means the boxes are correctly declined and nothing is recoverable, so it stays silent. That kills shape (b) — the dangerous one — outright. `spouse_not_filing_a_return == Some(false)` is the *ordinary* MFS case, so without that guard the advisory was wrong on the commonest shape it can meet. |
| **I-2** — the carryover laundering (a REGRESSION from r3) | **FIXED** by a vouch-for gate in `apply_carryover_writeback`. Refuses to persist a charitable carryover when a restriction is DECLARED (any amount) or DUE-but-unanswered (a Section B year). Placed in **core**, not the CLI, so the signature makes an omission fail to compile — the compiler enumerated the one production call site. ★ `--force` does NOT open it: that flag overwrites a figure the USER entered; it is not a licence to write one btctax knows is wrong. Five clauses pinned, including `!is_empty()` — whose absence would have been r3's I-3 false-block one clause over. |
| **I-3** — three export screens held by no test | **FIXED** with three tests in `export_irs_pdf.rs`, each asserting BOTH halves: the refusal fires, AND `wrote_nothing(out)` — checked against the directory itself, not a hand-list that would rot. Deleting each screen reds its own test. |
| M1/M2 (13 stale `screen_compute_dependent` sites incl. a census PROVENANCE REASON, and a deleted test name) · M3 (stale MFJ-only doc cluster, incl. the doc on `standard_deduction` itself) · M8 (two `Option<bool>` leaves on bare `_`) · M9 (FBAR doc still says class (A)) · M10 (`LIMITATIONS.md` MFS widening) | **ALL FIXED.** |

### ★★★ The durable lesson, and it is not about donations

**Narrowing a refusal can leak through a different PERSISTENCE path.** r3's I-3 fix was correct about
the *year* — a standard-deduction return claims no §170 deduction, so a restriction moves no figure on
it. But `apply_170b` runs unconditionally so the carryover ages, and the carryover it produced was
still full-FMV. The year stopped being wrong and the **carryover** became wrong instead, in a row that
next year has no way to question. When you narrow a gate, ask what else consumed the value it used to
protect.

### ★★ On the review's own two blind spots

The report named them itself, which is why they are worth recording rather than resenting:

1. **Nobody built a vault and read the ink.** Four of five lenses worked at the library seam. The only
   end-to-end CLI execution came from a *skeptic*, and it is what proved finding 3. The three new
   export tests close exactly that gap — they drive `export_irs_pdf` and assert on the directory.
2. **The shared tree was contaminated.** 16 agents ran concurrently in one worktree and several
   mutated files to verify kills; three lenses independently reported finding edits they did not
   plant. Verification agents needed `isolation: worktree`. The final state was verified clean and the
   suite green, but this is a process defect in how the review was RUN, not in what it found.
