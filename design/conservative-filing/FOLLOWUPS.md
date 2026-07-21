# Conservative-filing — follow-ups (per-phase burndown by ownership)

Each item has an **owning phase**; reconcile at the top of that phase (do the ones it owns, confirm
the rest are parked later). Only ownerless cross-cutting residue batches to the end. Blocking findings
(Critical/Important) are NEVER filed here — they are fixed before their gate closes.

## Open

### From the Phase-1 implementation review (r1, 2026-07-21) — all Minor/Nit, non-blocking

- **[Minor] `build_op`'s DeclareTranche arm + engine-level input validation are reachable only from the
  CLI record path, not the engine.** (tax r1 Minor 1 + arch r1 Nit 1) — `resolve.rs:396` (the arm) folds a
  `$0 EstimatedConservative` lot for ANY `DeclareTranche` payload that reaches it, and a hand-crafted vault
  can route one through the import/`applied` path (`ClassifyRaw{as_: DeclareTranche}` or an Import-id
  DeclareTranche) → the lot homes at the import timestamp (bypassing D-2) and skips the `sat>0`/window
  guards (which live only in `cmd/tranche.rs`). No product surface can author this (the CLI `classify-raw`
  verb refuses non-`is_imported()` payloads; a hand-crafter can already forge a worse arbitrary-basis
  `Acquire`), and the projection backstop still fires on a pre-2025 tranche lot. **Fix:** guard the
  `build_op` arm on `EventId::Decision` (else `Op::Skip`) and/or add a resolve-time blocker for a `sat ≤ 0`
  tranche, so the engine posture is uniform against hand-crafted vaults. **Owning phase: P9 / Task 15**
  (the engine never-understate-invariant + integrity task).

- **[Minor] `in_force_allocation_exists` diverges from engine void semantics for a dangling void on an
  EFFECTIVE allocation.** (arch r1 Minor 1) — the record-time predicate treats ANY `VoidDecisionEvent`
  target as not-in-force, but the engine keeps an *effective* allocation in force after a (conflicting)
  void (`resolve.rs:466-472,1344-1352`). Product-reachable: allocate→effective → `reconcile void
  decision|N` (Hard conflict, allocation stays effective) → `declare-tranche` pre-2025 records CLEANLY
  (the friendly refusal the SPEC promises for "ANY in-force allocation" doesn't fire). The T5 backstop
  still catches it loudly on the next projection (Path B → Path A), and the vault is already hard-blocked
  by the bad void — so no wrong tax, only a missed friendly refusal. **Fix:** on the allocation side of the
  predicate, count a void only if the allocation is engine-inert (mirror `voidable_decisions`' blocker-keyed
  `effective_alloc`), or refuse the tranche when any non-voided-OR-effective allocation exists. **Owning
  phase: T16 (whole-branch review)** — revisit the friendly-layer/engine-semantics alignment before merge.

- **[Minor] `safe_harbor_residue`'s throwaway projection mis-states documented remainders when pre-2025
  disposals actually consumed tranche sats.** (tax r1 Minor 4 + arch r1 Minor 3) — `session.rs:687-700`
  excludes the `DeclareTranche` event but keeps pre-2025 disposal imports, so a filer whose pre-2025 sale
  was covered by the tranche gets a short residue projection (an ignored "dispose short" blocker; the sale
  eats documented lots the tranche covered) → the **TUI allocate opener** displays an understated/empty
  allocatable residue. Fail-closed: the CLI allocate guards before residue, and TUI persist refuses, so
  nothing wrong is recorded. **Fix:** the plan's own alternative — refuse opening the allocate flow when a
  pre-2025 tranche exists (the CLI path already effectively does) — or pin the disposal-present skew as
  accepted with a comment/test. **Owning phase: T16 (whole-branch review)** (TUI-opener UX; no P2–P8 phase
  owns the allocate opener).

- **[Nit] `--wallet` is not validated against wallets known to the vault.** (tax r1 Nit) — a typo strands
  the `$0` lot in a phantom wallet (tax-neutral: it still files at $0 in whatever wallet). **Fix:** warn (not
  refuse) when `--wallet` names a wallet with no prior events. **Owning phase: P8 / Task 14** (the
  self-custody nudge / advisory task — the reviewer's suggested home).

- **[Nit] The future-`window_end` warning computes "today" in UTC, not the filer's zone.** (tax r1 Nit) —
  `main.rs` dispatch compares `window_end` against `tax_date(now, UtcOffset::UTC)`; near midnight in a
  behind-UTC zone it can UNDER-warn (never mis-record — the warning is advisory, the lot still homes at
  `window_end`). **Fix:** use the filer's configured offset if/when one exists. **Owning phase: P8 / Task 14.**

### Filed test-pins the review named (were only in the review doc until r2 — arch r2 Nit 2)

- **[Nit] No test asserts `row.date_acquired == window_end` on a tranche 8949 row.** (tax r1 Nit 2) — the
  D-6 column mapping is held indirectly (`lot.acquired_at == window_end` + `forms.rs` copies it to
  `date_acquired`); a direct one-assert would pin col (b) end-to-end. **Owning phase: P9 / Task 15**
  (the invariant/test-pin task).
- **[Nit] No Σ-conservation assert over a projection containing a tranche.** (tax r1 Nit 3) — the
  `sigma_in` bump is structural (shared `Op::Acquire` arm) but unpinned. **Owning phase: P9 / Task 15**
  (beside the never-understate invariant KAT).

### From the Phase-1 fold re-review (r2, 2026-07-21)

- **[Nit] SPEC.md §104 / IMPLEMENTATION_PLAN.md §556 quote the pre-split "both directions" hedge.** (tax
  r2 Nit) — after the refusal-hint split into `ALLOCATION_IS_FINAL_HINT` / `TRANCHE_IS_FINAL_HINT`, the
  allocation-side message no longer contains the verbatim quoted sentence (both directions still satisfy
  the normative "hedges irrevocability" requirement). Doc-consistency only. **Owning phase: T16**
  (whole-branch doc-consistency sweep).

## Folded during the Phase-1 gate (2026-07-21) — recorded for the audit trail

- **[Important] TUI void flows rendered a DeclareTranche as `?`/`?`** (arch r1) — FIXED: added the
  `DeclareTranche` arm to `summarize_void_payload` + a human-label KAT (mirrors the CLI sibling).
- **[Minor] Both attest-site guards untested** (tax r1 Minor 3 + arch r1 Minor 2) — FIXED: CLI +
  TUI attest-guard tests, each mutation-proven RED.
- **[Minor] Backstop KAT missed the SPEC-named inert-then-declare ordering** (arch r1 Minor 4) — FIXED:
  seq-swapped twin KAT (`backstop_fires_when_the_allocation_is_recorded_before_the_tranche`).
- **[Minor] ≥2025 non-poisoning had no assertion** (tax r1 Minor 2) — FIXED: extended CLI test (d) to
  assert the allocation stays effective + the ≥2025 tranche coexists, and (r2 tax Minor) DIRECTLY pins
  Path B via `basis_source == SafeHarborAllocated` so a no-blocker Path-A flip goes RED.
- **[Nit] Direction-crossed refusal hint** (tax r1) — FIXED: split into `ALLOCATION_IS_FINAL_HINT` /
  `TRANCHE_IS_FINAL_HINT`; (r2 arch/tax Nit) capitalized the sentence-initial "Void".
- **[Nit] KAT-G1 re-export needed a rationale comment** (arch r1) — FIXED: comment at the `lib.rs`
  re-export stating the pure-predicate exemption.
