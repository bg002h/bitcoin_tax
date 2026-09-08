# Fold — the T9 seam review (1C / 1I / 2M / 1N), main working tree

Brief: `design/agent-reports/BRIEF-fold-interview-T9-review.md`.
Review: `design/agent-reports/2026-09-07-build-interview-T9-review.md` (persisted at `7ef1c36b`).
Ledger: `design/agent-reports/2026-09-07-build-interview-T9-review-VERIFICATION.md` (`39b73ca7`).

Folded on branch `main` from `39b73ca7`. **Nothing committed, nothing pushed.** Every plant was
restored from a `cp` backup under the session scratchpad; `git status --short` lists only the eleven
files below. No subagents.

★ Every panic below is pasted **verbatim as observed**, so a few of the `file:line` prefixes are the
line numbers at the moment that plant ran and have since shifted by the later edits of this same fold
(the M-2 comments added ~11 lines above them). The assertion messages are unique and greppable.

```
 M crates/btctax-cli/src/cmd/admin.rs
 M crates/btctax-cli/tests/export_irs_pdf.rs
 M crates/btctax-cli/tests/open_next_year_t4b.rs
 M crates/btctax-core/src/tax/questions.rs
 M crates/btctax-core/src/tax/return_inputs.rs
 M crates/btctax-core/src/tax/return_refuse.rs
 M crates/btctax-core/src/tax/transcription_warnings.rs
 M crates/btctax-forms/src/lib.rs
 M crates/btctax-forms/src/schedule_a.rs
 M crates/btctax-forms/tests/full_return_forms.rs
 M design/SPEC_interview.md
11 files changed, 777 insertions(+), 119 deletions(-)
```

---

## Commands, with their real output

```
$ cargo fmt --all                                              # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 3.61s     # clean, exit 0

$ bash scripts/pii-scan-generic.sh
pii-scan: clean (HEAD).

$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
f1040s3:5 f1040sa:21 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12 f8959:17
f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)

$ cargo run -q -p xtask -- census-join
census join: 283 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -q -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 91 registry prompts scanned;
no forbidden shape

$ cargo run -q -p xtask -- prompt-check
xtask prompt-check: OK — 88 assertions, all verbatim

$ cargo run -q -p xtask -- box-census
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)

$ make check
     Summary [  20.671s] 3475 tests run: 3475 passed, 12 skipped
```

**All five instruments are byte-identical to their HEAD values** (375 / 18 / 31 / 0 / 17; 283 across
13; 8 + 4, 91; 88; 268 / 19 / 9). Nothing moved, and nothing should have: no form, map, prompt or
census entry changed in this fold — the C-1 fix is liveness, the I-1 fix is a manifest string, and
the rest is tests and doc comments.

**Suite: 3470 → 3475 passed, 12 skipped — +5 tests.**

| new test | what it holds |
|---|---|
| `the_ceiling_warning_is_silent_for_a_standard_deduction_filer` | M-1 |
| `the_ceiling_warning_formats_both_figures_as_money` | N-1 |
| `the_opener_seeds_a_1098_onto_a_standard_deduction_year_and_nothing_refuses` | C-1, the reachable path |
| `line_8b_overflows_to_see_attached_on_both_years_and_names_every_recipient` | I-1, the emitter |
| `the_manifest_names_the_line_8b_statement_when_the_identities_do_not_fit` | I-1, the manifest |

Two existing tests were rewritten rather than added to:
`a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing` (C-1's kill, which
was a shadow) and `the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523` (M-2).

---

## C-1 (Critical) — a standard-deduction filer is asked and refused over a mortgage they do not deduct

### What changed

**One reader carries the itemize election, and every line-8 rule goes through it.** The brief's ★
asked for a shape in which the conjunct cannot be forgotten by the next rule rather than three sites
that each remember it, so the fix is an accessor, not three conjuncts:

```rust
// crates/btctax-core/src/tax/return_inputs.rs
pub fn form_1098_deducted(&self) -> &[Form1098] {
    if self.schedule_a.is_some() { &self.form_1098 } else { &[] }
}
```

