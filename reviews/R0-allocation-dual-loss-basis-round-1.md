# R0 Architect Review (mandatory gate) — `reconcile-allocation-dual-loss-basis` (Slug 1), Round 1

**Artifact:** `design/SPEC_allocation_dual_loss_basis.md`
**Reviewer role:** R0 architect (independent; author ≠ reviewer).
**Method:** Reviewed the spec+plan against the REAL workspace source at every cited line/symbol, plus the data-flow it depends on (pools → fold four-zone) and the canonical `design/SPEC_foundation.md`.
**Verdict:** **NOT GREEN — 0 Critical / 3 Important.** The executable code change is correct and complete, but the spec's §1015(a) gain/loss-basis *semantics are inverted* relative to the engine, `state.rs`, AND the canonical foundation spec. That inversion would (a) plant tax-wrong comments into a conservation-critical public struct + corrupt the foundation-spec doc-update, (b) make KAT B1 un-passable, and (c) make KAT A1's tacking assertion tax-wrong while leaving the `donor_acquired_at` half of the fix unproven by any disposition.

---

## Positive confirmations (verified correct — do NOT change)

These are the load-bearing correctness checks the gate exists to protect; all pass:

1. **The 3 executable change sites are correct and label-agnostic.**
   - `AllocLot` field *names* match `Lot` (`state.rs:64/66/67`), so the verbatim copies are right regardless of the labeling defect below.
   - `resolve.rs:581-582` (`dual_loss_basis: None, donor_acquired_at: None`) → `l.dual_loss_basis` / `l.donor_acquired_at`: correct site, correct fields.
   - `reconcile.rs:234-239` builder add `dual_loss_basis: l.dual_loss_basis, donor_acquired_at: l.donor_acquired_at`: `l` is the residue `Lot`, which carries the gift dual basis verbatim from the pre-2025 projection. Correct.

2. **The fix genuinely flows through to the disposition.** Verified end-to-end:
   `Lot.dual_loss_basis`/`donor_acquired_at` → `consume_fifo` (`pools.rs:70-86`: `dual = lot.dual_loss_basis.is_some()` @82, `loss_basis` @71-73, `gain_hp_start = lot.gain_hp_start()` @79, `loss_hp_start` @80, `donor_acquired_at` @86) → `Consumed` (`pools.rs:105-118`) → `make_disposal_legs` four-zone (`fold.rs:80-109`). The loss zone (`fold.rs:91-99`) reads `c.loss_basis`; the gain/no-dual term tacks via `c.gain_hp_start` (= `donor_acquired_at.unwrap_or(acquired_at)`, `state.rs:73`). Seeding these fields on the Path-B `Lot` **will** change the loss-zone basis and the gain-side term. The spec's claim that `make_disposal_legs` "already keys on `dual_loss_basis.is_some()`" is accurate.

3. **Conservation is unaffected (`SafeHarborUnconservable` safe).** The guard sums only `alloc_sat = Σ l.sat` (`resolve.rs:545`) and `alloc_basis = Σ l.usd_basis` (`resolve.rs:546`), compared to `snap.held_sat` / `snap.basis` (`resolve.rs:547`). `dual_loss_basis` is an *alternative* basis and is correctly NOT in the conservation identity. Adding it cannot break the guard. ✓

4. **No fingerprint / event-id / dedup change.** `fingerprint()` returns `None` for `SafeHarborAllocation` (`persistence.rs:96`, `_ => return None`; only the six imported payloads are fingerprinted). Decisions persist with `fp = None` (`append_decision` → `insert(..., KIND_DECISION, None)`, `persistence.rs:259`) and carry `EventId::Decision { seq }` (seq-based, content-independent). Adding fields to `AllocLot` changes **no** fingerprint, dedup key, or event id. The persistence `normalize`/`d`/`od`/… path (`persistence.rs:25-99`) never touches decision payloads. ✓

