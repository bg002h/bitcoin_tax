# Whole-branch review — tui-edit-hardening (Phase E, round 1)

**Branch:** `feat/tui-edit-hardening` @ `ea4a40e` (diff `main..HEAD`, main == `8c8b924`; commits: spec
`b3db87f`, Task 1 `91141fd`, Task 2 `c6cf531`, Task 3 `ea4a40e`).
**Spec:** `design/SPEC_tui_edit_hardening.md` (R0-GREEN, 2 rounds). Implementation was delegated to a
single implementer; this is the independent verification gate.

## Controller fold disposition
- **[M1] Minor** — `KAT-VOID-INERT-ALLOC-LISTED` doesn't E2E-assert the void modal's `is_safe_harbor`
  flag on the (now-reachable) inert-allocation path (the ATTEST-VOID rewrite removed the only E2E
  assertion of it) → **FOLD** (press Enter, assert the flag — restores coverage this cycle removed).
  Test-only addition; self-verified (compiles + passes + assertion load-bearing).

## Reviewer output (verbatim)

# Whole-diff review — `feat/tui-edit-hardening` (Phase E)

**Verdict: 0 Critical / 0 Important / 1 Minor / 0 Nit — SHIP.**

Scope reviewed: `git diff main..HEAD` (main `8c8b924`; commits `b3db87f`, `91141fd`, `c6cf531`, `ea4a40e`). Product code: `crates/btctax-tui-edit/src/{main.rs, edit/form.rs, draw_edit.rs}`. `btctax-core` confirmed **untouched** (`git diff main..HEAD -- crates/btctax-core/` empty). Clean `cargo build -p btctax-tui-edit` (zero warnings). 15 targeted KATs run green.

## Fixes verified

- **#8 (quit-first fold).** All six arms carry `"…or quit the editor and run: btctax reconcile void…"` (main.rs:2062, 2080, 2098, 2134, 2346, 2375). No `"or CLI"` remains anywhere in the crate. KAT-RS-1..4 enriched with `assert!(status.contains("quit the editor"))`; RS-5/RS-6 added for the two previously-uncovered derivers (`derive_reclassify_income_status`, `derive_set_fmv_status`). RS-6 correctly threads past the FmvMissing arm to hit DecisionConflict.
- **#6 (per-instance cap).** `FieldBuffer{cap}` + `with_cap` pre-allocates `String::with_capacity(cap)` (invariant preserved); `push_char`/`set` gate on `self.cap`. `FREETEXT_CAP=512`. Donation form (main.rs:3039-3048): exactly the 6 free-text buffers use `with_cap(FREETEXT_CAP)`; the 4 structured (`donee_ein`, `appraiser_tin`, `appraiser_ptin`, `appraisal_date`) keep `new()`/64. KAT-STRUCTURED-CAP asserts `donee_ein_buf.buf.len()==64` after 100 chars; KAT-FREETEXT-CAP round-trips 200 chars through save/reload.
- **#7 (effective-alloc void pre-filter).** `effective_alloc` (main.rs:2492) is exactly "`SafeHarborAllocation` AND neither `SafeHarborTimebar` NOR `SafeHarborUnconservable` on `e.id`", reading `snap.state.blockers`. Applied as `.filter(|e| !effective_alloc(e))`. Inert (timebarred/unconservable) allocations return `false` → stay listed (KAT-VOID-INERT green). Already-voided allocs handled by the prior `voided`-set filter, so no interaction. The retained §7.4 engine guards are present and named exactly: `crates/btctax-core/tests/transition.rs:365 void_of_effective_allocation_is_a_decision_conflict` and `:403 void_of_inert_allocation_applies_no_conflict`.
- **The ATTEST-VOID rewrite (highest-risk).** New `kat_e2e_attest_void_list_empty_after_attest` asserts `void_flow.is_none()` + status `"No revocable decisions to void"` after attest, with a self-validating precondition that the new alloc is effective. The old trap body (`kat_e2e_attest_void_new_alloc_yields_conflict_status`: asserted the attested alloc IS listed → select → modal → "remains in force") is **fully removed** — no dead/contradictory asserts remain; the deleted assertions targeted the now-genuinely-unreachable effective-alloc modal.
- **#1 (SelfTransfer).** New `DisposalKind::SelfTransfer`; both exhaustive matches updated (`draw_edit.rs:1529, 1691`); no wildcard `DisposalKind` arm exists anywhere (compiler-enforced exhaustive). Reconstruction (main.rs:3340-3372) is **faithful to the engine**: `transfer_links` sorted by `decision_seq` asc; FIRST-WINS on dup `out_event`; `consumed_ins` dedup; in-event skipped via `ev_idx.get(...).and_then(|e| e.wallet.as_ref()).is_none()` (never index). I cross-checked against `resolve.rs:480-527` (link build) and `:201-216` (projection): every out ∈ engine `links` resolves to `Some(dest)` and projects to `SelfTransfer`, so the TUI's `linked_outs` == `links.keys()` exactly. `principal_sat = TransferOut.sat`, source wallet, and `honoring_principal` returns `Some(t.sat)` for SelfTransfer (`resolve.rs:1013`) → no spurious `LotSelectionInvalid`. The builder also guards `out_id` is really a `TransferOut`.
- **#2 (pre-2025 gate).** Gate (main.rs:2750-2755) is exactly the spec: `item.date < TRANSITION_DATE ⇒ l.acquired_at < TRANSITION_DATE && l.basis_source != BasisSource::SafeHarborAllocated`, else `wallet_ref.is_some_and(|w| &l.wallet == w)`. The pre-2025 branch does **not** reference `item.wallet`, fixing the original `wallet==None ⇒ zero lots` bug. Date comparison is `<` on both sides, matching `pool_key`'s `date < TRANSITION_DATE ⇒ Universal` (no off-by-one).
- **#3 (uncovered pre-filter).** `uncovered` set built from `UncoveredDisposal` blockers and applied to **all three** builders — disposals (3381), removals (3408), self-transfers (3431). KAT-UNCOVERED uses a genuinely partially-covered fixture (acquire 300K / dispose 500K) and asserts the disposal IS in `state.disposals` AND carries the blocker before asserting the flow doesn't open — so it truly exercises the pre-filter, not a not-recorded artifact.

