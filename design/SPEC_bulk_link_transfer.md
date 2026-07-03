# SPEC — bulk self-transfer (`bulk-link-transfer`)

**Source baseline:** `main` @ `a16ea00` (mutating-TUI feature-complete; all anchors verified at write time).
**Review status: DRAFT — awaiting user review, then mandatory R0.**
**Design lineage:** brainstorm with the user (2026-07-03). Settled decisions:
scope = **self-transfer first**; mechanic = **link all selected outflows to ONE destination wallet**
(out→wallet, no fuzzy matching); home = **both** CLI + TUI over a shared `btctax-cli` core; **source-wallet
filter IN**; confirm = **explicit** (not typed-word — the op is reversible); **TUI per-row exclude IN**.

**Goal.** Apply self-transfer (`TransferLink` → `Op::SelfTransfer`, non-taxable, basis carries) to MANY
pending outbound transfers at once — filtered by time frame (a year, a `--from/--to` range, or the entire
set) and optionally by source wallet — each linked to one chosen destination wallet, in a single atomic,
reversible batch, behind a preview that surfaces the total USD value being reclassified as non-taxable.

**Non-goals (v1).** Other reconcile decision types (only self-transfer); out→in auto-matching (only
out→wallet); linking to a wallet never seen in any event via the TUI (CLI `--to-wallet` covers arbitrary
wallets — the chunk-4a R0-I2 limitation carries).

---

## SemVer / lockstep

- **btctax-core:** UNCHANGED — `TransferLink`/`TransferTarget`/`WalletId`/`PendingTransfer` all pre-exist.
- **btctax-cli:** MINOR/additive — new `Session::bulk_link_transfer_plan` read helper, new
  `cmd::reconcile::bulk_link_transfer`, new `Reconcile::BulkLinkTransfer` clap variant.
- **btctax-tui-edit:** MINOR/additive — new `b` flow + `persist_bulk_link_transfer`.
- **Lockstep: NONE.** Verified at write time: this repo has **no `docs/manual/`** and **no GUI crate**
  → the CLI-manual and GUI `schema_mirror` mirrors do not apply. (Confirm again at R0.)

---

## Grounding (verified at `a16ea00`)

- **Pending outs** = `snap.state.pending_reconciliation: Vec<PendingTransfer>`; `PendingTransfer {
  event: EventId, principal_sat: Sat, fee_sat: Option<Sat>, legs: Vec<PendingLeg> }` (`state.rs:197`).
  The engine post-filters this: a linked out projects as `Op::SelfTransfer` and is NOT pushed to
  `pending_reconciliation` — so **the set already excludes already-linked / already-decided outs** (the
  clean safety boundary; matches the single `open_link_transfer_flow` out-list, `main.rs:3679`).
  `legs` carry each removed lot's basis + acquired_at (for the basis total).
- **Destination** = `WalletId` (`identity.rs`): `Exchange{provider,account}` | `SelfCustody{label}`.
  CLI parses `--to-wallet <str>` via `eventref::parse_wallet_id` (the single-flow dispatch,
  `main.rs:791`). `TransferTarget::Wallet(WalletId)` (`event.rs:94`).
- **USD-value-at-date** (preview safety number) = `principal_sat` (as BTC) × `BundledPrices::usd_per_btc
  (tax_date) -> Option<Usd>` (`btctax-adapters/src/price.rs:47`; `BundledPrices::load()` :20). Missing
  price → render `—` (advisory; never blocks the link).
- **Single-append persist precedent:** `persist_link_transfer` (`edit/persist.rs:335`) — snapshot →
  `append_decision(TransferLink)` → `save_or_rollback`. The batch version loops the append, saves ONCE.
- **CLI clap:** `enum Reconcile` (`main.rs:208`); `LinkTransfer { out, #[arg(conflicts_with="to_wallet")]
  to_event, to_wallet }` + dispatch (`main.rs:791`, `cmd::reconcile::link_transfer` :210). `append_and_save`
  (`reconcile.rs:26`) is the single append+save; the batch appends all then saves once.
- **KAT-G1** persist-only tokens (`edit/persist.rs:1270`): `conn(`/`save(`/`tax_profile::set`/`append_`/
  `donation_details::set`/`optimize_attest::set`/`restore(`. The batch append uses `append_` → confined
  to `edit/persist.rs`; the TUI opener does its selection ONLY through the Session plan helper (no
  forbidden token), exactly as chunks 4b/5 did for `optimize_proposal`/`safe_harbor_residue`.
