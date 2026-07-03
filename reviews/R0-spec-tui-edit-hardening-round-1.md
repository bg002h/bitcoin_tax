# R0 spec review — `SPEC_tui_edit_hardening.md` (round 1)

**Artifact:** `design/SPEC_tui_edit_hardening.md`
**Baseline verified against:** current source (branch `feat/tui-edit-hardening`, working tree at review time).
**Reviewer:** independent adversarial architect (did NOT author the spec).
**Bar:** 0 Critical / 0 Important.

## Verdict: **0 Critical / 1 Important / 2 Minor / 2 Nit** — BLOCKED

One Important finding gates implementation: the spec's stated "Verified core fact" underpinning **#2** (the acquisition-date gate) is factually false — it ignores the §7.4 boundary drain (`transition::seed_transition`), and the accompanying "over-inclusion is engine-safe (method-order fallback)" claim is also false (an infeasible pick becomes a *hard* `LotSelectionInvalid` blocker). Together they let #2 offer doomed lots for pre-2025 disposals under Path B. Everything else verified sound: #8's 6-arm/4-fn count and every anchor, #6's cap plumbing and free-text/structured split, #7's effective-condition + KAT-rewrite (void list genuinely empty after attest) + retained engine coverage, #1's link-build replica fidelity + blast radius, and #3's shortfall/UncoveredDisposal iff logic.

---

### [I-1] IMPORTANT — #2's "a lot is in Universal iff `acquired_at < TRANSITION_DATE`" is false (ignores the §7.4 boundary drain); the gate offers doomed lots, and "engine-safe fallback" is not the actual behavior

