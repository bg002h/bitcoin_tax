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

Basis is a **disposition-time** concept. This feature acts ONLY on BTC that **left the filer's records
with no acquisition record** — the engine's `BlockerKind::UncoveredDisposal` *sat shortfalls* (a
lot-drawing op consumed more sat than the ledger holds). This family is wider than "sold": it includes
**self-transfer shorts** (the audience's MOST COMMON shape — withdraw to self-custody, then sell; the
shortfall lands on the *transfer*), fee shorts, and gift/donate shorts, as well as sale shorts. The
without-wallet and degenerate `UncoveredDisposal` variants (no sat quantity) are **data errors**, not
coverable gaps (see DFW-D4). **Unsold no-records holdings are OUT of scope** (no current-return effect;
engine-invisible; forward-planning only). There is **no manual "I hold N BTC" data model.**

---

## 3. Binding decisions (defend these against a review flip)

- **DFW-D1 (architecture — three seams).** (a) `btctax-core`: pure, KAT-able, mutation-proven
  computations only — a derived `journey_view`, the shortfall→candidate discovery, the era→window
  table; NO UI, NO session. (b) `btctax-cli`: **plan/confirm/apply chokepoints** — the sole home of the
  verb glue (gate ordering, private helpers, consent render). (c) `btctax-tui-edit`: the dashboard + the
  `*_flow` structs, thin drivers over (a)+(b); step availability **derived from the chokepoint's
  plan/guard results, never independently encoded** (no second gating authority — I-2).

- **DFW-D2 (the chokepoint contract — I-1; SPEC-r1 arch-I-3/m-2/m-3/n-1/n-2).** Each mutating verb
  (`declare_tranche`, `promote_tranche`, the crypto-slice + full-return export) is extracted into a
  session/state-parameterized trio in `btctax-cli`, with the CLI verb and the dashboard as thin drivers:
  - `plan(state, events, …, target_shortfall: Option<EventId>) -> Plan | Refusal` (DFW-D5.2),
  - `render_consent(&Plan) -> String` (deterministic — clock-free `consent_terms`, ordered
    `Vec<ConsentTerm>` over a `BTreeSet<i32>`, fixed-point `Decimal` rendering),
  - `apply(&mut Session, Plan, acknowledge: Option<&str>) -> …`.
  The **gate ordering** is written ONCE and MUST match the shipped promote pipeline exactly: resolve-live
  → BG-D5 provenance → BG-D7 Part II → BG-D3 floor/coverage → BG-D6 `consent_terms` → synthetic-promote
  advisory → gift-only relabel → consent render (incl. `wide_window_note`) → **ack** → `would_conflict` →
  append.
  - **★ BG-D6 ack residency (arch-I-3):** `require_promote_ack` is enforced **INSIDE `apply`**, fail-
    closed (`None` refuses; the shipped distinct `None`-vs-`Some(wrong)` refusals preserved), BEFORE
    `would_conflict` → append. Drivers only **collect** the phrase — they NEVER validate it — so BG-D6's
    enforcement point stays single-sourced in the chokepoint, not scattered across N drivers.
    `would_conflict` likewise stays inside `apply`.
  - **★ Acceptance (m-3, precise):** the **rendered consent copy + advisory/refusal output** are
    **byte-identical** between the CLI and the dashboard for the same tranche/state; the recorded
    `Acknowledgment.shown_terms` (`Vec<ConsentTerm>`) is equal by **structural `Eq`** (not "bytes"). The
    §6664(c) artifact must equal what the filer saw, on either surface.
  - **★ Parity KATs (N-4 + n-1 altitude):** drive **both FULL driver paths** (the CLI verb fn AND the
    TUI flow's persist path) and compare the *recorded* artifacts + captured output — comparing two
    calls to the single-sourced renderer is a tautology; the mutation that must red is a **driver**
    post-processing / re-wrapping / bypassing the chokepoint. Cover the REFUSAL paths and advisory lines,
    incl. the shipped "consent printed BEFORE the ack gate, so a refused ack still surfaces the figures"
    contract (`promote.rs:451-458`).
  - **Export's trio is degenerate (n-2):** `plan` = the shipped gates over `&Session`/state, `apply` =
    write files, NO consent/ack/`Plan`-confirm — do not manufacture a consent step for symmetry.
  - **Plan→apply staleness (N-4):** state MUST NOT mutate between `plan` and `apply` (TUI: the one-flow
    invariant + single-threaded event loop, with a debug assertion — M-4; CLI: one call). `would_conflict`
    in `apply` is the backstop.

- **DFW-D3 (dashboard = fork, not progress — I-3; SPEC-r1 tax-M-4/N-2).** A derived, read-only journey
  view whose ROWS launch ordinary sibling flows — NOT a modal linear super-flow. It IS the resume
  mechanism (fully derived from state; nothing persisted). The `$0`/promote choice is rendered as **two
  equal branches**; a `$0`-declared tranche is NEVER incomplete; promote is never a default (G-1); export
  is never a checkable "done" (exports write files, not events — M-5). **Revocability copy carve
  (tax-M-4):** a `DeclareTranche` with a live promote is engine-adjudicated `DecisionConflict` and is NOT
  voidable — the declare-row's "revocable" copy must carry that carve (revocable until promoted), not an
  unconditional claim. **Advisory rows (tax-N-2):** the dashboard SHOULD surface the shipped, state-
  derived `method_inversion_advisory` / `tranche_dip_advisory` on tranche rows — under an elected FIFO a
  tranche's `$0`/floor basis lands on EARLIER disposals than the covered shortfall implies (coverage is
  method-invariant; basis *allocation* is not).

