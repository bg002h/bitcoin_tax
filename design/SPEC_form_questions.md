# SPEC — P9: the FORM QUESTION REGISTRY

*Status: r1, awaiting independent review. Supersedes the hand-wired answered-ness machinery.*
*Origin: `design/full-return/reviews/ARCH-P9-fable-question-registry.md` (Fable), asked at the owner's
direction. Owner approved building it.*

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

To add one yes/no field today, an author must independently remember to:

1. declare it `Option<bool>`, not `bool` (nothing enforces this);
2. hand-write an "unanswered" refusal in `screen_inputs`;
3. get that refusal's **scope** right (spouse questions only when a spouse exists, …);
4. make every consumer read it safely (`return_1040.rs` ×2, `packet.rs`);
5. write a test that the refusal actually **fires**;
6. add it to `income answer`'s prompt list — *or the year is bricked with no way to answer it.*

**Each of these has been forgotten at least once, and the ledger is not ambiguous:**

- Step 1 forgotten → **D-8**: `can_be_claimed_as_dependent_taxpayer` was a bare `bool` with
  `#[serde(default)]` for eight phases. Silence became an answered "No", which took the full standard
  deduction instead of the §63(c)(5) dependent floor, skipped the §1(g) kiddie-tax refusal, and printed an
  **unchecked box on a filed 1040 that the filer never affirmed**. Shipped, in v0.2.0.
- Step 1, the first time → **SPEC review r1, finding I7, day one of the program**: `foreign_accounts: bool`
  auto-checking Schedule B line 7a "No". *The identical defect, named at the very beginning, and then
  hand-patched per-instance for eight phases instead of being closed as a class.*
- Step 3 → **P8a r1 I1**: my own D-8 refusal dropped the MFJ disjunct the spec required, so an MFJ return
  with no spouse *Person* captured passes the screen unanswered and prints the spouse box unchecked.
- Step 5 → **P8a r1 I2**, and twice before it (§199A over-threshold; `ScheduleCNoBusinessDescription`):
  guards shipped with **zero** tests. Deleting them left the whole suite green.
- Step 4 → **P8a r1 I3**: the print boundary projects `Option<bool>` → `bool` trusting a screen two crates
  away, pinned by nothing.

This class is **~21% of every blocking finding in the program** — the largest single class, and the only one
that recurred at *every* stage with identical mechanics.

### 1.1 What this spec does NOT claim

Fable's audit corrected the motivating framing, and the correction belongs in the spec so nobody re-derives
a false lesson from it:

- The review volume is **not** mostly this defect. ~60% of blocking findings are the honest price of the
  IRC under a 0C/0I gate (wrong tax law is ~20%, and owns 5 of 11 Criticals). A third of the review
  *artifacts* found nothing — they are the re-verification rounds our own workflow mandates.
- **The engine's architecture is sound.** The compute pipeline, the frozen seam, the resolver ladder and
  the printed-chain are not in question and are not touched here.
- **The ternary is not the fix, and is already implemented.** All five fields are `Option<bool>` *today*.
  Anyone who reads this spec and concludes "so we need a tri-state" has learned nothing: we have one. The
  gap is that nothing *forces* it and the obligations are hand-wired.

## 2. The design

A single registry. **One entry per question, owning every obligation.**

```rust
// crates/btctax-core/src/tax/questions.rs   (NEW)

/// A yes/no the return asks that has **no safe default** — one where guessing is not conservative in
/// either direction, so the only honest behaviour is to refuse until the filer answers.
///
/// ONE entry per question. The prompt, the refusal, the liveness scope, and the accessors live here and
/// NOWHERE ELSE. `screen_inputs`, `income answer`, and the print boundary all DERIVE from this list —
/// they do not restate it. Restating it is what let the refusal scope and the prompt scope disagree
/// (P8a r1 I1), and what let a question be refusable but unaskable (a bricked year).
pub struct FormQuestion {
    pub id: QuestionId,
    /// Form-phrased, because the filer is answering a 1040 line, not a struct field.
    pub prompt: &'static str,
    /// The refusal raised when this question is LIVE and unanswered.
    pub unanswered: RefuseReason,
    /// ★ THE liveness predicate — the only copy in the codebase.
    pub live: fn(&ReturnInputs) -> bool,
    pub get: fn(&ReturnInputs) -> Option<bool>,
    pub set: fn(&mut ReturnInputs, bool),
}

/// Exhaustive. A `match` on this is how the classifier (§2.3) forces a new field to be classified.
pub enum QuestionId {
    DependentTaxpayer,
    DependentSpouse,
    MfsSpouseItemizes,
    ForeignAccounts,
    ForeignTrust,
}

pub const FORM_QUESTIONS: &[FormQuestion] = &[ /* five entries */ ];
```

### 2.1 The three derivations

**`screen_inputs` (`return_refuse.rs`).** Four hand-written unanswered-blocks collapse to one loop:

```rust
for q in FORM_QUESTIONS {
    if (q.live)(ri) && (q.get)(ri).is_none() {
        return refuse(q.unanswered.clone(), /* text derived from q.prompt */);
    }
}
```

**Value-dependent refusals stay hand-written** and are explicitly NOT registry business: `ForeignTrust ==
Some(true)` → Form 3520; `DependentSpouseUnsupported` (a claimable spouse limits the joint standard
deduction). Those are **domain rules about the answer**, not about *whether there is* an answer. Conflating
the two would be the same category error in the other direction.

