# FR-108 (the wrap indentation baked into filer-facing literals) and FR-109 (`income answer` stops re-asking, behind a flag)

Implementer report, 2026-09-08. Shared main tree at `/scratch/code/bitcoin_tax`, branch `main`, from
HEAD `db591e10`. **Nothing is committed and nothing is pushed.** The two tasks are separable — no
source file is touched by both, and only `FOLLOWUPS.md` carries hunks from each.

Closing gate, run once at the end:

```
$ make gate
     Summary [  21.060s] 3558 tests run: 3558 passed, 12 skipped
```

3549 → 3558 is **+9 new tests and no test removed or renamed**: 4 for the FR-108 lint, 5 for FR-109.

---

## Task 1 — FR-108

### The count was re-measured, not trusted — and the entry's MECHANISM is refuted

FR-108's detector, run verbatim at `db591e10` before anything was edited:

```
$ .venv/bin/python detect.py
8 crates/btctax-core/src/tax/classifier.rs
7 crates/btctax-tui-edit/src/draw_edit.rs
5 crates/xtask/src/box_census.rs
2 crates/btctax-core/src/tax/return_refuse.rs
2 crates/btctax-cli/tests/repo_hygiene.rs
2 crates/btctax-forms/src/form8283.rs
2 crates/btctax-input-form/src/spec/sections.rs
1 crates/btctax-core/src/tax/provenance.rs
1 crates/btctax-core/src/tax/return_1040.rs
1 crates/btctax-cli/src/cmd/tax.rs
1 crates/xtask/src/r15_stop_list.rs
1 crates/xtask/src/line_coverage_check.rs
1 crates/btctax-forms/tests/full_return_forms.rs
TOTAL 34
```

Still 34, still 13 files. Two things then came out of reading them, and both changed the work.

**(a) The stated mechanism is wrong.** FR-108 blames a Rust `\`-newline continuation *"which eats the
newline and KEEPS the next source line's indentation"*. It does not. Measured with `rustc`:

```rust
fn main() {
    let a = "alpha \
             beta";
    let b = "alpha\
             beta";
    println!("[{a}]"); println!("[{b}]");
}
```
```
$ ./cont
[alpha beta]
[alphabeta]
```

A continuation skips the newline **and the next line's leading whitespace** — which is why every
correctly-wrapped literal in this repo puts the space *before* the backslash. So a continuation is the
**cure**, not the disease: the defect is a literal that was never wrapped at all, written or joined as
one physical line with the wrap indentation typed in as real spaces. This matters because a fix built
from the stated mechanism would have re-wrapped these literals and left the runs sitting inside them.

**(b) 5 of the 34 are correct as written**, and a derived predicate separates them with no file named
anywhere: **a literal carrying an embedded `\n` escape prints more than one line**, so it is a table, a
code sample or an aligned key/value block, and a run of spaces in it is the content. Partitioning the
34 by exactly that:

```
$ .venv/bin/python detect3.py
FLAG 29
   … (the 29)
SKIP (has \n escape) 5
   crates/btctax-cli/tests/repo_hygiene.rs:259      (a shell script)
   crates/btctax-tui-edit/src/draw_edit.rs:979      (  fmv_at_gift:       {…}\n    donor_basis: …)
   crates/btctax-tui-edit/src/draw_edit.rs:1002     (aligned key/value block)
   crates/btctax-tui-edit/src/draw_edit.rs:1296     (aligned key/value block)
   crates/xtask/src/r15_stop_list.rs:598            (a Rust source fixture)