- **DFW-D4 (triage — cover-with-a-tranche vs fix-the-data — N-1; SPEC-r1 tax-I-3/arch-I-1).** A
  shortfall is NOT always a missing acquisition. The triage MUST be **total by construction** — the
  classifier keys on the **structural presence of `short_sat`** (DFW-D7's signal, emitted at exactly the
  sat-carrying fold sites), NEVER on a `Blocker.detail` string or a hand-maintained family list:
  1. **Coverable (declare candidate) = every sat-carrying shortfall** — the missing acquisition is
     missing regardless of which op consumed it: `dispose`/`gift-out`/`donate`/**`self-transfer`**/`fee`
     shorts. EXCEPTION: a **`pending-out` short** is routed through its co-emitted `UnmatchedOutflows`
     triage FIRST (a later `TransferLink` may re-shape it) — resolve-data-first (below).
  2. **Data-fix (NOT declarable) = every shape with NO `short_sat`** — all five without-wallet variants
     (dispose / pending-out / self-transfer / **gift-out / donate** without wallet) AND the four
     degenerate fee-carry guards. No enumeration to drift: absence of `short_sat` routes it to a
     data-fix by construction.
  3. **Resolve-data-first ordering:** when an **acquisition-shaped blocker** (`UnknownBasisInbound`/
     unknown-basis, `Unclassified`, `ImportConflict`, `UnmatchedOutflows`) is open on the **same pool
     and timeframe** as a shortfall — same-pool = `pool_key(date, wallet)` equivalence (pre-2025
     `Universal`, post-2025 `Wallet(w)`); timeframe = blocker-event date ≤ short-op date — surface it
     FIRST with "resolve these — the shortfall may disappear" routing (to the shipped set-fmv / classify
     / reconcile flows), BEFORE offering declare. Prevents declare-then-classify from re-minting the C-1
     double-count at the blocker level.
  4. Word the declare confirmation to assert the coins were **acquired entirely outside the vault's
     records** (not merely "unpriced").
  KATs (§5): a self-transfer-short yields exactly one candidate of `short_sat`; a gift-out-without-wallet
  and a donate-without-wallet each yield ZERO candidates (routed as data-fixes); a shortfall behind an
  open `Unclassified` surfaces the classify remedy first and offers no declare candidate for it.

