# P-C PHASE GATE — US-federal-tax-correctness lens (Opus) — persisted verbatim before folding

Scope: Tasks 8–9 + `badfae4` Minor-burndown. Base `8074a3e` → head `badfae4` (4 commits).
Reviewer: Opus, tax lens, 51 tool uses.

## Verdict

**NOT GREEN — 0 Critical / 3 Important / 5 Minor / 3 Nit**

The phase's *filed-number* spine is sound: every dollar the TUI shows or writes comes from a shipped primitive (`filed_basis_for`, `window_reference`, `clamped_promote_year_saving`, `compute_tax_year`), both writes go through `plan_*`/`apply_*` behind the KAT-G1-confined `persist_declare_tranche`/`persist_promote_tranche`, the declare confirm re-plans **fresh** (no stale plan), all shadows force `pseudo_reconcile=false`, and the recorded `Acknowledgment` is proven `Eq`-identical to the CLI's. The `badfae4` consent echo **is** display-only (verified: `chokepoint/mod.rs` is untouched by the diff; `render_consent` still builds only from `advisory_lines`/`terms`/`gift_only_years`/`post_consent_note`; the echo lives in `render_promote_flow`). What blocks the gate is that **two engine gates the SPEC requires the driver to only *collect* for are instead answered by the driver itself**, plus one filer-facing statement about a gate that is false on the wizard's own common path.

---

## Important (block P-C)

### I-1 — BG-D6 ack residency violated: the TUI validates the phrase, then hands `apply_promote` the **constant**
`crates/btctax-tui-edit/src/main.rs:4362-4381` (driver-side exact compare) and `:4417-4422`:
```rust
crate::edit::persist::persist_promote_tranche(session, *plan, Some(btctax_cli::PROMOTE_ACK_PHRASE), now)
```
The filer's typed buffer is never passed; worse, `promote_flow_confirm` `mem::replace`s the `Consent` step (`:4396`) and discards the buffer immediately before the write.

SPEC DFW-D2 ★ *BG-D6 ack residency*: "**Drivers only collect the phrase — they NEVER validate it** — so BG-D6's enforcement point stays single-sourced in the chokepoint." SPEC §5 even names the mutation that must red: "*a driver cannot append without a correct phrase reaching `apply`*".

Failure scenario: `require_promote_ack` (`chokepoint/mod.rs:301-312`) can **never** refuse on the TUI path — it always receives the literal correct phrase. Any future edit that reaches `promote_flow_confirm` without the pre-check (a key-dispatch change, a new "confirm" entry, a refactor dropping `if typed != …`) appends a `PromoteTranche` — a >$0 estimated-basis filing position plus a mandatory Form 8275 — with **no filer acknowledgment at all**, and the recorded §6664(c) `Acknowledgment.phrase` still reads as though the filer typed it. `wrong_ack_phrase_refuses_fail_closed_and_records_nothing` (promote_flow.rs:~486) proves the *harness* is fail-closed, not the production driver; the e2e test only pins the pre-check. This is the "held by convention, not by construction" class.

**Fix:** capture `typed` in `handle_promote_flow_consent_key`, thread it (`promote_flow_confirm(app, typed)`) into `persist_promote_tranche(session, *plan, Some(&typed), now)`; keep the pre-check as the documented UX nicety. Add the SPEC-named mutation KAT: delete the driver pre-check → the e2e wrong-phrase step must still record nothing.

### I-2 — BG-D5 provenance is answered *for* the filer: `ProvenanceKind::Purchase` hardcoded, the closed enumeration never asked
`crates/btctax-tui-edit/src/edit/promote_flow.rs:102` (`review()` passes `btctax_cli::ProvenanceKind::Purchase`).

The module's justification — "this flow only ever targets a DFW-D8 '$0, no acquisition record' declared tranche, which **by construction has no OTHER real acquisition provenance to attest to**" — is unsound. "No acquisition record" ≠ "purchased." The flow's own default era preset is **2009–2011** (the CPU-mining era); a no-records 2009–2013 holder is at least as likely to hold **mined / forked / airdropped** coins, which carry a documented FMV-at-receipt or carryover basis (Notice 2014-21; Rev. Rul. 2019-24) — exactly why `ProvenanceKind` is a CLOSED enumeration (`cmd/promote.rs:28-36`) and every non-Purchase value is refused and routed (`chokepoint/mod.rs:332-334`).

