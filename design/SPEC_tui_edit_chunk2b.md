# SPEC — btctax-tui-edit chunk 2b: reclassify-income + set-fmv + void

**Source baseline:** `main` @ `fe726ff` (working tree re-verified file-by-file at write time; all
line citations checked against the current source).
**Review status:** R0 round 1 (`reviews/R0-spec-tui-edit-chunk2b-round-1.md`) FOLDED — findings
tagged `[I1]…[I4]` / `[M1]…[M5]` / `[N1]…[N4]` inline; re-review required per §2 of
`STANDARD_WORKFLOW.md`.
**Goal:** Chunk 2b of the mutating-TUI program — three new decision flows added to the existing
`btctax-tui-edit` crate:

1. **reclassify-income** — flips `business` (required-explicit) and optionally `kind` on an
   already-imported `Income` event. Appends
   `EventPayload::ReclassifyIncome{income_event, business, kind}`.
2. **set-fmv** — supplies a manual FMV for a `FmvMissing`-blocked `Income` event. Appends
   `EventPayload::ManualFmv{event, usd_fmv}`.
3. **void** — revokes a revocable decision. Appends
   `EventPayload::VoidDecisionEvent{target_event_id}`. **Closes the in-editor remedy loop:**
   the CLI void path named in chunk 2a's status strings becomes a first-class GUI flow.

All three flows reuse the chunk-2a substrate verbatim: the `TargetList<T>` widget, the
modal → flow → form → screen dispatch order with `q`-swallow and Esc-steps-back, the
`edit/persist.rs` persist pattern with `now`-injection, the blocker-derived status discipline
(`derive_*_status` fns), the strict-prefix KATs, per-flow cancel-bytes + save-failure KATs,
and the 2a retry-story discipline — instantiated per flow: FIRST-WINS + conflict for
reclassify-income, LATEST-WINS (no conflict) for set-fmv, idempotent re-void for void
(Hard constraints below state each precisely [M1]).

**SemVer:** Three new `pub fn` items in `edit/persist.rs`; three new flow/modal struct families
in `edit/form.rs`; key bindings `r` / `f` / `v` in `main.rs`. No new workspace member, no new
`[lib]` targets, no `btctax-core` or `btctax-cli` changes. **MINOR** (pre-1.0; additive).

---

## Hard constraints (chunk-1 + chunk-2a guarantees inherited verbatim)

The editor's crate-level guarantee is **unchanged by this spec**:

> "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`, each behind
> an explicit payload-showing confirmation; the vault file only via `Vault::save`'s atomic path."

Chunk-2b instantiation:

- `persist_reclassify_income`, `persist_set_fmv`, and `persist_void` are the ONLY three new
  writers; all live exclusively in `edit/persist.rs` (the sole allowlisted module in KAT-G1).
- Each calls `append_decision(conn, payload, now, UtcOffset::UTC, None)` then `session.save()`,
  mirroring `cmd::reconcile::append_and_save` (reconcile.rs). `now: OffsetDateTime` is INJECTED
  at Enter-press for test determinism.
- `persist_void` has an additional side-effect when the voided decision is a `LotSelection`:
  it calls `btctax_cli::optimize_attest::clear(session.conn(), &disposal_event)` BEFORE the
  `save()`, atomically (same in-memory Connection, single `session.save()`) — mirroring the CLI
  `void` command (reconcile.rs:117–147). Non-`LotSelection` decisions are unaffected (no row to
  clear; the delete is a no-op).
- `"append_"` is already in the KAT-G1 `persist_only_tokens` allowlist (persist.rs:577); no
  gate change required.
- Per-mutation confirmation: the modal precedes ALL writes; Esc cancels with nothing written.

**Failed-save + retry semantics (chunk-2a [R0-I1] rule, reapplied for the two append flows):**

- **`persist_reclassify_income`** (FIRST-WINS, resolve.rs:662–676): a retry appends a duplicate
  `ReclassifyIncome` for the same income event → Hard `DecisionConflict` on the retry decision's
  id; the FIRST (failed-save) decision stays in force. The post-persist status surfaces this (D5
  step-2 check). Remedy: void the duplicate (the new in-editor void flow; CLI fallback).
