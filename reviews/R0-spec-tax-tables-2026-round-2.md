# R0 — SPEC_tax_tables_2026 — round 2 (verification of round-1 folds)

- **Artifact:** `design/SPEC_tax_tables_2026.md`
- **Baseline:** branch `feat/tax-tables-2026` @ `9443628` (main == `f97adac`)
- **Reviewer role:** independent architect (read-only; no implementation)
- **Bar:** 0 Critical / 0 Important
- **Round-1 review folded:** `reviews/R0-spec-tax-tables-2026-round-1.md` (was 0C/2I/3M/2N).
  Per the round-2 charge, the tax **figures were verified exact in round 1 and are NOT
  re-verified here** — this pass audits only the wiring / regression / doc folds against
  CURRENT source.

## VERDICT: 0 Critical / 0 Important / 2 Minor / 2 Nit — **R0-GREEN** (cleared to implement)

Both Important findings from round 1 are correctly folded and confirmed against current
source. I1 (wiring) now describes the real `BTreeMap`/`load()` mechanism exactly, and the
old wrong `2026 => Some(&self.ty2026)` form survives only as an explicit labeled retraction.
I2 (behavior change) is now owned: the `NotComputable → Computed` flip is stated, the M4
test re-point is planned, and every "audited-CLEAR" site checks out against source. I
independently swept `2026` across `crates/` and found **no additional site** that assumes
the *real* `BundledTaxTables(2026)` is `None`. The two Minors are precision refinements
(a residual "no logic change" phrase in the intro; the "re-point to 2027" mechanics are
under-specified); neither blocks implementation. Nits are citation-precision and
completeness.

---

## I1 (wiring) — FOLDED, CONFIRMED ✅

Verified against `crates/btctax-adapters/src/tax_tables.rs`:

- `by_year` is `BTreeMap<i32, TaxTable>` — **tax_tables.rs:55-57** (`pub struct BundledTaxTables { by_year: BTreeMap<i32, TaxTable> }`). ✅
- 2024/2025 eagerly inserted in `load()` — **tax_tables.rs:63-65** (`by_year.insert(2024, ty2024());` / `insert(2025, ty2025());`). ✅
- Line **66** is the commented 2026 stub — `// by_year.insert(2026, ty2026());`, with the guard comment at **67** (`// ^ add ONLY when verified vs Rev. Proc. 2025-32 + OBBBA structural law`). ✅
- `table_for` is `self.by_year.get(&year)` and needs NO change — **tax_tables.rs:73-75**. ✅

Spec §Mechanism (lines 52-56) + Plan T1 (lines 94-95) both now state the correct operative
instruction: "uncomment `by_year.insert(2026, ty2026())` in `load()` (tax_tables.rs:66);
`table_for` (73-75) needs NO change." The old wrong `2026 => Some(&self.ty2026)` phrasing is
**gone as an instruction**; it appears once (line 55-56) only as an explicit self-correction
("My earlier … match-arm description was wrong for this codebase — no such field/match").
That is acceptable — it documents *why* the earlier hedge was wrong and is not a competing
instruction. **No residual I1 defect.**

## I2 (behavior change owned) — FOLDED, CONFIRMED ✅

