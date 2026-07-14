# SPEC — P9: the FORM QUESTION REGISTRY

*Status: **r7**. Folds Fable spec review r6 (0C/1I/2M/5Nit — `reviews/P9-SPEC-fable-r6.md`), r5
(`reviews/P9-SPEC-fable-r5.md`), r4
(`reviews/P9-SPEC-fable-r4.md`), r3
(`reviews/P9-SPEC-fable-r3.md`), r2 (`reviews/P9-SPEC-fable-r2.md`) and r1 (`reviews/P9-SPEC-fable-r1.md`).*
*Two OWNER decisions (2026-07-14) are folded here and are not review findings: the **hard-refuse migration**
(§2.6) and the **circular-liveness bug the owner's question found in shipped code** (§2.9). r5 adjudicated
**both sound** and **verified the §2.9 bug end to end against shipped code.***
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

And **r4 — which added the "COMPLETE" checkbox census — published it complete while the 8949 filler was
checking Box I/L on every filed return**, on a form the sweep's own row swore was checkbox-free
(`fill8949.rs:86-95`; r4 I-1). *The census built to close the class had a box nobody looked at.*

And **r5 — which replaced the migration with a refusal — gave that refusal a remedy that DOES NOT WORK**:
it told the filer to run `btctax income import`, and `income import` **reads the stale row itself**
(`cmd/tax.rs:63`), so the named exit errors on every row it exists to cure (r5 I-1). *"A refusal with no exit
is just a brick with better prose"* — **this document's own words, in the section that quotes them.**

And **r6 — which fixed the broken remedy — traced it exactly one command short of the data**: `clear` +
`import` unbricks the refusal and then leaves year+1 without the `Computed` carryover it was redesigned to
protect, with the rebuild command named nowhere (r6 I-1).

**Seven revisions; seven recurrences; each one authored by someone who had just finished writing that this
exact thing keeps happening.** The class does not respect the author's intentions, his attention, or his
sincerity. **It is only closed by construction** — which is why §3.3's honest limit (r3 I-5) matters more
than any prose in this document.

### 1.2 ★ AND IT IS NOT ONLY IN THE SPEC — the OWNER found a live one in SHIPPED code

The owner asked a design question — *"are we using the 'does not apply' criterion to our advantage?"* — and
the act of checking the answer found **a sixth instance, in v0.2.0, today** (§2.9). `schedule_b_files` is the
liveness predicate for the foreign-account and foreign-trust declarations, **and the foreign-account answer is
one of its own inputs.** Unanswered, it resolves to a fixed point that omits **an entire required schedule**.

**Note what that means about the defect's shape.** Every prior instance was a *value* with a bad default (a
bare `bool`). This one has the tri-state **already** — `foreign_accounts: Option<bool>` — and it is **still**
laundered, because the **liveness predicate** defaults instead. *The class is not "a bool that should be an
Option." The class is: **anything that can silently answer for the filer.** A predicate can do it too.* Any
future statement of this defect that says "use a tri-state" has, again, learned nothing.

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
>    theirs — is permitted **only by a WRITTEN EXEMPTION** naming what makes the answer *entailed*, **plus a
>    `LIMITATIONS.md` entry.** Silence is not an exemption. Two grades of entailment, and they carry different
>    weight (r4 M-2):
>    - **entailed by POSITIVE DATA** — the record itself proves the answer *(1040 digital-asset "Yes"; Form
>      8283 line k)*. The exemption register alone documents these.
>    - **★ entailed by SCOPE** — the answer follows only from *"a filer to whom this is false is outside the
>      set of returns btctax supports"* *(Schedule D QOF "No"; Form 8949 Box I/L)*. **These are the dangerous
>      ones** — the entailment is a claim about the *user*, not about the *data*, and it is silently false the
>      moment someone outside the archetype runs the program. **Each REQUIRES a `LIMITATIONS.md` entry
>      (step 11).**

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
| Schedule C G / I / J | (—) | pairs — deferred to the pen ⇒ **`LIMITATIONS.md` entry (step 11)** |
| Schedule C H (started/acquired business) | (C) | single box, **no tax direction** ⇒ **`LIMITATIONS.md` entry (step 11)** *(r3 I-4: r2 called it "disclaimed"; no such disclaimer existed)* |
| **Schedule D QOF pair — "No" CHECKED unconditionally** (`schedule_d_full.rs:283-294`) | **exempt (rule 3)** | **★ the only box software answers in the NEGATIVE.** Entailed by **scope**: btctax supports returns whose dispositions all come from the bitcoin ledger, and a QOF disposition has no input and no ledger representation — a filer with one is outside the supported set (P6 r1 I4). **Requires the `LIMITATIONS.md` entry it never had (step 11).** *(r4 I-1: r4 called this "the only box software answers" — false; see 8949 below.)* |
| Schedule D 17 / 20 / 22 pairs | — | genuinely data-derived from computed lines |
| **★ Form 8949 Box I / Box L — CHECKED on every filed 8949 with rows** (`fill8949.rs:86-95`, via `packet.rs:129-134` → `fill_8949_parts_with_identity`) | **exempt (rule 3)** | **★ r4 I-1 — THE BOX NOBODY LOOKED AT.** r4's census row said this filler *"writes no checkboxes at all"*, **with the word "verified".** It writes one on every return with a disposal. Box I (ST) / Box L (LT) = *"not reported to you on Form 1099-B"* — **never Box C/F** (`fill8949.rs:4-6`). Entailed by **data + scope**: btctax has no 1099-B / 1099-DA input at all, so every ledger disposition is by construction un-reported. **Needs its `LIMITATIONS.md` entry (step 11).** |
| Form 8283 line k (digital assets) | **exempt (rule 3)** | checked — entailed by data (the donation *is* crypto) |
| Form 8283 donee acknowledgment / restricted use | (—) | deferred blank by the module's fill/blank table (`form8283.rs:1-10`) |
| Form 8960 §6013(g)/(h) · §1.1411-10(g) | (C) | **opt-in elections** — lawful unchecked |
| Schedules 1 / 2 / 3, SE, 8959, 8995 | — | **verified (twice, independently): these fillers write no `FieldValue::Check` at all** |
| **STRUCTURALLY INAPPLICABLE — boxes btctax never writes** (r4 I-1): 1040 third-party-designee pair; 1040 L6c (lump-sum election — no SSA input exists); 1040 L35a (Form 8888) + L35c checking/savings pair (v1 never fills direct deposit — `RefundByPaperCheck` advisory); Schedule 2 L8 "not required" box (L8 is never fillable); Schedule SE line A (§1402(e) ministerial — not representable); Schedule C 32a/32b (a Sch C loss refuses upstream — `ScheduleCLoss`) | **(—)** | **A NAMED CATEGORY, not an omission.** Each is a box whose *input does not exist in the model*, so no answer — ours or the filer's — can reach it. The designee pair is pen-deferred (rule 1). **This row exists so the census is closed by ENUMERATION, not by silence.** |

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
>
> *(Don't count moving money between your own HSAs — a trustee-to-trustee transfer is not a contribution or a
> distribution. **r4 Nit-2**: clause (a) otherwise reads as covering it, and it triggers no Form 8889 —
> Notice 2004-50 Q&A-55, **persuasive**. A ROLLOVER, by contrast, correctly refuses via clause (b).)*

