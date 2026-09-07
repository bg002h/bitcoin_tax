# Seam review — interview build T3 (`0807335b` + its pre-review fold `d44f82e4`)

Independent adversarial build review, own worktree at `07fcb501`. Read-only for the record: every
plant below was made in this worktree and reverted from a `cp` backup; `git status --porcelain` is
empty at the end and the scoped suite returns to its baseline. No commits, no subagents.

---

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo build --locked -p btctax-core
cargo nextest run --locked -p btctax-core -p btctax-input-form --no-fail-fast          # seams 1, 5
cargo nextest run --locked -p btctax-core -p btctax-cli --no-fail-fast                 # seam 5 cross-crate
cargo nextest run --locked -p btctax-input-form -p btctax-core -p btctax-tui-edit --no-fail-fast
cargo nextest run --locked -p btctax-core -E 'test(review_probe_truthful_yes_refuses)' --no-capture
cargo nextest run --locked -p xtask --no-fail-fast                                     # seam 3 in-suite twin
cargo nextest run --locked -p btctax-core -p btctax-input-form -p btctax-forms -p xtask -p btctax-cli --no-fail-fast
cargo run -q -p xtask -- census-join        # ×12, once per direction-table plant
cargo run -q -p xtask -- stop-list          # ×2 (clean, then the planted progress field)
```

Restored-tree baseline: `2542 tests run: 2535 passed, 7 failed, 6 skipped` across the five crates —
the seven failures are the six `form_delta::*` and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` the brief
pre-declares as PDF-less-worktree **environment**, not findings. `census-join` and `stop-list` both
return their committed clean lines.

Also read: `design/SPEC_interview.md` §5.1, §8 (the guarantee table, lines 1216–1226), §7 row T3;
`design/agent-reports/2026-09-07-build-interview-T3-implementation.md` in full including the
"Pre-review fold (D1, D11)" appendix.

---

## Summary

**The machinery discriminates almost everywhere, and one field in it does not.** Fifteen plants were
made across the six seams; thirteen red with a message that names the defect. The two that stay green
are the same field: **`direction = "…"` in a `[[direction]]` block is the one value in the whole
table that is never read off the form** — the caption is asserted verbatim at a pinned extract line,
the range end and the overlap are asserted, and D11's flip parses its line number out of the form's
own sentence, but the direction itself is a bare judgement typed into the map with nothing behind it
and no test pinning it. Two TOML keys move Form 1040 line 1b — *Household employee wages not reported
on Form(s) W-2*, a numbered income line — under an `Advisory` cover, and `census-join` prints its
success line. Spec §8 states the guarantee in exactly the words that fail: *"the direction of a line
is read off the form, **not typed**"*.

The second cluster is D1's own premise. The fold's rule — the four 1099 rows take the W-2's three
rules — assumes each row's `Vec` can hold the document it declares. For **`b_1099`** and **`g_1099`**
it cannot, and both refuse a filer who answers truthfully: a 1099-B whose transactions belong on Form
8949 (the form's own printed option, and btctax's only crypto path) and a 1099-G reporting only box 2
(the state refund — `Form1099G` has no box-2 field, though §5.1 names one).

Nothing here is a wrong printed figure. Every finding either fails closed or laundered a line that is
correctly covered today, so the risk is in the instruments and in who gets refused, not in a number
on a filed page.

---

## Findings

### I1 — the DIRECTION of a block is typed, not read; two keys launder a 1040 income line

**Where.** `crates/xtask/src/census_join.rs:174-253` (the caption / range / overlap assertions),
`crates/btctax-forms/src/map.rs` (`DirectionBlock`), the eleven `[[direction]]` tables. Spec §8
line 1220: *"the direction of a line is read off the form, not typed | the `[direction]` table's
headings asserted verbatim against `design/forms/extract/` (T3)"*, and line 1219: *"a line whose
omission UNDERSTATES tax is never covered by an advisory"*.

