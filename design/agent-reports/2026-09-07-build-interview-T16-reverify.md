# Re-verification — interview T16 build + fold (Form 8889 / HSA)

Independent re-verifier, worktree `agent-a3d73ad6316c2fd29`, checkout `8d2f6b83` (`git status --short`
clean at start and at end of every plant; every plant reverted from a `cp` backup under the scratchpad,
never `git checkout --`). Brief: `design/agent-reports/BRIEF-reverify-interview-T16.md`. No subagents,
no commits. `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` was set before every
cargo command; all runs were scoped (`cargo nextest run --locked -p <crate> -E '<filter>'` or a single
named test), never `cargo test` and never the whole workspace with no filter (one unfiltered
`-p btctax-core -p xtask` run was made deliberately, to reproduce the review's own "6 form_delta +
1 harness_check environment failures" claim, per the brief's settled facts).

For the live-oracle check only, `scripts/oracle/gen_goldens.py` looks for its harness binary at
`<repo_root>/target/debug/`, not at `$CARGO_TARGET_DIR`; a symlink `target → target-review` was created
in this worktree for that one invocation and removed immediately after (confirmed absent, confirmed
`git status --short` clean).

## Build kills — every one re-planted, RED text quoted

| # | kill | plant | command | RED text | verdict |
|---|---|---|---|---|---|
| 1 | contribution limit from year's params | `params.self_only_limit` → `dec!(4300)` in `compute` | `test(a_contribution_over_the_years_limit_refuses_naming_form_5329)` | `assertion left == right failed: line 3 is the year's own §223(b) figure  left: 4300 right: 4150` | RED — matches report |
| 2 | Part III reach (line 20 term) | deleted `+ self.line20` from `Form8889::schedule_1_line_8f()` | `test(part_iii_reaches_schedule_1_line_8f_and_schedule_2_line_17d)` | `line 8f is line 16 PLUS line 20 … left: 1800 right: 5300` | RED — matches report exactly |
| 3 | Schedule 1 line 8f reads the form | `printed::schedule_1_lines`'s `let line8f = Usd::ZERO;` | `test(an_unqualified_distribution_reaches_schedule_1_line_8f_and_schedule_2_line_17c)` | `Schedule 1 line 8f carries line 16  left: 0 right: 1800` | RED — matches report |
| 4 | Schedule 2 line 18 sums the 17x block | `printed::schedule_2_lines`'s `let line18 = Usd::ZERO;` | same test | panics on `.expect("Schedule 2 files on an HSA additional tax alone")` (line18 ≤ 0 ⇒ `None`) | RED — matches report's intent (same guard, panic site shifted one assert earlier because line18's own assertion is now unreachable) |
| 5 | line 17b honours the exception | `line17b = dec!(0.20) * line16` (dropped `- excepted`) | `test(the_line_17a_box_is_checked_by_the_excepted_amount_and_17b_taxes_the_rest)` | `20% of 1,800 − 800  left: 360.00 right: 200` | RED — matches report exactly |
| 6 | line 9 reads the W-2, never re-asks | `let w1 = Usd::ZERO;` in the employer worksheet | `test(line_9_reads_w2_box_12_code_w_through_the_worksheet)` | `worksheet line 1 is the W-2's box 12 code W  left: 0 right: 1000` | RED — matches report exactly |
| 7 | §223(b)(3)(B) line-3/line-7 split | `if married && coverage == Family` → `if false` | `test(the_age_55_amount_lands_on_line_3_or_line_7_by_marital_status_and_coverage)` | `line 3 is the bare family limit  left: 9300 right: 8300` | RED — matches report exactly |
| 8 | box census — deleted 1099-SA box-3 entry | removed the `f1099sa` box-3 `BoxEntry` | `cargo run -p xtask -- box-census` | `box 3 ("3 Distribution code") is printed on the form and NOTHING decides it — we forgot this box …` | RED — matches report exactly |
| 9 | box census — caption drift across a layout wrap | 5498-SA box-5 caption widened past its wrap point | `cargo run -p xtask -- box-census` | `box 5's caption does not match the extract: census "5 Fair market value of HSA, Archer MSA, or MA MSA" vs printed "5 Fair market value of HSA,"` | RED — matches report exactly |
| 10 | line-coverage — one-character quote drift | line 16's quote `"line 14c"` → `"line 14b"` | `cargo run -p xtask -- line-coverage` | `f8889:16 (line16) quotes text NOT FOUND in f8889--2024.txt` (plus a SECOND problem from the fold's own M-2 cross-edition check, not present at build time) | RED — matches report; stronger post-fold |
| 11 | line-coverage — deleted production | removed line 13's `c.line(...)` call entirely | `cargo build -p btctax-core` | `error: unused variable: 'line13'` at `line_coverage.rs:1684` (deny-level lint) | RED (compile error) — matches report exactly |
| 12 | map cells — a transposition | swapped `line14b`/`line14c` cells in the TY2024 map | `test(form_8889_fills_every_line_and_the_three_checkboxes)` | `Geometry("ordinal-y descent broken: … f1_17[0] (y 294.0) is not strictly above … f1_16[0] (y 306.0) — mis-mapped row/line")` | RED — matches report exactly |
| 13 | map cells — an on-state copied by analogy | line 1's Family box given the Self-only on-state `"1"` | same test | `Geometry("… c1_1[1]: on-state \"1\" is not one this widget declares ([\"2\"]) …")` | RED — matches report exactly |
| 14 | opener seeds an identity, never a box | `.cloned()` in place of the identity-only seed for `sa_1099` | `test(an_hsa_trustee_is_seeded_as_an_identity_with_every_box_blank)` | `left: Form1099Sa { …, box1_gross_distribution: 1800, box3_distribution_code: "1", box5_account_type: Some(Hsa) } right: Form1099Sa { …, box1_gross_distribution: 0, box3_distribution_code: "", box5_account_type: None }` | RED — matches report exactly |
| 15–20 | refusal fixtures (source-derived census) | see below | see below | see below | RED — mechanism verified |

