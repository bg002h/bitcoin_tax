# Brief — fold the T8 seam review (0C / 3I / 2M / 1N)

You are folding an independent seam review into the **main working tree** of `/scratch/code/bitcoin_tax`,
at `HEAD` on branch `main`. You are the only agent editing this tree. **Commit nothing** — the controller
commits through the pre-commit gate when you return. Do not push, do not `git stash`, do not spawn
subagents, do not run `git checkout -- <file>` over your own uncommitted work.

Read first, in this order:
1. `design/agent-reports/2026-09-07-build-interview-T8-review.md` — the review, verbatim.
2. `design/agent-reports/2026-09-07-build-interview-T8-review-VERIFICATION.md` — the controller's ledger.
   **Every finding below is already machine-confirmed by the controller.** Do not re-litigate whether a
   finding is real; the ledger records the observed output for each. Your job is the fix and its kill.
3. `design/agent-reports/2026-09-07-build-interview-T8-implementation.md` — the build report you are
   correcting (its §1 and §5 both contain sentences the review falsified; fix the prose too).
4. `CLAUDE.md` at the repo root — the standing rules bind this fold.

**A fold is authorship and re-earns the build gate.** Review-response edits are the text nobody has read
yet. Everything machine-checkable gets machine-checked by you, before you return.

## Validation

`export CARGO_TARGET_DIR=` **unset** — you are in the main tree, use the default `target/`. Scoped runs
while you work (`cargo nextest run --locked -p <crate> -E '<filter>'`); `make check` once at the end
(~33 s, 3449 tests + clippy). The instruments are `cargo run -q -p xtask -- line-coverage` /
`census-join` / `stop-list` / `prompt-check` — run all four before you return; they are OK at HEAD with
373/18/31/0, 290/13, 8+4/86, and 86 verbatim assertions respectively, so any change there is yours.

## I-1 (fold first) — row (5)(b) prints CHECKED under an unchecked row (5)(a)

Confirmed by the controller: `demands(LivedWithYouInUs) = false`, the leaf survives as `Some(true)`,
`screen_param_free` returns `None`, and the printed grid is `(5)(a) = false , (5)(b) = true`.

`ReturnHeader::build` (`packet.rs:476-484`) projects the grid from the **raw leaves**, so a filer who
answers (5)(a) *Yes* + (5)(b) *Yes* and then flips (5)(a) to *No* leaves a stale `Some(true)` that nothing
clears, nothing screens, and `push_dependents_grid` (`form1040_full.rs:633`) writes unconditionally. The
form seam then hides it (`sections.rs:546-552`, `get` → `None`) and refuses to clear it
(`sections.rs:576-587`, `clear` → `SetError::NoSuchRow`), so it is unreachable through the product surface
and printed on the page — a sub-condition asserted under a condition the return does not assert.

**Fix.** Build the grid from the **walk**, not the leaf: take `let walk = walk_dependent(ri, row);` once in
`ReturnHeader::build` and gate each row-(5)/(6) bool on `walk.demands(gate) && leaf == Some(true)`.
`DependentWalk::demands` is already `pub` (`dependent_gates.rs:183`) and is the module's stated definition
of liveness. `credit` can then read that same walk instead of walking a second time — do that only if it
falls out cleanly; correctness first.

Decide and state which of the four row-(5)/(6) gates are unconditionally demanded in the Step 1 block (the
review says (5)(a) and both row-(6) gates are, so only (5)(b) can go stale today). Gate **all four**
anyway: the guarantee is *not-demanded ⇒ blank*, and a future gate that becomes conditional must not
re-open this hole silently.

**Kills (B1 — plant the defect, watch the instrument red).**
- The one the existing test skips: 5(a) = `Some(false)` **with 5(b) = `Some(true)`** must print (5)(b)
  blank. Assert on `ReturnHeader::build`'s output, not on a hand-set `None`.
- Revert your fix (in the buffer, not via git) and watch that test red; paste the failure.
- Fix `the_credit_column_reaches_the_row_the_emitter_prints` (`dependent_gates.rs:1690-1699`) or leave it
  and add yours beside it — but do not leave a test whose comment claims the guarantee it does not test.
- The doc comment at `packet.rs:258-261` currently states the guarantee as fact. After your fix it is
  true; leave it, and make sure the wording still matches what the code does.

## I-2 — the FR-85 CTC pin cannot red on the defect it is documented to catch

Confirmed: with TY2026's package bundled the pin **passes**; and flipping the constant alone flips
`ctc_provably_zero(MFJ, 2 kids, AGI 482000)` from `true` to `false`, i.e. the stale ceiling swears a `0` on
line 19 for a household that still has credit.

`the_named_ceiling_is_the_years_own_figure` (`advisories.rs:906-960`) reads
`crate::tax::testonly::ty2024_params()` — a core-local literal hardcoded to `year: 2024`. Core cannot see
`BundledFullReturnTables` at all (zero occurrences in `crates/btctax-core/src/`; it lives in
`btctax-adapters`), so no bundling anywhere can red it. Its own doc calls this the *"`1..=38` trap in its
usual costume"*.

