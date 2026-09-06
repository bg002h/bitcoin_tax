# Wave 2 — #13 (authority ratchet discards the YEAR) + R16 (`EMITTED_FORMS` omits `f8995a`)

**Agent:** wave-2 fixer. **File owned and touched:** `crates/xtask/src/cite_check.rs` (only).
**Date:** 2026-09-05. **Status:** both fixed, by construction. Five planted defects observed RED.

---

## 1. What was wrong — measured, not inferred

### #13 — coverage was keyed on the form, with `FormAuthority::year` thrown away

`authority_coverage_may_only_improve` built `archived` as `BTreeSet<&str>` of `f.form`. `FORMS` holds
exactly one row, `(f1040s1a, 2025)`. So the obligation "TY2026 Schedule 1-A" would have been reported
as **already covered** by the TY2025 archive, and `cite-check` would have gone on checking a
retargeted document against `schedule_1a_2025_*.txt` while printing coverage. The TY2026 draft of that
same form keeps 10 of the TY2025 revision's 219 AcroForm field names (`TY2026_PORT_REPORT.md:255`,
`:510`), so "same form, different year" is not a detail.

The old excuse list `AUTHORITY_NOT_YET_ARCHIVED` was likewise form-keyed, so a prior-year excuse would
also have discharged a later-year obligation.

### R16 — the hand-list did not match the emitting surface

| list | count | contents |
|---|---|---|
| old `EMITTED_FORMS` (`:678`) | **16** basenames | omits `f1040s1` **and** `f8995a`; includes `f1040s1a` |
| `btctax-forms/src/packet.rs` `push("…")` | **17** stems | pushes `f1040s1` (`:103`) and `f8995a` (`:193`); never pushes `f1040s1a` |
| `crates/btctax-forms/forms/<year>/*.pdf` + `*.map.toml` | **18** stems / **37** `(form, year)` pairs | the real surface |

So the omission was **two** forms, not one. `f1040s1` was missing as well as `f8995a`, and both are
pushed into the filed packet. The ratchet passed on both by finding nothing.

Measured obligation set (`ls crates/btctax-forms/forms/*/`): 2017 → 5 stems, 2024 → 17, 2025 → 16;
**37 `(form, year)` pairs, 18 distinct stems.** No stem has a blank without a map or a map without a
blank. The old ratchet tracked 16 form-level obligations against a 37-pair surface.

### `f8615` — checked, and it is **not** the same shape

Asked in the brief. Measured three ways: no `crates/btctax-forms/forms/*/f8615.pdf`, no
`f8615.map.toml`, no `push("f8615"…)` in `packet.rs`, no `form8615.rs` emitter. btctax **refuses** the
§1(g) case and files a Form 8275 disclosure instead — `btctax-core/src/tax/form8275.rs:218`
(*"Tax on unearned income of a child — Form 8615 not filed"*) and `:264`. Its absence from the
obligation set is therefore **correct**, not an omission. `design/forms/2025/f8615--2025.pdf` is
archived authority for a form we do not print. The measurement is pinned by an assertion (below) that
reds if btctax ever does embed an 8615 template.

---

## 2. What I changed

All in `crates/xtask/src/cite_check.rs`. `EMITTED_FORMS` is **deleted**; nothing outside this file
referenced it (grep over `crates/`, `design/`, `docs/` — only prose mentions in prior agent reports).