⚠️ **One clause is CONSERVATISM, not necessity, and the difference is recorded (r4 M-1).** A surviving
**spouse** who inherits an HSA answers clause (c) "yes" and is refused — but **§223(f)(8)(A)** (*statute*)
simply *treats the spouse as the account beneficiary*: no income inclusion, and on an activity-less year
arguably **no Form 8889 at all**. Only the Form 8889 instructions' "Who Must File" bullet (*persuasive, not
law*), read literally, requires the form. We refuse anyway — an **over-ask** is recoverable and an under-ask is
not — but under this project's [[tax-authority-hierarchy]] discipline the spec must not dress a conservative
choice up as a statutory command. *Refusing here is a decision, not a deduction.*

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

### 2.6 ★ THE MIGRATION — there isn't one. `SCHEMA_VERSION = 2`, and a stale row REFUSES

***OWNER DECISION, 2026-07-14 — not a review finding.*** *The owner confirmed that **no real tax-return data
has ever been entered into this program.** v0.2.0 is on crates.io and unused; every vault in existence holds
**test fixtures**. r4's §2.6 — a per-key unlaundering migration, adjudicated correct by Fable on its own terms
— was **protecting nothing**, and it is deleted.*

**Why the replacement is STRONGER, not merely cheaper.** r4's migration was a **hand-written list of keys to
unlaunder**: `blind` ×2, `salt_use_sales_tax`, plus a bespoke rename rule for `hsa_present`. *That is five
hand-wired obligations that must each be independently remembered on every future field flip* — **the exact
shape §1 indicts.** It is the P9 defect, committed inside P9's own migration. And Fable proved the point: r4
I-4 found that the `blind` ×2 mutation-check **goes vacuous** — both keys serialize as the same string
`"blind"`, the cited fixture base has no spouse, so the spouse unlaunder would have been **held by nothing**
(*the P8a I2 spouse-half failure, rebuilt one struct level deeper, inside the section that quotes P8a I2 as
its lesson*).

⇒ **A version check cannot forget a key.**

```rust
// crates/btctax-cli/src/return_inputs.rs
pub const SCHEMA_VERSION: i64 = 2;

fn row_to_inputs(year: i32, json: &str, version: i64) -> Result<ReturnInputs, CliError> {
    if version != SCHEMA_VERSION {
        return Err(CliError::StaleReturnInputs { year, found: version, expected: SCHEMA_VERSION });
        //  "the stored inputs for {year} predate the form-question registry (schema v{found}; this
        //   build reads v{expected}). Run `btctax income clear {year}` — which DISCARDS any carryover
        //   this row's prior reports computed onto it — then re-run `btctax income import`."
        //  ★ NOT "just re-import": `income import` reads this row too (cmd/tax.rs:63). r5 I-1.
    }
    serde_json::from_str(json).map_err(...)
}
```

**What this deletes, and why each deletion is a gain:**

| deleted | why it can go |
|---|---|
| `unlaunder(taxpayer.blind)`, `unlaunder(spouse.blind)`, `unlaunder(salt_use_sales_tax)` | no v1 row ever loads ⇒ nothing to launder. **r4 I-4 dissolves with them.** |
| the `hsa_present` → `hsa_activity` rename rule, the forbidden `serde(alias)`, the literal-v1 fixture | no v1 row ever loads ⇒ the rename cannot mistranslate. **r3 I-1 / r4's adjudication become moot** *(the reasoning is preserved below — it was right, and it will be needed again)*. |
| **the D-8 `version < 1` unlaunder itself** (`return_inputs.rs:56-64`) | now **dead code**, and §2.3's own doctrine convicts dead code: *"a captured-but-unconsumed field is a lie about what the app honors, and it is pre-armed."* **Delete it.** Its P8a tests are rewritten to assert the v0 row **refuses**. |

#### ★ THE REMEDY MUST WORK — r5 I-1, and the reviewer's fix is REJECTED

**r5's refusal text said *"re-run `btctax income import`"* — and `income import` READS THE STALE ROW.**
`import_return_inputs` opens with `return_inputs::get(s.conn(), year)?` (`cmd/tax.rs:63`) — the §4 R3-M6
**carryover-preservation read** — so the `?` propagates `StaleReturnInputs` and **the named exit errors on
exactly the rows it exists to cure.** The same circle binds `income answer` (`answer.rs:191`) and
`report --write-carryover` (`tax.rs:412/:426`). *A refusal whose stated exit does not work is a brick with
better prose* — §3.1's own doctrine, violated by the section that quotes it.

**★ Fable proposed: let `income import` treat a stale row as ABSENT. REJECTED — it silently re-arms a
fail-open understatement.** That read is not incidental; it exists because **`income import` is a whole-blob
upsert**, and R3-M6 (Fable P4.9 r1 I2) established that dropping a **`Computed`** carryover-in *overstates the
QBI deduction ⇒ **understates tax***. "Treat as absent" would **silently discard** the stale row's computed
carryovers — reintroducing, inside P9's own migration policy, the exact class of silent loss P9 exists to
close. *We do not get to solve a refusal problem by making a different guard fail open.*

⇒ **The mechanism is RIGHT; only the TEXT was wrong.** Every read of a stale row refuses — including
`income import`'s — and the **working** exit is the one command that never deserializes:

> **`btctax income clear <year>`** *(→ `return_inputs::delete`, a bare `DELETE` — `return_inputs.rs:118-122`;
> it cannot refuse)*, **then `btctax income import`** — **and if this row carried a computed carryover, then
> `btctax report --tax-year <year−1> --write-carryover` to REBUILD it.**

**And requiring `clear` first is a FEATURE, not a tax.** It makes discarding the stale row's computed
carryovers an **explicit act by the filer** instead of a silent one by us — which is precisely what R3-M6
demands. The refusal detail must therefore name **all** commands, in order, **and say what `clear` throws
away.**

**★ BUT DISCLOSURE IS NOT RESTORATION — r5 I-1's shape, one square further (r6 I-1).** `clear` + `import`
unbricks the refusal, and then leaves year+1's row **without** the `Computed` charitable carryover and QBI
REIT/PTP carryforward a prior `report --write-carryover` had put there — *the exact fail-open understatement
whose avoidance justified rejecting the reviewer's simpler fix.* The command that rebuilds it exists and works
(`write_carryover` recomputes onto year+1's row from year N's inputs — `cmd/tax.rs:405-450`), **but r6 named it
nowhere**, and the only shipped note that names `--write-carryover` (`cmd/tax.rs:88-95`) is **unreachable after
a `clear`** — it fires only when a row existed to preserve from. *The remedy was traced for the refusal and
not for the data it was redesigned to protect.* ⇒ **The rebuild step is named above, in step 11's LIMITATIONS
entry, and in §4's bullet.**

**★ The rebuild is ALWAYS possible while the policy's premise holds** — the carryforward chain is **depth 1**
in v1 (`write_carryover` requires `full_return_for(year)`, TY2024 only, and writes onto year+1's existing row;
both rows required — `cmd/tax.rs:426`), and every `Computed` carryover is a pure function of the prior year's
inputs, **which survive in the TOML the owner premise guarantees.** *This is one more reason the §5 step 12(b)
expiry follow-up is load-bearing: the first real return, whose filer may have deleted the TOML, breaks the
rebuild — refuse-and-reimport must retire before then.*

**Named tests:** `income import` over a stale row **refuses** (naming `income clear`); `income clear` then
`income import` **succeeds** and the row is v2; **delete the `clear`-first requirement ⇒ a computed carryover
is silently lost ⇒ a named test fails**; **★ the full-chain restoration test** — seed year N, `write-carryover`
onto N+1, mark **both** rows stale, run the whole remedy (`clear`+`import` ×2, then `write-carryover`), assert
N+1's carryover **equals its pre-stale value** ⇒ **drop the rebuild step ⇒ this test fails.**

**The guard is mutation-checked, and non-vacuously** — this is the shape r4 I-4 could not have: insert a row at
`schema_version = 0` and at `= 1`, assert `get` **and** `all` both return `StaleReturnInputs`; **delete the
version check ⇒ both tests fail** (the row loads, and its `blind: false` arrives as `Some(false)` — the
laundering, caught in the act). There is **no key-name ambiguity to go vacuous through**, because there are no
keys.

**Forward skew is the same check** (r3 Nit-2, now free): `version > SCHEMA_VERSION` refuses by the same `!=`.

⚠️ **Cost, stated plainly:** every existing vault must re-run `btctax income import`. **Today that costs
nothing** — it is all test data, and the TOML fixtures are in the repo.

⚠️ **★ THIS POLICY HAS AN EXPIRY, AND IT MUST NOT OUTLIVE ITS PREMISE.** The instant a real return is entered,
"re-import everything" stops being free — and prior-year data (capital-loss and charitable carryforwards, the
QBI REIT/PTP carryforward) is exactly what a real filer cannot reconstruct. **The first real return retires
refuse-and-reimport and requires real migrations.** *Filed as a follow-up owned by the **release gate** — not
by P9, and not by "later".*

#### ★ The reasoning that is NO LONGER NEEDED, but is PRESERVED because it will be

*(r3 I-1, and Fable's r4 adjudication of it — the author was upheld. When real migrations return, this is the
rule they must follow, and it cost two review rounds to establish.)*

> **An answer to a question is not an answer to a DIFFERENT question, and carrying it across is not
> preservation — it is MISTRANSLATION.**
>
> `hsa_present = true` said *"I hold an HSA."* `hsa_activity = Some(true)` asserts *"a Form 8889 trigger
> fired."* Fable proposed migrating `true → Some(true)` to preserve the filer's typed answer; that would have
> **refused the dormant-HSA holder as unsupported** — re-arming the very Critical (r2 C-1) that
> forced this field to be re-scoped. A renamed or re-scoped field's old values must map to **`None` (re-ask)**,
> never to a fabricated answer. *Software cannot supply belief on the filer's behalf* (§2) — and it cannot
> supply it retroactively either.

### 2.7 ★ Schedule A line 8 — the mixed-use mortgage box (Fable r2 I-4, r3 I-2, r4 I-2)

*"If you didn't use all of your home mortgage loan(s) to buy, build, or improve your home… check this box."*
We fill 8a from `mortgage_interest_1098` and **never touch the box** (the Schedule A filler writes exactly two
checkboxes). **A single box, printed unchecked on every itemizing return with a mortgage.**

Under **§163(h)(3)(F)** (2018–2025), interest on proceeds **not** used to buy/build/improve is **not
deductible at all**. So an unchecked box beside a full 8a deduction is **an unaffirmed statement AND an
understatement** — identical in shape to §2.5, and caught by the §2.1 criterion (a single box whose *unchecked*
state asserts).

⇒ **NEW `ScheduleAInputs.mortgage_all_used_to_buy_build_improve: Option<bool>`** — a class-(A) registry
question, live on **`schedule_a.is_some()` ∧ `mortgage_interest_1098 > 0`**. New field ⇒ no migration.

#### ★ What `Some(false)` DOES — and why it is no longer a refusal at all

