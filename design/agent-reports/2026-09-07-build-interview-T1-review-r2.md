# Re-verification — the T1 seam review's fold at `d49de0c7`

Independent, read-only re-verifier. Own worktree (`agent-a339e291891bacc9b`), checked out to
`d49de0c7` detached (the worktree's own branch tip, `2bd04d45`, was 5 commits behind main and had to
be advanced to reach the commit under review — `d49de0c7` is `main`'s HEAD, checked out in the sibling
worktree `/scratch/code/bitcoin_tax`). Nothing committed here; every plant below was reverted via `cp`
from a pre-plant backup (never `git checkout --`) and confirmed with `git diff --stat` / `git status
--porcelain` returning empty. This is the ONE re-verification the owner's S6 rule allows.

Reviewed: does `d49de0c7` resolve each of C1, I1, M1–M4, N1–N3 from
`design/agent-reports/2026-09-07-build-interview-T1-review.md` (findings from line 194), and does each
kill it adds go red when the guarantee is removed. Implementer's account read:
`design/agent-reports/2026-09-07-build-interview-T1-fold-implementation.md`.

## Commands run

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo build --locked --workspace --all-targets                                            ok, clean
cargo nextest run --locked -p btctax-core -p btctax-input-form -p btctax-cli \
  -p btctax-tui-edit -p xtask -E 'not test(form_delta) and not test(harness_check)'        2507 passed, 27 skipped (baseline, re-run identically after all plants)
cargo fmt --all -- --check                                                                 clean
cargo clippy --locked -p btctax-core -p btctax-input-form -p btctax-cli -p btctax-tui-edit \
  -p xtask --all-targets --all-features -- -D warnings                                     clean
```

Per-finding kill runs are listed in the table below; each was run green → planted → red →
reverted → confirmed clean, individually.

**Environment note (not a fold defect).** Two modules fail in this worktree unrelated to the diff:
`xtask::form_delta::*` (6 tests — `no PDF found for f6251--2026-DRAFT`; the gitignored source PDF for
that draft form isn't present in this worktree, and `form_delta.rs` is not among the 9 files this fold
touched) and `xtask::harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
(1 test — depends on the binary landing at the *default* `target/`, and the task's own instructions put
`CARGO_TARGET_DIR` at `target-review`). Both excluded from the baseline filter above; neither touches
any file in this fold's diff (`git show d49de0c7 --stat`).

## Verdict table

