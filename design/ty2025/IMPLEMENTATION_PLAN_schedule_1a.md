# Schedule 1-A (TY2025) — IMPLEMENTATION PLAN

**Status: r5** — lineage: r1 → r2 → r3 (the provenance census) → **r4 fold** (two lenses, 0C/9I) →
**r5 fold** (this document, 2026-09-05: the r4 FOLD ITSELF reviewed — **0 Critical / 4 Important /
4 Minor / 1 Nit**, one lens, persisted verbatim at `reviews/PLAN_schedule_1a-r4fold-review.md`; see the
**r5 FOLD** block below). ★ This header read *r3* while already carrying an r4 fold, which is the same
drift r5 N-1 measures in the citations — so the status line is itself part of the artifact under review.
(r2→r3: the 13-agent provenance census folded — see `reviews/PROVENANCE_CENSUS_schedule_1a.md`. It found a MISSING INPUT SURFACE and a hole in T2's own conformance approach; C-1 grew. Earlier: (r1 folded 2026-07-29 — **2 Critical, 4 Important, 2 Minor, 1 Nit**, all confirmed against the text layer before folding; reviewer output persisted verbatim at `design/ty2025/reviews/PLAN_schedule_1a-opus-r1.md`, with my re-verification notes appended there rather than mixed into it). The two Criticals were both **missing eligibility**, not wrong arithmetic — the defect class this project keeps rediscovering. Implements `design/ty2025/SPEC_schedule_1a.md` **r3** (0 Critical / 0 Important),
which is branch **B3** of `design/ty2025/SPEC.md` §8a. Parent decisions D-1 … D-11 bind.

**★★★ r4 FOLD (2026-08-01) — 0 Critical / 9 Important, two independent lenses.** This plan read
**r3** while `reviews/` held exactly ONE independent review of it; the r2→r3 fold was a 13-agent
provenance CENSUS, which is not a review, and it *grew* a Critical. Two folds went unreviewed against a
rule that says re-review after every fold. Reviews persisted verbatim at
`reviews/PLAN_schedule_1a-conformance-r4.md` and `reviews/PLAN_schedule_1a-buildability-r4.md`.

| # | finding | resolution |
|---|---|---|
| C-I1 | T2's KAT cannot produce 48 from the extract — the text layer yields **50** labels; lines **4 and 22** are headings with instruction text and no box | drive the expected set from `xtask/src/label_reader.rs` (landed 2026-07-30, one day AFTER this plan's last commit), which already adjudicates 50 / 48 / 2 with a required `note` per non-Amount row |
| C-I2 | **no COMPLETION conditions** — the only gate is `L38 > 0`, so a car-loan-only filer computes 15 phase-out lines the form says to skip and **line 35 prints $6,000 for a non-senior**; a filer with no §911 exclusion prints `$0` on 2a–2e | ★ **SUPERSEDED BY r5 I-1.** The r4 answer — transcribe *each part's Caution* — closes **neither** case in the finding column: Part V's Caution omits the birth date, and Part I has no Caution at all. The completion **source is named per part** and the predicate is **per line**; see §T2 CORRECTION 4. The half that stands: the affected leaves must be able to express *not completed* **at T2**, because B4 cannot fix it if T2 makes every line non-optional. |
| C-I3 | ★ the **refinance balance cap** was LOST in the r1 fold, and the prose states the rule BACKWARDS | see T3 below — added as a per-vehicle condition; **understates tax** without it |
| C-I4 | Part II's *"not qualified tips"* list has **three** bullets; the table carries one | add the illegal-service exclusion, worded on the IRS's own matched examples |
| B-I1 | the KAT has CLAUDE.md's half (a) and **not half (b)** — no per-line provenance, so "present but never populated" passes | every line carries a `Production` (or an `Exception` with a reason); build the actual set as `(label, got.lineN)` pairs so the compiler ties names to the struct |
| B-I2 | `line_coverage` was structurally blind to a new `schedule_1a.rs` | ⚠ **TWO-THIRDS FIXED IN CODE** (r5 I-2 corrected the r4 ✅). **Landed:** scope is derived from the emitter (`crates/xtask/src/line_coverage_check.rs:471-484`) and `Option<Usd>` counts as money (`crates/xtask/src/line_coverage_check.rs:883-888`). **Not landed:** `LineCoverage.year` is per-row but **no constructor can set it**, and `Coverage::exception` was never widened to `Option<Usd>` — the exact type CORRECTION 4 needs on lines 10/18/27/33. Both are §T2 items below. |
| B-I3 | T2's doc-comment gate is `cite-check` semantics: it proves a quotation is *the form's* words, not *that line's* | use `tables.rs::printed_line(label)` — which **T1, in this same plan, already built for exactly this**, whose own doc says *"The fix is not more citation checking"* |
| B-I4 | T3a refuses on `schedule_c.is_some()` | ✅ corrected in T3a below — gate on the CLAIM |
| B-I5 | line **4b** (Form 4137) is a third line of the T3a shape with no recorded disposition | ✅ added to the T3a table |

★★ **AND THE CENTRAL SEQUENCING CLAIM WAS CORRECTED.** The author had concluded §G-11's `line_coverage`
was the prerequisite for T3's ~25 `Option<Usd>` inputs. It is not — `line_coverage` is a **printed-line**
instrument and does not answer the input-side question. The input-side answer **already exists**:
`crates/btctax-core/src/tax/return_inputs.rs:960-1010` (r5 N-1 — the r4 fold cited `:626-652`, decayed),
where answered-ness rides an `Option<bool>` **class-(A) gate** (which the
classifier *forbids* `_` on) with the amounts hanging off it as plain `Usd`. Its own doc says an
`Option<Usd>` is a scalar the `_` rule permits — which would make this convention again.
**⇒ T3 uses ONE CLAIM GATE PER PART, not ~25 loose `Option<Usd>`s.** Part I needs no new input at all —
its four add-backs are already collected.

★ **Sizing correction (r4 M-1):** "~25 leaves plus six declarations" is stale by ~3×. Recount: Part II 5,
Part III 3, **Part IV 9 PER VEHICLE**, plus the SSN bar per person for II/III/V ⇒ **≈19 declarations**,
against a line-22 structure (2 rows × 3 columns) `ReturnInputs` has no shape for. T3's touchpoint list
also omits `crates/btctax-input-form/src/attribute.rs` and the pinned counters
(`crates/btctax-core/src/tax/questions.rs:1386-1387`, and `EXPECTED_LEAF_PATHS` at
`crates/btctax-input-form/src/spec/coverage.rs:493-662`, which is **93 rows**, not 80 — both addresses
and the count re-measured in the r5 fold, and neither changes the conclusion). All compiler- or
test-forced, so nothing escapes — but
"landed whole in one pass" is a materially larger pass than stated.

**★★★ r5 FOLD (2026-09-05) — 0 Critical / 4 Important / 4 Minor / 1 Nit, one lens, and its artifact
was THE r4 FOLD ITSELF.** Persisted verbatim at `reviews/PLAN_schedule_1a-r4fold-review.md`. Four
consecutive rounds on this branch have now found their Importants in the *previous* round's fold, and
three of these four are r4 resolutions landing one clause short of the thing they name. Every finding
below was re-verified against the primary source before folding; none was overturned.

