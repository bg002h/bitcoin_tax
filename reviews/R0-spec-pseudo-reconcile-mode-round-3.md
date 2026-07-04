# R0 — SPEC_pseudo_reconcile_mode.md — round 3

**Artifact:** `design/SPEC_pseudo_reconcile_mode.md` (sub-project 2 of auto-pseudo-reconcile).
**Baseline:** branch `feat/pseudo-reconcile-mode` @ `80c5636` (main `514875b`). Read-only architect verification; do NOT implement.
**Bar:** 0 Critical / 0 Important. **Prior rounds:** round 1 `reviews/R0-spec-pseudo-reconcile-mode-round-1.md` (2C/5I/6M/2N); round 2 `reviews/R0-spec-pseudo-reconcile-mode-round-2.md` (0C/1I/4M/2N).
**Scope of this pass:** (1) confirm the round-2 blocker **[I2-R]** is fixed AND correct against source; confirm the round-2 Minors are addressed. (2) Confirm SELF-CONSISTENCY after the in-place surgical rewrite — no residual contradictions across Goal / Mechanism / defaults-table / Guard / Invariants / Plan, the appended "folds" section cleanly gone, nothing orphaned or double-stated, and the still-load-bearing folds (C1/C2/I1/I3/I4/I5/M4/M6) still hold.

## Verdict: **0 Critical / 0 Important / 2 Minor / 2 Nit** — **R0-GREEN**

The round-2 Important **[I2-R] is fully and correctly resolved** against source, in all three of its sub-defects, and the T3 KAT description is reframed to match. The round-2 Minors are addressed (M-new-1 export-placeholder tension eliminated; M-new-2 sink list expanded; M-new-3 header over-claim gone / M3 verify-vs-render split honest; M-new-4 standalone "~zero tax" dropped). The spec is **self-consistent after the surgery** — no contradictions across the six structural regions, the appended folds section is gone with no orphaned or double-stated claims, and all eight still-load-bearing folds re-verify against current source. Two residual Minors carry over from earlier rounds (both twice-deferred, non-blocking, self-correcting under the T3 TDD plan) and two Nits. **This spec may proceed to implementation.**

---

## Job 1 — [I2-R] fixed + correct (verified against source)

The round-2 Important had three compounding sub-defects. Each is now fixed; I re-derived each against current source.

### (a) The source-false "still computes with them surfaced" clause — REPLACED, now correct
- **Spec now (lines 22-25):** "**A tax TOTAL computes only when 0 Hard blockers of ANY kind remain** — `compute_tax_year` returns `NotComputable` on the first Hard blocker (compute.rs:242,445-450; every excluded kind is Hard, state.rs:71-83). So with any excluded kind present, the user sees the flagged HOLDINGS/skeleton (and those blockers) but not a tax number until they resolve them."
- **Source.** `compute_tax_year` opens with `if let Some(b) = first_hard_blocker(state) { … return TaxOutcome::NotComputable(Blocker{ kind: TaxYearNotComputable, … }) }` (`compute.rs:242-256`). `first_hard_blocker` = `state.blockers.iter().find(|b| b.kind.severity() == Severity::Hard)` (`compute.rs:445-450`). Every excluded kind is `Hard`: `state.rs:71-83` matches `FmvMissing | UncoveredDisposal | ImportConflict | DecisionConflict | UnknownBasisInbound | Unclassified | SafeHarborUnconservable | MethodElectionBackdated | LotSelectionInvalid | Pre2025MethodConflictsAllocation | TaxYearNotComputable | TaxProfileMissing | TaxTableMissing => Severity::Hard`. So the new clause is exactly true: any surfaced Hard blocker ⇒ `NotComputable`, no total. The prior over-promise is gone. ✓
- The binding rule ("0 Hard of ANY kind" + the pointer "every excluded kind is Hard, state.rs:71-83") is **stronger and more robust than an enumerated list** — it captures the whole Hard set by reference to `severity()`, so the Goal's curated 4-item surfaced list (line 20) does not need to be exhaustive. Acceptable, not a defect.

### (b) `TaxTableMissing` mislabel — RELABELLED, now correct
- **Spec now (line 21):** "`TaxTableMissing` (a missing-bundle defect)". **Source:** `compute.rs:258-264` returns `TaxTableMissing` when `tables.table_for(year)` is `None` ("no bundled tax table for {year}") — a data/bundle defect, not a real-decision-defect. Relabel is source-accurate. ✓

