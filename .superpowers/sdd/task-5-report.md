# Task 5 report — The five payload probes and their gating KATs

**Status: DONE / GREEN.** Branch `feat/stale-snapshot-latch`.

(Note: this file previously held a stale report from an unrelated task run — the Defensive Filing
Wizard's C-2 `tranche_guard` predicate move, on a different branch. That content is gone; this is the
`feat/stale-snapshot-latch` Task 5 report, per the orchestrator's instruction to write to this exact
path.)

## What was done

Added the `if let Some(s) = app.stale_reason() { app.status = Some(s); return; }` probe, before any `&mut`
field borrow, at all five sites named in the brief (line numbers were stale as warned; located by symbol):

1. `handle_declare_flow_key`'s `Char('t')` arm (the on-demand tax-Δ preview).
2. `declare_flow_confirm` (function top) — builds + writes the `DeclarePlan`.
3. `handle_promote_flow_provenance_key`'s `Enter` arm (`attest_provenance`).
4. `promote_flow_review` (function top) — builds the `PromotePlan` (`Acknowledgment{shown_terms}` +
   `filed_basis`).
5. `handle_pseudo_approve_modal_key`'s `Enter` arm — builds + writes the pseudo-approve payload set.

Removed `stale_reason`'s `#[allow(dead_code)]` (confirmed by grep it was the last one in the file).

## Tests added (all in `crates/btctax-tui-edit/src/main.rs`'s `mod tests`, end of file)

Fixtures: `vault_with_declare_flow_open`, `vault_at_declare_confirm`, `vault_with_promote_flow_open`,
`vault_at_promote_consent`, `vault_with_pending_pseudo_defaults` (each backed by a REAL vault + session,
not a bare in-memory snapshot), plus `pseudo_approve_modal_fixture`, `promote_flow_reached_consent`,
`live_declare_count`, `live_decision_count` (the last two read the CURRENT session's on-disk state via
`btctax_core::persistence::load_all`, not the possibly-stale `app.snapshot` — the "written artifact"
check the brief asked for).

Five tests:

- `no_decision_payload_is_constructible_while_the_stale_latch_is_armed` — the brief's 3-in-1 gating KAT:
  proves no `PromotePlan` is constructed (`promote_flow_reached_consent` stays false), no `DeclareTranche`
  lands (`live_declare_count == 0`), and no pseudo decision lands (`live_decision_count == 0`), all with
  the latch armed.
- `declare_flow_confirm_refusal_while_stale_preserves_the_flows_window_sat_wallet` — the "must keep
  working" property for site 2: sat/wallet/window and the Confirm step are byte-identical before/after
  the refusal.
- `promote_flow_review_refusal_while_stale_preserves_the_authored_part_ii_narrative` — same property for
  site 4: the Part II narrative buffer (`part_ii`, which lives outside `step`) survives verbatim, and the
  step stays at `PartII` (never reaches `Consent`).
- `declare_flow_char_t_preview_refuses_to_read_the_stale_snapshot` (site 1) — includes a positive control
  (the identical keystrokes, unarmed, really do compute `tax_delta`), so the armed `None` is provably the
  refusal, not a fixture that never would have computed anything.
- `handle_promote_flow_provenance_key_enter_refuses_attest_provenance_while_stale` (site 3) — a `Purchase`
  selection (pure setter) followed by an armed `Enter` must not advance to `PartII`.

`vault_with_pending_pseudo_defaults` asserts its own fixture sanity (`pseudo_plan` non-empty) before the
gating test runs it, so that test cannot pass vacuously.

## Mutation proof (one probe deleted at a time, via `Edit` + re-run + restore from a `cp` backup — never
`git checkout --`)

- **Site 2 probe deleted** (`declare_flow_confirm`): `no_decision_payload_is_constructible_…` and
  `declare_flow_confirm_refusal_while_stale_…` both RED —
  `live_declare_count(&app2)` was `1` (the write went through); the flow also closed, since the write
  succeeded. Restored — GREEN.
- **Site 4 probe deleted** (`promote_flow_review`): both gating tests RED —
  `promote_flow_reached_consent(&app)` was `true`, with the panic output showing a fully-constructed
  `PromotePlan` (`Acknowledgment{phrase: "I understand and accept this estimated-basis risk", ...}`,
  `filed_basis: 2848.96`). Restored — GREEN.
- **Site 5 probe deleted** (`handle_pseudo_approve_modal_key`): `no_decision_payload_is_constructible_…`
  RED — `live_decision_count(&app3)` was `1`. Restored — GREEN.
- **Site 1 probe deleted** (`Char('t')` arm): `declare_flow_char_t_preview_refuses_…` RED —
  `tax_delta` was computed (`Some(..)`) even while armed. Restored — GREEN.
- **Site 3 probe deleted** (provenance `Enter` arm): `handle_promote_flow_provenance_key_enter_refuses_…`
  RED — the step advanced to `PartII { error: None }` even while armed. Restored — GREEN.

All five confirmed independently; each restore verified GREEN again before moving to the next.

## Gate results

- `cargo nextest run -p btctax-tui-edit` — 468 passed, 2 skipped (was 463 pre-task; +5 new tests).
- `make check` (workspace) — **2444 passed, 11 skipped** (brief's stated baseline was 2439; +5 matches the
  5 new tests, no other change to the suite size).
- `cargo fmt --all -- --check` — exit 0 (ran `cargo fmt --all` once to settle one multi-line `assert_eq!`
  wrapping introduced by a doc/test edit, then reconfirmed `--check` clean).

## Scope discipline

Only `crates/btctax-tui-edit/src/main.rs` was touched, as instructed. `execute_defensive_export` and
`open_defensive_filing` were not touched (Task 6's territory — they deliberately keep the original
`residue_latch_status`/`stale_or_residue_latch_status` behavior so the export route stays reachable while
stale). No other agent's work was disturbed; did not touch `.superpowers/sdd/task-2-report.md`, which
already carried an unrelated pre-existing uncommitted diff at session start.

## Concerns

None blocking. Two things worth a reviewer's glance, both Minor:

1. Sites 1 and 3 (the two non-writing, non-`PromotePlan`/`DeclarePlan`/pseudo-plan sites) aren't among the
   brief's named "three gating KATs," but I added dedicated tests + mutation proof for them anyway (all
   five sites are filing-relevant reads/writes per the brief's own enumeration), since leaving them
   unverified felt like a gap given the brief's "not merely a status-only" bar.
2. `live_decision_count` counts ANY `EventId::Decision`-identified event in the vault, not specifically a
   pseudo-approve payload — correct for its one use (the pseudo fixture's vault starts with zero
   decisions of any kind, so "count == 0" is an unambiguous artifact-absence check), but it would need
   narrowing if reused against a vault that already has unrelated decisions on it.
