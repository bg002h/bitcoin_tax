# UI re-add sketch: a swappable interactive editor (incl. Defensive Filing)

Advisory sketch, not a plan. Question answered: how to bring back an interactive editing UI —
including the declare/promote flows — so that the UI layer is genuinely swappable and a web UI
(or a third front end) reuses the decision logic instead of reimplementing it.

Grounding: `design/arch-engine-keep-wizard-cut/COMPARISON.md` (measurements taken as given),
plus direct reads of `btctax-input-form/src/seam.rs`, `btctax-tui-edit/src/edit/{persist,form,tax_inputs}.rs`,
`btctax-tui-edit/src/{main,editor}.rs`, `btctax-cli/src/chokepoint/mod.rs`,
`btctax-core/src/defensive/mod.rs`, `docs/architecture/ARCHITECTURE.md`, and the deleted wizard
files read from `main` via `git show`. Line numbers cited against the current branch
(`arch/engine-keep-wizard-cut`) unless marked `main@`.

---

## 1. Recommended branch: `arch/engine-keep-wizard-cut`

**Decisive reason:** the two things this architecture must *delete* are already deleted there, and
everything it must *reuse* is intact there. The re-layering below replaces exactly the two
subsystems the cut branch removed — the keypress-fused confirm tails and the generalized
stale-snapshot latch (superseded here by a typed generation check, §2.4) — and keeps exactly what
the cut branch kept: the engine, `chokepoint/`, `edit/persist.rs`, `edit/form.rs`,
`edit/tax_inputs.rs`, and the new `defensive status` renderer (which becomes the first proof that
the dashboard view-model renders without a TUI). Building on `main` means first porting the latch
subsystem and its four mechanized source-scanning guards through the crate split, then deleting
them anyway. Building on `backout/pre-approach-b` throws away the settled engine and is a
non-starter.

The wizard flows are not lost by this choice: `main@crates/btctax-tui-edit/src/edit/declare_flow.rs`
and `main@…/promote_flow.rs` contain **zero** ratatui/crossterm code (their only grep hits are
comments saying "no ratatui dependency here — draw_edit.rs wraps these lines in a Paragraph",
`main@declare_flow.rs:332`, `main@promote_flow.rs:227`), and `main@…/defensive_dashboard.rs`'s only
coupling is one `crossterm::event::{KeyCode, KeyEvent}` import (`main@defensive_dashboard.rs:25`)
for a handler that, per its own module doc, "only NAMES the intent a key press represents
(`DashboardIntent`)". They restore near-verbatim from git history onto the new seam (§3).

**Strongest argument against this choice:** those 3,901 lines of already-UI-agnostic wizard state
machines are *live and tested* on `main` and merely *recoverable* on the cut branch — delete-then-
restore is churn, and the restored files must be re-reviewed as if new. That is a real cost
(≈ 2–4 tasks of the estimate in §5). It is smaller than the cost `main` imposes: carrying the
stale-latch subsystem, its 27 instrumented write tails, and ~1,200 gross lines of wizard key
dispatch in `main.rs` through a crate split they do not survive. Accepting this sketch also
implicitly resolves the cut branch's merge question in its favor.

---

## 2. The seam

### 2.0 The reference pattern, and how this generalizes it

`btctax-input-form` already is the seam in miniature (`ARCHITECTURE.md:56`, `:358-363`): stable
identity enums (`SectionId` at `seam.rs:14`, `FieldId` at `seam.rs:31`), an owned serde command
type (`Edit`, `seam.rs:216` — "the web wire"), a pure apply, and a UI-free render projection its
TUI consumer derives (`Pane`/`focused_field`, `edit/tax_inputs.rs:38`, `:125`) so "the render and
the nav can never disagree" (`tax_inputs.rs:33`). The generalization is one sentence: **do to the
whole editor what `Edit` + `Pane` did to tax-inputs** — a stable `ActionId` where input-form has
`FieldId`, an editor-wide serde `Cmd` where it has `Edit`, an editor-wide `ViewModel` where it has
`Pane`, and — the one genuinely new piece — a read-only availability/candidates query that the
current `open_*_flow` fusion makes impossible. `btctax_input_form::Edit` is *embedded verbatim* as
one `Cmd` variant, so there is exactly one field-level wire, never two competing ones.

### 2.1 Where it lives

