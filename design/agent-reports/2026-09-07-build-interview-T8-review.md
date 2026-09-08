# Seam review — interview build T8 (row (7), HoH / QSS, the TY2025 dependents grid)

Independent adversarial build review, own worktree at `121c8805`
(`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a3c18342285127807`). Every plant was made here and
reverted; `git status --porcelain` is empty at the time of writing. No commits, no pushes, no subagents.

**Environment.** `crates/btctax-forms/forms/2025/f1040.pdf` **is** bundled (220,237 bytes), so every
emitter read-back kill below genuinely ran against the real TY2025 template. Six `form_delta` tests fail
in this worktree, as the brief predicted; they are not findings and are excluded from every count.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo build --locked --workspace --all-targets                       # green
cargo run -q -p xtask -- line-coverage                               # 373 money lines, 18 forms, 31 exceptions (ratchet 31), 0 unverifiable
cargo run -q -p xtask -- census-join                                 # 290 unmodeled entries across 13 maps, every one placed
cargo run -q -p xtask -- stop-list                                   # 8 + 4 sources, 86 registry prompts; no forbidden shape
cargo run -q -p xtask -- prompt-check                                # OK — 86 assertions, all verbatim
cargo nextest run --locked -p btctax-forms -p btctax-input-form -p btctax-cli -p btctax-adapters --no-fail-fast
                                                                     # Summary [12.380s] 1342 tests run: 1342 passed, 5 skipped
cargo nextest run --locked -p btctax-core -p xtask                   # 1444/1500 run: 1438 passed, 6 failed (all form_delta = environment), 1 skipped
cargo nextest run --locked -p btctax-core -E 'test(/hoh/) or test(/qss/) or test(/credit_column/) or test(/nra_spouse/) or test(/line_19_forgo/) or test(/marital/) or test(/dependent_gates/) or test(/ctc_per_child/)'
                                                                     # 34 tests run: 34 passed
