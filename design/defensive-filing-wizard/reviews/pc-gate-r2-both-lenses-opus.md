# P-C PHASE GATE — round 2, BOTH LENSES (Opus) — persisted verbatim before folding

Base `badfae4` → head `47225af` (the fix wave folding r1's 5 Importants + 4 Minors).
r1 reviews: `pc-gate-tax-opus.md` (0C/3I), `pc-gate-architecture-opus.md` (0C/3I).

---

# TAX LENS r2 — **GREEN — 0 Critical / 0 Important / 3 Minor / 2 Nit**

(The 3 Minor / 2 Nit are *fresh this round*. r1 M-3/M-4/M-5 remain on the agreed DEFER list and are re-confirmed
correctly deferred; they are not re-counted.)

The fix wave adds **no new tax logic and alters no engine gate** — `chokepoint/mod.rs` is untouched by the range, and
`cmd/promote.rs`'s only production change is two inert data accessors (`promote_tranche`'s body is unchanged). Every gate
this phase relies on is still the shipped one; what changed is that the driver now *reaches* two of them instead of
answering them itself.

## Resolution audit

**I-1 (BG-D6 ack residency) — RESOLVED.** `promote_flow_confirm` destructures `ack` out of the `mem::replace`d `Consent`
step and passes the filer's buffer: `main.rs:4481` — `persist_promote_tranche(session, *plan, Some(&typed_ack), now)`.
Grepped every call site: that is the ONLY production caller of `persist_promote_tranche`; `PROMOTE_ACK_PHRASE` now appears
in `main.rs` only in the pre-check compare (`:4422`), its inline-error message (`:4431`), doc comments, and tests. The
constant is off the write path. Semantics unchanged (both the pre-check and `require_promote_ack` trim). **The SPEC-named
mutation KAT is real:** `promote_flow_confirm_hands_the_engine_the_typed_phrase_not_the_constant` builds a real vault,
drives real keystrokes to Consent with `"not the phrase"`, calls `promote_flow_confirm` DIRECTLY (exactly the state that
deleting the pre-check produces), and asserts (a) the status carries the ENGINE's `require_promote_ack` wording and (b)
`promote_count == 0` re-read from disk. Reverting to the constant reds BOTH assertions. The false doc comment is corrected.

**I-2 (BG-D5 provenance answered FOR the filer) — RESOLVED.** `PromoteFlowState::new` opens at
`PromoteFlowStep::Provenance { error: None }` with `provenance: None`; `render_promote_flow` offers all seven
`ProvenanceKind::ALL` entries with nothing pre-selected. The `Purchase` literal is gone from `review()`. **The refusal is
engine-driven:** `attest_provenance` routes any non-`Purchase` pick through `plan_promote` and renders the resulting
`Refusal::Provenance` via the shipped `From<Refusal> for CliError`.
`a_non_purchase_answer_surfaces_the_shipped_refusal_and_never_advances` loops all six non-Purchase variants and
BYTE-COMPARES the displayed string against `plan_promote → CliError` — a TUI re-implementation reds it. That test also
proves gate ORDER: the TUI state has an EMPTY narrative while the oracle passes a real one, and the strings are equal, so
BG-D5 genuinely precedes BG-D7 and an empty narrative cannot mask the provenance refusal. Both directions pinned
(mining e2e records 0 `PromoteTranche` even after typing the ack; the §6664(c) parity KAT still green).
**The answered-ness invariant is now STRUCTURAL, not conventional:** `Consent` is reachable only from `review()`'s `Ok`,
which requires `provenance == Some(k)` AND `plan_promote` accepting `k` — and BG-D5 accepts only `Purchase`. Both
`review()` and `attest_provenance` fail closed on `None` (bounce, no substitution), and there is no back-edge from
`PartII`/`Consent` into `Provenance` that could mutate the answer after the plan was computed.

