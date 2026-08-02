# Filing trial — what stops a return, and what comes out wrong

**Date:** 2026-08-02 · **Binary:** `btctax` @ `cd03a37` (v0.15.0, unpublished) · **Year: TY2024**
(the only year the full return supports — `report --tax-year 2025` fail-closes with a clear message).

★ Filed here rather than in a new directory because `BRIEF-strategic.md` asked exactly this question —
*"is this converging on a shippable filing tool, or is it accreting rigour around a core that still
cannot take an ordinary person from CSV to signature?"* — and answered it from **reading** the code.
This is the same question answered by **driving the shipped CLI end to end**. Working vaults are in the
session scratchpad; nothing here touches the repo.

---

## Scenario A — the owner's scenario

> $85,000 earned income self-employed · $85,000 donation to church · $2M capital gains · 9 dependent
> children · $375,000 medical expenses · $10,000 student loan interest · $2,500 car loan interest

**Assumptions I had to make** (the scenario does not say): filing status **MFJ**; the capital gain is
**long-term**; the 9 children are all CTC-qualifying ages; no wages, no withholding, no estimated
payments.

**Result: the return cannot be filed as specified.** Two hard stops, either of which alone blocks it.
With both worked around it computes and emits a correct packet — see "what it got right".

---

## BLOCKERS — these stop a return

### B1 · `NOT COMPUTABLE [QbiAboveThreshold]` — self-employment + high income refuses

```
NOT COMPUTABLE [QbiAboveThreshold]: taxable income before the QBI deduction is above the
§199A(e)(2) threshold — the Form 8995-A phase-in (SSTB / wage-and-UBIA limits) is out of scope for v1
```

Documented in `btctax limitations` and deliberately fail-closed. **But the refusal is caused by a
missing INPUT, not by an uncomputable quantity.** `QbiInputs` (`return_inputs.rs:430`) carries only the
two loss carryforwards — there is no field for **W-2 wages paid by the business** or **UBIA of
qualified property**. For a sole proprietor with neither, §199A(b)(2) caps the deduction at
`max(50% × 0, 25% × 0 + 2.5% × 0)` = **$0**, which is knowable without Form 8995-A's phase-in.

Collecting those two numbers would turn a refusal into a computation for the whole "self-employed, no
employees, above the threshold" population. `CLAUDE.md`'s own corollary applies: *"If the form asks
something our input surface cannot answer, collect it."*

**Blocks any return with a Schedule C and taxable income over $383,900 MFJ / $191,950 otherwise.**

### B2 · 9 dependents exceed the 1040's 4-row grid — the PDF refuses

```
error: IRS form fill: 9 rows exceed the 4-row capacity of a single the 1040 dependents table page
```

Documented, and the intent is right — *"It will not silently print a subset of your payers or your
children."* But **Form 1040 has a checkbox for exactly this case**: *"If more than four dependents,
see instructions and check here ▶"*. The IRS remedy is four rows + that box + a continuation
statement. btctax fills neither.

Note the asymmetry: `report --tax-year 2024` **computes the whole return with 9 dependents without
complaint**; only the emitter refuses, so the filer discovers this at the last step.

★ The message also has a grammar bug: *"a single **the** 1040 dependents table page"*.

**Blocks any family with 5+ dependents.**

### B3 · Self-employment income cannot be entered — only mined

`ScheduleCInputs` (`return_inputs.rs:252`) has `expenses` but **no gross-receipts field**. Schedule C
revenue comes exclusively from the Bitcoin ledger (income events reclassified with
`reconcile reclassify-income --business true`). A filer with $85,000 of consulting, freelance or any
other non-Bitcoin self-employment income **cannot represent it at all**.

**Blocks every self-employed filer whose business is not Bitcoin.** For this trial I modelled the
$85,000 as mining, which is in scope — but that is my substitution, not the scenario.

### B4 · Capital gains cannot be entered — only disposed

