# R0 spec review — SPEC_self_transfer_inbound (Cycle A) — round 2 (fold verification)

**Artifact:** `design/SPEC_self_transfer_inbound.md` (round-1 folds applied).
**Reviewer:** independent adversarial architect (did NOT author).
**Verified against current source:** branch `feat/self-transfer-inbound` @ `6fdb682` (spec-only diff
over `main` @ `a740b3d`; source is live `a740b3d`). All anchors re-grounded this round.

## Verdict

**R0-GREEN — 0 Critical / 0 Important / 0 Minor / 0 Nit.**

All three Minors and three Nits from round 1 are resolved accurately with **no new drift**. Every
folded citation was re-verified against the current tree. Implementation may proceed.

## Per-fold verification

- **[M1] G6 catch-all reframe — ACCURATE.** Re-confirmed all three sites are genuinely `_ =>`
  defaults, and the cited line numbers are exact:
  - `is_disposition_op` (`resolve.rs:996`) → `_ => false` (`resolve.rs:1002`). ✓
  - `honoring_principal` (`resolve.rs:1008`) → `_ => None` (`resolve.rs:1014`). ✓
  - `evaluate.rs::honoring_sat` (`:76`) → `_ => None` (`evaluate.rs:82`). ✓ (the site round-1
    flagged as missing; now added and correctly located).
  All three default CORRECTLY for a lot-creating op (false / None / None), and the spec's warning
  that a wrong explicit arm (`honoring_principal => Some(sat)`) would silently break invariant #7
  is exactly right. Cross-check: the ONLY exhaustive `Op` match is `fold_event` (`fold.rs:538`) —
  confirmed it has NO `_ =>` catch-all, so the C4 arm is genuinely compile-forced. The G6(a)/(b)
  exhaustive-vs-catch-all split is sound. (Terminology nit, non-blocking: G6(a) folds `build_op`'s
  *InboundClass* match under "Op-match site"; substantively fine — both compile-forced arms, C3 for
  InboundClass and C4 for Op, are fully specified.)

- **[M2] wallet-missing corner — CONSISTENT now.** C4 (spec lines 121-124) and G5 (247-249) agree:
  emit Hard `UnknownBasisInbound` with a self-transfer message + return; do NOT copy the
  `IncomeInbound` guard (which emits `FmvMissing "income inbound without wallet"`, `fold.rs:830-839`
  — re-verified, semantically wrong for a non-income self-transfer). KAT added (lines 220-221).
  The self-contradiction is gone.

- **[M3] TUI enumeration — COMPLETE for the compile-forced sites.** I swept every `match` over an
  `InboundClass` / `InboundVariant` / `ClassifyInboundStep` value in `btctax-tui-edit` + `btctax-cli`
  (non-test). The exhaustive (arm-forcing) sites are exactly:
  - InboundClass: `build_op` (C3, core), `draw_classify_inbound_modal` (`draw_edit.rs:729`),
    `cls_desc` (`main.rs:2193`). — all enumerated (M3 lists `:728`/`:2193`). ✓
  - InboundVariant: Tab-cycle (`main.rs:769`), variant→form (`main.rs:783`), VariantPicker draw
    (`draw_edit.rs:604`, inside `draw_classify_inbound_form` — covered by the `:591` pointer). ✓
  - ClassifyInboundStep: draw `match step` (`draw_edit.rs:603`, inside `draw_classify_inbound_form`),
    step-index (`main.rs:698`). — enumerated (M3 lists `:698`; draw via `:591`). ✓
  The `main.rs:2168` `match as_ { Income, _ => "?" }` correctly stays OFF the list — it has a
  catch-all and self-transfer never fires `FmvMissing`. The `form.rs` `if let InboundClass::…` sites
  are non-exhaustive (no arm needed). "Modal covers it" is fixed (STATE reused vs. render fn arm).

- **[N1]** advisory message now `btctax reconcile classify-inbound-self-transfer --basis` (line 130). ✓
- **[N2]** KATs added for the pre-2025-Universal path + wallet-missing corner (lines 219-221). ✓
- **[N3]** G7 `acquired_at > receipt` hygiene note added (lines 258-260); correctly framed as
  optional (a future date is the conservative/short-term direction). ✓

## One non-blocking observation (does NOT affect GREEN)

M3's scope is *exhaustive compile-forced match sites*, and within that scope it is complete. The
new `SelfTransferForm` step also needs ordinary **functional key-handler wiring** (Enter→validate→
modal, Tab/focus, text input, Esc→picker) — the `handle_ci_*` branches at `main.rs:822/857/949/988`
are `if let` guards (non-exhaustive, so NOT compile-forced), and adding a SelfTransferForm branch is
implied by "add a SelfTransferForm step" and is exercised by the Task-3 E2E KAT ("classify a raw
TransferIn as self-transfer-in → lot created"). This is normal form-flow implementation, not a
missed match arm — noted only so Task 3 scopes the handler alongside the draw/validator.

**Gate: R0-GREEN. Cleared for implementation (Task 1 first).**
