# Stale-snapshot latch — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** When a write lands on disk but its re-projection fails, arm a latch that refuses every action which
could derive a filed number from the now-stale in-memory image — and fix the two shipped bypasses of the
sibling residue latch found while designing it.

**Architecture:** One `after_write` helper replaces the `match build_snapshot(session)` block at all 27 write
tails; its testable inner `apply_reprojection` arms `EditorApp.stale_after_write` on failure. A combined
`stale_or_residue_latch_status()` sits beside the shipped `residue_latch_status()`; 25 openers take the
combined one, `execute_defensive_export` and `open_defensive_filing` keep the original (D-7, so the export
route stays reachable). Five payload sites probe `stale_reason()` directly as defence-in-depth. Three
mechanized source guards replace the type-level enforcement that the design deliberately dropped.

**Tech Stack:** Rust 2021, ratatui 0.29, `cargo nextest`. All work is in `crates/btctax-tui-edit`.

**Design of record:** `design/stale-snapshot-latch/DESIGN.md` (r4-folded). Eight review rounds in
`design/stale-snapshot-latch/reviews/`.

## Global Constraints

- Base branch: `main` @ `8dce32a`. Work on `feat/stale-snapshot-latch`.
- **SemVer: PATCH.** `crates/btctax-tui-edit` is a `[[bin]]` with no lib; no published API changes.
- Validation: `make check` (nextest + clippy `-D warnings`, ~21s) must be green at every commit. It does
  **not** cover fmt/msrv/pii-scan/net-isolation — those are CI-only.
- Every fix is **mutation-proven**: after the test passes, revert the production change, confirm the test
  reds, restore. Record the mutation in the test's doc comment.
- Never `git checkout -- <file>` to revert a mutation; use a `cp` backup/restore.
- Copy rule — the arm status must never claim the write's *effect* was verified. Exact fact 1 wording:
  `the write reached disk, but whether it had the intended effect could not be verified, because the ledger
  would not re-project`.
- Marker strings (verbatim):
  - Browse row: `STALE — figures below predate your last write. DO NOT FILE from them.`
  - DefensiveFiling notice: the same sentence plus the remedy sentence.
- Goldens are armed-conditional: **no existing golden may change.** If one does, the change is wrong.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `crates/btctax-tui-edit/src/editor.rs` | `EditorApp` state | +1 field (`stale_after_write`) |
| `crates/btctax-tui-edit/src/main.rs` | latch fns, `after_write`, 27 tails, openers, guards | the bulk |
| `crates/btctax-tui-edit/src/draw_edit.rs` | marker rendering on both screens | reservation + Browse row |
| `crates/btctax-tui-edit/src/defensive_dashboard.rs` | — | untouched |

---

### Task 1: The latch field and the two status functions

**Files:**
- Modify: `crates/btctax-tui-edit/src/editor.rs:299` (after `rollback_failed`)
- Modify: `crates/btctax-tui-edit/src/main.rs:672-690` (`residue_latch_status`)
- Test: `crates/btctax-tui-edit/src/main.rs` test module (after `:10707`)

**Interfaces:**
- Consumes: nothing.
- Produces: `EditorApp.stale_after_write: Option<String>`; `fn stale_reason(&self) -> Option<String>`;
  `fn stale_or_residue_latch_status(&self) -> Option<String>`. `residue_latch_status` is **unchanged**.

- [ ] **Step 1: Write the failing test**

```rust
    /// Precedence: attest > rollback > stale. The three latches carry OPPOSITE remedies —
    /// `attest_save_failed` means the write did NOT land (retry via CLI), `stale_after_write` means it
    /// DID (do not retry) — so emitting the wrong one has a filing consequence.
    ///
    /// Mutation: reorder the `stale_after_write` branch above `rollback_failed` → reds on case 3.
    #[test]
    fn stale_or_residue_latch_status_precedence_is_attest_then_rollback_then_stale() {
        let (mut app, _dir) = app_with_unlocked_vault();

        assert_eq!(app.stale_or_residue_latch_status(), None, "clean app must not refuse");

        app.stale_after_write = Some("boom".to_string());
        let s = app.stale_or_residue_latch_status().expect("stale must refuse");
        assert!(s.contains("could not be verified"), "stale copy must not claim an effect: {s}");
        assert!(app.residue_latch_status().is_none(), "the ORIGINAL fn must ignore the stale latch");

        app.rollback_failed = true;
        let s = app.stale_or_residue_latch_status().expect("rollback must refuse");
        assert!(s.starts_with("CRITICAL"), "rollback outranks stale: {s}");

        app.attest_save_failed = true;
        let s = app.stale_or_residue_latch_status().expect("attest must refuse");
        assert!(s.contains("btctax reconcile safe-harbor-attest"), "attest outranks all: {s}");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p btctax-tui-edit stale_or_residue_latch_status_precedence`
