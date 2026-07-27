# cycle-prep recon — 2026-07-26 — defensive-filing-wizard post-merge queue

**Origin/main SHA at recon time:** `8dce32a`
**Local branch:** `main`
**Sync state:** up-to-date (0 ahead / 0 behind `origin/main`)
**Untracked:** none. Dirty (tracked, unstaged): `.superpowers/sdd/task-2-report.md`, `.superpowers/sdd/task-5-report.md` (SDD build ledger residue from the wizard build — not source; decide commit-or-revert before branching).
**Validation baseline:** `make check` GREEN — 2420 passed / 11 skipped, clippy `-D warnings` clean, no `make check: FAILED` banner. (Reminder: `make check` is nextest+clippy ONLY — fmt/msrv/pii-scan/net-isolation are CI-only.)

Registry: `design/defensive-filing-wizard/FOLLOWUPS.md` (per-project; the root `FOLLOWUPS.md` carries no
wizard entries). Slug verified: the **post-merge / next cycle** ownership class, i.e. every `[open]` item in
§"P-D/whole-branch", §"whole-branch FINAL review", §"FINAL review round 2", §"Copy pass residue", and
§"Post-merge — filed at the r3/r4 gate" — 17 live items — plus the spun-out sub-project-1 pseudo-`Acknowledgment`
latent gap. Drift expectation going in: HIGH for line numbers filed pre-fold, since the r1→r4 folds moved code
under the citations. Confirmed: the doc-precision batch (filed last, at r3/r4) is clean; the two tax-M items
(filed earliest, at the P-C gate) are mis-anchored; and one entry's *justification* is factually false.

---

## Per-slug verification

### PM-0 — sub-project-1 pseudo-`Acknowledgment` latent gap → **ALREADY DISCHARGED, close the doc**
- **WHAT (from `SPEC.md:444-447` §8):** "★ File against sub-project 1 (independent of this feature): the CLI
  `promote_tranche` can already fold pseudo numbers into the recorded `Acknowledgment` (DFW-D6/C-2). Fix at the
  shared chokepoint; add the latent-gap KAT."
- **Citations:**
  - "not fixed / to be filed" — **STRUCTURALLY-WRONG (stale).** The fix **shipped in this very build**:
    `crates/btctax-cli/src/chokepoint/mod.rs:356-361` (`let mut honest_cfg = …; honest_cfg.pseudo_reconcile = false;`
    on an own `Copy`, before `consent_terms`), documented at `chokepoint/mod.rs:23-26` as "the ONE intended
    behavior change — the sub-1 pseudo-off fix".
  - "add the latent-gap KAT" — **DONE.** `crates/btctax-cli/tests/chokepoint_promote.rs:280`
    `pseudo_active_promote_records_honest_terms_not_synthetic`, on the purpose-built
    `build_pseudo_off_vault` fixture (`:220`), whose doc records it was mutation-verified "empirically by
    temporarily disabling the fix". `SPEC.md:400-405` §5 already states this ("a **bug fix** to the latent
    sub-1 pseudo-`Acknowledgment` gap: the KATs it changes are the buggy ones").
- **Action for brainstorm spec:** NO code work. `SPEC.md` §8's first bullet is self-contradictory with §5 —
  rewrite it as discharged (name the chokepoint line + the KAT). Correct the `defensive-filing-wizard`
  memory entry, which still lists this as spun out/open. Cite `8dce32a`.

---

### Group A — the stale-`app.snapshot` class (the only group carrying code risk)

### PM-1 — Stale `app.snapshot` after a failed re-projection — the CLASS (`FOLLOWUPS.md:253-265`)
- **WHAT:** 26 `"Saved but re-projection failed … restart to refresh"` tails each leave `app.snapshot` on the
  pre-write image; nothing stops a filer ignoring the status and driving another read off it. Remedy: invalidate
  `app.snapshot` on failed re-projection, or route every write through one `after_write` helper that owns it.