cargo nextest run --locked -p xtask -E 'test(/dependents_grid/)'     # 4 tests run: 4 passed
```

## Summary

The mechanical half of T8 is strong and I could not break it. The TY2025 grid's 25 mapped cells are
derived from geometry, every one resolves in the real template, the on-states are measured, the register
moved by exactly 25, and I reproduced eleven of the report's own kills verbatim — including the two the
report leans hardest on (the transposed pair and the `Neither`-row fixture that discriminates the ODC
plant). TY2024 is structurally safe: `DependentRow::grid` has exactly **one** reader in the whole
emitter, and it is unreachable unless a year's map declares `[dependents_grid]`, which TY2024's does not.

Three Important findings, all outside the ground the report's own kills cover.

1. A grid checkbox reaches the printed TY2025 page that the filer's live answers do not establish —
   row **(5)(b)** checked under an unchecked **(5)(a)** — and the filer can neither see nor clear it.
   The build report asserts the opposite in terms.
2. The FR-85 guard **cannot red on the defect it is documented to catch**. Both the code doc and build
   report §5 claim `the_named_ceiling_is_the_years_own_figure` reds "the moment TY2025's or TY2026's
   package is bundled". I bundled TY2026 and it stayed green, and `ctc_odc_line19` then swore a `0` on
   line 19 for a household that still has credit.
3. `hoh_qualifying_child_name` is collected, classified, scrubbed and helped — and has **no reader on
   any year**. It is also not live on QSS, which the form's own sentence names alongside HOH.

`Counts: C=0 I=3 M=2 N=1`

---

## I-1 — Row (5)(b) prints CHECKED under an unchecked row (5)(a); the filer cannot see it or clear it

**Where.** `crates/btctax-core/src/tax/packet.rs:476-484` (`ReturnHeader::build`);
`crates/btctax-forms/src/form1040_full.rs:633`; `crates/btctax-input-form/src/spec/sections.rs:545-587`.

**What is wrong.** `DependentGridRow` is built by reading the raw leaves —
`lived_with_you_in_us: d.lived_with_you_in_us == Some(true)` — with no reference to whether the walk
*demanded* the gate. `walk_dependent` demands `LivedWithYouInUs` only under a *Yes* on
`LivedWithYouOverHalfYear` (`dependent_gates.rs:281-283`), so a filer who answers (5)(a) *Yes* and
(5)(b) *Yes*, then flips (5)(a) to *No*, leaves `lived_with_you_in_us = Some(true)` behind. Nothing
clears it, nothing screens it, and `push_dependents_grid` writes it unconditionally.

The seam makes it worse rather than better. For a gate the walk does not demand, `get` returns `None`
(sections.rs:546-552) so the stale answer is **invisible** in the form, and `clear` returns
`SetError::NoSuchRow` (sections.rs:576-582) so the filer **cannot erase it**. It is unreachable through
the product surface and printed on the page.

This is exactly what the build report says cannot happen — *"a gate the walk never demanded (row (5)(b)
under a No on (5)(a)) is lawfully blank"* (report §1) — and the same claim is in the `DependentGridRow`
doc comment (`packet.rs:258-261`). The test that looks like it covers this
(`the_credit_column_reaches_the_row_the_emitter_prints`, `dependent_gates.rs:1690-1697`) sets
`lived_with_you_in_us = None` by hand before asserting, so it tests `None ⇒ false`, never
*not-demanded ⇒ blank*. It is a shadow of the guarantee, not the guarantee.

**Evidence.** Probe added to `full_return_forms.rs`, driving the production `push_dependents_grid` +
`pdf::` path into the real TY2025 template and reading the boxes back off the saved bytes:

```rust
btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);   // a qualifying child: 5(a)=Yes, 5(b)=Yes
ri.header.dependents[0].lived_with_you_over_half_year = Some(false); // the filer flips (5)(a) to No
btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);   // …and answers the Step 4 gates
assert_eq!(ri.header.dependents[0].lived_with_you_in_us, Some(true)); // (5)(b) survives — passes
```

```
PROBE unanswered_tier = None
PROBE printed: (5)(a) = None, (5)(b) = Some("1") (on-state 1)
```

Both screening tiers pass (`screen_param_free` = `None`, `screen_inputs_tiered{unanswered_refuses:true}`
= `None`); the row is a claimable ODC dependent; and the filed page carries *"(b) And in the U.S."*
checked with *"(a) Yes"* blank. The form prints (b) nested under (a) — *"(5) Check if lived with you more
than half of 2025 — (a) Yes / (b) And in the U.S."* (`f1040--2025.txt:44-46`) — so the page asserts a
sub-condition of a condition the return does not assert.

Not raised to Critical because no dollar figure moves and the filer did state (5)(b) at some earlier
point; the controller may reasonably read it as Critical under the *entry-is-testimony* doctrine, since
the printed page contradicts itself and the build report guarantees it does not.

**Minimal change.** Build the grid from the walk rather than from the leaf: in `ReturnHeader::build`,
take `let walk = walk_dependent(ri, row);` once and gate each row-(5)/(6) bool on
`walk.demands(gate) && leaf == Some(true)` — `DependentWalk::demands` is already public and is the
module's stated definition of liveness, and `credit_column` can then read that same walk instead of
walking a second time. Add the kill that the existing test skips: 5(a) = `Some(false)` with 5(b) =
`Some(true)` must print (5)(b) blank.

## I-2 — The FR-85 CTC pin cannot red on the defect it is documented to catch

**Where.** `crates/btctax-core/src/tax/advisories.rs:806-821` (the `CTC_PER_CHILD_SS24H2` doc) and
`:906-960` (`ctc_per_child_tests::the_named_ceiling_is_the_years_own_figure`); build report §5.

**What is wrong.** The constant's doc says the pin *"asserts it equals
`FullReturnParams::child_tax_credit_per_child` **for every year whose package is bundled**"* and that
*"the moment TY2025's or TY2026's package is bundled the pin REDS"*. The test's own doc adds: *"It is
DERIVED over the bundled years rather than pinned to 2024, so a new package cannot arrive unnoticed —
the `1..=38` trap in its usual costume is a hand-written year list."* The build report repeats it: *"the
pin is what keeps that true: the moment TY2025's or TY2026's package is bundled the test REDS."*

None of that is true. The test's only source of params is `crate::tax::testonly::ty2024_params()`
(`advisories.rs:923`), a **core-local test fixture literal** (`testonly.rs:232-256`) hardcoded to 2024.
It never touches `BundledFullReturnTables` and cannot see any package that lands. It is the
hand-written-year-list trap its own doc names, wearing the costume.

**Evidence.** Two plants.

Plant A — move the shipped TY2024 figure (`btctax-adapters/src/tax_tables.rs:151`, `2000` → `2200`):

```
Summary [0.004s] 1 test run: 1 passed, 1332 skipped     # the pin is green on a moved shipped figure
```

(A different test, `btctax-adapters::…::every_shipped_full_return_params_equal_the_ones_the_corpus_validates`,
does red — so the shipped figure is pinned, just not by the test that claims to pin it.)

Plant B — bundle TY2026's package (`by_year.insert(2026, ty2026_full_return());`, whose
`child_tax_credit_per_child` is `dec!(2200)`):

```
PASS [0.003s] (1/1) btctax-core tax::advisories::ctc_per_child_tests::the_named_ceiling_is_the_years_own_figure
```

…and with that same plant in place, the harm the doc describes is live. Probe in `advisories.rs`:

```
PROBE ctc_odc_line19(TY2026, MFJ, 2 kids, AGI 482000) = Some(0)
PROBE CTC_PER_CHILD_SS24H2 = 2000
```

Schedule 8812: L3 = 482,000, L9 = 400,000, L11 = 5% × 82,000 = 4,100. At the real §24(h)(2) figure for
that year, L8 = 2 × 2,200 = 4,400 > 4,100, so line 12 is *Yes* and the credit is **not** zero. The
hardcoded ceiling gives L8 = 4,000 ≤ 4,100, so `ctc_odc_line19` returns `Some(0)` and Form 1040 line 19
prints a sworn `0` (26 USC §6065) for a household that still has credit — taxpayer-adverse and invisible
on the page, exactly as the doc predicts, with the guard green.

The statutory half of the report is correct and I verified it against the repo's primary sources:
`PLAW-119publ21_OBBBA.txt` §70104(a)(2) strikes `$2,000` and inserts `$2,200`, §70104(f) applies the
amendments *"to taxable years beginning after December 31, 2024"*, and `RevProc_2025-32.txt` .03 says
$2,200 for a taxable year beginning in 2025 and .05(1) republishes $2,200 for 2026. So the danger is
real and arrives on the very next task in this roadmap (TY2025 is the stated blocker). The separate
`ty2025_full_return_must_stay_fail_closed` / `ty2026_full_return_must_stay_fail_closed` gates will red
when a package lands, but their message is about bundling, not about this ceiling — someone
deliberately landing TY2025's package updates them because that is their purpose, and sails past.

**Minimal change.** Make the test read what it claims to read: iterate the years
`BundledFullReturnTables::load()` actually registers (or take `FullReturnTables` as a parameter and have
a btctax-adapters test drive the real one, since core cannot depend on adapters) and assert
`p.child_tax_credit_per_child == CTC_PER_CHILD_SS24H2` for each. Then plant a second bundled year and
watch it red once, per B1. Fix the two doc comments and the report's §5 sentence at the same time.

## I-3 — `hoh_qualifying_child_name` is collected and never printed, and QSS cannot supply it

**Where.** `crates/btctax-input-form/src/spec/sections.rs:321-345`;
`crates/btctax-forms/src/form1040_full.rs:476-490`; `crates/btctax-forms/forms/2024/f1040.map.toml:128`.

**What is wrong — two halves of one gap.**

*(a) No reader.* T8 added the leaf, a `Field`, a classifier row, a `scrub` rule with a `scrub_axis`
matrix row and a `coverage` entry — and nothing anywhere writes it onto a form:

```
$ grep -rn "qualifying_child_name" crates/btctax-core/src/tax/packet.rs crates/btctax-forms/
(no matches)
```

The cell it belongs in exists and is mapped. On the TY2024 form the entry space is one shared cell —
*"If you checked the MFS box, enter the name of your spouse. If you checked the HOH or QSS box, enter
the child's name if the qualifying person is a child but not your dependent"* (`f1040--2024.txt:28-29`)
— mapped as `mfs_spouse_name = "…f1_18[0]"`. Its single write site is

```rust
if let Some(sp) = &header.spouse {
    …
    if status == FilingStatus::Mfs {
        text(w, p, &cells.mfs_spouse_name, &sp.full_name());
    }
}
```

so on a HoH or QSS return — which carries no spouse `Person` — the cell can never be reached. The
comment two lines above still reads *"(On HoH/QSS that same cell wants the qualifying CHILD's name,
**which v1 does not capture**, so it stays blank.)"* — a statement T8 falsified without updating.
TY2024 is the only year `full_return_for` returns `Some` for, so this is the filable year, and the
field's own help promises the opposite: *"enter the child's name in the entry space below qualifying
surviving spouse. If you don't enter the name, it will take us longer to process your return."*

*(b) QSS is excluded.* `live: |ri| ri.filing_status == HoH` (sections.rs:333). The form's sentence names
both — *"If you checked the **HOH or QSS** box…"* — and QSS condition 2 is satisfied precisely by a
child the filer *"could claim as a dependent except that"* their gross income reached the §152(d)(1)(B)
limit, they filed a joint return, or the filer is themselves claimable
(`i1040gi--2025.txt:1298-1306`). That is the exact household the entry space is for, and it cannot fill
it. Deviation 3 of the build report argues at length against narrowing this field because *"hiding a
cell the IRS asks for is the silent-omission direction"* — the identical argument applies to QSS and
was not made.

**Evidence.** The two greps above, plus `grep -rn "mfs_spouse_name" crates/btctax-forms/src/` returning
exactly the one guarded write and the map field declaration.

**Minimal change.** Carry the name onto `ReturnHeader` and write it into `cells.mfs_spouse_name` when
`status` is `HoH` or `Qss` and the string is non-empty (the MFS branch and this one are mutually
exclusive by filing status, so no cell is contested); widen `live`/`get`/`set` to
`HoH | Qss`; rename the leaf and `FieldId` to drop the `hoh_` prefix; and correct the emitter comment.
A KAT that fills the field on a TY2024 HoH return and reads `f1_18` back off the PDF is the kill.

## M-1 — `the_ty2024_1040_is_byte_identical_…` does not assert byte identity with pre-T8

**Where.** `crates/btctax-forms/tests/full_return_forms.rs:770-793`.

The hash assertion is `assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)))` where `a` and `b`
are two calls to `fill_form_1040_full` **in the same build**. That is a determinism tautology, not a pin
against what the emitter produced before T8; no committed pre-T8 digest is compared anywhere. The
substantive guarantee is carried by the second half of the test (the TY2024 ctc/odc boxes stay `None`),
which does discriminate — I planted the report's own defect and reproduced its red at line 790:

```
plant: check(w, p, &row.ctc, d.grid.credit == CreditColumn::ChildTaxCredit);  // in the TY2024 row loop
  left: Some("1")
 right: None
