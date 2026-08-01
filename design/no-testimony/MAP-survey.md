# §G-11 survey MAP — synthesis of five readers over every money field on every printed form

Working material for the brainstorm. Built from five independent surveys against `main` @ `18eb2ed`.
Verbatim code quotes below re-verified against current source at write time.

---

## 1. THE NUMBER

**64 fabrication sites out of 168 money quantities that reach a PDF today — 38%.**

This is a **program, not a targeted fix.** Every printed form in the packet has at least one; the
1040 itself has 19 of 34.

### Denominator, stated honestly

| | count | note |
|---|---|---|
| raw rows surveyed | 221 | the aggregate the five readers returned |
| − duplicate quantities | −11 | `Form1040Income`'s 11 money fields are destructured verbatim into `Form1040Lines` (printed.rs:634-647); surveyed once |
| − never emitted | −42 | 41 Form 6251 lines + `Form1040Income::qdcgt_net_capital_gain` reach **no AcroForm cell** |
| **= emitted surface** | **168** | money quantities that actually print today |
| **fabrication sites** | **64** | all 64 are on the emitted surface; 0 on Form 6251 |

### Per-form breakdown

| form | emitted money fields | fabrication sites | already-suppressed | form instructs `-0-` |
|---|---|---|---|---|
| Form 1040 (`Form1040Lines`) | 34 | **19** | 0 | 2 |
| Schedule 1 | 10 | **6** | 0 | 0 |
| Schedule 2 | 4 | **1** | 0 | 0 |
| Schedule 3 | 5 | **5** | 0 | 0 |
| Schedule A | 18 | **8** | 0 | 1 |
| Schedule B | 4 | **3** | 1 (row-level) | 0 |
| Schedule C | 7 | **2** | 0 | 0 |
| Schedule D | 15 | **11** | 3 (routing) | 0 |
| Schedule SE | 12 | **1** | 3 (routing) | 2 |
| Form 8949 | 8 | **1** | 2 (cell + part) | 0 |
| Form 8283 | 3 | **1** | 1 (`Option<Usd>`) | 0 |
| Form 8275 | 1 | 0 | 0 | 0 |
| Form 8959 | 17 | **2** | 0 | 5 |
| Form 8960 | 14 | **2** | 0 | 2 |
| Form 8995 | 16 | **2** | 1 (line-level) | 5 |
| *Form 6251* | *0 emitted (41 modelled)* | *0 — **18 in waiting*** | — | 7 |
| **TOTAL** | **168** | **64** | **11** | **24** |

### ★ Two counting ambiguities that move the number — the brainstorm must settle them

The readers did **not** apply one rule consistently, and both disagreements are substantive, not
clerical:

1. **Pure carries.** Reader 3 counted Schedule B line 4 (`let line4 = line2;`) as its own site
   because it lands on 1040 line 2b. Reader 1 counted 1040 line 1z (`let line1z = line1a;`) as NOT
   a site, pushing the defect upstream per brief rule 1. **Same shape, opposite verdicts.** Applying
   "defect moves upstream" strictly to all 9 derivative totals → **55**.
2. **Ledger silence.** Reader 4 marked Schedule C line 1 and Schedule D 3(d)/(e)/(h) as sites
   because btctax has no channel for non-crypto receipts or non-crypto dispositions, and self-flagged
   that a reasonable person could call the reconciled ledger the filer's own affirmative statement —
   *"in which case those four lines drop out and the count falls from 14 to 10."*

**Defensible band: 51–64.** Even the floor is a program.

### The one structural asymmetry worth stating up front

Reader 1: **every one of the 34 `Form1040Lines` money fields is written on every full-return export**,
unconditionally, and `line34` is written **twice** (into L34 and L35a, form1040_full.rs:170-171). There
is no path on which a 1040 money cell is blank. 19 of those 34 can be fabricated.

---

## 2. THE KINDS

Grouped by **mechanism**. This is what a fix addresses; the forms are just where it surfaces.

| # | kind | count | layer |
|---|---|---|---|
| 1 | Σ over a possibly-empty `Vec` | **20** | `return_1040.rs` |
| 2 | bare `#[serde(default)] Usd` scalar (incl. struct `Default`) | **14** | `return_inputs.rs` |
| 3 | an `Option` that carries "absent" flattened to `Usd::ZERO` | **15** | `printed.rs` / `fold.rs` |
| 4 | hardcoded `let lineN = Usd::ZERO;` — the line has no field at all | **3** | `printed.rs` / `other_taxes.rs` |
| 5 | emitter literal `Usd::ZERO` — no struct field | **2** | `btctax-forms` |
| 6 | derivative total over a structurally-blank column | **9** | `printed.rs` |
| 7 | closed input surface — ledger/absent-channel silence printed as a figure | **1** | `return_1040.rs` |
| | **total** | **64** | |

