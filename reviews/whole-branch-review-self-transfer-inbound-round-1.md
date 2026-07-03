# Whole-diff review (Phase E) — self-transfer-inbound (Cycle A), round 1

**Reviewer:** independent adversarial (Phase E). **Branch:** `feat/self-transfer-inbound` @ `92ab579`
(main `a740b3d`). **Diff:** `git diff main..HEAD` — 16 files, +1988/−29 (code-only 13 files, +1441/−29;
the extra 3 files/547 lines are the spec + 2 R0-spec reviews). **Contract:**
`design/SPEC_self_transfer_inbound.md` (R0-GREEN). **Bar:** 0 Critical / 0 Important.

## Verdict: **0 Critical / 0 Important / 1 Minor / 1 Nit — SHIP** (after the Minor is either fixed or filed to FOLLOWUPS; it does not gate).

The engine change is faithful to the spec and tax-honest. Every load-bearing invariant (G1 never-gate,
G2 non-taxable, G3 no-tacking, G4 advisory-keys-on-None, G5 wallet-missing, G6 outside-FIFO catch-alls
untouched, FR9 conservation, pool⊥HP orthogonality) is verified against current source AND KAT-pinned.
The four fault-injections (G1/G2/G6 + G4) each drove exactly the intended KAT RED and were restored
byte-for-byte (tree clean, baseline re-green 12/12). Cross-crate lockstep is consistent; clippy clean on
all four changed crates; the CLI (3) and TUI (7) tests pass.

---

## Fault-injection results (all confirmed; tree restored clean after each)

| Probe | Injection | KAT driven RED | Result |
|---|---|---|---|
| **G1** (silent gate) | `fold.rs:1005` `basis_pending: false → true` | `self_transfer_in_is_never_basis_pending_or_gated` (kat_tax.rs:3076) | **RED ✓** — "$0 basis is computable — NEVER pending (G1)" |
| **G2** (non-taxable) | push an `IncomeRecord` in the arm (copied the IncomeInbound line) | `self_transfer_in_is_non_taxable` (kat_tax.rs:3051) | **RED ✓** — "self-transfer-in recognizes NO income" |
| **G6** (outside-FIFO) | `resolve.rs` add `Op::SelfTransferInbound { sat, .. } => Some(*sat)` to `honoring_principal` | `self_transfer_in_is_outside_fifo_but_sellable` (kat_tax.rs:3233) | **RED ✓** — "a LotSelection cannot target a lot-creating self-transfer-in" |
| **G4** (advisory keys on None) | `fold.rs` `if basis.is_none()` → `if usd_basis == Usd::ZERO` | `self_transfer_in_supplied_basis_has_no_advisory` (kat_tax.rs:2961) | **RED ✓** — "attested Some(0) must be silent" |

After all four: `git status` clean; `cargo test -p btctax-core --test kat_tax self_transfer_in` → 12
passed / 0 failed.

---

## Per-item verification (against current source)

- **[G1 ★] `basis_pending: false` never gates** — `fold.rs:1005` sets `false` unconditionally (even at
  the defaulted $0). `make_disposal_legs` gates on `c.basis_pending` (`fold.rs:138`); a `false` lot never
  trips it. KAT invariant-1/5 prove a later Sell computes a real gain (basis 0 → gain == proceeds, term
  ST) with NO `FmvMissing`. **Highest-stakes invariant — sound.**
- **[G2 ★] Non-taxable** — the arm pushes NOTHING to `st.income_recognized` (contrast IncomeInbound
  `fold.rs:843`). `self_transfer_in_is_non_taxable` asserts `income_recognized`, `disposals`, `removals`
  all empty. **Sound.**
- **Conservative direction + defaults** — `basis.unwrap_or(Usd::ZERO)` (`fold.rs:988`),
  `acquired_at.unwrap_or(date)` (`fold.rs:990`). $0 default → max eventual gain (never under-reports);
  receipt-date default → short-term. Nothing under-states a gain/loss. **Sound.**
