# Schedule 1-A PLAN — review of the **r4 FOLD** (Opus)

**Date:** 2026-09-05 · **Artifact:** the r4 fold of `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`
(header table + the ★★ sequencing correction + the ★★★ r4 CONFORMANCE block in §T3 + the four r4
CORRECTIONS in §T2 + the rewritten §T3a table) · **Brief:**
[`BRIEF-plan-r4-fold.md`](./BRIEF-plan-r4-fold.md)

**Scope honoured:** the fold's *answers* only. SPEC r3 not re-reviewed; r1's and r4's original findings
not re-litigated; no new scope, forms or checkers proposed.

---

## VERDICT

**0 Critical / 4 Important / 4 Minor / 1 Nit — DO NOT BUILD YET.**

The fold's central move is right and I tried hard to break it: `return_inputs.rs`'s class-(A) gate is
real, its doc says verbatim what the fold quotes, and one `Option<bool>` claim gate per part *is* a
smaller and stronger shape than ~25 loose `Option<Usd>`s. Three of the four Importants below are the
fold's own resolutions landing one clause short of the thing they name, and all four decide a **type or
a gate at T2**:

- **I-1** — C-I2's mechanism (*"transcribe each part's Caution"*) closes **neither** of the two cases
  C-I2 names. Part V's Caution is not a completion predicate, so *line 35 still prints $6,000 for a
  non-senior*; Part I has no Caution at all and its predicate is line-scoped, so applying the mechanism
  at part granularity would blank **line 3**, which feeds every phase-out in Parts II–V.
- **I-2** — B-I2's ✅ **FIXED IN CODE** is two-thirds landed. `LineCoverage.year` is per-row but **no
  constructor can set it**, so a Schedule 1-A row resolves to a TY2024 extract that does not exist and
  hard-errors; and `Coverage::exception` was never widened to `Option<Usd>`, which is exactly the type
  C-I2 requires on lines 10/18/27/33.
- **I-3** — the re-scoped T3a refusal repeats, one notch narrower, the mistake it cites: Part II holds
  **two independently-claimable things** (employee tips, trade-or-business tips) and got one gate, so a
  W-2-only tipped employee is refused on a line whose own predicate is false for them. Part II becomes
  unreachable in B3.
- **I-4** — C-I3 restored the refinance **cap** but not the **precondition in the same paragraph**
  (*"your prior loan **that had QPVLI**"*), so a pre-2025 vehicle launders into eligibility by
  refinancing. Understates tax — the direction this project treats as worst.

None of the four is expensive. I-1 and I-4 are sentences; I-3 is a conjunction; I-2 is two small
signature changes plus a T2 line item.

---

## FINDINGS

### I-1 — Important. C-I2's resolution is keyed to "each part's Caution", and **neither** of the two cases C-I2 names is reachable that way

**What the fold says.** Header table, C-I2 resolution: *"transcribe each part's Caution as a completion
predicate; make the affected leaves able to express not completed **at T2**."* §T2 r4 CORRECTION 4:
*"Each part's Caution — 'Fill out Part II only if you received qualified tips' — is transcribed as a
completion predicate, and the KAT asserts an uncompleted part's lines are **not entered**."* The finding
column names the two cases: *"line 35 prints $6,000 for a non-senior"* and *"a filer with no §911
exclusion prints `$0` on 2a–2e."*

**What is actually true.** Only Parts II, III and IV print a completion condition in their Caution.
Checked against `design/forms/extract/f1040s1a--2025.txt`:

| part | the form's Caution | is it a completion predicate? |
|---|---|---|
| I | *(none printed)* | — |
| II | *"Fill out Part II **only if** you received qualified tips…"* | **yes** |
| III | *"Fill out Part III **only if** you received qualified overtime compensation…"* | **yes** |
| IV | *"Fill out Part IV **only if** you, or your spouse if married filing jointly, paid or accrued … (QPVLI)."* | **yes** |
| V | *"You and/or your spouse must have a valid social security number. If married, you must file jointly to claim this deduction. See instructions."* | **NO** — an eligibility bar |
| VI | *(none)* | unconditional |

