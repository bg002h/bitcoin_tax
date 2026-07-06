# R0 review — SPEC_irs_form_fill_sp3.md — round 1

- **Artifact:** `design/SPEC_irs_form_fill_sp3.md` (DRAFT) @ `feat/irs-form-fill-sp3` `bea032c` (main == `55f5812`)
- **Reviewer:** independent architect (R0, round 1; model: Fable). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical feature (filling OFFICIAL IRS PDFs for PRIOR years 2017 + 2024).
- **Method:** evidence-driven. Inspected the actual official PDFs (`f8949-{2017,2024}.pdf`,
  `schedD-{2017,2024}.pdf`, `fSE-{2017,2024,2025}.pdf`, `f1040-{2017,2024}.pdf` in the scratchpad
  `irsforms/` dir) with pypdf (scratchpad venv): full AcroForm leaf-field dumps (`/FT`/`/Rect`/on-states/
  `/MaxLen`/pre-set `/V`), page-text extraction, text-position visitor runs, same-y `/Btn` clustering.
  **Additionally downloaded and verified the two missing Form 8283 revisions** (the spec's recon 404'd on
  them): `irs.gov/pub/irs-prior/f8283--2014.pdf` and `f8283--2023.pdf`. Verified every repo claim against
  source: `btctax-adapters/src/tax_tables.rs`, `btctax-core/src/tax/{tables,se,compute}.rs`,
  `btctax-core/src/forms.rs`, `btctax-forms/src/{lib,map,verify,fill8949,schedule_d,schedule_se,form1040}.rs`,
  `btctax-forms/forms/2025/*.toml`, `btctax-cli/src/cmd/admin.rs`.

## VERDICT: **BLOCKED — 3 Critical / 5 Important / 5 Minor / 2 Nit**

The per-year recon that WAS done is largely right — Box C/F, 14 rows/part, 1040 line 13 (2017, no DA
question) vs line 7 + DA (2024), the old short+long 2017 SE, line 12 = 10+11 — all re-verified TRUE against
the PDFs. But the spec has three load-bearing holes. **(C1) btctax cannot compute a 2017 Schedule SE at
all** — the bundled tax tables are TY2024/2025/2026 only, `compute_se_tax` requires the year's
`TaxTable.ss_wage_base`, and the spec's scope never adds a TY2017 table, so the promised 2017 "full packet"
is unachievable as scoped (the other four 2017 forms ARE computable — this gates the SE leg, not the whole
sub-project, but the spec must either add the table or descope the leg). **(C2) the 2024 Form 1040 breaks
the SP2 DA-question oracle**: the top-most same-y `{/1,/2}` `/Btn` pair on 2024 page 1 is the FILING-STATUS
row, not the DA pair, so the shipped map-independent guard fails closed on a *correct* 2024 map — the 2024
1040 leg cannot ship under "the engine does not change". **(C3) the entire 8283 row of the spec's
"recon-verified" facts table is unverified** — the downloaded files are IRS 404 HTML pages, yet the spec
asserts "All 9 forms re-verified fillable XFA hybrids"; this review resolved the facts (Rev. 12-2014 for
TY2017, no digital-asset box, "j Other" exists; Rev. 12-2023 for TY2024, "k Digital assets" exists) and they
must be folded. Beyond those: the 2017 forms split every money amount into DOLLARS + CENTS field pairs, the
2017 SE blank ships factory-pre-filled `/V` values that trip `no_unmapped_filled`, 2017 Schedule D has no
QOF question (the map schema requires one), and the 8949 oracle's table token doesn't match either new
year's PDF — each a concrete engine delta the "SP3 adds per-year DATA only" framing hides.

---

## Critical

### C1 — [★ gates the 2017 SE leg + the spec's "full packet" goal] btctax ships NO TY2017 tax table; `compute_se_tax` requires one; the spec's scope never adds it. As written, `--tax-year 2017` can never produce a Schedule SE.

**Spec claim (lines 10–12):** "Extend `export-irs-pdf` to accept `--tax-year 2017` … for the full packet
(8949, Schedule D, 8283, Schedule SE, 1040-cap-gains)". **Scope (lines 66–69):** `btctax-forms` data + the
`export-irs-pdf` per-year dispatch; "NO engine-logic change".

