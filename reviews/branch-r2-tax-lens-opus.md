# Whole-diff review r2 — TAX-CORRECTNESS lens (Opus)

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `afa0ffe..HEAD` (9 substantive
commits: SSN unblock, death-gate downgrade, the on-state guard, §G-9a, Form 8995 line 3, 8283 page-2
identity, Schedule C I/J, the 1040 line-7 box, advisory reach).

**Brief:** one question — *does any change put a WRONG FIGURE on a signed return, fabricate an answer
the filer never gave, or newly BLOCK a return that should compute?* Five named probes, ordered by what
the author was least sure of. Settled facts supplied (dumped on-states, the frozen pin, the green
suite, 8283 5a/5b/5c out of scope). Everything before `afa0ffe` explicitly out of scope — reviewed in
round 1.

**Reviewer output reproduced VERBATIM below.**

---

`VERDICT: findings` (2 Minor, 1 Nit — no Critical, no Important)

```
SEVERITY: Minor
WHERE: crates/btctax-input-form/src/spec/coverage.rs:305 ("qbi" in EXEMPT_PREFIXES)
       crates/btctax-forms/forms/2024/f8995.map.toml (deleted census entry, diff line 2808)
       crates/btctax-forms/tests/field_census.rs:166 (GAPS 13 → 6)
CLAIM: The Form 8995 line-3 census gap is closed, but the understatement path it recorded is not — the
       input is reachable only by hand-editing TOML, no question asks for it, and no advisory names it.
FAILURE: Single, TY2024. W-2 $110,000, Schedule C mining net $50,000, taxable income before QBI
       $150,000 (below the $191,950 threshold, so no refusal). The filer's TY2023 return, prepared
       elsewhere, produced a $20,000 §199A qualified-business-loss carryforward. They drive btctax
       through `income answer` / the TUI, which never poses the question (`qbi` is a WHOLESALE prefix
       exemption in the input-form coverage census, so no Field exists), and `advisories.rs` contains
       no advisory mentioning a carryforward at all (grep: zero hits). `qbi_carryforward_in` stays 0 ⇒
       Form 8995 line 3 prints BLANK, line 4 = $50,000 instead of $30,000, line 15 = $10,000 instead
       of $6,000. Taxable income understated $4,000; tax understated ≈$880. That is verbatim the
       hazard the deleted census entry recorded — and the entry is now gone while the hazard is not.
EVIDENCE: deleted from f8995.map.toml: `{ line = "3", rule = "gap", reason = "…btctax neither models
       nor asks for it, so a filer with a prior-year QBI loss gets an INFLATED deduction and
       UNDERSTATED tax. Closes by collecting it." }`. FOLLOWUPS.md now claims "Form 8995 line 3 — …
       the only gap ever recorded in the UNDERSTATEMENT direction" is closed. LIMITATIONS.md's own
       remedy is a TOML key: "enter it as a POSITIVE number under `[qbi]` as `qbi_carryforward_in`".
       coverage.rs EXEMPT_PREFIXES still contains "qbi".
       ★ Minor, not Important, because this is exactly the shape of the long-accepted
       `reit_ptp_carryforward_in` (line 7) beside it — also import-only, also unadvised — and the diff
       makes no figure worse than it was. The regression is in the SAFETY NET, not in a number.
```

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/printed.rs:703 (`line7_schedule_d_not_required: !schedule_d_filed`)
       crates/btctax-core/src/tax/packet.rs:618 (`sch_d.must_file()`)
CLAIM: The line-7 "if not required" box is now an affirmative assertion derived from btctax's own
       incomplete model of capital activity, so it turns a lawful blank into a FALSE statement for a
       filer whose capital transactions btctax cannot see.
FAILURE: A filer with a $10,000 long-term capital gain on a partnership Schedule K-1 (Schedule D
       line 12), no crypto disposals, no 1099-DIV box 2a, no carryover. `ScheduleDLines::must_file()`
       tests only lines 3d/3e/3h, 6, 10d/10e/10h, 13, 14, 16 — btctax models none of Schedule D lines
       4, 5, 11 or 12 — so it returns false and the 1040 goes out with "If not required, check here"
       CHECKED. The filer signs a return under §6065 stating Schedule D is not required when it is.
       Before this commit the box was blank, which asserted nothing.
EVIDENCE: f1040--2024.txt line 71: "Capital gain or (loss). Attach Schedule D if required. If not
       required, check here". i1040gi--2024.txt:3260 ties the box to Exception 1: "If Exception 1
       applies, enter your total capital gain distributions (from box 2a of Form(s) 1099-DIV) on line
       7 and check the box on that line." printed.rs:814 `must_file()` enumerates ten fields, none of
       which is Schedule D line 4, 5, 11 or 12.
       ★ Note the inverse, which is SAFE and worth recording: btctax never checks the box in the one
       case i1040gi names for it, because `line13 != 0` makes `must_file()` true — so an
       Exception-1 filer gets a Schedule D attached and an unchecked box. Over-filing is permitted.
       ★ Minor because no figure moves, the failing filer is already outside btctax's documented
       surface, and their return is far more wrongly understated by the omitted gain than by the box.
```

```
SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/advisories.rs:161-170 (Advisory::AgedBoxForfeitedDeathUnanswered)
CLAIM: The message quotes the PER-BOX amount as the value of answering, even when two boxes are forgone.
FAILURE: MFJ, TY2024, both spouses born on or before 1960-01-01, both death gates unanswered ⇒
       `persons == 2`. The text renders "…is worth $1,550: your tax is currently OVERSTATED", while
       the actual forfeit is 2 × $1,550 = $3,100. `persons` is computed and used to select the
       pronoun ("you and your spouse") but never to scale `amt`.
EVIDENCE: `amt = fmt_usd(*per_box)` is interpolated twice — once correctly as "({amt} per box)" and
       once as "worth {amt}". The sibling `AgedBoxForfeitedNoDob` has the identical shape, which is
       why this is a Nit and not a defect introduced here.
```

---

`WHAT I VERIFIED AND FOUND SOUND:`

1. **Probe 1 — Form 8995 line 16 is right.** f8995--2024.txt:37/38/55 confirm line 3 is a parenthesized box (magnitude ⇒ negative), line 4 = "Combine lines 2 and 3. If zero or less, enter -0-", line 16 = "Combine lines 2 and 3. If greater than zero, enter -0-". Combining is therefore `line2 − mag3`; line 4 = `max(line2 − line3, 0)` ✓; line 16 as a printed magnitude = `|min(line2 − line3, 0)|` = `max(line3 − line2, 0)` ✓. Both `compute_8995` and `form_8995_lines` implement exactly that, and the two interact correctly (carry-in > QBI ⇒ line 4 floors at -0- **and** line 16 carries the remainder). i8995--2024.txt:557 confirms line 16 is the amount carried to next year, which is what the write-back does. Line 2 can never be negative here (`business_qbi.max(0)`, and half-SE is ~7% of profit), so the `max(0)` floors are not hiding a case.
2. **Probe 2 — the new refusal is correct, not an over-refusal.** i8995--2024.txt:41-51: Form 8995 applies only if "You have QBI, qualified REIT dividends, or qualified PTP income or loss" **and** taxable income ≤ threshold; "Otherwise, use Form 8995-A." A prior-year qualified-business-loss carryforward is QBI (i8995:326-329, 460-470 treat it as a loss from a separate trade or business). It is also exactly the pre-existing behaviour for `reit_ptp_carryforward_in` alone, which `qbi_over_threshold` has always refused above threshold. Argument order at return_1040.rs:1581-1589 is correct. Worth knowing: the refused return's tax figure would have been identical (line 4 floors at -0-), so the refusal costs the filer a return, not a dollar — but the packet would be missing the 8995-A the form demands, so refusing is the right call.
3. **Probe 3 — no combination where an unanswered gate grants the box.** `is_aged`'s arms in order: `(Some(false), _) => true` (unchanged, pre-existing); `(_, Some(dod)) => dod >= reaches_65_on(dob)`; `(Some(true), None) => false`; `(None, None) => false`. The TOML case you asked about lands in arm 2 — `(None, Some(dod))` — and arm 2 grants only when the person actually reached 65 before dying, which is the correct i1040gi answer, not an understatement. The registry-clear case (gate cleared, date left behind) lands there too, with the same correct result. I grepped every consumer of `taxpayer_died_during_year` / `spouse_died_during_year` / `date_of_death`: outside tests there are exactly two — `AgedBlindBoxes::for_return` (packet.rs:289/297) and the new advisory. No other figure reads them. The spouse tightening is consistent: `for_return` filters `joint_spouse` to `Mfj` before calling `is_aged`, so the MFJ-only liveness matches its only consumer. The oracle harness sets no `date_of_birth` on any household, so no sweep vector silently loses an aged box.
4. **Probe 4 — line J's derivation is total and correct.** `None → None`; `Some(true) → will_file`; `Some(false) → None`. That is precisely the form's "If 'Yes,'" (f1040sc--2024.txt:26, i1040sc--2024.txt:633-637). It cannot produce a J-without-I page from a TOML import, and `schedule_c.rs`'s `if let (Some(pair), Some(answer))` keeps `None` structurally distinct from `Some(false)` all the way to the widget.
5. **Probe 5 — no malformed SSN can reach a printed cell.** `ReturnHeader::build` canonicalises the taxpayer and spouse via `FiledPerson::build` (packet.rs:204/208) and every dependent (packet.rs:407/411), and `proprietor` is derived from already-canonicalised persons. `export_full_return` calls `assemble_printed_return` (which returns `HeaderError`) at admin.rs:807 — **before** `mkdir_out`, so a typo still writes zero bytes. The crypto-slice path writes no SSN at all: it calls `fill_form_8949` (no identity), `fill_form_8283` (no identity) and `fill_form_1040_capgains` (two cells). `RefuseReason` derives no `Serialize`/`Deserialize`, so removing the variant cannot break stored data.
6. Skippable registry indices 8–11 in `registries.rs` match `SKIPPABLE_QUESTIONS`' declaration order exactly (verified by reading both), and the coverage census derives each `FieldId`'s leaf path by mutate-and-diff against a pinned literal, so a transposed index could not pass.
7. `assemble_absolute`, `screen_absolute`, `assemble_printed_forms` and `apply_carryover_writeback` all pass the new `qbi_carryforward_in` in the correct positional slot; the write-back guard sits inside the `if !force` block with its siblings.
8. The 1040 line-7 box and the packet's Schedule D membership are one decision (`packet.rs:618` / `btctax-forms/src/packet.rs:124`), so a checked box beside an attached Schedule D is unreachable.
9. `apply_writes`' on-state guard is fail-open by construction (`button_on_states` returns empty for a widget with no `/AP /N`, and does not walk Kids), so it can reject only a genuinely undeclarable on-state — it cannot newly block a legitimate write.
10. `map.lines()`/`plan` are a correct 16-wide 1:1 zip; `std::ptr::eq(*cell, &map.line3)` compares two references into the same `map` binding, so the blank-when-zero gate can only ever suppress line 3.

`WHAT WOULD MAKE THIS REVIEW WRONG:` It assumes the on-state strings and field FQNs in `f1040sc.map.toml`, `f8283.map.toml` and `f8995.map.toml` were dumped from the bundled PDFs rather than inferred — you listed that as settled, and I verified the *consequences* of those values (the new `apply_writes` guard would refuse an undeclared on-state) but did not re-dump the PDFs myself.

---

## Disposition (author, same day)

The reviewer's load-bearing assumption **holds**: every on-state and FQN in this diff was produced by
`cargo run -p xtask -- dump-fields` against the bundled PDF, and the per-revision differences it would
have found are exactly what forced the three separate map shapes (Schedule C `"Yes"/"No"`, Schedule
B/D/8283 `"1"/"2"`, 8283 page-2 `f2_01`/`f2_1`/`p2-t1`).

| finding | disposition |
|---|---|
| Minor 1 — 8995 line 3 is collectable but never ASKED | **RE-FILED as §G-22.** The reviewer is right and this is my error: I closed a `gap` whose own reason said *"Closes by collecting it"*, having only made it collect**able**. Deleting the census entry removed the register's only record of a live understatement path. The residual hazard is now tracked as an INPUT-COLLECTION gap, which is its true species — the field-provenance census could never have held it, since the field IS now mapped and filled. |
| Minor 2 — the line-7 box can be a FALSE statement | **ESCALATED TO THE OWNER, not folded.** See below. |
| Nit — the advisory quotes per-box when two boxes are forgone | **FIXED.** It is a wrong dollar figure in filer-facing text, and `persons` was already computed and used for the pronoun. |

### ★★ Minor 2 is the one that matters, and it is a repeat

`must_file()` answers *"does **btctax's model** require a Schedule D?"* — not *"does the **form**
require one?"*. Those differ for any filer with Schedule D line 4, 5, 11 or 12 activity (Forms 6252,
4684, 6781, 8824, 4797, 2439, or a K-1), none of which btctax models. For them the box is now
**checked**, which asserts under §6065 that Schedule D is not required when it is. Before the commit
it was blank, which asserted nothing.

★★★ **This is the second time in two days I have replaced a lawful silence with an affirmative
statement btctax cannot support** — the first was §G-19a's `§1411 0`. Both times the change looked
like completeness and was in fact new testimony. That is worth naming as a pattern, not two accidents:
*filling a blank is not automatically an improvement, and "the form offers two states" does not mean
btctax can always tell which one is true.*
