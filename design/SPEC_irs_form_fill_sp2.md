# SPEC — official IRS PDF form-fill, sub-project 2 (Form 8283 + Schedule SE + Form 1040 cap-gains, TY2025)

**Source baseline:** `main` @ `99f26ca` (branch `feat/irs-form-fill-sp2`). **Review status: DRAFT — awaiting R0
(model: Fable — 3 new tax forms with per-form gotchas, correctness-critical).** SP2 of task #45. Builds on the
R0-GREEN, shipped SP1 engine (`btctax-forms`). SP1 = 8949 + Schedule D; SP2 = the rest of the packet; SP3 =
other years.

## Goal (SP2)
Extend `btctax export-irs-pdf --tax-year 2025` to ALSO populate the official **Form 8283** (noncash charitable —
BTC donations), **Schedule SE** (self-employment tax — mining/staking business income), and the capital-gains
lines of **Form 1040**, REUSING the SP1 engine. btctax already computes all three (`form_8283()` forms.rs:356;
`compute_se_tax`→`SeTaxResult` se.rs:25/99, written to `schedule_se.csv`; capital gain = Schedule D line 16 →
1040 line 7).

## Reuse the proven SP1 engine (no re-derivation)
`btctax-forms` already provides: the fill primitive (drop `/XFA`, set `/V`/`/AS`, NeedAppearances, strip
dates/`/ID`), the **geometric map-independent read-back** verifier (fails closed), `stamp_draft_watermark`, the
attestation gate, and the per-year TOML-map + golden-test pattern. SP2 adds THREE `fill_*(data, year)`
functions + THREE per-year maps + THREE bundled PDFs. **All three forms are XFA hybrids** (verified: f8283 2pg/
145 fields, Sch SE 2pg/32, f1040 2pg/229 — all `XFA=True`, FILLABLE) → the engine's XFA-drop applies unchanged.

## The three forms
- **Form 8283** — `fill_form_8283(form_8283_data, year)`. Section A (donations ≤ $5,000) vs Section B (>
  $5,000, requires a qualified appraisal — §170(f)(11)(C), the app already flags this). Fill the donee, date,
  description, FMV, cost/acquisition per the app's `form_8283()` rows. **Conditional:** only written when there
  ARE donations (else skipped, with a note). Map: `forms/2025/f8283.{pdf,map.toml}`.
- **Schedule SE** — `fill_schedule_se(se_result, year)` from `SeTaxResult` (se.rs). Fill the net-SE-earnings
  line (net × 0.9235), the SS portion (capped at the ss_wage_base), the Medicare portion, the SE-tax total
  (line 12), and the one-half-SE deduction (line 13). **Conditional:** only when `se_result` is `Some` (business
  SE income exists). Map: `forms/2025/schedule_se.{pdf,map.toml}`.
- **Form 1040 (cap-gains summary ONLY)** — `fill_form_1040_capgains(sched_d_totals, year)`. Fill EXACTLY:
  **(1) the digital-asset question = YES** (1040 asks "did you … sell, exchange, or otherwise dispose of a
  digital asset?" — YES for any BTC seller; the impl identifies the exact `c1_*` checkbox GEOMETRICALLY, by its
  y-band near the top), and **(2) line 7 (capital gain/loss) = Schedule D line 16**. **[★ scope]** NOTHING else
  on the 1040 (it is not a complete return) — print a LOUD notice: "Form 1040: only the digital-asset question
  and line 7 (capital gain) are filled from btctax; every other line is yours to complete." Map:
  `forms/2025/f1040.{pdf,map.toml}`.

## Command
`export-irs-pdf --tax-year 2025 --out <dir>` now ALSO writes (when applicable): `form_8283.pdf` (if donations),
`schedule_se.pdf` (if SE income), `form_1040_capgains.pdf` (always). Each reuses the pseudo attestation gate +
DRAFT watermark. An optional `--forms 8949,schedule_d,8283,se,1040` filter (default = all applicable) is a nice-
to-have; SP2 may default to all-applicable without the flag.

## KATs (per form)
- **★ geometric read-back + fault-inject** for EACH new form (swap two map fields ⇒ RED, fails closed) —
  `fill_8283/se/1040_readback_geometric`; `no_unmapped_field_filled` per form.
- **conditional presence:** `form_8283_only_when_donations`; `schedule_se_only_when_se_income`;
  `form_1040_always_present`.
- **correctness:** `form_1040_digital_asset_question_is_yes`; `form_1040_line7_equals_schedule_d_line16`;
  `schedule_se_line12_equals_compute_se_tax`; `form_8283_section_b_when_over_5000`.
- **determinism:** `each_new_form_is_byte_deterministic` (golden sha per form); `map_2025_matches_bundled_pdf_fieldset` per form.
- **safety:** the pseudo attestation + DRAFT watermark apply to every new form (`pseudo_fill_*` extended);
  `form_1040_prints_partial_scope_notice`.
- **isolation:** unchanged (btctax-forms already in TAX_CRATES; no new network dep).

## Scope / SemVer / lockstep
`btctax-forms` (+3 `fill_*` fns, +3 maps, +3 bundled public-domain PDFs) + btctax-cli `export-irs-pdf` extension.
No core change (reuse `form_8283()`/`compute_se_tax`/`schedule_d()`). MINOR. Man page + README updated (fills the
full packet; 1040 partial-scope; conditional 8283/SE). cargo-tree isolation unchanged (all pure Rust).

## Plan (TDD)
- **T1 (Form 8283)** — extract + commit the 2025 8283 map + bundled PDF; `fill_form_8283` + geometric read-back
  + Section A/B logic; wire the conditional command output; the 8283 KATs.
- **T2 (Schedule SE)** — the 2025 SE map + `fill_schedule_se` from `SeTaxResult`; the conditional output; the
  SE KATs (line 12 == compute_se_tax).
- **T3 (Form 1040 cap-gains)** — the 2025 1040 map (digital-asset checkbox found geometrically + line 7);
  `fill_form_1040_capgains`; the LOUD partial-scope notice; the 1040 KATs; man page + README; whole-diff.

## Gotchas
- **[★ 1040 partial scope]** fill ONLY the digital-asset question (YES) + line 7; LOUD notice the rest is the
  filer's — never imply a complete return.
- **[conditional]** 8283 only with donations; Schedule SE only with SE income; 1040 always.
- **[★ per-form geometric read-back]** each new form gets the same fail-closed verifier + fault-inject (a wrong
  field on ANY official form is unacceptable) — do NOT trust the maps.
- **[XFA]** all three are hybrids — the engine's `/XFA` drop applies; `output_has_no_xfa` per form.
- **[8283 §170(f)(11)] Section B + appraisal** for donations > $5,000 (the app already flags it).
- **[digital-asset question]** identify the exact `c1_*` checkbox GEOMETRICALLY (y-band near the top), not by a
  guessed name — verify the on-state renders "Yes".
- **[safety]** pseudo ⇒ attestation + DRAFT watermark on every new form; determinism (golden sha) per form.