| # | what the review claimed | what I verified, and how | what changed here |
|---|---|---|---|
| **I-1** | C-I2's mechanism (*transcribe each part's Caution*) closes **neither** case C-I2 names — Part V's Caution is an eligibility bar, so line 35 still prints $6,000 for a non-senior; Part I has no Caution and its predicate is line-scoped, so a part-scoped reading blanks **line 3** | **Confirmed, both halves.** `design/forms/extract/f1040s1a--2025.txt:97-98` is Part V's whole Caution — SSN and joint filing only, **no birth date**; the form puts *born before January 2, 1961* on lines 36a/36b (`:104-107`), which gate line 37, not the completion of 31–35. The condition is instructions-only (`design/forms/extract/i1040gi--2025.txt:44609-44617`). Part I (`design/forms/extract/f1040s1a--2025.txt:15-22`) prints **no Caution**, and the instructions' predicate (`design/forms/extract/i1040gi--2025.txt:43275-43285`) is scoped to lines 2a–2e. Form lines 8, 16, 25 and 31 each read line 3 | §T2 CORRECTION 4 rewritten: a per-part **source table**, the predicate scoped **per line**, and the two named cases spelled out. The C-I2 row records that its r4 resolution was superseded |
| **I-2** | B-I2's ✅ FIXED IN CODE is two-thirds landed — no constructor can set `LineCoverage.year`, and `Coverage::exception` still takes `Usd` | **Confirmed.** `crates/btctax-core/src/tax/line_coverage.rs` has exactly two constructors, `line` (`:137`) and `exception` (`:159`), and both write `year: DEFAULT_ROW_YEAR` (`:148`, `:170`) where that const is `"2024"` (`:49`). `exception`'s `_value` is `Usd` (`:161`); `line`'s is `impl Into<Option<Usd>>` (`:139`). `design/forms/extract/` holds `f1040s1a--2025.txt` and no `--2024`, and `crates/btctax-forms/forms/2024/` holds no `f1040s1a.map.toml` ⇒ the hard-error branch at `crates/xtask/src/line_coverage_check.rs:530-560`. The two halves that DID land are real (`:471-484`, `:883-888`) | The B-I2 row now reads **two-thirds fixed** and names both residues; §T2 item 8 carries them as work. ★ The **code** fix is the controller's, deliberately not this fold's — no `.rs` file is touched here |
| **I-3** | Part II holds two independently-claimable things and got one gate, so T3a refuses every W-2-only tipped employee and Part II is unreachable in B3 | **Confirmed.** The form separates them: 4a–4c are tips *received as an employee* (`design/forms/extract/f1040s1a--2025.txt:27-38`), line 5 is tips *received in the course of a trade or business* (`:39-42`). `schedule_c: Option<ScheduleCInputs>` (`crates/btctax-core/src/tax/return_inputs.rs:704`) is the only trade-or-business surface — no Schedule E, Schedule F or line 8z field exists. `other_out_of_scope_income` (`:994`) is always live (`crates/btctax-core/src/tax/questions.rs:597`) and refuses on `Some(true)` (`crates/btctax-core/src/tax/return_refuse.rs:995`), so the backstop the review names is real | T3a's lines 5 and 14b refuse on the **conjunction**, with the no-trade-or-business case recorded as genuinely blank, the backstop named, and the KAT that reds against the r4 wording written down |
| **I-4** | C-I3 restored the refinance cap and dropped the same paragraph's precondition (*your prior loan that had QPVLI*), laundering a pre-2025 vehicle into eligibility | **Confirmed, and it understates tax.** `design/forms/extract/i1040gi--2025.txt:44486-44494` opens the *Refinanced loan* paragraph with exactly that condition; requirement 1 sits at `:44447-44448` and requirement 3 at `:44450-44452`, and a refinance satisfies neither on its own terms — which is why the paragraph exists. A 2023 vehicle refinanced in 2025 answers *yes* to every YES-condition the r4 row collects | §T3's r4 CONFORMANCE (1) gains a **second** YES-condition defaulting to NO, plus one sentence fixing which loan requirement 1 binds |

**The four Minors and the Nit, all folded.**

- **M-1** — C-I1's resolution sits across a crate boundary. Verified: `label_reader` is
  `crates/xtask/src/label_reader.rs`; `crates/xtask/Cargo.toml:20` depends on `btctax-core` and
  `crates/btctax-core/Cargo.toml` names no `xtask`, so the dependency is one-way. ⇒ §T2 item 5 **says
  which**: the KAT moves to `xtask`, the struct stays in `btctax-core`.
- **M-2** — the KAT has four halves and B1 wants a planted defect per half. ⇒ §T2 item 6 names the four.
- **M-3** — rule (4b) cannot reach Schedule 1-A during B3. Verified:
  `grep -rnE 'Schedule1a|schedule_1a|f1040s1a' crates/btctax-forms/src/ crates/btctax-forms/forms/`
  returns **0**, and §3 puts the emitter in B4. ⇒ §T2 item 7 says the KAT is the only guard in B3.
- **M-4** — B-I5's second half was deferred into a §T2 that had no Form 4137 item to receive it. ⇒
  **answered in place** in T3a's 4b row instead of creating a task: the form-directed `-0-` needs no
  companion declaration, and the `box8_allocated_tips` refusal is a guard for a different defect.
- **N-1** — four stale citations in the fold's own new text; every underlying **claim** still checks
  out, only the addresses had decayed. Corrected, and ★ the sweep was widened to **every** `file:line`
  in the document because a command can resolve them all at once:

  | cited | actual |
  |---|---|
  | `return_inputs.rs:626-652` | `crates/btctax-core/src/tax/return_inputs.rs:960-1010` |
  | `return_inputs.rs:417-423` | `crates/btctax-core/src/tax/return_inputs.rs:693-704` |
  | `return_refuse.rs:769` | `crates/btctax-core/src/tax/return_refuse.rs:1182` |
  | `questions.rs:546-552` | `crates/btctax-core/src/tax/questions.rs:810-816` |
  | `tables.rs:1295-1313` | `crates/btctax-core/src/tax/tables.rs:1360-1379` |
  | `return_1040.rs:1269` | `crates/btctax-core/src/tax/return_1040.rs:1790` |
  | `return_1040.rs:1432` | `crates/btctax-core/src/tax/return_1040.rs:1809` |
  | `printed.rs:384` | `crates/btctax-core/src/tax/printed.rs:459` |
  | `printed.rs:384-387` | `crates/btctax-core/src/tax/printed.rs:435-440` |
  | `questions.rs:985-986` | `crates/btctax-core/src/tax/questions.rs:1386-1387` |
  | `coverage.rs`'s **80-row** `EXPECTED_LEAF_PATHS` | `crates/btctax-input-form/src/spec/coverage.rs:493-662`, **93 rows** |

  `crates/btctax-core/src/conventions.rs:28` and the `i1040gi` refinance address were checked and are
  correct. Bare filenames now
  carry their crate path wherever a line number is attached, so `sed -n '<line>p' <file>` resolves.

★ **Nothing was overturned, and nothing was widened.** No task was added, no form entered scope, and
the one place the review offered a choice (M-1's KAT placement) is answered rather than deferred.

**Branch:** `feat/amt-e2-vector-population` (current; B1 + B2 already on it).

---

## 0. What this plan is, and what it deliberately is not

**It is a sequencing document.** The transcription itself lives in the code, per this project's standing
rule: *one field per numbered line, in the form's own numbering, carrying the official instruction text
verbatim as its doc comment.* This plan does **not** restate the 48 lines — restating them here would
create a second, unexecutable copy that can drift from both the form and the struct. The extracted text
layer is the source: `f1040s1a--2025.pdf` (`64f97b38`) and `i1040gi--2025.pdf` (`482e9c48`) pp. 101-110,
both archived in `design/amt-form6251/`.

**Consequently the review gate on this work is mechanical** (CLAUDE.md): *is every line present, and
does each doc comment match the instruction text?* That is a test, and T2 writes it.

**★ This plan does NOT delete `ty2025_full_return_must_stay_fail_closed_until_complete`.** B3 satisfies
the gate's **condition 4** only. Conditions 2 and 3 landed in B2; condition 1 (the `FullReturnParams`
themselves) is the LAST thing to land, after B4. Until then Schedule 1-A is fully built and fully tested
against synthetic params and reaches no filed return.

