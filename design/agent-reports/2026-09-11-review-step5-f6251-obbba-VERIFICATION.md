# Controller ledger — machine-checking the step-5 seam review

**Report:** `design/agent-reports/2026-09-11-review-step5-f6251-obbba.md` (persisted verbatim `fbe9d788`,
323 lines, **0C/2I/3M/1N**). Nothing folded before this ledger.

---

## The round's finding — 37 → 43 is a COLLISION, not a renumber. **CONFIRMED verbatim.**

Read from the two extracts, not from the report:

| Schedule 1-A | TY2025 final (`f1040s1a--2025.txt`) | TY2026 draft (`f1040s1a--2026-DRAFT.txt`) |
|---|---|---|
| **line 37** | `:108` — *"Enhanced deduction for seniors. Add lines 36a and 36b"* | `:213` — *"Enter the amount from line 3"* |
| **line 43** | *(does not exist)* | `:222` — *"Enhanced deduction for seniors. Add lines 42a and 42b"* |
| line 3 | `:22` — *"Add lines 1 and 2e"* (the MAGI) | — |

So the senior-deduction subtotal moved 37 → 43, **and TY2026's line 37 is now occupied by modified AGI.**
The same line number names a ≤$6,000 deduction in one revision and a six-figure income in the next. A
renumber would have left 37 vacant; this one refilled it.

**Direction of the error, derived from the form's own arithmetic.** Form 6251 line 1a = *"Subtract
Schedule 1-A line 43 from Form 1040 line 14"*; line 1b = *"Subtract line 1a from Form 1040 line 11b"*.
Substituting MAGI for the senior deduction drives 1a sharply negative, so 1b — the AMT base — is
**overstated by approximately MAGI**. Taxpayer-adverse, six figures, and the opposite direction from the
TY2025 defect the existing kills pin. The report's characterisation is exact.

### ★★ The false premise is IN THE SOURCE, twice — and the second occurrence prescribes the dangerous edit

- `crates/btctax-forms/src/f6251_revision.rs:23` — *"Schedule 1-A line 37 is the senior deduction
  subtotal and line 43 is its TY2026 **renumber**."*
- `crates/btctax-core/src/tax/return_1040.rs:2971-2973` — *"TY2026 is expected to **REUSE the `Y2025`
  shape** rather than gain a variant (§7 D3: Form 6251 does not renumber for TY2026; only the cited
  Schedule 1-A line moves, 37 → 43). Adding the `2026 =>` arm is therefore **a one-line edit** once the
  final form has been read."*

The second is the more dangerous: it tells a future implementer that the safe action is a one-line arm
reusing `Y2025`, whose field is literally named `schedule_1a_l37` and is populated from
`s.part5.line37` (`return_1040.rs:2988-2990`). Follow that instruction on TY2026 and the AMT base takes
MAGI. **Both comments must be corrected in the fold**; a false premise recorded in a doc comment is worse
than no comment, because the next reader trusts it.

## I-1 — the cross-reference is a FIELD NAME, and the one function holding both sides discards it. **CONFIRMED.**

`crates/btctax-forms/src/form6251.rs:296-303` is

```
revision.schedule_1a_line().ok_or_else(|| { … })?;
```

— a bare statement. The resolved number is **existence-checked and then dropped**; nothing binds it. And
this is the one function that holds *both* the form's stated line number (from the revision table's
sentence) and the computation's value (`Form6251Line1::Y2025`, whose field is `schedule_1a_l37`), so it is
the natural place for the comparison that does not happen. The report's reading is right.

## 0 Critical is the correct severity — the three gates hold. **CONFIRMED.**

`form6251_line1_rule` (`return_1040.rs:2974-2994`) is `2024 => Y2024`, `2025 => Y2025 {…}`, `_ => None`.
TY2026 therefore yields `None` today and no figure is produced; `full_return_for(2026)` is also `None`;
and `LineSet` has no `F6251_2026`. A derived sweep over `1990..=2060` exists at `return_1040.rs:12034`.
So the defect is **armed, not live** — Important, exactly as reported, and it becomes Critical the moment
anyone takes the invitation at `:2971`.

## I-2 — the new emitter's Part III is executed by no test. **CONFIRMED by count.**

`grep -c "money_cells\|part_iii_completed = true"` over `crates/btctax-forms/tests/f6251_obbba.rs`
returns **0**. The only fixture sets `part_iii_completed = false` (`:774`). The TY2024 equivalent does
both: `f6251_fill.rs` sets it `true` at `:126` and `:203`, and sweeps every cell at `:211-216`
(`for cell in map.money_cells()`, with a `checked` counter). So the sweep that would exercise the new
map's Part III **already exists in the branch for the sibling revision** — B3's documented shape, a third
time in this repo.

## ★ A coordinator measurement the report refuted, and it was right

The review brief said `d8d023af` is *"11 files, +919/−110"*. `git diff --stat d8d023af^ d8d023af` gives
**12 files changed, 2588 insertions(+), 110 deletions(-)**.

**Mechanism, recorded because it will recur:** the coordinator ran `git diff --stat` over the **working
tree**, which does not list untracked files — so the three files the build *added*
(`src/f6251_revision.rs`, `tests/f6251_obbba.rs`, the report) were silently excluded, and the quoted
`+919` was a sum over the visible rows of a `tail`-truncated listing rather than the tool's own total
line. **`git show --stat <commit>` is the measurement.** This is the third coordinator measurement error
in this session and all three are one shape: a number quoted from a truncated or lossy read instead of
from the tool's total.

## Not re-derived

Seams 2–5's clean verdicts (the five constant lines being `AmtParams`/`TaxTable`-driven with no literals;
the packet refusal; both rewritten instruments discriminating in both directions; Part I's verbatim
transcription) and the three Minors and the Nit. Spot-checked only where the findings above touch them.

## Verdict

**The report stands at 0C/2I/3M/1N, and its Important-not-Critical call is correct.** Its central finding
is one the build could not have made — the build closed the forms-side trap and then handed over the
computation-side twin, and the reviewer found that the twin is *worse than the build believed*, because
the number is not vacant on the other side. Both Importants are real, both are mechanical to fix, and the
two false source comments are the fold's first job.
