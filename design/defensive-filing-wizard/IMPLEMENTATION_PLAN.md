# Defensive Filing Wizard — Implementation Plan (Approach-B sub-project 2)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax. **Reviewers:** plan-review r1 =
> Fable (architecture lens) + Opus (tax lens); r2+ = Opus both lenses (user-directed).

**Goal:** A derived "Defensive filing" dashboard in `btctax-tui-edit` that walks a filer whose *sales are
imported but purchases are gone* through covering each `UncoveredDisposal` shortfall with a declared
tranche, forking (file `$0` or promote to a `>$0` floor + Form 8275), and exporting — composing shipped
primitives via new `plan/confirm/apply` chokepoints, with pure `journey_view`/discovery/era in core.

**Architecture:** Three seams (SPEC DFW-D1): (1) `btctax-core` — pure `journey_view` + shortfall discovery
+ era table + the derived advisories; (2) `btctax-cli` — `plan/confirm/apply` chokepoints extracted from
the shipped verbs (the single home of gate ordering + consent render), driven by both the CLI verb and the
dashboard; (3) `btctax-tui-edit` — the dashboard + `*_flow` structs. NO new tax logic; every filing gate
stays engine-enforced.

**Tech Stack:** Rust workspace; `ratatui` (tui-edit); `lopdf` (forms, unchanged); the shipped
`conservative`/`conservative_promote` signals; `rust_decimal`; nextest + `make check`.

## Global Constraints (SPEC §3, verbatim values — every task implicitly includes these)

- **No new tax logic.** Every filed number flows through shipped primitives; new code is gates/refusals/
  derived views only. The ONE intended behavior change is the DFW-D6 pseudo-off fix (a sub-1 bug repair).
- **Chokepoint gate ordering (DFW-D2), written ONCE, MUST match `cmd/promote.rs:378-485` exactly:**
  resolve-live → BG-D5 provenance → BG-D7 Part II → BG-D3 floor/coverage → BG-D6 `consent_terms` →
  synthetic-promote advisory → gift-only relabel → consent render (incl. `wide_window_note`) → **ack
  inside `apply`, fail-closed** → `would_conflict` → append.
- **Consent parity (DFW-D2):** rendered consent copy + advisory/refusal output **byte-identical** between
  CLI and dashboard; `Acknowledgment.shown_terms` (`Vec<ConsentTerm>`) equal by **structural `Eq`**. Parity
  KATs drive **both full driver paths** (CLI verb fn AND TUI persist), incl. refusal paths.
- **Pseudo-off (DFW-D6):** EVERY chokepoint/journey-view shadow projection forces `pseudo_reconcile=false`
  (mirroring `would_conflict`). Journey gated on `!state.pseudo_active()`.
- **Triage total-by-`short_sat` (DFW-D4/D7):** classify on structural `short_sat` presence, never a
  `Blocker.detail` string. `short_sat` is the **per-event aggregate**; clearance is **event-level**.
- **Over-coverage is a dashboard ADVISORY, never a hard refusal (DFW-D5.3):** the shared promote gate is
  behavior-preserving. Scoped to `covered_sat > 0 && live_sat > covered_sat`.
- **Whole-tranche only** (sub-1 non-goal): no partial-sat promote/clamp; remedy for over-size = void +
  re-declare.
- **Write confinement (KAT-G1, `edit/persist.rs:1897`):** in `btctax-tui-edit`, `edit/persist.rs` is the
  ONLY module permitted to name the mutation surface — every `chokepoint::apply_*` call lives in a
  `persist.rs` wrapper; flows/dashboard only COLLECT input + read `plan_*`/`journey_view`. The gate's
  allowlist + guarantee text + self-check are extended in the SAME task that adds each write path.
- **Core takes core types only:** core fns take `&dyn TaxTables` (never the adapters `BundledTaxTables`)
  and never call a `btctax-cli` symbol (no core→cli inversion). The tui-edit drivers reach the chokepoint
  via a `btctax_cli` crate-ROOT re-export (never a `cmd::` path — KAT-G1/the source gate ban it).
- `make check` + `cargo fmt --all --check` + the CI-only jobs (msrv-1.88, net-isolation, pii-scan,
  examples/man drift, forms-census) green; every primitive TDD + mutation-proven; two-lens review to 0C/0I.

---

## File Structure Map

**`btctax-core`** (pure, KAT-able):
- Create `crates/btctax-core/src/defensive/mod.rs` — `journey_view(events, state, prices, tables: &dyn
  TaxTables, cfg) -> DefensiveFilingView` (★ C-1: `&dyn TaxTables`, the core trait — NOT the adapters
  `BundledTaxTables`; every shipped core fn does this. The flavor gate is `tables.table_for(y).is_some()`);
  the `DefensiveFilingView`/`ShortfallCandidate`/`TrancheRow`/`Advisory` types.
- Create `crates/btctax-core/src/defensive/discovery.rs` — the structured shortfall signal + the DFW-D4
  triage classifier (coverable / data-fix / resolve-first).
- Create `crates/btctax-core/src/defensive/era.rs` — the era→window preset table + `era_window(preset)`.
- **★ C-2: move the three pure event-scan predicates into core** — Create
  `crates/btctax-core/src/tranche_guard.rs`: `void_targets(events)`, `in_force_allocation_exists(events)`,
  `pre2025_tranche_exists(events)` (★ tax-N-1: the shipped predicate takes only `events` — NO `we` arg; they
  use only core types + `conventions::TRANSITION_DATE`; today in `cmd/tranche.rs` — `void_targets`:40
  (private), `in_force_allocation_exists`:54, `pre2025_tranche_exists`:71 (the last two `pub` but
  cli-crate-only); `guard_tranche_vs_allocation`:107 + `guard_allocation_vs_tranche`:93 STAY in cli). The
  guards keep their thin `CliError`-wrapping OVER these (single source preserved for the four allocation
  append sites); ★ **arch-I-new-2**: `session.rs:714` is a FIFTH, DIRECT consumer (`pre2025_tranche_exists`,
  not via a guard — the safe-harbor allocate-opener precheck) that rewires to the core predicate too.
  `journey_view` + the declare flow read the core predicate directly.
- Modify `crates/btctax-core/src/lib.rs` — `pub mod defensive; pub mod tranche_guard;`.
- Modify `crates/btctax-core/src/state.rs` + `project/fold.rs` — a derived structured `state.shortfalls`
  populated at the SIX sat-carrying `UncoveredDisposal` emit sites: the `BlockerKind::UncoveredDisposal`
  lines `fold.rs:388`(fee)`,710`(dispose)`,831`(pending-out)`,876`(self-transfer)`,1196`(gift-out)`,1274`
  (donate) — the `short by {shortfall} sat` message is ~2 lines below each (★ arch-m-5: re-grep, don't
  trust the list). Each RAW record carries `{event, wallet, date, principal_sat, fee_sat}` so the
  principal-vs-fee split survives (★ I-4/arch-I-2 — `FeeOnlyPromoteNoop` needs it; note `fold.rs:827`
  already lumps `*sat + fee_sat` into the ONE pending-out blocker, so the split is per-record, not
  uniform across sites); `shortfalls()` aggregates per event into `Shortfall{short_sat = Σ(principal+fee),
  fee_sat = Σ fee}`. `discovery.rs` never parses `Blocker.detail`. Additive/derived only (no filed-number change).

