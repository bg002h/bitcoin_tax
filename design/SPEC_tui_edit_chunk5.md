# SPEC — btctax-tui-edit chunk 5: safe-harbor-allocate (CREATION flow)

**Source baseline:** `main` @ `f31c1d6` (post chunk 4; all anchors verified at write time).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-tui-edit-chunk5-round-{1,2}.md` (round 1: 0C/0I/1M/2N — verified the 3 residue gotchas;
round 2: 0C/0I — the helper-returns-method fold clean).**
**Design lineage:** chunk-5 architect design (citations verified against `f31c1d6`). The LARGE/COMPLEX
final feature cycle; the pre-2025 residue math is the hard part.

**Goal.** A new TUI decision flow `safe-harbor-allocate` (`A`) that CREATES a `SafeHarborAllocation` —
the §7.4 pre-2025 Universal-residue snapshot @ 2025-01-01. Counterpart to the shipped chunk-3
`safe-harbor-attest` (`a`, which CURES a created allocation): create with `A`, cure with `a`.
**Creation yields a REVOCABLE allocation** (`timely_allocation_attested: false`) — voidable while inert —
so NO typed-word gate (unlike attest). Attesting it (the `a` flow) makes it §7.4-irrevocable.

**SemVer.** New `pub fn Session::safe_harbor_residue` (`btctax-cli`, additive read helper) + refactor
`cmd::reconcile::safe_harbor_allocate` to call it (internal, behavior-preserving); new
`persist_safe_harbor_allocate` + flow/modal structs + key `A` (`btctax-tui-edit`). `btctax-core`
UNCHANGED (all types/fns pre-exist). **MINOR/additive** (cli + tui-edit, matching the chunk-4b
optimize-accept precedent). **No lockstep** (no clap flags; a Session read helper is flag-free).

---

## Grounding (verified at `f31c1d6`)

- **CLI:** `cmd::reconcile::safe_harbor_allocate(vault, pp, method: AllocMethod, attested, now)`
  (`reconcile.rs:250-323`): (1) gates on `config.pre2025_method_attested` (`:264-279`); (2) builds a
  **pre-2025-only subset** — imports with tax-date `< 2025-01-01` + ALL reconciliation decisions, DROP
  any prior `SafeHarborAllocation` (`:281-291`); (3) `project()`s the subset, `residue.lots.filter
  (remaining_sat > 0)` → `AllocLot` (`:293-305`); (4) refuses if empty (`:306-310`); (5) appends ONE
  `SafeHarborAllocation` via `append_and_save` (`:26-34`). **The CLI does NOT guard a pre-existing live
  allocation.**
- **Payload** (`event.rs:145-174`): `AllocMethod{ActualPosition,ProRata}`;
  `AllocLot{wallet,sat,usd_basis,acquired_at,dual_loss_basis:Option<Usd>,donor_acquired_at:Option<TaxDate>}`;
  `pre2025_method: LotMethod` (`#[serde(default)]→Fifo`).
- **Residue engine** (`project/transition.rs`): `universal_snapshot` (`:32-72`, method-aware conservation
  ref); `seed_transition` Path A (`:75-103`, moves each Universal lot to its wallet 1:1). The residue
  subset (no live allocation in-subset) projects Path A → `residue.lots` totals == `universal_snapshot`
  → the created allocation conserves. KAT-locked at `crates/btctax-cli/tests/reconcile.rs:570`.
- **Effectiveness/blockers** (`resolve.rs:826-968`): `timebarred = !attested && (made > bar ||
  method==ProRata)`; `bar = ActualPosition→min(first_2025_disposition, TY2025_RETURN_DUE=2026-04-15),
  ProRata→max(...)`. `SafeHarborUnconservable` (`:882-889`). **Void of an EFFECTIVE allocation →
  `DecisionConflict` (`:926-934`); multiple effective → `DecisionConflict` (`:958-966`).**
  `TRANSITION_DATE=2025-01-01` / `TY2025_RETURN_DUE=2026-04-15` (`conventions.rs:17,19`).
