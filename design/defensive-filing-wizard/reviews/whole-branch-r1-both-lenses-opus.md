# WHOLE-BRANCH FINAL REVIEW — round 1, BOTH LENSES (Opus) — persisted verbatim

Scope: the whole feature branch. Merge-base `35b351b` (= `main` @ v0.9.0) → head `9cbd65e`. 41 commits, 61 files, ~19.7k insertions.
Persisted retroactively by the controller (a lapse: these were folded from controller briefs before being persisted — see the
r2 fold report's concern #1). Both lenses returned NOT GREEN; the fold is `55188c1`.

---

# ARCHITECTURE LENS r1 — **NOT GREEN — 0 Critical / 1 Important / 4 Minor / 2 Nit**

The three seams are coherent and the load-bearing invariant holds. One defect in the head commit — the export step, which is
the only code on this branch that never faced a two-lens phase gate — blocks merge.

## Verified mechanically (all clean)

**Write confinement (all three paths).** `apply_declare(` / `apply_promote(` / `apply_export(` each have exactly ONE production
call site, all in `crates/btctax-tui-edit/src/edit/persist.rs` (`:443`, `:471`, `:502`). All three tokens are in
`persist_only_tokens` (`persist.rs:2055-2066`) with planted-token self-checks (`:2293-2304`). I checked the gate's one
structural weakness — `scan_non_test` latches `in_test` at the first `#[cfg(test)]` and never resets, so production code after
that marker would be invisible. Every file's first `#[cfg(test)]` immediately precedes `mod tests {` running to EOF, and
`execute_defensive_export` (main.rs:4576) sits well before main.rs's marker at 10630. The gate genuinely covers all production
code. `cmd::` is in `everywhere_tokens`.

**Layering.** The C-2 move is correct: pure predicates in `btctax-core/src/tranche_guard.rs`, `CliError`-mapping guards still in
`cmd/tranche.rs`. No duplicate definitions, no cli→core inversion. `void_targets` correctly `pub(crate)`.

**Engine change is additive.** `ShortfallRecord` + `LedgerState.shortfalls` (`state.rs:252-311`) with 6 emission sites in
`fold.rs`. There are 15 `UncoveredDisposal` blocker sites — I read all 9 non-emitting ones: 5 are "without wallet"
early-`return`s and 4 are documented-unreachable `principal == 0` degenerate guards. None carry a sat amount, and
`discovery.rs:116-127` routes exactly that class to `Triage::DataFix`. The completeness invariant holds; no filed number changes.

**CLI verbs are genuinely thin.** `cmd/tranche.rs:89-130` and `cmd/promote.rs:199-227` are `Session::open` → plan → render →
apply, with no pipeline logic left behind. The phantom-wallet `eprintln!` is correctly placed after `plan_declare` succeeds.

## Important

**I-1 — the export step plans from a possibly-stale snapshot, producing a silently incomplete IRS packet set reported as a full
success.** `crates/btctax-tui-edit/src/main.rs:4576-4616`.

`execute_defensive_export` computes the plan from `app.snapshot` (`:4581`, `:4589-4598`) but the write,
`chokepoint::apply_export` (`crates/btctax-cli/src/chokepoint/mod.rs:730-731`), re-loads `events`/`state` fresh from `session`.
So the **year set is stale while the PDF content is fresh** — and the year set is the whole DFW-D11 guarantee.

Scenario: (1) filer promotes; `persist_promote_tranche` succeeds but `build_snapshot` fails — `main.rs:4519-4523` sets "Saved
but re-projection failed … restart to refresh", closes the flow, leaves `app.snapshot` AND `app.defensive_dashboard` stale (same
shape on the declare tail, `:4224-4228`). (2) The filer presses `x`. `main.rs:493` (`app.status = None;`) runs before dispatch —
the warning is **erased by the very keypress that acts on the stale state**. (3) `plan_export` computes `flagged_years` from the
stale state, omitting a prior year the promote reordered. (4) `apply_export` regenerates every planned year from fresh data and
returns all-`Ok`; `render_export_status` reports "export: N of N year(s) written" — full success, amended-return year missing.

Vault never corrupted, no wrong write (a re-promote is caught by `would_conflict`) — an output-completeness defect, hence
Important not Critical. But it is the "silently answer for the filer" class, on the one action that emits filed artifacts.
`residue_latch_status()` covers only `attest_save_failed`/`rollback_failed`, not a failed re-projection. No test for this path.
The "Saved but re-projection failed" convention appears at 11+ sites, but everywhere else a stale snapshot only affects what is
*displayed*. This branch is the first to hang a filed-artifact generator off it.

