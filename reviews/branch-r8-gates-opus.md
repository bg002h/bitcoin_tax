# r8 — THE GATES LENS (§3 B11 · §4 R1 · §5 `_` on `Option<Usd>`)

Reviewer: independent adversarial pass, sections 3/4/5 only. Sections 1 and 2 ignored per brief.
Range: `95f7f34..HEAD` (`d7ded00`), branch `main`.

**VERDICT: 0 Critical / 3 Important**

Environment: `make check` run three times (baseline green 2559/2559; once under a planted defect —
see F3; once after restore). `target/release/btctax` rebuilt at `d7ded00` and driven end to end
against four scratch vaults.

**Tree state.** My one mutation (`return_refuse.rs`) was backed up with `cp` and restored;
`git diff -- crates/btctax-core/src/tax/return_refuse.rs` is empty, i.e. byte-identical to HEAD. My
only write is this report. NOTE for the orchestrator: a **concurrent** reviewer is working in the same
worktree — at the time I finished, `advisories.rs`, `printed.rs` and `return_1040.rs` showed
uncommitted modifications that are **not mine** (`reviews/branch-r8-ink-opus.md` is present). Because
of that shared-tree race, F3 is additionally backed by a static proof that does not depend on any
build: `grep -rn "OtherIncomeOutOfScope" crates` returns three hits, all production code
(`return_refuse.rs:158`, `:668`, `attribute.rs:46`) and none in a test.

---

## F1

```
SEVERITY: Important
WHERE: crates/btctax-cli/src/testonly.rs:120  (and its consequence, docs/examples/examples.md J10)
CLAIM: The B11 commit pasted a literal `\n` into a RAW string literal, so J10's TOML is malformed,
       `income import` now exits 2, and the whole J10 worked example — btctax's only end-to-end AMT
       demonstration — was deleted from the shipped docs by the golden regeneration that ratified it.
```

**FAILURE.** `J10_FULLRETURN_TOML` is `r#"…"#`. In a raw string `\n` is two characters, not a
newline. Line 120 reads, byte for byte (`cat -A`):

```
has_income_exclusion = false\nother_out_of_scope_income = false$
```

Driving the shipped binary against exactly those bytes:

```
$ btctax --vault v.pgp income import --year 2024 --file bad.toml
error: usage: invalid ReturnInputs TOML: TOML parse error at line 1, column 29
  |
1 | has_income_exclusion = false\nother_out_of_scope_income = false
  |                             ^
expected newline, `#`
EXIT=2
```

The committed golden `docs/examples/examples.md` was regenerated **with the failure baked in**
(`git show 35ebf4b --numstat`: `5 55 docs/examples/examples.md` — 55 lines deleted, 5 added). The
J10 section now reads:

```console
$ btctax --vault v.pgp income import --year 2024 --file fullreturn.toml
[exit 2]
```
```console
$ btctax --vault v.pgp report --tax-year 2024
Federal tax attributable to crypto — tax year 2024
  NOT COMPUTABLE [TaxProfileMissing]: no tax_profile set for 2024
…
[exit 1]
```

…while the prose immediately below it, untouched, still says:

> "…no Form 6251 need be attached … **and the return is produced in full**"
> "The **Alternative Minimum Tax** block above is the point: it shows the comparison that cleared
> them — AMTI, the exemption, the tentative minimum tax, and the regular tax it is measured against"

There is no AMT block above. The entire *Absolute filed return* section, the AMT line, the total tax,
the amount owed and the four advisories are gone from the document a user reads to learn the tool.

**EVIDENCE that the guard could not catch it.** Two instruments read this artifact and both stayed
green:

- `xtask::examples::tests::examples_golden_matches_committed` compares regeneration to the committed
  file — regenerating a *broken* journey and committing it satisfies the test by construction. It
  PASSED at 2559/2559 on my baseline run.
- `crates/btctax-forms/tests/census.rs:262` reads the committed `docs/examples/examples.md`, but
  `j6_packet_names()` is bounded to `golden.find("## J6")` and stops at the next `\n## `. J10 is
  outside its window, so the census cannot see a whole journey die.

