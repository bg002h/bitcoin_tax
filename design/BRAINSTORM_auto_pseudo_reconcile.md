# BRAINSTORM — auto-pseudo-reconcile + companions (design of record)

**Status: brainstorm COMPLETE, design settled with the user (2026-07-04). NOT yet specced/R0/implemented.**
Decomposed into **3 sequenced sub-projects**, each getting its own `SPEC_* → R0 → TDD → whole-diff → ship`.
Resume at sub-project 1. Umbrella task #36.

## The vision
A one-command way to take an unreconciled ledger (many blockers) to **zero blockers** using
DELIBERATELY-FICTIONAL-but-reasonable default decisions — a clearly-flagged **starting point** the user then
corrects toward the truth. Ties directly to the README disclaimers ("you are the preparer / verify it yourself").

## Cross-cutting decisions (settled with user)
- **Guard = on-screen-only warnings + clean output + attestation gate.** Placeholder `[PSEUDO]` flags + a
  banner show in `verify`/`report`/TUI, but appear in NO output file (CSVs/forms are clean, never watermarked).
  Producing `export-snapshot` or any IRS-form output requires the user to TYPE **"I attest this is true"** —
  **only when the ledger is pseudo-active/tainted** (a fully-real, fully-attested ledger exports with no prompt).
- **Mechanism = reversible MODE + bulk-approve.** `reconcile pseudo on` sets a vault flag; the engine fills
  defaults at PROJECTION time only where no real decision exists → instant 0 classification-blockers + an
  on-screen estimate. Real decisions auto-win (no conflict, no void). `reconcile pseudo off` reverts all.
  Nothing fictional is written to the decision ledger by default. **Bulk-approve** promotes chosen pseudo
  defaults into REAL (attested) decisions in bulk (reuses the existing bulk-reconcile machinery).
- **Fees are de-minimis (pennies).** Do NOT track per-fee basis in rounding-error detail. Pseudo just drops
  the fee sats from holdings; basis stays with the coins the user still possesses; NO re-homing math. (A
  network fee to take/keep custody is a cost of holding, not a taxable disposition — no gain/loss on the fee.)