No 1099-B input exists anywhere in `ReturnInputs`. The only capital-gain paths are the BTC ledger and
1099-DIV box 2a capital-gain *distributions* (Schedule D line 13). **$2M of stock gain is
inexpressible.** Same modelling substitution applied.

### B5 · An income amount cannot be stated in dollars

Ledger income is valued at the bundled daily-close FMV, so producing exactly $85,000 meant solving for
a satoshi quantity and iterating:

```
1.00000000 BTC @ 2024-11-01  ->  $69,715.15
1.21924511 BTC               ->  $84,999.86
1.21924712 BTC               ->  $85,000.00   (third attempt)
```

Not a defect in the tax logic, but it makes reproducing any published test case laborious, and it is
why B3/B4 bite so hard.

### B6 · An income lot can be silently consumed by an earlier-dated sale — **NOT A DEFECT**

★ On re-examination this is **correct behaviour**, not a blocker. Selling coins you mined is ordinary,
HIFO legitimately selected the highest-basis lot, and the report printed both disposal legs plainly.
What went wrong was my reading, not the software. Kept on the list only as a **usability** note:

My first attempt dated the mining receipt June and the sale September. HIFO selected the *income* lot
(highest basis) for the disposal, splitting it into a short-term leg and changing both the gain
character and the income position. Correct lot-selection behaviour — but nothing warned that the
receipt just added had been consumed, and the reported figures moved without explanation.

### B7 · TY2025 is not supported, so the car loan interest has nowhere to go

`$2,500` of car loan interest is an **OBBBA TY2025** item (the new Schedule 1-A). TY2024 has no such
deduction, so **no field is the CORRECT answer for the year I could file** — but the scenario is
therefore unfileable as stated in any supported year. TY2025 fail-closes cleanly:

> `tax year 2025 has full-return inputs, but full-return computation is not supported for 2025 in this
> version (v1 supports TY2024)`

### ~~B8 · `income answer` is interactive-only~~ — **RETRACTED, I was wrong**

**This is not a blocker.** `income answer` reads an ordinary stdin stream, so it scripts fine:

```
$ printf 'n\nn\nn\nn\nn\nn\nn\nn\ny\n\n\n\n\n\n' | btctax income answer --year 2024
Answered the full-return questions for tax year 2024.
$ btctax income show --year 2024 | jq '.header.taxpayer.blind, .schedule_a.salt_use_sales_tax'
false
true
```

