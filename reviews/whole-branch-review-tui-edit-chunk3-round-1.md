# Whole-branch review — mutating-TUI chunk 3 (round 1)

**Branch:** `feat/tui-edit-chunk3` @ `b0fffcc` (diff = `main..HEAD`, main == merge-base `7ba67a1`).
**Scope:** the three remaining decision flows — select-lots (`s`), set-donation-details (`d`),
and safe-harbor-attest (`a`, IRREVOCABLE) — added to `btctax-tui-edit`.
**Spec:** `design/SPEC_tui_edit_chunk3.md` (R0 GREEN, 2 rounds; round-1 findings in
`reviews/R0-spec-tui-edit-chunk3-round-1.md`, fold tags `[R0-*]`).
**Method:** Phase E, STANDARD_WORKFLOW §2/§3 — a **panel of three independent, fresh-context
adversarial reviewers** (safety-critical, irrevocable change ⇒ §3 multi-lens with distinct
attack surfaces), each over the whole diff as one system. Every engine-behavior claim was
required to be verified against the ACTUAL `btctax-core`/`btctax-cli` source, not the spec.

**Controller-verified full gate at HEAD (`b0fffcc`), all green:**

```
cargo test --workspace --locked        → 868 passed; 0 failed (exit 0)
cargo clippy --all-targets -- -D warnings → 0 warnings (exit 0)
cargo fmt --all -- --check              → clean
cargo +1.88.0 check --workspace --locked → exit 0 (MSRV; no manifest/dep changes on this branch)
scripts/pii-scan-generic.sh             → clean
```

A red suite would itself be a blocking finding (§6); it is green, so the gate rests on the
review below.

---

## Consolidated verdict — 0 Critical / 2 Important / 5 Minor / 2 Nit

| Lens | Reviewer | C | I | M | N |
|---|---|---|---|---|---|
| Safety & irrevocability | agent 1 | 0 | 0 | 2 | 1 |
| Engine semantics & pre-filters | agent 2 | 0 | 0 | 2 | 1 |
| Test fidelity & spec conformance | agent 3 | 0 | **2** | 1 | 0 |

Both Importants are on the **test-fidelity** lens and are confined to the test + docs surface —
no product-code defect was found by any of the three reviewers. The safety reviewer independently
confirmed the R0-C1 residue latch airtight (all 9 openers, single setter, no `session.save()`
bypass, defense-in-depth traced to `session.rs:124-132`); the engine reviewer confirmed every
pre-filter / retry / status claim against `resolve.rs`/`fold.rs`/`state.rs`/`reconcile.rs`.

### Fold disposition (controller)

- **[I1] Important — KAT-V-DD-4 coverage theatre → FOLD.** Rewrite to drive the production
  pre-population path and assert all 10 buffers; verify by fault injection that it fails when the
  production mapping is broken.
- **[I2] Important — chunk-3 FOLLOWUPS absent → FOLD.** Write the 7 spec-mandated records plus
  the 3 review-surfaced items (SAFE-M2, ENG-m1, ENG-m2).
- **[TF-M1] Minor — ERRLATCH pins 3/9 openers → FOLD** (loop the refusal over all 9; cheap
  strengthening; production already carries all 9 guards).
- **[SAFE-M1] Minor — dead code in the select-lots "no lots" branch → FOLD** (delete
  `main.rs:2713-2718` empty `if let` + `let _ = flow.step`; trim `main.rs:2616` unused bind).
- **[SAFE-N1] Nit — `validate_donation_details` inlines the date parse → DECLINED (nit).**
  Reusing `btctax_cli::eventref::parse_date_arg` would surface `CliError::Usage`'s Display, which
  prepends `"usage: "` (`lib.rs:38`) — a CLI-oriented prefix inappropriate for a TUI field error.
  The inline `time::Date::parse` uses the byte-identical `[year]-[month]-[day]` format and the same
  `"bad date {t:?}: {e}"` wording, and is pinned by KAT-V-DD-3 (`err.contains("bad date")`). Kept
  as-is; the spec §D2 intent ("parse YYYY-MM-DD as the CLI does") is met by identical logic.
- **[SAFE-M2] Minor (pre-existing, out of chunk-3 scope) → FILE FOLLOWUP.** The chunk-2a/2b
  void-remedy statuses omit "quit the editor first"; present verbatim at `main`, in-editor `v`
  remedy is primary → not a chunk-3 regression.
- **[ENG-m1] Minor — pre-2025 Universal-pool wallet under-inclusion → FILE FOLLOWUP** (narrows
  the spec-acknowledged lot-display caveat; under-inclusion only, no wrong tax).