### Kind 1 — Σ over a possibly-empty `Vec` (20 sites)

`Iterator::sum` over an empty iterator **is** `Usd::ZERO`. There is no `Option` to lose, so nothing
warns. The Vecs are all `#[serde(default)]`; several element boxes are too.

```rust
// return_1040.rs:494-496
let w2_ss_wages: Usd = ri.w2s.iter().filter(|w| w.owner == se_owner)
    .map(|w| w.box3_ss_wages + w.box7_ss_tips).sum();
```

Sites: 1040 1a, 2a, 2b, 3a, 3b, 25a, 25b · Sch 1 L7, L18 · Sch 3 L1 · Sch A 11, 12, 13 ·
Sch B 2, 6 · Sch D 13 · Sch SE 8a · 8959 L1, L19 · 8995 L6.
Source Vecs: `w2s`, `int_1099`, `div_1099`, `g_1099`, `charitable`, `charitable_carryover_in`.

★ **The repo already names this exact hazard, in its own words, for a different field** —
`return_inputs.rs:551-557`: *"`CharitableCarryItem` already carries a per-item provenance, which is
useless for the case that matters: an **EMPTY vec has no items**, so an empty list carries no
provenance at all — and an empty list is exactly the state that is ambiguous between "no carryover"
and "never asked"."*

### Kind 2 — bare `#[serde(default)] Usd` scalar (14 sites)

A missing TOML key and a stated zero are the same bytes.

```rust
// return_inputs.rs:260-261
#[serde(default)]
pub expenses: Usd,
```

Sites: 1040 25c, 26 · Sch 1 L1, L21 · Sch 3 L10 · Sch A 1, 5a, 5b, 5c, 8a · Sch C 28 ·
Sch D 6, 14 (via `Carryforward::default()`) · 8995 L7.

★ **The asymmetry is exact:** `parse_return_inputs_toml` (cmd/tax.rs:113-131) goes to real lengths to
reject **UNKNOWN** keys via `serde_ignored` — and defaults **MISSING** ones in silence. A typo is
loud; an omission is invisible.

★★ **And serde-requiredness is already used as an answered-ness mechanism, on exactly three boxes:**
`W2::box1_wages`, `Form1099Int::box1_interest`, `Form1099Div::box1a_ordinary` carry **no**
`#[serde(default)]` — a TOML omitting them refuses to parse. The mechanism exists; it was applied to
three boxes and not the other ~30.

### Kind 3 — an `Option` flattened to zero (15 sites)

**The most concentrated and the cheapest to fix**: in every one of these the `None` *already means
exactly* "this does not file / there is none", and it is thrown away one line before it would matter.

```rust
printed.rs:591   let line8  = sch_1.map_or(Usd::ZERO, |s| s.line10);
printed.rs:593   let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
printed.rs:654   let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
printed.rs:677   let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
printed.rs:680   let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
printed.rs:690   let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
printed.rs:866   let st = f8949.map(|f| f.st_totals).unwrap_or_default();
printed.rs:867   let lt = f8949.map(|f| f.lt_totals).unwrap_or_default();
```

Plus `Sch 2 L12` (`f8960.map_or`), and the **ledger fold**, which is a layer lower again:

```rust
// project/fold.rs:1151
let usd_basis = basis.unwrap_or(Usd::ZERO);   // conservative $0 default (max eventual gain)
```
→ Form 8949 column (e) and Form 8283 column (g). The defaulted-vs-supplied signal survives only in a
*separate* Advisory blocker (`SelfTransferInboundZeroBasis`, state.rs:100) which **never gates**, and
`event.rs:26-29` says so outright: *"the defaulted-vs-supplied signal rides the ... advisory, not this
source."*

★ On the 8949 pair, `printed.rs:73-74` states the intended output and the emitter does the opposite:
*"`None` when the year has no disposals — a carryover/distribution-only Schedule D files with lines
3/10 blank and NO 8949 attached."* It prints `0` and attaches no 8949.

### Kind 4 — hardcoded `Usd::ZERO` in core, no field anywhere (3 sites)

```rust
printed.rs:674     let line17 = Usd::ZERO;  // Schedule 2 Part I is blank in v1
printed.rs:676     let line19 = Usd::ZERO;  // ★ CTC/ODC — a §3.4 conservative omission (advisory fires)
other_taxes.rs:312 let line9d = Usd::ZERO;  // v1 models no investment expenses
```

### Kind 5 — emitter literal, no struct field (2 sites)

The **only two of the 64 that originate inside `btctax-forms`.** Schedule D lines 18/19, written as
`push_p2(..., Usd::ZERO, ...)` on the `BothGains` branch (schedule_d_full.rs:201-212), pinned by a KAT
asserting `Some("0")`.

### Kind 6 — a derivative total over a structurally-blank column (9 sites)

