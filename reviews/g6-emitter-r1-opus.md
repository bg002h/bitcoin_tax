# §G-6 Form 6251 emitter — independent review r1 (Opus)

**Commit:** `942120e` · **Branch:** `feat/no-pen-deferrals` · **Brief:** `reviews/g6-emitter-r1-BRIEF.md`

**The one question:** can this commit put a wrong number, or a wrong blank, on a return signed under
26 USC §6065?

**Method.** Read the whole diff; verified the map assignment independently against the PDF's own
widget rects (`cargo run -p xtask -- dump-fields`) and against
`design/forms/extract/f6251--2024.txt`; traced the four chains that carry the AMT figure
(`Form6251::line11` → `Schedule2Lines::line2/line3` → `Form1040Lines::line17/18/24`, and the parallel
`AbsoluteReturn::l18 → total_tax`); ran `cargo nextest run -p btctax-forms --test f6251_map --test
f6251_fill --test full_return_forms` (77/77 green). No file was modified.

---

## Answers to the brief's five risk areas

**1. `compute_6251` moved above the credits block — does it still get the same inputs?**
Yes. Every argument (`agi`, `taxable_income`, `deduction`, `qbi.deduction`, `deduction_is_itemized`,
`schedule_a`, `qualified_dividends`, `net_ltcg`, `regular_tax`, `foreign_tax_credit`) is bound above
the new position at `return_1040.rs:1849`; `foreign_tax_credit` at `:1830` is the last of them, which
is exactly what the comment claims. Nothing between the new position and the old one (`l18`,
`nonrefundable_credits`, `tax_after_credits`, `schedule_2_other_taxes`, `total_tax`,
`excess_social_security`, the withholding block, `overpayment_refund`, `amount_owed`) rebinds any of
them, and none of them is an input to `compute_6251`. The move is value-neutral.

**No downstream double-count.** With AMT owed, `l18 = regular_tax + line11 = regular_tax + (line9 −
line10) = regular_tax + (line7 − FTC) − (regular_tax − FTC) = line7`, so `tax_after_credits = line7 −
FTC = line9` — the correct §55 result, and the FTC cancels because Form 6251 line 10 already
subtracted it. `schedule_2_other_taxes` is Part II only (`se + addl_medicare + niit`); Schedule 2's
printed `line21` likewise excludes lines 2/3, so 1040 line 23 does not re-add the AMT. The excess-SS
path reads neither.

**2. Is `line11 > 0 ⇒ line7 > line10` true against the actual field definitions?**
Yes, and it is airtight. `form6251.rs:607-613`: `line8 = if line10 >= line7 { 0 } else {
schedule_3_line1 }`, `line9 = line7 − line8`, `line11 = (line9 − line10).max(0)`. If `line10 ≥ line7`
then `line8 = 0`, so `line9 = line7 ≤ line10` and `line11 = 0`. Hence `line11 > 0 ⇒ line10 < line7`.
The remaining premise — `line8 ≥ 0` — holds because `schedule_3_line1` is `foreign_tax_credit`, a sum
of 1099-INT box 6 / 1099-DIV box 7, and `return_refuse.rs:652` (`first_negative_amount`) refuses any
negative money input before anything accumulates. So the unconditional `+ amt.line11` never adds a
figure the printed chain drops — *given Schedule 2 files at all*, which is M4 below.

**3. `line2: Option<Usd>` vs `line3: Usd` — is the asymmetry right?**
Right today, and correctly reasoned: line 2 is a conditional entry ("Attach Form 6251"), line 3 is
arithmetic btctax performs, and 1040 line 17 names line 3 by number, so line 3 must not be blank
behind a filed figure. But the emitter's gate is `part_i = lines.line2.is_some()`
(`schedule23.rs:59`), which governs **both** lines. If Schedule 2 line 1z ever gains an input, line 3
would vanish while 1040 line 17 read it — the exact defect this commit fixed, one line up. Latent
only: excess APTC has no input and is listed UNREPRESENTABLE. Noted in M4.