**Evidence:**
- `crates/btctax-adapters/src/tax_tables.rs:74-76` — the bundled tables are exactly
  `by_year.insert(2024, …); insert(2025, …); insert(2026, …)`. The module doc (lines 1–47) covers Rev. Proc.
  2023-34 / 2024-40 / 2025-32 and says "TY2027+ are omitted". **There is no TY2017 entry** (the only "2017"
  hits in the file are bracket dollar values like `201775`).
- `crates/btctax-core/src/tax/se.rs` — `compute_se_tax(…, table: &TaxTable, …)` uses `table.ss_wage_base`
  for the §1401(a) cap; there is no table-free path.
- `crates/btctax-cli/src/cmd/admin.rs` (`export_irs_pdf`, the SE block): `tables.table_for(tax_year)` →
  `None` for 2017 → `se_computed = None` → `se_income_without_profile = true` → Schedule SE **silently
  skipped with a note** for every 2017 vault with SE income. The command cannot fabricate the form — good —
  but the spec's goal is then unreachable.
- `crates/btctax-core/src/tax/compute.rs:260` — `report --tax-year 2017` is likewise
  `NotComputable(TaxTableMissing)` today (context: the 2017 vault's tax REPORT has never been computable;
  only the table-free projections work).

**What this does NOT gate (say it precisely, not maximally):** `form_8949` (forms.rs:99), `schedule_d`
(forms.rs:172), and `form_8283` (forms.rs:356) are pure over `LedgerState` — no `TaxTable` anywhere — and
the bundled price CSV has all 365 daily closes for 2017 (`btctax-adapters/data/btc_usd_daily_close.csv`).
So 8949 / Schedule D / 8283 / 1040-cap-gains for 2017 are fully computable today. **The sub-project is not
moot; the Schedule SE leg is.**

**Fix (pick one, in the spec, before implementation):**
1. **Add a TY2017 `TaxTable` to `btctax-adapters` scope** — encoded verbatim from **Rev. Proc. 2016-55**
   (ordinary brackets — note the pre-TCJA 7-rate structure 10/15/25/28/33/35/39.6%, which the
   `Vec<OrdinaryBracket>` shape holds fine; §1(h) LTCG breakpoints — in 2017 these are the tops of the 15%
   and 39.6% brackets, still expressible as `max_zero`/`max_fifteen` dollar amounts; §2503(b) annual
   exclusion $14,000; §2010(c)(3) $5,490,000) + the **SSA 2016-10-18 wage base $127,200** (which the 2017
   SE form itself preprints on line 7 — see I2, a free cross-check KAT). Same primary-source verification
   bar as ty2024/ty2025/ty2026. This is a data-only adapters change but it is OUTSIDE the spec's stated
   scope ("btctax-forms + the export-irs-pdf dispatch") and MUST be named, planned (T2 prerequisite), and
   KAT'd. Bonus: it un-gates `report --tax-year 2017` for the demo vault.
2. Or **explicitly descope the 2017 Schedule SE** (packet = 4 forms for 2017; loud note reusing the existing
   `se_income_without_profile` discriminator) and correct the Goal text.

Option 1 is strongly preferred — the ReadOnly TY2017 vault is a miner's ledger; a 2017 packet without
Schedule SE is missing its largest tax line, and the spec's own T2 end-to-end would demonstrate the gap.

### C2 — The 2024 Form 1040 breaks the SP2 map-independent DA-question oracle: the top-most same-y `{/1,/2}` pair on page 1 is the FILING-STATUS row, not the Digital-Asset pair. A CORRECT 2024 map fails closed; the 2024 1040 leg cannot ship as "data-only".

**Spec claim (lines 32–33):** 2024 → "the DA question present (SP2's C4 YES-iff-activity logic applies, 2024
field ids)"; (lines 47–49, 85): the oracle re-derives per year "provided the per-year logical-sequence
config … is supplied as data".

**Evidence (measured, `f1040-2024.pdf` page 1, all `/Btn` widgets grouped by center-y):**

