# R0 spec review — `SPEC_tui_edit_hardening.md` (round 2)

**Artifact:** `design/SPEC_tui_edit_hardening.md` (post round-1 fold).
**Reviewer:** independent adversarial architect. All fold claims re-verified against current source.

## Verdict: **0 Critical / 0 Important / 0 Minor / 2 Nit** — **R0-GREEN (0C / 0I)**

All round-1 findings resolved with no new drift. The I-1 fold is tax-correct and the `basis_source` discriminator is provenance-sound. Two trivial Nits (non-blocking) noted for polish.

---

## Fold-by-fold verification

### [I-1] RESOLVED — the acquisition-date gate is now feasibility-honest, and the discriminator is sound

The corrected "Verified core fact" (lines 34-47) now states the true §7.4 boundary-drain reality, and the gate (lines 218-228) excludes Path-B seed lots. Verified against source:

- **`SafeHarborAllocated` is produced by exactly one site** — `resolve.rs:915` (the Path-B effective-allocation seed). `grep` over `crates/btctax-core/src` confirms no other producer. So `l.basis_source != SafeHarborAllocated` excludes **precisely** the Path-B seed lots and nothing else.
- **Path-A relocated lots are `ReconstructedPerWallet`** — `transition.rs:83` (the only producer). Not excluded, and feasible (lot_id preserved through the drain), so the real fix — cross-wallet Universal offering — works.
- **The two provenances are mutually exclusive per projection.** `seed_transition` (`transition.rs:75-103`) runs exactly one arm: Path A relocates the whole residue as `ReconstructedPerWallet`; Path B discards it and installs only `SafeHarborAllocated` seeds. There is no mixed state in which the discriminator misfires.
- **The discriminator never drops a feasible lot.** A `SafeHarborAllocated` lot's lot_id (`{allocation_id, seq}`, `resolve.rs:906-908`) never existed in `Universal`, so it is *never* feasible for a pre-2025 disposal; excluding all of them is safe. Non-seed pre-2025 lots (original `Universal` residue that never drained, or `ReconstructedPerWallet`) keep their `Universal` lot_ids → feasible → correctly offered.
- **"No lots" is the honest Path-B outcome.** Under Path B the feasible pre-2025 lots are the *discarded* originals — absent from `snap.state.lots` entirely — so there is genuinely no TUI-offerable feasible lot; excluding the seeds yields the existing empty-rows path (`main.rs:2738-2748`), a safe under-inclusion with the CLI (which re-projects at fold position) still available. Strictly better than offering a systematically doomed set.
- **The scoping is correct.** The discriminator is applied only in the `item.date < TRANSITION_DATE` branch; the post-2025 branch is unchanged, which is right — for a post-2025 disposal the seed lots live in `Wallet(w)` post-boundary and *are* feasible, so they must stay offered there.
- **`btctax_core::BasisSource` resolves** (crate-root glob re-export, `lib.rs:17 pub use event::*`), and `Lot.basis_source` is a public field (`state.rs:99`), so the gate compiles as written.

**The residual is honestly scoped and non-silent (verified backstop).** The acknowledged residual (lines 240-243) — a lot created by a *later* split/relocation offered for an *earlier* pre-2025 disposal — is the irreducible "final-state ≠ fold-time-state" gap; fixing it fully would need pool-state re-projection at each disposal's fold position (correctly scoped out to the `resolve.rs` pub-fn alternative). Its backstop is real: `derive_select_lots_status` **arm 2** (`main.rs:3440-3447`) explicitly surfaces `LotSelectionInvalid` for the disposal ("LotSelection saved but invalid — see Compliance … Correct via Void flow (press 'v') then re-select."), and the blocker *gates* `compute_tax_year` — so the residual can never silently corrupt a tax number; it fails safe and actionable. Appropriate FOLLOWUP disposition, not a blocking defect. New Path-B KAT (`KAT-PRE2025-PATHB-SEEDLOTS-EXCLUDED`, line 247) locks the systematic fix.

### [M-1] RESOLVED — engine-coverage citations now point at `crates/btctax-core/tests/transition.rs:365/:403` (lines 56-58, 149, 159). Correct file; both directions still pinned.

### [M-2] RESOLVED — #1 detection now (a) collects non-voided `TransferLink`s and **sorts by `decision_seq` ascending** before the FIRST-WINS loop (lines 184-190), matching the engine's `resolve.rs:349-356`, and (b) uses `ev_idx.get(in_id).and_then(|e| e.wallet.as_ref()).is_none()` (line 195) — no index panic, and behaviorally identical to `resolve.rs:509` (missing in-event → skip).

### [N-1] RESOLVED — `FREETEXT_CAP` documented as a render-safe bound, not literal CLI parity (lines 112-114).

### [N-2] RESOLVED — #8 fold now instructs per-arm literal folding with the split-line caveat (site 2's `or CLI:` on `:2078`) and relies on per-arm RS KATs to catch a miss (lines 84-87). Verified the six anchors and no 7th production site still hold.

---

## Nits (non-blocking; optional polish)

- **N-3 (Nit):** the residual example (line 241) names only a "pre-2025 self-transfer fragment." The identical gap also arises under Path B when a *post-2025* self-transfer relocates a seed lot into a `CarriedFromTransfer` lot with `acquired_at < 2025` (new lot_id, `fold.rs:768,779`) — not `SafeHarborAllocated`, so not excluded, and infeasible for any pre-2025 disposal. It is fully covered by the spec's general "final-state ≠ fold-time-state" framing and the same `LotSelectionInvalid` backstop, so no correctness gap — but the example could name this Path-B sub-case for completeness.
- **N-4 (Nit):** the header still reads "Review status: DRAFT — awaiting R0 round 1" (line 4); update to reflect round-2 GREEN.

---

## Confirmation

The spec is **R0-GREEN (0 Critical / 0 Important)**. The I-1 fold eliminates the systematic doomed-offer under both transition paths; the `basis_source != SafeHarborAllocated` discriminator is provenance-exact (single producer, mutually-exclusive paths, no false exclusion); the surviving residual is irreducible, fails safe (gated + surfaced), and is FOLLOWUP-tracked. Cleared to proceed to Plan/Implement.
