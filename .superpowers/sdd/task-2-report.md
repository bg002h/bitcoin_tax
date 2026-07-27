# Task 2 report — Extract the DECLARE chokepoint (`plan(target_shortfall: Option<EventId>)` + clearance)

Status: **DONE**

## Summary

Extracted the shipped `cmd::tranche::declare_tranche` verb into a reusable `plan_declare`/`apply_declare`
chokepoint in `crates/btctax-cli/src/chokepoint/mod.rs` (Task 1's module), mirroring the `PromotePlan`
trio's shape. Added the target-scoped clearance shadow (`target_shortfall: Option<EventId>`) so a future
dashboard can verify a candidate tranche actually covers a shortfall, while the free-form CLI path
(`target_shortfall = None`) stays byte-for-byte identical to the shipped verb.

## Files changed

- `crates/btctax-cli/src/chokepoint/mod.rs` — added `DeclarePlan { payload: EventPayload }`,
  `plan_declare(...)`, `apply_declare(...)`; extended the module doc header; amended `Refusal::Coverage`'s
  doc comment to note its dual use (promote's floor-coverage AND declare's shipped-set gates + clearance).
- `crates/btctax-cli/src/cmd/tranche.rs` — bumped `guard_tranche_vs_allocation` from private to
  `pub(crate)` (so the chokepoint can call it — it stays DEFINED here, not duplicated, per the parent
  orchestrator's instruction); reduced `declare_tranche` to a thin driver over
  `plan_declare(..., target_shortfall=None, ...)` / `apply_declare`; the phantom-wallet stderr warning
  (tax-M-3) stays in the driver, emitted after `plan_declare` succeeds, before `apply_declare`; dropped
  now-unused imports (`DeclareTranche` struct, `append_decision`, `UtcOffset`).
- `crates/btctax-cli/tests/chokepoint_declare.rs` (new) — 7 tests: 2 characterization (Step 1) + 5
  behavior KATs (Step 5).

## Interfaces produced (match the brief verbatim)

```rust
pub struct DeclarePlan { pub payload: EventPayload }
pub fn plan_declare(events: &[LedgerEvent], prices: &dyn PriceProvider, cfg: &ProjectionConfig,
    sat: Sat, wallet: WalletId, window_start: TaxDate, window_end: TaxDate,
    target_shortfall: Option<EventId>, now: OffsetDateTime) -> Result<DeclarePlan, Refusal>;
pub fn apply_declare(session: &mut Session, plan: DeclarePlan, now: OffsetDateTime) -> Result<EventId, CliError>;
```

`plan_declare` gates on the shipped set ALWAYS (`sat>0`, `ws<=we`, `guard_tranche_vs_allocation`, which
stays defined in `cmd::tranche`). **Iff `target_shortfall = Some(id)`**, it additionally: appends the
candidate `DeclareTranche` → re-projects with `pseudo_reconcile` FORCED off on a config copy (mirroring
`would_conflict`, `project/mod.rs:118`) → asserts no `BlockerKind::UncoveredDisposal` remains on `id`; else
`Refusal::Coverage`. `apply_declare` is a plain append+save — no acknowledgment gate, no `would_conflict`
pre-check (the shipped verb never ran one; declaring is DFW-D8's plain `$0`/revocable confirmation, unlike
promote's typed-phrase tier).

## One design decision not fully specified by the brief (documented, not blocking)

The brief names `Refusal::Coverage` explicitly for the new clearance-shadow failure, but is silent on which
of the shared enum's four variants (`Target`/`Provenance`/`Coverage`/`PartII`) should wrap the three
"shipped-set" gate failures (`sat<=0`, `ws>we`, `guard_tranche_vs_allocation` conflict). The plan's
Self-Review explicitly rejects adding a new `Conflict` variant ("`Refusal` (`Target`; no `Conflict`)"), so
the enum must stay closed at four. None of `Target`/`Provenance`/`PartII` fit these declare-side gates
semantically (they're promote-specific: resolve-live target, acquisition provenance, Form 8275 narrative).
I mapped all three shipped-set gates to `Refusal::Coverage` as well — this is a pure internal-taxonomy
choice with **zero observable effect**: `From<Refusal> for CliError` (Task 1) collapses every variant to
the identical `CliError::Usage(msg)`, and every existing test asserts on the message text /
`CliError::Usage`, never on the `Refusal` variant. Documented in the module doc's "Refusal-variant note"
and on the `Coverage` variant's doc comment so a future task/reviewer can retaxonomize freely without
touching behavior. Did not treat this as a blocking ambiguity per the task's own criteria (no wrong
result / no unmet guarantee turns on this choice).

## TDD evidence

**Step 1 (characterization) — pin shipped `declare_tranche` before any refactor code existed:**
Both characterization tests (`shipped_declare_tranche_zero_basis_declare_succeeds`,
`shipped_declare_tranche_refuses_under_effective_allocation`) were written and passed against the
UNMODIFIED shipped verb first (Step 2: PASS), confirming they pin real, already-correct behavior — then
stayed green through the Step 3/4 refactor (proving behavior-preservation), per the same methodology Task
1 used.

**Mutation 1 — clearance shadow disabled (`if false && still_uncovered`):**
```
$ cargo test -p btctax-cli --test chokepoint_declare
test some_path_candidate_at_disposal_date_fails_clearance ... FAILED
test clearance_shadow_forces_pseudo_off_a_pseudo_selftransfer_cannot_mask_a_real_shortfall ... FAILED
test result: FAILED. 5 passed; 2 failed
```
Reverted → both green again. Confirms both refusal-path KATs actually depend on the clearance logic, not
an accident of the fixture.

**Mutation 2 — `honest_cfg.pseudo_reconcile = false;` commented out (arch-I-5):**
```
$ cargo test -p btctax-cli --test chokepoint_declare
test clearance_shadow_forces_pseudo_off_a_pseudo_selftransfer_cannot_mask_a_real_shortfall ... FAILED
    called `Result::unwrap_err()` on an `Ok` value: DeclarePlan { payload: DeclareTranche(... sat: 1 ...) }
test result: FAILED. 6 passed; 1 failed
```
i.e. with pseudo left on, the pseudo `SelfTransferMine{$0}` default (from an unresolved `TransferIn`)
masked the real shortfall and the clearance shadow incorrectly reported "clears" for a candidate that does
not itself cover anything. Reverted → green again. Confirms the pseudo-off line is load-bearing.

**Final green run (targeted tests, per the controller's build-hygiene instruction):**
```
$ cargo test -p btctax-cli --test chokepoint_declare --test declare_tranche_cli
running 7 tests ... test result: ok. 7 passed; 0 failed
running 17 tests ... test result: ok. 17 passed; 0 failed
```
Also re-ran Task 1's suites to confirm no regression from the shared-module edits:
```
$ cargo test -p btctax-cli --test chokepoint_promote --test promote_cli
running 3 tests ... ok. 3 passed
running 25 tests ... ok. 25 passed
```
`cargo build -p btctax-cli` — clean, zero warnings. `cargo clippy -p btctax-cli --tests --all-targets` —
clean, zero warnings (the `#[allow(clippy::too_many_arguments)]` on `plan_declare`, 9 positional args
matching the brief's exact signature, is intentional and needed — `plan_promote` sits at exactly 7, the
threshold, so it needed none). `cargo fmt --all` applied; `cargo fmt --all --check` clean. Sanity-built the
downstream consumer crate, `cargo build -p btctax-tui-edit` — clean.

## KATs added (Step 5, `chokepoint_declare.rs`)

1. `none_path_targets_no_shortfall_is_not_refused` — (a) the CLI `None` path via the new
   `plan_declare`/`apply_declare` is not refused and folds a $0 lot (shipped preserved).
2. `some_path_candidate_at_disposal_date_fails_clearance` — (b) a candidate whose `window_end == disposal
   date` refuses via `Refusal::Coverage` (a decision's synthetic acquisition sorts AFTER a same-instant
   import, so the lot isn't in the pool yet when the same-day disposal folds).
3. `some_path_candidate_before_disposal_date_clears` — (b) mutation: the same candidate with `window_end`
   moved to the day before the disposal clears.
4. `clearance_shadow_forces_pseudo_off_a_pseudo_selftransfer_cannot_mask_a_real_shortfall` — arch-I-5: with
   the STORED config's `pseudo_reconcile = true` and an unresolved `TransferIn` sized exactly to the
   shortfall, a non-covering candidate still correctly refuses (pseudo forced off inside the shadow).
5. `apply_declare_clears_the_uncovereddisposal_blocker_on_the_targeted_disposal` — the cleared-row KAT: a
   clearing candidate, once applied, leaves NO `UncoveredDisposal` blocker on the targeted disposal in the
   real (post-apply) projection.

Plus 2 characterization tests (Step 1) described above. `declare_tranche_cli.rs` (shipped, 17 tests) run
green, unmodified.

## Self-review

- **Gate ordering preserved:** `plan_declare`'s `None` path runs exactly the three shipped checks in the
  shipped order (`sat>0` → `ws<=we` → `guard_tranche_vs_allocation`), replicating `cmd/tranche.rs:134-154`.
  The phantom-wallet warning stays I/O in the driver, firing only after a successful plan — matching the
  shipped verb's "guard first, warn only once admitted" ordering (arch N-1), even though no existing test
  independently asserts on stderr silence for a refused declare.
- **No new tax logic:** every filed number (the `$0` basis, `EstimatedConservative` tag, `window_end`
  homing) is unchanged — `plan_declare` only reshapes control flow and adds a pure re-projection check that
  never touches what gets appended.
- **`guard_tranche_vs_allocation` single-sourced:** stayed defined in `cmd/tranche.rs` (not duplicated into
  the chokepoint) per the parent orchestrator's instruction; only its visibility widened to `pub(crate)`.
- **Pseudo-off is load-bearing and tested, not just asserted in a doc comment** — see Mutation 2 above
  (this project's own memory flags "shipping a correct fix with no test holding it" as a recurring
  failure mode; avoided here deliberately).
- **arch-m-new-3 preserved:** `plan_declare` takes no `Session`/state, only already-loaded
  `events`/`prices`/`cfg` — identical shape to `plan_promote`.
- **Known/accepted side-effect of arch-m-new-3:** the thin driver now calls `Session::open` +
  `session.config()` BEFORE the `sat<=0`/`window_start>window_end` checks (previously these ran with "no
  vault access needed"). This is an unavoidable consequence of `plan_declare` needing already-loaded
  `events`/`cfg`, mirrors the identical tradeoff already accepted in Task 1's `plan_promote`, and is not
  observable in any current test (every fixture opens a valid vault regardless of the sat/window values
  under test).
- **Minor/Nit (non-blocking):** `plan_declare` has 9 positional arguments (brief's exact signature) —
  flagged via `#[allow(clippy::too_many_arguments)]` rather than restructured, since the brief specifies the
  signature verbatim and a struct-of-args would deviate from it.

## Concerns

None blocking. The one open item is the `Refusal` variant-mapping decision documented above (Minor,
non-observable, explicitly reasoned through) — flagging for the whole-branch review in case a later task
(building `journey_view`/the dashboard, Tasks 5-8) wants a more granular declare-specific taxonomy; nothing
here needs to change for THIS task's scope or tests to hold.

## Commit

`refactor(chokepoint): declare plan/apply + target-scoped clearance` (per the brief's exact wording).