| y | widgets | on-states | what it is |
|---|---|---|---|
| 602 | `c1_1[0]` x=504, `c1_2[0]` x=540 | {/1,/1} | Presidential campaign You/Spouse (rejected — states not {1,2}) |
| **588** | **`c1_3[0]` x=103, `c1_3[0]` x=369** | **{/1,/2}** | **FILING STATUS row 1 — matches the predicate** |
| **487** | **`c1_5[0]` x=504, `c1_5[1]` x=540** | **{/1,/2}** | **the actual Digital-Asset Yes/No pair** |

`verify.rs::topmost_yes_no_pair` (verify.rs:406-455) selects "the top-most row of EXACTLY two `/Btn`
widgets whose combined on-states are exactly {1,2}" — that is y=588, the filing-status pair. Then
`form1040.rs:118-130` hard-errors when the map's `da_yes`/`da_no` (correctly `c1_5[0]`/`c1_5[1]`) don't
equal the predicate's result. **Every 2024 1040 export fails closed at runtime.** (The 2025 form happens to
have a layout where the DA pair is top-most — SP2 measured it; 2024 does not. That is precisely the class of
per-year structural break the spec's "oracle re-derives per year" hand-waves over: the same-y-pair predicate
is a heuristic, not map data.)

**Fix:** the spec must OWN a predicate revision as engine work (SP2-C3 style), e.g. require the pair's two
widgets to be horizontally ADJACENT (the DA members are 36 pt apart at x=504/540; the filing-status members
are 266 pt apart at x=103/369 — a `Δx ≤ ~60pt` guard separates them cleanly and still holds on 2025's
c1_10 pair at x=518/554), or require both members in the right-hand fifth of the page (x > 480). KATs: the
predicate selects `c1_5` on the 2024 blank AND still selects `c1_10` on 2025 (regression); keep the Yes/No
swap fault-inject per year. Whatever variant is chosen, it must be verified against BOTH years' PDFs at spec
level, not deferred to extraction.

### C3 — The spec's 8283 facts are asserted as recon-verified but were NEVER verified: both downloaded "PDFs" are IRS 404 HTML pages. The applicable revisions are unnamed. (This review resolves the facts; fold them.)

**Spec claims:** line 12 — "All 9 forms re-verified fillable XFA hybrids"; facts table line 22 — 8283 "Rev.
applicable to 2017 / Rev. applicable to 2024"; lines 40–44 — bundle "the rev current in early 2018 / early
2025", "confirm the 'k Digital assets' property box EXISTS in that revision".

**Evidence of the hole:** `file f8283-2017.pdf f8283-2024.pdf` → "HTML document" (both are the irs.gov 404
page, title "404 | Internal Revenue Service"). No 8283 of either era was ever opened. A tax-critical spec
asserting verification that did not happen is exactly what this workflow's independent review exists to
catch (SP1-C1 precedent: the unverified XFA assumption was a Critical).

**Root cause + resolved facts (verified THIS review — fold into the spec):**
- The recon fetched revision-dated URLs by TAX YEAR: `irs-prior/f8283--2017.pdf` and `f8283--2024.pdf` are
  **HTTP 404 because no 2017- or 2024-dated revisions exist** (8283 revisions are sparse). The correct
  archive scheme is `https://www.irs.gov/pub/irs-prior/f8283--YYYY.pdf` where YYYY is a **revision** year.
- **TY2017 → Form 8283 (Rev. December 2014)** (`f8283--2014.pdf`): pages=2, **XFA hybrid confirmed**
  (`/XFA` present, 154 AcroForm leaf fields — the XFA-drop engine applies). **NO digital-asset text
  anywhere.** Section B Part I line 4 property-type boxes run **a–j** and **"j Other" EXISTS** — the spec's
  "closest box, likely 'Other', with a printed note" is therefore workable (verified, no longer "likely").
  **OLD part numbering**: Section B **Part II = Taxpayer (Donor) Statement, Part III = Declaration of
  Appraiser, Part IV = Donee Acknowledgment** (n.b. this matches the OLD numbering `donation.rs`'s
  doc-comments still cite — right for 2017, wrong for 2025; per-revision map data). Section A has **5 rows
  (Line1A–Line1E)** (2025: 4) — overflow capacity is per-revision data. Legacy field conventions throughout:
  `topmostSubform[0].Page1[0].Table1[0].Line1A[0].Row1[0].c1_01_0_[0]` with on-state `/Yes`, and Yes/No
  pairs like `p1-cb1[0]`/`p1-cb1[1]` with on-states `/1`/`/0` — a wholesale fresh map extraction, nothing
  reusable from the 12-2025 map.
