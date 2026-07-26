# Architecture review — Defensive Filing Wizard BRAINSTORM (r1, Fable lens)

**Artifact reviewed:** `design/defensive-filing-wizard/BRAINSTORM.md` (PRE-SPEC).
**Reviewer stance:** independent architecture critique, re-derived from current source at
`feat/conservative-filing` / v0.9.0-era tree. Every load-bearing claim below was checked against the
real code; file:line citations are to the current tree.

---

## Verdict

**The journey concept is right; the design as brainstormed NEEDS REWORK on two load-bearing elements
before it is spec-able.** The composition thesis — no new tax logic, shipped verbs, engine-enforced
gates — is the correct posture, and steps 2–4 (Declare → Assess → Promote) compose over primitives
that genuinely exist and are genuinely reusable. But (1) the **Find step's discovery semantics are
wrong at the object level** — it keys on existing `state.lots`, where *every* candidate its predicate
matches must NOT become a tranche and the feature's real audience signal (uncovered disposals /
unrecorded holdings) is absent from lots entirely — and (2) the **"reuse the shipped verbs" principle
is unimplementable as written across the session/lock seam**, so without an explicitly-owned
chokepoint refactor the wizard will silently become a second implementation of the promote pipeline —
the exact drift the principle exists to prevent. Both are cheap to fix now and expensive after a spec
is written around them. A third, softer shape issue — the linear 5-step framing treats the promote as
*progress* rather than a *fork* — should be resolved at the same time because it changes the top-level
UI shape (I recommend a derived "journey dashboard + ordinary sibling flows" over a modal super-flow).

---

## Critical

### C-1 — The Find step discovers the wrong object: every lot its predicate matches must NOT become a tranche, and the real signal is not a lot

**The hazard.** The brainstorm (§2, §3.1) keys discovery on `state.lots`: "propose ONLY lots the
engine itself cannot price ($0-basis / basis-pending / no documented `BasisSource`)", each becoming a
tranche after the typed guardrail. But `declare_tranche` does not *convert* a lot — it appends a
`DeclareTranche` decision that folds to a **NEW** `EstimatedConservative` lot homed at `window_end`
(`crates/btctax-cli/src/cmd/tranche.rs:120-175`; the payload carries only
`sat/wallet/window_start/window_end` — no reference to any existing lot). So "Find an existing lot →
Declare" **double-counts sats by construction**: the un-priceable source lot stays in the pool AND a
new tranche lot is minted for the same BTC — Σ-in inflates, disposals can now draw the phantom
duplicate, and the filing is wrong.

Trace the predicate against the actual `BasisSource` taxonomy (`crates/btctax-core/src/event.rs:17-37`
— note the enum is **total**: there is no "no documented `BasisSource`" lot state at all, so that leg
of the predicate matches nothing). What "$0-basis / basis-pending" actually surfaces:

1. **`EstimatedConservative` $0 lots — the already-declared tranches themselves, including the
   wizard's own output.** On re-entry, the wizard's step 1 would propose re-declaring the tranche it
   declared last session. The predicate is self-triggering.
2. **`basis_pending` lots** (`state.rs:124`): FMV-missing income (`fold.rs:792` — `(Usd::ZERO, true)`)
   and unknown-basis gifts. These have **documented non-purchase provenance** — exactly the
   gift/income class BG-D5 refuses — and they already **Hard-gate** any disposal that consumes them
   (`fold.rs:148-153`). Their remedy is `set-fmv` / basis resolution, never a tranche.
3. **`SelfTransferInbound` $0-default lots** (`event.rs:26-29`): the user-mandated conservative
   default ("my own coins", zero-basis, never-gates). The coins are already modeled; a tranche on top
   is a pure duplicate.
4. **Pseudo-tainted lots** (`lot.pseudo`, `state.rs:120-131`): synthetic, non-persisted provenance
   that "must NEVER reach any export file" — and must never seed a REAL declared position.
5. **Genuine $0-cost documented acquires** (an `ExchangeProvided`/`ComputedFromCost` lot with
   `usd_basis == 0`): real records that happen to say $0. A `usd_basis == 0` test cannot distinguish
   "no records" from "records say zero".

