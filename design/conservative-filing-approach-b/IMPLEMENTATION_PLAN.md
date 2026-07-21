# Conservative-Filing Approach B — Sub-project 1 (Basis-Floor Engine + Form 8275) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** DRAFT — **plan-review r1 FOLDED** (tax 0C/8I/7M/3N + arch 0C/6I/4M/2N, both persisted verbatim in
`reviews/plan-{tax,architecture}-fable-review-r1.md`); pending the **r2** re-review to 0C/0I per
`STANDARD_WORKFLOW.md` **before any execution**. r1 confirmed the engine core (T2–T7 decomposition/evaporation/
void-adjudication) faithful + all ~60 source citations accurate; the folded findings were completeness/wiring
gaps — the `PromoteSet` threading (T2/T3/T4), the void-adjudication insertion point (T7), the advisory's call
sites both directions (T8/T10), the verify-drift task (T11), `ConsentTerm`/`Printed8275` shapes (T1/T13), the
8275 Part-I amount + year coverage (T13/T15), and under-pinned KATs.

**Goal:** Let a filer knowingly promote a v1 `$0`-basis conservative tranche to a filed **`>$0` basis floor**
(window-min daily close), backed by a mandatory **Form 8275**, with every gate/advisory/decomposition the GREEN
spec (`design/conservative-filing-approach-b/SPEC.md`, BG-D1..D11) mandates — never silently understating.

**Architecture:** A new `EventPayload::PromoteTranche` decision is **layered on** an existing `DeclareTranche`
and resolved as **pass-2 Op-construction INSIDE `resolve`** — it rewrites the target's `Op::Acquire.usd_cost`
to the stored floor while the lot keeps `BasisSource::EstimatedConservative` (BG-D1: **no new `BasisSource`**),
so every v1 by-construction guarantee (D-8 backstop, Path-A seed, relocation carve, record-time refusals,
tranche⇄safe-harbor exclusion) holds unchanged. The estimate attaches ONLY to disposal-leg gain (clamped,
BG-D4); on every other basis path it is documented-only (removal legs, BG-D11) or evaporates (fee draws). The
whole thing is gated by a record-time provenance attestation (BG-D5), a two-sided informed-consent
acknowledgment (BG-D6), a mandatory auto-generated Form 8275 (BG-D7), and a real export-refusal gate (BG-D8).

**Tech Stack:** Rust workspace (`btctax-core`, `btctax-cli`, `btctax-forms`, `btctax-tui-edit`); `serde` vault
persistence; `lopdf` fillable-PDF forms; `cargo nextest` + `clippy`; TDD (`cargo-mutants` optional per primitive).

**Phasing (ONE ship gate — SPEC §4):** Phase 1a = the engine + 8275 *content*; Phase 1b = the official Form
8275 *fillable PDF*. Per-phase MERGE to `main` is authorized (each phase after its own whole-diff two-lens
review to 0C/0I + full suite green), but **RELEASE (crate publish) waits for the complete 1a+1b unit** — Reg
§1.6662-4(f) makes disclosure adequate only on a completed Form 8275, so `promote` must not reach a *released*
binary without the PDF. (`main` is unreleased between; there are no users yet.)

## Global Constraints

Every task's requirements implicitly include this section. Exact values copied from the SPEC:

- **BG-D1 (load-bearing):** NO new `BasisSource` variant. A promoted tranche stays
  `BasisSource::EstimatedConservative`. The promote rewrites ONLY `Op::Acquire.usd_cost`, applied INSIDE the
  `resolve` timeline build (so pass-1 §7.4 effectiveness + `universal_snapshot` see the floor) — NEVER as a
  post-`resolve` timeline mutation. `acquired_at`/term/holding-period are byte-identical pre/post promote.
- **BG-D2:** `FloorMethod = { WindowLowClose }` only (enum kept for future extension). No §1014, no PartialRecords.
- **BG-D3:** `filed_basis` is COMPUTED (no `--basis` flag), `Coverage::Full` REQUIRED (Partial/None → hard
  refuse), and STORED on the event (whole-tranche `round_cents(window_min_close_price × sat / SATS_PER_BTC)` —
  NOT a per-BTC price). No `acquired_at` field on the payload. `verify` flags drift, direction-aware.
- **BG-D4:** estimate-claimed basis per disposal leg = `clamp(net_proceeds_share, $0, estimate_share)` =
  `min(estimate_share, max(net_proceeds_share, $0))`. `estimate_share = filed_basis × leg.sat / tranche_sat`
  keyed via `leg.lot_id.origin_event_id` → the promote set (NOT `lot.usd_basis`, which merges the TP8(c) fee
  carry). `documented_share = usd_basis_share − estimate_share`, UNCLAMPED. Unclaimed floor EVAPORATES.
- **BG-D4 fee rule:** the estimate component of FIFO-consumed fee-sats EVAPORATES; only documented fee basis
  re-homes via `FeeCarry`.
- **BG-D11:** the estimate NEVER funds a deduction or outbound carry — decomposed at the REMOVAL-leg builder
  (`make_removal_legs`), documented-only, so all consumers (fold `claimed_deduction`, `crypto_charitable_gifts`
  → Schedule A, Form 8283 `cost_basis` column, `removals.csv`, §1015 carryover) inherit by construction.
- **BG-D5:** record-time purchase-provenance attestation; refuse gift/inheritance/mining/staking-earning/
  airdrop/fork/any-non-purchase, pointing them at real-acquisition modeling.
- **BG-D6:** two-sided consent, figures re-folded through the CLAMPED path; Σ over every year the fold-diff
  flags INCLUDING the current year; a removal-flagged year quotes deduction-Δ (donations) or §1015
  carryover-basis-Δ (gifts), never a bare $0; uncomputable years surfaced loud (gain/deduction-Δ, never silent
  $0); the cross-year §1212(b)/§170(d) cascade NAMED. Typed `Acknowledgment` recorded ON the event (three
  flavors: computed-tax-Δ / gain-deduction-Δ-with-uncomputable-flag / named-unquantified cascade). Non-TTY path:
  `--i-acknowledge <phrase>`.
- **BG-D7:** Form 8275 MANDATORY for every promote. Copy NEVER says "safe harbor" or promises immunity; states
  the 20%/40% worst case **against the underpayment/additional tax** (not the disallowed basis); clamped-leg
  narrative adds "limited so as not to report a loss from the estimate". Part II filer-facts narrative captured
  ON the event, empty/scaffold-only REFUSED at record time.
- **BG-D8:** export-time completeness gate is a REAL refusal (pseudo-export-block precedent, refuse-before-bytes
  on CSV / `export-irs-pdf` / full-return), NOT the always-written `basis_methodology.txt` pattern. Clean
  export, no watermark.
- **BG-D9:** engine-adjudicated lifecycle. Second promote → `DecisionConflict`. Void-of-tranche-with-live-promote
  → resolver-inert + `DecisionConflict` (deferred adjudication against the FINAL non-voided-promote set,
  mirroring `allocation_voids`); a non-voided promote with an absent/wrong-type target → hard `DecisionConflict`;
  `PromoteTranche` in `is_revocable_payload`/`voidable_decisions`, its `DeclareTranche` target excluded from the
  bulk-void candidate set while a live promote exists. Prior-year advisory fires on a FOLD-DIFF over disposal
  AND removal legs (leg-SET diff, NOT `tax_total`, NOT Σ-gain), both directions; §6511 + conditional 1040-X copy;
  the §1212(b)/§170(d) carryover cascade NAMED.
- **BG-D10:** §6662(e)/(h) 40% gross-valuation-misstatement risk disclosed (*Woods*; Reg §1.6662-5(g); adequate
  disclosure does NOT shield §6662(b)(3)); §6664(c)(2) removes the reasonable-cause defense for
  charitable-deduction-property valuation misstatements (§6664(c)(3) = qualified-appraisal special rule).
- **Two censuses (SPEC §3):** the tag-side (`== EstimatedConservative` copy/math) AND the payload-side (no
  compile-forced `EventPayload` match — enumerate every consumer). Both must be swept.
- **Validation:** `make check` (workspace nextest + clippy) is the fast gate. Green = full suite + all CI-only
  jobs (fmt / check-isolation / pii-scan / msrv) + 0C/0I two-lens review. Every ruling pinned by a §6 KAT
  (mutation-proven where practical — the "name the mutation each test kills" discipline; there is no
  `cargo-mutants` tooling).
- **Plan test/code idioms (real, from the harness — do NOT reinvent):** `Usd = Decimal` (`conventions.rs`) —
  build test money with `dec!(12_000)` (the `rust_decimal::dec!` macro the KATs use), NEVER a nonexistent
  `Usd::from_dollars`; whole-sat→Usd via `Usd::from(sat)`. `EventId` has no `Display` — render it with
  `.canonical()` (`identity.rs`), never `{}`/`format!("{}", id)`. Core-KAT harness = `kat_tranche.rs`
  (`exch()`, `dec_ev`, `tranche_ev`, `void_ev`, `prices()`, `cfg()`, `project(...)`); CLI harness =
  `declare_tranche_cli.rs` (`pp()`, `now()`, vault builders, `count()`); forms KATs = the geometric read-back
  oracle (`sp2.rs`/`sp3.rs`). Every code snippet below is illustrative shape, not verbatim — the implementer
  matches surrounding style.

---

## File-Structure Map

**Created:**
- `crates/btctax-core/src/conservative_promote.rs` — the promote's engine helpers: `filed_basis_for`
  (BG-D3 compute), `PromoteSet` (`DeclareTranche EventId → {filed_basis, tranche_sat}` lookup built from the
  resolved events), the BG-D4 `estimate_share`/clamp decomposition, and the fold-diff/consent/cascade helpers
  (Tasks 2, 4, 8, 9). Keeps `conservative.rs` (already ~450 lines of advisories) from growing unwieldy; the
  new module is `mod conservative_promote;` re-exported through `conservative`.
- `crates/btctax-core/src/tax/form8275.rs` — the 8275 *content* generator (Part I auto + the stored Part II);
  the disclosure struct `Disclosure8275` the CLI/forms consume (Task 13).
- `crates/btctax-cli/src/cmd/promote.rs` — the `promote-tranche` record path + the provenance/consent gates
  (Task 10), sibling to `cmd/tranche.rs`.
- `crates/btctax-forms/src/form8275.rs` + `crates/btctax-forms/forms/2024/f8275.{pdf,map.toml}` — the official
  fillable PDF (Phase 1b, Tasks 15–16).
- Tests: `crates/btctax-core/tests/kat_promote.rs`, `crates/btctax-cli/tests/promote_cli.rs`,
  `crates/btctax-forms/tests/sp4.rs` (8275 fill KATs).

**Modified (by task):**
- `crates/btctax-core/src/event.rs` — `EventPayload::PromoteTranche` + `FloorMethod`/`Acknowledgment`/
  `ConsentTerm` types + serde/vault-compat KATs (T1). NO change to `BasisSource` (BG-D1).
- `crates/btctax-core/src/conservative.rs` — add `Serialize, Deserialize` to `Coverage` (T1); make the five
  advisories promote-aware (T11); the consent seam (T9).
- `crates/btctax-core/src/persistence.rs` — no code change (the `_ => None` catch-all covers a decision); KAT only (T1).
- `crates/btctax-core/src/project/resolve.rs` — the step-2 promote rewrite + step-1a/step-3 void adjudication (T3, T7).
- `crates/btctax-core/src/project/fold.rs` — the BG-D4 disposal-leg clamp/decomposition (T4), the `consume_fee`
  evaporation (T5), the `make_removal_legs` documented-only decomposition (T6).
- `crates/btctax-core/src/void.rs` — `is_revocable_payload` + the promoted-target exclusion closure (T7, T12).
- `crates/btctax-core/src/session.rs`, `crates/btctax-cli/src/main.rs`, `crates/btctax-tui-edit/src/main.rs` —
  payload-side render arms (T12).
- `crates/btctax-core/src/forms.rs`, `crates/btctax-core/src/tax/{return_1040,printed}.rs` — verified to inherit
  BG-D11 by construction; KAT-covered, not independently patched (T6).
- `crates/btctax-core/src/tax/compute.rs`, `crates/btctax-cli/src/cmd/tax.rs` — carryover-cascade naming hooks (T8).
- `crates/btctax-cli/src/cli.rs`, `main.rs`, `render.rs`, `cmd/admin.rs`, `lib.rs` — the CLI verb (T10), the §3
  copy sweep (T11), the real export-refusal gate (T14).
- `crates/btctax-forms/src/{lib,pdf,map,packet}.rs`, `crates/btctax-core/src/tax/packet.rs`,
  `crates/btctax-forms/tests/{sp3,census}.rs`, `docs/`, `LIMITATIONS.md` — the 8275 PDF wiring (T15–T16).

