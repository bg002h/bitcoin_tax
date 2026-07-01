# FOLLOWUPS — bitcoin_tax (TaxApp)

Open/!resolved action items (STANDARD_WORKFLOW §4). Each: what · why · status · pointer.

---

## Standing roadmap — next program (user-approved 2026-06-30; auto-pick-up after slugs ship)

After the Phase-1 burndown (below) + the two in-flight slugs (pre-2025 filed-method reconciliation
mechanism; minimal appraisal-trigger FMV>$5k∧basis>$5k) ship, **automatically pick up Phase 2: Forms
& §170(e) deduction computation** — no re-ask. Sequence: §170(e) charitable-deduction computation
(FMV-vs-basis, ST/LT reduction) → upgrade the minimal appraisal-trigger to the precise
>$5k-claimed-deduction trigger (§170(f)(11)(C)); Form 8949 + Schedule D generation; Form 8283 + Form
709 routing; SE-tax routing (business mining → Schedule SE); slot in **B-M1** (NIIT loss-year
understatement). Lower/triggered: adapter refinements (TransferIn basis gap, Gemini BTC-pair FMV,
owner-confirms), hardening + Windows/macOS CI, 2026/2027 tax tables (arms the 2027+ broker gate),
§1091 wash-sale enactment, multi-year horizon optimization, non-BTC scope. (Mirror of memory
`phase2-standing-roadmap`.)

## ✅ Burndown pass 2 (2026-06-30) — A/B/C deferrals resolved

Branch `chore/followups-burndown-2`, three groups each independently reviewed to 0 Critical / 0
Important; workspace gate green (433 tests). Closed:

- **A (lot-id):** A-M1 (`disposal_compliance` SelfTransfer scope — documented intentional exclusion,
  code doc + SPEC §A.5); A-Task-7-M2 (extracted shared `method_election_is_forward` predicate, DRY,
  De-Morgan-verified behavior-preserving); A-Task-8a (`compliance_status_tag` stable, both renderers
  off `{:?}`); A-Task-8b (selection_count guard — moot, documented); A-Task-9b (no-op identity KAT
  `evaluate_disposal(existing,no-selection)==project()`); A-M3 (binary-level `Command::Config`
  dispatch tests); A-Task-4 plan doc `90.00`→`90.25`.
- **A-N2 / A-N3 — RESOLVED:** N2 (evaluate_disposal `lots_after` shape for C) — C shipped and Mode-2
  `consult_sale` consumes `evaluate_disposal` successfully. N3 (B/C per-year Hard-blocker gate) — B's
  `compute_tax_year` `first_hard_blocker` gate + C's `PreTransitionYear`/`YearNotComputable` refusal
  both shipped. No code owed.
- **B (rate engine):** B-F1 (`fmt_money` 2dp on all tax-report money fields, display-only — no tax
  figure changed); B-Minor (`niit_applies` doc aligned to code semantic); B-nits (redundant
  rust_decimal_macros dev-dep removed; `filing_status_tag` stable in tax-profile --show; `events`
  param kept+documented; advisory-only→Computed KAT; §4.3 stale doc line).
- **C (optimizer):** C-M1 (exhaustive_min eviction strict-only → baseline wins exact ties, no
  delta==0 divergent pick; oracle-exactness + delta≤0 + determinism preserved; regression KAT
  `tie_exact_baseline_kept_when_lex_smaller_is_not_baseline`); C-M2 (`ConsultReport.approximate` from
  the heuristic flag + ⚠ note in render_consult); C-M3 (proposal scope-boundary footer).

---

## ✅ Slug: pre-2025 filed-method reconciliation mechanism — SHIPPED (2026-06-30)

Branch `feat/pre2025-reconciliation`; R0 spec 2 rounds to 0C/0I; 2 impl passes each reviewed 0C/0I;
whole-slug review 0C/0I. Gave the pre-2025 method declaration engine teeth: `ProjectionConfig`
gains `pre2025_method_attested` (plumbed via `to_projection`); `note_pre2025_once` advisory is
attestation-aware (unattested "have NOT declared" + guidance / attested "DECLARED + ATTESTED", still
Advisory — never gates `compute_tax_year`); `safe-harbor-allocate` REFUSES under an undeclared method
(appends nothing; reads the config flag, not `timely_allocation_attested`). Basis-adjustment math
unchanged. 441 tests.

Deferred from this slug (OPEN):
- **Durable Path-A `Pre2025MethodDeclaration` ledger event (R0-I2).** For a Path-A (no-allocation)
  taxpayer the attested method lives only in mutable `cli_config` (not source-of-truth per NFR6) — no
  audit trail. Add an append-only, supersede-tracked declaration event so the attestation is auditable
  in the ledger. Deferred because it changes NO number for Path A (basis recomputes live under the set
  method; the advisory updates with it) — audit-trail enhancement, not a correctness gap. — OPEN.
- **N-1 (Nit) — `safe_harbor_allocate` reads `session.config()?` twice** (gate + `to_projection`);
  collapse to one read. Cleanup, no correctness impact. — OPEN.
- **N-2 (Nit) — no separate non-FIFO attested-allocate success KAT.** The gate is method-agnostic
  (`if !attested { refuse }`) and KAT (c) proves attested-FIFO allocate records the method; a
  LIFO/HIFO-attested allocate test would round out coverage. — OPEN.

---

## C.5 — Monitor §1091 crypto wash-sale enactment (OPEN)

**What.** §1091 currently disallows losses only on "stock or securities"; crypto is property
(Notice 2014-21) and is **exempt**. The optimizer therefore selects loss lots freely — there is
no 30-day disallowance rule in the current code.

