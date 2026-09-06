# VERIFICATION ledger — spec-1099da r3 review (2026-09-06-spec-1099da-review-r3.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `fab4f2fa` + the r3 report staged.

| finding | claim | check | verdict |
|---|---|---|---|
| I-1 | sold-out lots leave `LedgerState.lots`; the state has no event log; so a lot's acquisition event date is unreachable at `form_8949` | `fold.rs:1644-1650` pushes a lot only `if lot.remaining_sat > 0`; `LedgerState` fields: lots, holdings_by_wallet, disposals, removals, income_recognized, pending_reconciliation, blockers, stats, pseudo_synthetic_count, promoted_origins, shortfalls — no events; `promoted_origins` doc (`state.rs:345-352`) is the precedent for carrying a fact the leg alone cannot show | **TRUE** |
| I-2 | rows are emitted for every leg incl. `WalletId::SelfCustody`; R2 gives them no key | `forms.rs:148` `box_needs_review = matches!(leg.wallet, Exchange{..})`; the spec text | **TRUE** |
| I-3 | `BrokerCohortUnknown` refuses with no exit on `FmvAtIncome`/`CardRewardRebate` (FR-45, shipped) | `event.rs:36-51` FR-45 doc; `return_1040.rs:2637-2643` condemns the "escapable only by a false No or deleting a truthful event" shape | **TRUE** |
| I-4 | Rev. Proc. 2024-28 is the §1012(c)(1) safe harbor, not the specific-ID rule; Notice 2026-20 (committed) extends §1.1012-1(j)(3)(ii) relief through 2026-12-31 and lets a standing order on the taxpayer's books govern regardless of the broker's report | `legal/SOURCES.md:36` "Per-wallet basis transition + safe harbor"; `legal/text/irs-guidance/Notice_2026-20.txt:6` title, `:301` "January 1, 2025, and ending on December 31, 2026", `:332` "Recording a standing order on the taxpayer's books and records", `:281` | **TRUE** |
| M-1 | the TOML path is `[broker_reporting.by_provider.<provider>]`; unknown keys are rejected | `tax.rs:219-236` `serde_ignored` → "unknown key(s) in the ReturnInputs TOML" | **TRUE** |
| M-2 | a third `default_year()` caller in `unlock.rs:234` (a fallback) | printed | **TRUE** |
| M-3 | S10 overstates: liveness also needs ≥1 exchange disposition | the spec text | **TRUE** |
| M-4/M-5/M-6 | holding period differs too; `Cohort` has no `Unknown` variant; the two rules overlap | by reading | **TRUE** |

**Verdict: 10/10 TRUE, 0 refuted.**
