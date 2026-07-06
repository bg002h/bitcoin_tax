# SPEC — official IRS PDF form-fill, sub-project 2 (Form 8283 + Schedule SE + Form 1040 cap-gains, TY2025)

**Source baseline:** `main` @ `99f26ca` (branch `feat/irs-form-fill-sp2`). **Review status: R0 round 2 folded
(r1 4C/4I on Fable; r2 0C/1I/3M/2N on Opus — all r1 folds PDF-verified correct, the C3 oracle proven sound).
Awaiting R0 round 3 (changed lines only).** Reviews: `reviews/R0-spec-irs-form-fill-sp2-round-{1,2}.md`. SP2 of task #45; builds on the shipped SP1 engine. **Every
load-bearing fact below was PDF-verified by the Fable R0 (fill-experiment per form).**

## Goal (SP2)
Extend `export-irs-pdf --tax-year 2025` to ALSO populate **Form 8283** (Rev. 12-2025; BTC donations),
**Schedule SE** (self-employment tax), and the **capital-gains cells of Form 1040** — reusing the SP1 fill
primitive, honestly (partial-scope, conditional, correct lines).

## Reuse vs NEW engine work
- **Reuses cleanly:** the fill primitive (drop `/XFA`, set `/V`/`/AS`, NeedAppearances, strip dates/`/ID`),
  `no_unmapped_filled` (form-agnostic), determinism, `stamp_draft_watermark`, the attestation gate,
  `merge_copies` (overflow), the per-year TOML-map + golden pattern. All 3 forms re-verified XFA-hybrid with a
  complete AcroForm layer → XFA-drop works.
- **[★ R0-C3] NEW: the geometric read-back must be REDESIGNED per form — it does NOT reuse.** `verify.rs`
  derives its oracle from `Row{n}` grid subforms; NONE of these forms fits: 8283 rows are `Row1A/1B/1C/1D` (split
  across two tables, heterogeneous widths), Schedule SE has NO table (27 flat leaf fields), Form 1040's only
  grid is `Table_Dependents` (irrelevant; the DA checkbox is `Geo::Check`, geometry-exempt). So the spec OWNS a
  per-form oracle built from structure derivable from the blank PDF alone (no map trust):
  1. **column-x membership** — assert each written value's center-x sits in its logical column's geometric
     cluster (SE amount col x≈[504,576]; SE mid col x≈[410,482]; catches right↔mid swaps, e.g. SE 12 vs 13);
  2. **ordinal-y descent** — for a logically ordered line sequence (SE 2,3,4a,4c,6,8a,8d,9,10,11,12; 8283 rows
     A→D), assert the written widgets' center-y are strictly descending in logical order (a swap breaks
     monotonicity). **[R0-M1] 8283 ordinal-y must be PER-COLUMN** — Section A rows A–D split across two
     y-bands / two tables, so the descent is asserted within each column's own field set, not globally;
  3. **same-y pair predicate** for the DA question — the Yes box is the LEFT member of the top-most same-y
     `/Btn` pair on page 1 with on-states {`/1`,`/2`} (measured: `c1_10[0]`=/1 @[518,497], `c1_10[1]`=/2 @[554,497]);
  4. **spacer + page pins** — exclude 1-pt preprinted-constant spacer fields (rect width <2pt — SE line 7
     `f1_13`=[575,384,576,396], line 14 `f2_1`); assert correct page membership. Keep `no_unmapped_filled` + the
     swap-two-map-entries ⇒ RED fault-inject against THESE pins + `map_2025_matches_bundled_pdf_fieldset` per
     form. **[R0-M2] the column-x logical→cluster table is hand-pinned, so the fault-inject KAT must cover BOTH
     a cross-column swap (caught by column-x, e.g. SE 12↔13) AND a same-column swap (caught by ordinal-y, e.g.
     SE 10↔11)** — one of each proves both oracle legs bite.

## Form 8283 (Rev. 12-2025) — `fill_form_8283`
- **[R0-I3] Revision facts:** Section A = 4 rows (A–D); Section B Part I = 3 rows (A–C); parts renumbered —
  Taxpayer stmt = **Part III**, Appraiser Declaration = **Part IV**, Donee Acknowledgment = **Part V** (core
  `DonationDetails` doc-comments cite OLD numbering — donation.rs; the map fixes it). **[★] Section B line 2
  "k Digital assets" property-type checkbox = `Lines2i-l[0].c1_6[2]`, on-state `/11`** (the Box-I/L analogue for
  8283 — MUST be checked for BTC; do NOT check "l Other"/"f Securities"). Date-acquired = **"(mo., yr.)"**
  format, NOT SP1's MM/DD/YYYY. Header block (originally-reported entity, pass-through) stays blank.
