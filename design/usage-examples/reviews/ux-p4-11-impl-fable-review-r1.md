# UX-P4-11 `btctax events list` — implementation review r1 (Fable, independent/adversarial)

**Artifact:** commit `8ddeb46` (`git diff b2a9f7d..HEAD`) — new read-only subcommand `events list`
(spec §3.6, plan Step-1a).
**Reviewer basis:** whole-diff read + current-tree source verification + live-binary probes
(link-transfer/void/conflict vaults) + a mutation run (voided-filter disabled → suite re-run).
CI surface was reported green by the requester (make check 2004 + clippy/fmt/msrv/pii/isolation);
I independently re-ran the 5 new tests.

**Verified-good up front (claims I checked, not assumed):**

- **Read-only / §1:** `events_list` (`crates/btctax-cli/src/cmd/inspect.rs:17-113`) opens the
  session, reads `load_all` / `load_all_ordered` / `session.prices()`, and never calls
  `session.save()` or any append; `main.rs:120-122` only prints. No projection/tax code is touched
  anywhere in the diff. §1 intact.
- **NFR4 determinism / pairing:** the join key is guaranteed — the single INSERT site
  (`crates/btctax-core/src/persistence.rs:153-157`) stores `ev.id.canonical()` in the UNIQUE
  `event_id` column, so `by_id.get(&raw.event_id)` (`inspect.rs:85`) can never silently drop a row;
  `load_all_ordered` is `ORDER BY ordinal` (`persistence.rs:351-357`) — deterministic for a given
  vault. "Insertion order = event sequence" is a sound reading of §3.6's "stable ordering (by event
  sequence)".
- **Pseudo-decidable "by construction" claim: TRUE.** `resolve.rs:411-418` (note `[R0-I1]`):
  synthetics are map-layer entries, "we NEVER mint `EventId::Decision{seq}`", never persisted; only
  `pseudo approve` persists real decisions (which then legitimately list as decided). The KAT
  (`tests/events_list.rs:149-188`) additionally proves no `ClassifyInbound` was persisted.
- **No false inclusions in the universe:** every listed kind is genuinely decidable — TransferIn
  (classify-inbound-*, link-transfer `--to-event`, passthrough), TransferOut (reclassify-outflow,
  link-transfer, passthrough), Unclassified (classify-raw), ImportConflict (accept/reject-conflict),
  Income (set-fmv — Income-only per `resolve.rs:592-596` — and reclassify-income). Acquire is never
  a decision target (verified against every decision payload's target field in `event.rs`).
- **The paste KAT is genuinely end-to-end** (`tests/events_list.rs:103-146`): real binary via
  `CARGO_BIN_EXE_btctax`, ref scraped as the first whitespace token of the printed row, fed verbatim
  to `reconcile reclassify-outflow`, exit 0 asserted with stderr on failure. It reds if the rendered
  ref is decorated/truncated or the ref-first row contract breaks (the render test at
  `render.rs:3668-3695` pins the same contract from the display side).
- Man pages (`docs/man/btctax-events.1`, `docs/man/btctax-events-list.1`, `btctax.1` index) +
  `examples.md` help-census regenerated; help text's verb citations (`classify-inbound-self-transfer`,
  `reclassify-outflow --as-kind sell --amount`, `reconcile void decision|N`) all name real verbs
  (`cli.rs:547`, `cli.rs:580-581`; the reclassify invocation is executed by the KAT).

## Critical

None.

## Important

