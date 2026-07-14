# SPEC — P9: the FORM QUESTION REGISTRY

*Status: **r4**. Folds Fable spec review r3 (1C/7I/6M/4Nit — `reviews/P9-SPEC-fable-r3.md`), r2
(`reviews/P9-SPEC-fable-r2.md`) and r1 (`reviews/P9-SPEC-fable-r1.md`).*
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

**★ And it has now recurred INSIDE THIS SPEC in every single revision.** r2 flipped
`hsa_present: bool → Option<bool>` **with no migration** — so serde would read every stored `false` as
`Some(false)`, ratifying a never-asked default as the filer's answer: *verbatim the D-8 laundering, re-armed
in the document written to abolish it* (r2 I-1). r2's anti-vacuity property test **iterated the registry**, so
dropping an entry would silently drop its own scenario (r1 I-4). And **r3 — the revision that added the
migration — wrote it against a key that does not exist** (`hsa_present` is *renamed*, so the unlaunder was
dead code and its mandatory mutation-check was unsatisfiable: a fixture that passes vacuously, which is the
**P8a I2 shape rebuilt inside the migration that cites P8a I2**), and **r3's re-scoped HSA question covered
two of the form's four triggers**, so a testing-period filer answers "No" truthfully and understates (r3 C-1,
r3 I-1).

**Four revisions; four recurrences; each one authored by someone who had just finished writing that this
exact thing keeps happening.** The class does not respect the author's intentions, his attention, or his
sincerity. **It is only closed by construction** — which is why §3.3's honest limit (r3 I-5) matters more
than any prose in this document.

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

> ### The criterion (★ REFINED — r3 I-3: r2's version contradicted §2.2 on the spec's own flagship fields)
>
> 1. **A Yes/No PAIR may be left BOTH-BLANK and deferred to the filer's pen** — both-blank is *facially
>    incomplete*, so the form does not lie; the human completes it.
> 2. **A SINGLE checkbox has no blank state** — unchecked **is** the "No". So ask what the *unchecked* state
>    does:
>    - **Unchecked ASSERTS or CLAIMS** — it states a fact the filer never affirmed, or grants them a benefit
>      that turns on that fact ⇒ **class (A)**: a registry question, refuse when unanswered.
>      *(dependent ×2, MFS-itemize, dual-status, Schedule A line 8)*
>    - **Unchecked merely FORGOES** — nothing false is stated; the filer simply does not claim
>      (*New Colonial Ice*) ⇒ **class (B)**: lawful unchecked, **but only with §2.2's mandatory advisory.**
>      *(aged, blind, CTC/ODC)*
> 3. **★ A box SOFTWARE ANSWERS on the filer's behalf** — checked or unchecked by *our* reasoning rather than
>    theirs — is permitted **only by a WRITTEN EXEMPTION** that names what makes the answer *entailed* (either
>    positive data, or the scope of a supported return), **plus a `LIMITATIONS.md` entry.** Silence is not an
>    exemption. *(1040 digital-asset "Yes" — entailed by ledger evidence; Schedule D QOF "No" — entailed by
>    scope; Form 8283 line k — entailed by data.)*

Rule 2 is what makes §2.5 (dual-status) principled rather than ad hoc, and what identifies §2.7 (Schedule A
line 8). Rule 3 is what stops the *other* evasion: an assertion is no less an assertion because **we** made it.

⚠️ **Rule 1's honest limit.** Deferring a pair to the pen assumes the filer *notices* the blank on an
otherwise machine-filled form. That is a real assumption, and the mitigation is not in this spec: it is the
existing `DRAFT` watermark plus the packet's own instructions. Recorded, not waved.

**The printed-checkbox sweep — COMPLETE.** *(r3 I-4: r2's sweep said "all seven supported forms"; the packet
prints **fourteen** — `PrintedForms`, `packet.rs:361-389`. It omitted nine boxes and mislabeled the one box on
the whole surface that software answers as "data-derived".)*

