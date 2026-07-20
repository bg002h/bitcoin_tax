# Conservative / Defensive Filing — Implementation Plan

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
- `crates/btctax-cli/src/render.rs` — `basis_source_tag` (`:43-57`), `render_tax_outcome` (`:1204-1209`).
- `crates/btctax-tui-edit/src/edit/form.rs` — edit-ring (`:1756-1766`), `basis_source_display` (`:1770-1781`).
- `crates/btctax-cli/src/cli.rs` + `src/main.rs` + a new `src/cmd/tranche.rs` — the `declare-tranche` verb.
- `crates/btctax-tui/src/tabs/tax.rs` (`:40`) — surface P6 nudge + P7 disclosure marker.

---

## Phase 1 — P1: `DeclareTranche` core (the tranche exists, folds, its tag survives, it is mutually exclusive with an effective Path-B allocation)

Everything else depends on Phase 1. Land it fully (Tasks 1–7) before Phase 2.

### Task 1: Schema + exhaustive-`match` sweep (compile-forced)

**Files:**
- Modify: `crates/btctax-core/src/event.rs` — add the `BasisSource` variant + the `DeclareTranche` payload.
- Modify (compile-forced): `crates/btctax-core/src/forms.rs:265-275`, `crates/btctax-cli/src/render.rs:43-57`,
  `crates/btctax-tui-edit/src/edit/form.rs:1756-1766` and `:1770-1781`.
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
Then the four sweep sites (all are exhaustive over the 9 `BasisSource`):
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
```
`EventPayload::DeclareTranche` is a decision, so also add it to any `is_imported`-style match as **not**
imported (it stays out of the `:335-345` imported set — confirm the match is non-exhaustive or add the arm).
- [ ] **Step 4: Run — expect PASS.** Also `cargo build -q -p btctax-core -p btctax-cli -p btctax-tui-edit` clean.
- [ ] **Step 5: Commit** `feat(tranche): BasisSource::EstimatedConservative + EventPayload::DeclareTranche schema + sweep`.

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
// Add BEFORE that continue, an explicit admit for a DeclareTranche (honoring `voided`):
if let EventPayload::DeclareTranche(t) = applied.get(&e.id).unwrap_or(&e.payload) {
    if voided.contains(&e.id) { continue; }               // D-1a / arch r3 N-2: a voided tranche folds nothing
    let seq = match &e.id { EventId::Decision { seq } => *seq, _ => 0 };
    // Effective date = window_end, DECOUPLED from creation utc (D-1a-a): build a projection utc at
    // midnight window_end so eff.date() == window_end and pool_key/conservation bucket correctly.
    // The persisted LedgerEvent.utc_timestamp is untouched (no back-dating).
    let eff_utc = window_end_to_utc(t.window_end);        // helper: OffsetDateTime, midnight UTC on window_end
    timeline.push(Eff {
        id: e.id.clone(), utc: eff_utc, tz: UtcOffset::UTC,
        src_priority: u8::MAX,                            // decisions sort after same-instant imports
        src_ref: SourceRef::new(&format!("{seq}")),       // display only; ties break numerically below
        wallet: Some(t.wallet.clone()),
        op: build_op(&e.id, applied.get(&e.id).unwrap_or(&e.payload), /* … */),
        pseudo: false,                                    // D-5
    });
    continue;
}

// resolve.rs build_op (:258-393): add an arm BEFORE the `_ => Op::Skip`:
EventPayload::DeclareTranche(t) => Op::Acquire(Acquire {
    sat: t.sat, usd_cost: Usd::ZERO, fee_usd: Usd::ZERO,
    basis_source: BasisSource::EstimatedConservative,
}),

// resolve.rs sort_canonical (:1376-1382): add a final NUMERIC decision-seq tie-break (D-1a-b, arch r3 N-4)
// so two same-window tranches are deterministic (they share utc/src_priority/src_ref):
timeline.sort_by(|a, b| a.utc.cmp(&b.utc)
    .then(a.src_priority.cmp(&b.src_priority))
    .then(a.src_ref.cmp(&b.src_ref))
    .then(decision_seq(&a.id).cmp(&decision_seq(&b.id))));  // fn decision_seq(id)->u64: Decision{seq}=>seq else 0
```
Add the `window_end_to_utc` + `decision_seq` helpers privately in `resolve.rs`.
- [ ] **Step 4: Run — expect PASS.**
- [ ] **Step 5: Mutation** — in `build_op`, change `usd_cost: Usd::ZERO` to a non-zero, and separately delete the
  timeline-admit block: both must turn the test RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): fold DeclareTranche via Op::Acquire — $0 EstimatedConservative lot at window_end`.

### Task 3: A `DeclareTranche` yields an `Op`, never `Op::Skip`; a VOIDED tranche folds nothing (D-1a-c, D-1a-d)

**Files:** Modify `resolve.rs` (done in Task 2 — this task adds the guard KATs). Test: `kat_tranche.rs`.

- [ ] **Step 1: Failing tests** (two):
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
    assert!(st.all_lots().find(|l| l.wallet == w).is_none(), "a voided tranche folds nothing (D-1a-d)");
}
```
(If `build_op` is private, expose a `#[cfg(test)] pub(crate)` shim or assert via a projected lot instead.)
- [ ] **Step 2–4:** the Task-2 implementation already satisfies both (the admit checks `voided`); run to confirm GREEN. If `voided.contains` was omitted, add it (RED→GREEN).
- [ ] **Step 5: Mutation** — remove the `if voided.contains(&e.id) { continue; }` guard → `voided_declare_tranche_folds_no_lot` RED. Restore.
- [ ] **Step 6: Commit** `test(tranche): pin no-Skip + voided-folds-nothing guards`.

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
    let leg = st.disposals_in(2025).next().unwrap().legs.iter().find(|l| l.wallet == w).unwrap();
    assert_eq!(leg.basis_source, BasisSource::EstimatedConservative, "tag survives Path-A seed (D-8)");
    assert_eq!(leg.term, Term::LongTerm, "term DERIVED from window_end (G-4), not assumed");
    // D-6 (inherited from the merged box fix — NOT reimplemented here): the tranche disposal flows to a
    // normal 8949 row, term-aware + year-aware. A 2025 long-term tranche disposal → Part II, Box L.
    let rows = btctax_core::form_8949(&st, 2025);
    let row = rows.iter().find(|r| r.cost_basis == Usd::ZERO).expect("the $0 tranche row");
    assert_eq!(row.part, Form8949Part::LongTerm, "LT → Part II (derived, D-6/G-4)");
    assert_eq!(row.box_, Form8949Box::L, "TY2025 no-1099-DA LT → Box L (inherited)");
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
    let lot = st.all_lots().find(|l| l.basis_source == BasisSource::EstimatedConservative)
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