**Fix (~3 lines):** add a `snapshot_stale` flag set in both `Err(build_snapshot)` arms and consult it in
`execute_defensive_export` — refuse with the existing wording. Cheaper alternative: rebuild the snapshot before planning.

## Minor
- **M-1** — `x` silently does nothing when `snapshot`/`session` is `None` (`main.rs:4581-4583`, `:4607-4609`) — no status, and
  `:493` just cleared it. Every other refusal path sets one.
- **M-2** — `chokepoint::promoted_filing_years` (`:626`) is public API only to serve `tests/promote_cli.rs:1562`; its sole
  production caller is in-crate (`cmd/admin.rs:89`). `pub(crate)` + a `#[cfg(test)]` shim would keep it off v0.10.0's surface.
- **M-3** — two public `render_consent`s with different signatures (`btctax_cli::render_consent(&PromotePlan)` vs
  `btctax_cli::cmd::promote::render_consent(&[ConsentTerm], &BTreeSet<i32>)`). Deliberate and documented, but a confusing first
  public surface; consider renaming the chokepoint one `render_consent_screen`.
- **M-4** — `DeclareFlowState::clearance()` (`edit/declare_flow.rs:228`) is dead code (only its own test). Already filed as
  arch-M-2 owned by P-D/whole-branch — the phase closing now. Wire it or delete it.

## Nit
- **N-1** — year derivation drifts from the chokepoint's convention: `main.rs:4586` uses `now.year()`; the chokepoint uses
  `tax_date(now, UtcOffset::UTC).year()`. Consistent with existing editor sites, so pre-existing convention.
- **N-2** — the export trio has no parity contract because it has no second driver (no CLI verb calls it). Placement is right,
  but the "both surfaces drive identical gates" rationale doesn't apply; `chokepoint_parity.rs` is promote-only. Worth a doc line.

## FOLLOWUPS triage
This review IS the P-D/ship gate, so both P-D sections are due now.
- **Era→window preset table** — a USER decision. Both prior lenses ruled it non-blocking on the merits and I concur. But
  `era.rs:17` literally reads *"This table is NOT a product-approved artifact"* — that sentence ships to crates.io. Owner call
  required before release; not a merge blocker on correctness.
- **tax-M-5 (default preset seeds a long-term holding date)** — rides the above. `DeclareFlowState::new` seeds `ALL_PRESETS[0]`,
  so `window_end` (the acquisition date) defaults to 2011-12-31 → nearly every disposal long-term at preferential rates.
  Conservative on basis, taxpayer-favorable on rate, in a tool whose thesis is conservative filing. Decide explicitly rather
  than by array order.
- **SPEC line for the provenance step** — doc-only, five minutes. Close it.
- **Close before merge (cheap, phase owns them):** arch-M-2 (dead code), arch-N-1 (one-line `debug_assert` tightening, 3 sites).
- **Highest value before a public release though rated Minor:** tax-M-3 + tax-M-4 (missing displacement caveats on a
  filer-facing tax-Δ), and tax Minor 2 (no scrolling; `shown_terms` can record a term as SHOWN that was never rendered — the
  only open item with a §6664(c) artifact stake).
- **Can genuinely follow:** arch-M-1 (confirm-tail dedup), Browse-footer `w`, arch N-r2-1/2/3, N-r2-4 residue.

## Merge recommendation
**Hold the merge on I-1 only.** ~3 lines plus one KAT in the style of the existing
`x_exports_both_a_promoted_2025_leg_and_a_2024_removal_reordered_year_including_form_8275` (`main.rs:14108`). Fold M-1 in the
same pass; close arch-M-2 and arch-N-1; re-review. Everything the phase gates certified holds up. The one gap is precisely where
the process predicts it — the commit that shipped after the last gate.

---

# TAX LENS r1 — **NOT GREEN — 0 Critical / 2 Important / 5 Minor / 3 Nit**

