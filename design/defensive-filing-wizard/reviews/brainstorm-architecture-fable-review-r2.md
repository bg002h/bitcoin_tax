# Architecture review — Defensive Filing DESIGN.md (r2, Fable lens)

**Artifact reviewed:** `design/defensive-filing-wizard/DESIGN.md` (PRE-SPEC, adjudicated rework of the
BRAINSTORM after my r1). **Stance:** independent re-review, re-derived from current source on
`feat/conservative-filing`; every load-bearing design claim was re-checked against the real code, not
taken from §10's self-claimed table. Citations are to the current tree.

---

## Verdict

**SOUND to proceed to SPEC.** All r1 findings — both Criticals, all seven Importants, all five Minors —
are genuinely resolved at design altitude, not merely cited. The rework's three-seam architecture
(pure `journey_view` in core; plan/confirm/apply chokepoints in `btctax-cli`; flows/dashboard in
`btctax-tui-edit`) matches the shipped seams I verified, and the dispositions-only owner decision kills
the C-1 double-count and provenance-laundering shapes **by construction**, not by filtering.

Two **new Important findings (N-1, N-2)** emerged — both are consequences the shortfall-as-candidate
model *unlocks* rather than defects it *reintroduces*, both are spec-resolvable without changing the
design's shape, and both must be owned by the SPEC (they bind the discovery/declare semantics, i.e.
P-B/P-C). This is the same posture as my r1 close ("the remaining Importants can be resolved in the
spec itself"). No new Critical. Proceed.

---

## r1 finding → resolution audit

### C-1 (wrong-object discovery) — **RESOLVED**, with two spec-owned residues (see N-1, N-2)

The rework's §2/§4 shape — `UncoveredDisposal` shortfalls are the ONLY declare candidates; existing
lots are categorically routed, never declared — eliminates every hazardous shape I enumerated:

- **Self-triggering predicate:** dead. A declared tranche is a lot, not a shortfall; once its lot
  covers the disposal the blocker itself disappears from the projection, so the candidate list is
  self-clearing by construction (the fold only emits the blocker when `consume` comes up short,
  `fold.rs:706-713`).
- **BG-D5 provenance laundering:** dead. `basis_pending` gift/income lots and `SelfTransferInbound`
  $0-defaults are lots, hence categorically outside the candidate set; §4 routes them to set-fmv /
  gift-basis / classify (all existing flows — `SetFmvFlowState`, `ClassifyInboundFlowState` et al.,
  `editor.rs` imports). The wizard can no longer walk a vault-documented non-purchase into a purchase
  attestation.
- **Double-count at the lot level:** dead. Nothing that already exists as a lot ever mints a second
  lot.
- **Owner scope cut** (no manual "I hold N BTC" entry): a clean *narrowing*. r1 named exactly two
  shapes for which `declare_tranche`'s new-lot semantics are correct; the owner kept one and cut the
  other. No correctness hole opens — unsold unrecorded holdings have no current-return effect, exactly
  as §2 argues.

**Is a shortfall the correct-and-sufficient declare candidate?** Correct, yes — the shortfall is
precisely "this disposal drew N sat the ledger has never seen," which is the definition of a missing
acquisition, and the minted `EstimatedConservative` lot at `window_end` is the shipped, reviewed
answer to it. Sufficient, **only under conditions the design does not yet state** — coverage is
*emergent from the fold*, not guaranteed by the declaration:

1. **Date:** the tranche folds at `window_end`, and decisions sort AFTER same-instant imports
   (`resolve.rs:1308-1312` — `src_priority: u8::MAX, // decisions sort after same-instant imports`).
   A tranche with `window_end == disposal date` folds after the disposal and does NOT cover it. The
   candidate prefill must pin `window_end` strictly before the earliest short disposal.
2. **Pool/wallet:** pre-2025 both sides land in `PoolKey::Universal` (`pools.rs:15-21`) — wallet is
   irrelevant. A post-2025 disposal draws `Wallet(w)`; a pre-2025-window tranche reaches it only via
   the Path-A transition re-home, which routes each residual lot to **its own `lot.wallet`**
   (`transition.rs:93-106`), so the candidate must prefill `wallet` = the disposal's wallet. (Path B
   is excluded by the shipped mutual exclusion + the `estimated_conservative_remaining_sat` backstop,
   `transition.rs:22-27` — consistent with §5's safe-harbor pre-check.)
3. **Nothing verifies clearance today:** `declare_tranche` validates only `sat > 0` and window order
   (`cmd/tranche.rs:134-146`); whether the blocker actually clears is discovered only on
   re-projection.

These are N-2 below (Important, spec-owned). They are refinements the correct object-choice *exposed*,
not a survival of the r1 defect.

### C-2 (pseudo contamination) — **RESOLVED**

Re-verified the premises: `conservative.rs` and `conservative_promote.rs` contain **zero** `pseudo`
references (grep, exit 1); `CliConfig::to_projection` carries `pseudo_reconcile` straight through
(`config.rs:38-45`); the shipped `promote_tranche` uses `session.config()?.to_projection()` unmodified
(`cmd/promote.rs:396`); `would_conflict` is the lone precedent that forces `cfg.pseudo_reconcile =
false` (`project/mod.rs`, "Never sees the stored pseudo cfg's taint"). The design's two-layer answer —
gate the journey on `!state.pseudo_active()` (`state.rs:290`) with routing guidance, AND fix the latent
CLI gap at the shared chokepoint, filed against sub-project 1 regardless of scheduling — is exactly the
r1 recommendation, and §7's no-pseudo-attest-on-the-real-path closes the I-4c corner. One nuance in the
design's favor: `pseudo_active()` is contribution-based (mode ON with zero contributing synthetics
passes), which is the *precise* predicate — and the chokepoint pseudo-off fix backstops even that case.
Also note the discovery signal itself is pseudo-stable: pseudo-reconcile explicitly does NOT clear
`UncoveredDisposal` (`tests/pseudo_reconcile.rs:343`), so the dashboard's candidate list cannot be
pseudo-distorted in the first place. Sound.