- **[G3/G4]** — `donor_acquired_at: None` (`fold.rs:1004`) ⇒ `gain_hp_start() =
  donor_acquired_at.unwrap_or(acquired_at) = acq` (`state.rs:114`), no tacking; the disposal-leg
  `acquired_at == acq` KAT confirms. Advisory fires on `basis.is_none()` (`fold.rs:991`), so an attested
  `Some(0)` is silent — proven both by the passing KAT and the G4 fault-injection. **Sound.**
- **[G6 ★] outside-FIFO catch-alls UNTOUCHED** — `is_disposition_op` (`resolve.rs:1011`, `_ => false`),
  `honoring_principal` (`resolve.rs:1023`, `_ => None`), `evaluate::honoring_sat` (`evaluate.rs:76`,
  `_ => None`) — none received a `SelfTransferInbound` arm; all keep their correct defaults. A
  `LotSelection` targeting the event → `LotSelectionInvalid` while FIFO still sells the lot (invariant-7
  KAT). The G6 fault-injection proves a wrong `Some(sat)` would silently break it. **Sound.**
- **Wallet-missing (M2/G5)** — `fold.rs:975-983`: `eff.wallet == None` → Hard `UnknownBasisInbound`
  (self-transfer message), `return` before lot creation — NOT the IncomeInbound `FmvMissing` guard. KAT
  `self_transfer_in_without_wallet_emits_hard_unknown_basis_not_fmv_missing` asserts both the Hard
  blocker and the absence of `FmvMissing`, no panic. **Sound.**
- **Conservation (FR9)** — `stats.sigma_in += *sat` (`fold.rs:1010`). `self_transfer_in_conserves_sigma_in`
  asserts `sigma_in == 100_000`, `conservation_report(...).balanced`, `sigma_held == 100_000`. The pre-2025
  Universal-path KAT (`pre_2025_..._universal_pool`) confirms balance both lot-only and post-sale. **Sound.**
- **pool_key ⊥ acquired_at** — `pool_key(date, &wallet)` keys on the RECEIPT `date` while `acquired_at =
  acq` carries supplied-or-receipt. KAT `..._supplied_old_date_is_long_term_in_wallet_pool` (2026 receipt,
  2013 acquired) proves the 2026-Wallet-pool sale FINDS the lot (no `UncoveredDisposal`) AND it is
  Long-Term. Independently corroborated: invariant-1's sale dated 2025-06-01 (before the 2026-01-01
  classify decision) still produces the disposal — the lot is keyed at the TransferIn receipt date, not
  the decision date. **Sound.**
- **[item 9] forms.rs `how_acquired_from`** — `SelfTransferInbound → Form8283HowAcquired::Review`,
  grouped with `CarriedFromTransfer`/`SafeHarborAllocated`/`ReconstructedPerWallet` (`forms.rs:243-246`).
  Match is fully exhaustive (all 9 variants, no `_ =>`). **Assessment: `Review` is the tax-honest answer.**
  A self-transfer-in coin's original provenance is lost (it arrived from an un-imported wallet with an
  attested-or-defaulted basis); the engine cannot soundly assert "Purchased" or "Gift" on Form 8283's
  "how acquired" line, so deferring to manual `Review` is the correct conservative choice — asserting any
  concrete category would be a fabricated provenance. Correct.
- **[item 10] cross-crate lockstep glue** — `cli render.rs:52` `self_transfer_in`; `tui tabs/tags.rs:26`
  `self_transfer_in`; `tui-edit form.rs:1381` `self-transfer-inbound`; modal render arm `draw_edit.rs:803`;
  `cls_desc` status arm `main.rs:2318` "SelfTransferMine". A workspace-wide grep found NO `match` over
  `BasisSource`/`InboundClass` with a wildcard `_ =>` that silently swallows the new variant — every site
  is exhaustive with an explicit, consistent label. **Sound.**
- **[item 11] reuse** — `SelfTransferMine` rides the unchanged `ClassifyInbound` (no new `EventPayload`).
  Duplicate-first-wins (`duplicate_classify_inbound_self_transfer_first_wins` → 1 `DecisionConflict`, first
  basis governs), void re-exposes `UnknownInbound` (`void_..._re_exposes_unknown_inbound`), and CLI
  bad-target → `DecisionConflict` all hold. The `handle_ci_self_transfer_form_key` handler is wired at
  dispatch step_kind 4 (`main.rs:710`) and the E2E (`kat_e2e_ci_classify_inbound_self_transfer_default_basis`)
  drives it end-to-end through persist + reprojection (lot created, `UnknownBasisInbound` cleared, advisory
  present). **Sound.**