---

## 1. Exit criteria (the definition of green for B3)

1. The **five gates**: `make check` · `cargo fmt --all --check` · `cargo +1.88 check --workspace
   --locked` · `cargo run -p xtask -- check-isolation` · `bash scripts/pii-scan-generic.sh`.
2. **All 48 line LABELS present** — not 38, which is merely the highest line *number* (r1 I-1).
   Asserted by a closed-at-both-ends KAT (T2), plus all four worksheets.
3. **Every part's phase-out tested at its own knee in its own direction** (S-1), including the recon's
   worked examples (b) $2,300 and (c) $5,000 — figures that differ under the wrong rounding.
4. **TY2024 provably unmoved**: golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47`, both oracles
   exit 0.
5. **Mutation-verified**, per guard. A guard whose mutation survives is not a guard.
6. **TY2029+ fails closed**, mutation-verified.

---

## 2. Task sequence

Each task is test-first and lands green. Tasks T1-T2 are the chokepoints — T3 onward all depend on the
shapes they fix, so they are done first and reviewed before the surface work fans out.

### T1 — the per-year table, with rounding as a parameter

`crates/btctax-core/src/tax/tables.rs`.

A `Schedule1aParams` carried per year for **2025-2028 only** (S-7: nothing is indexed, all four
provisions expire after TY2028, so a Rev. Proc. lookup is not merely unnecessary but wrong).

**The three things that must not be shared:**

- **Rounding direction is an explicit argument, never baked in** (S-1). Parts II/III **floor** (lines 11,
  19: *"decrease the result to the next lower whole number"*); Part IV **ceils** (line 28: *"increase the
  result to the next higher whole number"*). A `phase_out(excess, per_step, step)` helper with one
  direction is silently wrong on one side, by exactly $100 and $200 — which is precisely what worked
  examples (b) and (c) measure.
- **Three distinct threshold pairs** (spec F-4): $150,000/$300,000 (lines 9, 17), $100,000/$200,000
  (line 26), $75,000/$150,000 (line 32). No `threshold_for(status)`.
- **Three distinct caps**: $25,000 tips (line 7, per-return regardless of status — S-3), $12,500/$25,000
  MFJ overtime (line 15), $10,000 QPVLI (line 24).

★ **A THIRD rounding site, easily missed because the form states no direction for it** (r1 M-1). Line 34
(*"Multiply line 33 by 6% (0.06)"*) is its own printed dollar line, so the general IRS whole-dollar
convention governs it (`round_dollar`, `MidpointAwayFromZero`, `crates/btctax-core/src/conventions.rs:28`) and line 35 subtracts
the **printed** line 34. `6,000 − round(0.06 × L33)` and `round(6,000 − 0.06 × L33)` differ by $1 whenever
`0.06 × L33` lands on a half-dollar (excess ≡ 25 mod 50, e.g. $50,025 → $3,001.50) — **doubled** on a
two-senior MFJ return, and the "round the difference" form is the one that understates tax. Round the
line, then subtract; that is the project's standing rule that every line takes that schedule's *printed*
figure.

**Tests.** Each constant against the extracted line text. `TY2029+` returns `None` and a mutation that
extends the table reds.

★★ **The exhaustion identity is PER DIRECTION — a single closed form is false for Part IV** (r1 I-2, and
the plan's own r1 asserted the false one). Because Part IV **ceils**, it exhausts one dollar past the
last full step, not at the round number:

| part | direction | exhausts at |
|---|---|---|
| II tips | floor | `threshold + (cap/per_step) × step` = `+$250,000` |
| III overtime | floor | `+$125,000` (`+$250,000` MFJ) |
| **IV car loan** | **ceil** | `threshold + (cap/per_step − 1) × step + 1` = **`+$49,001`**, NOT `+$50,000` |
| V seniors | smooth | `+$100,000` (S-4) |

Check the arithmetic once, here, so nobody re-derives it: at excess $49,000 line 28 = 49 → line 29 =
$9,800 → line 30 = **$200**; at $49,001 line 28 = 50 → line 30 = $0. ★ **The assertion that actually
distinguishes the two directions is the paired one** — "at the knee, $0" *and* "one step below the knee,
still > $0". A knee-only test passes under both roundings, which is exactly the failure S-1 warns about.

### T2 — the struct: 48 line labels (52 leaves) + four worksheets, and the conformance KAT

`crates/btctax-core/src/tax/schedule_1a.rs` (new).

One field per line LABEL, named for it (`line4a`, `line36b`, …), instruction text verbatim as the
doc comment. Sub-structs per part keep the names short without renumbering.

**The four worksheets are their own transcribed types**, not `min()` calls in the emitter (spec F-2, and
OQ-2's closure): *Qualified Tips From More Than One Employer*, *Multiple Trades or Businesses*,
*Qualified Overtime Compensation From More Than One **Employer***, and *… From More Than One **Payor***.
The last two are distinct forms of the same idea (W-2 side vs 1099 side) and the r1 branch list
collapsed them.

★★ **THE LINE-5 CEILING IS A PROPERTY OF LINE 5, NOT OF THE WORKSHEET** (r1 **C-1** — the highest-value
finding in the round, and my r1 got it wrong in the direction that understates tax).

The ceiling is *not* net profit. It is net profit (Schedule C line 31 / the total of Schedule E lines
28(g) through 28(k) / Schedule F line 34) **minus** the deductible part of self-employment tax, the
deduction for contributions to self-employed SEP/SIMPLE/qualified plans, and the self-employed health
insurance deduction, **floored at zero**, and expressly **not** reduced by the qualified-tips deduction
itself (which is what keeps it acyclic).

★ **And it applies when there is exactly ONE business, which is btctax's only reachable case.**
`ReturnInputs::schedule_c` is `Option<ScheduleCInputs>` — singular — so the multi-business branch cannot
be reached today and the single-business branch is the whole surface. My r1 put the reduction *only* in
the *Multiple Trades or Businesses* worksheet, which the instructions make conditional (*"If … more than
one trade or business, complete the …Worksheet"*) while the ceiling itself is not. The instructions give
the one-business case outright:

> "For example, a sole proprietor who **only has one business** … The net income limitation will be the
> net profit shown on the Schedule C for the business, **less the amount from Schedule 1, line 15**. The
> sole proprietor would include on line 5 of Schedule 1-A the lesser of (i) the qualified tips received
> in the business, or (ii) the net profit for the business less the amount from Schedule 1, line 15. **If
> the business shows a net loss on Schedule C, then the sole proprietor would not include any qualified
> tips** received in the business on line 5.

`min(tips, net_profit)` would overstate the ceiling by half the SE tax → larger lines 6/7/13/38 →
**understates tax**. No new input is needed: printed Schedule 1 line 15 already exists
(`crates/btctax-core/src/tax/printed.rs:459`, `p.half_se_15` — address re-measured in the r5 fold). So **the ceiling lives on line 5, and the worksheet is the multi-row
aggregation of that same ceiling** — its only home was the defect.

★ **A tension to adjudicate in the code comment, not rediscover.** The two numbered *Examples* that
follow compute a limitation of $4,500 (gross $5,000 − expenses $500) and $1,000 (gross $15,000 −
expenses $14,000) **without** subtracting Schedule 1 line 15. The narrative states the rule; the examples
illustrate an outcome. **The narrative governs** — and it is also the fail-closed direction (lower
ceiling ⇒ smaller deduction ⇒ higher tax). Cite both in the doc comment so a future reviewer or oracle
pointing at the examples meets the adjudication instead of re-deriving it.

★ **LINE 22's ARITY IS A T2 DECISION, NOT A B4 ONE** (r1 M-2). The form has rows **a and b only** —
*"If more than two VINs, see instructions"*, and the instructions say *"attach a statement to your return
showing the information required on line 22."* Line 23 adds *"lines 22a and 22b, column (iii)"*. T2 is a
declared chokepoint, so this cannot be discovered in B4: decide now between a fixed 2-row structure that
**refuses** above two vehicles and an N-row structure whose rows 3+ route to a statement, and pin the
choice with a KAT. Capping silently at two drops a third vehicle's interest (overstates tax); summing
three into two rows yields a correct line 23 while omitting a VIN the deduction is conditioned on.

★★ **THE EXPECTED SET COMES FROM BOTH EXTRACTS, NOT THE FORM ALONE** (census **F-4** — a hole in this
task's own approach, found by measurement). `grep -c "Keep for Your Records"` on the **form** extract is
**0**: the four worksheets exist only in the *instructions* extract. So a label census driven off the form
fixture — which is what r2 specified — **could never red on a worksheet omission.** It would have passed by
finding nothing, which is the exact false-completeness this plan warns about everywhere else.

**The conformance KAT** — the mechanical gate, executed:

- ★ **all 48 line LABELS are fields** (r1 I-1). 38 is the highest line *number*, not the count. The
  lettered runs are `2a-2e`, `4a-4c`, `14a-14c`, `22a-22b`, `36a-36b`, and **neither line 4 nor line 14
  has a bare amount box** — each is a heading for its lettered sub-lines. Per part: I 7, II 12, III 10,
  IV 10, V 8, VI 1 = **48**; and **52 data leaves** once line 22's three columns ((i) VIN, (ii) deducted
  on Schedule C/E/F, (iii) Schedule 1-A) are counted. ★ **The expected set is ENUMERATED FROM THE
  EXTRACT, never from a range** — a `BTreeSet` built from `1..=38` either reds on every lettered field as
  an unexpected extra, or the struct gets collapsed to match and a sub-line is lost. That is the exact
  defect CLAUDE.md records as shipped: *"Later drafts dropped Form 6251 line 2b."* Dropping 2b or 2c
  under-adds MAGI ⇒ phase-outs too small ⇒ understates tax; dropping 22 column (ii) lets the same
  interest be deducted twice.
- the list is **closed at both ends** via a `BTreeSet` comparison so neither a missing line nor an
  unexpected extra passes (the pattern the Form 6251 KAT settled on);
- each field's doc comment contains the line's own instruction text, checked against a committed
  extract of the text layer, so a paraphrase reds.

★★ **r4 CORRECTIONS TO THIS KAT — four, and the first two are what make it a conformance check.**

1. **Half (b) is missing.** The bullets above are (a), (a) and quotation. CLAUDE.md is explicit that a checker
   which cannot distinguish *this line encodes no decision* from *we forgot this line* is not a
   conformance check. As written, a field declared, doc-commented and **never assigned by T4** passes. ⇒ every
   line additionally carries a `Production` from `tax/line_coverage.rs` — `Collected`, `Carry`,
   `Combine`, `Clamped(Polarity)`, `Scaled`, `Bounded`, `Constant`, or an `Exception` **with a written
   reason**, ratcheted. Schedule 1-A maps onto it almost exactly; lines 10/18/27/33 are `Exception`s
   (conditional *jumps*, not clamps — the class the ratchet already records for `f1040:34`).
2. **The doc-comment check must be PER LINE, not per document.** *"Checked against a committed
   extract"* is `cite-check` semantics, which proves a quotation is **the form's** words, not **that
   line's**. Line 28's rounding sentence sitting on line 11 passes — and that swap *inverts the
   rounding for Parts II/III*. ★ `crates/btctax-core/src/tax/tables.rs:1360-1379` already has `printed_line(label)`, built by **T1
   in this same plan**, whose own doc says in terms that the fix is **not** more citation checking Use it:
   `printed_line(<the field's label>)` ∋ the quoted instruction, whitespace-normalized.
3. **The expected set comes from `label_reader`, not a fresh parse.** The text layer yields **50**
   labels; lines 4 and 22 are headings that carry instruction text and no amount box, and the layer
   *cannot say which* — `label_reader.rs`'s own doc says distinguishing a heading from a label
   means knowing whether the line has an amount box — **which the text layer does not directly say**. It
   resolves this with two witnesses and asserts 50 / 48 / `["4","22"]`. Assert against that. ★ The
   plan's gloss elsewhere names line **14** as the second box-less heading — that is wrong; 14a is a
   real entry line, and the two headings are **4 and 22**. The per-part counts (7+12+10+10+8+1 = 48 entry lines) are right.
4. **Completion conditions are part of conformance — and the SOURCE IS NAMED PER PART, the PREDICATE
   SCOPED PER LINE** (r4 C-I2, as corrected by r5 I-1). The r4 wording said *each part's Caution*, and
   that closes **neither** of the two cases C-I2 names, because only Parts II, III and IV print a
   completion condition in their Caution:

   | part | completion source | scope of the predicate |
   |---|---|---|
   | I | the instructions (`design/forms/extract/i1040gi--2025.txt:43275-43285`) — the form prints **no Caution** | **lines 2a–2e only**; lines 1 and 3 are ALWAYS entered |
   | II | the form's Caution (`design/forms/extract/f1040s1a--2025.txt:24-26`) | the part |
   | III | the form's Caution (`design/forms/extract/f1040s1a--2025.txt:53-55`) | the part |
   | IV | the form's Caution (`design/forms/extract/f1040s1a--2025.txt:73-74`) | the part |
   | V | the instructions (`design/forms/extract/i1040gi--2025.txt:44609-44617`) — the Caution omits the birth date | the part |
   | VI | none printed | unconditional |

   ★★ **Part V's Caution is an ELIGIBILITY BAR, not a completion predicate.** In full it reads
   *"You and/or your spouse must have a valid social security number. If married, you must file jointly
   to claim this deduction."* — no birth date anywhere in it. The completion condition is
   instructions-only: *"Fill out Schedule 1-A, Part V, only if:"* … *"You (and/or your spouse if filing
   a joint return) were born before January 2, 1961."* Transcribe the Caution alone and a non-senior
   single filer with a valid SSN "completes" Part V, lines 31–35 are computed, and **line 35 still
   prints $6,000 for a non-senior** — verbatim the case C-I2 exists to close, with the KAT green and
   blind to it. (Line 37 is still `$0`, because *"were born before January 2, 1961"* gates 36a/36b on
   the form itself — which is why this is a fabricated-testimony defect, §G-11's class, and not a wrong
   figure.)

   ★★ **Part I's predicate is LINE-SCOPED, and a part-scoped reading blanks the MAGI.** The
   instructions say *"If you don't have income from Puerto Rico that you excluded from your income, or
   you aren't filing Form 2555 or 4563, then enter the amount from Form 1040, 1040-SR, or 1040-NR,
   line 11b, on Schedule 1-A, line 3. If you do have excluded income from Puerto Rico, or you are
   filing Form 2555 or 4563, complete lines 2a through 2e in Part I of Schedule 1-A to figure your
   MAGI."* So the common filer's correct Part I is **1 and 3 entered, 2a–2e blank**. There is no
   Caution to transcribe, and an implementer who invents a part-level predicate for Part I blanks
   **line 3** — the MAGI that lines 8, 16, 25 and 31 each read — while one who backs off prints `$0` on
   2a–2e, which is the defect C-I2 filed. Both exits are wrong.

   ⇒ completion is expressible at **line** granularity inside a part, and the KAT asserts a per-LINE
   completion set, not *a part's lines*. This is a T2 struct-shape decision because B4 cannot add
   line-granular completion later, which is the same reason r4 gave for the coarser version.