**4. Does any consumer read a rounded line where it needs the unrounded one?**
No mismatch found. `Schedule2Lines::line2 = round_dollar(f.line11)` (`printed.rs:368`) and the
emitter writes `f.printed().line11 = round_dollar(line11)` (`form6251.rs:90` in the forms crate) —
literally the same expression on the same value; the packet KAT asserts both. `must_attach()` is
evaluated on the unrounded struct, which is correct (it is a decision, not a printed figure).
`PrintedForms::f6251` stores the *unrounded* core struct while its siblings store printed chains;
that asymmetry is safe only because the sole consumer is the filler, which rounds — I checked, there
is no other reader. One genuine second-authority is M5.

**5. The map's page-1 assignment, and whether the inset test constrains it.**
**The assignment is correct.** I verified it directly rather than from the map's prose:

| widget | rect (x0,y0–x1,y1) | width | line |
|---|---|---|---|
| `f1_3` | 504,636–576,648 | 72 | 1 |
| `f1_4` | 504,612–576,624 | 72 | 2a |
| `f1_5` | **508**,600–**572**,612 | **64** | 2b `(  )` |
| `f1_9` | **508**,552–**572**,564 | **64** | 2f `(  )` |
| `f1_22` | **508**,396–**572**,408 | **64** | 2s `(  )` |
| `f1_23` | 504,384–576,396 | 72 | 2t |
| `f1_24` | 504,372–576,384 | 72 | 3 |
| … | | | |
| `f1_32` | 504,48–576,60 | 72 | 11 |

Page 1 carries exactly 30 money widgets (`f1_3`..`f1_32`) against exactly 30 numbered boxes
(1, 2a–2t, 3, 4, 5–11); the three w=64 widgets fall on `f1_5`/`f1_9`/`f1_22`, which under this
assignment are precisely 2b/2f/2s, the form's only three parenthesised boxes. The y-gaps corroborate
the extract's row counts (line 4 = 2 rows, line 5 = the exemption table, line 7 = the bullet block,
line 10 = 5 rows). Page 2 is `f2_N` = line N+11 for all 29, gaps again matching (line 13 = 4 rows,
line 19 and line 25 = bullet lists, line 20/27 = 5 rows). Schedule 2's two new cells check out too:
`f1_11`@390 = 1z, `f1_12`@372 = 2, `f1_13`@354 = 3, `f1_14`@324 = 4.

**Does the inset test constrain the offset, or merely happen to pass?** It constrains — partly, and
the boundary is worth stating precisely, because the map's header claims more than the tests deliver.

* *Constrained.* `the_three_inset_widgets_land_on_the_three_parenthesised_lines` pins `2b → f1_5`, so
  the `1 / 2a / 2b` prefix cannot slide. `the_lines_descend_each_page_in_order` forbids any
  transposition within a page. `every_mapped_field_exists_in_the_blank_form` forbids any **upward**
  shift of the `3..11` block: `+1` puts line 11 on `f1_33`, which does not exist.
* *Not constrained.* The exhaustiveness assertion is `m.len() + censused + 2 == fields.len()` — a
  **count, not a partition**. It never checks that the mapped set and the censused set are disjoint.
  A **downward** shift of the `3..11` block (line 3 → `f1_23`, …, line 11 → `f1_31`) keeps the count
  at 61, keeps every FQN existent, keeps y descending, and keeps 2b inset — it passes all four tests
  while printing line 3's figure in line 2t's box and leaving `f1_32` blank. The same hole admits any
  single mapped line pointed at a censused widget.

  This is *not* a claim that the map is wrong — I verified above that it is right. It is a claim
  about what the tests would catch if it were changed. Closing it is one line: assert the mapped FQNs
  and the censused FQNs form a **disjoint union** of `fields`, not just that the cardinalities add up.

---

## Findings

