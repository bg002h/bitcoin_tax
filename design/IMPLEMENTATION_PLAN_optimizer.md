# IMPLEMENTATION PLAN — Sub-project C: Rate-Aware Optimizer

**Program:** Lot-Identification & Tax-Optimization (Phase-2). **Sub-project:** C (rate-aware optimizer — the **final** sub-project; built after A and B; A → B → C).
**Source of truth:** `design/SPEC_lot_optimization_program.md` (R0-GREEN 2026-06-29, folds R0 rounds 1–2). **Sub-project C (§C.1–C.5)** + the **Cross-cutting** section + **Legal grounding** are **binding**.
**Predecessors (SHIPPED on main):**
- **A (lot-id substrate):** `LotSelection`/`LotPick` decisions, `evaluate_disposal`/`CandidateDisposal`/`EvaluateOutcome`/`EvaluateError`, `disposal_compliance`/`ComplianceStatus` (incl. the reserved `AttestedRecording` variant), the per-wallet `consume(method, selection)` path with `LotSelectionInvalid`, `MethodElection`, `Pre2025MethodConflictsAllocation`.
- **B (rate engine):** `compute_tax_year -> TaxOutcome{Computed(TaxResult)|NotComputable(Blocker)}`, `TaxProfile`, `Carryforward`, `MarginalRates`, `BundledTaxTables`, the `report --tax-year` surface, the `tax_profile` side-table.

**Status:** DRAFT — **R0 rounds 1–2 folded** (see "Fold record (R0 round 1)" and "Fold record (R0 round 2)" at the end: round 1 = 3 Critical + 4 Important + 5 Minor + 2 Nit; round 2 = 1 Critical + 1 Important + 2 Minor residuals — the two sibling-path residuals of C1/C2). **PENDING RE-REVIEW (round 3)** per `STANDARD_WORKFLOW.md §2` ("re-review after every fold, including the last") — must reach **0 Critical / 0 Important before any code**. Then executed subagent-driven (one implementer carrying the whole plan).

