# R0 review — SPEC_irs_form_fill_sp1.md — round 2 (verification of the round-1 fold)

- **Artifact:** `design/SPEC_irs_form_fill_sp1.md` @ `feat/irs-form-fill-sp1` `e5984df` (main == `3117379`)
- **Reviewer:** independent architect (R0, round 2). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical feature (filling OFFICIAL IRS PDFs).
- **Charge:** confirm each round-1 fold (2C/5I/4M/3N from
  `reviews/R0-spec-irs-form-fill-sp1-round-1.md`) is captured correctly + internally consistent, spot-verified
  against the real PDFs; surface any residual contradiction or NEW gap.
- **Method:** re-ran the evidence path independently. Inspected `f8949-2025.pdf` + `schedD-2025.pdf` with
  pypdf (scratchpad `.venv`): AcroForm dict-key dump, full terminal-field tree (`/FT`/`/T`/`/Rect`/`/AP /N`
  on-states), page-text extraction, XFA-packet listing. Re-derived the grid geometry (rows, per-row fields,
  totals x-bands) and the Schedule D line→field map from the raw `/Rect`s. Confirmed the round-1 fill
  experiment artifacts (`filled_noxfa.pdf`, `render_noxfa-1.png`). Verified every repo citation against
  source (`btctax-core/src/forms.rs`, `btctax-cli/src/render.rs`, `crates/xtask/src/check_isolation.rs`).

## VERDICT: **0 Critical / 0 Important / 1 Minor / 2 Nit → R0-GREEN (cleared to implement)**

All two Critical and five Important round-1 findings are folded correctly, and every load-bearing fact is
now consistent with the actual 2025 PDFs. The two known-wrong 2025-form facts (XFA hybrids; Box C/F vs I/L)
and the cascade (11 rows, 5-field totals, 6 checkboxes, overflow, geometric read-back, Schedule D 7/15,
box-review warning) are all captured and internally consistent. I re-verified each against the PDFs — the
spec's numbers, field names, on-states, and box text are exact. The remaining items are one Minor (an
underspecified fill-vs-rename ordering in the overflow path) and two Nits (band-provenance wording; line-21
scope-out clarity) — none block implementation.

---

## Fold-by-fold verification (with re-verified evidence)

