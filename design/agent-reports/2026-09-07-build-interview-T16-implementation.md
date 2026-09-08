# T16 build — Form 8889 (Health Savings Accounts), transcribed

Implementer report. Branch `main`, shared tree, HEAD `8767b937` (nothing committed). Brief:
`design/agent-reports/BRIEF-build-interview-T16.md`. Contract: `design/SPEC_interview.md` r2 §7 row
T16, R2, R3, R4, R10, R14; `FOLLOWUPS.md` FR-76.

**Status: all six numbered items COMPLETE.** Whole workspace green, fmt clean, clippy clean, every
conformance gate OK.

---

## 0. What the interruption left, and how I verified it

The controlling process exited mid-build and was restarted; my own working context survived intact,
so no state had to be reconstructed from the tree. I nonetheless re-established it the way the
coordinator asked:

| check | result |
|---|---|
| `git status --short \| wc -l` | 119 (74 modified + 45 untracked) |
| `git diff --stat` | 74 files, +3601 / −159 |
| planted-defect residue (`grep -rn "PLANT:" crates/`) | **one hit, and it is pre-existing** — `cite_check.rs:1258`, confirmed present at HEAD by `git show HEAD:crates/xtask/src/cite_check.rs \| grep -c` = 1. None of my own plants survived; each was restored from a `cp` backup under the scratchpad, never `git checkout --`. |
| `cargo build --workspace` | clean |
| `cargo nextest run --locked --workspace` | 3388 passed, 12 skipped, 0 failed |

**The controller's own uncommitted files were left untouched.** `CONTINUITY.md` and
`design/ROADMAP_STATUS.md` are modified in the tree and neither diff is mine (I read
`git diff design/ROADMAP_STATUS.md` to confirm: it is the T6-CLOSED / T16-BUILDING row). The
`design/agent-reports/BRIEF-*.md` and `2026-09-07-build-interview-T6-reverify.md` untracked files
are likewise the controller's.

The one design document I did edit is `design/TY2026_WORK_LIST.md` — mechanically required, because
`form_delta::tests::the_committed_work_list_matches_form_delta_at_head` reds on a bundled stem with
no row in either table (§ *"Pinned numbers moved"* below).

---

## 1. Archive

Ten IRS documents plus three Revenue Procedures. Every sha256 and byte count is **measured**
(`sha256sum` / `stat -c%s` on the fetched bytes), and every revision is **read off the document's
own text**, never assumed from the filename.

| stem | edition | revision, read off the document | sha256 (12) | bytes |
|---|---|---|---|---|
| `f8889` | 2024 | footer *"Form 8889 (2024)"* (`extract:61`); face year 2024 (`:4`). ANNUAL. Attachment Sequence No. **52** (`:8`) | `a93c1f651b26` | 74,152 |
| `f8889` | 2025 | footer *"Form 8889 (2025) Created 3/28/25"* (`:61`). ANNUAL, IRS-final (0 hits for "draft") | `15ed4587f75b` | 74,277 |
| `i8889` | 2024 | *"Instructions for Form 8889 (2024) Catalog Number 37971Y"* (`:117`). ANNUAL | `accd65700f75` | 217,639 |
| `i8889` | 2025 | *"Instructions for Form 8889 (2025) Catalog Number 37971Y"* (`:123`) | `da6f2f588b06` | 216,817 |
| `f5498sa` | 2024 | face year 2024 (`:7`), box 2 *"Total contributions made in 2024"* (`:9`). **ANNUAL** — no "Rev." anywhere in the document | `07409c2db1e2` | 79,913 |
| `f5498sa` | 2025 | face year 2025 (`:7`); box 2 *"…made in 2025"* (`:9`) | `3506c7849bd7` | 79,992 |
| `f1099sa` | **2019** | *"(Rev. November 2019)"* (`:6`); footer *"Form 1099-SA (Rev. 11-2019)"* (`:28`). **PERIODIC** | `1cc3c9fe2416` | 90,823 |
| `f1099sa` | 2025 | *"(Rev. April 2025)"* (`:6`); footer *"(Rev. 4-2025)"* (`:26`). PERIODIC | `cc2324629a9b` | 79,460 |
| `i1099sa` | 2024 | *"Instructions for Forms 1099-SA and 5498-SA (2024)"* (`:249`). ANNUAL | `a8779d33b4a5` | 160,995 |
| `i1099sa` | 2025 | *"…(2025)"* (`:242`) | `e2c8bba99bc0` | 154,197 |
| `RevProc_2023-23` | — | §2.01(1): *"For calendar year 2024 … $4,150 … $8,300"* | `cdc4574bde94` | 58,027 |
| `RevProc_2024-25` | — | §2.01(1): 2025 → $4,300 / $8,550 | `6093d2e3923f` | 55,683 |
| `RevProc_2025-19` | — | §2.01(1): 2026 → $4,400 / $8,750 | `ff2ba4bac62e` | 65,328 |

### ★★★ The TY2024 Form 1099-SA is `irs-prior/f1099sa--2019.pdf`, and no Wayback was needed

