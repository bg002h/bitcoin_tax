# Defensive Filing Wizard — IMPLEMENTATION_PLAN tax-correctness review (Opus, r1)

Lens: US-federal-tax-correctness. Re-derived from source (`cmd/promote.rs:364-488`,
`cmd/admin.rs:78-116`, `cmd/tranche.rs:107-175`, `conservative.rs:689`, `conservative_promote.rs:50-130,258,487`,
`project/fold.rs` six sat sites, `project/mod.rs:107-119`, `resolve.rs:1300-1320`, `transition.rs:95-110`,
`event.rs:328-368`, `tax_tables.rs:73-79`). Verdict below is about the PLAN implementing the GREEN SPEC.

## Verdict

**NOT GREEN** — **0 Critical / 3 Important / 3 Minor / 1 Nit**

The plan's tax spine is sound: the gate ordering it pins matches `promote.rs:378-485` exactly; the
DFW-D6 pseudo-off fix is tax-correct (I confirmed `would_conflict` forces `pseudo_reconcile=false` at
`project/mod.rs:119`, so `would_conflict`-in-`apply` inherits it; `filed_basis_for` reads prices+window only,
so the fix moves ONLY `shown_terms`, not a filed number, and no shipped `promote_cli.rs` KAT exercises a
pseudo-active promote → nothing flips at Step 5). The `export_year_set` fold-diff (disposal AND removal, `<
current` ∪ {current}) captures the 1040-X set correctly and the removal-reorder KAT is the right acceptance
test. Bundled tables are exactly {2017,2024,2025,2026}; the DFW-D10 flavor gate is right. No task introduces a
filed number outside the shipped primitives; `state.shortfalls` is additive.

Three SPEC §5-MANDATED KATs are absent from the plan's enumerated KAT lists — each leaves a real tax/UX
guarantee unpinned, so a regression ships green.

## Important

### I-1 (Task 5 / DFW-D4.1, D4.3) — no pending-out routing KAT; the C-1 double-count guard is unpinned
SPEC §5 mandates "a `pending-out` short routes through `UnmatchedOutflows` first." The plan's Task 5 Step-1 KAT
list omits it (only self-transfer, gift/donate-without-wallet, unclassified, principal+fee, detail-grep). The
triage PROSE is correct ("pending-out → ResolveFirst via its co-emitted `UnmatchedOutflows`" — and I confirmed
`fold.rs:833`+`:854` co-emit both on the same event), but nothing tests it. **Wrong filing it permits:** a
mutation/regression that lets a `pending-out` short surface as a `DeclareCandidate` lets the filer declare a
tranche, then a later `TransferLink` matches the same outflow to a real inflow → the tranche's sats PLUS the
linked basis double-count (the C-1 double-count DFW-D4.3 exists to prevent) → understated gain.
**Fix:** add `pending_out_short_routes_resolve_first_via_unmatched_outflows` (a pending-out short → `ResolveFirst{UnmatchedOutflows}`, ZERO `DeclareCandidate`); mutation: route it to `DeclareCandidate` → reds.

### I-2 (Task 6 / DFW-D10) — no "computable-table year, no TaxProfile" flavor KAT; bare-$X guard unpinned
SPEC §5 mandates TWO cases: "a no-`TaxProfile` year AND a 2018–2023 year each render the gain-Δ+uncomputable /
named-unquantified flavor, not a dollar." The plan has only `uncomputable_audience_year_2020` (the no-table
door). `clamped_promote_year_saving` returns `$0` when `crypto_tax_of`→`None` for ANY reason (no table / no
profile / Hard blocker; `conservative_promote.rs:504`). **Failure it permits:** an implementation that gates
`SavingFlavor::ComputedTax` on table-presence ALONE (2024 has a table) but forgets the profile/blocker check
renders a bare `$0 (2024)` for a no-profile year — the exact DFW-D10 "never a bare $X for a non-computing year"
violation — and every listed KAT still passes. **Fix:** add a KAT: a stored-table year (e.g. 2024) with NO
`TaxProfile` → `SavingFlavor::Uncomputable{gain_delta}`, never `ComputedTax`/a dollar; mutation: drop the
profile/no-Hard-blocker conjunct from the flavor predicate → reds.