- **Revocability:** `TransferLink` is in `is_revocable_payload` (`form.rs:841`) → every bulk-created link
  is individually voidable (`v`); this is why explicit-confirm (not typed-word) is the right ceremony.
- **#7 void pre-filter** (`main.rs:2544`) excludes only effective SafeHarborAllocations → bulk TransferLinks
  are normally voidable and appear in the `v` list. No change to `#7`.

---

## D1 — the shared plan (read-only) `Session::bulk_link_transfer_plan` (btctax-cli)

Modeled on `optimize_proposal`/`safe_harbor_residue` (a `&self` read helper on the held session; appends
nothing; KAT-G1-clean at the TUI call site). Signature:
```rust
pub struct BulkFilter { pub frame: Frame, pub from_wallet: Option<WalletId> }
pub enum Frame { All, Year(i32), Range { from: TaxDate, to: TaxDate } }   // to inclusive
pub struct BulkLinkRow {
    pub out_event: EventId, pub date: TaxDate, pub source_wallet: Option<WalletId>,
    pub principal_sat: Sat, pub usd_value: Option<Usd>,   // principal × usd_per_btc(date); advisory
    pub basis_usd: Usd,                                    // Σ leg basis carried (non-taxable → carries)
}
pub struct BulkLinkPlan {
    pub dest: WalletId,
    pub included: Vec<BulkLinkRow>,   // eligible + in-frame + passes from_wallet + source != dest
    pub skipped_same_wallet: Vec<BulkLinkRow>,   // source_wallet == dest → cannot self-link to itself
    pub total_sat: Sat, pub total_usd_value: Option<Usd>, pub total_basis_usd: Usd, // over `included`
}
pub fn bulk_link_transfer_plan(&self, filter: BulkFilter, dest: WalletId)
    -> Result<BulkLinkPlan, CliError>;
```
**Selection** (over `snap.state.pending_reconciliation`, enriched from the event index):
1. Enrich each pending out: `date = tax_date(ev.utc_timestamp, ev.original_tz)`, `source_wallet =
   ev.wallet`, `principal_sat`, `usd_value = prices.usd_per_btc(date).map(|p| p × principal_sat_as_btc)`,
   `basis_usd = Σ leg basis`.
2. **Frame filter:** `All` → keep all; `Year(y)` → `date.year() == y`; `Range{from,to}` → `from ≤ date ≤ to`.
3. **Source-wallet filter:** if `from_wallet = Some(w)`, keep only `source_wallet == Some(w)`.
4. **Same-wallet guard:** `source_wallet == Some(dest)` → into `skipped_same_wallet` (a self-link to the
   same wallet is meaningless), NOT `included`.
5. Sort `included` by `date`. Totals are over `included` only.
`usd_per_btc` missing for a date → that row's `usd_value = None` and the total is `None` if ANY included
row is missing a price (so the UI shows "≥ $X (some prices unavailable)" honestly — never a false total).

---

## D2 — CLI surface `bulk-link-transfer`

**Clap variant** (in `enum Reconcile`, `main.rs:208`):
```rust
/// Bulk-confirm self-transfers: link every PENDING outbound transfer in a time frame to one
/// destination wallet (non-taxable). Shows a preview + requires --yes (or interactive y/N).
BulkLinkTransfer {
    /// Destination wallet every selected outflow links to.
    #[arg(long)] to_wallet: String,
    /// Restrict to a single tax year (mutually exclusive with --from/--to).
    #[arg(long, conflicts_with_all = ["from", "to"])] year: Option<i32>,
    #[arg(long, requires = "to")] from: Option<String>,   // YYYY-MM-DD
    #[arg(long, requires = "from")] to: Option<String>,    // YYYY-MM-DD, inclusive
    /// Only outflows FROM this source wallet.
    #[arg(long)] from_wallet: Option<String>,
    /// Print the preview and exit without writing.
    #[arg(long)] dry_run: bool,
    /// Skip the interactive confirmation (non-interactive apply).
    #[arg(long)] yes: bool,
}
```
Frame: none of year/from/to → `All`; `year` → `Year`; `from`+`to` → `Range`. **Dispatch/command
`cmd::reconcile::bulk_link_transfer`:** parse dest + optional from_wallet via `parse_wallet_id`; build
the plan via `session.bulk_link_transfer_plan(filter, dest)`; **render the preview table** (date · source
wallet · BTC · USD-value + the totals footer incl. "**total USD reclassified non-taxable**" and the
skipped-same-wallet count). Then:
- `included.is_empty()` → print "no pending outbound transfers match" and exit 0 (no write).
- `--dry-run` → stop after the preview (exit 0).
- else confirm: `--yes` skips the prompt; otherwise interactive `y/N` (default No). On No → abort, no write.
- **Apply (atomic):** append a `TransferLink { out_event, in_event_or_wallet: Wallet(dest) }` for each
  `included` row, then a SINGLE `save`. All-or-nothing (batch-append-then-one-save via
  `append_decision` loop + `session.save()`; mirror `append_and_save` but N appends / one save). Print
  "linked N outflows to <dest>; M skipped (same wallet)".

