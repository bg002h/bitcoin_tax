# Conservative-filing — Phase-2 whole-branch ARCHITECTURE review — r1

**Reviewer:** independent Fable agent (architecture/correctness lens). **Date:** 2026-07-20.
**Scope:** the Phase-2 delta (Tasks 8–15) + T16 follow-up fixes on `feat/conservative-filing`
(`9a3f163..HEAD`, head `45c6882`), per SPEC.md v4 (D-1..D-10) and IMPLEMENTATION_PLAN.md. Phase 1
(Tasks 1–7, gate-green) re-examined only where Phase 2 composes with it. All citations verified
against current source at HEAD. `make check` re-run by the reviewer: **2155/2155 green**.
Two findings below were **verified empirically** with a scratchpad harness building against
`btctax-core` (no repo modification); probe transcripts in the Evidence appendix.

**Verdict: 1 Critical / 2 Important / 4 Minor / 5 Nit.** Not green.

---

## CRITICAL

### C-1. `tranche_report_advisory` panics on an out-of-range `--tax-year` whenever a tranche lot is held

`crates/btctax-core/src/conservative.rs:486-487`:

```rust
let as_of = time::Date::from_calendar_date(year, time::Month::December, 31)
    .expect("Dec 31 is always a valid date");
```

The expect message is false: `time::Date` (default features) spans years −9999..=9999, so
`from_calendar_date` returns `Err(ComponentRange)` for any year outside that range. The branch is
reached whenever `tranche_wallets` is non-empty — i.e. the vault holds **any** `EstimatedConservative`
lot with `remaining_sat > 0` — and `year` flows straight from the unvalidated CLI flag
(`cli.rs:63`, `tax_year: Option<i32>`; `report_tax_year` reaches the advisory even for a
profile-less/uncomputable year, `cmd/tax.rs:426-441`).

**Failure scenario (verified — Evidence B):** `btctax reconcile declare-tranche …` (any tranche, undisposed)
then `btctax report --tax-year 10000` (or the plain typo `20026`) →
`panicked at crates/btctax-core/src/conservative.rs:487: Dec 31 is always a valid date: ComponentRange`.
A raw-mode-free CLI panic, but a panic on plain user input nonetheless, and it contradicts the
codebase's own posture for exactly this construction: `tax/return_1040.rs:47` uses `is_ok_and(…)`,
`btctax-tui/src/whatif_panel.rs:86` uses `unwrap_or(today)`. The TUI is not exposed
(`selected_year` is dataset-bounded); the CLI is.

**Fix shape:** treat an unconstructible Dec-31 as "no advisory" (`.ok()?`-style early `None`/skip),
or clamp/validate `--tax-year` once at the CLI boundary.

---

## IMPORTANT

### I-1. Void-an-inert-allocation → declare-tranche (guard-ADMITTED) → permanent Hard `SafeHarborUnconservable` on the voided allocation — every tax year becomes NotComputable

The composition of three deliberate pieces produces a bricked vault on a fully supported flow:

1. **The engine never treats an allocation as `voided`.** Pass 1a routes a `VoidDecisionEvent`
   targeting a `SafeHarborAllocation` into `allocation_voids`, never into `voided`
   (`resolve.rs:477-483`), so step 3 re-evaluates a voided allocation on every rebuild
   (`resolve.rs:1260-1263` skips only `voided`). §7.4 semantics: a void of an *inert* allocation
   "applies" (`transition.rs` test (vii)) — but "applies" only means no `DecisionConflict`; the
   allocation keeps being conservation-checked.
2. **The D-8 backstop (Task 5) fires on that re-evaluation.** With a pre-2025 tranche in the residue,
   `has_tranche_residue` pushes the **Hard** `SafeHarborUnconservable` (`resolve.rs:1301-1317`) —
   on the **voided** allocation's id.
3. **The T16 record-time predicate correctly admits the tranche first.**
   `in_force_allocation_exists` (`cmd/tranche.rs:55-77`) counts a voided allocation as in force only
   while it is still engine-effective; a voided *inert* (e.g. timebarred) allocation is
   `non_voided=false ∧ effective=false` → not in force → the pre-2025 tranche records.

