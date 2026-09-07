# Brief — fold the interview T4b seam review (C-1, I-1, I-2, I-3, M-1–M-4, N-1, N-2)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert
a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once; quote the red.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T4b-review.md` (read whole) and the
ledger `2026-09-07-build-interview-T4b-review-VERIFICATION.md` — every claim is machine-verified;
its **Disposition — the shape** section fixes the design: *the opener may carry an identity only
where this year has a surface to answer it*, made structural by one provenance leaf
`ReturnInputs.opened_from: Option<i32>`. Your own T4b report is what exists. Spec: R10 part 4, §7
T4b; `Durability` (`questions.rs:28-35`); T1's `record_answer` (the only writer of `answer_log`);
T3's classifier discipline (no `..`, no `_`); T4's draft rules.

## The fold
1. **`opened_from: Option<i32>`** on `ReturnInputs` (`#[serde(default)]`), set by `seed` to N;
   classified as provenance (its own class or the `answer_log`-style exemption with a reason — never
   a filer answer); `LEAF_SOURCE` entry; `income import` round-trips it; the coverage KAT exempts it
   by name with its reason.
2. **C-1 — shown, not pre-filled.** `seed` leaves `date_of_birth: None` on taxpayer and spouse.
   `income answer`'s `Date` prompt and the TUI's DOB field, when `opened_from` is `Some(N)`, read
   year N's committed row and DISPLAY the prior date as a hint in the prompt text (*"TY(N)'s return
   gave 1980-05-05 — type it to confirm, Enter to skip"*); typing it is a fresh `SetField` → a fresh
   record; a bare Enter records `Declined` and `is_aged` forgoes. Kills: the review's probe (open,
   bare-Enter the DOB → record absent or `Declined`, never `Given`); the hint appears only on an
   opened year; the seed's `date_of_birth` is `None` (walk with `leaf_walk`, not by name).
3. **I-1 — filing status has a surface, and what crossed is named.** A class-(A) `FormQuestion`
   `FilingStatusConfirmed` on `ReturnInputs` (`filing_status_confirmed: Option<bool>`,
   `#[serde(default)]`), prompt *"Your TY(N) return filed as <status>. Is <status> your filing status
   for TY(N+1)? (Marital status is determined on the last day of the tax year — Form 1040
   instructions, Filing Status.)"* — the status and years rendered from the return; live iff
   `opened_from.is_some()`; `None` blocks (the census pattern); `No` refuses naming where to change
   the status (the TUI header / the TOML) and the question is re-asked on the new status (its
   `prompt_hash` changes with the status text, so T1's re-ask rule does this for free — assert it).
   Classifier row; `interview_state` lists it. Correct `classifier.rs:146-151`'s `SerdeRequired`
   text for `filing_status` to state the real ground on an opened year (this question). `Opened`
   gains `carried_identity: Vec<String>` (filing status, taxpayer/spouse name, SSN, mailing address —
   the DOB is now shown, not carried) and `render`, `cli.rs`'s `--help`, the man page and the TUI
   offer (`draw_edit.rs:2886-2900`) all say what crossed and that everything else is blank; the
   existing test that pinned `seed.filing_status == Single` on a Single fixture is rewritten on a
   non-Single fixture so it distinguishes carried from defaulted.
4. **I-2 — dependents are not seeded.** `seed` carries no `Dependent` row; `identities_of` names each
   prior dependent in the prompt from `prior` (name and relationship; never the SSN — keep the
   no-SSN-printed kill); FR-70 (T7) is where the row returns with a gate to answer it. Kill: the
   seed's `dependents` is empty; the prompt names them.
5. **I-3 — no venue key.** Remove the `broker_reporting` arm from `seed`; the venue prompt already
   reads `prior`. Kill: `draft.broker_reporting.0.is_empty()` after opening from a year with a venue;
   `resolve.rs:184-186`'s `answers_stored` is false on the seed; the R6 M-4 sentence is absent.
   Rewrite the test that pinned the seeded key as intended.
6. **M-1** — the opener's report says year N's `--write-carryover` is now unnecessary and will
   refuse; `coherence_clear`'s "discarded" note is emitted only after the write has saved (or reads
   *"will be discarded when this write saves"*). Kill: the review's probe prints no false discard.
7. **M-2** — `payer_of`'s match is exhaustive (no `_`); **M-3** — the retention kill compares year
   N's committed row, draft row and parked flag; **M-4** — the census-delegation kill runs once per
   kind in its own fixture so a non-first offender reds; **N-1** — `render` uses `format!`; **N-2** —
   `--from` is range-checked (`checked_add`, and within the years the tree knows) before any
   arithmetic.

## Constraints
- `record_answer` stays the only writer of `answer_log`; no default answers; every new prompt is the
  form's or the instructions' words.
- `report --write-carryover` and the R6 slice path unchanged (their tests green).
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
APPEND a section `## Fold (seam review C-1, I-1, I-2, I-3, M-1–M-4, N-1, N-2)` to
`design/agent-reports/2026-09-07-build-interview-T4b-implementation.md`: per item what changed,
the `opened_from` classification, the new question's prompt as committed, every kill with its red
text, every pinned number moved, suite lines per crate. Return only a 4-line summary.
