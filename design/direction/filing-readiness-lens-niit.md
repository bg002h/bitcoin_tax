# Lens: NIIT / Form 8960 (§1411) — MFJ, $1M wages + $1M LTCG + $1M church gift, 5 kids

## VERDICT — can this slice FILE today?

**Files but WRONG — in the overstating direction only, and only in Part II.** Form 8960 is genuinely
transcribed, not closed-form: `Form8960Lines`
(`/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/other_taxes.rs:256-289`) carries one field per
printed line in the form's own numbering (1, 2, 5a, 5d, 7, 8, 9d, 11, 12, 13, 14, 15, 16, 17), each
with the instruction text as its doc comment, and `btctax-forms` does zero arithmetic — its 14-cell
`plan` array (`/scratch/code/bitcoin_tax/crates/btctax-forms/src/form8960.rs:41-56`) is a pure
field→AcroForm transcription. For this household Part I is **exactly right**: line 5a = 1040 line 7 =
$1,000,000 (i8960 line 5a says precisely "Form 1040 …, line 7, and Schedule 1 …, line 4" —
`/scratch/code/bitcoin_tax/design/forms/extract/i8960--2024.txt:869-880` — and btctax has no Schedule 1
line 4 path, so nothing is dropped), line 13 = AGI = $2,000,000, line 14 = $250,000, line 15 =
$1,750,000, line 16 = $1,000,000, **line 17 = $38,000** → Schedule 2 line 12
(`/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs:373`).

What is not clean: **Part II prints `0` on lines 9d and 11** for a filer who paid state income tax on
$1,000,000 of long-term gain. §1.1411-4(f)(3)(vi) and i8960 line 9b allow that tax, allocated to NII by
any reasonable method, as a deduction. btctax hardcodes `line9d = Usd::ZERO` and `line11 = line9d`
(`other_taxes.rs:312-313`) and has no line-9a/9b/9c/10 field at all. Combined with §G-11 (no line can
express blank), the filed page carries an affirmative `0` where the honest mark is either the real
allocated figure or a blank — the "an entry is testimony" case, on the exact line where this household
has a live entitlement.

**Hard gate first:** Form 8960 is emittable for **TY2024 only** —
`Form8960Map::for_year` (`/scratch/code/bitcoin_tax/crates/btctax-forms/src/map.rs:1253-1258`) and
`pdf::f8960_pdf` (`/scratch/code/bitcoin_tax/crates/btctax-forms/src/pdf.rs:110-116`) both return
`FormsError::UnsupportedYear` for anything else, and `crates/btctax-forms/forms/2025/` ships no
`f8960.map.toml` or `f8960.pdf`. Tax *tables* exist for 2024/2025/2026
(`/scratch/code/bitcoin_tax/crates/btctax-adapters/src/tax_tables.rs:76-78`), so "a tax year with tables
available" is **not** the same set as "a tax year Form 8960 can be filed for." This vector must be
TY2024 or the packet errors out.

---

## WHAT IS MISSING

### N-1 — Form 8960 line 9b (state/local income tax allocable to NII) is not modelled. LIVE on this vector.
- **Form requires:** i8960 *Line 9b—State, Local, and Foreign Income Tax*
  (`design/forms/extract/i8960--2024.txt:1836-1850`): *"Include state, local, and foreign income taxes
  you paid for the tax year that are attributable to net investment income… You can determine the
  portion … using any reasonable method."* The instructions' own worked example of a reasonable method
  (`:1798-1806`) is *"an allocation of the deduction based on the ratio of the amount of a taxpayer's
  gross investment income (Form 8960, line 8) to the amount of the taxpayer's AGI."*
- **btctax does:** `other_taxes.rs:312` — `let line9d = Usd::ZERO;` with the comment "v1 models no
  investment expenses". The map records it as `unmodeled`
  (`crates/btctax-forms/forms/2024/f8960.map.toml:79`).