```

The partition matches the by-eye reading of all 34 exactly. **29 were fixed; 5 were left alone**, and
the reason is in the lint's module doc rather than in a list.

### What changed, and where

Each of the 29 was collapsed (runs of ≥6 interior spaces → one space) and **re-wrapped as a real
`\`-newline continuation**, by a script that asserts the round trip per site: it reconstructs the
literal's value from the emitted source lines by Rust's own continuation rule and compares it to the
collapsed target. All 29 round-trips held.

```
crates/btctax-cli/src/cmd/tax.rs               1   ← the `income scrub` stale-draft note
crates/btctax-cli/tests/repo_hygiene.rs        1
crates/btctax-core/src/tax/classifier.rs       8
crates/btctax-core/src/tax/provenance.rs       1
crates/btctax-core/src/tax/return_1040.rs      1   ← the Form 8615 advisory
crates/btctax-core/src/tax/return_refuse.rs    2   ← the two Schedule 1-A trade-or-business REFUSALS
crates/btctax-forms/src/form8283.rs            2
crates/btctax-forms/tests/full_return_forms.rs 1
crates/btctax-input-form/src/spec/sections.rs  2   ← the two Form 8995-A field help strings
crates/btctax-tui-edit/src/draw_edit.rs        4
crates/xtask/src/box_census.rs                 5
crates/xtask/src/line_coverage_check.rs        1
TOTAL 29
```

All six of the literals FR-108 names as reaching a person verbatim are in that set.

**One site needed a hand fix after the mechanical pass.** `classifier.rs:490` had wrapped mid-path:
`Some(RefuseReason::` + 9 spaces + `HohMaritalBasisUnanswered)`. Collapsing to one space would have
printed `RefuseReason:: HohMaritalBasisUnanswered`, so the line break was moved to the word boundary
before it and the whole backticked path now sits on one continuation line, unbroken. It is the only
site where the run fell inside a token; the whole 29-site diff was read line by line to confirm that.

```
$ .venv/bin/python detect3.py          # after
FLAG 0
SKIP (has \n escape) 5
```

### The lint

`crates/xtask/src/wrapped_literal_check.rs`, wired as `cargo run -p xtask -- wrapped-literals` in
`crates/xtask/src/main.rs`, beside the other source-scanning instruments.

```
$ cargo run -q -p xtask -- wrapped-literals
wrapped literals: 356 sources scanned, no one-line literal over 120 chars carries a 6-space gap
```

It enumerates `crates/**/*.rs` from the tree — **there is no site list, no allow list and no escape
comment anywhere in it**, on purpose: an exemption list is FR-99's shape with a laundering path
attached. If a legitimate case ever reds, the fix is to widen one of the four derived conditions with
that case as its evidence. A `FILE_FLOOR` of 200 makes a walk that scans nothing an error rather than
a pass.

The scanner is hand-rolled rather than a regex over `"…"`, because three shapes would otherwise open a
phantom literal that swallows the rest of a file and make the checker silently blind: a `"` inside a
comment (block comments nest in Rust), a `"` inside a char literal (`'"'`, which this repo has), and a
raw string's `#` delimiters. Lifetimes are told apart from char literals.

**What it covers.** A finding is a string literal that is all four of: opened and closed on the same
source line; longer than 120 characters including its quotes; free of an embedded `\n` escape; and
carrying a run of six or more spaces **between two non-space characters**.

**What it deliberately does not cover**, stated in the module doc so nobody mistakes silence for
coverage:

- comments and doc comments — skipped outright;
- raw strings (`r"…"`, `r#"…"#`) — a raw string cannot use the continuation escape, so a wrapped one
  holds a real newline and every space in it was typed on purpose;
- literals of 120 characters or fewer. This is the load-bearing exclusion and it is measured: at >120
  the tree has 29 candidates, at no length limit it has 113, and the 84 extra are column templates
  (`"  Adjustments (L10):        {}"`), indentation prefixes and code generation;
- runs of five spaces or fewer — sentence typography, not a source indent;
- padding at either END of a literal (a fixed-width cell) — the gap is looked for in the body only;
- anything assembled at RUNTIME. This reads source: a gap built from two `format!` arguments is
  invisible here.

### The kill and its observed red

Four tests, all green on the committed tree. The live plant put `return_refuse.rs:2498` back to its
`db591e10` mangled form, verbatim:

```
$ cp …/return_refuse.rs …/scratchpad/return_refuse.rs.bak
$ python3 …  # PLANTED
$ touch crates/btctax-core/src/tax/return_refuse.rs
$ cargo run -q -p xtask -- wrapped-literals
xtask wrapped-literals: FR-108 — 1 string literal(s) print with the wrap indentation still inside the
quotes. Wrap them with a `\`-newline continuation, which removes the next line's leading whitespace
(so the space goes BEFORE the backslash):
  crates/btctax-core/src/tax/return_refuse.rs:2498 — a 18-space gap inside a one-line literal:
  …ported on Form 1099-NEC box 1,                  1099-MISC box 3 or 109…
exit=1

$ cargo nextest run -p xtask -E 'test(no_committed_literal)'
    thread '…::no_committed_literal_carries_its_wrap_indentation' panicked at
    crates/xtask/src/wrapped_literal_check.rs:381:23:
    FR-108 — 1 string literal(s) print with the wrap indentation still inside the quotes …
      crates/btctax-core/src/tax/return_refuse.rs:2498 — a 18-space gap inside a one-line literal …
     Summary 1 test run: 0 passed, 1 failed, 176 skipped

$ cp …/return_refuse.rs.bak crates/btctax-core/src/tax/return_refuse.rs
$ touch crates/btctax-core/src/tax/return_refuse.rs      # FR-90
$ cargo run -q -p xtask -- wrapped-literals
wrapped literals: 356 sources scanned, no one-line literal over 120 chars carries a 6-space gap
exit=0
```

**B1a — the in-test fixture is asserted to present the case.** `the_check_reds_on_a_baked_in_wrap_and_
not_on_its_near_misses` measures the plant's own properties *before* using it as a plant: one source
line, over `MIN_LEN` characters, carrying a `MIN_RUN` gap, and no newline escape. A plant that drifted
under the threshold would otherwise make every assertion in the test pass for the wrong reason.

**And it was watched staying green on seven near misses**, because a checker that reds on everything is
deleted by the next person who trips it: the same sentence wrapped with a real `\`-continuation (the
fix — the check must accept it or the fix does not converge), an aligned multi-line display block, a
short column template, the identical gap inside a comment, a raw string, a five-space run, and a run at
the edge of a literal.

`a_quote_inside_a_comment_a_char_literal_or_a_raw_string_does_not_blind_the_scan` plants a real finding
*after* each of six scanner hazards (line comment, nested block comment, char literal, raw string with
hashes, lifetime, escaped quote) and asserts it is still seen — the false-GREEN direction, which is the
one that matters.

---

## Task 2 — FR-109 / FR-105

**Owner's ruling, verbatim:** *"For 109, don't re-ask unless a command line option to re-answer
questions is present."*

### What changed, and where

All in `crates/btctax-cli`:

| where | what |
|---|---|
| `src/cmd/answer.rs` | `AskScope { StillNeeded (default), Every }`, `AnswerOptions { discard_draft, scope }`, `answer_key_of`, `needs_asking`, the sweep filter, the two header sentences, the "Nothing to ask" line |
| `src/cli.rs` | the `--re-answer` flag and its help |
| `src/main.rs` | flag → `AskScope` |
| `tests/{year_gate_t4,open_next_year_t4b,step0_panel,tax_report}.rs` | 14 call sites take `AnswerOptions::default()` (plus 2 more inside `cmd/answer.rs`'s own tests: 16 in all) |
| `docs/man/btctax-income-answer.1` | regenerated by `make docs` |

`AnswerOptions` is a struct rather than an eighth parameter because clippy's `too_many_arguments`
fires at eight under `-D warnings`; the function already took seven.

**The skip rule**, `needs_asking` (`cmd/answer.rs`), drawn by `answer_status` (`provenance.rs:965`):

| status | verdict |
|---|---|
| leaf unanswered (`NeverAsked`, empty leaf) | **ASK** — unchanged |
| `WordingChanged` | **ASK**, flag or no flag |
| `NeverAsked` with a value (the imported TOML) | **ASK, once** — asking stamps a record, so it converges |
| `Given` | **SKIP** |
| `Declined` | **SKIP** |

★ **The skip is a CONJUNCTION, and that half was not in the brief.** For a class-(A) ask the leaf must
*also* hold what the record says was given. A `Given` record standing over an empty declaration is a
record of testimony the return does not carry, and skipping there is the one outcome worse than
re-asking: `screen_inputs` refuses the commit, R12 lists the question as blocking, and the command that
exists to fix it declines to ask — a **brick**, the same shape as the T7 seam review's starved
dependent row. *Fail-closed on the claim, fail-OPEN on the interview.* A class-(B) skippable takes no
leaf test, because for it an empty leaf IS a lawful answer (`Declined`). This conjunct has its own
kill (#5 below) and was watched red.

**The sweep still terminates.** A skipped ask is not inserted into the session's `asked` set, but
nothing in the loop can turn a `Given` back into a `NeverAsked`, so the same question is filtered out
again next pass and `round` empties. The one direction that *does* change mid-session is toward asking
**more** — an answer given now can reword a question that quotes it (R10.4) — and the next sweep picks
that up.

**The FR-105 sentence was rewritten, because it is now false.** It said *"every question that applies
to this return is asked again each time — this command does not skip the ones already on file."* A
filer-facing sentence left standing over a mechanism that moved underneath it is the FR-108 class in
prose. It is now per scope:

```
StillNeeded:  (only what this return still needs is asked: a question already answered — in the words
               it is asked in now — is skipped. A value that arrived by `income import` is asked once,
               so that it gets a record. Press Enter to keep an answer shown, or re-run with
               `--re-answer` to be put through every question again.)