**The flip is now owned.** Spec line 61 (§heading `[R0-I2] Behavior change + regression
audit (NOT "no logic change")`), lines 62-63 (`table_for(2026)` flips `None → Some` ⇒ 2026
compute flips `NotComputable(TaxTableMissing) → Computed`), and SemVer line 90 ("own it, not
'no logic change'"). ✅

**M4 test really uses 2026 against the real tables ⇒ genuinely needs re-pointing.**
`crates/btctax-cli/tests/tax_report.rs:601 carryforward_mismatch_advisory_rendered` calls
`cmd::tax::report_tax_year(&vault, &pp(), 2026, dec!(0))` (**tax_report.rs:656**), and
`report_tax_year` builds the **real** `BundledTaxTables::load()`
(`crates/btctax-cli/src/cmd/tax.rs`, `let tables = …load(); … n_year(…, year, …, &tables)`).
Today 2026 is unbundled ⇒ main outcome `NotComputable`; after T1 it becomes `Computed`. ✅
The stale premise comments are exactly where the spec says: **tax_report.rs:599**
("2026 TaxTable is not bundled → … NotComputable(TaxTableMissing); exit 0") and **:654**
("main outcome is NotComputable (no TY2026 table)"). ✅ The "likely still PASSES" claim is
sound — the advisory renders in **both** arms: `render.rs:1083-1087` ("render after the main
block … regardless of whether the outcome is Computed or NotComputable" + `if let Some(msg)
= advisory { … ADVISORY (M4) … }`). ✅

**2027 is a valid replacement (still unbundled after the change).** After T1, `load()`
inserts only 2024/2025/2026; 2027 is never inserted ⇒ `table_for(2027) → None`. Consistent
with the spec's "no 2027 — unpublished" statements (lines 6, 91, 107). ✅

**"Audited CLEAR" list — all four confirmed against source:**
- `optimize_mode2.rs:176/213` uses a **local `OneTable` double** — struct at
  `crates/btctax-core/tests/optimize_mode2.rs:42-46`, `fn synth(year)` at **:47**; the test
  `consult_picks_high_basis_lot_min_tax` (**~176**) asserts `r.timing.is_none()` (**~213**)
  because the crossover lands in 2026, which is absent from its **own** `synth(2025)` table —
  independent of `BundledTaxTables`. Unaffected. ✅
- `export.rs:388` — `cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2026), None)`; the
  test sets **no tax profile** (only a donation reclassify), so the SE-tax path has no 2026
  profile and stays `None` regardless of table availability. ✅
- `tax_profile.rs:113` — `show_profile(&vault, &pp(), 2026).unwrap() == None` is
  profile-store state, not table-dependent. ✅
- `missing_year_returns_none` uses **2099** — `tax_tables.rs:432-433`
  (`table_for(2099).is_none()`); no test asserts `table_for(2026).is_none()`, so no hard RED
  from an availability assertion. ✅

**Any OTHER site that assumes 2026 is unbundled? — No (I swept `2026` across `crates/`).**
- The **only** real-`BundledTaxTables` 2026 compute in the whole test suite is the M4 test
  above; every other `report_tax_year`/`n_year` test uses 2025 or a runtime year. The
  production path (`main.rs` runtime `y`) is the *intended* arming, not a regression. ✅
- `crates/btctax-core/tests/optimize_mode1.rs` uses `synth(2026)` **heavily** (e.g. lines
  277-571), but `synth` returns a **local `OneTable`** double (**optimize_mode1.rs:28-34**),
  so it constructs its *own* 2026 table and does not assume the real tables are `None` —
  same independent-double pattern as the cited mode2. Genuinely CLEAR (see Nit N2 re the
  audit list not enumerating it).
- `tax_compute.rs:542` (`date!(2026-01-01) // wrong year → ignored`) and `kat_forms.rs`
  2026 dates are `btctax-core` tests using synthetic tables (core cannot depend on adapters);
  they are disposal-date literals, not table lookups. Unaffected. ✅
- All other `2026` hits (`verify_report.rs`, `reconcile.rs`, `optimize_run/accept.rs`,
  `tui/export.rs`, `tui-edit`) are attestation/`now`-clock/export-timestamp/donation-date
  literals — none compute a 2026 tax outcome against the real bundled tables. ✅

**No residual I2 defect.**

## M1 (stale docs) — FOLDED, all locations exist and say what round 1 claimed ✅

- `tax_tables.rs:1-2` header — "Bundled per-year tax tables — **TY2024 and TY2025** …". ✅
- `tax_tables.rs:37-40` — `# TY2026` / "TY2026 is **omitted** (pending verification …) …
  receive `None` … `NotComputable(TaxTableMissing)`". ✅
- `tax_tables.rs:48-51` — struct doc "Currently contains **TY2024** … and **TY2025** …
  TY2026 will be added once verified". ✅
- `tax_tables.rs:60-67` — `load()` doc ("TY2024 and TY2025 bundled; later years added …")
  plus the commented stub 66/67. ✅
- `optimize.rs:1309` rationale — confirmed at **`crates/btctax-core/src/optimize.rs:1309`**
  ("a 2026+ re-score would hit a missing bundled table → `NotComputable` and fail the whole
  consult"), and the real guard `if latest_crossover.year() != at.year()` is at
  **:1338**. ✅ (See Nit N1 — the spec cites this as bare `optimize.rs`, but the file is in
  `btctax-core`, not `btctax-cli`.)

## M2 / N1 (round-1 minors) — FOLDED ✅

- **M2 (SS wage-base cite):** now "Federal Register **2025-11-03**" (spec line 40) and the
  `source` string "SS wage base $184,500 per SSA (Fed. Reg. 2025-11-03)" (line 47). This is
  the exact determination the round-1 review's own Sources list endorsed
  (`…/2025/11/03/2025-19763/…`), and the spec correctly keeps flagging the field as
  SSA-sourced / NOT in the Rev. Proc. (line 39). **Acceptable.** ✅
- **N1 (KAT name):** renamed to `ty2024_and_2025_tables_unchanged` and explicitly tagged
  "[R0-N1 — a NEW spot-check … not an existing test name]" (spec lines 81-82). Honest and
  addresses the round-1 nit. ✅

---

## MINOR (do not block; recommend folding)

- **[M-a] Residual "no logic change" in the intro contradicts the folded I2/SemVer framing.**
  Spec line 7 still reads "Data-add only; **no logic change**", while §I2's own heading (line
  61) is "Behavior change + regression audit (**NOT "no logic change"**)" and SemVer line 90
  says "own it, **not 'no logic change'**". The whole point of the I2 fold was to retract the
  blanket "no logic change" claim; leaving it verbatim in the summary line is a literal
  self-contradiction (a reader of only the intro gets the wrong framing). A full read
  resolves it, and the Plan/Mechanism are correct, so this is Minor. **Fix:** reword line 7
  to "Data-add that ARMS TY2026 — see [R0-I2]: owns the `NotComputable → Computed` flip".