**I-3 (false "declare will be refused") — RESOLVED.** `open_declare_flow` (`main.rs:4066-4070`) threads
`tranche_guard::in_force_allocation_exists` — the DIRECTIONAL core predicate — into the renamed
`DeclareFlowState::allocation_in_force`; `safe_harbor_blocked` is no longer consulted by the declare flow. Verified the
boundary: `TRANSITION_DATE == 2025-01-01`, gate is `window_end < TRANSITION_DATE && in_force_allocation_exists`, and the
new copy is conditional ("a declare whose window ends before 2025-01-01 will be refused") — strictly correct even for a
post-2024 shortfall. Three KATs incl. the majority-path one:
`a_pre2025_tranche_without_an_allocation_never_claims_a_declare_will_be_refused` asserts the predicate state, the absence
of the claim, AND that `plan_declare` does not refuse it — render and engine pinned to agree.

**r1 Minors folded:** M-1 (clamp order + `nudge_window_end` floor + `new()` seed, two mutation-verified KATs), M-2
(`excess_sat()` + the DFW-D8 confirm-note naming the excess and the real promote consequence), N-1 (`clearance` doc),
N-2 (`tax_delta_stale` split; "not computed yet" on first open).

## Adjudications
1. **arch M-3's premise — implementer right, deviation safe. APPROVE.** Verified: `persist_promote_tranche`
   (`persist.rs:464-475`) snapshots then routes EVERY `apply_promote` error through `rollback`, which returns
   `RolledBack` on successful restore and `ResidueLive` otherwise. `NoChange` is reachable only from the PRE-write
   `session.snapshot()?`. Keeping the flow open on `RolledBack` is safe (that arm restored to `pre`); `ResidueLive` closes
   the flow and arms the latch. The tax-critical property survives: the bounce goes to `PartII`, the `Consent` step (which
   held the plan) was consumed, so a retry must go back through `review()`'s FRESH `plan_promote` — no stale plan reaches
   `apply`, "what was shown is what is recorded" holds.
2. **SPEC gap — RECOMMEND a line; not blocking.** The step is the IMPLEMENTATION of DFW-D2's own mandate (gate ordering
   at SPEC.md:58-60 names BG-D5 first after resolve-live; the ★ ack-residency clause establishes "drivers only collect").
   What is missing is that DFW-D12 (SPEC.md:268-271) enumerates the wizard's per-tranche inputs as "consent figures, Part
   II narrative, and `Acknowledgment`" and should now also name the provenance attestation.
3. **`ProvenanceKind::ALL` + `pub label()` — no concern.** Inert data, drift-guarded against clap's `value_variants()`
   (length + membership) and pinning `ALL[0] == Purchase` so the `1`-key binding cannot shift onto a different value.
   Widening `label()` is a correctness IMPROVEMENT: picker and `refuse_non_purchase` now render the same words from one source.

## New findings (none blocking)