**Failure scenario (verified end-to-end — Evidence A):** pre-2025 buy → 2025 sell (time-bars) →
`safe-harbor-allocate` (inert, Advisory-only Timebar) → `reconcile void <alloc>` (allowed; inert
allocations are voidable, pinned by `tests/voidable.rs` / `transition.rs` (vii)) →
`declare-tranche` pre-2025 (guard ADMITS; probe shows `in_force=false`) → next projection emits
`[SafeHarborUnconservable] severity=Hard event=decision|1` → `compute_tax_year` returns
`TaxYearNotComputable` **for every year, permanently**. No product action clears it: the void
already applied and cannot be re-issued; the allocation cannot be removed; the only escape is
voiding the tranche — i.e. abandoning the feature. The blocker text ("v1 makes them mutually
exclusive") gaslights a user who *did* dissolve the allocation exactly as the D-8 hint family
("revisit the in-app safe-harbor allocation") directs.

Sharpening the defect: because the record-time guard refuses a pre-2025 tranche under **any
non-voided** allocation, **the voided-inert case is the only product-reachable configuration in which
the Task-5 backstop's tranche arm can ever fire** — and in that one reachable configuration, Hard is
the wrong severity. (For the hand-crafted/ordering hazard the backstop exists for, Hard is right;
those paths bypass the guards.) The backstop's own SPEC narrative promises the benign outcome:
"denied effectiveness (kept inert → **Path A** → the tag survives)" — Path A *does* govern in the
probe, but the projection-wide Hard gate makes that Path-A state unusable.

Pre-existing context (checked — Evidence C): a voided **totals-mismatch** allocation also keeps its
Hard `SafeHarborUnconservable` forever on main-line semantics, so "voided allocations still emit
blockers" is not new to this branch. What this branch adds is the first **guard-blessed** route into
that trap for a filer who made no error, plus the T16 fix (`45c6882`) analyzing the dangling-void
seam for the *effective* side only and missing the inert side's post-void consequence. No test
covers void-inert-then-declare (the declare_tranche_cli suite covers inert-non-voided (refused),
effective-voided-handcrafted (refused), and the engine-level non-voided backstop — never the
admitted path's projection outcome).

**Fix constraints (design decision needed, not prescribed here):** the deny-effectiveness `continue`
must stay (it IS the D-8 guarantee); the change is to the *loudness* for an allocation with an
on-file void that is not engine-effective. Note the coupling: `in_force_allocation_exists`'s
`effective` mirror keys on the *absence* of Timebar/Unconservable blockers on the allocation's id,
so suppressing blockers for voided allocations changes what the guard sees (a voided totals-mismatch
allocation with no emitted blocker would read as "effective" → in force → wrongly refuse further
tranches). Any fix must hold all four states (voided×{effective,inert}) against both consumers
(the step-3 gate and the record-time mirror) — and add the missing KAT:
void-inert-alloc → declare-tranche → **year computes** (Path A, tag intact, no Hard blocker).

### I-2. The TUI Tax tab now re-projects the entire ledger on every draw tick — unconditionally, even for vaults with no tranche

`btctax-tui/src/tabs/tax.rs:183-193` calls `tranche_report_advisory` inside `render_tax_content`,
which the run loop invokes on **every** iteration — `terminal.draw(…)` at `btctax-tui/src/lib.rs:651`
with a 100 ms poll, i.e. ~10 draws/second continuously while the Tax tab is visible (tui-edit's
viewer reuses the same `render`). Inside the assembler, `overpayment_nudge_lines`
(`conservative.rs:360-369`) evaluates

```rust
let baseline = match tax_total(events, &project(events, prices, config), …)
```

**before** any gate — the full `project()` (resolve + fold of the whole ledger) runs even when the
vault contains **zero** `DeclareTranche` events and even when there is no profile (the `None` return
happens after the projection has been built and discarded). With a tranche and a profile the cost is
`2 + T` full projections per tick: the baseline `project`, the `resolve` inside `in_force_methods`
(`project/mod.rs:179`), and one `resolve`+`fold`+`compute_tax_year` per tranche
(`overpayment_delta_one`, `conservative.rs:286-303`).

**Failure scenario:** any realistic ledger (thousands of events) → the Tax tab pegs a core and
render latency grows linearly with ledger size × (2+T), ~10×/second, for every user of the viewer —
a regression from the pre-branch tab, which performed zero projections per frame (the `Snapshot`
holds the folded state precisely so draw never folds). The CLI path is fine (once per invocation).

**Fix shape:** early-return `None` from `tranche_report_advisory`/`overpayment_nudge_lines` when
`events` contains no non-voided `DeclareTranche` (a cheap scan — restores the no-tranche vault to
zero-cost); and hoist/memoize the advisory into `Snapshot` construction (it is a pure function of
snapshot inputs), mirroring how every other number the tab shows is precomputed.

---

## MINOR

### M-1. The methodology disclosure's preamble unconditionally asserts "$0 … was used as filed", contradicting its own basis-as-filed enumeration in the TP8(c) corner

`conservative.rs:152-159`: the header text says "A conservative $0 basis (the IRS fallback for
unprovable basis) was used as filed". The per-unit lines (correctly, per tax r1 I-1) print
`leg.basis` **as filed**, which is `> $0` when the TP8(c) fee-sat carry lands on the tranche leg —
the exact corner pinned by `tp8c_fee_sat_basis_can_land_on_the_last_tranche_leg_corner_b`
(`kat_conservative.rs:1219`). In that year the mandatory filing artifact (`basis_methodology.txt`)
states "$0 was used as filed" in the preamble and "filed at $X basis" (X>0) in the enumeration.
Failure scenario: a specific-ID sale naming the tranche with an on-chain fee → self-contradictory
disclosure attached to the return. Fix: condition the preamble ("$0, plus documented fee-derived
basis where noted below") or derive it from the enumerated legs.

### M-2. `overpayment_delta`'s "Never negative" contract is false under basis-swap-induced reordering; the pub Σ sums signed terms

`conservative.rs:270` (doc: "Never negative: a higher basis lowers the realized gain, so
`baseline ≥ with`") and `:314-340`. Raising ONE lot's basis re-sorts HIFO consumption; the swap can
pull the (now high-basis, LT) tranche into an earlier year and push a low-basis **short-term**
documented lot into `year`, so `year`'s with-scenario tax exceeds baseline → a negative per-tranche
delta. The production nudge path is safe (`overpayment_nudge_lines` skips `delta <= 0`,
`conservative.rs:381`), but the pub `overpayment_delta` sums raw signed terms, so one negative term
silently understates (or negates) the Σ, and the doc misleads future callers. Today the pub fn has
no production caller (tests only). Fix: clamp per-term at `Usd::ZERO` (matching the nudge's skip) or
correct the doc to state the reordering caveat.

### M-3. `overpayment_delta_one` accepts any `Op::Acquire` id as "the tranche"

`conservative.rs:288-297`: the swap keys on `eff.id == *tranche_id && matches!(op, Op::Acquire)` —
nothing requires `EventId::Decision` or `EstimatedConservative`. A caller passing an **import**
Acquire's id gets a computed "delta" for a documented lot, while the doc claims a non-tranche id
returns `$0`. Contained today (production builds `refs` only from `DeclareTranche` events,
`conservative.rs:371-379`), but the pub-adjacent seam contradicts its contract. Fix: also require
`a.basis_source == BasisSource::EstimatedConservative` (one line, makes the doc true).

### M-4. CLI/TUI advisory drift in the pseudo-placeholder corner (the "can never drift" claim doesn't fully hold)

CLI `report_tax_year` resolves the year's profile via `resolve_and_screen`, which under pseudo mode
injects the all-$0 **placeholder** profile (`cmd/tax.rs:281-289`) — so the CLI's P6 nudge quantifies
"could save ~$X" against a synthetic profile (globally `[PSEUDO]`-bannered, but the nudge line
itself is unmarked). The TUI passes `snap.profiles.get(&year)` (`tabs/tax.rs:189`), which never
contains a placeholder year → no nudge. Same year, different advisory content, despite the shared
assembler ("the CLI and TUI can never drift", `conservative.rs:431`, `tabs/tax.rs:181-182`). The
drift is in the *inputs*, which the shared-assembler design doesn't unify. Fix: gate the P6 lines on
a non-placeholder provenance (thread it, or suppress the quantified nudge under
`Provenance::PseudoPlaceholder`), or add the placeholder to the TUI side — either way, pick one.

---

## NIT

- **N-1.** `declare_tranche` prints the phantom-wallet WARN (`cmd/tranche.rs:164-171`) *before* the
  D-8 guard (`:177`) can refuse — a refused declaration still emits "the $0 tranche lot is
  stranded…", implying something was recorded. Reorder guard-then-warn.
- **N-2.** Advisory/disclosure money formatting prints raw `Decimal` (`${basis}`, `${gain}`,
  `conservative.rs:31-41,139-146`) — "$40000"/"$0", unlike the report's `{:.2}` figures; a negative
  gain renders "$-30".
- **N-3.** `window_reference` iterates day-by-day over an unbounded user window
  (`conservative.rs:196-207`; `declare-tranche` accepts any `window_start`, e.g. year 1 → ~740k
  `usd_per_btc` probes per tranche per advisory assembly — compounding I-2's per-frame cost).
- **N-4.** The P4 warning copy ("the sale defaults to FIFO", `conservative.rs:236-244`) is
  retrospective phrasing on an already-completed, non-specific-ID disposal; the deliberate
  disposal-scoped over-breadth is documented (`conservative.rs:447-452`) but the copy asserts more
  than the engine did.
- **N-5.** `basis_methodology.txt` is exercised only through `write_form_csvs`
  (`tests/basis_methodology_export.rs`); the `write_csv_exports` command path (`render.rs:871`) is
  untested for the artifact (same helper, same year-gate — low risk).

---

## Scope-checklist walk (items examined and found sound)

- **P6 clone-fold-discard re-fold (arch M-4):** sound. `resolve → mutate the matched Eff's
  `Acquire.usd_cost` → fold → compute_tax_year` mirrors `project` exactly (fold applies
  `sort_canonical` itself); `compute_tax_year` ignores `events` (`tax/compute.rs:239` `let _ = events;`),
  so passing the unmodified event slice beside the modified state is not a coherence hazard.
  Baseline computed once and threaded into every `overpayment_delta_one` (`conservative.rs:360-379`);
  the pub Σ recomputes its own baseline — consistent. Voided/undisposed tranches degrade to `$0`
  via `swapped=false` / identical-fold. **Per-tranche (vs joint) summation is intentional**
  (plan Task-12 interface: "the PER-TRANCHE reference … must never quote one joint number") and
  pinned by `overpayment_delta_sums_per_tranche_with_each_tranches_own_reference`. No
  borrow/ownership issues (owned `Resolution`, moved into `fold`). Deterministic (NFR4): event-order
  iteration, `BTreeSet` wallets, no clock/randomness/HashMap in the new module (the injected `now`
  stays in the CLI layer).
- **build_op id-guard (P9/T15), `resolve.rs:405`:** closes both holes without touching the
  legitimate path. The pass-2 admit (`resolve.rs:1085-1115`) matches only
  `(EventId::Decision, DeclareTranche)` and honors `voided`; a `ClassifyRaw{as_: DeclareTranche}`
  arrives at `build_op` under an Import id → guard fails → `Op::Skip`
  (KAT `classify_raw_declaretranche_on_an_import_folds_nothing_id_guard`); a hand-crafted
  `sat <= 0` Decision tranche is admitted as an inert `Op::Skip` Eff — folds nothing, Σ-conserves
  (KAT `sat_le_zero_decision_tranche_folds_nothing_and_conserves`). Silent-skip (no blocker) matches
  the engine's posture for every other malformed hand-crafted payload routed through the
  `_ => Op::Skip` catch-all — accepted.
- **`tranche_report_advisory` signature (+profile +tables):** both call sites updated and
  content-consistent — CLI `cmd/tax.rs:432-441` (rendered unconditionally at `main.rs:190-193`,
  non-gating, exit-code-neutral) and TUI `tabs/tax.rs:183-193`; the shared assembler holds the
  surfaces identical except for the M-4 input-drift corner and the I-2 cost profile. The P6 lines
  are correctly profile-gated (`tax_total → None → return` for a `None` profile;
  KAT `overpayment_nudge_absent_without_a_profile`).
- **basis_methodology export:** written by BOTH `write_form_csvs` (`render.rs:911`) and
  `write_csv_exports` (`render.rs:871`) via the one 0o600 helper (`render.rs:920-932`); listed by
  the TUI `compute_files` on the same predicate (`export.rs:110-119`, with a unit KAT); the
  "four form CSVs" guarantee comments swept across `app.rs`/`lib.rs`/`unlock.rs`/`export.rs`
  (one residual phrase is the accurate "the four named form CSVs plus …"). KAT-E10 is unaffected:
  the write lives in `btctax-cli`; no new write-class I/O enters `btctax-tui` source.
- **`in_force_allocation_exists(events, blockers)`:** the effective-mirror
  (`!has(Timebar) && !has(Unconservable)`, `cmd/tranche.rs:66-73`) matches `void.rs:79`
  (`effective_alloc`) exactly; the `non_voided || effective` disjunction correctly blocks
  {non-voided × any} and {voided × effective (dangling hand-crafted void)} — tested including the
  hand-crafted case. The voided×inert cell is where I-1 lives (admission is CORRECT; the engine's
  reaction is the defect). `declare_tranche`'s added pre-append projection is once-per-invocation
  (fine) and correctly projects the PRE-tranche event set. `safe_harbor_residue`'s refusal is
  handled at every caller: CLI allocate refuses earlier via `guard_allocation_vs_tranche`
  (`reconcile.rs:956` before `:983`), the TUI opener surfaces the `Err` as pre-flight status
  (`tui-edit/main.rs:6354-6360`), tests updated. All four allocation append sites route through the
  one guard (`reconcile.rs:956,1199`; `persist.rs:1032,1105`).
- **`window_reference`:** inclusive `[start, end]` iteration, `start > end` → `None`,
  `Date::MAX` handled (`next_day() == None` breaks after the final day is processed — no infinite
  loop), `covered == total` ⇒ Full else Partial, no-overlap ⇒ `None`; no off-by-one found. Cost
  note at N-3.
- **Exhaustiveness/dead code/unwraps:** no missed match arm (the `Term` match is total; the new
  `build_op` arm's guard-failure deliberately falls to `Op::Skip`, KAT-pinned); no dead code
  (`Coverage`/`WindowRef`/all pub fns consumed); the only panicking construct in the new code is
  C-1's `.expect`.
- **T16 items (a)/(b)/(c):** (a) the dangling-void defense-in-depth is correct for the effective
  side and mutation-pinned; the inert side is I-1. (b) the residue refusal is correctly scoped to
  non-voided pre-2025 tranches and test-pinned. (c) doc split verified in SPEC §D-8 / plan Task 6.

## Evidence appendix (scratchpad probes, built against `btctax-core` at HEAD; repo untouched)

**A — I-1 end-to-end** (pre-2025 buy; 2025 sell; inert ActualPosition alloc; void of the alloc;
pre-2025 tranche):

```
PRE-TRANCHE blockers (guard's view):
  [SafeHarborTimebar] severity=Advisory event=Some("decision|1")
guard view: non_voided=false, engine_effective=false -> in_force=false (tranche ADMITTED)

blockers:
  [SafeHarborUnconservable] severity=Hard event=Some("decision|1")
    a conservative-filing tranche ($0 EstimatedConservative) remains in the pre-2025 residue — …
2025: NOT COMPUTABLE [TaxYearNotComputable] year 2025 not computable: unresolved Hard blocker
  [SafeHarborUnconservable] decision|1 :: …
```

**B — C-1 panic** (vault with one undisposed tranche; `tranche_report_advisory(year=10000)`):

```
thread 'main' panicked at crates/btctax-core/src/conservative.rs:487:14:
Dec 31 is always a valid date: ComponentRange { name: "year", is_conditional: false }
```

**C — pre-existing sibling** (voided totals-mismatch allocation, NO tranche):

```
blockers after voiding a totals-mismatch allocation (NO tranche):
  [SafeHarborUnconservable] severity=Hard event=Some("decision|1")
```

(Pre-branch semantics; recorded for scoping — the branch's new contribution is the guard-blessed
route in Evidence A.)