Not arithmetic over printed lines: **arithmetic over lines that do not exist as fields.**

```rust
printed.rs:402   let line9  = line8v;   // 8a-8u and 8w-8z are blank
printed.rs:691   let line32 = line31;   // 27-30 blank (EIC omitted conservatively; rest unrepresentable)
printed.rs:1322  let line8  = line1;    // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
```

Sites: 1040 7, 32 · Sch 1 L9, L10, L26 · Sch 3 L8, L15 · Sch B 4 · 8960 L11.

★★ **This is the sharpest self-contradiction in the whole survey.** `Schedule3Lines`' own doc
(printed.rs:1296) says of the omitted credits *"They are left BLANK, never a misleading 0"*, and the
map header repeats it. **The individual boxes honour it exactly. Then `let line8 = line1;` prints `0`
on the TOTAL of those same blanks.** Schedule 1 line 9 is the extreme case: **20 of 21 operands are
structurally absent.**

### Kind 7 — closed input surface (1 site, contested)

Schedule C line 1: gross receipts from the ledger, on a business whose non-crypto receipts btctax has
no channel to ask about. See §1 ambiguity 2.

---

## 3. WHERE THE DEFECT ACTUALLY LIVES

## ★★★ 62 of 64 are manufactured BEFORE the emitter. Changing `fmt_money` and the printed types would fix TWO.

All five readers converged on this independently, unprompted, and three of them volunteered it as the
more important half:

- Reader 3: *"**Not one of the 11 fabrication sites originates in `btctax-forms`.** ... Fixing
  `fmt_money` alone would give the emitter a vocabulary with nothing left to say."*
- Reader 4: *"11 of the 14 are `Transcribed` — the emitter is faithfully printing a `Usd::ZERO` that
  was manufactured in `return_1040.rs` from an absent input."*
- Reader 5: *"a `Option<Usd>` money type in btctax-forms alone would fix nothing: the emitter would
  faithfully receive a `Some(0)`."*
- Reader 2: *"The emitter is a faithful transcriber; every one of the 12 sites is decided before
  `printed.rs` is reached."*
- Reader 1: *"Ten of the nineteen ... in six of them an `Option` that already carries exactly the right
  information is destroyed at the 1040 boundary."*

### Coercion by layer

| layer | file | sites | what happens |
|---|---|---|---|
| input schema | `return_inputs.rs` | 14 | `#[serde(default)] Usd` — missing key ⇒ `ZERO` |
| ledger fold | `project/fold.rs:1151` | 2 | `basis.unwrap_or(Usd::ZERO)` |
| absolute return | `return_1040.rs` | 20 | Σ over empty `Vec` |
| printed structs | `printed.rs`, `other_taxes.rs`, `qbi.rs` | 26 | `map_or(ZERO)`, hardcoded consts, blank-column totals |
| **emitter** | **`btctax-forms`** | **2** | **emitter literals (Sch D 18/19)** |

### ★ The emitter is not the constraint — but it is not innocent either

`push_money` (cells.rs:44-69) is unconditional on every path:

```rust
MoneyCell::Single(fqn) => { w.push((fqn.clone(), FieldValue::Text(fmt_money(value)))); }
```
```rust
// lib.rs:78  (FOLLOWUPS §G-11 cites lib.rs:77 — off by one on current main)
pub(crate) fn fmt_money(d: Usd) -> String { d.to_string() }
```

So the emitter cannot *express* a blank even where core knows one is right — and core, in at least
four places, demonstrably knows (printed.rs:73-74, printed.rs:1296, return_inputs.rs:551-557,
`Advisory::QbiCarryforwardNotStated` / `BenefitCarryoversNotStated` firing on the very returns that
print the zero). **Both ends need the vocabulary. Only the upstream end needs new information.**

### ★★ Three more upstream facts that bound any fix

1. **The input-form seam cannot express it either.** `seam.rs:154-163` — `FieldValue::TriState(Option<bool>)`
   and `Date(Option<Date>)` exist *precisely* so "never asked" survives to storage (the P9
   answered-ness work). **`Money(Usd)` has no such state.** Even on the interactive path a pre-filled
   `0` the filer scrolled past and a typed `0` are the same value.
2. **The answered-ness classifier is structurally blind to money.** `classifier.rs` r2 M-6 forbids a
   bare `_` on an `Option<bool>` leaf — *"the whole point of the census is to distinguish 'this encodes
   no decision' from 'we forgot it'"* — and **explicitly permits `_` on every `Usd` leaf**. So the one
   instrument built to catch this class cannot see any of the 64. `classify_payments` records
   **nothing at all**, not even an `exempt(..)` with a reason.
