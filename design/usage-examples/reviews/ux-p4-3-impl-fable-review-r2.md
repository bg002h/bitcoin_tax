# UX-P4-3 (#14) implementation review — r2 (Fable, independent/adversarial, closing gate)

**Scope:** the r1 folds — commits `e223fc5` (persist r1) + `81a63cd` (fold I1+I2) on top of `990f786`;
whole-#14 diff `666a868..HEAD`. Task: verify the two r1 Importants are closed with no drift, and rule
GREEN or not. r1 already established the `would_conflict` construction is definitionally correct; that
was sanity-checked, not re-derived.

**Method (evidence, not assertion):** full read of the fold diff + current
`crates/btctax-core/src/project/resolve.rs` (all `DecisionConflict` emitters enumerated, rewritten and
untouched), `crates/btctax-cli/src/cmd/reconcile.rs` (wrapper + all six guard sites),
`crates/btctax-cli/tests/record_time_validation.rs` (15 KATs, file byte-identical to the fold commit).
Baseline `make check` run myself: **2021/2021 pass** (incl. the xtask examples goldens, the forms
golden packet, and the oracle smoke) + clippy; `cargo fmt --all --check` clean. **Mutation A** (delete
the `classify_raw` guard, `reconcile.rs:472`) → `duplicate_classify_raw_is_refused` REDS, 14/15 —
r1's proven survivor is dead. **Mutation B** (delete the `set_fmv` guard, `reconcile.rs:225`) →
`set_fmv_on_non_income_is_refused` REDS, 14/15. **Throwaway probe** (accept-governed fixture →
`classify_raw` on the target; then follow the hint and void the `SupersedeImport`) — run and deleted.
**Mutation C** (one-writer-short pass-1c duplicate check — see I1-r2) → **survived the entire
2021-test workspace suite**. Tree restored byte-clean after every experiment via cp-backup/restore
(`cmp` + `git status` clean; never `git checkout`).

---

## Critical

None. Fold-drift sweep (all clean):

- **No logic drift.** The fold is strings + tests + FOLLOWUPS only: `resolve.rs` changes touch only
  `detail:` string expressions plus the `CONFLICT_HINT` const (`resolve.rs:417`) — blocker kinds,
  blamed events, and control flow byte-identical; `reconcile.rs` changes only the wrapper's message
  (suffix removed, `reconcile.rs:59-61`); the test file gained 80 inserted lines, zero modified — no
  pre-existing KAT weakened. `would_conflict` (`project/mod.rs`) untouched since r1. §1 read-only and
  fail-closed wiring therefore stand as r1 verified them.
- **Detail rewrite is value-inert for the validator by construction.** `would_conflict` diffs
  `(event, detail)` pairs produced by the SAME resolver on both sides of the baseline diff, so
  changing detail text cannot change accept/refuse behavior — and empirically, 2021/2021 including
  every golden (examples journeys, forms packet, oracle) passed with no golden regenerated.
