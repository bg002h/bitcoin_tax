# Brief — seam review of interview build T6 (the exchange seam)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T6 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- stop-list` / `census-join` / `line-coverage`. **Environment, not findings:** the archived
PDFs are gitignored, so six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Can the Digital Assets box be printed with a value the filer did not assert, or can the filer be
refused for a truthful answer? Concretely: (a) is the printed `da_yes`/`da_no` now the filer's
ANSWER and only the answer; (b) does a contradicted `No` refuse naming the FIRST qualifying event
by the instruction's own list (receipt as reward/award/payment, disposition — never a purchase,
never a self-transfer); (c) is an unwitnessed `Yes` accepted with the warning and never refused;
(d) does the Step 0 panel read the ledger and write nothing; (e) does the cross-check live in the
compute-dependent tier (never param-free, never at import). Not a fresh audit of T1–T5; not a spec
re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T6 build report `2026-09-07-build-interview-T6-implementation.md` lists its five-row table
outputs, panel rows, kills and deviations; the controller has confirmed the suite line it states
and that `stop-list`, `census-join` and `line-coverage` are clean/unmoved. Spec:
`design/SPEC_interview.md` r2 R9 (the mechanism and its kill), §7 row T6, R12, R15; spec 1099-DA
R1 (answered-ness lives in the key set) and R6; `legal/text/irs-guidance/Notice_2026-20.txt`
§4.02(2). Prior builds: `digital_asset_activity(state, year)` (`return_1040.rs:2468`, the
instruction's list in its doc comment); `screen_compute_dependent` (`:907`); the old printed-box
site `admin.rs:1122`; T3's `interview_state()`; T4's `screen_param_free` tier census KAT; T4b's
`opened_from`; T5's grouped `live_questions`.

## Seams
1. **The five-row table, re-driven.** Build the five ledgers yourself (activity ∧ `No`; activity ∧
   `Yes`; no activity ∧ `No`; no activity ∧ `Yes`; `Acquire`-only + linked self-transfers ∧ `No`)
   and run them through commit AND export: the refusal names the first qualifying event (date,
   venue, kind) — plant a ledger whose first event is a purchase and second a disposition and
   confirm the disposition is named; the warning text on the unwitnessed `Yes`; the printed box on
   each accepting cell. Then `None` blocks commit and is listed by `interview_state`.
2. **The predicate is the instruction's list, not "the ledger has events".** Read
   `digital_asset_activity`'s doc comment against `i1040gi--2025.txt:1346-1362` and its bullets;
   plant an income-recognition event (a reward) with no disposal → activity; a purchase only → no
   activity; a self-transfer pair → no activity. Is the predicate's year boundary the event's
   date in the tax year?
3. **Tier placement.** The cross-check is in `screen_compute_dependent` (the ledger-seeing tier),
   not in `screen_param_free` and not at `income import`; the T4 tier census KAT lists it correctly
   (plant: move it into the param-free body → the KAT reds). Does `income import` of a TOML with
   `digital_asset_activity = false` on a ledger with a disposal import (it must) and refuse at
   commit (it must)?
4. **The panel writes nothing.** Trace the Step 0 panel function: every input is a read of the
   session/ledger/answers; no `save_draft`, no `return_inputs::set`, no `record_answer`, no
   `BrokerReporting` key insertion (T4b I-3's class). Plant a write → which test reds? The
   standing-order rows: a 2026 Coinbase disposal with no scoped `MethodElection` warns with the
   §4.02(2) consequence; one effective the day before silences it; the day OF the disposal —
   which way, and is it the Notice's rule? A venue in the year's 8949 rows with no
   `BrokerReporting` answer is NAMED.
5. **The printed box.** `admin.rs`'s old `da_yes = !rows.is_empty() || …` is gone; the emitter
   prints the answer; a `None` never reaches the emitter (the commit gate); the R6 slice path
   (`export-irs-pdf` on a params-less year from stored answers) prints the answer too — or refuses
   on `None` — say which and whether it is what R6 requires.
6. **No `reconcile` question in a registry.** `xtask stop-list` still clean; the panel's prompts
   are not `FormQuestion`s; `stop-list` extended if the prompt set grew; T4b's opener seeds
   `digital_asset_activity = None`.

## Severity
A box printed from anything but the answer, a truthful `Yes` refused, a refusal that names the
wrong event, a panel that writes, or a cross-check in the wrong tier is **Critical** or
**Important**; a kill that does not red is **Important**. Secret-handling defects are never
Critical/Important. Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T6-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