- **[R0-I4] Fill vs blank (a scope table + a 1040-style LOUD notice):** FILL from `form_8283()`/`DonationDetails`
  — donee name/EIN/address (Part V identity), donee/date/description/FMV/cost per row, appraiser
  identity name/address/TIN/PTIN (Part IV identity). **Leave BLANK (another party's declaration):** the Part III
  taxpayer signature, the Part IV appraiser SIGNATURE/date, the Part V donee ACKNOWLEDGMENT (receipt date,
  "unrelated use?" `c2_4`, authorized signature/title/date), the Part II restriction questions `c2_1..c2_3`.
  A Section-B 8283 without a signed Part IV/V is NOT filing-ready — say so; escalate the notice when any row has
  `needs_review==true` (forms.rs:426).
- **Conditional + overflow:** written only when donations exist. One row per `RemovalLeg` (forms.rs:333) → a
  multi-lot donation overflows 4 Section-A / 3 Section-B rows immediately → `merge_copies` ("Attach one or more
  Forms 8283" sanctions it); per-copy renamed fields. KATs: `form_8283_section_b_checks_digital_assets_box`,
  `form_8283_overflow_pages`, `each_8283_copy_renamed_no_shared_value`, `form_8283_leaves_other_party_decls_blank`.

## Schedule SE (unified 2025) — `fill_schedule_se`
- **[★ R0-C1] Line 12 = lines 10 + 11 (SS + regular Medicare ONLY).** `SeTaxResult.total = ss + medicare +
  addl` (se.rs:160) INCLUDES the 0.9% Additional Medicare Tax — which is a **Form 8959** item, NOT on Schedule
  SE (the form: "12 … Add lines 10 and 11"; the core already knows — se.rs:42 "line 13 = SS + regular Medicare
  only"). Line 12 := `ss + medicare`; line 13 := `deductible_half` (consistent by construction). **When
  `addl>0`, print a loud advisory** ("Additional Medicare Tax $X is a Form 8959 item, not on Schedule SE").
  KATs: `schedule_se_line12_equals_ss_plus_medicare`, `schedule_se_line12_excludes_addl_medicare` (the $300k
  c1_lock golden: line 12 = 29,870.85 NOT 30,564.30).
- **[R0-I1] Full arithmetic chain (self-consistency, like SP1's Sch D 7/15):** fill 2 (net_se), 3(=2), 4a(base),
  4c(=4a), 6(=4c), **8a (W-2 SS wages — needs `session.tax_profile(y).w2_ss_wages`, NOT in `SeTaxResult` —
  thread it in via the fill signature)**, 8d(=8a), **9(= line 7 − 8d, where line 7 is the `ss_wage_base`
  $176,100 constant — [R0-M3] thread `ss_wage_base` in too, not just `w2_ss_wages`)**, 10(ss), 11(medicare),
  12, 13(=12×50%, MID-column `f1_22`). Explicit BLANK set: A-checkbox, 1a/1b, 4b, 5a/5b, 8b/8c, Part II. If 8a≥$176,100, follow the form
  ("skip 8b–10"; 9/10 blank, matching ss==0).
- **[★ R0-I2] $400 floor:** the form line 4c says "if less than $400, STOP; you don't owe SE tax", but
  `compute_se_tax` has no $400 threshold (se.rs:109 → None only when net_se==0). **SP2 skips Schedule SE when
  `base < $400`** + a printed note + KAT `schedule_se_skipped_below_400_floor`. FOLLOWUP: whether
  `compute_se_tax` itself should return None below the floor (core change, out of SP2 scope).
- **[R0-M2] Conditional discriminator:** `se_result==None` has 3 causes (no SE income / no profile / no table,
  admin.rs:83); use the `se_net_income` discriminator (se.rs:55) so a miner WITH SE income but no stored profile
  gets a NOTE, not a silent skip. **[R0-M1]** exclude the 1-pt spacer constant fields from map targets.

## Form 1040 (capital-gains cells ONLY) — `fill_form_1040_capgains`
- **[★ R0-C2 + R0-r2-I★1] Line is 7a (renumbered in 2025), with a new 7b checkbox pair.** Amount field `f1_70`
  ([504,90,576,102]). **Fill 7a ONLY when Schedule D is ACTIVE (there are capital disposals) AND line 16 ≥ 0**
  (gain → line 16; **active-and-netted-to-zero → "-0-"**). **[★ I★1] If Schedule D is INACTIVE** (an income-only
  / donation-only year — DA question may be YES but there are no capital disposals; `schedule_d()` forms.rs:172
  returns zero totals but the attached Schedule D `active()`-gates line 16 to BLANK), **leave 7a BLANK even when
  DA = YES** — stamping "-0-" against a blank Schedule D line 16 is an unearned zero-capital-gains claim on one
  of the two cells btctax vouches for. Reserve "-0-" for the active-netted-to-zero case only. **On a NET LOSS,
  leave 7a BLANK** + a loud notice ("line 16 is a loss — the §1211 $3,000/$1,500-MFS cap on Schedule D line 21
  is yours to complete and enter on 1040 line 7a"); SP1 scoped out line 21, so SP2 does not auto-fill an
  uncapped loss. **7b checkboxes stay UNCHECKED** (Schedule D IS attached; the child-gain box is not btctax's).
  Notice text says **"line 7a"**, never "line 7". KATs: `form_1040_line7a_gain_equals_schedule_d_line16`,
  `form_1040_line7a_loss_is_blank_with_notice`, `form_1040_7a_blank_when_schedule_d_inactive` (income-only year,
  DA=YES, 7a empty), `form_1040_7b_checkboxes_untouched`.
- **[★ R0-C4] Digital-asset question = YES only with btctax-evidenced qualifying activity.** The question is
  answered under penalty of perjury; buy-and-hold is a **No**. Yes = `c1_10[0]`, on-state `/1` ([518,497]).
  **Check YES iff** any `form_8949` row (disposal) ∨ ANY `income_recognized` entry ∨ any Gift/Donate removal
  (source the gift signal from `state.removals` — `form_8283()` emits no gift rows; r2-confirmed). **[R0-N1]**
  treat ANY income_recognized as qualifying (mining/staking/airdrop/interest/reward all = clause-(a) receipt) —
  NOT a narrow kind whitelist. Otherwise **leave BOTH boxes blank** (never fill No — btctax cannot
  know the filer's full digital-asset universe) — and **skip the whole 1040 when there is no reportable
  activity** (no 7a value either) + a note. KATs: `form_1040_da_yes_iff_reportable_activity`,
  `form_1040_present_iff_reportable_activity`, `form_1040_da_blank_when_hold_only`.
- **Partial-scope notice** enumerating EXACTLY what was filled (7a + the DA question) and that every other line
  is the filer's.

## Command
`export-irs-pdf --tax-year 2025` additionally writes (when applicable) `form_8283.pdf` (donations),
`schedule_se.pdf` (SE income ≥ $400 net-earnings floor), `form_1040_capgains.pdf` (reportable activity). Each
reuses the pseudo attestation gate + DRAFT watermark. Optional `--forms …` opt-out filter (default = all
applicable).

## Scope / SemVer / lockstep
`btctax-forms` (+3 `fill_*` fns, +**the per-form Geo oracle + verify fns [C3]**, +3 maps, +3 bundled
public-domain PDFs) + btctax-cli (`export-irs-pdf` extension; thread `w2_ss_wages` into the SE fill).
**No core tax-logic change** (reuse `form_8283()`/`compute_se_tax`/`schedule_d()`; SE line 12/13 split is a
FILL-layer choice using the already-exposed `ss`/`medicare`/`deductible_half`). MINOR. Man page + README (full
packet; 1040 partial-scope & 7a; conditional 8283/SE + $400 floor; the addl-Medicare/8959 advisory).

## Plan (TDD)
- **T0 (engine)** — the per-form geometric oracle in `btctax-forms` (`Geo` extension: column-x clusters,
  ordinal-y descent, same-y `/Btn` pair, spacer/page pins) + per-form verify fns; unit-test against the bundled
  PDFs. (This is the C3 work the spec now owns.)
- **T1 (Schedule SE)** — the 2025 SE map (spacers excluded) + `fill_schedule_se(se_result, w2_ss_wages,
  ss_wage_base, year)` (line 12 = ss+medicare; full chain; $400 skip; addl advisory) + the SE oracle + KATs.
  **NOTE: `export_irs_pdf` today computes only 8949+SchedD — T1 must ADD the SE computation** (`compute_se_tax`
  via `session.tax_profile(y)`; `w2_ss_wages` is reachable there, admin.rs:86) to the command.
- **T2 (Form 8283)** — the 2025 map (k-Digital-Assets `/11`, part renumbering, mo/yr dates) + `fill_form_8283`
  (fill/blank scope table + notice) + overflow + the 8283 oracle + KATs. **[R0-N2] verify Part IV/V field
  positions at map-extraction** — the round-1 appendix's tentative Part V donee cells actually fall in the
  Part IV appraiser block; pin each by its own `/Rect`, do not trust the appendix names.
- **T3 (Form 1040)** — the 2025 map (7a `f1_70`, DA `c1_10[0]`=/1) + `fill_form_1040_capgains` (gain-only 7a,
  loss-blank+notice, DA-iff-activity, 7b untouched, partial-scope notice) + the 1040 oracle + KATs; man page +
  README; whole-diff.

## Gotchas
- **[C1] SE line 12 = SS + Medicare only** (addl Medicare → Form 8959; advisory); line 13 = deductible_half.
- **[C2] 1040 line 7a** (not 7); fill only on gain/zero; loss ⇒ blank + notice (§1211 line-21 cap is the filer's);
  7b unchecked.
- **[C3] the geometric read-back is NEW per-form engine work** — column-x + ordinal-y + same-y-pair + spacers;
  it does NOT free-reuse; `no_unmapped_filled` does.
- **[C4] DA YES only with evidenced activity**; else blank BOTH + skip the 1040 (never fill No).
- **[I1] thread `w2_ss_wages` in**; fill the full SE chain (no dangling line). **[I2] skip SE below $400.**
- **[I3] 8283 "k Digital assets" `/11`**, part renumbering III/IV/V, mo/yr dates, 4+3 rows. **[I4] 8283 fill/blank
  scope + notice + overflow.**
- **[safety]** pseudo ⇒ attestation + DRAFT watermark on every new form; determinism (golden sha) per form; the
  bundled 8283 is revision-dated "Rev. 12-2025" (record it — SP3).
