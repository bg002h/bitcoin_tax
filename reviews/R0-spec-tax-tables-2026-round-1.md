# R0 — SPEC_tax_tables_2026 — round 1

- **Artifact:** `design/SPEC_tax_tables_2026.md` (DRAFT)
- **Baseline:** branch `feat/tax-tables-2026` @ `e18e56d` (main == `f97adac`)
- **Reviewer role:** independent architect + tax-data verifier (read-only; no implementation)
- **Bar:** 0 Critical / 0 Important
- **Primary source:** Rev. Proc. 2025-32 (I.R.B. 2025-45) PDF, read directly via the Read tool
  (`…/tool-results/webfetch-1783275393363-f79lh1.pdf`); §4.01 pp.10–12, §4.03 p.13, §2.14 p.8,
  §4.42 p.25. SS wage base cross-checked against SSA/press coverage (not in the Rev. Proc.).

## VERDICT: 0 Critical / 2 Important / 3 Minor / 2 Nit — NOT R0-GREEN

Every 2026 tax figure in the spec is **exactly correct** against the primary source — including
all three transcription traps the prompt flagged (HoH 32%/35% = 201,750/256,200; MFS 37% = 384,350).
No Critical. The two Important findings are both in the **wiring / regression analysis**, not the
numbers: (I1) the `table_for` mechanism the spec describes does not exist in the code, and (I2) the
spec's "no logic change / no regression possible" claim omits the TY2026 `NotComputable → Computed`
behavior flip and the existing test/doc-comments that encode the "2026 is unbundled" assumption.

---

## PART A — FIGURE VERIFICATION (re-derived from the PDF; every figure confirmed)

### A.1 Ordinary brackets — §4.01, Tables 1–4 (pp.10–12). ALL MATCH.

Lower bound of each rate, transcribed from the "If Taxable Income Is / Over $X but not over" column:

| Status (Table) | 10% | 12% | 22% | 24% | 32% | 35% | 37% | PDF pp. | Spec |
|---|--:|--:|--:|--:|--:|--:|--:|--|--|
| MFJ/QSS (Tbl 1, §1(j)(2)(A)) | 0 | 24,800 | 100,800 | 211,400 | 403,550 | 512,450 | 768,700 | p.10 | ✅ |
| HoH (Tbl 2, §1(j)(2)(B)) | 0 | 17,700 | 67,450 | 105,700 | **201,750** | **256,200** | 640,600 | p.10–11 | ✅ |
| Single (Tbl 3, §1(j)(2)(C)) | 0 | 12,400 | 50,400 | 105,700 | 201,775 | 256,225 | 640,600 | p.11 | ✅ |
| MFS (Tbl 4, §1(j)(2)(D)) | 0 | 12,400 | 50,400 | 105,700 | 201,775 | 256,225 | **384,350** | p.11–12 | ✅ |

**The three flagged traps all verify EXACTLY:**
- **HoH 32% = $201,750, 35% = $256,200** (p.10–11) — distinct from Single's $201,775 / $256,225.
  Independently corroborated by the PDF's own base-tax column: HoH "Over $201,750 … **$39,207** plus
  32%" — and 24% band 105,700→201,750 = 96,050 × 24% = 23,052; cumulative 16,155 + 23,052 = **39,207**.
  Single's "Over $201,775 … **$41,024** plus 32%" (24% band 105,700→201,775 = 96,075 × 24% = 23,058;
  17,966 + 23,058 = 41,024). The two schedules are genuinely different at the 32%/35% edges; the
  spec's numbers are the HoH ones. ✅
- **MFS 37% = $384,350** (p.12) — PDF "Over $384,350 … $103,291.75 plus 37%"; = ½ × MFJ 768,700. ✅
- **MFS lower bands 10–35% = Single** (p.11–12) — confirmed identical. ✅

Full internal-consistency cross-check of MFJ base-tax also passes (…512,450 → $116,896; 35% band ×
256,250 = 89,687.50 → over 768,700 = **$206,583.50**, matching the PDF). High confidence the entire
§4.01 transcription is right.

### A.2 §1(h) LTCG breakpoints — §4.03, p.13. ALL MATCH.

