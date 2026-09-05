# Form 8615 (kiddie tax) — the screen, its inputs, and its refusals — SPEC

**Status: DRAFT r1** (written 2026-09-05, branch `feat/schedule-1a-ty2025`). Fixes `FOLLOWUPS.md`
FR-29 (`FOLLOWUPS.md:5722-5744`), a **★★★ CRITICAL understatement path** whose owning phase is
*"BEFORE any first filing"*.

Passes an independent review loop to **0 Critical / 0 Important** before an implementation plan.
`design/ty2025/SPEC.md`'s parent decisions bind here unless restated.

**Scope.** The *screen* — deciding whether Form 8615 is required, collecting what the form's own
conditions ask, and refusing when it is. **Filling Form 8615 is out of scope** and stays out: the
form needs the parent's taxable income and every sibling's §1(g) amount, none of which btctax ever
sees.

---

## 1. Sourcing of record — READ THIS FIRST

There is no separate `i8615` in the archive for this cycle. **`i1040gi` carries the whole test**, in
the file that already sources Schedule 1-A. Everything below is quoted from the extracted text layer
(`CLAUDE.md`, *Transcribe IRS forms — never paraphrase them*), never from a rendered page.

| authority | extract of record | what it holds |
|---|---|---|
| `i1040gi--2025.pdf` (`482e9c48`, `design/ty2025/SPEC.md:70`) | `design/forms/extract/i1040gi--2025.txt` | the five Form 8615 conditions (`:3927-3944`), the January-1 age rule (`:3945-3951`), Chart A (`:625-666`), Chart B (`:691-728`), Chart C (`:732`) |

Every quotation in this document was checked against that extract by normalised containment before it
was written. **P-1 (plan task):** register this file with `cargo run -p xtask -- cite-check` by adding
it and `design/forms/extract/i1040gi--2025.txt` to `schedule_1a_docs()`
(`crates/xtask/src/cite_check.rs:404-416`) so the check is a command rather than a promise.

**No oracle can adjudicate this.** Neither engine derives Form 8615's applicability: `grep 8615`
over `scripts/oracle/*.py` is empty, and Tax-Calculator models only the *different* §59(j) kiddie-AMT
exemption cap (`.venv/lib/python3.12/site-packages/taxcalc/calcfunctions.py:2431,2489,2592`), keyed on
`age_head`, not on §1(g). This is the §G-9 limit in its purest form — **the form is the authority and
there is no witness** — so every guarantee below is held by a KAT against the extract, not by
agreement.

### 1.1 The five conditions, verbatim

`design/forms/extract/i1040gi--2025.txt:3927-3944`:

> *"You must file Form 8615 if you meet all of the following conditions."*
>
> 1. *"You had more than $2,700 of unearned income (such as taxable interest, ordinary dividends, or
>    capital gains (including capital gain distributions))."*
> 2. *"You are required to file a tax return."*
> 3. *"You were either:"*
>    a. *"Under age 18 at the end of 2025,"*
>    b. *"Age 18 at the end of 2025 and didn’t have earned income that was more than half of your
>       support, or"*
>    c. *"A full-time student at least age 19 but under age 24 at the end of 2025 and didn’t have
>       earned income that was more than half of your support."*
> 4. *"At least one of your parents was alive at the end of 2025."*
> 5. *"You don’t file a joint return in 2025."*

And the age convention (`:3945-3951`):

> *"A child born on January 1, 2008, is considered to be age 18 at the end of 2025; a child born on
> January 1, 2007, is considered to be age 19 at the end of 2025; and a child born on January 1,
> 2002, is considered to be age 24 at the end of 2025."*

**Dependency appears in none of the five.**

---

## 2. The defect, measured

`crates/btctax-core/src/tax/return_1040.rs:989`:

```rust
if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false) {
```

That one condition gates the entire screen (`return_1040.rs:979-1003`). A filer who answers *"No"* to
*"Can someone claim YOU as a dependent on their return?"* (`crates/btctax-core/src/tax/questions.rs:289`)
never reaches the `unearned` computation at all, is taxed at their own rates, and files.

**Whom that loses.** A self-supporting minor or student whose support comes from *unearned* income —
this product's own user. They are not claimable as anyone's dependent (they provide more than half
their own support), so they truthfully answer No; but Form 8615 asks about **earned** income
(*"didn’t have earned income that was more than half of your support"*), and unearned support does
not satisfy it. Every one of the five conditions holds. §1(g)(1) applies *"the greater of"* the child's
rate and the parent's-rate computation, so **this can only understate**.

**A green test pins the wrong reading**, and says so: `return_1040.rs:4465-4475` carries the FR-29
warning and then asserts

```rust
let mut not_dep = dependent(dec!(9000));
not_dep.header.can_be_claimed_as_dependent_taxpayer = Some(false);
assert_eq!(screened(&not_dep, &empty), None);
```

`RefuseReason::KiddieTax`'s doc comment (`crates/btctax-core/src/tax/return_refuse.rs:263-273`) carries
the same correction. **Both the comment and the doc were fixed on 2026-09-04; the gate was not.**

---

## 3. Why the obvious fix is worse than the defect

