# Re-verification — interview T9 build + fold (independent worktree)

Verifier: independent sonnet re-verifier, own worktree at `0f6603a9` (the T9 fold commit, `main`).
Worktree: `/scratch/code/bitcoin_tax/.claude/worktrees/agent-af40f2143e485057a` — clean throughout;
every plant made from a `git checkout -- <file>`-clean tree and reverted the same way; `git status
--short` is empty at every checkpoint below. No commits, no subagents.

Brief: `design/agent-reports/BRIEF-reverify-interview-T9.md`. Artifacts under re-verification:
`2026-09-07-build-interview-T9-implementation.md`, `…-review.md`, `…-review-VERIFICATION.md`,
`…-fold.md`. `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` for every cargo command;
all runs scoped, never `make check` / `cargo test --workspace`.

**The one question, answered: yes.** Every kill named in the build report and the fold report REDs
on its defect today, and every one of the review's five findings (C-1, I-1, M-1, M-2, N-1) has a
holding test. No shadow kills found. 0 Important, 0 Critical.

---

## C-1 (Critical, fold) — the itemize election, through the single accessor `form_1098_deducted()`

| Kill / claim | Plant | Command | Result | Verdict |
|---|---|---|---|---|
| The accessor gates on `schedule_a.is_some()` | `form_1098_deducted()` → `&self.form_1098` unconditionally | `cargo nextest run -p btctax-core -E 'test(a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing)'` | `panicked at return_refuse.rs:3915: MortgageAllUsedToBuyBuildImprove must NOT be live on a standard-deduction return` | **RED — PASS** |
| Same plant, differential/broader effect | (as above) | same test, plus `-p btctax-core -E 'test(the_ceiling_warning_is_silent_for_a_standard_deduction_filer)'` and `-p btctax-cli -E 'test(the_opener_seeds_a_1098_onto_a_standard_deduction_year_and_nothing_refuses)'` | ceiling test: `panicked at transcription_warnings.rs:613: …so the question the warning points at is not even asked`; opener test: `panicked at open_next_year_t4b.rs:1687: ★ THE KILL: MortgageAllUsedToBuyBuildImprove must not be asked of a filer with no Schedule A` | **RED — PASS** (all three call sites root to one accessor) |
| One call site reads `ri.form_1098` directly (shared-interest loop) | `return_refuse.rs:3239` `ri.form_1098_deducted()` → `ri.form_1098` | same standard-deduction test | `panicked at return_refuse.rs:3940: a standard-deduction filer is never refused over line 8a (shared=None, 8396=None) — left: Some(SharedMortgageInterestUnanswered) right: None` | **RED — PASS** |
| `mortgage_question_live` (the three OLDER declarations) reads `ri.form_1098` directly | `questions.rs:657` `!ri.form_1098_deducted().is_empty()` → `!ri.form_1098.is_empty()` | same standard-deduction test | `panicked at return_refuse.rs:3915: MortgageAllUsedToBuyBuildImprove must NOT be live…` | **RED — PASS** (fold's "Plant 3") |
| `mortgage_interest_credit_question_live` (8396 gate) reads `ri.form_1098` directly | `if false && ri.claiming_mortgage_interest_credit == Some(true)` (guard neutralised in `screen_param_free`'s 8396 block) | `-p btctax-core -E 'test(the_form_8396_gate_refuses_on_yes_and_is_not_even_asked_without_mortgage_interest)'` | `panicked at return_refuse.rs:4104: the mortgage interest credit refuses on BOTH tiers` | **RED — PASS** (fold's "Plant 1", also build report kill 6) |
| Census row's own liveness (`document_census::row_is_live(Form1098)`) | `document_census.rs:703` `ri.schedule_a.is_some()` → `true` | same standard-deduction test | `panicked at return_refuse.rs:3920: the "form_1098" census row is live iff the filer itemizes` | **RED — PASS** (build report kill 4b) |
| **Box 4 stays ungated — must still refuse a standard-deduction filer** | none (probe on unmodified tree) | temporary probe test in `return_refuse.rs` (reverted): `schedule_a: None`, one Form 1098 row with `box4_refund_overpaid_interest = $1` | `REVERIFY-PROBE reason(&standard) = Some(MortgageInterestRefundNotComputed)` | **CONFIRMED UNGATED — PASS** |
| Box 4 refusal disabled at the commit gate | `return_refuse.rs:3218` `if m.box4_refund_overpaid_interest > Usd::ZERO` → `if false && …` | `-p btctax-core -E 'test(a_1098_box_4_refund_refuses_naming_schedule_1_line_8z_and_a_zero_does_not)'` | `panicked at return_refuse.rs:3801: a box-4 refund refuses on BOTH tiers` | **RED — PASS** (build report kill 2) |
| Box 4 refusal disabled at the import gate | same plant | `-p btctax-cli -E 'test(a_1098_box_4_refund_refuses_at_import_naming_line_8z_and_writes_no_row)'` | `panicked at year_gate_t4.rs:105: a box-4 refund must refuse at import: ()` | **RED — PASS** (build report kill 12) |

**C-1 disposition: CONFIRMED FIXED.** The accessor `ReturnInputs::form_1098_deducted()` at
`crates/btctax-core/src/tax/return_inputs.rs:2477` is the single reader for the three mortgage
declarations, the Form 8396 gate, the shared-interest refusal and the ceiling warning; every one of
five independent plants against it (the accessor itself, each of two call sites bypassing it, and
the two OLD/NEW declaration liveness functions) reds through the shipped kill test, and the reachable
path through the real `open_next_year` opener also reds. Box 4 was independently probed and stays
ungated as required — it refuses a standard-deduction filer with a box-4 refund, correctly, since
that refund is Schedule 1 line 8z income regardless of itemizing.

---

## I-1 (Important, fold) — `schedule_a_line8b_overflow`, read by both the emitter and `hand_marks`

| Kill / claim | Plant | Command | Result | Verdict |
|---|---|---|---|---|
| Emitter half — overflow reported empty | `schedule_a.rs::line8b_overflow` → always returns `&[]` | `-p btctax-forms -E 'test(line_8b_overflows_to_see_attached_on_both_years_and_names_every_recipient)'` | `panicked at full_return_forms.rs:3095: ★ every recipient is on the statement, not just the third — the escape takes the whole block — left: [] right: ["SELLER 0, …", "SELLER 1, …", "SELLER 2, …"]` | **RED — PASS** |
| Manifest half — `hand_marks` entry removed | `admin.rs`'s `if !line8b_overflow.is_empty() { marks.push(…) }` block deleted | `-p btctax-cli -E 'test(the_manifest_names_the_line_8b_statement_when_the_identities_do_not_fit)'` | `panicked at export_irs_pdf.rs:2010: ★ THE KILL: the page printed "See attached" and the manifest must say so: […]` | **RED — PASS** |
| Both years driven | (none — read the test) | inspected `line_8b_overflows_to_see_attached_on_both_years_and_names_every_recipient` | asserts TY2024 (two fit / three overflow) AND TY2025 (one fits / two overflow, the merged 24pt box) | **CONFIRMED — PASS** |

**I-1 disposition: CONFIRMED FIXED.** `btctax_forms::schedule_a_line8b_overflow` (`lib.rs:342`) is
the one door both the emitter (`schedule_a.rs:75`) and the packet manifest's `hand_marks`
(`cmd/admin.rs`) read; killing either reader reds a distinct, real test, and both TY2024 and TY2025
are driven through the real `fill_schedule_a`.

---

## M-1 (Minor, fold) — the ceiling warning is silent for a standard-deduction filer

| Kill | Plant | Command | Result | Verdict |
|---|---|---|---|---|
| Ceiling warning reads every row, not `form_1098_deducted()` | (covered above — the accessor's own plant) | `-p btctax-core -E 'test(the_ceiling_warning_is_silent_for_a_standard_deduction_filer)'` | `panicked at transcription_warnings.rs:613: …so the question the warning points at is not even asked` | **RED — PASS** |

**M-1 disposition: CONFIRMED FIXED.**

---

## M-2 (Minor, fold) — the home-sale table now crosses all 24 combinations

| Kill | Plant | Command | Result | Verdict |
|---|---|---|---|---|
| `can_exclude_all_gain == Some(false)` arm made unreachable | `return_refuse.rs:3344` `if h.can_exclude_all_gain == Some(false)` → `if false && …` | `-p btctax-core -E 'test(the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523)'` | `panicked at return_refuse.rs:4190: (true,true,false,s_1099=Some(false)) must not be the blank branch` | **RED — PASS** |
| Full 24-row cross, not a sample | (none — read the test) | inspected the test body | `for s_1099 in [Some(false), Some(true), None] { for t1 … { for t2 … { for exclude … } } } }`, `assert_eq!(rows, 24, "the full cross, not a sample")` | **CONFIRMED — PASS** |

**M-2 disposition: CONFIRMED FIXED.** The fold's own disclosed residual — the `s_1099 == Some(true)`
arm inside this same rule is unreachable because the document census screens first on both tiers —
was independently re-read (`return_refuse.rs:3330-3338`) and matches the fold report and the
controller's settled note verbatim; it is already filed as **FR-87**, owned by T12, and is not
re-reported here.

---

## N-1 (Nit, fold) — the ceiling warning now formats through `money()`

| Kill | Plant | Command | Result | Verdict |
|---|---|---|---|---|
| `money()` calls removed, raw interpolation restored | `transcription_warnings.rs` — deleted `let (total, limit) = (money(total), money(limit));` | `-p btctax-core -E 'test(the_ceiling_warning_formats_both_figures_as_money)'` | `panicked at transcription_warnings.rs:641: both figures print through money(): "…adds up to 900000, which is more than the 750000…"` (no `$`, no decimals) | **RED — PASS** |

**N-1 disposition: CONFIRMED FIXED.**

---

## Build-report kills (checklist part 1) — re-planted directly against today's tree

| # | Kill | Plant | Command | Result | Verdict |
|---|---|---|---|---|---|
| 1 | 8e = 8a + 8b + 8c | `printed.rs:1679` `line8a + line8b + line8c` → `line8a` | `-p btctax-core -E 'test(schedule_a_line_8e_adds_8a_8b_and_8c_from_the_return)'` | `panicked at printed.rs:2862: ★ THE KILL: "Add lines 8a through 8c" — 12,700 + 940 + 300 — left: 12700 right: 13940` | **RED — PASS** |
| 2 | box-4 rule (commit gate) | see C-1 table | see C-1 table | see C-1 table | **RED — PASS** |
| 3 | shared-interest BLANK silently "no" | `None` arm of `match m.other_borrower_paid_interest` in `return_refuse.rs` made `None if false`, fallback added | `-p btctax-core -E 'test(the_shared_interest_gate_refuses_on_yes_and_on_a_blank_but_not_on_no)'` | `panicked at return_refuse.rs:3841: a BLANK is not a "no"… left: None right: Some(SharedMortgageInterestUnanswered)` | **RED — PASS** |
| 4 | declarations lose the itemize-election conjunct | see C-1 table (`mortgage_question_live`) | see C-1 table | see C-1 table | **RED — PASS** |
| 4b | census row loses it | see C-1 table (`row_is_live`) | see C-1 table | see C-1 table | **RED — PASS** |
| 5 | 8b identity rule cannot fire | `return_refuse.rs` `if r.recipient_tin.trim().is_empty()` → `if false && …` | `-p btctax-core -E 'test(a_line_8b_row_with_no_identifying_number_refuses_and_one_with_a_number_files)'` | `panicked at return_refuse.rs:4061: an unidentified 8b recipient refuses on BOTH tiers` | **RED — PASS** |
| 6 | 8396 rule cannot fire | see C-1 table | see C-1 table | see C-1 table | **RED — PASS** |
| 7 | home-sale "cannot exclude all gain" branch dropped | see M-2 table (identical mechanism) | see M-2 table | see M-2 table | **RED — PASS** |
| 8 | ceiling becomes strict `<` | `transcription_warnings.rs:363` `if total <= limit` → `if total < limit` | `-p btctax-core -E 'test(the_acquisition_debt_warning_fires_over_the_ceiling_and_is_silent_under_it)'` | `panicked at transcription_warnings.rs:457: AT the ceiling is not over it — the instruction says "up to $750,000" — left: Some("…$750000.00…") right: None` | **RED — PASS** |
| 9 | ceiling becomes per-row (max) instead of aggregate (sum) | `return_inputs.rs::form_1098_outstanding_principal` — `.sum()` → `.max().unwrap_or(Usd::ZERO)` | `-p btctax-core -E 'test(the_acquisition_debt_warning_sums_every_1098_row)'` | `panicked at transcription_warnings.rs:481: TWO of them are $1,000,000 of acquisition debt and must warn on the SUM` | **RED — PASS** |
| 10 | untranscribed box 3 takes the pre-2018 (larger) ceiling | `tables.rs::for_loan` — `origination.is_some_and(…)` → `origination.is_none_or(…)` | `-p btctax-core -E 'test(the_acquisition_debt_warning_halves_for_mfs_and_reads_the_origination_date)'` | `panicked at transcription_warnings.rs:519: an UNTRANSCRIBED box 3 takes the stricter post-2017 ceiling — fail-closed` | **RED — PASS** |
| 11 | MFS is not halved | `tables.rs::for_loan` — `(false, Mfs) => after_dec_15_2017_mfs` → `after_dec_15_2017` | same test as #10 | `panicked at transcription_warnings.rs:505: $400,000 is over the $375,000 MFS ceiling` | **RED — PASS** |
| 12 | box-4 rule cannot fire, import side | see C-1 table | see C-1 table | see C-1 table | **RED — PASS** |
| 13 | ceiling display beside the declaration removed | `answer.rs:668` `if q.id == QuestionId::MortgageWithinDebtLimit` → `if false && …` | `-p btctax-cli -E 'test(the_acquisition_debt_ceiling_is_shown_beside_the_debt_limit_question)'` | test FAILED (panic in the assertion body; output confirms the warning line is absent from the transcript) | **RED — PASS** |
| 14a | opener's `payer_of` has no Form 1098 arm | the `DocumentRow::Form1098` arm deleted from `payer_of`'s exhaustive `match` in `open_next_year.rs`, with no replacement | `cargo build -q -p btctax-cli` | `error[E0004]: non-exhaustive patterns: 'DocumentRow::Form1098' not covered` | **COMPILE-FAILS — stronger than a runtime kill — PASS** |
| 14b | opener's `seed` does not carry the lender identity | `open_next_year.rs` — `seed`'s `form_1098: …collect()` → `form_1098: Vec::new()` | `-p btctax-cli -E 'test(a_form_1098_lender_is_seeded_as_an_identity_with_every_box_blank)'` | `panicked at open_next_year_t4b.rs:1604: the lender identity is carried — left: 0 right: 1` | **RED — PASS** |
| box census ×3 | deleted box-4 entry / one-character caption drift / edition-narrowed entry, all embedded as `expect_err` inside one test | `-p xtask -E 'test(the_1098_entries_red_on_a_deleted_box_a_drifted_caption_and_a_narrowed_edition)'` | test **passes**, and passing IS the observed red on all three `expect_err` calls (verified by reading the three `.expect_err(...)` sites and their message assertions in `box_census.rs`) | **PASS (self-contained kill, confirmed structurally)** |

Every one of the 14 build-report kills, plus the box-census's own three embedded plants, reproduces
its stated red on the fold commit's tree. Kill 14a is now a compile error rather than a runtime
panic — a **stronger** guarantee than the build report originally shipped (the exhaustive `match` with
no `_` arm was itself added by T9, per its own D-8.3 note), so this is a strengthening, not a
regression, and is recorded as a pass.

---

## Negative claims (checklist part 3)

| Claim | Check | Result | Verdict |
|---|---|---|---|
| `box-census` OK with both 1098 editions joined | `cargo run -q -p xtask -- box-census` | `f1098--2022`: 11 boxes, 8/1/2; `f1098--2025`: 11 boxes, 8/1/2; `box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one decided (268 entries)` | **PASS — byte-identical to the settled figure** |
| `line-coverage` holds 8a/8b/8c/8e | `cargo run -q -p xtask -- line-coverage` | `line-coverage OK: 375 money lines across 18 form(s) […f1040sa:21…], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0), 17 not line-bound (ratchet 17)` | **PASS — byte-identical** |
| `census-join` as stated | `cargo run -q -p xtask -- census-join` | `census join: 283 unmodeled entries across 13 maps…` | **PASS — byte-identical** |
| The two-oracle sweep reconciles | `cargo nextest run -p btctax-oracle-harness` (`OTS_DIR` unset in this worktree — the live-OTS test is the 1 skipped) | `5 passed, 1 skipped`, including `check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells … ok` | **PASS on the available (pinned-golden) oracle check; the live-OTS half is environment-unavailable, marked unverified rather than passed, per the brief** |
| TY2024 golden corpus files with the scalar replaced by the row | read `testonly.rs:1229-1243` | `let interest = golden_usd(i.itemized_deductions + i.mortgage_interest); … ri.form_1098.push(Form1098 { box1_interest: interest, other_borrower_paid_interest: Some(false), … })` — exactly the moved figure the build report describes, and the sweep test above passes with it in place | **PASS** |
| A standard-deduction fixture's printed Schedule A is unchanged | `-p btctax-forms -E 'test(a_five_thousand_dollar_gift_under_the_standard_deduction_files_no_schedule_a_and_no_8283)'` | `test … ok` | **PASS** |
| Every new shaped identifier in fixtures is allowed | `bash scripts/pii-scan-generic.sh` | `pii-scan: clean (HEAD).` | **PASS, exit 0** |

## Scoped suites (sanity — not a substitute for the per-kill runs above)

All pass on the clean, unplanted tree: `-p btctax-core -E 'test(form1098) | test(home_sale) |
test(schedule_a) | test(return_refuse) | test(printed)'` → 155 passed; `-p btctax-forms -E
'test(f1040sa) | test(field_census)'` → 1 passed (1 skipped elsewhere is name-filter shape, not a
failure); `-p btctax-input-form` → 72 passed; `-p xtask -E 'test(box_census) |
test(line_coverage)'` → 19 passed; `-p btctax-cli` fullreturn/oracle fixture tests → 2 passed.
(`make check`'s 3475/12/exit-0 is the controller's already-settled figure and was not re-run, per
the brief.)

## What was NOT independently re-derived

Per the brief: the ledger's dispositions, the design/spec judgment calls (D-1 through D-10), and the
`make check` baseline count — all taken as settled. FR-87 (the home-sale rule's own unreachable
`s_1099 == Some(true)` arm) was read and confirmed present, not re-reported.

---

Counts: C=0 I=0 M=0 N=0
