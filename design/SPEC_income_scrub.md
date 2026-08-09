# SPEC — `btctax income scrub`

*Ceremony scaled per `STANDARD_WORKFLOW.md` §8: one spec, no separate plan (build order is §8). The
gates are NOT scaled: this reaches 0C/0I before a line of the rework is written, then again on the
whole branch before it ships.*

**Status:** **r5 — BUILD-READY.** Supersedes the unreviewed behaviour on `feat/income-scrub`
(`31d5c79`, `2449ee4`), held back from every release pending this spec.
Prior art: `reviews/scrub-r1-workflow.md` (code review, 19 confirmed + 6 sweep),
`reviews/scrub-r2-fable-consult.md` (scope adjudication), `reviews/scrub-spec-r1-review.md`
(**r1: 31 raw → 19 blocking**), `reviews/scrub-spec-r2-review.md` (**r2: 0C/4I/3M**),
`reviews/scrub-spec-r3-review.md` (**r3: 0C/4I/3M/1N**), `reviews/scrub-spec-r4-review.md`
(**r4: 0C/2I/3M/1N**) — all folded.

★★★ **r4 ANSWERED THE QUESTION OF WHEN TO STOP, AND ITS ANSWER GOVERNS: the remaining risk is
TEST-SHAPED, not prose-shaped. No fifth whole-document round.** Its evidence was *how* its own findings
were reached — not by reading section against section, but by executing §3.3's derivation against the
real fixtures (`grep ip_pin testonly.rs` → **0**; `foreign_country_names` → **0**; `ein:` → **0**; one
`b_1099`, in the wrong household). **The risk has moved out of the prose and into a fixture nobody has
written yet**, which a fifth prose round structurally cannot reach and one command answers.

**So the next action is §8 step 2, not another review** — and §8 step 4's first act is to land the axis
derivation and *watch it go red* (B1). The one-command check that settles this permanently: **the
derived path set must contain `w2s[].ein`, `header.ip_pin`, `foreign_country_names` and
`b_1099[].payer`.** If it does not, the sentinel fixture is not maximal and the derivation is blind on
exactly the four fields four review rounds were spent on.

★★★ **THREE ROUNDS HAVE NOW FOUND THE SAME DEFECT AT THREE DEPTHS, AND THAT IS THE MOST IMPORTANT
THING ON THIS PAGE.** §3.2 states its rule over a **mechanism**; each round then found that some
*instrument* implementing it had been written as a hand-list one level down:

| round | the list that was not derived |
|---|---|
| r2 | §3.3's field axis and §5's divergence set were four fields and two entries, chosen by what redded an existing test |
| r3 | the fold's replacements: an emptiness rule keyed to `""` when every reader keys on `trim()`, scoped to "payer/employer loops" when `business_description` is the field that matters, and a §5.1 table that enumerated the fields already under discussion — **missing the IP PIN, which is replaced, printed, and invisible to both §3.3 tests** |
| r4 | the derivation r3 added named the wrong **blind spot** (value collision, not structural absence) and authorised its third verdict by the wrong **discriminator** (the type, which does not decide malformed-ness) — each one level short, and each still hiding the same four fields |

★★ **The lesson is not "derive it" — every round said that. It is that a derivation is not one until
the spec names the operation that computes it, the blind spot that operation has, and the property that
authorises each exemption.** "The set of fields `scrub_pii` replaces" is a hand-list wearing a
mechanism's clothes, because Rust has no reflection to answer it — and naming an operation is still not
enough if the named blind spot is the wrong one, which is exactly how r4's version stayed blind to the
same four fields.