**Two revisions have now failed to make the refusal work, and the second failure explains the first.** r3
refused on `Some(false)` outright — **bricking the truthful mixed-use filer whose STANDARD deduction wins**,
whose return btctax computes correctly today and prints no Schedule A at all (r3 I-2). r4 moved the refusal to
`screen_compute_dependent` so it would fire *only when itemizing* — but Fable then proved (r4 I-2) that this
function **cannot see the fact it is told to fire on**: its signature is `(ri, state, year, params)` — no
`TaxTable`, no `AbsoluteReturn` — while `deduction_is_itemized` is computed mid-`assemble_absolute`
(`return_1040.rs:1143`). Worse, **"itemized was selected" is not one fact**: `derive_tax_profile` computes its
*own* Schedule A total on **non-crypto** AGI (`return_1040.rs:686/:730`), so the delta report can itemize while
the absolute return takes the standard deduction. Implementing r4 would have meant a **second, drifting copy**
of the deduction decision — forbidden by this codebase's own doctrine (*the count and the amount must come from
one derivation*).

**★ The right move is not to fix the refusal's plumbing. It is to notice the refusal was never necessary.**

A refusal is for *"we cannot produce a correct return."* But we **can**: **§163(h)(3)(F) makes the
non-acquisition portion non-deductible, and the filer has told us there IS one.** We do not know the Pub. 936
allocation — so we claim **none of it**, and we **say so**:

| answer | Schedule A line 8a | the box | advisory |
|---|---|---|---|
| `Some(true)` | full `mortgage_interest_1098` | **unchecked** — the filer affirmed it | — |
| **`Some(false)`** ∧ `mortgage_interest_1098 > 0` | **`0`** | **CHECKED — truthfully, for the first time** | **`MixedUseMortgageNotAllocated` — MANDATORY (§3.4)** |
| `Some(false)` ∧ `mortgage_interest_1098 == 0` | — (not printed) | — | **none (r6 Nit-3)** — the question is not live, and a $0-interest answer forgoes nothing; the box-check and the advisory are BOTH scoped to the live predicate, never to the bare field value |
| `None` | — | — | **REFUSE** (`MixedUseMortgageUnanswered`) |

**Why this is correct and not merely convenient:**

- **It cannot brick anyone.** No compute-dependent layer, no second derivation, no `screen_compute_dependent`
  signature change. **r4 I-2 does not need fixing — it needs deleting.** The standard-wins filer computes; so
  does the itemizing one.
- **It cannot understate tax.** $0 ≤ the true allocation, always. The one direction that was ever dangerous
  here (full 8a beside an unchecked box) is now impossible: `Some(false)` zeroes the line *and* checks the box.
- **It is lawful.** *New Colonial Ice* — the burden to **claim** is the taxpayer's, and they have not given us
  the substantiation. Claiming **less** than one's entitlement is always permitted; claiming more is not.
- **It prints a MORE truthful form than today.** Today the box is unchecked on every mixed-use return — a false
  statement. Now it is checked whenever the filer says so.
- **And it obeys the owner mandate** (§2.2): the benefit is forgone, so the filer is **told**, in dollars.

⚠️ **The honest cost, recorded:** a filer with a $500k acquisition mortgage and a $20k car HELOC gets **$0** of
mortgage interest instead of ~96% of it. That is a **materially** overstated tax, and the advisory must say so
in those words. **The cure is an input we do not have** — the Pub. 936 worksheet result. ⇒ **Follow-up, owned
by P8** (the input-surface phase): capture `mortgage_interest_deductible` so a filer who *has* done the
worksheet can enter its result, and 8a takes it. **P9 closes the false statement; P8 recovers the money.**

### 2.8 The class boundary is wider than `bool` — defaulted ENUMS

| field | default | status |
|---|---|---|
| `ScheduleCInputs.accounting_method` | `Cash` | `"accrual"` accepted, unmodeled, **unrefused**, and it **flips the printed Sch C line F** — *already filed → P8* |
| `ReturnInputs.itemize_election` | `Auto` | **class (C) — exempt (r3 M-4).** *Reason:* `Auto` takes the **larger** of standard vs Schedule A — an **optimization**, not an assertion and not a forgone benefit (it cannot lose money by construction). The §63(e) `ForceItemize` election is opt-in. The assertion hazards near it are carried by the §2.5 and §2.7 questions, not by this field. *(r3 M-4: r2 left it "interacts with §2.5/§2.7" — which is not a classification, and the §3.3 classifier will demand one at step 10.)* |
| `W2.owner` / `ScheduleCInputs.owner` | `#[default] Taxpayer` | **★ NOT a silent-default defect** — Fable r2 M-2 **corrects its own r1 aside, which r2 inherited**: neither carries `#[serde(default)]`, so **the TOML import REQUIRES the key**. The `#[default]` reaches only Rust-side fixture construction. Exemption reason: *"serde-required at import."* |
| `QbiInputs.reit_ptp_carryforward_in_provenance`, `CharitableCarryItem.provenance` (`CarryProvenance`) | — | no print, no tax direction ⇒ class (C) |
### 2.9 ★★ THE CIRCULAR LIVENESS — a defect in SHIPPED code, found by the owner's question

***OWNER-FOUND, 2026-07-14 — not a review finding.*** *The owner asked whether we exploit "this question does
not apply, because the form it lives on is not part of this return." **We do — it is what the `live` predicate
is.** But checking whether we use it **correctly** found this.*

```rust
// crates/btctax-core/src/tax/return_1040.rs:1462  — SHIPPED, v0.2.0
pub fn schedule_b_files(ri: &ReturnInputs) -> bool {
    sum_taxable_interest(ri) > SCHEDULE_B_THRESHOLD
        || sum_ordinary_dividends(ri) > SCHEDULE_B_THRESHOLD
        || ri.foreign_accounts == Some(true)      // ← ★ THE ANSWER IS INSIDE ITS OWN LIVENESS PREDICATE
}
```

`ForeignAccounts` and `ForeignTrust` are live **iff `schedule_b_files(ri)`** — in `screen_inputs`
(`schedule_b_part3_unanswered`, `return_1040.rs:1473`) **and** in `income answer` (`live_questions`,
`answer.rs:94-97`). **And `schedule_b_files` reads `foreign_accounts` itself.** That circle has a fixed point,
and it resolves the wrong way:

> A filer with **$100 of interest** and a **foreign bank account** has `foreign_accounts == None`.
> ⇒ `schedule_b_files` is **false** ⇒ the question is **never live** ⇒ it is **never asked** ⇒ the answer stays
> `None` ⇒ `schedule_b_files` stays **false** ⇒ **Schedule B never prints.**
> **Their foreign-account declaration is silently omitted from the filed return.**