**Where:** spec lines 34-37 ("Verified core facts") and lines 199-207 (D-#2), against `crates/btctax-core/src/project/transition.rs:75-103` (`seed_transition`), `crates/btctax-core/src/project/fold.rs:59-67` (`consume_principal`), `crates/btctax-tui-edit/src/main.rs:2718-2732` (candidate source = final `snap.state.lots`).

**The defect.** The spec's design basis asserts (line 36-37): *"origin lots are placed with `pool_key(acquired_at, wallet)` … so a lot is in `Universal` iff `acquired_at < TRANSITION_DATE`."* The premise is true (`fold.rs:566,695,877,955` confirmed) but the inference is invalid: it omits the one-shot §7.4 boundary seed. `seed_transition` (`transition.rs:75-103`), fired when the fold crosses the first `≥ 2025-01-01` event, **removes the `Universal` pool entirely** and redistributes pre-2025 lots into per-wallet pools:

- **Path A** (`transition.rs:82-86`): each remaining Universal lot is `push_lot`'d to `Wallet(lot.wallet)` — with its **lot_id preserved** (only `basis_source → ReconstructedPerWallet`).
- **Path B** (`transition.rs:88-100`): the Universal residue is **discarded** and replaced by allocation **seed lots** with **new** lot_ids `{allocation_id, seq}` (`basis_source == SafeHarborAllocated`), carrying pre-2025 `acquired_at`, in per-wallet pools.

So once any `≥2025` event has fired the seed, the final `snap.state.lots` (which is what the candidate filter at `main.rs:2718-2722` reads) contains **no `Universal` pool at all**; every pre-2025 lot sits in a `Wallet` pool. The stated invariant is not true.

**Why it gates (the real over-inclusion is a HARD blocker, not a fallback).** The gate `l.acquired_at < TRANSITION_DATE` offers every pre-2025 lot in final state. A pre-2025 disposal, however, is folded and consumes from `Universal` *before* the boundary seed. `selection_feasible` (`pools.rs:107-153`) is evaluated at that fold position, so the pick must exist in `Universal` **at the disposal's fold time**:

- **Path A** — offered pre-2025 lots keep their original lot_ids, which *do* exist in `Universal` at that time → feasible. The gate works here (the intended cross-wallet case). It works despite the false invariant, purely because Path A preserves lot_ids.
- **Path B** (an *effective* `SafeHarborAllocation` — a feature this project ships: safe-harbor-allocate + the attest flow just landed) — the only pre-2025 lots in final state are **seed lots** whose lot_ids `{allocation_id, seq}` **never existed in `Universal`** (they materialize only at the boundary). Picking any of them for a pre-2025 disposal → `selection_feasible` → `Err` → `consume_principal` raises a **hard `LotSelectionInvalid` blocker** (`fold.rs:63-65`) that gates `compute_tax_year`. Under Path B every offered pre-2025 lot is doomed.
- A second, path-independent manifestation: a **pre-2025 self-transfer** relocates fragments with **new** lot_ids (`bump_split`, `fold.rs:768`); offering such a fragment for an *earlier* pre-2025 disposal is likewise infeasible-at-fold-time → hard `LotSelectionInvalid`.

This is compounded by the spec's second false claim (lines 206-207): *"Over-inclusion is engine-safe (method-order fallback)."* It is **not**. `consume_principal` (`fold.rs:63-65`) turns any `selection_error` into a hard `LotSelectionInvalid` blocker; the method-order consumption is only how Σsat stays conserved — the blocker still fires and gates tax. There is no silent safety net. (This exact doomed-selection class is what #1/#3 exist to eliminate, so shipping it via #2 is self-defeating.)

Note the pre-existing `l.wallet == w` filter already offers same-wallet Path-B seed lots (this bug predates #2); #2 *widens* the surface to cross-wallet seed lots and the spec's "Verified core fact" wrongly certifies the offer as always-Universal/feasible. The spec's KATs (`KAT-PRE2025-CROSSWALLET-LOTS` etc.) only exercise Path A, so none would catch this.

**Concrete fix (pick one, and correct both false claims):**
1. Make the candidate filter *feasibility-honest* rather than an `acquired_at`/`wallet` heuristic. For a pre-2025 disposal, only lots whose lot_id would reside in `Universal` at that disposal's fold position are selectable. Under Path B those are the discarded originals (not visible), so the correct behavior is to **offer nothing cross-pool** — and arguably to exclude pre-2025 disposals from select-lots entirely when an effective allocation governs (Path B has already superseded their Specific-ID). At minimum, exclude `basis_source == SafeHarborAllocated` (seed) lots from the pre-2025 gate; **keep** `ReconstructedPerWallet` (Path A) lots, which are feasible.
2. Correct the "Verified core fact" to the true invariant: pre-2025 lots are drained per-wallet at the 2025-01-01 boundary (`seed_transition`); feasibility for a pre-2025 disposal depends on lot_id survival, which holds under Path A but not Path B (nor for pre-2025 self-transfer fragments vs. an earlier disposal).
3. Correct "engine-safe (method-order fallback)" → "an infeasible pick is a *surfaced* hard `LotSelectionInvalid` blocker (`fold.rs:63-65`)."
4. Add a Path-B KAT (effective allocation + a pre-2025 `Sell`): assert the pre-2025 disposal's select-lots offers no doomed lot (or does not offer the seed lots).

---

### [M-1] MINOR — engine-coverage citation points at the wrong file

**Where:** spec lines 46-47 (also the #7 KAT-rewrite justification, lines 143-144).

The spec cites `project/transition.rs:365 void_of_effective_allocation_is_a_decision_conflict` and `:403 void_of_inert_allocation_applies_no_conflict` as the engine coverage retained after the `KAT-E2E-ATTEST-VOID` rewrite. Those tests do exist and pin **both** directions (verified) — but they live in the **integration-test file** `crates/btctax-core/tests/transition.rs:365,:403`, not the source module `crates/btctax-core/src/project/transition.rs` (which is only 103 lines). The claim ("engine coverage NOT lost") is true; only the path is wrong. Since this citation is the load-bearing argument that the KAT rewrite is safe, a verifier following it lands in the wrong file. **Fix:** cite `crates/btctax-core/tests/transition.rs:365,:403`.

---

### [M-2] MINOR — #1 detection: seq-order requirement not mechanized, and a panic hazard in the pseudo-code

**Where:** spec lines 168-179 (D-#1 detection loop), against `resolve.rs:349-356,486-527` and `main.rs:3257` (`events_by_id`).

Two faithful-replication gaps that an implementer could get wrong:

1. **Order.** The engine builds `links`/`consumed_ins` over `decisions` sorted ascending by `decision_seq` (`resolve.rs:349-356`). FIRST-WINS on a duplicate *in_event* (M-3) determines *which* out-event projects to `SelfTransfer`; if the TUI replica iterates in a different order it can list the losing out-event, which the engine treats as non-honoring (`PendingOut`) → a selection on it fires `LotSelectionInvalid`. The spec says "in seq order" but does not say how to obtain it from `snap.events`, a `Vec<LedgerEvent>` with no spec-stated ordering guarantee. (In practice it is `load_all_ordered` output — ordinal order, which equals decision_seq order for decisions — so the risk is low, but the spec should require sorting the TransferLink decisions by `decision_seq`, mirroring `resolve.rs:349-356`, rather than leaving it implicit.)
2. **Panic hazard.** The pseudo-code `ev_idx[in_id].wallet.is_none()` (line 176) index-panics if `in_id` is absent from the ledger. The engine treats a missing in-event as "no resolvable wallet → skip/conflict" (`resolve.rs:509`). The implementation must use a fallible lookup (`ev_idx.get(in_id).map_or(true, |e| e.wallet.is_none())`) so a dangling link is skipped, not a crash.

Both are cheap to state; leaving them implicit is a Minor.

---

### [N-1] NIT — `FREETEXT_CAP = 512` is not literal CLI parity

The CLI donation free-text fields are unbounded `Option<String>` (spec cites `cli/src/main.rs:318,336`). 512 is a reasonable render-safe cap, but a `>512`-char value (e.g. a long `appraiser_qualifications`) still truncates in the TUI where the CLI would not, so "CLI-parity length" (lines 17, 91) overstates it. Recommend wording it as "a much larger, render-safe cap (512) vs. the structured fields' 64," and confirming 512 comfortably covers realistic appraiser-qualification text.

### [N-2] NIT — #8 site-#2 anchor and the "add-comma" instruction are slightly imprecise

The fold-target token `or CLI:` for site #2 (FmvMissing arm) is on `main.rs:2078`; the table anchors it at `:2079`, which is the continuation line carrying `btctax reconcile void decision|{seq})`. Also, the instruction to "add the comma after `(press 'v')`" (line 73) applies only to the **four** DecisionConflict arms (`:2061,:2133,:2345,:2374`), whose text is `(press 'v') or CLI:`; the FmvMissing/UnknownBasisInbound arms (`:2079,:2097`) read `(Void flow: press 'v'; or CLI: …)` and take no comma. The per-arm KAT coverage (RS-1..4 enriched + new RS-5/6, all asserting `contains("quit the editor")`) catches any missed arm, so this is cosmetic — but note two of the six sites have the `or CLI: … btctax reconcile void` phrase split across source lines by `\` continuations, so a naive single-line find/replace would miss them; fold each of the six arms individually.

---

## What I verified sound (for the record)

- **#8 count is exactly 6 arms / 4 fns.** All anchors correct: `derive_classify_inbound_status` DecisionConflict `:2061`, FmvMissing `:2079`, UnknownBasisInbound `:2097`; `derive_reclassify_outflow_status` DecisionConflict `:2133`; `derive_reclassify_income_status` DecisionConflict `:2345`; `derive_set_fmv_status` DecisionConflict `:2374`. No 7th production site (only other `btctax reconcile void` hits are the canonical `derive_select_lots_status` at `:3433-3434`, which already says "quit the editor and run", and the test asserts). RS-1..4 (`:7860,:7883,:7907,:7933`) currently assert only `contains("'v'")` + `contains("btctax reconcile void")` — both survive the fold; enrichment is additive. RS-5/6 targets `derive_reclassify_income_status`/`derive_set_fmv_status` genuinely have **no** unit callers today (only production call sites `:1366,:1439`); `make_synthetic_snapshot_with_conflict` exists to mirror RS-1.
- **#6 cap plumbing is contained and correct.** `FieldBuffer` (`form.rs:26-63`), `FIELD_CAP=64` (`:18`), checks at `push_char:39`/`set:50`. **No struct-literal constructions of `FieldBuffer` exist** (all via `::new`/`::default`), so adding a private `cap` field is safe. The donation `FieldForm` (`main.rs:3002-3011`) split is right: 6 free-text (`donee_name, donee_address, appraiser_name, appraiser_address, appraiser_qualifications, fmv_method_override`) vs 4 structured (`donee_ein, appraiser_tin, appraiser_ptin, appraisal_date`).
- **#7 effective condition + KAT rewrite verified.** "Neither `SafeHarborTimebar` nor `SafeHarborUnconservable`" is exactly the `effective`-vector condition (`resolve.rs:846-921`); the unconservable-first short-circuit does not affect the predicate. Empty-list status string is exactly `"No revocable decisions to void"` (`main.rs:2512`). The rewrite's claim that the void list is **empty** after attest is correct: `seed_safe_harbor_vault` (`main.rs:10321-10352`) seeds *only* the prior allocation; `persist_safe_harbor_attest` (`persist.rs:340-388`) appends `VoidDecisionEvent{prior}` + attested `SafeHarborAllocation`. So post-attest the revocable set is {prior (voided→excluded), attested (effective→#7-filtered), VoidDecisionEvent (non-revocable→excluded)} = ∅ → `void_flow.is_none()`. The current KAT does pin the trap (`:10974`), so the rewrite is genuinely required. Engine coverage retained (see M-1 for the path correction). Inert allocations (timebarred/unconservable) correctly stay listed.
- **#1 SelfTransfer detection is faithful.** `consumed_ins`/`links` are written *only* by the TransferLink arm (`resolve.rs:521,525`), so a TransferLink-only replica matches the engine; FIRST-WINS on dup out (`:492`), M-3 dup-in (`:501`), and I-1 no-wallet-in-event skip (`:509`) all replicate correctly (subject to M-2's order/lookup notes). `principal_sat = TransferOut.sat` is correct — `Op::SelfTransfer.sat = t.sat` (`resolve.rs:211-214`) and `honoring_principal` returns that same `sat` (`:1008-1016`); the TUI's `validate_select_lots` conserves against the *same* value the engine checks, so no principal-mismatch doom (unlike the under-covered case). `wallet = source wallet` is the right axis for the candidate filter. Blast radius is **exactly** the two exhaustive `match` on `DisposalKind` (`draw_edit.rs:1524-1529,1685-1690`); all other `DisposalKind::` sites are constructions. A no-source-wallet self-transfer raises `UncoveredDisposal` (`fold.rs:747`) and is caught by #3.
- **#3 shortfall pre-filter is exact.** For disposals/removals `principal_sat = Σ legs.sat` (`main.rs:3294,3320`); `Σ legs < op.sat` ⟺ `shortfall > 0` ⟺ `UncoveredDisposal` fired. Selecting lots can never cure under-coverage: the engine's `honoring_principal` is the full `op.sat`, which exceeds the short pool, so no conserving selection exists → pre-filtering removes only genuinely doomed disposals. SelfTransfer `principal_sat = op.sat` directly, so the `UncoveredDisposal` blocker is the correct universal gate over the merged list.
- **#2 mechanics (aside from I-1):** the filter site is `main.rs:2722` (candidate source `snap.state.lots`), and `TRANSITION_DATE` is not yet a value-import in `main.rs` (only in comments), so the spec's "import it" note is correct. `selection_feasible` cross-pool rejection is a fallback *plus* a hard blocker via `consume_principal` (this is the crux of I-1).

Re-review required after the I-1 fold (and confirm the corrected #2 gate + Path-B KAT).