- **A Sell (BTC→USD) is a taxable disposition, NOT a self-transfer.** Assumption 2 ("all outbound = self-
  transfer") applies ONLY to `TransferOut` withdrawals (BTC leaving to an address), never to exchange Sells
  (which import as `Dispose` events already). Sell gain uses the attested cost-basis method (sub-project 1).
- **Aggressive scope.** Pseudo clears EVERYTHING it can with default guesses — incl. import/decision conflicts
  (accept-first) and a placeholder tax profile — to reach truly 0 blockers with zero setup. (All flagged.)

## The 5 assumptions → pseudo defaults (sub-project 2)
| Assumption | Pseudo default (applied only where no real decision exists) |
|---|---|
| 1. Basis $0 when unknown | unknown-basis inbound → `ClassifyInbound(SelfTransferMine)` at $0 basis |
| 2. All outbound transfers = self-transfer | every `TransferOut` → non-taxable self-transfer → no disposal/gain (Sells excluded — they're already disposals) |
| 3. No mining income | inbounds never auto-classified as income (they're self-transfers) |
| 4. Interest can be BTC or USD | pseudo creates NO income; income classification (user's later corrections) accepts BTC-denominated (inbound→interest at FMV) or USD interest |
| 5. Fees disappear sats | de-minimis: drop fee sats, basis stays with held coins, no re-homing |
Net effect: a "null-hypothesis ledger" — all movement non-taxable, ~zero tax owed → obviously a placeholder.

---

## SUB-PROJECT 1 [FIRST] — per-exchange cost-basis method election + attestation
**Why first:** foundational (pseudo's taxable Sells need an attested method) AND standalone-useful under the
IRS 2025+ per-account broker rules.

**Current model (verified):** `MethodElection { effective_from: TaxDate, method: LotMethod }` (`LotMethod =
Fifo|Lifo|Hifo`) is a GLOBAL forward standing order that already resolves per-wallet disposals; the election
itself is not wallet-scoped. `WalletId::Exchange { provider, account }` (identity.rs:110-111) is the key.
`method_election_is_forward` guard (resolve.rs:16) + `MethodElectionBackdated` blocker prevent back-dating.

**Design:**
- **Model:** add an optional scope → `MethodElection { effective_from, method, wallet: Option<WalletId> }`.
  `None` = global default (today's behavior, untouched); `Some(wallet)` = that **exchange ACCOUNT**'s method
  (granularity = full `WalletId::Exchange{provider, account}`, matching the IRS per-account broker election).
  [Serde: an ADDED optional field must default to `None` on old events — additive, back-compatible.]
- **Precedence** (fold-time, disposal on wallet W, date D): latest-in-force election scoped to **W** →
  else latest **global** election → else **FIFO**. Per-disposal `LotSelection` still overrides everything;
  `pre2025_method` still governs the pre-2025 residue. Extend the resolve.rs per-wallet method resolution to
  consult wallet-scoped elections before the global one.
- **Attestation:** setting a per-exchange method IS an attested, timestamped election event (the user affirms
  "I use/elected `<method>` for `<exchange account>`"). Light touch (a forward election, updatable — NOT the
  irrevocable safe-harbor-attest typed-word flow). The election event is the attestation of record.
- **Surfaces:** CLI — extend to `reconcile method-election [--exchange <provider/account>] --method <fifo|hifo|lifo>`
  (the no-`--exchange` form = the existing global election). btctax-tui-edit — a flow to LIST the vault's
  exchange accounts + view/set/attest each one's method (a new modal/keybinding, mirroring the reconcile flows).
- **Safety/KATs:** per-wallet election governs only that wallet's disposals; global unaffected wallets still
  FIFO/global; backdating still blocks; NFR4 determinism (latest by `(effective_from, decision_seq)`); a KAT
  proving two exchanges with different methods produce different (correct) gains on the same-shaped disposals.
- **SemVer:** new optional serde field + a new CLI flag/arg + a TUI flow. Additive. Lockstep: GUI schema_mirror
  if a clap flag name changes; man pages regen (`make docs`); the `MethodElection` doc-comments.

---

## SUB-PROJECT 2 — pseudo-reconcile mode
Reversible vault MODE that fills the §"5 assumptions" defaults at projection time where no real decision
exists → 0 classification-blockers + on-screen flagged estimate; aggressive scope (default-guess conflicts +
placeholder profile); `[PSEUDO]` flags in verify/report/TUI (never in output); `reconcile pseudo on/off`;
**bulk-approve** promoting chosen defaults to real decisions (reuses bulk machinery). Sells use sub-project-1's
attested method (defaulting to FIFO per exchange until attested). Uses TP8-c/de-minimis fee handling.
Tax-safety: the mode must NEVER let pseudo output be mistaken for real — enforced by the attest gate (sub-3).
KATs incl. fault-injection that pseudo defaults are flagged everywhere on-screen and absent from every output.

## SUB-PROJECT 3 — attestation export gate
Producing `export-snapshot` / any IRS-form output when the ledger is pseudo-active/tainted requires typing
**"I attest this is true"** (a typed-phrase gate, mirroring the safe-harbor-attest typed-word pattern). A
fully-real, fully-attested ledger (no pseudo defaults in play) exports with no prompt. Cross-cutting: needs a
"is any pseudo default contributing to this output?" signal from sub-project 2.

---

## Resumption
Next action: `SPEC_cost_basis_method_election.md` (sub-project 1) → R0 (2 rounds to 0C/0I) → TDD → whole-diff →
ship; then sub-project 2, then 3. All cross-cutting decisions above are settled with the user — do NOT re-ask.
Memory: [[auto-pseudo-reconcile-roadmap]]. See the user's original framing + the 5 assumptions above.
