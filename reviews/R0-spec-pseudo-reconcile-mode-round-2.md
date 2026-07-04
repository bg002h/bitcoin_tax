# R0 — SPEC_pseudo_reconcile_mode.md — round 2

**Artifact:** `design/SPEC_pseudo_reconcile_mode.md` (sub-project 2 of auto-pseudo-reconcile).
**Baseline:** branch `feat/pseudo-reconcile-mode` @ `6c2f20e` (main `514875b`). Read-only architect verification of the folded spec.
**Bar:** 0 Critical / 0 Important. **Round-1:** `reviews/R0-spec-pseudo-reconcile-mode-round-1.md` (2C/5I/6M/2N).
**Scope of this pass:** confirm each round-1 fold is (a) present in the AUTHORITATIVE "R0 round-1 folds" section, (b) resolved, and (c) *correct against current source*; then hunt for gaps the folds introduce.

## Verdict: **0 Critical / 1 Important / 4 Minor / 2 Nit** — NOT R0-GREEN

Both Criticals are cleanly resolved and structurally correct against source. C1's taint-propagation path (Lot→Consumed→leg) is threadable; C2's DecisionConflict/ImportConflict split is honest and matches `resolve.rs`. I1, I3, I4, I5, M4, M6, N1 all check out against source. **One Important remains: the fold's I2 restatement re-introduces a source-contradicted claim** — "the estimate still computes with them surfaced" is false (`compute.rs:242`/`:445` gate *every* year on *any* Hard blocker), the I2 exclusion set mislabels `TaxTableMissing`, and the still-uncorrected defaults-table row (line 67) wrongly says the placeholder profile clears `TaxYearNotComputable`. This is precisely the honesty defect I2 existed to fix, so I2 is not fully resolved. Everything else is Minor/Nit. Details below.

---

## Per-fold confirmation (against source)

### C1 — pseudo taint propagates Lot→Consumed→leg — **RESOLVED + correct**
- The synthetic `SelfTransferMine{basis:$0}` origin lot is created at `fold.rs:994-1008` (`Op::SelfTransferInbound`; `usd_basis = basis.unwrap_or(Usd::ZERO)` at `:980`, `basis_source: BasisSource::SelfTransferInbound` at `:1004`). Confirmed the fold's cite of `fold.rs:958-1012`.
- The threading path physically exists and is additive: `Lot` (`state.rs:100-111`) → `Consumed` (`pools.rs:289-302`, built from lot fragments on consume) → `DisposalLeg` (`state.rs:124-140`, built in `make_disposal_legs` `fold.rs:200-211`) / `RemovalLeg` (`state.rs:157-171`) / `PendingLeg` (`state.rs:198-203`, built in `Op::PendingOut` `fold.rs:720-728`). A `pseudo: bool` can be added to each struct; none is serialized by an export writer (see I4). The seed is at lot creation from a pseudo Eff (see I1/N-new).
- **On-screen YES, output NO — provable:** `render_report` renders held lots (`render.rs:211-227`) and per-leg via `render_disposal_leg` (`render.rs:252`) — both can carry a `[PSEUDO]` marker. The two export writers list their columns explicitly — `lots.csv` (`render.rs:588-599`) and `disposals.csv` (`render.rs:617-639`) — so *omitting* a pseudo column keeps output clean and satisfies the ★ grep-KAT. The fold's cite "render.rs:617-639 (disposals)… on-screen only" is slightly loose (617-639 is the CSV **export** writer, not the on-screen path; the on-screen path is `render_report` at `:195`/`:252`), but the intent — that leg basis/gain is where the marker must show on-screen and be omitted from the CSV — is correct. See M-new-2 for the propagation-sink enumeration caveat.

