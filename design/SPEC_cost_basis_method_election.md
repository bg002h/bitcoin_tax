# SPEC — per-exchange cost-basis method election + attestation (sub-project 1 of auto-pseudo-reconcile)

**Source baseline:** `main` @ `fa675bb` (branch `feat/cost-basis-method-election`). **Review status: R0-GREEN
(2 rounds; 0C/0I). Reviews: `reviews/R0-spec-cost-basis-method-election-round-{1,2}.md`. Cleared to implement.**
Round 2 verified all folds + confirmed no third resolution site; folded 2 impl Minors (tier-1 respects
`effective_from ≤ D`; shared resolver returns `Option<&ElectionRec>`). Headline risks came back CLEAN: serde/fingerprint back-compat SAFE (decision payloads
return `None` from `persistence::fingerprint`, `EventId=f("decision",seq)` is payload-independent; precedent
`#[serde(default)] pre2025_method` event.rs:188); precedence reachable (single caller `consume_principal`
fold.rs:60, wallet bound at every arm). Key folds: **[I1] `compliance.rs` is a SECOND election-resolution site
— scope it too**; **[I2] reuse the canonical `parse_wallet_id` grammar** (`exchange:PROVIDER:ACCOUNT`), do NOT
invent `/`. Design of record: `design/BRAINSTORM_auto_pseudo_reconcile.md`. **Cross-cutting decisions settled —
do NOT re-brainstorm.**

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
Signature changes `applicable_method(date, ctx)` → `applicable_method(date, wallet, ctx)`; `fold.rs:60`
(`consume_principal`, the single caller — wallet already bound at every arm) passes the disposal's wallet.
**`LotSelection` still overrides everything per-disposal**; `pre2025_method` unchanged. Backdating guard applies
to BOTH scoped + global elections (a per-wallet election is still a forward standing order — `effective_from ≥
TRANSITION_DATE ∧ ≥ made-date`, else `MethodElectionBackdated`).
- **[R0-M2 — TWO SEPARATE TIERS, not one merged max]** the scoped tier and the global tier are resolved
  INDEPENDENTLY: a later-dated GLOBAL election does NOT override an in-force SCOPED one for its wallet (step 1
  is decided purely among `wallet==Some(W)` elections; only if that set is empty do we fall to step 2). Do NOT
  implement as a single `max_by` over all elections merged — that would let a fresh global election silently
  flip a wallet the user scoped. Fault-inject this.
- **[R0-r2-M1 — tier 1 respects `effective_from ≤ D`]** a scoped election that is NOT YET effective (`effective_from
  > D`) must NOT suppress an in-force GLOBAL election: tier 1 selects among `wallet==Some(W)` elections *with
  `effective_from ≤ D`*; if none qualify, fall to tier 2 (global), NOT straight to FIFO. KAT:
  `not_yet_effective_scoped_falls_to_global`.
- **[R0-r2-M2 — resolver return contract]** the shared resolver returns `Option<&ElectionRec>` (the winning
  election, or `None` ⇒ FIFO) — NOT a FIFO-defaulted `LotMethod` — because `disposal_compliance` needs both
  existence AND `effective_from` to emit `StandingOrder{effective_from}`. `fold::applicable_method` maps
  `None ⇒ Fifo`.
- **[R0-I1 — `compliance.rs` is a SECOND resolution site — MUST use the same rule]** `disposal_compliance`
  (`compliance.rs:47-67,169-180`) independently collects elections (no wallet) and picks the standing order as a
  GLOBAL max — so a scoped `Coinbase→HIFO` election would falsely tag a `Gemini` disposal as `StandingOrder` in
  `verify` (render.rs:540), over-reporting §A.5(a). Fix: extract a SHARED wallet-aware resolver used by BOTH
  `fold::applicable_method` AND `compliance` (compliance already builds `wallet_of` at :105-108). One rule, two
  callers — never two divergent implementations.