- **CONSEQUENCE — OVERSTATES the tax, silently on the report.** Ratio = line 8 / AGI = 1,000,000 /
  2,000,000 = 0.50. Under the narrow (SALT-capped) reading, 9b = 0.50 × $10,000 = $5,000 → line 12
  $995,000 → **line 17 $37,810, an overstatement of $190**. Under the broad ("taxes you *paid*")
  reading, 9b = 0.50 × the actual state income tax on $2M — in a high-tax state that is $100k+ →
  **line 17 ≈ $34,200, an overstatement of ~$3,800**. Direction is safe, magnitude is not trivial, and
  which reading is right is genuinely unsettled (see *WHAT I AM NOT SURE OF*).
- **The input already exists.** `salt_line_5a` (`crates/btctax-core/src/tax/return_1040.rs:200-206`)
  already separates the income-tax variant from the §164(b)(5) sales-tax election, and
  `ScheduleALines::line5a` / `line5e` (`printed.rs:1299-1307`) already print it. Nothing new must be
  collected for the ratio method. **But the sales-tax branch must be honoured**: i8960 line 9b says
  *"Sales taxes aren't deductible in computing net investment income"* (`:1841`), so a filer with
  `salt_use_sales_tax == Some(true)` gets 9b = 0 from the state line, not an allocation of line 5e.
- **§3.4 is half-satisfied.** `crates/btctax-cli/LIMITATIONS.md:282` documents the omission, but SPEC
  §3.4 (`design/SPEC_full_return.md:184-192`) requires *"a **loud advisory** + LIMITATIONS entry and a
  KAT pinning the line to 0 + advisory."* There is **no `Advisory` variant** for it — the enum
  (`crates/btctax-core/src/tax/advisories.rs:53-182`) has `CtcOdcOmitted`,
  `Mfs63fSpouseBoxesForgone`, `MixedUseMortgageNotAllocated`, `SalesTaxElectionNotAsked` for exactly
  this class, and nothing for Form 8960. The report never tells this filer they overpaid.

### N-2 — line 9d / line 11 print `0`, which is testimony this filer cannot make. LIVE.
- **Form requires:** line 9d = "Add lines 9a, 9b, and 9c"; line 11 = "Add lines 9d and 10".
- **btctax does:** prints both at zero by design — map comment,
  `forms/2024/f8960.map.toml:23-24`: *"The DERIVED totals 9d and 11 ARE filled, at zero: the form's own
  arithmetic adds them … so a reader re-adding the column must find them."*
- **CONSEQUENCE — files an incomplete return whose incompleteness is invisible.** For a filer with
  genuinely no investment expenses, `0` is true and the map comment's rationale holds. For *this*
  filer, `0` on 9d asserts that 9a+9b+9c summed to zero when 9b was merely never computed. That is the
  §G-11 / "blank is the normal case" second row — nothing populated it — laundered as a printed zero,
  and it is the one place on this form where the two provenances are indistinguishable on the page.
  Note this is a **consequence of N-1**, not an independent defect: fix 9b and 9d becomes true again.

### N-3 — line 9a (investment interest expense) is unreachable end-to-end. NOT live on this vector.
- **Form requires:** i8960 line 9a (`:1817-1824`): *"Enter on Form 8960, line 9a, interest expense you
  paid or accrued during the tax year **deducted on Schedule A (Form 1040), line 9**."*
- **btctax does:** Schedule A line 9 does not exist in the model — `ScheduleALines` goes 8a → 8e →
  `line10 = line8e` with the doc comment *"add 8e and 9 (9 blank)"* (`printed.rs:1313-1315`); there is
  no `ScheduleAInputs` field for it and, per the shared facts, no Form 4952. So 9a's blank has a
  determinate provenance (Schedule A line 9 is blank ⇒ 9a is blank) and is internally consistent.
- **CONSEQUENCE — overstates for any filer with margin/investment interest; zero effect here** (this
  vector states no investment interest). Undocumented: LIMITATIONS.md:282 names only the state/local
  allocation, not 9a. Filed as a boundary, not a blocker for this household.