**(a) Part V — the fold's own named case survives its own mechanism.** Part V's completion condition is
not on the form; it is in the instructions (`i1040gi--2025.txt:44609`):

> "Fill out Schedule 1-A, Part V, only if:
> • You (and/or your spouse if filing a joint return) **were born before January 2, 1961**.
> • You have a valid social security number (SSN). …"

The **born-before-1961** condition appears nowhere in the Caution. So a non-senior with a valid SSN
filing single satisfies the transcribed Caution, Part V is "completed", lines 31–35 are computed, and
**line 35 prints $6,000 for a non-senior** — verbatim the case C-I2 exists to close. Line 37 is still
`$0` (36a/36b gate on the birth date), which is why this is Important and not Critical: no figure is
wrong, but the fold's stated fix does not fix it, and the KAT of CORRECTION 4 would be **green while
blind** to it.

**(b) Part I — no Caution, and the predicate is line-scoped, not part-scoped.** `i1040gi--2025.txt:43275`:

> "If you don't have income from Puerto Rico that you excluded from your income, or you aren't filing
> Form 2555 or 4563, then **enter the amount from Form 1040, 1040-SR, or 1040-NR, line 11b, on
> Schedule 1-A, line 3**. If you do have excluded income … **complete lines 2a through 2e**…"

Part I's correct shape for the common filer is **lines 1 and 3 entered, 2a–2e blank**. There is no
Caution to transcribe, and CORRECTION 4's assertion — *"an uncompleted part's lines are not entered"* —
is false for Part I in the dangerous direction: applied at part granularity it blanks **line 3**, the
MAGI every phase-out in Parts II–V reads (lines 8, 16, 25, 31). An implementer who notices that and
backs off instead prints `$0` on 2a–2e, which is the defect C-I2 filed. Both exits are wrong, and the
fold gives no third.

This is a **T2 struct-shape decision**, per the fold's own reasoning: completion must be expressible at
**line** granularity inside a part, not only at part granularity, and B4 cannot add that later.

**Smallest fix.** In C-I2's resolution and §T2 CORRECTION 4, name the source per part rather than "each
part's Caution": the **form Caution** for II/III/IV; the instructions' *"Fill out Schedule 1-A, Part V,
only if:"* block (`i1040gi--2025.txt:44609`) for V, whose born-before-January-2-1961 condition the
Caution omits; and `i1040gi--2025.txt:43275` for Part I, **scoped to lines 2a–2e**, with lines 1 and 3
always entered. Make the KAT assert a per-line completion set, not "a part's lines".

---

### I-2 — Important. B-I2's ✅ **FIXED IN CODE** is two-thirds landed; the two residues bite precisely on the leaves C-I2 makes optional

**What the fold says.** *"B-I2 … ✅ **FIXED IN CODE** — scope is now derived from the emitter,
`Option<Usd>` counts as money, and **the year is per-row**."*

**What is actually true.** Two of the three hold. Verified:

- scope derived from the emitter — `line_coverage_check.rs:465-490`, `if !mentions_ident(emitter_code,
  &name) { continue; }`, over every module under `tax/` (`:725-745`). Real.
- `Option<Usd>` counts as money — `money_bearing_types` now matches `": Usd" || ": Option<Usd>"`
  (`line_coverage_check.rs:887`), and `Coverage::line` takes `_value: impl Into<Option<Usd>>`
  (`line_coverage.rs:137-141`). Real.

The third does not hold, and there is a fourth gap the fold did not touch:

**(a) `year` is per-row on the struct, and nothing can write it.** `LineCoverage.year` exists
(`line_coverage.rs:106`) and its doc even cites this finding. But `grep -n "pub fn " line_coverage.rs`
returns exactly two constructors, and **both hardcode the year**:

```
line_coverage.rs:148            year: DEFAULT_ROW_YEAR,   // in Coverage::line
line_coverage.rs:170            year: DEFAULT_ROW_YEAR,   // in Coverage::exception
line_coverage.rs:49    pub const DEFAULT_ROW_YEAR: &str = "2024";
```