Every:        (`--re-answer`: every question that applies to this return is put again, including the
               ones already answered. Press Enter to keep the answer shown.)
```

And a session that asks nothing now says so, rather than printing two panels and going quiet:

```
Nothing to ask: every question this return needs is already answered, in the words it is asked in now.
Run `btctax income answer --year 2024 --re-answer` to be put through all of them again.
```

`btctax income answer --help` confirms the flag, its default, and that it changes nothing on its own.

### The five kills, and each one's observed red

All five drive the **real** `answer_return_inputs` from a keystroke script and read the screen — never
a re-implementation of its loop, which is the T7 seam-review lesson. The fixture is a TY2024 draft with
two dependents, so it covers both halves of what FR-105 measured (declarations *and* §152 gates); the
first pass on it puts **69** questions.

The prompt counter reads `]: ` off the screen and is **cross-checked against the derived live set** in
kill 2, so it cannot be measuring something nobody has tied to a prompt.

| # | test | asserts |
|---|---|---|
| 1 | `a_fully_answered_return_asks_nothing_and_says_so` | EMPTY stdin, default scope: 0 prompts, and the screen says `Nothing to ask:` and names `--re-answer`. Before FR-109 an empty stdin could only end in *"input ended before every question was answered"*, so the empty input IS the kill |
| 2 | `re_answer_puts_every_live_question_again` | `--re-answer` asks 69 = the derived live count = what the first pass asked; the default on the same return asks 0 |
| 3 | `a_reworded_question_is_asked_again_without_the_flag` | one live declaration re-recorded under words never shown (leaf untouched) ⇒ exactly 1 prompt with NO flag, the panel prints `WORDING_CHANGED_REASON`, and answering it settles it back to `Given` |
| 4 | `an_imported_leaf_is_asked_once_and_not_twice` | log cleared off an answered draft (the imported shape): pass one asks 69, pass two asks 0 |
| 5 | `a_record_standing_over_an_empty_class_a_leaf_is_still_asked` | `Given` over an empty declaration ⇒ ASK; the same record with the leaf filled ⇒ SKIP; a `Declined` skippable with an empty leaf ⇒ SKIP |

**B1a** — each fixture is asserted to present its case before it is used as one: #1 measures every live
question `Given`/`Declined` from the log; #3 asserts `WordingChanged` **and** that the leaf value is
unchanged; #4 asserts the log is empty and every live ask reads `NeverAsked`; #5 asserts the leaf is
empty and the status is `Given`.

Four mutations, each reverted with a `cp` backup and a `touch` (FR-90):

**Mutation A — the skip filter removed (the pre-FR-109 behaviour):**

```
$ … .filter(|a| scope == AskScope::Every || true || needs_asking(&ri, a))
        PASS  a_record_standing_over_an_empty_class_a_leaf_is_still_asked
        FAIL  a_fully_answered_return_asks_nothing_and_says_so
    a fully-answered return must not read a single keystroke:
    Usage("input ended before every question was answered — nothing was stored")
        FAIL  a_reworded_question_is_asked_again_without_the_flag
        FAIL  re_answer_puts_every_live_question_again
        FAIL  an_imported_leaf_is_asked_once_and_not_twice
    the second pass must not read a keystroke:
    Usage("input ended before every question was answered — nothing was stored")
     Summary 5 tests run: 1 passed, 4 failed, 241 skipped
