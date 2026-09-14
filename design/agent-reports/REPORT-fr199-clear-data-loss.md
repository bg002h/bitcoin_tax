# REPORT — FR-199: `income clear` destroys a committed return's recorded answers with no guard

**Verdict: BOTH HALVES REPRODUCE.** Driven end to end against an isolated vault, not reasoned from
source. Half (a) is a data-loss defect; half (b) reproduces exactly as reported and is a consequence of
(a) rather than a second bug, so one guard closes both. Fixed, gate green, **nothing committed** — the
work is uncommitted in worktree `agent-a7013b87b906a9132` (branch `worktree-agent-a7013b87b906a9132`).

FR-199's headline count is wrong in one detail and should be reworded — see §7.

---

## 1. The isolation recipe — and why the controller's attempt did not redirect the store

★ **`HOME` / `XDG_DATA_HOME` do not move the vault. They were never going to.** `dirs::data_dir()` is
used in exactly one place in the CLI:

    crates/btctax-cli/src/price_cache.rs:23    dirs::data_dir().map(|d| d.join("btctax").join("price_cache.csv"))

That is the **price cache**, not the store. The vault is a plain path argument:

    crates/btctax-cli/src/cli.rs:28-29
      #[arg(long, global = true, default_value = "vault.pgp")]
      pub vault: PathBuf,

So the default store is `./vault.pgp` **relative to the process CWD** — and there is a real one at
`/scratch/code/bitcoin_tax/vault.pgp` (1,314,975 bytes, mtime 2026-07-03). Any `btctax` run with the
main tree as CWD and no `--vault` writes to the operator's real vault. That is the trap the controller
fell into.

**THE RECIPE, for every future journey test:**

```
export BTCTAX_PASSPHRASE=<scratch pass>        # non-interactive; main.rs:49-53
btctax --vault SCRATCHDIR/vault.pgp init --key-backup SCRATCHDIR/key-backup.asc
btctax --vault SCRATCHDIR/vault.pgp <anything>
```

`--vault` moves the **whole** store: `vault.pgp`, `vault.key`, `vault.pgp.bak` and `vault.pgp.lock` all
appeared beside it in the scratch directory and nowhere else. Verified, not assumed.

**AND VERIFY IT HELD, with a witness hash, before anything destructive.** I recorded
`sha256(/scratch/code/bitcoin_tax/vault.pgp)` before the first command and re-checked it after every
destructive one:

    16af86f6a2545f1ba6b73b72c279f0128028f7c0fe4404d88c96e00040075351   (before, and after every step)

Identical at the first check and the last. The real vault was never opened, read or written.

★ Pass the explicit flag on *every* invocation rather than relying on a CWD: one forgotten flag in a long
script silently retargets the real store, and nothing in the CLI warns you.

★ Two more mechanics that made the drive possible, for whoever writes the journey harness:
`BTCTAX_PASSPHRASE` skips the prompt (`main.rs:49-53`), and `income answer` reads plain lines from stdin
where a bare Enter KEEPS the value on file. Only a question with nothing on file rejects a bare Enter and
re-asks, so piping `500 blanks, then the answer, then 500 blanks` drives the whole interview without
knowing the prompt order.

---

## 2. Half (a) — REPRODUCED. A committed return is deleted on nothing at all.

**Setup** (isolated vault): `income import --year 2024` of
`crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` (the J6 worked example, minus
`digital_asset_activity` so the interview has something to ask), a 2-row Coinbase 2024 CSV (one Buy, one
Sell — `verify` reports conservation BALANCED, 0 hard blockers), then `income answer --year 2024` driven
over stdin, answering the Digital Assets question **Yes**.

**State before:**

    tax_year                = 2024
    digital_asset_activity  = True
    answer_log records      = 67
    THE DIGITAL-ASSET RECORD: question:DigitalAssetActivity ->
      {"answered_on": [2026, 257], "prompt_hash": "fd817aaa24c3abdac9a48c3130cbfe62c7a9a81301b668a4b87f077d3d91a32d", "state": "given"}

