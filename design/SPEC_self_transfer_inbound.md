# SPEC — self-transfer completion, Cycle A: inbound self-transfer-in

**Source baseline:** `main` @ `a740b3d` (all anchors verified at write time).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-self-transfer-inbound-round-{1,2}.md` (round 1: 0C/0I/3M/3N — all tax-correctness
invariants verified; round 2: 0C/0I/0M/0N, no drift). Cleared to implement.**
**Design lineage:** brainstorm with the user (2026-07-03) → architect design (grounded at `a740b3d`).
First half of the "self-transfer completion" program. **Cycle B (matched in/out pairs — the
`SelfTransferPassthrough` drop primitive + the confirmed matcher) is OUT OF SCOPE here** — its own cycle.

**The gap.** A `TransferIn` (`event.rs:73`) that is the receiving side of a self-transfer whose matching
withdrawal was NEVER imported (e.g. the user's largest transaction — 50 BTC into Coinbase from their own
un-imported external wallet) has no correct classification: it is not `Income` and not `GiftReceived`, so
it projects to `Op::UnknownInbound` → **Hard `UnknownBasisInbound` blocker, NO lot created**
(`resolve.rs:279`, `fold.rs:815-822`). This cycle adds the fourth path: classify such an inbound as
**"my own coins" (self-transfer-in)** — a non-taxable receipt that creates a fresh lot.

**Why it's not the existing out-side self-transfer.** `Op::SelfTransfer` (`fold.rs:742`) RELOCATES a lot
the source pool already holds. The unmatched inbound has NO source lot, so the new op must CREATE a fresh
origin lot (like `Op::IncomeInbound`/`GiftReceived` via `pools.new_origin_lot`), not relocate one.

---

## User-mandated tax semantics (REQUIREMENTS — settled in dialogue; do not re-litigate)

1. **Non-taxable.** No income recognized, nothing on any form, not reported to the IRS. (Consistent with
   the standing self-transfer = non-taxable / basis-carries mandate.)
2. **Basis defaults to $0** when unspecified — the CONSERVATIVE direction (max gain when later sold →
   NEVER under-reports; FMV-at-receipt would under-report and is REJECTED). **Optionally set** to real
   cost. An **Advisory** honest flag fires ONLY when basis was defaulted (`None`), not when the user
   supplies a value (including an explicit `Some(0)`).
3. **Holding period:** acquisition date defaults to the RECEIPT date (⇒ short-term until proven — again
   conservative), **optionally set** to the real original acquisition date (may make the lot long-term).
4. **Outside FIFO/HIFO/LIFO.** A self-transfer is not a disposition; it selects/consumes no lots. The new
   op is lot-CREATING, so it is neither a disposition nor method-honoring → outside lot-selection by
   construction (no `is_disposition_op`/`honoring_principal` change).
5. **NEVER gates the return.** A $0 basis is a *computable* value, so `basis_pending: false` and a later
   disposal computes a real gain with NO `FmvMissing` gate. The honesty flag is a separate **Advisory**
   blocker — it must NOT gate `compute_tax_year`. **(This is the single easiest invariant to break by
   copy-pasting the `IncomeInbound` arm — see Gotcha G1.)**

---

## SemVer / lockstep

- **btctax-core:** additive enum-variant additions — `InboundClass::SelfTransferMine`,
  `BasisSource::SelfTransferInbound`, `Op::SelfTransferInbound`, `BlockerKind::SelfTransferInboundZeroBasis`.
  Consumers are all in-workspace; exhaustive matches gain arms → a **workspace lockstep rebuild**, not an
  external break. Bump all six `0.1.0` crates together.
- **Serialized vault (forward-only):** a vault CONTAINING a `SelfTransferMine` classification fails to load
  on a pre-feature binary (serde unknown-variant) — the identical accepted trade-off as every prior
  decision addition (cf. the `ReclassifyIncome` old-binary note, `event.rs`). Every EXISTING vault (no new
  variant) loads unchanged on the new binary — `Income`/`GiftReceived` serializations untouched.
- **No lockstep mirror:** verified NO `docs/manual/` and NO GUI crate exist. Nothing to mirror.
- **No new `EventPayload` variant / no fingerprint change** — Cycle A rides the EXISTING `ClassifyInbound`
  decision, so no fingerprint KAT is needed (Cycle B's `SelfTransferPassthrough` will need one).

---

## Grounding (verified at `a740b3d`)

- `InboundClass` (`event.rs:123`), `ClassifyInbound` (`event.rs:136`, `EventPayload::ClassifyInbound`),
  `BasisSource` (`event.rs:17`, 8 variants, none zero/unsubstantiated).
- `build_op` inbound_class match: `Income`→`Op::IncomeInbound` (`resolve.rs:257-266`),
  `GiftReceived`→`Op::GiftReceived` (`:267-277`), else `Op::UnknownInbound` (`:279`).
- fold arms: `Op::IncomeInbound` (`fold.rs:823` — income lot at FMV + IncomeRecord, `basis_pending` at
  `:875`, `new_origin_lot` `:877`, `sigma_in` `:878`); `Op::UnknownInbound` (`:815` — Hard
  `UnknownBasisInbound`, no lot); the Acquire-style clean lot at `:564` (`basis_pending: false`,
  `new_origin_lot` `:566`, `sigma_in` `:567`) is the closest CREATE-a-clean-lot template.
- `BlockerKind` (`state.rs:23`); Advisory severity arm (`state.rs:78-79`:
  `SafeHarborTimebar | UnmatchedOutflows | Pre2025MethodNote | QualifiedAppraisalNote`).
- Reuse-unchanged: `ClassifyInbound` pass-1e collection / duplicate-first-wins / bad-target
  (`resolve.rs:529-579`); CLI `classify_inbound` (`reconcile.rs:38-52`); TUI `persist_classify_inbound`
  (`persist.rs:122-137`); `open_classify_inbound_flow` (filters `UnknownBasisInbound` raw-TransferIn).

---

## C1 — `InboundClass::SelfTransferMine` (rides the existing `ClassifyInbound`)

Add a third variant to `InboundClass` (`event.rs:123`). **NO new `EventPayload` decision** — it is carried
by the existing `ClassifyInbound`, so the entire collection/validation/persist path is reused unchanged.
```rust
SelfTransferMine {
    /// Basis of the returning coins. None ⇒ default $0 (conservative) AND the zero-basis advisory fires.
    /// Some(v) ⇒ user-supplied real cost, NO advisory. Some(0) (attested zero-cost) is honored WITHOUT
    /// the advisory — the flag keys on None, not the numeric value.
    #[serde(default)] basis: Option<Usd>,
    /// Original acquisition date. None ⇒ default = receipt date (short-term). Some(d) ⇒ real date.
    #[serde(default)] acquired_at: Option<TaxDate>,
},
```
(`#[serde(default)]` is hygiene, not back-compat — a new variant has no legacy records. Name:
`SelfTransferMine`; `SelfTransferIn` is an acceptable alt.)

