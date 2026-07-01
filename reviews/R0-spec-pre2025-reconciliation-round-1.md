# R0 architect review — SPEC_pre2025_method_reconciliation (round 1)

- **Artifact:** `design/SPEC_pre2025_method_reconciliation.md`
- **Source verified against:** `origin/main` @ `c70922d` (== spec baseline; HEAD == origin/main, clean except the untracked spec). No citation drift from baseline.
- **Reviewer hat:** independent architect, adversarial pass against the §6 rubric.
- **Verdict: NOT GREEN — 0 Critical / 2 Important / 8 Minor / 1 Nit.** Two blocking findings (I1, I2) must be folded, then re-review.

---

## Premise verification (highest priority) — CONFIRMED ACCURATE

The spec's core premise — "the basis machinery is already complete and method-aware end-to-end; the only gap is the declaration's teeth" — is **true** against current source. I traced it end to end:

- `ProjectionConfig.pre2025_method` (`project/mod.rs:35`) → `applicable_method` returns it for `date < TRANSITION_DATE` (`fold.rs:30-32`) → `consume_principal` → `pools.consume` → `consume_ordered`/`method_order` (`pools.rs:179-268`). Setting LIFO/HIFO genuinely changes which Universal lots survive to 2025-01-01. ✔
- The boundary seed carries that residue forward — Path A relocate / Path B seed (`transition.rs:75-103`). ✔
- The Path-B allocation records the method immutably (`event.rs:162-168`; written at `reconcile.rs:290`), conservation is method-aware against the **recorded** method (`transition.rs:32-72` via `resolve.rs:668-675`), and `Pre2025MethodConflictsAllocation` (Hard) enforces live-vs-recorded sync (`resolve.rs:742-751`). ✔

The three claimed gaps are all real:
1. **`pre2025_method_attested` is engine-inert.** `to_projection` (`config.rs:30-35`) maps `self_transfer_fee` + `pre2025_method` only — it drops the attested flag; `ProjectionConfig` has no such field. The flag's only consumers are `build_verify` (`render.rs:488`, display) and the CLI config print (`main.rs:510`). The engine reads it nowhere. ✔
2. **`note_pre2025_once` is attestation-blind** — its signature is `(st, date, ev, method)` (`fold.rs:80`); it never sees the flag and always emits "verify against those filings." ✔
3. **`safe_harbor_allocate` records the live method without an attestation check** (`reconcile.rs:259,290`) — a never-declared user silently commits default FIFO into the irrevocable allocation. ✔

The spec neither under- nor over-states what exists, and correctly declines to rebuild the basis engine. Good.

---

## Blocking findings

### I1 (Important) — The plan omits updating the existing CLI tests that drive `safe_harbor_allocate`; the suite goes RED.