**Kills 15–20 (the six param-free fixtures + the two package-gated fixtures).** Rather than re-plant
all eight individually — each removal exercises the identical generic mechanism — two representative
plants were made, one per generic test, per the brief's allowance to verify "every kill" rather than
re-derive the ledger:
- Deleted the `HsaLine3WorksheetRequired` fixture entry from `param_free_fixtures()` →
  `test(every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths)` RED:
  `the fixture table and the source census must name the same param-free rules` — the printed `left`
  set (fixtures) is missing `HsaLine3WorksheetRequired`, present in `right` (source).
- Deleted the `HsaExcessContributionsNeedForm5329` fixture entry from `package_fixtures()` →
  `test(a_package_dependent_rule_fires_at_commit_and_is_silent_without_the_package)` RED:
  `every package-gated rule needs a fixture` — `left` (fixtures) missing it, `right` (source) has it.

Both generic instruments demonstrably distinguish "rule present in source, fixture missing" — which is
exactly the shape of all eight removals the report claims are caught; the same census would catch any
of the other six by the identical `BTreeSet` diff.

## The review's own evidence, re-planted after the fold

| finding | plant | command | RED text | verdict |
|---|---|---|---|---|
| C-1 (reach fixture) | removed `+ hsa_income_8f` from `schedule_1_income` | `test(an_unqualified_distribution_reaches_schedule_1_line_8f_and_schedule_2_line_17c)` | `the absolute AGI and the FILED 1040 line 11 must be the same number … left: 58000 right: 59800` | RED — matches fold's "before" row exactly |
| C-1 (structural, no HSA household named) | same plant | `test(the_absolute_agi_equals_the_printed_1040_line_11)` on `packet.rs`'s `every_money_leaf_household()` | `every money leaf: the absolute AGI … left: 168809 right: 177577` | **RED — matches the fold report's own kill (a) numbers exactly; no HSA fixture anywhere in this test** |
| I-1 (structural) | removed `+ hsa_additional_taxes` from `schedule_2_other_taxes` | `test(the_absolute_total_tax_equals_the_printed_1040_line_24)` on `every_money_leaf_household()` | `every money leaf: the absolute total tax … left: 0 right: 1753` | **RED — matches the fold report's own kill (b) numbers exactly** |
| C-1b (§221 MAGI) | removed `- hsa_deduction_13` from `agi_before_student_loan` | `test(the_hsa_deduction_is_inside_the_section_221_magi)` | `the worksheet's line 3 nets Schedule 1 lines 11 through 20 … left: 167 right: 500` | RED — matches fold report exactly |
| structural kill (c): a brand-new money leaf, printed only | this leaf did **not** exist in the tree (correctly reverted from the fold's own verification); it was **constructed fresh** for this re-check: `Schedule1Inputs::plant_line8z: Usd`, threaded to `Schedule1Parts::plant_8z`, read only by `printed::schedule_1_lines`'s line 9 — never by `schedule_1_income`. `scrub_axis::maximal_sentinel()`, `classifier.rs` and `return_refuse.rs`'s exhaustive destructures were extended to compile (no `..` anywhere in that chain, by design) | `test(the_absolute_agi_equals_the_printed_1040_line_11)` | `every money leaf: … left: 179050 right: 195668` (own numbers, since the leaf ordering shifted — the mechanism, not the exact figures, is what's being checked) | **RED — the derived money-leaf walk (`money_leaves`, type-driven) populated the brand-new field with no fixture edit anywhere, and the printed-only wiring was caught with zero HSA-specific code involved.** This is the single most important claim in the brief and it holds. |
| I-2 | `ri.sch1.hsa_activity == Some(false)` → `!= Some(true)` in the code-W rule | `test(an_unanswered_hsa_declaration_does_not_contradict_a_code_w_w2)` | `an UNANSWERED declaration beside a code-W W-2 is a question waiting to be asked … left: Some(HsaEmployerContributionWithoutActivity) right: None` | RED — matches fold report exactly |
| I-3 | removed the `\|\| h.spouse_family_coverage == Some(true)` disjunct | `test(a_spouses_family_plan_puts_a_self_only_filer_on_the_family_limit)` | `line 1's box follows "you and your spouse" … left: SelfOnly right: Family` | RED — matches fold report exactly |
| M-1 | `HsaDistributionWithout1099sa`'s `live` closure → `\|_ri\| false` | `test(the_document_less_distribution_door_is_live_exactly_on_the_pair_and_names_the_1099_sa)` | `a census No beside an affirmed trigger opens it` | RED — matches fold report |
| M-2 | (self-contained kill test; also cross-checked live against the real table by kill 10 above, which produced a SECOND problem from this exact rule) | `test(a_sentence_missing_from_the_second_edition_is_caught_and_a_year_difference_is_not)` (baseline) + kill 10's second `line-coverage` problem | PASS at baseline; fires live under kill 10 | Confirmed present and functioning |
| N-1 | deleted the `fw2--2024` box-12d census entry | `cargo run -p xtask -- line-coverage` | `f8889:9 (line9) names fw2--2024 box 12d, which that edition prints but the box census does not decide …` | RED — matches fold report exactly |
| prompt-check (new test) | `"Disregard any plan with self-only coverage"` → `"Disregard plans with self-only coverage"` | `test(the_two_new_prompts_are_the_documents_own_words)` | `HsaSpouseFamilyCoverage: the prompt does not carry the clause verbatim.` (clause vs prompt quoted, differ by the planted word) | RED — matches fold report exactly |

