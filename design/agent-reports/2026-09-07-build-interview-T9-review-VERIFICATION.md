# Ledger — controller machine-check of the T9 seam review

Every measurable claim in `2026-09-07-build-interview-T9-review.md` (persisted verbatim at `7ef1c36b`),
re-run **independently by the controller** before anything is folded. Report counts: `C=1 I=1 M=2 N=1`.

Baseline: `make check` at `18935bfd` — **3470 passed, 12 skipped, exit 0**. Worktree hygiene: the
reviewer's worktree carried no change but the report itself. The controller's own probe ran in a
throwaway detached worktree, since removed; `git worktree list` is back to one entry.

| # | Claim | Check | Result |
|---|---|---|---|
| **C-1** | A standard-deduction filer with a 1098 is asked and refused | probe re-run | **CONFIRMED** |
| **I-1** | Line 8b prints *"See attached"* for a statement nothing produces or names | source + map | **CONFIRMED** |
| **M-1** | The ceiling warning fires for a standard-deduction filer | source | **CONFIRMED** |
| **M-2** | The home-sale table crosses 10 of 24 combinations, not the described full cross | source | **CONFIRMED** |
| **N-1** | The ceiling warning does not use the module's money formatter | source | **CONFIRMED** |
| **D-1** | The build's 8b/8c geometry is right, the spec is wrong | re-measured (TY2024) | **CONFIRMED** |

---

## C-1 — CONFIRMED, and it is a genuine Critical

Controller's probe (temporary test, public API, worktree since removed): `ri()` with
`schedule_a: None` and one Form 1098 — the spec's own J-24 filer.

```
LEDGER schedule_a.is_none() = true
LEDGER form_1098.len() = 1
LEDGER live(ClaimingMortgageInterestCredit) = true
LEDGER reason(blank shared-interest) = Some("SharedMortgageInterestUnanswered")
LEDGER reason(shared-interest = Some(true)) = Some("SharedMortgageInterest")
```

`design/SPEC_interview.md:1164` (journey J-24) states the guarantee this violates, verbatim:

> | J-24 | takes the standard deduction and holds a $900k 2019 1098 | the census row and the three
> declarations are not live; nothing is transcribed and **nothing refuses** | — | R8 |

The premise that made the omission look safe is false. `questions.rs:2509-2511` says *"It does NOT
read `schedule_a.is_some()` on its own: the 1098 SECTION is already gated on the itemize election"* —
but `section_is_live` (`btctax-tui-edit/src/edit/form.rs:796-809`) handles `W2Box12`,
`ScheduleACharitable`, `Spouse`, `QbiLimitation` and `BrokerReporting`, and everything else falls into
`_ => true`. There is no `Form1098s` arm. The section is offered to every filer, which is what the
build's own §1 intends.

Reachable with no filer action: `grep -c schedule_a crates/btctax-cli/src/open_next_year.rs` returns
**0**, so the opener seeds a prior lender as an identity onto a year that carries no `schedule_a`. A
filer who itemized last year and takes the standard deduction this year lands in exactly that shape.

**The shipped test cannot red on it**, for the two compounding reasons the review gives — both
verified at `return_refuse.rs:3838-3856`: its `three` array enumerates only
`MortgageAllUsedToBuyBuildImprove`, `AmtQualifiedDwelling` and `MortgageWithinDebtLimit` (not
`ClaimingMortgageInterestCredit`), and its fixture literal sets `other_borrower_paid_interest:
Some(false)`, pre-answering the shared-interest gate. Its `assert_eq!(reason(&standard), None)` is
therefore true of a return in which both new gates were already answered — a shadow of the guarantee
it is named for. This is the same defect shape as T8's I-1 one task ago.

**Severity sustained at Critical.** It is the brief's one question answered in the affirmative, it
violates a written spec guarantee, and it is reachable through the shipped opener without the filer
doing anything.

## I-1 — CONFIRMED

`grep -c 'See attached\|SEE_ATTACHED'`: `schedule_a.rs` **4**, `cmd/admin.rs` (`hand_marks`) **0** —
so the packet manifest's *"COMPLETE BY HAND"* block, which frames itself as the closed list of marks
btctax deliberately did not make, does not name it. The trigger is as close as the review says:
`forms/2025/f1040sa.map.toml:134` gives `line8b_payee` exactly **one** entry
(`Line8b_ReadOrder[0].f1_16[0]`, the merged 24pt box), so **two** recipients overflow on TY2025.

Under this repo's own *an-entry-is-testimony* rule, the printed page asserts an attachment the filer
was never told to write. Sustained as Important.

## M-1 — CONFIRMED

`transcription_warnings.rs:346` is `if ri.form_1098.is_empty() { return None; }` — the same root
cause as C-1. Minor is right: the warning writes nothing and changes nothing.

## M-2 — CONFIRMED · N-1 — CONFIRMED

The home-sale loop crosses the three tests at `s_1099 = Some(false)` only, adding the other two states
on the blank branch alone. Benign, but the report and brief describe a full cross. N-1:
`transcription_warnings.rs:364` interpolates `${total}` / `${limit}` directly while `money()` exists
at `:312`, so the filer reads `$900000`.

## D-1 — CONFIRMED (the spec is wrong, the build is right)

The controller re-measured TY2024 with `xtask dump-fields` before the review was dispatched, and the
reviewer independently re-measured **both** years. Agreed: `f1_17[0]` (115.2, 348)–(396, 360) and
`f1_18[0]` (115.2, 336)–(396, 348) are both wide left-side cells in 8b's band — both are 8b's dotted
description lines — and `f1_20[0]` (417.6, 312)–(488.9, 324) is line 8c in the money column with no
description cell beside it. SPEC R8's cell pairing must be corrected.

## D-2 — the reviewer's verdict accepted

`other_borrower_paid_interest` refusing on `None` as well as `Some(true)` is correct fail-closed
behaviour **conditional on C-1's gate**: once the rule only fires on an itemizing return, the filer
being refused is one actually claiming line 8a, and a blank there is not a "no". Without C-1's gate it
is precisely the mechanism refusing filers who deduct nothing.

## What the controller did NOT independently re-run

The review's **"Seams checked clean"** negative claims and its `-p btctax-oracle-harness` /
`-p btctax-forms` baseline runs. The fold acts on none of them; the gate output above is the
controller's independent evidence of tree state.

## Disposition

**1C / 1I blocking.** Both fold, plus M-1, M-2 and N-1. The spec needs settling in the same pass — R8
currently says *both* that the document is top-level *and* that the section is live iff
`schedule_a.is_some()`, and the build followed the first. Fold brief:
`BRIEF-fold-interview-T9-review.md`.
