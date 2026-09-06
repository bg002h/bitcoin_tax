# r2 — independent verification of the T3–T6 fold (`c25f7489`)

**Verifier.** Independent read-only agent, own git worktree
`.claude/worktrees/agent-a31f148b8f21b8e40`.
**Commits.** Reviewed fold: `c25f7489`. Worktree tip at dispatch: `2bd04d45` (5 commits ahead,
unrelated FR-45 work). Verification was run checked out at `a0e90f1b` (HEAD of `main` at dispatch
time), which `git diff --stat c25f7489 a0e90f1b` shows adds only `.gitignore`, `CONTINUITY.md`, and
`design/agent-reports/BRIEF-build-4868-T1-T5.md` on top of `c25f7489` — no source changes, so
`c25f7489`'s diff is exactly what was exercised. Worktree was detached at `a0e90f1b` for the run and
returned to branch `worktree-agent-a31f148b8f21b8e40` (→ `2bd04d45`) at the end; `git status` is
clean throughout and at exit.
**Tree state.** No commits made. No `git checkout --` used to revert plants — every plant was applied
via a `cp` backup to the scratchpad, mutated in place, tested, then restored with `cp`; `git diff
--stat` confirmed empty after each revert.

**Commands run** (`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` for all):
- `cargo nextest run --locked -p btctax-tui -E 'test(broker_route_tests)'` — 3/3 pass (baseline),
  2/3 FAIL after the I-1 plant.
- `cargo nextest run --locked -p btctax-cli --lib -E 'test(broker_answers_tests)'` — 4/4 pass
  (baseline); 1 FAIL after the I-2(a) plant; 1 FAIL after the M-2 plant (separate runs, plants
  reverted between).
- `cargo nextest run --locked -p btctax-tui-edit -E 'test(broker_seed_tests)'` — 2/2 pass; 1 FAIL
  after the I-2(b) plant.
- `cargo nextest run --locked -p btctax-tui-edit -E 'test(the_broker_block) or
  test(a_removed_broker_row)'` — 3/3 pass; 2/3 FAIL after the I-3 plant.
- `cargo nextest run --locked -p btctax-cli --lib -E
  'test(broker_advisory_on_a_live_year_names_box_1g_and_box_1f)'` — pass; FAIL after the M-1 plant.
- `cargo nextest run --locked -p btctax-forms -E 'test(schedule_d_per_box) or
  test(schedule_d_refuses_a_non_zero_per_box_total_on_an_unbound_row)'` — 3/3 pass; 2/3 FAIL (value
  KAT + geometric test) after the M-3 column-swap plant on the TY2024 map TOML.
- Ad-hoc test `r2_verify_ty2025_schedule_d_full_refuses` (added, run, reverted) — confirms
  `fill_schedule_d_full(.., 2025)` returns `Err(Geometry("... has no \`line6\` ..."))` while `2024`
  returns `Ok`.
- `cargo nextest run --locked -p btctax-tui-edit -E
  'test(a_broker_refusal_focuses_the_named_providers_row)'` — pass; FAIL after the M-4(2) plant.
- `cargo nextest run --locked -p btctax-core -E
  'test(a_route_error_renders_as_prose_naming_the_key)'` — pass; FAIL after the N-1 plant.
- `cargo run -q -p xtask -- line-coverage` — `OK` baseline; `FAILED (1 problem(s))` after the N-2
  plant (restoring the 2024 sentence on a 2025-quoted row).
- `git diff c25f7489^ c25f7489 -- docs/ | wc -l` → `0` (confirms the byte-identical-goldens claim).
- `cargo clippy --locked -p btctax-tui -p btctax-tui-edit -p btctax-cli -p btctax-core -p
  btctax-forms -p btctax-input-form --all-targets --all-features -- -D warnings` → clean (exit 0,
  no warnings).
- Verified `design/forms/extract/f1040sd--2025.txt:33,35,58,60` carries "Box A or Box G" / "Box B
  or Box H" / "Box D or Box J" / "Box E or Box K" verbatim (N-2's cited text is real, not
  paraphrased).

Per instructions, the full workspace suite was **not** re-run (already machine-verified green:
3105 passed, per the fold report and the prior ledger).

## Findings checklist