Replace the dependency test with *screen unless the filer is provably outside Form 8615’s reach*
and the screen fires for everyone. `Person::date_of_birth` is `Option<Date>`
(`crates/btctax-core/src/tax/return_inputs.rs:198,228`) and is deliberately a **class-(B) skippable**
— *"A mandatory DOB prompt would force the filer to INVENT a birthday,
and an invented-old one understates tax — so `None` must stay reachable."* (`questions.rs:800-801`). Conditions 3 and 4 are not
collected at all. So *"provably outside"* would refuse every filer with no DOB on file and more than
the threshold of unearned income — in a crypto tax tool, most of them.

**This repo's rule decides it** (`CLAUDE.md`, *Transcribe IRS forms*): *"If the form asks something our
input surface cannot answer, collect it. That is following instructions, not scope creep."*

### 3.1 The shape: class-(A) semantics in the class-(B) registry

The brief for this spec asked for the class-(A) `FORM_QUESTIONS` pattern. **This spec deliberately
does not use it, and the reason is structural, not stylistic.**

`FormQuestion::live` is `fn(&ReturnInputs) -> bool` (`questions.rs:53-54`) and `screen_inputs` refuses
every live-and-`None` declaration *before* any compute (`return_refuse.rs:823-826`). But condition 1 —
the only condition that makes the question worth asking — needs the **ledger**: `unearned` includes
`crypto.nonbusiness_ordinary` and `capital_gain_line7(ri, state, year, …)`
(`return_1040.rs:990-996`), and `ReturnInputs` carries no crypto by construction
(`return_1040.rs:1045-1046`). Two dead ends follow:

- **Liveness ignoring condition 1** ⇒ a hard `screen_inputs` refusal on every DOB-less return,
  including a return with zero unearned income. Strictly worse than the defect for most filers.
- **Liveness using only the `ReturnInputs`-visible unearned income** ⇒ a filer whose unearned income
  is entirely crypto has the question **not live**, so `btctax income answer` never offers it, while
  the compute-time gate demands it: **a refusal with no reachable remedy.** That is the shipped
  circular-liveness bug of `questions.rs:344-345` in a new costume.

The registry already has the right shape for exactly this, twice, and both are shipped and tested:
`SkippableId::ScheduleCIsCooperativePatron` and `ScheduleCIsSstb`. Their contract is stated at
`crates/btctax-input-form/src/attribute.rs:68` — *"SKIPPABLES, offered always and mandatory only where
the answer changes the form"* — with the mandatory half living in the compute-time screen that can see
what the input screen cannot (`return_1040.rs:2676-2690`).

So: **class-(A) semantics — silence never answers for the filer, and where the answer changes the
number btctax refuses rather than guess — delivered through `SKIPPABLE_QUESTIONS`, whose liveness can
be broad and whose mandatory half sits in `screen_compute_dependent`.**

The justification for refusal is the registry's own three-part test (`questions.rs:837-839`): refusal
is warranted only when proceeding without the answer would *produce a wrong number*, *put fabricated
testimony on a signed return*, or *silently expose the filer to a penalty or a lost right*. Here the
**first** limb is met exactly: proceeding computes tax at the child's rate where §1(g) requires the
greater of that and the parent's-rate figure.

---

## 4. What is COLLECTED — two questions, one per numbered condition

Two `Option<bool>` leaves on `HouseholdHeader`, beside the dependency flag
(`crates/btctax-core/src/tax/return_inputs.rs:262`), each named for the condition it transcribes,
each `#[serde(default)]`, each carrying the instruction text verbatim as its doc comment.

```rust
/// **Form 8615, condition 3** (`design/forms/extract/i1040gi--2025.txt:3932-3940`), verbatim:
/// "3. You were either:
///     a. Under age 18 at the end of 2025,
///     b. Age 18 at the end of 2025 and didn’t have earned income that was more than half of your
///        support, or
///     c. A full-time student at least age 19 but under age 24 at the end of 2025 and didn’t have
///        earned income that was more than half of your support."
///
/// ONE leaf, because the form states ONE numbered condition with three alternatives and asks the
/// filer for the disjunction ("You were either: a…, b…, or c…"). Splitting it into three would be
/// the compression this repo's rule forbids, in reverse: it would require btctax to re-derive the
/// disjunction the form already writes.
///
/// `None` ⇒ REFUSED by `screen_compute_dependent` wherever the answer changes the number, and unread
/// everywhere else. It never defaults in either direction. See `Self::form8615_condition4_parent_alive`.
#[serde(default)]
pub form8615_condition3_age_support: Option<bool>,

/// **Form 8615, condition 4** (`design/forms/extract/i1040gi--2025.txt:3941-3942`), verbatim:
/// "4. At least one of your parents was alive at the end of 2025."
///
/// `None` ⇒ REFUSED, on the same terms as `Self::form8615_condition3_age_support`, and additionally
/// only once condition 3 is answered YES — the form reaches condition 4 no other way.
#[serde(default)]
pub form8615_condition4_parent_alive: Option<bool>,
```

### 4.1 The registry entries

