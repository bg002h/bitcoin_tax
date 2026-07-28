# Form 6251 (AMT) — implementation plan

**Status:** r2-FOLDED and RESTRUCTURED. Awaiting r3 (scoped to the fold only).

**Goal:** stop refusing returns over the AMT screen, by **implementing Form 6251**.

**Base:** `main` after `fix/amt-screen-line2`. **Lineage:** `FOLLOWUPS.md` §G-4.
**Reviews:** `design/amt-form6251/reviews/` — r1 (Fable, primary-source tax) 5C/12I; r2 (Opus, fold +
mechanism) 2C/8I. Both folded here.

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
| r1 C-1 | two rounds on a Part III question **line 20 answers in one sentence** |
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

**Rounding:** whole dollars **per line**, because they are lines (SPEC §3.1). `preferential_tax`
(`compute.rs:57`) rounds once to cents, so it is reused **only for the band split** (`at_0/at_15/at_20`);
each band's tax is rounded at its own line. KAT on a vector where the two orders differ by $1 (r2 Minor).

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
mirror does the opposite: `return_refuse.rs:1006` asserts `reason == None` for `Some(false)`, commented
*"No brick: the screen does not refuse a truthfully-answered mixed-use return."* **We mirror it only on
the unanswered half and deliberately diverge on the adverse half** — a zeroed line 8a is conservative; a
missing AMT add-back is not.

Liveness predicates: `schedule_a.mortgage_interest_1098 > 0`; `capital_loss_carryforward_in.short > 0 ∨
.long > 0`.

---

## 4. Files

**Tier 1 — the form**
`btctax-core/src/tax/form6251.rs` (new; §2) · `tax/amt.rs` (screen kept as a fast path; **gains the MFS
kicker**) · `tax/tables.rs` (`AmtParams` += MFS kicker start/max; += the 26%/28% **rates**, since §6's
no-literal-constant rule otherwise reds against `amt.rs:111,123`) · `tax/return_1040.rs`
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
apply.rs}` · **`btctax-input-form/src/spec/mod.rs:110`** (hard-codes `decl_count == 7` — a guaranteed red)
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

**Discharge (r2 I-5 — these were unowned, and the original was unbuildable):** T2 carries a KAT asserting
that for every vector the set of applied AMT adjustments is exactly {line 2a, line 2b, MFS kicker} and
each is a §56(b)(1) exclusion item. **Not** an "8801 recompute" — there is no 8801 code and §1 excludes
building it.

---

## 6. Global constraints

- **Gate:** `make check` **and** `cargo fmt --all -- --check`, from the first commit.
- **Transcription is the primary gate.** Every numbered line present; every doc comment matching i6251.
- **Fail-closed at every commit.** Nothing previously refused computes unless line 7 ≤ line 10 is proven.
- **Never understate.** The MFS kicker and §3's two adverse branches are the understatement risks.
- **Every guarantee ships with a test that reds when the guarantee is removed.**
- **No literal AMT dollar amount or rate outside `AmtParams`** (`#[cfg(test)]` exempt for boundary KATs).
- **Exhaustiveness is guarded in two shapes, not one** (r2 I-6 — a source scan cannot assert a refusal):
  (i) behavioural — §57(a)(5) PAB interest still refuses; (ii) input-surface — the eleven uncapturable
  items still have no `ReturnInputs` leaf, via `spec/coverage.rs`'s existing mutate-and-diff mechanism.

---

## 7. Tier 1 tasks

### T1 — transcribe the form (BLOCKING)

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
- [ ] **Construct V7–V10** — inputs, not just purpose (r2 I-3). V9 needs a search: line 7 strictly inside
      `(1040 L16 − FTC, 1040 L16]` with the FTC at its §904(j) ceiling ($300, ×2 MFJ).

### T2 — `form6251.rs`
- [ ] Transcribe lines 1–11 and 12–40 per `PART_III.md`. Line 1 has a **negative branch** (`L15 = 0 ⇒
      1040 L11 − L14`); the `"if zero or less, enter -0-"` text belongs to **line 6**, not line 1
      (r2 Minor) — plus the one-line unreachability proof (`taxable_income` is floored at 0).
- [ ] MFS kicker in **both** `form6251.rs` and `amt.rs` — in the screen it goes on `line5`, after the
      state-refund subtraction and before the exemption test, so it also feeds the phase-out (r2 Minor).
- [ ] KATs: every §8 vector; phase-out boundaries; the 26/28 breakpoint; MFS kicker boundaries
      ($875,950 / $1,142,550); line 6 ≤ 0; line 1's negative branch; the §5 adjustment-set KAT; the two
      §6 exhaustiveness guards; the rounding-order KAT.
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
- [ ] Retarget `smoke.rs`'s `EXPECTED_REFUSED` and its two assertions — retarget, do not delete (r2 I-7).
- [ ] **Mutations:** revert to the blanket refusal ⇒ the V1 KAT reds. Replace `line7 > line10` with
      `amt > 0` ⇒ the **V9** KAT reds.

### T4 — output
- [ ] A no-attachment return's printed 1040/Sch 2 against a **hand-built expected packet** (L17 = 0,
      Sch 2 L3 = 0, no 6251).
- [ ] Add a **screen-tripping, no-attachment** journey — every bundled journey is deliberately sized
      under the screen (`testonly.rs:48-51,58-59`), so regeneration alone proves nothing (r2 I-10).
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

**Tier 1 gate:** suite green, 0C/0I, every line of the form present with matching doc text, a
no-attachment return exports a complete packet, and V4/V5/V6 reconcile against `c09600`.

---

## 8. Vectors — MFJ, TY2024. ✅ = independently recomputed and confirmed in r1.

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
  via `verify_flat`; byte-reproducible golden for V5.
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
| MFS kicker omitted ⇒ **understates** | V8 KAT absent | T2 pins both boundaries in `amt.rs` **and** `form6251.rs` |
| an adverse declaration answer computes | V-adverse KATs absent | §3's ternary; T3 KAT per declaration |
| attach test loosened to `AMT > 0` | — | T3's V9 mutation |
| line 2b dropped | — | T2's V7 mutation |
| the oracle certifies a misreading | V2/V2b insensitive to `c09600` | T1 commits hand figures as data; `c09600` cross-checks only V4/V5/V6 |
| Tier 1 read as closing G-4 | Tier 2 slips | §1's trigger rule and $26,271 exemplar |

---

## 11. SemVer

`RefuseReason` (`return_refuse.rs:33`), `AbsoluteReturn` (`return_1040.rs:836`) and `Schedule2Lines`
(`printed.rs:293`) are **plain derives, not `#[non_exhaustive]`** — adding variants/fields is breaking
(r2 Minor). Mark all three `#[non_exhaustive]` in the Tier-1 commit (cheap; `no-users-yet`), making
**Tier 1 MINOR**; otherwise Tier 1 is MAJOR for `btctax-core`. **Tier 2 MINOR.**

**Second behaviour change to disclose:** stored returns carrying a 1098 or a capital-loss carryforward
now **refuse** until the new declarations are answered.