★★★ **THE MEASURE IS THE FOUR FIELDS, NOT THE PROSE.** Every round has been about the same handful —
`w2s[].ein` (§3.1's Critical), `header.ip_pin`, `foreign_country_names`, `b_1099[].payer` — and each
round's instrument failed to reach them for a *different* reason. That is why §8 step 4 now begins by
printing the derived set and asserting those four are in it: **the question stops being reviewable and
becomes a command.**

---

## 1. The problem

A filer hits a refusal, or a figure they believe is wrong. Diagnosing it needs the return. The return
is the most sensitive document most people own.

Prose does not work: transcription is where this project's defects come from, and the figures that
matter are the ones a summary rounds off. Sending the real file is not acceptable. So: emit a copy with
the identity replaced and every figure intact.

**The product is the authorization, not the file.** `income scrub` tells a filer "this is safe to hand
to a stranger." That sentence *is* the feature. A wrong figure is a bug a filer can audit; a wrong
safety claim is one nobody can audit afterward, and it persists in every installed copy forever
(crates.io yanks, never deletes). This spec is written to make the command **refuse more than it
scrubs**.

---

## 2. Scope — inputs only, and it REFUSES rather than half-answer

### 2.1 What it emits

The year's `ReturnInputs` as TOML, carrying the §4.2 provenance marker. The recipient loads it with
`income import --force` (§4.3) — the marker makes a plain `import` refuse, deliberately.

### 2.2 ★★★ The refusal predicate is SCRUB-OWNED, and it is not the digital-asset box

**Obligation, stated as a mechanism rather than a list of outcomes:** scrub may emit only when **the
ledger contributes nothing to any figure, refusal, gate, watermark or advisory btctax produces for this
year**. A new `pub fn ledger_contributes(state, year) -> bool` in `btctax-core::tax::scrub` owns that
sentence; its doc comment carries it, and every disjunct names the mechanism that put it there.

```
ledger_contributes(state, year) =
      digital_asset_activity(state, year)          // disposals / income / removals in-year
   || digital_asset_activity(state, year - 1)      // DELIBERATE OVER-REFUSAL — see below; this
                                                   //   disjunct secures no live read
   || state.pseudo_active()                        // DRAFT watermark + attestation gate
   || first_hard_blocker(state).is_some()          // NotComputable, projection-wide
```

★★ **The `year - 1` disjunct is retained as DELIBERATE OVER-REFUSAL, and two rounds of naming a
mechanism for it were both wrong (r2 M-1, r3 M-1).** The honest statement is that it secures no live
read:

- It is **not** the M4 carryforward-consistency advisory. That needs the prior year's stored *profile*,
  not merely its ledger (`cmd/tax.rs:465-487` — `resolve_screened(.., year - 1, ..)` yields
  `Ready { profile: None }` on a recipient who imported one year), so M4 does not reproduce from a
  one-year file **whatever this disjunct decides**.
- It is **not** "the prior year's ledger feeds this year's carryforward-in" either. `scrub_pii`
  preserves `capital_loss_carryforward_in` and its provenance tag verbatim (`scrub.rs:220-221` — r3
  cited `:222-223`, which is the AMT carryover pair; corrected here), so
  those figures travel in the TOML and reproduce on the recipient's machine **with no ledger at all**.
  Nothing derives them live: the prior year's ledger produces the prior year's `carryforward_out`
  (`compute.rs:415`), which is either compared against the declared figure by `carryforward_consistency`
  (`compute.rs:450`) or written into next year's stored inputs by the separate `write_back_carryover`.

**So it is kept because scrub declines to prove a negative** — that no path reads the prior year's
ledger live — on a command whose whole posture is to refuse more than it scrubs. The direction of any
error is over-refusal, which is the safe direction, and the comment now says that instead of
manufacturing a third mechanism. ★ The cross-year residue is recorded in §5.1.

**r1 killed the first draft's predicate and its justification.** It reused
`digital_asset_activity(state, year)` alone, arguing the refusal was "exactly as accurate as a box
btctax already prints." Both halves were wrong:

- **Year-scope is not artifact-scope.** `pseudo_active()` and `first_hard_blocker()` are
  **projection-wide** and year-blind. A vault whose only activity is unclassified inbound transfers has
  `digital_asset_activity(2024) == false` yet `pseudo_active() == true` — so the *filer's* export is
  DRAFT-watermarked and refuses without an attestation phrase, while the recipient's copy is clean and
  ungated. A 2022 `ImportConflict` makes the filer's 2024 delta `NotComputable` and the recipient's
  computable. **That is exactly the filer most likely to run `income scrub`** — "btctax won't export my
  forms" — handed the one file on which the problem vanishes.
- **It leaned on a don't-know.** The predicate's own doc says `false` leaves the box **unchecked, NOT
  answered "No"** — btctax never answers No. Reading that deliberate silence as "the ledger is empty"
  is `widening-an-exemption-is-never-the-safe-edit` in miniature.

★ **The two helpers are private and live in TWO DIFFERENT modules, neither of them `scrub`** —
`digital_asset_activity` at `return_1040.rs:2013` and `first_hard_blocker` at `compute.rs:462`. So the
§8 step widens **two** visibilities, not one, and `ledger_contributes` takes a dependency on two
modules it does not live in. Widen each to `pub(crate)` only — the precedent is `canonical_ein`
(`return_1040.rs:681`), already `pub(crate)` and already called cross-module from
`return_refuse.rs:941`. `ledger_contributes` itself is the only new `pub` item, and its doc names both
callers so neither drifts. The scrub path must also **project the ledger**, which it currently never
does (`Session::project()`); if projection fails, scrub **refuses** — it never falls back to "assume
empty".

**Why refuse rather than emit the inputs half.** For most users of a *crypto* tax tool the ledger is
most of the return. A file reproducing half a return, stamped shareable, is the "blank that answers for
the filer" pattern at file granularity. A caveat in `--help` is not equivalent: the caveat is read
once, the file is used repeatedly.

**Ledger-bearing years have no scrub path in v1, and the refusal says so plainly.** r1 refuted the
first draft's consolation — "describe it in prose, or send the outcome" — as unsupported (§1 already
rejects prose; "send the outcome" is not always sufficient). The refusal names what does not travel and
stops. It must **never** point at `export-snapshot`, which is the FR10 plaintext exception and is not
scrubbed at all.

### 2.3 ★★★ REJECTED, permanently: scrubbing the ledger

Recorded so it is not re-proposed as the obvious fix.

A `Disposal` carries exact satoshi quantities, exact timestamps and wallet attribution. **These are
public blockchain quantities.** Scrub's invariant is that no figure moves — and one cannot
figure-preservingly de-identify data whose figures *are* the identifier on a public ledger. An amount
and a date locate a transaction; chain analysis re-identifies from exactly that.

A "scrubbed" ledger marked safe to share would be the most dangerous artifact this tool could produce.
The honest end state for ledger-bearing years may permanently be *"this cannot be made safe to share."*
Any future proposal must first answer how it defeats chain analysis, not how it removes names.

### 2.4 Non-goals

- **Anonymity.** The figures are a financial profile even with the identity gone; the help claims only
  de-identification.
- **Perturbing figures.** Rounding or fuzzing crosses brackets, phase-outs and the
  standard-vs-itemized line, destroying the behaviour being reported. Explicitly refused.

---

## 3. The one invariant: scrub preserves the EQUIVALENCE CLASS, never just the value

