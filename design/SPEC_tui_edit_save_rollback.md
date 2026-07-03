# SPEC — tui-edit-save-rollback: clean in-memory rollback on a failed save

**Source baseline:** `main` @ `45b9332` (working tree re-verified file-by-file at write time; all
line citations checked against current source).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/spec-review-tui-edit-save-rollback-r0-round-1.md` (0C/2I/2M/1N — attest-scope fork ruled:
leave attest out) and `-round-2.md` (0C/0I — folds verified).**
**Design lineage:** two independent architect consultations — a recommend pass (A′ snapshot/restore)
and an adversarial pass (verdict **ENDORSE-WITH-CHANGES**: measured the happy-path cost to
~0.1 ms/decision, proved the SAVEPOINT alternative dead against `append_decision`'s internal
`BEGIN`, and required three changes now folded in: the restore-failure latch, folding
`persist_tax_profile` into the rollback set, and the precise KAT rewrite).

**Goal.** When an editor persist fn's `session.save()` fails, revert the **in-memory** SQLite DB so
it is byte-identical to the (untouched) on-disk vault. This eliminates the "confirmed-but-unsaved
decision piggy-backs onto a later successful save" hazard for the eight editor persist fns, and
replaces today's "failed save → residue → retry produces `N+2` rows + a `DecisionConflict`" with
"failed save → clean no-op; retry is clean, same `decision_seq`."

**SemVer.** New `pub fn`s `Vault::snapshot`/`Vault::restore` (`btctax-store`),
`Session::snapshot`/`Session::restore` (`btctax-cli`); a new `edit/persist.rs` outcome enum
`PersistError` (return-type change on the 8 persist fns, editor-crate-internal); a new `EditorApp`
field `rollback_failed: bool`; `"restore("` added to `persist_only_tokens`. No `btctax-core`
changes. No new workspace member. **MINOR** (pre-1.0; additive).

---

## Background — the hazard being closed

`append_decision` (`crates/btctax-core/src/persistence.rs:238`) opens its own
`conn.unchecked_transaction()` (`persistence.rs:245`) and **commits** the append to the in-memory
conn before returning. Each editor persist fn (`crates/btctax-tui-edit/src/edit/persist.rs`) then
calls `session.save()` — which serializes the whole in-memory DB (`sqlite_io::db_to_bytes`) and
atomically writes the encrypted vault file (`crates/btctax-store/src/vault.rs:147-150`). The
serialize/encrypt/`atomic_write` step is entirely separate from the append's transaction.

So when `save()` returns `Err` (transient disk-full / permission / I/O), the committed append is
**residue** in the in-memory conn while the on-disk vault is unchanged (KAT-S1 already pins
byte-identical-on-error). Today's Err arms close the modal and keep the form open with a `"Save
error"` status (e.g. `main.rs:519, 974, 1320, 1387, 1728, 2603, 2893`; `317` for tax-profile). Two
concrete failures follow:

1. **Cross-flow piggy-back (reachable, not theoretical).** The Err arm closes only the *modal*, not
   the flow; the user can `Esc` out of the form, return to Browse, and confirm an unrelated flow.
   The residue then rides along on that flow's successful `session.save()` — a decision the user
   believes failed lands silently later.
2. **Same-flow retry is a self-inflicted conflict.** The four shipped save-error KATs
   (`kat_s2_save_error_path_classify_inbound_chmod` `main.rs:5052`,
   `kat_s2_ro_save_error_path_reclassify_outflow_chmod` `:6571`,
   `kat_s2b_save_error_path_set_fmv_chmod` `:8058`,
   `kat_s3a_save_error_path_select_lots_chmod` `:9389`) currently **assert** `post.len() == pre + 2`
   after failure+retry — i.e. the residue AND the retry both land — and for classify-inbound a real
   `DecisionConflict` the user must clear via the CLI. That "recovery" is a wart, not a fix.

For the **irreversible** `safe-harbor-attest` the analogous residue is catastrophic (an unrecoverable
double-batch), so it is already defended by the `attest_save_failed` latch (`editor.rs:142`) that
refuses every mutating opener until quit. **This spec leaves that latch untouched** (see Out of
scope); it addresses the eight *other* persist fns, whose residue is benign-but-dishonest.

---

## Hard constraints

