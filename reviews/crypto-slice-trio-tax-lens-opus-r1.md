# Whole-diff review — crypto-slice trio + FBAR reversal — TAX-CORRECTNESS lens (Opus, r1)

**Date:** 2026-07-30 · **Branch:** `feat/no-pen-deferrals` · **Range:** `7bde148..65270db`
(commits `1a757f0`, `3e16f85`, `cbe651d`, `65270db`).

**Brief:** one question — *does any change put a WRONG FIGURE, a FABRICATED ANSWER, or an
UNJUSTIFIED REFUSAL/NON-REFUSAL on a return a filer signs?* Settled facts supplied so they would not
be re-derived (line 20's Form 4952 conjunct; the 2017 vs 2024/2025 on-states; the deliberate
`frozen_guard` pin bump; the green suite). Fresh-audit scope creep, style and prose forbidden.

**Reviewer output is reproduced VERBATIM below.** Nothing edited, reordered, or summarised.

---

VERDICT: findings

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/compute.rs:405 (+ crates/btctax-core/src/tax/types.rs:109)
CLAIM: The `nii_with >= Usd::ZERO` conjunct is evaluated against the model's PARTIAL net investment income (QD + capital gains + crypto interest only), not the filer's NII, so in a net-capital-loss year the headline all-in LTCG rate omits 3.8 points that the next long-term dollar actually owes — the under-reserve direction.
FAILURE: Single filer. `tax-profile`: ordinary_taxable_income 250,000; magi_excluding_crypto 300,000 (of which $60,000 is rental income — NII the profile has no field for); qualified_dividends_and_other_pref_income 0. Crypto: net short-term loss $80,000. Then `nii_with = 0 + 0 + 0 − 3,000 = −3,000 < 0`, `magi_with = 297,000 > 200,000` ⇒ `niit_at_margin = false` ⇒ `report --tax-year` prints `LTCG 0.15 all-in (§1(h) 0.15 + §1411 0)`. Truth: real NII = 60,000 − 3,000 = 57,000; NIIT base = min(57,000, 97,000) = 57,000 > 0, so the next LT dollar raises the base and owes 3.8%. The correct headline is 0.188. The previous display made no forward claim about §1411 at all; the new one affirmatively prints "§1411 0".
EVIDENCE: `crates/btctax-core/src/tax/compute.rs:361-363` — `let nii_with = qd + with.ordinary_gain + with.preferential_gain - with.loss_deduction + interest_nii;` — and `crates/btctax-core/src/tax/types.rs:32-68`, whose `TaxProfile` has `ordinary_taxable_income`, `magi_excluding_crypto`, `qualified_dividends_and_other_pref_income`, `other_net_capital_gain`, `capital_loss_carryforward_in`, `w2_*`, `schedule_c_expenses` — and no general NII field. Display-only (see verified list), which is why this is Minor and not Important.
```

```
SEVERITY: Minor
WHERE: crates/btctax-forms/forms/2025/schedule_d.map.toml `[line17]` / crates/btctax-forms/tests/kats.rs:402-404
CLAIM: Nothing in the suite can distinguish a correct `[line17]` map from one whose `yes`/`no` FIELD names are swapped, because every assertion reads the widget through `pair.yes.field` / `pair.no.field` — the map is both the thing under test and the test's own index.
FAILURE: Swap only the two `field =` values in `crates/btctax-forms/forms/2025/schedule_d.map.toml` `[line17]` (leaving `on = "1"` / `on = "2"` in place). `apply_writes` then sets `/AS = /1` on `c2_1[1]`, whose only real on-state is `/2`; the widget has no `/AP /N /1` entry, so every filed 2025 Schedule D renders line 17 **blank** while `checkbox_on` reads the raw `/AS` back as `Some("1")` and `schedule_d_line17_is_derived_on_every_revision` stays GREEN. Both goldens simply re-pin. Net effect: a "blank because the map is wrong", the second row of the CLAUDE.md provenance table, invisible to the suite. (I verified the CURRENT maps are correct — `xtask dump-fields` gives 2025 `c2_1[0]` at y 578-586 on=["1"], `c2_1[1]` at y 566-574 on=["2"], and 2017 `c2_01_0_[0]` on=["Yes"] upper / `[1]` on=["No"] lower, matching "Yes. Go to line 18." printed above "No." — so this is a latent instrument gap, not a live defect.)
EVIDENCE: `crates/btctax-forms/src/pdf.rs:432-435` — `FieldValue::Check { on } => { dict.set("V", …); dict.set("AS", …) }` with no check against `button_on_states`. The repo already solved exactly this for the 1040's Yes/No question with a map-independent geometric oracle: `crates/btctax-forms/src/verify.rs:456` `pub fn topmost_yes_no_pair(...)` — *"Derived from the blank PDF's widget geometry + appearance states, never the map."* Schedule D line 17 has no equivalent, though the map comment already cites the geometry ("Yes is the UPPER widget (y 578-586)") that would supply it.
```

```
SEVERITY: Minor
WHERE: crates/btctax-forms/src/schedule_d.rs:124
CLAIM: On the crypto slice, `schedule_d_line17` is fed lines 15/16 that structurally omit Schedule D lines 6, 13 and 14, so a **checked** line-17 box can be printed where the filer's real return leaves line 17 blank — converting an already-disclosed wrong amount into an affirmative routing checkbox.
FAILURE: Crypto-only 2025 year (no `income import`), filer carries a $20,000 long-term capital-loss carryover (`tax-profile --capital-loss-carryforward-long 20000`, which `compute_tax_year` does use), and has $5,000 of long-term BTC gain and no short-term activity. The slice writes line 10/15 = 5,000, line 16 = 5,000, and now line 17 = **Yes**. The filer's actual Schedule D has line 14 = (20,000) ⇒ line 15 = −15,000 ⇒ line 16 = −15,000, a loss — and the form says *"If line 16 is a loss, skip lines 17 through 20 below."* So the exported page carries a checked Yes on a line their return must leave blank, and routes them to 18→20→the QDCGT worksheet instead of line 21's §1211 limit. Minor because the slice's line 15/16 amounts and the 1040 line 7a were already wrong for this filer and the man page discloses it ("it has no line 13 … and no lines 6/14 (capital-loss carryovers)"); this adds no new class of error, only a new cell that inherits it.
EVIDENCE: `crates/btctax-forms/forms/2025/schedule_d.map.toml` carries only `line3`, `line7_h`, `line10`, `line15_h`, `line16_h`, `qof_*` and the new `[line17]` — no `line6`, `line13`, `line14` (those keys exist only in `forms/2024/schedule_d.map.toml`, under its "FULL-RETURN extension (P6)" block). `design/forms/extract/f1040sd--2024.txt:82` — *"If line 16 is a loss, skip lines 17 through 20 below."*
```

```
SEVERITY: Minor
WHERE: crates/btctax-cli/LIMITATIONS.md:249 (behaviour at crates/btctax-cli/src/cmd/admin.rs:731)
CLAIM: The fourth leg of the class-(B) justification — "skipping it prints a genuine blank plus a second advisory" — holds only on `btctax report --tax-year`. `advisories_for` has exactly one production call site, and the export command that actually writes `schedule_b.pdf` renders no advisories.
FAILURE: Full-return year, `foreign_accounts = Some(true)`, `foreign_country_names` set, `fbar_filing_required = None`. Before: `btctax export-irs-pdf` hit `screen_inputs` and refused with `FbarFilingRequirementUnanswered`, writing no bytes. Now: `export_full_return` passes all three screens and writes `schedule_b.pdf` with 7a checked Yes and the FBAR pair blank, printing **no** FBAR notice on that invocation. The filer sees `Advisory::FbarSubQuestionNotAnswered` only if they separately run `btctax report --tax-year N`. Minor, not Important: the three load-bearing legs of the reversal are all verified true (no figure reads the box; blank is no testimony; the Caution's penalty attaches to not filing FinCEN 114), so the non-refusal itself is justified — only the disclosure's reach is narrower than the docs say, and the pre-existing `Advisory::FbarFinCen` has the same reach.
EVIDENCE: `crates/btctax-core/src/tax/advisories.rs:241 pub fn advisories_for(...)` has one non-test caller, `crates/btctax-cli/src/cmd/tax.rs:364`, inside `report_tax_year`'s dual-report block. `crates/btctax-cli/src/cmd/admin.rs:768-780` runs `screen_inputs` / `screen_compute_dependent` / `screen_absolute` and never calls `advisories_for` or `render_advisories`.
```

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/printed.rs:919-921 (and crates/btctax-forms/tests/full_return_forms.rs:915)
CLAIM: The `ScheduleBLines` doc comment still states a safety guarantee this diff deleted — *"It is still never GUESSED: unanswered refuses"* — and names `QuestionId::FbarFilingRequired`, a variant removed in this branch.
FAILURE: This is the module doc a future author reads when deciding whether `fbar_filing_required` may be collapsed to `bool`/`unwrap_or(false)`. The two tests that hold the `None` ⇒ blank behaviour (`printed.rs::an_unanswered_part_iii_question_stays_none_and_is_never_defaulted_to_no`, `full_return_forms.rs::schedule_b_part3_transcribes_the_filers_own_answers_including_the_fbar_subquestion`) both cite that refusal as the reason the bug was "latent" — a reason that is now false, and the box is now genuinely reachable as `None` on a filed Schedule B. The backticked `QuestionId::FbarFilingRequired` is not an intra-doc link, so rustdoc will not flag the dangling name.
EVIDENCE: `crates/btctax-core/src/tax/printed.rs:921` — *"unanswered refuses, and when 7a is \"No\" the pair is left unwritten because the form does not ask it."* `crates/btctax-core/src/tax/return_refuse.rs` no longer contains `FbarFilingRequirementUnanswered`; `crates/btctax-core/src/tax/questions.rs:585` now defines `SkippableId::FbarFilingRequired`.
```

```
SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/types.rs:104-112
CLAIM: `ltcg_all_in()` is documented as "the all-in marginal rate on the next dollar of long-term capital gain", but `ltcg` is a *last-dollar* rate, so exactly at a §1(h) breakpoint the accessor is one bracket low.
FAILURE: Single, `top == bp.max_fifteen` exactly ($533,400 for TY2025): `ltcg` reports 0.15 (`top <= bp.max_fifteen`), so `ltcg_all_in()` reports 0.188 while the next dollar is taxed at 0.20 + 0.038 = 0.238. Same at `top == bp.max_zero`, and at `magi_with == thr` exactly the next dollar does attract §1411 while `niit_at_margin` is false. The `niit_at_margin` field doc discloses the boundary convention; `ltcg_all_in`'s does not. Measure-zero inputs, display-only.
EVIDENCE: `crates/btctax-core/src/tax/compute.rs:394-403` — `ltcg: if top <= bp.max_zero { ZERO } else if top <= bp.max_fifteen { dec_15() } else { dec_20() }`.
```

WHAT I VERIFIED AND FOUND SOUND:

- **(a) is genuinely display-only.** Grepped every reference to `marginal_rates` / `ltcg_all_in` / `niit_at_margin` across `crates/`: the only readers are `btctax-cli/src/render.rs:1350-1362`, `btctax-tui/src/tabs/tax.rs:122-135`, and tests. `optimize.rs` carries `marginal_rates` on `OptimizeProposal` (line 92, set at line 949) but no arithmetic reads it; `whatif.rs` computes its own `niit_applies` from the NIIT delta (`whatif.rs:361, 897`), not from `MarginalRates`. No tax figure moved.
- **`niit_at_margin`'s right-derivative is correct within the model.** Adding $1 of LT gain raises both `NII` and `MAGI − thr` by exactly $1, so `min(·)` rises by exactly $1, and `max(0, ·)` has derivative 1 iff the old `min` was ≥ 0 — which is `magi > thr && nii >= 0`. `thr` is filing-status-aware (`niit_threshold`: MFJ/QSS 250,000, Single/HoH 200,000, MFS 125,000). The cross-netting-absorption case (a net-loss year where an added LT dollar is swallowed by the loss) makes the flag *over*-state — the safe direction, and already true of bare `ltcg`.
- **`schedule_d_line17` is faithful to the form on all three revisions.** `pdftotext` of the bundled 2017 PDF and `design/forms/extract/f1040sd--2024.txt:78-89` both read *"If line 16 is a gain … go to line 17"* / *"If line 16 is a loss, skip lines 17 through 20"* / *"If line 16 is zero, skip lines 17 through 21"* and *"17 Are lines 15 and 16 both gains?"* — exactly `(line16 > 0).then_some(line15 > 0)`. Zero is correctly not a gain.
- **The `ScheduleDRouting` refactor is bit-equivalent.** `Some(true)`→`BothGains`, `Some(false)`→`ShortGainLongLoss`, `None && line16<0`→`NetLoss`, `None`→`Zero` reproduces the old four-branch chain exactly.
- **The slice's line-17 inputs match the printed cells.** `fmt_money` is `d.to_string()` (no rounding), line 15 is `fmt_money(totals.lt.gain)`, line 16 is `fmt_money(st.gain + lt.gain)`, and `active()` guarantees `!lt_active ⇒ lt.gain == 0`, so the LT-inactive case (blank line 15, line 16 a gain) correctly prints **No**.
- **The 2017 and 2025 `[line17]` maps are correct against the actual PDFs.** `xtask dump-fields` confirms field names, on-states and the Yes-above-No geometry on both; the 2024 map already carried `[line17]` for the full return, which is why only two files changed.
- **(d): "no figure on the return reads it" is TRUE.** Repo-wide grep of `fbar_filing_required` finds only: the field definition, the classifier's `c.exempt`, the registry get/set, the advisory trigger, a `_` binding in `first_negative_amount`, `ScheduleBLines`, and `schedule_b.rs:154`'s checkbox write. `schedule_b_files` (`return_1040.rs:1657`) reads `foreign_accounts`/`foreign_trust` only. No compute path, no screen, no gate.
- **The printing side needed no change and holds.** `schedule_b.rs:152-158` `filter_map(|(pair, answer)| answer.map(...))` writes nothing for `None`; two mutation-verified tests pin it.
- **No dangling refusal behaviour.** `RefuseReason::FbarFilingRequirementUnanswered` and `FieldId::DeclFbarFilingRequired` are gone from every match arm and map; `SKIPPABLE_QUESTIONS` index 7 really is `FbarFilingRequired` (matching `skippable_tristate!(7, …)`); `FieldId` serialises by variant name and is not persisted to disk. The classifier's no-`..` destructure plus `#![deny(unused_variables)]` still forces the leaf to be classified.
- **(c) the watermark.** Applied only in `admin.rs:606-610` to `form_1040_capgains.pdf`; the KAT asserts the full-return `00_f1040.pdf` is NOT stamped and that `f8949.pdf`/`schedule_d.pdf` are not either. Both text matrices keep their string inside the 612×792 media box (worksheet: (60,700)→≈(480,280); draft: (90,250)→≈(463,623)), and the content stream is uncompressed so the byte-substring assertions are meaningful.

WHAT WOULD MAKE THIS REVIEW WRONG: it assumes `TaxProfile.magi_excluding_crypto` is the filer's complete MAGI while `nii_with` is not their complete NII — if the product actually intends `TaxProfile` to describe a filer whose only investment income is qualified dividends plus capital gains, finding 1 collapses to a documentation gap.

---

## Disposition (author, same day)

The reviewer's own "what would make this wrong" was checked FIRST, because finding 1 stands or falls
on it. `TaxProfile.magi_excluding_crypto` is documented *"Modified AGI excluding crypto, for the
§1411 NIIT threshold comparison"* — the filer's **complete** non-crypto MAGI — while the NII terms
are QD + capital gains + crypto interest only. **The premise holds**, so finding 1 is real.

| finding | disposition |
|---|---|
| 1 — partial NII ⇒ under-reserve | **FILED, `FOLLOWUPS.md` §G-19a.** Not fixed: the narrow case is genuinely *unknown*, and both repairs (fail-safe vs. a third display state) are product judgments that also cost a second `frozen_guard` pin exception. Owner's call. |
| 2 — no map-independent oracle for line 17 | **FILED, §G-19b**, with the cheaper class-wide fix named (reject an `on` value absent from the widget's own `button_on_states`). |
| 3 — line 17 inherits the missing 6/13/14 | **FILED, §G-19c.** Scope boundary, not a defect: fixing it properly means the slice collecting the carryover, i.e. becoming the full return. |
| 4 — advisory reach | **FILED, §G-19d.** Whole-advisory-surface question; `FbarFinCen` has the same reach and always did. |
| 5 — stale `ScheduleBLines` doc | **FIXED INLINE.** It was a false safety claim in the exact doc a future author reads before collapsing the `Option`. Rewritten to say the opposite and louder: `None` is now REACHABLE on a filed Schedule B, and the `Option` — not a screen — is the only thing keeping the box blank. The `# Panics` block's stale *"`unwrap_or` defaults defensively to `false`"* was corrected in the same pass. |
| 6 (Nit) — `ltcg_all_in` at a §1(h) breakpoint | **FILED with §G-19a**, which it rides along with (same file, same pin exception). |
