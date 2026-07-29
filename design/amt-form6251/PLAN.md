# Form 6251 (AMT) — implementation plan

**Status:** NARROW-RE-CHECK FOLDED. **0 Critical across all four rounds' final state.** The re-check
confirmed §2 and T3's `smoke.rs` bullet FIXED and found four propagation defects (edits made in §2/§6/§11
that were never carried into T2/T7/T3) plus five Minors — all folded here. **Owner's call whether this is
build-ready or wants one more pass**; the residue is bookkeeping, not decisions.

**Goal:** stop refusing returns over the AMT screen, by **implementing Form 6251**.

**Base:** `main` after `fix/amt-screen-line2`. **Lineage:** `FOLLOWUPS.md` §G-4.
**Reviews:** `design/amt-form6251/reviews/` — r1 (Fable, primary-source tax) 5C/12I; r2 (Opus, fold +
mechanism) 2C/8I; r3 (Sonnet checklist + Opus fresh-eyes) 0C/4I; narrow re-check (Sonnet + Opus,
four sections only) 0C/4I — all propagation, no new decisions — the restructure closed all seven
prior Criticals; the four Importants were defects the fold itself introduced. All folded here.

---

## 0. Governing principle — the plan's own shape changed because of it

**Transcribe the form. Do not model it.** (`CLAUDE.md`, "Transcribe IRS forms"; `FOLLOWUPS.md` §G-5.)

Form 6251 is two pages. It never asks anyone to *derive* anything — it says "enter the amount from",
"enter the smaller of", "if X, skip to Y". An ordinary filer completes it in twenty minutes.

Every defect in this plan's two review rounds was a line that was never typed in:

| defect | what it was |
|---|---|
| the shipped v0.9.0–v0.13.0 bug | screening worksheet reduced to `AGI − QBI`; Sch A line **7** conflated with line **17** |
| r1 C-3 | line 2b dropped by writing AMTI as a formula |
| r1 C-4 | MFS kicker dropped — it **is** line 4's instruction text |
| r1 C-1 | two rounds on a Part III question **line 20 answers in one sentence** — and the half that was actually wrong is lines 16/17/22's "smaller of" plus the line-32 skip (r3 Minor: line 20 settles only *positioning*) |
| r1 I-1 | "attach when AMT > 0" instead of Who Must File condition 1 |
| r2 C-A | an oracle gate built on `c09600` (the *AMT*) to validate a *TMT* |

So this plan no longer contains a computation model. **It contains an instruction to transcribe, and the
tests that prove the transcription is faithful.** Note this document deliberately does *not* quote the
line texts itself — repeating them here from memory would be the same paraphrasing error one level up.
T1 transcribes them from the PDF.

---

## 1. Scope

**Tier 1** — compute the form; proceed when no attachment is required.
**Tier 2** — fill and attach it when one is.

The boundary is **Who Must File condition 1** (line 7 > line 10), *not* `AMT > 0`: there is a window
where AMT is $0 and the form must still be attached (r1 I-1). Only condition 1 is reachable in v1.

**Why Tier 2 is mandatory** (r1 §1, unchanged and verified): AMT is genuinely owed whenever the exemption
is fully phased out (AMTI ≥ $1,751,900 MFJ) **and** ordinary taxable income is below the crossover —
≈$769,139 at a zero add-back, ≈$800,250 SALT-capped, ≈$859,983 standard-deduction. A salaried filer
selling a large Bitcoin position is inside that region: $250,000 wages + $2M gain ⇒ **$26,271.00** of AMT.
Exposure peaks at $24,619 (zero add-back) rising to **$32,795** (standard deduction).

**Out of scope:** Form 8801 (§5 argues no new obligation arises). Every Part I adjustment other than
those below is either refused upstream (§57(a)(5) PAB interest) or has no input (ISO, §1202, §4952,
depreciation, NOL/ATNOL, K-1, §56(a)(6), depletion, IDC, long-term contracts, pre-1987 installments).

---

## 2. The artifact

```rust
/// Form 6251 (2024). One field per numbered line, in the form's numbering, each doc comment the
/// official instruction text VERBATIM from i6251. Nothing here is derived; every line is transcribed.
#[non_exhaustive]
pub struct Form6251 { /* line1 … line11, line12 … line40 */ }
```

Naming follows the **form's** labels, not ours: line 7 is "Tentative minimum tax" only after the FTC
subtraction at line 9 — so `line7_*` and `line9_tentative_minimum_tax`, never a bare `tmt` (r2 Nit).

Three consequences, and they are the reason for the restructure:
- **Lines cannot be dropped.** 2b and the MFS kicker are fields, not terms in a formula I might forget.
- **Part III cannot be misread.** Lines 12–40 get typed in; line 20 states its own source.
- **Tier 2 is nearly free.** If the struct *is* the form, the emitter is a field→AcroForm mapping with
  no logic in it. "Compute it" and "file it" stop being separate hard problems.

