# Brief — fold the T11 seam review (2C / 4I / 3M / 1N)

You are folding an independent seam review into the **main working tree** of `/scratch/code/bitcoin_tax`,
at `HEAD` on branch `main`. You are the only agent editing this tree. **Commit nothing** — the controller
commits through the pre-commit gate when you return. Do not push, do not `git stash`, do not spawn
subagents, and never `git checkout -- <file>` over your own uncommitted work (keep a `cp` backup).

Read first, in this order:
1. `design/agent-reports/2026-09-07-build-interview-T11-review.md` — the review, verbatim.
2. `design/agent-reports/2026-09-07-build-interview-T11-review-VERIFICATION.md` — the controller's ledger.
   **C-1, C-2, I-2 and I-4 are machine-confirmed at source level by the controller**; I-1, I-3 and the
   Minors are accepted on the reviewer's evidence. Do not re-litigate whether a finding is real.
3. `design/agent-reports/2026-09-07-build-interview-T11-implementation.md` — the build you are correcting.
4. `CLAUDE.md` at the repo root — **"Two oracles, and the `.venv`"** binds this fold hardest.

**A fold is authorship and re-earns the build gate.**

## ⚠ FR-90 — a stale `target/` can make your measurements lie

`cargo`/`nextest` silently reused artifacts across plant → measure → restore cycles during T10's fold and
produced a **phantom red at a clean HEAD**. **`touch` the file after every plant AND after every
restore**, rebuild the harness binary explicitly after touching anything it links, and
`find crates -name '*.rs' -exec touch {} +` before the closing gate. Re-run anything surprising after a
touch before recording it.

## Validation

Main tree ⇒ leave `CARGO_TARGET_DIR` unset; `CARGO_TARGET_DIR=target-clippy` for clippy. Scoped runs
while you work; `make check` once at the end (~20 s; 3506 tests at HEAD). Instruments at HEAD:
`line-coverage` 375/18/31 (ratchet 31)/0/17; `census-join` 274 across 13 maps; `stop-list` 8+4 sources,
91 prompts; `prompt-check` 88; `box-census` 268/19/9.

Oracles: taxcalc 6.8.2 + pandas in `.venv/bin/python`; OpenTaxSolver at
`~/OpenTaxSolver2024_22.07_linux64` (`OTS_DIR` unset by default — export it).
`check_return.py --selftest` runs the offline B1 kill with no OTS, no taxcalc and no vault. **Re-run
both engines over the committed corpus before you finish and confirm neither baseline moved.**

---

## ★ Fold the ROOT, not the four instances

**C-2 and I-1 are one defect.** The completeness partition is derived over `Usd` **leaves**, so anything
that is not a `Usd` leaf is invisible to it: the itemize election (a fact that *routes* money to
Schedule A) and a charitable gift's *class* (`Cash60` vs the rest) both fall straight through. Deleting
the class filter entirely leaves all 9 T11 KATs plus `golden_returns`/`kat_tax` green — that is the
partition failing, not the tests.

**Extend the derivation past `Usd` to the facts that route money**, and let a planted routing fact that
no oracle is told go red. Then C-2 and I-1 are consequences you fix once, with one kill that holds both.
If you cannot derive the full set, say so plainly and state exactly what the assertion does cover.

## C-1 (Critical) — the harness reconciles a return `btctax report` refuses

Confirmed: `build_golden_return` (`testonly.rs:1557-1559`) sets `foreign_accounts = Some(false)`,
`foreign_trust = Some(false)` and calls `answer_all_live_declarations`; `check_return.py:196` hands it
the projected row; `cli.rs:512` calls the command *"how a REAL return reaches an oracle."* So the
verification instrument answers the filer's FBAR and declaration gates on their behalf and then reports
success on a return that does not file. The docstring's own `Exit 2 = … a refused return` cannot fire.

