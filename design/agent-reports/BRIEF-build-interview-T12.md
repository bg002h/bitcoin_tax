# Brief — interview build T12: the panel in the TUI, the commit modal's lists, the manifest block, the docs (R12 render)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing; `make docs` clean. Every guarantee lands with a kill seen red once; quote the red.
Every pinned number moved: old → new with cause. `/tmp` is a 32 GB tmpfs. Process in force (owner
S6): ONE seam review and ONE re-verification after you.

★ **Standing rules:** the panel is DERIVED from the registries through T3's `interview_state()` —
render it, never recompute it; no progress bar, no persisted "remaining" (R15); class (B) is never
flattened into class (A); a `Declined` benefit is still forgone and listed *(declined)*; a document
row with no `transcribed_on` is named *transcribed without a date*, never omitted; a blank is
normal.

## The contract
`design/SPEC_interview.md` r2 **R12** (the seven states and how each is listed; no progress bar;
the TUI pane; `income answer` prints it first and last — done in T3; the commit modal prints the
forgoing AND refusing lists — J-12/J-17/J-32), **§4.1** (the TUI flow: one journey with the Step-0
panel from T6 at the entry, the census first, the document sections, the gates, the answer panel,
commit), **§4.4** (`report` prints the census, the panel, the home-sale decision, row (7) per
dependent, the `LEAF_SOURCE` provenance of every collected figure with payer TIN masked; the
packet's `manifest.txt` "COMPLETE BY HAND" block gains the forgoing list with `(declined)` marks and
the undated document rows named — J-15), **§7 row T12** (its kills), **R15**'s grep-KATs (`xtask
stop-list` — extend if prompts/fields grew), `docs/` conventions (`binary-docs-infra`: man pages are
generated from clap doc-comments via `make docs`; `LIMITATIONS.md` is the filer-facing list). Build
AS WRITTEN; the tree's real names win; deviations recorded.

## Settled facts (controller-measured at `1f3cc137`; re-measure at dispatch)
- `interview_state()` / `interview_state_with_params()` (`crates/btctax-core/src/tax/interview_state.rs`,
  T3) with `blocking / forgoing / refusing / waiting / answered / not_live`, `Declined` marked, the
  hash-mismatch reason, the forgo size when params exist; `income answer` prints it first and last
  (T3/T4); the TUI's discard-only and year-gate screens (T4: `draw_edit.rs`, the entry gate 1–3
  lines); T6's Step-0 panel function and pane; T4b's `Opened::render`; T7/T8's dependents gates and
  the line-19 forgo size; T9's home-sale decision.
- The packet manifest: `crates/btctax-cli/src/cmd/admin.rs` (grep `COMPLETE BY HAND`); the TUI
  commit modal (`crates/btctax-tui-edit/src/main.rs`, grep `commit` modal / `forgoing`); T5's
  *transcribed without a date* manifest row.
- `docs/man/btctax-income-answer.1` and siblings are generated (`make docs`); `docs/LIMITATIONS.md`
  exists (check the path); the TUI walkthrough goldens (`docs/examples-tui-walkthrough/j6/*`) move
  when a screen changes — regenerate via the Makefile target and explain each moved line.

## What T12 delivers
1. **The answer-panel pane in the TUI** — a pane rendering `interview_state()` (with params when
   the year has them): the blocking list (question, anchor, what it accounts for), the forgoing
   list (benefit, size where computable, *(declined)* marks), the refusing list (row, reason, exit),
   the waiting list (named with the package it waits on), and the counts; no progress bar; reachable
   from the entry and from any section; snapshot tests (N blocking; forgo sizes present with params,
   absent without).
2. **The commit modal** prints the forgoing AND refusing lists before the confirm (J-12, J-17, J-32);
   a commit with a non-empty `refusing` is refused by the existing gate (assert, do not add).
3. **`report`'s rendering** (§4.4): the document census (type → declared / rows), the answer panel,
   the home-sale decision and its answers, row (7) per dependent, the `LEAF_SOURCE` provenance of
   every collected figure (document with payer TIN masked / filer's records), undated rows named.
4. **The manifest block** — the "COMPLETE BY HAND" block gains the forgoing list with `(declined)`
   marks and the undated document rows; present on a fixture packet.
5. **Docs** — `make docs` regenerates the man pages for every `income` subcommand the interview
   added (`answer`, `import`, `open-next-year`, `project`, `clear` …) and the TUI walkthrough;
   `LIMITATIONS.md` gains the interview's stop list in the filer's words (§2.2's exits; line 19 a
   forgo; the home sale; the 1099-DA answers as keystrokes; no self-custody import).

## Kills (each seen red once)
Snapshots: a fixture with N unanswered live items renders N blocking rows in the pane and the modal;
a `Declined` skippable renders *(declined)* in forgoing and never in blocking; the forgo size present
with params and absent without; the refusing list names the row's exit; `make docs` clean and the
generated pages byte-stable on a second run; the manifest block present on a fixture packet with a
`(declined)` mark and an undated row; `report` prints the census and the provenance with the TIN
masked (plant an unmasked TIN → red); the walkthrough goldens regenerated with each moved line
explained; the R15 greps still clean.

## Constraints
- Render only; no new state, no new writer, no new question. Nothing changes what is filed.
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T12-implementation.md`: per numbered item what
landed, the pane and modal snapshots on the fixture, the manifest block as printed, the docs
regenerated, every deviation, every kill with its red text, every pinned number moved, suite lines
per crate. Return only a 4-line summary plus the path.