```

TY2024 is safe by construction anyway: `grep -rn "\.grid\b" crates/btctax-forms/src/` returns exactly
one reader (`form1040_full.rs:631`), inside `push_dependents_grid`, called only under
`map.dependents_grid.is_some()`, and TY2024's map declares none. So this is a naming/claim defect, not a
behavioural one. Minimal change: pin a committed digest, or rename the test to what it asserts.

## M-2 — `prompt-check` reads only quoted spans, so a T8 prompt's operative sentence can drift silently

`crates/xtask/src/prompt_check.rs`. The checker holds the *quoted* instruction spans inside a prompt. It
does not hold the prompt's own lead-in — which is the sentence the filer actually answers.

Plant, in `HohPaidOverHalfCostOfKeepingUpHome`'s first clause only (leaving the quoted span intact):
`"did you pay over half the cost of keeping up a home"` → `"did you pay most of the cost of keeping up a
home"`. *"Most"* and *"over half"* are not the same test — with three contributors, 40% can be the most.

```
$ cargo run -q -p xtask -- prompt-check
xtask prompt-check: OK — 86 assertions, all verbatim
$ cargo nextest run --locked -p btctax-core -p xtask
Summary [9.604s] 1444/1500 tests run: 1438 passed, 6 failed, 1 skipped
```

