# SPEC + PLAN — `reconcile-allocation-dual-loss-basis` (Slug 1)

**Source baseline:** `origin/main` @ `db9f074` (cycle-prep recon `428f457`, CLEAN — gap confirmed real).
**Goal:** A pre-2025 **received-gift** lot carries a §1015(a) **dual basis** — **gain basis = donor carryover basis; loss basis = FMV-at-gift** (the loss basis applies only when FMV-at-gift < donor basis) — plus §1223(2) tacking (`donor_acquired_at`). Today, electing safe-harbor **Path B** over such a lot **collapses it to single-basis** — the FMV-at-gift loss basis and the tacking are dropped — because `AllocLot` cannot carry them.

> **§1015(a) orientation (canonical — matches the engine `fold.rs:679-680` and `SPEC_foundation.md` TP11/§6.4/§7.4):** `usd_basis` is the GAIN basis = **donor carryover basis**; `dual_loss_basis = Some(FMV-at-gift)` is the LOSS basis, set ONLY when **FMV-at-gift < donor basis**; `donor_acquired_at` tacks the holding period on the GAIN side (§1223(2)). The loss side uses the gift date (no tacking). Extend `AllocLot` + the Path-B seed + the CLI builder so Path B preserves the dual basis, matching Path A (the default, which already preserves it).
**SemVer:** additive public fields on `AllocLot` (a serde struct inside `EventPayload`) ⇒ **MINOR** (pre-1.0). Backward-compatible: `EventPayload` persists as `serde_json` (`persistence.rs:165`), so `#[serde(default)]` makes pre-existing `SafeHarborAllocation` events (without the fields) deserialize to `None`. GUI/manual locksteps: N/A.

## Problem (verified against current source @ db9f074)

- `AllocLot` (`event.rs:145-150`) = `{ wallet, sat, usd_basis, acquired_at }` — **single-basis**; no `dual_loss_basis`/`donor_acquired_at`.
- The Path-B seed (`resolve.rs` `.map(|(i,l)| Lot { … })`, ~`:566-587`) builds each seeded `Lot` with `dual_loss_basis: None, donor_acquired_at: None` — **drops** the dual basis.
- The CLI `safe_harbor_allocate` (`reconcile.rs:234-239`) builds `AllocLot`s from the pre-2025 projection's `residue.lots`, mapping `usd_basis`/`acquired_at` but **dropping** `l.dual_loss_basis`/`l.donor_acquired_at`.
- The §1015(a) four-zone logic in `fold.rs` `make_disposal_legs` already keys on `dual_loss_basis.is_some()` and tacks HP via `donor_acquired_at` — it is **correct**; it just never sees the data under Path B. Path A preserves it (`transition.rs:61-65`, lots moved 1:1).

Net effect: only when a taxpayer **elects Path B over a pre-2025 received-gift lot** is the dual basis lost: a later **loss-zone** disposition then mis-states basis (uses the gain basis = donor carryover instead of the lower **FMV-at-gift** loss basis, understating the loss), and a **gain-zone** disposition loses §1223(2) tacking (term measured from the gift date instead of `donor_acquired_at`). Narrow but a real §1015(a) error.

## Design

### `Lot` (state.rs) — destination, already correct
`Lot` already has `usd_basis` (gain basis), `dual_loss_basis: Option<Usd>`, `donor_acquired_at: Option<TaxDate>` (state.rs:64-67). No change.

### `event.rs` — extend `AllocLot`
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocLot {
    pub wallet: WalletId,
    pub sat: Sat,
    pub usd_basis: Usd,                                   // GAIN basis = donor carryover basis (§1015(a))
    pub acquired_at: TaxDate,                             // gift date = loss-zone HP start (no tacking on the loss side)
    #[serde(default)] pub dual_loss_basis: Option<Usd>,  // §1015(a) LOSS basis = FMV-at-gift; Some only when FMV-at-gift < donor basis; None otherwise
    #[serde(default)] pub donor_acquired_at: Option<TaxDate>, // §1223(2) tacking; gain/no-dual-zone HP start; None otherwise
}
```
`#[serde(default)]` ⇒ old persisted events without these keys deserialize to `None` (backward-compatible; verified `EventPayload` → `serde_json::to_string` at persistence.rs:165).

### `resolve.rs` — Path-B seed preserves the dual basis
In the `a.lots.iter().enumerate().map(|(i, l)| Lot { … })` seed, replace the two `None`s with the AllocLot's values:
```rust
dual_loss_basis: l.dual_loss_basis,
donor_acquired_at: l.donor_acquired_at,
```
**Conservation is UNCHANGED:** the guard sums `Σ l.sat` and `Σ l.usd_basis` (gain basis) only (`resolve.rs:547-549`); `dual_loss_basis` is an *alternative* basis, correctly NOT part of the sat/value conservation identity. So adding it cannot break `SafeHarborUnconservable`.

### `reconcile.rs` — CLI `safe_harbor_allocate` carries the dual basis
In the `residue.lots … .map(|l| AllocLot { … })` builder, add:
```rust
dual_loss_basis: l.dual_loss_basis,
donor_acquired_at: l.donor_acquired_at,
```
(The residue comes from the pre-2025-only projection, so its lots carry the gift dual basis verbatim.)