## Fault-injection probes (all RED then restored; `git status` clean, diff stat unchanged after each)

1. **#7 / ATTEST-VOID** — neutered `.filter(|e| !effective_alloc(e))` → `kat_e2e_attest_void_list_empty_after_attest` **FAILED** and `kat_void_effective_prefilter_mixed` **FAILED**.
2. **#2 / PATHB-SEEDLOTS** — removed the `&& l.basis_source != SafeHarborAllocated` clause → `kat_pre2025_pathb_seedlots_excluded` **FAILED** (panic: seed lots offered → transitions to LotsForm instead of staying on List).
3. **#1 / SELFTRANSFER** — neutered the Wallet-arm `linked_outs.insert` → `kat_selftransfer_selectable` **FAILED** (panic: flow doesn't open without the reconstructed row).

All three restored via `git checkout`; tree verified clean and diff stat identical.

## Minor

### [M1] MINOR — the positive `is_safe_harbor` void-modal flag is no longer asserted through a reachable flow
`crates/btctax-tui-edit/src/main.rs` KAT-VOID-INERT-ALLOC-LISTED (11411) stops at asserting the inert allocation is *listed* (`payload_tag == "SafeHarborAllocation"`); it never presses Enter to open the modal. The old ATTEST-VOID test's now-deleted back half was the only place that E2E-asserted `modal.is_safe_harbor == true` — but it did so on the effective/new-attested alloc, whose modal is now genuinely unreachable. The still-reachable path (voiding an *inert* allocation, which per spec still carries the Path-B warning) is the natural home for that assertion and doesn't cover it. Does **not** gate: production is the trivial one-liner `is_safe_harbor = item.payload_tag == "SafeHarborAllocation"` (main.rs:1853), covered piecewise (render test `draw_edit.rs:2348`, payload_tag pin in KAT-VOID-INERT, negative pin main.rs:8777). Fix (optional): in KAT-VOID-INERT, `press(Enter)` + `assert!(app.void_modal.as_ref().unwrap().is_safe_harbor)`.

## Notes (non-findings)
- The `#7` comment cite `resolve.rs:865-921` is accurate (superset of the spec's `883-921`).
- The acknowledged `#2` residual (a later `bump_split` lot offered for an earlier pre-2025 disposal, backstopped by `LotSelectionInvalid`) is recorded in `FOLLOWUPS.md`.
- `#3`'s uncovered filter is directly exercised only on the disposals builder by KAT-UNCOVERED, but the removals/self-transfer builders carry the identical, verified `.filter(...!uncovered...)`.

**0 Critical / 0 Important — approved to ship.**
