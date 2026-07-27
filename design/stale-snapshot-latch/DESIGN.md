# Stale-snapshot latch — adjudicated design (post-merge Cycle 1)

**Status:** **r4-folded — BUILD-READY.** Rounds: arch r1 1C/6I → r2 0C/6I → r3 2C/6I → **r4 0C/5I**;
tax r1 2C/4I → r2 1C/5I → **r3 0C/3I**. All eight persisted verbatim in `reviews/`. Both lenses are now
**0 Critical**; the r4/r3 Importants are folded below. Per the owner's minimal-fix operating rule, the design
proceeds to implementation rather than to a further review round; the residual Minors/Nits are recorded in
the reviews and carried as post-build follow-ups.
**Base:** `main` @ `8dce32a` (v0.10.0). Baseline: `make check` 2420 passed / 11 skipped.
**Lineage:** Cycle 1 of the Defensive-Filing-wizard post-merge burndown (**PM-1..PM-4**) + **two shipped
Criticals** found in review (§0.3).

> **★ Scope cut at r3 (owner decision, superseding D-5).** Drafts 1-3 layered a ~176-site privatization of
> `EditorApp.snapshot` on top of the fix, to make the staleness check compiler-enforced. Review r3 proved it
> **cannot** be compiler-enforced at the four sites that motivated it: `live_snapshot(&self)` borrows all of
> `self`, and those bodies need `self.{declare,promote}_flow.as_mut()` → `E0502`. The guarantee there is a
> KAT either way. The privatization is therefore **dropped**, along with everything that existed only to
> serve it (both render accessors, the `#[cfg(test)]` seam and its scanner hazard, the ~111-site test
> migration, the `build_snapshot` re-export). This design is now the defect and nothing else.

## 0. The defect

### 0.1 It is a FILING-CORRECTNESS latch (tax r1 C-1, refined by tax r2)

The registry (`FOLLOWUPS.md:263-264, 371-373`), the recon and drafts 1-2 all asserted that "both confirm
tails re-run their own FRESH `plan_declare`/`plan_promote` against `session`". **False** — and it is the
justification on which PM-1/PM-2 were deferred past the merge gate.

Four sites build a payload or a preview from `app.snapshot`; none reads `session`:

| site | builds | reaches a filed form |
|---|---|---|
| `main.rs:4367-4375` `promote_flow_review` | the `PromotePlan`, incl. `Acknowledgment{shown_terms}` | **`filed_basis`/`coverage`** → the promoted lot's basis → **Form 8949 col (e)** (`chokepoint/mod.rs:348-354`, written `:409-410`) |
| `main.rs:4172-4188` `declare_flow_confirm` | the `DeclarePlan`, appended verbatim by `apply_declare` with **no** `would_conflict` re-check (`chokepoint/mod.rs:601-609`) | the tranche's synthetic acquisition → 8949 basis for the covered disposal |
| `main.rs:4319-4328` `attest_provenance` | a `plan_promote` on the always-refused non-`Purchase` path | — |
| `main.rs:4116-4124` declare-flow `t` | the on-demand tax-Δ preview | — (display) |
| `main.rs:8209-8227` **`handle_pseudo_approve_modal_key`** | `pseudo_plan(&snap.events, …)` → a `Vec<EventPayload>` appended verbatim by `persist_bulk_decisions` (`:8242`); its own doc says *"the plan is RE-derived here from the held snapshot"* (`:8201-8202`) | the approved synthetic defaults' FMV/kind → Schedule 1 income and 8949 basis |

**Five sites, not four** (arch r4 I-1 / tax r3 I-1; drafts 1-4 said four). **And the honest characterisation
of what guarantees the property:** every one of the five sits behind a surface `close_all_mutation_surfaces`
clears *and* an opener that refuses, so the §2.3 step 4 probes are unreachable in production. The primary
guarantee is **§2.3 steps 1-3 plus the §3.5 guards**; the probes and the §3.3 KATs are defence-in-depth —
which is what the dropped privatization would have bought here anyway.

`promote_flow_review` is the sole payload-bearing `PromotePlan` constructor (`PromoteFlowStep::Consent` is
built only at `edit/promote_flow.rs:210`, reached only from `main.rs:4367-4376`).

