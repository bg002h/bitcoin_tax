# Defensive Filing Wizard — IMPLEMENTATION_PLAN tax-correctness review (Opus, r2)

Lens: US-federal-tax-correctness. Re-derived from source (`conservative_promote.rs:197-205,258-273,487-505`,
`conservative.rs:689-761`, `project/fold.rs:388,710,826-857,876,1196,1274`, `cmd/tranche.rs:40-165`) and the
post-fold plan @ 333f79b. Verdict is about the PLAN implementing the GREEN SPEC.

## Verdict

**GREEN** — **0 Critical / 0 Important / 3 Minor / 1 Nit**

## r1-resolution note (all 3 r1 Importants + both flagged Minors folded correctly)

- **tax-I-1 RESOLVED.** Task 5 adds `pending_out_short_routes_through_unmatched_outflows_first`
  (line 280): a pending-out short → `ResolveFirst`, ZERO `DeclareCandidate`. Confirmed real at source —
  `fold.rs:831` (`UncoveredDisposal` "pending out short") + `:854` (`UnmatchedOutflows`) co-emit on the SAME
  `eff.id`, so a mutation routing it to `DeclareCandidate` reds and the C-1 double-count (tranche sat + a
  later `TransferLink`'s basis) is pinned. Correct.
- **tax-I-2 RESOLVED.** Task 6 adds `table_year_with_no_TaxProfile_shows_uncomputable_not_a_bare_dollar`
  (line 328). Mutation is real: `crypto_tax_of` (`conservative_promote.rs:197-205`) returns `None` on a
  missing profile, so `clamped_promote_year_saving` hits the `_ => Usd::ZERO` arm (`:505`) — a table-only
  flavor gate would render a bare `$0 (2024)`; the KAT forces `Uncomputable{gain_delta}`. Kills the DFW-D10
  "never a bare $X for a non-computing year" mutation.
- **tax-I-3 RESOLVED (improved).** The per-tranche `TrancheStatus::DidNotCover` is gone; `TrancheStatus` is
  now `{DeclaredZero, Promoted}` and the pool residual moved to a view-level `still_short: Vec<PoolShort>`
  (lines 298-305), with `a_live_tranche_not_clearing_its_pool_shows_pool_still_short` (line 329). This
  structurally forbids per-tranche attribution (DFW-D5.3) and kills both r1 mutations (per-tranche attribute
  / drop the pool match). Correct.
- **tax-M-1 folded:** the negative `now_displacing_uses_basis_source_composition_not_leg_set_inequality`
  (line 326) now pins the false-fire guard. **tax-M-2 folded:** a shared core `flagged_years(...)->BTreeSet`
  (line 206) is extracted from `promote_prior_year_advisory`, `plan_export.years = {current} ∪ flagged_years`,
  `⊋ promoted_filing_years` — matches `conservative.rs:756-758` (disposal ∪ donation ∪ gift diff).

The arch-driven moves are tax-sound: `journey_view(tables: &dyn TaxTables)` gates the three-flavor
`SavingFlavor` on `table_for(y).is_some() ∧ profile ∧ no-Hard-blocker` (lines 310-312), matching the shipped
`consent_terms` discipline; the C-2 predicate move to core keeps `guard_tranche_vs_allocation`'s gate in the
chokepoint (safe_harbor is a READ-only precheck); routing writes through `persist_*` changes no filed-number
path (every filed number still flows through the characterization-pinned `apply_*`). No new wrong-filing path;
every SPEC §5 minimum KAT is present.

## Minor

- **M-1 (Task 5/6, DFW-D3/D7, arch-I-4) — the `kind` retention for `FeeOnlyPromoteNoop` is unbuildable as
  typed + unpinned.** The File-Structure map (line 72) says `state.shortfalls: Vec<Shortfall>` retains the
  principal-vs-fee `kind` "for `FeeOnlyPromoteNoop`", but `Shortfall` (line 261) has NO `kind` field and
  `shortfalls()` (line 264) sums principal+fee per event — so the distinction the advisory needs is dropped,
  and `FeeOnlyPromoteNoop` (Advisory, line 299) has no core KAT (only the UX suppress in Task 7(e)/Task 9).
  Also uneven at source: `fold.rs:827` already combines `*sat + fee_sat` into ONE pending-out blocker, so
  `kind` isn't uniform across the six sites. **Non-gating:** SPEC DFW-D3 classes this "UX, not a
  filed-number issue" — the promote stays behavior-preserving and BG-D4 fee-evaporation leaves the filed gain
  unchanged either way, so a mis-derived/absent advisory misfiles nothing. **Fix:** give the RAW
  `state.shortfalls` records a `kind`, aggregate (dropping it) in `shortfalls()`, and add a core KAT
  (fee-only-coverage tranche → `FeeOnlyPromoteNoop`; principal-coverage → none) — or drop the I-4 `kind`
  claim if the advisory is deferred.
- **M-2 (Task 6, DFW-D5.3) — the `still_short` KAT doesn't assert the residual value.** Line 329 asserts
  "ONE `PoolShort`" but not `short_sat`/`live_tranche_sat`, so a wrong-residual mutation survives. The
  "still short by S" figure is filer-facing (advisory, NOT a filed number → Minor). **Fix:**
  `assert_eq!(pool_short.short_sat, S)`.
- **M-3 (Task 2, carry-over r1 M-3) — phantom-wallet stderr warning preservation still unstated.** The
  `None`-path thin driver must keep the shipped `eprintln!` (`tranche.rs:159`) for byte-for-byte
  behavior-preservation; `plan_declare` returning a pure `DeclarePlan` moves that I/O to the driver.
  Tax-neutral ($0 tranche), non-gating. State it; let `declare_tranche_cli.rs` hold it.

## Nit

- **N-1 (Task 1, carry-over r1 N-1):** `Refusal::Conflict` is still in the enum (line 124), but
  `would_conflict` lives in `apply_promote` → `CliError`; `plan_promote` can never return it. Drop the
  variant or document it apply-only. (Also line 66's core `pre2025_tranche_exists(events, we)` adds a `we`
  arg the cli predicate `tranche.rs:71` lacks — reconcile the signature; tax-neutral.)

## Confirmed correct (adversarially checked, no finding)
- Flavor gate consistency: table+profile+no-Hard-blocker ⟹ BOTH promote folds price (the promote adds
  basis, never a blocker; the `DeclareTranche` baseline survives in both), so `journey_view`'s independent
  flavor gate never disagrees with `clamped_promote_year_saving` → no bare `ComputedTax{$0}`.
- Export set: `flagged_years` unfiltered + `∪ {current}` is a correct superset of `promoted_filing_years`;
  the removal-reordered-2025-donation KAT (Task 3) is the right acceptance test.
- `still_short` predicate: `we ≤ short date` bound is correct — a `we > short date` tranche genuinely
  doesn't cover (refused at clearance), so its non-firing can't hide a duplicate-declare hazard.
- Clearance pseudo-off (Task 2, arch-I-5) + `journey_view_forces_pseudo_off` (Task 6) pin DFW-D6 so a
  synthetic `SelfTransferMine{$0}` can't mask a real shortfall.
