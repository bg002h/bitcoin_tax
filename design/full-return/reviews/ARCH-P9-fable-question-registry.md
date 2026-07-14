# Fable — architecture consult: "is there an architectural problem?" (P9)

*Persisted VERBATIM before folding. Asked because the project owner said: "This is a lot of reviews for a
spec. Ask fable if there is an architectural problem." Author of the question = Opus; answer = Fable.*

**Verdict: YES — one specific defect. The engine's architecture is otherwise sound. The recommendation is
a QUESTION REGISTRY, and explicitly NOT a `ScreenedInputs` witness type.**

---

# Verdict

**Yes, there is an architectural defect — one, specific, and it is not "the architecture" of the engine.** The compute pipeline, the frozen seam, the single-resolver ladder, and the printed-chain are sound; the reviewer said so on the record four separate times ("None is architectural", "architecturally sound", "no change to the architecture… required", "the core printed-chain architecture is genuinely good"), and my own read of the source agrees.

The defect is this: **"an input with no safe default must be answered before it reaches compute or print" is a load-bearing invariant of this system, and it is the only load-bearing invariant in the codebase still enforced by convention instead of construction.** This codebase already enforces its other invariants structurally: money non-negativity by exhaustive destructure (`first_negative_amount`, no `..`), packet completeness by `PrintedReturn`'s no-`..` destructure, rounding regime by the `Printed8283Rows` newtype, SSN validity by `Ssn`'s private field with `canonical()` as sole constructor. Answered-ness got none of that. It got five hand-wired obligations across four crates, and the ledger shows each has been forgotten at least once.

But the review volume is **not** primarily evidence of that defect, and you should not conclude the whole program was fighting one architecture problem. Three corrections to the premise:

1. **"38 reviews" overstates defect volume by roughly 2x.** 13 of the 38 artifacts are pure fold-verification rounds that found **zero** new blocking findings (SPEC r4, PLAN r2, P0 r2, P1 r3, P2 r3+r4, P3 r2, P4 r2, P4.9 r2, P5 r2, P6 r3, P7 r4+r5). They exist because your workflow mandates re-review after every fold. Three more are consults, not reviews. About 21 rounds actually found defects.

2. **The findings are genuinely heterogeneous.** Across all ~98 blocking findings (11C/86I): wrong tax law is ~20% and owns 5 of the 11 Criticals (QSS-is-not-a-joint-return alone struck three separate times — §904(j), §221, §3101(b) — that is a knowledge recurrence, not an architecture one); form-completeness ("the money lines are right" concealing a blank cell) is ~13%, and it *stopped* once the ARCH citation-walk attacked it systematically; test-rigor findings ~11%; spec/plan artifact defects ~12%. That part of the volume is what a 0C/0I gate costs on the IRC. It was buying real things: the QSS Critical was a silent $360 understatement on a filed return.

3. **The largest single recurring class — ~21% — is exactly the one you named**, and it is the only class that recurred at every stage with identical mechanics and was hand-patched per-instance every time: `foreign_accounts: bool` auto-checking 7a "No" (SPEC r1 I7, **day one**), the captured-but-never-consumed spouse flag (P3 r1 I1), D-8 itself, and then all three of P8a's Importants (scope divergence, untested migration guard, packet projection). The P8 spec's seven rounds are the pathology concentrated: r3–r6 are one absence problem — launder, wrong scope, wrong migration object, "the bug reconstituted itself out of its own fix" — re-patched round after round. Your coordinator's reading is confirmed in substance, with one sharpening: the reason it took four-plus rounds is not merely that absence was representable, it is that the fix had to be **restated by hand in five places**, and each round caught one of the five wrong. Program-wide, ~12% of all blockers were introduced by folds — hand-wiring N coupled edits under review pressure is precisely how that happens.

# On your ternary instinct

You said: *"we need some form of ternary instead of binary storage: not answered/no/yes."* Your instinct is right — **and it is already implemented.** All five fields are `Option<bool>` today (`None` = never asked, `Some(false)` = answered no) at `crates/btctax-core/src/tax/return_inputs.rs:179,183,383,396,399`. Do not credit the ternary as the fix, or the next review cycle will re-teach the same lesson. The ternary is necessary and insufficient, because:

- **Nothing forces it.** `can_be_claimed_as_dependent_taxpayer` was a bare `bool` for eight phases and nothing objected. The next yes/no field can be a `bool` tomorrow, and `#[serde(default)]` will fabricate a "No" out of silence again.
- **Even with the ternary, five obligations stay hand-wired**: optional field, refusal, refusal *scope*, safe consumers, a test that the refusal fires, and the `income answer` prompt entry. Each is independently forgettable and each has been forgotten.
- **Scope is written twice** — once in `screen_inputs`, once in `live_questions` — so they can disagree. P8a I1 is the proof: the refusal predicate dropped the MFJ disjunct that the spec required, and only a review caught it.