| box | class | disposition |
|---|---|---|
| 1040 header: dependent ×2, MFS-itemize | (A) | covered (registry) |
| **1040 header: dual-status alien** | **(A)** | **→ §2.5 (NEW question)** |
| 1040 header: aged ×2, blind ×2 (`packet.rs:200-216`) | **(B)** | unchecked forgoes §63(f). `blind` → §2.2 (tri-state + advisory); aged → `AgedBoxForfeitedNoDob` ✅ *exists* |
| 1040 dependents: CTC / ODC per row (`form1040_full.rs:377`) | (B) | deliberately unchecked, L19 = $0; `Advisory::CtcOdcOmitted` names the money ⇒ owner mandate satisfied |
| 1040 `more_than_four_dependents` | — | >4 dependents **refuses** (`form1040_full.rs:366`) — the box is never needed |
| 1040 presidential fund ×2 (`form1040_full.rs:331-338`) | (C) | filled from input; **no tax direction** (§6096 is a fund designation, not a liability) |
| 1040 digital-asset pair | **exempt (rule 3)** | "Yes" **entailed by positive ledger evidence**; never auto-"No" (`return_1040.rs:986`) |
| Schedule B 7a / 8 | (A) | covered (registry) |
| Schedule B FBAR sub-question | (—) | pair — deferred to the pen, documented (`schedule_b.rs:22`) |
| Schedule A 5a (sales tax), 18 (elects smaller) | — | filled from input |
| **Schedule A line 8 (mixed-use mortgage)** | **(A)** | **→ §2.7 (NEW question)** |
| Schedule C F (accounting method) | — | filled *(accrual is a separate open Important → P8)* |
| Schedule C G / I / J | (—) | pairs — deferred to the pen ⇒ **`LIMITATIONS.md` entry (step 12)** |
| Schedule C H (started/acquired business) | (C) | single box, **no tax direction** ⇒ **`LIMITATIONS.md` entry (step 12)** *(r3 I-4: r2 called it "disclaimed"; no such disclaimer existed)* |
| **Schedule D QOF pair — "No" CHECKED unconditionally** (`schedule_d_full.rs:283-294`) | **exempt (rule 3)** | **★ the only box software answers.** Entailed by **scope**: btctax supports returns whose dispositions all come from the bitcoin ledger, and a QOF disposition has no input and no ledger representation — a filer with one is outside the supported set (P6 r1 I4). **Requires the `LIMITATIONS.md` entry it never had (step 12).** |
| Schedule D 17 / 20 / 22 pairs | — | genuinely data-derived from computed lines |
| Form 8283 line k (digital assets) | **exempt (rule 3)** | checked — entailed by data (the donation *is* crypto) |
| Form 8283 donee acknowledgment / restricted use | (—) | deferred blank by the module's fill/blank table (`form8283.rs:1-10`) |
| Form 8960 §6013(g)/(h) · §1.1411-10(g) | (C) | **opt-in elections** — lawful unchecked |
| Schedules 1 / 2 / 3, SE, 8949, 8959, 8995 | — | **verified: these fillers write no checkboxes at all** |

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
| `ScheduleAInputs.salt_use_sales_tax` | **`Option<bool>`** ← was `bool` | §164(b)(5) sales-tax deduction | **`None` ∧ `schedule_a.is_some()`** — *not* "∧ the return itemizes", which would go silent exactly when the unasked election is what would have **flipped** the return into itemizing (§3.4, r3 Nit-3) |
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
that module's documented forward-compat discipline. So this must bind **only the CLI's TOML import**, not the
core struct.

**★ Mechanism — `serde_ignored`, NOT a hand-written key list (r3 M-3).** r3 said "walk it against the known-key
set" — **a hand-maintained mirror of the struct is the exact hand-wiring pattern P9 exists to abolish**, and it
drifts the first time someone adds a field. `serde_ignored::deserialize` reports every ignored path *during the
same deserialization*, so the key set is **derived from the type**: no list to forget, and `[[w2]]` arrays,
nested tables and comments all work for free. *(Verified available offline: `serde_ignored` v0.1.14.)*

⇒ `income import` deserializes through `serde_ignored`, collects ignored paths, and **refuses, naming every
one.** It must reject `hsa_present` (the §2.6 rename) and the three deleted fields — which is precisely the
"faithfully transcribed W-2 box 13 silently vanishes" hole this section exists to close.

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

⇒ **`sch1.hsa_present: bool` → `sch1.hsa_activity: Option<bool>`**, and the question must cover **ALL FOUR**
Form 8889 filing triggers.

**★ r3 C-1 — r3's prompt covered TWO of the four, and the gap understated tax.** r3 asked *"did **you or your
employer** contribute to, or did you take a distribution from…"* and let `Some(false)` **proceed**. The four
triggers (Form 8889 "Who Must File" — *persuasive*; each resting on statute — *law*) are:

| # | trigger | statute | does r3's prompt catch it? |
|---|---|---|---|
| 1 | a contribution by **anyone on your behalf** — not just you or your employer | §223(a) | **NO** — a filer whose parent funded the HSA answers "no" truthfully |
| 2 | you received a **distribution** | §223(f) | yes |
| 3 | **testing-period failure** — you used the prior-year **last-month rule** (or an **IRA-to-HSA funding distribution**) and then ceased to be an eligible individual: you include the shortfall in gross income **plus a 10% additional tax** — ***in a year with NO contribution and NO distribution*** | §223(b)(8)(B), §408(d)(9)(D) | **NO — and this is the Critical.** The trigger fires on a year with *no HSA activity at all* in the ordinary sense |
| 4 | you **acquired an interest by the death** of the account beneficiary (a non-spouse beneficiary includes the FMV in gross income) | §223(f)(8)(B) | **NO** — they "took" nothing |