### I-1 (verbs unreusable across the lock/session seam) — **RESOLVED**

- **Feasibility:** `btctax-tui-edit` depends on `btctax-cli` (`btctax-tui-edit/Cargo.toml:19`), and the
  state-parameterized pattern is already shipped twice: `promote_export_gate(state, events, year)`
  (`cmd/admin.rs:78-116`) called from `btctax-tui/src/export.rs`, and the four-allocation-append-sites
  guard chokepoint (`cmd/tranche.rs:5-9`). The extraction is an extension of a proven pattern, not a
  novel seam.
- **Fidelity of the ordering contract:** I re-derived the shipped promote pipeline line-by-line
  (`cmd/promote.rs:364-488`): resolve-live (378) → BG-D5 provenance (381) → BG-D7 Part II (386) →
  BG-D3 floor/coverage (397) → BG-D6 `consent_terms` (410) → synthetic-promote advisory (422-445) →
  gift-only relabel (449) → consent render + `wide_window_note` (453-456) → ack (458) →
  `would_conflict` (477) → append (485). The design's §3.2 ordering table matches this **exactly**. The
  private helpers r1 named (`gift_only_flagged_years` :216, `wide_window_note` :153) and the raw-mode
  `println!` poison (:443-455) are precisely what the plan/confirm/apply split absorbs.
