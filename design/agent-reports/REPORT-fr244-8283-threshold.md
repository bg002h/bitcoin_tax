# REPORT — FR-244: the Form 8283 attachment threshold is measured BEFORE the §170(b) ceiling

**Status: FIXED, gates green, NOT committed.** The work is in the worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a0d1fa6cfda7f1d05`, four files modified, nothing
staged, no commit made.

    crates/btctax-core/src/tax/charitable.rs   |  69 ++++++++
    crates/btctax-core/src/tax/packet.rs       | 255 +++++++++++++++++++++++++++++-
    crates/btctax-core/src/tax/printed.rs      |  19 ++-
    crates/btctax-core/src/tax/return_1040.rs  | 170 ++++++++++++++------
    4 files changed, 458 insertions(+), 55 deletions(-)

---

## 1. The fix, in one line

`assemble_printed_forms` decided Form 8283's presence from the **post-ceiling** printed Schedule A
line 12; it now decides it from the **pre-ceiling** noncash total, which is what both instruction
booklets define the $500 threshold to mean. The trigger is still a total; only the measure moved.

## 2. The quantity used, and where it comes from

**`AbsoluteReturn::noncash_gifts_before_limits`** — a new `Usd` field, computed in
`assemble_absolute` from **the very `gifts` slice that is then handed to `apply_170b`**:

    crates/btctax-core/src/tax/return_1040.rs (assemble_absolute)
        gifts.extend(crypto_charitable_gifts(state, year));
        let noncash_gifts_before_limits = crate::tax::charitable::noncash_before_limits(&gifts);
        let charitable = apply_170b(agi, &gifts, &ri.charitable_carryover_in, year);

That is deliberate and is the answer to *"find the pre-ceiling quantity rather than inventing one."*
`gifts` is already the full pre-ceiling gift set — the filer's `ri.schedule_a.charitable` entries plus
`crypto_charitable_gifts(state, year)` (LT legs at FMV → `CapGainProp30`, ST legs at §170(e) basis →
`OrdinaryProp50`). Taking the measure off that same slice one line before `apply_170b` consumes it
means the attachment measure and the §170(b)-limited figure **cannot be computed over two different
gift sets**. Nothing re-derives it: the field is read, never recomputed, at both readers.

Two new functions in `crates/btctax-core/src/tax/charitable.rs`:

- `noncash_before_limits(gifts: &[CharitableGift]) -> Usd` — the measure, carrying the three archived
  passages verbatim in its doc comment (i8283:46-52, i1040sca:1255-1259, f8283 header), plus the
  adjudication of the line-12 tension and the note that the trigger stays a total.
- `is_noncash(class: CharitableClass) -> bool` — an **`_`-free match** over the six variants. Both
  sites that needed the cash/noncash split previously used the typed negation
  `!matches!(g.class, Cash60 | Cash30)`, which silently sorts a *seventh* class into "noncash". A new
  variant is now a build error in one place (FR-99). `screen_compute_dependent`'s
  `NonCryptoNoncashGift` guard was switched onto it; its behaviour is unchanged (the same six variants
  partition the same way today).

★ I did **not** touch what line 12 itself is. The ceiling genuinely limits the deduction and the
excess genuinely carries over. Only the attachment decision's input changed.

## 3. The lockstep change nobody could skip — and the one premise that deliberately did not move

`screen_absolute`'s §G-21 restriction gate has a premise that decides whether Form 8283 Section B
lines 5a/5b/5c **actually print**, and the source comment above it is explicit that it must be keyed
on *"the quantity `packet.rs` ITSELF filters on, so the gate and the packet cannot disagree about
whether a Form 8283 exists."* So that premise moved with the filter:

    -   claimed_noncash > FORM_8283_THRESHOLD && donated > QUALIFIED_APPRAISAL_THRESHOLD
    +   ar.noncash_gifts_before_limits > FORM_8283_THRESHOLD && donated > QUALIFIED_APPRAISAL_THRESHOLD

**The gate's FIRST premise (`claimed_noncash > $0`, which gates the DECLARED arm) deliberately did
NOT move**, and this is the judgment call in the change. That premise is keyed on the *deduction*, not
on the paperwork: Reg §1.170A-7 reduces a **claim**, and a ceiling-zeroed year claims nothing, so a
declared restriction moves no figure on it. Re-keying it to the pre-ceiling measure would reinstate
exactly the r3 defect recorded in that test's own doc — *"escapable only by a false 'No' under §6065
or by deleting a truthful ledger event."* That block is not reintroduced.

★ **A structural consequence, stated because it is not obvious and it removed a test row.** With the
first Section-B conjunct now pre-ceiling, it is **entailed by** the second: the crypto aggregate
`donated` sums is a *subset* of `noncash_gifts_before_limits`, so `donated > $5,000` forces
`pre-ceiling total > $5,000 > $500`. The two conjuncts are therefore no longer independently
falsifiable, and **no fixture can invent a household where the $500 term does the work.** I kept the
conjunction (dropping a term would be the compression `CLAUDE.md` forbids), stated the entailment in
the test doc, and turned the row that used to falsify the $500 term into the **guard on the
entailment**: it now asserts REFUSE, and reds if `noncash_gifts_before_limits` ever stops carrying the
crypto aggregate. The $5,000 conjunct is still falsified on its own, so a gate keyed on the $500 term
alone still fails.

## 4. B1 — "which test reds when this is reverted?"

**The packet filter.** Exactly one test, and it is the new one:
`btctax-core tax::packet::tests::form_8283_attaches_on_the_pre_ceiling_noncash_total_not_the_printed_line_12`.
Measured by reverting only `packet.rs`'s filter to `.filter(|a| a.line12 > FORM_8283_THRESHOLD)` and
running the whole core lib (`cargo nextest run -p btctax-core --lib --no-fail-fast`):

    Summary [0.537s] 820 tests run: 819 passed, 1 failed, 0 skipped
    FAIL (330/820) btctax-core tax::packet::tests::form_8283_attaches_on_the_pre_ceiling_noncash_total_not_the_printed_line_12

    thread '...' panicked at crates/btctax-core/src/tax/packet.rs:2399:13:
    assertion `left == right` failed: cash crowds the ceiling to $300: contributed property
    (pre-ceiling) = 50000, printed Schedule A line 12 = 300. Form 8283 attaches when the PRE-ceiling
    total is over $500, because both instruction booklets define this threshold's measure as the
    deduction "before applying any income limits".
      left: false
     right: true

**The near-miss, i.e. the anti-"always attach" half.** Dropping the threshold to `> Usd::ZERO` —
"always attach, wearing a measure's clothes" — reds the same test on the boundary row:

    thread '...' panicked at crates/btctax-core/src/tax/packet.rs:2399:13:
    assertion `left == right` failed: slack ceiling, exactly $500: contributed property (pre-ceiling)
    = 500, printed Schedule A line 12 = 500. Form 8283 attaches when the PRE-ceiling total is over
    $500, ...
      left: true
     right: false

**The gate's lockstep premise.** Reverting *only* the `return_1040.rs` premise back to
`claimed_noncash` reds two tests, both on the same household:

    Summary [0.533s] 820 tests run: 818 passed, 2 failed, 0 skipped
    FAIL (470/820) tax::return_1040::tests::an_itemizer_whose_170b_ceiling_zeroes_the_gift_claims_nothing_and_is_not_blocked
    FAIL (476/820) tax::return_1040::tests::an_unanswered_restriction_question_refuses_on_a_section_b_year_only

    panicked at crates/btctax-core/src/tax/return_1040.rs:4544:9:
    assertion `left == right` failed: the ceiling holds line 12 at $480, but $50,000 of contributed
    property still files a Section B Form 8283 — and btctax holds no answer to the three boxes on it
      left: None
     right: Some(DonationRestrictionsUnresolved)

### The plant is the household's CEILING POSITION, not the predicate (FR-235)

The kill's defect row does not mention $500 or the threshold anywhere. It moves **where the §170(b)
ceiling lands**, by giving the household a large *cash* gift: `CapGainProp30`'s ceiling is
`min(30% × AGI, 50% × AGI − allowed cash)`, so $29,700 of cash at $60,000 of AGI leaves **$300** of
50%-room and Schedule A line 12 prints $300 on **$50,000** of contributed property. The two measures
then disagree by construction — and the test *asserts* that they do rather than assuming it (it
classifies each row from the two measures, never from the expectation).

Four rows, and three `saw_*` flags that fail the test if any shape stops being exercised:

| row | cash gift | contributed | line 12 | 8283 | shape |
|---|---|---|---|---|---|
| cash crowds the ceiling to $300 | $29,700 | $50,000 | $300 | **yes** | measures DISAGREE — the kill |
| slack ceiling, $6,000 donated | — | $6,000 | $6,000 | yes | agreeing-YES (kills "never attach") |
| slack ceiling, exactly $500 | — | $500 | $500 | **no** | boundary — the form says *over* $500 |
| slack ceiling, $400 donated | — | $400 | $400 | no | agreeing-NO (kills "always attach") |

### B1a — what makes the checker's subject present

Every row asserts, before it asserts anything about the form: the household **itemizes**
(`ar.deduction_is_itemized` — the other term of the filter, so it is never the thing under test); the
ledger **yields Form 8283 rows** (`!crate::forms::form_8283(&state, 2024, &BTreeMap::new()).is_empty()`,
without which a presence assertion passes for a reason unrelated to the threshold); the pre-ceiling
measure **equals the contributed amount**; and `screen_absolute` returns `None`, so the kill is on a
household btctax actually **files** rather than one it refuses anyway.

## 5. Goldens — measured, and the answer is that NO page set changes

The brief anticipated goldens gaining a Form 8283 page, so I measured rather than inferring it from a
green suite. A temporary diagnostic (since removed) walked **111 fixture households** — the four
`testonly` households (`kitchen_sink`, `w2_only`, `amt_owing`, `every_money_leaf`) plus all **107**
golden households from `crates/btctax-core/tests/goldens/full_return_goldens.json` via
`build_golden_household` — and printed, for each, the pre-ceiling noncash total, the printed line 12,
whether an 8283 attaches, and how many `Donation` removals its ledger holds:

    --- households with ANY noncash: see above; measures-disagree count = 0
    --- total households scanned = 111

**Not one line printed above that summary.** The print was guarded by
`gross > 0 || donation_removals > 0 || f8283.is_some()`, so the measurement is that **no committed
golden or fixture household has any noncash contribution at all** — no donation removal, no noncash
`CharitableGift`, no Form 8283 in the packet. Corroborated independently from the data:

    $ python3 -c "... json.load('full_return_goldens.json') ... keys matching char|don|noncash|crypto|gift"
    ['charitable_cash']

The 107-household oracle corpus carries **only** `charitable_cash`. Nothing in it can gain a Form 8283
under any measure, so the "regenerate to match a breakage" trap has no surface here — there was no
golden to re-bless. The households that *do* donate are built at the call site (`attestation.rs`
pushes `crypto_gift_5k()`; `census.rs` injects an 8283 arm explicitly), and neither sits in the
disagreeing band: attestation's itemizing twin has $5,000 contributed against a $9,000 ceiling at
$30,000 of AGI, so line 12 is $5,000 and both measures said "attach" before and after.

The regenerable goldens that *would* have absorbed a change silently were checked by their own
content-property tests and stayed green: `examples_golden_matches_committed`,
`walkthrough_console_golden_matches_committed`, `no_worked_example_shows_a_command_that_errored`,
`derived_form_set_reproduces_the_twelve_anchors`.

**So the only two tests whose expectations moved are the two §3 lockstep guards**, and both moved
because the *authority* moved their premise, not because output broke. Justified by household:

**(a) `an_unanswered_restriction_question_refuses_on_a_section_b_year_only`** — Single, AGI $1,600,
$20,000 of Form 1098 interest, $10,000 SALT; **contributed $50,000**; **printed line 12 $480**.
`None` → `DonationRestrictionsUnresolved`. Why the new answer is right: $50,000 > $5,000 ⇒ **Section B**
(§170(f)(11)(C)); $50,000 > $500 measured pre-ceiling ⇒ the 8283 **is filed** (i8283 / i1040sca,
*"before applying any income limits"*). Its lines 5a/5b/5c are therefore printed and the filer never
answered them. The refusal's own sentence — *"this year files a Form 8283 SECTION B"* — is now literally
true of this return; before the fix it was false, which is why `None` was right then.

**(b) `an_itemizer_whose_170b_ceiling_zeroes_the_gift_claims_nothing_and_is_not_blocked`** (its
`band(None)` row) — Single, AGI $1,600, $20,000 of Form 1098 interest; **contributed $50,000**;
**printed line 12 $480**. `None` → `DonationRestrictionsUnresolved`, same household and same reason.
The row's premise was *"no 8283 attaches, so 5a/5b/5c are never printed"* — a true statement about a
defective packet. ★ This is **not** the r3 block returning: that block had no honest exit (a false "No"
under §6065, or deleting a truthful event), whereas this filer is being asked a printed question they
can simply answer. The test's `Some(true)` and `claiming(...)` rows are untouched, so the r3 regression
guard is intact.

★ That second test was **renamed** (`..._files_no_8283_and_is_not_blocked` →
`..._claims_nothing_and_is_not_blocked`) because its old name asserted something the fix makes false:
its rows contribute $4,000 and $50,000 of property, so they now *do* carry a Form 8283. The subject it
guards — a ceiling-zeroed year is not blocked — is unchanged.

## 6. Gate — run SERIALLY, in the foreground, on the final tree

    $ cargo nextest run --workspace
        Summary [  23.874s] 3822 tests run: 3822 passed, 12 skipped

    $ cargo clippy --workspace --all-targets -- -D warnings
        Finished `dev` profile [optimized + debuginfo] target(s) in 3.75s        # exit 0

    $ cargo fmt --all --check
                                                                                # exit 0

`cargo fmt --all --check` failed once (one wrapped `let` in `return_1040.rs`); `cargo fmt --all` fixed
it and all three gates were then re-run to green on the final tree, in that order, never concurrently.

★ **Nothing for the controller to move.** No `RefuseReason` variant was added, so `xtask blockers`'
pinned census count is unchanged, and all 260 `xtask` tests pass inside the workspace run above —
including `ledger_check` and the census pins. `crates/xtask/**`, `crates/btctax-oracle-harness/**`,
`design/forms/extract/**`, `legal/**`, the other `design/agent-reports/REPORT-*` files and
`FOLLOWUPS.md` were not touched.

## 7. Recommended follow-ups (NOT filed — `FOLLOWUPS.md` is the controller's)

**FU-1 (Important-shaped, PRE-EXISTING, unchanged by this fix) — a standard-deduction filer can be
refused for a Section B Form 8283 that is never filed.** `screen_absolute`'s restriction gate reads
`ar.schedule_a`, which `schedule_a_parts` builds whenever `ri.schedule_a` is `Some`, *regardless of the
§63(e) election*. So a filer who takes the standard deduction, holds Schedule A inputs, and donates
over $5,000 of crypto gets `DonationRestrictionsUnresolved` saying *"this year files a Form 8283
SECTION B"* — but no Schedule A is filed, so `packet.rs` attaches no 8283 at all. The fix is one term,
`&& ar.deduction_is_itemized`, which is exactly the itemizing term `packet.rs` already ANDs. I left it
alone because it is a **false refusal** rather than wrong output, it predates FR-244 (the pre-fix
premise `claimed_noncash > $500 && donated > $5,000` is true on the same household), and the brief
scopes this task to the measure. Reproduction: a `screened_restriction_at`-shaped fixture with the
mortgage interest dropped below the standard deduction and `donated = $9,000`. Related precedent is
this gate's own r3 finding and the phase-2 "merge Minor" that added `deduction_is_itemized` to the
appraisal manifest line in `cmd/admin.rs` for the identical reason — **that fix was never carried back
to this gate**, which is the B3 shape (*"the fix already existed in the branch"*).

**FU-2 (Minor, introduced by this fix) — `Some(true)` on a ceiling-zeroed year can now reach the
printer.** `Printed8283Rows`' doc and `form8283.rs:533` both assert *"`Some(true)` cannot arrive —
`screen_absolute` refuses the year"*. With the pre-ceiling measure that is no longer exactly true: a
filer with line 12 = `$0` (so the DECLARED arm's premise is false), contributed > $5,000, and
`donations_had_restrictions = Some(true)` now gets a Section B 8283 whose 5a/5b/5c print **blank**. A
blank is no testimony, so nothing is fabricated and this is not a wrong-output defect — but the form's
own scoping (*"Complete lines 5a through 5c if conditions were placed on a contribution listed in
Section B, Part I"*) says they should be completed, and the map only carries a `.no` field. I updated
the surrounding docs in `printed.rs` so no comment asserts an invariant the code no longer holds, and
left `form8283.rs` alone (it is `btctax-forms`, and printing a *"Yes"* needs a map field that does not
exist).

**FU-3 (Minor, fixed inline) — a swapped conjunct label.** The `$4,000` row in
`an_unanswered_restriction_question_refuses_on_a_section_b_year_only` was commented *"CONJUNCT 1
falsified"*; $4,000 clears $500, so it is the **$5,000** conjunct that is false there. Corrected while
rewriting the block, and the comments now name the conjuncts by their thresholds rather than by
position.

## 8. What I did NOT do

- No commit, no stage, no branch change. Four modified files sit in the worktree.
- No subagents or forks spawned.
- No gate command backgrounded — every one ran in the foreground to completion.
- No golden regenerated (none needed regenerating; see §5).
- The line-12-vs-pre-ceiling adjudication in the brief was implemented, not re-opened.
