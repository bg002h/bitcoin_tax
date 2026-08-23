# Review — the FOLD `02939632..HEAD` (does the response to B-1/B-2 introduce a defect of its own?)

_Read-only, independent. Range: `02939632..HEAD` = `9728e2ec` (the fold) + `e4e56a30` (docs-only
continuity). Scoped to the ONE question: **does the fold introduce a defect of its own?** Not "was
B-1 real" (settled), not "is the branch sound overall"._

---

VERDICT: sound

## SCOPE

I read `git show 02939632` (the review), `git show 9728e2ec` (the fold + its commit message), and the
whole range diff. I then read the **current** tree rather than the diff for everything load-bearing:
`apply_carryover_writeback` and both new functions (`return_1040.rs:2918-3155`), the whole of
`write_back_carryover` (`cmd/tax.rs:803-1005`), `import_return_inputs` including the normalisation
block and the four-arm preservation block (`cmd/tax.rs:48-201`), `m4_authority` and its three cases
(`cmd/tax.rs:785-795`), `BenefitCarryoversNotStated` and `QbiCarryforwardNotStated`
(`advisories.rs:1076-1138`), the two carryforward-conditioned questions and their shared liveness
predicate (`questions.rs:200-203, 715-775`), the reworded §108 refusal (`return_refuse.rs:846-864`),
the classifier's treatment of the provenance fields (`classifier.rs:104-155`), and the three new/changed
tests. I enumerated callers with grep restricted to `crates/` (the repo also contains stale
`.claude/worktrees/` copies that pollute an unrestricted grep): `apply_carryover_writeback` has one
production caller (`cmd/tax.rs:899`), `write_back_carryover` has one (`main.rs:197`, unconditional
`println!`), `capital_loss_roll_is_grounded` has two (`return_1040.rs:3103`, `cmd/tax.rs:972`) reading
the same `&ar`/`&ri`. I checked the §108(b) timing claim against `26 USC §108(b)(2)(G)` /
`§108(b)(4)(A)` and against the worksheet header as extracted:
`design/forms/extract/i1040sd--2025.txt:1819` — *"If you excluded canceled debt from income in 2025,
see Pub. 4681."* (2024 file, line 1454, is the same sentence with 2024). I did not re-run the suite,
clippy or fmt — stated as already machine-verified — and I did not re-derive the reproductions.

## FINDINGS

**No Critical. No Important.**

The three things the brief pointed me at came back clean:

1. **The `★ NOT WRITTEN` message is true on every branch that reaches it.** `grounded` is false only
   when *all three* of `provenance != Computed`, `carryforward_in == {0,0}`, and *rounded*
   `carryforward_out == {0,0}` hold. Negatives are impossible here (`first_negative_amount` refuses at
   `screen_inputs`, and `write_back_carryover` runs both screens first), so there is no case where the
   branch fires with a material carryover in play. I checked it on the first-roll path, the re-roll
   path (`updated.*_provenance == Computed` ⇒ the stale clause fires and names the figure), and the
   `--force` path (`force` never reaches the `grounded` gate, so `--force` prints the same accurate
   message). The one wording imprecision is a Nit, below.
2. **Normalising the four provenances to `User` at `income import` cannot move a filed figure.** The
   classifier already classifies every `CarryProvenance` field as `Class::NoTaxDirection` — *"no print,
   no tax direction"* (`classifier.rs:148-160`) — and I confirmed by enumeration that every production
   reader is one of: an advisory (`BenefitCarryoversNotStated`, `QbiCarryforwardNotStated` — normalising
   makes them fire *more*, the over-warning direction), a `!force` guard in `apply_carryover_writeback`
   (normalising makes it refuse *more*, fail-closed), `m4_authority`'s third case (v1-unreachable by its
   own documented argument), or the import preservation block — which reads `existing`, never `ri`, for
   all four arms. **The QBI direction that would understate tax is untouched**: both QBI arms key on
   `ri.qbi.*.is_zero()` and `existing.*_provenance`, neither of which the block writes.
3. **The §108(b) timing claim is correct.** §108(b)(4)(A) makes the attribute reduction *after* the
   determination of tax for the year of discharge, and §108(b)(2)(G) puts the net capital loss for that
   year and the carryover to it on the list — so the reduced figure is what is available going forward.
   The new sentence — *"The REDUCED carryover Pub. 4681 leaves you with is what carries into the
   following year"* — states exactly that. The frame also matches the form: the worksheet header asks
   the condition about the **return year**, and `ExcludedCanceledDebt` is `Durability::PerYear` and
   scoped *"this year"* in its prompt, so *"it stays Yes however you edit the carryover"* and *"btctax
   can file that year once its own answer here is No"* are both true. **The rewording is also a tax
   improvement over what it replaced**: the old *"enter the reduced carryover"* pointed the filer at
   putting the reduced figure on the **discharge year**, which §108(b)(4)(A) says is the wrong year.

