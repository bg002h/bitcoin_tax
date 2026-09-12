# BRIEF — folding the step-5 seam review (`f6251/2025`)

**Report folded:** `design/agent-reports/2026-09-11-review-step5-f6251-obbba.md` (persisted verbatim
`fbe9d788`, **0C/2I/3M/1N**). **Ledger:** `…-VERIFICATION.md` (`c91eb5e5`) — every measurable claim
re-derived from the tree before this brief; read it and do not re-derive what it settles.
**Build under repair:** `d8d023af` (12 files, +2588/−110 — `git show --stat`, not a working-tree diffstat).

You are ONE opus implementer in the **shared main tree**. Nothing is committed while you work.

---

## 0. Stop-and-report

**Stop and report rather than build on any premise here you can disprove.** Six briefs in this arc have
been refuted by measurement; three of the six were the coordinator's, two of those about this very form,
and the most recent was refuted by the reviewer whose findings you are now folding (a diffstat quoted from
a `git diff --stat` that silently omits untracked files). A refuted premise is a first-class finding.

## 1. Scope, in priority order

Six items. **The first is not a comment cleanup — it is the fix.**

### ★★★ (1) The two source comments that call the collision a "renumber"

The ledger established, verbatim from the extracts, that **Schedule 1-A 37 → 43 is a COLLISION, not a
renumber**: TY2025 line 37 is *"Enhanced deduction for seniors"*, TY2026 line 37 is *"Enter the amount
from line 3"* (the MAGI), and the senior subtotal is TY2026's line 43. The number was **refilled, not
vacated**. Substituting one for the other overstates the AMT base by ≈MAGI — six figures,
taxpayer-adverse.

Two comments say otherwise, and the second **prescribes the dangerous edit as safe**:

- `crates/btctax-forms/src/f6251_revision.rs:23` — *"line 43 is its TY2026 **renumber**"*.
- `crates/btctax-core/src/tax/return_1040.rs:2971-2973` — *"TY2026 is expected to **REUSE the `Y2025`
  shape** … only the cited Schedule 1-A line moves, 37 → 43 … Adding the `2026 =>` arm is therefore **a
  one-line edit**"*. That variant's field is `schedule_1a_l37`, populated from `s.part5.line37`
  (`:2988-2990`). A future implementer following this instruction ships the six-figure error.

Correct both to state the collision and what it costs. **A false premise in a doc comment is worse than
no comment, because the next reader trusts it** — and in this repo two such comments have already been
carried through folds.

### ★★ (2) I-1 — join the two sides, structurally

`crates/btctax-forms/src/form6251.rs:296-303` is `revision.schedule_1a_line().ok_or_else(…)?;` — a bare
statement, so the form's stated line number is **existence-checked and dropped**. That function is the one
place holding *both* that number and the computation's figure, so it is where the comparison belongs.

The obstacle, and the design question: **the cross-reference lives on the computation side as a FIELD
NAME** (`schedule_1a_l37`), not as data, so there is nothing to compare against. Requirement:

- after the fold, a TY2026 arm that reuses the `Y2025` shape **must not silently produce a figure** —
  compile error, or a refusal, or a red test. Not a convention.
- ★ Consider whether the honest shape is to name the computation's field for **what it is** (the senior
  deduction subtotal — a semantic cross-year quantity) and carry the **cited line number as data** beside
  it. `CLAUDE.md`'s scope note draws exactly this line: a field inside a *transcription* struct is named
  for its line; a *cross-year quantity* the return consumes is semantic, and naming it for one revision's
  line number is the compression the rule forbids. You choose; say why.
- Do not bundle params, do not add a `2026` arm, do not touch the fail-closed gates. The fix is the
  mechanism, not the year.

### ★★ (3) M-1 — promoted, because it is the instrument that would hold (1)

`crates/btctax-core/src/tax/line_coverage.rs:682-698` is `if let Form6251Line1::Y2024 { line1 } = p`, so a
TY2025/TY2026 chain yields an **empty** `Coverage` and nothing reds — an `if let` that matches nothing is
silent. Its own comment says the 1a/1b rows arrive *"when that year's map lands"*. **That map landed in
`d8d023af` and no rows were added.**

