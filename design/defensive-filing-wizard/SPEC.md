# Defensive Filing — SPEC (Approach-B sub-project 2)

**Status:** SPEC (first draft), to pass the independent two-lens (tax + architecture) review loop to
0 Critical / 0 Important before a plan is written. Adjudicated from `DESIGN.md` after two Fable-architect
critiques (`reviews/brainstorm-architecture-fable-review-r{1,2}.md`; r2 verdict: SOUND to proceed).
**Branch:** `feat/defensive-filing-wizard` (off `main` @ v0.9.0).
**Lineage:** Approach-B sub-project 2 — the guided actuator the conservative-filing **G-3** promised.
Sub-project 1 (PromoteTranche floor + official Form 8275 PDF) shipped **v0.9.0**. It adds **NO new tax
logic**: it composes shipped, reviewed primitives; every filing gate stays engine-enforced.

---

## 1. Purpose & guardrails

A **derived "Defensive filing" dashboard** in `btctax-tui-edit` for the filer whose **sales are
imported but purchases are gone** (Mt. Gox / LocalBitcoins / old-wallet). It surfaces the dispositions
that need a defensible basis and, per disposition, walks: **triage** (cover-with-a-tranche vs
fix-the-data) → **declare** a tranche that provably covers the shortfall → **fork** (file `$0` =
complete + conservative, *or* knowingly **promote** to a `>$0` floor + Form 8275) → **export** the
packet.

Guardrails (inherited, non-negotiable):
- **G-1 filer choice, never a default.** Promoting is a choice; a `$0`-declared filing is a complete,
  conservative end-state, never rendered as unfinished.
- **The answered-ness invariant.** No step silently answers for the filer.
- **No new tax logic.** All mutation goes through the shipped verbs' extracted chokepoints; all gates
  (BG-D1..D11) stay engine-enforced.

## 2. Object scope (owner decision, binding)

Basis is a **disposition-time** concept. This feature acts ONLY on BTC **sold or given away with no
purchase record** — the engine's `BlockerKind::UncoveredDisposal` *sat shortfalls*. **Unsold no-records
holdings are OUT of scope** (no current-return effect; engine-invisible; forward-planning only). There
is **no manual "I hold N BTC" data model.**

---

## 3. Binding decisions (defend these against a review flip)

- **DFW-D1 (architecture — three seams).** (a) `btctax-core`: pure, KAT-able, mutation-proven
  computations only — a derived `journey_view`, the shortfall→candidate discovery, the era→window
  table; NO UI, NO session. (b) `btctax-cli`: **plan/confirm/apply chokepoints** — the sole home of the
  verb glue (gate ordering, private helpers, consent render). (c) `btctax-tui-edit`: the dashboard + the
  `*_flow` structs, thin drivers over (a)+(b); step availability **derived from the chokepoint's
  plan/guard results, never independently encoded** (no second gating authority — I-2).

- **DFW-D2 (the chokepoint contract — I-1).** Each mutating verb (`declare_tranche`, `promote_tranche`,
  the crypto-slice + full-return export) is extracted into a session/state-parameterized
  `plan(state, events, …) -> Plan | Refusal` / `render_consent(&Plan) -> String` / `apply(&mut Session,
  Plan) -> …` trio in `btctax-cli`. BOTH the CLI verb and the dashboard are thin drivers. The **gate
  ordering** is written ONCE and MUST match the shipped promote pipeline exactly:
  resolve-live → BG-D5 provenance → BG-D7 Part II → BG-D3 floor/coverage → BG-D6 `consent_terms` →
  synthetic-promote advisory → gift-only relabel → consent render (incl. `wide_window_note`) → ack →
  `would_conflict` → append. **`would_conflict` stays inside `apply`.**
  - **★ Acceptance (hard):** the recorded `Acknowledgment.shown_terms` AND the rendered consent copy are
    **byte-identical** between the CLI and the dashboard for the same tranche/state — the §6664(c)
    good-faith artifact must equal what the filer saw, on either surface.
  - **★ Parity KATs cover the REFUSAL paths and advisory lines too** (N-4), incl. the shipped "consent
    is printed BEFORE the ack gate, so a refused ack still surfaces the figures" contract
    (`promote.rs:451-458`) — not just the happy path.
  - **Plan→apply staleness (N-4):** state MUST NOT mutate between `plan` and `apply` (TUI: the one-flow
    invariant + single-threaded event loop, with a debug assertion — M-4; CLI: one call). Stated
    explicitly; `would_conflict` in `apply` is the backstop.

- **DFW-D3 (dashboard = fork, not progress — I-3).** A derived, read-only journey view whose ROWS
  launch ordinary sibling flows — NOT a modal linear super-flow. It IS the resume mechanism (fully
  derived from state; nothing persisted). The `$0`/promote choice is rendered as **two equal branches**;
  a `$0`-declared tranche is NEVER incomplete; promote is never a default (G-1); export is never a
  checkable "done" (exports write files, not events — M-5).

