# RECON — TY2026 Schedule A: the line set, the move table, the §68 gate, the two worksheets

**Brief:** `design/agent-reports/BRIEF-schedule-a-ty2026-recon.md` (`45e4c5b16`). **Read-only** — no
source file was changed. Own worktree, all commands foreground. Agent: opus.

## 0. Verdict, and the one thing the port report got wrong

**The extract is present and complete, and every port-report claim about the form checks out against the
text layer — but the port report's headline understates the problem by a factor of six.** The brief asks
whether the Schedule 1-A 37→43 "reused for a different quantity" shape occurs here. It does, **six times
in one form**, and the port report names only one of them (17→18). Lines **13, 14, 15, 16, 17 and 18 all
carry a different quantity in TY2026 than they carried in TY2025** — a shift-by-one cascade from line 13
onward, plus a seventh reuse at line **8d** (from the IRS's own *"Reserved for future use"*).

Two of the six are adverse in the same direction as the Schedule 1-A case if a stale cross-reference is
reused, and one is worse in kind: **TY2025 line 18 is a CHECKBOX and TY2026 line 18 is the itemized
TOTAL**, so a site reading "Schedule A line 18" as a boolean now names a six-figure dollar amount, and
vice versa.

**Second, smaller correction:** the port report's D5 (`:623`) says *"One draft is stale"* about the
13a/13b discrepancy. That is not supported — see §5.2. The two drafts are simultaneously satisfiable,
and **the Schedule A gate does not depend on the answer.**

**Nothing is blocked on a missing artifact for the FORM.** Two things are blocked on missing artifacts
for the **worksheets** — see §1.1.

## 1. Stop-and-report: what is on disk and what is not

| artifact | status |
|---|---|
| `design/forms/extract/f1040sa--2026-DRAFT.txt` | **PRESENT**, 167 lines, `sha256:d49acff9afc2faa1…`, `pdftotext -layout`, both form pages |
| `design/forms/2026/f1040sa--2026-DRAFT.pdf.txt` (provenance note) | **PRESENT** — bytes 268032, full sha256 `d49acff9afc2faa178b7bf1f70d826d03c2c6def43700880f8ae5bb8bb5a0ffa`, with fetch + verify commands |
| the PDF itself | not committed by design (`.gitignore:87` `design/forms/**/*.pdf`) |
| `crates/btctax-forms/forms/2026/f1040sa.map.toml` | **ABSENT** — `forms/2026/` holds only `YEAR.toml` |
| `design/forms/extract/f1040--2026*.txt` | **ABSENT** — and so is the note; the wrong-year file §2.1 #7 records is gone from `design/forms/2026/` |
| `i1040gi--2026` (carries the **Itemized Deductions Worksheet**) | **ABSENT** — IRS publishes it ~Jan–Feb 2027 |
| `i1040sca--2026` (Schedule A instructions) | **ABSENT** — same window |
| the **Charitable Contribution Limitation Worksheet** | **ABSENT** in every year; new for TY2026, lives in the instructions |

`YEAR.toml` already declares the refusal honestly: `f1040sa = "TY2026 revision not released; draft
archived (REBUILT); January 2027 package"`, `forms_expected = []`, `status = "preparing"`. And
`ScheduleAMap::for_year(2026)` refuses twice over — `bundled::map_text` returns `None` so the result is
`FormsError::UnsupportedYear(2026)`; even with a map, `LineSet::parse("f1040sa/2026")` is `None` because
`line_set.rs` knows only `F1040sa_2024` and `F1040sa_2025`, both mapping to the *same*
`Schema::ScheduleAMap`. **There is no silent fallback in the forms layer.** The risk is entirely in
`btctax-core`'s printed layer, which has no `year` in scope at all.

### 1.1 What this means for scheduling

Everything in §2 to §4 can be transcribed **today** from the archived draft. The two worksheets cannot be
transcribed at all — not from a draft, not from a prior year (§68 was suspended 2018–2025, and the
charitable worksheet is brand new). So:

- **The Schedule A line set, the moves, the 17a–17k enumeration, the 8d/8e restructure, the 5e
  constants, the gate's threshold and the gate's operand** — available now.
- **The two worksheets' own line sets** — NOT available, and no proxy exists. Any plan that schedules
  "model the Itemized Deductions Worksheet" before ~Jan 2027 is scheduling an invention.

## 2. The TY2026 line set, from the extract

Captions verbatim from `design/forms/extract/f1040sa--2026-DRAFT.txt`; the `:N` is that file's line.
Column key: **M** = mid money column, **A** = far-right amount column, **cb** = checkbox, **hd** = a
printed heading with no box of its own.

**Machine-derived box-label count.** The right-column labels were enumerated from each extract with
`grep -o -E '[[:space:]](1[0-9]|[1-9])[a-z]?[[:space:]]*$'`, piped through `tr -d ' '`, `sort -u`,
`wc -l` — never hand-counted:

| year | money-box labels | the set |
|---|---|---|
| TY2025 | **25** | 1 2 3 4 5a 5b 5c 5d 5e 6 7 8a 8b 8c 8d 8e 9 10 11 12 13 14 15 16 17 |
| TY2026 | **37** | 1 2 3 4 5a 5b 5c 5d 5e 6 7 8a 8b 8c 8d 8e 9 10 11 12 13 14 15 16 17a 17b 17c 17d 17e 17f 17g 17h 17i 17j 17k 17z 18 |

**+12 net** (13 added: 17a–17k, 17z, 18; 1 retired: the old total at 17). Adding the headings and the
checkbox-only line, the full label set goes **28 → 41** (TY2025's map records *"28 labels, 27 entry
lines, 5 the lone heading"*; TY2026 has **41 labels, 39 entry lines, and TWO headings — 5 and 17**).

### Page-1 masthead

`:44` — *"**Caution:** If you are claiming a net qualified disaster loss on Form 4684, see the
instructions for line **17**."* ★ TY2025 says *"line 16"* — the caution's cross-reference moved with the
enumeration.

### Medical and Dental Expenses

| line | col | caption verbatim | cite |
|---|---|---|---|
| 1 | M | Medical and dental expenses (see instructions) | `:51` |
| 2 | inline | Enter amount from Form 1040 or 1040-SR, line 11b | `:52-53` |
| 3 | M | Multiply line 2 by 7.5% (0.075) | `:54` |
| 4 | A | Subtract line 3 from line 1. If line 3 is more than line 1, enter -0- | `:55` |

### Taxes You Paid

| line | col | caption verbatim | cite |
|---|---|---|---|
| 5 | hd | State and local taxes (SALT). | `:56` |
| 5a | M + cb | State and local income taxes or general sales taxes. You may include either income taxes or general sales taxes on line 5a, but not both. If you elect to include general sales taxes instead of income taxes, check this box | `:58-64` |
| 5b | M | State and local real estate taxes (see instructions) | `:66` |
| 5c | M | State and local personal property taxes | `:68` |
| 5d | M | Add lines 5a through 5c | `:70` |
| 5e | M | Enter the smaller of line 5d or **$40,400** (**$20,200** if married filing separately). If Form 1040 or 1040-SR, line 11b, is more than **$505,000** (**$252,500** if married filing separately), or if you completed Form 2555, Form 4563, or excluded income from Puerto Rico, see instructions | `:73-78` |
| 6 | M | Other taxes. List type and amount: | `:79` |
| 7 | A | Add lines 5e and 6 | `:82` |

### Interest You Paid

| line | col | caption verbatim | cite |
|---|---|---|---|
| 8 | cb | Home mortgage interest and points. If you didn't use all of your home mortgage loan(s) to buy, build, or improve your home, see instructions and check this box | `:83-85` |
| 8a | M | Home mortgage interest and points reported to you on Form 1098. See instructions if limited | `:88-89` |
| 8b | M + dotted | Home mortgage interest not reported to you on Form 1098. See instructions if limited. If paid to the person from whom you bought the home, see instructions and show that person's **name, identifying number, and address** | `:91-94` |
| 8c | M | Points not reported to you on Form 1098. See instructions for special rules | `:98-99` |
| 8d | M | ★ **Mortgage insurance premiums (see instructions)** | `:100` |
| 8e | M | ★ Add lines 8a through **8d** | `:102` |
| 9 | M | Investment interest. Attach Form 4952 if required. See instructions | `:104` |
| 10 | A | Add lines 8e and 9 | `:105` |

### Gifts to Charity

| line | col | caption verbatim | cite |
|---|---|---|---|
| 11 | M | Gifts by cash or check. If you made any gift of $250 or more, see instructions | `:110-111` |
| 12 | M | Other than by cash or check. If you made any gift of $250 or more, see instructions. You must attach Form 8283 if over $500 | `:113-114` |
| 13 | M | ★★★ **Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet** | `:116-117` |
| 14 | M | ★★★ **Carryover from prior year** | `:119` |
| 15 | A | ★★★ **Add lines 13 and 14** | `:120` |

★ Note what line 15 does **not** say: it is *"Add lines 13 and 14"*, **not** "add 11 through 14". Lines
11 and 12 feed the **worksheet**, and only the worksheet's line 6 returns to the schedule at line 13. So
on TY2026, **lines 11 and 12 are inputs to a document that is not on disk, and the schedule's own charity
subtotal never adds them directly.**

### Casualty and Theft Losses

| line | col | caption verbatim | cite |
|---|---|---|---|
| 16 | A | ★★★ Casualty and theft loss(es) from a **federally or state-declared** disaster (other than net qualified disaster losses). Attach Form 4684 and enter the amount from line 18 of that form. See instructions | `:121-123` |

### Other Itemized Deductions

| line | col | caption verbatim | cite |
|---|---|---|---|
| 17 | hd | ★★★ **Other itemized deductions (see instructions).** | `:124` |
| 17a | M + cb | Deductible gambling losses … *"If you have gambling winnings that you reported on Schedule C or Schedule E, check here"* | `:131-133` |
| 17b | M | Net qualified disaster loss | `:135` |
| 17c | M | Standard deduction claimed with qualified disaster loss | `:137` |
| 17d | M | Casualty and theft losses of income-producing property | `:139` |
| 17e | M | Federal estate tax on income in respect of a decedent | `:141` |
| 17f | M | Deduction for amortizable bond premium | `:143` |
| 17g | M | Ordinary loss attributable to a contingent payment debt instrument or an inflation-indexed debt instrument | `:145-146` |
| 17h | M | Deduction for repayment of amounts under a claim of right if over $3,000 | `:148-149` |
| 17i | M | Certain unrecovered investment in a pension | `:151` |
| 17j | M | Impairment-related work expenses of a disabled person | `:153` |
| 17k | M | Deductible educator expenses **not reported on Schedule 1 (Form 1040)** | `:155-156` |
| 17z | A | Add lines 17a through 17k | `:157` |

★ **17c is structurally novel for btctax.** *"Standard deduction claimed with qualified disaster loss"* is
a line a **standard-deduction** filer puts on Schedule A. `printed::schedule_a_lines` returns `None`
unless `ar.deduction_is_itemized`, so that path cannot exist today. It is reachable only through a net
qualified disaster loss, which needs Form 4684 — unmodelled — so its determinate provenance is a
**refusal**, not a new field. It must be *recorded* as such, not left blank.

### Total Itemized Deductions

| line | col | caption verbatim | cite |
|---|---|---|---|
| 18 | A | ★★★ **Is the amount on Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and 13b of that form, more than $384,350?** / **No.** Your deductions are not limited. Add the amounts in far-right column for lines 4 through 17z. Also enter this amount on Form 1040 or 1040-SR, line 12e. / **Yes.** Your deductions may be limited. See the Itemized Deductions Worksheet in the instructions to figure the amount to enter | `:158-163` |
| 19 | cb | ★★★ If you elect to itemize deductions even though they are less than your standard deduction, check this box | `:165-166` |

## 3. The move table — and yes, the dangerous shape occurs

Classes: **same** / **moved** (same quantity, new number) / **retired** / ★ **REUSED** (the number stays,
the quantity behind it changes) / **restated** (same number and quantity, changed text or constants).

| TY2025 | quantity | TY2026 | class | note |
|---|---|---|---|---|
| 1 | medical expenses | 1 | same | |
| 2 | AGI (from 1040 L11b) | 2 | same | |
| 3 | 7.5% floor | 3 | same | |
| 4 | medical allowed | 4 | same | |
| 5 | SALT heading | 5 | same | |
| 5a | income or sales taxes | 5a | same | |
| 5b | real-estate taxes | 5b | same | |
| 5c | personal-property taxes | 5c | same | |
| 5d | add 5a–5c | 5d | same | |
| 5e | SALT cap | 5e | **restated** | 40,000→**40,400**, 20,000→**20,200**, 500,000→**505,000**, 250,000→**252,500**; and *"line 11b is more than"* → *"line 11b, is more than"* |
| 6 | other-taxes write-in | 6 | same | |
| 7 | add 5e and 6 | 7 | same | |
| 8 | mortgage heading + mixed-use cb | 8 | **restated** | *"see instructions and check this box"* (TY2025: *"check this box"*) |
| 8a | 1098 interest | 8a | same | |
| 8b | non-1098 interest | 8b | **restated** | *"identifying no."* → *"identifying number"* |
| 8c | points not on 1098 | 8c | same | |
| 8d | *"Reserved for future use"* | 8d | ★ **REUSED** | now **Mortgage insurance premiums**. OBBBA §70108(a)(1)(D) inserts §163(h)(3)(F)(i)(III). **Benign on its own** because nothing read it (a ReadOnly widget, never written) — but see 8e |
| 8e | **Add lines 8a through 8c** | 8e | ★ **REUSED (formula)** | now **Add lines 8a through 8d**. Same number, same field name, **different sum** — an 8e that omits 8d **understates the deduction and OVERSTATES tax** by the MIP |
| 9 | investment interest | 9 | same | |
| 10 | add 8e and 9 | 10 | same | |
| 11 | cash gifts | 11 | same | **but it now feeds the worksheet, not the subtotal** |
| 12 | noncash gifts (crypto lands here) | 12 | same | same caveat |
| **13** | **Carryover from prior year** | **14** | **moved** | |
| **13** | — | **13** | ★★★ **REUSED** | now *"line 6 of the Charitable Contribution Limitation Worksheet"* |
| **14** | **Add lines 11 through 13** | **15** | **moved + restated** | now *"Add lines 13 and 14"* |
| **14** | — | **14** | ★★★ **REUSED** | now the prior-year carryover |
| **15** | casualty/theft (federally declared) | **16** | **moved + restated** | *"federally"* → *"federally **or state-declared**"* |
| **15** | — | **15** | ★★★ **REUSED** | now the charity subtotal |
| **16** | *"Other—from list in instructions"* write-in | **17a–17k + 17z** | **retired → enumerated** | one free-text block becomes **11 named lines + a subtotal** |
| **16** | — | **16** | ★★★ **REUSED** | now casualty/theft |
| **17** | **the itemized TOTAL, to 1040 L12e** | **18** | **moved + gated** | the §68 gate is new; the "No" branch also changes *"the amounts in the far right column for lines 4 through 16"* → *"the amounts in far-right column for lines 4 through 17z"*, and *"Also, enter"* → *"Also enter"* |
| **17** | — | **17** | ★★★ **REUSED** | now the *"Other itemized deductions"* **heading** — a label with no box |
| **18** | §63(e) **CHECKBOX** | **19** | **moved** | |
| **18** | — | **18** | ★★★ **REUSED, and across types** | a checkbox becomes the **money total** |

**Six reuse collisions (13, 14, 15, 16, 17, 18), plus one at 8d and a formula reuse at 8e.** The Schedule
1-A 37→43 case was one collision; this is a contiguous cascade.

**Is it adverse?** Yes, in the same direction as the Schedule 1-A case, at two points:

- **13 versus 14.** A site reading TY2026 line 13 believing it is *"carryover from prior year"* gets the
  **worksheet-limited current-year gift total** instead — systematically **larger** for an itemizer with
  gifts. If that value is then also added as a current-year gift, the charity block double-counts: an
  **overstated deduction / understated tax**, the direction that matters.
- **18 versus 17.** A site reading TY2026 line 18 as the §63(e) **boolean** gets a dollar total; a site
  reading it as the total when the gate answered **Yes** gets an **unlimited** total the form says *"may
  be limited"* — again understating tax.
- Opposite direction, still wrong: **8e omitting 8d**, and **15 read as the old "add 11 through 13"**,
  both overstate tax (a forgone deduction).

## 4. Every site in `crates/` naming a Schedule A line number

Derived with grep, not recalled. Prose citations — the pattern
`schedule ?a,? ?(line|l)s? ?-?[0-9]+[a-z]?`, case-insensitive, over `crates/` — match **243 source
lines**. Bucketed by the number cited:

| Sch-A line cited | prose citations | still means the same thing in TY2026? |
|---|---|---|
| 1 | 1 | **yes** |
| 2 | 5 | **yes** |
| 3 | 1 | **yes** |
| 5a | 10 | **yes** |
| 5b | 7 | **yes** |
| 5c | 1 | **yes** |
| 5d | 2 | **yes** |
| 5e | 8 | **yes** — number stable; **the four constants inside the caption all change** |
| 7 | 22 | **yes** |
| 8 | 4 | **yes** (caption restated) |
| 8a | 41 | **yes** |
| 8b | 29 | **yes** (caption restated: *"identifying number"*) |
| 8c | 2 | **yes** |
| 8d | 3 | ★ **NO — REUSED.** Was *"Reserved for future use"*, now *mortgage insurance premiums* |
| 8e | 1 | ★ **NO — the SUM changed** (8a–8c becomes 8a–8d) |
| 9 | 27 | **yes** |
| 11 | 8 | **yes** number; **no longer summed into the subtotal directly** |
| 12 | 42 | **yes** number; same caveat as 11 |
| 13 | **3** | ★★★ **NO — REUSED** |
| 17 | **4** | ★★★ **NO — moved to 18** |

About 215 numbered prose citations in total. **Cited numbers 1 through 12 are safe; 13 and 17 are not;
8d and 8e are wrong in a way a number-only grep does not reveal.** Line 14 draws 0 prose citations but 4
structural ones.

### The structural sites — field names and cells, not prose

| site | what it names | verdict |
|---|---|---|
| `crates/btctax-core/src/tax/printed.rs:1556` `ScheduleALines` | `line13`, `line14`, `line17` fields plus `line18_elects_smaller`; derived at `:1689-1693`, moved at `:1722-1724` | ★ **all four wrong for TY2026** (13/14 reused, 17→18, 18→19). 9 in-file references to `line13`/`line14`/`line17` in the Schedule A region |
| `printed.rs:1642` `schedule_a_lines` | `let line14 = line11 + line12 + line13;` and `let line17 = line4 + line7 + line10 + line14;` | ★ **both formulas retired.** TY2026 is `15 = 13 + 14`, and the total is `4 + 7 + 10 + 15 + 16 + 17z` at line 18, behind the gate |
| `printed.rs:1543` (struct doc) | *"Unmodeled lines are BLANK … line 6 …, line 15 … and line 16"* | ★ 15 becomes 16; 16 becomes 17a–17k/17z |
| `printed.rs:1547-1554` (struct doc) | ★★ already documents 8d/8e coming back for TY2026, citing `f1040sa--2026-DRAFT.txt:100-102` | **correct** — the one TY2026 Schedule A fact already written down |
| `crates/btctax-core/src/tax/return_1040.rs:339` `ScheduleAParts` | `charitable_carryover_13` (5 sites), `charitable_14` (4), `total_17` (9) | ★ **all 18 name a moved number.** `charitable_carryover_13` is the exact 13-versus-14 collision |
| `return_1040.rs:1875` `AbsoluteReturn.itemized_deduction` | doc: *"Schedule A **line 17** itemized total"* | ★ **stale.** The field name is semantic and correct per `CLAUDE.md`; only the doc's number is wrong |
| `crates/btctax-forms/src/map.rs:3734` `ScheduleAMap` | `line13`/`line14`/`line17` `MoneyCell`s plus `check_18_elects_smaller` | ★ TY2026 needs a **new schema variant**, not a widened one: 13/14 change meaning, 12 boxes are added, one is retired |
| `crates/btctax-forms/forms/2024/f1040sa.map.toml:82-86`, `forms/2025/f1040sa.map.toml:143-147` | `line13`/`line14`/`line17` FQNs with the captions in trailing comments; `check_18_elects_smaller` bound to `Line18_ReadOrder[0]` | correct **for their own years** — this is the right pattern; TY2026 needs its own file |
| `crates/btctax-forms/src/schedule_a.rs:94-116` | the `[(Usd, usize); 21]` plan, ending `(lines.line17, COL_AMOUNT), // 17 total itemized -> 1040 L12` | ★ **length, order and the last entry all change.** TY2026 is 37 money cells and the destination is 1040 **L12e** |
| `crates/btctax-core/src/tax/line_coverage.rs:1326` `cover_schedulealines` | `Coverage::quoting("2024")` plus rows `"13"`, `"14"`, `"17"` carrying TY2024 caption text | ★ three rows name a moved number, and the whole block is verified against the **TY2024** extract |
| `crates/btctax-core/src/tax/packet.rs:1796` | the literal `"1040 12 ← Schedule A line 17"` | ★ **two numbers wrong**: TY2026 is `1040 12e ← Schedule A line 18` |
| `crates/btctax-core/src/tax/charitable.rs:258` | *"Carryover-in is consumed AFTER current-year gifts, within the class ceiling → Schedule A **line 13**"* | ★★★ **the 13-versus-14 collision, in the module that owns the quantity** |
| `crates/btctax-core/src/tax/return_1040.rs:3918` | *"{next}'s Schedule A **line 13** is deliberately outside …"* | ★★★ the same collision, on the **carry-forward into next year's return** |
| `crates/btctax-core/src/tax/amt.rs:75` | *"Never pass the itemized total (Schedule A line 17)"* | ★ stale number in a warning that is otherwise right |
| `crates/btctax-core/src/tax/testonly.rs:649` | *"L12 IS Schedule A line 17"* | ★ stale |
| `crates/btctax-core/src/tax/advisories.rs:572-580` `UnmodeledDeductionsOmitted` | **filer-facing prose** naming *"(line 15, Form 4684)"* and *"(line 16)"* | ★★ a return-adjacent string citing two numbers that both move |
| `crates/btctax-core/src/tax/return_inputs.rs:352` `Form1098::box5_mortgage_insurance` | *"Collected against Schedule A's **line 8d**"* | ★ **correct, and already collected** — see §4.1 |
| `crates/xtask/src/box_census.rs:913` | the 8d census note, already citing `f1040sa--2026-DRAFT.txt:100-102` | **correct** |
| `crates/btctax-forms/tests/full_return_forms.rs:1159` `sch_a_lines()`, and `:1258` | a hand-built literal carrying `line13`/`line14`/`line17`; *"Line 8d (f1_21) is the IRS's own ReadOnly 'Reserved for future use' widget — never written"* | ★ the 8d assertion is **true for 2024/2025 and false for 2026**, in a test that would keep passing |
| `crates/xtask/src/census_join.rs:197-199` `DIRECTION_OF_CAPTION` | `("Itemized Deductions", Overstates, "Add the amounts in the far right column for lines 4 through 16")` | ★★★ see §4.2 — a **false green waiting to happen** |
| `crates/btctax-forms/forms/2025/f1040sa.map.toml:185-189` `[[direction]]` | `first_line = "1"`, `last_line = "18"` | ★ TY2026's last line is **19** |
| `forms/2025/f1040sa.map.toml:212-214` census | one `line = "15"` entry and two `line = "16"` entries, each quoting the TY2025 caption verbatim | ★ both numbers move and line 16's *text* also changes; the line-16 entry must become **11 entries** |

### 4.1 One genuinely good surprise

`Form1098::box5_mortgage_insurance` is **already collected**, deliberately, with its provenance recorded,
against a line that printed *"Reserved for future use"* in both shipped years. So **line 8d is the one
new TY2026 quantity the input surface can already answer.** The census note and the struct doc both cite
the 2026 draft. This is the shape the rest of the port should look like.

### 4.2 One instrument that will go green on the wrong year

`census_join.rs::evidence_findings` (`:716`) asserts that each `DIRECTION_OF_CAPTION` evidence sentence is
printed by *some* in-scope extract — and `run()` (`:800-827`) builds `corpus` by **concatenating every
committed year's extract**. `DIRECTION_OF_CAPTION` is keyed on the **caption alone**, and Schedule A's
caption *"Itemized Deductions"* is stable across years.

Consequence: once TY2026 is wired, the reading for TY2026 Schedule A is graded on the evidence sentence
*"Add the amounts in the far right column for lines 4 through 16"* — which the TY2026 form **does not
print** — and the check stays **green**, because `f1040sa--2025.txt` is still in the corpus. This is
`CLAUDE.md`'s *"the thing that decides was not the thing that knows"* exactly. Flagged only; no change
made.

## 5. The §68-style gate

### 5.1 The threshold, quoted

Line **18**, `design/forms/extract/f1040sa--2026-DRAFT.txt:158-163`, verbatim:

*"Is the amount on Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and 13b of that form,
more than **$384,350**?"*

*"**No.** Your deductions are not limited. Add the amounts in far-right column for lines 4 through 17z.
Also enter this amount on Form 1040 or 1040-SR, line 12e."*

*"**Yes.** Your deductions may be limited. See the Itemized Deductions Worksheet in the instructions to
figure the amount to enter"*

**The line it reads:** Form 1040 line **11b** (adjusted gross income), minus 1040 lines **13a** and
**13b**.

**★ The threshold is a number already in the codebase, meaning something else.** `dec!(384350)` lives at
`crates/btctax-adapters/src/tax_tables.rs:823` and `crates/btctax-core/src/tax/testonly.rs:2893` as the
**TY2026 MFS 37% bracket start** (half of MFJ 768,700, §1(j)(2)(D)), pinned by
`tax_tables.rs:1415 ty2026_mfs_37_pct_starts_at_384350`. The TY2026 37% starts measured from the shipped
table are Single **640,600**, MFJ **768,700**, HoH **640,600**, MFS **384,350** — so the form's single
printed threshold is the **minimum across filing statuses**, a deliberately conservative universal
screen, consistent with the form saying *"may be limited"* rather than *"is limited"*.

Two things follow, pulling in opposite directions:

- **The constant is already transcribed** and needs no fresh sourcing.
- **It must NOT be read from the bracket table.** The gate wants one number for every filer; reading
  `ordinary_for(status).brackets.last().lower` would hand a Single filer 640,600 and **screen out filers
  the form screens in**. Whether 384,350 is 384,350 *by construction* or merely equal to the MFS start
  this year is a question only `i1040gi--2026` settles, and it is not on disk. Until then the honest form
  is a transcribed literal with the coincidence documented beside it.

### 5.2 Adjudicating the 13a / 13b discrepancy (port report `:623`, D5)

**What the document actually prints:** the TY2026 Schedule A draft line 18 prints *"lines 13a and 13b of
that form"* — **both**, verbatim, `:158-159`. There is **no discrepancy internal to Schedule A**.

**The port report's D5 says *"One draft is stale."* That is not supported.** The three relevant texts:

- `f1040s1a--2025.txt:110-111` — L38: *"Add lines 13, 21, 30, and 37. Enter here and on Form 1040 or
  1040-SR, line **13b**, or on Form 1040-NR, line 13c"*
- `f1040s1a--2026-DRAFT.txt:224` — L44: *"Add lines 15, 27, 36, and 43. Enter here and on Form 1040,
  1040-SR, or 1040-NR, line **13a**"*
- `f1040--2025.txt:98,100` — **13a** = *"Qualified business income deduction from Form 8995 or Form
  8995-A"*; **13b** = *"Additional deductions from Schedule 1-A, line 38"*

So TY2026 Schedule 1-A moved its destination from 13b to 13a **and** folded the 1040-NR's separate 13c
into it. That is the signature of a deliberate **swap** (TY2026 1040: 13a = Schedule 1-A total, 13b =
QBI), under which **both drafts are simultaneously correct**. Schedule A names *both* sub-lines;
Schedule 1-A names *one*. They contradict only on the assumption that Schedule A's pair is a closed
statement of which sub-line is which — and it is not.

**The load-bearing conclusion: the Schedule A gate is INDIFFERENT to the 13a/13b assignment.** Its
operand is `AGI − (QBI deduction) − (Schedule 1-A total)` in whichever order the 1040 prints them. And
that quantity is exactly §68(b)'s *"taxable income plus itemized deductions"*, because TY2025's own 1040
gives `L15 taxable income = L11b − (L12e + L13a + L13b)`, hence `L11b − L13a − L13b = taxable income +
L12e`.

So **re-archiving `f1040--2026` does not block Schedule A.** D5 lists it as one of the two things that
unblock the port; for Schedule A specifically it is not one of them. It stays required for the 1040 map
and emitter, where the assignment does matter.

### 5.3 Does `FullReturnParams` have a field for it?

**No.** `crates/btctax-core/src/tax/tables.rs:542` `FullReturnParams` carries exactly: `year`,
`std_deduction`, `std_aged_blind_married`, `std_aged_blind_unmarried`, `dependent_std_floor`,
`dependent_std_earned_addon`, `salt: SaltLimitation`, `kiddie_unearned_threshold`,
`acquisition_debt_ceiling`, `qualifying_relative_gross_income_limit`, `child_tax_credit_per_child`,
`credit_for_other_dependents_per_person`, `elective_deferral_limit`, `ftc_ceiling`,
`qbi_ti_threshold_unmarried`, `qbi_ti_threshold_married`, `qbi_phase_in_range_unmarried`,
`qbi_phase_in_range_married`, `student_loan_phaseout_unmarried`, `student_loan_phaseout_married`,
`amt: AmtParams`, `hsa: HsaParams`.

A repo-wide grep over `crates/` for `pease`, `sec_68`, `§68`, `itemized_limit`, `charitable_floor` and
`0.005` returns **no §68 modelling and no charitable-floor modelling anywhere** — not in
`FullReturnParams`, not in `charitable.rs`, not in `printed.rs`. Both are entirely new surface.

★ The precedent for the right shape is one field above: `salt: SaltLimitation` is documented as *"Per-year
INSTRUMENT, not a per-year constant — the 2024 and 2025 Schedule A ask different questions, so
`SaltLimitation` is an **enum** and the compiler forces every consumer to handle both."* The §68 gate is
the same kind of thing — TY2025 asks nothing, TY2026 asks a question — so it wants an **enum with an
unlimited arm for 2024/2025 and a gated arm for 2026**, not an `Option<Usd>` threshold a 2025 consumer
can quietly ignore.

## 6. The two worksheets

**Neither is printed on the schedule.** The archived draft is two form pages ending at line 19 (`:167`
*"Schedule A (Form 1040) 2026"*); no worksheet appears anywhere in the 167-line extract. Line 18 says
*"See the Itemized Deductions Worksheet **in the instructions**"*, and line 13 cites the *"Charitable
Contribution Limitation Worksheet"* by name with no location — the IRS convention for a worksheet in the
same instruction booklet.

So **both live in `i1040gi--2026` / `i1040sca--2026`, and neither is on disk.** They are separate
documents to **archive**, and then to transcribe under `CLAUDE.md`'s worksheet rule — not lines to add to
the schedule. Archiving them is gated on the IRS, ~Jan–Feb 2027, per `design/TY2026_PORT_REPORT.md:40`.

★ They are also **not fetchable by the existing archiver**: `scripts/archive_drafts.py`'s `STEMS` is
`draft_stems()`, derived from the `crates/btctax-forms/forms/*/*.map.toml` glob — i.e. only the **forms
btctax bundles**. No `i`-prefixed instruction stem can ever be enumerated by it. Whatever archives these
two needs a different door. (Naming the gap only; nothing proposed.)

### 6.1 Charitable Contribution Limitation Worksheet — what it needs

From the schedule alone, the structural facts are: it **consumes lines 11 and 12**, it has **at least 6
lines**, its **line 6 becomes Schedule A line 13**, and the prior-year carryover is **not** inside it
(that is line 14, added separately at line 15).

**What btctax already has.** `charitable.rs` computes the §170(b) class ceilings and the §170(d) carryover
split; `ScheduleAParts` carries `charitable_cash_11`, `charitable_noncash_12`, `charitable_carryover_13`,
`charitable_14`; crypto donations route through `crypto_charitable_gifts` and §170(e) onto line 12; Form
8283 attaches over the 500 printed on line 12. AGI is available. So the **machinery is largely present**
— what is absent is the worksheet's own **line structure**, which is what the transcription rule requires
and what cannot be invented.

**The one input question worth flagging now.** The worksheet is the place a **new floor** on an itemizer's
charitable deduction would be applied — which is presumably why a limitation worksheet appears in the
same year that charitable gifts stop being summed directly into the subtotal. If the floor is a
percentage of AGI, btctax can answer it from AGI with **no new collection**. If the worksheet splits gifts
by a **class btctax does not collect** — a contribution-type distinction the current `CharitableGift`
does not carry — that is a collection gap. **This is not determinable from the form text.** I am not
going to guess it; it is the first thing to read in `i1040sca--2026`.

### 6.2 Itemized Deductions Worksheet — what it needs

Inputs the gate itself names, all of which btctax can produce today: 1040 line 11b (AGI), the QBI
deduction, the Schedule 1-A total, and the schedule's own lines 4 through 17z.

Inputs it will need that btctax **cannot** produce today:

1. **The filer's 37%-bracket dollar amount by filing status.** Available in `tax_tables.rs`, but not
   reachable from the printed layer, which has **no `year` in scope at all** (`printed.rs`; port-report
   row 21).
2. **Which itemized deductions are excluded from the limitation.** §68 has always carved out classes. If
   the TY2026 worksheet carves out, say, medical or investment interest, the limitation is a function of
   a **partition of Schedule A's own lines** — a transcription target that exists nowhere in the codebase
   today.
3. **Nothing about the filer.** ★ Worth stating plainly: unlike the Schedule 1-A work, **the §68 gate asks
   the filer no new question.** Every operand is already computed. The blocker is the worksheet's text,
   not the input surface.

### 6.3 The 11 new 17a–17k lines — provenance, per the answered-ness rule

TY2025 handled all of this with **one** `unmodeled` census entry (line 16, *"btctax offers no write-in
itemized deduction"*, `covered_by = "Advisory::UnmodeledDeductionsOmitted"`). TY2026 needs **11 separate
determinate provenances**. Measured against the current input surface:

| line | current btctax posture | determinate provenance available today |
|---|---|---|
| 17a deductible gambling losses | `DocumentRow::W2g` answers *"btctax cannot take gambling winnings or losses"* (`document_census.rs:209`); `questions.rs:945` and `return_refuse.rs:3075` refuse a return with gambling | ★ **REFUSAL** — it already exists; it only needs recording against this line |
| 17b net qualified disaster loss | Form 4684 unmodelled (13 mentions across `crates/`, no model) | **refusal / unmodeled** |
| 17c standard deduction claimed with QDL | unreachable — `schedule_a_lines` needs `deduction_is_itemized` | **refusal**, via Form 4684 |
| 17d casualty/theft of income-producing property | not modelled | **unmodeled** + advisory |
| 17e federal estate tax on income in respect of a decedent | not modelled | **unmodeled** + advisory |
| 17f amortizable bond premium | 9 mentions, none a model | **unmodeled** + advisory |
| 17g contingent-payment / inflation-indexed debt ordinary loss | **0** mentions | **unmodeled** + advisory |
| 17h claim-of-right repayment over 3,000 | **0** mentions | **unmodeled** + advisory |
| 17i certain unrecovered investment in a pension | **0** mentions | **unmodeled** + advisory |
| 17j impairment-related work expenses | 1 mention, not a model | **unmodeled** + advisory |
| 17k educator expenses **not on Schedule 1** | Schedule 1 L11 educator expenses are `unmodeled` (`forms/2024/f1040s1.map.toml:153`) — ★ but **this is a different quantity**, expressly the part *not* reported there | **unmodeled** + advisory, and it needs its **own** entry, never a cross-reference to Schedule 1's |

★ All eleven have a determinate provenance available **without collecting anything new**, and
`Advisory::UnmodeledDeductionsOmitted` already exists as the announcement channel. But that advisory's own
text names *"(line 15, Form 4684)"* and *"(line 16)"*, both of which move, so it must be re-transcribed
for TY2026 alongside the census entries.

## 7. What must be transcribed, in one list

Available **now**, from the archived draft:

1. A TY2026 Schedule A **transcription struct** carrying all 41 labels / 37 money boxes, named for the
   TY2026 line numbers, instruction text verbatim — **not** a widened `ScheduleALines`. The
   13/14/15/16/17/18 collisions mean one struct cannot serve both revisions without a field name meaning
   two different things.
2. A new `LineSet::F1040sa_2026` mapping to a **new** `Schema` variant. Reusing `Schema::ScheduleAMap`
   would let a TY2026 map parse straight into TY2025 semantics.
3. The 5e constants: **40,400 / 20,200 / 505,000 / 252,500**.
4. **8d** (mortgage insurance premiums — the figure is already collected) and the **8e = 8a+8b+8c+8d**
   sum.
5. The **17a–17k / 17z** enumeration, with 11 provenance decisions (§6.3) and a re-transcribed
   `UnmodeledDeductionsOmitted`.
6. Line **16**'s widened caption (*"federally or state-declared"*), line **19**'s checkbox, the page-1
   caution's *"line 17"*, the `[[direction]]` block's `last_line = "19"`, and `DIRECTION_OF_CAPTION`'s
   evidence sentence — which cannot stay keyed on the caption alone (§4.2).
7. The §68 gate as a **per-year instrument** on the `SaltLimitation` pattern, with **384,350** a
   transcribed literal plus a comment stating explicitly that it is *not* to be read from the bracket
   table (§5.1).
8. `printed.rs` needs `year` in scope before any of 1 through 7 is expressible at all. That is port-report
   row 21, and it sits upstream of this entire list.

**Blocked on the IRS** (~Jan–Feb 2027), and not inventable:

9. The **Itemized Deductions Worksheet** line set — from `i1040gi--2026`.
10. The **Charitable Contribution Limitation Worksheet** line set — from `i1040sca--2026` /
    `i1040gi--2026`, plus whatever gift classification it demands (§6.1).

**Not blocked on** `f1040--2026` (§5.2) — that blocks the 1040, not Schedule A.

## 8. Corrections to `design/TY2026_PORT_REPORT.md`

| where | what it says | what the extract says |
|---|---|---|
| row 20 (`:135`) and the "new" row (`:454`) | *"moves the total to line 18 … enumerates the former free-text line 16 as 17a–17k/17z, and moves charitable to a new Charitable Contribution Limitation Worksheet at line 13"* | **all three confirmed verbatim** |
| row 20 and the "new" row | names **one** moved line (17→18) | ★ **six lines are reused for a different quantity** — 13, 14, 15, 16, 17, 18 — plus 8d and the 8e formula. The report's scope for this area is one sixth of the real one |
| §7 D5 (`:623`) | *"One draft is stale, and the arbiter — a genuine TY2026 Form 1040 — is the wrong-year document … **Action:** re-archive `f1040--2026` first; this is one of the two things that unblock it"* | ★ **not supported.** Both drafts are satisfiable under a 13a/13b swap, and the Schedule A gate subtracts **both**, so it is indifferent. Re-archiving `f1040--2026` does not unblock Schedule A |
| row 20 | silent on 8d/8e and on the 5e constants | 8d is **REUSED**, 8e's sum **changed**, and 5e's four constants all move |
| row 20 | silent on the instruments | `census_join.rs:197-199` will grade TY2026 on TY2025's evidence sentence and stay green (§4.2); `full_return_forms.rs:1258` asserts 8d is a never-written ReadOnly widget — true for 2024/2025, false for 2026 |
| §2.1 #7 | *"`design/forms/2026/f1040--2026-DRAFT.pdf` is the TY2025 Form 1040"* | the file and its provenance note are **no longer in `design/forms/2026/`**, and `YEAR.toml`'s `f1040` entry records why. The claim is historically accurate and now stale |

## 9. Scope discipline

No source file changed. No commit, no push, no subagents, every command in the foreground. `printed.rs`'s
field count was not re-derived — FR-159's **149** stands. No claims about TY2025 as a filed year, no state
returns, and no implementation proposed beyond naming what must be transcribed and collected.
