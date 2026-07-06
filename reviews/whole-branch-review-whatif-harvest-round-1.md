# Whole-diff review (Phase E) — feat/whatif-harvest STAGE P2 (harvest optimizer) — round 1

**Verdict: 0 Critical / 0 Important — SHIP (P2).**

Diff `main (806af93)..08db97a` — 1 task commit (P2 harvest). Contract: `design/SPEC_synthesize_whatif.md` +
the authoritative `design/agent-reports/fable-harvest-optimizer-advice.md`. **P3 (TUI) deferred.**

## ★ The optimizer is the architect's segment walk (NOT bisection) — trap KATs prove it
All 22 core `harvest.rs` KATs pass (my run), the whole non-monotone battery:
- **`harvest_dip`** — HIFO loss-first → marginal −660 @1 BTC then +1500 @2 BTC; the walk pins ~1.75 BTC (net
  gain crosses 0), STRICTLY past where a naive global bisection lands wrong.
- **`harvest_fifo_non_contiguous`** — FIFO election, `tax≤$3k` true→false→true; **prefix semantics returns the
  FIRST boundary** (~1.5 BTC), not the later 3-BTC feasible island (island existence verified via `sell`).
- **`harvest_carryforward_burn_disclosed`** — cf_long=$50k, all-gain, `tax=$0`: marginal flat $0 across
  absorption; exact identity `lt_gain + carryforward_delta.long == 0` (burn == gain absorbed), disclosed.
- Plus `harvest_3k_pin_flat_notbinding`, `st_feedback_shrinks_zero_room`, `cross_net_expands_room`,
  `qd_stacking_shrinks` + `qd_alone_already_breached`, `dual_basis_ngnl_zero_slope`, `two_edge_0_15_20`,
  `niit_kink`, `per_segment_monotone` (T1), `boundary_exactness`.

## Verified by KAT + a real run
- **[engine-verified answer]** `harvest_marginal_identity_engine_verified_and_deterministic` — the returned N*
  is always re-folded + predicate-checked; marginal == with.total − baseline.total; deterministic.
- **[★ non-persistence]** `harvest_writes_nothing` (core events byte-identical) + **my binary check: the vault
  SHA-256 is byte-identical after `--target zero-ltcg`**. Clone-fold-discard holds.
- **My real run:** `--target zero-ltcg, single/$40k, 2026` → FOUND, sell 0.08930044 BTC, bound by the 0% LTCG
  ceiling, $8,144.25 LT gain ALL in the 0% bracket (0 in 15%/20%), **marginal $0.00** — exactly the harvest primitive.
- **[predicates]** read the with-scenario `PrefSplit` (not `MarginalRates.ltcg`); the 4 targets + prefix semantics.
- **[disclosures]** carryforward burn, NIIT kink, $3k plateau; `HarvestStatus` 7-variant; `--magi`-floors caveat.
- **[refusals]** `harvest_refusals`, `harvest_no_lots`, `harvest_invalid_negative_target`, `harvest_mfs_1500`,
  `harvest_qss_maps_to_mfj`.

## ★ A validated deviation from the architect report (no tax-correctness guess)
The architect modeled a basis-pending lot as an `Ok` "pending-basis cap." The implementer correctly found the
premise FALSE for this engine: a basis-pending origin raises a resting Hard blocker, so `compute_tax_year`
refuses the whole year → the tax-correct behavior is to **REFUSE (`YearNotComputable`)**, not truncate + plan.
`harvest_pending_basis_refuses_year_not_computable` locks the honest refusal (the `N_avail`-truncation kept as
belt-and-suspenders for a hypothetical future non-gating pending lot). This is the "stop-and-don't-guess"
behavior working: a correct, conservative resolution, not a fabrication.

## Scope / suite
btctax-core (+`harvest` in whatif.rs) + btctax-cli (`what-if harvest` + render + docs). 22 core harvest KATs +
CLI. Full workspace (my close-out re-running); the P0+P1 numbers unchanged. Breaking already accounted (0.4.0).

**SHIP P2 — the harvest optimizer is the architect's non-monotone-safe segment walk; every trap KAT passes,
the answer is always engine-verified, and the vault is provably never written. P3 (TUI overlay) is the last
slice of task #43.**