### (c) Defaults-table placeholder row — CORRECTED, now precise
- **Spec now (line 51):** "CLI-layer PLACEHOLDER profile … injected at `report_tax_year` (tax.rs:66-68) — clears `TaxProfileMissing` ONLY (compute.rs:265-272), NOT `TaxYearNotComputable`."
- **Source, re-derived by control-flow order.** `compute_tax_year` checks in order: (1) `first_hard_blocker` → `TaxYearNotComputable` (`:242`); (2) missing table → `TaxTableMissing` (`:258`); (3) `let Some(profile) = profile else { return … TaxProfileMissing }` (`:266-272`). `TaxProfileMissing` is a **compute-time outcome** produced by branch (3) when `profile` is `None`; it is not pushed into `state.blockers` during projection, so it never trips `first_hard_blocker`. Injecting a placeholder profile makes branch (3) pass ⇒ clears `TaxProfileMissing` **only**. `TaxYearNotComputable` is produced solely by branch (1), gated on `state.blockers` Hard set, which the placeholder profile never touches ⇒ the placeholder **cannot** clear `TaxYearNotComputable`. The spec's claim is exactly correct. ✓
- `report_tax_year` (`tax.rs:66-68`) reads `s.tax_profile(year)` then passes `profile.as_ref()` into `compute_tax_year` — the CLI-layer injection site is real and non-persisting. ✓

### (d) T3 KAT reframed (the mis-shape [I2-R] warned about) — FIXED
- **Spec now (Plan T3, line 94):** "the '0 Hard classification blockers; tax TOTAL only at 0 Hard total' end-to-end KAT (I2-precise)." The old "≈zero tax, 0 classification-blockers" KAT phrasing is gone. The KAT now asserts the **computability rule**, not an estimate value, so it is buildable against a fixture whose Hard blockers are all classification kinds. ✓

**[I2-R] is RESOLVED + correct in all four aspects.**

### Round-2 Minors — status
- **[M-new-1] (export-placeholder dead branch): RESOLVED.** The spec now names **only** `report_tax_year` (tax.rs:66-68) as the placeholder site (line 51); `export_snapshot` under pseudo-active **refuses** (I3, line 60-62). No dead second site. `export_snapshot` (`admin.rs:45-85`) projects at `:53` then writes at `:78` with `state` in hand, so it can refuse before writing. ✓
- **[M-new-2] (C1 sink list under-listed): substantially ADDRESSED.** Line 36-37 now threads the `pseudo` bit through `Lot` **(incl. relocated lots fold.rs:766-813)** → `Consumed` (pools.rs:289-302) → `DisposalLeg`/`PendingLeg`/held-lot row (state.rs:124-140). The relocation sink (`fold.rs:766-813`, `relocated.push(Lot{ usd_basis: c.gain_basis, … })`), `PendingLeg` (built in `Op::PendingOut` fold.rs:720-728), and the held-lot render row are now named. Residual: `RemovalLeg` is not in the enumeration — covered by the general rule ("existence OR basis traces to any synthetic") → downgraded to Nit N2 below. ✓
- **[M-new-3] (header over-claim + M1/M3/M5): ADDRESSED at header level.** The header (lines 3-5) no longer claims "all Minors resolved"; it states the round-2 findings were merged in place and the appended section removed. M3 (verify shows blocker-lists, not per-row disposals) is now honest: line 57-58 puts `[PSEUDO]` on the **render** report/TUI rows (render.rs:211-227,252) and a **separate** `PseudoReconcileActive` **advisory** blocker in `verify`. M1/M5 residuals remain (see M1/M2 below). ✓ (partial by design; residuals tracked)
- **[M-new-4] (~zero tax wording): standalone claim DROPPED.** The blunt "all movement non-taxable, ~zero tax" line is gone. A qualified "≈zero-tax null-hypothesis" survives once at line 25 — see residual Minor M1. ✓ (partial)

---

## Job 2 — self-consistency after the surgical rewrite

Swept all six structural regions for contradictions introduced by the in-place fold. **None found.**

