# Crypto-engine understatement audit — ACQUISITION half (basis, lots, pool entry)

**Date:** 2026-09-05 · **Branch:** `feat/schedule-1a-ty2025` · **Scope:** acquisition only
(lot creation, basis at creation, §1012/§1015/§1014, acquisition fees, self-transfer inbound,
pool membership, cross-year carryforward). Disposition and classification were a second agent's half
and are NOT audited here.

**The one question:** what makes btctax compute a cost basis that is TOO HIGH, or a holding pool that
is TOO LARGE?

---

## VERDICT

**Yes — basis can be inflated, and the pool can be doubled, with NO refusal and (in the worst case)
NO advisory of any kind.** Both are reachable from documented, first-class CLI commands, and one of
them is reachable from the *bulk* commands, i.e. at scale in two keystrokes.

| Severity | Count |
|---|---|
| **Critical** | **2** |
| **Important** | **1** |
| Minor | 2 |

All three blocking findings are **machine-verified** by driving `btctax_core::project` directly
(reproduction code inlined below; nothing in the repo was modified). No finding rests on reading alone.

---

## FINDINGS

### C-1 (Critical) — `link-transfer --to-wallet` + `classify-inbound-self-transfer` books the SAME coins twice: pool doubled, basis doubled, ZERO blockers

**The filer.** Anyone who moves coins from an exchange to a self-custody wallet **whose deposits they
also import.** This is the ordinary Bitcoiner. Concretely:

```
btctax reconcile bulk-link-transfer --to-wallet self:cold --year 2025 --yes
btctax reconcile bulk-classify-inbound-self-transfer --year 2025 --yes
```

Both commands are first-class, both are the *bulk* front doors, and the second is the one the tool's
own hard blocker tells the filer to run.

**The mechanism.** A `TransferLink` decides its inbound leg only when the `--to-event` form is used.
The `--to-wallet` form names no in-event, so the matching `TransferIn` is never marked consumed:

`crates/btctax-core/src/project/resolve.rs:852-859` — `consumed_ins` is populated **only** on the
`InEvent` arm:

```rust
if let TransferTarget::InEvent(in_id) = &tl.in_event_or_wallet {
    if consumed_ins.contains(in_id) { ... }
    else if by_id.get(in_id).and_then(|e| e.wallet.as_ref()).is_none() { ... }
    else {
        consumed_ins.insert(in_id.clone());
    }
}
```

`crates/btctax-core/src/project/resolve.rs:331-336` — the wallet arm resolves a destination and
returns, touching no in-event:

```rust
let dest = match target {
    TransferTarget::InEvent(in_id) => by_id.get(in_id).and_then(|e| e.wallet.clone()),
    TransferTarget::Wallet(w) => Some(w.clone()),
};
```

`crates/btctax-cli/src/cmd/inspect.rs:76-83` states the consequence in a comment and treats it as
benign:

```rust
// A link decides its outbound leg, AND — when `--to-event` was used — the inbound
// leg it relocates onto (resolve.rs consumes that TransferIn). A `--to-wallet` link
// has no in-event to decide.
```

So the outbound leg **relocates** the real lot into `self:cold` (`fold.rs:918-947`, carrying basis and
holding period), and the inbound `TransferIn` is left carrying a **Hard `UnknownBasisInbound`**
(`fold.rs:983-990`). That blocker is exactly the selection key of the bulk classify path —
`crates/btctax-cli/src/session.rs:883-886`:

```rust
for b in &state.blockers {
    if b.kind != BlockerKind::UnknownBasisInbound { continue; }
```

— so the tool actively routes the filer into classifying the coins that were *already* relocated.
`Op::SelfTransferInbound` then creates a **fresh origin lot** for them
(`crates/btctax-core/src/project/fold.rs:1180-1200`, `pools.new_origin_lot(...)`, `stats.sigma_in += *sat`).

