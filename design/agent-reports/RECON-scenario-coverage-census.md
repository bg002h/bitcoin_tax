# RECON — the SCENARIO coverage census

**Agent:** opus, worktree-isolated, READ-ONLY. **Date:** 2026-09-13.
**Brief:** `design/agent-reports/BRIEF-scenario-coverage-census.md` (`ca342ef5e`).
**Owner ask:** *"look for tax scenarios that we do not cover so we can decide if permanently,
temporarily, or accidentally omitted."*

---

## 0. STOP-AND-REPORT — three §2 facts are wrong, one of them load-bearing

The brief's §5 asked me to refute §2 rather than build on it. Three corrections; the third changes
how the census had to be run.

| §2 claim | measured | how |
|---|---|---|
| `RefuseReason` variants = **140** | **128** | variant lines in the `pub enum RefuseReason` block, `return_refuse.rs:75` |
| `YEAR.toml [forms_absent]` = **13** entries | **25** across three years — 2024:**1**, 2025:**3**, 2026:**21** | TOML keys under each `[forms_absent]`. Distinct stems = 21 = `Stem::ALL`, since `forms_expected` plus `forms_absent` partitions it: 20+1, 18+3, 0+21 |
| ★ **EITC / §32 is absent and "no `RefuseReason` names it"** ⇒ offered as the open question | **REFUTED. EITC is the best-documented omission in the repo** | six independent provenance records, below |
| `LONG_RANGE_PLAN_filing.md` §7 has six subsections 7.1–7.6 as described | **CONFIRMED** verbatim (`:574`–`:635`) | — |
| 126 extracts / 70 geometry fixtures | **CONFIRMED** (126 / 70) | — |

### ★ Why the EITC refutation matters: the brief looked for the wrong instrument

§2 reasoned *"no `RefuseReason` names it ⇒ possibly unaccounted."* But `RefuseReason` is not where a
**forgone taxpayer-favourable** item is recorded — by design, since refusing one would be the wrong
answer. EITC's provenance:

| record | evidence |
|---|---|
| an `Advisory` | `Advisory::EicOmitted` (`advisories.rs:57`), text at `:565`–`:571` naming 8863/2441/8880/5695/8839 alongside |
| a census entry on the exact line | `forms/2024/f1040.map.toml:343` — line 27, `rule = "unmodeled"`, `covered_by = "Advisory::EicOmitted"`, reason: *"btctax computes no EIC; a forgone refundable credit can only overstate the tax"* |
| a follow-up with an owning phase | **FR-16**, `FOLLOWUPS.md:5601` — owning phase **P6** |
| a §7 subsection | **§7.5** (`:626`–`:634`): *"Schedule EIC … EITC in particular stays deferred"* |
| an owner ruling | **D-G** (`:656`): *"Stays deferred"* — §32(i)'s investment-income limit is exceeded by the capital gains |
| a filer-facing disclosure | `LIMITATIONS.md:318` |
| a quantified direction of harm | `LONG_RANGE_PLAN_filing.md:395` row **R12** — *"overstates"*, **$8,781** forfeited on a MFJ/$40k/2-child household |

**Verdict: EITC = TEMPORARILY OMITTED**, with more provenance than any other candidate in this census.
Same correction applies to §2's **1099-R** line: it is not unrefused — `DocumentRow::R1099` plus
`RefuseReason::DocumentTypeUnsupported { kind }`, with lines 4a–6b censused at
`f1040.map.toml:326`–`331`. Only §2's **Form 8606 / RMD = 0 mentions** survived (finding U6, and it is
narrow — see why).

---

## 1. How the candidate set was DERIVED, with counts per source