**The command, with stdin closed — no TTY, no flag, no prompt:**

```
$ btctax --vault SCRATCHDIR/vault.pgp income clear --year 2024 < /dev/null
Cleared full-return inputs for tax year 2024.
### EXIT=0
```

**State after:**

```
$ btctax --vault SCRATCHDIR/vault.pgp income show --year 2024
No full-return inputs set for tax year 2024.

$ btctax --vault SCRATCHDIR/vault.pgp income answer --year 2024
error: usage: no full-return inputs and no draft for tax year 2024 — `income answer` fills in the
questions on an EXISTING return. …
```

67 `AnswerRecord`s gone. **No draft, no stash, no backup, no undo** — `income answer`'s own error
confirms neither a committed row nor a draft survives. One line of output, exit 0.

### Why this is data loss and not merely a deletion

The product states the reason itself, on the *weaker* artifact's flag (`cli.rs:610-618`,
`--discard-draft`):

> *"its recorded answers cannot be re-created by re-typing (btctax records when and in what words it
> asked)"*

Every word applies verbatim to the committed row, which holds the **same** `answer_log`. And `income
import` is deliberately **not** a restore: it discards an incoming file's `answer_log` and re-attaches
the stored row's (`cmd/tax.rs:158-181`, the seam-review C1 fold) — so once the stored row is gone,
re-importing the identical TOML brings back the figures and *nothing* of the record. Confirmed both ways:
a re-import preserved all 67 records while the row existed; after the clear there was nothing to preserve.

### The asymmetry is the finding, and the codebase already knew the rule

| artifact | content | guard before this fix |
|---|---|---|
| PARKED draft | a screened return, sole copy | **unconditional refusal** (`ParkedDraftBlocksWrite`) |
| WIP draft holding one answer | an interview | **refused without `--discard-draft`** |
| **COMMITTED return: 67 answers + 6 document rows + a dependent + a Schedule A** | a screened, stored return | **nothing** |

`open_next_year` states the principle out loud (`open_next_year.rs:214-220`), refusing to reset a
committed row because *"the opener starts a year, it does not reset one"* — and then names `income
clear` as the way to do it anyway.

### ★ And this path is PRESCRIBED BY THE PRODUCT, five times

FR-199's note that it *"surfaced as the only escape from FR-200's wall"* is exact. Reproduced verbatim,
before the fix:

```
$ btctax --vault SCRATCHDIR/vault.pgp report --tax-year 2024
error: usage: tax year 2024 cannot be computed from its full-return inputs: Schedule C net profit is
negative (a loss) — §465 at-risk substantiation is out of scope for v1; run `income clear --year 2024`
to remove them and use a raw `tax-profile`
```

The five in-product prescriptions (grep for the command string over the crate sources, non-test):

1. `resolve.rs:227` — the uncomputable-refusal sentence above.
2. `year_readiness.rs:472` — the not-ready-year fallback sentence.
3. `open_next_year.rs:219` — *"clear it first … which discards what it holds"*.
4. `lib.rs:142` — `StaleReturnInputs`' three-command remedy.
5. `btctax-tui-edit/src/edit/persist.rs:341` — the D-4 tax-profile refusal.

None said what would be lost. This is the whole shape of the defect: **the destructive path is the
documented workaround for something else**, so it is reached by a filer who is already stuck, following
instructions.

### ★★ It was also an unimplemented SPEC MANDATE

`design/SPEC_input_surface.md` §D-7 already required it, and named the same entry point:

> *"**`income clear` warns and requires confirmation when the header carries secrets.** It is the tool's
> own advertised recovery from an uncomputable year (`resolve.rs:216–224`) — and after Cycle 2 it
> destroys SSNs that exist nowhere else."*

