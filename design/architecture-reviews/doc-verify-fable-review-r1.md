# Fact-check review: ARCHITECTURE.md + CONSTELLATION.md vs source @ 0.7.0

## Scope of verification performed

- Rebuilt the full `btctax-*` dependency edge set from all 12 `crates/*/Cargo.toml` manifests.
- Spot-checked ~28 `file:line` citations across all four layers.
- Read the load-bearing sources behind every named invariant (`project/mod.rs`, `resolve.rs`, `fold.rs`, `state.rs`, `conventions.rs`, `frozen_guard.rs`, `classifier.rs`, `questions.rs`, `verify.rs`, `capture.rs`, `check_isolation.rs`, `input_form_store.rs`, `main.rs` of both binaries, store internals, Makefile, ci.yml).

## What checks out (the load-bearing claims)

- **Dependency graph: exact.** Every edge in ARCHITECTURE's "at a glance" block and CONSTELLATION's layering matches the manifests precisely: core and store have zero `btctax-*` deps (mutually independent — core carries only `rusqlite` for persistence glue); adapters/forms/input-form → core only; cli → core+store+adapters+forms; tui → cli+store+core+adapters; tui-edit → tui+cli+core+input-form+store+adapters; update-prices → adapters only; oracle-harness → core+forms (`publish=false`); xtask → cli+update-prices+forms (`publish=false`); `btctax` stub has an empty `[dependencies]` and a 14-line lib.rs. No dev-dep introduces a new edge (cli dev-deps adapters, already a normal dep). Clean DAG confirmed. No `[workspace.dependencies]` table exists (crates even carry "NO [workspace.dependencies] table exists" comments, e.g. `crates/btctax-tui/Cargo.toml:26`).
- **Net isolation.** `crates/xtask/src/check_isolation.rs:11-19` gates exactly the six tax crates named (cli, tui, tui-edit, core, adapters, forms), forbids `ureq`/`rustls`, positive control on update-prices — as both docs state.
- **Key structural claims all true**: `project` = resolve+fold (`project/mod.rs:63`); `would_conflict` (`mod.rs:107`) genuinely runs the real projection twice and diffs the `DecisionConflict` set, pseudo forced off — doc comment says "DEFINITIONALLY the resolver"; `resolve_election` (`resolve.rs:175`) is the sole two-tier resolver with HIFO-by-`None` fall-through (`fold.rs:33 applicable_method`); `fold_event` (`fold.rs:554`) is the one dispatcher shared by the fold, `pools_before`, `state_as_of`, and `transition::universal_snapshot:32`; `frozen_guard.rs` pins `tax/types.rs`+`tax/compute.rs` by SHA-256; the classifier has `#![deny(unused_variables)]` (`classifier.rs:21`) and no `..` in any destructure; `verify.rs:147 verify_8949` / `:337 verify_flat` are the map-independent geometry oracles incl. the Digital-Asset oracle (`:432`) and character-counted `/MaxLen` (`:409`); `to_golden` (`btctax-tui/src/capture.rs:29`) emits exactly the glyph-grid + style-run format described; the I-11 finalize guard is real (`input_form_store.rs:280-293`, `NoTables` per-year); core's only I/O is `persistence.rs` (append_import_batch `:172`, append_decision `:238`).
- **Counts and numbers that verify exactly**: 13 decision variants / 6 imported / 1 system; exit codes 0/1/2; `render.rs` 3,984 lines; `reconcile.rs` 1,742; tui-edit `main.rs` 25,831 (~26k); `production_now_utc_lines:14103` and `capture_edit_frame:14195` dead-on; four committed goldens; J1–J9 all exist (`xtask/src/examples.rs:303-311`); crypto tables 2017/2024/2025/2026 and full-return TY2024-only; no `serde_json` in btctax-forms; `make check` structure; the 3-OS + clippy/fmt/msrv-1.88/net-isolation/pii-scan/advisory-examples CI; twelve §9A sections; the two rounding regimes; Hard/Advisory split (`state.rs:80-102`); the cross-type overlap guard (`resolve.rs:910-917`); store internals.

## Findings

**CRITICAL — C1. CONSTELLATION cites a function that does not exist: `stamp_draft_watermark`.**
The only public function is `stamp_draft` — `crates/btctax-forms/src/watermark.rs:21`. Dead reference for an LLM using this map as a grep index. Fix: `stamp_draft`.

**IMPORTANT — I1. CONSTELLATION's per-form module glob expands to three non-existent files.**
`form82{83,95,59,60}` → form8283/form8295/form8259/form8260 — only the first exists. Real files: `form8283.rs`, `form8959.rs`, `form8960.rs`, `form8995.rs`. Fix: list the four names.

**MINOR — M1.** CONSTELLATION's `FORM_QUESTIONS` line ~`:572` points past EOF (file is 392 lines); registry is at `questions.rs:80`. Fix: ~`:80`.

**MINOR — M2.** ARCHITECTURE SEE ALSO implies `LIMITATIONS.md` is a repo-root sibling; it lives at `crates/btctax-cli/LIMITATIONS.md`. Fix: cite the path (or the `btctax limitations` command).

**NIT — N1.** ARCHITECTURE net-boundary wording: the positive control asserts only `ureq`'s presence, not rustls (`check_isolation.rs:36-42`). Over-symmetric.

**NIT — N2.** ARCHITECTURE's Hard/Advisory parentheticals read as exhaustive but each omits three kinds (`state.rs:80-102`). Suggest "among others".

**NIT — N3.** CONSTELLATION dispatch-ladder counts ("22 modal → 21 flow", "~26 flows"): 22 modal matches; flow is ~23. Off by ~2. Recount or soften.

## VERDICT

**Not green as written — 1 Critical, 1 Important to fix** (C1 `stamp_draft_watermark` → `stamp_draft`; I1 the `form82{...}` glob), plus two Minors and three Nits. Everything else — the dependency graph, the four layers, the mutual core/store independence, the single-net-crate claim, all sampled invariants, and essentially every other line citation — is factually accurate against current source, several to the exact line.

---
FOLD (2026-07-19): all seven findings corrected in both docs; C1/I1/M1/M2 fixes
re-verified against source (`stamp_draft` @ watermark.rs:21; form8283/8959/8960/8995.rs
exist; FORM_QUESTIONS @ questions.rs:80; LIMITATIONS.md @ crates/btctax-cli/). N1/N2/N3
softened.