Trigger 3 is not exotic: *an employee with HDHP coverage who used the last-month rule and changed jobs in
January.* They answer r3's prompt **"No" — truthfully — and btctax computes a return omitting both the income
and the 10% additional tax.* **§2.4's own indictment of the *unasked* filer applies verbatim to the *mis-asked*
one**, and this is the same field's **second** Critical (r2 C-1 was the mirror image: it bricked a valid
return). *A question is a piece of tax law. Scoping it is not prose — it is the statute, and it gets the same
citation discipline as a computation.*

⇒ **The question, covering the trigger set:**

> *"In {year}, did ANY of these happen with a health savings account? — (a) anyone (you, your employer, or
> anyone else on your behalf) put money into one for you; (b) you took money out of one; (c) you inherited
> one; or (d) you stopped being HSA-eligible after using the last-month rule or an IRA-to-HSA funding
> distribution in a prior year."*

- `None` ⇒ **REFUSE** (`HsaActivityUnanswered`).
- `Some(true)` ⇒ refuse as unsupported (Form 8889 / 1099-SA out of scope for v1) — existing behaviour, and
  correct for **every** trigger: each one requires the form we do not have.
- `Some(false)` ⇒ **proceed.** A **dormant**-HSA holder answers "no" to all four clauses **truthfully** and is
  **not** bricked (r2 C-1 stays cured — verify this in the same regression test).

*Partial backstop, unchanged:* employer contributions surface as W-2 box 12 code **W**, outside
`INERT_BOX12_CODES`, so they refuse by another path. That covers **one clause of one trigger** — it does
**nothing** for distributions, third-party contributions, inheritance, or the testing period.

**★ The lesson, recorded twice now.** In r2 I reached for §6065's "complete" because it was the statute already
in my hand from the previous question, and it *sounded* right. In r3 I reached for §223(f) — the right statute
for the *distribution* half — and let it stand for the whole field. Both times the error was the same:
**generalizing from the statute I had already found instead of enumerating the ones the field actually
touches.** *Find every statute for **the field**. "I found A statute" is not "I found THE statutes."*

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

### 2.6 ★ THE MIGRATION — `SCHEMA_VERSION` 2 (Fable r2 I-1, r3 I-1)

**Why a migration at all, given there are no users?** The owner has confirmed (2026-07-14) that back-compat is
not a concern. **It is not a stranger's vault this protects — it is the owner's own.** Every vault touched
during this program holds a blob whose `blind` and `salt_use_sales_tax` are bare `false`. The instant those
become `Option<bool>`, serde reads that `false` as `Some(false)` — *"the filer answered No"* — when nobody was
ever asked. **That is the D-8 laundering, re-armed inside the spec that abolishes it**, and it would land in a
real return, silently forfeiting a §63(f) deduction.

So "no back-compat" does not license *no migration*. **It licenses a DESTRUCTIVE one** — we may discard a
stale value outright rather than translate it (see the HSA rule below). What it does *not* license is a hard
**reject** of pre-P9 blobs: the docs tell the filer to delete their input TOML after import and `income show`
masks PII, so refusing to load would **brick a real year with no recovery path** — *precisely the "bricks a
valid return" failure this spec caught itself committing in §2.4.* Unlaundering re-asks through `income
answer`, which is exactly why that command exists.

⇒ **`SCHEMA_VERSION = 2`.** Three fields change type, and they do **not** all migrate the same way — because
**one of them also changes its NAME, and that changes everything** (r3 I-1).

#### The three SAME-NAME keys — an in-memory unlaunder

`Person.blind` (taxpayer **and** spouse) and `ScheduleAInputs.salt_use_sales_tax` keep their JSON keys. A
stored `false` deserializes to `Some(false)`, so `row_to_inputs` must fix it in memory:

`version < 2 ⇒ unlaunder(taxpayer.blind), unlaunder(spouse.blind), unlaunder(schedule_a.salt_use_sales_tax)`

Each is **mutation-checked** (delete one ⇒ a NAMED test fails), with the fixture built the P8a I2 way: rewrite
**every** key it claims to cover, and **assert each rewrite lands**.

#### ★ The RENAMED key — `hsa_present` → `hsa_activity` — and why r3's `unlaunder` was DEAD CODE

`row_to_inputs` **deserializes first, then fixes fields in memory** (`return_inputs.rs:50-66`). A v1 blob has
`"hsa_present": false|true`; the new struct has no such field, and with no `deny_unknown_fields` **serde drops
it on the floor** and defaults `hsa_activity` to `None`. So r3's `unlaunder(sch1.hsa_activity)` operated on a
key **that does not exist in any v1 blob**: it was dead code, its mandatory mutation-check was
**unsatisfiable** (deleting dead code fails no test), and the P8a fixture pattern **cannot even build the
fixture** — it rewrites keys of the *current* struct, which no longer emits `hsa_present`.

**★ THE FIX — and it is NOT the reviewer's.** Fable proposed reading `hsa_present` from the raw JSON and
mapping `true → Some(true)`, to preserve the filer's typed answer. **That is wrong, and it re-arms r2's
Critical.** `hsa_present` and `hsa_activity` **are not the same question**:

- `hsa_present = true` said *"I hold an HSA."*
- `hsa_activity = Some(true)` says *"a Form 8889 trigger fired"* — which the filer **never told us**.

A **dormant**-HSA holder — the exact population §2.4 exists to un-brick — typed `hsa_present = true`
truthfully. Under `true → Some(true)` they are **permanently refused as unsupported**, having never claimed
any activity. *The migration would restore the brick that the field's re-scoping was written to remove.* An
answer to a different question is **not** an answer; carrying it across is not preservation, it is
**mistranslation**.

⇒ **BOTH v1 values map to `None`: the filer is RE-ASKED.** Nothing is laundered (the year refuses as
*unanswered* — it cannot silently compute), and `income answer` is the exit. This is the destructive migration
the no-users mandate licenses, and it is the **conservative** direction.

**★ And that means this key has NO unlaunder to mutate — so the guard has to be a different shape, and the
spec must say so rather than claim a mutation-check it cannot perform.** Since serde's unknown-key drop already
yields `None`, correctness here is held by an **absence**, not by code. The mutation that would break it is an
**addition**:

> **`#[serde(alias = "hsa_present")]` is FORBIDDEN on `hsa_activity`.** An alias would make serde read the v1
> `false` as `Some(false)` — resurrecting the laundering — and read the v1 `true` as `Some(true)` —
> resurrecting the brick. **One attribute re-arms both Criticals this field has already produced.**

