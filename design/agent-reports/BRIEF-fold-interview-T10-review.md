# Brief — fold the T10 seam review (0C / 3I / 4M / 1N)

You are folding an independent seam review into the **main working tree** of `/scratch/code/bitcoin_tax`,
at `HEAD` on branch `main`. You are the only agent editing this tree. **Commit nothing** — the controller
commits through the pre-commit gate when you return. Do not push, do not `git stash`, do not spawn
subagents, and never `git checkout -- <file>` over your own uncommitted work (keep a `cp` backup).

Read first, in this order:
1. `design/agent-reports/2026-09-07-build-interview-T10-review.md` — the review, verbatim.
2. `design/agent-reports/2026-09-07-build-interview-T10-review-VERIFICATION.md` — the controller's ledger.
   **Every finding is machine-confirmed.** Do not re-litigate whether one is real. Your job is the fix
   and its kill.
3. `design/agent-reports/2026-09-07-build-interview-T10-implementation.md` — the build you are correcting.
4. `CLAUDE.md` at the repo root.

**A fold is authorship and re-earns the build gate.** Machine-check everything machine-checkable before
returning.

## Validation

Main tree ⇒ leave `CARGO_TARGET_DIR` unset; `CARGO_TARGET_DIR=target-clippy` for clippy. Scoped runs
while you work; `make check` once at the end (~19 s, 3491 tests at HEAD). Run all five instruments and
quote them: at HEAD `line-coverage` 375/18/31 (ratchet 31)/0/17; `census-join` 274 across 13 maps;
`stop-list` 8 + 4 sources, 91 prompts; `prompt-check` 88; `box-census` 268/19/9.

## I-1 (fold first) — the opener carries the phone and foreign address, and says it carries neither

Confirmed. `seed` clones `header.phone`, `header.foreign_country`, `header.foreign_province`,
`header.foreign_postal_code` (`open_next_year.rs:411-434`); `CARRIED_IDENTITY` (`:288-319`) names none
of them — its address phrase's prefix is `header.address_`. `Opened::render()` therefore prints
*"Everything else is blank and every question is unanswered"* while four leaves cross.

The harm is on the printed page, not just in the wording: `foreign_address_is_live()`
(`return_inputs.rs:1224-1226`) reads **only** `foreign_country`, so a filer who moves from abroad to the
US opens the year, corrects the domestic address lines they were told to confirm, and
`f1_15`/`f1_16`/`f1_17` print **last year's foreign address on a domestic return**.

**Fix.** Add the two phrases to `CARRIED_IDENTITY` and the mirror list in the test's `NAMED`:
`("your phone number", &["header.phone"], false)` and
`("your foreign address, if you have one", &["header.foreign_"], false)`. The second is the one that
changes what prints.

**Then fix the guard, which matters more than the two phrases.** Controller's plant:

```
FAIL every_leaf_the_seed_carries_is_named_in_the_report
panicked at open_next_year_t4b.rs:1427:13:
the seed writes `header.phone`, and no phrase in the opener's report names it …
```

It is green only because `vault_with_year_n` / `a_year_with_money_in_it` never populate the leaves it
walks. Extend the fixture so every leaf `seed` carries is populated — and prefer a shape where that is
**derived** rather than remembered: if the fixture can be built from the leaf set the guard walks (or
asserted to cover it), a future carried leaf cannot be invisible the same way. If you cannot derive it,
say so plainly and add the assertion that the fixture covers the set.

★ This is the third consecutive task whose defect was a well-built guard blinded by a hand-written
fixture (T8 I-1, T9 C-1, T10 I-1 — the ledger tabulates them). Fix this one so the *class* is closed
here, not just the instance.

## I-2 — line 35c prints an account type the filer never chose