**★ Rounding — which LAYER `Form6251` is (r3 I-1).** An earlier draft said "whole dollars per line,"
which contradicted §8's cent-precise vectors and §4's exact `AbsoluteReturn` chain, and would have moved
§1's own headline exemplar by a dollar (`round_dollar` is half-up, `conventions.rs:37`; V5's L31 =
54,442.50 rounds to make AMT $26,270 **or** $26,271). The repo already settled this and the plan simply
failed to cite it — `printed.rs:1-14` + `design/full-return/ROUNDING_AUTHORITY.md` Reading A:

- **`Form6251` carries EXACT cents** and lives in the absolute chain beside `AbsoluteReturn`. §8's
  figures are that layer; T2's KATs pin them to the cent.
- **The PRINTED form rounds per line** and cross-foots over already-rounded lines, per `printed.rs`'s
  existing discipline. T7's byte-reproducible V5 golden pins it. The "two orders differ by $1" KAT
  belongs to **that** layer.
- **The Who-Must-File line-7-vs-line-10 comparison uses the EXACT values**, so an attachment can never
  flip on a rounding tie. This also fixes V9, whose acceptance window is ≤$600 wide.

`preferential_tax` (`compute.rs:57`) rounds once to cents, so it is reused only for the band split
(`at_0/at_15/at_20`).

---

## 3. What the form asks that we cannot answer

Following instructions means collecting what the instructions require. Two lines ask questions our input
surface does not carry — this is **not** scope creep:

| Line | Question | Input today |
|---|---|---|
| **2k** | *"...including any carryover that is different for the AMT"* | `capital_loss_carryforward_in` carries no AMT twin |
| **3** | is the dwelling a principal/qualified residence? | `mortgage_interest_1098` carries no dwelling question |

**★ DECISION (r2 C-B) — the full ternary, for both:**

| Answer | Behaviour |
|---|---|
| unanswered | **refuse** |
| adverse ("my AMT carryover differs" / "not a qualified dwelling") | **refuse** — v1 models neither |
| neutral | the line is 0 |

r2 caught that the earlier draft specified only the unanswered branch, so an adverse answer would have
computed with no add-back and **understated** tax. It also caught that the exemplar I told the builder to
mirror does the opposite: `return_refuse.rs:1004-1005` asserts `reason == None` for `Some(false)`,
commented *"No brick: the screen does not refuse a truthfully-answered mixed-use return."* **We mirror it
only on the unanswered half and deliberately diverge on the adverse half** — a zeroed line 8a is
conservative; a missing AMT add-back is not.

Liveness predicates: `schedule_a.mortgage_interest_1098 > 0`; `capital_loss_carryforward_in.short > 0 ∨
.long > 0`.

**★ Mechanism (r3 Minor) — the two branches refuse by different routes:**
- *unanswered* ⇒ the **registry loop** (`return_refuse.rs:543`), like every other declaration.
- *adverse* ⇒ a **value-refusal**, following the `ForeignTrust` pattern (`return_refuse.rs:552-568`) —
  **not** the registry loop, which only sees `None`.