| Status | max_zero | max_fifteen | PDF (p.13) | Spec |
|---|--:|--:|--|--|
| Single ("All Other Individuals") | 49,450 | 545,500 | 49,450 / 545,500 | ✅ |
| MFJ/QSS ("MFJ & Surviving Spouse") | 98,900 | 613,700 | 98,900 / 613,700 | ✅ |
| HoH ("Heads of Household") | 66,200 | 579,600 | 66,200 / 579,600 | ✅ |
| MFS ("MFS Returns") | 49,450 | 306,850 | 49,450 / 306,850 | ✅ |

(Estates & Trusts $3,300 / $16,250 present in the Rev. Proc. but correctly NOT modeled — the
`TaxTable` only carries the four individual statuses, matching TY2024/TY2025.)

### A.3 Ancillary scalars

- **gift_annual_exclusion = $19,000** — §4.42(1), p.25 verbatim: "For calendar year 2026, the first
  **$19,000** of gifts to any person … under § 2503 …". Spec cite "§4.42(1)" is exact. ✅
- **gift_lifetime_exclusion = $15,000,000** — §2.14, p.8 verbatim: ".14 Section **70106** of the OBBBA
  amends § 2010(c)(3) by increasing the basic exclusion amount to **$15,000,000** for calendar year
  2026. … The basic exclusion amount will be adjusted for inflation for calendar year **2027** and
  future years." Confirms BOTH the amount, the OBBBA Pub. L. 119-21 §70106 authority, AND the spec's
  claim that it is a flat statutory figure first inflation-indexed in 2027 (so cite OBBBA, not a
  §4.xx). ✅
- **ss_wage_base = $184,500** — NOT in the Rev. Proc. (it is an SSA §230 / 42 U.S.C. §430 figure).
  Cross-checked on the web: the 2026 OASDI contribution & benefit base is **$184,500** (up from
  $176,100 in 2025), confirmed by SSA and multiple payroll/tax sources. Value is **correct**. See
  Minor M2 re the exact announcement-date cite.

**No figure is wrong or unconfirmed.** (0 Critical.)

---

## PART B — WIRING / DESIGN VERIFICATION

### [Important I1] The `table_for` mechanism in the spec does not exist in the code

Spec §Mechanism (line 48–49) instructs:
> `table_for` — add the `2026 => Some(&self.ty2026)` arm …

That is a **match-on-year with a `self.ty2026` struct field** — neither construct exists.
`crates/btctax-adapters/src/tax_tables.rs`:
- `BundledTaxTables` stores a **`BTreeMap<i32, TaxTable> by_year`** (line 55–57), not per-year fields.
- Tables are **eagerly built in `load()`** via `by_year.insert(2024, ty2024())` / `insert(2025,
  ty2025())` (lines 63–65) — NOT lazily, NOT match-constructed.
- `table_for` (line 73–75) is just `self.by_year.get(&year)` and **needs no change at all**.
- There is already a **ready-to-uncomment stub**: line 66 `// by_year.insert(2026, ty2026());` and
  line 67 `// ^ add ONLY when verified vs Rev. Proc. 2025-32 + OBBBA structural law`.

So the correct wiring is a single line in `load()` (uncomment/add `by_year.insert(2026, ty2026());`),
not a new `table_for` arm. The spec's parenthetical hedge ("mirror how 2024/2025 are stored/
dispatched; check whether tables are lazily built or stored in a field and follow the existing
pattern exactly") points the right way, but the concrete instruction it sits next to is factually
wrong about this codebase and ignores the existing stub. The prompt asked specifically that "the spec
must match the real pattern"; it does not. **Fix:** replace the `2026 => Some(&self.ty2026)` sentence
with "uncomment/add `by_year.insert(2026, ty2026());` in `BundledTaxTables::load()` (line 66); the
`BTreeMap`-backed `table_for` requires no change."

### [Important I2] Unacknowledged TY2026 behavior change + no audit of the "2026-is-unbundled" call sites

The spec asserts "Data-add only; no logic change" (line 5), "no 2024/2025 regression is possible"
(prompt paraphrase; spec KAT line 59), and lists only `ty2025_and_2024_unchanged` as the regression
guard. But bundling TY2026 is an **observable behavior change for 2026 itself**: every caller of
`table_for(2026)` flips from `None` to `Some`, and any compute path for year 2026 flips from
`TaxOutcome::NotComputable(TaxTableMissing)` to `Computed`. The spec does not mention this, and there
are existing sites that encode the old assumption:

1. **`crates/btctax-cli/tests/tax_report.rs:586-675`** — `carryforward_mismatch_advisory_rendered`.
   Its docstring (line 599) and inline comment (line 654) state the tested mechanism is
   "2026 TaxTable is not bundled → **NotComputable(TaxTableMissing)**". It calls
   `cmd::tax::report_tax_year(&vault, &pp(), 2026, …)`, and `report_tax_year` uses the **real**
   `BundledTaxTables::load()` (`crates/btctax-cli/src/cmd/tax.rs:93`). After bundling 2026 the main
   outcome becomes `Computed`. The test will most likely **still pass** (its three assertions are all
   about the M4 advisory, which `render_tax_outcome` appends in *both* arms —
   `crates/btctax-cli/src/render.rs:1083-1085` "render after the main block … regardless of whether
   the outcome is Computed or NotComputable"), but its documented premise is invalidated and the
   intended "advisory renders even on NotComputable" coverage for 2026 is silently lost. This test
   must be **re-pointed to a still-unbundled year (2027)** or restructured, and its comments fixed.
2. **`crates/btctax-cli/src/optimize.rs:1309`** *(doc)* and **stale comments** at tax_report.rs:599/654
   — see Minor M1.
3. **Checked and CLEAR (no regression), but the spec should record the audit:**
   - `crates/btctax-core/tests/optimize_mode2.rs:176,213` ("2026 unbundled → timing None") uses a
     **local `OneTable` test double** `synth(2025)` (helper at line 47), independent of
     `BundledTaxTables` — unaffected. ✅
   - `crates/btctax-cli/tests/export.rs:388` exports with `Some(2026)` but sets **no 2026 profile**,
     so `export_snapshot`'s only table use (the SE-tax path, `admin.rs:70-88`) stays `None` either
     way. ✅
   - `crates/btctax-cli/tests/tax_profile.rs:113` (`show_profile(2026) == None`) is profile-store
     state, not table-dependent. ✅
   - No test asserts `BundledTaxTables::load().table_for(2026).is_none()` directly
     (`missing_year_returns_none` uses 2099 — adapters line 432), so there is **no hard RED** from an
     availability assertion. ✅

**Fix:** the spec must (a) state the intended TY2026 `NotComputable → Computed` behavior change
explicitly; (b) include the tax_report.rs M4 test update (re-point to 2027) in the plan; (c) list the
audited-and-clear sites above so the whole-diff reviewer isn't operating on the false "no regression
possible" premise. This is exactly the class of thing R0 exists to catch before implementation.

### Wiring items that CHECK OUT

- **`ty2025()` is the right template** (lines 225–341): same shape the spec proposes to mirror
  (BTreeMap `ordinary`/`ltcg`, all four statuses, QSS omitted, then the scalar fields). ✅
- **`TaxTable` fields** (`crates/btctax-core/src/tax/tables.rs:53-82`): `year: i32`,
  `source: &'static str`, `ordinary`, `ltcg`, `gift_annual_exclusion`, `ss_wage_base`,
  `gift_lifetime_exclusion` — exactly the seven the spec lists. `source` is `&'static str`, so the
  spec's literal source string is well-typed. ✅
- **QSS aliases MFJ via `TaxTable::key`** (tables.rs:87-92: `Qss → Mfj`, all else identity), used by
  `ordinary_for`/`ltcg_for` (94-106). Spec's "QSS aliases MFJ (no separate entry)" is correct; the
  `full_schedule` KAT already asserts Qss is not a stored key (adapters 867-882). ✅
- **KAT plan mirrors the existing `ty2025_*` tests** (adapters 354-455): single/mfj/hoh/mfs bracket
  KATs, `ltcg_breakpoints_all_statuses`, `mfs_37_pct_...`, `gift_annual_exclusion_is_19000`, and the
  shared `statutory_values_...` guard (447-455, whose comment 453-454 literally invites the 2026
  contrast). The spec's additional `ss_wage_base` / `lifetime_exclusion` KATs are *stronger* than
  TY2025 (which bundles those only into the TY2024 `ancillary_fields` KAT) — fine. ✅
- **Fault-inject target is meaningful:** a `ty2026_hoh` KAT pinning 32% = $201,750, with the inject
  swapping to Single's $201,775, is a real $25 discriminator that exercises exactly the flagged
  transcription trap and would go RED. ✅
- **STATUTORY constants correctly out of scope:** `NIIT_RATE`, `SE_RATE_*`, `SE_NET_EARNINGS_FACTOR`,
  `se_addl_medicare_threshold`, `niit_threshold`, `loss_limit`, `QUALIFIED_APPRAISAL_THRESHOLD` all
  live in `tables.rs:127-209` as year-independent items, never in a `TaxTable` (I4). None change for
  2026; the spec is right to leave them untouched. ✅
- **No other year-literal dispatch exists.** `rg '202[0-9]\s*=>'` finds only the unrelated
  `Persistability::ForbiddenBroker2027`. All `table_for` callers (cli render/cmd, tui export/tax,
  core optimize/compute) route generically through the trait — no per-caller 2026 arm needed. The
  `synthetic_table` helper (tables.rs:218) takes `year` as a parameter and needs no 2026 case. ✅

---

## MINOR

- **[M1] Stale doc after add.** (a) `optimize.rs:1309` doc says "a 2026+ re-score would hit a missing
  bundled table → NotComputable and fail the whole consult" — after 2026 is bundled this reasoning is
  wrong for 2026 (the actual protection is the `latest_crossover.year() != at.year()` guard at line
  1338, which trips first regardless of bundling, so behavior is safe — but the *stated rationale*
  should be corrected to cite the cross-year guard). (b) The module header + struct docs in
  `tax_tables.rs` (lines 1, 37-40 "# TY2026 … omitted", 48-51 "Currently contains TY2024 and TY2025",
  60-67 including the `// by_year.insert(2026…)` comment) all need updating when 2026 lands. The spec
  should list these doc edits so the diff isn't left self-contradictory.
- **[M2] SS wage-base date cite.** The *value* $184,500 is confirmed, but the spec cites "SSA
  2025-10-24" as the announcement date. Sources place the SSA 2026 COLA/wage-base announcement in
  mid-to-late Oct 2025 with the Federal Register determination dated 2025-11-03; the precise date
  string should be pinned to a citable SSA source (fact sheet / Federal Register) rather than asserted,
  since — unlike every other figure — it cannot be verified from the Rev. Proc. The spec already
  correctly flags this field as SSA-sourced, not Rev.-Proc.-sourced.
- **[M3] `source` string spans two authorities without the year's Rev.-Proc. §-for-scalars form.** The
  proposed `source` (spec line 43) mixes §4.01/§4.03/§4.42 + OBBBA + SSA; that's accurate, but note
  TY2024/TY2025 `source` strings also name the §-for-the-basic-exclusion (e.g. TY2025 "§2.41"). For
  2026 the basic exclusion is OBBBA-set (no §4.xx), so citing §70106 is right — just confirm the KAT
  that reads `source` (if any) isn't pinned to a "§4.NN" substring. (No such KAT proposed; noting for
  completeness.)

