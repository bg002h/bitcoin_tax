# Phase 2 — J1 (single buyer) walkthrough review r1 — GREEN

_Reviewer: Fable (independent). Scope: the first Phase-2 journey J1 (commit `a596a12`), the template the
other 7 journeys copy. Persisted verbatim before folding the Minors/Nits, per STANDARD_WORKFLOW §2._

**VERDICT: GREEN — 0 Critical / 0 Important** (2 Minor, 2 Nit). J1 is a sound template: all figures shown to the filer are correct and cross-consistent with the shipped CLI report, every gate re-captures fresh and was mutation-proven red-capable, and both regens are byte-stable.

## Evidence run (read-only; tree left clean, `git status --porcelain` empty)

- **Frames byte-stable:** `cargo test -p btctax-tui --lib emit_btctax_tui_walkthrough_goldens -- --ignored` → `git diff` empty (J8 + all 3 J1 frames re-emitted identically).
- **Console byte-stable:** `cargo test -p xtask --bins emit_walkthrough_console_golden -- --ignored` → `git diff` empty (both J8 and J1 transcripts).
- **Gates green, then red under mutation, then restored:** dropping `"j1/03-tax"` from `WALKTHROUGH_VIEWER_STEMS` reds the stem-set assert (tests.rs:1009); deleting the J1 `FRAME 02-disposals.txt` manifest line reds the bijection (examples.rs:1589); appending one byte to `j1/00-setup.console.md` reds the console gate with the J1 path named — the refactored loop genuinely asserts both journeys.
- **Whole suite:** `make check` green — 2071/2071 passed + clippy (caveat per project memory: this is nextest+clippy, not the CI-only jobs).
- **`make tui-walkthrough`** assembles J1 (Makefile:97 globs `*/manifest.txt`) and emits a valid PDF — the J1 roff PROSE is well-formed.

## Tax-fact verification (all correct)

From `J1_CSV` (`crates/btctax-cli/src/testonly.rs:16-19`): buy 0.10 BTC, total-incl-fees $8,450; sell 0.02 BTC, subtotal $1,350 − $10 fee = $1,340 proceeds. Basis sold = 0.02/0.10 × 8,450 = **$1,690**; gain = 1,340 − 1,690 = **−$350** (a loss — proceeds < basis, correctly stated); remaining basis 8,450 − 1,690 = **$6,760** on 0.08 BTC. Held 2025-03-01 → 2025-06-15 = 106 days → **short-term** ✓. Tax: $100k single is inside the 2025 22% bracket, so −350 × 0.22 = **−$77** exactly; §1211 deduction 350 (< $3,000 cap) with zero carryforward ✓; NIIT false (MAGI 100k < 200k) ✓; LTCG marginal 0.15 ✓. Conservation line: 10,000,000 = 2,000,000 + 8,000,000 sats ✓. `manifest.txt`'s stated figures match the frames verbatim, and the frames match the shipped `docs/examples/examples.md` J1 report (−350/−77/0.22/0.15) line for line. `seed_j1_with_profile` (testonly.rs:224-245) builds the exact profile the transcript's `tax-profile` flags produce (CLI defaults all optional fields to 0 — main.rs:962-1005), so the viewer state provably matches the console session.

## Determinism

No residual nondeterminism found: 120x40 `TestBackend` (tests.rs:127), price cache pinned, no clock use in any of the three tab renderers (grep clean; the Pinned clock is belt-and-braces), year is data-derived (`latest_year`, unlock.rs:206 — not wall clock), title bar shows the fake `/vault.pgp` so no tempdir leak, single row per table so no ordering exposure. The `cfg(all(test, unix))` retrofit matches the xtask tests module's own `#[cfg(test)] #[cfg(unix)]` (examples.rs:1361-1362) — on Windows neither the fns nor their callers compile (no dead-code, no missing-symbol), on unix everything ran above.

## Findings

**Minor**
1. `docs/examples-tui-walkthrough/j1/manifest.txt:6` + `00-setup.console.md:22` — the happy-path transcript's last verify line is `cb-sell … :: non_compliant`, and the prose ("BALANCED, no blockers — exit 0" — true) never mentions it. A first-time filer's template journey ends with their only sale flagged non-compliant, unexplained. Matches shipped `examples.md` practice and states nothing false, but since 7 journeys copy this template: add one PROSE clause (e.g., that the sale used the default method with no specific lot identification — the J9 topic).
2. `docs/examples-tui-walkthrough/j1/manifest.txt:7` vs `01-holdings.txt` line 37 — the prose highlights "$6,760 of the $8,450" while the frame's TOTAL row shows `84500.00` in the same "USD Basis" column (intentional weighted-avg $/BTC footer, holdings.rs:113-126; J8 identical). Juxtaposed with the $6,760 prose it reads like an absurd total basis. A half-sentence in the PROSE ("the TOTAL line's basis column is average cost per BTC") would defuse it walkthrough-wide.

**Nit**
1. `crates/btctax-tui/src/tabs/tests.rs:1033` — `#[ignore = "…rewrites docs/examples-tui-walkthrough/j8/04-*.txt"]` is stale; the emit now also rewrites `j1/01..03-*.txt`.
2. `crates/btctax-tui/src/tabs/tests.rs:906` — section header comment still reads "J8 PoC viewer frame"; the section now hosts J1 too.
