# R0 — Lot-Identification & Tax-Optimization Program (Phase-2) — Round 1

**Artifact:** `design/SPEC_lot_optimization_program.md` (the A/B/C program spec)
**Reviewer hat:** principal architect + US-crypto-tax reviewer (independent, adversarial)
**Date:** 2026-06-29
**Engine baseline read:** `crates/btctax-core/src/project/{pools,transition,resolve,fold,mod}.rs`, `state.rs`, `event.rs`, `persistence.rs`, `conventions.rs`, `identity.rs`; `crates/btctax-cli/src/config.rs`; `design/SPEC_foundation.md`.
**Process:** `STANDARD_WORKFLOW.md` §6 rubric (Critical/Important block the gate). Per §3, every external-fact claim was verified against the **authoritative source text**, not against the draft.

---

## 0. Verdict (up front)

**NOT green. Blocking design defects present. Do NOT fold-and-proceed to per-sub-project planning until the Criticals below are resolved and the spec is re-reviewed (loop per §2).**

The decomposition (A→B→C), the two-knob model, and the standalone-B idea are fundamentally sound and well-matched to the existing engine. But the spec is built on a **stale and partly incorrect reading of the identification regulations**, and that error propagates into the headline feature (the optimizer). Specifically:

1. The single most load-bearing premise — that post-hoc lot selection is permissible for 2025 (Open Q#4) — is **wrong**, and the spec is **missing Notice 2026-20** (issued ~2026-03, directly on point, extends the relief through 2026-12-31). Verified below.
2. The forward standing order (`lot_method`) is modeled as a **mutable, non-event-sourced side-table**, which structurally **cannot satisfy the regulation's "recorded before the sale" requirement** and will silently manufacture non-compliant retroactive HIFO/LIFO positions.
3. The `pre2025_method` knob can **silently invalidate an already-attested (irrevocable) safe-harbor allocation** through the existing conservation guard.

These are exactly the failure classes the §6 rubric calls Critical (a wrong tax position; a guarantee — "the least tax the *law permits*" — that the artifact does not deliver; a missing interaction that corrupts the existing safe-harbor conservation). They are fixable without abandoning the design, but they reshape **A** before it is built, so they must be folded now.

---

## 1. Primary-source verification (confirm / correct, with citations)

> Sources consulted: eCFR/Cornell LII 26 CFR 1.1012-1(j); IRS Notice 2025-07 (n-25-07); **IRS Notice 2026-20** (extension); Rev. Proc. 2024-28 (rp-24-28); IRS NIIT Q&A / Form 8960 (2025) instructions; multiple practitioner analyses (Journal of Accountancy 2025-05; Current Federal Tax Developments 2026-03-18; Camuso CPA; thetaxadviser; Tax Notes). The IRS PDF auto-summarizer hallucinated a "post-hoc by due date" reading of Notice 2025-07 on first fetch; that reading was **contradicted by four independent sources quoting the operative text verbatim** and is rejected (§3 false-consensus discipline).

### 1.1 §1.1012-1(j)(3) — specific-ID standard, timing, FIFO default, standing order, per-wallet scope
**CONFIRMED, with one material correction to the spec's framing.**

- **Adequate-identification standard & timing.** For broker-custodied units, **(j)(3)(ii)**: adequate identification is made if, *"no later than the date and time of the sale, disposition, or transfer,"* the taxpayer specifies to the custodial broker the particular units. The identification may be a transaction-level instruction **or a standing order on file with the broker**. The **timing is "by the date and time of the sale" — not post-hoc.**
- **(j)(3)(i) FIFO default.** If no adequate identification is made, units are treated as sold *"in order of time from the earliest date on which units of that same digital asset held in the custody of the broker were acquired."* **FIFO is the default — CONFIRMED.**
- **Non-broker / self-custody (the app's core case).** A separate rule governs digital assets **not** held by a broker (unhosted/self-custody wallets): a specific identification is made *"no later than the date and time of the sale, disposition, or transfer, [when] the taxpayer identifies on its books and records the particular units…"*; absent that, **FIFO** by earliest-acquired *within units not held by a broker*. **Adequate records must establish the unit is removed from the wallet.** This rule lives in the **base regulation** and is **not subject to any post-hoc relief.**
- **Per-wallet / wallet-by-wallet scope.** CONFIRMED. Treasury read §1012(c)(1) as requiring **wallet-by-wallet** application; the **universal method is no longer permissible** for 2025+. The engine's `PoolKey::Wallet` post-transition is correct.
- **Effective date.** §1.1012-1(h) and (j) apply to acquisitions/dispositions **on or after 2025-01-01**. CONFIRMED (matches `TRANSITION_DATE`).

**Correction to the spec (line 14).** "LIFO/HIFO as a standing instruction are permitted, per wallet" is correct in substance **but omits the binding timing condition**: a standing order is adequate ID **only if recorded before the units it covers are sold.** The spec treats `lot_method` as a config flag with no temporal/recorded-before semantics → see Critical **C2**.

### 1.2 Notice 2025-07 — 2025 records-based relief; scope; post-hoc?
**CONFIRMED scope; spec's post-hoc premise (Open Q#4) is REFUTED; spec is MISSING the 2026 extension.**

- Relief period: **2025-01-01 → 2025-12-31** (broker-custodied units only — it relieves the *(j)(3)(ii)* "specify to the broker" requirement, which self-custody never had).
- Mechanism: during the relief period the taxpayer may make adequate ID **on the taxpayer's own books and records**, by either (a) *"identifying, **no later than the date and time of the sale**, … the particular units … by reference to any identifier…"* or (b) *"recording a standing order on the taxpayer's books and records … **entered into the taxpayer's books and records before the units covered by the order are sold**…."*
- **Post-hoc?** **NO.** The relief changes **who** you tell (own books instead of broker), not **when** (still by time of sale / pre-recorded standing order). Multiple sources verbatim. The first-pass "on or before the due date" reading was a summarizer artifact and is rejected.

### 1.3 Notice 2026-20 — the spec MISSES this; it answers Open Q#4 for 2026
**CONFIRMED — and it is directly dispositive of Open Q#4.** Issued ~2026-03 (before this review's date). It:
- **Extends the relief period to 2025-01-01 → 2026-12-31** (so the spec's "what happens for 2026+" question is partly answered: 2026 is covered by extended own-books relief).
- Keeps the **same timing**: ID *no later than the date and time of sale*, or a **standing order recorded before** the covered units are sold. **Still no post-hoc.**
- Adds **§4.05**: the units sold are those identified in the taxpayer's books and records *regardless of whether the broker's 1099-DA matches* — useful for the app's posture.
- **After 2026-12-31**, the permanent (j)(3)(ii) applies: broker-held units require **specifying to the broker** by time of sale (own-books-only is insufficient for broker-held); self-custody continues under the base own-books-by-time-of-sale rule.

### 1.4 Rev. Proc. 2024-28 — pre-2025 recital; §5.02 reasonable allocation
**CONFIRMED; the engine models it correctly.**
- Recites the pre-2025 universal specific-ID-or-FIFO regime concluding 2024-12-31; safe harbor for remaining basis allocation **as of 2025-01-01**, effective for dispositions **on/after 2025-01-01**.
- **§5.02 reasonable allocation**: either **specific-unit allocation** (assign basis to specific units / a pool *within a single account*) or **global allocation** (a written rule, created **before 2025-01-01**, allocating across accounts). Allocation must conserve (same number of units; same asset). The deadline split — specific-unit barred at the **earlier-of** first-disposition/return-due; global at the **later-of**, with its method description predating 2025 — matches `resolve.rs` step 3 (`min_opt`/`max_opt`, `timely_allocation_attested`). Good.

### 1.5 Rate authorities (B.3)
- **§1(h) LT 0/15/20 stacking** — CONFIRMED. LT/QD sit **on top of** ordinary income for breakpoint placement; B.3's "stacked on ordinary income" is correct.
- **§1411 NIIT 3.8% × min(NII, MAGI − threshold)** — CONFIRMED. Thresholds **$250k MFJ / $125k MFS / $200k Single & HoH** (and QSS = $250k). **CORRECTION:** these thresholds are **statutory and NOT inflation-indexed** — see **I4** (B.2 wrongly lumps them into "tables inflation-adjusted yearly").
- **§1211/§1212 $3,000 ($1,500 MFS) loss limit + indefinite carryforward, character-preserving** — CONFIRMED, and the **$3k/$1.5k figures are fixed, not indexed** (also relevant to I4).
- **§1222 netting order** (net ST vs ST, LT vs LT, then cross-net) — CONFIRMED as stated in B.3. The **§1212(b) carryforward ordering** (net ST loss absorbs the $3k ordinary offset first) must be pinned in B's KATs (see I-list).
- **B.3 contains no rate mis-statement** other than the NIIT/loss-limit indexing issue (I4).

### 1.6 §1091 wash-sale crypto exemption (C.5)
**CONFIRMED accurate as of 2026.** Crypto is property, not "stock or securities"; §1091 does not apply; no statute has passed (recurring Greenbook/legislative proposals only). C.5's "exempt; monitor" posture is correct. (Note: Form 1099-DA box 1i exists to report disallowances *if* an asset is in fact a security — not a change to crypto generally.)

---

## 2. Overall opinion (architect)

**Strengths.**
- **Decomposition is right.** A (substrate) → B (rate engine) → C (optimizer) is the correct dependency order, and the forced build order (C needs B's rate-awareness; both ride A's selections) is justified. A is independently shippable.
- **Reuse of the existing event-sourced spine is excellent.** `LotSelection` as an appended, voidable decision; config/profile as projection-input side-tables; the side-effect-free evaluate path mirrors the proven `universal_snapshot` throwaway-fold pattern (transition.rs). This is the right grain.
- **B's "minimal per-year profile" is the correct YAGNI line** — it resists becoming a 1040 engine while capturing the stack base needed for §1(h)/NIIT marginal placement. With two corrections (I5, I9) it is sufficient for *correct* (not just precise) marginal rates for the stated objective.
- **Holistic single-year optimizer is the correct objective** (greedy is unsafe — see Open Q#2).
- The spec **already names the right risks** (Open Q#1–#6). The problem is that two of them (#3, #4) are blocking and are left open rather than resolved, and #4 is resolved *incorrectly* by assumption.

**Weaknesses (the load-bearing ones).**
- **The compliance model is the weakest part and it's the whole point.** The north-star is "least tax the *law permits and regulation does not forbid*." The identification regs **forbid post-hoc selection** (verified §1) — so the optimizer's Mode-1, as written, points the user at positions the regulation forbids. The forward standing order is the *compliant* lever, and it's the part the spec under-builds (mutable flag, no recorded-before semantics).
- **`acquire_ref` is not a sufficient lot key** (I1) — a latent determinism/exactness bug against NFR4/NFR5.
- **Several consumption-site and conservation questions are under-specified** (I2, I3) and must be pinned before planning.

---

## 3. Findings

### CRITICAL

#### C1 — Mode-1 "post-hoc identification" is not adequate identification in any year; the spec works from stale/incorrect law (Open Q#4)
**Where:** C.2 (Mode 1), Legal grounding line 15, Open Q#4.
**What:** The regulations require specific identification **no later than the date and time of the sale** — for **self-custody** directly in the base reg (no relief ever), and for **broker-custodied** units via (j)(3)(ii) as relieved by Notices 2025-07 / **2026-20** (own books, *still by time of sale or a pre-recorded standing order*). **No authority permits choosing lots for already-executed disposals at tax-filing time.** Mode-1's core conceit — "over the disposals already in the ledger for a tax year, compute the tax-minimizing `LotSelection` set … accepts … appends the `LotSelection` decisions and records the attestation" — is, for a sale that already happened without a contemporaneous ID/standing order, **undocumented FIFO reported as HIFO**. Recording it as the filed position is taking a position the regulation forbids, directly violating the north-star. The spec also **omits Notice 2026-20 entirely** (it post-dates the cited 2019 FAQ / 2024-28 / 2025-07 and is directly on point).
**Why Critical:** produces an unsupportable tax position; defeats the artifact's stated guarantee.
**Fix (reframe, don't delete):**
1. Make the **forward standing order + contemporaneous `select-lots` the compliant primary path** (A), and make Mode-1 a **planning/what-if by default** ("this is the tax if you had identified thus").
2. Persisting `LotSelection` decisions that change a filed result must require an **accurate, narrow attestation** — that a contemporaneous identification or a standing-order-on-books existed at/before the sale that matches these units — and the app must **refuse to invite a false attestation** (do not auto-attest on `optimize accept`). Default `optimize accept` to **current, not-yet-filed** dispositions for which the user is recording the ID essentially contemporaneously.
3. Update Legal grounding to cite **Notice 2026-20** and restate the timing rule (self-custody = base reg, no relief; broker = relieved who-not-when through 2026; broker requires broker-communication 2027+).
4. C must surface, per disposal, whether a selection is **supported** (contemporaneous/standing order present) vs **post-hoc/unsupported** (→ FIFO is the defensible filing position).

#### C2 — `lot_method` as a mutable, non-event-sourced side-table cannot express "standing order recorded before the sale"; silently produces non-compliant retroactive HIFO/LIFO
**Where:** A.1 (`lot_method` repurposed from `cli_config`), Cross-cutting "config/profile are projection-input side-tables (not ledger state)", `cli/src/config.rs`, `ProjectionConfig`.
**What:** `lot_method` is set via `config --set-lot-method` and applied **globally to all post-2025 disposals** with **no record of when it was set**. A standing order is valid adequate ID **only if recorded before the covered units are sold** (§1, (j)(3)(ii) + Notices). A user setting `lot_method=HIFO` in 2026-06 would have the engine **retroactively** re-order the consumption of 2026-01…2026-05 disposals under HIFO — which were **not** covered by any standing order recorded before them, so their correct treatment is **FIFO**. The mutable side-table has no temporal dimension to prevent this, and **no audit trail** of the order's effective date (violates the spirit of NFR6 for a fact that now has tax-compliance significance).
**Why Critical:** silently manufactures positions the regulation forbids; no auditability of the standing order's recording time.
**Fix:** Event-source the forward standing order as a **dated decision** (e.g., `StandingOrder { method, effective_from }`, an appended decision with the existing `Decision{seq}` + `utc_timestamp`/`original_tz` made-date). The fold applies a standing order **only to disposals on/after its recorded made-date**; disposals before any standing order use **FIFO** (the default). Multiple standing orders over time are honored by made-date (latest-in-force at each disposal). This also gives `verify` a truthful "standing order recorded YYYY-MM-DD: HIFO" line. (`pre2025_method` is *not* affected — see C3 — it is an attested historical fact, correctly a side-table.)

#### C3 — `pre2025_method` change silently invalidates an already-attested (irrevocable) SafeHarborAllocation via the conservation guard (Open Q#3)
**Where:** A.3 ("`pre2025_method` automatically becomes the conservation baseline upstream of safe-harbor"), `transition::universal_snapshot`, `resolve.rs` step 3 conservation check, §7.4 irrevocability.
**What:** `snap.basis` (Σ remaining basis in the Universal pool) is **method-dependent**: FIFO vs HIFO consume *different* pre-2025 lots, leaving *different* remaining basis (sat total is invariant; **basis is not**). The safe-harbor guard checks `alloc_basis == snap.basis` (resolve.rs ~L547). An allocation attested to conserve against the **FIFO** residue will **fail** conservation once `pre2025_method` flips to LIFO/HIFO → hard `SafeHarborUnconservable` with the misleading detail *"allocation totals != Universal remainder"* (it isn't bad data — it's a method change). Worse, an **effective** allocation is **irrevocable** (§7.4); the user cannot void it to fix it, and may "repair" the blocker by silently re-writing an irrevocable position — corrupting the carried-forward per-wallet basis for **all** post-2025 dispositions.
**Why Critical:** the prompt's named Critical trigger — a missing interaction that corrupts the existing safe-harbor conservation; can lock the user into an unrepairable state or an altered irrevocable basis.
**Fix:**
1. **Bind the method to the allocation.** Record the `pre2025_method` in force when a `SafeHarborAllocation` is attested (or compute the conservation snapshot for an effective Path-B allocation against the *recorded* method, not the current config). Legally this is correct: Rev. Proc. 2024-28 requires the allocation to reflect the residue **under the taxpayer's actual historical method**.
2. **Precedence rule:** `pre2025_method` is declared **before** any allocation; a later change that would break an effective allocation's conservation is a **material change that re-enters the gate** (STANDARD_WORKFLOW §1 "material change"), surfaced via a **dedicated, explanatory blocker** (e.g., `Pre2025MethodConflictsAllocation`) — never the generic `SafeHarborUnconservable`.
3. Add the composition KAT the spec promises **plus** a KAT for the method-change-vs-effective-allocation conflict.

### IMPORTANT

#### I1 — `LotPick { acquire_ref: EventId, sat }` is an ambiguous, non-deterministic lot key
**Where:** A.2 (`LotSelection`/`LotPick`), A.2 CSV (`disposal_ref,acquire_ref,sat`).
**What:** One origin event can produce **multiple lot fragments** with distinct `split_sequence` — via self-transfer relocation (`fold.rs` `Op::SelfTransfer` `bump_split`), pre-2025 *and* post-2025, and the same origin can appear in **different wallets** *or twice in the same wallet*. These fragments are **not perfectly fungible**: TP8(c) fee re-home (`rehome_onto_lot`) adds fee-sat basis onto `relocated.last()`, so two fragments of one origin can differ in per-sat basis by cents. Selecting "`sat` from `acquire_ref`" cannot deterministically choose among fragments → violates NFR4 (determinism) and NFR5 (exact arithmetic).
**Fix:** Pick by the **full `LotId` (origin_event_id + split_sequence)**, or define and document a deterministic same-origin consumption order (and prove fragments are basis-equal, which they are not under TP8(c)). The CSV needs the same discriminator. Given LotId is already the stable, `Ord`ered lot identity, key on it.

#### I2 — Which consumption sites honor `lot_method` / `LotSelection` is unspecified
**Where:** A.3 (generalize `consume_fifo`→`consume`), but `consume_fifo` is called in **six** places: `Dispose` principal, `consume_fee`, `PendingOut`, `SelfTransfer` principal, `GiftOut`, `Donate`.
**What:** The spec only describes the `Dispose` path. But: a **self-transfer's** lot choice materially changes future per-wallet HIFO/gains (and the donee's carryover basis for gifts) and is itself a "transfer" under (j); `GiftOut`/`Donate` consumption changes remaining basis and the donee's carryover; `PendingOut` is provisional; `consume_fee` is de minimis. Leaving this implicit will produce divergent, untested behavior and an under-powered optimizer (it can't pre-position lots via self-transfer if self-transfer ignores selections).
**Fix:** Enumerate all six. Recommended: honor `lot_method`+`LotSelection` on `Dispose`, `GiftOut`, `Donate`, and `SelfTransfer`; `PendingOut` and `consume_fee` stay **FIFO** (provisional / de minimis) and say so. Pin whether `select-lots` may target non-`Dispose` ops.

#### I3 — Conservation rule A.4(a) vs on-chain `fee_sat` is ambiguous
**Where:** A.4(a) "Σ picked sat == the disposal's sat"; `Op::Dispose.fee_sat`, `consume_fee`.
**What:** A reclassified `TransferOut`→`Dispose` carries a separate `fee_sat` (on-chain fee), consumed **after** principal via `consume_fee` (FIFO). Does "the disposal's sat" in A.4(a) include `fee_sat`? If the selection covers only principal, where do fee-sats come from — FIFO remainder? If they must be covered by the selection, the conservation total is `sat + fee_sat`. Unpinned, the validator is ambiguous and could reject valid selections or mis-conserve.
**Fix:** State that `LotSelection` covers **principal only** (`= Op` principal sat); `fee_sat` continues to consume **FIFO** from the post-selection remainder (deterministic). Add a KAT with a fee-bearing reclassified disposal under a selection.

#### I4 — B.2 tax-law error: NIIT thresholds and the $3k loss limit are NOT inflation-indexed; bundled tables must track enacted law, not just inflation
**Where:** B.2 ("NIIT thresholds (§1411) … Tables are inflation-adjusted yearly (IRS Rev. Proc.)").
**What:** §1411 thresholds ($250k/$200k/$125k) are **statutory and never inflation-indexed** (verified). The $3,000/$1,500 §1211 limit is likewise **fixed**. Only the §1(h) breakpoints and the ordinary brackets are inflation-adjusted. A maintenance process that "inflation-adjusts" the NIIT threshold would compute **wrong** NIIT. Separately, year-over-year table changes are driven by **enacted law**, not only indexing (e.g., 2026 brackets reflect post-OBBBA law, not a CPI bump on 2025) — the "inflation-adjusted yearly" framing understates the currency risk and the per-year sourcing burden.
**Fix:** In B.2, classify each table: **indexed** (§1(h) breakpoints, ordinary brackets, std deduction if used) vs **fixed by statute** (NIIT thresholds, $3k/$1.5k limit). Require each bundled year to be **sourced to the enacted authority for that year** (Rev. Proc. for indexed items; statute for fixed; the year's enacted law for structural changes), dated and source-noted, with a KAT asserting the NIIT threshold is constant across years.

#### I5 — Optimizer objective / "total_federal_tax_attributable" must be an incremental delta, and `ordinary_taxable_income` must exclude crypto ordinary income (double-count hazard)
**Where:** B.1 (`ordinary_taxable_income` "excluding the crypto gains"), B.3 (`TaxResult.total_federal_tax_attributable`, income events on the ordinary stack), C.4 objective.
**What:** (a) B places crypto **ordinary** income (mining/staking) on the ordinary stack, but B.1 only says `ordinary_taxable_income` excludes "the crypto **gains**." If the user includes their mining income in `ordinary_taxable_income`, it is **double-counted**. (b) "Total federal tax attributable to the crypto activity" is only well-defined as an **incremental** quantity — `tax(return with crypto) − tax(return without crypto)` computed ceteris-paribus on the minimal profile — otherwise it reads as the user's total 1040 liability (which the minimal profile cannot produce) and the optimizer's objective is ambiguous.
**Fix:** Define `ordinary_taxable_income` as "ordinary taxable income excluding **all** app-computed crypto items (both capital gains and crypto ordinary income placed on the stack)." Define the objective explicitly as the **delta** vs a no-crypto baseline, and name the **ceteris-paribus assumption** (the figure does not capture AGI-driven second-order effects outside the model — see Open Q#1). This also keeps B out of full-1040 territory (Open Q#6).

#### I6 — B must refuse to emit a TaxResult (and C must refuse to optimize) when the year has unresolved HARD blockers
**Where:** B.3/B.4, C.1–C.2; existing hard blockers (`FmvMissing`/basis-pending, `UncoveredDisposal`, `UnknownBasisInbound`, `ImportConflict`, etc.).
**What:** A disposal can consume a **basis-pending** lot (gain gated, `make_disposal_legs` raises `FmvMissing`) or be short (`UncoveredDisposal`). Computing a year's tax (or optimizing it) while such blockers affect that year's disposals yields a **wrong number presented as authoritative**. The spec gates only on missing profile/table, not on ledger hard-blockers.
**Fix:** B emits no tax number for a year with hard blockers touching that year's disposals → a hard blocker (e.g., `TaxYearNotComputable`) or an explicit "incomplete" result. C refuses to optimize such a year. Add KATs.

#### I7 — A.6 evaluate entrypoint must support a *hypothetical* (synthetic) disposal for C Mode-2; build it that way in A (sequencing)
**Where:** A.6, C.3 (Mode 2 consult on a sale **not in the ledger**).
**What:** Mode-2 scores a disposal that does not exist in the ledger. If A.6's "evaluate this selection set" entrypoint is built only to score **existing-ledger** `LotSelection`s, C must retrofit it to inject a synthetic disposal at a hypothetical date/wallet/proceeds — a design change mid-build. Also, `--fmv` cannot value a **future** date (no price in the dataset), so Mode-2 must require `--proceeds` for future/no-price dates.
**Fix:** Specify A's evaluate entrypoint to accept an **arbitrary candidate disposal (synthetic `Eff`) appended to the canonical timeline**, run through the same `consume`/validation/scoring path (the `universal_snapshot` throwaway-fold pattern). State that Mode-2 requires `--proceeds` when no price is available for `--at`. This reshapes **A** before it is built.

#### I9 — Minimal profile omits qualified dividends / other preferential-rate income, mis-placing §1(h) brackets
**Where:** B.1 (`other_net_capital_gain` optional; no qualified-dividend field), B.3 LT bracket placement.
**What:** Under §1(h), **qualified dividends share the same 0/15/20 bracket space** as net LT gain and stack together on ordinary income. A user with QD but the field omitted will have the **0%/15%/20% breakpoints mis-applied** to their crypto LT gain → wrong LT rate. `other_net_capital_gain` partially covers other LT but not QD.
**Fix:** Add a `qualified_dividends_and_other_pref_income: Usd` input (or a combined "preferential-rate income excluding app-computed crypto LT") to B.1, stacked with crypto LT for breakpoint placement. KAT: a user pushed across the 15→20 breakpoint by QD.

### MINOR

- **M1 — HIFO ordering key for dual-basis gift / basis-pending lots.** A.3 orders HIFO by `usd_basis`-per-sat (the **gain** basis). For a dual-basis gift lot in the **loss** zone the effective basis is `dual_loss_basis`; HIFO by gain-basis can mis-rank it. Basis-pending lots (`usd_basis = 0`) sort last under HIFO. Both are defensible **as a documented standing-order simplification** (C's scored optimum is zone-aware via the evaluate path and can differ from the manual HIFO order). Pin the rule and the divergence in the spec + a KAT.
- **M2 — Fully specify HIFO/LIFO total order.** A.3 gives FIFO/LIFO tie = `lot_id`; HIFO tie = oldest-first. Pin the final tiebreak after `acquired_at` (→ `lot_id`) so HIFO is a **total** order (NFR4).
- **M3 — §1212(b) carryforward ordering.** B.3 says "split ST/LT per §1212" — pin that the **net short-term loss absorbs the $3,000 ordinary offset first**, then net long-term; KAT it.
- **M4 — carryforward_in ↔ prior-year carryforward_out consistency.** B.1 takes `carryforward_in` as a user input while B.3 computes `carryforward_out`. Add a check/warn when year N+1's entered `carryforward_in` ≠ year N's computed `carryforward_out` (silent inconsistency hazard).
- **M5 — Optimizer determinism/exactness restated at the algorithm level.** C.4 must use `Decimal`/`i64` and `BTreeMap`/sorted iteration only; any DP/ILP table must be integer/Decimal-keyed (no float, no `HashMap` iteration). The spec asserts NFR4/5 generally; make it a C-plan acceptance criterion.
- **M6 — `verify` (A.5) should report the standing order's *recorded date*** (post-C2), not just the current method, and per-disposal supported/post-hoc status (post-C1).
- **M7 — Pre-2025 specific-ID.** `select-lots` works pre-2025 (Universal pool). Note that pre-2025 specific ID is governed by the pre-2025 regime (2019 FAQ A39–A40 / universal) and must match the taxpayer's **filed** result — i.e., a pre-2025 `LotSelection` that contradicts the attested `pre2025_method` for a **closed** year is itself a restatement, not a free optimization. Pin the interaction with `pre2025_method` and closed years.

### NIT

- **N1 — `Pre2025MethodNote` wording.** fold.rs's advisory still says "reconstructed under FIFO (the legal default)." Once `pre2025_method` exists (A.5 says the note should reflect the declared method), update the literal string; it currently hard-codes FIFO.
- **N2 — Naming.** "two method knobs" reads as symmetric, but `pre2025_method` (attested historical fact, side-table) and the forward standing order (dated decision, post-C2) are **different kinds of object**. Rename to make the asymmetry obvious (e.g., `pre2025_filed_method` vs `forward_standing_order`).
- **N3 — Backward-compat claim.** Cross-cutting says new `EventPayload`/`BlockerKind` variants are additive serde-default with no fingerprint change. Confirm in the plan that `LotSelection`/`StandingOrder` decisions get `fingerprint = None` (consistent with `SafeHarborAllocation`, persistence.rs `fingerprint` returns `None` for non-imported) — true today, but it's load-bearing for NFR4 and worth an explicit KAT.

---

## 4. Answers to Open Questions #1–#6

**#1 — Is the minimal tax-profile sufficient for correct §1(h)/NIIT marginal rates?**
**Yes, for the stated objective, with two fixes.** It is sufficient to compute first-order-correct §1(h) bracket placement, NIIT, ST stacking, the $3k limit and carryforward — **provided** (a) `ordinary_taxable_income` excludes **all** app-computed crypto items including crypto ordinary income (I5), and (b) a **qualified-dividends/other-preferential-income** field is added so the 0/15/20 breakpoints are placed correctly (I9). It is **not** a true marginal model for the whole return (AGI-driven phaseouts, SS taxability, IRMAA, AMT, QBI) — correctly out of scope — so the output must be defined as an **incremental, ceteris-paribus** delta and labeled as such (I5). With those changes: sufficient for *correct*, not merely precise, results for what the app claims to compute.

**#2 — Is greedy-per-disposal ever wrong vs holistic? Is holistic strictly required?**
**Holistic is strictly required; greedy can be strictly wrong.** Disposals are coupled by: §1(h) **breakpoints** (an incremental LT dollar can be 0/15/20 *and* 3.8% NIIT depending on fill), the **$3k limit + §1212 character-preserving carryforward**, and **§1222 cross-netting**. Concrete failure of greedy HIFO: realizing a large **short-term** gain (ordinary + NIIT) from a high-basis ST lot when a marginally lower-basis **LT** lot would be taxed at 15% — greedy "highest basis" loses. Another: harvesting a loss beyond the **usable** amount (only $3k offsets ordinary; the rest only matters if it nets against a gain) — greedy over-harvests a lot that yields no current benefit and wastes high-basis lots. The spec's holistic single-year objective is the right call; greedy is acceptable only as a **candidate generator** feeding the holistic scorer (as C.4 hints). Confirm in C's plan with the optimality KATs already listed.

**#3 — pre2025_method vs an already-attested SafeHarborAllocation.**
**Real defect — see Critical C3.** Because `snap.basis` is method-dependent, flipping `pre2025_method` after an allocation is attested breaks the allocation's conservation (hard `SafeHarborUnconservable`) and, given §7.4 irrevocability, can strand the user or alter an irrevocable basis. Resolution: **bind the method to the allocation** (record method-in-force; conserve an effective Path-B allocation against the recorded method), enforce **ordering** (method declared before allocation; later change = material re-entry), and emit a **dedicated** blocker (`Pre2025MethodConflictsAllocation`), not the generic one. Add the conflict KAT.

**#4 — Adequate-ID timing vs post-hoc Mode-1; Notice 2025-07 coverage; 2026+ boundary.**
**Decisively answered against the spec's premise.** (a) Notice 2025-07 does **not** support post-hoc identification — it relieves *who you tell* (own books vs broker), not *when* (still by time of sale, or a pre-recorded standing order). (b) **Self-custody** (the app's core) has **no relief at all** — the base reg requires own-books ID by time of sale; FIFO otherwise. (c) The spec is **missing Notice 2026-20**, which **extends the same own-books-by-time-of-sale relief through 2026-12-31** (broker-held), adds §4.05 (books control over a mismatched 1099-DA), and leaves **2027+** requiring **broker communication** by time of sale for broker-held units. **Net:** post-hoc Mode-1 selection is non-compliant in **every** year for self-custody, and for broker-held in every year too (relief is not post-hoc; 2027+ needs the broker). The compliant lever is the **forward standing order / contemporaneous per-sale ID** — which is why **C1 + C2** must be folded: reframe Mode-1 as what-if + accurate-attestation-gated, and event-source the dated standing order.

**#5 — Bundled per-year bracket/threshold tables: acceptable recurring dependency?**
**Acceptable** (same model as the price dataset; offline/deterministic; public reference data), **with I4's correction**: do not "inflation-adjust" fixed-by-statute items (NIIT thresholds, $3k limit), source each year to **enacted law** (not just CPI), and date/attribute each table. The `TaxTableMissing` hard blocker is the right safety. This is a maintenance burden, not a design flaw.

**#6 — Scope-creep check (toward a full 1040 / investment advice)?**
**Two yellow flags, both correctable, neither fatal.** (a) B computing a "total federal tax" and stacking crypto **ordinary** income drifts toward a 1040 engine — fix by defining the output as an **incremental delta** on the minimal profile (I5). (b) C.3's "waiting moves it to LT, saving ≈ $Y" timing insight borders on investment advice but stays on the right side as a **tax-consequence-of-a-contemplated-sale** (user-initiated, no "you should sell/hold"). Keep that boundary crisp in the spec (no hold/sell recommendation language). With I5 and the boundary statement, scope is contained.

---

## 5. Verdict & required folds before the gate

**Design is structurally sound but NOT green.** Fold the three Criticals and the Importants, then re-run the review loop (§2 — re-review after the fold, including the last one):

- **C1** Reframe Mode-1 (what-if default; accurate-attestation-gated persistence; cite Notice 2026-20; per-disposal supported/post-hoc status).
- **C2** Event-source the forward standing order as a **dated decision**; apply by made-date; FIFO before it.
- **C3** Bind `pre2025_method` to the safe-harbor allocation; ordering rule + dedicated conflict blocker.
- **I1–I9** as above (lot key, consumption sites, fee conservation, NIIT/loss-limit indexing, incremental-delta objective + double-count, hard-blocker gating, hypothetical-disposal entrypoint, qualified dividends).

Because C1/C2/I7 reshape **A** (the standing-order object model, the evaluate entrypoint, and the Mode-1/attestation contract) and C3 touches A's interaction with the existing §7.4 transition, **A cannot be planned safely until these folds land.** The "spec all three, then build A→B→C" sequencing is otherwise correct.

*Reviewer's note (process):* the spec's "verified 2026-06-29" header did not catch Notice 2026-20 (issued ~2026-03) — re-verify the identification-timing citations against current IRS guidance at fold time (STANDARD_WORKFLOW §4 "verify citations at write time").

---

# Round 2 — fold re-review (post-fold, 2026-06-29)

**Artifact re-read:** `design/SPEC_lot_optimization_program.md` (folded; "revised after R0 round 1", fold record lines 230-259).
**Reviewer hat:** principal architect + US-crypto-tax reviewer (independent, adversarial), §6 rubric.
**Engine facts re-verified against current source** (not the draft): `identity.rs` (`LotId{origin_event_id,split_sequence}` 116-120, derives `Ord`); `state.rs` (`BlockerKind` hard/advisory set 22-49; `Lot.usd_basis`=gain basis, `dual_loss_basis`); `event.rs` (`SafeHarborAllocation` payload 155-161 has **no** method field; decision variants); `pools.rs` (`consume_fifo`, `PoolKey` Universal/Wallet 7-19); `transition.rs` (`universal_snapshot` folds **only** pre-2025 via shared `fold_event`, sums `usd_basis` 25-51); `resolve.rs` (conservation guard `alloc_basis != snap.basis`→`SafeHarborUnconservable` 547-553; single snapshot at 520; irrevocable-void→`DecisionConflict` 589-599); `fold.rs` (six `consume_fifo` sites: `consume_fee` 232 / Dispose 367 / PendingOut 483 / SelfTransfer 526 / GiftOut 745 / Donate 811; `rehome_onto_lot`→`relocated.last()` 186-196,570; `Pre2025MethodNote` string 28-41); `mod.rs` (`LotMethod` FIFO-only, `ProjectionConfig.lot_method` mutable flag 20-29); `conventions.rs` (`TaxDate=Date` 10, `TRANSITION_DATE=2025-01-01` 17).

## 0. Verdict (up front)

**Substantially green, but not yet 0I.** All three Criticals (C1, C2, C3) and all nine Importants (I1–I9) are **genuinely closed** — each verified against the spec text **and** the current engine source, not merely asserted in the fold record. The reshape of Sub-project A is sound and introduces **no new Critical**. It introduces **one new Important** (a residual post-hoc back-door in C.2's default-`accept` wording, inconsistent with A.5's own `Contemporaneous` definition) plus several Minors/one Nit. The new Important is a **one-line clarification fold**, not a redesign.

**A's own surface (A.1–A.7) is fully closed and unambiguous;** the lone Important lives in C.2 (Sub-project C prose). Per the hard 0C/0I gate the spec is **not yet green**, so fold R2-I1 and re-review (§2) before proceeding; after that one-liner, the design is sound enough to proceed to per-sub-project planning (A first).

## 1. Critical closures — confirmed

### C1 — compliance model: CLOSED.
- "Adequate ID by the time of sale; **no post-hoc identification in any year**" is now the load-bearing premise (Legal grounding **line 13**), repeated at **§A.5 line 112** and **Cross-cutting line 210** ("no artifact… may describe post-hoc selection as compliant").
- **Notice 2026-20 present with the correct boundary** (**line 23**): extends own-books relief to **2025-01-01 → 2026-12-31**, §4.05 books-control-over-mismatched-1099-DA, and **after 2026-12-31 broker-held requires specifying to the broker** (own-books insufficient). Net timing rule stated three-ways (self-custody / broker 2025-26 / broker 2027+) at **lines 24-27**.
- **Self-custody "no relief ever"** — **lines 18, 25, 112**.
- **Binding levers** = (a) dated standing order, (b) contemporaneous `select-lots`, (c) Mode-2-before-selling (**§A.5 lines 107-110**).
- **Mode 1 = what-if-by-default, non-binding** (**§C.2 line 187** "Nothing is filed or bound by running it"); persistence only behind the narrow contemporaneous-ID attestation **within the permitted envelope** (own-books 2025-2026; **never 2027+ broker-held**) (**lines 189-192**), with "must not auto-attest on `optimize accept`" and "refuse to invite a false attestation" (**line 190**).
- **Per-disposal compliance status** surfaced via the `DisposalCompliance` projection over every method-honoring disposal (**§A.5 lines 114-118**: `StandingOrder`/`Contemporaneous`/`AttestedRecording`/`NonCompliant`), reported by `verify` (**line 120**) and `optimize` (**line 193**).
- **Implementability check (new):** the broker-vs-self-custody distinction the model rests on is real in the engine — `WalletId::Exchange{…}` vs `SelfCustody{…}` (identity.rs:110-113) — so the 2027+ broker-held rule is computable. (Pin the mapping; see R2-M5.)
- Residual: the C.2 default-`accept` wording (R2-I1 below).

### C2 — dated `MethodElection`: CLOSED, sound, and (with the noted clarification) unambiguous.
- `EventPayload::MethodElection { effective_from: TaxDate, method: LotMethod }` (**§A.1 lines 43-49**) replaces the mutable `lot_method` flag — and that flag genuinely exists today (mod.rs:28), so "removed" is accurate (**line 135**).
- Applies to **per-wallet disposals on/after `effective_from`**; **FIFO before any election**; **latest-in-force by `effective_from`, ties by `decision_seq`** (**lines 49, 92**) — a total order.
- **Back-dating rejected:** `effective_from` may not precede the made-date → hard `MethodElectionBackdated` (**line 49**). This is what structurally kills the round-1 C2 defect: an election made today cannot retroactively re-order a prior disposal (its date < `effective_from` ⇒ not in force). Verified no path manufactures retroactive HIFO/LIFO.
- **Method/pool alignment is clean** (probed per the prompt): `pool_key` routes date `< TRANSITION_DATE`→Universal, else Wallet (pools.rs:13-19), and the applicable-method rule mirrors it exactly — pre-2025⇒`pre2025_method`, post-2025⇒in-force election (**§A.3 lines 90-92**). A disposal **exactly on** an election's `effective_from` ⇒ applies (on/after = ≥, in-force = ≤; both inclusive). **Multiple elections** ⇒ decision_seq tiebreak. **`universal_snapshot` is unaffected** by elections (it folds pre-2025 only; elections govern only Wallet pools) — so the conservation baseline is not perturbed by a forward standing order. No "ambiguous applicable method" case found. (Minor edge — election with `effective_from < TRANSITION_DATE`, and voided-election exclusion — see R2-M4.)

### C3 — method-in-force bound to the allocation: CLOSED, and consistent with §7.4.
- **Method bound to the allocation; conservation computed against the recorded method** (**§A.7.1 lines 130**), **ordering rule** (method declared before allocation; later change = material gate re-entry, **A.7.2**), **dedicated `Pre2025MethodConflictsAllocation` blocker — never the generic `SafeHarborUnconservable`** (**A.7.3**), composition + conflict KATs (**A.7.4**).
- **Real-defect confirmed and the fix is correct:** `snap.basis` is method-dependent because `universal_snapshot` sums the post-consume `usd_basis` of the Universal residue (transition.rs:49), and consume order will now key off `pre2025_method`; the guard checks `alloc_basis == snap.basis` (resolve.rs:547). Binding the recorded method makes the conserved residue stable.
- **No deadlock, no conflict with existing irrevocability** (probed per the prompt): an effective allocation is irrevocable — a void of it yields `DecisionConflict` (resolve.rs:589-599) — so the user cannot "repair" by rewriting it. But `pre2025_method` is a **side-table**, so the escape hatch from a `Pre2025MethodConflictsAllocation` blocker is to **revert the method** to the recorded one; the irrevocable allocation correctly *pins* the method. This is strictly better than the round-1 behavior (generic "bad data" blocker inviting an irrevocable rewrite). Blocker is correctly scoped to **effective** allocations only (A.7.2/A.7.4), so an inert/Path-A allocation never strands a method change. (Storage/plumbing implications: R2-M2.)

## 2. Important closures — confirmed

- **I1 (LotId key): CLOSED.** `LotPick{ lot: LotId, sat }` keys on full `LotId`; CSV `disposal_ref,origin_event_id,split_sequence,sat` (**§A.2 lines 58-66**). Rationale verified against source: `rehome_onto_lot` adds gain-basis cents onto `relocated.last()` and **drops** the loss-basis fragment for non-dual survivors (fold.rs:186-196), so two fragments of one origin genuinely differ in per-sat basis — `acquire_ref` could not choose deterministically. (Decision-origin lot-ids exist — Path-B seed lots use the allocation `EventId` as origin, resolve.rs:574 — so CSV parsing must round-trip all three `EventId` variants; R2-M4.)
- **I2 (consume sites): CLOSED.** A.3 table (**lines 70-81**) enumerates exactly the six real `consume_fifo` sites; Dispose/GiftOut/Donate/SelfTransfer honor method+selection, PendingOut + fee leg stay FIFO; `select-lots` on a non-honoring op → `LotSelectionInvalid` (**line 81**). Verified 1:1 against fold.rs.
- **I3 (principal-only conservation + fee FIFO): CLOSED.** A.4(a) (**line 99**): `Σ picked == principal sat` excluding `fee_sat`; fee consumes FIFO from the post-selection remainder. Matches the real order — principal `consume_fifo` then `consume_fee` after (fold.rs:367,383). Fee-bearing-reclassified-disposal KAT promised.
- **I4 (NIIT/$3k statutory non-indexed): CLOSED.** B.2 (**lines 152-158**) classifies indexed vs **fixed-by-statute** (NIIT thresholds, $3k/$1.5k), sources structural changes to enacted law, KAT asserts NIIT/$3k constant across years; restated in Legal grounding line 30.
- **I5 (incremental delta + crypto ordinary once): CLOSED.** `ordinary_taxable_income` excludes **all** app-computed crypto items incl. ordinary income (**B.1 line 146**); added back exactly once on the stack (**B.3 line 165**); objective = `tax(with) − tax(without)` ceteris-paribus, labeled not-a-1040 (**B.3 line 167**).
- **I6 (refuse on hard blockers): CLOSED.** B emits `TaxYearNotComputable` (no number) for a year touched by an unresolved hard blocker; C refuses such a year (**B.4 line 171, C.1 line 185**). (Gate should key on Hard *severity* generally, not the enumerated subset — R2-M3.)
- **I7 (synthetic-disposal evaluate): CLOSED.** A.6 (**lines 122-124**): side-effect-free entrypoint accepts an arbitrary candidate disposal (existing **or** synthetic `Eff` appended), same consume/validation/scoring path (the `universal_snapshot` clone-append-fold-discard pattern, which is exactly transition.rs:36-46); requires `--proceeds` when no price exists for the date. Built in A before C.
- **I9 (qualified dividends in §1(h) stack): CLOSED.** `qualified_dividends_and_other_pref_income` added (**B.1 line 148**), stacked with crypto LT for 0/15/20 breakpoint placement (**B.3 line 163**); 15→20-by-QD KAT (**line 177**). Tax-law treatment correct (QD + net LT share the preferential brackets on top of ordinary income).

Minors/Nits **M1–M7, N1–N3** are all folded (lines 250-259); see R2-M1/M5/N1 for residue.

## 3. New-defect scan of the reshaped surface (the critical part)

Probed exactly the prompt's four vectors. Result: **0 new Critical; 1 new Important; 5 Minor; 1 Nit.**

### IMPORTANT (new)

#### R2-I1 — C.2's default-`accept` keys on *filing status* ("not-yet-filed"), not the legal *time-of-sale* test, contradicting A.5's own `Contemporaneous` definition — a residual post-hoc back-door
**Where:** §C.2 **line 193** ("`optimize accept` defaults to **current, not-yet-filed** disposals being recorded essentially contemporaneously") vs §A.5 **line 116** (`Contemporaneous` = "a `LotSelection` recorded **at/before the time of sale**").
**What:** The legal boundary C1 establishes is **time of sale**, not filing. A disposal can be *current/not-yet-filed* **and** *after its time of sale* (e.g., executed three weeks ago, return due next April). The C.2 default would persist a tax-minimizing selection for such a disposal as the filed position "essentially contemporaneously" **without attestation** — which is precisely the undocumented-FIFO-reported-as-HIFO position C1 forbids. A.5 line 116 gives the correct mechanical test (made-date ≤ time of sale); C.2 line 193 does **not** cite it and substitutes a filing-status trigger with no pinned contemporaneity test. The two sections are internally inconsistent on the load-bearing compliance contract, and an implementer following C.2's text literally would build the back-door.
**Why Important (not Critical):** the spec's dominant guidance is correct (line 13 "no post-hoc in any year"; line 190 "must not auto-attest / refuse to invite a false attestation"; A.5 line 116 has the right test). This is an under-pinned/inconsistent clause, not an affirmative decision to allow post-hoc — fixable in one line, but it must reach 0-ambiguity because it is the whole north-star.
**Fix:** In C.2, make default-`accept` confer `Contemporaneous` **only** when the selection's decision made-date is at/before the disposal's time of sale (the A.5 line 116 test; mechanically checkable, the mirror of `MethodElectionBackdated`). Any already-executed disposal (made-date after disposal date) must route through the attestation gate (→ `AttestedRecording`, envelope-checked) or be `NonCompliant`/read-only what-if. "Not-yet-filed" must never by itself confer contemporaneous status. (Consequence — correct and intended: Mode-1 auto-accept over already-in-ledger disposals is essentially never `Contemporaneous`; that is C1 working as designed.)

### MINOR (new / residual)

- **R2-M1 — HIFO dual-basis zone key is under-pinned at ordering time.** §A.3 line 88: "for a dual-basis gift lot whose disposal lands in the **loss zone** the effective basis is `dual_loss_basis`." Zone depends on per-sat proceeds, which is known (disposal total ÷ disposal sat) but the spec doesn't state that the zone is determined from the **disposal's per-sat net proceeds** at ordering time. The total-order property still holds (the tiebreak chain is total for any deterministic key), but pin the zone-determination input so the HIFO key is reproducible. (Documented simplification + KAT already promised.)
- **R2-M2 — C3 storage/plumbing.** A.7.3 names "recorded vs attempted method" in the blocker, which **requires the recorded method to be retained immutably with the allocation** — but `SafeHarborAllocation` (event.rs:155-161) has no method field, and `pre2025_method` is a *mutable* side-table. A's plan must add an immutable carrier (e.g., an optional field on the allocation payload, serde-default) and make `universal_snapshot` method-aware: today it is computed **once** with the live `config` (resolve.rs:520); A.7's "conserve against the recorded method" implies evaluating it under the allocation's recorded `pre2025_method`. Pin whether the snapshot is recomputed per candidate allocation or collapsed to one by the precedence rule.
- **R2-M3 — I6 refusal should gate on Hard *severity* generally.** B.4's enumerated trigger list (line 171, "e.g.") omits Hard kinds `DecisionConflict`, `Unclassified`, `SafeHarborUnconservable` (state.rs:39-45). Gate `TaxYearNotComputable` on any Hard blocker touching the year's disposals so future hard kinds are auto-covered.
- **R2-M4 — MethodElection/LotSelection resolve-pass edges to pin in A's plan:** (a) an election with `effective_from < TRANSITION_DATE` governs post-2025 disposals only (pre-2025 stays `pre2025_method`) — state it so it cannot be read as reaching closed years; (b) a **voided** `MethodElection` must be excluded from the in-force computation (mechanical via the existing `voided` set, resolve.rs:269-303) — add to the A test list (currently lists backdate/latest-in-force but not void); (c) **duplicate `LotSelection` for one disposal** must resolve to latest-wins-or-`DecisionConflict` (mirror duplicate-`ReclassifyOutflow`, resolve.rs:459-468); (d) `select-lots`/CSV must parse all three `EventId` origin variants (Decision-origin lots exist for Path-B seeds, resolve.rs:574).
- **R2-M5 — Pin the WalletId→custody mapping for `DisposalCompliance`.** A.5's broker-held-2027+ rule needs a concrete mapping: `WalletId::Exchange{…}` ⇒ broker-held; `SelfCustody{…}` ⇒ self-custody (identity.rs:110-113). State it so the compliance projection is unambiguous.

### NIT (citation accuracy)

- **R2-N1 — `Pre2025MethodNote` "already done" is false against current source.** §A.5 line 120 says the advisory "reflects the **declared** pre-2025 method (already done in the burndown commit … never hard-coded 'FIFO')." Current source still hard-codes the literal string "pre-2025 lots reconstructed under **FIFO** (the legal default, §7.4)…" (fold.rs:38). Either the note is not yet method-aware (most likely) or the citation is stale — correct the parenthetical, and keep N1 as a genuine A task rather than a closed item. (STANDARD_WORKFLOW §4: verify citations at write time.)

## 4. Other vectors explicitly cleared

- **Tax-law:** no new misstatement from the reshape. MethodElection-as-standing-order is sound under (j)(3)(ii) ("by a standing order"), gated to recorded-before-sale by the backdate blocker; the 2027+ broker-held carve-out is correctly *not* satisfied by an own-books `MethodElection` alone (A.5 line 115).
- **Determinism (NFR4):** in-force election lookup (effective_from ≤ date, tie decision_seq) and the method orderings (A.3) are total; pools remain `BTreeMap`; no `HashMap` iteration / float introduced. The boundary seed still fires by tax-date for ≥2025 (fold.rs:281-295), so a 2025-01-01 disposal under an election consumes a seeded Wallet pool — no gap.
- **Event-sourcing / fingerprint:** `MethodElection`/`LotSelection` are appended voidable `EventId::Decision` decisions with `fingerprint = None` (Cross-cutting line 213, consistent with `SafeHarborAllocation`); new `BlockerKind` variants are computed-only (not serialized in the event log), so additive without serde risk — append at the enum tail to avoid churning the `finalize` blocker-sort snapshots (fold.rs:888).

## 5. Round-2 verdict

**C1, C2, C3 and I1–I9 are closed** (verified against spec + current source). The Sub-project A reshape introduced **no new Critical**. **One new Important (R2-I1)** — a one-line tightening of C.2's default-`accept` to A.5's time-of-sale test — keeps the spec from being 0I. Fold R2-I1 (and ideally R2-M1…M5/N1), re-review per §2, and the program spec is green; **A may then proceed to planning.** A's own object model (A.1–A.7) is already sound, so the remaining work is a localized clarification, not a redesign.

---

# Round 3 — fold re-review (post-R2-fold, 2026-06-29)

**Artifact re-read:** `design/SPEC_lot_optimization_program.md` (folded again; "revised after R0 round 1–2", fold records lines 234–279).
**Reviewer hat:** principal architect + US-crypto-tax reviewer (independent, adversarial), §6 rubric.
**Scope (focused):** confirm the **1 Important (R2-I1) + 5 Minor (R2-M1…M5) + 1 Nit (R2-N1)** that round 2 raised are genuinely closed by this fold, and that the fold introduced **no new Critical/Important**. Earlier-round Criticals (C1/C2/C3) and Importants (I1–I9) were confirmed closed in round 2 and are **not** re-litigated (the R2 fold did not disturb them — spot-checked: the compliance premise line 13, the `MethodElection` backdate kill, and the C3 method-binding are all intact).
**Engine facts re-verified against current source at this review's write time** (not the draft): `SafeHarborAllocation` payload (event.rs:155–161) carries `lots`/`as_of_date`/`method: AllocMethod`/`timely_allocation_attested` — **no** lot-consumption-method field; `AllocMethod = ActualPosition|ProRata` (event.rs:140–143), i.e. the *allocation* method, confirming a new carrier is required; `universal_snapshot` computed **once** with live `config` at resolve.rs:520; conservation guard `alloc_basis != snap.basis → SafeHarborUnconservable` (resolve.rs:547–553); void-of-effective-allocation → `DecisionConflict` (resolve.rs:589–599) and multiple-effective → `DecisionConflict` (resolve.rs:601–615); Path-B seed lots use the allocation **`Decision`** EventId as `origin_event_id` (resolve.rs:571–572); `EventId` = `Import`/`Conflict`/`Decision` with `canonical()` string forms (identity.rs:56–105); `WalletId::Exchange{provider,account}` / `SelfCustody{label}` (identity.rs:110–113); `LotId{origin_event_id,split_sequence}` derives `Ord` (identity.rs:116–120); `BlockerKind::severity()` Hard set = FmvMissing/UncoveredDisposal/ImportConflict/DecisionConflict/UnknownBasisInbound/Unclassified/SafeHarborUnconservable (state.rs:36–48); duplicate `ReclassifyOutflow → DecisionConflict` (resolve.rs:459–468); `voided` set (resolve.rs:269–303); `Lot.usd_basis` = gain basis / `dual_loss_basis: Option<Usd>` / `basis_pending` (state.rs:64–68); a basis-pending (FMV-missing) lot gets `usd_basis = Usd::ZERO` (fold.rs:629); `Pre2025MethodNote` **still hard-codes "FIFO"** (fold.rs:38).

## 0. Verdict (up front)

**GREEN. 0 Critical / 0 Important.** R2-I1 is genuinely closed; R2-M1/M2/M3/M4/M5 and R2-N1 are correctly applied; the fold introduced **no new Critical or Important** and no determinism gap, tax-law misstatement, or §7.4/conservation contradiction. The spec is ready for **per-sub-project planning, A first**. One non-blocking **Nit** (line-citation drift) is recorded below for §4 hygiene; it does not gate.

## 1. R2-I1 — CLOSED (the load-bearing one)

The default-`accept` gate now keys on the **A.5 `Contemporaneous` test**, not filing status, and the two sections are mutually consistent:

- **A.5 (line 118)** defines `Contemporaneous` = "a `LotSelection` whose **made-date is at/before the disposal's date-and-time of sale** (the canonical test; **not a filing-status proxy**)" — reinforced at the binding-lever list (line 111: "filing status is irrelevant to this test").
- **C.2 (line 197)** now reads: `optimize accept` confers `Contemporaneous` (persists without attestation) **only** for a disposal whose `LotSelection` made-date is **at/before that disposal's date-and-time of sale** — "the **A.5 `Contemporaneous` test** … the mirror of `MethodElectionBackdated`, **not** a filing-status trigger." The old "current / not-yet-filed" trigger is explicitly **negated**: "Being 'current / not-yet-filed' never by itself confers contemporaneous status (a disposal can be unfiled yet already executed — return due next April for a sale three weeks ago — and that is exactly the post-hoc position C1 forbids)."
- **No silent post-hoc persist-by-default path remains.** Any **already-executed** disposal (made-date *after* its time of sale) is routed to the narrow contemporaneous-ID **attestation gate** (→ `AttestedRecording`, envelope-checked per C.2(1)/(2)) **or** marked `NonCompliant` and left read-only what-if (FIFO is its defensible filing position). The C tests (line 205) mirror this exactly ("an already-executed disposal … is NOT auto-persisted … no post-hoc-by-default"). The stated consequence — "Mode-1 auto-accept over disposals already in the ledger is essentially never `Contemporaneous`" — is C1 working as designed, not a leak.
- **No residual filing-status gate.** The only surviving "filing position" reference (line 191, "incremental tax delta vs the **current filing position** (FIFO, or the in-force standing order)") is the **what-if baseline** for the delta, not a persistence trigger — correct and harmless.

A.5's definition and C.2's reference use **identical** operative wording (made-date ≤ disposal's date-and-time of sale). Internally consistent. **R2-I1 closed; the fold introduced no new defect here.**

## 2. R2-M2 — SOUND (method carrier + method-aware snapshot, vs §7.4)

The C3 plumbing is now implementable and does **not** contradict §7.4 irrevocability or regress conservation:

- **Immutable carrier, correctly motivated.** A.7.1 adds an **immutable, serde-`default`ed `pre2025_method` field to the `SafeHarborAllocation` payload**, captured at attestation. The justification checks out against source: the existing `method` field is `AllocMethod` = `ActualPosition|ProRata` (event.rs:140–143, 159) — the *allocation* method, **not** the lot-consumption method — so a distinct field is genuinely required (not a re-use). serde-`default` → `Fifo` is correct for backward-compat: the engine is FIFO-only today (`LotMethod` FIFO-only), so every already-persisted allocation was in fact attested against the FIFO residue.
- **Method-aware snapshot is conservation-stable.** Today `universal_snapshot` is computed **once** with the live `config` (resolve.rs:520, implicitly FIFO); the guard checks `alloc_basis == snap.basis` (resolve.rs:547). Computing each effective allocation's residue under **its own recorded method** means the conserved residue is **stable across later config changes** — precisely the property that prevents the round-1 C3 trap (an irrevocable allocation going `SafeHarborUnconservable` on a config flip). The "collapses to one snapshot in a clean state" claim is valid: ≤1 allocation is ever effective (multiple → `DecisionConflict`, resolve.rs:601–615) and the A.7.2 precedence rule keeps the recorded method aligned with the single pre-allocation declaration.
- **§7.4 preserved; no coercion.** The allocation payload (including its recorded method) is **never rewritten** — only the **revertible side-table** `pre2025_method` config is. `Pre2025MethodConflictsAllocation` fires precisely on **live config ≠ recorded method** (a dedicated Hard blocker, never the generic `SafeHarborUnconservable`), and the escape hatch is to **revert the live config to the recorded method**. The void-of-effective-allocation → `DecisionConflict` path (resolve.rs:589–599) is untouched, so the irrevocable allocation still cannot be voided/rewritten. This is strictly better than round-1 behavior and is internally consistent.
- **No conservation regression / no wrong number.** When live = recorded (clean state), the pre-2025 disposal fold and the conservation snapshot use the same method → consistent. When live ≠ recorded, the Hard `Pre2025MethodConflictsAllocation` blocker fires → the year is gated by B.4 (no `TaxResult` emitted) → no incorrect figure is ever surfaced. **R2-M2 sound.**

## 3. R2-M1 / M3 / M4 / M5 / N1 — applied correctly

- **R2-M1 (HIFO gain-basis key): correct, and cleaner than asked.** A.3 (lines 86–88) pins HIFO = **gain basis (`usd_basis`) per sat** desc, tie → oldest `acquired_at`, tie → `lot_id` asc — a **total order** with **no proceeds/zone input** at ordering time (loss-basis `dual_loss_basis` explicitly does **not** reorder). This dissolves the round-2 concern (zone-determination input unpinned) by removing zone-dependence entirely from the standing order; the legitimate zone-aware result lives in C's evaluate path (correctly noted as a permitted divergence). Verified against source: `usd_basis` is the gain basis (state.rs:64) and a basis-pending lot has `usd_basis = Usd::ZERO` (fold.rs:629) — so "basis-pending sorts last" under a descending key is exact, not aspirational. Tax-law: a standing order need not be the per-disposal optimum, so this is a sound simplification, not a misstatement.
- **R2-M3 (Hard-severity gating): correct.** B.4 (line 175) keys `TaxYearNotComputable` on `BlockerKind::severity() == Severity::Hard` (state.rs:36–48), **not** an enumerated subset, auto-covering future hard kinds; the enumerated current Hard set in the prose matches state.rs:39–45 exactly, and the three new kinds are all added at Hard.
- **R2-M4 (resolve-pass edges): all four pinned and source-accurate.** (i) `MethodElection.effective_from ≥ TRANSITION_DATE` else rejected (A.1 line 49) — forward-only, never reaches closed pre-2025 years; (ii) **voided** `MethodElection`/`LotSelection` excluded via the existing `voided` set (resolve.rs:269–303, confirmed); (iii) two non-voided `LotSelection`s for one `disposal_event` → `DecisionConflict`, explicitly mirroring duplicate `ReclassifyOutflow` (resolve.rs:459–468, confirmed); (iv) CSV `LotId` parsing round-trips all **three** `EventId` origin variants via `canonical()` (identity.rs:56–105, confirmed), with the correct rationale that Path-B seed lots carry a **`Decision`** origin (resolve.rs:571–572). The test list (line 140) carries each as an A KAT.
- **R2-M5 (WalletId → custody mapping): correct.** A.5 (line 122) maps `Exchange{provider,account}` = broker-custodied (2027+ broker-communication rule), `SelfCustody{label}` = self-custody (own-books, all years) — verified verbatim against identity.rs:110–113. Makes the `DisposalCompliance` projection unambiguous and the 2027+ rule computable.
- **R2-N1 (Pre2025MethodNote): correctly downgraded to a live A task.** A.5 (line 124) + artifacts (line 140) now state the advisory **still hard-codes "FIFO"** and that rendering the declared method is a **not-yet-done Sub-project-A task** — verified against fold.rs:38 (still "pre-2025 lots reconstructed under FIFO …"). The stale "already in the burndown commit" claim is removed. Citation is now truthful per §4.

## 4. New-defect scan of the R2-folded surface — 0 Critical / 0 Important

Probed the touched sections (A.1, A.2, A.3, A.4, A.5, A.7, B.4, C.2, artifacts line 140, Cross-cutting line 216) for inconsistency, determinism gaps, tax-law misstatements, and §7.4/conservation contradictions:

- **Determinism (NFR4):** every new/changed ordering is total — in-force election lookup (`effective_from ≤ date`, tie `decision_seq`), HIFO gain-basis key (tie oldest→`lot_id`), voided-exclusion, duplicate-selection→`DecisionConflict`. No `HashMap` iteration or float introduced. The boundary case (disposal exactly on `effective_from`, and on `TRANSITION_DATE`) is well-defined (inclusive ≥/≤; `2025-01-01` routes to a Wallet pool). **No gap.**
- **Tax-law:** custody mapping, the 2027+ broker carve-out (own-books `MethodElection` alone does not satisfy it), the contemporaneous time-of-sale boundary, and HIFO-ignores-loss-basis (a standing-order simplification) are all correct. **No misstatement.**
- **§7.4 / conservation:** covered in §2 — allocation never rewritten; conservation computed under the recorded method; conflict is a revertible-config blocker. **No contradiction; no regression.**
- **Internal consistency:** A.5↔C.2 (R2-I1) now identical wording; A.7.1↔artifacts↔Cross-cutting line 216 describe the same carrier + method-aware snapshot; B.4's new Hard kinds match the artifact list. **Consistent.**

**Nit (non-blocking, §4 citation hygiene):** two line-citations drift by 1–2 lines from current source — A.2/A.7.1 cite **resolve.rs:574** for the Path-B seed-lot `origin_event_id`, which is actually assigned at **resolve.rs:571–572** (inside the `Lot` built at 570–585); and "event.rs:156–161" for the `SafeHarborAllocation` payload points at the struct whose declaration starts at line **155**. Both point at the correct code block and the substantive claims hold; refresh the exact line numbers at each plan's write time (STANDARD_WORKFLOW §4). Nit only — does not gate.

## 5. Round-3 verdict

**R0 is GREEN: 0 Critical / 0 Important.** R2-I1 is genuinely closed (no silent post-hoc persist-by-default path; A.5 and C.2 mutually consistent on the time-of-sale test); R2-M2 is sound and §7.4-safe; R2-M1/M3/M4/M5/N1 are correctly applied and source-accurate; the fold introduced no new Critical or Important. Per the §2 loop (re-review after the last fold, including this one) the program spec passes the gate. **Proceed to per-sub-project planning, A first.** Only residue: one non-blocking citation-precision Nit and the already-tracked live A tasks (e.g., `Pre2025MethodNote` rendering, R2-N1) to be executed during A's plan/implementation.