| # | source | measured size | unaccounted candidates it produced |
|---|---|---|---|
| **1** | Numbered lines of the 1040 and its schedules, from `design/forms/extract/*.txt` | **126** text layers, **59** distinct stems, **70** geometry fixtures | **0** — this source is closed by construction, see below |
| **2** | Every `Form NNNN` the 1040 instructions reference (`i1040gi--2024` + `--2025`, 91,502 lines) | **110** distinct form numbers; **24** distinct `Schedule X` references | **11** |
| **3** | The instructions' own decision points | Chart C's **7** conditions (`i1040gi--2025:732`–`770`); Charts A/B thresholds (`:698`–`:720`); **39** "who-must-file"-shaped sentences; the **Community Property States** section (`:2153`–`:2200`); **Claiming a refund for a deceased taxpayer** (`:23811`–`:23816`) | **3** — and the two highest-ranked findings came from here and from nowhere else |
| **4** | `Stem::ALL` vs each `YEAR.toml` | **21** stems; 2024 = 20 filled + 1 absent, 2025 = 18 + 3, 2026 = 0 + 21 | **0** — the partition is asserted |
| **5 ★** | **`DocumentRow::ALL`** — the instrument the brief did not name, and the actual scenario-granularity gate | **20** document families | **4**, including the structural finding |

### ★ Source 1 is closed by construction — which is why every finding comes from sources 2, 3 and 5

At **line/box** granularity, provenance is machine-enforced, not reviewed:

- `census_accounts_for_every_field` — `crates/btctax-forms/tests/field_census.rs:293`
- its B1 kill: `the_gate_reds_on_every_planted_defect` — `field_census.rs:338`
- `every_emittable_form_is_reached_by_the_gate_or_named_absent` — `field_census.rs:598`
- shrink-only ratchets: `recorded_gaps_may_only_shrink` (`:474`), `the_uncensused_register_may_only_shrink` (`:554`)
- the corpus it holds: **743** `rule = "unmodeled"` plus **35** `rule = "artifact"` plus **0** `rule = "gap"` entries across **38** `*.map.toml` files, each carrying a written reason and most a `covered_by` join.

So I did **not** re-run the field provenance census (1158 fields / 662 mapped / 496 undecided — cited,
not redone). The consequence for this census is precise and worth stating plainly:

> **A scenario that corresponds to a line on a bundled form cannot be accidentally omitted — the gate
> would red.** Therefore every accidental omission must be a scenario with **no field to census**.
> There are exactly three such shapes, and all my findings are one of them:
> (a) a **document family nobody is asked about**; (b) an **allocation rule that makes collected
> figures wrong** without leaving any line blank; (c) a **filing-mechanics requirement that is not an
> AcroForm field at all** — a notation written across the top, an attachment, a signature capacity.

### The two catch-alls that close most of sources 2 and 3

- **`QuestionId::OtherOutOfScopeIncome`** (`questions.rs:940`–`1000`) — a scope attestation whose
  prompt runs ~1,100 words and names, by form number, the understate-direction scenarios: Form 5329,
  Schedule H, 8962, 4137, 8919, 4797, 2555, 8853, 8814, 4972, 4255, 5405, 8611, 8697, 8621, 965-A,
  2210, 4361, 4029, 6252, 4684, 6781, 8824, 2439, 4136, NOL, and ISO / **Form 3921**. Unanswered gives
  `RefuseReason::OtherIncomeUnanswered`; affirmed gives `OtherIncomeOutOfScope`. **This is what makes
  the understate side fail-closed.**
- **Three unconditional advisories** for the overstate side: `OtherCreditsOmitted`,
  `UnmodeledDeductionsOmitted`, `UnmodeledReturnOptionsOmitted` (`advisories.rs:196`/`209`/`227`).

Chart C's 7 conditions therefore all land accounted-for: 1a AMT (bundled), 1b/1c/1d/1f (attestation
limb d), 1e (Schedule 2 line 13 `FAIL-CLOSED` box-12 allowlist), 2 (`Sa1099` plus `f8889` plus
`ArcherOrMaMsaNeedsForm8853`), 3 (Schedule SE bundled), 5 (`A1095`), 6 and 7 (limb d). **Chart C item
4, church-employee wages, is recorded too** — `schedule_se.map.toml:55` line 5a, and the clergy
archetype carries a model boundary statement at `:75`–`:93`: *"btctax models no clergy concept
ANYWHERE else … asking the question would imply support that does not exist."*

---

## 2. THE FOUR-BUCKET TABLE — every candidate in exactly one

### Bucket 1 — COVERED (a test names it)