### Task 6: The friendly record-time refusal — tranche ⇄ in-force allocation, BOTH directions (D-8 UX layer)

The backstop (Task 5) is the guarantee; this is the early, friendly error. Refuse recording a
`DeclareTranche` when ANY in-force (non-voided) `SafeHarborAllocation` exists — **effective OR inert** —
and symmetrically refuse recording a `SafeHarborAllocation` when a pre-2025 tranche exists.

**Files:**
- Create: `crates/btctax-cli/src/cmd/tranche.rs` (the record path validates before appending).
- Modify: wherever `EventPayload::SafeHarborAllocation` is appended in `btctax-cli` (grep for its
  constructor) — add the symmetric refusal.
- Test: `crates/btctax-cli/tests/declare_tranche_cli.rs`.

**Interfaces:**
- The refusal message HEDGES irrevocability (tax r2 N-3): *"revisit the in-app safe-harbor allocation; if
  your filed allocation is already final, unallocated pre-2025 units are a facts-and-circumstances matter
  for a professional."* Scope "in-force" = the `effective_alloc` predicate in `void.rs:71-88` widened to
  include inert (not-voided, regardless of `SafeHarborTimebar`/`SafeHarborUnconservable`).

- [ ] **Step 1: Failing test** (both directions) — assert the refusal + that no event is appended.
- [ ] **Step 2: Run — expect FAIL.**
- [ ] **Step 3: Implement** the two record-time guards (event-scan for any in-force allocation / any pre-2025 tranche).
- [ ] **Step 4: Run — expect PASS.**
- [ ] **Step 5: Mutation** — scope the tranche-side check to *effective-only* allocations → the inert-direction test RED (this is exactly the arch r2 New-3 bug). Restore.
- [ ] **Step 6: Commit** `feat(tranche): record-time mutual-exclusion refusal, both directions, hedged copy (D-8)`.