- **TY2024 → Form 8283 (Rev. December 2023)** (`f8283--2023.pdf`): XFA hybrid confirmed; **"k Digital
  assets" EXISTS** (root subform is `Form8283[0]`, checkbox groups `Lines2a-c[0].c1_6[*]`/`Lines2d-h[0]…`
  with sequential on-states — same family as 12-2025's `Lines2i-l[0].c1_6[2]`=/11 but re-pin at extraction);
  part numbering **III/IV/V — same as Rev. 12-2025**, so the SP2 fill/blank scope table ports.
- **Revision-selection rule:** cite the Form 8283 instructions' rule — revisions are keyed to **tax years**
  ("use prior revisions of the form for earlier tax years"), not to the filing date. The spec's "the rev
  current in early 2018 / early 2025" phrasing lands on the same two revisions here (12-2014, 12-2023) but
  states the wrong rule — a 2017 return filed/amended in 2026 still uses the 2017-era revision under the
  instruction rule. Fix the wording; key the bundled dirs/maps by revision string ("12-2014", "12-2023") as
  the spec already intends.
- **Scope-out is NOT warranted:** a >$5,000 BTC donation in 2017 still required Section B + a qualified
  appraisal; "j Other" + the printed note is the correct, honest path. Keep 8283-2017 in scope.

**Fix:** replace the 8283 row + lines 40–44 with the named, verified revisions above; correct the "All 9
forms re-verified" sentence (it was 8 of 10 artifacts; with the two revisions verified here it becomes
true — count them explicitly); add the URL scheme so implementation bundles the right files; KATs name the
revisions (`rev_12_2014_8283_has_no_digital_asset_box_uses_j_other_with_note`,
`rev_12_2023_8283_digital_asset_box`).

---

## Important

### I1 — "The engine does not change; SP3 adds per-year DATA" is materially false: the 2017 forms split EVERY money amount into a DOLLARS field + a CENTS field, which the map schema, `fmt_money`, and the flat-oracle placements cannot express.

**Evidence:**
- `fSE-2017.pdf`: every amount line is a field PAIR — e.g. line 1a = `f2_3[0]` [477,624,552,636] +
  `f2_4[0]` [556,624,576,636] `maxlen=3`; the whole long form (page 2) and short form (page 1) follow the
  pattern (all cents columns are the `maxlen=3` fields at x≈556–576 / 434–452).
- `f1040-2017.pdf`: **79 cents fields** (`maxlen=3`) — line 13 = dollars `f1-_51[0]` [482,336,554,348] +
  cents `f1_52[0]` [554,336,576,348]. (2024 forms: ZERO cents fields; `schedD-2017.pdf`: zero — single
  fields; this is a 2017-SE + 2017-1040 problem only.)
- Repo: `map.rs::ScheduleSeMap` holds ONE `String` fqn per line; `lib.rs:61` `fmt_money` renders one
  string; `schedule_se.rs:67-87` builds one write + one `FlatPlacement` per line with strictly-descending
  ordinals — a dollars+cents twin at the SAME center-y cannot both join the descent (strict `<` breaks), and
  the cents twin left unmapped would... be written by nobody, but a dollars-only write of "1234.56" into the
  dollars box is a MISRENDERED amount on an official form.
- Also hard-coded per-2025 engine surface the spec's "per-year branch" must actually parameterize:
  `lib.rs:40-47` `SUPPORTED_YEAR`/`require_year`; every `fill_*` calls `*Map::ty2025()` and a 2025 PDF const
  (`schedule_se.rs:89` `pdf::SCHEDULE_SE_PDF_2025`); `schedule_se.rs:29` `SE_CLUSTERS = [(410,482),(504,576)]`
  (2017 long-form clusters are mid [355,452] / amount [477,576] — different values) and `schedule_se.rs:86`
  pins **page 0** while the 2017 long form lives on **page index 1**; `form1040.rs:27` `F1040_CLUSTERS =
  [(504,576)]` (2017 line-13 dollars column is [482,554]).

