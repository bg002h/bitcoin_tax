# Conservative / Defensive Filing — Implementation Plan

**Status:** ★ GREEN — both lenses 0 Critical / 0 Important. Tax r1 (0C/2I) → r2 (0C/1I) → r3 GREEN → r4
GREEN; architecture r1 (0C/3I) → r2 (0C/1I) → r3 (0C/1I) → r4 GREEN. Reviews in `./reviews/`
(`plan-tax-*`, `plan-architecture-*`). Ready to implement (execution mode is the owner's call).

> **For agentic workers:** implement task-by-task in phase order. Each task is TDD (write the failing
> test, watch it fail, minimal implementation, watch it pass, commit) and every primitive is
> **mutation-proven** (see §"Mutation discipline"). Steps use `- [ ]` checkboxes for tracking.

**Goal:** Ship the Approach-C *primitives* that let a poor-records BTC holder file maximally
defensively with least effort — declare undocumented coins as `$0`-basis "tranches", report every
disposal correctly (term- and year-aware), and surface informational nudges quantifying what
reconstructing records would save — without ever understating tax or filing a `>$0` estimate.

**Architecture:** One new first-class decision event `EventPayload::DeclareTranche` folds (via the
existing `Op::Acquire` path) into a `$0`-basis lot tagged `BasisSource::EstimatedConservative` and
homed at `acquired_at = window_end`. Everything else is layered on top: the tag must survive the two
sites that overwrite `basis_source` (2025 Path-A seed + self-transfer relocation), a tranche and an
*effective* Rev-Proc-2024-28 safe-harbor allocation are made structurally mutually exclusive, and a
set of advisories/engines (dip, method-inversion, custody, window-reference price, overpayment-delta
nudge, mandatory methodology disclosure, self-custody nudge) read off the tag. No new tax math: term,
box, and gain are all *derived* from the existing engine.

**Tech Stack:** Rust workspace (`btctax-core` engine under `src/project/{resolve,fold,transition,pools}.rs`;
`btctax-cli`; `btctax-tui`; `btctax-tui-edit`; `btctax-forms`). `rust_decimal` `Usd`, `time` dates.
Tests: `cargo nextest` + KAT files under `crates/*/tests/`. Validation: `make check` (nextest + clippy)
**plus** the CI-only jobs (`cargo fmt --check`, `cargo run -p xtask -- check-isolation`,
`bash scripts/pii-scan-generic.sh`, `cargo +1.88 build`).

## Global Constraints

Copied verbatim from `SPEC.md` §1 / §6 — every task's requirements implicitly include these:

- **G-1 Never omit a taxable event.** Every disposal (incl. private/P2P) is reported.
- **G-2 `$0` is the only unassailable basis.** `$0` is the v1 filed basis for unprovable coins.
- **G-3 The fairness ↔ attack-surface curve is the filer's to walk.** v1 files `$0`; nudges quantify
  the other end; the choice is informed and theirs, never made silently.
- **G-4 Never UNDERSTATE tax.** Character (ST vs LT) and Part/box are **derived** from the computed
  holding period, NEVER assumed long-term. Use the engine's existing `term_for` / `is_long_term`.
- **D-7 v1 declares & files `$0` ONLY.** `DeclareTranche` carries no floor; nothing `>$0` is ever
  written to a filed 8949 by this flow. The window-low reference (P5) feeds ONLY P6's informational delta.
- **D-5 A tranche is FILING-READY, NOT pseudo.** It must export clean — no `[PSEUDO]` banner / attestation.
- **Every primitive TDD + mutation-proven; full suite + all CI-only jobs green; reviewed to 0C/0I under
  BOTH the tax and architecture lenses before merge.**
- Fold machinery is under `crates/btctax-core/src/project/` (NOT `src/` directly — the SPEC's older
  paths drifted). `Usd::ZERO` is the zero constant; `BasisSource` is `#[derive(Copy)]`.
- Forward-only vault compat: a new `EventPayload`/`BasisSource` variant means older binaries can't read
  new vaults. There is **no installed base** (memory: `no-users-yet`) — harmless; no migration needed.

## Mutation discipline (applies to every task)

A fix/feature isn't done until the mutation dies (memory: `untested-guard-pattern`). For each task,
after GREEN, perform the named mutation (flip the new predicate / revert the new arm), run the task's
tests, confirm **RED**, then restore. Record it in the commit body. Cheap way: `cp` the file to the
scratchpad, `sed` the mutation, run the test, `cp` back (never `git checkout --` a tracked file mid-work —
memory: `git-checkout-eats-uncommitted-mutation-reverts`).

## File Structure

**New files:**
- `crates/btctax-core/src/conservative.rs` — the feature's owned surface: `window_reference` (P5),
  `overpayment_delta` (P6), the advisory builders (P3/P8), and the methodology-disclosure text builder
  (P7). One module, feature-cohesive; `pub use` the public fns from `lib.rs`.
- `crates/btctax-core/tests/kat_tranche.rs` — P1 core KATs (fold, tag survival, mutual exclusion).
- `crates/btctax-core/tests/kat_conservative.rs` — P2–P8 + invariant KATs.
- `crates/btctax-cli/tests/declare_tranche_cli.rs` — the CLI verb + record-time refusal KATs.

**Modified (exact sites, verified on `feat/conservative-filing`):**
- `crates/btctax-core/src/event.rs` — `BasisSource` (`:16-30`), `EventPayload` (`:298-331`),
  `is_imported` (`:335-345`); a `DeclareTranche` payload struct.
- `crates/btctax-core/src/project/resolve.rs` — timeline builder (`:1055-1083`), `build_op` (`:258-395`),
  `sort_canonical` (`:1376-1382`), safe-harbor effective/inert decision (`:1210-1296`, unconservable at
  `:1237`).
- `crates/btctax-core/src/project/fold.rs` — relocation arm (`:812`).
- `crates/btctax-core/src/project/transition.rs` — `UniversalSnapshot` (`:19-22`), `universal_snapshot`
  (`:32-72`), Path-A seed overwrite (`:83`).
- `crates/btctax-core/src/forms.rs` — `how_acquired_from` (`:265-275`).
- `crates/btctax-core/src/void.rs` — `is_revocable_payload` (`:20-34`, arch I-2; non-`matches!`, NOT
  compile-forced). **Do not touch `effective_alloc`/`voidable_decisions` semantics** (arch M-5).
- `crates/btctax-cli/src/render.rs` — `basis_source_tag` (`:43-57`), `render_tax_outcome` (`:1204-1209`).
- `crates/btctax-cli/src/cmd/tax.rs` — `TaxYearReport` advisory field(s) (`:238-247`, arch M-3, P3/P6 surface).
- `crates/btctax-cli/src/main.rs` — `bulk_void_payload_summary` readable arm (`:2085-2133`, arch I-2).
- `crates/btctax-cli/src/cmd/reconcile.rs` — allocation append sites (`:984`, `:1273`, Task 6).
- `crates/btctax-cli/src/session.rs` — `safe_harbor_residue` (`:681-700`, arch I-3, exclude tranches).
- `crates/btctax-tui-edit/src/edit/form.rs` — edit-ring (`:1756-1766`), `basis_source_display` (`:1770-1781`).
- `crates/btctax-tui-edit/src/edit/persist.rs` — TUI allocation append sites (`:1031`, `:1114`, Task 6).
- `crates/btctax-tui/src/tabs/tags.rs` — `basis_source_rank` (`:29-40`) + `basis_source_tag` (`:54-65`, arch M-1).
- `crates/btctax-cli/src/cli.rs` + `src/main.rs` + a new `src/cmd/tranche.rs` (registered in `src/cmd/mod.rs`) — the `declare-tranche` verb.
- `crates/btctax-tui/src/tabs/tax.rs` (`:40`) — surface P6 nudge + P7 disclosure marker.

---

## Phase 1 — P1: `DeclareTranche` core (the tranche exists, folds, its tag survives, it is mutually exclusive with an effective Path-B allocation)

Everything else depends on Phase 1. Land it fully (Tasks 1–7) before Phase 2.

### Task 1: Schema + exhaustive-`match` sweep (compile-forced)

**Files:**
- Modify: `crates/btctax-core/src/event.rs` — add the `BasisSource` variant + the `DeclareTranche` payload;
  update the `is_imported` doc (N-2: `DeclareTranche` folds as a primary movement, so "imported are the only
  ones folded as primary movements" is now false).
- Modify (compile-forced — **6** exhaustive `BasisSource` sites, not 4): `crates/btctax-core/src/forms.rs:265-275`
  (`how_acquired_from`), `crates/btctax-cli/src/render.rs:43-57` (`basis_source_tag`),
  `crates/btctax-tui-edit/src/edit/form.rs:1756-1766` (edit-ring) and `:1770-1781` (`basis_source_display`),
  `crates/btctax-tui/src/tabs/tags.rs:29-40` (`basis_source_rank`) and `:54-65` (`basis_source_tag`).
- Modify (NOT compile-forced — a non-exhaustive `matches!`, so the compiler will NOT flag it — arch I-2):
  `crates/btctax-core/src/void.rs:20-34` (`is_revocable_payload`) — add `EventPayload::DeclareTranche(_)`.
  Cosmetic sibling: `crates/btctax-cli/src/main.rs:2085-2133` (`bulk_void_payload_summary`, a `Debug` wildcard
  today) — add a readable arm.
- Test: `crates/btctax-core/tests/kat_tranche.rs` (new).

**Interfaces:**
- Produces: `BasisSource::EstimatedConservative`; `EventPayload::DeclareTranche(DeclareTranche)` where
  `pub struct DeclareTranche { pub sat: Sat, pub wallet: WalletId, pub window_start: TaxDate, pub window_end: TaxDate }`.

- [ ] **Step 1: Failing test** — the new `BasisSource` maps to the conservative labels/donor field.
```rust
// crates/btctax-core/tests/kat_tranche.rs
use btctax_core::event::BasisSource;
use btctax_core::forms::how_acquired_from; // pub-export if not already
use btctax_core::Form8283HowAcquired;

#[test]
fn estimated_conservative_donor_field_is_review() {
    // tax min-6: EstimatedConservative is NOT an 8949 column; on Form 8283 (donation) it needs manual
    // review (an LT tranche donation → FMV; an ST-held tranche donation → deduction limited to basis = $0).
    assert_eq!(how_acquired_from(BasisSource::EstimatedConservative), Form8283HowAcquired::Review);
}
```
- [ ] **Step 2: Run — expect FAIL** (`error[E0599]`/`E0004`: variant missing). `cargo test -p btctax-core --test kat_tranche`.
- [ ] **Step 3: Implement** — add the variant + payload, then satisfy every exhaustive match the compiler flags:
```rust
// event.rs BasisSource (:16-30), append:
    /// A conservative-filing tranche: undocumented coins declared at $0 basis (the IRS fallback),
    /// homed at window_end. Filing-ready (NOT pseudo). See conservative-filing SPEC D-1.
    EstimatedConservative,

// event.rs, near the other decision payload structs:
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclareTranche {
    pub sat: Sat,
    pub wallet: WalletId,
    pub window_start: TaxDate,
    pub window_end: TaxDate,
}
// event.rs EventPayload (:298-331), add a variant:
    DeclareTranche(DeclareTranche),
```
Then the SIX exhaustive `BasisSource` sites (the compiler flags each; a 7th, `is_revocable_payload`, is
`matches!` and will NOT be flagged — add it too):
```rust
// forms.rs how_acquired_from (:265-275): add
    BasisSource::EstimatedConservative => Form8283HowAcquired::Review,
// render.rs basis_source_tag (:43-57): add
    BasisSource::EstimatedConservative => "estimated_conservative",
// tui-edit/edit/form.rs edit-ring next() (:1756-1766): keep EstimatedConservative OFF the ring
//   (precedent: SelfTransferInbound is off-ring). Add the defensive exit arm:
    BasisSource::EstimatedConservative => BasisSource::ExchangeProvided,
// tui-edit/edit/form.rs basis_source_display (:1770-1781): add
    BasisSource::EstimatedConservative => "estimated-conservative",
// btctax-tui/src/tabs/tags.rs basis_source_rank (:29-40): rank it with the other origin-lost sources
    BasisSource::EstimatedConservative => <rank alongside ReconstructedPerWallet>,
// btctax-tui/src/tabs/tags.rs basis_source_tag (:54-65): add
    BasisSource::EstimatedConservative => "estimated-conservative",
// btctax-core/src/void.rs is_revocable_payload (:20-34) — NON-exhaustive matches!, NOT compiler-flagged.
//   Without this, voidable_decisions excludes tranches → bulk-void + both TUI void flows treat a tranche
//   as permanent, contradicting D-1a-d while the engine-side void still works (Task 3's KAT would pass
//   over the gap). Add DeclareTranche to the revocable list:
    | EventPayload::DeclareTranche(_)
// btctax-cli/src/main.rs bulk_void_payload_summary (:2085-2133): add a readable arm (was a Debug wildcard).
```
- [ ] **Step 4: Run — expect PASS.** Build ALL affected crates (M-1: the earlier line omitted `btctax-tui`,
  giving a false local green): `cargo build -q -p btctax-core -p btctax-cli -p btctax-tui -p btctax-tui-edit`.
- [ ] **Step 5: Commit** `feat(tranche): BasisSource::EstimatedConservative + EventPayload::DeclareTranche schema + full sweep (6 BasisSource sites + is_revocable_payload)`.

### Task 2: Fold a `DeclareTranche` into a `$0` `EstimatedConservative` lot (D-1, D-1a, D-2)

This is the core. Reuse the `Op::Acquire` fold arm: it already builds a lot with `acquired_at = eff.date()`,
`usd_basis = usd_cost + fee_usd`, `basis_source = a.basis_source`, `pool_key(eff.date())`, and bumps
`stats.sigma_in` (`fold.rs:566-596`). We only need (a) the timeline builder to admit the decision with
`Eff.utc` set so `eff.date() == window_end`, (b) `build_op` to return `Op::Acquire` with `$0` cost and the
tag, (c) `sort_canonical` to break same-window ties on `decision_seq` numerically.

**Files:**
- Modify: `crates/btctax-core/src/project/resolve.rs` — timeline builder (`:1055-1083`), `build_op` (`:258-395`
  + catch-all `:393`), `sort_canonical` (`:1376-1382`).
- Test: `crates/btctax-core/tests/kat_tranche.rs`.

**Interfaces:**
- Consumes: `Op::Acquire(Acquire { sat, usd_cost, fee_usd, basis_source })` (`resolve.rs:24`; fold arm
  `fold.rs:566-596`), `Eff { id, utc, tz, src_priority, src_ref, wallet, op, pseudo }` (`resolve.rs:102-116`),
  `pool_key(date, wallet)` (`pools.rs:15-21`), `EventId::Decision { seq }` (`identity.rs:56-70`).
- Produces: a folded `Lot { usd_basis: 0, basis_source: EstimatedConservative, acquired_at: window_end,
  wallet: declared, pseudo: false }`.

- [ ] **Step 1: Failing test** — a `DeclareTranche` folds to the exact lot; and its `Op` is never `Skip`.
```rust
// kat_tranche.rs
use btctax_core::{project, event::{LedgerEvent, EventPayload, DeclareTranche, BasisSource}};
// helpers: build a vault-less LedgerState via project(events, prices, config) — mirror kat_forms.rs setup.

#[test]
fn declare_tranche_folds_to_zero_basis_estimated_conservative_lot_homed_at_window_end() {
    let w = exchange_wallet();
    let ev = decision_event(1, EventPayload::DeclareTranche(DeclareTranche {
        sat: 50_000_000, wallet: w.clone(),
        window_start: date!(2018-01-01), window_end: date!(2018-12-31),
    }));
    let st = project(&[ev], &prices(), &config());
    let lot = st.all_lots().find(|l| l.wallet == w).expect("a tranche lot");
    assert_eq!(lot.usd_basis, Usd::ZERO, "tranche basis is $0 (G-2/D-7)");
    assert_eq!(lot.basis_source, BasisSource::EstimatedConservative);
    assert_eq!(lot.acquired_at, date!(2018-12-31), "acquired_at = window_end (D-2)");
    assert_eq!(lot.original_sat, 50_000_000);
    assert!(!lot.pseudo, "a tranche is filing-ready, NOT pseudo (D-5)");
}
```
- [ ] **Step 2: Run — expect FAIL** (no lot: the decision is skipped by the `EventId::Import`-only timeline filter and `build_op`'s `_ => Op::Skip`).
- [ ] **Step 3: Implement** — three edits:
```rust
// resolve.rs timeline builder (:1055-1083). Today: `_ => continue` drops all decisions.
// Add BEFORE that continue, an explicit admit for a DeclareTranche. GUARD on EventId::Decision (N-4):
// pass-1c ClassifyRaw does NO payload-type validation of `as_`, so a hand-crafted vault could map an
// IMPORT id to a DeclareTranche payload; without the id guard it would enter here with seq semantics
// it doesn't have.
// arch r2 NEW-N-2: match on `&e.payload` DIRECTLY, not `applied.get(&e.id)`. ClassifyRaw's contract
// (resolve.rs:557) scopes overrides to Unclassified imports; decisions are never legitimately overridden,
// and reading through `applied` would let a hand-crafted ClassifyRaw suppress a real tranche or forge one.
if let (EventId::Decision { seq }, EventPayload::DeclareTranche(t)) = (&e.id, &e.payload)
{
    if voided.contains(&e.id) { continue; }               // D-1a-d / arch r3 N-2: a voided tranche folds nothing
    // Effective date = window_end, DECOUPLED from creation utc (D-1a-a): build a projection utc at
    // midnight window_end so eff.date() == window_end and pool_key/conservation bucket correctly. The
    // persisted LedgerEvent.utc_timestamp is untouched (no back-dating). Idiom precedent:
    // `candidate.date.midnight().assume_utc()` at optimize.rs:1280-1297.
    let eff_utc = t.window_end.midnight().assume_utc();
    timeline.push(Eff {
        id: e.id.clone(), utc: eff_utc, tz: UtcOffset::UTC,
        src_priority: u8::MAX,                            // decisions sort after same-instant imports
        // ★ D-1a-b / arch r1 I-1: a CONSTANT src_ref, NOT format!("{seq}"). sort_canonical compares
        //   src_ref (a String-Ord) at key 3, BEFORE any later key — a per-seq string ("2" vs "10")
        //   would decide the tie lexicographically and misorder seq 10 before seq 2 (the exact r3 N-4
        //   failure the SPEC ★-forbids). A constant lets ties fall through to the numeric id key below.
        src_ref: SourceRef::new(""),
        wallet: Some(t.wallet.clone()),
        op: build_op(&e.id, &e.payload, /* … */),         // &e.payload, not `applied` (arch r2 NEW-N-2)
        pseudo: false,                                    // D-5
    });
    let _ = seq; // (seq is the tie-break key, applied in sort_canonical via EventId::Ord)
    continue;
}

// resolve.rs build_op (:258-393): add an arm BEFORE the `_ => Op::Skip`:
EventPayload::DeclareTranche(t) => Op::Acquire(Acquire {
    sat: t.sat, usd_cost: Usd::ZERO, fee_usd: Usd::ZERO,
    basis_source: BasisSource::EstimatedConservative,
}),

// resolve.rs sort_canonical (:1376-1382): add a final tie-break on the EventId itself. EventId derives
// Ord and Decision{seq} compares `seq` as u64 NUMERICALLY (identity.rs) — so two same-window tranches
// (identical utc/src_priority/empty src_ref) order by seq: 2 before 10 (D-1a-b, arch r1 I-1/r3 N-4).
timeline.sort_by(|a, b| a.utc.cmp(&b.utc)
    .then(a.src_priority.cmp(&b.src_priority))
    .then(a.src_ref.cmp(&b.src_ref))
    .then(a.id.cmp(&b.id)));   // Decision{seq}: numeric u64 compare; identical ids never both appear
```
Add the `t.window_end.midnight().assume_utc()` inline (no helper needed).
- [ ] **Step 4: Run — expect PASS.**
- [ ] **Step 5: Mutation** — (a) change `usd_cost: Usd::ZERO` to non-zero → the lot test RED; (b) delete the
  timeline-admit block → RED; (c) revert `src_ref: SourceRef::new("")` back to `format!("{seq}")` → the
  Task-3 **timeline** ordering KAT (`…canonical_timeline`, seqs 2 vs 10) must go RED. (This ONLY discriminates
  because the KAT asserts on `res.timeline`; an `st.lots` assertion would stay GREEN — `finalize` re-sorts
  lots by `lot_id` which already encodes seq numerically — arch r2 NEW-1.)
  Restore each.
- [ ] **Step 6: Commit** `feat(tranche): fold DeclareTranche via Op::Acquire — $0 EstimatedConservative lot at window_end (numeric-seq ordering, id-guarded admit)`.

> **Test-sketch API note (N-1):** the sketches use illustrative helpers. Real APIs: iterate `state.lots`
> (not `st.all_lots()`); disposals are `state.disposals` filtered by year (not `st.disposals_in(y)`);
> `how_acquired_from` is private → add a `pub use` in `forms`/`lib.rs` (already flagged). Mirror
> `crates/btctax-core/tests/kat_forms.rs` for the `project(events, prices, config)` fixture setup.

### Task 3: `DeclareTranche` yields an `Op` never `Skip`; VOIDED folds nothing; is voidable on the product surfaces; same-window ordering (D-1a-c, D-1a-d, D-1a-b)

**Files:** Modify `resolve.rs` (done in Task 2). Test: `kat_tranche.rs`. **Depends on** Task 1's
`is_revocable_payload` arm (arch I-2) for the voidable KAT.

- [ ] **Step 1: Failing tests** (four):
```rust
#[test]
fn declare_tranche_never_yields_op_skip() {
    // Pin D-1a-c: the build_op arm exists, so build_op(DeclareTranche) is not Op::Skip.
    let op = build_op(&EventId::Decision { seq: 1 }, &EventPayload::DeclareTranche(sample_tranche()), /*…*/);
    assert!(!matches!(op, Op::Skip), "a DeclareTranche must fold as an Op, never Skip");
}
#[test]
fn voided_declare_tranche_folds_no_lot() {
    let w = exchange_wallet();
    let t  = decision_event(1, EventPayload::DeclareTranche(sample_tranche_in(&w)));
    let v  = decision_event(2, EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: EventId::Decision { seq: 1 } }));
    let st = project(&[t, v], &prices(), &config());
    assert!(st.lots.iter().find(|l| l.wallet == w).is_none(), "a voided tranche folds nothing (D-1a-d)");
}
#[test]
fn a_tranche_is_listed_as_a_voidable_decision_on_the_product_surface() {
    // arch I-2: voidable_decisions is the SINGLE source of truth for bulk-void + both TUI void flows.
    // Without the is_revocable_payload arm (Task 1), the engine-side void works but the product treats a
    // tranche as PERMANENT — contradicting D-1a-d. Pin the product surface, not only the engine.
    let events = vec![decision_event(1, EventPayload::DeclareTranche(sample_tranche()))];
    let voidable = btctax_core::void::voidable_decisions(&events /* + the resolve inputs it needs */);
    assert!(voidable.iter().any(|id| *id == EventId::Decision { seq: 1 }), "a tranche is sweep-voidable (D-1a-d)");
}
#[test]
fn two_same_window_tranches_are_ordered_by_seq_in_the_canonical_timeline() {
    // ★ D-1a-b / arch r1 I-1 + r2 NEW-1 + r3 NEW-I-1: canonical order is what `sort_canonical` produces,
    // applied by the fold pipeline at `fold.rs:381`. `resolve()` returns the timeline UNSORTED (raw input
    // order), so the KAT must COMPOSE `sort_canonical` explicitly to observe the fix. Asserting on st.lots
    // instead would be blind — `finalize` re-sorts lots by (wallet, acquired_at, lot_id) and LotId Ord
    // compares origin_event_id (Decision{seq}) as u64, so st.lots shows 2-before-10 regardless of the fix.
    let w = exchange_wallet();
    let a = decision_event(2,  EventPayload::DeclareTranche(sample_tranche_in(&w)));  // same window
    let b = decision_event(10, EventPayload::DeclareTranche(sample_tranche_in(&w)));  // same window
    let mut res = btctax_core::project::resolve::resolve(&[b, a], &prices(), &config());  // returns UNSORTED
    btctax_core::project::resolve::sort_canonical(&mut res.timeline);                     // pub fn; project+resolve are pub mods
    let seqs: Vec<u64> = res.timeline.iter()
        .filter_map(|e| match &e.id { EventId::Decision { seq } => Some(*seq), _ => None })
        .collect();
    // Discrimination (all verified against code): revert constant src_ref → "10" < "2" (String-Ord) → [10,2] RED;
    // remove `.then(a.id.cmp(&b.id))` → all keys tie, stable sort preserves push order [10,2] RED; correct → [2,10].
    assert_eq!(seqs, vec![2, 10], "same-window tranche Effs are canonically ordered by numeric seq (D-1a-b)");
}
#[test]
fn two_same_window_tranches_are_additive_not_a_duplicate_conflict() {
    // D-1a-d: two same-window tranches yield TWO lots (no duplicate-conflict). (Observable seq order on the
    // OUTPUT lots is delivered by LotId tie-breaks in finalize/consumption — not by sort_canonical — so
    // this half is captioned as the additivity + observable-order guarantee, not the sort-fix pin.)
    let w = exchange_wallet();
    let a = decision_event(2,  EventPayload::DeclareTranche(sample_tranche_in(&w)));
    let b = decision_event(10, EventPayload::DeclareTranche(sample_tranche_in(&w)));
    let st = project(&[b, a], &prices(), &config());
    let lots: Vec<_> = st.lots.iter().filter(|l| l.wallet == w).collect();
    assert_eq!(lots.len(), 2, "two same-window tranches are additive (D-1a-d)");
    assert_eq!(lots[0].lot_id.origin_event_id, EventId::Decision { seq: 2 });
    assert_eq!(lots[1].lot_id.origin_event_id, EventId::Decision { seq: 10 });
}
```
(Real signature: `voidable_decisions(&[LedgerEvent], &[Blocker]) -> Vec<&LedgerEvent>` — the sketch elides
the `blockers` arg; mirror `crates/btctax-core/tests/voidable.rs`. `build_op` may need a `#[cfg(test)]`
shim.)
- [ ] **Step 2–4:** Task 1 + Task 2 satisfy all five; run to confirm GREEN. If any is RED, that finding wasn't folded — fix it.
- [ ] **Step 5: Mutation** — (a) remove the `voided.contains` guard → `voided_…` RED; (b) revert the Task-1
  `is_revocable_payload` arm → `…voidable_decision…` RED; (c) revert Task-2's constant src_ref → the
  **timeline** seq-order KAT (`…canonical_timeline`) RED (NOT the `st.lots` additivity KAT, which `finalize`
  keeps GREEN — arch r2 NEW-1). Restore each.
- [ ] **Step 6: Commit** `test(tranche): pin no-Skip + voided-folds-nothing + product-voidable + timeline seq order + additivity`.

### Task 4: The `EstimatedConservative` tag survives BOTH overwrite sites (D-8, term-derived)

**Files:**
- Modify: `crates/btctax-core/src/project/transition.rs:83` (Path-A seed), `crates/btctax-core/src/project/fold.rs:812` (relocation).
- Test: `kat_tranche.rs`.

- [ ] **Step 1: Failing tests** (two — the two overwrite sites):
```rust
#[test]
fn tranche_tag_survives_2025_path_a_seed_and_reaches_a_2025_disposal_leg() {
    // A pre-2025 tranche → Path-A reconstructs per-wallet at 2025-01-01; the tag must NOT be overwritten
    // to ReconstructedPerWallet, and a 2025 disposal of it must carry EstimatedConservative + derive term.
    let w = exchange_wallet();
    let t = decision_event(1, EventPayload::DeclareTranche(DeclareTranche {
        sat: 100_000_000, wallet: w.clone(), window_start: date!(2015-01-01), window_end: date!(2015-12-31) }));
    let sell = import_sell(&w, date!(2025-06-01), 100_000_000, usd(60_000)); // > 1yr after window_end
    let st = project(&[t, sell], &prices(), &config_hifo());
    let leg = st.disposals.iter().filter(|d| d.disposed_at.year() == 2025)
        .flat_map(|d| &d.legs).find(|l| l.wallet == w).unwrap();
    assert_eq!(leg.basis_source, BasisSource::EstimatedConservative, "tag survives Path-A seed (D-8)");
    assert_eq!(leg.term, Term::LongTerm, "term DERIVED from window_end (G-4), not assumed");
    // D-6 (inherited from the merged box fix — NOT reimplemented here): the tranche disposal flows to a
    // normal 8949 row, term-aware + year-aware. A 2025 long-term tranche disposal → Part II, Box L.
    let rows = btctax_core::form_8949(&st, 2025);
    let row = rows.iter().find(|r| r.cost_basis == Usd::ZERO).expect("the $0 tranche row");
    assert_eq!(row.part, Form8949Part::LongTerm, "LT → Part II (derived, D-6/G-4)");
    assert_eq!(row.box_, Form8949Box::L, "TY2025 no-1099-DA LT → Box L (inherited)");
    // N-1 (tax): an Exchange-sold tranche row flags box_needs_review — the broker may have issued a
    // 1099-DA, in which case the filer reclassifies to K/H by hand (existing behavior; cheap pin).
    assert!(row.box_needs_review, "exchange-sold tranche → box_needs_review (reclass to K/H if 1099-DA)");
}

#[test]
fn a_short_term_tranche_disposal_is_box_i_never_hard_coded_long_term() {
    // G-4 the other direction: a tranche sold < 1yr after window_end is SHORT-term → Part I / Box I,
    // never silently long-term. Pins that character is derived, not assumed by the conservative flow.
    let w = exchange_wallet();
    let t = decision_event(1, EventPayload::DeclareTranche(DeclareTranche {
        sat: 100_000_000, wallet: w.clone(), window_start: date!(2025-01-01), window_end: date!(2025-02-01) }));
    let sell = import_sell(&w, date!(2025-09-01), 100_000_000, usd(60_000)); // < 1yr after window_end
    let st = project(&[t, sell], &prices(), &config_hifo());
    let row = btctax_core::form_8949(&st, 2025).into_iter().find(|r| r.cost_basis == Usd::ZERO).unwrap();
    assert_eq!(row.part, Form8949Part::ShortTerm);
    assert_eq!(row.box_, Form8949Box::I, "TY2025 no-1099-DA ST → Box I");
}

#[test]
fn holding_period_boundary_is_iff_exactly_one_year() {
    // tax M-1: pin SPEC P1's "LT IFF window_end > 1yr before disposal" END-TO-END through the tranche
    // wiring, at the exact boundary — not only in conventions' unit test. is_long_term is strict `>`
    // (conventions.rs), so a sale EXACTLY one year after window_end is SHORT-term (§1222 / Pub 544
    // day-after convention); one day later is LONG-term.
    let w = exchange_wallet();
    let window_end = date!(2025-03-01);
    let t = decision_event(1, EventPayload::DeclareTranche(DeclareTranche {
        sat: 100_000_000, wallet: w.clone(), window_start: date!(2025-03-01), window_end }));
    // exactly +1yr → ST / Box I
    let st_exact = project(&[t.clone(), import_sell(&w, date!(2026-03-01), 100_000_000, usd(60_000))],
                           &prices(), &config_hifo());
    assert_eq!(st_exact.disposals.iter().flat_map(|d| &d.legs).find(|l| l.wallet == w).unwrap().term,
               Term::ShortTerm, "exactly one year after window_end is SHORT-term (strict >)");
    // +1yr+1day → LT / Box L
    let st_plus1 = project(&[t, import_sell(&w, date!(2026-03-02), 100_000_000, usd(60_000))],
                           &prices(), &config_hifo());
    assert_eq!(st_plus1.disposals.iter().flat_map(|d| &d.legs).find(|l| l.wallet == w).unwrap().term,
               Term::LongTerm, "one day past a year is LONG-term");
}

#[test]
fn a_pre_2025_tranche_disposal_files_the_securities_boxes_c_f() {
    // tax r2 N-2 (adopted): D-6 is year-aware in BOTH directions. A tranche disposed in a pre-2025 tax
    // year files the securities Box C (ST) / F (LT), not the digital-asset I/L. (The year-awareness itself
    // is held by the merged box fix; this pins it end-to-end through the tranche wiring for a back year.)
    let w = exchange_wallet();
    let t = decision_event(1, EventPayload::DeclareTranche(DeclareTranche {
        sat: 100_000_000, wallet: w.clone(), window_start: date!(2015-01-01), window_end: date!(2015-12-31) }));
    let sell = import_sell(&w, date!(2020-06-01), 100_000_000, usd(40_000)); // pre-2025, > 1yr → LT
    let st = project(&[t, sell], &prices(), &config_hifo());
    let row = btctax_core::form_8949(&st, 2020).into_iter().find(|r| r.cost_basis == Usd::ZERO).unwrap();
    assert_eq!(row.box_, Form8949Box::F, "pre-2025 LT tranche → securities Box F, not digital-asset L");
}
#[test]
fn tranche_tag_survives_self_transfer_relocation() {
    // Exchange → SelfCustody relocation (exactly the move P8 recommends) must keep the tag (D-8/arch r2 New-2).
    let ex = exchange_wallet(); let sc = self_custody_wallet();
    let t   = decision_event(1, EventPayload::DeclareTranche(sample_tranche_in(&ex)));
    let link = confirmed_self_transfer(&ex, &sc, /*sat*/ sample_sat());
    let st  = project(&[t, link], &prices(), &config());
    let lot = st.all_lots().find(|l| l.wallet == sc).expect("relocated lot in self-custody");
    assert_eq!(lot.basis_source, BasisSource::EstimatedConservative, "tag survives relocation (D-8)");
}
```
- [ ] **Step 2: Run — expect FAIL** (both overwrite to ReconstructedPerWallet / CarriedFromTransfer).
- [ ] **Step 3: Implement** — exempt the tag at both sites:
```rust
// transition.rs Path-A seed (:83). Was: lot.basis_source = BasisSource::ReconstructedPerWallet;
if lot.basis_source != BasisSource::EstimatedConservative {
    lot.basis_source = BasisSource::ReconstructedPerWallet;  // D-8: tranche tag is exempt
}
// fold.rs relocation arm (:812). Was: basis_source: BasisSource::CarriedFromTransfer,
basis_source: if c.basis_source == BasisSource::EstimatedConservative {
    BasisSource::EstimatedConservative                       // D-8: tranche tag survives relocation
} else { BasisSource::CarriedFromTransfer },
```
(Both are tag-only: Path A already keeps `usd_basis`/`acquired_at` and routes to `PoolKey::Wallet`;
relocation already carries `usd_basis`/`acquired_at` — tax/term/HIFO are identical either way.)
- [ ] **Step 4: Run — expect PASS.**
- [ ] **Step 5: Mutation** — revert each exemption independently → the matching test RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): exempt EstimatedConservative from both basis_source overwrite sites (D-8)`.

### Task 5: Projection-time backstop — an effective Path-B allocation can never coexist with a live tranche residue (D-8 invariant)

The real correctness guarantee (independent of declaration order). Extend `UniversalSnapshot` by one field
and deny a `SafeHarborAllocation` effectiveness when the pre-2025 Universal residue still holds an
`EstimatedConservative` lot with `remaining_sat > 0`.

**Files:**
- Modify: `crates/btctax-core/src/project/transition.rs` — `UniversalSnapshot` (`:19-22`), `universal_snapshot` (`:32-72`).
- Modify: `crates/btctax-core/src/project/resolve.rs` — the conservability check that pushes
  `SafeHarborUnconservable` and keeps the allocation inert (`:1237-1245`).
- Test: `kat_tranche.rs`.

**Interfaces:**
- Produces: `UniversalSnapshot { held_sat, basis, estimated_conservative_remaining_sat: Sat }`.

- [ ] **Step 1: Failing test** — the inert-then-declare ordering (arch r3 New-1):
```rust
#[test]
fn allocation_that_would_conserve_over_a_tranche_residue_is_kept_inert_and_tag_survives() {
    // A pre-2025 tranche whose $0 sats would complete an otherwise-inert SafeHarborAllocation's sat total.
    // The allocation must be DENIED effectiveness (kept inert → Path A → tag survives), via a Hard
    // SafeHarborUnconservable blocker — regardless of which was declared first.
    let w = exchange_wallet();
    let alloc   = safe_harbor_alloc_needing(&w, /*sat that only the tranche completes*/);
    let tranche = decision_event(9, EventPayload::DeclareTranche(pre2025_tranche_in(&w)));
    let st = project(&[alloc, tranche], &prices(), &config());
    assert!(st.blockers.iter().any(|b| matches!(b.kind, BlockerKind::SafeHarborUnconservable)),
            "the allocation is denied effectiveness by a Hard blocker");
    // and the tranche lot is still $0 EstimatedConservative (Path A, not discarded by Path B):
    let lot = st.lots.iter().find(|l| l.basis_source == BasisSource::EstimatedConservative)
        .expect("tranche survives via Path A");
    assert!(lot.remaining_sat > 0);
}
```
- [ ] **Step 2: Run — expect FAIL** (today the allocation may go effective → Path B silently discards the residue).
- [ ] **Step 3: Implement:**
```rust
// transition.rs UniversalSnapshot (:19-22):
pub struct UniversalSnapshot {
    pub held_sat: Sat,
    pub basis: Usd,
    /// D-8 backstop: pre-2025 Universal residue still held under an EstimatedConservative tranche tag
    /// (remaining_sat > 0). Non-zero ⇒ a SafeHarborAllocation must be denied effectiveness.
    pub estimated_conservative_remaining_sat: Sat,
}
// transition.rs universal_snapshot (:32-72): sum remaining_sat over Universal lots whose
// basis_source == EstimatedConservative into the new field.

// resolve.rs conservability check (:1237): BEFORE building the effective seed, if
// snapshot.estimated_conservative_remaining_sat > 0, push a Hard SafeHarborUnconservable blocker and
// keep the allocation inert (the same `continue` path the existing unconservable branch uses).
```
- [ ] **Step 4: Run — expect PASS.**
- [ ] **Step 5: Mutation** — change `> 0` to `> Sat::MAX` (never fires) → RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): projection-time backstop — deny Path-B effectiveness over a live tranche residue (D-8)`.

### Task 6: The friendly record-time refusal — tranche ⇄ in-force allocation (D-8 UX layer)

The backstop (Task 5) is the guarantee; this is the early, friendly error. **Scoping (tax r1 I-2):** the
hazard (Path-B discard) touches ONLY the pre-2025 Universal residue — a tranche with
`window_end ≥ TRANSITION_DATE (2025-01-01)` folds straight into a per-wallet pool with zero Rev-Proc-2024-28
interaction. So:
- **Tranche side:** refuse recording a `DeclareTranche` with `window_end < TRANSITION_DATE` when ANY
  in-force (non-voided) `SafeHarborAllocation` exists — **effective OR inert** (arch r2 New-3: inert can be
  flipped effective by the tranche). A `window_end ≥ 2025` tranche records CLEANLY even alongside an
  effective allocation (else we permanently foreclose P7's MANDATORY disclosure for the mixed-records filer
  with post-2025 undocumented coins — tax r1 I-2).
- **Allocation side:** refuse recording a `SafeHarborAllocation` when a **pre-2025** tranche exists.

**Files (arch r1 I-3 — all FOUR append sites, CLI *and* TUI, not just the CLI):**
- Create: `crates/btctax-cli/src/cmd/tranche.rs` (the tranche record path validates before appending) +
  register in `crates/btctax-cli/src/cmd/mod.rs`.
- Modify the allocation append sites — add the symmetric refusal at each (or route both CLI sites through
  one `session` chokepoint and both TUI sites through one `persist` chokepoint):
  `crates/btctax-cli/src/cmd/reconcile.rs:984` (CLI allocate) + `:1273` (CLI attest);
  `crates/btctax-tui-edit/src/edit/persist.rs:1031` (TUI allocate) + `:1114` (TUI attest).
- Modify `crates/btctax-cli/src/session.rs:681-700` (`safe_harbor_residue`): exclude `DeclareTranche`
  decisions from the pre-2025 allocatable residue (or refuse opening the allocate flow when a tranche
  exists). Else the allocate opener pre-populates the tranche's `$0` sats as allocatable — authoring the
  very coexistence allocation v1 forbids (and, for a ≥2025 tranche, a lot that isn't even pre-2025 residue →
  a guaranteed-unconservable dead-end).
- **Do NOT edit `crates/btctax-core/src/void.rs`** (arch r1 M-5): its `effective_alloc`/`voidable_decisions`
  must stay as-is (inert allocations must remain voidable — pinned by `tests/transition.rs:379`,
  `tests/voidable.rs:170`). The "in-force" predicate here is a NEW record-time check defined at the guard
  sites: payload is `SafeHarborAllocation` ∧ its id not in `voided`.
- Test: `crates/btctax-cli/tests/declare_tranche_cli.rs` (CLI) + a TUI-persist refusal KAT.

**Interfaces:**
- The refusal message HEDGES irrevocability (tax r2 N-3): *"revisit the in-app safe-harbor allocation; if
  your filed allocation is already final, unallocated pre-2025 units are a facts-and-circumstances matter
  for a professional."*

- [ ] **Step 1: Failing tests** — (a) pre-2025 tranche refused under an **effective** allocation (CLI);
  (a2) pre-2025 tranche refused under an **inert** allocation (e.g. timebarred — arch r2 NEW-N-1; needed so
  Step-5(b)'s effective-only mutation can go RED); (b) same refused via the TUI persist path; (c) allocation
  refused under a pre-2025 tranche; (d) **a ≥2025-window tranche records CLEANLY alongside an effective
  allocation** (tax r1 I-2 — the foreclosure guard); (e) each refusal appends NO event; (f)
  `safe_harbor_residue` does not list tranche sats as allocatable.
- [ ] **Step 2: Run — expect FAIL.**
- [ ] **Step 3: Implement** the record-time guards at all four append sites + the `safe_harbor_residue` exclusion.
- [ ] **Step 4: Run — expect PASS.**
- [ ] **Step 5: Mutation** — (a) drop the `window_end < TRANSITION_DATE` scope → the ≥2025-coexists test RED
  (over-broad refusal); (b) scope the tranche-side check to *effective-only* allocations → the inert-direction
  test RED (arch r2 New-3); (c) guard only the CLI sites → the TUI-persist test RED. Restore each.
- [ ] **Step 6: Commit** `feat(tranche): record-time mutual-exclusion (pre-2025-scoped, all 4 append sites + residue) (D-8)`.

### Task 7: The `declare-tranche` CLI verb + clean (non-pseudo) export (D-5)

**Files:**
- Modify: `crates/btctax-cli/src/cli.rs` (a `DeclareTranche` subcommand: `--sat`/`--btc`, `--wallet`,
  `--window-start`, `--window-end`), `src/main.rs` (dispatch), `src/cmd/tranche.rs` (handler → append the event).
- Test: `crates/btctax-cli/tests/declare_tranche_cli.rs` + a `kat_tranche.rs` export-clean KAT.

- [ ] **Step 1: Failing tests** — (a) the verb appends a `DeclareTranche` with `$0` basis and the given window/wallet;
  (b) a year with a filed tranche exports CLEAN (no `[PSEUDO]` banner / no attestation required) —
  `pseudo_active()` stays false (D-5); (c) **input validation (tax M-3):** `--sat 0` (or negative) is
  REFUSED with no event appended — a `sat ≤ 0` tranche would bump `stats.sigma_in` by a non-positive amount
  (`fold.rs:596`), corrupting Σ-conservation; (d) `window_start > window_end` is REFUSED (undefined P5/P7
  window). Warn (not refuse) on a future `window_end` (it merely strands the lot — conservative but confusing).
- [ ] **Step 2–4:** implement the verb (mirror an existing decision-appending command, e.g. the safe-harbor
  or classify verbs) with the `sat > 0` / `window_start ≤ window_end` record-time guards, RED→GREEN.
- [ ] **Step 5: Mutation** — remove the `sat > 0` guard → the `--sat 0` refusal test RED; for the export-clean
  KAT, assert `!report.watermarked` and no `AttestationRequired`. Restore.
- [ ] **Step 6: Commit** `feat(tranche): declare-tranche CLI verb (validated) + clean non-pseudo export (D-5)`.

- [ ] **Phase 1 gate:** `make check` + all CI-only jobs green; then dispatch an independent Fable review
  (tax + architecture lenses) of Phase 1 to 0C/0I before starting Phase 2. Persist verbatim; fold; re-review.

---

## Phase 2 — P2: Steered matching is EMERGENT under HIFO (verify + pin the dependence; D-9)

No new matching code. HIFO's `hifo_cmp` already sorts `usd_basis == 0` lots LAST (`pools.rs:275-287`).

### Task 8: Pin HIFO-draws-documented-first AND the FIFO inversion

**Files:** Test only — `crates/btctax-core/tests/kat_conservative.rs` (new).

> **Method-staging note (arch M-2):** `config.pre2025_method` governs ONLY pre-2025-dated disposals;
> post-2025 method comes from `MethodElection` records and defaults to HIFO (`fold.rs:33-45`). So a
> `config_fifo()` fixture cannot make a 2025+ sale FIFO. Stage BOTH tests in the **pre-2025 Universal pool**
> (tranche `window_end` + documented buy + sale all pre-2025, method = `config.pre2025_method`), OR use a
> forward `MethodElection(Fifo)` for a post-2025 sale. Pin the year explicitly so the fixtures aren't a
> confusing RED.

- [ ] **Step 1: Failing tests** (two) — staged in the pre-2025 pool per the note:
```rust
#[test]
fn under_hifo_a_sale_draws_the_documented_lot_before_the_zero_basis_tranche() {
    // Same wallet: one documented lot (basis > 0) + one tranche ($0), all pre-2025. A partial sale under
    // HIFO consumes the documented lot first (tranche $0 sorts LAST) → higher basis used first (P2).
    let st = project(&[documented_buy(&w), tranche(&w), partial_sell_pre2025(&w /* < total */)],
                     &prices(), &config_hifo_pre2025());
    let leg = only_disposal_leg(&st);
    assert_ne!(leg.basis_source, BasisSource::EstimatedConservative, "HIFO draws the documented lot first");
}
#[test]
fn under_fifo_the_old_zero_basis_tranche_is_consumed_first_inversion() {
    // Pin D-9: under FIFO an OLD $0 tranche (early window_end) is consumed FIRST — a gain-maximizing
    // inversion. Correct application of the in-force method (never an understatement), and the reason
    // P3's method-inversion advisory exists.
    let st = project(&[old_tranche(&w), later_documented_buy(&w), partial_sell_pre2025(&w)],
                     &prices(), &config_fifo_pre2025());
    let leg = only_disposal_leg(&st);
    assert_eq!(leg.basis_source, BasisSource::EstimatedConservative, "FIFO consumes the old tranche first");
}
```
- [ ] **Step 2: Run — expect PASS immediately** (this is a *characterization* pin of existing behavior — the one
  case where a passing-on-write test is correct, because the SPEC's claim is "emergent, no new code"). If either
  FAILS, the emergence assumption is wrong — STOP and escalate (do not add matching code without a SPEC change).
- [ ] **Step 3: Mutation** — n/a (no production change); instead confirm discrimination by flipping the fixture
  method (hifo↔fifo) and observing the assertions swap.
- [ ] **Step 4: Commit** `test(tranche): pin HIFO steering + FIFO inversion (P2/D-9)`.

---

## Phase 3 — P3: Dip + method-inversion advisory (D-9)

### Task 9: `tranche_dip_advisory` + `method_inversion_advisory`

**Files:**
- Create/extend: `crates/btctax-core/src/conservative.rs` — advisory builders.
- **Surface (arch M-3 — the "advisory Vec on the tax outcome" does NOT exist):** `TaxOutcome` is just
  `Computed | NotComputable` (`tax/types.rs:136-139`); report advisories are SCALAR `Option<String>` fields
  on `TaxYearReport` (`crates/btctax-cli/src/cmd/tax.rs:238-247`: `advisory`, `gift_advisory`, appraisal),
  rendered via `render_tax_outcome`'s single `advisory` param (`render.rs:1204`). Add new `Option<String>`
  field(s) on `TaxYearReport` following the `gift_advisory` precedent (populate in `report_tax_year`, render
  in `render_tax_outcome`), and mirror into the TUI Tax tab. **A surfacing KAT is REQUIRED** — as sketched,
  tests that call only the `Option<String>` builder would pass even if P3's user-visible half were dropped.
- Test: `kat_conservative.rs` (builders) + a CLI `report --tax-year` KAT (the advisory reaches stdout).

**Interfaces:**
- Produces: `fn tranche_dip_advisory(disposal) -> Option<String>` (Some iff a matched leg is
  `EstimatedConservative`; names the window, the basis **AS FILED** — `$0` when no fee-sat carry lands on the
  leg (this includes corner-(a) USD-fee disposals, which still file `$0` basis — tax r2 N-4), or the
  documented fee-sat basis when the TP8(c) carry lands on the tranche leg (tax r1 I-1); print `leg.cost_basis`
  directly, do NOT hand-branch on "fee-free" — and the resulting gain; **provenance-neutral** — never asserts
  "purchases"). `fn method_inversion_advisory(state, wallet,
  method) -> Option<String>` (Some iff a non-HIFO in-force method would consume a tranche lot while a
  documented lot remains in the same wallet; recommends a HIFO election).

- [ ] **Step 1: Failing tests** — dip advisory present iff a tranche leg is consumed; inversion advisory present
  iff a non-HIFO method consumes a tranche while a documented lot remains; both absent otherwise; the dip text
  contains no "purchase"/"bought" (provenance-neutral, tax min-8c); **the advisory reaches `report --tax-year`
  stdout** (surfacing KAT); and the fee-carry case states the basis **as filed**, not "$0" (tax r1 I-1).
- [ ] **Step 2–4:** implement the builders + the `TaxYearReport` field + rendering, RED→GREEN.
- [ ] **Step 5: Mutation** — invert each `Option` guard → RED; drop the report-field wiring → the surfacing KAT RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P3 dip + method-inversion advisory (D-9), provenance-neutral, basis-as-filed, surfaced`.

---

## Phase 4 — P4: Custody-aware compliance warning (D-3; reuse)

### Task 10: Fire the existing `ForbiddenBroker2027` envelope for a ≥2027 Exchange specific-ID

**Files:** Test-led reuse of `optimize.rs` `persistability` (`:473-488`) / `is_broker` (`:455-457`). Likely no
new production code beyond wiring the warning into the tranche/advisory path. Test: `kat_conservative.rs`.

- [ ] **Step 1: Failing tests** — fires for a ≥2027 Exchange specific-ID selection; silent for SelfCustody; silent ≤2026.
- [ ] **Step 2–4:** wire it (reuse the envelope; no transfer-statement modeling — D-3). RED→GREEN.
- [ ] **Step 5: Mutation** — n/a if pure reuse; the three KATs (2027-exchange / self-custody / 2026) are the discrimination.
- [ ] **Step 6: Commit** `feat(tranche): P4 custody warning via the existing ForbiddenBroker2027 envelope (D-3)`.

---

## Phase 5 — P5: Window reference-price engine (informational only; NEVER filed — D-7)

### Task 11: `window_reference(prices, start, end) -> Option<Usd>`

**Files:** Extend `crates/btctax-core/src/conservative.rs`. Test: `kat_conservative.rs`.

**Interfaces (arch M-6 — the return type must CARRY the caveat, not just doc it):**
- Produces: `pub fn window_reference(prices: &dyn PriceProvider, start: TaxDate, end: TaxDate) ->
  Option<WindowRef>` where `pub struct WindowRef { pub min: Usd, pub coverage: Coverage }` and
  `pub enum Coverage { Full, Partial }`. The MIN **daily close** over `[start, end]` from the bundled dataset
  (`PriceProvider` is `usd_per_btc(date)` — `price.rs:5-8` — so iterate the days). **NOT a true floor**
  (intraday lows can be lower — tax I-3). Partial overlap → `min` over the covered part + `Coverage::Partial`
  (which P6 MUST surface in user-visible copy — tax r1 N-3, since a covered-part min can EXCEED the true
  window min, inflating "~$X"); no overlap → `None`.

- [ ] **Step 1: Failing tests** — min-close over a range (`Coverage::Full`); partial-overlap returns the
  covered min + `Coverage::Partial`; out-of-range → `None`.
- [ ] **Step 2–4:** implement over the `PriceProvider`, RED→GREEN.
- [ ] **Step 5: Mutation** — change `min` to `max`, and the empty-overlap `None` to `Some(0)` → RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P5 window_reference min-daily-close engine (informational, never filed)`.

---

## Phase 6 — P6: Overpayment-delta nudge (informational; the G-3 lever)

### Task 12: `overpayment_delta` + surface it in `report --tax-year` and the TUI Tax tab

**Files:**
- Extend `crates/btctax-core/src/conservative.rs` — the delta computation. **Mechanism (arch M-4 — the cited
  seam is Dispose-INJECTION, not basis-replacement):** `optimize::synthetic_state` (`optimize.rs:1264-1301`)
  appends a synthetic `Op::Dispose`; computing `tax(window-reference)` instead requires re-folding with the
  tranche's `Op::Acquire.usd_cost` REPLACED by the reference. Add a new clone-fold-discard variant following
  the same pattern: `resolve(events, …)` → swap the tranche Eff's `op` (`Acquire.usd_cost = reference`) →
  `fold` → `compute_tax_year`. It therefore needs `events + prices + config` (NOT a folded `LedgerState`,
  from which no re-fold is possible).
- Surface: `crates/btctax-cli/src/render.rs` (`render_tax_outcome` `:1204`, via the Task-9 `TaxYearReport`
  advisory field) + `crates/btctax-tui/src/tabs/tax.rs` (`:40`).
- Test: `kat_conservative.rs` + a CLI KAT.

**Interfaces (arch M-4 signature corrected; tax r1 M-4 / arch M-7 copy):**
- Produces: `fn overpayment_delta(events, prices, config, year, profile, tables, refs: &[(TrancheId, Usd)])
  -> Usd` = Σ over this year's consumed tranche legs of `tax($0) − tax(reference)`, using the **per-tranche**
  reference (a year consuming legs from multiple differently-windowed tranches must not quote one wrong
  "~$X"); `$0` when references are `$0`/absent. Copy: *"reconstructing this <window> tranche and importing
  the records could save ~$X — at the cost of a documented basis an examiner can question."* The §1014 note
  ships **unconditional and provenance-neutral** (tax r1 M-4 / arch M-7 — `DeclareTranche` carries NO
  provenance field, and adding one would undercut min-8c): *"if these coins were inherited, basis is
  reconstructable by law from date-of-death FMV — no purchase records needed"* (§1014(a); §1223(9) automatic
  LT). When `WindowRef::coverage == Partial`, the copy carries the caveat (tax r1 N-3). Year-scope = tranche
  legs consumed in the report's year, plus a one-line note if undisposed tranche sats remain. **Nothing
  `>$0` is filed** (D-7).

- [ ] **Step 1: Failing tests** — delta = Σ `tax($0) − tax(reference)` for a fixed profile; `$0` when
  references absent; a **multi-window** year sums per-tranche deltas (not a single reference); nudge present
  iff a filed `$0` tranche has a non-zero recoverable delta this year; the §1014 line is present + provenance-
  neutral; the partial-coverage caveat surfaces.
- [ ] **Step 2–4:** implement the basis-replacement what-if variant; render via the Task-9 advisory field + TUI. RED→GREEN.
- [ ] **Step 5: Mutation** — swap the subtraction operands (`reference − $0`) → sign flips, RED; feed one
  reference to a two-window year → the multi-window test RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P6 per-tranche overpayment-delta nudge (basis-replacement what-if), surfaced (G-3, never filed)`.

---

## Phase 7 — P7: Methodology disclosure (D-4; REQUIRED whenever a tranche is filed)

### Task 13: `basis_methodology.txt` export + TUI required-artifact marker

**Files:**
- Extend `crates/btctax-core/src/conservative.rs` — the disclosure text builder.
- Export: alongside the form CSVs in the export dir (the `export-irs-pdf` / CSV export path, `cmd/admin.rs`).
- Surface: TUI marks it a required artifact whenever a tranche is filed.
- Test: `kat_conservative.rs` + a CLI export KAT.

**Interfaces:**
- Produces: `fn basis_methodology(state, year) -> Option<String>` — Some iff a filed tranche exists; enumerates
  each tranche's window + the position **AS FILED** (`$0`, or the documented fee-sat basis when the TP8(c)
  carry lands on the tranche leg — tax r1 I-1; do NOT unconditionally say "$0") + the "records
  unreconstructable → conservative" rationale, **provenance-neutral** (never "purchases"/"bought" — tax
  min-8c) and **term-correct** (states LT/ST as computed — NEVER hard-codes "long-term", G-4).

- [ ] **Step 1: Failing tests** — present iff a filed tranche exists; enumerates each tranche; a filed-tranche
  year WITHOUT it is a hard gap (assert presence); **the text is provenance-neutral** (contains no
  "purchase"/"bought" — tax r1 M-2); and, using a **short-term-held** fixture tranche, the text contains no
  hard-coded "long-term" (an ST fixture is REQUIRED for the mutation to discriminate — a legitimately-LT
  fixture would contain "long-term" in the honest term-correct text, tax r1 M-2).
- [ ] **Step 2–4:** implement + wire the export write + TUI marker. RED→GREEN.
- [ ] **Step 5: Mutation** — hard-code "long-term" in the builder → the no-hard-LT test (ST fixture) RED; make
  the builder always return `None` → the presence test RED; hard-code "purchased" → the provenance test RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P7 mandatory methodology disclosure (basis_methodology.txt), basis-as-filed, provenance-neutral (D-4)`.

---

## Phase 8 — P8: Self-custody nudge (advisory)

### Task 14: `self_custody_nudge`

**Files:** Extend `crates/btctax-core/src/conservative.rs`; surface via the advisory channel. Test: `kat_conservative.rs`.

**Interfaces:**
- Produces: `fn self_custody_nudge(state) -> Option<String>` — Some for an Exchange tranche (suggest holding
  oldest/no-records tranches in SelfCustody, where own-books specific-ID never expires; recommend a HIFO
  election, D-9); absent for a SelfCustody tranche.

- [ ] **Step 1: Failing tests** — present for an Exchange tranche; absent for a SelfCustody tranche.
- [ ] **Step 2–4:** implement, RED→GREEN.
- [ ] **Step 5: Mutation** — invert the wallet guard → RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P8 self-custody nudge (advisory)`.

---

## Phase 9 — Invariant KAT + whole-diff review

### Task 15: No-loss-**from-the-estimate** invariant + the two fee corners (tax min-7, amended SPEC §6 / tax r1 I-1)

**Files:** Test only — `kat_conservative.rs`. **The unqualified "gain ≥ 0" invariant is FALSE on reachable
inputs** (tax r1 I-1): the engine can put a negative gain / `>$0` basis on a tranche row via documented,
real amounts — never the estimate. The scoped invariant + both corners must be pinned so the KAT can't ship
a false claim (and so it would actually STOP on a real understatement).

- [ ] **Step 1: Tests** (three):
```rust
#[test]
fn fee_free_tranche_disposal_never_files_a_loss() {
    // The invariant's true core: absent fees, gain = proceeds − $0 ≥ 0.
    let st = project(&[tranche(&w), fee_free_sell(&w, usd(50_000))], &prices(), &config_hifo());
    assert!(only_disposal_leg(&st).gain >= Usd::ZERO);
    // and a $0-proceeds disposal yields exactly 0, never a loss.
    let st0 = project(&[tranche(&w), fee_free_sell(&w, Usd::ZERO)], &prices(), &config_hifo());
    assert_eq!(only_disposal_leg(&st0).gain, Usd::ZERO);
}
#[test]
fn negative_tranche_gain_comes_only_from_documented_fees_corner_a_usd_fee() {
    // Corner (a): fee_usd > proceeds → net = proceeds − fee_usd < 0 (fold.rs net-netting). The negative
    // gain is attributable to the DOCUMENTED USD fee (§1001(b) reduces amount realized), NOT the $0 estimate.
    let st = project(&[tranche(&w), sell_with_usd_fee(&w, /*proceeds*/ usd(10), /*fee_usd*/ usd(40))],
                     &prices(), &config_hifo());
    let leg = only_disposal_leg(&st);
    assert!(leg.gain < Usd::ZERO, "characterize: a documented USD fee can drive the tranche leg negative");
    assert_eq!(leg.cost_basis, Usd::ZERO, "the ESTIMATE is still $0 — the loss is the fee, not the estimate");
}
#[test]
fn tp8c_fee_sat_basis_can_land_on_the_last_tranche_leg_corner_b() {
    // Corner (b), reachable staging (plan-tax r2 NEW-1 — NOT under pure HIFO, where the $0 tranche is
    // consumed LAST so NO documented lot remains for the fee draw). Use a SPECIFIC-ID sale that names the
    // tranche while a documented lot remains: the on-chain fee then consumes FIFO from the remainder
    // (resolve.rs:1122/:1172), re-homing that DOCUMENTED fee-sat basis onto the (last) tranche leg → its
    // filed cost basis > $0. Real documented basis (§1011), never the estimate. Pins that P3/P7 state
    // basis AS FILED. (Alternate staging: FIFO with pool [D_old, T, D_new] and principal = D_old+T exactly.)
    // Fixture footnote (tax r3 N-6): name the FULL tranche (or make the documented lot FIFO-FIRST in the
    // post-selection remainder) — else a partial naming leaving older-dated $0 tranche sats FIFO-ahead would
    // draw the fee at $0 and the assert would go RED (loud, never a false pass).
    let st = project(&[documented_buy(&w), tranche(&w),
                       specific_id_sell_naming_the_full_tranche(&w /* documented lot remains, FIFO-first */)],
                     &prices(), &config());
    let tranche_leg = only_disposal_leg_from(&st, BasisSource::EstimatedConservative);
    assert!(tranche_leg.cost_basis > Usd::ZERO, "documented fee-sat basis re-homed onto the tranche leg (TP8c)");
}
```
- [ ] **Step 2: Run — expect PASS** (characterization of existing engine behavior). If corner (a)/(b) instead
  showed the *estimate* driving the loss (filed basis ≠ `$0`-plus-documented-fee-sat), that IS a Critical — STOP.
  (M-5: a cent-scale pro-rata rounding remainder (≤ ½¢ per prior leg) on a multi-leg dust sale is a THIRD,
  fee-free non-estimate attribution — Σ-conserving, vanishes at 8949 whole-dollar rounding; add a
  characterization assert if a fixture surfaces it, else it stays documented in the SPEC's Invariant-KAT
  clause.)
- [ ] **Step 3: Mutation** — n/a (characterization); the three tests together are the discrimination (fee-free
  single-leg ≥ 0; the two negatives trace to documented fees with the estimate intact).
- [ ] **Step 4: Commit** `test(tranche): no-loss-from-the-estimate invariant + the two documented-fee corners (tax min-7, SPEC §6 amended)`.

### Task 16: Whole-diff review to green + merge

- [ ] `make check` + `cargo fmt --check` + `cargo run -p xtask -- check-isolation` + `bash scripts/pii-scan-generic.sh`
  + `cargo +1.88 build --workspace` — all green.
- [ ] Dispatch the independent whole-branch Fable review under BOTH lenses (tax + architecture) to 0C/0I.
  Persist each verbatim under `design/conservative-filing/reviews/`; fold; re-review after every fold, incl. the last.
- [ ] Update `FOLLOWUPS.md` (mark conservative-filing v1 shipped; note Approach-B deferrals: floor+8275,
  wizard, VARIOUS multi-date, coexistence). Update memory `conservative-filing-project`.
- [ ] Merge to main is the **owner's call** — present green + merge-ready; do not merge without direction.

---

## Non-goals (v1) — do NOT implement (SPEC §4)
The guided wizard (Approach B); filing a `>$0` floor + its Form 8275 (D-10); VARIOUS multi-date rows;
tranche ⇄ Path-B allocation **coexistence** (v1 makes them mutually exclusive); ProRata auto-split; AMT
compute; non-BTC assets; broker transfer-statement/covered-lot modeling. The shipped 8949-box fix is a
**prerequisite** (already merged), NOT part of this plan — D-6 inherits its corrected box logic.
