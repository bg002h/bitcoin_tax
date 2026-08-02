# Schedule 1-A PLAN — review r4, BUILDABILITY LENS (Opus)

**Date:** 2026-08-01 · **Artifact:** `IMPLEMENTATION_PLAN_schedule_1a.md` (Status: r3) ·
**Brief:** [`../BRIEF-plan-r4.md`](../BRIEF-plan-r4.md)

**Result: 0 Critical / 5 Important / 3 Minor / 2 Nit.** Baseline re-verified green (1069 tests).

★★★ **IT CORRECTED THE AUTHOR ON THE CENTRAL SEQUENCING CLAIM.** I had concluded that §G-11's
`line_coverage` mechanism was the prerequisite for T3's ~25 `Option<Usd>` inputs. **Wrong** —
`line_coverage` is a **printed-line** provenance instrument (form / line / production / verbatim
instruction) and does not answer the input-side question at all. The input-side answer **already exists
in the tree**: `return_inputs.rs:626-652`, where answered-ness lives on an `Option<bool>` **class-(A)
gate** — which the classifier *forbids* `_` on, which `FORM_QUESTIONS` gives liveness and a refusal, and
which `attribute.rs` `E0004`s — with the amounts hanging off it as plain `Usd`. Its own doc says why:
*"`Option<Usd>` is a scalar the `_` rule permits, which would make this convention again."*
**So T3 should use one claim gate per part, not ~25 loose `Option<Usd>`s** — smaller, stronger, and
already proven. ★ And the four Part I add-backs are *already collected*, so Part I needs no new input.

★★ **BUT `line_coverage` DOES belong — on the T2 OUTPUT struct, and it is structurally blind to it
today (I-2).** Three hard-codings, all verified: the completeness rule reads a **three-file list** that
cannot see a new `schedule_1a.rs`; `money_bearing_types` gates on `": Usd"` and so never matches
`Option<Usd>`; and `const YEAR = "2024"` makes an `f1040s1a` row resolve to a 2024 extract that does
not exist. T2/T5 would land 48 printed money lines while the checker reports *"OK"* over zero of them.

★★ **I-4 is a repeat of a mistake this tree already made and already fixed.** T3a refuses on
`schedule_c.is_some()` — but a Schedule C **is the mining household**, and Part II's own Caution makes
the deduction opt-in. Once the fail-closed gate comes out, every TY2025 return with a Schedule C
refuses. `questions.rs:546-552` records the identical lesson: *"It was a class-(A) declaration that
REFUSED, and it was `live: |_| true` — so it blocked EVERY return btctax could compute."*
★ Directly relevant to the owner's plan: **if their real TY2025 return has a Schedule C, the diff cannot
run at all.**

★ **I-3 is the B3 field-of-view failure inside one document.** T2's doc-comment gate is `cite-check`
semantics, which proves a quotation is *the form's* words, not *that line's* — so line 28's rounding
sentence on line 11 passes. T1, in the same plan, already built `printed_line(label)` for exactly this
and its own doc says *"The fix is not more citation checking."* Nobody carried it forward.

★ **I-1:** the KAT has CLAUDE.md's half (a) and not half (b) — no per-line provenance, so "present but
never populated" passes. And the precedent T2 cites derives its *expected* side from a JSON fixture.

**Verbatim below.**

---

# Schedule 1-A IMPLEMENTATION_PLAN — independent review r4, **BUILDABILITY lens** (Opus)

**Scope:** is this executable against the actual tree at `main` (884bea2), and does each of T2–T7 leave it green? Read: the brief, the plan (r3), `PLAN_schedule_1a-opus-r1.md`, `PROVENANCE_CENSUS_schedule_1a.md`, and the source — `classifier.rs`, `line_coverage.rs`, `line_coverage_check.rs`, `tables.rs`, `return_inputs.rs`, `questions.rs`, `registries.rs`, `coverage.rs`, `attribute.rs`, `label_reader.rs`, `cite_check.rs`, `field_census.rs`, `input_form_store.rs`, `return_1040.rs`, `tax_tables.rs`, and both committed Schedule 1-A extracts. Baseline confirmed green: `cargo nextest run -p btctax-core -p xtask -p btctax-input-form` → **1069 passed**, 6.6s.