The filed-number spine is sound and I confirm the three per-phase gates' conclusions rather than re-litigating them: every
dollar the wizard shows or writes comes from a shipped primitive; both mutating verbs are thin drivers over
`chokepoint::plan_*`/`apply_*` with the gate order written once (`chokepoint/mod.rs:320-429`); BG-D6's ack is enforced inside
`apply_promote` on the filer's own buffer (`main.rs:4483`); BG-D5 provenance is now an engine-refused filer selection; every
shadow projection forces `pseudo_reconcile = false`; write confinement survives all three new write paths. The §6664(c) chain is
unbroken **within** the promote branch: `plan.terms` → `render_consent(&plan)` → `Acknowledgment.shown_terms` → the 8275 Part I
amounts read off the as-filed `leg.basis`, with `promote_export_gate` refusing any year whose Part II is empty.

What blocks is at two seams no per-phase gate could see. The last two commits (`0a6cf21`, `9cbd65e` — Task 10 / P-D, the export
step) landed **after** the P-C gate reviews and have had no two-lens gate of their own; both Importants live there or in the item
that gate explicitly re-owned to P-D/ship.

## Important

**I-1 — The export step systematically omits the year a `$0`-only declare fixed; the conservative branch and the promote branch
are not "two equal branches" at the journey's terminal step.** `chokepoint/mod.rs:699-701`:
```rust
let mut years = conservative::flagged_years(events, state, prices, tables, cfg, current_year);
years.insert(current_year);
```
`flagged_years` (`crates/btctax-core/src/conservative.rs:1028`) iterates `live_promote_ids` (`:923`) only — a vault with
`DeclareTranche` decisions and **no** `PromoteTranche` yields the empty set. So the wizard's export set for the `$0` branch is
exactly `{current_year}`.

But a `$0` declare *does* materially change the shortfall year's filed forms. `make_disposal_legs`
(`crates/btctax-core/src/project/fold.rs:128-147`) splits the full `net` proceeds **pro-rata across `consumed`**, so a short
disposal already reports full proceeds against partial basis. Adding the tranche leaves the year's *total* gain unchanged but
re-splits the proceeds and gives the uncovered share its own leg with `acquired_at = window_end` — i.e. it changes the Form 8949
row set and the **short-term/long-term split on Schedule D**, and it clears the Hard `UncoveredDisposal` that made the year
not-computable. Under an elected FIFO it also re-orders documented lots across years.

Concrete failure: a 2024 sale of coins bought on LocalBitcoins with no records. In 2026 the filer opens the wizard, declares a
`$0` tranche covering the 2024 shortfall — the end-state SPEC DFW-D3/G-1 calls "complete, conservative" — and presses `x`.
`plan.years = {2026}`; 2026 has no bundled IRS template, so the status reads `export: 0 of 1 year(s) written … — 2026 failed:
unsupported tax year 2026`. The 2024 packet — the only year the journey changed, and a year whose templates *are* bundled — is
never planned. Had the same filer taken the **aggressive** branch and promoted, `flagged_years` would have caught 2024 and
written it. Nothing surfaces the omission: `DefensiveFilingView.flagged_years` is computed by `journey_view`
(`defensive/mod.rs:681`) and **never rendered** by `render_dashboard`.

This is the feature's own headline guardrail (G-1 "filer choice, never a default"; DFW-D3 "two equal branches") failing at the
seam. Every SPEC/plan review round refined the *promote*-driven year set; no round asked what a declare alone changes.

**Fix:** extend `plan_export`'s set with the declare-side equivalent — a `promote_changed_years`-shaped
with/without-this-`DeclareTranche` fold diff over every live tranche (disposal ∪ removal legs, `< current`), unioned in. Amend
SPEC DFW-D11 to state the union explicitly, and add the mirror KAT (mutation: promote-only set → the declare-fixed prior year
drops → reds).

**I-2 — The era-preset default answers the filer's acquisition-date question in the taxpayer-FAVORABLE direction, and it is due
at this gate.** `edit/declare_flow.rs:71` seeds `ALL_PRESETS[0]` (2009-01-03 … 2011-12-31), and `:74-78` clamps `window_end` to
the before-op day. `window_end` **is** the lot's acquisition date, so on the `$0` branch — where the window has no effect on
basis at all, since basis is `$0` regardless — the window's *only* filing-substantive effect is the holding-period character.
Defaulting to the oldest bucket makes essentially every covered disposal **long-term** at preferential rates. The code's own
justification for oldest-first (`declare_flow.rs:66-69`, "most conservative — DFW-D9's 'wider window → lower floor' bias") is a
*basis*-axis argument that does not apply on the branch where it is the default.

