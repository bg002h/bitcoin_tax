# IMPL review — Full Return Phase 0 (r2, Fable, independent re-review)

- **Scope:** fold commit `8d73673` ("fold Fable code review r1") + the tree at HEAD; confirms
  resolution of r1's 0C/4I/4M and scans for regression.
- **Prior:** `reviews/IMPL-P0-fable-review-r1.md` (0 Critical / 4 Important / 4 Minor).
- **Reviewer:** Fable (independent; author was a different model). Date: 2026-07-12.
- **Validation run:** `cargo test -p btctax-core --lib` → **116 passed** (114 + the 2 new tests);
  `cargo test -p btctax-adapters --lib` → **58 passed**; `cargo clippy -p btctax-core -p
  btctax-adapters --lib --tests -- -D warnings` → clean. `frozen_engine_files_are_unchanged` green
  at HEAD; the fold touched **no frozen file** (`git diff 8d73673^ 8d73673 --name-only`:
  tax_tables.rs, conventions.rs, method.rs, tables.rs, FOLLOWUPS.md, and the r1 review — which is
  persisted verbatim, 166 lines, unmodified).
- **Verdict: GREEN — 0 Critical / 0 Important / 3 Minor.** All four Importants resolved (I-4 by an
  adjudicated, recorded deferral — ruling below). **Phase 0 passes the gate; ready for Phase 1.**

---

## Resolution of r1 findings

### I-1 — RESOLVED (false se.rs/TaxTable justification)

Both doc comments corrected and the replacement claims verified **true** against current source:

- `crates/btctax-core/src/tax/tables.rs:218-224` — the false "a FROZEN file (`tax/se.rs`)
  constructs `TaxTable` by struct literal" claim is gone; the new text states the true grounds and
  explicitly retracts the old one ("This does NOT rely on any frozen-file constraint (`se.rs` only
  *calls* the unfrozen `synthetic_table`, so `TaxTable` could technically gain a field)").
