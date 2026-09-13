# REPORT — FR-197: the W-2 box 12 code table, derived instead of typed

**Agent:** opus, own worktree, `CARGO_TARGET_DIR=<worktree>/target-box12`.
**Files changed:** `crates/btctax-core/src/tax/return_refuse.rs` **only** (971 insertions, 30 deletions).
**Gate:** `make gate` (forced rebuild + nextest + clippy `-D warnings`, concurrently) → **exit 0**,
`3654 tests run: 3654 passed, 12 skipped`. `cargo fmt --all -- --check` → exit 0.
**Not committed, not pushed.** No subagents. Everything foreground.

---

## 1. What was wrong, and what replaced it

`INERT_BOX12_CODES` was **11 codes typed by hand** — `D E F G H S AA BB EE DD W` — beside a set of
**33** that the IRS prints. Anything else refused `UnsupportedBox12Code` and the packet printed zero
pages.

It is now a **33-row transcription of the IRS's own table**, `BOX12_CODES`, with one verdict per code
and the authority for that verdict written above the row. Measured off the source, not counted by
hand:

```
rows        : 33
ADMIT  (16) : C D E F G J S V W Y AA BB DD EE GG HH
REFUSE (17) : A B H K L M N P Q R T Z FF II TA TP TT
newly admitted : C J V Y GG HH
newly refused  : H
```

Verdict distribution (`grep -c` on the row initialisers): `AlreadyInBox1` 6, `NoLineReadsIt` 4,
`ReadByBtctax` 6 — 16 admitted; `DrivesAnUncomputedLine` 16, `NotAdjudicated` 1 — 17 refused.

### The six newly admitted, each with the sentence that admits it

| code | why admitting changes no figure | authority |
|---|---|---|
| **C** group-term life > $50k | *"Also include this amount in boxes 1, 3 (up to the social security wage base), and 5."* | `iw2w3--2026.txt:2428-2434` |
| **V** NSO exercise spread | *"Include this amount in boxes 1, 3 (up to the social security wage base), and 5."* | `iw2w3--2026.txt:2623-2633` |
| **GG** §83(i) equity grants | *"This amount is wages for box 1 …"* | `iw2w3--2026.txt:1516-1520` |
| **J** nontaxable sick pay | *"… not includible in income (and not shown in boxes 1, 3, and 5) …"*; `"sick pay"` occurs **0** times in `i1040gi--2025.txt` | W-2 instructions + measured absence |
| **Y** §409A deferrals | *"It is not necessary to show deferrals in box 12 with code Y."* — a deferral, not income; the 1040 names code **Z** for §409A and never Y | W-2 instructions + `i1040gi` grep |
| **HH** aggregate §83(i) deferrals | a running total of income *deferred*, not income of the year; `"83(i)"` occurs **0** times in `i1040gi--2025.txt` | measured absence |

### ★ The one row that NARROWS the old list — code H

This is the finding, and it is a **behaviour change that stops a return that used to print**. Code H
was in the "inert" list and is not inert. Its own instruction says so:

> *"Be sure to include this amount in box 1 as wages. **The employee will deduct the amount on their
> Form 1040 or 1040-SR.**"* (`iw2w3--2026.txt:2535-2538`)

That deduction is **Schedule 1 line 24f** — *"Enter contributions to section 501(c)(18)(D) pension
plans"* (`i1040gi--2025.txt:43215-43217`) — which btctax censuses `unmodeled`
(`crates/btctax-forms/forms/2024/f1040s1.map.toml:173`). So the amount sits in box 1, btctax files it
as wages on line 1a, and the deduction that is supposed to come back out never does: **the return
overstated the filer's own tax by the whole entry, silently.** Refusing over a forgone deduction is
what this same screen already does one rule up, where a statutory-employee W-2 refuses because filing
it would *"forgo the Schedule C deductions"*.

### Why each refusal refuses (all 17 named, with the line)

* **Additional tax → understatement if admitted.** `A`, `B`, `M`, `N` → Schedule 2 line 13
  (*"This tax should be shown in box 12 of Form W-2 with codes A and B or M and N."*,
  `i1040gi--2025.txt:44983-44987`); `K` → Schedule 2 line 17k; `Z` → Schedule 2 line 17h.