The brief expected the T2 Form-1098 procedure (picklist, then a Wayback capture of the moving
`irs-pdf` URL). It was not required, and the reason is measured rather than assumed. I probed
`irs-prior/f1099sa--{2015..2023,2025}.pdf`: **2015–2019 are 200, 2020–2023 are 404, 2025 is 200.**
`f1099sa--2019` is the *"(Rev. November 2019)"* continuous-use edition, so by the IRS's own rule
(*"the year of the revision date is the first year for which issuers are to use the form to report
amounts"* — `f1098e--2026.txt:2-5`) it governs from TY2019 until Rev. April 2025 displaces it. That
is the Form 1099-SA a TY2024 filer holds, furnished by 2025-01-31, months before the April 2025
revision existed. It lives in a **new** `design/forms/2019/` directory, per the archive's rule that
a periodic edition is filed under its own revision year (the `f1098--2022` precedent).

★ `irs-pdf/f1099sa.pdf` and `irs-prior/f1099sa--2025.pdf` are **byte-identical**
(`cc2324629a9bd416…`, both measured 2026-09-07), which is the evidence that no later revision has
displaced Rev. April 2025.

### Derived artifacts

- **Extracts** — `design/forms/extract/` gained ten files. Forms `pdftotext -layout`, instruction
  booklets plain `pdftotext` — the convention **measured** against the committed T2 extracts, not
  guessed: `pdftotext -layout i1099int--2024.pdf` differs from the committed extract and plain
  `pdftotext` is byte-identical to it; for `f1099int--2024` `-layout` is byte-exact.
- **Geometry** — `f8889--2024.json` (1129 words, 27 boxes) and `f8889--2025.json` (1131, 27).
- **In-crate fixtures** — `crates/btctax-core/src/tax/fixtures/f8889_{2024,2025}_{form,instructions}.txt`,
  generated by `xtask extract-schedule-1a` from two new `cite_check::FORMS` rows. **Form 8889 reached
  step 2 of the archive ladder the day it was transcribed**, which is what makes the transcription
  checkable: `line-coverage` reads these fixtures.
- **`authority-manifest --regen`** → *"regenerated 183 entries"*, then verify → *"OK — every entry
  resolves and every source is listed"*. `archive-check` OK.
- `design/forms/README.md`: archived-document count **115 → 125** (measured
  `ls design/forms/*/*.pdf.txt | wc -l`), and `design/forms/2019/` added to the layout table.
- `legal/`: `_scripts/fetch_hsa_limit_revprocs.sh` (re-runnable, same `dl` helper and provenance log
  as its siblings), three text layers, `SHA256SUMS` 47 → 50 lines, `sha256sum -c` **50 OK**.

**★ A finding recorded in `legal/SOURCES.md` rather than silently fixed.** Its *"Files: 42
documents"* line was stale — `find legal/primary-sources -type f | wc -l` is **58**, and
`SHA256SUMS` covers **50** of them. I replaced the hand-count with the measured number plus the
command that produces it, and named the eight uncovered files (four `SSA_COLA_Determinations_*`,
`RevProc_2016-55/2023-34/2024-40/2025-32` — all recorded in `design/forms/MANIFEST.json`, so hashed
once rather than twice). Reconciling the two hash lists is **not T16's**; it is recorded so a future
reader does not read "50 OK" as "all of them".

---

## 2. Form 8889 as a transcription struct

`crates/btctax-core/src/tax/form8889.rs` — 21 numbered lines in the form's own numbering, each
carrying the printed text verbatim as its doc comment, plus the instructions' **Employer
Contribution Worksheet** as its own struct.

### ★★ One struct serves TY2024 and TY2025, and that is measured

`diff` of the two extracts with the year and the §223(b) figures normalised leaves exactly two
differences: a period after *"(see instructions)"* on line 13, and the 2025 footer's
`Created 3/28/25`. The LINE SET is identical, and
`diff <(label-boxes f8889--2024) <(label-boxes f8889--2025)` is **empty** — same 27 AcroForm fields,
same line assignments. A per-revision fork would have been a fork with no line in it.

### The line table — line → production → reach

`xtask line-coverage` verifies every quotation verbatim against `f8889--2024.txt`, and every
`Collected(FilerRecords)` sentence against a `Line N` block of `i8889--2024.txt`.

| line | production | source / reach |
|---|---|---|
| 1 (checkbox) | — | `hsa.family_coverage`, a class-(A) declaration |
| 2 | `Collected(FilerRecords)` | i8889 *"Include on line 2 only those amounts you, or others on your behalf, contributed to your HSA for 2024."* |
| 3 | `Constant` | §223(b)(2) from `FullReturnParams::hsa` + the 55+ amount when it lands here |
| 4 | `Carry` | Form 8853 — **always blank by decision**; any Archer MSA refuses |
| 5 | `Clamped(FloorAtZero)` | *"Subtract line 4 from line 3. If zero or less, enter -0-"* |
| 6 | `Carry` | line 5 (the two-spouse allocation refuses) |
| 7 | `Constant` | §223(b)(3)(B) $1,000, married + family coverage only |
| 8 | `Combine` | *"Add lines 6 and 7"* |
| 9 | `DocBox(fw2, 12a)` | **Form W-2 box 12 code W**, through the Employer Contribution Worksheet |
| 10 | `Collected(FilerRecords)` | i8889's *"A distribution from your traditional IRA or Roth IRA to your HSA…"* |
| 11 | `Combine` | *"Add lines 9 and 10"* |
| 12 | `Clamped(FloorAtZero)` | *"Subtract line 11 from line 8…"* |
| 13 | `Bounded` | i8889 *"enter the smaller of line 2 or line 12"* → **Schedule 1 line 13** |
| 14a | `DocBox(f1099sa, 1)` | Σ Form 1099-SA box 1 over the HSA rows |
| 14b | `Collected(FilerRecords)` | *"Include on line 14b any distributions you received in 2024 that qualified as a rollover…"* |
| 14c | `Combine` | *"Subtract line 14b from line 14a"* |
| 15 | `Collected(FilerRecords)` | *"In general, include on line 15 distributions from all HSAs in 2024 that were used for the qualified medical expenses"* |
| 16 | `Clamped(FloorAtZero)` | *"Taxable HSA distributions…"* → **Schedule 1 line 8f** |
| 17a (checkbox) | — | derived from the excepted amount, which is the form's own sentence |
| 17b | `Scaled` | 20% of line 16 less the excepted part → **Schedule 2 line 17c** |
| 18 | `Collected(FilerRecords)` | *"Enter on line 18 the excess of the amount contributed over the redetermined amount."* |
| 19 | `Collected(FilerRecords)` | *"Enter the total of any qualified HSA funding distribution (see line 10)."* |
| 20 | `Combine` | *"Total income. Add lines 18 and 19…"* → **Schedule 1 line 8f** |
| 21 | `Scaled` | *"Multiply line 20 by 10% (0.10)…"* → **Schedule 2 line 17d** |
| W1–W5 | `Exception` ×5 | the Employer Contribution Worksheet — instructions only, so `(none)` line, no quotation |

★ **No exception was needed on any of Form 8889's own 21 lines.** That is the transcription rule
paying off rather than luck: a form written for a person to follow has a production for every line.

`line-coverage` now reports **f8889:27** money lines (22 form lines + 5 worksheet lines) inside
*"373 money lines across 18 form(s) … 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)"*.

### Params

`FullReturnParams` gained `hsa: HsaParams` — three fields, each cited:

| field | TY2024 | TY2026 | authority |
|---|---|---|---|
| `self_only_limit` | $4,150 | $4,400 | §223(b)(2)(A); Rev. Proc. 2023-23 / 2025-19 §2.01(1). ★ $4,150 is also what `f8889--2024.txt:22` prints on line 3 |
| `family_limit` | $8,300 | $8,750 | §223(b)(2)(B), same procedures |
| `additional_contribution_55` | $1,000 | $1,000 | **§223(b)(3)(B) — statutory, NOT indexed**, which is why it does not move |

★ The doc comment records **why these are not in the autumn inflation Rev. Proc.**: §223(g) requires
publication by June 1 of the *preceding* year, so the HSA amounts get their own spring procedure a
year and a half ahead of the return. The TY2026 figures are therefore already known while the TY2026
FORM is not published at all.

`shipped_tables_are_the_validated_tables.rs` compares `s_hsa` against `v_hsa` — the two are
transcribed independently from the same Rev. Proc., exactly as every other figure in that test is.

---

## 3. The document screens

### `Form1099Sa` — box census, both editions

Rev. 11-2019 and Rev. 4-2025 print **identical captions**, so one entry per box lists both editions.

| box | caption (verbatim from the extract) | decision |
|---|---|---|
| 1 | `1 Gross distribution` | `Collected(Sa1099Box1GrossDistribution)` → Form 8889 line 14a |
| 2 | `2 Earnings on excess cont.` | `RefuseIfNonzero(OtherIncomeLine8zNotModeled)` — the form's own *Instructions for Recipient*: *"Include the earnings on the 'Other income' line of your tax return"* |
| 3 | `3 Distribution code` | `Collected(Sa1099Box3DistributionCode)` — a code, not an amount |
| 4 | `4 FMV on date of death` | `RefuseIfNonzero(OtherIncomeLine8zNotModeled)` — a non-spouse beneficiary *"must report as income … the FMV"* |
| 5 | `5 HSA` (quoted to its first printed line) | `Collected(Sa1099Box5AccountType)` — the box that decides **which form**; unanswered refuses |

`box-census`: *"f1099sa--2019 (i1099sa instructions): 5 boxes — 3 collected, 2 refuse-if-nonzero,
0 not read"*, and the same for `--2025`.

### `Form5498Sa` — box census, both editions

An **annual** form, so boxes 2 and 3 print their own tax year in the caption and split per edition —
which is exactly the visible split the per-edition census exists for.

| box | 2024 caption | 2025 caption | decision |
|---|---|---|---|
| 1 | `1 Employee’s or self-` (wrapped) | same | `Collected(Sa5498Box1ArcherContributions)` |
| 2 | `2 Total contributions made in 2024` | `…in 2025` | `Collected(Sa5498Box2TotalContributions)` — **not** Form 8889 line 2 |
| 3 | `3 Total HSA or Archer MSA contributions made in 2025 for 2024` | `…2026 for 2025` | `Collected(Sa5498Box3NextYearForThisYear)` |
| 4 | `4 Rollover contributions` | same | `Collected(Sa5498Box4Rollover)` — line 2's instruction excludes rollovers by name |
| 5 | `5 Fair market value of HSA,` (wrapped) | same | `Collected(Sa5498Box5Fmv)` — no line reads it |
| 6 | `6 HSA` | same | `Collected(Sa5498Box6AccountType)` |

`box-census` OK: **268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)** — up from 246 / 15 / 7.