**Fix:** the spec must own a T0-style engine task: (a) a money-cell representation of `fqn` OR
`{dollars_fqn, cents_fqn}` in the map schema, with the fill writing split dollars/cents (comma-grouped
dollars, zero-padded 2-digit cents) and the oracle treating the twin as ONE logical placement (dollars field
carries the column/descent geometry; cents field joins the allowed/no-unmapped set); (b) a per-year
`FormConfig` (PDF bytes, map, clusters, page indexes, SE chain page) driving each `fill_*` — the spec's
"prefer a per-year config struct read from the map over `if year==` ladders" line is right but must be
scoped as ENGINE work, not data. The 2017 SE chain itself maps cleanly (verified: 2=`f2_7`@540, 3=`f2_9`@528,
4a=`f2_11`@516, 4c=`f2_15`@468, 6=`f2_21`@420, 8a=`f2_25`@360(mid), 8d=`f2_31`@324, 9=`f2_33`@312,
10=`f2_35`@300, 11=`f2_37`@288, 12=`f2_39`@276, 13=`f2_41`@240(mid) — strictly descending, so the ordinal-y
leg of the oracle DOES hold on 2017 once the twins are modeled).

### I2 — The 2017 Schedule SE blank ships FACTORY-PRE-FILLED `/V` values; `no_unmapped_filled` fails on a correct fill, and the SP2 1-pt-spacer exclusion does not cover them.

