# UX-P4-1 implementation review — r2 (Fable, independent, focused re-review)

**Scope.** Verify that the two Important findings from r1
(`ux-p4-1-impl-fable-review-r1.md`) are genuinely closed by the fold commit `46c9eae`
(test-only: `crates/btctax-cli/tests/tax_report.rs` + `crates/btctax-tui/src/unlock.rs`), and that
the two test additions introduce no new issue. Review only; both mutations below were applied
temporarily via cp-backup/restore and the tree was verified clean after.

**Method.** Read the full `46c9eae` diff and both tests in context; re-verified the anchors in
current source (`cmd/tax.rs` `pseudo_contributed` arm, `resolve.rs` rung 3, `render.rs`
`PseudoDisclosure::banner()/suffix()` + `render_tax_outcome`, `main.rs:151-155` wire-through,
`session.rs:489-520` `resolve_all_screened` enumeration, `unlock.rs build_snapshot`,
`tabs/tax.rs render_tax_content`, `compute.rs compute_tax_year` rungs 1–3). **Live-mutation-tested
both KATs** (not just statically), then ran the full gate: `make check` **1971/1971 passed**
(r1 baseline 1970; the +1 is the new I2 KAT — I1 extended an existing test).

## Resolution table

| r1 ID | Status | Evidence |
|---|---|---|
| I1 — placeholder-channel KAT missing | **RESOLVED, mutation-proven** | `pseudo_mode_injects_placeholder_profile_clearing_tax_profile_missing` (tax_report.rs) now destructures `pseudo_contributed` + `advisory` from the real `report_tax_year` on the exact placeholder vault shape (pseudo ON, `count==0` — the fixture's fully-real buy+sell keeps `pseudo_synthetic_count == 0`, so `pseudo_active()` is false — no stored profile → resolve rung 3 → `Provenance::PseudoPlaceholder`). Assert chain is non-vacuous end-to-end: (a) `assert_eq!(pseudo_contributed, Placeholder)` — **live mutation: deleting the `else if provenance == PseudoPlaceholder` arm in `report_tax_year` reds at tax_report.rs:156 (`left: None, right: Placeholder`)**, independently reproducing the author's kill; a precedence flip (`Synthetic`) also reds it; (b) the rendered-banner assert matches `"estimated on a synthetic $0 placeholder profile"`, a phrase unique to the `Placeholder` banner arm (the `Synthetic` banner has no such text) — so channel identity, not just contributed-ness, is pinned in the render; (c) the TOTAL-line assert requires `"TOTAL federal tax attributable"` and `"[PSEUDO]"` on the same line, live because the outcome is asserted `Computed` first (the TOTAL line only renders in the Computed arm). The test drives the same `render_tax_outcome(y, &outcome, advisory, pseudo_contributed)` call `main.rs:151-155` makes with the same values — wire-through covered. SPEC §3.1 KAT (b) discharged, both halves (false-negative + correct-text). |
| I2 — TUI enumeration invariant not pinned | **RESOLVED, NON-VACUOUS, mutation-proven** | `build_snapshot_pseudo_on_unprofiled_year_stays_not_computable_in_the_viewer` (unlock.rs) goes through the REAL path: `cmd::init` → `pseudo_set_mode(true)` → `Session::open` → the `pub build_snapshot` → `resolve_all_screened` (session.rs:497-498 enumerates `tax_profile::years ∪ return_inputs::years`) → the production `tabs::tax::render_tax_content`. NOT a hand-built `Snapshot`, and "pseudo on" IS in the fixture (the r1 gap). **It passes for the RIGHT reason**: today the bare 2025 is never enumerated → absent from `snap.profiles` → `compute_tax_year` rung 3 → `NotComputable(TaxProfileMissing)`. Under the hypothesized regression the same vault takes a categorically different path: resolve rung 3 hands back the placeholder profile, all three refusal rungs pass (empty ledger ⇒ no Hard blocker; 2025 table bundled — an existence independently pinned by the CLI Computed KATs; profile present) ⇒ `Computed($0)`. **Live mutation: adding CLI-parity enumeration of the bare year inside `resolve_all_screened` reds the test at unlock.rs:672, and the captured render is exactly the reborn C2 channel — `TOTAL federal tax attributable (delta): 0.00  [= ord + LTCG + NIIT]` with NO `[PSEUDO]` anywhere (count==0 suppresses the banner/suffix)** — i.e., the assert genuinely distinguishes "not enumerated → NotComputable" from "placeholder injected → unflagged Computed $0". The `!contains("[PSEUDO]")` companion assert is belt-and-suspenders plus a deliberate trip-wire: a future viewer placeholder-parity change must consciously rewrite this test (matching the `tabs/tax.rs:70` TRIP-WIRE comment). SPEC §3.1 KAT (f) now pins the invariant itself, not just its render-layer consequence. |

## New issues from the two additions

None found. Both tests reuse the established fixture patterns of their files (tempdir + init +
typed accessors; the I2 test mirrors the adjacent `build_snapshot_prices_parity` shape); no
product code was touched (`46c9eae` is test-only, so the §1 tax-figure invariant is trivially
intact); no flake channels (no ambient env, no timing); the banner-phrase and TOTAL-line
substrings are unique to their intended arms in current `render.rs`; both tests red loud with
the offending render in the panic message. Restored tree verified clean; full gate green.

## Verdict

**GREEN — 0 Critical / 0 Important.** Both r1 Importants are closed by genuine, live-mutation-
verified KATs; no new findings. r1's recorded residue (M1, M2, N1, N2) stands as non-gating,
unchanged by this fold.