- **DFW-D5 (coverage is emergent — the declare candidate must provably cover — N-2; SPEC-r1
  tax-I-4/I-5, arch-I-2/I-4).** Declaring a tranche does not guarantee it covers the shortfall. The
  declare chokepoint MUST:
  1. **Per-class prefill:** `window_end` strictly **before the short op's date** (decisions sort AFTER
     same-instant imports — `resolve.rs:~1312`), and `wallet` = **the short op's source-pool wallet**
     (post-2025 `Wallet(w)`; Path-A re-homes each residual to `lot.wallet` — `transition.rs:~104`). For
     a **self-transfer / pending-out short** the anchor is the *transfer* date and the *source* wallet,
     NOT a disposal.
  2. **Target-parameterized clearance (arch-I-2/tax-I-5):** the shared `plan` takes
     `target_shortfall: Option<EventId>`. The dashboard candidate path always passes `Some(short op)` and
     gets a **plan-time clearance check** — append-the-candidate → re-project (forcing
     `pseudo_reconcile = false`, mirroring `would_conflict`) → assert the targeted `UncoveredDisposal`
     cleared; a candidate that would NOT clear is a **refusal with a reason**, not a silent append. The
     **CLI free-form declare passes `None`** and keeps the shipped verb's gate set byte-for-byte
     (validation + `guard_tranche_vs_allocation` only) — so DFW-D8 / §5 behavior-preservation and the
     single-source-of-gating (DFW-D1) BOTH hold. Clearance is thus dashboard-scoped without being a
     second gating authority.
  3. **Two derived coverage-mismatch states** (no per-tranche persisted target — DFW-D3):
     - **Didn't-cover (arch-I-4):** a shortfall row enters a combined **pool-level** "still short"
       state iff a live `DeclareTranche` exists whose pool (`pool_key(window_end, wallet)`) matches the
       shortfall's pool AND `window_end` ≤ the short op's date. Render as ONE pool state ("a tranche of
       N sat is live here but this op is still short by S — review the window/wallet; do NOT declare
       again"), never a per-tranche attribution.
     - **Redundant / over-covered (tax-I-4 — the BG-1-critical direction):** a live `EstimatedConservative`
       tranche whose removal leaves NO `UncoveredDisposal` that it was clearing is a **phantom** (a later
       import/classify supplied the real coins). Surface "this tranche no longer covers anything real —
       void it" routing; AND the **promote chokepoint** carries a refusal-grade check — a target tranche
       that currently covers no shortfall is refused, because a promoted phantom's `>$0` per-sat basis
       exits `hifo_cmp`'s sort-last case and is drawn FIRST → **understated gain on double-counted
       coins** (the direction BG-1 forbids). Same shadow-projection machinery; derived; no new tax logic.

- **DFW-D6 (pseudo gate — C-2; SPEC-r1 tax-I-2).** The whole journey is gated on
  **`!state.pseudo_active()`** with routing guidance ("resolve/approve pseudo defaults first"); a
  defensive-filing journey over synthetic estimates is incoherent. ★ **Correction:** pseudo-reconcile is
  NOT shortfall-stable — Phase B synthesizes a real `SelfTransferMine{basis:None}` lot for every
  unresolved `TransferIn` (`resolve.rs:~1156`), whose sats CAN clear a `dispose short` (likewise an
  accept-first `ImportConflict`). So EVERY chokepoint shadow projection — the **discovery** signal
  (DFW-D7), the **DFW-D5 clearance** re-projection, AND the **consent/savings** computation — MUST force
  `pseudo_reconcile = false` (exactly as `would_conflict` does, `project/mod.rs:~119`); the chokepoint
  must not depend on its caller's journey gate (the CLI drivers have none). The **latent sub-project-1
  gap** (`cmd/promote.rs:396` folds the stored `pseudo_reconcile` into `consent_terms` /
  `promote_prior_year_advisory` / `gift_only_flagged_years`, immortalizing synthetic numbers in the
  recorded §6664(c) `Acknowledgment` — confirmed real in source) is fixed by the SAME chokepoint pseudo-
  off, so both surfaces inherit it — **filed against sub-project 1 regardless of this feature's
  scheduling** (see §8). With pseudo forced off, pseudo-papered years surface Hard-blocked →
  `TaxYearNotComputable` → the BG-D6 three-flavor discipline records the honest gain-Δ / named-
  unquantified consent artifact.

- **DFW-D7 (structured shortfall signal — N-3).** `journey_view` MUST consume a **structured** shortfall
  record `{event, wallet, date, short_sat}` (a small derived `state` signal or a recompute inside
  `journey_view`) — it MUST NOT parse `Blocker.detail`'s display string. Derived state only; no new tax
  logic.

- **DFW-D8 (declare guardrail — I-6; SPEC-r1 tax-N-1).** Declaring is **`$0`, revocable (until promoted
  — DFW-D3 carve), NO Form 8275** — a plain confirmation matching the shipped verb (input validation +
  the allocation guard only). The **CLI free-form declare passes `target_shortfall = None`** to the
  DFW-D5.2 plan, so it keeps the shipped gate set byte-for-byte (no clearance refusal); the dashboard's
  candidate declare passes `Some`. The typed-phrase tier is reserved for PROMOTE (mirrors
  `PROMOTE_ACK_PHRASE`). The no-records assertion is worded as an ordinary confirmation and **NOT
  recorded** (recording it would be a schema change / new tax logic). If the dashboard lets the filer
  edit `sat` **above** the prefilled `short_sat`, the excess is the out-of-scope manual-holdings shape
  (§2) entering by a side door — it files nothing wrong at `$0`; a confirm-note suffices (do not build
  the holdings model).

- **DFW-D9 (era presets + safe-harbor — I-7, M-3; SPEC-r1 tax-M-3, arch-m-1).** Presets are confirm/edit
  **starting points**, not authoritative windows. The **preset-confirm copy MUST frame the window as the
  filer's OWN knowledge of when they acquired the coins** — the attested window is the *substance* of the
  BG-D5 attestation and the Cohan/§6664(c) footing, and must never read as tool-sourced. **DFW-D5's
  before-the-short-op prefill governs over a preset's `window_end` where they conflict** (an era end
  after the short op would not cover — and `window_end` IS the lot's holding-period start,
  `resolve.rs:~1310`, so it also sets short/long-term). A **mandatory live readout** shows the resulting
  window-min floor + coverage + holding-date + (on-demand, per DFW-D10) clamped saving as the filer
  edits — wider window → lower floor, the conservative direction, made visible. The can-never-promote
  coverage states are surfaced live: `Coverage::Partial` and the `filed_basis_for`
  `NoCoverage`/`PartialCoverage` **refusal outcomes** (the enum is `{Full, Partial}` — there is no
  `None` variant; `conservative_promote.rs:56-69`). The **safe-harbor exclusion**
  (`guard_tranche_vs_allocation`: a pre-2025 `DeclareTranche` is refused beside an in-force
  `SafeHarborAllocation`) is a **first-class dashboard state at entry**, not a final-Enter surprise. The
  preset table gets copy-level review rigor.