The named v1-blob test asserts a literal v1 JSON (`"hsa_present": false` **and** `: true`) loads as
`hsa_activity == None`, **and its mutation is `add the alias` ⇒ the test fails.** The fixture must own the
*old* key (build it by rewriting `"hsa_activity":null` → `"hsa_present":false`, asserting the rewrite lands) —
it **cannot** be built by serializing the current struct, which is what r3's cited pattern would have done, and
it would have passed vacuously.

*(A TOML that **explicitly writes** `blind = false` is a **typed answer** and correctly loads `Some(false)`.
Only the **stored-row** path launders. `dual_status_alien` and the mortgage flag need no migration — they are
new fields, and `None` on an existing row is honest. `hsa_present` in a TOML becomes an **unknown key** and
refuses under §2.3 — correct: it names a question we no longer ask.)*

**Forward guard (r3 Nit-2):** `row_to_inputs` gains `version > SCHEMA_VERSION ⇒ typed error`. P9 creates the
first-ever version skew; a v2 blob silently half-read by an older binary is exactly the class this spec closes.

### 2.7 ★ Schedule A line 8 — the mixed-use mortgage box (Fable r2 I-4)

*"If you didn't use all of your home mortgage loan(s) to buy, build, or improve your home… check this box."*
We fill 8a from `mortgage_interest_1098` and **never touch the box** (the Schedule A filler writes exactly two
checkboxes). **A single box, printed unchecked on every itemizing return with a mortgage.**

Under **§163(h)(3)(F)** (2018–2025), interest on proceeds **not** used to buy/build/improve is **not
deductible at all**. So an unchecked box beside a full 8a deduction is **an unaffirmed statement AND an
understatement** — identical in shape to §2.5, and caught by the §2.1 criterion.

⇒ **NEW `ScheduleAInputs.mortgage_all_used_to_buy_build_improve: Option<bool>`.** New field ⇒ no migration.

**★ But the two obligations live at DIFFERENT LAYERS, and r3 collapsed them into one — bricking a truthful
filer (r3 I-2).** Schedule A **prints only when the deduction is itemized** (`printed.rs:1082-1086`:
`if !ar.deduction_is_itemized { return None }`). r3 made `Some(false)` refuse whenever `schedule_a` is present
with mortgage interest — so a filer who supplies Schedule A inputs *to let `Auto` compare*, is genuinely
mixed-use, **and whose STANDARD deduction wins**, is asked, answers **"No" truthfully**, and is **refused** —
though their return prints no Schedule A, no line 8, no box, and **btctax computes it correctly today.** Their
only exits are to delete legitimate inputs or lie. *Post-TCJA, standard-wins is the COMMON outcome for a
mortgaged household exploring itemizing* — and §4 promises **"no valid return is bricked."*

⇒ **Split by layer:**

1. **The QUESTION is input-level** (registry): live on `schedule_a.is_some() ∧ mortgage_interest_1098 > 0`.
   Unanswered ⇒ **refuse** (`MixedUseMortgageUnanswered`). This **over-asks** the standard-deduction filer —
   recorded, and always answerable, so it can never brick.
2. **The VALUE-refusal is compute-dependent**: `MixedUseMortgageUnsupported` fires in
   `screen_compute_dependent` (`return_1040.rs:546`) — **only when the itemized path is actually selected**
   (`Auto` picks Schedule A, or `ForceItemize`).

