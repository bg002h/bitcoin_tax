# Whole-diff review (Phase E) — fix/irs-8283-multi-donee — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Diff `main (d43d294)..6b9b227` — production change is `crates/btctax-forms/src/form8283.rs` ONLY (+ KATs in
`sp2.rs`, the spec, and the 2 R0 reviews). Contract: `design/SPEC_irs_8283_multi_donee.md` (R0-GREEN, 2 rounds,
Opus). Resolves the `irs-8283-multi-donee-identity` FOLLOWUP + a second latent bug R0 found (page-2-blank
identity on an overflowing single donee).

## Verified by KAT (my own run — sp2: 36 passed / 0 failed)
- **★ the fix's core** `form_8283_multi_donee_one_copy_per_donee` — 2 donees ⇒ 2 copies, each its OWN Part V
  donee + Part IV appraiser (RED pre-fix: one form named only donee A).
- **[C1] `form_8283_second_donee_without_details_still_separate`** — donee B carrier has `details: None` (only
  `section` set) ⇒ still separate; NOT absorbed onto A's named form. Confirms the `row.section.is_some()`
  carrier signal (not `details.is_some()`).
- **[global group-by] `form_8283_interleaved_same_donee_groups_globally`** (A,B,A ⇒ A's copy has both A
  donations) — proves grouping is global-by-identity, not an adjacency run.
- **[I3] `form_8283_same_donee_different_appraiser_splits`** — split-on-difference (correct Part IV each).
- **[★ byte-identity] `form_8283_single_donee_unchanged`** + the existing `form_8283_is_byte_deterministic`
  (SHA unchanged) — the common single-donee path is byte-identical (total-1-copy returns `fill_one` directly,
  bypassing `merge_copies`).
- **[I2] `form_8283_section_a_multi_donee_stays_one_form`** — Section A pagination unchanged (per-row donee
  column; grouping is Section-B-only).
- **[2nd bug cured] `form_8283_one_donee_overflow_carries_identity_on_both_pages`** — identity on BOTH pages.
- **[m2] `form_8283_multi_group_with_overflow_global_rename`** — 3 copies from 2 groups, globally-unique fields.
- **[fail-closed unchanged] `form_8283_multi_donee_per_copy_readback_fault_injected_is_red`** — the per-copy
  geometric read-back still fails closed on a swapped map.

## Scope / suite
`btctax-forms` only (no core/map/PDF/engine change; the read-back oracle untouched). Full workspace `cargo test
--locked` = 0 failed (implementer; my close-out re-running); clippy -D + fmt clean; isolation OK. PATCH-level
behavior fix (a new correct output for multi-donee; single-donee byte-identical) — ships inside 0.3.0.

**SHIP — multi-donee years now get one correct Form 8283 per donee (right Part IV/V identity), single-donee is
byte-identical, and the read-back still fails closed. Closes FOLLOWUP `irs-8283-multi-donee-identity`.**
