# WHOLE-BRANCH FINAL REVIEW — round 2, BOTH LENSES (Opus) — persisted verbatim

Scope: the fold of r1's blockers. Base `9cbd65e` → head `eb840ed` (4 commits: README, the whole-branch fix `55188c1`, the
owner-ratified era decisions `46c6ac5`, the era↔SUPPORTED_YEARS drift guard `eb840ed`).
Arch → **GREEN, "ship it"**. Tax → **NOT GREEN 0C/2I** (both doc-only). The r2 fold is `f791729`.

---

# ARCHITECTURE LENS r2 — **GREEN — 0 Critical / 0 Important / 6 Minor / 5 Nit**

## Resolution audit

**I-1 (stale-snapshot short packet set) — RESOLVED, by construction.** `main.rs:4618-4639` re-projects before planning. I
verified the closure argument rather than taking it: `btctax_tui::unlock::build_snapshot` (`unlock.rs:171-219`) calls
`session.load_events_and_project()` — **literally the same call** `apply_export` makes (`chokepoint/mod.rs:761`) — and
`cli_config = session.config()?`, which is what `plan_export`'s `cfg` derives from. Plan inputs and apply inputs are the same
read of the same DB. No interleaving is possible (single-threaded TUI, one synchronous keypress, `VaultLock` excludes other
processes) — **no new race**. The refusal path writes nothing and states why. The `None` (no session) arm falls through to
`plan_export`'s own "no open vault session" refusal before any write.

Crucially this is **stronger than the latch I proposed**: it does not enumerate the ways a snapshot goes stale, so it also
covers staleness sources that emit no marker at all (including any future write path that forgets to arm a flag).

**Judgment on latch-vs-re-projection: the implementer was right, and the 24-emitter finding is real.** I counted at source.
Production side of `main.rs` (tests start at line 10705): **25** `"Saved but re-projection failed"` emitters and **30**
`build_snapshot` call sites. A latch would have been a ~55-site cross-cutting change with a stuck-flag failure mode that refuses
valid exports — for a *weaker* guarantee than 15 local lines buy. Rejecting option (i) was correct, and reverting the built latch
rather than shipping it was the right call. Cost is one extra projection per `x` keypress, dwarfed by the multi-year PDF fill it
precedes; the retained refresh is a real bonus.

| r1 item | Status |
|---|---|
| **M-1** status on every `x` refusal path | **FOLDED** — KAT `x_with_no_loaded_ledger_refuses_with_a_reason_never_a_silent_no_op` |
| **M-2** `promoted_filing_years` public-for-a-test | **FOLDED** — `pub(crate)` at `:632`; sole production caller in-crate; verified NOT re-exported from `lib.rs` |
| **M-3** two public `render_consent`s | **STILL OPEN** (see M-6) |
| **M-4** dead `clearance()` | **FOLDED — deleted**, not wired. The right call: wiring it would create the second gating authority DFW-D1 forbids |
| **N-1** `debug_assert!(<=1)` | **FOLDED** at all three sites |

Write confinement verified mechanically: the three `apply_*(` tokens appear outside `edit/persist.rs` only in a doc comment;
every `fs::write`/`create_dir` in `main.rs` is past line 14888 (test region). **Intact after both fix waves.**

## NEW findings

**M-1 — the era table's pooling rationale is factually wrong, and it is now in the SPEC.** `era.rs:45-48` (repeated in
`defensive_era.rs` and **SPEC.md DFW-D9**) asserts *"`pool_key` puts a pre-2025 lot in the Universal pool and a 2025+ lot in its
wallet's own pool, so a pre-2025 tranche cannot cover a post-2025 disposal in the same wallet."* That is false under Path A —
the default. `project/transition.rs:96-106` moves every remaining Universal lot into `PoolKey::Wallet(lot.wallet)` at the
cutover, and carries an explicit D-8 carve-out written *for this exact case* (`if lot.basis_source !=
BasisSource::EstimatedConservative { ... }`). Under Path B the claim is vacuous, because a pre-2025 tranche is refused beside an
in-force allocation anyway. The **decision is still correct** — it has a different reason: (a) a filer who genuinely acquired in
2025 must be able to *say so*, and (b) for a Path-B filer with a 2025+ shortfall, `Y2025Onward` is the **only** reachable bucket.
Fix the causal clause in all three places. I considered Important (an unsound assumption written into the contract, from which a
future phase could suppress pre-2025 presets for a post-2025 shortfall — re-answering the era question in exactly the direction
this owner decision forbids); it lands at Minor because it drives no behavior. **Fix in this cycle rather than deferring.**

