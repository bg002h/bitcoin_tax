# SPEC — btctax-tui-edit chunk 4a: link-transfer + classify-raw

**Source baseline:** `main` @ `755e47c` (post save-rollback + hardening; all citations verified at write time).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-tui-edit-chunk4a-round-{1,2}.md` (round 1: 0C/2I/3M/1N — struct-field + wallet-source
catches; round 2: 0C/0I).**
**Design lineage:** chunk-4 architect design (anchors verified against `755e47c`), which recommended
splitting chunk 4 into **4a** (link-transfer + classify-raw — raw-import resolution, plain revocable
decisions) and **4b** (resolve-conflict + optimize-accept). This spec is 4a.

**Goal.** Two new decision flows on the existing `btctax-tui-edit` substrate:
1. **link-transfer** (`l`) — link a pending TransferOut to a TransferIn or a wallet → `TransferLink`
   decision; the pair projects the TransferOut to `Op::SelfTransfer` (non-taxable relocation).
2. **classify-raw** (`u`) — classify an `Unclassified` raw import event → `ClassifyRaw` decision
   (structured builder; **Income + Acquire variants only** this cycle — see D2).

**SemVer.** Two new `pub fn`s in `edit/persist.rs` (`persist_link_transfer`, `persist_classify_raw`);
new flow/modal struct families; key bindings `l`/`u`. No new forbidden token (both append via
`append_decision`; `append_` already covers them in KAT-G1). No `btctax-core`/`btctax-cli` public API
change; every `EventPayload` variant used is already `pub`. **MINOR** (pre-1.0; additive). **No
lockstep** (TUI-only; no clap flags; viewer untouched).

---

## Substrate inherited (all confirmed at `755e47c`)

- Dispatch (`main.rs:109-262`): modal layers 1-8 → flow layer 9 (8 flows) → form → screen. 4a appends
  new modal layers + `if app.<flow>.is_some()` arms.
- **Every opener** calls `app.residue_latch_status()` FIRST (returns `Some(msg)` when EITHER
  `attest_save_failed` OR `rollback_failed` is set — `main.rs:409-427`), then the `snapshot.is_none()`
  guard, then the pre-filter; empty filtered list → status + no-open.
- **Every rollback-class persist fn** (the save-rollback baseline): `let pre = session.snapshot()?;` →
  `append_decision(...)?` → `save_or_rollback(session, pre)?` → returns `Result<EventId, PersistError>`
  (`PersistError{NoChange,RolledBack,ResidueLive}`, `persist.rs:33-80`). The modal Enter arm closes its
  own modal then routes `Err(e)` through `app.on_persist_error(e)` (`main.rs:434-452`) — the single site
  arming `rollback_failed`. **Neither 4a flow is an unrecoverable batch → NO bespoke latch** (unlike
  chunk-3 attest). Both openers still call `residue_latch_status()` first (both latches gate them).
- Pick-list = `TargetList<T>` owned by the flow; `q`-swallow, Esc-steps-back; post-save status via
  `build_snapshot` re-projection; `events_by_id(snap)` at `main.rs:1878`.
- **#8 quit-first convention (cycle B):** any new CLI-pointing status says "quit the editor and run:
  btctax …".
- Free key bindings (`main.rs:249-257` binds `p c o r f v s d a` + nav): `l` and `u` confirmed free.

---

## D1 — link-transfer (`l`)

Mirrors `cmd::reconcile::link_transfer` (`reconcile.rs:211`). Appends
`EventPayload::TransferLink { out_event: EventId, in_event_or_wallet: TransferTarget }` where
`TransferTarget = InEvent(EventId) | Wallet(WalletId)` (`event.rs:93-102`). The linked pair projects
the TransferOut to `Op::SelfTransfer` (`resolve.rs:201-218`).

**CLI parity.** The CLI validates only ref-parse + "exactly one of `--to-event`/`--to-wallet`"
(`main.rs:796-805`); ALL correctness checks are engine-side `DecisionConflict`s (`resolve.rs:492-519`):
(a) duplicate link on the same `out_event`; (b) duplicate link targeting the same `in_event`; (c)
in-event with no resolvable dest wallet. The pre-filter prevents (a) and (b) up front; the engine is
the backstop.

**Pre-filter (three sets):**
- **Out-list** (step 1): `snap.state.pending_reconciliation` (`Vec<PendingTransfer>`; `.event: EventId`,
  `.principal_sat: Sat`) — the same inherently-post-filtered source `open_reclassify_outflow_flow` uses
  (`main.rs:2001-2024`): exactly the unlinked, unreconciled TransferOuts. Date/wallet from the raw event
  via `events_by_id`. (A pending TransferOut can be resolved by EITHER reclassify-outflow OR link-transfer
  — mutually exclusive within a session; no double-offer.)
- **In-list** (step 2, mode = InEvent): `snap.events` for `EventPayload::TransferIn` whose raw
  `LedgerEvent.wallet` [R0-M1: the wallet is the event's `wallet` field, not a TransferIn payload field]
  `is_some()` (engine requires a resolvable dest wallet — `resolve.rs:509-519`) AND not already targeted
  by a non-voided `TransferLink::InEvent` (build the consumed-in set like `open_classify_inbound_flow`'s
  `already_classified`, `main.rs:1919-1930`).
- **Wallet-list** (step 2, mode = Wallet): **all distinct `snap.events[].wallet` (the `Some` values)**
  [R0-I2] — NOT just `holdings_by_wallet` keys, which only contain wallets with `remaining_sat > 0`
  (`fold.rs:1170-1187`) and would hide a zero-balance destination wallet (the primary Wallet-target use
  case). Sorted for stable display. **Acknowledged limitation:** a wallet that has NEVER appeared in any
  event is not offerable (no wallet registry exists; the CLI `--to-wallet` accepts an arbitrary parsed
  `WalletId`) → the CLI remains available for a brand-new destination; recorded in FOLLOWUPS.

**Flow (two steps, no free text).** Step 1 pick a TransferOut (`Table`: Date | Principal Sat | Wallet |
EventId, title `" Link Transfer — select the outgoing transfer "`). Step 2 = a **mode toggle** (`Tab`
cycles InEvent ⇄ Wallet) over a second `TargetList` (the in-list or wallet-list). `Enter` on a target →
`link_transfer_modal`. Esc steps back one level (target → out-list → close). `q` swallowed.

**Flow/modal state:**
```rust
pub enum LinkTransferStep { OutList, TargetPick { out: TransferOutItem, mode: LinkMode } }
pub enum LinkMode { InEvent, Wallet }
pub struct LinkTransferFlowState { pub out_list: TargetList<TransferOutItem>, pub step: LinkTransferStep,
    pub in_list: TargetList<InEventItem>, pub wallet_list: TargetList<WalletItem> }
