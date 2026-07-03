# SPEC — bulk classify-inbound-self-transfer

**Source baseline:** `main` @ `569a5ee` (post self-transfer-completion A+B; all anchors verified at write time).
**Review status: DRAFT — awaiting mandatory R0.**
**Design lineage:** user-approved (2026-07-03) as queue item 2 after the self-transfer completion program;
a close MIRROR of the shipped `bulk-link-transfer` (the outbound bulk) applied to Cycle A's
`InboundClass::SelfTransferMine`. Governed by memory `self-transfer-completion-policy`.

**Goal.** Apply the inbound self-transfer-in classification (Cycle A — "my own coins", **$0 conservative
basis**, non-taxable) to MANY pending unknown-basis inbound deposits at once — filtered by time frame +
optional receiving wallet — in a single atomic, reversible batch, behind a preview that surfaces the
**total USD being given $0 basis** (the deliberate over-tax exposure). The inbound mirror of bulk-link.

**Workflow ordering (documented, not enforced).** Run the Cycle B `match-self-transfers` matcher FIRST —
it pairs inbounds that have a matching withdrawal (RELOCATE, carrying REAL basis; or DROP). This bulk flow
then sweeps the LEFTOVER unmatched inbounds (origin genuinely lost) as $0-basis self-transfer-ins. Doing
it in that order means you only zero-basis coins whose cost was truly unrecoverable.

**Scope (v1).** Uniform `SelfTransferMine { basis: None, acquired_at: None }` — i.e. **strictly $0 basis /
receipt-date HP** for every selected inbound. No per-item basis/date input (that is what makes it cleanly
bulk-able); a deposit that has a real recoverable basis is handled by **excluding it** in the preview and
doing it single-item (`classify-inbound-self-transfer --basis`). Per-row exclude is the precision tool.

---

## SemVer / lockstep
- **btctax-core:** UNCHANGED — reuses Cycle A's `InboundClass::SelfTransferMine` + the `ClassifyInbound`
  decision. No new core; no forward-only-vault concern beyond what Cycle A already introduced.
- **btctax-cli:** MINOR/additive — new `Session::bulk_self_transfer_in_plan` read helper +
  `cmd::reconcile::{bulk_self_transfer_in_plan, apply_bulk_self_transfer_in}` + a `Reconcile` clap variant.
- **btctax-tui-edit:** MINOR/additive — a new bulk flow + `persist_bulk_self_transfer_in`.
- **Lockstep: NONE** (no `docs/manual/`, no GUI crate — verified).

---

## Grounding (verified at `569a5ee`)
- **Bulk pattern to mirror:** `Session::bulk_link_transfer_plan` (`session.rs:333`),
  `cmd::reconcile::{bulk_link_plan:214, apply_bulk_link_transfer:229}`, `persist_bulk_link_transfer`
  (`edit/persist.rs:393` — the batch-append + single `save_or_rollback` + the **[bulk-I1] mid-batch
  rollback** `if let Err(e) = append_decision(...) { return Err(rollback(session,&pre,e.into())) }`), and
  the honest-floor plan shape (`total_usd_value_floor: Usd` + `missing_price_count`).
- **Cycle A single-item to bulk:** `InboundClass::SelfTransferMine { basis, acquired_at }` (`event.rs:140`);
  the emitter `cmd::reconcile::classify_inbound` (`reconcile.rs:38`); `persist_classify_inbound`
  (`edit/persist.rs:122`). Bulk appends `ClassifyInbound { transfer_in_event, as_: SelfTransferMine{None,
  None} }` per selected inbound.
- **Candidate source (structural false-classify safety):** the pending unknown-basis inbounds =
  `TransferIn` events still flagged `BlockerKind::UnknownBasisInbound`, enumerated from the blocker set
  joined to the raw `TransferIn` via the event index — the EXACT pattern `Session::self_transfer_match_plan`
  uses (`session.rs:421-438`). An already-classified inbound (Income/Gift/self-transfer-in) or a matched
  leg is no longer `UnknownBasisInbound`, so it can NEVER be swept (mirrors bulk-link's
  `pending_reconciliation` boundary).
- **USD-at-receipt** (preview safety number) = `btctax_core::price::fmv_of(&prices, receipt_date,
  in.sat) -> Option<Usd>` (the vetted helper; missing price → `None`). This is the market value being
  given $0 basis.
- Free Browse key: **`B`** (capital; only `A`/`G` capitals bound — `B` pairs with `b` bulk-link).

---

## D1 — the shared plan (read-only) `Session::bulk_self_transfer_in_plan` (btctax-cli)

