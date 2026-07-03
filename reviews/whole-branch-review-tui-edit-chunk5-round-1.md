# Whole-branch review — tui-edit-chunk5 (Phase E, round 1)

**Branch:** `feat/tui-edit-chunk5` @ `f59e7fc` (diff `main..HEAD`, main == `f31c1d6`; commits: spec
`64f168e`, Task 1 `cf171fd`, Task 2 `1028a40`, Task 3 `f59e7fc`). Delegated-implementer cycle;
independent gate.
**Spec:** `design/SPEC_tui_edit_chunk5.md` (R0-GREEN, 2 rounds).
**Controller-verified full gate at HEAD:** 931 workspace tests, clippy `-D warnings` clean, fmt clean.
`btctax-core` untouched; `btctax-cli` gains only the additive `Session::safe_harbor_residue`.

## Controller fold disposition
All findings are Minor/Nit (non-blocking; "no change required to ship"):
- **[M-DATE] Minor** — the 2 allocate E2E tests embed "today > 2026-04-15" → **no code change**
  (monotonically safe: passes now and forever forward; production is date-correct; date-independent
  arm-3 coverage via the ProRata seed already exists). Recorded as a FOLLOWUP.
- **[N1] Nit** — opener doc comment over-lists `load_all`/`project` as gated → recorded (cosmetic).
- **[N2] Nit** — `AllocLotRow` duplicates `AllocLot` → recorded (harmless convention).
- **[N3] Nit** — `fmt_btc` vs existing `sat_to_btc` → recorded (cross-crate, cosmetic).

## Reviewer output (verbatim)

# R0 Whole-Diff Review — btctax-tui-edit chunk 5: safe-harbor-allocate (`A`) CREATION flow

## Verdict: **0 Critical / 0 Important / 1 Minor / 3 Nit — PASSES. Ship gate: GREEN.**

The implementation faithfully realizes the R0-GREEN spec. The residue helper is byte-identical to the CLI subset it factors out and the refactor is behavior-preserving; the six eligibility guards are a strict superset of the CLI; residue is method-independent; persist is a clean single-append with correct rollback; and the status arms are keyed correctly. `btctax-core` is untouched (0 files). All 9 new tui-edit KATs + the CLI KAT pass; the 3 fault-injections I ran went RED as required. No blocking finding.

## Verification highlights (against source)

**1 — Helper + CLI refactor (additive, behavior-preserving).** `Session::safe_harbor_residue` (`session.rs:181-220`) reads config ONCE (`cfg.pre2025_method` returned; `cfg.to_projection()` drives the residue) — the M1 fold landed exactly as specced. Subset logic (`Import → tax_date < TRANSITION_DATE`; else drop `SafeHarborAllocation`), `project`, `residue.lots.filter(remaining_sat>0)→AllocLot` are byte-identical to the pre-refactor command body. The refactored `cmd::reconcile::safe_harbor_allocate` (`reconcile.rs:280-298`) destructures `(lots, pre2025_method)` and records the RETURNED method. Public-API delta is additive only. `safe_harbor_residue_matches_command_lots` pins BOTH lots AND method. Existing `safe_harbor_*` reconcile tests (570/636/682/733/828 + explicitly_attested_fifo) — ran all 7, GREEN.

**2 — KAT-G1 cleanliness.** The gate (`persist.rs:1212-1508`) scans non-test regions for the 7 persist-only tokens, excluding only `edit/persist.rs`. New opener/modal/status delegates writes to `persist_safe_harbor_allocate` and residue math to `session.safe_harbor_residue()`; no forbidden token leaks into main.rs. KAT-G1 GREEN.

**3 — Eligibility guards (`open_safe_harbor_allocate_flow`, `main.rs:4967-5065`).** All six steps present/ordered: latch → snapshot → `pre2025_method_attested` gate → residue non-empty → non-voided-`SafeHarborAllocation` scan → open. The live-allocation guard builds the voided set from `VoidDecisionEvent.target_event_id` then scans non-voided — correctly excludes voided priors, stricter than the CLI. Four refusal KATs cover steps 3/5/4/1.

