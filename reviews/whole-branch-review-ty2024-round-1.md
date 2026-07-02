# Whole-diff review — TY2024 tables backfill (round 1)

**Diff:** `42ddab8..e437a74` (2 commits: `81bcd4f` spec+R0, `e437a74` implementation)
**Artifacts read:** `design/SPEC_ty2024_tables.md` (R0-GREEN payload), `reviews/R0-spec-ty2024-round-1.md`
(rounds 1+2), `.superpowers/sdd/ty2024-report.md`, review package
`.superpowers/sdd/review-42ddab8..e437a74.diff`, and current source at HEAD `e437a74` (working tree clean —
`git status --porcelain` empty; file contents == commit contents).
**Reviewer stance:** independent. The transcription was compared spec → code digit-by-digit, and the
spec's payload was re-verified by this reviewer against the primary sources fetched TODAY
(2026-07-01): IRB 2023-48 HTML (`irs.gov/irb/2023-48_IRB`, the publication of record for Rev. Proc.
2023-34) and the SSA Federal Register determination (FR Doc. 2023-23317,
`govinfo.gov/content/pkg/FR-2023-10-23/pdf/2023-23317.pdf`). Not trusted from the spec or R0.
**Gate:** taken as given per assignment (692 passed / 0 failed; clippy `-D warnings` clean; fmt clean;
PII clean) — not re-run.

**Verdict: READY TO MERGE — 0 Critical / 0 Important / 1 Minor / 1 Nit.**

---

## 1. Transcription — spec → code, digit-by-digit (`tax_tables.rs:96–215`)

All 28 ordinary bracket edges, all 8 LTCG breakpoints, and all 3 ancillary values in `ty2024()`
match the spec's verified payload exactly, AND match this reviewer's independent IRB 2023-48 fetch:

| Rate | MFJ (l.121–127) | HoH (l.138–144) | Single (l.104–110) | MFS (l.156–162) |
|------|-----------------|-----------------|--------------------|-----------------|
| 10%  | 0 ✓ | 0 ✓ | 0 ✓ | 0 ✓ |
| 12%  | 23,200 ✓ | 16,550 ✓ | 11,600 ✓ | 11,600 ✓ |
| 22%  | 94,300 ✓ | 63,100 ✓ | 47,150 ✓ | 47,150 ✓ |
| 24%  | 201,050 ✓ | 100,500 ✓ | 100,525 ✓ | 100,525 ✓ |
| 32%  | 383,900 ✓ | **191,950** ✓ | **191,950** ✓ | **191,950** ✓ |
| 35%  | 487,450 ✓ | **243,700** ✓ | 243,725 ✓ | 243,725 ✓ |
| 37%  | 731,200 ✓ | 609,350 ✓ | 609,350 ✓ | **365,600** ✓ |

- **Rate order/structure:** 0.10/0.12/0.22/0.24/0.32/0.35/0.37 ascending in all four statuses; seven
  `br(lower, rate)` pairs each; shape is field-by-field identical to `ty2025()` (the `TaxTable` /
  `OrdinarySchedule` shape carries `(lower, rate)` only — no cumulative-base field exists, verified
  against `crates/btctax-core/src/tax/tables.rs:53–82`; the engine derives cumulative tax itself).
- **Lower-bound semantics** match the Rev. Proc.'s "over $X" column and `ty2025()`'s convention
  (e.g. Single 22% row "Over $47,150 … $5,426 plus 22% of the excess over $47,150" → lower 47,150).