**Fix.** Make the test read what it claims to read. The crate boundary is the constraint, so either:
(a) take the params (or a `FullReturnTables`) as a parameter to a `pub(crate)` core helper and drive it
from a **btctax-adapters** test over `BundledFullReturnTables::load()`'s real registered years; or
(b) whatever cleaner shape you find — but it must iterate the years the bundle actually registers, never
a hand-written year list or a fixture literal.

**Kills.**
- Bundle a second year (`by_year.insert(2026, ty2026_full_return());` in `tax_tables.rs:102`, whose
  `child_tax_credit_per_child` is already `dec!(2200)`) and watch your new test **red once**. Paste it.
  Then revert the plant.
- Move the shipped TY2024 figure (`tax_tables.rs:151`, `2000` → `2200`) and confirm your test reds too
  (today only `every_shipped_full_return_params_equal_the_ones_the_corpus_validates` does). Revert.

**Do not change `CTC_PER_CHILD_SS24H2` to 2200.** TY2024 is the only bundled year and $2,000 is right for
it. The defect is the blind guard, not the number. The correct fix *when TY2025's package lands* is to
thread the package into `ctc_odc_line19` — say so in the doc.

**Correct the two doc comments and the build report's §5 sentence**, all three of which currently assert
the pin reds on a bundled package. And update `FOLLOWUPS.md` FR-85 to record that the pin was blind and is
now derived — cite the ledger.

## I-3 — `hoh_qualifying_child_name` is collected and never printed; QSS cannot supply it

Confirmed: zero matches for `qualifying_child_name` in `packet.rs` and all of `crates/btctax-forms/`. The
only write to the shared cell (`form1040_full.rs:476-489`) sits inside `if let Some(sp) = &header.spouse`
and under `status == FilingStatus::Mfs`, so HoH/QSS can never reach it. TY2024 is the only year
`full_return_for` returns `Some` for, i.e. the filable year. The field's own help quotes *"enter the child's
name in the entry space below qualifying surviving spouse"* while `sections.rs:333` is
`live: |ri| ri.filing_status == HoH`.

**Fix.** Carry the name onto `ReturnHeader`; write it into `cells.mfs_spouse_name` when `status` is `HoH`
or `Qss` and the string is non-empty (the MFS branch and this one are mutually exclusive by filing status,
so no cell is contested — assert that, don't assume it). Widen `live`/`get`/`set` to `HoH | Qss`. Rename
the leaf and the `FieldId` to drop the `hoh_` prefix, and follow the compiler through the classifier,
`scrub`, `scrub_axis`, `coverage` and the CLI fixture. Correct the stale emitter comment
(*"which v1 does not capture"*).

The form's sentence is one cell for three statuses — check the extract before you write:
`f1040--2024.txt:28-29`. QSS condition 2 (`i1040gi--2025.txt:1298-1306`) is exactly the household it serves.

**Kill.** A KAT that fills the field on a **TY2024 HoH** return and reads `f1_18` back off the filled PDF;
a second on QSS. Plant: drop the `Qss` arm → the QSS KAT reds. Note in your report whether the TY2025
template's cell is the same FQN, and whether anything needs a census entry.

## M-1, M-2, N-1 — fold as cleanups (do not skip; they are cheap)

- **M-1** `the_ty2024_1040_is_byte_identical_…` (`full_return_forms.rs:770-793`) hashes two calls in the
  same build — a determinism tautology. Either pin a **committed** digest, or rename the test to what it
  actually asserts. Your call; state which and why.
- **M-2** `prompt-check` reads only quoted spans, so a prompt's operative lead-in can drift silently
  (controller reproduced: *"over half"* → *"most"*, `prompt-check` still OK). Minimal fix: quote the
  operative clause inside the prompt so the existing checker covers it — no new instrument. Apply it to the
  HoH cost prompt (`questions.rs:2210`) and sweep the other T8 prompts for the same shape.
- **N-1** `every_slot_caption_is_the_forms_own_words` (`dependents_grid.rs:538-557`) is
  `hay.contains(&needle)` — presence, not ORDER. The order is available: `f1040--2025.txt:47` reads
  `Full-time  Permanently  Full-time  Permanently …`. Add the order assertion and plant a caption swap to
  watch it red.

## Standing lessons that bind this fold

- **No decision keys on a hand-typed list beside derived data** — I-2 is that failure exactly, and I-1 is
  its cousin (a projection typed beside the walk that defines liveness).
- **A kill CALLS the instrument.** A test that re-implements the predicate is a shadow — I-1's existing
  test is one. Run the real command / the real build path.
- **A prompt hash keys on the registry's WORDS, never display chrome.**
- **Blank is the normal case**; assert provenance, never non-blankness.
- **Transcribe from the text layer**, and re-verify every cross-reference against the extract.

## Report — your FINAL action

Write `design/agent-reports/2026-09-07-build-interview-T8-fold.md`: Commands (with real output);
one section per finding (What changed / Where / The kill and its observed red / Anything you decided
differently and why); a **Deviations** section if you departed from this brief; the four instrument
outputs; the closing `make check` summary line. Then return ONLY a 3-line summary plus the path.