**4 — Residue method-INDEPENDENCE (G3).** `safe_harbor_residue` depends only on `config.pre2025_method`, never `AllocMethod`. Rows/totals computed ONCE at open; the `Tab` toggle only sets `flow.method`, never recomputes. The banner states the lots are identical across methods; the modal doesn't imply ProRata redistributes basis. Matches core O4.

**5 — persist + status.** `persist_safe_harbor_allocate` (`persist.rs:501-522`) is the single-append template (no latch, no side-table; `timely_allocation_attested: false`, `as_of_date=TRANSITION_DATE`, helper-returned `pre2025_method`). `derive_allocate_status` arms keyed to `new_id`; arm 3 (`SafeHarborTimebar`) is the expected outcome. Rollback KAT correct.

**Arm-2 `event: None` soundness (R0-N2) — verified safe.** Enumerated every `DecisionConflict` emission: all 20 are in `resolve.rs`; the ONLY `event: None` one is the "multiple effective SafeHarborAllocations" case (`:962-963`). So arm 2's `event.is_none()` predicate can never be triggered by an unrelated conflict — the exception is exact and stale-free.

## Fault-injection probes (all RED then restored; tree byte-clean)
1. `persist.rs:517` `timely_allocation_attested: false→true` → `kat_persist_allocate_single_append_strict_prefix` RED ("must be REVOCABLE").
2. `main.rs:5034` live-alloc `.any(...)→.any(|_e| false)` → `kat_allocate_refuses_when_live_allocation_exists` RED.
3. `main.rs:318` `A` binding → no-op → `kat_e2e_allocate_then_void` RED.
`git status --porcelain` empty after restoration.

## Item-6 date-dependence assessment (explicit)
**The two E2E tests are date-dependent but monotonically safe; production is correct regardless of date; no masked bug.** Timebar rule (`resolve.rs:859-866`): unattested `ActualPosition` is timebarred ⟺ `made > bar`, `bar = min(first_2025_disposition, TY2025_RETURN_DUE=2026-04-15)`. The E2Es create with `now_utc()` (2026-07-03), no 2025 disposition → `made > bar` → timebarred/inert. If run BEFORE 2026-04-15 the allocation would be immediately effective and the "REVOCABLE, timebarred" assertion would fail — but that window is permanently closed, so the tests pass now and forever forward. **Production is correct at any date** (uses `now_utc()`, the semantically-correct creation time; the timebar decision is date-driven and correct — no boundary bug). A date-injection fix isn't feasible through the public key handler and isn't needed: `kat_allocate_status_timebarred` gives date-independent arm-3 coverage via a ProRata-unattested seed (unconditionally timebarred). Recorded as Minor (test hygiene), non-blocking.

## Findings
### [M-DATE] MINOR — the two allocate E2E tests embed an implicit "today > 2026-04-15" assumption
Passes now and monotonically forward; production is date-correct; date-independent arm-3 coverage exists. Non-gating. Fix optional (leave as-is with the existing G2 comments, or add a `now < 2026-04-15` skip-guard).
### [N1] NIT — opener doc comment lists `load_all`/`project` as if KAT-G1-gated (they are not); intent correct, list inaccurate. Cosmetic.
### [N2] NIT — `AllocLotRow` is a byte-identical duplicate of `AllocLot` (`TargetList<AllocLot>` would suffice). Harmless.
### [N3] NIT — new `fmt_btc` vs existing `sat_to_btc` (different type/crate; outputs agree). Cosmetic.

## Also confirmed
`btctax-core` untouched (0 files). `close_all_mutation_surfaces` clears both new fields; `new()` inits both `None`; `A` bound; modal dispatched before flow; mutual exclusion holds. FOLLOWUPS `[C5-1]` records the ProRata gap (core O4); ROADMAP updated. No exhaustive-match gaps; no dead code (arm 4 intentionally retained per G2).