Each task ends with an independently testable deliverable and a commit. A task's `Produces` block is the only
place a later task learns its neighbors' exact names/types (implementers see only their own task).

---

# PHASE 1a — the engine + Form 8275 content

### Task 1: `PromoteTranche` event schema + typed payload

**Files:**
- Modify: `crates/btctax-core/src/event.rs` (add the variant near `DeclareTranche`, event.rs:214-355)
- Modify: `crates/btctax-core/src/conservative.rs:173-177` (add `Serialize, Deserialize` to `Coverage`)
- Test: `crates/btctax-core/src/event.rs` `mod tests` (serde round-trip ~:406-582; no-fingerprint ~:585)

**Interfaces:**
- Consumes: `EventId`, `Usd`, `crate::conservative::Coverage` (existing).
- Produces (every later task consumes these exact shapes):
  ```rust
  // event.rs
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
  pub enum FloorMethod { WindowLowClose }               // BG-D2: exactly one method

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ConsentTerm {                                // BG-D6 flavors (arch r5 N-1; plan-r1 tax I-4/I-5)
      // A COMPUTING year. `deduction_delta_usd` is Some when a removal (donation/gift) leg diffed — engine B's
      // computed tax-Δ EXCLUDES crypto donations by design (tax r3 I-2), so the deduction effect rides HERE,
      // fold-pair-derived, and the copy must say the tax-Δ does not capture it.
      ComputedTax { year: i32, delta_usd: Usd, deduction_delta_usd: Option<Usd> },
      // A year the tax engine can't price (no table/profile/blocked): the profile-free fold-pair deltas.
      Uncomputable { year: i32, gain_delta_usd: Usd, deduction_delta_usd: Usd },
      // Undisposed sats: hypothetical-not-filed line, with the tax r3 N-2 no-current-price fallback.
      Unrealized { sat: Sat, hypothetical_reduction: Option<Usd>, as_of: Option<TaxDate> },
      CascadeNamed { year: i32 },                       // §1212(b)/§170(d), named-unquantified (tax r4 I-1)
  }

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct Acknowledgment {                           // BG-D6; the §6664(c) good-faith artifact
      pub phrase: String,                               // the typed consent phrase
      pub shown_terms: Vec<ConsentTerm>,                // snapshot of the exact figures shown
      pub provenance_text: String,                      // BG-D5 attested statement (verbatim)
      pub provenance_version: String,                   // attestation-text version
  }

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct PromoteTranche {
      pub target: EventId,          // the DeclareTranche decision this promotes (BG-D1)
      pub method: FloorMethod,      // BG-D2
      pub filed_basis: Usd,         // BG-D3: WHOLE-tranche, computed at record time + STORED
      pub coverage: Coverage,       // BG-D3 snapshot (Full required at record time)
      pub provenance_attested: bool,// BG-D5
      pub acknowledgment: Acknowledgment, // BG-D6
      pub part_ii_narrative: String,      // BG-D7 (empty/scaffold-only refused at record time, T10)
  }
  // + enum arm:  EventPayload::PromoteTranche(PromoteTranche)
  ```

- [ ] **Step 1: Write the failing serde round-trip + no-fingerprint KATs.** In `event.rs mod tests`, add a
  `PromoteTranche` value to the `payloads` vec of `every_variant_serde_round_trips` (:406-582), and add:
  ```rust
  #[test]
  fn promote_tranche_decision_has_no_fingerprint() {
      // Mirrors declare_tranche_decision_has_no_fingerprint: decisions are never fingerprinted.
      let p = EventPayload::PromoteTranche(PromoteTranche {
          target: EventId::decision(1),
          method: FloorMethod::WindowLowClose,
          filed_basis: dec!(12_000),
          coverage: crate::conservative::Coverage::Full,
          provenance_attested: true,
          acknowledgment: Acknowledgment {
              phrase: "I understand and accept the risk".into(),
              shown_terms: vec![],
              provenance_text: "acquired by purchase within the declared window".into(),
              provenance_version: "v1".into(),
          },
          part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
      });
      assert!(crate::persistence::fingerprint(&p).is_none());
  }
  ```
- [ ] **Step 2: Run — expect FAIL** (type `PromoteTranche` not found; `Coverage` not `Serialize`).
  Run: `cargo test -p btctax-core --lib event::tests -- promote_tranche 2>&1 | tail -20`
