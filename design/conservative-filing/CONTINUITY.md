# Conservative-Filing — Build Continuity (resume point)

**Updated 2026-07-21: PHASE 1 (Tasks 1–7) COMPLETE + GATE GREEN. Everything below is committed; the tree is clean. Resume at Phase 2 / Task 8.**

## Where we are

- **Branch:** `feat/conservative-filing` (rebased onto `main`, carries the shipped 8949-box fix — D-6's prerequisite). ~24 commits ahead of main.
- **SPEC + PLAN are GREEN** (both tax + architecture lenses, 0C/0I): `design/conservative-filing/SPEC.md`, `design/conservative-filing/IMPLEMENTATION_PLAN.md`. Review rounds persisted in `design/conservative-filing/reviews/`.
- **PHASE 1 (P1 core, Tasks 1–7) COMPLETE + gate-green**, inline TDD, each task green + mutation-proven; full workspace suite **2117 green**, clippy/fmt clean. **Phase-1 gate CLOSED:** all CI-only jobs green (nextest 2117 / clippy 0 / fmt / check-isolation / pii-scan / msrv 1.88 build) **AND** the independent two-lens Fable review is 0C/0I: r1 (tax 0C/0I, arch 0C/**1I**) → folded → r2 re-review (tax 0C/0I, arch 0C/0I). All four reviews persisted in `reviews/phase1-impl-{tax,architecture}-fable-review-r{1,2}.md`. Open follow-ups (all Minor/Nit, non-blocking) in `FOLLOWUPS.md` with owning phases.

  | T | Commit | What |
  |---|---|---|
  | 1 | `9a67046` | Schema: `BasisSource::EstimatedConservative` + `EventPayload::DeclareTranche{sat,wallet,window_start,window_end}` (event.rs) + the 6-site exhaustive sweep (forms `how_acquired_from`→Review [now `pub`], render `basis_source_tag`, tui-edit `cycle_basis_source`[off-ring]+`basis_source_display`, tui tags `basis_source_rank`[9]+`basis_source_tag`) + `void.rs is_revocable_payload += DeclareTranche` + `main.rs bulk_void_payload_summary` arm + `is_imported` doc. |
  | 2 | `1c535f2` | The core fold. `project/resolve.rs`: timeline-builder admit for a `DeclareTranche` (guard `(EventId::Decision, &e.payload)` — NOT `applied`; honor `voided`; `Eff.utc = t.window_end.midnight().assume_utc()`; `src_priority=u8::MAX`; **constant** `src_ref=""`); `build_op` arm → `Op::Acquire{usd_cost:0,fee_usd:0,basis_source:EstimatedConservative}` (reuses the Acquire fold arm → acquired_at=window_end, $0, pool_key, sigma_in); `sort_canonical` final `.then(a.id.cmp(&b.id))` numeric tie-break. |
  | 3 | `9ee2156` | Guard KATs (test-only): no-Skip, voided-folds-nothing, product-voidable (`voidable_decisions`), canonical seq-order **asserted on `resolve()`+`sort_canonical`** (resolve returns UNSORTED), additivity. |
  | 4 | `0f65429` | D-8 tag survives BOTH `basis_source` overwrite sites: `transition.rs` Path-A seed + `fold.rs` relocation (each exempts `EstimatedConservative`). KATs: Path-A survival → 2025 LT leg → Part II/Box L + box_needs_review; ST → Box I; boundary iff-1yr; pre-2025 → Box F; relocation survival. |
  | 5 | `033923d` | D-8 projection-time backstop: `UniversalSnapshot += estimated_conservative_remaining_sat` (transition.rs); `resolve.rs` effectiveness check denies a `SafeHarborAllocation` (→ `SafeHarborUnconservable`, inert, Path A) over a live tranche residue, independent of declaration order. |
  | 6 | `a541452` | D-8 record-time mutual-exclusion. New `cmd/tranche.rs`: pure log predicates (`in_force_allocation_exists` = non-voided alloc, effective OR inert; `pre2025_tranche_exists` = window_end<2025 non-voided) + guard chokepoints wired at ALL FOUR allocation append sites (CLI allocate/attest + TUI allocate/attest). Guard re-exported at btctax_cli root (`guard_allocation_vs_tranche`) so the TUI avoids `cmd::` (KAT-G1). `session.rs safe_harbor_residue` excludes DeclareTranche. |
  | 7 | `be5580a` | `reconcile declare-tranche --amount --wallet --window-start --window-end` (--amount = sat int OR BTC decimal). `declare_tranche` guards: sat>0, window_start≤window_end (refuse); future window_end warns. Regenerated per-subcommand man page. Clean non-pseudo export (D-5). |
  | fold | `69fec06`, `4d86df8` | Phase-1 review r1→r2 folds: TUI `summarize_void_payload` DeclareTranche arm (the 1 Important) + attest-guard tests + inert-then-declare backstop KAT + ≥2025 non-poisoning (Path-B) pins + refusal-hint split + Nits. |

  **Test harness:** `crates/btctax-core/tests/kat_tranche.rs` (16 KATs) + `crates/btctax-cli/tests/declare_tranche_cli.rs` (12 CLI tests) + TUI KATs in `btctax-tui-edit/src/edit/persist.rs` + `.../src/main.rs`. Core fixtures: `exch()`, `tranche_ev`, `void_ev`, `alloc_ev`, `alloc_lot`, `prices()`, `cfg()`. Mirror for new KATs.

## Resume here — Phase 2 (PLAN Tasks 8–15) + T16 whole-branch review

**Phase 1 is DONE + gate-green (above). Reconcile FOLLOWUPS.md at each phase entry** (do the items that phase owns — e.g. P8/T14 owns the `--wallet` + UTC-warning Nits; P9/T15 owns the build_op id-guard hardening + the 8949-date/Σ-conservation test-pins; T16 owns the in_force dangling-void + residue-skew + doc-consistency items).

- **T8 / P2** (test-only, characterization — passes on write): pin HIFO draws-documented-first + the FIFO inversion. Stage in the **pre-2025 Universal pool** (config method governs pre-2025; post-2025 method = MethodElection, defaults HIFO). New `crates/btctax-core/tests/kat_conservative.rs`. If a test FAILS on write, the emergence assumption is wrong — STOP, don't add matching code.
- **T9 / P3**: `tranche_dip_advisory` + `method_inversion_advisory`. Surface via NEW `Option<String>` field(s) on `TaxYearReport` (`cmd/tax.rs:~238`, follow the `gift_advisory` precedent — there is NO "advisory Vec"), rendered in `render_tax_outcome`, mirrored into the TUI Tax tab. A surfacing KAT (advisory reaches stdout) is REQUIRED.
- **T10 / P4**: custody warning — reuse `optimize.rs ForbiddenBroker2027`.
- **T11 / P5**: `window_reference -> Option<WindowRef{min,coverage}>`.
- **T12 / P6**: per-tranche overpayment delta via a basis-replacement what-if (needs events+prices+config, NOT a folded state).
- **T13 / P7**: `basis_methodology.txt` — provenance-neutral, term-correct, basis AS FILED.
- **T14 / P8**: self-custody nudge. Owns FOLLOWUPS: `--wallet`-not-known warn + future-`window_end` filer-zone.
- **T15 / P9**: no-loss-from-the-estimate invariant + 2 fee corners (corner-(b) staged as a **specific-ID** sale, not HIFO). Owns FOLLOWUPS: build_op `EventId::Decision` id-guard + engine `sat≤0` blocker; 8949 `date_acquired==window_end` pin; Σ-conservation-with-a-tranche pin.
- **T16**: whole-branch review (both lenses) → 0C/0I. Owns FOLLOWUPS: `in_force` dangling-void divergence; `safe_harbor_residue` disposal-present display skew; SPEC/PLAN split-hedge doc-consistency.

## Standing rules (do not violate)
- **Mutation discipline:** a task isn't done until the mutation dies. `cp` file to scratchpad → `sed`/`perl` the mutation → run test → confirm RED → `cp` back (NEVER `git checkout --` a tracked file mid-work).
- **Fast validation:** `cargo nextest run --workspace` + `cargo clippy --workspace --all-targets` + `cargo fmt --check` per task; the CI-only jobs at the Phase gate.
- **Merge to main is the OWNER'S CALL** — never merge without explicit direction. Present green + merge-ready.
- Preserve: $0-only filing (D-7), never understate (G-4, term derived), provenance-neutral copy, and the policies in memory (`self-transfer-*`, `full-return-draft-gate-policy`).
- Reviews are two-lens Fable (`model:"fable"`, `subagent_type:"general-purpose"`), run in parallel; persist each verbatim before folding; re-review after every fold including the last.

## One-line resume
`feat/conservative-filing` @ `4d86df8`; PLAN green; **Phase 1 (Tasks 1–7) DONE + gate-green** (2117 tests; two-lens Fable review 0C/0I r1→r2); **start at Task 8 / Phase 2** (HIFO-steering characterization pins in a new `kat_conservative.rs`) per `IMPLEMENTATION_PLAN.md`. Reconcile `FOLLOWUPS.md` at each phase entry. Merge to main is the OWNER'S call.