> **How to execute (per `STANDARD_WORKFLOW.md`).** Each numbered task is a TDD phase. Per task: (1) write the failing test(s) — real test code; (2) run → confirm **RED**; (3) minimal real implementation; (4) run → confirm **GREEN**; (5) run the **whole** validation surface (`cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`); (6) independent review loop → 0 Critical / 0 Important (persist each reviewer's output verbatim **before** folding; re-review after every fold, including the last); (7) commit. **Gates are hard.** Ceremony scales *down* for small tasks; it is never removed. The closing **Task 12** is the mandatory whole-diff review (Phase E).

---

## 0. Global Constraints (apply to EVERY task — a violation is a blocking finding)

- **NFR4 — the optimizer MUST be deterministic.** Identical `(events, prices, config, year, profile, tables, proposal_made, attested)` (and, for consult, the identical `ConsultRequest`) → **byte-identical** `OptimizeProposal` / `ConsultReport` and an identical persisted `LotSelection` set on `accept`. (`proposal_made` and `attested` are in the tuple because the proposal's per-disposal `status`/`persistable` depend on them — R2-M2; the optimization **core** — picks/`optimized_tax`/`delta` — does NOT, which is correct.) Every map is a `BTreeMap`/`BTreeSet`; every candidate list / cartesian-product iteration is over a **pre-sorted `Vec`**; **no `HashMap` iteration** anywhere. Any tie (equal `total_federal_tax_attributable`) is broken by a **total order** — the lexicographically smallest assignment by `(disposal EventId, sorted LotPick list)`. **No `Date::now`/RNG in `btctax-core`** (the consult "today/at" date is passed in from the CLI seam, never read in core).
- **NFR5 — exact arithmetic / no float.** Money is `Usd = Decimal` (`crates/btctax-core/src/conventions.rs:8`); sats are `Sat = i64` (`conventions.rs:6`). The objective and every comparison use `Decimal`/`i64` only — **no `f32`/`f64` anywhere**, including the search/scoring loop, the tie-break, and the timing-insight delta. All tax dollars come straight from B's `round_cents` (ROUND_HALF_EVEN) `TaxResult` fields; C never re-rounds.
- **Federal only.** State tax is out of scope (app charter). C optimizes B's federal `total_federal_tax_attributable` only.
- **The §1.1012-1(j) compliance boundary is load-bearing, not cosmetic.** Adequate identification must exist **by the time of sale** in **every** year. **There is no compliant post-hoc selection.** No artifact, command, output string, or doc comment introduced by C may describe a post-hoc selection as compliant. `optimize` is **what-if by default**; persistence is narrowly gated (Task 9). Self-custody = own-books base reg (all years, no relief). Broker-held = own-books relief **through 2026-12-31 only**; **2027+ broker-held can NEVER be persisted by C** (own-books is then insufficient; the only compliant lever is a broker-side standing order the app cannot manufacture).
- **Event-sourcing boundary.** **Mode-1 produces `LotSelection` decisions** (gated; reuse A's event — **no new `EventPayload` variant**). **Mode-2 produces NOTHING** (read-only; no events, no side-table writes, no mutation of any kind). The new **attestation record is a CLI side-table** (a projection input, like `cli_config`/`tax_profile` — `crates/btctax-cli/src/config.rs:1-4`, `tax_profile.rs:1-5`), **not** ledger state and **not** a new event type.
- **Refuse on unsound data (I6).** C **refuses to optimize / consult** any year B reports as `TaxOutcome::NotComputable` (any `Severity::Hard` blocker anywhere — `BlockerKind::severity()`, `state.rs:56-76` — or missing profile/table). It never returns a "best" selection over a year whose baseline is not computable.
- **§1091 wash sale.** Crypto is currently exempt (not a "stock or security"; no enacted statute — Greenbook proposals only). The optimizer may **freely select loss lots** (loss harvesting is unconstrained). Documented + a monitoring note (Task 7). *Monitor for enactment.*
- **Privacy.** Tests use **synthetic fixtures + temp vaults only** (`tempfile`); no real reads, no PII. Bundled tax tables are public reference data.
- **Citations.** Re-verify every `file:line` in §1 against **current source at task write time** (`STANDARD_WORKFLOW.md §4`); line numbers decay every merge.

---

## 1. Source grounding (re-verified against CURRENT source at write time, 2026-06-30)

| Symbol / fact (consumed by C) | Current location |
|---|---|
| `Usd = Decimal`, `Sat = i64`, `TaxDate = Date`; `round_cents`; `one_year_after`; `is_long_term`; `TRANSITION_DATE = 2025-01-01` | `crates/btctax-core/src/conventions.rs:6,8,10,22-24,57-62,65-67,17` |
| `evaluate_disposal(events, prices, config, candidate, selection) -> Result<EvaluateOutcome, EvaluateError>` | `crates/btctax-core/src/project/evaluate.rs:98-213` |
| `CandidateDisposal { existing_event: Option<EventId>, wallet: WalletId, date: TaxDate, sat: Sat, kind: DisposeKind, proceeds: Option<Usd> }` | `evaluate.rs:31-43` |
| `EvaluateOutcome { legs: Vec<DisposalLeg>, st_gain, lt_gain, lots_after: Vec<Lot>, blockers }` | `evaluate.rs:46-60` |
| `EvaluateError { ProceedsRequired, UnknownExistingDisposal }` | `evaluate.rs:63-70` |
| synthetic-append pattern (clone `resolve`, push `Eff{ Op::Dispose }`, inject `res.selections`, `fold`, discard) | `evaluate.rs:105-179` |
| `resolve(events, prices, config) -> Resolution` (pub); `Resolution.timeline: Vec<Eff>`, `Resolution.selections: BTreeMap<EventId, Vec<LotPick>>` (pub) | `crates/btctax-core/src/project/resolve.rs:128-136` ; `resolve` is `pub fn` |
| `Eff { id, utc, tz, src_priority, src_ref, wallet, op }`, `Eff::date()`, `Op::Dispose{ sat, proceeds, fee_usd, fee_sat, kind }` | `resolve.rs:82-96,14-24` |
| `fold(res, prices, config) -> LedgerState` (**pub**) | `crates/btctax-core/src/project/fold.rs:330` |
| `consume_principal` maps a `selection_error` → hard `BlockerKind::LotSelectionInvalid` (so an infeasible injected selection self-eliminates) | `fold.rs:51-66` |
| `PoolKey{Universal, Wallet(WalletId)}`, `pool_key(date, wallet)`; HIFO/FIFO/LIFO `consume`; `method_order`/`hifo_cmp` | `crates/btctax-core/src/project/pools.rs:10-21,71-102,248-285` |
| `LotPick { lot: LotId, sat: Sat }` (serde) | `crates/btctax-core/src/event.rs:190-194` |
| `LotSelection { disposal_event: EventId, lots: Vec<LotPick> }` (serde; reused — no new event type) | `event.rs:214-218,243` |
| `LotId { origin_event_id: EventId, split_sequence: u32 }` (`Ord`) | `crates/btctax-core/src/identity.rs:116-120` |
| `WalletId::Exchange{provider,account}` = broker; `WalletId::SelfCustody{label}` = self-custody (R2-M5 custody map) | `identity.rs:110-113` |
| `Lot { lot_id, wallet, acquired_at, original_sat, remaining_sat, usd_basis, basis_source, dual_loss_basis, donor_acquired_at, basis_pending }`; `gain_hp_start()`/`loss_hp_start()` | `crates/btctax-core/src/state.rs:84-106` |
| `Disposal { event, kind, disposed_at, legs, fee_mini_disposition }`; `DisposalLeg { lot_id, sat, proceeds, basis, gain, term, basis_source, gift_zone }`; `Term{ShortTerm,LongTerm}` | `state.rs:119-128,108-118,6-10` |
| `compute_tax_year(events, state, year, profile, tables) -> TaxOutcome` | `crates/btctax-core/src/tax/compute.rs:221-377` |
| `TaxOutcome{Computed(TaxResult), NotComputable(Blocker)}`; `TaxResult{ st_net, lt_net, ordinary_from_crypto, ltcg_tax, niit, loss_deduction, carryforward_out, total_federal_tax_attributable, marginal_rates }`; `MarginalRates` | `crates/btctax-core/src/tax/types.rs:92-96,65-87,54-59` |
| `Blocker { kind, event: Option<EventId>, detail }`; `BlockerKind::severity()` Hard set | `state.rs:77-82,56-76` |
| `disposal_compliance(events, state) -> Vec<DisposalCompliance>`; `ComplianceStatus{StandingOrder{effective_from}, Contemporaneous, AttestedRecording, NonCompliant}`; `DisposalCompliance{ disposal, wallet, date, status }` | `crates/btctax-core/src/project/compliance.rs:18-33,76-206` |
| `ProjectionConfig{ self_transfer_fee, pre2025_method }`; `project`; `PriceProvider`; `fmv_of` | `crates/btctax-core/src/project/mod.rs:31-56`; `price.rs` |
| core `lib.rs` re-exports (`pub use project::{…}`, `pub use tax::{…}`, `pub use state::*`) | `crates/btctax-core/src/lib.rs:12-26` |
| **Side-table pattern** (`init_table`/`get`/`set`/`all`, idempotent DDL, `BadConfigValue`) | `crates/btctax-cli/src/tax_profile.rs:16-81` |
| `Session::{open, conn, save, from_fresh_vault, project, load_events_and_project, tax_profile, config}` | `crates/btctax-cli/src/session.rs:28-112` |
| `append_decision(conn, payload, utc, tz, wallet) -> Result<EventId, CoreError>` (monotonic `decision_seq`); the reconcile `append_and_save` wrapper; `select_lots`/`import_selections` | `crates/btctax-core/src/persistence.rs:238-262`; `crates/btctax-cli/src/cmd/reconcile.rs:22-30,267-289` |
| `eventref::{parse_event_id, parse_wallet_id, parse_usd_arg, parse_date_arg, parse_lot_pick}`; `render::wallet_label` | `crates/btctax-cli/src/eventref.rs:24-120`; `render.rs:107-112` |
| clap `Command`/`Reconcile` enums + dispatch; `now = OffsetDateTime::now_utc()` seam in `main` | `crates/btctax-cli/src/main.rs:25-206,266-294,494-612` |
| `cmd/mod.rs` module list (add `pub mod optimize;`) | `crates/btctax-cli/src/cmd/mod.rs:1-7` |
| `CliError{ Store, Core, Adapter, Sqlite, Csv, Io, BadEventRef, Usage, BadConfigValue{key,value} }` | `crates/btctax-cli/src/lib.rs:16-41` |
| CLI integration-test harness (call `cmd::`/`render::` directly over a temp vault + synthetic CSV; `Passphrase`) | `crates/btctax-cli/tests/tax_report.rs:14-60` |
| `BundledTaxTables::load()` (TY2025 only; later years → `TaxTableMissing`) | `crates/btctax-adapters/src/tax_tables.rs:48-54` |

> **Naming adaptation (noted once).** The spec writes C's outputs inline ("a proposal/what-if report type carrying per-disposal compliance status + delta"). The codebase convention is a **named struct per concept** with `snake_case` fields. This plan therefore defines `OptimizeProposal`, `DisposalProposal`, `Persistability`, `ConsultRequest`, `ConsultReport`, `TimingInsight`, and `OptimizeError`, and **reuses** `LotPick`, `ComplianceStatus`, `TaxResult`, `MarginalRates` verbatim from A/B.

---

## 2. New public API surface introduced by Sub-project C

### Core (`btctax-core`, new top-level module `optimize`)
- `pub fn score_assignment(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig, year: i32, profile: Option<&TaxProfile>, tables: &dyn TaxTables, assignment: &BTreeMap<EventId, Vec<LotPick>>) -> TaxOutcome` — the holistic scorer: inject the per-disposal selections, **fold once**, run `compute_tax_year`. (Pub for reuse + KATs.)
- `pub fn optimize_year(events, prices, config, year: i32, profile: Option<&TaxProfile>, tables, attested: &BTreeSet<EventId>, proposal_made: TaxDate) -> Result<OptimizeProposal, OptimizeError>` — **Mode 1** (`proposal_made` = the proposed picks' made-date from the CLI seam; drives honest compliance/persistability — R0-C2).
- `pub fn consult_sale(events, prices, config, year_profile: Option<&TaxProfile>, tables, req: &ConsultRequest) -> Result<ConsultReport, OptimizeError>` — **Mode 2** (read-only; the CLI loads the year's profile and passes it so core stays clock-free).
- Types:
  - `OptimizeProposal { year: i32, baseline_tax: Usd, optimized_tax: Usd, delta: Usd, per_disposal: Vec<DisposalProposal>, marginal_rates: MarginalRates, approximate: bool, approx_reason: Option<ApproxReason> }` (`delta = optimized − baseline ≤ 0` always — baseline-seeded; `approximate`/`approx_reason` disclose a local/under-enumerated result — R0-C1/C3).
  - `ApproxReason { ComboCapExceeded { combos, cap }, ContentionUnenumerated { contended, combos, cap } }` — why a result is not the proven global minimum (R0-C1/C3).
  - `DisposalProposal { disposal: EventId, wallet: WalletId, date: TaxDate, current_selection: Vec<LotPick>, proposed_selection: Vec<LotPick>, status: ComplianceStatus, persistable: Persistability }`.
  - `Persistability { ContemporaneousNow, NeedsAttestation, ForbiddenBroker2027 }` — the `accept` gate verdict (computed in core, enforced in the CLI).
  - `ConsultRequest { sell_sat: Sat, wallet: WalletId, at: TaxDate, proceeds: Option<Usd>, kind: DisposeKind }`.
  - `ConsultReport { req: ConsultRequest, proposed_selection: Vec<LotPick>, st_gain: Usd, lt_gain: Usd, total_federal_tax_attributable: Usd, timing: Option<TimingInsight> }`.
  - `TimingInsight { st_sat_in_selection: Sat, latest_crossover: TaxDate, tax_if_sold_long_term: Usd, saving_if_waited: Usd }` (`saving_if_waited = total_now − tax_if_sold_long_term ≥ 0`).
  - `OptimizeError { YearNotComputable(Blocker), Evaluate(EvaluateError), NoDisposals, NoLots, PreTransitionYear(i32) }`.
- Pure compliance/persistability overlay (Task 5): `pub fn compliance_overlay(base: &[DisposalCompliance], attested: &BTreeSet<EventId>, unchanged: &BTreeSet<EventId>) -> Vec<DisposalCompliance>` (R2-I1 — `unchanged` = disposals whose proposed pick equals the in-force persisted-and-attested selection; the `AttestedRecording` upgrade is gated on it so an attestation binds only the exact attested selection, never a divergent re-run pick), `pub fn persistability(wallet: &WalletId, sale_date: TaxDate, selection_made: TaxDate) -> Persistability`, and **`pub fn proposed_compliance_status(wallet, sale_date, made, proposed, current, baseline_status) -> ComplianceStatus`** (R0-C2 — judges a proposed pick by its own made-date; never lets a standing order rescue a divergent post-hoc pick). `OptimizeProposal` also carries `approximate: bool` + `approx_reason: Option<ApproxReason>` (R0-C1/C3, R2-C1).

### CLI (`btctax-cli`)
- New side-table module `optimize_attest` (`crates/btctax-cli/src/optimize_attest.rs`): table `optimize_attestation(disposal_event TEXT PRIMARY KEY, attestation TEXT NOT NULL, attested_at TEXT NOT NULL)`; `init_table`, `get`, `set`, `all`, `attested_set(conn) -> BTreeSet<EventId>`. Initialized in `Session::from_fresh_vault`; exposed via `Session::optimize_attested_set()`.
- `cmd::optimize::{run, accept, consult}` (new module `crates/btctax-cli/src/cmd/optimize.rs`).
- clap `Command::Optimize(Optimize)` with `Optimize { Run{ tax_year }, Accept{ tax_year, disposal: Option<String>, attest: Option<String> }, Consult{ sell, wallet: Option<String>, at: Option<String>, proceeds: Option<String>, fmv: bool } }`.
- `render::{render_optimize_proposal, render_consult}`.

---

## 3. Task list

1. **Core `optimize` module skeleton** — module, types, `OptimizeError`, `lib.rs` re-export (no logic).
2. **Holistic year scorer** — `score_assignment` (inject selections → fold once → `compute_tax_year`); empty-assignment == plain projection; a higher-basis injection lowers tax.
3. **Candidate generation** — `available_lots_before(disposal)` (truncated clone-fold) + `candidate_selections` (bounded-complete vertex enumeration; deterministic greedy fallback beyond the bound); principal-conservation, per-wallet, determinism KATs.
4. **Mode-1 optimizer `optimize_year`** — assemble + holistically score + min + build `OptimizeProposal`; refuse `NotComputable`/pre-2025. **Optimality KATs vs an independent brute-force oracle** (HIFO-beats-FIFO; ST-vs-LT where naive-HIFO LOSES to an LT pick; loss-harvest within the $3k limit; per-wallet constraint) + determinism.
5. **Compliance + persistability overlay** — `compliance_overlay` (`NonCompliant → AttestedRecording` within envelope) + `persistability`; all variants incl. **2027+ broker forbidden**, self-custody all years, contemporaneous-iff-made-date-≤-sale.
6. **Mode-2 consult `consult_sale`** — synthetic disposal via the `evaluate_disposal` path; per-candidate full-year scoring; ST/LT; **ST→LT timing insight**; **`ProceedsRequired` for a future date**; never mutates.
7. **§1091 wash-sale documentation + monitoring** — module doc note + a KAT proving loss lots are freely selectable (harvest unconstrained).
8. **CLI attestation side-table** — `optimize_attest` (modeled on `tax_profile`); `Session` init + accessor; round-trip + tableless-vault KATs.
9. **CLI `optimize run`** — `cmd::optimize::run` + clap + render; **propose-doesn't-mutate** (event count unchanged), delta + per-disposal compliance shown.
10. **CLI `optimize accept`** — gated persistence: **auto-`Contemporaneous` only when made-date ≤ sale**; **already-executed routes to per-disposal `--attest`** (→ `AttestedRecording`) **or is skipped `NonCompliant`**; **refuse 2027+ broker-held**; **void revokes**. accept-mutates / refuses-without-attestation / refuses-2027-broker / determinism KATs.
11. **CLI `optimize consult`** — `cmd::optimize::consult` + clap + render; what-if + timing line; **`--proceeds`-required-for-future**; **consultation-never-writes-events**.
12. **Whole-diff review + full-suite green (Phase E gate).**

**Dependency order:** 1 → 2 → 3 → 4; 5 depends on 1; **4's compliance wiring depends on Task 5's pure helpers** (`proposed_compliance_status`/`compliance_overlay`/`persistability`) — implement those (Task 5) before finalizing Task 4's proposal-row construction (R0-C2); 6 depends on 2,3; 7 depends on 4; 8 standalone; 9 depends on 4,5; 10 depends on 4,5,8; 11 depends on 6. **12 last.**

---

## 4. Algorithm — decision + justification (§C.4; NFR4/NFR5 acceptance criterion)

**Chosen approach: per-disposal candidate generation → holistic exact scoring.** Greedy-per-disposal is *strictly wrong* (resolved Q#2: §1(h) breakpoints, the $3k/§1212 limit, and §1222 cross-netting couple disposals), so greedy is used **only as a candidate generator** and the *decision* is made by a holistic scorer that re-prices the whole year through B.

**Why a candidate set + brute scoring is exact on the modeled cases.** For a single disposal of `N` sats from a pool, total proceeds are fixed (`proceeds-per-sat` is constant across lot choice — `make_disposal_legs` allocates pro-rata by sat, `fold.rs:103-139`). The disposal's `(ST gain, LT gain)` contribution is therefore a **linear** function of the per-lot sats `x_lot` subject to `Σx_lot = N`, `0 ≤ x_lot ≤ remaining_lot`. The achievable `(ST,LT)` set is the image of a box-capped simplex — a convex polygon whose **vertices** are exactly the selections where every lot is at a bound except at most one (= "consume some subset whole, plus ≤1 partial lot to top up to `N`"). The year's tax is a function only of the summed `(ST,LT)` (plus the fixed profile), so enumerating the **vertex selections** per disposal and scoring the **cartesian product** through B finds the optimum over all whole-lot/vertex identifications. For **whole-lot fixtures** (every disposal's `N` equals a sum of whole lots — how the KATs are built) the achievable points *are* the vertices, so the search is exhaustive and the result is the **true optimum**. Each optimality KAT asserts equality with an **independent exhaustive oracle** that enumerates the same space.

**Generators (per post-2025 disposal, deterministic, deduped):**
- when the available pool has `≤ LOT_ENUM_BOUND` lots (`const LOT_ENUM_BOUND: usize = 12`): **complete vertex enumeration** (all whole-lot subsets summing to `N`, plus each subset extended by one partial lot to reach `N`);
- otherwise (large pool): a deterministic heuristic set — the FIFO / LIFO / HIFO orderings' greedy-fill, plus a "long-term-first" and a "highest-loss-first" greedy-fill, plus per-lot "lead this lot then HIFO-fill". (KAT pools are small → the complete path is what the oracle is checked against.)

**A heuristic-branch pool is INCOMPLETE (R2-C1).** When a disposal's available pool exceeds `LOT_ENUM_BOUND`, the generator returns a strict **subset** of that pool's vertices, so a result computed over it is **not** a proven global minimum. `candidate_selections` therefore **reports** whether it took the heuristic branch, and `optimize_year` propagates that signal: if **any** target disposal's pool was heuristic, the proposal is flagged **`approximate = true, approx_reason = PoolHeuristic { lots, bound }`** (and `ComboCapExceeded` retains precedence). `> 12`-lot single-wallet pools are common (weekly DCA, active trading) and yield a *small* `product`, so without this signal a single big pool would otherwise score `approximate = false` and render as "the proven optimum" with no banner — the headline-forbidden false-global claim. The search stays baseline-seeded, so `delta ≤ 0` still holds; the disclosure fixes the false *"this is the global optimum"* claim, not the safety of the pick.

**Contention grouping (R0-C3) — fold this BEFORE the bound.** Generating each disposal's candidates against the *baseline* consumption of earlier same-pool disposals (`available_lots_before`) **misses cross-period reassignment optima** — e.g. two same-wallet sells in one year where a lot that is ST at `D1`'s date but LT at `D2`'s date should move to `D2` (LT/15%) instead of `D1` (ST/ordinary). That reassignment never appears in `D2`'s independently-generated set. So the optimizer **detects contention**: partition the year's target disposals into **contention groups** — disposals sharing one `PoolKey::Wallet` pool whose available-lot sets overlap (a non-contended disposal is a singleton group). For a contended group, enumerate the **joint** candidate set by **nesting** generation in canonical order: the group's first disposal draws from the pre-group pool; each subsequent disposal's candidates are generated from the pool *resulting from the prior disposal's chosen candidate* (not the baseline). A group's joint candidate list is the set of consistent per-disposal sequences; groups are independent (disjoint pools) so the overall search is the cartesian product **across groups**. If a single group's joint enumeration would exceed its sub-bound (`GROUP_COMBO_BOUND`, ≤ `MAX_COMBOS`), that group falls back to per-disposal-independent generation and the proposal is marked **`approximate = true`, `approx_reason = ContentionUnenumerated { contended, .. }`** (joint enumeration within the bound is preferred; flag-approximate only beyond it). Singleton groups are exactly today's per-disposal path. A result is **never** rendered as "the optimum" when a contended group was not fully enumerated.

**Holistic scoring + bound (R0-C1):** take the cartesian product of the per-**group** candidate lists (deterministic order); cap at `const MAX_COMBOS: usize = 50_000`. **Seed the incumbent with the BASELINE assignment** (each disposal's current-method `baseline_selection`) scored at `base.total_federal_tax_attributable`, so the returned result's `delta ≤ 0` **always** — the optimizer NEVER recommends an assignment worse than doing nothing. If `product ≤ MAX_COMBOS` **and** no contended group was truncated → **exhaustive** search over the enumerated candidates; this earns `approximate = false` (the **proven** global minimum over the vertex space) **only if additionally every target disposal's pool was fully (non-heuristically) enumerated** (≤ `LOT_ENUM_BOUND`) — if any pool took the heuristic branch the proposal is `approximate = true, PoolHeuristic { lots, bound }` even though `product ≤ MAX_COMBOS` (R2-C1). If `product > MAX_COMBOS` → fall back to **baseline-seeded coordinate descent** (start from the **baseline assignment**, NOT all-HIFO; repeatedly, per disposal in `EventId` order, fix the others and pick its best candidate by full-year score; iterate to a fixed point, bounded passes — still deterministic), and set **`approximate = true`, `approx_reason = ComboCapExceeded { combos, cap }`**. Whenever `approximate` is set, **carry the reason out of core** (core has no logger) so the CLI logs `combos`/`cap`/why and the renderer prints the APPROXIMATE banner (Task 9). Score each combination with `score_assignment`; **skip any `NotComputable`** combination (this is how cross-disposal-infeasible combinations self-eliminate — the fold raises `LotSelectionInvalid` → `compute_tax_year` returns `TaxYearNotComputable`); choose the minimal `total_federal_tax_attributable`, ties broken by the §0 total order. Because the incumbent starts at the baseline score, both the exhaustive and the fallback paths satisfy `optimized_tax ≤ baseline_tax`.

**Determinism + exactness:** all containers are `BTreeMap`/`BTreeSet`/sorted `Vec`; the objective and tie-break are `Decimal`/`i64`/`EventId`/`LotId` comparisons only; **no float, no `HashMap` iteration, no `Date::now` in core.**

**Documented bounds:** (i) **multi-partial / tax-kink (R0-M2).** Non-vertex multi-partial selections can matter only by landing exactly on a §1(h)/bracket kink hyperplane; vertex granularity is the natural specific-ID granularity, so they are out of scope — but this limitation is now **disclosed in the rendered output** (a one-line caveat footer, Task 9), not only in `FOLLOWUPS.md`, since the result is otherwise labeled "the least tax." (ii) **contended same-wallet disposals (R0-C3) are no longer silently approximated.** Contention is **detected** and, within `GROUP_COMBO_BOUND`, **jointly enumerated** (the true cross-period optimum is found); only beyond the bound is it approximated, and then the proposal is explicitly `approximate = true` (`ContentionUnenumerated`) and the renderer discloses it. (iii) **large pools above `LOT_ENUM_BOUND` (R2-C1)** take the deterministic heuristic generator (an INCOMPLETE vertex subset); this is now **detected and disclosed** — the proposal is flagged `approximate = true, PoolHeuristic` and the renderer shows the banner. In every case the holistic scorer still rejects infeasible combos to `NotComputable`, so an unsafe/wrong selection is never emitted. None of these limitations ever compromises NFR4/NFR5, and **no non-fully-enumerated result (coordinate-descent fallback, un-enumerated contention, OR a heuristic-branch pool) is ever rendered as "the optimum" without the approximate banner.**

**Resolved spec ambiguities (folded into this plan):**
1. **`AttestedRecording` persistence.** A's `disposal_compliance` only emits `StandingOrder`/`Contemporaneous`/`NonCompliant`; `AttestedRecording` is reserved for C. C records the narrow attestation in a **CLI side-table** (projection input, not a new event type, consistent with the §0 event-sourcing boundary) and a **pure core overlay** (`compliance_overlay`) upgrades a post-hoc `NonCompliant` → `AttestedRecording` **only within the permitted envelope**. A's shipped `disposal_compliance` is unchanged; the overlay is authoritative for the `optimize` surface.
2. **Optimizer scope = the post-2025 per-wallet regime.** `optimize_year`/`consult_sale` target years **≥ 2025** (`disposed_at`/`at` ≥ `TRANSITION_DATE`), the per-wallet `PoolKey::Wallet` world where specific-ID is a live forward lever. A pre-2025 year is **refused** (`OptimizeError::PreTransitionYear`) — a pre-2025 selection is a *restatement* of a closed year, not a free optimization (Cross-cutting M7).
3. **Optimization domain = vertex selections** (whole-lot + ≤1 partial); exhaustive within bounds; (i)/(ii) above are documented limitations.
4. **Mode-2 optimizes only the synthetic disposal's selection** (existing disposals keep their current identification); strictly read-only.
5. **`optimize accept` recomputes deterministically** (run and accept agree by NFR4). Attestation is **per-disposal** (`--disposal <ref> --attest "<text>"`) so the app never invites a blanket false attestation.

---

## TASK 1 — Core `optimize` module skeleton (types + `OptimizeError`)

**Goal.** Stand up `btctax-core::optimize` with the result/error/request types (no logic), so later tasks compute over concrete types.

**Files**
- new `crates/btctax-core/src/optimize.rs`
- modify `crates/btctax-core/src/lib.rs` (`pub mod optimize;` + re-export)
- modify `crates/btctax-core/src/event.rs` (add `PartialOrd, Ord` to `LotPick`'s derive — R0-I2; additive)

**Interfaces (produces)**
```rust
// crates/btctax-core/src/optimize.rs
//! Sub-project C — rate-aware optimizer. ASSIGNS lots to disposals (specific identification);
//! it does NOT advise whether to sell/hold (no investment advice — §C scope). Minimizes B's
//! federal `total_federal_tax_attributable` over feasible per-disposal `LotSelection`s, within the
//! §1.1012-1(j) identification boundary (adequate ID by the time of sale; no compliant post-hoc).
//! Deterministic (NFR4) + exact (NFR5): BTreeMap/sorted iteration, Decimal/i64 only, no float.
//! §1091 wash-sale does NOT apply to crypto — loss lots are freely selectable (Task 7; monitor).
use crate::conventions::{Sat, TaxDate, Usd};
use crate::event::{DisposeKind, LotPick};
use crate::identity::{EventId, WalletId};
use crate::project::ComplianceStatus;
use crate::state::Blocker;
use crate::tax::MarginalRates;
use crate::project::EvaluateError;

/// The `accept`-gate verdict for one disposal (computed in core; enforced by the CLI, Task 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistability {
    /// The selection's made-date is at/before the sale → §A.5(b) `Contemporaneous`; persist freely.
    ContemporaneousNow,
    /// Already-executed (made-date after the sale) but within the own-books envelope → persist ONLY
    /// behind the narrow contemporaneous-ID attestation (→ `AttestedRecording`).
    NeedsAttestation,
    /// 2027+ broker-held: own-books is insufficient; C may NEVER persist (no attestation can cure it).
    ForbiddenBroker2027,
}

/// One disposal's line in a Mode-1 proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalProposal {
    pub disposal: EventId,
    pub wallet: WalletId,
    pub date: TaxDate,
    pub current_selection: Vec<LotPick>,   // lots the CURRENT projection consumes (baseline)
    pub proposed_selection: Vec<LotPick>,  // the optimizer's tax-minimizing pick
    pub status: ComplianceStatus,          // overlay-aware (may be AttestedRecording, Task 5)
    pub persistable: Persistability,
}

/// Why a proposal is only APPROXIMATE (not a proven global minimum). Carried OUT of core (core has no
/// logger) so the CLI can log the cap/why and the renderer can show the banner. Plain counts only →
/// deterministic + serde/Eq-friendly (R0-C1/C3 fold).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproxReason {
    /// The cartesian product of per-group candidate lists exceeded `MAX_COMBOS`; the baseline-seeded
    /// coordinate-descent fallback ran (a LOCAL optimum — disclosed, and never worse than baseline).
    ComboCapExceeded { combos: usize, cap: usize },
    /// ≥1 contended same-wallet pool could not be JOINTLY enumerated within the bound; its disposals
    /// fell back to per-disposal-independent generation (a cross-period reassignment optimum may be
    /// missed — R0-C3). `contended` = number of disposals in the un-enumerated contention group(s).
    ContentionUnenumerated { contended: usize, combos: usize, cap: usize },
    /// ≥1 target disposal's available pool exceeded `LOT_ENUM_BOUND`, so `candidate_selections`
    /// returned a deterministic but INCOMPLETE heuristic SUBSET of that pool's vertices (not the full
    /// vertex enumeration) — the result over that pool is therefore NOT a proven global minimum
    /// (R2-C1). Common in practice (weekly-DCA / active-trading pools with > 12 lots). `lots` = the
    /// largest heuristic pool's lot count; `bound` = `LOT_ENUM_BOUND`. Baseline-seeded, so `delta ≤ 0`
    /// still holds — the disclosure corrects the false "proven optimum" claim, not the pick's safety.
    PoolHeuristic { lots: usize, bound: usize },
}

/// Mode-1 proposal: what-if by default (running this binds NOTHING — §C.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizeProposal {
    pub year: i32,
    pub baseline_tax: Usd,   // total_federal_tax_attributable under current identification
    pub optimized_tax: Usd,  // under the proposed selections
    pub delta: Usd,          // optimized − baseline — ALWAYS ≤ 0 (baseline-seeded search; never worsens)
    pub per_disposal: Vec<DisposalProposal>,
    pub marginal_rates: MarginalRates,
    /// `false` ⇔ the vertex set was **FULLY enumerated AND exhaustively scored** — i.e. EVERY target
    /// disposal's pool was ≤ `LOT_ENUM_BOUND` (complete vertex enumeration, NOT a heuristic subset —
    /// R2-C1), the overall `product` was ≤ `MAX_COMBOS` (exhaustive, not coordinate-descent), AND every
    /// contended pool was jointly enumerated. ONLY then is the result the PROVEN global minimum over the
    /// vertex space. `true` ⇔ ANY of those failed (a disclosed LOCAL / under-enumerated / heuristic-pool
    /// result) — the renderer MUST print the "APPROXIMATE — not a guaranteed global minimum" banner and
    /// the CLI MUST log `approx_reason` (R0-C1/C3, R2-C1). NEVER render `optimized_tax` as "the optimum"
    /// when this is `true`.
    pub approximate: bool,
    pub approx_reason: Option<ApproxReason>,
}

/// Mode-2 (pre-trade consultation) request — a hypothetical sale NOT in the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultRequest {
    pub sell_sat: Sat,
    pub wallet: WalletId,
    pub at: TaxDate,
    pub proceeds: Option<Usd>, // required when no dataset price exists for `at` (future dates)
    pub kind: DisposeKind,
}

/// §C.3 ST→LT crossover timing insight (tax decision-support; NOT a hold/sell recommendation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingInsight {
    pub st_sat_in_selection: Sat,   // sats in the best selection that are short-term as of `at`
    pub latest_crossover: TaxDate,  // the last date any of those lots becomes long-term
    pub tax_if_sold_long_term: Usd, // same lots, scored as if sold on/after `latest_crossover`
    pub saving_if_waited: Usd,      // total_now − tax_if_sold_long_term (≥ 0)
}

/// Mode-2 read-only what-if result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultReport {
    pub req: ConsultRequest,
    pub proposed_selection: Vec<LotPick>,
    pub st_gain: Usd,
    pub lt_gain: Usd,
    pub total_federal_tax_attributable: Usd,
    pub timing: Option<TimingInsight>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizeError {
    /// B refuses to compute the year (any Hard blocker anywhere, or missing profile/table) — I6.
    YearNotComputable(Blocker),
    /// A synthetic consult disposal needs `--proceeds` (no dataset price for `at`), etc.
    Evaluate(EvaluateError),
    /// Mode 1: the year has no method-honoring disposals to optimize.
    NoDisposals,
    /// Mode 2: the wallet has no lots available to sell at `at`.
    NoLots,
    /// The requested year is pre-2025 — a restatement of a closed year, not an optimization (M7).
    PreTransitionYear(i32),
}
```
Add to `crates/btctax-core/src/lib.rs` after the `pub mod tax;` block:
```rust
pub mod optimize;
pub use optimize::{
    consult_sale, optimize_year, score_assignment, ApproxReason, ConsultReport, ConsultRequest,
    DisposalProposal, OptimizeError, OptimizeProposal, Persistability, TimingInsight,
};
```

> **A-side additive change (R0-I2).** `LotPick` (`event.rs:190`) currently derives only
> `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`, but C keys a `BTreeSet<Vec<LotPick>>`
> (Task 3), tie-breaks on `(total, &Vec<LotPick>)` (Tasks 4/6), and orders
> `BTreeMap<EventId, Vec<LotPick>>` assignments (§0 total order) — **all require `Vec<LotPick>: Ord`,
> hence `LotPick: Ord`.** Both fields are `Ord` (`LotId: Ord` `identity.rs:116`; `Sat = i64`), so add
> `PartialOrd, Ord` to `LotPick`'s derive. This is a **tiny, purely ADDITIVE** change: it adds a
> trait impl, perturbs no existing field/serde representation, and changes no runtime behavior (nothing
> in A relied on `LotPick` being un-ordered). The plan's earlier "reuse `LotPick` verbatim / no A-side
> change" phrasing is corrected accordingly (this derive is the one exception).

**Steps**
1. **RED.** In `optimize.rs` add `#[cfg(test)] mod tests` with a compile/shape test:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_variants_are_constructible_and_eq() {
        let e = OptimizeError::PreTransitionYear(2024);
        assert_eq!(e, OptimizeError::PreTransitionYear(2024));
        assert_ne!(e, OptimizeError::NoDisposals);
        assert_eq!(Persistability::ForbiddenBroker2027, Persistability::ForbiddenBroker2027);
    }
    #[test]
    fn lot_pick_is_totally_ordered() {
        // R0-I2: the dedup/tie-break machinery requires `Vec<LotPick>: Ord`. A BTreeSet of pick-vecs
        // must compile and sort deterministically.
        use std::collections::BTreeSet;
        let mut s: BTreeSet<Vec<LotPick>> = BTreeSet::new();
        s.insert(vec![/* pick(b) */]);
        s.insert(vec![/* pick(a) */]);
        let _sorted: Vec<Vec<LotPick>> = s.into_iter().collect(); // compiles ⇒ LotPick: Ord
    }
}
```
   `cargo test -p btctax-core optimize::tests` → **RED** (module/types absent; `LotPick: Ord` missing).
2. **GREEN.** Add the module + `lib.rs` wiring above, **and add `PartialOrd, Ord` to `LotPick`'s
   `#[derive(...)]` in `event.rs:190`** (R0-I2; additive — confirm `cargo test -p btctax-core` and the
   existing A-side serde round-trip KATs still pass unchanged). `cargo test -p btctax-core` green.
3. **Full suite** (`cargo test --workspace && clippy -D warnings && fmt --check`) → review-to-green → **commit** `feat(core): optimize module skeleton (Sub-project C types)`.

---

## TASK 2 — Holistic year scorer `score_assignment`

**Goal.** The single primitive every mode uses: inject a per-disposal selection set, **fold once**, run B's `compute_tax_year`. No mutation (clone-fold-discard, mirroring `evaluate.rs:105-179`).

**Files**: modify `crates/btctax-core/src/optimize.rs`; new test `crates/btctax-core/tests/optimize_score.rs`.

**Interfaces (consumes A+B, produces)**
```rust
use crate::event::LedgerEvent;
use crate::price::PriceProvider;
use crate::project::fold::fold;
use crate::project::resolve::resolve;
use crate::state::LedgerState;
use crate::tax::{compute_tax_year, TaxOutcome, TaxProfile, TaxTables};
use crate::ProjectionConfig;
use std::collections::BTreeMap;

/// Fold the canonical timeline with `assignment`'s per-disposal selections injected (overriding any
/// persisted selection for those events), WITHOUT mutating the ledger. Clone-fold-discard.
fn fold_with(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    assignment: &BTreeMap<EventId, Vec<LotPick>>,
) -> LedgerState {
    let mut res = resolve(events, prices, config);
    for (disposal, picks) in assignment {
        res.selections.insert(disposal.clone(), picks.clone()); // BTreeMap iteration = deterministic
    }
    fold(res, prices, config)
}

/// Holistic score: B's federal `TaxOutcome` for `year` under `assignment`. An infeasible selection
/// (cross-disposal contention / over-draw / cross-wallet) folds to a hard `LotSelectionInvalid`
/// (fold.rs:51-66) → `compute_tax_year` returns `NotComputable` — the caller skips that combination.
///
/// **PRECONDITION — principal conservation (R0-M1).** Each injected pick-set MUST satisfy
/// `Σ LotPick.sat == the disposal's principal sat`. `fold_with` injects straight into
/// `res.selections`, bypassing `resolve`'s `Σ == principal` guard (resolve.rs); and the fold's
/// `consume_picks` hardcodes `shortfall = 0` (pools.rs:175) while `selection_feasible` checks only
/// per-lot availability — **not** the sum. A NON-conserving assignment therefore under-consumes
/// *silently* (no blocker) → a falsely-low score. `optimize_year`/`consult_sale` generators always
/// conserve, but `score_assignment` is `pub` for KATs, so it **`debug_assert!`s** the sum against the
/// per-disposal principal (looked up once from a baseline fold) and documents this precondition loudly.
pub fn score_assignment(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    assignment: &BTreeMap<EventId, Vec<LotPick>>,
) -> TaxOutcome {
    // R0-M1 guard: in debug builds, assert each injected pick-set conserves the disposal's principal.
    debug_assert!(
        assignment_conserves_principal(events, prices, config, assignment),
        "score_assignment: injected assignment violates Σpicks == principal (R0-M1)"
    );
    let state = fold_with(events, prices, config, assignment);
    compute_tax_year(events, &state, year, profile, tables)
}
```
> `assignment_conserves_principal` folds the **baseline** (no injection) once, maps each disposal →
> `Σ legs.sat` (its principal), and checks every injected `assignment` entry's `Σ picks.sat` against it
> (a missing/zero-principal disposal id ⇒ fail). It runs only under `debug_assert!` (zero release cost)
> and adds an explicit Task-2 KAT: a deliberately non-conserving hand-built assignment trips the assert
> in a `#[should_panic]` debug test (documenting the precondition).
> `resolve` is `pub` and `Resolution.selections` is a `pub` field (resolve.rs:135); `fold` is `pub` (fold.rs:330); `sort_canonical` is `pub` (resolve.rs:805, used by Task 3's R0-I1 fix). **The only A-side change in C is the additive `PartialOrd, Ord` derive on `LotPick` (Task 1, R0-I2) — serde-compatible, no behavior change.** No A-side *visibility* change is otherwise required. If a future refactor narrows `fold`, widen it to `pub(crate)` (no behavior change).

**Steps**
1. **RED** — `crates/btctax-core/tests/optimize_score.rs`. Build a synthetic ledger: a 2025 self-custody wallet with two pre-`disposed_at` lots of equal sats but **different basis** (low-basis "A", high-basis "B"), one 2025 `Sell` of A+B-worth of sats. Profile + a synthetic 2025 `TaxTables` (reuse the `tests/kat_tax.rs` builder style). Assert:
```rust
// (a) empty assignment == plain projection's compute_tax_year
let plain = compute_tax_year(&events, &project(&events,&prices,&cfg), 2025, Some(&profile), &tables);
let empty: std::collections::BTreeMap<EventId, Vec<LotPick>> = Default::default();
assert_eq!(score_assignment(&events,&prices,&cfg,2025,Some(&profile),&tables,&empty), plain);

// (b) selecting the HIGH-basis lot first yields LESS taxable gain than the FIFO baseline
let pick_high = btreemap!{ sell_id.clone() => vec![pick(lot_b, half), pick(lot_a, half)] };
let s = score_assignment(&events,&prices,&cfg,2025,Some(&profile),&tables,&pick_high);
let (TaxOutcome::Computed(hi), TaxOutcome::Computed(base)) = (s, plain) else { panic!() };
assert!(hi.total_federal_tax_attributable <= base.total_federal_tax_attributable);
```
   → **RED** (`score_assignment` absent).
2. **GREEN** — add `fold_with` + `score_assignment`. Tests pass.
3. **Full suite → review-to-green → commit** `feat(core): holistic year scorer score_assignment`.

---

## TASK 3 — Candidate generation (available-lots pre-pass + bounded-complete vertex enumeration)

**Goal.** Per post-2025 disposal, produce a **deterministic, deduped** list of principal-conserving, per-wallet-legal candidate `Vec<LotPick>` whose vertex set is **complete on small pools** (the optimality guarantee, §4).

**Files**: modify `crates/btctax-core/src/optimize.rs`; new test `crates/btctax-core/tests/optimize_candidates.rs`.

**Interfaces (produces, all private except as tested via `optimize_year`)**
```rust
use crate::conventions::TRANSITION_DATE;
use crate::project::pools::{pool_key, PoolKey};
use crate::state::Lot;

const LOT_ENUM_BOUND: usize = 12; // ≤ this many lots → complete vertex enumeration

/// Lots available to a disposal at `date` in `wallet`, computed by a clone-fold of the timeline
/// TRUNCATED just before the disposal (NFR4: deterministic; no fold modification). Post-2025 →
/// the disposal's own wallet pool (§1.1012-1(j) per-wallet). Returns lots with remaining_sat > 0.
fn available_lots_before(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    disposal: &EventId,
    date: TaxDate,
    wallet: &WalletId,
) -> Vec<Lot> {
    let mut res = resolve(events, prices, config);
    // R0-I1: `resolve` builds `timeline` in DB/load order (resolve.rs:492-518) — it does NOT sort.
    // The fold sorts canonically THEN stable-partitions by transition side (fold.rs:335,341), so we
    // MUST replicate BOTH before locating/truncating, else `position`/`truncate` operate on load order
    // and drop a lot acquired-before-`disposal`-in-TIME but loaded-after it (→ missed candidate →
    // non-optimal). Mirror the exact fold ordering:
    crate::project::resolve::sort_canonical(&mut res.timeline);     // pub (resolve.rs:805)
    res.timeline.sort_by_key(|e| e.date() >= TRANSITION_DATE);      // stable pre-2025 partition (fold.rs:341)
    let Some(idx) = res.timeline.iter().position(|e| &e.id == disposal) else { return Vec::new() };
    res.timeline.truncate(idx);            // drop the disposal and everything at/after it (canonical order)
    let pre = fold(res, prices, config);   // pool state just before the disposal (fold re-sorts the subset)
    let want = pool_key(date, wallet);     // post-2025 → Wallet(wallet); pre-2025 → Universal
    let mut lots: Vec<Lot> = pre
        .lots
        .into_iter()
        .filter(|l| l.remaining_sat > 0 && pool_key(date, &l.wallet) == want)
        .collect();
    lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id)); // total order (NFR4)
    lots
}

/// All principal-conserving vertex selections of `need` sats over `lots` (same pool): every whole-lot
/// subset summing to `need`, plus each strict subset (Σ<need) extended by ONE partial lot to reach
/// `need`. Deduped + sorted (NFR4). On pools > LOT_ENUM_BOUND, a deterministic heuristic set instead.
///
/// **Returns `(candidates, heuristic)` (R2-C1).** `heuristic == false` ⇔ the COMPLETE vertex set was
/// enumerated (`lots.len() ≤ LOT_ENUM_BOUND`); `heuristic == true` ⇔ the pool exceeded the bound and
/// only a deterministic INCOMPLETE subset was returned — the caller (`optimize_year`) MUST then flag
/// the proposal `approximate = true, PoolHeuristic { .. }` (a heuristic-pool result is not a proven
/// global minimum). Without this signal a single > 12-lot pool would score `approximate = false` and
/// render as "the optimum" — the headline-forbidden false-global claim.
fn candidate_selections(lots: &[Lot], need: Sat) -> (Vec<Vec<LotPick>>, bool) {
    let heuristic = lots.len() > LOT_ENUM_BOUND; // R2-C1: did we take the incomplete branch?
    let mut out: std::collections::BTreeSet<Vec<LotPick>> = std::collections::BTreeSet::new();
    let canon = |mut v: Vec<LotPick>| { v.sort_by(|a, b| a.lot.cmp(&b.lot)); v }; // canonical key

    if lots.len() <= LOT_ENUM_BOUND {
        // complete vertex enumeration over 2^n subsets (n ≤ 12)
        for mask in 0u32..(1u32 << lots.len()) {
            let mut whole: Vec<LotPick> = Vec::new();
            let mut sum: Sat = 0;
            for (i, l) in lots.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    whole.push(LotPick { lot: l.lot_id.clone(), sat: l.remaining_sat });
                    sum += l.remaining_sat;
                }
            }
            if sum == need {
                out.insert(canon(whole));            // whole-lot vertex
            } else if sum < need {
                let short = need - sum;              // top up with ONE partial lot not in the mask
                for (i, l) in lots.iter().enumerate() {
                    if mask & (1 << i) == 0 && l.remaining_sat >= short {
                        let mut v = whole.clone();
                        v.push(LotPick { lot: l.lot_id.clone(), sat: short });
                        out.insert(canon(v));
                    }
                }
            }
        }
    } else {
        // deterministic heuristic generators (greedy-fill in a given lot order)
        let fill = |order: Vec<usize>| -> Option<Vec<LotPick>> {
            let mut v = Vec::new();
            let mut rem = need;
            for i in order {
                if rem <= 0 { break; }
                let take = rem.min(lots[i].remaining_sat);
                if take > 0 { v.push(LotPick { lot: lots[i].lot_id.clone(), sat: take }); rem -= take; }
            }
            (rem == 0).then(|| canon(v))
        };
        let by = |key: &dyn Fn(&Lot, &Lot) -> std::cmp::Ordering| {
            let mut ix: Vec<usize> = (0..lots.len()).collect();
            ix.sort_by(|&a, &b| key(&lots[a], &lots[b])); ix
        };
        use std::cmp::Ordering;
        let hifo = |a: &Lot, b: &Lot| (b.usd_basis * Usd::from(a.remaining_sat))
            .cmp(&(a.usd_basis * Usd::from(b.remaining_sat)))
            .then(a.acquired_at.cmp(&b.acquired_at)).then(a.lot_id.cmp(&b.lot_id));
        let fifo = |a: &Lot, b: &Lot| a.acquired_at.cmp(&b.acquired_at).then(a.lot_id.cmp(&b.lot_id));
        let lifo = |a: &Lot, b: &Lot| b.acquired_at.cmp(&a.acquired_at).then(b.lot_id.cmp(&a.lot_id));
        let lt_first = |a: &Lot, b: &Lot| a.gain_hp_start().cmp(&b.gain_hp_start()).then(a.lot_id.cmp(&b.lot_id));
        for k in [&hifo as &dyn Fn(&Lot,&Lot)->Ordering, &fifo, &lifo, &lt_first] {
            if let Some(v) = fill(by(k)) { out.insert(v); }
        }
        for lead in 0..lots.len() { // per-lot lead, then HIFO-fill
            let mut order = vec![lead];
            order.extend(by(&hifo).into_iter().filter(|&i| i != lead));
            if let Some(v) = fill(order) { out.insert(v); }
        }
    }
    (out.into_iter().collect(), heuristic) // (sorted Vec — NFR4, heuristic-branch flag — R2-C1)
}
```

**Contention grouping + joint enumeration (R0-C3).** A non-contended disposal keeps the independent
path above. For ≥2 disposals consuming the **same `PoolKey::Wallet` pool in the year** whose available
lots overlap, generate the **joint** candidate set by nesting `candidate_selections` in canonical
order, each later disposal drawing from the pool left by the earlier disposal's chosen candidate —
this is what recovers cross-period reassignment optima (the ST-at-D1 / LT-at-D2 lot).
```rust
const GROUP_COMBO_BOUND: usize = 4_096; // per-group joint-enumeration ceiling (≤ MAX_COMBOS)

/// Partition the year's targets into contention groups: disposals on one `PoolKey::Wallet` pool whose
/// `available_lots_before` sets overlap. Deterministic: groups keyed by `PoolKey`, members in canonical
/// (EventId) order; a non-overlapping disposal is its own singleton group.
fn contention_groups(
    events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig,
    targets: &[(EventId, WalletId, TaxDate, Sat)],
) -> Vec<Vec<usize>> { /* group indices into `targets`; singletons for non-contended */ unimplemented!() }

/// Joint candidate assignments for ONE contention group, in canonical disposal order. Returns
/// `Some((maps, heuristic_lots))` where `maps` is a deterministic, sorted list of
/// `BTreeMap<EventId, Vec<LotPick>>` (one map per consistent sequence), generated by NESTING:
/// disposal[0] from the pre-group pool; disposal[k] from the pool resulting after disposal[k-1]'s
/// chosen candidate (re-fold the truncated timeline with the prior picks injected, or consume the prior
/// picks from a working pool copy). `heuristic_lots` (R2-C1) = `Some(max lot-count)` if ANY nested
/// `candidate_selections` call took the `> LOT_ENUM_BOUND` heuristic branch (so the joint set, though
/// within the bound, is still over an incomplete per-disposal vertex subset — the caller flags
/// `PoolHeuristic`), else `None`. Returns the outer `None` (→ caller flags `ContentionUnenumerated`)
/// if the joint count would exceed `GROUP_COMBO_BOUND`.
fn group_candidate_assignments(
    events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig,
    group: &[(EventId, WalletId, TaxDate, Sat)],
) -> Option<(Vec<BTreeMap<EventId, Vec<LotPick>>>, Option<usize>)> { unimplemented!("implemented in this task") }
```
A singleton group degenerates to today's `candidate_selections(available_lots_before(D), need)` (whose
returned `heuristic` flag the caller folds into `PoolHeuristic`), one map per candidate. `optimize_year`
(Task 4) takes the cartesian product **across groups** (disjoint pools ⇒ cross-group feasibility is
automatic) and, if any group returns the outer `None`, marks the proposal
`approximate = true, ContentionUnenumerated { contended }` and falls that group back to the independent
per-disposal lists (still baseline-seeded); if any group (or singleton) reports a heuristic pool it
marks `approximate = true, PoolHeuristic` (R2-C1).

**Steps**
1. **RED** — `tests/optimize_candidates.rs` (unit-tests the two helpers through a small `pub(crate)` test shim, or re-exports them `#[cfg(test)]`). With three whole 100k-sat lots A/B/C and `need = 200_000`:
   - every returned candidate has `Σsat == 200_000` (principal conservation);
   - the set equals `{ {A,B}, {A,C}, {B,C} }` (complete subset enumeration; canonical/sorted);
   - with `need = 150_000` (one partial), the set includes e.g. `{A(100k), B(50k)}`, `{B(100k), A(50k)}`, … (whole + one partial);
   - calling twice returns byte-identical `Vec`s (determinism);
   - a lot in another wallet is **excluded** by `available_lots_before` (per-wallet) — exercised via a two-wallet fixture asserting the cross-wallet lot never appears.

   (All of the above destructure `candidate_selections(..) -> (candidates, heuristic)` and assert over `candidates`.)
   - **R2-C1 (heuristic flag, both ways).** A pool of **≤ `LOT_ENUM_BOUND`** lots → returned `heuristic == false` (complete vertex enumeration); a pool of **> `LOT_ENUM_BOUND`** lots (e.g. 13–20) → `heuristic == true` (the deterministic INCOMPLETE subset) and the returned `candidates` is a strict subset of the full vertex set. This is the signal `optimize_year` propagates into `approximate`/`PoolHeuristic`.
   - **R0-I1 (load-order ≠ canonical-order).** Build the fixture so the ledger/DB (`events`) load order is
     **not** the canonical time order — e.g. a lot acquired *earlier in time* but appended *later* in
     `events`, and a not-yet-acquired lot appended earlier. Assert `available_lots_before(D)` returns
     exactly the lots acquired-before-`D`-in-TIME (the early-time/late-load lot **present**; the
     late-time/early-load lot **absent**). Without the `sort_canonical` + partition fix this fails.
   - **R0-C3 (contention).** Two same-wallet disposals `D1`,`D2` drawing from overlapping lots: assert
     `contention_groups` puts them in ONE group, and `group_candidate_assignments` yields a joint
     assignment in which `D1` deviates from its independent baseline to free a lot for `D2` (a sequence
     unreachable by the independent per-disposal product). Two non-overlapping disposals → two singleton
     groups. A group whose joint count exceeds `GROUP_COMBO_BOUND` → `None`.
   → **RED**.
2. **GREEN** — implement the helpers (`available_lots_before` with the R0-I1 `sort_canonical` + transition
   partition; `candidate_selections`; `contention_groups`; `group_candidate_assignments`). Tests pass.
3. **Full suite → review-to-green → commit** `feat(core): per-disposal + joint contended candidate generation`.

---

## TASK 4 — Mode-1 optimizer `optimize_year` (+ optimality KATs vs brute-force oracle)

**Goal.** §C.1/C.2 holistic single-year optimizer: assemble per-disposal candidates, holistically score the cartesian product through B, pick the deterministic minimum, build the what-if `OptimizeProposal`. Refuse `NotComputable`/pre-2025.

**Files**: modify `crates/btctax-core/src/optimize.rs`; new test `crates/btctax-core/tests/optimize_mode1.rs`.

**Interfaces (produces)**
```rust
use crate::project::{disposal_compliance, DisposalCompliance}; // A's projection (R0-C2: BASELINE status only)
use crate::tax::TaxResult;
use std::collections::{BTreeMap, BTreeSet};
// R0-C2/C1/C3: `optimize_year` also calls the same-module pure helpers `proposed_compliance_status`,
// `compliance_overlay`, `persistability` (Task 5) and constructs `ApproxReason`/`approximate` (Task 1).

const MAX_COMBOS: usize = 50_000;

pub fn optimize_year(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    attested: &BTreeSet<EventId>, // CLI-supplied attested disposals (Task 5 overlay); empty for pure what-if
    proposal_made: TaxDate,       // R0-C2: the proposed picks' made-date (= "now"), passed from the CLI
                                  // seam so core stays clock-free (NFR4). Drives the HONEST compliance +
                                  // persistability of each proposed pick (never read from a clock in core).
) -> Result<OptimizeProposal, OptimizeError> {
    if year < TRANSITION_DATE.year() {
        return Err(OptimizeError::PreTransitionYear(year));
    }
    // Baseline = current filing position (no injected selections).
    let baseline_state = fold_with(events, prices, config, &BTreeMap::new());
    let base = match compute_tax_year(events, &baseline_state, year, profile, tables) {
        TaxOutcome::Computed(r) => r,
        TaxOutcome::NotComputable(b) => return Err(OptimizeError::YearNotComputable(b)),
    };

    // The year's method-honoring disposals (Disposal records for `year`), in EventId order (NFR4).
    let mut targets: Vec<(EventId, WalletId, TaxDate, Sat)> = baseline_state
        .disposals
        .iter()
        .filter(|d| !d.fee_mini_disposition && d.disposed_at.year() == year)
        .filter_map(|d| {
            let wallet = events.iter().find(|e| e.id == d.event).and_then(|e| e.wallet.clone())?;
            let sat: Sat = d.legs.iter().map(|l| l.sat).sum();
            Some((d.event.clone(), wallet, d.disposed_at, sat))
        })
        .collect();
    targets.sort_by(|a, b| a.0.cmp(&b.0));
    if targets.is_empty() {
        return Err(OptimizeError::NoDisposals);
    }

    // R0-C3: group targets into contention groups; build each group's candidate list (JOINT where
    // contended, independent for singletons). Each group's list is a Vec of partial assignments
    // (BTreeMap<EventId, Vec<LotPick>>). A contended group that cannot be jointly enumerated within
    // GROUP_COMBO_BOUND falls back to independent per-disposal generation AND flags the proposal.
    let groups = contention_groups(events, prices, config, &targets);
    let mut group_lists: Vec<Vec<BTreeMap<EventId, Vec<LotPick>>>> = Vec::new();
    let mut product: usize = 1;
    let mut approximate = false;
    let mut contended_unenum = 0usize;
    // R2-C1: the largest pool that used the `> LOT_ENUM_BOUND` heuristic (INCOMPLETE) branch, across
    // every singleton AND every nested group generation; `Some(_)` ⇒ flag `PoolHeuristic`. (Updated
    // inline — a capturing closure would hold a mutable borrow past the loop's later read.)
    let mut pool_heuristic_lots: Option<usize> = None;
    for g in &groups {
        let members: Vec<_> = g.iter().map(|&i| targets[i].clone()).collect();
        let maps: Vec<BTreeMap<EventId, Vec<LotPick>>> = if members.len() == 1 {
            let (id, wallet, date, need) = &members[0]; // singleton: today's independent path
            let lots = available_lots_before(events, prices, config, id, *date, wallet);
            let (mut cands, heuristic) = candidate_selections(&lots, *need);
            if heuristic { // R2-C1: incomplete vertex subset for this pool → track the largest such pool
                pool_heuristic_lots = Some(pool_heuristic_lots.map_or(lots.len(), |m| m.max(lots.len())));
            }
            if cands.is_empty() { cands.push(baseline_selection(&baseline_state, id)); }
            cands.into_iter().map(|p| BTreeMap::from([(id.clone(), p)])).collect()
        } else {
            match group_candidate_assignments(events, prices, config, &members) {
                Some((joint, heur_lots)) => { // jointly enumerated within the bound → EXACT unless a
                    if let Some(n) = heur_lots { // nested pool was heuristic (R2-C1)
                        pool_heuristic_lots = Some(pool_heuristic_lots.map_or(n, |m| m.max(n)));
                    }
                    joint
                }
                None => {             // beyond the bound → disclose + independent fallback (baseline-safe)
                    approximate = true;
                    contended_unenum += members.len();
                    independent_group_maps(events, prices, config, &baseline_state, &members)
                }
            }
        };
        product = product.saturating_mul(maps.len());
        group_lists.push(maps);
    }
    if pool_heuristic_lots.is_some() { approximate = true; } // R2-C1: a heuristic pool is never "proven"

    // R0-C1: BASELINE-SEED the search so `delta ≤ 0` ALWAYS (never recommend worse-than-doing-nothing).
    let baseline_assignment: BTreeMap<EventId, Vec<LotPick>> = targets
        .iter().map(|(id, ..)| (id.clone(), baseline_selection(&baseline_state, id))).collect();
    // Exhaustive (PROVEN optimum, approximate=false) within MAX_COMBOS; else baseline-seeded coordinate
    // descent (a disclosed LOCAL optimum, approximate=true). Both incumbents START at the baseline score.
    let best: BTreeMap<EventId, Vec<LotPick>> = if product <= MAX_COMBOS {
        exhaustive_min(events, prices, config, year, profile, tables,
                       &group_lists, &baseline_assignment, &base)
    } else {
        approximate = true;
        coordinate_descent(events, prices, config, year, profile, tables,
                           &group_lists, &baseline_assignment, &base)
    };
    // Reason precedence: a blown overall product (coordinate descent ran) dominates an un-enumerated
    // contended group, which dominates a per-pool heuristic (R2-C1). ALL THREE set `approximate` and
    // raise the same banner; precedence only picks which (most-severe) reason is reported.
    let approx_reason = if product > MAX_COMBOS {
        Some(ApproxReason::ComboCapExceeded { combos: product, cap: MAX_COMBOS })
    } else if contended_unenum > 0 {
        Some(ApproxReason::ContentionUnenumerated { contended: contended_unenum, combos: product, cap: MAX_COMBOS })
    } else if let Some(lots) = pool_heuristic_lots {
        Some(ApproxReason::PoolHeuristic { lots, bound: LOT_ENUM_BOUND })
    } else {
        None
    };

    let opt_state = fold_with(events, prices, config, &best);
    let opt = match compute_tax_year(events, &opt_state, year, profile, tables) {
        TaxOutcome::Computed(r) => r,
        TaxOutcome::NotComputable(b) => return Err(OptimizeError::YearNotComputable(b)),
    };

    // Per-disposal proposal rows. R0-C2: status/persistability are judged by the PROPOSED pick's OWN
    // timeliness, NOT by `disposal_compliance(events, opt_state)`. The proposed pick lives in the fold's
    // `selections`, NOT in `events`; feeding `events` to `disposal_compliance` skips the selection
    // branch (compliance.rs:144-149) and a divergent post-hoc cherry-pick falls through to
    // `StandingOrder` = compliant — FORBIDDEN (§0). We instead compute each row via
    // `proposed_compliance_status` (Task 5), then apply the `compliance_overlay`. A's
    // `disposal_compliance(events, &baseline_state)` supplies only the BASELINE status, used solely to
    // preserve a genuine StandingOrder/Contemporaneous when the proposal does NOT diverge from current.
    let base_comp = disposal_compliance(events, &baseline_state);
    let mut rows: Vec<DisposalCompliance> = Vec::new();
    let mut row_meta: Vec<(EventId, WalletId, TaxDate, Vec<LotPick>, Vec<LotPick>)> = Vec::new();
    for (id, wallet, date, _need) in &targets {
        let current = baseline_selection(&baseline_state, id);
        let proposed = best.get(id).cloned().unwrap_or_else(|| current.clone());
        let baseline_status = base_comp.iter().find(|c| &c.disposal == id)
            .map(|c| c.status.clone()).unwrap_or(ComplianceStatus::NonCompliant);
        // The proposed pick's own made-date governs; a standing order NEVER rescues a divergent
        // post-hoc pick (§1.1012-1(j)). `proposed == current` keeps the genuine baseline status.
        let status = proposed_compliance_status(
            wallet, *date, proposal_made, &proposed, &current, &baseline_status);
        rows.push(DisposalCompliance { disposal: id.clone(), wallet: wallet.clone(), date: *date, status });
        row_meta.push((id.clone(), wallet.clone(), *date, current, proposed));
    }
    // Task-5 overlay: NonCompliant + attested + within own-books envelope + proposed==current →
    // AttestedRecording (never for 2027+ broker; never for a non-attested post-hoc pick; never for a
    // DIVERGENT re-run pick — R2-I1). The attestation side-table is keyed by DISPOSAL, not by selection,
    // so the upgrade is gated on `unchanged` = the disposals whose proposed pick equals the in-force
    // (persisted-and-attested) current selection. After attesting+persisting P1 the baseline reflects
    // P1, so `current == P1`; a re-run proposing a divergent better P2 has `proposed != current` ⇒ NOT
    // in `unchanged` ⇒ stays NonCompliant (mirrors `proposed_compliance_status`'s `proposed==current`
    // rule — symmetric with the C2 standing-order fix).
    let unchanged: BTreeSet<EventId> = row_meta
        .iter()
        .filter(|(_, _, _, current, proposed)| proposed == current)
        .map(|(id, ..)| id.clone())
        .collect();
    let rows = compliance_overlay(&rows, attested, &unchanged);

    let per_disposal: Vec<DisposalProposal> = row_meta.into_iter().zip(rows).map(
        |((id, wallet, date, current, proposed), row)| DisposalProposal {
            disposal: id,
            wallet: wallet.clone(),
            date,
            current_selection: current,
            proposed_selection: proposed,
            status: row.status,
            // R0-C2/N2: the REAL made-date governs persistability — only genuinely-contemporaneous
            // picks (made ≤ sale) are persistable; 2027+ broker NEVER. `proposal_made` is the actual
            // made-date threaded from the CLI seam (NOT a "conservative stand-in" — the old `date,date`
            // call returned ContemporaneousNow for EVERY disposal, the least-conservative verdict).
            persistable: persistability(&wallet, date, proposal_made),
        }).collect();

    Ok(OptimizeProposal {
        year,
        baseline_tax: base.total_federal_tax_attributable,
        optimized_tax: opt.total_federal_tax_attributable,
        delta: opt.total_federal_tax_attributable - base.total_federal_tax_attributable,
        per_disposal,
        marginal_rates: opt.marginal_rates,
        approximate,
        approx_reason,
    })
}

/// The lots the CURRENT projection consumes for `disposal` (its baseline disposal legs), as picks.
fn baseline_selection(state: &LedgerState, disposal: &EventId) -> Vec<LotPick> {
    let mut picks: Vec<LotPick> = state
        .disposals
        .iter()
        .find(|d| &d.event == disposal)
        .map(|d| d.legs.iter().map(|l| LotPick { lot: l.lot_id.clone(), sat: l.sat }).collect())
        .unwrap_or_default();
    picks.sort_by(|a, b| a.lot.cmp(&b.lot));
    picks
}

/// Exhaustive cartesian-product minimisation over the per-GROUP candidate lists. Each combination
/// merges one chosen partial-map per group into a single `BTreeMap<EventId, Vec<LotPick>>`. Skips
/// `NotComputable` combinations (infeasible cross-disposal contention). Ties → lexicographically
/// smallest assignment (NFR4 §0 total order).
fn exhaustive_min(/* events, prices, config, year, profile, tables,
                     group_lists, baseline_assignment, base */) -> BTreeMap<EventId, Vec<LotPick>> {
    // R0-C1: SEED the incumbent with `baseline_assignment` scored at `base.total_federal_tax_attributable`
    // (the current filing position), so the returned optimum can NEVER be worse than the baseline.
    // Iterative odometer over group_lists indices (no recursion). For each combination:
    //   merge the chosen per-group partial maps → assignment: BTreeMap<EventId, Vec<LotPick>>;
    //   match score_assignment(..) { Computed(r) => consider r.total_federal_tax_attributable; _ => skip };
    //   keep (min total, then min assignment-key) vs the baseline-seeded incumbent.
    // Returns the winning assignment (ties resolve to the baseline only if nothing strictly beats it).
    unimplemented!("implemented in this task")
}

/// Deterministic, BASELINE-SEEDED coordinate descent for huge products (KATs reach it only via the
/// dedicated R0-C1 fallback KAT). R0-C1: START from `baseline_assignment` (NOT all-HIFO), so the
/// incumbent is the current filing position and `optimized_tax ≤ baseline_tax` holds even if every
/// candidate basin is worse than baseline. Then per disposal in EventId order pick its best candidate
/// (holding others fixed) by full-year score, accepting a move ONLY if it strictly lowers the total;
/// iterate to a fixed point (bounded passes). No float, no RNG, no clock.
fn coordinate_descent(/* events, prices, config, year, profile, tables,
                         group_lists, baseline_assignment, base */) -> BTreeMap<EventId, Vec<LotPick>> {
    unimplemented!("implemented in this task")
}
```
> **R0-C2 made-date threading.** `proposal_made` (the proposed picks' made-date) is the `Date::now()`
> tax-date derived **at the CLI seam** and passed into `optimize_year` (`run` and `accept` both pass it);
> core never reads a clock (NFR4). Every `DisposalProposal.status`/`.persistable` is computed against
> THIS real made-date and the disposal's sale date + the 2025–2026/2027+ broker envelope — so `run` and
> `accept` agree, and a post-hoc proposed pick is `NonCompliant`/not-persistable in the **proposal the
> user acts on**, not only when `accept` recomputes. `proposed_compliance_status`, `persistability`,
> and `compliance_overlay` are the pure Task-5 functions (defined there with their KATs); `optimize_year`
> calls them, so Task 4's compliance wiring depends on Task 5's pure helpers landing first.

**Steps**
1. **RED** — `tests/optimize_mode1.rs`. Each KAT builds a synthetic ledger + synthetic 2025 `TaxTables` + profile and an **independent exhaustive oracle** helper:
```rust
/// Oracle: enumerate ALL whole-lot subset assignments per disposal, score each via score_assignment,
/// return the min total. Independent of the optimizer's generators (proves optimality, not just self-consistency).
fn oracle_min_total(/* events, prices, cfg, year, profile, tables, per-disposal whole-lot subsets */) -> Usd { /* ... */ }
```
   KATs (each small enough to brute-force; assert the optimizer's `optimized_tax == oracle_min_total` AND that it beats the named naive baseline):
   - **HIFO-beats-FIFO.** One wallet, lots {low-basis old, high-basis old}, one all-LT `Sell`. `optimized_tax < baseline_tax` (FIFO baseline picks the low-basis lot → more gain); optimum == oracle; proposed picks the high-basis lot.
   - **Rate-awareness: naive-HIFO LOSES to an LT pick.** One wallet with a **short-term** high-basis lot and a **long-term** lower-basis lot; profile placed so the ST marginal rate (ordinary) far exceeds the 15% LT rate. Naive HIFO would take the ST high-basis lot (smaller gain but taxed at the high ordinary rate); the optimum takes the LT lot (slightly larger gain taxed at 15%) for a **lower total tax**. Assert `optimized_tax == oracle_min_total` and that it is **strictly less** than the all-HIFO assignment's score (so rate-awareness, not just basis-greed, is proven).
   - **Loss-harvest within the $3k limit (R0-I3 — assert ONLY what the single-year objective pins).** A gain disposal + a wallet holding loss lots. The single-year objective is `total_federal_tax_attributable`, and `net_1222` caps the current deduction at `loss_limit` (compute.rs:174-178): once a selection offsets all gains and takes the $3,000 §1211 offset, **any** additional realized loss only grows `carryforward_out` — the objective is **identical**. So "harvest exactly enough" and "over-harvest" are **ties** on the objective, broken lexicographically (§0) — NOT by minimal harvest. Therefore assert ONLY: (a) `optimized_tax == oracle_min_total` and `optimized_tax < baseline_tax`; (b) the in-year `loss_deduction == $3,000` (gains fully offset + the §1211 cap taken), read via a follow-up `score_assignment` on `best`. Do **NOT** assert a specific `carryforward_out` split — the objective does not pin it (the original KAT's hand-derived carryforward figure was unsound). Inline note: the single-year objective is **carryforward-blind**; the multi-year "don't waste high-basis lots" intuition is out of scope and disclosed (§4 caveat + FOLLOWUPS). Making harvest uniquely determined would need a documented **secondary** objective (a specced scope change), not assumed here.
   - **Per-wallet constraint respected.** Two wallets; the *globally* cheapest lot lives in the *other* wallet. The optimizer must **not** pick it (cross-account ID forbidden, §1.1012-1(j)); the optimum equals the oracle restricted to the disposal's own wallet, and the cross-wallet lot never appears in any `proposed_selection`.
   - **R0-C1 approximate fallback (baseline-seeded, delta ≤ 0).** A fixture whose per-group product exceeds `MAX_COMBOS` (use a `#[cfg(test)]` bound override or enough lots) drives `coordinate_descent`. Assert `proposal.approximate == true`, `approx_reason == Some(ApproxReason::ComboCapExceeded { .. })`, and `delta ≤ 0` — the baseline-seed guarantees the result never worsens the baseline even if every candidate basin is worse. A within-bound fixture asserts `approximate == false` and `approx_reason == None`. (Adversarial: a fixture where all-HIFO is strictly worse than baseline confirms the OLD all-HIFO seed would have returned `delta > 0`; the baseline seed must not.)
   - **R2-C1 pool-heuristic disclosure (both ways).** A **single** disposal drawing from a pool with **> `LOT_ENUM_BOUND`** lots (e.g. 20 — a weekly-DCA / active-trading pool; use enough fixture lots, no bound override needed since `LOT_ENUM_BOUND` is the trigger). `candidate_selections` returns the heuristic subset, so the overall `product` is small (`≪ MAX_COMBOS`) yet the result is NOT proven. Assert `proposal.approximate == true`, `proposal.approx_reason == Some(ApproxReason::PoolHeuristic { lots: 20, bound: 12 })`, and `delta ≤ 0` (baseline-seeded — never worse than current). The **mirror**: a disposal over a **≤ `LOT_ENUM_BOUND`** pool with a small overall product → `approximate == false`, `approx_reason == None` (fully enumerated ⇒ proven global minimum). Together these pin `approximate == false ⇔ fully-enumerated-global`.
   - **R0-C3 contended intra-year sells across an ST/LT crossover.** Two same-wallet sells `D1` (earlier) and `D2` (later) in one year, with a lot ST at `D1`'s date but LT at `D2`'s date. The true optimum reassigns that lot from `D1` (ST/ordinary) to `D2` (LT/15%) — a sequence the **independent** per-disposal product cannot reach. Within `GROUP_COMBO_BOUND`: assert the optimizer finds the **joint** optimum (`optimized_tax == oracle_min_total` where the oracle enumerates the JOINT assignment space, not independent per-disposal) and `approximate == false`. A variant forcing the group past the bound: assert `approximate == true`, `approx_reason == Some(ApproxReason::ContentionUnenumerated { .. })`, and still `delta ≤ 0`.
   - **Refusals.** `optimize_year` over a year with an unresolved Hard blocker → `Err(YearNotComputable(_))`; over `year = 2024` → `Err(PreTransitionYear(2024))`; over a clean year with no disposals → `Err(NoDisposals)`.
   - **Determinism.** Two calls → byte-identical `OptimizeProposal` (derive `PartialEq`); also assert the all-equal-tax tie fixture returns the lexicographically-smallest assignment.
   - **Compliance/persistability rows** are KAT-tested in Task 5 (which owns the pure `proposed_compliance_status`/`persistability`/`compliance_overlay`), incl. the R0-C2 divergent-post-hoc-pick KAT.
   → **RED**.
2. **GREEN** — implement `optimize_year` (group-based, baseline-seeded search + `approximate`/`approx_reason`), `contention_groups`, `group_candidate_assignments`, `independent_group_maps`, `exhaustive_min` (odometer over group lists), `coordinate_descent` (baseline-seeded), `baseline_selection`. Tests pass.
3. **Full suite → review-to-green → commit** `feat(core): Mode-1 holistic optimizer optimize_year (baseline-seeded, contention-aware)`.

---

## TASK 5 — Compliance + persistability overlay (`AttestedRecording` + envelope/timing gates)

**Goal.** §A.5/§C.2: confer `AttestedRecording` for a narrowly-attested post-hoc selection **within the permitted envelope**, and compute the `accept` gate verdict. A's `disposal_compliance` is unchanged; C owns the overlay.

**Files**: modify `crates/btctax-core/src/optimize.rs` (overlay + `persistability` + wire into `optimize_year`'s `DisposalProposal`); new test `crates/btctax-core/tests/optimize_compliance.rs`.

**Interfaces (produces)**
```rust
use crate::project::{ComplianceStatus, DisposalCompliance};

/// §A.5 custody → envelope (R2-M5): Exchange = broker (own-books insufficient 2027+);
/// SelfCustody = own-books, all years, no relief ever needed.
fn is_broker(w: &WalletId) -> bool { matches!(w, WalletId::Exchange { .. }) }

/// The §C.2 accept-gate verdict for one disposal.
/// - made-date ≤ sale → `Contemporaneous` lever (A.5(b)); persist freely (`ContemporaneousNow`).
/// - already-executed (made-date > sale) AND 2027+ broker-held → `ForbiddenBroker2027` (NEVER persist).
/// - already-executed otherwise (self-custody any year, or broker-held 2025–2026) → `NeedsAttestation`.
pub fn persistability(wallet: &WalletId, sale_date: TaxDate, selection_made: TaxDate) -> Persistability {
    if selection_made <= sale_date {
        Persistability::ContemporaneousNow
    } else if is_broker(wallet) && sale_date.year() >= 2027 {
        Persistability::ForbiddenBroker2027
    } else {
        Persistability::NeedsAttestation
    }
}

/// R0-C2: compliance status of a PROPOSED pick, judged by ITS OWN timeliness. The proposed pick is a
/// would-be `LotSelection` made at `made` (= proposal/now), NOT a persisted selection in `events`;
/// `disposal_compliance(events, …)` would skip the selection branch and let a standing order RESCUE a
/// divergent post-hoc cherry-pick as `StandingOrder` (FORBIDDEN — §1.1012-1(j)). This judges it directly:
/// - `proposed == current` (no change): keep `baseline_status` — adopting an identical pick binds nothing
///   new (`accept` skips it), so the disposal's genuine `StandingOrder`/`Contemporaneous`/`NonCompliant`
///   stands. **This is the ONLY path that may report `StandingOrder`; the divergent path never does.**
/// - diverges: 2027+ broker → `NonCompliant` (own-books insufficient); else `made ≤ sale` →
///   `Contemporaneous`; else (post-hoc) → `NonCompliant`. The overlay may later upgrade a post-hoc
///   `NonCompliant` to `AttestedRecording` ONLY when attested AND within the own-books envelope.
pub fn proposed_compliance_status(
    wallet: &WalletId,
    sale_date: TaxDate,
    made: TaxDate,
    proposed: &[LotPick],
    current: &[LotPick],
    baseline_status: &ComplianceStatus,
) -> ComplianceStatus {
    if proposed == current {
        return baseline_status.clone(); // no divergence ⇒ the real, already-established status stands
    }
    if is_broker(wallet) && sale_date.year() >= 2027 {
        return ComplianceStatus::NonCompliant;
    }
    if made <= sale_date {
        ComplianceStatus::Contemporaneous
    } else {
        ComplianceStatus::NonCompliant // divergent post-hoc cherry-pick — NEVER StandingOrder
    }
}

/// Upgrade A's per-disposal compliance for the `optimize` surface: a disposal carrying an applied
/// selection that A marks `NonCompliant` PURELY because the selection is post-hoc (made-date > sale)
/// is upgraded to `AttestedRecording` IFF (a) it is in `attested`, (b) it is within the envelope
/// (NOT 2027+ broker-held — that path can never be attested), AND (c) it is in `unchanged` — the
/// PROPOSED pick equals the in-force (persisted-and-attested) selection (R2-I1).
///
/// **Why (c) (R2-I1).** The attestation side-table is keyed by DISPOSAL, not by selection. Without (c),
/// after the user attests+persists pick `P1` for disposal X, a later re-`run` that finds a strictly
/// better DIVERGENT pick `P2 ≠ P1` would be laundered as `AttestedRecording` (compliant) — even though
/// `P2` was never attested and `proposed_compliance_status` correctly returns `NonCompliant` for it.
/// Gating on `unchanged` makes the attestation confer `AttestedRecording` ONLY on the exact selection
/// that was attested (when `proposed == current == P1`), keeping the overlay SYMMETRIC with the C2
/// fix (a standing order never rescues a divergent pick; an attestation now likewise cannot).
/// `StandingOrder`/`Contemporaneous` are left untouched; a NON-attested OR divergent post-hoc selection
/// stays `NonCompliant` (the conservative direction).
pub fn compliance_overlay(
    base: &[DisposalCompliance],
    attested: &BTreeSet<EventId>,
    unchanged: &BTreeSet<EventId>, // R2-I1: disposals whose PROPOSED pick == the in-force (persisted &
                                   // attested) current selection — the ONLY rows eligible for upgrade.
) -> Vec<DisposalCompliance> {
    base.iter()
        .map(|c| {
            let upgrade = matches!(c.status, ComplianceStatus::NonCompliant)
                && attested.contains(&c.disposal)
                && unchanged.contains(&c.disposal) // R2-I1: attestation binds only the attested pick
                && !(is_broker(&c.wallet) && c.date.year() >= 2027);
            let mut out = c.clone();
            if upgrade {
                out.status = ComplianceStatus::AttestedRecording;
            }
            out
        })
        .collect()
}
```
**Wiring (R0-C2 + R2-I1 — corrected).** `optimize_year` (Task 4) builds each row's status via
`proposed_compliance_status(wallet, date, proposal_made, &proposed, &current, &baseline_status)` over A's
**baseline** `disposal_compliance(events, &baseline_state)`, then applies
`compliance_overlay(&rows, attested, &unchanged)` — where `unchanged` = the disposals whose `proposed ==
current` (R2-I1, so an attestation upgrades ONLY the exact persisted-and-attested selection, never a
divergent re-run pick) — and sets `persistable = persistability(wallet, date, proposal_made)` — the
**real** made-date threaded from the CLI seam. The plan no longer calls `disposal_compliance(events,
&opt_state)` (it reads `events`, which LACKS the proposed pick → a divergent post-hoc pick would fall
through to `StandingOrder`), and no longer uses `persistability(wallet, date, date)` (which returned
`ContemporaneousNow` for EVERY disposal — the least-conservative verdict, R0-N2). There is no
`let _ = attested;`.

**Steps**
1. **RED** — `tests/optimize_compliance.rs`:
   - `persistability`: self-custody, made-date ≤ sale → `ContemporaneousNow`; self-custody, made-date > sale, any year → `NeedsAttestation`; broker, sale 2026, made-date > sale → `NeedsAttestation`; **broker, sale 2027, made-date > sale → `ForbiddenBroker2027`**; broker, sale 2027, made-date ≤ sale → `ContemporaneousNow` (a genuinely contemporaneous standing instruction is fine even 2027+, but C never auto-reaches this for already-executed disposals).
   - `compliance_overlay` (R2-I1 — now also gated on `unchanged`, the disposals whose proposed pick == the in-force/persisted-attested selection): a `NonCompliant` self-custody disposal in `attested` **and** `unchanged` → `AttestedRecording`; the same disposal **not** in `attested` (but in `unchanged`) → stays `NonCompliant`; **in `attested` but NOT in `unchanged`** (a divergent re-run pick) → **stays `NonCompliant`** (an attestation binds only the exact attested selection — R2-I1); a 2027+ **broker** `NonCompliant` disposal in `attested`+`unchanged` → **stays `NonCompliant`** (envelope forbids); a `Contemporaneous` row → unchanged (no spurious downgrade/upgrade); a `StandingOrder` row → unchanged.
   - **`proposed_compliance_status` (R0-C2):** (i) `proposed == current` → returns `baseline_status` verbatim (e.g. a genuine `StandingOrder` is preserved when the optimizer changes nothing); (ii) **diverges**, self-custody, `made > sale` → `NonCompliant` (NOT `StandingOrder`, even with an in-force election); (iii) diverges, `made ≤ sale` → `Contemporaneous`; (iv) diverges, 2027+ broker, any `made` → `NonCompliant`.
   - **End-to-end R0-C2 (through `optimize_year`):** a fixture with an **in-force `MethodElection`** (standing order) and an **already-executed** disposal (sale date < `proposal_made`) where the optimizer's proposed pick **diverges** from the standing-order-dictated lots. Assert the `DisposalProposal` row is `status == NonCompliant` (the standing order does NOT rescue it) and `persistable == NeedsAttestation` (i.e. NOT `ContemporaneousNow` — not freely persistable). A second assertion: **every** already-executed disposal in the proposal is NOT `ContemporaneousNow` (kills the old `persistability(date,date)` bug).
   - **End-to-end R2-I1 (attestation binds only the attested selection, through `optimize_year`):** attest disposal X for pick `P1` (write the `optimize_attestation` row) and persist `P1` as a `LotSelection` so the **baseline/current** selection for X is `P1`. (i) A no-change re-`run` (proposed `== P1 == current`, so X ∈ `unchanged`) → X's row is `AttestedRecording` (the legitimate attested selection). (ii) Then introduce a change (e.g. a newly-acquired lower-basis lot) so the re-`run`'s proposed pick for X is a strictly-better **divergent** `P2 ≠ P1` (X ∉ `unchanged`): assert X's row is `status == NonCompliant` — **NOT** `AttestedRecording` — even though X ∈ `attested`. The attestation does not launder the new, never-attested pick.
   → **RED**.
2. **GREEN** — implement `is_broker`, `persistability`, `proposed_compliance_status`, `compliance_overlay`; wire into `optimize_year` (Task 4) per the corrected wiring. Tests pass.
3. **Full suite → review-to-green → commit** `feat(core): proposed-pick compliance + AttestedRecording overlay + persistability gate`.

---

## TASK 6 — Mode-2 consult `consult_sale` (synthetic disposal + ST→LT timing)

**Goal.** §C.3 read-only pre-trade consultation: pick the tax-minimizing lots for a **hypothetical** sale, report ST/LT + incremental federal tax + the ST→LT timing insight; require `--proceeds` for future dates; **mutate nothing**.

**Files**: modify `crates/btctax-core/src/optimize.rs`; new test `crates/btctax-core/tests/optimize_mode2.rs`.

**Interfaces (consumes A's `evaluate_disposal` + B; produces)**
```rust
use crate::conventions::one_year_after;
use crate::event::DisposeKind;
use crate::project::{evaluate_disposal, CandidateDisposal};

pub fn consult_sale(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year_profile: Option<&TaxProfile>, // CLI loads the year's profile; keeps core clock-free
    tables: &dyn TaxTables,
    req: &ConsultRequest,
) -> Result<ConsultReport, OptimizeError> {
    let year = req.at.year();
    if year < TRANSITION_DATE.year() {
        return Err(OptimizeError::PreTransitionYear(year));
    }
    // The year's profile must exist + the base year must be computable (I6). We score the synthetic
    // disposal by APPENDING it to the timeline and folding (clone-fold-discard, mirroring evaluate.rs).
    // R0-M3: available lots = the wallet pool AS OF `at`. The end-of-timeline pool equals the as-of-`at`
    // pool ONLY when `at` ≥ the latest ledger event (consult is forward-looking, so the normal case);
    // for an interleaved/past `at` the end pool is WRONG (lots later disposed are missing). So compute
    // the pool by folding the canonical timeline TRUNCATED to events with `date() ≤ at` (same
    // `sort_canonical` + transition partition as R0-I1) — correct for ALL `at`, removing the caveat.
    let pre = fold_as_of(events, prices, config, req.at); // truncate-at-`at` clone-fold (mirrors R0-I1)
    let want = pool_key(req.at, &req.wallet);
    let mut lots: Vec<Lot> = pre.lots.into_iter()
        .filter(|l| l.remaining_sat > 0 && pool_key(req.at, &l.wallet) == want && l.acquired_at <= req.at)
        .collect();
    lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
    if lots.iter().map(|l| l.remaining_sat).sum::<Sat>() < req.sell_sat {
        return Err(OptimizeError::NoLots);
    }

    let candidate = CandidateDisposal {
        existing_event: None, // synthetic (Mode-2)
        wallet: req.wallet.clone(),
        date: req.at,
        sat: req.sell_sat,
        kind: req.kind,
        proceeds: req.proceeds, // None on a future date → ProceedsRequired (see below)
    };
    // Resolve proceeds once up front so a missing future price fails fast with ProceedsRequired.
    if req.proceeds.is_none() && crate::price::fmv_of(prices, req.at, req.sell_sat).is_none() {
        return Err(OptimizeError::Evaluate(EvaluateError::ProceedsRequired));
    }

    // Enumerate candidate selections for the synthetic disposal and score each via the synthetic
    // evaluate+compute path; pick the deterministic minimum federal tax. R2-C1: Mode-2 reports a what-if
    // tax-min selection, NOT a "proven global minimum" claim (`ConsultReport` has no `approximate`
    // field and `render_consult` never says "the optimum"), so the heuristic flag is not surfaced here —
    // it governs `OptimizeProposal` (Mode-1), which is what R2-C1 scopes.
    let (cands, _heuristic) = candidate_selections(&lots, req.sell_sat);
    let mut best: Option<(Usd, Vec<LotPick>, Usd, Usd)> = None; // (total, picks, st, lt)
    for picks in &cands {
        // year_profile is required for the full-year compute (I6); passed through to keep core clock-free.
        let (st, lt, total) = score_synthetic(events, prices, config, year_profile, tables, &candidate, picks)?;
        let cand = (total, picks.clone(), st, lt);
        best = Some(match best {
            None => cand,
            Some(b) if (cand.0, &cand.1) < (b.0, &b.1) => cand, // min tax, tie → smallest picks
            Some(b) => b,
        });
    }
    let (total, proposed_selection, st_gain, lt_gain) = best.ok_or(OptimizeError::NoLots)?;

    // ST→LT timing insight: lots in the chosen selection that are short-term as of `at`. R0-I4: returns
    // `None` (omit) — NEVER `Err` — when the crossover lands outside `at`'s bundled year/profile, so an
    // unbundled crossover year degrades gracefully instead of failing the whole consult.
    let timing = timing_insight(events, prices, config, year_profile, tables,
                                &candidate, &proposed_selection, &lots, total);

    Ok(ConsultReport { req: req.clone(), proposed_selection, st_gain, lt_gain,
                        total_federal_tax_attributable: total, timing })
}

/// As-of-`at` pool: fold the canonical timeline TRUNCATED to events with `date() ≤ at` (R0-M3),
/// reusing the R0-I1 ordering (`sort_canonical` + transition partition) so truncation is by TIME, not
/// load order. Sibling of `available_lots_before` (which truncates before a specific disposal id).
fn fold_as_of(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig, at: TaxDate)
    -> LedgerState { unimplemented!("implemented in this task") }

/// Score one synthetic-disposal selection: append the synthetic Op::Dispose to a clone of the
/// resolution, inject the selection, fold, run compute_tax_year for `at.year()` with `year_profile`.
/// Returns (st_gain, lt_gain, total_federal_tax_attributable). Uses A's evaluate_disposal for the
/// per-leg ST/LT split and a parallel fold for the year tax (both clone-fold-discard; no mutation).
fn score_synthetic(/* events, prices, config, year_profile, tables, candidate, picks */)
    -> Result<(Usd, Usd, Usd), OptimizeError> {
    // 1) ST/LT split for THIS disposal via A's side-effect-free entrypoint:
    //    let out = evaluate_disposal(events, prices, config, candidate, Some(picks))
    //                  .map_err(OptimizeError::Evaluate)?;
    //    (out.st_gain, out.lt_gain)
    // 2) Full-year federal tax: rebuild the resolution, append the SAME synthetic Op::Dispose
    //    (mirror evaluate.rs:135-153), inject {synthetic_id => picks}, fold, compute_tax_year.
    //    The synthetic disposal's profile is the stored TaxProfile for the year (CLI passes it in via
    //    a profile arg — see note); refuse YearNotComputable.
    unimplemented!("implemented in this task")
}
```
> **Profile threading.** `consult_sale` needs the year's `TaxProfile`. To keep core clock-free and side-effect-free, the CLI loads it and passes `Option<&TaxProfile>` into `consult_sale` (extend the signature to `consult_sale(events, prices, config, year_profile, tables, req)`). A missing profile → the underlying `compute_tax_year` returns `TaxProfileMissing` → `OptimizeError::YearNotComputable`.

**Timing insight** (`timing_insight`) — **returns `Option<TimingInsight>` (omit, never error — R0-I4/M4).** For each pick in the chosen selection, find the source lot; it is short-term iff `!is_long_term(lot.gain_hp_start(), at)` (`conventions.rs:65-67`). If none are short-term → `None`. Otherwise `st_sat_in_selection = Σ` their sats and `latest_crossover = max` over them of the first **strictly** long-term date = `one_year_after(gain_hp_start).next_day()`. **R0-M4:** `time::Date::next_day()` returns `Option` (Dec-31 / max-date edge) — if `None` for any contributing lot, OMIT the insight (`None`) rather than unwrapping. **R0-I4 (same year/profile, term-flip, degrade):** compute `tax_if_sold_long_term` **within the SAME tax year and profile as `at`** — re-score the SAME selection with the SAME proceeds, flipping only the short-term legs' term to long-term, realized as a synthetic disposal dated `latest_crossover` carrying `proceeds = Some(actual_proceeds)` (so ONLY the term changes, not the price). Do this **only when `latest_crossover.year() == at.year()`** AND `tables.table_for(at.year())` and `year_profile` are both present (lots already LT as of `at` stay LT, so this is exactly "the same selection sold long-term"). **OTHERWISE return `None`** — never re-score in a future crossover year: only TY2025 is bundled (`tax_tables.rs:48-54`), so a 2026+ re-score would `TaxTableMissing → NotComputable → Err` and fail the whole consult; degrade instead. `saving_if_waited = (total − tax_if_sold_long_term).max(0)`. Phrase the rendered line as a **tax consequence**, never "you should wait/sell."

**Steps**
1. **RED** — `tests/optimize_mode2.rs`:
   - **Tax-min lots.** Wallet with a low-basis and a high-basis lot; consult to sell one lot's worth → `proposed_selection` is the high-basis lot; `st_gain`/`lt_gain`/`total` match a hand-derived figure.
   - **ST→LT timing insight (R0-I4 — same-year crossover).** `at` and `latest_crossover` in the SAME bundled tax year: a lot ST as of `at` that crosses to LT later **that same year**; the chosen selection includes it → `timing` is `Some`, `latest_crossover` equals the hand-computed crossover date, and `saving_if_waited == total_now − tax_if_sold_long_term` where `tax_if_sold_long_term` is the same selection/proceeds scored with the term flipped (same year/profile/table). A purely-LT fixture → `timing == None`.
   - **R0-I4 degrade (crossover into an unbundled year).** A lot whose `latest_crossover` lands in a year with **no bundled table/profile** (e.g. 2026+): `consult_sale` still returns `Ok(ConsultReport)` with `timing == None` (omitted) — it does NOT `Err`. Asserts the degrade-not-error contract directly.
   - **R0-M4 (next_day edge).** A lot whose `gain_hp_start` anniversary makes `one_year_after(start).next_day()` `None` (Dec-31/max-date edge) → `timing == None`, no panic/unwrap.
   - **R0-M3 (as-of-`at` pool).** An interleaved fixture where a later acquisition and a later disposal both exist after `at`: assert the consult pool reflects holdings **as of `at`** (a lot acquired after `at` is excluded; a lot disposed after `at` is still available at `at`), distinguishing `fold_as_of` from the end-of-timeline pool.
   - **`--proceeds` required for a future date.** `at` is a future date with **no** dataset price and `proceeds = None` → `Err(Evaluate(ProceedsRequired))`; supplying `proceeds = Some(..)` → `Ok`.
   - **Never writes events.** Snapshot `events.len()` (and the `Vec<LedgerEvent>` itself) before/after `consult_sale`; assert **unchanged** (the function takes `&[LedgerEvent]` and only clone-folds — a structural guarantee, asserted explicitly).
   - **Determinism.** Two calls → identical `ConsultReport`.
   → **RED**.
2. **GREEN** — implement `consult_sale`, `fold_as_of` (R0-M3), `score_synthetic` (reusing `evaluate_disposal` for the split + a parallel synthetic fold for the year tax, mirroring `evaluate.rs:135-153`), `timing_insight` (returning `Option<TimingInsight>`, same-year term-flip + degrade — R0-I4/M4). Tests pass.
3. **Full suite → review-to-green → commit** `feat(core): Mode-2 pre-trade consult + ST→LT timing`.

---

## TASK 7 — §1091 wash-sale documentation + monitoring (C.5)

**Goal.** Make the wash-sale posture explicit and load-bearing: crypto is exempt → the optimizer may freely select loss lots; document + monitor.

**Files**: modify `crates/btctax-core/src/optimize.rs` (module doc note); new test `crates/btctax-core/tests/optimize_wash_sale.rs`; append a monitoring line to `FOLLOWUPS.md`.

**Doc note (top of `optimize.rs`)**
```rust
//! ## §1091 wash sale (C.5) — crypto is currently EXEMPT.
//! §1091 disallows a loss only on "stock or securities"; the IRS treats convertible virtual currency
//! as property, not a security, and **no statute extending §1091 to crypto has been enacted** (only
//! recurring Greenbook/legislative proposals). The optimizer therefore selects loss lots **freely** —
//! loss harvesting is unconstrained, and a chosen loss is never disallowed/deferred here. Form 1099-DA
//! box 1i reports wash-sale disallowances only for assets that are in fact securities — not a change to
//! crypto. **MONITOR for enactment**; if §1091 is extended, loss-lot selection must add a disallowance
//! rule and this note must change in lockstep (FOLLOWUPS.md).
```

**Steps**
1. **RED** — `tests/optimize_wash_sale.rs`: a wallet with a clear **loss** lot and a gain disposal where harvesting the loss strictly lowers `total_federal_tax_attributable`; assert the optimizer's `proposed_selection` **includes the loss lot** and `optimized_tax < baseline_tax`, with **no** wash-sale blocker/adjustment anywhere in the proposal (loss applied in full up to the §1211 limit). → **RED** (until the loss-selection path + doc land; the optimizer from Task 4 already selects it — this task pins the *intent* with a dedicated regression KAT and the doc).
2. **GREEN** — add the doc note; confirm the KAT passes (it exercises Task-4 machinery). Append to `FOLLOWUPS.md`: *"Monitor §1091 crypto wash-sale enactment; if enacted, add loss-lot disallowance to `optimize` + update the C.5 doc note (lockstep)."*
3. **Full suite → review-to-green → commit** `docs(core): §1091 wash-sale exemption note + loss-harvest KAT`.

---

## TASK 8 — CLI attestation side-table (`optimize_attest`)

**Goal.** Persist the narrow contemporaneous-ID attestation per disposal (a projection input, modeled on `tax_profile`); expose it to the overlay. **No new event type.**

**Files**: new `crates/btctax-cli/src/optimize_attest.rs`; modify `crates/btctax-cli/src/lib.rs` (`pub mod optimize_attest;`), `crates/btctax-cli/src/session.rs` (init in `from_fresh_vault` + `optimize_attested_set` accessor).

**Interfaces (produces, mirrors `tax_profile.rs:16-81`)**
```rust
// crates/btctax-cli/src/optimize_attest.rs
use crate::CliError;
use btctax_core::EventId;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeSet;

pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS optimize_attestation \
         (disposal_event TEXT PRIMARY KEY, attestation TEXT NOT NULL, attested_at TEXT NOT NULL);",
    )?;
    Ok(())
}
/// Record a narrow attestation for `disposal` (canonical EventId string key). Upsert.
pub fn set(conn: &Connection, disposal: &EventId, attestation: &str, attested_at: &str) -> Result<(), CliError> {
    init_table(conn)?;
    conn.execute(
        "INSERT INTO optimize_attestation(disposal_event,attestation,attested_at) VALUES(?1,?2,?3) \
         ON CONFLICT(disposal_event) DO UPDATE SET attestation=excluded.attestation, attested_at=excluded.attested_at",
        rusqlite::params![disposal.canonical(), attestation, attested_at],
    )?;
    Ok(())
}
pub fn get(conn: &Connection, disposal: &EventId) -> Result<Option<String>, CliError> {
    init_table(conn)?;
    Ok(conn.query_row(
        "SELECT attestation FROM optimize_attestation WHERE disposal_event=?1",
        [disposal.canonical()], |r| r.get(0),
    ).optional()?)
}
/// All attested disposals as a parsed `BTreeSet<EventId>` (NFR4-stable; feeds `compliance_overlay`).
pub fn attested_set(conn: &Connection) -> Result<BTreeSet<EventId>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare("SELECT disposal_event FROM optimize_attestation ORDER BY disposal_event")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = BTreeSet::new();
    for r in rows { out.insert(crate::eventref::parse_event_id(&r?)?); }
    Ok(out)
}
```
`session.rs`: add `optimize_attest::init_table(vault.conn())?;` to `from_fresh_vault` (after `tax_profile::init_table`); add:
```rust
pub fn optimize_attested_set(&self) -> Result<std::collections::BTreeSet<btctax_core::EventId>, CliError> {
    crate::optimize_attest::attested_set(self.conn())
}
```

**Steps**
1. **RED** — unit tests in `optimize_attest.rs` (mirror `tax_profile.rs` tests): `set` then `get` round-trips; `attested_set` returns the keys parsed back to `EventId` in sorted order; `get`/`attested_set` on a tableless in-memory vault return empty (defensive `init_table`). → **RED**.
2. **GREEN** — implement the module + `lib.rs`/`session.rs` wiring. Tests pass.
3. **Full suite → review-to-green → commit** `feat(cli): optimize attestation side-table`.

---

## TASK 9 — CLI `optimize run` (read-only proposal)

**Goal.** §C.2 `optimize run --tax-year <y>` → print the what-if proposal (delta vs current + per-disposal compliance). **Binds nothing.**

**Files**: new `crates/btctax-cli/src/cmd/optimize.rs`; modify `cmd/mod.rs` (`pub mod optimize;`), `render.rs` (`render_optimize_proposal`), `main.rs` (clap `Optimize` + dispatch).

**Interfaces (produces)**
```rust
// crates/btctax-cli/src/cmd/optimize.rs
use crate::{CliError, Session};
use btctax_adapters::{BundledPrices, BundledTaxTables};
use btctax_core::conventions::tax_date;
use btctax_core::{optimize_year, OptimizeError, OptimizeProposal};
use btctax_store::Passphrase;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

/// `optimize run` — Mode 1 what-if. READ-ONLY: opens the vault, projects, optimizes, returns the
/// proposal. Appends/persists NOTHING. `now` is the CLI clock seam → the proposed picks' made-date
/// (R0-C2: core stays clock-free; the proposal the user reads is judged against the REAL made-date).
pub fn run(vault: &Path, pp: &Passphrase, year: i32, now: OffsetDateTime) -> Result<OptimizeProposal, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, _state, cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(year)?;
    let prices = BundledPrices::load()?;
    let tables = BundledTaxTables::load();
    let attested = s.optimize_attested_set()?;
    let proposal_made = tax_date(now, UtcOffset::UTC); // R0-C2: real made-date threaded into core
    let p = optimize_year(&events, &prices, &cfg, year, profile.as_ref(), &tables, &attested, proposal_made)
        .map_err(map_opt_err)?;
    // R0-C1: core has no logger — log the cap/why HERE (CLI seam) when the result is approximate.
    if p.approximate {
        eprintln!(
            "warning: optimize result is APPROXIMATE (not a guaranteed global minimum): {:?}",
            p.approx_reason
        );
    }
    Ok(p)
}

pub(crate) fn map_opt_err(e: OptimizeError) -> CliError {
    match e {
        OptimizeError::YearNotComputable(b) => CliError::Usage(format!(
            "year not computable — resolve the blocker first: [{:?}] {}", b.kind, b.detail)),
        OptimizeError::PreTransitionYear(y) => CliError::Usage(format!(
            "{y} is pre-2025: a pre-2025 selection restates a closed year — not an optimization (M7)")),
        OptimizeError::NoDisposals => CliError::Usage("no method-honoring disposals in that year".into()),
        OptimizeError::NoLots => CliError::Usage("no lots available to sell".into()),
        OptimizeError::Evaluate(ev) => CliError::Usage(format!("evaluate error: {ev:?}")),
    }
}
```
`render.rs`:
```rust
pub fn render_optimize_proposal(p: &btctax_core::OptimizeProposal) -> String {
    use btctax_core::{ApproxReason, ComplianceStatus, Persistability};
    let mut s = String::new();
    let _ = writeln!(s, "Optimize (what-if) — tax year {} — NOTHING is filed or bound by running this.", p.year);
    // R0-C1/C3: a non-fully-enumerated result is NEVER presented as "the optimum" without this banner.
    if p.approximate {
        let why = match p.approx_reason {
            Some(ApproxReason::ComboCapExceeded { combos, cap }) =>
                format!("input exceeded the exhaustive bound ({combos} combos > {cap}); a coordinate-descent fallback ran"),
            Some(ApproxReason::ContentionUnenumerated { contended, .. }) =>
                format!("{contended} contended same-wallet disposal(s) could not be fully joint-enumerated"),
            Some(ApproxReason::PoolHeuristic { lots, bound }) =>
                format!("a pool of {lots} lots exceeds the {bound}-lot exhaustive-enumeration bound; only a deterministic heuristic SUBSET of that pool's identifications was searched"),
            None => "approximate".to_string(),
        };
        let _ = writeln!(s, "  ⚠ APPROXIMATE — NOT a guaranteed global minimum: {why}.");
        let _ = writeln!(s, "    The true least-tax assignment may be lower; this is a disclosed improvement over your");
        let _ = writeln!(s, "    current filing position (delta ≤ 0), NOT 'the least tax.'");
    }
    let _ = writeln!(s, "  current federal tax (attributable): {}", p.baseline_tax);
    let _ = writeln!(s, "  optimized federal tax (attributable): {}", p.optimized_tax);
    let _ = writeln!(s, "  delta (optimized − current): {}  (negative = saving; always ≤ 0)", p.delta);
    for d in &p.per_disposal {
        let _ = writeln!(s, "  {} @ {} [{}] :: {:?}", d.disposal.canonical(), d.date,
                         crate::render::wallet_label(&d.wallet), d.status);
        // R2-M1: a NO-CHANGE row (proposed == current) has nothing to attest/persist — `accept` SKIPS it
        // ("already optimal under current identification"). Do NOT print a persistability line here: a
        // `NeedsAttestation` "needs --attest" line on a disposal the optimizer is NOT asking to change is
        // misleading and invites a pointless/contradictory attestation. Show a no-change note instead.
        if d.proposed_selection == d.current_selection {
            let _ = writeln!(s, "      proposed: {}  [no change — already optimal under current identification]",
                             picks_str(&d.proposed_selection));
            continue;
        }
        let persist = match d.persistable {
            Persistability::ContemporaneousNow => "persistable now (made ≤ sale → Contemporaneous)",
            Persistability::NeedsAttestation  => "already executed — needs `optimize accept --disposal <ref> --attest \"…\"` (genuine contemporaneous ID only)",
            Persistability::ForbiddenBroker2027 => "2027+ broker-held — CANNOT be persisted (own-books insufficient); FIFO is the defensible position",
        };
        let _ = writeln!(s, "      proposed: {}  [{}]", picks_str(&d.proposed_selection), persist);
    }
    // R0-M2: surface the vertex-granularity limitation in OUTPUT, not only in docs.
    let _ = writeln!(s, "  (vertex-granularity identification: a multi-partial split landing exactly on a tax-bracket kink is out of scope.)");
    let _ = writeln!(s, "  (this is the tax IF you had identified thus; adequate ID must exist by the time of sale — §1.1012-1(j))");
    s
}
```
`main.rs`: add `Optimize(Optimize)` to `Command` and an `Optimize` subcommand enum (`Run { #[arg(long)] tax_year: i32 }`, plus `Accept`/`Consult` stubbed in Tasks 10/11). Dispatch `Optimize::Run` → `cmd::optimize::run(vault, &pp, tax_year, now)` (threads the `now` seam — R0-C2) → `print!("{}", render::render_optimize_proposal(&p))`.

**Steps**
1. **RED** — `crates/btctax-cli/tests/optimize_run.rs` (temp vault + synthetic Coinbase CSV with a pre-2025 buy + 2025 buy + a 2025 sell that the optimizer can improve; set a `TaxProfile`; inject a fixed `now`). Assert: `cmd::optimize::run(.., now)` returns `Ok` with `delta <= 0`; **`load_all(conn).len()` is identical before and after `run`** (propose-doesn't-mutate); the rendered string contains "NOTHING is filed" and the per-disposal compliance status. **R0-C1:** an approximate fixture renders the "⚠ APPROXIMATE — NOT a guaranteed global minimum" banner and an exact fixture does NOT. **R2-C1:** a fixture whose target pool exceeds `LOT_ENUM_BOUND` renders the banner with the `PoolHeuristic` reason text ("exceeds the … exhaustive-enumeration bound; only a … heuristic SUBSET … was searched"). **R0-C2:** when the 2025 sell is already-executed relative to `now`, the rendered persistability is the already-executed line (NeedsAttestation), never "persistable now". **R2-M1:** a NO-CHANGE row (`proposed == current`) renders the "no change — already optimal" note and does NOT print the `NeedsAttestation` "needs --attest" line. → **RED**.
2. **GREEN** — implement `cmd::optimize.rs` (`run` only, threading `now`), `render_optimize_proposal` (incl. the approximate banner + R0-M2 caveat), clap `Optimize::Run` + dispatch (passing `now`). Tests pass.
3. **Full suite → review-to-green → commit** `feat(cli): optimize run (read-only proposal)`.

---

## TASK 10 — CLI `optimize accept` (gated persistence + revocation)

**Goal.** §C.2 persistence: recompute the optimum (deterministic), persist `LotSelection`s **only** behind the time-of-sale / attestation / envelope gates; **never** auto-attest; **refuse** 2027+ broker-held; revocable via existing `reconcile void`.

**Files**: modify `crates/btctax-cli/src/cmd/optimize.rs` (`accept`); `main.rs` (`Optimize::Accept` + dispatch threading `now`).

**Interfaces (produces)**
```rust
use btctax_core::{persistability, EventId, EventPayload, LotSelection, Persistability};
use btctax_core::persistence::append_decision;
use btctax_core::conventions::tax_date;
use time::{OffsetDateTime, UtcOffset};

/// The result of `optimize accept` — what was persisted vs skipped (for rendering).
pub struct AcceptOutcome {
    pub persisted: Vec<(EventId /*disposal*/, EventId /*decision*/, &'static str /*basis*/)>,
    pub skipped:   Vec<(EventId /*disposal*/, String /*reason*/)>,
}

/// `optimize accept` — apply the recomputed optimum, gated per disposal.
/// `only`: if `Some(disposal)`, restrict to that one disposal (the form that carries `--attest`).
/// `attestation`: the user's narrow contemporaneous-ID statement (required to persist an already-
/// executed disposal; the app NEVER fabricates it and refuses to persist a post-hoc selection without it).
pub fn accept(
    vault: &Path, pp: &Passphrase, year: i32,
    only: Option<&str>, attestation: Option<&str>, now: OffsetDateTime,
) -> Result<AcceptOutcome, CliError> {
    let mut session = Session::open(vault, pp)?;
    let (events, _state, cfg) = session.load_events_and_project()?;
    let profile = session.tax_profile(year)?;
    let prices = BundledPrices::load()?;
    let tables = BundledTaxTables::load();
    let attested = session.optimize_attested_set()?;
    let made = tax_date(now, UtcOffset::UTC); // the LotSelection's made-date (decisions are UTC)
    let only_id = only.map(crate::eventref::parse_event_id).transpose()?;
    // R0-M5: validate the --attest/--disposal precondition BEFORE recomputing or appending ANYTHING —
    // `--attest` requires a single `--disposal` scope (the app never invites a blanket false
    // attestation). Hoisting this above the loop means no disposal is appended before the guard fires.
    if attestation.is_some() && only_id.is_none() {
        return Err(CliError::Usage(
            "--attest must be scoped to ONE disposal via --disposal (no blanket attestation)".into()));
    }
    // R0-C2: judge the proposal against the REAL made-date (`made`), so `run` and `accept` agree.
    let proposal = optimize_year(&events, &prices, &cfg, year, profile.as_ref(), &tables, &attested, made)
        .map_err(map_opt_err)?;

    let mut out = AcceptOutcome { persisted: vec![], skipped: vec![] };
    for d in &proposal.per_disposal {
        if let Some(target) = &only_id { if &d.disposal != target { continue; } }
        // Nothing to persist if the proposed selection equals the current one.
        if d.proposed_selection == d.current_selection {
            out.skipped.push((d.disposal.clone(), "already optimal under current identification".into()));
            continue;
        }
        match persistability(&d.wallet, d.date, made) {
            Persistability::ForbiddenBroker2027 => {
                out.skipped.push((d.disposal.clone(),
                    "2027+ broker-held: own-books is insufficient; cannot persist (FIFO is the defensible position)".into()));
            }
            Persistability::ContemporaneousNow => {
                let id = persist_selection(&mut session, &d.disposal, &d.proposed_selection, now)?;
                out.persisted.push((d.disposal.clone(), id, "Contemporaneous"));
            }
            Persistability::NeedsAttestation => {
                // Already executed: refuse WITHOUT a narrow per-disposal attestation (never auto-attest).
                let Some(att) = attestation else {
                    out.skipped.push((d.disposal.clone(),
                        "already executed — re-run `optimize accept --disposal <ref> --attest \"<genuine contemporaneous ID>\"`".into()));
                    continue;
                };
                // Blanket-attest is already rejected up-front (R0-M5); here `only_id == d.disposal`,
                // so no append-before-guard can occur on the error path.
                let id = persist_selection(&mut session, &d.disposal, &d.proposed_selection, now)?;
                crate::optimize_attest::set(session.conn(), &d.disposal, att, &made.to_string())?;
                out.persisted.push((d.disposal.clone(), id, "AttestedRecording"));
            }
        }
    }
    session.save()?;
    Ok(out)
}

/// Append the LotSelection decision for one disposal (no save; caller batches the save).
fn persist_selection(session: &mut Session, disposal: &EventId, picks: &[btctax_core::LotPick], now: OffsetDateTime)
    -> Result<EventId, CliError> {
    let payload = EventPayload::LotSelection(LotSelection {
        disposal_event: disposal.clone(),
        lots: picks.to_vec(),
    });
    Ok(append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?)
}
```
> **Determinism contract.** `accept` calls the **same** `optimize_year` as `run`; by NFR4 the recomputed proposal is byte-identical, so `accept` persists exactly the selections `run` displayed (asserted in the KATs). The persisted `LotSelection` carries `fingerprint = None` (consistent with all decisions; `append_decision` passes `None`).

`main.rs`: `Optimize::Accept { #[arg(long)] tax_year: i32, #[arg(long)] disposal: Option<String>, #[arg(long)] attest: Option<String> }` → `cmd::optimize::accept(vault, &pp, tax_year, disposal.as_deref(), attest.as_deref(), now)` → render the persisted/skipped summary.

**Steps**
1. **RED** — `crates/btctax-cli/tests/optimize_accept.rs` (temp vault + synthetic fixtures; inject `now` so tests are deterministic):
   - **accept-mutates (Contemporaneous).** A 2025 self-custody disposal whose `disposed_at` is **on/after** the injected `now` (so made-date ≤ sale → `ContemporaneousNow`): `accept` (no `--attest`) persists a `LotSelection` (event count +1); a re-`run`/`verify` shows it `Contemporaneous`; the persisted picks equal the proposal's `proposed_selection`.
   - **accept refuses without attestation (already-executed).** A 2025 self-custody disposal with `disposed_at` **before** `now` (post-hoc): plain `accept` persists **nothing** for it and the skip reason mentions `--attest`; event count unchanged.
   - **attested → AttestedRecording.** The same already-executed disposal with `--disposal <ref> --attest "I identified these units at the time of sale in my books"` → persists the `LotSelection` **and** an `optimize_attestation` row; after which a re-`run`'s proposed pick equals the now-persisted (current) selection, so it is in `unchanged`, and `compliance_overlay(disposal_compliance(...), &attested_set(), &unchanged)` reports `AttestedRecording`. (R2-I1: assert the disposal is in `unchanged` here — the upgrade requires it.)
   - **refuse 2027+ broker-held.** A 2027 `WalletId::Exchange` disposal (post-hoc) → **never** persisted even with `--attest`; skip reason cites the 2027+ broker rule; no attestation row written.
   - **blanket-attest guard (R0-M5 — fires BEFORE any append).** `accept --attest "…"` **without** `--disposal` → `Err(Usage(... "no blanket attestation" ...))`, and **`load_all(conn).len()` is unchanged** (the guard is hoisted above the loop, so no `ContemporaneousNow` disposal is appended before it fires — no partial/abandoned writes).
   - **void revokes.** After persisting, `cmd::reconcile::void(<decision-id>)` → the disposal no longer reports a selection (A's resolve excludes voided `LotSelection`); the optimize proposal returns to baseline for it.
   - **determinism.** `run` then `accept` persist exactly the displayed selection (compare picks).
   → **RED**.
2. **GREEN** — implement `accept` + `persist_selection` + clap/dispatch + a render summary. Tests pass.
3. **Full suite → review-to-green → commit** `feat(cli): optimize accept (time-of-sale/attestation/envelope-gated persistence)`.

---

## TASK 11 — CLI `optimize consult` (read-only pre-trade what-if)

**Goal.** §C.3 `optimize consult --sell <sat> [--wallet <w>] [--at <date>] [--proceeds <usd>|--fmv]` → tax-min lots + ST/LT + federal tax + ST→LT timing. **No mutation.**

**Files**: modify `crates/btctax-cli/src/cmd/optimize.rs` (`consult`); `render.rs` (`render_consult`); `main.rs` (`Optimize::Consult` + dispatch).

**Interfaces (produces)**
```rust
use btctax_core::{consult_sale, ConsultReport, ConsultRequest, DisposeKind};

pub fn consult(
    vault: &Path, pp: &Passphrase,
    sell_sat: i64, wallet: btctax_core::WalletId, at: btctax_core::TaxDate,
    proceeds: Option<btctax_core::Usd>, kind: DisposeKind,
) -> Result<ConsultReport, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, _state, cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(at.year())?;
    let prices = BundledPrices::load()?;
    let tables = BundledTaxTables::load();
    let req = ConsultRequest { sell_sat, wallet, at, proceeds, kind };
    // consult_sale is READ-ONLY (clone-fold-discard); no save() is ever called.
    consult_sale(&events, &prices, &cfg, profile.as_ref(), &tables, &req).map_err(map_opt_err)
}
```
`render.rs` `render_consult`: print proposed lots, ST/LT gains, `total_federal_tax_attributable`, and — when `timing.is_some()` — a single tax-consequence line:
```
"  timing: {st_sat} sat of the best selection is short-term until {latest_crossover};
   selling on/after then would be taxed long-term, a ≈ {saving_if_waited} difference."
```
plus a footer: *"Tax decision-support only — consequences of a contemplated sale; not investment advice (no buy/sell/hold recommendation)."*
`main.rs`: `Optimize::Consult { #[arg(long)] sell: String, #[arg(long)] wallet: Option<String>, #[arg(long)] at: Option<String>, #[arg(long)] proceeds: Option<String>, #[arg(long)] fmv: bool }`. Parse `sell` (sat i64), `wallet` (`eventref::parse_wallet_id`; required — error if absent since the per-wallet pool is mandatory post-2025), `at` (`eventref::parse_date_arg`; default `now`'s UTC date), `proceeds` (`eventref::parse_usd_arg`). `--fmv` and `--proceeds` are mutually exclusive (clap `conflicts_with`); `--fmv` simply leaves `proceeds = None` (forces dataset FMV; future dates without a price → `ProceedsRequired`). Default `kind = DisposeKind::Sell`.

**Steps**
1. **RED** — `crates/btctax-cli/tests/optimize_consult.rs`:
   - **what-if output.** Wallet with a high- and low-basis lot; `consult --sell <one-lot> --wallet self:x --at <date with a price> --fmv` → `Ok`; proposed selection = high-basis lot; ST/LT + total match a hand-derived figure; rendered string contains the lots + total.
   - **ST→LT timing.** Fixture with a soon-to-cross ST lot → rendered output contains the timing line and the crossover date; a purely-LT fixture → no timing line.
   - **`--proceeds` required for future.** `--at <future date, no price>` without `--proceeds`/with `--fmv` → `Err(Usage)` mentioning proceeds; with `--proceeds <usd>` → `Ok`.
   - **never writes events.** `load_all(conn).len()` identical before/after `consult` (read-only).
   → **RED**.
2. **GREEN** — implement `consult` + `render_consult` + clap/dispatch. Tests pass.
3. **Full suite → review-to-green → commit** `feat(cli): optimize consult (read-only pre-trade what-if + timing)`.

---

## TASK 12 — Whole-diff review + full-suite green (Phase E gate)

**Goal.** The mandatory whole-branch review (`STANDARD_WORKFLOW.md`): the entire Sub-project-C diff reviewed independently to **0 Critical / 0 Important**, full validation green.

**Steps**
1. Run the full surface: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`. Paste evidence.
2. Persist the reviewer output verbatim to `design/reviews/whole-branch-review-optimizer-round-1.md` **before** folding; fold; **re-review after every fold including the last**.
3. Re-verify the §0 invariants hold across the final diff: **NFR4** (no `HashMap` iteration, no `Date::now`/RNG in core, deterministic tie-break — grep `HashMap`, `now_utc`, `rand` under `crates/btctax-core/src/optimize.rs`); **NFR5** (no `f32`/`f64` — grep); **event-sourcing** (Mode-1 emits only `LotSelection`; Mode-2 emits nothing; attestation is a side-table); **compliance boundary** (no string/path labels a post-hoc selection compliant; `ForbiddenBroker2027` is never persisted). **R0/R2 invariants:** (a) **no silent local optimum** — every path that sets `best` without full enumeration sets `approximate=true` + a reason: the coordinate-descent fallback (`ComboCapExceeded`), an un-enumerated contended group (`ContentionUnenumerated`), AND any disposal whose pool used the `> LOT_ENUM_BOUND` heuristic subset (`PoolHeuristic` — R2-C1; grep that `candidate_selections` returns its heuristic flag `(_, bool)` and that `optimize_year` ORs it into `approximate`/`pool_heuristic_lots`). `render_optimize_proposal` shows the banner whenever `approximate` (grep that no render path prints `optimized_tax` as "optimum"/"least tax" without the `approximate` guard, and that the banner `match` has a `PoolHeuristic` arm); (b) **delta ≤ 0** — the search is baseline-seeded (grep for the `baseline_assignment` seed in both `exhaustive_min` and `coordinate_descent`); (c) **no post-hoc-compliant proposed pick** — `optimize_year` builds status via `proposed_compliance_status`/`compliance_overlay` (NOT `disposal_compliance(events, &opt_state)`) and persistability via the real `proposal_made` (grep that `persistability(.., date, date)` and the `(events, &opt_state)` compliance call are GONE), AND `compliance_overlay`'s `AttestedRecording` upgrade is gated on `unchanged` (proposed == current) so a divergent re-run pick on an attested disposal stays `NonCompliant` (R2-I1; grep that `compliance_overlay` takes the `unchanged` arg and that `optimize_year` builds it from `proposed == current`); (d) `LotPick` derives `Ord` (grep `event.rs`).
4. Confirm `FOLLOWUPS.md` carries: the §1091 monitor (Task 7); the **vertex-granularity / multi-partial-kink** limitation (now ALSO disclosed in output — R0-M2); the **single-year-objective carryforward-blindness** note (R0-I3: the objective is indifferent to over-harvesting beyond the $3k cap; a multi-year "retain high-basis lots" secondary objective would be a specced scope change); the **contended-same-wallet beyond-`GROUP_COMBO_BOUND`** approximation (now detected + jointly enumerated within the bound, flagged `ContentionUnenumerated` beyond it — R0-C3); and the "wire `verify` to read the attestation side-table so `verify` also surfaces `AttestedRecording`" follow-up (C's overlay is authoritative on the `optimize` surface; `verify` currently shows the conservative A-level status).
5. **Commit** `chore(optimizer): Sub-project C whole-diff review green (0C/0I)`.

---

## 5. Self-review

### Spec coverage map (§C.1–C.5 + Cross-cutting + Legal)
| Spec item | Where covered |
|---|---|
| **C.1** holistic single-year, carryforward-linked; refuse `TaxYearNotComputable` | §4 (holistic scoring), Task 2 (`score_assignment` runs the whole year through `compute_tax_year` which applies §1222/§1211/§1(h)/NIIT), Task 4 (`YearNotComputable` refusal KAT) |
| **C.2** scope: assigns lots (specific-ID), no sell/hold advice | Task 1 module doc + Task 11 render footer ("not investment advice") |
| **C.2** objective: minimize `total_federal_tax_attributable` s.t. A.4 constraints, deterministic | §4, Task 2, Task 4 (optimality KATs vs oracle; per-wallet KAT; determinism KAT) |
| **C.2** Mode-1 propose → tax delta vs current; what-if by default; nothing bound by running; **never present a local/under-enumerated/heuristic-pool result as the optimum** | Task 4 (`baseline_tax`/`optimized_tax`/`delta ≤ 0` baseline-seeded; `approximate`/`approx_reason` incl. `PoolHeuristic` — R0-C1/C3, R2-C1), Task 9 (`run` read-only; propose-doesn't-mutate KAT; "NOTHING is filed" + APPROXIMATE banner with the `PoolHeuristic` arm) |
| **C.2** persist only behind narrow attestation; never auto-attest; revocable; surface honest `DisposalCompliance` | Task 5 (`proposed_compliance_status` + overlay + `persistability` — R0-C2), Task 8 (attestation side-table), Task 10 (accept gates + refuse-without-attestation + up-front blanket-attest guard + void revokes) |
| **C.2** `Contemporaneous` only if made-date ≤ sale; already-executed → attestation gate or `NonCompliant`; a standing order NEVER rescues a divergent post-hoc pick; **an attestation rescues ONLY the exact attested selection, never a divergent re-run pick (R2-I1)**; 2027+ broker can't persist | Task 5 (`persistability` + `proposed_compliance_status` — R0-C2 KATs: divergent-post-hoc ≠ StandingOrder, already-executed ≠ ContemporaneousNow; `compliance_overlay` gated on `unchanged` — R2-I1 KAT: divergent pick on an attested disposal stays `NonCompliant`), Task 10 (KATs: contemporaneous-now, refuse-without-attestation, refuse-2027-broker) |
| **C.2** commands `optimize run` / `optimize accept` | Tasks 9, 10 |
| **C.3** Mode-2 consult: tax-min lots + ST/LT + federal tax + ST→LT timing; `--proceeds` for future; via `evaluate_disposal`; no mutation/events | Task 6 (`consult_sale`, `timing_insight`, `ProceedsRequired`, never-writes KAT), Task 11 (CLI + render + read-only KAT) |
| **C.4** algorithm decided + justified; determinism/exactness acceptance criterion | §4 (candidate-gen + holistic exact scoring; bounds; coordinate-descent fallback), Tasks 3–4 |
| **C.5** §1091 crypto-exempt; free loss selection; documented + monitored | Task 7 (doc + loss-harvest KAT + FOLLOWUPS monitor); Task 4 (loss-harvest-within-$3k KAT) |
| **Cross-cutting** NFR4/NFR5; event-sourcing (Mode-1 decisions, Mode-2 nothing); no post-hoc-compliant; synthetic-only | §0, every task's tests; Task 12 grep checks |
| **Legal** custody map (Exchange=broker, SelfCustody=self) + 2025–2026 own-books / 2027+ broker | Task 5 (`is_broker`, envelope), Task 10 (2027-broker refusal) |

### Placeholder scan
The only `unimplemented!()` markers are in the **interface sketches** for `contention_groups`, `group_candidate_assignments` (Task 3), `exhaustive_min`, `coordinate_descent` (Task 4), and `fold_as_of`, `score_synthetic` (Task 6), each immediately followed by the prose describing the exact implementation; the task's GREEN step replaces them with real code. No silent stubs survive a task. Every other code block is complete and grounded in cited source.

### Type-consistency vs cited A/B source
- `score_assignment` reuses `resolve`/`Resolution.selections`/`fold` (resolve.rs:128-136, fold.rs:330) and `compute_tax_year` (compute.rs:221) verbatim — same signatures.
- Candidate/selection types are A's `LotPick`/`LotSelection`/`LotId` (event.rs:190-218, identity.rs:116) — **no new event type**. The sole A-side change is the **additive `PartialOrd, Ord` derive on `LotPick`** (R0-I2) needed by the `BTreeSet<Vec<LotPick>>` dedup and the total-order tie-break — serde-compatible, no behavior change (corrects the earlier "reuse verbatim / no A-side change" phrasing).
- Mode-2 uses A's `evaluate_disposal`/`CandidateDisposal`/`EvaluateError` (evaluate.rs:31-104) unchanged; `ProceedsRequired` is A's existing variant.
- Compliance reuses A's `ComplianceStatus` (incl. the reserved `AttestedRecording`) + `DisposalCompliance` (compliance.rs:18-33); the overlay does not modify A's shipped `disposal_compliance`.
- Tax outputs reuse B's `TaxResult`/`MarginalRates`/`TaxOutcome` (types.rs:54-96); C re-rounds nothing.
- CLI side-table mirrors `tax_profile.rs:16-81`; `append_decision` signature (persistence.rs:238) matches `persist_selection`'s call.

### No post-hoc-compliant path (confirmed — R0-C2)
The proposal the user reads is judged by the PROPOSED pick's own made-date, NOT by feeding `events` (which
lacks the proposed pick) to `disposal_compliance` — so a divergent post-hoc pick can **never** fall through
to `StandingOrder` (`proposed_compliance_status` reports `StandingOrder` ONLY when `proposed == current`,
i.e. no divergence; a divergent pick is judged by `made` vs `sale`). `persistability` returns
`ContemporaneousNow` **only** when `selection_made ≤ sale_date` (the REAL `proposal_made`, threaded from the
CLI seam — not the old `(date,date)` that returned `ContemporaneousNow` for every disposal). For any
already-executed disposal the verdict is `NeedsAttestation` (envelope-checked, requires a genuine
per-disposal attestation, never auto) or `ForbiddenBroker2027` (never persisted). `compliance_overlay`
upgrades to `AttestedRecording` **only** for a `NonCompliant` disposal that is attested, within the
own-books envelope, **and** `proposed == current` (R2-I1 — the proposed pick equals the in-force
persisted-and-attested selection) — never a 2027+ broker-held post-hoc selection, never a non-attested
post-hoc selection, and **never a divergent re-run pick on an attested disposal** (the attestation binds
only the exact attested selection; a later better divergent pick stays `NonCompliant`, symmetric with the
standing-order rule). No output string describes a post-hoc selection as compliant (Task 12 grep gate).

### No silent local optimum (confirmed — R0-C1/C3)
The search is **baseline-seeded** (incumbent starts at the current filing position), so `delta ≤ 0` always —
the optimizer never recommends worse-than-doing-nothing. A result is `approximate = false` ONLY when the
exhaustive search ran within `MAX_COMBOS`, every contended pool was jointly enumerated, **AND every target
disposal's pool was fully (non-heuristically) enumerated** (≤ `LOT_ENUM_BOUND`); otherwise `approximate =
true` with a structured `approx_reason` (`ComboCapExceeded` / `ContentionUnenumerated` / `PoolHeuristic` —
R2-C1), the CLI logs it, and `render_optimize_proposal` prints the "⚠ APPROXIMATE — NOT a guaranteed global
minimum" banner. So `approximate == false ⇔ fully-enumerated-global`, and no path renders a
non-fully-enumerated result (including a single `> LOT_ENUM_BOUND` pool) as "the optimum" (Task 12 grep gate).

### Mode-2 is side-effect-free (confirmed)
`consult_sale` takes `&[LedgerEvent]` and uses only clone-fold-discard (`fold_with`, `evaluate_disposal`); `cmd::optimize::consult` never calls `session.save()` and writes no side-table. A KAT asserts `load_all(conn).len()` is unchanged across a consult. No `Date::now`/RNG in core (the `at` date is supplied by the CLI seam).

---

## 6. Concise summary (for the R0 reviewer)

**Task list (titles):** (1) Core `optimize` module skeleton; (2) Holistic year scorer `score_assignment`; (3) Candidate generation (available-lots pre-pass + bounded-complete vertex enumeration); (4) Mode-1 optimizer `optimize_year` + optimality KATs vs brute-force oracle; (5) Compliance + persistability overlay (`AttestedRecording` + envelope/timing gates); (6) Mode-2 consult `consult_sale` + ST→LT timing; (7) §1091 wash-sale documentation + loss-harvest KAT; (8) CLI attestation side-table; (9) CLI `optimize run`; (10) CLI `optimize accept`; (11) CLI `optimize consult`; (12) whole-diff review (Phase E).

**New public API surface:**
- Core (`btctax-core::optimize`): `optimize_year(.., attested, proposal_made) -> Result<OptimizeProposal, OptimizeError>` (Mode 1; `proposal_made` = the CLI-seam made-date — R0-C2); `consult_sale(...) -> Result<ConsultReport, OptimizeError>` (Mode 2); `score_assignment(...) -> TaxOutcome` (holistic scorer); pure functions `compliance_overlay`, `persistability`, `proposed_compliance_status` (R0-C2); types `OptimizeProposal` (now carrying `approximate: bool` + `approx_reason: Option<ApproxReason>` — R0-C1/C3), `ApproxReason`, `DisposalProposal`, `Persistability`, `ConsultRequest`, `ConsultReport`, `TimingInsight`, `OptimizeError` (reusing `LotPick` — with the additive `Ord` derive, R0-I2 — `ComplianceStatus`/`AttestedRecording`, `TaxResult`, `MarginalRates`).
- CLI: `optimize run --tax-year <y>`, `optimize accept --tax-year <y> [--disposal <ref>] [--attest "<text>"]`, `optimize consult --sell <sat> [--wallet <w>] [--at <date>] [--proceeds <usd>|--fmv]`; an `optimize_attestation` side-table (projection input — **not** a new event type); `render_optimize_proposal`/`render_consult`.

**Chosen algorithm + why deterministic-and-optimal-on-the-modeled-cases:** per-disposal candidate generation feeding a **holistic** scorer (greedy alone is strictly wrong — §1(h)/§1211/§1222 coupling). For a single disposal, total proceeds are fixed, so the achievable `(ST,LT)` set is a convex polygon whose **vertices** are "consume a subset whole + ≤1 partial"; C enumerates the **complete vertex set** on small pools, takes the deterministic cartesian product, and scores each full-year combination through B's `compute_tax_year`, choosing the minimal `total_federal_tax_attributable` (total-order tie-break). Infeasible cross-disposal combinations self-eliminate (`LotSelectionInvalid` → `TaxYearNotComputable` → skipped). It is **deterministic** (BTreeMap/sorted `Vec`, `Decimal`/`i64` only, no float, no `HashMap` iteration, no clock in core) and **optimal on the modeled cases** (whole-lot fixtures within the enumeration bound → vertices = the achievable set), proven by KATs asserting equality with an independent exhaustive oracle. Contended same-wallet disposals are **detected and jointly enumerated** within `GROUP_COMBO_BOUND` (recovering cross-period reassignment optima — R0-C3). The search is **baseline-seeded** so `delta ≤ 0` always (R0-C1: never recommend worse-than-doing-nothing). Beyond the bounds a deterministic **baseline-seeded** coordinate-descent fallback applies, and any non-fully-enumerated result — the fallback, an un-enumerated contended group, **or a single pool exceeding `LOT_ENUM_BOUND` that took the heuristic subset (`PoolHeuristic` — R2-C1)** — is flagged `approximate = true` with a structured `approx_reason`, logged at the CLI, and rendered with an explicit "APPROXIMATE — not a guaranteed global minimum" banner — never presented as "the optimum" (`approximate == false ⇔ fully-enumerated-global`).

**Spec ambiguities resolved:** (1) `AttestedRecording` is persisted via a CLI **side-table** + a pure core **overlay** (no new event type; A's `disposal_compliance` untouched); (2) the optimizer targets the **post-2025 per-wallet** regime — a pre-2025 year is **refused** as a restatement (M7); (3) the optimization domain is **vertex selections** (whole-lot + ≤1 partial); contended-same-wallet disposals are **detected and jointly enumerated** within the bound (or explicitly flagged `approximate`/`ContentionUnenumerated` beyond it — R0-C3), and the multi-partial-kink limitation is **disclosed in output** (R0-M2); neither compromises NFR4/NFR5 or emits an unsafe selection, and no non-fully-enumerated result is ever rendered as "the optimum"; (4) Mode-2 optimizes **only** the synthetic disposal's selection (existing disposals untouched, read-only); (5) `optimize accept` **recomputes** the same deterministic optimum and gates **per disposal**, with attestation scoped to a single `--disposal` so the app never invites a blanket false attestation.

---

## Fold record (R0 round 1)

**Review folded:** `reviews/R0-plan-optimizer-round-1.md` (verdict: NOT GREEN — 3 Critical, 4 Important, 5 Minor, 2 Nit). **Re-grounded against shipped A+B source at fold time (2026-06-30):** `crates/btctax-core/src/project/{fold.rs,resolve.rs,pools.rs,compliance.rs,evaluate.rs}`, `event.rs`, `state.rs`, `tax/compute.rs`, `crates/btctax-adapters/src/tax_tables.rs`. **Algorithm spine confirmed SOUND and kept** (vertex sufficiency, feasibility-by-scoring self-elimination, Mode-2 read-only, I6 refusal — all re-verified). All fixes are on the **honesty / compliance / candidate-set-correctness** surface. **Citations confirmed at fold time:** `LotPick` derive lacks `Ord` (event.rs:190); `disposal_compliance` builds `sel_made` from `events` (compliance.rs:98-119,144-149); `resolve` does not sort `timeline`, `fold` sorts (fold.rs:335,341), `sort_canonical` is `pub` (resolve.rs:805); `net_1222` caps the deduction at `loss_limit` (compute.rs:174-178); only TY2025 is bundled (tax_tables.rs:48-54); `consume_picks` hardcodes `shortfall=0` (pools.rs:175); `btctax-core` has no logger.

**Per STANDARD_WORKFLOW §2, this plan now RE-ENTERS the review loop (round 2). The fold is not "done" until a fresh independent review confirms 0C/0I.**

### Criticals

- **C1 — silent local optimum presented as "the least tax."** **RESOLVED.**
  - `OptimizeProposal` gains `approximate: bool` + `approx_reason: Option<ApproxReason>` (new enum: `ComboCapExceeded { combos, cap }` / `ContentionUnenumerated { contended, combos, cap }`) — Task 1 types + re-export.
  - The search is **baseline-seeded** (`baseline_assignment` = current-method picks; incumbent starts at `base.total_federal_tax_attributable`) in **both** `exhaustive_min` and `coordinate_descent`, so `delta ≤ 0` ALWAYS (coordinate descent no longer starts all-HIFO) — §4 + Task 4.
  - Exhaustive (≤ MAX_COMBOS, all contention enumerated) sets `approximate=false`; the fallback sets `approximate=true` + reason. CLI `run` **logs** the reason (`eprintln!`, since core has no logger) and `render_optimize_proposal` prints the **"⚠ APPROXIMATE — NOT a guaranteed global minimum"** banner — Task 9.
  - KAT: a fixture driving the fallback asserts `approximate==true` + reason + `delta ≤ 0`; within-bound asserts `approximate==false` — Task 4.

- **C2 — post-hoc / standing-order-divergent picks shown compliant/persistable.** **RESOLVED.**
  - Root cause confirmed: `disposal_compliance(events, &opt_state)` reads `events` for `sel_made`, but the proposed pick lives in the fold's `selections` → it skips the selection branch and a divergent pick falls through to `StandingOrder`; `persistability(wallet, date, date)` returns `ContemporaneousNow` for every disposal.
  - Fix: `optimize_year` gains a `proposal_made: TaxDate` parameter (the real made-date threaded from the CLI seam; core stays clock-free). Each row's status is computed by the new pure helper **`proposed_compliance_status(wallet, sale, made, proposed, current, baseline_status)`** (Task 5): `proposed==current` keeps the genuine baseline status (the ONLY path that may report `StandingOrder`); a **divergent** pick is judged by its own made-date — 2027+ broker → `NonCompliant`, `made ≤ sale` → `Contemporaneous`, else (post-hoc) → `NonCompliant` (a standing order NEVER rescues it). `compliance_overlay` then runs as before. `persistable = persistability(wallet, date, proposal_made)`. The `disposal_compliance(events,&opt_state)` call and the `(date,date)` persistability call are removed.
  - KAT (Task 5): a proposed pick diverging from an in-force standing order on an already-executed disposal → `status==NonCompliant`, `persistable==NeedsAttestation` (not `ContemporaneousNow`); plus "every already-executed disposal is not `ContemporaneousNow`." N1 (false "status reflects the proposed selections" comment) and N2 ("conservative stand-in" for the least-conservative `ContemporaneousNow`) fixed in the same edits.

- **C3 — contended same-wallet disposals silently drop the cross-period optimum.** **RESOLVED.**
  - `available_lots_before` generated each disposal's candidates against earlier disposals' baseline consumption, missing reassignment optima. Fix: **detect contention** (`contention_groups` — ≥2 disposals on one `PoolKey::Wallet` pool with overlapping available lots) and **jointly enumerate** contended groups (`group_candidate_assignments` — nest `candidate_selections` in canonical order, each later disposal drawing from the prior candidate's resulting pool) within `GROUP_COMBO_BOUND` — Task 3. `optimize_year` takes the cartesian product across groups (Task 4).
  - Beyond the bound: the group falls back to independent generation AND the proposal is flagged `approximate=true, ContentionUnenumerated` (same disclosure mechanism as C1). Joint enumeration is preferred within the bound; flag-approximate only beyond it. No result is rendered as "the optimum" when contention was not fully enumerated.
  - KAT: two contended intra-year sells across an ST/LT crossover → the joint optimum is found (`optimized_tax == oracle_min_total` over the JOINT space) with `approximate==false`; a past-bound variant asserts the flag — Task 4.

### Importants

- **I1 — `available_lots_before` truncated in unsorted load order.** **RESOLVED.** It now applies `sort_canonical(&mut res.timeline)` (pub, resolve.rs:805) + the stable transition partition `sort_by_key(|e| e.date() >= TRANSITION_DATE)` (mirroring fold.rs:341) **before** `position`/`truncate`. KAT fixtures use **load-order ≠ canonical-order** (early-time/late-load lot present; late-time/early-load lot absent) — Task 3.
- **I2 — `LotPick` lacks `Ord`/`PartialOrd`.** **RESOLVED.** Added an explicit additive A-side step: derive `PartialOrd, Ord` on `LotPick` (event.rs:190; both fields `Ord`). Serde-compatible, no behavior change. The "reuse verbatim / no A-side change" claims (Task 1 note, fold_with note, §5 type-consistency, §6 summary) are corrected to acknowledge this one exception. A Task-1 KAT exercises `BTreeSet<Vec<LotPick>>`.
- **I3 — loss-harvest KAT unsound under the single-year objective.** **RESOLVED.** The KAT now asserts ONLY what the objective pins — `optimized_tax == oracle_min_total`, `optimized_tax < baseline_tax`, and in-year `loss_deduction == $3,000` — and explicitly does **not** assert a `carryforward_out` split (the objective is carryforward-blind: over-harvesting beyond the $3k cap is an objective tie, compute.rs:174-178). Carryforward-blindness is disclosed (§4 + FOLLOWUPS); a uniquely-determined harvest would require a specced secondary objective — Task 4.
- **I4 — Mode-2 timing re-scored in the crossover year (errors on unbundled year).** **RESOLVED.** `timing_insight` now returns `Option<TimingInsight>` (omit, never `Err`) and computes `tax_if_sold_long_term` **within `at`'s own tax year/profile/table** by a term-flip (synthetic disposal dated `latest_crossover` carrying the actual proceeds), **only** when `latest_crossover.year() == at.year()` and the table/profile exist; otherwise it OMITS rather than re-scoring a future (unbundled, tax_tables.rs:48-54) year. KAT: a crossover into an unbundled year still yields a valid `ConsultReport` (timing omitted) — Task 6.

### Minors / Nits

- **M1 (principal conservation).** **FOLDED** — `score_assignment` documents the precondition loudly and `debug_assert!`s each injected pick-set's `Σsat == principal` (via a baseline-fold lookup; `consume_picks` hardcodes `shortfall=0`, pools.rs:175); a `#[should_panic]` debug KAT — Task 2.
- **M2 (multi-partial kink).** **FOLDED** — disclosed in the rendered output (caveat footer in `render_optimize_proposal`), not only FOLLOWUPS — Task 9 + §4.
- **M3 (consult pool validity).** **FOLDED** — `consult_sale` computes the as-of-`at` pool via `fold_as_of` (truncate canonical timeline to `date() ≤ at`, R0-I1 ordering), correct for all `at`, removing the "valid only at end-of-timeline" caveat; KAT with an interleaved `at` — Task 6.
- **M4 (`next_day()` Option).** **FOLDED** — `one_year_after(start).next_day()` is `Option`; `None` (Dec-31/max edge) → omit the timing insight, no unwrap; KAT — Task 6.
- **M5 (`accept` appends before attest gate).** **FOLDED** — the blanket-attest precondition (`--attest` requires `--disposal`) is hoisted **above the loop** (and above the `optimize_year` recompute), so no disposal is appended before the guard fires; KAT asserts event count unchanged on the error — Task 10.
- **N1 / N2 (false / backwards comments).** **FOLDED** — the "status reflects the proposed selections" comment (was false) and the "conservative stand-in" wording (was the least-conservative verdict) are corrected in the C2 edits.

### Self-consistency pass (per the brief)
- **No silent local optimum:** baseline-seeded search (`delta ≤ 0`), `approximate`/`approx_reason` set on every non-fully-enumerated path, CLI log + render banner, Task-12 grep gate. ✔
- **No post-hoc-compliant proposed pick:** status via `proposed_compliance_status` (divergent post-hoc → `NonCompliant`, never `StandingOrder`); `persistable` via the real `proposal_made` (already-executed → never `ContemporaneousNow`); overlay unchanged (pure). ✔
- **Mode-2 read-only + same-year:** `consult_sale` clone-folds only; timing stays within `at`'s year/profile and degrades (omits) rather than erroring. ✔
- **`LotPick: Ord` reconciled:** additive derive added; "no A-side change" claims corrected. ✔

---

## Fold record (R0 round 2)

**Review folded:** `reviews/R0-plan-optimizer-round-1.md` → **"Round 2 — re-review"** section (verdict: NOT GREEN — 1 Critical, 1 Important, 2 Minor). **Re-grounded against CURRENT shipped A+B source at fold time (re-read 2026-06-30):** `crates/btctax-core/src/project/{compliance.rs,resolve.rs,fold.rs,pools.rs,evaluate.rs}`, `event.rs`, `state.rs`, `tax/compute.rs`, `crates/btctax-adapters/src/tax_tables.rs`. **Round 2 confirmed five of seven round-1 blockers fully CLOSED (C3, I1, I2, I3, I4) and all Minors/Nits folded — those are NOT re-touched here.** The two remaining items are *sibling-path residuals* of C1 and C2 (each closed only on the path round 1 named), plus two doc/render minors. **Citations confirmed at fold time:** `candidate_selections` returns `Vec<Vec<LotPick>>` with no heuristic signal (plan Task 3); `compliance.rs:144-149` judges an applied selection by its own made-date and reaches `StandingOrder` (step 3) only when NO selection was applied (so a what-if's divergent pick falls through — root of C2, and the overlay's blind spot is the read-only sibling); the attestation side-table (Task 8) is keyed by `disposal_event` **only** (attests a disposal, not a selection); `event.rs:215-217` `LotSelection { disposal_event, lots }` (a persisted selection drives the baseline fold, so on re-run `current == the persisted-and-attested pick`); `tax_tables.rs` bundles TY2025 only.

**Per STANDARD_WORKFLOW §2, this plan now RE-ENTERS the review loop (round 3). The fold is not "done" until a fresh independent review confirms 0C/0I.**

### Critical

- **R2-C1 — large-pool (`> LOT_ENUM_BOUND`) heuristic candidate sets scored and returned with `approximate = false` (presented as the PROVEN global minimum, no banner, no cap log).** **RESOLVED.**
  - Root cause confirmed: `candidate_selections` returns a strict, deterministic **subset** of a pool's vertices when `lots.len() > LOT_ENUM_BOUND` (=12) but gave the caller **no signal**; a single `> 12`-lot pool (weekly-DCA / active trading — common) yields a small `product ≪ MAX_COMBOS`, so `exhaustive_min` ran over the incomplete list and `optimize_year` left `approximate = false`, `approx_reason = None` — the headline-forbidden false-global claim, the same failure-class round 1 graded Critical (C1/C3). The fold's `approximate` contract (added in round 1) had been wired to exactly two triggers (`product > MAX_COMBOS`; a contended group returning `None`) and not to this pre-existing approximate path.
  - Fix (mechanical, same disclosure mechanism as C1/C3): `candidate_selections` now returns **`(Vec<Vec<LotPick>>, bool)`** — the `bool` reports whether it took the `> LOT_ENUM_BOUND` heuristic branch (Task 3). `group_candidate_assignments` propagates a nested-heuristic indicator (`Option<(maps, Option<usize>)>`). `optimize_year` accumulates `pool_heuristic_lots` across every singleton/group; if **any** target disposal's pool was heuristic it sets `approximate = true` and (precedence: `ComboCapExceeded` > `ContentionUnenumerated` > `PoolHeuristic`) reports the **new `ApproxReason::PoolHeuristic { lots, bound }`** (Task 1 enum). The renderer's banner `match` gains a `PoolHeuristic` arm (Task 9); the CLI already logs any `approx_reason`. The baseline-seed is untouched, so `delta ≤ 0` still holds — the disclosure fixes the false *"proven optimum"* claim, not the pick's safety.
  - Task-1 **contract corrected**: `approximate == false ⇔ the vertex set was FULLY enumerated AND exhaustively scored` (every pool ≤ `LOT_ENUM_BOUND`, `product ≤ MAX_COMBOS`, every contended pool jointly enumerated) ⇒ the proven global minimum; `true` otherwise. §4 (Generators + bound + documented-bounds (iii)), §5/§6 self-review, Task 12 grep gate updated in lockstep.
  - KAT (both ways): a single disposal over a `> LOT_ENUM_BOUND` pool ⇒ `approximate == true`, `approx_reason == Some(PoolHeuristic { lots, bound: 12 })`, `delta ≤ 0`; a `≤ 12`-lot pool ⇒ `approximate == false`, `approx_reason == None`. Plus a Task-3 unit KAT on the `candidate_selections` heuristic flag, and a Task-9 render KAT asserting the `PoolHeuristic` banner text. (Mode-2 `consult` destructures `(_, heuristic)`; R2-C1 scopes the Mode-1 `OptimizeProposal` "proven optimum" claim — `ConsultReport` makes no such claim and surfaces no `approximate` field, noted in Task 6.)

### Important

- **R2-I1 — `compliance_overlay` could upgrade a divergent, un-attested proposed pick to `AttestedRecording` (un-attested post-hoc cherry-pick shown compliant).** **RESOLVED.**
  - Root cause confirmed: the attestation side-table is keyed by **disposal**, not by selection, and the overlay upgraded `NonCompliant → AttestedRecording` gated only on `attested.contains(disposal)` + envelope. Sequence: attest+persist `P1` for X → later re-`run` finds a strictly-better divergent `P2 ≠ P1` → `proposed_compliance_status` correctly returns `NonCompliant`, but the overlay laundered it to `AttestedRecording` because X ∈ `attested`. This was **asymmetric** with the round-1 C2 fix (a standing order can't rescue a divergent pick, but an attestation could).
  - Fix: gate the upgrade **additionally on `proposed == current`**. `compliance_overlay`'s signature gains `unchanged: &BTreeSet<EventId>` (the disposals whose proposed pick equals the in-force, persisted-and-attested current selection); the upgrade now requires `attested ∧ unchanged ∧ envelope` (Task 5). `optimize_year` builds `unchanged` from `row_meta` (`proposed == current`) and passes it (Task 4). **How `current`/the attested selection is compared:** a persisted `LotSelection` drives the baseline fold, so on re-run `current_selection == the persisted-and-attested pick (P1)`; thus `proposed == current` is exactly "the proposed pick equals the attested selection." This mirrors `proposed_compliance_status`'s `proposed == current` rule, restoring symmetry (attestation now confers `AttestedRecording` ONLY on the exact attested selection). `accept` already gates the **write** (`persistability ⇒ NeedsAttestation`); this fixes the read-only proposal's **status**.
  - KAT: attest X for `P1`+persist; (i) no-change re-run (proposed `== P1 == current`) ⇒ X row `AttestedRecording`; (ii) introduce a change so the re-run proposes divergent `P2 ≠ P1` ⇒ X row `NonCompliant`, **not** `AttestedRecording` (Task 5 unit KAT on the overlay + end-to-end through `optimize_year`).

### Minors

- **R2-M1 (no-change persistability line is misleading).** **FOLDED** — `render_optimize_proposal` now suppresses the persistability line on a no-change row (`proposed == current`) and prints a "no change — already optimal under current identification" note instead (consistent with `accept`'s skip), so an already-executed no-change disposal no longer shows a "needs `--attest`" line for a change the optimizer is not requesting — Task 9 (+ render KAT).
- **R2-M2 (NFR4 determinism-tuple wording).** **FOLDED** — §0 NFR4 input tuple now reads `(events, prices, config, year, profile, tables, proposal_made, attested)`, noting `proposal_made`/`attested` are included because the proposal's `status`/`persistable` depend on them (the optimization core — picks/`optimized_tax`/`delta` — does not). Doc-lag only; behavior was already deterministic given all inputs.

### Self-consistency pass (per the round-2 brief)
- **`approximate == false` strictly ⇔ fully-enumerated-global:** every non-fully-enumerated path — coordinate-descent fallback (`ComboCapExceeded`), un-enumerated contention (`ContentionUnenumerated`), AND a `> LOT_ENUM_BOUND` heuristic pool (`PoolHeuristic`, R2-C1) — sets `approximate = true` + reason; no incomplete enumeration renders as "the optimum." ✔
- **No laundered attestation:** the `AttestedRecording` overlay upgrade is gated on `attested ∧ unchanged (proposed == current) ∧ envelope`, so an attestation confers `AttestedRecording` ONLY on the exact attested selection; a divergent re-run pick on an attested disposal is `NonCompliant` (R2-I1) — symmetric with the standing-order rule. ✔
- **No misleading no-change persistability line:** suppressed when `proposed == current` (R2-M1). ✔
- **NFR4 tuple complete:** `proposal_made` + `attested` added (R2-M2). ✔
- **Untouched closed findings:** C3/I1/I2/I3/I4 and all round-1 Minors/Nits are left exactly as folded in round 1. ✔
