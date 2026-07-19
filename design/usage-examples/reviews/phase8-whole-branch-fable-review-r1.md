# WHOLE-BRANCH REVIEW — `feat/post-v070-product-cycle` (109a27b9..92acb77, 102 commits) — Phase 8 cycle close

**Reviewer:** independent adversarial whole-branch pass, re-derived from current source. Read the full code diff (~41 code files), re-ran the validation surface, cross-checked goldens, reconciled the follow-up ledger, and probed the four cross-cutting seams named in the charter.

## Validation evidence (re-run, not trusted)

- `make check`: **2067/2067 passed, 8 skipped** — every skip inspected and principled (`#[ignore]` full-corpus twins run in CI, two golden regen helpers, one live-network smoke). Nothing improperly skipped.
- `cargo fmt --all --check`: clean. `scripts/pii-scan-generic.sh`: clean. `xtask check-isolation`: clean (ureq confined to btctax-update-prices).
- Golden gates re-run explicitly: `examples_golden_matches_committed`, `btctax_tui_goldens_match_committed`, `btctax_tui_edit_goldens_match_committed` — all PASS.
- Man-page idempotence: `xtask docs-man` regenerated **zero diff** — the committed `docs/man/*.1` (including the new `btctax-events.1`/`btctax-events-list.1` and the UX-P4-12(b)/UX-P4-4 help changes) are current with `cli.rs`.
- Merge topology: `git merge-base origin/main HEAD` == `origin/main` tip (109a27b9) — **main is a clean fast-forward target**; branch is pushed and up to date with its remote.

## Focus 1 — shared-surface conflicts: no defect found

- **render.rs** (5 phases touched it): the UX-P4-12(c) refactor extracted `voided_targets`/`method_election_lines` and `build_verify` consumes the extracted versions — `verify` and `config` share one implementation, no drift. `PseudoDisclosure` (UX-P4-1) is threaded as one type through delta report, dual report, and TUI tax tab; the full §3.1 predicate travels as a single value so no caller can drop a disjunct.
- **M-1 `preserve_order` blast radius is genuinely contained.** Fingerprints are hand-rolled length-delimited bytes (`persistence.rs:25` — no `Value` anywhere near them); stored payloads use typed serde (field-ordered regardless of the feature). The tripwire (`tax_profile.rs::m1_preserve_order_value_output_sites_are_enumerated`) scans every `crates/*/src` line for `serde_json::to_value`/`json!` and pins the three audited display-only sites; a companion test pins the feature as active. I independently verified no other workspace crate even depends on serde_json (store/tui/forms/adapters/xtask: none), so the per-crate flip covers the whole surface, including standalone (publish) builds. `income show` ordering is pinned by a distinguishing-pair KAT and the J6 golden was regenerated.
- **session.rs / main.rs**: `Session::open`'s PathIo enrichment (UX-P4-8) was checked against every consumer — the TUI unlock concise-message regression was fixed and pinned in the *shared* `btctax_tui::unlock` module, which btctax-tui-edit also uses, so both TUI binaries are covered. The `config` arm's second `Session::open` after `show_config` is safe (the admin helpers drop their temporary sessions before returning; NFR7 lock is never held twice).
- The stale "FIFO default" comments corrected to HIFO in resolve.rs/session.rs are doc-only: `fold.rs:43` confirms the fall-through has been `LotMethod::Hifo` all along.

## Focus 2 — integration correctness: one Minor composition wrinkle, otherwise sound

