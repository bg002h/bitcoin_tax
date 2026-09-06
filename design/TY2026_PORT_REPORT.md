# TY2026 port — the decision report

**Date:** 2026-09-05. **Sources:** six recon lenses, `design/agent-reports/2026-09-05-ty2026-port-{seams,constants,authority,maps,oracles,machine}.md`.
This document synthesizes; it does not restate. Every claim below is cited to the lens that measured it.

---

## 1. The one-line answer

**TY2025 is what stands between the TY2025 work and a TY2026 return.**

There is exactly one per-year gate on the entire full-return path — `full_return_for(year) -> Some`
(`crates/btctax-adapters/src/tax_tables.rs:99-104`, which inserts **2024 only**) — consumed at
`cmd/tax.rs:475/512/819`, `session.rs:526/569` and `resolve.rs:264`. TY2025 is still a deliberate
`None`. **No 1040 can be assembled for 2026 until it returns `Some` for 2025 first**, and every field
name, citation shape and changed instrument TY2025 is currently building (the §164(b) worksheet, the
Form 6251 1a/1b split, the Schedule 1-A emitter) is a TY2026 requirement too.

The second-order answer, which is the surprising one: **TY2026's *numbers* are almost all published
already.** All seven `TaxTable` fields are bundled and KAT-pinned, `btctax report --tax-year 2026`
computes and exits 0 today (seams §1), Rev. Proc. 2025-32 shipped Oct 2025, and the OBBBA statute
settles the rest. What waits on the IRS is the **forms** — and therefore everything that must be
*transcribed from* a form rather than looked up. That is the whole shape of the schedule.

---

## 2. What can start TODAY, ordered

Nothing in this section waits on the IRS. Ordered by *what unblocks what*, not by size.

### Tier 0 — do these BEFORE TY2025 params land, because that commit is what arms them

1. **Thread `year` into `form6251_inputs_from_parts`** (`return_1040.rs:2476-2493`) and dispatch
   `line1_rule` on it instead of the literal `Y2024`. Behaviour-preserving today. Land it with the test
   that reds when year and rule disagree — a grep of `crates/*/tests/` for `line1_rule` /
   `Form6251Line1Rule` returns **zero hits** (seams §4.1). *Highest-value single item on the list, and
   a pure refactor.*
2. **Close the Schedule 1-A range gate.** `schedule_1a_params` returns the identical 2025-shaped struct
   for 2025..=2028 (`tables.rs:1088`) and `Schedule1A` bakes the TY2025 line numbers into field names
   and emitted labels (`schedule_1a.rs:379-382`, `:545-546`). Either give it the per-year TYPE treatment
   `Form6251Line1` already has, or narrow the gate to `year == 2025`. **The one-line narrowing is the
   fail-closed direction and costs nothing.**
3. **Make `f1040_clusters` (`form1040.rs:34-39`) and `se_clusters` (`schedule_se.rs:38-43`)
   exhaustive.** They are the only two year functions in `btctax-forms` ending in `_ =>` rather than
   `UnsupportedYear`, and they are the map-independent geometry *oracle* `verify_flat` checks against.
   The one instrument built to distrust the map is the one that would not be told the year changed.
4. **Add `Form6251Line1Rule::Y2026`** as free compiler-driven recon — the enum is not
   `#[non_exhaustive]`, so the variant enumerates every site that must move before its contents are
   known (seams §5.8).

### Tier 1 — build the instruments, then use them

5. **Promote the line↔printed-label join into `map_pdf_conformance.rs`** as a derived filesystem walk.
   The instrument (`xtask label-boxes`) and the fixtures (`design/forms/geometry/`, 12 files) exist; the
   maps lens watched it go **RED on a planted blind copy (12/41 mismatches on f6251) and GREEN on the
   real batch (130 lines, 0 mismatches)**, and the machine lens independently reproduced **177 of 177**
   committed TY2025 bindings with zero disagreements. That pairing is a ready-made B1 kill-test. **This
   is the only check that catches the class that would put the AMT on the wrong line.**
6. **Generalise `the_lines_descend_each_page_in_order`** (currently per-form and 2024-pinned at
   `tests/f6251_map.rs:160`) into the same walk. Needs no text layer; would have caught Schedule C's
   f1_39/f1_40 row swap.
7. **Un-pin `field_census.rs:105` (`let year = 2024;`, also `:193`, `:261`)** to walk year directories
   the way its sibling `map_pdf_conformance.rs:71-93` already does. **The B1 kill-test is free: it reds
   on today's tree**, revealing 330 unaccounted TY2025 boxes.
8. **Land a real `xtask forms extract`.** 58 of 64 committed extracts name a command that does not
   exist; the ② step of the authority pipeline — the text layer every conformance instrument reads —
   **has no committed producer**. Per-file `pdftotext` flags are already recorded in each header (32
   `-layout`, 27 no-flags), so this is wiring, not discovery. It blocks pass 2 of the port outright.
9. **Build pass 1b: `label-boxes` diffed against last year's map**, emitting a
   *carried / moved / new / retired* per-line worklist. Biggest single saving in the whole port; no new
   derivation logic needed.
10. **Fix the stem→year derivation** at `form_geometry.rs:186` and `label_reader.rs:456` (they split on
    the last `--`, so `f6251--2026-DRAFT` resolves to a directory `design/forms/2026-DRAFT/` that will
    never exist, and the error blames a missing fetch). The repo's one TY2026 artifact is currently
    undriveable by its own tooling.
11. **Give MANIFEST.json a draft/final discriminator** (`draft: true` or `Kind::FormDraft`). Today a
    draft records `kind: "form"`, identical to a final; only the filename says DRAFT — and the filename
    is exactly what item 10 chokes on.