Two `SkippableQuestion`s appended at the **end** of `SKIPPABLE_QUESTIONS`
(`crates/btctax-core/src/tax/questions.rs:952`). Appended so the `Skippables` section's field order —
derived from registry order at `crates/btctax-input-form/src/spec/mod.rs:295-300` — grows rather than
shifts. ★ Note the `decl_tristate!` INDEX hazard recorded at `questions.rs:99-102` binds
`FORM_QUESTIONS`, **not** this registry: `skippable_to_field` matches on `SkippableId` by name, so a
mid-array insert here mis-orders a UI section rather than silently repointing a question. Both `SkippableKind::YesNo`, both
`Durability::PerYear` (age and a parent's survival both change between years; the durability test at
`questions.rs:1587-1598` asserts that *exactly* the two dates of birth are `Durable`, so `PerYear` is
required, not merely preferred).

**Condition 3 — prompt** (year-free, because `prompt` is `&'static str` and TY2024 is still the only
year btctax can file; the year-bearing sentences live in the doc comment and in the refusal detail,
which is `format!`ed at the gate where `year` is in scope):
```text
Form 8615, condition 3 — at the end of the tax year, were you either: (a) under age 18, (b) age 18
and didn’t have earned income that was more than half of your support, or (c) a full-time student
at least age 19 but under age 24 and didn’t have earned income that was more than half of your
support? Answer YES if any one of (a), (b) or (c) is true.
```

★ The **only** permitted departure from the extract is *"at the end of 2025"* → *"at the end of the
tax year"*, three times. Every other clause is byte-identical, and §9's `prompt-check` asserts it
against the extract.

**Condition 3 — help:**
```text
Form 8615 taxes part of a child's unearned income at the parent's rate (§1(g)). Skipping is harmless
if your unearned income is at or below the §1(g) threshold for the year, or if btctax can already
see from your date of birth that you were 24 or older at the end of the year — condition 3 cannot be
true at 24. Where it does matter, btctax refuses rather than answer for you: §1(g)(1) takes the
GREATER of your own rate and the parent's-rate figure, so a wrong "no" can only understate your tax.
"Support" is the whole cost of keeping you for the year; the test is whether your EARNED income —
wages, salaries, tips, professional fees — covered more than half of it. Income from investments,
crypto or a trust is not earned income.
```

**Condition 4 — prompt:**
```text
Form 8615, condition 4 — was at least one of your parents alive at the end of the tax year?
```

**Condition 4 — help:**
```text
Form 8615's condition 4. It is asked only when you answered YES to condition 3, because the two
together are what send you to Form 8615. Skipping is harmless if condition 3 is "no"; otherwise
btctax refuses rather than answer for you.
```

### 4.2 Liveness — when is each even ASKED?

Pure functions of `ReturnInputs`, as the registry requires:

```rust
// condition 3
live: |ri| ri.filing_status != FilingStatus::Mfj && !provably_24_or_older(ri, ri.tax_year),
// condition 4
live: |ri| ri.filing_status != FilingStatus::Mfj
        && !provably_24_or_older(ri, ri.tax_year)
        && ri.header.form8615_condition3_age_support != Some(false),
```

- **`!= Mfj`** is condition 5, computed (§5.3). A joint filer is never asked.
- **`!provably_24_or_older`** is the computed half of condition 3 (§5.1). A filer with a date of birth
  showing 24 or older at year end is never asked either question. This is what keeps the interview
  from asking a 60-year-old about full-time student status.
- **Condition 4 additionally requires condition 3 not to be a definite "no"**, so a filer who answers
  condition 3 = No never sees condition 4. `None` keeps it live, so both are offered together on a
  first run and there is no answer-then-come-back-for-the-next-one loop.