### N-4 — line 4a / Schedule E: correctly blank for this vector, and the reason is already written down.
The map's line-4a census entry (`f8960.map.toml:70`) is twice-corrected and now correct: btctax's only
trade-or-business income is Schedule C, derived as the SE base, which i8960's **line 4b body** (not its
caption) backs out — *"Net income or loss from a section 1411 trade or business that's taken into
account in determining self-employment income"* — so 4a in / 4b out / 4c = 0, and blank is
arithmetically identical. **This household has no rental and no K-1**, so the genuinely-unmodelled part
of line 4a (rents, royalties, partnerships, S corps, trusts) has nothing to carry. **No consequence on
this vector.** It would be a hard understatement for a filer with rental income; that is a Schedule E
gap, not a Form 8960 gap.

### N-5 — §G-19a (`niit_at_margin` reads the model's partial NII) is DISPLAY-ONLY, and is *correct* here.
- **Verified independently, not taken from FOLLOWUPS.** `MarginalRates::niit_at_margin`
  (`crates/btctax-core/src/tax/compute.rs:405`) and its consumer
  `MarginalRates::ltcg_all_in()` (`crates/btctax-core/src/tax/types.rs:109-115`) are read by exactly
  two production sites — `crates/btctax-cli/src/render.rs:1351-1353` and
  `crates/btctax-tui/src/tabs/tax.rs:123-125` — plus tests. **No filed line reads either.** Schedule 2
  line 12 comes from `f8960.line17` (`printed.rs:373`); `AbsoluteReturn::total_tax` comes from
  `niit.tax` (`return_1040.rs:1883`). Neither touches `MarginalRates`.
- **CONSEQUENCE on this vector: none.** G-19a's failure mode requires modelled NII < 0. Here
  `nii_with` = $1,000,000 ≥ 0 and `magi_with` = $2,000,000 > $250,000, so `niit_at_margin == true` and
  the report headlines 0.238 — the right answer. **Do not spend plan budget on G-19a for this vector.**

### N-6 — the double-oracle sweep is structurally blind to N-1. Method finding, no tax consequence.
`scripts/oracle/ots_direct.py:538-556` drives OTS's Form 8960 with `{L1, L2, L5a, L13}` only — it never
supplies L9a/L9b/L9c/L10. Tax-Calculator's `niit` (`scripts/oracle/gen_goldens.py:212`) has no NII
deduction term at all. **CONSEQUENCE:** both oracles will agree with btctax's $38,000 forever, so the
line-9b omission can never be caught by the sweep. This is the §G-9 shape — a value the oracles are
handed rather than deriving. It must be closed by transcription against i8960, not by an oracle run.
Corollary: there is **no charitable-contribution household in the oracle corpus at all** (`grep -c
charit scripts/oracle/corpus.py` → 0), so the charitable × NIIT non-interaction below is currently
unwitnessed by either engine.