### CRITICAL — none

I could not construct a filer, from inputs this program accepts, for whom this commit produces a
wrong figure or a wrong blank on a filed page.

### IMPORTANT — none

Stated plainly, per the brief: nothing at this severity. The three chains agree, the map is right,
the two sign conventions are right, the Part III gate is right, and the `l18` implication holds.

### MINOR — Form 6251 is stapled OUT of Attachment Sequence order, after Forms 8995 / 8995-A

**File:line:** `crates/btctax-forms/src/packet.rs:169-201`

**Failure:** The block's own comment says Form 6251 "staples between Schedule SE (`17`) and Form 8995
(`55`)" — but the code pushes it *after* the `f8995` (`55`) and `f8995a` (`55A`) blocks. Emitted
order for a filer with **both** a §199A deduction and an AMT (e.g. the `amt_owing_household` fixture
plus a Schedule C): `… schedule_se(17), f8995(55), f6251(32), f8959(71) …`. That contradicts the
module doc ("the packet is emitted in that order (1040 first, then ascending)") and the
`manifest.txt` the CLI presents as "← your stapling order" (`btctax-cli/src/main.rs:790`). No figure
is wrong, which is why this is Minor and not Important — but the filer is told to assemble their
return against the IRS's own printed sequence numbers and is given the wrong order.

The guard that would catch it exists and is blind: `golden_packet.rs:663-689` asserts the emitted
sequence list is sorted, but no golden household owes AMT, and
`the_packet_emits_every_required_form_in_attachment_sequence_order` runs on `kitchen_sink_household`,
whose `expect_attached` is `false`.

**Fix:** move the `if let Some(amt) = f6251 { … }` block above the `f8995` block (between Schedule SE
and Form 8995), and add an AMT+QBI household to the ordering test so the sortedness fold can fire.

### MINOR — the §6.1 forms census still says 15/16 forms; `fill_full_return` can now emit one more

**File:line:** `crates/btctax-forms/tests/common/mod.rs:16-33` (`CENSUS_KEYS: [&str; 16]`),
consumed by `crates/btctax-forms/tests/census.rs:115-150` and `:372-389`

**Failure:** `CENSUS_KEYS` enumerates "the forms `fill_full_return` can emit" and does **not** contain
`"f6251"`. The commit added a new emittable form and did not extend it. `census_is_exactly_15_…`
still passes only because its all-arms household has no AMT, so the new arm is never exercised; and
`every_census_form_demonstrated_in_j6` will never require an AMT demonstration. The census's own
assertion message — *"the emitted form-name set must equal the §6.1 census keys exactly — a
difference is a new or renamed form the census does not yet track"* — is now silently false for every
AMT packet.

This is the same shape as the two defects the commit celebrates finding (a census whose reason the
emitter falsified). It is Minor because it is test-side only: nothing at runtime reads `CENSUS_KEYS`,
and I confirmed there is no hardcoded form-name list in the CLI or TUI.

**Fix:** add `"f6251"` to `CENSUS_KEYS` and give the all-arms fixture an AMT (or add a second arm) so
the key is demonstrated rather than merely declared.

### MINOR — the coverage table records a TRUNCATED instruction for Form 6251 line 6 — the exact trap the map file documents

**File:line:** `crates/btctax-core/src/tax/line_coverage.rs` (`cover_form6251`, the `line6` row)

The recorded text is:

> `"Subtract line 5 from line 4. If more than zero, go to line 7. If zero or less, enter -0- here and on lines 7, 9, and"`

It stops at "and", dropping **"11, and go to line 10"** — i.e. it drops line 11 from the zero-out
set. `f6251.map.toml:50-51` names this failure explicitly: *"SPLIT across extract rows 56 and 62 by
four brace-glyph rows — the survey's trap #1. A row-wise reader stops at 'lines 7, 9,' and silently
drops 11 from the zero-out set."*