### Task 7: The `declare-tranche` CLI verb + clean (non-pseudo) export (D-5)

**Files:**
- Modify: `crates/btctax-cli/src/cli.rs` (a `DeclareTranche` subcommand: `--sat`/`--btc`, `--wallet`,
  `--window-start`, `--window-end`), `src/main.rs` (dispatch), `src/cmd/tranche.rs` (handler → append the event).
- Test: `crates/btctax-cli/tests/declare_tranche_cli.rs` + a `kat_tranche.rs` export-clean KAT.

- [ ] **Step 1: Failing tests** — (a) the verb appends a `DeclareTranche` with `$0` basis and the given window/wallet;
  (b) a year with a filed tranche exports CLEAN (no `[PSEUDO]` banner / no attestation required) —
  `pseudo_active()` stays false (D-5).
- [ ] **Step 2–4:** implement the verb (mirror an existing decision-appending command, e.g. the safe-harbor
  or classify verbs), RED→GREEN.
- [ ] **Step 5: Mutation** — n/a for the verb wiring beyond the KATs; for the export-clean KAT, assert
  `!report.watermarked` and no `AttestationRequired`.
- [ ] **Step 6: Commit** `feat(tranche): declare-tranche CLI verb + clean non-pseudo export (D-5)`.

- [ ] **Phase 1 gate:** `make check` + all CI-only jobs green; then dispatch an independent Fable review
  (tax + architecture lenses) of Phase 1 to 0C/0I before starting Phase 2. Persist verbatim; fold; re-review.

---

## Phase 2 — P2: Steered matching is EMERGENT under HIFO (verify + pin the dependence; D-9)

No new matching code. HIFO's `hifo_cmp` already sorts `usd_basis == 0` lots LAST (`pools.rs:275-287`).

### Task 8: Pin HIFO-draws-documented-first AND the FIFO inversion

**Files:** Test only — `crates/btctax-core/tests/kat_conservative.rs` (new).

- [ ] **Step 1: Failing tests** (two):
```rust
#[test]
fn under_hifo_a_sale_draws_the_documented_lot_before_the_zero_basis_tranche() {
    // Same wallet: one documented lot (basis > 0) + one tranche ($0). A partial sale under HIFO consumes
    // the documented lot first (tranche $0 sorts LAST) → higher basis used first (P2).
    let st = project(&[documented_buy(&w), tranche(&w), partial_sell(&w /* < total */)], &prices(), &config_hifo());
    let leg = only_disposal_leg(&st);
    assert_ne!(leg.basis_source, BasisSource::EstimatedConservative, "HIFO draws the documented lot first");
}
#[test]
fn under_fifo_the_old_zero_basis_tranche_is_consumed_first_inversion() {
    // Pin D-9: under FIFO an OLD $0 tranche (early window_end) is consumed FIRST — a gain-maximizing
    // inversion. Correct application of the in-force method (never an understatement), and the reason
    // P3's method-inversion advisory exists.
    let st = project(&[old_tranche(&w), later_documented_buy(&w), partial_sell(&w)], &prices(), &config_fifo());
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
- Surface: wherever disposal advisories are already rendered (reuse the existing advisory channel the
  report/TUI use — grep for the advisory `Vec` on the tax outcome).
- Test: `kat_conservative.rs`.

**Interfaces:**
- Produces: `fn tranche_dip_advisory(disposal) -> Option<String>` (Some iff a matched leg is
  `EstimatedConservative`; names the window, `$0` basis, resulting gain; **provenance-neutral** — never
  asserts "purchases"). `fn method_inversion_advisory(state, wallet, method) -> Option<String>` (Some iff
  a non-HIFO in-force method would consume a tranche lot while a documented lot remains in the same wallet;
  recommends a HIFO election).

- [ ] **Step 1: Failing tests** — dip advisory present iff a tranche leg is consumed; inversion advisory present
  iff a non-HIFO method consumes a tranche while a documented lot remains; both absent otherwise; assert the
  dip text contains no "purchase"/"bought" (provenance-neutral, tax min-8c).
- [ ] **Step 2–4:** implement, RED→GREEN.
- [ ] **Step 5: Mutation** — invert each `Option` guard → RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P3 dip + method-inversion advisory (D-9), provenance-neutral`.

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

