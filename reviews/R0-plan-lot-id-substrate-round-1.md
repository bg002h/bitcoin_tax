# R0 architect review — IMPLEMENTATION_PLAN_lot_id_substrate.md (round 1)

**Artifact:** `design/IMPLEMENTATION_PLAN_lot_id_substrate.md` (Sub-project A — lot-identification substrate)
**Contract:** `design/SPEC_lot_optimization_program.md` (R0-GREEN 2026-06-29): Sub-project A + Legal grounding + Cross-cutting.
**Reviewer role:** independent architect (author ≠ reviewer). Source re-grounded against CURRENT tree at review time (2026-06-29).
**Gate:** must reach 0 Critical / 0 Important before implementation.

## Verdict

**NOT GREEN. 1 Critical, 0 Important, 4 Minor, 3 Nit.**

The Critical is exactly the hazard the R0 charge named: the plan's "FIFO-as-an-explicit-total-order" is **not** equivalent to today's insertion-order FIFO, and the divergence is **reachable, gains- and conservation-affecting, and invisible to the entire current test suite.** The plan asserts equivalence with a logically invalid argument ("distinct `acquired_at` ⇒ push-order ≡ total-order"). That inference is false for **relocated** (SelfTransfer) and **Path-B-seeded** lots, which carry an older `acquired_at` than their insertion position. This must be resolved (adopt-the-change-with-tests, or pin insertion-order) before any code.

Everything else in the plan is strong: API signatures match current source, serde-default + `fingerprint=None` backward-compat is correct, the `lot_method` blast radius is fully enumerated (3 test sites all use `..ProjectionConfig::default()`), the six consume sites / four honoring sites are correct, the safe-harbor method-binding (A.7) is sound, and the six resolved ambiguities are well-adjudicated.

---

## CRITICAL

### C1 — FIFO total-order ≠ insertion-order FIFO; diverges on relocated & Path-B-seeded lots, changing gains AND the safe-harbor conservation residue, with zero test coverage

**Where in the plan:** Task 2 step 4 "Regression-watch" (lines ~463); ambiguity #3 (lines ~1696); the `method_order`/`consume_ordered` design (lines ~338–441); the inheritance into the two FIFO-pinned sites (`consume_fifo` delegating to `consume(.., Fifo, None)`).