**`btctax-cli`** (the chokepoints — the single home of verb glue):
- Create `crates/btctax-cli/src/chokepoint/mod.rs` — the `plan/confirm/apply` trios for declare / promote /
  export; `PromotePlan`, `DeclarePlan`, `ExportPlan`, `Refusal`; `render_consent(&PromotePlan)->String`.
- Modify `crates/btctax-cli/src/cmd/promote.rs` — reduce `promote_tranche` to a thin driver over the
  chokepoint; apply the DFW-D6 pseudo-off fix.
- Modify `crates/btctax-cli/src/cmd/tranche.rs` — `declare_tranche` thin driver; `plan` takes
  `target_shortfall: Option<EventId>`.
- Modify `crates/btctax-cli/src/cmd/admin.rs` — export driver over the export chokepoint; extract
  `promoted_filing_years(state)`; the fold-diff export year-set. **★ arch-C-1:** ALSO extract the
  crypto-slice body + the full-vs-slice `return_inputs::exists` dispatch out of the self-opening
  `export_irs_pdf` (`:350` — opens its OWN `Session` at `:358`, slice body `:385-599`) into a `&Session`
  inner `export_irs_pdf_from_session(&Session, …)` (mirroring the already-`&Session` `export_full_return:642`),
  so `apply_export` composes over the TUI's already-open `&Session` — a second `Session::open` under the
  editor's held `VaultLock` deadlocks (`session.rs:662`). `export_irs_pdf(vault_path, pp, …)` stays as a
  thin opener over the inner (shipped CLI byte-preserving).
- Modify `crates/btctax-cli/src/lib.rs` — `pub mod chokepoint;` + `pub use` the chokepoint entry points
  WITHOUT a `cmd::` path leak (KAT-G1's `kat_g1_mechanized_source_gate` `everywhere_tokens` forbids `cmd::`
  in `btctax-tui-edit`; `persist.rs:1897`).

**`btctax-tui-edit`** (the dashboard + flows):
- Create `crates/btctax-tui-edit/src/defensive_dashboard.rs` — the dashboard screen (derived rows,
  fork, advisories) + key-dispatch that LAUNCHES the sibling flows. Read-only + dispatch; **NO direct
  write/chokepoint-apply calls** (C-3).
- Create `crates/btctax-tui-edit/src/edit/declare_flow.rs`, `promote_flow.rs` — the `*_flow` structs
  (`.step`), which COLLECT input + read `journey_view`/`plan_*`; the era-preset + live-readout declare UX;
  the promote consent TypedWord gate + Part II authoring. Flows do NOT call `apply_*` directly (C-3).
