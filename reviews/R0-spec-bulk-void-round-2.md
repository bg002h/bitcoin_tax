# R0 — SPEC_bulk_void.md — round 2 (independent architect, verification of the fold)

**Artifact:** `design/SPEC_bulk_void.md` @ `9a3f90e` (branch `feat/bulk-void`; main == `13cb135`).
**Bar:** 0 Critical / 0 Important. **Scope:** read-only; verify each round-1 fold (0C/0I/3M/3N) against
CURRENT source, no new drift. Round 1 was R0-GREEN with both flagged decisions adjudicated (bespoke persist,
Tier-B).

## Verdict: **0 Critical / 0 Important / 0 Minor / 1 Nit — R0-GREEN.**

Every round-1 finding is resolved against current source. The one commit since round 1 (`3bee644..9a3f90e`)
is the fold commit itself — it touches ONLY `design/SPEC_bulk_void.md` and the round-1 review file; **zero
`crates/` source changed**, so no regression surface was introduced and all round-1 source verifications
still hold. I independently re-confirmed each cited range against current source (below). The lone Nit is a
readability observation about crate-ambiguous `main.rs` cites; it does not block implementation.

---

## Fold verification

### M1 — effective-allocation engine cites — RESOLVED
Spec §Candidate-set item 4 now cites `resolve.rs:989-994` / `997-1002` / `1030-1039` and flags the stale
`865-921` cite in `main.rs:2749` for Task-1 fixing. Confirmed against `resolve.rs`:
- **`989-994`** — `if unconservable { blockers.push(Blocker { kind: BlockerKind::SafeHarborUnconservable, event: Some(d.id), … }); }` (line 995 is the `continue`). Exactly the unconservable⟹blocker firing. ✓
- **`997-1002`** — `if timebarred { blockers.push(Blocker { kind: BlockerKind::SafeHarborTimebar, event: Some(d.id), … }); }` (1003 is the `continue`). Exactly the timebarred⟹blocker firing. ✓
- **`1030-1039`** — the §7.4 irrevocability comment (1030-1031) + `for v in &allocation_voids { if effective.iter().any(|(id,_,_)| id == &v.target) { blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: Some(v.void_id), … }); } }`. Exactly the void-of-effective ⟹ `DecisionConflict`. ✓
- **"Hard" is accurate** (independently checked, since the spec leans on "Hard … gates the whole tax year"):
  `state.rs:68-83` folds `DecisionConflict` into `severity() => Severity::Hard`; `compute.rs:199,237` — ANY
  unresolved `severity()==Hard` blocker anywhere ⟹ `TaxYearNotComputable`. So the year-gating framing holds. ✓
- The stale `resolve.rs:865-921` cite indeed still sits in `main.rs:2749` (tui-edit) — as EXPECTED: the spec
  defers fixing that inherited comment to Task 1, not the spec. Correctly flagged, not silently left. ✓

### M2 — inert-void invariant + KAT cite — RESOLVED
Spec now separates the source invariant `resolve.rs:1030-1031` from the KAT
`crates/btctax-core/tests/transition.rs:403` (no bare `transition.rs:403` source line). Confirmed:
- **Source invariant `resolve.rs:1030-1031`** — comment: "a Void of an EFFECTIVE allocation → conflict (it
  stays in force); a Void of an inert/absent allocation simply applies (no conflict; Path A already governs)."
  Correct statement of "inert voids apply cleanly." ✓
- **KAT `crates/btctax-core/tests/transition.rs:403`** — `fn void_of_inert_allocation_applies_no_conflict`;
  voids an inert (timebar) `SafeHarborAllocation` and asserts `!has(&st, BlockerKind::DecisionConflict)`
  (line 429) + `Path A` (basis_source `ReconstructedPerWallet`, 430-433). Exactly the pinned behavior. ✓
- The two `transition.rs` files are now disambiguated (`crates/btctax-core/src/project/transition.rs` is only
  103 lines; the KAT lives in the `tests/` file). ✓

### M3 — CLI #7-enforcement-at-apply is explicit — RESOLVED (closes the CLI-layer #7 gap)
Spec §CLI now states: `apply_bulk_void`'s `targets` MUST be the predicate-filtered `bulk_void_plan` rows
re-derived in the dispatch, NEVER raw `--ref` ids, with a plan-level KAT feeding an effective allocation's id
and asserting the plan omits it. Cross-checked against the shipped pattern it mirrors:
- **The derive-from-`plan.rows` dispatch is real and is at the cited `main.rs:1267-1268` (in the CLI crate,
  `crates/btctax-cli/src/main.rs`):** `let conflict_events: Vec<_> = plan.rows.iter().map(|r| r.conflict_event.clone()).collect();` — apply-targets are derived from the predicate-filtered plan, and the
  interactive `y/N` confirm is a thin shell (1242-1278). `apply_bulk_accept_conflicts` (`reconcile.rs:315-332`)
  takes those ids and does NO re-check. ✓
