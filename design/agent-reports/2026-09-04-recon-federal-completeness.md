# Recon — what's missing from a COMPLETE US federal return, whole-return (not bitcoin-specific)

**Date:** 2026-09-04. **Scope:** recon only, no code touched. Every claim below is either a command's
literal output, a source citation with line numbers, or a quote from a repo-committed doc (mostly
`crates/btctax-cli/LIMITATIONS.md`, which is `include_str!`'d verbatim into the shipped `btctax
limitations` command — i.e. it is a live artifact, not a stale design note).

---

## 1. Forms/schedules btctax can fill TODAY, per tax year

**The compiler-enforced universe of form types** is `PrintedForms` in
`crates/btctax-core/src/tax/packet.rs:483-529` — 17 named form slots: `f1040`, `sch_1/2/3/a/b/c`,
`sch_d`, `f8949`, `sch_se`, `f8959`, `f8960`, `f8995`, `f8995a`, `f6251`, `f8283`, `f8275`. Two are
mutual alternatives (`f8995`/`f8995a`, exactly one `Some` on any return that claims §199A —
`packet.rs:509`).

**The compiler-checked census** (`crates/btctax-forms/tests/census.rs::census_is_exactly_15_forms_including_8275_when_a_promote_is_present`)
pushes an all-arms `PrintedReturn` through `fill_full_return` and asserts the emitted form-name set
equals the 17-key `CENSUS_KEYS` list minus the two forms that cannot co-occur with the fixture's other
arms (`f8995a`, `f6251`) — i.e. **exactly 15 forms emitted from one fixture**; `packet.rs`'s
no-`..`-destructure in `fill_full_return` makes a new/unfilled arm a compile error, so this ceiling
cannot silently rot.

**Per-year fillable-PDF bundles** (`crates/btctax-forms/forms/<year>/`, each a `.map.toml` +
`.pdf` pair, verified by `ls`):

| year | forms with a map+PDF | what this means |
|---|---|---|
| **2024** | f1040, f1040s1, f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, f6251, f8275, f8283, f8949, f8959, f8960, f8995, f8995a, schedule_d, schedule_se — **16 map/PDF pairs** | **the only year with the full packet.** `crates/btctax-cli/LIMITATIONS.md:63-66`: "Filled as an official IRS PDF … for a year with full-return inputs: the whole packet — Form 1040 and Schedules 1, 2, 3, A, B, C, D, SE, plus Forms 6251, 8949, 8959, 8960, 8995/8995-A and (when required) 8283." |
| **2025** | f1040, f8283, f8949, schedule_d, schedule_se — **5 pairs only** | **no full return.** Only the "crypto slice" (disposals + SE tax + the 1040's capital-gain line, not the whole 1040). |
| **2017** | f1040, f8283, f8949, schedule_d, schedule_se — **5 pairs** | same crypto-slice-only shape, legacy year. |

This is not merely a missing-asset gap — it's structural and fails closed at TWO independent layers:
- **PDF-fill layer:** `crates/btctax-forms/src/form1040_full.rs:53-59`, `need()`: *"the TY{year} Form
  1040 map has no `{what}` — the full-return fill needs it. Full-return v1 is TY2024-only."*
- **Tax-computation layer, independently:** `crates/btctax-cli/src/resolve.rs:249` and
  `crates/btctax-cli/src/input_form_store.rs:262,292,734` — a full-return commit for a year with no
  bundled tables/params returns `CommitOutcome::NoTables` and **writes nothing** ("v1 bundles TY2024
  only — I-11").

**Line-level coverage, machine-counted** (`cargo run -p xtask -- line-coverage`, exact output):

```
line-coverage OK: 279 money lines across 16 form(s)
[f1040:45 f1040s1:10 f1040s2:6 f1040s3:5 f1040sa:19 f1040sb:5 f1040sc:7 f1040sd:19 f1040sse:22
 f6251:41 f8949:12 f8959:17 f8960:15 f8995:16 f8995a:39 i1040gi:1],
15 exception(s) (ratchet 15), 0 unverifiable (ratchet 0), 8 not line-bound (ratchet 8)
```
This asserts every one of those 279 money-bearing lines quotes its own form's own instruction text
verbatim (rule (2b), the Form-6251-line-33 defense) — it is a conformance test, not an estimate.

**Field-level completeness is a THIRD, stricter axis** — see §2(b) below; a form being "in the 15/16"
does not mean every box on it is accounted for.

`design/forms/MANIFEST.json` archives **102 authorities** (34 form PDFs, 28 instructions, 12 guidance,
6 publications, 6 regulations, 16 statutes) — this is the *reference corpus* the transcription is
checked against, and it is intentionally wider than what's emitted (it includes e.g. `f1040s8`
(Schedule 8812) and `f8283--2025.pdf`, archived for future work but **not** wired into any `.map.toml`
today).

---

## 2. What a typical filer needs that btctax CANNOT produce

Classified per the recon brief's three buckets, sourced primarily from `LIMITATIONS.md`'s own
(i)/(ii)/(iii) split, cross-checked against source.

### (a) Not modeled at all

**No struct field exists** — verified by reading `Form1040Lines` end to end
(`crates/btctax-core/src/tax/printed.rs:502-581`): fields run `line1z, line1a, line2a, line2b, line3a,
line3b, line7, line8, line9 … line37`. **There is no line4a/4b/4c (IRA distributions), no line5a/5b
(pensions/annuities), no line6a/6b/6c (Social Security).** `LIMITATIONS.md:294`: *"Retirement /
pension / IRA / annuity / Social Security income (1040 lines 4a–6b; 1099-R, SSA-1099)"* is listed
under **(iii) UNREPRESENTABLE — no input exists.**

Also **(iii) UNREPRESENTABLE**, quoted verbatim from `LIMITATIONS.md:290-305`:
- Schedule E (rental, royalty, partnership/S-corp K-1) and Schedule F (farm)
- A non-crypto Schedule C (any other self-employment) and a second SE earner
- Marketplace health coverage / excess APTC (Form 8962, Schedule 2 line 1a) — "there is no input for
  it; if there were, it would refuse"
- Non-passive foreign tax (a Form 1116 category other than passive)
- State/local returns; e-filing
- Any tax year other than TY2024 for the full packet, and the TY2025 Schedule 1-A
- "Any line requiring a worksheet v1 does not model"

**(a), computed-but-hardcoded** — a distinct sub-shape: **CTC/ODC** (Schedule 8812). `Form1040Lines::line19`
doc comment (`printed.rs:544-546`): *"CTC / credit for other dependents. **Always 0** (a §3.4
conservative omission…)"*. `map.rs:493-497` — the `ctc` checkbox is *"NEVER checked: v1 omits CTC/ODC
entirely."* No Schedule 8812 map/PDF exists in any `forms/<year>/` directory. Same shape for **EIC**
(line 27, blank), **education credits** (Form 8863), **dependent-care** (Form 2441), **saver's credit**
(Form 8880), **energy credits** (Form 5695), **adoption credit** (Form 8839) —
`advisories.rs:448-451`. All fire a loud advisory (`LIMITATIONS.md` table at line 210-222); none is
silent.

**AMT — a partial case worth flagging on its own** (see §5): Form 6251 Part I models only lines 1, 2a,
2b (`LIMITATIONS.md:274-281`); the other 18 add-back lines (2c–2t) are unmodeled. The single largest
one in practice — an **ISO exercise still held at year-end (line 2i)** — and the AMT capital-loss/
depreciation/mortgage variants are each covered by an explicit declaration that **refuses** rather than
silently zeroing (`return_refuse.rs`, the §G-22/B11 scope attestation, question text at
`questions.rs:546-556`).

### (b) Modeled but not emitted — the field-provenance census

This is the sharper, box-level finding, and it is the repo's own instrument
(`design/forms/FIELD_PROVENANCE.md`, `crates/btctax-forms/tests/field_census_slice.rs`). A *form*
being in the 15-form census does not mean every *box* on it has a decided provenance.

**Static census (TY2024, every form's full AcroForm field set vs. the committed map), §6:**

```
form            fields  mapped  UNACCOUNTED
f1040             141      87      54
f1040s1            69      12      57
f1040s2            60       6      54
f1040s3            39       7      32
f1040sa            37      23      14
f1040sb            72      68        4
f1040sc           105      13      92
f8275              95      53      42
f8283             117      59      58
f8949             244     238        6
f8959              26      19        7
f8960              38      16      22
f8995              33      20      13
schedule_d         55      27      28
schedule_se        27      14      13
─────────────────────────────────────
TOTAL           1158     662     496
```

**Vertical slice on one real household** (`kitchen_sink`, TY2024, `field_census_slice.rs`, §6h):
13 of 15 forms emit (not f8283/f8275); of 946 emitted AcroForm fields, 550 are mapped and **396 are
unaccounted** — "most of the static surface really is present," the doc's own correction to an earlier
over-estimate.

★ **496/396 is explicitly NOT a defect count** (FIELD_PROVENANCE.md §6): it's "boxes with no recorded
decision" — could be `not-applicable`, `not-ours` (signature/preparer blocks), or the genuine defect
class (never asked, never modeled). The doc's own region breakdown (§6a) estimates ~45% is
header/trailer boilerplate (signature block, third-party designee, preparer block — dispositioned by a
handful of blanket rules) and ~55% is per-line body content that needs real classification.

★★ **FR-15 (open):** *"`design/forms/FIELD_PROVENANCE.md` is a dated snapshot with no generator and no
test."* The 496/396 numbers can silently go stale — there is no CI check re-deriving them.

### (c) Deliberately refused with a stated reason

Extensive and fail-closed (`LIMITATIONS.md:224-288`, `RefuseReason` enum in `return_refuse.rs`).
Selected list, verbatim reasons: foreign trust (Form 3520); dual-status alien; foreign tax over the
§904(j) $300/$600 ceiling (Form 1116); HSA activity affirmed (Form 8889 out of scope); IRA deduction
claimed (`return_refuse.rs:1272-1276`, "the active-participant phase-out worksheet is unmodeled");
Schedule C net loss (§465 at-risk); Form 8615 kiddie tax; >14/15 Schedule B payers; >$500 non-crypto
noncash gift with insufficient Form 8283 detail; a Form 6251 add-back declared present (ISO exercise,
mortgaged non-AMT dwelling, AMT capital-loss carryover divergence, AMT depreciation divergence);
clergy self-employment (Schedule SE exemption unmodeled — "use a preparer"); the catch-all §G-22/B11
scope attestation, whose text (`questions.rs:546-556`) enumerates named yes-conditions **plus** a
catch-all — *"a business this tool did not capture, or anything else it never asked about"* — so a
truthful filer with, say, 1099-R income who answers this question honestly is refused rather than
silently omitted, even though there is no dedicated retirement-income question.

---

## 3. Common filer vs. unusual filer

**Common-filer profile from the prompt: W-2 wages, interest, dividends, a house, kids, retirement
accounts.**

| item | status | blocks the return? |
|---|---|---|
| W-2 wages, 1099-INT, 1099-DIV | fully modeled, filed | no |
| a house (mortgage interest, SALT via Schedule A) | modeled; itemization computed | no — but several sub-questions REFUSE if unanswered (debt-ceiling §163(h)(3)(B), acquisition-use §163(h)(3)(F), mixed-use) |
| **kids (Child Tax Credit / ODC)** | **not computed at all** — line 19 hardcoded $0, advisory only | **no refusal, but return files with tax OVERSTATED** if the family actually qualifies; filer must hand-complete Schedule 8812 |
| **retirement accounts (1099-R / pension / SSA-1099)** | **no input surface exists** (no `Form1040Lines` field) | **effectively YES for TY2024** via the catch-all scope attestation (a truthful "yes" refuses); a filer who misreads the prompt's examples (none of which name retirement income by name) and answers "no" gets a return silently missing that income — **the catch-all is the only thing standing between this and a silent gap** |
| **filing TY2025 (the year due in the current filing season)** | **the FULL packet cannot be produced at all** — only the crypto-slice (8949/Sch D/Sch SE/partial 1040) | **effectively yes** — a common filer trying to file the return actually due this year gets no complete 1040 from btctax, full stop |
| low-income / EIC | not computed, advisory only, no refusal | no (but leaves refund money on the table) |

**Unusual-filer gaps mostly REFUSE cleanly** rather than producing a silently wrong return: rental/K-1/
farm income (Schedule E/F), a second self-employed earner, non-passive foreign tax, an HSA event, a
foreign trust, dual-status alien status, clergy SE exemption, an ISO exercise, a Schedule C loss, kiddie
tax, marketplace ACA subsidies. This matches the repo's stated fail-closed doctrine and is corroborated
structurally, not just by prose: `return_refuse.rs` is exhaustively destructured against
`ReturnInputs` (`return_refuse.rs:378-403`, "a new `ReturnInputs` field breaks this destructure until
it is classified").

**Net assessment:** for the *common* profile, the two live gaps are (1) **CTC/ODC**, which doesn't
block filing but silently costs money unless the filer reads the advisory and self-files Schedule
8812, and (2) **TY2025 full-return capacity**, which is an outright capability gap for anyone filing
the current season's return. Retirement income is a near-miss — covered by a broad catch-all
refusal rather than a dedicated question, which is fail-closed today but fragile to a filer who
reasonably reads the prompt's named examples as exhaustive.

---

## 4. The repo's own open-work list relevant to completeness

- **FR-1** (`FOLLOWUPS.md:5351-5359`) — *"N3: 1040 line 19 prints `0` for families the credit belongs
  to."* **NOT BUILT.** Names the exact three-part fix (`line19: Option<Usd>`, exposing
  `ctc_provably_zero`, blank-propagation through lines 21/22) and records that the forms lane
  deliberately refused to build a second, divergent §24(b) implementation inside `btctax-forms`.
- **FR-16** (`FOLLOWUPS.md:5572-5581`) — **EITC/ACTC. "DELIBERATELY NOT STARTED in this branch."**
  Owner decision 11 put it in scope, but "the plan itself says it 'is not a plan item at this scope —
  it is a project,' to start at brainstorm." Needs Schedule 8812 **and** Schedule EIC maps (neither
  exists), a refundable-credit path that doesn't exist yet, new collected inputs (earned income, the
  §32(i) investment-income limit, qualifying-child residency), and a two-oracle witness. De-risked by
  `design/direction/ORACLE-TRAP-credit-takeup.md` (Tax-Calculator's default EITC=$0 trap, machine-
  verified, position-dependent).
- **FR-15** (`FOLLOWUPS.md:5566-5570`) — the field-provenance census (§2b above) is a hand-run
  snapshot with **no generator, no test** — can silently drift stale.
- **SPEC `design/SPEC_full_return.md` §1.2** itself names, as v1 out-of-scope: CTC/ODC/education/
  dependent-care/saver's/energy/adoption credits (advisory), Schedule E/F, retirement/IRA/pension/SS
  income, Form 1116-scale foreign tax, Form 3520, Form 8615 kiddie tax, Schedule C loss, Form 8962,
  state returns, e-file, TY2025 + Schedule 1-A, "any line needing an unmodeled worksheet." **This list
  is the primary source for §2 above and matches `LIMITATIONS.md` closely — except for AMT (see §5).**
- **TY2025 Schedule 1-A / OBBBA provisions** (tips, overtime, car-loan interest, senior deduction) —
  per project memory, spec+plan green, partially built (T1), not complete; tracked as its own track in
  `CONTINUITY.md`'s AMT/TY2025 history, separate from the full-return TY2024 packet.
- **FR-21 / FR-22** (`FOLLOWUPS.md:5452-5491`) — two of the repo's own completeness/drift *checkers*
  (a doc-drift grep, `cite_check`'s plain-quotation pass) are themselves "proven blind" — relevant not
  to filing completeness directly but to how much the repo's own green signals can be trusted.
- **§G-18, now RESOLVED** (`FOLLOWUPS.md:872-926`) — see §5, kept here because it's the model case for
  "a completeness fix was itself wrong."

---

## 5. Claims of completeness that don't hold up (or didn't, until corrected)

- **`design/SPEC_full_return.md` §1.2 is stale about AMT.** It lists *"AMT computation (screen-only,
  §4.11)"* under v1 **out of scope**. Current reality, verified independently three ways: (1)
  `packet.rs:512-518`, `PrintedForms.f6251` doc comment — *"btctax has computed this form for every
  return since v0.14.0 — what was missing was the ability to FILE it"* [now fixed]; (2)
  `crates/btctax-forms/forms/2024/f6251.map.toml` + `.pdf` exist and line-coverage counts 41 f6251
  money lines; (3) `LIMITATIONS.md:64` lists Form 6251 in the filed packet. The SPEC undersells actual
  capability (safe direction), but it is exactly the "doctrine written down and violated" shape this
  repo's own harness (`design/HARNESS.md` class-β) exists to catch, and it was not caught here — the
  SPEC section was not updated when AMT filing shipped.
- **The field-provenance census (§2b) is the sharpest self-aware instance in the repo.** Its own text:
  *"496 is NOT a defect count… the open work is classifying them."* A shallower reading of "15/15 forms
  emit, line-coverage says OK" would conclude the return is complete; the census exists precisely to
  show that **form-level and line-level completeness do not imply box-level completeness** — 396 of 946
  emitted fields on one real household still have no recorded decision.
- **§G-18** (`FOLLOWUPS.md:872-926`), now resolved, is the model case of a *previous* completeness fix
  being wrong: Form 1040 line 7's "if not required, check here" box was changed from blank to
  conditionally-checked based on `ScheduleDLines::must_file()` — which answers *"does btctax's model
  need a Schedule D,"* not *"does the form require one."* A filer with an input btctax cannot see (Forms
  6252/4684/6781/8824/4797/2439, a K-1) would have had the box assert a false negative under §6065.
  Reverted same-day; the repo's own lesson, verbatim: *"Filling a blank is not automatically an
  improvement… ask first: can btctax establish the proposition the mark asserts, or only that it has no
  evidence against it?"* — directly on point for how easy it is to mistake "the box now has a value" for
  "the return is more complete."
- **`crates/btctax-forms/tests/common/mod.rs:16` `CENSUS_KEYS` array is declared `[&str; 17]`** while
  the surrounding doc comments and test name say "15 forms" — reconciled correctly in the test body
  (17 total form-name keys minus the 2 that cannot co-occur in that specific fixture = 15 emitted), but
  worth flagging as a place where a quick read of "17" vs "15" looks like a discrepancy; it is not, once
  the mutual-exclusion arms (`f8995a`/`f6251`, `f8995`/`f8995a`) are accounted for.

---

## Bottom line

btctax's **TY2024** full-return path is the only one that can emit a complete 15-form federal packet
today (1040 + Sch 1/2/3/A/B/C/D/SE + 6251/8949/8959/8960/8995(-A)/8283/8275), verified by a compiler-
enforced form census and a 279-line conformance checker. **TY2025 — the return currently due — has no
full-return capability at all**, gated fail-closed at both the tax-table layer and the PDF-map layer.
Within TY2024, the biggest gaps for an ordinary household are **Child Tax Credit / Additional Child Tax
Credit / EIC** (advisory-only, overstates tax, doesn't block filing) and **retirement/pension/Social
Security income** (no input surface, caught only by a broad catch-all refusal rather than a dedicated
question). Unusual-filer income (rental, K-1, farm, non-passive foreign tax, HSA activity, kiddie tax,
etc.) fails closed via explicit refusals. Below the form level, the repo's own field-provenance census
shows real forms still carry hundreds of AcroForm boxes with no recorded fill/skip decision — the
harder, self-acknowledged, and currently un-instrumented (FR-15) completeness question.