**`declare_flow_confirm`'s gates also run against the stale image** — `guard_tranche_vs_allocation`
(`chokepoint/mod.rs:537`) and the DFW-D5.2 clearance shadow (`:559-591`) re-project `events + candidate`
from stale `events`, so a tranche the current ledger would refuse can land.

**Two corrections to earlier drafts.** (a) `shown_terms` does not itself reach a filed form — Form 8275
renders the filer-authored `part_ii_narrative` (`btctax-core/src/tax/form8275.rs:86-96` →
`btctax-forms/src/form8275.rs:123`). It remains the §6664(c) record and is still corrupted by a stale
image; the number that reaches a *return* is `filed_basis`. (b) Draft 2's bulk-income `fmv` claim is wrong:
`bulk_income_recompute_preview` (`main.rs:8504-8550`) calls `session.bulk_classify_income_plan`, which
re-loads and re-projects from the session (`session.rs:947`). Only the opener's filter comes from `snap`,
and a stale filter yields an empty plan, not a wrong amount.

### 0.2 The stale-snapshot class — 27 write tails

From `build_snapshot(` over production lines (NR ≤ 10706) — the class predicate, not the copy string:

`559, 1347, 1530, 1661, 2241, 2588, 2660, 2861, 3197, 4217, 4498, 4809, 5130, 6147, 6716, 7032, 7303,
7787, 8153, 8253, 8679, 8968, 9199, 9543, 9747, 10031, 10375`

24 share the literal `"Saved but re-projection failed …"`; three do not (`:1347` commit, `:1530` park,
`:7032` attest). The FOLLOWUP's "26" counted its own doc comments; draft 1's "24" counted the copy string.
The export's pre-plan re-projection is not a member — `.map(btctax_tui::unlock::build_snapshot)`, no parens
(`:4626`); its `:4630` assignment is the 28th and only non-tail `app.snapshot = Some(snap)`.

**No test holds any of it:** zero tests assert the `Err` arm of any of the 27.

### 0.3 Two reachable bypasses of the SHIPPED residue latch

`handle_key` (`:126-386`) dispatches on **47** surfaces; `close_all_mutation_surfaces` (`:719-764`) clears
**44**; the difference is exactly `{tax_inputs_form, bulk_income_flow, bulk_income_modal}`.
`on_persist_error`'s `ResidueLive` arm (`:705-712`) depends on that call to honour *"no in-editor action
will save until you quit"*.

**Chain A.** Bulk-income modal `Enter` → `ResidueLive`; `:8696-8698` nulls only the modal; `close_all`
leaves `bulk_income_flow`; `handle_key:327` dispatches to it; `:8640` re-opens the modal with no latch
check; `Enter` saves again.

**Chain B.** `open_pseudo_approve_flow` (`:8185-8198`) has **no** latch check — the only `fn open_*_flow`
without one. `P` opens the modal; its `Enter` (`:8242`) calls `persist_bulk_decisions`. No surviving surface
needed. The shipped KAT `kat_rollback_failed_latch_refuses_all_openers` (`:21381-21414`) loops 9 of the 25
Browse opener keys, which is why `P` was never caught.

Both are **Critical against shipped v0.10.0**.

*Correction to arch r1:* its `tax_inputs_form` route cannot arm the **shipped** latch
(`handle_tax_inputs_key` never calls `on_persist_error`; those persist fns return `CliError` —
`edit/persist.rs:128`, `:183`, `:205`). It is reachable only for the **new** latch, via the park tail.

## 1. Owner decisions

- **D-1 Hard latch.** While armed, every action deriving a filed number, a decision payload or a §6664(c)
  record refuses. Scope set by D-7.
- **D-2 Render the stale image, marked** (§2.4).
- **D-3 All 27 tails** route through one helper.
- **D-4 (amended r3, corrected r4) One arm site, two clear sites — only one of them reachable.** A
  successful re-projection falsifies the latch's premise and clears it. **The reachable clear is the
  export's own inline rebuild** (`main.rs:4629-4632`), which gets an explicit `stale_after_write = None`
  beside its `app.snapshot = Some(snap)`. The clear in `apply_reprojection`'s `Ok` arm is **fail-safe only**
  and can never fire in production: all 27 tails sit behind surfaces `close_all` clears and openers that
  refuse, so no tail runs while armed (arch r4 I-2). The export cannot be routed through
  `apply_reprojection` — that helper's `Err` arm *arms* and calls `close_all`, whereas D-7 requires "Err ⇒
  refuse, latch unchanged" (`:4634-4638`), and its `Ok` arm sets `app.status`, which the export must not do
  (its status comes from `persist_defensive_export`, `:4685`).
