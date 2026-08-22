# Lens: $1M long-term bitcoin gain — Schedule D, Form 8949, and tax-computation routing

Repo read at `main @ f5ba41a1` unless noted. All citations are `file:line` from that tree.

---

## VERDICT — can this slice FILE today?

**REFUSES — conditionally, and for a mechanical reason that has nothing to do with the tax.** The
arithmetic in this slice is in good shape: §1222 netting, the §1(h) preferential split, the QDCGT
worksheet, the Schedule D Part III routing enum, the donation-is-a-removal separation and the 30%
capital-gain-property charitable ceiling are all implemented and all give the right answer for this
household. The Schedule D Tax Worksheet is **not** implemented, and — given the two guards that are in
place (`UnrecapturedOrSpecialRateGain` refuses §1250/§1202/28% gains, and btctax models no Form 4952)
— it is **not reachable**, so its absence is not a defect for this filer. What blocks the return is
**Form 8949 pagination**: the full-return filler refuses at more than 14 rows per part
(`fill8949.rs:167-180`, `f8949.map.toml:3`), and there is no continuation-page path. A $1,000,000
long-term bitcoin gain realized from more than 14 lots — the ordinary case for anyone who accumulated
over years — makes `fill_full_return` return `Err(Overflow { part: "Part II", .. })` and **the entire
packet fails to emit, every form, not just the 8949**. The pagination code that fixes this already
exists in this repo and is already tested; the full-return path simply does not call it.

The second, quieter problem is testimony rather than arithmetic: the full return writes **"Yes" on
Schedule D line 20**, which asserts *"…and you are not filing Form 4952"* — a fact the filer was never
asked, and one the repo has already ruled unanswerable from btctax's input surface. Today that answer
happens to be right for this vector and produces the correct tax; it is right by luck of the input
surface, not by construction.

---

## WHAT IS MISSING

### C-1. Form 8949 has no continuation pages on the full-return path — the packet fails to emit