### 3. Attestation
Setting a per-exchange method IS the attestation: a user-made, timestamped `MethodElection` event affirming "I
use/elected `<method>` for `<provider/account>`." **Light touch** — a forward election the user can update going
forward (NOT the irrevocable safe-harbor-attest typed-word flow). The event is the attestation of record; no
separate attestation table. (The CLI/TUI confirmation text states it's an attestation.)

### 4. CLI surface
Extend the election path with an optional exchange scope:
`btctax config --set-forward-method <fifo|hifo|lifo> [--exchange <exchange:PROVIDER:ACCOUNT>] [--effective-from <d>]`.
- **[R0-I2] `--exchange` uses the CANONICAL wallet grammar `eventref::parse_wallet_id`** (`exchange:PROVIDER:ACCOUNT`,
  eventref.rs:57-74) — do NOT invent a `/` delimiter (accounts can contain `/`; forking the grammar mis-parses).
- **[R0-M3] reject the `self:LABEL` form** (only `exchange:` wallets are electable — a method election is a
  brokerage-account concept). VALIDATE the parsed `WalletId` against the vault's known Exchange wallets
  (enumerate distinct `WalletId::Exchange` from the loaded events) — reject an unknown one LOUDLY so a typo
  can't silently create a dead election. No `--exchange` = the existing global election (unchanged).
- `cmd::reconcile::set_forward_method` gains an `Option<WalletId>` param; the arm parses via `parse_wallet_id`
  + validates. **[R0-M1] the scope lives in the `MethodElection` PAYLOAD**, not the `LedgerEvent.wallet` column
  (`append_decision` passes `wallet=None` for decisions).

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
- **[R0-I1] `scoped_election_does_not_taint_compliance_of_other_wallets`** — a `Coinbase→HIFO` election; a
  `Gemini` disposal's `verify` compliance is NOT `StandingOrder` on account of it (the shared resolver scopes
  correctly in `disposal_compliance`).
- **[R0-M2] `later_global_does_not_override_in_force_scoped`** — scoped `Coinbase→HIFO` (Jan) then a LATER
  global `LIFO` (Mar): a Coinbase disposal in Apr still uses HIFO (two independent tiers, not a merged max).
- **[R0-M2] gaps:** `two_accounts_same_provider_independent` (exchange:coinbase:A=HIFO vs :B=FIFO);
  `voided_scoped_election_falls_back` (void → wallet reverts to global/FIFO);
  `pre2025_residue_plus_post2025_scoped_election` (a scoped election on a wallet with pre-2025 residue —
  `pre2025_method` still governs the residue lots; the scoped method governs post-2025 disposals).
- CLI: `config_set_forward_method_exchange_scoped` (appends a scoped election; unknown exchange rejected);
  TUI: `method_election_flow_sets_and_attests_per_account` (TestBackend snapshot).

## Scope / SemVer / lockstep
btctax-core (model + fold + resolve + **compliance.rs** [R0-I1] + a shared wallet-aware resolver) + btctax-cli
(CLI arm) + btctax-tui-edit (flow). **Additive serde field** (`#[serde(default)]`) — old vaults load unchanged.
New CLI arg + TUI flow. **PATCH-class** (additive; no behavior change when no scoped election exists).
**[R0-N1] NO GUI `schema_mirror`** — there is no GUI/tauri crate in the tree (verified). Real lockstep = regen
man pages (`make docs`), the `?`-overlay keymap, and the `MethodElection` doc-comments (event.rs).

## Plan (TDD)
- **Task 1** — model: add `wallet: Option<WalletId>` to `MethodElection` (serde-default) + `ElectionRec`;
  serde-backcompat KAT.
- **Task 2** — resolve + fold + **compliance** [R0-I1]: collect the scope; extract ONE shared wallet-aware
  resolver (two independent tiers — scoped, then global; [R0-M2]) used by `fold::applicable_method(date,
  wallet, ctx)` AND `disposal_compliance`; the governance/precedence/backdating/determinism/compliance KATs
  (incl. later-global-doesn't-override-scoped + the compliance-taint KAT).
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