**Minor 1 — `cycle_preset`'s M-1 clamp collapses a non-applicable preset into a degenerate one-day, before-op window,
silently in the LEAST conservative direction.** `declare_flow.rs:98-105`. For a 2018-06-01 shortfall, cycling to
`Y2021To2024` yields `window_end = min(2024-12-31, 2018-05-31) = 2018-05-31` and `window_start = min(2021-01-01,
2018-05-31) = 2018-05-31` — a single-day window. Before the fix this state was inverted and `plan_declare` REFUSED it; it
now ACCEPTS. A one-day window's min IS that day's price, i.e. the HIGHEST floor the flow can produce (lowest gain — the
opposite of DFW-D9's "wider window → lower floor, the conservative direction"), and it sets the acquisition date one day
before the disposal, making the lot SHORT-term. Disclosed (readout shows preset name + window, whose mismatch is visible)
and the promote still shows `filed_basis` behind an ack, so not a silent filing — but the new KAT asserts only
`window_start <= window_end`, not that the window bears any relation to the preset chosen. **Fix:** skip presets whose own
start is after `window_end` when cycling, or render a note that the selected preset does not apply and the window was
clamped. Rides the owner era-table decision.

**Minor 2 — the flow renders have no scrolling, and the new NOTICE rect takes 3 more rows off the promote Consent
surface.** `draw_edit.rs:162-174` + `DEFENSIVE_NOTICE_LINES = 3`. One `Paragraph` with `Wrap`, no `.scroll(...)`, so
content beyond the rect is not drawn. The Consent step renders all of `render_consent(&plan)` (advisory lines + a
`ConsentTerm` per disposal year + gift-only years + post-consent note) plus ~8 flow lines — already taller than 80×24 for
a multi-year tranche. PRE-EXISTING and untouched; the wave makes it 3 rows worse on the narrow path where a `RolledBack`
sets a status, the flow bounces to `PartII`, and the filer Tabs back. Tax stake: `Acknowledgment.shown_terms` records
terms as SHOWN; on a short terminal a trailing term can be recorded as shown without being rendered. Partially
self-limiting (the ack prompt is at the bottom, so a truncated screen tends to be un-completable rather than silently
wrong). **Fix:** scroll support or an "N more lines" indicator.

**Minor 3 — SPEC DFW-D12 does not name the provenance attestation step.** Doc-only.

**Nit 1 — "donor/decedent carryover" is loose for inheritance.** Provenance-step copy: "already have a documented, real
basis (FMV at receipt, or donor/decedent carryover)". §1015 gift basis IS a donor carryover (with the dual-basis loss
rule); §1014 inherited basis is a **step-up to FMV at date of death**, NOT a carryover. A reader can map inheritance onto
the "FMV at receipt" half, so the substance is reachable, but the wording pairs the wrong mechanism with the wrong regime.
The rest is accurate (mining/staking/airdrop/fork → FMV at receipt: Notice 2014-21, Rev. Rul. 2019-24, Rev. Rul. 2023-14).

**Nit 2 — the Provenance screen names which answer passes the gate before the filer answers.** "Only a PURCHASE can be
promoted…" renders above the picker. Honest disclosure of the rule (same wording `refuse_non_purchase` uses), but stating
it BEFORE the attestation is mildly leading on a screen whose purpose is an unprompted filer answer. Consider moving that
sentence into the refusal path.

## Still-deferred — re-confirmed
r1 M-3/M-4 (displacement-caveat holes) untouched and still filing-safe; r1 M-5 (default-preset holding-period character)
genuinely untouched — `new()` still seeds `ALL_PRESETS[0]`; the M-1 clamp changes only window COHERENCE, never which
preset is default. Era-table follow-up still NOT tax-blocking (all four r1 grounds hold); r1 sub-point (b) is now
satisfied by the clamp, though it converts into Minor 1 above.

---

# ARCHITECTURE LENS r2 — **GREEN — 0 Critical / 0 Important / 2 Minor / 4 Nit**

All three r1 Importants RESOLVED, each with a mutation-killing test at the right layer. Write confinement survives the two
new code paths (verified mechanically). The new flow step is engine-enforced and adds no second gating authority.

## Resolution audit

**I-1 — RESOLVED.** `main.rs:4458-4467` passes `Some(&typed_ack)`; the constant is off the write path. `require_promote_ack`
(`chokepoint/mod.rs:301-312`) trims, the driver pre-check (`main.rs:4419`) trims — byte-identical semantics, so the
pre-check is a genuine happy-path no-op nicety, not a second authority. The doc at `main.rs:4375-4382` is now true. The
mutation KAT (`main.rs:13698`) calls `promote_flow_confirm` DIRECTLY with `"not the phrase"` — exactly the delete-the-
pre-check mutation — asserting the engine's refusal AND `promote_count == 0` against a real vault.

**I-2 — RESOLVED.** `draw_edit.rs:99-169`: `draw_defensive_filing` computes a NOTICE rect from `app.status` BEFORE the
content branch, so every surface (declare flow / promote flow / dashboard / fallback) inherits it — the CRITICAL residue
notice included. Red+bold for `CRITICAL`, cyan otherwise. **Layout math sound:**
`let notice_h = DEFENSIVE_NOTICE_LINES.min(inner.height.saturating_sub(1));` — `notice_h <= inner.height - 1` always, so
`Constraint::Min(0)` content keeps ≥1 row; height 0 or 1 yields `notice_h == 0` (zero-area render = no-op, no panic). Rows
reserved only when a status exists. Render-level KATs (`draw_edit.rs:7068` + sibling) drive a real `TestBackend` through
`draw()` and assert the shipped `ResidueLive` prefix reaches the buffer; `status → None` reds both. Also confirmed the
flow-dispatch layer (`main.rs:366`/`372`) returns BEFORE the `DefensiveFiling` arm's `app.status = None`, so an in-flow
refusal persists across keypresses.