- **Is byte-identical `Acknowledgment.shown_terms` + consent copy the right acceptance gate?** Yes —
  it is the strongest *checkable* proxy for the guarantee that matters (the recorded §6664(c) artifact
  equals what the filer saw, on either surface), and equality-of-artifact is mechanically KAT-able
  where "semantic parity" is not. Two sharpening notes, folded into N-4: the parity KATs must cover
  the **refusal paths and advisory lines** too (the shipped verb deliberately prints the consent
  screen BEFORE the ack gate so a refused ack still surfaces figures — `promote.rs:451-453`, the N-2
  contract from sub-project 1's review), and the plan→apply staleness contract must be stated.
- **Regression risk of refactoring shipped code:** real but contained — see §11 answer 4.

### I-2 (state machine in core / second gating authority) — **RESOLVED**

§3's split is exactly the r1 recommendation: core gets the pure, KAT-able `journey_view` + discovery
predicate + era table; the `*_flow` structs stay in `btctax-tui-edit` beside their ~25 siblings; step
availability is derived from the chokepoint's plan/guard results. No second gating authority is
created; the engine's gates (`resolve_live_tranche`, BG-D8, `guard_tranche_vs_allocation`) remain the
only encodings. One signature note in passing: `journey_view` will also need the tax tables handle
(the savings flavors need them, as `consent_terms` does) — a spec-level detail, not a finding.

### I-3 (linear bias toward promote) — **RESOLVED**

§4 is the dashboard-not-super-flow shape: derived read-only rows launching ordinary sibling flows, the
$0/promote fork rendered as two equal branches, `$0`-declared never rendered incomplete, promote never
a default (BG-1), export never a checkable "done" (M-5 folded). The modal super-flow and its
nested-dispatch surface are gone.

### I-4 (Forms under-specified) — **RESOLVED**

All three axes answered: callability via the chokepoint (parameterized over `&Session`/state, not a
second `Session::open`); year-set = {current year} ∪ {advisory-flagged prior years}, which keeps the
BG-D9 1040-X packets; refuse+route instead of pseudo-attest. One operational sharpening (N-5, Minor):
"advisory-flagged prior years" must be *recomputed from state* at export time (the years a promoted
disposal leg files in — the enumeration `promote_export_gate(None)` already performs,
`cmd/admin.rs:84-98`), never a remembered advisory from promote time — otherwise the derived-from-state
principle acquires its first exception.

### I-5 (Assess figures) — **RESOLVED**

Clamped-only (never the unclamped reconstruction what-if), the BG-D6 three-flavor discipline with
2018–2023 named as uncomputable-forever audience years, compute-once-per-state-change caching, and the
Declare live readout limited to the cheap trio with tax-Δ on demand. Matches the shipped code's own
distinction (`overpayment_nudge_lines` clamped-vs-unclamped split) and the projection-cost reality
(two full projections per `clamped_promote_year_saving`).

### I-6 (guardrail copy / tier) — **RESOLVED**

Declare = `$0`, revocable, NO Form 8275, plain confirmation — verified against the verb, which gates
declare on input validation + the allocation guard only (`cmd/tranche.rs:134-154`); typed phrase
reserved for promote (mirrors `PROMOTE_ACK_PHRASE`); the no-records assertion worded as an ordinary
confirmation and deliberately NOT recorded (no schema change). Matches the shipped tier taxonomy.

### I-7 (era presets / safe-harbor collision) — **RESOLVED**

Presets are confirm/edit starting points with a mandatory live floor/`Coverage`/holding-date readout
(wider window → lower floor made visible — the honest direction), the preset table gets copy-level
review rigor, and the safe-harbor exclusion is a first-class dashboard state at entry — verified
against `guard_tranche_vs_allocation` (`cmd/tranche.rs:107-118`: pre-2025 `window_end` + in-force
allocation → refuse). No era table exists in the tree (re-grepped — still absent), so the design
correctly treats it as new product-authored reference data. M-3 (live `Coverage`, the can-never-promote
dead end) is folded in.

### M-1..M-5 — **ALL RESOLVED**

M-1 one-tranche-at-a-time promote (§8); M-2 in-TUI narrative authoring with the BG-D7 refusal at the
chokepoint, option picked (§8); M-3 live Coverage (§5); M-4 dispatch-order conventions + the one-flow
debug assertion (§8); M-5 export-never-a-done-step (§4).

---

## New findings

### N-1 (Important) — A shortfall is not always a missing acquisition: the triage must distinguish "cover with a tranche" from "fix the data", or C-1's double-count re-enters at the blocker level

The design's routing table (§4) is complete over **lots** but silent over **blocker-shaped acquisition
gaps**, and `BlockerKind::UncoveredDisposal` is a coarser bucket than "missing acquisition":

1. **Unresolved inbounds masquerade as shortfalls.** An `UnknownInbound` creates NO lot (hard blocker,
   "sats not yet in the ledger", `fold.rs:945-949`); an `Unclassified` likewise folds nothing. The
   coins those events represent therefore surface as an `UncoveredDisposal` shortfall on the LATER
   disposal that drew them. The correct remedy is **classify the inbound** — and if the filer instead
   declares a tranche and *later* classifies the inbound, the pool holds tranche + real lot for the
   same BTC: Σ-in inflates and the phantom lingers for future disposals — the exact C-1 double-count,
   re-entered through the blocker level. Nothing in the design prevents this today.
2. **Without-wallet variants are data errors, not coverable gaps.** `UncoveredDisposal` also fires as
   "dispose without wallet" (`fold.rs:687-696`), "pending out without wallet" (:814-824), "self
   transfer without source wallet" (:859-869) — the fold returns early, consumes nothing, and there is
   no sat quantity. A tranche cannot fix these; the remedy is supplying the wallet. They must be
   excluded from the candidate set and routed as data fixes.
3. **Duplicate-imported sales / wrong-wallet attribution** (post-2025, sale recorded in wallet B while
   the coins sit in wallet A) likewise produce shortfalls whose remedy is void/reclassify/self-transfer,
   not a tranche.

**Required spec behavior:** the dashboard's "Needs a basis" section must (a) exclude the without-wallet
shapes from declare candidates; (b) when acquisition-shaped blockers (`UnknownBasisInbound`,
`Unclassified`, `ImportConflict`, `UnmatchedOutflows`) are open on the same pool/timeframe as a
shortfall, surface them FIRST with "resolve these — the shortfall may disappear" routing, before
offering declare; (c) word the declare confirmation to assert the coins were acquired entirely outside
the vault's records. This is the direct answer to §11 Q2 — see below.

### N-2 (Important) — Coverage is emergent, not guaranteed: the spec must own the window/wallet prefill rules, a plan-time clearance check, and a "declared-but-didn't-cover" dashboard state

Evidence in the C-1 audit above. Concretely the spec must own: (a) candidate prefill `window_end`
strictly **before** the earliest short disposal (decisions sort after same-instant imports,
`resolve.rs:1308-1312`) and `wallet` = the disposal's wallet (post-2025 `Wallet(w)` pools; Path-A
re-homes by `lot.wallet`, `transition.rs:93-106`); (b) a plan-time clearance verification in the
declare chokepoint — append-the-candidate → re-project → assert the targeted shortfall cleared, the
`would_conflict` shadow-projection pattern (`project/mod.rs`), cheap at record-time frequency; (c) a
first-class dashboard state for a live tranche that did NOT clear its shortfall (both the shortfall row
and the tranche row would otherwise render disconnected, and a confused filer's natural move is to
declare AGAIN — a second phantom lot). Without (a)-(c), §4's "`[declare tranche]` (covers it)" is an
aspiration, not a property. All three are engine-mechanics-faithful spec work; none changes the
design's shape.