(§6 of the same spec lists *"`income clear` — confirmation + `--keep-identity`"*.) The confirmation
shipped for the draft and never for the committed row. `--keep-identity` does not exist at all — §7.

---

## 3. Half (b) — REPRODUCED, machine-checked at the printed-checkbox level

This needed a ledger with 2024 crypto activity (so the fallback packet writes a Form 1040 page at all);
with an empty ledger the export writes only `f8949.pdf` + `schedule_d.pdf` and the question never
appears. With the 2-row Coinbase CSV in place:

**BEFORE `income clear`** — full-return packet, `pkt-before2/00_f1040.pdf`. AcroForm values for the
Digital-Asset Yes/No button pair (`crates/btctax-forms/forms/2024/f1040.map.toml:24-25`:
`da_yes = c1_5[0] on "1"`, `da_no = c1_5[1] on "2"`), read out of the PDF with `qpdf --json=2`:

    obj=obj:567  /T=u:c1_5[0]  /V='/1'    /AS='/1'    /FT='/Btn'      <- YES is marked
    obj=obj:568  /T=u:c1_5[1]  /V='/Off'  /AS='/Off'  /FT='/Btn'

and `pkt-before2/manifest.txt`'s *"COMPLETE BY HAND"* section carries exactly **one** mark — the
signature block. Nothing about the Digital Asset question, correctly: it is answered.

**AFTER `income clear`** — the same command, same year, now the crypto-slice packet
(`pkt-after2/form_1040_capgains.pdf`):

    obj=obj:567  /T=u:c1_5[0]  /V='/Off'  /AS='/Off'  /FT='/Btn'      <- NEITHER box marked
    obj=obj:568  /T=u:c1_5[1]  /V='/Off'  /AS='/Off'  /FT='/Btn'

and the export prints:

```
⚠ 1 mark(s) on this worksheet are YOURS to make by hand and are deliberately blank:
  • Form 1040 — the Digital Asset question (above line 1a): neither "Yes" nor "No" is marked, because
    this return does not record an answer to it. btctax will not swear either way for you — a wrong
    "No" here is sworn testimony under §6065. The question is MANDATORY: answer it
    (`btctax income answer`) and re-export, or mark it yourself before you sign.
```

**The sentence is made true by the destruction.** The filer answered it — on 2026 day 257, against
prompt hash `fd817aaa…`, `state: given` — and the tool now tells them the return does not record an
answer. Per *"an entry is testimony"*: a `0` on an unasked line fabricates testimony; this **fabricates
its absence**, on a return that gave it. The provenance was destroyed and the question silently re-asked.

### ★ It is NOT a display bug, and the display must NOT be the fix

`DIGITAL_ASSET_HAND_MARK` (`cmd/admin.rs:448`) and the blank pair are **correct** given the inputs: with
no `ReturnInputs`, `da_answer = working.and_then(|ri| ri.digital_asset_activity)` is `None`
(`cmd/admin.rs:1153`), and printing *"Yes"* from a record that no longer exists would be inventing
§6065 testimony — strictly worse. **(b) is a consequence of (a) and its only correct fix is at the point
of destruction**, which is what §5 does. Nothing in the print path was touched.

### ★ A second-order defect found while reproducing (b), NOT fixed — see §7

