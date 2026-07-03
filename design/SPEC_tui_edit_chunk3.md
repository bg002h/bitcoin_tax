# SPEC — btctax-tui-edit chunk 3: select-lots + set-donation-details + safe-harbor-attest

**Source baseline:** `main` @ `7ba67a1` (working tree re-verified file-by-file at write time; all
line citations checked against the current source).
**Review status: R0 round 1 FOLDED — awaiting round 2**
**R0 round 1:** `reviews/R0-spec-tui-edit-chunk3-round-1.md` (1 Critical / 7 Important /
10 Minor / 2 Nit) — ALL findings folded below; fold tags `[R0-C1]`…`[R0-N2]` mark the sites.
**Goal:** Chunk 3 of the mutating-TUI program — three new flows added to the existing
`btctax-tui-edit` crate:

1. **select-lots** — specific-identification lot assignment for a method-honoring disposal.
   Appends `EventPayload::LotSelection{disposal_event, lots: Vec<LotPick>}`.
2. **set-donation-details** — Form 8283 Section-B appraiser + donee metadata for a Donation
   removal. Writes to the `donation_details` side-table (NOT a decision append — last-write-wins
   upsert, identical in structure to chunk-1's `persist_tax_profile`).
3. **safe-harbor-attest** — cures a `SafeHarborTimebar` blocker by voiding the single live
   prior allocation and re-appending it with `timely_allocation_attested = true`. Appends
   TWO decisions atomically. **IRREVOCABLE once effective (§7.4); requires typed-word
   confirmation.**

All three flows reuse the chunk-2b substrate verbatim: the `TargetList<T>` widget, the modal →
flow → form dispatch order with `q`-swallow and Esc-steps-back, the `edit/persist.rs` persist
pattern with `now`-injection, the blocker-derived status discipline, the strict-prefix KATs,
per-flow cancel-bytes + save-failure KATs, and the 2a/2b retry-story discipline — instantiated
per flow.

**Attest scoping call (recon finding):** `btctax reconcile safe-harbor-attest` is a
RECONCILE-level subcommand (reconcile.rs:468-564), not config-level. It appends two decisions
(VoidDecisionEvent + SafeHarborAllocation) atomically and is fully governed by §7.4
irrevocability once the re-attested allocation becomes effective. It belongs in chunk 3 as a
decision flow with typed-word confirmation, not in a config flow. `safe-harbor-allocate`
(creating the allocation) remains out of scope for chunk 3 (2b Out of Scope list).

**SemVer:** Two new `pub fn` items in `edit/persist.rs` (`persist_select_lots`,
`persist_safe_harbor_attest`); one new side-table persist call (`persist_donation_details`,
mirroring chunk-1's `persist_tax_profile`); three new flow struct families in `edit/form.rs`
(two with confirmation modals; the attest flow's TypedWord step is its own gate, D3); the
`attest_save_failed` latch field on `EditorApp` [R0-C1]; key bindings `s` / `d` / `a` in
`main.rs`. No new workspace member, no new `[lib]` targets, no `btctax-core` changes.
**MINOR** (pre-1.0; additive).

---

## Hard constraints (chunk-1 + chunk-2b guarantees inherited verbatim)

The editor's crate-level guarantee is **unchanged by this spec**:

> "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`, each behind
> an explicit payload-showing confirmation; the vault file only via `Vault::save`'s atomic path."

Chunk-3 instantiation:

- `persist_select_lots` and `persist_safe_harbor_attest` are the ONLY two new append-decision
  writers; both live exclusively in `edit/persist.rs` (the sole allowlisted module in KAT-G1).
- `persist_donation_details` is a new side-table writer in `edit/persist.rs`; it calls
  `donation_details::set(session.conn(), &event_id, &details)` then `session.save()` — never
  `append_decision`. `"donation_details::set"` is ADDED to `persist_only_tokens` so KAT-G1
  mechanically forbids it outside `edit/persist.rs` [R0-I4].
- `persist_safe_harbor_attest` appends BOTH a `VoidDecisionEvent` and a `SafeHarborAllocation`
  atomically (same in-memory Connection, single `session.save()`), mirroring
  `cmd::reconcile::safe_harbor_attest` (reconcile.rs:541-563).
- `persist_select_lots` appends `EventPayload::LotSelection` and calls `session.save()`. It
  does NOT write to the `optimize_attestation` side-table (mirrors the CLI `select-lots` command:
  reconcile.rs:325-352). Clearing `optimize_attestation` happens only on void of a LotSelection
  (chunk-2b `persist_void`, already shipped).
- `now: OffsetDateTime` is INJECTED at Enter-press for test determinism on all three new writers.
- Per-mutation confirmation: the modal precedes ALL writes for all three flows.

**Failed-save + retry semantics:**

- **`persist_select_lots`** (duplicate ⇒ conflict, NEITHER applies: resolve.rs:787-800 — a
  second non-voided LotSelection for the same `disposal_event` fires `DecisionConflict` on the
  SECOND decision's id, and `selections.remove(..)` then drops the first as well; the disposal
  falls back to METHOD ORDER until one of the two is voided) [R0-I2]. A retry after a failed
  save appends exactly such a duplicate. Post-persist status surfaces this (D1 status arm 1).
  Remedy: void the duplicate (void flow, press `v`) to reinstate the FIRST selection's picks;
  if the user EDITED picks before re-submitting, voiding the FIRST instead keeps the edited
  picks — the conflict clears either way.
- **`persist_donation_details`** (last-write-wins upsert, `donation_details` side-table — no
  decision append). A retry is identical to a re-submit: the upsert simply overwrites. No
  conflict. Status after retry: clean-success.
- **`persist_safe_harbor_attest`** (two-decision batch: Void + re-attested SafeHarborAllocation).
  On `Err(save)` the on-disk vault is unchanged (atomic `Vault::save`, NFR2/NFR3) but BOTH
  appends remain in the in-memory Connection — see the residue latch below [R0-C1]. There is
  NO TUI retry path (close-on-Err): a second confirmed batch would append Void again (inert —
  allocation-targeted voids take the resolve.rs:322-328 arm into `allocation_voids`, a `Vec`
  whose duplicates accumulate harmlessly; the §7.4 conflict fires only for EFFECTIVE targets,
  and the prior is timebarred-inert [R0-M1(d)]) + SafeHarborAllocation again → TWO effective
  allocations. **The double-batch consequence is unrecoverable** [R0-M2]: each copy conserves
  independently, so `effective.len() == 2` → Hard
  `DecisionConflict("multiple effective SafeHarborAllocations")` + Path-A fallback
  (resolve.rs:958-967), and per resolve.rs:924-933 voiding EITHER copy then fires the §7.4
  conflict — a permanently Hard-gated vault. This is why the flow closes entirely on Err (a
  deliberate break from the "keep form open on Err" pattern — stated so no implementer
  "fixes" it), the latch blocks all further mutating flows, and the only safe remediation is
  to QUIT the editor (discarding the in-memory residue) and attest via the CLI.

**Unsaved-residue latch — attest save-Err [R0-C1]:**

The editor's `Session` holds the store's exclusive `VaultLock` for the editor's entire lifetime
(editor.rs:8-14; session.rs:53-58) — **the CLI cannot open the vault until the editor quits**
(`StoreError::Locked`). And `session.save()` serializes the WHOLE in-memory DB, so after a
failed attest save the confirmed irrevocable Void+Attest batch would silently piggy-back onto
the NEXT successful save of ANY unrelated confirmed mutation — violating "persisted only when
a confirmation's own save succeeds". Both hazards are closed as follows:

- New `EditorApp` field: `attest_save_failed: bool` (default `false`), set to `true` ONLY in
  the attest flow's `Err` arm.
- While `true`, EVERY mutating-flow opener (`p`/`c`/`o`/`r`/`f`/`v`/`s`/`d`/`a`) refuses with
  status `"A failed attest save left unsaved decisions in memory — quit the editor (the
  unsaved attestation is discarded on quit), then retry via CLI: btctax reconcile
  safe-harbor-attest"`. Read-only navigation (tabs, scrolling, `q` to quit) is unaffected;
  quitting discards the residue (the on-disk vault is pre-action).
- Defense-in-depth: even bypassing the latch, re-pressing `a` hits the session-sourced
  pre-flight (ONE `session.load_events_and_project()`, D3/[R0-I5]) which sees the in-memory
  already-attested allocation and refuses via the "already attested" arm, appending NOTHING.
- Pinned by KAT-E2E-ATTEST-ERRLATCH (D5).
- **CLI-pointing statuses (audit):** every status message in this spec that names a CLI
  command says to quit the editor first — the lock makes an in-editor "retry via CLI"
  physically impossible.

**Dispatch order invariant (extending chunk-2b's 9-layer dispatch to 11 layers [R0-M4]):**

```
1. mutation_modal              (chunk 1)
2. classify_inbound_modal      (chunk 2a)
3. reclassify_outflow_modal    (chunk 2a)
4. reclassify_income_modal     (chunk 2b)
5. set_fmv_modal               (chunk 2b)
6. void_modal                  (chunk 2b)
7. select_lots_modal           (chunk 3)
8. set_donation_details_modal  (chunk 3)
9. Flow layer — any open flow claims ALL keys; q swallowed, Esc steps back.
   The attest flow (incl. its TypedWord step — there is NO separate attest modal, D3)
   is handled entirely here.
10. Form layer — profile_form (chunk 1)
11. Screen dispatch
```

At most one flow `Some` and at most one modal `Some` at any time.

---

## Pre-filter verification

### Claim F — select-lots list: method-honoring disposals without a non-voided LotSelection

**Basis: resolve.rs:758-824 (pass-2c, LotSelection collection + validation); state.rs:133-179
(Disposal, RemovalKind, Removal) [R0-N2]; honoring_principal fn (resolve.rs:1008-1016).**

`honoring_principal` returns `Some(sat)` for `Op::Dispose`, `Op::GiftOut`, `Op::Donate`,
`Op::SelfTransfer`; returns `None` for all other ops (`PendingOut`, fee legs, non-disposals).
Fee-mini-disposition Disposals (`Disposal.fee_mini_disposition == true`) arise from TP8-(b)
fee-sat mini-disposal ops. Their exclusion from the TUI list is via the
`!d.fee_mini_disposition` flag filter — NOT via the honoring filter: a TP8-(b) fee record
shares its SelfTransfer's event id, and `honoring_principal(Op::SelfTransfer)` IS
`Some(principal)` (resolve.rs:1008-1016) [R0-M7]. (Together with the acknowledged SelfTransfer
under-inclusion below, no fee-mini row can reach the list.)

**Method-honoring disposal sources in the Snapshot:**
- `snap.state.disposals`: `Vec<Disposal>` where `Disposal.event: EventId`,
  `Disposal.legs: Vec<DisposalLeg>`. Principal sat = `Σ d.legs.iter().map(|l| l.sat).sum()`.
  Exclude `fee_mini_disposition == true`.
- `snap.state.removals`: `Vec<Removal>` where `Removal.event: EventId`,
  `Removal.kind: RemovalKind` (Gift or Donation). Principal sat = `Σ r.legs.iter().map(|l|
  l.sat).sum()`. BOTH Gift and Donation removals are method-honoring targets (`Op::GiftOut` and
  `Op::Donate` both return `Some(sat)` from `honoring_principal`). The `set-donation-details`
  filter (Claim G) is separately restricted to `Donation` only; select-lots is NOT.
- SelfTransfer linked events: these appear as `Op::SelfTransfer` and are nominally selectable,
  but they are rare and their principal is not directly exposed in `snap.state` without
  iterating raw events. **Known under-inclusion:** SelfTransfer events are EXCLUDED from the
  select-lots TUI list (the disposals + removals sources above do not include them). Under-
  inclusion only (safe direction — the CLI select-lots remains available); recorded in FOLLOWUPS.

**Pre-filter for the select-lots disposal list:**

```rust
// Build voided-decision set and already-selected disposal set (hoisted once).
let voided: BTreeSet<&EventId> = snap.events.iter()
    .filter_map(|e| {
        if let EventPayload::VoidDecisionEvent(v) = &e.payload {
            Some(&v.target_event_id)
        } else { None }
    })
    .collect();

let already_selected: BTreeSet<&EventId> = snap.events.iter()
    .filter(|e| !voided.contains(&e.id))
    .filter_map(|e| {
        if let EventPayload::LotSelection(ls) = &e.payload {
            Some(&ls.disposal_event)
        } else { None }
    })
    .collect();

// Disposals (sell / spend).
let disposal_items: Vec<DisposalListItem> = snap.state.disposals.iter()
    .filter(|d| !d.fee_mini_disposition)
    .filter(|d| !already_selected.contains(&d.event))
    .map(|d| DisposalListItem { /* from d */ })
    .collect();

// Removals (gift / donation — BOTH kinds).
let removal_items: Vec<DisposalListItem> = snap.state.removals.iter()
    .filter(|r| !already_selected.contains(&r.event))
    .map(|r| DisposalListItem { /* from r */ })
    .collect();

// Merge and sort by date (desc = most recent first, matching the display tabs).
let mut items = [disposal_items, removal_items].concat();
items.sort_by(|a, b| b.date.cmp(&a.date));
```

**Duplicate LotSelection ⇒ conflict, NEITHER applies** (resolve.rs:787-800) [R0-I2]: a second
non-voided LotSelection for the same `disposal_event` fires `DecisionConflict` on the SECOND
decision's id, and `selections.remove(..)` then drops the FIRST as well — "a conflicted
disposal applies NEITHER selection" (resolve.rs:762, 799); the disposal falls back to METHOD
ORDER while both are live. Voiding the duplicate reinstates the first; voiding the first keeps
a re-edited duplicate. The pre-filter excluding already-selected disposals prevents the user
from selecting an already-selected disposal via the TUI. The only way to reach a duplicate is
the failed-save-retry path (Hard constraints above).

**Validation (principal conservation):** `Σ pick.sat == disposal_principal_sat` is checked
by the TUI before opening the modal. This mirrors resolve.rs:811-820 (the static check). The
engine's fold additionally validates lot existence, per-wallet, and over-draw; those are
engine-side and may produce `LotSelectionInvalid` blockers even for a TUI-valid selection.

---

### Claim G — set-donation-details list: Donation removals only

**Basis: reconcile.rs:596-631 (`set_donation_details`); state.rs:144-178 (RemovalKind,
Removal); donation.rs:1-80 (DonationDetails); donation_details.rs:1-88 (side-table).**

The CLI `set_donation_details` (reconcile.rs:619-625) validates against projected
`state.removals`: only `RemovalKind::Donation` is accepted; `RemovalKind::Gift` fires
"not a donation" usage error. The TUI mirrors this: only `snap.state.removals` entries where
`r.kind == RemovalKind::Donation` appear in the list.

**Pre-filter:**
```rust
snap.state.removals.iter()
    .filter(|r| r.kind == RemovalKind::Donation)
    // All Donation removals are actionable — there is no "already-complete" exclusion
    // (set-donation-details is last-write-wins; re-setting is always valid).
```

**No already-set exclusion:** unlike the pre-filter for reclassify-income or set-fmv, there
is no pre-filter excluding donations that already have stored details — re-setting is the
INTENDED workflow (the user progressively fills in appraiser fields). If details are stored,
the form is pre-populated with the existing values (see D2).

**DonationDetails fields (donation.rs:17-48):** 10 fields, 2 REQUIRED:
- `donee_name: String` (required by CLI — `clap(required = true)`, main.rs:315)
- `appraiser_name: String` (required by CLI — `clap(required = true)`, main.rs:323-324) [R0-M1(c)]
- 8 optional: `donee_address`, `donee_ein`, `appraiser_address`, `appraiser_tin`,
  `appraiser_ptin`, `appraiser_qualifications`, `appraisal_date` (Option<TaxDate>),
  `fmv_method_override` (Option<String>).

**`is_review_complete` (donation.rs:68-79):** Section B (year-aggregate > $5k) requires all of:
appraiser_name non-empty, (appraiser_tin OR appraiser_ptin), appraisal_date, appraiser_qualifications,
donee_ein. Section A (≤ $5k): complete on presence (any details stored). The TUI does not
compute Section A/B at form time (that requires year-aggregate FMV, which the engine computes
globally). Post-save status uses `is_review_complete(Form8283Section::B)` as the conservative
check and notes that Section A is complete on presence.

---

### Claim H — safe-harbor-attest pre-flight: single live timebarred allocation

**Basis: reconcile.rs:468-564 (`safe_harbor_attest`); state.rs:22-82 (BlockerKind —
SafeHarborTimebar, SafeHarborUnconservable); event.rs:161-173 (SafeHarborAllocation).**

The CLI `safe_harbor_attest` runs the following pre-flight before any writes:
1. Build the voided-decision set (VoidDecisionEvent targets).
2. Collect live (non-voided) `SafeHarborAllocation` events.
3. Error if count == 0: "no allocation to attest; run safe-harbor allocate first".
4. Error if count > 1: "multiple live allocations present; void the stale one before attesting".
5. Error if `prior.timely_allocation_attested`: "allocation is already attested".
6. Check projected state.blockers for `prior_id`:
   - `SafeHarborUnconservable` → "allocation fails conservation; re-run safe-harbor-allocate".
   - NOT `SafeHarborTimebar` (i.e., allocation is already-effective) → "allocation already
     effective; no attestation needed — run verify".
   - `SafeHarborTimebar` present → proceed.

All pre-flight checks run IN the `open_safe_harbor_attest_flow` Browse-key handler (before any
flow state is set), sourced from ONE `session.load_events_and_project()` call (the CLI's exact
shape, reconcile.rs:473-474; the method name carries no persist-only token, so it is
KAT-G1-legal in `main.rs`) [R0-I5]: the event scans read the returned `events`; the blocker
checks read the returned `state.blockers`. NO mixing with the cached `snap` — session events
and cached-snapshot projection agree only when no unsaved residue exists, and the
session-sourced already-attested arm is the defense-in-depth guard against the C1 double-batch
(Hard Constraints). Failures show a status message and return to Browse without opening the
flow [R0-M8 discipline].

**The timebar condition (state.rs:78):** `SafeHarborTimebar` is Advisory (never gates
compute_tax_year); it fires when the allocation exists but the §5.02(4) 12-month deadline has
not been cured by attestation. The attest operation cures this PRECISELY by re-appending with
`timely_allocation_attested = true` (reconcile.rs:551-562). Once attested, the allocation
passes the timeliness check → becomes effective → §7.4 irrevocable.

---

## Current state (recon @ `7ba67a1` — chunk 2b is shipped)

All chunk-2b infrastructure is live and tested. The shipped source baseline for chunk 3 is:

- **`crates/btctax-tui-edit/src/main.rs`** — `handle_key` dispatch order: 6 modal layers (1–6)
  + flow layer (5 flows: classify_inbound, reclassify_outflow, reclassify_income, set_fmv,
  void) + form layer + screen (main.rs:79–225). Browse key bindings: `p/c/o/r/f/v`.
  Keys `s`, `d`, `a` are FREE at HEAD and not claimed by any existing handler.
- **`crates/btctax-tui-edit/src/edit/persist.rs`** — contains `persist_tax_profile` (chunk 1),
  `persist_classify_inbound`, `persist_reclassify_outflow` (chunk 2a), `persist_reclassify_income`,
  `persist_set_fmv`, `persist_void` (chunk 2b). The `"append_"` token is in
  `persist_only_tokens` (persist.rs:685) [R0-M1(b)]. Two new append writers + one side-table
  writer land here.
- **`crates/btctax-tui-edit/src/edit/form.rs`** — `TargetList<T>`, all chunk-2b flow/modal
  structs. Three new flow/modal struct families added here.
- **`crates/btctax-tui-edit/src/editor.rs`** — `EditorApp` carries 10 Option fields (5 flows +
  5 modals for chunks 1/2a/2b). Chunk 3 adds 5 more (3 flows + 2 modals — the attest flow has
  no separate modal, D3 [R0-M4]) plus the `attest_save_failed: bool` latch [R0-C1].
- **`crates/btctax-cli/src/donation_details.rs`** — `get(conn, event)`, `set(conn, event,
  details)`, `all(conn)`, `init_table(conn)` — the side-table accessors used by `persist_donation_details`.
- **`crates/btctax-core/src/donation.rs`** — `DonationDetails` (10 fields, 2 required) +
  `is_review_complete(Form8283Section)`.
- **`crates/btctax-core/src/state.rs`** — `LedgerState.disposals: Vec<Disposal>`,
  `LedgerState.removals: Vec<Removal>` — the two filter sources for the select-lots list.
- **`crates/btctax-core/src/event.rs`** — `LotSelection{disposal_event, lots}`, `LotPick`,
  `LotId{origin_event_id, split_sequence}`, `SafeHarborAllocation` — all existing payload types.
  No new core types needed.

---

## Design

### D1 — select-lots flow

**Key binding:** `s` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (disposal list).** Build `TargetList<DisposalListItem>` from the snapshot
using the pre-filter in Claim F. If empty: status `"No method-honoring disposals available for lot
selection"`, return to Browse [R0-M8 — empty-list no-open discipline].

Render as a `Table` with columns: `Date | Kind | Principal Sat | Wallet | EventId`. Kind
column: `"sell"`, `"spend"`, `"gift"`, `"donate"`. Title:
`" Select Lots — select disposal event "`.

**Display data type:**

```rust
pub struct DisposalListItem {
    pub disposal_event: EventId,
    pub date: TaxDate,
    pub kind: DisposalKind,      // Sell | Spend | Gift | Donate
    pub principal_sat: Sat,      // Σ legs.sat (from Disposal or Removal)
    pub wallet: Option<WalletId>,
}

pub enum DisposalKind { Sell, Spend, Gift, Donate }
```

`date` from `Disposal.disposed_at` or `Removal.removed_at`. **`wallet` — for ALL list items —
is sourced from the raw ledger event**: `events_by_id(snap)[&item.disposal_event].wallet.clone()`
(`LedgerEvent.wallet: Option<WalletId>`, event.rs:297-304; the `events_by_id` helper exists at
main.rs:1765-1769) [R0-I1]. It CANNOT come from the legs: `DisposalLeg` has a `wallet` field
(state.rs:131) but **`RemovalLeg` has none** (state.rs:148-163) — Gift/Donate rows would have
no wallet source. `None` (edge case): display `"(no wallet)"`.

**Step 2 — lot-pick form.** After `Enter` on a disposal item, transition to
`SelectLotsStep::LotsForm`. Build a `Vec<LotPickFormRow>` from `snap.state.lots` filtered to
the disposal's wallet: match `item.wallet` — `Some(w)` → keep lots with `l.wallet == w`;
`None` → zero lots, which lands in the existing "no lots" error below [R0-I1]. Sort by
`acquired_at` ascending (oldest first — the Specific-ID natural display order; FIFO lots are
at the top). If no lots for the wallet: show error `"No lots available for wallet {wallet};
check Holdings"` and remain at the List step.

The LotsForm is a scrollable multi-row form. Each row displays one lot with an editable `pick_sat`
field. The user navigates rows with ↑/↓ and types digit characters to set the sat amount for the
focused row.

| Column | Source |
|---|---|
| `Acquired` | `Lot.acquired_at` |
| `LotId` | `{lot_id.origin_event_id.canonical()}#{lot_id.split_sequence}` |
| `Remaining Sat` | `Lot.remaining_sat` (display only; not the validation target) |
| `Basis/Sat (USD)` | `Lot.usd_basis / Lot.remaining_sat` (display only) |
| `Pick Sat` | editable FieldBuffer (initially empty = 0; display `"0"` when empty) |

A running-total footer line: `"Picked: {Σ_pick_sat} / {disposal_principal_sat} sat"`.

**Known display caveat (stated, not a blocker):** `snap.state.lots` reflects the CURRENT
projected lot pool (after all existing method-order consumption). Lots consumed by the current
method-order for THIS disposal may not appear (or may appear with reduced `remaining_sat`).
After the LotSelection is appended and the projection re-runs, the actual lot pool at the
disposal date governs. The engine validates lot existence, per-wallet, and over-draw on each
re-projection (producing `LotSelectionInvalid` blockers if the picked lots are invalid). This
is intentional: `cmd::reconcile::select_lots` "does NOT attempt to validate up-front"
(reconcile.rs:325-329). Stated in FOLLOWUPS; the display is a best-effort guide.

**Flow state struct:**

```rust
pub enum SelectLotsStep {
    List,
    LotsForm {
        item: DisposalListItem,
        rows: Vec<LotPickFormRow>,
        cursor: usize,      // focused row index
        error: Option<String>,
    },
}

pub struct LotPickFormRow {
    pub lot: LotId,
    pub remaining_sat: Sat,
    pub acquired_at: TaxDate,
    pub usd_basis: Usd,
    pub pick_sat_buf: FieldBuffer,  // digits only; initially empty
}

impl LotPickFormRow {
    /// Parse pick_sat_buf as i64; 0 when empty.
    fn pick_sat(&self) -> Result<Sat, String> {
        if self.pick_sat_buf.is_empty() { return Ok(0); }
        self.pick_sat_buf.as_str().trim().parse::<i64>()
            .map_err(|e| format!("bad sat in row {}: {e}", self.lot.origin_event_id.canonical()))
    }
}

pub struct SelectLotsFlowState {
    pub list: TargetList<DisposalListItem>,   // OWNED by the flow [R0-I2 discipline]
    pub step: SelectLotsStep,
}
```

**LotsForm keys:**
| Key | Action |
|---|---|
| `↑` / `k` | `cursor = cursor.saturating_sub(1)` |
| `↓` / `j` | `cursor = (cursor + 1).min(rows.len().saturating_sub(1))` |
| `g` | `cursor = 0` |
| `G` | `cursor = rows.len().saturating_sub(1)` |
| `0`…`9` | push digit to `rows[cursor].pick_sat_buf` (FIELD_CAP=64 cap) |
| `Backspace` | pop last byte from `rows[cursor].pick_sat_buf` |
| `Enter` | validate → open `select_lots_modal` if valid, else set error |
| `Esc` | back to the **List** step (one step per press: LotsForm → List → close flow [I4]) |
| `q` | SWALLOWED (flow is blocking) |

**Validation (pure fn, validate-at-submit):**
1. Parse every `pick_sat_buf` → error on any non-integer.
2. Collect only rows with `pick_sat > 0` → `Vec<LotPick>`.
3. If no picks (all zero): error `"pick at least one lot"`.
4. `Σ picked_sat` must equal `item.principal_sat`: error `"picked {Σ} sat != disposal principal {principal} sat; adjust to match exactly"`.
5. Build `EventPayload::LotSelection(LotSelection { disposal_event: item.disposal_event.clone(), lots: picks })`.

**Modal content — select-lots:**

```rust
pub struct SelectLotsModalState {
    pub disposal_event: EventId,
    pub disposal_date: TaxDate,
    pub disposal_kind: DisposalKind,
    pub principal_sat: Sat,
    pub picks: Vec<LotPick>,   // the validated picks (non-zero only)
    pub pick_count: usize,     // display summary
    pub total_sat: Sat,        // Σ picks.sat (== principal_sat by construction)
}
```

Modal rendering:
```
╔═ Confirm: select-lots — WRITES THE VAULT ════════════════╗
║  disposal: import|coinbase|out-001  (sell)               ║
║  date:     2025-09-15                                     ║
║  principal: 500000 sat                                    ║
║                                                           ║
║  Picks: 2 lot(s), 500000 sat total                        ║
║    import|coinbase|in-001#0   →  300000 sat               ║
║    import|coinbase|in-002#1   →  200000 sat               ║
║                                                           ║
║  Appended as a decision event (append-only log).          ║
║  Saved immediately via the vault's atomic write path.     ║
║                                                           ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing ║
╚═══════════════════════════════════════════════════════════╝
```

Picks listed individually (one line per pick: `{lot_id}#{seq} → {sat} sat`) up to the first
**8** picks; beyond that, the overflow line `"… and {K} more picks ({sat} sat in the
remainder)"` — the pick COUNT and TOTAL are always shown in the summary line, so the payload
is fully characterized even when elided [R0-M8]. No tax-figure estimates in the modal (engine
may fire `LotSelectionInvalid` on re-projection; the modal shows only what will be persisted).

**Post-effect status — derived from RE-PROJECTED state:**

```
derive_select_lots_status(snap, &disposal_event, &decision_id) → String
```

- **`DecisionConflict` attributed to `decision_id`** (failed-save-retry duplicate — NEITHER
  selection applies; method-order fallback until one is voided [R0-I2]):
  → `"Saved, but DecisionConflict fired — neither selection applies (method order governs);
     clear with Void flow (press 'v'), or quit the editor and run: btctax reconcile void
     {decision_id.canonical()}"` [R0-C1 lock audit].
- **`LotSelectionInvalid` with `event == disposal_event`** (engine rejected the selection:
  unknown lot, cross-wallet, over-draw, or principal mismatch detected by the fold):
  → `"LotSelection saved but invalid — see Compliance for detail; the disposal falls back to
     method order. Correct via Void flow (press 'v') then re-select."`.
- **Clean (neither of the above):** → `"Lot selection recorded for {disposal_event.canonical()} —
  {pick_count} lot(s), {total_sat} sat; check Compliance for §1.1012-1(j) contemporaneity."`.

**Enter-arm semantics:** `Ok(id)` → re-project + derive status + close modal + close flow;
`Err(e)` → close modal, keep LotsForm open (buffers intact), status = `"Save error: {e}"`.

---

### D2 — set-donation-details flow

**Key binding:** `d` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (donation list).** Build `TargetList<DonationListItem>` from the snapshot
using the pre-filter in Claim G. If empty: status `"No donation removals found (donate a
TransferOut first via reclassify-outflow)"`, return to Browse [R0-M8].

Render as a `Table` with columns: `Date | Sat | Donee (if any) | Completeness | EventId`.
`Completeness` column: `"B-complete"` if
`item.existing_details.as_ref().is_some_and(|d| d.is_review_complete(Form8283Section::B))`,
`"present"` if stored but not B-complete, `"(none)"` if no details stored yet. Stored details
come from **`snap.donation_details`** (`BTreeMap<EventId, DonationDetails>` — a field the
`Snapshot` ALREADY carries: btctax-tui/src/app.rs:104-111, populated by `build_snapshot` via
`session.donation_details()`, unlock.rs:177,185). `main.rs` makes NO `donation_details::get`
calls — `conn(` is a persist-only token forbidden outside `edit/persist.rs` (KAT-G1,
persist.rs:685); the snapshot is the only KAT-G1-clean read source [R0-I3(a)].
Title: `" Set Donation Details — select Donation event "`.

**Display data type:**

```rust
pub struct DonationListItem {
    pub event_id: EventId,
    pub date: TaxDate,
    pub total_sat: Sat,               // Σ removal.legs.iter().map(|l| l.sat).sum()
    pub donee: Option<String>,        // from Removal.donee (the free-form label if any)
    pub existing_details: Option<DonationDetails>, // snap.donation_details.get(&event_id).cloned() [R0-I3]
}
```

**Step 2 — field form.** After `Enter` on a list item, transition to
`SetDonationDetailsStep::FieldForm`. The form is pre-populated from `item.existing_details` if
present (all 8 optional fields pre-filled with their stored values; the user edits and re-submits
to update).

Fields (10 total, matching DonationDetails struct exactly):

| Field | Buffer | Required? | Validation |
|---|---|---|---|
| `donee_name` | `FieldBuffer` | **REQUIRED** | empty → `"donee-name is required"` |
| `donee_address` | `FieldBuffer` | optional | empty → `None` |
| `donee_ein` | `FieldBuffer` | optional | empty → `None` |
| `appraiser_name` | `FieldBuffer` | **REQUIRED** | empty → `"appraiser-name is required"` |
| `appraiser_address` | `FieldBuffer` | optional | empty → `None` |
| `appraiser_tin` | `FieldBuffer` | optional | empty → `None` |
| `appraiser_ptin` | `FieldBuffer` | optional | empty → `None` |
| `appraiser_qualifications` | `FieldBuffer` | optional | empty → `None` |
| `appraisal_date` | `FieldBuffer` | optional | empty → `None`; non-empty → `parse_date_arg(trim)` (YYYY-MM-DD) |
| `fmv_method_override` | `FieldBuffer` | optional | empty → `None` |

Focus order: donee_name (0) → donee_address (1) → donee_ein (2) → appraiser_name (3) →
appraiser_address (4) → appraiser_tin (5) → appraiser_ptin (6) → appraiser_qualifications (7)
→ appraisal_date (8) → fmv_method_override (9). `↑/↓` move focus. `Enter` validates →
opens modal. `Esc` → back to the **List** step [I4 — Esc-steps-back: FieldForm → List → close flow].

**Flow state struct:**

```rust
pub enum SetDonationDetailsStep {
    List,
    FieldForm {
        item: DonationListItem,
        donee_name_buf: FieldBuffer,
        donee_address_buf: FieldBuffer,
        donee_ein_buf: FieldBuffer,
        appraiser_name_buf: FieldBuffer,
        appraiser_address_buf: FieldBuffer,
        appraiser_tin_buf: FieldBuffer,
        appraiser_ptin_buf: FieldBuffer,
        appraiser_qualifications_buf: FieldBuffer,
        appraisal_date_buf: FieldBuffer,
        fmv_method_override_buf: FieldBuffer,
        focus: usize,
        error: Option<String>,
    },
}

pub struct SetDonationDetailsFlowState {
    pub list: TargetList<DonationListItem>,
    pub step: SetDonationDetailsStep,
}
```

**Validation (pure fn, validate-at-submit):**
1. `donee_name`: REQUIRED (empty → error).
2. `appraiser_name`: REQUIRED (empty → error).
3. `appraisal_date`: if non-empty → `parse_date_arg(trim)` (YYYY-MM-DD → error on bad format).
4. All other optionals: empty → `None`.
5. Build `DonationDetails { donee_name, donee_address, donee_ein, appraiser_name, …, fmv_method_override }`.

**Modal content — set-donation-details:**

```rust
pub struct SetDonationDetailsModalState {
    pub event_id: EventId,
    pub event_date: TaxDate,
    pub total_sat: Sat,
    pub details: DonationDetails,      // the VALIDATED details payload
}
```

Modal rendering:
```
╔═ Confirm: set-donation-details — WRITES THE VAULT ══════╗
║  event:  import|coinbase|out-001  (Donation)            ║
║  date:   2025-06-15                                     ║
║  sat:    500000                                         ║
║                                                         ║
║  donee_name:           Community Foundation             ║
║  donee_ein:            12-3456789                       ║
║  appraiser_name:       Jane Appraiser                   ║
║  appraiser_tin:        987654321                        ║
║  appraisal_date:       2025-05-20                       ║
║  appraiser_qualifications: certified bitcoin appraiser  ║
║  [optional fields omitted when None]                    ║
║                                                         ║
║  Stored in side-table (last-write-wins; not a decision  ║
║  event). Saved via vault's atomic write path.           ║
║                                                         ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

Only non-None fields are shown. A footer line explicitly states "last-write-wins; not a decision
event" — this is a critical distinction from the append-only flows.

**Post-effect status — derived from the IN-HAND validated `details` [R0-I3(c)]:**

```
derive_donation_details_status(&event_id, &details) → String
```

Last-write-wins guarantees the value just written IS the stored value (the disk round-trip
stays pinned by KAT-DD-PERSIST), so the status calls `details.is_review_complete(..)` directly
— no side-table re-load, no `conn(` in `main.rs`:
- `is_review_complete(Form8283Section::B) == true` →
  `"Details saved for {event_id.canonical()} — Section B complete (§6695A fields present)"`.
- `is_review_complete(Form8283Section::B) == false` →
  `"Details saved for {event_id.canonical()} — Section A complete on presence; add appraiser
   TIN/PTIN + appraisal date + qualifications + donee EIN for Section B completeness"`.

**Enter-arm semantics:** `Ok(())` → REBUILD the snapshot exactly like every other flow (the
set-fmv Enter arm, main.rs:1318-1339) — this refreshes `snap.donation_details`, so the `d`
list's Completeness column, form pre-population, and any Forms-tab consumers immediately
reflect the new details [R0-I3(b)] — then derive the status from the in-hand `details`, close
modal, close flow. `Err(e)` → close modal, keep FieldForm open (buffers intact), status =
`"Save error: {e}"`.

---

### D3 — safe-harbor-attest flow (IRREVOCABLE — typed-word confirmation)

**Key binding:** `a` in the Browse screen (no-op when `snapshot.is_none()`).

**Irrevocability UX design.** The safe-harbor-attest is the ONLY fully irrevocable action in
chunk 3. Per the baseline requirement ("any flow with IRREVOCABLE effects gets a distinct
double-confirmation or typed-word modal"), this flow uses a **typed-word confirmation** where
the user must type `ATTEST` (all-caps, 6 characters, case-sensitive) to confirm. This is
modeled on the GitHub typed-word deletion pattern: the cost of typing a specific word prevents
accidental confirmation while making the irrevocability viscerally clear.

The irrevocability is §7.4-based: once the re-attested SafeHarborAllocation passes the engine's
§7.4 conservation + timeliness checks (it will, having been previously validated for conservation
and now cured of the time-bar), it becomes effective and cannot be voided (any void attempt
fires Hard `DecisionConflict`, resolve.rs:924-933 [R0-M1(a)], and the allocation stays in force).

**Step 0 — latch check [R0-C1].** If `app.attest_save_failed` → status `"A failed attest save
left unsaved decisions in memory — quit the editor (the unsaved attestation is discarded on
quit), then retry via CLI: btctax reconcile safe-harbor-attest"`, return (this same check
guards every mutating opener; Hard Constraints).

**Step 1 — pre-flight (in Browse key handler, before any flow state is set).**
`open_safe_harbor_attest_flow(app)` runs the pre-flight synchronously off ONE
`let (events, state, _config) = session.load_events_and_project()?` [R0-I5] — no cached-`snap`
reads anywhere in the pre-flight:
1. Build voided set; collect live (non-voided) `SafeHarborAllocation` events from `events`.
2. Match on count:
   - 0 → status `"No allocation to attest — quit the editor, then run: btctax reconcile
     safe-harbor-allocate"`, return.
   - 2+ → status `"Multiple live allocations present — void the stale one (press 'v') before
     attesting"`, return.
   - 1 → continue.
3. If `prior.timely_allocation_attested` → status `"Allocation already attested — nothing to
   attest"`, return. (This arm is the defense-in-depth guard against the C1 double-batch: the
   session-sourced load sees in-memory residue.)
4. Check the freshly-projected `state.blockers` for `prior_id`:
   - `SafeHarborUnconservable` present → status `"Allocation fails conservation — attestation
     cannot cure it; quit the editor, then re-run: btctax reconcile safe-harbor-allocate"`, return.
   - `SafeHarborTimebar` NOT present (allocation already-effective, no time-bar) → status
     `"Allocation already effective — no attestation needed"`, return.
   - `SafeHarborTimebar` present → open the flow.

**Flow state struct:**

```rust
pub struct SafeHarborAttestFlowState {
    pub prior_id: EventId,                    // the allocation being voided + re-attested
    pub prior_alloc: SafeHarborAllocation,    // the allocation details (for display)
    pub step: SafeHarborAttestStep,
}

pub enum SafeHarborAttestStep {
    Info,          // Step 1: shows allocation details + warnings
    TypedWord {
        buf: FieldBuffer,  // user must type "ATTEST" exactly
        error: Option<String>,
    },
}
```

**Step 1 — AllocationInfo display.** Show the prior allocation's details and a danger warning.
Keys at Info step:
| Key | Action |
|---|---|
| `Enter` | advance to TypedWord step |
| `Esc` | close flow → Browse (nothing written) |
| `q` | SWALLOWED (flow is blocking) |

Info display (draw_edit.rs):
```
╔═ Safe-Harbor Attestation — IRREVOCABLE ══════════════════╗
║  Allocation: {prior_id.canonical()}                      ║
║  As-of date: 2025-01-01  (§5.02(4) universal snapshot)  ║
║  Method:     {method:?}                                  ║
║  Pre-2025 method: {pre2025_method:?}                     ║
║  Lots:        {lots_count}  ({total_sat} sat)            ║
║  Attested:   false  ←  time-bar active (§5.02(4))        ║
║                                                          ║
║  STATUS: this allocation is inert due to the §5.02(4)    ║
║  time-bar. Attestation CURES the time-bar and makes the  ║
║  allocation EFFECTIVE and IRREVOCABLE (§7.4).            ║
║                                                          ║
║  !! IRREVOCABLE WARNING:                                  ║
║  Once attested, this allocation CANNOT be voided — any    ║
║  void attempt fires a PERMANENT Hard DecisionConflict     ║
║  that gates tax computation (§7.4): the doomed void is    ║
║  itself append-only and cannot be undone. Do NOT attest   ║
║  unless the lot list and method match your filed return.  ║
║                                                          ║
║  The operation voids the current allocation and re-       ║
║  appends it as attested (TWO decision events written).    ║
║                                                          ║
║  [Enter] Proceed to confirmation   [Esc] Cancel          ║
╚══════════════════════════════════════════════════════════╝
```

**Step 2 — TypedWord confirmation.** The user types `ATTEST` character by character into the
`buf: FieldBuffer`. Digit/alpha keys push to buf; Backspace pops. `Enter` submits.

```
╔═ IRREVOCABLE: type ATTEST to confirm ════════════════════╗
║                                                          ║
║  Type exactly:  ATTEST                                   ║
║  Your input:    {buf.as_str()}                           ║
║                                                          ║
║  This attestation is permanent. The allocation becomes   ║
║  immediately irrevocable upon save.                      ║
║                                                          ║
║  {error if any}                                          ║
║                                                          ║
║  [Enter] Submit (if "ATTEST" typed)  [Esc] Cancel        ║
╚══════════════════════════════════════════════════════════╝
```

TypedWord step keys:
| Key | Action |
|---|---|
| Printable char | push to `buf` (FIELD_CAP=64; no digit-only restriction) |
| `Backspace` | pop from `buf` |
| `Enter` | if `buf.as_str().trim() == "ATTEST"` → call `persist_safe_harbor_attest`; else → `error = Some("type ATTEST (all caps) to confirm")`, **buffer PRESERVED** (substrate FieldBuffer behavior — the user corrects with Backspace) [R0-I7] |
| `Esc` | back to Info step [I4 — one step back: TypedWord → Info → close flow] |
| `q` | SWALLOWED |

**No separate `safe_harbor_attest_modal`** — the TypedWord step IS the final gate; there is no
modal-layer entry for this flow (the modal chain ends at layer 8; the attest flow's steps,
including TypedWord, are handled entirely in the flow layer, layer 9 [R0-M4]). This deviates
from the standard "List → Form → modal" pattern deliberately: a second payload-modal after the
typed word would dilute the typed-word gate.

**Post-effect status:**

```
derive_attest_status(snap, &new_attest_id) → String
```

- **`SafeHarborUnconservable` attributed to `new_attest_id`** (conservation failed on the
  re-attested allocation — this should not happen if pre-flight passed, but defensively):
  → `"ATTEST FAILED: allocation fires SafeHarborUnconservable — see Compliance; the prior
     void and re-append both landed; quit the editor, then repair via CLI"`.
- **`SafeHarborTimebar` still present for `new_attest_id`** (unexpected — re-attested allocation
  still time-barred; should not occur):
  → `"ATTEST SAVED but SafeHarborTimebar re-fired — check Compliance; the allocation may not
     have cured the time-bar"`.
- **`DecisionConflict` on `new_attest_id`** (multiple live allocations after the batch write —
  edge case if pre-flight raced with a concurrent write):
  → `"ATTEST SAVED but DecisionConflict fired — check Compliance; vault integrity may be
     affected; quit and run: btctax verify"`.
- **Clean (no timebar, no unconservable, no conflict on new_attest_id):**
  → `"Allocation attested (IRREVOCABLE, §7.4) — {new_attest_id.canonical()};
     quit and run btctax verify to confirm effectiveness"`.

All four arms are keyed to the NEW allocation id ONLY. **Stale Advisory on the voided prior
[R0-M10]:** allocation-targeted voids take the resolve.rs:322-328 arm into `allocation_voids`
and never enter the `voided` set (resolve.rs:847), so post-attest the voided PRIOR is
re-evaluated every projection and keeps firing `SafeHarborTimebar` on ITS id. This is a
harmless engine fact (Advisory; Path B governs via the new allocation) — do NOT "fix" the
stale advisory, and never widen a status arm or KAT assertion to "no timebar anywhere".

**Enter-arm semantics (DIFFERS from other flows):** On `Ok((void_id, attest_id))` → re-project
+ derive status + **close the entire flow** (both Info and TypedWord steps). On `Err(e)` →
close the entire flow (no retry path — Hard Constraints; a retry would create the
unrecoverable double-batch), **set `app.attest_save_failed = true`** [R0-C1], status =
`"Save error: {e} — quit the editor now (the unsaved attestation is discarded on quit), then
run: btctax reconcile safe-harbor-attest"`.

---

### D4 — `edit/persist.rs` additions

Three new `pub fn` items in `edit/persist.rs` (the ONLY location permitted to name
`append_decision` or `donation_details::set`):

```rust
/// Append a `LotSelection` decision and atomically save the vault.
///
/// `payload` is the VALIDATED `EventPayload::LotSelection(…)`.
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # Duplicate ⇒ conflict, NEITHER applies (resolve.rs:787-800) [R0-I2]
/// A retry appends a duplicate `LotSelection` for the same `disposal_event`. The dup
/// fires a Hard `DecisionConflict` on ITS id and NEITHER selection applies (the disposal
/// falls back to METHOD ORDER) until one of the two is voided — voiding the duplicate
/// reinstates the first. Surfaced by the D1 status; cleared via the Void flow ('v') or,
/// after quitting the editor, CLI: `btctax reconcile void decision|<seq>`.
///
/// Does NOT write to `optimize_attestation` (only `optimize accept --attest` does that).
/// Clearing `optimize_attestation` on void is handled by `persist_void` (chunk 2b, D4).
pub fn persist_select_lots(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,   // must be EventPayload::LotSelection
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, btctax_cli::CliError> {
    let id = btctax_core::persistence::append_decision(
        session.conn(), payload, now, time::UtcOffset::UTC, None,
    )?;
    session.save()?;
    Ok(id)
}

/// Store `DonationDetails` for `event_id` in the `donation_details` side-table
/// and atomically save the vault (last-write-wins upsert; NOT a decision event).
///
/// Mirrors `tax_profile::set` discipline (chunk 1 D3). No `append_decision` call.
/// `is_review_complete` is NOT checked here — it is checked post-save for the status string.
pub fn persist_donation_details(
    session: &mut btctax_cli::Session,
    event_id: &btctax_core::EventId,
    details: &btctax_core::DonationDetails,
) -> Result<(), btctax_cli::CliError> {
    btctax_cli::donation_details::set(session.conn(), event_id, details)?;
    session.save()?;
    Ok(())
}

/// Void the existing live SafeHarborAllocation and re-append it as attested.
///
/// `prior_id` is the EventId of the live (non-voided, timebarred) allocation.
/// `prior_alloc` is the allocation payload (cloned from the pre-flight load).
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # Two-decision atomic batch (reconcile.rs:541-563)
/// 1. Appends `VoidDecisionEvent{target_event_id: prior_id}` (inlines the void).
/// 2. Appends `SafeHarborAllocation{..prior_alloc, timely_allocation_attested: true}`.
/// Both land in the same in-memory Connection; the single `session.save()` flushes both.
///
/// # Failed-save + retry (NO retry path — Hard Constraints [R0-C1])
/// On `Err(save)`: the vault is pre-action on-disk, but BOTH appends remain in the
/// in-memory Connection — any later `session.save()` would flush them as a side effect
/// (the piggy-back hazard). The caller MUST set `app.attest_save_failed = true` (the
/// residue latch: all mutating openers refuse until the editor quits). A retry would
/// duplicate the batch → two effective allocations → Hard DecisionConflict + Path A
/// (resolve.rs:958-967), both copies §7.4-unvoidable — unrecoverable [R0-M2]. The flow
/// closes on Err; the safe remediation is QUIT (discards the residue), then the CLI.
pub fn persist_safe_harbor_attest(
    session: &mut btctax_cli::Session,
    prior_id: btctax_core::EventId,
    prior_alloc: btctax_core::event::SafeHarborAllocation,
    now: time::OffsetDateTime,
) -> Result<(btctax_core::EventId, btctax_core::EventId), btctax_cli::CliError> {
    use btctax_core::{EventPayload, event::{SafeHarborAllocation, VoidDecisionEvent}};
    use btctax_core::persistence::append_decision;
    use time::UtcOffset;

    let void_id = append_decision(
        session.conn(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: prior_id }),
        now,
        UtcOffset::UTC,
        None,
    )?;
    let attested = SafeHarborAllocation {
        timely_allocation_attested: true,
        ..prior_alloc
    };
    let attest_id = append_decision(
        session.conn(),
        EventPayload::SafeHarborAllocation(attested),
        now,
        UtcOffset::UTC,
        None,
    )?;
    session.save()?;
    Ok((void_id, attest_id))
}
```

**KAT-G1 change (REQUIRED) [R0-I4]:** ADD `"donation_details::set"` to `persist_only_tokens`
(persist.rs:685). That list is the set of tokens FORBIDDEN outside `edit/persist.rs` non-test
code — `tax_profile::set` is guarded precisely BY BEING IN it; parity for the new side-table
writer requires the same entry. This mechanizes Task 4's claim that `donation_details::set` is
callable only from `edit/persist.rs`. `persist_select_lots` and `persist_safe_harbor_attest`
need no token change: `append_`, `conn(`, and `save(` already cover them.

---

### D5 — safety tests (KATs)

All tests are TDD-red first, then implementation, then green. The full validation suite must
pass at every step.

#### KAT-P2g — strict-prefix test for select-lots

(Re-lettered [R0-M3]: the name KAT-P2f is already taken at HEAD by
`kat_p2f_void_lot_selection_clears_optimize_attest_method_election_does_not`, persist.rs:1186.)
Same pattern as KAT-P2a/P2b/P2c (persist.rs, the established skeleton):

```
post.len() == pre.len() + 1
post[..pre.len()] == pre[..]
post[pre.len()].kind == "decision"
post[pre.len()].decision_seq
  == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
serde_json::from_str::<EventPayload>(&post[pre.len()].payload_json)
  == EventPayload::LotSelection(expected)
```

Seed: a genuine `TransferOut` + `ReclassifyOutflow(Donate)` + `Acquire` (so there's a Donation
removal with a lot to pick). The `LotSelection` payload references the TransferOut EventId as
`disposal_event` and at least one `LotPick` referencing the Acquire's lot. Both in-memory and
drop+reopen assertions. Payload round-trips correctly (including `LotId` format).

#### KAT-P2h — strict-prefix test for safe-harbor-attest (two-decision batch)

```
post.len() == pre.len() + 2                       // two decisions appended
post[..pre.len()] == pre[..]                      // prefix equality
post[pre.len()].kind == "decision"                // first new: VoidDecisionEvent
post[pre.len()].decision_seq
  == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
serde_json::from_str::<EventPayload>(&post[pre.len()].payload_json)
  == EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: prior_id })
post[pre.len() + 1].kind == "decision"            // second new: SafeHarborAllocation
post[pre.len() + 1].decision_seq == post[pre.len()].decision_seq.unwrap() + 1
serde_json::from_str::<EventPayload>(&post[pre.len() + 1].payload_json)
  == EventPayload::SafeHarborAllocation(expected_with_attested_true)
```

Seed: a `SafeHarborAllocation` with `timely_allocation_attested: false`. Call
`persist_safe_harbor_attest(session, prior_id, prior_alloc, now)`. Assert both in-memory and
drop+reopen. The `prior_alloc` struct-update pattern (`..prior_alloc, timely_allocation_attested:
true`) is verified by the round-trip: the new allocation JSON has `timely_allocation_attested:
true`; all other fields match the seed.

#### KAT-DD-PERSIST — side-table write test for set-donation-details

Not a strict-prefix test (no decision event). Pattern mirrors `persist_tax_profile`'s KAT:
- Create temp vault; seed a Donation event.
- Call `persist_donation_details(session, &event_id, &minimal_details())`.
- Assert: `donation_details::get(session.conn(), &event_id) == Some(minimal_details())`.
- Drop + reopen: assert same result on disk.
- Call again with `full_details()` (upsert): assert `donation_details::get == Some(full_details())`.
- Assert event log has NO new decision rows (strict: `post.len() == pre.len()`).

#### KAT-C2f — cancel-path bytes-unchanged (select-lots)

Pattern: chunk-1 KAT-C1 / chunk-2b KAT-C2c discipline.

Seed disposal (principal = **100000 sat**) + lot [R0-M5 — the modal opens only when
Σ picks == principal, so the typed pick must equal the seeded principal]. `bytes_before`.
Press `s` → flow opens at List; `Enter` → LotsForm; type
`"100000"` on first lot row; `Enter` → modal opens (assert `select_lots_modal.is_some()`);
`Esc` → modal closes (LotsForm still open, buffer intact); `Esc` → back to List; `Esc` → flow
closes. Assert `q` swallowed at each flow step. `bytes_after == bytes_before`. Complement:
confirmed path writes.

#### KAT-C2g — cancel-path bytes-unchanged (set-donation-details)

`d` → list; `Enter` → FieldForm; type into donee_name_buf and appraiser_name_buf; `Enter` →
modal; `Esc` → modal closes (form still open); `Esc` → back to List; `Esc` → flow closes.
`q` swallowed at each step. `bytes_after == bytes_before`. Complement: confirmed path writes.

#### KAT-C2h — cancel-path bytes-unchanged (safe-harbor-attest)

Seed a valid timebarred allocation. `a` → flow opens at Info step; `Enter` → TypedWord step;
type partial word `"ATT"` (not `"ATTEST"`); press `Enter` → error shown, TypedWord stays open;
press `Esc` → back to Info step; press `Esc` → flow closes. `q` swallowed at each step.
`bytes_after == bytes_before`. Complement: typing `"ATTEST"` then `Enter` writes.

#### KAT-S3a — save-error path for select-lots (`#[cfg(unix)]`)

**Justification for sampling:** KAT-S2 (classify-inbound) and KAT-S2b (set-fmv) already prove
the failed-save chmod pattern for the substrate's retry stories. Select-lots has the
duplicate-conflict retry story (NEITHER applies until one is voided [R0-I2]); this KAT pins it
plus the multi-pick assembly validation (a new input shape). Donation-details (last-write-wins
upsert, idempotent) needs no chmod test beyond its cancel-path KAT; safe-harbor-attest gets
its own dedicated chmod KAT (KAT-E2E-ATTEST-ERRLATCH [R0-C1]).

Steps (mirrors KAT-S2, including root-skip guard):
1. Seed disposal + lot; `pre = load_all_ordered(conn)`.
2. Navigate to LotsForm → enter valid pick → modal → chmod parent dir 0o500 → `Enter` →
   assert: modal closed, LotsForm still open with buffers intact, status "Save error", bytes
   unchanged.
3. Restore perms; re-submit → retry appends second LotSelection (decision N+2), save succeeds.
4. Assert: `post.len() == pre.len() + 2`; BOTH tails are LotSelection for the same disposal;
   re-projected state has `DecisionConflict` attributed to the RETRY decision's id, and NEITHER
   selection applies — the disposal falls back to METHOD ORDER until one is voided [R0-I2].
   Status surfaces the conflict (voiding the duplicate reinstates the FIRST selection's picks;
   voiding the first instead keeps re-edited picks — the conflict clears either way).

#### KAT-E2E-SL — end-to-end select-lots (full round-trip, discriminating seed [R0-M6])

1. Seed TWO lots with distinct acquisition dates: Acquire **A** (1_000_000 sat, the EARLIER
   date — FIFO's pick) + Acquire **B** (1_000_000 sat, later date), same wallet; then
   TransferOut(500_000 sat) + ReclassifyOutflow(sell, proceeds=$30k). Confirm the TransferOut
   is a Disposal in projected state and that method order (FIFO) consumes lot A. Assert the
   disposal appears in the `s` list (not already-selected).
2. Drive `s` → list shows the disposal; `Enter` → LotsForm shows BOTH lots; type `"500000"`
   on lot **B** (the NON-FIFO pick — this is what makes the test discriminating: a silently
   dropped selection would fall back to consuming A); `Enter` → modal (assert shows
   `"disposal: ... (sell)"`, `"1 lot(s), 500000 sat"`); `Enter` → save + re-project.
3. Re-project: assert no `LotSelectionInvalid` blocker for the disposal; assert the disposal's
   legs consume lot **B** (`DisposalLeg.lot_id` == B's LotId) — the selection demonstrably
   overrode method order.
4. Assert the disposal NO LONGER appears in the `s` list (already-selected pre-filter).

#### KAT-E2E-SL-DONATE — select-lots through a Donate removal [R0-I1]

Seeds the removal FORM path (no existing KAT drives it: KAT-P2g seeds a Donation but calls the
persist fn directly; KAT-E2E-SL uses a sell):
1. Seed: Acquire (wallet W) + TransferOut + ReclassifyOutflow(Donate) → a Donation removal in
   projected state.
2. Drive `s` → the donate removal appears (Kind column `"donate"`; Wallet column = W, sourced
   from the raw `LedgerEvent.wallet` via `events_by_id` — `RemovalLeg` has no wallet field, so
   the raw-event source is load-bearing); `Enter` → LotsForm lists wallet-W lots; pick the
   full principal; `Enter` → modal → `Enter` → save + re-project.
3. Re-project: no `LotSelectionInvalid`; the removal's legs consume the chosen lot.

#### KAT-E2E-SL-VOID — select-lots + void round-trip

After KAT-E2E-SL:
1. Drive `v` → void list shows the LotSelection decision; select + confirm. Re-project: no
   `LotSelectionInvalid` for the disposal; the disposal re-appears in the `s` list.
2. Confirm that `optimize_attestation` was cleared for the disposal (call
   `btctax_cli::optimize_attest::get(session.conn(), &disposal_event)` → None).
   [This tests the chunk-2b persist_void side-effect, pinned here for the select-lots case.]

#### KAT-E2E-DD — end-to-end set-donation-details (completeness progression)

1. Seed a Donation removal (Acquire + TransferOut + ReclassifyOutflow(Donate)).
2. Drive `d` → list shows the donation; `Enter` → FieldForm (pre-populated: all fields empty,
   no existing details); fill only donee_name and appraiser_name; `Enter` → modal; `Enter` → save.
3. Status: `"Details saved … Section A complete on presence; add …for Section B completeness"`.
4. Drive `d` again → list shows the donation with `"present"` in the Completeness column;
   `Enter` → FieldForm PRE-POPULATED with the saved donee_name and appraiser_name; add
   `appraiser_tin`, `appraisal_date`, `appraiser_qualifications`, `donee_ein`; `Enter` → save.
   (Step 4 works BECAUSE the `Ok` arm rebuilds the snapshot, refreshing
   `snap.donation_details` — the list/pre-population read source [R0-I3].)
5. Status: `"Details saved … Section B complete"`. List now shows `"B-complete"` for the event.

#### KAT-E2E-ATTEST-PREFLIGHT — safe-harbor-attest pre-flight cases

Drive `a` with vaults covering each pre-flight failure arm:
1. No allocation → status "No allocation to attest". Flow not opened.
2. Allocation already-attested (`timely_allocation_attested: true`) → status "Allocation already
   attested". Flow not opened. [To seed: append an already-attested allocation directly.]
3. Allocation `SafeHarborUnconservable` → status "Allocation fails conservation". Flow not opened.
4. Allocation already-effective (no SafeHarborTimebar blocker) → status "Allocation already
   effective". Flow not opened.
5. Allocation with SafeHarborTimebar → flow DOES open at Info step. (Positive control for the E2E below.)

#### KAT-E2E-ATTEST — end-to-end safe-harbor-attest (typed-word round-trip)

1. Seed: append a `SafeHarborAllocation` with `timely_allocation_attested: false` that will
   produce a `SafeHarborTimebar` blocker (requires a properly seeded pre-2025 vault with the
   §5.02(4) deadline condition; use a synthetic timestamp that satisfies the timebar). Confirm
   `SafeHarborTimebar` is present in projected blockers.
2. Drive `a` → pre-flight passes; Info step displayed; assert it mentions `"IRREVOCABLE"` and
   prior allocation's canonical id.
3. Press `Enter` → TypedWord step; type `"ATTES"` (incomplete) → `Enter` → error shown
   (`"type ATTEST"`); TypedWord still open, **buffer preserved** [R0-I7].
4. Type `"T"` (completing `"ATTEST"` in the preserved buffer) → `Enter` → save + re-project.
5. Assert: `post.len() == pre.len() + 2` (Void + new SafeHarborAllocation appended).
6. Re-project: NO `SafeHarborTimebar` attributed to the NEW allocation id;
   `timely_allocation_attested` is `true` in the new allocation row. Do NOT assert "no
   timebar anywhere" [R0-M10]: the voided PRIOR keeps firing a stale Advisory
   `SafeHarborTimebar` on ITS id every projection (allocation-targeted voids never enter the
   engine's `voided` set — resolve.rs:322-328 vs 847) — assert keyed to the new id only.
7. In the TUI void list's raw voided-set scan, the prior IS voided → it no longer appears in
   the `v` list; the NEW attested allocation DOES appear (see KAT-E2E-ATTEST-VOID).
8. Status: `"Allocation attested (IRREVOCABLE, §7.4) — {new_attest_id.canonical()}"`.

#### KAT-E2E-ATTEST-WRONGWORD — typed-word case-sensitivity [R0-I7]

At the TypedWord step, type `"attest"` (lowercase) → `Enter` → error: `"type ATTEST (all caps)
to confirm"`; the buffer is PRESERVED (D3 key table). Press `Backspace` ×6 (clearing
`"attest"`), then type `"ATTEST"` → `Enter` → submits. Pinned: case-sensitivity AND the
preserved-buffer semantics are both load-bearing (this script and KAT-E2E-ATTEST steps 3-4
share the same buffer model).

#### KAT-E2E-ATTEST-VOID — post-attest void is REJECTED, permanently [R0-I6]

The chunk-3 flow creates the first TUI-reachable EFFECTIVE allocation, and the shipped 2b void
flow offers it (`is_revocable_payload` includes `SafeHarborAllocation`, form.rs:822-836; the
list filter is raw-decision + non-voided, main.rs:2353-2374). Pin the §7.4 interaction:
1. After KAT-E2E-ATTEST: drive `v` → the NEW attested allocation IS listed, with the SHA
   warning (draw_edit.rs:1415-1420).
2. Select + confirm the void. Status is the 2b arm-1 wording `"Void saved, but DecisionConflict
   fired — the target decision remains in force"` (main.rs:2410-2417) — NOT `"Voided…"`.
3. Re-project: the allocation is STILL effective; the doomed void's `DecisionConflict`
   (resolve.rs:924-933) is present — and PERMANENT: the void event is append-only and is
   itself non-revocable (resolve.rs:312-321), so the Hard conflict re-fires every projection
   (`TaxYearNotComputable` persists). This is exactly what the D3 Info warning tells the user
   to weigh BEFORE typing ATTEST.

#### KAT-E2E-ATTEST-ERRLATCH — attest save-Err residue latch (`#[cfg(unix)]`) [R0-C1]

Mirrors the KAT-S2 chmod pattern (incl. the root-skip guard):
1. Seed a valid timebarred allocation; `pre = load_all_ordered(conn)`; `bytes_before`.
2. Drive `a` → Info → `Enter` → TypedWord → type `"ATTEST"` → chmod parent dir 0o500 →
   `Enter` → assert: flow FULLY closed (no keep-open retry), status is the quit-first remedy
   (`"Save error: … quit the editor now (the unsaved attestation is discarded on quit) …"`),
   `app.attest_save_failed == true`, on-disk bytes unchanged.
3. Press `a` again → opener refuses with the latch status; nothing appended. Press `f` (an
   unrelated mutating opener) → same refusal — **this is the piggy-back guard**: with every
   mutating opener latched shut, no later `session.save()` can flush the in-memory Void+Attest
   residue.
4. Restore perms; assert `bytes_after == bytes_before` still (no write occurred).
5. Defense-in-depth [R0-I5]: call the pre-flight fn directly (bypassing the latch) — the
   session-sourced `load_events_and_project` sees the in-memory already-attested allocation
   and refuses via the "already attested" arm; log length unchanged.

#### KAT-V-SL-1..3 — select-lots validation

- **KAT-V-SL-1:** all picks zero → error `"pick at least one lot"`.
- **KAT-V-SL-2:** Σ picked_sat < principal_sat → error `"picked {Σ} sat != disposal principal {principal} sat"`.
- **KAT-V-SL-3:** Σ picked_sat == principal_sat → valid; builds correct LotPick list with non-zero picks only.

#### KAT-V-DD-1..3 — set-donation-details validation

- **KAT-V-DD-1:** donee_name empty → error `"donee-name is required"`.
- **KAT-V-DD-2:** appraiser_name empty → error `"appraiser-name is required"`.
- **KAT-V-DD-3:** appraisal_date non-empty with bad format → parse error from `parse_date_arg`.

#### KAT-V-DD-4 — pre-population round-trip

Open the set-donation-details FieldForm for an event that already has stored details (read
from `snap.donation_details` [R0-I3]). Assert all 10 FieldBuffers are pre-populated with the
stored values (mapping `Option<String>` into the buffer via `FieldBuffer::set`, form.rs:47
[R0-N1]). This is the canonical test that the "re-edit pre-populates" contract works.

#### KAT-G1 (inherited — must stay green throughout)

- `persist_select_lots` and `persist_safe_harbor_attest` name `append_decision` — in `edit/persist.rs`
  only; `append_` in `persist_only_tokens` covers both.
- `persist_donation_details` names `donation_details::set` — in `edit/persist.rs` only. No
  `append_decision` call. `"donation_details::set"` is ADDED to `persist_only_tokens`
  (persist.rs:685) so the guard mechanically forbids the new writer outside `edit/persist.rs`
  [R0-I4].
- No forbidden tokens appear in any non-test region of `main.rs`, `editor.rs`, `form.rs`, or
  `draw_edit.rs` (the D2 reads use `snap.donation_details`; the D3 pre-flight uses
  `session.load_events_and_project()`, which carries no forbidden token).

---

## Plan (TDD)

### Task 1 — select-lots flow

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `DisposalListItem`, `DisposalKind`,
  `LotPickFormRow`, `SelectLotsStep`, `SelectLotsFlowState`, `SelectLotsModalState`,
  `validate_select_lots`.
- `crates/btctax-tui-edit/src/editor.rs` — add `select_lots_flow: Option<SelectLotsFlowState>`,
  `select_lots_modal: Option<SelectLotsModalState>`.
- `crates/btctax-tui-edit/src/main.rs` — `s` key dispatch; modal layer 7;
  `handle_select_lots_flow_key` (list + lots-form steps), `handle_select_lots_modal_key`;
  `derive_select_lots_status`; `open_select_lots_flow` (pre-filter in Claim F; per-item wallet
  from the raw event via `events_by_id` [R0-I1]).
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_select_lots_list`, `draw_lots_form`,
  `draw_select_lots_modal`.
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_select_lots`.

**KATs:** KAT-P2g [R0-M3], KAT-C2f, KAT-S3a, KAT-E2E-SL, KAT-E2E-SL-DONATE [R0-I1],
KAT-E2E-SL-VOID, KAT-V-SL-1..3. TDD-red before implementation; green after.

### Task 2 — set-donation-details flow

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `DonationListItem`, `SetDonationDetailsStep`,
  `SetDonationDetailsFlowState`, `SetDonationDetailsModalState`, `validate_donation_details`.
- `crates/btctax-tui-edit/src/editor.rs` — add `set_donation_details_flow: Option<…>`,
  `set_donation_details_modal: Option<…>`.
- `crates/btctax-tui-edit/src/main.rs` — `d` key dispatch; modal layer 8;
  `handle_set_donation_details_flow_key`, `handle_set_donation_details_modal_key`;
  `derive_donation_details_status`; `open_set_donation_details_flow` (pre-filter in Claim G;
  per-item details from `snap.donation_details` [R0-I3]).
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_donation_details_list`,
  `draw_donation_details_form`, `draw_donation_details_modal`.
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_donation_details`; ADD
  `"donation_details::set"` to `persist_only_tokens` (KAT-G1 [R0-I4]).

**KATs:** KAT-DD-PERSIST, KAT-C2g, KAT-E2E-DD, KAT-V-DD-1..3, KAT-V-DD-4. TDD-red before;
green after.

### Task 3 — safe-harbor-attest flow

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `SafeHarborAttestFlowState`,
  `SafeHarborAttestStep`.
- `crates/btctax-tui-edit/src/editor.rs` — add `safe_harbor_attest_flow: Option<SafeHarborAttestFlowState>`
  (no separate modal field — the TypedWord step is inside the flow [R0-M4]) and the
  `attest_save_failed: bool` latch [R0-C1].
- `crates/btctax-tui-edit/src/main.rs` — `a` key dispatch; `handle_safe_harbor_attest_flow_key`
  (Info + TypedWord steps); `derive_attest_status`; `open_safe_harbor_attest_flow` (latch check
  + one-source pre-flight in Claim H [R0-I5]); latch checks in ALL mutating openers
  (`p/c/o/r/f/v/s/d/a`) [R0-C1].
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_attest_info`, `draw_attest_typed_word`.
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_safe_harbor_attest`.

**KATs:** KAT-P2h [R0-M3], KAT-C2h, KAT-E2E-ATTEST-PREFLIGHT, KAT-E2E-ATTEST,
KAT-E2E-ATTEST-WRONGWORD, KAT-E2E-ATTEST-VOID [R0-I6], KAT-E2E-ATTEST-ERRLATCH [R0-C1].
TDD-red before; green after.

### Task 4 — whole-diff review (Phase E) + FOLLOWUPS

Cross-cutting checks:

- **Editor guarantee unchanged:** `append_decision` and `conn(`/`save(` appear only in
  `edit/persist.rs` non-test code; `donation_details::set` likewise only in `edit/persist.rs`.
  KAT-G1 green.
- **Modal gating:** `persist_select_lots` sole non-test call site = select-lots modal Enter;
  `persist_donation_details` = donation-details modal Enter; `persist_safe_harbor_attest` =
  attest TypedWord Enter. Verified by grep + KAT-G1.
- **Dispatch order:** 11-layer dispatch [R0-M4]; layers 7 and 8 are the two new modals; layer 9
  the extended flow layer covering all 8 flows (incl. the attest TypedWord step); `q` swallowed
  everywhere; Esc steps back one step at every flow step [I4].
- **Typed-word irrevocability:** `"ATTEST" != "attest"` — case-sensitivity + preserved-buffer
  semantics [R0-I7]; KAT-E2E-ATTEST-WRONGWORD pins both.
- **Attest close-on-Err + residue latch [R0-C1]:** flow closes entirely on `Err(e)` from
  `persist_safe_harbor_attest`; the Err arm sets `attest_save_failed` (and is the ONLY setter);
  every mutating opener checks the latch first; the status names the quit-first CLI remedy;
  KAT-E2E-ATTEST-ERRLATCH pins the latch, the reopen refusal, and the piggy-back guard.
- **Donation-details post-save rebuild [R0-I3]:** the `Ok` arm rebuilds the snapshot (uniform
  with all flows); status derives from the in-hand `details`; no `conn(`-bearing reads outside
  `edit/persist.rs`.
- **CLI-pointing statuses:** every status naming a CLI command says to quit the editor first —
  the Session holds the exclusive VaultLock for the editor's lifetime (editor.rs:8-14)
  [R0-C1 lock audit].
- **Select-lots principal-conservation:** `Σ pick_sat == disposal_principal_sat` validated in
  the form before modal; engine additionally validates lot existence + per-wallet + over-draw.
- **Pre-filter correctness:**
  - select-lots: excludes fee_mini_disposition (flag filter [R0-M7]) and already-selected; BOTH
    Gift and Donation removals included, with wallet from the raw event [R0-I1]; SelfTransfer
    excluded (known under-inclusion, FOLLOWUPS).
  - donation-details: Donation removals only (Gift excluded, per reconcile.rs:619); details
    from `snap.donation_details` [R0-I3].
  - attest: latch first [R0-C1]; pre-flight covers all four failure arms from ONE
    `load_events_and_project` [R0-I5].
- **`now` injection:** all three persist fns receive `now` from the confirmation Enter-press.
- **Retry semantics pinned:**
  - select-lots: duplicate ⇒ conflict on the dup's id, NEITHER applies (method-order fallback)
    until one is voided [R0-I2]; KAT-S3a pins.
  - donation-details: last-write-wins, no conflict; KAT-DD-PERSIST pins.
  - attest: close-on-Err + latch, no TUI retry; KAT-E2E-ATTEST-ERRLATCH pins.
- **Viewer untouched:** no viewer files change; E10 gate continues to pass.

FOLLOWUPS to record for chunk 3:
- **SelfTransfer lot-selection under-inclusion:** linked TransferOut events that become
  `Op::SelfTransfer` are method-honoring (honoring_principal returns Some) but are absent
  from the TUI select-lots list (not in `state.disposals` or `state.removals`). Under-inclusion
  only; CLI remains available. Fix = scan `snap.events` for TransferOut events with a non-voided
  TransferLink decision (the SelfTransfer case) and include in the disposal list.
- **Lot-display at disposal date:** the TUI shows currently-projected lots, not lots available
  AT the disposal date. The engine validates accurately; the display is a best-effort guide.
- **Safe-harbor-allocate TUI flow:** `reconcile safe-harbor-allocate` is the CREATION side of
  the allocation; it is out of scope for chunk 3 (noted in 2b Out of Scope). The attest-only
  chunk-3 coverage means the user must use the CLI to create the allocation, then the TUI to
  attest it.
- **WB-I4(a) carryforward:** raw-vs-effective under-inclusion (2b FOLLOWUP) does not affect
  chunk 3 (select-lots uses disposals/removals which are already projected; donation-details
  targets removals by RemovalKind; attest targets SafeHarborAllocation by voided-set scan).
- **FIELD_CAP=64 CLI-parity limit [R0-M9]:** the free-text donation fields (addresses,
  `appraiser_qualifications`) truncate at 64 chars in the TUI (form.rs:17, 35-38); the CLI
  accepts arbitrary length. Recorded as a parity limit; candidate fix = a larger cap for
  designated free-text fields.
- **Void-list pre-filter for effective allocations [R0-I6 optional]:** the 2b void flow still
  LISTS an effective (attested) allocation, and a confirmed void is a permanently-damaging
  no-op (§7.4 doomed-void conflict; KAT-E2E-ATTEST-VOID pins today's behavior). Effectiveness
  is derivable from blockers — pre-filter effective allocations out of the void list in a
  later chunk so the trap is unreachable.
- **In-memory residue after failed saves (2a/2b flows):** the C1 piggy-back mechanics exist
  for the single benign appends of the shipped flows too (keep-form-open retry). Benign there
  (re-confirm is the intended remedy; the payloads are revocable), but consider generalizing
  the `attest_save_failed` latch into a session-dirty latch for all failed saves.

---

## Out of scope

- **Chunk 4 — import-level decisions:** `link-transfer`, `classify-raw`, `accept-conflict`,
  `reject-conflict`, `optimize-accept`.
- **Chunk 5 — safe-harbor-allocate:** the creation of the SafeHarborAllocation decision (the
  attest-only flow above covers the cure path; creation requires pre-2025 residue computation
  and is more complex).
- **`import-selections` (batch CSV):** the CLI batch import of LotSelections from a CSV file
  (reconcile.rs:354-432). Out of scope; the per-disposal TUI select-lots covers the interactive
  path.
- **Viewer changes:** frozen (E10 gate, write-free guarantee).
- **`btctax-core` or `btctax-cli` changes:** no new core types, no new CLI commands.
- **`optimize-accept` / `optimize-run` / `optimize-consult`:** optimizer flows entirely deferred.