**Why that split is exactly right, not merely convenient:** full-8a is an **upper bound** on the true itemized
total (the correct mixed-use figure is *smaller*). So when even the upper bound **loses** to the standard
deduction, the standard return is right regardless of the allocation, and we need not know it. When it
**wins**, we cannot know the deduction without Pub. 936 — and must refuse. The refusal detail names the benign
exit ("your standard deduction wins anyway — remove the Schedule A inputs, or answer as your allocation
requires").

⚠️ **Recorded — "files" is not one predicate.** For Schedule B, `schedule_b_files(ri)` is a pure *input*
predicate. For Schedule A, "files" is **compute-dependent** (which deduction wins), which a
`fn(&ReturnInputs) -> bool` liveness **cannot express**. That asymmetry is why the question's liveness is
`schedule_a.is_some()`, not "Schedule A files."

### 2.8 The class boundary is wider than `bool` — defaulted ENUMS

| field | default | status |
|---|---|---|
| `ScheduleCInputs.accounting_method` | `Cash` | `"accrual"` accepted, unmodeled, **unrefused**, and it **flips the printed Sch C line F** — *already filed → P8* |
| `ReturnInputs.itemize_election` | `Auto` | **class (C) — exempt (r3 M-4).** *Reason:* `Auto` takes the **larger** of standard vs Schedule A — an **optimization**, not an assertion and not a forgone benefit (it cannot lose money by construction). The §63(e) `ForceItemize` election is opt-in. The assertion hazards near it are carried by the §2.5 and §2.7 questions, not by this field. *(r3 M-4: r2 left it "interacts with §2.5/§2.7" — which is not a classification, and the §3.3 classifier will demand one at step 10.)* |
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
| `MortgageAllUsedToBuyBuildImprove` | **`schedule_a.is_some()` ∧ `mortgage_interest_1098 > 0`** — an *input* predicate, deliberately **not** "Schedule A files" (which is compute-dependent and would brick the standard-wins filer — §2.7, r3 I-2). A recorded over-ask. |

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
one). *(Fable verified the two kinds cannot double-report: `None` and `Some(true)` are disjoint.)*

| refusal | trigger | layer |
|---|---|---|
| `ForeignTrustUnsupported` | `foreign_trust == Some(true)` | `screen_inputs` |
| `DependentSpouseUnsupported` | `can_be_claimed_as_dependent_spouse == Some(true)` | `screen_inputs` |
| `HsaActivityUnsupported` | `hsa_activity == Some(true)` | `screen_inputs` |
| `DualStatusAlienUnsupported` | `dual_status_alien == Some(true)` | `screen_inputs` |
| `SalesTaxElectionWithoutAmount` | `salt_use_sales_tax == Some(true)` ∧ `salt_sales_tax_amount == 0` ∧ any income-tax SALT input > 0 (§2.2) | `screen_inputs` |
| **`ScheduleBForeignCountryMissing`** (r1 I-6, **named at last** — r3 M-2) | `foreign_accounts == Some(true)` ∧ `foreign_country_names.trim().is_empty()` — Sch B **7a "Yes" with a blank 7b**. Detail names **`income import`** as the remedy: `answer` captures bools and dates, **never strings** | `screen_inputs` |
| **`MixedUseMortgageUnsupported`** | `mortgage_all_used_to_buy_build_improve == Some(false)` | **★ `screen_compute_dependent`** — **only when the itemized path is selected** (§2.7, r3 I-2). *The one value-refusal that is NOT input-level, because Schedule A's filing is compute-dependent.* |

**`income answer`.** `live_questions`/`current_bool`/`set_bool` become registry iteration. The `Question` enum
**shrinks to the DOB residue** (r1 M-1) — it does not delete. The no-brick property (*everything the screen can
refuse for is askable*) becomes true **by identity**.

It also gains the class-(B) **skippable** prompts (§2.2): `blind` (taxpayer + spouse) and `salt_use_sales_tax`
(**only when `schedule_a` exists** — §2.2's footgun scope). Empty input ⇒ stays `None` ⇒ the advisory fires.

**★ BOTH spouse prompts are gated on `header.spouse.is_some()` — DOB *and* blind (r3 I-7).** The spouse-DOB
prompt already carries this gate because `set_date` **silently discards** a spouse DOB when there is no spouse
`Person` (`answer.rs:147-151`). **A spouse-`blind` set discards identically** — so an ungated prompt would take
the filer's typed answer, throw it away, and leave the on-`None` advisory nagging **forever**: the exact
"fires forever" failure r2 I-2 was graded Important for. r3 specified the gate on one twin and omitted it on
the other, *which is precisely the shape of P8a I1.* Both are gated, in one sentence, here.

*(Note the deliberate asymmetry: the spouse **question**'s liveness widens to `Mfj || spouse.is_some()` (§3.1)
because the 1040's box is joint-only and a missing spouse `Person` must not hide it; the spouse **prompts** are
narrower because a prompt with nowhere to write is worse than no prompt. Different jobs, different scopes.)*

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
`first_negative_amount`" was a **false floor**: `fna` waives the whole header with `header: _`
(`return_refuse.rs:204`), so it recurses into **neither `Person` nor `Dependent`**, which is where half the
bools live). The reachable set is `HouseholdHeader`, `Person`, `Dependent`, `W2`, **`Box12Entry`**,
`Form1099Int/Div/G`, `ScheduleAInputs`, `Schedule1Inputs`, `ScheduleCInputs`, `QbiInputs`, `Payments`,
`CharitableGift`, `CharitableCarryItem`, **`Carryforward`** — *(the last two added by r3 M-1: both are
reachable (`W2.box12: Vec<Box12Entry>`, `capital_loss_carryforward_in: Carryforward`) and neither has a
classifiable leaf **today**, which is exactly why their destructures must exist — the guarantee is about the
bool added **tomorrow**. Destructuring the FROZEN `Carryforward` reads it; it modifies nothing.)*