On the CLI the filer must **select** from that enumeration (`cli.rs:911-914`). In the TUI the question is never asked; the tool supplies the only gate-passing value, so BG-D5's engine gate is structurally unreachable on the wizard surface (`refuse_non_purchase` is dead code on this path). A mining-origin filer can drive the wizard end-to-end and record `provenance_attested: true` + `PROVENANCE_TEXT` asserting purchase, filing a window-min floor + Form 8275 on a position the CLI refuses. This matters doubly because the whole DFW-D5.3 adjudication rests on it — SPEC §3: "**Only the filer's provenance attestation distinguishes them**", and `spec-tax-opus-review-r4.md:60`: "The whole feature already trusts BG-D5 provenance."

Not Critical because `badfae4` did make it **disclosed**: `PROVENANCE_TEXT` renders on the Part II screen and is echoed directly above the ack prompt, and the filer must act affirmatively twice. But disclosure ≠ the affirmative selection the CLI requires, and no design artifact blesses hardcoding (grepped SPEC/PLAN/DESIGN/all 21 review rounds: no provenance-picker decision exists either way).

**Fix (small, no new tax logic):** one explicit step before `review()` using the already-crate-root-re-exported `ProvenanceKind`; on a non-Purchase pick, drive it through `plan_promote` so the shipped `Refusal::Provenance` text is what the filer sees (keeps the gate engine-enforced, DFW-D1). Minimal version: a y/n "were these acquired by PURCHASE (not gift/inheritance/mining/staking/airdrop/fork)?" whose "n" arm renders `refuse_non_purchase`'s shipped message.

### I-3 — the declare flow tells the filer a declare "will be refused" when it will not
`crates/btctax-tui-edit/src/edit/declare_flow.rs:241-249`:
> "Note: an in-force safe-harbor allocation **or a pre-2025 tranche** is present — **a pre-2025 declare here will be refused** (the two are mutually exclusive)."

`journey_view.safe_harbor_blocked` is a **symmetric** mutual-exclusion flag: `in_force_allocation_exists(events) || pre2025_tranche_exists(events)` (`defensive/mod.rs:683`). The actual declare gate is **directional**: `guard_tranche_vs_allocation` refuses only when `window_end < TRANSITION_DATE && in_force_allocation_exists(events)` (`cmd/tranche.rs:64-75`). The `||` disjunct exists for the *other* direction (allocation-vs-tranche).

Failure scenario — the wizard's own majority path: the default preset yields `window_end = 2011-12-31`, so **after the first declare** `pre2025_tranche_exists` is true (`tranche_guard.rs:55-61`). Every subsequent declare-flow open in a vault with no allocation at all renders "a pre-2025 declare here will be refused" while `plan_declare` accepts it. A filer covering their second shortfall reads a false blocker and abandons a correct, available action; the uncovered `UncoveredDisposal` stays a Hard blocker and the year stays not-computable. The KATs (`declare_flow.rs:361-387`) only assert the substring "safe-harbor" is present/absent — nothing pins the claim's accuracy.