This is the exact class `CLAUDE.md` §B1 names — an instrument that reports success while blind to
the thing it exists to protect.

**Why Important and not Critical:** no wrong mark reaches a filed return, and no real filer's return
is refused — the broken bytes live in `testonly.rs`. The damage is a shipped user-facing document
that demonstrates the tool failing, and the silent loss of the only worked AMT example.

**Note:** J6 is unaffected — `J6_FULLRETURN_TOML` is `include_str!("../tests/fixtures/examples/fullreturn_inputs.toml")`
and that fixture got the new key on its own line (line 23). Only J10's inline literal is damaged.

---

## F2

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/advisories.rs:465-471 (`ctc_provably_zero`)
CLAIM: `ctc_provably_zero` open-codes Schedule 8812 line 3 instead of calling
       `ReturnInputs::modified_agi()`, so it consumes the four §911/931/933 add-backs WITHOUT the
       answered-ness gate that gives them meaning — and on TY2024, the only computable year, that
       gate is never asked. The result is a filer told "there is no Schedule 8812 for you to file"
       when Schedule 8812 gives them the credit.
```

**FAILURE — reproduced against the shipped binary.** MFJ, one dependent, the committed J6 fixture
with `[schedule_c]` dropped and one field changed to `form_2555_line45 = "200000"`:

```
$ btctax --vault b.pgp income show --year 2024 | jq
{'has_income_exclusion': None, 'form_2555_line45': '200000',
 'excluded_puerto_rico_income': '0', 'other_out_of_scope_income': False}

$ btctax --vault b.pgp report --tax-year 2024
  AGI (L11):                296500.00
  • CTC/ODC NOT COMPUTED, AND NOT AVAILABLE TO YOU — you captured 1 dependent(s) … your income
    phases the credit out entirely under §24(b), whatever your dependents' ages … 1040 line 19 is
    $0 and that is the correct figure — there is no Schedule 8812 for you to file.