**VERDICT: 0 Critical / 5 Important**

---

### I-1. T2's conformance KAT satisfies CLAUDE.md's half (a) and omits half (b) — it has no per-line PROVENANCE, so it cannot red on "present but never populated"

**SEVERITY: Important**
**WHERE:** PLAN §T2, "The conformance KAT — the mechanical gate, executed" (three bullets).
**CLAIM:** The KAT as specified checks *membership* (48 labels ⇔ 48 fields, closed at both ends) and *quotation* (doc comment vs extract). It records nothing about what DECIDES each line, so a field that is declared, doc-commented, and never assigned by T4 passes.

**CONSEQUENCE:** The plan's own §0 says *"the review gate on this work is mechanical … That is a test, and T2 writes it."* Executing T2 exactly as written yields a gate that answers "is every line present?" and cannot answer "is every line accounted for?" — which is the half CLAUDE.md names as load-bearing, and the half the emitted PDF and both oracles are blind to.

**EVIDENCE:** CLAUDE.md: *"a conformance KAT must (a) enumerate the expected line set **from the form's extracted text** … and (b) **require every line to be accounted for** — mapped to a field or decision, or explicitly recorded as carrying none **with a reason**. A checker that cannot distinguish 'this line encodes no decision' from 'we forgot this line' is not a conformance check."* The plan's three bullets are (a), (a), and the quotation check; there is no (b).

The mechanism for (b) already exists and is not named: `line_coverage.rs:38-68` defines `Production { Collected, Carry, Combine, Clamped(Polarity), Scaled, Bounded, Constant, Exception }` — *"productions of the forms' own grammar"* — with `Exception` requiring a written reason, ratcheted at 9. Schedule 1-A's 48 lines map onto it almost exactly: line 1 `Carry`, 2a–2d `Collected`, 2e/3/6/14c/23/37/38 `Combine`, 7/15/24 `Bounded`, 12/20/29/34 `Scaled`, 9/17/26/32 `Constant`, 13/21/30/35 `Clamped(FloorAtZero)`, and lines 10/18/27/33 as `Exception`s (they are conditional *jumps*, not clamps — the same class as the ratchet's existing `f1040:34` entry, whose note reads *"A CONDITIONAL ENTRY, NOT A CLAMP — and the distinction is the whole program"*).

**Also note the precedent T2 cites is the wrong one.** *"the pattern the Form 6251 KAT settled on"* is `form6251.rs:640-712`: the actual side is a hand-written `[("line1", got.line1), …]` tuple list and the **expected side is the JSON fixture's keys** (`form6251_vectors.json`), not the form extract. Followed literally, that is fixture-vs-hand-list — precisely the shape the plan warns against two paragraphs earlier. What *does* work in it is the tuple form (`got.lineN` is a field access, so the compiler ties the name list to the struct); T2 should mandate that form explicitly, or the serde leaf-walk `coverage.rs::walk` already uses on `ReturnInputs`.

**Fix:** In T2, add a fourth bullet: every line label carries a `Production` (or an `Exception` with a reason), asserted alongside the label-set comparison; and state that the actual set is built as `(label, field-access)` pairs so the compiler ties it to the struct.

---

### I-2. The new `schedule_1a.rs` is structurally invisible to the §G-11 line-coverage checker — the one shipped mechanism that forbids `_` on money will not reach this form

**SEVERITY: Important**
**WHERE:** PLAN §T2 (`crates/btctax-core/src/tax/schedule_1a.rs` (new)) and §T5. The plan never mentions `line_coverage` (grep of the plan for `line_coverage|line-coverage|Production`: 0 hits; `coverage` matches only the input-form KAT and R-3).
**CLAIM:** `xtask line-coverage` — enforced on every commit via `line_coverage_check::tests::the_committed_coverage_table_is_consistent_with_the_form_text`, which runs inside `make check` — cannot see a new module, cannot see `Option<Usd>`, and cannot resolve a 2025 extract. All three are hard-coded.

