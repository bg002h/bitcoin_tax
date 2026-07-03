# R0 spec review — `SPEC_tui_edit_save_rollback.md` (round 1)

**Artifact:** `design/SPEC_tui_edit_save_rollback.md` (A′ snapshot/restore rollback on failed save).
**Baseline:** `main` @ `45b9332`, all citations re-checked against current source at review time.
**Reviewer role:** independent adversarial architect (did NOT author). Bar: 0 Critical / 0 Important.

## Verdict: 0 Critical / 2 Important / 2 Minor / 1 Nit

The core mechanism is sound and unusually well-grounded: restore-correctness (D1), `decision_seq`
reuse (D3), the `persist_void` whole-DB revert (D3), the latch coexistence (D4), the `restore(`
token safety (KAT-G1), and the "rewrite exactly 4" completeness (D5) all hold up against source.
The two Important findings are integrity gaps, not mechanism defects: (I1) the enum's stated safety
rationale is false and its one catastrophic-path write is untested; (I2) the spec mandates rolling
back `persist_tax_profile` but leaves a doc header that says "do NOT roll back," and omits it from
its own doc-rewrite list. Both are cheap to close.

---

## RULING — the `safe_harbor_attest` scope fork

**Leaving attest OUT this cycle (keep its `attest_save_failed` latch; add rollback for the other 8)
is the RIGHT scope. It is NOT an incoherent half-measure. I concur with the two architects who said
leave it out, and I reject the third's premise.**

Grounded in what the attest path actually does:

- `persist_safe_harbor_attest` (`persist.rs:288-323`) appends **two** decisions (void + re-attest)
  then one `save()`. On save failure both are residue; a retry double-batches into two effective
  allocations → Hard `DecisionConflict` + Path A, **both copies §7.4-unvoidable → unrecoverable**
  (documented `persist.rs:280-287`). This is categorically worse than the 8 benign fns, whose
  residue is at worst a `DecisionConflict` the user can void.
- The Err arm (`main.rs:3706-3712`) sets `attest_save_failed = true` and shows a quit-first status.
  The latch is dead-simple: a bool that makes all 9 openers refuse until quit; quitting discards the
  in-memory residue (disk is pre-action). It is proven and has a passing KAT
  (`kat_e2e_attest_errlatch_chmod`, `main.rs:10951`).

