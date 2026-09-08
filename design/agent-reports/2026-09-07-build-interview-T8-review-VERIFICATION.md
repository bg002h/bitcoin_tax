# Ledger — controller machine-check of the T8 seam review

Every measurable claim in `2026-09-07-build-interview-T8-review.md` (persisted verbatim at `4baf159c`),
re-run **independently by the controller** before anything is folded. Report counts: `C=0 I=3 M=2 N=1`.

Baseline: `make check` at `121c8805` in the main tree — **3449 passed, 12 skipped, clippy clean, exit 0**.
So no red below is pre-existing.

Worktree hygiene: the reviewer's worktree carried no change but the report itself (`git status --short`
showed one untracked file), i.e. every plant was reverted. The controller's own plants were made in a
throwaway detached worktree, reverted, and the worktree removed; `git worktree list` is back to one entry.

---

## Verdict per finding

| # | Claim | Controller's check | Result |
|---|---|---|---|
| **I-1** | Row (5)(b) prints CHECKED under an unchecked (5)(a); invisible and unclearable | re-derived from source **and re-run** | **CONFIRMED** |
| **I-2** | The FR-85 CTC pin cannot red on the defect it documents | plant re-run; both halves | **CONFIRMED** |
| **I-3** | `hoh_qualifying_child_name` has no reader on any year; QSS excluded | greps + source | **CONFIRMED** |
| **M-1** | The "byte identical" test is a determinism tautology | source | **CONFIRMED** |
| **M-2** | `prompt-check` reads only quoted spans; the operative clause can drift | plant re-run | **CONFIRMED** |
| **N-1** | The caption check asserts presence, not ORDER | source + extract | **CONFIRMED** |

Commands block: all four instrument outputs reproduce **exactly** as the report states them at HEAD —
`line-coverage` 373 money lines / 18 forms / 31 exceptions (ratchet 31) / 0 unverifiable; `census-join`
290 unmodeled entries across 13 maps; `stop-list` 8 + 4 sources, 86 registry prompts, no forbidden shape;
`prompt-check` OK — 86 assertions, all verbatim.

---

## I-1 — CONFIRMED, and it is the whole finding

Static chain, each line read at HEAD:

- `packet.rs:476-484` builds the grid from the **raw leaves** with no reference to the walk —
  `lived_with_you_in_us: d.lived_with_you_in_us == Some(true)`.
- `dependent_gates.rs:279-281` demands `LivedWithYouInUs` **only** under a *Yes* on
  `LivedWithYouOverHalfYear` (`if w.yes(G::LivedWithYouOverHalfYear) { w.demand(G::LivedWithYouInUs); }`).
- `form1040_full.rs:633` writes it unconditionally:
  `check(w, p, &col.lived_with_you_in_us, g.lived_with_you_in_us);`
- `sections.rs:546-552` — `get` returns `None` for a gate the walk does not demand, so the stale answer
  is **invisible** in the form; `sections.rs:576-587` — `clear` returns `Err(SetError::NoSuchRow)` for the
  same gate, so the filer **cannot erase it**. (The `clear` symmetry was itself added by an earlier seam
  review's N-3, for a good reason; it is what closes the escape hatch here.)
- `packet.rs:258-261` — the doc comment asserts the opposite **in terms**: *"a gate the walk never
  demanded (row (5)(b) under a No on (5)(a)) is lawfully blank."*
- `dependent_gates.rs:1690-1699` — the test that looks like the guarantee sets
  `lived_with_you_in_us = None` **by hand** before asserting. It tests `None ⇒ false`; it never tests
  *not-demanded ⇒ blank*. A shadow of the guarantee.

Controller's own probe (temporary test, main-tree code path, reverted):

```
LEDGER demands(LivedWithYouInUs) = false
LEDGER leaf survives = Some(true)
LEDGER screen_param_free = None
LEDGER printed grid: (5)(a) = false , (5)(b) = true
```

`DependentWalk::demands` is `pub` at `dependent_gates.rs:183`, so the reviewer's minimal change (build the
grid from the walk) needs no new API.

**Severity ruling.** Sustained as **Important, blocking, folded first.** The Critical argument was
considered — the brief's one question is *"can a … grid cell be printed that the filer's answers do not
establish"* and the answer is yes — but the reviewer's reasoning holds: no dollar figure moves and the
filer did state (5)(b) at an earlier moment, so this is *stale* testimony rather than *fabricated*
testimony. Critical and Important block identically; relabelling would change the record without changing
the work.

## I-2 — CONFIRMED on both halves

*The guard is blind.* `advisories.rs:923` reads `crate::tax::testonly::ty2024_params()` — a **core-local
fixture literal** (`testonly.rs:232`, `year: 2024`, hardcoded). `BundledFullReturnTables` has **zero**
occurrences anywhere in `crates/btctax-core/src/`; it lives in `btctax-adapters`. The test is therefore
structurally incapable of seeing any bundled package, while its own doc claims it is *"DERIVED over the
bundled years rather than pinned to 2024, so a new package cannot arrive unnoticed — the `1..=38` trap in
its usual costume is a hand-written year list."* It is that trap.

