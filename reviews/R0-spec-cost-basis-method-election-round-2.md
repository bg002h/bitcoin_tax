# R0 — SPEC review (round 2): per-exchange cost-basis method election

**Artifact:** `design/SPEC_cost_basis_method_election.md` (folded, round-1 findings applied)
**Branch/commit:** `feat/cost-basis-method-election` @ `efcc340` (main == `fa675bb`)
**Reviewer role:** independent architect (R0, read-only). **Bar:** 0 Critical / 0 Important.
**Round-1:** 0C / 2I / 3M / 2N (`reviews/R0-spec-cost-basis-method-election-round-1.md`). All findings folded.
**Design of record:** `design/BRAINSTORM_auto_pseudo_reconcile.md` (settled decisions NOT relitigated).

## Verdict: 0 Critical / 0 Important / 2 Minor / 1 Nit — **R0-GREEN**

Both round-1 Important findings are resolved AND correct against source. Both `I`-blockers were
genuine second-path omissions; the folded spec closes them at the right altitude:

- **I1** (`compliance.rs` second resolution site) — CONFIRMED real and CONFIRMED fixed. `compliance.rs`
  is in scope (spec §Scope line 116, Task 2 line 124-128) and the fix is a SHARED wallet-aware resolver
  used by both `fold::applicable_method` and `disposal_compliance`, with a dedicated taint KAT.
- **I2/M3** (canonical grammar) — CONFIRMED. The CLI now reuses `eventref::parse_wallet_id`
  (`exchange:PROVIDER:ACCOUNT`) and rejects `self:`; the invented `/` delimiter is gone.
- **M1 / M2 / N1** — all folded correctly (verified below).

Two Minors remain (neither blocks green): a narrow uncovered corner in the two-tier rule (not-yet-effective
scoped + in-force global), and the shared-resolver return contract. Details below.

---

## Verification of each round-1 fold against source

### [I1 — RESOLVED, correct] `compliance.rs` is genuinely a second resolution site; the shared-resolver fold is right

Confirmed against `crates/btctax-core/src/project/compliance.rs` @ `efcc340`:

- `disposal_compliance` (compliance.rs:91) DOES do independent election resolution. Its internal
  `Election` struct is `{ effective_from, decision_seq }` — **no wallet** (compliance.rs:37-40), collected
  by `collect_elections` (compliance.rs:47-67). Step (3) of the classifier picks the standing order as a
  **GLOBAL `max_by`** over ALL elections, ignoring the disposal's wallet (compliance.rs:169-180,
  `.filter(|e| e.effective_from <= date).max_by(effective_from, then decision_seq)`).
- So the round-1 taint is real: a scoped `Coinbase→HIFO` election would satisfy step (3) for a `Gemini`
  disposal and return `StandingOrder`, over-reporting §A.5(a). (One nuance the round-1 note under-stated but
  which does not change the finding: step (2), compliance.rs:159-164, returns Contemporaneous/NonCompliant
  first when a `LotSelection` was applied, so the taint reaches only disposals with NO applied selection —
  precisely the KAT's setup.)
- `wallet_of: EventId → WalletId` IS available at compliance.rs:105-108 (built from `e.wallet`), so the
  disposal's wallet can be fed to a shared resolver. **Implementable at both call sites:** compliance has
  `date` (compliance.rs:199/213) + wallet (`wallet_of.get(disposal)`) + elections; fold has `date` +
  the disposal's wallet (already bound at every `consume_principal` caller, round-1 ★Q1) + `ctx.elections`.
- The spec correctly places `compliance.rs` in scope (line 116) and in Task 2 (line 124-128), and adds the
  KAT `scoped_election_does_not_taint_compliance_of_other_wallets` (line 103-105). The expected result is
  tax-correct: Gemini's scoped tier and global tier are both empty ⇒ falls through to `NonCompliant`
  (compliance.rs:183), NOT `StandingOrder`.

One implementation-contract nuance carried into a Minor below (M2): the shared primitive must return the
**in-force election record (Option)**, not a FIFO-defaulted `LotMethod`, because compliance needs both
"does an in-force election exist" and its `effective_from` to build `StandingOrder { effective_from }`.

### [I2 / M3 — RESOLVED, correct] canonical grammar reused; `self:` rejected; enumeration feasible

Confirmed against `crates/btctax-cli/src/eventref.rs` @ `efcc340`:

- `parse_wallet_id` (eventref.rs:57-74) exists exactly as the spec describes: `splitn(3, ':')`, matching
  `["exchange", provider, account]` (both non-empty) → `WalletId::Exchange { provider, account }`
  (eventref.rs:61-66), and `["self", label]` → `WalletId::SelfCustody { label }` (eventref.rs:67-69). The
  CLI rejects `self:` by matching the returned variant (must be `Exchange`), per spec line 76-79. Because
  `splitn(3, ':')` keeps everything after the second colon in `account`, the grammar tolerates `:` and `/`
  inside the account — the exact hazard I2 flagged against a `/` delimiter is avoided.
- Enumerating distinct `WalletId::Exchange` for validation is feasible: import events carry
  `LedgerEvent.wallet` (surfaced at resolve.rs:833, and the `wallet_of` pattern at compliance.rs:105-108).
  Iterate events' `wallet`, filter to `Exchange`, collect a set — reject an unknown parse loudly (spec
  line 76-79). `WalletId::Exchange` derives `Eq` over BOTH `provider` and `account` (identity.rs:109-111),
  so validation and `wallet == Some(W)` are account-keyed (not provider-keyed) — this grounds the
  `two_accounts_same_provider_independent` KAT.