## NITS

- **[N1]** Spec names a KAT `ty2025_and_2024_unchanged`; no single existing test has that name — TY2024
  and TY2025 coverage is spread across their respective KAT clusters. Either add the new combined guard
  or cite the existing tests it stands in for. Cosmetic.
- **[N2]** Spec line 61 "`statutory_values_are_not_in_the_table_and_constant_across_years` —
  extend/confirm it still holds for 2026": the existing test (adapters 447-455) already contains a
  comment (453-454) describing the intended 2026 indexed-vs-statutory contrast; wiring the 2026 arm
  into that same test satisfies the intent. No change needed beyond following that comment.

---

## Bottom line

All 2026 tax data is transcribed correctly — the numbers are safe to ship. The blockers are two
non-numeric wiring/analysis defects: **I1** (the `table_for` change is a one-line `by_year.insert` in
`load()`, not the described `2026 => Some(&self.ty2026)` match arm) and **I2** (the spec must own the
TY2026 `NotComputable → Computed` behavior change and the `tax_report.rs` M4 test / doc-comments that
assume 2026 is unbundled). Resolve I1 + I2 (and fold the Minors) → re-review → expected GREEN.

**Sources (SS wage base cross-check):**
- [SSA — Contribution and Benefit Base](https://www.ssa.gov/oact/cola/cbb.html)
- [SSA — 2026 COLA Fact Sheet](https://www.ssa.gov/news/en/cola/factsheets/2026.html)
- [The Tax Adviser — Social Security wage base and COLA announced for 2026](https://www.thetaxadviser.com/news/2025/oct/social-security-wage-base-and-cola-announced-for-2026/)
- [Federal Register — Cost-of-Living Increase and Other Determinations for 2026](https://www.federalregister.gov/documents/2025/11/03/2025-19763/cost-of-living-increase-and-other-determinations-for-2026)