**This is worse than an unaffirmed checkbox.** D-8 printed a *box* unchecked; here **an entire required
schedule is absent**, so there is nothing on the form for the filer's eye to catch. Schedule B Part III is the
**FBAR/FinCEN** disclosure surface, where the penalties are not small.

**`foreign_trust` is worse still: it is not even IN the predicate.** A foreign-trust distribution independently
requires Part III — but `schedule_b_files` never mentions `foreign_trust`. So a filer with a foreign trust and
modest interest is never asked, and **the `ForeignTrust ⇒ refuse` guard (Form 3520 unsupported) is
UNREACHABLE** for exactly the population it exists to catch.

**★ THE RULE, and it generalizes:**

> **A question's liveness may NEVER depend, transitively, on that question's own answer.**
> When the answer is what *determines* whether the form files, the question **cannot** be scoped by whether the
> form files. It must be **live unconditionally.**

*(This is the identical argument §2.4 already makes for `HsaActivity` — "we cannot scope it by 'Form 8889
doesn't file', because the answer is what decides whether 8889 files." **The rule was already in the spec, on
one field, and unapplied to the two fields that had shipped the bug.** That is the class, once more: a correct
principle held by convention instead of construction.)*

⇒ **Both questions become live ALWAYS** (§3.1). The honest price: a simple single filer answers **five**
questions, not three. ⇒ **`schedule_b_files` gains the foreign-trust trigger** (`foreign_trust == Some(true)`),
so the predicate is true whenever Part III is required — *and note it is **refusal-shadowed** (r5 Nit-3):
`foreign_trust == Some(true)` refuses unconditionally in `screen_inputs` (`return_refuse.rs:508`) before
anything prints, so no return reaches the print with it set. It is **belt-and-braces, deliberately**: the
predicate should be TRUE whenever Part III is required, independent of screen order. **Do not write an
unreachable-print test for it, and do not file it as dead code in r7** — and update `schedule_b_files`'s
shipped doc comment, which currently asserts "never a Schedule-B path".* ⇒ **`schedule_b_part3_unanswered` is DELETED** — the
registry's always-live check strictly subsumes it (it refuses on `None` regardless of the threshold, which is
what the form requires).

