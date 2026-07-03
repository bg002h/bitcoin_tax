# Autonomous run roadmap — mutating-TUI: save-rollback → hardening → chunks 4/5 → burndown

**Mandate (user, 2026-07-02):** proceed autonomously through the post-chunk-3 burndown cluster
(save-rollback + the 6 hardening items), then chunks 4 and 5, then their burndowns; **ask an
architect (not the user) for burndown/sequencing decisions; STOP after the chunk-5 burndown.**

**Sequencing set by architect (`45b9332`).** Order: **A → B → C → D → E.** A-before-B is a hard
constraint (both edit the 9 opener heads); B-before-C/D is a safety + decay + template argument.

| # | Cycle | Scope | Status |
|---|---|---|---|
| A | `tui-edit-save-rollback` (#9) | `Vault::snapshot`/`restore` + `save_or_rollback` over the 8 persist fns; `rollback_failed` latch. | ✅ **SHIPPED** (main `8c8b924`) |
| B | `tui-edit-hardening` (6 items) | #1 SelfTransfer, #2 pre-2025 Universal gate, #3 UncoveredDisposal pre-filter, #6 FIELD_CAP, #7 void-list effective-alloc pre-filter, #8 quit-first. | ✅ **SHIPPED** (main `755e47c`) |
| C | chunk 4 — import decisions | 4a link-transfer+classify-raw + 4b resolve-conflict+optimize-accept. | ✅ **SHIPPED** (main `f31c1d6`; 921 tests) |
| D | chunk 5 — safe-harbor-allocate | CREATION side of SafeHarborAllocation (`reconcile.rs:250`; pre-2025 residue math via `transition.rs`). Completes create→attest→void loop. LARGE/COMPLEX. | **IN FLIGHT** — design |
| E | chunk-5 burndown (TERMINAL) | Clear C+D whole-branch-review followups (E4+E5). **Run STOPS here.** | pending D |

## Carried hazards / watch-items (from the sequencing architect)

1. **A retires the reason for the C1 latch → re-recon B against post-A HEAD** before B's spec
   (B's citations in `cycle-prep-recon-tui-edit-hardening.md` are at `45b9332`, pre-A; main.rs churns).
2. **#8 seeds the "quit-first, then CLI" status convention** — keep it authoritative; C and D copy it
   for every new CLI-pointing status string.
3. **`main.rs` (11k lines) is the shared collision file** — conflicts are ordered away at the
   function level (A→openers; B→openers+derivers+candidate-filter; C/D→new openers+derivers).
4. **`is_revocable_payload` (form.rs:824) already lists chunk-4/5 payloads** (TransferLink, ClassifyRaw,
   SafeHarborAllocation) → the void list surfaces them immediately → **#7 must land in B before chunk 5**
   makes create→attest→void reachable in-TUI.
5. **No CLI-surface lockstep expected** anywhere (B/C/D mirror existing reconcile commands, no new clap
   flags → no `schema_mirror`/manual-mirror) — confirm per-spec.
6. **C/D's new `persist_*` fns MUST use A's `save_or_rollback`** + KAT-G1 persist-only treatment for
   each new writer (as chunk 3 did for `donation_details::set`). Make it a checklist item in 4/5 specs.
7. **Chunk 4 is the overrun risk** — split 4a/4b rather than compressing review ceremony.

## Open scope fork (for A's R0 to adjudicate)
Sequencing architect assumed A *retires* the `attest_save_failed` latch. A's spec (per design-architects
1 & 2) deliberately **leaves attest out** of the rollback set and keeps its latch, filing "attest adopts
snapshot/restore + retires latch" as a FOLLOWUP. R0 decides: leave-out (conservative) vs. include-now.

## Per-cycle gate (every cycle, non-negotiable — STANDARD_WORKFLOW)
brainstorm (if new) → SPEC → **R0 architect review → 0C/0I** → PLAN → **R0 → 0C/0I** → phased TDD
(per-phase review) → **whole-diff review → 0C/0I** → ship (merge to main, flip FOLLOWUPS). Reviews
persisted verbatim to `reviews/` before folding. Citations verified at write time.