- **D-5 (REPLACED at r3 — scope cut).** No privatization. The staleness check at the four payload sites is
  an **explicit `stale_reason()` probe**, and its guarantee is the **§3.3 KATs, which are GATING**. Systemic
  protection comes from the three mechanized source guards (§3.5) rather than the type system — which is
  what the privatization would have delivered at these sites anyway.
- **D-6 Both shipped Criticals are fixed in this cycle**, each with its own KAT and changelog line.
- **D-7 The export ROUTE is exempt.** `execute_defensive_export` re-projects before planning and refuses on
  its own failure (`:4614-4640`), so it provably cannot act on a stale image, and it is the filer's route to
  the packet. `x` is reachable only via `w` → `open_defensive_filing`, so **that opener keeps the original
  `residue_latch_status()`** (arch r3 C-2 / tax r2 C-1). `main.rs:4606-4610` stands unamended.

## 2. Architecture

Four pieces, all in `crates/btctax-tui-edit/`. `EditorApp.snapshot` stays `pub` and every existing reader is
untouched.

### 2.1 State (`editor.rs`)

```rust
/// Armed by `apply_reprojection` when a write LANDED on disk but its re-projection failed; carries
/// the reason so the refusals and the marker can name it. Cleared by a later SUCCESSFUL
/// re-projection (D-4) — today only the export performs one while armed.
pub stale_after_write: Option<String>,
```

Sibling of the shipped `rollback_failed` (`editor.rs:299`) / `attest_save_failed` pair. `snapshot: None`
keeps its existing meaning (never unlocked; a documented silent no-op, `editor.rs:462`); staleness is a
separate axis.

### 2.2 The write tail

```rust
/// THE write tail. All 27 sites call this and nothing else.
fn after_write(&mut self, status: impl FnOnce(&Snapshot) -> String) {
    let rebuilt = self.session.as_ref().map(btctax_tui::unlock::build_snapshot);
    self.apply_reprojection(rebuilt, status)
}

/// The testable unit: the failure is an argument, so every KAT drives the arm path directly —
/// no `cfg(test)` hatch, no corrupt-vault fixture.
fn apply_reprojection(
    &mut self,
    rebuilt: Option<Result<(Snapshot, i32), CliError>>,
    status: impl FnOnce(&Snapshot) -> String,
) { … }
```

The closure is required — 19 of the 27 derive their status from the rebuilt snapshot. Verified buildable at
all 27: no `Ok` arm reads an `app.*` field while computing its status.

**Site taxonomy — measured, 19 / 7 / 1 = 27** (arch r3 I-8):
- **snapshot-derived (19):** `1661, 2241, 2588, 2660, 3197, 4809, 6147, 6716, 7032, 7303, 7787, 8153, 8253,
  8679, 8968, 9543, 9747, 10031, 10375`.
- **pure literal (7):** `559, 1530, 2861, 4217, 4498, 5130, 9199`. `:5130` and `:9199` call `derive_*` fns
  that take **no** snapshot (`derive_donation_details_status` `:5789`; `derive_bulk_void_status` `:9232`),
  so "calls a `derive_*` ⇒ snapshot-derived" is **not** a safe mechanical rule.
- **status hoisted (1):** `:1347` `commit_tax_inputs` computes `let status: Option<String> = match outcome`
  and assigns unconditionally at `:1411`, which would overwrite whatever the tail set (with `None` if the
  arm yields `None`) — the filer commits a full return and sees no confirmation. `:1332-1411` is
  restructured so the tail owns the status. Verified the only such site.

Arms:
- `Some(Ok((snap, _)))` → compute `status(&snap)`; set snapshot and status; **clear `stale_after_write`
  (D-4)**; refresh the defensive dashboard **only if one is already open** — unguarded,
  `refresh_defensive_dashboard` (`:4562-4579`) *creates* state and would fabricate a dashboard for a filer
  who never opened one.
- `Some(Err(e))` → arm; set the prefixed arm status; `close_all_mutation_surfaces(may_save = true)`.
- `None` (no session) → arm. Production-unreachable (`editor.rs:93`) but fail-closed.

### 2.3 The refusal surface

