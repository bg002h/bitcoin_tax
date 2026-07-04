# SPEC — per-exchange cost-basis method election + attestation (sub-project 1 of auto-pseudo-reconcile)

**Source baseline:** `main` @ `fa675bb` (branch `feat/cost-basis-method-election`). **Review status: DRAFT —
awaiting R0 (2 rounds to 0C/0I).** Design of record: `design/BRAINSTORM_auto_pseudo_reconcile.md`; roadmap
memory `auto-pseudo-reconcile-roadmap`. **All cross-cutting decisions are settled with the user — do NOT
re-brainstorm.**

## Goal
Let the user declare + attest a cost-basis lot method (FIFO/HIFO/LIFO) **per exchange ACCOUNT**, matching what
they elected with each broker (IRS 2025+ per-account rule). Extends the existing GLOBAL forward `MethodElection`
to an optional per-wallet scope. Foundational for sub-project 2 (pseudo Sells need an attested method); also
standalone-useful.

## Current model (verified at baseline)
- `MethodElection { effective_from: TaxDate, method: LotMethod }` (`event.rs:245`, serde-derived).
  `LotMethod = Fifo(default)|Lifo|Hifo`. A GLOBAL forward standing order.
- `resolve.rs`: collects non-voided, non-backdated elections into `Resolution.elections: Vec<ElectionRec>`
  (`ElectionRec { effective_from, method }`); backdating → `MethodElectionBackdated` blocker
  (`method_election_is_forward`, resolve.rs:16).
- `fold.rs:30 applicable_method(date, ctx) -> LotMethod`: latest-in-force election with `effective_from ≤ date`
  by the `(effective_from, decision_seq)` total order (NFR4); empty ⇒ `Fifo`. Called at `fold.rs:60` per
  disposal. Threaded via `FoldCtx.elections: &[ElectionRec]`.
- `WalletId::Exchange { provider: String, account: String }` (`identity.rs:111`) — the per-exchange key. Every
  disposal already resolves on a wallet.
- Set today via `config --set-forward-method <m> [--effective-from <d>]` → `cmd::reconcile::set_forward_method`
  (main.rs:239). `LotSelection` (specific-ID, §A.4) still overrides per-disposal; `pre2025_method` governs the
  pre-2025 residue (untouched here).