---

## D3 — TUI surface: the bulk flow (`b`)

New Browse key **`b`** (free; capitals only `A`/`G` bound; lowercase `b` unused). `open_bulk_link_transfer_flow`
(latch → snapshot → `pending_reconciliation` non-empty else status "No pending outbound transfers to bulk-link").
A four-step flow on the `TargetList` substrate:

1. **Destination pick** — a wallet pick-list = the DISTINCT wallets across ALL `snap.events` (the same
   union the single `open_link_transfer_flow` offers, `main.rs:3679` — a destination may be a wallet that
   only ever appears as an inbound, so it must be the full event-wallet union, NOT just pending-out source
   wallets). Per R0-I2 the TUI cannot offer a never-seen wallet → the footer notes "for a new destination
   wallet, use the CLI `--to-wallet`". Enter → step 2.
2. **Filter step** — (a) source-wallet: "Any" + each distinct source wallet; (b) time frame: "All" + each
   distinct year present in the pending outs (avoids free-text date entry in v1; `--from/--to` is CLI-only).
   Enter → recompute the plan → step 3.
3. **Preview checklist (per-row exclude)** — a scrollable `TargetList<BulkLinkRow>` where every included
   row starts CHECKED; `Space`/`x` toggles a row's exclusion; `k/j/g/G` scroll. Each row: `date · source
   wallet · BTC · USD-value`. Footer: **checked count · Σ BTC · Σ USD reclassified non-taxable** (live).
   Enter → confirm modal (over the CHECKED rows). Esc → back to filter.
4. **Confirm modal** (explicit confirm, NOT typed-word): shows dest wallet, checked count, Σ BTC, **Σ USD
   being made non-taxable**, and "Each link is individually voidable ('v'). Enter: apply — writes the vault.
   Esc: cancel." Enter → `persist_bulk_link_transfer`.

`q` swallowed throughout; Esc steps back one layer; the modal dispatches before the flow (shipped
convention). `reset_flows`/`close_all_mutation_surfaces` clear the new `Option` state. New form/editor
state structs mirror the safe-harbor-allocate flow (`SafeHarborAllocateFlowState`/`ModalState`).

**`persist_bulk_link_transfer` (edit/persist.rs — batch append, single save_or_rollback, no latch):**
```rust
pub fn persist_bulk_link_transfer(session: &mut Session, out_events: Vec<EventId>, dest: WalletId,
    now: OffsetDateTime) -> Result<usize, PersistError> {
    let pre = session.snapshot()?;
    for out_event in &out_events {
        let payload = EventPayload::TransferLink(TransferLink {
            out_event: out_event.clone(), in_event_or_wallet: TransferTarget::Wallet(dest.clone()) });
        append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;   // no save yet
    }
    save_or_rollback(session, pre)?;   // ONE save; on failure the whole batch reverts
    Ok(out_events.len())
}
```
An empty `out_events` (user unchecked everything) → refuse before persist (status "Nothing selected");
never append zero + save.

**Post-apply status** (`derive_bulk_link_status`, re-project): `"Linked {N} outflow(s) to {dest} as
self-transfers ({remaining} pending outbound remain)."` No blocker arm is normally reachable (each append
is the same shape the single flow uses; a failed save rolls back clean via `on_persist_error`).

---

## Atomicity & correctness (both surfaces)

