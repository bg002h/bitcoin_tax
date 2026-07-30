# Schedule 1-A — provenance census (13-agent workflow, 2026-07-29)

Ran after the user's reframing: **an entry is testimony; a blank is no testimony; most fields are blank by
design.** So the census asked, per label, *what decides this value, and if it is blank, WHY* — hunting the
difference between "blank because the inputs say so" (correct) and "blank because nothing populates it"
(the defect). 6 units classified, each adversarially verified, then a census re-deriving the closed set.

★ **HARNESS ARTIFACT — read before believing the raw output.** The script truncated each classification to
14 KB before handing it to its verifier, so several reviewers reported labels as *unverifiable* and the
aggregation recorded them as *omitted* (notably "10 of 12 Part II labels"). That is **my script's defect,
not a classifier omission**, and one reviewer correctly said so ("This may be transport truncation rather
than a classifier omission"). The findings below are only those I re-verified against the source myself.

## CONFIRMED — a whole input surface is missing, so two lines can never be anything but blank

**F-1. There is no 1099-NEC / 1099-MISC / 1099-K input surface at all.** `ReturnInputs` carries `w2s`,
`int_1099`, `div_1099`, `g_1099` — and nothing else (`return_inputs.rs:417-423`). But:

- **Line 5** reads *"Qualified tip amount included in Form 1099-NEC, box 1; Form 1099-MISC, box 3; or Form
  1099-K, box 1a."*
- **Line 14b** reads *"Qualified overtime compensation included in Form 1099-NEC, box 1, or Form
  1099-MISC, box 3."*

So both lines would be **permanently blank because nothing can populate them** — the exact class this
census existed to find, and indistinguishable on the page from a filer who simply had no such income.
Under §G-11 that is not a gap, it is **fabricated testimony**: a printed `0` swearing the amount is zero.
The three lawful moves are collect, refuse, or genuinely blank — and "silently zero" is none of them.

**F-2. The line-5 ceiling needs two deductions btctax models nowhere.** Plan r1 C-2 folded "net profit −
Schedule 1 line 15". The instructions require more: *"including the deductible part of self-employment
tax; the deduction for contributions to self-employed SEP, SIMPLE, and qualified plans; and the
self-employed health insurance deduction, but not including the deduction for qualified tips."* Printed
Schedule 1 Part II carries lines **15/18/21 only** (`printed.rs:384-387`) — no SEP/SIMPLE field, no
self-employed-health-insurance field. An implemented ceiling is therefore structurally **too high** ⇒ line
5 too large ⇒ **understates tax**. Worksheet column (c) is exactly this: *"Other deductions allocable to
the trade or business and not reported on Schedule C, Schedule E, or Schedule F."*

**F-3. Worksheet column (b) references Schedule E and Schedule F, neither of which btctax has.**

## CONFIRMED — the conformance approach I proposed has a hole

**F-4. The four worksheets appear ZERO times in the form extract** (`grep -c "Keep for Your Records"` on
`schedule_1a_2025_form.txt` = **0**). They exist only in the *instructions* extract. So a label census
driven off the form fixture — which is what T2 was going to do — **can never red on a worksheet
omission**. The census must draw its expected set from **both** fixtures, or the worksheets are invisible
to it and pass by being absent.

**F-5. The worksheet holds FOUR 1099 columns while the overflow prose says "more than three."** Columns
(e)/(f)/(g)/(h) are *first/second/third/fourth* (instructions extract lines 618, 630), but the narrative
says *"If you … received more than three Forms 1099-NEC, 1099-MISC, or 1099-K, then complete as many
copies of the worksheet as needed."* A transcriber trusting the prose drops column (h) and silently
truncates the fourth 1099 — smaller line 5, **overstates tax**. Pin the arity from the worksheet, not the
narrative.

**F-6. `schedule_1a_additional` is a hardcoded zero whose doc comment expires.**
`return_1040.rs:1269` — *"the 2024 form has no such line, so zero is the RIGHT value there, not a stub."*
True for TY2024, **false the moment TY2025 lands**, and a comment cannot red. This is G-11's shape exactly:
a correct blank and a laundered one, sharing a code path.

## Reported, NOT verified — treat as leads

- **WS3 has no column for line 14a's deferred-overtime add-on, yet WS3 line 2 overwrites line 14a**, so a
  multi-employer filer with deferred overtime would lose it (**overstates tax**). Plausible and specific;
  not re-checked.
- **WS4 has no comma after "column (b)" where WS3 does** — a verbatim trap for `cite-check`.
- **A FIFTH worksheet exists**: the instructions point to a *"No Tax on Car Loan Interest"* worksheet in
  the Form 1041 instructions. Genuinely `out_of_scope` (btctax emits a 1040, never a 1041) — but per the
  blank/blank distinction **the reason must be written down**, because "cannot apply" and "we never looked"
  are the two blanks this project exists to separate.

## What this changes for T2

1. The expected label set is drawn from **both** extracts (F-4), not the form alone.
2. Lines 5 and 14b cannot be transcribed as ordinary money leaves: with no 1099 surface they must
   **refuse** or be **genuinely blank**, never zero (F-1).
3. The line-5 ceiling is **not implementable** as specified until the two missing deductions exist —
   so it refuses rather than computing a too-high ceiling (F-2).
4. Worksheet arity comes from the worksheet (F-5).