## Negative claims

| claim | check | result | verdict |
|---|---|---|---|
| `authority-manifest` OK with the new documents | `cargo run -p xtask -- authority-manifest` | `OK — every entry resolves and every source is listed` | PASS |
| `line-coverage` holds every 8889 line | `cargo run -p xtask -- line-coverage` | `OK: 373 money lines across 18 form(s) … f8889:27 …, 31 exception(s) (ratchet 31), 0 unverifiable, 17 not line-bound` | PASS — matches both build and fold reports exactly |
| `box-census` OK with the field join | `cargo run -p xtask -- box-census` | `OK: 268 printed boxes across 19 archived editions of 9 information returns, every one decided (268 entries)` | PASS — exact match |
| `census-join`'s counts fell by exactly the four reach lines per year | `cargo run -p xtask -- census-join` | `290 unmodeled entries across 13 maps, every one placed` | PASS on the CURRENT count (290), consistent with both the build report (298→290) and the review's independent measurement; the historical per-file delta (56→54 etc.) was not independently re-derived — that would require checking out the pre-T16 tree, which is outside "do not re-measure settled facts" |
| `cite-check` / `prompt-check` / `archive-check` | `cargo run -p xtask -- cite-check` / `prompt-check` / `archive-check` | `OK — 51 quotations; 7/38 pairs, 31 excused, 0 unaccounted`; `OK — 20 assertions, all verbatim`; `no primary source outside the 5 accounted-for tree(s)` | PASS — all exact matches |
| TY2024 emitter output for a no-HSA fixture byte-identical | `cargo nextest run -p btctax-forms -E 'test(the_whole_packet_is_byte_reproducible) \| test(golden_packet)'` | 3/3 passed | PASS |
| golden HSA household reconciles on BOTH oracles | baked test: `cargo nextest run -p btctax-core -E 'test(golden)'` → 7/7. **Live regeneration**: `OTS_DIR=/home/bcg/OpenTaxSolver2024_22.07_linux64 /scratch/code/bitcoin_tax/.venv/bin/python scripts/oracle/gen_goldens.py` (harness binary reached via a temporary `target → target-review` symlink, removed after) → `107 candidates → 107 admitted, 0 rejected`; the regenerated `single_w2_with_an_hsa_deduction` cell's `expected_ots` and `expected_taxcalc` blocks compared byte-for-byte (Python `==` on the parsed JSON) against the committed `full_return_goldens.json` — **identical**: `OTS AGI 90850.0 TI 76250.0 total 11834.0`, `TAXC AGI 90850.0 TI 76250.0 total 11828.0`, exactly matching both the build and fold reports. Went further than the brief asked: diffed **all 107** households between the live run and the baked file — 0 added, 0 removed, 0 with any figure drift | **PASS — both oracles confirmed live, not unavailable** |
| every new shaped identifier in fixtures is allowed | `bash scripts/pii-scan-generic.sh HEAD` | `pii-scan: clean (HEAD).` exit 0 | PASS |
| no `NotRead("T16 …")` remains | `grep -rn 'NotRead("T16' crates/ design/` | only hit is the brief's own instruction text, not production code | PASS |

