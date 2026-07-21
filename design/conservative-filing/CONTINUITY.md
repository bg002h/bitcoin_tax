# Conservative-Filing — Build Continuity (resume point)

**Written 2026-07-20 at a clean context-clear boundary. Everything below is committed; the tree is clean.**

## Where we are

- **Branch:** `feat/conservative-filing` (rebased onto `main`, so it carries the shipped 8949-box fix — D-6's prerequisite). 17 commits ahead of main.
- **SPEC + PLAN are GREEN** (both tax + architecture lenses, 0C/0I): `design/conservative-filing/SPEC.md`, `design/conservative-filing/IMPLEMENTATION_PLAN.md`. All review rounds persisted in `design/conservative-filing/reviews/` (`spec-*`, `plan-tax-*`, `plan-architecture-*`).
- **Build in progress, inline TDD** (user chose this mode). Phase-1 **engine core is DONE** — Tasks 1–5, each green + mutation-proven, full workspace suite **2101 green**, clippy/fmt clean:

  | T | Commit | What |
  |---|---|---|
  | 1 | `9a67046` | Schema: `BasisSource::EstimatedConservative` + `EventPayload::DeclareTranche{sat,wallet,window_start,window_end}` (event.rs) + the 6-site exhaustive sweep (forms `how_acquired_from`→Review [now `pub`], render `basis_source_tag`, tui-edit `cycle_basis_source`[off-ring]+`basis_source_display`, tui tags `basis_source_rank`[9]+`basis_source_tag`) + `void.rs is_revocable_payload += DeclareTranche` + `main.rs bulk_void_payload_summary` arm + `is_imported` doc. |
  | 2 | `1c535f2` | The core fold. `project/resolve.rs`: timeline-builder admit for a `DeclareTranche` (guard `(EventId::Decision, &e.payload)` — NOT `applied`; honor `voided`; `Eff.utc = t.window_end.midnight().assume_utc()`; `src_priority=u8::MAX`; **constant** `src_ref=""`); `build_op` arm → `Op::Acquire{usd_cost:0,fee_usd:0,basis_source:EstimatedConservative}` (reuses the Acquire fold arm → acquired_at=window_end, $0, pool_key, sigma_in); `sort_canonical` final `.then(a.id.cmp(&b.id))` numeric tie-break. |
  | 3 | `9ee2156` | Guard KATs (test-only): no-Skip, voided-folds-nothing, product-voidable (`voidable_decisions`), canonical seq-order **asserted on `resolve()`+`sort_canonical`** (resolve returns UNSORTED), additivity. |
  | 4 | `0f65429` | D-8 tag survives BOTH `basis_source` overwrite sites: `transition.rs` Path-A seed + `fold.rs` relocation (each exempts `EstimatedConservative`). KATs: Path-A survival → 2025 LT leg → Part II/Box L + box_needs_review; ST → Box I; boundary iff-1yr; pre-2025 → Box F; relocation survival. |
  | 5 | `033923d` | D-8 projection-time backstop: `UniversalSnapshot += estimated_conservative_remaining_sat` (transition.rs); `resolve.rs` effectiveness check denies a `SafeHarborAllocation` (→ `SafeHarborUnconservable`, inert, Path A) over a live tranche residue, independent of declaration order. |

  **Test harness:** `crates/btctax-core/tests/kat_tranche.rs` (13 KATs + fixtures: `exch()`, `cold()`, `dec_ev`, `tranche_ev`, `void_ev`, `imp`, `sell_ev`, `self_transfer`, `alloc_ev`, `alloc_lot`, `prices()`=StaticPrices::default, `cfg()`=ProjectionConfig::default). Mirror it for new KATs.

## Resume here — remaining work (follow the PLAN task-by-task)

### Task 6 — record-time mutual-exclusion refusal (CLI/TUI UX layer)
The engine backstop (T5) is the guarantee; T6 is the friendly early error. See PLAN §Task 6.
- **Tranche side:** refuse recording a `DeclareTranche` with `window_end < TRANSITION_DATE` when ANY in-force (non-voided) `SafeHarborAllocation` exists (effective OR inert). A `window_end ≥ 2025` tranche records CLEANLY even beside an effective allocation (do NOT foreclose P7).
- **Allocation side:** refuse recording a `SafeHarborAllocation` when a pre-2025 tranche exists.
- **All FOUR append sites** (CLI + TUI): `crates/btctax-cli/src/cmd/reconcile.rs:984` (allocate) + `:1273` (attest); `crates/btctax-tui-edit/src/edit/persist.rs:1031` (allocate) + `:1114` (attest). Consider one `session`/`persist` chokepoint each.
- **Also:** `crates/btctax-cli/src/session.rs` `safe_harbor_residue` (~`:681-700`) — EXCLUDE `DeclareTranche` decisions from the allocatable pre-2025 residue (else the allocate opener self-poisons).
- **Do NOT edit** `crates/btctax-core/src/void.rs` `effective_alloc`/`voidable_decisions` semantics (inert must stay voidable). The "in-force" predicate is a NEW record-time check at the guard sites (payload is `SafeHarborAllocation` ∧ id not in `voided`).
- New `crates/btctax-cli/src/cmd/tranche.rs` (record path validates before appending) + register in `cmd/mod.rs`.
- **Tests** (`crates/btctax-cli/tests/declare_tranche_cli.rs` + a TUI-persist refusal KAT): (a) pre-2025 tranche refused under an EFFECTIVE alloc; (a2) refused under an INERT alloc (needed so the effective-only mutation can RED); (b) same via TUI persist; (c) alloc refused under a pre-2025 tranche; (d) a ≥2025 tranche records CLEANLY beside an effective alloc; (e) refusal appends NO event; (f) `safe_harbor_residue` omits tranche sats. Mutations per the PLAN.

### Task 7 — `declare-tranche` CLI verb + clean export
- `crates/btctax-cli/src/cli.rs` subcommand (`--sat`/`--btc`, `--wallet`, `--window-start`, `--window-end`), `src/main.rs` dispatch, `src/cmd/tranche.rs` handler. Mirror an existing decision-appending verb.
- **Validation (record-time refuse):** `sat > 0` (a `sat ≤ 0` corrupts sigma_in), `window_start ≤ window_end`; warn (not refuse) on a future `window_end`.
- **KATs:** verb appends a `$0` tranche; a filed-tranche year exports CLEAN (`pseudo_active()` stays false, `!report.watermarked`, no `AttestationRequired`). Mutation: remove `sat>0` guard → refusal test RED.

### Then the Phase-1 gate
`make check` + `cargo fmt --check` + `cargo run -p xtask -- check-isolation` + `bash scripts/pii-scan-generic.sh` + `cargo +1.88 build --workspace` — all green. Then dispatch an **independent Fable review (tax + architecture lenses)** of Phase 1 to 0C/0I (persist verbatim; fold; re-review) BEFORE starting Phase 2.

### Then P2–P8 (PLAN Tasks 8–15) + T16 whole-branch review
T8 P2 (HIFO steering + FIFO inversion, characterization), T9 P3 (dip + method-inversion advisory — surface via a NEW `Option<String>` on `TaxYearReport`, not a nonexistent "advisory Vec"), T10 P4 (custody warning — reuse `optimize.rs` `ForbiddenBroker2027`), T11 P5 (`window_reference -> Option<WindowRef{min,coverage}>`), T12 P6 (per-tranche overpayment delta via a basis-replacement what-if — needs events+prices+config, NOT a folded state), T13 P7 (`basis_methodology.txt`, provenance-neutral, term-correct, basis AS FILED), T14 P8 (self-custody nudge), T15 no-loss-from-the-estimate invariant + 2 fee corners (corner-(b) staged as a **specific-ID** sale, not HIFO). T16: whole-branch review both lenses → 0C/0I.

## Standing rules (do not violate)
- **Mutation discipline:** a task isn't done until the mutation dies. `cp` file to scratchpad → `sed`/`perl` the mutation → run test → confirm RED → `cp` back (NEVER `git checkout --` a tracked file mid-work).
- **Fast validation:** `cargo nextest run --workspace` + `cargo clippy --workspace --all-targets` + `cargo fmt --check` per task; the CI-only jobs at the Phase gate.
- **Merge to main is the OWNER'S CALL** — never merge without explicit direction. Present green + merge-ready.
- Preserve: $0-only filing (D-7), never understate (G-4, term derived), provenance-neutral copy, and the policies in memory (`self-transfer-*`, `full-return-draft-gate-policy`).
- Reviews are two-lens Fable (`model:"fable"`, `subagent_type:"general-purpose"`), run in parallel; persist each verbatim before folding; re-review after every fold including the last.

## One-line resume
`feat/conservative-filing` @ `033923d`; PLAN green; Tasks 1–5 done (engine core, 2101 green); **start at Task 6** (record-time refusal, 4 append sites) per `IMPLEMENTATION_PLAN.md`.
