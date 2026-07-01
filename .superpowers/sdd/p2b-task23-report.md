# P2-B Task 2 + Task 3 — Form 8949 + Schedule D — implementation report

**Branch:** `feat/p2b-form8949` (base `ec755bf`, Task 1 already committed).
**Scope:** Task 2 (Form 8949 rows + `form8949.csv`) and Task 3 (Schedule D totals + `schedule_d.csv`
+ text summary + engine-B reconciliation). Output-only over existing `state.disposals`; **no change** to
capital-gains tax math (B) or removal/disposal-basis math. Federal-only, exact `Decimal` (NFR5),
deterministic (NFR4), privacy (synthetic + tempdirs only).

## Validation (authoritative, clean)

- `cargo test --workspace` → **487 passed; 0 failed; 0 ignored** (49 test binaries, exit 0).
- `cargo clippy --workspace --all-targets -- -D warnings` → clean (exit 0).
- `cargo fmt --all -- --check` → clean (exit 0).

## Task 2 — `Form8949Row` + `form_8949` builder + `form8949.csv`

New core module **`crates/btctax-core/src/forms.rs`** (registered in `lib.rs`, re-exported:
`form_8949, schedule_d, Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals`).

- **`Form8949Row`** carries: `part` (`Form8949Part::{ShortTerm,LongTerm}`), `box_`
  (`Form8949Box::{C,F}`), `box_needs_review: bool`, `description: String`, `date_acquired`,
  `date_sold`, `proceeds`, `cost_basis`, `adjustment_code: String`, `adjustment_amount: Usd`, `gain`,
  `wallet: WalletId`, `disposition_kind: DisposeKind`.
