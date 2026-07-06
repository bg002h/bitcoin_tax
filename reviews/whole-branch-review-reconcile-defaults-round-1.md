# Whole-diff review (Phase E) — feat/reconcile-defaults — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (b976621)..HEAD` — 1 impl commit (`7c79cd5`), 32 files, +1,239/−73
(the bulk = the 42-test migration). Contract: `design/SPEC_reconcile_defaults.md` (R0-GREEN, 2 rounds).
Tax-critical user-mandated default change.

## Fault-injection of both guards (restored byte-for-byte)
- **[★ HIFO global default] CONFIRMED load-bearing.** The fold's only method-resolution default —
  `fold.rs:41 .unwrap_or(LotMethod::Hifo)` — is the load-bearing site (config.rs:29 + mod.rs:55/128 flip the
  config/UI defaults in step). **Fault-inject:** reverting it to `Fifo` drove `default_method_is_hifo` RED. A
  no-election vault now computes HIFO end-to-end.
- **[★ long-term self-transfer-in] CONFIRMED load-bearing.** `fold.rs:1024`
  `acquired_at.unwrap_or_else(|| long_term_default_acquired(date))` — the single common default for the pseudo
  AND manual paths. **Fault-inject:** reverting to `unwrap_or(date)` (receipt-date short-term) drove
  `self_transfer_in_defaults_to_long_term` RED.

## Verified by inspection + named KATs
- **Correct sites (R0-C1):** the four flips are `fold.rs:41` + `config.rs:29` + `mod.rs:55/128`; the
  `#[cfg(test)]` fixtures + the `pools.rs` FIFO mechanic are untouched.
- **serde untouched (R0-C2):** the enum `#[default]` stays Fifo; the immutable `SafeHarborAllocation` serde
  default is preserved (`safe_harbor_allocation_pre2025_method_serde_default_fifo` reused as the guard).
- **Leap-safe long-term (R0-I1):** `conventions::long_term_default_acquired` (`replace_year(y-1)`, Feb-29→28,
  −1 day); `self_transfer_long_term_leap_crossing` covers a leap-window sale AND a Feb-29 receipt.
- **Disclosure independent of basis (R0-I2):** new `SelfTransferInboundDefaultedAcquired` advisory gated on
  `acquired_at.is_none()` (`classify_with_basis_no_acquired_discloses_long_term`); stale short-term text fixed
  in the advisory + `--help` + code comments + man pages (R0-I3).
- **Test migration (R0-I4):** the optimizer suite pinned explicit FIFO elections to restore each fixture's
  baseline (not a rename — the disposal-compliance model showed a standing order is the only lever);
  method_election/transition/tui/kat clusters migrated; the inverted `..._defaults_to_receipt_date_short_term`
  KAT REPLACED. New KATs: `default_method_is_hifo`, `explicit_fifo_election_still_fifo`, `pools_mechanic_stays_fifo`,
  `explicit_acquired_supersedes`, `manual_classify_inbound_self_transfer_also_long_term`.
- HIFO stays `attested: false` (specific-ID reminder preserved); basis stays $0 (conservative on amount).

## Suite
`cargo test --workspace --locked` (implementer 1222 passed / 0 failed; re-run at merge); clippy -D + fmt clean.
Behavior change to default tax outcomes → MINOR (README "Realistic reconcile defaults" note + FOLLOWUPS +
[[self-transfer-completion-policy]] memory updated).

**SHIP — realistic auto-reconcile defaults (HIFO + long-term), both fault-verified.**
