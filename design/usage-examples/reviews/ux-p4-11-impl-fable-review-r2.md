# UX-P4-11 `btctax events list` — implementation review r2 (Fable, independent/adversarial, closing gate)

**Artifact:** the whole #18 diff `b2a9f7d..HEAD` (HEAD = `f338e6b`), i.e. base `8ddeb46` + r1 persist
`4b7cbb1` + fold `f338e6b`. Scope of this round: verify the two r1 Important folds close their
findings, hunt fold-introduced drift, and rule on the whole diff.
**Reviewer basis:** whole-diff re-read (saved diff byte-identical to `git diff b2a9f7d..HEAD` —
verified), current-tree source verification (`inspect.rs`, `events_list.rs`, `cli.rs`, `event.rs`,
`resolve.rs`, SPEC §3.6, FOLLOWUPS), an independent full `make check` run, `cargo fmt --check`, BOTH
mutations re-run live (in-leg drop + voided-filter disable), and a live-binary probe of the exact I1
fixture (init → import → `link-transfer --to-event` → `events list` + `verify`).

**Fold verification (each r1 Important, attacked not assumed):**

- **I1 CLOSED — and closed correctly.**
  - *The fix is the right shape.* `TransferTarget` has exactly two variants
    (`crates/btctax-core/src/event.rs:98-101`): `InEvent` and `Wallet`. The resolver consumes an
    in-event **only** under the `InEvent` arm (`crates/btctax-core/src/project/resolve.rs:645-666`,
    `consumed_ins.insert(in_id)`); a `--to-wallet` link consumes nothing. The fold's guard
    (`crates/btctax-cli/src/cmd/inspect.rs:81-83`, `if let TransferTarget::InEvent(in_id)`) matches
    that consumption condition exactly — no over-marking of `Wallet` links, no missed in-leg case.
    It now mirrors the `SelfTransferPassthrough` both-legs treatment (`inspect.rs:85-88`), which r1
    named as the consistent model.
  - *The mutation dies.* I removed the in-leg insert and re-ran: `events_list_transfer_link_decides_both_legs`
    **FAILED** at `tests/events_list.rs:149-153` ("the linked IN-leg must be decided (not decidable);
    got None") while the other 4 tests passed — the KAT reds on precisely the folded line, so it is
    exercising the reverse-map itself, not failing for a side reason. Restored; all 5 green again.
  - *The KAT's link genuinely reaches the projection healthily.* Live probe of the KAT's exact
    fixture (Buy 0.05 / Send 0.05 / Receive 0.05, `link-transfer 'import|coinbase|out|cb-send'
    --to-event 'import|coinbase|in|cb-recv'`): `events list` prints BOTH legs
    `[decided: decision|1]`, header "2 decided, 0 open", and `verify` exits 0 — conservation
    BALANCED, 0 pending, 0 hard blockers. So the fold marks the in-leg decided exactly in the
    healthy-vault case where the resolver has in fact consumed it — r1's reproduced trap
    (fully-reconciled vault, in-leg `[decidable]`, remedy pointing at an inert re-decision) is gone.
  - *The KAT pins the same-ref contract* (`tests/events_list.rs:158-162`): in-leg ref == out-leg
    ref == the link's decision — the "void decision|N" remedy now names the governing link.

- **I2 CLOSED — r1's exact surviving mutation now dies.**
  - I re-applied r1's mutation verbatim (`inspect.rs:51`, `if voided.contains(&e.id)` →
    `if false && voided.contains(&e.id)`) and re-ran: `events_list_void_returns_to_decidable_then_redecide`
    **FAILED** at `tests/events_list.rs:207-209` ("a voided decision must return the row to
    decidable; got Some(\"decision|1\")"). Restored; green.
  - *The killing assertion is the right one.* The KAT's middle phase (void → `decision_ref.is_none()`,
    `tests/events_list.rs:203-210`) is what kills the filter mutation; the final phase pins the
    survivor claim of the comment at `inspect.rs:46-48` — `assert_eq!` on d2's canonical ref
    (`tests/events_list.rs:226-230`, "the survivor, not the voided d1") plus an explicit
    `assert_ne!` against d1 (`tests/events_list.rs:232`). Note the survivor phase alone would NOT
    kill the mutation (later-wins would mask it) — the KAT's three-phase structure is what makes it
    mutation-proof, and it has it.