**I-3 — RESOLVED.** `draw_edit.rs:317` `help_overlay_lines()` extracted; the `w` entry added to the Reconcile block.
**The sync guard is real:** `draw_edit.rs:7097 kat_keymap_overlay_lists_every_browse_char_binding` `include_str!`s
`main.rs`, scans between `KEYMAP-SYNC-BEGIN`/`-END`, skips `//` lines, and checks each char against the SAME
`help_overlay_lines()` text the filer sees. Verified against the real region: 37 concrete `KeyCode::Char('x')` arms, NO
`Char(c)` catch-all that could dilute it; `bound.len() > 20` and `bound.contains('w')` prevent a silently-matching-nothing
scanner. A future unlisted Browse `Char` arm reds.

**r1 Minors:** M-4 RESOLVED (renamed + doc names the short-circuit + a real added assertion); M-3 RESOLVED via
substitution; M-1/M-2 correctly DEFERRED and filed with owning phase P-D/whole-branch.

## New-code architecture checks
1. **Write confinement INTACT.** `apply_declare(` and `apply_promote(` each exactly ONE real call site (`persist.rs:443`,
   `:471`); every other hit is a doc comment or the gate's own token. No new `cmd::` in production (the `promote_flow.rs`
   occurrences are one doc comment at `:57` and test-region fixtures past `#[cfg(test)]` at `:336`). The provenance step
   reaches the chokepoint via crate-ROOT re-exports (`lib.rs:37`, `:74`).
2. **DFW-D1 no second gating authority HOLDS.** `attest_provenance` drives every non-`Purchase` pick through
   `plan_promote` and displays `CliError::from(refusal).to_string()` verbatim. Verified gate ordering: `plan_promote` runs
   `resolve_live_tranche` → `refuse_non_purchase` (`:333`) → the Part II emptiness check (`:337`), so a still-empty
   narrative cannot mask the provenance refusal. The KAT at `promote_flow.rs:485` byte-compares the TUI's error (EMPTY
   buffer) against `plan_promote`'s output for a NON-EMPTY narrative — equality is only possible if the refusal is
   provenance-driven and narrative-independent. The `PROVENANCE_ENGINE_DID_NOT_REFUSE` string is a **sound defensive
   assert, not a second authority**: reachable only if BG-D5 itself is broken, guarded by `debug_assert!(false)`, discards
   the plan, fails CLOSED, and authors no refusal CRITERION.
3. **State machine COHERENT.** Step-exhaustive dispatch inside the flow layer (so `q`/`Esc` never fall through to a Browse
   quit arm); `review()` cannot be reached without a pick (double-guarded, fail-closes to `Provenance` rather than
   substituting — `review_never_substitutes_a_provenance_the_filer_did_not_give`); nothing pre-selected, `Enter` alone
   insists, digits outside 1-7 inert, and `PROMOTE_ACK_PHRASE` contains no digits so stray ack-typing cannot mutate the
   selection; `provenance` and `part_ii` live OUTSIDE `step` so bounces preserve authored work; one-flow invariant intact.
4. **The two new inert `btctax-cli` items — right seam, real guard.** `ProvenanceKind` was already crate-root re-exported
   specifically so the TUI need not name `cmd::`; hanging `ALL` + `pub label()` on that type is the minimal seam. The
   drift guard (`cmd/promote.rs:239`) compares against clap's derive-generated `value_variants()`, which extends
   automatically when a variant is added while `ALL` does not — so it genuinely reds. It also pins `ALL[0] == Purchase`.

