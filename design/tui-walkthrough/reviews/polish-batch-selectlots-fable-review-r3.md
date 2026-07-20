# App-side polish batch (walkthrough residue) — select-lots fold review r3 — GREEN (0C/0I)

_Fable, independent. Verifies the r2 NEW-1 fold (unify both transition arms onto `available_lots_before`)
and scrutinizes the tax-sensitive pre-2025 Path-B behavior change. 0 Critical / 0 Important; suite green
(make check 2072/2072 + clippy). Two Minors (fold into SL-r2-a) + two Nits._

---

VERDICT: GREEN

NEW-1: RESOLVED

_Fable, independent, r3. Verified against the working tree (the supplied diff matches `git diff` except it omits the FOLLOWUPS.md hunk, which I read from the tree). Re-ran the 8 targeted KATs and the full fast gate: `make check` 2072/2072 + clippy clean._

## Q1 — Is the pre-2025 Path-B offer of the original pre-seed lot feasible? CONFIRMED, not doomed.

Trace, real fold: `fold` sorts canonical then stably partitions pre-2025 first (fold.rs:381-387); the §7.4 seed fires only at the first `>= TRANSITION_DATE` event (fold.rs:420-424). D @2024-08 folds in the pre-partition with `seeded == false`, so at `fold_event(D)` the Universal pool holds the original 2024-06 acquire at 1,000,000 sat. D's pick is validated by `selection_feasible` against `pools.get(pool_key(2024-08, w)) = Universal` (pools.rs:15-21, 107-153) — a 500K pick of the original lot passes (500K ≤ 1M, lot present).

Offered set == accepted set: `pools_before` mirrors the fold's exact ordering (fold.rs:456-458) with the same `FoldCtx` including selections (fold.rs:463-467), fires the seed only if a ≥2025 event precedes the target (fold.rs:471-474), and stops BEFORE folding D (fold.rs:475-477) — for a pre-2025 D the seed never fires, so `SafeHarborAllocated` seed lots are absent by construction. `available_lots_before` filters that identical `PoolSet` by `pool_key(date, &l.wallet) == pool_key(item.date, wallet)` (optimize.rs:318-327) — the same key the fold's draw uses. `item.date`/`item.wallet` come from `d.disposed_at` and the raw event's wallet (main.rs:4744-4748), matching the fold's `eff.date()`/`eff.wallet`. The rewritten test proves it end-to-end: 500K pick → save → re-project → no `LotSelectionInvalid`, no `SafeHarborUnconservable` (main.rs:22029-22047, passes).

## Q2 — Test rewrite faithful? YES — the protective property is preserved and STRENGTHENED.

The old property ("a SafeHarborAllocated seed lot must never be offered for a pre-2025 disposal") is still asserted, twice over: `rows.len() == 1` + `rows[0].lot_id.origin_event_id == orig_acq` (main.rs:22013-22022) — seed lots carry `origin_event_id = the allocation decision id` (resolve.rs:1260-1263), so any offered seed lot fails both asserts. What changed is the OTHER half: the old test pinned "offer NOTHING", which was itself the over-strict shadow of the residue defect — the engine accepts a pick of the original lot for D (Q1), so denying it was a false limitation, not a protection. The rewrite adds the at-disposal amount assert (1,000,000, main.rs:22023-22026) and the end-to-end clean acceptance — strictly stronger than the old empty-offer assert. Not flip-to-match-code; a legitimate correction with more discriminating power.

## Q3 — Multi-lot Path-B conservation: the engine SURFACES basis drift; SL-r2-a is accurate on its main claim, with two refinements (Minor, below).

The conservation check is selection-aware: `universal_snapshot` receives `&selections` (resolve.rs:1227-1234) and folds the pre-2025 timeline through the SAME `fold_event` with them in `FoldCtx` (transition.rs:38, 55-61). A non-FIFO pre-2025 pick that changes the residue's Σ basis makes `alloc_basis != snap.basis` (resolve.rs:1235-1237) → `SafeHarborUnconservable` on the allocation id (resolve.rs:1238-1244), which is HARD (state.rs:89) → the allocation goes inert (Path A) AND the Hard blocker gates the tax year (`TaxYearNotComputable`, state.rs:46-49). No silent wrong number for basis drift; recoverable by voiding the LotSelection (selection dropped → FIFO residue → conserves again). SL-r2-a's leave-or-suppress framing is correct and it is NOT a blocking hole needing suppression now. Two accuracy refinements to its text:

- The check is TOTALS-only (Σsat, Σbasis). An equal-totals composition drift — two pre-2025 lots with identical per-sat basis but different `acquired_at`, specific-ID'd contrary to FIFO — passes conservation while the attestation's per-lot `acquired_at` now misdescribes the true residue (potential LT/ST character skew on later seed-lot disposals). This is NOT surfaced. However: it is pre-existing engine semantics (resolve.rs untouched by this diff; the CLI has always accepted pre-2025 selections under Path B — `honoring`/retain at resolve.rs:1127-1179 has no transition-mode gate), and the engine never verifies per-lot composition even with ZERO selections (the attestation's per-lot dates are the user's claim by design; "allocation totals != Universal remainder" is the spec'd semantic, resolve.rs:1242). Scope note for SL-r2-a, not a blocker.
- `derive_select_lots_status` has no arm for this outcome: a save whose pick fires `SafeHarborUnconservable` hits the clean Arm 3 "Lot selection recorded …" (main.rs:4894-4922) while the year just went NotComputable. Loud in Compliance, but the immediate status is misleadingly green. Belongs in SL-r2-a's scope (add a 4th arm if the leave option is chosen).

## Q4 — NEW-1 genuinely resolved. CONFIRMED.

Both classes die by construction: `pools_before` stops before D in the fold's own order, so (a) a lot acquired after D and (b) a later relocation's fresh-lot_id fragment (`bump_split`) simply do not exist in the returned `PoolSet` — and the original pre-relocation lot is offered at its at-D amount, which is exactly what the real fold validates at D. No date-conditional code remains in the TUI (single `match` arm, main.rs:4102-4117); the `TRANSITION_DATE` import is gone. The new KAT discriminates: on the old residue arm, final state is {X: 500K, Y: 300K} → `rows.len() == 1` fails, `remaining_sat == 1_000_000` fails, and the no-Y assert fails — three independent kills for a revert-to-residue mutation (main.rs:18707-18762 asserts; seed at 18611-18698). Ran: passes. The primitive stays held by the 5 `available_lots_before_*` core KATs (ran: 5/5). Residual nit: no TUI-level pre-2025 later-relocated-fragment KAT (the class is covered by construction + the core canonical-order KAT) — nit only.

## Q5 — Regressions from deleting the pre-2025 arm? NONE found.

- Cross-wallet offering preserved: for a pre-2025 date, `pool_key(date, &l.wallet) == Universal` for EVERY wallet (pools.rs:15-21), so the filter (optimize.rs:324) passes the whole pre-boundary Universal pool cross-wallet. `kat_pre2025_crosswallet_lots` (main.rs:21796-21901) still asserts River + Kraken offered, post-2025 lot NOT offered (main.rs:21858-21869), plus a clean cross-wallet 500K pick e2e — ran: passes. That test IS the XW test (the "XW" asserts live inside it).
- Seed-lot exclusion preserved by construction (Q1); explicitly re-asserted in the rewritten Path-B KAT.
- Dropped-behavior audit: (i) the old arm needed only `snap`; the new arm needs a held `Session` — Browse always holds one (r2 N-1), and the `_ => Vec::new()` arm degrades to "No lots available" rather than panicking; (ii) a wallet-less item now gets no offer — but the old cross-wallet residue offer for such an item was membership-wrong anyway, and wallet-less disposals are pathological/pre-filtered; (iii) amounts changed from drained-residue to at-disposal — that is the fix, not a regression. Bonus: r2 N-2's false "(no method-order fallback)" comment died with the deleted arm; the new comment (main.rs:4088-4100) makes no false claim.

## New findings

- **Minor NF-1 (fold into SL-r2-a):** SL-r2-a's safety claim is overbroad as written — the conservation guard surfaces Σ-basis drift but is blind to equal-totals COMPOSITION drift (acquired_at character skew), per Q3. Pre-existing, CLI-reachable, attestation-domain; amend the follow-up's text and make the planned multi-lot KAT cover the surfaced (basis-drift → blocker) case, with the composition corner documented as out of engine scope or given its own decision.
- **Minor NF-2 (fold into SL-r2-a):** `derive_select_lots_status` reports the clean Arm-3 success message when the save fires `SafeHarborUnconservable` on the allocation (main.rs:4894-4922) — the newly-reachable TUI path lands a Hard, year-gating blocker behind a green status line. Compliance shows it; the status should too (4th arm), on SL-r2-a's schedule.
- **Nit:** optimize.rs doc-comment merge is disordered — the new summary paragraph (optimize.rs:266-271) was appended AFTER the historical Task-3 detail (255-265) instead of leading it.
- **Nit (carried from r2 I3):** the eventual commit message should state the correction of e59768c's "provably never over-offers" overclaim; the work is still uncommitted, so this lands at commit time.

Bottom line: NEW-1 is genuinely fixed by unification onto the validator's own pool primitive; the Path-B behavior change is sound — the old "offer nothing" was the defect's over-strict shadow, the new offer is exactly the engine-accepted set, the protective seed-lot property is preserved with stronger asserts, and the multi-lot conservation hazard is engine-surfaced (Hard, gating) for the basis-drift case with the residual composition corner correctly parked as a scoped Minor. Suite green (2072/2072 + clippy), 0 Critical / 0 Important. GREEN.