| `:line` | what |
|---|---|
| `759` | `pub type FormYear = (String, i32)` — the obligation unit is now a pair |
| `770` | `STEM_ALIASES` — the only two template stems that are not IRS basenames (`schedule_d`→`f1040sd`, `schedule_se`→`f1040sse`) |
| `774` | `irs_basename()` — total; a stem that is neither an alias nor `f`+digit is an **Err**, never a skip |
| `802` | `emitted_form_years()` — **derives** the surface from `crates/btctax-forms/forms/<year>/<stem>.pdf` + `.map.toml`. Fails on a blank-without-map, on an untranslatable stem, and on a vacuous walk (zero pairs, zero year dirs, or no `f1040`) |
| `883` | `AUTHORITY_NOT_YET_ARCHIVED: &[(&str, &[i32])]` — 17 rows, 36 pairs, **no wildcard and no "all supported years" sentinel** (a sentinel would silently pre-excuse TY2026, which is the hole being closed) |
| `917` | `archived_form_years()` — a registry row counts as coverage **only if its committed extract is on disk**; rows whose fixture is missing come back in a second list and fail by name |
| `952`/`964` | `CoverageVerdict` + `adjudicate_coverage()` — **pure**, so the ratchet's own logic is testable against planted inputs rather than only against today's repo |
| `421`/`453` | `design_year_dirs()` + `unchecked_design_years()` — see §4, the second year hole |
| `run()` | prints `(form, year)` coverage and now **returns Err** on unaccounted / stale / phantom, and on an unread design year. A reporting-only coverage line is an instrument that cannot fail |

Current state, from `cargo run -p xtask -- cite-check`:

```
cite-check: OK — 51 quotations, all verbatim.
cite-check: authority archived + extracted for 1/37 emitted (form, year) pairs [f1040s1a--2025]; 36 excused, 0 unaccounted
```

**The year transition now fails closed.** Dropping `crates/btctax-forms/forms/2026/` into the tree
creates ~17 new `(form, year)` obligations, and every one is unaccounted until it is archived or
consciously excused with its year typed out.

---

## 3. Which test reds for which planted defect

Five plants, each applied, run, observed RED, and reverted. `cite_check.rs` was restored from a byte
identical backup after each (no git was run).

| # | planted defect | test that reds | observed failure text |
|---|---|---|---|
| **1** | **the original #13**: `adjudicate_coverage` keyed on the form, year discarded | `a_prior_year_archive_does_not_discharge_a_new_year_obligation` (`:1104`) | *"a TY2025 archive discharged a TY2026 obligation — the coverage key has lost the YEAR"* |
| **2** | **the original R16**: `emitted_form_years()` skips `f8995a`, re-creating the hand-list's omission | `the_emitting_surface_is_derived_and_carries_the_year` (`:1147`) **and** `authority_coverage_may_only_improve` (phantom excuse) | *"f8995a is pushed by btctax-forms/src/packet.rs but is not in the derived emitting surface"* |
| **3** | `FORMS[0].extract_stem` → `"schedule_1a_2026"`: registry claims an extract that is not on disk | `authority_coverage_may_only_improve` (`:1050`) | *"1 registry row(s) claim an extracted authority that is NOT on disk … f1040s1a--2025: …/schedule_1a_2026_form.txt, …_instructions.txt"* |
| **4** | `emitted_form_years()` skips year 2017: a **supported** year that generates no obligation | `the_template_years_are_exactly_the_supported_years` (`:1197`) | *"have drifted … left: {2024, 2025} right: {2017, 2024, 2025}"* |
| **5** | a real `design/ty2026/` directory created on disk (then removed) | `every_design_year_on_disk_has_a_document_in_the_checked_set` (`:1216`) | *"design year(s) [2026] have a corpus on disk that cite-check never reads"* |

Plant 1 is the load-bearing one and it is worth being explicit about **why a planted-input test was
required**: under plant 1, `authority_coverage_may_only_improve` still **PASSED**. The live repo has
one archived row and no year collision, so the real-state ratchet cannot see the year bug at all. A
test that only exercises today's repo would have certified the defect as fixed.

Each of plants 1, 3, 4 and 5 also reds a *different* test than the others — no single test is carrying
all five.

### Guarding the guards

- `emitted_form_years()` refuses to return a set that is empty, has no year directories, or lacks
  `f1040` (which `packet.rs` pushes unconditionally). A derivation that finds nothing would make every
  check downstream pass vacuously.
- `every_design_year_on_disk_has_a_document_in_the_checked_set` asserts `design_year_dirs()` is
  non-empty before asserting anything about it, and carries its own planted-input half so it cannot be
  satisfied by an empty list.