**★ The `_` rule (r2 M-6)** — "no `_` at all" is impossible (every `String`/`Usd`/`Date` leaf must bind
*something*):

> **`_` — and every `_`-prefixed binding — is FORBIDDEN** on structs and collections (must **recurse**) and on
> `bool` / `Option<bool>` / defaulted-enum leaves (must **classify**).
> **`_` is permitted on other scalar leaves** (`String`, `Usd`, `Date`, …).

**★ What actually holds it, and what does not (r3 I-5).** r3 claimed the classifier makes a new bool *"not
compile until registered or exempted with its class and statutory reason."* **It does not, and Fable proved it
by compiling the counterexample:** `w2s: _w2s` is not `_`, recurses into nothing, and builds clean. **One token
defeats the rule as r3 stated it.**

⇒ The teeth are a **lint**, and they must be named: the classifier module carries **`#![deny(unused_variables)]`**.
Then a named binding that ignores its field (`w2s: whatever`) is a **hard compile error**, and the `_`-prefix —
the one spelling that *suppresses* that lint — is forbidden by the rule above.

⚠️ **The honest limit, and §4 must not overclaim past it.** Even so, the compiler holds
**"a new field does not compile until a human EDITS the classifier"** — not *"until they classify it
correctly."* The residual evasions (`_`-prefixed bindings, `let _ = x;`) are **grep-able review residue**, not
compile errors. *That is the whole guarantee, and it is worth having: every recurrence of this class began with
a field nobody looked at.* The open `header: _` follow-up is fixed here.

### 3.4 The advisories the owner mandate requires (§2.2)

```rust
/// §63(f): a person was never asked about blindness, so the additional standard deduction was not granted.
/// Fires on None (NEVER ASKED) — never on Some(false). Same statute, worksheet line, and dollars as
/// `AgedBoxForfeitedNoDob`, and the two STACK.
BlindBoxForfeitedNotDeclared { per_box: Usd, persons: usize },
/// §164(b)(5): the sales-tax election was never asked, so SALT used income tax. Fires when the election is
/// None AND a Schedule A exists (see the scope note — NOT "only when the return itemizes").
SalesTaxElectionNotAsked,
```

**★ `persons` — whose blindness counts (r3 I-7).** r3 left this to the implementor, *which is how P8a I1
happened.* The rule mirrors the code that already decides the identical §63(f) question for the **aged** box —
`AgedBlindBoxes::for_return` counts the spouse's boxes **only on a joint return** (`packet.rs:200-216`), and
the aged advisory adds P5-M2's rule that **an absent spouse record on MFJ forfeits the box just as surely** as
a missing birthday:

> **`persons` = `[taxpayer.blind.is_none()]` + `[filing_status == Mfj ∧ (spouse absent ∨ spouse.blind.is_none())]`**

**MFS never counts the spouse box** (the spouse's blindness is not this taxpayer's checkbox), so an MFS filer
is never nagged about a box that cannot exist. On MFJ, a **missing spouse `Person`** counts — there is no
`blind` field to be `None`, and forfeiting for want of a *record* is still forfeiting. Advisory fires iff
`persons > 0`.

**★ SALT scope (r3 Nit-3).** r3 scoped this to *"only when the return ITEMIZES."* **That is wrong at exactly
the boundary that matters:** an unasked sales-tax election can be the very thing that **flips** a near-standard
return into itemizing — and under r3's scope the return takes the standard deduction, does not itemize, and so
**stays silent about the election that would have changed the answer.** *The advisory would go quiet in the one
case where it is worth money.* ⇒ Scope it to **`schedule_a.is_some()` ∧ election `is_none()`** — the filer has
told us they have Schedule A items, so SALT is live for them whichever deduction currently wins.

### 3.5 The tests

**Per-question property test** — for each `FormQuestion`: build a return where it is live; blank it ⇒ assert
`screen_inputs` refuses **with that entry's `RefuseReason`**; answer it (`n` **and** `y`) ⇒ assert **that
entry's `unanswered` reason no longer fires**; assert `income answer` **asks** it.