The worst corner is (2): BG-D5's provenance gate is enforced only through the filer's self-declared
`--provenance` (`cmd/promote.rs:34-43,380-383`); the vault itself KNOWS a `GiftCarryover`/
`FmvAtIncome` lot is not a purchase. A wizard that lists such a lot as a "no records" candidate and
then walks the filer to a purchase-provenance attestation is **manufacturing the BG-D5 lie** — the
product would be actively laundering documented non-purchase provenance into a purchase attestation.
BG-D5 at promote-time cannot catch it, because the filer will truthfully-feeling type "purchase" for
a tranche the wizard told them was "unpriceable BTC".

Meanwhile the feature's actual audience signal is **not a lot at all**. The Mt. Gox/LocalBitcoins
filer's vault has the *sales* imported and the *acquisitions* missing — which the engine already
surfaces as `BlockerKind::UncoveredDisposal` "dispose short by N sat" (`fold.rs:707-713`), and as
BTC the filer holds that the vault has never seen (representable only by asking the filer). Neither
appears in `state.lots`.

**Recommendation.** Re-spec Find as a **triage step over the right signals**, with existing lots
categorically excluded from the declare path:

- **Declare-tranche candidates:** `UncoveredDisposal` shortfalls (quantified per wallet/date — the
  engine already computes the sat shortfall) + a filer-entered "I hold N BTC with no acquisition
  records" manual entry. These are the only two shapes for which `declare_tranche`'s new-lot
  semantics are correct.
- **Route, don't declare, everything else:** `basis_pending` income → set-fmv flow; unknown-basis
  gift → gift-basis resolution; unclassified/unknown inbound → the classify flows; `EstimatedConservative`
  → the Assess/Promote steps (status "declared"). This turns Find into a genuinely useful
  "no-records triage" screen and kills the double-count and provenance-laundering shapes by
  construction.
- Pin with KATs: a vault whose only $0 lots are gift/income/self-transfer/pseudo yields ZERO declare
  candidates; a declared tranche never reappears as a candidate; a shortfall yields a candidate of
  exactly the shortfall sats.

### C-2 — Pseudo-mode contamination: the wizard's Assess figures and the recorded promote `Acknowledgment` can be folded over synthetic decisions

**The hazard.** Pseudo-reconcile rides `ProjectionConfig.pseudo_reconcile`, carried straight through
`CliConfig::to_projection()` (`crates/btctax-cli/src/config.rs:38-45`). `would_conflict` deliberately
forces pseudo OFF for record-time adjudication (`core/project/mod.rs:100-119` — "Never sees the
stored pseudo cfg's taint"), but **nothing in `conservative.rs` or `conservative_promote.rs` does**
(grep: zero `pseudo` references in either module). `consent_terms`, `clamped_promote_year_saving`,
and `promote_prior_year_advisory` all project with the config as given. The shipped CLI
`promote_tranche` passes `session.config()?.to_projection()` unmodified (`cmd/promote.rs:396,410-418`)
— so with pseudo mode active, the consent screen's figures fold over synthetic defaults, and the
recorded `Acknowledgment.shown_terms` — the §6664(c) good-faith artifact — immortalizes
pseudo-contaminated numbers on a REAL filed-position event. The `Lot.pseudo` bit's own doc says the
taint "must NEVER reach any export file" (`state.rs:130`); reaching a *recorded consent artifact* is
strictly worse. This is a **latent sub-project-1 gap** the wizard makes far more likely to be hit: the
wizard lives inside `btctax-tui-edit`, the surface where the pseudo-approve modal and pseudo banners
live, and its Assess step would naturally read the pseudo-honoring snapshot.

**Recommendation.** The wizard spec must (a) force pseudo OFF for every wizard computation (mirroring
`would_conflict`), or better (b) gate the journey on `!state.pseudo_active()` with routing guidance
("resolve/approve pseudo defaults first") — a defensive-filing journey over synthetic estimates is
incoherent, and the standing DRAFT-gate policy (attest stays pseudo-only) points the same way. Fix
the latent CLI-side gap **at the shared chokepoint** (see I-1) so both surfaces inherit it; file it
against sub-project 1 regardless of wizard scheduling.

---

## Important

### I-1 — "Reuse the shipped verbs" is unimplementable across the session/lock seam; without an owned chokepoint refactor the wizard becomes a second promote pipeline

The TUI holds the live `Session` — and therefore the exclusive `VaultLock` — for its whole lifetime
(`crates/btctax-tui-edit/src/editor.rs:8-14,79-89`). Every verb the brainstorm names reuses via
`Session::open(vault_path, pp)`: `declare_tranche` (`cmd/tranche.rs:148`), `promote_tranche`
(`cmd/promote.rs:374`), `export_snapshot`/`export_irs_pdf` (`cmd/admin.rs:133,350+`). Called from
inside the running editor they would hit `StoreError::Locked` — they are **not callable, period**.
`promote_tranche` additionally `println!`s the advisory and consent screen mid-function
(`cmd/promote.rs:443-455`) — poison in raw-mode.

The codebase already has the correct pattern for exactly this: the tranche⇄allocation guard
predicates are "the single source … for ALL FOUR allocation append sites (CLI … TUI
`persist_safe_harbor_allocate` + `persist_safe_harbor_attest`)" (`cmd/tranche.rs:5-9`), and the
viewer's export module calls the state-parameterized `promote_export_gate(state, events, year)`
directly (`btctax-tui/src/export.rs:202-206`; gate at `cmd/admin.rs:78-116`). The core computations
(`consent_terms`, `filed_basis_for`, `promote_prior_year_advisory`, `clamped_promote_year_saving`)
are already session-agnostic. What is NOT shared today is the CLI's glue — the gate **ordering**
contract (resolve-live → BG-D5 → BG-D7 → BG-D3 → BG-D6 → advisory → consent → ack → `would_conflict`
→ append), `gift_only_flagged_years` (**private**, `cmd/promote.rs:216`), `wide_window_note`
(private), and the refusal/consent copy.