**Form requires:** i8949 — every transaction gets its own row; more rows than fit go on additional
copies of the form ("If you have more short-term transactions than will fit on this page … use as many
Forms 8949 as you need"). Schedule D lines 3 and 10 are defined as the *totals* of those Forms 8949.

**btctax does:** `fill8949_full.rs:76 fill_8949_full_with_map` passes **all** rows straight to
`fill8949.rs:161 fill_8949_parts_inner`, which hard-errors above `map.rows_per_page`:

```
crates/btctax-forms/src/fill8949.rs:174
    if long.rows.len() > map.rows_per_page {
        return Err(FormsError::Overflow { part: "Part II", rows: long.rows.len(), capacity: map.rows_per_page });
```

`crates/btctax-forms/forms/2024/f8949.map.toml:3` → `rows_per_page = 14`. One row is emitted per
**`DisposalLeg`** (`crates/btctax-core/src/forms.rs:132`), not per disposal, and
`btctax_core::tax::printed::form_8949_printed` does no consolidation — so a single $1M sale that draws
on 20 lots is 20 rows.

**★ The fix already exists, nine files away, and the code comment claiming otherwise is false.** The
*crypto-slice* entry point paginates and has since it shipped:

```
crates/btctax-forms/src/lib.rs:90-112   fill_form_8949  →  chunks into n_pages, then overflow::merge_copies
```
with `crates/btctax-forms/tests/overflow.rs` proving it. But
`crates/btctax-forms/src/fill8949_full.rs:73-75` states *"more rows than a page holds REFUSES
(`FormsError::Overflow`) **exactly as the slice does** — the continuation-page pattern is a post-v1
item."* The slice does **not** refuse. This is the `CLAUDE.md` B3 pattern verbatim: a fix that exists
in the branch, never carried across, because no reviewer held both call sites at once.

**Consequence: the return cannot be filed at all** — not a wrong number, a **zero-byte packet**. And
because `packet.rs:154-160` propagates the error with `?`, the 1040, Schedule A, Schedule D, Form 6251
and every other form in the vector are lost with it.

**No test holds this.** `full_return_forms.rs:2914` asserts the packet refuses on a **Schedule B**
overflow; there is no equivalent for Form 8949. Per B1 the guard does not exist.

### C-2. Schedule D line 20 is answered "Yes" from a fact nobody collected

**Form requires:** Schedule D line 20 — *"Are lines 18 and 19 both zero or blank **and you are not
filing Form 4952**?"* Yes → Qualified Dividends and Capital Gain Tax Worksheet; No → **Schedule D Tax
Worksheet**.

**btctax does:** `crates/btctax-forms/src/schedule_d_full.rs:279-284` checks **Yes**, unconditionally,
on the `BothGains` branch. Lines 18 and 19 are correctly left blank (`:263-278`, with the reasoning
recorded) — that half is right and the form itself says a blank answers line 20 identically to a zero.
The **4952 conjunct is the part with no source.** `crates/btctax-forms/forms/2024/f1040sa.map.toml:100`
records Schedule A line 9 as `rule = "unmodeled"`, reason *"btctax models no Form 4952, so line 10
equals line 8e"*. There is **no input** for investment interest expense and **no refusal** when a filer
has it — the return simply files as though they do not.

The repo already adjudicated this correctly for the crypto slice (`CONTINUITY.md:290-296`, `README.md:236`,
`design/no-testimony/MAP-survey.md:317`): line 17 is answerable because it reads two lines printed on
the same page; line 20 is not. The **full return took the opposite decision on the same fact** and
nothing reconciles them.

**Consequence for *this* filer:** the answer is materially correct (a wage-earner + bitcoin household
with no margin borrowing is not filing Form 4952), and the tax is right. It is a **provenance defect,
not a dollar defect**: a printed "Yes" is sworn testimony under §6065 on a question never asked.
**For the adjacent filer it is a dollar defect in both directions** — the missing Schedule A line 9
**overstates** tax by dropping a deduction the filer had, while routing to QDCGT instead of the
Schedule D Tax Worksheet **understates** it, because SDTW line 4 subtracts Form 4952 line 4g (the
§163(d)(4)(B)(iii) election to treat net capital gain as investment income) from the preferential slice
before applying the 0/15/20% bands. Net direction is indeterminate, which is worse than a known bias.

### I-3. The QDCGT worksheet is a five-line closed form, and Form 6251 cites its lines by number

**Form requires:** the Qualified Dividends and Capital Gain Tax Worksheet (i1040, 25 numbered lines).
Form 6251 Part III then reads it *by line number*: line 13 = *"the amount from **line 4** of the
Qualified Dividends and Capital Gain Tax Worksheet"*; lines 20 and 27 = *"the amount from **line 5** of
the Qualified Dividends and Capital Gain Tax Worksheet … (as figured for the regular tax)."*

**btctax does:** `crates/btctax-core/src/tax/method.rs:74-91 qdcgt_line16` is a compression to five
statements (`pref_full`, `bottom`, `pref`, `l23`, `l24`) returning one scalar. The worksheet's lines 4
and 5 are computed inside it as `pref_full` and `bottom` and then **discarded**. Because nothing can
read them, Form 6251's citation is satisfied by a **second, independent derivation** at
`crates/btctax-core/src/tax/return_1040.rs:2061`:

```
qdcgt_line5_regular: (taxable_income - pref.min(taxable_income)).max(Usd::ZERO),
```

and a **third** in the 6251 fixture harness at `crates/btctax-core/src/tax/form6251.rs:727`. All three
are algebraically equal today — `(ti − min(p, ti)).max(0) ≡ (ti − p).max(0)` — so there is no wrong
number now.

**Consequence:** no understatement today; the defect is that Form 6251 line 20 and QDCGT line 5 are
**two definitions of one line with no test pinning them together.** A future edit to either (an OBBBA
year, a §1250 tranche, an SDTW) moves one and not the other, and the whole AMT Part III is keyed off
that value. This is also the exact structure the doctrine warns about: the dropped term becomes
invisible once the lines are gone. I did **not** find a test that reds if the two diverge.

Note the closed form does carry a partial equivalence discussion (F-A: the `min(L1, L4)` cap; F-B: the
binding `min(L23, L24)`) with KATs at `method.rs:274` and `method.rs:288`. That covers two named
branches; it does not make lines 4 and 5 readable.

### I-4. Nothing reds when the Schedule D routing enum and the tax method disagree

`ScheduleDRouting` (`printed.rs:823-841`) decides **which worksheet the printed form tells the filer to
use**. `return_1040.rs:1800` calls `qdcgt_line16` **unconditionally**, with a comment
(`return_1040.rs:1796-1799`) arguing it degenerates to the plain Tax Table / TCW when preferential
income is zero.

I walked all four branches and the comment is **correct**: `BothGains` → QDCGT (line 20 = Yes);
`ShortGainLongLoss` → line 17 = No → line 22, and `net_ltcg` is 0 there so QDCGT collapses to
QD-only, which is what line 22 routes to; `NetLoss` and `Zero` → line 22 again, same collapse.

**Consequence:** none today. But the equivalence is asserted in a comment, not held by a test. The
`ScheduleDRouting` enum is exhaustive and typed; the tax method ignores it entirely. If an SDTW branch
is ever added to the enum, `qdcgt_line16` will keep being called and nothing will notice.

### M-5. Form 8949 box selection is hardcoded per year-map; the broker-review flag is dropped on the full path

**Form requires:** i8949 — *"You must check Box A, B, or C… Check only one box."* Box F = long-term,
**not** reported on a Form 1099-B.

**btctax does:** for TY2024 (`f8949.map.toml:12-13, 35-36`) the box is a **map constant**: Box C /
Box F. Core's `Form8949Box` taxonomy (`forms.rs:42-51, 135-145`) is year-aware and never auto-assigns
a broker-reported box, with the reasoning recorded at `forms.rs:37-40`.

**This is correct for this vector.** TY2024 is pre-1099-DA; a bitcoin disposal was not broker-reported
on a 1099-B, so Box F is the right box for a $1M long-term crypto disposal, and full-return v1 is
TY2024-only anyway (`schedule_d_full.rs:77`, `form1040_full.rs:57`).

**The gap:** `box_needs_review` (`forms.rs:73, 149` — set when the disposing wallet is an
`Exchange`) is surfaced only through `btctax-cli/src/render.rs:1174` (the CSV) and
`btctax-forms/src/lib.rs:418 rows_possibly_broker_reported`. Neither is reachable from the full-return
packet: `Printed8949Row` does not carry the flag, and `fill8949_full.rs` never consults it.

**Consequence:** no wrong dollar for TY2024. For TY2025+ when the full return is extended, an exchange
sale reported on a 1099-DA would be filed under Box L ("not reported") with the advisory silently
dropped — an IRS matching mismatch on $1,000,000 of proceeds, invisible to every value-checking test.
Flagging now because the year gate is the only thing holding it.

---

## What is NOT missing — checked and clean

Recording these so the synthesis agent does not re-open them.

- **A donation is a REMOVAL, not a disposal — CONFIRMED, structurally.** The fold pushes to
  `st.removals` (`crates/btctax-core/src/project/fold.rs:1412`) with `kind: RemovalKind::Donation`;
  `crates/btctax-core/src/forms.rs:200-218 schedule_d` iterates **`state.disposals` only**. There is no
  code path by which a donated lot reaches Schedule D, Form 8949, or `net_1222`. The donated coin's
  appreciation is correctly never recognized (§170 / §1001 — no sale or exchange), and its FMV reaches
  Schedule A through the separate `crypto_charitable_gifts` projection
  (`return_1040.rs:642-671`), which reads `state.removals` and partitions by holding period:
  LT → `CapGainProp30` at FMV, ST → `OrdinaryProp50` at `min(FMV, basis)` per §170(e)(1)(A). The
  removal does consume lots from the same pool the disposals draw on, so the surviving basis for the
  $1M *sale* is correct under HIFO/FIFO.
- **The §1(h) stacking order is right, and it is what decides this return.** `qdcgt_line16` computes
  `bottom = max(0, TI − (QD + net LTCG))` (`method.rs:84`), so the $1M charitable deduction lands on
  the **ordinary** slice first — exactly the §1(h) stacking. `preferential_tax`
  (`compute.rs:57-99`) then fills the 0% / 15% / 20% bands from `bottom` upward against the bundled
  TY2024 MFJ breakpoints `max_zero = 94,050`, `max_fifteen = 583,750`
  (`crates/btctax-adapters/src/tax_tables.rs:396-399`). With this vector's ordinary slice well above
  $94,050, the 0% band is fully consumed by ordinary income and none of the $1M gets it — which is the
  correct and non-obvious answer.
- **The charitable ceiling is 30%, not 60%, and the excess carries.** `apply_170b`
  (`charitable.rs:106+`) uses `pct(dec!(0.30))` for `CapGainProp30` against AGI, with §170(d)(1)
  5-year carryover and Pub. 526 Worksheet-2 ordering. For AGI $2,000,000 that admits **$600,000** this
  year and carries **$400,000**. A church is a §170(b)(1)(A)(i) 50%-org, so
  `RefuseReason::NonPublicCharityContribution` (`return_refuse.rs:859`) does not fire.
- **§1222 / QDCGT line 3.** `net_1222` (`compute.rs:137-195`) cross-nets characters and yields
  `preferential_gain` ≡ *"smaller of Schedule D line 15 or line 16"*, verified against all four sign
  combinations. `net_ltcg = cap.preferential_gain` at `return_1040.rs:1545`.
- **The 1099-B / basis-reported path is the cleanest thing in this slice.** `Form1099B`
  (`return_inputs.rs:138-190`) is totals-only *because the form says so* — Schedule D line 1a/8a are
  available exactly when basis was reported **and** there are no adjustments, and that pair of
  conditions is **collected as the filer's own testimony** (`basis_reported_and_no_adjustments`), with
  anything else refused by `RefuseReason::Form1099BNeedsForm8949` (`return_refuse.rs:236, 678`). No
  second lot engine, no fabricated box. Not exercised by this vector (a pure-crypto filer has no
  1099-B), and `schedule_d_full.rs:103-124` correctly writes **nothing** to lines 1a/8a rather than a
  zero when there are none.
- **Lines 18/19 blank, not zero** (`schedule_d_full.rs:263-278`) — already fixed, with the §G-24
  reasoning in place.
- **Schedule D `must_file`** (`printed.rs:904-935`) enumerates the individual columns rather than
  trusting the net, so offsetting broker totals cannot make a required schedule vanish.
- **The Schedule D Tax Worksheet is genuinely unreachable**, not merely unimplemented. Every route to
  it is closed: 1099-DIV boxes 2b/2c/2d refuse (`return_refuse.rs:1031`,
  `UnrecapturedOrSpecialRateGain`); Schedule D line 19 is structurally 0 (`form6251.rs:572`); no Form
  4952 exists. Form 6251 line 15 correctly takes the *"did not complete a Schedule D Tax Worksheet"*
  branch with the reasoning transcribed at `form6251.rs:573-580`.

---

## THE SMALLEST THING THAT CLOSES IT

Sequenced. (1) is the only one that blocks filing.

1. **Paginate the full-return Form 8949 by calling the pagination that already exists.**
   In `crates/btctax-forms/src/lib.rs:336 fill_8949_full`, replace the single-shot call with the same
   chunk-and-merge shape as `fill_form_8949` (`lib.rs:90-112`), but using
   `fill8949::fill_8949_parts_with_identity` per copy so **every** page carries the name/SSN header
   (`fill8949.rs:149-159` — an unnamed 8949 is not filable, and each page has its own header). Then
   `overflow::merge_copies` (`overflow.rs:23`), which already uniquifies field names per copy.
   Attaches to: `Printed8949 { short_term, long_term }`, chunked by `map.rows_per_page`.
   - **Correct the false comment** at `fill8949_full.rs:73-75` ("exactly as the slice does").
   - **B1 pairing (required):** a test that builds a `Printed8949` with 15 long-term rows, asserts
     `fill_full_return` returns `Ok`, and reads back **row 15 on copy 2** — and a second that plants
     the regression (call `fill_8949_parts_with_identity` directly) and asserts `Overflow`. There is
     currently no 8949 overflow test at all; the Schedule B one at `full_return_forms.rs:2914` is the
     model.
   - Also assert the **cross-foot**: Σ of the per-copy line-2 totals ≡ Schedule D lines 3/10, since
     `schedule_d_lines` (`printed.rs:970-971`) reads `f8949.st_totals` / `lt_totals`, which are
     computed over all rows and must not be re-derived per page.

2. **Collect investment interest expense, and let Schedule D line 20 read the answer.**
   This is *following instructions*, not scope creep: the form asks a question our input surface
   cannot answer, so collect it.
   - Add `investment_interest_4952: Option<Usd>` (or a `filing_form_4952: Option<bool>` +
     amount) to `ScheduleAInputs` in `crates/btctax-core/src/tax/return_inputs.rs`. `Option` so
     unanswered is distinguishable from zero.
   - Add a `ScheduleDRouting` variant or field carrying line 20's answer, replacing the hardcoded
     `check(..., true, ...)` at `schedule_d_full.rs:279-284`. Adding the variant reds the exhaustive
     match in `schedule_d_full.rs:234` — free blast radius, per house doctrine.
   - **Refuse, do not guess**, when the answer is `None` *and* the return is on the `BothGains`
     branch — a new `RefuseReason` alongside the existing unanswered-declaration family
     (`AmtCarryoverDeclarationUnanswered`, `MixedUseMortgageUnanswered`). Also refuse when the answer
     is "filing 4952", since that routes to the unimplemented SDTW. **Refusing is strictly better than
     today**: today btctax swears "not filing 4952" on the filer's behalf.
   - Wire Schedule A line 9 at the same time (`f1040sa.map.toml:100` currently `unmodeled`) — the
     input and the line are the same fact, and leaving line 9 blank while asking the question would be
     the §G-24 defect again.

3. **Transcribe the QDCGT worksheet as a struct, lines 1–25, and delete the two duplicate line-5
   derivations.**
   New `QdcgtLines` in `crates/btctax-core/src/tax/method.rs` with one field per numbered line
   carrying the i1040 instruction text verbatim; `qdcgt_line16` becomes `qdcgt(...) -> QdcgtLines` and
   `line16 = round_dollar(min(l23, l24))` reads off it. Then `return_1040.rs:2061` becomes
   `qdcgt.line5` (delete the re-derivation) and `form6251.rs:727` in the fixture harness likewise.
   This is the doctrine-conforming move *and* the prerequisite for ever adding the SDTW, whose lines
   13/14/21 Form 6251 Part III cites in exactly the same way.
   - **Guard that reds:** a KAT asserting `f6251.line20 == qdcgt.line5` and
     `f6251.line13 == qdcgt.line4` over the existing 30-vector 6251 fixture corpus. Mutation-verify by
     perturbing one derivation.

4. **(TY2025 gate, file as a follow-up owned by whatever phase extends the full return past TY2024.)**
   Carry `box_needs_review` into `Printed8949Row` and refuse — or emit the advisory — when a
   full-return 8949 would file an exchange disposal under Box I/L on a 1099-DA year.

---

## WHAT I AM NOT SURE OF

- **I did not execute the vector.** Every claim above is read off source; I did not build a
  `ReturnInputs` TOML and run `assemble_absolute` end to end, so the specific dollar figures for this
  household (TI, QDCGT line 16, the $600,000 / $400,000 charitable split) are *derived from the code I
  read*, not measured. **Someone should run it against both oracles before any of this is treated as
  settled** — in particular the interaction of a $600,000 Schedule A charitable deduction with the
  AMT, which is not my slice but sits directly downstream of `taxable_income`.
- **Whether the vector's donation is bitcoin from the ledger or cash.** I assumed ledger bitcoin
  (the brief's "removal, not disposal" question implies it). If it is **cash**, the class is `Cash60`,
  the ceiling is 60% of AGI = $1,200,000, the entire $1,000,000 is deductible this year with no
  carryover, and taxable income drops another $400,000 — which changes the QDCGT ordinary slice and
  every downstream figure. **This materially changes the answer and should be pinned by the synthesis
  agent.**
- **Realistic lot count.** C-1 is conditional on more than 14 legs in Part II. I have no data on the
  lot distribution of a real $1M bitcoin position; if the filer bought once and sold once it is 1 row
  and the packet emits fine. My judgement is that >14 is the overwhelmingly common case for a holder
  who accumulated to $1M+ of gain, but that is judgement, not measurement.
- **Whether `merge_copies` is safe with identity written on every copy.** The slice's paginated path
  uses `fill_8949_parts` (no identity) and merges; I did not verify that `merge_copies`'
  root-component renaming (`overflow.rs:41-43`) behaves correctly when each fragment also carries
  `identity_page1` / `identity_page2` writes. It should — the rename is on the root prefix and all
  descendants inherit — but this is the one step of fix (1) I could not settle by reading.
- **Whether the §163(d)(4)(B)(iii) election is in scope at all**, or whether refusing on
  "filing Form 4952" (my proposal 2) is the accepted resolution versus implementing the Schedule D Tax
  Worksheet. Refusal is the conservative move and matches how the repo has handled every other
  unmodeled worksheet, but it is a product decision, not mine.
- **Rounding inside `preferential_tax`.** The worksheet has separate lines 16 and 19 (15% and 20%
  products); the code sums them and rounds once (`compute.rs:92 round_cents`). At whole-dollar inputs
  each product is exact to two places, so I believe this is a no-op, but I did not prove it for
  fractional-cent inputs and the frozen-guard pin on `compute.rs` means it is not casually testable.