- **On-disk append-only is preserved.** The vault file only ever receives *successful* appends via
  `Vault::save`'s atomic path. `restore` never touches the disk — it replaces only the in-memory
  conn. The rollback removes a row that **never reached disk**, so no observer of the vault file
  ever sees a delete.
- **KAT-G1 (the editor's mechanized source gate).** Only `edit/persist.rs` may name the
  vault-mutation tokens. `"restore("` is **added** to `persist_only_tokens` (`persist.rs:788`) and to
  the plant-a-token self-check (`persist.rs:962-982`), so `Session::restore` is callable only from
  `edit/persist.rs`. Verified safe: NO `restore(` substring exists in any non-test region of
  `btctax-tui-edit/src` today (ratatui teardown is `restore_terminal` = `restore_`, not `restore(`).
  [R0-M2] Construct the token at RUNTIME (`format!("{}(", "restore")`, mirroring the shipped
  `tok_save`/`tok_conn`) rather than embedding a literal, and add a one-line comment that
  `restore_terminal` is not a false positive. `"snapshot("` is left **ungated** (a pure read, and
  `build_snapshot(` at `main.rs:3691` contains that substring, so gating it would false-positive).
- **The `attest_save_failed` latch is untouched.** Its semantics, status, and KAT
  (`kat_e2e_attest_errlatch_chmod`) stay exactly as shipped. The new `rollback_failed` latch is a
  sibling, folded into a shared opener guard (D4) that *preserves the attest status verbatim*.
- **`persist_tax_profile` is INCLUDED** in the rollback set for a clean universal invariant:
  *every editor persist fn reverts the in-memory DB on a failed save — memory always equals disk —
  except the latched irreversible attest.* (Rolling back an idempotent upsert is unnecessary but
  harmless; the alternative — one unexplained exception — is worse.)
- **The editor holds the store's exclusive `VaultLock` for its lifetime** (`vault.rs:21` `_lock`);
  `restore` leaves `_lock`, `cert`, and `path` untouched, so the lock is held across the conn swap.
- **`PersistError` MUST NOT implement `Display`** [R0-I1] (`Debug` only). This makes a lazy
  `format!("{e}")` a compile error, forcing every consumer through `on_persist_error` (D4) — the one
  place that arms the `rollback_failed` latch — instead of a catch-all arm that silently drops it.

---

## Design

### D1 — `Vault::snapshot` / `Vault::restore` (`btctax-store`)

Reuses the exact serialization the vault already round-trips on every `open()` (`vault.rs:136`
builds its conn via `sqlite_io::db_from_bytes`), so a restored conn is **state-identical to a
freshly-opened vault**. Grounding facts (verified): the tree has **zero** `PRAGMA` writes,
`foreign_keys`, `busy_timeout`, prepared-statement caches, or custom SQL functions — the live conn
is plain SQLite defaults, so nothing cached/attached is lost across the swap. `db_to_bytes` is
`conn.serialize(Main).to_vec()` (a page-image memcpy; `sqlite_io.rs:9-11`), measured at ~0.1 ms for
a realistic vault (≤1 ms at 20k events) — negligible against `save()`'s existing serialize + OpenPGP
encryption + three fsyncs.

```rust
// crates/btctax-store/src/vault.rs — in `impl Vault`, adjacent to `save` (vault.rs:147).
/// Serialize the current in-memory DB image (no disk I/O). A prior `snapshot()` result can be
/// fed to `restore()` to revert an attempted mutation whose `save()` failed.
pub fn snapshot(&self) -> Result<Vec<u8>, StoreError> {
    sqlite_io::db_to_bytes(&self.conn)
}

/// Replace the in-memory DB with `image` (a prior `snapshot()`). No disk I/O; the vault file,
/// `VaultLock`, `cert`, and `path` are untouched. On `Err` the assignment never runs, so
/// `self.conn` is UNCHANGED and the caller must treat `Err` as "residue may still be live".
pub fn restore(&mut self, image: &[u8]) -> Result<(), StoreError> {
    self.conn = sqlite_io::db_from_bytes(image)?;
    Ok(())
}
```

**Restore-failure semantics (load-bearing).** `db_from_bytes`'s only failure is `sqlite3_malloc64`
OOM (`sqlite_io.rs:38-43`). Because `self.conn = db_from_bytes(image)?` evaluates the `?` **before**
the assignment, an OOM leaves `self.conn` = the residue-bearing conn (untouched-on-failure). This is
provable by inspection but not chmod-inducible; the caller (D3/D4) MUST treat a `restore` `Err` as
"residue live" and latch, never silently swallow it.

**Unit tests (store):** near `vault.rs` tests — (a) mutate → `snapshot` → mutate more → `restore` →
assert the DB image equals the snapshot; (b) assert `restore` never changes the vault file's bytes
on disk (compare `fs::read` before/after); (c) round-trip fidelity mirrors
`sqlite_io::tests::db_roundtrip`.

### D2 — `Session::snapshot` / `Session::restore` (`btctax-cli`)

Thin wrappers over `Vault`, mirroring `Session::save` (`session.rs:66-69`). `StoreError` converts to
`CliError` via the existing `#[from]`.

```rust
// crates/btctax-cli/src/session.rs — adjacent to `save`.
pub fn snapshot(&self) -> Result<Vec<u8>, CliError> { Ok(self.vault.snapshot()?) }
pub fn restore(&mut self, image: &[u8]) -> Result<(), CliError> { self.vault.restore(image)?; Ok(()) }
```

### D3 — `edit/persist.rs`: `save_or_rollback` + the eight persist fns

A persist fn can fail at three points, which the caller must treat differently. A crate-internal
outcome enum plus a SINGLE tested handler (`on_persist_error`, D4) localise the dangerous
`ResidueLive` case to exactly one place. **`PersistError` deliberately does NOT implement `Display`**
[R0-I1] — so a lazy `format!("{e}")` cannot collapse the three variants into one catch-all arm that
silently drops the latch write; every consumer routes through `on_persist_error`, which is unit-tested
against a hand-built `ResidueLive` (D5).

```rust
/// Outcome of a persist fn. `NoChange` and `RolledBack` are both "nothing persisted, safe to
/// retry"; `ResidueLive` is the (astronomically rare) unrecoverable case.
#[derive(Debug)]
pub enum PersistError {
    /// Failed at snapshot or append — NOTHING was written and there is no residue.
    /// (snapshot failure = fail-closed refusal; append failure = no row added.)
    NoChange(btctax_cli::CliError),
    /// `save()` failed; the in-memory DB was cleanly reverted to its pre-mutation image.
    /// No residue; a retry re-appends with the SAME `decision_seq`.
    RolledBack(btctax_cli::CliError),
    /// `save()` failed AND the revert ALSO failed — the unsaved mutation is LIVE in the
    /// in-memory DB. The caller MUST latch every mutating opener and prompt an immediate quit
    /// (the on-disk vault is pre-action; quitting discards the residue).
    ResidueLive(btctax_cli::CliError),
}
// BOTH `From` impls are required [R0-M1]: `session.snapshot()?` yields `CliError`, but
// `append_decision(...)?` / `persist_void`'s `load_all(...)?` yield `btctax_core::CoreError`, and
// Rust does NOT chain `CoreError → CliError → PersistError`. Neither impl targets `ResidueLive`.
impl From<btctax_cli::CliError> for PersistError {
    fn from(e: btctax_cli::CliError) -> Self { PersistError::NoChange(e) }
}
impl From<btctax_core::CoreError> for PersistError {
    fn from(e: btctax_core::CoreError) -> Self { PersistError::NoChange(e.into()) } // CoreError → CliError
}

/// `save()`; on failure revert the in-memory DB to `pre`. Ok on success; `RolledBack` on a save
/// failure cleanly reverted; `ResidueLive` if the revert itself failed.
fn save_or_rollback(session: &mut btctax_cli::Session, pre: Vec<u8>) -> Result<(), PersistError> {
    match session.save() {
        Ok(()) => Ok(()),
        Err(save_err) => match session.restore(&pre) {
            Ok(()) => Err(PersistError::RolledBack(save_err)),
            Err(_revert_err) => Err(PersistError::ResidueLive(save_err)),
        },
    }
}
```

Each of the **eight** persist fns (`persist_tax_profile` `:36`, `persist_classify_inbound` `:60`,
`persist_reclassify_outflow` `:99`, `persist_reclassify_income` `:126`, `persist_set_fmv` `:153`,
`persist_void` `:186`, `persist_select_lots` `:237`, `persist_donation_details` `:258`) changes:

```rust
// return type:  Result<EventId, CliError>   →   Result<EventId, PersistError>
//               Result<(),      CliError>   →   Result<(),      PersistError>   (tax_profile, donation_details)
pub fn persist_classify_inbound(session: &mut Session, payload, now) -> Result<EventId, PersistError> {
    let pre = session.snapshot()?;                       // fail-closed: no snapshot ⇒ NoChange, refuse
    let id  = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?; // CoreError ⇒ NoChange
    save_or_rollback(session, pre)?;                     // save/rollback ⇒ RolledBack / ResidueLive
    Ok(id)
}
```

**`persist_void`'s side-effect is covered for free.** `persist_void` calls
`optimize_attest::clear(session.conn(), &disposal)` (`persist.rs:216`) *before* `save()`. Because
`save_or_rollback` restores the **whole DB image**, that side-table clear is reverted along with the
`VoidDecisionEvent` append — the exact thing a per-row DELETE would miss. This is the load-bearing
reason A′ uses whole-DB restore, not a logical undo (D-Alt below).

**`decision_seq` after rollback is correct by construction.** `append_decision` computes the seq as
`SELECT COALESCE(MAX(decision_seq),0)+1 … WHERE kind='decision'` (`persistence.rs:246-250`) — a fresh
query. After a byte-identical restore the table is exactly pre-attempt, so a retry recomputes and
reuses the identical seq: no gap, no duplicate.

**Doc-comment rewrites (required).** The stale headers that describe the *old* residue/retry reality
must be rewritten to the new one — a retry after a failed save is clean (no duplicate, no conflict,
same `decision_seq`), and the in-memory DB is reverted on failure:
- `persist_tax_profile:30-35` — currently says **"This divergence is intentional and safe — do NOT
  roll back the side-table"**, which now directly CONTRADICTS this spec's inclusion of tax-profile in
  the rollback set [R0-I2]. Rewrite to "reverted on a failed save via `save_or_rollback`."
- `persist_reclassify_outflow:84-98` — "[R0-I1] duplicate ⇒ FIRST-WINS … resolve via CLI".
- `persist_select_lots:222-233` — "[R0-I2] Duplicate ⇒ conflict, NEITHER applies".
- `persist_void:176-185` — "[M1]" block.
- [R0-N1, optional] the four silent headers (`persist_classify_inbound:46-59`,
  `persist_reclassify_income:115-125`, `persist_set_fmv:142-152`, `persist_donation_details:253-257`)
  each gain a one-line "reverted on a failed save; retry is clean" note so the universal invariant is
  self-documenting.

### D4 — restore-failure latch (`rollback_failed`) + shared opener guard

New `EditorApp` field `rollback_failed: bool` (default `false`), set **only** when a persist fn
returns `PersistError::ResidueLive`. It is a sibling of `attest_save_failed`; the nine mutating
openers guard on **either**. To avoid nine duplicated two-branch checks, the shipped inline
`if app.attest_save_failed { … }` blocks are refactored to a single helper that returns the correct
status for whichever latch is set — **preserving the attest status string verbatim** so
`kat_e2e_attest_errlatch_chmod` stays green:

```rust
impl EditorApp {
    /// The residue-latch status, if any mutating opener must refuse. `attest_save_failed` keeps its
    /// exact shipped wording; `rollback_failed` reports the unrevertable-residue remedy.
    fn residue_latch_status(&self) -> Option<String> {
        if self.attest_save_failed {
            Some("A failed attest save left unsaved decisions in memory — quit the editor \
                  (the unsaved attestation is discarded on quit), then retry via CLI: \
                  btctax reconcile safe-harbor-attest".to_string())
        } else if self.rollback_failed {
            Some("CRITICAL: a save failed and could not be reverted — unsaved data is in memory. \
                  Quit the editor NOW (the vault on disk is unchanged); no in-editor action will \
                  save until you quit, then re-run the operation via the CLI.".to_string())
        } else {
            None
        }
    }
}
// each of the 9 openers, first statement:
//   if let Some(s) = app.residue_latch_status() { app.status = Some(s); return; }
```

**The Enter arms delegate to ONE tested handler [R0-I1].** Rather than each of the eight arms
inlining a `match` (which an implementer could collapse into a catch-all that silently never arms the
latch — the exact gap R0 caught), every arm closes its own modal and calls `app.on_persist_error(e)`:

```rust
impl EditorApp {
    /// The SINGLE home for persist-error effects (unit-tested, D5). NoChange/RolledBack → benign
    /// keep-open status (nothing was persisted; safe to retry). ResidueLive → arm the
    /// `rollback_failed` latch, show the CRITICAL status, and close every open mutation surface.
    /// `PersistError` has no `Display`, so a lazy `{e}` cannot bypass the ResidueLive arm.
    fn on_persist_error(&mut self, e: edit::persist::PersistError) {
        use edit::persist::PersistError::{NoChange, ResidueLive, RolledBack};
        match e {
            NoChange(err) | RolledBack(err) => {
                // "Save error" prefix retained so existing substring asserts hold; form stays open.
                self.status = Some(format!("Save error: {err} — no changes were recorded; safe to retry."));
            }
            ResidueLive(err) => {
                self.rollback_failed = true;
                self.status = Some(format!(
                    "CRITICAL: a save failed and could not be reverted ({err}) — unsaved data is in \
                     memory. Quit the editor NOW (the vault on disk is unchanged); no in-editor action \
                     will save until you quit, then re-run the operation via the CLI."));
                self.close_all_mutation_surfaces(); // set every flow/modal Option to None
            }
        }
    }
}
// each of the 8 Enter arms, Err branch:
//   Err(e) => { app.<this_flow's_modal> = None; app.on_persist_error(e); }
```

`close_all_mutation_surfaces()` sets all flow/modal `Option`s to `None` (at most one is ever open, so
this closes the active flow). The `rollback_failed` write lives in exactly this one function, which
D5 unit-tests directly.

### D-Alt — rejected alternatives (recorded so R0 need not re-litigate)

- **SAVEPOINT/ROLLBACK TO** around the mutation: dead. `append_decision`'s `unchecked_transaction()`
  emits `BEGIN DEFERRED`; a `BEGIN` inside a wrapping `SAVEPOINT` errors "cannot start a transaction
  within a transaction" (verified live). Viable only by modifying `btctax-core`'s shared appender —
  out of proportion for this cycle.
- **Per-row DELETE / logical undo:** must hand-reverse every side-table write a flow makes
  (`persist_void`'s `optimize_attest::clear` first), a "forgot one undo step" bug class, and it is in
  tension with the `ordinal` column that was added specifically to detect tail-delete+reinsert
  (`persistence.rs:104, 330-334`). Trades trivial runtime for real correctness surface. Rejected.
- **Deferring the append's commit until save succeeds:** requires holding a `Transaction<'conn>`
  across `Session::save`'s `&mut self`, a cross-crate refactor of `append_decision`. Rejected.

---

## D5 — KATs (test plan)

TDD-red first, then implementation, then green. Full validation suite green at every step.

**Store (`btctax-store`):** `snapshot`/`restore` round-trip; `restore` leaves the vault file bytes
unchanged on disk; restore into a mutated conn reverts it.

**CLI (`btctax-cli`):** `Session::snapshot`/`restore` wrappers revert an appended decision in-memory
(assert `load_all_ordered` count reverts).

**`edit/persist.rs` — per-fn rollback KATs (`#[cfg(unix)]`, chmod-0o500-parent technique, root-skip
guard):** for a representative decision fn (`reclassify_outflow`) — induce save failure; assert
`load_all_ordered(conn) == pre` (residue gone) AND on-disk bytes unchanged; restore perms; retry;
assert `post == pre + 1` with the **same** `decision_seq` and NO `DecisionConflict`. Plus:
- **`persist_void` side-table revert (the critical one):** seed a `LotSelection` + a populated
  `optimize_attestation` row; induce a `persist_void` save failure; assert `optimize_attest::get`
  for the disposal is STILL present (the pre-save `clear` was reverted by the whole-DB restore).
- **`persist_donation_details` revert:** induce failure; assert `donation_details::get` reverts to
  the prior value/`None`.

**`main.rs` — rewrite the 4 shipped residue KATs (the highest-attention diff).** Each currently
asserts *two* things: **(1)** the save-failure UX + byte-identical disk (`main.rs:~5133, ~9473`) —
**PRESERVE verbatim**; **(2)** retry `post == pre + 2` + (classify) a `DecisionConflict` —
**the removed behavior → rewrite to `post == pre + 1`, no conflict**. And **ADD the new pin**:
immediately after the failed save and *before* retry, assert `load_all_ordered(conn) == pre`
(residue gone) — this is what actually verifies A′. Flag this rewrite explicitly for the whole-diff
reviewer as an *intentional supersession*, not a regression.

**`main.rs` — the `ResidueLive` producer test [R0-I1] (the load-bearing one).** The single line that
arms the catastrophic-path latch lives in `on_persist_error`; test it directly (it is pure over
`&mut EditorApp` + a hand-built value): call `app.on_persist_error(PersistError::ResidueLive(<any
CliError>))` and assert `app.rollback_failed == true`, the status is the CRITICAL residue string, and
every mutation surface is closed. Complement: `on_persist_error(NoChange(..))` and
`on_persist_error(RolledBack(..))` set the benign "no changes were recorded" status, leave
`rollback_failed == false`, and do NOT close the flow.

**`main.rs` — restore-failure latch consumer KAT.** Mirror the `attest_save_failed` ERRLATCH pattern:
directly set `rollback_failed = true` and assert all nine mutating openers (`p/c/o/r/f/v/s/d/a`)
refuse with the CRITICAL residue status and open no flow. Also assert `residue_latch_status()`
returns the **verbatim attest wording** when `attest_save_failed` is set (regression guard for the
9-guard refactor — the shipped `kat_e2e_attest_errlatch_chmod` must stay green).

**KAT-G1 (inherited):** add `"restore("` to `persist_only_tokens` and to the plant-a-token
self-check; assert no non-test region outside `edit/persist.rs` names `restore(`.

---

## Plan (TDD, phased — each phase's diff to R0/whole-diff green before the next)

**Task 1 — the primitive.** `Vault::snapshot`/`restore` + `Session::snapshot`/`restore` + their
store/cli unit tests. No editor changes yet. (Smallest, lowest-risk; establishes the mechanism.)

**Task 2 — the persist layer.** `PersistError` + `save_or_rollback`; convert the 8 persist fns
(snapshot-at-top, `save_or_rollback`); rewrite the 4 stale doc comments (incl. `persist_tax_profile`'s "do NOT roll
back" header — [R0-I2]); add `"restore("` to KAT-G1;
the per-fn rollback KATs (incl. the `persist_void` side-table revert and `donation_details` revert).

**Task 3 — the editor surface.** `rollback_failed` field + `residue_latch_status` helper (fold the 9
opener guards onto it, attest wording verbatim) + `on_persist_error` handler; the 8 Enter arms
delegate to `on_persist_error`; the `on_persist_error` **producer** test (hand-built `ResidueLive`
arms the latch) + the restore-failure latch **consumer** KAT; **rewrite the 4 shipped residue KATs**
(preserve UX + byte-identical; invert residue to `pre+1`/no-conflict; add the `== pre` post-failure
pin).

**Task 4 — whole-diff review (Phase E) + FOLLOWUPS.** Independent adversarial review of the whole
diff; record the `attest_save_failed`→`rollback_failed` **latch-unification** as a FOLLOWUP (once
A′ soaks, attest can adopt snapshot/restore and even retry in-editor). Verify no non-test region
outside `edit/persist.rs` names `restore(`; verify the 4-KAT supersession is intentional; verify all
8 Enter arms delegate to `on_persist_error` (the sole `rollback_failed` writer).

---

## Out of scope

- **The `attest_save_failed` latch / `persist_safe_harbor_attest`.** Its double-batch is
  unrecoverable; a latent bug in a *new* rollback mechanism must not be able to touch it this cycle.
  Latch-unification (attest adopting snapshot/restore, enabling safe in-editor retry) is filed as a
  FOLLOWUP.
- **The six `tui-edit-hardening` items** (#1/2/3/6/7/8) — their own later cycle, built on this
  baseline.
- **`btctax-core` changes** — none (the SAVEPOINT/deferred-commit routes that would need them are
  rejected above).
- **CLI-surface changes** beyond the additive `Session::snapshot`/`restore` — none; no clap flags,
  so no `schema_mirror` / manual-mirror lockstep.
