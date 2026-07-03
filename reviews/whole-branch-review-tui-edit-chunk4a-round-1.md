# Whole-branch review — tui-edit-chunk4a (Phase E, round 1)

**Branch:** `feat/tui-edit-chunk4` @ `caecf4c` (diff `main..HEAD`, main == `755e47c`; commits: spec
`ef6e2b9`, Task 1 `188fc08`, Task 2 `caecf4c`). Delegated-implementer cycle; this is the independent gate.
**Spec:** `design/SPEC_tui_edit_chunk4a.md` (R0-GREEN, 2 rounds).
**Controller-verified full gate at HEAD:** 906 workspace tests, clippy `-D warnings` clean, fmt clean.
The large `main.rs` numstat (~9k/7k) was verified a git diff-ALIGNMENT ARTIFACT — only 8 content lines
truly removed (old import-block wrappings); every `main` fn survives at HEAD; two spot-checked existing
KATs byte-identical; `#[test]` 88→103 (+15).

## Controller fold disposition
- **[WB4a-1] Minor** — FOLLOWUPS.md not yet updated with the spec Task-3 out-of-scope items → **FOLD**
  (docs-only; the Task-3 deliverable — record before ship).
- **[WB4a-2] Nit** — `derive_classify_raw_status` arm 2 message unconditional (correct for the shipped
  Acquire/Income variants; doc-scoped) → no action.
- **[WB4a-3] Nit** — TargetPick reachable with both target lists empty (no status hint; edge-case UX;
  CLI `--to-wallet` is the documented escape) → recorded as a minor UX-polish FOLLOWUP; no code change.

## Reviewer output (verbatim)

# Whole-Diff Review (Phase E) — tui-edit-chunk4a (link-transfer + classify-raw)

**Verdict: 0 Critical / 0 Important / 1 Minor / 2 Nit — SHIP.**

The two flows deliver the spec's guarantees. Both persist fns are the exact single-append `snapshot → append_decision → save_or_rollback` shape with no bespoke latch; pre-filters are conservative supersets of the engine's DecisionConflict backstops; the structured builders are byte-accurate against `event.rs`; status derivation and revocability are correct; `btctax-core`/`btctax-cli` are untouched (empty diff). The numstat is confirmed a diff-alignment artifact — the `open_safe_harbor_attest_flow` fn "deleted" in the diff is byte-identical to HEAD (just displaced by the inserted flows), every `main.rs` fn from `main` survives in HEAD (no rename/removal), and two spot-checked existing KATs are byte-identical. `#[test]` went 88→103 (+15). KAT-G1 passes; clippy on the crate is clean.

### Verification performed
- **Pre-filters vs engine (`resolve.rs:485-519`):** out-list = `pending_reconciliation`; a linked out projects to `Op::SelfTransfer` (`resolve.rs:201-217`) which does NOT push `pending_reconciliation` (`fold.rs:729` vs `:742`) → no double-offer, prevents dup-out (a). In-list requires `e.wallet.is_some()` (prevents (c)) AND excludes non-voided `TransferLink::InEvent` targets (prevents (b)). Wallet-list is the `BTreeSet` union of all distinct `snap.events[].wallet` Some-values — not `holdings_by_wallet` (the R0-I2 catch), pinned by `kat_lt_wallet_list_includes_zero_balance_wallet` (asserts kraken is offerable while absent from `holdings_by_wallet`).
- **Builders struct-accurate:** `Acquire{sat,usd_cost,fee_usd,basis_source}` (no acquired-at), `Income{sat,usd_fmv,fmv_status,kind,business}` built directly (not via `InboundClass`). fmv mapping: typed → `ManualEntry`, empty → `None`+`Missing` (matches `resolve.rs:187` discard). `basis_source` is a real 8-variant `BasisSource` pick; both built payloads satisfy `is_imported()`. Only Acquire+Income offered (`kat_cr_only_acquire_income_offered`).
- **Status + revocability:** both `TransferLink` and `ClassifyRaw` are in `is_revocable_payload` (void works). CLI-pointing arms use quit-first wording. classify-raw arm 3 (`FmvMissing`) verified live by the empty-FMV KAT.
- **E2E SelfTransfer proxy is SOUND:** the wallet-target E2E asserts kraken holds 600K (100K acq + 500K relocated). Lot relocation carrying basis to a destination wallet is reachable only through `Op::SelfTransfer` — no other Op relocates. The in-event E2E's 500K-at-kraken is likewise dispositive. Combined with gone-from-pending + absent-from-disposals + select-lots reconstruction, no failure mode is missed.
- **Existing coupling resolved, not just reachable:** chunk-3 Task 3 already implemented the SelfTransfer select-lots reconstruction; `kat_e2e_lt_wallet_target` confirms the linked out appears as a `DisposalKind::SelfTransfer` row. Select-lots correctly not widened further.

### Fault-injection (all RED, tree restored byte-for-byte — `git status` clean)
1. `kat_e2e_lt_wallet_target` — neutered the Wallet-mode target to a bogus wallet → `left: Some(100000), right: Some(600000)` at `main.rs:13603` (relocation went elsewhere). Pins target-wallet propagation.
2. `kat_e2e_cr_income` — mapped typed-FMV branch to `FmvStatus::Missing` → status became "FmvMissing now applies" not "cleared" (`main.rs:14274`). Pins the ManualEntry mapping.
3. `kat_cr_income_empty_fmv_missing_arm` — mapped empty branch to `(Some(ZERO), ManualEntry)` → `usd_fmv left: Some(0), right: None` (`main.rs:14348`). Pins empty→Missing+FmvMissing.

### [WB4a-1] MINOR — FOLLOWUPS.md not yet updated with the spec Task-3 out-of-scope items
Record (a) classify-raw `Dispose/TransferOut/TransferIn/Unclassified` variant parity; (b) link-transfer to a brand-new wallet never seen in any event (CLI `--to-wallet` fallback, R0-I2). Part of Task 3 (this review's fold); not a code defect.

### [WB4a-2] NIT — `derive_classify_raw_status` arm 2 message is unconditional
Correct for the two shipped variants (Acquire never blocks; Income blocks only via `FmvMissing`); doc-scoped. No action.

### [WB4a-3] NIT — TargetPick reachable with both target lists empty
Edge-case UX only (Enter is a graceful no-op; Esc exits); within spec. Optional: a "no link targets available" status.

**Ship gate: PASS (0C/0I).**