3. **`PrintedInputs` already carries the right rule, one layer too high.** return_1040.rs:1077-1080:
   *"**No `Default`** — deliberately ... a silently-zeroed field here is a wrong number on a filed
   return."* The struct cannot be defaulted — and the Σ expressions that *fill* it manufacture the
   zeros anyway.
4. **Six leaves have no interactive path at all.** `coverage.rs:300-342`: `EXEMPT_PREFIXES` =
   `["int_1099", "div_1099", "g_1099", ...]`, `EXEMPT_LEAVES` includes `sch1.state_refund_taxable`,
   `sch1.student_loan_interest_paid`. The census is **one-directional**: it asserts every non-exempt
   leaf IS covered, and **nothing asserts that an exempt leaf's printed line renders blank rather than
   `0`.** B1-shaped gap — a checker never watched red on this defect.

---

## 4. WHAT THE FORMS SAY — the constraint a fix must not break

**24 lines where the form instructs the zero.** These are *correct* and must keep printing.
Verbatim, grouped:

### Form 1040 (2)
- **L15** — *"Subtract line 14 from line 11. If zero or less, enter -0-. This is your taxable income"*
- **L22** — *"Subtract line 21 from line 18. If zero or less, enter -0-"*

### Schedule A (1 — the only `-0-` on the whole form)
- **L4** — *"Subtract line 3 from line 1. If line 3 is more than line 1, enter -0-"*

### Schedule SE (2)
- **L9 (and L10 by the same sentence)** — *"Subtract line 8d from line 7. If zero or less, enter -0- here and on line 10 and go to line 11"*

### Form 8959 (5)
- **L6** — *"Subtract line 5 from line 4. If zero or less, enter -0-"*
- **L8** — *"Self-employment income from Schedule SE (Form 1040), Part I, line 6. If you had a loss, enter -0-"*
- **L11** — *"Subtract line 10 from line 9. If zero or less, enter -0-"*
- **L12** — *"Subtract line 11 from line 8. If zero or less, enter -0-"*
- **L22** — *"Subtract line 21 from line 19. If zero or less, enter -0-. This is your Additional Medicare Tax withholding on Medicare wages"*

### Form 8960 (2)
- **L12** — *"Net investment income. Subtract Part II, line 11, from Part I, line 8. ... If zero or less, enter -0-"*
- **L15** — *"Subtract line 14 from line 13. If zero or less, enter -0-"*

### Form 8995 (5)
- **L4** — *"Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-"*
- **L8** — *"Total qualified REIT dividends and PTP income. Combine lines 6 and 7. If zero or less, enter -0-"*
- **L13** — *"Subtract line 12 from line 11. If zero or less, enter -0-"*
- **L16** — *"Total qualified business (loss) carryforward. Combine lines 2 and 3. If greater than zero, enter -0-"*
- **L17** — *"Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 6 and 7. If greater than zero, enter -0-"*

### Form 6251 (7 — not emitted today, but the constraint lands with Tier 2)
L6, L10, L11, L20, L21, L27, L29. Sharpest: **L6** — *"Subtract line 5 from line 4. If more than zero,
go to line 7. **If zero or less, enter -0- here and on lines 7, 9, and 11**, and go to line 10"* — an
instructed zero that *propagates by name to three other lines*.

### ★ Schedules 1, 2 and 3 contain NO `-0-` instruction at all
`grep -n -- '-0-'` over all three extracted texts returns nothing (exit 1). Schedule C likewise: none.
**Every one of the 19 fabrication sites on those four forms is on a line the form is silent about.**

### ★★ Four constraints pointing the OTHER way — forms that instruct a BLANK

1. **Form 8959 masthead**, verbatim: *"**If any line does not apply to you, leave it blank.** See
   separate instructions."* A form-level blank instruction, over a form that today prints `0` on
   L1/4/6/7/19/20/21/22/24 for a pure-SE filer with no W-2.
2. **Form 6251 L34 routing**: *"If line 14 is zero **or blank**, skip lines 35 through 37..."* — the
   IRS itself treating zero and blank as two distinct states of one cell.
3. **Form 6251 L20 routing**: *"Are lines 18 and 19 both zero **or blank** and you are not filing Form
   4952?"* — so Schedule D's hardcoded `0` on lines 18/19 **buys nothing**; L20 routes to Yes either way.
4. **Form 1040 L7 line text**: *"Capital gain or (loss). Attach Schedule D if required. **If not
   required, check here**"* — the form's own remedy for the no-Schedule-D case is a **checkbox, not a
   zero**.

### ★ A rendering question the survey surfaced twice
Where the form says `-0-`, **btctax writes the bare string `"0"`** (`fmt_money` = `Decimal::to_string`).
The only `push_literal(..., "-0-", ...)` for a money cell in the repo is the crypto-slice 1040 line 7a.

---

## 5. ALREADY-SOLVED PRECEDENT

Five working patterns exist in-repo. **A fix should extend one, not invent one.**