- **One save per bulk op.** Both surfaces append all N `TransferLink`s, then save once. A mid-batch or
  save failure reverts the ENTIRE batch (TUI: `save_or_rollback`/whole-DB restore; CLI: no save happened
  yet, so the in-memory conn is discarded on error). Never a partial apply.
- **Idempotence:** because the source set is `pending_reconciliation` (which drops linked outs),
  re-running never double-links an out. A row unchecked/excluded is simply not appended.
- **Each link is independently voidable** (`v`) → a mistaken bulk apply is recoverable per-row.

## Interactions

- **Single link-transfer (`l`):** shares the source set (`pending_reconciliation`) and the payload shape;
  bulk is the many-at-once form. No change to `l`.
- **Void (`v`) + #7:** bulk-created `TransferLink`s are ordinary revocable decisions → appear in the void
  list, voidable individually. No change to `v`/#7.
- **Self-transfer treatment:** stays TP8-(c) (non-taxable, basis carries) — the mandated default; this
  feature does NOT alter self-transfer tax treatment, only bulk-applies the existing `TransferLink`.

## Gotchas (for the reviewer)

1. **Selection MUST be `pending_reconciliation`** (not "all TransferOut events") — that set already
   excludes already-decided/linked outs, so bulk can never re-touch or override a prior decision.
2. **Same-wallet skip is mandatory** — an out whose source == dest must be skipped (reported), or the link
   would be a self-referential no-op/anomaly.
3. **USD total honesty:** if any included row lacks a price, the total is rendered as a floor
   ("≥ $X, some prices unavailable"), never a wrong exact total.
4. **KAT-G1:** the batch append lives in `edit/persist.rs` (`append_`); the opener/plan use ONLY the
   Session helper — no forbidden token in tui-edit non-test source.
5. **Atomic single save** — do NOT save per-row (N saves = N re-projections + partial-apply risk).
6. **Empty-selection guards** at every gate (plan empty → no open/exit; all unchecked → refuse apply).

## KATs

**btctax-cli:** `bulk_plan_selects_pending_in_frame` (frame + from_wallet filters; same-wallet skipped);
`bulk_plan_usd_total_floor_when_price_missing`; `bulk_cli_dry_run_writes_nothing`;
`bulk_cli_apply_is_atomic_single_save` (N TransferLinks appended, one save, all project as SelfTransfer);
`bulk_cli_no_match_exits_clean`.
**edit/persist.rs:** `kat_persist_bulk_link_strict_prefix` (exactly N TransferLinks tail-appended, all
`Wallet(dest)`); `kat_persist_bulk_link_rolls_back_on_failed_save` (chmod → `Err(RolledBack)`, log
unchanged, retry clean); `kat_persist_bulk_link_refuses_empty`.
**main.rs (opener/flow):** `kat_bulk_refuses_when_no_pending`; `kat_bulk_per_row_exclude_drops_row`
(unchecking a row omits it from the appended batch); `kat_bulk_same_wallet_row_absent`; E2E
`kat_e2e_bulk_link_then_selftransfer` (`b` → filter → exclude one → confirm → the included outs project as
`Op::SelfTransfer`, the excluded one stays pending) + `kat_e2e_bulk_link_then_void` (one bulk link voids
cleanly). **KAT-G1** stays green.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)

- **Task 1 — core plan + CLI** (`Session::bulk_link_transfer_plan`; `Reconcile::BulkLinkTransfer` variant
  + dispatch + `cmd::reconcile::bulk_link_transfer` with preview/dry-run/--yes/atomic apply; CLI KATs).
- **Task 2 — TUI flow** (`b` opener + 4-step flow [dest pick, filter, preview checklist w/ per-row
  exclude, confirm modal]; `persist_bulk_link_transfer`; `derive_bulk_link_status`; draw overlays;
  persist + opener + exclude KATs).
- **Task 3 — E2E round-trips + whole-diff review (Phase E) + FOLLOWUPS** (bulk→selftransfer, bulk→void;
  record deferrals: out→in auto-match, other decision types, TUI never-seen-wallet dest, `--from/--to`
  in the TUI).

## Out of scope
- Auto-matching outs to in-events; any decision type other than self-transfer; TUI free-text date range
  and never-seen-wallet destinations (CLI covers both); undo-the-whole-batch as one action (per-row `v`).