**Failure:** No wrong figure — the code is correct (`form6251.rs:548-550` returns early, leaving 7, 9
and 11 at zero), and the `Form6251::line6` doc comment carries the full sentence. But the artifact
whose job is *"does each doc comment match the official instruction text?"* now carries the degraded
version, and it passes because `xtask line-coverage`'s `normalize()` collapses whitespace only, while
`f6251_map.rs`'s `norm()` also strips standalone `{`/`}` glyphs. The full quote would not match the
extract (`"…lines 7, 9, and } 11, and go to line 10"`); the truncated one does. Two authorities for
one line's instruction text, and the checker was satisfied by shortening the citation.

**Fix:** teach `line_coverage_check.rs::normalize` the same standalone-brace filter
`f6251_map.rs::norm` already has (it is mechanical layout normalisation, not a per-line exception),
then restore the full sentence.

### MINOR — three stale claims left in source, each asserting the refusal this commit deleted

**File:line:**
* `crates/btctax-core/src/tax/return_1040.rs:2069-2070` — `screen_absolute`'s **own doc comment**
  still lists row *"(b) **Form 6251 Who Must File condition 1** … so the form must be attached and v1
  cannot yet file it (§4.11)"*. That row was deleted from that function's body by this commit.
* `crates/btctax-core/src/tax/printed.rs:306-310` — the `Schedule2Lines` doc still reads *"**Part I
  is entirely BLANK in v1** … line 2 (AMT) is $0 by construction — the return is refused outright if
  the official 'Should You Fill In Form 6251' worksheet trips. So 1040 line 17 is zero, and nothing in
  Part I is printed."* It sits directly above the struct this commit added `line2`/`line3` to.
* `crates/btctax-core/src/tax/printed.rs:349-350` — `schedule_2_lines`'s doc: *"Returns `None` when
  there is nothing to report — no SE tax, no Additional Medicare Tax, no NIIT"*. Still an accurate
  description of the code, but it now omits the AMT, which is the whole of M4 below.

(`return_refuse.rs:238-239` carries a fourth, on `RefuseReason::AmtScreenTriggered`; the brief
excludes that variant, so I fold it in rather than count it.)

**Failure:** No wrong figure. But defect #1 in the commit message *was* a stale warrant that made a
blank look structural, and the commit message says docs were swept for claims the code no longer
makes. These three are the same claim, in the two files the commit changed most, including the doc of
the very function whose body it edited. The next author reading `schedule_2_lines`'s doc will not
learn that the AMT can now reach Part I.

**Fix:** rewrite the three doc comments to describe the post-§G-6 behaviour.

### MINOR — `schedule_2_lines` drops the whole schedule on `line21 <= 0`, with no term for the AMT

**File:line:** `crates/btctax-core/src/tax/printed.rs:373-377`

```rust
let line21 = line4 + line11 + line12; // ★ sums the PRINTED lines
if line21 <= Usd::ZERO {
    return None;
}
```

**Failure (latent, and I could not reach it — here is the analysis, so the next reader does not have
to redo it):** `line21` is Part **II** only (SE tax + Additional Medicare + NIIT). If `must_attach()`
held while `line21 == 0`, the packet would staple a Form 6251 showing `line11 > 0` while `sch_2` was
`None`, so 1040 line 17 = `sch_2.map_or(ZERO, …)` = `$0` — an understatement whose own evidence is
attached to it — and `AbsoluteReturn::total_tax` (which adds `amt.line11` unconditionally) would
split from the filed 1040 line 24.

I believe that state is **unreachable today**, by a coincidence of two unrelated thresholds rather
than by construction:

1. AMT can only bite on preferential income. With btctax's modelled add-backs (line 2a ≤ the standard
   deduction, or Schedule A line 7 with SALT capped at $10,000; line 2b negative; lines 2c–2t absent),
   a pure-ordinary-income return has `flat_26_28(line6) < graduated(TI)` at every income — the 26/28%
   schedule on `TI + addback − exemption` never overtakes graduated rates topping out at 37%. So
   `preferential = net_ltcg + qualified_dividends > 0` is necessary.