`bulk-link-transfer` has no `--to-event` form at all — `crates/btctax-cli/src/cmd/reconcile.rs:594-598`
hard-codes the wallet target for every row it writes:

```rust
let payload = EventPayload::TransferLink(TransferLink {
    out_event: out_event.clone(),
    in_event_or_wallet: TransferTarget::Wallet(dest.clone()),
});
```

**Measured (1.0 BTC bought for $50,000, moved once to cold storage):**

| variant | lots | held sat | Σ basis | FR9 `balanced` | blockers |
|---|---|---|---|---|---|
| A `--to-event` (correct form) | 1 | 100,000,000 | $50,000 | true | none |
| B `--to-wallet` only | 1 | 100,000,000 | $50,000 | true | Hard `UnknownBasisInbound` |
| **C `--to-wallet` + bulk classify ($0)** | **2** | **200,000,000** | $50,000 | true | 2 advisories, both saying the basis is *conservative* |
| **D `--to-wallet` + `--basis 50000`** | **2** | **200,000,000** | **$100,000** | **true** | **NONE** |

**Direction.** Variant D is the pure understatement case: basis is exactly doubled and the ledger is
*completely silent* — no Hard, no advisory, and `conservation_report` reports `balanced: true`.
Variant C doubles the pool (2 BTC held where 1 exists) and its only two blockers *reassure* the filer
that the treatment is conservative.

**Why FR9 does not catch it.** `crates/btctax-core/src/project/conservation.rs:61-62`:

```rust
let balanced = !has_uncovered
    && sigma_in == sigma_disposed + sigma_removed + sigma_held + sigma_fee_sats + sigma_pending;
```

The phantom lot increments `sigma_in` **and** `sigma_held` by the same amount, so the identity is
structurally incapable of detecting a phantom acquisition. And there is **no Σbasis conservation
invariant anywhere** outside the safe-harbor check (`resolve.rs:1513`) — the doubled $50,000 has no
instrument watching it.

**Smallest fix.** Two parts, either of which alone closes the measured case:

1. *Record time (cheap, testable):* refuse `link-transfer --to-wallet <w>` / `bulk-link-transfer
   --to-wallet <w>` when `<w>` is a **tracked** wallet (one that has imported `TransferIn` events),
   pointing at `--to-event` / `reconcile self-transfer-match`. `--to-wallet` is only sound for an
   *untracked* destination; that is its whole purpose.