- **★ C-3: Modify `crates/btctax-tui-edit/src/edit/persist.rs`** — the write-confinement module (KAT-G1,
  `persist.rs:1897`; `edit/mod.rs:1-9` = "`persist` is the ONLY module permitted to name the mutation
  surface"). Add `persist_declare_tranche` / `persist_promote_tranche` / `persist_defensive_export`
  wrappers — these are the ONLY place that calls `chokepoint::apply_declare`/`apply_promote`/`apply_export`.
  Extend KAT-G1's `persist_only_tokens` with `apply_declare(`/`apply_promote(`/`apply_export(`, amend the
  `edit/mod.rs` guarantee text for the editor's new (chokepoint-only) export surface, and plant one new
  token in the G1 self-check.
- Modify `crates/btctax-tui-edit/src/editor.rs` — `EditorScreen::DefensiveFiling`; the flow fields +
  the one-flow debug assertion (M-4); the `!pseudo_active()` entry gate.
- Modify `crates/btctax-tui-edit/src/draw_edit.rs` — render the dashboard + the two flows.

---

## PHASE P-A — the plan/confirm/apply chokepoints (the spine; consent-parity KATs GATE it)

### Task 1 — Extract the PROMOTE chokepoint (plan/confirm/apply, ack inside `apply`) + the DFW-D6 pseudo-off fix

**Files:**
- Create: `crates/btctax-cli/src/chokepoint/mod.rs`
- Modify: `crates/btctax-cli/src/cmd/promote.rs:364-488` (reduce to a thin driver), `crates/btctax-cli/src/lib.rs`
- Test: `crates/btctax-cli/tests/chokepoint_promote.rs` (new); existing `promote_cli.rs` stays green

**Interfaces — Produces:**
```rust
// chokepoint/mod.rs
pub struct PromotePlan {            // everything computed BEFORE the filer types the ack
    pub target: EventId,            // the PromoteTranche decision id
    pub terms: Vec<btctax_core::ConsentTerm>,   // BG-D6 consent_terms output
    // ★ I-1: three ORDERED pieces so render_consent reproduces promote.rs:443-455 byte-for-byte —
    pub advisory_lines: Vec<String>,             // PRE-consent synthetic-promote advisory (promote.rs:443)
    pub gift_only_years: BTreeSet<i32>,          // INPUT to the shipped render_consent(terms, gift_only_years) (promote.rs:333/:453), NOT a pre-rendered string
    pub post_consent_note: Option<String>,       // wide_window_note, printed AFTER consent (promote.rs:454)
    pub payload: btctax_core::EventPayload,       // the PromoteTranche payload to append
}
// ★ arch-m-6/tax-N-1: `Target` covers the resolve-live gate — unknown/voided/WRONG-TYPE target only
// (`resolve_live_tranche`, promote.rs:95-118). Already-promoted (DOUBLE-promote) is NOT caught here; it is
// `would_conflict` at APPLY-time → CliError (promote.rs:475-483), so `would_conflict` is not a plan Refusal.
pub enum Refusal { Target(String), Provenance(String), Coverage(String), PartII(String) }
pub fn plan_promote(events: &[LedgerEvent], prices: &dyn PriceProvider,   // ★ arch-m-new-3: no `state` — the shipped pipeline rebuilds from `events` (promote.rs:364-488)
    cfg: &ProjectionConfig, target: &EventId, provenance: ProvenanceKind, part_ii: &str, now: OffsetDateTime)
    -> Result<PromotePlan, Refusal>;
pub fn render_consent(plan: &PromotePlan) -> String;   // advisory_lines → shipped render_consent(&terms, &gift_only_years) → post_consent_note; byte-== shipped promote.rs:443-455
pub fn apply_promote(session: &mut Session, plan: PromotePlan, acknowledge: Option<&str>, now: OffsetDateTime)
    -> Result<EventId, CliError>;   // ack inside; fail-closed; would_conflict; append
```
`plan_promote` MUST force `cfg.pseudo_reconcile = false` on its own copy before `consent_terms` /
`promote_prior_year_advisory` / `gift_only_flagged_years` (DFW-D6; mirrors `would_conflict`,
`project/mod.rs:118`).

- [ ] **Step 1: Characterization test — pin the shipped promote output BEFORE refactor.** In
  `chokepoint_promote.rs`, build a promoted-disposal vault (reuse `promote_cli.rs`'s
  `build_promoted_vault`) chosen to exercise all THREE ordered pieces (a non-empty synthetic-promote
  advisory, a gift-only prior year, AND a wide window that fires `wide_window_note`), capture the current
  CLI `promote_tranche` **full ordered stdout transcript** (advisory → consent → note) + recorded
  `Acknowledgment.shown_terms`. Assert exact values (copy them from a `cargo run` of the current verb).
- [ ] **Step 2: Run — PASS** (`cargo test -p btctax-cli --test chokepoint_promote pins_shipped_promote`).
- [ ] **Step 3: Write `chokepoint/mod.rs`** — move the promote pipeline (`promote.rs:364-488`) verbatim
  into `plan_promote`/`render_consent`/`apply_promote`, splitting at the ack: everything up to and incl.
  consent computation → `plan_promote`, which captures the THREE ordered pieces the shipped verb prints
  (`promote.rs:443-455`) into the `PromotePlan`: (a) `advisory_lines` = the PRE-consent synthetic-promote
  advisory (`for line in &advisory`, `:443`); (b) `gift_only_years` = `gift_only_flagged_years(...)` — an
  INPUT to the shipped `render_consent(terms, gift_only_years)` (`:333`/`:453`), NOT a rendered string; (c)
  `post_consent_note` = `wide_window_note(...)` (`:454`), printed AFTER consent. ★ **I-1:**
  `render_consent(&plan)` re-emits them in the shipped order — `advisory_lines` → shipped
  `render_consent(&plan.terms, &plan.gift_only_years)` → `post_consent_note` — so the full filer-visible
  string is byte-identical (a single flat Vec CANNOT place `terms` BETWEEN the pre-advisory and the note;
  do NOT collapse the three). Keep the shipped `render_consent(terms, gift_only_years)` in `promote.rs`
  (make it `pub(crate)` and call it from the chokepoint); move `gift_only_flagged_years`/`wide_window_note`
  to the chokepoint (`pub(crate)`). `require_promote_ack` + `would_conflict` + append →
  `apply_promote(acknowledge)`. Add the `pseudo_reconcile=false` copy in `plan_promote`. Map the
  resolve-live gate failure (`:377`) to `Refusal::Target`; `would_conflict` stays inside `apply_promote`
  (→ `CliError`, never a plan `Refusal`).
- [ ] **Step 4: Reduce `cmd/promote.rs::promote_tranche` to a thin driver** — `Session::open` → build
  args → `plan_promote` (map `Refusal` to `CliError`) → `println!(render_consent(&plan))` →
  prompt/collect ack → `apply_promote(session, plan, ack, now)`. No pipeline logic remains in the verb.
- [ ] **Step 5: Run the characterization + full `promote_cli.rs`** — `cargo test -p btctax-cli --test
  chokepoint_promote --test promote_cli` → all PASS (consent string + `shown_terms` unchanged; behavior-
  preserving except pseudo — see Step 6).
- [ ] **Step 6: DFW-D6 pseudo-off KAT (the sub-1 bug fix).** Add
  `pseudo_active_promote_records_honest_terms_not_synthetic`: on a pseudo-active vault whose consent
  figures fold a synthetic default TODAY, assert `apply_promote`'s recorded `shown_terms` are the
  pseudo-OFF three-flavor terms (mutation-verify: remove the `pseudo_reconcile=false` line → the KAT reds
  with synthetic numbers). This is the ONE intended behavior change; note any shipped KAT it flips is the
  buggy one.
- [ ] **Step 7: ack-fail-closed KAT.** `apply_promote(session, plan, None, now)` refuses (distinct from
  `Some("wrong")`); mutation: drop the `require_promote_ack` call → reds.
- [ ] **Step 8: `make check` + `cargo fmt --all`; Commit.**
  `git add crates/btctax-cli/src/chokepoint/ crates/btctax-cli/src/cmd/promote.rs crates/btctax-cli/src/lib.rs crates/btctax-cli/tests/chokepoint_promote.rs`
  `git commit -m "refactor(chokepoint): extract promote plan/confirm/apply + DFW-D6 pseudo-off fix"`

### Task 2 — Extract the DECLARE chokepoint (`plan(target_shortfall: Option<EventId>)` + clearance)

**Files:** Create part of `chokepoint/mod.rs`; Modify `crates/btctax-cli/src/cmd/tranche.rs:120-175`;
Test: `crates/btctax-cli/tests/chokepoint_declare.rs`

**Interfaces — Produces:**
```rust
pub struct DeclarePlan { pub payload: EventPayload }   // a DeclareTranche
pub fn plan_declare(events: &[LedgerEvent], prices: &dyn PriceProvider,   // ★ arch-m-new-3: no `state` — None path is events-only (tranche.rs:134-154); the Some clearance re-projects from events+candidate
    cfg: &ProjectionConfig, sat: i64, wallet: WalletId, ws: Date, we: Date,
    target_shortfall: Option<EventId>, now: OffsetDateTime) -> Result<DeclarePlan, Refusal>;
pub fn apply_declare(session: &mut Session, plan: DeclarePlan, now: OffsetDateTime) -> Result<EventId, CliError>;
```
`plan_declare` gates on the shipped set (`sat>0`, `ws<=we`, `guard_tranche_vs_allocation`) ALWAYS; AND,
**iff `target_shortfall = Some(id)`**, runs the clearance shadow: append the candidate → re-project
(pseudo-off) → assert no `UncoveredDisposal` remains on `id`; else `Refusal::Coverage`. `None` = shipped
semantics byte-for-byte.

- [ ] **Step 1: Characterization** — pin current `declare_tranche` behavior (a `$0` declare succeeds; an
  allocation-conflicting pre-2025 declare refuses) in `chokepoint_declare.rs`.
- [ ] **Step 2: Run — PASS.**
- [ ] **Step 3: Implement `plan_declare`/`apply_declare`** in `chokepoint/mod.rs`; the `None` path replicates
  `cmd/tranche.rs:134-154` exactly; the `Some` path adds the clearance shadow (reuse the `would_conflict`
  shadow-projection pattern; force pseudo off).
- [ ] **Step 4: Reduce `cmd/tranche.rs::declare_tranche` to a thin driver** passing `target_shortfall=None`.
  ★ tax-M-3: `plan_declare` returns a pure `DeclarePlan`, so the shipped phantom-wallet stderr warning
  (`eprintln!`, `tranche.rs:159`) moves to the driver — keep it emitted byte-for-byte on the `None` path
  (`declare_tranche_cli.rs` holds it); it is I/O, not gate logic, and must not migrate into the chokepoint.
- [ ] **Step 5: KATs** — (a) CLI `None` path: a targets-no-shortfall declare is NOT refused (shipped
  preserved); (b) `Some` path: a candidate whose `we == disposal date` fails clearance → `Refusal::Coverage`
  (mutation: prefill `we` before the disposal → passes). ★ arch-I-5: a third KAT — the clearance shadow forces `pseudo_reconcile=false` (a pseudo `SelfTransferMine{$0}` must not mask a real shortfall); AND a candidate that DOES clear → `apply_declare` removes the shortfall row (the cleared-row KAT). Run `declare_tranche_cli.rs` (shipped) → green.
- [ ] **Step 6: `make check` + fmt; Commit** `refactor(chokepoint): declare plan/apply + target-scoped clearance`.

### Task 3 — Extract the EXPORT chokepoint (degenerate trio) + the fold-diff export year-set

**Files:** `chokepoint/mod.rs`; Modify `crates/btctax-cli/src/cmd/admin.rs`, `crates/btctax-core/src/conservative.rs`
(`flagged_years`), `crates/btctax-cli/src/lib.rs` (`IrsPdfReport` re-export — ★ arch-m-new-2); Test: `promote_cli.rs` (extend)

**Interfaces — Produces:**
```rust
// ★ structured year-set (r1): promote_prior_year_advisory returns Vec<String> — unusable; add a typed fn.
// crates/btctax-core/src/conservative.rs (beside promote_prior_year_advisory):
pub fn flagged_years(events: &[LedgerEvent], state: &LedgerState, prices: &dyn PriceProvider,
    tables: &dyn TaxTables, cfg: &ProjectionConfig) -> BTreeSet<i32>;   // BG-D9 fold-diff years, disposal∪removal
// crates/btctax-cli/src/chokepoint/mod.rs:
pub fn promoted_filing_years(state: &LedgerState) -> BTreeSet<i32>;      // extracted from admin.rs:84-98 (8275 gate ONLY)
pub struct ExportPlan { pub years: BTreeSet<i32>, pub out_dir: PathBuf, pub forms: Vec<FormArg> }
pub fn plan_export(events: &[LedgerEvent], state: &LedgerState, prices: &dyn PriceProvider,
    tables: &dyn TaxTables, cfg: &ProjectionConfig, current_year: i32, out_dir: PathBuf, forms: Vec<FormArg>)
    -> Result<ExportPlan, Refusal>;   // gates over state; NO consent/ack; refuses when pseudo_active (DFW-D11)
// ★ arch-C-1: extract EVERYTHING after Session::open (the return_inputs::exists dispatch + BOTH the
// crypto-slice body AND the export_full_return delegation) OUT of the self-opening export_irs_pdf into a
// &Session inner. crates/btctax-cli/src/cmd/admin.rs (the slice branch mirrors export_full_return:642):
pub(crate) fn export_irs_pdf_from_session(session: &Session, state: &LedgerState, events: &[LedgerEvent],
    out_dir: &Path, tax_year: i32, forms: &[FormArg], attest: Option<&str>) -> Result<IrsPdfReport, CliError>;
//   _from_session performs the return_inputs::exists slice-vs-full dispatch (shipped admin.rs:373) INTERNALLY;
//   export_irs_pdf(vault_path, pp, …) becomes a THIN opener: Session::open → export_irs_pdf_from_session.
// ★ arch-n-1: re-export IrsPdfReport at the cli crate root (precedent: pub use cmd::admin::promote_export_gate,
//   lib.rs:37) so persist.rs never names a `cmd::` token (KAT-G1 everywhere_tokens bans cmd::).
pub use crate::cmd::admin::IrsPdfReport;   // crate-root re-export
pub fn apply_export(session: &Session, plan: ExportPlan) -> Result<Vec<IrsPdfReport>, CliError>;  // &Session (export mutates no events)
```
`plan_export.years` = `{current} ∪ { y ∈ flagged_years(...) : y < current }` (DFW-D11; recomputed from state —
the BG-D9 fold-diff over disposal AND removal legs; **strictly ⊇** `promoted_filing_years`). ★ tax-N-1: the
`< current` filter mirrors `promote_prior_year_advisory`'s prior-year filter (`conservative.rs:729`) so a year
≥ current still being AUTHORED is never emitted a premature 1040-X packet — `{current}` supplies the current
year. ★ tax-M-1: `flagged_years` is the UNION of per-promote fold-diffs (matching `promote_prior_year_advisory`'s per-`promote_id` semantics — NOT a
single whole-state with-all-vs-without-all diff, where two promotes' per-year effects could cancel and drop a
year). `apply_export` writes ONE packet per year by calling `export_irs_pdf_from_session(session, …, year, …)`
per year; the full-vs-slice `return_inputs::exists` dispatch lives ONCE INSIDE `_from_session` (exactly as
shipped `admin.rs:373`) — `apply_export` itself does not branch, and no year re-`Session::open`s (a second
open under the TUI's held `VaultLock` deadlocks, `session.rs:662`). `promoted_filing_years` stays the
8275-completeness gate enumeration only — single-sourced into `promote_export_gate`'s `None` arm.

- [ ] **Step 1: Characterization — pin the shipped `export_irs_pdf` packet BEFORE extraction, BOTH dispatch
  arms.** In `promote_cli.rs`, capture the shipped `export_irs_pdf` (self-opening) output for TWO vaults:
  (a) a promoted-disposal vault with NO `return_inputs` (the crypto-slice arm) and (b) a vault WITH
  `return_inputs::exists(year)` (the full-return arm, `admin.rs:373`). Assert each emitted file set + the
  `form_8275.pdf` presence + the `IrsPdfReport` struct (★ arch-m-new-1: pinning the full-return arm proves
  the retained dispatch survives extraction — a slice-only `_from_session` would red case (b)).
- [ ] **Step 2: Run — PASS** (behavior baseline for the extraction, both arms).
- [ ] **Step 3: Extract `export_irs_pdf_from_session`** (★ arch-C-1, task-sized surgery like Task 1's
  promote extraction) — the thin `export_irs_pdf(vault_path, pp, …)` KEEPS `Session::open` (`:358`) and calls
  the inner; ★ arch-n-new-1: everything AFTER the open — the full-vs-slice `return_inputs::exists` dispatch
  (`:373`), the `export_full_return` delegation, AND the crypto-slice body (`:385-599`) — moves INTO the
  `&Session` `_from_session` (its slice branch mirrors `export_full_return:642`). Re-run Step 1's
  characterization (both arms) → still PASS (the thin opener emits the identical packet — behavior-preserving).
- [ ] **Step 4: KAT — export set includes a removal-reordered prior year with NO promoted disposal leg.**
  Build: undisposed 2016-window promoted tranche + a 2025 donation whose lots the promote's HIFO reorder
  changes. Assert `flagged_years(...).contains(&2025)` AND `promoted_filing_years(state)` does NOT.
  (Mutation: define the export set as `promoted_filing_years` → 2025 dropped → reds.) ★ tax-M-1: ADD a
  two-live-promote POSITIVE coverage KAT — two promotes flagging DIFFERENT removal-reordered prior years
  Y1≠Y2; assert BOTH ∈ `plan_export.years` (realizable mutation: `flagged_years` iterates only ONE promote /
  last-promote-wins → the other year drops → reds — this pins per-promote ITERATION). NB: disjoint years
  can't cancel, so this does NOT falsify union-vs-whole-state (a whole-state diff also returns {Y1,Y2}); the
  union is taken as the DFW-D11-mandated provably-safe superset — a same-year cancellation fixture is
  unrealizable at leg-set-equality altitude, so it is deliberately NOT asserted.
- [ ] **Step 5: Run — FAIL** (functions not defined).
- [ ] **Step 6: Implement** `promoted_filing_years` (extract `admin.rs:84-98`), `flagged_years` (the UNION of
  per-promote fold-diffs via `promote_prior_year_advisory`'s per-`promote_id` iteration — ★ tax-M-1, NOT a
  single whole-state diff), the crate-root `IrsPdfReport` re-export (arch-n-1), and the export `plan/apply`
  trio — `apply_export` calls `export_irs_pdf_from_session(session, …, year, …)` per year (the
  `return_inputs::exists` slice-vs-full dispatch lives ONCE inside `_from_session`), NO re-`Session::open`;
  point `promote_export_gate(None)` at `promoted_filing_years` (single-source).
- [ ] **Step 7: Run — PASS** + full `promote_cli.rs`/census green.
- [ ] **Step 8: `make check` + fmt; Commit** `feat(chokepoint): export trio + &Session slice extraction + fold-diff year-set`.

### Task 4 — Consent-parity harness (the P-A gate)

**Files:** Test: `crates/btctax-cli/tests/chokepoint_parity.rs` (new). No production change (drives Tasks 1–3).

- [ ] **Step 1: Parity KAT at full-driver altitude.** For a fixture promoted tranche: (a) run the CLI verb
  `promote_tranche` capturing stdout + the recorded `Acknowledgment`; (b) drive the chokepoint the way the
  TUI will (`plan_promote` → `render_consent` → `apply_promote(Some(phrase))`) capturing the rendered
  string + recorded `Acknowledgment`. Assert: rendered consent copy + advisory lines **byte-identical**;
  `shown_terms` structurally `Eq`. Repeat on: happy path, refused-ack (consent still surfaced), and each
  refusal (BG-D5 bad provenance / BG-D3 partial coverage / BG-D7 empty Part II).
- [ ] **Step 2: Run — PASS.** Mutation: make the CLI driver post-process the consent string (e.g. trim) →
  the parity KAT reds (proves it drives real driver paths, not a renderer tautology).
- [ ] **Step 3: Commit** `test(chokepoint): full-driver consent-parity harness (P-A gate)`.

**★ P-A GATE:** Tasks 1–4 green + the parity harness passing + `make check` + CI-only jobs; whole-P-A
two-lens review (r1 = Fable arch / Opus tax; r2+ Opus) to 0C/0I before P-B.

---

## PHASE P-B — the derived `journey_view` (core) + the dashboard (tui-edit)

### Task 5 — Core: the `tranche_guard` predicate move (C-2) + the structured shortfall signal + DFW-D4 triage

**Files:** Create `crates/btctax-core/src/tranche_guard.rs`, `crates/btctax-core/src/defensive/mod.rs` (skeleton
`pub mod discovery;` — Task 6 fills in `journey_view`; ★ arch-m-new-2), `crates/btctax-core/src/defensive/discovery.rs`;
Modify `crates/btctax-core/src/lib.rs` (`pub mod tranche_guard; pub mod defensive;`), `crates/btctax-core/src/state.rs`,
`crates/btctax-core/src/project/fold.rs` (populate `state.shortfalls`; ★ arch-m-new-2), `crates/btctax-cli/src/cmd/tranche.rs`
(rewire guards over the moved predicates), `crates/btctax-cli/src/session.rs` (★ arch-I-new-2: rewire the FIFTH
consumer `:714` — a DIRECT `pre2025_tranche_exists` call, not via a guard); Test: `crates/btctax-core/tests/defensive_discovery.rs`,
`crates/btctax-cli/tests/declare_tranche_cli.rs` (shipped allocation-guard KATs — stay green; sweep its stale
`:320/:352/:358` "lives HERE (btctax-cli)"/`session.rs:692` locator comments).

**Interfaces — Produces:**
```rust
// ★ I-new-1 (finishing r1-C-2): the three pure event-scan predicates MOVED from cmd/tranche.rs to core
// (events-only, use btctax_core::conventions::TRANSITION_DATE). crates/btctax-core/src/tranche_guard.rs:
pub fn void_targets(events: &[LedgerEvent]) -> BTreeSet<EventId>;          // was tranche.rs:40 (private)
pub fn in_force_allocation_exists(events: &[LedgerEvent]) -> bool;         // was tranche.rs:54 (pub, cli-only)
pub fn pre2025_tranche_exists(events: &[LedgerEvent]) -> bool;             // was tranche.rs:71 (pub, cli-only)
pub struct Shortfall { pub event: EventId, pub wallet: Option<WalletId>, pub date: TaxDate,
    pub short_sat: i64, pub fee_sat: i64 }   // ★ arch-I-2/tax-M-1: short_sat = per-event principal+fee aggregate (DFW-D7/D8 clearance/prefill); fee_sat = the fee component (principal = short_sat - fee_sat)
pub enum Triage { DeclareCandidate(Shortfall), ResolveFirst { shortfall: Shortfall, blocker: BlockerKind },
                  DataFix(EventId) }   // without-wallet / degenerate
pub fn shortfalls(state: &LedgerState) -> Vec<Shortfall>;   // per-EVENT aggregate; NO Blocker.detail parse
pub fn triage(events: &[LedgerEvent], state: &LedgerState) -> Vec<Triage>;
```
`shortfalls` keys on the fold's sat-carrying `UncoveredDisposal` records (add a structured `state.shortfalls`
of raw `{event,wallet,date,principal_sat,fee_sat}` records populated in `fold.rs` at the six sat-carrying
`BlockerKind::UncoveredDisposal` sites — `:388`(fee)`,710,831,876,1196,1274` (re-grep per arch-m-5) —
aggregated per event into `Shortfall{short_sat = Σ(principal+fee), fee_sat = Σ fee}`). `triage`: a `Shortfall` on the same
`pool_key(date,wallet)` + `blocker_date <= short_date` as an open acquisition blocker
(`UnknownBasisInbound`/`Unclassified`/`ImportConflict`/`UnmatchedOutflows`) → `ResolveFirst`; a
`pending-out` short → `ResolveFirst` via its co-emitted `UnmatchedOutflows`; else `DeclareCandidate`.

**Steps 1–3 = the C-2 predicate move (do FIRST — a behavior-preserving refactor that Tasks 6/8 consume; ★ I-new-1):**
- [ ] **Step 1: Characterization — pin the shipped allocation-guard behavior BEFORE the move.** Confirm the
  shipped `declare_tranche_cli.rs` allocation-guard KATs are green (a `$0` declare succeeds; a pre-2025-
  tranche-vs-in-force-allocation conflict refuses at `guard_tranche_vs_allocation`/`guard_allocation_vs_tranche`).
  These are the behavior baseline for the cross-crate move.
- [ ] **Step 2: Move the three predicates to core + rewire ALL FIVE cli consumers.** Create
  `crates/btctax-core/src/tranche_guard.rs` with `in_force_allocation_exists`/`pre2025_tranche_exists`
  moved verbatim from `cmd/tranche.rs` (`:54,71`) and `void_targets` (`:40`) moved too (★ arch-n-new-3: it
  is a private `fn` today — keep it `pub(crate)`/module-private in core, a deliberate visibility choice, not
  a verbatim signature); all events-only, using `btctax_core::conventions::TRANSITION_DATE`. Add
  `pub mod tranche_guard;` + `pub mod defensive;` (with a `defensive/mod.rs` skeleton `pub mod discovery;`)
  to core `lib.rs`. Rewire all FIVE predicate CALL SITES to `btctax_core::tranche_guard::*` and DELETE the
  cli copies: 4 sites inside the two guards `guard_tranche_vs_allocation`/`guard_allocation_vs_tranche`
  (`:107,93` — the GUARDS themselves STAY in cli; only their internal predicate calls at
  `tranche.rs:61,72,94,111` rewire), AND ★ **arch-I-new-2/tax-M-2** the FIFTH, DIRECT consumer
  `session.rs:714` (`crate::cmd::tranche::pre2025_tranche_exists(&all)` → `btctax_core::tranche_guard::
  pre2025_tranche_exists(&all)` — rewire, do NOT leave a duplicate). The four allocation APPEND sites
  (`reconcile.rs:1015,1258`, `edit/persist.rs:1032,1105`) call the STAYING `guard_allocation_vs_tranche` and
  are preserved automatically (no rewire).
- [ ] **Step 3: Sweep stale locator comments + run.** Update `declare_tranche_cli.rs`'s `:320/:352/:358`
  comments that say the predicate "lives HERE (btctax-cli)" / cite `session.rs:692` (now → the core
  `tranche_guard` module); run the shipped allocation-guard KATs still green + `make check`; Commit
  `refactor(core): move tranche_guard predicates to btctax-core (C-2)` (behavior-preserving).
**Steps 4–8 = the structured shortfall signal:**
- [ ] **Step 4: KATs (DFW-D4, §5).**
```rust
#[test] fn self_transfer_short_is_one_declare_candidate_of_short_sat() { /* a self-transfer-short vault → exactly one DeclareCandidate, short_sat = the shortfall */ }
#[test] fn gift_out_without_wallet_yields_zero_declare_candidates() { /* → DataFix, no candidate */ }
#[test] fn donate_without_wallet_yields_zero_declare_candidates() { /* → DataFix */ }
#[test] fn shortfall_behind_open_unclassified_is_resolve_first() { /* same pool+timeframe → ResolveFirst, no candidate */ }
#[test] fn pending_out_short_routes_through_unmatched_outflows_first() { /* ★ tax-I-1/arch-I-5 the C-1 double-count guard: a pending-out short with a co-emitted UnmatchedOutflows → ResolveFirst, NOT a DeclareCandidate (a later TransferLink may reshape it) */ }
#[test] fn principal_plus_fee_short_on_one_event_aggregate_to_one_shortfall() { /* ★ arch-I-2: per-event sum → short_sat == principal+fee AND fee_sat == the fee component */ }
#[test] fn fee_only_short_has_fee_sat_equal_short_sat() { /* ★ arch-I-2/tax-M-1: a pure-fee short (fold.rs:388) → fee_sat == short_sat; a pure-principal short → fee_sat == 0 */ }
#[test] fn shortfalls_never_parses_blocker_detail() { /* grep-guard: discovery.rs contains no ".detail" */ }
```
- [ ] **Step 5: Run — FAIL.**
- [ ] **Step 6: Implement** `state.shortfalls` (populate the raw `{…,principal_sat,fee_sat}` records in
  `fold.rs` at the six sat sites `:388,710,831,876,1196,1274`), `shortfalls()` (aggregate per event, summing
  `fee_sat` separately), `triage()`, `pool_key(date,wallet)` reuse (`pools.rs:15`).
- [ ] **Step 7: Run — PASS** + `make check`.
- [ ] **Step 8: Commit** `feat(defensive): structured shortfall signal + total-by-short_sat triage`.

### Task 6 — Core: `journey_view` + the derived advisories (didn't-cover, over-covered, drift)

**Files:** Create `crates/btctax-core/src/defensive/mod.rs`; Test: `crates/btctax-core/tests/defensive_journey.rs`

**Interfaces — Produces:**
```rust
pub struct TrancheRow { pub target: EventId, pub sat: i64, pub status: TrancheStatus,
    pub clamped_saving: Vec<SavingFlavor>, pub advisories: Vec<Advisory> }
pub enum TrancheStatus { DeclaredZero, Promoted }   // ★ I-3: NO per-tranche DidNotCover (DFW-D3/D5.3 forbid attribution)
pub enum Advisory { OverCovered { by_sat: i64 }, NowDisplacing, MethodInversion(String), TrancheDip(String), FeeOnlyPromoteNoop }
pub enum SavingFlavor { ComputedTax { year: i32, delta: Usd }, Uncomputable { year: i32, gain_delta: Usd },
    Named(String) }
pub struct PoolShort { pub pool: PoolKey, pub short_sat: i64, pub live_tranche_sat: i64 }  // ★ I-3 pool-level
pub struct DefensiveFilingView { pub candidates: Vec<Shortfall>, pub resolve_first: Vec<Triage>,
    pub tranches: Vec<TrancheRow>, pub still_short: Vec<PoolShort>, pub flagged_years: BTreeSet<i32>,
    pub safe_harbor_blocked: bool }
pub fn journey_view(events: &[LedgerEvent], state: &LedgerState, prices: &dyn PriceProvider,
    tables: &dyn TaxTables, cfg: &ProjectionConfig) -> DefensiveFilingView;
```
All shadow projections force `pseudo_reconcile=false` (DFW-D6); AND `journey_view` opens with
`debug_assert!(!state.pseudo_active())` (★ arch-m-new-4 — the DFW-D6 precondition the Task 7 entry gate
enforces) so the DISCOVERY read `shortfalls(state)` is never taken off a pseudo-active state whose
`state.shortfalls` are synthetic-cleared (a Phase-B `SelfTransferMine{$0}` masking a real short). `clamped_saving` = clamped only
(`clamped_promote_year_saving`), three-flavor: `ComputedTax` only when both folds price the year (table ∈
{2017,2024,2025,2026} ∧ stored `TaxProfile` ∧ no Hard blocker), else `Uncomputable`, else `Named`. The
advisories are **derived** (no gate): `OverCovered{by_sat}` iff (`covered_sat>0` ∧ `live_sat>covered_sat`)
via a without-promote sat-count shadow (DFW-D5.3, M-1 scope — NOT for a fully-undisposed tranche);
`NowDisplacing` iff a `basis_source`-composition with/without-promote fold-diff shows a documented leg
replaced by an `EstimatedConservative` floor leg (mirrors `promote_drift_advisory`); `FeeOnlyPromoteNoop`
iff the shortfall(s) the tranche covers are all fee-component (`Shortfall.short_sat == fee_sat`, ★ arch-I-2/
tax-M-1); `MethodInversion(msg)`/`TrancheDip(msg)` = the shipped `conservative::method_inversion_advisory`/
`tranche_dip_advisory` (`conservative.rs:61,27`) surfaced VERBATIM on the tranche's disposal row (DFW
tax-N-2). The pool-level `still_short` (★ I-3 — one combined `PoolShort` per pool, NOT a per-tranche status):
a `PoolShort{pool, short_sat, live_tranche_sat}` iff a live `DeclareTranche` has `pool_key(we,wallet)` =
the shortfall's pool ∧ `we <= short date` while the pool is still short — no per-tranche attribution.