**Recommendation.** The spec must own, as in-scope work: extract each verb into a session/state-
parameterized **plan → confirm → apply** chokepoint (in `btctax-cli`, which `btctax-tui-edit` already
depends on) with the CLI and the wizard as thin drivers. Non-negotiable acceptance criterion:
`Acknowledgment.shown_terms` and the rendered consent copy are **byte-identical** between surfaces
(the recorded §6664(c) artifact must equal what the filer saw, on either surface). Without this the
brainstorm's own claim "the two surfaces stay consistent by construction" is false — they would be
consistent by diligence.

### I-2 — The wizard state machine does not belong in `btctax-core`; a core "sequencing/gating" module creates a second gating authority

The brainstorm (§4) puts "step sequencing/gating" in core. Two problems. (a) It inverts the shipped
seam: all ~25 `*_flow` structs live in `btctax-tui-edit` (`editor.rs:16-30`); core has no notion of
steps, key dispatch, or typed-phrase buffers, and acquiring one smears presentation into the tax
kernel. (b) More dangerous: the *real* sequencing rules are already enforced by the engine and verbs
(can't promote a non-live tranche — `resolve_live_tranche` + `DecisionConflict`; can't export
undisclosed — BG-D8; can't declare pre-2025 beside an allocation — `guard_tranche_vs_allocation`). A
core wizard-sequencer would re-encode those rules in a second place that can drift — the dual-source
shape the answered-ness work just spent P9 eliminating.

**Recommendation.** Split it: **core** gets a pure, KAT-able, mutation-proven *derived journey
projection* — `fn journey_view(events, state, prices) -> DefensiveFilingView` (per-tranche status
declared/promoted + candidates per C-1 + savings figures) — plus the discovery predicate and the
era→window table. **`btctax-tui-edit`** gets the flow struct with `.step`, exactly like its siblings.
Step availability is *derived from engine refusals* (call the chokepoint's plan/guard fns), never
independently encoded. This keeps core presentation-free and the gating single-sourced.

### I-3 — The linear 5-step journey mis-models the decision structure and biases toward promote; a derived dashboard + sibling flows is the simpler, safer shape

Declared-at-$0 is a **complete, conservative end-state** (the v1 product), not step 2 of 5. A wizard
that renders "Find → Declare → Assess → Promote → Forms" as a progress sequence tells the filer a
non-promoted tranche is an unfinished journey — a structural nudge toward the aggressive end of the
curve, against BG-1's "filer choice, never a default" and the answered-ness principle the brainstorm
itself cites. The true structure is: triage → declare → **fork** (file $0 — done; or knowingly
promote + 8275), with Forms needed on *both* branches.

**Recommendation (also the "simpler design that delivers the same value"):** ship a **"Defensive
filing" dashboard** — a derived, read-only journey view (C-1 triage candidates, declared tranches
with clamped savings, promote status, export-gate status) whose rows *launch* ordinary sibling flows
(declare-tranche flow, promote flow, export). The dashboard IS the resume mechanism (fully derived
from state — no wizard-progress state to persist or invalidate), each flow stays independently
reachable and testable like every existing flow, the fork is presented as a fork, and the modal
super-flow — with its novel nested-dispatch and mid-journey-abandonment surface — disappears. If the
spec keeps the wizard framing, it must at minimum render Promote as an explicitly optional branch
("filing $0 is complete and conservative; promoting is a choice") and never show $0-declared as
incomplete.

### I-4 — Step 5 (Forms) is under-specified on three axes: callability, year-set, and pseudo/attest policy

(a) **Callability:** `export_irs_pdf` opens its own session (I-1) and has no TUI-side equivalent —
the only in-TUI export precedent is the viewer's CSV + `form_8275.txt` writer
(`btctax-tui/src/export.rs`), which `tui-edit` classifies as "viewer-only export surface"
(`edit/persist.rs:1918`). Producing "form_8275.pdf + f8949.pdf" from inside the editor is new
plumbing the spec must scope (parameterize the PDF path over `&Session`/state, or have step 5 emit
the viewer-precedent CSV+txt and hand off an exact CLI command for PDFs). (b) **Year-set:** a promote
routinely rewrites PRIOR years (the BG-D9 fold-diff advisory; the 1040-X path is an explicitly
supported flow). Which years does step 5 export? Exporting only the current year silently drops the
amended-year packets the promote's own consent screen told the filer about. The spec must define the
export set as {current year} ∪ {advisory-flagged years}, or explicitly hand off the flagged years.
(c) **Pseudo/attest:** `export_snapshot`/PDF exports require `ATTEST_PHRASE` when pseudo-active
(`cmd/admin.rs:143-145`). Per C-2 and the standing DRAFT-gate policy, the wizard should refuse+route
rather than prompt the pseudo attest phrase inside a defensive-filing journey.

### I-5 — Assess must adopt the three-flavor ConsentTerm discipline and clamped-only figures; and the live-savings readout is a projection-storm

(a) The brainstorm's Assess shows "`overpayment_delta` / `clamped_promote_year_saving` … 'filing $0
overpays by ~$X (year Y)'". Two corrections the shipped SPEC already litigated: `overpayment_delta`
is the **unclamped reconstruction** what-if — quoting it as promote value resurrects the tax r1 I-3
over-quote (the code itself keeps the two distinct: `overpayment_nudge_lines` quotes the unclamped
figure only as the *reconstruct* nudge and the CLAMPED figure for the promote funnel,
`conservative.rs:403-411,464-476`). And a bare "$X (year Y)" presumes the year computes — tables ship
only for 2017/2024/2025/2026, so **2018–2023 are uncomputable forever and are precisely this
feature's audience years**; Assess must display the computed-tax-Δ / gain-Δ-with-uncomputable-flag /
named-unquantified three-flavor discipline (BG-D6), not a single dollar rank. (b) **Performance:**
each `clamped_promote_year_saving` call runs TWO full projections (`conservative_promote.rs:487-507`);
the TUI draws at ~10 Hz and the existing code already had to short-circuit projections out of the
draw path (`conservative.rs:423-426`). Assess must compute once per entry/state-change and cache; the
Declare step's LIVE readout must be limited to the cheap trio — window-min floor, `Coverage`, holding
date (`window_reference` is a price-table scan) — with tax-Δ recomputed on demand, never per
keystroke.

### I-6 — The Find guardrail's copy is factually wrong, its assertion is unrecorded, and its tier contradicts the shipped confirmation taxonomy

The proposed typed guardrail says the declared units "will file at an ESTIMATED basis backed by Form
8275, a filed position you must defend". False on both counts for Declare: an unpromoted tranche
files at **$0** with the P7 methodology disclosure; **Form 8275 is promote-only** (BG-D7), and $0
"cannot understate gain" (the v1 posture). This copy front-loads the promote's consequences onto the
safe act — scaring filers off the conservative path or, read the other way, presupposing at step 1
that the promote (step 4) is already decided: a wizard answering for the filer. Also: the typed
assertion ("no acquisition records exist") is displayed but recorded nowhere — asymmetric with the
promote, whose `Acknowledgment` snapshots exactly what was shown; and the shipped tier taxonomy
reserves TypedWord for the irrevocable attest (`edit/form.rs:1537,2050-2051` — "creation is
REVOCABLE, so NO typed-word gate"), while `DeclareTranche` is revocable. **Recommendation:** fix the
copy (declare = $0, revocable, no 8275; promote is a later, separate choice); use the plain-modal
tier for Declare (matching the CLI verb, which gates declare on nothing beyond validation); reserve
the typed phrase for the promote step where it mirrors `PROMOTE_ACK_PHRASE`; decide deliberately
whether the no-records assertion should be recorded (if yes, that is a schema change and the "no new
tax logic" claim must be amended — I recommend NOT recording it and wording it as an ordinary
confirmation).

### I-7 — Era presets are new reference data with G-4 exposure, and they collide with the safe-harbor mutual exclusion at exactly the eras they target

No era table exists anywhere in the tree (grep: nothing) — this is NEW product-authored reference
data, and its shape has anti-overstatement stakes: a preset that seeds a *tight* window seeds a
*high* floor, and BG-D3/G-4's honest direction is filer-widened windows ("set `window_start` early
enough to actually cover the purchase"). Presets must be framed as starting points the filer
confirms/edits (the live wider-window-lower-floor readout the brainstorm already requires is the
right mitigation — keep it mandatory), and the preset table needs the same review rigor as copy.
Separately: the presets are by definition pre-2025 eras, and a pre-2025 `DeclareTranche` is
**refused while an in-force `SafeHarborAllocation` exists** (`guard_tranche_vs_allocation`,
`cmd/tranche.rs:104-118`). The wizard must pre-check this at Find/Declare entry and present it as a
first-class state — not let the filer pick lots, type the guardrail, choose an era, and then bounce
off a refusal at the final Enter.

---

## Minor

- **M-1 — No bulk promote.** Find will propose multiple candidates; Declare can be per-tranche
  sequential, but the spec should state explicitly that promotion is one-tranche-at-a-time (each
  needs its own consent figures, narrative, and `Acknowledgment`) — a bulk-promote would dilute
  BG-D6's informed consent. The bulk-* flow precedents make this an easy accidental "improvement".
- **M-2 — Part II narrative authoring is unspecced UX machinery.** The CLI takes `--part-ii-file`;
  the wizard needs an in-TUI multiline authoring path with the non-empty/non-scaffold refusal
  (BG-D7) enforced at the same chokepoint. Options (in-TUI editor vs suspend-to-$EDITOR vs refuse-
  and-point-at-CLI) have materially different scope — the spec must pick one.
- **M-3 — Show `Coverage` live in Declare.** A window with `Partial`/`None` coverage declares fine
  but can never promote (BG-D3 hard refusal, `cmd/promote.rs:133-148`). The live readout should show
  coverage alongside the floor so a filer intending to promote doesn't discover the dead end at
  step 4.
- **M-4 — Dispatch-chain growth.** The wizard adds flow state to an `EditorApp` whose "at most one
  flow is `Some`" invariant is held by convention across ~25 hand-ordered fields
  (`editor.rs:110-266`). Follow the existing dispatch-order conventions exactly; consider this the
  moment to add a debug assertion for the one-flow invariant rather than more comment-law.
- **M-5 — Journey "done"-ness for step 5 is not derivable.** Nothing in the vault records that an
  export happened (exports write files, not events). The dashboard/wizard must present Forms as
  always-available, never as a checkable "done" step — or the resumability story quietly acquires a
  side-table the design says it doesn't need.

---

## Answers to the five stress questions

**1. Full journey as ONE sub-project?** Keep **one sub-project (one ship gate)** — no installed base
makes internal phasing free (the sub-project-1 precedent) — but the brainstorm underestimates where
the work is. The promote/consent core is shipped; the real scope is: the I-1 chokepoint refactor of
reviewed code (touches `cmd/tranche.rs`, `cmd/promote.rs`, export), the C-1 Find/triage respec (new
semantics, new signals), era-preset reference data (I-7), TUI narrative authoring (M-2), and export
plumbing (I-4). Phase internally as: **P-A** chokepoint extraction + Declare & Promote flows (the
spine — consent-parity KATs gate it); **P-B** Assess + dashboard (I-3/I-5); **P-C** Find/triage +
era presets (the piece whose design must change most — last, not first); **P-D** Forms. If C-1's
respec balloons during spec review, split Find into sub-project 2b rather than letting it drag the
spine.

**2. Resumability-via-events sound?** **Sound for steps 2–4, and only there — and only if re-entry is
fully derived.** Declared-not-promoted is a legitimate, complete $0 filing state (not hazardous —
but see I-3: it must not render as unfinished). Promoted-not-exported is engine-safe: BG-D8 gates
every export surface (`promote_export_gate` is called refuse-before-bytes on all of them), and an
interrupted wizard leaves no gate un-armed because the gates live in the engine/verbs, not the
wizard. `would_conflict` + `resolve_live_tranche` make re-entry idempotent for promote. The two
places the claim overreaches: step 1's filer-supplied facts (unrecorded holdings) and step 5's
done-ness (M-5) are NOT in the event log — the design must say so and derive nothing from remembered
wizard state. One sharp re-entry trap to pin: the Find predicate as-written matches the wizard's own
previously-declared tranches (C-1 item 1) — re-entry would re-propose them. Discovery must exclude
`EstimatedConservative` by construction, with a KAT.

**3. What does "engine can't price" surface?** See C-1 — this is the review's Critical. Precisely:
`EstimatedConservative` (the declared tranches themselves), `basis_pending` gift/income lots
(documented NON-purchase provenance, BG-D5's refused class, already Hard-gating), `SelfTransferInbound`
$0-defaults (user-mandated policy, already modeled), pseudo-tainted lots (synthetic), and genuine
$0-cost documented acquires. "No documented `BasisSource`" is unrepresentable (the enum is total).
**Yes, discovery must pre-exclude — but the deeper fix is that existing lots are never declare
candidates at all** (declare mints a new lot; converting is not a shipped verb, and adding one would
be new tax logic). Discovery's positive signals are `UncoveredDisposal` shortfalls and filer-asserted
unrecorded holdings; everything else routes to its existing remedial flow. BG-D5 at promote-time is
the filer's self-attestation and cannot defend against a wizard that proposed a vault-documented gift
lot as "no records" — pre-exclusion is the only sound layer.

**4. Pure-core / thin-TUI split?** Half right. Pure *computations* (discovery predicate, era table,
floor/saving, the derived journey view) → core, KAT-able, mutation-proven: correct and consistent
with the existing signal modules. The *state machine* → **not core** (I-2): flows live in
`btctax-tui-edit` by shipped convention, core has no UI vocabulary, and a core sequencer would
re-encode gating the engine already enforces — a second authority that drifts. The missing third
layer is the real seam: session-parameterized **plan/confirm/apply chokepoints in `btctax-cli`**
(I-1), which both the CLI verbs and the wizard drive — the four-allocation-append-sites pattern,
already proven in this codebase.

**5. Coexistence hazards?** Four concrete ones: (a) **pseudo-mode** — C-2, the worst: consent/Assess
figures and the recorded `Acknowledgment` fold over synthetic decisions unless pseudo is forced off
or the journey is gated on `!pseudo_active()`; also the export attest phrase (I-4c). (b) **The
tranche⇄safe-harbor mutual exclusion** — era presets aim at exactly the pre-2025 window that
`guard_tranche_vs_allocation` refuses beside an in-force allocation (I-7); surface it at entry.
(c) **The DRAFT gate / export policy** — exports stay clean (no watermark) per standing policy; the
wizard must not reintroduce attestation on the real path, and must resolve the year-set question
(I-4b) so prior-year 1040-X packets aren't silently dropped. (d) **The CLI promote flow** — no
lock-level race (the `VaultLock` makes TUI/CLI mutually exclusive, `editor.rs:8-14`), but consent-
artifact parity across surfaces is required (I-1's byte-identical `shown_terms` criterion); without
the chokepoint, the wizard could record an `Acknowledgment` whose figures differ from what a CLI
promote of the same tranche would have shown — two surfaces filing different good-faith evidence for
the same position. The answered-ness invariant is respected by the brainstorm's explicit-act design
EXCEPT at the two points named in I-3 (progress framing pre-answers "should I promote?") and I-6
(the Find copy pre-answers it the other way).

---

*End r1. Verdict: NEEDS REWORK before SPEC — resolve C-1 and C-2, and settle I-1/I-2/I-3 (they
determine the spec's top-level shape). The remaining Importants can be resolved in the spec itself.*