### [M1 — RESOLVED] scope in the payload, not the event-wallet column

Confirmed: `append_and_save` (reconcile.rs:28-36) hard-codes `wallet: None` via
`append_decision(session.conn(), payload, now, UtcOffset::UTC, None)` (reconcile.rs:33), with the doc-comment
"decisions are not wallet-scoped" (reconcile.rs:27). `append_decision`'s `wallet` param
(persistence.rs:238-262) writes the `LedgerEvent.wallet` column (persistence.rs:256) — for a decision this
stays `None`. `set_forward_method` (reconcile.rs:826-843) builds the `MethodElection` payload and routes
through `append_and_save`, so the scope has nowhere to live but the payload. Spec line 82 states this
correctly ("scope lives in the `MethodElection` PAYLOAD ... `append_decision` passes `wallet=None`"). The
payload is the right home — resolve.rs reads it at collection time (resolve.rs:847, `me.method` at :859; the
new `me.wallet` reads alongside).

### [M2 — RESOLVED, correct + tax-safe] two INDEPENDENT tiers, not a merged max

Confirmed the current `applicable_method` (fold.rs:30-45) is today a single global `max_by` over all
elections — exactly the shape M2 warns against carrying forward. The spec now mandates two independent tiers
(line 44-57): step 1 among `wallet == Some(W)` with `effective_from ≤ D`; only if that in-force set is empty,
step 2 among global; else FIFO — explicitly "Do NOT implement as a single `max_by` over all elections
merged." The `later_global_does_not_override_in_force_scoped` KAT (line 106-107) pins it. Tax-correct: a
user who scoped a wallet must not have it silently flipped by a later global order. `ElectionRec.decision_seq`
(resolve.rs:145) + the unique-seq total order make each tier deterministic (NFR4), no `max_by` ties.

### [N1 — RESOLVED] GUI schema_mirror dropped; real lockstep named

Re-verified by grepping the tree: crates are `btctax`, `-adapters`, `-cli`, `-core`, `-store`, `-tui`,
`-tui-edit`, `xtask` — **no GUI/tauri/desktop crate**; `schema_mirror` appears ONLY in `reviews/*.md` and
`design/*.md` prose, never in source. Spec line 119-120 drops the GUI mirror and names the real lockstep:
`make docs` (man pages), the `?`-overlay keymap, and the `MethodElection` doc-comments (event.rs:240-248).
Correct.

---

## No new drift / residual-gap sweep

- **Serde back-compat still sound.** `MethodElection { effective_from, method }` (event.rs:244-248) gains a
  `#[serde(default)] wallet: Option<WalletId>`; direct precedents `#[serde(default)] pre2025_method`
  (event.rs:188) and `#[serde(default)] kind: Option<IncomeKind>` (event.rs:228). Adding a within-variant
  field (not a new variant) is forward+backward compatible — old JSON without the field loads as `None` =
  today's global behavior. (The event.rs:216-218 "old-binary limitation" note is about NEW variants and does
  not apply here.) The pinned old-JSON fixture KAT (line 100-101) is the right guard.
- **Precedence still reachable.** `applicable_method` has exactly ONE caller, `consume_principal` (fold.rs:60),
  which every method-honoring op reaches with `wallet` already bound (round-1 ★Q1). Threading `wallet` is
  mechanical.
- **No THIRD resolution site.** A tree sweep finds `effective_from <= date` + `max_by` in exactly two places:
  `fold.rs:36` and `compliance.rs:171` — the two the spec addresses. `transition.rs` builds the same
  `FoldCtx { elections }` and delegates to the shared `fold_event` (transition.rs:55-58) — no independent
  resolution; pre-2025 routes through the Universal pool + `pre2025_method`, elections never apply.
  `optimize.rs` / `evaluate.rs` only inject `res.selections` (LotSelection candidates) and re-fold via the
  shared path — no independent election logic. So the two-site fix is complete.
- **KAT coverage complete for the identified paths:** compliance taint (line 103-105), two-tier
  (line 106-107), same-provider-two-accounts (line 108), voided-scoped fallback (line 109-110, grounded by
  the `voided.contains` skip at resolve.rs:844), pre-2025-residue + scoped (line 110-111, grounded by the
  pre-2025 Universal-pool route), serde-backcompat fixture (line 100-101), plus governance / scoped-beats-
  global-beats-FIFO / LotSelection-override / backdating-blocks / determinism / CLI / TUI. No path by which a
  tax-wrong method reaches a disposal is left un-KAT'd — **except the narrow corner in M1 below.**