- **DFW-D4 (triage — cover-with-a-tranche vs fix-the-data — N-1).** A shortfall is NOT always a missing
  acquisition. The "Needs a basis" section MUST:
  1. **Exclude the without-wallet `UncoveredDisposal` variants** ("dispose/pending-out/self-transfer
     without wallet") from declare candidates — they carry no sat quantity and a tranche cannot fix
     them; route them as data-fixes.
  2. When **acquisition-shaped blockers** (`UnknownBasisInbound`/unknown-basis, `Unclassified`,
     `ImportConflict`, `UnmatchedOutflows`) are open on the same pool/timeframe as a shortfall, surface
     them **FIRST** with "resolve these — the shortfall may disappear" routing (to the shipped set-fmv /
     classify / reconcile flows), **before** offering declare. This prevents declare-then-classify from
     re-minting the C-1 double-count at the blocker level.
  3. Word the declare confirmation to assert the coins were **acquired entirely outside the vault's
     records** (not merely "unpriced").

- **DFW-D5 (coverage is emergent — the declare candidate must provably cover — N-2).** Declaring a
  tranche does not guarantee it covers the shortfall. The declare chokepoint MUST:
  1. **Prefill** `window_end` strictly **before** the earliest short disposal (decisions sort AFTER
     same-instant imports — `resolve.rs:1308-1312`) and `wallet` = the disposal's wallet (post-2025
     `Wallet(w)` pools; Path-A re-homes each residual to `lot.wallet` — `transition.rs:93-106`).
  2. Perform a **plan-time clearance check**: append-the-candidate → re-project → assert the targeted
     `UncoveredDisposal` shortfall cleared (the `would_conflict` shadow-projection pattern; cheap at
     record-time frequency). A candidate that would NOT clear is a refusal with a reason, not a silent
     append.
  3. Surface a **first-class "declared-but-didn't-cover" dashboard state** for a live tranche that did
     not clear its shortfall (else the two rows render disconnected and the filer's natural move is to
     declare AGAIN → a second phantom lot).

- **DFW-D6 (pseudo gate — C-2).** The whole journey is gated on **`!state.pseudo_active()`** with
  routing guidance ("resolve/approve pseudo defaults first"); a defensive-filing journey over synthetic
  estimates is incoherent. The **latent CLI-side gap** (consent/savings folding pseudo numbers into the
  recorded `Acknowledgment`) is fixed at the shared chokepoint (force `pseudo_reconcile=false` for the
  consent/savings computation, mirroring `would_conflict`) so BOTH surfaces inherit it — **filed against
  sub-project 1 regardless of this feature's scheduling** (see §8). The candidate signal itself is
  pseudo-stable (pseudo-reconcile does NOT clear `UncoveredDisposal`).

- **DFW-D7 (structured shortfall signal — N-3).** `journey_view` MUST consume a **structured** shortfall
  record `{event, wallet, date, short_sat}` (a small derived `state` signal or a recompute inside
  `journey_view`) — it MUST NOT parse `Blocker.detail`'s display string. Derived state only; no new tax
  logic.

- **DFW-D8 (declare guardrail — I-6).** Declaring is **`$0`, revocable, NO Form 8275** — a plain
  confirmation matching the shipped verb (which gates declare on input validation + the allocation guard
  only). The typed-phrase tier is reserved for PROMOTE (mirrors `PROMOTE_ACK_PHRASE`). The no-records
  assertion is worded as an ordinary confirmation and **NOT recorded** (recording it would be a schema
  change / new tax logic).

- **DFW-D9 (era presets + safe-harbor — I-7, M-3).** Presets are confirm/edit **starting points**, not
  authoritative windows; a **mandatory live readout** shows the resulting window-min floor + `Coverage`
  + holding-date + clamped saving as the filer edits (wider window → lower floor, the conservative
  direction, made visible). `Coverage::Partial/None` (can never promote — BG-D3) is shown live. The
  **safe-harbor exclusion** (`guard_tranche_vs_allocation`: a pre-2025 `DeclareTranche` is refused
  beside an in-force `SafeHarborAllocation`) is a **first-class dashboard state at entry**, not a final-
  Enter surprise. The preset table gets copy-level review rigor.

- **DFW-D10 (Assess figures — I-5).** Uses the shipped **clamped** promote saving (never the unclamped
  reconstruction what-if), in the BG-D6 **three-flavor** discipline: computed-tax-Δ where tables exist
  (2017/2024/2025/2026); **gain-Δ + uncomputable flag** for 2018–2023 (uncomputable forever and the
  audience years); named-but-unquantified otherwise. Figures **computed once per entry/state-change and
  cached** (each `clamped_promote_year_saving` = two full projections; TUI draws ~10 Hz). The Declare
  live readout is limited to the cheap trio (floor/coverage/holding-date); tax-Δ recomputes on demand,
  never per keystroke.

- **DFW-D11 (Forms/export — I-4, N-5).** Export is driven through the chokepoint (parameterized over
  `&Session`/state), NOT a second `Session::open`. The export set is **{current year} ∪ {years in which
  a promoted disposal leg files}**, recomputed from state at export time via the
  `promote_export_gate(None)` enumeration — never a remembered promote-time advisory. Refuse+route
  instead of prompting the pseudo attest phrase on the real path (standing DRAFT-gate policy).

