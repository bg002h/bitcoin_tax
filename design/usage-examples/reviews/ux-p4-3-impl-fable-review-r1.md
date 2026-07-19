# UX-P4-3 (#14) implementation review — r1 (Fable, independent/adversarial)

**Scope:** commit `990f786` (diff `666a868..HEAD`) — record-time reconcile-decision validation that
mirrors the resolver. Reviewed against SPEC §3.2 (`SPEC_post_v070_product_cycle.md:122-186`) and PLAN
Step-1c (`IMPLEMENTATION_PLAN_post_v070_product_cycle.md:83-92`), with the resolver
(`crates/btctax-core/src/project/resolve.rs`) read in full.

**Method (evidence, not assertion):** full source read of `would_conflict`
(`crates/btctax-core/src/project/mod.rs:88-156`), `guard_decision_conflict` + all six wired verbs
(`crates/btctax-cli/src/cmd/reconcile.rs`), all four resolver decision passes (1a–1e + R0-I1 overlap
guard + §7.4 allocation arms); enumeration of every `append_decision`/`append_and_save` call site in
`reconcile.rs`; baseline test run (20/20 green); a **mutation experiment** (guard call deleted from
`classify_raw` → full `btctax-cli` suite run); and a **throwaway behavioral probe** of the
accept-governed `SupersedeImport` channel (three directions), run and then deleted. Tree restored
byte-clean after both experiments (`git status` clean).

---

## Critical

None.

The definitional-equivalence claim (item 1) HOLDS. I attacked it from every direction I could
construct and found no false-accept or false-refuse relative to the resolver's own adjudication:

- **Candidate identity is exact.** `would_conflict` wraps the payload as
  `EventId::decision(max_seq+1)` with `utc=now`, `tz=UTC`, `wallet=None`
  (`project/mod.rs:132-147`); the real append path assigns the identical event —
  `SELECT COALESCE(MAX(decision_seq),0)+1` + `UtcOffset::UTC` + `wallet=None`
  (`persistence.rs:245-250`, `reconcile.rs:33`). The shadow with-candidate projection IS the
  post-append projection under pseudo-OFF, not an approximation of it.
- **First-wins/last-wins races resolve identically.** The candidate's max+1 seq iterates last in
  every decision pass, so it is always the LOSING side of a first-wins race (duplicate blocker blamed
  on the candidate's unique id → never in baseline → refused) and the WINNING side of ManualFmv's
  last-wins (no blocker → accepted) — exactly what the real append would produce
  (`resolve.rs:543-560/694-709/746-762/807-821` vs `:564-597`).
- **Key masking is impossible where it would matter.** The diff key is `(blocker.event, detail)`.
  Every decision-blamed `DecisionConflict` keys to the emitting decision's unique id, and the
  candidate's id (`decision(max+1)`) cannot exist in the baseline, so a candidate-blamed conflict is
  never masked by an equal baseline key. The non-candidate-keyed emitters — passthrough-overlap
  (`resolve.rs:914-937`, keyed to the passthrough decision), allocation-void irrevocability
  (`resolve.rs:1264-1272`, keyed to the void), and multiple-effective-allocations
  (`resolve.rs:1305-1312`, keyed `None`) — each emit at most once per projection with a fixed detail,
  so "same key already in baseline" implies "the blocker set is unchanged," i.e. the resolver itself
  adjudicates NO new conflict — accepting is then *correct* mirroring, not a miss. (Worked example: a
  `ClassifyInbound` on the in-leg of a passthrough already excluded for an out-leg overlap → the R0-I1
  guard still emits exactly one blocker with the same key → accepted; verify's blocker set is
  byte-identical pre/post append. Mirrors.)
- **Baseline-diff direction is right for third-party-keyed conflicts.** A candidate that flips an
  EXISTING decision's adjudication (e.g. a `ClassifyRaw` that rewrites a target out from under a live
  `ClassifyInbound`, or a classification that makes an inert allocation effective so an existing
  allocation-void becomes irrevocable-conflicted) produces a new key blamed on the existing decision —
  caught by the diff, refused, and verify WOULD have shown exactly that conflict. A candidate that
  *removes* a baseline conflict (e.g. voiding a conflicted duplicate) adds nothing new → accepted.
  A single candidate cannot simultaneously remove and re-create the same key (remove requires
  un-classifying, create requires classifying).