- **No stale hint text survives in source.** Grep for every old phrasing ("void the decision to clear
  this blocker", "to re-decide, void the prior decision first", "multiple classifications of one
  target", "duplicate ClassifyInbound/ReclassifyOutflow for the same …") hits only design-history
  documents and the explanatory comment at `resolve.rs:415`. No test, golden, or TUI code
  string-matches any resolver detail (the TUI's "non-revocable" strings are its own modal framings).
- **`CONFLICT_HINT` usage correct.** Declared in `resolve()`'s body, interpolated inline in all 16
  rewritten `format!` sites (grep-enumerated); compiles, clippy/fmt clean.
- **Surface-neutrality of every rewritten detail — walked both surfaces.** At RECORD time no refusal
  any longer instructs voiding a decision that was never recorded: non-duplicate arms carry only the
  discovery pointer; duplicate arms add "void the prior decision to re-decide", and at record time a
  prior decision genuinely exists (one accept-governed edge → M1 below). At VERIFY time every
  rewritten detail reads correctly for a recorded decision (the list shows its `decision|N`; voiding
  the prior makes the recorded second effective — a valid remedy). No load-bearing information was
  dropped: the corrective-verb hints survive (`classify-inbound-income` on the ManualFmv non-Income
  and ReclassifyIncome arms, `resolve.rs:620/816/848`; `reclassify-income` on the ReclassifyOutflow
  wrong-type arm, `:788`), and the duplicate + wrong-type arms *gained* the target ref / a reason
  clause they lacked. On the SPEC letter "one phrasing pointing at `events list` + 'void decision|N
  first'": the two components cannot both be unconditional on a shared surface (an unconditional
  void-first is exactly the record-time contradiction I2 gated on), so scoping the void component to
  the duplicate arms is the correct reading — noted here as reviewed-and-endorsed cover.

## Important

**I1-r2 — The SPEC-named refuse KAT "`classify-raw` on an accept-governed target `[R3-I1]`" is still
absent, and the behavior it pins has a PROVEN surviving mutant.**
r1's I1 fix was "add the four KATs above"; the fold added coverage for three — `[G2-6]` classify-raw
first-wins duplicate (`record_time_validation.rs:340`, now mutation-pinning the `classify_raw` guard:
Mutation A reds it), the accept-governed set-fmv ACCEPT, and the accept-governed wrong-type REFUSE
(both in `record_time_validation.rs:376`) — but not the fourth: SPEC §3.2's acceptance list names,
in bold, refuse "**`classify-raw` on an accept-governed target** `[R3-I1]`"
(`SPEC_post_v070_product_cycle.md:180-182`), the §3.2 body names it as the r3 draft's miss (ii)
(`applied.contains_key`, now `resolve.rs:562`), and PLAN Step-1c repeats "the accept-governed
`SupersedeImport` accept **+ `classify-raw` refuse** cases". The existing accept-governed KAT never
calls `classify_raw` — it witnesses the `SupersedeImport→applied` writer only through the pass-1d/1e
type-read path (`.unwrap_or(&raw.payload)`), not through pass-1c's duplicate check. No core test
covers the pass-1c duplicate arm in ANY configuration (grep across `btctax-core/tests/`), so the
accept-governed configuration is unpinned workspace-wide. **Empirically proven (Mutation C):** I
rebuilt pass 1c one writer short — duplicate check against a `raw_classified` set fed only by
ClassifyRaw, ignoring `SupersedeImport`-written `applied` entries (the exact drift shape `[R3-I1]`
exists to prevent) — and the **entire 2021-test workspace suite stayed green**, while under the mutant
(a) record time silently ACCEPTS a `classify-raw` on an accepted-conflict target and (b) the
ClassifyRaw payload then **overwrites the accepted conflict resolution** in `applied` with no blocker
at verify — the user's permanent accept silently loses, wrong effective payload downstream. The
missing KAT is exactly the test that kills this mutant (in both directions: the record-time refusal
and, via the shared resolver, the verify-surface blocker). Behavior at HEAD is correct — my probe got
`cannot record this decision — duplicate classify-raw: import|coinbase|in|cb-recv is already
classified — see `btctax events list` …; void the prior decision to re-decide` — the gap is the
mandated test, the same class r1's I1 gated on.
Fix (~4 lines): at the end of `accept_governed_supersede_import_income_is_effective_income`
(`record_time_validation.rs:376`), `classify_raw` the same `in_ref` with the probe's JSON → assert
`unwrap_err()` contains "already classified" + `count(ClassifyRaw) == 0`.

## Minor

**M1 — The duplicate hint recommends an impossible remedy when the prior occupant is an accepted
`SupersedeImport`.** `resolve.rs:568`'s "void the prior decision to re-decide" is correct whenever the
occupant is a ClassifyRaw (the common case), but on an accept-governed target the "prior decision" is
the non-revocable `SupersedeImport`: my probe followed the hint and got `void targets a non-revocable
decision (accept/reject-conflict and void are permanent)` — a dead end, and on this arm there
genuinely IS no re-decide path (accepts are permanent by design). Only the classify-raw duplicate arm
can blame a non-revocable prior (`applied` is the sole two-writer map; the other duplicate arms' maps
are written only by revocable decisions). Non-gating: the follow-up refusal states the permanence, so
the user is informed, not misled about facts — but it is the same *class* of hint defect I2 gated on,
one arm, one edge. Suggest fixing alongside I1-r2's KAT (which will pin whatever text is chosen),
e.g. "…; if the prior decision is revocable, void it to re-decide".

## Nit

**N1 — `cli.rs:643` help (and the generated `btctax-reconcile-reclassify-income.1` man page) still
describe the record-then-surface flow** ("The engine validates that the target … fires a Hard
DecisionConflict blocker (decision excluded)… DecisionConflict is Hard — to re-decide, `void` the
prior decision first, then re-issue"). True of the engine, but the CLI verb now refuses at record
time. Docs/man regeneration was not in §3.2's mandate; fold into the cycle's doc pass.

**N2 — Two near-identical pointer phrasings.** The hand-written already-voided refusal
(`reconcile.rs:284-288`: "Run `btctax events list` to see event refs + their decision status.") vs
`CONFLICT_HINT` ("see `btctax events list` for event refs + decision status"). Cosmetic; the mandate's
"one phrasing" is materially met (both name the same command).

**N3 — The non-§3.2 `DecisionConflict` emitters keep bare details** (TransferLink arms
`resolve.rs:653/663/672`, R0-I1 overlap `:921` — which still says "void the conflicting decision" —
LotSelection `:1142`, allocation arms `:1283/:1316`). Consistent with r1 N3's scoping (outside the
six-verb subject of §3.2); recorded so the later cycle N3 anticipates can sweep hints and wiring
together. No action now.

## Verdict

**NOT GREEN — 0 Critical / 1 Important (I1-r2) blocks.**

The folds are high quality and almost complete. I2 is **fully closed**: all sixteen six-verb-family
`DecisionConflict` details are unified at the source behind one surface-neutral `CONFLICT_HINT` naming
`events list`, the duplicate arms carry the void-first remedy where it is valid on both surfaces, the
record-time wrapper no longer double-hints, the flagship typo'd-ref refusal no longer tells the user
to void a nonexistent decision, no stale phrasing survives anywhere in source, and nothing broke —
2021/2021 with every golden byte-stable, fmt/clippy clean. I1 is **three-quarters closed with teeth**:
Mutations A and B prove the `classify_raw` and `set_fmv` guard wirings are now test-pinned (r1's exact
survivor now reds), and the accept-governed KAT genuinely mints and accepts a real `ImportConflict`
and witnesses the `SupersedeImport` `applied`-writer in both the accept and wrong-type-refuse
directions. What blocks is the fourth mandated KAT — SPEC §3.2's bolded refuse case "`classify-raw` on
an accept-governed target `[R3-I1]`" — whose absence leaves pass-1c's duplicate check against the
`SupersedeImport` writer unpinned workspace-wide: my one-writer-short mutant (the precise `[R3-I1]`
drift class, under which an accepted conflict resolution is silently overwritten) survives the entire
suite. The fix is a ~4-line extension of the existing accept-governed KAT; M1 rides along free.
