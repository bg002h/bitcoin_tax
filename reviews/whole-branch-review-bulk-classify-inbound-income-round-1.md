# Whole-diff review (Phase E) — feat/bulk-classify-inbound-income (Cycle 4) — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 1 Nit (folded) — SHIP.**

Independent Phase-E review (reviewer ≠ author). Diff `33f7fd5..HEAD` (Task 1 `fb7b108` CLI plan+apply+subcommand;
Task 2 `28150f5` TUI `I` flow + wrapper). Contract: `design/SPEC_bulk_classify_inbound_income.md` (R0-GREEN,
2 rounds). 10 files (btctax-cli: session, cmd/reconcile, main, lib, tests/reconcile; btctax-tui-edit: main,
draw_edit, edit/form, edit/persist, editor). No btctax-core change.

## Verification + fault-injection (probe restored the tree byte-for-byte)

**1. [★ #a tax-safety — the whole cycle] the `fmv_of == None` exclusion — CONFIRMED.**
`Session::bulk_classify_income_plan` (session.rs): candidates = `state.blockers` `UnknownBasisInbound`
inbounds − already-classified (filter-3) − wallet-less − **`fmv_of == None`** (`match fmv_of(...) { Some(fmv)
=> included.push(row{fmv}), None => excluded_missing_price += 1 }`). `included` carries a RESOLVED `fmv: Usd`
(non-Option). `excluded_missing_price` is surfaced, not dropped.
- **[★ fault-inject]** Rewrote the `None` arm to INCLUDE the row (fabricated `$0` fmv) instead of excluding:
  `bulk_income_plan_excludes_missing_price` went RED. The exclusion is load-bearing. Restored.
- **Why this matters** (verified in R0): a persisted `Income{fmv:None}` raises Hard `FmvMissing`
  (fold.rs:853-860) that gates the year, and is NOT clearable by `ManualFmv` on the inbound path (that is
  itself Hard `DecisionConflict`, resolve.rs:481-493) — only void+reclassify. So exclusion is the sole safe path.

**2. CLI own-loop (R0-I1) — CORRECT.** `apply_bulk_classify_inbound_income` (reconcile.rs:321) is its OWN
`for in_event in &in_events` append-loop + single `session.save()` (mirrors shipped `apply_bulk_self_transfer_in`),
bare `?`-before-`save` = in-memory discard on mid-batch failure. **No `persist_bulk_decisions` call anywhere
in btctax-cli** (grep: only a comment) — the dependency-cycle trap is avoided. The dispatch derives `in_events`
from `plan.included` (never raw `--ref`), so the fmv-exclusion is non-bypassable via the CLI.

**3. Structural no-`None` guarantee — HARDENED (the folded Nit).** The apply RE-COMPUTES `fmv_of(date, sat)`
with `date = tax_date(ev.utc_timestamp, ev.original_tz)` — byte-identical to the plan's derivation, same
`BundledPrices` — so it is deterministically `Some` for every plan-included id. Functionally correct, but the
`pub` fn achieved "`Income{fmv:None}` unrepresentable" only via determinism + dispatch-protection, not
structure. **Folded:** `let Some(fmv) = fmv_of(...) else { continue };` + `fmv: Some(fmv)` — now a missing-price
row is STRUCTURALLY skipped even if a future caller passed a non-plan-filtered id. Restores the spec's intent
on a year-gating money path.

**4. Uniform params + payload — CORRECT.** `InboundClass::Income { kind, fmv, business }` (no phantom
`fmv_status`; `fmv` is `Option<Usd>`). Uniform `kind`/`business` per batch; per-row auto-FMV. TUI `I` flow →
revocable (non-typed) confirm tier (matches bulk-sti, correct for a voidable classification) → wrapper
`persist_bulk_classify_income` → `persist_bulk_decisions` (tui-edit CAN reach it).

**5. KAT coverage — complete (all 12 pass).** Plan: `_lists_pending_inbounds`, `_excludes_already_classified`,
`_excludes_missing_price`, `_excludes_wallet_less`, `_apply_sets_autofmv`, `_apply_recognizes_income` (E2E:
`income_recognized` grows, `UnknownBasisInbound` clears, NO `FmvMissing`), `_dry_run_writes_nothing`,
`_uniform_kind_and_business`, `_empty_refuses`; TUI `_refuses_when_no_candidates`, `_per_row_exclude_drops_row`,
`_preview_shows_total_and_excluded`. The shipped bulk-sti KATs stayed GREEN unchanged.

**6. No over-reach** — reuses `ClassifyInbound` + `InboundClass::Income` + `fmv_of`; no new `EventPayload`
variant, no serde break, no btctax-core change. The two implementer notes (CLI `--year/--from/--to/--wallet`
filter flags; inline kind/business cycling) are faithful mirrors of bulk-sti, not divergences.

## Full suite
`cargo test --workspace --locked` + `clippy -D warnings` + `fmt --check` — green after the fold (see ship gate).

**SHIP.**
