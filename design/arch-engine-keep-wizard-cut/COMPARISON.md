# Comparison: keep the engine, delete the TUI wizard

Branch `arch/engine-keep-wizard-cut`, from `main` @ `198710c` (v0.12.0). For comparison against
`main` and against `backout/pre-approach-b`, per an independent architect's recommendation. **Not a
decision** — nothing here is merged, tagged, or pushed.

## What was deleted

- `crates/btctax-tui-edit/src/defensive_dashboard.rs`, `src/edit/declare_flow.rs`,
  `src/edit/promote_flow.rs` — the wizard dashboard and its Declare/Promote flow state machines.
- `EditorScreen::DefensiveFiling`, its key-dispatch arm, the Browse `w` binding, and the three
  chokepoint-adjacent wrappers in `edit/persist.rs` (`persist_declare_tranche`,
  `persist_promote_tranche`, `persist_defensive_export`).
- The stale-snapshot latch in its entirety: `EditorApp::stale_after_write`, `after_write` /
  `apply_reprojection` / `arm_stale`, `stale_reason`, `stale_or_residue_latch_status`, the two STALE
  markers (Browse's one-row banner + DefensiveFiling's own), the content-sized notice-height
  machinery, and the four mechanized source-scanning guard tests (`main.rs`'s "Task 8: the mechanized
  guards" block — `handle_key_dispatch_close_all_and_any_open_agree_on_the_surface_set` (guard a),
  `exactly_two_openers_use_the_plain_latch_and_every_other_opener_uses_the_combined_one` (guard b),
  `build_snapshot_is_named_only_by_after_write_and_the_export` (guard d), plus their shared
  `fn_starts`/`fn_body`/`dispatch_surface_fields` scanning helpers). Guard (c) — `flush_tax_inputs_
  draft`'s own `attest_save_failed || rollback_failed` check — is a *different*, residue-latch guard
  that predates the stale latch and stays; it is not part of this deletion. All 27 write tails were
  reverted to their pre-latch inline `build_snapshot` + match form (documented per-site in the commit).
- The experimental banner's rendering on the (now-gone) DefensiveFiling screen; `uses_approach_b`
  stays wired into Browse's own banner, untouched.
- Two golden fixtures made of the deleted screen (`docs/examples-tui/btctax-tui-edit-defensive-filing-
  {pseudo-stub-export-refused,stale-armed-no-status}.txt`) and the tests that produced them.

## Kept, untouched

`btctax-core` (`DeclareTranche`/`PromoteTranche`, the resolve-time basis rewrite, `Coverage::Full`,
the loss clamp, the drift advisory, `defensive::{journey_view, discovery, era}`), `btctax-forms` (8275
content, AcroForm fill, Part II overflow), `btctax-cli/src/chokepoint/` (`plan_declare`/`plan_promote`/
`apply_*` — still the single front door), every CLI verb (`reconcile declare-tranche`, `reconcile
promote-tranche`, `export-irs-pdf`, `export-snapshot`), and the rest of `btctax-tui-edit` — classify-
inbound, reclassify-outflow/income, set-fmv, void, select-lots, link-transfer, the bulk paths, method
election, safe-harbor, tax-inputs. `close_all_mutation_surfaces`, `on_persist_error`, and the residue
latch (`rollback_failed`, `attest_save_failed`) stay: they predate this work and serve the whole editor.

## Added

