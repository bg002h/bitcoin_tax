# IMPL review — Full Return Phase 0 (r1, Fable, independent)

- **Scope:** `git diff 6ef29c1 HEAD -- crates/` (3 commits: bce4f53, 46353cc, e6efb0f); 6 files, +501/−4.
- **Implements:** SPEC_full_return §3.1/§8; IMPLEMENTATION_PLAN_full_return Phase 0; KATs 1, 2, 3, 9.
- **Reviewer:** Fable (independent; author was a different model). Date: 2026-07-12.
- **Validation run:** `cargo test -p btctax-core -p btctax-adapters --lib` → **core 114 passed / adapters
  58 passed / 0 failed**; `cargo clippy -p btctax-core -p btctax-adapters --lib --tests` → clean.
- **Verdict: NOT GREEN — 0 Critical / 4 Important / 4 Minor.** All tax arithmetic verified correct
  (independently re-derived against the IRS worksheet and the primary Rev. Proc.); the Importants are a
  false load-bearing doc-comment claim, a wrong primary-source citation, and two missing Phase-0
  deliverables (task-6 KAT-9 cross-foot; CI param cross-check).

---

## Critical

None.

## Important

### I-1 — False architectural justification: no frozen file constructs `TaxTable` by struct literal

`crates/btctax-core/src/tax/tables.rs:214-216` ("Kept OUT of [`TaxTable`] on purpose: a FROZEN file
(`tax/se.rs`) constructs `TaxTable` by struct literal, so adding a field there is forbidden by the
frozen-engine guard") and `crates/btctax-adapters/src/tax_tables.rs:90-91` (same claim); echoed in
`frozen_guard.rs`-adjacent prose.

**Verified false.** Exhaustive search: the only `TaxTable { … }` struct literals are
`tables.rs:297` (inside `synthetic_table`, which is `#[cfg(test)] pub(crate)` at `tables.rs:268` —
**unfrozen**; this very diff edits tables.rs) and the four adapter constructors
(`tax_tables.rs:251/385/511/649` — unfrozen). Frozen `se.rs` merely **calls** `synthetic_table(2025)`
(se.rs:184, 206) and never names `TaxTable` fields; frozen `compute.rs` takes `&dyn TaxTables`;
frozen `types.rs` never touches it. Adding a field to `TaxTable` (the plan-task-5 approach: "Bundle
per-year indexed data in `TaxTable` … as Option/defaulted") would have required edits only to
unfrozen files. So this is an **unrecorded deviation** from plan task 5 and spec §8 ("SALT cap
statutory-constant"; params "in `TaxTable`"), justified by an incorrect factual claim — exactly what
"verify citations against current source at write time" exists to catch.

**The design itself can stand** — it is defensible on true grounds: (a) a separate per-year
`FullReturnParams` fails closed for years without full-return data (2017/2025/2026 have `TaxTable`s
but return `None` → `NotComputable`), which Option-fields-on-`TaxTable` would make messier; (b)
`TaxTable` is a published (crates.io) all-pub-fields struct — new fields break downstream literal
constructors; (c) SALT/§402(g)/§1(g)/§63 amounts genuinely move year-over-year (OBBBA moved SALT for
2025+), so per-year beats "statutory constant" for fail-closed correctness.

**Fix:** rewrite both doc comments with the true rationale above (drop the se.rs claim entirely), and
record the plan-task-5 / spec-§8 deviation as a FOLLOWUPS erratum (pattern: `spec-s8-kat3-mod25`).
No code change required.

### I-2 — Wrong primary-source citation: Rev. Proc. 2023-34 §3.16 is Cafeteria Plans, not the standard deduction

`crates/btctax-adapters/src/tax_tables.rs:113` ("Rev. Proc. 2023-34 §3.16"), `:124` ("§3.16(3)"),
`:126` ("§3.16(2)").

**Verified against the primary source** (fetched `irs.gov/pub/irs-drop/rp-23-34.pdf` during this
review): **§3.15 = "Standard Deduction"** — §3.15(1) basic ($29,200 / $21,900 / $14,600 / $14,600),
§3.15(2) dependent ($1,300 / $450 + earned), §3.15(3) aged-or-blind ($1,550; $1,950 if unmarried and
not a surviving spouse). **§3.16 = "Cafeteria Plans"** (§125(i) FSA limit $3,200). The project's own
verbatim-read recon agrees (`recon/02-computation-worksheets.md:45,68` cites §.15 / §.15(3)); the
§3.16 error also appears in `reviews/DESIGN-audit-fold-confirm.md:29` (likely the contagion source —
flag it there too). All **dollar values are correct**; only the cites are wrong. In a tax product the
citation is the audit trail for the number — gate-relevant here.

**Fix:** s/§3.16/§3.15/ at all three sites (and note the audit-doc occurrence in FOLLOWUPS).

### I-3 — Plan task 6 (KAT-9's Phase-0 half) not implemented: the cross-foot arithmetic KAT is missing

Plan Phase 0 task 6 / KAT-ownership ("KAT 9 → P0 (arithmetic + round-mode)") / spec §10 KAT-9: two
`.50` components — 8959 Part I L7 = **271.50** + Part II L13 = **499.50** — must be `round_dollar`ed
per printed line → **272 + 500 = 772**, proving printed-line rounding + cross-foot beats
sum-then-round (`round_dollar(771.00) = 771`). Repo-wide search: no such test exists. Task 1's
mode-discriminating cells (1,163/303, `conventions.rs:120`) cover only KAT-9's *round-mode* half —
the plan splits these explicitly (task-1 note; FOLLOWUPS `pm-r2-m2`). Phase 0 is tasks 0–**6**;
acceptance can't be met with task 6 unlanded.

**Fix:** ~8-line test (in `conventions.rs` tests or `method.rs` tests): assert
`round_dollar(271.50) + round_dollar(499.50) == 772` and that the wrong composition
`round_dollar(271.50 + 499.50) == 771` — i.e., the fixture discriminates. (Re-assert on real 8959
lines at P4/P6 per plan.)

### I-4 — Phase-0 acceptance item "CI param cross-check green" not implemented

Plan Global invariants: "vendor a TY2024 slice of CC0 PSL Tax-Calculator params; a CI test diffs the
bundled `tax_tables.rs` standard-deduction / bracket / LTCG / NIIT / Add'l-Medicare values against
it (spec §9/deep05)". Phase 0 acceptance line ends: "…**CI param cross-check green.** FROZEN guard
green." Spec §9:478 mandates the same. Repo-wide search (crates, `.github/workflows/ci.yml`, data
files): nothing vendored, no such test. The FROZEN guard half was delivered; the cross-check half was
not, and Phase 0 is the phase that first bundles standard-deduction figures (task 5) — the check's
whole purpose.

**Fix:** vendor the TY2024 slice (CC0) + add the diff test; or, if deliberately re-scoped to a later
phase, that is a plan edit and must go through the §2 review loop and be recorded — not silently
skipped.

## Minor

- **M-1** `crates/btctax-adapters/src/tax_tables.rs:689` — the mod-25 sweep hardcodes
  `[2017, 2024, 2025, 2026]`. Iterate the bundled map's keys (test is in-module; `by_year` is
  accessible) so a future bundled year cannot silently escape the KAT-3 invariant ("assertion over
  every bundled `TaxTable` year").
- **M-2** `crates/btctax-adapters/src/tax_tables.rs:709` — `ty2024_full_return_params_bundled` never
  asserts `dependent_std_earned_addon` ($450, §3.15(2)) — the one `FullReturnParams` field with no
  test pin (`year` also unasserted).
- **M-3** `crates/btctax-core/src/tax/method.rs:60` — `regular_tax` (pub API, the ordinary-only 1040
  L16 path) has no direct test. Both underlying paths are covered via `worksheet_tax`, but a 2-assert
  KAT (one Table-path, one TCW-path input) closes the public surface.
- **M-4** `crates/btctax-core/src/tax/method.rs:71,84` — mixed worksheet numbering: "L5"/"L10" follow
  deep/01's compressed numbering (official 2024 QDCGT: bottom = L7, cap = L11 = min(L1, L6)) while
  L22–L25 follow the official sheet. Math verified correct; label the compressed cites
  ("deep/01 numbering") to prevent a future reviewer "correcting" the right code against the wrong
  line map.

---

## Checked and found CLEAN

1. **`round_dollar`** (`conventions.rs:37`): `MidpointAwayFromZero` to 0 dp — genuinely distinct from
   `round_cents` (`MidpointNearestEven`, 2 dp). Discriminating cells independently re-derived: MFJ
   bin [11,600, 11,650) midpoint 11,625 → 10% → 1,162.50 → **1,163** (half-even: 1,162); Single
   [3,000, 3,050) → 302.50 → **303** (half-even: 302). Fault-inject asserting `MONEY_ROUNDING` yields
   the wrong cells is valid and discriminating. Negative symmetry (−2.50 → −3) covered.
2. **`bin_midpoint`** (`method.rs:27`): probed 0, 4, 5, 14, 15, 24, 25, 49, 2,975, 2,999, 3,000,
   58,049, 99,999 — every bin/boundary matches the IRS structure ([0,5)/[5,15)/[15,25); $25 bins to
   3,000; $50 bins to 100,000; midpoint = lower + width/2). `(ti/width).floor()*width` is exact
   Decimal arithmetic on the domain (ti < 100,000, width 25/50); no off-by-one found.
3. **`worksheet_tax`/`qdcgt_line16`** (`method.rs:47,74`): Table-vs-TCW selection inclusive at
   exactly $100,000 → TCW ("$100,000 or more"), chosen independently per amount (matches official
   L22/L24 instructions). All five fixtures **independently re-derived against the official 25-line
   2024 QDCGT worksheet**: (a) MFJ 85,000/QD 6,000 → L22 = table(79,000) = 9,019, pref all in 0% band
   → **9,019** ✓; (b) Single 120,000/LTCG 20,000 → L23 = TCW(100,000) = 17,053.00 + 15%×20,000 =
   20,053 binds vs L24 = 21,842.50 → **20,053** ✓; (c) Single 60,000/QD 2,000 → 7,819 + 300 =
   **8,119** ✓; KAT-1 35,400/QD 50,000 → **$0** ✓ (cap = official L11 = min(L1, L6); uncapped ⇒
   446.25 → 446 — the KAT discriminates); KAT-2 58,010/QD 10 → L23 = 7,820.50 vs L24 = 7,819, min
   binds → **7,819** ✓ (min removed ⇒ 7,821 — discriminates). `bottom = clamp(TI − pref_full)` =
   official L7; `pref = min(TI, pref_full)` = official L11; frozen `preferential_tax` stacking
   re-verified by hand for each case. "Carry cents in worksheets, round once at line 16" matches the
   spec §3.1 election; `round_dollar` applied exactly once (Table values already whole → idempotent).
4. **`first_unbinnable_edge`** (`method.rs:98`): mod-25 predicate correct; 0-edge and ≥$100k edges
   excluded; Decimal `%` exact for the domain; midpoint edge 11,925 (≡ 25 mod 50) permitted with the
   pinned cell 1,192.50 → **1,193** = the real TY2025 Single [11,900, 11,950) value (the synthetic
   schedule shares the ≤11,925 region with real TY2025 Single, so the pin is faithful); 12,340 caught.
   Real TY2024 Single/MFJ pass; adapter sweep covers 2017/2024/2025/2026 × 4 statuses, green.
5. **Frozen guard** (`frozen_guard.rs`): pins **content** (SHA-256 over `include_bytes!` — compile
   -time, hermetic, no runtime IO; an edit changes the embedded bytes → test fails). Recomputed all
   three hashes: `types.rs` 0d51da82…, `compute.rs` 38e87b7d…, `se.rs` 3aba83c2… — **match the pins
   AND match the same files at base commit 6ef29c1** → no frozen file was edited in this diff
   (`git diff --name-only` confirms: only the 6 reviewed files). `.gitattributes` (`* text=auto
   eol=lf`) protects the pins from CRLF divergence on the Windows CI matrix. `sha2` was already a
   core dependency (no manifest change). Exception process documented; pm-r2-m4 (never-alter vs
   content-pinned distinction) resolved in the module docs.
6. **TY2024 figures** (`tax_tables.rs:115`): every value verified against the fetched primary source
   — §3.15(1) 14,600/29,200/14,600/21,900; §3.15(3) 1,550/1,950 (QSS correctly on the married
   amount); §3.15(2) 1,300/450; §3.02 kiddie $1,300 ⇒ 8615 trigger $2,600; SALT $10k (§164(b)(6),
   pre-OBBBA TY2024, MFS-halving deferred to use site as documented); §402(g) $23,000 (Notice
   2023-75); §904(j) $300 (MFJ doubling at use site). `std_deduction_for` Qss→Mfj via the module-
   private `TaxTable::key` — correct and tested. Non-2024 years fail closed (tested: 2017/2025 →
   `None`). Method-test inline schedules match Rev. Proc. 2023-34 §3.01/§3.03 exactly.
7. **Conventions/idiom:** no float anywhere; all `Decimal` literals; additive-only `mod.rs`
   re-exports; `bin_midpoint`/`worksheet_tax` correctly private; documented-panic indexing consistent
   with `ordinary_for`; clippy clean.

## Gate

**NOT GREEN.** 0 Critical / **4 Important** / 4 Minor. I-1/I-2 are documentation-and-recording fixes
(no behavior change); I-3 is one small test; I-4 is the vendored cross-check or a reviewed re-scope.
Re-review (r2) required after the fold per §2.