The prior tax lens raised exactly this (`pc-gate-tax-opus.md` M-5), correctly rated it non-blocking **for P-C**, and re-owned it
— with the whole era table — to **P-D/ship** (`FOLLOWUPS.md:53-70`). That is this gate. Per the standing rule ("Phase-owned or
gating → burn down in/before that phase closes green… not deferrable past its owning phase"), it cannot cross this merge open.
Reinforcing that: `crates/btctax-core/src/defensive/era.rs:14-19` still says in shipped source **"This table is NOT a
product-approved artifact"** — v0.10.0 would publish, to crates.io and to real filers, a filing-substantive reference table whose
own module doc disclaims approval.

I am not overturning the P-C adjudication; I am reporting the item as **due**. Discharging it may be a one-line owner
ratification rather than a code change — but if ratified as-is, `era.rs`'s provisional language and the `FOLLOWUPS.md` entries
must be updated in the same pass, or the shipped source contradicts the shipped decision.

Three tax sub-decisions ride with it: (a) which preset is default (this finding); (b) the cycling clamp (fixed at `47225af`,
re-check against any new table); (c) there is no ≥2025 bucket (M-3 below).

## Minor
- **M-1** — the only saving figure the wizard ever displays is the uncaveated one, and the caveat attaches to a figure that is
  never drawn. `TrancheRow.clamped_saving` is computed by `journey_view` (`defensive/mod.rs:458-462`, two full projections per
  realized year per unpromoted tranche) but `render_tranche_row` (`defensive_dashboard.rs:281-303`) never renders it.
  Consequently `Advisory::WouldDisplaceIfPromoted`'s copy ("any saving/gain-Δ **shown above** would UNDERSTATE the gain…")
  points at nothing, while the one saving the filer *can* see — the declare flow's on-demand tax-Δ — carries no displacement
  caveat at all. This inverts the premise of filed follow-up `tax-M-4`.
- **M-2** — the export plan unconditionally includes `current_year`, guaranteeing a reported failure at release.
  `btctax_forms::SUPPORTED_YEARS = &[2017, 2024, 2025]`, so in 2026 every filer's `x` reports "2026 failed". Worse,
  `export_irs_pdf_from_session` `mkdir`s the year directory and writes `basis_methodology.txt`/`form_8275.txt` *before* the PDF
  fill errors, leaving a half-populated `out_dir/2026/`.
- **M-3** — no ≥2025 era bucket, so a post-2024 shortfall has no reachable truthful window; reaching the true date needs ~150
  single-day presses. The practical outcome is a filer attesting a window they know is wrong — undermining the very attestation
  the Cohan/§6664(c) footing rests on — again in the earlier-is-favorable direction.
- **M-4** — flow renders have no scrolling; a promote's consent terms can be recorded as `shown_terms` without being drawn.
  Largely self-limiting (the ack prompt renders after the terms), but this is the §6664(c) artifact on the aggressive branch.
- **M-5** — the declare flow warns about the tranche⇄safe-harbor exclusion in one direction only: nothing tells the filer that
  the *default* window makes `pre2025_tranche_exists` true and thereby forecloses any future Rev. Proc. 2024-28 safe-harbor
  allocation. Recoverable, but it is a filing election taken by a default the filer never made.

## Nit
- **N-1** — `"[optional, SUPPRESSED] promote"` reads as disabled, but the `'p'` arm still promotes a fee-only row (correctly).
- **N-2** — `{:?}` Debug formatting on filer-facing lines (`Decision { seq: 1 }`, `SelfCustody { label: … }`, `Y2009To2011`) —
  on the wizard's *first* screen in a public v0.10.0.
- **N-3** — `triage` silently drops an open-acquisition blocker whose raw event carries no wallet
  (`defensive/discovery.rs:102-108`), so such a shortfall is offered as a clean `DeclareCandidate`. Backstopped by
  `Advisory::OverCovered` + revocability.

## Merge recommendation
**Hold.** Do not merge or cut v0.10.0 until (1) the export year-set gap (I-1) is fixed with its KAT and the SPEC DFW-D11
amendment, and (2) the owner discharges the era-table decision including the default preset (I-2), with `era.rs`'s own
provisional disclaimer updated to match. Both contained. Everything else is merge-ready — the chokepoint extraction is
behavior-preserving, the DFW-D6 pseudo-off fix is a genuine correction to a shipped sub-project-1 defect, and no path I found
lets the tool file a wrong number or record an `Acknowledgment` that diverges from what the filer was shown.