- [ ] **Step 3: Add the types + enum arm.** Insert the four items above into `event.rs`; add the enum arm
  `PromoteTranche(PromoteTranche)` after `DeclareTranche(DeclareTranche)` (:354) with the **forward-only
  vault-compat doc note** (copy the `DeclareTranche` doc pattern at :214-220 / the long-form at
  `ReclassifyIncome` :234-240 — "Old-binary limitation: a vault containing this variant fails to load on a
  pre-promote binary (serde unknown-variant) — harmless, no installed base"). Add `Serialize, Deserialize` to
  `Coverage` in `conservative.rs:173-177`.
- [ ] **Step 4: Run — expect PASS.** Run: `cargo test -p btctax-core --lib event::tests 2>&1 | tail -20`
  (both the round-trip and no-fingerprint KATs green). Then `make check` for the workspace.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/event.rs crates/btctax-core/src/conservative.rs
  git commit -m "feat(promote): PromoteTranche event schema + typed Acknowledgment/ConsentTerm (BG-D1/D6)"
  ```

**Mutation to kill (name it in the test):** dropping `Serialize` on `Coverage` reds the round-trip; a stray
`fingerprint` arm for `PromoteTranche` reds the no-fingerprint KAT. NO new `BasisSource` variant is added — the
`BasisSource` enum is untouched (BG-D1); a KAT in Task 3 pins that the promoted lot still reads `EstimatedConservative`.

### Task 2: `filed_basis` compute + `Coverage::Full` guard (BG-D3)

**Files:**
- Create: `crates/btctax-core/src/conservative_promote.rs` (+ `mod conservative_promote;` in `lib.rs`; re-export)
- Reference: `crates/btctax-core/src/conservative.rs:193-197` (`window_reference` → `WindowRef{min: Usd/BTC, coverage}`)
- Test: `crates/btctax-core/tests/kat_promote.rs` (new)

**Interfaces:**
- Consumes: `window_reference(prices, start, end) -> Option<WindowRef>`; `WindowRef { min: Usd, coverage: Coverage }`
  (`min` is USD **per whole BTC** — a price); `round_cents`, `SATS_PER_BTC`.
- Produces (in `conservative_promote.rs` — the ★ SHARED types every leg-builder task (T4/T5/T6) consumes;
  defined HERE so they have one owner, arch r1 I-1):
  ```rust
  // BG-D3 compute. Returns Err with the exact refusal copy on Partial/None coverage.
  pub struct ComputedFloor { pub filed_basis: Usd, pub coverage: Coverage } // filed_basis = whole-tranche
  pub fn filed_basis_for(
      prices: &dyn PriceProvider, sat: Sat, window_start: TaxDate, window_end: TaxDate,
  ) -> Result<ComputedFloor, PromoteRefusal>;   // PromoteRefusal: NoCoverage | PartialCoverage

  // The BG-D4/D11 decomposition key. `tranche_sat` is the denominator; both fields are needed at the leg
  // builders (T4/T5/T6). Built by resolve (T3) and threaded to the fold (T4). Keyed by the target
  // DeclareTranche EventId == a promoted leg's `lot_id.origin_event_id`.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct PromoteEntry { pub filed_basis: Usd, pub tranche_sat: Sat }
  pub type PromoteSet = std::collections::BTreeMap<EventId, PromoteEntry>;
  ```

- [ ] **Step 1: Write the failing tests** in `kat_promote.rs`:
  ```rust
  // BG-D3: whole-tranche scaling (per-BTC price × sat / SATS_PER_BTC), NOT a per-BTC price.
  #[test]
  fn filed_basis_is_whole_tranche_scaled() {
      let prices = prices_with_window_min(12_000); // helper: min daily close = $12,000/BTC, Full coverage
      let cf = filed_basis_for(&prices, 50_000_000 /* 0.5 BTC */, d("2017-12-01"), d("2017-12-31")).unwrap();
      assert_eq!(cf.filed_basis, dec!(6_000)); // 12_000 × 0.5, not 12_000
      assert_eq!(cf.coverage, Coverage::Full);
  }
  #[test]
  fn partial_coverage_is_hard_refused() {
      let prices = prices_with_partial_window(); // some day in [start,end] has no close
      let err = filed_basis_for(&prices, 100_000_000, d("2013-01-01"), d("2017-12-31")).unwrap_err();
      assert!(matches!(err, PromoteRefusal::PartialCoverage));
  }
  ```
- [ ] **Step 2: Run — expect FAIL** (`filed_basis_for` undefined).
  Run: `cargo test -p btctax-core --test kat_promote -- filed_basis 2>&1 | tail -20`
- [ ] **Step 3: Implement** `filed_basis_for`: call `window_reference`; `None → Err(NoCoverage)`;
  `Some(WindowRef{coverage: Partial, ..}) → Err(PartialCoverage)`;
  `Some(WindowRef{min, coverage: Full}) → Ok(ComputedFloor { filed_basis: round_cents(min * Usd::from(sat) / Usd::from(SATS_PER_BTC)), coverage: Full })`.
  (This mirrors `overpayment_delta_one`'s scaling at `conservative.rs:309`.)
- [ ] **Step 4: Run — expect PASS.** Run: `cargo test -p btctax-core --test kat_promote 2>&1 | tail -20`; `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/conservative_promote.rs crates/btctax-core/src/lib.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): computed whole-tranche filed_basis + Coverage::Full hard-refuse (BG-D3)"
  ```

**Mutation to kill:** replacing `round_cents(min * sat / SATS_PER_BTC)` with a bare `min` reds
`filed_basis_is_whole_tranche_scaled` (the per-BTC-vs-whole-lot bug the parent P6 review flagged); accepting
`Partial` reds `partial_coverage_is_hard_refused`.

### Task 3: Pass-2 Op-construction rewrite INSIDE `resolve` (BG-D1) + by-construction KATs

**Files:**
- Modify: `crates/btctax-core/src/project/resolve.rs` (step-2 DeclareTranche `Eff`-admit branch, :1085-1114)
- Reference: `overpayment_delta_one` swap (`conservative.rs:303-309`) is the "how"; step-2 admit is the "where".
- Test: `crates/btctax-core/tests/kat_promote.rs`

**Interfaces:**
- Consumes: T2 `PromoteSet`/`PromoteEntry`; `Op::Acquire { usd_cost, basis_source, .. }`; `EventId::Decision`;
  `Resolution` (resolve.rs:201), `FoldCtx` (fold.rs:21).
- Produces:
  ```rust
  // resolve.rs — built once before the step-2 loop; pushes conflicts (Task 7 fills the liveness rules).
  //   target DeclareTranche EventId -> PromoteEntry{filed_basis, tranche_sat = target DeclareTranche.sat}
  fn live_promotes(events: &[LedgerEvent], voided: &BTreeSet<EventId>, blockers: &mut Vec<Blocker>) -> PromoteSet;
  // + a new field on Resolution so the fold can reach it (T4 threads it into FoldCtx):
  //   pub struct Resolution { …existing…, pub promotes: PromoteSet }
  ```

- [ ] **Step 1: Write the failing KATs** (mirror the kat_tranche.rs harness — `dec_ev`, `tranche_ev`, a new
  `promote_ev(seq, target, filed_basis)`, `project`, `prices`, `cfg`):
  ```rust
  #[test]
  fn promote_rewrites_usd_cost_but_keeps_the_tag() {
      let w = exch();
      let t = tranche_ev(1, &w, 100_000_000, d("2017-12-01"), d("2017-12-31"));
      let p = promote_ev(2, EventId::decision(1), dec!(12_000));
      let st = project(&[t, p], &prices(), &cfg());
      let lot = st.lots.iter().find(|l| l.wallet == w).unwrap();
      assert_eq!(lot.usd_basis, dec!(12_000), "usd_cost rewritten to the floor");
      assert_eq!(lot.basis_source, BasisSource::EstimatedConservative, "NO new BasisSource (BG-D1)");
      assert_eq!(lot.acquired_at, d("2017-12-31"), "term-invariance: acquired_at still window_end (BG-D9/M-7)");
  }
  #[test]
  fn promoted_pre2025_tranche_still_trips_the_d8_backstop() {
      // The backstop keys on the EstimatedConservative TAG, not $0 — so a promoted (>$0) tranche still
      // denies a SafeHarborAllocation effectiveness. This is the load-bearing by-construction guarantee.
      let w = exch();
      let t = tranche_ev(1, &w, 100_000_000, d("2018-01-01"), d("2018-12-31"));
      let p = promote_ev(2, EventId::decision(1), dec!(4_200));
      let alloc = safe_harbor_alloc_ev(3, /* pre2025 residue */ ..);
      let st = project(&[t, p, alloc], &prices(), &cfg());
      assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::SafeHarborUnconservable),
          "a promoted pre-2025 tranche still trips the D-8 backstop (tag-keyed, BG-D1)");
  }
  #[test]
  fn snapshot_timing_the_floor_is_visible_to_pass1_conservation() {
      // ★ BG-D1 / arch r1 M-3: the rewrite is INSIDE resolve step 2, so step-3 universal_snapshot sees the
      // floor. Construct a pre-2025 promoted tranche + a SafeHarborAllocation whose conservation outcome
      // DIFFERS between a floor-VISIBLE and a floor-BLIND snapshot (alloc_basis vs snap.basis, resolve.rs
      // :1305). A post-resolve rewrite (the overpayment_delta_one timing) would compute the WRONG snapshot.
      let st = project(&promoted_pre2025_tranche_plus_alloc_basis_sensitive(), &prices(), &cfg());
      assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::SafeHarborUnconservable),
          "conservation adjudicated against the FLOOR-visible residue (rewrite is in step 2, not post-resolve)");
  }
  #[test]
  fn relocated_promoted_tranche_keeps_tag_and_floor() {
      // §6: a promoted tranche self-transferred Exchange→SelfCustody keeps EstimatedConservative + the floor
      // (fold.rs:816-820 tag carry; origin_event_id preserved fold.rs:801-806).
      let st = project(&promote_then_self_transfer_to_selfcustody(dec!(12_000)), &prices(), &cfg());
      let lot = st.lots.iter().find(|l| matches!(l.wallet, WalletId::SelfCustody { .. })).unwrap();
      assert_eq!(lot.basis_source, BasisSource::EstimatedConservative);
      assert_eq!(lot.usd_basis, dec!(12_000), "the floor rides the relocation");
  }
  #[test]
  fn a_promoted_tranche_still_refuses_a_safe_harbor_allocation_at_record_time() {
      // §6: both record-time refusal directions still fire for a PROMOTED (>$0) tranche — the guards are
      // tag-keyed (cmd/tranche.rs:93-97 / session.rs:694), so a promote on file must not slip past them.
      let events = tranche_then_promote_events(dec!(12_000));
      assert!(cmd::tranche::guard_allocation_vs_tranche(&events).is_err(),
          "a promoted pre-2025 tranche still blocks a safe-harbor allocation (D-8, tag-keyed)");
  }
  ```
- [ ] **Step 2: Run — expect FAIL** (`promote_ev` helper + the rewrite don't exist).
  Run: `cargo test -p btctax-core --test kat_promote -- promote_ snapshot relocated refuses 2>&1 | tail -30`
- [ ] **Step 3: Implement the rewrite + the PromoteSet.** Before the step-2 loop, build
  `let promotes = live_promotes(events, &voided, &mut blockers);` (iterate non-voided `PromoteTranche` events
  whose `target` is a present, non-voided `DeclareTranche`; map `target -> PromoteEntry{ filed_basis,
  tranche_sat: target_declare.sat }`; Task 7 fills the double-promote/absent-target `DecisionConflict` pushes —
  here take the single live promote per target). Store `promotes` on `Resolution` (T4 threads it to the fold).
  In the DeclareTranche admit branch (resolve.rs:1085-1114), after `let op = build_op(...)`, rewrite:
  ```rust
  let op = match (op, promotes.get(&e.id)) {
      (Op::Acquire(mut a), Some(entry)) if a.basis_source == BasisSource::EstimatedConservative => {
          a.usd_cost = entry.filed_basis;      // BG-D1: rewrite ONLY usd_cost, inside resolve (step 2)
          Op::Acquire(a)                        // → visible to step-3 universal_snapshot + the void passes
      }
      (op, _) => op,
  };
  ```
  (Everything else — `acquired_at = window_end`, `basis_source`, the tag exemptions — is unchanged.) **Census
  item 11 (arch M-4):** add an explicit `EventPayload::PromoteTranche(_) => Op::Skip` arm + comment in
  `build_op` (:405-413) documenting that a promote never folds as its own `Op` (its effect is the target
  rewrite above) — the `_ => Op::Skip` catch-all handles it today, but the census wants it non-silent.
- [ ] **Step 4: Run — expect PASS.** `cargo test -p btctax-core --test kat_promote 2>&1 | tail -30`; then the
  full v1 tranche suite must stay green (no regression): `cargo test -p btctax-core --test kat_tranche 2>&1 | tail -20`; `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/project/resolve.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): rewrite Op::Acquire.usd_cost inside resolve step-2 + PromoteSet on Resolution; tag/term/backstop/snapshot hold by construction (BG-D1)"
  ```

**Mutation to kill:** moving the rewrite to AFTER `resolve()` returns (the `overpayment_delta_one` timing)
leaves `universal_snapshot` blind to the floor — `snapshot_timing_the_floor_is_visible_to_pass1_conservation`
reds. Changing `basis_source` reds `promote_rewrites_usd_cost_but_keeps_the_tag` AND the backstop KAT; dropping
the relocation tag-carry reds `relocated_promoted_tranche_keeps_tag_and_floor`.

### Task 4: BG-D4 disposal-leg loss clamp + stored-`filed_basis` decomposition

**Files:**
- Modify: `crates/btctax-core/src/project/fold.rs` (★ FIRST thread the `PromoteSet`: add `promotes: PromoteSet`
  to `FoldCtx` :21, populate it in `fold` :376 from `res.promotes` (T3), and pass `&ctx.promotes` to the six
  builder call sites :362/:635/:641/:832/:1118/:1122/:1195/:1199 — arch r1 I-1; then `make_disposal_legs`
  non-dual arm :192-202)
- Create helper in: `crates/btctax-core/src/conservative_promote.rs` (`clamped_leg_basis`)
- Modify: `crates/btctax-core/tests/kat_conservative.rs` (amend the parent Invariant KAT wording per BG-D4 —
  SPEC §3 item 6; the attribution sentence now reads "…never the estimate; a promoted-tranche leg's
  estimate-attributable gain is ≥ 0 and its estimate basis ≥ $0")
- Test: `crates/btctax-core/tests/kat_promote.rs`

**Interfaces:**
- Consumes: `PromoteSet`/`PromoteEntry` (T2), reachable via `leg.lot_id.origin_event_id`; `Resolution.promotes`
  (T3); `split_pro_rata`, `round_cents`, `Usd::ZERO`.
- Produces: `FoldCtx` gains `promotes: PromoteSet`; the three leg builders (`make_disposal_legs`,
  `make_removal_legs` T6, `consume_fee` T5) each gain a `promotes: &PromoteSet` param.
- Produces:
  ```rust
  // conservative_promote.rs — BG-D4. `usd_basis_share` = c.gain_basis (may include a TP8(c) fee carry).
  //   estimate_share = filed_basis × leg_sat / tranche_sat  (from the STORED promote, NOT usd_basis_share)
  //   documented_share = usd_basis_share − estimate_share   (UNCLAMPED)
  //   reported_basis   = documented_share + clamp(net_proceeds_share, $0, estimate_share)
  pub fn clamped_leg_basis(
      promote: Option<&PromoteEntry>, leg_sat: Sat, usd_basis_share: Usd, net_proceeds_share: Usd,
  ) -> Usd; // returns usd_basis_share unchanged when promote is None
  ```

- [ ] **Step 1: Write the failing KATs:**
  ```rust
  #[test]
  fn floor_below_window_low_files_zero_gain_not_a_loss() {
      // window-min $12k floor; sold below it. estimate gain clamped to 0; NO loss off the estimate.
      let st = project(&promote_then_sell(100_000_000, floor=12_000, proceeds=8_000), &prices(), &cfg());
      let leg = only_disposal_leg(&st);
      assert_eq!(leg.gain, Usd::ZERO, "estimate gain clamped ≥ 0 (BG-D4)");
      assert!(leg.gain >= Usd::ZERO && leg.basis == leg.proceeds, "basis = proceeds, no fabricated loss");
  }
  #[test]
  fn relocated_with_fee_then_promoted_keeps_documented_fee_unclamped() {
      // The documented fee carry (TP8(c), merged into usd_basis) stays UNCLAMPED; the estimate share is
      // decomposed from the STORED filed_basis, not lot.usd_basis (arch r1 I-2 / tax r1 I-1).
      let st = project(&promote_relocate_fee_then_sell_below_floor(), &prices(), &cfg());
      let leg = only_disposal_leg(&st);
      assert!(leg.gain < Usd::ZERO, "the documented fee corner still reaches negative (attribution intact)");
  }
  #[test]
  fn estimate_basis_never_goes_negative_when_fee_exceeds_proceeds() {
      // fee_usd > proceeds → net < 0 → clamp(net, $0, est) = $0, not a negative basis (arch r1 I-1 / M-2).
      let st = project(&promote_then_dispose_fee_gt_proceeds(), &prices(), &cfg());
      let leg = only_disposal_leg(&st);
      assert!(leg.basis >= Usd::ZERO, "estimate basis ≥ $0");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- clamp floor relocated estimate_basis 2>&1 | tail -30`
- [ ] **Step 3: Implement.** First do the `FoldCtx`/`Resolution` threading (Files above), so `&ctx.promotes`
  reaches every builder. In `make_disposal_legs` (add `promotes: &PromoteSet`), non-dual arm (fold.rs:192-202),
  replace `basis = c.gain_basis` with
  `basis = clamped_leg_basis(promotes.get(&c.lot_id.origin_event_id), c.sat, c.gain_basis, net_share)`
  where `net_share` is this leg's pro-rata share of `net` (the cent-remainder-takes-rest split already present
  at :133-140). `clamped_leg_basis`: if `promote` is `None`, return `usd_basis_share`; else compute
  `estimate_share = round_cents(filed_basis * Usd::from(leg_sat) / Usd::from(tranche_sat))`,
  `documented_share = usd_basis_share − estimate_share`,
  return `documented_share + min(estimate_share, max(net_proceeds_share, Usd::ZERO))`. `gain = round_cents(proceeds − basis)`
  (unchanged formula). The §1015 NoGainNoLoss precedent (fold.rs:181-190, reported≠consumed) makes this legal;
  a tranche lot never enters the dual arm (`rehome_onto_lot` never promotes `dual_loss_basis` None→Some), so
  the non-dual-arm placement is complete.
- [ ] **Step 4: Run — expect PASS** + no regression: `cargo test -p btctax-core --test kat_promote 2>&1 | tail -30`;
  amend + re-green the parent Invariant KAT (`kat_conservative.rs`, SPEC §3 item 6); `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/project/fold.rs crates/btctax-core/src/conservative_promote.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): BG-D4 disposal-leg clamp decomposed from stored filed_basis (never a loss, never negative)"
  ```

**Mutation to kill:** decomposing from `c.gain_basis` instead of the stored `filed_basis` reds
`relocated_with_fee_then_promoted_keeps_documented_fee_unclamped`; a bare `min(est, net)` (no `max(net,0)`)
reds `estimate_basis_never_goes_negative_when_fee_exceeds_proceeds`; clamping to `net` unconditionally (not
just the estimate share) reds the documented-fee-negative KAT.

### Task 5: BG-D4 fee-draw evaporation at `consume_fee` / `FeeCarry`

**Files:**
- Modify: `crates/btctax-core/src/project/fold.rs` (`consume_fee` TreatmentC summation, :348-357; the `FeeCarry` it returns)
- Test: `crates/btctax-core/tests/kat_promote.rs`

**Interfaces:**
- Consumes: `PromoteSet`; `Consumed { lot_id, sat, gain_basis, .. }` (pools.rs:290-307);
  `FeeCarry { gain_basis, loss_basis }` (fold.rs:273-277).
- Produces: no new public API — `consume_fee` gains a `promotes: &PromoteSet` param and withholds the estimate
  component of each promoted fee fragment from the summed `FeeCarry.gain_basis`.

- [ ] **Step 1: Write the failing KAT** (the worked corner from the review):
  ```rust
  #[test]
  fn tranche_fee_draw_evaporates_estimate_then_sale_files_zero_loss() {
      // Promote 1 BTC to $12k; self-transfer paying a 10,000-sat fee drawn FIFO from the tranche
      // (the oldest lot); later sell below the window low. The $1.20 of FLOOR fee basis must EVAPORATE —
      // NOT re-home onto the surviving lot — so the sale files $0 estimate loss, not a loss that is 100%
      // estimate money (tax r2 I-2).
      let st = project(&promote_then_self_transfer_fee_then_sell_below_floor(), &prices(), &cfg());
      let leg = only_disposal_leg(&st);
      assert!(leg.gain >= Usd::ZERO, "the burned fee-sats' estimate component evaporated, not a filed loss");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- fee_draw_evaporates 2>&1 | tail -20`
- [ ] **Step 3: Implement.** In `consume_fee` TreatmentC (fold.rs:348-357), when summing `gain_basis` across
  the FIFO `Consumed` fee fragments, for a fragment whose `lot_id.origin_event_id ∈ promotes`, subtract its
  estimate component (`round_cents(filed_basis * frag.sat / tranche_sat)`) before adding to the `FeeCarry` —
  i.e. only the documented fee basis re-homes; the estimate evaporates (BG-D4's evaporation rule — basis
  forfeiture is always conservative). TreatmentB needs no change (its mini-disposition routes through
  `make_disposal_legs`, so Task 4's clamp applies by construction).
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/project/fold.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): BG-D4 estimate evaporates on the FIFO fee draw; only documented fee re-homes"
  ```

**Mutation to kill:** re-homing the raw `gain_basis` (no estimate subtraction) reds
`tranche_fee_draw_evaporates_estimate_then_sale_files_zero_loss` (the $1.20 estimate loss reappears).

### Task 6: BG-D11 removal-leg-builder documented-only decomposition

**Files:**
- Modify: `crates/btctax-core/src/project/fold.rs` (`make_removal_legs` basis, :256)
- Verify-only (KAT, NOT patched): `forms.rs:154/213/439`, `tax/return_1040.rs:535`, `tax/printed.rs:82/155/157`
- Test: `crates/btctax-core/tests/kat_promote.rs` (fold + the computed 1040 / 8283 surfaces)

**Interfaces:**
- Consumes: `PromoteSet`; `RemovalLeg { basis, lot_id, .. }`; `crypto_charitable_gifts`, `apply_170b` (read-only assertions).
- Produces: `make_removal_legs` gains `promotes: &PromoteSet`; a promoted-lot removal leg carries the
  **documented component only** (`documented_share = c.gain_basis − estimate_share`; estimate EVAPORATES on a
  removal — a removal recognizes no gain, so there is nothing to clamp into).

- [ ] **Step 1: Write the failing KATs — assert BOTH §170(e) emitters + the 8283 column:**
  ```rust
  #[test]
  fn promoted_tranche_donated_short_term_deducts_documented_only_on_both_emitters() {
      // BG-D11: an ST donation of a promoted tranche files a $0/documented §170(e)(1)(A) deduction (NOT the
      // floor) on the FOLD (claimed_deduction) AND the full-return Schedule A (crypto_charitable_gifts).
      let events = promote_then_donate_short_term(1_00000000, floor=60_000);
      let st = project(&events, &prices(), &cfg());
      let rem = st.removals.iter().find(|r| r.kind == RemovalKind::Donation).unwrap();
      assert_eq!(rem.claimed_deduction, Some(Usd::ZERO), "fold claimed_deduction documented-only");
      // full-return second emitter:
      let sched_a_ded = crypto_charitable_gifts_deduction(&st, YEAR); // helper → apply_170b Schedule A line 12
      assert_eq!(sched_a_ded, Usd::ZERO, "the full-return engine also deducts documented-only");
      // Form 8283 basis column (via forms.rs → printed.rs) prints the documented component:
      assert_eq!(form_8283_cost_basis(&st, YEAR), Usd::ZERO);
  }
  #[test]
  fn promoted_tranche_gifted_carries_documented_only_1015_basis() {
      let st = project(&promote_then_gift(1_00000000, floor=12_000), &prices(), &cfg());
      let rem = st.removals.iter().find(|r| r.kind == RemovalKind::Gift).unwrap();
      assert!(rem.legs.iter().all(|l| l.basis == Usd::ZERO), "§1015 carryover documented-only (BG-D11)");
  }
  #[test]
  fn long_term_donation_deduction_is_fmv_and_8283_column_is_documented_only() {
      let st = project(&promote_then_donate_long_term(1_00000000, floor=12_000, fmv=50_000), &prices(), &cfg());
      let rem = st.removals.iter().find(|r| r.kind == RemovalKind::Donation).unwrap();
      assert_eq!(rem.claimed_deduction, Some(dec!(50_000)), "LT deduction = FMV, basis uninvolved");
      // §6 (tax r4 M-3): the 8283 cost_basis COLUMN is documented-only for ST AND LT (an LT donation still
      // prints the basis column, and it must not print the floor).
      assert_eq!(form_8283_cost_basis(&st, YEAR), Usd::ZERO, "8283 basis column documented-only, LT too");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- donated gifted long_term 2>&1 | tail -30`
- [ ] **Step 3: Implement.** In `make_removal_legs` (fold.rs:256), replace `basis: c.gain_basis` with
  `basis: documented_share_for(promotes.get(c.lot_id.origin_event_id), c.sat, c.gain_basis)` where
  `documented_share_for` returns `c.gain_basis` when promote is `None`, else `c.gain_basis − round_cents(filed_basis * c.sat / tranche_sat)`
  (estimate evaporates; the pool-side Σbasis is conserved by `take_from`'s debit, the §1015 NoGainNoLoss
  precedent). All six downstream `leg.basis` consumers (fold `claimed_deduction` :1237, `crypto_charitable_gifts`
  :535, `forms.rs` 8949/SchD/8283, `printed.rs`) inherit — verified by the KATs above, NOT independently patched.
- [ ] **Step 4: Run — expect PASS** + `make check` (the forms/return_1040/printed tests stay green — they now
  read the documented component).
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/project/fold.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): BG-D11 removal-leg documented-only basis — estimate never funds a deduction/carry (both §170(e) emitters)"
  ```

**Mutation to kill:** leaving `basis: c.gain_basis` reds all three KATs on the §170(e) surface; patching only
the fold's `claimed_deduction` (not the builder) would leave `crypto_charitable_gifts_deduction` non-zero —
the second-emitter KAT catches exactly that (the tax r2 I-1 harm).

### Task 7: BG-D9 engine-adjudicated lifecycle (deferred void + DecisionConflict)

**Files:**
- Modify: `crates/btctax-core/src/project/resolve.rs` (pass-1a collect :453-496; adjudicate the deferred
  tranche-voids **immediately after the pass-1a loop, BEFORE step 2** — arch r1 I-2; `live_promotes` conflict
  pushes from T3)
- Modify: `crates/btctax-core/src/void.rs` (`is_revocable_payload` :20-35; a `promoted_target` exclusion
  closure in `voidable_decisions`, mirroring `effective_alloc` :72-81)
- Reference: `would_conflict` (`project/mod.rs:107-155`) surfaces any `DecisionConflict` at record time free.
- Test: `crates/btctax-core/tests/kat_promote.rs`

**Interfaces:**
- Produces: `PromoteTranche` in `is_revocable_payload`; `voidable_decisions` excludes a `DeclareTranche` that
  carries a live promote; resolve pushes `BlockerKind::DecisionConflict` for: a second live promote on one
  target (both inert), a void of a tranche with a live promote (the void inert), a promote with an
  absent/wrong-type target (non-voided only, arch r3 N-1).

- [ ] **Step 1: Write the failing KATs:**
  ```rust
  #[test]
  fn second_promote_on_one_target_conflicts_neither_applies() {
      let st = project(&[tranche(1), promote(2, tgt=1, 12_000), promote(3, tgt=1, 20_000)], &prices(), &cfg());
      assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::DecisionConflict));
      assert_eq!(lot_basis(&st), Usd::ZERO, "neither promote applies under conflict (not last-wins)");
  }
  #[test]
  fn void_of_tranche_with_live_promote_is_inert_and_conflicts() {
      // A RAW void of the DeclareTranche while a promote is live → resolver-inert + DecisionConflict
      // (never a dangling target). Deferred adjudication against the FINAL non-voided-promote set.
      let st = project(&[tranche(1), promote(2, tgt=1, 12_000), void(3, tgt=1)], &prices(), &cfg());
      assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::DecisionConflict));
      assert_eq!(lot_basis(&st), dec!(12_000), "the tranche-void is inert; the promote still applies");
  }
  #[test]
  fn both_voids_either_order_converge_no_brick() {
      for order in [ [void_t(3), void_p(4)], [void_p(3), void_t(4)] ] { /* build events */
          let st = project(&events_with(order), &prices(), &cfg());
          assert!(st.lots.iter().all(|l| l.wallet != exch()), "promote dead + tranche voided");
          assert!(!st.blockers.iter().any(|b| b.kind == BlockerKind::DecisionConflict), "no spurious Hard (arch r3 N-1)");
      }
  }
  #[test]
  fn a_promoted_tranche_target_is_not_bulk_voidable() {
      let events = vec![tranche_ev(7, ..), promote_ev(8, tgt=7, ..)];
      let voidable = voidable_decisions(&events, &[]);
      assert!(!voidable.iter().any(|e| e.id == EventId::decision(7)), "the tranche target is excluded while a promote is live");
      assert!(voidable.iter().any(|e| e.id == EventId::decision(8)), "but the promote itself is voidable");
  }
  #[test]
  fn void_of_promote_alone_reverts_to_zero_tag_intact() {
      // §6 plain void → reverts to $0 (the DeclareTranche is intact). (Distinct from the both-voids end state.)
      let st = project(&[tranche(1), promote(2, tgt=1, 12_000), void(3, tgt=2)], &prices(), &cfg());
      let lot = only_lot(&st);
      assert_eq!(lot.usd_basis, Usd::ZERO, "voiding the promote reverts the tranche to $0");
      assert_eq!(lot.basis_source, BasisSource::EstimatedConservative, "the intact tranche keeps its tag");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- conflict void both_voids voidable revert 2>&1 | tail -30`
- [ ] **Step 3: Implement.** (a) Add `PromoteTranche(_)` to `is_revocable_payload` (void.rs:20-35). (b) In
  pass-1a, a void whose target is a `PromoteTranche` applies **inline+unconditionally** (`voided.insert` — a
  promote-void is always allowed); a void whose target is a `DeclareTranche` that has **ANY promote event in
  the ledger** (never evaluate "live" inline — arch r2 M-1) is collected into a `tranche_voids` deferred list,
  NOT inserted. (c) `live_promotes` (T3): a target with ≥2 non-voided promotes → push `DecisionConflict`, apply
  none; a non-voided promote whose target is absent/wrong-type → `DecisionConflict`. (d) **Immediately after
  the pass-1a loop and BEFORE step 2 (arch r1 I-2 — NOT step 3, which runs after the timeline is built and
  after `universal_snapshot`, where a `voided.insert` is a no-op):** adjudicate `tranche_voids` against the
  FINAL non-voided-promote set — target still has a live promote → the void is inert + `DecisionConflict`; else
  `voided.insert(target)` so the step-2 admit branch (`if voided.contains(&e.id)`) drops the tranche. This is
  deferred+order-independent (promote-liveness depends only on promote-targeted voids, all applied inline in
  (b)) — the settled arch r2 M-1 ruling, at the correct insertion point. (e) In `voidable_decisions`, add a
  `promoted_target` closure (mirror `effective_alloc` :72-81) excluding a `DeclareTranche` whose id has any
  live-promote targeting it.
- [ ] **Step 4: Run — expect PASS** + `make check`. Verify `would_conflict` surfaces the conflict at record
  time (a CLI-level test in T10 depends on this).
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/project/resolve.rs crates/btctax-core/src/void.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): BG-D9 engine-adjudicated lifecycle — deferred void + DecisionConflict + voidable exclusion"
  ```

**Mutation to kill:** classifying the tranche-void inline in pass-1a (not deferred) reds
`both_voids_either_order_converge_no_brick`; omitting the `is_revocable_payload` arm reds
`a_promoted_tranche_target_is_not_bulk_voidable`'s promote-voidable half; last-wins (apply the newest promote)
reds `second_promote_on_one_target_conflicts_neither_applies`.

### Task 8: BG-D9 prior-year fold-diff advisory (disposal ∪ removal legs) + carryover-cascade naming

**Files:**
- Modify: `crates/btctax-core/src/conservative.rs` (new `promote_prior_year_advisory` fn, sibling to the P6 nudge)
- Modify: `crates/btctax-cli/src/cmd/reconcile.rs` (the void verb) + the bulk-void path — **WIRE
  `Direction::Void`** (arch/tax r1 I-3): when a `PromoteTranche` OR a promoted-tranche `DeclareTranche` target
  is voided, print the advisory lines before recording (the amend-to-pay warning). The `Direction::Promote`
  call site is T10 (the consent screen).
- Reference: fold-pair precedent `overpayment_delta_one` (`conservative.rs:298-317`, clone-resolve→fold);
  `carryforward_consistency` (`tax/compute.rs:436-448`), `charitable_carryover_out` (`return_1040.rs:1311`),
  `capital_loss_carryforward_in` (`compute.rs:317`). (This TASK owns the SPEC file-map's `compute.rs`/`cmd/tax.rs`
  cascade-naming reads; T8 quotes their diffs, it does not make their existing copy promote-aware — that is a
  no-change decision, recorded here.)
- Test: `crates/btctax-core/tests/kat_promote.rs` + a CLI void-direction test in `crates/btctax-cli/tests/promote_cli.rs`

**Interfaces:**
- Consumes: the pre-promote `LedgerState` (baseline `project`) and the post-promote `LedgerState`;
  `Disposal { disposed_at, legs }`, `Removal { removed_at, legs }` (both derive `PartialEq, Eq`).
- Produces:
  ```rust
  // conservative.rs — fires on the leg-SET diff (disposals ∪ removals), NOT tax_total, NOT Σ-gain.
  pub fn promote_prior_year_advisory(events: &[LedgerEvent], prices: &dyn PriceProvider,
      config: &ProjectionConfig, promote_id: &EventId, direction: Direction /* Promote|Void */,
      profile: Option<&TaxProfile>, tables: &dyn TaxTables) -> Vec<String>;
  ```

- [ ] **Step 1: Write the failing KATs** (the review's worked corners):
  ```rust
  #[test]
  fn undisposed_promote_that_hifo_reorders_a_prior_year_fires_the_advisory() {
      // Table-less/profile-less 2018 year (only 2017/2024/2025/2026 tables ship): the fold-diff STILL fires
      // (leg-set diff, profile/table-independent) — the tax_total-keyed r1 predicate could not (tax r2 I-3).
      let lines = promote_prior_year_advisory(&mixed_vintage_hifo_2018_disposal(), .., Direction::Promote, None, ..);
      assert!(lines.iter().any(|l| l.contains("2018") && l.contains("1040-X")));
  }
  #[test]
  fn promote_reordering_a_prior_DONATION_only_year_fires_and_names_the_deduction() {
      // No disposal-leg change; a removal-leg (donation) reorder — caught by the disposal∪removal diff (tax r3 I-2).
      let lines = promote_prior_year_advisory(&prior_donation_only_reorder(), .., Direction::Promote, ..);
      assert!(lines.iter().any(|l| l.contains("charitable deduction")));
  }
  #[test]
  fn a_loss_stealing_reorder_names_the_1212b_carryover_cascade() {
      // Absorbing a prior year's capital loss into Y strands Y+1's filed carryforward — NAME it (tax r4 I-1).
      let lines = promote_prior_year_advisory(&loss_stealing_reorder(), .., Direction::Promote, ..);
      assert!(lines.iter().any(|l| l.contains("carryover-linked lines of later filed years")
                                && l.contains("§1212(b)")));
  }
  #[test]
  fn a_gift_only_reorder_quotes_the_1015_carryover_and_asserts_NO_1040X() {
      // tax r4 M-1: a gift changes no line of the donor's 1040 → NO 1040-X assertion (the false-amend bug).
      let lines = promote_prior_year_advisory(&prior_gift_only_reorder(), .., Direction::Promote, ..);
      let joined = lines.join(" ");
      assert!(joined.contains("donee-basis") && !joined.contains("$0 / $0"));
      assert!(!joined.contains("1040-X"), "a gift reorder must NOT tell the donor to amend");
  }
  #[test]
  fn a_both_deltas_zero_flagged_year_names_the_changed_content_not_a_bare_zero() {
      // BG-D9: an equal-basis-swap reorder (Δgain=Δded=$0) that changes 8949 dates → name the content, no "$0".
      let lines = promote_prior_year_advisory(&equal_basis_date_swap_reorder(), .., Direction::Promote, ..);
      assert!(lines.iter().any(|l| l.contains("acquisition date") || l.contains("donee")));
      assert!(!lines.iter().any(|l| l.trim().ends_with("$0")));
  }
  #[test]
  fn a_donation_reorder_names_the_170d_charitable_carryover_direction() {
      let lines = promote_prior_year_advisory(&prior_donation_reorder_over_ceiling(), .., Direction::Promote, ..);
      assert!(lines.iter().any(|l| l.contains("§170(d)") && l.contains("charitable carryover")));
  }
  #[test]
  fn the_void_direction_fires_amend_to_pay() {
      // §6: the SAME advisory in the VOID direction (voiding a promote over a filed floor-year → amend-to-PAY).
      let lines = promote_prior_year_advisory(&void_promote_over_filed_year(), .., Direction::Void, ..);
      assert!(lines.iter().any(|l| l.contains("1040-X") && l.to_lowercase().contains("additional tax")));
  }
  ```
  Plus a CLI test in `promote_cli.rs`: voiding a promoted tranche's promote PRINTS the `Direction::Void` lines.
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- advisory reorder cascade gift_only both_deltas 170d void_direction 2>&1 | tail -30`
- [ ] **Step 3: Implement the advisory.** Fold pair: `with = project(events)` (the promote applies post-T3);
  `baseline = project(events_without_the_promote_event)` — **exclude the `PromoteTranche` EVENT itself, NOT its
  `DeclareTranche` target** (excluding the target deletes the lot and diffs every tranche-touching year — tax
  r1 M-1). For each `year < current`, diff the per-year `disposals`+`removals` leg sets (group by
  `disposed_at.year()`/`removed_at.year()`; `PartialEq` set compare). On a diff, emit: `"changes year Y's
  reported gain by ~$G [and its charitable deduction by ~$D] [and computed tax by ~$Δ, when Y computes]; if Y
  was already filed, claiming it requires a Form 1040-X for Y with the 8275 attached"` — the `~$D` clause for a
  DONATION reorder = `Σ claimed_deduction` diff (with the 1040-X clause); for a GIFT reorder = the `Σ leg.basis`
  §1015 carryover-Δ with "donee-basis documentation changes; the donor's 1040 is unaffected" and **NO 1040-X**;
  a both-Δs-zero flagged year names the changed 8283 dates/donee records, **never a bare `$0`**; the `~$Δ`
  clause only when `compute_tax_year(Y)` computes both folds, else `"tax not computable for Y (no table/
  profile/blocked)"`; note §6511. When Y's net capital gain/loss OR charitable deduction changed, append the
  cascade clause naming §1212(b) (Schedule D carryforward) AND §170(d) (Schedule A charitable carryover) —
  quoting the `carryforward_out` / `charitable_carryover_out` diff only when computable, else named-unquantified.
  `Direction::Void` = the same diff over the promote-removed vs promote-present pair, amend-to-**pay** (promote
  direction) / amend-to-refund (void) copy.
- [ ] **Step 3b: Wire `Direction::Void`.** In the void verb (`cmd/reconcile.rs`) + the bulk-void path, when the
  void target is a `PromoteTranche` or a promoted-tranche `DeclareTranche`, compute + print
  `promote_prior_year_advisory(.., Direction::Void, ..)` before recording (a warning, non-gating).
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/conservative.rs crates/btctax-cli/src/cmd/reconcile.rs crates/btctax-core/tests/kat_promote.rs crates/btctax-cli/tests/promote_cli.rs
  git commit -m "feat(promote): BG-D9 prior-year fold-diff advisory (disposal∪removal) + cascade naming + VOID-direction wiring"
  ```

**Mutation to kill:** keying the diff on `tax_total` reds `undisposed_promote_that_hifo_reorders_a_prior_year_fires_the_advisory`
(the table-less 2018 year returns `None==None`); diffing disposals only reds
`promote_reordering_a_prior_DONATION_only_year_fires_and_names_the_deduction`; dropping the cascade clause reds
`a_loss_stealing_reorder_names_the_1212b_carryover_cascade`; appending "1040-X" to a gift year reds
`a_gift_only_reorder_quotes_the_1015_carryover_and_asserts_NO_1040X`; leaving the void path unwired reds the
CLI void-direction test.

### Task 9: BG-D6 consent quantification (clamped, current-year Σ, gift/donation, uncomputable, cascade)

**Files:**
- Modify: `crates/btctax-core/src/conservative_promote.rs` (new `consent_terms` fn)
- Reference: `overpayment_delta_one` (`conservative.rs:283-322`) — reuse the swap, but thread a **synthetic
  promote set so the BG-D4 clamp binds** (tax r1 I-3), and range the year set over the fold-diff INCLUDING the
  current year (tax r3 I-1).
- Test: `crates/btctax-core/tests/kat_promote.rs`

**Interfaces:**
- Produces:
  ```rust
  // Returns the exact terms the consent screen shows AND records in Acknowledgment.shown_terms (T1 ConsentTerm).
  pub fn consent_terms(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig,
      tranche_id: &EventId, filed_basis: Usd, profile: Option<&TaxProfile>, tables: &dyn TaxTables)
      -> Vec<ConsentTerm>;
  ```

- [ ] **Step 1: Write the failing KATs:**
  ```rust
  #[test]
  fn below_window_low_sale_quotes_the_CLAMPED_saving_not_an_unclaimable_loss() {
      // window-min $12k, sold at $8k. True promoted saving = tax on $8k gain (clamped), NOT a $4k loss.
      let terms = consent_terms(&promote_sell_below_low(floor=12_000, proceeds=8_000), .., Some(&profile), ..);
      // the recorded ComputedTax delta must equal the clamped saving, never include the forbidden loss.
      assert_eq!(clamped_saving(&terms), tax_on_gain(8_000));
  }
  #[test]
  fn fully_undisposed_promote_records_an_unrealized_term_not_empty() {
      // A fully-undisposed promote flags NO year (no filed content changes) → the Σ is empty; BG-D6 mandates
      // an UNREALIZED line (never a bare nothing — tax r1 I-2 / plan-r1 I-5).
      let terms = consent_terms(&promote_undisposed_2017_window(), .., None, ..); // no profile
      assert!(terms.iter().any(|t| matches!(t, ConsentTerm::Unrealized { .. })), "unrealized hypothetical line present");
      assert!(terms.iter().all(|t| !matches!(t, ConsentTerm::ComputedTax { delta_usd, .. } if *delta_usd == Usd::ZERO)),
          "never a bare $0 (tax r2 I-3 / r3 I-1)");
  }
  #[test]
  fn no_current_price_falls_back_to_the_floor_as_max_reduction() {
      // tax r3 N-2: bundled prices end at release; "today" often has no close → fallback, never a dropped line.
      let terms = consent_terms(&promote_undisposed_no_current_price(), .., None, ..);
      assert!(terms.iter().any(|t| matches!(t, ConsentTerm::Unrealized { hypothetical_reduction: None, .. })),
          "no-price → the floor itself ($filed_basis) named as the max reduction, not $0");
  }
  #[test]
  fn a_computing_removal_flagged_year_carries_the_deduction_delta() {
      // 2024 (table ships) + profile + a donation reorder → ComputedTax with Some(deduction_delta), NOT
      // labeled uncomputable and NOT dropping the Schedule-A change (engine B can't price it — tax r3 I-2 / plan I-4).
      let terms = consent_terms(&promote_reorders_2024_donation_with_profile(), .., Some(&profile), ..);
      assert!(terms.iter().any(|t| matches!(t, ConsentTerm::ComputedTax { deduction_delta_usd: Some(d), .. } if *d != Usd::ZERO)));
  }
  #[test]
  fn sell_this_year_then_promote_includes_the_current_year_term() {
      let terms = consent_terms(&sell_march_promote_july_same_year(), .., Some(&profile), ..);
      assert!(terms.iter().any(|t| matches!(t, ConsentTerm::ComputedTax { year, .. } if *year == CURRENT_YEAR)),
          "the current-year realized delta is quoted, not dropped (tax r3 I-1)");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- consent below_window undisposed no_current_price computing_removal sell_this_year 2>&1 | tail -30`
- [ ] **Step 3: Implement.** Build the fold pair (baseline = promote-event-excluded; with = the promote
  applied via the T3 rewrite path so the BG-D4 clamp binds). Range over every year the fold-diff flags
  INCLUDING the current year (the T8 diff without the `< current` filter). Per flagged year: if
  `compute_tax_year` computes both folds → `ConsentTerm::ComputedTax { year, delta_usd, deduction_delta_usd }`
  where `deduction_delta_usd = Some(Σ claimed_deduction / Σ gift leg.basis diff)` when a removal leg diffed
  (the tax-Δ EXCLUDES it — tax r3 I-2), else `None`; else → `ConsentTerm::Uncomputable { year, gain_delta_usd,
  deduction_delta_usd }` (both profile-free from the fold pair). **For sats NOT disposed in any flagged year**
  (a fully- or partly-undisposed promote), emit `ConsentTerm::Unrealized { sat, hypothetical_reduction, as_of }`:
  `hypothetical_reduction = Some(the today-price clamped gain reduction)` when a current close exists, else
  `None` (the render says "no current price data — the floor itself, $filed_basis, is the maximum gain
  reduction" — tax r3 N-2). When a later year's carryover-in derives from a flagged year and cannot be priced →
  `ConsentTerm::CascadeNamed { year }`. NEVER emit `ComputedTax{delta:0}` for a real change; NEVER return an
  empty vec for a promote with latent exposure.
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/conservative_promote.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): BG-D6 consent terms — clamped, current-year-inclusive, gift/donation, uncomputable-loud, cascade-named"
  ```

**Mutation to kill:** using the un-clamped `overpayment_delta_one` directly reds
`below_window_low_sale_quotes_the_CLAMPED_saving`; emitting `ComputedTax{delta:0}` for an undisposed/table-less
year reds `undisposed_promote_records_no_bare_zero`; keeping the `< current` filter reds
`sell_this_year_then_promote_includes_the_current_year_term`.

### Task 10: BG-D5 provenance + BG-D6 consent recording + the `promote-tranche` CLI verb

**Files:**
- Create: `crates/btctax-cli/src/cmd/promote.rs` (mirror `cmd/tranche.rs::declare_tranche` :123-172)
- Modify: `crates/btctax-cli/src/cli.rs:882-900` (add the `Reconcile::PromoteTranche` clap variant),
  `crates/btctax-cli/src/main.rs:1162-1187` (dispatch arm), `crates/btctax-cli/src/lib.rs` (re-export)
- Reference: `append_decision` (`persistence.rs:238-262`); `would_conflict` (`project/mod.rs`) to pre-check;
  `require_attestation`/`ATTEST_PHRASE` (`lib.rs:197/208`) as the phrase-gate PRECEDENT (a NEW distinct const
  `PROMOTE_ACK_PHRASE = "I understand and accept this estimated-basis risk"` — NOT the pseudo-attest phrase; N-1);
  T8 `promote_prior_year_advisory` (`Direction::Promote`).
- Test: `crates/btctax-cli/tests/promote_cli.rs` (mirror `declare_tranche_cli.rs`)

**Interfaces:**
- Consumes: T2 `filed_basis_for`, T9 `consent_terms`, T1 payload types, T7 conflict (`would_conflict`),
  T8 `promote_prior_year_advisory`.
- Produces: `pub fn promote_tranche(vault, pp, target_ref, provenance: ProvenanceKind, part_ii: String,
  acknowledge: Option<&str>, now) -> Result<EventId, CliError>`; clap `PromoteTranche { target, provenance,
  part_ii_file, i_acknowledge: Option<String> }`; `const PROMOTE_ACK_PHRASE`.

- [ ] **Step 1: Write the failing CLI tests** (mirror the `declare_tranche_cli.rs` harness — `pp()`, `now()`,
  vault builders, `count()`):
  ```rust
  #[test]
  fn every_non_purchase_provenance_is_refused_fail_closed() {
      // §6 / tax r1 M-6: refuse Gift/Inheritance/Mining/Earned/Airdrop/Fork — not just Gift.
      let v = vault_with_tranche(); // a declared $0 tranche
      for pk in [ProvenanceKind::Gift, ProvenanceKind::Inheritance, ProvenanceKind::Mining,
                 ProvenanceKind::Earned, ProvenanceKind::Airdrop, ProvenanceKind::Fork] {
          let err = cmd::promote::promote_tranche(&v, &pp(), "d1", pk, "facts".into(), None, now()).unwrap_err();
          assert!(matches!(err, CliError::Usage(m) if m.contains("purchase") && m.contains("real acquisition")));
      }
      assert_eq!(count(&v, |p| matches!(p, EventPayload::PromoteTranche(_))), 0, "fail-closed: nothing recorded (BG-D5)");
  }
  #[test]
  fn the_consent_copy_names_the_underpayment_base_and_never_says_safe_harbor() {
      // §6 Copy bullet covers the CONSENT copy too (not just the 8275, T13).
      let screen = cmd::promote::render_consent(&consent_terms_fixture());
      assert!(!screen.to_lowercase().contains("safe harbor"));
      assert!(screen.contains("of the resulting additional tax") && screen.contains("plus interest")); // N-2
  }
  #[test]
  fn empty_part_ii_narrative_is_refused_at_record_time() {
      let v = vault_with_tranche();
      let err = cmd::promote::promote_tranche(&v, &pp(), "d1", ProvenanceKind::Purchase, "  ".into(), Some(ATTEST_PHRASE), now()).unwrap_err();
      assert!(matches!(err, CliError::Usage(m) if m.contains("Part II")));  // BG-D7 present-by-construction
  }
  #[test]
  fn a_recorded_promote_carries_the_acknowledgment_and_stored_floor() {
      let v = vault_with_tranche();
      cmd::promote::promote_tranche(&v, &pp(), "d1", ProvenanceKind::Purchase, "cash P2P, no records".into(),
          Some(ATTEST_PHRASE), now()).unwrap();
      let p = only_promote(&v);
      assert!(p.filed_basis > Usd::ZERO && !p.acknowledgment.phrase.is_empty() && p.provenance_attested);
  }
  #[test]
  fn a_second_promote_is_refused_by_would_conflict() {
      let v = vault_with_promoted_tranche();
      let err = cmd::promote::promote_tranche(&v, &pp(), "d1", ProvenanceKind::Purchase, "x".into(), Some(ATTEST_PHRASE), now()).unwrap_err();
      assert!(matches!(err, CliError::Usage(m) if m.contains("conflict")));  // BG-D9 via would_conflict
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-cli --test promote_cli 2>&1 | tail -30`
- [ ] **Step 3: Implement** `promote_tranche`: open `Session`/`load_all`; resolve `target_ref`→`EventId` +
  assert it is a live `DeclareTranche`; **BG-D5** refuse unless `provenance == Purchase` (copy: the closed
  enumeration + "…model the real acquisition"); **BG-D7** refuse an empty/whitespace/scaffold-only `part_ii`;
  compute `filed_basis_for` (BG-D3 hard-refuse on Partial/None); compute `consent_terms` (T9); **compute +
  print `promote_prior_year_advisory(.., Direction::Promote, ..)` (T8 — the 1040-X/§6511/cascade lines) BEFORE
  the consent prompt (arch/tax r1 I-3 — this is the ONLY promote-direction call site of the advisory)**; render
  the consent screen (`render_consent` — BG-D6/D10 copy: penalty base = "of the resulting additional tax",
  "plus interest", NEVER "safe harbor", the wide-window "this floor is trivial" note); require
  `--i-acknowledge <PROMOTE_ACK_PHRASE>` on the non-TTY path **with the computed figures still printed to
  stdout** (N-2); build `PromoteTranche{..}` with `Acknowledgment{ phrase: PROMOTE_ACK_PHRASE, shown_terms:
  consent_terms, provenance_text, provenance_version }`; **pre-check** `would_conflict` (BG-D9 second-promote) →
  refuse; `append_decision`; `save`. Wire the clap variant + dispatch (mirror `DeclareTranche` at cli.rs:882 /
  main.rs:1162).
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-cli/src/cmd/promote.rs crates/btctax-cli/src/cli.rs crates/btctax-cli/src/main.rs crates/btctax-cli/src/lib.rs crates/btctax-cli/tests/promote_cli.rs
  git commit -m "feat(promote): promote-tranche CLI verb + BG-D5 provenance + BG-D6 consent recording + BG-D7 Part II gate"
  ```

**Mutation to kill:** accepting a non-Purchase provenance reds `non_purchase_provenance_is_refused_fail_closed`;
accepting an empty Part II reds `empty_part_ii_narrative_is_refused_at_record_time`; skipping the `would_conflict`
pre-check reds `a_second_promote_is_refused_by_would_conflict`.

### Task 11: §3 tag-side census — promote-aware advisories + the `$0` copy sweep

**Files:**
- Modify: `crates/btctax-core/src/conservative.rs` (the five advisories + `basis_methodology` :125-168)
- Modify: `crates/btctax-cli/src/cmd/tranche.rs` (`TRANCHE_IS_FINAL_HINT` :29-31, the "$0" refusal :95, the
  phantom-wallet copy :156-161), `crates/btctax-cli/src/session.rs:694-695`, `crates/btctax-core/src/project/resolve.rs`
  (the `SafeHarborUnconservable` blocker `$0` detail :1305-1315)
- Test: `crates/btctax-core/tests/kat_promote.rs`, `crates/btctax-cli/tests/promote_cli.rs`

**Interfaces:**
- Consumes: the promote set (reach a promoted leg/lot via `origin_event_id`); T8's advisory replaces the stale
  method-inversion/self-custody framing where a promoted lot is involved.

- [ ] **Step 1: Write the failing KATs:**
  ```rust
  #[test]
  fn basis_methodology_no_longer_claims_never_the_estimate_for_a_promoted_leg() {
      let text = basis_methodology(&promoted_state(), YEAR).unwrap();
      assert!(!text.contains("never the estimate"), "the >$0 promoted basis IS the estimate re-homed (tag-side census)");
      assert!(text.contains("estimated at the minimum daily closing price"), "promoted legs get the estimate disclosure");
  }
  #[test]
  fn dip_and_self_custody_copy_distinguishes_a_promoted_tranche() {
      // a promoted tranche is no longer "$0-basis" / already the substantiated-higher-basis case.
      let out = tranche_dip_advisory(&promoted_disposal());
      assert!(out.map_or(true, |s| !s.contains("$0")));
  }
  #[test]
  fn the_promote_funnel_line_quotes_the_clamped_delta() {
      // §3 item 2 / tax r1 I-3: an unpromoted tranche's nudge advertises a saving the CLAMPED promote can
      // deliver (or states the below-window-low caveat), never an unclamped over-quote.
      let lines = overpayment_nudge_lines(&unpromoted_below_low_tranche(), .., Some(&profile), ..);
      assert!(lines.iter().any(|l| l.contains("promote-tranche")));
      // the quoted funnel saving must equal the clamped promote delta (not the unclamped what-if):
      assert_eq!(funnel_quoted_saving(&lines), clamped_promote_saving(&unpromoted_below_low_tranche()));
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- basis_methodology dip_and_self funnel 2>&1 | tail -20`
- [ ] **Step 3: Implement.** `basis_methodology` (:141-165): distinguish promoted legs (via
  `leg.lot_id.origin_event_id ∈ promote set`) — the promoted `>$0` line gets the estimate disclosure ("basis
  estimated at the minimum daily closing price over the attested acquisition window (Cohan)"; the clamped-leg
  "limited so as not to report a loss" sentence when the clamp bound); the documented-fee sentence stays for
  the non-promoted `>$0` fee case; the `$0` sentence stays for an unpromoted tranche. `method_inversion_advisory`
  (:57-87) / `self_custody_nudge` (:96-114) / `tranche_dip_advisory` (:25-49): generalize the "$0-basis" copy
  (basis-as-filed). `overpayment_nudge_lines` (:366-451): an unpromoted tranche → the existing nudge + a
  `promote-tranche` funnel line (quoting the CLAMPED delta or the below-low caveat); a promoted tranche → a
  status line. Sweep the CLI `$0` copy sites (tranche.rs, session.rs, resolve.rs blocker) to say "$0 or a
  promoted floor" where a promoted tranche is now representable. **★ SPEC §3 items 6 + 7 (this task owns them —
  plan-r1 M-2/M-6):** amend the parent D-7 wording in `design/conservative-filing/SPEC.md` (re-scope "nothing
  >$0 ever filed" to UNPROMOTED tranches); fix the now-false `event.rs` `DeclareTranche` doc ("v1 declares $0
  ONLY (no floor)", :214-220) and the `EstimatedConservative` doc; fix the now-false `forms.rs` §170(e) "$0"
  doc *sentence* (:264-268) — **NB: T6's "verify-only, NOT patched" rule applies to the six basis CONSUMERS,
  not to this doc comment, which this task DOES fix**; re-scope every "$0-only" test (kat_tranche/kat_conservative)
  to "unpromoted". **Whole-surface rule:** grep `"$0"` + man/docs/goldens in one pass (project memory
  `whole-surface-sweep-on-taxonomy-change`).
- [ ] **Step 4: Run — expect PASS** + `make check` + regen any advisory goldens (`make examples` if J-journeys touch tranche copy).
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/conservative.rs crates/btctax-cli/src/cmd/tranche.rs crates/btctax-cli/src/session.rs crates/btctax-core/src/project/resolve.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): §3 tag-side census — promote-aware advisories + \$0 copy sweep"
  ```

**Mutation to kill:** leaving `basis_methodology`'s "never the estimate" sentence reds
`basis_methodology_no_longer_claims_never_the_estimate_for_a_promoted_leg` (a promoted filer's disclosure would
misstate that the >$0 is documented fee basis, not the estimate — a §6662 honesty defect).

### Task 12: §3 payload-side census — render arms for `PromoteTranche`

**Files:**
- Modify: `crates/btctax-cli/src/main.rs` (`bulk_void_payload_summary` catch-all :2171; decision-list render :2164-2172)
- Modify: `crates/btctax-tui-edit/src/main.rs` (`summarize_void_payload` catch-all `_ => ("?", …)` :3844)
- Modify: `crates/btctax-cli/src/session.rs` (`safe_harbor_residue` filter :713-716)
- Test: inline unit tests + a comment KAT on the deliberate `bulk_resolve_payload_summary` omission (:2083)

**Interfaces:** consumes T1 payload. No new API — these are render/filter arms the compiler does NOT force.

- [ ] **Step 1: Write the failing tests:**
  ```rust
  #[test]
  fn bulk_void_summary_renders_a_promote_readably_not_debug() {
      let s = bulk_void_payload_summary(&promote_payload());
      assert!(s.contains("PromoteTranche") && !s.contains("filed_basis:"), "human-readable, not {:?} debug");
  }
  #[test]
  fn tui_void_flow_labels_a_promote_not_question_mark() {
      let (tag, _, _, _) = summarize_void_payload(&promote_payload());
      assert_ne!(tag, "?");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-cli -- bulk_void_summary_renders_a_promote 2>&1 | tail; cargo test -p btctax-tui-edit -- tui_void_flow_labels 2>&1 | tail`
- [ ] **Step 3: Implement.** Add a `PromoteTranche(p) => format!("PromoteTranche of {} → ${} floor", p.target.canonical(), p.filed_basis)`
  arm (`.canonical()` — `EventId` has no `Display`, arch r1 N-1) to `bulk_void_payload_summary` (before the
  `other =>` at :2171) and the decision-list render (:2164-2172);
  add the 4-tuple arm to `summarize_void_payload` (tui-edit :3832-3843, before `_ =>` :3844); add
  `| EventPayload::PromoteTranche(_)` to the `safe_harbor_residue` drop filter (session.rs:713-716) so a
  promote layered on a dropped `DeclareTranche` does not leak into the pre-2025 residue. Add a one-line comment
  + no-op test at `bulk_resolve_payload_summary` (:2083) documenting that a promote is deliberately unreachable
  there (imported-conflict-scoped — arch r2 N-2 false lead).
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-cli/src/main.rs crates/btctax-tui-edit/src/main.rs crates/btctax-cli/src/session.rs
  git commit -m "feat(promote): §3 payload-side census — void/summary render arms + safe_harbor_residue filter"
  ```

**Mutation to kill:** removing the `bulk_void_payload_summary` arm reds `bulk_void_summary_renders_a_promote_readably_not_debug`
(a promote void renders as `{:?}` in the exact flow BG-D9 depends on — the silent hazard BG-D1 claims to kill on the payload side).

### Task 13: Form 8275 content generation (Part I auto + Part II narrative) + BG-D7/D10 copy

**Files:**
- Create: `crates/btctax-core/src/tax/form8275.rs` (+ `mod form8275;` in `tax/mod.rs`)
- Reference: `basis_methodology` (`conservative.rs:125-168`) is the Part-I seed; the promote's `part_ii_narrative`
  (on the event) is Part II.
- Test: `crates/btctax-core/tests/kat_promote.rs`

**Interfaces:**
- Produces (BOTH the content struct AND the `Printed8275` newtype Phase 1b consumes — arch r1 I-5; defined
  HERE, next to `Disclosure8275`, so T15/T16 have one owner):
  ```rust
  // tax/form8275.rs
  pub struct Part1Item { pub form: String, pub line: String, pub description: String, pub amount: Usd }
  pub struct Disclosure8275 {
      pub part_i: Vec<Part1Item>,  // item = Form 8949 col (e), the AS-FILED amount (tax r1 I-6), per promoted DISPOSAL leg
      pub part_ii: String,         // the filer's stored narrative (present-by-construction, BG-D7)
      pub incomplete: bool,        // T14 gate condition: empty/scaffold Part II (a raw-vault bypass)
  }
  pub fn disclosure_8275(events: &[LedgerEvent], state: &LedgerState, year: i32) -> Option<Disclosure8275>;
  impl Disclosure8275 { pub fn render(&self) -> String; }
  // in crates/btctax-core/src/tax/printed.rs (mirror Printed8283Rows :135), constructed from Disclosure8275:
  pub struct Printed8275 { pub part_i: Vec<Part1Item>, pub part_ii: String } // whole-dollar-rounded amounts
  pub fn printed_8275(d: &Disclosure8275) -> Printed8275;
  ```

- [ ] **Step 1: Write the failing KATs:**
  ```rust
  #[test]
  fn disclosure_is_some_iff_a_promoted_leg_is_filed_this_year() {
      assert!(disclosure_8275(&promoted_disposal_events(), &promoted_state(), YEAR).is_some());
      assert!(disclosure_8275(&unpromoted_tranche_events(), &unpromoted_state(), YEAR).is_none());
  }
  #[test]
  fn disclosure_copy_names_the_underpayment_penalty_base_never_safe_harbor() {
      let d = disclosure_8275(&promoted_disposal_events(), &promoted_state(), YEAR).unwrap();
      let text = d.render();
      assert!(!text.to_lowercase().contains("safe harbor"));             // BG-D7
      assert!(text.contains("of the resulting additional tax"));          // BG-D10 penalty base (tax r1 M-3)
      assert!(text.contains("40%"));                                      // BG-D10 §6662(h) worst case
      assert!(text.contains("§6664(c)(2)"));                              // corrected cite (tax r2 N-1)
  }
  #[test]
  fn a_clamped_leg_disclosure_adds_the_no_loss_sentence_and_files_the_clamped_amount() {
      // BG-D7 (tax r1 M-4/I-6): the Part I amount is the AS-FILED 8949 col (e) = the clamped basis (= net
      // proceeds), NOT the floor — disclosing the floor while filing less recreates the examiner mismatch.
      let d = disclosure_8275(&promote_sold_below_low(floor=12_000, proceeds=8_000), &state, YEAR).unwrap();
      assert!(d.render().contains("limited so as not to report a loss from the estimate"));
      assert_eq!(d.part_i[0].amount, filed_8949_col_e_basis(&state), "Part I amount = as-filed 8949 col (e)");
      assert_ne!(d.part_i[0].amount, dec!(12_000), "NOT the pre-clamp floor");
  }
  #[test]
  fn removal_donation_legs_are_absent_from_part_i() {
      // Post-BG-D11 a promoted removal leg files documented-only; an 8275 "form 8283, amount=floor" would
      // disclose a position the return never takes (tax r1 I-6). Part I is 8949-DISPOSAL-scoped.
      let d = disclosure_8275(&promote_then_donate_short_term(), &state, YEAR);
      assert!(d.map_or(true, |d| d.part_i.iter().all(|i| i.form == "8949")), "no 8283/removal items in Part I");
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-core --test kat_promote -- disclosure clamped_leg removal_donation 2>&1 | tail -30`
- [ ] **Step 3: Implement** `disclosure_8275`: `None` unless a promoted **disposal** leg is filed in `year`
  (a promoted removal leg files documented-only per BG-D11, so it takes no estimated position to disclose —
  tax r1 I-6); Part I = one item per promoted **8949 disposal** leg (`form: "8949"`, the col/line, description
  "basis estimated at the minimum daily **closing** price over the attested acquisition window (Cohan; the
  bearing-heavily minimum)", **amount = `leg.basis` AS FILED** — the clamped amount where the clamp bound, NOT
  the pre-clamp floor); when a leg's clamp bound, append "limited so as not to report a loss from the
  estimate"; Part II = the promote's stored `part_ii_narrative` (`incomplete = part_ii.trim().is_empty()` — the
  raw-vault bypass condition for T14). `render()` also emits the BG-D10 risk
  paragraph: "20% ordinary / 40% worst-case **of the resulting additional tax** (the underpayment attributable
  to the misstatement), plus interest; the 8275 and good-faith methodology mitigate, they do not eliminate;
  adequate disclosure does NOT protect against the §6662(e)/(h) valuation-misstatement penalty (Woods); for
  charitable-deduction property §6664(c)(2) removes the reasonable-cause defense" — and NEVER "safe harbor".
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/tax/form8275.rs crates/btctax-core/src/tax/mod.rs crates/btctax-core/tests/kat_promote.rs
  git commit -m "feat(promote): Form 8275 content (Part I auto + Part II narrative) + BG-D7/D10 honest copy"
  ```

**Mutation to kill:** emitting "safe harbor" or "40% on the disallowed basis" reds
`disclosure_copy_names_the_underpayment_penalty_base_never_safe_harbor`; dropping the clamped-leg sentence reds
`a_clamped_leg_disclosure_adds_the_no_loss_sentence`.

### Task 14: BG-D8 real export-refusal gate (the completeness gate)

**Files:**
- Modify: `crates/btctax-cli/src/cmd/admin.rs` (the three export fns: `export_snapshot`/CSV :68-82,
  `export_irs_pdf` :281-284, `export_full_return` :535-538 — the refuse-before-bytes checked-first slots)
- Reference: the pseudo-export-block precedent (`fold.rs:396-406`, `state.pseudo_active()` state.rs:282-284);
  T13 `disclosure_8275`.
- Test: `crates/btctax-cli/tests/promote_cli.rs`

**Interfaces:**
- Produces: `fn promote_export_gate(state, events, year: Option<i32>) -> Result<(), CliError>` — refuses (ZERO
  bytes written) when a promoted disposal leg is filed but its 8275 disclosure is `None`/`incomplete`. `year:
  None` for the non-year-scoped CSV/snapshot export means "any year with a promoted filed leg in the exported
  range" (N-3). In Phase 1a the artifact is the content (`disclosure_8275`, `incomplete` = empty/scaffold Part
  II); T16 re-points the gate at the fillable PDF.

- [ ] **Step 1: Write the failing test.** The refusing state is reached only via a **raw-vault bypass** — a
  hand-appended `PromoteTranche` with an empty/scaffold `part_ii_narrative` (the CLI's T10 record-time refusal
  can't produce it; the hand-crafted-vault class the spec uses for BG-D9), so `disclosure_8275().incomplete`:
  ```rust
  #[test]
  fn export_with_a_promoted_leg_but_incomplete_8275_refuses_before_bytes() {
      let v = raw_vault_promote_with_empty_part_ii(); // bypasses T10; disclosure_8275().incomplete == true
      let err = cmd::admin::export_irs_pdf(&v, &pp(), &out, YEAR, all_forms(), None).unwrap_err();
      assert!(matches!(err, CliError::Usage(m) if m.contains("Form 8275")));
      assert!(std::fs::read_dir(&out).map(|mut d| d.next().is_none()).unwrap_or(true), "zero bytes written (refuse-before-bytes)");
  }
  #[test]
  fn a_clean_promoted_export_writes_the_8275_by_name_no_watermark() {
      let v = vault_with_promoted_disposal_via_cli(); // T10 path → complete disclosure
      cmd::admin::export_irs_pdf(&v, &pp(), &out, YEAR, all_forms(), None).unwrap();
      assert!(out.join("form_8275.txt").exists(), "the 8275 content is emitted by its OWN name (NOT || basis_methodology)");
      // clean export, no DRAFT watermark (real ledger, not pseudo) — SPEC BG-D8
  }
  ```
- [ ] **Step 2: Run — expect FAIL.** Run: `cargo test -p btctax-cli --test promote_cli -- export 2>&1 | tail -30`
- [ ] **Step 3: Implement.** Add `promote_export_gate` and call it FIRST in each of the three export fns
  (mirroring the `if state.pseudo_active() { require_attestation(...)? }` checked-first slot at admin.rs:80/283/537),
  BEFORE any file is written: if a promoted disposal leg is filed and `disclosure_8275(...)` is `None`/
  `incomplete` → `Err(CliError::Usage("refusing to export a packet with a promoted-basis leg but no complete
  Form 8275 …"))`. On success, write the 8275 disclosure content to `form_8275.txt` (its OWN name) alongside
  the packet, at the `write_basis_methodology_txt` call sites (render.rs:871/911, admin.rs:304/555). Clean
  export, no watermark. `basis_methodology.txt` continues to be written too, but the GATE and the success KAT
  key on the 8275 artifact by name — never the always-written `basis_methodology.txt` (tax r1 I-8).
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-cli/src/cmd/admin.rs crates/btctax-cli/src/render.rs crates/btctax-cli/tests/promote_cli.rs
  git commit -m "feat(promote): BG-D8 real export-refusal gate (refuse-before-bytes on a promoted leg without its 8275)"
  ```

**Mutation to kill:** implementing the gate as the always-writes `basis_methodology.txt` pattern (never refuses)
reds `export_with_a_promoted_leg_but_no_8275_content_refuses_before_bytes`.

---

**★ PHASE 1a GATE:** run `make check` + the CI-only jobs (fmt / check-isolation / pii-scan / msrv), then the
whole-1a-diff two-lens (tax + architecture) Fable review to **0C/0I**. Merge 1a to `main` (authorized per-phase).
Do NOT release — `promote` is not exposed in a *released* binary until Phase 1b lands the official PDF.

---

# PHASE 1b — the official Form 8275 fillable PDF

### Task 15: Official Form 8275 AcroForm map + fill + geometry readback

**Files:**
- Create: `crates/btctax-forms/forms/2024/f8275.pdf` (the official IRS blank) + `f8275.map.toml` (per-year field map)
- Create: `crates/btctax-forms/src/form8275.rs` (+ `mod form8275;` in `lib.rs`; `pub fn fill_form_8275(...)`)
- Modify: `crates/btctax-forms/src/pdf.rs` (`F8275_PDF_2024` const + `f8275_pdf(year)`), `map.rs` (`F8275_MAP_2024`
  + `Form8275Map` + `for_year`/`field_names`), `lib.rs` `testonly` re-exports
- Reference: `form8283.rs` `fill_one` skeleton (:323-485) — 8275 is **free-text** (`push_free`/`FlatPlacement::free`),
  NOT a money grid; `verify.rs::verify_flat` (:337).
- Test: `crates/btctax-forms/tests/sp4.rs` (new; mirror `sp2.rs` 8283 fault-injection + `map_2024_matches_bundled_pdf_fieldset`)

**Interfaces:**
- Consumes: T13 `Printed8275` + `printed_8275` (defined in T13's `tax/printed.rs` — arch r1 I-5; NOT created here).
- Produces: `pub fn fill_form_8275(printed: &Printed8275, header: &ReturnHeader, year: i32) -> Result<Option<Vec<u8>>, FormsError>`.

- [ ] **Step 1: Write the failing KATs** (mirror `sp2.rs`): `map_YEAR_matches_bundled_pdf_fieldset` for EVERY
  `SUPPORTED_YEAR` (every mapped field exists in the blank PDF); a fill-then-readback KAT; a fault-injected
  swapped-field map → fill FAILS CLOSED (`verify_flat`); `f8275_is_byte_deterministic` (SHA-256 golden); **a
  per-year fill KAT for a NON-2024 year (2025 and 2017)** so the year coverage is pinned (arch r1 I-6).
- [ ] **Step 2: Run — expect FAIL** (no `fill_form_8275`, no asset).
- [ ] **Step 3: Implement.** Add the blank `f8275.pdf` + `f8275.map.toml`; the `pdf.rs`/`map.rs` consts +
  accessors; `form8275.rs::fill_form_8275` following `fill_one`'s load→`drop_xfa_and_set_needappearances`
  →`apply_writes`→`strip_nondeterminism`→save→reload→`verify_flat`, emitting `push_identity` (filer) + `push_free`
  Part I item rows + `push_free` Part II narrative (geometry-exempt free-text with `/MaxLen` checked). ★ **Year
  coverage is MANDATORY, not conditional (arch r1 I-6 / tax r1 M-7):** Form 8275 is revision-versioned (not
  tax-year-versioned), so alias the single bundled revision to **every** year in `SUPPORTED_YEARS = {2017, 2024,
  2025}` — `f8275_pdf(year)` and the map `for_year(year)` return the same asset for all supported years. This
  prevents the re-pointed BG-D8 gate (T16) from PERMANENTLY refusing a promoted 2025/2017 export (the dominant
  current-year flow) while 2024-only KATs stay green.
- [ ] **Step 4: Run — expect PASS** + `make check`.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-forms/forms/2024/ crates/btctax-forms/src/form8275.rs crates/btctax-forms/src/pdf.rs crates/btctax-forms/src/map.rs crates/btctax-forms/src/lib.rs crates/btctax-forms/tests/sp4.rs
  git commit -m "feat(8275): official Form 8275 fillable PDF (AcroForm map + free-text fill + geometry readback)"
  ```

**Mutation to kill:** a swapped-column map lands a value in the wrong widget band → `verify_flat` fails closed
(the geometric oracle KAT reds); a non-deterministic fill reds the byte-golden.

### Task 16: Wire the 8275 PDF into export-irs-pdf + full-return packet + the DRAFT gate; re-point BG-D8

**Files:**
- Modify: `crates/btctax-core/src/tax/packet.rs:421` (`PrintedForms` — add `f8275: Option<Printed8275>`;
  populate in `assemble_printed_return` :461 when a promoted leg is present)
- Modify: `crates/btctax-forms/src/packet.rs:38-58` (add `f8275` to the **no-`..` exhaustive destructure** + a
  `push("f8275", Some("92"), ...)` — the compile-forced hook)
- Modify: `crates/btctax-cli/src/cli.rs:958-970` (`FormArg::Form8275`), `crates/btctax-cli/src/cmd/admin.rs:307+`
  (crypto-slice fill → `form_8275.pdf`); re-point T14's `promote_export_gate` at the PDF
- Modify: `crates/btctax-forms/tests/census.rs` (`CENSUS_KEYS` 14→15 + array arity + the J6 demonstration),
  `crates/btctax-forms/tests/sp3.rs` (`map_2024_matches_bundled_pdf_fieldset` block)
- Modify docs: `crates/btctax-cli/LIMITATIONS.md:63`, `crates/xtask/src/docs.rs:184/219`, `cli.rs` help
  doc-comments, then `make docs` to regen `docs/man/btctax-export-irs-pdf.1`
- Test: `crates/btctax-forms/tests/census.rs`, `crates/btctax-cli/tests/promote_cli.rs`

**Interfaces:** consumes T15 `fill_form_8275` + T13 `Disclosure8275`; the DRAFT gate (`admin.rs:533-538`) and
`stamp_draft_watermark` cover the 8275 for free.

- [ ] **Step 1: Write the failing tests:**
  ```rust
  #[test]
  fn census_is_exactly_15_forms_including_8275_when_a_promote_is_present() { /* mirror census_key_set_is_exactly_14 */ }
  #[test]
  fn full_return_packet_emits_8275_iff_a_promoted_leg_is_filed() { /* assemble_printed_return → fill_full_return */ }
  #[test]
  fn export_gate_now_refuses_when_the_8275_PDF_is_absent() { /* T14 gate re-pointed at the PDF */ }
  #[test]
  fn a_promoted_2025_export_fills_the_8275_and_the_gate_passes() {
      // arch r1 I-6: the 8275 asset aliases every SUPPORTED_YEAR, so a 2025 (or 2017) promoted export is NOT
      // permanently refused. Exercise a non-2024 promoted year end-to-end (fill + gate green).
      let v = vault_with_promoted_disposal_via_cli_year(2025);
      cmd::admin::export_irs_pdf(&v, &pp(), &out, 2025, all_forms(), None).unwrap();
      assert!(out.join("form_8275.pdf").exists());
  }
  ```
- [ ] **Step 2: Run — expect FAIL** (destructure won't compile without the `f8275` member arm → the compile-forced hook fires).
- [ ] **Step 3: Implement.** Add `f8275` to `PrintedForms` + the `fill_full_return` exhaustive destructure + a
  `push` at Attachment Sequence "92" (8275's IRS sequence); populate `f8275` in `assemble_printed_return` only
  when a promoted leg is filed; add the crypto-slice fill + `FormArg::Form8275`; bump `CENSUS_KEYS` to 15
  (+ arity + J6); re-point `promote_export_gate` (T14) to require the **PDF** artifact; update
  LIMITATIONS.md/docs.rs/cli help + `make docs`.
- [ ] **Step 4: Run — expect PASS** + `make check` + `make docs` (determinism KATs green) + the CI-only jobs.
- [ ] **Step 5: Commit.**
  ```bash
  git add crates/btctax-core/src/tax/packet.rs crates/btctax-forms/src/packet.rs crates/btctax-cli/src/cli.rs crates/btctax-cli/src/cmd/admin.rs crates/btctax-forms/tests/census.rs crates/btctax-forms/tests/sp3.rs crates/btctax-cli/LIMITATIONS.md crates/xtask/src/docs.rs docs/
  git commit -m "feat(8275): wire Form 8275 into export-irs-pdf + full-return packet + DRAFT gate; BG-D8 points at the PDF"
  ```

**Mutation to kill:** omitting the `f8275` arm from the no-`..` destructure fails to compile (the anti-drift
hook); a stale `CENSUS_KEYS[14]` reds `census_is_exactly_15_forms_including_8275_when_a_promote_is_present`.

---

**★ PHASE 1b GATE + SHIP:** `make check` + CI-only jobs + `make docs` + the whole-1b-diff two-lens Fable review
to **0C/0I**; merge 1b to `main`. The complete 1a+1b unit is now shippable → RELEASE (version bump + tag +
GitHub release + `cargo publish --workspace`) per the [[crate-publishing-state]] recipe.

---

## Self-Review (author checklist — run against the SPEC)

**Spec coverage:** BG-D1 → T1/T3; BG-D2 → T1 (`FloorMethod`); BG-D3 compute → T2, **verify-drift advisory →
T11 (added, plan-r1 I-2/I-4)**; BG-D4 clamp → T4 (+ `PromoteSet` threading), fee-evaporation → T5; BG-D5 → T10;
BG-D6 → T9 (terms incl. the unrealized flavor) + T10 (recording + consent copy); BG-D7 → T10 (Part II gate) +
T13 (content, 8949-scoped Part I); BG-D8 → T14 (real refusal via `incomplete`) + T16 (PDF); BG-D9 lifecycle →
T7 (deferred void, correct insertion point), advisory+cascade → T8 (fn) + T10/T8-3b (**wired both directions**);
BG-D10 → T13 + T10 (consent copy); BG-D11 → T6 (+ LT-column KAT). §3 tag-side → T11 (incl. items 6/7 doc/test
re-scope); §3 payload-side → T7 (`is_revocable_payload`) + T3 (`build_op` item 11) + T12 (render/filter). Phase
1b → T15 (multi-year 8275) / T16. The `PromoteSet`/`Printed8275` types have one owner each (T2 / T13).

**Plan-review r1 fold (both lenses):** the "No gap found" claim above was an **overclaim** and is retracted —
r1 found two spec surfaces with no owning task (BG-D3 drift → now T11; the BG-D9 advisory's call sites → now
T10 + T8-3b), a `PromoteSet` with no producer/threading (→ T2 defines it, T3 produces + puts it on
`Resolution`, T4 threads it into `FoldCtx`), a mis-placed void adjudication (T7 → after pass-1a, before step 2),
`ConsentTerm` flavors that couldn't express the computing-deduction-Δ and the unrealized line (→ T1), a
floor-vs-as-filed 8275 amount (→ T13), an uncreated `Printed8275` (→ T13), 8275 year-coverage (→ T15 mandatory
aliasing), and several under-pinned/vacuous KATs (→ folded into the owning tasks). All 8+6 Importants + 11
Minors/Nits folded; the `Usd::from_dollars` → `dec!` sweep applied file-wide; `EventId` rendered via
`.canonical()`.

**Placeholder scan:** no "TBD"/"handle edge cases"/"similar to Task N" — each task carries its own real test +
implementation sketch + exact insertion points. (Test/impl bodies use the SPEC's exact formulae and the
exploration's exact symbols; helper fixtures like `promote_ev`/`prices_with_window_min` are named where a task
first needs them.)

**Type consistency:** `PromoteTranche`/`Acknowledgment`/`ConsentTerm`/`FloorMethod` (T1) are referenced with
the same field names throughout (`filed_basis`, `part_ii_narrative`, `shown_terms`, `target`); `filed_basis_for`
(T2) → `filed_basis` on the payload (T1) → the T3 rewrite → the T4/T5/T6 decomposition key
(`leg.lot_id.origin_event_id`) are one consistent chain. `consent_terms` (T9) returns the exact `Vec<ConsentTerm>`
recorded by T10. `disclosure_8275`/`Disclosure8275` (T13) is consumed by T14 and T16.

## Execution Handoff

Two execution options:
1. **Subagent-Driven (recommended)** — a fresh subagent per task, two-stage review between tasks, fast iteration
   (superpowers:subagent-driven-development). Best for a 16-task engine change where each task is independently gated.
2. **Inline Execution** — batch execution in-session with checkpoints (superpowers:executing-plans).

**BUT FIRST (STANDARD_WORKFLOW gate):** this plan is a written design artifact — it passes its own independent
two-lens (tax + architecture) Fable review to **0C/0I** *before* any execution. Persist each reviewer verbatim,
fold, re-review to green. Only then pick an execution mode.