**Mutation-check:** a fixture with **$100 interest, $0 dividends, and `foreign_accounts = None`** must
**REFUSE** (`ScheduleBPart3Unanswered`). Under the shipped predicate it computes clean — *that test is red on
`main` today, and turning it green is the fix.*


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
| **`ForeignAccounts` / `ForeignTrust`** | **`always`** ← **was `schedule_b_files(ri)`, which is CIRCULAR — the answer is one of that predicate's own inputs, and the fixed point silently omits Schedule B entirely (§2.9). A defect in SHIPPED code.** |
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
| `ForeignTrust` *(existing variant, kept — r4 Nit-3: r4 invented the name `ForeignTrustUnsupported`; the shipped variant is `RefuseReason::ForeignTrust`, `return_refuse.rs:510`)* | `foreign_trust == Some(true)` | `screen_inputs` |
| `DependentSpouseUnsupported` | `can_be_claimed_as_dependent_spouse == Some(true)` | `screen_inputs` |
| **`HsaActivityUnsupported`** *(★ r6 Nit-5: this is a **RENAME** of the shipped `RefuseReason::HsaPresent`, `return_refuse.rs:108/740` — step 1 carries it; do not invent a new variant beside the old)* | `hsa_activity == Some(true)` | `screen_inputs` |
| `DualStatusAlienUnsupported` | `dual_status_alien == Some(true)` | `screen_inputs` |
| `SalesTaxElectionWithoutAmount` | `salt_use_sales_tax == Some(true)` ∧ `salt_sales_tax_amount == 0` ∧ any income-tax SALT input > 0 (§2.2) | `screen_inputs` |
| **`ScheduleBForeignCountryMissing`** (r1 I-6, **named at last** — r3 M-2) | `foreign_accounts == Some(true)` ∧ `foreign_country_names.trim().is_empty()` — Sch B **7a "Yes" with a blank 7b**. Detail names **`income import`** as the remedy: `answer` captures bools and dates, **never strings** | `screen_inputs` |
| ~~`MixedUseMortgageUnsupported`~~ | — | **★ DELETED (r4 I-2).** `Some(false)` no longer refuses at all: it **zeroes 8a, CHECKS the box, and fires a mandatory advisory** (§2.7). No compute-dependent layer, no second derivation of `deduction_is_itemized`, no brick. *The refusal was never necessary — two revisions were spent fixing its plumbing before noticing that.* |

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
/// §163(h)(3)(F): the filer declared a MIXED-USE mortgage, and v1 cannot compute the Pub. 936 allocation —
/// so Schedule A line 8a claimed $0 and the box was checked. This can be a LARGE overstatement of tax
/// (a $500k acquisition mortgage with a $20k HELOC forfeits ~96% of a real deduction). MANDATORY: it names
/// the whole forgone amount in dollars. Fires on Some(false) — the filer ANSWERED, and answered the way
/// that costs them money. (§2.7 / the owner mandate.)
MixedUseMortgageNotAllocated { forgone_interest: Usd },
```

⚠️ **★ Its TEXT must branch on the deduction actually taken, and `forgone_interest` is a CEILING (r5 M-1).**
A zeroed 8a can leave — or flip — the return onto the **standard** deduction, in which case **no Schedule A
prints, no box was checked, and the benefit actually forgone is at most (true-itemized − standard), possibly
$0.** An advisory that says *"your Schedule A claimed $0 on line 8a"* to a filer who **took the standard
deduction** is **describing a form they did not file.** The owner mandate forbids *silence*; it does not
license a false description or an inflated loss.

> **itemized:** *"Your Schedule A claimed **$0** on line 8a and the mixed-use box is checked. A Pub. 936
> allocation could restore **up to {forgone_interest}** of mortgage interest — your tax is **OVERSTATED**."*
> **standard:** *"Your return took the standard deduction. Because you declared a mixed-use mortgage, line 8a
> was treated as **$0**; a Pub. 936 allocation of **up to {forgone_interest}** might have made itemizing win."*

**`forgone_interest` = the full `mortgage_interest_1098`** — documented as **"up to"**, never as "the amount
you lost". The true allocation is the input **P8** will capture (§2.7's follow-up).

**★ Note the shape of the third one.** The other two class-(B) advisories fire on **`None`** (never asked).
This one fires on **`Some(false)`** — *an answered question that costs the filer money.* The owner mandate is
not *"tell them when we didn't ask"*; it is **"never forgo a benefit in silence"** — and a benefit forgone
because the filer told us the truth is forgone just as hard. **`AgedBoxForfeitedNoDob` fires on unknown;
this one fires on known.** Both are the mandate.

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

⚠️ **★ "…no longer fires" — NOT "the refusal is gone" (r3 M-5).** For **four** of the eight questions the
*other* half of the answer space is a **value-refusal**: `y` refuses on `HsaActivity` / `DualStatusAlien` /
`ForeignTrust` / `DependentSpouse`. A naive `assert!(screen_inputs(..).is_none())` **fails on four of eight** —
and the natural "fix" (weakening the assertion) would make **the anti-vacuity machinery itself vacuous**.
Assert on the *specific reason*.

⚠️ **The count is fixture-contingent, and the fixtures must be built so the intent holds (r6 M-2).** "Four" is
the number of questions whose `Some(true)` **unconditionally** fires a value-refusal. But `ForeignAccounts`
answered `y` on a *minimal* fixture ALSO refuses — `ScheduleBForeignCountryMissing` fires on a blank
`foreign_country_names` (§3.2) — so a naive fixture makes it five. ⇒ **The `ForeignAccounts` fixture supplies a
non-empty `foreign_country_names`**, so its `y` case exercises the intended path. The operative instruction
(assert on the *specific* unanswered reason) is robust either way; the fixture discipline is what keeps the
count honest.

**★ FOUR, not five — and the mortgage question is the one that moved (r5 I-2).** r5 folded §2.7's dissolution
into the design sections and **left this paragraph asserting the refusal it had just deleted** ("`n` refuses on
the mortgage question (when itemizing)"), *complete with the compute-dependent trigger r4 I-2 proved
unimplementable.* An implementor following it would have **re-added the deleted refusal to make the test
pass** — the fold un-folding itself through the test suite. *This is why the workflow says re-review after
every fold, including the last.* ⇒ The mortgage question's answered-half assertion is:

> `n` ⇒ **the return COMPUTES**, `MixedUseMortgageUnanswered` no longer fires, **8a = 0**, **the box is
> CHECKED**, and **`MixedUseMortgageNotAllocated` fires** (§2.7/§3.4).

**★ The hand-written per-question refusal tests are KEPT, not deleted.** r1 said the property test "replaces
per-question tests forever" — which invited deleting exactly the tests that catch a dropped entry.

**Mutation-checked (acceptance):** delete the registry loop → a named test fails; delete the `build` refusal →
a named test fails; drop a `FORM_QUESTIONS` entry → a named test fails; drop **any** of the three advisories →
a named test fails; **delete the `SCHEMA_VERSION` check → a named test fails** (§2.6); **delete the `clear`-first
requirement → a computed carryover is silently lost → a named test fails** (§2.6, r5 I-1).

**★ AND EVERY VALUE-REFUSAL — the ones the property test CANNOT hold (r5 I-3).** §3.5's per-question test
asserts only that *the unanswered reason* stops firing on `y`; **it passes with or without the
`Some(true)` guard.** So each value-refusal needs its own named test and its own mutation — *delete the
`Some(true)` arm ⇒ that test fails*:

| guard | its mutation |
|---|---|
| **`DualStatusAlienUnsupported`** | **★ r5 I-3 — it was specified in §3.2 and scheduled NOWHERE**: no build step wrote it, no test named it, and **nothing else consumes the field**. The phase could have closed "green" with a truthful dual-status **"yes" COMPUTING** — taking the full standard deduction that **§63(c)(6)(B) denies**. *The untested-guard pattern, on the one new refusal with a silent understatement behind it.* |
| `HsaActivityUnsupported` | *(compile-forced: step 1's rename breaks `return_refuse.rs:738` — the only one of the four with a forcing function, which is exactly why the other three need naming)* |
| `SalesTaxElectionWithoutAmount` | delete the arm ⇒ named test fails |
| `ScheduleBForeignCountryMissing` | delete the arm ⇒ named test fails |



**★ The migration guard cannot go vacuous, and that is the POINT (r4 I-4).** r4's per-key fixture *could*: both
`blind` keys serialize as the string `"blind"`, and the cited fixture base has no spouse — so a
`str::replace` "covering both" would have silently hit the taxpayer's key **twice**, and the spouse unlaunder
would have been held by **nothing** (*the P8a I2 failure, one struct level deeper*). The §2.6 guard has **no
keys to confuse**: insert a row at `schema_version = 0` and at `= 1`; assert **`get` AND `all`** both return
`StaleReturnInputs`; delete the version check ⇒ **both fail** (the row loads, and its `blind: false` arrives as
`Some(false)` — the laundering, caught in the act). *A guard that cannot be written vacuously beats a guard
that must be written carefully.*

**★ And the SHIPPED circular-liveness bug gets its own red test (§2.9):** a fixture with **$100 interest, $0
dividends, `foreign_accounts = None`** must **REFUSE**. On `main` today it computes clean — **that test is red
before the fix and green after, which is the definition of the bug being real.**

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
- **★★ THE SHIPPED CIRCULAR-LIVENESS BUG IS FIXED** (§2.9). A return with **$100 of interest and an unanswered
  `foreign_accounts`** REFUSES. *Today it computes clean and silently omits Schedule B — the FBAR/FinCEN
  disclosure surface.* **No question's liveness may depend, transitively, on its own answer**, and
  `schedule_b_part3_unanswered` is deleted because the registry subsumes it.
- **★ NO VALID RETURN IS BRICKED — the guarantee this spec violated in r2 AND r3.** Named regression tests:
  1. a **dormant**-HSA holder answers "no" to all four §2.4 clauses truthfully ⇒ their return **computes**;
  2. a **mixed-use-mortgage filer** answers "no" truthfully ⇒ their return **computes** — with 8a = $0, the box
     **checked**, and a mandatory advisory naming the money (§2.7). *There is no mortgage refusal left to
     brick anyone with.*
- **★ The HSA question covers ALL FOUR Form 8889 triggers** (§2.4), and the spec records which clause is
  **conservatism rather than statutory necessity** (the spousal-inheritance sub-case rests on the instructions,
  not the Code).
- **★ Stale stored rows REFUSE** (§2.6): `schema_version != SCHEMA_VERSION` ⇒ `StaleReturnInputs`. **There is
  no unlaundering key-list to forget** — and therefore no vacuous-fixture failure mode (r4 I-4 dissolves). The
  **dead D-8 `version < 1` unlaunder is deleted.**
- **★★ AND THE REMEDY WORKS — INCLUDING CARRYOVER RESTORATION (r5 I-1, r6 I-1).** The refusal names, in
  order: **`btctax income clear <year>`** (a bare `DELETE` — it cannot refuse), **`btctax income import`**
  (which reads the stale row at `cmd/tax.rs:63` and would otherwise fail on every row it exists to cure), **and,
  if the row carried a computed carryover, `btctax report --tax-year <year−1> --write-carryover`** to rebuild
  what `clear` discarded. Requiring `clear` is **deliberate** — it makes discarding the `Computed` carryovers
  an explicit, *reversible* act, never a silent loss (§4 R3-M6 — a dropped REIT/PTP carryforward **understates
  tax**). **Named tests: `clear`+`import` succeeds and the row is v2; AND the full-chain restoration test —
  seed N, write-carryover onto N+1, stale both, run the whole remedy, assert N+1's carryover equals its
  pre-stale value.** *(The rebuild is possible only while the policy's premise holds — TOMLs still exist; the
  §5 step 12(b) expiry is load-bearing.)*
- **A new `bool` / `Option<bool>` / defaulted enum anywhere reachable from `ReturnInputs` does not compile until
  a human EDITS the classifier** — `#![deny(unused_variables)]` makes an ignoring named binding a hard error.
  *(★ r3 I-5: the guarantee is "a human must look", NOT "it is classified correctly." `_`-prefixed bindings and
  `let _ = x;` are **grep-able residue**. Stated at its true strength — the previous wording claimed a force
  the mechanism does not have, and Fable defeated it in one token.)*