5. **Serde backward-compat holds.** `EventPayload` persists via `serde_json::to_string` (`persistence.rs:165`) and loads via `serde_json::from_str` (`persistence.rs:290`). A pre-existing `SafeHarborAllocation` JSON lacking the two keys deserializes to `None`. `#[serde(default)]` is correct (`Option` impls `Default`). See N-1 for a redundancy nit.

6. **Blast radius is complete; workspace will compile.** Workspace = 4 crates (`Cargo.toml:3`). The ONLY `AllocLot` literal-construction sites are `event.rs:325`, `event.rs:335`, `tests/transition.rs:102` (the `alloc_lot` helper — a single literal that covers all 14 call sites at lines 129/163/196/197/262/288/317/318/353/405/506/648/680/681), and `reconcile.rs:234`. The plan enumerates exactly these. No exhaustive `AllocLot { .. }` destructuring patterns exist anywhere (only field reads on `Lot`, e.g. `reconcile.rs:237`), so no match-arm breakage. ✓

7. **LotId uniqueness / split-counter / pool routing untouched.** The seed `LotId { origin_event_id, split_sequence: i }` (`resolve.rs:571-574`) and `init_split_counter` (`pools.rs:52`) are independent of the added fields. ✓

8. **SemVer additive MINOR (pre-1.0) is correct;** scope matches the follow-up intent; no over-engineering.

9. **Spec line citations are structurally accurate:** `AllocLot` @ `event.rs:145-150` ✓; Path-B seed @ `resolve.rs:566-585` (the two `None`s at 581-582) ✓; conservation guard @ `resolve.rs:545-547` ✓; `Lot` fields @ `state.rs:64-67` + accessors @ `state.rs:70-79` ✓; `persistence.rs:165` ✓; `reconcile.rs:234-239` ✓; `transition.rs:101-108` helper, `event.rs:325/335` literals ✓.

---

## Findings

### CRITICAL
None. (The fix flows through; no fingerprint break; no missed literal; no conservation regression.)

### IMPORTANT

#### I-1 — §1015(a) gain/loss basis is INVERTED throughout the spec; merging it corrupts source comments and the canonical foundation spec.
The new spec states (Goal line 4; Design line 19/27/29; A1 narration line 64):
> "dual basis (**gain basis = FMV-at-gift, loss basis = donor basis**)"
> `usd_basis: Usd, // gain basis (= FMV-at-gift for a received gift)`
> `dual_loss_basis: Option<Usd>, // §1015(a) loss basis (donor basis) for received gifts`

This is backwards. §1015(a): **gain basis = donor's carryover basis**; **loss basis = FMV-at-gift**, and the dual basis exists **only when FMV-at-gift < donor basis**. Three independent authorities in this repo agree, and all contradict the spec:
- **Engine:** `fold.rs:679-680` — Case 2 (`fmv_at_gift < b`): `(*b, Some(*fmv_at_gift), ...)` ⇒ `usd_basis = donor basis b`, `dual_loss_basis = Some(fmv_at_gift)`. Comment @679: "dual: gain basis = donor basis, loss basis = FMV."
- **State:** `state.rs:64` `usd_basis // gain basis`; `state.rs:66` `dual_loss_basis // ... loss basis when FMV-at-gift < donor basis`.
- **Canonical spec:** `SPEC_foundation.md` TP11 (line 34): "gain-basis = donor carryover (HP tacks); if FMV-at-gift < donor basis, **loss-basis = FMV-at-gift**, HP from gift date"; §6.4 (line 128); §7.4 dispose rule (line 124).