### Tier 2 — finish TY2025, because TY2026 is a repeat of it

12. **Wire the ten inert TY2025 maps** (`f6251, f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, f8959,
    f8960, f8995, f1040s1a`) — `include_str!` + `include_bytes!` + a `for_year` arm + a `*_pdf` arm
    each. Until then "TY2025 is 15 of 17" is an unexecuted claim: **no compiled consumer loads them.**
13. **Fix `Form6251Map`** — `line1` → `line1a`/`line1b`, so `forms/2025/f6251.map.toml` has a
    deserialization target. The exhaustive `money_cells()` destructure at `map.rs:183` has no `..`, so
    the compiler names every call site. **Add a `Schedule1AMap`** for the 54-field `f1040s1a` map.
14. **Build the Schedule 1-A emitter** — map struct, fill fn, packet push. A grep of
    `crates/btctax-forms/src/*.rs` for `Schedule1a|schedule_1a|f1040s1a` returns **nothing**: TY2025
    currently *computes* Schedule 1-A and **cannot print it**.
15. **Carry `f1040s1`, `f8275`, `f8995a` to TY2025.** Sources exist. Doing them now makes TY2026 a
    one-year hop for all 18 forms rather than a two-year hop for three.
16. **Archive the three missing TY2025 authorities** — all three HTTP 200 today, hashes and
    face-year verified by the authority lens: `irs-prior/f1040s1--2025.pdf` (99,940 B,
    `8dafec71…`), `irs-prior/f8995a--2025.pdf` (117,129 B, `3362db81…`),
    `irs-prior/i8995a--2025.pdf` (248,825 B, `6df1301c…`). (`i1040s1--2025` is 404 and *correctly* so.)
17. **Decide the periodic-alias question for `f8275`/`i8275`.** The bytes are already in the repo under
    2024 and are byte-identical to what `irs-pdf` serves today, so there is nothing to fetch —
    `DUPLICATE_SOURCE_GROUPS = 0` is the blocker. Either teach `duplicates()` the
    same-tree/same-stem/different-year alias with a planted-defect test, or archive once and record
    year-independence in the note. **Doing it now disarms the same trap for the other four periodic
    documents at TY2026.**
18. **Point `xtask cite-check` at `crates/btctax-forms/forms/*/*.map.toml`.** It covers 51 quotations in
    two documents today; the maps carry **272 quoted spans of ≥16 chars that nothing checks**.
19. **Give `design/forms/FIELD_PROVENANCE.md` a generator** (FR-15). Its table claims 496 unaccounted
    across 15 TY2024 forms; measured today it is **0 across 17** (1330 fields, 779 mapped, 551
    censused). ~20 lines of script; the document's headline is currently wrong by its own magnitude.

### Tier 3 — oracles (all achievable today; none waits on OTS 2026)

20. **Upgrade taxcalc 6.7.2 → 6.8.2.** Measured: **zero** effective TY2026 policy differences (only
    `RPTC_c`/`RPTC_rt` removed), and it **fixes PSLmodels#3108** — `calcfunctions.py` now computes the
    standard-deduction AMTI branch as `c00100 - e00700 - qbided - sch1a_amount`, folding Schedule 1-A
    into AMTI as the TY2026 form requires. Expect `verify_f6251.py::_amti_verdict` to red **by design**;
    the response is to delete expected-gap #1 and the `STANDARD_DEDUCTION` excuse table, **not** to
    widen a tolerance.
21. **Do the TY2025 oracle port.** OTS 2025 is installed, final, and correct on §55(d)(3); the golden
    and all 31 AMT vectors are still TY2024, and the corpus carries only Single and MFJ. **A TY2025
    two-oracle baseline is what TY2026 gets diffed against, and it does not exist.** Widen the corpus
    past Single/MFJ while doing it — HoH, MFS and QSS never enter the golden matrix, and QSS is exactly
    where the one live taxcalc disqualification sits.
22. **Fix `ots_direct.py:658-665 version()`**, which returns `"OpenTaxSolver 2024 …"` regardless of
    `OTS_YEAR`. It feeds `gen_goldens.py:511 oracle_1_version` and **SPEC §11 gates regeneration on that
    string** — so the version gate is blind to the exact transition it guards. De-hardcode
    `gen_goldens.py:518 "tax_year": 2024` and the `:506` prose. Land with a planted-defect test.
23. **Add 2026 (and 2027, 2028) to `gen_goldens.py:196 TAXCALC_EXACT_YEARS`** before any TY2026 census
    runs. Without `exact`, taxcalc smooths the Schedule 1-A step and diverges by up to $100/$200 — the
    single most likely source of a spurious "btctax rounding defect."
24. **File `AMT_em_pe` TY2026 upstream** (639,200 → 640,200). Two independent witnesses in hand — the
    IRS draft Form 6251 line 4 prints `$640,200`, and taxcalc's own `AMT_em_ps + AMT_em / AMT_prt` gives
    500,000 + 70,100/0.5 = 640,200 (the identical relation holds exactly for TY2025). **This clears the
    repo's never-file-on-one-oracle bar that #3108 did not.**
25. **Encode `PT_qbid_taxinc_thd` MFS 2026 = 201,775 as a *computed* disqualification** (MFS ≠ the other
    non-joint statuses), not a vector-name excuse. Unfixed in 6.8.2.
26. **Extend `verify_schedule_1a.py` to TY2026** — a `year` parameter plus a second `BTCTAX` literal
    set. The oracles lens transcribed every figure from the published TY2026 draft; the QSS
    disqualification re-derives itself with no mechanism change.

### Tier 4 — constants and provenance

