# Independent tax-correctness review — year-aware Form 8949 box fix (C/F → I/L for TY2025+)

Reviewer: independent Fable (did not author the change). Round: r1.
Scope: the diff at `box-fix.diff` plus every surface in the repo that emits or documents a Form 8949 box, verified against the bundled genuine IRS form PDFs.

## Verification performed (evidence base)

1. **Boundary predicate.** `year >= DIGITAL_ASSET_8949_FIRST_YEAR (2025)` at `/scratch/code/bitcoin_tax/crates/btctax-core/src/forms.rs:131`. Rows are filtered to `disposed_at.year() == year` (forms.rs:126), so the box year and the row's inclusion year are the same variable — no cross-year contamination is possible. TY2024 → C/F, TY2025 → I/L, TY2026+ → I/L. No off-by-one: I extracted text from the bundled IRS PDFs — the 2024 revision's Part I has only (A)(B)(C) with C = "not reported to you on Form 1099-B" (forms/2024/f8949.pdf), and the 2025 revision's Part I is (A)(B)(C)(G)(H)(I) with C = "**other than digital asset transactions**" and I = "Short-term **digital asset** transactions not reported to you on Form 1099-DA or Form 1099-B"; Part II is (D)(E)(F)(J)(K)(L). `DIGITAL_ASSET_8949_FIRST_YEAR = 2025` is correct (1099-DA gross-proceeds reporting begins TY2025; the 2025 revision is the first with the G–L grid — confirmed on the form itself, the only authority that matters here).
2. **No wrong box is reachable.** The enum has exactly {C, F, I, L}; the 1099-reported boxes (A/B/D/E, G/H/J/K) have no variants and are constructed nowhere. Both letter-emission sites in the codebase (`crates/btctax-cli/src/render.rs:190-196`, `crates/btctax-tui/src/tabs/forms.rs:28-34`) are exhaustive 4-arm matches (a future variant fails to compile, not silently). The TUI CSV export delegates to the patched `btctax_cli::render::write_form_csvs` (`crates/btctax-tui/src/export.rs`), so it is covered. The text-printed packet (`printed.rs` `Printed8949Row`) prints only columns (d)/(e)/(h) — no box letter. `btctax-tui-edit` and the oracle harness emit no box letter.
3. **fill8949 claim VERIFIED — it was already correct, independently of the enum.** `crates/btctax-forms/src/fill8949.rs:86-96` checks the box from the per-year map (`part.box_field`/`box_on`), never from `row.box_`. The 2025 map (`forms/2025/f8949.map.toml`) targets `c1_1[5]`/on-state `"6"` (Part I) and `c2_1[5]`/`"6"` (Part II) — the 6th checkbox in each group, which per the extracted label order (A,B,C,G,H,I / D,E,F,J,K,L) is Box **I** / Box **L**. The 2024 and 2017 maps target `c1_1[2]`/`"3"` = Box **C** and `c2_1[2]`/`"3"` = Box **F**. Pinned by real read-back tests: `crates/btctax-forms/tests/kats.rs:88-115` (2025: I/L asserted ON, C/F asserted OFF) and `crates/btctax-forms/tests/sp3.rs:48-84` (2024: C on `/3`; 2025 regression: I on `/6`, C off). The 2025 Schedule D placement is also right — the bundled 2025 Schedule D line 3 reads "with Box C **or Box I** checked" and line 10 "Box F **or Box L**" (extracted from forms/2025/schedule_d.pdf).
4. **Core tests pin both directions non-tautologically.** `kat_forms.rs`: `ty2025_st_leg_is_part_i_box_i_and_lt_leg_is_part_ii_box_l` (year 2025 → `Form8949Box::I`/`L` by enum equality) and `pre_2025_st_leg_is_box_c_and_lt_leg_is_box_f` (2024 fixtures, `form_8949(&st, 2024)` → `C`/`F` — verified the assertion tail at kat_forms.rs:149-151). The CSV test (`crates/btctax-cli/tests/export.rs:190-194`) pins "I" at TY2025. The exchange-flag KAT correctly re-pins that `box_needs_review` never mutates the box (stays I, never G/H).
5. **No understatement risk.** The diff changes only the (part, box) tuple selection; part mapping (ST→Part I, LT→Part II) is byte-identical before/after, and proceeds/basis/gain/term/Schedule D totals are untouched. Pure reporting-label change. Suite: 2083/2083 pass, clippy clean.
6. **Design intent honored.** Conservative not-reported default in every year; no auto-assignment of any 1099-reported box; `box_needs_review` semantics unchanged. The CLI PDF-export advisory is exemplary and year-correct (`docs/examples/examples.md:129`: "…would belong on a SEPARATE Form 8949 under **Box G/H/J/K**. This export files EVERY Bitcoin row under **Box I/L**…", from `crates/btctax-forms/src/lib.rs:354-359`).

## Findings

### IMPORTANT-1 — TUI Forms tab advisory still directs a TY2025 filer to the forbidden securities boxes
`crates/btctax-tui/src/tabs/forms.rs:165-168`

The unconditional footnote rendered for **every** year, including 2025:

```rust
"NOTE: Review box C/F — exchange disposals may require A/B/D/E (1099-B/1099-DA)."
```