- **`persist_set_fmv`** (LATEST-WINS, resolve.rs:453–456): a retry appends a second `ManualFmv`
  for the same event → NO conflict (latest-seq-wins is the explicit design: "a valid re-pointing
  of an FMV is a correction flow, not a conflict"). The on-disk log grows by 2 rows; the second
  `ManualFmv` governs; no Hard blocker. Status is clean-success after the retry.
- **`persist_void`** [M1 — the void retry story differs from 2a's FIRST-WINS, stated precisely]:
  a retry appends a second `VoidDecisionEvent` targeting the SAME original (revocable) decision.
  This is NOT a void-of-void (the target is the original decision, not the first
  VoidDecisionEvent), so the non-revocable arm at resolve.rs:312–321 does not fire. The original
  decision's id is inserted into the `voided` BTreeSet twice — `BTreeSet::insert` is idempotent
  (resolve.rs:330) — so NO conflict fires and no behavior changes beyond the first void. On-disk
  log grows by 2 `VoidDecisionEvent` rows; the second is inert. Status after retry:
  clean-success (original decision still voided, its effects still un-projected). Pinned by the
  KAT-VOID-RETRY unit test (D5 — [M1]).
  **Save-error UI state for void [M1]:** D1/D2's Err-arm ("keep FieldForm open") cannot apply —
  void has no FieldForm. On `Err(e)`: close `void_modal`, the flow REMAINS OPEN at its `List`
  step (list intact), `status = "Save error: {e}"`. Retry = re-select the same row → `Enter` →
  modal → `Enter`. No re-projection on `Err` (vault unchanged on disk; the in-memory session
  carries the committed void — the idempotent-retry contract above covers the re-confirm).

**Dispatch order invariant (chunk-2a [R0-I2] lesson — six MODAL layers in a nine-layer dispatch)
[N4]:**

```
1. mutation_modal         (chunk 1)
2. classify_inbound_modal (chunk 2a)
3. reclassify_outflow_modal (chunk 2a)
4. reclassify_income_modal  (chunk 2b)
5. set_fmv_modal             (chunk 2b)
6. void_modal                (chunk 2b)
7. Flow layer — any open flow claims ALL keys; q swallowed, Esc steps back (chunk 2a)
8. Form layer — profile_form (chunk 1)
9. Screen dispatch
```

At most one flow `Some` and at most one modal `Some` at any time. The flow Option (not the step)
is the guard for the flow layer — every step of an open flow is claimed, so `q` and `Esc` can
never fall through to the Browse quit arm mid-flow.

---

## Pre-filter verification (re-verified at write time)

### Claim C — reclassify-income list: raw Income events with no non-voided ReclassifyIncome

**Basis: resolve.rs lines 635–694 (pass-1e, ReclassifyIncome collection).**

**Verdict: CONFIRMED.** Resolve collects non-voided `ReclassifyIncome` decisions into
`income_reclassify: BTreeMap<EventId, ReclassifyIncome>` keyed by `income_event` (resolve.rs:484).
If a second non-voided decision targets the same `income_event`, resolve pushes a Hard
`DecisionConflict` blocker and EXCLUDES the second decision (resolve.rs:665–673 — FIRST-WINS,
ascending seq iteration means the first decision wins). Therefore, a second non-voided
`ReclassifyIncome` for the same `income_event` fires a Hard `DecisionConflict` that gates
`compute_tax_year`.

**Required pre-filter for the reclassify-income list:**

```rust
// [N3] Both sets are HOISTED once before the per-event filter (the 2a precedent:
// `open_classify_inbound_flow`, main.rs:1194–1220) — never rebuilt inside the closure.

// Voided-decision set (VoidDecisionEvent targets).
let voided: BTreeSet<&EventId> = snap.events.iter()
    .filter_map(|ev| {
        if let EventPayload::VoidDecisionEvent(v) = &ev.payload {
            Some(&v.target_event_id)
        } else { None }
    })
    .collect();

// Income EventIds already targeted by a non-voided ReclassifyIncome
// (a second would fire DecisionConflict; FIRST-WINS).
let already_reclassified: BTreeSet<&EventId> = snap.events.iter()
    .filter(|ev| !voided.contains(&ev.id))
    .filter_map(|ev| {
        if let EventPayload::ReclassifyIncome(ri) = &ev.payload {
            Some(&ri.income_event)
        } else { None }
    })
    .collect();

// 1. Filter snap.events for raw Income payloads.
//    Under-inclusion note: ClassifyRaw'd Unclassified events whose effective payload became
//    Income are excluded (raw payload != Income). Same WB-I4(a) limitation as 2a; deferred.
snap.events
    .iter()
    .filter(|e| matches!(e.payload, EventPayload::Income(_)))
    // 2. Exclude if a non-voided ReclassifyIncome already targets this event.
    .filter(|e| !already_reclassified.contains(&e.id))
```

**Known under-inclusion [WB-I4(a) deferred]:** raw-payload filter misses Unclassified events
whose effective payload became Income via a non-voided `ClassifyRaw`. Under-inclusion only
(safe direction); interim path is CLI. Recorded in FOLLOWUPS.

**Display data:** the list item derives date/sat/kind/business from the Income payload directly
(before any ReclassifyIncome override, since the filter excludes already-reclassified events),
enriched with the income_recognized entry for fmv (if present: the event has a clean FMV). For
`FmvMissing` events (no income_recognized entry): fmv displayed as `"(pending)"`.

---

### Claim D — set-fmv list: FmvMissing-blocked Income events; latest-wins (no conflict)

**Basis: resolve.rs lines 423–474 (pass-1d, ManualFmv collection).**

**Verdict: CONFIRMED.** ManualFmv explicitly uses latest-seq-wins with NO duplicate blocker
(resolve.rs:427–428 [N1]: "Note: ManualFmv deliberately keeps latest-seq-wins with NO duplicate
blocker — a valid re-pointing of an FMV is a correction flow, not a conflict"). The target validation at
pass 1d checks the effective payload: `None` (target absent) → Hard `DecisionConflict`, decision
EXCLUDED (resolve.rs:443–450); `Income` → valid, inserted or overwritten (resolve.rs:453–456);
anything else (including `TransferIn`, even if classified as Income via `ClassifyInbound`) → Hard
`DecisionConflict`, decision EXCLUDED (resolve.rs:458–470 — detail: "ManualFmv targets non-Income
event … for a TransferIn classified as income, set the FMV via classify-inbound-income (its own
`fmv` field)").

**Required filter for the set-fmv list:**

```rust
snap.state.blockers
    .iter()
    .filter(|b| b.kind == BlockerKind::FmvMissing)
    .filter(|b| {
        // ManualFmv pass-1d validates EFFECTIVE payload == Income.
        // Filter on RAW EventPayload::Income to approximate this — same WB-I4(a) limitation.
        // (TransferIn classified as Income via ClassifyInbound is excluded — correct, because
        // ManualFmv on a TransferIn fires DecisionConflict; the remedy is void+re-classify.)
        b.event.as_ref().map_or(false, |id| {
            snap.events.iter().any(|e| &e.id == id
                && matches!(e.payload, EventPayload::Income(_)))
        })
    })
```

**No pre-filter for already-set FMVs:** the list naturally empties when the `FmvMissing` blocker
clears after a successful `persist_set_fmv` + re-projection. A second `ManualFmv` is NOT a
conflict (latest-wins, resolve.rs:453–456), so showing events with an existing ManualFmv is safe
— the user can re-point. If a ManualFmv is present but FmvMissing has not cleared yet (e.g.
income-without-wallet blocker instead), the event still appears; the next set-fmv is a re-point.

**FmvMissing fire paths (fold.rs):**
- `Op::Income` with `fmv=None` (fold.rs:672–677): income without FMV → blocker event = Income
  event id. This is the actionable path for set-fmv.
- `Op::IncomeInbound` with `fmv=None` (fold.rs:854–859): classified TransferIn without FMV →
  blocker event = TransferIn id. Excluded by the raw `EventPayload::Income` filter above (the
  TransferIn raw payload is NOT Income). The remedy for this case is supply fmv via
  `classify-inbound-income`'s own `fmv` field after voiding the old ClassifyInbound.
- `Op::Income` / `Op::IncomeInbound` without wallet (fold.rs:651–656, 833–839): also fires
  FmvMissing. The wallet-less `IncomeInbound` case is excluded by the raw-payload filter (its raw
  payload is TransferIn). The wallet-less `Income` case PASSES the filter (its raw payload IS
  Income) and appears in the set-fmv list. Verified behavior [M4 — rewritten to state only what
  the source shows]: the `ManualFmv` decision on that target is VALID at pass-1d (the effective
  payload is Income → inserted into `manual_fmv`, resolve.rs:453–456 — NO DecisionConflict). But
  in the fold, the wallet check at fold.rs:648–657 runs BEFORE the FMV match and `return`s early,
  so `FmvMissing` re-fires from the wallet arm ("income without wallet") on the next projection
  regardless of the supplied FMV. The set-fmv is therefore recorded but ineffective until the
  wallet is fixed. The `derive_set_fmv_status` FmvMissing-still-present arm (D2) surfaces this
  honestly — the status includes the blocker's `detail` string, which reads "income without
  wallet" in this case. Documented, not a critical design flaw — the typical income event HAS a
  wallet.

---

### Claim E — void list: revocable payload variants, not already voided

**Basis: resolve.rs lines 300–340 (pass-1a, VoidDecisionEvent handling).**

**Revocable set (VERIFIED from resolve.rs lines 301–340):**

The resolve.rs comment at line 301–305 states:
> "Revocable targets: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw,
>  MethodElection, LotSelection, ReclassifyIncome.
>  NON-revocable targets: SupersedeImport, RejectImport, VoidDecisionEvent.
>  Void of a non-revocable target → DecisionConflict (target stays in force; void is inert).
>  Void of SafeHarborAllocation → collected in allocation_voids (deferred to Task 12)."

Specifically:
- **Immediately revocable** (added to `voided: BTreeSet<EventId>` at line 330 [N1], `Some(_)`
  arm): TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw, MethodElection,
  LotSelection, ReclassifyIncome. A void of any of these takes effect immediately.
- **Conditionally revocable** (resolve.rs lines 322–328 [N1], `SafeHarborAllocation` arm): the
  void is collected in `allocation_voids` and adjudicated at step 3 (resolve.rs:924–934, conflict
  attributed to `v.void_id` at line 930) — if the allocation is EFFECTIVE (conservation + deadline
  checks pass), the void fires a Hard `DecisionConflict` (irrevocable §7.4) and the allocation
  stays in force; if the allocation is INERT, the void applies (Path A).
  SafeHarborAllocation IS included in the void list (its void is not immediately rejected), but
  the modal MUST display a consequence note: "If this allocation is effective (Path B), voiding it
  fires DecisionConflict — it remains irrevocable (§7.4); if inert, the void un-projects it."
- **Non-revocable** (resolve.rs lines 312–321, immediate `DecisionConflict` + blocker):
  SupersedeImport, RejectImport, VoidDecisionEvent. These are EXCLUDED from the void list.

**Already-voided pre-filter — justification:**

A VoidDecisionEvent whose `target_event_id` resolves to ANOTHER VoidDecisionEvent payload fires
an IMMEDIATE `DecisionConflict` (resolve.rs line 314: "Non-revocable: the void itself is the
conflict; target stays in force"). This is why `VoidDecisionEvent` is excluded from the revocable
set entirely.

Separately, attempting to void an ALREADY-VOIDED original decision (e.g., voiding a ClassifyInbound
that was already voided) does NOT fire a conflict — the BTreeSet insert at resolve.rs:330 [N1] is
idempotent; the original decision was already excluded from the projection. The pre-filter
excluding already-voided decisions is justified by **UX cleanliness** (no point offering already-
inert decisions), not by conflict prevention. The void-of-void conflict behavior at resolve.rs:314
independently justifies excluding `VoidDecisionEvent` from the revocable-payload filter.

**[M3] Known exception to the "already-inert" justification — rejected SafeHarbor void hides an
IN-FORCE allocation.** A REJECTED void attempt against an EFFECTIVE `SafeHarborAllocation`
(conflict fires; the allocation stays in force, resolve.rs:926–933) still places the allocation's
id in the TUI's voided set — the pre-filter cannot distinguish an applied void from a rejected
one without re-running the step-3 effectiveness adjudication. The allocation therefore vanishes
from the void list while NOT inert. **Decision: record the gap, do not refine the pre-filter.**
Justification: re-voiding an effective allocation only re-fires the same conflict (§7.4
irrevocability — no remedial power is lost by hiding it), and re-listing correctly would require
duplicating resolve's step-3 conservation + deadline adjudication in the TUI — disproportionate
for chunk 2b. The D3.1 SafeHarbor modal warning states the permanence ("a rejected void
permanently removes this allocation from this list; the CLI void remains available"), and the
gap is recorded in FOLLOWUPS (Task 3).

**Required filter for the void list:**

```rust
// Build voided set (IDs that have been targeted by a VoidDecisionEvent).
let voided: BTreeSet<EventId> = snap.events.iter()
    .filter_map(|e| {
        if let EventPayload::VoidDecisionEvent(v) = &e.payload {
            Some(v.target_event_id.clone())
        } else { None }
    })
    .collect();

snap.events
    .iter()
    .filter(|e| matches!(e.id, EventId::Decision { .. }))
    .filter(|e| !voided.contains(&e.id))           // exclude already-voided
    .filter(|e| is_revocable_payload(&e.payload))   // exclude non-revocable types
```

Where `is_revocable_payload` returns `true` for:
`TransferLink | ReclassifyOutflow | ClassifyInbound | ManualFmv | ClassifyRaw | MethodElection
| LotSelection | ReclassifyIncome | SafeHarborAllocation` and `false` for everything else
(SupersedeImport, RejectImport, VoidDecisionEvent, Acquire, Income, Dispose, TransferOut,
TransferIn, Unclassified, ImportConflict — imported and system events have Import EventIds,
not Decision EventIds, so they cannot appear in the void list; the is_revocable check on
Decision-id'd events guards only the decision payload variants).

---

## Current state (recon @ `fe726ff` — chunk 2a is shipped)

All chunk-2a infrastructure is live and tested. The shipped source baseline for chunk 2b is:

- **`crates/btctax-tui-edit/src/edit/persist.rs`** — contains `persist_tax_profile` (chunk 1),
  `persist_classify_inbound` (chunk 2a), `persist_reclassify_outflow` (chunk 2a). The
  `"append_"` token is in `persist_only_tokens` (persist.rs:577). Three new writers land here.
- **`crates/btctax-tui-edit/src/edit/form.rs`** — contains `FieldBuffer` (FIELD_CAP=64,
  form.rs:17–61), `TargetList<T>` (form.rs:208–259), `InboundListItem`, `OutflowListItem`,
  `ClassifyInboundFlowState`/`Step`, `ClassifyInboundModalState`, `ReclassifyOutflowFlowState`/
  `Step`, `ReclassifyOutflowModalState`, validation fns, cycle helpers. The `TargetList<T>` widget
  is reused verbatim by the three new flows.
- **`crates/btctax-tui-edit/src/editor.rs`** — `EditorApp` carries `classify_inbound_flow:
  Option<ClassifyInboundFlowState>`, `classify_inbound_modal: Option<ClassifyInboundModalState>`,
  `reclassify_outflow_flow: Option<ReclassifyOutflowFlowState>`, `reclassify_outflow_modal:
  Option<ReclassifyOutflowModalState>`. Chunk 2b adds six more fields (three flows, three modals).
- **`crates/btctax-tui-edit/src/main.rs`** — `handle_key` dispatch order: modal layers (1–3,
  chunk 1 + 2a) → flow layer (chunk 2a) → form layer → screen. Chunk 2b inserts three more modal
  checks (layers 4–6) and extends the flow layer. Browse key bindings: `p`/`c`/`o` (chunk 1/2a);
  chunk 2b adds `r` / `f` / `v`.
- **`crates/btctax-tui-edit/src/draw_edit.rs`** — `draw_mutation_modal`, `draw_classify_inbound_*`,
  `draw_reclassify_outflow_*` renderers. Chunk 2b adds `draw_reclassify_income_*`,
  `draw_set_fmv_*`, `draw_void_*`.
- **`crates/btctax-core/src/project/resolve.rs`** — ReclassifyIncome FIRST-WINS (lines 662–676),
  ManualFmv latest-wins (lines 423–456), VoidDecisionEvent handling (lines 300–340). No change.
- **`crates/btctax-core/src/event.rs`** — `ReclassifyIncome{income_event, business, kind}`,
  `ManualFmv{event, usd_fmv}`, `VoidDecisionEvent{target_event_id}` — all existing payload types.
  No new types needed.
- **Status remedy strings in `main.rs`** — `derive_classify_inbound_status` and
  `derive_reclassify_outflow_status` currently name only the CLI void path. Chunk 2b updates
  these to name the in-editor void flow first (D3 — exact strings specified there).

---

## Design

### D1 — reclassify-income flow

**Key binding:** `r` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (target list).** Build `TargetList<IncomeListItem>` from the snapshot using
the pre-filter in Claim C:
- Filter `snap.events` by `EventPayload::Income(_)`.
- Exclude events targeted by a non-voided `ReclassifyIncome` decision.
- For each surviving event: construct `IncomeListItem` by looking up the event in `snap.events`
  (for date, sat, wallet) and optionally in `snap.state.income_recognized` (for fmv).

If the filtered list is empty, show status `"No reclassifiable income events"` and return to
Browse without opening the flow [R0-M8 discipline, identical to 2a].

Display as a `Table` with columns: `Date | Sat | Kind | Business | FMV | EventId`. FMV column
shows `"(pending)"` for events with `FmvMissing` (no income_recognized entry).
Title: `" Reclassify Income — select Income event target "`.

**Display data type:**

```rust
pub struct IncomeListItem {
    pub income_event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: IncomeKind,           // from Income payload (original, pre-reclassify)
    pub business: bool,             // from Income payload (original)
    pub fmv: Option<Usd>,          // from income_recognized if present; None if FmvMissing
    pub wallet: Option<WalletId>,
}
```

Date and wallet derived from `snap.events` lookup by `EventId` (same O(n) helper
`events_by_id` from chunk 2a, or reuse directly). Kind and business from the `Income` payload.
FMV from `snap.state.income_recognized` for events with the matching `event` field.

**Step 2 — field form (no variant picker; single form directly).** After `Enter` on a list item,
transition to `ReclassifyIncomeStep::FieldForm`. Two fields:

| Field | Type | Required? | Semantics |
|---|---|---|---|
| `business` | `Option<bool>` toggle | **REQUIRED-EXPLICIT** | Initial `None` (not chosen); Tab cycles `None → true → false → None`; display `"---"` when None (required, must choose); error on submit if None ("business is required (press Tab to choose)"). CLI parity: `--business` is `required=true` with `ArgAction::Set` (main.rs:301–302). |
| `kind` | `Option<IncomeKind>` picker | optional | Initial `None` = keep original; Tab cycles `None → Mining → Staking → Interest → Airdrop → Reward → None`; display `"keep original"` when None. |

Focus order: business (row 0, Tab-cycles the 3-state) → kind (row 1, Tab-cycles 6 options).
`↑/↓` move focus. `Enter` validates (business must be non-None) + opens modal. `Esc` → back to
the **List** step [I4 — the 2a Esc-steps-back rule, inherited verbatim: every Esc press steps
back exactly ONE step; FieldForm → List → close flow. The earlier "no picker step to step back
to" rationale was false — the List step exists and is the one-step-back target].

**Flow state struct:**

```rust
pub enum ReclassifyIncomeStep {
    List,
    FieldForm {
        item: IncomeListItem,
        /// 3-state: None = not chosen (REQUIRED-EXPLICIT); Some(true/false) = chosen.
        business: Option<bool>,
        /// None = keep original kind (optional).
        kind: Option<IncomeKind>,
        /// 0 = business, 1 = kind.
        focus: usize,
        error: Option<String>,
    },
}

pub struct ReclassifyIncomeFlowState {
    pub list: TargetList<IncomeListItem>,    // OWNED by the flow [R0-I2 discipline]
    pub step: ReclassifyIncomeStep,
}
```

**Validation** (pure fn, validate-at-submit):
- `business`: `None` → error `"business is required (press Tab to choose true or false)"`.
  `Some(b)` → valid.
- `kind`: always valid (Option picker).
- Builds `EventPayload::ReclassifyIncome(ReclassifyIncome { income_event: item.income_event.clone(), business: b, kind })`.

**Modal content — reclassify-income:**

```rust
pub struct ReclassifyIncomeModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub original_kind: IncomeKind,
    pub original_business: bool,
    pub new_business: bool,           // the validated choice
    pub new_kind: Option<IncomeKind>, // None = keep original
}
```

Modal rendering:
```
╔═ Confirm: reclassify-income — WRITES THE VAULT ═════════╗
║  target:  in|river|…   (Income)                         ║
║  date:    2025-06-15                                     ║
║  sat:     100000                                         ║
║                                                          ║
║  original: kind=staking  business=false                  ║
║  override:                                               ║
║    business: true        (was false)                     ║
║    kind:     keep original                               ║
║                                                          ║
║  Effects: income_recognized updates; SE/NIIT exposure    ║
║  may change depending on the flip direction.             ║
║                                                          ║
║  Appended as a decision event (append-only log).         ║
║  Saved immediately via the vault's atomic write path.    ║
║                                                          ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

When `kind` is `Some(k)`: show `"kind: {display} (was {original})"`. The consequence note ("SE/
NIIT exposure may change") is ALWAYS shown — this is the general consequence, not a computed
figure. The status after save is blocker-derived, not tax-figure-keyed (no claim like "SE tax
increased by $X").

**Post-effect status — derived from RE-PROJECTED state:**

```
derive_reclassify_income_status(snap, &target_event, &decision_id) → String
```

- **DecisionConflict attributed to `decision_id`** (retry-duplicate FIRST-WINS, resolve.rs:665):
  → `"Saved, but DecisionConflict fired on this decision — see Compliance; clear with Void flow
     (press 'v') or CLI: btctax reconcile void {decision_id.canonical()}"` [D3 remedy update]
- **No target-attributed blocker**: clean success → `"Reclassified income: business={new_business},
  kind={effective_kind}"` where `effective_kind` = `new_kind.map(display).unwrap_or("original")`.

**Enter-arm semantics** (identical to chunk 2a D4): `Ok(id)` → re-project + derive status + close
modal + close flow; `Err(e)` → close modal, keep FieldForm open (buffers intact), status =
`"Save error: {e}"`.

---

### D2 — set-fmv flow

**Key binding:** `f` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (target list).** Build `TargetList<FmvListItem>` using the filter in
Claim D. If empty: status `"No FMV-missing income events"`, return to Browse [R0-M8].

Render as a `Table`: `Date | Sat | Kind | EventId`. Title:
`" Set FMV — select FmvMissing Income event "`.

**Display data type:**

```rust
pub struct FmvListItem {
    pub event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: IncomeKind,          // from Income payload
    pub wallet: Option<WalletId>,
}
```

**Step 2 — field form (single field; no picker).** After `Enter` on a list item:

| Field | Type | Required? | Validation |
|---|---|---|---|
| `usd_fmv` | `FieldBuffer` | **REQUIRED** | empty → `"usd-fmv is required"`; non-empty → `Usd::from_str(trim)` (parse_usd_arg semantics). |

No other fields. `Enter` validates → opens modal. `Esc` → back to the **List** step [I4 —
Esc-steps-back, one step per press: FieldForm → List → close flow].

**Flow state struct:**

```rust
pub enum SetFmvStep {
    List,
    FieldForm {
        item: FmvListItem,
        usd_fmv_buf: FieldBuffer,
        error: Option<String>,
    },
}

pub struct SetFmvFlowState {
    pub list: TargetList<FmvListItem>,   // OWNED by the flow
    pub step: SetFmvStep,
}
```

**Modal content — set-fmv:**

```rust
pub struct SetFmvModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub target_kind: IncomeKind,
    pub usd_fmv: Usd,
}
```

Modal rendering:
```
╔═ Confirm: set-fmv — WRITES THE VAULT ═══════════════════╗
║  target:  in|river|…   (Income)                         ║
║  date:    2025-04-01                                     ║
║  sat:     50000                                          ║
║  kind:    staking                                        ║
║                                                          ║
║  usd_fmv: 45.00                                          ║
║                                                          ║
║  Effects: FmvMissing blocker clears; income + lot        ║
║  materialize at this FMV.                                ║
║                                                          ║
║  Appended as a decision event (append-only log).         ║
║  Saved immediately via the vault's atomic write path.    ║
║                                                          ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

**Post-effect status — derived from RE-PROJECTED state:**

```
derive_set_fmv_status(snap, &target_event, &decision_id) → String
```

- **`FmvMissing` with `event == target_event` STILL present** (unusual — could be the wallet-less
  Income arm, Claim D note, or an unexpected edge): → `"FMV set but FmvMissing re-fired for this
  event — see Compliance; blocker detail: {b.detail}"`.
- **DecisionConflict attributed to `decision_id`**: unreachable for set-fmv (latest-wins, no
  conflict on retry), but defensively: → `"Saved, but DecisionConflict fired on this decision —
  see Compliance; clear with Void flow (press 'v') or CLI: btctax reconcile void
  {decision_id.canonical()}"`. [Stated for completeness; KAT does not probe this arm.]
- **Clean (FmvMissing cleared):** → `"FMV set: {usd_fmv} for {target_event.canonical()} —
  FmvMissing blocker cleared"`.

The re-point case (second set-fmv, re-projected cleanly): the FmvMissing blocker should be
absent (it cleared after the FIRST set-fmv + re-projection). Re-pointing shows the second value
as the displayed FMV in income_recognized; status = clean success with the new FMV value.

---

### D3 — void flow + remedy-string update

#### D3.1 — void flow

**Key binding:** `v` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (target list).** Build `TargetList<VoidListItem>` using the filter in
Claim E. If empty: status `"No revocable decisions to void"`, return to Browse [R0-M8].

Render as a `Table`: `Seq | Type | Target summary`. Title:
`" Void Decision — select decision to void "`.

**Display data type:**

```rust
pub struct VoidListItem {
    pub event_id: EventId,           // the decision EventId; EventId::Decision{seq}
    pub seq: u64,                    // for display in the Seq column
    pub payload_tag: &'static str,   // "TransferLink" | "ReclassifyOutflow" | "ClassifyInbound"
                                     // | "ManualFmv" | "ClassifyRaw" | "MethodElection"
                                     // | "LotSelection" | "ReclassifyIncome" | "SafeHarborAllocation"
    pub target_summary: String,      // human-readable target: e.g. "→ in|river|…" or "from 2025-01-01"
    /// [M5] The decision's OWN inner target event (the event the decision acts on):
    /// tl.out_event / ro.transfer_out_event / ci.transfer_in_event / m.event / cr.target /
    /// ls.disposal_event / ri.income_event; None for MethodElection and SafeHarborAllocation
    /// (no single inner event). Used by `derive_void_status`'s returned-blocker check.
    pub inner_target: Option<EventId>,
}
```

The `target_summary` is a best-effort short description of what the decision targets, computed
once at list-open from the decision payload:
- TransferLink: `"out → {tl.out_event.canonical()}"`.
- ReclassifyOutflow: `"out {ro.transfer_out_event.canonical()} as {kind}"`.
- ClassifyInbound: `"in {ci.transfer_in_event.canonical()} as {class}"`.
- ManualFmv: `"fmv={m.usd_fmv} for {m.event.canonical()}"`.
- ClassifyRaw: `"raw {cr.target.canonical()}"`.
- MethodElection: `"method={me.method:?} from {me.effective_from}"`.
- LotSelection: `"lots for {ls.disposal_event.canonical()}"`.
- ReclassifyIncome: `"income {ri.income_event.canonical()} biz={ri.business}"`.
- SafeHarborAllocation: `"alloc {lots_count} lots as_of {a.as_of_date}"`.

**Step 2 — no field form.** The void flow has NO FieldForm step. Pressing `Enter` on a list item
transitions DIRECTLY to opening `VoidModalState` on `EditorApp`. There is no intermediate
picker or field form.

List-step keys:
| Key | Action |
|---|---|
| `↑` / `k` | `list.scroll_up()` |
| `↓` / `j` | `list.scroll_down()` |
| `g` | `list.go_top()` |
| `G` | `list.go_bottom()` |
| `Enter` | select item → open `void_modal` (DIRECTLY to modal) |
| `Esc` | close flow → Browse (nothing written) |
| `q` | SWALLOWED (flow is blocking) |

**Flow state struct:**

```rust
pub enum VoidStep {
    List,
    // No FieldForm step — Enter from List goes directly to VoidModalState on EditorApp.
}

pub struct VoidFlowState {
    pub list: TargetList<VoidListItem>,   // OWNED by the flow
    pub step: VoidStep,
}
```

**Modal content — void:**

```rust
pub struct VoidModalState {
    pub target_event_id: EventId,     // the decision being voided
    pub seq: u64,
    pub payload_tag: &'static str,
    pub target_summary: String,
    pub inner_target: Option<EventId>, // carried through from VoidListItem [M5]
    pub is_safe_harbor: bool,         // true if SafeHarborAllocation — show conditional note
}
```

Modal rendering (the consequence note states BOTH consequence categories [I1]):
```
╔═ Confirm: void decision — WRITES THE VAULT ═════════════╗
║  decision: decision|42  (ReclassifyIncome)               ║
║  target:   income {ri.income_event.canonical()}          ║
║                                                          ║
║  Consequence: this decision's effects un-project.        ║
║  Prior blockers may return (e.g. voiding a ClassifyInbound║
║  returns UnknownBasisInbound; the pending row re-lists). ║
║  Decisions that DEPENDED on this one (e.g. a ManualFmv or ║
║  ReclassifyIncome on a ClassifyRaw'd event, or a          ║
║  LotSelection picking its lots) may now fire              ║
║  DecisionConflict/LotSelectionInvalid — void those too.   ║
║                                                          ║
║  Appended as a VoidDecisionEvent (append-only log).      ║
║  Saved immediately via the vault's atomic write path.    ║
║                                                          ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

For `SafeHarborAllocation` (when `is_safe_harbor = true`), APPEND a warning line [M3 permanence
note included]:
```
║  WARNING: If this allocation is effective (Path B), voiding ║
║  it fires DecisionConflict — irrevocable (§7.4). If inert,  ║
║  the void applies and the Path A default resumes.           ║
║  A rejected void permanently removes this allocation from   ║
║  this list (CLI void remains available).                    ║
```

**[I1] The dependent-decision cascade (verified at HEAD).** Pass-1d `ManualFmv` and pass-1e
`ReclassifyIncome` validate the **effective** payload of their target on EVERY projection
(`applied.get(target).unwrap_or(&raw.payload)`, resolve.rs:436–438, 644–646). Voiding a
`ClassifyRaw` whose target's effective payload had become `Income` therefore ORPHANS every
non-voided `ManualFmv` (resolve.rs:458–471) and `ReclassifyIncome` (resolve.rs:678–692) targeting
that event: on the next projection each orphan fires a **Hard `DecisionConflict` attributed to
the ORPHANED decision's own id — not the void's** — gating `compute_tax_year`. The same shape
exists via lots: voiding a `ClassifyInbound`/`ClassifyRaw` that created a lot picked by a
`LotSelection` makes the fold fire `LotSelectionInvalid`. This is reachable from the TUI
(`ClassifyRaw` is in the void list; a mixed CLI+TUI vault holds dependent decisions freely).
**Remedy: void the orphan — it appears in the `v` list** (it is a revocable, non-voided
decision). The modal consequence note above states this; KAT-E2E-VOID-CASCADE (D5) pins the
full loop.

**Post-effect status — derived from RE-PROJECTED state:**

```
derive_void_status(snap, &void_decision_id, &target_event_id, inner_target: Option<&EventId>)
    → String
```

- **DecisionConflict attributed to `void_decision_id`** (fires exactly when the void was
  REJECTED: void of an effective SafeHarborAllocation — resolve.rs:926–933, attributed to
  `v.void_id` — or void targeting an unknown event, resolve.rs:333–340): →
  `"Void saved, but DecisionConflict fired — the target decision remains in force (see
  Compliance)"` [I2 — the string must NOT lead with "Voided": the target decision was NOT
  revoked. No `{seq}` interpolation in this arm — the conflict is attributed to the VOID
  decision while the seq names the TARGET; mixing the two identities in one line was the I2
  ambiguity].
- **Returned blocker attributed to `inner_target`** [M5] (when `inner_target` is `Some` and a
  blocker in the re-projected state carries `event == inner_target`): →
  `"Voided {payload_tag} decision|{seq} — {blocker_kind} returned for {inner_target.canonical()}
  (see Compliance)"`. This makes the "prior blockers may return" case concrete (e.g. voiding a
  ClassifyInbound names the returned `UnknownBasisInbound`).
- **Clean (neither of the above):** → `"Voided {payload_tag} decision|{seq} — effects
  un-projected; check Compliance for any returned blockers"`.

**[I1] Deliberate surfacing limit of the clean arm:** cascade conflicts are attributed to the
ORPHANED decision's id — an id this fn does not know — so `derive_void_status` CANNOT detect
them; the cascade case reports through the clean (or returned-blocker) arm. This is deliberate:
the modal consequence note carries the warning pre-write, the Compliance tab lists the orphan's
Hard conflict post-write, and the orphan appears in the `v` list as its own remedy. A
whole-blockers diff (pre-void vs post-void) could detect cascades generically; that is deferred
(FOLLOWUPS, Task 3) as disproportionate for chunk 2b.

**Special case: voiding a `LotSelection`.** `persist_void` calls
`btctax_cli::optimize_attest::clear(session.conn(), &ls.disposal_event)` atomically before
`session.save()`. The status is the same as the clean case — no additional UI note required
(the attestation side-table is an internal optimization; the user's mental model is simply
"the lot selection is gone").

#### D3.2 — remedy-string update in `derive_classify_inbound_status` and `derive_reclassify_outflow_status`

Chunk 2b ships the void flow. The four existing status strings in `main.rs` that name only the
CLI void path MUST be updated to name the in-editor void flow FIRST, with the CLI as a fallback.

**[M2] The KAT updates are STRENGTHENINGS, not break-fixes.** Verified at HEAD: every currently
pinned substring SURVIVES the new strings, because the CLI path is retained verbatim in each —
KAT-S2 (main.rs:2896–2910) and KAT-S2-RO (main.rs:4400–4415) pin `"DecisionConflict"` +
`"void {canonical}"`; `kat_e2e_fmv_missing` (main.rs:3135), `kat_e2e_gift_unknown`
(main.rs:3202), and `kat_e2e_gift_price_gap_donor_date_outside_price_dataset` (main.rs:3256,
pinning arm 3's string — previously omitted from this list) pin `contains("void")`. Nothing
breaks; the mandate is to EXTEND each of these in-test assertions in place to ALSO pin the new
`"Void flow (press 'v')"` / `"'v'"` fragment, keeping every existing pin. Additionally the four
NEW derive-fn unit tests (KAT-RS-1..4, D5) are added. Nothing is deleted.

**Arm 1 — DecisionConflict, `derive_classify_inbound_status` (main.rs ~1346–1351):**

OLD:
```
"Saved, but DecisionConflict fired on this decision — see Compliance; \
 clear with CLI: btctax reconcile void {decision_id.canonical()}"
```
NEW:
```
"Saved, but DecisionConflict fired on this decision — see Compliance; \
 clear with Void flow (press 'v') or CLI: btctax reconcile void {decision_id.canonical()}"
```

**Arm 2 — FmvMissing, `derive_classify_inbound_status` (main.rs ~1363–1371):**

OLD:
```
"Classified as Income({kind}) but FMV missing — FmvMissing blocker fired; \
 to supply the FMV, void this decision (CLI: btctax reconcile void \
 decision|{seq}) and re-classify with an FMV"
```
NEW:
```
"Classified as Income({kind}) but FMV missing — FmvMissing blocker fired; \
 to supply the FMV, void this decision (Void flow: press 'v'; or CLI: btctax reconcile void \
 decision|{seq}) and re-classify with an FMV"
```

**Arm 3 — UnknownBasisInbound, `derive_classify_inbound_status` (main.rs ~1382–1387):**

OLD:
```
"Gift recorded but basis unknown — UnknownBasisInbound re-fired; \
 void this decision (CLI: btctax reconcile void decision|{seq}) \
 and re-classify with donor basis or a donor date covered by the price dataset"
```
NEW:
```
"Gift recorded but basis unknown — UnknownBasisInbound re-fired; \
 void this decision (Void flow: press 'v'; or CLI: btctax reconcile void decision|{seq}) \
 and re-classify with donor basis or a donor date covered by the price dataset"
```

**Arm 4 — DecisionConflict, `derive_reclassify_outflow_status` (main.rs ~1417–1421):**

OLD:
```
"Saved, but DecisionConflict fired on this decision — see Compliance; \
 clear with CLI: btctax reconcile void {decision_id.canonical()}"
```
NEW:
```
"Saved, but DecisionConflict fired on this decision — see Compliance; \
 clear with Void flow (press 'v') or CLI: btctax reconcile void {decision_id.canonical()}"
```

**Which arms change:** all four above. KAT assertions that pin "void" must be updated to also
assert the presence of `"'v'"` or `"Void flow"` (in addition to retaining the `"void "` check for
the CLI-path substring, which remains in the string).

---

### D4 — `edit/persist.rs` additions

Three new `pub fn` items in `edit/persist.rs` (the ONLY location permitted to name
`append_decision`):

```rust
/// Append a `ReclassifyIncome` decision and atomically save the vault.
///
/// `payload` is the VALIDATED `EventPayload::ReclassifyIncome(…)`.
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # FIRST-WINS semantics (resolve.rs:662–676)
/// A retry appends a duplicate `ReclassifyIncome` for the same `income_event`.
/// The FIRST (failed-save) decision stays in force; the duplicate fires a Hard
/// `DecisionConflict` on ITS id. Surfaced by the D1 step-2 status; cleared via
/// the in-editor Void flow ('v') or CLI: `btctax reconcile void decision|<seq>`.
pub fn persist_reclassify_income(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, btctax_cli::CliError> {
    let id = btctax_core::persistence::append_decision(
        session.conn(), payload, now, time::UtcOffset::UTC, None,
    )?;
    session.save()?;
    Ok(id)
}

/// Append a `ManualFmv` decision and atomically save the vault.
///
/// `payload` is the VALIDATED `EventPayload::ManualFmv(…)`.
/// `now` is INJECTED at Enter-press.
///
/// # LATEST-WINS semantics (resolve.rs:453–456)
/// A retry appends a second `ManualFmv` for the same event — NO conflict (latest-seq-wins;
/// the second FMV governs). Status after retry is clean-success. On-disk log grows by 2 rows.
pub fn persist_set_fmv(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, btctax_cli::CliError> {
    let id = btctax_core::persistence::append_decision(
        session.conn(), payload, now, time::UtcOffset::UTC, None,
    )?;
    session.save()?;
    Ok(id)
}

/// Append a `VoidDecisionEvent` decision and atomically save the vault.
///
/// `target_event_id` is the EventId of the revocable decision to void.
/// `now` is INJECTED at Enter-press.
///
/// # LotSelection side-effect (reconcile.rs:117–147)
/// If the target decision is a `LotSelection`, also calls
/// `btctax_cli::optimize_attest::clear(session.conn(), &ls.disposal_event)` BEFORE save —
/// same atomic batch as the CLI void command. Non-LotSelection targets are unaffected.
///
/// # Idempotent retry semantics [M1]
/// A retry appends a second `VoidDecisionEvent` for the SAME original target (NOT a
/// void-of-void — the target is the original decision, so resolve.rs:312–321 does not fire).
/// The BTreeSet insert in resolve.rs:330 is idempotent — no conflict fires; the second row is
/// inert. Status after retry: clean-success. On `Err(save)`: the void modal closes, the flow
/// stays at List, status "Save error: {e}"; retry = re-select → modal → Enter. Pinned by
/// KAT-VOID-RETRY.
pub fn persist_void(
    session: &mut btctax_cli::Session,
    target_event_id: btctax_core::EventId,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, btctax_cli::CliError> {
    use btctax_core::{EventPayload, event::VoidDecisionEvent};
    use btctax_core::persistence::{append_decision, load_all};

    // Detect LotSelection target for the optimize_attest side-effect.
    let events = load_all(session.conn())?;
    let disposal_to_clear: Option<btctax_core::EventId> = events
        .iter()
        .find(|e| e.id == target_event_id)
        .and_then(|e| match &e.payload {
            EventPayload::LotSelection(ls) => Some(ls.disposal_event.clone()),
            _ => None,
        });

    let id = append_decision(
        session.conn(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id }),
        now,
        time::UtcOffset::UTC,
        None,
    )?;

    if let Some(disposal) = disposal_to_clear {
        btctax_cli::optimize_attest::clear(session.conn(), &disposal)?;
    }

    session.save()?;
    Ok(id)
}
```

**Implementation notes:**
- `persist_void` uses `load_all` (not `load_all_ordered`) for the LotSelection lookup; the full
  event list is already in the in-memory Connection (same as the CLI). The lookup is O(n) and
  void is infrequent — acceptable.
- `btctax_cli::optimize_attest` is already a `pub mod` in `btctax-cli/src/lib.rs` (line 9), so
  it is accessible from `btctax-tui-edit` via the existing `btctax_cli` dependency.
- The `"append_"` token already covers all three fns (persist.rs:577); KAT-G1 requires no change.
- `load_all` reference: the token `"load_all"` should be verified against KAT-G1's fs-write token
  list — it is NOT in the forbidden set (it is a read, not a write).

---

### D5 — safety tests (KATs)

All tests are TDD-red first, then implementation, then green. The full validation suite must
pass at every step.

#### KAT-P2c — strict-prefix test for reclassify-income

Same pattern as KAT-P2a/P2b (persist.rs, the `kat_p2a_*` and `kat_p2b_*` skeletons):

```
post.len() == pre.len() + 1
post[..pre.len()] == pre[..]
post[pre.len()].kind == "decision"
post[pre.len()].decision_seq
  == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
serde_json::from_str::<EventPayload>(&post[pre.len()].payload_json)
  == EventPayload::ReclassifyIncome(expected)
```

Seed: a genuine `Income` import event + 1 prior `MethodElection` (so pre has 2 rows, non-trivial
decision_seq). The `persist_reclassify_income` payload references the seeded Income EventId.
Both in-memory and drop+reopen assertions.

#### KAT-P2d — strict-prefix test for set-fmv

Same structure, `persist_set_fmv` and `EventPayload::ManualFmv`. Seed a genuine `Income` import.
Payload round-trips; returned EventId matches tail.

#### KAT-P2e — strict-prefix test for void

Same structure, `persist_void` and `EventPayload::VoidDecisionEvent`. Seed a `MethodElection`
decision to void. Assert tail payload round-trips as `VoidDecisionEvent{target_event_id}`.
Additionally assert: the seeded MethodElection's EventId == `target_event_id` in the tail
payload (the void targets the right event).

#### KAT-C2c — cancel-path bytes-unchanged (reclassify-income)

Pattern: chunk-1 KAT-C1 (`kat_c1_*`, main.rs:2099–2172 [N1 citation fix]) and KAT-C2a/C2b.

Temp vault; `bytes_before`. Press `r` → flow opens at List; press `Enter` → FieldForm; choose
business (Tab to `true`); press `Enter` → modal opens; press `Esc` → modal closes (FieldForm
still open); press `Esc` → back to **List** [I4 — one step back per press]; press `Esc` → flow
closes. Assert `q` swallowed at each flow step. `bytes_after == bytes_before`. Complement:
confirmed path writes.

#### KAT-C2d — cancel-path bytes-unchanged (set-fmv)

`f` → list; `Enter` → FieldForm; type FMV; `Enter` → modal; `Esc` → modal closes (FieldForm
still open); `Esc` → back to **List** [I4]; `Esc` → flow closes; `q` swallowed at each step.
Bytes unchanged. Complement: confirmed path writes.

#### KAT-C2e — cancel-path bytes-unchanged (void)

`v` → list; `Enter` → modal opens (DIRECTLY, no form); `Esc` → modal closes (back to the List
step — the modal-Esc lands on the list); `Esc` → flow closes [N2 — exactly TWO Esc presses
after the modal opens: modal→list, list→close]. `q` swallowed at list and at modal. Bytes
unchanged. Complement: confirmed path writes.

#### KAT-S2b — save-error path for set-fmv (`#[cfg(unix)]`)

**Justification for sampling (one flow rather than three):** KAT-S2 (classify-inbound, main.rs
2734–2911 [N1 citation fix]) already proves the failed-save chmod pattern. The ONLY new detail
in chunk 2b is the LATEST-WINS semantics for set-fmv (retry yields +2 rows, NO conflict, second
FMV governs). Applying the chmod pattern to set-fmv targets this unique claim; reclassify-income
would be redundant with S2 (FIRST-WINS + conflict, already proven by that pattern). Void's
retry contract (+2 inert rows, no conflict) is pinned by the cheap KAT-VOID-RETRY unit test
below [M1] — no chmod machinery needed for it.

Steps (mirrors KAT-S2, including root-skip guard):
1. Seed an Income event; open session; `pre = load_all_ordered(conn)`.
2. Navigate to modal; chmod parent dir 0o500; `Enter` → save error; assert: modal closed, form
   open, status "Save error", bytes unchanged. In-memory carries ManualFmv seq N+1.
3. Restore perms; re-submit → retry appends ManualFmv seq N+2, save succeeds.
4. Assert: on-disk log == `pre` + **2** decision rows; both payload_json == same ManualFmv; re-
   projected state has NO `FmvMissing` for the target (second FMV governs, NO conflict); status =
   clean-success with the FMV value.

#### KAT-E2E-RI — end-to-end reclassify-income (business flip)

1. Seed an `Income{kind: Reward, business: false}` event. Confirm it projects as
   `IncomeRecord{business: false, kind: Reward}` (project call).
2. Drive `r` → list shows the event; `Enter` → FieldForm; Tab business to `true`; leave kind as
   `None` (keep original); `Enter` → modal (assert shows "business: true (was false)" and "kind:
   keep original"); `Enter` → save.
3. Re-project; assert `IncomeRecord{kind: Reward, business: true}` for the target EventId; the
   original `business: false` record is GONE (ReclassifyIncome override applies).
4. Assert the event no longer appears in the `r` list (non-voided ReclassifyIncome pre-filter
   excludes it).

#### KAT-E2E-RI-SE — reclassify-income NIIT + SE effect (Interest → Mining, exact figures) [I3]

**[I3] The original Reward→Mining fixture could not move NIIT — corrected.** Verified at HEAD:
NII = capital-gain components + `interest_nii`, and `interest_nii` filters
`kind == IncomeKind::Interest` ONLY (compute.rs:306–309, 352–354 — the `business` flag is not
consulted); SE eligibility = `business && kind != Interest` (se.rs:59). A Reward→Mining flip
therefore moves SE but CANNOT move NIIT (neither kind is ever NII; MAGI is unchanged). This KAT
uses an **Interest → Mining** flip instead, so BOTH figures move, with exact values reused from
the core ReclassifyIncome KAT fixture (`crates/btctax-core/tests/reclassify_income.rs` —
`niit_profile()` at lines 130–145 and the ±$380 derivation at lines 369–388, plus the SE math
at lines 166–173):

Fixture: `Income{kind: Interest, business: false, fmv: $10,000}` + `niit_profile()` (Single,
`ordinary_taxable_income = $0`, `magi_excluding_crypto = $205,000` — above the Single $200,000
§1411 threshold, so NIIT is non-vacuous).

1. **Before reclassify** (project + `compute_tax_year`): `interest_nii = $10,000` →
   `niit = round_cents(3.8% × $10,000) = $380.00` (exact); `se_net_income = 0` →
   `compute_se_tax = None` (Interest is SE-EXCLUDED per §1402(a)(2) regardless of `business`).
2. **Drive the TUI flow:** `r` → select the Interest event → FieldForm: Tab business to `true`,
   Tab kind to `Mining` → `Enter` → modal (assert it shows "business: true (was false)" and
   "kind: mining (was interest)") → `Enter` → save + re-project.
3. **After reclassify** (exact asserts):
   - `niit = $0` — Interest left NII; the delta is **−$380.00** (exact).
   - `compute_se_tax = Some(…)` with the core-KAT hand-derived figures for fmv=$10,000, Single,
     no W-2: `base = $9,235.00`, `ss = $1,145.14`, `medicare = $267.82`, `total = $1,412.96`,
     `deductible_half = $706.48`.
   - The TUI `status` is the CLEAN success string (`"Reclassified income: …"`) — NO tax figure
     appears in the status (the blocker-derived-status discipline; figures are asserted on the
     computed `TaxResult`, never on the status text).

#### KAT-E2E-FMV — end-to-end set-fmv (clears blocker + income materializes)

1. Seed an `Income{kind: Staking, business: false}` event with no FMV (or FmvStatus::Missing).
   Confirm `FmvMissing` blocker fires; event appears in `f` list; income_recognized does NOT
   contain an entry for the target.
2. Drive `f` → list; `Enter` → FieldForm; type `"45.00"`; `Enter` → modal (assert shows
   `usd_fmv: 45.00` and target canonical id); `Enter` → save + re-project.
3. Assert: `FmvMissing` for target is GONE; `income_recognized` contains an entry with
   `{usd_fmv: 45.00, kind: Staking, business: false}`; event is no longer in the `f` list.
4. The lot has `usd_basis = 45.00` (not `basis_pending`).

#### KAT-E2E-FMV-REPOINT — set-fmv re-point (second set-fmv, NO conflict)

After KAT-E2E-FMV (FmvMissing cleared), attempt a second set-fmv on the same event:
1. The event NO LONGER appears in the `f` list (FmvMissing cleared → event not in blockers list).
   So the re-point can only be tested at the unit level (persist_set_fmv called twice).
2. Unit test: call `persist_set_fmv` twice on the same event, second with different FMV. Assert:
   on-disk log has `pre + 2` ManualFmv rows; no DecisionConflict; `income_recognized` reflects
   the SECOND FMV. This proves latest-wins (resolve.rs:453–456) is correct and the spec's
   "no pre-filter for already-set FMVs" claim is safe.

#### KAT-E2E-VOID-ROUNDTRIP — void round-trip (classify → void → blocker returns → re-classify)

This is the primary correctness proof for the in-editor remedy loop:

1. Seed a `TransferIn` event. Confirm `UnknownBasisInbound` fires; event appears in `c` list.
2. **Classify-inbound (chunk 2a flow):** drive `c` → select event → Income → enter kind=Staking,
   fmv=200 → confirm. Re-project: `UnknownBasisInbound` GONE; `IncomeRecord` appears.
3. The event is now PRE-FILTERED from the `c` list (has a non-voided ClassifyInbound).
4. **Void the ClassifyInbound (chunk 2b void flow):** drive `v` → list shows the ClassifyInbound
   decision; select it → modal shows the decision + consequence note; confirm. Re-project:
   `UnknownBasisInbound` RETURNS; the event is back in the `c` list; `IncomeRecord` GONE.
5. **Re-classify cleanly:** drive `c` → select the same TransferIn → Income → kind=Mining,
   fmv=250 → confirm. Re-project: `UnknownBasisInbound` GONE; new `IncomeRecord{kind:Mining,
   fmv:250}` appears. No `DecisionConflict` (the old ClassifyInbound is voided; the new one is
   the first non-voided one for this target — resolve.rs:554–563 FIRST-WINS does not conflict).
6. Assert the event does NOT appear in the `v` list as the OLD (voided) ClassifyInbound
   (already-voided pre-filter, Claim E).

#### KAT-E2E-VOID-RECLASSIFY-INCOME — void + reclassify-income round-trip

1. Seed an `Income{business:false}` event.
2. Reclassify-income: `business=true`. Re-project: IncomeRecord has business=true; event absent
   from `r` list (pre-filter).
3. Void the ReclassifyIncome: `v` → list shows the ReclassifyIncome; confirm. Re-project:
   IncomeRecord has `business=false` (original restored); event is back in `r` list.
4. Re-reclassify: `business=true, kind=Mining`. Re-project: IncomeRecord has `{business:true,
   kind:Mining}`. No conflict (old ReclassifyIncome voided; new one is the first non-voided).

#### KAT-E2E-VOID-CASCADE — dependent-decision cascade: orphaned ManualFmv → its own void [I1]

Pins the full cascade + remedy loop stated in D3.1:

1. Seed an `Unclassified` event (with a wallet — the fold's Income arm needs one to reach the
   FMV check). Append (CLI-style, test-region `append_decision`) a
   `ClassifyRaw{target, as_: Income{fmv: None, …}}` — the effective payload becomes Income;
   `FmvMissing` fires. Append a `ManualFmv{event: target, usd_fmv: 100}` — valid at pass-1d
   (effective payload IS Income); `FmvMissing` clears. Confirm the projected pre-state is clean.
2. **TUI-void the ClassifyRaw:** drive `v` → list shows BOTH the ClassifyRaw and the ManualFmv
   decisions; select the ClassifyRaw → modal (assert the consequence note mentions dependent
   decisions) → confirm.
3. Re-project; assert the cascade: a **Hard `DecisionConflict` attributed to the ManualFmv
   decision's id** (NOT the void's id — pass-1d re-validation, resolve.rs:458–471: the target's
   effective payload reverted to `Unclassified`); `compute_tax_year` gated.
4. Assert the surfacing limit honestly: the void's own status was the CLEAN (or returned-blocker)
   string — the cascade conflict is attributed elsewhere and is NOT in the void status (the
   D3.1 deliberate limit).
5. **The remedy loop:** drive `v` again → the orphaned ManualFmv IS in the list (revocable,
   non-voided); select + confirm. Re-project: the `DecisionConflict` is GONE; the state is clean
   (the original `Unclassified` blocker for the raw event is back — the honest baseline).

#### KAT-VOID-CONFLICT-ARM — void conflict-arm status string (synthetic snapshot unit KAT) [I2]

No Path-B vault fixture needed. Build a synthetic `Snapshot` whose `state.blockers` contains a
`DecisionConflict` blocker with `event == Some(void_decision_id)`. Call `derive_void_status`
with that snapshot. Assert the returned string:
- starts with (or contains) `"Void saved, but DecisionConflict fired"`;
- contains `"the target decision remains in force"`;
- does NOT start with `"Voided"` (the void did NOT take effect — the honesty pin).

(The effective-SafeHarborAllocation *E2E* remains deferred to FOLLOWUPS — this unit KAT covers
the string arm, which previously had zero coverage.)

#### KAT-VOID-RETRY — void retry contract (persist.rs unit test) [M1]

Cheap unit test, no chmod machinery: seed a `MethodElection` decision; `pre =
load_all_ordered(conn)`; call `persist_void(session, target, now)` TWICE. Assert:
- on-disk log == `pre` + **2** rows; BOTH tails round-trip as `VoidDecisionEvent` with the
  SAME `target_event_id`;
- the re-projected state shows the target still excluded (the MethodElection contributes no
  in-force election) and **no new blocker** (idempotent re-void — resolve.rs:330 insert);
- no `DecisionConflict` anywhere (the retry is NOT a void-of-void: its target is the original
  decision, not the first VoidDecisionEvent).

#### KAT-VOID-EXCLUSIONS — void list correctly excludes non-revocable + already-voided

Drive `v` with a vault containing:
1. A `SupersedeImport` decision → NOT in the void list.
2. A `RejectImport` decision → NOT in the void list.
3. A `VoidDecisionEvent` decision → NOT in the void list.
4. A `ClassifyInbound` that has already been voided → NOT in the void list.
5. A non-voided `ReclassifyOutflow` → IS in the void list.
6. A non-voided `MethodElection` → IS in the void list.

Assert the list contains items 5 and 6 only.

#### KAT-RI-REQUIRED-BUSINESS — business required-explicit: cannot submit without a choice

On the reclassify-income FieldForm:
- Initial state: `business = None`; form shows `"business: ---"` label and `"[required]"` marker.
- Press `Enter` → error: `"business is required (press Tab to choose true or false)"`.
- Press `Tab` → `business = Some(true)`.
- Press `Enter` → modal opens successfully (no error).
- (Also: Tab again → `Some(false)`; Tab again → `None`; Tab again → `Some(true)` — cycle pins.)

#### KAT-V-RI-1..4 — reclassify-income field validation

- **KAT-V-RI-1:** business = None → error "business is required".
- **KAT-V-RI-2:** business = Some(true) → valid; builds payload with `business: true`.
- **KAT-V-RI-3:** business = Some(false) → valid; builds payload with `business: false`.
- **KAT-V-RI-4:** kind = None → `kind: None` in payload; kind = Some(Mining) → `kind: Some(Mining)`.

#### KAT-V-FMV-1..3 — set-fmv field validation

- **KAT-V-FMV-1:** `usd_fmv` empty → error "usd-fmv is required".
- **KAT-V-FMV-2:** `usd_fmv` valid decimal → parses correctly.
- **KAT-V-FMV-3:** `usd_fmv` whitespace-only → parse error (not "required" — [R0-M4] pin).

#### KAT-REMEDY-STRINGS — updated remedy strings in derive_* fns [M2]

Four NEW unit tests (one per updated arm, D3.2) — **added alongside** the existing E2E pins,
which are **strengthened in place**; nothing is deleted [M2]:

- **KAT-RS-1:** `derive_classify_inbound_status` with a `DecisionConflict` blocker → status
  contains `"'v'"` AND `"btctax reconcile void"` (both: new form has both; old form had only CLI).
- **KAT-RS-2:** `derive_classify_inbound_status` with a `FmvMissing` blocker → status contains
  `"'v'"` AND `"btctax reconcile void"`.
- **KAT-RS-3:** `derive_classify_inbound_status` with `UnknownBasisInbound` re-fire → status
  contains `"'v'"` AND `"btctax reconcile void"`.
- **KAT-RS-4:** `derive_reclassify_outflow_status` with `DecisionConflict` → status contains
  `"'v'"` AND `"btctax reconcile void"`.

In-place strengthenings of the existing 2a tests (every existing pin SURVIVES — the CLI path
is retained in each new string; these edits ADD the new-fragment assert, they remove nothing):
KAT-S2 (main.rs:2896–2910), KAT-S2-RO (main.rs:4400–4415), `kat_e2e_fmv_missing`
(main.rs:3135), `kat_e2e_gift_unknown` (main.rs:3202), and
`kat_e2e_gift_price_gap_donor_date_outside_price_dataset` (main.rs:3256) each gain a
`contains("'v'")` (or `contains("Void flow")`) assert next to their existing `"void"` /
`"void {canonical}"` pins.

#### KAT-G1 (inherited — must stay green throughout)

All three new persist fns live in `edit/persist.rs`. No `append_decision`, `conn(`, `save(`, or
`append_` token appears in any other source file's non-test region. KAT-G1
(`kat_g1_mechanized_source_gate`) requires no modification.

---

## Plan (TDD)

### Task 1 — reclassify-income + set-fmv flows

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `IncomeListItem`, `FmvListItem`,
  `ReclassifyIncomeStep`, `ReclassifyIncomeFlowState`, `ReclassifyIncomeModalState`,
  `SetFmvStep`, `SetFmvFlowState`, `SetFmvModalState`, `validate_reclassify_income`,
  `validate_set_fmv`.
- `crates/btctax-tui-edit/src/editor.rs` — add `reclassify_income_flow: Option<…>`,
  `reclassify_income_modal: Option<…>`, `set_fmv_flow: Option<…>`, `set_fmv_modal: Option<…>`;
  corresponding `None` in `EditorApp::new`.
- `crates/btctax-tui-edit/src/main.rs` — add `r` / `f` key dispatch; extend modal checks
  (layers 4–5); `handle_reclassify_income_*` and `handle_set_fmv_*` handlers; new
  `derive_reclassify_income_status` and `derive_set_fmv_status` fns.
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_reclassify_income_*` and `draw_set_fmv_*`
  renderers.
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_reclassify_income`,
  `persist_set_fmv`.

**KATs:** KAT-P2c, KAT-P2d, KAT-C2c, KAT-C2d, KAT-E2E-RI, KAT-E2E-RI-SE, KAT-E2E-FMV,
KAT-E2E-FMV-REPOINT, KAT-RI-REQUIRED-BUSINESS, KAT-V-RI-1..4, KAT-V-FMV-1..3. TDD-red before
implementation; green after.

### Task 2 — void flow + remedy-string update

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `VoidListItem`, `VoidStep`,
  `VoidFlowState`, `VoidModalState`, `is_revocable_payload` helper.
- `crates/btctax-tui-edit/src/editor.rs` — add `void_flow: Option<…>`, `void_modal: Option<…>`.
- `crates/btctax-tui-edit/src/main.rs` — `v` key dispatch; modal layer 6; `handle_void_flow_key`
  (list + direct-to-modal on Enter), `handle_void_modal_key`; `derive_void_status` (I2 wording,
  M5 inner-target arm); update the four remedy-string arms (D3.2); strengthen the five existing
  KAT assertions in place per [M2] (add the `"'v'"` pin; delete nothing).
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_void_list`, `draw_void_modal` (cascade
  consequence note [I1] + SafeHarbor warning incl. the [M3] permanence line).
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_void`.

**KATs:** KAT-P2e, KAT-C2e, KAT-S2b, KAT-E2E-VOID-ROUNDTRIP, KAT-E2E-VOID-RECLASSIFY-INCOME,
KAT-E2E-VOID-CASCADE [I1], KAT-VOID-CONFLICT-ARM [I2], KAT-VOID-RETRY [M1],
KAT-VOID-EXCLUSIONS, KAT-REMEDY-STRINGS (KAT-RS-1..4 + the five in-place strengthenings [M2]).
TDD-red before; green after.

### Task 3 — whole-diff review (Phase E) + FOLLOWUPS

Cross-cutting checks:

- **Editor guarantee unchanged:** `append_decision` and `conn(`/`save(` appear only in
  `edit/persist.rs` non-test code; KAT-G1 green; no new forbidden tokens elsewhere.
- **Modal gating:** `persist_reclassify_income` sole non-test call site = reclassify-income
  modal Enter; `persist_set_fmv` = set-fmv modal Enter; `persist_void` = void modal Enter.
  Verified by grep + KAT-G1.
- **Dispatch order [N4]:** six modal layers (1–6) → flow layer → form → screen; each modal check
  precedes the flow layer; `q` swallowed by every modal and every flow step at every step; Esc
  steps back exactly one step at every flow step [I4] (KAT-C2c/C2d/C2e pin the sequences).
- **Pre-filter correctness:**
  - reclassify-income: no already-reclassified event in list (Claim C); WB-I4(a) deferred.
  - set-fmv: only FmvMissing-blocked Income events; no pre-filter for already-set FMVs (Claim D).
  - void: revocable types only; non-revocable excluded; already-voided excluded (Claim E);
    the [M3] rejected-SafeHarbor-void list gap is documented (spec + FOLLOWUPS) and the modal
    warning carries the permanence line.
- **`now` injection:** all three persist fns receive `now` from the Enter-press handler.
- **Retry semantics pinned:**
  - reclassify-income: FIRST-WINS, conflict fires on retry (D4 doc comment); covered by the
    generic S2 pattern (sampling justification in KAT-S2b).
  - set-fmv: LATEST-WINS, no conflict; KAT-S2b + KAT-E2E-FMV-REPOINT pin this explicitly.
  - void: idempotent re-void, no conflict, save-error leaves the flow at List [M1];
    KAT-VOID-RETRY pins the +2-inert-rows contract.
- **Cascade honesty [I1]:** the void modal's consequence note names the dependent-decision
  cascade; KAT-E2E-VOID-CASCADE pins the orphan conflict + in-list remedy; the D3.1 surfacing
  limit of the clean arm is stated (cascade conflicts are attributed to the orphan's id).
- **Void conflict-arm honesty [I2]:** the rejected-void status never claims "Voided";
  KAT-VOID-CONFLICT-ARM pins the wording.
- **Remedy strings:** all four arms updated (D3.2); KAT-RS-1..4 pin both `"'v'"` and
  `"btctax reconcile void"`; the five existing E2E/S2 pins strengthened in place, none deleted
  [M2].
- **Void LotSelection side-effect:** `persist_void` calls `optimize_attest::clear` before `save`
  for LotSelection targets; verified by grep + unit test in persist.rs (not a KAT-E2E, just a
  direct fn call on a seeded vault with a LotSelection decision).
- **No tax-figure claims in status:** all `derive_*_status` fns are blocker-derived; no computed
  SE/NIIT amounts appear in any status string; KAT-E2E-RI-SE verifies the status is the CLEAN
  string (not a figure).
- **Viewer untouched:** no viewer files change; E10 gate continues to pass.

FOLLOWUPS to record for chunk 2b:
- **WB-I4(a) deferred:** raw-vs-effective under-inclusion for BOTH reclassify-income and set-fmv
  filters (same as the classify-inbound deferred item; ClassifyRaw'd Income events invisible). The
  cheap fix is also treating as Income any event targeted by a non-voided `ClassifyRaw` whose
  `as_` payload is `Income`. Deferred to chunk 3+.
- **Chunk 3+:** `link-transfer`, `safe-harbor-allocate`, `select-lots`, `set-donation-details`,
  `classify-raw`, `optimize-accept`. The link flow must use the disposals/removals presence check
  for the link-vs-reclassify precedence case [2a R0-M1].
- **SafeHarborAllocation void conditional behavior:** the void-modal warning note is specified
  (D3.1) and the conflict-arm STRING is covered by KAT-VOID-CONFLICT-ARM [I2], but an E2E for
  the effective-allocation conflict case would require a valid Path-B vault fixture (complex).
  Deferred to chunk 3+ when SafeHarborAllocation is better tested.
- **[M3] Rejected-SafeHarbor-void list gap:** a rejected void permanently removes an IN-FORCE
  effective allocation from the TUI void list (the pre-filter cannot distinguish applied from
  rejected voids without duplicating resolve's step-3 adjudication). Acceptable (re-voiding only
  re-fires the conflict; CLI void remains available); refine only if a real user hits it.
- **[I1] Generic cascade detection:** a whole-blockers diff (pre-void vs post-void) in the void
  Enter-arm could surface cascade conflicts generically (they are attributed to the orphaned
  decision's id, invisible to `derive_void_status`). Deferred as disproportionate for 2b — the
  modal note + Compliance tab + the orphan's own `v`-list entry carry the remedy today.
- **[R0-N3] Negative-sign parity:** same carryforward from 2a; usd_fmv accepts negatives on
  both surfaces (CLI parity); tightening must land on both together.

---

## Out of scope

- **Chunk 3+:** `link-transfer`, `safe-harbor-allocate`, `select-lots`, `set-donation-details`,
  `classify-raw`, `accept-conflict`, `reject-conflict`, `optimize-accept`.
- **Viewer changes:** frozen (E10 gate, write-free guarantee).
- **`btctax-core` or `btctax-cli` changes:** no new core types, no new CLI commands. All three
  payload types (`ReclassifyIncome`, `ManualFmv`, `VoidDecisionEvent`) are existing variants.
- **`set-donation-details`:** side-table write (no event append); out of scope for chunk 2b.
- **Batch void / multi-select:** single-selection only (one void per flow open), matching the
  2a single-selection precedent.
- **Negative-sign validation tightening:** parity-preserving carryforward (FOLLOWUPS only).