- **[M-b] "Re-point to 2027" is under-specified — a literal `2026 → 2027` swap goes RED.**
  Plan step (a) (lines 96-97) says "RE-POINT `tax_report.rs:586-675 …` to year 2027 + fix
  its docstring/comment (599/654)." But the M4 advisory compares year-N declared
  `carryforward_in` against **year-(N-1)** *computed* `carryforward_out`
  (`cmd/tax.rs`: `prior_out = n_year(…, year - 1, …)`). The loss that produces the
  `{short: 7000}` carryforward is in the **2025** CSV/profile. If the implementer changes
  only the year literals to 2027, the comparison becomes 2027-declared vs **2026**-computed
  (which is `{0,0}` — no 2026 disposition) ⇒ **advisory MATCHES ⇒ does not fire ⇒ the three
  assertions fail RED.** A correct re-point must shift the **whole scenario forward one
  year**: CSV disposition dates 2025→2026, the "2025" profile→2026, the "2026" profile→2027,
  **and** the entire docstring narrative (lines **586-598**, not just 599/654). This is
  TDD-catchable (the test will go RED and force the fix), so it is Minor — but the plan
  should state "re-point = full one-year forward shift of the scenario + narrative", not
  "change 2026 to 2027 + fix 599/654", so T1 doesn't stall on a confusing RED.

## NITS

- **[N1] `optimize.rs` citation is not crate-qualified.** Two `optimize.rs` files exist
  (`crates/btctax-cli/src/cmd/optimize.rs` and `crates/btctax-core/src/optimize.rs`); only
  **`btctax-core/src/optimize.rs`** carries the 1309 rationale + 1338 guard. Spec lines 88
  and 99 write bare `optimize.rs` (line 88 even lists it right after `btctax-cli/tests/…`,
  inviting a wrong-crate read). Line numbers + content disambiguate, but fully-qualify to
  `crates/btctax-core/src/optimize.rs` for the implementer.

- **[N2] Audit list names only `optimize_mode2`; `optimize_mode1` is the same clear pattern.**
  The "audited CLEAR" enumeration (spec lines 70-74) cites `optimize_mode2.rs:176/213` as the
  local-double exemplar but omits `optimize_mode1.rs`, which also drives `synth(2026)` (a
  local `OneTable`, `optimize_mode1.rs:28-34`) and is equally independent of
  `BundledTaxTables`. Not a defect (it constructs its own 2026 table; it does not assume the
  real tables are `None`) — noting for completeness so the whole-diff reviewer isn't
  surprised by mode1's `synth(2026)` usage.

---

## Self-consistency & implementability

- Aside from **M-a** (the one leftover "no logic change" phrase) the spec is internally
  consistent: figures, Mechanism, I2, SemVer (PATCH), Gotchas, and Plan agree; QSS-aliases-MFJ,
  statutory-constants-out-of-scope, and the fault-inject target are unchanged from the
  round-1-verified content.
- The Plan **is implementable** as `T1 = write failing KATs → add `ty2026()` → uncomment the
  one `load()` insert (tax_tables.rs:66) → green → fold regression (M4 re-point + doc
  updates)`, provided "re-point" is read as the full one-year scenario shift of **M-b**
  (which TDD will enforce regardless).

**Bottom line:** Both round-1 Importants (I1 wiring, I2 behavior-change-owned) are correctly
folded and verified against current source; the round-1 Minors/Nit (M1 docs, M2 SS-cite, N1
KAT-name) are addressed; no additional 2026-is-unbundled site was missed. 0 Critical / 0
Important ⇒ **R0-GREEN — cleared to implement.** Recommend folding M-a (one-line reword) and
M-b (spell out the re-point year-shift) opportunistically during T1; N1/N2 are cosmetic.
