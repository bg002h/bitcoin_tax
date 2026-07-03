# R0 spec review — tui-edit chunk 4a (link-transfer + classify-raw), round 2

**Artifact:** `design/SPEC_tui_edit_chunk4a.md` (post round-1 fold)
**Baseline:** `main` @ `755e47c` (re-grounded against current source).
**Reviewer:** independent adversarial architect (did NOT author).
**Bar:** 0 Critical / 0 Important.

## Verdict: 0 Critical / 0 Important / 0 Minor / 1 Nit → **R0-GREEN**

All six round-1 findings verified resolved against the actual structs; no new drift introduced by the fold.

### [I1] D2 builder field lists — RESOLVED ✓
- **Acquire** (`spec:144-148`) = `{ sat: Sat, usd_cost: Usd, fee_usd: Usd, basis_source: BasisSource }` — matches `event.rs:45-50` field-for-field. `basis_source` is a required `BasisSource` PICK (default `ExchangeProvided`, valid variant `event.rs:18`); `fee_usd` optional→$0 is a sound form convention for the non-`Option` `Usd` field; `acquired-at` correctly DROPPED with the right reason (effective event keeps the target's timestamp, `resolve.rs:709-729`). Non-gating note: `ExchangeProvided` vs `ComputedFromCost` as the *default* is cosmetic — both are known-basis provenance labels for an `Op::Acquire`, neither sets `basis_pending` nor alters gain, and it's a user PICK.
- **Income** (`spec:149-154`) = `{ sat: Sat, usd_fmv: Option<Usd>, fmv_status: FmvStatus, kind: IncomeKind, business: bool }` — matches `event.rs:52-58`. The `fmv_status` mapping (typed → `ManualEntry`; empty → `None` + `Missing`) is sound against the load-bearing `resolve.rs:187` (`fmv = usd_fmv.filter(|_| fmv_status != Missing)`), and empty→`Missing` correctly fires `FmvMissing` (`fold.rs:652`), surfaced by status arm 3. Builder emits `EventPayload::Income` DIRECTLY (not `InboundClass::Income`) — the type mismatch is closed.

Both builders are now struct-accurate and buildable.

### [I2] wallet-target list source — RESOLVED ✓
`spec:68-73`: now "all distinct `snap.events[].wallet` (the `Some` values)," with the correct rationale that `holdings_by_wallet` only holds `remaining_sat > 0` wallets (`fold.rs:1170-1187`). The union over `LedgerEvent.wallet` (`event.rs:302`) is the most complete "known wallets" set available (no registry exists), and strictly dominates the old source (it now includes drained/zero-balance wallets that still appear in events). The false "any known wallet is valid" claim is corrected. The residual gap — a wallet that has NEVER appeared in any event — is honestly documented as an acknowledged limitation with the CLI `--to-wallet` escape and a FOLLOWUP (`spec:71-73`, `206-210`). A consciously-scoped, documented limitation with an escape hatch is a legitimate design decision, not an open defect.

### [M1] in-list wallet — RESOLVED ✓
`spec:63-64`: clarified to `LedgerEvent.wallet` (event-level field, not a `TransferIn` payload field), tagged `[R0-M1]`, consistent with `resolve.rs:509` and `open_classify_inbound_flow`.

### [M2] arm-3 example — RESOLVED ✓
`spec:168-170`: now `FmvMissing`-only for the scoped Income/Acquire variants (`UnknownBasisInbound` removed), tagged `[R0-M2]`.

### [M3] DecisionConflict arm framing — RESOLVED ✓
`spec:114-115`: reframed as "effectively unreachable given the exclusive lock + up-front pre-filter; a defensive arm," tagged `[R0-M3]`. Matches `persist.rs:8-11` (no concurrent writer).

### [N1] Unclassified citation — RESOLVED ✓
`spec:132`: `event.rs:80`.

### Remaining
- **[N2] NIT (bookkeeping, non-gating):** the header (`spec:4`) still reads "Review status: DRAFT — awaiting R0 round 1." Update to reflect the round-1 fold / R0-GREEN status. Does not affect the design.

## New-drift sweep (fold introduced no regressions)
- Field lists/types match the structs; `BasisSource` (`event.rs:16`), `FmvStatus` (`event.rs:8`), `IncomeKind` (`event.rs:28`) are all `pub` with the cited variants.
- Substrate anchors unchanged and still correct (dispatch, latch, persist shape, revocability, KAT-G1 `append_` coverage).
- The two confirmed load-bearing invariants from round 1 still hold: a linked TransferOut leaves `pending_reconciliation` (pushed only in `Op::PendingOut`, `fold.rs:729`); both persist fns are single-append with no post-append fallible step (no bespoke latch needed).

**Gate:** 0 Critical / 0 Important → **R0-GREEN. Cleared to proceed to Plan/Implement.** The lone Nit (N2, a stale status header) may be fixed in passing; it does not gate.