- **I1 — The reverse-map omits the `TransferLink` in-leg: a matched-pair in-leg lists as
  `[decidable]` in a healthy vault though the engine has decided it.**
  `crates/btctax-cli/src/cmd/inspect.rs:72-74` maps only `d.out_event`; but a
  `TransferLink { in_event_or_wallet: InEvent(in_id) }` — created by
  `reconcile link-transfer <out> --to-event <in>` (`cli.rs:516-522`,
  `cmd/reconcile.rs:817-831`) and by `match-self-transfers`' RELOCATE case
  (`cmd/reconcile.rs:846`) — **consumes the in-event**: `resolve.rs:666`
  (`consumed_ins.insert`), and its Op becomes `Skip` (`resolve.rs:355-357`, "Consumed by a
  TransferLink"). Reproduced live: after `link-transfer --to-event`, `verify` reports
  0 pending / 0 blockers (fully reconciled), yet `events list` prints the in-leg
  `[decidable]` and the header counts it "open". The induced trap is exactly the one UX-P4-11
  exists to close: following the affordance, `classify-inbound-self-transfer <in-ref>` is
  **accepted (decision|2 recorded) but inert** — the resolver deliberately type-passes a consumed
  TransferIn (`resolve.rs:694-698`, D1 precedence) and `build_op` lets the link win — and the row
  then shows `[decided: decision|2]`, so the documented remedy ("void decision|N first") points at
  the inert decision, not the governing link. Spec §3.6: "include decided rows with their decision
  ref" — wrong for this whole reachable class. The code already treats the two-leg analogue
  correctly (`SelfTransferPassthrough` maps BOTH legs, `inspect.rs:75-78`); the `TransferLink` arm
  is the inconsistent one. **Fix:** in the `TransferLink` arm also insert
  `TransferTarget::InEvent(in_id) → e.id`, + a KAT (link a pair via `--to-event`, assert the in-leg
  row carries the link's decision ref).

- **I2 — The voided-decision handling is untested: disabling the `voided` filter passes the whole
  suite (mutation survives).**
  I mutated `inspect.rs:47` (`if voided.contains(&e.id)` → `if false && …`) and re-ran: all 5 new
  tests PASS (2 render + 3 integration), and no other test in the workspace touches `events_list`
  (grep: only `tests/events_list.rs`, `main.rs`, `render.rs`, `inspect.rs`). The void→re-decide loop
  is the feature's own documented remedy flow (`cli.rs:508-509`: "void decision|N first, then
  re-decide"), and the standing review invariant is mutation-proof KATs — a load-bearing branch of
  the new logic reds nothing when broken. **Fix:** extend the first KAT: void the recorded decision
  → the row returns to `[decidable]` (and the header count reflects it); re-decide → the NEW
  decision ref is shown (this also pins the "void→re-decide leaves only the survivor" claim of the
  comment at `inspect.rs:43-44`, currently also unwitnessed).

## Minor

- **M1 — Void-of-a-non-revocable decision diverges from the resolver** (owning step: 1c).
  The resolver keeps `SupersedeImport`/`RejectImport` **in force** when a void targets them — the
  void is inert and itself raises the DecisionConflict (`resolve.rs:424-443`) — but
  `events_list`'s voided-set (`inspect.rs:33-40`) honors every `VoidDecisionEvent`
  unconditionally. Reproduced live: accept-conflict → `[decided: decision|1]`; `void decision|1` →
  `verify` says "void targets a non-revocable decision" (supersede still in force), yet
  `events list` flips the conflict row back to `[decidable]` — inviting a duplicate supersede
  (itself a further conflict, `resolve.rs:806-812`). Mitigations that keep this out of the gate: the
  state only exists in a vault already hard-blocked by that very void, verify names the offender,
  and Step-1c's record-time `void` refusal ("void refuses non-revocable/already-voided", plan :86-88)
  makes it unreachable going forward. Burn down with 1c: either mirror the resolver's revocability
  rule here or derive decided-status from resolver outputs.
- **M2 — `Dispose` is excluded from the row universe though `reconcile select-lots` acts on a
  Dispose** (`LotSelection.disposal_event`, `event.rs:292-293`). The literal spec text
  ("Row universe `[G-M3]`: every decidable event") and the command's own help definition
  ("the imported rows a `reconcile` verb can act on") both read as including it; the five-kind pin
  is implementation-local (spec + plan never enumerate). The exclusion is defensible — select-lots
  refs have a first-class documented channel (disposals.csv `event` column, `cli.rs:626-630`), the
  spec's KATs and the §3.2 refuse-hint context are all classification-flow, and nothing is
  *mislabeled* (rows are absent, not wrong) — but the deviation should be RECORDED: either a
  one-line spec clarification (universe = the reconciliation-classification surface) or Dispose
  rows with LotSelection reverse-mapping. Note the 1c unified DecisionConflict hint will name
  `events list` for LotSelection conflicts too — decide coherence there at latest. Also soften the
  overstated doc-comment `inspect.rs:10-12` ("no reconcile verb retargets it" — select-lots does
  target a Dispose, even if it doesn't reclassify it).
- **M3 — The Income amount column ignores a live `ManualFmv` override (and `fmv_status`).**
  `inspect.rs:98-103` shows the imported `usd_fmv` (else close price); the resolver's effective FMV
  is the ManualFmv override when present (`resolve.rs:287-289`, latest-wins per `resolve.rs:594-596`).
  So the one row a set-fmv user just corrected displays the pre-correction figure next to
  `[decided: <that ManualFmv>]`. The `~$` marker softens it (the column is explicitly indicative for
  every other kind), hence Minor: prefer the override when one is live.

## Nit

- **N1 —** `fmt_btc` (`render.rs:3573-3577`) drops the sign for sat ∈ (−10^8, 0): `whole` truncates
  to 0 and `unsigned_abs` erases the sign → `-0.5` renders "0.50000000". Unreachable for persisted
  payloads (adapters `.abs()` before building them — `parse.rs:54` contract; river/swan/gemini
  verified), so display-robustness only. No panic path (i64::MIN is why `unsigned_abs` is right).
- **N2 —** In an already-blocked vault the `decided` map is later-wins (`inspect.rs:43-44`) while
  the resolver is first-wins for e.g. ClassifyInbound (`resolve.rs:700-707`), and resolver-EXCLUDED
  (type-invalid) decisions still insert. In practice the shown ref is then usually the *correct*
  void target (the excluded duplicate is what must be voided), so this is note-level; it dissolves
  if I1's fix moves toward resolver-derived status.
- **N3 —** Rows print in ledger (ordinal) order, which can interleave calendar dates (observed:
  a June send before a March receive when the CSV ordered them so). Sound per §3.6, but the
  user-facing help never states the ordering; one clause in the clap doc-comment would prevent a
  "why isn't this chronological" report.

## Verdict

**NOT GREEN — 0 Critical / 2 Important.** What blocks:

1. **I1:** map the `TransferLink` `InEvent` in-leg in the reverse-map (mirror the
   `SelfTransferPassthrough` both-legs treatment) + a KAT for the linked in-leg's decided status.
2. **I2:** a void→re-decide KAT that reds when the `voided` filter is broken (the mutation I ran
   must die).

Both are small, localized fixes in `inspect.rs` + `tests/events_list.rs`. Everything else verified
clean: read-only/§1 confirmed, NFR4 ordering/pairing sound, the pseudo-decidable claim is true by
construction against `resolve.rs`, the paste KAT is genuinely end-to-end, no false inclusions in the
universe, docs/man/golden regenerated. M1 is owned by Step-1c; M2 needs a recorded spec
clarification (or Dispose inclusion) but does not gate this step; M3/N* are recorded. Re-review
after the fold.