| scenario | source | test that holds it |
|---|---|---|
| Whole-return figures vs **two independent oracles** | 1 | `every_golden_household_matches_the_independent_oracles` — `btctax-core/tests/golden_returns.rs:146` |
| Every field on every bundled form has determinate provenance | 1 | `census_accounts_for_every_field` — `btctax-forms/tests/field_census.rs:293`, killed by `the_gate_reds_on_every_planted_defect` (`:338`) |
| No emittable form escapes the gate | 1,4 | `every_emittable_form_is_reached_by_the_gate_or_named_absent` — `field_census.rs:598` |
| AMT / Form 6251 including Part III routing | 2 | `part_iii_is_blank_when_the_form_says_to_skip_it` and `part_iii_prints_in_full_when_line_7_routes_there` — `btctax-forms/tests/f6251_fill.rs:120`, `:143`; plus `f6251_map.rs`, `f6251_obbba.rs` |
| Digital-asset dispositions, 8949 / Schedule D | 1 | `kat_digital_asset_question.rs`, `kat_forms.rs`, `broker_boxes.rs`, `oracle_sweep_readback.rs` |
| Schedule 1-A (OBBBA tips / overtime / senior / auto-loan) | 2 | `btctax-core/tests/schedule_1a_compute.rs`, `schedule_1a_inputs.rs` |
| HSA / Form 8889 | 2,3 | `full_return_forms.rs`; 61 `RefuseReason` sites including `Form8889Unanswered`, `HsaLine3WorksheetRequired` |
| QBI 8995 / 8995-A | 2 | `f8995a_fill.rs`, `f8995a_map.rs`, and `an_above_threshold_return_files_8995a_instead` — `census.rs:347` |
| The printed packet, all 15 census forms | 1 | `census_is_exactly_15_forms_including_8275_when_a_promote_is_present` (`census.rs:127`), `every_census_form_demonstrated_in_j6` (`:412`) |
| All five filing statuses including **MFS** | 3 | `tax_compute.rs`, `full_return_forms.rs`, `f1040v_fill.rs`, `f4868_fill.rs`, `schedule_1a_compute.rs` |
| Document-family census is internally total | 5 | `every_document_kind_is_listed_once` — `provenance.rs:1246`; `every_document_family_that_carries_a_transcription_date_is_named` (`:1179`) |
| Puerto Rico excluded income | 2 | Schedule 1-A lines 2a–2e — `schedule_1a.rs:70`, `line_coverage.rs:3735` |
| Amended-return pointer after a promote | 2 | `promote_cli.rs:168` asserts the advisory names Form 1040-X |

### Bucket 2 — PERMANENTLY OMITTED (a policy ceiling)

| scenario | evidence |
|---|---|
| E-file / MeF transmission | **§7.1** (`:576`–`:602`) — paper is the channel; owner-settled |
| State returns: 40+ jurisdictions, conformity, CP allocation | **§7.2** (`:604`–`:610`) — owner-settled |
| A second §24(b) / §32 implementation in `btctax-forms` | **§7.3** (`:612`–`:616`) — owner-settled |
| Form 2210 / line 38 estimated-tax penalty | **§7.4** — blank is the i1040 default and the IRS bills it |
| Line 36 (apply overpayment to next year), third-party designee, email | **§7.4** plus `Advisory::UnmodeledReturnOptionsOmitted` |
| **Clergy self-employment** — Form 4361/4029, the "Exempt—Form 4361" notation | `schedule_se.map.toml:75`–`:93`, a written boundary: *"a lone checkbox would advertise support for a filer archetype the product cannot serve"* |
| Aggregate 1099-B with adjustments as the ceiling | owner ruling **FR-110** |
| Filer told to google the mailing address | owner ruling **FR-132** |
| No TY2025 return will ever be filed | owner ruling, 2026-09-05 |

### Bucket 3 — TEMPORARILY OMITTED (scheduled, with a reason)

