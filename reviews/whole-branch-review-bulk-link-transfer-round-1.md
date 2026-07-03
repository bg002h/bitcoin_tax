# Whole-branch review — bulk-link-transfer (Phase E, round 1)

**Branch:** `feat/bulk-link-transfer` @ `22c182b` (diff `main..HEAD`, main == `a16ea00`; commits: spec
`8257705`, Task 1 `7d91c29`, Task 2 `111d962`, Task 3 `22c182b`). Delegated-implementer cycle; independent gate.
**Spec:** `design/SPEC_bulk_link_transfer.md` (R0-GREEN, 2 rounds).
**Controller-verified full gate at HEAD:** 946 workspace tests, clippy `-D warnings` clean, fmt clean.
`btctax-core` untouched; `btctax-cli` gains only the three additive fns (`bulk_link_transfer_plan`,
`bulk_link_plan`, `apply_bulk_link_transfer`).

## Controller fold disposition
All findings Minor/Nit (non-blocking; "cleared to ship"):
- **[M1] Minor** — CLI prints an empty preview table before the "no match" line → recorded as a FOLLOWUP
  (cosmetic; output still correct).
- **[M2] Minor** — two-phase CLI re-opens the vault between plan/apply without re-validation → by-design
  (spec-sanctioned R0-M2; safe for single-user local CLI). No action.
- **[N1] basis_usd computed-not-rendered** (carried per spec D1 for future use; not dead code),
  **[N2] Preview re-entry resets exclusions** (conscious UX), **[N3] now_utc() inline** (fine) → no action.

## Reviewer output (verbatim)

# Whole-diff review — `feat/bulk-link-transfer` (round 1)

**Verdict: 0 Critical / 0 Important / 2 Minor / 3 Nit — SHIP.**

`btctax-core` untouched (verified). The 25 deletions are all `use`-block reformatting; no existing test body or assertion was modified — the test diffs are purely additive tails. Every spec-load-bearing guarantee verified against current source; the three highest-value KATs fault-injected (neutered → RED → restored byte-for-byte; `git status` clean afterward).

## Dig-list verification (all PASS)
**1. Selection boundary — PASS.** `bulk_link_transfer_plan` iterates `for pt in &state.pending_reconciliation` (the projected set); never "all TransferOut events" → an already-decided/linked out cannot be re-touched.
**2. [I1] mid-batch rollback — PASS (fault-injected).** `persist_bulk_link_transfer` (`edit/persist.rs:378-394`) uses `if let Err(e) = append_decision(...) { return Err(rollback(session, &pre, e.into())); }` — not `?`. KAT injects a `BEFORE INSERT … RAISE(ABORT)` trigger at `decision_seq = pre_max+2` (append #1 commits, #2 aborts), asserts `RolledBack` + `post == pre` byte-identical + clean retry. Fault-inject: replaced rollback with `?` → KAT FAILED (`Err(NoChange)`). Restored.
**3. [I2] honest floor — PASS.** `total_usd_value_floor: Usd` + `missing_price_count: usize` (neither Option). `usd_value = btctax_core::price::fmv_of(&prices, date, principal_sat)` [M1]. Floor = Σ of priced rows; render exact `$X` when missing==0 else `≥ $X (N unavailable)` in all three renderers. `bulk_plan_usd_total_floor_when_price_missing` locks it.
**4. Atomicity (both surfaces) — PASS.** CLI `apply_bulk_link_transfer` = N `append_decision?` then one `save?` (mid-batch `?` drops the unwritten Session); TUI = the I1 path + one `save_or_rollback`. Neither saves per-row.
**5. Same-wallet skip + empty guards — PASS (fault-injected).** `source_wallet == Some(dest)` → `skipped_same_wallet`. Empty guards at every gate (CLI empty→exit 0 no write; TUI opener refuses; recompute-empty stays on Filter; modal refuses 0-checked; persist refuses empty). Fault-inject: routed same-wallet into `included` → `kat_bulk_same_wallet_row_absent` FAILED (`left: 2, right: 1`). Restored.
**6. Fork B typed destination — PASS.** `handle_bulk_dest_type_key` parses via `eventref::parse_wallet_id`; `kat_bulk_typed_dest_cold_wallet` types `self:cold-wallet` (in no event) → `SelfCustody{label}` dest the batch links to. Engine maps `Wallet(w)→SelfTransfer` with no in-event requirement.
**7. KAT-G1 cleanliness — PASS.** Batch append confined to `edit/persist.rs`; opener reads `snap` directly; only the priced plan uses the Session helper. No persist-only token in tui-edit non-test source. Gate GREEN.
**8. Honesty notes — all VERIFIED.** (a) The E2E's INDIRECT `Op::SelfTransfer` proxy (out leaves pending + one TransferLink to Wallet(dest) + dest gains holdings) is SOUND — a disposal would not add `cold` to holdings; the conjunction rules out the disposal reading. (b) The untested interactive `y/N` cut (tested `bulk_link_plan`/`apply_bulk_link_transfer`) is right per the two-phase R0-M2 split. (c) Draw code exercised by build + KAT-G1 only — acceptable per the spec's KAT list.
**9. No regression + test integrity — PASS.** 5 CLI + 10 TUI bulk KATs + KAT-G1 GREEN; existing tests additive-only. Three NEW KATs fault-injected RED-then-restored; final `git diff HEAD` empty.
**10. Dead code / clippy / docs — PASS.** clippy clean; matches exhaustive (`BulkLinkStep`/`Frame`); `on_persist_error` maps benign bulk `RolledBack`/`NoChange` to keep-open retry, only `ResidueLive` arms the latch; `close_all_mutation_surfaces` clears `bulk_link_flow`/`bulk_link_modal` (`main.rs:559-560`); opener gated by `residue_latch_status()`.

## Minor
### [M1] MINOR — CLI prints an empty preview table before the "no match" line
`render_bulk_link_preview(&plan)` runs unconditionally, then the empty arm prints "no pending outbound transfers match" — a header-only table then the no-match line. Harmless; output correct. Fix (optional): move the empty check above the render.
### [M2] MINOR — two-phase CLI re-opens the vault between plan and apply (no re-validation)
Spec-sanctioned split [R0-M2]; safe for a single-user local CLI (a concurrent external mutation between the two opens wouldn't be re-validated). Acceptable given the threat model. No fix required.

## Nit
### [N1] `basis_usd`/`total_basis_usd` computed but never rendered — carried per spec D1 for future use; not dead code (public fields, `PartialEq`-exercised). No action.
### [N2] TUI `Preview` re-entry resets exclusions — reasonable "re-filter resets selection" UX; conscious choice.
### [N3] TUI apply uses `OffsetDateTime::now_utc()` inline — consistent with the modal-apply pattern; fine.

## Ship gate
**Cleared to ship.** 0 Critical / 0 Important. The load-bearing properties — `pending_reconciliation` selection boundary, [I1] whole-batch revert, [I2] honest floor, dual-surface single-save atomicity, same-wallet skip, empty guards, Fork-B typed destination, KAT-G1 confinement — all implemented as specified and independently proven (three fault-injections RED-then-restored; KAT-G1 + clippy green; core untouched). Minor/Nit items are quality observations, none blocking.
