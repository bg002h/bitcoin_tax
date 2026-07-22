# Defensive Filing — adjudicated design of record (Approach-B sub-project 2)

**Status:** PRE-SPEC, adjudicated after the Fable-architect r1 critique + owner scope decisions. This
SUPERSEDES `BRAINSTORM.md` (kept as history; the r1 review at `reviews/brainstorm-architecture-fable-
review-r1.md` critiqued that version). Being sent for a Fable-architect **r2** re-review to confirm the
r1 findings are resolved before this becomes the SPEC.
**Lineage:** Approach-B sub-project 2 (the guided actuator the conservative-filing G-3 promised).
Sub-project 1 (PromoteTranche floor + official Form 8275 PDF) shipped **v0.9.0**. Sub-project 3
(VARIOUS multi-date rows) is separate/later.
**Branch:** `feat/defensive-filing-wizard` (off `main` @ v0.9.0).

## 1. Purpose & audience

A **derived "Defensive filing" dashboard** in `btctax-tui-edit` for the filer whose **sales are
imported but purchases are gone** (the Mt. Gox / LocalBitcoins / old-wallet case). It surfaces the
positions that need a defensible basis and walks each through: **declare** a tranche to cover it →
**fork** (file at `$0` — complete + conservative — *or* knowingly **promote** to a `>$0` floor + Form
8275) → **export** the packet. It adds **NO new tax logic**: it composes already-shipped, already-
reviewed primitives, and every gate stays engine-enforced.

## 2. Object scope (owner decision) — dispositions, not holdings

Basis is a **disposition-time** concept, so the feature acts ONLY on BTC that was **sold or given
away with no purchase record** — precisely the engine's `BlockerKind::UncoveredDisposal` shortfalls
("dispose short by N sat", `fold.rs:710`; also fee/pending-out shortfalls). **Unsold no-records
holdings are OUT of scope** (no current-return effect; invisible to the engine; forward-planning only).
Consequence: the "find" step reads a signal the engine already computes — **no manual "I hold N BTC"
data model**.

## 3. Architecture — three seams (resolves r1 I-1/I-2)

1. **`btctax-core` — pure, KAT-able, mutation-proven computations only** (no UI, no session):
   - `journey_view(events, state, prices, cfg) -> DefensiveFilingView` — the DERIVED projection the
     dashboard renders: per-shortfall declare candidates, per-declared-tranche status (declared / can-
     promote / promoted), the clamped savings figures, the export-gate status. Fully derived from
     state — no persisted wizard progress.
   - the discovery predicate (shortfalls → candidates; existing lots categorically excluded) + the
     era→window preset table.
   - reuses the shipped signals (`overpayment`/clamped savings, advisories) verbatim.
2. **`btctax-cli` — the plan/confirm/apply chokepoints** (the shared seam, resolves I-1): each verb is
   extracted into a session/state-parameterized `plan → confirm → apply` trio that BOTH the CLI verb
   (thin driver) and the dashboard (thin driver) call. The gate ORDERING contract (resolve-live →
   BG-D5 provenance → BG-D7 8275 → BG-D3 coverage → BG-D6 consent → advisory → consent-render → ack →
   `would_conflict` → append) lives here ONCE. **Non-negotiable acceptance:** the recorded
   `Acknowledgment.shown_terms` and the rendered consent copy are **byte-identical** between the CLI
   and the dashboard (the §6664(c) artifact must equal what the filer saw, on either surface).
3. **`btctax-tui-edit` — the dashboard + the flows** (the presentation): the `*_flow` structs with
   `.step` live here, exactly like the ~25 shipped siblings; step availability is DERIVED from the
   chokepoint's plan/guard results, never independently encoded (no second gating authority).

## 4. The dashboard (resolves r1 I-3 — fork, not progress)

A derived, read-only journey view whose ROWS launch ordinary sibling flows — NOT a modal linear
super-flow. It is the resume mechanism (fully derived from state; nothing to persist or invalidate):

- **Needs a basis (shortfalls):** each `UncoveredDisposal` shortfall → `[declare tranche]` (covers it).
  Other $0 lots that are NOT shortfalls are **routed** to their correct remedial flow (basis-pending
  income → set-fmv; unknown-basis gift → gift-basis; unclassified inbound → classify) — never declared.
- **Declared tranches (the fork):** each shows its clamped savings and **two equal branches** —
  "file at `$0` (complete + conservative) ✓" and "promote to a `>$0` floor + Form 8275". A `$0`-declared
  tranche is NEVER rendered as incomplete. Promote is a choice, never a default (BG-1).
- **Forms:** an always-available `[export packet]` action (export writes files, not events, so it is
  never a checkable "done" step — M-5).

Launched flows are the shipped, chokepoint-backed declare / promote / export — each independently
reachable + testable like every existing flow.

## 5. Guardrails & safety (resolves I-6, C-2, I-7)

- **Declare guardrail copy fixed (I-6):** declaring is **`$0`, revocable, NO Form 8275** — a plain
  confirmation matching the CLI verb (which gates declare on validation only). The typed-phrase tier is
  reserved for the PROMOTE step (mirrors `PROMOTE_ACK_PHRASE`). The no-records assertion is worded as
  an ordinary confirmation and NOT recorded (recording it would be a schema change / new tax logic).
- **Pseudo gate (C-2):** the whole journey is gated on `!state.pseudo_active()` with routing guidance
  ("resolve/approve pseudo defaults first") — a defensive-filing journey over synthetic estimates is
  incoherent. The latent CLI-side gap (consent/savings folding pseudo numbers) is fixed at the shared
  chokepoint so both surfaces inherit it; **filed against sub-project 1 regardless of scheduling.**