### ★★ Why nothing on the 5498-SA is summed, stated in the form's own words

Form 8889 line 2 asks for *"HSA contributions **you** made … Do not include employer contributions,
contributions through a cafeteria plan, or rollovers"*, while box 2 is the trustee's employer-and-
employee total by **calendar** year. Adding box 2 to line 2 would double-count the employer's share
(line 9 already carries it from the W-2) and silently include rollovers. The row is transcribed so
the filer can **check** line 2 against it, and `requires_transcription(Sa5498)` is therefore
`false` — the 1099-B's shape, not the 1099-G's.

### Census rows

`DocumentRow` gained `Sa1099` and `Sa5498` (18 → **20** rows), each with a prompt in the form's own
attribution, a `FormQuestion`, a `Vec` for `declared_rows`, and a `row_is_pre_named` arm. Both are
supported (no §2.2 exit sentence) and both have an `entry_route`.
`requires_transcription`: `Sa1099` **true** (nothing else carries an HSA distribution onto the
return, and what it hides is gross income plus a 20% tax), `Sa5498` **false** (see above).

### The `hsa_activity` join

`Some(true)` no longer refuses — it **opens** the form and makes Form 8889's seven questions live.
`Some(false)` beside a transcribed HSA row refuses `DocumentCensusContradicted` through the existing
generic rule. `Some(true)` with a declared-but-untranscribed 1099-SA refuses
`DocumentDeclaredNotTranscribed`. The W-2 box 12 code-W amount is **read, never re-asked**.

---

## 4. The map, the emitter, and the four reach lines

- **`crates/btctax-forms/forms/{2024,2025}/f8889.map.toml`** + the bundled PDFs. All 22 money lines,
  the identity pair and the three checkboxes are mapped; the `[census]` is **empty**, which is a
  claim rather than an omission — btctax fills Form 8889 completely, and every situation the form
  routes elsewhere refuses instead.
- **`Form8889Map`** (`map.rs`), `Stem::F8889`, `LineSet::F8889_{2024,2025}`, `Schema::Form8889Map`,
  `pdf::f8889_pdf`, `bundled::template`/`map_text` — the whole year-package chain.
- **`crates/btctax-forms/src/form8889.rs`** — the emitter, read back through `verify_flat` with a
  code-side column oracle `[(410.4, 481.6), (504.0, 576.0)]`: lines 9 and 10 sit in the MID cluster
  (the form insets them so line 11 can add them), everything else in AMOUNT.
- **`YEAR.toml`** 2024 and 2025 gained `f8889`; 2026 gained a `forms_absent` reason.
- **Attachment sequence 52**, read off `f8889--2024.txt:8` — held to the map row by
  `map_rows::packet_sequences_agree_with_every_map_row`.
- **`PrintedForms::f8889`** is `Some` exactly when `Form8889::must_file` — the §223 trigger
  declaration, never a threshold over the figures.

### The four reach lines, now mapped and filled

| line | cell (TY2024) | source |
|---|---|---|
| Schedule 1 **8f** *"Income from Form 8889"* | `f1_17[0]` | Form 8889 line 16 **+ line 20** |
| Schedule 1 **13** *"Health savings account deduction. Attach Form 8889"* | `f2_03[0]` | Form 8889 line 13 |
| Schedule 2 **17c** *"Additional tax on HSA distributions…"* | `f2_04[0]` | Form 8889 line 17b |
| Schedule 2 **17d** *"…because you didn't remain an eligible individual"* | `f2_05[0]` | Form 8889 line 21 |

Schedule 2 also gained **line 18** (*"Total additional taxes. Add lines 17a through 17z"*), which
line 21 sums. 17c and 17d are `Option<Usd>` and the emitter **skips the whole 17x block** when no
Form 8889 is attached — line 2's rule, for line 2's reason: a printed `0` would swear the filer
figured a tax on a form the IRS never receives.

Their five census entries retired (2 on `f1040s1--2024`, 3 each on `f1040s2--2024` and `--2025`).
`census-join` fell **298 → 290**, which is exactly the eight lines now modelled — verified per file:
`f1040s1--2024` 56 → 54, `f1040s2--2024` 52 → 49, `f1040s2--2025` 54 → 51. The `[[direction]]`
blocks are unchanged and `census-join` still passes.

---

## 5. The oracles

`GoldenInputs` gained `hsa_deduction`, wired to **both** engines:

- Tax-Calculator `e03290` (*"Health savings account deduction from Form 8889"*, verified against
  `taxcalc/records_variables.json`) in `gen_goldens.py`;
- OTS `S1_13` (*"Health savings account deduction. Attach Form 8889"*, verified in
  `OpenTaxSolver2024_22.07_linux64/tax_form_files/US_1040/US_1040_template.txt:157` and the 2025
  template `:191`) in `ots_direct.py`.

`build_golden_return` gives btctax the **contribution**, not the deduction, and lets Form 8889 line
13 produce the deduction — which is what makes the cell discriminating rather than a pass-through.

**Corpus cell `single_w2_with_an_hsa_deduction`** (`corpus.py` `NON_INTERACTION`): Single, $95,000
wages, $4,150 contributed to a self-only HSA. Regenerated with both engines live
(`OTS_DIR=… .venv/bin/python scripts/oracle/gen_goldens.py`, 107 candidates → **107 admitted, 0
rejected**), and the two agree:

```
OTS   AGI 90850.0  TI 76250.0  total 11834.0
TAXC  AGI 90850.0  TI 76250.0  total 11828.0
```

(the $6 gap is the pre-existing Tax-Table-vs-formula methodology difference the corpus already
carries). btctax reconciles against both — `btctax-core` golden tests 7/7. The corpus diff is
**purely additive**: one household added, none removed, none changed.

★ The cell sits **at** the §223(b) ceiling deliberately, and the `why` says so: neither oracle models
the limit, so one dollar more and btctax would refuse while both engines deducted the excess — the
V2b hazard `charitable_cash` records for §170(b).

★ **T11 is not built**, so `project_to_golden` / `ORACLE_INVISIBLE` do not exist yet; nothing was
added to a predecessor list. The oracle harness (`btctax-oracle-harness`) was taught to read Form
8889 back (`map_for("f8889")`) — it panics on a packet member with no line-map, and did.

---

## 6. Refusals — only where the form sends the filer elsewhere

`RefuseReason::HsaActivityUnsupported` is **deleted**. Eight variants replace it, each naming where
the form sends the filer:

| variant | fires when | the form's own words |
|---|---|---|
| `Form8889Unanswered { question }` | any of the seven Form 8889 questions is blank | (the registry's unanswered screen) |
| `HsaLine3WorksheetRequired` | not eligible every month with the same coverage, **or** Medicare for any month | line 3: *"All others, see the instructions for the amount to enter"* → the **Line 3 Limitation Chart and Worksheet** |
| `HsaSeparateForm8889Required` | both spouses have separate HSAs | *"Complete a separate Form 8889 for each spouse"* |
| `ArcherOrMaMsaNeedsForm8853(String)` | line 4's declaration, or a 1099-SA box 5 / 5498-SA box 6 saying Archer/MA MSA | *"Before you begin: Complete Form 8853 … if required"* |
| `HsaExcessContributionsNeedForm5329` | line 2 > line 13 | *"…you may have to pay an additional tax on the excess contributions … See Form 5329"* |
| `HsaExcessEmployerContributions` | line 9 > line 8 − line 10 | *"you must report it as 'Other income' on your tax return"* → Schedule 1 line 8z |
| `HsaTestingPeriodFailureNotComputed` | Part III's gate affirmed | line 18 needs *"the Line 3 Limitation Chart and Worksheet … **for the year the contribution was made**"* |
| `SaAccountTypeNotTranscribed(String)` | box 5 / box 6 never transcribed | the box that decides which form; `None` never defaults to HSA |

★ Every one is the **understatement** direction if guessed, so none can be an advisory (§3.4).

### ★★★ A finding the compiler forced: W-2 box 12 code W

Code W (*"Employer contributions to your Health Savings Account"*) was **absent from
`INERT_BOX12_CODES`**, so a W-2 carrying it refused `UnsupportedBox12Code` — the correct answer while
no form read it, and the wrong one the moment Form 8889 line 9 does. Found by the package-fixture
census, which reported `Some("UnsupportedBox12Code")` where it expected
`HsaExcessEmployerContributions`.

Code W is now admitted, and a **new** rule closes what admitting it opened:
`HsaEmployerContributionWithoutActivity` — a code-W amount beside `hsa_activity != Some(true)` is the
W-2 and the declaration saying opposite things about the same fact, and §223(a) requires the form
whenever a contribution is made.

---

## Kills — every one seen RED once, on a planted defect, with the red quoted

Thirteen tests in `form8889.rs`, three in `full_return_forms.rs`, one in `open_next_year_t4b.rs`,
plus six new refusal fixtures in the source-derived census. Each plant was reverted from a `cp`
backup under the scratchpad.

**1. The contribution limit comes from the year's params.** Planted `dec!(4300)` (TY2025's figure)
in place of `params.self_only_limit`:

```
a_contribution_over_the_years_limit_refuses_naming_form_5329 panicked at form8889.rs:474:
assertion `left == right` failed: line 3 is the year's own §223(b) figure
  left: 4300   right: 4150
```

**2. ★★★ Part III's reach — the plant that found a HOLE and closed it.** Deleting `+ self.line20`
from `schedule_1_line_8f()` left **all twelve other kills green**, because `compute` can never
produce a non-zero Part III (a testing-period failure refuses). A term no test can distinguish from
its absence is a guarantee that does not exist. I added
`part_iii_reaches_schedule_1_line_8f_and_schedule_2_line_17d`, which builds the struct directly, and
re-planted:

```
part_iii_reaches_schedule_1_line_8f_and_schedule_2_line_17d panicked at form8889.rs:764:
assertion `left == right` failed: line 8f is line 16 PLUS line 20 — the form routes both there,
and reading only one drops the whole testing-period inclusion from income
  left: 1800   right: 5300
```

**3. Schedule 1 line 8f reads the form.** Planted `let line8f = Usd::ZERO;`:

```
an_unqualified_distribution_reaches_schedule_1_line_8f_and_schedule_2_line_17c panicked at
form8889.rs:531: assertion failed: Schedule 1 line 8f carries line 16
  left: 0   right: 1800
```

**4. Schedule 2 line 18 sums the 17x block.** Planted `let line18 = Usd::ZERO;`:

```
…panicked at form8889.rs:539: Schedule 2 files on an HSA additional tax alone
```

**5. Line 17b honours the exception.** Planted `dec!(0.20) * line16`:

```
the_line_17a_box_is_checked_by_the_excepted_amount_and_17b_taxes_the_rest panicked at
form8889.rs:574: assertion failed: 20% of 1,800 − 800
  left: 360.00   right: 200
```

**6. Line 9 reads the W-2, never re-asks.** Planted `let w1 = Usd::ZERO;`:

```
line_9_reads_w2_box_12_code_w_through_the_worksheet panicked at form8889.rs:681:
assertion failed: worksheet line 1 is the W-2's box 12 code W
  left: 0   right: 1000
```

**7. The §223(b)(3)(B) line-3/line-7 split.** Planted `if false {` in
`additional_contribution_split`:

```
the_age_55_amount_lands_on_line_3_or_line_7_by_marital_status_and_coverage panicked at
form8889.rs:719: assertion failed: line 3 is the bare family limit
  left: 9300   right: 8300
```

**8. The box census — a deleted entry.** Deleted the `f1099sa` box-3 entry:

```
f1099sa--2019:
  box 3 ("3 Distribution code") is printed on the form and NOTHING decides it — we forgot this
  box, which is invisible on the page and to every value assertion
f1099sa--2025: (same)
```

**9. The box census — a caption the extract does not carry.** Quoted the 5498-SA box-5 caption
*across the layout wrap*:

```
f5498sa--2024:
  box 5's caption does not match the extract: census "5 Fair market value of HSA, Archer MSA, or
  MA MSA" vs printed "5 Fair market value of HSA,"
f5498sa--2025: (same)
```

**10. `line-coverage` — a one-character quotation drift.** Changed Form 8889 line 16's quote from
*"line 14c"* to *"line 14b"*:

```
xtask line-coverage: line-coverage FAILED (1 problem(s)):
  - f8889:16 (line16) quotes text NOT FOUND in f8889--2024.txt:
      "Taxable HSA distributions. Subtract line 15 from line 14b. If zero or less, enter -0-. …"
```

**11. `line-coverage` — a deleted production.** Deleting Form 8889 line 13's `c.line(...)` does not
even compile, which is stronger than a red test:

```
error: unused variable: `line13`
  --> crates/btctax-core/src/tax/line_coverage.rs:1652:9
```

**12. The map cells, read back from the filled PDF — a transposition.** Swapped `line14b` and
`line14c` in the TY2024 map:

```
form_8889_fills_every_line_and_the_three_checkboxes panicked:
Geometry("ordinal-y descent broken: …f1_17[0] (y 294.0) is not strictly above …f1_16[0] (y 306.0)
— mis-mapped row/line")
```

**13. The map cells — an on-state copied by analogy.** Gave line 1's *Family* box the *Self-only*
on-state `"1"`:

```
Geometry("…c1_1[1]: on-state \"1\" is not one this widget declares ([\"2\"]) — writing it would
render the box BLANK on the filed form while reading back as checked. This is what a swapped
Yes/No map looks like.")
```

**14. The opener seeds an identity, never a box.** Planted `.cloned()` in place of the identity-only
seed:

```
an_hsa_trustee_is_seeded_as_an_identity_with_every_box_blank panicked at
open_next_year_t4b.rs:1412: assertion failed: ★ every BOX is blank — including box 5, whose
carried value would be testimony about paper the filer has not seen.
  left:  Form1099Sa { …, box1_gross_distribution: 1800, box3_distribution_code: "1",
                      box5_account_type: Some(Hsa) }
  right: Form1099Sa { …, box1_gross_distribution: 0, box3_distribution_code: "",
                      box5_account_type: None }
```

**15–20. The refusal fixtures** are not hand-written tests but entries in the source-derived census
(`every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` and
`a_package_dependent_rule_fires_at_commit_and_is_silent_without_the_package`). Removing a rule from
the body removes it from the census and leaves its fixture refusing nothing — red on both halves.
Six were added: `Form8889Unanswered` (superseded by
`HsaEmployerContributionWithoutActivity` — see below), `SaAccountTypeNotTranscribed`,
`ArcherOrMaMsaNeedsForm8853`, `HsaSeparateForm8889Required`, `HsaLine3WorksheetRequired`,
`HsaTestingPeriodFailureNotComputed`, plus the two package-gated `HsaExcess*` rules.