- **`now` is inert.** Decision timestamps feed adjudication only for `MethodElection` /
  `SafeHarborAllocation` made-dates (`resolve.rs:1080/1191`); none of the six guarded payloads is
  either, and decisions never enter the pass-2 timeline (`resolve.rs:1040-1042`). Determinism (NFR4):
  both `project` calls are pure; `load_all` order is irrelevant (decisions sort by seq).
- **Read-only (§1).** `would_conflict` clones into a local vec and calls the pure `project` twice; the
  CLI guard does `load_all` (a SELECT, `persistence.rs:264`) + `read_config` and refuses BEFORE any
  `append_decision`, including in `void` where the batched append (`reconcile.rs:292`) sits after both
  refusal gates. On refusal no `session.save()` runs — the vault is untouched (the updated
  `classify_inbound_self_transfer_cli.rs:272-311` test pins fail-closed). Committed goldens
  byte-identical (full CI green).
- **Pseudo-forced-OFF is the spec'd shadow and behaves as specified.** `cfg.pseudo_reconcile = false`
  (`project/mod.rs:119`) removes exactly the `pseudo_on`-gated writers (`resolve.rs:522/949`), keeping
  void→re-decide and first-real-classify-of-a-pseudo-default working (KATs at
  `record_time_validation.rs:287/302`) while honoring REAL accepted-conflict `SupersedeImport`
  payloads (`resolve.rs:513`) — verified empirically, see the probe under Important 1.

## Important

**I1 — Mandated `[R3-I1]`/`[G2-6]` acceptance KATs are missing, and the `classify_raw` guard call is
mutation-unwitnessed (proven survivor).**
`crates/btctax-cli/tests/record_time_validation.rs` (13 KATs) omits cases the SPEC §3.2 acceptance
list and PLAN Step-1c mandate by name ("KATs (spec §3.2 both directions), **incl. the accept-governed
`SupersedeImport` accept + `classify-raw` refuse cases**"):
  - accept: `set-fmv`/`reclassify-income` on a target whose Income type comes **from an accepted
    `SupersedeImport` conflict** `[R3-I1]` — absent;
  - refuse: **`classify-raw` on an accept-governed target** `[R3-I1]` — absent;
  - refuse: wrong-type ref, **accept-governed** arm `[R3-I1]` — absent (the wrong-type KATs cover
    only the raw-log arm; test :353 covers the *voided*-ClassifyRaw revert, whose raw type is also
    non-Income, so it cannot discriminate effective-vs-raw in the refuse direction);
  - refuse: `ClassifyRaw` first-wins duplicate `[G2-6, T2-M1]` — absent (duplicates are KAT'd for 3
    of the 4 first-wins verbs).
Consequence, **empirically proven**: I deleted `guard_decision_conflict(...)` from `classify_raw`
(`crates/btctax-cli/src/cmd/reconcile.rs:470`) and the ENTIRE `btctax-cli` suite stayed green
(432/432 passed) — the wiring of one of the six spec'd choke points (`reconcile.rs:301` in the SPEC's
own list) is protected by no test, violating the §3.2 "Mutation reds" clause (the commit message's
"both would_conflict directions mutation-proven" claim is false for this call site). These KATs are
the regression guard for exactly the one-writer-short failure class the `[R3-I1]` spec amendment
exists for.
*Mitigating (why this is Important, not Critical): the behavior itself is CORRECT. My throwaway probe
(ImportConflict minted via `append_import_batch` on a TransferIn, real `accept_conflict`, then the
three directions) passed: set-fmv on the accept-governed Income target ACCEPTED; classify-inbound on
it REFUSED wrong-type (raw log says TransferIn — a raw-log validator would false-accept);
classify-raw on it REFUSED "multiple classifications". The gap is the missing mandated tests, not a
wrong result.*
Fix: add the four KATs above (the probe in this review's history is a ready-made fixture recipe:
`append_import_batch` with the target's own id + an Income payload mints the conflict; the
`reconcile.rs:612` test shows the accept flow).

