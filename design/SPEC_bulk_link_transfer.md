# SPEC — bulk self-transfer (`bulk-link-transfer`)

**Source baseline:** `main` @ `a16ea00` (mutating-TUI feature-complete; all anchors verified at write time).
**Review status: R0 round 1 folded (0C / 2I / 4M / 2N — all folded; Fork B EXPAND adopted); awaiting R0
round 2. Review: `reviews/R0-spec-bulk-link-transfer-round-1.md`.**
**Design lineage:** brainstorm with the user (2026-07-03). Settled decisions:
scope = **self-transfer first**; mechanic = **link all selected outflows to ONE destination wallet**
(out→wallet, no fuzzy matching); home = **both** CLI + TUI over a shared `btctax-cli` core; **source-wallet
filter IN**; confirm = **explicit** (not typed-word — the op is reversible); **TUI per-row exclude IN**.
R0 scope adjudication (architect, user-delegated): **Fork A** TUI date ranges → KEEP CLI-only (year +
per-row exclude covers it); **Fork B** TUI destination → **EXPAND** — the TUI accepts a TYPED destination
(`self:cold-wallet`), so a never-seen cold wallet is reachable in the editor (the headline case).

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
- **USD-value-at-date** (preview safety number) = `btctax_core::price::fmv_of(&prices, tax_date,
  principal_sat) -> Option<Usd>` ([R0-M1] the vetted helper — checked ops + `round_cents` + overflow→`None`
  — at `price.rs:13`; do NOT hand-roll `principal × usd_per_btc`. `usd_per_btc` is a `PriceProvider` trait
  method, `price.rs:46`, reachable via `use btctax_core::PriceProvider`, but `fmv_of` is the right API).
  `BundledPrices::load()` :20. Missing price → `None` → render `—` (advisory; never blocks the link).
- **Single-append persist precedent:** `persist_link_transfer` (`edit/persist.rs:335`) — snapshot →
  `append_decision(TransferLink)` → `save_or_rollback`. The batch version loops the append, saves ONCE.
- **CLI clap:** `enum Reconcile` (`main.rs:208`); `LinkTransfer { out, #[arg(conflicts_with="to_wallet")]
  to_event, to_wallet }` + dispatch (`main.rs:791`, `cmd::reconcile::link_transfer` :210). `append_and_save`
  (`reconcile.rs:26`) is the single append+save; the batch appends all then saves once.
- **KAT-G1** persist-only tokens (`edit/persist.rs:1270`): `conn(`/`save(`/`tax_profile::set`/`append_`/
  `donation_details::set`/`optimize_attest::set`/`restore(`. The batch append uses `append_` → confined
  to `edit/persist.rs`; the TUI opener does its selection ONLY through the Session plan helper (no
  forbidden token), exactly as chunks 4b/5 did for `optimize_proposal`/`safe_harbor_residue`.
- **Revocability:** `TransferLink` is in `is_revocable_payload` (`form.rs:853` [R0-N1]) → every bulk-created
  link is individually voidable (`v`); this is why explicit-confirm (not typed-word) is the right ceremony.
- **#7 void pre-filter** (`main.rs:2559-2583` [R0-N1]) excludes only effective SafeHarborAllocations → bulk
  TransferLinks are normally voidable and appear in the `v` list. No change to `#7`.

---

## D1 — the shared plan (read-only) `Session::bulk_link_transfer_plan` (btctax-cli)

Modeled on `optimize_proposal`/`safe_harbor_residue` (a `&self` read helper on the held session; appends
nothing; KAT-G1-clean at the TUI call site). Signature:
```rust
pub struct BulkFilter { pub frame: Frame, pub from_wallet: Option<WalletId> }
pub enum Frame { All, Year(i32), Range { from: TaxDate, to: TaxDate } }   // to inclusive
pub struct BulkLinkRow {
    pub out_event: EventId, pub date: TaxDate,
    pub source_wallet: Option<WalletId>,  // [R0-N2] ALWAYS Some for pending outs (a wallet-less TransferOut
                                          // never reaches pending_reconciliation, fold.rs); Option kept defensively
    pub principal_sat: Sat,
    pub usd_value: Option<Usd>,           // fmv_of(&prices, date, principal_sat) [R0-M1]; advisory, None on
                                          // missing price / overflow
    pub basis_usd: Usd,                   // Σ leg `usd_basis` carried (over principal+fee sats [R0-M3]; non-taxable → carries)
}
pub struct BulkLinkPlan {
    pub dest: WalletId,
    pub included: Vec<BulkLinkRow>,   // eligible + in-frame + passes from_wallet + source != dest
    pub skipped_same_wallet: Vec<BulkLinkRow>,   // source_wallet == dest → cannot self-link to itself
    pub total_sat: Sat,
    pub total_usd_value_floor: Usd,   // [R0-I2] Σ of the Some usd_values — a FLOOR, always a real number
    pub missing_price_count: usize,   // [R0-I2] rows priced None → render "≥ $X (N unavailable)" vs exact "$X"
    pub total_basis_usd: Usd,         // Σ over `included`
}
pub fn bulk_link_transfer_plan(&self, filter: BulkFilter, dest: WalletId)
    -> Result<BulkLinkPlan, CliError>;
```
**Selection** (over `snap.state.pending_reconciliation`, enriched from the event index):
1. Enrich each pending out: `date = tax_date(ev.utc_timestamp, ev.original_tz)`, `source_wallet =
   ev.wallet`, `principal_sat`, `usd_value = btctax_core::price::fmv_of(&prices, date, principal_sat)`
   ([R0-M1] the vetted helper — checked ops + `round_cents` + overflow→`None`; NOT a hand-rolled
   `principal × usd_per_btc`), `basis_usd = Σ leg usd_basis` ([R0-M3] `PendingLeg.usd_basis`, over the
   principal+fee sats the legs cover — a different sat basis than `usd_value`'s principal-only market value;
   both advisory under TP8-(c)).