**Fix** (the reviewer's, and it avoids a second computation path): `income project` runs
`screen_inputs` / `screen_absolute` on the **loaded** `ReturnInputs` and emits a `"refused"` block in the
wrapper naming the screen that fired; `check_return.py` exits 2 when it is present. Then correct the
docstring **and** `cli.rs`'s help to say plainly that the compared figures are the *projected
household's* — the check bounds the description, not the packet.

★ Do not fix this by making `build_golden_return` stop answering; the corpus depends on it. Fix the
**claim** and add the screen at the point the filer's own return is in hand.

**Kill.** A vault whose return `btctax report` refuses must make `check_return.py` exit 2 naming the
screen. Plant the `"refused"` block away → the run reconciles at exit 0 again → red.

## C-2 (Critical) — no projected row can drive OpenTaxSolver's Schedule A

Confirmed: `ots_direct.py:573` and `:687` gate every Schedule A line on
`standard_or_itemized == "Itemized"`; `corpus.py:164` notes it is *"not a GoldenInputs field"*;
`grep -c standard_or_itemized crates/btctax-core/src/tax/testonly.rs` → **0**. Every itemizing filer is
handed to OTS with no Schedule A: line 12 falsely DIVERGES, the run exits 1 telling the filer to
adjudicate against the form, and line 5e loses its OTS witness while the census claims only one engine
models it.

**Fix.** Add the election to `GoldenInputs` and set it in `project_to_golden` from btctax's own line-12
decision — `printed.rs:796` is already that quantity — and let `build_golden_return` ignore it as the
corpus does. **Kill:** the reviewer's itemizing household must reconcile on line 12 with **both**
engines; plant the field away → the false DIVERGES returns.

## I-2 (Important) — the credit excuse cannot tell a blank from an asserted `0`

Confirmed: `check_return.py:280` collapses `None` and `"0"` to the integer `0` under a comment saying
they are different; `credit_line_verdict` then reduces to `ok ⟺ btctax_value == 0`; and
`advisories.rs:937-939`'s `ctc_odc_line19` **does** print `Some(Usd::ZERO)` whenever `ctc_provably_zero`
fires.

★★ **This is the second half of a hole neither task owns.** T8's I-2 / **FR-85** established that the
§24(h)(2) ceiling is a year-blind $2,000 and that when TY2025's package lands `ctc_provably_zero` will
swear a `0` on line 19 for a household that still has credit. **This instrument is the one thing that
could catch that, and today it excuses it.** The reviewer planted the understated ceiling and watched
the btctax column move `blank` → `0` against an oracle computing $1,000 with the verdict unchanged.

**Fix.** Pass `printed` (the `Optional`) into `credit_line_verdict`: `None` ⇒ the forgo excuse (the gap
must equal the whole oracle credit); `Some(v)` ⇒ btctax **asserted** a figure and it must equal
`round(oracle_value)`, diverging otherwise. Fix the `selftest`'s third case at the same time.

**Kill.** Understate `CTC_PER_CHILD_SS24H2`, rebuild the harness (**touch first**), and the run must now
exit 1 naming line 19. Restore, and it must reconcile. Add that as a `--selftest` case so it runs
offline forever, and cross-reference FR-85 in the source so the two halves are linked.

## I-3 (Important) — the witness census prints a false mechanism

Four lines Tax-Calculator demonstrably computes are reported *"only one engine models this line"*
(measured `additional_medicare_tax = 1530.0` on one of them). A census that misstates why a line has one
witness is worse than none: the whole point is to know where two-oracle cover is genuinely absent.
Derive the witness set from what each engine actually returned for **this** run rather than from a
static table, and make a line that gains a witness stop claiming it has one.

## I-4 (Important) — HoH / MFS / QSS project a row that panics three steps later

Confirmed: `project_to_golden` (`testonly.rs:1826-1829`) maps only `Single` and `Mfj`, falling through
`other => format!("{other:?}")`; `testonly.rs:1363` panics on the result; `TAXCALC_MARS`
(`gen_goldens.py:119-125`) keys `"Married/Sep"`, `"Head_of_House"`, `"Widow(er)"`. **Three of five
filing statuses have no T11 gate at all**, `income project` exits 0 anyway, and the failure surfaces as
a Rust panic.

This is a hand-typed match beside a derived enum, written against a status set **T8 had just widened**.
**Fix:** map all five, `_`-free so the compiler catches the sixth. If a status genuinely cannot be
projected, `income project` must say so and refuse — never emit a row that panics downstream.

**Kill:** an exhaustive match plus a per-status round-trip; plant a status back to the Debug
fall-through → red.

## M-1, M-2, M-3, N-1

- **M-1** the ledger census is a hand list of two with no derivation and no completeness kill — the same
  family as the root above; derive it or state what it does not cover.
- **M-2** `check_return.py --year` silently overrides the wrapper's own `tax_year`. Make the mismatch
  loud.
- **M-3** seam 6's §G-9 statement is not written down. Write it.
- **N-1** the `[taxcalc]`-labelled deeper-line rows print `class: agree-ots`.

## Standing lessons that bind this fold

- **Two oracles, never one** — and a line whose second witness is structurally absent is a
  *single-witness* line that must say so truthfully.
- **A value the oracles take as INPUT is never validated by their agreement** (§G-9).
- **No decision keys on a list you typed beside derived data** — that is I-4, and half of C-2.
- **A blank is not a zero.** An entry is testimony; `0` is testimony, a blank is none. That is I-2.
- **A kill CALLS the instrument**, and a fixture decides what a checker can see (FR-88).

## Report — your FINAL action

Write `design/agent-reports/2026-09-07-build-interview-T11-fold.md`: Commands with real output; one
section per finding (What changed / Where / The kill and its observed red / Anything decided differently
and why); an explicit statement of what the extended completeness derivation now covers **and what it
does not**; whether both oracle baselines still hold; a **Deviations** section; the five instrument
outputs; the closing `make check` summary line. Then return ONLY a 3-line summary plus the path.
