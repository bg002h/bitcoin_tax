# Whole-branch review — tui-edit-chunk4b (Phase E, round 1)

**Branch:** `feat/tui-edit-chunk4b` @ `9008b54` (diff `main..HEAD`, main == `dc0859d`; commits: spec
`1e9a8fa`, Task 1 `97136d9`, Task 2 `9008b54`). Delegated-implementer cycle; independent gate.
**Spec:** `design/SPEC_tui_edit_chunk4b.md` (R0-GREEN, 2 rounds).
**Controller-verified full gate at HEAD:** 921 workspace tests, clippy `-D warnings` clean, fmt clean.
`btctax-core` untouched; `btctax-cli` gains only the additive `Session::optimize_proposal`.

## Controller fold disposition
- **[N1] Nit** — persistability `made` open-time vs `attested_at` `made` enter-time → **no code change**
  (no practical impact: modal flow; midnight-boundary at most; matches CLI intent, R0-round-2-blessed).
  Recorded as a minor FOLLOWUP.

## Reviewer output (verbatim)

# R0 Whole-Diff Review — tui-edit chunk 4b (resolve-conflict + optimize-accept)

**Scope:** `git diff main..HEAD` on `feat/tui-edit-chunk4b` (main `dc0859d`; commits `1e9a8fa`/`97136d9`/`9008b54`). 9 files, +2978/−36. `btctax-core` untouched (verified).

## Verdict: **0 Critical / 0 Important / 0 Minor / 1 Nit**

The implementation is sound and faithful to the R0-GREEN spec. Both flows sit correctly on the shipped substrate. All eight spec KATs plus the E2E and KAT-G1 gate pass; three fault-injection probes each went RED and were restored byte-for-byte. **Ship gate: PASS.**

## What I verified as correct

**1. `Session::optimize_proposal` (btctax-cli, additive) — clean.** The ONLY public-API addition. Assembly is byte-for-byte the same recipe as `cmd::optimize::run` (`cmd/optimize.rs:41-58`): `load_events_and_project()` → `self.tax_profile(year)` (FRESH, R0-N1) → `BundledPrices::load()` → `BundledTaxTables::load()` → `optimize_attested_set()` → `proposal_made = tax_date(now, UtcOffset::UTC)` (2-arg, R0-M1) → `optimize_year(...)` with the exact 8-arg order (`optimize.rs:713-722`) → `.map_err(map_opt_err)` applied INTERNALLY (`pub(crate)`, R0-M2). Uses the HELD session's `self` (no `Session::open` → no VaultLock deadlock). The opener calls it and on `Err(e)` sets status + returns (no-open, `main.rs:454-462`).

**2. optimize-accept dual-write — a faithful INVERSE of `persist_void`.** `persist_optimize_accept` snapshots → `append_decision(LotSelection)` → (if attested) `optimize_attest::set(conn, &disposal, &att, &made.to_string())` with the post-set failure routed through `rollback(session, &pre, e)` → final `save_or_rollback` whose whole-DB `restore(pre)` reverts BOTH. Structurally identical to `persist_void` (`persist.rs:259-291`), inverted. KAT-G1: `"optimize_attest::set"` added to `persist_only_tokens` (`persist.rs:1230`) AND the self-check plant (`:1407-1449`); not a substring of `optimize_attested_set`/`::get`/`::clear`, no false positive; no forbidden token in the 4 non-test production sources. KAT-G1 GREEN.

**3. Pre-filter — correct.** `filter_optimize_candidates` keeps rows where `proposed != current` AND `persistable != ForbiddenBroker2027` AND `!already_selected.contains(disposal)` (built from non-voided LotSelection disposal_events). Empty → status + no-open (R0-M3).

**4. NO per-disposal Δtax (R0-I1).** List columns `Date | Wallet | Persistability | Disposal EventId` (`draw_edit.rs:293`); the sole dollar figure is the flow-level banner from `OptimizeProposal.delta` + the approximate caveat. No fabricated per-row number.

**5. resolve-conflict non-revocability + both-sides modal.** `SupersedeImport`/`RejectImport` NOT in `is_revocable_payload` (`form.rs:841`; pinned by `kat_rc_supersede_reject_are_non_revocable`). Modal shows `current:` (target payload) vs `→new:` (conflict `new_payload`) + "!! This decision CANNOT be voided" — prominent, NOT typed-word. `conflict_event != target` honored. E2E: accept adopts basis 50000 / reject keeps 30000, both clearing the `ImportConflict` blocker.

**6. Fixture verified.** `oa_seed_computable_sell` (two same-wallet 2025 lots + 500k sale, no back-dated MethodElection → computable); `kat_e2e_oa_z_...` drives the FULL `z` path asserting the persisted LotSelection AND `optimize_attest::get == Some("attest-2025")`; void round-trip clears it (`get == None`).

**7. No existing-test regression.** The 36 "deletions" are a diff-rehunk artifact (`derive_attest_status` arms re-added verbatim + `use`-list reflow). 11 existing safe-harbor-attest/void KATs pass unchanged.

**8. Fault-injection (all RED, tree restored byte-for-byte):**
- Pre-filter live-LotSelection guard neutered → `kat_oa_filter_excludes_...` FAILED. Restored.
- resolve-conflict `Accept → RejectImport` → `kat_p2_rc_accept_...` + `kat_e2e_rc_accept` FAILED (basis 30000≠50000). Restored.
- optimize-accept skipped `optimize_attest::set` → `kat_p2_oa_attested_...` + `kat_e2e_oa_z_...` FAILED (att None≠Some). Restored.

**9. Quality.** Clippy clean on both crates; exhaustive `Persistability` matches; modal-dispatch layering matches convention; `reset_flows` clears the 4 new fields; docs match behavior.

### [N1] NIT — persistability `made` open-time vs attested_at `made` enter-time
`open_optimize_accept_flow` computes `proposal_made = tax_date(now_open, UTC)` (fixes each row's `Persistability`); `handle_optimize_accept_modal_key` computes a fresh `made = tax_date(now_enter, UTC)` for `attested_at` (`main.rs:682`). No practical impact — modal flow (no mutation between open and confirm); only `attested_at` could differ by one day at a midnight boundary; matches CLI intent (R0-round-2-blessed). Optional tighten: thread the opener's `proposal_made` through to persist.

**Gate: 0 Critical / 0 Important. Cleared to ship.**