pub struct LinkTransferModalState { pub out_event: EventId, pub out_date: TaxDate, pub out_sat: Sat,
    pub target: TransferTarget, pub target_label: String }
```

**Modal** — shows out (id/sat/date), the target (in-event id or wallet), and the non-taxable framing
(honors the user-mandated TP8-c default: fee non-taxable, basis carries):
```
╔═ Confirm: link-transfer — WRITES THE VAULT ═══════════════╗
║  out:    import|river|out-007  (250000 sat, 2025-08-01)   ║
║  →link:  TransferIn import|cex|in-012  (wallet self:cold)  ║
║  Records a NON-TAXABLE self-transfer (relocation).         ║
║  Basis carries; any fee is non-taxable (TP8-c).            ║
║  Appended as a revocable decision (void with 'v').         ║
║  [Enter] Confirm & save   [Esc] Cancel — writes nothing    ║
╚════════════════════════════════════════════════════════════╝
```

**persist fn:**
```rust
pub fn persist_link_transfer(session: &mut Session, payload: EventPayload /* TransferLink */,
    now: OffsetDateTime) -> Result<EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    save_or_rollback(session, pre)?;
    Ok(id)
}
```

**Post-save status (keyed to `decision_id`):** (1) `DecisionConflict` on `decision_id` (duplicate link —
effectively unreachable given the exclusive lock + the up-front pre-filter; a defensive arm [R0-M3]) →
`"Saved, but DecisionConflict fired — the link was not applied;
clear with Void flow (press 'v'), or quit the editor and run: btctax reconcile void {}"`. (2) Clean →
`"Self-transfer link recorded for {out.canonical()} → {target_label}; the TransferOut is now a
non-taxable relocation."` `TransferLink` is revocable (`is_revocable_payload` includes it) → standard
modal, no irrevocability warning.