27. **Archive the missing primary sources**: Rev. Proc. 2023-34 / 2024-40 / **2025-32**, the SSA
    Federal Register wage-base determination, and **26 U.S.C. §55 and §164 as amended by Pub. L.
    119-21**. `legal/primary-sources/irs-guidance/` holds exactly one Rev. Proc. (2024-28, the crypto
    safe harbour), yet those three Rev. Procs. are the **sole authority for every indexed figure in
    `ty2024()`, `ty2025()` and `ty2026()`** — nothing in this worktree can re-verify a TY2026 bracket.
    §55(d)(3)'s amended text settles `mfs_kicker_rate`, the one genuinely open AMT cell; §164(b)(6)'s
    settles the TY2026 SALT cap, threshold and escalator.
28. **Read Rev. Proc. 2025-32 end to end**, enumerating every section naming a Code section btctax uses.
    This is also the decisive test for R10 below: if any of §224/§225/§163(h)(4)/§151(d)(5) is
    inflation-adjusted for 2026, the annual inflation Rev. Proc. is where the amount would appear.
29. **Pin `ftc_ceiling`** (§904(j)'s statutory $300/$600) with a cross-year equality assertion in the
    existing bundled-year sweep at `tax_tables.rs:719` — one line. Its neighbour
    `qbi_phase_in_range_*` already has `qbi_phase_in_top()` guarding exactly this; `ftc_ceiling` has
    nothing.
30. **Write and commit the TY2026 fetch script.** Every URL is mechanical (`irs-prior/<stem>--2026.pdf`
    for the 30 annual documents, `irs-pdf/<stem>.pdf` for the 6 periodic), and it can be **tested end
    to end against TY2025**, where every URL is live and every hash is known.

---

## 3. What must wait, and on precisely what

| # | Blocked thing | Blocked on | Earliest realistic |
|---|---|---|---|
| 1 | Every TY2026 AcroForm map except Form 6251; all field names, FQNs, geometry | The TY2026 **final** PDFs. `irs-prior/<stem>--2026.pdf` is **404 on all 36 stems**, probed 2026-09-05 | First final **late Oct – mid Nov 2026**; half the set **mid-Dec 2026 – early Jan 2027**; complete **Jan – Feb 2027** ★ |
| 2 | Anything quoting a TY2026 Schedule 1-A / Schedule 2 / Schedule 3 instruction sentence | **`i1040gi--2026`** — last or nearly last in both observed years (TY2024 final 2025-01-03; TY2025 final **2026-02-25**) | **Jan – Feb 2027** |
| 3 | The §55(d)(3) **MFS kicker rate and cap** — the one genuinely open `AmtParams` cell | **`i6251--2026`**. `irs-dft/i6251--dft.pdf` still serves the TY2025 revision (it contains `$900,350`), confirmed live by two lenses | **~Jan 2027** for confirmation; the *statutory reading* is available today from amended §55(d)(3) |
| 4 | The TY2026 Schedule 1-A's actual **shape** (what it gained between line 37 and line 43) | `f1040s1a--2026`. A **draft exists** (`Created 6/16/26`) and was **revised 2026-09-04 — yesterday**; the final does not | Draft usable now (provisionally); final **Nov 2026 – Jan 2027** |
| 5 | Naming the 1040 lines a TY2026 census cross-references (the draft 6251 cites 1040 **line 7a** where TY2025 cites line 7) | The **TY2026 Form 1040**. `irs-dft/f1040--dft.pdf` still prints 2025, ModDate 2025-09-08. **Every other headline form has a TY2026 draft; the 1040 does not** | Unknown — no draft yet |
| 6 | A **second oracle** for any TY2026 figure | **OpenTaxSolver2026 (v24.00)**. Newest in existence is OTS2025_23.06 (confirmed live). Cadence from two READMEs: v22.00 prelim 2025-01-27, v23.00 prelim 2026-01-27 — same calendar day | Preliminary **~2027-01-27**; settled build **~2027-03**. Hard external dependency; nothing brings it forward |
| 7 | Whether the six periodic forms (`f8275 i8275 f8275r i8275r f8283 i8283`) get a TY2026 revision | The IRS revising them. Decides whether the alias mechanism is needed once or six times | Unknown; cheap to re-probe monthly by hash |
| 8 | Whether TY2026 adds a form btctax does not emit (as Schedule 1-A did for TY2025) | The TY2026 draft set is the early warning | Unknown |
| 9 | Whether Form 8949's box set moves again | The TY2026 form. `SPEC_lot_optimization_program.md:23` records the 1099-DA regime changing again at 2027 | Hypothesis only until the form exists |
| 10 | TY2027 anything | The next inflation Rev. Proc. + SSA determination; taxcalc's `LAST_KNOWN_YEAR` is 2026 and extrapolates past it | Rev. Proc. **~Oct 2026** (weeks out); already parked as externally blocked, `FOLLOWUPS.md:159` |

★ **The date basis is honest and weak.** The IRS publishes **no** finalisation schedule —
`irs.gov/draft-tax-forms` shows draft posting dates and says only "do not file draft forms." Row 1 is the
archive's own measured cadence over **two years** of PDF `CreationDate` metadata, and TY2025 ran ~30 days
later than TY2024 at every percentile. Treat it as a range, not a date.

**★ What the IRS HAS published for TY2026, measured live 2026-09-05:** **20 of the 30 annual documents
exist as TY2026 drafts** — 16 of 17 forms (`f1040s1 f1040s1a f1040s2 f1040s3 f1040sa f1040sb f1040sc
f1040sd f1040sse f6251 f8615 f8949 f8959 f8960 f8995 f8995a`) and 4 of 13 instructions (`i1040sb
i1040sse i8615 i8959`). This is *far* more TY2026 material than any single lens assumed (see D3). It does
not change row 1 — a draft is not authority — but it does mean the port machine's passes 1 and 2 can be
**exercised** against real TY2026 files months early.

---

## 4. ★ WRONG-NUMBER RISKS — 27 places TY2026 would emit a wrong figure instead of refusing

**This section outranks everything else in the document.**

Four states, because "fails closed" is not binary here:

- **REFUSES** — a gate catches it today and will catch it in 2026.
- **DORMANT** — the wrong code is *in the tree*; a higher gate hides it; **the port is the commit that opens that gate.**
- **UNGUARDED** — no wrong code yet, but **no instrument would catch the wrong number when the port introduces it.**
- **LIVE** — wrong right now.

### 4a. Engine — would print a wrong figure on a signed return

| # | Risk | State | Sev |
|---|---|---|---|
| **R1** | `return_1040.rs:2493` hardcodes `line1_rule: Form6251Line1Rule::Y2024`. The comment above it claims "the year lives at THIS call site" — **false**: `form6251_inputs_from_parts` (`:2476`) has no `year` parameter; its only caller has `year: i32` at `:1826`. Line 4 combines `line1` under Y2024 and `line1b` under Y2025 — **different quantities**. Wrong AMTI → wrong TMT → wrong 1040 L17/L24, silently. ★ The same function gets this right 400 lines earlier: `Schedule1A::compute` at `:2064-2069` is passed `year` with a doc comment about exactly this failure mode. No test in any `crates/*/tests/` mentions `line1_rule`. | **DORMANT** | **CRITICAL** |
| **R2** | `schedule_1a_params` (`tables.rs:1088`) returns `Some` for 2026 and `Schedule1A` (`schedule_1a.rs:397-404`) is **one struct with the TY2025 line numbers baked in** — `line37` as a field name at `:382`, emitted as the literal label `"37"` at `:545`. The TY2026 draft 6251 line 46 reads *"Subtract Schedule 1-A (Form 1040), line **43**"* where TY2025 reads line 37, and the TY2026 draft Schedule 1-A independently shows Part IV at 28-36 and Part V at 37-43. **`Form6251Line1Rule::Y2025 { schedule_1a_l37 }` is not reusable for 2026**; a port writing `2025 \| 2026 => Y2025 {..}` subtracts the wrong line. | **DORMANT** | **CRITICAL** |
| **R3** | **No committed test joins a mapped line to the printed label beside that box.** `map_pdf_conformance.rs:76` checks only that an FQN *exists*. Measured: applying the TY2024 f6251 map to the TY2025 PDF gives **0 of 61 FQNs absent (every check passes)** while **12 of 41 mapped lines land on the wrong label — TY2024 `line11`, the AMT itself, prints in the TY2025 form's line-10 box.** One added field on page 1 walked everything below it down with **zero renames**. The softer per-binding relaxation is no better: it accepts 125 of 155 bindings, **16 of them wrong** (f1040s2 line11/12, f1040s3 line8/10/11, eleven f6251 page-1 bindings). | **UNGUARDED** | **CRITICAL** |
| **R4** | **23 of 266 in-place renames are SILENT** — the TY2024 spelling still exists in the TY2025 PDF but now names a box **12 to 252 points away** (f1040 11, f1040s3 4, f8949 4, schedule_d 3, f1040sc 1). The exists-check reds on the other 243 and is blind to these. | **UNGUARDED** | **IMPORTANT** |
| **R5** | **Form 8949's crypto box moved from C to I.** TY2025 went 3 boxes → 6 per part (`c1_1[0..2]` on 1,2,3 → `c1_1[0..5]` on 1..6), and TY2025's Box C now reads *"other than digital asset transactions."* Keeping `box_field = c1_1[2] / box_on = "3"` **ticks a box that explicitly excludes crypto, on a return signed under 26 USC 6065.** Not derivable from any field-name diff. | **UNGUARDED** | **IMPORTANT** |
| **R6** | **Schedule C 27a/27b: the printed letters swapped while the field names stayed pinned to their content** (`f1_39` is "Other expenses" in both years; the 2024 map calls it 27a, the 2025 map calls it 27b). **Keying a port off the printed form — the safer-*seeming* choice — is exactly what breaks here.** Both cells are `rule = "unmodeled"` today, so no money moved; the mechanism is the risk. | **UNGUARDED** | **IMPORTANT** |
| **R7** | f8949 `rows_per_page` 14 → 11 and `table_token` `Table_Line1` → `Table_Line1_Part`. Pagination and overflow behaviour are **year-dependent and carried in the map**; a stale value silently changes how many rows fit before a continuation page. | **UNGUARDED** | **IMPORTANT** |
| **R8** | `f1040_clusters` (`form1040.rs:34-39`) and `se_clusters` (`schedule_se.rs:38-43`) are **the only two year functions in `btctax-forms` ending in a wildcard `_ =>`** and return the 2024/2025 x-bands for any year ≠ 2017. They are the map-independent geometry oracle `verify_flat` checks against (`form1040.rs:148`). A 2026 arm added to `Form1040Map::for_year` without touching these applies **2024 geometry to a 2026 PDF** — the instrument built to catch a mis-mapped cell is the one that was never told the year changed. | **DORMANT** | **IMPORTANT** |
| **R9** | The AMT screening worksheet hardcodes the rate TY2026 changes: `dec!(0.25)` at `amt.rs:129` and `dec!(0.26)` at `:141`, while `AmtParams::exemption_phaseout_rate` and `rate_26` exist and *are* read correctly throughout `form6251.rs`. At TY2026's **50%** the literal understates line 10 → line 11 → the screen answers "no AMT" for filers who owe it. **Mitigating, verified: `amt_should_file_6251` has no production caller** (Form 6251 became unconditional under §G-6) — a latent trap, not a live defect. | **DORMANT** | **IMPORTANT** |
| **R10** | `schedule_1a_params(2026)` already returns TY2025's **dollar amounts**, and its guard `schedule_1a_exists_only_for_2025_through_2028` (`tables.rs:1139`) **enforces** field-identity across 2025..=2028 rather than checking it — so if any OBBBA deduction is indexed, the test reds **on the correction, not on the defect.** The claim rests on a doc comment (`tables.rs:1086-1087`); none of §224/§225/§163(h)(4)/§151(d)(5) is archived. **See D1 — the amounts are corroborated but the structure is not.** | **DORMANT** | **IMPORTANT** |
| **R11** | `ftc_ceiling` (`tables.rs:474`, §904(j)'s statutory $300/$600) lives inside the per-year *indexed* struct with **no invariant protecting it** — exactly where a mechanical "index everything for the new year" pass goes wrong. Its neighbour `qbi_phase_in_range_*` is protected by `qbi_phase_in_top()`. | **DORMANT** | MINOR |
| **R12** | `dependent_std_earned_addon` was **$450 in BOTH TY2024 and TY2025** — a §63(c)(5)(B) *indexed* figure that happened not to move. A copy-forward pass reads "unchanged last year" as "not indexed" and carries it silently. **A figure being unchanged is not evidence that it is unindexed.** | **DORMANT** | MINOR |
| **R13** | `return_1040.rs:2073-2074` passes `false, false` for taxpayer/spouse senior qualification into `Schedule1A::compute`, commented "not inferred here." Direction is safe (a forfeited senior deduction *overstates* tax), but it is a **hardcoded answer to a question the filer was never asked** — "blank because nothing populated it," not "blank because the inputs say so." | **LIVE (TY2025)** | MINOR |
| **R14** | `conventions.rs:19 TY2025_RETURN_DUE = 2026-04-15` is a **singular constant, not a per-year function**, consumed at `resolve.rs:1668`. Its shape does not admit a TY2026 due date. | **DORMANT** | MINOR |

### 4b. Instrument blind — a wrong number would not be caught

| # | Risk | State | Sev |
|---|---|---|---|
| **R15** | `field_census.rs:105 let year = 2024;` (also `:193`, `:261`) pins the **omission-direction** gate — the one that costs a filer money — to TY2024. Measured: TY2024 has 0 unaccounted across 17 forms; **TY2025 has 330 unaccounted across 15** (f1040 196, f8283 63, schedule_d 40, f8949 16, schedule_se 15), plus 244 census dispositions reachable by no test. Its sibling `map_pdf_conformance.rs:71-93` already *derives* its set by walking the filesystem: **the two halves of one invariant sit in one directory, one year-general and one year-pinned.** | **LIVE** | **IMPORTANT** |
| **R16** | `EMITTED_FORMS` (`cite_check.rs:678-681`) lists 16 stems and **omits `f8995a`**, which btctax does emit (`form8995a.rs`, `packet.rs:192`). The ratchet `authority_coverage_may_only_improve` is keyed on that list, so **it passes by finding nothing for a form whose transcription no instrument checks.** `f8615` may be the same shape. | **LIVE** | **IMPORTANT** |
| **R17** | §5.2 pass 3 carries a census reason when field name and line text are both unchanged — but **a reason is a claim about btctax's ENGINE**, so a zero-delta form can still invalidate it. Three measured in-repo instances: `forms/2024/f8960.map.toml:27` went stale via FR-12 on a form with a **zero** field diff; `map.rs:106-108` says f6251 lines 2c-2t are censused `gap` when both years rule all 18 `unmodeled`; `field_census.rs:177-181` records Form 1040 line 7's reason as simply false. **A pass keyed on form stability cannot tell "still true" from "nobody re-read it."** | **UNGUARDED** | **IMPORTANT** |
| **R18** | `design/forms/FIELD_PROVENANCE.md` claims **496 unaccounted across 15 TY2024 forms**; measured today it is **0 across 17** (1330 fields, 779 mapped, 551 censused). The port machine would be checked against a baseline stale by its own entire magnitude. | **LIVE** | MINOR |
| **R19** | The two moving-URL notes — `design/forms/2025/f8275r--2025.pdf` and `i8275r--2025.pdf` point at `irs-pdf/f8275r.pdf` and `irs-pdf/i8275r.pdf`. Both hashes are current today, but **`irs-pdf` silently flips to the TY2026 final** once each is finalised. `verify()` **never touches the network** — for `Storage::Note` it only asserts the note file exists (`authority_manifest.rs:233-268`) — so an operator who re-fetches after the flip and skips the note's `Verify:` line gets **a different tax year's document with no alarm from the repo.** | **LIVE ON FLIP (Nov 2026 – Jan 2027)** | **IMPORTANT** |
| **R20** | A TY2026 draft archived under a clean stem is **indistinguishable from a final** in MANIFEST.json (`kind: "form"`, no draft field). A conformance test keyed on stem would read **draft line numbering as authority**, and draft line numbers change before final — the TY2026 Schedule 1-A draft was revised **2026-09-04**. | **UNGUARDED** | **IMPORTANT** |
| **R21** | `ots_direct.py:658-665 version()` returns `"OpenTaxSolver 2024 …"` regardless of `OTS_YEAR` — pasted output driving the 2025 tree reads `OpenTaxSolver 2024 (OpenTaxSolver2025_23.06_linux64)`. It feeds `gen_goldens.py:511 oracle_1_version`, and **SPEC §11 gates golden regeneration on that string**: a genuine engine-year change looks like no change at all. | **LIVE** | **IMPORTANT** |

### 4c. Oracle — a wrong witness, which is how a wrong number gets blessed

| # | Risk | State | Sev |
|---|---|---|---|
| **R22** | taxcalc `AMT_em_pe` TY2026 = **639,200** where the IRS draft Form 6251 line 4 prints **$640,200** (and taxcalc's own `AMT_em_ps + AMT_em / AMT_prt` = 500,000 + 70,100/0.5 = 640,200). Used alone, taxcalc **OVERSTATES** AMT for an MFS filer with AMTI in [639,200, 640,200) — `calcfunctions.py:2590` zeroes an exemption worth up to $500 there. **Silent: it produces a plausible number, not a refusal.** Unfixed in 6.8.2. | **LIVE** | **IMPORTANT** |
| **R23** | On the **installed** taxcalc 6.7.2, AMTI is **understated by the whole standard deduction** for every standard-deduction filer. Measured on TY2026: AMT 9,444 vs the fixed engine's 13,630 (single), 17,288 vs 25,660 (MFJ), 4,175 vs 9,180 (MFS). **Agreeing with 6.7.2 on TY2026 AMT means agreeing with a known-wrong AMTI** — and TY2026 is a materially bigger AMT year for exactly btctax's large-LTCG population (`AMT_prt` 0.25 → 0.5; MFJ +38%; MFS 0 → owing). | **LIVE** | **CRITICAL as a witness** |
| **R24** | `AutoLoanInterestDed_ps` gives a QSS the **$200,000 MFJ threshold** where the TY2026 draft Schedule 1-A line 32 says "Enter $100,000 ($200,000 if married filing jointly)." Direction is an **understatement of tax**. The parameter has a single non-indexed year-2013 row, identical 2025-2028. **Partially mitigated: `verify_schedule_1a.py` carries a *computed* disqualification that re-derives itself for TY2026 with no mechanism change.** | **LIVE (caught)** | **IMPORTANT** |
| **R25** | A naive year bump that does not extend `TAXCALC_EXACT_YEARS` makes taxcalc **smooth the Schedule 1-A phase-out step** (`calcfunctions.py:1074/1094/1115`), producing a divergence of up to **$100** (Parts II/III) or **$200** (Part IV) at every non-$1,000-multiple MAGI — **which will read as a btctax rounding defect.** The oracles lens's own TY2026 probe ran with `exact` off, which is exactly what a year bump would do. | **LIVE** | **IMPORTANT** |
| **R26** | `verify_schedule_1a.py::_rows()` returns the latest row **at or before** the requested year, so a census asked for the **wrong year silently compares an earlier year's parameters and prints OK** rather than complaining. Correct semantics for a step function; wrong as a year gate. | **LIVE** | **IMPORTANT** |
| **R27** | `PT_qbid_taxinc_thd` TY2026 MFS = **201,775** where single/HoH/QSS = 201,750 and MFJ = 2 × 201,750 — and 201,775 is exactly `II_brk4`'s single/MFS value. Uniform across non-joint statuses in 2025, so the anomaly is **new in 2026**. Shifts the QBI limitation boundary by $25 at the exact point btctax switches to `RefuseReason::QbiAboveThreshold`. Unfixed in 6.8.2. | **LIVE (out of reach today)** | MINOR |

### What DOES fail closed — the good news, so budget is not spent re-checking it

All **17** `for_year` constructors in `map.rs` and all **17** `*_pdf(year)` fns in `pdf.rs` end in
`UnsupportedYear`; `SUPPORTED_YEARS = &[2017, 2024, 2025]`. Machine-checked: `btctax export-irs-pdf
--tax-year 2026` exits 2 with "unsupported tax year 2026", and `btctax report --tax-year 2027` refuses
with `TaxTableMissing`. `full_return_for` returns `None` for everything but 2024, reinforced by
`input_form_store.rs:307`'s `table.year != year || params.year != year` consistency guard and by two
named standing kill-tests. `Form6251Map`'s `line1` vs `line1a`/`line1b` mismatch **refuses** today
(`packet.rs:181` dispatches by year and there is no `F6251_MAP_2025`). `verify_f6251.py`'s
`STANDARD_DEDUCTION` raises a **named** KeyError for 2026. `era.rs:112`'s hardcoded 2025 date is coupled
by a live test that **reds the moment `SUPPORTED_YEARS` gains 2026** — the model of a good hardcode.
And archiving `f8275--2025` would turn the suite **RED** rather than wrong, which is the safe failure
(see §6).

---

## 5. The critical path

Ordered. **M** = mechanical (a tool does it or a compiler names every site). **H** = a human reads a
form or makes a product decision.

| # | Step | M/H | Gate |
|---|---|---|---|
| 1 | Close R1, R2, R8 — the three year-seam hardcodes that the *next* commit arms | **M** | before step 2 |
| 2 | Build the line↔label join test + pass-1b worklist generator (R3), un-pin `field_census.rs` (R15) | **M** (the B1 kill-tests are already observed red) | before any map port |
| 3 | Land `xtask forms extract`; fix the stem→year split; add the draft/final discriminator | **M** | before pass 2 |
| 4 | **TY2025 `FullReturnParams` + `AmtParams` → `full_return_for(2025) = Some`** | **M** for the 30-odd indexed figures (Rev. Proc. 2024-40, `i1040gi--2025`, both published); **H** for the §164(b) `Worksheet2025` instrument | **the gate** — nothing downstream exists without it |
| 5 | Wire the ten inert TY2025 maps; fix `Form6251Map`; **build the Schedule 1-A emitter** | **M** for the 5×N wiring edits; **H** for the `Schedule1AMap` shape | TY2025 can print |
| 6 | Census the 330 slice-map boxes exposed by step 2 | **H** — 330 reasons, and no tooling reaches "does this line encode a decision" | TY2025 exact cover |
| 7 | Upgrade taxcalc → 6.8.2; fix `ots_direct.version()`; **build the TY2025 two-oracle baseline** on OTS 2025 (installed, final, correct on §55(d)(3)); widen the corpus past Single/MFJ | **M** for the upgrade and the fixes; **H** for the golden regeneration review under SPEC §11 | TY2025 validated |
| 8 | Archive Rev. Proc. 2025-32 + §55/§164 as amended; read them; settle `mfs_kicker_rate` and R10 | **H** — statute reading | TY2026 constants knowable |
| 9 | *Optional now:* port Form 6251 to TY2026 **speculatively against the draft** — pass 1 is zero-delta (62/62 fields, identical names, one rect differing by 0.6pt) and pass 2's five findings are already enumerated | **M** pass 1; **H** pass 2 | flushes the shape questions early |
| 10 | **~Nov 2026 – Jan 2027:** fetch TY2026 finals as they land; run passes 1/1b/2 per form | **M** for the inventory and the join; **H** for every rename's re-mapping and every new line's census reason | — |
| 11 | **~Jan – Feb 2027:** `i1040gi--2026` and `i6251--2026` close the last constants; **OTS 2026 (~2027-01-27)** gives the second witness | **H** | TY2026 validated |

**The honest mechanical/human split, measured rather than estimated:**

- **Fully mechanical:** the field inventory (`dump-fields`), the line↔box join (**177/177 reproduced,
  0 disagreements**), the exact-cover arithmetic, and the verbatim-quote check once `cite-check` is
  pointed at the maps. Roughly **60% of the field surface is a free copy** — 642 of 1055 spellings
  survived TY2024→TY2025.
- **Mechanically checkable, human transcription:** the ~15% of fields that get renamed. The exists-check
  says *that* it changed (243 of 266) and the label reader says *where the new box is*; re-pointing 18
  fields on Form 8995 is transcription, not judgment, but nobody can skip it.
- **Irreducibly human, and where the whole cost sits:** TY2025 cost **242 census reasons / 31,587 bytes**
  of judgment prose — and it grew *most on the forms where nothing changed*: `f8959` had 26 identical
  fields, 0 renames, 0 new, 0 gone, and went 35 → 101 comment lines. **The per-form floor is ~40-65
  lines of new hand-written justification even for a zero-diff form.** Add the ~330 TY2025 census
  reasons still owed (step 6), and the Schedule 1-A / Form 1040 restructures, which are `btctax-core`
  struct changes and statute reading, not map ports at all.

**Pass 1 is a safe filter and a poor accelerator, measured:** it permits a verbatim copy on **2 of 9**
forms. That coarseness is load-bearing — the intuitive relaxation is R3.

---

## 6. What should NOT be built

1. **Do NOT encode any TY2026 figure from a draft form.** Not the §55(d) resets, not `$640,200`, not the
   0.50 phase-out rate, not the Schedule 1-A amounts. `design/ty2025/SPEC.md:67` classifies the draft
   *"evidence only, never transcribe,"* drafts are replaced in place, and the TY2026 Schedule 1-A draft
   was revised **yesterday**. The *encodable* sources are Rev. Proc. 2025-32 §55(d), the amended
   statute, and the final form. **Knowable ≠ encodable.**
2. **Do NOT build a port machine that emits only `.map.toml`.** That reproduces exactly today's state —
   ten committed, correct, and **loaded by nothing**. The five wiring edits per `(stem, year)` are a
   pure function of the pair; generate them.
3. **Do NOT build a per-binding name-existence carry-forward.** It is the intuitive relaxation of pass 1
   and it ships **16 wrong bindings**, one of them the AMT into line 10's box (R3).
4. **Do NOT make a geometry-only matcher the sole witness.** Schedule C's 27a/27b swap breaks precisely
   the port keyed off the printed form; the label reader **plus** the map's own reasoning is what
   settles it.
5. **Do NOT edit `DUPLICATE_SOURCE_GROUPS` to get past an `f8275` red.** The red is the *safe* failure
   and the code comment names this exact moment. Build the alias with a planted-defect test, or archive
   once under 2024 and record year-independence in the note.
6. **Do NOT widen a tolerance when `_amti_verdict` reds after the 6.8.2 upgrade.** Its docstring says
   "the day taxcalc fixes #3108 this reds and tells us." Delete expected-gap #1 and the
   `STANDARD_DEDUCTION` excuse table instead.
7. **Do NOT regenerate a golden to make a red green.** If the fix for a red golden is "regenerate," then
   regenerating to match a breakage is always green.
8. **Do NOT widen OTS's year-scoped excuse sets speculatively.** `OTS_YEARS_WITH_STALE_MFS_KICKER` and
   `OTS_YEARS_WITHOUT_CASH_CEILING` are correctly `{2024}`; TY2026 needs its own read of
   `taxsolve_US_1040_2026.c` **when it exists**. And no excuse list keyed by vector *name* — compute the
   disqualification from the defect's mechanism.
9. **Do NOT re-wire the AMT screening worksheet.** `amt_should_file_6251` has no production caller since
   Form 6251 became unconditional. Fix its constants (R9) or delete it; do not re-enable a
   screen that can answer "no AMT" for a filer who owes it.
10. **Do NOT start TY2026 `FullReturnParams` before TY2025's.** No gate skips TY2025, the field names
    and citation shapes are shared, and `design/OWNER_DECISIONS_2026-09-04.md:43` already records that
    no TY2025 work is wasted on TY2026.
11. **Do NOT archive TY2026 drafts under clean stems** until the manifest can tell a draft from a final
    (R20) and the stem→year split is fixed. Archiving them under `--DRAFT` today is inert by
    construction.
12. **Do NOT chase TY2027.** taxcalc extrapolates past `LAST_KNOWN_YEAR = 2026` from CPI growth factors,
    not law; the item is already parked as externally blocked.
13. **Do NOT file the two remaining taxcalc anomalies as single-oracle reports.** R22 has two
    independent witnesses and clears the bar; R24 and R27 do not yet — encode them as computed
    disqualifications instead.

---

## 7. Where the lenses disagreed — and which I believe

**D1 — Schedule 1-A's TY2026 dollar amounts. I believe the ORACLES lens; the CONSTANTS lens's
structural point survives intact.**
The constants lens ranks `schedule_1a_params(2026)` returning 2025's amounts as its **highest**
wrong-number risk, direction unknown, on the ground that the "nothing is indexed" claim rests on a doc
comment with **no archived primary source**. The oracles lens independently read the published TY2026
**draft Schedule 1-A** text layer and enumerated **every figure as unchanged** — $25,000; $12,500/$25,000
MFJ; $10,000; $6,000; thresholds $150,000/$300,000, $100,000/$200,000, $75,000/$150,000; steps
$100/$100/$200; 6%. That is a primary-source measurement the constants lens did not have, and it
**demotes the amounts risk from #1**. Two qualifications hold: the source is a draft **revised
2026-09-04**, and the constants lens's *structural* finding — that the guard test **enforces** sameness
rather than checking it, so it reds on the correction rather than on the defect — is untouched and stays
open as R10. Item 28 in §2 (read Rev. Proc. 2025-32 for a §224/§225/§163(h)(4)/§151(d)(5) hit) settles
it properly. **Meanwhile the Schedule 1-A *line-number* risk (R2) is confirmed by both lenses and rises
to the top.**

**D2 — Is `i6251--2026` published? SETTLED: no.**
The seams lens flagged this as "the highest-leverage unknown — one fetch settles it," working from a
five-week-old in-repo note. The authority and oracles lenses both fetched it: `irs-dft/i6251--dft.pdf`
still serves the **TY2025** revision (it contains `$900,350`). The unknown is closed and the answer is
"still blocked" — the MFS kicker rate and cap remain the one genuinely open `AmtParams` cell.

**D3 — Do TY2026 drafts exist? SETTLED: yes, and far more than any single lens assumed.**
The maps lens explicitly listed this as "UNKNOWN, not probed — one HTTP HEAD per stem would settle it."
The authority lens probed all 36 stems: **16 of 17 forms and 4 of 13 instructions carry genuine TY2026
drafts.** Neither lens saw the other. This **materially softens the maps lens's must-wait**: passes 1
and 2 can be exercised against real TY2026 files months early, provisionally — subject to R20 and to
rule 1 of §6.

**D4 — Is the 0.50 exemption phase-out rate encodable today? I believe CONSTANTS/ORACLES on the
arithmetic and SEAMS on the policy.**
The seams lens invokes `CLAUDE.md`'s ban on encoding an inferred constant. The constants and oracles
lenses argue it is a *transcription*, not an inference: 500,000 + 70,100/0.50 = 640,200 is the only rate
reproducing the form's printed line-4 threshold (25% gives 780,400), and the repo **already carries the
equivalence proof executably** (`mfs_kicker_constants_satisfy_the_two_section_55d3_identities`). The
derivation is sound and I accept it — **but the source is still a draft**, so the correct verdict is
*knowable now, encodable from Rev. Proc. 2025-32 §55(d) or the final form.* That distinction is the
whole of §6 rule 1.

**D5 — The label-boxes join count: 130 vs 177. NOT a contradiction.**
The maps lens counts 130 mapped money lines across 9 forms; the machine lens counts 177 `lineN` bindings
across 10 ported forms (including f1040s1a's 54-field map). **Both report zero disagreements on the real
batch and both watched the instrument go red on a planted defect.** Different denominators, same
conclusion. Noted so nobody chases a phantom discrepancy.

**D6 — "TY2025 is 15 of 17 done." The MACHINE lens's decomposition is decisive and the phrase is
misleading.** Ten are full ported maps with an exact cover; the other five are the **original
crypto-slice maps** carrying **330 boxes with no recorded decision**; and **ten of the fifteen are
inert** — no `include_str!` reaches them. The accurate statement is *"10 of 17 ported, 5 of 17 wired."*

---

## 8. Reading the lens files

| lens | file | own it for |
|---|---|---|
| code year-seams | `design/agent-reports/2026-09-05-ty2026-port-seams.md` | R1, R2, R8, R9, R13, R14; the full year-keyed/year-hardcoded inventory |
| constants & indexing | `…-ty2026-port-constants.md` | R10-R12; the statutory/indexed/expiring three-way split; what Rev. Proc. 2025-32 supplies |
| form authority pipeline | `…-ty2026-port-authority.md` | R19, R20; the 36-stem probe, the measured IRS cadence, the periodic-alias decision |
| AcroForm map porting | `…-ty2026-port-maps.md` | R3-R7; the 266-rename classification, the f6251 blind-copy measurement, per-form tiering |
| oracle readiness | `…-ty2026-port-oracles.md` | R21-R27; taxcalc 6.8.2, the OTS cadence, the three TY2026 parameter defects |
| the year-port machine | `…-ty2026-port-machine.md` | R15-R18; the 177/177 join, the pass-1/1b/2/3 shape, the wiring the plan omits |
