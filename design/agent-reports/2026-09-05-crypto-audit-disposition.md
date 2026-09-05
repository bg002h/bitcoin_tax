# Crypto-engine understatement audit — DISPOSITION half

**Date:** 2026-09-05 · **Branch:** `feat/schedule-1a-ty2025` · **Scope:** what leaves the pool and how the
gain is characterised (proceeds, lot selection/method, holding period, character, 8949 boxes/adjustments,
§1211/§1212, wash sales, gifts/donations/spends/self-transfers). Basis, acquisition and lot construction
belong to the other agent and were not audited.

**The one question:** what makes btctax report a gain that is too small, or characterise it in a way that
lowers tax?

---

## VERDICT

| Severity | Count |
|---|---|
| Critical | 1 |
| Important | 3 |
| Minor | 2 |
| Cleared (checked, sound) | 11 |

**Plain answer: YES — a reported capital gain can be materially understated with no refusal and no Hard
blocker, on the default path.** All four blocking findings share one mechanism: **the engine computes a
per-disposal identification-compliance verdict (`ComplianceStatus`) and a pool-conservation verdict, and
NEITHER ever changes the number that is filed.** `§1.1012-1(j)(3)`'s deemed-FIFO consequence is described
in three doc comments and implemented nowhere.

The repo's own KAT quantifies the size on a three-lot fixture: `crates/btctax-core/tests/method_election.rs:104
default_method_is_hifo` — a $95.00 sale reports basis `$90.00` (HIFO lot B) where FIFO gives `$50.00`
(lot A). **Gain $5.00 vs $45.00 — an 89% understatement of that disposition**, produced by the no-election
default.

---

## FINDINGS

### F1 — CRITICAL. A post-hoc `LotSelection` is APPLIED to the filed gain, ungated — while the optimizer doing the identical thing is attestation-gated.

**The filer.** Sells 1 BTC from self-custody on 2025-07-01. In October, before filing, runs
`btctax reconcile select-lots <disposal> --from <high-basis-lot>:100000000`. The reported basis becomes the
cherry-picked lot's, the gain drops, the packet exports, and the only trace is one line in `btctax verify`.

**The mechanism.** `resolve`'s `LotSelection` collection block validates four things and **not the
made-date**. Compare it against the `MethodElection` block twenty lines above, which *does* carry the guard:

`crates/btctax-core/src/project/resolve.rs:1361-1372` (elections — guarded):

```rust
        if let EventPayload::MethodElection(me) = &d.payload {
            let made = tax_date(d.utc_timestamp, d.original_tz);
            if !method_election_is_forward(me, made) {
                blockers.push(Blocker {
                    kind: BlockerKind::MethodElectionBackdated,
                    event: Some(d.id.clone()),
                    detail: "MethodElection effective_from precedes its made-date or TRANSITION_DATE (2025-01-01) — a standing order cannot be back-dated".into(),
                });
                continue;
            }
```

`crates/btctax-core/src/project/resolve.rs:1405-1417` (selections — **no date test at all**):

```rust
let EventPayload::LotSelection(ls) = &d.payload else {
    continue;
};
if !seen.insert(ls.disposal_event.clone()) { ... }
selections.insert(ls.disposal_event.clone(), ls.lots.clone());
```

and the retain that follows (`resolve.rs:1423-1445`) checks only targeting and `Σ picks.sat == principal`.
The applied selection then drives the reported basis — pinned by
`crates/btctax-core/tests/lot_selection.rs:153-154`:

```rust
assert!(!has(&st, BlockerKind::LotSelectionInvalid));
assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // picked A, not HIFO's B
```

Meanwhile the SAME projection classifies exactly this event as forbidden —
`crates/btctax-core/tests/compliance.rs:201-230`:

```rust
/// §1.1012-1(j) no post-hoc: a `LotSelection` whose made-date (2025-09-01) is AFTER the disposal
/// (2025-07-01) is NonCompliant.
...
assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
```

`ComplianceStatus::NonCompliant` has exactly two consumers outside tests: a display string
(`crates/btctax-cli/src/render.rs:239`) and the optimizer overlay. It never becomes a `Blocker`, so
`compute_tax_year`'s Hard-blocker gate (`crates/btctax-core/src/tax/compute.rs:462-467`) cannot see it, and
the full-return path never consults it either — `capital_net` reads `schedule_d(state, year)` straight off
`state.disposals` (`crates/btctax-core/src/tax/return_1040.rs:684`).

**Why this is the sharp one, not a design choice.** Sub-project C built the guard for this exact act and
put it on the optimizer only. `crates/btctax-core/src/optimize.rs:473-486`:

```rust
    if is_broker(wallet) && sale_date.year() >= 2027 {
        Persistability::ForbiddenBroker2027
    } else if selection_made <= sale_date {
        Persistability::ContemporaneousNow
    } else {
        Persistability::NeedsAttestation
    }
```

`optimize accept` refuses to persist an already-executed disposal's tax-minimizing pick without a narrow
attestation. `reconcile select-lots` performs the same write with no gate, no attestation, and a `--help`
line that says only *"§A.4 Specific-ID: pick the exact lots a disposal consumes"*
(`crates/btctax-cli/src/cli.rs:707`). The CLI handler appends unconditionally —
`crates/btctax-cli/src/cmd/reconcile.rs:1067-1088`; its own doc comment ends *"Identification must exist by
the time of sale (§1.1012-1(j))"*, which nothing enforces.

**Direction:** understates. The picked lot is chosen after the fact, so it is chosen to minimise gain.

**Smallest fix:** in `resolve.rs`'s `selections.retain(...)`, compare the decision's made-date against the
target disposal's date and, when `made > disposal_date`, either (a) drop the selection (falling back to
method order, exactly as every other rejection there already does) unless the C-gate attestation covers it,
or (b) raise a Hard blocker. Option (a) reuses the existing rejection path verbatim and needs no new
`BlockerKind`. Mirror the attestation lever the optimizer already has so the two write paths agree.

---

### F2 — IMPORTANT. A no-election disposal computes at HIFO; the resulting `NonCompliant` verdict never reaches `report --tax-year` or any export surface.

**The filer.** Imports a year of exchange trades, sets a tax profile, runs `report --tax-year 2025` and
`export-irs-pdf`. Records no `MethodElection` — because nothing asks for one on that path.

**The mechanism.** `crates/btctax-core/src/project/fold.rs:41-50`:

```rust
    if date < TRANSITION_DATE {
        ctx.config.pre2025_method
    } else {
        crate::project::resolve::resolve_election(date, wallet, ctx.elections)
            .map(|e| e.method)
            .unwrap_or(LotMethod::Hifo)
    }
```

and the pre-2025 twin, `crates/btctax-core/src/project/mod.rs:55` /
`crates/btctax-cli/src/config.rs:29`: `pre2025_method: LotMethod::Hifo`.

`disposal_compliance` gives this same disposal `NonCompliant`
(`crates/btctax-core/src/project/compliance.rs:176-179`, "No envelope hit, no applied selection, no in-force
election"), pinned by `crates/btctax-core/tests/compliance.rs:233-253`, whose own doc comment states the
correct answer:

```rust
/// Self-custody sell with no election and no selection: FIFO is the defensible fall-through but
/// the identification basis is absent → NonCompliant.
```

btctax's user-facing vocabulary says the same thing —
`crates/btctax-cli/src/render.rs:231`:

```rust
/// - `non_compliant`         — no adequate identification; FIFO is the defensible filing position.
```

The engine files the HIFO position instead.

**Where the disclosure is, and is not.** `disposal_compliance` is called from exactly one place in the CLI:
`crates/btctax-cli/src/render.rs:704`, inside `VerifyReport`. It is therefore visible only in
`btctax verify`. It is **not** in `report --tax-year`, not in `export-irs-pdf`, not in `export-snapshot`,
and not in `write_csv_exports`. The pre-2025 advisory `Pre2025MethodNote` exists
(`fold.rs:90-119`) but is gated on `date < TRANSITION_DATE`, so a post-2025 no-election disposal gets **no
blocker of any severity**.

**Owner-mandate boundary — read this before acting.** The HIFO default is user-mandated
(`design/SPEC_reconcile_defaults.md`, "Change 1", 2026-07-05; `FOLLOWUPS.md:2864`), and that spec is honest
about the direction: *"both REDUCE the estimate (HIFO minimizes gain; long-term lowers the rate)"*. **I am
not proposing to reverse it.** The finding is that the spec's own stated mitigation does not cover the path
it is invoked for. `design/SPEC_reconcile_defaults.md:41`:

> **[compliance]** HIFO needs specific-ID/records; the default stays `attested: false` (config already
> surfaces this) so the user is prompted to affirm it per exchange. Keep that surface.

`attested: false` is `pre2025_method_attested` — a pre-2025-only flag
(`crates/btctax-cli/src/config.rs`, threaded into `note_pre2025_once` at `fold.rs:113-116`). The
**post-2025** default at `fold.rs:49` has no attestation flag and no advisory. The cited mitigation is
attached to the wrong half of the change.

**Direction:** understates. HIFO maximises basis per disposition by construction.

**Smallest fix (no policy reversal):** emit a `Pre2025MethodNote`-shaped Advisory from
`applicable_method`'s `unwrap_or` arm — once per projection — naming HIFO, naming FIFO as the
§1.1012-1(j)(3) fall-through, and naming `config --set-forward-method hifo` as the compliant lever (the
text already exists verbatim at `crates/btctax-core/src/conservative.rs:80-88`). Then surface the
`compliance` rows on the `report --tax-year` and `export-irs-pdf` surfaces, not `verify` alone.

---

### F3 — IMPORTANT. The identification-timeliness test is DATE-granular where the regulation — and this repo's own spec — say "date **and time** of the sale".

**The filer.** Sells at 09:00 on 2025-07-01. Watches the price move. At 16:00 the same day records a
`LotSelection` naming the highest-basis lot. btctax stamps it `Contemporaneous` and, on the optimizer path,
`ContemporaneousNow` — *"persist freely"*, no attestation.

**The mechanism.** Both timeliness tests compare `TaxDate` (day granularity, `conventions.rs:9`), discarding
the `utc_timestamp` both sides actually carry:

`crates/btctax-core/src/project/compliance.rs:160-164`:

```rust
        if let Some(made) = sel_made.get(disposal) {
            if *made <= date {
                return ComplianceStatus::Contemporaneous;
            }
            return ComplianceStatus::NonCompliant;
        }
```

`crates/btctax-core/src/optimize.rs:483`: `} else if selection_made <= sale_date {`

The authority is unambiguous and is archived in-repo
(`legal/primary-sources/regulations-cfr/26CFR_1.1012-1_basis.xml`; quoted at
`reviews/R0-lot-optimization-program-round-1.md:32`): adequate identification is made *"no later than the
date and time of the sale, disposition, or transfer"*. So does the project's own spec review, which fixed
the canonical wording for this very test — `reviews/R0-lot-optimization-program-round-1.md:312`:

> **A.5 (line 118)** defines `Contemporaneous` = "a `LotSelection` whose **made-date is at/before the
> disposal's date-and-time of sale** (the canonical test; **not a filing-status proxy**)"

The implementation narrowed "date-and-time" to "date". `MethodElection` back-dating has the same
granularity but is harmless there (a standing order is a forward instrument and `effective_from` is a date
by construction); for a per-sale identification it is a live lever.

**Direction:** understates. It admits a same-day post-hoc cherry-pick as compliant, and on the optimizer
path it routes that pick around the `NeedsAttestation` gate entirely.

**Smallest fix:** thread the disposal event's `utc_timestamp` (already in `events`, already used to build
`wallet_of` at `compliance.rs:106-110`) and the decision's `utc_timestamp` into both comparisons; fall back
to the date comparison only when a timestamp is genuinely absent. `persistability` takes `sale_date`/
`selection_made` as parameters, so the change is confined to two call sites plus the two signatures.

---

### F4 — IMPORTANT. An unclassified outflow removes sats from the pool, reports no disposition, and is only an **Advisory** — and the export surface never mentions it.

**The filer.** An on-chain withdrawal that was actually a sale (OTC, peer-to-peer, a DEX) is imported and
never classified. `btctax export-irs-pdf` writes a Form 8949 and Schedule D that omit it entirely.

**The mechanism.** `crates/btctax-core/src/project/fold.rs:833-886` — the sats are consumed FIFO out of the
pool, a `PendingTransfer` is recorded, **no `Disposal` is pushed**, and:

```rust
            // Advisory blocker: unmatched outflow (may be resolved by a later TransferLink in Task 8+).
            st.add_blocker(
                BlockerKind::UnmatchedOutflows,
                Some(eff.id.clone()),
                "unmatched transfer out",
            );
```

`UnmatchedOutflows` is `Severity::Advisory` (`crates/btctax-core/src/state.rs:96-103`), so
`compute_tax_year` computes and `report --tax-year` prints a number.

**The asymmetry that makes it a finding.** A *partially* covered disposal is Hard (`UncoveredDisposal`,
`state.rs:84`) — the engine refuses the year when it can account for the disposition but not for all of its
sats. A *wholly unaccounted-for* outflow — strictly more missing information — is Advisory. The severity
runs backwards relative to how much is unknown.

**Where it is disclosed, and where it is not.** `render_report` prints
`"Pending: N BTC (n unreconciled transfers — see \`btctax verify\`)"`
(`crates/btctax-cli/src/render.rs:319-327`), and `verify` reports the count
(`render.rs:714`). But `IrsPdfReport` carries no pending field at all —
`grep -n "pending" crates/btctax-cli/src/cmd/admin.rs` returns nothing — and the export handler's only
completeness warning is keyed on `unresolved_hard`, which counts Hard blockers only
(`crates/btctax-cli/src/main.rs:976-982`). So the one surface that hands the filer a signable Form 8949
says nothing about the dispositions missing from it.

**Not Critical** because `report`/`verify` do disclose it and the export prints the standing
"NOT AUTHORISED FOR FILING / check every figure" notice.

**Direction:** understates — the omitted disposition's entire gain.

**Smallest fix:** add `pending_outflows: usize` (or `sigma_pending`) to `IrsPdfReport` at the two
construction sites (`admin.rs:835`, `admin.rs:1183`) and warn on it beside the `unresolved_hard` line, in
the same wording `render_report` already uses. Separately, consider whether `UnmatchedOutflows` should gate
`compute_tax_year` for a year whose return is being filed; that is a policy call, not a mechanical fix.

---

### F5 — MINOR. A crypto donation is hardcoded to the 50%-organization classes; the identical non-crypto gift is REFUSED.

`crates/btctax-core/src/tax/return_1040.rs:714-721`:

```rust
/// A includes crypto donations, unlike the derive-side non-crypto profile). Per §170(e): a **long-term**
/// donation leg deducts FMV → `CapGainProp30`; a **short-term** leg deducts §170(e) basis `min(FMV,
/// basis)` → `OrdinaryProp50`. Both are 50%-org classes, so `apply_170b`'s "50%-org only" precondition
/// holds by construction. The per-leg sums reconcile with `Removal.claimed_deduction`
```

Donee type is never asked (`ReclassifyOutflow.donee` is a free-form label, `event.rs:127-131`). A
long-term crypto gift to a **private foundation** should be reduced to basis under §170(e)(1)(B)(ii) —
crypto is not qualified appreciated stock — under a 20% ceiling; btctax deducts FMV under 30%. The matching
refusal exists but cannot fire here, and says so:
`crates/btctax-core/src/tax/return_refuse.rs:175-178`:

```rust
    /// A charitable gift/carryover to a **non-50%-organization** (Cash30/OrdinaryProp30/CapGainProp20 —
    /// private foundations etc.) needs the Pub. 526 "special 30% limit" ordering v1 doesn't implement;
    /// refuse rather than mis-limit and understate tax (review C1). Never produced by the crypto ledger.
    NonPublicCharityContribution,
```

**Why only Minor: the guard exists as a disclosure, and it names the exact risk.**
`crates/btctax-core/src/tax/advisories.rs:1197-1204` fires on *every* year with a donation, and
`advisories.rs:429-434` reads:

> "CHARITABLE DONEE ASSUMED — your {n} crypto donation(s) were valued assuming a PUBLIC CHARITY
> (50%-organization) donee: long-term gifts at fair market value under the 30%-of-AGI ceiling. If the donee
> is a PRIVATE FOUNDATION, the correct treatment is the 20% ceiling at BASIS (which v1 refuses). Verify who
> you gave to."

**Direction:** understates (deduction too large, ceiling too high). **Fix if wanted:** ask the donee class
once per donation and route a non-50%-org answer into the existing `NonPublicCharityContribution` refusal,
converting an advisory into the refusal its non-crypto twin already gets.

---

### F6 — MINOR. `optimize`'s principal-conservation precondition is `debug_assert!`-only, and `score_assignment` is `pub`.

`crates/btctax-core/src/optimize.rs:225-239` guards a documented silent-understatement path
(*"a NON-conserving assignment under-consumes silently → a falsely-low score"*) with:

```rust
    debug_assert!(
        assignment_conserves_principal(events, prices, config, assignment),
        "score_assignment: injected assignment violates Σpicks == principal (R0-M1)"
    );
```

In-tree callers all conserve by construction, and any *persisted* selection re-enters through `resolve`'s
Hard `Σ == principal` check, so nothing filed is reachable through this today. It is recorded because the
function is `pub` API on a published crate and the guard is compiled out of release builds. **Fix:** make it
a real `Result`/early-return, or drop `pub`.

---

### Direction note, for completeness (NOT a finding — overstates)

A partially covered disposal allocates the **full** net proceeds across only the covered fragments
(`fold.rs:128-146` with `total_sat = Σ consumed.sat`), so basis is proportionally short and the reported
gain is too large. Conservative, correctly paired with a Hard `UncoveredDisposal`, and out of scope.

---

## CLEARED

Checked against statute/reg/form and found sound. Each was a plausible understatement channel.

1. **Holding-period boundary, ordinary case.** `conventions.rs:80` —
   `is_long_term(a, d) = d > one_year_after(a)`. Pub. 544's rule (count from the day after acquisition;
   the disposal day counts) makes a sale on the anniversary exactly one year, hence short-term. Acquire
   2020-06-19: 2021-06-19 → ST, 2021-06-20 → LT. Pinned at `conventions.rs:198-205`. Correct, and correct
   in the *conservative* direction at the boundary.
2. **Holding-period boundary, leap-day acquisition.** `one_year_after` falls back Feb-29 → Feb-28
   (`conventions.rs:72-77`). I checked this specifically as a suspected one-day understatement and it is
   **right**: acquire 2020-02-29 → counting starts 2020-03-01 → one year completes at the end of
   2021-02-28 → a 2021-02-28 sale is exactly one year (ST) and 2021-03-01 is more than one year (LT).
   btctax returns exactly that. Pinned at `conventions.rs:207-211`.
3. **Single term-assignment site.** `term_for` (`fold.rs:78-84`) is the only production constructor of
   `Term`, used by both `make_disposal_legs` and `make_removal_legs`; `leg.acquired_at` is set from the
   *same* HP-start branch that selects `term_for`'s first argument, so the printed 8949 column (b) can
   never contradict the Part I/II placement.
4. **§1223(2) gift tacking.** `Lot::gain_hp_start()` = `donor_acquired_at.unwrap_or(acquired_at)`;
   `loss_hp_start()` = the gift date (`state.rs:135-141`). Gain side tacks, §1015(a) loss side does not —
   Pub. 551. Unknown donor date degrades to the gift date, i.e. toward short-term. Conservative.
5. **§1015(a) four-zone dual basis.** `fold.rs:166-224`. Gain zone (proceeds > donor basis) → donor basis,
   tacked; loss zone (proceeds < FMV-at-gift) → FMV basis, HP from the gift date; no-gain-no-loss →
   basis = proceeds, gain 0. Zone ordering cannot overlap because `loss_basis < gain_basis` for a true dual
   lot. Correct.
6. **Proceeds netting.** `fold.rs:734` — `round_cents(*proceeds - *fee_usd)` ("amount realized" less
   expenses of sale, Pub. 544). Pro-rata allocation across legs is remainder-takes-the-rest
   (`split_pro_rata`, `conventions.rs:50-65`), so Σproceeds is exact.
7. **`LotSelection` principal conservation.** `resolve.rs:1423-1445` — `Σ picks.sat != principal` raises the
   **Hard** `LotSelectionInvalid` and DROPS the selection. This closes the over-consumption channel
   (Σ picks > principal would spread fixed proceeds over more sats ⇒ more basis ⇒ smaller gain). It has to
   live there because `consume_picks` hardcodes `shortfall = 0` (`pools.rs:156-173`) and
   `selection_feasible` checks per-lot availability only — the code says so at `resolve.rs:1387-1389`.
8. **Cross-account identification.** `pools.rs:118-130` refuses a pick living in another wallet's pool with
   the §1.1012-1(j) citation, and post-2025 pools are per-wallet by `pool_key` (`pools.rs:15-21`).
9. **§1222 / §1211 / §1212 netting.** `net_1222` (`compute.rs:135-190`): within-character netting first,
   cross-netting with the residual keeping its surviving character, `loss_limit` $3,000 / **$1,500 MFS**
   (`tables.rs:240-245`), §1212(b)(2) short-term-first absorption of the allowed deduction. The §1212(b)(2)(B)
   Capital Loss Carryover Worksheet is separately transcribed line-by-line
   (`tax/capital_loss_carryover.rs`) with the taxable-income term the flat rule was missing.
10. **§1091 wash sales.** The claim is *stated at the strength the authority supports*, which is what I was
    asked to check. `optimize.rs:7-17` argues from the statutory text ("stock or securities"), cites
    Notice 2014-21 for property characterisation, notes Rev. Rul. 2023-14 is *not* on the §1091 scope
    question, says no extending statute has been enacted, and carries a MONITOR obligation with a KAT
    (`tests/optimize_wash_sale.rs`). The terser `forms.rs:85` comment ("wash sale is N/A to crypto") is the
    weakest wording in the repo but is not load-bearing. Separately, **securities** entering via 1099-B are
    gated: `Form1099B::basis_reported_and_no_adjustments` is a YES-condition on both limbs, where `None`
    and `Some(false)` both refuse (`return_inputs.rs:173-187`) — so a wash-sale-adjusted broker row cannot
    slip in as a line-1a total.
11. **Form 8949 boxes and adjustments.** Only the conservative "not reported to the IRS" boxes are ever
    emitted — C/F pre-TY2025, I/L from TY2025 (`forms.rs:29-52`, `forms.rs:132-143`), matching
    `design/forms/extract/f8949--2025.txt:25,51-53,71,97-99`. The 1099-reported boxes are never
    auto-asserted, and column (f)/(g) are always empty/zero, so no adjustment can reduce a reported gain.
    An exchange disposition sets `box_needs_review`. *(A box that should have been G/H/J/K is a
    matching/reporting mismatch, not a gain change.)*

Also checked and non-filing: `whatif.rs` (KAT `whatif_never_persists`, `whatif.rs:6`) and
`project/evaluate.rs` (clone-fold-discard, `evaluate.rs:178-181`) — both read-only, neither writes events.

**Owner-mandated policies: conformance verified, not challenged.**
- *Self-transfer = treatment (c).* `fold.rs:918-978` relocates fragments carrying `acquired_at`,
  `donor_acquired_at`, basis and dual-loss basis; no `Disposal`/`Removal`; `basis_source` becomes
  `CarriedFromTransfer` (a tranche keeps `EstimatedConservative`). Conforms.
- *Inbound self-transfer defaults to long-term.* `fold.rs:1155` uses `long_term_default_acquired`, and the
  `SelfTransferInboundDefaultedAcquired` advisory fires on `acquired_at.is_none()` **independently of**
  `basis.is_none()` (`fold.rs:1167-1178`) — exactly the [R0-I2] requirement in
  `design/SPEC_reconcile_defaults.md:52-57`. Conforms. The rate exposure is the owner's accepted trade.
- *Gifts made recognise no gain* (`fold.rs:1277-1286`, §102) and *§170(e) reduction*
  (LT → FMV; ST → `min(FMV, basis)`, `fold.rs:1374-1382`) are both correct, with the dealer/inventory and
  private-foundation caveats disclosed in the `QualifiedAppraisalNote` text.

---

## WHAT I DID NOT READ

- `crates/btctax-core/src/tax/return_1040.rs` (9,392 lines) — read only `capital_net`,
  `capital_gain_line7`, `crypto_charitable_gifts`, `form_1099b_gains` and the head of `screen_absolute`.
- `crates/btctax-core/src/tax/printed.rs` beyond the 8949 / Schedule D slice (lines 80-150);
  `advisories.rs` beyond the charitable-donee and FBAR blocks; `line_coverage.rs`, `packet.rs`,
  `questions.rs`, `scrub*.rs`, `form6251.rs`, `qbi*.rs`, `se.rs`, `other_taxes.rs` — not disposition.
- `conservative_promote.rs`, `tranche_guard.rs`, `project/transition.rs`, `donation.rs` internals — basis
  construction, the other agent's half. I read `clamped_leg_basis`'s two **call sites** only, to confirm
  they clamp downward (never upward) at `fold.rs:209-214` and `fold.rs:290-295`.
- `crates/btctax-tui`, `crates/btctax-tui-edit`, `crates/btctax-input-form`, `crates/btctax-forms` fill
  code, `crates/btctax-adapters`, `crates/btctax-oracle-harness`.
- `optimize.rs` lines 556-1130 (contention grouping, `optimize_year`, `exhaustive_min`,
  `coordinate_descent`) — I read the gates (`persistability`, `proposed_compliance_status`,
  `compliance_overlay`, `assignment_conserves_principal`, `score_assignment`,
  `available_lots_before*`, `timing_insight`) and the CLI accept path's doc contract, not the search.
- The test suite was not executed. No claim here rests on a test passing; the two tests quoted are quoted
  for the *values and doc comments they assert*, both read directly from source.

**One structural caveat on my own coverage.** F1–F4 are all the same shape — *a verdict the engine computes
and then does not act on*. I found four; I did not enumerate every consumer of every verdict. A mechanical
sweep would be worth more than another read: for each of `ComplianceStatus`, `BlockerKind` and
`state.pending_reconciliation`, grep every consumer and ask whether any of them changes a filed number or
gates a filed artifact. Three of the four findings above would have fallen out of that grep in minutes.