- **The owner mandate holds: no benefit is forgone in silence** — and it now covers **both** shapes: forgone
  because we **never asked** (`blind`, `salt_use_sales_tax` — fire on `None`) and forgone because the filer
  **answered truthfully and it cost them** (`MixedUseMortgageNotAllocated` — fires on `Some(false)`, naming the
  whole forgone deduction). `persons` follows the §3.4 formula (MFJ-only spouse box; an **absent** MFJ spouse
  still forfeits).
- **The SALT prompt cannot silently zero line 5a** (`SalesTaxElectionWithoutAmount`).
- **`dual_status_alien` and the Schedule A line 8 box exist, are asked, and refuse when unanswered — AND
  `dual_status_alien == Some(true)` REFUSES AS UNSUPPORTED, with a named test and a named mutation** (r5 I-3).
  *Without that clause the phase could close green while a truthful "yes" **computes**, taking the standard
  deduction §63(c)(6)(B) denies a nonresident. The guard was specified in §3.2 and scheduled nowhere — the
  [[untested-guard-pattern]], caught by a reviewer walking the build order rather than the design.*
- **The three DEAD fields are deleted, AND `income import` refuses unknown TOML keys** — via **`serde_ignored`**,
  not a hand-written key list (§2.3). It must reject `hsa_present` and each deleted field, **naming them**.
- **Schedule B 7a "Yes" with a blank 7b refuses** (`ScheduleBForeignCountryMissing`), and its detail names
  `income import` as the exit (`answer` cannot capture strings).
- **★ The checkbox census is CLOSED BY ENUMERATION** (§2.1) — including the boxes btctax *never writes*, which
  are a **named category**, not silence. Every printed box across all **fourteen** forms is a registry question,
  a class-(B) advisory, a pen-deferred pair, a **rule-3 exemption**, or structurally inapplicable.
  **Form 8949's Box I/L and Schedule D's QOF "No" — both software-answered, both scope-entailed — get the
  `LIMITATIONS.md` entries they never had.** *(r4 I-1: the census was published "COMPLETE" while 8949 was
  checking a box on every filed return. **A census is only as good as its worst row**, and this one is now
  verified twice, independently.)*
- **MFJ with no spouse identity ⇒ `ReturnHeader::build` REFUSES** (r3 M-6 — *decided, not deferred*).
- Every guard above is **mutation-checked**, and each mutation is **named in §3.5** — *including the four
  value-refusals, whose mutation the per-question property test structurally cannot catch* (r5 I-3).
- `make check` green; **0 Critical / 0 Important** from independent review.
- FROZEN (`tax/{types,compute,se}.rs`) unchanged. `screen_inputs` **and `screen_compute_dependent`** keep their
  signatures (§2.7 removed the only reason to change one); `resolve.rs` untouched.

## 5. Build order (TDD; each step red → green)

*★ r2's order did not **compile** (r2 I-5). r3's ended step 4 **red** (r3 I-6). r4's ALSO ended step 4 red —
on the same property test's **third** assertion, `income answer` asks it (r4 I-3). **Three revisions, three
red steps.** "Red → green" is a contract about each step's CLOSE. ⇒ **The registry, its tests, `screen_inputs`
AND `income answer` are ONE step** — they are one mechanism, and the tests that prove it cannot be split from
either consumer.*