r1 found eight code defects that are one defect: scrub replacing a value with a stand-in from a
*different* class, so the scrubbed copy behaves differently.

> **A scrubbed value must be indistinguishable from the original to every predicate the program applies
> to it — and different only in content.**

★ Stated as a rule over the mechanism, not a hand-list: **every field `scrub_pii` replaces must preserve
every property any screen, `ReturnHeader::build`, or form filler reads from it.** §3.2's four fields are
the currently-known instances, not the definition.

### 3.1 Identity partition, under the program's OWN comparison

Where code compares identifiers, scrub preserves the partition **that comparison** induces — never
string equality.

`EinMap` keys on `canonical_ein(real)`, not the raw spelling. §6413(c) turns on "more than one
employer" and compares canonicalized EINs, so `"11-1111111"` and `"111111111"` are ONE employer. The
held code keys the raw string, splits them, and manufactures an excess-Social-Security credit —
reproduced at $1,546.80 against the repo's own `two_spellings` fixture. r1's CRITICAL.

### 3.2 Validity class, and ★ NO ORIGINAL VALUE EVER SURVIVES

**Governing sentence, because r1 found the first draft's `malformed → malformed` read most naturally as
"pass the filer's real malformed SSN through":** *no original identity value is emitted in any class.*

`absent → absent`, `malformed → SYNTHETIC-malformed`, `valid → synthetic-valid`. The preserved thing is
the **error variant**, not the bytes: `Missing → Missing`, `WrongLength(n) → WrongLength(n)`,
**`NotDigits(_) → NotDigits('x')`**.

> ★★★ **`NotDigits` is the one deliberate COARSENING, and dropping it is how r2's I-1 got in.**
> `SsnError::NotDigits(c)` carries a **character of the filer's real entry**
> (`btctax-core/src/tax/packet.rs:66-67`: `digits.chars().find(|c| !c.is_ascii_digit())` — note there
> is a second, unrelated `packet.rs` in `btctax-forms`). It is the only `SsnError` payload that is *content*
> rather than *shape* — `WrongLength(n)` carries a digit count, which is what makes it both safe to
> preserve and diagnostically load-bearing ("the filer says their SSN has four digits").
>
> So the two payloads are treated differently **on purpose**, and the asymmetry is stated here rather
> than left to the implementer: preserving `NotDigits`' payload verbatim would emit an original
> identity byte into a file stamped shareable, contradicting the governing sentence one paragraph
> above. r1 named this resolution; r2's fold transcribed the two safe legs and dropped this one **and
> its reason**, which is the dropped-term-becomes-invisible pattern `CLAUDE.md` names.

★★ **Emptiness is a validity class too, and one field violates this rule TODAY.** `screen_inputs`
reads `b.payer.trim().is_empty()` to choose the refusal wording — `"(unnamed broker)"` versus the
payer's name (`return_refuse.rs:672-676`) — and `scrub_pii` sets `f.payer = format!("Broker{}", i + 1)`
**unconditionally** (`scrub.rs:271-273`), turning empty into non-empty. So the ★ rule below is
violated in current code, and §3.3's variant-only comparison cannot see it (the `RefuseReason` is
`Form1099BNeedsForm8949` on both sides; the difference lives in the message).

**Every replacement preserves TRIM-emptiness: a value with `trim().is_empty()` is replaced by `""`,
never by a stand-in.** It applies to **every string `scrub_pii` replaces** — not to the payer/employer
loops only — because the next reader of one of those fields is not required to announce itself.