A new app-layer crate, **`btctax-edit`**, slotted into the existing DAG between the CLI library
and the front ends:

```
btctax-edit      -> btctax-cli (Session, chokepoint), btctax-input-form, btctax-core, btctax-adapters
btctax-tui-edit  -> btctax-edit, btctax-tui (rendering + unlock chrome only)
btctax-web (future) -> btctax-edit only
```

`btctax-edit` links **no ratatui/crossterm** — enforced structurally by a `cargo tree` gate
modeled on the existing net-isolation check (`ARCHITECTURE.md:500-505`), not by discipline.
One relocation this forces: `Snapshot` and `build_snapshot` currently live in the viewer crate
(`btctax-tui/src/app.rs:104`) but are pure data (events, state, config, profiles, tables, prices —
no UI types); they move below the UI line (into `btctax-cli` next to `Session`, or into
`btctax-edit`). `edit/persist.rs`, `edit/form.rs`, and `edit/tax_inputs.rs` have zero
`btctax_tui` references (verified by grep) and move without surgery, except one line:
`form.rs:15` imports `ratatui::widgets::TableState` for `TargetList` (`form.rs:504`); it becomes a
plain `cursor: usize` and the TUI derives a `TableState` at render time.

### 2.2 Who owns state

```rust
/// One per unlocked vault. Owns the live Session — and therefore the VaultLock — exactly as
/// EditorApp does today (editor.rs:66, :89). The UI above it owns ONLY presentation state
/// (scroll offsets, styles, terminal size); a web backend owns only transport.
pub struct EditSession {
    session: btctax_cli::Session,
    snapshot: Snapshot,          // re-projected after every commit
    gen: SnapshotGen,            // bumped on every re-projection (§2.4)
    latch: Residue,              // rollback_failed / attest_save_failed, moved from EditorApp (editor.rs:271,:276)
    flow: Option<Flow>,          // ONE field — the "at most one flow is Some" invariant
                                 // (editor.rs:116) becomes typed instead of conventional
    clock: Clock,                // the existing seam (editor.rs:284)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SnapshotGen(u64);

/// The ~27 flow states, one enum instead of EditorApp's ~30 Option fields (editor.rs:112-266).
/// The payload types are today's *FlowState structs from edit/form.rs, moved as-is.
pub enum Flow {
    Void(VoidFlowState),
    ClassifyInbound(ClassifyInboundFlowState),
    /* … the other general-editor flows … */
    TaxInputs(TaxInputsFormState),
    DefensiveDashboard(DefensiveDashboardState),   // restored, §3
    Declare(DeclareFlowState),                     // restored, §3
    Promote(PromoteFlowState),                     // restored, §3
}
```

### 2.3 The read plane — the named gap, closed

Today `open_void_flow` (`main.rs:3923`) fuses the residue-latch check (`:3924-3927`), the
candidate computation (`:3938-3956`, via the already-pure `btctax_core::voidable_decisions`), and
an `EditorApp` mutation (`:3966-3969`) into one function reachable only by calling it. The fix is
to make the first two a total, read-only query and leave the third to the write plane:

```rust
/// Stable action identity — the ActionId is to verbs what FieldId is to fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionId {
    ClassifyInbound, ReclassifyOutflow, ReclassifyIncome, SetFmv, Void, SelectLots,
    LinkTransfer, ClassifyRaw, MethodElection, SafeHarborAllocate, SafeHarborAttest,
    ResolveConflict, OptimizeAccept, SetDonationDetails, MatchSelfTransfers,
    BulkLink, BulkSelfTransferIn, BulkClassifyIncome, BulkResolve, BulkVoid,
    BulkReclassifyOutflow, PseudoApprove, TaxProfile, TaxInputs,
    DefensiveDashboard, DeclareTranche, PromoteTranche, ExportYear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Availability {
    /// Legal now; `candidates` is the same count the flow's list step would show.
    Ready { candidates: usize },
    /// Legal in principle, nothing to act on ("No revocable decisions to void", main.rs:3962).
    Empty { note: String },
    /// Illegal now, with the same reason string the opener shows (residue latch,
    /// pseudo-active for the defensive actions, allocation-in-force, …).
    Refused { reason: String },
}

impl EditSession {
    /// Total over ActionId; pure over (snapshot, latch, flow). THE query a second front end
    /// polls to decide what to render — no speculative opener call, no duplicated legality.
    pub fn availability(&self, action: ActionId) -> Availability;

    /// The typed candidate rows (VoidListItem, InboundListItem, … — the form.rs item structs,
    /// which are already display-ready data). Same filter chain the opener uses, by construction:
    /// the opener will be *implemented as* query-then-construct-flow.
    pub fn candidates(&self, action: ActionId) -> CandidateSet;

    /// The serde render model of everything visible (open flow step, its fields/buffers/errors,
    /// status line, banners). The TUI renders it in-process; a web backend serializes it.
    pub fn view(&self) -> ViewModel;
}
```