- **The single CLI `void` (`reconcile.rs:110-149`) does NO `effective_alloc`/blocker check** — it parses the
  ref, finds the `LotSelection` disposal, appends `VoidDecisionEvent`, clears the attestation, saves. So a
  raw-id bulk path WOULD let a caller void an effective allocation ⟹ Hard `DecisionConflict`. The spec's claim
  that plan-derivation is "the ONLY CLI-layer #7 defense" is exactly right, and the fold now pins it. ✓
- **Why still not Important:** even the worst case is a LOUD Hard blocker, not silent corruption; the fold
  turns an implicit contract into an explicit one + a KAT. Correctly Minor→resolved.

### N1 — shared-disposal double-clear idempotent — RESOLVED
Spec §Persist [R0-N1] states two `LotSelection`s targeting one disposal call `optimize_attest::clear` twice —
harmless idempotent DELETE; precompute MAY dedup but correctness doesn't depend on it. Confirmed:
`optimize_attest::clear` (`crates/btctax-cli/src/optimize_attest.rs:86-93`) is `DELETE FROM optimize_attestation WHERE disposal_event=?1`, doc line 83 "idempotent — no-op if absent." ✓

### N2 — lockstep comment on the bespoke fn — RESOLVED
Spec §Persist [R0-N2] now requires the bespoke `persist_bulk_void` to carry a lockstep comment
cross-referencing `persist_bulk_decisions` (the mirrored safety skeleton) so a future edit to the shared
rollback invariant is echoed. Present in the spec as an implementation requirement. ✓

### N3 — `optimize_attest` is cli, not core — RESOLVED
Spec §Core no longer claims core "reuses … the `optimize_attest` side-table"; it now reads core "Reuses the
existing `VoidDecisionEvent`" and carries the explicit [R0-N3] note that the `optimize_attest` side-table is
`btctax-cli`, not core, and the per-`LotSelection` clear is a cli/tui-edit concern. Confirmed
`optimize_attest.rs` lives at `crates/btctax-cli/src/optimize_attest.rs` (not core). ✓

---

## Regression spot-checks (nothing drifted)

- **Extracted predicate still byte-equivalent to `open_void_flow`** — `main.rs:2733-2770` (tui-edit) filters:
  `matches!(e.id, EventId::Decision{..})` (2767), `!voided.contains(&e.id)` (2768; `voided` built 2733-2744),
  `is_revocable_payload(&e.payload)` (2769), `!effective_alloc(e)` (2770; `effective_alloc` at 2752-2762 =
  `SafeHarborAllocation` ∧ ¬`SafeHarborTimebar` ∧ ¬`SafeHarborUnconservable` blocker on `e.id`). This is
  verbatim the 4-clause predicate the spec quotes. `is_revocable_payload` (`form.rs:896-911`) is an allow-list
  that INCLUDES `SafeHarborAllocation` but EXCLUDES `VoidDecisionEvent`/`SupersedeImport`/`RejectImport` — so
  item 3 alone does NOT exclude effective allocations; the `!effective_alloc` clause (item 4) is the sole #7
  defense, consistent with the spec's "#7 is the whole ballgame." ✓
- **Persist atomicity anchors unchanged** — `persist_void`@`persist.rs:248`, `persist_bulk_decisions`@`394`,
  `rollback`@`62`, `Session::snapshot`@`session.rs:247`, `restore`@`253`. Whole-DB snapshot/restore (covering
  the `optimize_attestation` side-table for free) is intact. ✓
- **Tier-B basis unchanged** — `VoidDecisionEvent` is excluded from `is_revocable_payload` (`form.rs:892`
  doc + 896-911 body), so a bulk-void is non-revocable ⟹ Tier-B (non-revocable warning, not typed-word) is the
  correct ceremony; the round-1 adjudication stands. ✓

---

## Findings

### [N1] NIT — bare `main.rs` in the M3 cite is crate-ambiguous
The fold's new §CLI cite `main.rs:1267-1268` refers to `crates/btctax-cli/src/main.rs` (the CLI dispatch),
while the surrounding §Candidate-set/§Per-row cites (`main.rs:2733-2770`, `2749`, `2641`, `2764`) refer to
`crates/btctax-tui-edit/src/main.rs`. There are three `main.rs` in the workspace (tui-edit, cli, tui). The
context disambiguates and every line range is unambiguous once resolved, so this is cosmetic — but round-1's
M2 set a precedent of disambiguating same-named files (`transition.rs`). A one-word crate prefix on the CLI
cite (e.g. "`btctax-cli/src/main.rs:1267-1268`") would apply the same courtesy. Non-blocking; fix at leisure
or during Task 2.

---

## Bottom line
**R0-GREEN (0C / 0I).** All three Minors (M1/M2/M3) and all three Nits (N1/N2/N3) from round 1 are folded
correctly and verified against current source; the tax-safety core (#7 via the shared `!effective_alloc`
predicate, Hard year-gating `DecisionConflict`, inert-void-applies-cleanly KAT) re-verifies end-to-end; the
persist atomicity and Tier-B decisions are unchanged and sound. No source changed since round 1. The single
remaining item is a cosmetic crate-disambiguation Nit. **Clear to proceed to implementation.**
