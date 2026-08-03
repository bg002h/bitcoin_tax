# Review brief — §G-6 Form 6251 emitter (commit 942120e)

## The ONE question

**Can this commit put a WRONG NUMBER, or a WRONG BLANK, on a return signed under 26 USC §6065?**

Nothing else. Not style, not naming, not test coverage in the abstract, not "could this be
clearer." A finding is Critical/Important only if you can name the filer, the input, and the
figure that comes out wrong on a filed page.

## Scope — READ ONLY THESE

```
git show 942120e --stat
git show 942120e
```

Do **not** audit the rest of the repo. Do **not** re-derive the AMT rules from scratch. Do **not**
propose refactors.

## Facts ALREADY SETTLED — do not re-derive or re-litigate these

1. Who Must File condition 1 is `line 7 > line 10`, **not** `amt > 0`. This is correct and
   deliberate: when line 7 exceeds line 10 the AMTFTC is figured, so the form can be required while
   the AMT is $0.
2. Lines **2b, 2f, 2s** are parenthesised on the form and take a positive MAGNITUDE; the other
   "enter as a negative" lines take a literal minus. Two conventions, deliberate.
3. Part III (lines 12–40) is filed as a UNIT or not at all, gated on `part_iii_completed`. Writing
   29 zeros on a skipped page would be fabricated testimony. Deliberate.
4. Lines **2c–2t** are censused as `gap`, not modelled. Reachable only through the §G-22 out-of-scope
   refusal. This is a known, filed limitation (FOLLOWUPS §G-6) — **not** your finding.
5. Line 33 subtracts from line **22**; line 36 subtracts from line **12**. Both are pinned by a test.
6. Whole-dollar per-line rounding is SPEC §3.1's elected policy. Per-line rounding meaning the form
   may not foot to the cent is **expected**, not a defect.
7. `RefuseReason::AmtScreenTriggered` is now dead (declared, never constructed). Deliberately left
   for the Tier-2 rename. Not your finding.
8. TY2025 is out of scope for this branch; the TY2025 line-1 shape correctly refuses.

## Where the risk actually is — spend your budget here

* **`return_1040.rs`: `compute_6251` MOVED above the credits block, and `l18 = regular_tax +
  amt.line11`.** This is the only change to the tax computation itself. Does `compute_6251` still
  receive exactly the inputs it did before the move? Is anything it reads bound *after* its new
  position? Does adding `amt.line11` to `l18` double-count anything downstream —
  `tax_after_credits`, `total_tax`, `overpayment_refund`, `amount_owed`, the excess-SS path?
* **The `l18` term is UNCONDITIONAL** (`+ amt.line11` always), while the printed Schedule 2 line 2 is
  written only when `must_attach()`. The commit argues these agree because
  `line11 > 0 ⇒ line7 > line10` (since `line11 = line9 − line10` and `line9 = line7 − line8` with
  `line8 ≥ 0`). **Verify that implication against the actual field definitions in
  `crates/btctax-core/src/tax/form6251.rs`.** If `line8` can ever be negative, or `line11` is floored
  differently than assumed, the two chains diverge on a filed return.
* **`Schedule2Lines::line2` is now `Option<Usd>`; `line3` is plain `Usd`.** Is that asymmetry right?
  Line 3 is skipped by the emitter when `part_i` is false, but its *value* is still computed. Can a
  return exist where line 3 should print but line 2 should not, or vice versa?
* **`Form6251::printed()`** rounds every line independently. Does any consumer read a *rounded* line
  where it needs the unrounded one, or vice versa? Specifically: Schedule 2 line 2 is
  `round_dollar(f.line11)` and the emitter separately writes `printed().line11`. Same value?
* **The map's page-1 field assignment** (`f1_3` = L1, `f1_4..f1_23` = 2a..2t, `f1_24` = L3,
  `f1_25` = L4, `f1_26..f1_32` = L5..L11) and page-2 `f2_N` = line N+11. An off-by-one here puts a
  figure on the wrong line of a filed form. There is an inset-widget test that pins it; **say whether
  that test actually constrains the offset or merely happens to pass.**

## Output format

Markdown. For each finding:

```
### [CRITICAL|IMPORTANT|MINOR|NIT] <one-line claim>
**File:line:** ...
**Failure:** <concrete filer + inputs -> wrong figure on a filed page>
**Fix:** <one or two sentences>
```

Then a final line, exactly: `VERDICT: <n> Critical, <n> Important`.

If you find nothing at a severity, say so plainly. A clean result closes the loop — do not manufacture
findings to look thorough. Verify every claim against the source before writing it; a finding that
cites a line that does not say what you claim is worse than no finding.