**M-2 — `era::next_preset` is dead public API kept on a false justification.** `era.rs:89` justifies retention as an
*"already-published accessor"*. Verified: `git cat-file -e main:crates/btctax-core/src/defensive/era.rs` → **absent**. The module
is new on this branch; `next_preset` has never been published, and has zero production callers. Delete it with its two KATs, or
fix the justification.

**M-3 — `flagged_years` is now O(live declares + live promotes) × 2 full projections, on the dashboard path.** A 15-shortfall
ledger now costs ~30 full projections per dashboard refresh, on top of `clamped_saving`'s own cost. Pre-existing class, cached at
entry (DFW-D10 forbids per-keystroke recompute), so not a freeze-per-frame defect — but the constant got materially worse on an
interactive path with no memoization.

**M-4 — the declare flow's persist-error arm discards the filer's era attestation.** `main.rs:4233-4236`. `PersistError`'s
contract is that `NoChange`/`RolledBack` wrote nothing — yet the flow (era pick + window/sat nudges) is thrown away. Precisely
the defect **arch M-3 fixed for the promote flow** at the P-C gate. The owner's explicit-pick change raised the stake: the era
pick is now a *required filer attestation*. The refusal arm above it (`4188-4197`) gets this right.

**M-5 — residual stale-snapshot exposure on the declare write path.** `declare_flow_confirm` runs `plan_declare` fresh, but over
`snap.events` — the possibly-stale image. Unlike `apply_promote` (which re-loads and re-runs `would_conflict`), **`apply_declare`
appends verbatim with no fresh re-gate**. Not blocking: the payload is the filer's own input, the direction is taxpayer-adverse
($0 overstates gain), it is revocable, `OverCovered` surfaces it. But the FOLLOWUPS justification ("no writer re-derives a filed
number from `app.snapshot` without its own fresh `plan_*`") is a shade generous: the `plan_*` is fresh, its **input** is not.

**M-6 (carried, r1 M-3) — two public `render_consent`s survive to v0.10.0.** Same name, different arity, one calls the other.
The second is `pub` to serve four assertions in `tests/promote_cli.rs` — the *identical* situation that got
`promoted_filing_years` demoted. Mitigating: it is pre-existing published API (v0.9.0).

## ★ Public API for v0.10.0
New public surface: `btctax_core::{defensive::{mod, discovery, era}, tranche_guard, conservative::flagged_years}`;
`btctax_cli::{chokepoint::*, ProvenanceKind, PROVENANCE_TEXT, IrsPdfReport}`. Correctly narrowed on this branch:
`promoted_filing_years` → `pub(crate)`, `export_irs_pdf_from_session` → `pub(crate)`, `guard_tranche_vs_allocation` →
`pub(crate)`, `clearance()` deleted.

- **Nit-N1: fixed-length array consts.** `era::ALL_PRESETS: [EraPreset; 6]` and `ProvenanceKind::ALL: [ProvenanceKind; 7]` bake
  the length into the public type. This branch just changed one 5→6, and the census drift guard **actively schedules the next
  change**. `&'static [T]` would make that additive instead of breaking. Cheap now, breaking later.
- **Nit-N2:** `ExportPlan` has all-pub fields and no `#[non_exhaustive]`.

The era drift guard is well placed: `btctax-forms` depends on `btctax-core` and core does not depend on forms — no cycle, and it
is the only crate that can see both constants.

## Era state machine
Coherent. `preset`/`window_start` move together; both nudges are inert while `window_start` is `None`, so a window cannot be
materialized by a side door; `review()` and `declare_flow_confirm` both fail closed; the pick lives outside `step` and survives a
bounce and a confirm-tail refusal. No unreachable state, no path to confirm without a pick. DFW-D5 before-op invariant intact.

**The replaced KAT is the right *invariant* but the wrong *stated reason*.** `pool_key(date, wallet)` takes a **single** date, so
a lot always lands in exactly one pool regardless of straddling — the "so" is a non-sequitur (**Nit-N3**). What the old guard
actually protected was strictly stronger and *deliberately abandoned*. Abandoning it is correct (both directional consumers key
on `window_end < TRANSITION_DATE`). The no-straddle property earns its keep as a *table-coherence* invariant, and the two added
reachability assertions (at least one bucket each side) are the genuinely valuable part.

Remaining Nits: **N4** keys `7`–`9` silently swallowed while the picker advertises `1-6`; **N5** `main.rs:4149`'s doc still cites
*"via `flow.clearance`"*, deleted by this same wave; the re-projection refusal copy misattributes a wider failure surface; the
universal-2026 case renders "0 of 0 year(s) written under `<path>`" naming a directory never created.

## Merge recommendation
**Ship it.** Both r1 blockers genuinely closed — I-1 by construction rather than bookkeeping, and the implementer's rejection of
my own suggested latch was the better engineering call on evidence I independently confirmed. Before tagging I would fold **M-1**
(the false pooling claim, three lines, and it is in the SPEC) and **N5**, and make a call on **M-2**, since narrowing public API
after the first crates.io release is the one thing this gate cannot undo.

---

# TAX LENS r2 — **NOT GREEN — 0 Critical / 2 Important / 6 Minor / 4 Nit**

Both r1 Importants are genuinely resolved, and the arch I-1 fold closes the stale-image hazard for filed artifacts. The two new
blockers are both *false statements shipping publicly at v0.10.0* — one in the SPEC + a published `btctax-core` API doc, one in
the README's description of the tax-aggressive branch. Neither changes a filed number today; both are doc-only folds.

## Resolution audit

**r1 I-1 (export omitted the `$0`-declare year) — RESOLVED.** `conservative.rs:1077-1105` unions the per-live-**declare** diff
alongside the per-live-**promote** one. **No new tax logic:** `promote_changed_years` → `decision_changed_years`
(`conservative.rs:998-1052`) is the same body — forced `pseudo_reconcile = false` on an own copy, disposal ∪ removal candidate
years, `retain(|y| *y < current)`, per-decision (never whole-state) union. The criterion was already decision-agnostic. **The
orphaned-promote argument checks out:** dropping only the `DeclareTranche` in the without-fold removes the tranche's *whole*
contribution (`resolve.rs:521-543` refuses a promote whose target is not a live `DeclareTranche`; `:442` folds it as `Op::Skip`).
**It really is the pro-rata re-split** (`make_disposal_legs` + `resolve.rs:~1310`). **KAT is real and mutation-killing**
(`promote_cli.rs:1652`): a 2024 sale with no records + a `$0` declare, `assert!(state.promoted_origins.is_empty())` as a fixture
gate, then 2024 ∈ `flagged_years` **and** ∈ `plan_export.years`; reverting to the promote-only union yields `{}` → both red.
**Branches are now equal at the terminal step** (`chokepoint/mod.rs:720-733` builds one candidate set for both forks, then
partitions). SPEC DFW-D11 amended.

**r1 I-2 (era default) — RESOLVED (structurally).** `preset: Option<EraPreset>` / `window_start: Option<TaxDate>`, both open
`None`. An unanswered window is **unrepresentable**, not merely undisplayed. `window()` is the single accessor and *every*
consumer routes through it — `floor_readout`, `holding_date`, `is_long_term_at_short_date`, `compute_tax_delta`, the render, and
the confirm tail. Fail-closed at **both** gates (`review()`; `declare_flow_confirm`). Nudges inert with no pick — no side-door
window creation. `Tab` gone; only `1..=N`. An inapplicable preset is **refused with the reason**, previous pick untouched —
strictly stronger than the old silent skip, and it closes the degenerate-one-day-window hazard. Disclaimer removal is honest, not
an upgrade in claim. `Y2025Onward` starts *exactly* at `TRANSITION_DATE`, so nothing straddles; clamped by the DFW-D5 prefill it
always yields a post-cutover, **short-term** acquisition date (the conservative direction). The replacement invariant is stronger
than what it replaced — it also asserts *both* sides of the cutover are reachable, so it cannot be satisfied vacuously. Census
drift guard mutation-verified. But the *stated reason* for the bucket is false — I-1 below.

**arch I-1 — EFFECTIVE.** Both sides now bottom out in the *same* `session.load_events_and_project()` with the same
`config()`-derived `ProjectionConfig`; `snap.prices` is documented+built as byte-identical to `session.prices()`. Plan year-set
and packet content are one image **by construction**. KAT
`x_replans_off_a_fresh_projection_so_a_stale_snapshot_cannot_shorten_the_year_set`.

## NEW Important

**I-1 — "a pre-2025 tranche cannot cover a post-2025 disposal in the same wallet" is FALSE, and it is the stated justification
for a ratified spec decision and a replaced KAT.** Where: `era.rs:44-48` (**published API doc → docs.rs at v0.10.0**);
`defensive_era.rs:60-63`; **SPEC.md DFW-D9**; and (pre-existing) `main.rs:14092-14097`.

Why false: with no `SafeHarborAllocation`, `resolve.rs:1595` selects `TransitionMode::PathA`; at the first `≥ TRANSITION_DATE`
event (`fold.rs:510-514`) `seed_transition` **drains** every Universal residue lot into `PoolKey::Wallet(lot.wallet)`
(`transition.rs:93-106`), *explicitly preserving* `BasisSource::EstimatedConservative` because — in that function's own words —
otherwise "the tag never reaches 2025+ disposal legs". The engine plainly contemplates the case the doc says is impossible.

**Refuted by a green test in this workspace:** `crates/btctax-core/tests/kat_tranche.rs:331
tranche_tag_survives_2025_path_a_seed_and_reaches_a_2025_disposal_leg` — a 2015-window tranche in wallet `w` fully covers a
2025-06-01 sale of the same sat in the **same** wallet, filing a long-term `$0` 8949 row. Fixture is
`ProjectionConfig::default()`, no allocation. The claim holds *only* under Path B — and a vault can never hold both an in-force
allocation and a pre-2025 tranche, so that case is unreachable by construction.

Failure scenario: a filer with a 2026 shortfall whose coins were genuinely acquired in 2013 is told (by the spec, the published
doc, and any future UI copy derived from them) that a pre-2025 bucket cannot cover it, and is steered to `Y2025Onward` — a window
they know is untrue, yielding a short-term character and a floor two orders of magnitude above anything they could document, on
top of an attestation that is the §6664(c) footing.

**Fix (doc-only; the DECISION stands).** State the two sound justifications instead: (a) a filer whose coins really are 2025+ can
attest truthfully instead of nudging ~150 times, and (b) covering a post-2025 shortfall with a *pre-2025* declare forfeits
safe-harbor eligibility for good. Correct the sentence in all four places in one sweep.

**I-2 — the README states a safety guarantee the engine does not provide.** `README.md`, "The fork: file $0, or knowingly claim a
floor" (commit `86e2c60`): *"btctax uses the **lowest closing price in your declared window** … it will never manufacture a loss,
and **it will never exceed basis you can already document**."*

The first two clauses are true and verified (`window_reference` `conservative.rs:236-264` is the min daily close;
`filed_basis_for` scales it by sat; `clamped_leg_basis` `conservative_promote.rs:179-192` bounds the estimate at `net −
documented`, so the estimate can never create a loss). **The third clause is unbacked.** Nothing compares the computed floor to
any documented basis. `plan_promote` gates only on provenance, non-empty Part II, `Coverage::Full`, consent, and the ack.
Counter-example: a filer with a documented 2013 buy at ~$100/BTC declares a 2021-01-01..2024-12-31 tranche → the floor is the
window minimum (order $15,000/BTC), ~150× anything they can document, and nothing refuses it. The shipped
`Advisory::WouldDisplaceIfPromoted`/`NowDisplacing` exist *precisely because* a promote can displace documented basis.

Why it blocks: this is the primary public description of the tax-aggressive branch, shipping at v0.10.0. It tells the filer the
estimate is self-limiting by documented basis, which undercuts the premise of the three-attestation §6662 gate described two
sentences later. **Fix:** delete the clause; state what *is* guaranteed.

## Minor
- **M-1** — `live_declare_ids` uses the naive record-time void set (`conservative.rs:941-969`), but `resolve.rs:627-638`
  **defers** a void of a `DeclareTranche` that a live promote references — so a promoted tranche whose void was rejected is
  excluded from `live_declare_ids` while still in force. Mostly compensated by the promote-side diff. One-line fix:
  `!voided.contains(&e.id) || state.promoted_origins.contains(&e.id)`.
- **M-2** — the same false premise is encoded in a computation: `tranche_pool(t) = pool_key(t.window_end, &t.wallet)`
  (`defensive/mod.rs:152-154`) scopes `covering_shortfalls` and `still_short_pools` to the tranche's own pool key, so a pre-2025
  tranche that genuinely covers a 2025+ disposal yields `covered_sat == 0` — `OverCovered`/`FeeOnlyPromoteNoop` can never fire
  for it. Compensating: the confirm step's `excess_sat()` note and `WouldDisplaceIfPromoted`.
- **M-3 — README accuracy cluster:** (1) "the tax delta where it can be computed" — `journey_view` always passes `profile:
  None`, so `ComputedTax` is structurally unreachable on the dashboard. (2) "which years your **promotion** actually changed" —
  after the I-1 fold the set also covers years a `$0` **declare** changed; the prose reproduces the very branch asymmetry the
  code fix removed. (3) "writes one packet per year" — for the wizard's core audience (2018-2023) `x` writes **no** packet.
  (4) "the §6664(c) record matches the screen you agreed to" is stated as a guarantee while the no-scrolling follow-up means a
  trailing term can be recorded as shown without being rendered.
- **M-4** — folding r1 tax M-1 *activated* the dormant displacement-caveat hole: a **correctly-sized** cover (`t.sat ==
  covered_sat`, which is the flow's own prefill) gets neither `OverCovered` nor the displacement caveat — and the new `[assess]`
  line now prints a bare per-year gain-Δ for it. `clamped_saving_for` enumerates only years where the tranche's own legs were
  disposed in the *without* state, so a HIFO reorder pushing gain into an untouched year produces no offsetting line.
- **M-5** — r1 M-4 still open: the declare flow's `t` readout has no displacement caveat, though `declare_preview_saving`
  already builds both folds. Both M-4/M-5 are owned by **P-D/whole-branch** — the phase closing at this gate — so they are
  overdue rather than parked; fold now or explicitly re-own to post-merge so the ledger stays greppable.
- **M-6** — the era pick's short/long-term consequence is hidden exactly where it matters most: `render_declare_flow` renders the
  holding date and `({term} at the short op's date)` **only** in the `Some(Ok(cf))` arm. The bundled dataset starts
  **2010-07-17**, so `Y2009To2011` is *always* `Coverage::Partial` → the `Err` arm → no holding date, no term. That is the oldest
  bucket: the one a lost-records filer is most likely to pick and the one that makes a disposal long-term. The new prompt copy
  promises the pick "sets … whether the disposal it covers is SHORT- or LONG-term" — not kept on that path. Hoist the line out of
  the floor match.

## Nit
- **N-1** — `decision_changed_years` recomputes the identical `with_state` on every call; hoisting halves the projections.
- **N-2** — keys `7`–`9` are silently swallowed while the picker advertises `1-6`.
- **N-3** — `next_preset` is production-dead; its only callers are its own two KATs, pinning a cycle order no UI uses.
- **N-4** — `x` on a year with stored `ReturnInputs` dispatches a **full-return** export, so the now-larger year set can write a
  complete prior-year 1040 packet as a side effect of one keypress. Worth a line in the `x` status or the README.

## Merge recommendation
**Do not merge yet — two Important findings are open, both doc-only.** Neither changes a filed number nor requires a code change
beyond `make check` for the `era.rs` doc edit. (1) I-2 — one sentence in `README.md`; fold M-3(1)-(4) in the same pass. (2) I-1 —
one paragraph, four files; `era.rs` is the urgent one because it publishes to crates.io/docs.rs and public docs are the hardest
thing to retract. After that fold: re-review, then this branch is ready. I would also fold **M-1** and **M-6** rather than carry
them — both are cheap and both sit in new code. **M-2**, **M-4**, **M-5** and the Nits are legitimately post-merge, but
**M-4/M-5 must be explicitly re-owned** in `FOLLOWUPS.md`, since their owning phase closes at this gate.
