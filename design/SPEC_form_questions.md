# SPEC — P9: the FORM QUESTION REGISTRY

*Status: **r2**. Folds Fable's spec review r1 (0C/6I/5M — `reviews/P9-SPEC-fable-r1.md`), the statutory
taxonomy the owner asked for, and the owner's advisory mandate.*
*Origin: `design/full-return/reviews/ARCH-P9-fable-question-registry.md`.*
*Subsumes the two open P8a findings, I1 and I3 (`reviews/P8a-fable-r1.md`), which are deliberately unpatched.*

---

## 1. The defect this closes

**"An input with no safe default must be answered before it reaches compute or print" is a load-bearing
invariant of this system, and it is the only load-bearing invariant in the codebase still enforced by
convention instead of construction.**

Every *other* load-bearing invariant here is held structurally:

| invariant | how it is held |
|---|---|
| no negative money | `first_negative_amount` destructures with **no `..`** — a new field is a compile error |
| the packet is complete | `PrintedReturn` destructured with no `..` |
| the §170 rounding regime | the `Printed8283Rows` newtype |
| an SSN is well-formed | `Ssn`'s field is private; `canonical()` is the only constructor |
| **an answerable input was answered** | **nothing. Five hand-wired obligations across four crates.** |

Adding one yes/no field today requires independently remembering to: declare it `Option<bool>`; hand-write
an "unanswered" refusal; get that refusal's **scope** right; make every consumer read it safely; write a
test that the refusal **fires**; and add it to `income answer` — *or the year is bricked with no way to
answer it.* **Each has been forgotten at least once**, and the ledger is not ambiguous: D-8 (step 1 — shipped
in v0.2.0, understating tax and printing an unaffirmed checkbox on a filed 1040); SPEC r1 I7, **day one of
the program** (step 1, `foreign_accounts: bool` auto-checking Schedule B 7a "No"); P8a I1 (step 3); P8a I2 +
§199A + `ScheduleCNoBusinessDescription` (step 5 — guards shipped with **zero** tests); P8a I3 (step 4).

This class is **~21% of every blocking finding in the program** — the largest single class, and the only one
that recurred at *every* stage with identical mechanics.

### 1.1 What this spec does NOT claim

- The review volume is **not** mostly this defect. ~60% of blocking findings are the honest price of the IRC
  under a 0C/0I gate (wrong tax law ~20%, owning 5 of 11 Criticals). A third of the review *artifacts* found
  nothing — they are the re-verification rounds our own workflow mandates.
- **The engine's architecture is sound** and is not touched here.
- **★ THE TERNARY IS NOT THE FIX, AND IS ALREADY IMPLEMENTED.** All five current questions are `Option<bool>`
  *today*. Anyone who reads this spec and concludes "so we need a tri-state" has learned **nothing**. The gap
  is that nothing *forces* it, and the obligations are hand-wired.

---

## 2. ★ The statutory test — what makes a default lawful