# Recommendation: the question registry — yes, build it

The registry proposal is correct, and it is the right *single* move. Concretely, in a new `crates/btctax-core/src/tax/questions.rs`:

```rust
/// A yes/no the 1040 asks that has NO safe default. ONE entry per question;
/// the refusal, its scope, the prompt, and the accessor all live here and
/// nowhere else.
pub struct FormQuestion {
    pub id: QuestionId,                        // exhaustive enum
    pub prompt: &'static str,                  // form-phrased, used by `income answer`
    pub unanswered: RefuseReason,              // the refusal when live and None
    pub live: fn(&ReturnInputs) -> bool,       // THE liveness predicate — the only copy
    pub get: fn(&ReturnInputs) -> Option<bool>,
    pub set: fn(&mut ReturnInputs, bool),
}
pub const FORM_QUESTIONS: &[FormQuestion] = &[ /* five entries */ ];
```

Call-site diff shapes:

- **`return_refuse.rs`** — four hand-written blocks (`DependentStatusUnanswered`, `DependentSpouseStatusUnanswered`, `MfsSpouseItemizeUnknown`, `ScheduleBPart3Unanswered`) collapse to one loop: `for q in FORM_QUESTIONS { if (q.live)(ri) && (q.get)(ri).is_none() { return refuse(q.unanswered.clone(), …) } }`. Value-dependent refusals (`ForeignTrust == Some(true)`, `DependentSpouseUnsupported`) stay hand-written — they are domain rules, not answered-ness.
- **`cmd/answer.rs`** — `Question`, `live_questions`, `current_bool`, `set_bool` all become registry iteration; the enum and three match blocks delete. The brick-risk property ("everything the screen refuses is askable") becomes true by identity instead of by test.
- **The classifier** — extend the `first_negative_amount` pattern: a function destructuring `ReturnInputs` *and* `HouseholdHeader` with no `..`, requiring every `bool`/`Option<bool>` field to be either a registry entry or on a named exemption list carrying the criterion the SPEC r3 review demanded ("absence fails loud, or is conservative *and* advised"). A new field is a compile error until a human classifies it.
- **One property test** replaces per-question refusal tests forever: for each registry entry, build a return where it is live, blank it → assert refuses; answer it → assert clears. Future questions are covered the day they are registered — this kills the untested-guard recurrence for this class.

**What the claim gets right:** it kills D-8's declaration class (via the classifier), kills I1 structurally (one predicate cannot disagree with itself), kills the brick risk and prompt drift (one list). **What it does not fix, honestly:** the print boundary (`packet.rs:321`) — close that with P8a I3's own fix, `ReturnHeader::build` returning `Err` when a live flag is `None` (one line plus a test, and it is already owed under the open 0C/3I review); tax-law wrongness — nothing architectural fixes the IRC; and one caution from your own repo: the classifier's force is "compile-error-until-a-human-*decides*", not until-correct — `first_negative_amount` demonstrated the escape hatch when `header: _` waved off the whole header (the Important you just filed). Watch `_` arms in review; that is the residual convention.

**Cost:** ~150-line registry module, net churn maybe 300–400 lines across two crates (`btctax-core`, `btctax-cli`). No frozen file is touched — `types.rs`/`compute.rs`/`se.rs` do not know `ReturnInputs` exists. No schema change, no migration. `resolve.rs` and the delta/derivation path are untouched: `screen_inputs` keeps its exact signature. One cycle under your workflow, and it subsumes the just-filed `header: _` follow-up.

**On `ScreenedInputs`, explicitly not now.** I checked the graph you asked about: it *would* work — `resolve_core` is already the choke point; `admin.rs:export_full_return` hand-replays the screens and would thread a witness naturally; it would even force fixing a real wart (`cmd/tax.rs:240` re-fetches the row *after* `resolve_and_screen` already screened a copy, so the dual report trusts "same bytes" by convention). But it is the answer to a class with exactly one Important on the ledger (P8a I3) and zero shipped wrong returns, at ~15 signature changes plus dozens of test sites — and it cannot prevent the next D-8, because a witness certifies only that the *existing* screens ran, not that the right screens *exist*. The recurring failure was missing and mis-scoped screens, never skipped ones. With the registry plus the `ReturnHeader::build` refusal, every consumer is locally fail-closed for this class and the witness's marginal value drops below its churn. Revisit only if a second computing consumer appears (e.g. the TUI ever computes returns directly off raw inputs).

**Bottom line:** ~60% of your review volume was the honest price of tax law under a 0C/0I gate, and a third of the rounds were mandatory re-verification, not defects. But one fifth of it was a single invariant being re-verified by humans because the type system was never asked to hold it — named in the very first spec review and hand-patched for eight phases. The registry asks the compiler to hold it. Build that, fold the packet-boundary refusal from the open P8a review, and skip the witness type.