- **DFW-D10 (Assess figures — I-5; SPEC-r1 tax-M-1/M-2).** Uses the shipped **clamped** promote saving
  (never the unclamped `overpayment_delta` reconstruction what-if — the sub-1 tax-r1 I-3 hazard), in the
  BG-D6 **three-flavor** discipline: **computed-tax-Δ only where BOTH folds compute the year** — the
  bundled table ships (exactly 2017/2024/2025/2026) AND a stored `TaxProfile` exists AND no Hard blocker
  — else **gain-Δ + an uncomputable flag** (2018–2023 are uncomputable-forever and ARE the audience
  years; also the no-profile / Hard-blocked doors), else named-but-unquantified. A bare `$X (year Y)` is
  NEVER shown for a non-computing year. Figures are **computed once per entry/state-change and cached**
  (each `clamped_promote_year_saving` = two full projections; the TUI draws ~10 Hz). The Declare live
  readout is limited to the cheap trio (floor/coverage/holding-date) — **DFW-D10 governs over DFW-D9's
  "clamped saving as the filer edits":** the saving is recomputed **on demand, never per keystroke, and
  is invalidated (blanked / "stale — recompute") on any window edit** so a `$` computed for a previous
  window is never displayed against the current floor. Recorded consent figures always come from the
  DFW-D2 chokepoint plan at promote time (the staleness clause), never the dashboard cache.

- **DFW-D11 (Forms/export — I-4, N-5; SPEC-r1 tax-I-1, arch-m-2).** Export is driven through the
  chokepoint (parameterized over `&Session`/state), NOT a second `Session::open`. ★ The export set is
  **{current year} ∪ {the BG-D9 fold-diff–flagged prior years across all live promotes, over disposal
  AND removal legs}** — enumerated via the `promote_prior_year_advisory` fold-pair machinery, recomputed
  from state at export time (derived, never a remembered advisory). It is **strictly larger** than the
  `promote_export_gate(None)` disposal-leg set: a promote's HIFO reorder can change a prior year's
  **donation / gift** (Schedule A deduction, Form 8283) or re-order documented lots with **no promoted
  disposal leg in that year at all** — those 1040-X packets MUST be in the export set (else a filed
  prior year silently keeps a now-wrong deduction/8949). `promote_export_gate` is retained for its
  **8275-completeness** purpose only (disposal-legs-only is correct there — BG-D11 files no estimate on
  a removal). Its private year-enumeration is extracted to a shared `promoted_filing_years(state)` used
  by both the gate's `None` arm and any 8275-completeness caller (arch-m-2), so that enumeration is
  single-sourced — but it is NOT the export set. Refuse+route instead of prompting the pseudo attest
  phrase on the real path (standing DRAFT-gate policy).

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