- **Session** (`session.rs`): `config:90`, `load_events_and_project:137`, and the READ-HELPER precedent
  `optimize_proposal:158` (recompute, appends nothing). **#7** void pre-filter (`main.rs:2544-2560`)
  excludes only *effective* allocations; inert ones stay voidable. `is_revocable_payload` includes
  `SafeHarborAllocation` (`form.rs:841`). Free Browse key: **`A`** (only capital bound is `G`).

---

## D1 — eligibility pre-filter (`open_safe_harbor_allocate_flow`)

Model on the attest opener (`main.rs:4657`) + optimize-accept opener (`:5259`). In order:
1. **Latch:** `if let Some(s) = app.residue_latch_status() { app.status=Some(s); return }`.
2. **Snapshot:** `if app.snapshot.is_none() { return }`.
3. **pre-2025 method declared+attested** (CLI-parity, `reconcile.rs:264-279`): `session.config()?` (clean
   call, no `conn(`); if `!cfg.pre2025_method_attested` → status `"Declare your filed pre-2025 method
   first — quit the editor, then run: btctax config --set-pre2025-method <m> --attest-pre2025-method"`,
   return.
4. **Residue non-empty:** `let (lots, pre2025_method) = session.safe_harbor_residue()?` (D3). Empty
   `lots` → status `"No pre-2025 lots to allocate (Path A applies; safe harbor unnecessary)"`, return.
   `Err(e)` → `"Pre-flight residue error: {e}"`, return.