---

## D2 — classify-raw (`u`)

Mirrors `cmd::reconcile::classify_raw` (`reconcile.rs:151`). Appends
`EventPayload::ClassifyRaw { target: EventId, as_: Box<EventPayload> }`; `as_` must satisfy
`EventPayload::is_imported()` (Acquire/Income/Dispose/TransferOut/TransferIn/Unclassified,
`reconcile.rs:159-165`).

**Pre-filter:** events carrying `BlockerKind::Unclassified` (Hard; `fold.rs:1157`) whose payload is
`EventPayload::Unclassified(Unclassified{raw})` (`event.rs:80`), minus those already targeted by a
non-voided `ClassifyRaw` (dup ⇒ `DecisionConflict`, `resolve.rs:411-418`) — same shape as
`open_classify_inbound_flow`'s filter (`main.rs:1933-1964`) keyed on `Unclassified`. List columns:
Date | Raw-text (elided) | Wallet | EventId.

**Form — SCOPED structured builder (the load-bearing decision).** The CLI takes raw `--payload-json`,
but the TUI `FieldBuffer` caps at `FIELD_CAP=64` (`form.rs:18`, the chunk-3 parity limit) — a full
imported-payload JSON exceeds that. So classify-raw uses a **variant picker** (`Tab` cycles the target
variant) + a per-variant sub-form that builds the `EventPayload` **directly** (NOT via
`InboundClass` — the classify-inbound sub-form outputs `InboundClass::Income`, a different type, so it
can't be reused wholesale [R0-I1]). This cycle covers the two variants that dominate raw-row
classification, with fields matching the actual structs (`event.rs`):
- **`Acquire { sat: Sat, usd_cost: Usd, fee_usd: Usd, basis_source: BasisSource }`** — fields: `sat`
  (required), `usd_cost` (required), `fee_usd` (optional → $0), `basis_source` (a `BasisSource` PICK —
  default `ExchangeProvided`; the 8-variant enum is tax-load-bearing, NOT optional). **No `acquired-at`
  field** — the effective event keeps the TARGET's timestamp (`resolve.rs:709-729`), so an acquired-at
  input would be inert [R0-I1].
- **`Income { sat: Sat, usd_fmv: Option<Usd>, fmv_status: FmvStatus, kind: IncomeKind, business: bool }`**
  — fields: `sat` (required), `usd_fmv` (optional), `kind` (an `IncomeKind` PICK), `business` (toggle),
  and the load-bearing `fmv_status` (`resolve.rs:187` discards `usd_fmv` when `Missing`): if `usd_fmv`
  typed → `fmv_status = ManualEntry`; if empty → `usd_fmv = None`, `fmv_status = Missing` (which fires a
  `FmvMissing` blocker, surfaced by status arm 3). (Mirrors the classify-inbound income FMV handling but
  emits `EventPayload::Income` directly.)

**Dispose/TransferOut/TransferIn/Unclassified are recorded as a CLI-only parity FOLLOWUP** (a full
6-variant builder would blow the cycle budget; candidate for a later chunk). Build
`ClassifyRaw { target, as_: Box::new(built_payload) }`.

**Modal** — shows target id + raw text + the *built* imported payload (variant + fields) + "Appended as
a revocable decision (void with 'v')."

**persist fn:** `persist_classify_raw(session, payload /* ClassifyRaw */, now) -> Result<EventId,
PersistError>` — identical `snapshot → append_decision → save_or_rollback` shape.

**Post-save status:** (1) `DecisionConflict` on `decision_id` (dup classify) → clear-with-void (quit-first)
wording. (2) Clean and the target's `Unclassified` blocker is gone → `"Classified {target.canonical()}
as {variant}; the Unclassified blocker is cleared."` (3) Clean but a NEW blocker attributes to the
target — for the scoped Income/Acquire variants this is `FmvMissing` (Income with empty `usd_fmv` →
`Missing`) [R0-M2] → `"Classified, but {blocker} now applies — see Compliance."`

---

## Interactions (chunk-4 architect)

- **link-transfer ↔ chunk-3 #1 SelfTransfer select-lots inclusion** — no code interaction; a FOLLOWUP
  coupling only. link-transfer CREATES the `TransferLink` that projects a TransferOut to
  `Op::SelfTransfer`; chunk-3's documented under-inclusion means such an event won't appear in the `s`
  list (select-lots sources only `state.disposals`/`removals`). link-transfer makes that FOLLOWUP newly
  *reachable* but fixing it is out of scope (SelfTransfer is non-taxable, so lot-selection over it is a
  niche nicety, not a gate). **Record in FOLLOWUPS; do NOT widen select-lots.**
- link-transfer out-list shares `pending_reconciliation` with reclassify-outflow (mutually-exclusive
  resolutions — fine).

## KATs (chunk-3 skeleton)

Per flow: **strict-prefix persist KAT** (`post == pre ++ [decision]`, `decision_seq == max+1`, payload
round-trips); **cancel-path bytes-unchanged KAT** (`q` swallowed each step, Esc steps back,
`bytes_after == bytes_before`); **save-error KAT** (`#[cfg(unix)]` chmod → `save_or_rollback` reverts,
retry clean, routes through `on_persist_error`, `post == pre + 1` no residue); **validation KATs**;
**E2E KAT**. Plus:
- **link-transfer:** E2E link → assert the TransferOut becomes `Op::SelfTransfer`; wallet-target AND
  in-event-target both covered; duplicate-link → `DecisionConflict` arm.
- **classify-raw:** E2E → the `Unclassified` blocker is cleared after classify; Income AND Acquire
  variants round-trip; the not-yet-supported variants are not offered.
- **KAT-G1** stays green (both flows use `append_` only; no new token).

## Plan (TDD, phased — each phase: KATs red → implement green → review to 0C/0I)

- **Task 1 — link-transfer** (form.rs structs/validation; editor.rs flow+modal fields; main.rs `l`
  dispatch + opener + handlers + status; draw_edit.rs render; persist.rs `persist_link_transfer`).
- **Task 2 — classify-raw** (the scoped Income+Acquire builder; persist.rs `persist_classify_raw`).
- **Task 3 — whole-diff review (Phase E) + FOLLOWUPS** (record the SelfTransfer-select-lots
  now-reachable coupling; the classify-raw remaining-variants parity FOLLOWUP).

## Out of scope
- classify-raw Dispose/TransferOut/TransferIn/Unclassified variants (FOLLOWUP — full builder later).
- link-transfer to a wallet that has NEVER appeared in any event (no wallet registry exists) — the
  Wallet-target pick-list offers only wallets seen in `snap.events`; the CLI `--to-wallet` remains for a
  brand-new destination (FOLLOWUP) [R0-I2].
- 4b (resolve-conflict + optimize-accept) — the next cycle.
- Widening select-lots to include SelfTransfers (chunk-3 FOLLOWUP; non-taxable, low value).
- Any `btctax-core`/`btctax-cli` public API change; viewer changes.