**Condition 1 is deliberately NOT in the liveness predicate** — it cannot be, per §3.1 — which makes
the live set a strict superset of the demanded set. Over-asking is a recorded outcome here, with
precedent (`questions.rs:309-310`: *"a stale spouse on a non-MFJ return is a recorded over-ask (§3.1),
never an under-ask"*). §9 G8 turns "superset" into a test.

★ **`ri.tax_year` vs the gate's `year`.** The gate is handed the authoritative `year`
(`return_1040.rs:906-911`); liveness only has `ri.tax_year` (`return_inputs.rs:688`, added by §G-15 for
exactly this). The CLI keeps them equal — `crates/btctax-cli/src/return_inputs.rs:75` assigns
`ri.tax_year = year` on load and `:103-110` errors on a stored disagreement — and where they *can*
differ (a hand-built `ReturnInputs`, whose `Default` is `tax_year: 0`, `return_inputs.rs:1081`) the
divergence is one-directional: a smaller `tax_year` yields a smaller computed age, so the question is
**live** when the gate might not demand it. Never the reverse. That is the safe direction and §9 G8
pins it.

---

## 5. What is COMPUTED — conditions 1, 2, 5, and the age arithmetic

### 5.1 The computed half of condition 3: `provably_24_or_older`

All three limbs of condition 3 bound the filer's age below 24 — (a) *"Under age 18"*, (b) *"Age 18"*,
(c) *"at least age 19 but under age 24"* (`:3933-3940`). **Therefore condition 3 is false for anyone
aged 24 or over at the end of the year.** That is an equivalence, not a heuristic, and it is the only
computation this spec permits to *suppress* a question.

The IRS's own convention is that a person attains an age on the day before their birthday, which
`i1040gi:3945-3951` states as three worked examples. Encode it as an age at a fixed boundary rather
than as a date:

```rust
/// The age a person born on `dob` is CONSIDERED to be at the end of `year` — i.e. their age on
/// January 1 of `year + 1`, which is what i1040gi:3945-3951's three examples state.
fn considered_age_at_year_end(dob: Date, year: i32) -> i32 {
    year - dob.year() + i32::from(dob.month() == Month::January && dob.day() == 1)
}

/// Condition 3 is FALSE by arithmetic — for a KNOWN date of birth only.
fn provably_24_or_older(ri: &ReturnInputs, year: i32) -> bool {
    ri.header
        .taxpayer
        .date_of_birth
        .is_some_and(|d| considered_age_at_year_end(d, year) >= 24)
}
```

**Why this form and not `reaches_65_on`'s (`return_1040.rs:110-122`).** That helper computes a *date*
and therefore needs a February-29 fallback (`replace_year` fails on a leap day). This computes an
*age at a fixed boundary*, so no date is constructed and no leap-day branch exists — February 29 is
never January 1. §9 G3 pins a leap-day vector anyway, because "no branch is needed" is a claim, not a
guarantee.

**The equivalence proof and the branch where it breaks** (`CLAUDE.md` requires both, plus a KAT):
the derivation is exact for a correct date of birth; it breaks on a **wrong** date of birth, in the
suppressing direction. That exposure is not new — the same field already decides the §63(f) age-65
addition (`return_1040.rs:89-91`, `:110-122`) — and it is bounded to filers who volunteered a DOB. `None` never
suppresses. KAT: §9 G3.

### 5.2 Condition 1 — computed, unchanged

`unearned` keeps the existing component sum and the existing conservative-direction argument at
`return_1040.rs:980-988`: it omits the Schedule 1 adjustments that Form 8615's `AGI − earned` would
net out, so it can only be **too high** and can therefore only over-refuse. **Do not "fix" that
without preserving the direction.** The comparison stays strictly greater than
`params.kiddie_unearned_threshold`, because the form says *"more than $2,700"*.

The threshold is a per-year parameter (`crates/btctax-core/src/tax/tables.rs:470`), currently
`dec!(2600)` for TY2024 (`crates/btctax-adapters/src/tax_tables.rs:134`). **The `$2,700` in
`i1040gi--2025` is the TY2025 value and lands with `ty2025_full_return()`** — which does not exist
yet (`full_return_for(2025)` returns `None` by design, `tax_tables.rs:814-820`). **Nothing in this
spec may hardcode either number.**

### 5.3 Condition 5 — computed

*"You don’t file a joint return in 2025."* ⇒ `ri.filing_status != FilingStatus::Mfj`. Exact, one
field, no collection. QSS is not a joint return and is therefore *not* exempted by this condition.

### 5.4 Condition 2 — assumed TRUE, with the branch named

*"You are required to file a tax return."* btctax cannot compute this without transcribing Charts A,
B and C (`i1040gi:625-666`, `:691-728`, `:732`). **It is assumed TRUE.** Direction: assuming a
condition of a conjunction is TRUE can only make the conjunction more often true, hence only ever
refuse more — fail-closed.

**Exact for the ordinary dependent.** Chart B's first bullet for a single dependent who is neither 65+
nor blind is *"Your unearned income was over $1,350."* (`:699`), and condition 1 already requires
unearned income above the (larger) §1(g) threshold. So for that filer the assumption is not an
assumption at all.

**The branch where it breaks**, named precisely so the KAT can pin it: a filer who is **not** required
to file yet meets conditions 1, 3, 4 and 5. Two populations reach it —

1. a **non-dependent** below Chart A's threshold (`$15,750` single under 65, `:634-639`), i.e. gross
   income between the §1(g) threshold and that figure; and
2. a **blind dependent under 24**, for whom Chart B's *"Yes"* branch raises the bullet to *"Your
   unearned income was over $3,350 ($5,350 if 65 or older and blind)."* (`:705`) — above the §1(g)
   threshold, so the range in between is genuinely not a filing requirement.

Both are refused where the form would not require Form 8615. Both are **over**-refusals of a filer
who was not required to file at all, and neither produces a wrong number. **OQ-2** proposes the
Chart A/B/C transcription that would close them; §9 G6 pins the branch so it cannot be forgotten.

---

## 6. The new gate

Replaces `return_1040.rs:979-1003` in place, inside `screen_compute_dependent` — the screen that
already has `(ri, state, year, params)`. **The dependency flag is not read.** Its deletion is the fix;
because the identifier disappears, no refactor can quietly restore the old behaviour without
re-introducing a reference someone must write down.

Evaluate the form's conjunction, three-valued:

| condition | source | value when unknown |
|---|---|---|
| 1 — unearned over the threshold | computed from `ri` + ledger (§5.2) | never unknown |
| 2 — required to file | assumed `true` (§5.4) | n/a |
| 3 — age/support | `false` if `provably_24_or_older`; else the collected answer | **UNKNOWN** |
| 4 — a parent alive | irrelevant if 3 is `false`; else the collected answer | **UNKNOWN** |
| 5 — not a joint return | `ri.filing_status != Mfj` | never unknown |

Ladder, in order:

1. `!c1 || !c5` → **proceed** (no refusal). One of the two is proved FALSE — condition 1 by
   arithmetic, condition 5 by filing status — so the conjunction cannot hold.
2. `c3 == false` → **proceed.** Either the filer said No, or the date of birth proves 24-or-older.
3. `c3 == UNKNOWN` → **refuse `Form8615AgeSupportUnanswered`.**
4. `c4 == UNKNOWN` → **refuse `Form8615ParentAliveUnanswered`.** (Reached only with `c3 == true`.)
5. `c4 == false` → **proceed.**
6. otherwise → **refuse `KiddieTax`.** All five conditions hold.

### 6.1 The unknown cases fail closed — the proof

Enumerate every combination of the two collected answers with condition 1 true and condition 5 true
(the only region where anything is demanded). `DOB≥24` means `provably_24_or_older`:

| DOB≥24 | cond 3 | cond 4 | outcome | why it is safe |
|---|---|---|---|---|
| yes | any | any | proceed | condition 3 is **proved false** (§5.1) — not assumed |
| no | `None` | any | refuse *AgeSupportUnanswered* | silence is not a No |
| no | `Some(false)` | any | proceed | the **filer** said No |
| no | `Some(true)` | `None` | refuse *ParentAliveUnanswered* | silence is not a No |
| no | `Some(true)` | `Some(false)` | proceed | the **filer** said No |
| no | `Some(true)` | `Some(true)` | refuse `KiddieTax` | all five conditions hold |

**Every "proceed" is licensed by a proof or by the filer's own answer. No row reaches "proceed"
through an absent answer.** That is the *widening an exemption is never the safe edit* rule
discharged: the YES-conditions are enumerated and the fallback is refusal, so every omission is an
over-refusal (recoverable) rather than an understatement (not).

★ And note which direction the two remaining *computed* conditions push. Condition 2 assumed TRUE and
`unearned` computed too high (§5.2) both make the conjunction **more** true. There is no computed
term in this gate whose error can silently exempt a filer, other than a wrong date of birth §5.1
already names.

### 6.2 Provenance — what each new leaf is, for the census

`crates/btctax-core/src/tax/classifier.rs:265-282` destructures `HouseholdHeader` with no `..`, so both
new leaves are a **compile error** until classified. Neither is a `Census::declaration` (that is
reserved for `FORM_QUESTIONS`), and neither is a `BenefitClaim` — no benefit is claimed by answering.
Use the `Class::NoTaxDirection` idiom already established for `qbi_w2_wages` / `qbi_ubia`
(`classifier.rs:498-509`), whose reason string is exactly this shape — *refused where it is needed,
unread where it is not, so it defaults in neither direction*:

```rust
c.exempt(
    form8615_condition3_age_support,
    Class::NoTaxDirection,
    "Form 8615 condition 3 (i1040gi:3932-3940) — `None` is REFUSED by screen_compute_dependent \
     wherever conditions 1 and 5 hold and age 24+ is not provable, and is unread everywhere else, \
     so it defaults in neither direction",
);
```

**OQ-1** asks whether that stretch of `NoTaxDirection` (documented as *"a lawful silent default"*,
`classifier.rs:50-51`) deserves its own variant now that three leaves share the pattern.

---

## 7. Migration — an existing vault has none of this

`#[serde(default)]` makes both leaves `None` on every vault written before this change. That is the
correct representation: the filer has not been asked.

**What must NOT happen.** No backfill. Writing `Some(false)` into existing vaults — or defaulting the
leaves to `false` in the struct — would be btctax answering a Form 8615 condition on the filer's
behalf, which is FR-29's own defect committed a second time, and the *an entry is testimony* rule
forbids it. There is no migration script.

**What happens instead**, for a vault with more than the threshold of unearned income, no date of
birth (or one under 24), and a non-joint status: `btctax report` / `export` return
`ProfileOutcome::Uncomputable` (`crates/btctax-cli/src/resolve.rs:188-205`) carrying
`Form8615AgeSupportUnanswered`'s detail, which names the missing answer and the remedy. **The old
behaviour is not silently kept** — it cannot be, since the gate no longer reads the flag it used.

The filer's exits, all of them cheap:

1. `btctax income answer` and answer condition 3 (one keystroke, and if No, nothing else is asked); or
2. give a date of birth at the same prompt (`SkippableId::DobTaxpayer`, `questions.rs:1003-1013`),
   which suppresses both questions permanently if it shows 24 or older at year end; or
3. answer both YES and receive `KiddieTax`, which is the correct outcome — Form 8615 is required and
   btctax does not fill it.

**Fixtures are migrated by making them realistic, not by silencing the gate.** The 15 CLI fixtures in
§10 are adult filers; give them a date of birth, which both fixes them and exercises §5.1's
suppression path. Reserve explicit `Some(false)` answers for fixtures that are *about* a young filer.
**Do not add the answers to the shared builders in `crates/btctax-core/src/tax/testonly.rs`** — a
blanket `Some(false)` there would green every future fixture by default and destroy the gate's
ability to catch this class again.

---

## 8. Refusals — exact wording and firing conditions

Two new `RefuseReason` variants (`return_refuse.rs`) plus a reworded `KiddieTax`. Adding a variant reds
the exhaustive cross-crate match in `crates/btctax-input-form/src/attribute.rs` — the free, exact blast
radius this repo prefers.

### R-1 `Form8615AgeSupportUnanswered`

**Fires:** condition 1 ∧ condition 5 ∧ ¬`provably_24_or_older` ∧ `form8615_condition3_age_support ==
None`.

**Detail** (`format!`ed at the gate; `{year}` and `{threshold}` are in scope):
```text
your unearned income is over the §1(g) threshold of ${threshold}, so Form 8615 may be required —
and its condition 3 asks whether, at the end of {year}, you were (a) under age 18, (b) age 18 and
didn’t have earned income that was more than half of your support, or (c) a full-time student at
least age 19 but under age 24 and didn’t have earned income that was more than half of your
support. btctax will not answer that for you: §1(g)(1) takes the GREATER of your own rate and the
parent's-rate figure, so a wrong "no" can only understate your tax. Run `btctax income answer` —
answering "no" clears this, and so does entering your date of birth there if you were 24 or older
at the end of {year}.
```

**Anchor** (`attribute.rs`): `vec![skip(SkippableId::Form8615Condition3AgeSupport)]`, matching the
`CooperativePatronUnanswered` precedent at `attribute.rs:70-72`.

### R-2 `Form8615ParentAliveUnanswered`

**Fires:** the R-1 conditions except `form8615_condition3_age_support == Some(true)` ∧
`form8615_condition4_parent_alive == None`.

**Detail:**
```text
you answered YES to Form 8615's condition 3, so whether Form 8615 is required now turns on its
condition 4 — "At least one of your parents was alive at the end of {year}." btctax will not answer
that for you; a wrong "no" understates your tax by the whole §1(g) difference. Run
`btctax income answer`.
```

**Anchor:** `vec![skip(SkippableId::Form8615Condition4ParentAlive)]`.

### R-3 `KiddieTax` — kept, reworded

**Fires:** all five conditions (ladder step 6).

**Detail** (replacing `return_1040.rs:1000`, which today says *"a claimable-as-dependent filer …"*):
```text
you meet all five of Form 8615's conditions for {year}: more than ${threshold} of unearned income, a
filing requirement, the condition-3 age and earned-income-support test, at least one parent alive at
the end of the year, and a return that is not joint. §1(g) then taxes part of your unearned income
at your parent's rate — Form 8615 computes the greater of that and your own — and btctax does not
fill Form 8615, because it needs your parent's taxable income and any siblings' §1(g) amounts, which
btctax never sees. Complete Form 8615 by hand.
```

**Doc comment** at `return_refuse.rs:263-273`: the FR-29 warning block is **deleted and replaced** with
the corrected reading and a pointer to this spec. Leaving the warning in place after the fix would be
its own defect — a doc telling a future maintainer the gate is wrong when it is right.

**Anchor:** unchanged (`attribute.rs:221-223`, `NotInForm`) — the note's wording is updated to say the
screen is computed at `report` from the two condition declarations. `KiddieTax` remains a refusal no
input can clear (like `CooperativePatron`), so it keeps its single `NotInForm` anchor and the existing
`deferred_and_defensive_refusals_are_not_in_form` test (`attribute.rs:429-452`) stays green unchanged.

---

## 9. How it is tested

Every guarantee below names the mutation that must make it RED (harness rule **B1**, `CLAUDE.md`).
"Red" means the named test fails, not that some test somewhere fails.

**G1 — dependency is not a condition.** `form8615_screens_a_filer_who_is_nobody's_dependent`: a filer
with `can_be_claimed_as_dependent_taxpayer = Some(false)`, unearned above the threshold, conditions 3
and 4 = `Some(true)` ⇒ `KiddieTax`.
**Mutation:** re-introduce `if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false)` around
the gate ⇒ red.
★ **This is the INVERSION of the assertion at `return_1040.rs:4473-4475`** — the one FR-29 flagged as
*"kept RED-ADJACENT on purpose"*. That assertion and the FR-29 comment block above it
(`:4465-4472`) are **replaced**, not deleted, so `git diff` shows a claim being corrected rather than
a test disappearing.

**G2 — silence never proceeds.** `form8615_unknown_conditions_fail_closed`: a table over the six rows
of §6.1, asserting the exact outcome of each.
**Mutation:** change either `None` arm of the ladder to proceed ⇒ red on that row. Change
`c3 == Some(false)` to `c3 != Some(true)` (the classic widening) ⇒ red on the `None` row.

**G3 — the age arithmetic is the form's.** `considered_age_matches_i1040gi_january_first_examples`:
the three printed examples for TY2025 — born `2008-01-01` ⇒ 18, `2007-01-01` ⇒ 19, `2002-01-01` ⇒ 24 —
plus the neighbours `2002-01-02` ⇒ 23 (asked) and `2002-01-01` ⇒ 24 (never asked, never refused), plus
a leap-day vector `2004-02-29` ⇒ 21.
**Mutations:** drop the `January && day == 1` term ⇒ the `2002-01-01` row flips to 23 and the filer is
asked ⇒ red. `>= 24` → `> 24` ⇒ the `2002-01-01` row is asked ⇒ red. `year - dob.year()` →
`year - dob.year() - 1` ⇒ every row shifts ⇒ red.

**G4 — condition 5.** `a_joint_return_is_never_screened_for_form_8615`: MFJ with everything else true
⇒ no refusal.
**Mutation:** delete the `!= Mfj` term ⇒ red.

**G5 — condition 1's strictness and its source.** `the_threshold_is_strict_and_comes_from_the_params`:
unearned exactly equal to `params.kiddie_unearned_threshold` ⇒ proceed; one cent more ⇒ refuse; and the
same fixture under a params value moved by $100 moves the boundary with it.
**Mutations:** `>` → `>=` ⇒ red. A literal `dec!(2600)` in place of the param ⇒ red on the third
assertion.

**G6 — the named over-refusal branch of condition 2** (§5.4).
`condition_two_is_assumed_and_this_is_the_filer_it_over_refuses`: a non-dependent single filer, gross
income below Chart A's threshold, conditions 1/3/4/5 true ⇒ `KiddieTax`, with the assertion message
stating that the form would **not** require Form 8615 here and citing `i1040gi:634-639`. A second
vector does the same for the blind dependent of `:705`.
This test exists to make the assumption **visible and deliberate**; it is the KAT `CLAUDE.md` requires
alongside a derived form's equivalence proof.
**Mutation:** implement condition 2 without updating this test ⇒ red, and the test is then rewritten
rather than deleted.

**G7 — prompt conformance against the extract.** A new `cargo run -p xtask -- prompt-check`, beside
`cite_check` (`crates/xtask/src/cite_check.rs`), owning a table of
`(SkippableId, &[&str])` clauses and asserting, for each clause, **both**:
(a) it appears verbatim (normalised) in `design/forms/extract/i1040gi--2025.txt`, and
(b) it appears verbatim in that question's `prompt`.
Clauses for condition 3: `"under age 18"`, `"Age 18"`, `"didn’t have earned income that was more than
half of your support"`, `"A full-time student at least age 19 but under age 24"`. For condition 4:
`"At least one of your parents was alive"`.
**Why both halves.** (b) alone lets the clause table drift from the form; (a) alone lets the prompt
drift from the table. Together, a paraphrase anywhere reds — the property B1 calls *"cannot be
satisfied performatively"*.
**B1 pairing (mandatory):** `prompt_check_rejects_a_paraphrased_prompt_and_accepts_the_real_one`,
modelled on `cite_check.rs::a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`, planting
*"more than half of your support"* → *"most of your support"* and asserting red.

**G8 — the demanded set is inside the live set (the anti-brick invariant).**
`every_form8615_refusal_names_a_question_the_interview_would_have_offered`: over a fixture grid that
crosses {no DOB, DOB 20, DOB 30} × {Single, MFJ, MFS} × {unearned from wages only, from interest only,
**from crypto only**} × {both answers `None`}, assert that whenever the gate returns R-1 or R-2, the
corresponding `SkippableQuestion::live` is `true` for that same `ReturnInputs`.
**Mutation:** add any income-dependent term to either liveness predicate ⇒ red on the crypto-only
fixture. This is the test that would have caught the shipped circular-liveness bug of
`questions.rs:344-345`, and it is the reason §3.1 chose this shape.

**G9 — registry bookkeeping.** The existing counts move and must be *moved*, not deleted:
`SKIPPABLE_QUESTIONS.len()` 16 → 18 (`questions.rs:1490-1495`); the `Skippables` section list in
`crates/btctax-input-form/src/spec/mod.rs:305-325`; the coverage KAT's `93` distinctly-covered leaves
(`crates/btctax-input-form/src/spec/coverage.rs:435`) and its two new `(FieldId, path)` rows
(`coverage.rs:578-582` for the idiom). `QuestionId::ALL`'s 17 is **unchanged** — no declaration is
added.

**G10 — the corpus floor holds.** `crates/btctax-oracle-harness/tests/smoke.rs:160-172` already
asserts `admitted >= 11` and `refused == []`. §10 measured four households newly refused. **Those
fixtures get dates of birth; the assertions are not relaxed.** A weakened floor here would hide
exactly the regression the floor exists to catch.

---

## 10. Blast radius — measured, not estimated

Measured by copying the tracked tree to an isolated build directory, applying the §6 gate in its
migration state (both answers `None`, which is every existing vault and every existing fixture), and
diffing the failing-test sets:

```
git ls-files -z | tar --null -T - -cf - | tar -xf - -C /scratch/tmp/fr29-blast
# gate at return_1040.rs:989 replaced with: filing_status != Mfj && !provably_24_or_older
cargo nextest run --workspace --no-fail-fast
```

| | tests | passed | failed |
|---|---|---|---|
| baseline (unpatched copy) | 2779 | 2775 | 4 |
| with the new gate | 2779 | 2757 | 22 |

The 4 baseline failures are copy-only (`repo_hygiene` ×2 and `harness_check` ×2 — git hooks and
packaging checks that need the real working tree) and are identical in both runs.

**18 tests newly fail.** In full:

| crate / binary | count | tests |
|---|---|---|
| `btctax-cli::tax_report` | 9 | `a_gift_over_its_ceiling_prints_its_charitable_carryover_out_in_the_report`, `a_gift_within_its_ceiling_prints_no_charitable_carryover_line`, `carryover_write_back_round_trips_and_respects_user_precedence`, `dual_report_renders_absolute_return_with_section_6_labels`, `a_computed_capital_loss_stamp_survives_every_command_that_should_retract_it`, `import_preserves_a_computed_carryover`, `full_return_report_surfaces_conservative_omission_advisories`, `the_full_remedy_chain_restores_a_computed_carryover`, `the_summary_does_not_claim_a_capital_loss_write_the_gate_skipped` |
| `btctax-cli::promote_cli` | 4 | `characterization_full_return_export_pins_the_shipped_file_set_and_report`, `export_full_return_with_an_overflowing_part_ii_narrative_refuses_with_a_named_remedy`, `export_full_return_writes_form_8275_txt_by_name`, `promoted_export_with_more_than_6_legs_refuses_cleanly_not_panics` |
| `btctax-cli::experimental_notice` | 2 | `full_return_export_notice_absent_from_every_file_in_the_export_directory`, `full_return_export_notice_reaches_stderr_not_stdout` |
| `btctax-core` | 2 | `tax::return_1040::tests::kiddie_tax_refuses_dependent_over_threshold`, `tax::return_1040::tests::business_interest_income_refuses` |
| `btctax-oracle-harness::smoke` | 1 | `check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells` |

**Shape of the 15 CLI failures.** Every one is the same thing — the year becomes uncomputable:

```
Usage("tax year 2024 cannot be computed from its full-return inputs: … needs Form 8615 …")
```

They are adult-filer fixtures with no date of birth and capital gains over the threshold, so §7's
migration (give the fixture a DOB) fixes all 15 and exercises §5.1 while doing it.

**`kiddie_tax_refuses_dependent_over_threshold`** fails at exactly the assertion FR-29 flagged
(`return_1040.rs:4473-4475`; the copy reported line 4483, +9 for the patch). That is the measurement
confirming the defect is the one being fixed, and G1 is its replacement.
**`business_interest_income_refuses`** fails at `return_1040.rs:4404` (the copy reported 4413) — the `$5,000` *hobby* interest
half of the test, which is unearned income over the threshold on a fixture with no DOB. Under §6 it
presents as R-1 rather than `KiddieTax`; the fixture needs a DOB or a condition-3 answer.

**The oracle corpus** names its casualties precisely: `single_w2_plus_crypto_ltcg`,
`single_qdcgt_both_slices`, `single_short_term_crypto_gain`,
`single_miner_qbi_limited_by_net_capital_gain` — four of the households the make-check sweep runs,
which `smoke.rs:193-196` describes as *"the twelve anchors + the two pinned cells"*. The `admitted >= 11`
floor still held in the patched run.
**P-2 (plan task):** the full corpus of ~104 households runs under `#[ignore]` (`smoke.rs:203`) and is
**not** covered by this measurement — run it before the phase closes.

**One known under-count.** The measurement patch raises `KiddieTax` where §6 raises R-1/R-2, so tests
that *expect* `KiddieTax` stayed green in the patched run and will need editing anyway. There is
exactly one such test — `kiddie_tax_refuses_dependent_over_threshold`, already counted above (its three
`Some(RefuseReason::KiddieTax)` assertions at `:4455`, `:4463`, `:4485`). The 18 is therefore complete
as a *test* count.

---

## 11. Out of scope, and what is filed instead

- **Filling Form 8615.** Needs the parent's taxable income and every sibling's §1(g) amount. R-3 says
  so to the filer.
- **§59(j) kiddie-AMT** (the AMT exemption cap for a child, which Tax-Calculator *does* model at
  `calcfunctions.py:2431`). A separate provision on a separate form; note it as a follow-up under the
  §G-6 AMT track rather than folding it in here.
- **A date of birth in the future, or after the tax year.** `considered_age_at_year_end` returns a
  negative age, `provably_24_or_older` is false, and the filer is asked — fail-closed, so it is a
  Minor. File it; do not gate on it.

---

## 12. Open questions for review

**OQ-1 (Minor).** §6.2 classifies both leaves `Class::NoTaxDirection` following the `qbi_w2_wages`
precedent, but that variant is documented as *"a lawful silent default"* (`classifier.rs:50-51`) and
these are not lawful silent defaults — they refuse. Three leaves now share the *refused where needed, unread
where not* pattern. Is that a fourth `Class` variant, or is the reason string enough?

**OQ-2 (Important if answered "collect it").** §5.4 assumes condition 2. Should Charts A, B and C be
transcribed now — making condition 2 computed and closing the two named over-refusal branches — or
does that belong in its own cycle? Transcribing them is a table plus a worksheet, and the charts are
already in the extract of record.

**OQ-3.** §4.1 makes the prompt year-free (*"at the end of the tax year"*) because `prompt` is
`&'static str`. The refusal details, which *are* `format!`ed, carry the year. Is the split acceptable,
or should `SkippableQuestion::prompt` gain a year-aware form? (Precedent for year-free prompts:
every entry in both registries today.)

**OQ-4.** Condition 3 is one leaf carrying a three-way disjunction (§4). The alternative — three
leaves (age band, full-time student, support test), with the disjunction computed — would let a known
date of birth answer part of it and would make each sub-question simpler. It would also mean btctax
re-deriving a disjunction the form writes out, which is the compression `CLAUDE.md` forbids. Confirm
the choice, or overturn it with the derivation written down.