`journey_view` (`btctax-core/src/defensive/mod.rs:630`) is the existence proof for this plane: a
pure "what can I do and with what candidates" over `(events, state, prices, tables, cfg, year)`,
already consumed by two renderers (the retired dashboard and the cut branch's
`render_defensive_status`). `availability`/`candidates` is that same idea extended to all ~27
verbs, extracting the first halves of the 25 openers.

### 2.4 The write plane — serde intents, and a generation-checked commit

```rust
/// The editor-wide wire. Everything a front end can DO, as data. Mirrors Edit (seam.rs:216).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Cmd {
    Open(ActionId),                       // availability-gated flow construction
    Nav(NavCmd),                          // Up | Down | Top | Bottom | Select(usize) — generic over
                                          // TargetList; kills the copy-pasted half of every
                                          // handle_*_list_key (cf. main.rs:3236-3286)
    Field { value_edit: FieldEdit },      // FieldBuffer push/pop/clear — shared, like FieldCap today
    Flow(FlowCmd),                        // per-flow semantic steps (the real decisions):
                                          //   PickEra(EraPreset), SetProvenance(ProvenanceKind),
                                          //   RequestTaxDelta, EnterRow(usize), ChooseKind(…), …
    TaxInputs(btctax_input_form::Edit),   // the existing wire, embedded VERBATIM
    Confirm,                              // plan at first Confirm; commit at the gated second
    Cancel,
}

impl EditSession {
    /// The ONLY entry point — the seam's handle_key. Every UI translates its native input into
    /// Cmd (the TUI: one KeyEvent→Cmd table; a web UI: JSON bodies) and renders view() after.
    pub fn handle(&mut self, cmd: Cmd, now: OffsetDateTime) -> Outcome;
}

/// A plan is pinned to the projection generation it was computed from. Commit refuses on
/// mismatch — the v0.10.0 defect class ("a confirm tail computing a filed number from a
/// projection older than the confirming keystroke") becomes UNREPRESENTABLE, replacing the
/// deleted stale-snapshot latch with one typed check instead of source-scanning guards.
struct ArmedPlan { kind: PlanKind, gen: SnapshotGen }
// commit path: assert plan.gen == self.gen  →  edit::persist::persist_*(…)  →  re-project
//              →  self.gen.0 += 1  →  derive_*_status  →  ViewModel
```

The persist layer is unchanged in shape — every mutation stays a plain
`fn(&mut Session, payload, now) -> Result<EventId, PersistError>` (`edit/persist.rs:285`), and the
chokepoint stays the single front door (`plan_promote` takes `events/prices/cfg`, no `Session` —
`chokepoint/mod.rs:320-328`; `apply_promote` reloads fresh from the session —
`chokepoint/mod.rs:457-466`). Since `handle` is the only entry point and re-projection is
synchronous inside commit, the gen can only mismatch when a write's re-projection failed — exactly
the residual hazard COMPARISON.md identified at the pseudo-approve site — and that mismatch is now
a typed refusal rather than a latch a reviewer must keep wired.

### 2.5 What a web UI actually does on this seam

A localhost backend owns one `EditSession` (the VaultLock exclusivity story maps one-to-one:
one live session, one lock holder — `editor.rs:8-14`), serves `view()` as JSON, accepts `Cmd` as
JSON over HTTP/WebSocket. Zero decision logic in the browser: legality is `availability`,
candidates are `candidates`, transitions are `Flow(FlowCmd)`, and the only mutation is the
gen-checked `Confirm`. The TUI is the same consumer minus serialization.

---

## 3. How declare/promote come back

The concrete case, on the seam:

- **Dashboard.** `Flow::DefensiveDashboard(DefensiveDashboardState)` restored from
  `main@defensive_dashboard.rs` minus its crossterm import — its `DashboardIntent` enum was
  already `Cmd::Flow(...)` avant la lettre ("only NAMES the intent a key press represents").
  Its data is `journey_view`'s output verbatim; `availability(DefensiveDashboard)` returns
  `Refused` on a pseudo-active projection, the same DFW-D11-mirroring gate `defensive status`
  applies today (COMPARISON.md "Added"). The CLI renderer and the TUI screen become the second
  and third consumers of one view — the seam's standing proof.
- **Declare.** `DeclareFlowState` restored near-verbatim (`main@declare_flow.rs` — zero UI code;
  the owner-decision era-pick invariant, the DFW-D5 prefill, and the tax-delta staleness flags all
  carry over untouched). Keys become `FlowCmd`s: `PickEra(EraPreset)`, `NudgeWindow(…)`,
  `RequestTaxDelta` (the old `t` preview — now planned against `self.snapshot` at `self.gen`,
  so the preview-from-stale-projection variant of the defect is also gone), then
  `Confirm` → `plan_declare(target_shortfall = Some(id))` (`chokepoint/mod.rs:513`) → gen-checked
  commit through a restored `persist_declare_tranche` wrapper (the three wizard wrappers were a
  −124-line deletion; they return to the persist module unchanged in shape).
- **Promote.** `PromoteFlowState` restored (`main@promote_flow.rs`): Provenance → Part II →
  Consent, all already engine-gated through `plan_promote`/`Refusal`
  (`chokepoint/mod.rs:104-120`). The consent screen is `render_consent(&plan)`
  (`chokepoint/mod.rs:438`) carried in the ViewModel; the typed acknowledgment phrase arrives as
  `FlowCmd::TypeAck(String)` and is enforced where it always was — inside `apply_promote`,
  fail-closed. The `PromotePlan` is armed with its `SnapshotGen`; the shipped v0.10.0 Critical
  (`promote_flow_confirm` building `Acknowledgment.shown_terms` off a stale `app.snapshot`)
  cannot recur because a plan cannot cross a gen bump.
- **Export.** Per the owner's ruling, the composed multi-year export stays dead. The dashboard's
  export action is per-year (`ActionId::ExportYear`), matching what `defensive status` already
  tells filers to do.

Net: the flows return as *data machines behind the seam*, driven by any front end; what does NOT
return is the ~1,200 gross lines of wizard key dispatch in `main.rs` and the entire latch
subsystem — their jobs are done by `Cmd` translation and `SnapshotGen`.

---

## 4. Reuse ledger

Lines are COMPARISON.md's measurements unless marked (est.).

| Asset | Lines | Fate |
|---|---:|---|
| `btctax-input-form` (whole crate) | 4,603 | **As-is** — embedded as `Cmd::TaxInputs(Edit)`; the wire, untouched |
| `btctax-cli/src/chokepoint/` (`plan_*`/`apply_*`) | ~1,050 | **As-is** — the fixed front door; `plan_export`/`apply_export` stay dead per owner ruling |
| `btctax_core::defensive` (`journey_view`, discovery, era) | pure | **As-is** |
| `derive_*_status` fns (20) | 794 | **As-is** — move to `btctax-edit`, become the Outcome/status producers |
| `edit/persist.rs` | 5,157 | **Mechanical** — crate move + Snapshot import path; +124 lines of wizard wrappers restored |
| `edit/form.rs` (~40 `*FlowState`/`*ModalState`) | 3,966 | **Mechanical** — crate move; `TableState` → `cursor: usize` (`form.rs:15`, `:504`) |
| `edit/tax_inputs.rs` (`Pane`/`focused_field`) | 1,210 | **As-is** — crate move only |
| `btctax_tui::app::Snapshot` + `build_snapshot` | ~200 (est.) | **Mechanical** — relocate below the UI line (pure data, `btctax-tui/src/app.rs:104`) |
| `main@` `declare_flow.rs` + `promote_flow.rs` | 2,341 | **Restore near-verbatim** (zero UI code); key drivers become `FlowCmd` arms |
| `main@` `defensive_dashboard.rs` | 1,560 | **Restore, small change** — drop the crossterm import; `DashboardIntent` ≈ already `FlowCmd` |
| `main.rs` `handle_*_key` (~40 fns) | 5,468 | **Rewrite as Cmd dispatch** — substantially compressed (generic `Nav` + shared `Field` kill the copy-pasted halves) |
| `main.rs` `open_*` (25) | 2,294 | **Rewrite as two halves** — pure `availability`/`candidates` + thin flow constructors |
| `draw_edit.rs` | 7,391 | **Kept, TUI-only** — mechanically re-pointed at seam state/ViewModel; ~1,169 lines of wizard screens restored from `main@` |
| Wizard-era tests (`main@` main.rs/draw) | ~3,473+ | **Re-target** — KeyEvent-driven KATs become Cmd-driven (simpler harness than synthetic key events) |
| Residue latch (`rollback_failed`/`attest_save_failed`) | small | **As-is** — moves onto `EditSession`; predates the wizard, serves all flows |
| Stale-snapshot latch subsystem | 0 (deleted) | **Not restored** — superseded by `SnapshotGen` (§2.4) |