### C2 — accept-first clears `ImportConflict` only; `DecisionConflict` stays surfaced — **RESOLVED + correct + honest**
- `ImportConflict` is a system event awaiting a choice: resolved by a real `SupersedeImport`/`RejectImport` writing into `conflict_res` (`resolve.rs:430-455`), which becomes an `applied` override or, if `None`, the `ImportConflict` blocker (`resolve.rs:457-472`). Pseudo can synthesize the Accept at this map layer (insert into `conflict_res`/`applied`) — legitimate and materializable. ✓
- `DecisionConflict` is a collision of **two REAL decisions** — e.g. duplicate `ClassifyInbound` for the same TransferIn (`resolve.rs:630-640`: first-wins, "Second decision EXCLUDED"), or a non-TransferIn target (`:642-649`). There is no unresolved event awaiting a default; the event is over-specified. Clearing it needs either a persisted void (breaks not-persisted) or in-memory suppression of a real decision (breaks real-supersedes). The fold correctly keeps `DecisionConflict` surfaced. Honest and source-accurate. ✓

### I1 — no `EventId::Decision{seq}` minting; inject at resolve MAP layer + `pseudo_ids` on `Eff` — **RESOLVED + feasible**
- `EventId::Decision{seq: u64}` is keyed solely by `seq` (`identity.rs:69`, `:82-83`, canonical `:103`); real decisions live in the same seq space (`resolve.rs:420-427`). A "derived" synthetic seq could collide — the fold's avoidance is justified. ✓
- `Eff` (`resolve.rs:102-111`) carries `id`/`op`/`wallet`; it is `Vec<Eff>` that fold consumes. Adding an `Eff`-level pseudo signal (bool or a `pseudo_ids` set threaded alongside the timeline) is feasible and reserves seq-minting for `approve`'s `append_decision`. ✓ (One under-specified seam — the Eff→Lot seed — noted as N-new-1.)

### I2 — "0 Hard CLASSIFICATION blockers" + enumerated exclusions — **NOT fully resolved (see [I2-R] Important below)**
The reframing to "0 Hard classification blockers" is right and matches the Hard set (`state.rs:71-83`): pseudo clears `Unclassified`/`UnknownBasisInbound`/`ImportConflict` (+ `TaxProfileMissing` via the placeholder). But the *closing clause* and the *exclusion labelling* are wrong against source — see the Important finding.

### I3 — interim export guard (refuse pseudo-active export) — **RESOLVED + feasible**
- `export_snapshot` (`admin.rs:45-85`) projects (`session.project()` `:53`) and calls `write_csv_exports` unconditionally (`:78`), emitting fictional `lots/disposals/form8949/schedule_d` with no marker. It has `state` in hand and can query the pseudo-contribution signal and return `Err` before writing. The interim refuse-until-sub-3 gate is implementable exactly as the fold states. ✓ (Interaction with M6 flagged as M-new-1.)

### I4 — dedicated marker bool the export writers OMIT (not `BasisSource::Pseudo`) — **RESOLVED + correct**
- `lots.csv` serializes `basis_source_tag(l.basis_source)` (`render.rs:596`); a `BasisSource::Pseudo*` variant would leak "PSEUDO" into the export and fail the grep-KAT. The fold's "dedicated bool the writers omit, never encoded in `basis_source`/`kind`/`term`/`detail`" is the correct channel. ✓

### I5 — TransferOut default = leave-as-pending (`Op::PendingOut`) — **RESOLVED + correct**
- `Op::PendingOut` (`fold.rs:698-740`) folds an unmatched outflow to a `PendingTransfer` + **advisory** `UnmatchedOutflows` (`:736-740`) — already non-taxable (no `Disposal`, no gain). Leaving it as-is needs no synthetic and matches today's behavior; `approve` writes the concrete self-transfer the user confirms. Internally consistent (the residual advisory is Advisory, never gates). ✓

### M4 — `apply_bulk_*` own-loop, not `persist_bulk_decisions` — **RESOLVED + correct**
- The CLI-side reuse target is the `apply_bulk_*` pattern in `crates/btctax-cli/src/cmd/reconcile.rs` — `append_decision` loop + single `session.save()` with `?`-before-save rollback (e.g. `apply_bulk_accept_conflicts:475-490`, `apply_bulk_self_transfer_in:286-305`). `persist_bulk_decisions` lives in `btctax-tui-edit` (`persist.rs:432`), which depends on `btctax-cli` (`crates/btctax-tui-edit/Cargo.toml:19`) → the dep cycle the fold names is real. ✓