```

**Mutation B — `WordingChanged` treated as ANSWERED** (the one plausible "simplification" of the rule):

```
$ … AnswerStatus::WordingChanged | AnswerStatus::Given | AnswerStatus::Declined => …
        FAIL  a_reworded_question_is_asked_again_without_the_flag
    assertion `left == right` failed: exactly the reworded question must be asked, and nothing else
     Summary 5 tests run: 4 passed, 1 failed, 241 skipped
```

Only kill 3 reds — which is what a discriminating kill looks like.

**Mutation C — `--re-answer` ignored:**

```
$ … .filter(|a| needs_asking(&ri, a))
        FAIL  re_answer_puts_every_live_question_again
    `--re-answer` must put every LIVE question — the screen counter and the derived live set
    disagree, so one of them is not measuring prompts:
      left: 0
     right: 69
```

**Mutation D — the class-(A) leaf conjunct removed:**

```
$ … Ask::Declaration(_) => false,
        FAIL  a_record_standing_over_an_empty_class_a_leaf_is_still_asked
    a `Given` record over an empty class-(A) leaf must still be asked — skipping it is a brick:
    the commit refuses and the command that fixes it will not ask
```

Restored after each, `touch`ed, and `make gate` run once at the end from touched sources.

---

## Deviations

1. **"Fix all 34" was not done — 29 were fixed and 5 were left alone**, because measurement refuted the
   premise. The five carry an embedded `\n` escape: they are a shell script, three aligned key/value
   display blocks and a Rust source fixture, and their space runs are the content. Collapsing them
   would have broken three TUI display panels. The derived `\n` predicate is what separates them, and
   it is in the lint rather than in a list. Evidence above.
2. **FR-108's stated mechanism is wrong and is corrected in the entry and in the lint's module doc.**
   A `\`-newline continuation removes the next line's leading whitespace; it is the cure, not the
   disease. Measured with `rustc`, output above. This is reported rather than silently worked around
   because a fix derived from the stated mechanism would not have converged.
3. **A fifth kill was added beyond the four the brief names**, for the class-(A) leaf conjunct — a
   guarantee the brief's skip table does not contain but which the implementation needs, because the
   log and the leaf can disagree and the failure is a brick. B1: a guarantee without a kill does not
   exist.
4. **`FOLLOWUPS.md` was edited in three places** and **no entry was marked resolved** — that is the
   controller's to do at commit time, following the FR-105 precedent. FR-108 gains the measured
   mechanism correction, the 29/5 split and what was built; FR-105 gains a one-paragraph note that the
   sentence it quotes is superseded by FR-109 (leaving a known-false quote on disk is the very class
   this task is about); FR-109 gains the ruling, the status table, the conjunction and the kill names.
5. **`docs/man/btctax-income-answer.1` is regenerated**, by `make docs`. It is the only committed doc
   artifact the flag moved.
6. **16 call sites changed shape** (`false` → `AnswerOptions::default()`) — 14 in the four
   integration test files, 2 in `cmd/answer.rs`'s own test module. None changed meaning:
   every one of those fixtures starts from an unanswered return, where every question reads
   `NeverAsked` and is asked exactly as before. The whole suite passing unchanged is the evidence.
7. **No new follow-up was filed.** Nothing out of scope was found that warranted one.

---

## The instruments

Run after the change. **Every one is byte-identical to the numbers quoted at `db591e10`.**

```
$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
f1040s3:5 f1040sa:21 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12 f8959:17
f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)

