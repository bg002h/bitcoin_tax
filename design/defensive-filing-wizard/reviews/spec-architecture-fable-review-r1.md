# Architecture review — Defensive Filing SPEC (r1, Fable lens)

**Artifact reviewed:** `design/defensive-filing-wizard/SPEC.md` (binding decisions DFW-D1..D12),
commit c84537f on `feat/defensive-filing-wizard`.
**Stance:** independent SPEC-level architecture review, re-derived from current source — every
load-bearing claim below was re-checked against the real tree, not carried forward from my r1/r2
brainstorm critiques. Citations are to the current tree.

---

## Verdict

**NOT GREEN** — **0 Critical / 4 Important / 3 Minor / 2 Nit**.

The three-seam architecture is sound and implementable, the DFW-D2 gate-ordering contract matches the
shipped promote pipeline **exactly** (re-derived line-by-line, `cmd/promote.rs:374-488`), every r1/r2
finding is faithfully encoded in a binding decision (full audit below), and byte-identical consent
parity is achievable (no rendering nondeterminism found — see Q1). But formalization exposed four
spec-level defects the brainstorm reviews could not have seen: the SPEC's own object definition (§2
"the engine's `UncoveredDisposal` *sat shortfalls*") is **wider than its prose** ("sold or given
away") and DFW-D4's triage taxonomy is not total over the real emitter set (I-1); DFW-D5.2's
mandatory clearance check **contradicts** DFW-D8's and §5's behavior-preserving pin on the shipped
declare verb (I-2); the BG-D6 ack gate's residency in the plan/confirm/apply trio is unspecified,
which is exactly the seam where the extraction could silently move a shipped guarantee's enforcement
point (I-3); and DFW-D5.3's "declared-but-didn't-cover" state references a tranche↔shortfall linkage
that is **not derivable** under the SPEC's own no-persistence constraints without a predicate the
SPEC never defines (I-4). All four are resolvable without changing the architecture's shape — hence
no Critical — but none is resolvable by a plan-writer without making a binding decision that belongs
in this SPEC.

---

## r1/r2 encoding audit (baseline check — all encoded)

Verified each adjudicated resolution against the SPEC's decisions: r1 C-1 → §2 dispositions-only +
DFW-D4; C-2 → DFW-D6 + §8 sub-project-1 filing + P-A ownership; I-1 → DFW-D2; I-2 → DFW-D1; I-3 →
DFW-D3; I-4 → DFW-D11; I-5 → DFW-D10; I-6 → DFW-D8; I-7 + M-3 → DFW-D9; M-1/M-2 → DFW-D12; M-4 →
DFW-D2 staleness clause; M-5 → DFW-D3. r2 N-1 → DFW-D4; N-2 → DFW-D5; N-3 → DFW-D7; N-4 → DFW-D2's
refusal-path-parity + staleness clauses; N-5 → DFW-D11's recomputed year-set. **Nothing was dropped
or diluted.** The findings below are new, surfaced by formalization — not survivals.

Key code re-verifications that HOLD: the DFW-D2 ordering ≡ shipped pipeline (resolve-live :378 →
BG-D5 :381 → BG-D7 :386 → BG-D3 :397 → BG-D6 :410 → advisory :433-445 → gift-only :449 → consent +
`wide_window_note` :453-456 → ack :458 → `would_conflict` :477-483 → append :485); consent printed
before the ack gate (:451-458); `Acknowledgment.shown_terms: Vec<ConsentTerm>` (`event.rs:359-368`);
`render_consent(&[ConsentTerm], &BTreeSet<i32>)` deterministic (`promote.rs:333-341`);
`consent_terms` is clock-free ("as_of = the ledger's latest recorded event date",
`conservative_promote.rs:250-258`); `would_conflict` forces `pseudo_reconcile = false`
(`project/mod.rs:118-119`); pseudo-reconcile does NOT clear `UncoveredDisposal`
(`tests/pseudo_reconcile.rs:343`); the decision sort (`src_priority: u8::MAX`, `resolve.rs:~1312`)
and the Path-A `lot.wallet` re-home (`transition.rs:91-106`) support DFW-D5.1's prefill rules
exactly; `btctax-tui-edit` depends on `btctax-cli` (`Cargo.toml:19`) and imports `btctax_cli::Session`;
the one-flow invariant exists as comment-law (`editor.rs:116,136` — the M-4 debug assertion is the
right hardening); `TaxTables` is a **core** trait (`tax/tables.rs:113`) so `journey_view` taking
prices + tables fits core with no dependency inversion; bundled tables are exactly
2017/2024/2025/2026 (`adapters/src/tax_tables.rs:75,158`); `clamped_promote_year_saving` is two full
projections (`conservative_promote.rs:487-507`); the TUI already calls
`promote_export_gate(&state, &events, Some(year))` refuse-before-bytes (`btctax-tui/src/export.rs:169`).

---

## Important

### I-1 (DFW-D4 + §2) — The triage taxonomy is not total over the real `UncoveredDisposal` emitter set, and §2's prose contradicts §2's definition

`fold.rs` has **15** `UncoveredDisposal` emitters, in four families:

- **Sat-carrying shortfalls** (`short by {N} sat`): dispose (:710), gift out (:1196), donate (:1274),
  **self transfer (:876)**, **pending out (:831)**, and the **fee helper** ("self-transfer/gift fee
  short by {N} sat", :388, reached from multiple call sites).
- **Without-wallet variants** (no sat quantity): dispose (:691), pending out (:819), self transfer
  (:864), **gift out (:1177)**, **donate (:1255)** — five, not three.
- **Degenerate m3 fee-carry guards** (no sat quantity, "unreachable for real events"): :742, :935,
  :1225, :1303.

Three concrete failures:

1. **§2 equates "sold or given away" with "the engine's `UncoveredDisposal` sat shortfalls" — but the
   sat-shortfall family also contains self-transfer shorts, pending-out shorts, and fee shorts**,
   none of which is a sale or gift. A plan-writer implementing §2's prose excludes them (leaving Hard
   sat shortfalls with NO dashboard route — the journey's own signal family half-covered); one
   implementing §2's definition includes them (contradicting the prose and the DFW-D4.3 confirmation
   copy's framing). The SPEC must decide, not the plan.
2. **DFW-D4.1 enumerates "dispose/pending-out/self-transfer without wallet" — 3 of the 5
   without-wallet variants.** "gift out without wallet" and "donate without wallet" match the
   clause's *principle* but not its *enumeration*, and §5's D4 KAT would be written from the
   enumeration. The four degenerate guards are unclassified entirely.
3. **DFW-D4.2's "same pool/timeframe" is undefined** — and the definition is not free: pre-2025 the
   pool is `PoolKey::Universal` (every wallet), post-2025 `Wallet(w)`; a plan-writer must know the
   scoping is pool-key semantics, not "same wallet" prose.

**Fix.** Make DFW-D4 total by construction: (a) the candidate/exclusion classifier keys on the
**structural presence of `short_sat`** (exactly the sites DFW-D7's structured signal is emitted
from), never on a detail-string family list — without-wallet AND degenerate shapes then route as
data-fixes automatically; (b) decide and record the policy for the three non-disposition sat-shorts —
my recommendation: self-transfer and fee shorts ARE declare candidates (the missing acquisition is
missing regardless of which op consumed it; covering restores Σ-conservation and downstream basis),
while **pending-out shorts route through the `UnmatchedOutflows` triage first** (the same event
already carries that blocker, :857-862 — a later `TransferLink` may re-shape it), consistent with
DFW-D4.2's resolve-data-first ordering; (c) reword §2's prose to match ("BTC that left the filer's
records with no acquisition record" or similar); (d) define same-pool via `pool_key(date, wallet)`
equivalence and timeframe as blocker-event date ≤ short-disposal date; (e) extend the §5 D4 KAT to
one of each excluded family (incl. gift/donate-without-wallet).

### I-2 (DFW-D5.2 vs DFW-D8 vs §5) — The mandatory clearance check contradicts the behavior-preserving pin on the shipped declare verb

DFW-D5.2: "The declare chokepoint MUST … perform a plan-time clearance check … A candidate that would
NOT clear is a refusal." DFW-D8: declare is "a plain confirmation **matching the shipped verb**
(which gates declare on input validation + the allocation guard only)" — verified: `declare_tranche`
gates on `sat > 0`, window order, and `guard_tranche_vs_allocation` only (`cmd/tranche.rs:134-154`).
§5: "the shipped BG-D1..D11 KATs remain green (the chokepoint extraction is behavior-preserving)."
DFW-D2 makes BOTH the CLI verb and the dashboard thin drivers over the SAME trio.

These cannot all hold as written. If the clearance check lives unconditionally in the shared declare
`plan`, the CLI verb inherits a NEW refusal — a free-form declare that targets no current shortfall
(a legitimate shipped use: declaring ahead of an import, deliberate over-declaring) is refused, a
behavior change to a reviewed verb. If instead the check is dashboard-only glue, the TUI driver
independently encodes a gate — the second gating authority DFW-D1 explicitly forbids.

**Fix.** Parameterize the target: `plan(state, events, …, target_shortfall: Option<EventId>)`. The
dashboard's candidate path always passes `Some(disposal_event)` and gets the clearance assertion
(refusal on non-clearing); the CLI verb's free-form path passes `None` and keeps its shipped gate set
byte-for-byte (DFW-D8 and §5 stay true; the check stays single-sourced in the chokepoint, DFW-D1
stays true). State this in DFW-D5.2 and cross-reference from DFW-D8. Also state that the clearance
shadow-projection forces `pseudo_reconcile = false`, mirroring `would_conflict`
(`project/mod.rs:118-119`) — DFW-D6's journey gate makes this nearly unreachable, but the chokepoint
must not depend on its caller's gating.

### I-3 (DFW-D2) — The BG-D6 acknowledgment gate's residency in the trio is unspecified — the one place the extraction could silently move a shipped guarantee's enforcement point

The trio is pinned as `plan(…) -> Plan | Refusal` / `render_consent(&Plan) -> String` /
`apply(&mut Session, Plan) -> …`, and the ordering pins "… consent render → **ack** →
`would_conflict` → append" with only `would_conflict`'s residency made explicit ("stays inside
`apply`"). But `apply(&mut Session, Plan)` has **no acknowledgment channel**: the `Plan` is produced
before the filer types anything, so the phrase cannot be in it. Today `require_promote_ack`
(`promote.rs:346-357`) is enforced INSIDE the verb — BG-D6 is engine-side by construction. If the
extraction resolves the signature gap the obvious wrong way (each driver validates the phrase, then
calls `apply`), BG-D6's enforcement point moves from one chokepoint into N drivers, each individually
capable of skipping it — precisely the "moved/weakened guarantee" class this review exists to catch.
The SPEC's silence makes that outcome plan-writer's-choice.

**Fix.** One sentence in DFW-D2: `apply` takes the acknowledgment (e.g.
`apply(&mut Session, Plan, acknowledge: Option<&str>)`), enforces the single-sourced
`require_promote_ack` **inside** `apply`, fail-closed (`None` refuses, exactly the shipped
`None`-vs-`Some(wrong)` distinct-refusal contract), before `would_conflict` → append; drivers only
COLLECT the phrase. The consent-before-ack refusal contract (`promote.rs:451-458`) is then a driver
obligation ("render before collecting") — already covered by the refused-ack parity KAT. Note the
declare/export members of the trio have no ack: say so, so nobody manufactures one (declare's plain
confirm and export's gate-only shape are driver-tier concerns, per DFW-D8/D11).

### I-4 (DFW-D5.3) — "Declared-but-didn't-cover" requires a tranche↔shortfall association that nothing persists and the SPEC never defines

`DeclareTranche` records `{sat, wallet, window_start, window_end}` only — no target
(`cmd/tranche.rs:166-171`), and the SPEC itself forbids adding one (DFW-D8: recording the assertion
"would be a schema change / new tax logic"; DFW-D3: "nothing persisted"). So "a live tranche that did
not clear **its** shortfall" has no well-defined "its": after later mutations (a new import
re-ordering consumption, a void, a classify moving pools, a CLI free-form declare), the dashboard
sees N live tranches and M persisting shortfalls in a pool with no recorded pairing. The §5 KAT ("a
live-but-didn't-cover tranche renders the linked 'didn't cover' state") is unwritable until the
association predicate exists, and different plan-writer inventions yield materially different UX and
KATs. This is the one point where DFW-D3's fully-derived resume model is currently aspirational
rather than specified.

**Fix.** Pin the derived, pool-level predicate — no per-tranche attribution claimed: a shortfall row
enters the linked "didn't cover" state iff a live `DeclareTranche` exists whose pool
(`pool_key(window_end, wallet)`) matches the shortfall's pool AND whose `window_end` ≤ the short
disposal's date. Render it as one combined pool-level state ("a tranche of N sat is live in this
wallet but this disposal is still short by S — review the window/wallet; do NOT declare again")
rather than pretending to know which tranche was "for" which shortfall. This is fully derivable,
honest about what the event log knows, and kills the declare-again reflex the clause exists to
prevent.

---

## Minor

### m-1 (DFW-D9) — `Coverage::None` does not exist

The enum is `{ Full, Partial }` (`conservative.rs:217-221`); the no-data case is not a variant but a
refusal: `filed_basis_for` returns `Err(PromoteRefusal::NoCoverage)` on an uncovered window and
`Err(PartialCoverage)` on a gap (`conservative_promote.rs:56-69`). A plan-writer told to show
"`Coverage::Partial/None` … live" will hunt for a variant that isn't there. Reword: the live readout
surfaces `Coverage::Partial` and the `NoCoverage`/`PartialCoverage` refusal outcomes (both
can-never-promote states), naming the real types.

### m-2 (DFW-D11) — `promote_export_gate(None)` checks, it does not enumerate; single-source the year-set extraction

The {years with a promoted filed disposal leg} enumeration is a private local loop inside the gate
(`cmd/admin.rs:84-98`); the gate returns `Result<(), CliError>` — a caller cannot obtain the year set
"via the `promote_export_gate(None)` enumeration" as DFW-D11 reads. The plan-writer will extract or
copy it; a copy is a second encoding of "which years file a promoted leg" that can drift from the
gate's — the drift consequence is a silently missing 1040-X packet (the exact I-4b hazard DFW-D11
exists to close; the gate still refuses inadequate disclosure per exported year, so nothing smuggles
out — the failure is omission, not leakage). Pin it: extract a `pub fn promoted_filing_years(state)`
used by BOTH the BG-D8 gate's `None` arm and the DFW-D11 export set.

### m-3 (DFW-D2) — "Byte-identical `shown_terms`" is a category error on a structured value

`Acknowledgment.shown_terms` is `Vec<ConsentTerm>` (`event.rs:363`) — it has no canonical bytes at
the comparison site. The rendered consent copy is a `String` (byte-identity well-defined and
achievable: `render_consent` iterates the slice in order over a `BTreeSet` with `Decimal::round_dp`
formatting; `consent_terms` is clock-free; the advisory's `now` is a fixture input in the KAT).
Restate the acceptance precisely: rendered consent copy + advisory/refusal output byte-identical;
`shown_terms` equal by structural `Eq` (or canonical-serialization equality) between the two
surfaces' recorded artifacts.

---

## Nit

- **n-1 (§5, DFW-D2 parity KAT altitude).** With the renderer single-sourced, comparing two calls to
  the chokepoint is a tautology — both change together and the KAT stays green through a shared-copy
  bug. The KAT must drive both **full driver paths** (the CLI verb fn; the TUI flow's persist path)
  and compare the *recorded* artifacts + captured output, so the mutation it kills is the real hazard:
  a driver post-processing, re-wrapping, or bypassing the chokepoint (the stated "perturb one consent
  line in one driver" only exists as a mutant at that altitude).
- **n-2 (DFW-D2).** The trio is described once for three verbs with different shapes; export has no
  consent/ack and no `Plan`-worthy confirm. One clause noting export's trio is degenerate
  (plan = gates over `&Session`/state; apply = write files; no consent artifact) prevents a
  plan-writer from manufacturing an artificial consent step for symmetry.

---

## Lens answers (condensed)

**Q1 DFW-D2:** Implementable; ordering correct and complete vs the shipped verb (verified exactly);
byte-identity achievable — no rendering nondeterminism found (clock-free terms, ordered/BTreeSet
iteration, fixed-point rendering); refine per m-3, and close the ack-residency gap (I-3).
**Q2 DFW-D1:** Holds. No inversion: `TaxTables` is a core trait, prices are `&dyn PriceProvider`, so
`journey_view` fits core cleanly; cli→core and tui-edit→{cli,core} edges already exist; export-gate
status is dashboard-level composition over the cli gate, correctly outside core.
**Q3 DFW-D5:** The shadow-clearance pattern is sound in `plan` (proven precedent, record-time cost),
**but** its placement contradicts DFW-D8/§5 until target-parameterized (I-2), and didn't-cover
derivability needs the I-4 predicate.
**Q4 DFW-D7:** Right seam; `Blocker` carries only a display string (`state.rs:107-111`), so the
structured record is necessary; a derived state signal emitted at the sat-carrying fold sites (or a
`journey_view` recompute) stays derived-only, no schema-of-record change. Emit it at exactly the
`short_sat` sites and I-1's classifier comes for free.
**Q5:** The internal contradiction is I-2; the under-specifications are I-1/I-3/I-4. No BG-D1..D11
enforcement point moves under the SPEC as fixed: BG-D5/D7/D3/D6 live inside the extracted sequence,
BG-D8 stays in the untouched `promote_export_gate`, the allocation-guard chokepoint is reused not
rewritten — with I-3 resolved, ack included. P-A's parity-KAT gate does contain the refactor risk,
with n-1's altitude correction making the gate non-vacuous.
**Q6 (new):** I-1 through I-4 are the formalization-exposed defects; nothing structural beyond them.

---

*End r1 (SPEC). Verdict: **NOT GREEN — 0C/4I/3m/2n**. All four Importants are wording-level to fix and
shape-preserving; none reopens an adjudicated r1/r2 decision. Re-review required after fold.*
