# SPEC — bulk-classify-inbound-income (queue item 3, Cycle 4)

**Source baseline:** `main` @ `c643ddd` (branch `feat/bulk-classify-inbound-income`). **Review status: R0 round 1 folded
(0C / 1I / 4M / 2N — all folded; confirm-tier flag adjudicated GREEN = reuse bulk-sti's revocable tier).
Review: `reviews/R0-spec-bulk-classify-inbound-income-round-1.md`. Awaiting R0 round 2.**
**Lineage:** queue item 3 of `bulk-reconcile-other-types` (architect-designed, user-approved safety-first
sequencing). Cycles 1-3 SHIPPED (extract → resolve-conflict → void). This is **Cycle 4**.

## The feature
The bulk analog of classify-inbound → **Income** (`c` → `open_classify_inbound_flow`, the `InboundClass::Income`
arm). Sweep MANY pending unknown-basis inbound deposits → `Income` (uniform `IncomeKind` + `business` flag,
**auto-FMV** at the receipt date) in one filtered, per-row-excludable, confirmed, atomic batch. Directly
mirrors the shipped **bulk-classify-inbound-self-transfer** (`B` / `bulk_self_transfer_in_plan` /
`apply_bulk_self_transfer_in`), with ONE load-bearing difference (see §Tax-safety). New TUI key **`I`**
(free — confirmed against the keymap; `i` is single resolve-conflict, `I` unused). New CLI `reconcile
bulk-classify-inbound-income`.

## Candidate set — mirror bulk-sti, then EXCLUDE missing-price [tax-safety-critical]
Start from the EXACT bulk-sti candidate set (`session.rs` `bulk_self_transfer_in_plan`, filter-3 lineage):
the pending unknown-basis inbounds = `state.blockers` with `kind == UnknownBasisInbound`
(`session.rs:569-573`, the bulk-sti seed) **MINUS** any already targeted by a NON-VOIDED `ClassifyInbound`
(mirror `open_classify_inbound_flow` filter-3 — a second `ClassifyInbound` fires a return-blocking Hard
`DecisionConflict`, `resolve.rs:582-592`) **MINUS** wallet-less inbounds (create no lot). Then the
**Cycle-4 addition**:
- **MINUS every row where `fmv_of(&prices, date, sat) == None`** (missing daily-close price OR overflow).
  These are EXCLUDED from `included` and reported separately as `excluded_missing_price` (NOT silently
  dropped). See §Tax-safety for why this is mandatory.

`Session::bulk_classify_income_plan(filter: BulkIncomeFilter) -> BulkIncomePlan` (mirror `BulkStiPlan`):
`included: Vec<BulkIncomeRow>` (each with `in_event`, `sat`, `date`, `fmv: Usd` [always `Some` — None
excluded]), `excluded_missing_price: usize`, `total_sat`, `total_income_usd` (Σ `fmv` over `included`).

## Tax-safety — the #a point (the whole ballgame for this cycle)
**Auto-FMV is TAX-AFFECTING.** Classifying an inbound as `Income` sets `usd_fmv` = the recognized income
(and the lot basis). If `fmv_of` returns `None` (no price for that date), a persisted
`InboundClass::Income { fmv: None, … }` projects (`Op::IncomeInbound`, resolve.rs:273-282) to
`Income { usd_fmv: None }` → the engine raises a **Hard `FmvMissing`** blocker (fold.rs:853-860) that GATES
the whole tax year. So a bulk-income that included a missing-price row would trade one blocker
(`UnknownBasisInbound`) for another Hard one (`FmvMissing`) — a damaging no-op. **Worse [R0-M1]: on the
inbound path this is NOT clearable by `ManualFmv`** — a `ManualFmv` aimed at a classified `TransferIn` is
itself rejected as a Hard `DecisionConflict` (resolve.rs:476-495), so the ONLY escape is void + reclassify.
Emitting such a row creates an unrecoverable-without-void year-gate. **The `fmv_of == None` exclusion is the sole
defense** and is the ONLY behavioral difference from bulk-sti (where missing-price rows are INCLUDED — a
$0-basis self-transfer needs no FMV, the price is a mere advisory floor). The preview surfaces
`excluded_missing_price` so the user knows N inbounds could NOT be auto-valued as income (they stay
pending; the user can set FMV manually or classify them as self-transfer instead).
- The other roadmap tax-safety points (b) estimated-gain, (c) void `#7` do NOT apply to this cycle.

## Uniform parameters (applied to every included row)
`InboundClass::Income { kind: IncomeKind, fmv: Some(row.fmv), business: bool }` where `row.fmv: Usd` is the
plan-resolved per-row auto-FMV. `IncomeKind` is one of {Mining, Staking, Interest, Airdrop, Reward} (user
picks ONE for the batch); `business: bool` (user toggles). `fmv` is PER-ROW (`fmv_of(date, sat)`, resolved
to `Some` at filter time), never a uniform user number. **[R0-M3]** `InboundClass::Income` has ONLY
`{ kind, fmv, business }` (`event.rs:127-132`) — there is **no `fmv_status` field** to set (that lives on
the imported-`Income` payload and is the engine's concern, not the classify decision's); and `fmv` is
already `Option<Usd>`, so it is `fmv: fmv_of(...)` / `Some(row.fmv)`, never `Some(fmv_of(...))`.

## Persist — CLI own-loop + TUI wrapper [R0-I1 — the shared helper is NOT cli-reachable]
`persist_bulk_decisions` lives in **btctax-tui-edit** (`edit/persist.rs:394`); **btctax-cli does NOT depend
on tui-edit** (tui-edit → cli, so the reverse is a dependency cycle) — the CLI CANNOT call it. This is the
same trap as Cycle-2's R0-I1. Mirror the SHIPPED `apply_bulk_self_transfer_in` (`reconcile.rs:273-294`)
exactly:
- **CLI** `apply_bulk_classify_inbound_income` — its OWN append-loop over the plan's `in_events` (one
  `ClassifyInbound{Income}` per row), bare `?`-before-`save` (a mid-batch failure returns before `save` →
  the in-memory session is discarded, nothing lands on disk = CLI atomicity), then a single
  `session.save()`. NOT the shared helper.