### M6 — placeholder profile at CLI layer (`report_tax_year` + `export_snapshot`), not resolve — **RESOLVED + correct (one tension: M-new-1)**
- `compute_tax_year` takes `profile: Option<&TaxProfile>` (`compute.rs:228-272`), and `report_tax_year` reads `s.tax_profile(year)` then passes `profile.as_ref()` (`tax.rs:66-68`). So a mode-on placeholder is injectable at the CLI layer without persisting. `export_snapshot` reads its own profile (`admin.rs:61`). Site is correct; the export-side placeholder collides with I3 (see M-new-1). ✓

### N1 — mode flag defaults `false` — **RESOLVED + feasible**
- `ProjectionConfig` (`mod.rs:31-40`) is `Copy` with a manual `impl Default` (`:41-50`); adding `pseudo_reconcile: bool` keeps `Copy`, and `Default` can be `false`. `CliConfig::to_projection` (`config.rs:30-34`) must carry it; `read_config` (`config.rs:76+`) has the corrupt-value discipline to mirror. ✓

---

## Findings

### [I2-R] IMPORTANT — the I2 fold re-introduces a source-contradicted computability claim; the exclusion set is mislabelled; the defaults table still misstates what the placeholder clears

**Fold text (AUTHORITATIVE, spec lines 24-27):** "…real-decision-defect Hard kinds (incl. DecisionConflict, C2). Enumerate these exclusions; **the estimate still computes with them surfaced.**"

**Evidence.** `compute_tax_year` gates on `first_hard_blocker(state)` — "**ANY** unresolved `severity()==Hard` blocker **ANYWHERE** in the projection gates **EVERY** year" (`compute.rs:237-256`; `first_hard_blocker` = `state.blockers.iter().find(|b| b.kind.severity()==Hard)`, `compute.rs:445-450`). The excluded kinds are all Hard (`state.rs:71-83`): `UncoveredDisposal`, native-`Income` `FmvMissing`, `DecisionConflict`, `SafeHarborUnconservable`, `MethodElectionBackdated`, `LotSelectionInvalid`, `Pre2025MethodConflictsAllocation`, `TaxTableMissing`. Therefore, whenever any of them is *surfaced*, `compute_tax_year` returns `TaxOutcome::NotComputable(TaxYearNotComputable)` and **no estimate is produced** — the exact opposite of "the estimate still computes with them surfaced." The clause is unconditionally false; there is no configuration where a Hard blocker is surfaced AND the year computes.

Two compounding errors:
1. **Exclusion mislabelling.** `TaxTableMissing` (no bundled tax table for the year, `compute.rs:257-264`) is a missing-bundle/data defect, **not** a "real-decision-defect Hard kind"; the umbrella phrase does not cover it, so the enumeration the task asks for is incomplete vs the BlockerKind set. (Round-1 I2 had listed it explicitly.)
2. **Uncorrected defaults-table row.** Line 67 still reads "`TaxProfileMissing` / `TaxYearNotComputable` | … PLACEHOLDER profile … so `report --tax-year` computes an estimate." The placeholder clears **only** `TaxProfileMissing` (`compute.rs:265-272`). `TaxYearNotComputable` is the aggregate gate (`compute.rs:242`), cleared **only** by clearing every underlying Hard blocker — which pseudo does for the classification kinds but explicitly does not for the excluded kinds. Listing `TaxYearNotComputable` as placeholder-cleared is wrong. The authoritative fold does not correct this.

**Why Important.** I2 (round 1) existed to make the goal honest; the fold restated the *goal* correctly ("0 Hard classification blockers") but then re-asserted the very over-promise it removed. This mis-shapes T3: the KAT is "≈zero tax, 0 classification-blockers end-to-end" (spec line 108), and "≈zero tax" is only *assertable* if the estimate computes — which requires 0 Hard blockers **total**, not just 0 classification blockers. An implementer taking the fold literally would build a T3 fixture containing (say) an `UncoveredDisposal`, expect an estimate "with it surfaced," and the KAT would fail (`NotComputable`, nothing to assert ≈zero against). It also overstates the feature's user-facing deliverable.