Its doc comment states the rule, names J-24, and records that box 4 is deliberately **not** read
through it. Callers now:

| site | before | after |
|---|---|---|
| `questions.rs::mortgage_question_live` | `schedule_a.is_some() && !form_1098.is_empty()` | `!ri.form_1098_deducted().is_empty()` |
| `questions.rs::mortgage_interest_credit_question_live` | `!ri.form_1098.is_empty() \|\| …8b` | `!ri.form_1098_deducted().is_empty() \|\| …8b` |
| `return_refuse.rs` shared-interest arms | inside the box-4 loop over `ri.form_1098` | **its own loop** over `ri.form_1098_deducted()` |
| `transcription_warnings.rs` ceiling warning | `if ri.form_1098.is_empty() { return None }` | `ri.form_1098_deducted()` (M-1) |
| `return_inputs.rs::form_1098_outstanding_principal` | Σ box 2 over `self.form_1098` | Σ box 2 over `self.form_1098_deducted()` |

**The false premise is deleted.** `questions.rs` used to say *"the 1098 SECTION is already gated on
the itemize election, so a row can only exist on an itemizing return"*. It now says the opposite in
terms — that `SectionId::Form1098s` has no arm in `section_is_live` and falls through `_ => true`,
that this is what R8 intends, and that `open_next_year` reaches the shape with no filer action. The
same false half was in `form_1098_interest_and_points`'s doc comment (*"the itemize election gates
the SECTION'S LIVENESS"*) and is replaced there too.

**Box 4 stays ungated, and the code says why.** The two refusals now iterate different row sets, with
a comment above them that names the reason and tells a later reader not to tidy them back into one
loop:

> ★★★ THE TWO ITERATE DIFFERENT ROW SETS, and that is the T9 seam review's C-1. Box 4 is INCOME on
> Schedule 1 line 8z — owed whether or not the filer itemizes — so it reads every transcribed row.
> The shared-interest gate is about SCHEDULE A LINE 8a summing box 1 in full, which a
> standard-deduction return does not have […]

### Where

