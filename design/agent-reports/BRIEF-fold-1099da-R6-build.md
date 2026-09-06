# Brief — fold the R6 build's seam review (1C/2I/4M/1N)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee you add needs a kill seen red once (plant, observe, revert via a
`cp` backup); the report says how, with the red text.

Read first: `design/agent-reports/2026-09-06-build-1099da-R6-review.md` (findings from line 146) and
its ledger `…-review-VERIFICATION.md` (every checkable claim HOLDS — fold, do not re-verify). The spec
is `design/SPEC_1099da_broker_reporting.md` R6; the build is `fb5e7fc3` and its report
`…build-1099da-R6-implementation.md`.

## Fold, item by item (the review's "Minimal change" is the default; deviate only with a reason)
- **C-1** `crates/btctax-cli/src/cmd/admin.rs` (~806-830): hoist the Form 8283 restriction row OUT of
  the `if files_from_answers` block so it runs whenever a working return exists (it reads `working`,
  not the answers), before any byte, on both arms. Fix the `printed.rs:201` comment to state the
  invariant that now holds and where it is enforced. Kill: the 2×2 sweep of the existing restriction
  test over {draft, committed} × {answers, no answers} on a params-less year with a restricted
  donation — every cell refuses with no `form_8283.pdf` and no `out_dir`; the (committed, no answers)
  cell is the one that reds on the build as it stands (the review's probe, on TY2025's real regime).
- **M-3** (rides with C-1): the full return's SECOND half of that gate — `donations_had_restrictions
  == None` when the year files a Section B 8283 (`claimed_noncash > $500 && donated > $5,000`, the
  rule at `crates/btctax-core/src/tax/return_1040.rs:2667-2695`) — applies on the slice too; share
  the predicate with the full return rather than copying it. Kill: an unanswered restriction on a
  Section-B-sized donation refuses; a Section-A-sized one prints.
- **I-1** the slice promise must be conditioned on the year's templates: add a templates term to
  `cmd/tax.rs::stored_answers_reach_the_slice` (a `slice_can_print(year)` helper probing
  `Form8949Map::for_year` and `ScheduleDMap::for_year`), and make `uncomputable_sentence` /
  `import_note` take the same helper instead of asserting unconditionally — **M-4** rides with it
  (the clause also needs the answers-stored term: no answers → no "from the stored answers"). Kills:
  TY2026 (no templates) + answers → the sentence does NOT promise the slice; TY2025 + answers → it does;
  TY2025 + no answers → it does not.
- **I-2** `crates/btctax-forms/tests/kats.rs`: the pre-2025 T8 kill must use pre-2025 boxes — build its
  `by_box` from C/F rows and assert line 3 and line 10 each carry their group (the review says this
  single edit reds both plants); `not_reported_by_box` may keep I/L.
- **M-1** `schedule_d.rs::fill_schedule_d_totals`: assert the six groups PARTITION `by_box` (every
  key in `by_box` is in exactly one group; A/B/D/E must be handled — refuse or map them to 1b/2/8b/9
  as the form's text says: 1b = A|G, 2 = B|H, 8b = D|J, 9 = E|K), never dropped silently. Kill: a
  `by_box` carrying box A reds an implementation that drops it.
- **M-2** run `slice_map_gate` on arm (3) too (the "write nothing" guarantee for a partially ported
  year, both arms). Kill: a partially ported fixture year reached on arm (3) → refusal, no `out_dir`.
- **N-1** `--forms` narrowing on arm (2): if Schedule D is selected without Form 8949, either refuse
  ("Schedule D's per-box lines cite Form 8949 page-sets the packet does not contain") or probe both
  maps — pick the refusal (fail closed) and say so.

## Constraints
Do not touch the spec or any `design/agent-reports/*.md` other than your report. Goldens and man pages
move only as a direct consequence (list any diff lines). Pinned counts: old → new with cause.

## Report — your FINAL action
`design/agent-reports/2026-09-06-build-1099da-R6-fold-implementation.md`: per finding the change
(file:line), the kill and how it was seen red, deviations with reasons, golden diffs, the exact
nextest commands with summary lines. Return ONLY a 3-line summary (what landed, suite results for the
crates you touched, the report path).