- **`form_8949(state, year) -> Vec<Form8949Row>`** — pure over `state.disposals`; **one row per
  `DisposalLeg`** whose `disposal.disposed_at.year() == year`.
  - `description` = **exact Decimal** `Decimal::from(leg.sat) / Decimal::from(SATS_PER_BTC)` formatted
    `"{btc:.8} BTC"` — **never** `sat as f64 / 1e8` [R0-M5]. (`SATS_PER_BTC` is the canonical
    `100_000_000` constant — value-identical to the spec's `dec!(100_000_000)`.) Since a satoshi count
    over 1e8 has ≤ 8 decimal places, the 8dp format is lossless (no rounding).
  - `date_acquired = leg.acquired_at` (zone-aware HP-start from Task 1); `date_sold =
    disposal.disposed_at`; `proceeds/cost_basis/gain` copied from the leg.
  - `part`/`box_`: `ShortTerm → (Part I, C)`, `LongTerm → (Part II, F)` — the conservative
    "not reported on a 1099-B" default; A/B/D/E are **never** auto-assigned.
  - `box_needs_review = matches!(leg.wallet, WalletId::Exchange { .. })` — **direct match** on
    `leg.wallet`, not `optimize.rs::is_broker` [R0-M2/D4].
  - `adjustment_code = ""`, `adjustment_amount = 0` (no §1091, no other adjustments).
  - NoGainNoLoss dual-basis gift legs **are** emitted (basis==proceeds ⇒ gain 0; no special code)
    [R0-M1].
  - **Deterministic ordering**: sorted by `(disposed_at, disposal.event, leg.lot_id)` — a total order
    independent of projection iteration order.
- **`form8949.csv`** written by `write_csv_exports` (render.rs) via the existing
  `csv::Writer::from_writer(fsperms::open_owner_only(..))` 0o600 pattern. Stable snake_case columns
  (exact contract): `part, box, box_needs_review, description, date_acquired, date_sold, proceeds,
  cost_basis, adjustment_code, adjustment_amount, gain, wallet, disposition_kind`.
  - `part` tag = `"ST"/"LT"`; `box` tag = `"C"/"F"` (new `form8949_part_tag`/`form8949_box_tag`
    free fns in render.rs, matching the crate's "CLI can't add methods to core types" convention);
    `disposition_kind` reuses the existing `dispose_kind_tag`.

## Task 3 — `schedule_d` totals + `schedule_d.csv` + text summary + B reconciliation

- **`ScheduleDTotals { st, lt }`** / **`ScheduleDPart { proceeds, cost_basis, gain }`** (both
  `Default`). **`schedule_d(state, year)`** sums proceeds/basis/gain within each character (ST/LT) over
  the year's disposal legs. RAW pre-netting totals — no §1222/§1211/§1212 netting or carryforward here.
- **`schedule_d.csv`** (columns `part, proceeds, cost_basis, gain`; rows ST then LT) — same 0o600
  writer pattern.
- **`render_schedule_d(year, &ScheduleDTotals) -> String`** text section added to the tax report; the
  `report --tax-year` path (`main.rs`) now prints it after `render_tax_outcome`. Carries the required
  note verbatim: "§1222/§1211/§1212 netting + carryforward are applied in the tax computation
  (report --tax-year); these are the raw pre-netting Form 8949/Schedule D part totals."

### Reconciliation KAT (R0-M3 — independent code paths, NOT a tautology) — **PASSES**

`crates/btctax-core/tests/tax_compute.rs::schedule_d_reconciles_with_engine_b_on_all_gains_fixture`:
on an all-gains fixture (ST gain 20,000 + LT gain 50,000, both 2025) with a profile carrying zero
`capital_loss_carryforward_in` and zero `other_net_capital_gain` (so §1222 does no cross-netting ⇒
net == raw), asserts `schedule_d(state, 2025).st.gain == compute_tax_year(..).TaxResult.st_net` **and**
`.lt.gain == .lt_net`. `schedule_d` and `compute_tax_year` are separate functions independently
aggregating the same `state.disposals` — a genuine cross-check (no shared helper). **They reconcile
(20,000 / 50,000).** Not BLOCKED.

## KAT coverage

- **`crates/btctax-core/tests/kat_forms.rs`** (new, 12 tests): ST→Part I/C; LT→Part II/F; exact
  description `"0.53000000 BTC"`; row fields match the leg (+ adjustment blank/0); multi-leg ST+LT →
  two rows in correct parts + within-disposal lot_id ordering; NoGainNoLoss gift → row present/gain 0;
  year-filter; deterministic (date, event, lot) ordering; exchange→`box_needs_review` true /
  self-custody→false; Schedule D hand-derived mixed golden (incl. signed LT loss leg + out-of-year
  exclusion); Schedule D year-filter; empty year → all-zero; form_8949 rows aggregate to schedule_d.
- **`forms.rs` unit test**: exact-Decimal description (0.53 / 1 sat / 1 BTC / 0.12345678).
- **`crates/btctax-cli/tests/export.rs`** (new test): `--tax-year` writes `form8949.csv` +
  `schedule_d.csv` (exact header contracts + one ST row: ST/C/`box_needs_review=true`/
  `"0.02000000 BTC"`/blank adjustment cols); omitted when `None`.
- **`crates/btctax-cli/tests/tax_report.rs`** (golden test enhanced): real-projection LT sell →
  `schedule_d` (proceeds 50000 / basis 30000 / gain 20000) + `render_schedule_d` carries the netting
  note and the LT part-total line.

## API / wiring notes

- **`write_csv_exports` signature changed** → `(out_dir, state, tax_year: Option<i32>)`. The
  all-years CSVs (lots/disposals/removals/income) are unchanged; `form8949.csv`/`schedule_d.csv` are
  written **only** when `tax_year` is `Some(y)`, year-scoped to `y` (they are inherently per-tax-year).
- **`cmd::admin::export_snapshot`** gains `tax_year: Option<i32>`; **`export-snapshot`** CLI gains an
  optional `--tax-year`. Existing all-years export tests updated to pass `None` (behavior preserved).
- **`cmd::tax::report_tax_year`** now returns `(TaxOutcome, Option<String>, ScheduleDTotals)` (computed
  from the same already-loaded projection); `main.rs` renders the Schedule D section after the tax
  outcome. Existing callers updated.
- **`Form8949Box` is intentionally two-variant `{C, F}`** — the model can only ever produce the
  not-reported-on-1099-B default; A/B/D/E reclassification is deferred to FOLLOWUPS (Task 4 controller
  gate), so a false substantiated box is impossible by construction.

## Constraints honored

Exact `Decimal` throughout (no float); deterministic ordering; federal-only; CSV 0o600 + stable
columns; **no capital-gains tax-math / basis-math change** (B untouched — verified by the reconciliation
KAT and all pre-existing tax tests passing unchanged); privacy (synthetic fixtures + tempdirs only;
`~/Documents/BitcoinTax/ReadOnly` never read). Task 4 (whole-diff review + FOLLOWUPS) is the
controller's gate and is intentionally **not** performed here.

---

## P2-B Whole-Diff Review Minor Fix — `render_schedule_d` NotComputable caveat (commit `42b0829`)

**Finding addressed:** `render_schedule_d` printed the §1222/§1211/§1212 netting note unconditionally,
including when the year's `TaxOutcome` is `NotComputable` — raw totals appeared without context next
to a "NOT COMPUTABLE" line.

**Fix (render-only; no tax computation change):**
- `render_schedule_d` signature changed to `(year, &ScheduleDTotals, &TaxOutcome)`.
- `NotComputable` branch: appends the caveat "(raw disposition totals shown above; the year's tax is
  NOT COMPUTABLE until the blocker is resolved — these Form 8949/Schedule D part totals are
  informational and are not netted/carried until the tax computes)."
- `Computed` branch: existing §1222/§1211/§1212 netting note unchanged.
- Raw totals (Part I ST / Part II LT) are always shown — never suppressed.
- Call site in `main.rs` updated to pass `&outcome`.

**KAT adjustments (crates/btctax-cli/tests/tax_report.rs):**
- `report_tax_year_renders_golden` (Computed): `render_schedule_d` now receives `&outcome`; asserts
  netting note present AND "NOT COMPUTABLE" absent.
- `report_tax_year_with_hard_blocker_says_not_computable` (NotComputable): `sched_d` is no longer
  `_sched_d`; renders Schedule D and asserts totals shown + caveat with "NOT COMPUTABLE" +
  "informational" present, netting note absent.

**Validation (clean):**
- `cargo test --workspace` → **487 passed; 0 failed** (exit 0).
- `cargo clippy --workspace --all-targets -- -D warnings` → clean (exit 0).
- `cargo fmt --all -- --check` → clean (exit 0).

**Files changed:** `crates/btctax-cli/src/render.rs`, `crates/btctax-cli/src/main.rs`,
`crates/btctax-cli/tests/tax_report.rs`.