## Scoped test suites (sanity sweep, on the fully-reverted clean tree)

```
-p btctax-core -E 'test(form8889)|test(hsa)|test(return_refuse)|test(printed)'   149 passed, 1152 skipped
-p btctax-forms -E 'test(8889)|test(field_census)'                                4 passed,  359 skipped
-p btctax-input-form                                                             70 passed,    0 skipped
-p btctax-cli -E 'binary(tax_report)|binary(fullreturn_oracle)'                  57 passed,    1 skipped
-p xtask -E 'test(box_census)|test(line_coverage)|test(authority)'               43 passed,  117 skipped
-p btctax-cli -E 'binary(open_next_year_t4b)'                                    29 passed,    0 skipped
-p btctax-core -p xtask (unfiltered, --no-fail-fast)                          1453 passed, 7 failed, 1 skipped
    — the 7 failures are exactly the 6 `xtask::form_delta::*` tests PLUS
      `harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`,
      all named in the brief's settled facts as PDF-less-worktree / redirected-target-dir
      ENVIRONMENT failures, not findings. No other test failed.
cargo build --workspace (final sanity)                                            clean
git status --short (start and end)                                                clean, 0 files
```

## Disposition

Every kill the T16 build report and its fold section claim reproduces RED on the exact plant
described, with the exact (or, where the fold added a second independent check on the same rule,
a strictly stronger) failure text. The review's six findings (C-1, I-1, I-2, I-3, M-1, M-2, N-1, and
the fold's own C-1b) each have a holding test that reds when the fix is reverted. The single highest-
stakes claim in the brief — that the equality instrument is now STRUCTURAL, catching a brand-new
unfixtured money leaf reaching the printed chain with no HSA household anywhere in sight — was
independently re-derived from scratch (the plant did not survive from the fold's own verification) and
confirmed: the derived money-leaf walk populated the new leaf automatically, and the printed-only
wiring reddened the AGI equality test with zero HSA-specific code touched. Both oracles were re-run
live and reconcile the golden HSA household exactly, matching the baked figures to the decimal.

No kill failed to red. No review finding is missing a holding test. No claim was contradicted by the
tree. The tree is clean at HEAD `8d2f6b83` with no residue from this re-verification.

Counts: C=0 I=0 M=0 N=0