Coherence check (the "half-measure" worry): the two mechanisms coexist **without interference**.
`residue_latch_status()` (D4) unifies the *opener-refusal UX* across BOTH `attest_save_failed` and
the new `rollback_failed`, so the user sees one consistent "quit + CLI" story regardless of which
latched. The attest latch is untouched; `rollback_failed` is a sibling for the OOM-only
`ResidueLive` case. There is no shared state that lets a latent bug in the new rollback path reach
the catastrophic attest path — which is exactly the property that justifies not wiring a brand-new
mechanism into the unrecoverable path until it soaks. The followup ("attest adopts snapshot/restore,
retire the latch, enable in-editor retry") is correctly filed (spec §Out-of-scope, Plan Task 4).

**On the third (sequencing) architect's assumption that "A retires the `attest_save_failed` latch":
that premise is false against this spec.** The spec explicitly keeps the latch (Hard Constraints
bullet 3, Out-of-scope bullet 1, and the D4 helper that reproduces its verbatim string). Nothing in
A′ retires it. So the sequencing concern is moot — there is no latch-retirement to sequence.

---

## Findings (most severe first)

### [I1] IMPORTANT — the enum's stated safety guarantee is false, and the sole `ResidueLive→latch` write is untested

`file: design/SPEC_tui_edit_save_rollback.md` D3 ("makes the dangerous case **impossible to ignore**
(the compiler forces every Enter arm to handle it)"); D4 (the `ResidueLive(e)` arm sets
`rollback_failed`); D5 (the restore-failure latch KAT).

**Defect.** The claim that the type system *forces* every Enter arm to handle `ResidueLive` is not
true. The current arms are `Err(e) => app.status = Some(format!("Save error: {e}"))`
(`main.rs:317, 519, 974, 1320, 1387, 1728, 2603, 2893`). After the return-type change, the only
thing the compiler forces is *some* destructuring — because `PersistError` derives `Debug` but not
`Display`, so `{e}` on the whole enum won't compile. An implementer can satisfy the compiler with a
single collapsed arm — `PersistError::NoChange(e) | RolledBack(e) | ResidueLive(e) => format!("Save
error: {e}")` — which is exhaustive, compiles clean, and **silently never sets `rollback_failed`**.
Nothing catches this: the D5 "restore-failure latch KAT" *directly sets* `rollback_failed = true`
and asserts the openers refuse — i.e. it tests the latch **consumer**, never the **producer**. The
one line in the entire change that arms the catastrophic-path latch has zero coverage, and
`ResidueLive` is OOM-only so no chmod/e2e test can reach it.

**Why it gates.** This is the load-bearing design rationale for choosing an enum over a bool
(`impossible to ignore`), and it is incorrect. For a residue-elimination spec, shipping the sole
producer of the unrecoverable-case latch with an untested, non-enforced path is exactly the kind of
gap R0 exists to close. (Runtime probability is admittedly tiny — a reasonable panel could rate this
Minor — but the false rationale + zero coverage on the safety-critical write earns Important.)

**Fix (cheap).** Factor the outcome→effect mapping out of the inline arms into a pure helper, e.g.
`fn apply_persist_err(app: &mut EditorApp, e: PersistError, keep_open: impl FnOnce(&mut EditorApp))`
(or a per-flow `handle_persist_outcome`), and add a **producer** unit test that constructs
`PersistError::ResidueLive(<any CliError>)` by hand, drives the helper, and asserts
`app.rollback_failed == true` + the CRITICAL status + flow closed. Additionally: (a) add a Hard
Constraint that `PersistError` must **not** implement `Display` (it is load-bearing — it forces
destructuring), and (b) state in D4 that `ResidueLive` must be its **own** match arm — a combined /
catch-all arm is forbidden — so whole-diff review has an explicit checklist item.

### [I2] IMPORTANT — spec mandates rolling back `persist_tax_profile` but its doc header still says "do NOT roll back," and the doc-rewrite list omits it (internal contradiction)

`file: crates/btctax-tui-edit/src/edit/persist.rs:30-35` vs `design/SPEC_tui_edit_save_rollback.md`
Hard-Constraints bullet 4 + D3 "Doc-comment rewrites (required)".

**Defect.** The spec deliberately **includes** `persist_tax_profile` in the rollback set "for a
clean universal invariant" (Hard Constraints: "Rolling back an idempotent upsert is unnecessary but
harmless"). But `persist_tax_profile`'s live doc header states verbatim:

> "This divergence is intentional and safe — **do NOT roll back the side-table**. The upsert is
> idempotent; a retry re-runs it on the next confirmed action." (`persist.rs:34-35`)

That is the exact behavior the spec is reversing. The spec's D3 "Doc-comment rewrites (required)"
list enumerates only `persist_reclassify_outflow` (`:84-98`), `persist_select_lots` (`:222-233`),
and `persist_void` (`:176-185`) — it **misses** `persist_tax_profile:30-35`. So the spec both
mandates a behavior and, by omission, leaves a doc header that flatly contradicts it. This is an
internal inconsistency in the artifact, and it would mislead the whole-diff reviewer and any future
maintainer ("the side-table is explicitly NOT rolled back here").

**Why it gates.** The spec claims its doc-rewrite list is the complete set of stale headers ("The
stale headers that describe the *old* residue/retry reality must be rewritten"); it is not, and the
missed one is a direct contradiction of the cycle's central invariant.

**Fix.** Add `persist_tax_profile:30-35` to the D3 doc-rewrite list. New text: the upsert is now
reverted with the whole DB on a failed save (memory always equals disk), same universal invariant as
the other seven; retry re-runs the idempotent upsert with a clean slate.

### [M1] MINOR — D3's persist-fn body won't compile: missing `From<CoreError> for PersistError` (and the inline comment misnames the type)

`file: design/SPEC_tui_edit_save_rollback.md` D3 snippet
(`let id = append_decision(...)?; // CliError ⇒ NoChange`).

`append_decision` returns `Result<EventId, btctax_core::CoreError>` (`persistence.rs:238-262`), **not**
`CliError`. Today's persist fns compile because they return `Result<_, CliError>` and `CliError` has
`Core(#[from] CoreError)` (`btctax-cli/src/lib.rs:18-24`). Once the fns return
`Result<_, PersistError>`, the bare `?` on `append_decision` (and on `persist_void`'s
`load_all(...)?`) needs `From<CoreError> for PersistError` — Rust's `?` does **not** chain
`CoreError → CliError → PersistError`. The spec defines only `From<CliError>`. So the 6 append-based
fns won't compile as written, and the comment "CliError ⇒ NoChange" is factually wrong (it's
`CoreError`). Compile-caught with an obvious fix, hence Minor — but it is a real gap in the "is the
enum + From sound?" question.

**Fix.** Add `impl From<CoreError> for PersistError { fn from(e) -> Self {
PersistError::NoChange(e.into()) } }` (CoreError→CliError→NoChange is the correct semantics: an
append failure leaves no residue), OR change the snippet to
`append_decision(...).map_err(btctax_cli::CliError::from)?`. Correct the comment.

### [M2] MINOR — KAT-G1 self-check should runtime-construct the `restore(` token (mirror the existing pattern)

`file: crates/btctax-tui-edit/src/edit/persist.rs:961-998` (the plant-a-token self-check) vs
spec D5 ("add `"restore("` to … the plant-a-token self-check").

The token addition itself is **safe and verified**: no non-test code in `btctax-tui-edit/src`
contains `restore(` today. The only near-collisions are `restore_terminal` (`main.rs:49, 3928`) and
the doc/test-comment mentions — none contain the substring `restore(` (it's `restore_`), so the gate
stays green. Good. Two small things for the implementer, though: (1) the existing self-check builds
every forbidden token at runtime (`format!("{}(", "save")`, etc.) specifically so no literal
forbidden token sits in this source file; the `restore(` addition should follow suit
(`format!("{}(", "restore")`) rather than embedding a literal, for consistency and future-proofing.
(2) Worth a one-line comment near the token list recording *why* `restore_terminal` is not a false
positive (substring is `restore_`, not `restore(`), so a future refactor that shortens the name
doesn't silently defeat the gate. (Confirms the spec's separate decision to leave `snapshot(`
**ungated** — `build_snapshot(` at `main.rs:3691` contains the substring `snapshot(`, so gating the
bare word WOULD false-positive; the spec's reasoning there is correct.)

### [N1] NIT — for the "universal invariant" framing, the four silent doc headers could carry a one-line failed-save note

`persist_classify_inbound` (`:46-59`), `persist_reclassify_income` (`:115-125`),
`persist_set_fmv` (`:142-152`), and `persist_donation_details` (`:253-257`) describe append/upsert
semantics but say nothing about failed-save. They don't *contradict* the new behavior (unlike I2),
so this is optional — but since the whole cycle sells a single invariant ("every editor persist fn
reverts on a failed save … except attest"), a one-line "on failed save the in-memory DB is reverted;
retry is clean, same seq" on each would make the invariant self-documenting and prevent a future
drift like I2. Purely stylistic.

---

## Pressure-test results (what was checked and found sound)

- **D1 restore-correctness — SOUND.** Verified against source: the whole tree has **zero** PRAGMA
  writes (only a `PRAGMA page_count` *read* in `db_to_bytes`, `sqlite_io.rs:18`), **no**
  `create_scalar_function`/custom SQL funcs, **no** `busy_timeout`/`foreign_keys`/`journal_mode`,
  **no** persistent `prepare_cached`. `Vault::open` builds its conn via `sqlite_io::db_from_bytes`
  (`vault.rs:136`) and `restore` uses the same call — so a restored conn is state-identical to a
  freshly-opened one; nothing cached/attached is lost. Untouched-on-failure holds:
  `self.conn = db_from_bytes(image)?` evaluates `?` before the assignment (`db_from_bytes`'s only
  failure is `sqlite3_malloc64` OOM, `sqlite_io.rs:38-43`). `restore` leaves `path`/`cert`/`_lock`
  untouched (`vault.rs:17-22`), so the exclusive lock is held across the swap.
- **`snapshot()` fail-closed — SOUND.** The snapshot is the **first** statement in every persist fn,
  before any `append`/`clear`, so a snapshot failure (OOM) is strictly pre-write ⇒ mapping to
  `NoChange` (nothing written, no residue) is correct. There is no path where snapshot runs after a
  partial write.
- **`persist_void` side-effect — SOUND.** `optimize_attest::clear` runs before `save()`
  (`persist.rs:216`); the whole-DB restore reverts both the `VoidDecisionEvent` append and the
  side-table clear — the exact thing a per-row DELETE would miss. Snapshot-at-top precedes the clear.
- **`decision_seq` reuse — SOUND.** `append_decision` computes seq as
  `COALESCE(MAX(decision_seq),0)+1 … WHERE kind='decision'` fresh each call (`persistence.rs:246-250`).
  After a byte-identical restore the table is pre-attempt, so a retry recomputes the identical seq —
  no gap, no duplicate, no `DecisionConflict`.
- **D4 latch refactor — SOUND.** All **9** opener guards (`open_profile_form:411`,
  `open_classify_inbound_flow:1825`, `open_reclassify_outflow_flow:1927`,
  `open_reclassify_income_flow:2118`, `open_set_fmv_flow:2230`, `open_void_flow:2416`,
  `open_select_lots_flow:3206`, `open_set_donation_details_flow:3330`,
  `open_safe_harbor_attest_flow:3465`) are **byte-identical** today, so folding them into one helper
  that returns that exact string is behavior-preserving. No 10th mutating opener exists (these are
  the only `open_*` fns). `kat_e2e_attest_errlatch_chmod` asserts the loop status
  `contains("failed attest save")` (`main.rs:11045-11052`) and the Err-arm status
  (`11005-11016`, set by the untouched attest arm) — both survive the refactor. The
  `attest`-branch-first ordering in the helper is correct (attest is the more catastrophic remedy).
  A second latch is warranted: the two conditions have genuinely different remedies/strings and the
  attest one must stay verbatim; collapsing to one bool would lose that.
- **KAT-G1 `restore(` — SAFE (verified).** No `restore(` substring in any non-test region of
  `btctax-tui-edit/src` today; `restore_terminal`/`Terminal::new` teardown does not collide. (See M2
  for the self-check style nit.)
- **The 4-KAT rewrite — COMPLETE.** Exactly four e2e residue KATs assert the old `pre+2` + conflict
  behavior: `kat_s2_…_classify_inbound_chmod:5052` (`:5163`), `kat_s2_ro_…_outflow_chmod:6571`
  (`:6683`), `kat_s2b_…_set_fmv_chmod:8058` (`:8155`), `kat_s3a_…_select_lots_chmod:9389` (`:9491`).
  I searched **beyond** the four and confirmed the other chmod/`pre+2` tests do **not** need
  rewriting: `kat_s1_save_error_path_chmod_parent:4546` is a **side-table** upsert (profile) that
  asserts the event log stays empty (`:4667-4670`) with **no** residue count — survives unchanged;
  `kat_e2e_fmv_repoint_…:7639` and `kat_void_retry_idempotent_…:8690` call the persist fn **twice
  with both saves succeeding** (`.unwrap()`), so `pre+2` is legitimate double-success, not residue;
  `ATTEST-HAPPY:10689` (`pre+2` = void+re-attest on success) and `E2E-FMV:7607` (2 decisions on
  success) are likewise legitimate. The spec's "rewrite exactly 4" is correct. The rewrite recipe
  (preserve UX + byte-identical disk `:5133`/`:9473`; invert `pre+2`→`pre+1`/no-conflict; ADD a
  `load_all_ordered(conn) == pre` pin immediately post-failure, pre-retry) is the right and
  sufficient set — the added `== pre` pin is what actually verifies A′.
- **Return-type ripple — CONTAINED.** Exactly 8 non-test call sites (`main.rs:286, 485, 940, 1288,
  1360, 1694, 2570, 2865`) plus the untouched attest arm (`3681`). No hidden call sites. No test
  matches persist results on a `CliError` variant; all test call sites use `.unwrap()` (needs only
  `Debug`, which is derived) — so the `PersistError` change won't break existing tests.
- **Citations — accurate.** All 8 persist-fn line numbers (`:36/60/99/126/153/186/237/258`), the
  Enter-arm lines, `optimize_attest::clear:216`, `persist_only_tokens:788`, `persistence.rs:238/
  245-250`, `vault.rs:21/136/147-150`, `sqlite_io.rs:9-11/38-43`, `session.rs:66-69`,
  `editor.rs:142`, and the four KAT line numbers all check out against current source.

---

**Gate status: BLOCKED on I1 + I2 (both cheap). Fold, re-review round 2. 0 Critical.**