$ cargo run -q -p xtask -- census-join
census join: 274 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -q -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91
registry prompts scanned; no forbidden shape

$ cargo run -q -p xtask -- prompt-check
xtask prompt-check: OK — 90 assertions, all verbatim

$ cargo run -q -p xtask -- box-census
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)

$ cargo run -q -p xtask -- wrapped-literals          # new
wrapped literals: 356 sources scanned, no one-line literal over 120 chars carries a 6-space gap
```

**Why none of them moved.** `stop-list`'s 91 prompts and `prompt-check`'s 90 assertions read the
REGISTRY prompts, and FR-109 changed none of them — it changed which of them are *put*, and the two
header sentences and the "Nothing to ask" line are command chrome, not registry entries. FR-108 touched
no prompt, label or caption: the 29 literals are exemption reasons, refusal details, advisories, field
help, geometry errors and test messages. `line-coverage`, `census-join` and `box-census` read the form
extracts and the maps, which neither task touched.

The one number that did move is the suite: **3549 → 3558**, +4 lint tests and +5 FR-109 kills.

## `make docs`

```
$ make docs
… wrote docs/pdf/btctax-what-if.pdf
… wrote docs/pdf/btctax.pdf
$ git status --short docs/
 M docs/man/btctax-income-answer.1
```

One committed man page changed, and it is the `income answer` one. (`docs/pdf/` is git-ignored.)

## Closing gate

```
$ cargo fmt --all && cargo fmt --all -- --check
fmt clean

$ make gate
     Summary [  21.060s] 3558 tests run: 3558 passed, 12 skipped
```

Clippy runs inside `make check` at `-D warnings` with `CARGO_TARGET_DIR=target-clippy`; the target
propagates both statuses and prints `make check: FAILED` on either, and did not.

## Separability

No source file is touched by both tasks.

- **Task 1** — `crates/xtask/src/wrapped_literal_check.rs` (new), `crates/xtask/src/main.rs`,
  `crates/xtask/src/box_census.rs`, `crates/xtask/src/line_coverage_check.rs`,
  `crates/btctax-cli/src/cmd/tax.rs`, `crates/btctax-cli/tests/repo_hygiene.rs`,
  `crates/btctax-core/src/tax/{classifier,provenance,return_1040,return_refuse}.rs`,
  `crates/btctax-forms/src/form8283.rs`, `crates/btctax-forms/tests/full_return_forms.rs`,
  `crates/btctax-input-form/src/spec/sections.rs`, `crates/btctax-tui-edit/src/draw_edit.rs`,
  and the FR-108 hunk of `FOLLOWUPS.md`.
- **Task 2** — `crates/btctax-cli/src/{cli,main}.rs`, `crates/btctax-cli/src/cmd/answer.rs`,
  `crates/btctax-cli/tests/{year_gate_t4,open_next_year_t4b,step0_panel,tax_report}.rs`,
  `docs/man/btctax-income-answer.1`, and the FR-105/FR-109 hunks of `FOLLOWUPS.md`.

`FOLLOWUPS.md` is the only file carrying hunks from both, and they are far apart in the file.
