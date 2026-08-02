# Review brief — r6, `fc96703..HEAD` on `main`

**~2,700 new lines of source across 5 files.** Read the range yourself:
`git diff fc96703..HEAD -- 'crates/*/src'`.

## The ONE question

**Can any of this put a wrong mark — or remove a required one — on a filed federal return?**

★ The range splits sharply, and your budget should split with it:

| file | lines | what it is |
|---|---|---|
| **`btctax-forms/src/form1040_full.rs`** | **60** | **the ONLY change that alters a filed PDF** |
| `btctax-core/src/tax/line_coverage.rs` | 2145 | an instrument — a table of every printed money line |
| `xtask/src/line_coverage_check.rs` | 566 | the checker that validates that table |
| `xtask/src/main.rs`, `tax/mod.rs` | 15 | wiring |

Everything but the first is **inert on paper**: it records provenance and moves no figure (the TY2024
golden matrix md5 `c4e1853…` is unchanged across the whole range, which is the evidence). So a defect
in the instruments costs a *false sense of coverage*; a defect in `form1040_full.rs` costs a *wrong
return*. Weight accordingly.

## The change that moves ink — §G-24

1040 line 34 is *"**If** line 33 is more than line 24, subtract line 24 from line 33"* — a condition,
with **no `-0-` clause**. Line 37 is *"Subtract line 33 from line 24"* — no clamp, no condition.
`printed.rs` computes both with `.max(Usd::ZERO)`, so before this change **every owing return printed a
`0` on "amount you overpaid"** and every refund return printed a `0` on "amount you owe".

The fix gates lines 34, 35a and 37 at the **writer**, on the form's own comparison. Things to check:

- **Placement and geometry.** The gate removes writes from the middle of a group and I renumbered the
  ordinals (`p2_amount` went 15 → 13, then `+ ord` / `+ 2`). `verify.rs` asserts strictly-decreasing
  centre-y across a descent group. Does skipping cells leave that sound — and is the ordinal
  arithmetic right, or does it now collide or gap in a way that matters?
- **Is the gate correct for every case?** Both conditions false (payments == tax) leaves both blank —
  is that right? What about a return where `line33`/`line24` are themselves absent or defaulted?
- **35a rides on 34's condition.** 35a is *"Amount of line 34 you want refunded to you"*. Is binding
  it to line 34's condition correct, or does 35a have its own rule?
- **Anything that READS these cells back** — `transcribe::extract_lines`, `no_unmapped_filled`, the
  oracle harness, `render.rs` — now sees an absent key where it saw `0`. Does anything break or
  silently change meaning?
- **The crypto slice.** Does it emit a 1040 with these lines by a different path?

## The instruments — the question is whether they can be satisfied VACUOUSLY

`line_coverage` claims: every printed money field is accounted for, a new one cannot compile, every
quoted instruction is verbatim from the committed form text, and clamp polarity is transcribed. The
checker enforces twelve rules with two ratchets (11 exceptions, 0 unverifiable).

Worth probing:

- **`money_bearing_types`** is a hand-rolled Rust parser over source text. What does it MISS? Type
  aliases, `cfg`-gated types, tuple structs, generics, a `Usd` behind a type alias, a field written
  `pub x : Usd` with odd spacing?
- **`mentions_ident`** decides whether a type is "printed" by whether the emitter crate names it. Can
  that be fooled — a type used only via a re-export, an alias, or generics? Conversely, does it admit
  a type merely *mentioned* in a string literal or a `#[cfg]` block?
- **The compile-time guarantee.** Is it real? A new money field is claimed to be a compile error via
  no-`..` destructuring under `#![deny(unused_variables)]`. Find the evasion.
- **The 11 exceptions.** Each is a line that fits no production, with a written reason. Is any of them
  a nearest-fit dodge — i.e. should it have been a real production, and does calling it an Exception
  hide a wrong disposition?
- **`shipped_tables_are_the_validated_tables.rs`** binds the shipped tax tables to the validated ones.
  It found MFS/HoH ordinary brackets unwitnessed. Is the comparison complete — does it cover every
  field of `TaxTable` and `FullReturnParams`, or does it silently skip some?

## Settled — do NOT re-derive or relitigate

- **Whether §G-11 should be worked at all.** A seven-agent strategic review said "stop the program,
  keep the invariant"; the owner then clarified that seasons are a planned axis and no statutory clock
  competes, which reinstated it. That is decided. `design/direction/` has the record.
- The Schedule 1-A **plan** (r5) is a separate document track with its own review; not in scope.
- The TY2024 golden matrix md5, the five gates, and the 2548-test baseline are green at HEAD.
- §G-25 (MFS/HoH brackets unwitnessed) and the 11 exceptions are **filed**, with reasons. Re-arguing a
  filed item is not a finding; showing one is *wrongly* filed is.

## Output

`VERDICT: clean` or `VERDICT: <n> Critical / <n> Important`

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: concrete inputs → the wrong mark on a filed return, or the coverage claim that is false.
EVIDENCE: quote the code AND the form text or the rule it violates.
```

**Critical** = a wrong or missing mark on a filed return. **Important** = a real defect, or an
instrument that cannot detect what it claims to. Do not inflate — a short clean report is a fine
outcome for 60 lines of ink plus instruments that were built with planted-defect tests throughout.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY on tracked files; you may mutate temporarily to verify a kill **if** you
back up with `cp` to /tmp and restore with `cp` — never `git checkout --`. Leave the tree clean and say
so. No commits. No subagents.
