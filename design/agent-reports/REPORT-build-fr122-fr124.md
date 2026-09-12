# REPORT — FR-122 and FR-124

**Builder:** one opus agent, shared main tree, HEAD `409cc053` at start. Nothing committed.
**Date:** 2026-09-12. **Brief:** `design/agent-reports/BRIEF-build-fr122-fr124.md`.

> ★★ **PROVENANCE — this file is a CONTROLLER TRANSCRIPTION, not an agent-written artifact.** The builder's
> harness refused its report write (`Write` returned *"Subagents should return findings as text, not write
> report files"*) — **FR-129 recurring, second occurrence in two days.** Per the brief it said so first and
> returned the text rather than skipping it. So "the responder must not be the scribe" is unsatisfied for
> this file; the byte-exact original is the agent's task output. Deliberate transcription edits: notification
> HTML entities `&gt;`/`&lt;` restored to `>`/`<`.

**Headline:** FR-124 was real and is closed, with a stronger fix than the entry named. **FR-122's premise is
REFUTED** — the test the brief asked me to write has existed since 2026-07-31 and `make gate` runs it — but
disproving it by mutation found a *different*, real gap in the same instrument, which is closed.

**Gate:** `make gate` → **3609 tests run: 3609 passed, 12 skipped**, exit 0. `cargo fmt --all --check` clean.

---

## 0. Refuted premises

### R1 (blocking, FR-122's whole premise) — `line_coverage_check::run()` IS run by the suite

The brief and the FR-122 entry both state that `run()` is *"reachable only from `cargo run -p xtask --
line-coverage`"*, that *"the 377-row table's verbatim check against `design/forms/extract/` is unheld"*, and
that the answer to *"which test reds when this checker is removed?"* is *"none, for the table as a whole"*.
**All three are false.**

`crates/xtask/src/line_coverage_check.rs` has carried this since commit **`3313ecf4`, 2026-07-31 22:38
−0700** — six weeks before FR-122 was filed (`6356043e`, 2026-09-11 19:33):

```rust
    /// The checker passes on the committed table.
    #[test]
    fn the_committed_coverage_table_is_consistent_with_the_form_text() {
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => panic!("{e}"),
        }
    }
```

It is in the gate's own inventory (`cargo nextest list -p xtask`), and it discriminates. **Brief kill 1, run
against the pre-fix tree** — a rotted cross-reference planted in one row (`cover_form8995`, Form 8995 line 2:
*"Combine lines 1i through 1v"* → *"1x"*):

```
FAIL [   0.061s] (6/7) xtask::bin/xtask line_coverage_check::tests::the_committed_coverage_table_is_consistent_with_the_form_text
  line-coverage FAILED (1 problem(s)):
    - f8995:2 (line2) quotes text NOT FOUND in f8995--2024.txt:
        "Total qualified business income or (loss). Combine lines 1i through 1x, column (c)"