| scenario | evidence |
|---|---|
| **EITC / ACTC** | FR-16 (owning phase **P6**) plus §7.5 plus owner **D-G** plus `Advisory::EicOmitted` plus `f1040.map.toml:343` plus R12 (*overstates*, $8,781) |
| Retirement income, 1040 lines 4a–6b — 1099-R, SSA-1099/RRB-1099 | `DocumentRow::R1099` / `Ssa1099` to `RefuseReason::DocumentTypeUnsupported`; `f1040.map.toml:326`–`331`; §7.5 |
| Rental / royalties (Schedule E), K-1 from partnership / S-corp / estate / trust | `DocumentRow::ScheduleERental`, `DocumentRow::K1`; §7.5 |
| Schedule 8812 (CTC/ACTC), Schedule EIC, Form 8962, Schedule E/F, Form 709, 8275-R, 1040-ES | **§7.5** "not never, but NOT NOW"; Schedule F also `RefuseReason::ScheduleFIncomeNotModeled` |
| Education, dependent-care, saver's, residential-energy and adoption credits — 8863, 2441, 8880, 5695, 8839 | `Advisory::OtherCreditsOmitted` (`advisories.rs:565`), direction stated: *"your tax is OVERSTATED"* |
| Schedule 1 Part II adjustments including **SE health insurance (Form 7206)** and IRA contributions; unmodelled Schedule A lines; Form 8815 | `Advisory::UnmodeledDeductionsOmitted` (`advisories.rs:572`) |
| **ISO exercise / Form 6251 line 2i** — CLAUDE.md's own standing example | `f6251.map.toml:161` (2024) and `:222` (2025): *"THE gap, and the standing example of this whole class… Needs Form 3921 boxes 3/4/5"*; refused via attestation **limb (b)**, which names Form 3921 explicitly |
| TY2025 `f1040s1` and `f8995a`; all 21 TY2026 stems | **25** `[forms_absent]` entries, each with a written reason — the January 2027 finals |
| Every Schedule 2 additional tax btctax does not compute, 24 lines | `f1040s2.map.toml` — each `rule = "unmodeled"` with a reason and `covered_by = "QuestionId::OtherOutOfScopeIncome"` |

### Bucket 4 — ★ ACCIDENTALLY OMITTED (nothing says anything) — the finding

Ranked in §3. Eleven candidates; **zero** matching files across `crates/**/*.rs` and
`crates/**/*.toml`, `FOLLOWUPS.md`, `LIMITATIONS.md`, `LONG_RANGE_PLAN_filing.md`, and all 38
`*.map.toml` census blocks.

---

## 3. THE UNACCOUNTED LIST — ranked by harm × plausibility

### U1 — Community-property allocation for MFS / RDP (Form 8958, §66, Pub. 555) · source 3

- **Direction of harm:** ★ **UNDERSTATES** for the lower-earning spouse — they report 100% of their own
  income where §66 requires half of the *combined* community income; **overstates** for the higher earner.
- **Plausibility: HIGH.** MFS is a fully built, tested filing status. The nine states include CA, TX, WA
  and AZ. `address_state` is **already collected** (`return_inputs.rs:891`), so the trigger is in hand.
- **Evidence of absence:** `"community property"`, `"Form 8958"` and `"8958"` = **0 files** across all of
  `crates/`. Repo-wide the only hits sit inside an accidentally-committed JSONL transcript
  (`design/no-testimony/reviews/SPEC-doctrine-opus-r2.md:931`), which is not a provenance statement.
- **IRS text**, `i1040gi--2025:2198`: *"Community property states include Arizona, California, Idaho,
  Louisiana, Nevada, New Mexico, Texas, Washington, and Wisconsin. If you and your spouse lived in a
  community property state, you must usually follow state law to determine what is community income."*
  `:2156` extends it to Nevada / Washington / California **registered domestic partners**.

### U2 — `Advisory::RefundByPaperCheck` asserts a fact the IRS retracted · source 3

- **Direction of harm:** the filer's **refund does not arrive** and they were told it would. Writing the
  numbers on the printed form by hand is offered only as an alternative, but is now the *only* route.
- **Plausibility: HIGH.** Fires on **every** refund return with no deposit block, in the first year this
  software will ever file.
- **Evidence:** `advisories.rs:619`–`626` states verbatim *"As filed, the IRS **will mail a check**."*
  `i1040gi--2025:23824` says: *"Starting in October 2025, the IRS will generally **stop issuing paper
  checks** for federal disbursements, **including tax refunds**, unless an exception applies."* Genuine
  drift — **0** occurrences in `i1040gi--2024`.
- ★ **§7.4's own ruling rests on the retracted premise**: *"Direct deposit / Form 8888 — a paper check is
  the disclosed consequence"*. TY2026 is the first filed year.