**CONSEQUENCE:** T2/T5 land 48 printed money lines and the checker stays green, reporting *"line-coverage OK: 179 money lines across 14 form(s)"* while the largest new form on the branch contributes zero rows. That is the false-completeness the plan's own §T2 ★★ paragraph (census F-4) warns about, one layer down.

**EVIDENCE — three separate hard-codings, all verified:**

1. **Completeness rule (4b) reads a three-file list.** `line_coverage_check.rs:282-286`:
   ```rust
   for rel in [
       "crates/btctax-core/src/tax/printed.rs",
       "crates/btctax-core/src/tax/other_taxes.rs",
       "crates/btctax-core/src/tax/qbi.rs",
   ] {
   ```
   A money-bearing type in `schedule_1a.rs` is never enumerated, so the rule's own promise — *"A new money-bearing type therefore fails the build the moment it is written"* — does not hold for this file.
2. **`Option<Usd>` is invisible to the type detector, and `Coverage::line` cannot accept it.** `money_bearing_types` gates on `l.contains(": Usd")` (`:458`), which does not match `: Option<Usd>`; and `Coverage::line(&mut self, _value: Usd, …)` (`:112`) takes `Usd` by value. T3a *requires* lines 5 and 14b to be able to express blank/refuse, so at least those fields cannot be plain `Usd`.
3. **`const YEAR: &str = "2024";`** (`:37`). A row quoting `form: "f1040s1a"` looks for `design/forms/extract/f1040s1a--2024.txt` (absent), then falls through to the map probe `crates/btctax-forms/forms/2024/f1040s1a.map.toml` — also absent (verified: no `f1040s1a*` under `crates/btctax-forms/forms/`, B4 owns it). Result today is a **hard error** with a misleading message: *"quotes form … which has NEITHER an extract NOR a map — the form name is wrong."*

There is a fourth, smaller hole worth knowing before adopting: rule (4b) checks that a `fn cover_<type>(` **exists**, not that it is called from `all()` (`:1964`), so a `cover_schedule1alines` defined and never wired contributes nothing and still passes.

