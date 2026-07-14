# SPEC — P9: the FORM QUESTION REGISTRY

*Status: **r3**. Folds Fable spec review r2 (1C/6I/6M — `reviews/P9-SPEC-fable-r2.md`) and r1
(`reviews/P9-SPEC-fable-r1.md`).*
*Origin: `design/full-return/reviews/ARCH-P9-fable-question-registry.md`.*
*Subsumes the two open P8a findings, I1 and I3 (`reviews/P8a-fable-r1.md`), deliberately unpatched.*

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
answer it.* **Each has been forgotten at least once**: D-8 (step 1 — shipped in v0.2.0, understating tax and
printing an unaffirmed checkbox on a filed 1040); SPEC r1 I7, **day one of the program** (step 1,
`foreign_accounts: bool` auto-checking Schedule B 7a "No"); P8a I1 (step 3); P8a I2 + §199A +
`ScheduleCNoBusinessDescription` (step 5 — guards shipped with **zero** tests); P8a I3 (step 4).

This class is **~21% of every blocking finding in the program** — the largest single class, and the only one
that recurred at *every* stage with identical mechanics.

**★ And it recurred INSIDE THIS SPEC, twice.** r2 flipped `hsa_present: bool → Option<bool>` **with no
migration** — so serde would read every stored `false` as `Some(false)`, ratifying a never-asked default as
the filer's answer: *verbatim the D-8 laundering, re-armed in the document written to abolish it* (r2 I-1).
And r2's anti-vacuity property test **iterated the registry**, so dropping an entry would silently drop its
own scenario (r1 I-4). The class does not respect the author's intentions. It is only closed by
construction.

### 1.1 What this spec does NOT claim

- The review volume is **not** mostly this defect. ~60% of blocking findings are the honest price of the IRC
  under a 0C/0I gate. A third of the review *artifacts* found nothing — they are re-verification rounds.