The hand-mark's own remedy is *"answer it (`btctax income answer`) and re-export"*. On a year with no
`ReturnInputs`, `income answer` **refuses** (*"no full-return inputs and no draft … Create one with
`btctax income import`"*). So the mark prescribes a command that cannot run — for every crypto-slice
filer, not only a post-clear one. Same class as FR-198. Reported, not fixed: different surface, and it
needs its own wording decision.

---

## 4. The general case — derived, not spot-checked

**"Which verbs can delete a committed return's recorded answers?"** answered by enumerating the writers,
per *"derive the list, or make the compiler hold it."* Three greps over the whole crate tree:

**`return_inputs::delete` — exactly 2 call sites** (the only way a committed row dies):

| site | verdict |
|---|---|
| `cmd/tax.rs:639` — `income clear` | **THE DEFECT.** Unguarded. Now guarded. |
| `input_form_store.rs:740` — `park_to_profile` | **Non-destructive**: stashes the row as `parked=1` *first*, atomically (`mutate_and_save`). |

**`delete_draft` — 5 production sites, all already guarded:** `coherence_clear`'s two arms (`Disposable`
= seed-equal only; `ConfirmedDiscard` = `--discard-draft` given, and it prints what it destroyed),
`load`'s stale discard (refuses via `StaleDraftHoldsInterview` when the draft holds an interview),
`commit` (the draft is superseded by the committed row it just wrote), and `discard_blocked_draft` (an
explicit affordance that refuses a readable WIP draft outright).

**`return_inputs::set` — 5 production sites**, and none loses the log: `commit`, `income answer`
(additive), `write_back_carryover` (in-place update of the fetched row), the TUI write-back, and `income
import` — which is a whole-blob upsert but **re-attaches the stored row's `answer_log` and
`answer_log_history` unconditionally** (`cmd/tax.rs:178-181`).

**Two near-misses checked and cleared:**
- `open_next_year` **refuses** an existing committed year-N+1 row (`open_next_year.rs:214-220`) and
  writes only a draft for N+1; it never touches year N.
