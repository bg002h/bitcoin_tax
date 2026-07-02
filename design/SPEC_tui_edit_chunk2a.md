# SPEC — btctax-tui-edit chunk 2a: classify-inbound + reclassify-outflow

**Source baseline:** `main` @ `096a07b` (working tree re-verified file-by-file at write time; all
line citations below checked against the current source).
**Review status:** R0 round 1 (`reviews/R0-spec-tui-edit-chunk2a-round-1.md`) FOLDED — findings
tagged `[R0-…]` inline; re-review required per §2 of `STANDARD_WORKFLOW.md`.
**Goal:** Chunk 2a of the mutating-TUI program: two new decision flows added to the existing
`btctax-tui-edit` crate —

1. **classify-inbound** — classifies an unclassified `TransferIn` as Income (with `IncomeKind`
   sub-picker, optional FMV, and a `business` toggle) or as a received Gift (with REQUIRED
   `fmv_at_gift`, optional `donor_basis`, optional `donor_acquired_at`). Appends
   `EventPayload::ClassifyInbound{transfer_in_event, as_: InboundClass::Income{…}|
   InboundClass::GiftReceived{…}}`.
2. **reclassify-outflow** — classifies a pending (unreconciled) `TransferOut` as a Sell, Spend,
   GiftOut, or Donation. Appends
   `EventPayload::ReclassifyOutflow{transfer_out_event, as_, principal_proceeds_or_fmv, fee_usd,
   donee}`.

Both flows follow the chunk-1 structural substrate verbatim: a selectable target LIST, a
variant/kind PICKER, per-variant field forms with capped buffers, a payload-showing confirmation
modal, `edit/persist.rs` writers that call `append_decision` + `session.save()`, Enter-arm
re-projection, and the chunk-1 failed-save M1 semantics (with the chunk-2a retry consequence
stated truthfully — [R0-I1], Hard constraints below).

**SemVer:** Changes are confined to the existing `btctax-tui-edit` crate (no new workspace
member, no new `[lib]` targets). Two new `pub fn` items in `edit/persist.rs` that name
`append_decision` — already allowlisted in KAT-G1. **MINOR** (pre-1.0; additive).

---

## Hard constraints (load-bearing — chunk-1 guarantee inherited verbatim)

The editor's crate-level guarantee is **unchanged by this spec**:

> "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`, each behind
> an explicit payload-showing confirmation; the vault file only via `Vault::save`'s atomic path."

Chunk-2a instantiation of the guarantee:
- `persist_classify_inbound` and `persist_reclassify_outflow` are the ONLY two new writers;
  both live exclusively in `edit/persist.rs` (already the sole allowlisted module in KAT-G1).
- Each calls `append_decision(conn, payload, now, UtcOffset::UTC, None)` followed by
  `session.save()`, mirroring `cmd::reconcile::append_and_save` (reconcile.rs:26–34) but against
  the HELD session (same live-session rationale as chunk 1, D3 of `SPEC_tui_edit_chunk1.md`).
- `now: OffsetDateTime` is INJECTED at Enter-press (not `OffsetDateTime::now_utc()` inline) for
  test determinism — the same discipline as the CLI's `now` parameter.
- No gate change is required: the `append_` token is ALREADY in the KAT-G1 persist-only
  allowlist (persist.rs:235 — `"append_"` in `persist_only_tokens`). **[Gate: VERIFIED
  unchanged.]**
- Per-mutation confirmation: the modal precedes ALL writes; Esc cancels with nothing written.

**Failed-save + retry semantics [R0-I1] (the chunk-1 M1 rule, with the chunk-2a consequence
stated truthfully).** When `append_decision` succeeds but `save()` fails, the HELD in-memory
session ALREADY carries the committed decision (say seq N+1) while the on-disk vault remains the
pre-action state (the atomic path leaves the old image). This divergence is intentional — do NOT
roll back (inherited verbatim from chunk 1 [R0-M1]). The chunk-2a-specific consequence, which
differs from chunk 1's idempotent upsert:

- A retry (re-confirm) calls `append_decision` AGAIN → decision N+2, an **identical-payload
  DUPLICATE** of N+1. Resolve is **FIRST-WINS** for both `ClassifyInbound` (resolve.rs:549–564)
  and `ReclassifyOutflow` (resolve.rs:600–617): the FIRST decision (N+1 — the failed-save one)
  stays **in force**; the duplicate N+2 is EXCLUDED from the classification maps **and fires a
  Hard `DecisionConflict`** (severity Hard, state.rs:65–77), which gates `compute_tax_year` via
  `TaxYearNotComputable`.
