# Select-lots residue burndown — independent review r2 — GREEN

_Fable, independent. Round-1 NEW-1 (untested NF-2 guard) and NEW-2 (boolean masking) both resolved; the
set-delta guard is now genuinely mutation-held — seven mutants of the condition/capture enumerated and run,
all enumerated condition mutants die. One new Minor (capture-side kind-filter mutant survives; corner-case,
status-line-only) + 1 Nit._

---

VERDICT: GREEN

NEW-1: RESOLVED — every enumerated mutant of the arm condition is killed by a named test (details below); both of r1's surviving mutants now die.
NEW-2: RESOLVED — fixed in code by the set-delta; logic-verified against resolve.rs (no dedicated two-allocation test — acceptable for a Minor, stated explicitly below).

## Verdicts on the six points

**1. NEW-1 — the guard is now genuinely mutation-held.** Condition under test (`crates/btctax-tui-edit/src/main.rs:4962-4967`): `any(kind == SafeHarborUnconservable && event.as_ref().is_some_and(|id| !unconservable_before.contains(id)))`. I ran seven mutation experiments myself (cp-backup + restore each time, `cmp` byte-identical verified; no `git checkout` used):

| Mutant | Change | Result | Killed by |
|---|---|---|---|
| M1 polarity | drop `!` | KILLED | `kat_pre2025_pathb_multilot_…` (break not surfaced) AND `kat_pre2025_pathb_preexisting_break_…` (false "BUT it broke") |
| M2 set-delta dropped | condition → kind only (r1 Mutant A) | KILLED | `kat_pre2025_pathb_preexisting_break_…` — panics on exactly r1's failure text ("BUT it broke … Void this selection … to restore it" blamed on a clean pick) |
| M3 kind conjunct dropped | condition → set-delta over ALL blockers | KILLED | `kat_pre2025_pathb_preexisting_break_…` (its world carries another blocker id outside the before-set → widened `any` false-fires; full tui-edit suite run: 349/350) |
| M4 arm-off | condition → `false` | KILLED | `kat_pre2025_pathb_multilot_…` |
| M5 arm-always | condition → `true` | KILLED | tightened ST-SEL (`kat_selftransfer_selectable`) AND preexisting KAT |
| M6 r1 Mutant B shape | condition → `unconservable_before.is_empty()` | KILLED | tightened ST-SEL — exactly the fix r1 prescribed, working |
| M7 capture emptied | `unconservable_before` → `Default::default()` | KILLED | `kat_pre2025_pathb_preexisting_break_…` |

This independently reproduces and extends the author's two claimed experiments. Every meaningful mutant of the condition itself dies. One capture-side extension mutant survives — see Minor NEW-3, which I judge below the untested-guard blocking bar.

**2. NEW-2 — resolved in code.** The set-delta handles the two-allocation masking exactly: broken-A + effective-B is a legal state (resolve.rs:1238-1245 — a non-conserving allocation `continue`s before the `effective` push, so only B counts toward the single-effective Path-B arm at resolve.rs:1293-1313). Before-set = {A}; a pick that newly breaks B emits `Unconservable(Some(B))`, B ∉ before-set → arm fires. No dedicated two-allocation test exists — the fix is argued-from-code here, and I accept that for a Minor: the logic is a straight set-membership check whose polarity, kind-scoping, and delta semantics are each independently mutation-held (M1/M3/M2). The Minor NEW-3 kill-recipe below would incidentally add adjacent-blocker coverage of the same masking class.

**3. Capture correctness — verified.** `unconservable_before` is built at main.rs:3981-3992 from `app.snapshot` strictly before `persist_select_lots` (main.rs:4002) and before `app.snapshot = Some(snap)` (main.rs:4021). `SafeHarborUnconservable` has exactly one construction site in the workspace (`crates/btctax-core/src/project/resolve.rs:1239-1243`), always `event: Some(d.id.clone())` = the allocation decision id — so the set keys correctly and the `is_some_and` None false-negative is structurally unreachable. The `unwrap_or_default()` snapshot-None fallback is unreachable in practice (`open_select_lots_flow` early-returns without a snapshot, main.rs:4670-4673; re-projection failure never clears it, main.rs:4024-4028), and even if reached it errs toward false-FIRING the warning, never masking.