- `crates/btctax-adapters/src/tax_tables.rs:89-92` — same correction ("separate … by design —
  published-crate-API stability + v1-only fail-closed gating").
- **Truth-check of the retraction:** `se.rs:205-206` is `fn tbl() -> TaxTable {
  synthetic_table(2025) }` — a *call*; se.rs otherwise only imports the type (`se.rs:16`) and takes
  `&TaxTable` (`se.rs:103`). Repo-wide grep for "struct literal" finds no surviving copy of the
  false claim (only an unrelated `btctax-tui/src/lib.rs:83` comment).
- **Truth-check of the new justification:** (1) `TaxTable` (`tables.rs:52-82`) is an
  **all-pub-fields** struct in btctax-core, which is **published on crates.io** (all 7 crates
  published since v0.2.0) — a new field breaks downstream literal constructors, so keeping the
  full-return params out of it is a real API-stability concern. (2) Fail-closed gating is real:
  `BundledFullReturnTables::load()` bundles **2024 only** (`tax_tables.rs:98-104`),
  `full_return_for` is a plain map `get` (no fallback), and the trait doc (`tables.rs:255-259`)
  pins `None` ⇒ caller fails closed (`NotComputable`). Sound.
- **Deviation recorded:** `design/full-return/FOLLOWUPS.md` **p0-taxtable-deviation** (RECORDED —
  no action) accurately captures the SPEC-§8 / plan-task-5 deviation, the true rationale, and the
  retraction of the wrong comment.

### I-2 — RESOLVED (§3.16 → §3.15), primary source re-verified this round

All three code sites corrected: `tax_tables.rs:113` (§3.15), `:124` (§3.15(3)), `:126` (§3.15(2)).
I re-fetched `irs.gov/pub/irs-drop/rp-23-34.pdf` **in this r2** and extracted the text:

- TOC: "**.15 Standard Deduction … 63**"; "**.16 Cafeteria Plans … 125**". §3.15 is confirmed the
  TY2024 standard-deduction section; §3.16 is confirmed Cafeteria Plans (§125(i), $3,200).
- §3.15(1): MFJ/QSS **$29,200**, HoH **$21,900**, Single **$14,600**, MFS **$14,600** — match
  `ty2024_full_return()` exactly. §3.15(2): dependent = greater of **$1,300** or **$450** + earned
  — matches `dependent_std_floor`/`dependent_std_earned_addon` and the `:126` sub-cite. §3.15(3):
  **$1,550** aged/blind, **$1,950** unmarried-not-surviving-spouse — matches and the `:124`
  sub-cite is right.

Residue (Minor, below): r1's fix line also asked that the §3.16 occurrence in the persisted audit
doc be noted in FOLLOWUPS; that note was not added.

### I-3 — RESOLVED (KAT-9 cross-foot test)

`round_dollar_cross_foots_printed_lines` (`crates/btctax-core/src/conventions.rs:140-146`)
exists, passes, and **genuinely discriminates**: `round_dollar(271.50) = 272` and
`round_dollar(499.50) = 500` (half-up), cross-foot **772**; the wrong composition
`round_dollar(271.50 + 499.50) = round_dollar(771.00) = 771`; the test asserts both values *and*
the inequality. The doc comment correctly scopes this as the P0 arithmetic half, with the real-8959
re-assertion deferred to P4/P6 per the plan split (pm-r2-m2).

### I-4 — RULING: deferral to Phase 7 is **ACCEPTED**

The CC0 PSL Tax-Calculator cross-check is deferred to P7 with FOLLOWUP **p0-cc0-crosscheck**
(`design/full-return/FOLLOWUPS.md:41-46`), which names the skipped acceptance line, the target
phase, and the justification. My ruling — the deferral is an acceptable resolution:

1. **The check's purpose is already fulfilled at higher authority.** Its purpose is independent
   verification of transcribed param values. r1 verified every bundled figure against the fetched
   primary source, and this r2 **re-fetched Rev. Proc. 2023-34 and re-confirmed all §3.15 values**
   (plus the 5 QDCGT fixtures re-derived cent-exact in r1). PSL Tax-Calculator is a *secondary*
   source; diffing against it adds redundancy, not authority.
2. **No silent-drift window.** `ty2024_full_return_params_bundled` pins every `FullReturnParams`
   field (all 9 scalars + 4 std-deduction cells) to exact values; the params are static between P0
   and P7, so no intermediate phase's error could be masked by deferring the oracle diff.
3. **Architecturally apt target.** P7 is where the independent-oracle layer lives (plan line
   208-212: golden returns + tenforty/PolicyEngine observe-only); consolidating oracle work there
   is coherent, not evasive.
4. **Process satisfied.** r1-I4 demanded that a re-scope "go through the §2 review loop and be
   recorded." The FOLLOWUP is the record; this r2 is the review. The plan's P0 acceptance text
   (`IMPLEMENTATION_PLAN_full_return.md:36,76`) remains uncorrected, which is consistent with the
   project's established FOLLOWUPS-erratum convention (cf. **spec-s8-kat3-mod25**); annotate at the
   next plan touch.

**Condition:** the P7 review must treat a missing vendored CC0 slice + diff test as **Important**
at that gate; and any edit to the bundled TY2024 values before P7 (necessarily loud — they are
test-pinned) re-opens this ruling.

### r1 Minors

- **M-1 RESOLVED** — the mod-25 sweep (`tax_tables.rs:686-707`) no longer hardcodes the year list:
  it derives coverage from the bundled map via `table_for` over 2000..=2100 (`table_for` is a plain
  `BTreeMap::get`, no fallback — exactly the bundled years are swept, future bundled years
  auto-included) and guards with `assert!(checked >= 4)`.
- **M-2 RESOLVED** — `dependent_std_earned_addon` = 450 now asserted (`tax_tables.rs:723`); every
  `FullReturnParams` field is pinned.
- **M-3 RESOLVED** — `regular_tax_table_and_tcw` (`method.rs:252-257`) directly exercises the pub
  API: Table path 58,000 → 7,819; TCW path 120,000 → 21,843 (21,842.50 half-up); 0 → 0. Correct.
- **M-4 NOT ADDRESSED** — carried forward as r2-M2 below.

---

## Findings (r2)

## Critical

None.

## Important

None.

## Minor

- **r2-M1** — I-2's prescribed FOLLOWUPS note for the §3.16 occurrences in *persisted* review
  artifacts was not added. Those docs are verbatim-persisted and must not be retro-edited, so the
  erratum belongs in FOLLOWUPS: `reviews/DESIGN-audit-fold-confirm.md:29` (flagged in r1) and —
  found this round — `reviews/DESIGN-fable-audit-final.md:56,77`. One line each.
- **r2-M2** (= r1 M-4, carried) — `crates/btctax-core/src/tax/method.rs:70-72,84-85` still label
  the QDCGT internals with deep/01's compressed numbering ("L5", "L10") unmarked, while L22-L25
  use the official sheet's numbering. Math verified correct (r1); add a "(deep/01 numbering)" tag
  so a future reader doesn't "correct" right code against the wrong line map.
- **r2-M3** (process; outside the P0 diff but must be on the record) — commit `376594b`
  ("impl(full-return P1): ReturnInputs data model", 06:25) landed **before** the P0 fold
  (`8d73673`, 06:40) while r1's 4 Importants were open — Phase-1 work proceeded past a red
  Phase-0 gate (STANDARD_WORKFLOW: "no work proceeds past a gate while a blocking finding is
  open"; the commit message itself notes "Phase 0 code review still in flight"). Verified
  non-contaminating: additive-only (new `tax/return_inputs.rs` + a one-line `mod.rs` re-export),
  disjoint from every r1 finding, frozen guard green at HEAD — so it does not affect P0's gate.
  Ranked Minor *here* because the P0 artifact is unaffected; two hard directions attach: (a) the
  **P1 r1 review must include `376594b` in scope**, and (b) do not repeat the pattern — the gate
  order is the contract.

---

## Regression scan

- **Suites:** core **116** / adapters **58** / 0 failed (116 = r1's 114 + the two new tests; the
  adapter count unchanged because M-1/M-2 amended existing tests). Clippy clean under
  `-D warnings`.
- **Frozen guard:** `frozen_engine_files_are_unchanged` green at HEAD; fold file list contains no
  frozen file; the r1 review artifact was added verbatim in the fold commit.
- **Behavior:** the fold's non-test changes are doc-comments only (tables.rs, tax_tables.rs
  headers + cite fixes); no production code path changed. Spot re-checks of the r1 clean list
  (worksheet fixtures, sweep, params pins) all still green via the suites.

## Gate

**GREEN.** 0 Critical / 0 Important / 3 Minor (two recording/labeling nits + one process note
directed at the P1 review). I-4's deferral to P7 is **accepted** under the stated condition.
**Phase 0 passes the §2 gate — proceed to Phase 1.**
