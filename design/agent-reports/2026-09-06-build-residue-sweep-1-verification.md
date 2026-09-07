# Verification — residue sweep 1 (`ca3b3eb9`, briefed by `BRIEF-build-residue-sweep-1.md`)

Independent, read-only verifier. Worktree started stale (on an unrelated branch,
`worktree-agent-aeb09be0126ff19f6` @ `2bd04d45`, FR-45 work); `main` was at `a97db15c` (confirmed via
`git merge-base --is-ancestor`). Checked out `a97db15c` **detached** for the whole session so nothing
was mutated on a branch ref; every plant was reverted from a `cp` backup and `git status` / `git diff
--stat` confirmed clean before moving on. `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
for all `cargo` calls (a separate `target-clippy` / `target-doc` dir for the clippy/doc gates, per the
brief's own convention). The whole-workspace suite (3175 passed) was **not** re-run, per instruction.

## Commands run (representative; not exhaustive — every item's section below lists its own)

```
cargo nextest run --locked -p btctax-tui-edit -E 'test(tax_inputs_notables_notice_carries_the_slice_clause_only_when_the_slice_would_print)'
cargo nextest run --locked -p btctax-tui-edit -E 'test(tax_inputs_notables_status_is_visible_in_flow_render)'
cargo nextest run --locked -p btctax-forms --test year_record
cargo nextest run --locked -p btctax-cli --test extension
cargo run -q -p xtask -- cite-check
cargo nextest run --locked -p xtask -E 'test(the_forms_const_row_agrees_with_its_map_row) or test(re_excusing_an_archived_pair_reds_the_ratchet) or test(authority_coverage_may_only_improve)'
cargo nextest run --locked -p btctax-core -E 'test(an_unanswered_restriction_question_refuses_on_a_section_b_year_only) or test(a_declared_restriction_refuses_at_any_amount_not_just_over_5000)'
cargo doc -p btctax-cli --no-deps
cargo run -q -p xtask -- authority-manifest
cargo run -q -p xtask -- port-status 2025 2026-DRAFT
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
curl -sL -o … https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf   # re-fetch, sha256sum, wc -c
curl -sL -o … https://www.irs.gov/pub/irs-dft/f8995a--dft.pdf      # re-fetch, sha256sum, wc -c
```

**Summary: all six items delivered as briefed. Every kill named in the implementation report was
observed to go red exactly as claimed, and reverted clean. fmt clean, clippy clean (0 warnings),
`cite-check` OK, `authority-manifest` OK. No Critical or Important findings.**

## Per-item table

| item | verdict | the kill | what I planted | red text observed |
|---|---|---|---|---|
| 1 FR-63 slice clause | **as briefed** | `tax_inputs_notables_notice_carries_the_slice_clause_only_when_the_slice_would_print` | `if false && saved && …` around the clause | `assertion left == right failed: the notice is two rows here: […] left: 1 right: 2` — exact match |
| 1 (sibling) | **as briefed** | `tax_inputs_notables_status_is_visible_in_flow_render` (r1-M1, the "three-needle" kill) | none needed — ran green, confirming untouched | n/a |
| 1 (TY2024) | **as briefed** | code inspection | — | `CommitOutcome::NoTables` arm is unreachable for TY2024 (the one year with full-return tables), so the whole slice-clause block never runs there — confirmed structurally, not just asserted |
| 2 §7503 calendar | **as briefed** | `the_section_7503_shifter_walks_past_weekends_and_dc_legal_holidays`, `ty2017s_committed_return_due_is_derived_by_the_dc_holiday_calendar`, `the_dc_legal_holiday_predicate_names_the_right_days` (all 3 share the wound) | `fixed(April, 16)` (Emancipation Day) row deleted from `dc_legal_holidays_observed_from` | `ty2017…`: `left: 2018-04-16 right: 2018-04-17`; shifter test: `§7503(2018-04-15): Sun → Emancipation Day Mon 16 → Tue 17 left: 2018-04-16 right: 2018-04-17`; predicate test: `2018-04-16: Emancipation Day 2018, a Monday left: false right: true` — all exact matches, 3/11 tests failed as claimed |
| 2 (extension_due_date wiring) | **as briefed** | `a_ty2017_shaped_record_never_prints_04_15_and_its_june_date_is_the_15th` | none needed — ran green | code inspection: `extension_due_date`'s both branches call `section_7503_shift`; no standalone weekend-only fn survives (only the private `is_weekend` helper inside the shifter) |
| 2 (140-year sweep) | **as briefed** | `the_shifter_lands_on_the_first_free_day_for_every_date_in_140_years` | — | ran green in isolation |
| 3 cite-check pairs | **as briefed, and the brief's own "40→36" pin is corrected by measurement, matching the report's stated deviation** | `authority_coverage_may_only_improve` (the load-bearing ratchet); `re_excusing_an_archived_pair_reds_the_ratchet` | `("f4868", &[2025])` re-added to `AUTHORITY_NOT_YET_ARCHIVED` | CLI: `xtask cite-check: authority coverage is not accounted for — unaccounted: []; stale excuses: [f4868--2025]; phantom excuses: []` — exact match. `authority_coverage_may_only_improve` panic text matched verbatim. (`re_excusing_an_archived_pair_reds_the_ratchet` itself failed on its own internal premise-check instead of its target assertion, because my external plant broke an assumption its own plant/revert cycle makes — not a defect, see Note 1 below.) |
| 4 Section-B kill | **as briefed, all 4 conjuncts independently falsified** | `an_unanswered_restriction_question_refuses_on_a_section_b_year_only` | (a) `answer.is_none() && section_b` arm removed from `donation_restriction_gate`; (b) `&& section_b` dropped, arm fires unconditionally on `None`; (c) `claimed_noncash > FORM_8283_THRESHOLD &&` dropped at the call site; (d) `&& donated > QUALIFIED_APPRAISAL_THRESHOLD` dropped at the call site | (a) `a Section B year PRINTS 5a/5b/5c … left: None right: Some(DonationRestrictionsUnresolved)`; (b) `a Section A year never poses the question … left: Some(…) right: None`; (c) `the ceiling held line 12 at $480 … left: Some(…) right: None`; (d) `a Section A year never poses the question … left: Some(…) right: None` — all 4 exact matches |
| 5 orphaned doc block | **as briefed** | `cargo doc -p btctax-cli --no-deps` | none (a doc-move has no runtime kill; verified structurally) | doc comment sits directly above `pub fn slice_broker_refusal` at :2370; `slice_map_gate` (:2279) and `first_unresolved_map` (:2347) already have one-line docs. 7 admin.rs warnings at lines 308, 327, 402, 410, 510, 620, 1988 — none in the moved block's range 2262–2370. See Note 2 (total-count discrepancy, Minor). |
| 6 f8995a--2025 archive | **as briefed** | independent re-fetch + sha256 + byte count; masthead text; manifest; port-status row | none (evidence archive; no code kill applies) — instead independently reproduced the artifact | re-fetched PDF: sha256 `3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa`, 117129 bytes — **exact match** to the committed note. Masthead/threshold text (`$197,300`/`$394,600` vs TY2024's `$191,950`/`$383,900`) confirmed in the extract. `authority-manifest`: OK, 139 entries, by-kind breakdown exact match. `port-status 2025 2026-DRAFT` reproduces `| f8995a | 111 | 3 | 0 | 0 | port |` **exactly**, matching the committed `TY2026_WORK_LIST.md:46` — but only after I placed the two gitignored source PDFs myself (see Note 3, not a defect) |

## Re-measured pins (all independently reconfirmed, not merely re-read from the report)

- `cite-check` at `a97db15c`: **5/36 archived** `[f1040s1a--2025, f1040v--2024, f1040v--2025, f4868--2024, f4868--2025]`; **31 excused**; **0 unaccounted**. At the *parent* commit `8eeb11d4` (checked out separately and re-run): **1/36 archived**; **35 excused**. So the real move is **35 → 31**, confirming the report's stated correction of the brief's stale "40 → 36" (which was the pre-S9 baseline).
- `AUTHORITY_NOT_YET_ARCHIVED` diff: exactly 2 `(form, years)` tuples removed (`f1040v` and `f4868`, each covering `[2024, 2025]`) = 4 pairs, 19 → 17 rows — confirmed via `git show ca3b3eb9` diff.
- Form-fixture page ranges: `f1040v` archived extract = 2 form-feed pages, `instr_pages = (1,2)`; `f4868` = 4 pages, `instr_pages = (1,4)` — both years, both forms. Since the IRS publishes no separate instructions booklet for either form, `instr_pages` spans the *whole* document in both cases, which is what the report's "the form IS its own instructions" claim requires. Confirmed for all 4 (form, year) pairs.
- Form-fixture *content*: `crates/btctax-core/src/tax/fixtures/{f1040v,f4868}_{2024,2025}_form.txt`, stripped of the generated header, are **byte-identical** to `design/forms/extract/{f1040v,f4868}--{2024,2025}.txt` (`diff` clean on all 4).
- §7503 dates, hand-derived independently via `date -d '<date>' +%A` and cross-checked against the shifter's stated output for every date in the brief: 2018-04-15 (Sun) → 2018-04-17 ✓; 2023-04-15 (Sat) → 2023-04-18 ✓ (Emancipation Day observed Mon the 17th also blocks); 2026-04-15 (Wed) → unchanged ✓; 2024-06-15 (Sat) → 2024-06-17 ✓; 2025-07-04 (Fri, the holiday itself) → 2025-07-07 (Mon) ✓; 2022-12-25 (Sun) → observed Mon the 26th (also blocked) → 2022-12-27 ✓. Also independently verified: 2013-01-20 (Sun, Inauguration Day) → observed 2013-01-21 (Mon) per the Sunday in-lieu rule; 2029-01-20 (Sat, Inauguration Day) → **no** in-lieu day, 1/19, 1/20, 1/22 all correctly non-holidays; 2021-12-25 (Sat, Christmas) → observed Fri the 24th; 2022-01-01 (Sat, New Year's) → observed **2021-12-31** (the year-boundary case); Memorial Day 2025 = last Monday in May = the 26th, not the 19th (third Monday) — every one matches the report's claimed rows exactly.
- `authority-manifest`: **139 entries** (was 138), by kind `form 56, guidance 22, instructions 30, publication 6, regulation 6, statute 19` — exact match.
- `f8995a--2025.pdf`: sha256 `3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa`, **117129 bytes** — independently re-fetched from `https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf` (HTTP 200) and matched exactly.
- `TY2026_WORK_LIST.md` numeric table row: `| f8995a | 111 | 3 | 0 | 0 | port |`, reproduced by running `port-status 2025 2026-DRAFT` with the two source PDFs placed — exact match to the committed table.
- Whole-suite delta claimed (3168 → 3175, **+7**): accounted for exactly by item's own new-test count — item 1 (+1), item 2 (+4: `the_section_7503_shifter_walks_past_weekends_and_dc_legal_holidays`, `ty2017s_committed_return_due_is_derived_by_the_dc_holiday_calendar`, `the_dc_legal_holiday_predicate_names_the_right_days`, `the_shifter_lands_on_the_first_free_day_for_every_date_in_140_years`), item 3 (+1: `re_excusing_an_archived_pair_reds_the_ratchet`), item 4 (+1: `an_unanswered_restriction_question_refuses_on_a_section_b_year_only`) = 7.
- `docs/man/btctax-extension.1`: exactly **one** line changed (the `--out-of-country` help text), matching the `cli.rs` doc-comment source verbatim.
- `FOLLOWUPS.md` / `design/FORM_AUTHORITY_TABLE_DESIGN.md` (committed in `a97db15c`, the docs-only commit atop `ca3b3eb9`): FR-63, and the FR-49/FR-62 residue items (4, 5, the §7503 holiday half, the 4 cite-check pairs) all marked ✅ CLOSED with descriptions matching the implementation report; `FORM_AUTHORITY_TABLE_DESIGN.md` pin text updated to "31 of 36 … was 36 of 37 when this was written" and "five rows" — both confirmed.

## Notes (not findings — recorded for the record)

**Note 1 (item 3).** `re_excusing_an_archived_pair_reds_the_ratchet` does its own internal plant/revert
against the *real* `AUTHORITY_NOT_YET_ARCHIVED` list at run time. When I planted externally (editing
the source list directly, to test the load-bearing `authority_coverage_may_only_improve` ratchet), that
test's own premise-check assertion ("nothing is currently both archived and excused") fired first,
since my plant had already made that untrue outside of the test's own control. This is expected
behavior for a self-contained test with its own fixture, not a defect — the ratchet's actual guarantee
(`authority_coverage_may_only_improve`) reds with the exact text the report cites, both as a unit test
and at the CLI.

**Note 2 (item 5).** The report claims "16 pre-existing rustdoc warnings" for `btctax-cli`; my
`cargo doc -p btctax-cli --no-deps` run measured **15** (`btctax-cli (lib doc) generated 15 warnings`).
The load-bearing claim — 7 of them originate in `admin.rs`, at lines 308, 327, 402, 410, 510, 620, 1988,
none inside the moved doc block's range (2262–2370) — is exact. The total-count pin (16 vs 15) is off
by one; **Minor**, does not affect the conformance question the item was scoped to.

**Note 3 (item 6).** My fresh worktree checkout has none of the ~30+ gitignored, evidence-only PDFs
under `design/forms/{year}/*.pdf` that a long-lived dev machine accumulates locally (by design — "keeps
the repo small"; committed bundled *template* PDFs under `crates/btctax-forms/forms/{year}/` are
present and unaffected). `port-status`, `form_delta::tests::the_committed_work_list_matches_form_delta_
at_head`, and `port_status_prints_the_committed_work_list` therefore show `NO PRIOR SIDE` for many
unrelated forms (e.g. `f1040s1`) and fail when run whole, in my environment. I re-fetched exactly the
two PDFs item 6's specific claim depends on (`f8995a--2025.pdf` — the new archive; `f8995a--2026-DRAFT.
pdf` — its known-hash draft, itself unchanged since the IRS reposts drafts in place and a hash mismatch
would have signalled a NEW draft, which it did not), placed them at their expected gitignored paths,
reran, confirmed the exact numeric row, then deleted both and verified `git status`/`git diff --stat`
clean. Not a defect in the commit — an artifact of a verifier worktree lacking the implementer's local
PDF cache. The specific claim (sha256, bytes, masthead year, geometry, manifest count, the exact
`| f8995a | 111 | 3 | 0 | 0 | port |` row) is independently confirmed regardless.

## New findings

None — no Critical, Important, Minor (beyond Note 2's pin-count nit), or Nit findings against the
commit's actual content. All six items were delivered as briefed; every kill named in the
implementation report was observed to go red on the exact plant described, with the exact cited red
text, and clean on revert.

Counts: C=0 I=0 M=0 N=1
