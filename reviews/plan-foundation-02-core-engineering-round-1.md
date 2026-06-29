# Engineering Review — IMPLEMENTATION_PLAN_foundation_02_core.md — Round 1

- **Reviewer:** independent senior Rust reviewer; verified cited APIs (rust_decimal RoundingStrategy::MidpointNearestEven/round_dp_with_strategy/normalize; time to_offset/from_calendar_date/rfc3339; rusqlite 0.31 unchecked_transaction/params!; sha2) — all plausible.
- **Date:** 2026-06-28
- **Verdict:** **NOT engineering-sound as written — 0 Critical, 3 Important** — all concentrated in **Task 12 (2025 transition)**. Tasks 0–11 + 13 concrete enough to execute. Persisted per STANDARD_WORKFLOW §2.

## Skeleton-task judgment (highest-priority check)
- **Task 3 (`load_all` id reconstruction): PASS** — schema persists event_id/kind/source/source_ref/decision_seq; Step 4 gives exact rebuild per kind. (Minor M5.)
- **Task 7 (pass-1 rewrite): PASS** — staged algorithm concrete (BTreeMap/BTreeSet, decision_seq-sorted, revocability classification, latest-seq-wins, explicitly-deferred 1e); build_op determinable from Tasks 4–6.
- **Task 12 (`transition.rs`): FAIL** — prose only, no code, unspecified pass-1↔pass-2 seam, implied-but-unstated signature change → I-1/I-2/I-3.

## IMPORTANT (all in Task 12)
### I-1 — conservation guard's "dry pre-fold" unspecified; pass-1↔pass-2 seam forces design invention
Guard requires `Σ allocation.usd_basis == Universal remaining pool basis` at 2025-01-01 — computing this means folding the entire pre-2025 timeline into the Universal pool (pass-2 fold logic) DURING pass-1 effectiveness evaluation. Plan hand-waves "build the pre-2025 Universal snapshot via a dry pre-fold" with no function/signature/seam. Two implementers will diverge (fold on filtered timeline vs transition-free mini-fold vs fold variant halting at boundary). The "invent design" failure the no-placeholder rule prevents, on the riskiest task.
### I-2 — required resolve/project signature change unstated (cross-task inconsistency)
`resolve(events) -> Resolution` is fixed; project calls resolve(events) then fold(resolution,prices,config). But Task 12 effectiveness runs in resolve and NEEDS config (first-2025-disposition trigger includes TP8(b) fee mini-disposition → depends on config.self_transfer_fee) and prices (pre-2025 basis snapshot includes GiftFmvFallback/FMV lots). So resolve must become resolve(events,prices,config) and project/mod.rs rewired. Task 12 Files list OMITS project/mod.rs; the plan's cross-task consistency note claims only Task 13 changes shape. Implementer "sees only their task" → hits a wall, redesigns outside scope.
### I-3 — Task 12 has no concrete failing tests (TDD cadence broken on riskiest task)
Scenarios (i)–(x) are comments with "write each as a #[test] mirroring Tasks 7–11." The §7.4 machinery (method-keyed earlier/later-of, attestation bypass, provisional effectiveness, inert→effective flips, void-of-effective vs void-of-inert, conservation-hard vs timebar-advisory) is where review-to-green matters most and is exactly where tests are deferred → weakens the gate, compounds I-1/I-2.

## MINOR
- M1 Debug-string sort key for blockers (fold.rs finalize): `format!("{:?}",event)` — Option<EventId> already derives Ord; sort by it; key omits `detail`; allocates String per cmp; fragile for NFR4.
- M2 Task 13 property tests partly inert: `sigma_lot_basis...` body `let _ = st;` asserts nothing; arb_events()/arb_events_no_pending_basis() referenced but undefined.
- M3 Task 11 FoldStats threading ambiguous ("return (LedgerState,FoldStats) or stash on private field"); mandate the on-LedgerState field approach (Task 13 assumes it).
- M4 `dec_ev` helper defined only in corrections.rs but used by KATs in kat_tax.rs (Tasks 8/9/11); integration files are separate crates → must duplicate into kat_tax.rs.
- M5 Task 3 ships `unreachable!()` id_for stub inside the "implement" block; verbatim-typer could ship it; restructure so Step 3's code is already correct.
- M6 Totality vs unchecked Decimal: §7.1 promises never-panics but split_pro_rata/fmv_of use unchecked `*`/`/` (panic on overflow); use checked_* or document an input bound.

## NIT
- N1 namespace drift (domain::conventions vs flattened conventions). N2 glob re-exports (event::*, state::*). N3 throwaway scaffolding lines in code blocks. N4 TransitionMode::PathB(()) unit placeholder (Task 12 churn).

## Solid (do not regress)
Determinism NFR4 (pure project; canonical sort utc→source_priority→source_ref injective; decision_seq order; only BTree*; permutation harness) sound apart from M1. Exact arithmetic NFR5 (Sat=i64, Usd=Decimal, HALF_EVEN, remainder-takes-rest split conserving exactly; no floats). Persistence boundary (rusqlite-only, &Connection, Vec<LedgerEvent>, project never sees Connection; atomic unchecked_transaction; fingerprint-normalized idempotent re-import). Identity/taxonomy (injective EventId, full §6.4 payloads, serde-str lossless Decimal). TP4 holding period boundary correct.

**Blocking work entirely in Task 12 (lines ~2262–2301): fold I-1/I-2/I-3 (+ cheap Minors), re-review → 0C/0I.**

## Round 2 (fold re-review) — ALL round-1 findings CLOSED, 0 Critical / 0 Important
I-1 CLOSED: concrete `transition::universal_snapshot(timeline,prices,config)->UniversalSnapshot` + a shared `pub(crate) fold_event(...)` reused verbatim by pass-2 `fold` and the pass-1 snapshot (cannot diverge); genuinely acyclic (filters pre-2025, never seeds; `resolve` reads only `snap.{held_sat,basis}`). I-2 CLOSED: `resolve(events,prices,config)->Resolution` everywhere (defs + call site + cross-task note), `project/mod.rs` in Task 12 Files, `project->LedgerState` unchanged, no stale single-arg calls. I-3 CLOSED: 11 runnable `#[test]`s covering the §7.4 machinery. M1–M6 + N4 all closed; C1 restructure (`FeeCarry`/`consume_fee`) type-consistent, `FoldStats` a `LedgerState` field, determinism/arithmetic intact. The two revised Task-12 tests are semantically coherent (allocation names the wallet holding the coins at the snapshot), not assertion-fudging.
**Severity tally: 0 Critical / 0 Important / 1 Minor / 2 Nit.** New Minor (→ FOLLOWUPS): seed fires on UTC-sorted iteration while routing/snapshot use tax-date → a sub-day year-boundary offset straddle can fold a pre-2025-tax-date event after the seed (fails safe: `uncovered_disposal` or a stranded lot; uncovered by the 11 KATs). Fix at implementation: partition the timeline at the tax-date boundary (or seed lazily on first wallet route) + add a reversed-offset KAT. Nits: Task-11 lifted arm bodies must drop the `&mut` prefixes (compiler-caught); declare the `allocation_voids` struct/collection explicitly. **Plan is engineering-sound — review-to-green at 0C/0I.**