**4. The pre-existing-break KAT is sound and discriminating.** (main.rs:22294-…) The wrong-Σ-basis attestation ($29999 vs the true HIFO residue 500k/$15000 from the 1M/$30000 lot minus the 500k disposal) genuinely produces a baseline `SafeHarborUnconservable` — and the KAT does not take this on faith: its baseline assert requires the blocker kind AND `event == Some(&alloc_id)`. The pick (500k from a lot with 1M at-disposal remaining) is feasible and residue-identical to HIFO's own draw, so no new break. The test cannot pass through the wrong arm: arm-1 ("Saved, but DecisionConflict…"), arm-2 ("LotSelection saved but invalid…") both lack "Lot selection recorded", which the KAT's second assert requires; arm-3 trips the `!contains("BUT it broke")` assert. It exercises arm-4 or fails. M2's kill (the panic message shows the full false-attribution string) confirms it discriminates exactly r1's failure scenario.

**5. Tightened ST-SEL assert — genuine and safe.** "contemporaneity" appears in exactly one product string in the binary (arm-4, main.rs:4979); "BUT it broke" in exactly one (arm-3, main.rs:4969). So `contains("Lot selection recorded") && contains("contemporaneity")` plus `!contains("BUT it broke")` (main.rs:21737-21744) pins arm-4 uniquely among all four arms. Empirically it kills both always-fire shapes (M5, M6). Brittleness direction is safe: rewording arm-4 fails the test loudly rather than letting it pass vacuously.

**6. No regressions.** Working-tree diff verified order-insensitively identical to the reviewed scratchpad diff (plus the FOLLOWUPS.md rewrite, which I read — accurate, including the NF-1 ActualPosition scope note in both FOLLOWUPS and the arm-3 doc-comment, main.rs:4922-4926). The M-1 message now renders `remaining_sat` once (`form.rs:1332`) and still satisfies `kat_v_sl_4` (`contains("80000") && contains("30000")` both hold in "picked 80000 sat on a lot with only 30000 sat available"). Full gate re-run twice on the reviewed tree, including once after byte-identical restore: `make check` exit 0 — 2076/2076 nextest + clippy `-D warnings` clean (noting the standing caveat: this is not the CI-only fmt/msrv/pii-scan/net-isolation jobs).

## New findings

**Minor NEW-3 — the capture-side kind filter is not mutation-held (over-broad before-set survives the full suite).** Dropping `.filter(|b| b.kind == BlockerKind::SafeHarborUnconservable)` from the capture (main.rs:3988) — so the before-set collects ALL pre-save blocker ids — passes the entire workspace suite (2076/2076, verified empirically). Reachable failure: an allocation id already carrying a DIFFERENT blocker kind pre-save whose conservation THIS save newly breaks gets masked. The sharpest instance: a timebarred allocation (`SafeHarborTimebar` is Advisory, state.rs:96-102 — the year is NOT yet gated) that still conserves; a specific-ID pick breaks its conservation → conservation is checked before the timebar (resolve.rs:1237-1252), so a new HARD `SafeHarborUnconservable` fires and the year newly gates — and the mutant shows the clean arm-4 message because the alloc id sat in the widened set via the Timebar. This is outside the enumerated condition-mutant set, the shipped code is correct, harm is status-line-only (the Hard blocker still renders loudly in Compliance; never a silent wrong number) — the same harm class r1 rated Minor for NEW-2, so it does not re-block the gate. Cheap kill: one KAT seeding a timebarred-but-conserving allocation (`timely_allocation_attested: false`, made past the ActualPosition bar), pick flips the residue, assert the status DOES carry "BUT it broke" — kills this mutant and adds the adjacent-blocker variant of the NEW-2 masking class in one test. File with an owner.

**Nit** — the FOLLOWUPS SL-r2-a done-record credits the mutation-holding to "TWO KATs" but omits the third leg it also shipped (the tightened ST-SEL assert, which is what actually kills the always-fire/Mutant-B shape). One clause would make the record complete.

## Bottom line

The fold does exactly what it claims and what r1 prescribed: the boolean became an id-level set-delta (fixing NEW-2's masking in code), the pre-existing-break KAT kills r1's Mutant A on its precise failure text, and the tightened ST-SEL assert kills the Mutant-B shape. Seven independent mutation experiments confirm the guard is held from every enumerated direction; capture timing, blocker-id keying, and arm discrimination all check out against source. The one surviving capture-side mutant is a bounded, Compliance-visible corner filed as Minor NEW-3 with a one-test kill recipe. 0 Critical / 0 Important — GREEN.