- **Citations:**
  - `main.rs:4618-4639` "export re-projects before planning and refuses outright" — **ACCURATE** (block runs
    `4619-4640` in `crates/btctax-tui-edit/src/main.rs`; `fn execute_defensive_export` at `:4614`; the refusal
    arm sets `"export refused: the ledger could not be re-projected …"` and returns).
  - "**26** tails (grep the literal)" — **WRONG THREE WAYS** (corrected 2026-07-26 during the Cycle-1
    design + its architecture review r1). `grep -c` = 26, but **2 of those hits are the follow-up's own doc
    comments** (`main.rs:4601`, `:14321`), leaving 24 literal code tails. And the literal is the wrong
    predicate: **3 further tails carry the identical defect with bespoke copy** and match no grep for it —
    `:1347` (tax-inputs commit), `:1530` (tax-inputs park), `:7032` (safe-harbor attest). Deriving the class
    from `build_snapshot(` over production lines gives the true membership: **27 write tails** at
    `main.rs:559, 1347, 1530, 1661, 2241, 2588, 2660, 2861, 3197, 4217, 4498, 4809, 5130, 6147, 6716, 7032,
    7303, 7787, 8153, 8253, 8679, 8968, 9199, 9543, 9747, 10031, 10375`. (`execute_defensive_export`'s
    deliberate pre-plan re-projection is not one of them — it spells the call `.map(build_snapshot)` with no
    parens, which is why it appears in neither grep.) Lesson, and the reason this line is kept rather than
    silently fixed: **counting a copy string is not counting a defect.**
  - "Not a filing-correctness gate today (no writer re-derives a filed number from `app.snapshot` without its own
    fresh `plan_*`)" — **ACCURATE**, and independently corroborated by PM-2's own statement of the same fact.
- **Action for brainstorm spec:** this is the cycle's spine. Scope decision to make up front: **invalidate-to-`None`**
  (every reader fails loud) vs **one `after_write` helper** (27 call sites → 1). Note the FOLLOWUP explicitly
  couples the helper route to closing PM-3. Cite `8dce32a`.

### PM-2 — arch M-5, the DECLARE path can plan off a stale image (`FOLLOWUPS.md:367-378`)
- **WHAT:** declare/promote flows still read `app.snapshot` for `plan_*` inputs; the SHOWN readout
  (floor/coverage/tax-Δ, dashboard rows) can lag. Self-labelled "duplicate-safe … same root cause as PM-1".
- **Citations:** the 26-tail count and the "both confirm tails re-run their own FRESH `plan_declare`/`plan_promote`
  against `session` at the Enter" claim — **ACCURATE** (same grep as PM-1; the confirm-tail behavior is what
  `arch-M-2`'s deletion of `clearance()` at `FOLLOWUPS.md:128-133` already established as the real gate).
- **Action for brainstorm spec:** fold into PM-1 as one work item, not two. Keep both lines in the registry
  (they cite different surfaces) but give them one owning task.

### PM-3 — arch-M-1's open half: `after_defensive_write(app, status)` (`FOLLOWUPS.md:122-127`)
- **WHAT:** the `refresh_defensive_dashboard(app)` half is DONE; the save→re-project→status→close-flow tail
  itself is still duplicated between `declare_flow_confirm` and `promote_flow_confirm`.
- **Citations:**
  - "`refresh_defensive_dashboard` extracted, single source for both confirm tails + the export step" —
    **ACCURATE** (`main.rs:4562`, called from `execute_defensive_export` at `:4628`).
  - "`after_defensive_write` still open" — **ACCURATE**: zero occurrences of the symbol in
    `crates/btctax-tui-edit/src/main.rs`.
  - "`EditorApp::open_defensive_filing` deliberately keeps its own copy (`&mut self` + must run the DFW-D6 entry
    gate first)" — **ACCURATE** as a design note; not re-verified line-by-line (no line cited).
- **Action for brainstorm spec:** PM-1's helper route *is* PM-3. If the invalidate-to-`None` route is chosen
  instead, PM-3 must be scoped explicitly or it survives the cycle.

### PM-4 — "~22 remaining stale-snapshot tails" (`FOLLOWUPS.md:379-380`)
- **CLAIM-COUNTING AMBIGUITY (flag).** This line says "**~22** remaining" while the same entry parenthesises
  "**26** literal sites today", and PM-1/PM-2 both say 26. Neither number is right: see PM-1 above — 26 is a
  grep that counts two doc comments, 24 is the literal code count, and the **class is 27**. "~22" is a stale
  estimate with no basis in the tree.