2. AMT further requires the exemption to be substantially phased out, i.e. AMTI ≳ $609,350 (Single /
   MFS) or ≳ $1,218,700 (MFJ/QSS). And `AMTI = TI + line2a ≤ AGI`, so `AGI ≳ $609,350` — far above
   every §1411(b) threshold.
3. `form_8960`'s NII is `interest + ordinary_dividends + L7 + lending`. When `preferential_gain > 0`,
   `loss_deduction = 0` and `L7 = ordinary_gain + preferential_gain ≥ net_ltcg`
   (`return_1040.rs:1544-1545`); and `box1b_qualified > box1a_ordinary` is refused
   (`return_refuse.rs:1012`), so `ordinary_dividends ≥ qualified_dividends`. Hence `NII ≥
   preferential > 0`, `MAGI − threshold > 0`, and NIIT > 0 → `line12 > 0` → `line21 > 0` → Schedule 2
   files.

So: AMT > 0 ⇒ NIIT > 0 ⇒ Schedule 2 exists. Nothing in the code says so, nothing tests it, and the
predicate that would break it lives in `other_taxes.rs`, two modules away. The `schedule2` biconditional
KAT (`packet.rs:1310`) *does* assert `f6251.is_some() == line2.is_some()`, so it would red if this
ever became reachable — but only for a household that reaches it, and none exists.

**Fix:** make the filing gate carry the AMT term explicitly — `if line21 <= ZERO && line2.is_none() {
return None; }` — which is also what Schedule 2's own filing instruction says (Part I *or* Part II).
Same one-line class of fix for `schedule23.rs:59`'s `part_i = lines.line2.is_some()`, which will drop
line 3 if line 1z ever becomes populated.

### MINOR — Form 6251 line 10 is built from `ar.regular_tax`, not the PRINTED 1040 line 16

**File:line:** `crates/btctax-core/src/tax/return_1040.rs:2057` (`regular_tax_l16: regular_tax`),
consumed at `crates/btctax-core/src/tax/form6251.rs:545`

**Failure:** The form says *"Add Form 1040 or 1040-SR, **line 16** … Subtract … Schedule 3 line 1."*
btctax feeds the exact-cents `ar.regular_tax`, while the **printed** 1040 line 16 is deliberately a
different figure — `printed.rs:713-718` explains at length that line 16 is the worksheet applied to
the *printed* line 15, "NOT a re-rounding of the tax computed on the exact-cents taxable income (Fable
P6 r1 I2)", and can differ by a whole Tax-Table bin step. So Form 6251 line 10 is derived from a
second authority for the number the form tells it to transcribe, and any difference propagates
straight through line 11 → Schedule 2 line 2 → 1040 line 17 → total tax.

The exposure is bounded and small: an AMT-owing return always has taxable income ≳ $600k, where the
Tax Table's step function does not apply and the QDCGT worksheet is continuous, so the two figures
differ by at most sub-dollar input residuals — i.e. ≤ $1 after rounding, inside SPEC §3.1's elected
per-line tolerance. That is why this is Minor. It is recorded because it is a
two-authorities-for-one-number instance that the same file elsewhere treats as a defect class
(Schedule 2 line 4 ← Schedule SE line 12, line 11 ← Form 8959 line 18, and this commit's own line 2 ←
Form 6251 line 11 all follow the opposite rule).

**Fix:** either feed the printed 1040 line 16 into Form 6251 line 10, or record in the field's doc why
this one line is exempt from the transcribe-the-printed-source rule the surrounding code states three
times.

### MINOR — LIMITATIONS.md's rewritten AMT bullet no longer tells the filer about lines 2c–2t

**File:line:** `crates/btctax-cli/LIMITATIONS.md:222`