### N-7 — the charitable deduction and NII: btctax is CORRECT, and the check is missing.
- **The law:** on the **individual** branch there is no charitable line. Charitable deductions appear on
  Form 8960 **only at line 18b**, which is in Part III's *Estates and Trusts* block
  (`design/forms/extract/f8960--2024.txt`, line 18b: *"Deductions for distributions of net investment
  income **and charitable deductions**"*). §170 is not in the §1.1411-4(f) properly-allocable list
  (i8960 `:1811-1815`: *"Unless a deduction is specifically identified as properly allocable to net
  investment income in the section 1411 regulations … the deduction isn't permitted."*), and it is
  below-the-line, so it does not touch line 13 MAGI either.
- **btctax agrees, structurally.** `form_8960_lines` (`other_taxes.rs:298-341`) takes six arguments —
  interest, dividends, net capital gain, crypto lending interest, AGI, status — and no charitable term
  can reach it. Line 13 = `round_dollar(agi)`, and AGI is 1040 line 11, computed *above* Schedule A
  (`Form1040Lines::line11` doc, `printed.rs:527`; line 12 is the itemized total). Part III lines 18a-21
  are absent from the struct and census-marked *unreachable* (`f8960.map.toml:86-93`). A $1M gift
  therefore moves nothing on this form: line 17 stays $38,000. **This is right.**
- **CONSEQUENCE — the guarantee has no test that reds when removed.** There is no KAT anywhere holding
  "charitable does not reduce NII or MAGI," and no corpus household combines a large gift with NIIT.
  Under house doctrine this guarantee does not exist. Cheap to close (see below).

### N-8 — two independent NIIT derivations, no reconciliation assertion. Minor, latent.
`form_8960` (`other_taxes.rs:224-243`, cent-rounded, feeds `total_tax` at `return_1040.rs:1883`) and
`form_8960_lines` (dollar-rounded, feeds Schedule 2 line 12 via `packet.rs:588` → `printed.rs:373`) are
two separate computations from two separate call sites. Their inputs are the same values today
(`return_1040.rs:1818-1825` uses the locals; `packet.rs:588` uses `ar.taxable_interest`,
`ar.ordinary_dividends`, `ar.capital_gain`, `pi.crypto_lending_interest` — which is
`crypto.nonbusiness_lending_interest` stored at `return_1040.rs:1962` — and `ar.agi`). **CONSEQUENCE:**
nothing asserts they agree. The property test at `other_taxes.rs:800-833` cross-foots the *printed*
chain against itself and never compares it to `form_8960`. This is the "a figure with no reader / two
chains ⇒ the test IS the comparison" shape that once left `total_tax` short by the whole AMT. No
observed divergence today; the guard is simply absent.

---

## THE SMALLEST THING THAT CLOSES IT

Sequenced. Steps 1-2 are the only ones this vector strictly needs; 3-5 are cheap and close the
doctrine gaps that made 1-2 invisible.

1. **Transcribe Form 8960 lines 9b, 9d and 11 (leave 9a, 9c and 10 as recorded blanks).**
   - Add `line9b: Usd` to `Form8960Lines`
     (`crates/btctax-core/src/tax/other_taxes.rs:256-289`), with i8960's own sentence as the doc
     comment. Adding a field to that struct `E0063`s every literal — that blast radius is the review.
   - `form_8960_lines` gains one argument, `state_local_income_tax_9b: Usd`, computed by the caller.
     `line9d = line9b` (9a/9c stay unmodelled); `line11 = line9d` is already correct.
   - Derive 9b in `return_1040.rs` next to the existing `form_8960` call (`:1818`), from lines that
     already exist: `schedule_a.salt_5a` **only when `salt_is_sales_tax == false`** (`salt_line_5a`,
     `return_1040.rs:200-206`), times the i8960 ratio `line8 / AGI`, then capped by the amount actually
     deducted (`ScheduleAParts::salt_5e`) — see the open question below before fixing the cap.
     Standard-deduction filers get 9b = 0 (no state tax was "properly deducted on your return").
   - Add `line9b` to the map (`crates/btctax-forms/forms/2024/f8960.map.toml`, field
     `topmostSubform[0].Page1[0].f1_17[0]` — already named in the census at `:79`), move that census
     entry from `unmodeled` to a mapped line, and extend `Form8960Map::lines()`
     (`crates/btctax-forms/src/map.rs:1260`) and `form8960.rs`'s `plan` array from 14 to 15 cells in
     printed reading order (9b sits between line 8 and line 9d, column MID).
   - **No new filer input.** This is arithmetic over lines btctax already collects.

2. **Add `Advisory::Form8960PartIIOmitted { line9a_live: bool }`** (or similar) to
   `crates/btctax-core/src/tax/advisories.rs`, fired whenever Form 8960 files, naming what was forgone
   and the direction. Even after step 1, lines 9a/9c/10 remain forgone and SPEC §3.4
   (`design/SPEC_full_return.md:188-191`) requires the loud advisory, not just the LIMITATIONS line.
   Update `crates/btctax-cli/LIMITATIONS.md:282` to name 9a as well as 9b, and move it into the
   "Advisories the report will show you" list once it fires.

3. **KAT the charitable non-interaction (N-7).** One test in
   `crates/btctax-core/tests/golden_returns.rs` or `tax_compute.rs`: the same MFJ household with and
   without a $1,000,000 §170(b)(1)(A)(i) cash gift, asserting `f8960.line13`, `line12`, `line16` and
   `line17` are **byte-identical** across the pair while 1040 line 15 moves by $1,000,000. Mutation-verify
   by subtracting the charitable amount from `agi` in `form_8960_lines` and watching it red. Also add a
   charitable household to `scripts/oracle/corpus.py` so both oracles witness it (N-6).

4. **Assert the two NIIT chains reconcile (N-8).** One assertion wherever the packet is assembled or in
   `golden_returns.rs`: `(ar.niit.tax - f8960.line17).abs() <= dec!(0.50)`. Mutation-verify by perturbing
   one call site's `agi` argument.

5. **Decide and record the TY2025 Form 8960 map.** If the plan's vector year is not 2024, step 1 is moot
   until `forms/2025/f8960.{map.toml,pdf}` exist and `for_year`/`f8960_pdf` learn 2025. The 2025 form's
   line numbering is unchanged (`design/forms/extract/f8960--2025.txt` is already in the repo), so this is
   a map + PDF drop plus two match arms, not a transcription.

---

## WHAT I AM NOT SURE OF

- **★ The size of line 9b turns on an unresolved question: does §164(b)(6)'s $10,000 cap limit it?**
  i8960 line 9b says *"income taxes you **paid** for the tax year that are attributable to net
  investment income"* — "paid", not "deducted" — and instructs you to enter it *"net of any deduction
  limitations imposed by **section 68**"* (`:1868-1874`), which is suspended for 2018-2025 and is a
  different provision from §164(b)(6). The *Lines 9 and 10* worksheet Part III (`:2089-2135`) caps the
  allocable total at *total itemized deductions reported on Form 1040* (line 5) less the non-§68 items —
  which for this household is ~$1,015,000 and therefore binds on nothing. Read literally, the worksheet
  permits allocating the **full** state income tax paid, not the capped $10,000. The contrary argument is
  §1411(c)(1)(B) ("deductions **allowed** by this subtitle") plus §164(b)(6). The two readings differ by
  ~$3,600 of NIIT on this vector. **This is exactly the "adjudicate an ambiguous instruction against the
  primary source" case — it needs §1411(c)(1)(B), §1.1411-4(f)(3)(vi) and §164(b)(6) read directly, and
  it must not be guessed.** Note line 9a's instruction *does* say "deducted on Schedule A", so the
  drafters used the two words deliberately.
- **Whether a reasonable-method choice needs to be collected from the filer.** i8960 says *"you may use
  any reasonable method"* and *"the reasonable method of allocation may differ from year to year"*
  (`:1776-1800`). Picking the line-8/AGI ratio silently is btctax choosing an elective allocation on the
  filer's behalf. It is the method the IRS itself names as an example, which is a strong defence, but
  under "an entry is testimony" it may still want to be an asked question rather than a default. I did
  not settle this.
- **Whether a standard-deduction filer gets any line 9b.** I asserted 0 above (nothing was "properly
  deducted on your return"), which matches i8960's repeated *"if properly deducted on your return when
  calculating your U.S. regular income tax"* qualifier (`:1783-1795`). Moot for this vector — with a $1M
  gift this household itemizes — but it is a branch step 1 must get right and I did not find a cite that
  states it in one sentence.
- **The vector's tax year.** I could not tell from the shared facts whether "a tax year with tables
  available" means 2024. Everything above assumes 2024; for 2025/2026 the whole slice is
  `UnsupportedYear` before any of this matters.
- **Whether the $1M gift is cash or appreciated BTC.** It changes Schedule A (60% vs 30% ceiling) and
  Form 8283, but I confirmed it changes nothing on Form 8960 either way: a donation is
  `RemovalKind::Donation` (`crates/btctax-core/src/tax/return_1040.rs:648`), a distinct kind from a sale,
  so it produces no Schedule D disposition and no line 5a amount. I checked this at the kind level only —
  I did not trace every path from `state.removals` into Schedule D to prove no donation leg can reach it.
- **Whether line 6 (CFC/PFIC) or the three Part I election checkboxes could ever bind here.** The census
  marks them unmodelled; nothing in this vector suggests a CFC, PFIC or nonresident-alien spouse. I took
  the census at its word rather than re-deriving it.