**Re-scan for sibling gaps (r2 attack #3):** with the fold in, I re-enumerated every decision
payload's event-target field in `event.rs` against the reverse-map (`inspect.rs:54-90`):
`TransferLink.out_event` + `InEvent` in-leg ✓ (both, post-fold), `ReclassifyOutflow.transfer_out_event` ✓,
`ClassifyInbound.transfer_in_event` ✓, `ManualFmv.event` ✓ (Income-only per resolver),
`ReclassifyIncome.income_event` ✓, `ClassifyRaw.target` ✓, `SupersedeImport.conflict_event` ✓,
`RejectImport.conflict_event` ✓, `SelfTransferPassthrough.{in,out}_event` ✓,
`VoidDecisionEvent.target_event_id` → the voided set ✓. The only decision variants NOT in the map:
`SafeHarborAllocation` and `MethodElection` (no per-event target — nothing to mislabel) and
`LotSelection.disposal_event` (`event.rs:292-295`), which targets a `Dispose` — a kind outside the
row universe by the now-recorded M2 decision (rows absent, not mislabeled). **No decision variant
that targets a listed kind is missing from the reverse-map.**

**No fold drift (r2 attack #4):** `TransferTarget` import in `inspect.rs:24` used at :81; test-side
`eventref`/`TransferTarget` imports (`tests/events_list.rs:9-10`) used at :138/:141; `Session` still
used (:315, the pseudo test). No existing test weakened — the fold only ADDS two tests (base file
unchanged above line 97 except the import line). Independently re-ran the full gate: `make check`
GREEN (2006/2006 passed, 8 skipped, no clippy failure line) including the three xtask examples
goldens — so the help change introduced no census staleness (verified directly: the census
`docs/examples/examples.md:39` carries only the parent `events` summary, which the fold did not
touch; both man pages `docs/man/btctax-events-list.1:5,9` + `docs/man/btctax-events.1:17` carry the
new ledger-order sentence — regenerated). `cargo fmt --check` clean. CI-only surfaces
(msrv/pii-scan/net-isolation) not re-run here: the fold adds no dependency, no PII surface, no
network code (one match-arm + tests + docs), and the author reports them green; noted, not assumed
as part of my own evidence.

**§1 / read-only (r2 attack #5):** intact. The fold's only product-code change is inside the
`events_list` reverse-map (pure in-memory); `events_list` still opens the session, reads
`load_all`/`load_all_ordered`/`prices`, and never appends or saves. Tests/docs/SPEC/FOLLOWUPS
otherwise.

**Spec/code agreement (r2 attack #6):** the §3.6 as-built amendment
(`design/usage-examples/SPEC_post_v070_product_cycle.md:261-265`) matches the code exactly — the
five kinds are the five arms of `inspect.rs:98-105`; "a `Dispose`'s only decision is specific-ID
`select-lots`" is true (`LotSelection` is the sole Dispose-targeting decision payload in
`event.rs`); "refs come from the `disposals.csv` `event` column" is the documented channel
(`cli.rs:627`). The softened doc-comment (`inspect.rs:12-20`) no longer overstates ("no reconcile
verb retargets it" is now said only of `Acquire`, which is correct). The `List` help states
ledger/import order (`cli.rs:503-504`) — N3 folded. FOLLOWUPS entries filed with ownership
(`FOLLOWUPS.md:2352-2374`; M1 explicitly owned by Step-1c/#14 per the in-phase burndown rule).

## Critical

None.

## Important

None. Both r1 Importants are closed, mutation-proven, and live-probed (evidence above).

## Minor

None new.

## Nit

- **N1(r2) — FOLLOWUPS N2's wording should name the TransferLink in-leg now in the same class.**
  `inspect.rs:81-83` inserts the in-leg unconditionally on link *validity*, but the resolver refuses
  to consume when a second link names the same in-event (first-wins + DecisionConflict,
  `resolve.rs:646-654`) or when the in-event has no resolvable destination wallet (link inert +
  DecisionConflict, `resolve.rs:654-664`) — in those vaults `events list` shows the in-leg
  `[decided]` though the engine did not consume it. This is exactly the already-filed
  M1/N2 blocked-vault-divergence class (`FOLLOWUPS.md:2360-2366,2371-2374`) with the same
  mitigations (only reachable in an already-hard-blocked vault; `verify` names the offending link;
  dissolves if decided-status ever becomes resolver-derived), but the N2 entry's text names only the
  ClassifyInbound first-wins case. One clause extending N2 to "and the TransferLink in-leg
  (duplicate-in-event / inert no-wallet link)" would keep the 1c burndown's grep complete.
  Note-level; not a defect in the fold (the healthy-vault behavior — the thing #18 ships — is
  correct and proven).

## Verdict

**GREEN — 0 Critical / 0 Important.** The whole #18 diff (`b2a9f7d..f338e6b`) passes:

- **I1 closed:** the `TransferLink` in-leg guard matches the resolver's consumption condition
  exactly (`InEvent` only), the drop-the-insert mutation reds the new KAT at the folded line, and
  the live probe shows the fixed display in a verify-clean vault.
- **I2 closed:** r1's exact surviving mutation (voided-filter disable) now reds the new
  void→re-decide KAT, whose three-phase structure also pins the survivor claim.
- No sibling reverse-map gap remains (full decision-payload enumeration above), no fold drift
  (suite independently GREEN 2006/2006 + clippy + fmt; goldens/census/man consistent), §1 read-only
  intact, and the §3.6 amendment agrees with the code.

One Nit recorded (extend FOLLOWUPS N2's wording to the link in-leg); nothing gates. UX-P4-11 (#18)
is done to the standard — proceed.