- **UX-P4-3 (`would_conflict`)** is definitionally the resolver (two-projection baseline diff, pseudo forced OFF, candidate appended as highest seq = losing side of any first-wins race) — I probed for false-accept/false-refuse constructions (seq drift, pseudo interplay, set-collapse) and found nothing new beyond the already-dispositioned already-blocked-vault residue (r1-N2/r2-class).
- **Composition loop is coherent:** refusal → `CONFLICT_HINT` names `events list` (which exists — #18 shipped before #14) → `events list` shows `[decided: decision|N]` → `void decision|N` → re-decide. `report --tax-year` exit contract (0/1/2) matches implementation exactly (main.rs:19–44, 201) and the dual-report/placeholder exit-0 non-triggers are KAT-pinned. The UX-P4-1 write-carryover gate (4a/4b) fires before the UX-P4-10 exit check can be reached, exactly as the main.rs comment claims. UX-P4-12(i) saves the draft while keeping the I-11 finalize guard (KAT asserts both `!exists(committed)` and `draft_exists`), per the user decision.
- **MINOR (NEW, cross-cutting UX-P4-4(d) × UX-P4-3):** in `cmd/reconcile.rs::reclassify_outflow`, the `--amount` FMV advisory is emitted to stderr *before* `guard_decision_conflict` runs. A duplicate/wrong-target reclassify with an implausible amount prints `warning: … recording it as entered (not fatal).` followed by `cannot record this decision — duplicate ReclassifyOutflow…` — the warning asserts a recording that is then refused. Reachable, contradictory, cosmetic (the refusal is loud, last, and nothing is appended). Fix: move the guard above the advisory block (or gate the advisory on the guard passing). Neither sub-review could see this cleanly: 4(d) predates the guard; the UX-P4-3 rounds focused on the guard's own semantics.

## Focus 3 — determinism / goldens: coherent

All committed goldens verified current against the code by regeneration-based gates (examples.md including J7/J8/J9, both TUI golden sets, man pages byte-idempotent under `docs-man`). Every change that should have regenerated a golden did (config output at b101fef, J6 under the preserve_order flip, the wording fixes). `generate_is_deterministic_and_captures_help` and the hermeticity test pass. The UX-P2-1 anchored matcher plus `new_journeys_demonstrate_their_reconcile_commands` close the false-coverage hole and are platform-unconditional. The Makefile `\m[` tripwire guards UX-P3-2 against silent monochrome regression.

## Focus 4 — half-finished / dangling work

- No TODO/FIXME/`dbg!`/commented-out code introduced by the branch.
- **User-mandated policies intact:** TP8 treatment (c) default untouched (only gained a human display label naming both treatments correctly); self-transfer zero-basis/never-gates default intact and its advisories re-pinned (wording-only changes, tests strengthened); HIFO default confirmed unchanged; DRAFT-gate policy untouched (full-return exports still clean, attestation still pseudo-only; the new write-carryover refusal is strictly *more* conservative); I-11 finalize guard kept per the stored user decision.
- **MINOR (overdue phase-owned follow-up):** UX-P4-3 **r2-N1** (FOLLOWUPS.md, owning phase = docs/#21): `cli.rs:655–658` and `docs/man/btctax-reconcile-reclassify-income.1` still describe the record-then-conflict-at-verify flow ("fires a Hard DecisionConflict blocker … void the prior decision first, then re-issue") — the CLI verb now *refuses at record time*. #21 closed green without folding or re-owning it. Per the workflow's own rule this is overdue, not deferred. Five-minute fix (one doc-comment + `docs-man` regen).
- **MINOR (overdue disposition, ledger reconciliation):** UX-P4-11 **M1** (FOLLOWUPS.md:2445, "owning phase = Step-1c / #14"): `events_list`'s voided-set still honors every `VoidDecisionEvent` without the resolver's revocability rule, and #14 closed without recording a disposition. I verified the defect is unreachable *going forward* (all three void surfaces — the guarded CLI verb, `voidable_decisions`-filtered bulk-void, `is_revocable_payload`-filtered TUI — refuse non-revocable targets), so only a vault written by ≤v0.7.0 binaries can exhibit it ("no users yet" makes that near-empty). But the ledger still reads as an open item whose owning phase passed: either land the one-line mirror or amend the entry to record "discharged by 1c unreachability; residual mirror re-owned to the events-list-M3 sweep."

## Focus 5 — merge-readiness

- Clean fast-forward onto `main`; full local CI surface green (with the caveat that msrv is CI-only — everything else re-verified here).
- **NIT (not in HEAD — in-flight courtesy flag):** while this review ran, an *uncommitted* concurrent edit to `FOLLOWUPS.md` appeared in the shared worktree (the Phase-8 burndown block, mtime 15:58). It claims UX-P4-12(a)'s runtime error "still does not enumerate" the valid kinds — **false against current source**: `eventref.rs:160–162` already enumerates them (fixed in the pre-v0.7.0 wording cycle, per FOLLOWUPS' own earlier entry). Correct that clause before committing. Also note the shared-worktree serialization rule while this reviewer and the writer overlap.
- The committed tui-walkthrough IMPLEMENTATION_PLAN is explicitly "HELD for owner approval" — a deliberate state; merging the branch ships the documents, not authorization to build.

## Findings summary

| # | Sev | New vs. dispositioned | Finding |
|---|-----|----------------------|---------|
| 1 | Minor | NEW (cross-cutting) | `reclassify_outflow` advisory ("recording it as entered") prints before the UX-P4-3 guard can refuse — contradictory message pair on the refusal path |
| 2 | Minor | Overdue (filed at #14-r2, owner #21 passed) | reclassify-income help + man page still describe verify-time conflict; verb now refuses at record time |
| 3 | Minor | Overdue disposition (filed at #18, owner #14 passed) | UX-P4-11 M1 revocability mirror in `events_list` neither landed nor formally re-owned; defect itself unreachable going forward |
| 4 | Nit | NEW (uncommitted tree only) | in-flight FOLLOWUPS burndown block misstates UX-P4-12(a) as unfixed |

None of these is Critical or Important; per §severity, Minors and Nits are recorded and do not hold the gate. Findings 1 and 2 are each <15-minute fixes — recommend sweeping them (plus the #3 ledger amendment and #4 correction) either before merging or as the first post-merge commit, so the cycle's own per-phase-burndown rule closes clean.

## VERDICT

**GREEN — 0 Critical / 0 Important — MERGE-READY.** The branch is coherent end to end: the shared hot files compose correctly across all eight phases, the preserve_order flip's blast radius is proven contained, every committed golden is current with the code, the exit-code/guard/discoverability changes form one consistent loop, no user-mandated policy was weakened, and `main` is a clean fast-forward target. Three recorded Minors (one new cross-cutting, two overdue follow-up dispositions) and one nit in an uncommitted concurrent edit; none gates.

---
FOLD (2026-07-19, Phase 8): all four non-gating findings swept before merge-ready —
(1) reclassify_outflow guard moved above the --amount advisory (reconcile.rs); (2)
reclassify-income doc-comment corrected to record-time refusal + man regen; (3) UX-P4-11
M1 amended with a Phase-8 disposition (discharged by unreachability, mirror re-owned);
(4) the burndown (a) clause corrected (eventref.rs:161 enumerates the kinds). make check
2067 green after the folds.
