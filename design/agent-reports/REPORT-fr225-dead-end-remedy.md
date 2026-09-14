# REPORT — FR-225 (remainder): the dead-end remedy

**Agent:** implementer (opus). **Base:** `cc29bb30e` (worktree `agent-a51adc553cf4cac50`).
**Committed:** nothing. Work is uncommitted in the worktree.
**Gate:** 3824/3824 nextest pass · clippy `-D warnings` clean · `cargo fmt --all --check` clean.

---

## 0. The headline, and one correction to the brief

The brief's facts 3 (first bullet) and 4 are **STALE**. `income answer` **does** reach the question,
and has since `a48edd2db` — which is in my base. `answer.rs:895-940` escalates
`AskScope::StillNeeded` to `Every` when *the return refuses AND the default scope would ask nothing*,
and the §170(f)(8) skippable is `live: |_ri| true`, so `Every` puts it. It is behaviourally tested at
`crates/btctax-cli/tests/year_gate_t4.rs::fr225_income_answer_states_the_returns_own_verdict_and_reaches_the_refusing_question`,
which drives the command over a bricked return and asserts the stored answer moves and `report`
agrees. That test is green in my gate run:

    PASS [   0.536s] (2482/3824) btctax-cli::year_gate_t4 fr225_income_answer_states_the_returns_own_verdict_and_reaches_the_refusing_question

So candidate **(a)** is moot (`--re-answer` need not be named; the command named already works) and
candidate **(b)** is **already built**. I did not re-do either. What remained open was fact 3's
*second* bullet — *"remove that gift from the deduction"* — plus the general class.

**And the general class was worth more than the instance.** The derived checker I built for it found
a **second, independent dead end on its first run**: the product prints `btctax set-pii` in two
places and **that verb has never existed anywhere in the clap tree**.

---

## 1. What I built, and why that shape

### 1a. `crates/btctax-cli/tests/refusal_remedies.rs` (new, 684 lines) — the general defect, made a command

The brief asked whether *"a refusal names a cure that does not work"* can be made machine-checkable
across all refusals. **It can, for the existence half.** Derived on **both** ends per `CLAUDE.md`'s
*"derive the list, or make the compiler hold it"*:

| end | derivation | what rots if you hand-list it instead |
|---|---|---|
| the **cited** set | walk every `.rs` under `crates/`, take backticked `` `btctax …` `` / `` `--flag` `` spans in **non-comment** lines (= string literals, what the product prints) | a refusal written next year names a verb nobody re-checked |
| the **valid** set | walk `Cli::command()` — clap's own tree, subcommands + long options | renaming or deleting a verb silently falsifies every printed instruction |

Plus a **third surface that a source scan cannot reach and that I nearly missed**: `cli.rs`'s `///`
doc comments **are** clap's `--help` text. Excluding doc comments is right everywhere except there,
so `help_strings()` walks `get_about`/`get_long_about`/`get_help`/`get_long_help` over the whole tree
and runs the same citation check on the **rendered** strings (`>100` help strings asserted). That
surface is where the live `set-pii` defect in `income answer --help` lived.

**Three kills:**

1. `every_command_and_flag_a_printed_string_names_exists_in_the_cli` — whole workspace + all `--help`.
2. `the_170f8_refusal_names_only_cures_that_exist_on_every_arm_that_fires` — the **runtime** refusal
   detail, on **both** arms (`None` and `Some(false)`), which are different sentences with different
   cures and only one of which any test had ever read. It also **reds a refusal that names *no*
   command at all** — an omitted remedy is the same dead end from the other side, and that is the
   mutation a pure existence check survives.
3. `the_crypto_arm_offers_no_forgo_because_btctax_cannot_un_claim_a_ledger_donation` — see 1c.