* **Income / credit the 1040 asks for.** `T` → Form 8839 line 31 → **1040 line 1f**; `Q` → the EIC
  election on **1040 line 1i**; `II` → **Schedule 1 line 8s** plus the same EIC election.
* **A deduction btctax cannot compute, so the figure would be wrong in the filer's direction.**
  `H` → Sch 1 line 24f; `L` → Form 2106 → Sch 1 line 12; `P` → Form 3903 → Sch 1 line 14;
  `R` → Form 8853 → Sch 1 line 23 (btctax already refuses Archer MSA activity outright).
* **A disqualification btctax cannot apply.** `FF` → *"A qualified small employer health
  reimbursement arrangement (QSEHRA) is considered to be a subsidized health plan maintained by an
  employer"* (`i1040gi--2025.txt:42683-42690`) — which kills the SEHI deduction for the months it
  covers and bears on the PTC.
* **Collected elsewhere and not reconciled.** `TP` → Schedule 1-A Part II (§224 qualified tips),
  `TT` → Part III (§225 overtime). btctax asks the filer for the *qualified subset* directly
  (`Schedule1aTips::qualified_tips_reported`), nothing reconciles the two figures, and **no question
  in `questions.rs` asks about tips or overtime at all** — so a return holding a TP while Part II is
  empty is a return whose deduction nothing claimed.
* **Not adjudicated.** `TA` (§128 Trump account employer contributions, first printed on the 2026
  W-2): excluded from gross income up to $2,500 against a $5,000 account limit, and btctax has
  adjudicated neither the limit, nor an excess, nor how it meets the new Form 4547.

★ **No exit anywhere says "delete the row."** The W-2 is the employer's testimony and the filer's
transcription of it; telling someone to drop a line off their own W-2 so the software prints is
telling them to file something they were not sent. The lawful exits are: file with a preparer,
complete the form btctax does model, or — for a code that is not a code — re-read the paper.

---

## 2. Derived, not extended — and the derivation is a test

`design/forms/extract/iw2w3--{2024,2025,2026}.txt` are all archived and in `MANIFEST.json`.
`tests::the_box12_table_is_the_irs_table` re-derives the table from **every** archived revision on
every run, out of **two independent regions of each document**:

1. the *Form W-2 Reference Guide for Box 12 Codes* table → `(code, label)` pairs (the guide is three
   columns; `pdftotext -layout` emits each cell as a code line, a blank, then wrapped description
   lines);
2. the per-code narrative `Code XX—…` headings → the code set.

The two must **agree per revision** before either is believed, and the parse must yield ≥ 30 codes or
the checker reports itself broken — because an empty set is a subset of everything and would pass
every assertion below it on nothing.

Measured code sets: **2024 → 30, 2025 → 30, 2026 → 33** (2026 adds `TA`, `TP`, `TT`); the two regions
agree exactly in all three. The list of years is **read out of the extract directory**, so archiving
`iw2w3--2027.txt` pulls a new revision into every assertion with no edit.

**Labels** are pinned to the *latest* archived revision and verified byte-for-byte after whitespace
collapsing. Four codes are worded differently between the three revisions — measured: `F` and `S`
changed in 2025, `P` and `W` in 2026, and **`W` differs by an apostrophe alone** (ASCII `'` → `’`),
which is exactly why the label check names one revision and the code-set check spans all of them. The
table carries the curly form, and the test is what proves it.

The four things the table does **not** cover are stated in the source rather than implied: the
verdicts are judgment; labels are one revision's wording; only archived revisions are covered; and
the three 2026-only codes are all refused, so the table needs no year axis yet (the first one admitted
gives it one).

**Compiler-held half:** `Box12Verdict::refusal()` is an `_`-free match over all five verdicts, so a
sixth verdict is a compile error rather than a code that silently starts admitting. And
`ELECTIVE_DEFERRAL_CODES` — the second hand-typed code list in this file, three lines below the first
— is now cross-checked by `every_elective_deferral_code_is_an_admitted_box12_code`: a §402(g) code
that starts refusing would never reach the cap sum, and that now reds.

---

## 3. B1 — planted defects, observed red

### (a) The instrument: a stale table