The behavioural delta the fold does introduce, and which I chased hardest, is real but already owned:
normalisation makes the fourth preservation arm (`cmd/tax.rs:176-188`) fire on a TOML that previously
bypassed it by asserting `provenance = "computed"`, so such an import now **preserves** the stored
carryover instead of **wiping** it. That is the direction that keeps a deduction alive — but it is the
review's own suggested fix, the bypass it removes is the forge the fold exists to close, it moves no
figure v1 can read, and the replacement escape (`income clear` then `income import`) is documented in
`LIMITATIONS.md` and asserted end-to-end by
`a_computed_capital_loss_stamp_survives_every_command_that_should_retract_it`. FR-17 states it. Not a
finding.

---

### Minor 1 — the `--force` guard promises an overwrite the `grounded` gate then refuses to perform

- **file:line** — `crates/btctax-core/src/tax/return_1040.rs:3065-3075` (the fourth guard) against
  `crates/btctax-cli/src/cmd/tax.rs:972-999` (the new note).
- **ASSERTED** — *"next year's capital-loss carryover was user-entered (`income import`) — pass
  `--force` to overwrite it with the computed §1212(b) carryover"*.
- **ACTUALLY TRUE** — the guard runs unconditionally, *before* and independently of
  `capital_loss_roll_is_grounded`. On an ungrounded year there is no computed §1212(b) carryover, so
  `--force` overwrites nothing; it only unlocks the other three carryovers.