**I2 — The §3.2 "Unify the DecisionConflict remedy hints" mandate is unimplemented on the verify
surface, and the record-time message inherits the contradictory old hints.**
SPEC §3.2: "**Unify** the `DecisionConflict` remedy hints to one phrasing pointing at `events list`
(3.6) + 'void decision|N first'"; PLAN Step-1c repeats it. The FOLLOWUPS origin entry
(`FOLLOWUPS.md:2195-2205`) defines the problem as the *verify-time* hint text being inconsistent
across variants ("some carry 'void the decision to clear this blocker'; the void-of-unknown carries
none; the unknown-event ReclassifyIncome hint suggests the wrong verb for a typo"). As built:
`resolve.rs` is untouched — its 11 divergent hint sites remain
(`resolve.rs:587/606/687/718/739/772/801/815/832/863/879`), zero of them name `events list`, and
verify renders them raw (`render.rs:2293/2307`). Only the record-time wrapper
(`reconcile.rs:55-60`) adds the unified pointer — and it appends it AFTER the resolver's stale hint,
so the flagship refusal for the typo'd-ref case (the exact trap UX-P4-3 exists to fix) reads:
"cannot record this decision — ClassifyInbound targets unknown event X **— void the decision to clear
this blocker**. Run `btctax events list` …; `btctax reconcile void decision|N` to change an existing
decision." — instructing the user to void a decision that was never recorded (and for `set-fmv`
wrong-type, `resolve.rs:606`'s "void this decision" likewise). The commit message silently narrows
the mandate to "record-time surface"; no deferral is filed in `FOLLOWUPS.md` and no spec amendment
covers the narrowing. A mandated spec bullet left partially implemented without documented cover is a
missing case (and this project's standing rule is that a deferral needs the mandating section's
blessing, not silence).
Fix: either (a) rewrite the resolver's `DecisionConflict` details to the one phrasing (and make the
record-time wrapper's suffix the single hint source), or (b) amend SPEC §3.2 with the author's
narrowed reading and file the verify-surface unification as an owned follow-up — (a) is what the
bullet says.

## Minor

None.

## Nit

**N1 — Double `load_all` + payload-build on the `classify_inbound` path.** The UX-P4-4(b)
acquired-date guard loads all events (`reconcile.rs:95`) and `guard_decision_conflict` loads them
again (`reconcile.rs:52`); with the two `project()` calls this is 2 loads + 2 projections per
interactive append. Ledger sizes make this immaterial (the whole 432-test suite, projecting
constantly, runs in ~3s); noting only so a future bulk-wiring attempt doesn't copy the pattern into a
loop. Perf item 9 is otherwise a non-issue — no pathological case found (the safe-harbor
`universal_snapshot` work is the same cost `verify` already pays).

**N2 — Documented, spec-conforming residual under stored-pseudo-ON (observation, no action).** With
the stored config pseudo-ON and an *unresolved* `ImportConflict` on a target, a real `classify-raw`
on that target is accepted at record time (pseudo-OFF shadow: `applied` lacks the target) but the
next pseudo-ON `verify` shows "multiple classifications of one target" (the `:522` accept-first
occupies `applied` before pass 1c). This is exactly what SPEC §3.2 mandates (`[T2-I1, R3-I1]`:
pseudo-gated inserts "absent under pseudo-OFF") — the record-then-conflict trap survives only inside
pseudo mode's advisory fiction, which never gates. Recording that it was examined and is intended.

**N3 — Spec-scope observation (no action).** Other single-verb appenders with user-typed refs that
can raise `DecisionConflict` at verify (`link_transfer` duplicate-TransferLink `resolve.rs:637-646`,
`select_lots` duplicate-LotSelection `resolve.rs:1127-1134`) are outside the SPEC's six-fn choke list
and correctly left unwired; noting so a later cycle can decide whether they deserve the same
treatment.

## Verdict

**NOT GREEN — 0 Critical / 2 Important (I1, I2) block.**

The core construction is exactly right: `would_conflict` is definitionally the resolver (identical
candidate identity, seq, config; baseline-diff over `(event, detail)` with no maskable divergence I
could construct or probe), pseudo-OFF is the correct and spec'd shadow, the six choke points are
wired fail-closed before any append, bulk `apply_*` paths are correctly untouched, set-fmv's
duplicate-exemption and void's non-revocable/already-voided refusals fall out of the resolver rather
than hand-coding, and §1/NFR4 hold (read-only, deterministic, goldens byte-identical, full CI green).
What blocks is (I1) the spec-mandated `[R3-I1]`/`[G2-6]` KATs that are absent — leaving the
`classify_raw` guard wiring a *proven* mutation survivor against a spec whose acceptance clause is
"Mutation reds" — and (I2) the "unify the remedy hints" bullet left unimplemented on the verify
surface, with the stale contradictory hints now embedded verbatim in the record-time refusals.
Both are contained, test-and-message-level fixes; neither requires touching `would_conflict`.