`a_table_that_has_fallen_behind_the_irs_is_caught` asserts the real table is clean **first** (or a
checker that reds on everything would pass), then plants three defects into a copy: code `C` deleted,
an invented code `QQ` added, and code `C`'s label changed by one digit ($50,000 → $5,000). Each must
come back named.

Beyond that self-contained kill, I planted the defect in the **real** table — deleted the code `V`
row — and watched both instruments red, then restored from a `cp` backup (never a VCS `checkout --`;
the tree had uncommitted work) and confirmed byte-identity:

```
thread 'tax::return_refuse::tests::the_box12_table_is_the_irs_table' panicked at
crates/btctax-core/src/tax/return_refuse.rs:7225:9:
the box 12 table has fallen behind the IRS:
box 12 code V is printed in the Form W-2 instructions and this table does not classify it — an
unclassified code refuses the whole return, silently, which is exactly what FR-197 found
test tax::return_refuse::tests::the_box12_table_is_the_irs_table ... FAILED
```

```
thread '…::an_inert_box12_code_files_and_a_consequential_one_refuses_by_name' panicked at
crates/btctax-core/src/tax/return_refuse.rs:7331:13:
assertion `left == right` failed: ★ box 12 code V changes no figure on the return and must file
  left: Some(UnsupportedBox12Code("V"))
 right: None
```

`RESTORED: byte-identical to the pre-plant file`. Because that plant→restore loop is exactly what
races cargo's mtime freshness check, every number reported here comes from **`make gate`** (which
`touch`es every `.rs` first), per the Makefile's own rule.

### (b) The behaviour: an inert code files, a consequential one refuses by name

Captured from a temporary `--nocapture` test (since removed; the permanent assertions live in
`an_inert_box12_code_files_and_a_consequential_one_refuses_by_name`, which checks that each refusal
names the code, the line it drives, the employer and an exit):

```
ADMIT  C   -> screen says None
ADMIT  V   -> screen says None
ADMIT  GG  -> screen says None
ADMIT  J   -> screen says None
ADMIT  Y   -> screen says None
ADMIT  HH  -> screen says None

REFUSE A   -> UnsupportedBox12Code("A")
  the Form W-2 from ACME reports box 12 code A — "Uncollected social security or RRTA tax on tips"
  — which is social security tax your employer could not collect out of your pay, and Schedule 2
  line 13 is where the 1040 collects it instead — btctax does not compute that line, so filing this
  return would understate the tax you owe. File this return with a preparer.

REFUSE K   -> UnsupportedBox12Code("K")
  the Form W-2 from ACME reports box 12 code K — "20% excise tax on excess golden parachute
  payments" — which is a 20% excise tax that belongs on Schedule 2 line 17k ("This tax should be
  shown in box 12 of Form W-2 with code K") — btctax does not compute that line, so filing this
  return would understate the tax you owe. File this return with a preparer.

REFUSE H   -> UnsupportedBox12Code("H")
  the Form W-2 from ACME reports box 12 code H — "Elective deferrals to a section 501(c)(18)(D)
  tax-exempt organization plan" — which is in your box 1 wages AND deductible back out on Schedule 1
  line 24f ("contributions to section 501(c)(18)(D) pension plans") — btctax files the wages and does
  not compute line 24f, so this return would overstate your tax by the whole entry. File this return
  with a preparer.

REFUSE Z   -> UnsupportedBox12Code("Z")
  the Form W-2 from ACME reports box 12 code Z — "Income under a nonqualified deferred compensation
  plan that fails to satisfy section 409A" — which is §409A-failed deferred compensation: it is in
  box 1 AND it carries an additional 20% tax plus interest on Schedule 2 line 17h ("This income
  should be shown in box 12 of Form W-2 with code Z, or in box 15 of Form 1099-MISC") — btctax does
  not compute that line, so filing this return would understate the tax you owe. File this return
  with a preparer.

REFUSE TP  -> UnsupportedBox12Code("TP")
  the Form W-2 from ACME reports box 12 code TP — "Total amount of cash tips reported to the
  employer" — which is the total cash tips you reported to your employer, and the §224 deduction on
  Schedule 1-A Part II is figured from the QUALIFIED subset of them — which btctax asks you for
  directly, because the qualifying occupation and the qualifying tips are yours to state and not your
  employer's. btctax does not reconcile the two figures, and a return holding this one while Part II
  is empty is a return whose deduction nothing claimed. Complete Schedule 1-A Part II if those tips
  qualify, and file with a preparer.

REFUSE TA  -> UnsupportedBox12Code("TA")
  the Form W-2 from ACME reports box 12 code TA — "Employer contributions under a section 128 Trump
  account contribution program paid to a Trump account of an employee or a dependent of an employee"
  — which is a §128 Trump account employer contribution — a code the IRS first prints on the 2026
  Form W-2. It is excluded from the employee's gross income up to $2,500 a year against a $5,000
  account limit, and btctax has adjudicated neither that limit, nor what a contribution over it does,
  nor how it meets Form 4547. File this return with a preparer.

REFUSE QQ  -> UnsupportedBox12Code("QQ")
  W-2 box 12 code QQ is not a code the IRS prints in the Form W-2 instructions (they run A through
  II, plus TA, TP and TT from 2026). btctax will not guess which code was meant. Re-read the paper:
  the code is the capital letter or two to the LEFT of the vertical line in box 12a–12d, and the
  money is to the right.

REFUSE     -> UnsupportedBox12Code("")
  a Form W-2 carries a box 12 row with an amount but NO code. Box 12 is a code and an amount
  together — "Even if only one item is entered, you must use the IRS code designated for that item"
  — and without the code there is no way to know what the money is. Type the code exactly as the
  employer printed it to the left of the vertical line in box 12a–12d, or remove the empty row.
```