★★ **r5 ADDITIONS TO THIS TASK — where the KAT lives, what it must red on, and two `line_coverage`
residues.**

5. **The conformance KAT is SPLIT ALONG THE CRATE LINE — membership in `crates/xtask/`, quotation
   and worksheets in `btctax-core`** (r5 M-1, corrected by r6 I-1 and I-2). ★★ The r5 fold moved the
   whole KAT to `xtask` and did not re-check what it could still reach from there. Two things it
   cannot:
   - **`printed_line` does not exist outside `btctax-core`'s own test build.** It is
     `crates/btctax-core/src/tax/tables.rs:1360`, a bare `fn` inside the `#[cfg(test)]` module that
     opens at `:1353` — so when `btctax-core` is compiled as `xtask`'s *dependency* the item is not
     generated at all. CORRECTION 2 orders half 2 of the KAT to call it, so half 2 **and its B1
     kill would not compile**. Verified 2026-09-05.
   - **The four worksheets have no mechanical source in `xtask`.** CORRECTION 3 drives the expected
     set from `label_reader`, whose box witness needs an AcroForm; the instructions PDF has none and
     there is no `design/forms/geometry/i1040gi--2025.json`. That is census F-4's own measured
     blindness, not a gap to paper over.
   ⇒ **Membership** (the 48 form labels, from `label_reader`) stays in `crates/xtask/`.
   **Per-line quotation and the four worksheets** go in `btctax-core`'s existing
   `schedule_1a_conformance` module, where `printed_line` and the in-crate fixture already live —
   `crates/btctax-core/src/tax/fixtures/schedule_1a_2025_instructions.txt` carries all four worksheet
   headers, verified at `:427`, `:556`, `:1049`, `:1066`. No new code and no new fixture is required
   by this split; it is a placement decision, not work.
   ★ The struct stays in `btctax-core` either way, and B-I1's `(label, got.lineN)` tuple form still
   ties the compiler to it from both sides.

   *(Superseded r5 reasoning, kept because the crate-direction argument still holds and is why
   membership stays in `xtask`:)* `label_reader` is `crates/xtask/src/label_reader.rs`, and `xtask`
   depends on `btctax-core` (`crates/xtask/Cargo.toml:20`) while `btctax-core`'s manifest names no
   `xtask` — the reverse would be a cycle. It also keeps `label_reader`'s repo-root fixture loading
   out of `btctax-core`, whose tests deliberately use in-crate fixtures — an escaping `include_str!`
   has shipped a broken tarball from this repo before.

   *(Original r5 text follows.)* **The conformance KAT lives in `crates/xtask/`, not in `btctax-core`** (r5 M-1 — *say which*).
   `label_reader` is `crates/xtask/src/label_reader.rs`, and `xtask` depends on `btctax-core`
   (`crates/xtask/Cargo.toml:20`) while `btctax-core`'s manifest names no `xtask` — the reverse would
   be a cycle. CORRECTION 3 drives the expected set from `label_reader`, which is therefore unreachable
   from `crates/btctax-core/src/tax/schedule_1a.rs` where r4 put both. ★ The **struct stays in
   `btctax-core`**; only the KAT moves. B-I1's `(label, got.lineN)` tuple form still ties the compiler
   to the struct, because `xtask` can see `btctax_core::tax::schedule_1a::*`. It also keeps
   `label_reader`'s repo-root fixture loading out of `btctax-core`, whose tests deliberately use
   in-crate fixtures — an escaping `include_str!` has shipped a broken tarball from this repo before.