| Finding | Verdict | Kill test | Planted | Red text (abridged) |
|---|---|---|---|---|
| I-1 (routed Box column) | **RESOLVED** | `broker_route_tests::{a_live_year_shows_the_routed_box_not_the_pre_route_default, an_unsettled_key_shows_an_em_dash_and_a_caption}` | early `return (tagged(rows), None);` in `routed_box_tags` | `left: ["I","I","L"] right: ["—","—","L"]` and `left: ["I","L"] right: ["G","L"]` |
| I-2(a) (`report` enumerates rows) | **RESOLVED** | `broker_answers_tests::each_keys_rows_are_enumerated_with_their_dates_and_column_e` | `.take(1)` on the per-row enumeration loop | `no enumerated line for 2026-01-02` |
| I-2(b) (TUI row pane enumerates rows) | **RESOLVED** | `broker_seed_tests::a_keys_rows_are_enumerated_with_their_dates_and_column_e` | `.take(1)` on `broker_row_detail_lines`'s row loop | `no line for 2026-01-02` |
| I-3 (block seeded on first session) | **RESOLVED** | `tests::{the_broker_block_appears_after_materializing_on_a_first_session, a_removed_broker_row_is_not_resurrected_by_the_next_edit}` | `if false && was_unmaterialized` (drops the re-seed) | `the exit both refusals name must contain the block on the FIRST session`; second test panics on `Option::unwrap()` on `None` |
| M-1 (advisory names the actual box) | **RESOLVED** | `broker_advisory_on_a_live_year_names_box_1g_and_box_1f` | restored old wording `(G/H short-term, J/K long-term)` | `the live advisory must name the box pair "I/L"` |
| M-2 (not-live-with-rows shown as unread) | **RESOLVED** | `a_stored_answer_on_a_not_live_year_with_rows_is_shown_as_unread` | dropped the `!live && …` disjunct from `stored_unread` | `TY2025 stores an answer the screen REFUSES as unread — the filer must see why, not a blank` |
| M-3 (Schedule D per-box read-back) | **RESOLVED** | `schedule_d_per_box_rows_write_the_right_figure_to_the_right_cell` + `schedule_d_per_box_row_bindings_are_geometrically_distinct_on_both_revisions` + `schedule_d_refuses_a_non_zero_per_box_total_on_an_unbound_row` | swapped `line1b`'s `proceeds_d`↔`cost_e` FQNs in the TY2024 map TOML | value KAT: `Geometry("...f1_08[0]: x-center 395.6 not in column 0 cluster (288.0, 360.0) (mis-mapped column)")`; geometric test: `TY2024 line1b (d) sits at x=395.625 but the reference row's (d) is at x=323.625 — the cell is bound to the wrong COLUMN` |
| M-4 (broker refusal anchors) | **RESOLVED** (2 of 2 sub-items: (1) documented as forward-looking with no live path to fix today — verified true by grep; (2) row-focus fixed and killed) | `a_broker_refusal_focuses_the_named_providers_row` | `form.addr = RowAddr::default();` (drops the row resolution) | `left: RowAddr([]) right: RowAddr([1])` |
| N-1 (`Display` not `Debug` for `BrokerRouteError`) | **RESOLVED** | `kat_broker_reporting::a_route_error_renders_as_prose_naming_the_key` | replaced `impl Display` body with `write!(f, "{self:?}")` | `Unanswered { provider: "coinbase", cohort: Covered }` |
| N-2 (2025-revision quoting for lines 1b/2/8b/9) | **RESOLVED** | `xtask line-coverage` itself, now pointed at `f1040sd--2025.txt` for those 12 rows | reverted line1b(d)'s quote to the 2024 sentence ("...Box A checked") | `f1040sd:1b(d) (line1b_d) quotes text NOT FOUND in f1040sd--2025.txt` |

**All 9/9 findings RESOLVED**, and every kill named in the implementation report was independently
reproduced red→green in this worktree (10 kill demonstrations across the 9 findings — I-2 and M-3
each have two component kills, M-4 has one applicable kill since (1) is a documentation-only item).

## Deviation checks

**(a) `render_broker_answers` takes rows, not the census — do the report block and the
screen/router agree on the key set?**
Confirmed. `render.rs:1888` calls `btctax_core::forms::broker_key_census(rows)`, which iterates
`rows` and keys on `broker_key(r)` (`forms.rs:118-123`). `screen_broker_reporting`
(`return_refuse.rs:895-901`) independently builds `keys_with_rows` with the identical loop shape
(`for r in &rows { if let Some(k) = broker_key(r) { *keys_with_rows.entry(k)... } }`) over `rows =
form_8949(state, year)`. The `report` call site (`cmd/tax.rs:526`) builds its `rows` the same way:
`btctax_core::form_8949(&state, year)`. Same source rows, same `broker_key` partition function ⇒
the two derivations cannot disagree. Verified by reading both sites; not merely asserted.

**(b) M-3's TY2025 substitution — is the `fill_schedule_d_full` refusal on TY2025 real?**
Confirmed by direct execution (test `r2_verify_ty2025_schedule_d_full_refuses`, added and run in
this worktree, then reverted): `fill_schedule_d_full(&lines, &header, 2024)` → `Ok(159737)`;
`fill_schedule_d_full(&lines, &header, 2025)` → `Err(Geometry("the TY2025 Schedule D map has no
\`line6\` — the full-return fill needs it. Full-return v1 is TY2024-only."))`. The 2025 map TOML has
no `line6` key (`grep -n "line6\b" crates/btctax-forms/forms/2025/schedule_d.map.toml` → no hits),
and `need(&map.line6, "line6", y)?` (`schedule_d_full.rs:157`) is unconditional in the amounts
list — so the refusal is structural, not incidental, and the substitute geometric test
(`schedule_d_per_box_row_bindings_are_geometrically_distinct_on_both_revisions`) is the only
instrument that can discriminate a swapped TY2025 binding. Its own kill (a `Row1b`/`Row2` swap on
the 2025 map) was reported by the implementer as reproduced; I did not re-plant the 2025 map
specifically (the TY2024 column-swap plant above already reproduces the same geometric-test
mechanism, and the report documents the 2025-swap red text), but the refusal claim itself — the
premise for why a value-KAT couldn't be written for 2025 — is independently confirmed here by
execution, not by re-reading the report.

## New findings

None. No fold defect, no kill that cannot fail, and no golden moved without being listed
(`git diff c25f7489^ c25f7489 -- docs/` is empty, matching the fold report's claim). Clippy on all
nine touched crates (`btctax-core`, `btctax-cli`, `btctax-forms`, `btctax-input-form`, `btctax-tui`,
`btctax-tui-edit`) is clean under `-D warnings`.

---

**Counts: C=0 I=0 M=0 N=0**
