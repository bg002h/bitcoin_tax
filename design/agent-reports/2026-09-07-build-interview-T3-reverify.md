# Reverify — the interview T3 fold's kills, replanted (sonnet, worktree, own worktree)

Independent re-verifier, own worktree (checked out at `15c15927`, main's HEAD at dispatch; the fold
under verification is `750c1b34`). No commits, no subagents. Every plant below was made on the real
tree from a `cp` backup and reverted; `git status --porcelain` is empty at the end. Brief:
`design/agent-reports/BRIEF-reverify-interview-T3.md`.

**Environment note (not a finding, resolved before any plant).** The worktree assigned at dispatch
was pinned to a stale branch tip (`2bd04d45`, pre-dating this fold) rather than `main`'s `15c15927`.
Fixed by `git checkout 15c15927` (a commit reachable via `main`, checked out directly by SHA — no
branch pointer touched, no push, no interference with the shared checkout). All work below runs on
that tree.

`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` was set before every cargo command.

---

## Part 1 — every kill in the fold report, replanted

| # | kill | plant made | command | observed | verdict |
|---|---|---|---|---|---|
| 1 | I1(b) — `direction` key is a parse error | `direction = "NoDollar"` added back into `2024/f1040.map.toml`'s `Income` block | `census-join` | `TOML parse error at line 221 … unknown field \`direction\`, expected one of \`caption\`, \`extract_line\`, \`first_line\`, \`last_line\`` | **RED — verbatim match** |
| 2 | I1 evidence — a `NoDollar` reading given a line range (Kill 1's plant, expressed as a reading; in-suite case 16) | `DIRECTION_OF_CAPTION`'s `"Income"` reading changed `Understates → NoDollar` | `census-join`; `nextest -p xtask -E 'test(census_join)'` | `[[direction]] "Income" is graded \`NoDollar\` and yet heads the form's numbered lines 1a–9 …`; in-suite `the_committed_maps_are_covered_and_placed` FAILS on the same message | **RED — verbatim match, both instruments** |
| 3 | Kill 3 / I1 evidence — one reading deleted | the `"Income"` tuple removed from `DIRECTION_OF_CAPTION` | `census-join`; `nextest -E 'test(every_committed_caption_has_exactly_one_reading_and_every_reading_is_used)'` | `17 finding(s)`: orphaned caption named first, then 16 unplaceable entries; in-suite `left: 0, right: 1` | **RED — count (17) matches exactly** |
| 4 | Kill 4 / I1 evidence — a reading for a caption no map carries | `("Part IV Information on Your Vehicle", Overstates, "bogus reading…")` added to `DIRECTION_OF_CAPTION` | `census-join`; same in-suite test | `2 finding(s)`: the dead-reading message, plus (bonus) the evidence sentence not printed by any extract; in-suite FAILS on the first message | **RED — verbatim match, and the evidence-assertion check fires too** |
| 5 | I1 evidence — line 1b's cover moved to an `Advisory` (the laundering plant, now unrepresentable via a block key) | `2024/f1040.map.toml` line 1b: `covered_by` changed `QuestionId::OtherOutOfScopeIncome → Advisory::UnmodeledDeductionsOmitted` | `census-join` | `an ADVISORY covers an \`Understates\` line ("Income"). A blank there is a FALSE STATEMENT …` | **RED — verbatim match; confirms the review's Plant 2 is closed even with no key left to downgrade the block** |
| 6 | K10 rerun | `2024/f1040s1` line 2a: `covered_by → Advisory::EicOmitted` | `census-join` | `an ADVISORY covers an \`Understates\` line ("Part I Additional Income") …` | **RED — verbatim match** |
| 7 | K11 rerun | `"Jury duty pay; "` deleted from the `OtherOutOfScopeIncome` prompt (`questions.rs`) | `census-join` | `line "8h": OtherOutOfScopeIncome's prompt never says "Jury duty pay". A variant that exists is not a cover …` | **RED — verbatim match** |
| 8 | K12 rerun | the `Part II Adjustments to Income` `[[direction]]` block deleted from `2024/f1040s1.map.toml` | `census-join` | `27 finding(s)` — 26 Part II entries unplaceable + the now-orphaned reading | **RED — count matches fold's "27 (was 26)" exactly** |
| 9 | K13 rerun | caption drift, `"…Adjustments to Income"` → `"…Incomes"` | `census-join` | `29 finding(s)` — verbatim-extract red, drifted caption has no reading, real caption's reading orphaned, 26 unplaceable entries | **RED — count matches exactly** |
| 10 | K-D11a rerun | `[[subtracts]]` block deleted from `2024/f1040sb.map.toml` | `census-join` | `1 finding(s)` — the Schedule B line 3 `Understates` cover breaks | **RED — matches exactly** |
| 11 | K-D11b rerun | `"Subtract line 3 from line 2"` → `"…from line 1"` | `census-join` | `2 finding(s)` — the fake sentence, and the cover it was propping up, together | **RED — matches exactly** |
| 12 | I2/I3 — probes P1/P2 pass; the negative refusals did not relax | none (ran the committed test as-is) | `nextest -p btctax-core -E 'test(a_truthful_1099b_on_form_8949_and_a_box_2_only_1099g_both_file)'` | PASS. P1 (`b_1099=true`, zero rows, ledger disposals) files `None`; P2 (`g_1099=true`, box 2 only) files `None`; `b_1099=false` beside a row still `DocumentCensusContradicted`; `w2=true` with zero rows still `DocumentDeclaredNotTranscribed` — all four asserted in the same test | **PASS as claimed — holding test confirmed** |
| 13 | Kill 5 | `requires_transcription(B1099)` flipped `false → true` (`document_census.rs`) | `nextest -E 'test(a_truthful_1099b…) \| test(the_rows_invariant…) \| test(the_four_1099_rows_declare…)'` | **3/3 tests FAILED**, each with the exact quoted diff (`left: [W2, Int1099, Div1099, B1099]` / `right: […, ]` without B1099; `left: Some(DocumentDeclaredNotTranscribed{…})` / `right: None`; the task-naming assertion) | **RED — verbatim match, all three** |
| 14 | Kill 6 | `requires_transcription(row)` conjunct removed from `census_row_invariant`'s guard (`interview_state.rs`) | `nextest -E 'test(the_declared_document_and_rows_invariant_is_a_panel_item)'` | `a Form 1099-B whose transactions are all on Form 8949 is a CORRECT return with zero summary rows — the panel must not refuse what the screen files: [Refusing { … DocumentDeclaredNotTranscribed { kind: B1099 } … }]` | **RED — verbatim match** |
| 15 | I4 kill | `"fuel tax"` removed from the `OtherOutOfScopeIncome` attestation prompt (`questions.rs`) | `nextest -p xtask -E 'test(schedule_c_line_6_is_covered_and_the_attestation_reaches_it)'` | `2024: REACH — the attestation never says "fuel tax", so a filer can answer "No" to it without ever reading the line's name` | **RED — verbatim match** |
| 16 | Kill 7 / I5 | old blanket sentence restored, `"THOSE SEVEN"` → `"these"` (`advisories.rs`) | `nextest -E 'test(the_return_options_advisory_scopes_its_reassurance_and_warns_off_the_nra_election)'` | `the blanket reassurance covered the §6013(g)/(h) election, which DOES change the tax: …` | **RED — verbatim match** |
| 17 | N1 kill | the liveness guard removed from `census_tristate!`'s `clear` arm (`registries.rs`) | `nextest -p btctax-input-form -E 'test(clearing_a_non_live_census_row_is_refused_exactly_as_setting_it_is)'` | `assertion \`left == right\` failed: ★ …and \`clear\` must refuse on the SAME predicate … left: Ok(()), right: Err(SetError(NoSuchRow))` | **RED — verbatim match** |
| 18 | M5 property | the document-first partition removed from `live_questions` (`answer.rs`), falling back to raw registry order | baseline: `nextest -p btctax-cli -E 'test(a_single_filer_is_asked_the_always_live_declarations_and_no_spouse_question)'` → PASS; then plant, same test | Baseline **PASS**; under the plant, **FAILED** — expected vector leads with `DocW2, DocInt1099, …` (census first), observed reverts to gate-declarations-first (`DependentTaxpayer, ForeignAccounts, … DocW2, …`) | **PASS at baseline + RED under the plant — the property holds and is guarded by a test** |

Every plant above was reverted immediately after observation; `git diff --stat` on the touched file
was empty before moving to the next plant, and a final `git status --porcelain` (below) confirms the
tree is clean.

---

## Part 2 — negative claims

| claim | check | result |
|---|---|---|
| `census-join`: 298 entries / 13 maps / 22 readings | `cargo run -q -p xtask -- census-join` | `census join: 298 unmodeled entries across 13 maps, … graded by one of DIRECTION_OF_CAPTION's 22 readings …` — **matches** |
| `line-coverage`: 341/24/0/12 | `cargo run -q -p xtask -- line-coverage` | `341 money lines across 17 form(s) …, 24 exception(s) (ratchet 24), 0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)` — **matches** |
| `stop-list` clean | `cargo run -q -p xtask -- stop-list` | `8 btctax-input-form sources, 4 state-bearing sources and 54 registry prompts scanned; no forbidden shape` — **matches** |
| no `direction =` key remains outside comments | `grep -rn 'direction *=' crates/btctax-forms/forms/` → 11 hits, all `#     \`deny_unknown_fields\`, so a \`direction = "..."\` key here is a PARSE ERROR …` comment lines; `grep -rn '^[^#]*direction *=' crates/btctax-forms/forms/` → **0 hits** | **matches — 11 comment hits, 0 keys** |
| the T3 build's original 19 kills and D1/D11 kills still green | full scoped-crate reruns (below) | **all green** |

**Full scoped-crate reruns** (tree clean, no plants active):

```
btctax-core          1251 tests run: 1251 passed, 0 skipped     (pinned: 1251 — matches)
btctax-cli             723 tests run:  723 passed, 1 skipped     (pinned: 723/1 — matches)
btctax-forms            354 tests run:  354 passed, 4 skipped     (pinned: 354/4 — matches)
btctax-input-form        68 tests run:   68 passed, 0 skipped     (pinned: 68 — matches)
xtask                   154 tests run:  147 passed, 7 failed, 1 skipped
```

xtask's 7 failures are exactly the six `form_delta::*` tests plus
`harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` — the
brief's pre-declared PDF-less-worktree / redirected-target-dir **environment** failures, not
findings. No other failure appeared in any scoped or full-crate run.

**Out of this reverify's scope** (per brief, which lists I1–I5, M5 and N1 as the review findings to
re-plant): M1 (stale doc comments), M3 (dead-sum conjunction note), M4 (D11 PAREN convention) and D16
were not re-verified here — they carry no "kill" test in the fold report's own checklist and the brief
does not ask for them.

---

## Final state

```
$ git status --porcelain
(empty)
```

Every one of the fold report's named kills reproduces verbatim (message and — where the fold gives
one — the exact finding count). Every review finding (I1–I5, M5, N1) that the brief asks to be
re-planted after the fold reds exactly as claimed, or (for I2/I3's probes) passes exactly as claimed
with the un-relaxed refusals still holding in the same test. All four negative claims hold. No
discrepancy, no kill that failed to red, no finding without a holding test.

Counts: C=0 I=0 M=0 N=0