The const's own doc says *"A TY2025+ form **sets its own**"* — and no API lets it. Traced through the
consumer (`line_coverage_check.rs:530-556`): a Schedule 1-A row built the ordinary way resolves to stem
`f1040s1a--2024`; `design/forms/extract/f1040s1a--2024.txt` does not exist (only `--2025` does,
verified) and `crates/btctax-forms/forms/2024/f1040s1a.map.toml` does not exist, so the checker takes
the **hard-error** branch — *"quotes form … which has NEITHER an extract NOR a map — the form name is
wrong"*. That is the exact third hard-coding B-I2 reports as removed, still armed, and it fires at T2/T5
with a message that misdiagnoses itself.

**(b) `Coverage::exception` still takes `Usd` by value.** `line_coverage.rs:159-161`:
`pub fn exception(&mut self, _value: Usd, …)`. The plan classifies lines **10/18/27/33** as `Exception`s
(§T2 CORRECTION 1), and C-I2 requires those same leaves to express *not completed*. An implementer must
write `.unwrap_or(Usd::ZERO)` — which `Coverage::line`'s own doc says it widened the parameter
specifically to avoid: *"narrowing it to `Usd` would have forced an `unwrap_or` that reads like a claim
about the figure."* The widening reached `line` and not `exception`.

**Smallest fix.** Add a year-carrying path (a `Coverage::for_year("2025")` on the accumulator, or a
`year` parameter on both constructors) and widen `exception`'s `_value` to
`impl Into<Option<Usd>>`. Add both to §T2's task list, since T2 is where the first TY2025 rows land.

---

### I-3 — Important. Part II holds **two independently-claimable things** and the fold gave it one gate, so T3a's refusal swallows the paradigmatic Part II filer

**What the fold says.** T3a, line 5: *"**REFUSE** when the Part II **claim gate** is `Some(true)` …
★★ **NOT `schedule_c.is_some()`**"*, justified by `questions.rs`'s recorded lesson: *"it was
`live: |_| true` — so it blocked EVERY return btctax could compute … and it bought nothing."*

**What is actually true.** Correcting `schedule_c.is_some()` → the claim gate is right, and it is not
far enough. Part II has two sources with different input paths:

- lines **4a–4c** — the *employee* side (W-2 box 7, Form 4137);
- line **5** — *"Qualified tips received **in the course of a trade or business**"*, from 1099-NEC /
  1099-MISC / 1099-K.

A W-2-only tipped employee — a waiter, the form's own worked example — has no trade or business, so
line 5's own predicate is determinately **false** and blank is the correct entry: there is nothing to
collect and nothing to be uncertain about. Under the fold they claim Part II, the gate is `Some(true)`,
and **the whole return refuses**. That is the cited failure one notch narrower: it no longer blocks
every return, it blocks every return that claims Part II — i.e. 100% of Part II's population — leaving
Part II unreachable in B3 and unexercisable end-to-end against T7's per-part oracle census.

The distinction is decidable in the current tree, verified:

- `ReturnInputs::schedule_c: Option<ScheduleCInputs>` (`return_inputs.rs:704`), and there is no
  Schedule E, Schedule F, or Schedule 1 line 8z other-income surface at all — so "no trade or business"
  is a fact btctax holds, not a guess;
- `other_out_of_scope_income` (`return_inputs.rs:~997`) refuses on `None` and on `Some(true)` and is
  explicitly *"scoped to **income** only"*, so the 1099-MISC box 3 half the fold correctly flags for
  line 14b is already backstopped for a filer with no Schedule C.

**Smallest fix.** Refuse on the **conjunction** — the part is claimed **and** a trade or business
exists (`schedule_c.is_some()`) — for line 5 and for line 14b; and record that with no trade or
business the line is *genuinely blank because the form's own predicate is false*, naming
`other_out_of_scope_income` as the backstop. Stated in the fold's own vocabulary: the claim gate is
**per part**, but the refusal predicate is **per line**, and Part II is the part where those differ.

*(This is the "tests for conformance" shape: the guard that should exist is a KAT that a W-2-only
tipped employee with no Schedule C produces a Part II deduction and does **not** refuse — it reds today
under the fold as written.)*

---

### I-4 — Important. C-I3 restored the refinance **cap** and dropped the **precondition in the same paragraph** — an understatement path