## C2 — `BasisSource::SelfTransferInbound`

One new variant (`event.rs:17`). Used for the created lot whether basis was defaulted or supplied — the
defaulted-vs-supplied signal is carried by the Advisory flag (C5), not a second basis_source.

## C3 — `Op::SelfTransferInbound` + `build_op` wiring

Add to `enum Op` (`resolve.rs`), mirroring `Op::GiftReceived` but simpler (no dual-basis):
```rust
/// ClassifyInbound::SelfTransferMine on a TransferIn: CREATES a new non-taxable origin lot
/// (basis default 0, acquired_at default = receipt date). NOT a relocation — no source lot exists.
SelfTransferInbound { sat: Sat, basis: Option<Usd>, acquired_at: Option<TaxDate> },
```
Wire the third arm in `build_op`'s `inbound_class` match (`resolve.rs:255-277`):
```rust
InboundClass::SelfTransferMine { basis, acquired_at } =>
    Op::SelfTransferInbound { sat: t.sat, basis: *basis, acquired_at: *acquired_at },
```
**No change to `is_disposition_op` / `honoring_principal`** — a lot-creating op is neither, so it is
outside FIFO/HIFO/LIFO and cannot be targeted by a `LotSelection` automatically (Req #4).

## C4 — `fold.rs` arm: create a non-taxable lot + the honest flag (THE HEART)

New arm in `fold_event`, modeled on `Op::IncomeInbound` (`fold.rs:823`) but **non-taxable** and **never
basis-pending**:
```rust
Op::SelfTransferInbound { sat, basis, acquired_at } => {
    let wallet = /* eff.wallet, else: nowhere to create the lot → emit Hard BlockerKind::UnknownBasisInbound
                    with a self-transfer message + return. [R0-M2] Do NOT copy the IncomeInbound guard
                    (fold.rs:830) verbatim — it emits FmvMissing "income inbound without wallet",
                    semantically wrong for a non-income self-transfer. */;
    let usd_basis = basis.unwrap_or(Usd::ZERO);   // conservative $0 default
    let acq = acquired_at.unwrap_or(date);        // conservative receipt-date default (date = event date)
    if basis.is_none() {
        st.add_blocker(BlockerKind::SelfTransferInboundZeroBasis, Some(eff.id.clone()),
            "basis defaulted to $0 — likely overstates your eventual gain; supply real cost if you have \
             it (btctax reconcile classify-inbound-self-transfer --basis). [R0-N1] Holding period also \
             defaults to the receipt \
             date (short-term) unless --acquired is supplied.");   // ADVISORY only; NEVER Hard
    }
    let lot = Lot {
        lot_id: LotId { origin_event_id: eff.id.clone(), split_sequence: 0 },
        wallet: wallet.clone(),
        acquired_at: acq,                 // HP start; gain_hp_start() == acq (no tacking / donor date)
        original_sat: *sat, remaining_sat: *sat,
        usd_basis,
        basis_source: BasisSource::SelfTransferInbound,
        dual_loss_basis: None,
        donor_acquired_at: None,          // NOT a gift — it's your own coin
        basis_pending: false,             // CRITICAL: computable, NEVER gated (contrast Income-FMV-missing)
    };
    pools.new_origin_lot(pool_key(date, &wallet), lot);
    stats.sigma_in += *sat;               // FR9 Σin: coins enter the ledger (externally-sourced)
}
```
Load-bearing points (each contrasts an existing path):
- **Non-taxable:** unlike `IncomeInbound` (`fold.rs:843-850`), pushes NO `IncomeRecord` — nothing to
  `income_recognized`, nothing on Schedule 1/SE. (Req #1, #4.)
- **`basis_pending: false` even at $0** — the sharp distinction from `Income`-FMV-missing (`fold.rs:671`)
  and `GiftReceived` case-4 (`fold.rs:929`), which set `true` and thereby GATE the eventual disposal
  (`make_disposal_legs` → `FmvMissing`, `fold.rs:138`). A $0 basis is computable/conservative → must NOT
  gate. (Req #5. **Gotcha G1.**)
- **`pool_key(date, …)` keys on the RECEIPT date** while `acquired_at` uses supplied-or-receipt —
  orthogonal (mirrors the gift path). A 2026 receipt with a real 2013 `acquired_at` lands in the 2026
  Wallet pool with a 2013 acquisition (long-term). No §7.4 transition breakage.
- **`sigma_in += sat`** keeps FR9 conservation balanced (the new lot's `remaining_sat` enters
  `sigma_held`, matched by `sigma_in`).

## C5 — `BlockerKind::SelfTransferInboundZeroBasis` (Advisory)

Add to `BlockerKind` (`state.rs:23`) and to the **Advisory** arm of `severity()` (`state.rs:78-79`).
Surfaces through the existing advisory bucket in verify/render — no render change. Fires ONLY when
`basis == None`; when a real basis is supplied, no blocker (clean). **Never Hard** (contrast
`UnknownInbound`, Hard because it creates no lot).

## CLI — `reconcile classify-inbound-self-transfer`

New `Reconcile` subcommand (sibling of `ClassifyInboundIncome`/`ClassifyInboundGift`, `main.rs:218-236`):
```
reconcile classify-inbound-self-transfer <in_ref> [--basis <USD>] [--acquired <YYYY-MM-DD>]
```
Dispatch builds `InboundClass::SelfTransferMine { basis, acquired_at }` (parse via existing
`eventref::parse_usd_arg` / date parser) and calls the UNCHANGED `cmd::reconcile::classify_inbound`
(`reconcile.rs:38-52`). Zero new emitter code.

## TUI — extend the classify-inbound single-item flow

`open_classify_inbound_flow` already lists exactly the right targets (`UnknownBasisInbound` blockers whose
event is a raw `TransferIn` with no non-voided `ClassifyInbound`). Add:
- `SelfTransferMine` to `InboundVariant` (`form.rs:327`) + a `SelfTransferForm { item, basis_buf,
  acquired_buf, focus, error }` step in `ClassifyInboundStep` (`form.rs:333`).
- `validate_classify_inbound_self_transfer(basis_buf, acquired_buf) -> Result<InboundClass, String>`
  beside the income/gift validators (`form.rs:412-449`): both fields optional; empty → `None`;
  whitespace-only is NOT empty (the shipped [R0-M4] rule).
- Draw arm in `draw_classify_inbound_form` (`draw_edit.rs:591`); persistence reuses
  `persist_classify_inbound` (`persist.rs:122-137`) unchanged (the MODAL STATE `ClassifyInboundModalState`
  is reused — `as_: InboundClass` covers the new variant).
- **[R0-M3] Enumerate the exhaustive-match sites Task 3 must add an arm to** (all compile-forced, not
  silent): `draw_classify_inbound_modal` (`draw_edit.rs:728`, the modal RENDER fn — a new arm, distinct
  from the reused modal STATE), the `cls_desc` status match (`main.rs:2193`), the variant Tab-cycle
  (`main.rs:769`), the variant→form transition (`main.rs:783`), the step-index helper (`main.rs:698`),
  and the VariantPicker/step draw arms (inside `draw_classify_inbound_form` `:591`). **[R0-r2]** ALSO wire
  the `SelfTransferForm` KEY-HANDLER (the `handle_ci_*` branches are `if let`, NOT compile-forced — a
  missed handler is a silent dead step, caught by the Task-3 E2E).

---

## Invariants (KAT-pinned; the load-bearing guarantees)
1. Conservative basis: `{basis:None}` → lot `usd_basis == 0`, `basis_source == SelfTransferInbound`; later
   Sell at P → gain == P (max gain).
2. Adjustable: `{basis:Some(v)}` → basis v, NO advisory; `{basis:Some(0)}` → basis 0, NO advisory (flag
   keys on `None`).
3. Conservative HP: `{acquired_at:None}` on date D → `acquired_at == D` → a <1yr-later disposal is
   Short-Term; `{acquired_at:Some(2013-…)}` on a 2026 receipt → Long-Term AND lot in the 2026 Wallet pool
   (pool/HP orthogonality).
4. Non-taxable: `income_recognized` empty for the event; no Disposal/Removal; nothing on forms.
5. **Never gated / never basis_pending:** `{basis:None}` → `basis_pending == false`; a later disposal
   computes gain with NO `FmvMissing` gate.
6. Honest flag is Advisory: `SelfTransferInboundZeroBasis.severity() == Advisory`; a vault with only this
   blocker still `compute_tax_year`s.
7. Outside FIFO: a `LotSelection` targeting the self-transfer-in event → `LotSelectionInvalid`; the created
   lot DOES participate normally in FIFO/HIFO/LIFO when later SOLD.
8. FR9 conservation: `sigma_in` increments by the received sats; `conservation_report(...).balanced`.

## KATs
- **btctax-core:** the 8 invariants above (basis/HP defaults + adjust; non-taxable; basis_pending-false +
  no-gate; advisory-not-hard + non-gating; outside-FIFO + sellable; conservation). Plus: serde
  round-trips the new variant; duplicate `ClassifyInbound` first-wins still holds; void re-exposes the
  inbound as `UnknownInbound`. **[R0-N2]** a PRE-2025 receipt self-transfer-in conserves + folds through
  the Universal pool (`universal_snapshot`) correctly; the **wallet-missing corner** [R0-M2] emits Hard
  `UnknownBasisInbound` (not `FmvMissing`) + creates no lot.
- **btctax-cli:** `classify-inbound-self-transfer` appends `SelfTransferMine` (defaults None; with
  `--basis`/`--acquired`); wrong-target (non-TransferIn) errors via the existing bad-target path.
- **btctax-tui-edit:** the flow lists the target, the validator (optional fields; whitespace≠empty), the
  persist strict-prefix, cancel-bytes-unchanged, save-error rollback; E2E: classify a raw `TransferIn` as
  self-transfer-in → lot created, `UnknownBasisInbound` cleared, `SelfTransferInboundZeroBasis` advisory
  present when no basis.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)
- **Task 1 — core** (`InboundClass::SelfTransferMine` + `BasisSource::SelfTransferInbound` + `Op::
  SelfTransferInbound` + `build_op` arm + the `fold.rs` non-taxable-lot arm + `BlockerKind::
  SelfTransferInboundZeroBasis` + severity; the 8 core invariants). The heart + highest risk.
- **Task 2 — CLI** (`classify-inbound-self-transfer` subcommand + dispatch; CLI KATs).
- **Task 3 — TUI single-item flow** (variant + form + validator + draw; reuse modal/persist; TUI KATs + E2E).
- **Task 4 — whole-diff review (Phase E) + FOLLOWUPS** (record Cycle B — matched pairs/passthrough — as
  the next cycle; and bulk-classify-inbound-self-transfer as a later bulk item).

## Gotchas (for the reviewer)
- **G1 (the one to watch):** `basis_pending: false` even for a defaulted $0 basis. Copy-pasting the
  `IncomeInbound` arm would set `basis_pending: true` and silently GATE later disposals. The zero-basis
  honesty is an Advisory blocker, NOT `basis_pending`.
- **G2:** push NO `IncomeRecord` (non-taxable). The `IncomeInbound` template does — delete that.
- **G3:** `donor_acquired_at: None` — it is NOT a gift (no tacking); `gain_hp_start()` must equal the
  lot's own `acquired_at`.
- **G4:** the Advisory blocker fires on `basis.is_none()`, NOT on `usd_basis == 0` (an attested
  `Some(0)` must be silent).
- **G5 [R0-M2]:** wallet-missing corner — a wallet-less `TransferIn` self-transfer-in has nowhere to
  create the lot → emit Hard `UnknownBasisInbound` with a self-transfer message + return (do NOT copy the
  `IncomeInbound` guard verbatim — it emits `FmvMissing`, semantically wrong here). Add a KAT. Don't panic.
- **G6 (two kinds of Op-match site — handle BOTH) [R0-M1]:**
  (a) **Exhaustive** matches force a new arm (compile error if missed): `build_op`'s inbound-class match
  (add the C3 arm, `resolve.rs:256-277`) + any exhaustive `Op` render/dispatch.
  (b) **Catch-all** (`_ =>`) matches do NOT force an arm and already default CORRECTLY for a lot-CREATING
  op — `is_disposition_op` (`resolve.rs:996`, `_ => false`), `honoring_principal` (`resolve.rs:1008`,
  `_ => None`), and `evaluate.rs::honoring_sat` (`:76`, `_ => None`). LEAVE these on their defaults; a
  WRONG explicit arm (e.g. `honoring_principal => Some(sat)`) would SILENTLY break invariant #7
  (outside-FIFO). VERIFY all three read correctly for the new op — don't "helpfully" add an arm.
- **G7 (nit) [R0-N3]:** the CLI `--acquired` / TUI validator MAY warn if `acquired_at > receipt date`
  (a future typo). Not required for correctness — a future date only makes the lot short-term (the
  conservative direction) — but it's cheap hygiene.

## Out of scope (later cycles)
- **Cycle B:** matched in/out pairs — the `SelfTransferPassthrough` drop primitive (both legs → `Op::Skip`),
  the read-only matcher/proposal, and the user-confirm flow; the RELOCATE half already exists
  (`TransferLink` out→in). Confirmed-not-automatic + false-match safety live there.
- Bulk-classify-inbound-self-transfer (a bulk version, after single-item ships).