- Because both payloads are identical, the *classification content* projects the same either
  way — but the projection is NOT clean: the user is left with a Hard blocker. The post-persist
  status MUST surface it (D4 step 2's re-projected-blockers check covers this by construction).
- Clearing path in chunk 2a: the CLI — `btctax reconcile void decision|<N+2>` (the TUI void flow
  is chunk 2b).
- A successful retry save persists BOTH decisions: the on-disk log grows by **2** rows relative
  to the pre-action state (KAT-S2 asserts this exact outcome).
- Quitting without any successful save loses both in-memory decisions — exactly the
  save-per-action contract (nothing is durable until `Vault::save` returns `Ok`).

These consequences are stated so no implementer "fixes" them (no rollback, no dedup-on-retry)
and no later reviewer calls the duplicate a leak.

**Chunk-1 substrate items cited throughout this spec** (all in `SPEC_tui_edit_chunk1.md`):
- `UnlockState` push/pop buffer discipline (chunk 1, D4 `FieldBuffer`) — reused verbatim.
- `FilingStatus` Tab-cycle precedent (chunk 1, D4 form keys) — the basis for variant/kind
  pickers in D2 and D3.
- `MutationModalState` + `draw_mutation_modal` pattern (chunk 1, D4 modal; draw_edit.rs:232–293)
  — the structural template for the two new flow modals.
- `persist_tax_profile` pattern (chunk 1, D3) — the structural template for D4's new persist fns.
- `load_all_ordered` / `RawEventRow` (chunk 1, D5; ordinal included per the N6 amendment,
  persistence.rs:334–380) — the strict-prefix test skeleton reused by KAT-P2a and KAT-P2b.
- Enter-arm semantics (chunk 1, D4 modal keys; main.rs:168–198): re-project on Ok, keep form +
  set error on Err.
- Dispatch order: modal → form → screen (chunk 1, main.rs:79–88) — EXTENDED by this spec to
  modal → **flow** → form → screen [R0-I2] (D1).

---

## Pre-filter verification (recon-verified, re-verified at write time; R0-confirmed)

### Claim A (inbounds): "the `UnknownBasisInbound` blocker only exists while unclassified"

**Verdict: QUALIFIED — the compound filter below is required and is R0-confirmed sound against
all four fire sources.** The `UnknownBasisInbound` blocker fires from four code paths:

1. `Op::UnknownInbound` (fold.rs:815–822): the TransferIn has NO non-voided
   `ClassifyInbound` decision → this IS the actionable case for `classify-inbound`.
2. `Op::GiftReceived` with `donor_basis=None, donor_acquired_at=None` (fold.rs:929–935):
   the TransferIn HAS a `ClassifyInbound::GiftReceived` decision but both donor fields are
   absent → the blocker's `event` field still points to the TransferIn EventId; a SECOND
   `ClassifyInbound` targeting the same TransferIn would fire `DecisionConflict`
   (resolve.rs:554–561: "duplicate ClassifyInbound for the same TransferIn event"; FIRST-WINS).
3. `Op::GiftReceived` with `donor_acquired_at=Some(d)` but BTC price unavailable at `d`
   (fold.rs:913–927): same situation — classification exists, `UnknownBasisInbound` still fires.
4. A removal consuming a basis-pending lot (fold.rs:230–237): the blocker's `event` field is
   the gift/donation (removal) event, NOT a TransferIn → filtered out by the payload-type check.

**Required compound pre-filter for the classify-inbound list:**
```
snap.state.blockers
  .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
  .filter(|b| b.event.as_ref().map_or(false, |id| {
      snap.events.iter().any(|e| &e.id == id && matches!(e.payload, EventPayload::TransferIn(_)))
  }))
  .filter(|b| {
      // Exclude already-classified TransferIns (paths 2 and 3 above):
      // A non-voided ClassifyInbound decision in snap.events targeting this TransferIn
      // would produce DecisionConflict — exclude from the actionable list.
      let target_id = b.event.as_ref().unwrap();
      let voided: std::collections::BTreeSet<EventId> = snap.events.iter()
          .filter_map(|e| if let EventPayload::VoidDecisionEvent(v) = &e.payload { Some(v.target_event_id.clone()) } else { None })
          .collect();
      !snap.events.iter().any(|e| {
          if voided.contains(&e.id) { return false; }
          if let EventPayload::ClassifyInbound(ci) = &e.payload { &ci.transfer_in_event == target_id } else { false }
      })
  })
```

R0 adversarial checks (all confirmed): a TransferIn whose only `ClassifyInbound` is voided
stays listed (resolve skips voided decisions, resolve.rs:487, and the filter's `voided` set
mirrors that); a TransferIn consumed by a `TransferLink` resolves to `Op::Skip` and never fires
the blocker; no over-inclusion path exists — the filter can never offer a target whose
`ClassifyInbound` would conflict.

**Known under-inclusion [R0-M2] (raw vs. effective payload — safe direction, documented
limitation):** filter 2 above matches the RAW payload in `snap.events`, while resolve validates
the EFFECTIVE payload (`applied.get(target).unwrap_or(&raw.payload)`, resolve.rs:531–533). An
event whose effective payload became `TransferIn` via a non-voided `ClassifyRaw` (or an accepted
`ImportConflict` supersede) fires `Op::UnknownInbound` and is CLI-classifiable, but is invisible
in the TUI list. This is under-inclusion — the safe direction (never a conflict) — and rare.
Interim path: the CLI (`btctax reconcile classify-inbound-income/-gift`). Recorded in FOLLOWUPS;
the cheap in-TUI fix (also treat as TransferIn any event targeted by a non-voided
`ClassifyRaw{as_: TransferIn(_)}`) is deferred there.

**Consequence for UX honesty [R0-I4]:** the incomplete-gift cases (paths 2 and 3) are NOT
surfaced in the classify-inbound list (a second `ClassifyInbound` would conflict). The ONLY
valid remedy for those cases today is **void the existing `ClassifyInbound`, then re-classify**
— in chunk 2a that void is the CLI: `btctax reconcile void decision|<seq>` (the TUI void flow is
chunk 2b). Neither `set-fmv` nor a bare re-classify works: `ManualFmv` validates its target as
an **Income event** — a TransferIn target fires "ManualFmv targets non-Income event" → Hard
`DecisionConflict`, decision excluded (resolve.rs:423–470), and `build_op`'s TransferIn arm
never consults `manual_fmv` (resolve.rs:251–281); a bare re-classify is a duplicate
`ClassifyInbound` → Hard `DecisionConflict` (resolve.rs:554–561). The D2 status strings name
the void-then-re-classify remedy exactly (no blocker-creating suggestions).

### Claim B (outflows): "pending_reconciliation is inherently post-filter for outflows"

**Verdict: CONFIRMED (R0-verified).** From resolve.rs:201–250 (`build_op` for `TransferOut`): a
TransferOut with a non-voided `ReclassifyOutflow` decision routes to `Op::GiftOut / Op::Donate /
Op::Dispose` (never `Op::PendingOut`). A TransferOut with a non-voided `TransferLink` routes to
`Op::SelfTransfer`. Only the residual (neither `outflow_class` nor `links` entry for the out
event) falls through to `Op::PendingOut`, which is the sole site pushing into
`pending_reconciliation` (fold.rs:729–734). **`pending_reconciliation` by construction contains
ONLY unreclassified, unlinked TransferOuts.** No additional client-side filter is required. The
advisory `UnmatchedOutflows` blocker fires only in the same arm (fold.rs:736–740), so it clears
together with the pending entry after a reclassify.

---

## Current state (recon @ `096a07b` — chunk 1 is shipped)

All chunk-1 infrastructure is live and tested:

- **`crates/btctax-tui-edit/src/edit/persist.rs`** — contains `persist_tax_profile`; the
  `append_decision` call is in the doc-comment but NOT yet implemented (chunk 1 is side-table
  only). KAT-G1 (`kat_g1_mechanized_source_gate`) is green. The `append_` token is already in
  the KAT-G1 `persist_only_tokens` allowlist (persist.rs:235) — ready for chunk-2a writers.
- **`crates/btctax-tui-edit/src/edit/form.rs`** — `FieldBuffer` (push/pop/is_empty, FIELD_CAP=64,
  form.rs:13–57), `ProfileFormState`, `MutationModalState`, `validate`, `cycle_filing_status`.
  The `FieldBuffer` + `FIELD_CAP` infrastructure is reused verbatim by the chunk-2a field forms.
- **`crates/btctax-tui-edit/src/draw_edit.rs:232–293`** — `draw_mutation_modal` (the
  payload-showing modal pattern); `centered_rect` helper (draw_edit.rs:296–305).
- **`crates/btctax-tui-edit/src/main.rs`** — `handle_key` with the 3-level dispatch order
  (modal → form → screen, main.rs:79–88); `handle_modal_key` (main.rs:146–208); `handle_form_key`
  (main.rs:210–…); `p` key opens the profile form. Chunk-2a adds `c` and `o` key bindings to
  open classify-inbound and reclassify-outflow flows respectively (no-op when
  `snapshot.is_none()`), and inserts the flow dispatch layer [R0-I2].
- **`crates/btctax-tui-edit/src/main.rs:972–1096`** — chunk-1 KAT-C1 (the cancel-path
  vault-bytes-unchanged test) [R0-M5 citation fix]; **main.rs:1100–1123** — chunk-1 KAT-S1
  (the chmod-0o500 save-error test, including its root-skip guard [R0-N5]).
- **`crates/btctax-tui-edit/src/editor.rs`** — `EditorApp` holds `profile_form:
  Option<ProfileFormState>` and `mutation_modal: Option<MutationModalState>`. Chunk-2a adds two
  flow-state fields and two modal fields (D1, D2, D3) — the flows OWN their lists [R0-I2].
- **`crates/btctax-core/src/persistence.rs:334–380`** — `load_all_ordered` returns
  `Vec<RawEventRow>` in ordinal order; `RawEventRow` carries `ordinal` (amendment N6 already
  shipped). The strict-prefix test skeleton (KAT-P1) is in `edit/persist.rs` tests; chunk-2a
  KAT-P2a/P2b reuse it.
- **`crates/btctax-core/src/persistence.rs:238–262`** — `append_decision`: the sole decision
  writer; allocates `decision_seq` via `COALESCE(MAX(decision_seq),0)+1` over ALL decision rows
  (persistence.rs:246–250 — the KAT expectation formula mirrors this MAX semantics [R0-I6]).
- **`crates/btctax-core/src/event.rs:104–139`** — `ReclassifyOutflow`, `OutflowClass`
  (`Dispose{kind} / GiftOut / Donate{appraisal_required}`), `InboundClass`
  (`Income{kind,fmv,business} / GiftReceived{donor_basis, donor_acquired_at, fmv_at_gift}`),
  `ClassifyInbound`. These are the exact payload types written by the two new flows.
  `Dispose.usd_proceeds` is GROSS proceeds for BOTH Sell and Spend (event.rs:62) [R0-I3].
- **`crates/btctax-core/src/state.rs:197–226`** — `PendingTransfer {event, principal_sat,
  fee_sat, legs}` and `LedgerState {pending_reconciliation, blockers}`. The two list sources.
- **`crates/btctax-tui/src/app.rs:104–111`** — `Snapshot {events: Vec<LedgerEvent>, state:
  LedgerState, …}`. `snap.events` is the raw event log (used for event lookup by both lists).
- **`crates/btctax-cli/src/eventref.rs:76–83`** — `parse_usd_arg`
  (`Decimal::from_str(s.trim())`) and `parse_date_arg` (`Date::parse(s.trim(),
  "[year]-[month]-[day]")`). These are the exact parse semantics for all monetary and date
  fields in the chunk-2a forms.

---

## Design

### D1 — the target list widget (new shared infrastructure) + the flow dispatch layer

Both flows begin with a **selectable list** of actionable targets rendered as a `Table` widget.
The list uses `ratatui::widgets::TableState` (the same type as `holdings_state`,
`disposals_state` etc. in `EditorApp`) to track the highlighted row and support `↑/↓`/`j/k`
scroll, `g/G` jump to top/bottom, and `Enter` to select.

**`TargetList<T>`** [R0-N1: named to avoid colliding with `ratatui::widgets::ListState`]
(new generic struct in `edit/form.rs` or a new `edit/list.rs`):

```rust
pub struct TargetList<T> {
    pub items: Vec<T>,       // pre-filtered, pre-computed display data; non-empty by contract
    pub table_state: TableState,
}

impl<T> TargetList<T> {
    /// Callers (D2/D3 flow-open) guarantee `items` is non-empty — an empty
    /// filtered list never opens a flow [R0-M8]. The render keeps a defensive
    /// "no items" row + Enter-swallow ONLY as a belt-and-suspenders path; it is
    /// unreachable under the D2/D3 open rule and carries no KAT.
    pub fn new(items: Vec<T>) -> Self {
        let mut table_state = TableState::default();
        if !items.is_empty() { table_state.select(Some(0)); }
        Self { items, table_state }
    }
    pub fn selected(&self) -> Option<&T> {
        self.table_state.selected().and_then(|i| self.items.get(i))
    }
    pub fn scroll_up(&mut self) { … }
    pub fn scroll_down(&mut self) { … }
    pub fn go_top(&mut self) { … }
    pub fn go_bottom(&mut self) { … }
}
```

`scroll_up/down/go_top/go_bottom` follow the same bounded clamping as `EditorApp`'s existing
scroll helpers in `main.rs`.

**Flow-layer dispatch [R0-I2] (normative — the R0-M4 bug class is designed out here).**
`EditorApp` gains exactly TWO flow fields and TWO modal fields (D2/D3 define the types):

```rust
pub classify_inbound_flow: Option<ClassifyInboundFlowState>,      // OWNS its TargetList
pub reclassify_outflow_flow: Option<ReclassifyOutflowFlowState>,  // OWNS its TargetList
pub classify_inbound_modal: Option<ClassifyInboundModalState>,
pub reclassify_outflow_modal: Option<ReclassifyOutflowModalState>,
```

There are **no standalone list fields on `EditorApp`** — each flow struct owns its list, so
list-state and step-state can never disagree. **State invariant (stated once, here):** at most
one flow is `Some` at any time, and at most one modal (of the three: `mutation_modal`,
`classify_inbound_modal`, `reclassify_outflow_modal`) is `Some` at any time.

**Dispatch order (extends chunk 1's modal → form → screen):**

1. **Modal layer** — any of the three modals `Some` → `handle_*_modal_key`; all unmatched keys
   swallowed (blocking).
2. **Flow layer** — `classify_inbound_flow.is_some() || reclassify_outflow_flow.is_some()` →
   the flow's key handler, which dispatches on the flow's CURRENT STEP (List / picker / field
   form). The guard is the flow `Option`, NOT the step — so EVERY step of an open flow is
   claimed by this layer and `q`/`Esc` can never fall through to the Browse quit arm mid-flow
   (the R0-M4 lesson). `q` is SWALLOWED at every flow step; `Esc` steps BACK one step (field
   form → picker → list → close flow), never quits.
3. **Form layer** — `profile_form.is_some()` (chunk 1, unchanged).
4. **Screen dispatch** (Unlock / Locked / Browse; unchanged).

**List-step keys** (inside the flow layer, when the flow's step is `List`):

| Key | Action |
|---|---|
| `↑` / `k` | `list.scroll_up()` |
| `↓` / `j` | `list.scroll_down()` |
| `g` | `list.go_top()` |
| `G` | `list.go_bottom()` |
| `Enter` | select highlighted item → transition to the picker step |
| `Esc` | close the flow → back to Browse (nothing written) |
| `q` | SWALLOWED (flow is blocking) |

**Display data types** (pre-computed at flow open from the snapshot; owned by the flow's list):

```rust
pub struct InboundListItem {
    pub blocker_event: EventId,       // Blocker.event (the TransferIn EventId)
    pub date: TaxDate,                // tax_date(e.utc_timestamp, e.original_tz)
    pub sat: Sat,                     // TransferIn.sat from the event payload
    pub wallet: Option<WalletId>,     // e.wallet from snap.events lookup
    pub detail: String,               // Blocker.detail
}

pub struct OutflowListItem {
    pub transfer_out_event: EventId,  // PendingTransfer.event
    pub date: TaxDate,
    pub principal_sat: Sat,           // PendingTransfer.principal_sat
    pub wallet: Option<WalletId>,
}
```

Both list items derive the date and wallet via a linear scan of `snap.events` indexed by
`EventId` (O(n); flow open is a one-time cost, not a hot path). Implement as a helper:
`fn events_by_id(snap: &Snapshot) -> BTreeMap<&EventId, &LedgerEvent>`. Events without a wallet
display `"(no wallet)"`.

---

### D2 — classify-inbound flow

**Key binding:** `c` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (target list).** Build the `TargetList<InboundListItem>` from the snapshot
using the compound pre-filter described in the Pre-filter verification section:
- Filter `snap.state.blockers` by `kind == BlockerKind::UnknownBasisInbound`.
- Keep only those where `Blocker.event` resolves to a `TransferIn` event in `snap.events`
  (raw payload; the [R0-M2] effective-payload limitation is documented above + FOLLOWUPS).
- Exclude TransferIn events already targeted by a non-voided `ClassifyInbound` decision in
  `snap.events`.
- For each surviving blocker: construct `InboundListItem` from the event lookup.

If the filtered list is empty, show a status "No unclassified inbound transfers" and return to
Browse **without opening the flow** [R0-M8 — this is the single normative empty-list rule; the
widget's defensive placeholder is unreachable].

Render the list as a `Table` with columns: `Date | Sat | Wallet | EventId (canonical)`. The
`[EDITOR]` badge and footer keybindings row are shown throughout. Title block:
`" Classify Inbound — select TransferIn target "`.

**Step 2 — variant picker.** After `Enter` on a list item, the flow transitions to the
**variant picker** step. The picker shows two choices: `Income` and `GiftReceived`, cycled via
`Tab` (the FilingStatus Tab-cycle precedent from chunk 1, D4). The initial selection is
`Income`. Keys:

| Key | Action |
|---|---|
| `Tab` | cycle Income ↔ GiftReceived |
| `Enter` | confirm variant → transition to field form |
| `Esc` | back to the target list |
| `q` | SWALLOWED |

**Step 3 — field form.** Per-variant fields using `FieldBuffer` (FIELD_CAP=64; push/pop/is_empty
exactly as in chunk 1):

**Income fields:**

| Field | Type | Required? | Validation |
|---|---|---|---|
| `kind` | `IncomeKind` picker | always | Tab-cycles: Mining → Staking → Interest → Airdrop → Reward → Mining (enum declaration order, event.rs:29–35). **Initial selection: Mining** [R0-M3]. Structurally always valid. |
| `fmv` | `FieldBuffer` | OPTIONAL | empty → `None`; non-empty → `parse_usd_arg(trim)` |
| `business` | toggle bool | always (default false — CLI parity, main.rs:224–225) | `Space` toggles; always valid |

Focus order: kind (row 0, Tab-cycles the 5 `IncomeKind` variants) → fmv (row 1, text input) →
business (row 2, Space toggle). `↑/↓` move focus. Tab on row 0 cycles IncomeKind; on rows 1–2
moves focus down. Enter validates + opens modal. Esc → back to variant picker.

**GiftReceived fields:**

| Field | Type | Required? | Validation |
|---|---|---|---|
| `fmv_at_gift` | `FieldBuffer` | **REQUIRED** | empty → error "fmv-at-gift is required"; non-empty → `parse_usd_arg(trim)` |
| `donor_basis` | `FieldBuffer` | optional | empty → `None`; non-empty → `parse_usd_arg(trim)` |
| `donor_acquired_at` | `FieldBuffer` | optional | empty → `None`; non-empty → `parse_date_arg(trim)` (YYYY-MM-DD) |

Focus order: fmv_at_gift (row 0) → donor_basis (row 1) → donor_acquired_at (row 2). Enter
validates. Esc → back to variant picker.

**Flow state struct** (in `edit/form.rs`):

```rust
pub enum ClassifyInboundStep {
    List,
    VariantPicker {
        item: InboundListItem,
        variant: InboundVariant,  // Income | GiftReceived
    },
    IncomeForm {
        item: InboundListItem,
        kind: IncomeKind,         // initial: Mining [R0-M3]
        fmv_buf: FieldBuffer,
        business: bool,
        focus: usize,
        error: Option<String>,
    },
    GiftForm {
        item: InboundListItem,
        fmv_at_gift_buf: FieldBuffer,
        donor_basis_buf: FieldBuffer,
        donor_acquired_at_buf: FieldBuffer,
        focus: usize,
        error: Option<String>,
    },
}

pub struct ClassifyInboundFlowState {
    pub list: TargetList<InboundListItem>,   // OWNED by the flow [R0-I2]
    pub step: ClassifyInboundStep,
}
```

**Validation** (pure fn, validate-at-submit):

- Income: `kind` always valid (picker); `fmv` = `None` if empty else `parse_usd_arg`; `business`
  already bool. No required field can be missing. Builds
  `InboundClass::Income { kind, fmv, business }`.
- GiftReceived: `fmv_at_gift` REQUIRED (empty → error); `donor_basis` optional;
  `donor_acquired_at` optional. Builds
  `InboundClass::GiftReceived { donor_basis, donor_acquired_at, fmv_at_gift }`.

**Post-effect status — derived from the RE-PROJECTED state, not the payload shape [R0-I5].**
After a confirmed save + `build_snapshot`, the Enter-arm inspects the NEW
`snap.state.blockers` and derives the status (D4 step 2 defines the uniform check):

- **No target-attributed blocker remains** → clean success:
  `"Classified inbound as Income ({kind})"` / `"Classified inbound as GiftReceived"`. This is
  the Income-with-FMV and gift-with-donor-info happy path (blocker clears; a lot + income
  record / a gift lot appear).
- **`FmvMissing` present with `event == target`** (the Income-fmv-None case, fold.rs:853–859:
  the lot IS created with `basis_pending=true`) → status:
  `"Classified as Income ({kind}) but FMV missing — FmvMissing blocker fired; to supply the FMV, void this decision (CLI: btctax reconcile void decision|{seq}) and re-classify with an FMV"`
  [R0-I4: no set-fmv suggestion — `ManualFmv` on a TransferIn is rejected with a Hard
  `DecisionConflict` (resolve.rs:423–470); no bare re-classify — that is a duplicate → Hard
  `DecisionConflict` (resolve.rs:554–561)].
- **`UnknownBasisInbound` present with `event == target`** (fires for BOTH gift case 4 —
  both donor fields None, fold.rs:929–935 — AND gift case 3 — donor date given but price
  unavailable at that date, fold.rs:913–927 [R0-I5]) → status:
  `"Gift recorded but basis unknown — UnknownBasisInbound re-fired; void this decision (CLI: btctax reconcile void decision|{seq}) and re-classify with donor basis or a donor date covered by the price dataset"`.
  This is expected and not a bug — the Compliance tab surfaces the re-fired blocker.
  **[Pre-filter note: the re-fired blocker will NOT appear in the classify-inbound list (the
  TransferIn now has a ClassifyInbound decision → pre-filtered out). This is correct; resolution
  is void-then-re-classify via the CLI until chunk 2b's void flow lands [R0-M4].]**
- **`DecisionConflict` attributed to the newly appended decision id** (the failed-save-retry
  duplicate case [R0-I1], or any stale edge) → status:
  `"Saved, but DecisionConflict fired on this decision — see Compliance; clear with CLI: btctax reconcile void decision|{seq}"`.

The gift form's both-donor-fields-empty condition ALSO gets an at-write-time warning line in
the modal (below) — the status is the post-write honesty; the modal line is the pre-write one.

**Modal content — classify-inbound:**

`ClassifyInboundModalState` (new; parallel to chunk-1 `MutationModalState`):

```rust
pub struct ClassifyInboundModalState {
    pub target_event: EventId,       // the TransferIn's EventId (displayed canonical)
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub as_: InboundClass,           // the VALIDATED classification payload
}
```

Modal rendering (draw_edit.rs, structural clone of `draw_mutation_modal` at lines 232–293):

```
╔═ Confirm: classify-inbound — WRITES THE VAULT ══════════╗
║  target:  import|river|…  (TransferIn)                  ║
║  date:    2025-03-15                                     ║
║  sat:     500000                                         ║
║                                                          ║
║  as: Income                                              ║
║    kind:     staking                                     ║
║    fmv:      45.50         (empty = FmvMissing will fire)║
║    business: false                                       ║
║                                                          ║
║  Appended as a decision event (append-only log).         ║
║  Saved immediately via the vault's atomic write path.    ║
║                                                          ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

For GiftReceived:
```
║  as: GiftReceived                                        ║
║    fmv_at_gift:       500.00   (REQUIRED)                ║
║    donor_basis:       400.00   (or empty = unknown)      ║
║    donor_acquired_at: 2022-04-01 (or empty = unknown)    ║
║                                                          ║
║  WARNING: both donor fields empty → UnknownBasisInbound  ║
║  will re-fire after classification.                      ║
```
(The warning line is shown only when both `donor_basis` and `donor_acquired_at` are `None`.)

Modal keys: Enter → `persist_classify_inbound`; Esc → close modal only (back to field form);
any other key swallowed. Dispatch: modal check precedes all other dispatch.

**Enter-arm semantics (chunk 1's [R0-M1] pattern + the D4 step-2 status check):**
- `Ok(id)` [R0-N2 — the persist fns return the new `EventId`] → re-project via
  `build_snapshot(session)`, run the D4 step-2 blocker check to derive the status, close modal
  + flow.
- `Err(e)` → close modal, keep field form open (buffers intact), set
  `status = "Save error: {e}"`. Vault unchanged on disk. No re-projection. (The in-memory
  session carries the committed decision — see the [R0-I1] retry semantics in Hard constraints.)

---

### D3 — reclassify-outflow flow

**Key binding:** `o` in the Browse screen (no-op when `snapshot.is_none()`).

**Step 1 — flow open (target list).** Build `TargetList<OutflowListItem>` from
`snap.state.pending_reconciliation` (no additional client-side filter required — Claim B
verified):

- For each `pt: PendingTransfer` in `snap.state.pending_reconciliation`: construct
  `OutflowListItem` from `pt.event`, `pt.principal_sat`, and the event lookup in `snap.events`
  for date and wallet.

If the list is empty, show status "No pending outbound transfers" and return to Browse without
opening the flow [R0-M8].

Render as a `Table` with columns: `Date | Principal Sat | Wallet | EventId (canonical)`.
Title block: `" Reclassify Outflow — select pending TransferOut target "`.

**Step 2 — kind picker.** After `Enter` on a list item, a kind picker shows four choices cycled
via `Tab`:

| Display | Maps to |
|---|---|
| `sell` | `OutflowClass::Dispose { kind: DisposeKind::Sell }` |
| `spend` | `OutflowClass::Dispose { kind: DisposeKind::Spend }` |
| `gift` | `OutflowClass::GiftOut` |
| `donate` | `OutflowClass::Donate { appraisal_required: false }` (appraisal toggled in next step) |

Initial selection: `sell`. Tab cycles sell → spend → gift → donate → sell. Enter confirms kind →
transitions to field form. Esc → back to target list.

**Step 3 — field form.** Fields are **kind-uniform** (the same fields are present for all kinds;
the label and required-ness differ):

| Field | Type | Required? | Applicable kinds | Label |
|---|---|---|---|---|
| `amount` | `FieldBuffer` | **REQUIRED** | all | **"gross proceeds (USD)" for sell AND spend; "FMV (USD)" for gift/donate** [R0-I3 — `Dispose.usd_proceeds` is GROSS for both Sell and Spend, event.rs:62; reconcile.rs:55–57] |
| `fee` | `FieldBuffer` | optional | all | "fee (USD, optional)" |
| `appraisal` | toggle bool | (default false) | donate only | "appraisal required" (Space toggles) — hidden/greyed for sell/spend/gift |
| `donee` | `FieldBuffer` | optional | gift, donate | "donee (free-form, optional)" — hidden for sell/spend |

Focus order: amount (row 0) → fee (row 1) → appraisal (row 2, donate only) → donee (row 3,
gift/donate only). Hidden rows are skipped in focus cycling. Enter validates. Esc → back to
kind picker.

**Validation** (pure fn, validate-at-submit):

- `amount`: REQUIRED; empty → error "amount is required"; non-empty → `parse_usd_arg(trim)`.
  The label on the field changes by kind (see table above); the validation rule is the same.
- `fee`: optional; empty → `None`; non-empty → `parse_usd_arg(trim)`.
- `appraisal`: toggle bool; always valid.
- `donee`: optional free-form text; empty → `None`; non-empty → `Some(buf.trim().to_owned())`.
  Cap: FIELD_CAP (64 bytes). [R0-N4] The TUI trims and caps `donee`; the CLI passes it
  untrimmed and unbounded — a harmless, deliberate divergence (the TUI buffer discipline),
  recorded here so no reviewer calls it accidental.

Build payload:
```rust
ReclassifyOutflow {
    transfer_out_event: item.transfer_out_event.clone(),
    as_: match kind {
        KindSell  => OutflowClass::Dispose { kind: DisposeKind::Sell },
        KindSpend => OutflowClass::Dispose { kind: DisposeKind::Spend },
        KindGift  => OutflowClass::GiftOut,
        KindDonate => OutflowClass::Donate { appraisal_required: appraisal },
    },
    principal_proceeds_or_fmv: amount,
    fee_usd: fee,
    donee,
}
```

**Flow state struct** (in `edit/form.rs`):

```rust
pub enum OutflowKind { Sell, Spend, Gift, Donate }

pub enum ReclassifyOutflowStep {
    List,
    KindPicker {
        item: OutflowListItem,
        kind: OutflowKind,        // initial: Sell
    },
    FieldForm {
        item: OutflowListItem,
        kind: OutflowKind,
        amount_buf: FieldBuffer,
        fee_buf: FieldBuffer,
        appraisal: bool,
        donee_buf: FieldBuffer,
        focus: usize,
        error: Option<String>,
    },
}

pub struct ReclassifyOutflowFlowState {
    pub list: TargetList<OutflowListItem>,   // OWNED by the flow [R0-I2]
    pub step: ReclassifyOutflowStep,
}
```

**Post-effect status — derived from the RE-PROJECTED state [R0-I5]:**

- **Clean:** the `PendingTransfer` entry and the advisory `UnmatchedOutflows` blocker clear
  from the re-projected state; a `Disposal` (sell/spend) or `Removal` (gift/donate) appears.
  Status: `"Reclassified outflow as {kind}"`. Robustness option [R0-M1]: the step-2 check may
  additionally assert the target now appears in `state.disposals`/`state.removals` and report
  otherwise — a stronger invariant than blocker-scanning alone.
- **`UncoveredDisposal` fires:** if the lot pool does not fully cover the principal at the
  TransferOut's date, the post-reclassify consume paths fire a Hard `UncoveredDisposal` —
  the Dispose/GiftOut/Donate arms of the fold (fold.rs ~575–630 and ~965–1095), NOT the
  `Op::PendingOut` arm (fold.rs:712–718), which stops executing once the outflow is
  reclassified [R0-M6 citation fix]. Note the blocker may ALSO pre-exist: an uncovered
  PendingOut fires `UncoveredDisposal` before any reclassification (fold.rs:713–718), so the
  step-2 check may surface a pre-existing shortfall — fine for honesty; KAT-E2E-UNCOVERED
  asserts the pre-state explicitly. Status:
  `"Reclassified outflow as {kind} — WARNING: UncoveredDisposal blocker fired; check Holdings"`.
- **`DecisionConflict` attributed to the new decision id** (the failed-save-retry duplicate
  [R0-I1], or the stale-duplicate edge): resolve fires `DecisionConflict` on a duplicate
  `ReclassifyOutflow` for the same out event, attributed to the NEW decision's id
  (resolve.rs:606–614) — so the check-by-returned-id works. Status:
  `"Saved, but DecisionConflict fired on this decision — see Compliance; clear with CLI: btctax reconcile void decision|{seq}"`.
  **[R0-M1 correction:** a link+reclassify overlap is NOT a conflict — resolve.rs:600–605
  explicitly treats it as *precedence* (the link silently wins in `build_op`; the reclassify is
  inert, no blocker). That arm is unreachable in chunk 2a — the editor holds the VaultLock for
  its lifetime (persist.rs:8–11) and no link flow exists — but the claim is corrected here so
  chunk 3 (link-transfer) does not inherit a false safety story. The disposals/removals
  presence check above is the robust detector for that future case.]

**Modal content — reclassify-outflow:**

`ReclassifyOutflowModalState`:

```rust
pub struct ReclassifyOutflowModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub principal_sat: Sat,
    pub payload: ReclassifyOutflow,   // the VALIDATED payload — what will be persisted
}
```

Modal rendering (structural clone of `draw_mutation_modal`, draw_edit.rs:232–293):

```
╔═ Confirm: reclassify-outflow — WRITES THE VAULT ════════╗
║  target:  import|coinbase|…  (TransferOut)              ║
║  date:    2025-06-01                                     ║
║  principal_sat: 1000000                                  ║
║                                                          ║
║  as: sell                                                ║
║    gross_proceeds: 640.00                                ║
║    fee_usd:        2.50    (None = omitted)              ║
║                                                          ║
║  Appended as a decision event (append-only log).         ║
║  Saved immediately via the vault's atomic write path.    ║
║                                                          ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

**[R0-I7] The `donee` line is shown for BOTH gift and donate** (the payload carries it for
both; `Op::GiftOut` consumes it, resolve.rs:222–228, and it lands in removals.csv / Form 8283
surfaces — a persisted field the user must see at the write gate):

Gift variant:
```
║  as: gift                                                ║
║    fmv:     640.00                                       ║
║    fee_usd: (none)                                       ║
║    donee:   Alice                                        ║   ← or "donee: (none)"
```

Donate variant:
```
║  as: donate                                              ║
║    fmv:     640.00                                       ║
║    fee_usd: (none)                                       ║
║    appraisal_required: true                              ║
║    donee:   Community Foundation                         ║   ← or "donee: (none)"
```

Modal keys: Enter → `persist_reclassify_outflow`; Esc → close modal (back to field form);
other keys swallowed. Same Enter-arm semantics as D2 (`Ok(id)` → re-project + step-2 status;
`Err` → keep form, "Save error").

---

### D4 — `edit/persist.rs` additions + modal plumbing

**Two new `pub fn` items in `edit/persist.rs`** (the ONLY location permitted to name
`append_decision`):

```rust
/// Append a ClassifyInbound decision and atomically save the vault.
///
/// Mirrors `cmd::reconcile::classify_inbound` (reconcile.rs:39–53) minus the open/drop.
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # Failed-save + retry semantics [R0-I1] (see Hard constraints — stated once there,
/// summarized here)
/// If `append_decision` succeeds but `save` fails, the in-memory session carries the committed
/// decision while the on-disk vault remains pre-action. No rollback. A retry appends an
/// identical-payload DUPLICATE with a new `decision_seq`; resolve is FIRST-WINS
/// (resolve.rs:549–564) — the FIRST (failed-save) decision stays in force, and the duplicate
/// fires a Hard `DecisionConflict` that gates the tax year. Classification content is unchanged
/// (identical payloads). The post-persist status surfaces the conflict (D4 step 2); the user
/// clears it via the CLI (`btctax reconcile void decision|<seq>`) until chunk 2b's void flow.
pub fn persist_classify_inbound(
    session: &mut Session,
    payload: EventPayload,   // must be EventPayload::ClassifyInbound
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}

/// Append a ReclassifyOutflow decision and atomically save the vault.
///
/// Mirrors `cmd::reconcile::reclassify_outflow` (reconcile.rs:59–80) minus the open/drop.
/// `now` is INJECTED at Enter-press.
///
/// # Failed-save + retry semantics [R0-I1]
/// Same as `persist_classify_inbound`: FIRST-WINS (resolve.rs:600–617) — the failed-save
/// decision stays in force; the retry duplicate fires a Hard `DecisionConflict` on ITS id
/// (resolve.rs:606–614). Surfaced by the D4 step-2 status check; cleared via CLI void.
pub fn persist_reclassify_outflow(
    session: &mut Session,
    payload: EventPayload,   // must be EventPayload::ReclassifyOutflow
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}
```

**Implementation notes:**
- Both functions receive a pre-built `EventPayload` (the validation happens in the form layer).
- The `now: OffsetDateTime` comes from the modal-Enter handler: capture
  `OffsetDateTime::now_utc()` AT Enter-press, pass it to persist (not captured in the form
  state, not captured at form open — test injection overrides this via the test's own `now`).
- `UtcOffset::UTC` and `wallet: None` are constants (decisions are not wallet-scoped), matching
  `cmd::reconcile::append_and_save` at reconcile.rs:31.
- KAT-G1 requires no change: `append_` is already in `persist_only_tokens` (persist.rs:235);
  `conn(` and `save(` are already allowlisted for `edit/persist.rs`.

**Dispatch plumbing [R0-I2]:** the modal fields and dispatch order are defined once in D1
(modal → flow → form → screen; at most one flow / one modal `Some`). The `handle_modal_key`
pattern is replicated per modal type.

**Re-projection + status (the Enter-arm "step 2" — the uniform post-persist blocker check
[R0-I5]):**

On `Ok(id)`:
1. Call `btctax_tui::unlock::build_snapshot(session)` → `app.snapshot = Some(new_snap)`.
2. Derive the status from the NEW `snap.state.blockers` (never from the payload shape):
   - any blocker with `event == Some(id)` (the returned decision id) → the DecisionConflict
     status (D2/D3);
   - inbound flow: `FmvMissing` / `UnknownBasisInbound` with `event == target` → the
     corresponding honest status (D2);
   - outflow flow: `UncoveredDisposal` with `event == target` → the warning status (D3);
     optionally assert target presence in `disposals`/`removals` [R0-M1];
   - otherwise → the clean success status.
3. Close the modal and the flow state.

On `Err(e)`: close modal, keep field form open, `status = "Save error: {e}"`. No re-projection.

---

### D5 — safety tests

All tests are TDD-red first, then implementation, then green. The full validation suite must
pass at every step.

#### KAT-P2a — strict-prefix test for classify-inbound

The STRICT form activates (unlike chunk 1's degenerate `post == pre`):

```
post.len() == pre.len() + 1
post[..pre.len()] == pre[..]          // prefix equality (full RawEventRow, ordinal included)
post[pre.len()].kind == "decision"
// [R0-I6] decision_seq expectation mirrors append_decision's MAX-over-all-decision-rows
// allocator (persistence.rs:246–250) — NOT pre.last(), which may be a non-decision row:
post[pre.len()].decision_seq
  == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
// payload round-trip:
serde_json::from_str::<EventPayload>(&post[pre.len()].payload_json)
  == EventPayload::ClassifyInbound(expected_classify_inbound_payload)
```

Seed: non-empty `pre` (≥ 2 rows, same fixture as KAT-P1 uses: two `MethodElection` events).
Flow: open session; `pre = load_all_ordered(conn)`; call `persist_classify_inbound(session,
payload, now)`; assert strict-prefix in-memory; drop + reopen; assert strict-prefix on disk.
The mutation-actually-happened guard: the new decision appears in the log and its `payload_json`
round-trips correctly. Seed a non-voided, genuine `TransferIn` event in the vault (via
`append_import_batch`, persistence.rs:172 — test-region use) so the classify-inbound payload
references a real EventId.

#### KAT-P2b — strict-prefix test for reclassify-outflow

Same structure as KAT-P2a (including the [R0-I6] `decision_seq` formula), but with
`persist_reclassify_outflow` and `EventPayload::ReclassifyOutflow`. Seed a `TransferOut` event
for the target. Assert `post[pre.len()].payload_json` round-trips as `ReclassifyOutflow`.

#### KAT-C2a — cancel-path bytes-unchanged test (classify-inbound)

Pattern: chunk-1 KAT-C1 (crates/btctax-tui-edit/src/main.rs:972–1096 [R0-M5 citation fix]).

Temp vault; `bytes_before = fs::read(vault)`. Open session; press `c` → flow opens at List;
press `↓` (select second item if present); press `Enter` → variant picker; press `Tab` →
GiftReceived; press `Enter` → gift form; type into `fmv_at_gift_buf`; press `Enter` → modal
opens (assert `classify_inbound_modal.is_some()`); press **`Esc`** → modal closes (assert form
still open); press `Esc` → form closes (back to variant picker); press `Esc` → back to list;
press `Esc` → flow closes; press `q` → quit path. Additionally assert at EVERY flow step that
`q` is swallowed (`!should_quit`, flow stays open) [R0-I2 — the Esc-walk doubles as the
fall-through regression test]. `bytes_after = fs::read(vault)`; assert byte-identical.

Complement: after a confirmed flow the vault bytes differ (the save actually wrote).

#### KAT-C2b — cancel-path bytes-unchanged test (reclassify-outflow)

Same structure: `o` → list; `Enter`; kind picker; `Enter` (sell); amount form; `Enter` → modal;
`Esc` → modal closes; `Esc` → form closes; `Esc` → kind picker; `Esc` → list; `Esc` → flow
closes; `q`. `q` swallowed at every step. Vault bytes unchanged. Complement: confirmed path
writes.

#### KAT-S2 — save-error path (`#[cfg(unix)]`) [R0-I1 — asserts the TRUE retry outcome]

Same chmod pattern as chunk-1 KAT-S1 (main.rs:1100–1123), **including its root-skip guard**
(main.rs:1120–1124: chmod 0o500 does not deny writes to root — probe and skip with an explicit
message) [R0-N5]. Applied to `persist_classify_inbound`:

1. Seed; open session; capture `pre = load_all_ordered(conn)` (on-disk row set).
2. Navigate to the modal; chmod the vault's parent dir 0o500; press `Enter` (confirm) →
   assert: (1) modal closed, (2) form still open with buffers intact, (3) `status` contains
   "Save error", (4) on-disk vault bytes unchanged. The in-memory log now carries decision
   N+1 (committed by `append_decision` before `save()` failed).
3. Restore permissions; re-submit the form (`Enter` → modal → `Enter`) → the retry appends
   decision N+2 and the save succeeds. Assert the TRUE outcome:
   - on-disk log == `pre` + **2** decision rows (NOT +1);
   - BOTH new rows' `payload_json` round-trip to the IDENTICAL payload;
   - the re-projected `snap.state.blockers` contains a Hard `DecisionConflict` attributed to
     the retry decision's id (FIRST-WINS: the failed-save decision governs);
   - the post-persist `status` surfaces the conflict (the D4 step-2 check).

**Pre-recorded fallback (chunk-1 [R0-M3] discipline):** if this KAT proves flaky in CI, mark it
`#[ignore]` AND move its claims to documented-not-tested status in FOLLOWUPS — never silently
dropped.

#### KAT-E2E-CI — end-to-end classify-inbound (Income with FMV)

Full key-driven flow test via `handle_key`:

1. Seed a `TransferIn` event in a temp vault (via `append_import_batch`). Confirm the seed
   produces `UnknownBasisInbound` in the projected state (`project` call on the events).
2. Drive the editor: `c` → list shows the TransferIn; `Enter`; variant picker shows `Income`
   (initial); `Enter` → Income form (kind row initial = **Mining** [R0-M3]); press `Tab` once
   on the kind row → **Staking** (exercises the picker in E2E); type FMV into `fmv_buf`;
   `Enter` → modal renders (assert contains canonical EventId + "staking" + the FMV value);
   `Enter` → save + re-project.
3. Reopen a fresh `Session`; call `project` → assert: `UnknownBasisInbound` blocker for the
   target EventId is GONE; a new `IncomeRecord` with the specified FMV appears in
   `state.income_recognized`; a new `Lot` with non-zero `usd_basis` appears in `state.lots`.
4. CLI reads: run `cmd::inspect::verify` on the vault → `UnknownBasisInbound` for that event
   is absent from the report.

#### KAT-E2E-FMV-MISSING — classify-inbound Income without FMV

Same as KAT-E2E-CI but `fmv_buf` left empty. After save + re-project: `FmvMissing` Hard blocker
fires for the target event; the `Lot` is created with `basis_pending=true`; status contains
`"FmvMissing"` AND `"void"` (the [R0-I4] remedy wording — no "set-fmv" string).

#### KAT-E2E-GIFT-UNKNOWN — classify-inbound GiftReceived with both donor fields empty

After save + re-project: the original `UnknownBasisInbound` (unclassified TransferIn) is gone;
a new `UnknownBasisInbound` fires (gift case 4, fold.rs:929–935); status contains
`"UnknownBasisInbound"` (or "basis unknown") AND `"void"` [R0-I4]; the classify-inbound list
(`c`) now does NOT show this TransferIn (it has a ClassifyInbound decision → pre-filtered out).

#### KAT-E2E-GIFT-PRICE-GAP — gift case 3: donor date given, price unavailable [R0-I5]

Seed as KAT-E2E-GIFT-UNKNOWN, but classify with `donor_basis` empty and `donor_acquired_at` set
to a date OUTSIDE the bundled price dataset (e.g. `1990-01-01`). After save + re-project:
`UnknownBasisInbound` re-fires (fold.rs:913–927); the status is the SAME honest
basis-unknown/void message — proving the status derives from the re-projected blockers, not the
payload shape (a shape-keyed status would falsely report clean success here).

#### KAT-E2E-GIFT-DUAL — gift happy path with donor basis (dual-basis pinning) [R0-M7]

Classify a TransferIn as GiftReceived with `fmv_at_gift < donor_basis` and a
`donor_acquired_at`. After save + re-project: NO `UnknownBasisInbound` for the target; the gift
lot pins the §1015 dual-basis construction (fold.rs:903–954): `usd_basis == donor_basis`,
`dual_loss_basis == Some(fmv_at_gift)`, `donor_acquired_at` carried through; status is the
clean success string.

#### KAT-E2E-RO — end-to-end reclassify-outflow (Sell)

1. Seed a `TransferOut` event; confirm it appears in `pending_reconciliation`.
2. Drive `o` → list; `Enter`; pick `sell`; assert the amount label reads "gross proceeds"
   [R0-I3]; type amount; `Enter` → modal (assert canonical EventId + "sell" + amount);
   `Enter` → save + re-project.
3. Reopen; project → `pending_reconciliation` no longer contains the target EventId; a
   `Disposal` with `kind=Sell` and the specified proceeds appears in `state.disposals`.
4. CLI reads back via `cmd::inspect::report`.

#### KAT-E2E-UNCOVERED — reclassify-outflow with UncoveredDisposal

Seed a `TransferOut` for more sat than the lot pool holds. **Assert the pre-state first
[R0-M6]: the uncovered PendingOut already fires `UncoveredDisposal` before reclassification
(fold.rs:713–718)** — so the test documents the transition truthfully. After reclassify-outflow
save + re-project: `UncoveredDisposal` still present (now from the Dispose consume path);
status contains "UncoveredDisposal".

#### KAT-E2E-DONATE — end-to-end reclassify-outflow (Donate with appraisal + donee)

Drive `o` → list; pick `donate`; fill amount (label reads "FMV"); toggle appraisal; fill donee;
modal renders → assert it shows `appraisal_required: true` AND the `donee` value [R0-I7];
confirm → assert `Removal` with `kind=Donation`, `appraisal_required=true`, `donee=Some("…")`
in state.

#### KAT-E2E-GIFTOUT-DONEE — gift-path modal shows donee [R0-I7]

Drive `o` → pick `gift`; fill amount + donee; modal renders → assert the `donee` value appears
(and `appraisal_required` does NOT); confirm → `Removal` with `kind=Gift`, `donee=Some("…")`.

#### Validation KATs (per-form field table)

**classify-inbound:**
- **KAT-V-CI-1:** Income `fmv` empty → `fmv=None` (valid, no error at validation).
- **KAT-V-CI-2:** Income `fmv` non-empty valid decimal → parses correctly.
- **KAT-V-CI-3:** Income `fmv` non-empty non-numeric → parse error "bad USD…".
- **KAT-V-CI-4:** Income `fmv` whitespace-only → parse error (not None; same [R0-M4] pin as
  chunk 1 — whitespace is not empty).
- **KAT-V-CI-5:** GiftReceived `fmv_at_gift` empty → error "fmv-at-gift is required".
- **KAT-V-CI-6:** GiftReceived `fmv_at_gift` valid → parses correctly.
- **KAT-V-CI-7:** GiftReceived `donor_acquired_at` non-empty, valid YYYY-MM-DD → parses.
- **KAT-V-CI-8:** GiftReceived `donor_acquired_at` non-empty, bad format → error "bad date…".
- **KAT-V-CI-9:** IncomeKind Tab cycles through all 5 variants in order: Mining → Staking →
  Interest → Airdrop → Reward → Mining; initial selection is Mining [R0-M3].

**reclassify-outflow:**
- **KAT-V-RO-1:** `amount` empty → error "amount is required".
- **KAT-V-RO-2:** `amount` valid decimal → parses.
- **KAT-V-RO-3:** `amount` whitespace-only → parse error (not "required" — [R0-M4] pin).
- **KAT-V-RO-4:** `fee` empty → `None` (no error).
- **KAT-V-RO-5:** `fee` valid → parses.
- **KAT-V-RO-6:** `appraisal` toggle: default false; Space toggles to true; Space again false.
- **KAT-V-RO-7:** `donee` empty → `None`; non-empty → `Some(trimmed)`.
- **KAT-V-RO-8:** OutflowKind Tab cycles: sell → spend → gift → donate → sell.
- **KAT-V-RO-9:** Label for `amount` is "gross proceeds (USD)" when kind=sell/**spend**;
  "FMV (USD)" when kind=gift/donate [R0-I3 — now consistent with the D3 table].

#### KAT-G1 (inherited) — stays green

Both `persist_classify_inbound` and `persist_reclassify_outflow` live in `edit/persist.rs`.
No new token appears in any other source file. KAT-G1 (`kat_g1_mechanized_source_gate`) requires
no modification and must continue to pass after chunk 2a is implemented.

---

## Plan (TDD)

### Task 1 — list widget + flow dispatch layer + classify-inbound flow

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `InboundListItem`, `OutflowListItem`,
  `TargetList<T>` [R0-N1], `ClassifyInboundFlowState`, `ClassifyInboundStep`,
  `ClassifyInboundModalState`, `validate_classify_inbound_income`,
  `validate_classify_inbound_gift`.
- `crates/btctax-tui-edit/src/editor.rs` — add `classify_inbound_flow: Option<…>`,
  `classify_inbound_modal: Option<…>` to `EditorApp` (NO standalone list fields [R0-I2]);
  corresponding `None` in `EditorApp::new`.
- `crates/btctax-tui-edit/src/main.rs` — add `c` key dispatch; insert the FLOW dispatch layer
  guarded by `flow.is_some()` covering ALL steps (modal → flow → form → screen) [R0-I2];
  `handle_classify_inbound_flow_key` (dispatches on step), `handle_classify_inbound_modal_key`;
  the D4 step-2 re-projected-blockers status check [R0-I5].
- `crates/btctax-tui-edit/src/draw_edit.rs` — add `draw_classify_inbound_list`,
  `draw_classify_inbound_form`, `draw_classify_inbound_modal`; render overlay in `draw_browse`.
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_classify_inbound`.

**All KAT-P2a, KAT-C2a, KAT-S2, KAT-E2E-CI, KAT-E2E-FMV-MISSING, KAT-E2E-GIFT-UNKNOWN,
KAT-E2E-GIFT-PRICE-GAP, KAT-E2E-GIFT-DUAL, KAT-V-CI-1..9 written TDD-red before implementation,
green after.**

KAT-G1 must stay green throughout.

### Task 2 — reclassify-outflow flow

**Files:**
- `crates/btctax-tui-edit/src/edit/form.rs` — add `ReclassifyOutflowFlowState`,
  `ReclassifyOutflowStep`, `ReclassifyOutflowModalState`, `OutflowKind`,
  `validate_reclassify_outflow`.
- `crates/btctax-tui-edit/src/editor.rs` — add `reclassify_outflow_flow: Option<…>`,
  `reclassify_outflow_modal: Option<…>`.
- `crates/btctax-tui-edit/src/main.rs` — `o` key; `handle_reclassify_outflow_*` handlers
  (in the flow layer).
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_reclassify_outflow_*` renderers (donee line
  for gift AND donate [R0-I7]).
- `crates/btctax-tui-edit/src/edit/persist.rs` — add `persist_reclassify_outflow`.

**All KAT-P2b, KAT-C2b, KAT-E2E-RO, KAT-E2E-UNCOVERED, KAT-E2E-DONATE, KAT-E2E-GIFTOUT-DONEE,
KAT-V-RO-1..9 written TDD-red before implementation, green after.**

### Task 3 — whole-diff review (Phase E) + FOLLOWUPS

Cross-cutting checks:

- **Editor guarantee unchanged:** `append_decision` and `conn(`/`save(` appear only in
  `edit/persist.rs` non-test code; KAT-G1 green; no new forbidden tokens elsewhere.
- **Modal gating:** `persist_classify_inbound` sole non-test call site is the classify-inbound
  modal's Enter arm; `persist_reclassify_outflow` sole non-test call site is the
  reclassify-outflow modal's Enter arm. Verified by grep + KAT-G1.
- **Dispatch order [R0-I2]:** modal → flow → form → screen verified in `handle_key`; the flow
  layer is guarded by the flow `Option` (not the step), so no step can leak keys to Browse;
  Esc from any inner step goes BACK one step, never quits; `q` swallowed by modal and every
  flow step (KAT-C2a/C2b assert this at each step).
- **Pre-filter correctness:** verify the compound inbound filter (Claim A) produces no
  already-classified TransferIn in the list; the [R0-M2] raw-vs-effective limitation is
  documented (spec + FOLLOWUPS); verify the outflow list is sourced from
  `pending_reconciliation` only (Claim B; no extra filter).
- **`now` injection:** `persist_classify_inbound` and `persist_reclassify_outflow` receive
  `now` from the Enter-press handler, NOT from `OffsetDateTime::now_utc()` inside the fn body.
- **Modal content completeness:** the classify-inbound modal shows the target canonical
  EventId, date, sat, and ALL `InboundClass` fields (kind, fmv, business for Income;
  fmv_at_gift, donor_basis, donor_acquired_at for GiftReceived, including the both-None
  warning). The reclassify-outflow modal shows target canonical EventId, date, principal_sat,
  and ALL `ReclassifyOutflow` fields — kind, principal_proceeds_or_fmv (label per [R0-I3]),
  fee_usd, appraisal_required (donate), **donee for BOTH gift and donate [R0-I7]**.
- **Post-effect honesty [R0-I4/I5]:** the status derivation is blocker-driven (re-projected
  state), never payload-shape-driven; no status string recommends set-fmv on a TransferIn or a
  bare re-classify; the void-then-re-classify remedy names the CLI path; KAT-E2E-FMV-MISSING /
  GIFT-UNKNOWN / GIFT-PRICE-GAP pin the strings.
- **Retry semantics [R0-I1]:** both persist-fn doc comments state FIRST-WINS + Hard
  `DecisionConflict` on the retry duplicate; KAT-S2 asserts `pre + 2`, both round-trips, the
  conflict in the re-projection, and the status surfacing it. No dedup/rollback "fix" crept in.
- **Prefix test correctness (strict form):** KAT-P2a and KAT-P2b assert
  `post.len() == pre.len() + 1`, `post[..pre.len()] == pre`, `kind=="decision"`, the [R0-I6]
  MAX-based `decision_seq` expectation, and the payload round-trip; seed is non-empty.
- **Stale-race wording [R0-M1]:** the DecisionConflict check is by the returned decision id
  (resolve.rs:606–614); no claim that a link overlap conflicts (it is precedence,
  resolve.rs:600–605); the disposals/removals presence check is noted for chunk 3.
- **Viewer untouched:** no viewer files change; E10 gate continues to pass.

FOLLOWUPS to record:
- **Chunk 2b:** `reclassify-income` / `set-fmv` / `void` / `set-donation-details` flows. The
  void flow retires the CLI-void interim path named in the D2/D3 statuses [R0-M4]. If 2b specs
  a set-fmv flow, it MUST be Income-event-targeted only (resolve.rs:423–470) [R0-I4].
- **Chunk 3+:** `link-transfer`, `safe-harbor-allocate`, `select-lots`, `optimize-accept` —
  chunk 3's link flow must use the disposals/removals presence check (not blocker-scanning)
  for the link-vs-reclassify precedence case [R0-M1].
- **[R0-M2] Effective-payload under-inclusion:** a TransferIn-via-`ClassifyRaw` (or accepted
  ImportConflict) is CLI-classifiable but invisible in the TUI list. Cheap fix: also treat as
  TransferIn any event targeted by a non-voided `ClassifyRaw{as_: TransferIn(_)}`.
- **Incomplete-gift rows (pre-filtered out):** resolvable only by void + re-classify; in 2a
  the void is the CLI (`btctax reconcile void decision|<seq>`) [R0-M4]; 2b's void flow makes
  this in-TUI.
- **[R0-N3] Negative-sign validation** on `amount`/`fmv`/`fee` fields: `parse_usd_arg` accepts
  negative decimals on BOTH surfaces (CLI + TUI, exact parity) — tightening must land on both
  together, parity-preserving (mirrors chunk 1's carryforward FOLLOWUP).
- **[R0-N4] donee trim/cap divergence** (TUI trims + caps at 64; CLI unbounded) — align if it
  ever matters.
- **List display polish:** structured wallet display; `events_by_id` caching if ever needed.
- If KAT-S2 was downgraded per its fallback: its claims are documented-not-tested and need a
  testing seam (record only if the downgrade happened).

---

## Out of scope

- **Chunk 2b:** `reclassify-income`, `set-fmv`, `void`, `set-donation-details`. Not in this
  spec.
- **Chunk 3+:** `link-transfer`, `safe-harbor-allocate`, `select-lots`, `optimize-accept`,
  `safe-harbor-attest`, `accept-conflict`, `reject-conflict`, `classify-raw`.
- **Viewer changes:** the viewer crate is frozen (E10 gate, guarantee wording, write-free
  status).
- **`btctax-core` or `btctax-cli` changes:** no new core types, no new CLI commands. Chunk 2a
  uses existing `EventPayload` variants and `append_decision` exactly as the CLI does.
- **`load_all_ordered` / `RawEventRow` changes:** both already shipped and correct (ordinal
  included per the N6 amendment, persistence.rs:334–380). No modification needed.
- **Donation details side-table** (`set-donation-details` / `DonationDetails`): chunk 2b.
- **Any non-TUI reconcile surface:** this spec is strictly TUI-side wiring for two existing
  decision payload types.
