# R0 — SPEC review (round 1): per-exchange cost-basis method election

**Artifact:** `design/SPEC_cost_basis_method_election.md` (DRAFT)
**Branch/commit:** `feat/cost-basis-method-election` @ `3fd619c` (main == `fa675bb`)
**Reviewer role:** independent architect (R0, read-only). **Bar:** 0 Critical / 0 Important.
**Design of record:** `design/BRAINSTORM_auto_pseudo_reconcile.md` (settled decisions NOT relitigated).

## Verdict: 0 Critical / 2 Important / 3 Minor / 2 Nit — NOT green (fold before implementation)

The two headline back-compat/implementability questions come back **clean**: the serde add is genuinely
additive (no fingerprint/ID perturbation), and the precedence rule is correct and reachable at the fold call
site. The two Important findings are both *omissions of a second code path*: (I1) `compliance.rs` resolves
elections independently of the fold and the spec's file-scope list leaves it out — a scoped election would
bleed into an un-elected wallet's `verify` compliance tag; (I2) the CLI invents a `/` delimiter when a
canonical `exchange:PROVIDER:ACCOUNT` grammar + parser already exist.

---

## Answers to the starred questions (affirmative parts)

**[★Q1] Precedence rule — correct + implementable? YES.**
- `applicable_method` is called from exactly ONE site, `consume_principal` (fold.rs:60), and every caller of
  `consume_principal` binds `wallet` before the call: `Op::Dispose` (fold.rs:576-587/595), `Op::SelfTransfer`
  (fold.rs:742-756), `Op::GiftOut` (fold.rs:1022-1042), `Op::Donate` (fold.rs:1098-1118). Threading a
  `wallet: &WalletId` param into `consume_principal` → `applicable_method` is mechanical; the wallet is
  already in scope at each site. (The optimizer reuses this exact path via `pools_before`/`state_as_of` →
  `fold_event`; it does NOT re-resolve the method — grep of `optimize.rs`/`evaluate.rs` finds no independent
  election logic — so no extra threading is needed there.)
- `(effective_from, decision_seq)` is available per election: `ElectionRec` already carries `decision_seq: u64`
  (resolve.rs:142-146), populated from the collection loop's `(seq, d)` (resolve.rs:843, 857-861). `decision_seq`
  is a unique u64, so the two-pass filter (scoped `wallet == Some(W)` max, else global `wallet.is_none()` max,
  else FIFO) is fully deterministic (NFR4) — `max_by` ties cannot occur.
- Caveat: this is the fold's resolution site. There is a SECOND independent resolution site the spec omits —
  see [I1].

**[★Q2] Serde / fingerprint back-compat — SAFE, not Critical.**
- `persistence::fingerprint(&EventPayload) -> Option<Fingerprint>` returns `None` for every decision/system
  payload via the catch-all `_ => return None` (persistence.rs:96); `MethodElection` is a decision variant, so
  it is never fingerprinted. This is pinned by an EXISTING test: `method_election_decision_has_no_fingerprint`
  asserts `fingerprint(&me).is_none()` (event.rs:547-553). Adding a field cannot perturb any fingerprint.
- Decision `EventId = f("decision", seq)` (identity.rs:69, canonical at :103) — payload-independent; an added
  field cannot change any election's id or dedup.
- The payload is stored as `payload_json TEXT` via `serde_json::to_string(&ev.payload)` (persistence.rs:165)
  and reloaded via `serde_json::from_str` (persistence.rs:290). An added `#[serde(default)] wallet:
  Option<WalletId>` therefore loads old rows (no field) as `None` = today's global behavior.
- `WalletId` derives `Serialize, Deserialize` (identity.rs:109). Direct precedent for the pattern:
  `#[serde(default)] pub pre2025_method: LotMethod` (event.rs:188), `#[serde(default)] pub kind:
  Option<IncomeKind>` (event.rs:228). Keep the spec's pinned old-JSON fixture KAT; it is the right guard.

**[★Q3] Backdating + pre-2025 — clean.**
- `method_election_is_forward(me, made)` reads ONLY `me.effective_from` (resolve.rs:16-18) — payload-shape
  agnostic; the added field cannot affect it. The collection loop runs the guard for ALL `MethodElection`
  decisions regardless of scope (resolve.rs:847-856), so scoped elections are guarded automatically.
- No conflict with `pre2025_method` / safe-harbor: pre-2025 disposals route through `PoolKey::Universal` and
  use `ctx.config.pre2025_method` (fold.rs:31-32), ignoring elections entirely. A wallet-scoped election
  (forced `effective_from >= TRANSITION_DATE` by the guard) can only govern post-2025 disposals on that
  wallet; it never touches the pre-2025 residue or a `SafeHarborAllocation`. (Add the confirming KAT — M2d.)