---

## Findings (both Minor; do NOT block green — fold into the Plan/Task 2 at implementation)

### [M1] MINOR — one uncovered two-tier corner: a not-yet-effective SCOPED election must not suppress an in-force GLOBAL one (→ FIFO instead of the global method)

The spec's step 1 correctly says "wallet == Some(W) **and `effective_from ≤ D`**" (line 44), but M2's gloss
"only if **that set** is empty do we fall to step 2" (line 55-56) could be misread as "only if NO scoped
election exists *at all* (regardless of effective date)." Under that misreading, a scoped election dated
AFTER a disposal (not yet in force) would suppress the fall-through to an **in-force GLOBAL** election and
wrongly yield FIFO (step 3) — a tax-wrong method reaching a disposal. The correct reading (and the natural
impl, since `applicable_method` already `.filter(effective_from <= date)` FIRST at fold.rs:36) is that the
`effective_from ≤ D` in-force filter applies **within each tier** before the tier fallback.

This corner is not pinned by any listed KAT (`scoped_beats_global_beats_fifo`,
`later_global_does_not_override_in_force_scoped`, and `voided_scoped_election_falls_back` all leave it open).

**Fix (Minor, Task 2):** In §2/M2, add one sentence: "the `effective_from ≤ D` in-force filter is applied
WITHIN each tier before the tier fallback — a scoped election whose `effective_from > D` is treated as absent
for date D, so an in-force GLOBAL election governs (not FIFO)." Add a KAT
`future_scoped_election_yields_in_force_global_not_fifo` (scoped `W→HIFO` eff 2025-09 + global `LIFO`
eff 2025-02; a 2025-06 disposal on `W` uses **LIFO**, not FIFO). Severity Minor: the spec is not wrong (step
1 already states `effective_from ≤ D`), and the existing fold filter makes the correct impl the natural one.

### [M2] MINOR — pin the shared-resolver return contract as the in-force election (Option), not a FIFO-defaulted `LotMethod`

The spec says "extract ONE shared wallet-aware resolver ... used by BOTH `fold::applicable_method` AND
`disposal_compliance`" (line 124-128) but does not pin its return type. The two callers have asymmetric
needs: fold wants a `LotMethod` (FIFO fallback for empty, plus the pre-2025 `pre2025_method` pre-check that
stays fold-local); compliance needs to know (a) whether ANY in-force election exists (None ⇒ not
`StandingOrder`) and (b) its `effective_from` (to build `StandingOrder { effective_from }`, compliance.rs:179).
A resolver returning `LotMethod` with a FIFO fallback strands compliance — it cannot recover `effective_from`
and cannot distinguish "no election" from "an explicit FIFO election." The right shared primitive is
`resolve_in_force_election(date, wallet, &[ElectionRec]) -> Option<&ElectionRec>`; fold maps
`.map(|e| e.method).unwrap_or(Fifo)` and compliance maps `.map(|e| StandingOrder { effective_from })`.

This is largely self-correcting (compliance won't compile against a `LotMethod`-returning resolver), so it is
a clarity Minor, not a tax hazard.

**Fix (Minor, Task 2):** State that the shared resolver returns the winning in-force `ElectionRec` (Option),
the two tiers applied inside it; each caller projects its field. Also note (mechanical) that
`compliance::collect_elections` must carry the new `wallet` scope on its record (today's internal `Election`
struct at compliance.rs:37-40 has no `wallet`) — reusing `ElectionRec` or adding the field — while the
existing shared `method_election_is_forward` guard (resolve.rs:16) stays the collection predicate.

### [N1] NIT — citations re-verified at `efcc340`

`compliance.rs:47-67/91/105-108/169-180`, `eventref.rs:57-74`, `fold.rs:30-45/60`,
`resolve.rs:16-18/141-146/842-863`, `event.rs:188/228/244-248`, `persistence.rs:238-262`,
`reconcile.rs:28-36/826-843`, `identity.rs:109-111`, `transition.rs:55-58` all check out. The round-1 note's
citation `disposal_compliance (compliance.rs:91)` is `:91`; the spec's shorthand
`compliance.rs:47-67,169-180` (collection + global-max) and `:105-108` (`wallet_of`) are accurate. No drift.

---

## Re-review note

0 Critical / 0 Important — **R0-GREEN**. Both round-1 blockers (I1 compliance second-site, I2 canonical
grammar) are resolved and correct against source; M1/M2/M3/N1 folds verified. The two remaining Minors (M1
two-tier corner + KAT; M2 shared-resolver return contract) are clarity/coverage refinements for Task 2, not
gate blockers — fold them into the Plan when it is written. The spec may proceed to planning.