- **Appended "folds" section is gone, cleanly.** Only meta-reference is the header note at line 4 ("the earlier appended folds section is removed, body is now self-consistent"). No "see folds below" / "R0 round-1 folds section" pointers survive. The inline `[R0-C1]`/`[R0-I2]`/… tags are provenance annotations on live claims, not references to a removed section — no orphaned or double-stated claim. ✓
- **per-event vs data-taint.** Line 35 is unambiguous and singular: "a per-event flag is INSUFFICIENT. A `pseudo` bit rides the DATA." No competing per-event mechanism stated elsewhere. The Eff-layer `pseudo_ids` set (line 32, "which events got synthetics") and the data-layer `pseudo` bit (line 35-36, "which lots/legs are tainted") are complementary layers, not rival encodings — no contradiction. The seam between them is unstated (Nit N1). ✓
- **accept-first scope.** Consistent everywhere: Goal clears `ImportConflict` (line 18); defaults-table `ImportConflict → SupersedeImport` of first-seen, map-clearable resolve.rs:430-472 (line 49); `DecisionConflict` **NOT cleared**, resolve.rs:630-640 (line 50); Goal surfaces `DecisionConflict` (line 20); Gotchas restate the not-cleared set (line 106). ✓
- **"0 blockers" wording.** Goal header "clear the Hard CLASSIFICATION blockers" (line 13); "0 blockers means: clears the Hard classification kinds" (line 17); binding "0 Hard of ANY kind" for the total (line 22); T3 "0 Hard classification blockers; tax TOTAL only at 0 Hard total" (line 94). The classification-clear vs total-computability distinction is now consistently drawn. ✓
- **EventId minting.** No mint at injection (line 33, "Do NOT mint `EventId::Decision{seq}`"; identity.rs:69 collision risk); mint at approve (line 67-68, `apply_bulk_*` → `append_decision`; line 107 Gotcha "seq-minting in approve"). Consistent. ✓
- **marker channel.** One `pseudo` bool, threaded through the data (C1), shown by render (line 57), omitted by CSV/form writers, never a `BasisSource` variant (line 58-59 + Gotcha line 108, render.rs:596 leak). The I4 "dedicated bool" and the C1 "pseudo bit riding the data" are the same channel — consistent. ✓
- **TransferOut default.** defaults-table "leave as `Op::PendingOut`" (line 47); Gotcha "only TransferOut withdrawals default (to leave-pending)" (line 105); T3 "leave-pending outflow" (line 93). `Op::PendingOut` (fold.rs:698-740) already yields a non-taxable `PendingTransfer` + an **advisory** `UnmatchedOutflows` (`:736-740`), so "leave-pending" is an honest no-op default that touches no Hard blocker — internally consistent with the Goal's Hard-classification scope. ✓

---

## Still-load-bearing folds — re-verified against current source (80c5636)