- **LTCG (§3.03), l.169–198:** Single 47,025/518,900 ✓; MFJ 94,050/583,750 ✓; HoH 63,000/551,350 ✓;
  MFS 47,025/**291,850** ✓.
- **Ancillary:** `gift_annual_exclusion` 18,000 (l.207) ✓; `ss_wage_base` 168,600 (l.210) ✓;
  `gift_lifetime_exclusion` 13,610,000 (l.213, `dec!(13_610_000)`) ✓; `year: 2024` ✓.
- **`source` field (l.202–203):** `"Rev. Proc. 2023-34 §3.01/§3.03 + §3.43 + §3.41 (TY2024);
  SSA 2023-10-12 (ss_wage_base $168,600)"` — §3.xx cites as mandated; the `\` string continuation
  strips the newline + leading whitespace (same convention as `ty2025()`'s source).
- **QSS not inserted** in either `ordinary` or `ltcg` (Task 2 item 4) ✓ — `TaxTable::key` maps
  `Qss → Mfj` (`tables.rs:87–93`).
- **All inline cites use §3.xx** — the four `// §3.01 — <Status>` headers, the §3.03 LTCG header,
  and the three ancillary-field comments. No `§2.xx` appears anywhere in the TY2024 region.

### The four flagged traps — independently re-confirmed against IRB 2023-48 (fetched 2026-07-01)

1. **HoH 35% @ $243,700** — IRB Table 2: "Over $243,700 but not over $609,350 — $53,977 plus 35% of
   the excess over $243,700" (vs Single/MFS $243,725). Code l.143: `br(dec!(243700), dec!(0.35))` ✓.
2. **MFS 37% @ $365,600** — IRB Table 4: "Over $365,600 — $98,334.75 plus 37% of the excess over
   $365,600". Code l.162: `br(dec!(365600), dec!(0.37))` ✓.
3. **MFS LTCG max_fifteen $291,850** — IRB §3.03: MFS Maximum 15-percent Rate Amount = $291,850
   (NOT 583,750/2 = 291,875). Code l.196: `max_fifteen: dec!(291850)` ✓.
4. **32% rows @ $191,950** (Tables 2–4) — IRB reads $191,950 in both columns; the printed base
   amounts arithmetically pin it: HoH 37,417 = 15,469 + 24%×(191,950−100,500) ✓; Single/MFS
   39,110.50 = 17,168.50 + 24%×(191,950−100,525) ✓. Code l.108/l.142/l.160: `dec!(191950)` ×3 ✓.
   (The PDF's "$191,150" rendering typo is documented in the `ty2024()` doc comment, l.88–90,
   per [R0-N2] — future verifiers won't stall.)

Cross-checks of the remaining printed base amounts (2,320/10,852/34,337/78,221/111,357/196,669.50;
1,655/7,241/15,469/53,977/181,954.50; 1,160/5,426/17,168.50/55,678.50/183,647.25/98,334.75) are all
arithmetically consistent with the transcribed bounds. **No digit mismatch found anywhere.**

### Ancillary values — independently re-confirmed

- §3.43: "the first **$18,000** of gifts to any person … calendar year 2024" ✓ (IRB).
- §3.41: "basic exclusion amount is **$13,610,000**" for decedents dying in 2024 ✓ (IRB).
- SSA: "The 2024 OASDI contribution and benefit base is **$168,600**, compared to $160,200 for 2023"
  ✓ (FR Doc. 2023-23317 — the §230 legal determination behind the 2023-10-12 press release).
- Section numbering: SECTION 3 = "2024 ADJUSTED ITEMS"; SECTION 2 = Superfund-rate changes ✓.

## 2. KATs (`tax_tables.rs:457–764`)

Every assert matches the R0-GREEN spec exactly; every derivation re-derived by this reviewer:

- **A1** (l.583–593): Single brackets[1]=11,600, [2]=47,150, [6]=609,350, [6].rate=0.37 ✓ (spec verbatim).
- **A2** (l.598–617): MFS last=365,600; MFJ last=731,200 ✓.
- **A3** (l.622–662): all five statuses incl. QSS≡MFJ (94,050/583,750) and MFS 291,850 ✓ (spec verbatim).
- **A4** (l.667–673): 18,000 / 168,600 / 13,610,000 ✓. **A5** (l.678–680): `table_for(2024).is_some()` ✓.
- **A6a** (l.691–695): ST 1,000 fully in 22% band → **220.00**; MAGI_with 48,150 < 200,000 → `niit == 0` ✓.
- **A6b** (l.706–710): 1,050×22% + 950×24% = 231 + 228 = **459.00**; 202,000 < 250,000 → `niit == 0` ✓.
- **A6c** (l.721–725): 100×12% + 400×22% = 12 + 88 = **100.00**; 63,500 < 200,000 → `niit == 0` ✓.
- **A6d** (l.738–742): 600×35% + 400×37% = 210 + 148 = 358.00; NIIT = 3.8% × min(1,000, 241,000)
  = **38.00** (MAGI_with 366,000 > MFS 125,000); total **396.00**. Asserts `niit == dec!(38.00)` AND
  `total == dec!(396.00)` — the C1-corrected values, with the correct §1411(c)(1)(A)(iii)/[R0-C1]
  comment. The pre-fold wrong value ($358.00 total / $0 NIIT) did NOT leak into the code ✓.
- **A7** (l.755–763): at_0 = 47,025−40,000 = 7,025; at_15 = 2,975 → ltcg_tax **446.25**; asserts
  `ltcg_tax`, `total`, and `niit == 0` ✓.
- **[R0-I1] fixture convention held:** all five fixtures pass `magi_excluding_crypto = OTI` —
  `p24_single(47150, 47150)`, `p24_mfj(200000, 200000)`, `p24_hoh(63000, 63000)`,
  `p24_mfs(365000, 365000)`, `p24_single(40000, 40000)` ✓ (helper doc comment states the convention,
  l.515–516).
- **No golden weakened:** every pre-existing test appears in the diff as pure context; the only
  changes inside `mod tests` are additive (new imports l.347–352, TY2024 helpers, 10 new KATs).

## 3. Registration + docs

- `load()` (l.62–69) inserts `2024 → ty2024()` before `2025 → ty2025()`; the commented-out TY2026
  placeholder + "add ONLY when verified" note are byte-identical ✓.
- Module docstring: separate "TY2024 values are encoded verbatim from" block (Rev. Proc. 2023-34
  §3.01/§3.03/§3.43/§3.41 + SSA 2023-10-12, l.14–21) and the original TY2025/Rev. Proc. 2024-40
  block (l.23–28); OBBBA note now says "**TY2025** bracket thresholds" + "(OBBBA is a 2025 enactment
  and does not affect TY2024 values.)" (l.30–35) — [R0-M1] discharged ✓.
- Five comment sites updated (docstring l.1–2; struct doc l.50–51; `load()` doc l.60–61;
  `optimize.rs:162` "tables (TY2024 and TY2025)"; `optimize_accept.rs:83–85` "cover TY2024 and
  TY2025"). Repo-wide `grep -rn "TY2025 only" crates/` → **zero hits** ✓. Both CLI hunks are
  comment-only (verified in diff: every +/− line is `///` or `//`).
- TUI blocker-flip (Task 2 item 11): `tax_tab_year_change_updates_figures` PASS recorded in the
  report (blocker at 2024 now `TaxProfileMissing`); `optimize_run_pre2025_is_usage_error` PASS —
  accepted with the green gate.

## 4. Regression

- **TY2025 byte-identical:** extracted `fn ty2025()` from `42ddab8` and `e437a74` and diffed —
  **identical** (verified with `git show | sed | diff`, not just hunk inspection). All TY2025 KATs
  unmodified.
- **Engine untouched:** `git diff 42ddab8..e437a74 -- crates/btctax-core` → **empty**. Statutory
  constants (`NIIT_RATE`, `niit_threshold`, `loss_limit`, SE constants,
  `QUALIFIED_APPRAISAL_THRESHOLD`) untouched; `statutory_values_are_not_in_the_table_and_constant_across_years`
  unmodified.
- **Diff-stat matches the review package exactly** (6 files: report, tax_tables.rs, optimize.rs,
  optimize_accept.rs, SPEC, R0 — 1392 insertions / 12 deletions); no stray files; working tree clean.
- `missing_year_returns_none` (2099) and the 2026-dependent `carryforward_mismatch_advisory_rendered`
  preserved (2026 stays unbundled).

## 5. Exactness / determinism

Every value is `dec!(…)` (rust_decimal); grep for `f64|f32|as f` over `tax_tables.rs` → no hits.
`load()` is pure BTreeMap construction, no I/O, deterministic. `source` is `&'static str`.

---

## 6. Findings

### M1 (Minor, non-blocking) — not all 28 bracket edges are pinned by a direct test assert

The KAT set implements the R0-GREEN spec exactly, but as a forward-regression net it leaves some
edges unpinned: A1/A2 assert 5 edges directly; the A6 compute KATs pin only the boundary they
straddle, because `ord_delta = tax_with − tax_without` cancels any error in edges below the fixture
window. Concretely un-asserted (exact-value) edges include the HoH 35% trap $243,700, all three
$191,950 32% edges, MFJ $383,900/$487,450, and the Single/MFS mid-table edges. The transcription is
verified correct by this review against IRB 2023-48, so this is not an error today — it is a
hardening gap for future accidental edits (the TY2025 tables share it). **Suggested FOLLOWUP (not a
gate item):** add a full-schedule equality KAT per status (assert the entire `brackets` vec) for
TY2024 and TY2025.

### N1 (Nit) — `leg24()` hardcodes `kat24_lot(1)` for every leg

`leg24` (l.477) always uses lot `kat24-1` regardless of position; harmless since every TY2024
fixture has exactly one leg. Cosmetic only; correct on next touch if multi-leg TY2024 fixtures are
ever added.

---

## 7. Disposition

| # | Severity | One-line | Blocking |
|---|----------|----------|----------|
| M1 | Minor | Not all 28 edges test-pinned (delta KATs cancel lower-edge errors); FOLLOWUP: full-schedule KAT | no |
| N1 | Nit | `leg24` hardcodes lot 1 | no |

**Gate result: 0 Critical / 0 Important — GREEN. Ready to merge.**

Transcription verified digit-by-digit spec → code with zero mismatches; the spec's payload
independently re-verified against IRB 2023-48 and FR Doc. 2023-23317 by this reviewer; all four
traps confirmed in code AND primary source (HoH 35% @ 243,700; MFS 37% @ 365,600; MFS LTCG
max_fifteen 291,850; 32% rows @ 191,950); TY2025 byte-identical; engine/statutory constants
untouched; KAT expectations match the C1/I1-corrected spec exactly.
