# Conservative-filing — follow-ups (per-phase burndown by ownership)

Each item has an **owning phase**; reconcile at the top of that phase (do the ones it owns, confirm
the rest are parked later). Only ownerless cross-cutting residue batches to the end. Blocking findings
(Critical/Important) are NEVER filed here — they are fixed before their gate closes.

## Open

### T16 whole-branch two-lens Fable review r1 (2026-07-20) — FOLDED; blocking findings fixed, residuals filed

Reviews persisted verbatim in `reviews/phase2-whole-branch-{tax,architecture}-fable-review-r1.md`
(counts: tax 0C/3I/4M/4N; architecture 1C/2I/4M/5N). **All BLOCKING findings fixed + tested + (where a
predicate changed) mutation-proven**, in one fold:

- **[Critical] Date panic** (`conservative.rs` `.expect` Dec-31) — FIXED: `if let Ok(as_of)` skip; KAT
  `tranche_report_advisory_does_not_panic_on_an_out_of_range_year`.
- **[Important] backstop poisons a voided-inert allocation → every year NotComputable** (`resolve.rs`) —
  FIXED: the tranche-residue Hard is skipped for a void-targeted allocation (§7.4 retires an inert one /
  Hard-conflicts an effective one), and the totals-mismatch compares against the DOCUMENTED residue
  (`held_sat − tranche_sat`). KAT `void_inert_alloc_then_declare_..._keeps_the_year_computable`, both
  halves mutation-proven RED.
- **[Important] P6 per-BTC reference used as the total lot basis** (`conservative.rs`) — FIXED: scaled
  `reference × sat / SATS_PER_BTC`; KAT `overpayment_delta_scales_the_per_btc_reference_...` (2-BTC == 2×).
- **[Important] `export-irs-pdf` / full-return packet omitted `basis_methodology.txt`** (`cmd/admin.rs`) —
  FIXED: the shared writer now runs on both PDF paths; KAT `export_irs_pdf_writes_basis_methodology_...`.
- **[Important] TUI Tax tab re-projected the whole ledger every draw tick** (`conservative.rs`) — FIXED:
  no-tranche early-return before any `project()` (a no-tranche vault pays zero again).
- **Minors folded inline:** header no longer asserts "$0 as filed" over a `>$0` fee-leg (M-1); overpayment
  per-tranche delta CLAMPED at $0 + doc corrected (M-2 arch); `overpayment_delta_one` requires
  `EstimatedConservative` (M-3 arch); residue refusal carries the D-8 finality hedge (M-3 tax); inversion
  advisory names BOTH levers (M-4 tax); phantom-wallet WARN moved AFTER the guard (N-1 arch); advisory
  money `{:.2}` (N-2); P4 copy de-retrospected (N-3/N-4 tax/arch).

**Residual Minor/Nit — FILED (non-blocking), owner = post-v1 cleanup:**
- **[Minor] disclosure omits `window_start`** (tax M-2) — the export CSV/PDF path does not thread `events`
  to `basis_methodology(state, year)`, only `state` (which carries `window_end` via `acquired_at`, not
  `window_start`); threading it through `write_form_csvs`/`write_csv_exports` is disproportionate. Mitigated
  by labeling the shown date "the conservative window-end date" (the binding date for term). The P6 nudge,
  which HAS `events`, already prints the full `{ws}–{we}`.
- **[Minor] CLI/TUI drift in the pseudo-placeholder corner** (arch M-4) — under pseudo mode the CLI resolves
  an all-`$0` placeholder profile and the TUI passes none. Benign in practice: an all-`$0` profile yields a
  `$0` overpayment delta → the P6 nudge does not fire either way; a defensive provenance-gate is optional.
- **[Nit] `window_reference` day-iterates an unbounded user window** (arch N-3) — the arch-I-2 early-return
  means it now runs ONLY for actual tranches; an absurd `window_start` (year 1) is a bounded ~740k no-op
  probe. A `declare-tranche` window-floor (dataset start) is the follow-up.
- **[Nit] `write_csv_exports` path untested for `basis_methodology.txt`** (arch N-5) — same shared
  `write_basis_methodology_txt` helper the `write_form_csvs` (tested) and `export-irs-pdf` (tested) paths use.

### From the Phase-1 implementation review (r1, 2026-07-21) — all Minor/Nit, non-blocking

- ~~**[Minor] `build_op`'s DeclareTranche arm + engine-level input validation are reachable only from the
  CLI record path, not the engine.**~~ **DONE (P9/T15)** — guarded the `build_op` DeclareTranche arm on
  `matches!(id, EventId::Decision { .. }) && t.sat > 0` (else `Op::Skip`). Both halves are reachable and
  mutation-proven: a `ClassifyRaw{as_: DeclareTranche}` on an Unclassified import (pass-1c applies the
  override un-type-validated → the import path folds it) now folds NOTHING instead of a bogus
  import-timestamped lot; a hand-crafted `sat ≤ 0` Decision tranche folds nothing instead of corrupting
  Σ-conservation. No loud blocker added (matches the engine's posture on any other malformed hand-crafted
  payload — an arbitrary-basis `Acquire` is likewise not specially rejected). KATs in `kat_conservative.rs`.