- `the_emitting_surface_…` asserts `irs_basename("dependents_statement").is_err()` — an untranslatable
  stem must error rather than be dropped — and that no raw `schedule_*` stem leaks into the obligation
  set (it would never match an authority filename and would read as a permanent gap).

### Gates

`cargo nextest run -p xtask` → **110 passed, 1 skipped**. `cargo fmt -p xtask -- --check` and
`cargo clippy -p xtask --all-targets` → **zero** diffs/warnings in `cite_check.rs`. (Both report a
pre-existing, unrelated issue in `crates/xtask/src/authority_manifest.rs` — an unformatted closure at
`:1645` and a `bool::then` lint at `:1613`. Not mine, not touched.)

---

## 4. Found, and FIXED although not in the brief

**The quotation pass is pinned to TY2025 by hand, and that is the other half of #13's consequence.**

`schedule_1a_docs()` (`:406`) names `design/ty2025/SPEC_schedule_1a.md`,
`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`, and the two `schedule_1a_2025_*` extracts as
literals. Making the *ratchet* year-aware stops the coverage **claim** from being false; it does
nothing about the checker's document→extract pairing, which still reads last year's corpus. Standing
up `design/ty2026/` would have produced a corpus `cite-check` silently does not read while it went on
printing *"51 quotations, all verbatim."*

I did **not** rewrite the pairing — `design/ty2026/` does not exist yet, and inventing a doc→form-year
convention now would be speculative. Instead the omission is made **loud**: `design_year_dirs()` walks
`design/ty<YYYY>` off the filesystem and `unchecked_design_years()` names any year with no checked
document. Test at `:1216`, plant #5 above. The real fix (per-`(form, year)` document and extract paths
carried on the `FormAuthority` row) is the follow-up.

---

## 5. Found, NOT fixed — for whoever owns the TY2026 archive

1. **`extract()` cannot name a DRAFT authority.** `:562`ff (`:574`, `:595`) builds
   `design/forms/{year}/{form}--{year}.pdf`. Every TY2026 authority on disk is
   `f…--2026-DRAFT.pdf` (all 15 of them). The moment a 2026 row enters `FORMS`,
   `cargo run -p xtask -- extract-schedule-1a` errors *"cannot read
   design/forms/2026/f1040s1a--2026.pdf"*. It fails **closed** and loudly, so this is not a silent
   hole — but the transition needs a decision I declined to invent: is a DRAFT an admissible authority,
   and how does the registry record that it is one? A `draft: bool` on `FormAuthority` that propagates
   into the extract header (which already pins a sha256) is the obvious shape.
2. **`f1040s1a` has a template and a 100%-mapped field map for TY2025 but `packet.rs` never pushes
   it.** It is an obligation under the derived surface (correctly — a map *is* a transcription) and it
   is the one pair that is archived, so nothing reds. Flagging it because it means the 2025 packet
   cannot currently emit Schedule 1-A at all, which is a product fact, not an instrument fact, and
   outside my file.
3. **`f8995a` and `f8275` are TY2024-only templates** (no 2025 blank, no 2026). They are excused for
   2024 alone, so the excuse list will *not* red when TY2025/TY2026 versions are added — the new
   `(form, year)` pairs simply arrive unaccounted, which is the intended behaviour. Noting it so the
   count change is not read as a regression. This is consistent with `TY2026_PORT_REPORT.md:403`:
   Form 8995-A is unmeasured for 2025 and 2026, and `f8995a--2025` is the prior side of a TY2026 delta
   that does not exist.
4. **The four-hand-lists problem is now three.** `EMITTED_FORMS` is gone, but `pdf.rs` consts,
   `map.rs` consts and `tests/common/mod.rs:16 CENSUS_KEYS` (17, map spelling) remain independent.
   `CENSUS_KEYS` is a test-module `const` in an integration test and is not importable from `xtask`;
   collapsing it onto the derived surface needs an edit in `btctax-forms`, which I do not own.
   `emitted_form_years()` is the derivation those three should be folded onto.