**Fix (one paragraph).** Replace the closing clause with the honest rule: *the on-screen estimate computes only when `state` carries **zero** Hard blockers (`compute.rs:242`); pseudo clears the classification kinds (`Unclassified`/`UnknownBasisInbound`/`ImportConflict`) + `TaxProfileMissing` (placeholder), so a ledger whose Hard blockers are all classification kinds becomes computable — but if any excluded Hard kind remains (`UncoveredDisposal`, native-`Income` `FmvMissing`, `DecisionConflict`, `SafeHarborUnconservable`, `MethodElectionBackdated`, `LotSelectionInvalid`, `Pre2025MethodConflictsAllocation`, `TaxTableMissing`), the year stays `NotComputable` and no estimate is produced. Pseudo does not promise a computable estimate for every ledger.* Correct defaults-table line 67 to say the placeholder clears `TaxProfileMissing` (not `TaxYearNotComputable`). State that the T3 fixture must contain only classification Hard blockers so the estimate computes.

---

### [M-new-1] MINOR — M6 (placeholder at `export_snapshot`) is contradicted by I3 (refuse export while pseudo-active); the export branch is dead

If `export_snapshot` refuses when the pseudo-contribution signal is non-zero (I3), it returns `Err` *before* any placeholder injection, so the M6 "placeholder at `export_snapshot`" site is unreachable while pseudo is active; and while pseudo is inactive there is no placeholder to inject (the placeholder is a mode-on default). The placeholder is therefore a **`report_tax_year`-only** concern. Not a safety issue (refuse is the safe behavior), but the spec names two placeholder sites where only one is live. Clarify: `export_snapshot` under pseudo-active = **refuse** (I3); the placeholder profile applies on the on-screen `report_tax_year` path only.

### [M-new-2] MINOR — C1's "Lot→Consumed→leg" shorthand under-lists the propagation sinks; enumerate all of them in the plan

The fold's general rule ("a row is `[PSEUDO]` if its existence OR basis traces to any synthetic") is correct and complete, but the mechanical thread "Lot→Consumed→leg" omits two sinks the round-1 C1 named: (a) **relocation** `Consumed → new Lot` when a later real `SelfTransfer` moves a pseudo lot (`fold.rs:766-813`, `relocated.push(Lot{ usd_basis: c.gain_basis, … })` `:769-783`) — the relocated lot must inherit `pseudo`; and (b) the **held-lot render** (`render.rs:211-227`) and `PendingLeg` (`fold.rs:720-728`). So the plan must thread `pseudo` through four sinks — `DisposalLeg`, `RemovalLeg`, `PendingLeg`, and relocated `Lot`/held-lot row — not just `DisposalLeg`. Under the general rule this is implied; call it out so "Lot→Consumed→leg" is not taken literally.

### [M-new-3] MINOR — the header claims "6M … all resolved," but only M4/M6 are visibly folded; M1/M3/M5 remain uncorrected in the body

