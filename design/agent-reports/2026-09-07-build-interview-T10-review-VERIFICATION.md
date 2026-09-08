# Ledger — controller machine-check of the T10 seam review

Every measurable claim in `2026-09-07-build-interview-T10-review.md` (persisted verbatim at
`586d97ce`), re-run **independently by the controller** before anything is folded. Report counts:
`C=0 I=3 M=4 N=1`.

Baseline: `make check` at `f8768e93` — **3491 passed, 12 skipped, exit 0**. Worktree hygiene: the
reviewer's worktree carried no change but the report itself. The controller's plants ran in a throwaway
detached worktree, since removed.

| # | Claim | Check | Result |
|---|---|---|---|
| **I-1** | The opener carries the phone and foreign address; `CARRIED_IDENTITY` names neither, and the guard is blind | source + **plant re-run** | **CONFIRMED** |
| **I-2** | Line 35c prints an account type the filer never chose | source | **CONFIRMED** |
| **I-3** | The ABA rule's written justification is false in all three claims | source | **CONFIRMED** |
| M-1…M-4, N-1 | see below | source | **accepted** |

---

## I-1 — CONFIRMED, and the guard's blindness reproduces on demand

Static chain at HEAD:

- `open_next_year.rs:411-434` — `seed` clones `phone`, `foreign_country`, `foreign_province` and
  `foreign_postal_code`.
- `CARRIED_IDENTITY` (`:288-319`) names `filing_status`, `filing_status_confirmed`,
  `header.taxpayer`, `header.spouse`, `header.address_`, the payer lists, the carryforwards and
  `opened_from` — **no `header.phone`, no `header.foreign_`**. The address phrase's prefix is
  `header.address_`, which matches none of the four.
- `return_inputs.rs:1224-1226` — `foreign_address_is_live()` reads **only** `foreign_country`. So a
  stale carried country alone makes the whole foreign block live and print.

Controller's plant — the four leaves added to the guard's own fixture, nothing else changed:

```
FAIL [0.150s] (1/1) btctax-cli::open_next_year_t4b every_leaf_the_seed_carries_is_named_in_the_report
panicked at crates/btctax-cli/tests/open_next_year_t4b.rs:1427:13:
the seed writes `header.phone`, and no phrase in the opener's report names it — a filer reading
"everything else is blank" would have no reason to look.
Summary [0.151s] 1 test run: 0 passed, 1 failed, 813 skipped
```

The guard is correct and well-written; it is green **only because its fixture never populates the
leaves it walks**. The filed-return harm is real: a filer who moves from a foreign address to the US
reads *"CONFIRM each: … your mailing address"*, corrects the domestic lines, is told nothing about the
foreign row, and `f1_15`/`f1_16`/`f1_17` print last year's foreign address on this year's domestic
return.

## I-2 — CONFIRMED

- `return_inputs.rs:1278-1283` — `DepositAccountKind` has exactly two variants, `Checking` and
  `Savings`. **No unanswered state.**
- `sections.rs:774-795` — `create` starts the type at `Checking`.
- `return_refuse.rs:2069-2091` — `screen_direct_deposit` validates `routing` and `account` only;
  nothing reads `kind`.

So a filer who creates the block, types the two numbers off their cheque and never opens the *"Account
type (line 35c)"* row files with the **Checking** box checked — testimony they never gave. The
instruction is explicit about the cost: *"You must check the correct box to ensure your deposit is
accepted."* A savings-account filer who leaves the default has the deposit rejected and the refund
delayed. The block-level classifier exemption is what hides it — the answered-ness machinery never
looks inside a class-(B) `Option` at a leaf with no unanswered state.

## I-3 — CONFIRMED

The ★★ paragraph of `RoutingNumber::canonical`'s doc is the entire argument for applying a validity
rule the IRS does not state, and its three factual claims are each false: the number **is** stored
(the seam's `set` writes the raw string unvalidated), the return does **not** file (`screen_direct_deposit`
is a value rule and `resolve.rs` fails closed on it), and `RefundByPaperCheck` **never fires** (advisories
are computed after the screen passes, and its guard is `direct_deposit.is_none()` anyway). The same
build states the truth 60 lines away.

Behaviour is fine — better than described. What is wrong is the record, at exactly the point a future
maintainer decides whether this class of rule may be extended. *"This costs nothing because a failure
just means a paper check"* is the sentence that would license btctax-only validity rules on other
cells, every one of which would in fact block a return. Important for that reason, not for a live
defect. **Note:** the controller repeated the same false claim in the T10 commit message (`f8768e93`),
taking it from the build report; the fold corrects the source and the build report, and this ledger is
the correction of record for the commit message, which cannot be rewritten.

## M-1…M-4, N-1 — accepted without independent re-run

M-1 (a year whose map has no `[direct_deposit]` drops the numbers silently, compensating advisory also
silent) is the one with teeth, and it conditions adjudication (1) below. M-2 (stale doc comment), M-3
(clearing the country leaves province and postal code at rest, and a later country resurrects them),
M-4 (secret handling — logged, never gating, per the owner ruling of 2026-08-27), N-1 (*"nothing on the
return reads it"* said of a field that prints). None gates.

## The three adjudications — the reviewer's verdicts accepted

1. **TY2025 getting no T10 cells is right** — the `[header]` refusal is the filler's first statement
   and `full_return_for(2025)` is `None` — **conditioned on M-1**, the silent-drop path that FR-84
   walks straight into. The fold must close M-1 or the conditioning fails.
2. **The ABA rule is right to apply** — false-refusal rate ≈ 0, and the filer is told loudly, just not
   the way the source says (which is I-3).
3. **The new `RefundByPaperCheck` exit is reachable** — `DIRECT_DEPOSIT` is in `form_spec()`, the TUI
   clamp moved 23 → 24, and the reviewer drove `create` + `set` to a printed block.

---

## ★ A pattern, now three tasks running — surfaced, not actioned

| task | defect | why the guard was green |
|---|---|---|
| **T8 I-1** | a stale (5)(b) printed under an unchecked (5)(a) | the test hand-set the leaf to `None`, so it tested `None ⇒ false`, never *not-demanded ⇒ blank* |
| **T9 C-1** | a standard-deduction filer asked and refused | the fixture **pre-answered** both new gates |
| **T10 I-1** | a foreign address crosses years unannounced | the fixture **never populates** the leaves the guard walks |

Each guard was well-designed and each was blind for the same structural reason: **a derived checker
was paired with a hand-written fixture, and the fixture decided what the checker could see.** `HARNESS.md`
B1 requires a checker be seen red on a planted defect — it does not require the fixture to exercise
every leaf the checker walks, which is the gap all three fell through.

Filed as **FR-88** with a proposed B1 amendment. **Not actioned** — amending the harness's doctrine is
the owner's call, and this ledger only records the measurement.

## Disposition

**0C / 3I blocking.** All three fold, plus M-1 (which adjudication 1 depends on), M-2, M-3 and N-1.
M-4 is logged, never gating. Fold brief: `BRIEF-fold-interview-T10-review.md`.