Also covered by named tests: `a_declared_1099_sa_with_no_row_refuses_and_one_row_files`,
`a_no_beside_a_transcribed_hsa_row_refuses_the_contradiction`,
`the_account_type_checkbox_refuses_unanswered_and_refuses_an_archer_msa`,
`an_all_zero_form_8889_still_files_because_the_declaration_says_so`,
`a_w2_code_w_beside_a_no_hsa_declaration_refuses_the_contradiction`, and
`part_year_eligibility_medicare_and_two_spouses_each_refuse_naming_where_to_go`.

---

## Every pinned number moved — old → new, with cause

| where | old → new | cause |
|---|---|---|
| `design/forms/README.md` archived count | 115 → **125** | ten documents archived |
| `legal/SOURCES.md` file count | 42 → **58** (measured) | stale hand-count corrected; +3 Rev. Procs |
| `legal/SHA256SUMS` | 47 → **50** lines | three Rev. Procs |
| `QuestionId::ALL.len()` / `FORM_QUESTIONS.len()` | 41 → **50** | 2 census rows + 7 Form 8889 questions |
| `DocumentRow::ALL.len()` | 18 → **20** | `Sa1099`, `Sa5498` |
| `declarations_section_delegates…` decl count | 21 → **28**; fields 22 → **29** | the seven declarations |
| `coverage.rs` field count | 183 → **216**; covered 182 → **215** | 2 census rows + 8 + 9 document fields + 7 money + 7 declarations |
| `MAX_EXCEPTIONS` | 24 → **26** → **31** | +2 for Schedule 2 17c/17d (conditional entries); +5 for the Employer Contribution Worksheet's `(none)` rows |
| `MAX_UNLOCATABLE` | 12 → **17** | the same five `(none)` rows |
| `LineSet::ALL.len()` | 36 → **38** | `F8889_2024`, `F8889_2025` |
| `cite_check` map-row count | 36 → **38** | two `f8889.map.toml` files |
| `cite_check` planted-pair guard | 5 → **7** | two `FORMS` rows reached step 2 |
| `YEAR.toml` expected forms | 2024 19 → **20**; 2025 17 → **18** | `f8889` |
| `BUNDLED_FORMS_PER_YEAR` | (2024,19)/(2025,17) → **(2024,20)/(2025,18)** | same |
| `tax_report` answer-record count | 35 → **37** | the two HSA census rows |
| TUI-edit section clamp | 17 → **20** | three new sections |
| `the_rows_invariant…` countable set | 6 → **8** kinds | two new `Vec`s |
| `census-join` unmodeled entries | 298 → **290** | the eight lines now modelled |
| `box-census` | 246 boxes / 15 editions / 7 returns → **268 / 19 / 9** | four HSA form editions |
| `line-coverage` | 346 money lines / 17 forms → **373 / 18** | f8889:27 |
| `cite-check` extracted pairs | 5/36 → **7/38** | both Form 8889 years |
| goldens | 106 → **107** households | `single_w2_with_an_hsa_deduction` |

---

## Deviations from the brief

1. **No Wayback capture was needed for the TY2024 Form 1099-SA.** `irs-prior/f1099sa--2019.pdf`
   serves the Rev. November 2019 edition that governs TY2019–TY2024. Recorded in the note with all
   five 404 probes measured.
2. **`is_information_return_stem` was widened to the 5498 series.** Form 5498-SA classified as *not*
   an information return, so archiving it would have left it outside
   `archived_information_returns` — censused by nothing, with `box-census` still printing OK. That
   is I2's silence one series over.
3. **Enumerator rule 5 was widened**, because the contiguity guard red on a document it had never
   seen: *"the box numbers this enumerator found are not contiguous: [6..19] missing from 1..=20"*.
   The Rev. 11-2019 Form 1099-SA drops another column's vertical run (`MSA`) between *"For calendar
   year"* and its `20` stub, so the one-line lookback failed. The rule now waits for the stub
   anywhere after its caption and consumes exactly one run equal to `20` — the century prefix of
   `20__`. The interposed run is LAYOUT, the same distinction the wrapped-caption limit rests on.
4. **The HSA documents carry no *Attention* preamble at all**, so `face_block` needed a per-edition
   marker: the red-ink control number the form prints in its own top-left corner (`9494` on every
   1099-SA, `2727` on every 5498-SA), each asserted to occur exactly once. `VOID` will not do — it
   occurs **twice** in `f1099sa--2019` (Copy A and Copy C), measured.
5. **`HsaInputs.family_coverage` is an `Option<bool>`, not an `Option<HdhpCoverage>`**, so it is a
   class-(A) registry declaration the answered-ness invariant covers by construction. The form's two
   boxes are mutually exclusive, so one bool says which; `HdhpCoverage` remains the transcription
   struct's line-1 type. If anything ever did default, `false` is self-only — the lower limit, the
   direction that cannot overstate a deduction.
6. **W-2 box 12 code W was admitted, and a new contradiction rule added** (§6 above). Not in the
   brief; forced by the census, and it is a live correctness change.
7. **`design/TY2026_WORK_LIST.md` gained an `f8889` row** — required by
   `the_committed_work_list_matches_form_delta_at_head`.
8. **Part III can never be non-zero today** (a testing-period failure refuses), so lines 18–21 are
   structurally blank. Their reaches are pinned on a directly-constructed struct — see kill 2.

---

## Follow-ups worth filing

- **`legal/SHA256SUMS` covers 50 of the 58 files under `legal/primary-sources/`.** The eight
  uncovered are in `design/forms/MANIFEST.json`, so they are hashed — but two hash lists over one
  tree is the duplication the 2026-07-30 hybrid decision meant to end. Named in `SOURCES.md`.
- **TY2025 cannot yet compute a Form 8889** — `full_return_for(2025)` is `None` (the year is PAUSED),
  so the TY2025 map and emitter are exercised only by the direct-fill tests. The §223(b) TY2025
  figures ($4,300 / $8,550, Rev. Proc. 2024-25) are archived and ready for whoever unpauses it.
- **The Line 3 Limitation Chart and Worksheet is the one worksheet T16 does not carry.** Every path
  to it refuses; transcribing it would let a part-year or Medicare filer file.

---

## Suite lines

```
workspace              3388 tests run: 3388 passed, 12 skipped
btctax-core            1294 tests run: 1294 passed, 0 skipped
btctax-forms            359 tests run:  359 passed, 4 skipped
btctax-input-form        70 tests run:   70 passed, 0 skipped
btctax-cli              798 tests run:  798 passed, 1 skipped
btctax-adapters         103 tests run:  103 passed, 0 skipped
xtask                   157 tests run:  157 passed, 1 skipped
btctax-oracle-harness     5 tests run:    5 passed, 1 skipped
btctax-tui              160 tests run:  160 passed, 2 skipped
btctax-tui-edit         392 tests run:  392 passed, 2 skipped
```

Gates:

```
cargo fmt --all                                                        clean
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets
  --all-features -- -D warnings                                        clean
authority-manifest   OK — every entry resolves and every source is listed
archive-check        no primary source outside the 5 accounted-for tree(s)
box-census           OK: 268 printed boxes across 19 archived editions of 9 information returns
line-coverage        OK: 373 money lines across 18 form(s) … f8889:27
census-join          290 unmodeled entries across 13 maps, every one placed
cite-check           7/38 emitted (form, year) pairs; 31 excused, 0 unaccounted
prompt-check         OK — 20 assertions, all verbatim
stop-list            no forbidden shape
harness-check        OK — 2 hook(s) wired
authority-conflicts  0 entries recorded, 0 undecided, 0 overdue
scripts/pii-scan-generic.sh   clean (HEAD)
sha256sum -c legal/SHA256SUMS 50 OK
```