- **C1 (taint Lot→Consumed→leg):** synthetic `SelfTransferInbound` lot at `fold.rs:994-1008` (`usd_basis = basis.unwrap_or(Usd::ZERO)` `:980`); `Consumed` fragment carries basis (`pools.rs:289-302`); `DisposalLeg`/`RemovalLeg`/`PendingLeg` structs in `state.rs:124-140`/`157-171`/… ; relocation `Consumed → new Lot` at `fold.rs:766-813` (`relocated.push(Lot{ usd_basis: c.gain_basis, basis_source: CarriedFromTransfer })`). A `pseudo: bool` is additive on each; none is serialized by an export writer (see I4). ✓
- **C2 (ImportConflict-only):** `ImportConflict` is map-clearable — a synthetic `SupersedeImport` writes `Resolved::Accept` into `conflict_res`/`applied` (`resolve.rs:436-472`); unresolved ⇒ `ImportConflict` blocker (`:465-469`). `DecisionConflict` is a collision of two REAL decisions (duplicate `ClassifyInbound`, first-wins, second EXCLUDED, `resolve.rs:630-640`; non-TransferIn target `:642-654`; contradictory `ClassifyRaw` `:482-487`) — cannot be cleared without breaking not-persisted or real-supersedes. Split is honest. ✓
- **I1 (no seq-mint; inject at map/`Eff` layer):** `EventId::Decision{seq: u64}` keyed solely by `seq` (`identity.rs:69`, canonical `:103`); real decisions live in that seq space (`resolve.rs:420-427`). `Eff` (`resolve.rs:102-111`) carries `id`/`op`/`wallet` and can bear the pseudo signal; seq-minting reserved for `approve`. ✓
- **I3 (export refusal):** `export_snapshot` (`admin.rs:45-85`) has `state` at `:53`, writes at `:78` — can query the contribution signal and `Err` before writing. ✓
- **I4 (dedicated bool the writers omit):** `lots.csv` serializes `basis_source_tag(l.basis_source)` (`render.rs:596`) — a `BasisSource::Pseudo` variant would leak into the export; a dedicated omitted bool does not. On-screen render (`render.rs:211-227` held lots, `:252` per-leg via `render_disposal_leg`) can carry the marker. ✓
- **I5 (leave-pending):** `Op::PendingOut` (`fold.rs:698-740`) → `PendingTransfer` + `PendingLeg` + **advisory** `UnmatchedOutflows` — non-taxable, no synthetic needed. ✓
- **M4 (`apply_bulk_*` own-loop):** `crates/btctax-cli/src/cmd/reconcile.rs` has the own-loop family (`apply_bulk_accept_conflicts:475`, `apply_bulk_self_transfer_in:286`, `…:334/409/537`): `append_decision` loop + single `session.save()` with `?`-before-save rollback; doc-comments at `:324`/`:398` explicitly say NOT the tui-edit `persist_bulk_decisions` (dep cycle). `btctax-tui-edit/Cargo.toml:19` depends on `btctax-cli`, so btctax-cli cannot depend back — cycle is real. ✓
- **M6 (CLI-layer placeholder):** `compute_tax_year(profile: Option<&TaxProfile>)` (`compute.rs:228`); `report_tax_year` reads `s.tax_profile(year)` and passes `profile.as_ref()` (`tax.rs:66-68`) — placeholder injectable at CLI layer without persisting. ✓

---

## Findings (round 3)