### N-3 (Minor) — The shortfall's structured data is not actually exposed: `Blocker` carries only a display string

The design's §2 consequence ("the find step reads a signal the engine already computes") oversells by
one step: the engine computes the shortfall but exposes it only as `Blocker { kind, event, detail:
String }` (`state.rs:113-117`) — "dispose short by N sat" is a *formatted* string. `journey_view`
needs `{event, wallet, date, short_sat}` per shortfall. The spec must add a small structured signal
(a `state` shortfall record, or recompute inside `journey_view`) and must NOT parse the detail string.
Derived state only — the "no new tax logic" claim survives.

### N-4 (Minor) — The plan/confirm/apply contract needs a staleness clause, and the parity KATs must pin the refusal paths

State must not mutate between `plan` and `apply` (TUI: structural via the one-flow invariant +
single-threaded event loop, with M-4's debug assertion; CLI: one call). The spec should state this
explicitly and keep `would_conflict` inside `apply` (as the shipped verb does, `promote.rs:477`).
Separately, the byte-identical-parity KATs must cover the refusal paths and advisory lines — including
the shipped "consent printed even when the ack fails" contract (`promote.rs:451-458`) — not just the
happy-path consent screen, or the extraction can silently regress a reviewed refusal behavior.

### N-5 (Minor) — Define the export year-set operationally, recomputed from state

"{current year} ∪ {advisory-flagged prior years}" should be pinned to the durable, derived signal —
the years in which a promoted disposal leg files (the `promote_export_gate(None)` enumeration,
`cmd/admin.rs:84-98`) — never a remembered promote-time advisory.

---

## Answers to §11's four open questions

**1. Does the chokepoint fully resolve reuse-vs-drift; is byte-identical consent parity the right
gate?** Yes and yes. The seam is feasible today (`tui-edit` → `cli` dependency exists; two shipped
precedents), the design's gate-ordering table matches the shipped verb exactly (verified
line-by-line), and artifact-equality is the strongest mechanically-checkable form of the guarantee
that actually matters (the recorded §6664(c) evidence ≡ what the filer saw). Sufficient, with N-4's
two sharpenings: parity must extend to refusal/advisory output, and the plan→apply staleness clause
must be written down.