★★★ **Both halves of that sentence were wrong in r3 and each was wrong in a way that kept the defect
alive (r3's I-1 and I-2).**

- **`""` is not the class.** All three emptiness readers in the screen key on **`trim()`**:
  `return_refuse.rs:672` (broker payer), `:798` (country names), `:901` (business description). A rule
  keyed to `""` leaves `"   "` in the replaced set, so the class still flips — and the repo already
  pins this exact substitution at `return_refuse.rs:1484`: *"Whitespace is not a name. This pins the
  `trim()`, which a naive `is_empty()` would miss."* Emitting `""` for a whitespace-only original
  preserves the class and leaks nothing, so the correction opens no disclosure channel.
- **The payer/employer scoping excluded the one field where the flip removes a REFUSAL.**
  `business_description` is replaced unconditionally at `scrub.rs:251`, and
  `screen_inputs` refuses `ScheduleCNoBusinessDescription` when it is trim-empty
  (`return_refuse.rs:897-905`; the three-space case is pinned at `:1487-1495`). It is `#[serde(default)]`,
  so an imported TOML that omits the key produces `""`. A filer whose description is blank therefore
  **hits that refusal, runs `income scrub` because of it, and hands over a file on which the refusal
  does not exist** — §3.2's own governing harm, reached through the field the rule declined to cover.

★ **`scrub.rs:246-250` orders the opposite and must be corrected in the same step** (§7). Its claim —
*"It must stay NON-EMPTY: an empty description refuses the return … so scrubbing it to `""` would make
the scrubbed copy refuse where the original filed"* — is sound only when the original was non-empty.
As written it is unconditional, and a red matrix cell will be argued away against it rather than fixed.
The correct rule is: **non-empty in, stand-in out; trim-empty in, `""` out.**

Applies to **SSN, EIN, `business_description`**, and — with a carve-out — the **IP PIN**:

> ★★ **The IP PIN is never synthesised.** It is a live IRS anti-fraud credential; minting a well-formed
> one inside a file stamped shareable would fabricate a credential, and neither §3.3 test can see it
> because nothing computes from it. So: **absent or valid → `None`** (keep dropping it, keep the
> shipped rationale); **malformed → a malformed non-credential stand-in**, which is the only leg
> `ReturnHeader::build` reads. This is a deliberate exception to the blanket rule above.

Why it matters: the held code normalizes every class into "valid", so identity-shaped fail-closed
screens vanish and **the scrubbed copy files where the original refused** — defeating the purpose, since
the filer is sending the file to reproduce a refusal.

### 3.3 How the invariant is HELD (B1 — each observed RED first)

Two class-level tests over the fixture set:

1. `screen_inputs(orig)` and `screen_inputs(scrubbed)` produce the **same `RefuseReason` variant**.
2. `ReturnHeader::build(orig).err()` and `..(scrubbed).err()` compare **by VALUE on `HeaderError` —
   with `SsnError::NotDigits` compared by discriminant only.** Not `is_err()` equality, which admits
   error substitution.

> ★★★ **State the comparison, because the two obvious ones are each wrong in a different direction,
> and this is r2's I-1.** Value equality is this repo's existing idiom on this exact call
> (`return_refuse.rs:1526-1535` asserts `SsnError::NotDigits('X')` as a *value*), and it is the only
> comparison that holds §3.2's explicitly-payloaded `WrongLength(n) → WrongLength(n)`. But applied to
> `NotDigits` it would **force the scrubbed SSN to carry the filer's real offending character** — the
> test would then bless an identity byte riding into the shareable file, and land green doing it.
> Discriminant equality everywhere avoids that and simultaneously drops the `WrongLength` fidelity r1
> filed this section to add ("an SSN has 4 digits" vs "has 2 digits", both `WrongLength`, test passes).
> Neither is right alone. The asymmetry is the fix, it is deliberate, and §3.2 carries its reason.

★★ **Both are VACUOUS on today's fixtures** — verified, not assumed: every fixture SSN in
`btctax-core/src/tax/testonly.rs` is well-formed (`111-22-3333`, `123456789`, `987654321`), so all
three comparisons are `None == None`. §3.3 therefore imposes a **fixture matrix obligation**, not a
count.

★★★ **The matrix's FIELD AXIS IS DERIVED FROM `scrub_pii`'s REPLACED SET — not from §3.2's four.**
This is r2's I-3, and it is the same "enumerate from the mechanism, never a hand-list" rule §3.3
already applied to the *state* axis while violating it on the field axis one line later.

**And a derivation is not a derivation until the spec names what computes it (r3's I-3).** "The set of
fields `scrub_pii` replaces" has no reflection behind it in Rust, so left unnamed an implementer writes
`const REPLACED: &[&str] = &[…]` — correct the day it is written, silently wrong the first time a field
changes from kept to replaced, and the spec's claim would then be a guarantee no test holds. Note the
`..`-free destructure guard does **not** close this: it fires when a field is *added* to
`ReturnInputs`/`Person`/`Dependent`/`HouseholdHeader`, never when an existing field's classification
changes.

**The mechanism, stated so it is not reinvented.** `ReturnInputs` derives `Serialize`
(`btctax-core/src/tax/return_inputs.rs:28` — there is an unrelated `return_inputs.rs` in `btctax-cli`),
so the replaced set is computable as

> the set of paths at which `serde_json::to_value(ri)` and `serde_json::to_value(scrub_pii(&ri))`
> **differ**, over a MAXIMAL sentinel fixture — **and a path present on one side and absent on the
> other is a difference.**

★ **Use `serde_json`, not `toml`.** `serde_json` is already a `btctax-core` dependency
(`btctax-core/Cargo.toml:19`) and `toml` is **not** one at all — TOML serialization happens a crate up,
at `btctax-cli/src/cmd/tax.rs:177`. It is also the correct choice on the merits: `serde_json` renders
`None` as `Value::Null` **at a present key**, so a presence change is an ordinary value difference,
whereas `toml::Value` omits the key entirely and `header.ip_pin` — replaced with `None` — would differ
only by absence. §5.1 argues that a cell going *missing* is as much a divergence as one whose contents
change; the differ has to agree.

★★★ **THE BLIND SPOT IS STRUCTURAL ABSENCE, NOT VALUE COLLISION — and getting this wrong would have
made the derivation blind to every field the last three review rounds were about.** A replaced field
produces no differing path when the fixture **never instantiates it**: `None` on both sides, `""` on
both sides, or inside an empty `Vec`. Constraining the fixture over *values* alone is also circular —
you would have to already know the replaced set to populate it. Measured against
`kitchen_sink_household`, the natural sentinel base, the derived axis would silently omit:

| field | why no path differs | the round it was |
|---|---|---|
| `w2s[].ein` | every W-2 literal ends `..Default::default()` ⇒ `ein: None` both sides (**zero `ein:` in the whole file**) | §3.1's **CRITICAL** — the $1,546.80 manufactured credit |
| `header.ip_pin` | **zero occurrences of `ip_pin` in the file** ⇒ `None` both sides | r3's I-4 |
| `foreign_country_names` | **zero occurrences in the file** ⇒ `""`, and `scrub_name_list` early-returns `String::new()` on trim-empty ⇒ `""` both sides | r2's I-3, and §3.3's own worked example below |
| `b_1099[].payer` | `kitchen_sink_household` has no `b_1099`; only `amt_owing_household` does (`btctax-core/src/tax/testonly.rs:498`) | r3's I-1 vector A |

★★ **So the sentinel fixture is MAXIMAL, and the constraint is a compile error rather than a
discipline:**

1. **Every `Option` is `Some`, every `Vec` holds ≥ 2 elements, every nested struct is present.** Two
   elements, not one — so an index-varying stand-in (`Payer{i+1}`) and `EinMap`'s distinctness both show.
2. **It is written as an exhaustive struct literal — no `..`, no `..Default::default()` — for all ten
   structs** (§7). A field added anywhere is then a compile error *in the fixture*, not a quietly
   smaller axis. This is the same construction §7 extends the `..`-free guard for, applied to the
   instrument that polices it.
3. **B1 kill-test for the derivation itself:** a fixture variant that leaves `ip_pin: None` must make the
   axis check **RED**. Without it the derivation ships never having been observed discriminating, which
   is the failure class B1 exists for.

★ The value-collision precondition is kept and is *secondary*: §3.4 models it for one field
(`assert_ne!(original.address_city, SCRUB_CITY)`) and it becomes general, one per derived field, so a
sentinel value cannot silently equal its own stand-in.

**The loop then enumerates `{derived field set} × {absent, empty, malformed, valid}`** and every cell
carries one of **three** verdicts:

| verdict | meaning |
|---|---|
| a fixture | the cell is exercised |
| **`no such state`, with the DISCRIMINATOR below as the reason** | the state is unrepresentable for this field |
| nothing | **the defect — fails the loop** |

★★ **The third verdict is not optional, and omitting it is what made r3's version unsatisfiable.**
Demanding a fixture for a malformed `occupation` is demanding the impossible, and the only escape from
an unsatisfiable loop is an exception hand-list — reintroducing precisely what this section removes.
This is the project's own conformance rule: a checker that cannot tell *"this cell encodes no
decision"* from *"we forgot this cell"* is not a conformance check.

★★★ **BUT THE REASON IS NEVER "THE TYPE" — the two axes have DIFFERENT discriminators, and conflating
them licenses exempting a cell on which the scrubber is wrong TODAY.** `header.taxpayer.ssn` is a plain
`String` (`btctax-core/src/tax/return_inputs.rs:197`) and it certainly has a malformed state, so the
type cannot be what decides:

| state | what decides representability |
|---|---|
| `absent` | **the type** — representable iff the field is an `Option` or lives in a `Vec`; mechanically checkable. (So for the `String` SSNs `absent ≡ empty`, both `""` → `SsnError::Missing`, while `w2s[].ein` and `header.ip_pin` are `Option<String>` and distinguish them.) |
| `malformed` | **whether any predicate in the program reads a VALIDITY CLASS off the field** — §3.2's ★ mechanism, not the type. |

**A `no such state` verdict on a `malformed` cell must name the absence of such a reader, never the
type.** There are exactly three such readers today, and they enumerate the malformed-bearing cells:

- `Ssn::canonical` — `btctax-core/src/tax/packet.rs:208` (taxpayer **and spouse**, via `FiledPerson::build`) and
  `btctax-core/src/tax/packet.rs:425` (**every dependent**)
- `IpPin::canonical` — `btctax-core/src/tax/packet.rs:169`
- `canonical_ein` — `return_1040.rs:681`

★★★ **The live vector this closes, which is why the distinction is Important and not pedantic.**
`header.spouse.ssn` and `header.dependents[].ssn` are plain `String`s, so a type-authorised rule marks
their `malformed` cells `no such state` — *correctly, by the wording* — and the loop stays green with
no fixture. But `Ssn::canonical` reads both, and `scrub_dependent` writes `synthetic_ssn(100 + n)`
**unconditionally** (`scrub.rs:111`). So: one dependent whose SSN is eight digits (an ordinary typo) ⇒
the original `ReturnHeader::build` returns `Err(HeaderError::Ssn(SsnError::WrongLength(8)))` and the
filer cannot export — *which is why they run `income scrub`* — while the scrubbed copy canonicalizes
`Ok` and **exports where the original refused**. §3.3 test 2 catches it, but only if the matrix demands
that cell.

The failure this closes, verified live: `screen_inputs:798` refuses on
`foreign_accounts == Some(true) && foreign_country_names.trim().is_empty()`. `scrub_name_list`
preserves that class correctly today and **nothing requires it to** — mutate it to return
`String::new()` unconditionally and the suite stays green, because `kitchen_sink_household` sets
`foreign_accounts: Some(false)` (`btctax-core/src/tax/testonly.rs:352` — there is an unrelated
`testonly.rs` in `btctax-cli`), no other fixture sets it at all, and
`foreign_country_names` was not on the old field axis. That mutation turns a filing return into a
refusing one: the exact inverse of the harm §3.2 exists to prevent.

★ **The existing figure invariant needs one more normalization or it reds on a CORRECT scrub.**
`AbsoluteReturn.excess_ss_not_creditable[].ein` is a scrubbed identity string, and §3.1's fix changes
which employers appear. `blank_identity` must blank each `ein` **and re-sort by `(owner, amount)`** —
the normalization set is stated explicitly in the test, never inherited. Known set: Schedule C header
pair, Form 8995-A `col_a_name`, `excess_ss_not_creditable[].ein`.

★★★ **AND THE FIGURE INVARIANT IS `AbsoluteReturn`-SCOPED, SO IT CANNOT SEE THE PRINTED-FORMS
SURFACE.** `scrub_preserves_every_computed_figure` compares `assemble_absolute`, whose `PrintedInputs`
(`return_1040.rs:1388-1435`) carries **no payer and no country list**. Everything reached only through
`assemble_printed_forms` / `ReturnHeader` is outside every instrument named in §3 — which is precisely
how §5 came to name two divergences when there are more (r2's I-2). **A replaced string that reaches a
printed cell will therefore never announce itself by reddening this test**, so §5's set is maintained
by derivation and not by waiting for a red. Do not "fix" this by widening the invariant: an
`assemble_printed_forms` comparison would red on every scrub, since replacing a printed identity string
is the *point*.

Plus a disclosure test that is **exhaustive, not a hand-list** (r1: 6 of 16 fields named; the entire
spouse untested), asserting `dependents.len()` **before** any `zip`, and mutation-verified with a
spouse-clone plant.

### 3.4 Fixture constants move, not the fixtures

r1: changing fixture addresses to break the Springfield/IL/62704 collision reds three committed
byte-pinned artifacts. **Change the SCRUB constants instead** — one file, zero golden churn, and it
immunises every future fixture. The disclosure test asserts its own precondition
(`assert_ne!(original.address_city, SCRUB_CITY)`) so it cannot pass vacuously.

---

## 4. The artifact must be safe as a FILE

1. **Owner-only write.** `--out` goes through `btctax_store::fsperms`, like every other decrypted
   output; it currently uses bare `std::fs::write` and lands world-readable at 0644. For a
   **pre-existing** path, follow with `fsperms::restrict_file_to_owner`.
2. **Provenance marker** — a **leading comment line** on the emitted raw text, at a fixed
   collision-proof token. Not a TOML key: `parse_return_inputs_toml` rejects unknown keys *before* any
   guard could run.
3. **`income import` refuses a marked file without `--force`** — the guard is a **pre-parse scan of the
   file text** in `import_return_inputs`, not in `parse_return_inputs_toml` (which the round-trip test
   also calls). The flag does not exist today (`cli.rs:399` carries only `--year` and `--file`), so
   this step CREATES it.

   > ★★ **`--force` overrides the provenance-marker guard and NOTHING else. The parked-draft refusal
   > stays unconditional.** `import_return_inputs` already calls `coherence_clear_or_refuse`, which
   > raises `CliError::ParkedDraftBlocksWrite` — the C-1 guard protecting the sole copy of a screened
   > return (`input_form_store.rs:238-254`). A flag named `--force` arriving with no stated scope is an
   > open invitation to gate that refusal on it too, which destroys irreplaceable data and is Critical
   > under this repo's rubric. The scope is one clause and it is written here so it is not inferred
   > (r2 M-3).

★ (2) and (3) are not cosmetic. A scrubbed file is schema-identical to a real one and `income import` is
an unconfirmed whole-blob upsert, so re-importing destroys the vault's real identity and IP PIN —
unrestorable — leaving a synthetic SSN well-formed enough to print on a filed 1040. **Data loss, which
this repo's rubric grades Critical.**

★ Each of the three lands **paired with a planted-defect test** (B1): a mode-0600 assertion on the
written file; an assertion that the emitted string carries the marker; and an import that refuses the
marked file and succeeds with `--force`. A negative test asserts both committed fixture TOMLs still
import clean.

---

## 5. What the help and man page may claim

Permitted once §3 and §4 land:

- "every computed **FIGURE** is preserved"
- "not anonymous: the figures are a financial profile"
- "do not round or fuzz the numbers"
- "load it with `income import --force`"

★ **"computes an IDENTICAL return" is NOT permitted unqualified.** r1 caught the first draft
authorizing it "unconditionally" on a justification covering only the ledger. The help names the
divergences that survive the ledger refusal, and per §5's own rule — *no claim of a guarantee "held by
a test" for a property no test holds* — the identical-figures claim cites the test that holds it.

### 5.1 ★★★ The divergence set is DERIVED, not listed

r2's I-2: the old two-item list was not chosen by a mechanism. It was exactly the two members that
**red an `AbsoluteReturn` comparison** — so the list was a shadow of that test's scope (§3.3), and
every replaced string reaching only the printed-forms surface was missing.

> **The rule: every field `scrub_pii` replaces that reaches a PRINTED CELL or a USER-FACING MESSAGE is
> a divergence, and the help names it — including a replacement that changes a printed cell's
> PRESENCE rather than its contents.**

★★★ **The filter, stated — r3's I-4 was that the table applied one the rule did not state.** The
header identity (`first_name`, `last_name`, `ssn`, `occupation`, `address_*`, `dependents[].name`,
`dependents[].ssn`) is replaced and *does* reach printed cells on Form 1040, and is deliberately **not
enumerated row by row**: the help announces the identity as replaced in one sentence, which is the
whole premise of the command. What the table enumerates is everything **beyond** that sentence — a
replacement a recipient would not predict from "the identity is replaced". Without this clause a future
re-derivation does not reproduce these rows, which is the same defect r2 filed one level up.

★ **Every key is a `ReturnInputs` path — the vocabulary §3.3's derivation emits** — so the table is
checkable against that derivation in both directions. r4 M-1: the excess-SS row was previously keyed
`excess_ss_not_creditable[].ein`, an `AbsoluteReturn` path that `scrub_pii` does not replace, so a
maintainer re-deriving from §3.3 would get `w2s[].ein`, find no row for it, and find a row with no
derivation.

Enumerated from that rule against current source, so a reader can check the derivation:

| replaced field | reaches | class |
|---|---|---|
| `schedule_c.business_description` | Schedule C line A; Form 8995-A line 1(a) (`printed.rs`, `col_a_name`) | **printed** |
| `foreign_country_names` | **Schedule B line 7b, printed verbatim** (`schedule_b.rs:183-192`) | **printed** |
| `int_1099[].payer` | Schedule B Part I payer rows (`printed.rs:1129` → `schedule_b.rs:84`) | **printed** |
| `div_1099[].payer` | Schedule B Part II payer rows (`printed.rs:1142` → `schedule_b.rs:84`) | **printed** |
| **`header.ip_pin`** | **Form 1040's IP PIN cell (`form1040_full.rs:449-450`) — the replacement is `None`, so the cell is not written AT ALL** | **printed (PRESENCE)** |
| `b_1099[].payer` | the `Form1099BNeedsForm8949` refusal text (`return_refuse.rs:672-676`) | message |
| `w2s[].ein` | the excess-SS advisory (`advisories.rs:706`), **via `NonCreditableSs.ein`** | message |
| `header.dependents[].date_of_birth` | nothing — dropped per §6; `DependentRow` carries `name`/`ssn`/`relationship` only (`btctax-core/src/tax/packet.rs:418-428`) and the dependent advisories read `.len()` and `.name` | neither |
| `w2s[].employer` | no printed cell and no computed message; its one reader is the input-form surface echoing the stored value back to its own author (`btctax-input-form/src/spec/sections.rs:642`) | neither |
| `g_1099[].payer` | no printed cell, no message, and no reader at all | neither |

★★ **`scrub_name_list` changes the SHAPE as well as the content**, which the old list would not have
surfaced even had it named the field: it splits on `,` only, so a comma-free entry
(`"Panama and Belize"`) collapses to a single `Country1`, and any other delimiter collapses the count
— on a line the spec elsewhere relies on being count-preserving (§3.2's refusal on an empty 7b).

★ **The residue from §2.2 M-1 belongs here too:** cross-year advisories — M4 carryforward-consistency
among them — do not reproduce from a one-year file, because they read the prior year's stored profile
and the recipient has only one year.

★★★ **The IP PIN row is the one NO instrument in §3 can ever surface, which is why it was missed.**
§3.2's carve-out already says it: *"neither §3.3 test can see it because nothing computes from it."*
Test 2 compares `ReturnHeader::build(..).err()`, and a **valid** IP PIN yields `Ok` on both sides, so
the drop is invisible; the figure invariant is `AbsoluteReturn`-scoped and never reaches Form 1040's
comb. So nothing will ever red on it. The vector is live: this repo has `wrap.rs`, `overflow.rs` and a
read-back `verify.rs`, so "my IP PIN prints outside its comb" is a real class of report — and on the
scrubbed copy `form1040_full.rs:449` never fires, so the cell is simply blank and the maintainer
cannot reproduce it. **A cell that goes MISSING is a divergence exactly as much as one whose contents
change**, and the rule above now says so.

★ The last two rows are *not* divergences and are recorded so the next reader does not re-derive them:
a field that reaches no printed cell and no computed message cannot make the recipient's copy behave
differently. They stay in the table because the derivation must be checkable in both directions —
"replaced but harmless" is a conclusion, not an omission. ★ Note `w2s[].employer` is not read by
*nothing*: the input-form surface echoes it back to the person who typed it
(`btctax-input-form/src/spec/sections.rs:642`). That is not a divergence in §5's sense — the recipient's
own copy shows the recipient's own stored value — but the row says so rather than claiming an absence
of readers that is not true (r3's Nit).

---

## 6. Decided (was an open question)

**`absent → absent` stands.** r1 showed the alternative is not a UX call but a correctness regression
reproducing one of its own confirmed defects. A filer with no SSN yet gets a scrubbed file whose
`ReturnHeader::build` refuses — for both copies, identically, which is the point. The recipient's
remedy is recorded in the help: `report` needs no SSN, and an export needs any nine digits added to the
scrubbed copy.

**Dependent `date_of_birth` is DROPPED (`None`), not quantized.** Nothing reads it — the CTC is not
computed — so retaining it is retention, not computation. The scrub.rs comment claiming "both are read"
is false and is corrected in the same step.

---

## 7. Findings that need their own step

r1 found four items no other section reaches:

- The `..`-free destructure guard covers **4 of 10** structs; the six it misses hold the free text scrub
  must replace. Give each the same treatment, and correct `scrub.rs:14`, which overclaims.
- ★ **A THIRD `scrub.rs` comment is wrong and must be corrected in the same step (r3's I-2).**
  `scrub.rs:246-250` orders `business_description` to *"stay NON-EMPTY"* unconditionally. That is sound
  only when the original was non-empty; applied to a trim-empty original it is the instruction that
  produces the refusal-losing vector in §3.2. Rewrite it to the actual rule — **non-empty in, stand-in
  out; trim-empty in, `""` out** — because a red matrix cell will otherwise be argued away against the
  comment rather than fixed. (The other two corrections: `scrub.rs:14`'s destructure overclaim, above,
  and the middle-group-`00` claim, below.)
- `w2s[].box12[].code` has no recorded decision: **kept** — a box-12 code is a taxonomy, not a person.
- **Synthetic EINs red the repo's own `pii-scan`.** ★★★ **DECIDED HERE — take the narrow option. There
  is no structural EIN window, so the generator-keyed allowlist is not available at any price.**

  r2's I-4: the two options this bullet used to offer were presented as equivalent and are not. A rule
  "keyed to the generator" means `^9[0-9]-[0-9]{7}$`, because `synthetic_ein(n) = format!("9{}-{:07}",
  n % 10, n + 1)` spans the entire `9x` prefix space — and that exempts **real** EINs issued under 91,
  94, 95 and 99 from a scan whose whole purpose is stopping a real taxpayer identifier from reaching a
  public repo. Both the code and the scanner had already recorded the premise: `scrub.rs:57-58` ("there
  is no 'impossible EIN' … merely synthetic, not provably unissued") and the scanner's own EIN block
  ("No structural rule is available … Token-exact … remains correct here"). The SSN rule works only
  because group `00` is provably never issued; there is no EIN analogue. Widening an exemption is never
  the safe edit, and a build step whose two branches differ in *whether a security guard is weakened*
  must not ship.

  **So:** narrow the `scrub.rs` module claim to SSNs — the middle-group-`00` sentence is true of
  `synthetic_ssn` and false of `synthetic_ein`, and today's doc lets the reader carry it across.
  **Committing a scrubbed return as a repo fixture is OUT OF SCOPE for v1** and the spec says so
  plainly rather than leaving a blocked workflow implied. If it is ever wanted, it is its own decision,
  and it must state the real-EIN intersection of whatever window it proposes **in writing** before a
  line of it is built. The three synthetic EINs already quoted in persisted reviews stay admitted by
  citation in their existing self-limiting bucket (`ALLOWED_EIN_REVIEW_ARTIFACT`); that bucket is
  residue and does not grow.
- **Draft/parked coherence, decided here rather than deferred:** `scrub_return_inputs` routes through
  `input_form_store::load` like every other reader of the committed row, and scrubs the
  `Loaded::Draft` when one exists.

---

## 8. Build order

1. This spec → review → 0C/0I. *(r5 is this document.)*
2. `ledger_contributes` made public with its obligation doc; the two helpers it calls widened to
   `pub(crate)` **in their own two modules** (§2.2 ★); ledger projection added to the scrub path;
   §2.2 refusal as a **`CliError`**, not a `RefuseReason`.
   ★ r1: a new `RefuseReason` is a category error — that taxonomy is *why a return cannot be FILED*, and
   its exhaustive cross-crate anchor map in `btctax-input-form` has no honest entry for a sharing
   refusal. One RED-first test per disjunct: an unclassified-inbound pseudo vault, and an out-of-year
   Hard blocker, must each refuse.
3. §3.1 canonical-EIN partition + RED-first test, **together with §3.3's figure-invariant normalization
   (`excess_ss_not_creditable`, blank + re-sort) in the same step.**
   ★ r2 M-2: these were steps 3 and 5 and are now one. §3.3 says the normalization is what stops the
   figure invariant redding on a *correct* scrub — so if step 3's RED-first vector (two spellings of
   one employer over the SS cap) goes into the figure-invariant loop, which is the natural place for
   it, splitting them leaves the suite red across two gates. A red suite is itself a blocking finding
   here.
4. §3.2 validity-class preservation — the IP PIN carve-out, the `NotDigits(_) → NotDigits('x')`
   coarsening, and **trim-emptiness preservation on every string `scrub_pii` replaces** (not the
   payer/employer loops only — `business_description` is the one that matters) — plus the two §3.3
   class-level tests, each RED first, over the §3.3 fixture matrix.
   ★★★ **BUILD THE AXIS DERIVATION FIRST, AND WATCH IT GO RED (B1).** Before any matrix row exists,
   land the derivation of §3.3 — `serde_json::to_value(ri)` vs `to_value(scrub_pii(&ri))` over the
   **maximal** sentinel fixture, presence-difference included — and **print the derived path set**. It
   must contain **`w2s[].ein`, `header.ip_pin`, `foreign_country_names` and `b_1099[].payer`**. If it
   does not, the fixture is not maximal and the derivation is blind on the four fields four review
   rounds were spent on. Pair it with its kill-test: a fixture variant leaving `ip_pin: None` must make
   the axis check **RED**. This is the step that converts the last four rounds of argument into a
   command, and it is why no fifth prose round was run.
   ★ The axis is derived, never a `const` list, and never the four fields §3.2 happens to discuss.
   ★ **The fixtures the matrix needs are built HERE, in this step, not later** (r3 M-2): a `b_1099`
   fixture exercising the payer loop, and a `foreign_accounts: Some(true)` fixture, since **no fixture
   is in that class today** and the `scrub_name_list` mutation ships green without one. Scheduling them
   after the test that fails without them leaves the suite red across a gate, which is itself a
   blocking finding here — the same defect merging old steps 3 and 5 fixed.
5. §3.4 scrub constants moved; exhaustive disclosure test; a fixture with
   `date_of_birth`/`blind`/`date_of_death` set so §3's KEPT fields are load-bearing on at least one
   vector.
6. §6 dependent-DOB drop + comment correction; §7's items — including **all three `scrub.rs` comment
   corrections**: the `:14` destructure overclaim, the `:246-250` unconditional non-empty order, and
   narrowing the middle-group-`00` claim to SSNs.
7. §4 artifact safety, each with its planted-defect test. `--force` is created here, scoped to the
   marker guard alone (§4.3).
8. TOML round-trip test; help + man regenerated to §5 — the claims name every member §5.1's table
   classes as **printed** or **message** (the two `neither` rows are part of the derivation's audit
   trail, not divergences, and do not belong in user-facing help).
9. Whole-branch `main..HEAD` review to 0C/0I, briefed on what **r1–r4** covered — all four are persisted
   in `reviews/`, with CLEAN sections recording what each could not break — so it spends its budget on
   the seams (harness B3). ★ Under-stating that count is not cosmetic: B3's whole point is telling the
   last pass what the earlier rounds already held.

Ships alone, in its own release.