Mirror `bulk_link_transfer_plan` exactly (a `&self` read helper; appends nothing; KAT-G1-clean at the TUI
call site). Signature:
```rust
pub struct BulkStiFilter { pub frame: Frame, pub wallet: Option<WalletId> }   // Frame reused from bulk-link
pub struct BulkStiRow {
    pub in_event: EventId, pub date: TaxDate, pub wallet: Option<WalletId>,
    pub sat: Sat, pub usd_fmv: Option<Usd>,   // fmv_of(&prices, date, sat); the USD being given $0 basis
}
pub struct BulkStiPlan {
    pub included: Vec<BulkStiRow>,
    pub total_sat: Sat,
    pub total_usd_fmv_floor: Usd,     // Σ of the Some usd_fmv — the honest floor (over-tax exposure)
    pub missing_price_count: usize,   // → render "≥ $X (N unavailable)" vs exact "$X"
}
pub fn bulk_self_transfer_in_plan(&self, filter: BulkStiFilter) -> Result<BulkStiPlan, CliError>;
```
**Selection:** enumerate `UnknownBasisInbound`-flagged `TransferIn` events (as
`self_transfer_match_plan` does); enrich each with `date = tax_date(...)`, `wallet = ev.wallet`, `sat`,
`usd_fmv = fmv_of(&prices, date, sat)`; **frame filter** (`All` / `Year(y)` / `Range{from,to}` — reuse
`Frame`); **wallet filter** (if `Some(w)`, keep `wallet == Some(w)`); sort by `date`. Honest floor: a row
with no price → `usd_fmv = None` + `missing_price_count += 1`; the total renders exact `$X` (0 missing) or
`≥ $X (N unavailable)`. **No same-wallet guard needed** (an inbound has no "destination"; it's the
receiving leg). Empty `included` → the caller no-opens / exits.

## D2 — CLI surface `bulk-classify-inbound-self-transfer`
Two-phase, mirror bulk-link (`reconcile.rs:214/229`): `bulk_self_transfer_in_plan(vault, pp, filter)`
(read) → dispatch renders the preview (date · wallet · BTC · **USD FMV given $0 basis**; totals footer
incl. "**total USD reclassified to $0 basis (you'll be conservatively over-taxed on this later)**") →
`apply_bulk_self_transfer_in(vault, pp, in_events, now)` (write, only when confirmed) = append a
`ClassifyInbound{SelfTransferMine{None,None}}` per `in_event`, then a SINGLE `save` (atomic; local Session
discarded on any error before save). Clap variant:
```
reconcile bulk-classify-inbound-self-transfer [--year Y | --from D --to D] [--wallet W] [--dry-run] [--yes]
```
`included.is_empty()` → "no unclassified inbound deposits match" + exit 0. `--dry-run` → preview + stop.
`--yes`|interactive `y/N` → confirm. Print "classified N inbound deposits as self-transfer-in ($0 basis)".

## D3 — TUI surface: the bulk flow (`B`)
New Browse key **`B`**. `open_bulk_self_transfer_in_flow` (latch → snapshot → non-empty candidate set else
status "No unclassified inbound deposits to bulk-classify"). Steps (mirror the bulk-link flow):
1. **Filter** — (a) wallet: "Any" + each distinct receiving wallet of the candidates; (b) time frame:
   "All" + each distinct year present (year picker; `--from/--to` is CLI-only, per the bulk-link Fork-A).
2. **Preview checklist (per-row exclude)** — a scrollable `TargetList<BulkStiRow>` all-checked;
   `Space`/`x` toggles exclusion; each row `date · wallet · BTC · USD-FMV`; footer live **checked count ·
   Σ BTC · Σ USD given $0 basis** (the `≥ $X (N unavailable)` floor).
3. **Confirm modal** (explicit, NOT typed-word — reversible): checked count, Σ BTC, **Σ USD → $0 basis**,
   and "each is a voidable classify-inbound decision; for any deposit whose real cost you can substantiate,
   exclude it here and classify it single-item with a real basis (`classify-inbound-self-transfer
   --basis`)". Enter → `persist_bulk_self_transfer_in`.

`q` swallowed; Esc steps back; modal before flow; `reset_flows`/`close_all_mutation_surfaces` clear the
new state. Empty selection (all unchecked) → refuse before persist.

**`persist_bulk_self_transfer_in` (edit/persist.rs — batch append, single save, mid-batch rollback):**
```rust
pub fn persist_bulk_self_transfer_in(session: &mut Session, in_events: Vec<EventId>,
    now: OffsetDateTime) -> Result<usize, PersistError> {
    let pre = session.snapshot()?;
    for in_event in &in_events {
        let payload = EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: in_event.clone(),
            as_: InboundClass::SelfTransferMine { basis: None, acquired_at: None } });
        if let Err(e) = append_decision(session.conn(), payload, now, UtcOffset::UTC, None) {
            return Err(rollback(session, &pre, e.into()));   // [bulk-I1] whole-batch revert
        }
    }
    save_or_rollback(session, pre)?;   // ONE save
    Ok(in_events.len())
}
```
Empty `in_events` → refuse (status "Nothing selected"); never append zero + save.