### (a) ★★ Line-level blank-vs-zero — exists exactly ONCE, and its own comment is falsified

`form8995.rs:245-256`, the single true instance on the whole emitted surface:

```rust
// ★★ LINE 3 IS WRITTEN ONLY WHEN THERE IS A CARRYFORWARD. It is `Qualified business net (loss)
// carryforward FROM THE PRIOR YEAR` — a fact the filer supplies about a return btctax did not
// compute. A printed `0` there is an affirmative sworn statement that they had no prior-year
// QBI loss; a BLANK is no statement at all ...
//
// Every other line here is DERIVED from figures already on the page, so a zero is btctax's own
// arithmetic and prints legitimately. Line 3 is the only line on this form that is neither
// derived nor computed — it is testimony — which is exactly why it is the only one gated.
if std::ptr::eq(*cell, &map.line3) && value.is_zero() {
    continue;
}
```

★ **"Line 3 is the only line on this form" is false on the very next iteration of the same loop.**
Line 7 — *"Qualified REIT dividends and qualified PTP (loss) carryforward from the prior year"* — is
line 3's exact structural twin, a prior-year figure from a return btctax did not compute, and it is
ungated. Note the mechanism, too: `std::ptr::eq` against one map cell — **an identity comparison, not
a property of the value.**

### (b) ★★★ `Option<T>` already reaches the writer — for CHECKBOX answers, three times

This is the mature pattern, and it is **structural rather than conventional** by design:

```rust
// schedule_b.rs — Part III (7a, the FBAR sub-question, 8)
// ★ EVERY Part III answer is written iff the filer ACTUALLY gave it. `None` means the question was
//   never answered (or, for the FBAR sub-question, never even asked because 7a was "No"), and a
//   checked box would then be testimony nobody gave. This `filter_map` is the mechanism that makes
//   "not answered ⇒ blank" structural; `unwrap_or(false)` here was a live fabricated-testimony bug.
.filter_map(|(pair, answer)| answer.map(|a| (pair, a)))
```

```rust
// schedule_c.rs:107-118 — lines I and J
// The `if let Some(..)` is the whole guarantee. `None` (never asked) and `Some(false)` (asked,
// answered no) are DIFFERENT marks on the page — an unwritten pair versus a checked No box — and
// an `unwrap_or(false)` here would print a "No" the filer never gave, on a form they sign under
// §6065. ... it is structural here rather than conventional, which is what makes it hold.
if let (Some(pair), Some(answer)) = (pair, answer) { ... }
```

```rust
// form8283.rs:139-141 — lines 5a/5b/5c
// ★ §G-21 — the filer's answer to lines 5a/5b/5c. `Some(false)` ⇒ all three print No; `None` ⇒ all
// three stay BLANK. The crypto slice always passes `None`: it writes no Section B declarations.
no_restrictions: Option<bool>,
```

**The type reaches the writer, and the writer's default action is to write nothing.** That is exactly
the shape the money path lacks.

### (c) ★ ONE money leaf is already `Option<Usd>` end-to-end

`Form8283Row.claimed_deduction` — `form8283.rs:436`:
```rust
if let Some(ded) = row.claimed_deduction { push_money(...) }
```
Non-carrier leg rows leave column (i) genuinely blank rather than printing a `0` that would
double-count. **The only proof in the repo that a blank money cell is achievable today.**

### (d) Row- and part-level suppression (ad hoc, value-derived, NOT answered-ness)

```rust
// schedule_d.rs:34-37  — a PART-level gate on the crypto slice only
/// A part is "active" (worth a Schedule D line) iff it has any proceeds/cost/gain.
fn active(p: &btctax_core::ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}
```
```rust
// fill8949.rs:44-48  — ONE cell, column (g), on the crypto slice
if r.adjustment_amount.is_zero() { String::new() } else { r.adjustment_amount.to_string() },
```
Plus `fill8949.rs:84` `if data.rows.is_empty() { return; }` (whole part), `fill8949.rs:101`
`if value.is_empty() { continue; // blank cell — do not write, do not authorize }` (the generic
empty-string skip the (g) blank rides), and `printed.rs:1012` `.filter(|r| r.amount > Usd::ZERO)` —
Schedule B payer rows, suppressed **in the constructor**, with the reason stated: *"A payer with
nothing to report is not listed: a zero row would name someone on a federal form for no reason."*

★ **Critical limits.** `active()` is **per-PART, never per-CELL** (an active part with a $0 cost basis
still writes `"0"` in column (e)); it exists **only** in `schedule_d.rs`, and the full-return emitter
`schedule_d_full.rs` has **no equivalent**; the two emitters are mutually exclusive per year, so on a
full return **it is not in play at all**. Every one of these keys off the **value**, not off whether
anyone was asked.

### (e) Form-level and routing-level blanks