- `tax-profile set --force` stores the profile *alongside* the return (`cmd/tax.rs:28-31`, *"Re-run with
  --force to store it anyway"*). It does **not** delete the row, despite two messages implying the
  filer must clear first.

**Conclusion: `income clear` was the only unguarded destroyer, and the derivation is a two-line grep** —
not a judgment. Any new deletion site shows up in a grep for `return_inputs::delete`.

★ Honest residue: `vault.pgp.bak` holds the previous whole-store image, so the bytes may be recoverable
by hand-swapping files — but nothing tells the filer that, it is the *whole store* and not the year, and
the next save overwrites it. It is not an undo and I did not treat it as one.

---

## 5. The fix

**(a) `income clear` REFUSES a stored return that holds more than the year's bare seed, unless
`--discard-return` is given.**

★ **The decision is `input_form_store::draft_is_disposable`, CALLED** — the same whole-struct comparison
against a fresh seed that the draft half keys on, and already `pub` and already taking `&ReturnInputs`
(it is not draft-specific; only its name is). So:
- the two halves **cannot diverge**;
- a field added to `ReturnInputs` tomorrow is protected the day it is added, with nobody remembering
  this file — the failure direction of a category list is OPEN, a seed comparison fails CLOSED;
- `holdings` is `describe_draft`'s clause, including its *"work not otherwise itemised"* fallback, so a
  filer is **never** told the return holds "nothing" while being asked to confirm destroying it.

**On the spelling.** Not a copy of `--discard-draft`: it names a different artifact and is deliberately
a *separate* flag, on the `--force` precedent (*"overrides THIS guard and nothing else"*). A year can
hold both a draft and a committed return, and authorising the discard of crash-scratch is not
authorising the deletion of a screened return. What the operator types is
`btctax income clear --year N --discard-return`.

**What the refusal actually says** (real output, isolated vault):

```
$ btctax --vault SCRATCHDIR/vault.pgp income clear --year 2024 < /dev/null
error: year 2024 has a stored full return holding 67 recorded answer(s), 2 Form W-2 row(s), 1 Form
1099-INT row(s), 1 Form 1099-DIV row(s), 1 Form 1099-G row(s), 1 Form 1098 row(s), 1 dependent(s), a
Schedule A, and `income clear` DELETES it. Nothing was deleted. The recorded answers are the part no
re-import restores: btctax records when, and in what words, it asked you, so re-importing the same TOML
brings the figures back and not the record — and a return whose Digital Asset answer is gone prints that
mandatory question with neither box marked. To fall back to your tax-profile and KEEP this return, open
the tax-inputs form for 2024 (`btctax-tui-edit`) and press 't' — that parks the return instead of
deleting it, and 't' reinstates it. To delete it, re-run with --discard-return.
### exit=2
```

★ **It names the NON-DESTRUCTIVE exit first, and that exit is real.** `income clear`'s stated purpose —
*"fall back to a raw `tax-profile`"* — is already achieved without deletion by `park_to_profile`, which
only the TUI reaches. Verified before writing it into a message (FR-198's lesson): `btctax-tui-edit` is
a real binary (`crates/btctax-tui-edit/Cargo.toml:14`), `KeyCode::Char('t')` dispatches `toggle_source`
(`main.rs:1384-1385`), and with a committed return active that opens the `ParkToProfile` confirm whose
own words are *"the committed return for {year} is stashed as a parked draft and the year resolves
through your tax-profile again. Reinstate it later with 't'"* (`main.rs:1740-1744`).

★ **Ordering: the committed guard runs BEFORE `coherence_clear_or_refuse`.** The M-1 rule that the
coherence call must precede any committed-row read exists so a writer that *early-returns* on an absent
row cannot shadow the parked-draft refusal. This guard never early-returns — an absent or disposable row
falls straight through — so a parked year (which has no committed row) still raises
`ParkedDraftBlocksWrite` exactly as before, and running it first means the refusal path mutates nothing
at all, not even the in-memory draft delete. Both pre-existing draft-refusal tests still pass with their
behaviour unchanged.

**(b)** is closed by the same guard: the only route from an answered return to that blank mandatory
question now requires typing `--discard-return`, and the refusal states in advance that the box will go
blank. No print-path change, deliberately (§3).

**Whole-surface sweep.** A guard that leaves four in-product remedies unrunnable is half a fix, so all
four readable-row prescriptions now name the flag, and the two that have a non-destructive alternative
name it *first*: `resolve.rs` (the reproduced route in), `year_readiness.rs`, `open_next_year.rs`,
`btctax-tui-edit/edit/persist.rs`, plus three `LIMITATIONS.md` passages. Re-driven after the fix:

```
error: usage: tax year 2024 cannot be computed from its full-return inputs: Schedule C net profit is
negative (a loss) — §465 at-risk substantiation is out of scope for v1; run `income clear --year 2024
--discard-return` to DELETE them and use a raw `tax-profile` — or keep the return and park it instead
(the tax-inputs form for 2024, `btctax-tui-edit`, then 't')
```

**★ A separate defect found by the sweep, and fixed because my own boundary argument depends on it.**
`CliError::StaleReturnInputs` prescribed a POSITIONAL year — `btctax income clear 2024`. That does not
parse:

```
$ btctax … income clear 2024
error: unexpected argument '2024' found
Usage: btctax income clear [OPTIONS] --year <YEAR>
```

So the one escape from a stale row named a command line that does not run (FR-198's class). Fixed to
`--year {year}` in `lib.rs:142` and in the two `LIMITATIONS.md` passages, and the text-test at
`return_inputs.rs:481` now pins the **parsable** form (it asserted `"income clear 2024"`, which the
new message no longer contains), which is what makes it a test rather than a proofread.

**★★ THE STALE-ROW BOUNDARY, STATED IN THE SOURCE** (rule 3 of *"derive the list, or make the compiler
hold it"*). The guard describes a row it can **read**. A row at a schema version this build does not
deserialize falls **through** and clears as before, deliberately: `StaleReturnInputs`' own remedy *is*
clear then import then `--write-carryover`; nothing can park, edit or export a row this build cannot
deserialize (`park_to_profile` reads it too); and refusing would leave the filer with no exit at all —
FR-102's shape, safe about the row and total about the outcome. Implemented as a `match` that passes
`StaleReturnInputs` through and propagates every other error, **not** a blanket swallow.

★ I did not discover that boundary by thinking of it. My first cut used `return_inputs::get(...)?` and
the build gate reded on `tax_report`'s existing *"clear works on a stale row (it never deserializes)"* —
i.e. my fix bricked the documented recovery, and an existing test was already the kill for it. Recorded
because it is the argument for running the gate before a reviewer sees anything.

### Files changed (14; nothing committed)

```
 crates/btctax-cli/LIMITATIONS.md           |   8 +-
 crates/btctax-cli/src/cli.rs               |  21 ++++
 crates/btctax-cli/src/cmd/tax.rs           | 183 ++++++++++++++++++++++++++++-
 crates/btctax-cli/src/lib.rs               |  38 +++++-
 crates/btctax-cli/src/main.rs              |   3 +-
 crates/btctax-cli/src/open_next_year.rs    |   8 +-
 crates/btctax-cli/src/resolve.rs           |   9 +-
 crates/btctax-cli/src/return_inputs.rs     |   6 +-
 crates/btctax-cli/src/year_readiness.rs    |   6 +-
 crates/btctax-cli/tests/tax_profile.rs     |   6 +-
 crates/btctax-cli/tests/tax_report.rs      |  19 ++-
 crates/btctax-cli/tests/year_gate_t4.rs    |   5 +-
 crates/btctax-tui-edit/src/edit/persist.rs |   9 +-
 docs/man/btctax-income-clear.1             |  13 +-
 14 files changed, 311 insertions(+), 23 deletions(-)
```

`docs/man/btctax-income-clear.1` is GENERATED — regenerated with `cargo run -p xtask -- docs`; it is the
only man page that changed, and `gen_docs_is_deterministic` passes.

---

## 6. B1 — seen red once

**Which test reds when this is reverted?**
`btctax-cli` lib test `cmd::tax::tests::income_clear_refuses_to_destroy_a_stored_returns_recorded_answers`.

**Observed RED** with the guard neutralised (`if !discard_return {` changed to `if false {`; via
cp-backup/restore, never a VCS checkout over uncommitted work):

```
running 1 test
test cmd::tax::tests::income_clear_refuses_to_destroy_a_stored_returns_recorded_answers ... FAILED

thread '…' panicked at crates/btctax-cli/src/cmd/tax.rs:1847:71:
called `Result::unwrap_err()` on an `Ok` value: true

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 257 filtered out
```

`Ok(true)` is *exactly* the shipped behaviour: a successful destructive clear. **Observed GREEN** after
restore:

```
running 2 tests
test cmd::tax::tests::income_clear_refuses_a_parked_draft_and_preserves_it ... ok
test cmd::tax::tests::income_clear_refuses_to_destroy_a_stored_returns_recorded_answers ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 256 filtered out
```

**The plant is in the FILER's vocabulary, not the checker's (FR-235).** The answer is stamped by
`btctax_core::tax::provenance::record_answer` — the one writer `income answer` itself reaches — and the
assertion is that the `AnswerRecord` is still **readable from disk** through a *fresh* `Session`
afterwards. Nothing in the test mentions `draft_is_disposable`, `describe_draft`, or the error variant's
fields, so a guard that refuses for the wrong reason, or refuses and saves anyway, still fails it.

Four arms, each with its own kill:
1. **no flag ⇒ refuse, and the record survives on disk** — the defect. Reverting the guard reds this.
2. **`--discard-return` ⇒ deletes, and the delete reaches disk** — the guard is an acknowledgement, not
   a brick.
3. **a row whose ONLY content is the filer's SSN is also refused**, and honestly described as *"work not
   otherwise itemised"*. This is `SPEC_input_surface.md` §D-7's unimplemented mandate, and it holds a
   structural claim the other arms cannot: that row has zero recorded answers, zero document rows, no
   dependent and no Schedule A, so **every category `describe_draft` itemises is empty** — swapping
   `draft_is_disposable` for a category list (`DraftHoldings::is_empty`) reds this arm *alone*.
4. **a bare-seed row still clears unprompted** — five in-product messages prescribe `income clear` as a
   recovery step and none should start demanding a flag for a row holding nothing.

The stale-row boundary has its own kill, already in the suite: `tax_report`'s *"clear works on a stale
row (it never deserializes)"* reds if the guard is written with `?` instead of the `StaleReturnInputs`
match (observed — it is how I found the brick).

---

## 7. Recommendations for the ledger (FOLLOWUPS.md is the controller's; nothing written there)

**FR-199 — reword, then close.** Both halves reproduced; the fix is in the worktree. Corrections to the
entry's wording:

- ★ **"46 answers" should not be a number.** The count is per-return: this vector destroyed **67**
  `AnswerRecord`s. Say *"a committed return's recorded answers (67 on the reproduction vector)"* — a
  fixed number in a ledger headline invites someone to check for 46, not find it, and doubt the report.
- Half (b) should be recorded as a **consequence of (a) whose only correct fix is at the point of
  destruction**, not as an independent defect. Printing the deleted answer would fabricate §6065
  testimony; the entry as written could be read as asking for exactly that.
- Worth adding: the defect was an **unimplemented spec mandate** (`SPEC_input_surface.md` §D-7), and the
  path is **prescribed in five in-product messages**.

**NEW — the crypto-slice hand-mark prescribes a command that refuses.** `DIGITAL_ASSET_HAND_MARK`
(`cmd/admin.rs:448-454`) says *"answer it (`btctax income answer`) and re-export"*; on a year with no
`ReturnInputs` that command refuses and points at `income import`. Affects **every** crypto-slice filer,
not only a post-clear one. FR-198's class, one sentence to fix. Not fixed here.

**NEW — `income clear --keep-identity` is specified and does not exist.** `SPEC_input_surface.md` §D-7
and §6 specify it, including the requirement that the kept row be marked incomplete and **refuse at
resolve** so the year cannot silently compute from zeros. There is no such flag on `Clear`. Either
implement it or retract it from the spec — a specified affordance that does not exist is exactly the
shape §D-7 itself warns about.

**NEW — no CLI verb parks a return.** `park_to_profile` is the non-destructive form of what `income
clear` advertises, and it exists only behind a TUI keystroke. Every one of the five prescriptions of
`income clear` actually wants *park*, not *delete*. An `income park --year N` would make all five
non-destructive by default; my refusal points at the TUI because that is the exit that exists today.
Worth a decision.

**NOTE for FR-200.** Reproduced in passing: a Schedule C **loss** refuses the whole 2024 packet, and the
only escape offered was the destructive one. That is FR-200's shape with a third trigger (negative
Schedule C, alongside the over-$750k mortgage and the noncash gift).

---

## 8. Gate — run SERIALLY, in the foreground, on the final tree

```
$ cargo nextest run --workspace
     Summary [  20.456s] 3822 tests run: 3822 passed, 12 skipped
NEXTEST EXIT=0

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 19.67s
CLIPPY EXIT=0          (0 lines matching ^warning or ^error)

$ cargo fmt --all --check
FMT EXIT=0             (empty output)
```

Not concurrent (FR-223). Nothing was backgrounded and left (FR-175). `cargo run -p xtask -- docs` was
re-run after the last clap edit; `gen_docs_is_deterministic` and the examples/walkthrough goldens are
inside the 3822.

**Not committed.** All changes are uncommitted in worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a7013b87b906a9132` on branch
`worktree-agent-a7013b87b906a9132`. `FOLLOWUPS.md`, `crates/btctax-oracle-harness/**`,
`crates/xtask/**` (run only, never edited), `design/forms/extract/` and every existing
`design/agent-reports/REPORT-*` were not touched.

**Real vault:** `sha256(/scratch/code/bitcoin_tax/vault.pgp)` =
`16af86f6a2545f1ba6b73b72c279f0128028f7c0fe4404d88c96e00040075351` before the first command and after
the last. Untouched.