Nothing committed; nothing pushed. `CONTINUITY.md`, `design/ROADMAP_STATUS.md` and the controller's
`design/agent-reports/BRIEF-*.md` / `…-T6-reverify.md` files were not touched.

---

## Fold (seam review C-1, I-1, I-2, I-3, M-1, M-2, N-1)

Folder's report. Shared main tree, branch `main`, HEAD `b9c21f8f` at dispatch (nothing committed).
Brief: `design/agent-reports/BRIEF-fold-interview-T16-review.md`. Review:
`2026-09-07-build-interview-T16-review.md`; ledger `…-review-VERIFICATION.md`. Every plant below was
reverted from a `cp` backup under the scratchpad — never `git checkout --`.

**Every item folded, plus one defect of the same shape found while folding (§C-1b).** Workspace
green, fmt clean, clippy clean, every conformance gate OK, both oracles re-run live on the T16 cell.

### 1. C-1 — Schedule 1 line 8f enters AGI

`return_1040.rs`: the `form_8889` / `hsa_income_8f` / `hsa_deduction_13` derivation is **hoisted
above** `schedule_1_income` (it needs only `ri` and `params.hsa`), and `+ hsa_income_8f` is now one
of that sum's terms beside the state refund, unemployment, Schedule C net and crypto 8v. The comment
that pointed at *"`schedule_1_income` below"* — a binding that did not exist, 27 lines the other way
— is replaced by the reason the derivation sits where it does.

### 2. I-1 — Schedule 2 lines 17c/17d enter total tax

`schedule_2_other_taxes = se_tax_sch2_l4 + additional_medicare + niit + hsa_additional_taxes`, where
`hsa_additional_taxes = f.schedule_2_line_17c() + f.schedule_2_line_17d()` — the two legs the
printed lane already sums into line 18 and carries to 1040 line 23.

### 3. C-1b — Form 8889 line 13 is inside the §221 MAGI (found while folding, same function)

The Form 1040 instructions' *Student Loan Interest Deduction Worksheet—Schedule 1, Line 21*
(`i1040gi--2024.txt:42253+`) reads: **2.** *"Enter the amount from Form 1040 or 1040-SR, line 9"*;
**3.** *"Enter the total of the amounts from **Schedule 1, lines 11 through 20**, and 23 and 25"*;
**4.** *"Subtract line 3 from line 2"*. The HSA deduction is Schedule 1 **line 13**, so it is inside
step 3. T16 added it to `adjustments` and not to `agi_before_student_loan`; lines 15 (½-SE) and 18
(early withdrawal) were the only members btctax modelled before, so the omission is exactly T16's.
`agi_before_student_loan` now subtracts it, with the worksheet quoted at the site and a note that
this is a **block** — a future Schedule 1 lines 11–20 adjustment belongs there the day it is added.

★ **Direction and disclosure.** This one OVERSTATES tax (an inflated MAGI phases the deduction out
too fast), the opposite direction to C-1. It is reported rather than folded silently because it
moves the brief's own expected number: with the fixture's $2,000 contribution restored, §221(b)(2)
on the correct $92,000 MAGI gives **$500**, not the $167 the un-netted $94,000 gives. So the C-1
probe test contributes **$0** of its own (line 13 = $0, worksheet step 3 empty), which isolates the
income leg and reproduces the review's $1,667 → $167 exactly; C-1b has its own fixture and its own
kill.

### The two probes — before and after

| figure | fixture | before the fold | after |
|---|---|---|---|
| `AbsoluteReturn::agi` vs printed 1040 **line 11** | reach ($3,000 distributed, $1,200 qualified) | **58000** vs 59800 | 59800 = 59800 |
| `AbsoluteReturn::taxable_income` vs printed **line 15** | same | 43400 vs 45200 | 45200 = 45200 |
| `AbsoluteReturn::total_tax` vs printed **line 24** | same | **4979** vs 5555 (both legs missing); **5195** vs 5555 (I-1's leg alone) | 5555 = 5555 |
| printed Schedule 1 **line 21** | probe 2 ($85,000 wages, $2,500 of 1098-E interest, a $9,000 fully-excepted distribution, **no** own contribution) | **1667** | **167** |
| printed Schedule 1 **line 21** | probe 2 **with** the fixture's $2,000 contribution (C-1b) | 2000 → 167 | **500** |

Reds, verbatim:

```
an_unqualified_distribution_reaches_schedule_1_line_8f_and_schedule_2_line_17c panicked at
form8889.rs:577: assertion `left == right` failed: the absolute AGI and the FILED 1040 line 11 must
be the same number — line 8f is Schedule 1 PART I income and reaches line 9 through line 10
  left: 58000   right: 59800

…panicked at form8889.rs:602: …and total tax, which carries Schedule 2 lines 17c and 17d through
line 21 to 1040 line 23
  left: 5195   right: 5555

the_hsa_income_leg_moves_the_printed_student_loan_deduction panicked at form8889.rs:683:
assertion `left == right` failed: §221(b)(2) on a $94,000 MAGI: $2,500 × (95,000 − 94,000) / 15,000
= $167. It printed $1,667 while line 8f was missing from `total_income` — a $1,500 overstated
deduction
  left: 1667   right: 167

the_hsa_deduction_is_inside_the_section_221_magi panicked at form8889.rs:757:
assertion `left == right` failed: the worksheet's line 3 nets Schedule 1 lines 11 through 20, so the
MAGI is $92,000, not $94,000
  left: 167   right: 500          (and left: 2000 with C-1 also reverted)
```

### 4. The equality instrument is now STRUCTURAL

**How the maximal fixture reaches it.** The brief's first choice —
`btctax-input-form::spec::coverage::maximal_fixture` — is unreachable **and would be vacuous**:
`btctax-core` cannot depend on `btctax-input-form` (that is the dependency direction), and every
money leaf in that fixture is deliberately **zero** (its sentinels are what the mutate-and-diff
compares against), so an equality test over it would compare `$0` to `$0` on every line. So the
brief's stated alternative was taken — *"build an equivalent from `LEAF_SOURCE`'s money leaves with
a guard that every money leaf is non-zero, and say which"*:

`testonly.rs::every_money_leaf_household()` — derived in two steps, neither naming a field:

1. `scrub_axis::maximal_sentinel()` — the repo's maximal `ReturnInputs`: every `Option` `Some`,
   every `Vec` two rows, written as an exhaustive struct literal with **no `..`**, so a field added
   anywhere fails to compile *there*;
2. every leaf `provenance::leaf_walk::money_leaves` classifies as money — **by type**, by
   round-tripping a decimal probe through `Decimal`'s own deserializer — is overwritten with a
   distinct non-zero whole-dollar amount (`1_000 + 137 * i`).

Then `sch1.hsa_activity = Some(true)` (the sentinel answers it `Some(false)`, which closes Form 8889
entirely) and `answer_all_live_declarations`. **One ordering constraint is stated in the fixture**:
`hsa.line16_amount_meeting_an_exception = $1`, because line 17b is *"20% … of the distributions
included on line 16 that are subject to the additional 20% tax"* and the derived pass cannot know
that two of its leaves sit on opposite sides of one subtraction. The anti-vacuity guard is what
forced that to be said rather than discovered.

`packet.rs` now has `two_chain_households()` — kitchen sink, AMT-owing, **every money leaf** — each
carrying its own anti-vacuity guard (kitchen sink must **not** owe AMT; AMT-owing must; the
structural row asserts *every* money leaf is non-zero via `nonzero_money_leaves`, plus a non-zero
Schedule 1 line 8f and Schedule 2 line 17c). Two tests loop over it:
`the_absolute_total_tax_equals_the_printed_1040_line_24` and the new twin
`the_absolute_agi_equals_the_printed_1040_line_11` (which also asserts taxable income ↔ line 15).

★ **No tolerance was needed.** Exact equality holds on all three households, so the strictness the
original doc comment defends is intact.

**Kills — all three reds are on the structural row, and NO HSA household is named anywhere:**

```
(a) remove `+ hsa_income_8f` again →
    the_absolute_agi_equals_the_printed_1040_line_11 panicked at packet.rs:1572:
    assertion `left == right` failed: every money leaf: the absolute AGI and the FILED 1040 line 11
    must be the same number …
      left: 168809   right: 177577

(b) remove `+ hsa_additional_taxes` again →
    the_absolute_total_tax_equals_the_printed_1040_line_24 panicked at packet.rs:1544:
    assertion `left == right` failed: every money leaf: the absolute total tax and the FILED 1040
    line 24 must be the same number. …
      left: 0   right: 1753

(c) a BRAND-NEW money leaf, printed only — `ReturnInputs::plant_line8z` → `Schedule1Parts::plant_8z`
    → `printed::schedule_1_lines`'s line 9, and nothing else →
    the_absolute_agi_equals_the_printed_1040_line_11 panicked at packet.rs:1572: … every money leaf …
      left: 179347   right: 193362
```

(c) is the one that matters: nobody added a fixture, and the field did not exist when the test was
written. The derived pass populated it the moment it appeared.

### 5. I-2 — the code-W contradiction is keyed on `Some(false)`

`return_refuse.rs`: `ri.sch1.hsa_activity == Some(false)`. `None` is the registry's to block
(`HsaActivity` is `live: |_| true`), so the import tier — whose premise is that an unanswered
declaration is lawful there — lets a code-W W-2 in. Kill
(`an_unanswered_hsa_declaration_does_not_contradict_a_code_w_w2`), with the rule keyed back on
`!= Some(true)`:

```
panicked at form8889.rs:1279: assertion `left == right` failed: an UNANSWERED declaration beside a
code-W W-2 is a question waiting to be asked, not two statements that disagree — the import tier
must let the W-2 in
  left: Some(HsaEmployerContributionWithoutActivity)   right: None
```

The test pins all three states: `None` → no param-free refusal but `HsaActivityUnanswered` on the
answering tier; `Some(false)` → the contradiction, unchanged and as sharp as before.

### 6. I-3 — the spouse's plan is collected, and OR'd into coverage

- `HsaInputs::spouse_family_coverage: Option<bool>`, `#[serde(default)]`, doc-commented with both
  instruction sentences (`i8889--2024.txt:466-470` and `:497-499`).
- `QuestionId::HsaSpouseFamilyCoverage` (ordinal **50**), a class-(A) `FormQuestion` whose
  `unanswered` is `Form8889Unanswered { question }`, live iff `hsa_question_live && (Mfj | Mfs)` —
  **MFS included**, because the instruction says *"regardless of whether you file jointly or
  separately"* in as many words.
- `compute`: `coverage = family_coverage == Some(true) || spouse_family_coverage == Some(true)`,
  which fixes **line 1's box**, **line 3's base**, and — because the same `coverage` feeds
  `additional_contribution_split` — **line 7's** *"married, and you or your spouse had family
  coverage"* condition.
- The `HsaFamilyCoverage` prompt is widened to the instruction's own other two sentences (the
  different-times and same-time rules) and now says the spouse's plan is asked separately.