### U3 — Decedent final-return mechanics: the DECEASED notation and Form 1310 · source 3

- **Direction of harm:** prints a return **not processable as a decedent refund claim**, so the refund is
  withheld. The FR-102 shape *inverted* — it prints where it should speak.
- **Plausibility: MEDIUM-HIGH.** btctax **already knows**: it asks `SkippableId::TaxpayerDiedDuringYear`
  and `SpouseDiedDuringYear` (`questions.rs:3034`+), and **Qss presupposes a dead spouse**.
- **Evidence:** `"Form 1310"`, `"deceased"` and `"decedent"` = **0 files** in `crates/`; `"DECEASED"` = 0
  in every `f1040.map.toml`. The death questions exist but are scoped **only** to the §63(f) age-65
  addition (`packet.rs:567`).
- **IRS text**, `i1040gi--2025:23815`: *"All other filers requesting the deceased taxpayer's refund must
  file the return and **attach Form 1310**."* Section heading *Death of a Taxpayer* at `:844`.

### U4 — ★ `DocumentRow::ALL` membership is a hand-authored list of 20 on the scenario gate · source 5

- **Direction of harm: UNDERSTATES** — an unasked document family is an unrecognised income or
  disposition event.
- **Plausibility: MEDIUM**, structural, and it compounds every future year.
- **Evidence:** `document_census.rs:50`–`96`. The enum is internally total — an `ALL` const, exhaustive
  matches, and `every_document_kind_is_listed_once` — but **nothing derives the set from an authority**.
  No test reads the i1040gi information-return list. This is exactly `CLAUDE.md`'s *"Derive the list, or
  make the compiler hold it"* shape, sitting on the one gate every out-of-scope income scenario must
  pass through. Absent members are U5.

### U5 — Form 1099-A (foreclosure/abandonment — a disposition), 1099-Q (529), 1099-LTC · sources 2, 5

- **Direction of harm: UNDERSTATES** — for 1099-A, an unreported disposition carrying gain.
- **Plausibility: LOW-MEDIUM.**
- **Evidence:** `"1099-A"`, `"1099-Q"`, `"1099-LTC"` = **0 files** in `crates/`. Note that **1099-C is a
  `DocumentRow`** — and a foreclosure usually generates both, so the cancelled-debt half is asked and the
  disposition half is not.
- **Mitigation, stated honestly:** attestation limb (a) ends *"or anything else it never asked about"*,
  and its capital list names installment sale, casualty, §1256, like-kind and 2439 — **not** a
  foreclosure. So these are covered by an **unnamed tail**, never by name, and the named list is where a
  filer's recognition actually happens.

### U6 — Form 8606, nondeductible IRA basis / Roth conversion basis · source 2

- **Direction of harm: UNDERSTATES** in principle.
- **Plausibility: LOW** — the money paths are closed upstream: distributions by `DocumentRow::R1099`, the
  deduction by `RefuseReason::IraDeductionClaimed`. The residual is a nondeductible-contribution year
  with no distribution, where Form 8606 is filed but no 1040 figure moves.
- **Evidence:** `"Form 8606"` = **0 files** in `crates/`. This is §2's only surviving claim.

### U7–U11 — the recording residue

| # | scenario | source | direction of harm | plausibility | evidence |
|---|---|---|---|---|---|
| **U7** | **Form 8862** — EIC/CTC/AOTC claimed after a prior disallowance | 2 | **OVERSTATES**, the credit is denied without it | LOW while EITC/CTC are deferred; listed because nothing records the dependency | **0 files**; 52 references in `i1040gi` |
| **U8** | **Forms 8379** (injured spouse), **8857** (innocent spouse) | 2 | none to the computed tax; a forgone claim | LOW — filed alongside or after, arguably "not our concern" | **0 files** each. Defensible as out of scope; **nothing says so** |
| **U9** | **Forms 2848** (POA), **9000** (alternative media) | 2 | none / accessibility | LOW | **0 files** each. §7.4 records the *third-party designee* but not Form 2848 |
| **U10** | **Form 8689** (Virgin Islands) | 2 | understates | VERY LOW | **0 files**. Sibling territory cases **are** handled — Puerto Rico via Schedule 1-A 2a–2e, Form 4563 = 9 files — so this is an inconsistency inside an otherwise-covered class |
| **U11** | `LIMITATIONS.md` header reads **"Tax year supported: TY2024 only"** while TY2025 and TY2026 packages are bundled | 4 | misinforms about scope, in the filer-facing scope document | MEDIUM, doc-only | `LIMITATIONS.md:3`. Hand-written, not generated — only `service_center_check.rs:67` references the path |