- **DFW-D2 parity (full-driver altitude):** driving BOTH full driver paths (the CLI verb fn AND the TUI
  flow persist path) over the same tranche+state, the rendered consent copy + advisory/refusal output are
  byte-identical and the recorded `Acknowledgment.shown_terms` are structurally `Eq`; on the happy path,
  the refused-ack path (consent still surfaced), AND each refusal path (BG-D5/D3/D7). Mutation: a driver
  that post-processes/re-wraps/bypasses the chokepoint reds the KAT.
- **DFW-D2 ack residency:** `apply` refuses fail-closed on `acknowledge=None` and on `Some(wrong)`
  (distinct refusals); a driver cannot append without a correct phrase reaching `apply` (mutation:
  driver-side ack validation that then calls `apply(None)` still refuses).
- **DFW-D4 triage (total):** a **self-transfer-short** yields exactly one candidate of `short_sat`;
  **gift-out-without-wallet** and **donate-without-wallet** each yield ZERO candidates (data-fix route); a
  shortfall behind an open `Unclassified`/`UnknownInbound` on the same `pool_key`+timeframe surfaces the
  classify remedy FIRST and offers no declare candidate; a `pending-out` short routes through
  `UnmatchedOutflows` first. Classifier keys on `short_sat` presence (grep guard: no `Blocker.detail`
  parse).
- **DFW-D5 coverage + over-coverage:** a dashboard candidate (`Some` target) whose prefill would NOT
  clear is refused with a reason (mutation: prefill `window_end == short-op date` → reds); the CLI
  free-form declare (`None`) is NOT refused (shipped semantics preserved); a cleared tranche removes the
  shortfall row; a live tranche whose pool matches an unresolved short renders the pool-level "still
  short — don't declare again" state; a declare→later-classify vault renders the **redundant/void-me**
  state AND the promote plan for that phantom tranche is **refused** (covers no shortfall).
- **DFW-D6 pseudo (all shadows):** with pseudo active the journey refuses+routes; and at the chokepoint
  the **discovery**, **clearance**, AND **consent/savings** projections all force `pseudo_reconcile=false`
  — a `SelfTransferMine{$0}`-cleared shortfall is NOT hidden and no pseudo number reaches a recorded
  `Acknowledgment` (the sub-project-1 latent-gap KAT).
- **DFW-D10 flavors:** no bare `$X` for a non-computing year — a no-`TaxProfile` year and a 2018–2023 year
  each render the gain-Δ+uncomputable / named-unquantified flavor, not a dollar.
- **DFW-D11 export set:** a **donation-reordered prior year with no promoted disposal leg** is in the
  export set (mutation: use the `promote_export_gate(None)` disposal-leg set → that year is dropped →
  reds).
- **DFW-D3 fork:** a `$0`-declared tranche renders complete (never "incomplete/step N of M"); promote is
  an explicit optional branch; export is always-available, never "done".
- **DFW-D7:** `journey_view` reads the structured `{event,wallet,date,short_sat}` (no `Blocker.detail`
  string parse — grep guard).
- Plus the shipped BG-D1..D11 KATs remain green (the chokepoint extraction is behavior-preserving).

## 6. Design provenance

Brainstormed 2026-07-22 (dialogue: surface=TUI-edit; scope→dispositions-only; discovery; window). Fable-
architect **r1** critique → NEEDS REWORK (2C/7I). Adjudicated (`DESIGN.md`) + owner scope decision
(dispositions-only). Fable-architect **r2** → SOUND to proceed; +N-1/N-2 (Important) +N-3/N-4/N-5. This
SPEC then passed a two-lens **SPEC review r1** (Fable): tax 0C/5I, arch 0C/4I — all seam-level, no
design reshape; **folded here** (triage total-by-`short_sat` DFW-D4; target-parameterized clearance +
over-coverage state DFW-D5; all-shadows pseudo-off DFW-D6; ack-inside-`apply` + full-driver parity
DFW-D2; fold-diff export set DFW-D11; both-folds-compute flavors DFW-D10; Coverage/preset/revocability
copy DFW-D9/D3/D8). Re-review = **SPEC r2 on OPUS** (user-directed model switch). Reviews verbatim in
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