I filed it after reading `--help` ("answer … **interactively**", "the only way to answer them without
editing a TOML file") and never ran it with a pipe. That is `CLAUDE.md`'s own recorded failure —
*"I conclude from not having looked"* — reproduced in the act of auditing.

What survives is smaller and real, and stays on the list as a **Minor**:

- **The question set is DYNAMIC** — answering the blindness questions reveals two death-of-taxpayer
  questions, and answering those reveals the Form 8283 restrictions question. A script cannot know the
  line count in advance.
- **Running out of input discards EVERYTHING**: `input ended before every question was answered —
  nothing was stored`. Correct (a partial answer set is not testimony), but combined with the dynamic
  count it means a scripted caller must over-supply blank lines and re-check.
- The `--help` text says "interactively" and names TOML editing as "the only" alternative, which is
  what misled me.

### B9 · `--forms full-return` is rejected with a misleading error — ✅ **FIXED**

```
error: invalid value 'full-return' for '--forms <FORMS>'
  [possible values: f8949, schedule-d, schedule-se, form8283, form1040, form8275]
```

The correct invocation was to **omit `--forms` entirely** — a year with `income import` dispatches to
the full packet and `--forms` is ignored. The value list did not say so, and the obvious guess failed.

**Fixed:** `full-return` is now a `FormArg` variant. On a full-return year it is *honored*, not reported
as an ignored slice. ★ On a crypto-only year it **refuses**, because `wants()` is
`selected.is_empty() || selected.contains(f)` — so the new variant matches no crypto-slice form and
would otherwise have written an **empty export directory and exited 0**, which a filer would reasonably
read as *"there was nothing to file"*. Adding the variant introduced that failure mode; the guard and
its planted-defect test land with it. Both legs mutation-verified.

---

## INCORRECT RESULTS

### R1 · The CTC advisory ignores the §24(b) phase-out — it advises claiming a credit that is $0

```
• CTC/ODC NOT COMPUTED — you captured 9 dependent(s) ... Your tax is OVERSTATED by up to
  $2,000 per qualifying child / $500 per other dependent. File Schedule 8812 yourself to claim it.
```

At this AGI the credit is **fully phased out**:

| | |
|---|---|
| CTC before phase-out | 9 × $2,000 = $18,000 |
| §24(b) reduction at AGI $2,085,000 (MFJ, $400,000 threshold) | $84,250 |
| **Correct CTC** | **$0** |

**1040 line 19 = $0 is correct.** The *advice* is not: it tells a filer their tax is overstated by up
to $18,000 and sends them to Schedule 8812 for nothing. The advisory is unconditional — it never
applies §24(b) — so it misfires on exactly the high-income returns btctax is otherwise careful about.

★ This is the only thing in the trial that would actively mislead a filer, and **it changes no number**,
which is why no oracle and no golden test would ever catch it.

---

## What it got RIGHT — verified by hand, line by line

With B1 and B2 worked around (mining income left non-business; 4 dependents), the packet emits and
every figure checks out:

| 1040 line | btctax | independent check |
|---|---|---|
| 9 total income | 2,085,000 | 2,000,000 LTCG + 85,000 mining |
| 10 adjustments | 0 | §221 student-loan interest **fully phased out** above $195,000 MAGI — the $10,000 correctly yields $0, not the $2,500 cap |
| 11 AGI | 2,085,000 | |
| 12 itemized | 303,625 | medical 375,000 − 7.5% × AGI (156,375) = 218,625, **+** 85,000 church (the 60%-AGI limit is 1,251,000, not binding) |
| 15 taxable income | 1,781,375 | |
| 16 tax | 312,980 | all LTCG: 0% ≤ 94,050; 15% to 583,750; 20% above → 73,455 + 239,525 |
| 8960 NIIT | 69,730 | 3.8% × min(NII 2,000,000, MAGI − 250,000 = 1,835,000) |
| 6251 AMT | 0 | exemption fully phased out; medical and charity are both allowed AMT deductions and SALT is 0 → AMTI = taxable income → TMT = regular tax |
| 24 total tax | 382,710 | |
| 37 amount owed | 382,710 | |

Schedule SE, computed in the blocked variant, is also right: `85,000 × 92.35% = 78,497.50`;
SS `× 12.4% = 9,733.69`; Medicare `× 2.9% = 2,276.43`; total **12,010.12**; §164(f) half **6,005.06**;
Additional Medicare **0** (SE earnings below the $250,000 MFJ threshold).

★ **The §G-24 fix is visibly working on a real return.** The emitted 1040 carries `382710` on line 37
and lines 34/35a are **absent** — not a printed `0`. Fixed earlier today, now confirmed on paper rather
than in a test.

★ **Tri-state answered-ness holds.** Leaving `blind` and `salt_use_sales_tax` unset produced advisories
naming the forgone benefit, not a fabricated "No".

Packet emitted: `00_f1040`, `01_f1040s1`, `02_f1040s2`, `07_f1040sa`, `12_schedule_d`, `12A_f8949`,
`72_f8960`, plus a staple-order manifest. No Schedule B (no interest/dividends) and no Form 6251
(AMT = 0, not required) — both correct omissions.

---

## Scenarios B–D — published gold standards

*Pending: a parallel search for published scenarios that ship a complete answer key. Results appended
here.*