1. **Repair `close_all_mutation_surfaces`** — add `tax_inputs_form`, `bulk_income_flow`,
   `bulk_income_modal` (§0.3 chain A).
   **★ The flush is caller-conditional** (arch r3 C-1 / tax r2 I-4). Draft 2's unconditional "flush first"
   would have written the unrevertable residue to disk: `close_all`'s only shipped caller is
   `on_persist_error`'s `ResidueLive` arm, which fires when `session.restore(pre)` **failed**
   (`edit/persist.rs:219-222`), and the flush chain (`flush_tax_inputs_draft` `:960` → `form_save_draft` →
   `input_form_store::save_draft` `:126-131` → `Session::save` `session.rs:467-470`) writes the **entire**
   in-memory image — residue included — under a status promising the vault is unchanged. Therefore:
   - `close_all_mutation_surfaces(&mut self, may_save: bool)`;
   - `ResidueLive` passes **false** — no flush; the copy names the year whose in-memory `ReturnInputs` is
     lost (the latch forbids every save, so it cannot be preserved);
   - `apply_reprojection`'s stale path passes **true** (memory == disk: the persist succeeded);
   - `flush_tax_inputs_draft` itself refuses under either save-forbidding latch, which also covers the
     **idle tick** (`:10658`) and `q` — write-side gating, not surface-side;
   - the flush's error must not clobber the arm status (`:982` currently overwrites `app.status`, which on
     the `ResidueLive` path would destroy the CRITICAL notice `draw_edit.rs:155` red-styles).
2. **Fix the two missing opener guards** — `open_pseudo_approve_flow` (`:8185`) and `open_bulk_income_modal`
   (`:8640`). Both take the combined latch.