The six failures are the `form_delta` environment set; nothing else reds. For contrast, the report's own
plant — paraphrasing the *quoted* span — reproduces exactly as reported:

```
xtask prompt-check: a prompt no longer matches the form:
  question clause 4 (HohPaidOverHalfCostOfKeepingUpHome) is NOT in the string the filer reads: "You paid over half the cost of keeping up a home"
```

Recorded rather than blocking: the checker's quoted-span scope is documented, this needs a future human
edit to bite, and the same shape is the in-repo model (`cite_check`). Minimal change: quote the
operative clause in the prompt so the existing checker covers it — a one-character-class edit, no new
instrument.

## N-1 — `every_slot_caption_is_the_forms_own_words` does not check slot ORDER

`crates/xtask/src/dependents_grid.rs:512-536` asserts each caption string appears *somewhere* in the
extract. It does not assert that *"Full-time student"* is position 0 and *"Permanently and totally
disabled"* is position 1, which is the half of `SLOT_CAPTIONS` the geometric derivation cannot supply.
The module's header says the order cannot be read from the form's text layer because `pdftotext -layout`
interleaves the four columns — but the interleaving preserves order:
`f1040--2025.txt:45-47` reads `Full-time  Permanently  Full-time  Permanently …`, so a *"the first row-6
token in the label's band is Full-time"* assertion is available. A caption swap would today print
*"permanently and totally disabled"* for a filer who said *"full-time student"* with every instrument
green — the same defect class `a_transposed_pair_reds` exists to catch, reached by editing the table
instead of the geometry.

