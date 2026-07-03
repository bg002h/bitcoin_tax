# SPEC — tui-edit-hardening: the six chunk-3 follow-up fixes

**Source baseline:** `main` @ `8c8b924` (post save-rollback; all anchors re-verified at write time).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-tui-edit-hardening-round-{1,2}.md` (round 1: 0C/1I/2M/2N — the #2 §7.4-boundary
feasibility catch; round 2: 0C/0I — folds verified, `basis_source` discriminator provenance-exact).**
**Design lineage:** an architect design pass over the 6 items (all anchors verified against current
source; corrected #8 to 6 arms/4 fns, sharpened #2 to an acquisition-date gate, defined "effective"
for #7 precisely).

**Goal.** Cycle B of the autonomous post-chunk-3 run (roadmap `design/ROADMAP_autonomous_run.md`,
order A→B→C→D→E). Six hardening fixes to `btctax-tui-edit`, grouped Group-A (select-lots correctness)
+ Group-B (safety/UX):

- **#1** SelfTransfer disposals are selectable in select-lots (under-inclusion fix).
- **#2** pre-2025 disposals offer Universal-pool (cross-wallet) candidate lots.
- **#3** under-covered (`UncoveredDisposal`) disposals are pre-filtered out of select-lots (no doomed
  selection).
- **#6** free-text donation fields accept CLI-parity length (per-instance `FieldBuffer` cap).
- **#7** the void list pre-filters EFFECTIVE SafeHarborAllocations (closes the permanent §7.4
  doomed-void trap).
- **#8** the CLI-void remedy in 6 status arms names "quit the editor first" (VaultLock audit).

**SemVer.** All additive / bugfix; `btctax-core` is **read-only** (no public API touched); `btctax-tui-edit`
is a binary (SemVer advisory). New `FieldBuffer::with_cap` + `FREETEXT_CAP` const; new
`DisposalKind::SelfTransfer` variant. **PATCH/MINOR**, no breaking changes.

**Lockstep: NONE** (verified: no clap flags added/changed; no `docs/manual/` exists; no `docs/**`
references the folded #8 strings or the `FIELD_CAP` value — the only user-visible contract touched is
TUI status text, not pinned outside the crate).

---

## Verified core facts (design basis)

- `TRANSITION_DATE = 2025-01-01` (`conventions.rs:17`); `pool_key(date, wallet)` → `Universal` iff
  `date < TRANSITION_DATE` (`project/pools.rs:15-19`); origin lots are placed with
  `pool_key(acquired_at, wallet)` (`project/fold.rs:566,695,877,955`), so a **pre-2025 origin** lot
  starts in `Universal`. **BUT [R0-I1] the §7.4 boundary seed drains `Universal` at the first ≥2025
  event** (`transition.rs:75-103`): Path A relocates each residue lot to `Wallet(lot.wallet)` with its
  lot_id preserved and `basis_source = ReconstructedPerWallet` (`transition.rs:83`); Path B **discards**
  the residue and installs allocation **seed lots** with NEW lot_ids `{allocation_id, seq}` and
  `basis_source = SafeHarborAllocated`. So in the FINAL `snap.state.lots` there is no `Universal` pool
  once any ≥2025 event exists, and `acquired_at < 2025` alone does NOT imply the lot is feasible for a
  pre-2025 disposal (which consumes from `Universal` at its pre-boundary fold position). Feasibility by
  provenance: `ReconstructedPerWallet` (lot_id preserved) is feasible; `SafeHarborAllocated` seed lots
  (lot_ids that never existed in `Universal`) are NOT — they raise a **hard `LotSelectionInvalid`**
  (`fold.rs:59-67` `consume_principal` turns any `selection_feasible` Err into a gating blocker; there
  is NO method-order fallback for an infeasible pick).
- `honoring_principal` (`project/resolve.rs:1008`) → `Some(sat)` for `Dispose|GiftOut|Donate|SelfTransfer`;
  a `LotSelection` targeting anything else → `LotSelectionInvalid` (`resolve.rs:802-808`). SelfTransfer
  principal = `TransferOut.sat` (fee excluded, `resolve.rs:211-214`).
- A `TransferOut` projects to `Op::SelfTransfer` iff a non-voided `TransferLink` names it, dest
  resolvable to a wallet (`resolve.rs:201-216`; link-build loop `resolve.rs:486-527`).
- **Effective allocation** (§7.4 irrevocable): a non-voided `SafeHarborAllocation` on whose id
  **neither** `SafeHarborTimebar` **nor** `SafeHarborUnconservable` fired (`resolve.rs:883-921`; attest
  arms `main.rs:3575/3583`). Core pins both directions [R0-M1: integration tests, NOT the 103-line
  `src/project/transition.rs` module]: `crates/btctax-core/tests/transition.rs:365
  void_of_effective_allocation_is_a_decision_conflict` and `:403
  void_of_inert_allocation_applies_no_conflict`.
- `Σ legs.sat < op.sat` (a shortfall) holds **iff** an `UncoveredDisposal` blocker fired for that event.

---

## Hard constraints

- `btctax-core` is READ-ONLY (no public API change). All six are TUI-local.
- The KAT-G1 mutation-surface gate is untouched (no persist-layer change).
- `#7` reads from the cached `snap` (not a fresh projection): `open_void_flow` already guards
  in-memory residue via `residue_latch_status()` (`main.rs:2463`), and `snap.state.blockers` is the
  same projection the list is built from.
- The save-rollback persist layer is untouched (`#7` is read-side list construction only).

---

## Design — Task 1: parity folds (#8 + #6)

### D-#8 — quit-first status fold (6 arms / 4 fns)

The string `"or CLI: btctax reconcile void …"` omits the "quit the editor first" clause the R0-C1 lock
audit requires (the editor holds the exclusive `VaultLock` for its lifetime). Canonical wording already
exists in `derive_select_lots_status` (`main.rs:3432-3434`): `"…clear with Void flow (press 'v'), or
quit the editor and run: btctax reconcile void {} …"`.

**Fold** `"or CLI: btctax reconcile void"` → `"or quit the editor and run: btctax reconcile void"`
at ALL SIX arms. [R0-N2] Some arms split the phrase across `\`-continuation lines (site 2's `or CLI:`
is on `:2078`), and the "add the comma after `(press 'v')`" tweak applies only to the DecisionConflict
arms — so **fold each arm's literal individually**, not a blanket search-replace; the per-arm RS KATs
catch a miss:

| # | anchor | fn / arm | KAT |
|---|---|---|---|
| 1 | `main.rs:2061` | `derive_classify_inbound_status` — DecisionConflict | KAT-RS-1 (`:7860`) |
| 2 | `main.rs:2079` | `derive_classify_inbound_status` — FmvMissing | KAT-RS-2 (`:7883`) |
| 3 | `main.rs:2097` | `derive_classify_inbound_status` — UnknownBasisInbound | KAT-RS-3 (`:7907`) |
| 4 | `main.rs:2133` | `derive_reclassify_outflow_status` — DecisionConflict | KAT-RS-4 (`:7933`) |
| 5 | `main.rs:2345` | `derive_reclassify_income_status` — DecisionConflict | **add KAT-RS-5** |
| 6 | `main.rs:2374` | `derive_set_fmv_status` — DecisionConflict | **add KAT-RS-6** |

KAT-RS-1..4 currently assert only `contains("'v'")` + `contains("btctax reconcile void")` — both
survive the fold. **Enrich** RS-1..4 with `assert!(status.contains("quit the editor"))` to lock the new
wording; **add** RS-5/RS-6 (both derivers have no current unit coverage — synthetic-conflict-snapshot
tests mirroring RS-1/RS-4).

### D-#6 — per-instance FieldBuffer cap for free-text fields

`FieldBuffer` (`form.rs:26-63`) hard-codes `FIELD_CAP = 64` in `push_char` (`:39`) and `set` (`:50`),
truncating donation free-text where the CLI (`cli/src/main.rs:318,336`, unbounded `Option<String>`)
does not.

- Add `cap: usize` to `FieldBuffer`; `new()` → `cap = FIELD_CAP`; add `with_cap(cap)`; `Default`
  unchanged; `push_char`/`set` check `self.cap`; pre-alloc `String::with_capacity(cap)` (preserves the
  "never reallocates" invariant).
- Add `pub const FREETEXT_CAP: usize = 512;` [R0-N1: a generous BOUND for TUI rendering, not literal
  parity — the CLI is unbounded `Option<String>`; 512 covers realistic addresses / multi-clause
  qualifications while keeping the buffer render-safe].
- In the donation FieldForm construction (`main.rs:3002-3011`), the **6 free-text** buffers use
  `FieldBuffer::with_cap(FREETEXT_CAP)`: `donee_name`, `donee_address`, `appraiser_name`,
  `appraiser_address`, `appraiser_qualifications`, `fmv_method_override`. Keep 64 for the **4
  structured** fields: `donee_ein`, `appraiser_tin`, `appraiser_ptin`, `appraisal_date` (fixed-format;
  a 512-char EIN is nonsense + a render hazard). The `.set(existing…)` seeding respects the per-field
  cap automatically.

**KATs.** KAT-FREETEXT-CAP: type a >64-char (200-char) `appraiser_qualifications`, save, reload, assert
full round-trip. KAT-STRUCTURED-CAP: `donee_ein` still caps at 64.

---

## Design — Task 2: void-list effective-allocation pre-filter (#7)

`open_void_flow` (`main.rs:2462`) lists every non-voided event passing `is_revocable_payload`
(`form.rs:824`, includes `SafeHarborAllocation`). A confirmed void of an **effective** allocation writes
a permanent `VoidDecisionEvent` the engine rejects with `DecisionConflict` (`resolve.rs:924-934`) — a
damaging §7.4 no-op.

**Fix.** After the `voided` set (`main.rs:2483`), exclude effective allocations from the items builder
(`:2490`):

```rust
let effective_alloc = |e: &LedgerEvent| {
    matches!(e.payload, EventPayload::SafeHarborAllocation(_)) && {
        let has = |k| snap.state.blockers.iter()
            .any(|b| b.kind == k && b.event.as_ref() == Some(&e.id));
        !has(BlockerKind::SafeHarborTimebar) && !has(BlockerKind::SafeHarborUnconservable)
    }
};
// in the list chain:  .filter(|e| !effective_alloc(e))
```

Inert allocations (timebarred OR unconservable) STAY voidable — voiding them applies cleanly
(`crates/btctax-core/tests/transition.rs:403`) — so they must remain listed. The `draw_edit.rs:1453` "if this allocation is
effective, voiding…" warning still applies to those inert allocations.

**KAT changes (the load-bearing part).**
- **REWRITE `KAT-E2E-ATTEST-VOID`** (`main.rs:10924`) — it currently *pins the trap* (asserts the
  attested alloc IS listed, `:10974`, and that voiding it yields "remains in force"). After attest, the
  attested alloc is effective (pre-filtered out) and the prior is already voided → the void list is
  **empty**. Assert `app.void_flow.is_none()` + `app.status == Some("No revocable decisions to void")`
  (confirm the empty-list status string in `open_void_flow`). Delete the select/confirm/DecisionConflict
  back half (TUI-unreachable now). **Engine coverage of the §7.4 guard is NOT lost** —
  `crates/btctax-core/tests/transition.rs:365` still pins it. Flag this rewrite for the whole-diff
  reviewer as an intentional supersession.
- **ADD KAT-VOID-EFFECTIVE-PREFILTER-MIXED:** seed an effective alloc PLUS one other revocable decision
  (e.g. a `LotSelection`) so the flow opens; assert the other decision IS listed and the effective alloc
  is NOT.
- **ADD KAT-VOID-INERT-ALLOC-LISTED:** a timebarred (unattested past-bar, e.g. ProRata) allocation
  REMAINS in the void list.

---

## Design — Task 3: select-lots coverage cluster (#1 + #2 + #3)

These three co-modify `open_select_lots_flow` (`main.rs:3247`) and the candidate-lot filter and
genuinely interact — one task.

### D-#1 — SelfTransfer under-inclusion

`open_select_lots_flow` builds from `snap.state.disposals` (`:3287`) + `snap.state.removals` (`:3314`)
only; SelfTransfers appear in neither. Reconstruct them in-TUI from `snap.events`, mirroring resolve.rs
pass-1.

- **New `DisposalKind::SelfTransfer`** variant (`form.rs:844`). Blast radius = exactly two exhaustive
  matches: `draw_edit.rs:1525-1528` and `:1686-1689` (each gains `SelfTransfer => "self-transfer"`).
- **Detection** (add after the `voided`/`already_selected` sets): replicate the engine — the engine
  iterates decisions by `decision_seq` (`resolve.rs:349-356`) and FIRST-WINS decides which out projects
  to `SelfTransfer`, so **collect non-voided `TransferLink` decisions and SORT them by `decision_seq`
  ascending** [R0-M2] before the loop; then FIRST-WINS on duplicate `out_event`; `consumed_ins` dedup
  (M-3); skip an `InEvent` link whose in-event is missing or has no wallet (I-1) — use `ev_idx.get`,
  never index [R0-M2]:
  ```
  linked_outs: BTreeSet<EventId>; consumed_ins: BTreeSet<EventId>;
  for non-voided TransferLink(tl), sorted by decision_seq asc:
    if linked_outs already has tl.out_event { continue }        // dup out -> first wins
    match tl.in_event_or_wallet {
      Wallet(_)      => linked_outs.insert(tl.out_event),
      InEvent(in_id) => if consumed_ins.contains(in_id) { continue }
                        else if ev_idx.get(in_id).and_then(|e| e.wallet.as_ref()).is_none() { continue }
                        else { consumed_ins.insert(in_id); linked_outs.insert(tl.out_event) },
    }
  ```
- Build `self_transfer_items` from raw `TransferOut` events whose id ∈ `linked_outs` and ∉
  `already_selected`: `disposal_event = e.id`; `date = tax_date(e.utc_timestamp, e.original_tz)` (as at
  `main.rs:2235`); `kind = SelfTransfer`; `principal_sat = transfer_out.sat` (NOT minus fee — matches
  `honoring_principal`); `wallet = e.wallet.clone()` (the SOURCE wallet — correct for the candidate-lot
  filter). Concat into `items` (`:3338`) before the sort.

**Decision:** in-TUI reconstruction (consistent with the opener already re-deriving voided/already_selected
from events; zero core-API change; residual drift backstopped by `LotSelectionInvalid`). A `resolve.rs`
`pub fn` exposing the honoring set is the zero-drift alternative but is out of scope (would make core MINOR).

**KATs.** KAT-SELFTRANSFER-SELECTABLE (seed TransferOut + non-voided TransferLink → row listed,
`principal_sat == TransferOut.sat`, `kind == SelfTransfer`; pick lots conserving principal → clean save +
status arm 3). KAT-SELFTRANSFER-VOIDED-LINK-ABSENT (void the TransferLink → row disappears).

### D-#2 — pre-2025 Universal-pool candidate-lot filter

Replace the candidate-lot filter (`main.rs:2722`, inside `handle_sl_list_key`'s Enter→LotsForm) — which
is `l.wallet == w` (and yields ZERO lots when `item.wallet == None`) — with a **feasibility-honest**
pre-2025 gate [R0-I1]:
```rust
let in_scope = if item.date < TRANSITION_DATE {
    // Pre-2025 disposals consume from the (pre-boundary) Universal residue. Offer pre-2025 lots across
    // wallets, but EXCLUDE Path-B SafeHarborAllocated seed lots — their lot_ids never existed in
    // Universal, so the engine raises a hard LotSelectionInvalid (no method-order fallback). Path-A
    // ReconstructedPerWallet lots preserve their Universal lot_ids and are feasible.
    l.acquired_at < TRANSITION_DATE
        && l.basis_source != btctax_core::BasisSource::SafeHarborAllocated
} else {
    wallet_ref.is_some_and(|w| &l.wallet == w) // per-wallet, unchanged
};
```
Behavior by governing path: **Path A** (no effective allocation) — pre-2025 lots are
`ReconstructedPerWallet` (lot_id preserved) → offered cross-wallet, engine-feasible (this is #2's actual
fix). **Path B** (an effective `SafeHarborAllocation` governs — a shipped/reachable feature) — the
pre-2025 residue is entirely seed lots (`SafeHarborAllocated`) → excluded → the disposal offers no
feasible lot → falls into the existing "No lots available for wallet …" path. This is a SAFE
under-inclusion (the CLI, which re-projects at the disposal's fold position, remains available) —
consistent with the acknowledged "lot-display at disposal date is best-effort" caveat, and strictly
better than offering doomed lots. Post-2025 origin lots (`acquired_at ≥ 2025`, Wallet pool) are still
correctly excluded for a pre-2025 disposal. Import `TRANSITION_DATE` and `BasisSource` into `main.rs`.

**Residual (acknowledged, backstopped):** a Path-A lot created by a LATER split (`bump_split`, e.g. a
pre-2025 self-transfer fragment) can still be offered for an EARLIER pre-2025 disposal where it was
infeasible at fold time — the irreducible "final-state ≠ fold-time-state" gap. The engine backstops
with `LotSelectionInvalid` (surfaced by `derive_select_lots_status` arm 2); recorded in FOLLOWUPS.

**KATs.** KAT-PRE2025-CROSSWALLET-LOTS (Path A: pre-2025 disposal wallet A; pre-2025
`ReconstructedPerWallet`-provenance lots in A and B → both offered; pick B → engine accepts, clean
save). KAT-PRE2025-PATHB-SEEDLOTS-EXCLUDED (Path B: an effective allocation governs → a pre-2025
disposal's `SafeHarborAllocated` seed lots are NOT offered → "No lots available" path; the engine would
have rejected them). KAT-POST2025-WALLET-SCOPED (post-2025 disposal offers only its wallet — regression
guard). KAT-PRE2025-EXCLUDES-POST2025-LOT (pre-2025 disposal does NOT offer a 2025-acquired lot).

### D-#3 — shortfall (UncoveredDisposal) pre-filter

`principal_sat = Σ legs.sat` (`main.rs:3294,:3320`); for an under-covered disposal this is `< op.sat`, so
`validate_select_lots` (`form.rs:955-961`) conserves against too-small a target and the engine rejects
the selection (`LotSelectionInvalid`). Selecting lots can never cure under-coverage (the pool is short).

**Fix — pre-filter, don't re-target.** In `open_select_lots_flow`, exclude any event carrying an
`UncoveredDisposal` blocker, over the MERGED list (disposals + removals + self-transfers):
```rust
let uncovered: BTreeSet<&EventId> = snap.state.blockers.iter()
    .filter(|b| b.kind == BlockerKind::UncoveredDisposal)
    .filter_map(|b| b.event.as_ref()).collect();
// add `.filter(|d| !uncovered.contains(&d.event))` to each builder (disposals, removals, self-transfers)
```
The disposal stays visible/actionable in Compliance (its `UncoveredDisposal` blocker names the real
remedy: add the missing acquisition). For SelfTransfer items `principal_sat = op.sat` directly, so the
legs-vs-op mismatch is a disposals/removals-only phenomenon — but the `UncoveredDisposal` guard is the
right universal gate.

**KAT.** KAT-UNCOVERED-EXCLUDED (an under-covered `Dispose` is absent; if it's the only candidate →
status "No method-honoring disposals available…" and the flow does not open).

---

## Plan (TDD, 3 tasks — each: KATs red → implement green → review to 0C/0I)

- **Task 1 — parity folds (#8 + #6).** Low-risk, non-interacting: the 6-arm string fold (+ enrich
  RS-1..4, add RS-5/6) and the contained `FieldBuffer` per-instance cap (+ KAT-FREETEXT-CAP / -STRUCTURED-CAP).
- **Task 2 — void safety (#7).** Standalone; risk concentrated in the `KAT-E2E-ATTEST-VOID` REWRITE —
  keep isolated so that rewrite is reviewed on its own. + mixed / inert-listed KATs.
- **Task 3 — select-lots cluster (#1 + #2 + #3).** Co-modify `open_select_lots_flow` + the candidate
  filter; ship together. Interactions: #1 sets date/wallet that #2's date-gate consumes; #3's
  `UncoveredDisposal` filter runs over the MERGED list (after #1 adds self-transfers).
- **Task 4 — whole-diff review (Phase E) + FOLLOWUPS.**

Sequence 1 → 2 → 3 (isolated first, interacting cluster last).

## Out of scope

- The optional `btctax-core` `resolve.rs` helper exposing the honoring set (would make core MINOR; the
  in-TUI reconstruction + `LotSelectionInvalid` backstop suffices).
- Any persist-layer / save-rollback change (#7 is read-side only).
- Chunk 4 (import) / chunk 5 (safe-harbor-allocate) — later cycles.