### Update the 4 `AllocLot` literal sites
Adding fields breaks existing literals. Update all (non-gift → `None, None`):
- `crates/btctax-core/src/event.rs:325` + `:335` (test-fixture literals).
- `crates/btctax-core/tests/transition.rs:102` (the `alloc_lot(...)` helper — add the two fields as `None`, OR extend the helper to accept optional dual basis for the new KAT).
- `crates/btctax-cli/src/cmd/reconcile.rs:234` (the fix site — carries from `l`, above).

### Spec doc
Update the canonical `AllocLot` schema (§6.4 / event-schema section of `design/SPEC_foundation.md`) to list the two new fields, and add a one-line §7.4 note: "Path B preserves a received-gift lot's §1015(a) dual basis and §1223(2) tacking (the allocation carries `dual_loss_basis`/`donor_acquired_at`)."
> **[R0-I1] DO NOT alter the existing §1015(a) labels in `SPEC_foundation.md` (TP11 ~line 34, §6.4 ~124, §7.4 ~128) — they are already correct (gain = donor carryover; loss = FMV-at-gift). The new `AllocLot` field descriptions MUST use those same labels.**

## Plan (TDD)

### Task A — core: `AllocLot` fields + Path-B seed + KAT
- **A1 (tests, `crates/btctax-core/tests/transition.rs` + a serde test):** the dual gift lot used by both KATs (canonical labels): **donor (gain) basis $100, FMV-at-gift $40** (FMV < donor ⇒ dual), gift date 2024-06-01, `donor_acquired_at` 2021-01-01. The `AllocLot` carries `usd_basis: $100`, `dual_loss_basis: Some($40)`, `acquired_at: 2024-06-01`, `donor_acquired_at: Some(2021-01-01)`; allocation made timely (effective Path B).
  - **`path_b_preserves_gift_dual_loss_basis`** (seeding + loss basis): project; assert the seeded `Lot` has `usd_basis == $100`, `dual_loss_basis == Some($40)`, `donor_acquired_at == Some(2021-01-01)`. Dispose post-2025 in the **loss zone** (proceeds $30 < FMV-at-gift $40); assert the loss is computed off the **FMV-at-gift loss basis $40** ⇒ loss $10, NOT the donor/gain basis $100. (Under OLD behavior `dual_loss_basis: None` ⇒ single-basis ⇒ loss $70 — so this fails pre-fix, proving the fix. [R0-I1: loss basis is FMV-at-gift, not donor basis.])
  - **`path_b_preserves_gift_tacking`** (gain-zone term, [R0-I3]): same dual gift lot; dispose post-2025 in the **gain zone** (proceeds $150 > donor/gain basis $100 ⇒ gain $50) on a date **>1yr after `donor_acquired_at` 2021 but <1yr after the gift date 2024-06** (e.g. 2025-03-01). Assert the term is **LONG-TERM** (gain-side HP tacked from `donor_acquired_at`). Under OLD behavior (`donor_acquired_at: None` ⇒ HP from gift date 2024-06) the same disposal is SHORT-TERM — proving `donor_acquired_at` is seeded + consumed. (Note: the loss side does NOT tack — §1223(2) gain-side only — so tacking is proven via the gain zone, not the loss zone.)
  - **`alloc_lot_serde_backward_compat`** (core unit or tests): serialize an `AllocLot` with `Some(..)` dual fields and round-trip (deep-eq); AND deserialize a JSON object that OMITS `dual_loss_basis`/`donor_acquired_at` → both `None` (proves old persisted events stay readable; [R0-N1] `#[serde(default)]` on `Option` is redundant but kept as defensive/explicit).
- **A2 (impl):** add the two `#[serde(default)]` fields to `AllocLot`; set them in the `resolve.rs` Path-B seed from `l`; update the event.rs + transition.rs literal sites; run the new KAT + the full core suite green. Conservation KATs must still pass unchanged.

### Task B — CLI: `safe_harbor_allocate` carries the dual basis + KAT
- **B1 (test, `crates/btctax-cli`):** **`safe_harbor_allocate_carries_gift_dual_basis`**: build a vault whose pre-2025 holdings include a received-gift lot in the **dual case** — import a Coinbase Receive, classify-inbound as `GiftReceived{donor_basis: $100, fmv_at_gift: $40, donor_acquired_at}` (TP11; FMV-at-gift < donor ⇒ dual), gift dated pre-2025 — then `safe_harbor_allocate(ActualPosition, …)`; load the appended `SafeHarborAllocation` and assert its `AllocLot` for that wallet has `usd_basis == $100` (donor/gain basis), `dual_loss_basis == Some($40)` (FMV-at-gift loss basis), and `donor_acquired_at == Some(...)` — not `None`. [R0-I2: engine stores `Some(fmv_at_gift)`, not `Some(donor_basis)`.]
- **B2 (impl):** add the two fields to the `reconcile.rs:234` `AllocLot` builder, carrying `l.dual_loss_basis`/`l.donor_acquired_at`; run the new KAT + the cli suite green.

## Validation gate
`cargo test -p btctax-core -p btctax-cli`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --check`; then full `cargo test --workspace`. (No release-binary surface change, but `cargo build --release --bin btctax` should still pass.)

## Out of scope / notes
- ProRata vs ActualPosition allocation methods are unchanged — both now carry the dual basis through identically.
- No change to conservation, the time-bar, or effectiveness logic — only the basis *content* of seeded lots.
- This only changes outcomes for the narrow case (Path B elected over a pre-2025 received-gift lot, later disposed in the loss zone); Path A behavior is untouched.