- [ ] **Step 1: KATs.**
```rust
#[test] fn fully_undisposed_tranche_shows_no_over_covered_advisory() { /* covered_sat==0 → no OverCovered */ }
#[test] fn over_sized_tranche_shows_over_covered_by_excess() { /* declare 100M, 60M in-pool import → OverCovered{by_sat:60_000_000} */ }
#[test] fn a_correctly_sized_cover_and_mixed_vintage_show_no_over_covered() { /* neither over-covered */ }
#[test] fn promoted_tranche_now_displacing_shows_now_displacing_advisory() { /* basis_source composition diff */ }
#[test] fn now_displacing_uses_basis_source_composition_not_leg_set_inequality() { /* ★ tax-M negative: a correctly-sized cover ALSO changes legs ($0→floor same lot) → must NOT show NowDisplacing */ }
#[test] fn uncomputable_audience_year_2020_shows_gain_delta_not_a_dollar_tax() { /* SavingFlavor::Uncomputable */ }
#[test] fn table_year_with_no_TaxProfile_shows_uncomputable_not_a_bare_dollar() { /* ★ tax-I-2: 2024 table exists but no stored profile → Uncomputable, never ComputedTax */ }
#[test] fn a_live_tranche_not_clearing_its_pool_shows_pool_still_short() { /* ★ tax-I-3/arch-I-5: DefensiveFilingView.still_short has ONE PoolShort; assert_eq! its short_sat AND live_tranche_sat (★ tax-M-2 — the residual value, not just the count); no per-tranche DidNotCover */ }
#[test] fn fee_only_coverage_tranche_shows_fee_only_promote_noop() { /* ★ arch-I-2/tax-M-1: covered shortfall short_sat==fee_sat → FeeOnlyPromoteNoop; a principal-coverage tranche → none */ }
#[test] fn hifo_steered_promote_surfaces_method_inversion_advisory() { /* ★ arch-I-2/tax-N-2: method_inversion_advisory (conservative.rs:61) present VERBATIM on the tranche row; absent when the method doesn't invert */ }
#[test] fn tranche_dip_surfaces_on_tranche_row() { /* ★ arch-I-2: tranche_dip_advisory (conservative.rs:27) present verbatim on the row; absent when no dip */ }
#[test] fn journey_view_forces_pseudo_off() { /* pseudo-active vault → candidates/savings unchanged by pseudo */ }
#[test] fn zero_declared_tranche_status_is_DeclaredZero_never_incomplete() { /* DFW-D3 */ }
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement `journey_view`** composing `shortfalls`/`triage` (Task 5), `clamped_promote_year_saving`,
  the with/without-promote shadow fold-pairs, `flagged_years()` (★ arch-m-4: the STRUCTURED `BTreeSet` fn from
  Task 3 for the `flagged_years` field — NOT the `Vec<String>` `promote_prior_year_advisory`, else the banned
  string-parse re-enters), the derived advisories — `OverCovered`/`NowDisplacing` (shadows), `FeeOnlyPromoteNoop`
  (covered `Shortfall.short_sat == fee_sat`), and `MethodInversion`/`TrancheDip` surfaced verbatim from
  `conservative::method_inversion_advisory`/`tranche_dip_advisory` (`:61,27`) — and
  `tranche_guard::in_force_allocation_exists`+`pre2025_tranche_exists` (safe_harbor_blocked — the CORE
  predicate, C-2, never the cli-private guard). Pure; mutation-proven per advisory.
- [ ] **Step 4: Run — PASS** + `make check`.
- [ ] **Step 5: Commit** `feat(defensive): journey_view + derived over-covered/drift/saving advisories`.

### Task 7 — tui-edit: the dashboard screen (derived rows, fork, launch)

**Files:** Create `crates/btctax-tui-edit/src/defensive_dashboard.rs`; Modify `editor.rs`, `draw_edit.rs`;
Test: `crates/btctax-tui-edit/src/defensive_dashboard.rs` `#[cfg(test)]`