Confirmed: `DepositAccountKind` has exactly `Checking` and `Savings` and **no unanswered state**
(`return_inputs.rs:1278-1283`); `create` starts it at `Checking` (`sections.rs:774-795`);
`screen_direct_deposit` (`return_refuse.rs:2069-2091`) validates the routing and account strings only
and never reads `kind`. So a filer who types the two numbers off their cheque and never opens the
account-type row files with **Checking** checked — testimony they never gave. The instruction:
*"You must check the correct box to ensure your deposit is accepted."* A savings filer who leaves the
default has the deposit **rejected**.

**Fix** (the reviewer's, and it matches this repo's answered-ness doctrine): make the stored field
`Option<DepositAccountKind>` — no `#[serde(default)]`, per §4.3 — have `create` leave it `None`, and add
a `DirectDepositCell::Kind` leg to `screen_direct_deposit` so an unconfirmed type refuses exactly as an
empty routing number does. `push_direct_deposit`'s exhaustive `match` then cannot print a box nobody
chose. `attribute.rs` already has the anchor shape for `FieldId::DdKind`.

Also delete the seam comment's first half — *"the starting position of a two-way control the filer must
confirm"* asserts a confirmation step that does not exist.

**Kill.** Drive the real `create` + the two `set`s, leave the type untouched, and assert the screen
**refuses**; then answer the type and assert the correct box prints. Plant the default back → red.

## I-3 — the ABA rule's written justification is false in all three of its claims

Confirmed. The ★★ paragraph of `RoutingNumber::canonical`'s doc (`packet.rs:268-276`) says a bad number
*"is not stored"*, the return *"files with NO deposit block"*, and *"Nothing is blocked"*. In fact the
seam's `set` stores the raw string unvalidated (`sections.rs:705-713`), `screen_direct_deposit` is a
value rule and `resolve.rs:95-102` fails closed on it, and `RefundByPaperCheck` never fires (advisories
run only after the screen passes, and its guard is `direct_deposit.is_none()`).

**The behaviour is right — refusing loudly is better than the degradation the doc describes.** Fix the
*record*: rewrite the paragraph to say what actually happens (a malformed number REFUSES, naming the
cell, the rule and both remedies), and make the argument for keeping a btctax-only validity rule on
those true terms. Correct the same claim in the build report's §3 D-3.

★ Say explicitly in the new wording that this rule blocks a return, so nobody reads it as licence to add
btctax-only validity rules to other cells "because a failure just means a paper check" — that inference
is exactly what the false version enabled.

## M-1 (fold — adjudication 1 depends on it) and the rest

- **M-1** — a year whose map has no `[direct_deposit]` drops the filer's numbers **in silence**, with the
  compensating advisory also silent. The reviewer's verdict that TY2025 correctly gets no T10 cells is
  **conditioned on this being closed**, so it is not optional: make the drop loud (a refusal, a hand-mark,
  or an advisory that actually fires — decide which and say why). FR-84 walks straight into this path.
- **M-2** `Advisory::RefundByPaperCheck`'s doc comment is stale.
- **M-3** clearing the foreign country leaves the province and postal code at rest, so a later country
  resurrects them. Clear the block as a unit, or refuse the partial state.
- **N-1** *"nothing on the return reads it"* is said of a field that prints on the return.
- **M-4 is secret handling** — per the standing owner ruling it is **never** Critical or Important. Fix it
  if it is cheap, otherwise file it. Do not let it hold the gate.

## Standing lessons that bind this fold

- **A guard is only as good as the fixture that feeds it** — three tasks running (see the ledger).
- **A default that reaches the page is testimony** — that is I-2, and the repo's own doctrine.
- **A written justification is load-bearing**: it licenses the next change (I-3).
- **Never hand-count what a tool can count.**

## Report — your FINAL action

Write `design/agent-reports/2026-09-07-build-interview-T10-fold.md`: Commands with real output; one
section per finding (What changed / Where / The kill and its observed red / Anything decided differently
and why); a **Deviations** section; the five instrument outputs; the closing `make check` summary line.
Then return ONLY a 3-line summary plus the path.