D3 gates `safe_harbor_allocate` on `pre2025_method_attested == true`. The default is `false` (`config.rs:23`). **Six existing tests** call `safe_harbor_allocate` and then `.unwrap()`, and **none of them attests the pre-2025 method first.** Under D3 every one returns `CliError` and the `.unwrap()` panics — these are *runtime* breaks (the call signature is unchanged, so the compiler won't catch them):

- `crates/btctax-cli/tests/reconcile.rs`
  - `safe_harbor_allocate_seeds_full_pre2025_residue_even_after_a_2025_disposal` (call @ `498`)
  - `safe_harbor_attest_cures_a_timebarred_allocation_excluding_voided_priors` (calls @ `552`, `561`)
  - `safe_harbor_attest_refuses_an_already_effective_allocation` (call @ `599`)
  - `safe_harbor_allocate_carries_gift_dual_basis` (call @ `684`)
- `crates/btctax-cli/tests/verify_report.rs`
  - test @ ~`90` (calls @ `112`, `125`)
  - test @ ~`195` (calls @ `216`, `227`)

(The core-crate tests in `safe_harbor_method.rs` / `transition.rs` build `SafeHarborAllocation` payloads directly and do NOT go through the CLI command, so they are unaffected — the gate is correctly contained to the user-facing command.)

The plan (Tasks 1–5) never includes this work. Task 5 only *narrates* "allocate now requires an attest step — call this out as the one behavior change," and Task 3 adds *new* KATs — but the green bar is "full validation suite passes" (§6), and following the plan literally produces a red suite with no task to fix it. A red suite is itself a blocking finding.

**Fix:** Add an explicit task (or fold into Task 3) to update each listed test: after `import`, before the first `safe_harbor_allocate`, call `cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true)` (attest FIFO — the default-method path D3's own KAT (c) describes). Enumerate the call sites above so none is missed. None of these tests asserts "unattested allocate succeeds," so the fix is uniform and does not change their intent.

### I2 (Important) — Autonomous decision (b) over-claims for Path-A filers: the attested method has NO immutable record; this cuts against the app's own "event log is the sole source of truth" posture.

Decision (b) justifies "no new ledger declaration event" by asserting "the config flag + attestation + the immutable `SafeHarborAllocation` (which records the method) together provide the record." That reasoning holds **only for Path B**. The legal default and the common case is **Path A**, where there is *no* `SafeHarborAllocation`. For a Path-A filer the attested method lives solely in `cli_config` (`config.rs:122-134`), which is:
- **mutable / overwritable** (`set_pre2025_method` upserts; no history),
- **explicitly not ledger state** — `config.rs:1-4` states "the event log remains the sole source of truth; this only selects a swappable rule" (NFR6),
- and under Path A the carryforward is **re-derived live under the current config on every projection**, with no `Pre2025MethodConflictsAllocation` to fire (that blocker needs an effective allocation). So a later silent `--set-pre2025-method` re-bases the 2025 carryforward with no audit trail and no record that the taxpayer ever attested the prior method.

So the spec's north-star — "recorded as their explicit representation" — is met for Path B but **not** for the dominant Path-A case, and the stated justification (the immutable allocation backs it) is factually inapplicable there. For an event-sourced app whose architecture treats config as a non-authoritative knob, a legally-significant taxpayer representation belongs in the append-only log, not a mutable side-table.

**Fix (either is acceptable; this is cheap to fold):**
- (a) Record the attestation durably in the event log (a minimal append-only declaration, or capture it on a Path-A marker), **or**
- (b) Keep config-only, but *correct the claim*: state plainly that for Path-A filers the attestation is recorded only in mutable `cli_config` and is NOT pinned immutably; justify why that is acceptable for this slug (Path-A carryforward is recomputed live — there is no irrevocable lock to protect — and `verify` surfaces the current attested state), and move "durable Path-A attestation" to `FOLLOWUPS.md` with rationale.

The author may not simply restate the current justification — it is unsound as written; clear it by fold or by a second independent reviewer's adjudication (§2).

---

## Minor

- **M1 — SemVer line contradicts the design.** Line 11 lists "**new Hard `BlockerKind` variant**" as part of the change, but the design adds none: D3 is a command-time `CliError` (chosen *over* a projection blocker), and D2 explicitly keeps `Pre2025MethodNote`. Also "`#[serde(default)]`-equivalent default" is loose — `ProjectionConfig` has no serde derives at all (`project/mod.rs:31`); it is a plain `Default`-impl field. Fix the summary to match D1–D3 (additive `ProjectionConfig` field via its manual `Default`; reuse of the existing advisory; a command-time refusal). The design body is correct; only the summary misleads.

- **M2 — Task 1 understates the literal-update surface.** Adding a field to `ProjectionConfig` (a plain struct, all-pub, manual `Default`) forces updates to: the `Default for ProjectionConfig` impl itself (`project/mod.rs:37-45`), and **~15 full struct literals** across `kat_tax.rs`, `method_election.rs`, `safe_harbor_method.rs`, `transition.rs`, `optimize_*.rs`, and `optimize.rs` (`#[cfg(test)]`). `transition.rs:40` uses `..*config` and is fine. These are compiler-enforced (low risk of silent miss), but Task 1's "any `ProjectionConfig` literal" should name the `Default` impl + the test files so the task is correctly sized.

- **M3 — D1's plumbing claim is imprecise.** "`fold.rs`'s context (`ctx.config`) now carries it, reaching `note_pre2025_once`" — but `note_pre2025_once` does **not** take `ctx`; its signature is `(st, date, ev, method: LotMethod)` (`fold.rs:80`) and it is called from three arms (`fold.rs:551`, `931`, `998`). The plan must change the signature (pass `ctx` or add an `attested: bool` param) and update all three call sites. State this in Task 2.

- **M4 — Task 2/Task 4 KAT fixtures must contain a pre-2025 disposition.** `note_pre2025_once` fires once on the first pre-2025 `Dispose`/`GiftOut`/`Donate` only. A pre-2025-acquisition-only vault emits no advisory, so "unattested → advisory contains 'have NOT declared'" and "attested → informational advisory" assertions would silently find no blocker. Spell out that the fixtures need a pre-2025 disposal.

- **M5 — Task 4 likely needs no `render.rs` code change.** `render_verify` prints all advisory blockers through the generic loop (`render.rs:982-990`); `Pre2025MethodNote`'s D2 text already flows through, and the attested/unattested text is consistent-by-construction with the existing "Pre-2025 method (attested historical fact) … (attested: N)" line (`render.rs:991-996`). Reframe Task 4 as a verification + KAT task rather than a `render.rs` edit (unless special-case formatting is actually wanted). Note the structured machine-readable signal the review asked about already exists: `VerifyReport.pre2025_method_attested` (`render.rs:488`) is a bool surfaced separately from the advisory text — so keeping the blocker text-only (D2) is the right call.

- **M6 — Out-of-scope decision (c) overstates current support.** "Direct pre-2025 ending-carryforward import … already covered by the Path-B safe-harbor allocation." The `SafeHarborAllocation` *payload* can carry arbitrary as-filed lots, but the `safe_harbor_allocate` *command* derives lots from a residue projection (`reconcile.rs:263-275`) — there is no command/flag to import user-supplied as-filed lot/basis. Clarify: the payload supports it; a direct-import UX does not exist today (and is correctly out of scope).

- **M7 — Pin the D3 gate's placement and the config source.** The gate must read the raw `CliConfig` (`session.config()?.pre2025_method_attested`), not `to_projection()` (which drops the flag) — the spec's D3 text is correct, but Task 3 should make it explicit. Also decide ordering relative to the existing empty-lots refusal (`reconcile.rs:276-280`): a user with no pre-2025 lots needs no allocation at all (Path A), so being told to "attest first" before the "no lots to allocate" message is mildly user-hostile. Recommend either gating after the lots computation, or documenting that the attestation precondition deliberately fires first.

- **M8 — Note that `safe_harbor_attest` is intentionally not gated.** `safe_harbor_attest` (`reconcile.rs:438`) voids + re-appends the existing allocation via `append_decision`, copying the prior `pre2025_method` (`..prior`, line 523) — it introduces no new undeclared method, so it correctly stays outside the D3 gate. The spec should say so explicitly so a later reviewer doesn't read it as a bypass hole.

## Nit

- **N1 — A few citation spans are off by one or over-wide:** advisory loop is `render.rs:982-990` (spec says 982-989); the attested-fact line is `render.rs:991-996` (spec says 991-995); `project/mod.rs:25-42` spans the `LotMethod` enum too (field is line 35). Cosmetic; verify-at-write-time hygiene.

---

## Answers to the seven review questions

1. **Premise accuracy:** CONFIRMED. Basis machinery is genuinely complete and method-aware; the three gaps are real; nothing is rebuilt that already works, and no real basis-adjustment gap is missed.
2. **D3 (allocate gate):** Mechanism is **correct**. A command-time refusal is the right tool precisely *because* the append is irrevocable — a projection-level Hard blocker fires only after the bad allocation is already immutably in the log (too late). It correctly prevents the append (return before `append_and_save`). Requiring attestation-of-FIFO is acceptable, not user-hostile, because the everyday compute path stays open (only the irrevocable step is gated). **No conflict / double-fire** with `Pre2025MethodConflictsAllocation`: D3 gates *creation* at command time; that blocker detects *post-creation* live-vs-recorded drift at projection time — different times, different causes, and if D3 fires no allocation exists for the blocker to evaluate. (See I1 for the missing test-update work this gate necessitates.)
3. **D2 (attestation-aware advisory):** Correct to keep `Pre2025MethodNote` Advisory and never gate `compute_tax_year`. Text-only is fine — a machine-readable signal already exists via `VerifyReport.pre2025_method_attested`. "Informational when attested, warning when unattested" is right. (See M3/M4/M5 for precision fixes.)
4. **D1 (plumb into `ProjectionConfig`):** Right home; `to_projection` maps cleanly. No serde/back-compat concern for `ProjectionConfig` (no serde derives; built per-projection). The real cost is the `Default` impl + ~15 test literals (M2) and the `note_pre2025_once` signature change (M3).
5. **Autonomous decisions:** (a) NOT hard-gating tax computation — **correct** (FIFO is the legal default; keep the basic flow usable). (b) No new ledger event — **flagged (I2)**: the justification is unsound for Path-A filers; needs durable record or an honest scope-narrowing. (c) Direct carryforward import "already covered" — **overstated (M6)**: payload yes, command UX no.
6. **Backward-compat:** The two named consequences (louder advisory for vaults with pre-2025 disposals; allocate now requires an attest step) are the right two and the only engine-visible ones I can find. But the spec/plan **does not update the existing tests those changes break (I1)** — that is the blocking gap here.
7. **Scope / right-sizing / TDD:** Tasks are individually testable; Task 3 is even independent of Task 1 (it reads the pre-existing `CliConfig` field, not the new `ProjectionConfig` one). The plan is **missing the existing-test-update task (I1)** and undersizes the literal churn (M2). Legal grounding (§7.4 FIFO default; attestation = taxpayer representation; §1.1012-1(j)(3) standing-order posture) is consistent with the existing code comments and reasonable for a design spec.

---

## Required to reach green (next fold)

1. **I1** — add the existing-CLI-test-update task; enumerate the 9 call sites; attest FIFO before allocate.
2. **I2** — either record the attestation durably (event-log) or correct decision (b)'s claim for Path A and defer durable Path-A attestation to `FOLLOWUPS.md` with rationale.
3. Fold M1 (SemVer line) at minimum; M2–M8 are non-blocking but cheap and improve plan fidelity.

Re-dispatch this review after the fold (the loop continues after every fold, including the last).

---

# Round 2 — re-review (post-fold)

- **Artifact:** `design/SPEC_pre2025_method_reconciliation.md` (revised).
- **Source re-verified against:** `origin/main` @ `c70922d` (== spec baseline; HEAD unchanged). All spot-checked citations re-confirmed at this HEAD.
- **Scope:** confirm the round-1 fold closed I1 + I2, confirm the listed Minors were addressed, and confirm the fold introduced no new Critical/Important. Premise was CONFIRMED accurate in round 1 — not re-litigated.
- **Verdict: GREEN — 0 Critical / 0 Important. I1 + I2 CLOSED; Minors folded; no new C/I. Ready to implement.**

## I1 — CLOSED (verified against source)

New **Task 0** is present and correct:
- **Sequencing:** explicitly the FIRST implementation step, "interleaved with Task 3 (the test fix and the gate must land together so the suite is never RED)." Closes the round-1 red-suite gap. ✔
- **Grep-ALL instruction:** "grep `safe_harbor_allocate` across `crates/btctax-cli/tests/` to find ALL call sites, not only the nine enumerated." ✔
- **Enumerated call sites all exist exactly:** `tests/reconcile.rs` @ 498, 552, 561, 599, 684 and `tests/verify_report.rs` @ 112, 125, 216, 227 — 9 call sites across 6 tests, matching the source line-for-line. My independent grep confirms these are the *only* `safe_harbor_allocate` call sites in `tests/`, so the enumeration is complete (the grep-ALL instruction is belt-and-suspenders, not load-bearing). ✔
- **The gate would indeed break them:** none of the 6 tests calls `set_pre2025_method(..., /*attested=*/true)` in its body — they run under the default (`pre2025_method=Fifo, attested=false`, `config.rs:23`). Under D3 each `safe_harbor_allocate(...).unwrap()` would return `CliError` and panic at *runtime* (the call signature is unchanged, so the compiler can't catch it). Confirmed by reading each test body (reconcile.rs 482–700; verify_report.rs 90–230). ✔
- **Fix correctness:** attesting FIFO before the first allocate is behavior-preserving — these tests already run under default FIFO and assert allocation behavior, not the new precondition. The CLI helper the tests already use is `cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true)` (cf. `verify_report.rs:290`). ✔

Non-defect note (no action required): the `false` fourth argument in these allocate calls is `timely_allocation_attested` (§5.02(4)) — a *different* attestation from the `pre2025_method_attested` config flag D3 gates on. The spec keeps these distinct (D3 reads `session.config()?.pre2025_method_attested`), so Task 0's config-flag fix is independent of the timely param. No conflation.

## I2 — CLOSED (claim now accurate; deferral is the right call)

- The north-star (spec lines 8–9) and decision (b) (lines 90–102) now scope the durable, immutable record to **Path B only** ("the `SafeHarborAllocation` records `pre2025_method` in the append-only ledger"), and state plainly that for **Path A** the attested method "lives solely in mutable `cli_config`, which is explicitly NOT the source of truth (NFR6) — so there is no durable/auditable record … for the majority case. This slug therefore does NOT claim to durably record the Path-A declaration." The unsound round-1 claim (that the immutable allocation backs the Path-A representation) is gone. ✔
- The durable Path-A `Pre2025MethodDeclaration` event is explicitly **deferred to FOLLOWUPS** (Task 5 line 170–171; Out-of-scope lines 174–178) with a sound rationale: "for Path A nothing is irrevocably committed (the basis recomputes from events under whatever method is set, and the advisory updates with it), so the absence of a durable record changes no number — it is an audit-trail gap, not a correctness gap." ✔
- **Is deferral right (vs in-slug)?** Yes. D3 gates the *only* irrevocable step (the Path-B allocation append); Path A has no allocation and re-derives the carryforward live under the current config on every projection, so a missing durable record protects no number. An append-only, supersede-tracked declaration event is a non-trivial new ledger event type that would over-size a "give the declaration teeth" slug. Round 1 offered "(a) build or (b) correct-claim-and-defer — either is acceptable"; choosing (b) is right-sized. I do **not** think the durable event must be in-slug.

## Minors — folded

- **M1 (SemVer):** lines 11–15 now read "additive `ProjectionConfig` field + a command-time refusal precondition ⇒ MINOR … No new `BlockerKind` variant (the allocate gate is a CLI refusal, not a projection blocker)." No more spurious new-Hard-variant claim; no more loose serde wording. ✔
- **M3 (signature):** D2/Task 2 now say `note_pre2025_once` "currently takes `method: LotMethod` (not the whole `ctx`); add an `attested: bool` parameter, threaded from `ctx.config.pre2025_method_attested` at all THREE call sites (`fold.rs:551, 931, 998`)." Verified against source: signature is `(st, date, ev, method: LotMethod)` @ `fold.rs:80`; exactly three call sites @ 551/931/998. ✔
- **M5 (round-1 M4 — KAT fixture):** Task 2 now requires "a real pre-2025 disposal (Dispose/GiftOut/Donate with `disposed_at.year() < 2025`) — the note only fires on the first such event … a buy-only ledger never triggers it." Verified the three fire-once arms are exactly `Op::Dispose` (532→551), `Op::GiftOut` (915→931), `Op::Donate` (978→998). ✔
- **M6 (round-1 M5 — Task 4 verification-only):** Task 4 is reframed "(likely verification-only)"; render.rs edit "ONLY if an actual contradiction is found." Verified accurate: `render_verify` already prints advisory blockers (the loop @ `render.rs:982–989/990`) and the separate "Pre-2025 method (attested historical fact): {m} (attested: {bool})" line (@ `991–996`); D2's text-only change flows through unchanged. ✔
- **M7 (round-1 M6 — out-of-scope import claim):** reframed to "Fully-unconstrained import of an as-filed ending carryforward"; the "already covered" overstatement is gone, replaced by an accurate conservation-checked characterization. ✔ (Minor residual below.)
- **M8 (attest not gated):** Out-of-scope lines 183–186 state `safe_harbor_attest` is intentionally NOT gated by D3 ("the allocation it operates on already passed the D3 attestation gate at creation time"), with an impl-time confirm. ✔

## No new Critical/Important from the fold

- **D3 unchanged + still right:** command-time refusal reading raw `session.config()?.pre2025_method_attested`, returning before the irrevocable append — the correct tool at the only irrevocable step. ✔
- **D1/D2 still sound.** ✔
- **Deferral leaves no correctness hole:** Path A commits nothing irrevocable (carryforward recomputes from events under the current config; the only irrevocable commit is the Path-B allocation, which D3 gates). The missing durable Path-A record is audit-trail-only. ✔
- No defect introduced by the fold's new prose. (Task 0's illustrative `config::set_pre2025_method(conn, …)` takes a `&Connection` the vault-path tests don't hold directly, but it is hedged with "or the CLI helper the test already uses" — see Residual N2.)

## Residual (non-blocking; carried from round 1, NOT introduced by the fold — none bars GREEN)

- **R-M2 (round-1 M2, Minor):** Task 1 still says only "any `ProjectionConfig` test literals" without naming the `Default for ProjectionConfig` impl + the ~15 struct literals across the core test files. Compiler-enforced (no silent miss), so non-blocking; would improve task sizing.
- **R-M7b (round-1 M7 tail, Minor):** the "read raw `CliConfig`, not `to_projection`" half of round-1 M7 is now satisfied by D3's `session.config()?` text; the gate-ordering-vs-empty-lots-refusal UX nuance is still unaddressed in Task 3. Minor UX, non-blocking.
- **R-N1 (round-1 N1, Nit):** citation spans still slightly narrow — advisory loop is `render.rs:982–990` (spec says 982–989); attested-fact line is `991–996` (spec says 991–995). Cosmetic.
- **R-N2 (Nit, new wording):** Task 0's example helper form is imprecise for vault-path tests (the `config::*` fn takes `&Connection`); already hedged, so harmless.

## Round-2 verdict

**I1 CLOSED. I2 CLOSED. All listed Minors (M1, M3, M5, M6, M7, M8) folded. 0 new Critical / 0 new Important.** Only four non-blocking Minor/Nit residuals remain, all carried from round 1 and explicitly optional. The spec is **R0 GREEN — ready to implement.** Per §2, the loop runs after every fold including the last: this round-2 pass closing at 0 C / 0 I is that terminal re-review.