On a TY2025 view the table above it now (correctly) shows boxes I/L, and this note tells the filer to review "box C/F" and reclassify to **A/B/D/E** — the 1099-B *securities* boxes. Authority: the 2025 i8949 forbids the securities boxes for digital assets ("Do not use box C… Use box I" / "Do not use box F… Use box L"), and a digital-asset disposition actually reported on a 1099-DA belongs in **G/H** (ST) / **J/K** (LT) — which is exactly the stated design intent of `box_needs_review` ("reclassify to G/H/J/K if they actually received a 1099-DA"). A filer following this advisory on a 2025 return would check a box the instructions explicitly prohibit. The stale text has shipped into the **test-enforced** walkthrough goldens showing a "Forms — 2025" pane with the C/F note (`docs/examples-tui-walkthrough/j2/03-forms.txt:36`, `docs/examples-tui-walkthrough/j6/04-forms.txt:37`; enforced by `xtask::examples::tests::walkthrough_console_golden_matches_committed`, so the golden currently *pins the wrong guidance*). The CLI-side advisory got this right — the TUI diverged.

Not graded Critical because no emitted box value or figure is wrong — it is advisory text — but it is a real, user-facing tax-guidance defect inside the exact scope of this fix, on a file the fix touched.

**Fix:** make the footnote year-aware, mirroring the core predicate: pre-2025 → "Review box C/F — exchange disposals may require A/B/D/E (1099-B)"; TY2025+ → "Review box I/L — exchange disposals may require G/H/J/K (1099-DA)". Then regenerate the TUI walkthrough goldens.

### IMPORTANT-2 — the TUI box-letter test is tautological and now documents the pre-fix (tax-wrong) expectation; the TUI's I/L mapping is held by no test
`crates/btctax-tui/src/tabs/tests.rs:1551-1571`

Test F1 builds a **2025** LT disposal, its doc comment says `Expected: 8949 Part "LT" + Box "F" appear`, and it asserts `buffer_has(&buf, "F")`. `buffer_has` (tests.rs:76-84) is a whole-buffer substring search — "F" matches "Forms", "Form 8283", and the C/F footnote, so the assertion passes vacuously while the actual rendered box is now "L". Two defects: (a) the only test that claims to pin the TUI box letter pins nothing — the TUI's `form8949_box_tag` is a *duplicate* mapping not covered by the core KATs, and no golden shows an 8949 row with a box letter (the j2 forms golden has zero 8949 rows), so reverting the TUI's `I`/`L` arms to `"C"`/`"F"` would pass the entire suite; (b) the test's stated expectation asserts the very behavior this fix removed, on a 2025 fixture. This is the project's named untested-guard failure mode: the fix on this surface isn't held by any test.

**Fix:** pin the mapping directly — a unit test asserting `form8949_box_tag` over all four variants (make it `pub(crate)`/`#[cfg(test)]`-reachable), and/or make F1 assert the box **cell** unambiguously (e.g. the rendered 8949 row line contains the `L` box column next to the `LT` part tag, not a bare one-letter buffer search). Update the doc comment to Box L; add the 2024→F direction cheaply alongside.

### MINOR-1 — `Form8949Row` field docs contradict the fix and repeat the A/B/D/E misdirection
`crates/btctax-core/src/forms.rs:63-68`

`box_`: "The conservative C (ST) / F (LT) 'not reported on a 1099-B' default (D4)" — stale (the enum doc immediately above was updated; the field doc was not). `box_needs_review`: "…the C/F default should be reviewed and reclassified to **A/B (ST) or D/E (LT)** if a 1099-B was issued" — for TY2025+ this is the same forbidden-box misdirection as IMPORTANT-1, in the API docs of the very struct the fix touched. Fix: reword both to the year-aware pairs (C/F↔A/B/D/E pre-2025; I/L↔G/H/J/K from 2025).

### MINOR-2 — stale cross-file comments perpetuating the pre-fix model (aggregate)
- `crates/btctax-forms/tests/common/mod.rs:29`: "The core taxonomy is C/F; the forms crate must map to Box I/L regardless" — now false (core is year-aware); the fixtures there and at `crates/btctax-forms/tests/full_return_forms.rs:1993, 2311-2313` and `crates/btctax-core/src/tax/printed.rs:1814` still hand-set `Form8949Box::C` on 2025-year rows (inert — the fill layer ignores `row.box_` — but the comment invites a future author to rely on "map regardless" instead of the year-aware core).
- `crates/btctax-forms/src/fill8949.rs:6`: "the core `Form8949Box::{C,F}` taxonomy is not reused" — the set is now {C,F,I,L}.
- `crates/btctax-core/src/tax/printed.rs:60-62` "Part I (short-term, Box C)… 'with Box C/F checked'" and `crates/btctax-core/src/tax/packet.rs:432` "with Box C/F checked" — year-dependent now (the 2025 Schedule D text is "Box C or Box I" / "Box F or Box L").

Fix: one doc-consistency sweep; no behavior change.

### NIT-1 — `pre_2025_st_leg_is_box_c_and_lt_leg_is_box_f` (kat_forms.rs) drops the `rows.len() == 2` assertion its 2025 twin has (the `find().unwrap()`s still guard). No test exercises a year > 2025 (the `>=` makes it trivial, but a 2026 KAT would pin the "and later" half of the contract for free).

## Verdict

The tax core of the fix is sound and well-pinned: the boundary is exactly right, verified against the actual IRS form revisions bundled in-repo; no wrong box is constructible; the filled-PDF layer was independently correct all along (map-driven, read-back-pinned, both eras); gain/term math is untouched. What blocks is the fix's own blast radius in the TUI: guidance text steering a 2025 filer to prohibited securities boxes (shipped into pinned goldens), and a vacuous test that leaves the TUI's I/L rendering unguarded while documenting the old wrong behavior.

**NOT-GREEN — 0 Critical / 2 Important (plus 2 Minor, 1 Nit).** Suite itself is green (2083/2083 + clippy); the two Importants are the gate.
