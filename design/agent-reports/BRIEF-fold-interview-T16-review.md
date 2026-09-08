# Brief — fold the interview T16 seam review (C-1, I-1, I-2, I-3, M-1, M-2, N-1)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once that RUNS the instrument; quote
the red. Synthetic identifiers only from the never-issued SSN space or `scripts/pii-scan-generic.sh`'s
`ALLOWED_EIN`. The Python stack is `.venv/bin/python`.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T16-review.md` (read whole) and the
ledger `2026-09-07-build-interview-T16-review-VERIFICATION.md` — every claim is machine-verified; its
"Disposition — the shape" fixes the design: the two chains are wired AND the equality instrument
becomes STRUCTURAL (it runs over the fixture that populates every leaf, so the class cannot recur by
omission). The build you are folding: `2026-09-07-build-interview-T16-implementation.md` (commits
`86bc9171` + `4b241dd7`). `CLAUDE.md`'s corollary: *"If the form asks something our input surface
cannot answer, collect it."* The instructions are the authority (`design/forms/extract/i8889--2024.txt`).

## The fold
1. **C-1 — line 8f enters AGI.** In `return_1040.rs`, hoist the `form_8889` / `hsa_income_8f`
   derivation above `schedule_1_income` (it needs only `ri` and `params.hsa`) and add
   `+ hsa_income_8f` to the sum; fix the comment that points at a binding that did not exist.
   Kill: on the build's reach fixture ($3,000 distributed, $1,200 qualified) assert
   `round_dollar(ar.agi) == pr.forms.f1040.line11` and `round_dollar(ar.taxable_income) ==
   pr.forms.f1040.line15` — red before (58000 vs 59800), green after; and the review's second probe
   (wages $85,000, $2,500 of 1098-E interest, a $9,000 excepted distribution): printed Schedule 1
   line 21 = $167, not $1,667.
2. **I-1 — lines 17c/17d enter total tax.** `schedule_2_other_taxes` gains the Form 8889 additional
   taxes (line 17b → 17c; line 21 → 17d) the way the printed lane sums them. Kill:
   `round_dollar(ar.total_tax) == pr.forms.f1040.line24` on the reach fixture (4979 vs 5555 before).
3. **The instrument becomes structural.** `packet.rs::the_absolute_total_tax_equals_the_printed_
   1040_line_24` and a new twin `the_absolute_agi_equals_the_printed_1040_line_11` (and taxable
   income ↔ line 15) run over the named households AND over `btctax-input-form`'s coverage
   `maximal_fixture` (every `Vec` with a row, every `Option` `Some`, every census row answered — the
   fixture that by construction exercises every leaf; if the crate boundary makes that awkward,
   expose it through `testonly` or build an equivalent from `LEAF_SOURCE`'s money leaves with a
   guard that every money leaf is non-zero, and say which). Each household carries the anti-vacuity
   guard the AMT row has (the fixture must produce a non-zero term for the leg it exercises). Kill:
   remove the `+ hsa_income_8f` term again → the maximal-fixture row reds WITHOUT any HSA household
   being named; plant a new money leaf into the printed chain only (e.g. add a `Usd` to
   `Schedule1Parts` line 8z from a new `ReturnInputs` field, printed but not summed) → red.
4. **I-2 —** `ri.sch1.hsa_activity == Some(false)` in the code-W rule (a `None` is the registry
   loop's to block on the answering tiers). Kill: `screen_param_free` on `hsa_household()` with
   `hsa_activity = None` → no refusal; with `Some(false)` and a code-W W-2 → the contradiction.
5. **I-3 — the spouse's plan.** A class-(A) `FormQuestion` `HsaSpouseFamilyCoverage` on the HSA
   inputs, live iff a spouse exists (MFJ or MFS — `i8889--2024.txt:460-472`: *"regardless of whether
   you file jointly or separately"*), `#[serde(default)]`, classifier row, `LEAF_SOURCE`; the
   `HsaFamilyCoverage` prompt widened to the instruction's own sentences (lines 1 and 3, `:460-472`,
   `:496-499`); `compute` uses `coverage = filer_family || spouse_family` for line 1 and line 3.
   Kill: MFJ, taxpayer self-only, spouse family, $6,000 contributed → line 3 = the family limit,
   line 13 = $6,000, NO `HsaExcessContributionsNeedForm5329`; the same with both self-only → the
   self-only limit and the excess refusal.
6. **M-1 — the document-less distribution door** (R3's pattern): `hsa_distribution_without_1099sa`
   live iff `hsa_activity == Some(true)` and `documents.sa_1099 == Some(false)`; `Yes` refuses
   naming the trustee's Form 1099-SA (a trustee must issue one — cite `i1099sa`); `None` blocks.
   Kill: liveness exactly on that pair; the refusal's text.
7. **M-2** — `line-coverage` covers Form 8889 for BOTH archived editions (the 2025 line-13 text
   differs by a trailing period — measured by the reviewer); a drift in either extract reds.
   **N-1** — the line-9 census label names every box-12 slot the code reads (or the production
   names the slot set), and the doc says so.

## Constraints
- `record_answer` stays the only writer; no prompt beyond the instruction's words; the golden HSA
  household still reconciles on both oracles (re-run the harness; if OTS is unavailable say so).
- `line-coverage` / `census-join` / `box-census` counts move only where a production or line
  changed (list each).
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
APPEND a section `## Fold (seam review C-1, I-1, I-2, I-3, M-1, M-2, N-1)` to
`design/agent-reports/2026-09-07-build-interview-T16-implementation.md`: per item what changed, the
before/after table for the two probes (AGI, taxable income, total tax, Schedule 1 line 21), how the
maximal fixture reaches the equality instrument, every kill with its red text, every pinned number
moved, suite lines per crate. Return only a 4-line summary.