## Adjudications
1. **M-3's premise wrong — implementer correct, substitute sound.** Verified `NoChange` is produced solely by the `?` on
   `session.snapshot()`, i.e. a snapshot failure — no `apply_promote` error (ack gate, BG-D9 `would_conflict`, append,
   save) can yield it. The substitute is consistent with `close_all_mutation_surfaces()` semantics: `on_persist_error`
   sets a status for `NoChange | RolledBack` and arms `rollback_failed` + closes surfaces only for `ResidueLive`. Keeping
   the flow open on `NoChange | RolledBack` keeps it open exactly where nothing was persisted and no latch is up.
   Ordering correct; the KAT induces a REAL engine refusal (out-of-band promote → BG-D9 conflict), not a mock.
2. **Browse footer without `w` — ACCEPTABLE.** Verified the overflow against the committed golden:
   `docs/examples-tui/btctax-tui-edit-browse.txt` ends the footer at `?: help` — the `q/Esc: quit [EDITOR]` tail is
   ALREADY clipped. Inserting a feature key ahead of it would push `?: help` (the pointer to the overlay listing `w`) off
   the visible row — strictly worse. Convention is footer = navigation + the `?` pointer, overlay = feature keys; the
   discoverability chain (footer → `?` → overlay → `w`) is now mechanically guarded. No goldens regenerated.
3. **DEFER entanglement — none harmful.** M-2 stays coherent (the corrected `clearance` doc now says wiring is a filed
   follow-up). M-1 narrows but survives: the two `Err` arms now legitimately differ, while the SUCCESS tail — what M-1's
   extraction targets — remains byte-identical.

## New findings

**M-r2-1 — `era.rs`'s module doc still asserts the P-C-blocking gate that FOLLOWUPS re-owned to P-D.**
`crates/btctax-core/src/defensive/era.rs:19-20` still reads "…it must be burned down before the P-C gate closes", while
`FOLLOWUPS.md:55-64` is correctly re-owned to P-D/ship with the widened blast radius. Two shipped artifacts now disagree
about whether this gate may close, and a grep-based ownership reconciliation finds a P-C claim in source. **Fold this one
line before recording P-C closed**, or a future reader concludes the gate was closed against its own stated precondition.

**M-r2-2 — the Provenance step's prose carries an unguarded second copy of the closed BG-D5 enumeration.**
`promote_flow.rs:245-251` hand-maintains "…gift, inheritance, mining, staking/earning, airdrop or fork…" while the picker
derives from `ProvenanceKind::ALL` + `label()` and the refusal comes from the chokepoint — both drift-guarded. Adding or
renaming a variant updates the picker automatically and reds the drift guard, but leaves this line stale, and no test
covers it. Exactly the taxonomy-drift class this project has burned on before. **Fix:** build the clause from
`ProvenanceKind::ALL.iter().filter(|k| **k != Purchase).map(|k| k.label())`, or assert in the existing test that the prose
names every non-`Purchase` label.

**Nits.** N-r2-1: `handle_promote_flow_part_ii_key`'s `Esc` cancels the whole flow rather than stepping back to
`Provenance`, while `Esc` at `Consent` steps back one — harmless (PartII is only reachable with `Purchase`), worth a doc
line. N-r2-2: the NOTICE collapses to zero rows at `inner.height <= 1` and the ~230-char CRITICAL status clips below ~77
columns; graceful, never a panic, and "Quit the editor NOW" survives in the first ~120 chars. N-r2-3: both I-2 render KATs
exercise the DASHBOARD surface only; add one case with `promote_flow` open (cheap insurance). N-r2-4: three r1 Nits are
neither folded nor filed (tax N-3 tautological assert + import-keepers; tax N-4 misleading test name; the "Debug-format
rows" follow-up scope-widening to the two new flow renders, which add `{:?}` on shortfall/wallet/era-preset/tranche).

## Bottom line
P-C closes clean on the architecture lens. Recommend folding **M-r2-1** (one line in `era.rs`) before recording the gate
closed; everything else is properly parked with an owning phase.