- **Safe-harbor exclusion pre-check (I-7):** a pre-2025 `DeclareTranche` is refused beside an in-force
  `SafeHarborAllocation` (`guard_tranche_vs_allocation`). The dashboard surfaces this as a first-class
  state at entry — never let the filer pick a shortfall, choose an era, then bounce off a refusal.
- **Era presets (I-7):** confirm/edit starting points, NOT authoritative windows; a mandatory LIVE
  readout shows the resulting window-min floor + `Coverage` + holding date + saving as the filer edits
  (wider window → lower floor, the conservative direction, visibly). The preset table gets the same
  review rigor as copy. `Coverage::Partial/None` (which can never promote, BG-D3) is shown live (M-3).

## 6. Assess figures (resolves I-5)

Uses the shipped **clamped** promote savings (never the unclamped reconstruction what-if), rendered in
the BG-D6 **three-flavor** discipline: computed-tax-Δ where the year's tables exist (2017/2024/2025/
2026); **gain-Δ with an uncomputable flag** for 2018–2023 (which are uncomputable forever and ARE the
audience years); named-but-unquantified where neither. Figures are computed once per entry/state-change
and **cached** (each `clamped_promote_year_saving` runs two full projections; the TUI draws at ~10 Hz).
The Declare live readout is limited to the cheap trio (floor/coverage/holding-date); tax-Δ recomputes
on demand, never per keystroke.

## 7. Forms / export (resolves I-4)

- **Callability:** the export is driven through the chokepoint (parameterized over `&Session`/state),
  not by re-opening a session.
- **Year-set:** the export set is **{current year} ∪ {advisory-flagged prior years}** (a promote
  routinely rewrites prior years — the BG-D9 1040-X path); the current-year-only default would silently
  drop the amended-year packets the promote consent told the filer about.
- **No pseudo-attest on the real path:** per C-2 + the standing DRAFT-gate policy, the journey refuses+
  routes rather than prompting the pseudo attest phrase.

## 8. Other resolutions

- **M-1 promote is one-tranche-at-a-time** (each needs its own consent figures + narrative +
  `Acknowledgment`; no bulk-promote — it would dilute BG-D6 informed consent).
- **M-2 Part II narrative authoring:** in-TUI multiline authoring with the BG-D7 non-empty/non-scaffold
  refusal enforced at the chokepoint (the spec picks the exact UX: in-TUI editor).
- **M-4 one-flow invariant:** follow the existing `EditorApp` dispatch-order conventions; add a debug
  assertion for the "at most one flow is `Some`" invariant.

## 9. Phasing (ONE ship gate; no installed base makes internal phasing free)

- **P-A** — the plan/confirm/apply chokepoint extraction (declare + promote + export) with the
  consent-parity KATs GATING it (the spine); fixes the C-2 CLI-side pseudo gap here too.
- **P-B** — the derived `journey_view` (core) + the dashboard (tui-edit) surfacing shortfalls +
  declared tranches + the fork + export status.
- **P-C** — era presets + the Declare flow's live floor/coverage/saving readout + safe-harbor pre-check.
- **P-D** — the Forms/export step (year-set, no pseudo-attest).
- If the shortfall→declare discovery balloons in spec review, it can split to a sub-project 2b rather
  than dragging the spine.

## 10. r1 finding → resolution (for the r2 reviewer to verify)

| r1 | resolution |
|----|-----------|
| C-1 wrong-object discovery | §2 dispositions-only + §4 shortfalls-as-candidates; existing lots routed, never declared |
| C-2 pseudo contamination | §5 journey gated on `!pseudo_active()`; CLI gap fixed at chokepoint + filed vs sub-1 |
| I-1 verbs unreusable across lock/session | §3.2 plan/confirm/apply chokepoints in cli + byte-identical consent parity |
| I-2 state machine in core | §3 split: computations→core, state machine→tui-edit, gating derived |
| I-3 linear bias toward promote | §4 dashboard + fork (not progress); `$0` never shown incomplete |
| I-4 Forms under-specified | §7 chokepoint callability + year-set + no pseudo-attest |
| I-5 Assess figures | §6 clamped-only + three-flavor + cached |
| I-6 guardrail copy wrong | §5 declare = `$0`/revocable/no-8275, plain confirm, unrecorded |
| I-7 era presets / safe-harbor | §5 confirm-edit presets + live readout + safe-harbor pre-check |
| M-1..M-5 | §8 (+ §4 export-not-a-done-step) |

## 11. Open questions for the r2 review

1. Does the **plan/confirm/apply chokepoint** extraction (I-1) fully resolve the reuse-vs-drift
   problem, and is the byte-identical-consent-parity acceptance criterion the right/sufficient gate?
2. Does **dispositions-only** scope (owner decision) leave any correctness hole — e.g. a shortfall the
   engine surfaces that is NOT a real missing-acquisition (a genuine over-disposal error the filer
   should FIX, not paper over with a tranche)? Should the dashboard distinguish "cover with a tranche"
   from "this is a data error to correct"?
3. Is the **dashboard-derived-from-state** resume model fully sound now (no persisted wizard state), or
   is there a re-entry/consistency trap left?
4. Any NEW issue the rework introduced (the chokepoint refactor touches shipped, reviewed sub-project-1
   code — does it risk regressing a BG-D* guarantee)?