*(New in r2. This replaces the engineering criterion of r1 ("absence must be conservative **and** advised"),
which Fable correctly rejected (M-4) as not fitting the fields it had to classify. **Conservatism is not the
test — it is a consequence.** The test is statutory, and it is the taxpayer's signature that decides it.)*

**26 U.S.C. §6065** — every return *"shall contain or be verified by a written declaration that it is made
under the penalties of perjury."* **26 U.S.C. §7206(1)** makes it a felony to willfully subscribe a return
*"which he does not believe to be true and correct **as to every material matter**."*

*Every material matter.* **The filer cannot believe true an answer they were never asked, and software
cannot supply belief on their behalf.**

Against that: **deductions are "a matter of legislative grace"**, and *"a taxpayer seeking a deduction must
be able to point to an applicable statute and show that he comes within its terms"* — *New Colonial Ice Co.
v. Helvering*, 292 U.S. 435, 440 (1934). The burden to **claim** is the taxpayer's. A filer who never says
"I am blind" simply does not get the §63(f) addition. **Nothing false has been stated; a benefit was
forgone.** (This is also why "blank = zero" is safe on a *deduction* line and lethal on an *income* line —
one doctrine, both directions.)

So the question that classifies every field is **not** "is `false` conservative?" It is:

> ### Does an unanswered box make the filer ASSERT something, or merely FORGO something?
>
> **Assert ⇒ there is no lawful default.** **Forgo ⇒ `false` is what the statute already assumes.**

| class | statutory basis | default | on absence |
|---|---|---|---|
| **(A) DECLARATION** | §6065 + §7206(1) — signed true as to *every material matter* | **none exists** | **REFUSE** |
| **(B) BENEFIT CLAIM** | *New Colonial Ice* — burden to claim is the taxpayer's | **`false` is lawful** | **grant nothing, and ADVISE** (§2.1) |
| **(C) NO TAX DIRECTION** | neither claims a benefit nor asserts a fact | **`false` is lawful** | silent |
| **(D) DEAD** | — | **FORBIDDEN** | **consume it or delete it** (§2.2) |

**Why (A) is not merely "conservative-in-reverse".** An unchecked *dependent* box is an assertion — *"No,
nobody can claim me"* — made under penalty of perjury by someone who was never asked. An unchecked
*presidential-fund* box asserts nothing; it is the absence of an opt-in, which is true of anyone who did not
opt in. That difference is the whole taxonomy.

**And it is why Schedule B 7a was the day-one finding.** Auto-answering *"do you have a foreign account?"*
with "No" is not a missed deduction. It is a **false declaration on the exact question the government uses
to establish willfulness** for FBAR purposes. Of the thirteen booleans in the input model, it is the worst
one to guess — and it is the first one we guessed.

### 2.1 ★ OWNER MANDATE — a forgone benefit must never be silent

> *"Let's not forgo a benefit without informing user they may be giving away more money than required."*
> — the owner, 2026-07-14

This makes class **(B)** a **two-part** rule, and the second part is **not optional**:

> **A `false` default that forgoes a benefit is lawful ONLY IF the filer is told they forwent it.**

We may not *refuse* on a benefit claim — *New Colonial Ice* puts the burden to claim on the taxpayer, and a
return is perfectly valid without it. But silence overtaxes someone and lets them believe the number was
theirs. So every class-(B) field gets **both**:

1. a **skippable** prompt in `income answer` (never mandatory — it must not brick a valid return); **and**
2. a **mandatory advisory** when the benefit is forgone, naming the money.

**The pattern already exists in this codebase and was simply never generalized:** `date_of_birth` is
skippable, and its absence fires `Advisory::AgedBoxForfeitedNoDob { per_box }`, which names the dollars. That
*is* the class-(B) contract. Two fields that should have it do not:

| conservative default | benefit forgone | advised today? |
|---|---|---|
| missing DOB → no §63(f) aged box | $1,950 unmarried / $1,550 per box (TY2024) | ✅ `AgedBoxForfeitedNoDob` |
| **`Person.blind = false`** | **the SAME §63(f) box — same statute, same worksheet line, same dollars** | ❌ **SILENT** |
| **`ScheduleAInputs.salt_use_sales_tax = false`** | **the entire §164(b)(5) sales-tax deduction** | ❌ **SILENT** |

The `blind` gap is near-absurd on inspection: §63(f) grants the addition for being 65-or-older **or** blind.
We tell the filer when they forfeit it for want of a *birthday* and say nothing when they forfeit it for want
of a *checkbox*.

The `salt` gap may be larger in dollars: a filer in a state with **no income tax** (TX/FL/WA/NV/…) has an
income-tax deduction of roughly **zero**, and the §164(b)(5) election to deduct *sales* tax instead is the
entire point of that line. Default `false` silently takes the zero. (We *do* refuse a sales-tax **amount**
entered with the election off — `SaltSalesTaxWithoutElection` — but a filer who never knew the election
existed gets nothing, and hears nothing.)

⇒ **Two new advisories are REQUIRED by this spec**: `BlindBoxForfeitedNotDeclared { per_box, persons }` and
`SalesTaxElectionNotMade`. Acceptance (§4) gates on them.

### 2.2 Class (D) — DEAD fields are FORBIDDEN, not exempt

A captured-but-unconsumed field is a lie about what the app honors: the user types it, we ignore it. Worse,
it is **pre-armed**. *This is exactly how D-8 happened* — `can_be_claimed_as_dependent_taxpayer` sat inert as
a defaulting `bool` until someone wired it to the standard deduction, and **at that instant the default
silently became an answer.**

Three fields are dead **today** (verified: zero consuming sites):

| field | what it will gate | direction when it goes live |
|---|---|---|
| `Person.ssn_valid_for_employment` | §32 EITC (an SSN not valid for employment disqualifies) | `false` denies the credit ⇒ **overstates** — safe |
| `Dependent.ssn_valid_for_employment` | §32 EITC / §24 CTC | as above |
| **`W2.box13_retirement_plan`** | **§219(g) IRA-deduction phase-out** | **`false` means "not covered by a plan" ⇒ NO phase-out ⇒ OVERSTATES the IRA deduction ⇒ ★ UNDERSTATES TAX** |

**`box13_retirement_plan` is a pre-armed D-8 pointing the dangerous way**, and it is currently filed nowhere.
Its sole "use" is `box13_retirement_plan: _` inside a destructure. `ssn_valid_for_employment` at least
pre-arms in the safe direction; box 13 does not.

**Resolution (this phase): DELETE all three.** They are not honored, nothing reads them, no test covers them,
and `income import` accepting them tells the filer a falsehood. When §32 or §219(g) is actually implemented,
the field returns **as a classified registry entry or a written exemption** — which the classifier (§3.3)
will then force. Deleting is a breaking change to the TOML surface; **there are no users** ([[no-users-yet]]),
so the cost is zero.

### 2.3 The census — every boolean in the input model, classified

*Verified against source. 13 fields.*

**(A) DECLARATION — registry questions, refuse when unanswered:**

| field | today | note |
|---|---|---|
| `HouseholdHeader.can_be_claimed_as_dependent_taxpayer` | `Option<bool>` ✅ | D-8 |
| `HouseholdHeader.can_be_claimed_as_dependent_spouse` | `Option<bool>` ✅ | D-8; predicate corrected (§3.1) |
| `ReturnInputs.mfs_spouse_itemizes` | `Option<bool>` ✅ | §63(c)(6) |
| `ReturnInputs.foreign_accounts` | `Option<bool>` ✅ | Sch B 7a — **the day-one finding** |
| `ReturnInputs.foreign_trust` | `Option<bool>` ✅ | Sch B 8 |
| **`Schedule1Inputs.hsa_present`** | **bare `bool` ⚠️ RECLASSIFIED** | **§2.4** |
| **`ReturnInputs.dual_status_alien`** | **DOES NOT EXIST ⚠️ NEW** | **§2.5** |

**(B) BENEFIT CLAIM — `false` lawful; skippable prompt + MANDATORY advisory:**

| field | benefit | advisory |
|---|---|---|
| `Person.blind` | §63(f) additional std deduction | **NEW — required** |
| `ScheduleAInputs.salt_use_sales_tax` | §164(b)(5) sales-tax deduction | **NEW — required** |
| *(`Person.date_of_birth` — not a bool, but the same class and the model for it)* | §63(f) aged box | ✅ exists |

**(C) NO TAX DIRECTION — silent default lawful:**

| field | why |
|---|---|
| `HouseholdHeader.presidential_fund_taxpayer` | the $3 Presidential Election Campaign Fund designation changes **neither tax nor refund** by statute. Unchecked = *did not opt in*, which is TRUE of anyone who did not. Not an assertion. |
| `HouseholdHeader.presidential_fund_spouse` | as above |

**(D) DEAD — DELETE (§2.2):** `Person.ssn_valid_for_employment`, `Dependent.ssn_valid_for_employment`,
`W2.box13_retirement_plan`.

### 2.4 `hsa_present` is a DECLARATION, not an exemption — RECLASSIFIED

`hsa_present = true` **refuses** (an HSA needs Form 8889, out of scope for v1). It is a bare `bool`
defaulting to `false`, and **nobody is ever asked**.

Its false default is **not** a forgone benefit — it is a **fail-open on a refusal**. A filer who has an HSA
and was never asked gets a return that **omits Form 8889**, and then signs, under §6065, that the return is
*"true, correct, and **complete**."* **An omitted required form is a completeness failure, not an
overpayment.** That is class (A), and it is the identical bare-`bool` shape as D-8.

*Partial backstop, recorded honestly:* employer-routed HSA contributions surface as W-2 box 12 code **W**,
which is outside `INERT_BOX12_CODES` and therefore refuses by another path. That backstop does **not** cover
a **direct** personal HSA contribution, which has no box-12 trace. So the hole is real, merely narrower than
it first appears.

⇒ `hsa_present` becomes `Option<bool>` and a registry question (`HsaPresentUnanswered`).

### 2.5 The dual-status alien — a declaration we already print, with NO field behind it (Fable I-5)

The 1040's third header checkbox reads: **"Spouse itemizes on a separate return **or you were a dual-status
alien**."** We print it from the MFS coupling alone (`packet.rs:325`). **The dual-status half is silently
answered "No" for every filer we have ever printed** — no field, no question, no refusal, no `LIMITATIONS`
entry.

It is squarely class (A): *"you **were** a dual-status alien"* is a statement about the filer, asserted under
penalty of perjury. And it is not idle: a dual-status alien **may not take the standard deduction**
(§63(c)(6)(B)), while our `ItemizeElection::Auto` grants it — **a silent understatement plus an unaffirmed
checkbox**, for a population (first-year visa holders with W-2s and crypto) squarely inside this app's
archetype.

⇒ **NEW field** `ReturnInputs.dual_status_alien: Option<bool>`, a registry question
(`DualStatusAlienUnanswered`), live on every return. `Some(true)` ⇒ a **value-dependent refusal**
(`DualStatusAlienUnsupported` — §63(c)(6)(B) bars the standard deduction and §1 rate application is
out of scope for v1). Being a new `Option<bool>`, `None` on an existing row is **honest** — no migration.

### 2.6 The class boundary is wider than `bool` — defaulted ENUMS have the same disease

A bool-only net misses these, and the classifier (§3.3) must cover them:

| field | default | why it is the same defect |
|---|---|---|
| `ScheduleCInputs.accounting_method` | `Cash` | `"accrual"` is accepted, unmodeled, **unrefused**, and flips the **printed** Sch C line F on a cash-basis return — the form asserts a method the numbers behind it do not use. *(already filed → P8)* |
| `W2.owner` / `ScheduleCInputs.owner` | `#[default] Taxpayer` | silently attributes a **spouse's** W-2 to the filer on MFJ, touching per-person §31(b) excess-Social-Security credit and §402(g) deferral buckets. **The §31(b) direction can UNDERSTATE.** |
| `ReturnInputs.itemize_election` | `Auto` | interacts with §2.5 (a dual-status alien barred from the std deduction) |

⇒ The classifier destructures these too, and each must be **registered or exempted with a written reason**.
The `owner` case is filed with an owning phase (it is not P9 scope to re-model ownership), but it must appear
in the exemption register with its criterion stated, not be silently absent.

---

## 3. The design

### 3.1 The registry

```rust
// crates/btctax-core/src/tax/questions.rs   (NEW)

/// A yes/no the return asks that is a DECLARATION (§2, class A) — the filer ASSERTS it under §6065's
/// penalties-of-perjury jurat, so there is NO lawful default and an unanswered one must REFUSE.
///
/// ONE entry per question. The prompt, the refusal, the refusal DETAIL, the liveness scope, and the
/// accessors live here and NOWHERE ELSE. `screen_inputs`, `income answer`, and `ReturnHeader::build`
/// DERIVE from this list; they do not restate it. Restating it is what let the refusal scope and the
/// prompt scope disagree (P8a I1), and what would let a question be refusable but unaskable (a brick).
pub struct FormQuestion {
    pub id: QuestionId,
    /// Form-phrased — the filer answers a 1040 line, not a struct field.
    pub prompt: &'static str,
    pub unanswered: RefuseReason,
    /// ★ The FULL refusal detail (Fable I-1). NOT derived from `prompt`: today's texts carry the statutory
    /// cite and, for D-8, the REMEDY (`run btctax income answer`) — which a named acceptance test asserts,
    /// and which the project's own doctrine requires ("a refusal with no exit is just a brick with better
    /// prose"). A prompt-derived text would drop both.
    pub unanswered_detail: &'static str,
    /// ★ THE liveness predicate — the ONLY copy in the codebase.
    pub live: fn(&ReturnInputs) -> bool,
    pub get: fn(&ReturnInputs) -> Option<bool>,
    pub set: fn(&mut ReturnInputs, bool),
}

/// ★ EXHAUSTIVE, and that exhaustiveness is the completeness anchor (Fable I-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionId {
    DependentTaxpayer,
    DependentSpouse,
    MfsSpouseItemizes,
    ForeignAccounts,
    ForeignTrust,
    HsaPresent,        // §2.4
    DualStatusAlien,   // §2.5
}

impl QuestionId {
    /// Every variant, written out. A `match` in the test (§3.5) makes a NEW variant a compile error until
    /// it is listed here — which is what makes "drop an entry from FORM_QUESTIONS → a named test fails"
    /// actually TRUE rather than vacuous.
    pub const ALL: &'static [QuestionId] = &[ /* all 7 */ ];
}

pub const FORM_QUESTIONS: &[FormQuestion] = &[ /* 7 entries */ ];
```

*(Fable verified by live `cargo check` that this shape compiles: fieldless variants of a `String`-carrying
enum are const-constructible, the slice promotes, and closures coerce to `fn` pointers.)*

**Liveness predicates** (each the only copy):

| question | `live` |
|---|---|
| `DependentTaxpayer` | always |
| `DependentSpouse` | **`filing_status == Mfj \|\| header.spouse.is_some()`** ← this correction **IS** P8a I1 |
| `MfsSpouseItemizes` | `filing_status == Mfs` |
| `ForeignAccounts` / `ForeignTrust` | `schedule_b_files(ri)` |
| `HsaPresent` | always |
| `DualStatusAlien` | always |

### 3.2 The three derivations

**`screen_inputs`.** The hand-written unanswered-blocks collapse to one loop, placed **after** the integrity
gates (negative money, malformed SSN) and before the value-dependent rules (Fable M-2):

```rust
for q in FORM_QUESTIONS {
    if (q.live)(ri) && (q.get)(ri).is_none() {
        return refuse(q.unanswered.clone(), q.unanswered_detail);
    }
}
```

**Refusal precedence is explicitly NOT contract** (M-2): on a multi-defect return the loop may report a
different reason than today (e.g. `foreign_trust = Some(true)` + unanswered 7a). Any test asserting a
*specific* reason on a multi-defect fixture must be made single-defect.

**Value-dependent refusals stay hand-written** and are NOT registry business: `ForeignTrust == Some(true)` →
Form 3520; `DependentSpouseUnsupported`; `DualStatusAlienUnsupported`; `HsaPresent == Some(true)`. These are
**domain rules about the answer**, not about *whether there is* an answer. (Fable verified the two kinds
cannot double-report: `None` and `Some(true)` are disjoint states.)

**`income answer`.** `live_questions`/`current_bool`/`set_bool` become registry iteration. The `Question`
enum **shrinks to the DOB residue** — it does not delete (Fable M-1), because DOBs are skippable dates, not
yes/no declarations. The no-brick property (*everything the screen can refuse for is askable*) becomes true
**by identity**.

`answer` also gains the class-(B) **skippable** prompts (§2.1): `blind` (taxpayer + spouse) and
`salt_use_sales_tax`. Skippable — empty input leaves them `false`, which is lawful; the advisory then fires.
**The spouse-DOB prompt stays gated on `header.spouse.is_some()`** even though the spouse *question* widens
to MFJ (§3.1), because `set_date` silently discards a spouse DOB when there is no spouse `Person` — asking it
would swallow a typed answer (Fable's own caveat).

**`ReturnHeader::build` — the print boundary** (P8a I3). It gains a **new error type** (Fable I-2), because
`SsnError` cannot represent this and the caller currently maps every build error to *"fix the identity"*:

```rust
pub enum HeaderError {
    Ssn(SsnError),
    /// A live DECLARATION is unanswered. At PRINT there is no conservative direction — an unchecked box is
    /// a false "No" and a checked box is a false "Yes" — so refusal is the ONLY fail-closed behaviour.
    Unanswered(QuestionId),
}
```

`Display` for `Unanswered` names the question and the remedy (`btctax income answer`), consistent with I-1.
`admin.rs`'s caller mapping is updated so an unanswered flag is not reported as an SSN problem.

**This closes a second print site Fable found that P8a I3 never mentioned:** `printed.rs:936/:943` project
Schedule B Part III with `unwrap_or(false)` — *the exact idiom D-8 names as "the very shape of this defect"*.
Because `ForeignAccounts`/`ForeignTrust` are registry questions live exactly when Schedule B prints, a build
refusal over **all** live questions closes that site too.

### 3.3 The classifier — the part that stops the NEXT D-8

A function destructuring the input model with **no `..` and NO `_` arm**, in which every `bool` /
`Option<bool>` / **defaulted enum** must be classified as a registry question (A), or **exempted** on a named
register carrying its **class and its statutory reason** (B/C per §2).

**★ It must RECURSE (Fable I-3).** r1 covered only `ReturnInputs` + `HouseholdHeader` — but *most of the
fields needing classification are nested*: `Person.blind`, `Person.ssn_valid_for_employment`,
`W2.box13_retirement_plan`, `W2.owner`, `ScheduleAInputs.salt_use_sales_tax`, `Schedule1Inputs.hsa_present`,
`ScheduleCInputs.accounting_method`. A top-two-level classifier classifies **none of them**, and the
guarantee would again be narrower than advertised — *the exact sin this spec charges to
`first_negative_amount`.* It must recurse at least as deep as `first_negative_amount` already does
(`W2`, `Person`, `Dependent`, `ScheduleAInputs`, `Schedule1Inputs`, `ScheduleCInputs`, `Payments`,
`QbiInputs`).

⚠️ **Honest limit, recorded so it is not oversold:** the classifier's force is
"compile-error-until-a-human-**decides**", not until-**correct**. `first_negative_amount` already demonstrates
the escape hatch — `header: _, // PII only — no money` waves off the *entire* `HouseholdHeader`, so that
struct is **not** exhaustively destructured and the guarantee its own doc-comment promises has a hole.
**Wildcard arms are the residual convention.** Hence: **no `_` arm**, and the open `header: _` follow-up is
fixed here.

### 3.4 The advisories the owner mandate requires (§2.1)

```rust
/// §63(f): a person did not declare blindness, so the additional standard deduction was not granted.
/// The SAME statute, worksheet line, and dollar amount as `AgedBoxForfeitedNoDob` — which we DO advise.
BlindBoxForfeitedNotDeclared { per_box: Usd, persons: usize },
/// §164(b)(5): the sales-tax election was not made, so SALT used income tax. In a state with no income
/// tax this is usually the strictly larger deduction, and the filer may be overpaying.
SalesTaxElectionNotMade,
```

### 3.5 The tests

**The per-question property test** — for each `FormQuestion`: build a return where it is live; blank it →
assert `screen_inputs` refuses **with that entry's `RefuseReason`**; answer it (both `n` and `y`) → assert
that refusal is gone; assert `income answer` **asks** it.

**★ The completeness anchor (Fable I-4).** r1's property test *iterated the registry*, so dropping an entry
would silently drop its scenario and the suite would stay green — **the anti-vacuity machinery was itself
vacuous, the very P8a-I2 shape it exists to prevent.** Fixed by anchoring to the **enum**, not the slice:

```rust
// A NEW QuestionId variant is a compile error here until it is registered.
for id in QuestionId::ALL {
    match id { /* exhaustive — no `_` arm */ }
    assert_eq!(FORM_QUESTIONS.iter().filter(|q| q.id == *id).count(), 1,
               "{id:?} must have exactly one registry entry");
}
```

**★ And the hand-written per-question refusal tests are KEPT, not deleted.** r1 said the property test
"replaces per-question tests forever" — which invited deleting exactly the tests that would catch a dropped
entry. They stay.

**Mutation-checked (acceptance, §4):** delete the registry loop → a named test fails; delete the `build`
refusal → a named test fails; drop an entry from `FORM_QUESTIONS` → a named test fails; drop *either* new
advisory → a named test fails.

### 3.6 Explicitly NOT doing: the `ScreenedInputs` witness

A newtype produced only by `screen_inputs`, so compute/print can only accept screened input, was considered
and **rejected**. It would work (`resolve_core` is already the choke point). But it answers a class with
**one** Important on the entire ledger, at ~15 signature changes plus dozens of test sites — and **it cannot
prevent the next D-8**: a witness certifies that the *existing* screens **ran**, not that the *right* screens
**exist**. Every recurrence in this program was a **missing or mis-scoped** screen, never a **skipped** one.
With the registry + the `build` refusal, every consumer is locally fail-closed for this class.

---

## 4. Acceptance

- **All seven declarations are registry entries**; `screen_inputs`, `income answer`, and
  `ReturnHeader::build` derive from the registry. **No liveness predicate is written twice.**
- **P8a I1 dies structurally** — refusal scope and prompt scope are the same `fn` and *cannot* disagree.
- **P8a I3 dies** — an unanswered live declaration cannot reach a printed form (`HeaderError::Unanswered`),
  and the `printed.rs` Schedule-B `unwrap_or(false)` site is closed with it.
- **A new `bool` / `Option<bool>` / defaulted enum anywhere in the input model does not compile** until it is
  registered or exempted **with its class and statutory reason**. No `_` arm in the classifier.
- **The owner mandate holds: no benefit is forgone in silence.** `blind` and `salt_use_sales_tax` each fire a
  mandatory advisory naming the money; each is a skippable prompt in `income answer`.
- **`hsa_present` refuses when unanswered**; **`dual_status_alien` exists, is asked, and refuses when
  unanswered**; `Some(true)` on either refuses as unsupported.
- **The three DEAD fields are deleted.**
- **Schedule B 7a "Yes" with a blank 7b refuses** (Fable I-6 — a country name is required text on the form;
  same class as `ScheduleCNoBusinessDescription`, which was graded Important on identical reasoning).
- **MFJ with no spouse identity**: `ReturnHeader::build` refuses (Fable M-5), or it is filed with an owning
  phase and the acceptance sentence names the surviving half.
- Every guard above is **mutation-checked** (§3.5).
- `make check` green; **0 Critical / 0 Important** from independent review.
- FROZEN (`tax/{types,compute,se}.rs`) unchanged. `screen_inputs` keeps its signature; `resolve.rs` and the
  delta path untouched.

## 5. Build order (TDD; each step red → green)

1. **`questions.rs`** — `QuestionId` (+`ALL`), `FormQuestion`, `FORM_QUESTIONS` (7 entries). Liveness lifted
   from the current refusals **except** `DependentSpouse`, corrected to `Mfj || spouse.is_some()` (**= P8a I1**).
2. **The completeness anchor + property test** (§3.5) — RED on the I1 case first, then GREEN.
3. **`screen_inputs`** derives from the registry; hand-written unanswered-blocks delete; value-dependent
   rules stay. Fix any multi-defect fixture that asserted a specific reason (M-2).
4. **`hsa_present` → `Option<bool>`** + registry entry (§2.4). **New `dual_status_alien: Option<bool>`** +
   registry entry + `DualStatusAlienUnsupported` (§2.5).
5. **`cmd/answer.rs`** derives from the registry; the `Question` enum shrinks to the DOB residue (M-1); adds
   the skippable class-(B) prompts.
6. **`ReturnHeader::build`** → `HeaderError` (I-2); update `admin.rs` caller mapping and all fixtures.
7. **The two advisories** (§3.4) — the owner mandate.
8. **Schedule B 7a-yes/7b-blank refusal** (I-6).
9. **The classifier**, recursing (I-3), no `_` arm; the exemption register with class + statutory reason.
   Folds the open `header: _` follow-up.
10. **DELETE the three dead fields** (§2.2).
11. `LIMITATIONS.md` — the new refusals; the two new advisories; record that Sch B / MFS refusal texts now
    name `btctax income answer` (a deliberate **improvement**, not a no-op — r1's step 7 wrongly asserted all
    four already did; only the two D-8 texts do).