```

The brief said *"Today nothing reds."* Today that reds, naming the row. **Brief kill 2 (anti-vacuity, empty
table) also already existed**, at `line_coverage_check.rs:2313-2318`, backed by `check`'s own guard at
`:1013` (*"line_coverage::all() is EMPTY — the checker would vacuously pass"*).

★ **The fact was already written down in this repo.** `design/ty2025/reviews/PLAN_schedule_1a-buildability-r4.md:75`
says: *"`xtask line-coverage` — enforced on every commit via
`line_coverage_check::tests::the_committed_coverage_table_is_consistent_with_the_form_text`, which runs inside
`make check`"*. So the entry contradicted a committed review.

**Why this matters beyond the item.** This is the arc's own dominant defect class turned on the follow-up
writer: *a claim about what an instrument covers, written without running the instrument.* Added as a row to
`FOLLOWUPS.md` FR-99's table.

**What of FR-122 survives:** only the narrower true statement that no **CI job** invokes the CLI subcommand —
`grep -rn 'line-coverage' .github Makefile` is still empty (verified). Immaterial: the test exercises the
same `run()`.

### R2 (not blocking) — the `exact` empty-run guard is a message, not a floor

My first draft of `build_calculator` claimed an empty row list *"reports clean"*. Measured false: taxcalc
refuses it itself with `ValueError: data missing one or more MUST_READ_VARS`. The guard is now documented as
making the refusal legible at the call site, nothing more (`taxcalc_exact.py:123-131`).

---

## 1. FR-124 — the inert `exact` flag ✅ CLOSED

### The premise, confirmed by direct measurement

```
exact AFTER advance_to_year, as a DataFrame column: [0]
exact AFTER calc_all, as a DataFrame column      : [0]
exact written through the Calculator             : [1]
```

(taxcalc 6.8.2, `.venv/bin/python`.) So `TAXCALC_EXACT_YEARS = frozenset({2025})` was inert, as filed.

### What changed, and why that mechanism

**New file `scripts/oracle/taxcalc_exact.py` (295 lines) is the one construction path.** The brief's ★★ point
drove this: `verify_schedule_1a.py` already had the *correct* implementation, so patching `gen_goldens.py` in
place would have left two implementations of one procedure — the divergence that produced the defect,
re-armed.

| thing | where | why that mechanism |
|---|---|---|
| `apply_exact` | `taxcalc_exact.py:81` | writes through the Calculator — the only route that sticks |
| `assert_exact_stuck` | `:94` | asserted **after `calc_all`**, on the value the engine actually held, and two-directional (`== want * n`): `sum == n` alone is satisfied by a run that always writes 1, and `sum == 0` alone by the original defect |
| `build_calculator` | `:112` | Records → Calculator → `advance_to_year` → `exact` → `calc_all` → assert, in one place |
| ★★ the refusal | `:132-142` | **the historical defect is now inexpressible**: a row dict carrying an `exact` key is refused by name, citing FR-124. A comment is guarded by the next reader; a refusal is guarded by the interpreter |
| `needs_exact` / `EXACT_OFF_YEARS` | `:76` / `:69` | see below |

**All five Calculator construction sites in the repo are routed through it** (five, not two —
`grep -rn 'tc.Calculator'`): `gen_goldens.taxcalc_run` (`:357`), `gen_goldens.taxcalc_credits` (`:325`),
`gen_goldens._taxcalc_amt_credits` (`:426`), `verify_schedule_1a._taxcalc_applied` (`:730`, the
previously-correct one), and `verify_f6251._taxcalc` (`:246`, TY2024 so behaviour-neutral; routed so a fourth
hand-rolled block cannot re-introduce the defect when those vectors are ported to TY2025+).

`_taxcalc_row`'s `**({"exact": 1} if year in TAXCALC_EXACT_YEARS else {})` is deleted; `TAXCALC_EXACT_YEARS`
no longer exists, and `gen_goldens.py:195-206` records what was there and why it moved.

### ★★ The year list is gone, not extended — which closes R25 too

The entry's named fix leaves `frozenset({2025})` standing. Measured from taxcalc's own
`policy_current_law.json`: 2024 has `TipIncomeDed_c / OvertimeIncomeDed_c / AutoLoanInterestDed_c` all 0.0
(no stepped provision); **2025, 2026, 2027 and 2028 are all stepped**; 2029 returns to 0.0.

So the ON-list was **already three years stale** — `TY2026_PORT_REPORT.md` R25, rated LIVE/URGENT. Extending
it re-arms the same class. Instead the polarity is reversed: `exact == 1` means *"compute as the printed tax
forms do"*, every comparison in this repo is against a printed form, so ON is right for every year, and
`EXACT_OFF_YEARS = frozenset({2024})` names only the years whose goldens were **already baked** with it off.
That set is **closed by history** — every generation from this commit forward turns it on — so no future year
can need an entry, and a year bump gets the form-faithful branch by doing nothing.

I derived the list first and rejected that: a `_po_step_size`-driven derivation is not clean, because taxcalc
has four such parameters (`II_em_po_step_size`, `CDCC_po1/2_step_size` are unrelated) and the cap naming is
irregular (`II_em_c` does not exist). Stating a closed boundary is `CLAUDE.md`'s option 3 and is honest; a
derivation over an irregular namespace would have been false precision.

★ **R25 is updated in `design/TY2026_PORT_REPORT.md:446` from LIVE/URGENT to CLOSED**, with the mechanism and
the kill.

### Kills — all run, with the red pasted

Permanent kill: `.venv/bin/python scripts/oracle/taxcalc_exact.py --selftest`. Green:

```
taxcalc_exact: `exact` sticks in both directions, an input column is refused by name, and 44
fractional-step vector(s) move off the smooth fallback (taxcalc 6.8.2) — B1 kills OK
```

**Kill 1 — it stuck.** Plant A: FR-124's shipped code restored verbatim inside `build_calculator`.

```
RuntimeError: taxcalc's `exact` is 0 across 1 row(s) where 1 was written (want=1 per row). `exact`
is a CALCULATED variable: a column named `exact` in the Records DataFrame is silently dropped, and
every stepped phase-out then takes the SMOOTH marginal-rate fallback — up to $100 (Schedule 1-A
Parts II/III) or $200 (Part IV) per return, reading as a btctax rounding defect. See FR-124.
```

…and the same plant reds the live census, not only the selftest:

```
INCONCLUSIVE — Tax-Calculator 6.8.2 could not run these vectors (taxcalc's `exact` is 0 across 138
row(s) where 138 was written …).
FAIL: 1 unexpected divergence(s) across both censuses.
```

**Kill 2 — ★ the BRANCH, not the flag.** The selftest drives the same vectors **twice**: once through
`build_calculator`, once through `_smooth_path_calculator` (`taxcalc_exact.py:154`), a local reconstruction of
the broken path. It demands the first **differ** from `verify_schedule_1a._smooth_fallback()` and the second
**equal** it — reused, not restated. Both legs assert, so neither can pass for an unrelated reason. With every
flag-level check stacked out (plants A + B + B2/B3):

```
AssertionError: II/single/+$1 over: taxcalc returned the SMOOTH value 24999.9 through
build_calculator — the stepped branch never ran, which is FR-124
```

Plants A+B alone produce **48** `★ taxcalc returned the SMOOTH value — the stepped branch never ran` findings
in `verify_schedule_1a.py`, plus `FAIL: Part(s) IV have NO witnessed vector that exercises the phase-out` and
`FAIL: 97 unexpected divergence(s)`.

★ One correction the measurement forced: the branch kill's first draft filtered only on `_discriminating` +
`_pins_the_arithmetic` and failed on `IV/qss/+$1 over`, where taxcalc returns the full $10,000 cap because
**FR-126** gives a QSS the MFJ threshold — the engine never enters the phase-out, so "smooth vs stepped" is
not the question there. The filter now also excludes vectors taxcalc is disqualified on, using the census's
own `_taxcalc_predicted(...)[1]`; nothing is hand-listed.

**Kill 3 — the year decision cannot go inert.** Plant C: `needs_exact` reverted to the shipped hand-typed
ON-list. → `AssertionError: TY2026 must compute as the forms do (R25: an ON-list went stale here)`

**Kill 4 — the refusal.** Plant: the offender check deleted. → `AssertionError: an `exact` input column was
ACCEPTED — FR-124 is re-armed`

**Kill 5 — anti-vacuity on the branch kill's own population.** Plant E2: the vector list truncated to three.
→ `AssertionError: only 3 vector(s) tell the stepped branch from the smooth one — the kill below would pass
by comparing nothing…`

**Kill 6 — the empty run.** Plant D: the `not rows` guard deleted → red, but from taxcalc
(`ValueError: data missing one or more MUST_READ_VARS`), which is R2 above. Recorded as a legibility guard,
not a floor.

Every plant reverted from a `cp` backup and verified by `md5sum`.

### Behaviour-neutrality, measured

- **The baked TY2024 corpus does not move.** `taxcalc_run` + `taxcalc_credits` + `_taxcalc_amt_credits` over
  all **107** households, serialised: md5 `c474d1f7220f961f5de217335f2aa2de` **before and after** —
  byte-identical.
- `verify_schedule_1a.py` full output: `diff` against the pre-change run is **empty**;
  `OK: 0 unexpected divergence(s)`.
- `verify_f6251.py`: `OK: every filing status in the fixture clears the Tier-2 attach gate`.
- `check_return.py --selftest`, `check_determinism.py --selftest`: both pass. Every oracle module still
  imports. No golden was regenerated; `gen_goldens.main()` was never run.

---

## 2. FR-122 — the whole-table check ✅ CLOSED, on the gap that was actually there

Premise refuted (§0 R1). The gap found while disproving it:

**Every rule inside `check` is per-row, so a DELETED row is invisible to all of them.** Measured: deleting
one `c.line(...)` — `cover_form8995`, Form 8995 line 3, the prior-year QBI-loss carryforward — took the table
377 → 376 and left the full gate at **3609 passed / 12 skipped**, identical to baseline.
`cover_fns_not_registered` holds whole coverage *functions*; `missing_cover_fns` holds money-bearing *types*.
Neither speaks about lines. Same false-completeness shape as the module header, one level finer.

**Fix:** `MIN_MONEY_LINES = 377` (`line_coverage_check.rs:427`), an inverse ratchet asserted in the existing
whole-table test (`:1831-1839`).

- **`#[cfg(test)]`, at module scope beside the other ratchets.** It belongs to the suite, not to `check`:
  `check` takes any table and every planted-defect table in `mod tests` is one row long, so a floor inside it
  would red all of them. Same placement and reason as `forge_reach_check::FILE_FLOOR`. (Without
  `#[cfg(test)]`, clippy reds `constant MIN_MONEY_LINES is never used` — caught by the gate, not a reviewer.)