1. **Fields first.** `sch1.hsa_present: bool` → `hsa_activity: Option<bool>` (§2.4 — a **rename**);
   `Person.blind` and `ScheduleAInputs.salt_use_sales_tax` → `Option<bool>` (§2.2); **NEW**
   `ReturnInputs.dual_status_alien` and `ScheduleAInputs.mortgage_all_used_to_buy_build_improve` (§2.5, §2.7).
   **⚠️ Churn (r3 Nit-4):** `HsaActivity`, `DualStatusAlien`, `ForeignAccounts` and `ForeignTrust` are live on
   **every** return, so every computing fixture in four crates must answer them. **Default them in the
   `testonly` builders** — one line there, not two hundred call sites. *(r4 Nit-4: the TUI test at
   `tabs/tests.rs:1704` supplies its **own** literal refusal string to the snapshot and needs no edit — do not
   go hunting for a dependency that isn't there.)* **★ And the mortgage question bricks fixtures too (r6 Nit-2):**
   `testonly.rs:223/:526` and ~7 inline `return_1040.rs` fixtures carry nonzero `mortgage_interest_1098` under
   a `schedule_a`. Default `mortgage_all_used_to_buy_build_improve = Some(true)` in the `testonly` Schedule-A
   builder — the same one-line move.
2. **★ `SCHEMA_VERSION = 2` + the stale-row refusal** (§2.6). Delete the dead D-8 `version < 1` unlaunder;
   rewrite the P8a migration tests to assert a v0/v1 row **refuses** on **both** `get` and `all`.
   Mutation: delete the version check ⇒ both fail.
3. **`questions.rs`** — `QuestionId` (+`ALL`), `FormQuestion`, `FORM_QUESTIONS` (8 entries). Liveness lifted
   from the current refusals **except** the two corrections: `DependentSpouse` → `Mfj || spouse.is_some()`
   (**= P8a I1**), and `ForeignAccounts`/`ForeignTrust` → **always** (**= §2.9, the shipped bug**).
4. **★ ONE STEP: the anchor + the property test + `screen_inputs` + `income answer`** (§3.5, §3.2, r4 I-3).
   Write the completeness anchor and the per-question property test **red** — *all three* of its assertions,
   including "`income answer` asks it". Then: derive `screen_inputs` from the registry; delete the hand-written
   unanswered-blocks **and `schedule_b_part3_unanswered`**; add `foreign_trust` to `schedule_b_files` (§2.9);
   derive `live_questions` from the registry; shrink the `Question` enum to the DOB residue; add the class-(B)
   skippable prompts (SALT scoped to `schedule_a.is_some()`; **both** spouse prompts gated on
   `spouse.is_some()`, r3 I-7). **Green.** Fix multi-defect fixtures that asserted a specific reason.
   *This is the step that turns P8a I1 green, and the §2.9 red test green. It is one mechanism; splitting it is
   what produced a red step in three consecutive revisions.*
5. **`ReturnHeader::build`** → `HeaderError`; update `admin.rs` mapping and all fixtures; **MFJ-with-no-spouse
   refuses** (r3 M-6).
6. **★ ONE STEP: Schedule A line 8's value behaviour AND its advisory** (§2.7 + §3.4 — r5 M-2). `Some(false)`
   ⇒ **8a = 0 AND the box CHECKED**, *and* `MixedUseMortgageNotAllocated` fires with the deduction-branched
   text. **They are one mechanism and must not be split**: shipping the advisory first (as r5's order did)
   leaves a green step where the advisory tells the filer *"line 8a claimed $0 and the box is checked"* while
   the filler **still prints full 8a with the box unchecked** — *a user-visible falsehood at a step boundary,
   with no red test to catch it, because nothing asserts 8a's value until the next step.* Regression test: the
   mixed-use filer **computes**, under BOTH `Auto`-standard-wins and `ForceItemize`.
7. **The other TWO advisories** (§3.4): `blind` and SALT, firing on **`None`** only. `persons` per the §3.4
   formula (MFJ-only spouse box; an **absent** MFJ spouse still forfeits).
8. **The VALUE-refusals the property test cannot hold** (§3.5, r5 I-3) — each with its own named test and
   mutation: **`DualStatusAlienUnsupported`** (★ *scheduled by no step until r5 caught it — a truthful "yes"
   would have COMPUTED*), **`SalesTaxElectionWithoutAmount`** (§2.2), and **`ScheduleBForeignCountryMissing`**
   (r1 I-6, named in §3.2). *(`HsaActivityUnsupported` needs no step: step 1's rename compile-forces it out of
   `return_refuse.rs:738` — the only one of the four with a forcing function.)*
9. **★ DELETE the three dead fields FIRST** (§2.3) **AND add `serde_ignored` unknown-key rejection to `income
   import`** — **together or not at all**; the rejection must name `hsa_present` too. *(r6 M-1: this MUST
   precede the classifier. Class (D)'s only lawful disposition is "consume or delete" — so if the classifier
   is built while these three fields still exist, it cannot bind them without inventing a forbidden
   "(D), pending deletion" exemption. Deleting first means the classifier never sees a field it may not
   classify. Neither the deletion nor `serde_ignored` depends on the classifier, so the swap is free.)*
10. **The classifier** (§3.3): every reachable struct (incl. `Box12Entry`, `Carryforward`);
    `#![deny(unused_variables)]`; the `_`-and-`_`-prefix rule; the exemption register with class + statutory
    reason (incl. `itemize_election`, §2.8). Folds the open `header: _` follow-up.
11. `LIMITATIONS.md` — the new refusals; **the three advisories**; **the two SCOPE-entailed software-answered
    boxes (Schedule D QOF "No" and Form 8949 Box I/L)** and the **Schedule C G/H/I/J blank boxes** (§2.1 — none
    was ever disclaimed); the **`income clear` → `income import` → (if a carryover existed)
    `report --tax-year <year−1> --write-carryover` requirement** for pre-P9 vaults, **that `clear` DISCARDS any
    computed carryover on the stale row** and how to rebuild it (§2.6, r5 I-1 / r6 I-1), **and that `income
    show` also refuses a stale row** (r6 Nit-4 — the filer cannot inspect what `clear` will discard before
    discarding it; acceptable only under the no-real-data premise, one more tooth for the step 12(b) expiry); that Sch B /
    MFS refusal texts now name `btctax income answer` (a deliberate **improvement**); that `income answer`
    cannot capture strings.
12. **FOLLOWUPS.md** — file the two items this spec creates, each with its **owning phase**:
    **(a) → P8:** capture `mortgage_interest_deductible` (the Pub. 936 worksheet result) so a mixed-use filer
    recovers the deduction §2.7 currently zeroes.
    **(b) → the RELEASE GATE:** *refuse-and-reimport expires the moment real data exists.* §2.6's policy is
    lawful only while every vault is test data; the first real return requires real migrations, and prior-year
    carryforwards are exactly what a filer cannot reconstruct.