**Evidence:** the blank `fSE-2017.pdf` AcroForm carries `/V` on four fields:
`Line7Dollars[0]`='127,200', `Line7Cents[0]`='00', `Line14Dollars[0]`='5,200', `Line14Cents[0]`='00'
(the preprinted §230 wage base and the optional-method maximum). These are FULL-SIZE fields
(`Line7Dollars` rect [477,396,552,408] — 75 pt wide), so the SP2 "spacer = rect width < 2pt" exclusion
(SP2 spec §oracle item 4; cf. the 2025 SE's 1-pt `f1_13` [575,384,576,396]) does not exclude them, and
`verify.rs::no_unmapped_filled` ("every filled field must be in `allowed`", verify.rs:225-235) will reject
the serialized output — the fields carry values but are not map targets.

**Fix:** per-year map data: an `allowed_preprinted` set with the EXACT expected values, asserted UNCHANGED
at read-back (never cleared by the fill, never treated as our write). This doubles as a free KAT: the
2017 form's own preprint '127,200' must equal the TY2017 `TaxTable.ss_wage_base` added under C1 — a
primary-source cross-check the spec should name (`ty2017_se_line7_preprint_matches_table_wage_base`).

### I3 — 2017 Schedule D has NO QOF question (QOF is 2018+), but `ScheduleDMap` REQUIRES `qof_yes`/`qof_no` and `fill_schedule_d` always writes "No". The spec never mentions QOF.

**Evidence:** `schedD-2017.pdf` — zero occurrences of "opportunity fund" in either page's text; its only
`/Btn`s are the page-2 lines 17/20/22 Yes/No fields (`c2_01_0_`… with `/Yes`/`/No` on-states — inside
SP1's 17-22 scope-out). `schedD-2024.pdf` HAS the QOF pair (`Page1[0].c1_1[0]`=/1, `c1_1[1]`=/2 @ y≈662 —
same shape as 2025). Repo: `map.rs:269-272` — `qof_yes`/`qof_no` are non-optional fields of
`ScheduleDMap`; `schedule_d.rs:111-119` — "Always answer the QOF question — No". A 2017 map literally
cannot be written against the current schema, and the fill logic must become conditional — another
engine-not-data delta.

**Fix:** make the QOF pair `Option<CheckChoice>` in the schema; write it only when present; spec the fact in
the per-year table (2017: none; 2024: present, `c1_1[0]/[1]`, on `/1`//`2`); KATs
`ty2017_schedule_d_has_no_qof_question` (absent from map AND no button written) +
`ty2024_schedule_d_answers_qof_no` + 2025 regression.

### I4 — The 2017 1040's produce/skip semantics are unspecified: with no DA question, the current gate (`da_yes`) + unconditional DA write would emit an EMPTY 1040 for an income-only 2017 year (or error on a missing field). "Skip it entirely" is about the DA field, not the form logic.

**Evidence:** `form1040.rs:63-66` — the whole form is skipped iff `!inputs.da_yes`; when produced, the DA
YES write is unconditional (form1040.rs:72-78) and line 7a is filled only when Schedule D is active with
line 16 ≥ 0. Port to 2017: a mining-only 2017 year has `da_yes == true` (income_recognized) and
`schedule_d_active == false` → the form is produced with... nothing (no DA field exists to write; 7a-analogue
line 13 blank) — an official 1040 with zero filled cells, or a hard error if the code writes a map field
that doesn't exist. Spec lines 31–34 only say "NO digital-asset question exists (skip it entirely; the map
has no DA field)" and assert I★1 "applies both years" — it does not, unmodified: `da_yes` doubles as the
form's existence condition, and that role has no 2017 meaning.

**Fix:** spec the 2017 rule explicitly — **produce the 2017 1040 iff line 13 will receive a value**
(Schedule D active ∧ line 16 ≥ 0; active-zero → "-0-"); net loss → skip + the §1211 notice (the 2017
Schedule D line-21 analogue is the filer's); inactive → skip + note. Also state (both years) that the
"Capital gain or (loss) … If not required, check here" CHECKBOX stays untouched (Schedule D IS attached):
2017 `c1_11[0]` @[441,338] (states `/1`); 2024 `c1_23[0]` @[465,168] — the 2024 analogue of SP2's "7b
checkboxes stay unchecked" rule, which the spec ports by name ("SP2's C4/I★1 logic") without noticing 2024
has a line-7 checkbox, not a 7b pair. KATs: `ty2017_1040_skipped_when_schedule_d_inactive` (mining-only
year), `ty2017_1040_line13_checkbox_untouched`, `ty2024_1040_line7_checkbox_untouched`.

### I5 — The 8949 oracle cannot derive its bands on EITHER new year: `F8949_TABLE_TOKEN = "Table_Line1_Part"` matches nothing in the 2017/2024 PDFs (their grid subform is `Table_Line1[0]`).

**Evidence:** `verify.rs:53` — `pub const F8949_TABLE_TOKEN: &str = "Table_Line1_Part";`
`derive_bands` (verify.rs:73-81) collects only fields whose fqn contains the token and hard-errors
("no data-grid widgets found") when none match. Both `f8949-2017.pdf` and `f8949-2024.pdf` name the grid
`topmostSubform[0].Page{1,2}[0].Table_Line1[0].Row{n}[0].f*` (14 rows × 8 fields — verified), vs 2025's
`Table_Line1_Part1[0]`/`Table_Line1_Part2[0]`. Every 2017/2024 8949 fill would fail closed at the
read-back despite a correct map.

**Fix:** move the table token into the per-year 8949 map (`table_token = "Table_Line1"` for 2017/2024,
`"Table_Line1_Part"` for 2025) — one line of schema + threading, but it must be in the spec's per-year
config enumeration (it is exactly the "8949 columns" class of config line 48 gestures at without naming).
The rest of the 8949 oracle (column-x from `Row{n}` grouping, ordinal-y, totals-below-grid) re-derives
cleanly on the 2017/2024 geometry — verified 8 columns per row, consistent counts, 14 rows.

---

## Minor

### M1 — Field-count claims in the facts table are wrong (and hide a favorable fact about 2024).
Measured: `fSE-2017.pdf` = **67** AcroForm leaf fields (spec: "70"); `fSE-2024.pdf` = **27** (spec: "32",
twice — and SP2's own C3 recon said 27 for the 2025 form). More useful than either number: **the 2024 SE
field SET is name-identical to 2025's** (verified `f1_*`/`c1_*` diff — empty), so the 2024 SE map is a
near-copy with re-verified geometry and the ty2024 wage base $168,600 (already bundled,
tax_tables.rs:218). Fix the numbers; state the congruence — it derisks T1.

### M2 — Pin the verified 8949 per-year data in the spec (all confirmed, several details differ from 2025 in ways the table should carry):
- Box fields: **3 checkboxes per part** (not 2025's 6): Box C = `Page1[0].c1_1[2]` on-state **`/3`**;
  Box F = `Page2[0].c2_1[2]` on-state **`/3`** — identical field ids in 2017 and 2024. Labels verified:
  "(C)/(F) … not reported to you on Form 1099-B"; **no digital-asset box on either year's form** (text
  negative both years). Core `Form8949Box::{C,F}` (forms.rs:114-116) semantics MATCH these years — and
  since `fill8949.rs` takes the box from `part.box_field`/`box_on` map data (fill8949.rs:86-94), nothing
  in core changes; the SP1 `box_needs_review` broker-warning stays valid.
- Totals rows differ per year: **2017 = 4 fields** `f1_115..f1_118` / `f2_115..118` (columns d,e,g,h — NO
  (f)-column spacer); **2024 = 5 fields** `f1_115..f1_119` (spacer `f1_117` @[403,60,446,72]); both ≠
  2025's `f1_91..95`. The map's 4-key totals struct fits both — just extraction data, but the spec's
  "totals-row fields … shift" bullet should carry the numbers now that they're measured.
- 14 rows × 8 fields per part verified both years; `rows_per_page` is already map data
  (`forms/2025/f8949.map.toml:3`) so the 14-row overflow reuses SP1's machinery unchanged.
- Schedule D routing verified: Box C totals → line 3, Box F → line 10, on BOTH years' page-1 text
  (2017 and 2024 wordings both say "Box C checked"/"Box F checked"; no I/L mention pre-2025). Lines
  3/7/10/15/16 all exist as single-amount fields (2017 Schedule D has NO cents split — measured 0
  `maxlen=3` fields).

### M3 — T2's end-to-end depends on the out-of-repo ReadOnly vault; there is no in-repo TY2017 fixture.
`grep -rln 2017 crates/` hits only the price CSV and tax_tables.rs comments — no test fixture. The
"ReadOnly TY2017 demo" is the user's real data kept outside the repo by design (README.md:104). The plan
should add a SYNTHETIC in-repo TY2017 fixture (SP1's demo shape: 26 ST + 28 LT legs ⇒ 2+2 overflow pages
at 14 rows/part; plus SE income ≥ $400 and one >$5k donation to exercise all five forms), so the KATs and
golden shas run in CI; the ReadOnly run stays a documented manual step.

### M4 — Per-year notice/doc text needs its own line-name sweep (the SP2 "says 7a, never 7" discipline).
Verified targets: 2017 SE line 12 text routes to **"Form 1040, line 57"** (not the modern Schedule 2);
1040 notices must say **"line 13"** (2017) / **"line 7"** (2024); the 8283 notices must use the
per-revision part numbers (2017: Taxpayer stmt = Part II, Appraiser = Part III, Donee = Part IV — C3).
Man page/README already planned (spec line 68-69) — add the QOF-less 2017 Schedule D and the line-name
table to it.

### M5 — Pin the verified 1040 field ids (and one booby-trap) in the facts table.
2024: line 7 amount = `Page1[0].Line4a-11_ReadOrder[0].f1_52[0]` @[504,162,576,174] (label y=164.3
measured inside the band); DA Yes = `c1_5[0]` on `/1` @x=504, No = `c1_5[1]` on `/2` @x=540 (y≈487).
2017: line 13 dollars field is literally named **`f1-_51[0]`** — an IRS field-naming glitch with a hyphen
(`topmostSubform[0].Page1[0].f1-_51[0]`) — pin it VERBATIM so map extraction (or a well-meaning reviewer)
doesn't "fix" it to `f1_51`; cents = `f1_52[0]` @[554,336,576,348].

---

## Nit

### N1 — "All 9 forms" doesn't add up as written.
5 forms × 2 years = 10 artifacts (or 8 + two 8283 REVISIONS). After C3's fold, state the count explicitly
(e.g. "all 10 bundled artifacts — 8 year-dated forms + 8283 Rev. 12-2014 + Rev. 12-2023 — verified
XFA-hybrid with complete AcroForm layers"). All 8 year-dated PDFs re-verified `/XFA`-present this review;
the two revisions likewise (C3).

### N2 — Facts-table polish: "Schedule D line 1b/8b or 3/10 wording per year — verify at extraction" is now verified (3/10 both years — M2); the SE row should say "67 fields (short §A p1 + long §B p2, split dollars/cents)" per M1/I1. Carry the measured numbers instead of the verify-later hedges where this review already measured them.

---

## Answers to the review charter's direct questions

1. **Does btctax compute 2017 (and 2024) taxes?** 2024: YES — full `ty2024` table (Rev. Proc. 2023-34,
   wage base $168,600). 2017: **NO table exists** → Schedule SE uncomputable (C1) and `report --tax-year
   2017` is `NotComputable(TaxTableMissing)`; but 8949/Schedule D/8283/1040-cap-gains are table-free and
   fully computable (prices cover 2017). The SE leg — not the sub-project — is gated; the spec must add the
   TY2017 table (preferred) or descope the leg.
2. **Box taxonomy:** verified — Box C/F, `c1_1[2]`//`3` + `c2_1[2]`//`3`, both years; no DA box; core
   `Form8949Box::{C,F}` matches; Schedule D 3/10 wording verified (M2).
3. **2017 SE:** 67 fields, short §A (page 1) + long §B (page 2); btctax's SE data maps 1:1 onto the §B
   chain (I1 lists the field-by-field mapping, strictly descending) — the LONG-form decision HOLDS (and is
   mandatory whenever W-2 wages exist, per the form's flowchart); line 12 = "Add lines 10 and 11" verified
   (the 0.9% addl Medicare was already a Form 8959 item in 2017 — off-form, matching SP2-C1's split); the
   2017-specific traps are the dollars/cents split (I1), the factory-preprinted `/V` on lines 7/14 (I2),
   and page-index 1.
4. **Per-year 1040:** 2017 = line 13 (`f1-_51[0]`+`f1_52[0]`), NO DA/virtual-currency text anywhere
   (verified negative), line-13 checkbox untouched; "skip the DA field" is right but the form-existence
   rule must be respecified (I4). 2024 = line 7 (`Line4a-11_ReadOrder[0].f1_52[0]`) + DA pair `c1_5` —
   but SP2's map-independent DA guard breaks on 2024 (C2); the C4 YES-iff-activity SEMANTICS port fine.
5. **8283 revisions:** TY2017 → Rev. 12-2014 (no DA box; "j Other" exists — closest-box + note is right;
   old part numbering II/III/IV; 5 Section-A rows); TY2024 → Rev. 12-2023 ("k Digital assets" exists;
   parts III/IV/V as 2025). Do NOT scope 8283 out for 2017 (C3). Downloads: use
   `irs-prior/f8283--{2014,2023}.pdf`.
6. **Oracle generalization:** 8949 bands break on the table token (I5); SE ordinal-y/column legs hold on
   2017 given per-year clusters + page + twin-modeling (I1/I2); the 1040 same-y-pair predicate breaks on
   2024 (C2). "Re-derives per year provided per-year config" is true only after the C2/I1/I2/I5 engine
   deltas are owned.
7. **Regression/architecture:** per-year `FormConfig` over `if year==` is the right call but must be
   specified as engine work with a schema (I1); the 2025 suite (`btctax-forms/tests/{kats,overflow,sp2}.rs`
   + goldens) guards regressions — add the 2025-oracle-still-selects-`c1_10` KAT under C2. **Split
   recommendation:** keep SP3 whole but restructure the plan as T0 (engine: split-money cells, per-year
   config incl. table token + clusters + pages, DA-pair predicate fix, preprint allowlist, optional QOF) →
   T1 (2024 — genuinely near-2025: SE fieldset identical, single-field amounts, QOF present) → T2 (2017 +
   the TY2017 tax table + Rev. 12-2014 8283). If the folded spec's T2 grows past a screen, promote 2017 to
   its own gated sub-project (SP3b) rather than thinning the ceremony — 2017 carries all the engine-touching
   weight.

**Round-1 disposition:** BLOCKED at 3C/5I. All findings have concrete, evidence-backed fixes; nothing here
kills the sub-project — 2024 is close, 2017 is real work the spec undersold as "data". Fold and re-review.