**Post-apply status** (`derive_bulk_sti_status`, re-project): `"Classified {N} inbound deposit(s) as
self-transfer-in ($0 basis); {remaining} unclassified inbound(s) remain."`

---

## Atomicity & correctness (both surfaces)
- **One save per bulk op**; a mid-batch append failure reverts the WHOLE batch (TUI via `rollback`, CLI
  discards the unwritten Session). Never partial.
- **Idempotence / structural safety:** candidates are only `UnknownBasisInbound` inbounds (already drops
  classified/matched ones), so re-running never double-classifies; an excluded row is simply not appended.
- **Reversible:** each is a voidable `ClassifyInbound` decision (`v`), re-exposing the inbound as
  `UnknownInbound`.

## Interactions
- **Cycle A single-item (`u` classify-inbound):** shares the candidate set + payload; bulk is the
  many-at-once form (all $0-basis). No change to `u`.
- **Cycle B matcher (`match-self-transfers`):** run FIRST — it removes the inbounds that have a matching
  withdrawal (which get REAL basis via RELOCATE or a DROP). This bulk handles only the leftover unmatched
  inbounds. Documented; not enforced (both draw from `UnknownBasisInbound`, so a matched-then-reconciled
  leg is no longer a candidate here).
- **Void (`v`):** bulk-created classifications are ordinary revocable decisions.

## Gotchas (for the reviewer)
- **G1:** selection MUST be the `UnknownBasisInbound` set (not "all TransferIn") — that set already
  excludes reconciled/matched inbounds → bulk can't sweep an income deposit or a matched self-transfer.
- **G2 (honesty):** the preview MUST surface the total USD FMV being given $0 basis (the over-tax the user
  is accepting) as a floor (`≥ $X` when a price is missing) — never blank, never a false exact.
- **G3 [bulk-I1]:** the persist fn reverts a mid-batch append failure via `rollback(session,&pre,e)` — do
  NOT use `?` on the append (append_decision commits per-call). Add the mid-batch-failure KAT.
- **G4:** `fmv_of` (checked + round_cents + overflow→None) for the FMV, not a hand-rolled multiply.
- **G5:** empty guards at every gate (empty plan → no open/exit; all unchecked → refuse).
- **G6:** KAT-G1 — the batch append is in `edit/persist.rs`; the opener's non-empty guard + candidate/
  wallet enumeration read `snap` directly (fine); the priced plan uses the Session helper. No forbidden
  token in tui-edit non-test source.

## KATs
- **btctax-cli:** `bulk_sti_plan_selects_unknown_inbounds_in_frame` (frame + wallet filters; classified/
  matched inbounds NOT selected — the structural safety); `bulk_sti_plan_fmv_floor_when_price_missing`;
  `bulk_sti_cli_dry_run_writes_nothing`; `bulk_sti_cli_apply_is_atomic_single_save` (N ClassifyInbound
  appended, one save, all project as non-taxable $0-basis lots); `bulk_sti_cli_no_match_exits_clean`.
- **edit/persist.rs:** `persist_bulk_sti_strict_prefix` (exactly N ClassifyInbound{SelfTransferMine}
  tail-appended); `persist_bulk_sti_reverts_mid_batch_append_failure` [bulk-I1]; `persist_bulk_sti_refuses_
  empty`.
- **main.rs:** `bulk_sti_refuses_when_no_candidates`; `bulk_sti_per_row_exclude_drops_row`; E2E
  `bulk_sti_then_lots_created` (`B` → filter → exclude one → confirm → included inbounds create non-taxable
  $0-basis lots + clear `UnknownBasisInbound`, the excluded one stays unclassified) + `bulk_sti_then_void`
  (one bulk classification voids cleanly, re-exposing `UnknownInbound`). **KAT-G1** stays green.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)
- **Task 1 — core plan + CLI** (`Session::bulk_self_transfer_in_plan`; the two-phase CLI +
  `Reconcile::BulkClassifyInboundSelfTransfer` variant + dispatch + atomic apply; CLI KATs).
- **Task 2 — TUI flow** (`B` opener + filter + preview checklist w/ per-row exclude + confirm modal;
  `persist_bulk_self_transfer_in`; `derive_bulk_sti_status`; draw overlays; persist + opener + exclude KATs
  + the two E2Es).
- **Task 3 — whole-diff review (Phase E) + FOLLOWUPS** (record queue item 3 — bulk for the OTHER decision
  types).

## Out of scope
- A uniform basis/acquired override for the bulk (v1 is strictly $0/receipt-date; per-row exclude + the
  single-item `--basis` cover the recoverable-basis case).
- TUI free-text date RANGE (CLI `--from/--to` covers it — year picker + per-row exclude is the TUI path).
- Bulk for the OTHER decision types (queue item 3).
