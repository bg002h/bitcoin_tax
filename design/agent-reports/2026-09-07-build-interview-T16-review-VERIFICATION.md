# Verification ledger — interview T16 seam review (`2026-09-07-build-interview-T16-review.md`, 1C/3I/2M/1N)

Controller machine-checks from the main tree at `adcf4837`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| C-1 | `hsa_income_8f` (Form 8889 line 16 + 20) reaches only the printed Schedule 1 and never `schedule_1_income` / `total_income` / `agi` | `sed` the `schedule_1_income` sum; `grep hsa_income_8f` | the sum is `state_refund_taxable + unemployment + schedule_c_net + crypto.nonbusiness_ordinary` — no 8f term; `hsa_income_8f` appears only at `:324` (the struct), `:2136` (derived), `:2164` (placed into `Schedule1Parts`) | true — the review's probe (AGI short by exactly the taxable distribution; a filed §221 deduction overstated by $1,500) follows; **FOLD** |
| I-1 | `schedule_2_other_taxes` omits Schedule 2 lines 17c/17d; the equality instrument runs on two non-HSA fixtures | `grep -A3 'let schedule_2_other_taxes'`; `grep hsa packet.rs` | `se_tax + additional_medicare + niit` only; `packet.rs` mentions `hsa` once (a different test setting it `None`); the loop holds `kitchen sink` and `AMT-owing` | true; **FOLD** — and the instrument becomes structural (below) |
| I-2 | the code-W contradiction rule fires on `None` in the import tier | `grep` | `return_refuse.rs:2374 … ri.sch1.hsa_activity != Some(true)` inside the unconditional W-2 loop | true; **FOLD** (`== Some(false)`) |
| I-3 | the coverage prompt asks about the filer's own plan only; the instructions say *"you or your spouse"* | `grep spouse` in the `HsaFamilyCoverage` entry | 0 hits | true; **FOLD** (collect the spouse's plan; the instruction's words) |
| M-1 | no channel for a distribution with no 1099-SA | read | true | fold (R3's door pattern) |
| M-2 | `line-coverage` checks Form 8889 against the 2024 extract only | `grep 'Coverage::quoting("2024")'` | two sites (`:411`, `:649`) | fold (cover both editions) |
| N-1 | the line-9 census label names slot 12a while the code sums every W slot | read | true | fold inline |
| (a)–(e) | the pointed questions | the review's seams-checked-clean section | (a) `family_coverage` `None` blocks; (b) code W never double-counted; (c) Part III refusal names the rule, reaches pinned; (d) every worksheet path refuses naming it; (e) both oracles reproduce the cell exactly | recorded |

## Disposition — the shape
This is the repo's recorded defect class (§G-6, `a-figure-with-no-reader`): two chains, and the
instrument that compares them runs over a HAND-LISTED set of households, so a new term added to
the printed chain and not the computed one passes. The fold wires the two legs AND makes the
instrument structural: `the_absolute_total_tax_equals_the_printed_1040_line_24` and a new AGI twin
(`ar.agi == printed 1040 line 11`) run over the coverage fixture that populates EVERY leaf
(`coverage.rs::maximal_fixture`) in addition to the named households — so any future leaf that
reaches a printed line without reaching the absolute chain reds by construction, never by
someone remembering to add a fixture.
- **C-1 / I-1 — FOLD** as above; the reach test asserts `round_dollar(ar.agi) == f1040.line11` and
  `round_dollar(ar.total_tax) == f1040.line24` on the HSA household with the anti-vacuity guard
  (the fixture must produce a non-zero 8f and 17c).
- **I-2 — FOLD**: `ri.sch1.hsa_activity == Some(false)`; a `None` is the registry's to block.
- **I-3 — FOLD**: `HsaSpouseFamilyCoverage` (class A, live iff a spouse exists — MFJ or MFS, per
  *"regardless of whether you file jointly or separately"*), OR'd into `coverage`; the prompt widened
  to the instruction's own sentences (`i8889--2024.txt:460-472, 496-499`); the MFJ self-only /
  spouse-family fixture → the family limit and no excess refusal.
- **M-1 — FOLD** (R3's pattern: `hsa_distribution_without_1099sa` live iff `hsa_activity ==
  Some(true)` and `sa_1099 == Some(false)`; `Yes` refuses naming the trustee's Form 1099-SA).
- **M-2, N-1 — fold inline.**

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T16-review.md`.