**Per FR-235, the plant is not written in the checker's vocabulary.** The checker resolves against
the *command tree*, not against the refusal string, so editing the refusal string cannot satisfy it —
it can only move the citation to a different verb, which still has to exist. And the behavioural half
(*does running it change the filer's state?*) is deliberately a **different instrument** in a
different file (`year_gate_t4.rs`, above), which the module doc names as boundary 1.

**Stated blind spots, in the module doc** (a gate that hides its own is worse than no gate):
existence not efficacy; backticked citations only; bare tokens past the subcommand path are values;
a bare flag is resolved tree-wide, so it catches *"exists nowhere"* but not *"wrong command"*; and
`xtask`'s bare flags are skipped because its ambient binary is `cargo xtask`, not `btctax` — its
`` `btctax …` `` spans **are** still checked, because a span naming the binary is unambiguous.

### 1b. `return_1040.rs` — the cure is now DERIVED from which gift armed the gate

New `pub fn cwa_gift_sources(ri, state, year) -> CwaGiftSources { schedule_a_entry, ledger_donation }`,
and `a_single_gift_reaches_the_cwa_threshold` is now `cwa_gift_sources(..).any()`. **One decider, two
readers** — the gate consults it, and the refusal's cure reads its fields, so the sentence a filer
reads cannot drift from the condition that produced it. Two copies of the threshold test is exactly
the *"the thing that decides was not the thing that knows"* shape.

Why the split matters, and it is not cosmetic:

- **`schedule_a_entry`** — the filer **owns** `ScheduleAInputs::charitable`. Forgoing is a real act:
  delete the `[[schedule_a.charitable]]` row and re-run `btctax income import`. **I verified that is
  safe to name**: `cmd/tax.rs:179-180` re-attaches the **stored** answer log rather than the file's,
  so the re-import does not destroy the interview (and `cmd/tax.rs:149` refuses forged records).
  Without that check, naming `income import` would have been a data-loss instruction.
- **`ledger_donation`** — **there is no path at all.** `Removal.claimed_deduction` is computed by
  `project/fold.rs:1624` from the donation's legs **unconditionally** (`claimed_deduction:
  Some(claimed_deduction)`), and no CLI verb writes it. `max_single_donation_contribution` gates on
  `claimed_deduction > 0`, so the *only* way to drop a crypto gift out of the §170(f)(8) population
  is to change the event — which is false testimony.

### 1c. The design judgement: I did **not** build candidate (c), and this is the reasoning

Per the brief's warning and `CLAUDE.md`'s *"an entry is testimony"*: forgoing a deduction and
asserting *"the gift was not a charitable contribution"* are different acts, and a verb that silently
converts one into the other is worse than the dead end. For the **crypto** source the only reachable
mutations (`reconcile void`, re-classify the outflow) are precisely that conversion. So:

- **btctax does not offer to rewrite the ledger, and it says so.** The crypto arm now prints an
  explicit boundary instead of an impossible act — *an honest boundary is reviewable, a silent one is
  the defect* — and KILL 3 pins **both** the absence of the forgo offer here **and** the presence of
  the refusal-to-rewrite sentence, because a filer with no exit will otherwise reach for
  `reconcile void` unprompted.
- **A real `forgo` verb is a recommendation, not this fix.** It needs a new `ReturnInputs` leaf with
  its own provenance, and it has a trap I would not want to get wrong under a bounded-fix brief: a
  forgone gift must **not** generate a §170(d)(1) carryover, or the filer deducts it in a later year
  anyway — the exact laundering `apply_carryover_writeback`'s I-2 gate and `cwa_unvouched_carryover`
  already guard against, arriving by a third route. Recommended in section 4.
- **I did not touch the refusal.** §170(f)(8)(A) denies the deduction outright; the refusal is
  correct on both arms. Only the remedy changed.

### 1d. `Some(false)` now names the command for its *primary* cure too

The arm said *"Then answer yes and re-run"* — with **no command named**, which is how a crypto-only
`Some(false)` refusal would have named zero runnable things. It now says
*"Then run `btctax income answer` and answer yes"*, and states **why** it re-asks a question already
answered: *a recorded answer that STOPS the return is not a settled question.* That is the FR-225
escalation, finally printed where the filer reads it.

### 1e. The second dead end, found by the new instrument: `btctax set-pii`

`HeaderError::MfjWithoutSpouse` (`packet.rs:170`) told an MFJ filer whose spouse row is missing to
*"add the spouse's identity (`btctax set-pii`)"*. **There is no `set-pii` subcommand.** There never
has been one — identity reaches the vault only through the `[header]` table of an `income import`
file (verified against `docs/income-import-schema.md:335-341`) or the input-form screens. Fixed at
all three sites: the printed error (`packet.rs`), the `income answer --help` text (`cli.rs:670`), and
a stale doc comment (`return_inputs.rs:877`). The regenerated man page is the only docs churn.

---

## 2. "Which test reds when this is reverted?" — one sentence each, with pasted red

Every plant was applied to **real committed strings**, run, then reverted with `cp` restores
(md5-verified back to baseline — never a checkout-based revert, per the standing note).

### Plant A — the refusal names a verb that does not exist
`btctax income answer` rewritten to `btctax income acknowledge` throughout `return_1040.rs` (13
sites). **All three kills red:**

    test the_crypto_arm_offers_no_forgo_because_btctax_cannot_un_claim_a_ledger_donation ... FAILED
    test the_170f8_refusal_names_only_cures_that_exist_on_every_arm_that_fires ... FAILED
    test every_command_and_flag_a_printed_string_names_exists_in_the_cli ... FAILED
    ---- the_170f8_refusal... ----
    the None arm of the §170(f)(8) refusal sends the filer to 1 thing(s) that do not exist:
    `btctax income acknowledge` — `acknowledge` is not a subcommand of `btctax income`
    (it has: import, show, project, scrub, clear, open-next-year, answer)

### Plant B — a cure names a flag that does not exist
`--write-carryover` rewritten to `--write-carryovers`. **KILL 1 red; KILLs 2/3 correctly stay green**
(that flag is in the deferred-carryover branch, which those fixtures do not reach — the instrument
reports what it measured):

    a printed string names 1 command(s)/flag(s) that do not exist. …
      crates/btctax-core/src/tax/return_1040.rs: the product prints `--write-carryovers`,
      which is not an option of any btctax command

### Plant C — `--help` text names a verb that does not exist (the surface a source scan cannot see)
Restored `btctax set-pii` into `cli.rs`'s `income answer` long help. **KILL 1 red via the clap walk,
not the source scan** — the source scan skips `///`, which is exactly why the walk exists:

    `btctax answer` --help prints `btctax set-pii` — `set-pii` is not a subcommand of `btctax`
    (it has: init, import, verify, report, events, limitations, reconcile, config, export-snapshot,
     export-irs-pdf, extension, backup-key, optimize, what-if, income, tax-profile, defensive)

### Plant D — the shipped defect restored: offer the forgo unconditionally
`if src.schedule_a_entry {` rewritten to `if true {`. **KILL 3 red, and only KILL 3** — the citation
checks cannot see this, because every command named is real. It is the *applicability* that is false:

    test the_crypto_arm_offers_no_forgo_because_btctax_cannot_un_claim_a_ledger_donation ... FAILED
    …and must not send them to a TOML table their gift is not in:

### Plant E — the live defect, observed red before it was fixed (not a plant: the real thing)
The checker's **first run**, against the tree as committed:

    a printed string names 1 command(s)/flag(s) that do not exist. …
      crates/btctax-core/src/tax/packet.rs: the product prints `btctax set-pii` —
      `set-pii` is not a subcommand of `btctax` …

### Plant F — B1a: the checker's own liveness guard is not vacuous
Made `citations()` return an empty set (a blind scanner). **The anchor assertion fires rather than
passing clean**, which is the F2/F4 shape caught:

    the scanner did not find `btctax income answer` — the §170(f)(8)(A) refusal's own remedy.
    Either the remedy was renamed, or the scanner has stopped reading printed strings. It saw: {}

Note KILL 3 **passed** under the blind scanner. That is honest and deliberate: its assertions are
`contains()` over the runtime detail, so it does not depend on the scanner at all.

---

## 3. The rendered refusals, captured from a run (not transcribed from source)

**`Some(false)`, Schedule-A source** — tail, the part that changed:

> … Ask each charity for one showing the amount of money and a description (but not the value) of any
> property, and whether it gave you goods or services in return. **Then run `btctax income answer`
> and answer yes. (It puts this question again even though you have already answered it — a recorded
> answer that STOPS the return is not a settled question. FR-225.)** ★ IF A CHARITY WILL NOT PROVIDE
> ONE, you may instead FORGO the deduction for that gift. That is lawful, and it is NOT a statement
> that the gift did not happen: delete its `[[schedule_a.charitable]]` entry and re-run
> `btctax income import` — your recorded answers are kept, btctax re-attaches them. This question is
> then put to you again about the gifts that REMAIN, and answering yes is honest only if you hold an
> acknowledgment for every one of those.

**`Some(false)`, ledger-donation source** — same prefix, then:

> ★ FOR A CRYPTO GIFT THERE IS NO SUCH ALTERNATIVE, and btctax says so rather than name an act you
> cannot perform: it computes the §170(e) deduction for a ledger donation from the donation event
> itself, and no command un-claims it. Voiding the event, or re-classifying it as something other
> than a charitable contribution, would record in your ledger something other than what happened —
> and btctax will not rewrite your ledger to get past a substantiation rule. For that gift the
> acknowledgment is the only cure.

The `None` arm is textually unchanged (it already ended by naming `btctax income answer`) and is now
checked on every run.

---

## 4. Recommendations for the controller's ledger (I did not touch `FOLLOWUPS.md`)

1. **[Important, product gap] A crypto donor who cannot obtain a CWA cannot file through btctax.**
   The refusal is now honest about it, but the filer's lawful option — forgo that gift's deduction —
   has no implementation. Design sketch: a `ReturnInputs` leaf recording the filer's own *forgo*
   election per donation event (never inferred), consumed by `crypto_charitable_gifts` **and** by
   `charitable_carryover_out` so a forgone gift generates no §170(d)(1) carryover. **The carryover
   leg is the trap**: without it the filer deducts the forgone gift in a later year and the forgo
   becomes a deferral, which is the laundering class `cwa_unvouched_carryover` exists to stop.
   It must be unmistakably a forgo, never a retraction of the CWA answer.
2. **[Minor, unverified claim in a cure]** The deferred-carryover cure asserts *"`--write-carryover`
   will refuse to persist it"*. I verified the **flag exists** (the checker does that now) but did
   **not** verify the **behavioural** claim. `apply_carryover_writeback`'s I-2 gate and
   `cwa_unvouched_carryover` look like they do it; worth a kill that reds if that refusal is removed.
   This is a *refusal that may not refuse* — the still-blocking class — so it deserves its own test.
3. **[Minor] Promote the checker's reach.** It reads `crates/**/*.rs` and the clap tree. It does
   **not** read `docs/`, `LIMITATIONS.md`, or the man pages, where a stale verb citation would be
   equally dead. Cheap extension: run the same resolver over `docs/**/*.md`.
4. **[Nit] `xtask`'s bare-flag exemption is by crate name.** Stated in the source with its reason, so
   it is reviewable, but it is a typed exemption next to a growing set of crates. If a second dev-only
   binary lands it needs the same treatment, and nothing will say so.

---

## 5. Files changed (nothing committed)

| file | change |
|---|---|
| `crates/btctax-cli/tests/refusal_remedies.rs` | **new**, 684 lines — the derived checker, 3 kills |
| `crates/btctax-core/src/tax/return_1040.rs` | `CwaGiftSources` + `cwa_gift_sources`; `a_single_gift_reaches_the_cwa_threshold` delegates to it; the `Some(false)` cure derived per source; the primary cure now names `income answer` |
| `crates/btctax-core/src/tax/packet.rs` | `MfjWithoutSpouse` no longer names the nonexistent `btctax set-pii` |
| `crates/btctax-cli/src/cli.rs` | `income answer` help no longer names `set-pii` |
| `crates/btctax-core/src/tax/return_inputs.rs` | stale `set-pii` doc comment corrected |
| `docs/man/btctax-income-answer.1` | regenerated (`cargo run -p xtask -- docs`) — the only docs churn |

Diffstat over tracked files: `5 files changed, 135 insertions(+), 15 deletions(-)`, plus the new
684-line test. Nothing on the do-not-touch list was modified — no `btctax-oracle-harness`, no
`xtask/src/blockers.rs`, no `xtask/src/r15_stop_list.rs`, no `design/forms/extract/`, no existing
`REPORT-*`, no `TY2026-BLOCKERS-PREDICTION-2026-09-13.txt`, no `FOLLOWUPS.md`.

## 6. Gate output — run SERIALLY, in the foreground

    cargo nextest run --workspace --no-fail-fast
         Summary [  19.398s] 3824 tests run: 3824 passed, 12 skipped

    cargo clippy --workspace --all-targets -- -D warnings
        Finished `dev` profile [optimized + debuginfo] target(s) in 0.23s      (no warnings emitted)

    cargo fmt --all --check
    FMT: clean, no diff

One red was hit en route and fixed rather than worked around:
`xtask::docs::tests::gen_docs_is_deterministic` failed because `cli.rs`'s doc comment **is** the man
page. Regenerating produced a one-line diff in `btctax-income-answer.1` and nothing else — which is
itself evidence that the clap-help surface the checker now reads is a real, shipped, filer-facing one.
