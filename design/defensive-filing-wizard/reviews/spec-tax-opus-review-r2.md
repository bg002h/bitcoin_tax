# SPEC review — Defensive Filing wizard — US-federal-tax-correctness lens, round 2 (Opus)

**Artifact:** `design/defensive-filing-wizard/SPEC.md` @ commit `0502de4` (folded after the two-lens Fable
SPEC-r1). **Reviewer:** Opus, tax lens (independent; user-directed model switch from the r1 Fable pass).
All load-bearing citations re-verified against current source on `feat/conservative-filing` (`project/fold.rs`,
`project/resolve.rs`, `project/pools.rs`, `project/transition.rs`, `project/mod.rs`, `conservative.rs`,
`conservative_promote.rs`, `config.rs`, `cmd/promote.rs`, `cmd/tranche.rs`, `cmd/admin.rs`) — not trusted from
the SPEC. I did not anchor on r1's conclusions; each finding below is re-derived from the real code.

## Verdict

**NOT GREEN — 0 Critical / 1 Important / 0 Minor / 1 Nit**

All five r1 Importants are genuinely resolved in the fold, and every resolution is grounded in real source
(not the SPEC's self-citations). The single blocking finding is a **residual in the I-4 fix itself**: DFW-D5.3's
over-coverage refusal is binary ("covers a shortfall vs covers nothing"), so it closes the *fully*-redundant
phantom but leaves the *partially* over-sized tranche promotable — re-minting the exact BG-1 understated-gain
direction DFW-D5.3 claims to close. It is seam-level and fixable in SPEC text (make the check sat-aware).

---

## r1-resolution audit

- **I-1 (DFW-D11 export year-set) — RESOLVED.** Verified `promote_export_gate(None)` (`cmd/admin.rs:85-98`)
  still enumerates **disposal legs only** (`state.disposals`, `origin ∈ promoted_origins`). DFW-D11 no longer
  adopts it as the export set: the set is now `{current year} ∪ {BG-D9 fold-diff flagged prior years over
  disposal AND removal legs}`, recomputed at export via the `promote_prior_year_advisory` fold-pair machinery —
  which I verified **does** range over `st.disposals ∪ st.removals` and diffs disposal + `Donation` + `Gift`
  legs (`conservative.rs:721-758`). Because a promoted disposal leg's basis always differs between the with/
  without folds, the fold-diff set ⊇ the disposal-leg set, so DFW-D11's "strictly larger" is exact and **no
  disposal-leg year is dropped** by the switch; removal-flagged and reorder-only prior years are now captured.
  `promote_export_gate` is correctly retained for 8275-completeness only, with `promoted_filing_years(state)`
  extracted for single-sourcing. KAT added (donation-reordered prior year, no promoted disposal leg).

- **I-2 (DFW-D6 pseudo) — RESOLVED.** The false "pseudo does not clear `UncoveredDisposal`" premise is
  corrected: I confirmed pseudo Phase B synthesizes `SelfTransferMine{basis:None}` for every unresolved
  effective `TransferIn` (`resolve.rs:~1156-1181`), whose sats enter the pool and **can** clear a `dispose
  short`. DFW-D6 now forces `pseudo_reconcile = false` on **every** chokepoint shadow projection — discovery,
  DFW-D5 clearance, and consent/savings — mirroring `would_conflict` (`project/mod.rs:119`, verified). The
  sub-1 latent gap is confirmed REAL and load-bearing: `cmd/promote.rs:396` takes `session.config()?
  .to_projection()` (which carries the stored `pseudo_reconcile`, `config.rs:38-43`) and threads that `cfg`
  into `consent_terms` (:413), `promote_prior_year_advisory` (:436), and `gift_only_flagged_years` (:449) —
  so today, with pseudo stored on, synthetic figures fold into the recorded `Acknowledgment.shown_terms`
  (:468). The same chokepoint pseudo-off fixes both surfaces; correctly filed back to sub-1 (§8). Right
  consent artifact (pseudo-off → Hard-blocked → three-flavor honest term).

- **I-3 (DFW-D4 triage census) — RESOLVED.** The classifier is now **total by construction**, keyed on the
  structural presence of `short_sat` (the DFW-D7 signal), never on `Blocker.detail`. I verified the emitter
  census: sat shortfalls at `fold.rs:388/710/831/876/1196/1276` (fee / dispose / pending-out / self-transfer /
  gift-out / donate), without-wallet at `:691/819/864/1179/1257` + the degenerate fee-carry `:744`. DFW-D4's
  cover/fix split matches exactly (every sat-carrying shortfall coverable incl. **self-transfer and fee**;
  every no-`short_sat` shape a data-fix incl. **gift-out/donate-without-wallet**). Self-transfer prefill is
  per-class and tax-correct: source-pool wallet + before-the-transfer date, Path-A re-homing to `lot.wallet`
  with the `EstimatedConservative` tag preserved (`transition.rs:96-104`, verified). `pending-out` routed
  through `UnmatchedOutflows` first.

- **I-4 (DFW-D4/D5 over-coverage) — RESOLVED for the full phantom; PARTIAL (see I-A).** The hazard mechanism
  is real: `hifo_cmp` sorts `usd_basis == 0` **last** (`pools.rs:276-287`), so a promoted lot's `>$0` basis is
  drawn FIRST; and the shipped promote verb (`cmd/promote.rs:364-488`) has **no** live-shortfall check
  (gates: resolve-live → provenance → narrative → `filed_basis_for` → consent → ack → `would_conflict`). DFW-D5.3
  adds the derived redundant/void-me state + a promote-chokepoint refusal, which closes the *fully*-redundant
  phantom. The refusal predicate is binary, so the partial case survives — the single open Important below.

- **I-5 (DFW-D5 vs DFW-D8/§5 clearance scope) — RESOLVED.** The chokepoint `plan` is parameterized with
  `target_shortfall: Option<EventId>`; the dashboard candidate passes `Some` (plan-time clearance refusal), the
  CLI free-form declare passes `None` and keeps the shipped gate set byte-for-byte. Verified the shipped
  `declare_tranche` gates on `sat > 0`, `window_start ≤ window_end`, and `guard_tranche_vs_allocation` only
  (`cmd/tranche.rs:125-175`) — so the `None` path preserves those exactly, no clearance break, no second gating
  authority. Both branches KAT'd.

---

## Important

### I-A (DFW-D5.3) — The over-coverage refusal is binary, so a PARTIALLY over-sized tranche stays promotable and re-mints the BG-1 understated-gain direction

DFW-D5.3's promote-chokepoint check refuses "a target tranche that **currently covers no shortfall**" — a
binary predicate (removal re-materializes *some* shortfall vs *none*). Its own stated rationale, however, is
the sat-level harm: a `>$0` per-sat basis "exits `hifo_cmp`'s sort-last case and is drawn FIRST →
**understated gain on double-counted coins**." That harm is not all-or-nothing, and the binary predicate
under-scopes it.

**Failure scenario (reachable for the exact audience):** a `dispose short by 100M sat` in pool P. The filer
declares tranche T = 100M sat (DFW-D5.1 prefill = `short_sat`); the DFW-D5.2 clearance check passes honestly
(100M clears 100M). *Later*, the filer recovers a real acquisition and imports 0.6 BTC (60M sat) into the same
pool before the disposal (Mt.-Gox filer partially reconstructs records — precisely this feature's audience).
The true gap is now 40M, but T is still 100M. Removing T re-materializes a **40M** shortfall, so T "covers a
shortfall" → **DFW-D5.3's refusal does NOT fire**. Nothing forces a re-declare (DFW-D4.3 resolve-data-first
only orders blockers open *at declare time*; DFW-D5.2 is not re-run at promote). The filer promotes the 100M
tranche. With the floor's per-sat basis above the import's, `hifo_cmp` draws the **100M tranche sat first**;
the disposal reports `100M × floor` basis and the 60M documented import sat are displaced into the pool. The
disposal now files estimate basis on 60M sat that were really the filer's *documented* coins → **understated
gain on the over-covered 60M** (the direction BG-1 forbids), behind a provenance attestation for coins the
vault meanwhile accounts for twice. (In the reverse per-sat ordering the current disposal stays honest but a
60M **fictional estimate-basis'd residue lot** is stranded in the pool, understating gain whenever it is
later disposed.) DFW-D8's "editing `sat` above `short_sat` files nothing wrong at $0" note does not reach this
— the over-sizing is not filer-typed and it manifests at **promote (`>$0`)**, not at `$0`.

This is the same hazard class r1's I-4 caught (hence Important, not Critical), narrowed to partial coverage;
the fold's fix simply stopped at the binary case.

**Fix:** make the promote-time check **sat-aware**, reusing the same shadow-projection machinery: at the
promote chokepoint re-project without the target tranche, measure the sat of the shortfall that
re-materializes, and refuse (or route to void+re-declare, or clamp the promotable/coverable sat) when the
tranche's live sat **exceeds** the sat it actually covers. Extend the DFW-D5.3 redundant-state derivation the
same way (over-covered-by-N, not just covers-nothing). KAT: declare `short_sat = 100M` → later import supplies
60M in-pool before the disposal → the promote of the 100M tranche is refused/clamped as over-covering by 60M
(mutation: keep the binary "covers any shortfall" predicate → the 100M promote is admitted → reds).

---

## Nit

- **N-1 (DFW-D4.1 / DFW-D12).** A pure **fee-short** tranche is coverable and promotable, but promoting it is
  a tax no-op: `consume_fee` draws fee-sats acquisition-date **FIFO** (`pools.rs:62`, method-independent), the
  old covering tranche is drawn first, and BG-D4's fee-evaporation forfeits the estimate component — so the
  floor never reduces a gain and `consent_terms` quotes ~$0 realized saving. Harmless (conservative), but the
  dashboard's fork could suppress/annotate the promote branch on a tranche whose coverage is fee-only, so the
  filer is not offered a five-figure-looking promote that yields nothing. UX, not a filed-number defect.

---

## Verified sound (do not re-litigate)

- The headline flow files the right number (r1's list holds; re-confirmed `hifo_cmp` sort-last at
  `pools.rs:276-287`, `filed_basis_for` `Coverage::Full`-only refusal at `conservative_promote.rs:50-69`,
  `clamped_leg_basis` `net − documented` bound at `:179-192`).
- **DFW-D10** three-flavor discipline matches `consent_terms` (`conservative_promote.rs:362-395`):
  `ComputedTax` only when **both** folds price the year, else `Uncomputable{gain-Δ, deduction-Δ}`, else the
  all-zero date-swap is skipped; `CascadeNamed` for later carryover-linked years. Both-folds-compute wording
  is exact; clamped-only saving (`clamped_promote_year_saving:487-507`) forbids the unclamped what-if.
- **DFW-D9** window precedence is tax-correct: `window_end` IS the covering lot's effective/holding-period
  start (`resolve.rs:429` DeclareTranche admit → `acquired_at = window_end`, `usd_basis = $0`), so DFW-D5's
  before-the-short-op prefill governing a preset's `window_end` also sets the correct term; attestation-
  substance copy (filer's own knowledge) and the safe-harbor exclusion as a first-class entry state are sound.
- **DFW-D12** one-at-a-time is BG-D6-required (sequential promotes change each other's consent figures).
- **No-new-tax-logic holds after the fold:** every DFW-D adds gates/refusals/derived views; no filed number is
  minted outside the shipped primitives, and the DFW-D6 chokepoint pseudo-off is honestly a sub-1 defect repair.

*End r2. NOT GREEN: 0 Critical / 1 Important / 0 Minor / 1 Nit. All five r1 Importants resolved and
source-grounded; the one open Important is a partial-coverage residual left by the I-4 fix's binary predicate.*