**Why monitor.** Recurring Greenbook proposals and legislative bills (e.g. various "Build Back
Better"-era and subsequent drafts) have proposed extending §1091 to digital assets. None have
been enacted as of this writing (2026-06-30).

**If enacted:** add a 30-day look-back disallowance guard to loss-lot selection in
`crates/btctax-core/src/optimize.rs` (the C.5 doc note identifies the attachment point) AND
update the `## §1091 wash sale (C.5)` module doc note in lockstep. The regression KAT
`tests/optimize_wash_sale.rs::loss_lot_freely_selectable_no_wash_sale_bar` must also be
revised to assert the guard (not the current free-selection behavior).

**Pointer.** `optimize.rs` module doc `## §1091 wash sale (C.5)`; KAT
`tests/optimize_wash_sale.rs`.

---

## Sub-project C (optimizer) — Task-3 review IMPORTANT resolved (2026-06-30)

- **RESOLVED — `available_lots_before` returned the wrong pre-disposal pool for the FIRST 2025 disposal
  under safe-harbor Path B (FIXED).** The old truncate-then-refold never crossed `TRANSITION_DATE` when the
  target disposal was the chronologically-first 2025 timeline event, so the re-fold never fired the §7.4
  transition seed and surfaced the UN-seeded Universal residue — harmless under Path A (residue relocates by
  wallet; lot_ids/basis preserved) but WRONG under Path B (the seed DISCARDS the residue and installs
  `SafeHarborAllocation` seed lots with different lot_ids/basis). Fix: new
  `pub fn fold::pools_before(res, prices, config, target) -> PoolSet` (fold.rs) folds the canonical timeline
  up to (but not including) the target and fires the real `transition::seed_transition` at the correct
  boundary (the seed check runs before the target short-circuit, so it fires even when the target is the
  first ≥2025 event); `available_lots_before` now delegates to it (no duplicated seed logic). KATs added:
  `available_lots_before_path_b_first_2025_disposal_returns_seeded_lots` (fails without the fix) +
  `available_lots_before_path_a_first_2025_disposal_relocates_residue`. R0-I1 canonical ordering preserved
  inside `pools_before`. — RESOLVED (2026-06-30). — optimize.rs / fold.rs; plan §TASK 3 updated.

---

## ✅ Burndown pass (2026-06-29) — actionable Phase-1 items resolved

Branch `chore/followups-burndown`, each fix independently reviewed to 0 Critical / 0 Important;
workspace gate green. What was closed:

**btctax-cli (commits f6880e6, 39e09e0, 282ae20, 4a78727):**
- **RESOLVED — `safe_harbor_status` goes dark when all Path-B lots consumed.** Now ORs in
  `state.disposals[*].legs[*].basis_source` + `removals[*].legs[*].basis_source == SafeHarborAllocated`
  (legs are not filtered by `remaining_sat`), so an effective Path B reports "effective" even after every
  allocated lot is disposed. Test added (all-consumed + stale advisory → still "effective"). Reviewer
  confirmed it cannot mask a genuine time-bar or unconservable state (those never seed SafeHarborAllocated lots).
- **RESOLVED — `verify` double-loads events (recon M-1 / eng M1).** Added
  `Session::load_events_and_project() -> (Vec<LedgerEvent>, LedgerState, ProjectionConfig)`; `verify` and
  `safe_harbor_attest` routed through it. Behavior-preserving; unit-tested.
- **RESOLVED — `{:?}` Debug enums in CSV (eng-M2).** Six stable snake_case `*_tag()` fns
  (`term`→`short`/`long`, `dispose_kind`→`sell`/`spend`, `basis_source`→`exchange`/`cost`/`safe_harbor`/…,
  etc.); all four CSV writers + text renderers switched off `{:?}`. CSV columns are now a stable contract.
  Export test asserts column values. (Exhaustive matches — no `_` fallback masking a real variant.)
- **RESOLVED — weak `set-fmv` test (recon N-1).** Repointed to an FMV-missing `Income` target; asserts the
  `FmvMissing` hard blocker present BEFORE and cleared AFTER `set-fmv` (+ income recognized at the manual FMV).
- **RESOLVED — attest leaves a stale `safe_harbor_timebar` advisory (Plan-4 fold I-2 follow-on).** Subsumed by
  the `safe_harbor_status` fix above (status now keyed on the effective-Path-B signal, not the advisory).

**btctax-adapters (commit 614d43a):**
- **RESOLVED — Swan zero-sat withdrawal counted under `dropped_no_btc` (tax Nit).** Added a distinct
  `skipped_zero_sat` counter to `GroupOutput`/`FileReport` (+ `merge`/`ingest` threading); the Swan arm now
  increments it instead of `dropped_no_btc`. Bucket-neutral (`parsed_rows = rows.len()` counted once), so the
  FR2 identity `parsed_rows = events + dropped_no_btc + unclassified + skipped_zero_sat` holds exactly. Test added.
  CLI import render reads named fields → no CLI change needed.
- **RESOLVED — River `business: false` immutability (tax M2).** Doc note added at both `Income` construction
  sites: `business: false` is hard-coded + immutable post-ingest (Income is not `ClassifyRaw`-able); SE-tax
  exposure requires confirming/changing the mapping at the adapter layer.

**btctax-core (verified by read-only survey — NO code change needed):**
- **VERIFIED already-handled — tax m1 (loss-basis cross-lot edge).** The `loss_basis` drop on a non-dual
  survivor is deliberate + taxpayer-conservative (promoting `None→Some` would misclassify a later sale as a
  §1015(a) dual-basis disposition — a far larger error). KAT `self_transfer_fee_c_cross_lot_normal_survivor_stays_non_dual` (kat_tax.rs:1204).
- **VERIFIED already-handled — tax m3 (principal==0 fee'd transfer).** All four fee arms raise an
  `UncoveredDisposal` blocker (not a silent drop) when there's no surviving leg/lot (fold.rs:569/394/770/836);
  fee-sats still consumed so conservation holds.
- **VERIFIED already-handled — 2025-transition timezone straddle.** Timeline partitioned at the **tax-date**
  boundary (`fold.rs:281` stable sort on `e.date() >= TRANSITION_DATE`); `universal_snapshot` + `pool_key` use
  the same tax-date predicate, so the pre-seed residue matches. KAT `reversed_offset_straddle_seeds_on_tax_date_not_utc_order` (transition.rs:546).
- **VERIFIED already-handled — `allocation_voids`.** Properly declared (resolve.rs:270), populated (286-289),
  consumed in the pass-3 irrevocability check (591-599) — the void-of-allocation behavior the CLI attest relies on.
- **ACCEPTED de-minimis tradeoff — tax m2 (exact-boundary fee holding-period attribution).** When principal
  drains exactly to a lot boundary, the fee-cents basis (from the next lot) rides the earlier lot's holding
  period. Total basis is conserved; only the HP anchor of a few cents shifts, only in the exact-boundary case.
  Fixing it requires splitting fee basis into a separate micro-leg/lot in the conservation-critical fold —
  not worth the complexity/risk for a cents-scale effect. WONTFIX (Phase-1); revisit only if shown material.

---

## ✅ Cycle-prep slug burndown (2026-06-29) — second pass

Ran `cycle-prep` recon (`reviews/cycle-prep-recon-2026-06-29.md`) on four slugs, then burned down one at a time
(cycle-prep → spec → opus R0 review-to-green → implement (SDD) → whole-slug review → ship). Each shipped at
0 Critical / 0 Important; PII-clean; workspace gate green throughout.

- **`vault-half-created-autorepair` — SHIPPED** (merge `db9f074`). `StoreError::HalfCreatedVault` + explicit
  `init --repair` that clears ONLY an orphan key (lock-first `AlreadyExists` guard provably never deletes a
  real/recoverable key); R0 caught the `init::run` arity blast-radius (fixed via wrapper); safety review 0C/0I.
- **`reconcile-allocation-dual-loss-basis` — SHIPPED** (merge `dd990f9`). `AllocLot` gains
  `dual_loss_basis`+`donor_acquired_at` (serde-default); Path-B seed + CLI allocate preserve the §1015(a) dual
  basis + §1223(2) tacking. R0 caught 3 inverted §1015(a) labels pre-implementation (gain=donor carryover,
  loss=FMV-at-gift); conservation unchanged.
- **`pre2025-filed-method-reconciliation` — Phase-1 part SHIPPED** (merge `c881967`). The advisory
  `Pre2025MethodNote` already existed + is surfaced in `verify`; made its message actionable (FIFO-assumed +
  reconcile-against-filings). **The runtime reconciliation MECHANISM (declare filed method → adjust
  reconstructed basis) remains OPEN — Phase-2 feature, deferred.**
- **`appraisal-trigger-precision` — NO-OP** (cycle-prep found the follow-up structurally wrong: no Phase-1
  FMV>$5k auto-flag exists; `appraisal_required` is a user CLI bool). Corrected the citation; Phase-2 only.

## Sub-project A (lot-id substrate) — items folded from the R0-plan review round 1 (2026-06-29)

- **Acquisition-date FIFO corrects a latent §1012 foundation deviation for relocated/seeded lots (R0-plan C1).**
  The shipped foundation's `consume_fifo` walks **insertion (push) order** (`pools.rs:58-100`); Sub-project A's plan
  makes FIFO **acquisition-date order** (`acquired_at` asc, tie `lot_id` asc) at all six consume sites. For
  **relocated** (self-transfer, `fold.rs:537-553,580-583`) and **Path-B-seeded** (`resolve.rs:566-586` →
  `transition.rs:67-73`) lots — which carry an `acquired_at` older than their push position — this is a **material
  behavior change**, not a no-op: it changes reported basis/term on the affected disposals **and** the safe-harbor
  conservation residue `snap.basis` (`transition.rs:25-51`; guard `resolve.rs:546-547`). It is the **legally-correct**
  rule (§1.1012-1(j)(3)(i): earliest *acquisition*; a self-transfer is not a new acquisition, `fold.rs:545`). Landed
  deliberately in A's plan (Task 2 deliberate-change statement + mandatory fixture-re-verification; RED→GREEN divergence
  KATs in Tasks 3 and 6), conservation-re-verified across existing self-transfer / Path-B / safe-harbor fixtures.
  **No real users exist yet (foundation just shipped), so no migration/restatement is owed.** Spec §A.3 reframed
  (deliberate-correctness note) + the spec M2 fold-record line updated. — RESOLVED-in-design (lands when A is
  implemented). — R0-plan C1, `reviews/R0-plan-lot-id-substrate-round-1.md`.

- **N3 (verified N/A) — `inspect::verify` "reads config twice."** `Session::load_events_and_project()` returns a
  **`ProjectionConfig`** as its third tuple element (burndown 2026-06-29, commit 39e09e0), *not* a `CliConfig`. `verify`
  needs the `CliConfig` (declared `pre2025_method` + `pre2025_method_attested`) for its new surfacing, so the separate
  `session.config()?` read is **required**, not redundant. No change. — R0-plan N3.

## Sub-project A (lot-id substrate) — whole-branch review round 1 deferrals (2026-06-29)

The blocking Important (post-hoc selection + in-force election mis-labeled `StandingOrder`) and in-area Minors
**M2** (`evaluate_disposal` existing-event principal) + **M3** (`config --set-forward-method` apply-all) were FIXED
on `feat/lot-id-substrate` (Task-10 fold). The remaining items below are deferred (non-blocking).
Source: `reviews/whole-branch-review-lot-id-substrate-round-1.md`.

- **M1 (Minor coverage gap) — `disposal_compliance` omits method-honoring SelfTransfers.** SelfTransfers produce no
  Disposal/Removal record, so they never get a compliance row (`compliance.rs` iterates only `state.disposals` /
  `state.removals`). A.3 lists SelfTransfer as method-honoring (a §1.1012-1(j) "transfer" that pre-positions lots
  for future HIFO/gains), so a post-hoc `select-lots` on a self-transfer is never compliance-flagged. Decide
  explicitly whether transfers belong in the projection; if intentionally excluded, document it. — OPEN. — whole-branch M1.

- **Task-4 plan-text `dec!(90.00)` → `90.25` (doc only).** A KAT-text figure in the Task-4 plan reads `90.00` where
  the implemented (correct) TP8(c) fee re-home yields `90.25`. Implementation is correct; only the plan doc text is
  stale. Reconcile the plan text. — OPEN (doc). — whole-branch Task-4 triage.

- **Task-7-M2 — shared election-collector DRY.** `compliance.rs::collect_elections` duplicates resolve's
  `MethodElectionBackdated` guard (both kept in sync by the shared spec rule). Extract a single shared collector to
  reduce drift risk (would also have de-risked the M1 classifier fix). DRY only — no behavior change. — OPEN. — whole-branch Task-7-M2.

- **Task-8 nits.** (a) `ComplianceStatus` is rendered with `{:?}` in `render_verify` — compliance-facing output should
  use a stable `compliance_status_display` (mirrors the burndown `*_tag()` work). (b) `selection_count` lacks a
  `Decision`-guard; moot in practice (a `LotSelection` payload only ever rides a `Decision` event). — OPEN. — whole-branch N1 / Task-8.

- **Task-9 nits.** (a) `evaluate_disposal`'s synthetic event id uses a `u64::MAX` sentinel — documented and
  unreachable by real sequences; revisit only if a typed sentinel is preferred. (b) Add a pinning KAT asserting
  `evaluate_disposal(existing-disposal, no selection) == project()` for that disposal (no-op identity). — OPEN. — whole-branch Task-9.

## ✅ RESOLVED earlier (kept for record)

## btctax-core whole-branch fixes (2026-06-29) — both Important findings resolved

- **I-1 — `ReclassifyOutflow → Dispose` on-chain `fee_sat` silently dropped (FIXED).**
  Added `fee_sat: Option<Sat>` to `Op::Dispose`; `OutflowClass::Dispose` arm now passes
  `t.fee_sat`; native `EventPayload::Dispose` passes `None`. Fold arm calls `consume_fee`
  after principal and re-homes carry onto last disposal leg via `rehome_onto_disposal_leg`.
  Fee-sats are consumed; holdings no longer overstated; conservation is honest.
  KATs: `reclassify_dispose_fee_sat_treatment_c_conservation_honest` and
  `reclassify_dispose_fee_sat_treatment_b_mini_disposition` — both pass.

- **I-2 — Path-B seeded-lot `LotId` collision after post-2025 `SelfTransfer` (FIXED).**
  Added `PoolSet::init_split_counter(origin, next)` and called it in `seed_transition`'s
  Path-B arm after pushing seed lots, setting `next_split[allocation_id] = seed.len()`.
  Later `bump_split(allocation_id)` returns `seed_len` (not 0), so relocated fragments get
  fresh unique split sequences.
  KAT: `path_b_seeded_lot_relocation_no_lotid_collision` — all LotIds unique, conservation
  balanced after partial relocation of a seeded lot.

- **Phase-2 refinement note:** The precise fee-sat disposition treatment when a
  `TransferOut` is reclassified as Dispose is a TP8-adjacent Phase-2 refinement (the Phase-1
  TP8 treatment is applied correctly per the existing TreatmentC/B config; any further
  guidance-specific nuance is deferred).

## btctax-adapters whole-branch fixes (2026-06-29) — both Important findings resolved

- **I-1 — Gemini Buy/Sell on BTC-quoted pairs (ETHBTC/BCHBTC) → Unclassified (FIXED).**
  Added `cols::SYMBOL` and gated `Buy/Sell → Acquire/Dispose` on `Symbol == "BTCUSD"` (case-insensitive)
  OR `USD Amount USD` present-and-non-empty. Any `Buy`/`Sell` row failing both checks emits `Unclassified`
  with `raw_of(row)` — never falls through to `usd_cost/proceeds = ZERO`, never guesses direction.
  KATs: `gemini_btcquoted_pair_buy_is_unclassified` (ETHBTC Buy → Unclassified, not Acquire, not zero-basis).
  §9.1 updated to state the rule.

- **I-2 — Gemini USD sign: magnitudes abs-normalized (FIXED).**
  Applied `.abs()` to `fee` at parse time in the Gemini parser and to `usd_abs` inside the Buy/Sell arm.
  `parse_usd` is unchanged (shared). A negative-encoded Buy no longer produces a negative `usd_cost`;
  a parenthesized Sell no longer produces a negative `usd_proceeds`. Applied only in `gemini.rs`.
  KATs: `gemini_negative_usd_normalized_to_positive` (negative USD Amount + parenthesized Fee → positive).

- **Phase-2 refinement note — full crypto↔BTC-pair FMV handling:** For a Gemini `ETHBTC` Buy/Sell the
  BTC leg IS a taxable disposition at FMV (or acquisition), but Phase 1 cannot auto-compute the BTC FMV
  for a non-BTCUSD pair without a second price lookup. These rows are conservatively emitted as
  `Unclassified` and require explicit user classification via reconciliation. Auto-recognizing the BTC
  disposition at FMV (e.g., by looking up the BTC/ETH rate from an exchange dataset) is a Phase-2
  refinement. — OPEN (Phase 2). — I-1 fix follow-on.

## btctax-adapters (Plan 3) — confirmed real schemas folded into §9.1 (2026-06-29)
- **CROSS-CRATE GAP — inbound `TransferIn` cannot carry cost-basis / acquisition-date (record clearly).**
  Swan `transfers` `deposit` rows carry **`USD Cost Basis` + `Acquisition Date`**, and Coinbase `Receive` /
  Gemini `Credit`(BTC) inbound rows may carry basis context, but core's
  `TransferIn { sat, src_addr?, txid? }` has **no field to hold a cost-basis or acquisition-date**. So at
  ingest every inbound on-chain row becomes a **plain `TransferIn`** and the exchange-supplied basis/date are
  **dropped from the event**. They must be **re-supplied by reconciliation (Plan 4)** — e.g. a
  `ClassifyInbound` decision (`GiftReceived{donor_basis, donor_acquired_at, …}`) or a future
  `ClassifyInbound`-style "external-acquisition" decision that records basis+date for an externally-sourced
  inbound. For a confirmed **self-transfer** the source lot is authoritative anyway (the Swan basis is only
  relevant for externally-sourced coins), so no data is lost there. **Candidate fix (Phase-2):** a
  reconciliation-hints side-table (or extra optional fields on `TransferIn`) so the adapter can persist the
  exchange-provided basis/date as a *hint* the reconciler can accept, instead of re-keying it by hand. —
  OPEN (Plan 4 reconciliation / Phase-2). — adapters §9.1 / plan FOUND GAP.
- **Swan withdrawals `source_ref` — native-vs-semantic owner question.** The confirmed withdrawals schema
  carries a `Transaction ID` column, but per the owner it is **not a stable per-row id** (the schema-only
  doc shows the column but not values; cf. Swan-trades' present-but-empty `Tag`). The adapter therefore
  treats withdrawals as **id-less** (synthesized `(source, direction, utc_ms, type, sat)` + occurrence_index,
  §6.2). If the withdrawals `Transaction ID` turns out to be stable/unique, switch to a native ref (one-line
  change). — OPEN (owner confirm). — adapters §9.1 / plan Schema-items.
- **Swan `Total/Transaction USD` purchase-cost semantics.** Swan transfers `purchase`→`Acquire` uses
  `Transaction USD` (principal) + `Fee USD` (fee), with `Total USD` as the basis cross-check (`Total ==
  Transaction + Fee`); confirm by fixture once real values are available. — OPEN (confirm). — adapters §9.1.
- **Coinbase internal-move default.** `Exchange/Pro Deposit/Withdrawal` (Coinbase↔Coinbase-Pro) are routed to
  `Unclassified` (likely self-transfers, but user-confirmed via reconciliation rather than auto-`TransferIn`/
  `TransferOut`). Confirm this conservative default is desired. — OPEN (owner confirm). — adapters §9.1.
- **XLSX-float→decimal precision bound; id-less `occurrence_index` file-order fragility** (River, Swan trades,
  Swan withdrawals, Gemini `Credit`/`Debit`) — both already noted; carry forward. **Pin** the resolved
  `csv`/`calamine`/`rust_xlsxwriter` versions + re-verify the `calamine::Data` variant list after first build.
  RESOLVED (versions pinned 2026-06-29): `csv` 1.4.0, `calamine` 0.26.1, `rust_xlsxwriter` 0.79.4.
  `calamine::Data` variant audit deferred to Task 2 (first build confirmed 0.26.1 resolves; no variant
  references in Task 0). — OPEN (Task 2 Data-variant audit). — plan Notes for Plan 4.
- **`AdapterError.source` field rename (thiserror compat, 2026-06-29).** The brief's `lib.rs` stub used
  `source: &'static str` (the adapter name) in `MissingColumn`/`Parse`/`FractionalSat` variants. Both
  thiserror 1.x and 2.x auto-treat any field named `source` as `Error::source()`, requiring `impl Error`.
  Field renamed to `adapter: &'static str`; format strings updated to `{adapter}`. Parse functions updated
  to construct with `adapter: source`. Display output unchanged. — RESOLVED (Task 0).

## Deferred to later phases (out of Phase-1 scope by design)
- **Forms generation (Phase 2):** filled IRS 8949 + Schedule D PDFs; §170(e) charitable-deduction computation (FMV vs basis); Form 8283 (>$5k qualified appraisal — §170(f)(11)(C), CCA 202302012); Form 709 routing for gifts. — *Phase 1 captures the metadata (FMV, ST/LT, appraisal-required, donor carryover) so Phase 2 can compute.* — OPEN (Phase 2). — tax-review N1/M-(donation), spec §16.
- **Rate/limit mechanics (Phase 2/3):** 0/15/20% (§1(h)), 3.8% NIIT (§1411), $3,000 loss limit + carryforward (§1211/§1212). — Confirmed safe to defer (downstream of per-lot basis/gain/ST-LT). — OPEN (Phase 2/3). — tax-review "Positions confirmed".
- **Self-employment tax routing (Phase 2):** business-vs-hobby mining → Schedule SE (Notice 2014-21 A-9). — *Phase-1 ledger tags `Income{Mining, business: bool}`; Phase 2 routes.* — OPEN. — tax-review N1.
- **Optimizer (Phase 3):** goal-driven specific-ID/HIFO/LIFO/loss-harvesting, bracket/NIIT-aware. — OPEN. — spec §16.
- **Non-BTC scope:** fork-coin income (e.g., 2017 BCH airdrop, RevRul 2019-24) and non-BTC dispositions are OUT of BTC-only scope and must be handled separately. — Acknowledged, not covered. — OPEN/won't-do-in-foundation. — tax-review M4.

## Deferred — precise Phase-2 tax refinements (Phase-1 over-approximates safely)
- **`appraisal-trigger-precision` — Qualified-appraisal trigger precision.** **[cycle-prep 2026-06-29 correction:** the earlier claim "Phase 1 flags `appraisal_required` on FMV>$5k (over-flag)" is FALSE — there is NO auto-computation; `appraisal_required` is a raw **user-supplied CLI boolean** on `reconcile reclassify-outflow … donate` (`main.rs` → `OutflowClass::Donate{appraisal_required}`). The earlier "§16" pointer is also wrong (§16 is the impl-order list).** The precise §170(f)(11)(C) trigger is a **claimed deduction > $5,000**, aggregating similar items in a year (§170(f)(11)(F)); for §170(e)-reduced property (≤1-yr / ordinary-income) the deduction is limited to **basis**, so a high-FMV short-term donation with basis ≤ $5k would not trigger an appraisal. Computing the exact trigger requires the *claimed-deduction* (= §170(e) deduction computation), which is itself Phase-2. **No Phase-1 action.** — OPEN (Phase 2; depends on deduction computation). — TP10, spec fold-record R3/TAX-N2.
- **§1015(d) gift-tax basis increase.** A donee's basis is bumped by gift tax paid attributable to net appreciation (§1015(d)). Rare for personal BTC gifts (mostly under the annual exclusion); omitted in Phase 1, noted for completeness. — OPEN (won't-do unless needed). — tax-review R3 N3; spec §15.

## btctax-store — whole-branch fix I-1 (owner-only perms) — deferred hardening
- **M-1: `open`/`recover_target` bak-on-corrupt.** `recover_target` restores from `.bak` only when the target is MISSING, not when it is present-but-corrupt. Consider retrying from `.bak` on decrypt/decode failure — but must NOT retry on `WrongPassphrase` (caller error, not corruption). Deferred hardening; overlaps the kill-mid-save fuzz-harness item. — OPEN. — I-1 fix follow-on.
- **M-2: save-path plaintext not zeroized.** The `db_to_bytes`/`encode_blob` `Vec`s produced during `save()` hold plaintext before encryption and are not zeroized on drop. Within the accepted R1 bound (SQLite heap already holds plaintext all session). Future: wrap in `SecretBuf`/zeroize after `encrypt_to`. — OPEN. — I-1 fix follow-on.
- **M-3: Windows owner-only perms — verify under CI.** All four sinks (`vault.key`, `vault.pgp`, `export_snapshot`, `backup_key`) now use the non-Unix ACL-inheritance path (no explicit DACL). Verify under Windows CI that the written files are not world-readable. — OPEN (CI). — I-1 fix follow-on.

## btctax-store (Plan 1) — deferred hardening (non-blocking; plan is review-green)
- **Password zeroization (Task-3).** Resolved: `sequoia-openpgp::crypto::Password` wraps `Encrypted`, which stores the plaintext in a `Protected` buffer. The `Protected` type implements `Drop` with `memsec::memzero` — the ciphertext (encrypted plaintext) IS zeroized on drop. The `salt` field in `Encrypted` is NOT explicitly zeroized, but it is a key-derivation salt, not the actual secret. Confirmed — Password zeroizes (Protected buffer). — RESOLVED (2026-06-28). — Task-3.
- **OS-crash mid-first-create residual.** A `kill -9`/power-loss between the `vault.key` write and the first `vault.pgp` rename leaves `vault.key` present + `vault.pgp`/`.bak` absent → `create`→`AlreadyExists`, `open`→`Io(NotFound)`; manual key deletion needed (no committed data lost). In-process failures are cleaned up. Add an OS-level kill-mid-save fuzz harness and/or treat "key present, pgp+bak absent" as a half-created vault to auto-repair. — OPEN. — plan-review R3 M2.
- **Lock file persists after a failed/`AlreadyExists` create** (lock-first; conventional flock pattern, lock files are never unlinked). Harmless. — WONTFIX/ack. — plan-review R3 N1.
- **Sequoia/S2K pin (R3) — CONFIRMED by Task-0 spike:** sequoia-openpgp `1.21` resolved to `1.22.0`; backend `crypto-nettle`. Spike confirmed secret-key S2K = `Iterated { hash: SHA256, hash_bytes: 65011712 }` (i.e. `0x3E00000`, max OpenPGP work factor, ~354 ms) — no Argon2 in 1.22, strongest available = high-work-factor iterated-salted SHA-256, satisfying spec §8. Both primary key and subkey carry this S2K. Revisit if a future Sequoia exposes Argon2 or a public S2K-work-factor setter. — RESOLVED/confirmed (2026-06-28). — plan-review R2/R3 + Task-0.
- **nettle 4.0 system incompatibility (CONCERN, non-blocking for now):** The dev machine has system nettle 4.0, but `nettle-sys-2.3.2` + `nettle-7.5.0` require nettle 3.x API (functions removed/renamed, SHA3 init symbols gone, digest callback arity changed). Build workaround: extracted cached `nettle-3.10.2-1.1-x86_64_v3.pkg.tar.zst` from pacman cache to `/tmp/nettle-3.10.2/`, set `PKG_CONFIG_PATH=/tmp/nettle-3.10.2/pkgconfig-custom LD_LIBRARY_PATH=/tmp/nettle-3.10.2/usr/lib` when running cargo. This workaround is session-local and NOT committed. Future task: either (a) wait for a new `nettle`/`nettle-sys` crate supporting nettle 4.0, (b) install nettle 3.x system-wide, or (c) switch to `crypto-rust` backend (pure Rust, no system lib dependency) for CI portability. Per task-0-brief, no silent backend switch; this is an explicit concern. — OPEN. — Task-0 report.
- **Two on-disk artifacts** (`vault.pgp` + `vault.key`) and the vault is **encrypted but not signed** — documented deviations from §8's single-artifact wording (NFR2 still holds; `vault.key` is S2K-encrypted). Sign-on-save is a future option. — ack. — plan-review R1 M2/M8.

## btctax-store — cross-platform + crypto-rust (user decisions 2026-06-28)
- **Target OS = Linux + macOS + Windows (NFR8).** Store crate abstracts OS primitives: single-instance lock via `fs2` (flock/LockFileEx); secret-memory lock via `rustix` mlock (Unix) / `windows-sys` VirtualLock (Windows); atomic save via `std::fs::rename` (POSIX atomic / Windows MoveFileEx-replace, with the fsync'd `.bak` as the safety net). Spec NFR8 + §8 + plan Tasks 0/4/5/6 updated. — RESOLVED (decision). — user OS choice.
- **Crypto backend = `crypto-rust` (pure Rust)** — supersedes the earlier `crypto-nettle` choice. Reasons: (a) the dev box's nettle 4.0 is incompatible with `nettle-sys` (the Task-0 hack is no longer needed/used); (b) NFR8 cross-platform (Windows can't use nettle). `crypto-rust` needs no system crypto lib on any OS. **Security trade-off accepted:** Sequoia labels RustCrypto variable-time / "not recommended for general use"; acceptable for local at-rest single-user encryption (no network/oracle exposure). `allow-variable-time-crypto` enabled. The Task-0 nettle-4.0 concern above is **SUPERSEDED** by this switch. — RESOLVED (decision). — user backend choice.
- **Cross-platform validation:** Linux is the dev/test OS; Windows/macOS code paths are `cfg`-gated and compile-checked but executed under per-OS CI (set up later). — OPEN (CI). — NFR8.
- **crypto-rust builds clean (no system crypto lib, nettle hack unused):** `cargo build -p btctax-store` + `cargo test --test smoke` pass with `features = ["crypto-rust", "allow-variable-time-crypto", "allow-experimental-crypto"]` and no `PKG_CONFIG_PATH`/`LD_LIBRARY_PATH` set; S2K = `Iterated{SHA256, hash_bytes=65011712}` confirmed unchanged under crypto-rust. `allow-experimental-crypto` is required (sequoia-openpgp build script gates RustCrypto behind it). — RESOLVED (2026-06-28). — Task-0 crypto-rust switch.
- **File-lock crate: `fs2` 0.4 (dormant ~2017) vs `fd-lock` (maintained).** We use `fs2::try_lock_exclusive`; on Windows it relies on Rust ≥1.64 mapping `ERROR_LOCK_VIOLATION(33)`→`WouldBlock` (MSRV 1.74 satisfies). `fd-lock 2.x` normalizes this explicitly and is maintained — preferred swap if Windows CI shows any mapping issue or if the dormant dep becomes a supply-chain concern. — OPEN (monitor; swap candidate). — plan-review delta M-1.

## btctax-core (Plan 2) — review-green; deferred Minors to address at implementation
- **TP8(c) loss-basis cross-lot edge (tax m1).** When a fee spans lots and `relocated.last()`/last removal-leg is non-dual-basis but the fee originates on a dual-basis received-gift lot, the carry's `loss_basis` fragment is dropped. Effect: future loss-zone basis understated by fee-cents (taxpayer-conservative); gain basis fully conserved. — OPEN (Task 11). — core tax-review R2 m1.
- **TP8 fee exact-boundary holding-period attribution (tax m2).** When principal consumes exactly to a lot boundary, the fee basis (from the next, later-acquired lot) rides the earlier relocated lot's holding period by a few cents. De-minimis; total basis conserved. — OPEN (Task 11). — core tax-review R2 m2.
- **Degenerate `principal==0` fee'd transfer (tax m3).** Carry is silently dropped (no relocated lot/leg) with no blocker — unreachable for real TransferLink/gift (principal>0). At implementation: assert principal>0 or raise `uncovered_disposal` instead of dropping. — OPEN (Task 11). — core tax-review R2 m3.
- **2025-transition seed timezone straddle (eng Minor).** The boundary seed fires on the UTC-sorted timeline while pool routing + `universal_snapshot` use the tax-date; a sub-day offset straddling 2025-01-01 (e.g. a +12:00 post-2025 event vs a −05:00 pre-2025 event) can fold a pre-2025-tax-date event after the seed → fails safe (`uncovered_disposal` or stranded lot), but `universal_snapshot` won't match the real fold's pre-seed residue. At implementation (Task 12): partition the timeline at the **tax-date** boundary (or seed lazily on first wallet route) + add a reversed-offset KAT. — OPEN (Task 12). — core eng-review R2 Minor.
- **`allocation_voids` declaration (eng Nit).** Referenced (pass-1 step 1a, deferred from Task 7) with `.target`/`.void_id` but its struct/collection isn't formally declared in the plan; declare it explicitly at implementation. — OPEN (Task 7/12). — core eng-review R2 Nit.

## Standing notes / decisions (informational)
- **PGP KDF tradeoff (user-mandated PGP retained).** Engineering review suggested age / XChaCha20-Poly1305+Argon2id as simpler with a stronger KDF; **declined — PGP is a hard user requirement.** Mitigation: protect the app-managed private key with the strongest S2K the chosen Sequoia version supports (Argon2 S2K if available, else high-work-factor iterated-salted S2K). — RESOLVED (decision) / monitor. — eng-review YAGNI, spec §8/§15.
- **TP8 self-transfer fee = treatment (c) default, config-switchable to (b) mini-disposition.** User-mandated default; do not flip. Contrary signal: §1.1012-1(h)(2)/(h)(4) (fees-in-crypto have disposition consequences in the *taxable-exchange* context; no on-point guidance for a pure self-transfer). — RESOLVED (decision). — spec TP8, memory `self-transfer-fee-treatment-c`.
- **Daily-close FMV is an approximation** of the "date and time of dominion & control" standard (RevRul 2023-14). Documented convention; revisit if higher precision is needed. — RESOLVED (decision) / monitor. — spec §9.2, tax-review M3.
- **`pre2025-filed-method-reconciliation` — Pre-2025 lot method = FIFO (legal default).** **[cycle-prep 2026-06-29 correction:** the advisory note ALREADY EXISTS — `BlockerKind::Pre2025MethodNote` (state.rs, Advisory severity) is emitted by `note_pre2025_once` (fold.rs) on any pre-2025 disposal, and `verify` already surfaces it. The earlier text implied it was unimplemented.** The Phase-1 advisory ("FIFO assumed; reconcile if your filed pre-2025 returns used a different method") is **DONE**. What is genuinely OPEN is a *runtime reconciliation mechanism* (taxpayer declares the filed method → engine adjusts the reconstructed carryforward basis), which does not exist and is a Phase-2 feature (needs a brainstorm to scope: method-declaration config + basis adjustment). — note DONE / reconciliation mechanism OPEN (Phase 2). — spec §7.4, eng-review I-2.
- **Source-priority tiebreak (Swan>Coinbase>Gemini>River)** is arbitrary-but-stable for same-instant cross-source FIFO ties; documented as such. — RESOLVED (decision). — spec §6.2, eng-review n-2.
- **Id-less-source `source_ref` fragility (River).** For sources without native ids, `source_ref = (source, direction, utc_ms, type, sat)` with a last-resort `occurrence_index` for exact duplicates in one import. Two known limitations: (a) `occurrence_index` shifts if a corrected re-export inserts an earlier row; (b) a re-export that edits a *constituent* field (e.g., `sat`) changes the `source_ref`, so it is NOT detected as a "same source_ref, changed content" conflict and cannot be auto-`SupersedeImport`-ed (old event orphans, new appears). — OPEN (documented limitation; prefer time-resolution / native ids where possible). — spec §6.2, eng-review round-2 m-2/m-5.
- **Daily-close FMV (labeled M3)** — see the "Daily-close FMV is an approximation" note above; flagged as the chosen convention vs the date-and-time dominion-and-control standard. — RESOLVED (decision). — tax-review M3.

## Resolved in SPEC v0.2 (folded round-1 reviews)
See the spec's "Fold record (v0.2)" section for the 1:1 mapping of each Critical/Important to its fix. Round-1 reviews: `reviews/spec-review-phase1-tax-round-1.md`, `reviews/spec-review-phase1-engineering-round-1.md`, `reviews/architecture-review-phase1-foundation-round-1.md`.

- **N-2 (export_snapshot silently overwrites snapshot.sqlite):** Current behaviour matches the brief (no mention of rotation); future improvement: timestamped filenames (e.g. `snapshot-20260628T120000Z.sqlite`) to avoid clobbering a previous export. **Windows owner-only perms** for both `export_snapshot` and `backup_key` rely on user-profile directory ACL inheritance (no explicit DACL set); verify under Windows CI that the written files are not world-readable.

## btctax-adapters plan — deferred Minors (review-green; 2026-06-29)

Non-blocking items raised during the round-1 review of `btctax-adapters` (IP-1 and all code-level Minors folded inline into the plan on 2026-06-29). These are deferred observations for implementation time or later phases.

- **River `Income`→`IncomeKind::Reward` documentation + `business: false` immutability (tax M1/M2).** River's `Income` tag maps to `IncomeKind::Reward` (non-business yield/reward); `business: false` is hard-coded at ingest. At implementation, add a module-doc note that `business: false` is immutable at the adapter layer — the Plan-4 reconciler cannot flip it without a re-import. If the owner's River income is business income (e.g., from professional mining operations), the `IncomeKind` / `business` mapping must be confirmed before implementing the River parser. — OPEN (confirm at River-parser implementation). — adapters tax-review M1/M2.
- **Swan zero-sat-withdrawal defensive counter (tax Nit).** The Swan withdrawals arm currently increments `dropped_no_btc` for a `sat == 0` row (defensive guard; Swan is BTC-only). At implementation, consider whether a zero-sat Swan withdrawal should be counted under a separate `skipped_zero_sat` field rather than the FR2 `dropped_no_btc` counter, since the two cases are semantically different. — OPEN (implementation note). — adapters tax-review Nit.
- **Coinbase internal-move = Unclassified decision (tax-review endorsed).** `Order` + `Exchange/Pro Deposit/Withdrawal` → `Unclassified` is the correct conservative default. The tax reviewer explicitly endorsed keeping this (over auto-routing to `TransferIn`/`TransferOut`), since these Coinbase↔Coinbase-Pro internal moves require user confirmation via reconciliation. No change to the plan; noted here so Plan-4 docs know the decision is reviewed and intentional. — RESOLVED (decision retained; no action needed). — adapters tax-review.
- **Swan withdrawals `Transaction ID` stability — treated id-less; confirm later.** The withdrawals file carries a `Transaction ID` column but the adapter treats it as non-stable (semantic `source_ref`). If confirmed stable/unique, switch to native ref (one-line change in `Swan::normalize` withdrawals arm). Cross-referenced with the existing schema-items entry above. — OPEN (owner confirm). — adapters plan Schema-items / tax-review Nit.

## btctax-core (Task 0) — dependency versions pinned for reproducibility
- btctax-core pinned `rust_decimal` 1.42.1 / `rust_decimal_macros` 1.40.0 (independent Cargo entries; `dec!` literals binary-compatible with the 1.42 `Decimal`) / `time` 0.3.51 — R3 pin record.

## btctax-cli plan (Plan 4) — deferred items from round-1 reviews (2026-06-29)

Non-blocking items raised in the round-1 reviews of `IMPLEMENTATION_PLAN_foundation_04_cli.md`
(`reviews/plan-foundation-04-cli-engineering-round-1.md`,
`reviews/plan-foundation-04-cli-reconciliation-round-1.md`). The blocking findings (C1, I-1, I-2/Eng-I1,
M3, N-2) were folded into the plan (see its "Fold record (round 1)"). These remain open.

- **M-2 (recon) — `AllocLot` carries no `dual_loss_basis` → a pre-2025 received-GIFT lot loses its
  §1015(a) dual basis under Path B.** A safe-harbor `SafeHarborAllocation.lots` entry is
  `{wallet, sat, usd_basis, acquired_at}` — single-basis. So when a pre-2025 gift lot (which under TP11
  carries a separate loss-basis = donor basis vs gain-basis = FMV-at-gift) is re-seeded via Path B, the
  loss-leg basis collapses to the single `usd_basis`. This is **spec-faithful** (the spec defines
  `AllocLot` without a dual-basis field), and Path A (the default) preserves the dual basis correctly, so
  the loss only arises when a taxpayer *elects* Path B over a gift lot. Effect: a future loss-zone
  disposition of that seeded lot could mis-state basis. **Phase-2 refinement:** extend `AllocLot` (and the
  Path-B seed in `transition::seed_transition`) to carry `dual_loss_basis` + `donor_acquired_at`. — OPEN
  (Phase 2; spec change required). — recon review M-2.

- **M-1 (recon) / M1 (eng) — `verify` double-loads events.** — **RESOLVED (burndown 2026-06-29, commit 39e09e0):**
  added `Session::load_events_and_project()`; `verify` + `safe_harbor_attest` routed through it. See the
  burndown section above.

- **eng-M2 — render + CSV use `{:?}` (Debug) for enums.** — **RESOLVED (burndown 2026-06-29, commit 282ae20):**
  six stable snake_case `*_tag()` fns; all CSV writers + text renderers switched off `{:?}`; export test
  asserts column values. CSV columns are now a committed contract. See the burndown section above.

- **N-1 (recon) — strengthen the `set-fmv` test.** — **RESOLVED (burndown 2026-06-29, commit 4a78727):**
  repointed to an FMV-missing `Income` target; asserts the `FmvMissing` blocker present before and cleared
  after `set-fmv` (+ income recognized at the manual FMV). See the burndown section above.

- **attest leaves a stale `safe_harbor_timebar` advisory (follow-on of the I-2 fold).** — **RESOLVED**
  (the CLI-I2 whole-branch fix made `safe_harbor_status` prefer the effective-Path-B signal over the advisory;
  the burndown fix (commit f6880e6) extended that signal to disposal/removal legs for the all-lots-consumed
  case). `verify` no longer mislabels an effective Path B as time-barred. See the burndown section above.

## Sub-project A (lot-id substrate) — whole-diff review deferrals (2026-06-29, round 2 residuals)
- **N2 — `evaluate_disposal` `lots_after` semantics for C.** Confirm the returned post-disposal lots/outcome shape is what Sub-project C (optimizer + Mode-2) needs before C consumes it. — OPEN (C planning).
- **N3 — B per-year hard-blocker gate.** B must refuse a TaxResult / C must refuse to optimize for a tax year with unresolved Hard blockers (basis-pending/uncovered/LotSelectionInvalid/etc.). — OPEN (B planning).
- **M3 binary-dispatch test.** The `config` multi-flag apply-all + attest-guard are tested at library level, not by driving the real clap `Command::Config` arm; add a binary-level dispatch test to fully retire the Task-5 note. — OPEN (B/C or a CLI test pass).

## Sub-project B (rate/NIIT/loss engine) — whole-diff review deferrals (2026-06-30)
- **F1 (Nit) — money "0" vs "0.00" display.** Load-bearing figures (ltcg_tax/niit/total) are round_cents-scaled and always print cents; descriptive level fields inherit source scale → cosmetic inconsistency. Add a `fmt_money` (`{:.2}`) render helper. — OPEN (polish).
- **Minor — `MarginalRates.niit_applies` doc vs code.** Doc says "MAGI exceeds threshold"; code computes "crypto increased NIIT" (niit_with>niit_without). Display-only, feeds no figure. Align doc or rename. — OPEN.
- **B-M1 (Phase-2) — minimal NII model can understate NIIT** in loss years (NII excludes crypto ordinary income + not reduced by §1211 loss). Disclosed in output. Phase-2 refinement. — OPEN.
- **Nits (DEFER):** unused `events` param in compute_tax_year; redundant rust_decimal_macros dev-dep (adapters); `{:?}` filing_status in tax-profile --show; advisory-only→Computed KAT; B-R2-N1 stale §4.3 doc line. — OPEN (cosmetic/doc).

## Sub-project C (optimizer) — Task-4 review Nit deferred (2026-06-30)

- **Nit — `proposed_compliance_status` / `persistability` asymmetry for divergent contemporaneous 2027+
  broker picks.** `proposed_compliance_status` returns `NonCompliant` for a selection that diverges from the
  current pick AND was made at/before the sale date (`made ≤ sale`, i.e. contemporaneous) when the wallet is a
  2027+ broker-held account. `persistability` returns `ContemporaneousNow` for the same inputs (made ≤ sale
  is the only criterion for `persistability`; the 2027+ broker check is only in `ForbiddenBroker2027`). This
  means the status says "NonCompliant" while the persistability gate says "persists freely" — an unusual
  combination that a caller would see only for a future-dated existing disposal to a 2027+ broker where the
  optimizer proposes a pick that differs from the current selection. In practice, the CLI's Task-10
  2027+ broker refusal prevents this path from being reached (the CLI refuses to persist any divergent pick
  for 2027+ brokers regardless of persistability). A one-line alignment (either widen
  `proposed_compliance_status` to return `NonCompliant` from `persistability == ForbiddenBroker2027` even
  for contemporaneous picks, OR add a `ForbiddenBroker2027` arm to `Persistability` and let the CLI check
  that instead of `persistability == ContemporaneousNow`) would remove the conceptual gap. — **RESOLVED
  (whole-diff-review fold, 2026-06-30):** `persistability` now tests the 2027+ broker envelope FIRST, ahead
  of the `made ≤ sale` contemporaneous branch, so a 2027+ broker lot is categorically `ForbiddenBroker2027`
  (never `ContemporaneousNow`) regardless of timing — matching `proposed_compliance_status` (which already
  returned `NonCompliant` ahead of the contemporaneous branch). Both core functions now agree, and `accept`'s
  gate categorically refuses these even when `made ≤ sale` (no own-books-insufficient 2027+ broker record can
  persist). Covered by `persistability_broker_2027_contemporaneous_is_forbidden`,
  `persistability_broker_pre_2027_contemporaneous` (regression), and the end-to-end
  `accept_refuses_2027_broker_contemporaneous_divergent_no_write` (synthetic TY2027 table; fails without the
  fix). `crates/btctax-core/src/optimize.rs` (`persistability`).

## Sub-project C (optimizer) — whole-branch review round 1 deferrals (2026-06-30)

Source: `reviews/whole-branch-review-optimizer-round-1.md` (VERDICT: READY TO MERGE — 0 Critical / 0
Important). The review's one MUST-FIX-before-TY2027-table item (the `persistability`/`proposed_compliance_status`
2027+ broker asymmetry) was folded this cycle (see the Task-4 nit above, now RESOLVED). The remaining three
new Minors are non-blocking and deferred here.

- **M-1 (Minor) — exact-tie tie-break can emit a `delta == 0` divergent pick.** In `exhaustive_min`
  (`crates/btctax-core/src/optimize.rs`, the `total == best_total && assign < best_assign` branch) a candidate
  that TIES the baseline total but is lexicographically smaller than `baseline_assignment` evicts the baseline
  incumbent (`best_total` stays `== base.total`). Result: `best != baseline_assignment` with `delta == 0`, so a
  disposal with two equal-basis/equal-term lots can yield `proposed != current` at zero tax benefit → `run`
  shows a "change … needs `--attest`" line for no benefit, and a future-dated (`made ≤ sale`) disposal would let
  a bare `accept` auto-persist a no-benefit divergent `LotSelection`. **No invariant is broken** (`delta = 0` is
  shown, the pick is gated/legally valid, the reported optimum is still a true minimum) — it is needless churn /
  a pointless attestation prompt. The lex-smallest tie-break is the spec'd §0 total order, so this is a quality
  choice, not a correctness bug. *Recommend* preferring the baseline on an exact tie (evict only on
  `total < best_total`). — OPEN (non-blocking polish).

- **M-2 (Minor) — Mode-2 `consult_sale` discards the `candidate_selections` heuristic flag.**
  `crates/btctax-core/src/optimize.rs` binds `let (cands, _heuristic) = candidate_selections(&lots, req.sell_sat)`.
  For a wallet pool > `LOT_ENUM_BOUND` (12) — common for weekly-DCA / active-trading wallets — the candidate set
  is a deterministic INCOMPLETE subset, so the proposed selection may not be the true tax-minimum, with NO
  disclosure (unlike Mode-1's `PoolHeuristic` banner). Mitigation: `ConsultReport` has no `approximate` field and
  the renderer hedges ("read-only what-if", "proposed selection", "federal tax attributable (estimated)") rather
  than claiming "the optimum" — so it is NOT a false-global claim (hence Minor). The plan scoped R2-C1's
  disclosure to Mode-1. *Recommend* a parallel "heuristic — searched a subset of a large pool" note in
  `render_consult` for symmetry. — OPEN (non-blocking; add a consult-level approximate disclosure later).

- **M-3 (Minor) — the optimizer's "global" excludes self-transfer lot-selection; scope undocumented.**
  `optimize_year` (`crates/btctax-core/src/optimize.rs`) targets only `baseline_state.disposals`; SelfTransfers
  produce no Disposal/Removal record, so a same-year self-transfer's lot routing is held at its baseline. Spec
  §A.3 lists SelfTransfer as method-honoring and says it "lets the optimizer pre-position lots," so a user could
  read "proven global minimum" (`approximate == false`) as including self-transfer re-routing. In practice the
  available-lots pools are still correct (the real fold, incl. self-transfers at baseline, is replayed), and
  self-transfers are non-taxable so they affect the single-year objective only indirectly via an uncommon
  intra-year move-then-sell pattern — hence Minor. The `approximate == false` "global" claim is global over
  taxable-disposal selections only. *Recommend* documenting the scope boundary in the proposal footer (mirroring
  the R0-M2 vertex-granularity caveat); relates to A's open `disposal_compliance`-omits-SelfTransfers item. —
  OPEN (non-blocking; document the scope boundary vs spec §A.3).