The old message for **all** of these was one line: *"W-2 box 12 code K is not supported in v1"* — no
line named, no direction, no exit, and it said "a limit of this tool" even about a code the IRS does
not print at all. There are now **three** shapes of message, because these are three different facts:
a limit of the tool, a transcription error, and an empty row.

★ The refusal also names the **employer**, which matters to a two-earner household holding four W-2s:
the old message never said which one stopped the return.

---

## 4. The retracted paper-check sentences

All three copies inside `return_refuse.rs` are corrected, keeping the *"or delete the direct-deposit
block"* exit and every existing quoted instruction sentence:

* the malformed-number message (`screen_direct_deposit`'s `bad` closure),
* the line-35c *"account type has not been chosen"* message,
* the `screen_direct_deposit` fn doc's one-line version.

Each now says *a return with no deposit instruction is still complete and filable*, then: *"Do not
count on a cheque in the post instead — \"Starting in October 2025, the IRS will generally stop
issuing paper checks for federal disbursements, including tax refunds, unless an exception
applies.\" — so a correct routing and account number is how a refund reaches you."*
(`i1040gi--2025.txt:23824-23827`.)

The `DirectDepositNumberMalformed` doc comment's **factual clause** is corrected and the strictness
argument is **not** weakened — it is restated on terms that never depended on the retracted fact: the
two costs are not commensurable, since refusing costs another look at a cheque (three cells, all
fixable, the return filable meanwhile) while accepting costs a refund wired irrecoverably to whoever
owns the account actually typed. If anything the retraction argues for getting those three cells
right. Nothing implies direct deposit is unbuilt: the comment links
`crate::tax::return_inputs::DirectDeposit` (35b routing / 35c type / 35d account) explicitly, and I
also fixed an internal inconsistency I introduced — the account type is not on a cheque at all, as
`DirectDepositCell::Kind` says in this same file.

Every citation I introduced was machine-resolved against the extract by a script that re-reads each
`file:line-line` span out of the source. The first draft had **three wrong ranges** and one quote with
an ASCII apostrophe where the IRS prints `’` (in *"Alex’s total elective deferral amount"*) — all four
corrected before the gate ran. Three truncated quotes are now marked with an ellipsis.

---

## 5. OUT OF SCOPE — reported, not edited

Other agents own these. I touched none of them.

1. **`crates/btctax-core/src/tax/packet.rs:307-308` quotes the exact sentence I changed**, verbatim:
   *"Correct it, or delete the direct-deposit block: a return with none is complete, and the refund
   then arrives as a paper check"*. It is now a quotation of a message that no longer exists.
   **Highest-value item in this list**: it is a copy of a retracted fact *and* a broken quote.
2. Other paper-check sentences still standing: `printed.rs:700` (*"so a refund arrives as a paper
   check"*), `advisories.rs:233` / `:2986` / `:3086`, `classifier.rs:466`, `return_inputs.rs:1181`,
   `btctax-input-form/src/spec/sections.rs:728`, and `crates/btctax-cli/LIMITATIONS.md:320` (*"you
   receive a **paper check**, and the report says so"*). ★ The advisories module already owns an
   instrument for this class — `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given`
   "scans this module's own source for the retracted sentence" (`advisories.rs:236-237`) — so that
   sweep has somewhere to land.
3. **A stale census REASON, in two files.** `btctax-forms/forms/2024/f1040s3.map.toml:96` and
   `forms/2025/f1040s3.map.toml:156` justify Form 8880 as `unmodeled` with *"no retirement
   contributions collected."* btctax **does** collect them: `w2s[].box12` code
   `D`/`E`/`F`/`G`/`S`/`AA`/`BB`/`EE` is exactly an elective-deferral figure. The `unmodeled` rule is
   still right; its stated reason is not, and the reason is the justification a reviewer reads.
4. **`ELECTIVE_DEFERRAL_CODES` under-detects a §402(g) excess — a potential UNDERSTATEMENT.** The
   limit covers elective deferrals *and designated Roth contributions*; the W-2 instructions' own
   worked example says so — *"Even though the 2026 limit for elective deferrals and designated Roth
   contributions is $24,500, Alex’s total elective deferral amount of $26,500 is reported in box 12
   with code D"*. The list sums only `D E F G S`, so an excess made up of `AA`/`BB`/`EE` is never
   detected and the taxable excess on 1040 line 1h stays unmodeled. I left the set **exactly** as it
   was — changing a cap threshold is a §402A adjudication, not a code-table repair — and recorded the
   boundary in the source above the const. **Recommend a follow-up.**
5. **`G` in that same list is arguably wrong, in the safe direction.** §457(b) deferrals are not
   aggregated under §402(g); they carry their own §457(e)(15) limit, which happens to equal it.
   Including `G` can only cause a *false refusal*, never a wrong figure, so it fails closed.
   Untouched, noted.
6. **Form 8880 / the saver's credit is a live boundary for the eight admitted deferral codes.** It is
   censused `unmodeled` and covered by `Advisory::OtherCreditsOmitted`; refusing here would close
   nothing (an IRA contribution reaches Form 8880 without passing through box 12) and would stop every
   401(k) household from filing. Recorded in the source at the `AA` row rather than acted on.

---

## 6. Residual risk and judgment calls a reviewer should check

* **Code `H` now refuses.** The only narrowing: a return that printed yesterday stops today. I believe
  it is right — the instruction text is explicit and the deduction is censused `unmodeled` — but it is
  a *policy* call about overstating the filer's own tax, and the owner may prefer the advisory route
  (file, and let `Advisory::UnmodeledDeductionsOmitted` say so) for the four forgone-deduction codes
  `H`, `L`, `P`, `R`. Flipping any of them is a one-line verdict change in the table, which is the
  point of having a table.
* **`J`, `Y`, `HH` and `GG` rest partly on a measured ABSENCE** — the code is never named in
  `i1040gi--2025.txt`. That is evidence, not proof; a Pub. 525 reading could in principle find a
  consumer the 1040 instructions do not name. The three admit-verdicts keep the claim inspectable per
  code instead of pooling it into one word ("inert").
* **`TP`/`TT` refuse over a purely informational box.** Defensible either way. I took the fail-closed
  side because nothing in `questions.rs` asks about tips or overtime at all, so admitting them means
  btctax holds the only evidence that this filer has a §224/§225 deduction and acts on none of it. The
  refusal points them at the Part that claims it.
* **The label check pins to the LATEST archived revision**, so archiving a 2027 W-2 that rewords a
  label will red this test. That is intended — come and re-transcribe — but it is a red someone will
  meet at an awkward moment, and it is better known before January than during.
* **The refusal call sites stay INLINE in `screen_inputs_tiered`'s body on purpose.**
  `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` reads the tier census
  out of this function's own source text, so moving them into a helper would take the rule out of the
  census while the test still printed OK. Noted in a comment at the call site so the next person does
  not tidy it away.