**2. Does dispositions-only leave a correctness hole — can a shortfall be a data ERROR, and must the
dashboard distinguish?** Dispositions-only itself is sound (the cut manual-holdings path was a clean
narrowing). But yes — a shortfall is not always a missing acquisition, and the dashboard MUST
distinguish (N-1): unresolved inbound events (`UnknownInbound`/`Unclassified` fold NO lot, so their
sats surface as the later disposal's shortfall — remedy: classify, and declare-then-classify
double-counts), the without-wallet `UncoveredDisposal` variants (pure data errors, no sat quantity, a
tranche cannot fix them), duplicate-imported sales, and post-2025 wallet-attribution errors.
Resolve-inbounds-first ordering + without-wallet exclusion + honest confirmation copy is the required
spec shape.

**3. Is the derived-dashboard resume model fully sound?** Yes — with one trap left, already filed as
N-2c. The model is self-consistent by construction: candidates are shortfalls, which vanish when
covered (no re-proposal trap — the r1 self-triggering shape is structurally dead); tranche/promote
status derives from events; export is never "done". The single re-entry inconsistency is the
declared-but-didn't-cover state, where the dashboard would show both a shortfall row and an unrelated-
looking tranche row; with N-2's linked state surfaced, nothing needs persisting.

**4. Does the chokepoint refactor risk regressing a BG-D* guarantee?** It is the design's one genuine
new risk surface — it rewrites the reviewed sub-project-1 verb glue — and the design contains it
correctly: P-A extracts with consent-parity KATs *gating* the phase, the ordering contract is written
down once (and matches the shipped code exactly, so the extraction target is unambiguous), and the
existing CLI integration tests (declare/promote/export suites) remain the behavioral net. With N-4's
refusal-path parity added, I see no BG-D1..D11 guarantee whose enforcement point the plan/confirm/apply
split would move or weaken — the gates all live in the extracted sequence itself or below it (engine).
Residual risk is ordinary refactor risk under KAT gate — acceptable.

---

*End r2. Verdict: SOUND to proceed to SPEC. All r1 findings resolved (C-1, C-2, I-1..I-7, M-1..M-5).
New: N-1, N-2 (Important — both must be owned in the SPEC's discovery/declare sections, P-B/P-C);
N-3..N-5 (Minor, spec-scope notes). No new Critical.*