- **TUI** — a thin `persist_bulk_classify_income(session, payloads, now)` in `edit/persist.rs` that DOES
  delegate to `persist_bulk_decisions` (tui-edit CAN reach it) with the classify-income empty-label; the
  editor path thus keeps the explicit mid-batch rollback + empty-guard for free.
- **Structural no-`None` guarantee:** `plan.included` carries the RESOLVED `fmv: Usd` (NON-Option — the
  `None` rows were excluded upstream), and BOTH builders construct
  `InboundClass::Income { fmv: Some(row.fmv), kind, business }`. So `Income{fmv:None}` is structurally
  UNREPRESENTABLE from the bulk path — the #a exclusion cannot be defeated by a later construction bug.
  (`InboundClass::Income.fmv` is `Option<Usd>`, and `fmv_of` already returns `Option<Usd>`; the plan
  unwraps once at filter time and the builders re-wrap `Some(_)`.)

## Confirm — match bulk-sti's tier (revocable, tax-affecting preview)
`ClassifyInbound` is REVOCABLE (voidable), so NOT Tier-B/typed-word. Use the SAME confirm strength as the
shipped bulk-sti flow, with the preview PROMINENTLY showing **total income being recognized**
(`total_income_usd`) + the count auto-valued + the `excluded_missing_price` count. [R0: confirm the tier
matches bulk-sti exactly — reuse, don't diverge.]

## CLI — two-phase (mirror the shipped bulk commands)
- `reconcile bulk-classify-inbound-income --kind <mining|staking|interest|airdrop|reward> [--business]
  --dry-run` (Phase 1): `bulk_classify_income_plan` lists `included` + totals + `excluded_missing_price`;
  writes NOTHING.
- `… --yes` (Phase 2): `apply_bulk_classify_inbound_income(vault, pp, in_events, kind, business, now)` —
  its OWN append-loop (one `ClassifyInbound{Income{Some(row.fmv)}}` per row) + single `session.save()`,
  mirroring the shipped `apply_bulk_self_transfer_in` (NOT the tui-edit `persist_bulk_decisions`, which the
  CLI can't reach — R0-I1). Dispatch derives `in_events` from the PLAN's `included` rows (predicate +
  fmv-exclusion filtered), never raw `--ref` ids.

## TUI — `I` flow
`I` → pick `IncomeKind` (list) + toggle `business` → filter → `TargetList` per-row-exclude checklist
(shows date, BTC, auto-FMV per row) → confirm modal (total income recognized + excluded-missing-price
note) → `persist_bulk_decisions`. Mirror the shipped bulk-sti (`B`) flow structure.

## Core / SemVer
- **btctax-core:** NONE — reuses `ClassifyInbound` + `InboundClass::Income` + `fmv_of`. No new variant, no
  serde break, no behavior change. Additive cli + tui-edit. New `bulk-classify-inbound-income` subcommand
  (its clap doc-comment is the reference row; no docs mirror / help-overlay change, per the parked help surface).

## KATs
- `bulk_income_plan_lists_pending_inbounds` — candidate = TransferIn − already-classified − wallet-less
  (mirror bulk-sti); **`bulk_income_plan_excludes_already_classified`** (a non-voided `ClassifyInbound`
  target is omitted → no double-classify `DecisionConflict`).
- **`bulk_income_plan_excludes_missing_price`** [#a tax-safety] — a row whose date has NO bundled price is
  NOT in `included` and IS counted in `excluded_missing_price`; a persisted batch therefore NEVER creates
  an `Income{fmv:None}` → NO Hard `FmvMissing`. Pair: `bulk_income_apply_sets_autofmv` (an included row's
  persisted `Income.usd_fmv == fmv_of(date, sat)`).
- **`bulk_income_plan_excludes_wallet_less`** [R0-M4 — a wallet-less inbound is ALSO a Hard-`FmvMissing`/
  no-lot vector (fold.rs:833), same year-gating class as missing-price] — a wallet-less pending inbound is
  NOT in `included` (mirrors bulk-sti's wallet-less exclusion).
- `bulk_income_apply_recognizes_income` (E2E: after apply, `state.income_recognized` grows by the
  included count and the `UnknownBasisInbound` blockers clear; NO new Hard blocker).
- `bulk_income_empty_refuses` (via persist_bulk_decisions empty-guard); `bulk_income_dry_run_writes_nothing`;
  `bulk_income_uniform_kind_and_business` (every persisted row carries the chosen kind + business).
- TUI: `bulk_income_refuses_when_no_candidates`, `bulk_income_per_row_exclude_drops_row`,
  `bulk_income_preview_shows_total_and_excluded`.

## Plan (TDD)
- **Task 1 — bulk-classify-inbound-income:** `Session::bulk_classify_income_plan` (mirror bulk-sti + the
  fmv-exclusion, `included` carries resolved `fmv: Usd`) + CLI `apply_bulk_classify_inbound_income`
  (two-phase, `--kind`/`--business`; CLI OWN append-loop) + TUI `persist_bulk_classify_income` wrapper →
  `persist_bulk_decisions` + TUI `I` flow. All KATs above.
- **Task 2 — whole-diff review (Phase E)** + full workspace suite + FOLLOWUPS.

## Gotchas
- **The `fmv_of==None` exclusion is the whole cycle** — never let a missing-price inbound be classified as
  income (→ Hard `FmvMissing`, year-gated). bulk-sti INCLUDES those rows; bulk-income EXCLUDES them. Do not
  copy bulk-sti's missing-price handling blindly.
- **Surface, don't drop** — report `excluded_missing_price` in the preview (honest; the user chooses what
  to do with those rows). Silent exclusion reads as "classified everything."
- **auto-FMV is per-row** (`fmv_of(date, sat)`), never a uniform user number. Only `kind` + `business` are uniform.
- **No bespoke persist** — reuse `persist_bulk_decisions`; classify-income has no side-effect.
- Dispatch derives targets from the plan's `included` rows, never raw ids (the fmv-exclusion must not be bypassable).
