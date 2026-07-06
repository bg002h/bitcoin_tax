# R0 review — SPEC_irs_form_fill_sp2.md — round 1

- **Artifact:** `design/SPEC_irs_form_fill_sp2.md` (DRAFT) @ `feat/irs-form-fill-sp2` `9c1da06` (main == `99f26ca`)
- **Reviewer:** independent architect (R0, round 1; model: Fable). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical feature (filling 3 more OFFICIAL IRS PDFs).
- **Method:** evidence-driven. Inspected the actual official PDFs (`f1040-2025.pdf`, `fSE-2025.pdf`,
  `f8283-2025.pdf`, plus the SP1 `schedD-2025.pdf` for the 1040↔Schedule-D contract) with pypdf 6.14.2
  (scratchpad venv): full AcroForm leaf-field dumps with `/FT`/`/Rect`/on-states/`/MaxLen`, layout-mode page
  text, text-position visitor runs, `/TU` sweep, and a **live fill experiment per form** (set `/V`+`/AS`,
  drop `/XFA`, set NeedAppearances, save, re-read). Verified every repo claim against source:
  `btctax-forms/src/{lib,pdf,verify,fill8949,schedule_d,map,overflow}.rs`, `btctax-core/src/tax/se.rs`,
  `btctax-core/src/forms.rs`, `btctax-core/src/donation.rs`, `btctax-cli/src/cmd/admin.rs`,
  `btctax-cli/src/cli.rs`.

## VERDICT: **BLOCKED — 4 Critical / 4 Important / 3 Minor / 2 Nit**

The shape is right (reuse the SP1 fill primitive + XFA-drop + attestation + watermark + per-year maps;
conditional 8283/SE; partial-scope 1040 with a loud notice), and the XFA-hybrid claim re-verified TRUE for
all three PDFs. But the spec repeats SP1-round-1 history: it was written against the wrong facts about the
2025 forms in load-bearing places. (C1) Schedule SE **line 12 is lines 10+11 only** — `SeTaxResult.total`
includes the §1401(b)(2) Additional Medicare Tax, which lives on **Form 8959, not Schedule SE**; the spec's
KAT would lock an overstated official SE-tax line. (C2) the TY2025 1040 renumbered capital gains to **7a**
(with a new 7b checkbox line), and 7a equals Schedule D line 16 **only for gains** — on a net loss the form
routes through the **line 21 §1211 cap that SP1 deliberately scoped out**, so the spec's `line7 == line16`
fill overstates loss deductions. (C3) the SP1 **geometric read-back cannot run on any of the three forms**
as built — none of them fits the `Row{n}` grid model — so the spec's central safety claim ("reuse the
verifier") is currently vacuous and the per-form oracle must be *designed*, not assumed. (C4) the
unconditional digital-asset **YES** + `form_1040_always_present` fabricates a YES for hold-only years. All
four have concrete fixes below.

---

## Critical

### C1 — Schedule SE line 12 = lines 10 + 11 (SS + Medicare **only**); `SeTaxResult.total` also includes the 0.9% Additional Medicare Tax, which belongs on **Form 8959**, not Schedule SE. The spec's fill rule + KAT lock in an overstated official line 12.

**Spec claim (lines 27–30, 51):** "the SE-tax total (line 12)" filled from `SeTaxResult`; KAT
`schedule_se_line12_equals_compute_se_tax`.

**Evidence (official 2025 Schedule SE, page 1 text):**
> "**12** Self-employment tax. **Add lines 10 and 11.** Enter here and on Schedule 2 (Form 1040), line 4 …"