2. *Engine (the guarantee):* in `resolve`, when a live `TransferLink{Wallet(w)}` coexists with a
   `ClassifyInbound{SelfTransferMine}` on a `TransferIn` into `w` with the same `sat` within a small
   window, raise a Hard `DecisionConflict` ("both legs of one movement are booked — the inbound would
   double the coins").

**B1 kill-test:** variant D above (2 lots / 200,000,000 sat / $100,000 with zero blockers) is the
planted defect the checker must red on.

---

### C-2 (Critical) — §1015(a) dual-basis loss limitation is skipped whenever the donor's basis is reconstructed from his acquisition date

**The filer.** Alice's father bought 1 BTC on 2021-11-09 (~$67,000) and kept no receipt, but remembers
the date. He gifts it to her on 2022-12-15 when BTC is ~$17,000. She records:

```
btctax reconcile classify-inbound-gift <in> --fmv-at-gift 17000 --donor-acquired 2021-11-09
```

She sells in 2025 for $40,000.

**The mechanism.** `crates/btctax-core/src/project/fold.rs:1071-1088`. Case 2 (donor basis *known*)
correctly builds the dual basis; Case 3 (donor basis *reconstructed*) returns `None` for the loss
basis:

```rust
    } else {
        // Case 2: FMV < donor basis — dual: gain basis = donor basis, loss basis = FMV.
        (*b, Some(*fmv_at_gift), BasisSource::GiftCarryover, false)
    }
}
None => match donor_acquired_at {
    Some(d) => {
        // Case 3: GiftFmvFallback — derive basis from BTC price at donor's acquisition date.
        match fmv_of(prices, *d, *sat) {
            Some(fmv) => (fmv, None, BasisSource::GiftFmvFallback, false),
```

`dual_loss_basis: None` ⇒ `Consumed.dual == false` (`pools.rs:221`) ⇒ `make_disposal_legs` takes the
simple single-basis branch (`fold.rs:160`) and the four-zone §1015(a) logic never runs.

**The authority is unambiguous and is in-repo.** `legal/primary-sources/statute-irc/26USC_s1015.html`,
§1015(a): the reconstructed figure *becomes* the donor's basis —

> "If the Secretary finds it impossible to obtain such facts, **the basis in the hands of such donor or
> last preceding owner shall be the fair market value** of such property … as of the date or approximate
> date at which … such property was acquired by such donor"

and the loss limitation in the same subsection then binds it:

> "except that if such basis … **is greater than the fair market value of the property at the time of the
> gift, then for the purpose of determining loss the basis shall be such fair market value.**"

Deriving basis from the donor's acquisition-date close is therefore *correct* (that half is right); what
is missing is that the result is still subject to the loss cap.

**Measured — identical economic facts, two paths:**

| donor basis | engine basis | proceeds | reported gain | zone |
|---|---|---|---|---|
| `Some($67,000)` (control) | $40,000 | $40,000 | **$0.00** | `NoGainNoLoss` ✅ |
| `None` + `--donor-acquired 2021-11-09` | **$67,000** | $40,000 | **−$27,000** | `None` ❌ |

**Direction.** A fabricated **$27,000 capital loss** where the statute yields $0 gain — a $27,000 swing
on one gift, and the deduction is fully available against other gains. The two rows are the same
transaction; only how much the donor remembered differs.

**No guard exists.** The one KAT on this path,
`crates/btctax-core/tests/kat_tax.rs:892-911` (`tp11_unknown_donor_basis_uses_fmv_at_donor_acquisition_date`),
uses `fmv_at_gift = $60` against a derived basis of `$28` — i.e. an appreciated gift, the case where the
loss cap is inert. The failing branch has never been exercised.

**Smallest fix.** In Case 3, apply the same test Case 2 applies, to the reconstructed figure:

```rust
Some(fmv) => (
    fmv,
    (*fmv_at_gift < fmv).then_some(*fmv_at_gift),   // §1015(a) loss cap binds the substituted basis too
    BasisSource::GiftFmvFallback,
    false,
),
```

**B1 kill-test:** row 2 of the table above — assert `gift_zone == Some(NoGainNoLoss)` and `gain == 0`,
and confirm it reds on today's code with `−27000.00`.

---

### I-1 (Important) — the "sats typed into a dollars field" guard was built, and then applied only to the field whose error OVERSTATES tax

**The filer.** Anyone who types `5000000` (the sats count) where dollars are wanted — the exact error
the repo already decided was worth a guard.

**The mechanism.** `crates/btctax-cli/src/cmd/reconcile.rs:196-214` implements `amount_fmv_advisory`
(>100× the date's market value ⇒ "did you enter the sats amount as dollars?"), with three unit tests
including a strict-boundary test. It has **exactly one call site**, `reconcile.rs:173`, inside
`reclassify_outflow` — i.e. on `--amount`, the **proceeds/FMV**. An inflated proceeds figure
*overstates* tax.

The mirror-image fields — the ones where an inflated figure **understates** tax — get only a sign check:

- `crates/btctax-cli/src/main.rs:1208-1211` — `classify-inbound-self-transfer --basis`
- `crates/btctax-cli/src/main.rs:1190-1193` — `classify-inbound-gift --donor-basis`

both via `parse_nonneg_usd_arg` (`crates/btctax-cli/src/eventref.rs:88-96`), whose entire contract is:

```rust
if v < Decimal::ZERO {
    return Err(CliError::Usage(format!(
        "{field} must be >= 0 (got {v}); no legitimate negative cost basis / FMV / fee / price exists"
    )));
}
```

So `--basis 5000000` on a 0.05 BTC deposit records a **$5,000,000 basis** in silence, while
`--amount 5000000` on the same 0.05 BTC draws a warning.

**Direction.** Understatement, unbounded, and it is the *same typo* the codebase already recognises —
guarded in the direction that costs the filer money, unguarded in the direction that costs the Treasury.

**Smallest fix.** Call `amount_fmv_advisory` (renamed, or a `usd_field_fmv_advisory`) from
`classify_inbound` for `--basis`, `--donor-basis` and `--fmv-at-gift`, yardsticked to the *event date*
close × the event's sats. Pure function, already tested; this is a call site, not new logic.

**B1 kill-test:** `classify-inbound-self-transfer --basis 5000000` on a 0.05 BTC inbound must print the
warning; assert the current code prints nothing.

---

### M-1 (Minor) — no §1014 inbound classification exists; inherited coins have no honest home

`InboundClass` (`crates/btctax-core/src/event.rs:143-172`) offers `Income`, `GiftReceived`, and
`SelfTransferMine`. There is no `Inherited { date_of_death, dod_fmv }`, and §1014 appears in the codebase
only as advisory *copy* (`crates/btctax-core/src/conservative.rs:516-522`) telling the filer their
inherited coins get a date-of-death FMV basis with no cost records needed. There is no statute file for
§1014 (`legal/primary-sources/statute-irc/` holds §1, 61, 170, 1001, 1011, 1012, 1015, 1016, 1031, 1091,
1211, 1212, 1221, 1222, 1223, 1411 — **no 1014**).

The filer who follows that advice must improvise: `GiftReceived` with `donor_basis` = DOD FMV (which
also mis-labels provenance as **Gift** on Form 8283 — `crates/btctax-core/src/forms.rs:274`, and wrongly
subjects it to the §1015 loss cap), or `SelfTransferMine --basis`. Both are large, unguarded,
filer-typed basis figures reached by inference rather than by an instruction. Not itself an inflation
bug, but it is the highest-value un-modelled acquisition path and it interacts directly with I-1.

*Owning phase: whichever cycle next touches `InboundClass`.*

### M-2 (Minor) — the Path-A transition erases gift provenance from pre-2025 lots

`crates/btctax-core/src/project/transition.rs:101-103` overwrites `basis_source` for every lot except
`EstimatedConservative`:

```rust
if lot.basis_source != BasisSource::EstimatedConservative {
    lot.basis_source = BasisSource::ReconstructedPerWallet;
}
```

Confirmed in probe F above: a 2022 gift disposed in 2025 reports `basis_source: ReconstructedPerWallet`,
not `GiftCarryover`/`GiftFmvFallback`. `forms.rs:274` maps those two to Form 8283 "How acquired = Gift";
after the transition that mapping can no longer fire for any pre-2025 gift. Provenance loss, not a basis
error — but it is the same D-8 exemption pattern that was already found necessary once for tranches, and
the gift tags need it for the same reason.

---

## CLEARED — checked, found sound

- **§1012 acquisition-fee capitalisation.** `fold.rs:680` — `usd_basis = a.usd_cost + a.fee_usd`. All
  four adapters supply a fee-*exclusive* cost with the fee alongside: Coinbase `Subtotal` + `Fees`
  (`coinbase.rs:151-155`), Swan `Sent Quantity` + `Fee Amount` (`swan.rs:190-197`) and
  `Transaction USD` + `Fee USD` (`swan.rs:244-252`), River `Sent Amount` + `Fee Amount`
  (`river.rs:137-142`), Gemini `|USD Amount USD|` + `|Fee (USD) USD|` (`gemini.rs:144-150`). No
  double-count. *Caveat:* the River/Swan convention is pinned only by a synthetic fixture
  (`crates/btctax-adapters/tests/river.rs:10-11,41-42`), not by a real export.
- **Pool arithmetic conserves basis exactly.** `pools.rs:208-234` (`take_from`) — `split_pro_rata`
  the consumed share, subtract it from the lot, decrement `remaining_sat`; `consume_ordered:200`
  retains only `remaining_sat > 0`. No lot can be consumed twice or survive consumption.
- **`Consumed`/relocation basis carry.** `fold.rs:918-947` — a `SelfTransfer` relocation carries
  `gain_basis`, `loss_basis`, both HP starts and `donor_acquired_at`; source lot is drawn down first.
  Σbasis conserved; not a lot-duplication site (the C-1 duplicate is a *separate* origin lot, not a
  relocation defect).
- **TP8(c) fee treatment.** `fold.rs:369-460` — fee sats are consumed from the pool and their basis
  re-homed onto the survivor, so total basis is conserved rather than increased; the promoted-tranche
  estimate share is *withheld* (forfeited), which is conservative. Owner-mandated policy, correctly
  implemented.
- **Safe-harbor Path B (Rev. Proc. 2024-28) conserves BOTH sat and basis.** `resolve.rs:1502-1514` —
  `alloc_sat != snap.held_sat || alloc_basis != snap.basis` ⇒ Hard `SafeHarborUnconservable`, and
  attestation cannot bypass it. A filer cannot allocate basis he does not have. This is the one place
  in the engine where a Σbasis invariant exists, and it is correct.
- **Duplicate `ClassifyInbound` on one `TransferIn`.** `resolve.rs:882-899` — first-wins plus a Hard
  `DecisionConflict`; the second decision is excluded, so no second lot. Same for duplicate
  `TransferLink` on an out-event (`resolve.rs:829-836`) and on an in-event (`resolve.rs:841-849`).
- **Void semantics.** A voided decision is skipped before any map is built (`resolve.rs:823-826`), so a
  voided classification leaves no lot behind — no resurrection path found.
- **`ImportConflict` accept-first** overwrites the payload for the *same* `EventId`
  (`resolve.rs:697-733`); it cannot add a lot alongside the original.
- **Income / mining / staking / airdrop lots** are created at FMV-at-receipt with a matching
  `IncomeRecord` (`fold.rs:775-832`, `fold.rs:991-1047`); a missing FMV yields `basis_pending: true` +
  Hard `FmvMissing` rather than a fabricated basis. Correct per Rev. Rul. 2019-24 / Notice 2014-21.
- **`hifo_cmp` zero-basis special case** (`pools.rs:275-287`) is a no-op for genuine $0 lots (a $0 lot
  is already lowest-basis and sorts last either way) — not a mis-ordering vector.
- **`DeclareTranche` enlarging the pool** is correct by design: it files at $0 basis
  (`event.rs:271-283`), is disclosed on Form 8275, and the `PromoteTranche` floor is clamped so it can
  never manufacture a loss (`conservative_promote.rs:154-193`). Not an understatement vector.
- **The only lot-construction sites in the shipping engine** are `fold.rs` (Acquire / Income /
  IncomeInbound / GiftReceived / SelfTransferInbound / SelfTransfer-relocate), `transition.rs:96-120`
  (Path A / Path B seed) and `resolve.rs:1541-1559` (safe-harbor seed). The `Lot {}` / `AllocLot {}`
  builders in `optimize.rs:1530,1613` are inside `#[cfg(test)]` (module at `optimize.rs:1485`). No
  other code path can put a lot in a pool.
- **Long-term acquisition default for inbound self-transfers** (`fold.rs:1156`,
  `long_term_default_acquired(date)`) resolves in the filer's favour (long-term rate) — but it is
  **explicitly owner-mandated**, with the tax direction stated honestly, in
  `design/SPEC_reconcile_defaults.md:10-20` ("both REDUCE the estimate … Revises the prior conservative
  short-term policy — intended"), and it is disclosed at fold time by
  `SelfTransferInboundDefaultedAcquired`. Recorded, not filed.

**Basis-too-LOW (opposite direction, not my target — one line as briefed):** the conservative defaults
are consistent and hold. `Op::UnknownInbound` creates *no* lot (`fold.rs:983-990`), a gift with neither
donor basis nor date lands at $0 + Hard blocker (`fold.rs:1097-1105`), `GiftFmvFallback` with no price
at the donor date lands at $0 + Hard blocker (`fold.rs:1088-1096`), and a promoted tranche's fee-draw
estimate share evaporates rather than migrating (`fold.rs:420-436`). All overstate tax; none gate
incorrectly.

---

## REPRODUCTION

Standalone, no repo modification — a scratch crate depending on `btctax-core` by path:

```rust
// C-1, variant D — basis doubled, zero blockers
let evs = vec![
    imp("buy1", datetime!(2025-02-01 12:00 UTC), Some(ex()),  Acquire{ sat: 100_000_000,
        usd_cost: dec!(50000), fee_usd: dec!(0), basis_source: ExchangeProvided }),
    imp("out1", datetime!(2025-03-01 12:00 UTC), Some(ex()),  TransferOut{ sat: 100_000_000, .. }),
    imp("in1",  datetime!(2025-03-01 13:00 UTC), Some(cold()),TransferIn { sat: 100_000_000, .. }),
    dec_ev(1, .., TransferLink { out_event: out1, in_event_or_wallet: TransferTarget::Wallet(cold()) }),
    dec_ev(2, .., ClassifyInbound { transfer_in_event: in1,
        as_: SelfTransferMine { basis: Some(dec!(50000)), acquired_at: Some(date!(2025-02-01)) } }),
];
// => lots=2  held_sat=200000000  Σbasis=100000.00  FR9 balanced=true  blockers: NONE

// C-2 — §1015(a) loss cap skipped
//   GiftReceived { donor_basis: None, donor_acquired_at: Some(2021-11-09), fmv_at_gift: 17000 }
//   price[2021-11-09] = 67000 ; sell 1 BTC in 2025 for 40000
// => basis=67000.00  gain=-27000.00  zone=None
//   (control: donor_basis: Some(67000) => basis=40000.00  gain=0.00  zone=NoGainNoLoss)
```

---

## WHAT I DID NOT READ

- **Disposition and classification** — `make_disposal_legs`' four-zone gain arithmetic beyond what C-2
  required, `make_removal_legs`, `Op::Dispose`/`GiftOut`/`Donate`, `LotSelection` §A.4 validation,
  method elections §A.5(a), `compliance.rs`, `donation.rs`, `optimize.rs`, `whatif.rs`. Second agent's
  half by assignment.
- **`crates/btctax-core/src/tax/**`** (~30 files) — form/return computation, entirely downstream of the
  lot pool.
- **`crates/btctax-tui-edit/` (45,741 LOC)** — I confirmed it constructs the same `TransferLink
  { Wallet(dest) }` payload at `edit/persist.rs:640`, so C-1 is reachable from the TUI too, but I did
  not audit its flows.
- **`defensive/`, `experimental.rs`, `scrub*.rs`, `persistence.rs`, `btctax-store`** — no lot
  construction (verified by grep), not read further.
- **Adapter parsing internals** beyond the four `Acquire` construction sites and their `usd_cost`/
  `fee_usd` semantics — `read.rs`, `parse.rs`, `normalize.rs`, price-dataset loading not read.
- **Real exchange exports.** The fee-exclusivity assumption behind the CLEARED §1012 item is pinned by
  in-repo synthetic fixtures only; I had no real River/Swan CSV to check against.
- **No test suite was run** (read-only audit); the three blocking findings were verified by direct
  library execution instead.