Impact: A1/A2 instruct adding the `AllocLot` struct *with these inverted comments* (they'd land in `event.rs`, directly contradicting `state.rs`/`fold.rs`), and the spec also directs a doc-update to `SPEC_foundation.md §6.4/§7.4` (Design line 58) which would *corrupt the currently-correct* TP11/§6.4/§7.4. In a §1015(a) tax engine an inverted basis label is a real hazard, and this inverted mental model is the root cause of I-2 and I-3.

**Fix:** Re-label everywhere so the artifact matches the engine and foundation spec:
- `usd_basis` comment → "gain basis = donor's carryover basis (§1015(a)); for a received gift this is `donor_basis`."
- `dual_loss_basis` comment → "§1015(a) loss basis = FMV-at-gift; `Some` only when FMV-at-gift < donor basis (the dual case), else `None`."
- Fix the Goal/Design prose and the A1 narration accordingly. The dual-basis trigger is **FMV-at-gift < donor basis** (not the reverse).

#### I-2 — KAT B1's assertion is wrong against the engine and cannot pass.
B1 (plan line 69) folds a real `GiftReceived{donor_basis, fmv_at_gift, donor_acquired_at}` then asserts the produced `AllocLot` has `dual_loss_basis == Some(donor_basis)`. The engine stores `dual_loss_basis = Some(fmv_at_gift)` (`fold.rs:680`), never `Some(donor_basis)`. For any genuine dual gift you need `donor_basis > fmv_at_gift`, so `Some(donor_basis) != Some(fmv_at_gift)` → the assertion fails. The only way to make `donor_basis == fmv_at_gift` is `fmv_at_gift >= donor_basis` → Case 1 (`fold.rs:677`) → `dual_loss_basis = None`, which also fails the assertion and produces no dual basis at all. **B1 as written is un-passable.**

**Fix:** Set up `donor_basis > fmv_at_gift` (e.g. `donor_basis = $100`, `fmv_at_gift = $40`) and assert the `AllocLot` carries `usd_basis == donor_basis` ($100) and `dual_loss_basis == Some(fmv_at_gift)` (Some($40)), plus `donor_acquired_at == Some(...)`.

#### I-3 — KAT A1 mis-models §1223(2) tacking (loss zone does not tack) and under-tests `donor_acquired_at`.
A1 (plan line 64) disposes the seeded lot in the **loss zone** and asserts "the holding period reflects tacking." But the loss zone uses `loss_hp_start = acquired_at = gift date` (`state.rs:76`; consumed @ `pools.rs:80`; used @ `fold.rs:93`), and §1223(2) does **not** tack on the loss side (basis is FMV, not carryover) — confirmed by `SPEC_foundation.md` line 124 ("loss ... HP from gift date") and TP11 line 34. So:
- Asserting loss-zone tacking is tax-wrong and would fail (term runs from the 2024 gift date, not donor's 2021).
- A loss-zone disposition is *independent of `donor_acquired_at`* (it's ignored on the loss side), so A1 proves the `dual_loss_basis` half of the fix but leaves the **`donor_acquired_at` half unproven by any disposition** — only by the seed-field-equality check. The prompt explicitly wants the KAT to prove flow-through to the disposition, "not just that a field is set."

Note: A1's field *values* are actually fine (`usd_basis = $100` IS the donor/gain basis; `dual_loss_basis = Some($40)` IS the FMV/loss basis) — only the *labels* are swapped per I-1, and the loss-zone arithmetic (proceeds $30 < $40 → loss $10 off $40) holds.

**Fix:** (a) Drop the loss-zone "tacking" assertion. (b) Add a **gain-zone** disposition (proceeds > gain_basis $100, e.g. proceeds $150) — or a NoGainNoLoss-zone disposition — and assert its term tacks from `donor_acquired_at` (2021 → long-term), contrasted against the old behavior where `donor_acquired_at = None` ⇒ `gain_hp_start = gift date 2024` (different term). That is the assertion that genuinely exercises the `donor_acquired_at` fix end-to-end.

### MINOR / NIT

- **N-1 (Nit) — `#[serde(default)]` on `Option` is redundant but harmless.** serde already defaults a missing `Option<T>` field to `None` (the `missing_field` deserializer routes `deserialize_option` → `visit_none`). Keep the attribute (explicit/defensive) — the backward-compat claim is valid either way. The dedicated round-trip + omit-keys test (`alloc_lot_serde_backward_compat`) is meaningful as a regression guard; no change required.
- **N-2 (Minor, optional) — ProRata / NoGainNoLoss coverage.** A separate ProRata KAT is not strictly needed: the `resolve.rs:566-585` seed loop is method-agnostic (method only affects the time-bar @ `resolve.rs:537-542`), so both methods seed dual basis identically. A NoGainNoLoss middle-zone case is nice-to-have but is subsumed by the gain-zone tacking case from I-3 (both use `gain_hp_start`). Mention as out-of-scope rationale rather than adding tests.

---

## Required to reach green (Round 2)
1. **I-1:** Swap the §1015(a) labels in Goal/Design prose, the proposed `AllocLot` field comments, and the `SPEC_foundation.md §6.4/§7.4` doc-update so they read: gain basis = donor carryover; `dual_loss_basis` = FMV-at-gift; dual triggers when FMV-at-gift < donor basis.
2. **I-2:** Rewrite B1 with `donor_basis > fmv_at_gift` and assert `dual_loss_basis == Some(fmv_at_gift)` (+ `usd_basis == donor_basis`).
3. **I-3:** Remove the loss-zone tacking claim; add a gain-zone (or NoGainNoLoss) disposition that proves `donor_acquired_at` tacking flows to the term.

The executable diff (3 change sites + literal updates) needs no change; the defects are confined to the spec's tax semantics, comments, doc-update, and the two KATs. Re-review required after the fold (including the last).

---

## Round 2 — fold re-review

**Artifact (revised):** `design/SPEC_allocation_dual_loss_basis.md`
**Reviewer role:** R0 architect (independent; re-review of the R0-I1/I2/I3 fold).
**Method:** Small doc fold. Re-read the revised spec end-to-end and re-checked the three §1015(a) labels + the two KATs against the engine ground truth: `fold.rs:673-681` (zone construction) + `fold.rs:80-109` (`make_disposal_legs`), `state.rs:58-79` (`Lot` fields + `gain_hp_start`/`loss_hp_start`), and `SPEC_foundation.md` (TP11 L34, §6.4 Lot L76, dispose rule L124, ClassifyInbound L128). Re-confirmed the executable change sites are unchanged and citations are live.
**Verdict:** **GREEN — 0 Critical / 0 Important.** All three Importants are closed; no new defect introduced.

### I-1 — CLOSED ✓ (§1015(a) labels now match the engine + foundation spec, no residual inversion)
The orientation is correct and *consistent* in every location:
- **Goal (L4):** "gain basis = donor carryover basis; loss basis = FMV-at-gift (... applies only when FMV-at-gift < donor basis) — plus §1223(2) tacking (`donor_acquired_at`)." ✓
- **§1015(a) orientation note (L6):** `usd_basis` = GAIN basis = donor carryover; `dual_loss_basis = Some(FMV-at-gift)` = LOSS basis, set ONLY when FMV-at-gift < donor basis; `donor_acquired_at` tacks on the GAIN side; loss side uses gift date (no tacking). Matches `fold.rs:679-680` Case 2 `(*b, Some(*fmv_at_gift), …)` and `state.rs:64/66/72-77`. ✓
- **`AllocLot` field comments (L29-32):** `usd_basis` = "GAIN basis = donor carryover basis"; `acquired_at` = "gift date = loss-zone HP start (no tacking)"; `dual_loss_basis` = "LOSS basis = FMV-at-gift; Some only when FMV-at-gift < donor basis"; `donor_acquired_at` = "§1223(2) tacking; gain/no-dual-zone HP start." All match `state.rs:61/64/66/67`. ✓
- **Problem paragraph (L16):** loss-zone error "uses the gain basis = donor carryover instead of the lower FMV-at-gift loss basis, understating the loss"; gain-zone "loses §1223(2) tacking (term from gift date instead of `donor_acquired_at`)." Correct. ✓
- **Foundation-spec guard (L61):** explicit `[R0-I1]` instruction to NOT alter the already-correct TP11/§6.4/§7.4 labels, and to reuse them for the new field descriptions. Present and correct; the foundation labels at L34/L76/L124/L128 are verified still correct, so "do not alter" is the safe directive.

A full scan of the revised spec found **no residual inverted label** (no "gain basis = FMV" / "loss basis = donor" anywhere).

### I-2 — CLOSED ✓ (KAT B1 now matches what the engine stores)
B1 (L73) folds `GiftReceived{donor_basis: $100, fmv_at_gift: $40, donor_acquired_at}` (FMV-at-gift $40 < donor $100 ⇒ dual) and asserts the produced `AllocLot` has `usd_basis == $100` (donor/gain basis) and `dual_loss_basis == Some($40)` (FMV-at-gift), plus `donor_acquired_at == Some(...)`. This is exactly `fold.rs:680` (`(*b, Some(*fmv_at_gift), …)`). Passable. The prior un-passable `Some(donor_basis)` assertion is gone.

### I-3 — CLOSED ✓ (tacking proof moved to the gain zone; loss-zone KAT no longer claims tacking; zones internally consistent)
- **`path_b_preserves_gift_tacking` (L68)** now disposes in the **gain zone**: proceeds $150 > gain_basis $100 ⇒ gain $50 (`fold.rs:82-90`), on 2025-03-01. `gain_hp_start` = `donor_acquired_at` 2021-01-01 ⇒ >1yr ⇒ **LONG-TERM**; under OLD `donor_acquired_at: None`, `gain_hp_start` = gift date 2024-06-01 ⇒ ~9mo ⇒ SHORT-TERM. The term flip end-to-end proves the `donor_acquired_at` half of the fix. Internally consistent ($150 > $100; 2021→2025-03 > 1yr; 2024-06→2025-03 < 1yr). ✓
- **`path_b_preserves_gift_dual_loss_basis` (L67)** no longer claims tacking; it asserts the **loss-zone** loss is computed off the FMV-at-gift loss basis $40 (proceeds $30 < $40 ⇒ loss $10 via `fold.rs:91-99`), vs the single-basis $70 (donor basis $100) under the old `None`. Consistent ($30 < $40). The L68 note correctly states the loss side does NOT tack (§1223(2) gain-side only), so tacking is proven via the gain zone, not the loss zone. ✓

### No new defect ✓
The executable change sites are byte-for-byte the round-1-verified design and the citations are live (re-grepped): `AllocLot` def @ `event.rs:145`; Path-B seed `None,None` @ `resolve.rs:581-582` → `l.dual_loss_basis`/`l.donor_acquired_at`; CLI builder @ `reconcile.rs:234`; literal sites @ `event.rs:325/335` + `transition.rs:101-102` helper. Conservation (Σsat / Σ`usd_basis` only — `dual_loss_basis` excluded), serde `#[serde(default)]` backward-compat (N-1 redundant-but-kept), fingerprint/dedup, and blast-radius conclusions from Round 1 all still hold unchanged.

### Nit (non-blocking, no action required)
- **N-3 (nit):** The L61 guard's parenthetical section→line map ("§6.4 ~124, §7.4 ~128") is slightly loose — in current `SPEC_foundation.md` the §6.4 `Lot` definition is L76, the dispose rule is L124, and ClassifyInbound is L128. Immaterial: the directive is "do not alter," every one of those lines is already correct, and the safe action is identical regardless of the exact mapping.

### Round 2 verdict
**0 Critical / 0 Important — R0 GREEN. Cleared to implement.** All three Round-1 Importants (I-1 label inversion, I-2 un-passable B1, I-3 mis-modeled tacking) are folded correctly and verified against the engine; the executable design is unchanged and remains correct.