```

The identical vault with `form_2555_line45 = "0"` prints the other arm ("Your tax is OVERSTATED …
File Schedule 8812 yourself to claim it.").

Worked against the archived form (`design/forms/extract/f1040s8--2024.txt`) for a filer whose stored
$200,000 is **not** an affirmed Form 2555 exclusion:

| Sch 8812 | value |
|---|---|
| 1 (1040 L11) | 296,500 |
| 2a/2b/2c | 0 |
| 2d, 3 | 0 · 296,500 |
| 9 (MFJ) | 400,000 |
| 10 ("If zero or less, enter -0-") | **0** |
| 11 | 0 |
| 8 ceiling | 2,000 |
| 12 "Is line 8 more than line 11?" | **Yes** ⇒ credit available |

btctax says $0 and tells the filer not to file the schedule.

**EVIDENCE.** The code:

```rust
fn ctc_provably_zero(ri: &ReturnInputs, dependents: usize, agi: Usd) -> bool {
    // L2d + L3 — the modified AGI the phase-out actually reads.
    let l3 = agi
        + ri.excluded_puerto_rico_income
        + ri.form_2555_line45
        + ri.form_2555_line50
        + ri.form_4563_line15;
```

The codebase's own single accessor for that identical quantity —
`crates/btctax-core/src/tax/return_inputs.rs:706` — refuses to do this:

```rust
pub fn modified_agi(&self, agi: Usd) -> Option<Usd> {
    match self.has_income_exclusion {
        None => None, // never asked — a caller that needs MAGI must refuse, not assume zero
        Some(false) => Some(agi),
        Some(true) => Some(agi + self.excluded_puerto_rico_income + …),
```

and `crates/btctax-input-form/src/spec/sections.rs:120-129` states the rule the new function breaks:

> "★ ONE quantity, FIVE phase-outs. … The yes/no that carries answered-ness is
> `QuestionId::HasIncomeExclusion`, in the Declarations section; **these are the amounts it gates**."

**Why it is reachable and not contrived.** `HasIncomeExclusion` is year-scoped —
`crates/btctax-core/src/tax/questions.rs:504`, `live: |ri| ri.tax_year >= 2025` — so on TY2024, the
only year btctax computes, the gate is **never asked at all**: `income answer` skips it (its own test
`a_ty2024_single_filer_is_not_asked_the_ty2025_magi_question` pins this), and the Declarations
tri-state is inert (`decl_tristate!`'s `get` returns `None` and `set` returns `NoSuchRow` when not
live). Meanwhile the four amounts are an **unconditional, always-live input section** —
`registries.rs:224` `INCOME_EXCLUSIONS` is in `form_spec()` with no year gate, and `ret_money!`
(`sections.rs:99-118`) hardcodes `live: |_| true`. So btctax presents four money boxes titled
"Income exclusions (§911/931/933)", never asks the question that makes them testimony, and — as of
`fa0559b` — is now the *only* TY2024 consumer of whatever lands in them.

The failure direction is the one the commit message itself calls "the more dangerous one":
`l3` can only move **up**, so an unaffirmed amount can only turn a real credit into "you get nothing".
`fa0559b`'s own test `the_line_2_add_backs_count` pins this exact behaviour as correct.

**Fix shape:** call `ri.modified_agi(agi)` and treat `None` as "cannot prove zero" (return `false`) —
which is the conservative arm and needs no new input.

---

## F3

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_refuse.rs:665-676
CLAIM: The B11 `Some(true)` refusal — the gate that actually stops the filer who ANSWERS YES to
       out-of-scope income — has no test anywhere. MUTATION-VERIFIED: deleting the whole block
       leaves the full gate green at 2559/2559.
```

**FAILURE.** With the block removed (backed up to `/tmp/rr_backup.rs`, restored with `cp`):

```
     Summary [  18.624s] 2559 tests run: 2559 passed, 12 skipped
```

Identical to baseline. With that block absent, a filer who truthfully answers **yes** — the Pub 559
filer with $8,183 of net rental income, the entire motivating case — computes and files a return
omitting §61 income. That is precisely the understatement B11 exists to prevent, and nothing in the
suite would notice its removal.

**EVIDENCE.** `grep -rn "OtherIncomeOutOfScope" crates` returns exactly three hits, all production:
the enum variant (`return_refuse.rs:158`), the refusal itself (`:668`), and the attribution arm
(`btctax-input-form/src/attribute.rs:46`). No test file names it.

The `None` leg, by contrast, IS covered — the `FORM_QUESTIONS` property test blanks each registry
entry in turn and asserts that entry's reason — which is what makes the asymmetry visible: the
registry gave the unanswered half a test for free, and the half that needed a hand-written one never
got it.

`CLAUDE.md` §B1: *"No checker exists until it has been observed RED on a planted defect."* The
reviewable question it poses — *"which test reds when this checker is removed?"* — has the answer
**none**.

---

## F4

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/advisories.rs:473-486
CLAIM: `ctc_provably_zero` hardcodes three year-sensitive §24 constants and ignores the `year` and
       `params` its caller already holds.
```

`$2,000` (line 486), `$400,000`/`$200,000` (473-477) are literals. `advisories()` receives both
`year: i32` and `params: &FullReturnParams`, and every other year-sensitive constant in this codebase
(`std_aged_blind_married`, the wage base, the AMT exemption) lives in `FullReturnParams`. The
function is correct for TY2024 and unreachable for other years today only because
`advisories_for` runs on a computed return and TY2024 is the only computable year — an accident of
scope, not a stated invariant. The failure direction when a later year is bundled is a ceiling that is
too small, i.e. an over-eager "you get nothing". A `debug_assert_eq!(year, 2024)` or reading the
constants from `params` closes it.

---

## F5

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/return_refuse.rs:673-675
CLAIM: The OtherIncomeOutOfScope refusal ends by advising the filer to "remove that income and file
       the rest yourself", which read literally is the understatement the previous sentence condemns.
```

Full text: *"… and a return that silently left it off would understate your tax. File with a
preparer, **or remove that income and file the rest yourself**."*

There is no field holding the income, so "remove that income" has no referent in the app. The
intended reading is "if you were mistaken and have none, answer No"; the available reading on a
return signed under §6065 is "leave the rental off and file the rest". On the one message whose whole
purpose is to stop an omission, the last clause should not be ambiguous about omitting.

---

## F6

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/classifier.rs:614-624 (the new rule's inputs)
CLAIM: The `Option<Usd>` rule reads ONLY `return_inputs.rs`, but the classifier destructures a struct
       defined elsewhere with `_` bindings today — so the rule cannot see the case its own neighbour
       comment says it exists for.
```

**EVIDENCE.** The rule's field set is `option_money_fields(include_str!("return_inputs.rs"))`
(`classifier.rs:618`). The classifier recurses into 16 structs; 15 are declared in
`return_inputs.rs`, and one is not:

```rust
// crates/btctax-core/src/tax/classifier.rs:515
fn classify_carryforward(_c: &mut Census, cf: &Carryforward) {
    // FROZEN struct — destructuring it READS it, modifies nothing (§3.3). No classifiable leaves today; the
    // guarantee is about the bool added tomorrow.
    let Carryforward { short: _, long: _ } = cf;
}
```

`Carryforward` is declared at `crates/btctax-core/src/tax/types.rs:23`. Because
`option_money_fields` never reads that file, no field of `Carryforward` can ever appear in the
violation set — so `pub unrecaptured_1250: Option<Usd>` added there tomorrow and bound `_` here is
green by construction, and the comment sitting directly above it ("the guarantee is about the …
added tomorrow") is exactly the guarantee the new rule does not extend to it.

Two smaller evasions in the same parser, for completeness:
- `Usd` is a type alias (`crates/btctax-core/src/conventions.rs:8`, `pub type Usd = Decimal;`), so a
  field written `Option<Decimal>` (or any alias/path form) is not matched by the literal
  `== "Option<Usd>"` comparison.
- `option_money_fields` requires the line to begin `pub ` after trimming, so a same-line attribute
  (`#[serde(default)] pub x: Option<Usd>,`) or a rustfmt-wrapped multi-line declaration slips past.

The false-positive directions (a doc comment in `classifier.rs` containing `foo: _,`; two structs
sharing a field name) only produce a spurious RED, which is the safe way to be wrong. The blind spot
above is the unsafe way.

---

## F7

```
SEVERITY: Nit
WHERE: crates/btctax-input-form/src/spec/mod.rs:69-72, 118
CLAIM: The counts around the declarations test were bumped in the assertions but not in the strings
       and doc comment.
```

`assert_eq!(decls.fields.len(), 13, "11 declarations + foreign_country_names")` — the message says 11
while asserting 13 (12 + 1). The doc comment above still says "**10** `Decl*` declarations (the 11th
— the mortgage box …)" and "TOTAL over all 11 questions"; both are two behind. The nearby comment
"that's 7" (line 88) was already stale before this range.

---

## F8

```
SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/return_refuse.rs:663-666
CLAIM: The B11 block was inserted between an existing comment and the `if` it documents.
```

```rust
// ★ P9 §2.5 (r5 I-3) — a truthful dual-status "yes" is UNSUPPORTED. VALUE-refusal (`Some(true)`);
// WITHOUT it a "yes" computes, taking the standard deduction §63(c)(6)(B) denies a nonresident alien.
// ★★★ §G-22 / B11 — the filer AFFIRMED income this version cannot model.
if ri.other_out_of_scope_income == Some(true) {
```

The dual-status comment now sits above the B11 gate; `if ri.dual_status_alien == Some(true)` is
eleven lines further down with no comment.

---

## F9

```
SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/advisories.rs:214-219 (the provably-zero advisory text)
CLAIM: The parenthetical overstates the test the code performs.
```

The message says "Schedule 8812 line 11 **already exceeds** the most line 8 could be". The code is
`l8_ceiling <= l11`, which includes equality — and equality is *exactly* the boundary the commit's own
test pins ($479,001 MFJ / 2 dependents ⇒ L11 4,000 = ceiling 4,000). The arithmetic matches the form
("Is the amount on line 8 **more than** … line 11?" ⇒ No when equal); only the sentence is imprecise.
"is at least" would be exact.

---

## ALSO CHECKED, SOUND:

**R1 — the ceiling argument itself.** It holds. Schedule 8812 line 4 counts qualifying children under
17 with the required SSN and line 6 counts other dependents *excluding* anyone on line 4, so
`line4 + line6 ≤ header.dependents.len()` and `dependents × $2,000` is a true ceiling on line 8
($500 ≤ $2,000). I could not construct a dependent composition that beats it. Note also that a "No"
on line 12 kills the refundable side too — the form says "Skip Parts II-A and II-B. Enter -0- on lines
14 **and 27**" — so the advisory's claim is not narrower than the credit it discusses.

**R1 — line 10's rounding, verified against the extracted text.** `design/forms/extract/f1040s8--2024.txt:33`:
*"If more than zero and not a multiple of $1,000, enter the next multiple of $1,000. For example, if
the result is $425, enter $1,000; if the result is $1,025, enter $2,000, etc."* `(over/1000).ceil()*1000`
reproduces both printed examples. The `if over.is_zero() { return false }` early exit matches "If zero
or less, enter -0-".

**R1 — the boundary.** MFJ / 2 dependents: 479,000 ⇒ L10 79,000 ⇒ L11 3,950 < 4,000 ⇒ credit survives;
479,001 ⇒ L10 80,000 ⇒ L11 4,000, not *more than* the ceiling ⇒ zero. Matches the form and the
committed test.

**R1 — the thresholds.** `f1040s8--2024.txt:26-27`: "• Married filing jointly—$400,000 • All other
filing statuses—$200,000". The `if Mfj {400k} else {200k}` is right for MFS, HoH, Single **and** Qss
(a QSS return is not a joint return; the form's "all other" covers it).

**R1 — `agi` is the right quantity.** `advisories_for` passes `ar.agi`, documented and used as 1040
L11, which is what Schedule 8812 line 1 asks for.

**R1 — the four add-backs map correctly** to Schedule 8812 lines 2a/2b/2c when they ARE affirmed
(`excluded_puerto_rico_income`→2a, `form_2555_line45`+`line50`→2b, `form_4563_line15`→2c). The defect
in F2 is the missing gate, not the mapping.

**B11 — the TUI (`btctax-tui-edit`) is NOT stranded.** It drives entirely off
`btctax_input_form::form_spec()` (`edit/tax_inputs.rs:468,662,736`), and the new leaf is a
`decl_tristate!(12, FieldId::DeclOtherOutOfScopeIncome, …)` in `DECL_FIELDS`, so it renders and
toggles like every other declaration. `FORM_QUESTIONS[12]` is verified to be `OtherOutOfScopeIncome`
(the array order is pinned by `questions.rs`'s completeness test). `attribute.rs:46` maps BOTH
refusal reasons to that declaration, and `resolve_field_anchor` therefore moves the cursor to it, so
a refused commit is navigable. `spec/coverage.rs` went 81→82 covered leaves, so the field is claimed
by exactly one Field.

**B11 — the defensive-filing wizard is unaffected.** `cmd/defensive.rs` calls
`btctax_core::defensive::journey_view` over the ledger projection and never touches `ReturnInputs`,
`tax_profile` or `screen_inputs`.

**B11 — the migration path works, and I drove it.** A pre-B11 TOML (the committed fixture with the
new key deleted) still **imports cleanly** (`#[serde(default)]` ⇒ `None`), then:

```
$ btctax --vault v.pgp report --tax-year 2024
error: usage: tax year 2024 cannot be computed from its full-return inputs: … Silence is not
testimony that there is none: answer it — run `btctax income answer`; run `income clear --year 2024`
to remove them and use a raw `tax-profile`
```

`btctax income answer --year 2024` then asks it as the only question with no default (`[y/n]` rather
than `[y/n, currently n]`), and the year computes. Critically, `answer_return_inputs` writes with
`return_inputs::set` **directly**, not through `input_form_store::commit` — so the screening commit
path cannot deadlock a return whose only defect is the unanswered question. I verified the round trip.

**B11 — `optimize` / `what-if` hard-error on a legacy vault, with the same actionable message.**
`resolve_core` (`resolve.rs:96`) is the single profile ladder; a screen refusal yields `profile: None`,
and `Session::resolve_screened_profile` turns that into `Err(CliError::Usage(detail))`. Confirmed live:
`what-if sell` and `optimize run` both print the B11 message. This is not a new class — any screen
refusal already did this — but B11 widens the population to **every** stored return authored before
`35ebf4b`. The message names the fix, so I do not file it. Two related observations, neither a
finding: `export-snapshot` silently omits Schedule SE on an uncomputable year (`admin.rs:161-163`,
`544-546`, pre-existing), and because the registry loop runs early, B11 now pre-empts every
value-dependent refusal — the code already disclaims refusal precedence as contract.

**B11 — the `None` leg and the always-live property ARE pinned.** `screen_inputs`'s registry loop
(`return_refuse.rs:598-602`) is exercised by the per-question property test that blanks each
`FORM_QUESTIONS` entry and asserts that entry's reason, and
`cmd/answer.rs`'s `a_single_filer_is_asked_the_always_live_declarations_and_no_spouse_question`
asserts the exact ordered vector including `OtherOutOfScopeIncome`, so `live: |_| true` cannot be
narrowed silently. Only the `Some(true)` leg is naked (F3).

**B11 — no filing shape where asking is wrong.** A year with no `income import` never reaches
`screen_inputs`; a parked return resolves through `tax_profile`; the raw `tax-profile` escape hatch is
untouched. The question is answerable by every filer without a form in hand.

**B11 — the fixture answers do not mask anything.** The six touched fixtures all set
`Some(false)`, which is the neutral-for-every-other-assertion value; the golden matrix md5 is
unchanged and no advisory or refusal test lost its subject. The one fixture change that DID break
something is F1, and it broke by malformation rather than by masking.

**§5 — the three planted kills genuinely discriminate.** I read the rule as a pure function: `_`
reds, `_unused` reds, a bound name stays silent, plain `Usd` keeps its permission. The
alphanumeric name filter (rather than lowercase-only) is load-bearing for `form_2555_line45` and
`qbi_w2_wages` and is pinned. The `Option<Usd>`-in-a-nested-struct hole the brief asked about is real
and is F6; the doc-comment case fails safe.

---

## WHAT WOULD MAKE THIS REVIEW WRONG:

1. **F2 turns on a judgement about intent.** If the project's position is that anything typed into
   the always-live "Income exclusions (§911/931/933)" section is testimony *whether or not* the gate
   was asked, then `ctc_provably_zero` is right and only `modified_agi`'s `None` arm is
   over-cautious. I read the codebase's own doc — *"The gate carries the answered-ness, not the
   amounts, and that is deliberate"* — as settling it the other way, but that is a reading, not a
   compiler error. The cheap resolution is a one-line change (`ri.modified_agi(agi)`), so the cost
   of my being wrong is near zero and the cost of being right is a filer talked out of $2,000.
2. **F1's severity depends on how `docs/examples/examples.md` is consumed.** I found no
   `include_str!` shipping it inside a crate, so it is a repo/GitHub document rather than something
   installed with the binary. If it is in fact bundled into a release artifact, F1 rises; if the
   project treats generated example docs as disposable, it falls to Minor. The *fact* — a raw-string
   `\n`, a hard `exit 2`, and 55 deleted lines that the golden test ratified — is not in doubt; only
   its weight is.
3. **F3 is an absence proof over the whole suite.** I established it by deletion + `make check`, not
   by reading every test. `make check` is nextest + clippy only and does not run the CI-only jobs
   (fmt / msrv / pii-scan / net-isolation); none of those could plausibly assert a refusal, but I did
   not run them.
4. **I did not attack the emitted PDF.** B11 and R1 both move no ink by construction (an advisory
   string and a refusal), and I took that as given rather than re-deriving it from the AcroForm map.
   If either can reach a cell, my whole severity scale for sections 3 and 4 is one notch too low.
5. **I did not re-derive the findings these commits answer.** Per the brief, the $3,894
   understatement, the CTC misdirection at AGI $2,085,000, and Pub 559's unaskable rental income are
   taken as real and correctly diagnosed.