**[★Q4] CLI/validation — see [I2] (delimiter) + [M3] (Exchange-only + known-wallet validation). No GUI
`schema_mirror` exists in this repo — see [N1].**

**[★Q5] Completeness / tax-safety — "only Exchange electable" is enforceable + right (M3); KAT gaps in M2.**

---

## Findings

### [I1] IMPORTANT — `compliance.rs` resolves elections globally; the spec's file-scope omits it → a scoped election bleeds a false `StandingOrder` tag onto un-elected wallets

`disposal_compliance` (compliance.rs:91) is a SECOND, independent election-resolution site that the spec's
scope list ("btctax-core (model + fold + resolve)", spec line 85) does not mention. It:
- collects elections with **no wallet field** — the internal `Election` struct is `{ effective_from,
  decision_seq }` only (compliance.rs:37-40, 47-67); and
- picks the standing order as a GLOBAL max over ALL elections, ignoring the disposal's wallet
  (compliance.rs:169-180).

Concretely: with a scoped `Coinbase/personal → HIFO` election and a disposal on `Gemini/main` (no election),
the classifier's step (3) still finds the Coinbase election in-force and returns
`StandingOrder { effective_from }` for the Gemini disposal. This is rendered to the user: `disposal_compliance`
is called in `render.rs:540` and `StandingOrder` prints an `effective_from` compliance tag
(`compliance_status_tag`, render.rs:141-148). So the fold would compute the correct gain (FIFO for Gemini),
but `verify` would FALSELY report the Gemini disposal as covered by a standing order — over-reporting §A.5(a)
compliance and understating the user's audit exposure. Direction of error is toward *false comfort*.

The fix is available and cheap — `disposal_compliance` already builds `wallet_of: disposal_event → WalletId`
(compliance.rs:105-108). Mirror the fold's precedence: filter `elections` to the disposal's wallet scope
(`wallet == Some(W)`), else global (`wallet.is_none()`), else none. Add `wallet: Option<WalletId>` to the
internal `Election` struct and carry it in `collect_elections`.

**Fix:** Add `crates/btctax-core/src/project/compliance.rs` to the spec's scope (§Scope/lockstep and the Plan —
this belongs in Task 2 alongside `applicable_method`). Add a KAT: scoped election on wallet A + post-election
disposal on wallet B (no election) ⇒ B's compliance is `NonCompliant` (or its own tier), NOT `StandingOrder`.

### [I2] IMPORTANT — CLI `--exchange <provider/account>` invents a fragile `/` delimiter; a canonical `exchange:PROVIDER:ACCOUNT` grammar + parser already exist and should be reused

The spec (line 56-59) and brainstorm (line 64) both write `--exchange <provider/account>` and defer the
delimiter to impl ("impl defines + validates the exact delimiter"). But there is already ONE canonical
CLI wallet grammar and parser: `eventref::parse_wallet_id` — `exchange:PROVIDER:ACCOUNT | self:LABEL`
(eventref.rs:57-74), using `splitn(3, ':')`. It is the format the rest of the CLI/render uses
(render.rs:185 emits `exchange:{provider}:{account}`; the LotId/LotPick parsers deliberately reuse the same
`|`/`#`/`:` grammar, eventref.rs:86-119). Inventing a second `/`-delimited grammar:
- **breaks on real data**: a provider or account containing `/` (e.g. account `taxable/2024`) either
  mis-parses or, worse, is REJECTED as an "unknown wallet" by the spec's own known-wallet validation — the
  exact "silent dead election / typo mis-fire" hazard the spec's Gotchas call out (spec line 105-106); and
- **splits the CLI's wallet-string grammar** into two incompatible forms for no benefit.

**Fix:** Specify reuse of `eventref::parse_wallet_id` (so `--exchange exchange:coinbase:personal`), OR — if a
provider/account shorthand is genuinely wanted — pin it in the spec with `splitn(2, ...)` semantics and state
which side may contain the delimiter. Either way, make it a design-of-record decision, not impl discretion,
and reject the `self:LABEL` form for `--exchange` (see M3). Recommend reusing `parse_wallet_id` outright.

### [M1] MINOR — make explicit that the scope lives in the `MethodElection` PAYLOAD, not the `LedgerEvent.wallet` column; name `consume_principal` as the signature that threads the wallet

Two distinct `Option<WalletId>` fields exist and are easy to conflate:
- `LedgerEvent.wallet` — persisted in the `wallet_json` column (persistence.rs:164) and surfaced as
  `Eff.wallet` (resolve.rs:109/833); for a *decision* event it is unused by the fold (decisions are not pushed
  onto the timeline — resolve.rs:810-813 `_ => continue`). `append_and_save` → `append_decision` hard-codes
  `wallet: None` for elections (reconcile.rs:28-36; :243).