6. **B1: this KAT has FOUR halves, and each needs its own planted defect** (r5 M-2). Exit criterion 5's
   generic *mutation-verify every guard* is not enough here, because CLAUDE.md B1 is scoped by name to
   a conformance check and a mutation that kills one half leaves the other three green. Name and plant:
   1. **membership** — drop a line from the struct;
   2. **per-line quotation** — move line 28's *"increase the result to the next higher whole number"*
      onto line 11 (the swap `printed_line` exists to catch, and the one that inverts the rounding for
      Parts II/III);
   3. **provenance** — declare a field and never assign it;
   4. **completion** — complete a part whose predicate is false (CORRECTION 4's Part V case: a
      non-senior reaching line 35).
7. **Rule (4b) is STRUCTURALLY BLIND to Schedule 1-A for the whole of B3** (r5 M-3), so during B3 the
   `Production` requirement is held by **this KAT alone**. B-I2's fix replaced a three-file hand-list
   with a derived predicate — a type is in scope iff the emitter crate names it in real code — and §3
   puts the emitter and the AcroForm map in **B4**, so
   `grep -rnE 'Schedule1a|schedule_1a|f1040s1a' crates/btctax-forms/src/ crates/btctax-forms/forms/`
   returns **0** and the checker contributes zero Schedule 1-A rows while reporting OK. Nobody may read
   the B-I2 row as cover for a checker that structurally cannot see this form yet.
8. **Two `line_coverage` residues land here, because T2 is where the first TY2025 rows exist** (r5 I-2):
   - `LineCoverage.year` is per-row (`crates/btctax-core/src/tax/line_coverage.rs:106`) and **no
     constructor can set it**. `Coverage::line` (`:137`) and `Coverage::exception` (`:159`) are the only
     two, and both write `year: DEFAULT_ROW_YEAR` (`:148`, `:170`), which is `"2024"` (`:49`) — even
     though that const's own doc says a TY2025+ form sets its own. A Schedule 1-A row built the ordinary
     way therefore resolves to stem `f1040s1a--2024`; that extract does not exist and neither does
     `crates/btctax-forms/forms/2024/f1040s1a.map.toml`, so the checker takes its **hard-error** branch
     (`crates/xtask/src/line_coverage_check.rs:530-560`) and reports that the form name is wrong —
     misdiagnosing itself. Add a year-carrying path.
   - `Coverage::exception` takes `_value: Usd` (`crates/btctax-core/src/tax/line_coverage.rs:161`) while
     `Coverage::line` takes `impl Into<Option<Usd>>` (`:139`). Lines **10/18/27/33** are `Exception`s
     (CORRECTION 1) *and* must express *not completed* (CORRECTION 4), so as it stands an implementer
     writes `.unwrap_or(Usd::ZERO)` — which is precisely what `line`'s own doc says its widening exists
     to prevent. Widen `exception` the same way.

### T3 — the input surface, landed whole

~25 leaves plus **≈19 declarations** (r4 M-1 recount: Part II 5, Part III 3, **Part IV 9 per
vehicle**, plus the SSN bar per person for II/III/V — the header said six), through the whole stack in
one pass (the G-9 walk, since the user
has directed that the input surface not lag the core): `return_inputs.rs` → `classifier.rs` →
`questions.rs` → `return_refuse.rs` → `input-form` `seam/registries/coverage/sections` → CLI `answer.rs`
→ TUI. The exhaustive matches and the coverage KAT force every site; nothing here is found by grep.