**Interfaces:**
- Produces: `pub fn window_reference(prices: &dyn PriceProvider, start: TaxDate, end: TaxDate) -> Option<Usd>`
  — the MIN **daily close** over `[start, end]` from the bundled dataset. **NOT a true floor** (intraday lows
  can be lower — tax I-3); caveated in the doc + P6 copy. Partial overlap → min over the covered part **with a
  caveat**; no overlap → `None`.

- [ ] **Step 1: Failing tests** — min-close over a range; partial-overlap returns the covered min (flagged);
  out-of-range → `None`.
- [ ] **Step 2–4:** implement over the `PriceProvider`, RED→GREEN.
- [ ] **Step 5: Mutation** — change `min` to `max`, and the empty-overlap `None` to `Some(0)` → RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P5 window_reference min-daily-close engine (informational, never filed)`.

---

## Phase 6 — P6: Overpayment-delta nudge (informational; the G-3 lever)

### Task 12: `overpayment_delta` + surface it in `report --tax-year` and the TUI Tax tab

**Files:**
- Extend `crates/btctax-core/src/conservative.rs` — the delta computation (reuses the `whatif.rs` /
  `optimize::synthetic_state` clone-fold-discard seam, `optimize.rs:1263-1301`).
- Surface: `crates/btctax-cli/src/render.rs` (`render_tax_outcome` `:1204-1209`, below the tranche figures)
  + `crates/btctax-tui/src/tabs/tax.rs` (`:40`).
- Test: `kat_conservative.rs` + a CLI KAT.

**Interfaces:**
- Produces: `fn overpayment_delta(state, year, profile, tables, reference: Usd) -> Usd` = `tax($0) −
  tax(window-reference)` for this year's consumed tranche legs; `$0` when the reference is `$0`/absent. Copy:
  *"reconstructing this <window> tranche and importing the records could save ~$X — at the cost of a
  documented basis an examiner can question."* For an **inherited** tranche, additionally note §1014
  date-of-death FMV reconstruction (tax min-8a). Year-scope = tranche legs consumed in the report's year,
  plus a one-line note if undisposed tranche sats remain. **Nothing `>$0` is filed** (D-7).

- [ ] **Step 1: Failing tests** — delta = `tax($0) − tax(reference)` for a fixed profile; `$0` when reference
  absent; nudge present iff a filed `$0` tranche has a non-zero recoverable delta this year.
- [ ] **Step 2–4:** implement via the what-if seam; render below the tranche figures + in the TUI Tax tab. RED→GREEN.
- [ ] **Step 5: Mutation** — swap the subtraction operands (`reference − $0`) → sign flips, RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P6 overpayment-delta nudge in report + TUI Tax tab (G-3, never filed)`.

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
  each tranche's window + `$0` position + the "records unreconstructable → conservative" rationale,
  **provenance-neutral** and **term-correct** (states LT/ST as computed — NEVER hard-codes "long-term", G-4).

- [ ] **Step 1: Failing tests** — present iff a filed tranche exists; enumerates each tranche; a filed-tranche
  year WITHOUT it is a hard gap (assert presence); the text contains no hard-coded "long-term".
- [ ] **Step 2–4:** implement + wire the export write + TUI marker. RED→GREEN.
- [ ] **Step 5: Mutation** — hard-code "long-term" in the builder → the no-hard-LT test RED; make the builder
  always return `None` → the presence test RED. Restore.
- [ ] **Step 6: Commit** `feat(tranche): P7 mandatory methodology disclosure (basis_methodology.txt) (D-4)`.

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

### Task 15: No-loss invariant (tax min-7)

**Files:** Test only — `kat_conservative.rs`.

- [ ] **Step 1: Test** — a `$0`-basis tranche disposal can NEVER produce a loss (`gain = proceeds − $0 ≥ 0`),
  so no §1211/§1212/§1091 interaction from a v1 tranche. Build a disposal of a tranche at any positive
  proceeds and assert `leg.gain >= 0`; also assert a `$0`-proceeds disposal yields exactly `0`, not a loss.
- [ ] **Step 2: Run — expect PASS** (characterization). If it can go negative, that's a Critical — STOP.
- [ ] **Step 3: Commit** `test(tranche): no-loss invariant for $0-basis tranche disposals (tax min-7)`.

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