- Wired through `classifier.rs`, `seam.rs` (`FieldId::DeclHsaSpouseFamilyCoverage`),
  `registries.rs` (`decl_tristate!(50, …)` + both directions of the FieldId↔QuestionId map),
  `coverage.rs`'s leaf-path table, and the `scenario_for` builders in `return_refuse.rs` and
  `cmd/answer.rs`. **`LEAF_SOURCE` is not touched: it maps MONEY leaf prefixes and this is a bool**
  — the KAT audits money leaves only and stays green.

Kill (`a_spouses_family_plan_puts_a_self_only_filer_on_the_family_limit`), with the `||` removed:

```
panicked at form8889.rs:1129: assertion `left == right` failed: line 1's box follows "you and your
spouse", not this filer's own plan
  left: SelfOnly   right: Family
```

The test pins both directions on MFJ, taxpayer self-only, $6,000 contributed: spouse family →
line 3 = `params.hsa.family_limit`, line 13 = $6,000, **no** `HsaExcessContributionsNeedForm5329`;
both self-only → `self_only_limit` and the excess refusal. Plus liveness: not live for Single, live
for MFS.

### 7. M-1 — the document-less distribution door (R3's pattern)

- `ReturnInputs::hsa_distribution_without_1099sa: Option<bool>`, beside its three R3 siblings.
- `QuestionId::HsaDistributionWithout1099sa` (ordinal **51**), live iff
  `hsa_activity == Some(true) && documents.sa_1099 == Some(false)`, neutral `false`.
- `RefuseReason::HsaDistributionWithoutForm1099Sa` on a `Yes`, naming **Form 8889 LINE 14a** and
  quoting the trustee's own obligation from `i1099sa--2024.txt:70-73`: *"File Form 1099-SA,
  Distributions From an HSA, Archer MSA, or Medicare Advantage MSA, to report distributions made
  from a health savings account (HSA)"*. `attribute.rs` anchors it on its own declaration; a census
  fixture was added so the source-derived refusal census sees it on both paths.

Kill (`the_document_less_distribution_door_is_live_exactly_on_the_pair_and_names_the_1099_sa`), with
the refusal's condition falsified:

```
panicked at form8889.rs:1233: a distribution with no document must refuse
```

The test pins liveness on **all four** corners of the pair (trigger + census-No → live; a
transcribed 1099-SA → not; no trigger → not; trigger unanswered → not), the `None` block, the `No`
pass, and both quoted sentences of the refusal text.

### 8. M-2 — both editions of Form 8889 are checked

`xtask/line_coverage_check.rs` gains `EDITIONS_SHARING_ONE_TRANSCRIPTION` — the pairs this build
claims ONE transcription struct serves, currently one row (`f8889`, 2024 → 2025) — and rule **(4c)**:
every sentence the table quotes from the first edition must be verbatim in the second **with the tax
year substituted, and nothing else relaxed**. Measured: all 22 line-bound Form 8889 quotations pass
that way. The rule is a per-pair CLAIM and says so: `LineSet` alone would make eleven other forms
two-edition pairs whose TY2025 sentences nobody has checked, and adding rows without doing that work
would report a completeness the checker does not have. A form absent from the table entirely stays
`cover_fns_not_registered`'s business, so the kill tests' synthetic single-row tables are unaffected.

Kill, committed as
`a_sentence_missing_from_the_second_edition_is_caught_and_a_year_difference_is_not` (three
directions: the real table silent; a real drift shape — the 2024 §223(b) figures, which the 2025
revision moved to $4,300/$8,550 — red; a year-only difference silent). Watched red end to end on the
committed table too:

```
xtask line-coverage: line-coverage FAILED (2 problem(s)):
  - f8889:3 (line3) is quoted from f8889--2024 and this build serves f8889--2025 from the SAME
    struct (…), but the sentence is NOT in f8889--2025.txt with the year substituted:
      "were, or were considered, an eligible individual with the same coverage, enter $4,150 ($8,300 for"
```

### 9. N-1 — the line-9 census names every box-12 slot the code reads

`CollectedFrom::DocBox` gains `also_labels: &'static [&'static str]` (empty for every ordinary box);
`Production::doc_box_slots(stem, first, also)` is the constructor for a line that reads a REPEATING
box. Form 8889 line 9 is now `doc_box_slots("fw2", "12a", &["12b", "12c", "12d"])` — which is what
`employer_contributions_from_w2s` actually sums. The checker's DocBox arm was refactored into
`one_box_slot(...)` and loops over the whole set, so **every** slot must be printed by the edition
in force, decided by the box census, and captioned in the document's own words. The variant's doc
comment states the rule and why the narrower label was the defect.