⚠️ **★ "…no longer fires" — NOT "the refusal is gone" (r3 M-5).** For five of the eight questions the *other*
half of the answer space is a **value-refusal**: `y` refuses on `HsaActivity` / `DualStatusAlien` /
`ForeignTrust` / `DependentSpouse`, and `n` refuses on the mortgage question (when itemizing). A naive
`assert!(screen_inputs(..).is_none())` **fails on five of eight** — and the natural "fix" (weaken the assertion
to `is_none() || whatever`) would make **the anti-vacuity machinery itself vacuous**. Assert on the *specific
reason*. *This is the third revision in which a test written to hold this class was itself the hole.*

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
a named test fails; drop a `FORM_QUESTIONS` entry → a named test fails; **drop any of the THREE same-name
`unlaunder`s (`blind` ×2, `salt_use_sales_tax`) → a named test fails** (the P8a I2 lesson: the fixture must
rewrite **every** key and **assert each rewrite lands**); drop either advisory → a named test fails.

**★ The renamed HSA key is mutation-checked in the OTHER direction (§2.6, r3 I-1).** It has **no `unlaunder` to
delete** — serde's unknown-key drop already yields `None`, so correctness is held by an *absence*. Its mutation
is therefore an **addition**: **add `#[serde(alias = "hsa_present")]` ⇒ a named test fails.** The fixture is a
**literal v1 JSON owning the OLD key** (built by rewriting `"hsa_activity":null` → `"hsa_present":false|true`,
asserting the rewrite lands) — it **cannot** be built by serializing the current struct, which no longer emits
that key and would pass **vacuously**. *Both v1 values must load as `None`.* Say the guard's shape out loud
rather than claim a mutation-check that cannot be performed.

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
- **★ NO VALID RETURN IS BRICKED — the guarantee this spec has now violated TWICE (r2 C-1, r3 I-2).** Three
  named regression tests, not a slogan:
  1. a **dormant**-HSA holder answers "no" to all four §2.4 clauses truthfully ⇒ their return **computes**;
  2. a **mixed-use-mortgage filer whose STANDARD deduction wins** answers "no" truthfully ⇒ their return
     **computes** (the refusal is compute-dependent — §2.7);
  3. a v1 blob carrying `hsa_present = true` (a **dormant** holder who typed it) loads as `None` and is
     **re-asked** — it is **not** ratified into `Some(true)` and refused (§2.6).
- **★ The HSA question covers ALL FOUR Form 8889 triggers** (§2.4) — contribution *by anyone*, distribution,
  **testing-period failure with no activity at all**, and inheritance. *This bullet is the one r3 got wrong; it
  is stated as a checkable claim precisely so the next reviewer can attack it.*
- **★ The v1→v2 migration holds** (§2.6): a stored `false` for `blind` ×2 / `salt_use_sales_tax` loads as
  `None`, not `Some(false)` — **mutation-checked per key**. The **renamed** `hsa_present` loads as `None` for
  **both** `false` and `true` — held by a literal-v1-JSON test whose mutation is **adding the forbidden
  `serde(alias)`**.
- **A new `bool` / `Option<bool>` / defaulted enum anywhere reachable from `ReturnInputs` does not compile until
  a human EDITS the classifier** — `#![deny(unused_variables)]` makes an ignoring named binding a hard error.
  *(★ r3 I-5: the guarantee is "a human must look", NOT "it is classified correctly." The exemption register
  carrying class + statutory reason is held by **review**, and `_`-prefixed bindings / `let _ = x;` are
  **grep-able residue**. Stated at its true strength — the previous wording claimed a force the mechanism does
  not have, and Fable defeated it in one token.)*
- **The owner mandate holds: no benefit is forgone in silence.** `blind` and `salt_use_sales_tax` each fire a
  mandatory advisory naming the money, **on `None` only** — never nagging a filer who answered. `persons`
  follows the §3.4 formula (MFJ-only spouse box; an **absent** MFJ spouse still forfeits).
- **The SALT prompt cannot silently zero line 5a** (`SalesTaxElectionWithoutAmount`).
- **`dual_status_alien` and the Schedule A line 8 box exist, are asked, and refuse when unanswered.**
- **The three DEAD fields are deleted, AND `income import` refuses unknown TOML keys** — via **`serde_ignored`**,
  not a hand-written key list (§2.3). It must reject `hsa_present` and each deleted field, **naming them**.
- **Schedule B 7a "Yes" with a blank 7b refuses** (`ScheduleBForeignCountryMissing`), and its detail names
  `income import` as the exit (`answer` cannot capture strings).
- **★ The checkbox census is CLOSED** (§2.1): every printed box on all **fourteen** forms is either a registry
  question, a class-(B) advisory, a pen-deferred pair, or a **written rule-3 exemption with a `LIMITATIONS.md`
  entry**. The Schedule D **QOF "No"** — the only box software answers on the filer's behalf — gets the
  exemption and the entry it never had.