---

## 5. Size and time

All figures are **estimates**; basis stated. Unit of work: one agent-driven, review-gated task
landing ≈ 1,000–1,700 lines including tests (basis: the wizard release added 16,718 lines across a
10-task plan, v0.9.0 → v0.10.0 in five calendar days at this repo's demonstrated cadence).

| Phase (§6) | New | Moved/mech. | Restored | Deleted | Basis |
|---|---:|---:|---:|---:|---|
| P1 crate split | ~300 | ~11,300 | — | — | persist+form+tax_inputs+status+Snapshot; TableState swap; tree gate |
| P2 read plane | 1,000–1,500 | — | — | — | ~27 availability arms × 30–60 lines, extracted from opener bodies |
| P3 write-plane core + 3 pilots | 1,500–2,300 | — | — | ~900 | Cmd/ViewModel/gen protocol ~800–1,200; pilots port their handlers |
| P4 remaining flows | 3,000–4,500 | — | — | ~6,900 | replaces the rest of 7,762 dispatch+opener lines at ~55–70% volume after Nav/Field compression |
| P5 defensive restore | 400–800 | — | ~5,200 | — | flows+dashboard+draw+wrappers from `main@`; FlowCmd adaptation |
| P6 headless proof | 300–600 | — | — | — | JSON script driver, no ratatui linkage |
| ViewModel derivation (across P3–P5) | 1,500–2,500 | — | — | — | render-model extraction from draw_edit's data logic |
| Tests (across all) | 6,000–10,000 | KATs move free | — | — | re-target ~137 wizard tests + new query/serde/gen tests |

**Totals (est.): ~8,000–12,200 new production lines, ~11,500 moved with mechanical change,
~5,200 restored, ~7,800 deleted, plus 6–10k test lines.** Net workspace growth vs the cut branch
roughly +14k to +22k.

**Effort: 18–30 review-gated tasks; calendar 2–5 weeks at this project's demonstrated cadence**
(the wizard's 16.7k lines shipped in ~5 days, but this work is more cross-cutting: a crate split
plus re-pointing a 27.5k-line `main.rs`). The spread is driven by, in order: (a) the compression
ratio actually achieved turning `handle_*_key` into `FlowCmd` arms — the 55–70% assumption is the
softest number here; (b) whether `draw_edit.rs` re-points cheaply or fights the ViewModel;
(c) review-round count on P2 and P5 (the two phases touching legality and filed numbers);
(d) how mechanically the KeyEvent test corpus re-targets.

---

## 6. Build sequence

1. **P1 — carve `btctax-edit`.** Move persist/form/tax_inputs/status + Snapshot; swap
   `TableState` for a cursor; add the no-ratatui `cargo tree` gate. *Proves:* `make check` +
   `cargo fmt --all -- --check` green, goldens byte-identical, zero behavior change.
2. **P2 — the read plane.** `ActionId`/`availability`/`candidates` extracted from the 25 openers;
   openers become query-then-construct. *Proves:* an exhaustive parity test — for every
   `ActionId`, the query's verdict equals the opener's observable behavior (incl. the 25-opener
   residue-latch KAT surface, the chain-B lesson).
3. **P3 — the write plane on three pilots** (void = simplest list flow, classify-inbound =
   representative multi-step, tax-inputs = the wire that already exists): `Cmd`, `ViewModel`,
   gen-checked plan/commit; delete the pilots' key handlers in the same commit. *Proves:* the TUI
   drives pilots through `handle(Cmd)` only; serde round-trips; goldens stable.
4. **P4 — port the remaining general-editor flows in small batches**, each batch deleting its
   `handle_*_key`/`open_*` pair as it lands. *Proves:* per-batch KAT parity + a transitional
   source-scan test that no flow is reachable through two dispatchers.
5. **P5 — restore Defensive Filing on the seam** from `main@` history (dashboard, declare,
   promote, persist wrappers, draw screens). *Proves:* the wizard-era KATs re-targeted to Cmd,
   plus one new structural regression test: an armed plan refuses to commit across a gen bump
   (the v0.10.0 Critical, now impossible by type).
6. **P6 — headless second-front-end proof.** A JSON script driver (test-only binary) runs
   unlock → declare → promote → export end-to-end through serde `Cmd`s with ratatui absent from
   its dep tree. *Proves:* the wire is real before any web code exists.
7. **P7 (separate, later project) — the web front end proper**: a localhost server + browser UI
   over `view()`/`handle()`. Out of scope here; P6 is its acceptance test in advance.

---

## 7. What would make this go wrong

1. **A second field wire.** `Cmd` sprouting its own field identifiers for tax-inputs instead of
   embedding `btctax_input_form::Edit` — two competing seams, the exact failure §2.0 forbids.
   *Early warning:* any `FieldId`-like enum appearing in `btctax-edit` that shadows input-form's.
2. **ViewModel bypass.** The in-process TUI quietly reading seam internals the ViewModel doesn't
   carry, so the web UI later gets a poorer, drifting view. *Early warning:* `draw_edit.rs`
   importing anything not reachable from `ViewModel`; mitigate by rendering the goldens *from*
   the ViewModel starting in P3.
3. **Two-dispatcher limbo.** P4 stalls with `handle_key` and `handle(Cmd)` both live for months —
   the worst of both worlds, and the state where the old defect class can return unnoticed.
   *Early warning:* a batch that ports a flow without deleting its key handler in the same commit.
4. **Gen-check erosion.** A commit path accepting a stale generation "because re-planning is
   expensive" — the same rationalization that shipped the v0.10.0 Critical as a follow-up.
   *Early warning:* any second commit entry point, or an `#[allow]`/bypass near the gen compare;
   keep commit a single grep-able chokepoint with one KAT on the refusal.
5. **Web-session scope creep.** Pre-solving remote-session lifecycle (multi-tab, idle timeout,
   lock handoff) inside `btctax-edit`. The seam should stay single-session-single-lock, mirroring
   VaultLock exclusivity; lifecycle is the web backend's problem. *Early warning:*
   `Arc<Mutex<Session>>` or a "session manager" type appearing in the seam crate.

---

## 8. The cheaper alternative considered

**"The CLI is already the UI-agnostic layer — wrap it."** Re-add the wizard as plain TUI code and
let a future web UI shell out to (or link) `btctax-cli`'s verbs. Rejected outright: the CLI is
one-shot and prompt-gated, has no interactive flow state, and its only capability query is
`defensive status`. A web UI on that wire reimplements the 77.9% dispatch-and-decide layer in the
browser — precisely the outcome the question forbids.

**The variant that partially wins, honestly stated:** do P1 + P2 + the `SnapshotGen` commit
protocol, restore the wizard's UI-free flow files behind that thinner seam, and *defer* the full
`Cmd`/`ViewModel` envelope (P3–P4, the ~6–9k-line port) until a second front end is actually
committed. That lands defensive filing back in the TUI in roughly 8–12 tasks instead of 18–30,
keeps the defect class dead (the gen check does not need the Cmd envelope), and banks the three
assets any web project certainly reuses (the crate split, the availability query, the persist/
chokepoint layer). Its cost: general-editor flow *transitions* stay keypress-shaped, so the editor
is swappable-for-defensive-filing but not yet swappable wholesale — and the envelope, built later
without a real second consumer today, risks being shaped slightly wrong either way. If the web UI
is more than ~a year speculative, this staged path is the better bet; the phase order in §6 is
deliberately arranged so that stopping after P2+gen (with P5 pulled forward onto the thin seam) is
a coherent, shippable landing point rather than a failure.