Line 10 = 12.4% × min(line 6, line 9); line 11 = 2.9% × line 6. The 0.9% Additional Medicare Tax appears
**nowhere on Schedule SE** — it is a Form 8959 item. But `se.rs:160`: `let total = ss + medicare + addl;`
and `SeTaxResult.total` is documented as "`ss + medicare + addl`" (se.rs:40–41). The core itself already
knows the distinction — se.rs:42–45 (on `deductible_half`): "**Schedule SE line 13 = SS + regular Medicare
only**", and the `c1_lock` golden (se.rs:241–266) pins `total = 30,564.30` where `ss + medicare =
29,870.85` (addl = 693.45). Filling line 12 with `total` overstates the official SE-tax line by `addl`
whenever `base > threshold` — and line 13 ("Multiply line 12 by 50%") would then visibly contradict the
filled `deductible_half`, which correctly excludes addl.

**Fix:** line 12 := `se_result.ss + se_result.medicare` (never `total`); line 13 := `deductible_half`
(consistent by construction). Rename the KAT `schedule_se_line12_equals_ss_plus_medicare` and add
`schedule_se_line12_excludes_addl_medicare` (fixture with `base > threshold`, e.g. the c1_lock $300k
golden: line 12 must be 29,870.85, NOT 30,564.30). When `addl > 0`, print a loud advisory: "Additional
Medicare Tax of $X is a Form 8959 item — it is NOT included on Schedule SE line 12."

### C2 — TY2025 Form 1040: capital gains is line **7a** (not 7), with a new **7b** checkbox line; and 7a = Schedule D line 16 **only when line 16 ≥ 0**. On a net loss, the form routes 7a through Schedule D **line 21** (§1211 cap: $3,000 / $1,500 MFS) — which SP1 scoped out. The spec's rule + KAT fill an overstated loss.

**Spec claim (lines 31–37, 50):** "(2) line 7 (capital gain/loss) = Schedule D line 16"; KAT
`form_1040_line7_equals_schedule_d_line16`.

**Evidence (official PDFs):**
- 2025 Form 1040 page 1 text: "**7a** Capital gain or (loss). Attach Schedule D if required … **7a**" and
  "**b** Check if: [ ] Schedule D not required [ ] Includes child's capital gain or (loss)". Fields:
  `f1_70[0]` rect [504,90,576,102] (7a amount); `c1_43[0]` [153,80] + `c1_44[0]` [261,80] (the 7b pair);
  `f1_71[0]` [403,78] (7b-adjacent amount). There is no "line 7" on the 2025 form. (The 2025 1040 is
  broadly revised: 3c/4c/5c checkbox lines, 6d, 11a AGI — the year-specific map was the right call, but
  the spec's facts must match it.)
- 2025 Schedule D page 2 text (the authoritative contract):
  > "If line 16 is a **gain**, enter the amount from line 16 on Form 1040 … **line 7a**."
  > "If line 16 is a **loss**, skip lines 17 through 20 … go to line 21."
  > "**21** If line 16 is a loss, enter here **and on Form 1040 … line 7a**, the **smaller of** • The loss
  > on line 16; or • ($3,000), or if married filing separately, ($1,500)"
  > "If line 16 is **zero** … enter **-0-** on Form 1040 … line 7a."
- SP1 deliberately scoped OUT lines 17–22 including 21 (SPEC_sp1 lines 60–63; `schedule_d.rs:5–6`), telling
  the user those are theirs. SP2 as specced would then AUTO-fill 7a = line 16 (say −50,000) on the official
  return — contradicting the attached Schedule D's own printed instruction and overstating the year's loss
  deduction by up to the full uncapped amount.

**Fix (pick one in the spec, explicitly):**
1. **Gain/zero-only fill (recommended, minimal):** fill 7a iff line 16 ≥ 0 (gain → line 16; zero → "-0-").
   On a net loss, leave 7a BLANK and extend the loud notice: "line 16 is a loss — complete Schedule D line
   21 and enter the capped amount on 1040 line 7a yourself." (Consistent with SP1's 17–22 scope-out.)
2. **Bring the cap in scope:** compute line 21 = max(line16, −3000/−1500-MFS) — requires filing status from
   `session.tax_profile(y)` (admin.rs:86), which may be absent → must fall back to option 1 when it is.
Either way: the **7b checkboxes stay UNCHECKED** (Schedule D IS attached — "not required" would be false;
the child-capital-gain box is not btctax's to answer) — say so, and the KATs become
`form_1040_line7a_gain_equals_schedule_d_line16` + `form_1040_line7a_loss_is_blank_with_notice` (or
`..._is_capped`) + `form_1040_7b_checkboxes_untouched`. The notice text must say "line 7a", not "line 7".

### C3 — The SP1 geometric read-back **cannot run on any of the three new forms**: none fits the `Row{n}` grid model. The spec's core safety claim ("the geometric map-independent read-back … applies", KATs `fill_8283/se/1040_readback_geometric`) is unimplementable as written; the per-form oracle must be designed in the spec.

**Spec claim (lines 15–18, 46–47, 74–75):** reuse "the **geometric map-independent read-back** verifier
(fails closed)"; per-form KATs "swap two map fields ⇒ RED".

**Evidence (engine vs the PDFs):**
- `verify.rs` derives its oracle from `Row{n}` grid subforms: `row_num` (verify.rs:56–61) parses the digits
  after `".Row"` — Form 8283's rows are named **`Row1A`/`Row1B`/`Row1C`/`Row1D`** (`"1A".parse::<u32>()`
  fails → every row excluded → `derive_bands` errors "no data-grid widgets"). The 8283 grid is also split
  across TWO tables (`Table_Line1_ColsA-C` + `Table_Line1_ColsD-I`) with heterogeneous per-row widget
  counts (ColsA-C rows nest a `ColB[0]` checkbox + VIN field), violating the consistent-`ncols` assertion
  (verify.rs:100–105).
- Schedule SE has **no table subform at all** — 27 flat leaf fields (`Page1[0].f1_1..f1_22`,
  `Page2[0].f2_1..f2_4`, two `ReadOrder` wrappers). Neither `derive_bands` nor `column_x_bands`
  (verify.rs:134–140, requiring a table token like Schedule D's `Table_PartI`, schedule_d.rs:18) has
  anything to derive from.
- Form 1040: the only grid is `Table_Dependents` — irrelevant to the two filled cells; `f1_70` is a flat
  field and the digital-asset checkbox is `Geo::Check`, which the SP1 verifier **exempts from geometry
  entirely** (verify.rs:169) — i.e. for the 1040 the "geometric read-back" as built would verify *nothing
  at all* beyond no-unmapped.

**What IS reusable:** `no_unmapped_filled` (verify.rs:225–242) is form-agnostic and load-bearing — keep it
per form. The fill primitive, XFA-drop, determinism, watermark, attestation, and `merge_copies` all reuse
cleanly.

**Fix — spec the per-form oracle (structural pins derivable from the PDF alone, no map trust):**
- **Column-x membership:** the SE/1040 amount column is a geometric cluster (right column x≈[504,576];
  SE mid-column x≈[410,482]) derivable from the page's Tx-field rects without the map; assert each written
  value's `cx` is in its logical column's band (catches right-vs-mid swaps, e.g. SE line 12 vs 13).
- **Ordinal-y descent:** for a logically ordered line sequence (SE 2,3,4a,4c,6,8a,8d,9,10,11,12; 8283 rows
  A→D), assert the written widgets' `cy` are strictly descending in logical order (catches any two swapped
  line fields — a swap breaks monotonicity on at least one side).
- **Same-y pair predicate for the DA question:** the Yes box is the LEFT member of the top-most same-y
  `/Btn` pair on page 1 whose on-states are {`/1`,`/2`} (measured: `c1_10[0]`=/1 at [518,497,526,505],
  `c1_10[1]`=/2 at [554,497,562,505]; the 'No' text label sits at x=565,y=498 — the only competing /1-/2
  same-y pair on page 1 is the dependents row at y=380, well below). Assert the checked field IS that
  left member.
- **Page membership + spacer exclusion** (see M1) everywhere.
- Keep the fault-inject KATs (swap two map entries ⇒ RED) against THESE pins, plus
  `map_2025_matches_bundled_pdf_fieldset` per form, plus SP1's one-time rendered-golden manual check.
This is real (small) engine work — a `Geo` extension + per-form verify fns — and the spec must own it
instead of claiming free reuse.

### C4 — Unconditional digital-asset **YES** + `form_1040_always_present` fabricates a YES for hold-only / no-activity years. The DA question is answered under penalty of perjury; "bought with USD and held" is a **No** per the IRS instructions.

**Spec claim (lines 31–34, 48–50, 73):** "the digital-asset question = YES … YES for any BTC seller";
`form_1040_always_present`; `form_1040_digital_asset_question_is_yes`.

**Evidence:** the 2025 question (page-1 text, verified): "At any time during 2025, did you: (a) receive (as
a reward, award, or payment for property or services); or (b) sell, exchange, or otherwise dispose of a
digital asset (or a financial interest in a digital asset)?" — matches the spec's quote. But
`export_irs_pdf` runs regardless of activity (admin.rs:151–199 fills even with zero rows), and SP2 writes
`form_1040_capgains.pdf` **always**. A user whose year has no disposals, no income, no donations — only
buys/holds — gets an official 1040 with YES checked, which the 1040 instructions expressly say is a No
(holding, wallet-to-wallet transfers, and purchases with U.S. dollars do not require Yes). "YES for any BTC
seller" is correct; "YES always" is not, and the KAT set mandates the false path.

**Fix:** YES iff btctax-evidenced qualifying activity in the tax year: any `form_8949` row (disposal) ∨ any
`income_recognized` entry (reward/mining/staking/airdrop = clause (a)) ∨ any Donation/Gift removal
(disposal by gift). Otherwise **leave BOTH boxes blank + notice** — never fill NO (btctax cannot know the
filer's full digital-asset universe: other wallets, NFTs, other coins). Simplest consistent shape: skip the
1040 entirely when there is nothing to fill (no qualifying activity ⇒ no 7a value either) with a printed
note, and replace `form_1040_always_present` with `form_1040_present_iff_reportable_activity` +
`form_1040_da_question_blank_or_skipped_when_no_activity`.

---

## Important

### I1 — `fill_schedule_se(se_result, year)` is an insufficient signature, and the fill line-set is under-specified: SP1's own self-consistency rule ([I4]) demands the full arithmetic chain, and line 8a needs `w2_ss_wages`, which `SeTaxResult` does not carry.

The spec (lines 27–30) lists ~5 values ("net-SE-earnings line, SS portion, Medicare portion, line 12,
line 13"). The 2025 Part I chain is: **2** (net profit — `net_se`), **3** (= 2), **4a** (= base), **4c**
(= 4a; 4b blank), **6** (= 4c; 5b blank), **8a** (W-2 SS wages + tips), **8d** (= 8a), **9** (line 7 −
8d), **10** (ss), **11** (medicare), **12** (10+11), **13** (12 × 50%, mid-column field `f1_22`).
Filling 10 while 9 is blank (or filling 4a with 2/3 blank) is exactly the "16 with 7/15 blank"
self-inconsistency SP1 fixed. And 8a/9 are **unrecoverable from `SeTaxResult`** (the min+round destroys
`ss_cap`); the export path already has the input — `session.tax_profile(y)?.w2_ss_wages` feeds
`compute_se_tax` (admin.rs:86–96). **Fix:** pass the profile's `w2_ss_wages` (or a small
`ScheduleSeFill { se: SeTaxResult, w2_ss_wages: Usd }`) into the fill; enumerate the full fill set + the
explicit blank set (A-checkbox, 1a/1b, 4b, 5a/5b, 8b/8c, Part II); decide the 8a≥$176,100 printed skip
rule ("skip lines 8b through 10") — recommend following the form: 9/10 blank in that case, matching
`ss == 0`.

### I2 — The line-4c **$400 floor** ("If less than $400, **stop**; you don't owe self-employment tax") is unmodeled: `compute_se_tax` has no $400 threshold, so SP2 would fill an official Schedule SE asserting SE tax the form's own printed rule says is not owed.

Evidence: form text at line 4c (quoted above); `se.rs:109–118` returns `None` only when `net_se == 0`
(doc: "no SE-eligible business income, OR expenses ≥ gross"). Example: `net_se = $400` → base $369.40 <
$400 → the app computes total $56.52 and SP2 would fill lines 10–13 on a form whose line 4c instructs
STOP. **Fix (SP2 boundary):** skip Schedule SE when `base < $400` with a printed note ("net SE earnings
under $400 — no SE tax owed; Schedule SE not produced"), + KAT `schedule_se_skipped_below_400_floor`.
File a FOLLOWUP for whether `compute_se_tax` itself should return `None` below the floor (core semantics
change — out of SP2 scope).

### I3 — Form 8283 is the **Rev. December 2025** revision and the spec misses its specifics — above all the new Section B line 2 **"k Digital assets" checkbox** (the direct Box-I/L analogue for this form).

Evidence (page-1 text + fields): "Form 8283 (Rev. December 2025)". Section B Part I line 2: "Check the box
that describes the type of property donated" with options a–l including "**k Digital assets**" — the
`c1_6` family split over three subforms; **k = `Lines2i-l[0].c1_6[2]`, on-state `/11`** (fill-verified in
the experiment). The spec never mentions the property-type checkbox; an implementer without this fact
plausibly checks "l Other" (wrong) or "f Securities" (worse). Also revision-specific and unmentioned:
(a) the Parts renumbered — Taxpayer statement = Part **III**, Declaration of Appraiser = Part **IV**,
Donee Acknowledgment = Part **V** (the core's `DonationDetails` doc comments cite the OLD numbering,
donation.rs:18–42 — harmless in core, but the SP2 map must not inherit it); (b) a new header block
(originally-reported entity name/ID `f1_3`/`f1_4`, family pass-through checkbox `c1_1`) — leave blank;
(c) **Section A = 4 rows (A–D)** and **Section B Part I = 3 rows (A–C)** — the row capacities are map
data; (d) the date-acquired columns are **"(mo., yr.)"** format (Section A col (e); Section B col (d)) —
NOT the SP1 `fmt_date` MM/DD/YYYY (see M-class note folded here: spec the month/year formatter, with the
customary "Various" open question resolved explicitly — one row per leg means each row has a single
acquisition date, so `MM/YYYY` per row is correct). **Fix:** add the checkbox (`/11`), the part
renumbering, the row capacities, and the date format to the 8283 section + KAT
`form_8283_section_b_checks_digital_assets_box`.

### I4 — The 8283 fill scope across Parts II–V is unspecified (what btctax fills vs what MUST stay blank), there is no 8283 partial-scope notice, and Section A/B overflow is unaddressed.

The app has more than the spec uses: `DonationDetails` carries `donee_ein`, `donee_address`,
`appraiser_name/address/tin/ptin`, `appraisal_date` (donation.rs:17–48) — so Part IV's appraiser identity
cells and Part V's donee name/EIN/address CAN be prefilled honestly. What must stay blank because it is
**another party's declaration**: the Part III taxpayer-statement signature, the Part IV appraiser
**signature/date** (the §6695A declaration is the appraiser's to sign), the Part V donee
acknowledgment — its receipt date, the "unrelated use?" Yes/No (`c2_4`), and the authorized
signature/title/date — and the Part II restriction questions (`c2_1..c2_3`, the donor's answers btctax
does not know). A Section-B 8283 without signed Part IV/V is **not filing-ready**, yet the spec gives the
1040 a loud partial-scope notice and the 8283 none. **Fix:** add a fill/blank scope table + a 1040-style
notice ("Form 8283 Section B requires the appraiser's signed declaration (Part IV) and the donee's signed
acknowledgment (Part V) — obtain both before filing"), escalate when any row has `needs_review == true`
(forms.rs:426–430). **Overflow:** one row per `RemovalLeg` (forms.rs:333–336) — a single multi-lot
donation overflows 4 Section-A / 3 Section-B rows immediately; `merge_copies` (overflow.rs:23–73) is
whole-document and reuses cleanly, and "Attach **one or more** Forms 8283" (form header) sanctions
multiple complete copies — but the spec must say so and define per-copy semantics (section checkbox,
Part IV/V repeated per copy). KATs: `form_8283_overflow_pages` + `each_8283_copy_renamed_no_shared_value`.

---

## Minor

### M1 — Schedule SE's preprinted-constant lines are 1-pt-wide spacer fields that must never be map targets.
Line 7 ($176,100) = `f1_13[0]` rect **[575,384,576,396]** (1 pt wide); line 14 ($7,240) = `f2_1[0]`
[575,696,576,708]. A write there is invisible-but-present (and `no_unmapped_filled` only guards
*unauthorized* fields — a mis-map ONTO the spacer would be authorized). Exclude them explicitly in the map
extraction + assert in `map_2025_matches_bundled_pdf_fieldset` that no mapped target is a spacer
(rect width < ~2 pt).

### M2 — The SE conditional conflates "no SE income" with "profile/table missing" — mirror the existing no-silent-drop rule.
`se_result` is `None` for THREE reasons (no business income / fully expensed / **no tax_profile or bundled
table**, admin.rs:83–99). The spec's "only when `se_result` is `Some`" (lines 29–30, 73) would silently skip
Schedule SE for a miner with SE income but no stored profile. The core already exposes the discriminator
(`se_net_income`, se.rs:55–62, "must emit a note, not silently drop — mirrors P2-C's m6"). Spec the note for
the PDF path.

### M3 — None of the three PDFs carries a single `/TU` tooltip (swept: 0 entries) — the "verify the on-state renders 'Yes'" step must be specified as label-anchored page-text geometry, and the fieldset-count method should be named.
With no tooltips, semantic identification at map-extraction time = match the checkbox rect against the
page's text runs (measured here: 'No' label at x=565,y=498 flanking `c1_10[1]`; the question text runs
y≈497–520). Pin that method (a Python extraction step, like SP1's band dump) in the spec, and record that
the spec's field counts (229/32/145, line 20) are pypdf `get_fields()` counts (leaf-field counts are
199/27/117 — the engine's `collect_fields` sees leaves), so the per-form `map_2025_matches_bundled_pdf_fieldset`
KAT defines its enumeration unambiguously.

---

## Nit

### N1 — KAT renames falling out of C1/C2/C4: `schedule_se_line12_equals_ss_plus_medicare`,
`form_1040_line7a_*` (piecewise gain/zero/loss), `form_1040_present_iff_reportable_activity`,
`form_8283_section_b_checks_digital_assets_box`. The notice string must say "line 7a".

### N2 — Form 8283 is revision-dated, not year-dated ("Rev. December 2025", form header + footer "Rev.
12-2025"). Keep the `forms/2025/` bundling convention, but record the revision string in the map TOML +
README (SP3's "other years" will otherwise assume year-editioned 8283s that don't exist). Optional note:
the form is only *required* when total noncash deductions exceed $500 (header text) — producing it below
that is harmless but worth a line in the man page.

---

## Answers to the review charter

1. **Per-form 2025 gotchas — found on all three.**
   - **1040:** DA question wording verified exact (spec quote correct); the Yes checkbox is
     `topmostSubform[0].Page1[0].c1_10[0]`, on-state **`/1`**, rect [518,497,526,505] ('No' = `c1_10[1]`
     `/2` at [554,497,562,505]) — the spec's geometric-identification plan is sound but must use the
     same-y-pair predicate (C3), since the nonresident-alien-spouse checkbox (`c1_9`, y=526, new for 2025)
     and the Presidential pair (`c1_6`/`c1_7`, y=590) are nearby. **Line 7 is now 7a** (`f1_70`), there is
     a new **7b** dual-checkbox line, and 7a ≠ line 16 on losses (C2). The 2025 1040 is broadly revised
     (3c/4c/5c/6d checkbox lines, 11a AGI) — year-specific map confirmed necessary.
   - **Schedule SE:** the 2025 form is the **unified** (post-2020) form — Part I lines 1a–13 on page 1,
     Part II optional methods on page 2; no short/long split, no Part III deferral. Line 12 is the SE-tax
     total line as the spec says, **but** = lines 10+11 only (C1 — `total` includes Form-8959 addl).
     0.9235 → line 4a; the wage-base cap → lines 7 (preprinted)/8a/9/10 — mapping cleanly REQUIRES
     `w2_ss_wages` (I1); $400 stop rule unmodeled (I2); preprinted-constant spacer fields (M1).
   - **8283:** Rev. 12-2025 — new "k Digital assets" type checkbox `/11` (I3), parts renumbered, 4+3 row
     capacities, "(mo., yr.)" dates, and the Part IV/V signature scope (I4).
2. **XFA + fillability:** confirmed — all three are `XFA=True` hybrids with complete classic AcroForm
   layers (f1040 199 leaves / SE 27 / 8283 117). Live experiment per form: set `/V`(+`/AS`), delete
   `/XFA`, NeedAppearances, save, re-read → values persist, XFA gone (`f1_70`='31337', `c1_10[0]`='/1';
   SE `f1_21`/`f1_22`; 8283 `f1_5` + `c1_6[2]`='/11'). Same static-hybrid LiveCycle family SP1
   render-proved with poppler; keep SP1's one-time manual Acrobat check per new form.
3. **Engine reuse:** fill primitive / XFA-drop / determinism / watermark / attestation /
   `no_unmapped_filled` / `merge_copies` — genuinely reusable. The **geometric band verifier is NOT**
   (C3): 8283's `Row1A` naming + split tables break `row_num`/`derive_bands`; SE and the 1040 have no
   grid at all (the 1040's two placements would degrade to `Geo::Check` + nothing). New per-form
   verification pins + a `Geo` extension are required engine work. New per-form TOML schemas are needed
   (expected — the spec doesn't claim schema reuse).
4. **Data wiring:** `form_8283()` (forms.rs:356) is real and rich (per-leg rows, carrier convention,
   §170(f)(11)(F) year-aggregate section A/B with strict >$5,000 — matches the form's "group of similar
   items" text; `DonationDetails` covers donee EIN + appraiser identity). `compute_se_tax`/`SeTaxResult`
   (se.rs:99/25) real; line-12 semantics (C1), w2 gap (I1), $400 floor (I2). Schedule D line 16 =
   `st.gain + lt.gain` (schedule_d.rs:101–109) real — but the 1040 contract is piecewise (C2).
   Conditionals: 8283-only-with-donations ✓; SE-only-with-SE-income needs the None-reason split (M2) and
   the $400 floor (I2); "1040 always" is wrong (C4).
5. **1040 partial-scope safety:** the bar (fill the DA question + the capital-gain line + a LOUD notice,
   under the same attestation/watermark regime) is **right** — it mirrors how commercial preparers emit
   partial officials for completion, and an un-named, un-signed, mostly-blank 1040 is unmistakably not a
   complete return; a separate opt-in flag would add friction without adding safety. What actually makes
   it safe is honesty at the two filled cells (C2, C4) plus a notice that enumerates exactly what was
   filled and that 7b/name/filing-status/everything-else are the filer's. The floated `--forms` filter is
   a fine place for per-form opt-out; default all-applicable is right.

## Appendix — measured field maps (for the fold; re-derive authoritatively at impl)

**Schedule SE 2025, page 1** (`topmostSubform[0].Page1[0].*`): name=f1_1, SSN=f1_2 (maxlen 11),
A-checkbox=c1_1(/1); 1a=f1_3, 1b=f1_4, 2=f1_5, 3=f1_6, 4a=f1_7, 4b=f1_8, 4c=f1_9,
5a=Line5a_ReadOrder.f1_10 (mid col), 5b=f1_11, 6=f1_12, [7=f1_13 SPACER], 8a=Line8a_ReadOrder.f1_14 (mid),
8b=f1_15, 8c=f1_16 (mid), 8d=f1_17, 9=f1_18, 10=f1_19, 11=f1_20, **12=f1_21** [504,252,576,264],
**13=f1_22** [410,216,482,228] (MID column). Page 2: [14=f2_1 SPACER], 15=f2_2, 16=f2_3, 17=f2_4.

**Form 1040 2025, page 1:** DA-Yes=`c1_10[0]`(/1) [518,497,526,505]; DA-No=`c1_10[1]`(/2) [554,497,562,505];
**7a=f1_70** [504,90,576,102]; 7b-boxes=c1_43 [153,80] ("Schedule D not required"), c1_44 [261,80]
("Includes child's capital gain or (loss)"); f1_71 [403,78] (7b-adjacent, not ours). Income rows
1a–1g=f1_47..f1_53, 1h=f1_54/f1_55, 1i=f1_56, 1z=f1_57, 2a/2b=f1_58/f1_59, 3a/3b=f1_60/f1_61,
4a/4b=f1_62/f1_63, 5a/5b=f1_65/f1_66, 6a/6b=f1_68/f1_69, 8=f1_72, 9=f1_73, 10=f1_74, 11a=f1_75.

**Form 8283 Rev. 12-2025:** Section A line 1: 4 rows Row1A–D; ColsA-C per row = donee-name+address
(`f1_5` …), vehicle-checkbox+VIN (`ColB[0].c1_2`+`f1_6`), description (`f1_7`); ColsD-I per row =
(d) date-contributed, (e) date-acquired-mo/yr, (f) how-acquired, (g) cost, (h) FMV, (i) method
(`f1_17..f1_22` for row A). Section B line 2 type checkboxes = `c1_6` family, states /1–/12 across
`Lines2a-c`/`Lines2d-h`/`Lines2i-l`; **k Digital assets = `Lines2i-l[0].c1_6[2]` on-state `/11`**;
b(1)=c1_7+f1_41(NPS#). Section B line 3: rows 3A–3C, ColsA-C = description/condition/appraised-FMV
(`f1_42..f1_50`), ColsD-I = date-acq(mo,yr)/how-acq/cost/bargain-sale/conservation-basis/amount-claimed
(`f1_51..f1_68`). Page 2: name=f2_1, id=f2_2; Part II 4a=f2_3 (maxlen 1), 4b(1)/(2)=f2_4/f2_5,
4c–4e=f2_6..f2_11; 5a/5b/5c Yes-No=c2_1/c2_2/c2_3 (/1,/2); Part III statement=f2_12; Part IV appraiser
sign/date=f2_13/f2_14, name=f2_15?, business address block=f2_15..f2_18 region; Part V donee
name=f2_15/EIN=f2_16 (maxlen 11)/address=f2_17/date-received=f2_9-region, unrelated-use=c2_4 (/1,/2),
authorized-sig/title/date=f2_19..f2_23. (Part IV/V exact cell assignment needs the label-anchored pass at
impl — the A/B row and line-2 checkbox assignments above were fill-verified.)
