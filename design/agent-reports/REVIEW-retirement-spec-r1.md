# REVIEW r1 — `design/ty2025/SPEC_retirement_income.md` (retirement income, 1040 lines 4a–6b)

**Reviewer:** independent agent, one round per owner ruling S6. **Date:** 2026-09-15.
**Artifact under review:** `design/ty2025/SPEC_retirement_income.md`, 52,410 bytes / 708 lines.
**Verdict: 3 Critical / 11 Important / 10 Minor / 2 Nit. NOT GREEN — the build is blocked.**

## Provenance of this review's own measurements — read this first

My worktree (`agent-aefb8bd1d4a98122a`) was branched at `cc29bb30e` = `origin/main`, which is **58 commits
behind local `main` (`bf2cbc2ca`)**. The controller overwrote the SPEC file mid-review but not the code.
**Every code, `FOLLOWUPS.md` and map-file measurement below was therefore re-taken at revision `main`
(`bf2cbc2ca`), not from my worktree's checkout.** Extract files under `design/forms/extract/` are unchanged
between the two revisions apart from two new 2026 HSA extracts, so every primary-source line number below is
valid in both trees.

Primary sources read directly: `design/forms/extract/i1040gi--2025.txt`,
`design/forms/extract/i1040gi--2024.txt`, `design/forms/extract/f1040--2025.txt`,
`design/forms/extract/f1040--2024.txt`. Both oracle engines read at source:
`/scratch/code/bitcoin_tax/.venv` taxcalc **6.8.2** (`taxcalc.calcfunctions.SSBenefits`) and
`/home/bcg/OpenTaxSolver2024_22.07_linux64/src/taxsolve_US_1040_2024.c` (`SocSec_Worksheet`).

---

# CRITICAL

## C-1. TY2024 Form 1040 HAS NO LINE 6d. The MFS lived-apart disclosure is a write-in "D" on line 6a, and the spec builds a checkbox that does not exist while omitting a disclosure whose absence the IRS says triggers a math error notice.

**Sections:** §6 (the `6d` row), S-10, S-7, OQ-2, §5.1 (ws9), R-8, §10 (the conformance KAT), §7.

**What is wrong.** The owner ruled **both years**. §6 gives line `6d` the field `line6d_checked: bool` with
production `Collected` and printed text cited to `f1040--2025.txt:80`. There is no such line on the TY2024
form, and the TY2024 mechanism is a **different kind of entry** — a character written into the line 6a cell,
not a checkbox.

**The primary source settles it, in four places.**

`design/forms/extract/f1040--2024.txt:65-71` — the 2024 income block runs 4a/4b, 5a/5b, 6a/6b, then
`c If you elect to use the lump-sum election method…`, then line 7. Searching `f1040--2024.txt` for `6d`
returns **nothing**; searching it for `lived apart` returns **nothing**.

Section headings, both years:

    i1040gi--2024.txt:3117   Lines 6a, 6b, and 6c
    i1040gi--2025.txt:3233   Lines 6a, 6b, 6c, and 6d