- **MFJ with no spouse identity ⇒ `ReturnHeader::build` REFUSES** (r3 M-6 — *decided, not deferred*; it is
  already inside step 7's blast radius). The `Mfj || spouse.is_some()` liveness makes the question live in that
  case, so the print boundary must be able to say so.
- Every guard above is **mutation-checked** — *including the ones whose mutation is an **addition** (§3.5).*
- `make check` green; **0 Critical / 0 Important** from independent review.
- FROZEN (`tax/{types,compute,se}.rs`) unchanged. `screen_inputs` keeps its signature; `resolve.rs` untouched.

## 5. Build order (TDD; each step red → green)

*★ r2's order did not **compile** (r2 I-5): steps 1–3 referenced fields step 4 created — fixed in r3.*
*★ r3's order did not stay **green** (r3 I-6): its step 4 ended **red**, because the property test cannot pass
until step 5 rewires `screen_inputs`. **Merged below.** "Red → green" is a contract about each step's CLOSE,
and a spec that ends a step red has planned a blocking finding into the schedule.*

1. **Fields first.** `sch1.hsa_present: bool` → `hsa_activity: Option<bool>` (§2.4 — a **rename**, not just a
   type flip: that is what makes step 2 subtle); `Person.blind` and `ScheduleAInputs.salt_use_sales_tax` →
   `Option<bool>` (§2.2); **NEW** `ReturnInputs.dual_status_alien` and
   `ScheduleAInputs.mortgage_all_used_to_buy_build_improve` (§2.5, §2.7).
   **⚠️ Churn warning (r3 Nit-4):** `HsaActivity` and `DualStatusAlien` are live on **every** return, so every
   computing fixture in four crates must answer them. **Default them in the `testonly` builders** — one line
   there instead of two hundred call sites. *(One TUI test asserts the HSA refusal text: `tabs/tests.rs:1704`.)*
2. **★ The migration — `SCHEMA_VERSION` 2** (§2.6). Three in-memory unlaunders (`blind` ×2, `salt`), each
   mutation-checked; the **renamed** HSA key held by a **literal-v1-JSON** test (both `false` **and** `true` ⇒
   `None`) whose mutation is *adding* `serde(alias)`; plus the forward-version guard (Nit-2).
   *(It lands here — before the phase's gate and before any new consumer trusts these fields — so the
   laundering can never ship. r3's note said "before any consumer reads the new types", which was confusing:
   step 1 already made every existing consumer read them.)*
3. **`questions.rs`** — `QuestionId` (+`ALL`), `FormQuestion`, `FORM_QUESTIONS` (8 entries). Liveness lifted
   from the current refusals **except** `DependentSpouse`, corrected to `Mfj || spouse.is_some()` (**= P8a I1**).
4. **★ The anchor + the property test + `screen_inputs`, as ONE red→green step** (§3.5, §3.2 — r3 I-6). Write
   the completeness anchor and the per-question property test **red**; derive `screen_inputs` from the registry;
   delete the hand-written unanswered-blocks; add the new value-dependent refusals; **green.** Fix multi-defect
   fixtures that asserted a specific reason (precedence is explicitly not contract). *This is the step that
   turns the P8a-I1 case green — it cannot be split from the test that proves it.*
5. **`cmd/answer.rs`** derives from the registry; the enum shrinks to the DOB residue; adds the class-(B)
   skippable prompts (SALT scoped to `schedule_a.is_some()`; **both** spouse prompts — DOB *and* blind — gated
   on `header.spouse.is_some()`, §3.2/r3 I-7).
6. **`ReturnHeader::build`** → `HeaderError`; update `admin.rs` mapping and all fixtures; **MFJ-with-no-spouse
   refuses** (§4, r3 M-6).
7. **The two advisories** (§3.4) — the owner mandate. Fire on `None` only; `persons` per the §3.4 formula;
   SALT scoped to `schedule_a.is_some()` (**not** "itemizes" — r3 Nit-3).
8. **`SalesTaxElectionWithoutAmount`** (§2.2) and **`ScheduleBForeignCountryMissing`** (r1 I-6, named in §3.2).
9. **`MixedUseMortgageUnsupported` in `screen_compute_dependent`** (§2.7, r3 I-2) — with the
   **standard-deduction-wins regression test** that proves it does not brick.
10. **The classifier** (§3.3): recursing over every reachable struct (incl. `Box12Entry`, `Carryforward`);
    `#![deny(unused_variables)]`; the `_`-and-`_`-prefix rule; the exemption register with class + statutory
    reason (incl. `itemize_election`, §2.8). Folds the open `header: _` follow-up.
11. **DELETE the three dead fields** (§2.3) **AND add `serde_ignored` unknown-key rejection to `income
    import`** — the two ship **together or not at all**; the rejection must name `hsa_present` too.
12. `LIMITATIONS.md` — the new refusals; the two advisories; **the Schedule D QOF "No" exemption** and the
    **Schedule C G/H/I/J blank boxes** (§2.1 — neither was ever disclaimed); that Sch B / MFS refusal texts now
    name `btctax income answer` (a deliberate **improvement**); that `income answer` cannot capture strings.
