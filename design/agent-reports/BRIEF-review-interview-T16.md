# Brief — seam review of interview build T16 (Form 8889, transcribed)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T16 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- line-coverage` / `box-census` / `census-join` / `authority-manifest`. The archived PDFs
are gitignored — re-fetch from each note's URL when you need bytes; the Python stack is
`.venv/bin/python`. **Environment, not findings:** six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Is Form 8889 TRANSCRIBED — every printed line present with the instruction's own text and the
arithmetic the form prescribes, each input line collected from the document or the filer's records
and each computed line computed from the lines the form names — and does every figure it produces
reach exactly the 1040 line the form says (Schedule 1 lines 8f and 13, Schedule 2 lines 17c and
17d), witnessed by BOTH oracles? Not a fresh audit of T1–T6; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T16 build report `2026-09-07-build-interview-T16-implementation.md` lists the archived
documents, the line table, the box censuses, the params figures, the oracle inputs, kills and
deviations; the controller has confirmed the suite line it states, `authority-manifest` OK,
`box-census` OK, `line-coverage` and `census-join` lines. The reach lines (`f1040s1--2024.txt:30,62`;
`f1040s2--2024.txt:84,87`; the 2025 twins) and the archive availability are in
`BRIEF-build-interview-T16.md`. The form and its instructions are the authority: `design/forms/
extract/f8889--<year>.txt`, `i8889--<year>.txt`; the HSA information returns `f1099sa`, `f5498sa`,
`i1099sa`.

## Seams
1. **Transcription fidelity, line by line.** Open the archived `f8889--2024.txt` and walk every
   numbered line of Parts I–III against the struct: is each line present, named for the line, its
   doc comment the instruction's sentence verbatim, and is each "enter the smaller of" / "subtract
   line N from line M" / "multiply by" implemented exactly as printed (the 2026-07-27 AMT lesson: a
   one-character line reference typed from the image taxed a slice twice)? Plant: change one line
   reference in a computed line → which test reds? `line-coverage` must hold every line (plant a
   quote drift → red). Compare the 2025 edition: any line that moved, split or was added, and does
   the struct follow it per revision?
2. **Reach.** A fixture with a deductible contribution moves Schedule 1 line 13 by exactly that
   amount and Form 1040 line 10 with it; a distribution not for qualified expenses moves Schedule 1
   line 8f AND Schedule 2 line 17c by the form's amounts; the last-month-rule failure moves 8f and
   17d; nothing else moves (read the printed chains, not the report). The `covered_by` census
   entries for those four lines retired and `census-join`'s counts fell by exactly four per year.
3. **The documents and the census.** 1099-SA and 5498-SA `[boxes]` censuses complete per edition
   with `Collected(FieldId)` joined (T5's instrument — plant a caption drift and a wrong-section
   field → red); the W-2 code-W amount is READ from the W-2 rows, never re-asked, and a code-W
   amount with `hsa_activity = Some(false)` refuses the contradiction; the census rows `sa_1099` /
   `sa_5498` obey the three rules; a distribution with no 1099-SA row and `Some(true)` refuses.
4. **The year's figures.** The §223(b) contribution limits (self-only / family, the 55+ addition)
   for TY2024 and TY2025 in `FullReturnParams` match the instruction's line-3 table (cite by line)
   and the Rev. Proc.; the months-of-eligibility proration follows the form's line-3 worksheet; a
   contribution over the limit produces what the form prescribes (excess → the additional tax path
   or the refusal naming it), never a silent clamp.
5. **The oracles.** Both oracles take the HSA deduction (Tax-Calculator `e03290`; OTS's line);
   the golden HSA household reconciles on both; a Form 8889 line neither oracle computes is excused
   by MECHANISM with its size, never by name; `project_to_golden` / the invisible list updated (or
   the T11 predecessor note present).
6. **Refusals and the packet.** Archer / Medicare Advantage MSA (Form 8853) refuses naming the
   form; any worksheet the build does not carry refuses naming it; the packet places Form 8889 at
   its attachment sequence number; `YEAR.toml` lists it; the TY2024 emitter output for a fixture
   WITHOUT an HSA is byte-identical to before (plant: the form emitted on a no-HSA return → red).

## Severity
A wrong printed figure, a line that reaches the wrong 1040 line, a computed line implemented
differently from the printed instruction, or a silent clamp is **Critical**; a missing line, a
census gap, an oracle taken singly, or a kill that does not red is **Important**. Secret-handling
defects are never Critical/Important. Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T16-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