- **Action for brainstorm spec:** state one number (**26**) once, in PM-1, and delete the separate line — it is
  explicitly "no separate work item".

---

### Group B — the two displacement-caveat holes (both mis-anchored; claims sound)

### PM-5 — tax-M-3, displacement-caveat hole for a correctly-sized cover (`FOLLOWUPS.md:106-111`)
- **WHAT:** when `covered_sat > 0 && t.sat == covered_sat`, neither `WouldDisplaceIfPromoted` nor `OverCovered`
  fires, yet a HIFO reorder across multi-year disposals still shifts gain between years — so that row's per-year
  delta is a reorder artifact shown as an unqualified saving.
- **Citations:**
  - `defensive/mod.rs:659-688` — **STRUCTURALLY-WRONG.** That range is `journey_view`'s tail (live-tranche
    collection → `still_short_pools` → `flagged_years` → the `DefensiveFilingView` literal). The advisory logic
    is at **`crates/btctax-core/src/defensive/mod.rs:389-421`**.
  - The *behavioral* claim — **CONFIRMED verbatim at the real site**: `:394 if covered_sat > 0 {` → `:395 if t.sat >
    covered_sat` gates `OverCovered`, so an exactly-sized cover pushes neither it nor `FeeOnlyPromoteNoop` (`:399`,
    which needs all-fee shortfalls); `WouldDisplaceIfPromoted` sits in the `:404 } else if !promoted {` arm, i.e.
    **only** when `covered_sat == 0`. The enum doc says so too (`:93-97`: "Fired ONLY when `covered_sat == 0`").
  - Fix sketch "fire on `!promoted && displaces_documented_basis(..)`" — **IMPRECISE.**
    `displaces_documented_basis` is the private inner helper (`:207`); the site already calls the public
    forward-looking wrapper **`would_displace_if_promoted`** (`:255-270`, invoked at `:410`), which is what the
    new condition should reuse. Note the wrapper needs a computable floor (`filed_basis_for` → `Coverage::Full`,
    `:409`), so the exactly-sized case inherits that precondition.
- **Action for brainstorm spec:** re-anchor to `defensive/mod.rs:389-421`; name `would_displace_if_promoted`;
  decide the suppression rule against `OverCovered` (per the FOLLOWUP, suppress only where `OverCovered` already
  carries displacement copy — which by construction cannot co-fire with the exactly-sized case, so re-derive it).
  This is the only Group-B item that changes an **advisory-firing condition**, so it needs its own mutation-proven
  KAT. Cite `8dce32a`.

### PM-6 — tax-M-4, the declare flow's on-demand tax-Δ carries no displacement caveat (`FOLLOWUPS.md:112-121`)
- **WHAT:** the `t` readout prints a bare `$delta`/`gain-Δ` with no displacement caveat; `declare_preview_saving`
  already builds both folds, so the check is nearly free.
- **Citations:**
  - `declare_flow.rs:293-307` "prints bare `$delta`/`gain-Δ`" — **STRUCTURALLY-WRONG.** That range is
    `compute_tax_delta`'s signature + the no-era fail-closed guard + the `declare_preview_saving` call (the
    *computation*, which prints nothing). The **render** is
    `crates/btctax-tui-edit/src/edit/declare_flow.rs:459-477`: `ComputedTax` → `:466`
    `"tax-Δ if later promoted ({year}): ${delta:.2}"` (bare); `Uncomputable` → `:468-471`
    `"… not a dollar figure — gain-Δ only: ${gain_delta:.2} (no stored tax profile / …)"`.
  - "`declare_preview_saving` already builds both folds" — **ACCURATE** (`defensive/mod.rs:494`; the flow calls
    it at `declare_flow.rs:305`).
  - The entry's own ★ premise-correction (that the dashboard NOW renders a caveated sibling via
    `render_saving_line`) — **ACCURATE**, matches the `[done]` tax M-1 entry at `FOLLOWUPS.md:236-244`.