- **DFW-D12 (one-tranche-at-a-time promote — M-1).** Promotion is per-tranche: each needs its own
  consent figures, Part II narrative, and `Acknowledgment`. **No bulk-promote** (it would dilute BG-D6
  informed consent). Part II narrative authoring (M-2) is an in-TUI multiline path with the BG-D7
  non-empty/non-scaffold refusal enforced at the chokepoint.

---

## 4. Non-goals (this sub-project)

Unsold no-records **holdings** (§2); a manual holdings-entry model; VARIOUS multi-date rows / 8275
pagination (sub-project 3); partial-sat promotion (whole-tranche only — unchanged from sub-project 1);
tranche⇄safe-harbor coexistence (surfaced + refused, never made to coexist); inherited/§1014 or
documented-basis floors; AMT/non-BTC; a CLI wizard front-end (the chokepoints make one possible later,
but it is not in scope).

## 5. Test / green definition

Every primitive TDD + mutation-proven; full suite + all CI-only jobs green; SPEC + downstream plan
reviewed to **0C/0I under BOTH the tax and architecture lenses** before merge. **Explicit KATs (min.):**

- **DFW-D2 parity:** for a fixture tranche+state, `render_consent` and the recorded
  `Acknowledgment.shown_terms` are byte-identical whether produced by the CLI verb or the dashboard
  driver — on the happy path AND on a refused-ack path AND on each refusal path (BG-D5/D3/D7). Mutation:
  perturb one consent line in one driver → the parity KAT reds.
- **DFW-D4 triage:** a vault whose only shortfall is caused by an open `Unclassified`/`UnknownInbound`
  surfaces the classify remedy FIRST and offers NO declare candidate for it; a without-wallet
  `UncoveredDisposal` yields ZERO declare candidates; a genuine sold-short-by-N vault yields exactly one
  candidate of N sat.
- **DFW-D5 coverage:** a candidate whose prefilled `window_end`/`wallet` would NOT clear the shortfall
  is refused with a reason (mutation: prefill `window_end == disposal date` → clearance check reds); a
  declared tranche that clears removes the shortfall row; a live-but-didn't-cover tranche renders the
  linked "didn't cover" state (never a bare second candidate).
- **DFW-D6 pseudo:** with pseudo active, the journey refuses+routes and NO wizard computation folds a
  pseudo number; the chokepoint consent/savings force pseudo off (the sub-project-1 latent-gap KAT).
- **DFW-D3 fork:** a `$0`-declared tranche renders complete (never "incomplete/step N of M"); promote is
  an explicit optional branch; export is always-available, never "done".
- **DFW-D7:** `journey_view` reads the structured `{event,wallet,date,short_sat}` (no `Blocker.detail`
  string parse — grep guard).
- Plus the shipped BG-D1..D11 KATs remain green (the chokepoint extraction is behavior-preserving).

## 6. Design provenance

Brainstormed 2026-07-22 (dialogue: surface=TUI-edit; scope→dispositions-only; discovery; window). Fable-
architect **r1** critique → NEEDS REWORK (2C/7I). Adjudicated (`DESIGN.md`) + owner scope decision
(dispositions-only). Fable-architect **r2** → SOUND to proceed; +N-1/N-2 (Important, folded here as
DFW-D4/D5) +N-3/N-4/N-5 (folded as DFW-D7 / DFW-D2 clauses / DFW-D11). Reviews persisted verbatim in
`reviews/`.

## 7. Phasing (ONE ship gate; internal phases free — no installed base)

- **P-A** — the plan/confirm/apply chokepoint extraction (declare + promote + export) with the DFW-D2
  consent-parity KATs GATING it; fixes the C-2 CLI-side pseudo gap here (DFW-D6).
- **P-B** — the derived `journey_view` (core) + the dashboard (tui-edit): shortfalls, DFW-D4 triage,
  declared tranches, the fork, export status.
- **P-C** — era presets + the Declare flow's live floor/coverage/saving readout + DFW-D5 prefill &
  clearance check + safe-harbor pre-check.
- **P-D** — the Forms/export step (DFW-D11 year-set, no pseudo-attest).
- If DFW-D4/D5's discovery/clearance semantics balloon in review, they may split to sub-project 2b
  rather than dragging the P-A spine.

## 8. Cross-references / follow-ups

- **★ File against sub-project 1 (independent of this feature):** the CLI `promote_tranche` can already
  fold pseudo numbers into the recorded `Acknowledgment` (DFW-D6 / C-2). Fix at the shared chokepoint;
  add the latent-gap KAT. This is a real (if narrow) sub-project-1 defect, not new to the wizard.
- See `[[conservative-filing-approach-b]]`, `[[answeredness-invariant]]`, `[[self-transfer-completion-policy]]`,
  `[[full-return-draft-gate-policy]]`, `[[tax-authority-hierarchy]]`.