`i1040gi--2024.txt:3361-3362` (the worksheet's own *Before you begin*):

> *"If you are married filing separately and you lived apart from your spouse for all of 2024, enter "D" to
> the right of the word "benefits" on line 6a. If you don't, you may get a math error notice from the IRS."*

`i1040gi--2024.txt:3415-3417` (worksheet line 9's STOP): *"…be sure you entered "D" to the right of the word
"benefits" on line 6a."* — where TY2025 reads *"…be sure you checked the box on line 6d"*
(`i1040gi--2025.txt:3445-3446`).

**The root cause is a machine-check claim that is false.** S-7 and OQ-2 both assert *"the worksheet is
byte-identical across the two years (S-7, machine-checked)"*, and the review brief repeats it as settled.
S-7's actual check is the four dollar thresholds only. A normalized diff of the two worksheet blocks
(`i1040gi--2024.txt:3334-3475` against `i1040gi--2025.txt:3364-3505`) returns **seven** substantive
differences, of which four are not year labels:

| ws | TY2024 | TY2025 |
|---|---|---|
| *Before you begin* 1 | *"Figure any write-in adjustments to be entered on Schedule 1, line 24z"* | *"If the instructions for Schedule 1, line 24z, have you enter a write-in adjustment on line 24z, figure that write-in before completing this worksheet"* |
| *Before you begin* 2 | **enter "D" to the right of the word "benefits" on line 6a** | **check the box on line 6d** |
| 3 | *"lines 1z, 2b, 3b, 4b, 5b, **7**, and 8"* | *"lines 1z, 2b, 3b, 4b, 5b, **7a**, and 8"* |
| 9 (STOP) | **entered "D" … on line 6a** | **checked the box on line 6d** |

So S-10's *"Only two things are year-shaped: the line-9 operand list … and the field map"* is false. At least
four things are: the line-9 operand list, the field map, **the worksheet's own line 3 and line 9 text**, and
**the entire MFS lived-apart disclosure mechanism**. A worksheet transcription cannot be year-agnostic
(S-4/S-10) when two of its own printed sentences differ; per `CLAUDE.md` a transcription struct is per
line-set revision, and this one has two revisions.

**Why Critical.** The TY2024 MFS-lived-apart retiree is exactly the filer R-8 exists to protect. Executing
this spec files that return with the $25,000/$9,000 thresholds correctly applied and **no disclosure at
all** — and the IRS's own sentence names the consequence: a math error notice, which recomputes benefits
without the lived-apart treatment. The filer gets the right figure on the page and an adjustment in the post.
Separately, `line6d_checked` has no TY2024 AcroForm cell to write to, so the TY2024 path cannot be built as
specified at all.

**What I would change.**
1. Retract the "byte-identical" claim in S-7 and OQ-2; replace it with the diff above and state that only the
   four §86 thresholds are year-invariant (that part of S-7 does verify: `i1040gi--2024.txt:3391,3394,3424`
   and `i1040gi--2025.txt:3421,3424,3453` print the identical $32,000 / $25,000 / $12,000 / $9,000).
2. Year-key the 6c/6d rows in §6, and add a TY2024-only annotation leaf for the "D" (a character in a cell,
   not a bool), with its own mutation row: *omit the D on a TY2024 MFS-lived-apart return; the emitted-PDF
   read-back must show the 6a cell carrying `D`*.
3. Verify the TY2024 AcroForm can carry a character next to line 6a and record the field name from
   `xtask dump-fields` — do not assume it.
4. Make §5.1 a per-year transcription for ws3 and ws9 (the doc comment must match the year's own text), and
   restate §10's conformance KAT as **derived per year from the extract** — "Lines 4a–6d must each be
   accounted for" is a hand-typed range that over-enumerates TY2024 by one line, which is the exact defect
   `CLAUDE.md`'s *Blank is the normal case* forbids (`1..=38` against a 48-label set).
5. Put the math-error-notice sentence into R-8's wording — see m-3.

---

## C-2. A retiree's WITHHOLDING is dropped. SSA-1099 box 6 and RRB-1099 box 10 are absent from the input surface, and the 1099-R box 4 → line 25b route exists only in a doc comment with no in-scope line, no test and no mutation.

**Sections:** §7 (`FormSsa1099`, `Form1099R::box4_fed_withheld`), §4.1, §6, §10.

**The primary source names the boxes.** `i1040gi--2025.txt:4225-4242`:

> *"**Line 25b—Form(s) 1099.** Include on line 25b any federal income tax withheld on your Form(s) 1099-R.
> The amount withheld should be shown in box 4. … If you received a 2025 Form 1099 showing federal income tax
> withheld on dividends, taxable or tax-exempt interest income, unemployment compensation, **social security
> benefits, railroad retirement benefits**, or other income you received, include the amount withheld in the
> total on line 25b. This should be shown in box 4 of Form 1099, **box 6, of Form SSA-1099, or box 10 of Form
> RRB-1099**."*

`FormSsa1099` in §7 declares `box3_benefits_paid`, `box4_benefits_repaid`, `box5_net_benefits` — and
**nothing for box 6**. Voluntary federal withholding on benefits is a routine election for a retiree with
other income; it is the most common way a Social Security recipient has tax withheld at all.

At `main`, `AbsoluteReturn::withholding_25b` is documented *"1040 **L25b** — federal income tax withheld from
Form(s) 1099 (Σ box 4, across INT/DIV/G)"*. That is a **typed list beside a set that grows** — `CLAUDE.md`'s
highest-yield rule — and this spec grows the set by two document families (1099-R, SSA-1099/RRB-1099) while
touching neither the list nor any test of it. §4.1's in-scope table stops at lines 4a/4b/5a/5b/6a/6b/9; §6's
transcription table has no 25b row; §10's mutation table has no 25b row. By the spec's own rule — *"A
guarantee without one does not exist"* — there is no guarantee that any retirement withholding reaches the
return.

**Direction of error:** payments understated ⇒ lines 25d, 33 and 34/37 wrong ⇒ **balance due overstated /
refund understated**. A wrong bottom line on a signed return.

**What I would change.** Add `box6_fed_withheld: Usd` to `FormSsa1099` (and state RRB-1099's box 10 — one
struct covering two forms needs both box names, or two structs). Put line **25b** in §4.1 and §6 with its new
operands. Add mutation rows: *drop the 1099-R box-4 term from 25b; a household with pension withholding must
stop reconciling*, and the same for SSA-1099 box 6. And restate `withholding_25b`'s own doc comment as a
derived sum over the document families rather than the four-letter list, so the next information return cannot
be added silently.

---

## C-3. §10's promised two-witness sweep on the MFS-lived-with branch has ZERO witnesses. Both oracles treat every MFS filer as lived-apart, and they AGREE while doing it — so a green sweep there is a false PASS, and reconciling to it drives btctax to the understating answer.

**Section:** §10 (*"6b gets a two-witness sweep across all five filing statuses, **including the MFS
lived-apart / lived-with pair**"*).

**Tax-Calculator 6.8.2, `taxcalc.calcfunctions.SSBenefits` docstring, read at source:**

> *"`SS_thd1[2]`/`SS_thd2[2]` ($25,000/$34,000) encode only the "MFS lived apart all year" case.
> Tax-Calculator records do not carry a lived-apart-all-year flag …, so the code **unconditionally treats
> every MARS=3 filer as lived-apart and under-taxes MFS-lived-with-spouse filers. Not fixable code-side.**"*

**OpenTaxSolver 2024, `taxsolve_US_1040_2024.c:1568-1583`:**

    if (status == MARRIED_FILING_JOINTLY) ws[8] = 32000.0; else ws[8] = 25000.0;
    ...
    if (status == MARRIED_FILING_JOINTLY) ws[10] = 12000.0; else ws[10] = 9000.0;

No lived-with branch exists. Neither engine implements the *skip lines 8 through 15* jump at all.

So the branch with the **largest money swing in this whole feature** — the one T-1 calls a trap, M-5 exists to
pin, and R-8 exists to refuse rather than guess — has **no independent witness**, and the two engines produce
the *same wrong answer*, which is `CLAUDE.md`'s *"★ Two disqualified oracles can align"* landing on a new
form. §10 asserts the opposite as fact.

**Why Critical rather than Important.** This is a defect in what an instrument *claims to have done*, which
the severity rule keeps blocking. Worse, it is actively dangerous: if the builder runs the sweep and
reconciles to it — the normal thing to do with a red sweep — btctax will adopt the lived-apart thresholds for
a lived-with MFS filer, which understates tax by up to 85% of benefits from the first dollar. §10 as written
points the build at that outcome.

**And the corpus cannot even run it yet.** `scripts/oracle/corpus.py` at `main` has **zero** retirement keys —
no social-security, pension, IRA-distribution or `e02400` input (measured: a case-insensitive count of
`social|pension|ssa|1099r|e02400` returns 0; the full input-key list is w2 / interest / dividends / capital
gains / SE / HSA / charitable / SALT / state-income-tax / real-estate-tax / mortgage). Day-one two-oracle
coverage of 4b/5b/6b is not thin, it is nil.

**What I would change.** §10 must say, in the spec:
1. The corpus generator gains retirement inputs before any of this is testable, with a stated household set
   covering all five statuses.
2. The MFS-lived-with vector is **disqualified on both oracles, computed from the mechanism** — the absence of
   a lived-apart flag in taxcalc's records and the absent branch in OTS's `ws[8]`/`ws[10]` — not listed by
   vector name, per the `verify_f6251.py` lesson. Name the *size* of each omission so a divergence of the
   wrong shape is still unexpected.
3. A **witness census** for 6b that fails the run if any filing status has no two-oracle vector — the
   mechanism already written for AMT in `CLAUDE.md`.
4. The MFS-lived-with jump is therefore held by a **hand-worked KAT from the worksheet itself**, and that KAT
   is the only thing standing behind it. Say so.

---

# IMPORTANT

## I-1. OQ-2's newest claim is FALSE: both engines COMPUTE line 6b, they do not consume it. §G-9 does not apply here, and the conclusion drawn from it inverts the right test strategy.

**Section:** OQ-2, the 2026-09-15 amendment — and the review brief's framing, which rests on it.

The amendment states: *"both engines take Form 1040 line 6b as an INPUT, so a green two-oracle sweep on the
Social Security worksheet proves nothing"*, and concludes the owner's real return *"**is** the validation"*.

Both halves of the premise are wrong, verified at source:

- **taxcalc**: `e02400` (gross OASDI benefits, worksheet line 1) is the input; `c02500` (*"Taxable OASDI
  benefits (Form 1040 line 6b)"*) is **computed** by `SSBenefits`, which reimplements the worksheet in three
  branches and documents the equivalence.
- **OTS**: `GetLineF("L6a", &L6a)` at `taxsolve_US_1040_2024.c:1975` is the input; `SocSec_Worksheet()` at
  `:2181` — *"Calculates L6b, which is L[6]"* — transcribes all eighteen worksheet lines.

So the two-oracle sweep on 6b is a **genuine three-way independent check** of the worksheet, and it is the
strongest instrument this feature can have short of a filed return. §G-9's limit applies to 4b and 5b — which
§10 already says correctly — and **not** to 6b.

This matters because the amendment's conclusion weakens §10 in the wrong direction: it argues the sweep proves
nothing and the real return is the validation, when the truth is that the sweep is real, must be built out
(C-3), and the real return is the check on the *collected* figures (4b, 5b, 6a, the box-6 withholding) that
the oracles genuinely cannot see. Keep the real-return requirement — it is right — but for the right reason,
and do not let it displace the sweep.

**What I would change.** Rewrite OQ-2's second paragraph: both engines compute 6b, so the sweep validates the
worksheet; §G-9 binds 4b/5b/6a and the withholding; the real return is what validates *those*, plus the
printed layout of 4a/5a/6d and the year-shaped cells in C-1.

## I-2. "Line 4a is structurally never populated" rests on one sentence and is contradicted by two others in the same section — and M-3 would pin the un-adjudicated reading with a KAT.

**Sections:** S-2, §4.1, §6, M-3.

S-2 quotes `i1040gi--2025.txt:2664-2667` (*"If the distribution from your IRA is fully taxable, enter the total
distribution on line 4b; don't make an entry on line 4a"*) and concludes **4a is structurally never populated
in v1**, with provenance *"the form instructs a blank here"*.

The same 4a/4b instructions say, unconditionally, in their closing paragraph
(`i1040gi--2025.txt:2789-2795`; TY2024 `:2726-2732`, identical):

> *"**More than one distribution.** If you (or your spouse if filing jointly) received more than one
> distribution, figure the taxable amount of each distribution and enter the total of the taxable amounts on
> line 4b. **Enter the total amount of those distributions on line 4a.**"*

§4.1 defines 4b as *"Σ Form 1099-R box 1 over IRA-flagged documents"* — so multiple IRA distributions are
squarely in scope, and this is the paragraph that governs them. Note the spec's own cited IRA range is
`2664-2790`; the paragraph begins at 2789 and was read past.

**The asymmetry with the pension side is the tell, and it is deliberate on the IRS's part.** The pension
analogue is *scoped*: `i1040gi--2025.txt:2978-2980` reads *"More than one pension or annuity. If you had more
than one **partially taxable** pension or annuity…"*. The IRA version carries no such limb.

**A second contradiction:** Exception 2's sub-branch (b) — `i1040gi--2025.txt:2698-2700` plus `:2707-2715` —
instructs *"enter the total distribution on line 4a"* and *"enter -0- on line 4b"* for a Roth IRA distribution
with **distribution code Q in box 7**. That is a populated 4a on a branch the instructions complete in one
sentence (see I-3).

**Why Important.** M-3 pins the claim with a mutation (*"change `line4a: Option<Usd>` to `Usd` / emit
`Usd::ZERO`; the read-back must show the 4a cell present"*), so if the reading is wrong the spec ships a defect
**with a test holding it**, and the filed 4a disagrees with the 1099-R box 1 totals the Service
document-matches against — the identical shape as the line 1a finding already recorded in `printed.rs` (*"Its
absence left the filed 1z sitting above an EMPTY operand column … on the very line the Service document-matches
against your W-2s"*).

**What I would change.** Adjudicate it in the spec against the form, in writing: either 4a = Σ box 1 when there
is more than one IRA document (and M-3 becomes *"a single-document return leaves 4a blank; a two-document
return populates it"*), or state the counter-reading with the authority for it. Do not leave "structurally
never populated" standing on the first sentence alone.

## I-3. A code-Q Roth IRA distribution is REFUSED (R-2) although the instructions finish it in one sentence. S-3's collapse of four Exceptions into one yes/no destroys the sub-branch that computes — and box 7, which the instructions route on, is deliberately not read.

**Sections:** S-3, R-1, R-2, §7 (`box7_distribution_codes`), §4.2.

`i1040gi--2025.txt:2707-2715`:

> *"2. You received a distribution from a Roth IRA. **But if either (a) or (b) below applies, enter -0- on
> line 4b; you don't have to see Form 8606 or its instructions.** a. Distribution code T is shown in box 7 of
> Form 1099-R and you made a contribution (including a conversion) to a Roth IRA for 2020 or an earlier year.
> b. Distribution code Q is shown in box 7 of Form 1099-R."*

Under the spec, that filer answers `exception_applies = Some(true)` (a Roth distribution is Exception 2) and
hits **R-2: refused**. But the return is trivially completable: 4a = total distribution, 4b = `-0-`, no Form
8606, no attachment, no election, no carryforward. Code Q is what a payer stamps on **every** qualified Roth
IRA distribution — the ordinary case for any retiree over 59½ with a five-year-old Roth. The spec turns that
filer away and the manual answers them in two lines. This is `the-answer-is-in-the-manual` exactly.

**The mechanism is S-3.** Collapsing four Exceptions into one boolean is defensible for the three that route to
genuine machinery; it is not defensible for the one sub-branch the instructions themselves close. And §7
compounds it: `box7_distribution_codes` is *"captured verbatim; **screened, never interpreted**"* — while the
instructions' own decision procedure **is** a box-7 test. The field that decides is held and not read.

**What I would change.** Either (a) split the Roth sub-branch out: a document whose box 7 contains `Q` (or `T`
with the pre-2021-contribution condition asked) computes as 4a = Σ box 1, 4b = `-0-`, with a KAT per
sub-branch; or (b) if it stays refused, say so **explicitly in §4.2 with the instruction quoted**, so the
refusal is a recorded decision rather than a consequence of a collapse nobody re-read. Option (a) is the one
the instructions support. Either way, delete *"screened, never interpreted"* from box 7 — the form interprets
it.

## I-4. M-1 is now UNKILLABLE. FR-184 landed a cardinality pigeonhole that explicitly does not bind a row to an occurrence, so the 4b/6b label swap M-1 names still passes — and S-9's prescribed fix was measured and refuted.

**Sections:** S-9, §6's closing note, M-1, R-A.

FR-184 is **closed** at `main` (commit `dd8f2e984`, 2026-09-15). Its own entry (`FOLLOWUPS.md:8308-8312` at
`main`) states the limit:

> *"A shared quote must be backed by at least as many printed occurrences as there are lines claiming it. **It
> does not pin which row owns which occurrence — no form-general discriminator exists**, which is what the
> three refuted attempts established."*

Apply that to M-1's mutation — *"swap the `line` labels on the 4b and 6b coverage rows, leaving both quotes as
`Taxable amount`"*. After the swap the distinct-line set is still `{4b, 5b, 6b}` = 3 and *"Taxable amount"* is
still printed 3 times, so the pigeonhole is satisfied and rule (2b) still admits each row individually. **The
swap still passes.** The fix commit says as much in its own words: *"A table row saying `f1040:4b` passes, and
would pass IDENTICALLY if it said `6b`."*

And S-9's prescribed remedy — *"anchor the match to the **physical extract row** whose first token is the stem
label"* — is one of the three candidates FR-184 measured and refuted (window / nearest-label / trailing
amount-box).

**Why Important.** The spec declares this a **prerequisite of the feature, not a follow-up**, on the grounds
that without it *"the three most important new rows in the census are unverified"*. FR-184 bought a real but
weaker guarantee: the three rows may coexist *because the form prints the words three times*. Row-to-line
misattribution is still accepted. If the build reads "FR-184 closed" as discharging M-1, the census ships green
over an unverified transcription — R-A, unchanged.

**What I would change.** Rewrite S-9 and M-1 to the guarantee that actually exists, and add the kill that can
red:
- M-1 becomes: *give the 4b/5b/6b rows a quote the form prints only twice; the pigeonhole must red.* (That is
  the plant FR-184 already ships — *"two lines claiming "Adjustments to income from Schedule 1, line 26",
  printed once"*.)
- Add M-1b for the residue the pigeonhole does **not** cover: a 4b/6b label swap is still accepted, so the
  row-to-line binding must be held by something else. The cheapest candidate that is not refuted:
  `label_reader.rs` already classifies each physical row, so bind the coverage row's `line` to the label token
  the reader found on that row. If that is out of scope for v1, say in the spec that the three rows' mutual
  attribution is **not** machine-held, and name the KAT that stands in.
- Delete the "must be strengthened before these rows land" framing and replace it with what landed.

## I-5. The §9 rewrite added two questions and left §7, §10 and §12 saying there are four. The two new questions have no field, no live condition, no `None` behaviour, and no refusal or advisory rule.

**Sections:** §9 (2026-09-15 rewrite) against §7, §10, §12 R-E.

§9 now rules **A-1 = ASK** (*"are you a retired public safety officer?"*) and **A-2 = ASK** (*"did the benefits
include a payment for an earlier year?"*). §7 still says, with two stars:

> *"★★ **No question is added for the PSO exclusion, the lump-sum election, or the §11 SSDI carve-out.** Under
> S-1 each can only lower the figure, so each gets an advisory instead (§9)"*

§12 R-E still says *"**Four** new mandatory questions"*, and §10's journey walk still asks *"Where does each of
the **four** declarations appear"*. So the input surface — the section a builder implements from — does not
contain two of the six questions the spec now mandates, and the contradiction is direct, in the same document,
introduced by the newest edit. This is the *"edited §X, forgot §Y"* class the project's own delivery doctrine
flags.

**And the new questions' semantics are undefined in a way that matters.** Under S-1 both can only lower the
figure, so neither may refuse when unanswered — yet §7's table shape is "`None` ⇒ R-n", and every
`Option<bool>` on `ReturnInputs` is a class-(A) declaration the classifier forbids `_` on. The spec must say:
are these class-(A) declarations (in which case an unanswered one blocks, contradicting S-1) or advisory-only
leaves (in which case they need a different home and a stated `None` = silent rule)? Silence here is how a
mandatory question appears by accident.

**What I would change.** Add both to §7's table with their `live` conditions (`Σ pension box 2a > 0` and
`Σ SSA-1099 box 5 > 0`), state explicitly that `None` is **silent, never a refusal** with S-1 as the reason,
and give each a mutation row (*answer yes; the advisory must appear / answer no; it must not*). Fix "four" to
"six" in §7, §10 and R-E.

## I-6. A-4's premise does not hold. In v1, line 5b ALWAYS comes from box 2a, so A-4's firing condition is identical to A-1's — it fires on 100% of in-scope pension returns and fails the very test §9 sets. And its claim about box 2a overstates the source.

**Section:** §9 (the A-4 verdict, flagged by the controller as *"is A-4's framing right, or did I misread
it?"*). **Answer: it is misread, and the fix is available.**

§9's table says A-4 is *"Unlike A-1…A-3 … a condition that **is** about the filer: box 2a was taken"*.

But §4.1 defines 5b as *"Σ Form 1099-R box 2a (pension-flagged)"* — box 2a is the **only** source of 5b in v1 —
and R-5 refuses whenever box 2a is absent or box 2b is checked. So in the admitted population:

    A-4 fires  <=>  5b came from box 2a and box 2a > 0  <=>  5b > 0  <=>  A-1 fires

They are the same condition. A-4 is exactly as blanket as A-1, and §9's own mechanism — *"An advisory must be
keyed on whether it applies to THIS filer"* — rejects it on identical grounds.

**And the source claim is stronger than the instruction.** `i1040gi--2025.txt:2902-2906` says *"If your Form
1099-R shows a taxable amount, you can report that amount on line 5b. But you **may be able to** report a lower
taxable amount by using the General Rule or the Simplified Method…"*. It does not say box 2a is the higher of
two lawful figures. The instruction's own definition of fully taxable (`i1040gi--2025.txt:2870-2876`) is *"(a)
you didn't contribute to the cost … or (b) you got your entire cost back tax free before 2025"* — and when
box 2a equals box 1, that is ordinarily what the payer is asserting, in which case **there is no cost left to
recover and no lower figure exists**. A-4 as specified fires hardest in the case where it is least likely to
apply.

**What I would change.** The filer-specific key is the existence of unrecovered cost in the plan, which is one
question (*"did you contribute to the cost of this pension, and have you not yet recovered it all tax free?"*)
— and note the payer already answers half of it: box 2a **< box 1** means the payer applied a method, box 2a
**== box 1** means it did not. So A-4's honest verdict is **ASK, keyed on box 2a == box 1** (or drop it), not
*"keep A-4 as the single advisory a box-2a pension earns"*. Either way the §9 table's claim that A-4 is unlike
A-1…A-3 must go.

## I-7. §1's newest paragraph claims "24 line-number citations remain and were VERIFIED accurate at HEAD (2026-09-15)". At `main`, at least 13 of the 26 are wrong — three of them inside the 2026-09-15 amendments.

**Section:** §1 (the 2026-09-15 CITATION CONVENTION amendment), and the citations throughout.

Measured at revision `main` for every `file:line` citation in the spec. The claim is the exact shape
`CLAUDE.md` calls an instrument reporting something other than what it measured, in the paragraph whose subject
is citation drift.

| spec citation | what the spec says it is | what is actually there / where it really is |
|---|---|---|
| `return_1040.rs:1697-1698` | the `total_income` expression (§2) | a doc table; the expression is at **2451-2452** |
| `line_coverage.rs:2265-2272` | the line-9 coverage row (§2) | the line-15 excess-SS row; line 9's row is at **3110-3116** |
| `questions.rs:548-556` | the `OtherOutOfScopeIncome` prompt (§2) | `spouse_63f_status_permits`; the prompt is at **973ff** |
| `questions.rs:583-584` | *"a filer cannot answer `no`…"* (§2) | a carryforward doc comment; the sentence is at **1060** |
| `questions.rs:585-587` | the safe-widening precedent (§2) | `carryforward_in_present` — **the spec itself says so at OQ-1 and left §2 citing it** |
| `return_refuse.rs:203-213` | the `SingleEmployerExcessSs` retraction (S-1) | a box-12 doc comment; the variant is at **1204** |
| `printed.rs:311-340` | `ScheduleALines.line2: Option<Usd>` (S-2) | `printed_8275`; `line2: Option<Usd>` is at **447** |
| `return_refuse.rs:1273-1278` | where `IraDeductionClaimed` fires (S-8) | a direct-deposit doc comment; variant **1433**, fires **5537** |
| **`return_1040.rs:2425-2428`** | the BLOCK comment (**S-5, 2026-09-15**) | Form 8889 wiring; the comment is at **2455-2468** |
| **`return_1040.rs:2442`** | the `adjustments` expression (**S-5, 2026-09-15**) | `+ schedule_c_net`; the expression is at **2481** |
| `classifier.rs:208` | `c.declaration(other_out_of_scope_income, …)` (§7) | `foreign_trust`; the real one is at **273** |
| `line_coverage.rs:462-468` | Form 6251 line 1 as the per-row-`year` precedent (S-10) | a 2a/2b `c.line` with no year argument; the mechanism is `Coverage::quoting` / `quoting_year` at **270 / 289-298** |
| **`f1040.map.toml:1-10`** | *"a stub carrying only line 7a"* (**OQ-2, 2026-09-15**) | lines 1-10 are comments plus `form`/`year`/`irs_stem`/`versioning`; `line7a` is at **16** |

Accurate at `main`, recorded so the next reader does not re-check them: `printed.rs:584-592`,
`advisories.rs:43`, `advisories.rs:44-68`, `return_inputs.rs:48-53`, `form1040_full.rs:263-317`,
`FOLLOWUPS.md:1747`, `label_reader.rs:33-42`, `line_coverage_check.rs:299-306`,
`line_coverage_check.rs:610-636`. Marginal (right neighbourhood, off by one or two): `registries.rs:210-211`,
`return_inputs.rs:963-966`, `line_coverage.rs:58-95`. Also wrong: `registries.rs:388` (*"the `QuestionId`
mapping"* — it is `Ok(())` inside a `decl_tristate!`).

**Note the symmetry.** `FOLLOWUPS.md`'s own FR-183 entry cites `return_1040.rs:2312`, `:2338-2343` and `:2349`
for the same three things the spec cites as `:2442` and `:2425-2428` — two different wrong answers, neither
matching `main`'s 2481 / 2455-2468 / 2469. The drift class the amendment set out to end is reproducing inside
the amendment.

**What I would change.** Retract the "24 verified accurate" sentence — it is the load-bearing claim and it did
not hold. Then apply §1's own standing instruction to all thirteen: **convert to file+symbol**, do not
renumber. The four-line check §1 describes should be committed as a script (`CLAUDE.md`: *"commit the extractor
as a script so the check is a command rather than a discipline"*) and run over this file.

## I-8. OQ-2's map-row table does not reproduce. The real gap is 33 money-line rows, not ten — a 3.3x understatement of the both-years work, in the amendment that sizes it for the owner.

**Section:** OQ-2, the 2026-09-15 amendment.

The spec's table claims `forms/2024/f1040.map.toml` = **14** map rows and `forms/2025/f1040.map.toml` = **4**,
concluding *"TY2025's map is a stub missing **ten** rows relative to TY2024's"*.

Measured at revision `main`, counting top-level keys matching `^line[0-9]`: **35** for TY2024 and **1** for
TY2025. One of the 35 is `line_set` (metadata), so **34 money-line cells for TY2024 and 1 (`line7a`) for
TY2025**. The 2025 file additionally carries `da_yes`/`da_no` and a generated four-column dependents grid.
Under the other definition I tried (all top-level `key =`) the numbers are 134 and 35. Neither 14 nor 4
reproduces under any definition I could construct.

**Why Important.** This is the number the owner's both-years ruling is being sized against, and the error runs
in the direction that makes the work look smaller. Building TY2025's 1040 map means pinning **33** absent cells
with `xtask dump-fields` and the read-back verifier before 4a–6b are even reached — which plausibly changes
whether TY2025 belongs in v1 or in the increment after it. That is an owner-level consequence, not a typo.

**What I would change.** Replace the table with the measured numbers and the command that produced them, and
say plainly that the TY2025 map is a capital-gains-only stub, not a 10-row shortfall. Then let the owner
re-confirm "both years" against the real figure.

## I-9. S-5's block rule names the wrong block. The worksheet's block is "11 through 20, **and 23 and 25**"; the derivation S-5 prescribes is "11 through 20" — and the worksheet's own first precondition is about a Schedule 1 line 24z write-in reaching it.

**Section:** S-5, the 2026-09-15 FR-183 correction.

S-5 quotes the sentence correctly — *"Schedule 1, lines 11 through 20, and 23 and 25"*
(`i1040gi--2025.txt:3403`) — then narrows to *"The block "11 through 20" **contains line 13**"*, concludes
*"worksheet line 6 is `printed Sch 1 L13 + L15 + L18`"*, and imports the code comment whose rule reads *"a
future Schedule 1 **lines 11–20** adjustment belongs here the day it is added"*.

The block has a second clause, and the worksheet's own *Before you begin* bullet 1 is specifically about it
(`i1040gi--2025.txt:3389-3390`):

> *"If the instructions for Schedule 1, line 24z, have you enter a write-in adjustment on line 24z, figure that
> write-in before completing this worksheet."*

Line 24z reaches worksheet line 6 through **line 25** (*"Total other adjustments. Add lines 24a through 24z"*).
The IRS puts that warning first in the worksheet because the clause is live.

**An independent witness agrees with the form, not the spec.** OTS implements exactly the printed block
(`taxsolve_US_1040_2024.c:1549-1551`):

    for (k = 11; k <= 20; k++) ws[6] = ws[6] + Sched1[k];
    ws[6] = ws[6] + Sched1[23] + Sched1[25];

**Currently latent, and that is the point.** At `main`, `Schedule1Lines` carries exactly four adjustment
fields — `line13`, `line15`, `line18`, `line21` (`printed.rs:584-591`, verified) — and none of 23/24z/25, so
`{13, 15, 18}` is the right *answer today*. But S-5 exists to state the *rule that survives growth*, and as
written it will drop the first 24z write-in or Archer MSA deduction btctax ever models, in the direction that
**overstates the filer's tax** — which is the exact defect FR-183 was filed about, one clause over. A list that
is correct on the day it is written, beside a set that grows: `CLAUDE.md`'s highest-yield rule, in the section
written to obey it.

**What I would change.** State the block as the worksheet states it — **11 through 20, and 23 and 25** — in
S-5, in the T14.3 derivation, and in the M-6b kill. Extend the drop-a-member mutation to a member of the second
clause so the rule is held in both clauses. And carry the same correction back to the code comment at
`return_1040.rs:2467`, whose `11–20` phrasing is where the narrowing came from — the §221 MAGI worksheet quotes
the identical sentence (`return_1040.rs:2356-2357`, verified), so the two blocks are the same block and both are
short by one clause.

## I-10. The whole input surface has NO archived primary source. §1's authority table lists only `f1040` and `i1040gi`; there is no 1099-R, SSA-1099 or RRB-1099 extract in the repo — so the box numbers in §7 are asserted from memory and sit outside cite-check.

**Sections:** §1 (sourcing of record), §7.

`design/forms/extract/` contains no file matching `ssa`, `rrb` or `1099r`. §7 transcribes boxes of three
information returns: 1099-R box 1 / 2a / 2b (two checkboxes) / 4 / 7, and SSA-1099 box 3 / 4 / 5. §1's authority
table covers neither, and the spec's own §1 rule is *"Everything below is quoted from the extracted text layer,
never from a rendered page"*. The i1040gi does describe SSA-1099 boxes 3 and 4 (`:3236-3239`) and box 5 via
worksheet line 1 (`:3396`) — so those three are sourced — but the 1099-R box numbering is not sourced at all,
and the *complete* box list of either form is nowhere.

**This is the root cause of C-2, which is the proof it matters.** SSA-1099 box 6 is precisely the box an
archived extract would have surfaced, and the spec asserts a three-box SSA-1099 with no source to check it
against. The repo's standing rule is that a transcription is graded against a line set **derived from the
form**, never a hand-written list — and §7 is a hand-written list.

**What I would change.** Archive `f1099r`, `i1099r` and the SSA-1099 / RRB-1099 box lists into
`design/forms/extract/`, add them to §1's authority table with hashes, and bring §7's box doc comments under
`cite-check` like every other transcription in this repo. Then re-derive §7's box set from the extract and see
what else is missing — C-2 was found by reading the 1040's line 25b instruction, not by auditing §7, so there
has been no systematic pass over these boxes at all.

## I-11. `Usd` cannot express a negative SSA-1099 box 5, so the R-6 population cannot enter what their form says.

**Sections:** §7 (`FormSsa1099.box5_net_benefits: Usd`), R-6.

R-6 fires when Σ box 4 > Σ box 3 — the case the instructions call *"None of your benefits are taxable"*
(`i1040gi--2025.txt:3251-3259`, and `i1040gi--2024.txt:3138-3150` verbatim for TY2024). On that form box 5 is
negative, printed in parentheses. `box5_net_benefits: Usd` cannot hold it; `Usd` is the type `FOLLOWUPS.md`
§G-11 records as unable even to express blank.

The refusal still fires (it reads box 3 and box 4), so this is not an escape hatch — but the filer is asked to
type a figure their document does not contain, and whatever they type becomes worksheet line 1 and 6a on any
path where R-6 does not fire. The spec should say what they enter and what `line6a` prints. Related: collecting
box 3, 4 and 5 as three independent leaves permits a triple that contradicts itself (box 5 is box 3 minus box 4
on the form) with nothing in the spec to catch it — derive box 5 or cross-check it, and note that I could not
source that relationship from `i1040gi`, so verify it against the SSA-1099 itself once I-10's archive exists.

---

# MINOR

**m-1. §5's reason 3 is stale against S-5's own correction.** It still says worksheet line 6 *"reads Schedule 1
adjustments that in v1 are ½-SE and the early-withdrawal penalty"* and gives the acyclicity chain as *"4b, 5b →
Sch 1 L15/L18 → worksheet"*. S-5 added L13 (HSA). The acyclicity conclusion survives — the Form 8889 line 13
deduction does not depend on benefits — but say so, rather than leaving the section that proves acyclicity
quoting a two-term operand set the same document corrected to three.

**m-2. R-8's wording overstates the lived-with penalty.** *"living together for any part of it skips straight to
85% of your benefits from the first dollar"* is not what the branch computes: `ws16 = 0.85 × ws7` and
`ws18 = min(ws16, 0.85 × ws1)`. A lived-with MFS filer with **only** benefits gets `0.85 × 0.5 × benefits` =
**42.5%** of benefits, not 85%. Accurate and still sharp: *"taxable from the first dollar, with no threshold at
all — up to 85% of your benefits."*

**m-3. §7 quotes the 6d instruction and drops its consequence.** `i1040gi--2025.txt:3332-3334` continues: *"If
you don't check the box on line 6d, you may get a math error notice from the IRS."* That sentence is the
strongest thing available for R-8's message and for the TY2024 "D" (C-1), and the spec cites only `3330-3332`.

**m-4. The worksheet's rounding convention is unstated.** ws13 halves ws12 and can land on $X.50; ws15 and ws17
multiply by 0.85. OTS applies `Conditional_Round` at exactly ws13, ws15 and ws17
(`taxsolve_US_1040_2024.c:1586-1590`). The spec inherits `SPEC.md`'s D-1…D-11 *"unless restated"* and does not
restate — say which convention governs, because a $1 difference here will read as an oracle disagreement in the
C-3 sweep.

**m-5. `box2b_total_distribution` has no reader.** It is collected and never consumed. Note the instructions
treat a plain total distribution as ordinary 5a/5b reporting (`i1040gi--2025.txt:3196-3204`: *"Enter the total
distribution on line 5a and the taxable part on line 5b"*) — Form 4972 is only the *election*, which §7's
question correctly scopes. So box 2b's `total_distribution` half may genuinely have no consumer; if so, say
that, with the reason. A figure with no reader is not thereby correct.

**m-6. RRB-1099-R has no representation.** `i1040gi--2025.txt:2883-2886`: *"If you received a Form RRB-1099-R,
see Pub. 575 to find out how to report your benefits."* That form has no box 1 and no box 2a, so `Form1099R`
cannot hold it, and the pension exception list never names it — so a railroad retiree's tier-2 annuity is caught
only by the widened scope attestation, which the spec itself now concedes is a **priming** protection rather
than a refusal (OQ-1). Add it to the pension YES-conditions, or to §4.2 with its own refusal.

**m-7. An IRA document with box 2a < box 1 and `exception_applies == Some(false)` is self-contradictory.** A
payer reduces box 2a on an IRA essentially only under Exception 2/3/4, all of which refuse. The spec computes
4b = Σ box 1 and says nothing. The direction is conservative so it does not gate, but the contradiction is free
to detect and worth a refusal or an advisory — the filer has almost certainly answered the exception question
wrong.

**m-8. R-7's sibling refusal is named but not specified.** `SocialSecurityWorksheetBarUnanswered` appears inside
R-7's prose with no R-number, no wording, no firing condition and no mutation row. And the
`has_income_exclusion == None` case is not stated — presumably refused upstream, but S-8 sets the precedent that
a guarantee held in another module *"lands with a KAT"*.

**m-9. §5 miscounts the Simplified Method's carryforwards.** *"it needs no prior-year carryforward — unlike the
Simplified Method Worksheet, which needs four (§4.2)"*. It needs **two**: line 4 via the Note at
`i1040gi--2025.txt:3000-3002` (*"skip line 3 and enter the amount from line 4 of last year's worksheet"*) and
line 10 into next year's line 6 at `:3013-3014`. The "four" in §4.2 counts *inputs*, not carryforwards.

**m-10. FR-253's suppression needs one check first.** Silencing `CtcOdcOmitted { provably_zero: true }` is safe
only if `ctc_provably_zero` proves the credit worthless including the **refundable ACTC**, not just the §24(b)
non-refundable portion. The variant and message quoted in §9 verify at `main` (`advisories.rs:43`, `:444-450`,
and the count of 27 variants is accurate), and the two branches are each correct about what prints
(`provably_zero` gives `Some(ZERO)` and prints `0`; otherwise the cell is blank). Just confirm the proof's scope
before making it silent.

---

# NIT

**n-1. S-9's stated mechanism is slightly off.** A correctly-labelled row matches at its *own* occurrence too,
so the reason a swap passes is not that `4b` *"matches at line 5b's position"* but that all three quotes are the
identical string and each label finds a stem token nearby. The conclusion is right and FR-184 reached the same
place; only the sentence is imprecise.

**n-2. The brief attributes the corpus gap to the wrong follow-up.** FR-250 at `main` (`FOLLOWUPS.md:9549`) is
about **charitable** coverage in the 107-household corpus, not retirement. The underlying fact is worse than
stated and verified independently in C-3: the corpus has *zero* retirement inputs of any kind. Worth its own
entry.

---

# What is RIGHT, so the next round does not re-derive it

- **The worksheet transcription (§5.1) is accurate, all 18 lines**, checked line by line against
  `i1040gi--2025.txt:3396-3463`. Every extract citation resolves, including the two split ones (ws7 at
  `3404-3408` plus `3416`, ws8 at `3420-3440` where the text layer shatters the third bullet into one word per
  line). The three *Before you begin* preconditions are at `3389-3394` and *"the second"* is indeed the 6d one.
  The TIP is at `3470-3471`. ws12's odd *"smaller of line 9 or line 10"* is the form's own text.
- **S-7's threshold claim verifies for the four figures.** `i1040gi--2024.txt:3391,3394,3424` and
  `i1040gi--2025.txt:3421,3424,3453` print identical $32,000 / $25,000 / $12,000 / $9,000. No year table. (The
  *worksheet* is not byte-identical — C-1 — but the thresholds are.)
- **S-6's asymmetry is real and correctly sourced.** The pension side says *"If your Form 1099-R shows a taxable
  amount, you can report that amount on line 5b"* (`:2902-2904`); the IRA side never mentions box 2a anywhere in
  `:2650-2795`. A shared box-2a helper would be wrong on the IRA side. R-C is a correct risk.
- **T-1 is right and its trap is real.** The `ws8 = 0` encoding does reproduce the jump's arithmetic, and only
  because ws10 is also zero on that branch — so M-5's insistence on asserting the **branch taken** rather than
  the dollar is correct and necessary. Both oracles' failure to model the jump (C-3) is why it needs a
  hand-worked KAT.
- **T-2 is right.** The form does carry both halves of §G-11 on one page: 4a blank by instruction, 6b written as
  `-0-` by the two STOPs. The `Exception`-with-a-reason production is the honest call.
- **S-8's coupling is real.** The worksheet's Exception 1 is at `i1040gi--2025.txt:3246-3250` and
  `i1040gi--2024.txt:3130-3137`, `RefuseReason::IraDeductionClaimed` exists at `main`
  (`return_refuse.rs:1433`, fires at `:5537`), and M-8 is the right shape.
- **§2's gap is real.** `Form1040Lines` at `main` has no 4a/4b/5a/5b/6a/6b/6d field. The line-9 coverage row does
  carry all eight operands under `Production::Combine` (`line_coverage.rs:3110-3116`). OQ-1's ruling **is**
  satisfied: the widened prompt at `questions.rs:974-980` opens with *"a PENSION, ANNUITY or IRA DISTRIBUTION
  (Form 1099-R), SOCIAL SECURITY or railroad retirement benefits (Form SSA-1099 or RRB-1099)"*, and OQ-1's own
  retraction to "priming, not silence" is the honest framing.
- **§9's core mechanism is sound and is the best thing in the amendments.** *"An advisory must be keyed on
  whether it applies to THIS filer, and when that cannot be known, the choice is to ASK or to say nothing —
  never to advise everyone"* is the right reading of the ruling, and the A-1/A-2/A-3 verdicts follow from it.
  Only A-4's exceptionalism is wrong (I-6). The shipped-code observation is real, the `Advisory` count of 27 is
  accurate, and FR-253 exists at `main` (`FOLLOWUPS.md:9637`).
- **R-1 through R-8 all fire on the state they name and all offer an exit.** The two exits used —
  `btctax income answer` for the unanswered pair, "file that distribution yourself" for the unsupported pair —
  are the right shapes, and R-8's refusal-over-either-default reasoning is correct: `Some(true)` is an
  understatement path and `Some(false)` overstates the majority. The one refusal that fires where it should
  compute is R-2 on the code-Q Roth branch (I-3).