Expected: FAIL — `no method named stale_or_residue_latch_status`.

- [ ] **Step 3: Write minimal implementation**

In `editor.rs`, after `pub rollback_failed: bool,` (`:299`):

```rust
    /// Third latch [stale-snapshot]: set ONLY by `apply_reprojection` when a write LANDED on disk but
    /// its follow-up `build_snapshot` failed, so `snapshot` holds the PRE-write image. Carries the
    /// reason (a bare `bool` cannot — the `CliError` is gone by then). Unlike its two siblings the
    /// write DID land, so the remedy is the opposite: do NOT retry. Cleared by a later SUCCESSFUL
    /// re-projection (D-4) — in practice only `execute_defensive_export`'s inline rebuild.
    pub stale_after_write: Option<String>,
```

Add `stale_after_write: None,` to the initializer at `editor.rs:320`.

In `main.rs`, immediately after `residue_latch_status` (`:690`):

```rust
    /// The stale-latch reason alone, as an owned `String` so the borrow ends at the call — the payload
    /// sites need `&mut` on a flow field immediately afterwards, which a `&self`-borrowing accessor
    /// would forbid (E0502).
    fn stale_reason(&self) -> Option<String> {
        self.stale_after_write.as_ref().map(|e| {
            format!(
                "refused: the write reached disk, but whether it had the intended effect could not be \
                 verified, because the ledger would not re-project ({e}). Quit and reopen the vault."
            )
        })
    }

    /// Every mutating opener refuses through this. Precedence attest > rollback > stale: the first two
    /// mean the write did NOT land (retry), the third means it DID (do not retry).
    /// `residue_latch_status` is deliberately left alone — `execute_defensive_export` and
    /// `open_defensive_filing` keep it so D-7's export route stays reachable while stale.
    fn stale_or_residue_latch_status(&self) -> Option<String> {
        self.residue_latch_status().or_else(|| self.stale_reason())
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run -p btctax-tui-edit stale_or_residue_latch_status_precedence`
Expected: PASS. Then `make check` — green.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/editor.rs crates/btctax-tui-edit/src/main.rs
git commit -m "feat(latch): add the stale_after_write latch and its combined status fn"
```

---

### Task 2: Fix both shipped residue-latch bypasses (D-6)

**Files:**
- Modify: `crates/btctax-tui-edit/src/main.rs:719-764` (`close_all_mutation_surfaces`), `:712` (caller),
  `:8185` (`open_pseudo_approve_flow`), `:8640` (`open_bulk_income_modal`)
- Test: `crates/btctax-tui-edit/src/main.rs` test module

**Interfaces:**
- Consumes: `stale_or_residue_latch_status` (Task 1).
- Produces: `fn close_all_mutation_surfaces(&mut self, may_save: bool)` — **signature change**, one
  existing caller updated to `close_all_mutation_surfaces(false)`.

- [ ] **Step 1: Write the failing tests**

```rust
    /// ★ SHIPPED CRITICAL, chain A. `close_all_mutation_surfaces` omitted `bulk_income_flow`, so a
    /// `ResidueLive` from the bulk-income modal left the FLOW alive; `handle_key:327` dispatched to it
    /// and `open_bulk_income_modal` (`:8640`) re-opened the modal with no latch check, letting the
    /// filer save again under a banner promising no save could occur.
    ///
    /// Mutation: remove `bulk_income_flow` from `close_all_mutation_surfaces` → the flow survives → reds.
    #[test]
    fn residue_latch_cannot_be_re_entered_through_the_surviving_bulk_income_flow() {
        let (mut app, _dir) = app_with_unlocked_vault();
        app.bulk_income_flow = Some(bulk_income_flow_fixture());
        app.on_persist_error(crate::edit::persist::PersistError::ResidueLive(
            btctax_cli::CliError::Other("disk".into()),
        ));
        assert!(app.rollback_failed, "the latch must arm");
        assert!(app.bulk_income_flow.is_none(), "chain A: the flow must not survive the latch");
        assert!(app.bulk_income_modal.is_none());
        assert!(app.tax_inputs_form.is_none());
    }

    /// ★ SHIPPED CRITICAL, chain B. `open_pseudo_approve_flow` had NO latch check at all, so `P` on
    /// Browse opened the modal and its Enter called `persist_bulk_decisions` — a save while the residue
    /// latch was armed, needing no surviving surface. The shipped opener KAT loops only 9 of 25 keys,
    /// which is why it was never caught.
    ///
    /// Mutation: delete the guard at `open_pseudo_approve_flow` → the modal opens → reds.
    #[test]
    fn pseudo_approve_opener_refuses_while_the_residue_latch_is_armed() {
        let (mut app, _dir) = app_with_unlocked_vault();
        app.rollback_failed = true;
        app.status = None;
        open_pseudo_approve_flow(&mut app);
        assert!(app.pseudo_approve_modal.is_none(), "chain B: P must not open a save surface");
        let s = app.status.clone().unwrap_or_default();
        assert!(s.starts_with("CRITICAL"), "the refusal must name the reason: {s}");
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo nextest run -p btctax-tui-edit residue_latch_cannot_be_re_entered pseudo_approve_opener_refuses`
Expected: both FAIL — the flow survives; the modal opens.

- [ ] **Step 3: Implement**

`close_all_mutation_surfaces` — change the signature and add the three missing fields before the closing
brace (`main.rs:763`):

```rust
    /// Close every mutating flow/modal. `may_save` gates the tax-inputs draft flush: the `ResidueLive`
    /// caller passes FALSE because that latch means `session.restore` failed, so an un-revertable
    /// residue is live in memory and ANY `Session::save` — including a draft flush — would persist it
    /// under a status promising the vault is unchanged. `apply_reprojection`'s stale path passes TRUE
    /// (memory == disk there: the persist succeeded). `may_save = true` is fail-safe, not an active
    /// flush: `handle_key:377` dispatches `tax_inputs_form` first, so no other tail can run with a
    /// live dirty form.
    fn close_all_mutation_surfaces(&mut self, may_save: bool) {
        if may_save {
            self.flush_tax_inputs_draft();
        }
        // … existing 44 assignments unchanged …
        self.tax_inputs_form = None;
        self.bulk_income_flow = None;
        self.bulk_income_modal = None;
    }
```

Update the sole existing caller at `:712` to `self.close_all_mutation_surfaces(false);`.

At the top of `open_pseudo_approve_flow` (`:8185`) and of the modal re-open at `:8640`, insert the shipped
opener idiom:

```rust
    if let Some(s) = app.stale_or_residue_latch_status() {
        app.status = Some(s);
        return;
    }
```

- [ ] **Step 4: Verify pass + mutation-prove**

Run: `cargo nextest run -p btctax-tui-edit residue_latch_cannot_be_re_entered pseudo_approve_opener_refuses`
Expected: PASS. Then for each: `cp main.rs /tmp/bak`, revert the one production line, re-run (expect RED),
`cp /tmp/bak main.rs`. Then `make check`.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/main.rs
git commit -m "fix(latch): close both shipped residue-latch bypasses (chains A and B)"
```

---

### Task 3: `after_write` / `apply_reprojection`

**Files:**
- Modify: `crates/btctax-tui-edit/src/main.rs` (new methods in the `impl EditorApp` block at `:668`)
- Test: `crates/btctax-tui-edit/src/main.rs` test module

**Interfaces:**
- Consumes: `stale_after_write` (Task 1), `close_all_mutation_surfaces(bool)` (Task 2).
- Produces: `fn after_write(&mut self, status: impl FnOnce(&Snapshot) -> String)` and
  `fn apply_reprojection(&mut self, rebuilt: Option<Result<(Snapshot, i32), CliError>>, status: impl FnOnce(&Snapshot) -> String)`.

- [ ] **Step 1: Write the failing tests**

```rust
    /// The seam that makes the arm path testable at all: the failure is an ARGUMENT, so no corrupt-vault
    /// fixture and no `cfg(test)` hatch is needed. Before this, zero tests exercised the `Err` arm of any
    /// of the 27 write tails.
    ///
    /// Mutation: drop the `close_all_mutation_surfaces` call from the Err arm → the flow survives → reds.
    #[test]
    fn apply_reprojection_err_arms_the_latch_closes_surfaces_and_never_claims_an_effect() {
        let (mut app, _dir) = app_with_unlocked_vault();
        app.declare_flow = Some(declare_flow_fixture());
        app.apply_reprojection(
            Some(Err(btctax_cli::CliError::Other("projector said no".into()))),
            |_| unreachable!("the status closure must NOT run on the Err arm"),
        );
        let s = app.status.clone().unwrap_or_default();
        assert!(app.stale_after_write.is_some(), "the latch must arm");
        assert!(s.contains("reached disk"), "fact 1 must say the write landed: {s}");
        assert!(s.contains("could not be verified"), "fact 1 must NOT claim the effect: {s}");
        assert!(!s.contains("is correct"), "fact 1 must never assert correctness: {s}");
        assert!(app.declare_flow.is_none(), "surfaces must be closed");
    }

    /// The Ok arm computes the status FROM the rebuilt snapshot (19 of the 27 tails need it), clears the
    /// latch (D-4), and refreshes the defensive dashboard ONLY if one is already open —
    /// `refresh_defensive_dashboard` CREATES state unconditionally, so an unguarded call would fabricate
    /// a wizard dashboard for a filer who never opened it.
    ///
    /// Mutation: drop the `defensive_dashboard.is_some()` guard → a dashboard materialises → reds.
    #[test]
    fn apply_reprojection_ok_derives_the_status_and_does_not_fabricate_a_dashboard() {
        let (mut app, _dir) = app_with_unlocked_vault();
        app.stale_after_write = Some("earlier".to_string());
        app.defensive_dashboard = None;
        let rebuilt = btctax_tui::unlock::build_snapshot(app.session.as_ref().unwrap());
        app.apply_reprojection(Some(rebuilt), |snap| format!("{} events", snap.events.len()));
        assert!(app.status.clone().unwrap_or_default().ends_with("events"));
        assert!(app.stale_after_write.is_none(), "a successful re-projection clears the latch (D-4)");
        assert!(app.defensive_dashboard.is_none(), "must not fabricate a dashboard");
    }

    /// Fail-closed: no session is production-unreachable (`session` is `Some` iff `snapshot` is), but it
    /// must arm rather than panic, so a future refactor cannot turn it into an `.unwrap()`.
    #[test]
    fn apply_reprojection_with_no_session_arms_rather_than_panicking() {
        let (mut app, _dir) = app_with_unlocked_vault();
        app.apply_reprojection(None, |_| "unused".to_string());
        assert!(app.stale_after_write.is_some());
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo nextest run -p btctax-tui-edit apply_reprojection`
Expected: FAIL — `no method named apply_reprojection`.

- [ ] **Step 3: Implement**

```rust
    /// THE write tail. All 27 sites call this and nothing else.
    fn after_write(&mut self, status: impl FnOnce(&Snapshot) -> String) {
        let rebuilt = self.session.as_ref().map(btctax_tui::unlock::build_snapshot);
        self.apply_reprojection(rebuilt, status)
    }

    /// The testable unit: `rebuilt` is an argument so every KAT drives the arm path directly.
    /// `status` is a CLOSURE because 19 of the 27 tails derive their success text from the freshly
    /// rebuilt snapshot; an `impl Into<String>` cannot express them.
    fn apply_reprojection(
        &mut self,
        rebuilt: Option<Result<(Snapshot, i32), btctax_cli::CliError>>,
        status: impl FnOnce(&Snapshot) -> String,
    ) {
        match rebuilt {
            Some(Ok((snap, _))) => {
                let s = status(&snap);
                self.snapshot = Some(snap);
                self.status = Some(s);
                // D-4: a successful re-projection falsifies the latch's premise. Fail-safe only —
                // no tail can run while armed (every one is behind a closed surface + a refusing
                // opener); the reachable clear is `execute_defensive_export`'s inline rebuild.
                self.stale_after_write = None;
                if self.defensive_dashboard.is_some() {
                    refresh_defensive_dashboard(self);
                }
            }
            Some(Err(e)) => self.arm_stale(e.to_string()),
            None => self.arm_stale("no session".to_string()),
        }
    }

    /// The ONE arm site.
    fn arm_stale(&mut self, reason: String) {
        self.stale_after_write = Some(reason.clone());
        self.status = Some(format!(
            "the write reached disk, but whether it had the intended effect could not be verified, \
             because the ledger would not re-project ({reason}). Quit and reopen the vault."
        ));
        self.close_all_mutation_surfaces(true);
    }
```

- [ ] **Step 4: Verify pass + mutation-prove both mutations named above.** Then `make check`.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/main.rs
git commit -m "feat(latch): add after_write/apply_reprojection with the failure as a parameter"
```

---

### Task 4: Migrate all 27 write tails

**Files:**
- Modify: `crates/btctax-tui-edit/src/main.rs` at the 27 sites
- Test: `crates/btctax-tui-edit/src/main.rs` test module

**Interfaces:**
- Consumes: `after_write` (Task 3). Produces: no new API; the literal
  `"Saved but re-projection failed"` disappears from the crate.

**The 27, by axis** (do not use "calls a `derive_*`" as a rule — `:5130` and `:9199` call `derive_*` fns that
take no snapshot):
- **snapshot-derived (19):** `1661, 2241, 2588, 2660, 3197, 4809, 6147, 6716, 7032, 7303, 7787, 8153, 8253,
  8679, 8968, 9543, 9747, 10031, 10375`
- **pure literal (7):** `559, 1530, 2861, 4217, 4498, 5130, 9199`
- **status hoisted (1):** `1347`

- [ ] **Step 1: Write the failing test**

```rust
    /// Representative end-to-end coverage across all three axes — draft 3 picked only literal-status
    /// sites, which would have left the hard case untested.
    #[test]
    fn representative_tails_keep_their_shipped_status_text_after_the_migration() {
        // snapshot-derived: safe-harbor attest (:7032), the highest-stakes write in the editor.
        let (mut app, _dir) = vault_with_safe_harbor_allocation();
        confirm_safe_harbor_attest(&mut app);
        assert!(app.status.clone().unwrap_or_default().contains("attest"));
        // no-snapshot derive_*: bulk void (:9199).
        let (mut app2, _d2) = vault_with_voidable_decisions();
        confirm_bulk_void(&mut app2);
        assert!(app2.status.is_some());
        // hoisted: commit_tax_inputs (:1347) — the confirmation must still appear.
        let (mut app3, _d3) = vault_with_return_inputs_draft();
        commit_tax_inputs(&mut app3);
        assert!(
            app3.status.clone().unwrap_or_default().contains("committed"),
            "the hoisted-status site must not be blanked by the migration"
        );
    }
```

- [ ] **Step 2: Run to verify it fails** (the hoisted case reds once `:1411` overwrites the tail's status).

Run: `cargo nextest run -p btctax-tui-edit representative_tails_keep_their_shipped_status_text`

- [ ] **Step 3: Implement — the three shapes**

Literal (e.g. `:559`):

```rust
    app.after_write(|_| format!("Saved tax profile for {year}"));
```

Snapshot-derived (e.g. `:2660`):

```rust
    app.after_write(|snap| derive_set_fmv_status(snap, &event_id, fmv));
```

Hoisted — restructure `commit_tax_inputs` (`:1332-1411`) so the tail owns the status: delete the
`let status: Option<String> = …` binding and the unconditional `app.status = status;` at `:1411`, and move
each arm's text into the `after_write` closure.

**Park tail (`:1530`) ordering:** insert the `after_write` call **after** the `:1532-1537` form block, not at
the current `build_snapshot` position which precedes it — otherwise `close_all(may_save = true)` runs before
`form.dirty = false` (`:1536`) and the flush stops being a no-op.

Preserve the three bespoke prefixes, with the attest one reworded (it claimed an *effect*
`derive_attest_status` exists to deny):
- `:1355` → `"committed {year} as {label}, but …"`
- `:1547` → `"parked the full return for {year}, but …"`
- `:7042` → `"the safe-harbor attest write landed, but …"`

- [ ] **Step 4: Verify**

Run: `make check` — all 2420 existing tests must stay green (this migration is behaviour-preserving on the
`Ok` path). Then `grep -c "Saved but re-projection failed" crates/btctax-tui-edit/src/main.rs` → expect the
two doc-comment hits only.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/main.rs
git commit -m "refactor(latch): route all 27 write tails through after_write"
```

---

### Task 5: The five payload probes and their gating KATs

**Files:**
- Modify: `crates/btctax-tui-edit/src/main.rs:4116-4124`, `:4172-4188`, `:4319-4328`, `:4367-4375`,
  `:8209-8227`
- Test: `crates/btctax-tui-edit/src/main.rs` test module

**Interfaces:** Consumes `stale_reason()` (Task 1). Produces no new API.

- [ ] **Step 1: Write the failing tests**

```rust
    /// ★ GATING (D-5). The filing-correctness property: with the latch armed, NO decision payload of any
    /// kind is constructible. Three constructors, because all three read `app.snapshot`:
    /// `plan_promote` (:4367 — builds `Acknowledgment{shown_terms}` AND `filed_basis`, which becomes the
    /// promoted lot's basis in Form 8949 col (e)), `plan_declare` (:4172 — appended verbatim by
    /// `apply_declare` with no `would_conflict` re-check), and `pseudo_plan` (:8209 — its own doc says
    /// "RE-derived here from the held snapshot").
    ///
    /// These KATs, not the compiler, are the guarantee at these sites (the privatization was cut at r3
    /// because it cannot be compiler-enforced where a `&mut` flow borrow is also needed). Defence in
    /// depth: the primary line is close_all + the opener guards.
    ///
    /// Mutation: delete any one probe → that constructor runs while armed → reds.
    #[test]
    fn no_decision_payload_is_constructible_while_the_stale_latch_is_armed() {
        // promote
        let (mut app, _d) = vault_at_promote_consent();
        app.stale_after_write = Some("boom".into());
        promote_flow_review(&mut app);
        assert!(!promote_flow_reached_consent(&app), "no PromotePlan while armed");
        // declare
        let (mut app2, _d2) = vault_at_declare_confirm();
        app2.stale_after_write = Some("boom".into());
        declare_flow_confirm(&mut app2);
        assert_eq!(live_declare_count(&app2), 0, "no DeclarePlan may be written while armed");
        // pseudo-approve
        let (mut app3, _d3) = vault_with_pending_pseudo_defaults();
        app3.pseudo_approve_modal = Some(pseudo_approve_modal_fixture());
        app3.stale_after_write = Some("boom".into());
        handle_pseudo_approve_modal_key(&mut app3, key(KeyCode::Enter));
        assert_eq!(live_decision_count(&app3), 0, "no pseudo payload set may be built while armed");
    }
```

- [ ] **Step 2: Run to verify it fails.**

Run: `cargo nextest run -p btctax-tui-edit no_decision_payload_is_constructible`

- [ ] **Step 3: Implement** — at the top of each of the five sites, before any `&mut` field borrow:

```rust
    if let Some(s) = app.stale_reason() {
        app.status = Some(s);
        return;
    }
```

The owned return is required: `app.snapshot.as_ref()` and `app.promote_flow.as_mut()` are disjoint field
borrows that compile today, but a `&self`-borrowing accessor held across them is `E0502`.

- [ ] **Step 4: Verify pass + mutation-prove each of the three probes separately.** Then `make check`.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/main.rs
git commit -m "feat(latch): refuse payload construction at all five snapshot-derived sites"
```

---

### Task 6: D-7 — the export route stays reachable

**Files:**
- Modify: 25 openers in `main.rs`; `editor.rs:474` and `:478-490`; `main.rs:4629-4632`
- Test: `crates/btctax-tui-edit/src/main.rs` test module

**Interfaces:** Consumes Tasks 1-2. Produces the latch partition asserted by Task 8's guard.

- [ ] **Step 1: Write the failing test**

```rust
    /// D-7: `execute_defensive_export` re-projects BEFORE planning and refuses on its own failure, so it
    /// provably cannot act on a stale image — and it is the filer's only in-app route to the 8949/8275
    /// packet. `x` is reachable ONLY via `w`, so `open_defensive_filing` must keep the ORIGINAL latch;
    /// routing it to the combined one silently revokes D-7 for 25 of the 27 arming tails.
    ///
    /// Mutation: switch `editor.rs:474` to the combined latch → `w` refuses → reds.
    #[test]
    fn w_then_x_still_exports_while_the_stale_latch_is_armed_from_a_browse_tail() {
        let (mut app, _dir) = vault_with_promoted_2025_leg_and_2024_reorder();
        app.stale_after_write = Some("price cache".to_string());
        app.screen = EditorScreen::Browse;

        app.open_defensive_filing();
        assert_eq!(app.screen, EditorScreen::DefensiveFiling, "D-7: w must open while armed");

        execute_defensive_export(&mut app);
        let s = app.status.clone().unwrap_or_default();
        assert!(s.contains("2 of 2"), "the packet must be written off the fresh rebuild: {s}");
        assert!(app.stale_after_write.is_none(), "a successful re-projection CLEARS the latch (D-4)");
    }

    /// The pseudo-approve tail is the one write that flips `pseudo_active()`, so DFW-D6 evaluated on the
    /// STALE image locks the filer out of the very route D-7 exists to provide.
    ///
    /// Mutation: restore the unconditional DFW-D6 refusal → `w` refuses → reds.
    #[test]
    fn w_opens_while_armed_even_though_the_stale_image_still_says_pseudo_active() {
        let (mut app, _dir) = vault_with_pending_pseudo_defaults();
        approve_all_pseudo_defaults_then_fail_reprojection(&mut app);
        assert!(app.stale_after_write.is_some());
        app.open_defensive_filing();
        assert_eq!(app.screen, EditorScreen::DefensiveFiling);
    }
```

- [ ] **Step 2: Run to verify they fail.**

- [ ] **Step 3: Implement**

- The **25** openers listed in `reviews/design-architecture-opus-review-r3.md` finding 2 switch
  `residue_latch_status()` → `stale_or_residue_latch_status()`.
- `execute_defensive_export` (`:4616`) and `open_defensive_filing` (`editor.rs:474`) keep the original.
- The dashboard's write intents (`:505-510`, `DashboardIntent::Declare`/`Promote`) take the combined latch.
- `open_defensive_filing`'s DFW-D6 refusal (`editor.rs:478-490`) is skipped while
  `self.stale_after_write.is_some()`.
- The export's Ok arm (`:4629-4632`) gets the reachable clear beside its assignment:

```rust
            app.snapshot = Some(snap);
            // D-4's only reachable clear: this rebuild succeeded, so the latch's premise is false.
            app.stale_after_write = None;
            refresh_defensive_dashboard(app);
```

- [ ] **Step 4: Verify pass + mutation-prove both.** Then `make check`.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/main.rs crates/btctax-tui-edit/src/editor.rs
git commit -m "feat(latch): keep the w->dashboard->x export route reachable while stale (D-7)"
```

---

### Task 7: The marker

**Files:**
- Modify: `crates/btctax-tui-edit/src/draw_edit.rs:107-119`, `:152-163`, `:169`, `:183-195`, `:279-294`
- Test: `crates/btctax-tui-edit/src/draw_edit.rs` test module (after `:5944`)

**Interfaces:** Consumes `stale_after_write`. Produces the two marker strings.

- [ ] **Step 1: Write the failing tests**

```rust
    /// From the SECOND keypress onward the marker is the only notice the filer sees — `main.rs:414`
    /// (Browse) and `:494` (DefensiveFiling) clear `app.status` before dispatch. So it must render with
    /// `status == None`, and it must name the CONSEQUENCE, not the cause. Register precedent: the PSEUDO
    /// banner's "FICTIONAL placeholders — DO NOT FILE".
    ///
    /// Mutation: gate the marker on `status.is_some()` → blank → reds.
    #[test]
    fn the_stale_marker_renders_on_both_screens_with_no_status_and_names_the_consequence() {
        let rendered = render_browse_to_string_armed(None, 80);
        assert!(
            rendered.contains("STALE — figures below predate your last write. DO NOT FILE from them."),
            "Browse renders 8949/Sch D figures off the stale snap; the marker must say so: {rendered}"
        );
        let d = render_defensive_filing_to_string_armed(None, 80);
        assert!(d.contains("predate your last write"), "{d}");
    }

    /// The composed arm status is ~300-350 chars; `DEFENSIVE_NOTICE_LINES = 3` gives ~228 usable at 80
    /// columns, and the Browse footer is a single UN-WRAPPED Paragraph that truncates. Without the
    /// resize, fact 3 (the remedy) is invisible at all 27 tails.
    #[test]
    fn the_full_arm_status_renders_unclipped_at_80_columns_on_both_screens() {
        let arm = longest_arm_status();
        let d = render_defensive_filing_to_string_armed(Some(&arm), 80);
        assert!(d.contains("Quit and reopen the vault."), "fact 3 must survive: {d}");
        let b = render_browse_to_string_armed(Some(&arm), 80);
        assert!(b.contains("Quit and reopen the vault."), "fact 3 must survive on Browse: {b}");
    }
```

- [ ] **Step 2: Run to verify they fail.**

- [ ] **Step 3: Implement**

- `draw_edit.rs:109`: reservation predicate becomes `status.is_some() || app.stale_after_write.is_some()`
  — without this there is no rect to render into.
- `:152`: render the fixed marker whenever armed, independent of `status`; when a status is present, both.
- `:169`: raise `DEFENSIVE_NOTICE_LINES` to fit the **measured** longest composed string at 80 columns
  (compute it in the test; do not estimate). Floor the `:111` `saturating_sub(1)` at 1.
- `draw_browse` (`:183-195`): push a state-conditional `Constraint::Length(1)` marker row exactly as the
  PSEUDO banner does, extending the `content_idx`/`footer_idx` bookkeeping; while armed also give the
  status a wrapped multi-row band rather than the truncating footer.

- [ ] **Step 4: Verify.** Run `make check`. **No existing golden may change** — the marker is
  armed-conditional and every committed golden is unarmed. If one changes, the predicate is wrong.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/draw_edit.rs
git commit -m "feat(latch): render the stale marker on both screens, independent of app.status"
```

---

### Task 8: The three mechanized guards

**Files:**
- Test only: `crates/btctax-tui-edit/src/main.rs` test module

**Interfaces:** Consumes everything above. These guards are what replaces the dropped type-level
enforcement, so they are the systemic protection, not a nicety.

- [ ] **Step 1: Write the failing tests**

```rust
    /// Guard (a). `handle_key` dispatches on 47 surfaces; `close_all_mutation_surfaces` cleared 44 — the
    /// gap WAS the shipped Critical. Assert the subset so a 48th surface cannot silently reopen it.
    /// `defensive_dashboard` is excluded deliberately: it is dispatched via `as_mut()` (:501) and every
    /// intent is individually handled — Declare/Promote take the combined latch, Export is exempt by
    /// design (D-7), RouteResolveFirst/None are inert. (NOT because nulling it would break D-7 — Esc→w
    /// rebuilds it.)
    #[test]
    fn every_handle_key_dispatch_surface_is_cleared_by_close_all_mutation_surfaces() { … }

    /// Guard (b). With TWO latch fns, "contains a check" stopped being the property — a silent downgrade
    /// of any combined opener to the original passes a presence test and reopens a filed-number path.
    /// Assert the exact PARTITION, both directions. Domain is every assignment of an `EditorApp` surface
    /// field to `Some(..)` in main.rs + editor.rs — NOT `fn open_*`, which misses the 12 sites that open
    /// a surface from inside a `handle_*_key`.
    ///
    /// Mutation: flip `open_declare_flow` to `residue_latch_status()` → reds.
    #[test]
    fn exactly_two_sites_use_the_original_latch_and_every_other_opener_uses_the_combined_one() { … }

    /// Guard (c). Write-side: `flush_tax_inputs_draft` is a real `Session::save` fired from the IDLE TICK
    /// (`main.rs:10658`), outside `handle_key` entirely — no key-scoped guard can ever see it.
    #[test]
    fn flush_tax_inputs_draft_refuses_under_either_save_forbidding_latch() { … }

    /// Tail-class guard. A `build_snapshot(` grep is unsound: `:4626` already spells it
    /// `.map(btctax_tui::unlock::build_snapshot)` with NO parens — the same escape by which :1347/:1530/
    /// :7032 evaded the literal-string count. Token WITHOUT the paren, two-entry allowlist, over
    /// non-test comment-stripped lines of main.rs + editor.rs. Follow the DE-STUCK scanner at
    /// `main.rs:16229-16259`, not the sticky `edit/persist.rs:2090-2104`.
    #[test]
    fn build_snapshot_is_named_only_by_after_write_and_the_export() { … }
```

- [ ] **Step 2: Run to verify they fail** (write each body fully before running; the sketch above shows the
      doc comments and names, and each body is a source scan in the shape of `main.rs:16229-16259`).

- [ ] **Step 3: Implement** — extend `kat_rollback_failed_latch_refuses_all_openers` (`:21381`) from 9 keys
      to **25** for `rollback_failed` and **24** (minus `w`) for the stale latch; Task 6's positive `w`/`x`
      KAT covers the difference.

- [ ] **Step 4: Verify.** `make check`, then mutation-prove guard (b) by flipping `open_declare_flow`.

- [ ] **Step 5: Commit**

```bash
git add crates/btctax-tui-edit/src/main.rs
git commit -m "test(latch): mechanize the dispatch-subset, latch-partition and write-side guards"
```

---

## Self-Review

**Spec coverage:** §0.1 five payload sites → T5. §0.2 27 tails → T4. §0.3 both chains → T2. D-1/D-2 → T7.
D-3 → T4. D-4 clears → T3 (fail-safe) + T6 (reachable). D-5 gating KATs → T5. D-6 → T2. D-7 → T6.
§2.3 steps 1-3 → T2, T6. §2.4 marker → T7. §2.5 copy → T3 (fact 1), T4 (prefixes, park ordering).
§3.5 guards → T8. §3.6 confinement → T8. No spec section is unassigned.

**Placeholders:** Task 8's four test bodies are given as names + doc comments + the scanner to copy
(`main.rs:16229-16259`) rather than full source — they are mechanical source scans whose exact text depends
on the final line numbers after Tasks 1-7. Every other step carries real code. Flagged rather than hidden.

**Type consistency:** `after_write(impl FnOnce(&Snapshot) -> String)` and
`apply_reprojection(Option<Result<(Snapshot, i32), CliError>>, impl FnOnce(&Snapshot) -> String)` are used
identically in T3, T4. `stale_reason() -> Option<String>` (T1) is what T5 calls.
`close_all_mutation_surfaces(bool)` (T2) is what T3's `arm_stale` calls. `stale_after_write: Option<String>`
throughout.
