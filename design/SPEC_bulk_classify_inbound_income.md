# SPEC — bulk-classify-inbound-income (queue item 3, Cycle 4)

**Source baseline:** `main` @ `c643ddd` (branch `feat/bulk-classify-inbound-income`). **Review status:
DRAFT — awaiting R0 (2-round independent architect loop to 0C/0I before implementation).**
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
`TransferIn` events (as `self_transfer_match_plan` enumerates) **MINUS** any already targeted by a
NON-VOIDED `ClassifyInbound` (mirror `open_classify_inbound_flow` filter-3 — a second `ClassifyInbound`
fires a return-blocking Hard `DecisionConflict`) **MINUS** wallet-less inbounds (create no lot). Then the
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
`InboundClass::Income { fmv: None, … }` projects to `Income { usd_fmv: None }` → the engine re-fires a
**Hard `FmvMissing`** blocker (resolve.rs:167 — `ManualFmv` is what clears it), which GATES the whole tax
year. So a bulk-income that included a missing-price row would trade one blocker (`UnknownBasisInbound`)
for another Hard one (`FmvMissing`) — a damaging no-op. **The `fmv_of == None` exclusion is the sole
defense** and is the ONLY behavioral difference from bulk-sti (where missing-price rows are INCLUDED — a
$0-basis self-transfer needs no FMV, the price is a mere advisory floor). The preview surfaces
`excluded_missing_price` so the user knows N inbounds could NOT be auto-valued as income (they stay
pending; the user can set FMV manually or classify them as self-transfer instead).
- The other roadmap tax-safety points (b) estimated-gain, (c) void `#7` do NOT apply to this cycle.

## Uniform parameters (applied to every included row)
`InboundClass::Income { kind: IncomeKind, fmv: Some(fmv_of(date, sat)), business: bool }`. `IncomeKind` is
one of {Mining, Staking, Interest, Airdrop, Reward} (user picks ONE for the batch); `business: bool`
(user toggles). `fmv` is PER-ROW auto-computed (`fmv_of(date, sat)`), never uniform. `fmv_status` = the
auto/ingest status the single classify-income arm already assigns (reuse it — do not invent a new status).

## Persist — REUSES the shared `persist_bulk_decisions` (no side-effect)
Unlike bulk-void, classify-income has NO side-effect (no attestation clear) — it is N `ClassifyInbound`
appends. So it uses the shipped `persist_bulk_decisions(session, payloads, now, empty_label)` directly
(empty-guard + mid-batch rollback + single save all already there + pinned). Each payload =
`EventPayload::ClassifyInbound(ClassifyInbound { transfer_in_event, as_: InboundClass::Income{…} })`. No new
persist fn.

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
  builds one `ClassifyInbound{Income}` per row (auto-FMV per row) and delegates to `persist_bulk_decisions`
  (single session, atomic). Dispatch derives `in_events` from the PLAN's `included` rows (predicate +
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
- `bulk_income_apply_recognizes_income` (E2E: after apply, `state.income_recognized` grows by the
  included count and the `UnknownBasisInbound` blockers clear; NO new Hard blocker).
- `bulk_income_empty_refuses` (via persist_bulk_decisions empty-guard); `bulk_income_dry_run_writes_nothing`;
  `bulk_income_uniform_kind_and_business` (every persisted row carries the chosen kind + business).
- TUI: `bulk_income_refuses_when_no_candidates`, `bulk_income_per_row_exclude_drops_row`,
  `bulk_income_preview_shows_total_and_excluded`.

## Plan (TDD)
- **Task 1 — bulk-classify-inbound-income:** `Session::bulk_classify_income_plan` (mirror bulk-sti + the
  fmv-exclusion) + CLI `apply_bulk_classify_inbound_income` (two-phase, `--kind`/`--business`) via
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