**What the fold says.** §T3, r4 CONFORMANCE (1): quotes the *Refinanced loan* paragraph, then *"⇒ add a
per-vehicle row asking whether this is a refinancing and, if so, the outstanding balance of the
refinanced loan on the refinancing date — limiting the interest to that fraction; and correct the
prose."*

**What is actually true.** The quotation is verbatim (`i1040gi--2025.txt:44486-44494`, checked; the
plan's whole quotation set is held green by
`cite_check::every_quotation_in_the_schedule_1a_documents_is_verbatim_from_the_manual`, which I ran).
The **cap** is restored and the prose is retracted. What is not carried is the paragraph's opening
condition — its first eight words:

> "If your **prior loan that had QPVLI** is later refinanced, interest paid on the refinanced amount is
> generally eligible…"

That clause is what the carve-in turns on. The five general requirements (`i1040gi--2025.txt:44445-44458`)
are *"1. **Your loan was originated after December 31, 2024.** … 3. **The proceeds from your loan were
used to purchase an APV** (lease payments do not qualify)."* A refinance's proceeds repay a prior loan
rather than purchase a vehicle, which is precisely why the *Refinanced loan* paragraph exists — and its
price is that the **prior** loan must itself have been a QPVLI loan.

The fold's row does not ask that. Walk a filer through the collected YES-conditions with a car bought in
**2023** and refinanced in 2025:

| collected condition | filer's honest answer |
|---|---|
| originated after 2024-12-31 | **yes** — the *new* loan was |
| originated by you | yes |
| proceeds used to purchase an APV | **yes** — as they read it, the loan is on the car they purchased |
| personal use / first lien / original use / US assembly / no negative equity | yes |
| is this a refinancing, and the prior balance? | yes, $X — and the cap binds at $X, which is the whole balance |

Every gate answers yes and up to **$10,000** of interest on a pre-2025 vehicle is deducted.
**Understates tax** — same direction, same paragraph, same class as C-I3 itself, and the same class
CLAUDE.md records as the one this project keeps rediscovering.

**Smallest fix.** The refinance row carries a second YES-condition defaulting to NO — *"the loan being
refinanced was itself a qualifying QPVLI loan (originated after 2024-12-31, by you, to purchase this
APV, secured by a first lien)"* — and one sentence stating that **for a refinance, requirement 1 is
tested against the prior loan, not the new one**.

---

### M-1 — Minor. C-I1's resolution sits on the far side of a crate boundary the plan does not mention

`label_reader` lives in `crates/xtask/src/`, and `xtask` **depends on** `btctax-core`
(`crates/xtask/Cargo.toml:20`); `btctax-core` does not depend on `xtask` (verified — the reverse would
be a cycle). §T2 puts the struct **and its KAT** in `crates/btctax-core/src/tax/schedule_1a.rs`, from
which `label_reader` is unreachable. Its fixtures compound it: `label_reader.rs`'s tests load via
`form_geometry::load(&form_geometry::repo_root(), …)` — a repo-root path outside the crate, which
btctax-core tests deliberately avoid (the in-crate `fixtures/schedule_1a_2025_{form,instructions}.txt`
exist for exactly that reason, and this repo has shipped a broken tarball to an escaping `include_str!`
before).

Not blocking, because a working placement exists and is one sentence: put the conformance KAT in
`xtask`, which can see both `label_reader` and `btctax_core::tax::schedule_1a::*`, so B-I1's
`(label, got.lineN)` tuple form still ties the compiler to the struct. Or commit `label-census`'s
adjudicated rows as an in-crate JSON fixture with a freshness check. Say which.

**On the rest of C-I1, the fold is right and now better than when written.** It drives the expected set
from the reader's adjudication rather than a hand-list, which is what CLAUDE.md requires; the reader's
`ece7e668` gutter defect did not touch the Schedule 1-A anchor; and the reader carries its own B1
kill-test (the synthetic two-money-column form in `label_reader.rs`). FR-28's residual — a second entry
on the same row — is Form 1040 only and cannot reach this form.

### M-2 — Minor. T2's KAT has no named B1 half, and the fold gave it three new halves

