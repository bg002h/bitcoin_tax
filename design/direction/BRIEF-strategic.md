# STRATEGIC CODEBASE REVIEW — can someone actually file a tax return with this?

**Repo:** `/scratch/code/bitcoin_tax`, branch `main`, ~2026-08-01. ~146k lines of Rust across 12 crates.
**This is NOT a defect hunt.** Several rounds of those have run and the tree is green. This is a
**direction** review of the REAL CODE, and the deliverable is judgment, not findings.

## The goal, in the owner's words

> *"…our goal of being able to file a tax return using CLI and TUI, and knowing we will someday use a
> web UI."*

The product is: **a real person, with real exchange data, produces a real signed US federal return —
today through a CLI and a TUI, later through a web front end.**

## THE ONE QUESTION

**Are we getting closer to that, and what is the shortest honest path to a filer completing a return?**

Not "is the code good." Not "what bugs exist." **Is this converging on a shippable filing tool, or is
it accreting rigour around a core that still cannot take an ordinary person from CSV to signature?**

If the current work is on the critical path, say so and show why. If the last stretch was rigour on a
surface that is not the bottleneck, **say that plainly**. The owner asked for direction; a comfortable
answer is worthless.

## READ THE CODE

Your slice is assigned separately. **Read it, run it, and judge from what is actually there** — not
from the design documents, which describe intent. Where a doc and the code disagree, the code wins and
that disagreement is itself a finding about direction.

Useful entry points: `cargo run -p btctax-cli -- --help`; the six worked journeys in
`docs/examples/examples.md`; the TUI walkthrough goldens in `docs/examples-tui-walkthrough/`; the test
fixtures under `crates/*/tests/`; `crates/btctax-cli/LIMITATIONS.md`.

## Facts — do not spend budget re-deriving these

- **btctax has never had a user.** v0.15.0 is prepared, reviewed, **unpublished**; v0.14.0 is live on
  crates.io. Back-compat is not sacred.
- Crates by size: `-tui-edit` 46k, `-core` 41k, `-cli` 20k, `-tui` 10k, `-forms` 8k, `xtask` 8k,
  `-input-form` 5k, `-adapters` 4k, `-store` 1.4k.
- **Two independent oracles** (OpenTaxSolver, Tax-Calculator) validate tax figures; a sweep reconciles
  admitted households line-by-line against both.
- Full 1040 + Schedules 1/2/3/A/B/C/D/SE + Forms 8949/8283/8275/8959/8960/8995/6251 are modelled and
  fill official IRS AcroForms.
- The engine has a large **refusal** surface — it declines to file rather than guess. Whether that
  surface is *calibrated* is a fair question for you.
- Doctrine is in `CLAUDE.md`: transcribe forms don't paraphrase; blank is the normal case; an entry is
  testimony; conformance ⇒ test, reviews for judgment.
- In flight: **§G-11** (`design/no-testimony/`) — making "not stated" expressible in the money path. A
  coverage checker landed (179 money lines, 14 forms); the type migration has not started.
- Open work: `FOLLOWUPS.md` (§G-*) and `CONTINUITY.md`.

## Output format — follow exactly

1. `VERDICT:` one sentence — converging, or not, and on what.
2. `WHAT I READ` — the files and commands. Be specific; this is how the synthesis weighs you.
3. `STATE OF MY SLICE` — what genuinely works end-to-end, what is scaffolding, what is absent. Name
   files and line numbers.
4. `THE BOTTLENECK IN MY SLICE` — the single thing most preventing a real filer from finishing.
5. `WEB-UI READINESS` — could this slice serve a non-terminal front end? What is coupled to the CLI/TUI
   that should not be? Be concrete about the seam.
6. `WHAT I WOULD DO NEXT, IN ORDER` — 3–6 concrete moves, each with why and a rough size.
7. `WHAT I WOULD STOP DOING` — **mandatory, must not be empty.** Work underway or planned that you
   judge is off the critical path. If you truly believe nothing should stop, argue it explicitly.
8. `WHAT WOULD MAKE THIS ASSESSMENT WRONG` — the assumption you did not verify.

Prefer *"a filer with a Coinbase CSV and a W-2 gets as far as X and then hits Y at `file.rs:NNN`"* over
abstractions.

**READ-ONLY: no edits, no commits, no branch mutation.** Do not spawn subagents.