`schedule_N_lines() -> None` ⇒ `packet.rs:77-96` skips the whole PDF. `ScheduleDRouting`
(schedule_d_full.rs:191-259) leaves L21 untouched on 3 of 4 branches. `let skip_8d_to_10 =
lines.line9 == Usd::ZERO;` (schedule_se_full.rs:78). **All three answer "does the form's own routing
reach this line", never "does this line have testimony behind it."** Schedule 2 line 21 is the only
line in the entire survey structurally incapable of printing a zero — because its gate is on its own
value and a zero total is expressed as an *absent form*.

### (f) The provenance-sibling pattern — built, wired to advisories, deliberately NOT printed

`CarryProvenance` siblings exist for two carryovers:
`capital_loss_carryforward_in_provenance` (return_inputs.rs:516-527) — *"without it a zero is
uninterpretable — 'the filer has no carryover' and 'nobody ever asked' are the same bytes"* — and
`charitable_carryover_in_provenance` (return_inputs.rs:551-557).

★★ `advisories.rs:621-633` computes **exactly the predicate a blank-capable line would need**:
```rust
let user = crate::tax::return_inputs::CarryProvenance::User;
let cl = ri.capital_loss_carryforward_in.short == Usd::ZERO
    && ri.capital_loss_carryforward_in.long == Usd::ZERO
    && ri.capital_loss_carryforward_in_provenance == user;
```
and fires `Advisory::BenefitCarryoversNotStated`. **So on the very return where btctax tells the filer
"your carryovers were not stated", it simultaneously prints `0` on Schedule D lines 6 and 14 asserting
they are zero.** `classifier.rs:131-133` classes the bit *"no print, no tax direction"* — the one bit
that distinguishes the two blanks is deliberately kept off the page.

### (g) The `Option<Usd>` + `Option<bool>`-gate design, already written down and argued

`return_inputs.rs:620-636`, the §911/931/933 add-backs:
> *"`Option<Usd>`, NOT defaulted `Usd`, and the distinction is the whole point. A plain `Usd`
> defaulting to 0 cannot tell "zero because the filer has none" from "zero because nobody asked""* —
> with the gate carried by an `Option<bool>` declaration because *"`Option<bool>` is a leaf the
> classifier **forbids** `_` on, so it cannot be added without a human classifying it — whereas
> `Option<Usd>` is a scalar the `_` rule permits, which would make this convention again."*

**That paragraph is a ready-made design, and it names the reason a bare `Option<Usd>` is not
sufficient on its own.**

---

## 6. THE HARD CASES

Where "blank or zero" is genuinely ambiguous, or turns on a fact btctax does not have.

### H1 — What does a TOTAL render as when an operand is not stated?
**Unavoidable: 9 of the 64 are totals.** Schedule 1 line 9 is the extreme — *"Add lines 8a through
8z"* with **20 of 21 operands structurally absent** (no fields exist). Schedule 3 line 8 sums five
credits the product deliberately never asks about and whose advisory exists *because they may well be
nonzero*. Making operands `Option<Usd>` does not by itself decide what `Some(5000) + None + None`
prints. All three lawful moves (collect / refuse / blank) are defensible **per line**, and inheriting
one rule for all nine is probably wrong.

### H2 — Is a reconciled LEDGER the filer's testimony?
Schedule C line 1, Schedule D 3(d)/(e)/(h), 1040 line 7, Schedule 1 line 8v. Reader 2 said yes for 8v
(*"btctax refuses to produce a return for an unreconciled year"*); Reader 4 said no for Sch C L1
(no channel for non-crypto receipts) and self-flagged the reversal. Moves the count by ~4.

### H3 — The SAME CELL, two emitters, opposite dispositions — and the rule is already written
`form1040.rs:8-10` (crypto slice), verbatim: *"Schedule D INACTIVE ... → **7a BLANK**. Stamping "-0-"
against a blank Schedule D line 16 would be an unearned zero-capital-gains claim."* The full-return
emitter writes the same cell via `push_money` with no test (form1040_full.rs:115). One of the two is
wrong, and the survey cannot say which without deciding H2. **B3-shaped: the fix already exists in the
tree and nobody carried it across.**

### H4 — Supplied-then-zeroed: a computed zero over a fact the filer DID state
An `Option<Usd>` at the leaves misses these entirely:
- **Sch A 8a** — `let mortgage_8a = if mortgage_mixed_use_box { Usd::ZERO } else { a.mortgage_interest_1098 };`
  (return_1040.rs:410-414). The filer reports $12,000 of Form-1098 interest and holds the 1098; the
  cell swears `0`. Only a checked box and an advisory qualify it — neither is on the amount line.