- **Pinned AT the measured count, compared with `>=`.** Growth is silent (378 clears 377, so the TY2026 port
  adds line-sets without touching this) and any shrink reds.
- ★ **A loose floor was tried first and measured useless.** At `MIN_MONEY_LINES = 370` the one-row deletion
  still **PASSED** — one row is exactly the size of the defect. That measurement is in the doc comment so the
  next author does not re-loosen it.
- **Stated residual, in the source:** an addition followed later by a deletion nets back to the floor and
  passes. Closing it needs `==`, which reds on every honest addition; the floor is raised opportunistically.

### Kills

**Row deletion:**

```
the coverage table has 376 money lines, below the 377 floor. Every rule in `check` is per-row and
none can see a row that is GONE, so a printed line of a filed form can leave the census with the
report still saying OK. If the deletion is deliberate, say which lines and why, and lower the floor
in the same diff.
```

**Extract set unreadable** (brief kill 2's second half). Plant: the extract path pointed at
`design/forms/extract-GONE/` → `line-coverage FAILED (55 problem(s))`, each naming a form with neither an
extract nor a map. (And with `repo_root()` bogus: `cannot read line_coverage.rs: No such file or directory`.)
So an unreadable extract set cannot pass by checking nothing — `MAX_UNVERIFIABLE = 0` is what makes that
true, and it is now watched.

### ★ Cost of the added check, measured (brief kill 3)

Three runs each of `cargo nextest run -p xtask -E 'test(the_committed_coverage_table)'`: with the floor
0.064 / 0.065 / 0.064 s; with the floor's `all()` call removed 0.064 / 0.068 / 0.066 s.

**The whole-table check costs ~65 ms and was already in `make gate`. The floor's marginal cost is below
run-to-run noise** — one extra `line_coverage::all()`. `make gate` wall time: 21.350s → 21.067s. Nothing to
relocate.

---

## 3. Files changed

| file | what |
|---|---|
| `scripts/oracle/taxcalc_exact.py` | **new**, 295 lines — the single authority on building a taxcalc run and on `exact`, plus `--selftest` |
| `scripts/oracle/gen_goldens.py` | `TAXCALC_EXACT_YEARS` and the `exact` row-dict key removed; three Calculator sites routed |
| `scripts/oracle/verify_schedule_1a.py` | imports `taxcalc_exact`; the hand-rolled Records/Calculator/`exact`/assert block replaced by `build_calculator` |
| `scripts/oracle/verify_f6251.py` | imports `taxcalc_exact`; its Calculator site routed. Behaviour-neutral (TY2024) |
| `crates/xtask/src/line_coverage_check.rs` | `MIN_MONEY_LINES` + the floor assertion in the existing whole-table test |
| `FOLLOWUPS.md` | FR-124 closed; FR-122 rewritten (premise refuted + the real gap closed); one row added to FR-99's table |
| `design/TY2026_PORT_REPORT.md` | R25 LIVE/URGENT → CLOSED |

`crates/btctax-core/src/tax/line_coverage.rs` is **unmodified** — all plants restored and md5-verified
(`454266d586aad536213a51fbcd74d85e`).

## 4. Residue, with owning phases

1. **`MIN_MONEY_LINES`'s add-then-delete residual.** Nit, stated in the source. *Ownerless — record only.*
2. **Raise `MIN_MONEY_LINES` when the table grows.** Nit. *Owning phase: the TY2026 port.*
3. **`design/` line-number citations into `gen_goldens.py` shifted.** Nit, FR-60's class. *Ownerless — batch
   with FR-60.*
4. **The `line-coverage` CLI subcommand is still in no CI job.** Nit, now immaterial. *Record only.*
5. **FR-99's table gained a row** for the FR-122 shape. *Owning phase: the harness / doctrine (owner).*
6. **FR-129 recurred** — this report's `Write` was refused, so it is returned inline. *Ownerless.*

Not touched, per the brief: the corpus's year (FR-128), FR-132, the params bundle, the fail-closed gates.