The bullet was rewritten by this commit and now names only the three declared adjustments (lines 3,
2k, 2l). The eighteen unmodelled Part I add-backs — including **2i, the ISO exercise**, which the
previous commit calls "the dominant real reason an individual owes AMT" post-TCJA — are not mentioned
anywhere in the file (I grepped: no occurrence of "incentive stock"). The §G-22 out-of-scope
declaration that gates them is likewise undocumented there. No figure is wrong (the refusal fires),
but the user-facing limitations document is where a filer checks whether btctax fits them, and this
commit is the one that made "AMT returns are refused outright" false.

**Fix:** add a bullet naming the 2c–2t gap and the declaration that gates it.

### NIT — `a_negative_magnitude_in_a_parenthesised_box_fails_closed` does not test that

**File:line:** `crates/btctax-forms/tests/f6251_fill.rs:141-150`

The test sets `line2b = Usd::ZERO` and asserts `is_ok()`. `assert_paren_magnitudes`
(`form6251.rs:68-79`) has therefore never been observed **red**, which is B1's whole point. Its guard
is genuinely unreachable through the public entry point (the `.abs()` at `:95` precedes it), so the
honest options are to test it directly if it can be made visible, or to rename the test to what it
asserts and record that the guard is a compile-time-unreachable backstop.

### NIT — `the_absolute_total_tax_equals_the_printed_1040_line_24` asserts an equality per-line rounding does not guarantee

**File:line:** `crates/btctax-core/src/tax/packet.rs:1408-1418`

`round_dollar(ar.total_tax)` is the rounding of an exact-cents sum; `f1040.line24` is a sum of
independently rounded lines. Under SPEC §3.1 those may legitimately differ by a dollar or two. The
assertion is the right *guard* (it is what caught the $13,461 understatement) but the wrong
*invariant*; a future fixture can red it for a reason that is not a defect, and the tempting fix then
is to weaken it. Consider `abs_diff <= $2` with a comment naming the rounding reason, or keep exact
equality and document that the fixtures are chosen to foot.

### NIT — Form 6251 line 25's coverage quote carries a stray `}` brace glyph

**File:line:** `crates/btctax-core/src/tax/line_coverage.rs` (`cover_form6251`, `line25`)

The recorded "official instruction text" ends `"… $551,350 if head of household. }"`. The brace is a
`pdftotext` layout artefact, not instruction text — the same artefact the line-6 row worked around by
truncating instead. Both disappear if `normalize()` learns the standalone-brace filter (see the
line-6 Minor).

---

## What I checked and found clean

* Two sign conventions: `line2b` stored negative, written as `.abs()` into the inset `f1_5`; line 1
  keeps its literal minus; both pinned by read-back tests off the serialized PDF.
* Part III as a unit: `part_iii_completed` is set only on line 7's Part III branch
  (`form6251.rs:599`), and the emitter writes lines 12–40 only under it. The routing predicate
  (`preferential > 0`) matches the form's three bullets on every case I could construct.
* `Form6251::printed()` destructures exhaustively with no `..`, including `part_iii_completed`, so a
  new line cannot ship cents.
* The TY2025 line-1 shape refuses against the TY2024 map rather than dropping sub-line 1a.
* Whole-dollar read-back over all 41 mapped cells, with a `checked == 41` anti-vacuity count.
* The biconditional KAT's `expect_attached` column is a real anti-vacuity guard and `amt_owing_household`
  is a genuine true case (narrow margin, ledger-free).
* Schedule 2's descent ordinals are incremented only for written cells, so skipping Part I does not
  corrupt the page-1 descent check.
* No consumer of `AbsoluteReturn::amt` outside the packet path and the CLI report block
  (`render.rs:1541-1558`), which prints an accurate attach message — no stale refusal text there.
* `cargo nextest run -p btctax-forms --test f6251_map --test f6251_fill --test full_return_forms`:
  77 passed, 0 failed.

VERDICT: 0 Critical, 0 Important