`btctax defensive status` (a new top-level command, sibling of `reconcile`/`export-*` — not nested
under `reconcile`, since it is a read, not a decision event): prints the same `journey_view` the
retired dashboard rendered — declare candidates, resolve-first shortfalls, live tranches and their
state, still-short pools, and flagged export years — each with the exact next verb. Gated on the same
`uses_approach_b` predicate the other verbs use for their stderr disclosure; refuses on a pseudo-active
projection (mirrors `plan_export`'s own DFW-D11 refusal) rather than risk the `journey_view` precondition
panic. `crates/btctax-cli/src/cmd/defensive.rs`, `render::render_defensive_status`
(`crates/btctax-cli/src/render.rs`), wired in `crates/btctax-cli/src/main.rs`; man pages
`docs/man/btctax-defensive{,-status}.1` and `docs/examples/examples.md` regenerated via
`cargo run -p xtask -- docs` / `-- examples` (both gated by xtask's own tests, both now green).

## Lines removed by area

Net line deltas, working tree vs. `main` @ `198710c`:

| File | Before | After | Δ |
|---|---:|---:|---:|
| `btctax-tui-edit/src/main.rs` | 31,711 | 27,537 | **-4,174** |
| `btctax-tui-edit/src/draw_edit.rs` | 8,560 | 7,391 | **-1,169** |
| `btctax-tui-edit/src/editor.rs` | 561 | 423 | -138 |
| `btctax-tui-edit/src/edit/persist.rs` | 5,281 | 5,157 | -124 |
| `btctax-tui-edit/src/edit/mod.rs` | 20 | 13 | -7 |
| `btctax-tui-edit/src/defensive_dashboard.rs` | 1,560 | 0 (deleted) | -1,560 |
| `btctax-tui-edit/src/edit/declare_flow.rs` | 1,388 | 0 (deleted) | -1,388 |
| `btctax-tui-edit/src/edit/promote_flow.rs` | 953 | 0 (deleted) | -953 |
| `docs/examples-tui/*-defensive-filing-*.txt` (2 goldens) | 61 | 0 (deleted) | -61 |
| **`btctax-tui-edit` + goldens total** | | | **-9,574** |
| `btctax-cli/src/{cli.rs,cmd/defensive.rs,cmd/mod.rs,main.rs,render.rs}` | | | **+456** |
| `btctax-cli/tests/defensive_status_cli.rs` (new) | | | +250 |
| `docs/man/btctax-defensive{,-status}.1` (new) + `btctax.1` diff | | | +33 |
| **Net across the branch** | | | **≈ -8,835** |

Inside `main.rs`, the split is telling: production code (everything above `#[cfg(test)]`) fell from
10,732 to 10,031 lines (-701, net of ~500 lines added back reverting 25 write tails to their inline
form). Test code fell from 20,978 to 17,505 lines (-3,473). **The wizard's test suite was larger than
its own production code**, and shrank harder than it did: the crate's discoverable test count dropped
from 508 to 371 (137 fewer; 100 `#[test]` fns removed from `main.rs`, 18 from `draw_edit.rs`, plus
their shared fixtures). Every remaining test passes; `make check` (nextest workspace + clippy
`-D warnings`) and `cargo fmt --all -- --check` are both green — workspace-wide test count 2,535 → 2,407
(128 fewer net, after the 11 new CLI tests this branch added).

For scale: the independent review's own split put the floor engine (v0.8.0→v0.9.0) at ~8.3k crate
lines and the wizard (v0.9.0→v0.10.0) at ~16.7k — I confirmed both directly (`git diff --stat
v0.8.0 v0.9.0`: +8,300/-242; `v0.9.0 v0.10.0`: +16,718/-4,175). This branch removes roughly **57%** of
what the wizard release added, net (the remainder is the general-editor fixes — the two shipped
Criticals below — and doc/test churn from later releases that this branch doesn't unwind).

## What a filer can and cannot still do

**Can, unchanged:** every reconciliation action in the general ledger editor (classify-inbound,
reclassify-outflow/income, set-fmv, void, select-lots, link-transfer, the bulk paths, method election,
safe-harbor allocate/attest, tax-inputs full-return authoring); `btctax reconcile declare-tranche` /
`reconcile promote-tranche` from the CLI, byte-identical to before (the chokepoint never moved);
`export-irs-pdf` / `export-snapshot`, including the 8275 fill and Part II overflow handling; and now
`btctax defensive status` for a plain-text read of the same shortfall/candidate/tranche/flagged-year
picture the dashboard showed.

**Cannot, that could before:**
- Browse the dashboard *interactively* inside the TUI (arrow through candidates/tranches/years in one
  screen) — replaced by a static `defensive status` printout; there is no cursor, no live re-render.
- Declare or promote a tranche *from inside the TUI* — the guided era-preset window picker, the
  live tax-delta preview (`t` on the declare-flow Edit step), the Promote flow's in-app multiline
  Form 8275 Part II text entry, and the two-sided consent screen are all gone. Both actions now
  require a separate CLI invocation (`reconcile declare-tranche --amount … --wallet … --window-start
  … --window-end …`; `reconcile promote-tranche <ref> --provenance … --part-ii-file <path>
  --i-acknowledge <phrase>`) — the Part II narrative has to be authored in an external editor first,
  not typed live against a rendered preview.
- **The composed multi-year export** (`w` → `x` on the dashboard, which called
  `btctax_cli::chokepoint::{plan_export,apply_export}` to export the current year plus every flagged
  year in one shot). That chokepoint is untouched per the brief (it lives in `chokepoint/`, the
  protected front door) and still has direct unit-test coverage, but **it now has zero callers from any
  shipped surface** — CLI or TUI. A filer must run `export-irs-pdf --tax-year Y` once per flagged year
  by hand; `defensive status`'s own "Flagged years" section says exactly that (it never suggests the
  now-unreachable composed verb). This is dead, still-compiled, still-`pub` capability — worth the
  owner's attention if `plan_export`/`apply_export` are meant to stay a real feature rather than
  become a documentation-only artifact of `chokepoint/`'s "single front door" promise.

## Which defect classes disappear with the code

I searched the whole review/FOLLOWUPS trail (`design/conservative-filing{,-approach-b}/`,
`design/defensive-filing-wizard/`, `design/stale-snapshot-latch/`) for every finding graded
Critical, split by era, and traced each one to its actual file. There is no single tally document;
this is reconstructed from the per-review `NC`/`NI` verdicts and the `FOLLOWUPS.md` entries.

**Engine era (`btctax-core`/`btctax-forms`, v0.8.0→v0.9.0): one shipped Critical, several caught
pre-merge.** The one that shipped — Form 8275 Part II narrative overflow, silently truncating the
filer's §1.6662-4(f) disclosure past ~137 characters with no error (`crates/btctax-forms/src/
form8275.rs`) — was *live in v0.9.0 and v0.10.0* (`design/f8275-part-ii-overflow/DESIGN.md:11`,
"shipped in v0.9.0 and v0.10.0") and fixed in a dedicated post-v0.10.0 cycle. It is untouched by this
branch — it lives in the kept `btctax-forms` crate and was never wizard-related. Every other engine
Critical (a 2025-transition tag-destruction bug, a holding-period gap, a Box E/F mapping error, a
`.expect()` panic on an out-of-range `--tax-year`, an ungated promoted-floor→§170(e) deduction leak)
was caught in spec/plan/whole-branch review and never reached a release.

**Wizard era (`btctax-tui-edit`, v0.9.0→v0.10.0): the wizard's own state-machine files
(`defensive_dashboard.rs`, `edit/declare_flow.rs`, `edit/promote_flow.rs`) never had a Critical survive
to a whole-branch review — every one of that era's Criticals lived in `main.rs`**, the file this branch
spends most of its effort on:

- **Chain A / Chain B** (kept, both fixes — see "Kept, untouched" above): `close_all_mutation_surfaces`
  omitting `bulk_income_flow`/`bulk_income_modal`/`tax_inputs_form`, and `open_pseudo_approve_flow`
  having no residue-latch check at all. Both are general-editor bugs — the KAT that would have caught
  chain B (`kat_rollback_failed_latch_refuses_all_openers`) only exercised 9 of 25 opener keys. The
  wizard didn't cause either; building the wizard's fourth mechanized guard is what triggered the audit
  that found them.
- **The stale-`app.snapshot` root defect — mixed, and mis-graded at ship time.** `promote_flow_confirm`
  built the filed `Acknowledgment.shown_terms` §6664(c) record and the Form 8949 col-(e) basis straight
  from `app.snapshot`, with no re-plan; `declare_flow_confirm` re-planned but still off `app.snapshot`.
  At the wizard's v0.10.0 merge this was explicitly judged **not blocking** — `design/defensive-filing-
  wizard/FOLLOWUPS.md` recorded, verbatim: *"Not a filing-correctness gate today … so this is
  post-merge-safe."* That line is now struck through in the same file, replaced with: **"THE
  NON-BLOCKING JUSTIFICATION ABOVE IS FALSE … Under the project severity rule this was Critical, and it
  should have blocked the merge rather than becoming a post-merge item"** (`design/defensive-filing-
  wizard/FOLLOWUPS.md:279-287`). This is the wizard's own real, shipped, wizard-specific Critical — and
  it is exactly the shape of defect this branch's deletion removes wholesale, by removing the two
  confirm tails that had it.

So: the engine's one shipped Critical was an isolated PDF-fill bug, unrelated to and unaffected by this
branch. The wizard's shipped Criticals were *not* in the 3,901 lines of dedicated wizard-only files this
branch deletes — they were in the 700-line net cut to `main.rs`'s dispatch/confirm-tail layer, in code
that mixed general-editor plumbing with the wizard's own confirm logic. What removes the *class* (a
multi-step flow's confirm tail computing a filed number from a projection that predates the confirm
keystroke) is deleting `declare_flow_confirm`/`promote_flow_confirm` themselves, which this branch does.

What the wizard *also* produced, on top of the two real confirm-tail sites: an entire generalized
stale-snapshot-latch subsystem built well after the fact to close that gap for good — a third
`EditorApp` latch field, a write-tail chokepoint (`after_write`/`apply_reprojection`/`arm_stale`), two
STALE marker renders, content-sized notice-height math, and mechanized guard tests scanning the crate's
own source for every "opener" and "payload-construction site." Sized directly (next section): it
protected five call sites, four of which existed only because Declare/Promote existed. The fifth
survives this deletion and needed none of that machinery — 15 lines of local re-projection at its one
write site closes it exactly the way `execute_defensive_export` already did, with no shared field and
nothing for a future reviewer to keep in sync. The underlying defect class — a stale in-memory
projection silently backing a filed number — is real and not wizard-specific in the abstract (see next
section), but a screen holding a cached `Snapshot` alive across three separate multi-step write flows is
what turned "close two confirm tails" into "build four mechanized source-scanning guards."

One honest trade-off from removing guards (a)/(b) wholesale rather than narrowing them: chain A and
chain B's *specific* fixes stay directly regression-tested (`residue_latch_cannot_be_re_entered_
through_the_surviving_bulk_income_flow`, `pseudo_approve_opener_refuses_while_the_residue_latch_is_
armed` — both kept, both green), but guard (a)'s *broader* promise — that `handle_key`'s dispatch set,
`close_all_mutation_surfaces`'s clear set, and (formerly) the stale latch's own open-surface scan can
never drift apart for *any* opener, not just the two that have already broken — is gone. Its two field
lists (`handle_key` dispatch, `close_all_mutation_surfaces` clear) still could support a narrower,
general-editor-only version of that guard; I did not attempt to reconstruct one, since guard (a) as
written was mechanically tied to the wizard's own field set (`defensive_dashboard`/`declare_flow`/
`promote_flow`) and doing this correctly is more than a delete.

## Where the recommendation turned out to be wrong

**The premise that all five stale-snapshot payload-construction sites were declare/promote flows is
false — one is general-editor code that survives this deletion, and it shares the exact hazard.**

`handle_pseudo_approve_modal_key`'s `Enter` arm (pseudo-reconcile mode, bound to `P` on Browse — not
part of the wizard) reads `app.snapshot` to build the `pseudo_plan` it then persists via
`persist_bulk_decisions`. Before this branch, that site was one of "Task 5"'s five stale-latch-gated
payload probes (see the pre-deletion doc trail, restated in the commit at the end of `main.rs`'s test
module). Four of the five — `handle_declare_flow_key`'s `Char('t')` preview, `declare_flow_confirm`,
the Promote provenance step's `Enter`, `promote_flow_review` — were deleted with the flows that owned
them. The fifth was not, because pseudo-approve is a general-editor surface, reachable and load-bearing
independent of whether the wizard exists at all.

Deleting the whole latch, as the brief's own text anticipated might be wrong ("if any surviving surface
still builds a filed payload from a cached projection, the latch must stay for that surface"), would
have silently reopened this hazard: under VaultLock exclusivity the *only* way `app.snapshot` can trail
the vault is an earlier write in the same session whose re-projection failed — a real, if narrow,
reachable state after any of the other 26 write tails. Rather than keep any part of the deleted latch
alive for one surface, `handle_pseudo_approve_modal_key` now re-projects a **fresh** snapshot
immediately before computing the payload (`crates/btctax-tui-edit/src/main.rs`, mirroring the pattern
the wizard's own `execute_defensive_export` used: re-project first, plan off the result). This is
correct by construction for every way the cache can go stale and needed no latch, no field, and no
cross-cutting guard — strictly less machinery than what it replaced, for the one site that needed
anything at all. `cargo nextest run -p btctax-tui-edit` is green including the general-editor residue-
latch KATs this touches.

Net: the recommendation's *conclusion* (delete the wizard, keep the engine) holds and the *specific
claim* used to justify one part of it (all five sites were wizard-owned) does not. The gap was narrow
and closable without compromising the deletion's scope.

## Seams a future web UI would need

The owner has separately flagged that the editor should eventually be modular enough to support a web
front end — not part of this branch's scope, and nothing here was restructured toward it. What follows
is only what fell out of doing this deletion: where the current code already draws that line, and
where it doesn't.

**A reference implementation already exists in this workspace.** `btctax-input-form`
(`crates/btctax-input-form`, 4,603 lines) is a UI-agnostic form engine for `ReturnInputs` — zero
`ratatui`/`crossterm` references anywhere in the crate — built explicitly "TUI now, web app later"
(`docs/architecture/ARCHITECTURE.md:56`), with its own serde `Edit` seam over stable `SectionId`/
`FieldId` enums the architecture doc calls "the web wire" (`ARCHITECTURE.md:358-363`). The editor's
`edit/tax_inputs.rs` (1,210 lines) — the tax-inputs field-editing dispatch — is likewise zero
`ratatui`/`crossterm`; only its *rendering* (`draw_edit.rs::draw_tax_inputs_form`) and its *key
dispatch* (`main.rs::handle_tax_inputs_key`, line 990) are TUI-specific.

Classification of what this deletion touched or left exposed, against that reference:

- **Already conforms** (a pure engine/data layer a web front end could consume unchanged):
  `btctax_core::defensive::journey_view` (`crates/btctax-core/src/defensive/mod.rs:630`) — the
  deletion's own centerpiece: the retired TUI dashboard and the new CLI `defensive status` both
  render the *same* call, proving the seam works. `btctax_core::experimental::{NOTICE,
  uses_approach_b}` — one predicate, consumed identically by CLI stderr and TUI banner.
  `btctax-input-form` (whole crate) and `edit/tax_inputs.rs` (both above). `edit/persist.rs`
  (5,157 lines post-deletion) — zero `ratatui`/`crossterm`; every mutation is a plain
  `fn(&mut Session, payload) -> Result<_, PersistError>`, directly callable from a web backend.
  `btctax_cli::chokepoint::{plan,apply}_*` — pure planners/appliers, no UI dependency, already
  shared between the CLI binary and (formerly) the TUI. The 20 `derive_*_status` functions in
  `main.rs` (794 lines total) — pure `String`-returning status-text derivation, already factored out
  of their rendering call sites. `edit/form.rs` (3,966 lines, ~40 `*FlowState`/`*ModalState` structs)
  is ratatui-free except for **one** line (`TableState` inside the shared `TargetList` cursor helper,
  `edit/form.rs:15`) — the state shape itself is already UI-neutral.

- **Partially conforms** (data is UI-agnostic; the *decision* of what's legal right now is not
  separately askable): every `open_*_flow` opener in `main.rs` (e.g. `open_void_flow`, line 3923) —
  each checks the residue latch, computes a candidate list (often via an already-pure core/session
  call like `voidable_decisions` or `Session::bulk_link_transfer_plan`), and either sets a status or
  mutates an `EditorApp` field, all in one function reachable only by *calling the opener itself*.
  There is no read-only "what can I legally do right now, and with what candidates" query a second
  front end could poll to decide what to show — it would have to call the same App-mutating,
  keypress-shaped function speculatively, or duplicate the legality logic.

- **Does not conform** (logic and rendering interleaved; a second front end reimplements it):
  `draw_edit.rs` (7,391 lines) — every screen's layout math, row/column cursor handling, and
  `TableState` bookkeeping is `ratatui`-coupled throughout; this is expected to be fully replaced by
  any second front end, not reused. `main.rs`'s key-dispatch layer — `pub fn handle_key`
  (line 125) and the ~40 `handle_*_key` functions it fans out to — is shaped around
  `crossterm::{KeyEvent, KeyCode}`, not a neutral command/intent type; a web front end would translate
  each arm into an HTTP route by hand, not reuse the dispatcher.

- **Rough proportion**, measured directly (function-body line counts in `main.rs`'s surviving
  10,031 production lines): `handle_*_key` dispatch is 5,468 lines (54.9%), `open_*` openers are
  2,294 lines (23.0%) — together **77.9%** of what's left is keypress-shaped dispatch-and-decide code
  a web front end cannot reuse as-is. `derive_*_status` (pure) is 794 lines (8.0%); the remainder
  (14.1%) is a mix of reusable `EditorApp` methods (`residue_latch_status`, `on_persist_error`,
  `close_all_mutation_surfaces`) and small helpers. Add `draw_edit.rs`'s 7,391 lines (100% view-only)
  and the total "would need reimplementing, not reusing" surface is roughly 15,150 lines — against a
  separately-reusable base of `edit/persist.rs` (5,157) + `edit/form.rs` (3,966) +
  `derive_*_status` (794) + the whole `btctax-input-form` crate (4,603) + `btctax_core::defensive`
  (already pure) that a web backend could largely stand on today.

This deletion did not change that ratio in any direction that matters — the wizard's own code was
already gone before this measurement, and the general editor's dispatch/render split was set well
before this branch. It's included here only because doing the deletion is what put it in view.