**The declarations, and why each is a declaration rather than a derived value.** ★★ r1 found this table
**incomplete in the dangerous direction, twice** (C-2 and I-3): it carried the *derived* questions and
dropped the *gating* ones, so a filer who merely typed a dollar figure got the deduction. Both Criticals
in the round were missing eligibility, not wrong arithmetic — the defect class CLAUDE.md names.

**Part II — tips.** ★ The FIRST condition is printed on the form itself and r1 had no row for it:

> **Caution:** Fill out Part II only if you received qualified tips. **These tips must have been received
> in an occupation listed at IRS.gov/TippedOccupations.**

| declaration | why btctax cannot answer it |
|---|---|
| ★ **occupation is on the Treasury list** (record the code) | The gating condition. *"In order for a tip to be a qualified tip, it must have been paid to you while you were working in an occupation that customarily and regularly received tips on or before December 31, 2024."* Spec §3 already says "the filer answers; we record the code" — r1 collected it nowhere. Defaults to **NO**. |
| ★ **multi-occupation carve-out** | *"If you received tips as an employee in more than one occupation for the same employer, only those tips that were received in an occupation on the list … are considered qualified tips. **Do not include tips received in occupations that are not included on this list in line 4a, 4b, or 4c.**"* |
| ★ **the qualified-tip amount criteria** | Cash medium, paid voluntarily, not negotiated, customer-determined — and *"Qualified tips do not include service charges, automatic gratuities, or any other mandatory amounts automatically added to a customer's bill"* (the instructions' own example: an 18% automatic gratuity *"is not a qualified tip and may not be deducted"*), nor *"Event tickets, Meals, Services"*. These ARE the criteria the "⊆ box 7" prompt needs; without them the prompt asks for a subset and offers no way to compute it. |
| SSTB tips | §224(d)(3), **as relaxed by Notice 2025-69** (OQ-3). ★ Nearly *subsumed* by the occupation gate — the relief answers SSTB **from** the occupation list — which is why carrying this row while dropping the gate was backwards. |
| qualified tips ⊆ W-2 box 7 | Spec **F-1**: the 2025 W-2/1099s *"were not updated to separately identify tips that may qualify"*. Box 7 is a starting point, not the figure. |

**Part III — overtime.** The three §4.1 traps, unchanged from r1: the FLSA **premium half** only (not
double-time's second half, not holiday/weekend premiums absent >40 hours); **excludes any amount received
as a qualified tip** (no double-dip with Part II — the surface must refuse the same dollars twice, not
silently allow them); and **state-law-only overtime for FLSA-ineligible employees does not qualify** (the
entitlement must arise under FLSA §7).

**Part IV — car loan interest.** ★★ r1 had **no eligibility declarations at all** for this part (C-2):
only the cap, the threshold, the ceil, and a VIN deferred to B4. So every filer who typed a car-loan
interest figure got up to **$10,000** of deduction — including on a lease, a used car, a
non-US-assembled car, ~~a refinance~~, a pre-2025 loan, or negative equity. **Understates tax.** The
instructions state the conditions flatly, and none is derivable:

> "the interest must be paid or accrued on a loan that generally meets **all** the following
> requirements. 1. Your loan was originated **after December 31, 2024**. 2. The loan was originated **by
> you**. 3. The proceeds from your loan were used to **purchase** an APV (**lease payments do not
> qualify**). 4. Your APV is for **personal use** … 5. Your loan is secured by a **first lien** on the
> purchased APV."

> "In general, an APV is any vehicle that meets the following conditions: • The **original use** of the
> vehicle starts with you (**a used vehicle does not qualify**) … • The vehicle is a car, minivan, van,
> SUV, pickup truck, or motorcycle, and has a **gross vehicle weight rating of less than 14,000
> pounds**, and • The vehicle has undergone **final assembly in the United States**."

> "amounts representing debt on a vehicle traded in as part of the purchase transaction for the APV
> (so-called negative equity), **is not eligible** for the deduction."

Collected **per vehicle**, as YES-conditions defaulting to NO — the same treatment the overtime traps
get. This is S-5's own principle ("eligibility bars are part of the transcription") applied past the
filing-status bars, which is as far as r1 took it.

**Parts II/III/V — the valid-SSN bar, per person.** Spec F-3: *"valid for employment and … issued by the
Social Security Administration (SSA) before the due date"* — neither property is visible to btctax.
★ **Scoped to Parts II, III and V only** (r1 N-1): Part IV's caution carries no SSN requirement and its
instructions have no *Valid SSN* paragraph, so gating Part IV on it would deny a deduction §163(h)(4)
allows.

★ **Prompt wording is the deliverable here, not plumbing** (R-2). A wrong prompt is a wrong return that
every test passes. Each prompt states the condition that permits a *yes* and defaults to the answer that
cannot overstate the deduction — the structural lesson from
`widening-an-exemption-is-never-the-safe-edit`: enumerate the YES-conditions so every omission fails
closed.

Death of a taxpayer/spouse needs **no new collection**: §G-9 landed
`HouseholdHeader::{taxpayer,spouse}_died_during_year` and `Person::date_of_death`, with the
day-before-the-65th-birthday convention in `reaches_65_on`. Part V reuses them at $6,000 per person.

★★★ **r4 CONFORMANCE — TWO ELIGIBILITY CONDITIONS ARE MISSING, BOTH IN THE UNDERSTATEMENT DIRECTION.**
This is the third and fourth instance of the class r1 found twice; this plan already records that r1 found
the same table incomplete in the dangerous direction, **twice**.

**(1) Part IV — the refinance balance cap (r4 C-I3).** r1's C-2 named a refinance **beyond the prior
balance**; the fold reduced that to a bare refinance and kept no condition. Worse, the prose listed a
refinance among the things that DISQUALIFY. The instructions say the opposite, in one paragraph
(`design/forms/extract/i1040gi--2025.txt:44486-44494`):

> **Refinanced loan.** If your prior loan that had QPVLI is later refinanced, interest paid on the
> refinanced amount is **generally eligible** for the deduction, so long as the new loan is secured by
> a first lien on the APV with respect to which the refinanced loan was incurred. **The loan amount is
> limited to the outstanding balance of the refinanced loan as of the date of the refinancing.**

Without the cap, a cash-out refi deducts interest on the **entire** new balance — every YES-condition
the plan collects answers *yes* — up to $10,000 of interest on non-qualifying principal. ⇒ add a
per-vehicle row asking whether this is a refinancing and, if so, the outstanding balance of the
refinanced loan on the refinancing date — limiting the interest to that fraction; and correct the prose.

★★ **AND THE PARAGRAPH'S OPENING CONDITION IS PART OF THE RULE — r5 I-4, a FIFTH instance of the same
class, in the same paragraph as the fourth.** The r4 fold restored the cap and dropped the first eight
words of the sentence it quotes: *"If your prior loan that had QPVLI is later refinanced"*
(`design/forms/extract/i1040gi--2025.txt:44486-44494`). That clause is what the carve-in turns on. The
five general requirements (`design/forms/extract/i1040gi--2025.txt:44445-44458`) open with *"Your loan
was originated after December 31, 2024"* and *"The proceeds from your loan were used to purchase an APV"* — and a refinance's proceeds
repay a prior loan rather than purchase a vehicle, which is exactly **why** the *Refinanced loan*
paragraph exists. Its price is that the **prior** loan must itself have been a QPVLI loan.

Without that condition, a car bought in **2023** and refinanced in 2025 answers *yes* to every
YES-condition the row above collects — the new loan really was originated after 2024-12-31, and as the
filer reads it the loan really is on the car they purchased — and up to **$10,000** of interest on a
pre-2025 vehicle is deducted. **Understates tax**: same direction, same paragraph and same class as
C-I3 itself.

⇒ the refinance row carries a **second** YES-condition defaulting to NO — *the loan being refinanced
was itself a qualifying QPVLI loan*: it met requirements 1-5 when originated, or the filer became the
obligor under the death exception, which the instructions state as *"If a loan met requirements 1
through 5 at the time it was originated by a previous obligor"*. ★ And one sentence in the prompt
stating that **for a refinance, requirement 1 binds the PRIOR loan**; testing the new loan's
origination date is vacuous, because a refinancing is by construction later than the loan it
refinances, so that gate can never fail and proves nothing.

**(2) Part II — the illegal-service exclusion (r4 C-I4).** The *"Amounts received that are not
qualified tips"* list has **three** bullets and the table carries one (SSTB). Missing: tips for a
service that is a felony or misdemeanour, and amounts for prostitution or pornographic activity.
★ The occupation gate **cannot** answer this, and the IRS proves it with a matched pair — a bartender
who served alcohol unlawfully has **non**-qualified tips, while a server working legally has qualified
ones. ⇒ one more YES-condition defaulting to NO, worded on that distinction (the *service* must be
legal; the employer's unrelated violations do not disqualify), citing both examples in the prompt.

★ Also carried (r4 N-1): loan requirement 2's *"change in obligor by reason of previous obligor's
death"* exception (an heir assuming a qualifying loan **does** qualify — omitting it fails closed, so
it is a Minor), and the *personal use* operative test (*"more than 50% of the time"*), which the plan
collects as a bare declaration and the instructions define.

### T3a — ★★ THREE LINES HAVE NO INPUT PATH AT ALL, AND MUST NOT PRINT ZERO

Census **F-1**, confirmed against source. `ReturnInputs` carries `w2s`, `int_1099`, `div_1099`, `g_1099`, `b_1099`
and **no other information-return struct** (`crates/btctax-core/src/tax/return_inputs.rs:693-704` — r5
N-1 corrected the address, and `b_1099` had joined the list since; `grep` for a 1099-NEC, 1099-MISC or
1099-K struct over `crates/*/src/` still returns nothing). But the form reads:

- **line 5** — *"Qualified tip amount included in Form 1099-NEC, box 1; Form 1099-MISC, box 3; or Form
  1099-K, box 1a."*
- **line 14b** — *"Qualified overtime compensation included in Form 1099-NEC, box 1, or Form 1099-MISC,
  box 3."*

**There is no 1099-NEC, 1099-MISC or 1099-K struct anywhere in the input model.** So both lines would be
blank *because nothing can populate them* — permanently, and indistinguishably on the page from a filer who
truly had no such income. Under `FOLLOWUPS.md` **§G-11** that is not a gap but **fabricated testimony**: a
printed `0` is an affirmative sworn statement that the amount IS zero.

★ **btctax has exactly three lawful moves here — collect, refuse, or genuinely blank — and "silently zero"
is none of them.** For B3, choose per line and record the reason:

| line | move | why |
|---|---|---|
| 4b | **form-directed `-0-`**, with the reason recorded | *"If Form 4137 is not filed, enter -0-."* btctax emits no Form 4137, so that condition is **true of every return it produces** — this is the form's own conditional constant, not a guess. ★ But the existing guard is PARTIAL: `crates/btctax-core/src/tax/return_refuse.rs:1182` refuses on `w2.box8_allocated_tips > Usd::ZERO` (*allocated* tips), while Form 4137 is also required for tips the employee did not report to the employer, which `W2` cannot see. (r4 buildability I-5; address corrected r5 N-1.) ★ **Answered here rather than deferred to T2** (r5 M-4, which found the deferral had no landing place and no owner): the `-0-` needs **no companion declaration**, because the form's condition is *"If Form 4137 is not filed, enter -0-"* and btctax emits no Form 4137 on any return — that is a fact about our own output, not a claim about the filer, and no declaration could make it more or less true. It follows that the `box8_allocated_tips` refusal is **not** the guard for line 4b at all; it guards a different defect (a return that ought to have carried a Form 4137), and it is partial in exactly the way the buildability lens says. That partiality shrinks line 4b, and line 4c takes the *larger* of 4a/4b, so the direction is fail-closed — record the partiality in the code comment and file the unreported-tips half alongside the 1099 surface below; do not add an input for it in B3. |
| 5 | **REFUSE** when the Part II **claim gate** is `Some(true)` **AND** `schedule_c.is_some()` | ★★ **NOT `schedule_c.is_some()` alone** (r4 buildability I-4) — and ★★ **not the claim gate alone either** (r5 I-3). It is the **conjunction**. A Schedule C **is the mining household** — btctax's core case — and Part II's own Caution makes the deduction opt-in: *"Fill out Part II only if you received qualified tips."* Refusing on the presence of a Schedule C would refuse **every** TY2025 mining return once the fail-closed gate comes out, on a part the filer was told not to complete. The tree already learned this: `crates/btctax-core/src/tax/questions.rs:810-816` records a class-(A) declaration that was always live and blocked **every** return btctax could compute, buying nothing (address corrected r5 N-1). ★★ **But Part II holds TWO independently-claimable things**, and gating this line on the part repeats that same lesson one notch narrower — it would block 100% of Part II's population instead of 100% of all returns, leaving Part II unreachable in B3 and unexercisable against T7's per-part census. Lines 4a–4c are tips *"received as an employee"*; line 5 is tips *"received in the course of a trade or business"*. A W-2-only tipped employee — a waiter, the form's own worked example — has no trade or business, so **line 5's own predicate is determinately false** and blank is the correct entry: nothing to collect, nothing to be uncertain about. `schedule_c: Option<ScheduleCInputs>` is the only trade-or-business surface btctax has (no Schedule E, no Schedule F, no Schedule 1 line 8z), so this is a fact the tool holds rather than a guess. Stated in this plan's own vocabulary: **the claim gate is per part, the refusal predicate is per line**, and Part II is the part where they differ. The backstop that keeps the blank honest is `other_out_of_scope_income`, which is always live and refuses on both `None` and `Some(true)`. **The KAT that holds this:** a W-2-only tipped employee with no Schedule C produces a Part II deduction and does **not** refuse — it reds against the r4 wording. |
| 14b | **REFUSE** when the Part III **claim gate** is `Some(true)` **AND** `schedule_c.is_some()`, else genuinely blank | Same correction, and the same **conjunction** (r5 I-3). ★ The old rationale — that with no Schedule C there is no payor relationship to report — is wrong on its own terms for the **1099-MISC box 3** half: that is Other Income on Schedule 1 line 8z, not Schedule C. ★ So note **which guard holds which half**: the conjunct is the live guard on the 1099-NEC box 1 side, because nonemployee compensation *is* trade-or-business income — with no Schedule C there is nothing on that side to claim; the box 3 side is held instead by `other_out_of_scope_income`, which a filer with no Schedule C must answer `Some(false)` for a return to be produced at all — so they have affirmatively sworn there was no such income, and 14b is genuinely blank rather than laundered. |

**Collecting the 1099 surface is the right long-term answer** (CLAUDE.md: *if the form asks something our
input surface cannot answer, collect it* — that is following instructions, not scope creep). It is out of
scope for B3 only because it is a new multi-form input surface with its own spec; file it, do not fake it.

★ **F-2 makes the line-5 ceiling un-implementable as specified, so it refuses rather than computing.**
Plan r2 folded "net profit − Schedule 1 line 15". The instructions require more: *"including the
deductible part of self-employment tax; the deduction for contributions to self-employed SEP, SIMPLE, and
qualified plans; and the self-employed health insurance deduction, but not including the deduction for
qualified tips."* Printed Schedule 1 Part II carries lines **15/18/21 only** (`crates/btctax-core/src/tax/printed.rs:435-440`) — no
SEP/SIMPLE field, no self-employed-health-insurance field, and (**F-3**) no Schedule E or Schedule F input
at all, which worksheet column (b) also reads. A ceiling built from what we have is structurally **too
high** ⇒ line 5 too large ⇒ **understates tax**. Computing it anyway would be the compression this
project's standing rule exists to forbid.

### T4 — compute, transcribed line by line

**The skip branches are the risk, not the arithmetic** (spec F-5):

- Lines **10, 18, 27** are the same routing three times, and each is quoted here as the form prints it
  rather than as one bracketed composite (a synthesized quotation is not a citation — `xtask cite-check`
  rejects it, which is how this one was caught): line 10 *"If zero or less, enter the amount from line 7
  on line 13"*, line 18 *"If zero or less, enter the amount from line 15 on line 21"*, line 27 *"If zero
  or less, enter the amount from line 24 on line 30"*. A jump **past** the phase-out, not a zero.
- Line **33**: *"If zero or less, **enter $6,000 on line 35**"* — a jump that writes a **nonzero
  constant** into a later line. Transcribing this as `-0-` yields **$0 instead of $6,000**: the whole
  senior deduction lost for every filer under the threshold, which is most of them. It happens to agree
  with `max(0, …)` only because 6% × 0 = 0, so a `max(0, …)` transcription passes for the wrong reason
  and breaks if the rate ever moves. Pin the branch itself.

Filing-status bars are transcribed, not inferred (S-5): Parts II, III and V print *"If married, you must
file jointly to claim this deduction"* ⇒ **zero for MFS**; Part IV prints no such caution ⇒ **allowed for
MFS**, which is adjudicated against the form over OTS 2025 (which bars all four — a witness, not the
authority).

### T5 — wiring, and the below-the-line invariant

`L38 → 1040 L13b` (the `AbsoluteReturn::schedule_1a_additional` seam B2 already threaded);
`L37 → Form 6251 line 1a` — the **senior subtotal**, not the total (parent D-3). File Schedule 1-A only
when `L38 > 0`.

★ **First, delete a comment that expires** (census **F-6**). `crates/btctax-core/src/tax/return_1040.rs:1790` is
`let schedule_1a_additional = Usd::ZERO;` under a comment reading *"the 2024 form has no such line, so zero is the RIGHT
value there, not a stub."* That is true for TY2024 and **false the moment TY2025 lands** — and a comment
cannot red. It is §G-11's shape in miniature: a correct blank and a laundered one sharing one code path.
T5 replaces it with the real line 38 and a per-year assertion that TY2024 still yields zero *because the
form has no such line*, not because a literal says so.

★★ **Spec §5.6b is the cheapest high-value guard in the whole branch, and it has TWO halves — r1 stated
only one** (r1 I-4). As written it listed the quantities that must **not** move and omitted the ones that
**must**, which invited a reading that would have *required* a bug: `ti_before_qbi` is derived from AGI
and so looks AGI-keyed, and a mechanical "every AGI-keyed quantity is byte-identical" would mandate
dropping the 13b term.

| | quantity | with vs without a Schedule 1-A deduction |
|---|---|---|
| **must MOVE**, by exactly L38 | `1040 L15`; **Form 8995 line 11** (`ti_before_qbi = agi − deduction − schedule_1a_additional`, `crates/btctax-core/src/tax/return_1040.rs:1809`) | changes |
| **must NOT move** | Form 8960's NIIT MAGI · Schedule A's 7.5% medical floor · the §164(b) SALT phase-down MAGI · the IRA/student-loan phase-outs | byte-identical |

Direction, from B2's own note: omitting 13b **overstates** `ti_before_qbi`, inflating the §199A deduction
→ **understates tax**, and it fires `qbi_over_threshold` too early.

★ **This is a debt B2 explicitly booked to B3.** `form_1040_line14_sums_12e_13a_and_13b_while_form_8995_
line11_excludes_only_12e_and_13b` carries a doc comment saying it is *"DELIBERATELY WEAK, and saying so is
the point … `assemble_absolute` hardcodes a zero 13b until Schedule 1-A lands (B3), so both assertions
below are trivial identities … It records the SHAPE so B3 has something to make real."* T5 makes it real
with a nonzero 13b and mutation-verifies it. (For accuracy: the composition is *not* currently unguarded —
the same comment names `printed::tests::printed_1040_line14_needs_all_three_terms` as the load-bearing
version that does die to the mutation. What T5 discharges is the *semantic* test's vacuity, and the
must-move/must-not-move split.)

### T6 — tests

Beyond each task's own: the recon's four worked examples ((b) $2,300, (c) $5,000, (d) two-senior MFJ at
MAGI $200,000 ⇒ L37 $6,000 / $3,000 each, proving S-4's 12¢-per-$1 aggregate slope); all five filing
statuses (S-3's per-return caps and S-5's MFS bar are status-dependent); `L38 > 0` gates filing; `L37`
(not `L38`) reaches Form 6251 line 1a.

**Mutation-verify every guard.** Two of the previous session's guards were vacuous until a mutation said
so, both because every term in them was zero — so for each guard, ask first *which real input axis it
cannot express* (the parametrization lesson from the AMT sweep, where pinning the base standard deduction
hid the §63(f) add-back direction entirely).

### T7 — the two-oracle census, per part

Disqualifications **computed and sized**, never a name list (the shape `verify_f6251.py` converged on).
Known going in: **OTS 2025's Part IV is defective three ways** and **taxcalc has the wrong QSS
threshold** (parent D-8), so **QSS Part IV inside the phase-out band ships zero-oracle** — and the census
must *say so per vector* rather than printing two independent "OK" lines. That failure mode is on the
record: for MFS AMT, two separately-disqualified oracles agreed and left three vectors with no witness at
all, and only a per-vector witness count found it.

Where the oracles cannot adjudicate, the **form** does, and the citation goes in the code (the standing
policy: we are literally told what to do on the form or in its instructions).

---

## 3. Out of scope for B3

- **The PDF and AcroForm map**, including the VIN's per-character comb boxes — parent **B4** owns form
  assets (`recon/fable/05-ty2025-field-maps.md` has the extracted field names). R-4 stands: the generic
  PII scanner does not cover VINs, so B4's emitter tests assert no VIN-shaped literal in fixtures.
- **`FullReturnParams` for TY2025** — lands after B4, and only then does the fail-closed gate come out.
- **Schedule 8812**, which shares the MAGI and the ceil but is its own work (§3).
- **Optimizer changes.** The phase-out bands add hidden marginal-rate adders, so per-$1 what-ifs show $0
  then a cliff. **Document it; do not "fix" it** — the step function is the law (§3).

---

## 4. Risks carried from the spec

R-1 the rounding asymmetry (T1 answers it); R-2 input definitions with no source document (T3, and the
reason prompt wording is the deliverable); R-3 Part IV's weak oracle coverage (T7 states it rather than
papering over it); R-4 the VIN as a new class of filed data (B4); R-5 expiry after TY2028 (T1).