Plant re-run (bundle TY2026, whose `child_tax_credit_per_child` is already `dec!(2200)` at
`tax_tables.rs:272`, by adding `by_year.insert(2026, ty2026_full_return());`):

```
PASS [0.004s] (1/1) btctax-core tax::advisories::ctc_per_child_tests::the_named_ceiling_is_the_years_own_figure
Summary [0.005s] 1 test run: 1 passed, 1332 skipped
```

*The harm is real.* Controller's probe, flipping only the constant:

```
LEDGER CTC_PER_CHILD_SS24H2 = 2000
LEDGER ctc_provably_zero(MFJ, 2 kids, AGI 482000) = true
LEDGER CTC_PER_CHILD_SS24H2 = 2200
LEDGER ctc_provably_zero(MFJ, 2 kids, AGI 482000) = false
```

So at the stale ceiling the proof concludes the credit is gone for a household that still has it, and
Form 1040 line 19 prints a sworn `0`.

*The statute checks out against the repo's primary sources* (the reviewer's file paths were slightly off;
the correct ones are recorded here): `legal/text/statute-irc/PLAW-119publ21_OBBBA.txt:5048` — §70104(a)(2)
strikes *"$2,000"* and inserts *"$2,200"*; `legal/text/irs-guidance/RevProc_2025-32.txt:184-188` — *"the
maximum amount of child tax credit is $2,200 for any taxable year beginning in 2025"*; `:592-594` —
republished $2,200 for 2026. **TY2025, not TY2026** — which is the very next task in this roadmap.

## I-3 — CONFIRMED

```
$ grep -rn "qualifying_child_name" crates/btctax-core/src/tax/packet.rs crates/btctax-forms/
(no matches)
```

It is referenced only in `return_inputs.rs`, `classifier.rs`, `scrub.rs`, `scrub_axis.rs`,
`spec/sections.rs`, `spec/coverage.rs` and one CLI fixture — collected, classified, scrubbed, covered,
**never printed**. The one write to the cell it belongs in (`form1040_full.rs:476-489`) is nested inside
`if let Some(sp) = &header.spouse` **and** guarded by `status == FilingStatus::Mfs`, so a HoH or QSS return
— which carries no spouse `Person` — cannot reach it. The comment above it still reads *"which v1 does not
capture, so it stays blank"*, which T8 falsified without updating.

The QSS half is sharper than the report puts it: the field's **own help text** quotes the instruction
*"enter the child's name in the entry space below qualifying surviving spouse"* while
`sections.rs:333` reads `live: |ri| ri.filing_status == FilingStatus::HoH`. The field's help names the
status the field refuses to serve. `f1040--2024.txt:28-29` prints *"If you checked the HOH or QSS box"*,
and `i1040gi--2025.txt:1298-1306` makes QSS condition 2 precisely the *"could claim as a dependent except
that"* household the entry space exists for.

## M-1 — CONFIRMED (naming defect, not behavioural)

`full_return_forms.rs:784-786`: `a` and `b` are two `fill_form_1040_full` calls in the **same build**, so
the digest equality is determinism, not a pin against pre-T8 bytes. The substantive half (TY2024 credit
boxes stay `None`) does discriminate, and `grep -rn "\.grid\b" crates/btctax-forms/src/` returns **exactly
one** reader (`form1040_full.rs:631`), reachable only under `map.dependents_grid.is_some()`, which TY2024's
map does not declare. TY2024 is safe by construction.

## M-2 — CONFIRMED

Plant re-run: paraphrase the prompt's own operative clause at `questions.rs:2210` (*"did you pay over half
the cost"* → *"did you pay most of the cost"*), leaving the quoted span at `:2219` intact:

```
xtask prompt-check: OK — 86 assertions, all verbatim
```

*Most* and *over half* are different tests — with three contributors, 40% can be the most.

## N-1 — CONFIRMED, and arguably M-3

`dependents_grid.rs:549-556` asserts `hay.contains(&needle)` — substring-anywhere, no ordering. The order
**is** available from the text layer: `design/forms/extract/f1040--2025.txt:47` reads
`Full-time  Permanently  Full-time  Permanently …`, so the interleaving preserves order and a
first-token-in-band assertion is writable. Graded a Nit by the reviewer; by parity with M-2 (both need a
future human edit to bite, both are B1-shaped gaps in an instrument) it is closer to Minor. Neither gates.

---

## What the controller did NOT independently re-run

The report's **"Seams checked clean"** section is a set of negative claims (eleven reproduced kills, the
`_`-free matches, the phantom-field and register plants, the HoH/QSS/FR-67 refusal kills). The fold acts on
none of them, so they were not re-executed; the four instrument outputs and the baseline suite above are
the controller's independent evidence that the tree is in the state the report describes. If a later
re-verification finds one of those clean claims false, it is a finding against the review, not against the
fold.

## Disposition

**0C / 3I blocking.** All three fold, plus M-1, M-2 and N-1 as cheap in-fold cleanups.
Fold brief: `BRIEF-fold-interview-T8-review.md`.