Kill, committed as `every_box_12_slot_form_8889_line_9_reads_is_checked_and_a_bogus_slot_reds`
(the real row silent; `also_labels: &["12b","12c","12e"]` red naming 12e). Watched red end to end by
deleting the box-12d census entry:

```
xtask line-coverage: line-coverage FAILED (1 problem(s)):
  - f8889:9 (line9) names fw2--2024 box 12d, which that edition prints but the box census does not
    decide — a collected figure with no decided box is the 'we forgot this box' defect one layer up
```

### 10. The prompts are the documents' own words, and that is now a test

`prompt-check` is keyed on `SkippableId` and covers only Form 8615, so a `FormQuestion` prompt has
no checker. `form8889.rs::the_two_new_prompts_are_the_documents_own_words` asserts each of the six
clauses this fold put in front of a filer **twice** — verbatim in the in-crate text layer it is
sourced from (`fixtures/f8889_2024_instructions.txt`, `…_form.txt`) **and** verbatim in the
question's own `prompt`. Kill (one word changed, *"Disregard any plan"* → *"Disregard plans"*):

```
panicked at form8889.rs:1158: HsaSpouseFamilyCoverage: the prompt does not carry the clause verbatim.
  clause: "Use the family coverage amount if you or your spouse had an HDHP with family coverage.
           Disregard any plan with self-only coverage."
```

### The oracles

Both engines re-run **live** on the T16 corpus cell `single_w2_with_an_hsa_deduction`
(`OTS_DIR=/home/bcg/OpenTaxSolver2024_22.07_linux64 .venv/bin/python`), compared field-by-field
against the baked expectations:

```
OTS   AGI 90850.0  TI 76250.0  total 11834.0     drift vs baked: NONE
TAXC  AGI 90850.0  TI 76250.0  total 11828.0     drift vs baked: NONE
```

(the $6 gap is the pre-existing Tax-Table-vs-formula difference the corpus already carries).
`every_golden_household_matches_the_independent_oracles` 7/7; the golden file is **unchanged**.
★ The cell has no distribution and no student-loan interest, so neither C-1/I-1 nor C-1b moves it —
which is the same limit the cell's own `why` already records.

### Every pinned number moved

| where | old → new | cause |
|---|---|---|
| `QuestionId::ALL.len()` / `FORM_QUESTIONS.len()` | 50 → **52** | I-3's spouse declaration, M-1's door |
| `declarations_section_delegates…` decl count | 28 → **30**; section fields 29 → **31** | the same two |
| `coverage.rs` field count | 216 → **218**; covered 215 → **217** | the same two |
| `stop-list` registry prompts scanned | 73 → **75** | the same two (a reported figure; its floor is 50) |
| `docs/examples/examples.md` | +2 lines | the two new `null` leaves in a printed `ReturnInputs` |
| `btctax-core` tests | 1294 → **1301** | 7 new tests |
| `xtask` tests | 157 → **159** | the M-2 and N-1 kills |
| workspace tests | 3388 → **3397** | the 9 above |

**Counts that did NOT move, and that is the claim:** `line-coverage` **373 money lines / 18 forms /
f8889:27**, exceptions **31** (ratchet 31), unverifiable **0**, not-line-bound **17** (ratchet 17);
`box-census` **268 / 19 / 9**; `census-join` **290**; `cite-check` **51 quotations, 7/38 pairs, 0
unaccounted**; `prompt-check` **20 assertions**. No production and no line changed — M-2 adds a
second-edition check over the SAME rows, and N-1 widens one row's slot set without adding a row.

### Suite lines

```
workspace              3397 tests run: 3397 passed, 12 skipped
btctax-core            1301 tests run: 1301 passed, 0 skipped
btctax-forms            359 tests run:  359 passed, 4 skipped
btctax-input-form        70 tests run:   70 passed, 0 skipped
btctax-cli              798 tests run:  798 passed, 1 skipped
btctax-adapters         103 tests run:  103 passed, 0 skipped
xtask                   159 tests run:  159 passed, 1 skipped
btctax-oracle-harness     5 tests run:    5 passed, 1 skipped
btctax-tui              160 tests run:  160 passed, 2 skipped
btctax-tui-edit         392 tests run:  392 passed, 2 skipped
btctax-update-prices      5 tests run:    5 passed, 1 skipped
```

Gates:

```
cargo fmt --all --check                                                clean
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets
  --all-features -- -D warnings                                        clean
line-coverage        OK: 373 money lines across 18 form(s) … f8889:27
box-census           OK: 268 printed boxes across 19 archived editions of 9 information returns
census-join          290 unmodeled entries across 13 maps, every one placed
cite-check           OK — 51 quotations; 7/38 pairs, 31 excused, 0 unaccounted
prompt-check         OK — 20 assertions, all verbatim
stop-list            no forbidden shape
harness-check        OK — 2 hook(s) wired
archive-check        no primary source outside the 5 accounted-for tree(s)
authority-manifest   OK — every entry resolves and every source is listed
authority-conflicts  0 entries recorded, 0 undecided, 0 overdue
scripts/pii-scan-generic.sh   clean (HEAD)
```

### Deviations from the brief, and follow-ups worth filing

1. **The structural fixture is built from `maximal_sentinel` + `money_leaves`, not from
   `coverage.rs::maximal_fixture`** — the brief's own stated alternative. Two reasons, both
   measured: the crate dependency runs `btctax-input-form → btctax-core`, and that fixture's money
   leaves are all zero, so the comparison would have been vacuous. §4 says which.
2. **C-1b is an extra fix** (the §221 MAGI), not in the brief. It is the same seam, the same
   function and the same shape as C-1, the instructions settle it in one sentence, and leaving it
   would have printed a wrong Schedule 1 line 21 on any return with both an HSA deduction and
   student-loan interest. It moves the brief's expected $167 to $500 on a fixture that has both,
   which is why the C-1 probe fixture contributes $0 of its own and the two are tested separately.
3. **`crates/btctax-cli/tests/tax_profile.rs`'s `ALLOWED` list gained
   `btctax-core/src/tax/testonly.rs`** — the M-1 `serde_json::Value` enumeration. Audited and
   recorded inline: the `Value` dies inside `every_money_leaf_household`, which returns a typed
   `ReturnInputs`, so key order reaches no persisted or fingerprinted bytes. Same audit as
   `scrub_axis.rs` and `provenance.rs`, whose walks it is built on.
4. **Follow-up — extend M-2's second-edition check to the other two-edition forms.** Eleven forms
   (`f1040`, `f8949`, `f8959`, `f8960`, `f8995`, `f1040sa`, `f1040sb`, `f1040sc`, `f1040s2`,
   `f1040s3`, `schedule_d`/`schedule_se`) have a TY2025 `LineSet` served by the same schema and a
   TY2024-quoted table. Whether their sentences survive year substitution is unmeasured.
5. **Follow-up — `prompt-check` covers only `SkippableId`.** The 52 `FormQuestion` prompts have no
   checker; §10's test holds the six clauses this fold is answerable for and nothing more.
6. **Follow-up — the age-55 line-3/line-7 split now reads the OR'd coverage** and so is correct for
   a filer whose only family coverage is the spouse's, but no fixture exercises that combination;
   `the_age_55_amount_lands_on_line_3_or_line_7_by_marital_status_and_coverage` still drives it from
   `family_coverage` alone.

Nothing committed; nothing pushed. `CONTINUITY.md` was left untouched.