**Not findings, recorded so the set is reviewable:** Form 1040-SR (identical line set —
`i1040gi--2025:2180`: *"The lines on Forms 1040 and 1040-SR are the same"*); Forms 940/941/943
(employer), 1065/1041 (entity), 4506/4506-T (transcripts), 8453 (e-file, §7.1), 9465/1127 (payment
arrangements), 8822 (address change), 1095-B/1095-C (informational, no return action), 1118
(corporate); and `Form 1062`, `Form 4547`, `Form 843`, `Form 172`, where the regex caught OCR artifacts
or index entries rather than real references.

---

## 4. The handful to put in front of the owner first

| rank | item | why this one, first |
|---|---|---|
| **1** | **U2 — the paper-check advisory** | The only finding that is **already wrong today, in shipped text, in the first year this software will file**. It is a two-line factual correction needing no new tax logic, and it invalidates the stated premise of a **§7.4 "DO NOT BUILD" ruling** — so it is a decision the owner has to make, not one that can be folded quietly. Cheapest fix, highest certainty, and dated October 2025, so it only gets more wrong. |
| **2** | **U1 — community property for MFS** | The only finding in the **understate** direction with **HIGH** plausibility and **zero** provenance of any kind. It is also the sharpest instance of this census's whole point: no line is blank, so `census_accounts_for_every_field` cannot see it and neither oracle would flag it — the figures are individually well-formed and collectively wrong. The discriminating input, `address_state`, is **already collected**, so even the minimum action needs no new collection. |
| **3** | **U3 — decedent final-return mechanics** | btctax **already asks whether the filer died** and already supports Qss, so this is not an unknown scenario — it is a known one whose consequences were scoped to a single deduction. That gap between "we asked" and "we acted" is what makes it a defect rather than an absence, and the harm — a withheld refund on a final return — lands on someone administering an estate. |
| **4** | **U4 — `DocumentRow` membership is hand-typed** | The only *structural* finding. It will not hurt anyone this year, and it is the mechanism by which U5-shaped gaps keep appearing: the repo has already paid for this exact disease twelve times (`CLAUDE.md`'s T8–T12 table). Worth a decision now precisely because the fix is cheap while the list is only 20 long. |

**Everything else (U5–U11) is recording work, not building work** — and the brief is right that recording
is the deliverable: a scenario btctax declines is fine once it is declined **by name**.

---

## 5. Method notes and limits of this census

- **Not re-derived:** the field provenance census — 1158 fields / 662 mapped / 496 undecided — cited only.
- **Not relitigated:** §7.1 e-file, §7.2 state, §7.3 second §32; FR-110, FR-132, the TY2025 ruling.
- **No implementation proposed** anywhere above, per §4 of the brief.
- **Absence was measured, not assumed.** Each U-row's "0 files" is `grep -rilE` over `crates/**` for
  `*.rs` and `*.toml` plus the four prose corpora, counted per **file**. My first pass mis-counted:
  `grep -c` over multiple files yields per-file counts, and `grep -n` line numbers look like form numbers
  — "8958" matched `return_refuse.rs:8958`. Every U-row was re-measured cleanly afterwards, and that
  error is why §0's numbers are stated together with their method.
- **Honest blind spot.** This census asks whether a position is *recorded*, not whether a recorded refusal
  actually **fires**. Three refusals were spot-checked for reachability: `R1099` / `Ssa1099` via
  `DocumentTypeUnsupported`, and Schedule H and ISO via attestation limbs (d) and (b), both pinned by
  `questions.rs:3770`'s limb assertion. A systematic reachability sweep over all 128 `RefuseReason`
  variants is a different instrument and was out of scope — it is the natural follow-on, and it is where
  an FR-102-shaped defect would live.