- **Sch A 11/12** — `current_allowed = current_total.min(ceiling)`, `ceiling = 0.60 * agi.max(ZERO)`
  (charitable.rs:63,117-119,141-147). A $10,000 gift on a ≤0-AGI return prints `0` and routes to
  `carryover_out`.
Is this in scope for §G-11, or a separate class?

### H5 — The mirror defect: an INSTRUCTED zero that btctax suppresses
Schedule SE. The form gives two instructions pointing opposite ways: line 8a's *"If $168,600 or more,
skip lines 8b through 10, and go to line 11"* versus line 9's *"If zero or less, enter -0- here and on
line 10 and go to line 11"*. `skip_8d_to_10` follows the first and blanks all three. Needs
adjudication against `i1040sse`. **§G-11's fix could make this worse if it only ever adds blanks.**

### H6 — Elections and conditional lines are a different class than amounts
- **1040 L35a** — *"Amount of line 34 you want refunded to you"* is an **election** (refund now vs
  apply to 2025 on line 36). btctax never asks, writes `line34` into the cell, leaves L36 blank.
- **1040 L34/L37** — the form makes L34 conditional in terms (*"**If line 33 is more than line 24**"*)
  and prints `0` when the condition fails; both print `0` when payments exactly equal tax.
The brief's test ("was the fact supplied?") does not cleanly apply to an unasked *choice*.

### H7 — Where the `None` genuinely means "computed, and there is none"
`Sch 2 L4` (`sch_se.map_or(ZERO)` — the §6017 $400 floor was not met) and `Sch 2 L11`. Reader 2 called
these NOT fabrications; Reader 1 called the structurally identical `1040 L23`
(`sch_2.map_or(ZERO)`) a fabrication because the total cites a schedule that is not attached.
**The readers split on the same idiom.** Is "a schedule that legitimately does not file" a stated zero
or an absent one? Note the form text on Sch 2 L4 is *"Self-employment tax. **Attach Schedule SE**"* —
and no Schedule SE is attached.

### H8 — Closed surfaces with no channel at all
Not defaulted — **absent**. Form 8960 lines 9a/9b/9c/10 (and every filer reaching 8960 is over the
§1411(b) threshold, i.e. exactly the population with allocable state income tax under 9b). Schedule 1
lines 2a/2b, 4, 5, 6 and 8a–8z. Schedule 2 Part II (Sch H, Form 5329, 4137/8919). Schedule 3 line 9
(net PTC) and 1040 line 17's excess-APTC half. 1099-R withholding (1040 25b — *i1040gi's own primary
case for that line*). Direct sales of collectibles / §1250 realty (Sch D 18/19). **Blank the line,
refuse the return, or advisory?** All three are defensible and the choice differs per line.

### H9 — 1040 line 19, the CTC/ODC: a hardcoded zero on a return that names the child
`fullreturn_inputs.toml` carries `[[header.dependents]] name = "Sam Doe" / relationship = "Son" /
date_of_birth = [2012, 106]` — a 12-year-old CTC-qualifying child, printed into the dependents table
with the CTC/ODC boxes **deliberately unchecked** — and `let line19 = Usd::ZERO;`. The struct's own
doc **concedes the discrimination and declines it**: *"Conservative omissions print as absent, not
zero, where the form allows — but line 19 (the CTC/ODC) is a computed credit line the form expects, so
it prints `0`."* Is "the form expects a computed credit line" a sufficient reason?

### H10 — Direction is not uniform, and one site runs the *other* way
Most fabrications overstate tax (harmless to the fisc, costly to the filer, **invisible to both
oracles and every reconciliation**). Two classes do not:
- **Schedule D line 13** (capital gain distributions) **understates** tax — omits real long-term income.
- **1040 25a/25b/26** (withholding, estimated payments) deflate line 33 and inflate line 37 *"amount
  you owe"* — the exact shape the repo was burned by once and wrote down: *"★ Dropping this line told a
  filer who had ALREADY paid with their extension to pay it again"* (printed.rs:1306-1309).
§G-11 forbids opining on lawfulness — but does direction legitimately order the work?

### H11 — Form 6251: 18 sites in waiting, including the standing example
Lines 2c–2t have **no fields**, the form reaches no PDF, and `line4 = line1 + line2a + line2b + line3`
treats all 18 as $0. The module doc already schedules the moment it becomes a defect: *"Tier 2, which
files the form rather than only computing it, must give 2c–2t real fields."* If those land as `Usd`
under today's unconditional `push_money`, **CLAUDE.md's standing example — an ISO exercise printed as
$0 on line 2i — ships on that commit.** Sequencing decision, not a code decision.

---

## 7. OPEN QUESTIONS FOR THE BRAINSTORM