Each therefore needs its own `RefuseReason` variant **and** its own `attribute()` arm (the match is
exhaustive; see §11). State each question's **polarity** explicitly, because `testonly.rs` prescribes
`true` for both: phrase them so `true` is the AMT-**neutral** answer ("is your AMT capital-loss
carryover the same as your regular one?", "is this a principal or qualified second residence?").

---

## 3.5 — ★ Lines 7–11 and the attach test (RESTORED 2026-07-28)

**This section existed before the transcription restructure and the restructure silently deleted it,
while leaving eight references to "line 7 > line 10" elsewhere in the plan.** Three review rounds missed
it — including one lens explicitly tasked with finding what the rewrite lost. It is Critical by this
plan's own rule: a builder is told to compare line 7 against line 10 and never told what line 10 is, and
the obvious guess (1040 **L24**, "total tax") overstates AMT by NIIT + Additional Medicare — **$25,750 on
V1 alone** (r2 I-1). Verbatim text is T1's job; this fixes the structure and the scope calls.

- **Line 7** — two branches, and the routing matters: complete **Part III** (and enter its line 40 here)
  when there are capital-gain distributions on 1040 L7, qualified dividends on 1040 L3a, **or** a gain on
  **both** Schedule D lines 15 and 16 *as refigured for the AMT*. Every btctax filer with a net LTCG takes
  that branch. **All others** take the flat 26%/28% on line 6 with the $4,652 ($2,326 MFS) subtrahend —
  transcribe both; do not assume Part III always runs.
- **Line 8** — the AMT foreign tax credit. For the ≤$300/$600 §904(j) elector this equals Schedule 3
  line 1, which is why net AMT is FTC-invariant — but lines 8/9/10 each print differently, so it must be
  computed, not cancelled (r2 I-1).
- **Line 9** — tentative minimum tax = line 7 − line 8. **This, not line 7, is the form's "Tentative
  minimum tax"** — hence §2's naming rule.
- **Line 10** — ★ the definition that vanished. It is **1040 L16 minus any Form 4972 tax, plus Schedule 2
  L1z, minus Schedule 3 L1, minus any negative Form 8978 line-14 amount (as a positive)**, floored at 0,
  and with L16 **refigured without Schedule J** if Schedule J was used. **Not 1040 L24.**
  Scope: Form 4972 (lump-sum distributions), Form 8978 (partner audit adjustments) and Schedule J
  (farm/fishing averaging) are inputs **v1 has no surface for**, so each term is structurally 0 — but T1
  transcribes all four and records that classification under §6's guard, rather than letting this plan's
  shorthand become the implementation.
- **Line 11** — AMT = line 9 − line 10, floored at 0 ("if zero or less, enter -0-"), → **Schedule 2
  line 2**. The `max(0, …)` is the form's own instruction, now cited rather than invented.

**The attach test is Who Must File condition 1 — line 7 > line 10** (i6251, r1-verified), *not* line 9 vs
line 10 and *not* `AMT > 0`. Note it compares **line 7**, before the FTC subtraction, which is precisely
why the window exists where AMT is $0 and the form must still be attached, and why V9 needs its own
line 7 / line 10 columns (§8). Per §2, that comparison uses the **exact** values.

*How this was found: by opening the PDF to start T1. The rule catches its own author — §0 forbids the
plan from quoting line texts from memory, and this section is what happens when a restructure paraphrases
away a section instead of transcribing it.*

---

## 4. Files

**Tier 1 — the form**
`btctax-core/src/tax/form6251.rs` (new; §2) · `tax/amt.rs` (screen kept as a fast path; **gains the MFS
kicker**) · `tax/tables.rs` (`AmtParams` += MFS kicker start/max; += the **25% §55(d)(3) phase-out rate**
and the 26%/28% §55(b)(1) rates — ★ final pass: `amt.rs:111` is the **0.25**, `:123` the 0.26, and 28%
arrives only with `form6251.rs`; the earlier text mis-cited both as 26/28 and would have left
`dec!(0.25)` outside `AmtParams` against §6) · **★ `btctax-adapters/src/tax_tables.rs:141` — THE SOLE
PRODUCTION `AmtParams` LITERAL**, where the real TY2024 MFS-kicker dollars ($875,950 / $1,142,550) are
typed under its `// §55(d) AMT amounts (Rev. Proc. 2023-34 §2.11)` convention. Adding fields **E0063s all
nine literals** (this one plus `btctax-core`'s `testonly.rs:74`, `advisories.rs:430`, `qbi.rs:326`,
`amt.rs:134`, `return_refuse.rs:835`, `return_1040.rs:1575`, `tables.rs`×2) — eight are fixtures where
any value compiles green; **only `tax_tables.rs` reaches a filed return** · `tax/return_1040.rs`
(**`assemble_absolute` computes and stores `ar.amt`; `screen_absolute` only reads it** — it takes `ar`
immutably, r2 Minor).

**Tier 1 — the two declarations (r2 I-1; this is our architecture's cost, not the form's)**
`tax/return_inputs.rs` (the `Option<bool>` fields + the shape test at `:490-517`) · `tax/questions.rs`
(`FormQuestion`, `QuestionId`, `QuestionId::ALL`) · **`tax/classifier.rs`** (must call
`c.declaration(field, QuestionId::…)`; it destructures `ScheduleAInputs` **by name** at `:307-320`, and a
field destructured but not registered reads as *answered* — this repo's one architectural fault line) ·
`tax/return_refuse.rs` (`:161` `AmtScreenTriggered` — **the variant is kept and its trigger narrowed**;
there is no `AmtOwed`, r2 Minor — plus the registry loop at `:543`) · `tax/advisories.rs` · `tax/packet.rs`
· `btctax-cli/src/cmd/answer.rs` · `btctax-input-form/src/{seam.rs, spec/sections.rs, spec/coverage.rs:460,
apply.rs}` · **`btctax-input-form/src/spec/mod.rs:110`** (hard-codes `decl_count == 7` — a guaranteed red; the
adjacent `decls.fields.len() == 8` assert at `:114-119` moves too) · **`spec/registries.rs`** (r3 Minor —
the actual site of both total maps, `field_to_question` `:235` and `question_to_field` `:251`, plus the
index-literal delegating fields). **Decide per new leaf:** its own `Decl*` field, or a Schedule-A dedup
like the mortgage one
· `tax/testonly.rs:33-39` (`answer_all_live_declarations` auto-answers `false` for every id but the
mortgage one, across `build_golden_return` and the whole baked corpus — answer **`true`** for both new
ids, matching the mortgage precedent) · `btctax-input-form/src/attribute.rs:144,:348` (exhaustively
anchors every `RefuseReason`) · the example fixture + docs.

**Tier 1 — oracle & goldens**
`btctax-oracle-harness/src/main.rs:704` (narrow to the **AMT reason only**; it is a combined check over
three refusal classes) · **`btctax-oracle-harness/tests/smoke.rs:98`** (`EXPECTED_REFUSED` pins a
screen-tripping zero-AMT household that T3 makes *proceed*; `:101-113` and `:153-156` both flip — r2 I-7)
· `scripts/oracle/corpus.py` (caps: W-2 $270k / LTCG $20k are far below the $1,218,700 phase-out start).

**Tier 2**
`btctax-forms/forms/2024/f6251.pdf` + `.map.toml` (new; 6251 is **tax-year**-versioned) ·
`btctax-forms/src/form6251.rs` (new; the field→AcroForm mapping) · `tax/printed.rs`
(`Schedule2Lines.line2/line3`) · **`tax/return_1040.rs` again** — AMT must thread into
`AbsoluteReturn.total_tax`/`amount_owed`, not only the printed packet, or the PDF shows the AMT while the
balance due omits it (r2 I-8) · `btctax-cli/{cmd/admin.rs,cli.rs}` · `scripts/oracle/gen_goldens.py`
(see T5) · docs, man, `btctax limitations`.

---

## 5. Why no NEW Form 8801 obligation — r1 CONFIRMED IN FULL

§53(d)(1)(B)(ii)(I) names **all** of §56(b)(1) — taxes and standard deduction — as **exclusion** items,
excluded from the minimum tax credit. Form 8801 Part I line 15 = the whole AMT; lines 18/21 = $0; per
i8801 *Who Should File* the filer is not directed to complete it. The §904(j) FTC cancels symmetrically.
Every deferral item is out of scope (§1).

**Discharge (r2 I-5 — these were unowned, and the original was unbuildable):** T2 carries a KAT
asserting, for every vector, that the set of applied **§56(b)(1) adjustments** is exactly
{line 2a, line 2b} and each is an exclusion item. ★ r3 Minor: the MFS line-4 kicker is asserted
**separately** — it is the §55(d)(3) exemption phase-out, not a §56(b)(1) adjustment, and the property
the no-8801 argument needs from it is that it introduces **no deferral item**. **Not** an "8801 recompute" — there is no 8801 code and §1 excludes
building it.

---

## 6. Global constraints

- **Gate:** `make check` **and** `cargo fmt --all -- --check`, from the first commit.
- **Transcription is the primary gate.** Every numbered line present; every doc comment matching i6251.
- **Fail-closed at every commit.** Nothing previously refused computes unless line 7 ≤ line 10 is proven.
- **Never understate.** The MFS kicker and §3's two adverse branches are the understatement risks.
- **Every guarantee ships with a test that reds when the guarantee is removed.**
- **No literal AMT dollar amount or rate outside `AmtParams`** (`#[cfg(test)]` exempt for boundary KATs).
- **★ Exhaustiveness is guarded in two shapes, keyed to the FORM'S numbering (r3 I-3).** State it as:
  *every Part I line 2c–2t and line 3 either refuses upstream or has no `ReturnInputs` leaf* —
  enumerated from `PART_III.md`, not from a prose list. (The earlier draft said "the eleven uncapturable
  items"; §1's prose omits 2m, 2n, 2o, 2q and 2r, and under §0's own thesis an exhaustiveness guard is
  the last place a prose list should stand in for numbered lines.)
  - **(i) behavioural** — §57(a)(5) PAB interest still refuses. A normal KAT.
  - **(ii) input-surface** — the remaining lines still have **no** `ReturnInputs` leaf. ★ The earlier
    draft cited `spec/coverage.rs`'s mutate-and-diff mechanism, which **structurally cannot do this**:
    it serializes a maximally-populated `ReturnInputs` and walks every **existing** leaf (`leaf_map`,
    `:67-75`), so a concept never given a field never appears and there is nothing to observe. The
    EXEMPT-list workaround is foreclosed too — `:262-279` panics on a "stale exemption" that matches no
    real leaf. **Mechanism instead:** a KAT reusing `leaf_map` on the maximal fixture asserting no key
    matches a literal blocklist of those lines' field-name patterns.
  - **★ SCOPE (narrow re-check) — (ii) EXCLUDES 2g and the two §3 lines.** As first written it said
    "every Part I line 2c–2t and line 3", which **contradicts §3**: 2k and line 3 are exactly the lines
    §3 gives leaves to (`capital_loss_carryforward_in`; `mortgage_interest_1098`, which
    `maximal_fixture` primes at `dec!(1)`, `coverage.rs:110`). Correct scope: *every Part I line 2c–2t
    and line 3 **except 2g** (guard (i)) **and except 2k and 3** (§3's declarations)*. For those two the
    guarantee is §3's ternary plus T3's per-declaration refuse KATs — if a leaf assertion is wanted
    there, blocklist only an AMT-twin name pattern (`*_amt*`), never the regular-side field.
  - **★ LOCATION (narrow re-check) — this KAT lives in `coverage.rs` and is owned by T3, not T2.**
    `leaf_map` is a **private** fn (`coverage.rs:68`) inside `#[cfg(test)] mod coverage;`
    (`spec/mod.rs:8`) — invisible outside that file and non-existent when the crate builds as a
    dependency. Put the KAT beside `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt`
    (`:154`), where `leaf_map` and `maximal_fixture` are in scope. T3 already edits `coverage.rs` (§4).
  - **Residual, worth one sentence in the KAT's doc:** leaves nested inside an absent `Option` or an
    empty Vec never appear in `leaf_map`'s output, so the blocklist only bites on leaves the maximal
    fixture realizes.

---

## 7. Tier 1 tasks

### T1 — transcribe the form (BLOCKING)

- [ ] ★ final pass — **sources and the failure branch.**
      `https://www.irs.gov/pub/irs-prior/f6251--2024.pdf` and `i6251--2024.pdf` (the repo's established
      `irs-prior` convention). Stash the blank at `crates/btctax-forms/forms/2024/f6251.pdf` for T6.
      **If either cannot be fetched, STOP and escalate — do not transcribe from memory.** §0 forbids
      exactly that, and nothing else in this plan told the builder to stop rather than paraphrase.
- [ ] From `f6251--2024.pdf` + `i6251--2024.pdf`, write `PART_III.md`: **every** line 1–40, its verbatim
      instruction text, and for Part III whether each input is AMT-side or regular-side. Quote lines
      20/27 *"(as figured for the regular tax)"* against line 13 *"(as refigured for the AMT)"*.
- [ ] **★ DECISION (r2 C-A) — the independence mechanism.** Hand-derive V1–V10 line by line and commit
      them as a **JSON fixture** in the same commit as `PART_III.md`. "Recorded before code" is then
      enforced by **git history**, not by a red suite — which also fixes "watch them fail" being
      uncompilable against an API T2 has not written (r2 Minor). `c09600` is **not** the source: it is
      Tax-Calculator's *AMT*, which is $0 on V2/V2b under **both** Part III readings, so it carries zero
      information on exactly the discriminating vectors. Use it as a **one-way cross-check only where
      AMT > 0** (V4/V5/V6), reconciling `TMT = c09600 + regular tax`; state it is inert for V1/V2/V2b/V3.
      Reaching it at all needs `_taxcalc_row` (`gen_goldens.py:104-129`) extended with cash charitable
      (`e19800`), `MARS = 3`, state refund and FTC — **that plumbing is part of T1**, not T5.
- [ ] **The form is the authority.** A taxcalc disagreement is adjudicated against the PDF, never encoded.
- [ ] ★ r3 Minor — **bind the fixture to the tests**: name its path, its schema, and the crate that
      deserializes it, and state that **T2's vector KATs read it** rather than retyping the figures
      (otherwise a builder satisfies every bullet with hand-copied numbers). The `PART_III.md` + fixture
      commit lands **before** the `_taxcalc_row` plumbing commit, or the git-history independence
      argument is weakened by this plan's own ordering.
- [ ] **Construct V7–V10** — inputs, not just purpose (r2 I-3). V9 needs a search: line 7 strictly inside
      `(1040 L16 − FTC, 1040 L16]` with the FTC at its §904(j) ceiling ($300, ×2 MFJ).

### T2 — `form6251.rs`
- [ ] Transcribe lines 1–11 and 12–40 per `PART_III.md`. Line 1 has a **negative branch** (`L15 = 0 ⇒
      1040 L11 − L14`); the `"if zero or less, enter -0-"` text belongs to **line 6**, not line 1
      (r2 Minor) — plus the one-line unreachability proof (`taxable_income` is floored at 0).
- [ ] MFS kicker in **both** `form6251.rs` and `amt.rs` — in the screen it goes on `line5`, after the
      state-refund subtraction and before the exemption test, so it also feeds the phase-out (r2 Minor).
- [ ] KATs: every §8 vector; phase-out boundaries; the 26/28 breakpoint; MFS kicker boundaries
      ($875,950 / $1,142,550); line 6 ≤ 0; line 1's negative branch; the §5 adjustment-set KAT; and
      **§6 guard (i) only** — the §57(a)(5) PAB-refusal KAT.
      ★ narrow re-check: **guard (ii) moves to T3** (it must live in `coverage.rs`, see §6) and the
      **rounding-order KAT moves to T7** (§2 re-scoped it to the printed layer, and nothing prints a
      6251 in Tier 1).
- [ ] ★ final pass — **extend the bundled-figure KAT at `tax_tables.rs:754-761`** with both MFS-kicker
      boundaries. The `amt.rs`/`form6251.rs` KATs pin a **test fixture**; this one pins the number that
      actually reaches a filed return. Without it the two new statutory figures ship **unpinned**.
- [ ] **Mutations:** delete the line-2b subtraction ⇒ the V7 KAT reds. Delete the MFS kicker ⇒ the V8 KAT
      reds.

### T3 — wire it
- [ ] Keep `AmtScreenTriggered`; narrow its trigger to **line 7 > line 10**.
- [ ] **Apply that test on BOTH arms** (r2 I-2). The screen's clear path never applies it today, and the
      attach window is reachable there: `amt_should_file_6251` compares against `regular_tax_l16` with no
      Sch-3-L1 subtraction. Either compute lines 7–10 whenever Schedule 3 line 1 > 0, or prove "clear ∧
      FTC = 0 ⇒ line 7 ≤ line 10" and route every FTC-bearing return through the computation.
- [ ] The §3 declarations, full ternary, with a KAT per declaration pinning the **`Some(adverse)` ⇒
      refuse** branch.
- [ ] Register each in `classifier.rs`; **mutation: drop the registration ⇒ the unanswered-⇒-refuse KAT
      must red.** Update `spec/mod.rs`'s `decl_count`, `coverage.rs`, `testonly.rs`, `attribute.rs`.
- [ ] ★ final pass — **§6 guard (ii), its own bullet** (caught "gestured at" by r3 I-3 in T2, and again
      here after the fold moved the ownership language to T3 but left the actionable bullet behind): in
      `btctax-input-form/src/spec/coverage.rs`, beside
      `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` (`:154`), a KAT asserting no
      `leaf_map(&maximal_fixture())` key matches the blocklist of Part I lines 2c–2t and 3 — **excluding
      2g** (guard (i), T2) **and 2k and 3** (§3's declarations). Absent-`Option`/empty-`Vec` caveat in its
      doc comment. **Not** the `coverage.rs` edit in the registration bullet above; both touch the file
      for different reasons.
- [ ] **★ `smoke.rs` (r3 I-4 — the earlier "retarget" instruction is impossible).** No retarget target
      can exist *by construction*: `gen_goldens.py:259`'s `if amt or credits` rejects any household with
      `c09600 != 0` and its comment says the substance check "applies to EVERYONE (anchors included)";
      with FTC = 0, line 7 ≤ line 10 ⟺ AMT = 0, and **no corpus household can carry an FTC** (no
      `foreign`/`e07300` field in `gen_goldens.py` or `corpus.py`). So every bakeable household proceeds
      after T3. Nor can the constant merely be emptied: `smoke.rs:101` indexes `EXPECTED_REFUSED[0]` and
      would panic. **Do this instead:** set `EXPECTED_REFUSED = &[]`, and **invert**
      `the_amt_screen_anchor_is_reported_refused_in_default_mode` so it asserts the anchor now *proceeds*
      and reconciles on every compared line — that inversion is the end-to-end proof T3 un-refused the
      target population, and is worth strictly more than the test it replaces. `sweep_check_reconciliation`
      then asserts emptiness; `admitted >= 10` moves by one. r2's "do not delete" attaches to the **test
      function**, not the constant.
      ★ narrow re-check — **the inversion needs `--check` mode.** `all_reconciled` / `reproduction_ok`
      exist only there (`main.rs:466-473`); DEFAULT mode returns `{"refused", "lines"}` only
      (`:156-163`). So switch the call to `run(&["--check"], raw_household(name))` — the **whole**
      household, not `["inputs"]` — rename off `_in_default_mode`, and assert `refused == false &&
      all_reconciled && reproduction_ok`. Also **rewrite `EXPECTED_REFUSED`'s doc comment**
      (`smoke.rs:91-97`), whose stated rationale T3 falsifies: record that the constant is empty **by
      design** and that its emptiness is the proof T3 un-refused the screen-tripping zero-AMT population.
- [ ] **Mutations:** revert to the blanket refusal ⇒ the V1 KAT reds. Replace `line7 > line10` with
      `amt > 0` ⇒ the **V9** KAT reds.

### T4 — output
- [ ] A no-attachment return's printed 1040/Sch 2 against a **hand-built expected packet** (L17 = 0,
      Sch 2 L3 = 0, no 6251).
- [ ] Add a **screen-tripping, no-attachment** journey — every bundled journey is deliberately sized
      under the screen, so regeneration alone proves nothing (r2 I-10). ★ r3 Minor: the warnings are at
      those lines in **`btctax-cli/src/testonly.rs`**, not `tax/testonly.rs` (which is `ty2024_params()`'s
      std-deduction table). **Also sweep every doc comment whose stated rationale is "the AMT screen
      refuses this"** — both J6 warnings and `btctax-forms/tests/full_return_forms.rs:425-429`, the
      justification for `schedule_2_fills_part_ii_and_leaves_part_i_blank`.
- [ ] `report` prints AMTI / exemption / line 7 / line 10 / AMT, with a golden. (It lives in
      `cmd/tax.rs` + `render.rs`; there is no `cmd/report.rs` — r2 Minor.)

### T5 — oracle domain (r2 I-4: the earlier bullet was wrong on every claim)
- [ ] The Tier-1 win is **T3's un-refusal plus wider `corpus.py` caps**, admitting screen-tripping,
      zero-AMT households that already pass the existing gates.
- [ ] There are **three** gates, not one: `gen_goldens.py:259` `if amt or credits` (**combined** — keep
      the `credits` conjunct; relaxing the `c09600` half is **Tier 2** work), `:280-283` rejecting
      `1040.line17 != 0` (the binding one for AMT), and the harness `refused` gate at `:266/:277` (cleared
      for free by T3). Also decide the `:266-276` anchor-exemption branch: retarget or remove.
- [ ] Assert a **numeric floor** on newly-covered households.

**Tier 1 gate:** suite green, 0C/0I, every line of the form present with matching doc text, and a
no-attachment return exports a complete packet.

**★ The `c09600` clause is RETIRED (2026-07-28).** It read "V4/V5/V6 reconcile against `c09600`" — a
gate that can never pass, for a reason discovered by running it. `scripts/oracle/verify_f6251.py`
reports **9 of 11 vectors agreeing** with Tax-Calculator; the two that do not are **V4 and V5, the only
standard-deduction vectors that owe AMT**, and the discrepancy is a defect in *Tax-Calculator*:
`calcfunctions.py`'s `if standard > 0.0` branch subtracts the standard deduction and never applies Form
6251 line 2a's add-back. Filed upstream as PSLmodels/Tax-Calculator#3108. The gate is therefore **"0
unexpected divergences from `verify_f6251.py`"**, with V4/V5 adjudicated against the form — per
`CLAUDE.md`, an oracle is a witness and the IRS PDF is the authority.

**And the two-oracle standard cannot be met here at all** (`FOLLOWUPS.md` §G-6): OpenTaxSolver computes
no Form 6251 — its 1040 solver is *fed* Schedule 2 rather than deriving it — so there is no second
engine to ask. What stands in its place: the form's own text, i6251, §56(b)(1)(D), a second IRS
worksheet reaching the same base by a different route, and two blind hand-derivations that never saw
this code. That is a **Tier-2 blocker**, not a Tier-1 one — Tier 1 ships only zero-AMT returns.

---

## 8. Vectors — MFJ, TY2024. ✅ = independently recomputed and confirmed in r1.

**★ r3 Minor — V9 needs `line 7` and `line 10` columns of its own.** On V1–V6 they coincide with
line 9 and regular tax, but V9 is *defined* by FTC > 0, which is exactly where both identities break —
so V9's discriminating figure and T3's mutation operand currently have no cell. Add both columns when
T1 constructs V7–V10.

| # | Wages | LTCG | Gift | Ded | Taxable income | Regular tax | line 9 (TMT) | AMT | Why |
|---|---:|---:|---:|---|---:|---:|---:|---:|---|
| V1 ✅ | 1,000,000 | 500,000 | 85,000 | item | 1,415,000 | 364,675.50 | 327,965.00 | 0 | baseline |
| V2 ✅ | 1,000,000 | 500,000 | 750,000 | item | 750,000 | 129,397.50 | 113,654.50 | 0 | TMT discriminates the readings |
| V2b ✅ | 1,000,000 | 500,000 | 1,000,000→900,000 | item | 600,000 | 87,918.50 | 70,005.00 | 0 | **excess < gain**: L16 caps, L17 = 0, L32 skip |
| V3 ✅ | 1,000,000 | 10,000,000 | 0 | std | 10,970,800 | 2,285,321.50 | 2,275,348.00 | 0 | 0.44% margin — **insensitive** to Part III |
| V4 ✅ | 700,000 | 10,000,000 | 0 | std | 10,670,800 | 2,175,529.50 | 2,191,348.00 | 15,818.50 | AMT owed, no donation |
| V5 ✅ | 250,000 | 2,000,000 | 0 | std | 2,220,800 | 420,929.50 | 447,200.50 | 26,271.00 | the archetypal user |
| V6 ✅ | 1,000,000 | 10,000,000 | 250,000 | item | 10,750,000 | 2,203,625.50 | 2,205,348.00 | 1,722.50 | a donation *creates* AMT |
| V7 | — | — | — | — | — | — | — | — | **state refund > 0** — line 2b. Construct in T1 (no oracle needed) |
| V8 | — | — | — | — | — | — | — | — | **MFS** at $875,950 and $1,142,550. Construct in T1 |
| V9 | — | — | — | — | — | — | — | — | **FTC > 0**, line 7 inside the attach window, AMT $0. Construct in T1 |
| V10 | — | — | — | — | — | — | — | — | regular ordinary < $94,050 so the **0% band** engages. Construct in T1 |

V1's other figures: NIIT $19,000; Additional Medicare $6,750; **balance due $83,225.50 against payments
of $307,200 = $300,000 W-2 box 2 + $7,200 mandatory Additional-Medicare withholding** (r1 I-4).

---

## 9. Tier 2 tasks

- **T6** — bundle `f6251.pdf` (2024) + `f6251.map.toml`; every mapped field verified present.
- **T7** — the emitter: a **field→AcroForm mapping** from the T2 struct, including lines 8/9/10. Read-back
  via `verify_flat`; byte-reproducible golden for V5. ★ narrow re-check — **also the rounding-order
  KAT** (moved from T2): the printed 6251 cross-foots over **already-rounded** lines (Reading A), not
  `round_dollar(exact_total)`.
- **T8** — line 11 → Sch 2 L2 → L3 → 1040 L17 → L18 → L24 → amount owed. **KAT asserts BOTH the absolute
  chain (`AbsoluteReturn.amount_owed`) and the printed L24**, so a one-sided fix reds (r2 I-8).
- **T9** — attach iff line 7 > line 10, with a **skip KAT + mutation** (r2 I-10: "no 6251 in the packet"
  is vacuous before the emitter exists). Relax `gen_goldens.py:259`'s `c09600` half here, paired with the
  `:280-283` L17 gate; decide what the `golden_packet.rs` L24 cross-foot compares when L17 > 0, and what
  happens when taxcalc says `c09600 > 0` while btctax computes line 7 ≤ line 10. Regenerate docs; retire
  the `limitations` bullet.

---

## 10. Risks

| Risk | Early warning | Mitigation |
|---|---|---|
| a line silently absent | transcription gate | §6's primary gate: every line present, doc text matches |
| MFS kicker omitted ⇒ **understates** | V8 KAT absent | T2 pins both boundaries in `amt.rs` and `form6251.rs` **and — ★ final pass — in `tax_tables.rs:754-761`, the only one pinning what gets FILED**; the other two pin fixtures |
| an adverse declaration answer computes | V-adverse KATs absent | §3's ternary; T3 KAT per declaration |
| attach test loosened to `AMT > 0` | — | T3's V9 mutation |
| line 2b dropped | — | T2's V7 mutation |
| the oracle certifies a misreading | V2/V2b insensitive to `c09600` | T1 commits hand figures as data; `c09600` cross-checks only V4/V5/V6 |
| **the L16/L22 caps dropped** (r1's own "most likely wrong filed number": V2b 75,812.50 vs 70,005.00) | V2b KAT | ★ r3 Minor — **T2 mutation: remove the `min(excess, gain)` cap ⇒ the V2b KAT must red** |
| Tier 1 read as closing G-4 | Tier 2 slips | §1's trigger rule and $26,271 exemplar |

---

## 11. SemVer

**★ Tier 1 is MAJOR for `btctax-core`. All three types stay plain derives (r3 I-2).** An earlier draft
chose "mark all three `#[non_exhaustive]`, cheap because `no-users-yet`". It is neither cheap nor safe:

- **`Schedule2Lines`** is built by **struct literal in another crate** —
  `btctax-forms/tests/full_return_forms.rs:430`. `#[non_exhaustive]` makes that **E0639, a hard compile
  error**, against §6's green-from-first-commit rule.
- **`RefuseReason`** is matched **exhaustively from another crate** at
  `btctax-input-form/src/attribute.rs:22-26`, whose own doc states the guarantee: *"An EXHAUSTIVE
  `match` — no `_` arm — so a new `RefuseReason` fails to compile until it is placed."*
  `#[non_exhaustive]` forces a wildcard there, converting the compile-time totality that §4 cites as its
  reason for putting `attribute.rs` in the blast radius into a silent catch-all — leaving only the
  hand-written list at `:348`, which by construction cannot notice a new variant. **That trades a
  structural guarantee for a SemVer label, in the one repo whose named architectural fault line is
  "held by convention, not construction."**

`no-users-yet` makes the major bump free, which is exactly why it is the right side to spend. If a
`#[non_exhaustive]` is still wanted later, restrict it to **`AbsoluteReturn`** — every literal is
in-crate (`return_1040.rs:1287,3760`; `printed.rs:1270,2132,2358`).

**★ Tier 2 is MAJOR too (narrow re-check).** The label was not updated when this decision flipped: with
`Schedule2Lines` staying a plain struct, §4's Tier-2 addition of `line2`/`line3` **E0063s** the
cross-crate literal at `btctax-forms/tests/full_return_forms.rs:430`, which is breaking. Free under
`no-users-yet`.

**★ `Form6251` may carry `#[non_exhaustive]`** (§2's sketch) — *not* a contradiction with the above,
because §11's case rests entirely on **pre-existing downstream sites** and a brand-new type has none.
Consequence to honour: **T7's V5 golden must obtain its `Form6251` from core** (the computed vector or a
`testonly` builder), never a struct literal.

**Behaviour changes to disclose:** previously-refused returns now compute; and stored returns carrying a
1098 or a capital-loss carryforward now **refuse until the new declarations are answered — or, if
answered adversely, refuse permanently** (§3's ternary; r3 Minor: the earlier wording said only "until
answered", which is false for the adverse branch).
