# R0 review — SPEC_irs_form_fill_sp1.md — round 1

- **Artifact:** `design/SPEC_irs_form_fill_sp1.md` (DRAFT) @ `feat/irs-form-fill-sp1` `9ee204e` (main == `3117379`)
- **Reviewer:** independent architect (R0, round 1). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical feature (filling OFFICIAL IRS PDFs).
- **Method:** evidence-driven. Inspected the actual downloaded official PDFs (`f8949-2025.pdf`,
  `schedD-2025.pdf`, `f8949-2017.pdf`, `schedD-2017.pdf`) with pypdf 6.14.2 (scratchpad venv): AcroForm
  dictionary dumps, full field-tree dumps with `/FT`/`/T`/`/Rect`/on-states, page-text extraction, XFA packet
  extraction, a live fill experiment (set `/V` + checkbox, save with and without `/XFA`, re-read, render with
  poppler). Verified repo claims against source (`btctax-core/src/forms.rs`, `btctax-cli/src/render.rs`,
  `btctax-cli/src/cmd/admin.rs`, `btctax-cli/src/lib.rs`, `crates/xtask/src/check_isolation.rs`). Verified
  lopdf's API surface against current docs (context7 `/j-f-liu/lopdf`).

## VERDICT: **BLOCKED — 2 Critical / 5 Important / 4 Minor / 3 Nit**

The overall shape (new pure-Rust `btctax-forms` crate, lopdf, per-year TOML maps, projection reuse,
read-back + determinism + golden KATs, attest + watermark safety) is sound and worth keeping. But the spec
was written against the **wrong facts about the 2025 forms** in two load-bearing places: (C1) all four
official PDFs are **XFA hybrids** and the spec's fill mechanism has no XFA handling — the filled forms would
open **blank in Adobe Acrobat/Reader**; (C2) the TY2025 Form 8949 was revised for **1099-DA** and re-lettered
its boxes — for digital assets the spec's Box C/F is now the **factually wrong box** on the official form.
Both have cheap, verified fixes. Several structural facts (rows/part, totals-row fields, checkbox count) are
also wrong for 2025 and cascade into the overflow design.

---

## Critical

### C1 [★ the feasibility question] — All four official PDFs are XFA hybrids; the spec's `lopdf` `/V`-only fill has no XFA handling and would display BLANK in Adobe Acrobat/Reader. **Fix: strip `/XFA` on fill (verified to work) + KAT + one-time Acrobat check.**

**Spec claim (lines 22–31, "Fill mechanism"):** load → set `/V`/`/AS` → `NeedAppearances` → strip dates →
read-back. The word "XFA" appears **nowhere** in the spec.

**Evidence (inspected, all four PDFs):** the `/AcroForm` dictionary of every downloaded form is
`['/DA', '/DR', '/Fields', '/XFA', '/SigFlags']` — a **full XFA hybrid** with packets
`preamble, config, template, datasets, …, form, postamble` (f8949-2025, schedD-2025, f8949-2017,
schedD-2017 all identical in kind). The `datasets` packet is a live XML data layer containing an **empty
node per field** (`<Row1><f1_03/><f1_04/>…`, extracted and read — 2,698 bytes in f8949-2025). `NeedAppearances`
is absent in the shipped files.

**Why it's Critical:** in a hybrid, Adobe Acrobat/Reader's XFA processor treats the XFA layer (template +
datasets) as authoritative. A fill that writes only the classic AcroForm `/V` (which is ALL lopdf touches,
and all the spec specifies) leaves the datasets packet empty — **Acrobat renders the fields empty**, and an
Acrobat re-save can discard the `/V` values. Acrobat/Reader is the viewer the IRS itself points filers at,
so "opens blank in Acrobat" on a tax deliverable is a shipping-blocker, and the spec's own read-back KAT
would stay GREEN through it (the `/V` data is present — see also I3). This is the well-documented reason
pdftk grew `drop_xfa` and iText recommends XFA removal before AcroForm filling.