**Interfaces — Consumes:** `btctax_core::defensive::{journey_view, DefensiveFilingView, ...}`;
launches the flows (Tasks 8–9). **Produces:** `EditorScreen::DefensiveFiling`; the dashboard render +
key-dispatch (`d`=declare on a candidate row, `p`=promote on a tranche row, `x`=export, `enter`=route a
ResolveFirst to its shipped flow).

- [ ] **Step 1: KATs.** (a) entry refuses when `state.pseudo_active()` with routing guidance (DFW-D6);
  (b) a `$0`-declared tranche row renders "filed \$0 — complete", NEVER "incomplete/step N" (DFW-D3);
  (c) the fork renders promote as an explicit optional branch; (d) `x`/export is always-available (never
  a "done" checkbox — M-5); (e) an `OverCovered` advisory row renders the void+re-declare copy; a
  fee-only-coverage tranche suppresses/annotates its promote branch (N-1); (f) ★ arch-m-3: a `PoolShort`
  row renders "still short by S — don't declare again" (the dashboard-render of I-5(b); the view-level
  KAT is Task 6, this pins the render).
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** the dashboard over `journey_view` (read-only derived render + dispatch to the
  flows/shipped remedial flows). Add `EditorScreen::DefensiveFiling` + the `!pseudo_active()` gate + the
  one-flow debug assertion (M-4). Respect KAT-G1's `kat_g1_mechanized_source_gate` `everywhere_tokens`
  (no `cmd::`; write-class tokens only in permitted modules) — ★ arch-m-2: the tui-edit gate is KAT-G1
  (`persist.rs:1897`), NOT `e10` (that gate is `btctax-tui`'s).
- [ ] **Step 4: Run — PASS** + `make check` (incl. KAT-G1's `kat_g1_mechanized_source_gate`).
- [ ] **Step 5: Commit** `feat(tui-edit): Defensive Filing dashboard (derived rows + fork + launch)`.

**★ P-B GATE:** Tasks 5–7 green + `make check` + CI-only; whole-P-B two-lens review to 0C/0I before P-C.

---

## PHASE P-C — era presets + the declare/promote flows (live readout, prefill, clearance, consent)

### Task 8 — Core era table + the declare flow (prefill, live floor/coverage/saving, safe-harbor precheck)

**Files:** Create `crates/btctax-core/src/defensive/era.rs`, `crates/btctax-tui-edit/src/edit/declare_flow.rs`;
Modify `draw_edit.rs`, `crates/btctax-tui-edit/src/edit/persist.rs` (★ arch-m-1: `persist_declare_tranche` + KAT-G1 allowlist);
Test: `crates/btctax-core/tests/defensive_era.rs`, declare_flow `#[cfg(test)]`

**Interfaces — Produces:** `era::era_window(preset: EraPreset) -> (Date, Date)`; the `DeclareFlow{step,
sat, window_start, window_end, ...}` driving `plan_declare(target_shortfall = Some(shortfall.event))`.
Prefill (DFW-D5): `window_end` strictly before the short op's date; `wallet` = the short op's source-pool
wallet. Live readout (cheap trio only — floor via `window_reference`, `Coverage`, holding date =
`window_end`); tax-Δ on demand, invalidated on any window edit (DFW-D10 M-1). Preset governs a starting
window; the before-op prefill governs on conflict (DFW-D9).

- [ ] **Step 1: KATs.** (a) `era_window` for each preset maps to a concrete window; (b) declare-flow
  prefill puts `window_end` before the disposal + the source wallet; (c) `Coverage::Partial`/`NoCoverage`
  refusal surfaces live in the readout; (d) the safe-harbor exclusion is a first-class entry state
  (the CORE `tranche_guard::{pre2025_tranche_exists, in_force_allocation_exists}`, C-2 — never the cli guard); (e) editing the window blanks the
  on-demand saving ("stale — recompute").
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** `era.rs` (the reviewed preset table), the declare flow (COLLECTS window/sat + reads `plan_declare(Some(shortfall.event))`; the WRITE goes through `persist_declare_tranche` in `edit/persist.rs` — C-3/KAT-G1; a thin driver over
  `plan_declare(Some)`), the live-readout, the safe-harbor precheck. Attestation-substance copy: the
  window is the filer's OWN knowledge (DFW-D9).
- [ ] **Step 4: Run — PASS** + `make check`.
- [ ] **Step 5: Commit** `feat(defensive): era presets + declare flow (prefill/live-readout/safe-harbor)`.

### Task 9 — The promote flow (consent TypedWord gate + Part II authoring, one-at-a-time)

**Files:** Create `crates/btctax-tui-edit/src/edit/promote_flow.rs`; Modify `draw_edit.rs`, `editor.rs`,
`crates/btctax-tui-edit/src/edit/persist.rs` (★ arch-m-1: `persist_promote_tranche` + KAT-G1 allowlist);
Test: promote_flow `#[cfg(test)]` + a TUI parity KAT tie-in to Task 4

**Interfaces — Consumes:** `chokepoint::{plan_promote, render_consent, apply_promote}`. The flow: select a
tranche row → author Part II (multiline; BG-D7 non-empty/non-scaffold refusal enforced at
`plan_promote`) → `render_consent` shown → TypedWord ack (mirrors `PROMOTE_ACK_PHRASE`) → `apply_promote`.
One tranche at a time (DFW-D12).

- [ ] **Step 1: KATs.** (a) the TUI promote records an `Acknowledgment` byte-identical to the CLI (tie to
  the Task-4 parity harness driving the TUI path); (b) an empty/whitespace Part II is refused (BG-D7);
  (c) a wrong ack phrase refuses (fail-closed); (d) an undisposed tranche promotes and records the
  `Unrealized` term (behavior-preserving — DFW-D5.3).
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** the promote flow (thin driver; Part II authoring; TypedWord gate). No `cmd::`
  token. ★ C-3: the WRITE goes through `persist_promote_tranche` in `edit/persist.rs` (KAT-G1) — the flow only COLLECTS Part II + ack and reads `plan_promote`/`render_consent`; reach the chokepoint via the `btctax_cli` crate-root re-export (no `cmd::`).
- [ ] **Step 4: Run — PASS** + `make check` (incl. KAT-G1's `kat_g1_mechanized_source_gate`).
- [ ] **Step 5: Commit** `feat(tui-edit): promote flow (Part II authoring + consent TypedWord gate)`.

**★ P-C GATE:** Tasks 8–9 green + `make check` + CI-only; whole-P-C two-lens review to 0C/0I before P-D.

---

## PHASE P-D — Forms / export step

### Task 10 — The export step (chokepoint-driven, year-set, no pseudo-attest)

**Files:** Modify `defensive_dashboard.rs` (the `x` action), `crates/btctax-tui-edit/src/edit/persist.rs`
(★ arch-m-1: `persist_defensive_export` + KAT-G1 allowlist/guarantee text); Test: dashboard `#[cfg(test)]`

**Interfaces — Consumes:** `btctax_core::conservative::flagged_years` + `btctax_cli::chokepoint::{plan_export, apply_export}`.

- [ ] **Step 1: KATs.** (a) `x` on a vault with a promoted 2025 leg + a 2024 removal-reordered year exports
  BOTH years' packets (`plan_export.years`); (b) a pseudo-active vault refuses+routes (never prompts the
  attest phrase — DFW-D11); (c) the packet includes `form_8275.pdf` for the promoted leg (reuses the
  shipped gated export via the chokepoint).
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** the `x` action: the dashboard reads `plan_export`, the WRITE goes through `persist_defensive_export` in `edit/persist.rs` (★ C-3/KAT-G1 — the editor's FIRST export surface; extend the G1 allowlist + guarantee text this task).
- [ ] **Step 4: Run — PASS** + `make check`.
- [ ] **Step 5: Commit** `feat(tui-edit): defensive-filing export step (year-set, no pseudo-attest)`.

**★ P-D GATE + SHIP:** all tasks green; `make check` + CI-only jobs + `make docs`; the whole-branch
two-lens (tax + arch, Opus) review to 0C/0I; per-phase-authorized merge to `main`. RELEASE (sub-2 bump +
publish) is a SEPARATE user call after the whole feature is green + merged.

---

## Self-Review (author checklist, run against SPEC)

- **Coverage:** DFW-D1→Tasks 5/6/7 seams; DFW-D2→Task 1/2/3/4 chokepoints + parity; DFW-D3→Task 7 fork;
  DFW-D4→Task 5 triage; DFW-D5→Task 2 clearance + Task 6 didn't-cover + Task 8 prefill; DFW-D5.3→Task 6
  advisories; DFW-D6→Task 1 pseudo-off + Task 6 shadows; DFW-D7→Task 5 signal; DFW-D8→Task 2/8; DFW-D9→Task 8
  era/safe-harbor; DFW-D10→Task 6 flavors + Task 8 readout; DFW-D11→Task 3 year-set + Task 10 export;
  DFW-D12→Task 9 one-at-a-time. §8 sub-1 pseudo fix → Task 1. All 12 decisions have a task.
- **Placeholders:** none (interfaces + KATs + code sketches concrete). **Type consistency:** `Shortfall`
  (`short_sat`+`fee_sat`) / `TrancheRow` / `Advisory` (incl. `TrancheDip`) / `SavingFlavor` / `PromotePlan`
  (`advisory_lines`+`gift_only_years`+`post_consent_note`) / `Refusal` (`Target`; no `Conflict`) /
  `plan_promote` / `plan_declare` / `flagged_years` / `export_irs_pdf_from_session` / `plan_export`
  consistent across tasks.
- **arch-r2 fold (this pass):** C-1 export `&Session` extraction (Task 3); I-1 `PromotePlan` ordered
  fields (Task 1); I-2 `Shortfall` fee/principal split + `FeeOnlyPromoteNoop`/`MethodInversion`/`TrancheDip`
  derivation (Tasks 5/6); m-1 `persist.rs` manifests (Tasks 8/9/10); m-2 e10→KAT-G1; m-3 `PoolShort` render
  KAT; m-4 `flagged_years()` not the `Vec<String>`; m-5 emit-site lines `:388,710,831,876,1196,1274`; m-6/
  tax-N-1 `Refusal::Target` + `pre2025_tranche_exists(events)`; n-1 crate-root `IrsPdfReport` re-export;
  tax-M-2 residual assert; tax-M-3 phantom-wallet stderr.
- **r3 fold (this pass):** arch **I-new-1** — the C-2 `tranche_guard` predicate move is now SCHEDULED (Task 5
  Steps 1–3, characterization + rewire + refactor commit), no longer File-Map-only; arch m-new-1/n-new-1 —
  the export dispatch lives ONCE inside `_from_session` (both `export_irs_pdf` and `apply_export` call it;
  full-return-arm characterization added); m-new-2 — Task 3/5 Files manifests; m-new-3 — dropped dead `state`
  from `plan_promote`/`plan_declare`; m-new-4 — `journey_view` `debug_assert!(!pseudo_active)` discovery
  guard; tax-M-1 — `flagged_years` per-promote union + two-promote KAT; tax-N-1 — `Refusal::Target` doc
  (already-promoted is apply-time `would_conflict`). Tax r3 was GREEN; arch r3 was 0C/1I.
- **r4 fold (this pass):** arch **I-new-2** = tax **M-2** (both lenses independently) — the C-2 move missed a
  FIFTH predicate consumer, `session.rs:714`'s direct `pre2025_tranche_exists`; Task 5 now rewires it (the 4
  allocation append sites call the STAYING guard, preserved automatically — grep-verified there is no sixth).
  arch n-new-2 slice body `:385-599`; arch n-new-3 `void_targets` visibility (`pub(crate)`, not verbatim);
  tax-M-1 the two-promote KAT reframed to a realizable per-promote-iteration mutation (union-vs-whole-state
  is unfalsifiable at leg-set altitude → union is the DFW-D11 safe superset, documented not asserted); tax-N-1
  `flagged_years` filtered `< current` (mirrors `conservative.rs:729`) so an in-authoring year gets no premature
  1040-X. Tax r4 GREEN 0C/0I; arch r4 0C/1I.