1. **Is the not-stated state a property of the VALUE or of the LINE?** `Option<Usd>` threaded through
   `ReturnInputs` → `AbsoluteReturn` → printed structs → `push_money` makes it a value. A per-line
   decision table (each line records which of the three lawful moves it takes, and why) makes it a
   line. The `form8995.rs` precedent is a *line* gate (`std::ptr::eq` on a map cell); the `8283
   claimed_deduction` precedent is a *value* gate. Both work today, on one line each.
2. **What does a total do with a not-stated operand?** Propagate not-stated · treat as zero and print ·
   refuse · decide per line. Schedule 1 line 9 (20 of 21 operands absent) and Schedule 3 line 8
   ("never a misleading 0" on the parts, `0` on the sum) are the cases that force it.
3. **Does a reconciled ledger constitute the filer's testimony?** Determines Schedule C line 1,
   Schedule D 3/10, 1040 line 7 — and therefore whether the number is 64 or ~60, and whether the
   crypto slice's blank or the full return's `0` on 1040 7a is the bug (H3).
4. **Is a pure carry (`let lineN = lineM;`) its own fabrication site or an upstream one?** Readers
   split on identical shapes (Sch B 4 = yes, 1040 1z = no). Moves the number by up to 9 and decides
   whether the fix must touch the totals at all.
5. **Serde-requiredness or `Option`?** Both mechanisms are already in the repo doing this job:
   `box1_wages`/`box1_interest`/`box1a_ordinary` carry no `#[serde(default)]` and a TOML omitting them
   *refuses to parse*; the §911 add-backs use `Option<Usd>` + an `Option<bool>` gate. Requiredness
   fails loud at parse and breaks every existing TOML; `Option` fails silent and needs a classifier
   rule to stay honest.
6. **Does `ReturnInputs` change shape, or does provenance ride alongside?** `CarryProvenance` is the
   in-repo sibling pattern (built, advisory-wired, deliberately not printed). Siblings are
   non-breaking and already proven; a shape change is the thing the compiler can enforce.
7. **Blank, or refuse?** §G-11 gives exactly three lawful moves. For a line where silence *asserts*
   (class A) — 1040 25a withholding, Schedule SE 8a Medicare wages, both document-matched by the
   Service against copies it already holds — is blank sufficient, or must the return refuse? Answering
   "always blank" makes the fix mechanical and may be wrong.
8. **Should the classifier's `_`-forbidden rule extend to `Usd` leaves?** It is the instrument built
   for exactly this and it currently exempts every money field by design (*"`_` is PERMITTED on other
   scalar leaves (`String`, `Usd`, `Date`, …)"*). Extending it makes the census the permanent gate and
   `E0063`s every omission — at the cost of forcing a human classification on ~200 leaves. B1 applies:
   *which test reds when this checker is removed?*
9. **Does the input-form seam grow `Money(Option<Usd>)`?** Without it, even the interactive path
   cannot express not-stated (a pre-filled `0` scrolled past == a typed `0`), and Schedule 3 line 10 —
   the one fabrication site *with* a real interactive field — stays broken. With it, every money field
   in the TUI needs a rendering decision for the empty state.
10. **Do we print `"0"` or `"-0-"` where the form instructs a zero?** `fmt_money` emits the bare
    string today; the only `-0-` literal in the repo is the crypto-slice 1040 line 7a. Cosmetic, or
    part of the same speech-act argument?
11. **Is "supplied-then-zeroed" (H4) in scope?** Schedule A 8a mixed-use and the §170(b) charitable
    ceilings print `0` over facts the filer *did* state. Not fabrications under the brief's test — but
    they are affirmative sworn zeros contradicting the filer's own testimony, and no leaf-level fix
    touches them.
12. **Are unasked ELECTIONS a separate program?** 1040 line 35a (refund vs apply-to-2025) and the
    L34/L37 conditional pair are unasked *choices*, not unasked *amounts*. Fold in, or file separately?
13. **Does Schedule SE 8d/9/10 get fixed in the same breath (H5)?** It is the mirror defect — an
    instructed `-0-` that btctax suppresses. A fix that only ever adds blanks entrenches it, and
    adjudicating it needs `i1040sse`, not the form text.
14. **Does Form 6251 Tier 2 land before or after this fix?** 18 sites in waiting including the
    standing ISO/2i example. Fix-first costs a rebase; Tier-2-first ships CLAUDE.md's canonical defect
    on purpose.
15. **Does direction order the work?** ~62 of 64 overstate tax; Schedule D 13 and the 1040
    withholding/payments cluster understate it or inflate "amount you owe". §G-11 forbids opining on
    lawfulness — is sequencing by direction a legitimate use of that fact, or the same judgment
    smuggled in?
16. **What is the B1 kill-test?** Every fix here is a *suppression*, and a suppression that fires when
    it should not is invisible — a missing cell looks like the common correct case. The negative test
    has to plant a defect that *prints a zero it should not* AND one that *blanks a line it must not*
    (the 24 instructed zeros). What single instrument reds on both?