**Verified fix (tested in this review):** after filling, **delete the `/XFA` key from the `/AcroForm`
dictionary**, converting the hybrid into a plain AcroForm PDF. These IRS forms are *static* XFA hybrids —
the page content streams and merged widget annotations are fully self-sufficient without the XFA layer.
Experiment run: filled `f1_03[0]` ("0.5 BTC"), `f1_06[0]` ("12,345.00") and checkbox `c1_1[5]` (`/6` = Box I)
via a raw `/V`/`/AS` write; saved once with `/XFA` kept and once dropped; **re-read: `/V` persisted in the
classic fields in both**; rendered the XFA-stripped file with poppler — values and the Box (I) check render
correctly (`render_noxfa-1.png` in the scratchpad). Fold into the spec:

1. Fill-mechanism step: "remove `/XFA` from `/AcroForm`" (one dictionary-key delete in lopdf — trivial).
2. New KAT: `output_has_no_xfa` (the written PDF's AcroForm has no `/XFA` key).
3. A one-time **manual Adobe Acrobat/Reader verification** of a filled sample, recorded in the review
   artifacts (Acrobat behavior cannot be CI'd; poppler/pdfium ignore XFA so they can't stand in for it).
4. (Documented alternative, NOT recommended for SP1: sync values into the XFA `datasets` XML packet —
   strictly more code, XML-in-PDF string surgery, and it conflicts with the overflow rename in I2.)

Note the synergy with I2: after per-copy field renaming for overflow, the XFA layer is stale anyway —
dropping it is required twice over.

### C2 [tax-critical] — TY2025 Form 8949 was revised for 1099-DA: the boxes are re-lettered and **Box C/F now explicitly EXCLUDES digital assets**. The spec (and the reused `Form8949Box::{C,F}` projection) would check a factually false box; the correct conservative default for BTC is **Box I (ST) / Box L (LT)**.

**Spec claims:** line 37 "Box A/B/C checkbox = `c1_*`, D/E/F = `c2_*`"; line 40 "line 3 (short-term Box-C
totals … line 10 (long-term Box-F totals)"; line 45 heading "Box C/F must be itemized"; lines 19–20 + 71
"reuses the form data … No core change".

**Evidence (extracted from f8949-2025.pdf page text):** Part I has **six** checkboxes —
(A), (B), **(C) "Short-term transactions, other than digital asset transactions, not reported to you on Form
1099-B or Form 1099-DA"**, (G) 1099-DA basis reported, (H) 1099-DA basis not reported,
**(I) "Short-term digital asset transactions not reported to you on Form 1099-DA or Form 1099-B"**.
Part II likewise: (D), (E), (F) "other than digital asset transactions…", (J), (K), (L) long-term digital
asset. The widgets are `c1_1[0]..c1_1[5]` / `c2_1[0]..c2_1[5]` with on-states `/1../6` (dumped). The
repo's projection hardcodes C/F: `crates/btctax-core/src/forms.rs:114-116`
(`Term::ShortTerm => (…, Form8949Box::C)`, `Term::LongTerm => (…, Form8949Box::F)`), enum + rationale at
`forms.rs:29-40` ("not reported on a 1099-B" conservative default — written against the pre-2025 form).

**Why it's Critical:** on the 2025 revision, checking Box C/F for bitcoin disposals asserts "other than
digital asset transactions" — a false statement on an official return, in the very first 1099-DA year. The
semantically identical conservative default is now **Box I / Box L**.

**What survives (verified):** Schedule D 2025 line 3 reads "…with **Box C or Box I** checked" and line 10
"…with **Box F or Box L** checked" (extracted) — so the spec's Schedule D **line numbers** 3/10/16 stay
correct with I/L. The overflow requirement also survives re-lettered: the form's aggregation shortcut
(direct-to-Schedule-D lines 1a/8a) applies only to basis-reported 1099-B/1099-DA transactions (Boxes A/G,
D/J) — Box I/L must itemize, same as C/F did.

**Concrete fix:** keep the core enum untouched (its *semantic* is "not broker-reported"); add a per-year
**box-letter mapping in `btctax-forms`** (map data, alongside the field map): TY2025 `C→I` (`c1_1[5]`,
on-state `/6`), `F→L` (`c2_1[5]`, `/6`); TY2017 (SP3) keeps C/F. This preserves "No core change" honestly.
Update spec lines 37/40/45 accordingly. Add a FOLLOWUPS item: the CSV `box` column (`render.rs:947`,
`form8949_box_tag` C/F) is now misleading for TY2025 output — out of SP1 scope but must not be forgotten.

---

## Important

### I1 — The spec's 2025 grid facts are wrong: **11 rows/part (not 14)**, totals row = **5 fields (not 8)**, **6 checkboxes (not 3)**; rows-per-page must be per-year map DATA.

**Spec claims:** lines 34–37 ("14 rows/part/page", "the line-2 totals row = its own 8 fields"); line 46
("Form 8949 holds 14 rows per part per page"); line 47 ("⌈rows/14⌉").

**Evidence (field dumps):** f8949-2025 `Table_Line1_Part1` has **Row1..Row11** (both pages) — 11 rows × 8
fields = `f1_03..f1_90`; the line-2 totals row is **`f1_91..f1_95`** = 5 fields whose `/Rect` x-bands match
columns (d) 274–338, (e) 338–402, (f) 403–446 (greyed on the form), (g) 446–510, (h) 511–576 — the totals
carry d/e/g/h (+ an unused (f) cell), never a/b/c. Checkboxes: 6 per part (see C2). The **2017** form is
where "14 rows" comes from: f8949-2017 has Row1..Row14, totals `f1_115..f1_118` (4 fields), 3 checkboxes —
the spec's numbers describe the OLD form.

**Why Important:** every derived number shifts — pagination is ⌈n/11⌉ (the ReadOnly demo's 26 ST + 28 LT =
3+3 form copies, not 2+2), the totals-row map has 5 entries, and since 2017 (SP3) really is 14 rows,
**rows-per-page/totals-shape/checkbox-states must live in the per-year TOML map**, not in code — otherwise
the spec's own "adding a year = data-only, NO code change" (lines 42–43) is broken on day one.

**Fix:** correct lines 34–37/45–49; declare rows-per-page, totals field list, and (box-letter → field +
on-state) as map data; make `field_map_2025_covers_all_cells` assert the map's declared shape against the
bundled PDF's actual field set.

### I2 — Overflow by page-cloning hits the same-name-field trap (duplicated fields share ONE value); and totals-on-last-page-only contradicts the form's own line-2 instruction. Rework the overflow model.

**Spec claims:** lines 45–49 ("cloning the 8949 page … ⌈rows/14⌉ times … the per-part line-2 TOTALS go on
the LAST page of that part (the intermediate pages leave totals blank)"), plan T2 line 81, KAT line 61–62
(`overflow_paginates_and_totals_on_last_page`).

**Evidence:** every data field is a merged widget+field whose `/T` is the leaf (e.g. `f1_03[0]`) under the
`topmostSubform[0].Page1[0].Table_Line1_Part1[0].Row1[0]` hierarchy (dumped). Duplicating a page's widgets
into the same document yields multiple field objects with the SAME fully-qualified name — per ISO 32000-1
§12.7.3.2 same-named fields ARE one field and share one `/V`: the second copy's fill overwrites the first,
and with `NeedAppearances` every copy renders the last-written values. The spec never mentions renaming.
lopdf has no page-clone primitive either — cloning is a manual deep copy of the page dict + `/Annots` +
field-tree registration (feasible, but it must be specified).

Additionally the form's own totals row reads (extracted): "**Totals.** Add the amounts in columns (d), (e),
(g), and (h) … **Enter each total here** and include on your Schedule D, line 1b …, line 2 …, or line 3 …",
and the header instructs "complete **as many forms** with the same box checked as you need" — i.e. each
generated form copy is a complete Form 8949 carrying **its own page totals**, and Schedule D aggregates
across copies ("include on"). Grand-totals-on-the-last-page-only, intermediates blank, is not what the form
says to do.

**Fix (pick one, spec it explicitly):**
- **(recommended)** Fill ⌈n/11⌉ **independent in-memory copies of the bundled form** and concatenate into
  one output PDF with a **rename pass** (e.g. prefix `p<k>.` on each copy's root field, or rename
  `topmostSubform[0]` → `topmostSubform_p<k>[0]`); dropping `/XFA` (C1) is mandatory here anyway since the
  XFA template still describes the un-renamed tree. Or emit one file per copy (`f8949-1.pdf`, …) and skip
  renaming entirely — procedurally fine ("as many forms as you need") but less convenient.
- Each copy gets name/SSN + the box re-checked + **its own line-2 totals over its 11 rows** (spec is silent
  on re-checking the box / name per copy — say it). Schedule D lines 3/10 carry the grand totals from the
  `schedule_d()` projection, unchanged.
- Rename the KAT to match (`overflow_paginates_with_per_page_totals`), and have read-back verify **every
  copy's** fields under their renamed paths (this is what catches the shared-value trap mechanically).

### I3 — Read-back is map-circular: it verifies through the SAME map it must distrust, so a swapped/mis-assigned map passes GREEN. Add a map-independent structural check.

**Spec claims:** lines 30–31 + 59–60 ("re-open the written PDF, read every mapped field's `/V`, assert it
equals the intended value … The fault-inject target: corrupt one map entry ⇒ RED").

**Why Important:** the read-back looks up the field **by the map entry's name** and compares to the intended
value — it catches a name that doesn't exist and post-write corruption, and the fault-injection only proves
that case. It **cannot** catch the headline risk it's advertised for: a map whose `row1.proceeds` points at
the cost-basis field and vice-versa reads back self-consistently. (It would also have stayed GREEN through
C1.) The "golden map" (line 65) only freezes the map — it doesn't ground it in the form's geometry.

**Fix:** add a map-independent structural KAT: for every map entry, assert the named widget's **page** and
`/Rect` fall inside the expected (column x-band × row y-band) — the bands are verifiable constants from the
official PDF (e.g. 2025 col (d) = x 274–338; row 1 = y 372–396, dumped above) and live beside the map;
checkbox entries assert the on-state exists in the widget's `/AP /N`. Plus: read-back additionally asserts
**no unmapped terminal field carries a value**, and checkbox read-back checks `/V` AND `/AS` (line 60 says
`/AS` — keep it). Plus a one-time human-eyeballed rendered sample (fill every cell with its own logical-cell
name, render, review) committed as a review artifact. With those, the mis-mapping risk is genuinely covered.

### I4 — Schedule D output as specced is internally inconsistent: line 16 filled while lines 7/15 (and 21 for losses, plus the mandatory yes/no boxes) are blank. Fill 7/15; explicitly decide 17–22 + the QOF question.

**Spec claim:** line 40–41 ("line 3 …, line 10 …, line 16 (net). Committed `schedule_d.map.toml`.") — nothing
else on Schedule D.

**Evidence:** schedD-2025 (68 fields, confirmed) has line 4–7 = `f1_19..f1_22`, 11–15 = `f1_39..f1_43`,
page 2: line 16 = `f2_1`, 17 = `c2_1` (Yes/No), 18 = `f2_2`, 19 = `f2_3`, 20 = `c2_2`, 21 = `f2_4`,
22 = `c2_3`; page 1 top also has the **qualified-opportunity-fund Yes/No** (`c1_1[0..1]`) which the form
requires answered. `schedule_d()` (`forms.rs:173-190`) supplies raw ST/LT sums — line 7 = line 3(h) and
line 15 = line 10(h) when lines 1a–6/8a–14 are blank, so filling 7/15 is pure arithmetic on data the spec
already reuses; §1211 netting for line 21 lives in engine B (`compute_tax_year`), out of the reuse path.

**Why Important:** an official Schedule D with 16 = 7 + 15 populated but 7/15 blank is visibly wrong to any
reviewer/preparer; line 21 is mandatory whenever line 16 is a loss; 17/20/22 are mandatory yes/nos. "Ready-
to-review" (line 11) doesn't excuse arithmetic inconsistency on the lines it DOES fill.

**Fix:** fill lines 7 and 15 (same totals as 3(h)/10(h)); for 17–22 + the QOF box, either fill (17 is
derivable; 18/19 are plausibly 0/blank for BTC-only; 21 needs engine-B netting) or — acceptable for SP1 —
**explicitly scope them out in the spec** with a printed CLI notice ("lines 17–22 and the QOF question left
for the filer") + FOLLOWUPS entry. Silent omission is the only wrong option. Also fix line 40's "d/e/h":
line 3 is four fields `f1_15..f1_18` = d/e/**g**/h (g exists even though btctax's adjustment is always 0).

### I5 — The official PDF silently drops the `box_needs_review` signal, in the first 1099-DA year. Specify the behavior when needs-review rows exist.

**Evidence:** `Form8949Row.box_needs_review` (`forms.rs:50-54`, set for Exchange-wallet disposals because
"such a disposition MAY have been reported on a 1099-B/1099-DA (2025+ broker reporting)") is a per-row CSV
column (`render.rs:956`). The official form has no such column, and for TY2025 the concern is live: exchange
disposals will commonly have a 1099-DA → Box G/H (ST) / J/K (LT), and the form mandates "Check only one
box… complete a separate Form 8949 … for each applicable box" — a single-box fill that includes 1099-DA-
reported rows under Box I/L is the exact mis-assertion `box_needs_review` exists to flag. The spec (lines
19–20) reuses the projection but says nothing about this flag.

**Fix (minimum):** when any row in the fill has `box_needs_review == true`, print a loud CLI warning with
the count ("N rows disposed from exchange wallets MAY appear on a Form 1099-DA; if so they belong under
Box G/H/J/K on a separate 8949 — review before filing") and record the count in the command's report;
consider requiring an explicit `--ack-box-review` style flag. State the decision in the spec either way, and
note per-box multi-form output as an SP2+ follow-up.

---

## Minor

### M1 — Extend the xtask isolation gate rather than (only) an in-crate cargo-tree KAT.
`crates/xtask/src/check_isolation.rs:11-17` `TAX_CRATES` must gain `btctax-forms` (transitive coverage via
btctax-cli exists but the gate's contract is per-crate and explicit). Note the file's own doc
(`check_isolation.rs:6-7`): cargo-tree checks live in xtask/CI, **not** a non-hermetic `#[test]` — the
spec's KAT `btctax_forms_has_no_network_dep` (line 68) should be re-homed/re-worded to match. lopdf itself
is pure Rust (flate2/nom/etc.; no ureq/rustls), so the gate passes.

### M2 — Field-name resolution is underspecified: names are hierarchical, bracketed, and inconsistently padded across forms; checkbox map entries need on-states.
The map examples ("`f1_03..f1_10`", line 35–36) are leaf names; the real terminal `/T` literally includes
the index (`f1_03[0]`), the fully-qualified name is e.g.
`topmostSubform[0].Page1[0].Table_Line1_Part1[0].Row1[0].f1_03[0]`, and **Schedule D uses UNPADDED leaves**
(`f1_3[0]`, dumped) vs 8949's `f1_03[0]`. lopdf has no forms API — the engine hand-walks
`/AcroForm → /Fields → /Kids` composing T-paths (straightforward; the widgets are merged so `/V`/`/AS` set
on the found dict is sufficient). Spec: state that map keys are fully-qualified names (or unique literal
leaf `/T`s), and that checkbox entries carry `(field, on-state)` pairs (Box I = `/6`, not `/Yes`).

### M3 — State the bundling mechanism: `include_bytes!` (embedded), so `cargo install btctax-cli` works.
"Bundled … `load_mem`" (lines 17, 23) implies embedding; make it explicit — a cargo-installed binary has no
data directory. Precedent: `btctax-adapters/src/price.rs:10` (`include_str!` dataset). Sizes are trivially
fine for crates.io (f8949 129 KB + schedD 98 KB). Also note: the published-crates set (all 7 on crates.io)
grows to 8 — the publish/release procedure needs the new crate added.

### M4 — Watermark mechanics: specify the overlay's own resources and that it covers cloned/instantiated pages; the watermark+attestation bar itself is RIGHT — keep it (don't switch to refuse-outright).
Feasibility confirmed: lopdf `add_to_page_content` appends operations (drawn above the page art; widget
values render above it — fine for a background diagonal). Specify: the overlay must register its own font in
the page's `/Resources` (or draw vector strokes to avoid font dependence entirely), diagonal via `cm`, on
EVERY page of every generated copy (I2's copies included) — and `pseudo_active_fill_is_watermarked` should
assert per-page. On the refuse-vs-watermark question: watermark+attest is the correct bar — it matches the
shipped, user-mandated pseudo-export policy (`cmd/admin.rs:71-78` gates but does not refuse; `lib.rs:90`
reuse is exactly right) and makes the PDF *strictly safer* than the already-allowed pseudo CSV (which has no
visual guard at all). Refusing would break policy consistency and kill the legitimate planning-draft use.
Cheap extra: suffix pseudo outputs' filenames (e.g. `f8949.DRAFT.pdf`).

---

## Nit

### N1 — "2pg/229 fields" counts container nodes.
229 total names = **202 terminal** (95 `f1_*` + 95 `f2_*` + 6 + 6 checkboxes) + 27 containers. Say
"202 terminal fields" to keep the golden-map coverage number honest.

### N2 — Demo pagination arithmetic.
Line 46's "26 ST + 28 LT" under the real 11-row grid = 3 + 3 form copies (⌈26/11⌉, ⌈28/11⌉), not 2 + 2.

### N3 — Determinism wording.
Stripping the SOURCE PDF's `/CreationDate`/`/ModDate`/`/ID` (line 28) is hygiene, not what makes output
deterministic — those bytes are already constant in the bundled file. The real requirement is "the writer
injects no time/randomness"; lopdf serializes document state as-is (no observed save-time stamping), and the
byte-golden KAT (line 64) is the actual enforcement. Reword so T1 verifies the right thing.

---

## Charter answers (1–7, condensed)

1. **AcroForm vs XFA:** **XFA hybrids, all four files** — the make-or-break risk is REAL but cheaply fixed:
   the classic AcroForm layer is complete; drop `/XFA` at fill time (tested: values persist, poppler renders
   correctly, checkbox included). Without the fix, filled forms open blank in Acrobat. → **C1**.
2. **lopdf capability:** right crate — dictionary-level `/V`/`/AS`/`NeedAppearances`/key-deletion/content
   overlay all confirmed in its API; no forms/page-clone convenience (hand-rolled walk + copy, feasible);
   no observed save-time timestamp/ID injection, byte-golden KAT enforces it. Higher-level crates don't fit
   (`printpdf` = generation-only, `pdf` = read-focused). → M2/N3.
3. **Field map:** the 8-fields-per-row grid and `f1_03` start are right; **rows (11 vs 14), totals row
   (5 fields), checkboxes (6, on-states /1../6), Schedule D unpadded names** are wrong/missing; col (f) is a
   plain `/Tx` text field (no dropdown); Schedule D lines 3/10/16 fields confirmed to exist
   (`f1_15..18`, `f1_35..38`, `f2_1`). TOML-map + golden model is sound once shape data moves into the map.
   → C2/I1/M2.
4. **Overflow:** itemization confirmed required for the non-broker-reported boxes (now I/L); mechanically the
   clone-in-place design hits the shared-field-name trap and the per-page-totals instruction — rework to
   independent renamed form copies with per-copy totals. → I2 (+I1 for ⌈n/11⌉).
5. **Estimate safety:** attestation reuse is exactly right (`require_attestation`, pure compare); watermark
   feasible in lopdf; watermark+attest (not refuse) is the correct, policy-consistent bar. → M4.
6. **Data source / architecture:** reuse is correct and verified — `form_8949()` / `schedule_d()`
   (`crates/btctax-core/src/forms.rs:99,173`) are precisely what `write_form8949_csv`
   (`btctax-cli/src/render.rs:947`) renders; new crate + embedded public-domain PDFs sound; isolation gate
   passes with M1's one-line extension.
7. **Verification sufficiency:** read-back+golden+determinism catch value/name-existence errors but NOT a
   self-consistently wrong map (circularity) — add the geometric structural KAT + unmapped-field-empty
   assertion + one-time rendered-sample eyeball + one-time Acrobat check. → I3 (+C1 item 3).

**Not R0-GREEN. Fold C1/C2 + I1–I5 and re-review.** The design core survives all findings — no restart
needed, but the 2025-form facts must be corrected from the actual PDFs before any implementation.