## Design
### 1. Model — add an optional wallet scope (additive/back-compatible)
- `MethodElection { effective_from, method, #[serde(default)] wallet: Option<WalletId> }`. **`#[serde(default)]`
  is mandatory** so pre-existing on-disk events (no `wallet` field) deserialize as `wallet: None` = a global
  election (today's exact behavior). `None` = global; `Some(WalletId::Exchange{provider,account})` = that
  account's method. (Only `Exchange` wallets are electable — reject/ignore a non-Exchange scope at construction.)
- `ElectionRec { effective_from, method, wallet: Option<WalletId> }` (carry the scope through resolve→fold).

### 2. Resolution precedence (the core rule)
`applicable_method(date, wallet, ctx)` — for a disposal on wallet `W`, date `D`:
1. latest-in-force election with `wallet == Some(W)` and `effective_from ≤ D` (by `(effective_from,
   decision_seq)`), else
2. latest-in-force GLOBAL election (`wallet == None`, `effective_from ≤ D`), else
3. `Fifo`.
Signature changes `applicable_method(date, ctx)` → `applicable_method(date, wallet, ctx)`; `fold.rs:60` passes
the disposal's wallet. **`LotSelection` still overrides everything per-disposal**; `pre2025_method` unchanged.
Backdating guard applies to BOTH scoped + global elections (a per-wallet election is still a forward standing
order — `effective_from ≥ TRANSITION_DATE ∧ ≥ made-date`, else `MethodElectionBackdated`).

### 3. Attestation
Setting a per-exchange method IS the attestation: a user-made, timestamped `MethodElection` event affirming "I
use/elected `<method>` for `<provider/account>`." **Light touch** — a forward election the user can update going
forward (NOT the irrevocable safe-harbor-attest typed-word flow). The event is the attestation of record; no
separate attestation table. (The CLI/TUI confirmation text states it's an attestation.)

### 4. CLI surface
Extend the election path with an optional exchange scope:
`btctax config --set-forward-method <fifo|hifo|lifo> [--exchange <provider/account>] [--effective-from <d>]`.
- `--exchange` value = `<provider>/<account>` (impl defines + validates the exact delimiter; VALIDATE against
  the vault's known Exchange wallets — reject an unknown provider/account loudly so a typo can't silently
  create a dead election). No `--exchange` = the existing global election (unchanged).
- `cmd::reconcile::set_forward_method` gains an `Option<WalletId>` param; the arm parses/validates `--exchange`.

### 5. btctax-tui-edit flow
A new editor flow mirroring the existing reconcile flows (`ClassifyInboundFlowState`/`ModalState`/`Step`
pattern, main.rs). New keybinding (pick a free key; document in the `?` overlay + man page). Flow:
- List the vault's **Exchange accounts** (distinct `WalletId::Exchange` from the events) with each one's
  **currently-resolved** method (scoped election → global → FIFO) + whether it's explicitly elected vs inherited.
- Select an account → choose FIFO/HIFO/LIFO → confirm ("attest") → append a scoped `MethodElection`.
- Single save; empty-guard; mid-batch rollback if multiple set at once (mirror the bulk-flow persistence).

## KATs (tax-critical)
- `per_wallet_method_governs_only_that_wallet` — Coinbase=HIFO election + Gemini disposals still FIFO/global;
  same-shaped disposals on the two wallets yield the correct (different) gains.
- `scoped_beats_global_beats_fifo` — with a global LIFO + a Coinbase HIFO election, Coinbase disposals use
  HIFO, all others LIFO; with neither, FIFO.
- `lot_selection_still_overrides_scoped_election` — a per-disposal `LotSelection` wins over the wallet election.
- `scoped_election_backdating_blocks` — a per-wallet election with `effective_from < made-date` (or pre-2025)
  → `MethodElectionBackdated`.
- `serde_backcompat_old_methodelection_loads_as_global` — an on-disk `MethodElection` WITHOUT `wallet`
  deserializes to `wallet: None` and behaves exactly as today (pin with a fixed JSON fixture).
- `determinism_two_scoped_elections_latest_wins` — `(effective_from, decision_seq)` total order per wallet.
- CLI: `config_set_forward_method_exchange_scoped` (appends a scoped election; unknown exchange rejected);
  TUI: `method_election_flow_sets_and_attests_per_account` (TestBackend snapshot).

## Scope / SemVer / lockstep
btctax-core (model + fold + resolve) + btctax-cli (CLI arm) + btctax-tui-edit (flow). **Additive serde field**
(`#[serde(default)]`) — old vaults load unchanged. New CLI arg + TUI flow. **PATCH-class** (additive; no
behavior change when no scoped election exists). Lockstep: GUI `schema_mirror` if any clap flag NAME changes
(here only an added `--exchange` arg — confirm mirror policy); regen man pages (`make docs`); update the
`MethodElection` doc-comments (event.rs) + the `?` overlay keymap.

## Plan (TDD)
- **Task 1** — model: add `wallet: Option<WalletId>` to `MethodElection` (serde-default) + `ElectionRec`;
  serde-backcompat KAT.
- **Task 2** — resolve + fold: collect the scope; `applicable_method(date, wallet, ctx)` precedence; the
  governance/precedence/backdating/determinism KATs.
- **Task 3** — CLI `--exchange` scope (parse + validate against known wallets); CLI KAT.
- **Task 4** — btctax-tui-edit flow (list accounts + set/attest); TUI KAT; `?`-overlay + man-page update.
- **Task 5** — whole-diff review + full suite + `make docs` + FOLLOWUPS.

## Gotchas
- **`#[serde(default)]` is load-bearing** — without it, every existing vault fails to load (missing field).
  Pin with a real old-JSON fixture KAT.
- **Precedence order is tax-critical** — scoped MUST beat global MUST beat FIFO; `LotSelection` still beats all.
  Fault-inject each layer.
- **Validate `--exchange`** against the vault's actual Exchange wallets — a silent dead election (typo'd
  provider) would mislead the user into thinking a method is in force when it isn't.
- **Backdating applies to scoped elections too** — a per-wallet election is still a forward standing order.
- **Only `Exchange` wallets are electable** (not Cold/On-chain) — a method election is a brokerage-account
  concept.