### [M1] MINOR — the surviving "≈zero-tax null-hypothesis" label (line 25) can still read as a near-zero promise
Line 25: "the '≈zero-tax null-hypothesis' total is assertable only once they're all cleared." The **computability** claim ("assertable only at 0 Hard total") is now correct. But the **characterization** of that total as "≈zero-tax" is still soft: a REAL imported Sell consuming a pseudo `$0`-basis lot computes `proceeds − 0` = max gain (the spec's own C1 case, lines 38-39). A Sell with basis covering proceeds is not `UncoveredDisposal` (not a Hard blocker), so such a ledger can be at **0 Hard total** yet produce a **high** total. So clearing all Hard blockers makes *a* total assertable, but not necessarily a ≈zero one. This is the residual of round-2 [M-new-4], now qualified and named ("null-hypothesis") rather than asserted, and it does not contradict C1 — so it is Minor, not a honesty defect. **Suggested fix:** drop "zero-tax" from the label (e.g. "the conservative null-hypothesis total"), or add a half-sentence: "…often high, not zero, when imported Sells consume pseudo `$0`-basis lots." Non-blocking.

### [M2] MINOR — defaults-table line 46 lumps `Unclassified` with `UnknownBasisInbound` under one synthetic that resolve rejects for the `Unclassified` case
Line 46 assigns **both** `UnknownBasisInbound` and `Unclassified` inbound the single synthetic `ClassifyInbound(SelfTransferMine{basis:$0})`. This is right for `UnknownBasisInbound` (its event is already a `TransferIn` — `Op::UnknownInbound`, `fold.rs:815-821`), but wrong for `Unclassified`: an `Op::Unclassified` row (`fold.rs:1213-1219`, `BlockerKind::Unclassified` "unclassified BTC-side row") is not yet a `TransferIn`, and `ClassifyInbound` targeting a non-`TransferIn` payload is REJECTED into a `DecisionConflict` (`resolve.rs:642-654`, "ClassifyInbound targets non-TransferIn event"). An `Unclassified` row is cleared by a **`ClassifyRaw`** decision that replaces its effective payload (`resolve.rs:474-492`), not by `ClassifyInbound`. Taken literally, the line-46 synthetic for the `Unclassified` case would inject a decision that produces a NEW **excluded** Hard blocker (`DecisionConflict`) — defeating the Goal's "clears `Unclassified` inbound" claim (line 18). This is the residual of round-1 M5 / round-2 [M-new-3]; it is caught by T3's per-default KAT (line 93, "the defaults … per-default KATs") and does not touch the headline C1/I2 correctness. **Suggested fix:** split the row — `UnknownBasisInbound → ClassifyInbound(SelfTransferMine{$0})`; `Unclassified inbound → ClassifyRaw(→ inbound self-transfer, $0)` — and note that "Unclassified inbound" presupposes the row is determinable-inbound (an acquire-without-wallet `Unclassified` has nowhere to home a lot; leave it surfaced). Non-blocking.

## Nits

### [N1] NIT — the Eff→Lot seed seam between I1 and C1 is still unstated (= round-2 N-new-1)
I1 puts pseudo-ness in a `pseudo_ids` set at the `Eff`/map layer (line 32); C1 puts a `pseudo` bool on `Lot`→`Consumed`→leg (line 35-36). The bridge — "when `fold` creates a `Lot` from a pseudo `Eff` (e.g. `Op::SelfTransferInbound`, `fold.rs:994`), set `Lot.pseudo = true`" — is implied but never written. One sentence in the Mechanism or T2 removes the ambiguity. No action required beyond the plan.

### [N2] NIT — `RemovalLeg` absent from the C1 sink enumeration
Line 36-37 lists `DisposalLeg`/`PendingLeg`/held-lot row but not `RemovalLeg` (`state.rs:157-171`, built for Gift/Donation of a lot). A pseudo lot that is later gifted/donated would flow through `RemovalLeg`. The general rule ("existence OR basis traces to any synthetic") covers it, and Form 8283 removals are a Guard-relevant surface, so name `RemovalLeg` in the T2/T4 sink list for completeness. Nit.

---

## ★ Task-question summary

- **[I2-R] fixed + correct?** YES. (a) The false "still computes with them surfaced" clause is replaced by "a tax TOTAL computes only when 0 Hard blockers of ANY kind remain" — exact against `compute.rs:242`/`445-450` + `state.rs:71-83`. (b) `TaxTableMissing` relabelled "missing-bundle defect" — matches `compute.rs:258-264`. (c) Defaults-table now says the placeholder clears `TaxProfileMissing` ONLY, NOT `TaxYearNotComputable` — exact against the `compute.rs:242` (Hard gate, before profile) vs `:266-272` (profile branch) control-flow order. (d) T3 KAT reframed to the computability rule (line 94).
- **Excluded kinds stay surfaced?** YES — `UncoveredDisposal`, native-`Income` `FmvMissing`, `DecisionConflict`, `TaxTableMissing` (line 20-21); all Hard (`state.rs:71-83`); the binding "0 Hard of ANY kind" generalizes to the whole Hard set.
- **Round-2 Minors addressed?** YES/substantially — M-new-1 export-placeholder tension eliminated (report_tax_year only; export refuses); M-new-2 sink list expanded (relocated `Lot` fold.rs:766-813 / `PendingLeg` / held-lot row); M-new-3 header over-claim gone + M3 verify-vs-render honest; M-new-4 standalone "~zero tax" dropped (residual qualified label = M1).
- **Self-consistent after surgery?** YES — no contradictions across Goal / Mechanism / defaults-table / Guard / Invariants / Plan; per-event-vs-data-taint, accept-first scope, 0-blockers wording, EventId minting, marker channel, TransferOut default all consistent; appended folds section gone with no orphaned or double-stated claim.
- **Load-bearing folds still hold?** YES — C1 (Lot→Consumed→leg incl. relocated fold.rs:766-813), C2 (ImportConflict-only resolve.rs:430-472 / DecisionConflict resolve.rs:630-640), I1 (no seq-mint identity.rs:69; Eff resolve.rs:102-111), I3 (admin.rs:45-85), I4 (render.rs:596), I5 (fold.rs:698-740), M4 (reconcile.rs:475 + Cargo.toml:19), M6 (tax.rs:66-68 + compute.rs:228) — all re-verified.
- **New gaps?** Two Minors (both twice-deferred, non-blocking, self-correcting under T3 TDD): M1 residual "≈zero-tax" label; M2 `Unclassified` needs `ClassifyRaw` not `ClassifyInbound`. Two Nits (Eff→Lot seam; `RemovalLeg` sink).

**R0-GREEN.** 0 Critical / 0 Important. This spec may proceed to implementation. The two Minors and two Nits are recommended for the implementer's attention (M2 in particular should be folded into the T3 defaults) but do not gate the phase.