- **Failing case** — year 2024 with a charitable carryover, no capital-loss carryover-in and no
  capital-loss activity; 2025's row carries a user-entered `long = $40,000`. `report --tax-year 2024
  --write-carryover` refuses with the message above. The filer passes `--force`, and the **same command**
  then prints *"★ NOT WRITTEN: the capital-loss carryover … stamps nothing."* Two statements from one
  invocation that contradict each other, and a filer who believes the first may go and "correct" a
  2025 figure that was right.
- **Smallest fix** — one conjunct, using the predicate the fold just created (`ar` and `ri` are both
  in scope at `return_1040.rs:2919-2920`):
  `if capital_loss_roll_is_grounded(ar, ri) && (next_year.capital_loss_carryforward_in.short > Usd::ZERO || …)`.
  Safe in both directions: an ungrounded roll then stops wedging the other three carryovers behind a
  `--force` that does nothing, and the guard still protects every roll that can actually overwrite.
- **Note on scope** — the guard predates the fold, so this is not *introduced* by it. I record it
  because it is the same class B-1 named (a message that reasons about the gated write without asking
  the gate), it is the one instance the fold left standing, and the fold's own new predicate reduces
  the fix to a single conjunct. Non-gating.

### Minor 2 — *"btctax CANNOT FILE THIS YEAR for you"* is broader than what the code enforces

- **file:line** — `crates/btctax-core/src/tax/return_refuse.rs:856-864`.
- **ASSERTED** — btctax cannot file the exclusion year, *because* the exclusion is reported on Form
  982, which btctax does not produce.
- **ACTUALLY TRUE** — the refusal is gated on
  `question_is_live(ExcludedCanceledDebt, ri)`, i.e. on `carryforward_in_present`
  (`questions.rs:200-203`, `return_refuse.rs:846-849`). The Form 982 rationale is not gated on
  anything of the kind: a filer who excluded canceled debt and has **no** capital-loss carryforward is
  never asked the question, never refused, and gets a return with no Form 982 on it.
- **Failing case** — same filer, carryover edited to `{0,0}`: the question goes dark, the refusal
  disappears, and btctax files the discharge year — the year the sentence says it cannot file. (Any
  action the sentence provokes still points the safe way, and the omission of the COD income itself is
  correct because it is excluded; the missing artefact is Form 982.)
- **Smallest fix** — either narrow the clause to the carryover (*"btctax cannot file this year for you
  while you are carrying a capital-loss carryforward …"*), or leave the wording and file the Form-982
  scope gap with an owning phase. **The gap is pre-existing** — v1 collects no canceled-debt income at
  all — so this is a claim-vs-enforcement mismatch the fold's new sentence surfaces, not one it
  creates. Non-gating.

### Nit 1 — *"{year} was never asked about one"*

`cmd/tax.rs:993`. There is no surface on which btctax asks about `capital_loss_carryforward_in`: it is
not a `FormQuestion`, `income answer` never reaches it, and the TUI's only carryover editor is the raw
`TaxProfile` one (`tui-edit/src/edit/form.rs:126-127`). The clause therefore describes btctax's own
ignorance rather than the filer's silence, and it is delivered verbatim to a filer who **did** write
`[capital_loss_carryforward_in] short = "0" / long = "0"` — the code says twice, in this very fold, that
an explicit zero and an absent key are the same bytes. Materially harmless (the operative claim,
*"stamps nothing"*, is true on every branch) and it matches the pre-existing `LIMITATIONS.md` wording, so
changing one means changing both. Suggested: *"btctax has no capital-loss carryover on file for {year},
and {year} produced none of its own"*.

### Nit 2 — the ungrounded branch names a stale `Computed` figure but not a live `User` one

`cmd/tax.rs:977-990`. The `stale` clause fires only on `CarryProvenance::Computed`. When the roll is
ungrounded and next year's row carries a **nonzero user-entered** carryover (reachable only under
`--force`, per Minor 1), the summary says *"stamps nothing"* and says nothing about the $40,000 sitting
on the row. T9's own standard is that all four carryovers are accounted for on the surface the filer
trusts; naming the `User` figure too would be one `else if`.

### Nit 3 — pre-existing, unchanged by the fold, recorded only because the new text builds on it

`return_refuse.rs:854-856`: *"the carryover it would **deduct** and carry forward is too large."* Under
§108(b)(4)(A) the discharge-year return uses the **unreduced** carryover — only the carry-forward half
is too large. The fold's new sentences make this inert (btctax refuses the year outright), which is why
it is a Nit and not a finding.

---

## WHAT ELSE I CHECKED AND FOUND CLEAN

- **One predicate, two readers, no divergence.** `cmd/tax.rs:972` and `return_1040.rs:3103` are handed
  the same `&ar` and `&ri`; `ri` is never mutated between `assemble_absolute` and the summary, and
  `apply_carryover_writeback` consumes `next` by value and returns `updated`, so the summary reads the
  post-write row. `rounded_capital_loss_carryforward_out` is byte-identical to the `ws_out` it replaced,
  so the gate's semantics did not shift when it was extracted.
- **`newly_unfilable` cannot contradict the new note.** On an ungrounded roll the only fields that move
  are the charitable list and the two QBI carryforwards; the only `screen_inputs` rule reading any of
  them is `NonPublicCharityContribution` (`return_refuse.rs:1059-1073`), and a year Y that could produce
  a non-50%-org carryover-out is itself refused before `write_back_carryover` reaches the write. So the
  hardcoded *"Writing a capital-loss carryover onto that row…"* attribution has no reachable
  counter-example on this branch.
- **K11 / K11b are genuine duals.** K11 pins `observed ⊆ summary` off `assigned_fields`; K11b pins the
  reverse off the same derivation, with a positive control (`charitable_carryover_in` must have moved)
  and an explicit assertion that the gate really skipped, so halves (2)/(3) cannot pass vacuously. The
  shared `ri_leaves`/`assigned_fields` extraction means the two cannot drift.
- **`income import` is the only TOML→`ReturnInputs` write path.** `parse_return_inputs_toml` is private
  with one production caller; the other `return_inputs::set` sites are the write-back
  (`cmd/tax.rs:901`), `income answer` (`answer.rs:212`) and the input-form finalize
  (`input_form_store.rs:321`), none of which parse filer-authored TOML. The normalisation is not
  bypassable from outside the process.
- **The escape `LIMITATIONS.md` promises works.** `clear_return_inputs` is wired to a real subcommand
  (`cli.rs:466`), and the FR-17 test asserts `income clear` + `income import` drops both the value and
  the stamp — the assertion that reds if the document starts making a promise btctax does not keep.
- **FR-17 no longer claims the escape the review named.** The review called hand-writing
  `provenance = "Computed"` *"the only technical escape"*; the fold closed it, and both `FOLLOWUPS.md`
  and `LIMITATIONS.md` now name `income clear` + `income import` instead. No stale claim was left
  behind.

## WHAT WOULD MAKE THIS REVIEW WRONG

If a `CarryProvenance` value can reach a printed line or a computed figure by some route the
classifier's `Class::NoTaxDirection` exemption and my grep of every `*_provenance` read in
`crates/*/src/` both missed — then the `income import` normalisation stops being advisory-only and
Minor 2's cost changes with it.
