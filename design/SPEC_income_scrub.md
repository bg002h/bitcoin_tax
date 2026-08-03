# SPEC — `btctax income scrub`

*Ceremony scaled per `STANDARD_WORKFLOW.md` §8: one spec, no separate plan (build order is §8). The
gates are NOT scaled: this reaches 0C/0I before a line of the rework is written, then again on the
whole branch before it ships.*

**Status:** DRAFT r2. Supersedes the unreviewed behaviour on `feat/income-scrub` (`31d5c79`,
`2449ee4`), held back from every release pending this spec.
Prior art: `reviews/scrub-r1-workflow.md` (code review, 19 confirmed + 6 sweep),
`reviews/scrub-r2-fable-consult.md` (scope adjudication), `reviews/scrub-spec-r1-review.md`
(**this spec's r1: 31 raw → 19 blocking**, all folded below).

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
   || digital_asset_activity(state, year - 1)      // M4 carryforward-consistency advisory
   || state.pseudo_active()                        // DRAFT watermark + attestation gate
   || first_hard_blocker(state).is_some()          // NotComputable, projection-wide
```

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

★ `digital_asset_activity` and `first_hard_blocker` are module-private today; making
`ledger_contributes` public is a §8 step, and its doc names both callers so neither drifts. The scrub
path must also **project the ledger**, which it currently never does (`Session::project()`); if
projection fails, scrub **refuses** — it never falls back to "assume empty".

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
`NotDigits → NotDigits`.

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
2. `ReturnHeader::build(orig).err()` and `..(scrubbed).err()` are the **same `HeaderError` variant, and
   the same `SsnError` variant inside it** — not merely `is_err()` equality, which admits error
   substitution.

★★ **Both are VACUOUS on today's fixtures** (all three would compare `None == None`). §3.3 therefore
imposes a **fixture matrix obligation**, not a count: the loop enumerates
`{SSN, EIN, IP PIN, business_description} × {absent, malformed, valid}` and **fails on any cell with no
fixture**. Enumerate from the matrix; never from a hand-list.

★ **The existing figure invariant needs one more normalization or it reds on a CORRECT scrub.**
`AbsoluteReturn.excess_ss_not_creditable[].ein` is a scrubbed identity string, and §3.1's fix changes
which employers appear. `blank_identity` must blank each `ein` **and re-sort by `(owner, amount)`** —
the normalization set is stated explicitly in the test, never inherited. Known set: Schedule C header
pair, Form 8995-A `col_a_name`, `excess_ss_not_creditable[].ein`.

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
   also calls).

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
authorizing it "unconditionally" on a justification covering only the ledger. Two divergences survive
the ledger refusal: `business_description` is replaced and **printed** on Schedule C line A and Form
8995-A line 1(a), and the EIN in the excess-SS advisory. The help names them. And per §5's own rule —
*no claim of a guarantee "held by a test" for a property no test holds* — the identical-figures claim
cites the test that holds it.

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
- `w2s[].box12[].code` has no recorded decision: **kept** — a box-12 code is a taxonomy, not a person.
- **Synthetic EINs red the repo's own `pii-scan`**, so the intended workflow (commit a scrubbed return
  as a fixture) is blocked. Mint them inside a documented structural window with a matching
  `ALLOWED_EIN_SYNTHETIC` rule keyed to the generator and paired with a planted-defect test — or narrow
  the scrub.rs claim to SSNs and say the EINs need an allowlist entry.
- **Draft/parked coherence, decided here rather than deferred:** `scrub_return_inputs` routes through
  `input_form_store::load` like every other reader of the committed row, and scrubs the
  `Loaded::Draft` when one exists.

---

## 8. Build order

1. This spec → review → 0C/0I. *(r2 is this document.)*
2. `ledger_contributes` made public with its obligation doc; ledger projection added to the scrub path;
   §2.2 refusal as a **`CliError`**, not a `RefuseReason`.
   ★ r1: a new `RefuseReason` is a category error — that taxonomy is *why a return cannot be FILED*, and
   its exhaustive cross-crate anchor map in `btctax-input-form` has no honest entry for a sharing
   refusal. One RED-first test per disjunct: an unclassified-inbound pseudo vault, and an out-of-year
   Hard blocker, must each refuse.
3. §3.1 canonical-EIN partition + RED-first test.
4. §3.2 validity-class preservation (with the IP PIN carve-out) + the two §3.3 class-level tests, each
   RED first, over the §3.3 fixture matrix.
5. §3.3 figure-invariant normalization (`excess_ss_not_creditable`, blank + re-sort).
6. §3.4 scrub constants moved; exhaustive disclosure test; `b_1099` fixture to exercise the payer loop;
   a fixture with `date_of_birth`/`blind`/`date_of_death` set so §3's KEPT fields are load-bearing on at
   least one vector.
7. §6 dependent-DOB drop + comment correction; §7's four items.
8. §4 artifact safety, each with its planted-defect test.
9. TOML round-trip test; help + man regenerated to §5.
10. Whole-branch `main..HEAD` review to 0C/0I, briefed on what r1 covered so it spends its budget on the
    seams (harness B3).

Ships alone, in its own release.