- **[ENG-m2] Minor — shortfall-disposal principal target → FILE FOLLOWUP** (degenerate; already
  Hard `UncoveredDisposal`; surfaced by status Arm 2).
- **[ENG-n1] Nit — dead defensive arms in `derive_attest_status` → NO CHANGE** (correct
  defensive code, already framed as such by its doc-comment).

Re-review after the fold (round 2) verifies the rewritten KAT-V-DD-4 genuinely pins the path and
that the fold introduced no drift, per §2 ("the loop continues after every fold, including the
last").

---

# Reviewer 1 — SAFETY & IRREVOCABILITY lens (verbatim)

## Whole-diff review — chunk-3 (`feat/tui-edit-chunk3`), SAFETY & IRREVOCABILITY lens

**Verdict: 0 Critical / 0 Important / 2 Minor / 1 Nit**

No Critical or Important findings on my lens. The editor's append-only guarantee, the `attest_save_failed` residue latch, typed-word irrevocability, and the two-decision atomic batch are all implemented faithfully and are airtight. Details of what I verified below, then the Minor/Nit items.

### What I verified (all CONFIRMED against current source)

**1. Editor guarantee — persist-only writes.** No vault-mutating token (`.conn(`, `.save(`, `append_decision`, `donation_details::set`, `tax_profile::set`) appears in the product (non-test) region of `main.rs`, `editor.rs`, `edit/form.rs`, or `draw_edit.rs` (verified by my own region-aware grep). The three new persist fns live only in `edit/persist.rs` (`persist.rs:237/258/288`). KAT-G1's `persist_only_tokens` list at `persist.rs:788-794` **does** include `"donation_details::set"`, and the plant-a-token self-check at `persist.rs:967/995-998` constructs and detects it — a regression would fail the gate. `kat_g1_mechanized_source_gate` passes.

**2. The `attest_save_failed` latch [R0-C1] — airtight.**
- Checked as the **first statement** in all NINE openers: `open_profile_form` (411), `open_classify_inbound_flow` (1825), `open_reclassify_outflow_flow` (1927), `open_reclassify_income_flow` (2118), `open_set_fmv_flow` (2230), `open_void_flow` (2416), `open_select_lots_flow` (3222), `open_set_donation_details_flow` (3346), `open_safe_harbor_attest_flow` (3481).
- Set to `true` in exactly ONE product location — the attest Err arm (`main.rs:3723`). Never reset in product code (the only `= false` are the initializer `editor.rs:181` and a deliberate test bypass `main.rs:10966`).
- **No mutating save can bypass a latched opener.** Every `session.save()` lives in one of the 9 persist fns; each is reachable only from a modal/flow Enter arm, which requires a flow/modal to be open, which is opened only by a latch-gated opener. When the latch is set the attest flow has just closed (`main.rs:3701`, before the match) and the at-most-one-flow/modal invariant guarantees nothing else is open — so every later save requires re-entering a latched opener. QUIT discards the in-memory residue.
- Flow closes on BOTH Ok and Err (`main.rs:3701` unconditional). No keep-open retry path.
- Defense-in-depth pre-flight sources ONE `session.load_events_and_project()` (`main.rs:3498`) with no cached-`snap` mixing; I confirmed at `session.rs:124-132` that this reads `load_all(self.conn())` — the live in-memory handle — so it sees the residue and the "already attested" arm (`main.rs:3552`) refuses. `kat_e2e_attest_errlatch_chmod` pins the latch, all-openers-refuse piggy-back guard, and the defense-in-depth arm — passes.

**3. Irrevocability honesty.** Typed word compared exactly `typed != "ATTEST"` case-sensitive (`main.rs:3681`); wrong word only sets the error and **preserves** the buffer (`main.rs:3684-3691`, verified by `kat_e2e_attest_wrong_word_preserves_buf`). Every chunk-3 CLI-pointing status tells the user to quit first: latch status (×9), Err status (`3725-3726`), pre-flight arms (`3533/3567`), `derive_attest_status` (`3765/3771`), and the select-lots conflict status (`3417-3418`). The Info modal shows the honest "CANNOT be voided … PERMANENT Hard DecisionConflict" warning (`draw_edit.rs:2001-2010`).

**4. Atomicity.** `persist_safe_harbor_attest` (`persist.rs:301-322`) appends Void then re-attested SafeHarborAllocation to the same connection with a single trailing `session.save()` — no interleaved save. `..prior_alloc` struct-update sets only `timely_allocation_attested: true`. `kat_p2h` verifies the two-decision strict prefix and round-trip.

---

### [M1] Minor — dead/confusing code in the select-lots "no lots" branch (CONFIRMED)
`main.rs:2713-2718`: an empty `if let SelectLotsStep::List = &flow.step { /* No per-step error */ }` block followed by `let _ = flow.step; // silence unused lint`. Behavior is correct (it sets the global status and stays on List), but the block does nothing and reads like it intended to. Suggest deleting lines 2713-2718 and keeping only the `app.status = Some(...)`. Also `main.rs:2616` (`let _ = (disposal_date, disposal_kind, principal_sat);`) extracts three modal fields never used in that arm — harmless but noise.

### [M2] Minor — pre-existing chunk-2a/2b void-remedy statuses omit "quit the editor first" (CONFIRMED; out of chunk-3 scope)
`derive_classify_inbound_status`/`derive_reclassify_income_status`/`derive_set_fmv_status` emit `"… or CLI: btctax reconcile void {}"` (`main.rs:2004/2022/2040/2076/2298/2327`) without the "quit the editor first" clause that the R0-C1 lock audit mandates. These are **present verbatim at `main`** (not introduced by this diff) and each offers an in-editor `press 'v'` remedy as the primary path, so they are not a safety hole. Flagging only because the review lens asks for a full CLI-status audit: for consistency, the same quit-first fold could be applied to these pre-existing strings in a follow-up.

### [N1] Nit — `validate_donation_details` inlines the date parse instead of reusing `parse_date_arg` (CONFIRMED)
`form.rs:1103-1104` uses `time::Date::parse(t, format_description!("[year]-[month]-[day]"))`, which is byte-for-byte the same format and result type (`TaxDate = time::Date`) as `btctax_cli::eventref::parse_date_arg` (`eventref.rs:80-83`) that spec §D2 named. Functionally identical (only the error-string wording differs); pre-population round-trips because `time::Date::Display` emits the same `YYYY-MM-DD`. No behavioral impact.

---

Files of record: `crates/btctax-tui-edit/src/main.rs` (openers/handlers/status derivers), `crates/btctax-tui-edit/src/edit/persist.rs` (three persist fns + KAT-G1), `crates/btctax-tui-edit/src/edit/form.rs` (validators + flow structs), `crates/btctax-tui-edit/src/editor.rs` (latch field), `crates/btctax-cli/src/session.rs:124-132` (the in-memory `load_events_and_project` that makes the defense-in-depth work).

---

# Reviewer 2 — ENGINE SEMANTICS & PRE-FILTER / STATUS CORRECTNESS lens (verbatim)

# Whole-diff review — ENGINE SEMANTICS & PRE-FILTER / STATUS CORRECTNESS lens

**Verdict: 0 Critical / 0 Important / 2 Minor / 1 Nit**

No blocking findings on my lens. The TUI faithfully mirrors the engine it wraps across all three flows. Every spec claim I was asked to distrust was checked against `resolve.rs`/`state.rs`/`fold.rs`/`reconcile.rs` — and holds. Details of what I verified, then the non-blocking notes.

## What I verified (CONFIRMED against engine source)

**1. select-lots pre-filter (`open_select_lots_flow`, main.rs:3221)**
- **fee-mini exclusion is correct and complete.** `fee_mini_disposition == true` is set ONLY on the TP8-(b) self-transfer fee record (`fold.rs:354-360`), which shares the SelfTransfer's event id (`event: ev.clone()`); native/reclassified disposals set it `false` (`fold.rs:638`). So `.filter(|d| !d.fee_mini_disposition)` (main.rs:3270) removes exactly the fee minis and nothing else. `honoring_principal(Op::SelfTransfer)` is `Some(sat)` (resolve.rs:1013) — the SelfTransfer principal is not in `state.disposals`, so it is correctly absent (acknowledged under-inclusion, see Minor-1).
- **`principal_sat = Σ legs.sat` is the right target.** For every non-shortfall disposal/removal, `Σ legs.sat == op.sat == honoring_principal(op)`, which is exactly what resolve.rs:811-820 conserves against. Fees consume separately (excluded from both legs and the engine target). Match.
- **Wallet sourcing is sound.** `RemovalLeg` has no `wallet` field (state.rs:148-163); `DisposalLeg` does (state.rs:132). Sourcing uniformly from raw `LedgerEvent.wallet` via `events_by_id` (main.rs:3275,3301) is correct and, crucially, matches the wallet the engine actually consumes the pool from (`pool_key(date, eff.wallet)`, fold.rs:576-587,742-754,965-977). Every listed disposal/removal is guaranteed `wallet: Some(_)` because the fold blocks + returns (no record pushed) when `eff.wallet` is `None`.
- **Voided/already-selected logic is correct.** `already_selected` (main.rs:3252-3263) collects `disposal_event` of NON-voided `LotSelection`s only; a voided selection's target correctly re-appears. Comparison is Decision-id vs Decision-id.

**2. select-lots duplicate/retry (`derive_select_lots_status`, main.rs:3405)** — resolve.rs:787-793 fires `DecisionConflict` on the SECOND decision's id and `selections.remove()` drops it so NEITHER applies. `append_decision` gives the new selection `MAX(seq)+1`, so it is always the "second" → the blocker lands on the id `persist_select_lots` returns → Arm 1 (main.rs:3413-3421) keys on the correct id. CONFIRMED.

**3. set-donation-details (`open_set_donation_details_flow`, main.rs:3345)** — filters `RemovalKind::Donation` only; the CLI rejects Gift (reconcile.rs:619-624). `existing_details` from `snap.donation_details` (last-write-wins). `derive_donation_details_status` uses `is_review_complete(Form8283Section::B)` (donation.rs:70-76) correctly; the Section A/B messaging is honest (reports what the stored details satisfy, fabricates no completeness). Form validation requires `donee_name`/`appraiser_name` (form.rs:1073-1082), matching the CLI's required fields. CONFIRMED.

**4. safe-harbor-attest pre-flight (`open_safe_harbor_attest_flow`, main.rs:3480)** — all six arms are a faithful mirror of `reconcile.rs:480-538`: same `voided` set from `VoidDecisionEvent.target_event_id`, same non-voided-allocation live set, same 0/2+/already-attested/unconservable/`!timebar`/timebar ordering, same `b.event == Some(&prior_id) && b.kind == k` blocker keying. CONFIRMED.

**5. `derive_attest_status` [R0-M10] (main.rs:3740)** — I confirmed the engine fact: a void targeting a `SafeHarborAllocation` goes to `allocation_voids`, NOT `voided` (resolve.rs:322-328), and step-3 skips only `voided.contains(&d.id)` (resolve.rs:846-850). So post-attest the voided prior is re-evaluated, stays timebarred (its `timely_allocation_attested` is unchanged), and stamps a STALE Advisory `SafeHarborTimebar` on `prior_id`; the new attested copy bypasses both prongs (resolve.rs:865) and is the sole effective allocation. All four status arms key strictly to `new_attest_id`, so the stale timebar on `prior_id` cannot misfire Arm 2. No arm widens to "no timebar anywhere." CONFIRMED correct.

**6. `persist_safe_harbor_attest` (persist.rs:288)** — builds `SafeHarborAllocation { timely_allocation_attested: true, ..prior_alloc }` (persist.rs:310-313), byte-for-byte matching the CLI (reconcile.rs:551-554): `lots`, `as_of_date`, `method`, `pre2025_method` all preserved → conservation identical → same allocation. Void targets `prior_id` (persist.rs:303-305). CONFIRMED.

## Non-blocking findings

### [Minor m1] MINOR — select-lots candidate-lot wallet filter is semantically wrong for pre-2025 disposals (under-inclusive)
`main.rs:2691` filters candidate lots by `l.wallet == item.wallet`. For a disposal DATED before `TRANSITION_DATE`, the engine consumes from `PoolKey::Universal` — un-partitioned by wallet (pools.rs:15-21) — so `selection_feasible` accepts a `LotPick` from ANY wallet's pre-2025 lot, while the TUI offers only the disposal-wallet's lots. FAILURE SCENARIO: a pre-2025 `Sell` in wallet W1 (principal 500K) whose pre-2025 residue lived mostly in W2; the engine would accept `500K from W2`, but the TUI hides W2's lots and can present no valid selection. Impact is under-inclusion only (no invalid selection can be built, no wrong tax), and it falls within the spec's acknowledged "Lot-display at disposal date … best-effort guide" limitation (SPEC:1310-1311). CONFIRMED. Suggested: when `item.date < TRANSITION_DATE`, drop the wallet filter (offer all lots), or add a FOLLOWUP note narrowing the caveat to name the Universal-pool case explicitly.

### [Minor m2] MINOR — shortfall disposal: displayed/validated `principal_sat` (Σ legs.sat) < engine target (op.sat)
For an under-covered disposal (`UncoveredDisposal`), `Σ legs.sat < op.sat`, so `validate_select_lots` (form.rs:956) conserves against a smaller number than the engine's `honoring_principal` (resolve.rs:811-813). A TUI-passing selection would then be engine-rejected as `LotSelectionInvalid`. This is degenerate (the disposal already carries a Hard `UncoveredDisposal`) and is SURFACED via `derive_select_lots_status` Arm 2 (main.rs:3425-3431) — no silent data loss. CONFIRMED. Acceptable as-is; worth a one-line FOLLOWUP.

### [Nit n1] NIT — `derive_attest_status` Arms 1-3 are effectively dead for `new_attest_id`
The re-attested allocation always bypasses the timebar (attested=true) and conserves iff the prior did (which the pre-flight guaranteed), so `SafeHarborUnconservable`/`SafeHarborTimebar`/`DecisionConflict` cannot land on `new_attest_id` on the normal path. The arms are correct defensive code; the doc-comment already frames them as edge/defensive. No change needed.

## Scope notes
The SelfTransfer select-lots under-inclusion (SPEC:1305-1309) and the lot-display caveat (SPEC:1310-1311) are spec-acknowledged, safe-direction (under-inclusive), CLI-still-available limitations recorded in FOLLOWUPS — not defects. The `set-donation-details` TUI persist (persist.rs:258-266) does not re-validate `RemovalKind::Donation` (unlike the CLI), but the flow's pre-filter guarantees the selected event is a Donation and the editor holds the exclusive lock (no TOCTOU) — sound.

---

# Reviewer 3 — TEST FIDELITY & SPEC CONFORMANCE lens (verbatim)

# Whole-diff review — chunk-3 (select-lots / set-donation-details / safe-harbor-attest)

**Verdict: 0 Critical / 2 Important / 1 Minor / 0 Nit**

The test suite is, with one exception, a faithful and often *strengthened* implementation of the spec's D5 KAT list. The two safety-critical `#[cfg(unix)]` chmod KATs both carry the root-skip guard and genuinely exercise the failure path; the ERRLATCH latch, piggy-back guard, and defense-in-depth pre-flight are all pinned; the discriminating select-lots seed is genuinely discriminating; KAT-G1 correctly enrolls and self-checks the new `donation_details::set` token; no pre-existing assertion was weakened or dropped (assertion counts rose 473→633 / 62→93 / 95→115). The two findings are a spec-named validation KAT that pins substrate instead of production code, and the absent Task-4 FOLLOWUPS.

## Spec-KAT → implementation mapping

| Spec KAT | Found | Location | Pins the load-bearing property? |
|---|---|---|---|
| KAT-P2g (strict-prefix select-lots) | ✅ | persist.rs:1476 | Yes — prefix, seq, LotSelection→out_id, LotId/sat round-trip, drop+reopen |
| KAT-P2h (two-decision attest batch) | ✅ | persist.rs:1813 | Yes — BOTH rows, seq contiguity, `timely_allocation_attested:true`, other fields match, drop+reopen |
| KAT-DD-PERSIST (side-table upsert) | ✅ | persist.rs:1644 | Yes — asserts `post==pre` (NO new decision rows) in-mem, on-disk, and after upsert |
| KAT-C2f (cancel select-lots) | ✅ | main.rs:9198 | Yes — bytes-unchanged, `q` swallowed at List/LotsForm/modal, Esc steps back; complement = E2E-SL |
| KAT-C2g (cancel donation-details) | ✅ | main.rs:9309 | Yes — bytes-unchanged, `q` swallowed at List, Esc steps back; complement = E2E-DD |
| KAT-C2h (cancel attest) | ✅ | main.rs:10136 | Yes — bytes-unchanged, `q` swallowed at Info/TypedWord, partial-word error+buf preserved, latch NOT set |
| KAT-S3a (save-error select-lots) | ✅ | main.rs:9405 | Yes — root-skip guard; modal closed/form open/buf intact/bytes unchanged; retry → 2nd LotSelection, DecisionConflict on the SECOND id, status surfaces conflict |
| KAT-E2E-SL (discriminating) | ✅ | main.rs:9571 | Yes — picks non-FIFO lot **B**, asserts re-projected leg `origin_event_id==lot_b_id` [R0-M6] |
| KAT-E2E-SL-DONATE | ✅ | main.rs:9708 | Yes — Donate kind, wallet sourced from raw event (`is_some`), no LotSelectionInvalid |
| KAT-E2E-SL-VOID | ✅ | main.rs:9802 | Yes — re-appears in list, FIFO restored, `optimize_attest::get==None` |
| KAT-E2E-DD (A→B progression) | ✅ | main.rs:9904 | Partial — drives real path; asserts completeness (none→present→B-complete) but pre-population of only **2 of 10** fields (see I1) |
| KAT-E2E-ATTEST-PREFLIGHT (4 arms + ctrl) | ✅ | main.rs:10235 | Yes — all four failure statuses + positive control opens at Info, prior_id match |
| KAT-E2E-ATTEST (typed-word round-trip) | ✅ | main.rs:10438 | Yes — IRREVOCABLE+canonical-id render, `ATTES`→error+buf preserved, `T`→save, +2 rows, attested id, no timebar on NEW id |
| KAT-E2E-ATTEST-WRONGWORD | ✅ | main.rs:10639 | Yes — `attest`≠`ATTEST` error, buf preserved, then corrects+saves |
| KAT-E2E-ATTEST-VOID (§7.4 trap) | ✅ | main.rs:10699 | Yes — new alloc listed, prior not; void → exact arm-1 rejected wording, DecisionConflict present, still effective |
| KAT-E2E-ATTEST-ERRLATCH | ✅ | main.rs:10826 | Yes — root-skip guard; latch set, flow closed, quit-first status, bytes unchanged; `a`+`f`+`p` refused; defense-in-depth pre-flight refuses appending nothing |
| KAT-V-SL-1..3 | ✅ | form.rs:2030/2045/2061 | Yes — pinned via pure `validate_select_lots` |
| KAT-V-DD-1..3 | ✅ | form.rs:2103/2114/2126 | Yes — pinned via pure `validate_donation_details` |
| KAT-V-DD-4 (pre-population round-trip) | ⚠️ | form.rs:2140 | **No** — re-implements the mapping in-test; never touches production pre-population (see I1) |
| KAT-G1 (source gate) | ✅ | persist.rs:734 | Yes — `donation_details::set` in `persist_only_tokens` + planted-token self-check |

No spec-mandated KAT is missing.

---

### [I1] IMPORTANT — KAT-V-DD-4 is coverage theatre: it never exercises the production pre-population path
`crates/btctax-tui-edit/src/edit/form.rs:2140` (`kat_v_dd_4_pre_population_round_trip`)

The spec (D5 KAT-V-DD-4) mandates: *"Open the set-donation-details FieldForm for an event that already has stored details (read from `snap.donation_details`). Assert all 10 FieldBuffers are pre-populated with the stored values… the canonical test that the 're-edit pre-populates' contract works."* The production code that implements that contract is the hand-written 10-field mapping at `main.rs:2987-3014` (List→FieldForm transition).

The as-built test does **not** open the FieldForm, does not read `snap.donation_details`, and never calls `handle_key`/`open_set_donation_details_flow`. It manually populates 10 buffers *in the test body* (form.rs:2158-2183 — an exact copy of the production mapping) and asserts those test-owned buffers are non-empty and round-trip through `validate_donation_details`. It therefore pins only `FieldBuffer::set` + `validate` round-trip (already-tested substrate), not the production wiring it is named for. KAT-E2E-DD drives the real path but asserts pre-population of only `donee_name` and `appraiser_name` (2 of 10, main.rs:10005-10014).

**FAILURE SCENARIO (CONFIRMED by fault injection):** I dropped the production pre-population of `appraiser_ptin` (main.rs:3002-3004) and both `kat_v_dd_4_pre_population_round_trip` and `kat_e2e_dd_donation_details_completeness_progression` still **passed**. So a wiring bug in any of the 8 optional fields (`donee_address`, `donee_ein`, `appraiser_address`, `appraiser_tin`, `appraiser_ptin`, `appraiser_qualifications`, `appraisal_date`, `fmv_method_override`) ships uncaught. This is not merely cosmetic: because the side-table is **last-write-wins**, a re-edit that opens with a silently-blank optional field and is re-submitted would upsert `None` over the previously-stored value — silent loss of a prior appraiser/donee field. The production mapping is currently correct, so no live data loss; the defect is that the spec's designated regression guard is inert.

**Suggested fix:** Rewrite KAT-V-DD-4 to drive the real path: seed a Donation with stored `DonationDetails` (all 10 fields set), `open_app`, press `d`, `Enter`, then read the `FieldForm` step and assert **each** of the 10 buffers equals the stored value. (Retain the round-trip assertion as a second phase.)

**CONFIRMED.**

---

### [I2] IMPORTANT — the seven spec-mandated chunk-3 FOLLOWUPS are absent from `FOLLOWUPS.md`
`FOLLOWUPS.md` (unchanged on this branch — `git diff --stat main..HEAD -- FOLLOWUPS.md` is empty; last FOLLOWUPS commit is chunk-2b on `main`).

Spec Task 4 mandates recording: SelfTransfer select-lots under-inclusion, lot-display-at-disposal-date, safe-harbor-allocate TUI deferral, WB-I4(a) carryforward, FIELD_CAP=64 CLI-parity limit, void-list effective-allocation pre-filter, and the session-dirty latch generalization. None are present (the `FOLLOWUPS.md` hits for "under-inclusion"/"self-transfer" are pre-existing chunk-2b / core entries, not these seven).

**Why it matters:** these are the acknowledged safe-direction gaps (notably the SelfTransfer select-lots under-inclusion and the void-list effective-allocation trap that KAT-E2E-ATTEST-VOID pins as *today's* behavior) — losing them drops the paper trail the workflow depends on. **Caveat:** this is the Task-4 / Phase-E docs deliverable that this very review gates, so it is in-progress by definition; it is not a code defect. Flagging so it is folded before ship.

**Suggested fix:** add the seven entries verbatim from spec Plan §Task 4 "FOLLOWUPS to record for chunk 3" before Ship.

**CONFIRMED.**

---

### [M1] MINOR — ERRLATCH pins the opener-latch on only 3 of 9 mutating openers
`crates/btctax-tui-edit/src/main.rs:10902-10948` (`kat_e2e_attest_errlatch_chmod`)

The residue-latch guarantee is "EVERY mutating opener (`p/c/o/r/f/v/s/d/a`) refuses." The KAT exercises the latch on `a`, `f`, and `p` only (the spec's KAT text itself mandates only `a`+`f`, so this is *above* spec). I verified production is correct: there are exactly nine `if app.attest_save_failed {` guards (main.rs:411, 1825, 1927, 2118, 2230, 2416, 3222, 3346, 3481) — one per opener — plus the single setter (3723). So the six untested openers (`c/o/r/v/s/d`) do carry the guard; this is a test-thoroughness gap, not a safety hole.

**Suggested fix (optional):** loop the latch-refusal assertion over all nine opener keys so a future opener added without the guard is caught.

**CONFIRMED.**

---

**Notes for the controller:** the whole-diff safety story is otherwise sound — the two chmod KATs genuinely ran their failure paths in this environment (root-skip did not fire), the §7.4 attest-void trap and the C1 piggy-back guard are faithfully pinned, KAT-G1's new-token self-check is real, and no chunk-2 pin was loosened. I did not run the full suite (controller owns the green gate); I ran only KAT-V-DD-4, KAT-E2E-SL, and KAT-E2E-ATTEST-ERRLATCH (all pass) plus the fault-injection probe for I1 (tree restored clean).

---

# Round 2 — fold verification (independent, fresh context) — GREEN

After folding the round-1 findings — **[I1]** KAT-V-DD-4 rewritten to drive the real
`d`→List→Enter→FieldForm path (controller fault-injection-verified: dropping the production
`appraiser_ptin` pre-population fails the test at the `appraiser_ptin` assertion); **[I2]** the
chunk-3 FOLLOWUPS written (7 spec-Task-4 items + ENG-m1/ENG-m2/SAFE-M2); **[TF-M1]** ERRLATCH loops
the latch refusal over all 9 openers; **[SAFE-M1]** dead code removed at root; **[SAFE-N1]** declined
(documented above) — an independent fresh-context reviewer re-verified the fold delta per §2 ("the
loop continues after every fold, including the last").

**Controller gate at the fold (all green):** `cargo test --workspace --locked` → **868 passed /
0 failed** (exit 0); `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0 warnings
(exit 0); `cargo fmt --all -- --check` → clean; PII generic-scan clean; MSRV `cargo +1.88.0 check
--workspace --locked` → exit 0 (no manifest/dep changes on this branch).

## Round-2 verdict (verbatim)

Working tree intact (`main.rs` md5 `d19decd…`, unchanged diffstat). Verdict below.

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — the fold is clean.**

The two round-1 Importants are correctly resolved and no new drift was introduced. Fault-injection confirms the rewritten KAT-V-DD-4 genuinely pins the production path.

### Fold parts (all correctly resolved)

1. **[I1] KAT-V-DD-4 rewrite — RESOLVED.** The old form.rs `kat_v_dd_4_pre_population_round_trip` is deleted with a breadcrumb comment; the new `main.rs::kat_v_dd_4_pre_population_drives_real_path` seeds a full 10-field `DonationDetails` via `persist_donation_details`, opens the app, drives `d`→list→Enter→FieldForm, destructures `&flow.step` and asserts each of the 10 **FieldForm STEP** buffers (`*_buf.buf.trim()`), then Enter→modal asserting `modal.details == details`. It reads the real step's buffers, not test-owned ones.
   **Fault injection (field the author did NOT use):** I commented out the production `fmv_method_override` pre-population block (main.rs:2995-2997). `cargo test -p btctax-tui-edit kat_v_dd_4_pre_population_drives_real_path` FAILED at main.rs:10187 — `assertion left == right failed: fmv_method_override / left: "" / right: "qualified appraisal"`. I then restored byte-for-byte: md5 back to `d19decd866ba4324d9487d939258e919` (identical pre/post) and diffstat unchanged. Test passes on the restored tree. The regression guard is real.

2. **[TF-M1] ERRLATCH loop — RESOLVED.** Loop key list `['p','c','o','r','f','v','s','d','a']` maps one-to-one onto the 9 openers at main.rs:248-256 (`open_profile_form`…`open_safe_harbor_attest_flow`). Each iteration asserts all 9 flow Options `is_none()` (profile_form, classify_inbound_flow, reclassify_outflow_flow, reclassify_income_flow, set_fmv_flow, void_flow, select_lots_flow, set_donation_details_flow, safe_harbor_attest_flow) AND status contains "failed attest save". This is strictly stronger than the old 3-opener/1-flow-each version. `kat_e2e_attest_errlatch_chmod` passes (genuinely ran the failure path, not root-skipped).

3. **[SAFE-M1] dead code removed — RESOLVED, behavior-preserving.** The Enter arm (main.rs:2544-2553) now extracts only `(disposal_event, picks, pick_count, total_sat)`; the dropped `disposal_date`/`disposal_kind`/`principal_sat` are referenced nowhere else in the arm (I read the whole arm, 2541-2605). Payload still built from `disposal_event`/`picks`; status still derived from `disposal_event`/`decision_id`/`pick_count`/`total_sat`. The `let _ = (...)` is gone. The "no lots" branch (2696-2707) sets the global status and `return`s (stays on List); the empty `if let SelectLotsStep::List` and `let _ = flow.step` are gone. `cargo clippy -p btctax-tui-edit --all-targets` is clean — no unused-var/unreachable warnings.

4. **[SAFE-N1 nit] declined — rationale holds.** `parse_date_arg` (eventref.rs:80-83) maps parse errors to `CliError::Usage(format!("bad date {s:?}: {e}"))`, and `CliError::Usage` Displays as `"usage: {0}"` (lib.rs:38-39) — reusing it would leak `"usage: "` into a TUI field error. The inline form.rs validator (1103-1104) uses the byte-identical `format_description!("[year]-[month]-[day]")` and `format!("bad date {t:?}: {e}")` without the prefix, pinned by KAT-V-DD-3 (`err.contains("bad date")`, form.rs:2133-2134). Declination is sound.

5. **[I2] FOLLOWUPS — RESOLVED, all 10 present.** The new chunk-3 section records all 7 spec-Task-4 items — SelfTransfer under-inclusion (#1), lot-display-at-date (#2), safe-harbor-allocate deferral (#4), WB-I4(a) (#5), FIELD_CAP=64 (#6), void-list effective-allocation pre-filter (#7), session-dirty latch (#9) — plus the 3 review-surfaced items: ENG-m1 pre-2025 Universal-pool wallet (embedded in #2), ENG-m2 shortfall (#3), SAFE-M2 pre-existing void-status quit-first (#8).

### Cross-checks
- **No pre-existing assertion deleted or weakened** beyond the two intended changes. Every removed `assert*` line (form.rs) belongs to the deleted coverage-theatre test; the ERRLATCH consolidation replaces 3 blocks with a strictly-stronger 9-iteration loop.
- **No new compile/clippy risk:** clippy clean on the restored tree; no unused imports, unused vars, or unreachable code.

The fold resolves both round-1 Importants without introducing drift. Recommend GREEN.

---

## Final disposition — GREEN (0C / 0I)

Whole-branch review complete: round 1 (3-lens panel) 0C/2I → fold → round 2 (independent
verification) 0C/0I. Full validation suite green (868/0 tests, 0 clippy, fmt/PII/MSRV clean).
Chunk 3 is ready to ship.