**`income answer` (`cmd/answer.rs`).** `Question`, `live_questions`, `current_bool`, `set_bool` all delete;
they become registry iteration. The no-brick property — *everything the screen can refuse for is askable* —
stops being a test and becomes **true by identity**.

**The print boundary (`packet.rs`).** `ReturnHeader::build` already returns `Result`. It gains: if a
registry question is live and `None`, **`Err`**. This is P8a r1 I3, and it is the one place a wrong value
becomes a false statement on a filed PDF. At print there is *no* conservative direction — an unchecked box
is a false "No" and a checked box is a false "Yes" — so refusal is the only fail-closed behaviour.

### 2.2 The property test that replaces per-question tests forever

```
for each entry in FORM_QUESTIONS:
    build a return where the question is LIVE
    blank it            → assert screen_inputs refuses, with THIS entry's RefuseReason
    answer it (n and y) → assert that refusal is gone
    assert income answer ASKS it (it is in live_questions for that return)
```

This is the anti-`untested-guard` machine: a question registered tomorrow is covered the day it is
registered. It directly retires the recurrence that produced three zero-test guards.

### 2.3 The classifier — the part that actually stops the next D-8

The registry alone does not prevent someone from adding a sixth yes/no field and never registering it.
That is what D-8 *was*. So, extending the `first_negative_amount` pattern:

A function destructuring **`ReturnInputs` AND `HouseholdHeader`** with **no `..`**, in which every
`bool` / `Option<bool>` field must be classified as either

- **a registry question** (fails loud when unanswered), or
- **exempted**, on a named list, each carrying its written criterion:
  *absence must be **conservative** (it can only overstate tax) **and** advised (the report says so).*

A new `bool` on either struct is then a **compile error** until a human classifies it.

⚠️ **Honest limit, from Fable, recorded so we do not oversell this:** the classifier's force is
"compile-error-until-a-human-**decides**", not "until-correct". `first_negative_amount` already demonstrates
the escape hatch — `header: _, // PII only — no money` waves off the *entire* `HouseholdHeader`, so that
struct is **not** exhaustively destructured and the stated guarantee has a hole (filed as an Important).
**Wildcard arms are the residual convention, and must be treated as review-visible.** This spec therefore
requires: **no `_` arm in the classifier** — every field named.

### 2.4 Explicitly NOT doing: the `ScreenedInputs` witness

A newtype produced only by `screen_inputs`, so compute/print can only accept screened input, was
considered and **rejected for now**. It would work (`resolve_core` is already the choke point). But:

- it answers a class with **one** Important on the entire ledger (P8a I3) and **zero** shipped wrong
  returns, at ~15 signature changes plus dozens of test sites; and
- **it cannot prevent the next D-8.** A witness certifies that the *existing* screens ran — not that the
  *right screens exist*. Every recurrence in this program was a **missing or mis-scoped** screen, never a
  **skipped** one. The witness is armour against a bullet nobody has fired.

With the registry + the `ReturnHeader::build` refusal, every consumer is locally fail-closed for this class.
Revisit only if a second computing consumer appears (e.g. the TUI computing returns off raw inputs).

## 3. Acceptance

- **All five existing questions are registry entries.** `screen_inputs`, `income answer`, and
  `ReturnHeader::build` derive from the registry; none restates a liveness predicate.
- **P8a I1 dies structurally**: refusal scope and prompt scope are the same `fn`, so they *cannot* disagree.
  The MFJ-with-no-spouse-Person hole closes as a consequence, not as a patch.
- **P8a I3 dies**: an unanswered live question cannot reach a printed form — `ReturnHeader::build` refuses.
- **A new unregistered `bool` on `ReturnInputs` or `HouseholdHeader` does not compile.**
- The §2.2 property test passes for every entry, and **mutation-checked**: delete the registry loop in
  `screen_inputs` → a named test fails; delete the `build` refusal → a named test fails; drop an entry from
  `FORM_QUESTIONS` → a named test fails.
- `make check` green; **0 Critical / 0 Important** from independent review.
- FROZEN (`tax/{types,compute,se}.rs`) unchanged. No schema change. No migration. `screen_inputs` keeps its
  signature; `resolve.rs` and the delta path are untouched.

## 4. Build order (TDD; each step red → green)

1. **`questions.rs`**: `QuestionId`, `FormQuestion`, `FORM_QUESTIONS` with the five entries — liveness
   predicates lifted from the *current* refusals, **except** `DependentSpouse`, whose predicate is corrected
   to `filing_status == Mfj || header.spouse.is_some()` (that correction **is** P8a I1).
2. **The §2.2 property test** — RED against the hand-wired screen for the I1 case, then GREEN.
3. **`screen_inputs`** derives from the registry; the four hand-written unanswered-blocks delete. The
   value-dependent refusals stay.
4. **`cmd/answer.rs`** derives from the registry; `Question`/`live_questions`/`current_bool`/`set_bool`
   delete. DOBs are **not** registry questions (they are skippable and are not yes/no) — they stay
   hand-written, and the spouse-DOB prompt must remain gated on `header.spouse.is_some()`, because
   `set_date` silently discards a spouse DOB when there is no spouse Person.
5. **`ReturnHeader::build`** refuses a live-and-unanswered question (P8a I3).
6. **The classifier**, with no `_` arm, and the exemption list with written criteria. Fold the open
   `header: _` follow-up here — `HouseholdHeader` gets destructured too.
7. `LIMITATIONS.md`: the refusal list is now generated from one place; no user-visible change expected,
   but verify the four refusal texts still name `btctax income answer`.