**Fix (concrete, in T2):** add `schedule_1a.rs` to the (4b) file list; make the type detector match `Option<Usd>` (and give `Coverage` an `optional_line` that consumes it); move `YEAR` from a module const onto the `LineCoverage` row so `f1040s1a--2025.txt` resolves; add the `cover_*` call to `all()`; land it with a B1 planted-defect test (drop one line's row, assert red).

---

### I-3. T2's doc-comment gate re-inherits §G-10's blind spot — the per-line matcher T1 built for this exact form is not carried forward

**SEVERITY: Important**
**WHERE:** PLAN §T2, third KAT bullet: *"each field's doc comment contains the line's own instruction text, checked against a committed extract of the text layer, so a paraphrase reds."*
**CLAIM:** "Checked against a committed extract" is `cite-check` semantics, and `cite-check` proves a quotation is **the form's** words, not **that line's** words. A line-11 doc comment carrying line 28's sentence passes.

**CONSEQUENCE:** The plan's stated mechanical gate is *"is every line present, and does each doc comment match the instruction text?"*. Half of it is a checker that cannot discriminate the single most dangerous swap on this form — and the swap it cannot catch is the one the plan itself calls out (*"moving line 28's 'increase … to the next higher' onto line 11 survives it — and that swap inverts the rounding for Parts II/III, the most dangerous single fact in this form"*).

**EVIDENCE:** That sentence is not mine; it is `tables.rs:1317-1320`, T1's own doc comment, which then says *"The fix is not more citation checking"* and builds `printed_line(label)` (`tables.rs:1295-1313`) — a per-label extractor that returns the numbered line plus its continuations, used by `each_phase_out_rounds_the_way_its_own_printed_line_says_to` to derive the rounding direction from the line itself and `panic!` rather than default. T2 does not reference it. `cite_check::schedule_1a_docs()` (`:399-411`) currently covers only `SPEC_schedule_1a.md` and `IMPLEMENTATION_PLAN_schedule_1a.md`; adding `schedule_1a.rs` to that list gives per-*document* verification, not per-*line*.

This is the B3 failure mode in CLAUDE.md verbatim: *"the fix already existed in the branch … Nobody carried it back, because no reviewer ever held both commits at once."* T1 and T2 are in the same plan.

**Fix:** One sentence in T2: the doc-comment check is `printed_line(<the field's label>)` ∋ the field's quoted instruction (whitespace-normalized), not a whole-document `contains`.

---

### I-4. T3a's refusal is scoped to "a Schedule C is present", which refuses every TY2025 mining household regardless of whether Part II/III is claimed

**SEVERITY: Important**
**WHERE:** PLAN §T3a, the two-row table.
**CLAIM:** The predicate is the presence of a Schedule C, not the claiming of the tips/overtime deduction. Part II's own Caution makes the deduction opt-in; a filer who received no qualified tips never reaches line 5, and refusing them is refusing on a line the form tells them not to fill out.

**CONSEQUENCE:** Once the fail-closed gate comes out, **every** TY2025 return carrying a Schedule C refuses — and in btctax a Schedule C is not an edge case, it is the mining household. `return_1040.rs:3072` is named `business_income_without_schedule_c_fails_loud`, i.e. business crypto income *requires* `ri.schedule_c`. The branch's stated purpose (diff btctax's TY2025 output against the owner's real prepared return) cannot be met if the owner's return has a Schedule C.

**EVIDENCE:** The plan: *"| 5 | **REFUSE** when a Schedule C is present | a filer with self-employment income may well have 1099-reported tips, and we cannot ask."* The form: *"**Caution:** Fill out Part II only if you received qualified tips."* T3 already collects the gating answer — *"★ occupation is on the Treasury list … Defaults to **NO**"* — so the claim-gate exists and the refusal can be conditioned on it.

This is a repeat of a mistake already made and already fixed once in this tree. `questions.rs:546-552`: *"★★★ It was a class-(A) declaration that REFUSED, and it was `live: |_| true` — so it blocked EVERY return btctax could compute. That is the single biggest usability cost in the registry, and it bought nothing."*

(Secondary, same row: 14b's rationale — *"with no Schedule C there is no payor relationship to report"* — does not hold for **1099-MISC box 3**, which is Other Income reported on Schedule 1 line 8z, not Schedule C. The predicate is wrong for the 1099-MISC half even on its own terms.)

**Fix:** Gate both refusals on the Part II / Part III **claim** gate (`occupation on the Treasury list == Some(true)` / the overtime gate), not on `schedule_c.is_some()`.

---

### I-5. T3a's "lines with no input path" table is not closed — line 4b (Form 4137) is the third line of that exact shape and carries no recorded disposition

**SEVERITY: Important**
**WHERE:** PLAN §T3a, *"★ btctax has exactly three lawful moves here — collect, refuse, or genuinely blank … For B3, choose per line and record the reason"*, followed by a table with rows for lines 5 and 14b only.
**CLAIM:** Line 4b names a form `ReturnInputs` does not model, exactly as lines 5 and 14b do. The census found the 1099 family because it went looking for 1099s; nobody looked for Form 4137.

**CONSEQUENCE:** T2/T4 reach line 4b with no instruction, and the path of least resistance is `Usd::ZERO`. For a filer with unreported tips — the population Part II exists for — that understates line 4c ⇒ smaller lines 6/7/13/38 ⇒ overstates tax, and prints sworn testimony ("I filed no Form 4137") that nobody gave. Direction is fail-closed, which is why this is Important and not Critical.

**EVIDENCE:** The form (`fixtures/schedule_1a_2025_form.txt:40-41`):
> "4b Qualified tips included on Form 4137, line 1, row A, column (c). If Form 4137 is not filed, enter -0-"

`grep -rn 4137 crates/ --include='*.rs'` returns four hits, none of them an input: `other_taxes.rs:92` records Schedule 2 lines 2/3 as *"unmodeled and deliberately absent"*, `map.rs:905` says the same, and `return_refuse.rs:769` refuses on `w2.box8_allocated_tips > Usd::ZERO` — *"W-2 box 8 allocated tips require Form 4137"*.

That refusal is a **partial** guard: box 8 is *allocated* tips, but Form 4137 is also required for tips the employee did not report to the employer, which btctax cannot see (`W2` carries `box7_ss_tips` and `box8_allocated_tips` and nothing else). The likely correct disposition is a form-directed `-0-` — btctax emits no Form 4137, so "Form 4137 is not filed" is true of the return it produces — but per the plan's own doctrine (*"'cannot apply' and 'we never looked' are the two blanks this project exists to separate"*) that reasoning has to be written down, together with whether the box-8 refusal needs widening to "tips not reported to your employer".

**Fix:** Add a line-4b row to the T3a table with its move and its reason, and state whether the existing `box8_allocated_tips` refusal is the guard or needs a companion declaration.

---

## Minor

**M-1 — "~25 leaves plus six declarations" is stale by roughly 3×, and T3 is the task the chokepoint review exists to size.** The r1 fold *added* the Part II gates (I-3) and the entire Part IV eligibility set (C-2) without updating the header. Recounting from T3's own body: Part II 5 (occupation gate, multi-occupation carve-out, qualified-tip amount criteria, SSTB, ⊆ box 7), Part III 3, Part IV **9 per vehicle** (originated after 2024-12-31; originated by you; purchase not lease; personal use; first lien; original use/new; body class + GVWR < 14,000 lb; US final assembly; no negative equity), plus the SSN bar per person for II/III/V ⇒ **≈19 declarations, not six**. And Part IV's are *per vehicle* against a line-22 structure (2 rows × 3 columns) that `ReturnInputs` has no shape for at all — a new row-shaped input-form section, not a leaf. Each declaration costs a `QuestionId` + `FORM_QUESTIONS` entry + `FieldId` + `Field` + an `EXPECTED_LEAF_PATHS` row, and each new `RefuseReason` costs an `attribute.rs` arm. T3's touchpoint list omits `crates/btctax-input-form/src/attribute.rs` (the exhaustive no-`_` `RefuseReason` match, 19 `QuestionId::` sites) and the pinned counters that must move in lockstep: `questions.rs:985-986` (`QuestionId::ALL.len() == 12`, `FORM_QUESTIONS.len() == 12`), `coverage.rs` (`field_count == 80`, `covered.len() == 80`, and the 80-row `EXPECTED_LEAF_PATHS` literal). All are compiler- or test-forced, so nothing escapes — but "landed whole … in one pass" is a materially larger pass than the plan states.

**M-2 — T2's prose names the wrong second heading.** *"neither line 4 nor line 14 has a bare amount box — each is a heading for its lettered sub-lines."* The form prints no standalone `14` at all (it prints `14a` on the first row); the two printed labels that carry no box are **4 and 22**. The per-part counts (I 7, II 12, III 10, IV 10, V 8, VI 1 = 48) are correct and I confirmed them by hand off the extract — the gloss is what is wrong, and an implementer building the expected set from the sentence rather than the counts gets 49. Independently settled since the plan was written: `label_reader.rs::the_witnesses_resolve_the_50_vs_48_question` asserts 50 printed labels, 48 with an entry box, `headings == vec!["4", "22"]`.

**M-3 — citation decay (the tree has moved ~40 lines since r3).** `return_1040.rs:1269` (`let schedule_1a_additional = Usd::ZERO;`) is now **:1309**; `:1432` (`ti_before_qbi`) is now **:1474**; `printed.rs:384` (`p.half_se_15`) is now **:406**; `return_inputs.rs:417-423` / `:425` are now **:691-695** / **:695**. And §0's *"both archived in `design/amt-form6251/`"* is wrong — that directory holds no Schedule 1-A asset. The primary sources are `design/forms/2025/f1040s1a--2025.pdf`, `design/forms/extract/{f1040s1a,i1040gi}--2025.txt`, and — the ones the tests actually read — `crates/btctax-core/src/tax/fixtures/schedule_1a_2025_{form,instructions}.txt`.

## Nit

**N-1** — the parent `SPEC.md:619` still sizes B3 as *"38 lines, 6 parts, ~25 collected inputs"*. Not under review, but it is the line a future reader will cite, and 38 is the count r1 I-1 corrected.
**N-2** — `label_reader.rs:38-41`'s `Kind::Heading` doc carries the same slip as M-2 (*"Schedule 1-A lines 4, 14, 22"*) while its own test asserts `["4", "22"]`.

---

## ALSO CHECKED, SOUND

**The fail-closed gate holds; nothing in T2–T7 opens TY2025 early.** `BundledFullReturnTables::load()` (`tax_tables.rs:99-104`) inserts **only** 2024. All twelve `full_return_for` call sites are `Option`-gated (`cmd/tax.rs:295/330/499`, `session.rs:517/560`, `resolve.rs:264/351`, `admin.rs:760`, `tui-edit/main.rs:1310`, plus test sites). `input_form_store::commit` (now ~`:302-315`) returns `NoTables` both on `None` params and on `table.year != year || params.year != year`. Decisively: **`schedule_1a_params(year)` is a free function** (`tables.rs:1023`), not a `FullReturnParams` field — so T5 can wire the real line 38 through `assemble_absolute` and still get `Usd::ZERO` for TY2024 *because `schedule_1a_params(2024)` is `None`*, which is exactly the "zero because the form has no such line" provenance T5 asks for, without bundling anything. `ty2025_full_return_must_stay_fail_closed_until_complete` and `ty2026_full_return_must_stay_fail_closed` both pass at HEAD.

**T1 is built and is stronger than the plan describes.** `StairStepPhaseOut::exhaustion_excess` implements the per-direction identity (`(full_steps - 1) * step + 1` for `Ceil` ⇒ **$49,001**); `each_phase_out_exhausts_at_its_own_knee_and_not_one_step_earlier` carries **both** halves of the paired assertion; `line_34_rounds_before_line_35_subtracts` pins r1 M-1 at the $50,025 half-dollar; `the_two_worked_examples_that_distinguish_floor_from_ceil` pins (b) $2,300 and (c) $5,000; `schedule_1a_exists_only_for_2025_through_2028` fails closed at TY2029+ *and* asserts 2026–2028 are byte-identical to 2025; and `each_phase_out_rounds_the_way_its_own_printed_line_says_to` derives the direction from `printed_line(label)` and panics rather than defaulting. T1 leaves the tree green.

**The 48-label count, independently confirmed twice.** By hand off `schedule_1a_2025_form.txt`: I {1,2a–2e,3}=7 · II {4a–4c,5,6,7,8,9,10,11,12,13}=12 · III {14a–14c,15–21}=10 · IV {22a,22b,23–30}=10 · V {31–35,36a,36b,37}=8 · VI {38}=1 = **48**, with `4` and `22` as box-less headings and 52 leaves once 22's three columns are counted. And by `xtask label_reader`'s two witnesses, which resolve the same 50/48/2 split from the PDF geometry with the column found by monotonicity rather than position.

**Census F-4 is executable today.** Both extracts are committed **in-crate** (`fixtures/schedule_1a_2025_form.txt`, `..._instructions.txt`), so "the expected set comes from both extracts" needs no fetch and no `include_str!` that escapes the crate.

**The plan's quotations are already held by a passing test.** `cite_check::schedule_1a_docs()` covers `SPEC_schedule_1a.md` and `IMPLEMENTATION_PLAN_schedule_1a.md` against both extracts (blockquote + `*"…"*` + plain `"…"` passes), and `every_quotation_in_the_schedule_1a_documents_is_verbatim_from_the_manual` is green. I did not re-verify the plan's quoted spans.

**The registry index-coupling hazard is real but caught.** `decl_tristate!($idx:literal, …)` binds `FORM_QUESTIONS[$idx]`, so a mid-array insert silently repoints every later entry (`registries.rs:201-203` says so, and records that a draft did exactly that). The guard is `coverage.rs`'s observed `EXPECTED_LEAF_PATHS` map: a repointed index writes a different leaf and names itself in the assert diff. Adding T3's declarations at the **end** of `FORM_QUESTIONS` keeps the existing literals valid.

**CLI `answer.rs` needs no per-question edit** — it walks `Ask::Declaration(q)`/`Ask::Skippable(s)` generically. Worth knowing: it collects **no money**, so the ~25 Schedule 1-A money leaves reach a return only via the input-form (TUI-edit) or TOML import.

**`classifier.rs`'s `_`-on-money permission is confirmed verbatim**, exactly as SPEC §8a records (`classifier.rs:15-17`): *"`_` … is **FORBIDDEN** on structs, collections, and `bool` / `Option<bool>` / defaulted-enum leaves … `_` is **PERMITTED** on other scalar leaves."* But the SPEC's conclusion needs one correction that materially helps T3: **the arrival of a new money leaf is already structural twice over**, and the tree already contains the structural answer for its *refusal semantics*.
- Arrival: `classifier.rs` destructures with no `..`, so a new `ReturnInputs` field is `E0027` until a human edits the file; and `coverage.rs::every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` derives the covered set **by observation** (mutate-and-diff) and fails until the leaf gets a `Field` or an `EXEMPT` entry with a reason, against pinned counts.
- Refusal semantics: `return_inputs.rs:626-652` is the built pattern — the answered-ness lives on an `Option<bool>` **class-(A) gate** (`has_income_exclusion`, which the classifier *forbids* `_` on, which `FORM_QUESTIONS` gives liveness and a refusal, which `attribute.rs` `E0004`s), and the four amounts hang off it as plain `Usd`. `modified_agi()` is the refuse-at-the-point-of-need accessor. Its own doc says why: *"`Option<bool>` is a leaf the classifier **forbids** `_` on … whereas `Option<Usd>` is a scalar the `_` rule permits, which would make this convention again."* **T3 should say to use that pattern per part** (a Part II claim gate, a Part III claim gate, a per-vehicle Part IV gate) rather than ~25 loose `Option<Usd>`s — and the four Part I add-backs (2a–2d) are *already collected*, so Part I needs no new input at all. That is a smaller and stronger change than adopting `line_coverage` on the input side, which is a **printed-line** provenance instrument (form/line/production/verbatim-instruction) and does not answer the input-side question. `line_coverage` belongs on the T2 *output* struct — see I-2.

## WHAT WOULD MAKE THIS REVIEW WRONG

- **I-1 halves.** If T2 is already intended to build the actual set as `(label, got.lineN)` tuples (the f6251 shape, which the compiler ties to the struct), then the membership half is sound and only the provenance half stands. The plan does not say, and the precedent it names derives its *expected* side from a JSON fixture, so I read the risk as live.
- **I-1 downgrades if B4 is the real backstop.** `field_census.rs::census_accounts_for_every_field` requires `(map FQNs) ∪ (census FQNs) == (PDF AcroForm FQNs)` exactly, with `GAPS` ratcheted to 0. Once `f1040s1a` is censused in B4, a struct field that was never added surfaces there as an unaccounted AcroForm field. That backstop is real — but it is out of B3's scope, and the plan asserts the mechanical gate is T2's.
- **I-5 becomes a documentation Minor** if one judges that btctax emits no Form 4137 and therefore *"Form 4137 is not filed"* is simply true of every return it produces. I think that argument is right; my finding is that it is nowhere written, and the plan's own doctrine says an unwritten reason and an oversight are the two blanks this project exists to separate.
- **I-4's blast radius shrinks** if the owner's real TY2025 return carries no Schedule C — but the predicate is still the wrong one.
- **Lens boundary.** I did not re-derive the eligibility *content* of T3's declaration tables against `i1040gi` pp. 101-110 (the brief's risk #1) — that is the correctness lens, and `cite-check` already holds every quotation in the plan verbatim. I also did not attempt T7's oracle census, which needs `OTS_DIR` and `.venv`.