- the NEW `MethodElection.wallet` — the election's SCOPE, which resolve.rs must read from `d.payload`
  (resolve.rs:847) when building `ElectionRec`.

The spec (line 31-35) puts the field on `MethodElection` — correct — but doesn't call out the trap. Also, the
spec says "`fold.rs:60` passes the disposal's wallet"; fold.rs:60 is inside `consume_principal`, so it is
`consume_principal`'s signature that gains the `wallet` param (then forwards to `applicable_method`).

**Fix:** In the spec's §1/§2, state: scope is stored in the `MethodElection` payload (never the event-wallet
column); `append_decision`'s `wallet` param stays `None` for elections; `consume_principal` gains the `wallet`
param and forwards it.

### [M2] MINOR — KAT set has real tax-relevant gaps

The listed KATs are good but miss cases the fold's tiering can get subtly wrong:
- **(a) same provider, two accounts** — the `per_wallet_method_governs_only_that_wallet` KAT uses Coinbase vs
  Gemini (different providers). Add `Coinbase/personal` (HIFO) vs `Coinbase/business` (no election) so a
  provider-keyed (not account-keyed) mis-impl is caught. `WalletId::Exchange` Eq is over BOTH fields
  (identity.rs:111), so a correct `wallet == Some(W)` handles it — but the guard is cheap insurance.
- **(b) tiering vs "newer-wins" merge** — a GLOBAL election with a LATER `effective_from`/higher `decision_seq`
  than an in-force SCOPED election must NOT override the scoped one (strict tiers, spec line 39-42). A naive
  single-pass `max_by` over all elections with scope as a tiebreak would be tax-WRONG here. Add: scoped
  `W→HIFO` (eff 2025-02) + later global `LIFO` (eff 2025-06); a 2025-07 disposal on W is HIFO, not LIFO.
- **(c) voided scoped election** — void → falls back to global/FIFO (resolve.rs:844 skips voided). One KAT.
- **(d) pre-2025 residue + post-2025 scoped election** — pre-2025 disposals on that wallet ignore the election
  (use `pre2025_method`); post-2025 honor it. Confirms [★Q3].

**Fix:** Add (a)-(d) to the KAT list under Task 2.

### [M3] MINOR — "only Exchange electable" + known-wallet validation: name the enumeration source and reject the `self:` form

The spec says validate `--exchange` against "the vault's known Exchange wallets" (line 57-59) and that only
Exchange wallets are electable (Gotcha, line 108-109), but doesn't name the source of the wallet set or the
`self:`-rejection. `parse_wallet_id` accepts `self:LABEL` too (eventref.rs:67-69), so the CLI must additionally
reject a `SelfCustody` scope. The set of distinct wallets is derivable from events' `wallet` field (the
`wallet_of` map pattern, compliance.rs:105-108) or `state.holdings_by_wallet` (fold.rs:1226-1243).

**Fix:** Spec the validation: parse via `parse_wallet_id`; reject non-`Exchange` with a clear message; validate
the `Exchange{provider,account}` against distinct Exchange wallets enumerated from the event log / projected
state; reject unknown loudly. Enforce Exchange-only at the core constructor too (defense in depth), so a
non-Exchange scope can never reach `ElectionRec`.

### [N1] NIT — spec's "GUI `schema_mirror` lockstep" references an artifact that does not exist in this repo

Spec line 86-87 (and brainstorm line 70) list "GUI `schema_mirror` if any clap flag NAME changes." There is no
GUI/tauri/desktop crate (crates are: `btctax`, `-adapters`, `-cli`, `-core`, `-store`, `-tui`, `-tui-edit`,
`xtask`) and no `schema_mirror` token anywhere in the tree. This is carried-over brainstorm boilerplate.

**Fix:** Drop the GUI-mirror line. Real lockstep surfaces (already listed): man pages via `make docs`, the
`?`-overlay keymap, and the `MethodElection` doc-comments (event.rs:240-248).

### [N2] NIT — citations verified

`event.rs:245` (`MethodElection`), `resolve.rs:16` (`method_election_is_forward`), `fold.rs:30`/`:60`
(`applicable_method` / call site), `identity.rs:111` (`WalletId::Exchange`), `main.rs:239`
(`set_forward_method` dispatch, now at :234-239) all check out at `3fd619c`. `ElectionRec.decision_seq`
(resolve.rs:146) and `FoldCtx.elections` (fold.rs:20-24) confirm the total order the spec relies on. No
citation drift.

---

## Re-review note
[I1] and [I2] are the blockers. [I1] is a genuine wrong-output bug the spec would ship (compliance.rs left out
of scope); [I2] is a design-of-record grammar decision to pin, not defer. Fold both, add the M-series KATs and
scope/plan edits, and re-run R0 round 2 to confirm 0C/0I.