### I-3 (Task 6 / DFW-D5 "didn't-cover") — `TrancheStatus::DidNotCover` is defined but never tested
SPEC §5 mandates "a live tranche whose pool matches an unresolved short renders the pool-level 'still short —
don't declare again' state." Task 6 defines `TrancheStatus::DidNotCover{pool_short_sat}` but its Step-1 KAT list
has no test for it (nor for the pool-level, not-per-tranche, attribution DFW-D5.3/arch-I-4 requires).
**Failure it permits:** an implementation that never emits `DidNotCover`, or emits it as a per-tranche
attribution, ships green; a filer whose tranche missed the pool/window sees no "still short" state, declares a
SECOND tranche over the same shortfall, and once both cover, files an over-stated basis (understated gain) —
and the wrong `pool_short_sat` is a filer-facing number. **Fix:** add a KAT — a live `DeclareTranche` on
`pool_key(we,wallet)` with `we ≤ short-op date` but insufficient sat renders ONE pool-level `DidNotCover{pool_short_sat=S}`
row (not a per-tranche one); mutation: attribute the residual per-tranche / drop the pool match → reds.

## Minor
- **M-1 (Task 6 / DFW-D5.3 tax-N-1):** the positive `NowDisplacing` KAT ("basis_source composition diff") does
  NOT kill a "bare leg-set inequality" mutation — a legitimate documented-lot HIFO reorder (the shipped
  `mixed_vintage` shape) would false-fire `NowDisplacing`, misadvising a filer to void a legitimate promote
  (over-conservative, advisory-only, no understatement). Add a NEGATIVE KAT: a documented-only reorder (leg set
  differs, composition unchanged) → NO `NowDisplacing`.
- **M-2 (Task 3 / DFW-D11):** the plan should EXTRACT a shared `flagged_years(promote_id)->BTreeSet<i32>` from
  `promote_prior_year_advisory` (as it explicitly does for `promoted_filing_years`) and have `export_year_set`
  consume it, not re-derive the fold-diff — else a replicated enumeration that drops `gift_changed`/`don_changed`
  silently drops a removal-reordered prior year from the 1040-X set (`conservative.rs:755-758` is the source).
- **M-3 (Task 2):** the `None`-path thin driver must still emit the shipped phantom-wallet stderr warning
  (`tranche.rs:158-165`) for byte-for-byte behavior-preservation; `plan_declare` returning a pure `DeclarePlan`
  moves that I/O to the driver — state it, and let `declare_tranche_cli.rs` hold it.

## Nit
- **N-1 (Task 1):** `Refusal::Conflict` is listed in the enum, but `would_conflict` lives in `apply_promote`
  and surfaces as `CliError`, not a plan-time `Refusal` — `plan_promote` can never return `Conflict`. Drop the
  variant or document it as apply-only.

## Confirmed correct (adversarially checked, no finding)
- Gate ordering / ack-inside-`apply` / consent-printed-before-ack (`promote.rs:451-458`) preserved; parity KATs
  drive both full drivers.
- DFW-D6 pseudo-off is a `shown_terms`-only change; non-pseudo vaults byte-identical; no shipped promote KAT
  flips.
- Declare clearance is event-level (DFW-D7); before-op prefill grounded in `resolve.rs:1310` (`window_end` =
  holding start, `src_priority=u8::MAX` → decisions sort after same-instant imports, so `we==disposal date`
  cannot cover); `Some`-clearance backstops even a mis-prefill; CLI `None` path spec-sanctioned.
- OverCovered scope (`covered_sat>0 ∧ live>covered`, not fully-undisposed), sat-count mutation, three-flavor
  clamped-only saving all tax-correct.