The plan carries exit criterion 5 (*"Mutation-verified, per guard. A guard whose mutation survives is
not a guard."*) and T6's *"Mutation-verify every guard"* — generic cover. But CLAUDE.md B1 is scoped by
name to *"every new census, **conformance check**, citation check, lint, or review harness"*, and this
KAT is the plan's declared mechanical gate (§0). It now has four halves and each needs its own planted
defect, because a mutation that kills one leaves the other three green:

1. membership — drop a line from the struct;
2. per-line quotation — move line 28's *"increase … to the next higher"* onto line 11 (the swap
   `printed_line` exists to catch, and the one that inverts the rounding for Parts II/III);
3. provenance — declare a field and never assign it;
4. completion — complete a part whose predicate is false (the I-1 case: a non-senior reaching line 35).

Name the four. The reviewable question stays one sentence with a factual answer.

### M-3 — Minor. Rule (4b) still cannot reach Schedule 1-A during B3 — now for a different reason

The B-I2 fix replaced the three-file hand-list with a derived predicate: *"a type is IN SCOPE iff the
emitter crate names it in real code"* (`line_coverage_check.rs:474-490`). Measured:
`grep -rn 'Schedule1a|schedule_1a|f1040s1a' crates/btctax-forms/src/ crates/btctax-forms/forms/`
returns **zero hits**, and §3 puts the emitter and the AcroForm map in **B4**. So for the whole of B3
the checker contributes zero Schedule 1-A rows and reports OK — the consequence B-I2 named, arriving by
a new route. The Production requirement is therefore held by **T2's KAT alone** during B3. Say so in
§T2, so nobody reads "✅ FIXED IN CODE" as cover for a checker that structurally cannot see this form
yet.

### M-4 — Minor. B-I5's second half is deferred into a task that carries no landing place for it

The new 4b row ends *"T2 must state whether that refusal needs a companion declaration"*, answering the
buildability fix's *"state whether the existing `box8_allocated_tips` refusal is the guard"* with a
deferral. §T2 contains no Form 4137 item, so the deferral has nowhere to land and no owner. Direction is
fail-closed (line 4c takes the *larger* of 4a/4b, so a false `-0-` shrinks the deduction), which is why
this is Minor. Add the item to §T2 or answer it in T3a. (`return_refuse.rs:1182` is the refusal, and it
is `> Usd::ZERO` on box 8 only, as the fold states.)

### N-1 — Nit. The fold's own new text carries stale line citations, including the one its central claim rests on

The buildability lens filed this as M-3 and the fold moved none of them. Current values:

| cited in the fold | actual |
|---|---|
| `return_inputs.rs:626-652` (the class-(A) gate — the sequencing correction's evidence) | `:960-1032` |
| `return_inputs.rs:417-423` (§T3a) | `:691-704` |
| `return_refuse.rs:769` | `:1182` |
| `questions.rs:546-552` | `~:810` |

Every **claim** made about that code checks out; only the addresses have decayed.

---

## WHAT I VERIFIED AND HOW

**The fold's central sequencing correction — sound, and quoted accurately.** Read
`return_inputs.rs:960-1032`. `has_income_exclusion: Option<bool>` is the class-(A) gate; the four
add-backs (`excluded_puerto_rico_income`, `form_2555_line45`, `form_2555_line50`, `form_4563_line15`)
hang off it as plain `Usd`; `modified_agi()` (`:1063`) returns `None` when the gate was never asked. The
field doc says verbatim what the fold quotes: *"whereas `Option<Usd>` is a scalar the `_` rule permits,
which would make this convention again."* The fold's *"Part I needs no new input at all — its four
add-backs are already collected"* is true as stated. One gate per part is the right shape; my I-3 is
that the **refusal** predicate is finer than the claim gate in Part II, not that the gate is wrong.

**Part V and Part IV's per-person / per-vehicle question, checked and clean.** Part V's two
independently-claimable things (36a taxpayer, 36b spouse) are covered by the plan's per-person SSN bar
plus §G-9's `Person::date_of_death` / `reaches_65_on`; Part IV's declarations are already stated
per-vehicle in §T3 even though the header sentence says "per part". Neither is a finding.

**The form's own text**, `design/forms/extract/f1040s1a--2025.txt` (113 lines, read in full) — the six
parts, all four Cautions, line 22's two-row a/b structure, line 33's *"enter $6,000 on line 35"*, and
lines 36a/36b's birth-date gate.

**The instructions**, `design/forms/extract/i1040gi--2025.txt` — the four *"Fill out Schedule 1-A, Part
X, only if"* blocks (`:43370`, `:44020`, `:44416`, `:44609`, located by grep, not by memory), Part I's
completion sentence (`:43275`), the five loan requirements (`:44445-44458`), the change-in-obligor
exception, the *Loan amount* / negative-equity paragraph, and the *Refinanced loan* paragraph
(`:44486-44494`).

**Code, read rather than inferred:** `line_coverage.rs:44-175` (the `year` const, the `LineCoverage`
fields, both constructors, the two signatures); `line_coverage_check.rs:455-500, 523-560, 682-745, 849-890`
(the emitter-derived scope, the extract-resolution error path, the module scan, the type detector);
`label_reader.rs:1-70, 742-780` (the two witnesses, `Kind::Heading`, the 50/48/`["4","22"]` assertion,
the B1 kill-test); `return_inputs.rs`; `return_refuse.rs`; `crates/xtask/Cargo.toml`.

**Commands run:**

- `cargo nextest run -p xtask -E 'test(schedule_1a) or test(cite)'` → **4 passed**, including
  `every_quotation_in_the_schedule_1a_documents_is_verbatim_from_the_manual` and
  `a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`. The fold's new quotations are verbatim.
- `grep -rn 'Schedule1a|schedule_1a|f1040s1a' crates/btctax-forms/src/ crates/btctax-forms/forms/` → 0.
- `grep -n 'xtask' crates/btctax-core/Cargo.toml` → none (dependency direction confirmed one-way).
- `ls design/forms/extract/ | grep 1040s1a` → `f1040s1a--2025.txt` only.
- `grep -n 'pub fn ' crates/btctax-core/src/tax/line_coverage.rs` → `line` and `exception` are the only
  constructors.

**Taken as given per the brief, not re-run:** `cargo run -p xtask -- label-census f1040s1a--2025`
→ 48 entry lines, 2 without a box.

**Also checked and sound:** B-I4's ✅ is a real correction (T3a no longer keys on `schedule_c.is_some()`);
B-I5's ✅ row exists with its move and reason; C-I4's illegal-service row is present and matches the
instructions' Example-1/Example-2 pair; the M-1 recount (≈19 declarations) and the M-2 heading
correction (4 and 22, not 4 and 14) are both right and both landed; the r4 fold did not disturb T1's
rounding rulings, the exhaustion table, or the fail-closed TY2025 gate.

## WHAT I COULD NOT CHECK

- **I did not run the full suite or `make check`.** The findings are static and the plan is unbuilt, so
  nothing here is a red-suite claim; I ran only the cite-check and label-reader tests above.
- **`cite-check`'s own blind spot (`FOLLOWUPS.md` FR-21 — the plain-quotation pass can be gutted with
  all tests green)** means the green result above is weaker evidence than it looks for quotations
  written as plain `"…"` rather than blockquotes. I spot-verified the fold's two load-bearing
  quotations (the refinance paragraph and the `return_inputs` doc) against source by hand instead.
- **The emitter's blank-vs-zero behaviour.** The conformance lens flagged that its I-2 softens if B4's
  emitter suppresses zero-valued boxes. B4 is out of scope, `crates/btctax-forms` has no Schedule 1-A
  asset, and §G-11's recorded position is that the emitter cannot express blank — so I reasoned as the
  fold does. If that position has changed, I-1's *printed-testimony* half softens; its **struct-shape**
  half (line-level vs part-level completion, and line 3) stands regardless, and that is the half T2
  cannot undo.
- **T7's oracle census** — needs `OTS_DIR` and the `.venv`; not attempted, and untouched by this fold.
- **Whether the owner's real TY2025 return carries a Schedule C**, which sets I-3's blast radius for
  line 14b specifically. I-3 stands on the employee-tips path either way.