### C1 (drop `/XFA` + NeedAppearances) — **FOLDED CORRECTLY, verified.**
- Spec captures it in the *feasibility headline* (lines 13–18), Fill-mechanism step 1 ("remove `/XFA` from
  `/AcroForm`") + step 3 (`NeedAppearances true`, appearance-gen deferred), the KATs `output_has_no_xfa` +
  `filled_value_persists_after_xfa_drop` (line 78), and the one-time **manual Acrobat** open ("documented,
  not CI", line 18) — matching round-1 C1's four sub-items.
- **PDF re-check:** `f8949-2025.pdf` `/AcroForm` keys = `['/DA','/DR','/Fields','/SigFlags','/XFA']` — `/XFA`
  present, `/NeedAppearances` absent; XFA packets `preamble, config, template, datasets, sourceSet,
  connectionSet, localeSet, xmpmeta, form, postamble` (a full hybrid). The classic layer is **complete**:
  **202 terminal fields** (190 `/Tx` + 12 `/Btn`); every checkbox carries `/AP /N` on-states.
- **Approach coherence — verified end-to-end on the artifact:** `filled_noxfa.pdf` has AcroForm keys
  `['/DA','/DR','/Fields','/NeedAppearances','/SigFlags']` — **no `/XFA`**, `NeedAppearances = true`, and the
  Box-I checkbox persisted (`c1_1[5]`: `/V = /6`, `/AS = /6`). `render_noxfa-1.png` renders the values
  ("0.5 BTC", "12,345.00") and the **(I) box checked** correctly in poppler. The drop-XFA + NeedAppearances
  + `/V`//`/AS` path produces exactly what the two KATs assert. Coherent.

### C2 (Box I / Box L for TY2025 digital assets, mapped in the forms crate) — **FOLDED CORRECTLY, verified.**
- Spec (lines 20–27) uses Box I (ST) / Box L (LT), cites on-states `/6` of `c1_1[5]` / `c2_1[5]`, states the
  core `Form8949Box::{C,F}` (`forms.rs:114-116`) is the OLD taxonomy and is **not reused directly** (the
  forms crate maps to the year's box via the per-year map, no core change), and that Schedule D line 3/10
  accept "Box C or Box I" / "Box F or Box L". KATs `ty2025_bitcoin_uses_box_I_and_L` +
  `schedule_d_line3_10_accept_I_L` (line 79).
- **PDF re-check — box text:** Part I has **six** boxes, ordered **A, B, C, G, H, I**. Box **C** =
  *"Short-term transactions, **other than digital asset transactions**, not reported to you on Form 1099-B or
  Form 1099-DA"* — checking it for BTC is factually false, as the spec says. Box **I** = *"Short-term digital
  asset transactions not reported to you on Form 1099-DA or Form 1099-B"* — the correct conservative default.
- **On-state citations — confirmed geometrically (not just plausible):** the six Part-I checkboxes sort
  top→bottom `c1_1[0]`(y530,`/1`) … `c1_1[5]`(y470,`/6`); against the visual A→I ordering this pins
  `c1_1[5]` = **Box I**, on-state `/6`. Part II mirrors it: `c2_1[5]` = **Box L**, `/6`. The render shows
  `c1_1[5]=/6` displaying as **(I) checked** — independent confirmation.
- **Schedule D survives — re-extracted:** line 3 text = "with **Box C or Box I** checked", line 10 =
  "with **Box F or Box L** checked". Line-number reuse (3/10) holds under the re-lettering.
- **Repo citation exact:** `forms.rs:115-116` are the `Term::ShortTerm ⇒ Form8949Box::C` /
  `Term::LongTerm ⇒ Form8949Box::F` arms (match at 114); the "not reported on a 1099-B" rationale is the
  pre-2025 default. Keeping the enum + mapping in the forms crate honors "no core change".

### I1 (grid facts: 11 rows, 5-field totals, per-year DATA) — **FOLDED CORRECTLY, verified.**
- Spec (lines 48–54): 8 sequential text fields/row = cols a–h; **11 rows/part/page (not 14)**; totals row =
  **5 fields `f1_91..f1_95`** (d,e,g,h + a spacer); 6 box checkboxes/part; **rows-per-page is per-year map
  DATA**; map keys fully-qualified/bracketed; Schedule D uses **unpadded** `f1_3`.
- **PDF re-check:** `Table_Line1_Part1` (Page1) = **Row1..Row11** exactly; Row1 = `f1_03[0]..f1_10[0]`
  (8 fields). Totals row (not under any Row node) = **`f1_91..f1_95`** with x-bands
  d `273.6–337.6` / e `338.4–402.4` / f `403.2–446.4` (greyed on the form = the spacer) / g `446.4–510.4` /
  h `511.2–576.0`. So the four filled totals are d/e/g/h with the greyed (f) cell `f1_93` left blank — exactly
  the spec's "d,e,g,h + a spacer". KATs `eleven_rows_per_page`, `rows_per_page_is_map_data`,
  `map_2025_matches_bundled_pdf_fieldset` cover it.

### I2 (overflow: rename-per-copy + per-copy totals) — **FOLDED CORRECTLY, consistent.**
- Spec (lines 62–67): >11 rows/part ⇒ `⌈rows/11⌉` page copies; deep-copy each page AND **rename its fields
  per copy** (ISO-32000 same-name single-`/V` trap explicitly named); **per-copy totals** (the form's line-2
  says "Enter each total here", not last-page-only); Schedule D aggregates the grand totals. Demo 26 ST +
  28 LT ⇒ 3 + 3 pages (⌈26/11⌉,⌈28/11⌉) — matches round-1 N2. KATs
  `overflow_renames_fields_per_copy_no_shared_value` + `each_copy_has_its_own_totals`.
- **Confirmed** on the render: line 2 reads "Add the amounts in columns (d), (e), (g), and (h) … **Enter each
  total here** … line 3 (if Box C or Box I above is checked)" — the per-copy-totals instruction the fold
  relies on. Coherent. (One NEW ordering ambiguity noted under Minor M-new-1 below — non-blocking.)

### I3 (geometric, map-independent read-back) — **FOLDED CORRECTLY, consistent.**
- Spec (lines 43–46 + KAT line 77): naive re-read-via-same-map is circular; verify GEOMETRICALLY (each
  written value sits in a widget whose `/Rect` is in the expected column-x / row-y band) AND assert **no
  unmapped field was filled**; fault-inject: swap two map columns ⇒ RED. `/AS`-checked read-back retained.
  This is the correct structural safety net and matches round-1 I3's fix precisely. The geometry it needs is
  present in the widgets' `/Rect`s (I re-extracted the full 8-column × 11-row band set trivially — see Nit
  N-2 on where the spec says those bands come from).

### I4 (Schedule D fills 3/7/10/15/16 + QOF + scope-out 17–22) — **FOLDED CORRECTLY, verified.**
- Spec (lines 55–58): fill lines 3 AND 7 (ST), 10 AND 15 (LT) — 7/15 pure arithmetic (else 16 = 7+15 with
  7/15 blank is self-inconsistent); line 16 net; answer QOF (No for SP1); **scope OUT 17–22** with a printed
  notice. KATs `schedule_d_fills_3_7_10_15_16_and_qof`, `schedule_d_totals_match_form8949_and_csv`.
- **PDF re-check (line→field, re-derived from `/Rect`-y):** line 3 = `f1_15..f1_18` (d/e/**g**/h),
  line 7 = `f1_22` (net ST, single h), line 10 = `f1_35..f1_38`, line 15 = `f1_43` (net LT), line 16 = `f2_1`
  (page 2). The **QOF Yes/No** exists at page-1 top (`c1_1[0]`=`/1`Yes / `c1_1[1]`=`/2`No) — distinct from
  lines 17–22, so filling QOF while scoping out 17–22 is not a contradiction. Scoped-out page-2 lines are
  `f2_2`(18)/`f2_3`(19)/`f2_4`(21) + yes/nos `c2_1`(17)/`c2_2`(20)/`c2_3`(22). The arithmetic identities
  (7 = 3h, 15 = 10h when 1a–6/8a–14 blank; 16 = 7+15) hold on the reuse-path data. Round-1's "line 3 is
  d/e/g/h (four fields), not d/e/h" correction is reflected (line 55 lists the whole totals shape).

### I5 (box_needs_review → loud Box G/H/J/K warning) — **FOLDED CORRECTLY, consistent.**
- Spec (lines 26–27, gotcha line 107, KAT line 83 via safety set): the CSV `box_needs_review` maps to a loud
  warning — if any row could belong on a separate 1099-DA-reported 8949 (Box G/H/J/K), warn; SP1 files all
  BTC under I/L and says so. Matches round-1 I5's "minimum" (loud warning + count). `box_needs_review` is
  real (`forms.rs:50-54`, surfaced at `render.rs:972`).

### Minors folded
- **M1 (isolation gate):** spec line 30–31 adds `btctax-forms` to `check_isolation.rs` TAX_CRATES and keeps
  it an xtask/CI step (not a `#[test]`); KAT line 84. **Confirmed the file + list exist:**
  `crates/xtask/src/check_isolation.rs:11-17` `TAX_CRATES = [btctax-cli, -tui, -tui-edit, -core, -adapters]`
  (btctax-forms correctly *to be added* by the implementation — the spec, not the code, is the artifact under
  review). lopdf is pure Rust so the gate will pass.
- **M2 (field-name resolution):** spec lines 53–54 + 59 state map keys are fully-qualified + bracketed
  (`topmostSubform[0].Page1[0].f1_03[0]`) with checkbox on-states, and flag **Schedule D unpadded `f1_3`**.
  **Verified:** Schedule D leaves are `f1_1`,`f1_2`,`f1_3`,…,`f1_43` (unpadded), vs 8949's zero-padded
  `f1_03`. Exact.
- **M3 (embedding):** spec line 32 "Bundled PDFs via `include_bytes!`" + scope line 87 — captured (a
  cargo-installed binary has no data dir; sizes 129 KB + 98 KB fine).
- **M4 (watermark resources):** spec line 72 "an lopdf content-stream overlay with its OWN embedded
  font/vector resources", on every page including I2's copies; KATs `pseudo_fill_is_watermarked` +
  `real_fill_is_clean`. Attest+watermark (not refuse) retained as the correct policy-consistent bar.
- **Projection reuse point — confirmed real:** `write_form8949_csv` (`render.rs:947`) calls
  `form_8949(state, year)` (`render.rs:968` → `forms.rs:99`) and `schedule_d(state, year)` (`render.rs:998` →
  `forms.rs:172`). The spec's "reuse `form_8949()`/`schedule_d()` behind `write_form8949_csv`, do NOT
  recompute" (lines 9–11) is accurate.

---

## New gap introduced by the rewrite (Minor)

### M-new-1 (Minor) — Overflow: the fill-vs-rename ORDER against the map's fully-qualified keys is unstated.
The map keys are now **fully-qualified** (`topmostSubform[0].Page1[0]...f1_03[0]`, M2/I1). The overflow path
(I2) "DEEP-COPIES each page AND RENAMES its fields per copy … filling each with its 11-row slice" — read in
that order, a rename that rewrites the `topmostSubform[0]` prefix (the spec's own suggested scheme) would make
the map's fully-qualified keys **no longer resolve** on a renamed copy. There is an obvious correct
resolution — apply the per-year map to each fresh clone on its ORIGINAL names, THEN rename the subtree for
`/V`-independence — but the spec doesn't pin it. The **geometric read-back (I3) is map-independent, so it
catches any resulting mis-fill regardless**, which is why this is Minor, not Important. **Suggested one-liner
for T2:** "fill each clone via the map on its original field names, then rename its field subtree; read-back
verifies each renamed copy geometrically." (Not a blocker; T2 detail.)

**Non-gap I explicitly checked and cleared:** the watermark's own resources vs `NeedAppearances`. These are
orthogonal layers — `NeedAppearances = true` instructs the viewer to regenerate **widget-annotation**
appearance streams, while the DRAFT watermark is a **page content-stream overlay** carrying its own entries
in the page `/Resources`. They don't share a resource namespace or a regeneration path, so there is no
conflict (the round-1 experiment already rendered a NeedAppearances form correctly). No finding.

---

## Nits (non-blocking)

- **N-1 — line 21 inside the 17–22 scope-out.** Line 57 says "Line 21 (loss limit) applies on a net loss"
  immediately before "Scope OUT 17-22". Line 21 ∈ 17–22, so it IS scoped out (covered by the printed
  notice); the sentence is rationale, not a fill instruction, but a reader could momentarily read it as a
  contradiction. One clarifying clause ("…relevant on a net loss but also deferred to the filer via the
  17–22 notice, since §1211 netting lives in engine B") would remove all doubt. Tax-safe either way — the
  scope-out RANGE is explicit.
- **N-2 — geometric-band provenance.** Line 46 says the read-back bands are "dumped in the R0 review", but
  round-1 dumped only a couple of sample bands, not the full 8-col × 11-row table. The authoritative source
  is the bundled PDF's `/Rect`s (I re-extracted the complete set in one pass), and round-1 I3 says the bands
  "live beside the map". Reword T1 to "bands derived from the bundled PDF at map-build time" so nobody hunts
  the review for a table that isn't fully there. Trivial; the data is present and cheap to extract.

---

## Self-consistency sweep

- **Baseline** "main @ `3117379`" is correct (`git rev-parse main` == `3117379`).
- **No residual wrong-form facts.** Every "14 rows / Box C-F / 8-field totals / 3 checkboxes" number from the
  pre-fold spec is gone; the surviving mentions of C/F (line 24) and "14→11" (line 62 heading) are explicit
  *corrections*, not live claims.
- **Cascade is internally consistent:** ⌈n/11⌉ pagination, 5-field totals, 6 checkboxes with `/1../6`,
  Schedule D 3/7/10/15/16 arithmetic, and the demo 3+3 all line up with the re-verified geometry.
- **Plan implementable with 0 open blocking questions.** T1 (engine: drop-XFA/`/V`//`/AS`/NeedAppearances/
  strip-dates primitive + geometric read-back + TOML map loader + extracted TY2025 maps + KATs) and T2
  (command + overflow + Schedule D + attest/watermark + box warning + docs) are both buildable from the spec.
  The only underspecified item is M-new-1's fill-vs-rename order, a T2 implementation detail with an obvious
  correct answer and a map-independent safety net — it does not gate the plan.

**R0-GREEN — cleared to implement.** 0 Critical / 0 Important. Recommend T2 pick up M-new-1's one-line
ordering note and the two Nits opportunistically; none block the start of implementation.