3. **Add `stale_or_residue_latch_status()`** beside `residue_latch_status()` (`:672-690`), which is left
   untouched. Precedence attest > rollback > stale, KAT'd; the remedies are opposite (`attest_save_failed` =
   did **not** land, retry; `stale_after_write` = **did** land, do not retry).
   **Call-site split** (arch r3 C-2's enumeration):
   - **original — 2 sites:** `execute_defensive_export` (`:4616`) and **`open_defensive_filing`
     (`editor.rs:474`)**. The dashboard is read-only (C-3) and both write flows are separately gated at
     `open_declare_flow` (`:3997`) and `open_promote_flow` (`:4253`), so keeping the original here is what
     makes D-7's route reachable.
   - **combined — the other 25 openers**, plus the two new checks from step 2.
   - The dashboard's write intents (`DashboardIntent::Declare`/`Promote`, `:505-510`) take the combined
     latch: entering the read-only dashboard is allowed, declaring/promoting from it is not.
4. **The five payload sites** (`:4172-4188`, `:4367-4375`, `:4319-4328`, `:4116-4124`, **`:8209-8227`**)
   each probe `stale_reason()` — an owned-return accessor whose borrow ends before the `&mut` field borrow
   — and refuse. Verified to compile at all five: the probe mirrors the shipped shape at `:4616-4630`, and
   `app.snapshot.as_ref()` + `app.*_flow.as_mut()` are already disjoint field borrows today.
5. **`open_defensive_filing` skips its DFW-D6 `pseudo_active()` refusal while armed** (tax r3 I-3, arch r4
   m-2). That gate reads `snap.state.pseudo_active()` off the stale image (`editor.rs:478-490`), and the
   pseudo-approve tail is the one write that flips exactly that predicate — so a filer who approves every
   default and then hits a failed re-projection is told to press `P` (which now refuses) and is locked out
   of D-7's route. Skipping is safe: the dashboard is read-only, `Declare`/`Promote` intents take the
   combined latch, and `plan_export` re-derives pseudo-activity from the fresh rebuild (`:4626-4665`) and
   refuses there.

**Stays live:** `q`, `Esc`, `?`, tab navigation, cursor/scroll, sort keys, and the `w` → dashboard → `x`
export route.

### 2.4 The marker (third specification; the first two were unbuildable)

**(a) Render, don't merely reserve — and reserve at all.** `draw_defensive_filing`'s guard is
`if let (Some(rect), Some(s)) = (notice_area, status)` (`draw_edit.rs:152`) and `main.rs:494` clears the
status on every keypress, so reserving rows while `status` is `None` draws a **blank band**. The notice rect
renders a **fixed marker text whenever armed**, independent of `app.status`; when a status is present, both
appear. The **reservation predicate at `:109` must also change** from `status.is_some()` to
`status.is_some() || armed` — otherwise there is no rect to render into (arch r4 n-2). At terminal height
≤ 3, `DEFENSIVE_NOTICE_LINES.min(inner.height.saturating_sub(1))` (`:111`) yields 0 — floored at 1.

**(a2) The copy must physically fit** (arch r4 I-3). `DEFENSIVE_NOTICE_LINES = 3` was sized for the ~230-char
residue notice; the composed arm status (prefix + fact 1 + `({e})` + facts 2-3) runs ~300-350 chars against
3 × ~76 ≈ 228 usable, and the marker now shares that rect. Raise `DEFENSIVE_NOTICE_LINES` to fit the
**measured** longest composed string at 80 columns — measure it, do not estimate — and KAT the full arm
status rendering **unclipped** at 80 columns, not merely present.

**(b) Browse needs its own mechanism.** The notice machinery exists **only** in `draw_defensive_filing`.
`draw_browse` (`:173-294`) is `[Length(3) tab bar, (optional) Length(1) pseudo banner, Min(0) content,
Length(1) footer]` (`:183-195`), with the status in that single footer row (`:280-294`) which reverts to the
keybinding hint the moment `:414` clears it. Browse gets a **state-conditional `Constraint::Length(1)` row**
keyed on `stale_after_write.is_some()`, inserted exactly as the PSEUDO banner is (`:184`, `:213-224`) with
the `content_idx`/`footer_idx` bookkeeping at `:194-195` extended. Browse is the screen that renders Form
8949 / Schedule D figures off `snap` (`:229-271`), so it needs the marker most. Its footer is a single
un-wrapped `Paragraph` (`:188`, `:279-294`) that **truncates**, so the three facts cannot live there: while
armed Browse gets the one-row marker **plus** a wrapped multi-row status band sized like (a2).

**(c) The marker strings are pinned here, with content KATs** (tax r3 I-2). From the second keypress onward
the marker is the *only* notice the filer sees, so it must name the consequence, not the cause. Register and
precedent: the PSEUDO banner's "FICTIONAL placeholders — DO NOT FILE" (`draw_edit.rs:214-217`).
- **Browse row (≤ 78 cols):** `STALE — figures below predate your last write. DO NOT FILE from them.`
- **DefensiveFiling notice (longer form):** the same claim plus the remedy sentence.
§3.9's KATs assert the **substring**, not merely that something rendered.

The block-title marker (draft 2) is withdrawn: Browse's title carries a variable-length vault path
(`:201-204`) and ratatui truncates without wrapping, so it clipped at 80 columns.

### 2.5 Copy

**Fact 1 does not claim correctness** (tax r1 C-2): *the write reached disk, but whether it had the intended
effect could not be verified, because the ledger would not re-project.* True at all 27 — every `Err`
re-projection arm is nested inside an `Ok(..)` persist result and every wrapper reaches disk
(`edit/persist.rs:227-232`; `input_form_store.rs:129`). Thirteen `derive_*` fns exist precisely to report a
write that did *not* achieve its intent (`derive_void_status` `:4714-4721`: *"Void saved, but
DecisionConflict fired — the target decision remains in force"*), and on the `Err` arm none of that is
computable.

**Fact 2 is computed, not assumed** (tax r2 I-6, arch r3 m-9). Draft 2's premise ("26 of 27 null their
surface above the match") is false — only 4 do (`:1340`, `:4219`, `:4500`, `:7026`); ~21 null *below* it, so
`close_all`, which now runs first, does close a live surface at most tails. The condition comes from a
pre-close survey (`any_mutation_surface_open()`). The park tail carves out explicitly:
`confirm_park_to_profile` sets `dirty = false` at `:1536` before the match and the parked draft is already
on disk, so nothing is discarded — saying otherwise would send the filer to re-author a return that
`park_to_profile` then refuses (`:1511`).

**Fact 3:** quit and reopen.

**Insertion point at the park tail** (arch r4 n-3): `after_write` must be called **after** the
`:1532-1537` form block, not at the current `build_snapshot` position (`:1528-1531`) which precedes it —
otherwise `close_all(may_save = true)` runs before `form.dirty = false` (`:1536`) and the flush stops being
a no-op.

**Per-tail prefixes survive**, since fact 1 no longer names what landed. **AMENDED at the T4 gate
(2026-07-26) — this section is the mandate; the shipped separator is an em-dash, not `", but "`:**

- `"committed {year} as {label} — "`
- `"parked the full return for {year} — "`
- `"the safe-harbor attest write landed — "` — reworded from the shipped `"Attested but…"`, which claims an
  *effect* `derive_attest_status` (`:7074-7078`) exists to deny.

Each carries its own **trailing space**; fact 1 is appended directly after it.

*Why amended:* the original `", but "` forms composed with fact 1 into `"committed 2024 as Single, but the
write reached disk, but whether it had the intended effect could not be verified…"` — two `but`s and a
restated outcome. Recorded here rather than only in a source comment, because a source comment superseding
a reviewed spec section is this repo's named "don't defer a spec mandate with a false citation" failure —
and T5-T8 read their wording from this section.

**If the reopen also fails:** `open_session` (`btctax-tui/src/unlock.rs:129-136`) and the viewer's
`attempt_open` (`:150-164`) both call `build_snapshot`, with deterministic arms (`:174-205`), so a restart
can bounce the filer out of both TUIs. D-7 is the mitigation and is now genuinely reachable. The CLI
(`btctax export-irs-pdf`, `btctax verify`) is the second line — recorded here, not in the arm copy (owner's
call).

### 2.6 Data flow

```
persist_*(…) ─Ok─→ after_write(|snap| status) ─→ apply_reprojection(rebuilt, status)
                                                  ├─ Ok  → status(&snap); set snapshot;
                                                  │        CLEAR the latch (D-4);
                                                  │        refresh dashboard IFF one is open
                                                  └─ Err → arm; prefixed status + 3 facts;
                                                           close_all_mutation_surfaces(may_save = true)
        └─Err─→ on_persist_error(e)              (unchanged; never arms;
                                                  close_all_mutation_surfaces(may_save = FALSE))

while armed:  stale_or_residue_latch_status()  → refusal      (25 openers + 2 new)
              residue_latch_status()           → unchanged     (export + open_defensive_filing, D-7)
              stale_reason()                   → refusal       (the 4 payload sites)
              renders                          → stale image + marker
export `x` →  re-project → Ok ⇒ install + CLEAR │ Err ⇒ refuse, latch unchanged
```

## 3. Testing

Every primitive TDD, every fix mutation-proven.

1. **`apply_reprojection` unit KATs** — `Err` arms, sets the prefixed status, closes surfaces with
   `may_save = true`; `Ok` computes the status from the rebuilt snapshot, **clears the latch**, refreshes
   the dashboard only if one is open (*mutation:* remove the `is_some()` guard → red); `None` arms.
2. **Latch accessors** — `stale_reason()` / `stale_or_residue_latch_status()` report while armed;
   precedence attest > rollback > stale; `residue_latch_status()` unchanged (its shipped KAT `:21359` stays
   green).
3. **★ GATING (D-5): no decision payload of ANY kind is constructible while armed.** Three constructors,
   each mutation-proven: `plan_promote` (`:4367`), `plan_declare` (`:4172-4188`), **`pseudo_plan`
   (`:8209-8227`)**. Draft 4 named only the first two (arch r4 I-1 / tax r3 I-1). Defence-in-depth per §0.1
   — the primary guarantee is §2.3 steps 1-3 + §3.5 — but these KATs gate the cycle.
4. **Both shipped chains (D-6)** — chain A: `bulk_income_flow` is `None` after a `ResidueLive` and `:8640`
   refuses; chain B: `P` refuses while `rollback_failed` is armed. Each mutation-proven.
5. **Three mechanized guards.**
   (a) `handle_key`'s dispatch set ⊆ `close_all`'s clear set. **Extraction rule stated:**
   `defensive_dashboard` is dispatched via `as_mut()` (`:501`) and is deliberately **excluded** — the true
   rationale (arch r4 m-1; draft 4's "nulling it would kill D-7's route" is **false**, since `Esc`→`w`
   rebuilds it) is that every dashboard intent is individually handled: `Declare`/`Promote` take the
   combined latch, `Export` is exempt by design, `RouteResolveFirst`/`None` are inert
   (`defensive_dashboard.rs:84-134`). Nested surfaces (`form.modal`) are out of a `handle_key`-scoped scan.
   (b) **An exact partition, not a presence test** (arch r4 I-4). With two latch fns, "contains a check" is
   no longer the property: the set calling `residue_latch_status()` must be **exactly**
   `{execute_defensive_export :4616, open_defensive_filing editor.rs:474}`, and every other opener must call
   `stale_or_residue_latch_status()`. Assert **both directions**; mutation-prove by flipping
   `open_declare_flow`. **Domain is every assignment of an `EditorApp` surface field to `Some(..)` in
   `main.rs` + `editor.rs`** — not `fn open_*`, which misses the **12** sites that open a surface from
   inside a `handle_*_key` (`:610, :1862, :1994, :2111, :2433, :2809, :2976, :3116, :3282, :5072, :5426,
   :6093, :6438, :6564, :9976`); the exempt list must name each one's parent surface, a grep-checkable
   claim rather than a name pattern. This is the same failure mode that produced chain A, one level deeper.
   (c) Write-side: `flush_tax_inputs_draft` refuses under either save-forbidding latch — covering the idle
   tick (`:10658`) and `q`, which no `handle_key`-scoped guard can see.
   Extend `kat_rollback_failed_latch_refuses_all_openers` (`:21381`) from 9 keys to the **opener subset** of
   the 25 Browse opener keys — not the whole `KEYMAP-SYNC` region (`:415-483`), which also binds `q`, `Tab`,
   `j/k`, `?` that §2.3 keeps live. **The two latches need different key sets** (arch r4 n-1): 25 keys for
   `rollback_failed`, **24 for the stale latch (minus `w`)**, with §3.7's positive `w`/`x` KAT covering the
   difference.
   **Flush-gate semantics, disambiguated** (arch r4 m-3): `flush_tax_inputs_draft` refuses under the two
   **save-forbidding** latches (`attest_save_failed`, `rollback_failed`) — *not* under `stale_after_write`,
   which would contradict `may_save = true`. `may_save` is fail-safe, not an active flush: in production the
   form cannot be live when a non-tax-inputs tail runs (`handle_key:377` dispatches it first), and at both
   tax-inputs tails it is already `None` (`:1340`) or clean (`:1536`).
6. **Tail-class guard** — assert `build_snapshot` (token **without** the paren; `:4626` proves the
   paren-less idiom is live) appears only in `after_write` and the export, over non-test, comment-stripped
   lines. Follow the **de-stuck** scanner at `main.rs:16229-16259` (mutation KAT `:16268`), not the sticky
   `edit/persist.rs:2090-2104`; the precedent is file-scope while this is function-scope, so the
   delimitation is specified. The re-export idea is **withdrawn as impossible** — `build_snapshot` is `pub`
   in a dependency crate (`btctax-tui/src/unlock.rs:171`), so `main.rs` can always name it by full path, and
   the only enforceable version demotes it to `pub(crate)`, a breaking change contradicting §5.
7. **Refusal + exemption KATs** — refusals name the reason; `q`/`Esc`/`?`/navigation stay live; **`w` opens
   the dashboard and `x` exports while armed** (D-7's route, armed from a *Browse* tail); a successful `x`
   **clears** the latch and the marker (D-4).
8. **Representative end-to-end KATs** — one snapshot-derived site, one of the two no-snapshot `derive_*`
   sites, and the hoisted site (`commit_tax_inputs`, whose confirmation must still appear).
9. **Marker KATs** — armed marker renders with `status == None` on both screens; Browse at 80 columns with a
   long vault path; height ≤ 3 floor.
10. **Goldens** — armed-conditional, so **no existing golden changes**. New goldens cover **both armed
    sub-states** (arch r3 n-15): armed + status present, and armed + status cleared (the frame after the
    next keypress, where the reservation changes the content area).

## 4. Non-goals

The other burndown cycles. Privatizing `EditorApp.snapshot` (cut at r3 — see the banner). A refresh/retry
*key*. **Withdrawn as a non-goal:** "any change to what a filed number is derived from" — §0.1.
**New FOLLOWUP (tax r2 m-4), owning phase = this cycle's ship:** an existing vault may already hold a
promote whose `Acknowledgment`/`filed_basis` was recorded off a stale image; nothing detects or surfaces it.
No shipped user has a vault, so the exposure is theoretical — recorded rather than silent.

## 5. SemVer

PATCH. `crates/btctax-tui-edit/Cargo.toml:13-15` declares a `[[bin]]` and no lib; `stale_after_write` is a
new `pub` field on a binary-crate struct, not a published API.
