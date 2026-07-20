# Conservative-filing IMPLEMENTATION_PLAN — tax re-reviews r3 (GREEN) + r4 (GREEN)

_Two persisted tax-lens verdicts. r3 was the convergence check after the r2 fold; r4 re-confirmed after the r3 Nit-folds + the arch-lens test restage._

## r3 verdict: GREEN — 0 Critical / 0 Important (two non-gating Nits N-5, N-6)

All four r2 findings genuinely resolved:
- **NEW-1 (corner-(b) unreachable under HIFO) — RESOLVED.** The restaged specific-ID-sale KAT is reachable on verified live-engine mechanics (selection-honoring principal → tranche leg last; `consume_fee` draws FIFO from the post-selection remainder — the documented lot; TreatmentC default returns the documented carry; `rehome_onto_disposal_leg` adds it to the last/tranche leg → `cost_basis > 0`). The amended SPEC clause (b) states a true mechanism; the HIFO-impossibility NB is correct; Task-15 Step-2 now has a sanctioned STOP outcome.
- **M-5 (sub-cent rounding) — RESOLVED.** SPEC clause (c) accurate vs `fold.rs:133-140`; the core assert correctly rescoped to fee-free + single-leg.
- **N-4 (Task 9 dichotomy) — RESOLVED.** Keyed on the fee-sat carry, not fee-free-ness; print `leg.cost_basis` directly.
- **N-2 (pre-2025 C/F box) — RESOLVED (adopted).** The 2020-disposal Box-F KAT pins the securities scheme end-to-end.

Tax-completeness re-swept: every G-1..G-4 / D-1..D-10 has an owning pin; no understatement path; $0-only filing intact.

New Nits (neither gates):
- **N-5:** the "≤$0.01" pro-rata rounding cap is loose for ≥4-leg dust disposals — the bound is ≤ ½¢ per prior leg. One-word fix.
- **N-6:** (i) SPEC clause (b)'s compressed "exhausts the documented lots" reads self-contradictory next to the named-lot staging (the NB disambiguates); (ii) the corner-(b) fixture should name the full tranche (or make the documented lot FIFO-first) — a partial naming fails loud.

## r4 verdict: GREEN — 0 Critical / 0 Important (post-Nit-fold + arch test-fix)

`git diff eb0cfa7..a96791d` confirms **docs-only** changes — no engine code touched.

- **N-5 fold — RESOLVED, now strictly correct.** "cent scale … ≤ ½¢ per prior leg" is the true bound (per-leg `round_cents` ties-to-even error ≤ ½¢; last leg absorbs `net − Σ allocated`). Σ-exact split ⇒ no understatement even adversarially.
- **N-6 fold — RESOLVED, both halves.** Clause (b) wording now states the necessary-and-sufficient reachable condition; both named stagings verified live. The fixture footnote's hazard is real (`consume_fifo` is acquisition-date order) and its remedies guarantee the documented draw; fail-loud, never a silent false pass.
- **Arch Task-3 test-fix — TAX-NEUTRAL.** The KAT composes the same `sort_canonical` the production fold applies (fold.rs:381); asserts event ordering only; two same-window $0-basis/same-date tranche lots yield identical tax in any order. No tax claim introduced, no filing behavior changed.
- **Final SPEC §6 invariant sweep — SOUND.** Internally consistent; (a) §1001(b) fee netting, (b) §1011 documented fee-sat basis, (c) Σ-conserving rounding — none understates tax, all documented-or-mechanical, none the estimate. $0-only filing intact; the defective HIFO clause fully excised.

**Both rounds GREEN. The plan is tax-sound: no reachable input produces an understatement, and the $0-only posture holds on every path.**