- **Action for brainstorm spec:** re-anchor to `declare_flow.rs:459-477`. Both arms need the caveat, not just
  `Uncomputable`. Render-only → will touch TUI goldens (see lockstep). Cite `8dce32a`.

---

### Group C — doc/test precision, filed at the r3/r4 gate (citations clean; one false premise)

### PM-7 — the straddle invariant's stated CONSEQUENCE is a non-sequitur (`FOLLOWUPS.md:424-431`)
- **Citations:**
  - `era.rs:25` (module doc claims a straddling window makes pool assignment ambiguous) — **ACCURATE**
    (`crates/btctax-core/src/defensive/era.rs:23-29`: "**No bucket STRADDLES the pooling cutover** … so a declared
    tranche's lot has an **unambiguous acquisition-time** pool assignment").
  - `era.rs:163-164` (unit-KAT assert message) — **ACCURATE** (`:162-164`: "… its lot's acquisition-time pool
    assignment would be ambiguous (Universal vs per-wallet)").
  - `defensive_era.rs:83-85` (integration-KAT assert message) — **DRIFTED-by-1**: the message is at
    `crates/btctax-core/tests/defensive_era.rs:84-86`.
  - `resolve.rs:1310` (one `Eff` at `window_end.midnight()`) — **ACCURATE**, exactly line 1310:
    `utc: t.window_end.midnight().assume_utc(), // effective date = window_end (D-1a-a)`.
  - The substance (assignment is a total function of `window_end` alone, so a straddling window is *determinate*;
    the real reason to pin the invariant is that it would let a filer attest a possible pre-2025 acquisition while
    `pool_key` and `tranche_guard::pre2025_tranche_exists` read only `window_end`) — **SOUND**, and consistent
    with `era.rs:58-66`'s justification (2) and `defensive_era.rs:70-71`.
- **Action for brainstorm spec:** `era.rs` is a **docs.rs publish surface** (already shipped at 0.10.0) — this is a
  published-doc correction, same class as the r2 blocker. Cite `8dce32a`.

### PM-8 — incomplete sweep of the arch-M-1 phrasing (`FOLLOWUPS.md:432-436`)
- **Citations:**
  - `defensive_era.rs:74-76` — **ACCURATE** (`:73-76`: "no window may STRADDLE the cutover … lands in exactly ONE
    pooling era (Universal before, per-wallet from it on) **rather than spanning the split**").
  - `era.rs:151-155` — **ACCURATE** (`:151-155`: "… so a declared tranche's lot lands in exactly one pooling era").
  - "contradicting the module doc's own new disclaimer ~130 lines above" — **ACCURATE**: `era.rs:26-28` now says
    "a pre-cutover lot is later drained into its wallet's pool by `seed_transition` under Path A; the invariant here
    is about the **initial assignment, not lifetime residence**" — which the two test doc-comments omit.
  - "the fold changed the assert strings but not the attached doc comments" — **ACCURATE** (asserts at
    `era.rs:162-164` / `defensive_era.rs:84-86` were reworded; the doc comments above them were not).
- **Action for brainstorm spec:** pure whole-surface sweep — this repo's recurring miss. Do PM-7 and PM-8 in ONE
  pass over `era.rs` + `defensive_era.rs` so the third round doesn't find a fourth copy.

### PM-9 — wording-precision nits (a)-(d) (`FOLLOWUPS.md:437-444`)
- **Citations:**
  - (a) `era.rs:62-63` "~1,461 presses … alone" — **ACCURATE** (`:62-64`); `SPEC.md:247-249` — **ACCURATE**
    (the sentence lands on `:248-249`). The correction (that `nudge_window_start` clamps at `window_end`, so
    `window_start` cannot reach 2025-01-01 until `window_end` crosses first) is consistent with the `[done]`
    clamp entry at `FOLLOWUPS.md:16-22`.
  - (b) `README.md:441-442` "no figure computable for **that year**" — **ACCURATE** (`:441-442`);
    `defensive/mod.rs:302-311` (`SavingFlavor::Named` is **tranche**-scoped) — **ACCURATE**: the `Named` arm is an
    early return at `:305-309`, before the per-year `BTreeSet` loop at `:313`.
  - (c) `README.md:448-449` "re-derived from your ledger" — **ACCURATE** (`:448-449`).
  - (d) `SPEC.md:70` — **ACCURATE** ("The §6664(c) artifact must equal what the filer saw, on either surface");
    `DESIGN.md:46` — **ACCURATE** (same sentence, parenthesised).
- **Action for brainstorm spec:** `README.md` and `era.rs` are both published surfaces; (d) touches two design
  artifacts that are themselves under the review gate. Batch (a)-(d) with PM-7/PM-8 as one doc-precision task.

### PM-10 — arch M-3, two public `render_consent` fns (`FOLLOWUPS.md:360-366` **and** `:445-450`)
- **Citations:**
  - `cmd/promote.rs:186` — **ACCURATE** (`pub fn render_consent(terms: &[ConsentTerm], gift_only_years:
    &BTreeSet<i32>) -> String`).
  - `chokepoint/mod.rs:438` — **ACCURATE** (`pub fn render_consent(plan: &PromotePlan) -> String`).
  - "the crate root has NO collision (`btctax_cli::render_consent` resolves unambiguously to the correct
    advisory-carrying one)" — **ACCURATE** (`crates/btctax-cli/src/lib.rs:74` re-exports only
    `chokepoint::{… render_consent …}`).
  - ★ "**it has NO production caller** — only four assertions in `promote_cli.rs`" — **STRUCTURALLY-WRONG (false
    claim).** `chokepoint/mod.rs:60` imports it as `render_consent as render_consent_terms` and calls it at
    `:444` (`out.push_str(&render_consent_terms(&plan.terms, &plan.gift_only_years));`) — so **every** production
    consent render, CLI and TUI alike, goes through the narrow fn. `cmd/promote.rs:184` and the module docs at
    `:10-12` and `chokepoint/mod.rs:19` all state this correctly; only the FOLLOWUP entry contradicts them. The
    four `promote_cli.rs` call sites (`:481, :490, :518, :543`) are the only *direct external* callers.
- **Action for brainstorm spec:** the SHIP-AS-IS decision still stands on its other two legs (the narrow fn
  shipped at v0.9.0, so *removing* it is the breaking change; the crate root is unambiguous) — but the false
  premise must be struck **before** anyone acts on "no production caller", because deleting the fn on that basis
  would break `chokepoint::render_consent`. Correct the entry; keep the decision. If a rename is ever wanted,
  it is a **breaking** change to `btctax-cli`'s public API (pre-1.0 ⇒ MINOR), not a patch.

---

### Group D — UX / render / test-hygiene residue

### PM-11 — tax Minor 2, flow renders have no scrolling (`FOLLOWUPS.md:149-154`)
- **Citations:**
  - `draw_edit.rs:162-174` — **DRIFTED / mis-anchored.** That range is the NOTICE helper's tail plus
    `const DEFENSIVE_NOTICE_LINES: u16 = 3;` (`:169`). The **flow content** Paragraph is
    `crates/btctax-tui-edit/src/draw_edit.rs:148-150`
    (`let para = Paragraph::new(lines).wrap(Wrap { trim: false }); frame.render_widget(para, content_area);`),
    fed by the flow renders at `:126-135`; the 3-row reservation is at `:109-119`.
  - "no `.scroll(...)`" — **ACCURATE and stronger than filed**: `grep -n "\.scroll(" draw_edit.rs` returns
    **zero** hits in the whole file.
  - `DEFENSIVE_NOTICE_LINES = 3` — **ACCURATE** (`:169`), and its own doc (`:166-168`) claims the 3 rows exist so
    the ~230-char CRITICAL status "still renders in full on an 80-col terminal" — which is the same figure PM-13
    disputes below 77 columns.
  - Tax stake ("`Acknowledgment.shown_terms` records terms as SHOWN; on a short terminal a trailing term can be
    recorded as shown without being rendered") — **still live**, and it is the reason the README's §6664(c)
    sentence was narrowed twice (`FOLLOWUPS.md:318-320` then `:321-325`). This is the highest-stake item in Group D.
- **Action for brainstorm spec:** re-anchor to `draw_edit.rs:148-150` (+`:109-119`). Decide scroll vs "N more
  lines" indicator; note the fail-closed option (refuse the consent step when the terms don't fit) is the only
  one that fully closes the tax stake.

### PM-12 — arch N-r2-1, `Esc` at PartII cancels the whole flow (`FOLLOWUPS.md:155-157`)
- **Citation:** `handle_promote_flow_part_ii_key` — **ACCURATE**, `main.rs:4339`; the `Esc` arm at `:4341-4343`
  is `app.promote_flow = None;` (whole-flow cancel), vs the Consent step's one-step-back. "Harmless (PartII is
  only reachable via an attested `Purchase`)" — consistent with `promote_flow.rs`'s gate order.
- **Action:** doc line only, as filed.

### PM-13 — arch N-r2-2, the ~230-char CRITICAL status clips below ~77 columns (`FOLLOWUPS.md:158-159`)
- **Citation:** no line cited. Corroborated indirectly: `draw_edit.rs:166-168` sizes `DEFENSIVE_NOTICE_LINES = 3`
  against exactly that ~230-char status at 80 columns, so the <77-col claim is arithmetically consistent
  (3 × 77 = 231 minus the 2-space indent). **PLAUSIBLE, not machine-verified** — no golden pins a 77-col render.
- **Action:** cosmetic, as filed; if touched, add the narrow-width golden that would have caught it.

### PM-14 — arch N-r2-3, both I-2 render KATs cover the dashboard only (`FOLLOWUPS.md:160-161`)
- **Citation:** `draw_edit.rs:7068` + sibling — **ACCURATE** (`:7067-7068`
  `fn defensive_filing_screen_renders_the_critical_unrevertable_residue_notice`, and the sibling arch-I-2 assert
  block ending `:7065`). Both drive `render_defensive_filing_to_string(...)`, i.e. the dashboard surface, so a
  `promote_flow`-open case is genuinely uncovered.
- **Action:** one added case; pairs naturally with PM-11 (both touch flow rendering).

### PM-15 — N-r2-4(b), misleading test name (`FOLLOWUPS.md:166-169`)
- **Citation:** `defensive_journey.rs::declare_preview_saving_edits_the_window_and_changes_nothing_it_should_not`
  — **ACCURATE**, `crates/btctax-core/tests/defensive_journey.rs:1393`.
- **Action:** rename, or rewrite to actually exercise re-derivation across an edit (the FOLLOWUP offers both;
  the rewrite is the one that adds coverage).

### PM-16 — T7-copy (`FOLLOWUPS.md:384-386`)
- **Citations:** `defensive_dashboard.rs` "[optional, SUPPRESSED] promote" — **ACCURATE**
  (`crates/btctax-tui-edit/src/defensive_dashboard.rs:293`, with the `[optional]` non-default rationale at
  `:282`); `[x] export` bracket style — **ACCURATE** (`:454`, pushed unconditionally per the doc at `:380`).
- **Action:** copy-only; touches dashboard goldens.

### PM-17 — Debug-format rows (`FOLLOWUPS.md:387-392`)
- **Citations:** **ACCURATE and UNDER-counted.** Live non-test `{:?}` sites:
  - `defensive_dashboard.rs:222` (declare candidate `EventId`), `:232` (**two** — `EventId` + `BlockerKind`),
    `:242` (`PoolKey`), `:342` (tranche `EventId`).
  - `edit/declare_flow.rs:339` (shortfall `EventId`), `:358` (`wallet: {:?}`), `:385` + `:390` (`{preset:?}`),
    `:419` (`coverage: {:?}`), `:432` (`Coverage::{:?}`), plus `:149` in a refusal message.
  - `edit/promote_flow.rs:264` (`Promote — tranche {:?}`), `:345` (the ack phrase via `{:?}` — deliberate
    quoting, arguably correct; treat separately).
- **Action for brainstorm spec:** the filed list names 4 dashboard fns + "the two flow renders"; the real surface
  is **~12 sites across 3 files**. Needs a filer-facing `Display` (or format helper) for `EventId`/`PoolKey`/
  `BlockerKind`/`EraPreset`/`Coverage` rather than 12 ad-hoc `format!`s — decide that in the brainstorm, and
  expect the largest golden churn of the cycle.

### Also live, verified, no action needed this cycle
- **Browse footer omits `w`** (`FOLLOWUPS.md:146-148`) — **ACCURATE**: `draw_edit.rs:286-290` carries the
  in-source rationale and the footer string ends `"… ?: help   q/Esc: quit   [EDITOR]"`. Entry's own disposition
  ("revisit only if the footer is ever reflowed") is correct as filed — leave parked.
- **tax Nit 2, the Provenance screen states the passing answer first** (`FOLLOWUPS.md:400-403`) — **ACCURATE**:
  `promote_flow.rs:252` renders "Only a PURCHASE can be promoted to a >$0 estimated-basis floor: …" above the
  picker rows.
- **Free-text date/sat entry** (`FOLLOWUPS.md:404-406`) — **ACCURATE**: nudge-only editing; the picker is
  `1..=ALL_PRESETS.len()` (`era.rs:86`, `ALL_PRESETS: &'static [EraPreset]` at `:94`), so the entry's "1-6" is
  len-driven and will not drift. Genuinely optional — a feature, not a defect.
- **Plan-doc drift** (`FOLLOWUPS.md:407-408`) — **ACCURATE**: `IMPLEMENTATION_PLAN.md:61` names
  `ShortfallCandidate`; `grep -rn ShortfallCandidate crates/` returns **zero** hits (shipped type is `Shortfall`).

---

## Cross-cutting observations

1. **Two structurally-wrong line citations, both in the earliest-filed items.** PM-5 (`defensive/mod.rs:659-688`
   → real `:389-421`) and PM-6 (`declare_flow.rs:293-307` → real `:459-477`) were filed at the P-C gate and never
   re-anchored across the r1→r4 folds. Both point at *plausible-looking* code (`journey_view`'s tail;
   `compute_tax_delta`), so an implementer following the citation would edit the wrong function — exactly the
   decay cycle-prep exists to catch. Neither *claim* is wrong.
2. **One factually false justification (PM-10).** "No production caller" is refuted by `chokepoint/mod.rs:60`+`:444`
   (the fn is imported under an alias, which is why a naive grep for `promote::render_consent` misses it). The
   ship-as-is decision survives on its other two legs, but the premise must be struck before it licenses a deletion.
   Same shape as the r2 blocker (a false claim in a published doc), and the same lesson: verify a
   "no caller" claim against **aliased** imports, not just the plain symbol.
3. **One claim-counting ambiguity (PM-4) — and the registry's number is wrong in BOTH directions.**
   "~22 remaining" vs "26 literal sites"; the truth is 26 grep hits (2 of them doc comments), 24 literal code
   tails, and a **class of 27** once the three bespoke-copy tails are counted (PM-1). The registry counted a
   copy string rather than the defect, which is exactly how `:1347`/`:1530`/`:7032` — including the
   highest-stakes write in the editor, safe-harbor attest — stayed invisible to it.
4. **The r3/r4-filed doc batch (PM-7..PM-9) is citation-clean** — 13 of 14 citations exact, one off by 1 line
   (`defensive_era.rs:83-85` → `:84-86`). Freshly-filed items decayed less; the drift is a function of fold count.
5. **PM-0 is already discharged in code and KAT'd** — but the SPEC still files it as open work against sub-project 1,
   contradicting its own §5. Doc-only, and the project memory carries the same stale claim.
6. **Sync state is clean** (`main` == `origin/main` == `8dce32a`) and the suite is green, so any drift found here is
   pure citation decay, not a broken tree. Two SDD ledger files are dirty and unrelated to source.
7. **Publish-surface exposure.** `era.rs`, `README.md`, and `btctax-cli`'s `render_consent` pair are all **already
   published at 0.10.0**, so PM-7/PM-9/PM-10 are corrections to shipped docs.rs/crates.io content, not pre-release
   cleanups. `no-users-yet` still holds, so a breaking narrow is cheap — but it is still breaking.
8. **Not re-verified (out of recon scope):** PM-13's exact 77-column threshold (no golden pins it) and
   `open_defensive_filing`'s `&mut self` rationale in PM-3 (no line cited). Both flagged in place.

---

## Recommended brainstorm-session scope

**Three cycles, in this order.** Total live: 17 items + PM-0. Nothing here is a filing-correctness gate today,
so the ordering is by coupling, not urgency.

**Cycle 1 — the stale-snapshot class (PM-1 + PM-2 + PM-3 + PM-4).** The only group with code risk and the only
one with an architectural decision in it: invalidate-`app.snapshot`-to-`None` vs one `after_write` helper. Choose
the helper route and PM-3 closes for free; choose invalidate and PM-3 must be scoped explicitly. **27** call sites in
one file (`crates/btctax-tui-edit/src/main.rs`) + the readers (`declare_flow`/`promote_flow` `plan_*` inputs,
`journey_view`, dashboard rows). Rough sizing **~150-300 LOC** plus KATs (one per route: a failed re-projection
followed by a read must fail loud, not read stale). **SemVer: PATCH** (`btctax-tui-edit` internals; no public API,
no clap surface). Do this first — it is the only cycle whose outcome changes the others' call sites.

**Cycle 2 — the two tax caveats (PM-5 + PM-6).** Both re-anchored per Group B. PM-5 changes an advisory-firing
condition (`defensive/mod.rs:389-421`, reusing `would_displace_if_promoted`) and needs a mutation-proven KAT;
PM-6 is caveat copy on both `tax_delta` render arms (`declare_flow.rs:459-477`). ~40-80 LOC. **SemVer: PATCH**
(new advisory *firings* on existing variants; no new public enum arm — confirm `Advisory` gains nothing, else
MINOR). Tax lens is the primary reviewer here.

**Cycle 3 — doc precision + render residue (PM-7..PM-9, PM-10 correction, PM-11..PM-17).** Two natural halves:
(a) **docs**, one pass over `era.rs` + `defensive_era.rs` + `README.md` + `SPEC.md` + `DESIGN.md` + the PM-10
entry correction + PM-0's SPEC §8 rewrite + the PLAN:61 type name — whole-surface sweep in a single commit, since
the recurring failure here is a partial sweep; (b) **render/test**, PM-11 (scroll — decide fail-closed vs
indicator; highest stake in the group because of `shown_terms`), PM-14 (flow-open render KAT), PM-15 (test
rename/rewrite), PM-16 (copy), PM-17 (~12 `{:?}` sites → a filer-facing formatter). PM-12/PM-13 are doc-line/cosmetic.
Docs ~0 LOC of logic; render half ~100-200 LOC + the golden churn. **SemVer: PATCH**, unless PM-10 is actioned
into a rename (**MINOR**, breaking `btctax-cli` public API — recommend NOT doing it this cycle; keep the
correction, keep the fn).

**Mandatory locksteps**
- **TUI goldens + xtask examples goldens** — PM-6, PM-11, PM-16, PM-17 all change rendered bytes. Regenerate in
  the SAME commit as the change: `examples_golden_matches_committed`, `walkthrough_console_golden_matches_committed`,
  and the style-aware TUI goldens all pin current output (they are green today at `8dce32a`).
- **No clap flag-NAME change anywhere in this queue** ⇒ no man-page/manual mirror obligation and no `--help`
  regeneration. Re-check if PM-17's formatter leaks into any CLI-printed string.
- **Published-surface review** — `era.rs` (docs.rs) and `README.md` (crates.io front page) are live at 0.10.0;
  PM-7/PM-9 land there. `no-users-yet` applies, but the text is public.
- **Gate discipline** — per `CLAUDE.md`/`STANDARD_WORKFLOW.md`, each cycle's SPEC/plan passes an independent
  review loop to **0C/0I under BOTH lenses** before implementation, persisted verbatim under
  `design/defensive-filing-wizard/reviews/`, re-reviewed after every fold. **Reviewers: Opus** (user directive,
  2026-07-26) — not Fable.
- **Registry hygiene** — burn-down edits land in `design/defensive-filing-wizard/FOLLOWUPS.md` (re-anchor PM-5/PM-6,
  strike PM-10's false premise, collapse PM-4 into PM-1, close PM-0) so the next reconciliation grep is clean.
