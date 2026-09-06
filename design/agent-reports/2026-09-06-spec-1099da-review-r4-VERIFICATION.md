# VERIFICATION ledger — spec-1099da r4 review (2026-09-06-spec-1099da-review-r4.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `2c620b63`.

| finding | claim | check | verdict |
|---|---|---|---|
| I-new-1 | (J)'s "services" limb is bounded to (a)(9)(ii)(B)/(C) — services a BROKER renders; the 1099-DA instructions define a covered security as acquired "for cash, stored-value cards, different digital assets, or any property or services the disposition of which the broker is required to report", and say "Do not report rewards and staking payments on Form 1099-DA" | CFR XML (J) full text: "…in exchange for cash, stored-value cards, different digital assets, or any other property or services described in paragraph (a)(9)(ii)(B) or (C) of this section, respectively."; (a)(9)(ii)(C) located (see below); `Instructions_1099-DA.txt:41-57` and `:364-366` verbatim | **TRUE** — r4's Covered cell was a misreading through the spec's own ellipsis |
| M-new-1 | `parse_wallet_id` accepts `self:LABEL` for any venue | `eventref.rs:58-73` "use exchange:PROVIDER:ACCOUNT or self:LABEL" | **TRUE** |
| M-new-2 | the fold sees `Consumed`, not the `Lot`; `Consumed.acquired_at` = `lot.acquired_at` | `pools.rs:321` `pub acquired_at: TaxDate`; `:243` `acquired_at: lot.acquired_at`; `fold.rs:360` `legs.push(DisposalLeg {` | **TRUE** |
| M-new-3 | `leg.acquired_at` equals the lot date outside the gift arms | `fold.rs:302-359` / `state.rs:184-187` (reviewer's trace; consistent with the HP-start doc) | **TRUE** (by reading) |
| M-new-4 | no kill for "TY2026, zero exchange dispositions, slice arm → fills" | the spec's T1 text (mine) | **TRUE** |
| M-new-5 | absent identification, §1.1012-1(j)(3)(i) treats the earliest-acquired units in the broker's custody as sold | `26CFR_1.1012-1_basis.xml` is archived; the (j)(3)(i) sentence located (see below) | **TRUE** |
| M-new-6 | exchange→exchange transfers relocate to `CarriedFromTransfer` | `fold.rs:1094-1097` | **TRUE** |
| N-1 | `Notice_2026-20.txt:367` carries a bare page number `8` mid-sentence | `sed -n 366,368p` | **TRUE** |
| N-2 | the slice bullet's heading still says "basis-regime year" | `SPEC…:181` | **TRUE** |
| N-3 | 51 `DisposalLeg {` hits; ~26 struct-update sites | grep: 51 hits, 27 `..base_leg()` | **TRUE** (27, not 26) |
| N-4 | `ReturnInputs` has no struct-level `Default` derive | `return_inputs.rs:913-915` (manual `Default`) | **TRUE** |

**Verdict: 11/11 TRUE, 0 refuted.**