---

## Seams checked clean

**Seam 1 — row (7) and rows (5)/(6).** `credit_column` is `_`-free and exhaustive over all seven
verdict arms; the sole entry point is `walk_dependent(ri,row).verdict.credit_column()`, so the printed
box and the refusal that let the row through are one reading. Row (7) has no `FieldId` — it cannot be
asked. The eight-row truth table plus the born-in-year row all pass. The report's kill reproduces:

```
plant: credit: CreditColumn::default()  in ReturnHeader::build
assertion `left == right` failed: the CTC edge itself: the row the emitter prints
  left: Neither / right: ChildTaxCredit
```

`date_of_birth = None` never reaches the emitter — I probed it directly and the unanswered tier
refuses: `Some(DependentGateUnanswered { row: 0, gate: DateOfBirth })`. (Row (5)(b) is I-1; rows (5)(a),
(6)×2 are always demanded in the Step 1 block, so they carry no equivalent staleness.)

**Seam 2 — the map and the register.** Every one of the 25 mapped cells resolves in the real TY2025
template, held by the census's phantom rule, which I watched red:

```
plant: c1_19[0] → c1_99[0]
2025/f1040: [...c1_99[0]] are named by the map or census but do not exist in the PDF — a stale entry…
```

Accounting is exact in both directions, Δ = 196 − 171 = 25:

```
plant: quote one commented identity-block FQN (the report's own stated hazard)
2025/f1040: recorded 171 unaccounted field(s), measured 170. The register is SHRINK-ONLY…
plant: leave the register at 196
2025/f1040: recorded 196 unaccounted field(s), measured 171. The register is SHRINK-ONLY…
```

`verdict()` refuses a field that is both mapped and censused, so "mapped" and "censused" cannot double-
count. Names and on-states are both held to the geometric derivation:

```
plant: swap dependent 1's two row-(6) fields in the committed map
dependent 1, FullTimeStudent / left: "…c1_21[0]" / right: "…c1_20[0]"
plant: credit_for_other_dependents on = "2" → "1"
left: "1" / right: "2"
```

The *more than four dependents* box and the statement are one decision (`header.dependents_split()`
feeds both), and the TY2025 seven-row statement is correct against the primary source: TY2025 says
*"include a statement showing the information requested in the Dependents section"*
(`i1040gi--2025.txt:1456-1459`), TY2024 says *"the information required in columns (1) through (4)"*
(`i1040gi--2024.txt:1452-1454`), and `render()` branches on `self.year >= 2025`.

**Seam 3 — TY2024.** No file under `forms/2024/` was touched by `f7b5407b`; the only fixture/doc edits
are input-side (`nra_spouse_resident_election = false`, `hoh_qualifying_child_name = ""`, the JSON dump).
Structurally safe as described in M-1. `census_accounts_for_every_field` passes over all 38 committed
maps.

**Seam 4 — HoH.** `HohMaritalBasis` is a four-variant enum matching the instruction's own three
"considered unmarried" bullets plus the unmarried base (`i1040gi--2025.txt:1144-1163`), with no serde
default and no `#[serde(other)]`. `Single`, `Mfj` **and `Mfs`** ask none of the eleven, with a positive
control per status. `MarriedLivedApart` refuses naming *MARRIED PERSONS WHO LIVE APART* and lists its
five conditions; `NraSpouseNoElection` refuses naming *NONRESIDENT ALIENS AND DUAL-STATUS ALIENS*; both
`No` answers refuse `HohTestNotMet` carrying *CHOOSE ANOTHER FILING STATUS*. The class-(A) set is derived
from each entry's own `unanswered` declaration, and `screen_filing_status_assertions` runs on **both**
tiers while the unanswered loop runs on one — the correct split. The EIC separated-spouse checkbox is
not asked and not mapped. Kills reproduce:

```
plant: class-(A) skippable loop stops refusing            → panicked at return_refuse.rs:4726
plant: MarriedLivedApart arm made unreachable             → panicked at return_refuse.rs:4797
```

**Seam 5 — QSS.** Five questions, live iff `Qss`, each refusing `QssTestUnanswered` on `None` and
`QssTestNotMet` + the exit on `No`; kill reproduces (`panicked at return_refuse.rs:4896`). The two-year
window is rendered from `tax_year` (`y-2`, `y-1`), matching *"Your spouse died in 2023 or 2024 and you
didn't remarry before the end of 2025"* verbatim for TY2025 (`i1040gi--2025.txt:1293-1297`), and the
static fallback names no year at all. The joint-rate mapping is asserted as **tax**, by the pre-existing
`tables::tests::qss_uses_mfj_schedule` and `return_1040::tests::qss_uses_married_basic_and_aged_blind_rate`
— not by the T8 test, which asserts only that the answered baseline files; that is adequate coverage,
just not new coverage.

**Seam 6 — FR-67 and the forgo.** The §6013(g)/(h) gate is live iff `header.spouse.is_some()`, refuses
`NraSpouseElectionUnanswered` on `None` and `NraSpouseElection` on `Yes` naming §6013(g), §6013(h),
worldwide income and a preparer; kill reproduces (`panicked at return_refuse.rs:4840`). Its interaction
with HoH is closed from the other side: a HoH filer whose basis is `NraSpouseNoElection` refuses at
`screen_filing_status_assertions` before liveness matters. The line-19 forgo sizes from
`params.child_tax_credit_per_child`, is absent on a params-less year, and is counted from the
**flowchart's verdict** rather than the dependent count; kill reproduces:

```
plant: each: Some(dec!(2000))
no package ⇒ no invented figure / left: Some(2000) / right: None
```

Every new refusal is anchored on a control in `attribute()`, and that match is `_`-free, so the compiler
— not a hand-list — enforces coverage.

Counts: C=0 I=3 M=2 N=1