**What is wrong.** Every other field of a `[[direction]]` block is anchored to the form: `caption` is
asserted verbatim at `extract_line` (K13 reproduces), `first_line`/`last_line` are range-checked and
overlap-checked (both reproduce, below), and D11's `[[subtracts]]` deliberately parses `X` out of the
sentence *"so the map cannot record a flip the form does not print"*. `direction` is the exception:
`Understates` / `Overstates` / `NoDollar` is a free choice, checked by nothing, and no test in the
workspace asserts the value of any committed block. Grep for `Direction::Understates` etc. across
`crates/` returns only the flip helper (`census_join.rs:114-116`), the rule itself (`:361`) and
synthetic in-suite fixtures (`:651,657,835`) — the real maps are never named. B1's own question,
*"which test reds when this is removed?"*, has no answer for this field.

The join's whole force flows through it. `verdict` reds an `Advisory` cover **only** under
`Understates` (`census_join.rs:361`), and the REACH check applies **only** to a `QuestionId` cover;
downgrade the block and both rules switch off for every entry in it at once.

**Evidence — the plant and the observed output.**

*Plant 1 (one key).* `2024/f1040.map.toml`, the `Income` block: `direction = "Understates"` →
`direction = "NoDollar"`. That block heads Form 1040 lines **1a–9** — wages, interest, dividends,
capital gain.

```
$ cargo run -q -p xtask -- census-join
census join: 298 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract and covered by an existing variant
```

*Plant 2 (the laundering, two keys).* The same downgrade, plus `2024/f1040.map.toml` line 1b —
`"Household employee wages not reported on Form(s) W-2"` — moved from
`covered_by = "QuestionId::OtherOutOfScopeIncome", names = "Household employee wages"` to
`covered_by = "Advisory::UnmodeledDeductionsOmitted"`:

```
$ cargo run -q -p xtask -- census-join
census join: 298 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract and covered by an existing variant

$ cargo nextest run --locked -p xtask --no-fail-fast
Summary [23.414s] 151 tests run: 144 passed, 7 failed, 1 skipped     # the 7 = the PDF-less environment
```

`the_committed_maps_are_covered_and_placed` — K10/K11/K12/K13's in-suite twin — passes throughout.
An income line the filer must be *asked* about is now announced only by an advisory they may ignore,
and every instrument reports success.

*Plant 3.* `Understates → Overstates` on the same block plus the same cover: also **green**. So the
hole is not specific to `NoDollar`; any downgrade works.

**Minimal change.** Two parts, both derived rather than listed:

1. **Structural, one line, kills plant 1.** A block carrying `first_line`/`last_line` may not be
   `NoDollar` — `NoDollar` means *carries no dollar*, and a line range is precisely the claim that it
   does. Verified against the committed tree: all six `NoDollar` blocks have no range, and every
   ranged block is `Understates` or `Overstates`, with no exception (the one rangeless non-`NoDollar`
   block is Schedule 1's unnumbered 1099-K line, which uses `part`).
2. **Anchored, kills plants 2 and 3, and is symmetric with D11.** Read the sign off the form the way
   `subtracted_line()` already does: the extract prints the block's own total sentence — f1040s1:56
   *"Combine lines 1 through 7 and 9. …"* into Schedule 1 line 10, f1040s3 *"Add lines 1 through 5…"*
   into a credit total. Assert that a block whose lines are ADDED into a total the form routes to
   taxable income or to tax is `Understates`, and one routed to a credit/payment/deduction total is
   `Overstates`. Same shape as the flip: parse it out of the form's sentence, never type it beside it.

Until (2) exists, at minimum record the guarantee's real coverage — §8 line 1220 currently claims
more than the heading assertion delivers.

---

### I2 — a truthful `documents.b_1099 = Yes` refuses, and the refusal's named remedy double-counts

**Where.** `crates/btctax-core/src/tax/return_refuse.rs:1132-1147` (the `Some(0)` arm),
`crates/btctax-core/src/tax/document_census.rs:251-255` (`entry_route(B1099)`),
`crates/btctax-core/src/tax/return_1040.rs:664-673` + `:1907-1918` (`form_1099b_gains`, added to the
ledger's `capital_net`).

**What is wrong.** After D1, `transcribed_rows(ri, B1099) = Some(ri.b_1099.len())`, so a declared
1099-B with zero rows refuses `DocumentDeclaredNotTranscribed`. But zero `[[b_1099]]` rows is the
**correct** return for two ordinary populations, and `Form1099B`'s own doc comment quotes the reason:

> *"However, if you choose to report all these transactions on Form 8949, leave this line blank and go
> to line 1b."*

- **The crypto filer** — btctax's own population. An exchange's Form 1099-B covers dispositions that
  are already in the ledger and print on Form 8949 / Schedule D lines 1b–3 and 8b–10. Their truthful
  census answer is Yes; their correct `[[b_1099]]` count is zero. The refusal tells them to *"enter it
  as a `[[b_1099]]` table through `btctax income import`"* — and `form_1099b_gains` is **added** to
  the ledger's `schedule_d(state, year)` inside `capital_net`, so following the instruction
  double-counts every gain.
- **The Box B/D/E filer** — basis not reported to the IRS, or an adjustment. Those transactions must
  go on Form 8949 with per-transaction detail, which btctax will not build for securities. Entering
  the row hits `Form1099BNeedsForm8949`; not entering it hits `DocumentDeclaredNotTranscribed`. The
  filer is refused either way, and the first refusal points at the second.

**Evidence — the plant and the observed output.** A probe test appended to `return_refuse.rs`'s test
module (reverted):

```
P1 1099-B, all on Form 8949  -> Some(DocumentDeclaredNotTranscribed { kind: B1099 })
P1 detail: you answered that you received one or more Form 1099-B, and none is transcribed on this
return. … Either enter it as a `[[b_1099]]` table through `btctax income import` — the 1099-B SCREEN
and its box census are task T5, but the rows themselves are read today (Schedule D lines 1a and 8a),
or change the answer to "no" if you received none.
P3 1099-B basis not reported -> Some(Form1099BNeedsForm8949)
P4 1099-B = No (false)       -> None
```

The build's own `the_four_1099_rows_take_the_same_three_rules_as_the_w2_row` assertion (1) does not
reach this: it uses `Form1099B::default()`, an all-zero row, which passes `screen_inputs` only because
`carries_totals` is false (`return_refuse.rs:1212-1216`). No committed test drives a *populated*
1099-B through the census.

**Minimal change.** The census question is *"did you receive one"*; the row count answers a different
question. Either (a) make `Some(true)` on `b_1099` require rows **only** when the filer has not
elected Form 8949 — i.e. gate the `Some(0)` arm on `broker_reporting` / the ledger being empty of
disposals — or (b) simplest and consistent with the existing `Form1099BNeedsForm8949` design: keep
`transcribed_rows(B1099) = None` (its pre-fold value) so the row declares without demanding a summary
the form itself makes optional. Whichever is chosen, the `entry_route(B1099)` sentence must stop
naming a route that double-counts a ledger return.

---

### I3 — `Form1099G` has no box 2, so a state-refund 1099-G cannot be transcribed truthfully

**Where.** `crates/btctax-core/src/tax/return_inputs.rs:143-156` (`Form1099G` = `box1_unemployment`,
`box4_fed_withheld` only), `document_census.rs:286-290` (the prompt), `:256-260` (`entry_route`).
Spec §5.1 line 1023 names the missing field by name: *"any `g_1099[].box2_state_refund > 0`"*.

**What is wrong.** The census prompt is *"Did you receive one or more Form 1099-G (a state that
refunded income tax, **or** paid unemployment compensation, should send you one — Schedule 1 line 1
and line 7 instructions)?"* — it asks about **both** boxes. `Form1099G` models only box 1. The taxable
refund reaches the return through a different, unmentioned field, the scalar
`sch1.state_refund_taxable`. So `g_1099` is **half scalar-shadowed** in exactly the way that made the
build mark `form_1098` and `form_1098e` non-live — and D1 promoted it to "the W-2's three rules"
anyway.

For the commonest 1099-G of all (an itemizer's prior-year state refund, box 2 only) the filer has
three moves and all three are wrong: answer `false` (false testimony beside a refund they did
receive), answer `true` and be refused, or answer `true` and enter an all-zero `[[g_1099]]` row — a
hollow transcription that satisfies the screen while the document's only amount sits in a field the
census never mentions. That last one is the census laundering the very thing it exists to catch.

**Evidence — the plant and the observed output.** Same probe:

```
P2 1099-G box 2 only         -> Some(DocumentDeclaredNotTranscribed { kind: G1099 })
P2b + one all-zero g_1099 row-> None            # files, with $900 of box-2 refund in sch1.state_refund_taxable
```

**Minimal change.** Either add `box2_state_refund` to `Form1099G` (the spec already names it and
`entry_route(G1099)` already promises the State and Local Income Tax Refund Worksheet to T5), or —
until T5 — treat `g_1099` the way `form_1098` is treated: `transcribed_rows → None`, so the row
declares without demanding a transcription surface that cannot hold the box. Do not leave the prompt
asking about a box the struct cannot carry.

---

### I4 — Schedule C line 6 is an income line the interview reaches, covered by nothing

**Where.** `crates/btctax-forms/forms/2024/f1040sc.map.toml` and `.../2025/f1040sc.map.toml`, the
`[census]` entry for `line = "6"`. Spec §8 line 1218: *"a line btctax cannot take is announced or
refused, never silent | the `covered_by` join over **every** `unmodeled` census entry (T3)"*.

**What is wrong.** Schedule C carries **88 `unmodeled` census entries in each year — 176 in total —
and not one of them carries a `covered_by`**; neither map has a `[[direction]]` table, and both sit
outside the join's thirteen maps. That is disclosed ("Open / not done"), but what the disclosure does
not say is that one of those entries is an **income line whose blank understates tax**:

> `'6' | Other income, including federal and state gasoline or fuel tax credit or refund — not
> collected (matching Schedule 3 line 12, Form 4136); line 7 (mapped) equals line 5.`

Every other consequential Schedule C blank runs the safe way and the map says so — line 2 (returns
and allowances), line 4 (COGS), line 30 (home office), line 41 (inventory) and the statutory-employee
box are all recorded as overstating. Line 6 is the one that runs the other way, and it is covered by
neither a question nor a refusal nor an advisory. The corresponding *credit* — Schedule 3 line 12,
Form 4136 — **is** in scope and correctly carries `Advisory::OtherCreditsOmitted`; the income side has
nothing. `grep -rn "4136\|fuel tax\|gasoline" crates/btctax-core/src` returns no hit, so the
attestation does not name it either and the REACH rule would red it if it were in scope.

The interview does reach Schedule C: `is_sstb`, `is_cooperative_patron`, `payments_requiring_1099`,
`will_file_required_1099` are `FORM_QUESTIONS` entries, `AmtDepreciationSameAsRegular`'s liveness is
`schedule_c.is_some()`, and `RefuseReason::ScheduleCLoss` fires on it.

**Evidence.** Mechanical sweep over every census-bearing map/extract pair
(`/tmp/…/scratchpad/sweep.py`), then `tomllib` over `f1040sc.map.toml`:

```
2024 unmodeled 88 without covered_by 88     line 6 cover: None
2025 unmodeled 88 without covered_by 88     line 6 cover: None
```

**Minimal change.** Add one limb to the residual attestation naming it in the form's words —
*"other income on a Schedule C, including a federal or state gasoline or fuel tax credit or refund
(Form 4136)"* — and a `covered_by = "QuestionId::OtherOutOfScopeIncome", names = "fuel tax"` on both
years' entries. Bringing all of Schedule C into the join is T5/T6-sized and is not asked for here;
covering the one understating line is a two-line edit. Whether the join's scope should widen belongs
in the follow-up list with an owning phase, since spec §8's guarantee says *every* entry while R2.2
scopes the instrument to seven forms.

---

### I5 — the new `UnmodeledReturnOptionsOmitted` advisory tells the filer something false, and then tells them to act on it

**Where.** `crates/btctax-core/src/tax/advisories.rs:518-529` (new in D2), covering
`f1040.map.toml:282-283` — `line = "NRA spouse box"` / `"NRA spouse name"`, the §6013(g)/(h) election.

**What is wrong.** The advisory's closing sentences are:

> *"…the §6013(g)/(h) election to treat a NONRESIDENT-ALIEN SPOUSE as a U.S. resident … **None of
> these changes your tax**; each is a choice the printed return leaves blank because btctax never
> asked. **If you want any of them, mark the form by hand before signing.**"*

For seven of the eight items listed (fiscal year aside) that is true. For the §6013(g)/(h) election it
is false in the largest possible way: the election is what makes MFJ available to a filer married to a
nonresident alien, and it makes the NRA spouse's **worldwide income** taxable on the return. btctax
asks nothing about an NRA spouse, so a filer in that position gets an MFJ return computed on the US
spouse's income alone, is told the box does not change their tax, and is instructed to check it by
hand. That is an invitation to sign an understated return, printed by the tool.

D2's own stated standard is the one this fails: *"Borrowing the credits advisory for them would have
been a false statement to the filer."* The advisory names the cell honestly; the reassurance beside it
does not.

**Evidence.** Read directly at `advisories.rs:518-529` against `f1040.map.toml:282`. No plant needed —
the two texts are in the tree and contradict each other on the same election. Nothing in
`btctax-core` mentions a nonresident-alien **spouse** at all (`dual_status_alien` is about the
*filer*), so no refusal stands between that filer and the advice.

**Minimal change.** Split the sentence: keep *"none of these changes your tax"* for the seven
administrative cells, and give the §6013(g)/(h) election its own clause — that it is a substantive
election which subjects the spouse's worldwide income to US tax, that btctax has not asked about it
and has not computed it, and that it is a preparer's return rather than a box to hand-check. If a
refusal is wanted instead, that is a follow-up with an owning task; the false clause should not wait
for it.

---

### M1 — `document_census.rs`'s doc comments still describe the pre-fold rule they sit above

**Where.** `crates/btctax-core/src/tax/document_census.rs:468-482` (`transcribed_rows`), `:129-137`
(`exit_sentence`), and the module header at `:22-26`.

`transcribed_rows`'s doc comment says of `int_1099` / `div_1099` / `b_1099` / `g_1099`: *"They
therefore count as `None` here and refuse outright on `Some(true)`"* — and eight lines below, the code
returns `Some(ri.int_1099.len())` under a `★★★ COUNTABLE, and the earlier reading that they were not
was simply WRONG` comment. `exit_sentence`'s says *"The four rows whose SCREEN is task T5 carry a
sentence of the same shape as §2.2's … a filer who has one is refused"*; the code returns `None` for
all four. The module header repeats it a third time. In a file whose entire subject is provenance, the
comment a reader reaches first states the opposite of the behaviour. Delete the three stale passages
(the corrected reasoning is already written inline).

### M2 — `form_1098` / `form_1098e` are unconditionally non-live, which §5.1 does not authorise, and it is not in the deviation list

`row_is_live` returns false for both rows always. §5.1 line 1015 reads *"Every row is live except
`form_1098`, which is live **iff `schedule_a.is_some()`**"* — conditionally live, not dead — and says
nothing at all about `form_1098e`, which it lists under "supported". The deferral is **substantively
right**: §5.5 (spec lines 586-618) shows the liveness arrives with T9's `form_1098: Vec<Form1098>`
section and the removal of `ScheduleAInputs.mortgage_interest_1098`, and making the row live now would
reproduce I2/I3 for every itemizer. It is disclosed in the code, in the report's row table, and in
`EXEMPT_LEAVES` with the owning tasks named. What is missing is a **D-number**: fifteen deviations
were listed for the controller to rule on and this one was not, so nobody ruled. Add it, with T9/T5 as
the owning phases. Note the residue while it is open: `schedule_a.mortgage_interest_1098` is a bare
`Usd`, so `$0` there remains indistinguishable from "never asked" — the D-8 trap R3 exists to close,
left open on Schedule A line 8a.

### M3 — a "dead sum" line's cover names one addend's refusal, and nothing checks the conjunction

Schedule 2 line 1z (`covered_by = RefuseReason::DocumentTypeUnsupported`), line 7
(`RefuseReason::AllocatedTips`) and line 18 (`RefuseReason::HsaActivityUnsupported`) are sums whose
blankness is correct **only if every addend is blank**, and each cover names a mechanism that keeps
just one of them blank — line 7's `AllocatedTips` says nothing about line 6's Form 8919 case, which
has its own separate cover. The entry `reason` strings state the conjunction honestly (*"dead with
both blank"*), so this is disclosed rather than hidden, and the direction rule existence-checks a
`RefuseReason` by design. But if line 6's cover were later weakened, line 7 would stay green on a
refusal that never touches it. The join has no way to express "covered by the conjunction of these
lines' covers"; recording that limitation at the site (as D9c does for the dead block) would at least
make it reviewable.

### M4 — the D11 flip reads one printed sentence form and misses the PAREN convention the maps already mark

`subtracted_line()` fires only on *"Subtract line X from line Y"*. Schedule 1 marks three entries
`★ PAREN` in their own `reason` strings — line 8a (net operating loss), 8d (Form 2555 foreign earned
income exclusion), 8s (nontaxable Medicaid waiver payments) — all of which the form prints in
parentheses and subtracts inside the line-9 total. They are graded `Understates` with the block and
covered by `OtherOutOfScopeIncome`, whose YES refuses. That is the same shape D11 was folded to fix on
Schedule B line 3, and it is **conservative** here (stricter than the line deserves, never laxer), so
it does not gate. But the map already knows which entries these are, and the flip cannot see it. Worth
a follow-up with an owning phase rather than a change now.

### M5 — the document census is asked LAST, after a 4,425-character attestation that duplicates five of its rows

`FORM_QUESTIONS` appends the eighteen census rows at indices 17..=34 (forced by `decl_tristate!`'s
index coupling, recorded in the build), and `live_questions` preserves registry order. So a Single
TY2024 filer answers eight gate declarations — including the 4,425-char residual attestation at
position 13 — and only then reaches *"Did you receive one or more Form W-2?"*. A **document-first**
interview asks the shoebox first. Worse, the attestation's limb (a) names *"a PENSION, ANNUITY or IRA
DISTRIBUTION (Form 1099-R), SOCIAL SECURITY … (Form SSA-1099 or RRB-1099) … any Schedule K-1 …
gambling winnings"*, and rows `DocR1099`, `DocSsa1099`, `DocK1`, `DocScheduleERental`, `DocW2g` then
ask the same five facts one at a time. Correctness is safe — `screen_document_census`
(`return_refuse.rs:1283`) runs **before** the `OtherOutOfScopeIncome` refusal (`:1453`), so a filer who
answers both truthfully gets the row's specific §2.2 exit rather than the generic one, and I verified
that ordering. This is a journey finding, not a correctness one: the ordering is an artefact of a
macro's array index reaching the filer's screen. Sorting `live_questions`' output (leaving the
registry array alone) would fix it without touching `decl_tristate!`.

### N1 — `census_tristate!`'s `clear` writes through on a non-live row

`registries.rs:342-345`: `clear` calls `ri.documents.set($row, None)` with no liveness guard, while
`get` and `set` both return early for a non-live row. Harmless today (the leaf is already `None` for
the two non-live rows), but it is the one arm of the three that does not check.

---

## Seams checked clean

**Seam 1 — the census chain.** Walked `int_1099` (supported) and `k1` (unsupported) end to end:
`FormQuestion` entry → `row_is_live` → `screen_document_census` → `census_tristate!`'s `apply` guard →
`live_questions` → `interview_state` → the TOML wire → `income import`. All eighteen `get`/`set` pairs
match their `DocumentRow` mechanically. Four plants, all red:

| plant | observed |
|---|---|
| `DocInt1099`'s `get` reads `div_1099` while `set` writes `int_1099` | 5 red, incl. `coverage::every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` — *"DocInt1099 (DocumentCensus): get after set must read back the written value"* |
| `DocOid1099` ⇄ `DocW2g` leaves swapped (each internally consistent) | 1 red, same test — the subtler shape is caught too |
| `row_is_live(K1) = false` (a liveness predicate never true) | 5 red, incl. `only_the_two_scalar_shadowed_rows_are_not_live`, `an_unsupported_census_yes_is_refusing_with_its_exit_before_commit` |
| `k1` dropped from the classifier (`k1: _` + declaration removed) | `classifier::every_registry_question_is_declared_exactly_once` |
| `census_tristate!(20, DocB1099, DocumentRow::G1099)` — field guards the wrong row's count | `apply::a_census_no_over_transcribed_rows_is_refused_and_stores_nothing` |

One plant is green **and correctly so**: swapping the `QuestionId`s of two leaves inside
`classify_document_census` changes nothing, because `Census::declaration` ignores its `_leaf`
(`classifier.rs:77-79`) — the classifier's contract is exhaustive destructuring plus
declared-exactly-once, and the leaf↔question pairing is enforced by `coverage.rs` instead. Correct
division of labour, not a hole.

TOML wire: `documents` is `#[serde(default)]` and `ReturnInputs` has no `deny_unknown_fields`, so a v3
row with no `[documents]` block loads all-`None` and **blocks** rather than answering for the filer.
`income import` re-attaches the stored `answer_log` but takes the census **values** from the file, so
a re-import that omits the block re-blanks the census — which fails closed at the registry loop's
`(q.get)(ri).is_none()` check. Recoverable, and the same property the seventeen pre-existing
declarations already had.

**Seam 2 — truthful answers never refuse.** Both committed fixtures carrying `documents` values were
enumerated and checked row-by-row against the rows they hold, and both are now truthful:
`nine_dependents_amt_inputs.toml` swears `int_1099 = true`, `b_1099 = true` beside a `[[int_1099]]`
row and a `[[b_1099]]` row (`basis_reported_and_no_adjustments = true`, so `Form1099BNeedsForm8949`
does not fire); `fullreturn_inputs.toml` swears `w2/int_1099/div_1099/g_1099 = true` beside two
`[[w2s]]`, one `[[int_1099]]`, one `[[div_1099]]` and one `[[g_1099]]`. Neither carries a false
census answer, and no comment excusing one survives. These are the only two fixture TOMLs in the
tree. The golden corpus files: the restored-tree run is 2535 passed with only the pre-declared
environment failures. The `1099-INT` and `1099-DIV` rows are fully modelled (boxes 1–9 and 1a–13),
so I2/I3 are specific to `b_1099` and `g_1099` and do not generalise to the other two.

**Seam 3 — the direction tables.** Six plants on the real maps, five red:

| plant | observed |
|---|---|
| `Advisory::EicOmitted` on Schedule 1 line 2a (Part I) | *"an ADVISORY covers an `Understates` line … A blank there is a FALSE STATEMENT"* — and the same advisory already stands green on Part II line 11 in the committed tree, so K10 discriminates rather than merely refusing |
| Part II direction block deleted | **26** findings, every Part II entry *"no [[direction]] block places it"*; zero silently regraded |
| caption `Income` → `Incomes` | *"the extract does not carry … it reads `Part II Adjustments to Income`"* |
| line 2a given `part = "…1099-K"` (laundering a numbered line) | *"a numbered line is placed by its number alone"* |
| Part II range widened to `1`–`26`, overlapping Part I | *"both claim a line — a line cannot move in two directions"* |
| D11's `[[subtracts]]` removed / replaced with a sentence the extract lacks | both red, and the second reds **twice** — the fake sentence and the cover it was propping up fall together |

(My first attempt at the fake-sentence plant edited a comment on `f1040sb.map.toml:56` rather than the
block at `:196` and appeared green; re-run correctly it reproduces K-D11b exactly. My error, not the
build's.)

The D11 sweep was re-run independently over every census-bearing map/extract pair. **Nine** *"Subtract
line X from line Y"* sentences exist across the eleven in-scope extracts, exactly as the report says,
and each X's mapped/unmodeled verdict reproduces — including the Schedule A line 4 text, which really
does read *"Subtract line 3 from **line 1**"* (`f1040sa--2024.txt:19`). The two Schedule B flips are
the only ones that land on an unmodeled line. The report's table is accurate.

The loosest covers, judged: **f1040 line 36** is honest — `Advisory::UnmodeledReturnOptionsOmitted`
names *"applying an overpayment to NEXT YEAR'S ESTIMATED TAX (line 36)"* in those words, and a blank
there is tax-neutral. The `NoDollar` blocks are correctly graded: all seventeen entries placed by
`part` are identity, address, designee or signature cells carrying no dollar. The **dead sums** are
M3. The one `NoDollar` cover I would not call honest is the NRA-spouse pair, which is I5 — not because
the cell carries a dollar, but because the sentence covering it is false.

**Seam 4 — REACH.** All 122 `QuestionId::OtherOutOfScopeIncome` covers resolve; the twelve on
Schedule 1 Part I and the twelve on Form 1040 were read against the extracted prompt one by one and
each named phrase is present in the words a filer reads, not as a bare token: *"Alimony received"*,
*"Jury duty pay"*, *"Prizes and awards"*, *"Alaska Permanent Fund dividends"*, *"Wages earned while
incarcerated"*, *"Household employee wages not reported on a Form W-2"*, *"Tip income not reported on
line 1a"*, and the form numbers each inside a phrase that says what the form is (*"other gains or
losses from a sale of business property (Form 4797)"*, *"the foreign earned income exclusion (Form
2555)"*, *"the tax on a lump-sum distribution (Form 4972)"*). Plant — delete `Olympic and Paralympic
medals and USOC prize money; ` from the prompt:

```
2024/f1040s1 form1[0].Page1[0].f1_24[0] (line "8m"): OtherOutOfScopeIncome's prompt never says
"Olympic and Paralympic medals". A variant that exists is not a cover …
```

**Is it answerable as a filer?** At 4,425 characters it is one `y/n` over five limbs, and a `Yes`
refuses the return, so the failure mode of a filer skimming it is fail-**open**: a missed limb yields
a `No` and an understated return. That is inherent to a compound attestation and is the spec's own
design, not a build defect — but it is why I4 matters, since anything the prompt does not name is
covered by nothing at all. The one structural improvement available without redesigning it is M5's
ordering: five of its limbs are asked again, one at a time, twelve questions later.

**Seam 5 — `interview_state()` vs the registries.** All seven states are exercised by committed tests
and each named kill reproduces. `refusing` is derived from the question's own refusal
(`refusal_of_answer`) and, for the two states a question cannot express, from `census_row_invariant` —
and the two cannot double-list, because `census_row_invariant` early-returns on
`transcribed_rows(...)?` (`None` for every §2.2 row) and the declared-not-transcribed arm additionally
requires `exit_sentence().is_none()`. `Declined` reaches `forgoing` marked and never `blocking`. D6's
planted `PARAMS_GATED_PROMPTS` occupant moves between `waiting` and `blocking`. Plant — omit
`QuestionId::ForeignTrust` from the `FORM_QUESTIONS` walk:

```
FAIL btctax-core  interview_state::tests::every_registry_entry_lands_in_exactly_one_bucket
FAIL btctax-core  interview_state::tests::a_hash_mismatched_record_reappears_with_the_changed_wording_reason
FAIL btctax-cli   answer::tests::answering_every_blocking_item_empties_the_panel_and_refusing_is_empty_before_the_screen_passes
```

The cross-crate no-brick test reds too, which is the property that matters. `is_committable()` can be
`true` on a return `screen_inputs` refuses (a populated 1099-B, a negative amount, the Schedule 1-A
gates) — checked deliberately, and **not a defect**: `grep` shows `is_committable` has no production
caller, and `interview_state` is used only to print the panel (`answer.rs:246,414`). No false gate.

**Seam 6 — what the build left out.** D7 (`DEPENDENT_GATES × rows`) is a `debug_assert!` on
`DependentGate::ALL.len() == 21` with T7 named at the site — an honest placeholder, not a walk, and
the report says so. D15's 24 asks are 8 live gates + 16 live census rows for a Single TY2024 filer,
each mandated by R3/§5.1; the ordering is M5. The `unmodeled` entries the interview reaches and the
join does not cover are Schedule C's 176 (I4) plus the 2025 Form 6251 PAREN entries at lines 2f/2s,
both `covered_by = None` and both outside the join.

**R15.** `stop-list` is clean on the tree; the planted `pub questions_remaining: usize` on
`InterviewState` reds with the file and line named. D14's own false-PASS fix (scanning production
source *after* a `#[cfg(test)]` module) is committed as a kill.

**The fold's arithmetic.** The report's self-correction is right and I did not re-measure the settled
counts: 28 `[[direction]]` headers across 11 tables, `Advisory 129 / QuestionId 122 / RefuseReason 47`
summing to 298, and the join's own line reproduces exactly on the restored tree.

---

Counts: C=0 I=5 M=5 N=1