- **[item 12] no regression / test integrity** — `kat_tax.rs` diff is purely additive
  (`@@ -2833,3 +2833,626 @@`, zero deletions); no existing KAT modified. +24 tests total (14 core + 3 CLI +
  7 tui-edit) = 946→970. The 4 fault-injections were restored byte-for-byte (clean tree confirmed 4×).
- **[item 13] serde / clippy / dead code** — `self_transfer_in_classify_round_trips_serde` round-trips the
  new variant (supplied + defaults). Old-binary-fails-loud on a vault CONTAINING the variant is the
  spec-accepted forward-only trade-off (`#[serde(default)]` is hygiene, not back-compat). `cargo clippy`
  clean on core/cli/tui-edit/tui. No dead code: the `cycle_basis_source` off-ring arm
  (`form.rs:1366` `SelfTransferInbound => ExchangeProvided`) is compiler-forced exhaustiveness — a
  deliberate, documented, unreachable-by-construction defensive exit (classify-raw never targets a
  self-transfer-in lot), not dead code. The `_ => "?"` at `main.rs:2290` is inside an FmvMissing-guarded
  branch that a self-transfer-in (which never emits `FmvMissing`) cannot reach — cosmetic-only, safe.

---

## Findings

### [M1] Minor — the zero-basis advisory's remediation text omits the void-then-reclassify step required by first-wins
`crates/btctax-core/src/project/fold.rs:993-998` (the `SelfTransferInboundZeroBasis` message).

The advisory reads: *"supply real cost if you have it (btctax reconcile classify-inbound-self-transfer
--basis)."* But `ClassifyInbound` is duplicate-**first-wins** (KAT
`duplicate_classify_inbound_self_transfer_first_wins`): once the event is already classified with the
default basis, simply re-running `classify-inbound-self-transfer --basis <x>` appends a SECOND decision
that is rejected as a `DecisionConflict` — the basis is NOT updated. The correct remedy is to VOID the
first classification and re-classify with `--basis`, exactly as the Income-inbound path's status message
already instructs (`main.rs:2286` "void this decision … and re-classify"). A user who follows the advisory
literally lands on a `DecisionConflict` instead of fixing the basis.

*Why it does not gate:* the default $0 basis is conservative and tax-correct; this is guidance-text
quality, not a wrong tax result or silent gate. The `DecisionConflict` it would provoke is itself a
visible Hard blocker, so the user is not silently misled into an understatement.

*Fix:* align the message with the Income path — e.g. append *"— if this event is already classified, first
void that decision (btctax reconcile void decision|<seq>), then re-classify with --basis."* Note the
current wording was authored verbatim in the R0-GREEN spec (C4/C5, lines 130-133), so an acceptable
alternative is to file it to `FOLLOWUPS.md` rather than block Phase E on it.

### [N1] Nit — optional future-date (`acquired_at > receipt`) warning (G7 / [R0-N3]) not implemented
`crates/btctax-cli/src/main.rs:876-885` and `crates/btctax-tui-edit/src/edit/form.rs:498-513`
(`validate_classify_inbound_self_transfer`).

Neither the CLI dispatch nor the TUI validator warns when the supplied `--acquired` / `acquired_at`
post-dates the receipt. The spec explicitly marks this **optional** ("Not required for correctness — a
future date only makes the lot short-term, the conservative direction"), so its absence is fine; noting
only for completeness. No action required.

---

## Ship gate

**PASS the 0C/0I bar → cleared to ship.** Recommended before merge (process, not review-blocking):
(1) resolve M1 (fix the advisory text or file to `FOLLOWUPS.md`); (2) record the Cycle B (matched
in/out pairs / `SelfTransferPassthrough`) and bulk-classify-inbound-self-transfer follow-ups in
`FOLLOWUPS.md` per the spec's Task 4; (3) run the full workspace suite (controller-owned) to confirm no
cross-crate regression beyond the targeted subset exercised here.