2. **Frame filter:** `All` → keep all; `Year(y)` → `date.year() == y`; `Range{from,to}` → `from ≤ date ≤ to`.
3. **Source-wallet filter:** if `from_wallet = Some(w)`, keep only `source_wallet == Some(w)`.
4. **Same-wallet guard:** `source_wallet == Some(dest)` → into `skipped_same_wallet` (a self-link to the
   same wallet is meaningless), NOT `included`.
5. Sort `included` by `date`. Totals are over `included` only.
[R0-I2] **Honest floor:** a row with no price → `usd_value = None` and increments `missing_price_count`;
`total_usd_value_floor` is the Σ of the priced rows only. The UI renders exact `$X` when
`missing_price_count == 0`, else **`≥ $X (N unavailable)`** — always a real floor, never a blank marquee.

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
Frame: none of year/from/to → `All`; `year` → `Year`; `from`+`to` → `Range`. **Command
`cmd::reconcile::bulk_link_transfer(vault, pp, filter, dest, confirmed: bool, now) -> Result<Outcome>`**
[R0-M2: a pure helper taking a `confirmed` bool, so the interactive `y/N` stays a thin, untested shell in
`main.rs` dispatch — the CLI has no existing stdin-confirm precedent; only `rpassword` for the passphrase].
It: parses dest + optional from_wallet via `parse_wallet_id`; builds the plan via
`session.bulk_link_transfer_plan(filter, dest)`; returns the plan for the caller to **render the preview
table** (date · source wallet · BTC · USD-value + the totals footer: "**total USD reclassified
non-taxable**" as exact `$X` or `≥ $X (N unavailable)` [R0-I2], and the skipped-same-wallet count). Flow
in dispatch:
- `included.is_empty()` → print "no pending outbound transfers match" and exit 0 (no write).
- `--dry-run` → render the preview and stop (exit 0).
- else render the preview, then confirm: `--yes` ⇒ `confirmed = true`; otherwise dispatch reads an
  interactive `y/N` (default No) → `confirmed`. On `!confirmed` → abort, no write.
- **Apply (atomic), only when `confirmed`:** append a `TransferLink { out_event, in_event_or_wallet:
  Wallet(dest) }` for each `included` row, then a SINGLE `save`. All-or-nothing — the local `Session` is
  discarded on any error before `save`, so a mid-batch append failure writes nothing (mirrors
  `import_selections`, `cmd/reconcile.rs:338` — the exact one-session / N-append / one-save precedent).
  Print "linked N outflows to <dest>; M skipped (same wallet)".

---

## D3 — TUI surface: the bulk flow (`b`)

New Browse key **`b`** (free; capitals only `A`/`G` bound; lowercase `b` unused). `open_bulk_link_transfer_flow`
(latch → snapshot → `pending_reconciliation` non-empty else status "No pending outbound transfers to bulk-link").
A four-step flow on the `TargetList` substrate:

1. **Destination pick** — a wallet pick-list = the DISTINCT wallets across ALL `snap.events` (the same
   union the single `open_link_transfer_flow` offers, `main.rs:3776-3786` — a destination may be a wallet
   that only ever appears as an inbound, so it must be the full event-wallet union, NOT just pending-out
   source wallets), PLUS a **"type a destination wallet…" affordance** [R0 Fork-B EXPAND] — a sentinel row
   (or key `n`) opening a one-line free-text field parsed by `eventref::parse_wallet_id` (the same call
   `--to-wallet` uses, `eventref.rs:57`), so a **never-seen cold wallet** (`self:cold-wallet` →
   `SelfCustody{label}`) is reachable directly in the TUI. This is the HEADLINE case — exchange outflows →
   cold storage, where the destination has no imported events (which is precisely why those outs are
   pending). Parse error → status + stay on the field. Enter → step 2. (FOLLOWUP: backport the typed
   destination to the single `l` flow, which today is pick-list-only per its R0-I2.)
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
        // [R0-I1] append_decision commits per-call to the in-memory conn — a mid-batch failure at row k>1
        // leaves appends 1..k-1 as live residue AND would return a bare NoChange (contract: "vault
        // unchanged") while phantom decisions sit in the DB. Revert the WHOLE batch on ANY append error.
        if let Err(e) = append_decision(session.conn(), payload, now, UtcOffset::UTC, None) {
            return Err(rollback(session, &pre, e.into()));
        }
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

- **One save per bulk op.** Both surfaces append all N `TransferLink`s, then save once. A **mid-batch
  append failure** reverts the ENTIRE batch: TUI routes it through `rollback(session, &pre, e)` [R0-I1]
  (whole-DB restore — `append_decision` commits per-call, so the prior appends MUST be reverted); CLI
  discards the in-memory `Session` (no save happened). A **save failure** reverts via `save_or_rollback`.
  Never a partial apply.
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
4. **KAT-G1:** the batch append lives in `edit/persist.rs` (`append_`). The opener's non-empty guard +
   wallet-union read `snap` DIRECTLY (already KAT-G1-clean, like `open_link_transfer_flow`); only the
   PRICED plan (step 2→3) routes through the Session helper [R0-M4] — no forbidden token in tui-edit
   non-test source either way.
5. **Atomic single save + mid-batch revert** [R0-I1] — do NOT save per-row; and on an append error at row
   k>1, revert the whole batch (don't leak a bare `NoChange` over live phantom appends).
6. **Empty-selection guards** at every gate (plan empty → no open/exit; all unchecked → refuse apply).

## KATs

**btctax-cli:** `bulk_plan_selects_pending_in_frame` (frame + from_wallet filters; same-wallet skipped);
`bulk_plan_usd_total_floor_when_price_missing`; `bulk_cli_dry_run_writes_nothing`;
`bulk_cli_apply_is_atomic_single_save` (N TransferLinks appended, one save, all project as SelfTransfer);
`bulk_cli_no_match_exits_clean`.
**edit/persist.rs:** `kat_persist_bulk_link_strict_prefix` (exactly N TransferLinks tail-appended, all
`Wallet(dest)`); `kat_persist_bulk_link_rolls_back_on_failed_save` (chmod → `Err(RolledBack)`, log
unchanged, retry clean); `kat_persist_bulk_link_reverts_mid_batch_append_failure` [R0-I1] (inject a
failing append at row k>1 → `Err`, event-log byte-unchanged, NO phantom residue, retry clean);
`kat_persist_bulk_link_refuses_empty`.
**main.rs (opener/flow):** `kat_bulk_refuses_when_no_pending`; `kat_bulk_per_row_exclude_drops_row`
(unchecking a row omits it from the appended batch); `kat_bulk_same_wallet_row_absent`;
`kat_bulk_typed_dest_cold_wallet` [Fork B] (typing `self:cold-wallet` — a wallet in NO event — yields a
`SelfCustody` destination the batch links to); E2E `kat_e2e_bulk_link_then_selftransfer` (`b` → filter →
exclude one → confirm → the included outs project as `Op::SelfTransfer`, the excluded one stays pending) +
`kat_e2e_bulk_link_then_void` (one bulk link voids cleanly). **KAT-G1** stays green.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)

- **Task 1 — core plan + CLI** (`Session::bulk_link_transfer_plan`; `Reconcile::BulkLinkTransfer` variant
  + dispatch + `cmd::reconcile::bulk_link_transfer` with preview/dry-run/--yes/atomic apply; CLI KATs).
- **Task 2 — TUI flow** (`b` opener + 4-step flow [dest pick, filter, preview checklist w/ per-row
  exclude, confirm modal]; `persist_bulk_link_transfer`; `derive_bulk_link_status`; draw overlays;
  persist + opener + exclude KATs).
- **Task 3 — E2E round-trips + whole-diff review (Phase E) + FOLLOWUPS** (bulk→selftransfer, bulk→void;
  record deferrals: out→in auto-match, other decision types, `--from/--to` ranges in the TUI, and the
  backport of the typed destination [Fork B] to the single `l` flow).

## Out of scope
- Auto-matching outs to in-events; any decision type other than self-transfer; TUI free-text date RANGE
  (CLI `--from/--to` covers it — the year picker + per-row exclude is the TUI path); undo-the-whole-batch
  as one action (per-row `v`). Never-seen-wallet destinations ARE in the TUI now (Fork B, typed entry).