The fold header (line 3) says the 6 Minors are "all resolved in the … folds section." Only **M4** and **M6** appear there. M1 (`~zero tax` is misleading with real Sells consuming pseudo $0-basis lots → max gain) is still asserted uncorrected at line 69 and echoed in T3 (line 108). M3 ("per-row markers" attributed to `verify`, which shows blocker lists not per-row disposals — `render.rs:1771-1822`) is still at line 73. M5 (the `Unclassified`→self-transfer default needs a two-hop `ClassifyRaw→TransferIn` synthesis, and "acquire-without-wallet" `Unclassified` can't be classified) is not addressed. M2 is subsumed by the I2 "Hard-only success metric." None of M1/M3/M5 blocks the 0C/0I bar, but the "all resolved" claim is inaccurate — fold them or state they are consciously deferred.

### [M-new-4] MINOR — "~zero tax null-hypothesis" (line 69) remains misleading (= uncorrected round-1 M1)

Line 69 "all movement non-taxable, ~zero tax" is false when the ledger has imported Sells consuming pseudo $0-basis lots (`proceeds − 0` = max gain; `fold.rs:190-198`, `render.rs:617-639`). This is the same root as I2-R and motivates why C1's on-screen flag is load-bearing. Reword to "a conservative (often high, not zero) estimate — non-Sell movement non-taxable; imported Sells taxable at pseudo-derived, often $0, basis."

---

## Nits

### [N-new-1] NIT — spell out the Eff→Lot seed step that connects I1 and C1
I1 puts pseudo-ness at the `Eff`/map layer; C1 puts a `pseudo` bit on `Lot`→`Consumed`→leg. The seam — "when `fold` creates a `Lot` from a pseudo `Eff` (e.g. `Op::SelfTransferInbound` at `fold.rs:994`), set `Lot.pseudo = true`" — is implied but never stated. One sentence removes the ambiguity between "a `pseudo_ids` set on `Eff`" and "a `pseudo` bool on `Lot`."

### [N-new-2] NIT — N2 approve order (unchanged from round 1)
`append_decision` assigns seqs by insertion order (`reconcile.rs:33`); feed the approve set sorted by target `EventId` for deterministic seqs (NFR4). Already noted in the fold (line 39); no action needed beyond keeping it in the plan.

---

## ★ Task-question summary

- **C1 — consume path can carry a pseudo bit to the leg; render on-screen only?** YES. `fold.rs:994-1008` creates the synthetic lot; the `Lot`→`Consumed`(`pools.rs:289-302`)→`DisposalLeg`(`state.rs:124-140`) chain is additive; `render_report` (`render.rs:211-227`,`:252`) can show it and both CSV writers (`render.rs:588-599`,`:617-639`) can omit it. Correct — with the sink-enumeration caveat (M-new-2).
- **C2 — accept-first = ImportConflict only; DecisionConflict stays surfaced; correct + honest?** YES. `ImportConflict` is map-clearable (`resolve.rs:430-472`); `DecisionConflict` is a real-decision collision (`resolve.rs:630-640`) that can't be cleared without breaking not-persisted or real-supersedes. Honest.
- **I1 — no `EventId::Decision` minting; inject at map layer + `pseudo_ids` on `Eff`; feasible?** YES. `identity.rs:69` collision risk is real; `Eff` (`resolve.rs:102-111`) can carry the signal; seq-minting reserved for `approve`.
- **I2 — "0 Hard classification blockers" + exclusions complete/accurate vs BlockerKind set?** **NO — the fold's closing clause is source-false and the exclusion set mislabels `TaxTableMissing`; defaults-table line 67 still misstates the placeholder.** See [I2-R] Important.
- **I3/I4/I5 — export guard / dedicated marker bool / leave-as-pending?** All confirmed against `admin.rs:45-85`, `render.rs:596`, `fold.rs:698-740`.
- **M4/M6 — `apply_bulk_*` own-loop / placeholder at CLI layer?** Confirmed against `reconcile.rs:475-490`+`Cargo.toml:19` and `tax.rs:66-68`+`compute.rs:228`. (M6 export-site tension = M-new-1.)
- **New gaps / residuals?** One Important (I2-R). Four Minors (export/placeholder tension; C1 sink list; header over-claim + uncorrected M1/M3/M5; "~zero tax" wording). Two Nits.
- **Is the spec implementable end-to-end with the ★ guard provable?** YES for the ★ on-screen-yes/output-no guard — the render/export split is real (`render_report` vs `write_csv_exports`), `snapshot.sqlite` is auto-clean (synthetics never persist; `project` is a pure read), the marker is a dedicated omitted channel (I4), and taint propagation (C1) closes the real-Sell-of-pseudo-basis hole. The remaining blocker to GREEN is the I2 honesty defect about **computability** — orthogonal to the ★ guard but load-bearing for the T3 KAT and the feature's stated deliverable.

**Not R0-GREEN.** Fold [I2-R] (one paragraph + one table-row correction), then re-review per §2 — including the last fold.