5. **No existing LIVE allocation** (TUI-added guard the CLI lacks — prevents the chunk-3 "Multiple live
   allocations present" tangle, `main.rs:4712`): build the voided set; scan `snap.events` for a
   non-voided `SafeHarborAllocation`; if any → status `"An allocation already exists — attest it with
   'a', or void it with 'v' before creating a new one"`, return.
6. Else: use the `pre2025_method` RETURNED by the helper [R0-M1] (structurally the one the residue was
   computed under), compute totals from `lots`, open the flow at Preview with `method =
   AllocMethod::ActualPosition` (default).

---

## D2 — form / flow (Preview)

The ONLY user input is **`method: AllocMethod`**. `pre2025_method` is read from config (CLI parity,
`reconcile.rs:320`), NOT a form field. `attested` is hard-coded `false`. `as_of_date`/`lots` are
computed. **No free-text, no typed-word.** **Residue is method-INDEPENDENT (gotcha G3): compute once at
open; the `method` toggle changes only the recorded tag, not the displayed lots.**

New `edit/form.rs` types (mirror `OptimizeAcceptFlowState`/`ModalState` `:1547-1569`):
```rust
pub enum SafeHarborAllocateStep { Preview }
pub struct SafeHarborAllocateFlowState {
    pub lots: Vec<AllocLot>, pub total_sat: Sat, pub total_basis: Usd,
    pub method: AllocMethod, pub pre2025_method: LotMethod,
    pub list: TargetList<AllocLotRow>, pub step: SafeHarborAllocateStep,
}
pub struct SafeHarborAllocateModalState {
    pub lots: Vec<AllocLot>, pub total_sat: Sat, pub total_basis: Usd,
    pub method: AllocMethod, pub pre2025_method: LotMethod, pub lot_count: usize,
}
```
`EditorApp` gains `safe_harbor_allocate_flow`/`_modal: Option<...>` (`None` in `new()`, cleared in
`close_all_mutation_surfaces`). Wire `A` into Browse dispatch, the modal into modal-dispatch (alongside
optimize-accept), the flow into flow-dispatch (before attest), and a `draw_edit.rs` overlay.

**Preview keys** (`handle_safe_harbor_allocate_flow_key`): `Tab`/`←`/`→` → cycle `method` (add
`cycle_alloc_method`, mirror `cycle_filing_status` `:415`); `k`/`j`/`g`/`G` scroll the lot list; `Enter`
→ open the confirm modal; `Esc` → close flow; all else incl. `q` swallowed.

**Preview display** (`draw_safe_harbor_allocate_preview`, model on `draw_optimize_accept_list` +
`draw_attest_info`): header `SAFE-HARBOR ALLOCATE — pre-2025 Universal residue snapshot @ 2025-01-01`;
method line (live toggle) + `pre-2025 method (recorded): FIFO`; scrollable lot table (wallet · sat/BTC ·
usd_basis · acquired_at · [dual-loss] [donor date] — dual columns only when `Some`); **totals footer**
(`N lots · Σ BTC · Σ basis`); framing `Creates a REVOCABLE allocation (unattested). TIMEBARRED until you
attest with 'a'. Void with 'v' while inert.`; hint `Tab: method  ↑/↓: scroll  Enter: confirm  Esc: cancel`.

---

## D3 — residue helper `Session::safe_harbor_residue` (btctax-cli, additive)

Modeled EXACTLY on `optimize_proposal` (`session.rs:158`). Keeps the delicate pre-2025 subset logic in
ONE place (btctax-cli), shared by the CLI command and the TUI opener, and KAT-G1-clean at the TUI call
site (the tui-edit call is `session.safe_harbor_residue()` — none of the persist-only tokens
`conn(`/`save(`/`append_`/`restore(`/`tax_profile::set`/`donation_details::set`/`optimize_attest::set`
appears in `btctax-tui-edit`) [R0-N1: `load_all`/`project` are NOT gated tokens]. **It RETURNS the
`LotMethod` it computed the residue under [R0-M1]**, so the caller records the SAME method structurally
(no second config read that could diverge from the residue's):
```rust
/// READ-ONLY: the 2025-01-01 pre-2025 Universal residue as AllocLots, plus the `pre2025_method`
/// (LotMethod) it was computed under. Appends/persists NOTHING. The single source of the pre-2025
/// subset, shared by cmd::reconcile::safe_harbor_allocate + the TUI allocate opener.
pub fn safe_harbor_residue(&self) -> Result<(Vec<AllocLot>, LotMethod), CliError> {
    let cfg = self.config()?;
    let pre2025_method = cfg.pre2025_method;                 // recorded field == the one used below
    let proj = cfg.to_projection();
    let pre2025: Vec<LedgerEvent> = load_all(self.conn())?.into_iter()
        .filter(|e| match &e.id {
            EventId::Import { .. } => tax_date(e.utc_timestamp, e.original_tz) < TRANSITION_DATE,
            _ => !matches!(e.payload, EventPayload::SafeHarborAllocation(_)),
        }).collect();
    let prices = BundledPrices::load()?;
    let residue = project(&pre2025, &prices, &proj);
    let lots = residue.lots.iter().filter(|l| l.remaining_sat > 0).map(|l| AllocLot {
        wallet: l.wallet.clone(), sat: l.remaining_sat, usd_basis: l.usd_basis,
        acquired_at: l.acquired_at, dual_loss_basis: l.dual_loss_basis,
        donor_acquired_at: l.donor_acquired_at,
    }).collect();
    Ok((lots, pre2025_method))
}
```
**Refactor `cmd::reconcile::safe_harbor_allocate` (`:281-310`) to call it** and record the RETURNED
`pre2025_method` in the appended payload (DRY, behavior-preserving; the `pre2025_method_attested` gate +
empty-check + payload build stay in the command). Existing reconcile tests pin the behavior; the new
`safe_harbor_residue_matches_command_lots` KAT pins BOTH the lots AND that the returned method equals
the recorded `pre2025_method` [R0-M1].

---

## D4 — confirmation modal (revocable framing; NOT typed-word)

`Enter` on Preview → `open_safe_harbor_allocate_modal` (captures lots/method/pre2025_method/totals).
`draw_safe_harbor_allocate_modal` shows the full payload:
```
Create SAFE-HARBOR ALLOCATION?
  method          : ActualPosition
  pre-2025 method : FIFO   (recorded, immutable)
  as_of_date      : 2025-01-01
  lots            : 1  (Σ 0.20000000 BTC, Σ basis $8,550.00)
  timely_attested : false  → REVOCABLE
This is a REVOCABLE snapshot: voidable ('v') while inert, TIMEBARRED until you attest ('a',
which makes it §7.4-IRREVOCABLE).
Enter: create    Esc: cancel
```
Keys (`handle_safe_harbor_allocate_modal_key`, model `:5488`): `Enter` → persist; `Esc` → close modal
only (back to Preview); all else swallowed. **No typed-word** (creation is reversible; contrast attest's
`ATTEST`).

---

## D5 — `persist_safe_harbor_allocate` (single append via save_or_rollback)

Standard single-append template (`persist_reclassify_outflow:158-173`) — NOT the attest special-case
(no latch; a single append rolls back cleanly, errors route through `on_persist_error`):
```rust
pub fn persist_safe_harbor_allocate(session: &mut Session, lots: Vec<AllocLot>, method: AllocMethod,
    pre2025_method: LotMethod, now: OffsetDateTime) -> Result<EventId, PersistError> {
    let pre = session.snapshot()?;
    let payload = EventPayload::SafeHarborAllocation(SafeHarborAllocation {
        lots, as_of_date: TRANSITION_DATE, method,
        timely_allocation_attested: false, pre2025_method,
    });
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    save_or_rollback(session, pre)?;
    Ok(id)
}
```
**No side-table.** Modal Enter arm: `Ok(id)` → re-project via `build_snapshot` → status from
`derive_allocate_status` → close modal+flow; `Err(e)` → close modal + `app.on_persist_error(e)`.

---

## D6 — `derive_allocate_status` (keyed to new_id; [R0-M10] discipline, mirror `derive_attest_status:4912`)

Priority: (1) `SafeHarborUnconservable` on new_id (defensive) → `"Created, but SafeHarborUnconservable
fired — see Compliance; void ('v') and re-run."` (2) `DecisionConflict` on new_id OR the `event:None`
"multiple effective" conflict (`resolve.rs:962`) [R0-N2: the `event:None` read is a DELIBERATE exception
to the new_id-only discipline — the multiple-effective conflict has no single owning id; defensive,
stale-free, and normally unreachable given the step-5 guard] → `"Created, but conflicts with an existing
effective allocation — void one ('v') (see Compliance)."` (3)
**`SafeHarborTimebar` on new_id — the EXPECTED arm** → `"Allocation created (REVOCABLE, timebarred) —
attest with 'a' to make it effective, or void with 'v'."` (4) Clean (no timebar; unreachable for a fresh
allocation at the current date — G2) → `"Allocation created and EFFECTIVE (Path B) — it can no longer be
voided; attest with 'a' to lock §7.4."`

---

## Gotchas (load-bearing — for the reviewer)

- **G1 — voidability tracks EFFECTIVENESS, not attestation.** `resolve.rs:926-934` fires
  `DecisionConflict` on a void of an *effective* allocation regardless of `timely_allocation_attested`.
  So a created allocation is voidable **iff inert** (timebarred OR unconservable) — the `#7` void
  pre-filter (`main.rs:2558`) already encodes exactly this. Framing conditions voidability on "while
  inert/timebarred" (done in D4/D6). **Do NOT tell the user a created allocation is unconditionally
  voidable.**
- **G2 — at the current date the point is moot in the safe direction.** `TY2025_RETURN_DUE=2026-04-15`;
  today ≥ 2026-07. A fresh allocation's made-date `> bar` (ActualPosition), and ProRata is always
  timebarred when unattested → **every freshly-created allocation today is timebarred → inert →
  voidable → status arm 3.** Arm 4 (immediately-effective) is currently unreachable but kept correct
  (do NOT delete it as dead code).
- **G3 — ProRata is not truly implemented** (`reconcile.rs:246-248` open question O4). Both
  `ActualPosition` and `ProRata` seed from the SAME per-wallet actuals — the recorded `method` changes
  ONLY the timebar/effectiveness rule (ProRata⇒always-timebarred-unless-attested), NEVER the displayed
  lots. The residue preview is identical across the toggle. **The modal must NOT imply ProRata
  redistributes basis cross-wallet — it does not yet.** (Out of scope; matches core.)
- **G4 — residue subset drops prior allocations → idempotent/allocation-independent**
  (`reconcile.rs:240-241`). A second identical allocation is never useful → the step-5 guard-and-refuse
  is correct (no dedup needed).
- **G5 — `pre2025_method` immutability.** The recorded `pre2025_method` MUST equal the config method the
  residue was computed under (else `Pre2025MethodConflictsAllocation`, `resolve.rs:946-955`). Capture at
  open, thread unchanged through flow→modal→persist. No in-editor `pre2025_method` writer (CLI-only,
  `config.rs:123`), so the capture is race-free.
- **G6 — do the residue math ONLY through `session.safe_harbor_residue()`** (never inline in the opener
  — would trip KAT-G1 and duplicate the subset). `session.config()` is KAT-G1-clean.

---

## KATs

**btctax-cli:** `safe_harbor_residue_matches_command_lots` (helper output == the lots the CLI appends;
guards the DRY refactor; reuse `reconcile.rs:570`'s vault). Existing reconcile.rs:570/:733/:828 stay green.
**edit/persist.rs:** `kat_persist_allocate_single_append_strict_prefix` (EXACTLY one
`SafeHarborAllocation`, `timely_allocation_attested==false`, tail-appended); `kat_persist_allocate_rolls_
back_on_failed_save` (chmod → `Err(RolledBack)`, log unchanged, retry clean).
**main.rs opener/eligibility:** `kat_allocate_refuses_when_pre2025_method_unattested`;
`kat_allocate_refuses_when_live_allocation_exists` (pre-seed one → step-5 fires, no second append);
`kat_allocate_noop_when_residue_empty`; `kat_allocate_latch_refuses` (`rollback_failed=true`).
**main.rs E2E:** `kat_e2e_allocate_then_attest` (`A`→`a` → effective, no `SafeHarborTimebar` on attested
id); `kat_e2e_allocate_then_void` (`A` → inert allocation listed by `v` (#7 keeps inert) → voids cleanly,
no `DecisionConflict`); `kat_allocate_status_timebarred` (arm 3 at a 2026 made-date).
**KAT-G1** stays green (no forbidden token in tui-edit; residue via the Session helper; single append via
`append_`).

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)

- **Task 1 — the residue helper + CLI refactor** (`Session::safe_harbor_residue` + refactor
  `cmd::reconcile::safe_harbor_allocate` to call it; `safe_harbor_residue_matches_command_lots` KAT;
  existing reconcile tests stay green). Lowest-risk, shared foundation.
- **Task 2 — the TUI flow** (`A` opener + eligibility pre-filter; Preview form + method toggle; modal;
  `persist_safe_harbor_allocate`; `derive_allocate_status`; draw overlays; the persist + opener +
  status KATs).
- **Task 3 — the E2E round-trips + whole-diff review (Phase E) + FOLLOWUPS** (allocate→attest,
  allocate→void; record the ProRata-not-redistributing FOLLOWUP (matches core O4)).

## Out of scope
- ProRata cross-wallet basis redistribution (core O4 — not implemented in the engine; the TUI records
  the method tag but shows actuals; FOLLOWUP tracks the core gap).
- In-editor `pre2025_method` declaration (CLI-only; the opener directs the user to the CLI).
- Any `btctax-core` change; viewer changes.