- **The engine's architecture is sound** and is not touched here.
- **★ THE TERNARY IS NOT THE FIX, AND IS ALREADY IMPLEMENTED.** All five original questions are
  `Option<bool>` *today*. Anyone who concludes "so we need a tri-state" has learned **nothing**. The gap is
  that nothing *forces* it, the obligations are hand-wired, and **a type flip without a migration re-creates
  the bug** (§1's ★).

---

## 2. ★ The statutory test — what makes a default lawful

*(Replaces r1's engineering criterion ("conservative **and** advised"), which Fable rejected as not fitting
the fields. **Conservatism is not the test — it is a consequence.**)*

**26 U.S.C. §6065** — every return *"shall contain or be verified by a written declaration that it is made
under the penalties of perjury."* **26 U.S.C. §7206(1)** — felony to willfully subscribe a return *"which he
does not believe to be true and correct **as to every material matter**."*

**The filer cannot believe true an answer they were never asked, and software cannot supply belief on their
behalf.**

Against that: deductions are *"a matter of legislative grace"*, and *"a taxpayer seeking a deduction must be
able to point to an applicable statute and show that he comes within its terms"* — ***New Colonial Ice Co. v.
Helvering*, 292 U.S. 435, 440 (1934)**. The burden to **claim** is the taxpayer's. A filer who never says "I
am blind" simply does not get the §63(f) addition: **nothing false is stated; a benefit is forgone.**

> ### The classifying question
> **Does an unanswered box make the filer ASSERT something, or merely FORGO something?**
> **Assert ⇒ no lawful default exists. Forgo ⇒ `false` is what the statute already assumes.**

| class | statutory basis | default | on absence |
|---|---|---|---|
| **(A) DECLARATION** | §6065 + §7206(1) | **none exists** | **REFUSE** |
| **(B) BENEFIT CLAIM** | *New Colonial Ice* | **lawful** | grant nothing, and **ADVISE** (§2.1) |
| **(C) NO TAX DIRECTION** | neither claims nor asserts | **lawful** | silent |
| **(D) DEAD** | — | **FORBIDDEN** | **consume or delete** (§2.2) |

⚠️ **Two caveats, recorded so the doctrine is not oversold** (Fable r2):
1. **§7206(1) does not expose the *unwitting* filer** — willfulness (*Cheek v. United States*) shields
   someone who never saw the box. Our load-bearing half is the §6065 jurat and the design norm it implies
   (*software cannot supply belief*), **not** a prosecution theory. Real-world stakes are civil (§6662) plus
   this application's own integrity. Do not let anyone read this spec as claiming the filer commits a felony.
2. The ASSERT/FORGO line is clean **on fields**; it fails only where a *field conflates two populations* —
   which is exactly what `hsa_present` did (§2.4).

### 2.1 ★ THE PRINT CRITERION — why a single checkbox cannot be deferred (Fable r2 I-4)

The census above classifies **inputs**. It does not, by itself, tell you which *printed* boxes must become
questions — and without that rule the census is a list, not a closed set. This codebase already practices the
distinction but never wrote it down:

- The **digital-asset question** (a Yes/No **pair**) prints **both boxes blank** on a no-activity year —
  *"btctax never answers 'No'… `false` means unchecked, not answered in the negative"*
  (`return_1040.rs:986`). Schedule B's FBAR sub-question and Schedule C's G/I/J pairs do the same.
- The **dependent checkbox** is a **single** box. Unchecked **reads as "No."**

> ### The criterion
> **A Yes/No PAIR can be visibly deferred to the filer's pen** — both-blank is *facially incomplete*, so the
> form does not lie; the human completes it.
> **A SINGLE checkbox cannot be deferred** — unchecked *is* the "No". There is no blank state. So a single
> box that carries a factual assertion **must be a class-(A) question.**

This is what makes §2.5 (dual-status) principled rather than ad hoc, and it is what identifies §2.7
(Schedule A line 8). **Any new single checkbox that asserts a fact is a registry question, by rule.**

**The printed-checkbox sweep** (Fable r2, verified across all seven supported forms):

| box | disposition |
|---|---|
| 1040 header: dependent ×2, MFS-itemize | covered (registry) |
| **1040 header: dual-status alien** | **★ OPEN → §2.5** |
| 1040 digital-asset pair | Yes/No pair — lawfully deferred, data-derived |
| Schedule B 7a / 8 | covered (registry) |
| Schedule B FBAR sub-question | pair — deferred, documented (`schedule_b.rs:22`) |
| Schedule A 5a (sales tax), 18 (elects smaller) | filled |
| **Schedule A line 8 (mixed-use mortgage)** | **★ OPEN → §2.7** |
| Schedule C F (accounting method) | filled *(accrual is a separate open Important → P8)* |
| Schedule C G / I / J | pairs — deferrable |
| Schedule C H (started/acquired business) | single box, **no tax direction** ⇒ class (C), disclaimed |
| Form 8960 §6013(g)/(h) · §1.1411-10(g) boxes | **opt-in elections** — lawful unchecked (class C) |
| 8949 / Schedule D | data-derived |

### 2.2 ★ OWNER MANDATE — a forgone benefit must never be silent

> *"Let's not forgo a benefit without informing user they may be giving away more money than required."*
> — the owner, 2026-07-14

Class **(B)** is therefore a **two-part** rule, and the second part is **not optional**:

> **A default that forgoes a benefit is lawful ONLY IF the filer is told they forwent it.**

We may **not refuse** on a benefit claim — the burden to claim is the taxpayer's, and a return is valid
without it. But silence overtaxes someone and lets them believe the number was theirs.

**★ Class (B) fields must therefore be `Option<bool>` TOO (Fable r2 I-2).** r2 kept them as bare `bool`s and
said "empty input leaves `false`, the advisory fires" — **but an explicit "no" also leaves `false`.** The two
are the same value, so the advisory would either fire forever (nagging a sighted filer who *just told us* they
are sighted) or never. **The asked/unasked distinction is exactly what an `Option` exists to carry** — which
is why `AgedBoxForfeitedNoDob` works: it fires on `date_of_birth.is_none()`, i.e. on **unknown**, never on
known-young. That is the class-(B) shape, and it needs the tri-state as much as class (A) does.

| field | type | benefit | advisory fires when |
|---|---|---|---|
| `Person.blind` | **`Option<bool>`** ← was `bool` | §63(f) additional std deduction | **`None`** (never asked) — *not* on `Some(false)` |
| `ScheduleAInputs.salt_use_sales_tax` | **`Option<bool>`** ← was `bool` | §164(b)(5) sales-tax deduction | **`None` ∧ the return ITEMIZES** (§3.4) |
| *`Person.date_of_birth`* (already `Option`) | — | §63(f) aged box | `is_none()` ✅ *exists — the model* |

**Verified (§63(f)(1)/(2)):** the aged and blind additional amounts are **identical** ($600/$750 base;
$1,550/$1,950 TY2024) **and they STACK** — a filer both 65+ and blind gets two boxes. So we currently tell a
filer they forfeited the box for want of a *birthday* and say nothing when they forfeit the *same box, same
statute, same dollars* for want of a *checkbox*.

**Both flips need the §2.6 migration** — a stored `false` was never asked (same rule, one discipline).

⚠️ **The SALT prompt is a footgun and needs a guard (Fable r2 I-3).** `salt_line_5a`: **election on ⇒ 5a =
`salt_sales_tax_amount` ONLY** — W-2 box 17/19 withholding and estimated payments drop out. A filer who
answers "y" (hearing *"would you like that deduction?"*) with no amount captured gets **5a = $0** — a
silently collapsed SALT deduction, i.e. the owner mandate violated by the very prompt written to honour it.
The existing guard is one-directional (`SaltSalesTaxWithoutElection` catches amount-without-election).
⇒ **New symmetric refusal `SalesTaxElectionWithoutAmount`**: election `Some(true)` ∧ `salt_sales_tax_amount
== 0` ∧ (any income-tax SALT input > 0) ⇒ **refuse**. And the prompt is **scoped to returns that already
carry a `schedule_a`** (when `schedule_a` is `None` the question is not live — there is nowhere to write it
and no deduction to elect).

### 2.3 Class (D) — DEAD fields are FORBIDDEN, and deletion alone is NOT the fix

A captured-but-unconsumed field is a lie about what the app honors, and it is **pre-armed**. *This is exactly
how D-8 happened* — the dependent flag sat inert as a defaulting `bool` until someone wired it to the standard
deduction, and **at that instant the default silently became an answer.**

Three fields are dead **today** (verified: zero consuming sites):

| field | what it will gate | direction when it goes live |
|---|---|---|
| `Person.ssn_valid_for_employment` | §32 EITC | `false` denies the credit ⇒ overstates — safe |
| `Dependent.ssn_valid_for_employment` | §32 EITC / §24 CTC | as above |
| **`W2.box13_retirement_plan`** | **§219(g) IRA-deduction phase-out** | **`false` = "not covered by a plan" ⇒ NO phase-out ⇒ OVERSTATES the IRA deduction ⇒ ★ UNDERSTATES TAX** |

**⇒ DELETE all three.** *(Direction analysis verified by Fable; box 13 is the dangerous one, and it was filed
nowhere before this spec.)*

**★ But r2's stated mechanism was BACKWARDS (Fable r2 I-6), and the correction is load-bearing.** r2 said
*"deleting is a breaking change to the TOML surface; there are no users, so the cost is zero."* **There is no
`deny_unknown_fields` anywhere in the workspace**, and `income import` is `toml::from_str` straight into
`ReturnInputs` — **serde ignores unknown keys.** So after deletion, a filer's TOML carrying
`box13_retirement_plan = true` — *a real W-2, box 13 checked, faithfully transcribed* — **imports clean and
the key vanishes**: no error, no warning, and (unlike today) no trace even in `income show`. **Nothing
breaks, and that is the problem.** §2.3's own indictment of dead fields — *"the user types it, we ignore
it"* — would describe the **post**-deletion state *more* accurately than the pre-deletion one.

⇒ **Deletion REQUIRES its companion: unknown-key rejection at the TOML import boundary.** *Design note:*
`#[serde(deny_unknown_fields)]` on `ReturnInputs` itself would also bind the **stored-JSON** path and break
that module's documented forward-compat discipline. So this is a **CLI-side** decision — parse to
`toml::Value` first, walk it against the known-key set, and **refuse unknown keys** (naming them) — not an
attribute tossed onto the core struct.

### 2.4 ★ `hsa_present` — RE-SCOPED. r2 would have BRICKED VALID RETURNS (Fable r2 C-1)

**r2's premise was FALSE, and it was my error, not the reviewer's.** r2 said *"an HSA needs Form 8889"* and
made the question *"do you have an HSA?"*, live always, refusing on `Some(true)`.

**26 U.S.C. §223(h)** puts HSA reporting on **trustees** and **health-plan providers** — *not on the
individual for merely holding an account.* A filer with a **dormant HSA** (funds from a prior year, no
contributions, no distributions) has **no Form 8889 obligation at all**. Their return omits nothing.

⇒ Under r2, that filer would be asked, would answer **truthfully "yes"**, and would be **permanently
refused** — a return btctax computes **correctly today**. Their only exits: answer falsely, or leave. **The
spec would have bricked a valid return, which is precisely the failure it exists to prevent.**

**The real hazard is ACTIVITY, and it is worse than the completeness argument r2 made.** **26 U.S.C.
§223(f)**: a distribution *"not used exclusively for qualified medical expenses **shall be included in the
gross income** of such beneficiary"* — plus **an additional tax equal to 20 percent**. And **btctax captures
no Form 1099-SA at all** (verified: zero hits). So an unasked filer who took a non-qualified distribution
gets a return that **omits the income AND the 20% penalty** — an **understatement**, not a paperwork lapse.

⇒ **`sch1.hsa_present: bool` → `sch1.hsa_activity: Option<bool>`**, and the question is scoped to the **Form
8889 triggers**:

> *"In {year}, did you (or your employer) **contribute to**, or did you **take a distribution from**, a health
> savings account?"*

- `None` ⇒ **REFUSE** (`HsaActivityUnanswered`) — an unasked distribution omits gross income (§223(f)).
- `Some(true)` ⇒ refuse as unsupported (Form 8889 / 1099-SA out of scope for v1) — existing behaviour.
- `Some(false)` ⇒ **proceed.** A dormant-HSA holder answers "no" **truthfully** and is **not** bricked.

*Partial backstop, unchanged:* employer contributions surface as W-2 box 12 code **W**, outside
`INERT_BOX12_CODES`, so they refuse by another path. That covers **contributions only** — it does **nothing**
for **distributions**, the understating half.

**★ The lesson, recorded:** I reached for §6065's "complete" because it was the statute already in my hand
from the previous question, and it *sounded* right. The operative statute for this field is **§223(f)**, and
it says something **stronger and different**. *Find the statute for **the field** — never reuse the statute
from the last argument.*

### 2.5 The dual-status alien — a declaration we already print, with NO field behind it (Fable r1 I-5)

The 1040's third header checkbox reads: *"Spouse itemizes on a separate return **or you were a dual-status
alien**."* We print it from the MFS coupling alone (`packet.rs:325`). **The dual-status half is silently
answered "No" on every non-MFS return we have ever printed** — no field, no question, no refusal, no
`LIMITATIONS` entry.

It is a **SINGLE box asserting a fact** ⇒ class (A) **by the §2.1 criterion** (this is the justification r2
lacked). **§63(c)(6)(B)**: a nonresident alien individual's standard deduction is **zero** — and a dual-status
person is a nonresident for part of the year — while `ItemizeElection::Auto` grants it. **A silent
understatement plus an unaffirmed checkbox**, for a population (first-year visa holders with W-2s and crypto)
squarely inside this app's archetype.

**Proportionality, argued and settled (Fable r2):** there is no cheaper correct answer. No input signals
residency; a skippable+advisory design leaves the box silently unchecked (*the defect itself*); and pen-deferral
is unavailable because it is a **single** box. **One question per filer per year is the honest price** of not
silently answering a statement that, wrong, is a false declaration *and* an understatement together.

⇒ **NEW `ReturnInputs.dual_status_alien: Option<bool>`**, registry question, live always.
`Some(true)` ⇒ `DualStatusAlienUnsupported`. **New field ⇒ `None` on an existing row is honest ⇒ no
migration** (contrast §2.4, whose *type changes* — §2.6).

### 2.6 ★ THE MIGRATION — `SCHEMA_VERSION` 2 (Fable r2 I-1)

**Three fields change type from `bool` → `Option<bool>`**: `sch1.hsa_activity` (§2.4), `Person.blind` and
`ScheduleAInputs.salt_use_sales_tax` (§2.2). None carries `skip_serializing_if`, so **every stored blob
already contains `false` for each** — and serde would read that as `Some(false)`: **a never-asked default
ratified as the filer's answer. That is the D-8 laundering, re-armed inside the spec that abolishes it.**

The consequence is not theoretical: the new `HsaActivityUnanswered` refusal would **never fire for exactly the
population that has the bug**, and this time the direction is the bad one (an HSA distribution left untaxed).

⇒ **`SCHEMA_VERSION = 2`**, and `row_to_inputs` gains:
`version < 2 ⇒ unlaunder(sch1.hsa_activity), unlaunder(taxpayer.blind), unlaunder(spouse.blind),
unlaunder(schedule_a.salt_use_sales_tax)` — with **named v1-blob tests** per the P8a I2 pattern (the fixture
must rewrite **every** key it claims to cover, and **assert each rewrite lands**).

*(A TOML that **explicitly writes** `blind = false` is a **typed answer** and correctly loads `Some(false)`.
Only the **stored-row** path launders. `dual_status_alien` needs no migration — it is a new field.)*

### 2.7 ★ Schedule A line 8 — the mixed-use mortgage box (Fable r2 I-4)

*"If you didn't use all of your home mortgage loan(s) to buy, build, or improve your home… check this box."*
We fill 8a from `mortgage_interest_1098` and **never touch the box** (the Schedule A filler writes exactly two
checkboxes). **A single box, printed unchecked on every itemizing return with a mortgage.**

Under **§163(h)(3)(F)** (2018–2025), interest on proceeds **not** used to buy/build/improve is **not
deductible at all**. So an unchecked box beside a full 8a deduction is **an unaffirmed statement AND an
understatement** — identical in shape to §2.5, and caught by the §2.1 criterion.

⇒ **NEW `ScheduleAInputs.mortgage_all_used_to_buy_build_improve: Option<bool>`** — registry question, live
when `schedule_a` files with mortgage interest > 0. `Some(false)` ⇒ **refuse**
(`MixedUseMortgageUnsupported` — the Pub. 936 allocation is unmodeled). New field ⇒ no migration.

### 2.8 The class boundary is wider than `bool` — defaulted ENUMS

| field | default | status |
|---|---|---|
| `ScheduleCInputs.accounting_method` | `Cash` | `"accrual"` accepted, unmodeled, **unrefused**, and it **flips the printed Sch C line F** — *already filed → P8* |
| `ReturnInputs.itemize_election` | `Auto` | interacts with §2.5/§2.7 |
| `W2.owner` / `ScheduleCInputs.owner` | `#[default] Taxpayer` | **★ NOT a silent-default defect** — Fable r2 M-2 **corrects its own r1 aside, which r2 inherited**: neither carries `#[serde(default)]`, so **the TOML import REQUIRES the key**. The `#[default]` reaches only Rust-side fixture construction. Exemption reason: *"serde-required at import."* |
| `QbiInputs.reit_ptp_carryforward_in_provenance`, `CharitableCarryItem.provenance` (`CarryProvenance`) | — | no print, no tax direction ⇒ class (C) |

---

## 3. The design

### 3.1 The registry

```rust
// crates/btctax-core/src/tax/questions.rs   (NEW)

/// A DECLARATION (§2, class A) — the filer ASSERTS it under §6065's jurat, so there is NO lawful default
/// and an unanswered one must REFUSE.
///
/// ONE entry per question, owning the prompt, the refusal, the refusal DETAIL, the liveness scope, and the
/// accessors. `screen_inputs`, `income answer`, and `ReturnHeader::build` DERIVE from this list.
pub struct FormQuestion {
    pub id: QuestionId,
    pub prompt: &'static str,
    pub unanswered: RefuseReason,
    /// ★ The FULL refusal detail (r1 I-1). NOT derived from `prompt`: today's texts carry the statutory
    /// cite, and the D-8 texts carry the REMEDY (`run btctax income answer`) — which a named acceptance
    /// test asserts, and which doctrine requires ("a refusal with no exit is just a brick with better
    /// prose"). A prompt-derived text would drop both.
    pub unanswered_detail: &'static str,
    /// ★ THE liveness predicate — the ONLY copy in the codebase.
    pub live: fn(&ReturnInputs) -> bool,
    pub get: fn(&ReturnInputs) -> Option<bool>,
    pub set: fn(&mut ReturnInputs, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionId {
    DependentTaxpayer, DependentSpouse, MfsSpouseItemizes,
    ForeignAccounts, ForeignTrust,
    HsaActivity,            // §2.4
    DualStatusAlien,        // §2.5
    MortgageAllUsedToBuyBuildImprove, // §2.7
}
impl QuestionId { pub const ALL: &'static [QuestionId] = &[ /* all 8 */ ]; }

pub const FORM_QUESTIONS: &[FormQuestion] = &[ /* 8 entries */ ];
```

*(Fable verified by live `cargo check` in r1 that this shape compiles.)*

| question | `live` |
|---|---|
| `DependentTaxpayer` | always |
| `DependentSpouse` | **`filing_status == Mfj \|\| header.spouse.is_some()`** ← **= P8a I1** |
| `MfsSpouseItemizes` | `filing_status == Mfs` |
| `ForeignAccounts` / `ForeignTrust` | `schedule_b_files(ri)` |
| `HsaActivity` | always |
| `DualStatusAlien` | always |
| `MortgageAllUsedToBuyBuildImprove` | `schedule_a` files ∧ `mortgage_interest_1098 > 0` |

**Recorded deliberate over-ask (r1 M-3, unfolded in r2):** `Mfj || spouse.is_some()` keeps the spouse-dependent
question live on an **MFS/QSS return carrying a stale spouse `Person`**, though the 1040's box is joint-only.
This **over-asks** (never under-asks) and is recoverable — `income answer` re-asks answered questions with the
current value as default. Accepted as conservative; recorded rather than fixed.

### 3.2 The three derivations

**`screen_inputs`.** One loop, placed **after** the integrity gates (negative money, malformed SSN) and
**before** the value-dependent rules (r1 M-2):

```rust
for q in FORM_QUESTIONS {
    if (q.live)(ri) && (q.get)(ri).is_none() {
        return refuse(q.unanswered.clone(), q.unanswered_detail);
    }
}
```

**Refusal precedence is explicitly NOT contract:** on a multi-defect return the loop may report a different
reason than today. Any test asserting a *specific* reason on a multi-defect fixture must be made single-defect.

**Value-dependent refusals stay hand-written** (domain rules *about the answer*, not about whether there is
one): `ForeignTrust == Some(true)`, `DependentSpouseUnsupported`, `HsaActivity == Some(true)`,
`DualStatusAlienUnsupported`, `MixedUseMortgageUnsupported`, `SalesTaxElectionWithoutAmount`. *(Fable verified
the two kinds cannot double-report: `None` and `Some(true)` are disjoint.)*

**`income answer`.** `live_questions`/`current_bool`/`set_bool` become registry iteration. The `Question` enum
**shrinks to the DOB residue** (r1 M-1) — it does not delete. The no-brick property (*everything the screen can
refuse for is askable*) becomes true **by identity**.

It also gains the class-(B) **skippable** prompts (§2.2): `blind` (taxpayer + spouse) and `salt_use_sales_tax`
(**only when `schedule_a` exists** — §2.2's footgun scope). Empty input ⇒ stays `None` ⇒ the advisory fires.
**The spouse-DOB prompt stays gated on `header.spouse.is_some()`** even though the spouse *question* widens to
MFJ, because `set_date` silently discards a spouse DOB when there is no spouse `Person`.

⚠️ **`answer` captures bools and dates only — it cannot capture STRINGS** (r2 M-5). So the §3.5 Schedule-B 7b
refusal has **no `answer` exit**: its detail must name `income import` as the remedy, and `LIMITATIONS.md` must
record the limit.

**`ReturnHeader::build` — the print boundary** (P8a I3). New error type (r1 I-2), because `SsnError` cannot say
this and `admin.rs` currently maps every build failure to *"fix the identity"*:

```rust
pub enum HeaderError {
    Ssn(SsnError),
    /// A live DECLARATION is unanswered. At PRINT there is no conservative direction — an unchecked box is a
    /// false "No" and a checked box is a false "Yes" — so refusal is the ONLY fail-closed behaviour.
    Unanswered(QuestionId),
}
```

`Display` names the question **and the remedy**. `admin.rs`'s mapping is updated.

**This also closes a second print site** Fable found that P8a I3 never mentioned: `printed.rs:936/:943` project
Schedule B Part III with **`unwrap_or(false)`** — *the exact idiom D-8 names as "the very shape of this
defect."* A build refusal over **all** live questions closes it.

### 3.3 The classifier

A function destructuring the input model in which every `bool` / `Option<bool>` / **defaulted enum** must be
classified as a registry question (A), or **exempted** on a named register carrying its **class and statutory
reason** (B/C per §2).

**★ It recurses over EVERY struct reachable from `ReturnInputs`** (r2 M-1 — r1's "as deep as
`first_negative_amount`" was a **false floor**: `fna` waives the whole header with `header: _`, so it recurses
into **neither `Person` nor `Dependent`**, which is where half the bools live). The reachable set includes
`HouseholdHeader`, `Person`, `Dependent`, `W2`, `Form1099Int/Div/G`, `ScheduleAInputs`, `Schedule1Inputs`,
`ScheduleCInputs`, `QbiInputs`, `Payments`, `CharitableGift`, `CharitableCarryItem`.

**★ The `_` rule, stated implementably (r2 M-6)** — "no `_` at all" is literally impossible (every
`String`/`Usd`/`Date` leaf must bind *something*):

> **`_` is FORBIDDEN on structs and collections** (must recurse) **and on `bool` / `Option<bool>` /
> defaulted-enum leaves** (must classify). **`_` is permitted on other scalar leaves** (`String`, `Usd`,
> `Date`, …).

⚠️ **Honest limit:** the classifier's force is "compile-error-until-a-human-**decides**", not
until-**correct**. `first_negative_amount` already shows the escape hatch (`header: _`). **Wildcard arms are
the residual convention.** The open `header: _` follow-up is fixed here.

### 3.4 The advisories the owner mandate requires (§2.2)

```rust
/// §63(f): a person was never asked about blindness, so the additional standard deduction was not granted.
/// Fires on None (NEVER ASKED) — never on Some(false). Same statute, worksheet line, and dollars as
/// `AgedBoxForfeitedNoDob`, and the two STACK.
BlindBoxForfeitedNotDeclared { per_box: Usd, persons: usize },
/// §164(b)(5): the sales-tax election was never asked, so SALT used income tax. Fires ONLY when the return
/// ITEMIZES (otherwise SALT is irrelevant and the advisory is pure noise) AND the election is None.
SalesTaxElectionNotAsked,
```

### 3.5 The tests

**Per-question property test** — for each `FormQuestion`: build a return where it is live; blank it ⇒ assert
`screen_inputs` refuses **with that entry's `RefuseReason`**; answer it (`n` and `y`) ⇒ assert the refusal is
gone; assert `income answer` **asks** it.

**★ The completeness anchor (r1 I-4).** r1's property test *iterated the registry*, so dropping an entry would
silently drop its scenario — **the anti-vacuity machinery was itself vacuous.** Anchor to the **enum**:

```rust
for (i, id) in QuestionId::ALL.iter().enumerate() {
    // exhaustive match — a NEW variant is a compile error until listed
    let idx = match id { QuestionId::DependentTaxpayer => 0, /* … */ };
    assert_eq!(idx, i, "QuestionId::ALL is out of order / missing {id:?}");   // r2 M-3: catches add-shirk
    assert_eq!(FORM_QUESTIONS.iter().filter(|q| q.id == *id).count(), 1);
}
assert_eq!(QuestionId::ALL.len(), N);
```

*(r2 M-3: the index round-trip is what stops "add the match arm, skip the `ALL` element" — which would compile
green and never be iterated.)*

**★ The hand-written per-question refusal tests are KEPT, not deleted.** r1 said the property test "replaces
per-question tests forever" — which invited deleting exactly the tests that catch a dropped entry.

**Mutation-checked (acceptance):** delete the registry loop → a named test fails; delete the `build` refusal →
a named test fails; drop a `FORM_QUESTIONS` entry → a named test fails; **drop any `unlaunder` in the v1→v2
migration → a named test fails** (the P8a I2 lesson: the fixture must rewrite **every** key and assert each
rewrite lands); drop either advisory → a named test fails.

### 3.6 Explicitly NOT doing: the `ScreenedInputs` witness

Rejected. It answers a class with one Important on the ledger, at ~15 signature changes plus dozens of test
sites — and **it cannot prevent the next D-8**: a witness certifies the *existing* screens **ran**, not that the
*right* screens **exist**. Every recurrence here was a **missing or mis-scoped** screen, never a **skipped** one.

---

## 4. Acceptance

- **All eight declarations are registry entries**; `screen_inputs`, `income answer`, and `ReturnHeader::build`
  derive from the registry. **No liveness predicate is written twice.**
- **P8a I1 dies structurally** (one `fn`, two consumers — they cannot disagree). **P8a I3 dies** (an unanswered
  live declaration cannot reach a printed form), and the `printed.rs` `unwrap_or(false)` site closes with it.
- **★ No valid return is bricked.** A dormant-HSA holder answers "no" truthfully and their return computes
  (§2.4). *This is a regression test, not a slogan.*
- **★ The v1→v2 migration holds**: a stored `false` for `hsa_activity` / `blind` / `salt_use_sales_tax` loads as
  `None`, not `Some(false)`. Mutation-checked per key.
- **A new `bool` / `Option<bool>` / defaulted enum anywhere reachable from `ReturnInputs` does not compile**
  until registered or exempted **with its class and statutory reason**.
- **The owner mandate holds: no benefit is forgone in silence.** `blind` and `salt_use_sales_tax` each fire a
  mandatory advisory naming the money, **on `None` only** — never nagging a filer who answered.
- **The SALT prompt cannot silently zero line 5a** (`SalesTaxElectionWithoutAmount`).
- **`dual_status_alien` and the Schedule A line 8 box exist, are asked, and refuse when unanswered.**
- **The three DEAD fields are deleted, AND `income import` refuses unknown TOML keys** (else deletion makes the
  lie worse, not better).
- **Schedule B 7a "Yes" with a blank 7b refuses**, and its detail names `income import` as the exit (`answer`
  cannot capture strings).
- **MFJ with no spouse identity**: `ReturnHeader::build` refuses, or it is filed with an owning phase.
- Every guard above is **mutation-checked**.
- `make check` green; **0 Critical / 0 Important** from independent review.
- FROZEN (`tax/{types,compute,se}.rs`) unchanged. `screen_inputs` keeps its signature; `resolve.rs` untouched.

## 5. Build order (TDD; each step red → green)

*★ Reordered — r2's order did not compile (I-5): steps 1–3 referenced fields step 4 created.*

1. **Fields first.** `sch1.hsa_present: bool` → `hsa_activity: Option<bool>` (§2.4); `Person.blind` and
   `ScheduleAInputs.salt_use_sales_tax` → `Option<bool>` (§2.2); **NEW** `ReturnInputs.dual_status_alien` and
   `ScheduleAInputs.mortgage_all_used_to_buy_build_improve` (§2.5, §2.7).
2. **★ The migration — `SCHEMA_VERSION` 2** (§2.6), with per-key mutation-checked v1-blob tests. *Before any
   consumer reads the new types, or the laundering ships.*
3. **`questions.rs`** — `QuestionId` (+`ALL`), `FormQuestion`, `FORM_QUESTIONS` (8 entries). Liveness lifted
   from the current refusals **except** `DependentSpouse`, corrected to `Mfj || spouse.is_some()` (**= P8a I1**).
4. **The completeness anchor + the property test** (§3.5). *Step 5 is what turns the P8a-I1 case green.*
5. **`screen_inputs`** derives from the registry; hand-written unanswered-blocks delete; value-dependent rules
   stay and gain the new ones. Fix multi-defect fixtures that asserted a specific reason.
6. **`cmd/answer.rs`** derives from the registry; enum shrinks to the DOB residue; adds the class-(B) skippable
   prompts (SALT scoped to returns carrying a `schedule_a`).
7. **`ReturnHeader::build`** → `HeaderError`; update `admin.rs` mapping and all fixtures.
8. **The two advisories** (§3.4) — the owner mandate. Fire on `None` only.
9. **`SalesTaxElectionWithoutAmount`** (§2.2) and the **Schedule B 7a-yes/7b-blank** refusal (r1 I-6).
10. **The classifier** (§3.3), recursing over every reachable struct; the `_` rule as stated; the exemption
    register with class + statutory reason. Folds the open `header: _` follow-up.
11. **DELETE the three dead fields** (§2.3) **AND add unknown-key rejection to `income import`** — the two ship
    together or not at all.
12. `LIMITATIONS.md` — the new refusals; the two advisories; that Sch B / MFS refusal texts now name `btctax
    income answer` (a deliberate **improvement**); that `income answer` cannot capture strings.