**Fix:** drop the directional clause (the dashboard's own P-B note is already correctly worded neutrally), or thread `in_force_allocation_exists` alone for the declare-side note. Best: surface the flow's already-built `clearance()`/`plan_declare` refusal instead of predicting one (see N-1).

---

## Minor

- **M-1 — an inverted window (`window_start > window_end`) is reachable and misdiagnosed.** `declare_flow.rs:85-92` (`cycle_preset` sets `window_start = preset.start` unconditionally while clamping `window_end` to the before-op day) and `:121-130` (`nudge_window_end` has an **upper** clamp only). Shortfall on 2018-06-01, Tab to `Y2021To2024` → start 2021-01-01, end 2018-05-31. `window_reference` returns `None` for `start > end` (`conservative.rs:234-235`) → the readout prints "floor: NOT COMPUTABLE — **no price data covers this window at all**", blaming missing data for an incoherent window, right across the 2018–2023 audience years. Filing-safe (`plan_declare` refuses at confirm, `chokepoint/mod.rs:531-535`) — but this is exactly the state the phase already burned down for `nudge_window_start` (FOLLOWUPS T8-review Minor-2), left half-fixed. **Fix:** `self.window_start = start.min(self.window_end)` in `cycle_preset`; floor `nudge_window_end` at `self.window_start`.
- **M-2 — DFW-D8's spec-named confirm-note for `sat > short_sat` is missing.** `nudge_sat` (`declare_flow.rs:136-140`) has no upper bound and the Confirm copy never mentions the excess. SPEC DFW-D8: "the excess is the out-of-scope manual-holdings shape … **a confirm-note suffices**." Nudging 10M → 40M sat files nothing wrong at $0, but leaves a $0-basis phantom lot that, if later promoted, files a >$0 floor on sat the shortfall never needed. Backstopped downstream by `Advisory::OverCovered`, so Minor. **Fix:** one Confirm-arm line when `state.sat > state.shortfall.short_sat`.
- **M-3 — the displacement caveat has a hole for a correctly-sized cover.** `defensive/mod.rs:659-688`: `WouldDisplaceIfPromoted` fires only in the `else if !promoted` arm (`covered_sat == 0`). When `covered_sat > 0` **and** `t.sat == covered_sat`, neither `OverCovered` nor `WouldDisplaceIfPromoted` fires — yet a HIFO reorder across multi-year disposals still shifts gain between years, so that row's per-year `ComputedTax`/`Uncomputable` delta is a reorder artifact shown as an unqualified saving. **Fix:** fire on `!promoted && displaces_documented_basis(..)`, suppressing only where `OverCovered` already carries its own displacement copy.
- **M-4 — the declare flow's on-demand tax-Δ carries no displacement caveat at all.** `declare_flow.rs:293-307` prints bare `$delta` / `gain-Δ $X` figures; the dashboard row's equivalent number is caveated by `WouldDisplaceIfPromoted`. Filing-safe (the preview files nothing; the real promote goes through the dashboard + consent screen), but it is the same class the phase just fixed one layer over, and `declare_preview_saving` already builds both folds, so the check is nearly free.
- **M-5 — the default preset seeds a *taxpayer-favorable* holding date.** `DeclareFlowState::new` (`declare_flow.rs:60-79`) seeds `ALL_PRESETS[0]` (2009-01-03..2011-12-31). Because `window_end` **is** the lot's acquisition date (`resolve.rs:1310`), the tool's default answer to "when did you acquire these?" makes nearly every disposal **long-term** at the preferential rate — while the doc justifies the oldest-first choice purely on the basis axis ("wider window → lower floor, the conservative direction"). Not silent (window + "(long-term at the short op's date)" render on both Edit and Confirm) and there is shipped precedent (`conventions::long_term_default_acquired`, the user-mandated self-transfer policy), so Minor — but this is a *tax* dimension of the era-table product decision and should be decided explicitly, not inherited from "oldest preset first".

## Nit

- **N-1 —** `DeclareFlowState::clearance` (`declare_flow.rs:168-190`) has **no non-test caller**, yet its doc claims the refusal "is surfaced **live** rather than discovered only at a final Enter (DFW-D5)". `render_declare_flow` never calls it; the real (and correct) gate is `declare_flow_confirm`'s fresh `plan_declare` (`main.rs:4124`). Wire it into the readout (which would also fix I-3 properly) or correct the doc.
- **N-2 —** the `tax_delta: None` arm renders "stale — recompute" even on first open, where nothing is stale (`declare_flow.rs:294`).
- **N-3 —** `render_promote_flow` indents the Part II buffer two spaces; the recorded `part_ii_narrative` is unindented. Display-only, no substantive divergence.

---

## Lens invariants — verified

- **No new tax logic / no TUI-minted number.** ✓ `declare_preview_saving` lives in core and mirrors `clamped_saving_for` exactly (`defensive/mod.rs:293-366` vs the new fn), threading a real `profile`; its synthetic-tranche timestamp is provably irrelevant (a tranche's `Eff.utc = window_end.midnight()`, `src_priority = u8::MAX`, `resolve.rs:1310-1314`). The TUI renders `cf.filed_basis` and `render_consent` output; it computes nothing.
- **§6664(c) artifact parity.** ✓ `tui_promote_records_an_acknowledgment_eq_identical_to_the_cli_driver` drives both **full** driver paths and asserts `Eq` on the recorded `Acknowledgment` incl. non-empty `shown_terms` + equal `filed_basis`. The plan is computed once at `review` and the *same* plan reaches `apply_promote` — what was SHOWN is what is RECORDED. Esc→PartII discards the plan and Tab recomputes, so an edited narrative can never be shown against a stale plan.
- **`badfae4` echo is display-only.** ✓ `chokepoint/mod.rs` unmodified in the range; `PROVENANCE_TEXT` appears nowhere in `render_consent`/`render_consent_terms`; the echo is a line in `render_promote_flow`'s own `Consent` arm.
- **BG-D7 / BG-D6 fail-closed / DFW-D12.** ✓ Empty and multiline-whitespace Part II refused at `plan_promote`; ack enforced inside `apply_promote` before `would_conflict`/append (subject to I-1); one tranche at a time, double-guarded (`handle_defensive_dashboard_key` `p` arm + `open_promote_flow` re-check `status == DeclaredZero`), with `would_conflict` as the engine backstop.
- **DFW-D5/D8 prefill.** ✓ `window_end` strictly before the short op (`before_op_date`; `nudge_window_end` clamps at that boundary; proven unbreakable even across tz skew since `(tax_date−1)T00:00Z < utc_instant` always), wallet = the short op's source pool, sat floored at 1.
- **DFW-D6.** ✓ Entry gate refuses on `pseudo_active()` (+ new residue-latch guard); `plan_declare(Some)`, `plan_promote`, and `declare_preview_saving` each force `pseudo_reconcile=false` on their own copy regardless of the caller's cfg; a declare/promote cannot turn pseudo on (`pseudo_synthetic_count` comes only from unresolved inbound/conflict/FMV shapes), so the post-write dashboard refresh stays coherent.
- **DFW-D10.** ✓ `snap.profiles` is the **resolved** per-year map with the CLI $0 placeholder excluded (`unlock.rs:179-211`), so `ComputedTax` is genuinely reachable; `cycle_preset`/`nudge_window_start`/`nudge_window_end`/`nudge_sat` all blank `tax_delta`; no bare `$X` for a non-computing year. Displacement caveat gaps are M-3/M-4.
- **P-C follow-up burndown.** ✓ All five `[done]` P-C items verify against the diff (nudge_window_start clamp + KAT; consent echo + adjacency KAT; phantom-wallet verbatim/silent subprocess KATs; three `Refusal::Target` parity KATs asserting byte-identical stderr through the same `From<Refusal>`; T2-M1 closed YAGNI-**confirmed** by grep, not presumed).

## The era-table `[open]` follow-up — **NOT tax-blocking** for closing P-C

No. The provisional table cannot reach a filed number un-validated, on four independent grounds:

1. Presets are seeds only. `plan_declare` re-validates the *chosen* window at confirm (`sat > 0`, `ws <= we`, `guard_tranche_vs_allocation`, and the DFW-D5.2 clearance re-projection with pseudo forced off — `chokepoint/mod.rs:512-594`), and the declare files **$0** regardless.
2. The filed floor is never a preset's; it is `filed_basis_for` over the **stored** window at promote time, requiring `Coverage::Full` (`chokepoint/mod.rs:346-353`) — a preset window with no/partial price data simply cannot be promoted, and the flow says so live.
3. `defensive_era.rs` KATs pin the structural properties that could bite: total function over every variant, well-formed and non-overlapping windows, strictly increasing, and **every bucket ends before `TRANSITION_DATE`** — so no preset can silently mint a post-2025 window or an inverted one on its own (M-1 is the *interaction* with the before-op clamp, which is a code fix, not a table fix).
4. The buckets are round calendar years with no named-exchange/historical-event claims, so there is no factual assertion that can be wrong.

It is a genuine **product/copy** decision, correctly filed as P-C-owned. Three things the owner should decide **with** it (they are tax-relevant, not just copy): **(a)** which preset is the *default* — that decides the default holding-period character (M-5); **(b)** whatever the final buckets, cycling to a preset later than the short op must not leave an inverted window (M-1); **(c)** the table has no ≥2025 bucket, so a post-2024 shortfall's window is only reachable by ±1-day nudges — see the already-filed free-text-entry follow-up. None of (a)-(c) changes the correctness argument above; all three are Minors that ride the same decision.