Consequence, and why this is folded with the Importants rather than filed: **`Form6251Line1::Y2025`'s doc
comment — the one that hardcodes "line 37", the artifact item (1) corrects — is verified verbatim by
nothing.** `cite-check` also excuses the form entirely (`cite_check.rs:958` carries
`("f6251", &[2024, 2025])` under `AUTHORITY_NOT_YET_ARCHIVED`). So fixing the comment without fixing this
leaves the corrected text unheld, and the next revision can rot it again in silence. Add the OBBBA rows;
if the `if let` shape is what allowed the silence, make the match exhaustive so a new variant cannot be
skipped.

### (4) I-2 — the Part III sweep already exists for the sibling revision

`tests/f6251_obbba.rs` sets `part_iii_completed = false` in its only fixture (`:774`) and never sweeps
`money_cells()` — measured: `grep -c "money_cells\|part_iii_completed = true"` = **0**. So 29 of the 42
money cells, including the line-33/36 pair, are never written or read back, and
`Form6251ObbbaMap::money_cells()` has no reader although its own doc says the read-back sweeps iterate it.

The fix is in the branch: `tests/f6251_fill.rs` sets it `true` (`:126`, `:203`) and sweeps every cell
(`:211-216`, `for cell in map.money_cells()` with a `checked` counter). **Port it, do not reinvent it** —
and keep the counter, so a cell that stops being reachable reds instead of quietly dropping out of the
sweep. This is B3's "the fix already exists in the branch" shape for the third time in this repo; the
whole point is to carry it across.

### (5) M-2 and M-3 — the false justification and the unchecked second copy

- **M-2** (`f6251_revision.rs:41-46`): the stated reason for not transcribing line 4's figure is that it
  is *"already per-year in `AmtParams`"* — but **no `AmtParams` carries TY2025's printed $900,350**
  (`grep -rn "900350" crates --include="*.rs"` → one hit, the module's own test at `:232`). So the
  "second authority" argument is false for the only revision this module holds. Correct the sentence to say
  what is actually true.
- **M-3** (`map.rs:628-631`): `Form6251ObbbaMap::line4`'s doc comment quotes TY2025's sentence **including
  `$900,350`**, inside a struct that also serves TY2026, and the verbatim checker does not reach Rust doc
  comments (it parses the map TOML's `# <label> "…"` lines, `tests/f6251_obbba.rs:563-612`). Either make
  that copy checked or stop quoting a year-specific figure in a shared struct's doc comment. A quoted
  figure no instrument reads is the same class as M-1.

### (6) N-1 — Nit, your discretion

The three cell accessors take the **first** occurrence of their anchor and only the zero-occurrence case
fails closed. Fix inline if it is small, or file it with an owning phase. Say which.

## 2. Kills — B1, and the one that matters

Every new or changed check runs against the pre-fix state with its **red pasted** into your report.
Specifically:

- **(1)+(2) together**: plant a TY2026 arm that reuses `Y2025` with `schedule_1a_l37` fed from
  `part5.line37`, and show the fold stops it — compile error, refusal, or red test, with the output. **A
  green here that cannot fail is worse than no test**, and this is the round's central kill.
- **(3)**: plant a dropped OBBBA row / a rotted quote in `Form6251Line1::Y2025`'s doc comment and show
  `line-coverage` reds. Today it cannot.
- **(4)**: with the sweep ported, gut one Part III cell's write and show the read-back reds; confirm the
  `checked` count is what you expect and paste the number.

## 3. Severity rules that bind this fold

**Critical** — wrong result, data loss, an unmet guarantee, or a defect in what a tool *claims* to have
done (a gate that cannot fail, a refusal that does not refuse, a false PASS). **Important** — a real
defect, a missing case, an unsound assumption. **Minor/Nit** — recorded. ★ Secret-handling defects never
gate. A **blank** is the normal case; assert provenance, never non-blankness. A hardcoded `0` on an
unasked line fabricates sworn testimony. **Do not widen scope** — anything else goes to `FOLLOWUPS.md`
with an owning phase.

## 4. Mechanics

**Main tree. Do NOT commit, push, `git stash`, `git checkout` or revert anything** — a builder in this arc
ran `git stash` against its brief. Revert a mutation with a **cp backup**. **No subagents.** No
`--no-verify`. Never hand-count what a tool can count. Baseline at HEAD: `make gate` **3589 passed / 12
skipped**, fmt clean — report your final numbers as numbers.

## 5. Your report — final action

Write `design/agent-reports/FOLD-step5-f6251-review.md`: per finding, what changed and where (`file:line`)
and **why that mechanism**; kills with pasted red-then-green, §2's first one especially; refuted premises;
residue filed with owning phases; the literal gate numbers. Return only a short summary plus the path.