- **[Minor] `in_force_allocation_exists` diverges from engine void semantics for a dangling void on an
  EFFECTIVE allocation.** (arch r1 Minor 1) — the record-time predicate treats ANY `VoidDecisionEvent`
  target as not-in-force… **DONE (T16)** — TWO findings folded: (1) the review's "product-reachable via
  `reconcile void`" premise was imprecise — `reconcile void` already REFUSES voiding an effective allocation
  (§7.4 irrevocable), pinned by `reconcile_void_refuses_voiding_an_effective_allocation`; (2) as
  defense-in-depth against a HAND-CRAFTED raw void, `in_force_allocation_exists(events, blockers)` now counts
  a `SafeHarborAllocation` as in force when non-voided **OR** still engine-effective despite a void (no
  timebar/unconservable blocker on its id — mirrors void.rs `effective_alloc`); `declare_tranche` projects
  the existing events to supply the blockers. Mutation-proven RED by
  `pre2025_tranche_refused_under_a_handcrafted_dangling_void_of_an_effective_allocation`.

- **[Minor] `safe_harbor_residue`'s throwaway projection mis-states documented remainders…** **DONE (T16)**
  — took the plan's "refuse opening the flow" alternative: `safe_harbor_residue` now returns a friendly
  `CliError::Usage` refusal when a pre-2025 tranche exists (a tranche and a safe-harbor allocation are
  mutually exclusive — D-8 — so there is no valid allocatable residue). The CLI allocate path already
  refuses earlier via `guard_allocation_vs_tranche`; the TUI opener surfaces this Err as its pre-flight
  status instead of a skewed residue (its `Err` arm already handles it gracefully). Test updated:
  `safe_harbor_residue_refuses_when_a_pre2025_tranche_exists`.

- ~~**[Nit] `--wallet` is not validated against wallets known to the vault.**~~ **DONE (P8/T14)** —
  added `cmd::tranche::wallet_is_known` (pure: an import's `e.wallet` OR a prior tranche payload) + a
  WARN (never refuse) in `declare_tranche` when `--wallet` is a phantom. Predicate test in
  `declare_tranche_cli.rs`.

- ~~**[Nit] The future-`window_end` warning computes "today" in UTC, not the filer's zone.**~~ **RESOLVED-AS-DOCUMENTED
  (P8/T14)** — `CliConfig` has NO filer time-zone field, so the Nit's "if/when one exists" precondition is
  unmet. Documented the accepted UTC caveat inline at the `main.rs` warning site (advisory-only; never
  mis-records — the lot homes at `window_end` regardless). Switch to the filer offset if/when one is added.

### Filed test-pins the review named (were only in the review doc until r2 — arch r2 Nit 2)

- ~~**[Nit] No test asserts `row.date_acquired == window_end` on a tranche 8949 row.**~~ **DONE (P9/T15)** —
  `tranche_8949_row_date_acquired_is_window_end` pins col (b) on the emitted `form_8949` row directly.
- ~~**[Nit] No Σ-conservation assert over a projection containing a tranche.**~~ **DONE (P9/T15)** —
  `sigma_conservation_holds_with_a_tranche` asserts `conservation_report(&st).balanced` over a tranche
  projection (also asserted in the `sat ≤ 0` id-guard KAT).

### From the Phase-1 fold re-review (r2, 2026-07-21)

- ~~**[Nit] SPEC.md §104 / IMPLEMENTATION_PLAN.md §556 quote the pre-split "both directions" hedge.**~~
  **DONE (T16)** — both docs now describe the DIRECTION-SPECIFIC split (`ALLOCATION_IS_FINAL_HINT` /
  `TRANCHE_IS_FINAL_HINT`) and quote each hint, noting both satisfy the normative "hedges irrevocability"
  requirement. Doc-consistency only (no code change).

## Folded during the Phase-1 gate (2026-07-21) — recorded for the audit trail

- **[Important] TUI void flows rendered a DeclareTranche as `?`/`?`** (arch r1) — FIXED: added the
  `DeclareTranche` arm to `summarize_void_payload` + a human-label KAT (mirrors the CLI sibling).
- **[Minor] Both attest-site guards untested** (tax r1 Minor 3 + arch r1 Minor 2) — FIXED: CLI +
  TUI attest-guard tests, each mutation-proven RED.
- **[Minor] Backstop KAT missed the SPEC-named inert-then-declare ordering** (arch r1 Minor 4) — FIXED:
  seq-swapped twin KAT (`backstop_fires_when_the_allocation_is_recorded_before_the_tranche`).
- **[Minor] ≥2025 non-poisoning had no assertion** (tax r1 Minor 2) — FIXED: extended CLI test (d) to
  assert the allocation stays effective + the ≥2025 tranche coexists, and (r2 tax Minor) DIRECTLY pins
  Path B via `basis_source == SafeHarborAllocated` so a no-blocker Path-A flip goes RED.
- **[Nit] Direction-crossed refusal hint** (tax r1) — FIXED: split into `ALLOCATION_IS_FINAL_HINT` /
  `TRANCHE_IS_FINAL_HINT`; (r2 arch/tax Nit) capitalized the sentence-initial "Void".
- **[Nit] KAT-G1 re-export needed a rationale comment** (arch r1) — FIXED: comment at the `lib.rs`
  re-export stating the pure-predicate exemption.