| Finding | Verdict | Kill run | Plant | Red text (abridged) |
|---|---|---|---|---|
| **C1** — `income import` forges/destroys `answer_log` | **RESOLVED** | `an_imported_toml_cannot_mint_an_answer_record_or_a_history_entry`, `a_re_import_keeps_every_answer_record_already_on_the_row`, `the_scrubbed_toml_round_trips_back_through_import` | Removed both `ri.answer_log.clear()`/`.answer_log_history.clear()` lines and the two `ri.answer_log = existing…` re-attach lines in `tax.rs::import_return_inputs` | All 3 RED. Forgery: `left: Some(AnswerRecord { answered_on: 1999-01-01, … state: Given })  right: None`. Destruction: `left: {} right: {…14 records…}`. Round-trip: `got 2 record(s) and 1 history entr(ies)` |
| **I1** — `record_answer` is not the one writer of R10.3 half two | **RESOLVED** | `provenance::changing_the_words_re_asks_and_the_re_answer_itself_historises_the_old_record`, `tax_report::re_answering_at_the_keyboard_moves_the_stale_record_into_history_by_itself` | Removed the `if let Some(superseded) = ri.answer_log.get(&key) { if superseded.prompt_hash != hash { … push to history } }` block from `record_answer` | Both RED. `left: 0 right: 1` (history empty when it must hold 1); `left: [] right: [(2026-09-01, "7e2ad…")]` (September record silently destroyed) |
| **I1 deviation** — read-boundary sweep would relabel `WordingChanged`→`NeverAsked`, guard pins the refusal | **CONFIRMED TRUE** | `tax_report::a_record_whose_words_changed_still_reads_as_wording_changed_after_a_load` | Re-implemented the deleted `supersede_stale_prompts` sweep inline in `return_inputs.rs::row_to_inputs` (the function itself is gone from `provenance.rs` — confirmed by grep) | RED, exact match to the report's own measurement: `left: NeverAsked right: WordingChanged` |
| supersede_stale_prompts removed from production | **CONFIRMED** | grep | — | Only 2 hits repo-wide, both in comments/doc-prose referencing the deleted name (`tax_report.rs:3219`, `provenance.rs:881`); zero calls |
| **M1** — `payer_tin: String` can't express "no TIN printed" | **RESOLVED (as designed: carried, not fixed)** | — | — | `payer_tin` confirmed still `String` at all 4 sites in `return_inputs.rs`; report explicitly deferred this to T5, no code change claimed or expected |
| **M2** — generated fixture had no test pinning it to its emitter | **RESOLVED** | `fullreturn_fixture_matches_its_emitter`, `fullreturn_fixture_is_the_kitchen_sink_oracle` | (a) one-byte value edit (`medical = "2000"` → `"2001"`) — reds **both** tests, since the value changes the parsed struct too. (b) comment-only insertion (no value change) — the **discriminating** case | (a) both FAIL, oracle test's own diff shows `medical: 2001` vs `2000`. (b) `fullreturn_fixture_is_the_kitchen_sink_oracle` **PASSES** (1 passed), `fullreturn_fixture_matches_its_emitter` **FAILS** — exactly the gap the finding named, now closed |
| **M3** — corrupted doc comments (hardcoded date instead of the `now`/`form.now` seam) | **RESOLVED** | — (doc-only) | — | Diff confirmed: `tax_inputs.rs:11` → `apply(SetField)`, `:262` → `apply(&mut form.working, Edit::…, now)`; literal `date!(2026-09-01)` removed from both prose lines |
| **M4** — money-detector's stated "honest limit" was wrong | **RESOLVED** | — (doc-only; underlying KAT unchanged, already exercised by the original review's Seam 1 plant) | — | Diff confirmed: paragraph replaced with the two real blind spots (`skip_serializing_if` fields; empty-`Vec` element leaves) and the correction that `Option<Usd> = None` **is** detected |
| **N1** — shrink floor `>50` vs measured 106 | **RESOLVED** | probe (`assert!(money.len() >= 100000, "MEASURED COUNT: {}", …)`) | Raised the assert threshold far past any real count, to force the panic message to print the actual count | `MEASURED COUNT (verification probe): 106` — the fixture doc comment's "106 leaves measured" and the landed `>= 100` floor are both exact. (No separate red/green kill cycle: this is a threshold-value correction, not a new guarded behavior — verified by direct measurement instead) |
| **N2** — overstated "no branch can be added that asks without recording" | **RESOLVED** | — (doc-only) | — | Diff confirmed: comment now states the declaration records in its own branch and that the `if let Ask::Skippable` site is not exhaustive, crediting the real net (the outer exhaustive `match`) |
| **N3** — `Declined` written, read by nothing in production, dated to T3 | **RESOLVED** | — (doc-only) | — | Diff confirmed: doc comment added to `AnswerState` naming T3/`interview_state()`/T12 as the eventual reader, same "stored value with no reader" caveat already used for `prompt_hash` |

**8/8 RESOLVED.** (C1, I1 — including its deviation — M1–M4, N1–N3; nine kills total, all reproduced
red-on-plant, clean-on-revert, matching the fold report's claimed red text.)

## Deviation check, in detail

Claim: a sweep at `return_inputs::row_to_inputs` (the one read boundary) would turn a stale
(`WordingChanged`) record into `NeverAsked` before any reader — including `return_refuse`'s
`screen_inputs` — ever sees it, because `answer_status` reports `NeverAsked` once the record is out of
`answer_log`, and `return_refuse.rs`'s "an absent record is not a mismatch" then treats it as never
having been asked at all. I reproduced this by resurrecting the sweep's logic inline (the actual
`supersede_stale_prompts` function no longer exists in the crate — confirmed by grep, 0 hits outside
comments) inside `row_to_inputs`, and running the fold's own guard test against it. It reds with the
exact left/right values the fold report quotes:

```
left: NeverAsked   right: WordingChanged
```

The guard (`a_record_whose_words_changed_still_reads_as_wording_changed_after_a_load`) does pin the
decision — it is a real, currently-passing test that fails the moment the refused option is re-added,
which is what a "permanent guard" needs to mean.

## New findings

None. No fold defect, no kill that fails to discriminate, and no golden moved that wasn't listed in the
fold report's own diff surface (`git status --porcelain` was empty both before I started and after every
plant was reverted; the fixture's 15+/10− regeneration diff matches the fold report's line-by-line
listing exactly, re-diffed here).

Counts: C=0 I=0 M=0 N=0