`crates/btctax-core/src/tax/return_inputs.rs` (the accessor, `form_1098_outstanding_principal`,
`form_1098_interest_and_points`'s comment); `crates/btctax-core/src/tax/questions.rs:652`, `:2519`;
`crates/btctax-core/src/tax/return_refuse.rs:3196-3272`;
`crates/btctax-core/src/tax/transcription_warnings.rs:341`.

### The kill and its observed red

The shipped `a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing` was a
shadow twice over — it enumerated only the three older declarations, and its fixture pre-answered
both new gates. It now:

1. blanks both gates (`other_borrower_paid_interest: None`, `claiming_mortgage_interest_credit =
   None`) — the shape the opener seeds and a filer who has just transcribed the paper is in;
2. asserts `!question_is_live` for **four** questions including `ClaimingMortgageInterestCredit`;
3. crosses `reason(&standard) == None` over shared-interest ∈ {`None`, `Some(true)`} ×
   8396 ∈ {`None`, `Some(true)`} — all four truthful states;
4. **the differential**: on a return with no `schedule_a`, adding the Form 1098 rows must change
   neither the live-question set nor `reason()`;
5. the other half — the identical paper on an itemizing return refuses on each blank in turn
   (`MixedUseMortgageUnanswered` → `MortgageInterestCreditUnanswered` →
   `SharedMortgageInterestUnanswered` → files) — so deleting the rules outright cannot pass.

**Plant 1 — the 8396 gate loses the election** (`mortgage_interest_credit_question_live` reads
`ri.form_1098`):

```
panicked at crates/btctax-core/src/tax/return_refuse.rs:3906:13:
ClaimingMortgageInterestCredit must NOT be live on a standard-deduction return
```

**Plant 1a — the same plant, with the hand-list narrowed back to the three older declarations** (i.e.
someone adds a rule and forgets the list): the 2×2 cross catches it —

```
panicked at crates/btctax-core/src/tax/return_refuse.rs:3930:17:
assertion `left == right` failed: a standard-deduction filer is never refused over line 8a (shared=None, 8396=None)
  left: Some(MortgageInterestCreditUnanswered)
 right: None
```

**Plant 1b — the same plant, with the hand-list AND the cross both neutralised**, so only the
differential is left standing. It reds, and names the exact question that leaked in:

```
panicked at crates/btctax-core/src/tax/return_refuse.rs:3942:9:
assertion `left == right` failed: ★ THE KILL: on a return with NO Schedule A, transcribing a Form 1098 must not make one single question live
  left: [… DocSa5498, ClaimingMortgageInterestCredit, SoldMainHome]
 right: [… DocSa5498, SoldMainHome]
```

**Plant 2 — the shared-interest loop reads every transcribed row again:**

```
panicked at crates/btctax-core/src/tax/return_refuse.rs:3931:17:
assertion `left == right` failed: a standard-deduction filer is never refused over line 8a (shared=None, 8396=None)
  left: Some(SharedMortgageInterestUnanswered)
 right: None
```

**Plant 3 — `mortgage_question_live` loses the election:**

```
panicked at crates/btctax-core/src/tax/return_refuse.rs:3906:13:
MortgageAllUsedToBuyBuildImprove must NOT be live on a standard-deduction return
```

**Plant 4 — the ONE reader loses it** (`form_1098_deducted` returns `&self.form_1098` always):

```
     Summary [   0.512s] 592/1348 tests run: 591 passed, 1 failed, 0 skipped
        FAIL btctax-core tax::return_refuse::tests::a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing
```

**The second kill — the reachable path.**
`the_opener_seeds_a_1098_onto_a_standard_deduction_year_and_nothing_refuses`
(`crates/btctax-cli/tests/open_next_year_t4b.rs`) drives the **real** `open_next_year` from a year
that itemized, reads the draft it wrote, measures the premises off the shipped opener
(`schedule_a.is_none()`, one seeded row, its gate blank), and asserts nothing is live and
`screen_param_free` is `None`. Observed red under plant 2:

```
panicked at crates/btctax-cli/tests/open_next_year_t4b.rs:1692:5:
assertion `left == right` failed: ★ THE KILL: the seeded row refuses nothing — a filer who has done
NOTHING but open the year cannot be blocked over a Schedule A line 8a their return does not have
  left: Some(SharedMortgageInterestUnanswered)
 right: None
```

### Anything decided differently and why

The brief prescribed `mortgage_interest_credit_question_live` →
`ri.schedule_a.as_ref().is_some_and(|a| !ri.form_1098.is_empty() || !a.mortgage_interest_not_on_1098.is_empty())`.
I implemented the **equivalent** predicate through `form_1098_deducted()` instead, because the brief's
own ★ asks for a shape the next rule cannot forget and the literal form re-types the conjunct at the
site. Same truth table; one reader. Recorded under **Deviations**.

---

## The spec change C-1 required

`design/SPEC_interview.md` R8 said **both** that the Form 1098 is top-level (*"it arrives whether or
not the filer itemizes"*) and that *"the `form_1098` census row **and the 1098 section** are live iff
`schedule_a.is_some()`"*. Per the brief I kept the **first** reading — the section stays top-level —
and rewrote the liveness paragraph so R8 says one thing:

- the **section** is offered to every filer (the document arrives regardless, and `open_next_year`
  seeds a lender onto a year that has elected nothing);
- the **census row** carries `schedule_a.is_some()`;
- **every rule that reads a row for a Schedule A line 8 purpose** carries it too — the three
  declarations, the 8396 gate, the shared-interest refusal and the ceiling warning — all through the
  single accessor `form_1098_deducted()`;
- **box 4 is the one deliberate exception**, named as such;
- and the paragraph cites J-24 as what this is for.

The edit is marked in the document (★ *corrected in the T9 fold*) with what it previously said, so a
later reader can see the contradiction that was resolved rather than only its resolution.

**Two further R8 corrections in the same pass** (see Deviations):

- **The 8b/8c cell pairing (review D-1).** R8 paired `f1_17`/`f1_19` for 8b and `f1_18`/`f1_20` for
  8c. The controller's ledger records D-1 as CONFIRMED with *"SPEC R8's cell pairing must be
  corrected"*, and the fold brief did not restate it. R8 now says: TY2024 8b = `f1_19` (amount) with
  `f1_17` and `f1_18` as its two dotted description lines; **8c = `f1_20`, which has no description
  cell on either form**; TY2025 merges the two dotted rows into one 24pt box
  `Line8b_ReadOrder[0].f1_16`, so **two** recipients overflow — and an overflow makes the statement a
  packet-manifest hand mark (I-1).
- **The T9 roadmap row (`§7`)** said the section is *"live iff `schedule_a.is_some()`"*, the same
  contradiction one table down. It now says the section is top-level with the census row and every
  line-8 consequence keyed on the election, and its kill column names the new kills.

`§5.1`'s *"Row liveness"* paragraph (line 1033) was already correct — it scopes the election to the
census row only — and is unchanged.

---

## I-1 (Important) — line 8b prints *"See attached"* for a statement nothing produces, asks for, or names

### What changed

**One condition, read by both surfaces.** The emitter's inline `fits` comparison became a named
function, and the manifest reads the same function through a thin public wrapper — so the packet
cannot print the assertion while the manifest stays silent:

```rust
// crates/btctax-forms/src/schedule_a.rs
pub(crate) fn line8b_overflow<'a>(lines: &'a ScheduleALines, map: &ScheduleAMap) -> &'a [String]
// crates/btctax-forms/src/lib.rs
pub fn schedule_a_line8b_overflow(lines: &ScheduleALines, year: i32) -> Result<Vec<String>, FormsError>
```

It returns **every** recipient, not the tail past the last dotted line, because the escape replaces
the whole block with one string — so on an overflow no identity survives on the page and all of them
belong on the statement.

`hand_marks` gained a parameter (`tax_year`, `line8b_overflow: &[String]`) and an entry conditioned
on the overflow having actually occurred, in the same style as the other marks. It quotes what the
page asserts (*"See attached"*), quotes what the instruction asks for (*"attaching a statement to
your paper return"*), names the recipients whose identities left the page, and prices the omission
the way the instruction does. The call site computes the overflow with `?`, so a year whose Schedule
A map is missing is an error rather than a silent "nothing overflowed".

### Where

`crates/btctax-forms/src/schedule_a.rs:57-79` and `:136-141`;
`crates/btctax-forms/src/lib.rs` (`schedule_a_line8b_overflow`);
`crates/btctax-cli/src/cmd/admin.rs` (`hand_marks` + its call site).

### The kill and its observed red

Two kills, because the finding has two halves (the page and the manifest).

**`line_8b_overflows_to_see_attached_on_both_years_and_names_every_recipient`**
(`crates/btctax-forms/tests/full_return_forms.rs`) drives the real `fill_schedule_a` on **both**
years and reads the cells back: TY2024 two recipients fit and both print, three overflow and
`f1_17 == "See attached"` with `f1_18` blank; TY2025 one fits, **two** overflow into the merged box.
Per B1 this is the branch's **first observed behaviour** — nothing exercised `fits == false` before.
Plant (`line8b_overflow` always reports that everything fits):

```
panicked at crates/btctax-forms/tests/full_return_forms.rs:3089:5:
assertion `left == right` failed: ★ every recipient is on the statement, not just the third — the escape takes the whole block
  left: []
 right: ["SELLER 0, 000-00-0000, 0 MAIN ST", "SELLER 1, 000-00-0001, 1 MAIN ST", "SELLER 2, 000-00-0002, 2 MAIN ST"]
```

**`the_manifest_names_the_line_8b_statement_when_the_identities_do_not_fit`**
(`crates/btctax-cli/tests/export_irs_pdf.rs`) exports a real TY2024 packet from a vault with three
seller-financed 8b recipients, asserts the mark exists, quotes both halves of the instruction and
names all three recipients, and asserts the manifest file itself carries it. The **other half** — two
recipients, which fit — asserts no such mark and no `line 8b` in the manifest, because a list that
always says the same thing signals nothing. Plant (the `hand_marks` entry cannot fire):

```
panicked at crates/btctax-cli/tests/export_irs_pdf.rs:2010:13:
★ THE KILL: the page printed "See attached" and the manifest must say so: ["Form 1040 line 7 — …",
"Form 1040 page 2 — the signature block: …"]
```

### Anything decided differently and why

**No advisory and no transcription warning fires, and the code says so.** A transcription warning is
about a transcribed *row* being suspect and none is here — the recipients are entered correctly and
the *form* is too small. An advisory lands on stderr, which is precisely the surface N4's own
rationale rejects and which the Form 8283 third-party-signature mark was moved **off** for this exact
reason. The statement goes in the envelope, and the manifest is the artifact the filer follows while
assembling the envelope. Stated in a `★★` comment beside the mark.

Synthetic identifiers only: recipient SSNs are `000-00-0000` … `000-00-0002` (never-issued area 000,
group 00). `scripts/pii-scan-generic.sh` is clean.

---

## M-1 (Minor) — the ceiling warning fires for a standard-deduction filer

**What changed.** `acquisition_debt_ceiling_warning` reads `ri.form_1098_deducted()` (the same reader
as C-1), so it is silent without a Schedule A. Its doc comment states the reason: the warning's own
closing sentence points at *"were you inside EVERY home-mortgage debt limit this year?"*, and
`MortgageWithinDebtLimit` is not live for that filer. The shared `warned(…)` test helper — which is
the single fixture behind all three `the_acquisition_debt_warning_*` tests, and the review's
predicted set of reds — now states the itemize election explicitly, with a comment saying which test
measures the gate instead.

**Where.** `crates/btctax-core/src/tax/transcription_warnings.rs:341-355`, `:409-421`.

**The kill.** `the_ceiling_warning_is_silent_for_a_standard_deduction_filer` — both halves on one
fixture (silent without a Schedule A; warns with one, same paper, same loan) plus the premise that
the question it points at is not live. Plant (read `ri.form_1098` again):

```
panicked at crates/btctax-core/src/tax/transcription_warnings.rs:617:9:
assertion `left == right` failed: ★ THE KILL: a limit on a deduction they are not claiming warns them about nothing
  left: Some("the outstanding mortgage principal in box 2 of the 1 Form 1098 on this return adds up
  to $900000.00, which is more than the $750000.00 the §163(h)(3)(B) limit allows …")
 right: None
```

---

## M-2 (Minor) — the home-sale table crossed 10 of 24 combinations

**What changed — I lifted the 1099-S state into the loop** (the brief's first option), so the
delivered coverage is the described coverage rather than the description being narrowed. The loop is
now `s_1099 ∈ {Some(false), Some(true), None}` × the eight test-triples = **24 rows**, asserted
`rows == 24` so a future edit cannot quietly drop the dimension, with one blank. Each 1099-S state
has its own expected exit, asserted separately:

| `s_1099` | outcome asserted |
|---|---|
| `Some(false)` | the three tests decide — blank iff all three met, else `HomeSaleNotComputed` naming *PUB. 523* and *code H* |
| `Some(true)` | `DocumentTypeUnsupported { S1099 }` on both tiers, its detail naming *Pub. 523* and *code H* |
| `None` | `DocumentCensusUnanswered { S1099 }` at commit **and** `HomeSaleNotComputed` at import |

**The build report is therefore NOT edited** — its sentence describing a full cross is now true, and
a build report is a historical record rather than a live artifact.

**The kill.** Plant (the `can_exclude_all_gain == Some(false)` arm made unreachable):

```
panicked at crates/btctax-core/src/tax/return_refuse.rs:4190:33:
(true,true,false,s_1099=Some(false)) must not be the blank branch
```

**★ Found while folding, and disclosed rather than papered over.** Widening the cross made me plant
the home-sale rule's **own** `s_1099 == Some(true)` arm — and **nothing red**. The census screen runs
inside `screen_inputs_tiered` ahead of that rule on *both* tiers (`screen_param_free` is the same
body), so `Some(true)` never reaches the arm: it is unreachable code, and the build report's D-4
claim that *"the `s_1099` conjunct is still in the home-sale rule so it is complete on its own terms
at import"* is not observable. I **kept** the arm — it is the fail-closed backstop if a value rule
were ever ordered ahead of the census — and recorded the measurement in two places so it cannot be
mistaken for a tested guard: a `★★` comment beside the arm in `screen_inputs_tiered`, and a
*"what the cross does NOT hold"* paragraph in the test's own doc comment. The observable guarantee —
that a `Some(true)` row refuses with the census's rule, naming the same exit — is what the eight
crossed rows assert. Deleting the arm was the alternative; I did not, because it is a behaviour-
neutral removal of defence in depth in a fold, which is not the moment for it.

---

## N-1 (Nit) — the ceiling warning's raw `${total}` / `${limit}`

Both figures now go through the module's own `money()` (`transcription_warnings.rs:312`), like every
other warning in the file. The filer reads `$900000.00` rather than `$900000`.

**No thousands separators**, per the brief's conditional (*"thousands separators if `money()`
provides them"*): `money()` is `format!("${v:.2}")` and provides none, and changing it would move the
text of every other warning in the module — out of scope for a nit.

**The kill.** `the_ceiling_warning_formats_both_figures_as_money` asserts `$900000.00` **and**
`$750000.00` are both present. The three pre-existing assertions that match on the bare digit strings
(`w.contains("900000")`, `"750000"`, `"1000000"`) are unaffected, since they are substrings of the
formatted values.

---

## Deviations

1. **C-1's fix is an accessor, not the literal predicate the brief printed.**
   `mortgage_interest_credit_question_live` reads `!ri.form_1098_deducted().is_empty() || …8b`
   instead of `ri.schedule_a.as_ref().is_some_and(|a| !ri.form_1098.is_empty() || …)`. The two have
   the same truth table (`form_1098_deducted()` is `form_1098` when `schedule_a.is_some()` and empty
   otherwise, and the 8b half already lives on `ScheduleAInputs`). Chosen because the brief's own ★
   asks for a shape the next rule cannot forget, and the literal form re-types the conjunct at the
   site — which is exactly how T9 and T8's I-1 went wrong. The same reader now serves five call
   sites, and the differential assertion catches a sixth that bypasses it.

2. **`form_1098_outstanding_principal()` was gated too** (not named in the brief). It is the ceiling
   warning's only caller, and leaving it summing every row while the warning gated separately would
   have left a public figure whose value disagrees with the deduction it describes. Its doc comment
   records why.

3. **Two R8 spec corrections beyond the one the brief named.** The brief required only the
   liveness contradiction. I also corrected (a) the 8b/8c **cell pairing**, which the review's D-1
   and the controller's ledger both record as *"must be corrected"* but the fold brief did not
   restate, and (b) the **T9 roadmap row**, which repeated the same *"live iff `schedule_a.is_some()`"*
   claim about the section one table below the paragraph I was fixing. Leaving either would have left
   R8 saying two things after a fold whose purpose was to make it say one.

4. **M-2 resolved by widening the test, not by narrowing the prose** — the brief offered either. The
   build report is unedited as a result.

5. **An unreachable arm kept and documented rather than deleted or silently left** (see M-2 above):
   the home-sale rule's `s_1099 == Some(true)` conjunct. This is a disclosure, not a change.

6. **No advisory or transcription warning for I-1** — the brief asked me to decide and state. Decided
   against both, reasoning in the code beside the mark and in I-1 above.

Nothing in the review's disposition was narrowed or skipped.

---

## Closing gate

```
$ make check
     Summary [  20.671s] 3475 tests run: 3475 passed, 12 skipped
```

3470 → **3475 passed, 0 failed, 12 skipped**, exit 0; `cargo fmt --all` clean; clippy
`--workspace --all-targets --all-features -D warnings` clean; `pii-scan: clean (HEAD)`; all five
instruments unchanged from HEAD. Nothing committed, nothing pushed, no stray files —
`git status --short` is the eleven modified files listed at the top.