**The false claim.** The plan states: *"Existing fixtures use distinct `acquired_at`, so push-order ≡ total-order"* (Task 2) and *"Behavior is identical on all current fixtures (distinct `acquired_at`)"* (ambiguity #3). This conflates "no `acquired_at` ties" with "insertion order equals `acquired_at` order." The latter is **false on every path that pushes a lot carrying an `acquired_at` older than lots already in the destination Vec.**

**Current behavior (ground truth).** `PoolSet.pools` is documented "kept in FIFO order … consume from the front" (`pools.rs:23`), and `consume_fifo` walks `idx` from `0` upward (`pools.rs:58-100`). Lots enter via `push_lot`/`new_origin_lot` which **append** (`pools.rs:42,46`). So today's FIFO == **insertion order**.

**Where insertion order ≠ `acquired_at` order (so the plan's sort reorders consumption):**

1. **SelfTransfer relocation** (`fold.rs:536-553,580-583`). A relocated fragment is built with `acquired_at: c.acquired_at` (the *original* acquisition date) and then `push_lot`'d to the **back** of the destination pool. If the destination wallet already holds a directly-acquired lot with a **later** `acquired_at`, then:
   - insertion-order FIFO consumes the pre-existing (newer) lot first;
   - total-order FIFO (`acquired_at` asc, `pools.rs` plan `method_order`) consumes the relocated (older) lot first.

   Concrete, fully in scope of the method-honoring sites (all post-2025, per-wallet):
   ```
   2025-05-01 Acquire 1 BTC in HOT          (lot A, acquired 2025-05-01, basis $X)
   2025-01-01 Acquire 1 BTC in COLD         (lot Z, acquired 2025-01-01, basis $Y)
   2025-06-01 SelfTransfer 1 BTC COLD→HOT   (relocates Z' acquired 2025-01-01, pushed AFTER A in HOT)
   2025-07-01 Dispose 1 BTC from HOT
   ```
   Today: consumes **A** (basis $X). Under the plan: consumes **Z'** (basis $Y). Reported basis, gain, and the ST/LT **term** all change; the *remaining* lot changes. Σsat and Σbasis are still conserved, but the **per-disposal tax result is materially different** — i.e. a wrong-gain / wrong-term flip.

2. **Path-B multi-lot-per-wallet seeding** (`resolve.rs:566-586` → `transition.rs:67-80`). Seed lots are pushed in **allocation-index order** (`seed.iter()`), carrying each `AllocLot.acquired_at`. An allocation may list two lots for the **same wallet** with `acquired_at` in non-ascending index order. Insertion-order FIFO consumes by index; total-order FIFO consumes by `acquired_at`. Same divergence, on the **safe-harbor (Path B)** path. (The existing fixture `tests/transition.rs:733 path_b_seeded_lot_relocation_no_lotid_collision` lists its two `cb()` seed lots `[2024-01-01, 2024-06-01]` — index order *coincides* with `acquired_at` order, so it does **not** detect the change. Reorder those two lines and it would.)

3. **`universal_snapshot` conservation residue** (`transition.rs:25-51`; plan Task 6 reuses this under the allocation's recorded method). The snapshot folds the **pre-2025** timeline through the same `fold_event`. A **pre-2025 SelfTransfer** (TransferLink-confirmed; both legs `< TRANSITION_DATE`) relocates a fragment within the single `PoolKey::Universal` pool — pushing an older `acquired_at` lot to the back. A subsequent pre-2025 partial disposal then consumes a **different** set of lots under total-order vs insertion-order, so the **remaining `Σ usd_basis`** (`snap.basis`) changes. The safe-harbor guard checks `alloc_basis == snap.basis` (`resolve.rs:546-547`). **The conservation reference value shifts** — an allocation that conserves today can read `SafeHarborUnconservable` after the change, or vice-versa. This is the "conservation-critical projection fold" the charge flagged.

**Why the suite will not catch it (it ships silently).** I grepped every self-transfer / TransferLink / Path-B fixture (`kat_tax.rs`, `properties.rs`, `transition.rs`). None places a relocated/seeded **older** lot into a pool that **already holds a newer** lot followed by a distinguishing partial disposal:
- `kat_tax.rs:236` and `golden_kat` relocate into an **empty** destination wallet (single lot — no order ambiguity).
- `properties.rs:177` checks only **Σ basis** (order-invariant — cannot detect a reorder).
- `transition.rs:266`/`:733` relocate into an empty wallet / list seed lots already in `acquired_at` order.

So the plan's "the full suite is the gate" provides **no** protection here, and the Task-2 instruction *"If any tie-break test moves … fold accordingly, do not silence"* will never fire — because **no test moves.** This is the worst case: a silent change to the conservation-critical fold.

**Severity rationale.** Per the charge's own rubric — *"a FIFO-divergence that changes gains/conservation … = Critical"*, and *"If they can diverge, the plan must either keep insertion-order FIFO or prove/encode equivalence; flag as Critical if unproven."* They **can** diverge; equivalence is **not** proven (the proof is fallacious); therefore **Critical.**

**Note on the spec.** SPEC §A.3/M2 defines FIFO as `acquired_at` asc (tie `lot_id`), framed as "total-order **tiebreaks**." The spec shares the plan's blind spot — it treats the total order as *only* a tiebreak rule, not noticing it **reorders** relocated/seeded lots even with distinct dates. Legally the spec's acquisition-date FIFO is the *more correct* rule (a tacked lot retains its acquisition date), so the right resolution is to **adopt** the change deliberately — but that makes it a **behavior change to the conservation-critical fold**, which under STANDARD_WORKFLOW must be acknowledged, KAT-locked, and conservation-re-verified, not asserted as a no-op.

**Required fix (preferred — adopt the spec's acquisition-date FIFO as a deliberate, tested correction):**
1. **Delete the equivalence claims.** Replace Task 2 step 4 "Regression-watch" and ambiguity #3 with an explicit statement: *FIFO is acquisition-date order (`acquired_at` asc, tie `lot_id`); for relocated/seeded lots this **intentionally differs** from the legacy insertion-order walk.* Remove "Behavior is identical on all current fixtures."
2. **Add divergence KATs (RED→GREEN, not "fold accordingly"):**
   - SelfTransfer relocating an **older** lot into a wallet that already holds a **newer** directly-acquired lot, then a partial FIFO `Dispose` — assert the **relocated (older)** lot is consumed first (basis/gain/term reflect it). Add LIFO and HIFO variants over the same fixture.
   - Path-B allocation with **two same-wallet lots in non-`acquired_at` index order**, then a post-2025 partial FIFO `Dispose` — assert oldest-first consumption.
   - `universal_snapshot` residue under a **pre-2025 SelfTransfer** that reorders the Universal Vec — assert `snap.basis` is computed by `acquired_at` order, and that a safe-harbor allocation built against the **acquisition-date-order** residue conserves (no `SafeHarborUnconservable`).
3. **Re-verify every existing safe-harbor / self-transfer fixture** under the new order and update any whose golden values were implicitly insertion-order-dependent (deliberately, with a comment). Confirm `conservation_report` stays balanced on all.
4. **Spec touch-up / material-change re-entry:** flag to the spec owner that §A.3/M2's "tiebreak" framing understates the change; either annotate the spec that FIFO is acquisition-date order for relocated/seeded lots, or run the one-line material-change loop per STANDARD_WORKFLOW §1.

**Acceptable alternative (inferior — zero behavior change):** keep `consume_fifo` as the legacy front-walk for the **two pinned sites** *and* for `LotMethod::Fifo`, and use the sorted total order **only** for LIFO/HIFO. This preserves today's numbers exactly but makes "FIFO" insertion-order (still deterministic, but **not** the acquisition-date FIFO the spec defines), so LIFO/HIFO and FIFO would not share one ordering primitive. If chosen, the plan must say so and reconcile with SPEC §A.3. The preferred path (adopt + test) is cleaner and spec-faithful.

---

## MINOR

### M1 — `disposal_compliance` made-date map is populated from an unordered `&[LedgerEvent]` (latent NFR4)
Plan Task 7 (`sel_made` build, lines ~1345-1351): iterates `events` in slice order with `BTreeMap::insert` (last-writer-wins). `verify` feeds it `load_all`/`load_events_and_project` output, whose order is not canonical. When a disposal has ≥2 (conflicting) `LotSelection`s, the chosen made-date — and thus the emitted `ComplianceStatus` (`Contemporaneous` vs `NonCompliant`) — becomes **load-order dependent**. The disposal is already hard-blocked (`DecisionConflict`), so tax is gated, but `verify` is a primary projection-derived output and NFR4 demands byte-identical results. **Fix:** iterate decisions in `decision_seq` order (mirror `resolve`'s `decisions` sort, `resolve.rs:311-318`) or skip disposals in the duplicate/conflict set. Same discipline as the rest of the plan's total-orders.

### M2 — `Pre2025MethodConflictsAllocation` is pushed inside the effectiveness loop before Path selection
Plan Task 6 (lines ~1219-1224): the conflict blocker is emitted for each candidate as it is `effective.push`'d. In the multiple-effective case (`resolve.rs:602-615` → `DecisionConflict` + Path A), the method-conflict blocker fires for **every** candidate even though **no** Path B governs. Spurious (the year is already hard-blocked), but noisy and slightly misleading. **Fix:** emit the conflict only for the single allocation that actually becomes effective (compute after Path selection, when `effective.len() == 1`).

### M3 — `config --set-forward-method` is a spec deliverable with no task/test
SPEC §A.1 lists `config --set-forward-method <m> [--effective-from <date>]` (appends a `MethodElection`). The plan covers the engine + decision path fully (Task 3) but the CLI alias is only a footnote in §4.1 — no failing test in Task 5's list, no dispatch sketch. Risk of being silently dropped. **Fix:** add it explicitly to Task 5 (a thin `append_decision(MethodElection{…})` wrapper) with a round-trip test, or state in scope why it is deferred.

### M4 — Several Task 7/8 test bodies are prose stubs, not constructed fixtures
Plan Task 7 (`let evs = vec![ /* … */ ];`) and Task 8 (`// … build vault …`) leave the fixtures as comments. §4.2 defends these as "scaffolding," and the asserted *behavior* is specified — acceptable for a plan, but the implementer must build real synthetic events (esp. the broker-2027 / self-custody / made-date-before-sale cases) or the compliance matrix risks being under-tested. **Fix:** at minimum enumerate the exact events each stub must construct, so RED is real before GREEN.

---

## NIT

- **N1 — `parse_lot_id` test coverage gap.** `rsplit_once('#')` / `rsplit_once(':')` are in fact correct even when `source_ref` contains `#` or `:` (the split/sat suffixes are always last; fingerprints are hex; `split_sequence` is digits). But the existing codebase deliberately tests `source_ref` containing `#` (`eventref.rs:115` uses `"in|99|credit|1#0"`), whereas plan Task 5's `parse_lot_id` test only exercises `|`. Add a `#`-in-`source_ref` case to `parse_lot_id`'s round-trip test to lock the rsplit choice.
- **N2 — `Decimal::is_zero()` in `hifo_cmp`.** Plan Task 2 uses `a.usd_basis.is_zero()`. Confirm the trait is in scope (`num_traits::Zero`) or use `== Usd::ZERO` for consistency with `fold.rs` (`carry.gain_basis > Usd::ZERO`). Trivial; clippy/build will catch it.
- **N3 — `inspect::verify` reads config twice.** Plan Task 8 adds `let cli = session.config()?;` while `load_events_and_project()` already returns a `_cfg`. If that third tuple element is the `CliConfig`, reuse it; otherwise harmless. Cosmetic.

---

## Confirmed correct (so the next round can focus on C1)

- **Six consume sites / four honoring.** Exactly matches SPEC §A.3 and `fold.rs` (`:367` Dispose, `:526` SelfTransfer, `:745` GiftOut, `:811` Donate honor; `:232` `consume_fee` and `:483` PendingOut stay FIFO). `select-lots` targeting validation (`honoring_principal`, Task 4) correctly excludes PendingOut/fee.
- **HIFO key.** `usd_basis`-per-sat desc via **cross-multiplication** (no float, NFR5), basis-pending (`usd_basis==0`) last, ties oldest→`lot_id`, `dual_loss_basis` ignored. Matches §A.3/M1 and the KAT set (incl. `hifo_ignores_dual_loss_basis`). Note `method_order` produces a **strict** total order (ties resolve to globally-unique `LotId`), so consumption is order-independent given pool contents — which is *why* C1's reorder is real and deterministic.
- **MethodElection resolve edges.** `effective_from < TRANSITION_DATE || < made-date` → `MethodElectionBackdated` (one family, per §A.1); voided excluded via the existing `voided` set; latest-in-force by `effective_from` tie `decision_seq`; FIFO before any election. All match §A.1/R2-M4.
- **LotSelection edges.** Principal conservation (excl. on-chain `fee_sat`), duplicate→`DecisionConflict` (mirrors `resolve.rs:459-468`), voided excluded, targeting/existence/per-wallet→`LotSelectionInvalid` (hard, with fall-back to method-order so Σsat conserves), fee FIFO from post-selection remainder. The per-wallet constraint is correctly *implicit* in pool partitioning (Universal pre-2025 = no wallet constraint; `PoolKey::Wallet` post-2025 = same-wallet only). Matches §A.4.
- **Backward-compat.** New `EventPayload`/`BlockerKind` variants additive; `SafeHarborAllocation.pre2025_method` `#[serde(default)]` (mirrors `AllocLot` `:150,152`); `persistence::fingerprint` `_ => None` arm (`:96`) covers the new decisions; `append_decision` inserts `None` (`:259`); no event-id/fingerprint change. KATs requested. Verified.
- **`ProjectionConfig` blast radius.** Removing `lot_method` breaks exactly `project/mod.rs:28,35`, `config.rs:13,21,31,143`, `main.rs:227-228`. The three test constructions (`transition.rs:665`, `kat_tax.rs:1002,1464`) all use `..ProjectionConfig::default()` and keep compiling. Fully enumerated by the plan.
- **A.7 safe-harbor binding.** Immutable serde-default `pre2025_method` on the payload (distinct from the existing `AllocMethod`), method-aware `universal_snapshot` consuming under the **recorded** method, `Pre2025MethodConflictsAllocation` firing only on live≠recorded (never `SafeHarborUnconservable`), Path B stays effective (irrevocable allocation pins the method), escape = revert config. No deadlock; clearable. Sound — **subject to C1** (the snapshot residue under a pre-2025 relocation must be re-verified under the new order).
- **Ambiguity #1 adjudication.** Defining the full 4-variant `ComplianceStatus` but having A's classifier emit only `StandingOrder`/`Contemporaneous`/`NonCompliant` (deferring `AttestedRecording` to C's attestation gate) is correct — A's `LotSelection` payload carries no attestation field, so A *cannot* and *should not* manufacture `AttestedRecording`. Keeps A's event shape exactly as §A.2 mandates.
- **Evaluate (A.6).** `evaluate_disposal` reuses the proven clone/append/fold/discard pattern, `--proceeds`-required when `fmv_of` returns `None`, synthetic `EventId::Decision{u64::MAX}` sentinel (decision seqs start at 1), injected selection overrides persisted, side-effect-free. Signatures match `resolve`/`fold`/`fmv_of`/`Op::Dispose`. Sound.

---

## What round 2 must show
0 Critical / 0 Important. Specifically: C1 resolved (equivalence claims removed; divergence KATs added for SelfTransfer relocation, Path-B same-wallet multi-lot, and the `universal_snapshot` pre-2025-relocation residue; existing safe-harbor/self-transfer fixtures re-verified; spec framing reconciled) and M1 (compliance determinism) fixed. M2-M4/N1-N3 folded or consciously deferred to `FOLLOWUPS.md`.

---

# Round 2 — re-review

**Re-reviewer role:** independent architect (author ≠ reviewer). Source re-grounded against the CURRENT tree at re-review time (2026-06-29): `pools.rs` (`consume_fifo` front-walk `:58-100`), `fold.rs` (relocation `acquired_at: c.acquired_at` `:545`, `push_lot` to back `:580-583`; six consume sites; `note_pre2025_once` literal "FIFO" `:38`), `resolve.rs` (decisions seq-sort `:311-318`; voided `:269-303`; single snapshot `:520`; `effective: Vec<(EventId, Vec<Lot>)>` `:523`; seed origin `= d.id`, `split_sequence = i` `:570-586`; multiple-effective `DecisionConflict` `:602-615`), `transition.rs` (`universal_snapshot` `:25-51`), `state.rs` (`BlockerKind`/`severity` Hard set `:35-49`; `Disposal{event,disposed_at,fee_mini_disposition}` `:92-101`; `Removal{event,removed_at}` `:116-124`; `DisposalLeg` `:81-91`). Spec §A.3 + fold records and `FOLLOWUPS.md` lines 82-94 read.

## Verdict

**GREEN. 0 Critical / 0 Important / 0 Minor / 0 Nit.** A's plan is ready to implement (subagent-driven). The round-1 Critical is closed by deliberate adoption-with-tests, not by an equivalence hand-wave; all four Minors and three Nits are folded or consciously deferred; the new acquisition-date-FIFO order introduces no conservation/determinism regression the plan fails to handle.

---

## 1. C1 — CLOSED (verified, not asserted)

**No residual equivalence claim anywhere.** Grepped the whole plan for `push-order ≡`, `behavior identical`, `no-op`, `equivalent`, `distinct acquired_at ⇒`, `insertion`, `tiebreak`. Every surviving "insertion-order / push-order" mention frames it as the **legacy behavior being deliberately corrected** (Task 2 step 4 line 465; Ambiguity #3 line 1976; KAT comments 645-649, 1303, 1328; fold record 1985-1988) or as the failure mode a KAT asserts against (`assert_eq!(... "legacy insertion-order FIFO would have wrongly picked A")`). The lone `tiebreak` at line 13 is the generic NFR4 total-order statement. **No "behavior is identical on all current fixtures" / "push-order ≡ total-order" text remains.**

**FIFO defined once, as a deliberate material correctness change.** FIFO = `acquired_at` asc, tie `lot_id` asc, stated consistently in Task 2 (`method_order`, lines 338-348), the Task 2 step-4 deliberate-change note (465-473), §A.3 (spec line 83-90), Ambiguity #3 (1976), and the C1 fold record (1988). All five say the same thing; none contradicts. The change is framed as correcting a **latent §1012(j)(3)(i) deviation** (a relocated lot retains its original `acquired_at` — confirmed `fold.rs:545`).

**The three divergence KATs are present AND genuinely exercise the divergence (each would FAIL under insertion-order FIFO).** I traced each against current source:

- **(a) SelfTransfer relocation** — `relocated_older_lot_consumed_first_…` (Task 3, lines 651-692). Z (COLD, acq 2025-01-01, $40) relocated via TransferOut+TransferLink into HOT which already holds A (acq 2025-08-01, $80). Confirmed `fold.rs:537-553` builds the relocated fragment with `acquired_at: c.acquired_at` and `:580-583` `push_lot`s it to the **back** → HOT insertion order `[A, Z']`, acquisition order `[Z', A]`. A 2026-02-01 partial Dispose under FIFO must take **Z' ($40, LongTerm)**; the KAT asserts exactly that, with the LIFO/HIFO variants (election seq 2, eff 2025-10-01) both taking A ($80, ShortTerm). **Under legacy insertion-order FIFO it would take A ($80, ST) → RED.** Genuine. Basis *and* term flip.
- **(b) Path-B non-`acquired_at` seeding** — `path_b_seed_in_non_acq_order_…` (Task 6, lines 1305-1325). Confirmed `resolve.rs:570-586` assigns seed `split_sequence = i` (enumerate index) with origin `= d.id`, and `transition.rs:70` `push_lot`s `seed.iter()` in index order. The allocation lists NEWER (idx 0, acq 2024-06-01, $60) before OLDER (idx 1, acq 2024-01-01, $40), so the cb() pool's insertion order is `[split 0, split 1]`. A post-2025 100k FIFO Dispose must take **split 1 ($40)**; the KAT asserts `leg.basis == $40` and `leg.lot_id.split_sequence == 1`. **Insertion-order would take split 0 ($60) → RED.** Genuine. (Totals 200k/$100 conserve against the FIFO residue, so Path B is effective.)
- **(c) pre-2025 SelfTransfer reordering the Universal residue** — `pre2025_self_transfer_reorders_universal_snapshot_residue_…` (Task 6, lines 1333-1360). B1 (acq 2024-01-01, $40) + B2 (acq 2024-06-01, $60) in Universal; a pre-2025 TransferOut+TransferLink to `cold` (still Universal, `pool_key(2024-09-01, cold) = Universal`) consumes B1 (oldest) and re-pushes B1' to the **back** → Universal insertion `[B2, B1']`. The pre-2025 Dispose then takes **B1' ($40)** under acquisition-date FIFO, leaving residue **B2 ($60)**; the allocation records $60 and conserves. The KAT asserts `snap.basis`-driven conservation holds (no `SafeHarborUnconservable`), Path B effective, and `d.legs[0].basis == $40`. **Under insertion-order the Dispose takes B2 (front, $60), residue = B1' ($40), and the $60 allocation reads `SafeHarborUnconservable` → RED.** Genuine; exercises the conservation-residue path specifically. (`universal_snapshot` folds the same pre-2025 timeline through the shared `fold_event`, so the snapshot residue tracks the real fold — `transition.rs:25-51`.)

**Fixture-re-verify step explicit and non-silent.** Task 2 step 4 (lines 472) names `kat_tax.rs` (self-transfer relocation, TP8(c) fee-rehome), `transition.rs` (Path-B seed, `universal_snapshot`, `:733`), `properties.rs` (Σ-basis), mandates updating each moved golden **in-line with a tax-reason comment** and confirming `conservation_report` balanced. Catch mechanism is sound: any legacy golden that legitimately moves turns the workspace suite RED (forcing confrontation), and the step forbids a silent "fix." Sufficient.

**Spec §A.3 reconciled.** Spec line 83 now reads "**deliberate, material adoption … that replaces the foundation's insertion-order FIFO … not merely a tiebreak rule**," with a dedicated deliberate-correctness note (line 88) and the M2 fold-record line (257) re-annotated. `FOLLOWUPS.md` lines 82-94 carry the matching "RESOLVED-in-design" entry (no real users yet → no migration owed). All three artifacts agree.

## 2. Minors / Nits — all closed

- **M1 CLOSED — compliance made-date determinism.** Task 7 (`compliance.rs`, lines 1588-1598) collects `LotSelection`s into a `Vec<(seq, &Event)>`, `sort_by_key(seq)`, then inserts into `sel_made` ascending → highest-seq-wins, load-order-independent (mirrors `resolve.rs:311-318`). The validity condition for elections (`effective_from >= TRANSITION_DATE && >= made`, line 1569) is the exact negation of resolve's rejection rule (`:706` in the plan) — no semantic drift.
- **M2 CLOSED — conflict only post-Path-selection.** Task 6 changes `effective` to `Vec<(EventId, Vec<Lot>, LotMethod)>` (line 1383), pushes the recorded method with the seed and emits **no** conflict inside the effectiveness loop; `Pre2025MethodConflictsAllocation` fires **only** in the `match effective.len() { 1 => … }` arm (1394-1404), and the `_ =>` arm keeps the verbatim multiple-effective `DecisionConflict` → Path A (1406). The multiple-effective case can never fire a spurious method-conflict.
- **M3 CLOSED — `config --set-forward-method` is a real task+tests.** Task 5 adds `reconcile::set_forward_method` (interface 1004; impl 1177-1184, appends a `MethodElection`, defaults `effective_from` to the made-date), two round-trip tests (`set_forward_method_appends_a_method_election_decision`, `…_defaults_effective_from_to_made_date`, 1087-1110), and a `Command::Config` dispatch branch (1208-1216). Coverage-map row (1938) + footnote (1953) updated.
- **M4 CLOSED — concrete RED→GREEN bodies.** Task 7 `compliance.rs` (1441-1541) and Task 8 `verify_report.rs` (1666-1730) are now executable fixtures: synthetic events for the six compliance states (incl. broker-2027 NonCompliant, broker-2026 StandingOrder, post-hoc NonCompliant), and real CLI flows for verify (reads disposal/lot refs from the projection, lines 1694-1702). No prose-stub fixtures remain in Tasks 7/8.
- **N1/N2/N3 folded/recorded.** N1: `parse_lot_id` test gains the `"in|99|credit|1#0"` `#`-in-`source_ref` case (1028). N2: `hifo_cmp` uses `== Usd::ZERO` (353). N3: correctly recorded as N/A — `load_events_and_project` returns `ProjectionConfig` not `CliConfig`, so verify's separate `session.config()?` read is required (fold record 1999; Task 8 inspect.rs 1751-1752).

## 3. No NEW Critical/Important from the fold

- **Conservation invariant intact.** `take_from` (Task 2, 445-459) is the byte-for-byte arithmetic of the old `consume_fifo` body (`split_pro_rata` on gain + dual-loss basis, `retain(remaining_sat>0)`); `method_order`/`consume_ordered` only change *which index* is visited, never how much is taken. Σsat and Σbasis are conserved under any permutation. Selection paths (`selection_feasible`/`consume_picks`) validate feasibility first then fall back to method order on failure, so Σsat always closes and the hard `LotSelectionInvalid` gates tax.
- **Method-aware snapshot + conflict-blocker sound under the new order.** `held_sat` (Σ remaining sat) is method-invariant, so the per-candidate snapshot's sat check is stable; only `basis` is method-dependent and is checked against the residue under **that allocation's recorded method** (Task 6, 1385). Live≠recorded surfaces the dedicated hard blocker (never `SafeHarborUnconservable`) while Path B stays effective — KAT-(c) and `live_config_differs_from_recorded_method_is_pre2025_conflict` cover both arms. `selections` is correctly threaded into `universal_snapshot` (pre-2025 selections can move the residue); elections are inert pre-2025 (all `effective_from >= TRANSITION_DATE`) but passed for the `FoldCtx` signature — harmless.
- **Determinism preserved.** `method_order` is a strict total order (FIFO/LIFO tie on `lot_id`; HIFO ties on `acquired_at` then `lot_id`, cross-multiplied — no float, NFR5); all maps/sets are `BTreeMap`/`BTreeSet`; decisions/selections iterate in `decision_seq` order; the `path_b`/relocation `split_sequence` counter (`init_split_counter`, `pools.rs:52`) keeps relocated-lot IDs collision-free under the reorder. `determinism_with_elections_and_selections_is_load_order_independent` (Task 4, 919-929) locks it.

## 4. Non-blocking observations (NOT findings — for the implementer / Task 10, no action required to ship)

- The election-validity collector is duplicated between `resolve.rs` and `compliance.rs`; the plan already flags extracting a shared collector as a Task-10 `FOLLOWUPS.md` item (Task 7 lines 1559-1561, Task 10 step 4). Consciously deferred; both cite the same spec rule, so no current drift.
- `DisposalCompliance` classifies on the **timing** test only, so a contemporaneous-but-arithmetically-invalid `LotSelection` reads `Contemporaneous` while its `LotSelectionInvalid` hard blocker gates tax separately. This is **spec-faithful** (A.5 defines `Contemporaneous` purely as made-date ≤ time-of-sale); noted only so Task 8/`verify` reviewers don't mistake it for a leak.

**Re-review conclusion:** C1 closed by a tested deliberate adoption; M1-M4 + N1-N3 closed; no new blocking finding. **0 Critical / 0 Important — A's plan is ready to implement (subagent-driven), Phase D.**
