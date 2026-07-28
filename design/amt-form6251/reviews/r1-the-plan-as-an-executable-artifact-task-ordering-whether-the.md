# r1 review — The plan as an executable artifact — task ordering, whether the gates would actually red on a mistake, tier boundaries, and completeness of the task list (NOT tax law).

**Headline:** The plan's one acknowledged unknown (§3.3 Part III positioning) is guarded by a test set that provably cannot detect the wrong answer — not one of the six §8 vectors' AMT column changes between the two candidate readings — while the independent oracle that would settle it in minutes already runs in this tree and is written off in §9 as unavailable.

## [Critical] T1 + §8 + §9 row 1 — UNSOUND

**Claim:** "Encode the resolution as three KATs against the worked vectors in §8 before writing any code, and watch them fail. Deliverable: the ambiguity is closed in writing, with a failing test per band." (T1) — with §9 row 1's early warning "V2/V3 KATs disagree with a hand-check".

**Authority:** Form 6251 (2024) line 25 fixes the 20% band at "$583,750 if married filing jointly"; line 20 takes "the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet … (as figured for the regular tax)". The two candidate readings can only diverge when that regular bottom (taxable income − preferential income) is BELOW $583,750. §8's bottoms: V1 = 1,415,000−500,000 = 915,000; V3 = 10,970,800−10,000,000 = 970,800; V4 = 10,670,800−10,000,000 = 670,800; V6 = 10,750,000−10,000,000 = 750,000 — all above it, and (exemption fully phased out in each, AMTI > $1,751,900) the AMT bottom is higher still, so BOTH readings tax the whole gain at 20% and yield an identical TMT. V2 is sensitive but its TMT (~110,090 vs ~116,255) is under regular tax 129,397.50 either way, so its **AMT is $0 under both readings**. V5 is sensitive by exactly 5% × the $29,200 add-back = $1,460, but is recorded only as "≈28,000". Net: the bold AMT column of every §8 vector is unchanged by the wrong answer.

**Fix:** State in T1 that (a) V3 is mathematically insensitive and cannot be the canary — strike it from §9 row 1; (b) the discriminating assertion is `tentative_minimum_tax`, not `amt`, so T2's KATs must assert the TMT field for V2; (c) V2's TMT and V5's AMT must be pinned to the cent by an independently derived figure recorded BEFORE T1 chooses, otherwise T1 authors both the answer and the only test of it; (d) add one vector with regular ordinary income below $94,050 so the 0% band is exercised at all. Settle the reading against Tax-Calculator (see next finding), not against T1's own arithmetic.

---

## [Important] T5 + §4 file map + §9 row 5 — WRONG

**Claim:** "Remove the `return None` at `oracle-harness/src/main.rs:705` for the AMT case; let AMT-screened returns into the differential sweep" / file map: "stop returning `None` on AMT (`main.rs:705`) — expands the sweep domain" / §9 row 5 mitigation: "treat §8 as the oracle for Tier 1".

**Authority:** `crates/btctax-oracle-harness/src/main.rs:704-706`: a single `if screen_absolute(...).is_some() { return None; }` — and `screen_absolute` carries three reasons, not one (`crates/btctax-cli/src/cmd/tax.rs:325`: "screen_absolute (QBI-over-threshold / AMT / TI≤0-with-carryforward)"). Deleting it admits the other two classes of refused return into the sweep. It is also a no-op for its stated purpose: after T3 a zero-AMT return no longer refuses, so it already passes line 705 untouched. The actual AMT exclusion is at bake time — `scripts/oracle/gen_goldens.py:257-260`: "The AMT/credit SUBSTANCE check applies to EVERYONE (anchors included) … if amt or credits: rejected.append(… f\"taxcalc AMT c09600={amt} …\")" — and `scripts/oracle/corpus.py:71-92` caps the domain at W-2 $270,000 / LTCG $20,000 / SE $120,000, far below the $1,218,700 phase-out start. Neither file is in §4 or any task. Critically, §9 row 5 is false about this tree: `gen_goldens.py:214-223` already runs Tax-Calculator and reads `c09600` — an independent Form 6251 implementation including Part III.

**Fix:** Rewrite T5: (1) NARROW main.rs:705 to the AMT reason only, keeping the QBI and TI≤0 refusals out of the sweep; (2) add `scripts/oracle/corpus.py` and `scripts/oracle/gen_goldens.py` to §4 and to T5's scope — the corpus needs an AMT-region axis and the admission loop needs to stop rejecting `c09600 != 0`; (3) replace §9 row 5's mitigation with the real one: drive `c09600` on the §8 vectors to settle §3.3 and to hold Part III thereafter. Do (3) inside T1, before any code.

---

## [Important] T2 / T7 (tier boundary) — INCOMPLETE

**Claim:** T2: "pub struct Amt6251 { amti, exemption, taxable_excess, tentative_minimum_tax, amt: Usd }" — T7: "filling Parts I–III from `Amt6251` + `ScheduleAParts`".

**Authority:** Form 6251 (2024) Part III is lines 12–40, every one a printed box: line 13 (QDCG line 4), 16 "Enter the smaller of line 12 or line 15", 17, 18, 19, 20, 21, 22–24, 26–34, 35–37, and 38/39/40 — line 40 "Enter the smaller of line 38 or line 39 here and on line 7". Part I adds lines 1, 2a, 3, 4; Part II adds 5–11. Five scalars cannot fill ~30 boxes, and PLAN §5 ("the printed chain rounds at the line and re-adds rounded lines") requires per-line values. §10 also ships `Amt6251` as public API in Tier 1, so Tier 2 must then add fields to a released struct.

**Fix:** Move the full line vector into Tier 1: T2 emits a `Form6251Lines`-shaped result (or `Amt6251` is `#[non_exhaustive]` and the plan says so). Also state the line-39/40 cap explicitly — §3.3's summary ("taxes the capital gain at §1(h) rates and the remainder at 26/28%") omits that TMT is the SMALLER of that computation and a flat 26/28% on line 12.

---

## [Important] T2 / T7 / §8 — INCOMPLETE

**Claim:** "`amt = max(0, TMT − regular_tax)`" with `regular_tax` an undefined parameter of `compute_6251`, and no §8 vector carrying a foreign tax.

**Authority:** Form 6251 (2024) line 10: "Add Form 1040 or 1040-SR, line 16 (minus any tax from Form 4972), and Schedule 2 (Form 1040), line 1z. Subtract from the result Schedule 3 (Form 1040), line 1 and any negative amount reported on Form 8978, line 14" — Schedule 3 line 1 is the foreign tax credit, and that IS a live v1 input (`crates/btctax-core/src/tax/return_inputs.rs:75` "box6_foreign_tax // → §904(j) FTC (§4.7a)"; only amounts ABOVE the ceiling refuse, as `ForeignTaxOverCeiling`). Line 8 is the AMTFTC and line 9 = line 7 − line 8. `crates/btctax-core/src/tax/amt.rs:40-47` already records why the two cancel for the ≤$300/$600 passive credit — the plan does not.

**Fix:** Define `regular_tax` in T2 as Form 6251 line 10 and cite it (a reader could plausibly pass 1040 L22 or L24 — L24 would understate AMT by the NIIT + Additional Medicare, $25,750 on V1 alone). State the line-8/line-10 cancellation, carrying amt.rs's existing argument forward. Add one §8 vector with a §904(j) foreign tax so T7 is forced to print line 8 = the FTC and line 10 net of it — otherwise the emitted form's own line 11 (= 9 − 10) disagrees with 1040 L17 by the FTC, which is exactly what the Tier 2 gate claims to check and would not catch.

---

## [Important] T4 / T9 (mutation discipline) — UNSOUND

**Claim:** T4: "Assert that for a zero-AMT return the printed 1040/Schedule 2 are identical … no Form 6251 in the packet" and "Regenerate docs/examples/examples.md and the TUI goldens; a zero-AMT journey must show no diff beyond the newly-computable return itself." T9: "the form is **skipped** when AMT is $0 (Who Must File)."

**Authority:** Both halves are vacuous. (a) "No Form 6251 in the packet" is trivially true for all of Tier 1 — the emitter does not exist until T7 — and no Tier-2 task re-asserts it, so T9's skip-when-$0 guarantee ships with nothing that would red if the skip were deleted. (b) Every bundled journey is deliberately sized below the screen: `crates/btctax-cli/src/testonly.rs:49-51` "the kitchen-sink household clears the 2024 Form-6251 AMT-screen worksheet by only a thin margin — a corpus editor who enlarges the sale, income, or donation must keep the household on the computable side of that screen", and :58-59 "Amounts kept small so the return stays under the AMT screen." The regeneration will therefore show no diff and prove nothing.

**Fix:** T4 must ADD a journey/example household that trips the screen with AMT $0 (that is the whole Tier-1 population) rather than only regenerating. T9 must carry its own KAT — a zero-AMT export contains no `form_6251.pdf` — with the same mutation check T3 has (delete the skip; the KAT must red).

---

## [Important] §4 file map / T3 — INCOMPLETE

**Claim:** The file map lists four core files, printed.rs, the forms crate, cli, the oracle harness and docs as the whole surface of the change.

**Authority:** `crates/btctax-input-form/src/attribute.rs:144` exhaustively matches every `RefuseReason` to an `Anchor` (`R::AmtScreenTriggered => vec![Anchor::NotInForm { note: "the Form 6251 AMT screen is computed at `report`, not a v1 form field" }]`); `attribute.rs:335-360`'s test enumerates the variants BY HAND, so a new variant compiles-then-escapes coverage. `crates/btctax-tui-edit/src/edit/tax_inputs.rs:768 focus_refusal` consumes that anchor to drive the TUI's focus jump and `!` glyph. Separately, the un-refusal's blast radius is wider than the packet: the AMT refusal reaches `TaxOutcome::NotComputable` / `BlockerKind::TaxYearNotComputable` (`crates/btctax-core/src/tax/compute.rs:252`), a projection-wide Hard gate, so Tier 1 also un-gates harvest/what-if (`WhatIfError::YearNotComputable`), `conservative_promote.rs:206`, and the TUI blocker list for this population. (Reassuring counterpart: `return_refuse.rs` derives no Serialize/Deserialize, so there is no persisted-refusal migration — a stored `ReturnInputs` simply starts computing.)

**Fix:** Add `btctax-input-form/src/attribute.rs` to §4 and have T3 state the `AmtOwed` anchor and add the variant to attribute.rs's hand-written test list. Add a T3 sub-item enumerating the surfaces that flip when the Hard blocker clears (report, harvest/optimize/what-if, conservative-promote, TUI blocker list) and say which goldens each requires.

---

## [Important] T9 vs §9 row 3 vs §5 — WRONG

**Claim:** T9: "Remove `RefuseReason::AmtOwed`."

**Authority:** §9 row 3's mitigation is "Tier 2 refuses instead of filing until 8801 exists", and §5 states "If Part III is uncertain for an input, refuse rather than guess low" and "When in doubt, refuse." Both require a refusal reason to exist in Tier 2; T9 deletes the only one. Deleting a public enum variant is also a breaking change, contradicting §10's "Tier 2: MINOR".

**Fix:** T9 keeps `AmtOwed` and narrows its trigger (AMT owed AND something the emitter cannot fill / §3.4's argument disturbed), or the plan names the replacement fail-closed reason. Fix §10 accordingly, or state that the variant is retained precisely so Tier 2 stays MINOR.

---

## [Important] §5 + §9 row 2 — INCOMPLETE

**Claim:** §5: "no literal AMT constant may appear outside the tax tables"; §9 row 2 mitigation: "a source-scan guard asserting §2's list is still refused".

**Authority:** Both are stated as constraints/mitigations and neither appears in any task (T1–T9). §3.1's exhaustiveness — "Those are the only two §56(b)(1) add-backs reachable in v1; §2's exclusion list is why" — is load-bearing for AMTI being correct at all, and it is guarded by nothing executable. §9 row 2's stated early warning ("a new 1099/W-2 input lands without an AMT review") is not observable by any mechanism the plan creates; the guard IS the warning.

**Fix:** Make the source-scan guard a Tier 1 task item under T2 (a test asserting each §2 preference still has a live refusal path), and add the no-literal-constant grep test alongside it. An unowned mitigation in a risk table is not a mitigation.

---

## [Minor] §1 table / T4 — UNSOUND

**Claim:** Tier 1 printed output is "**Byte-identical to today** — Sch 2 L2→L3→1040 L17 are already $0".

**Authority:** §1's own second paragraph: today btctax "refuses the entire return and writes no forms at all". There is no "today" packet for the Tier-1 population to be byte-identical to, so the phrase is not a checkable acceptance criterion.

**Fix:** Say what T4 actually means: the packet equals the one produced for an otherwise-identical household that never tripped the screen (or a hand-built expected packet). Keep the checkable form in T4 and drop the byte-identity phrasing from §1 so it cannot become the test.

---

## [Minor] T5 — UNSOUND

**Claim:** "Confirm the sweep still reconciles; record how many previously-skipped households it now covers."

**Authority:** Neither clause can fail. The sweep reconciles trivially (see the T5 finding above — nothing changes for it), and "record" is not an assertion, so a count of zero passes the gate silently while the plan reads as if AMT coverage were gained.

**Fix:** Turn it into a numeric assertion the suite enforces: at least N households whose Form 6251 screen fires are admitted and reconcile, with the per-household AMT compared against taxcalc `c09600`.

---

## [Minor] T1 — INCOMPLETE

**Claim:** "a failing test per band" / "three KATs against the worked vectors in §8".

**Authority:** Form 6251 (2024) line 19 sets the 0% band at "$94,050 if married filing jointly". No §8 vector has regular ordinary income anywhere near it (lowest is V5 at 220,800), so the 0% band cannot be exercised from §8 at all, and only V2/V5 touch the 15% band. Three band KATs are not constructible from the vectors T1 is told to use.

**Fix:** Add the missing low-ordinary-income vector to §8 (or say the 0% band is unreachable for the population and drop the per-band claim).

---

## [Minor] Tier 1 gate / missing task — INCOMPLETE

**Claim:** "Tier 1 gate: full suite green, 0C/0I, and a zero-AMT return exports a complete packet."

**Authority:** No task changes any user-visible output to show that an AMT computation happened. After Tier 1 the filer's return turns from refused to filable on the strength of a number (AMTI / exemption / TMT) that appears nowhere in `report --tax-year`, the packet, or the TUI. The plan asks §5 to guarantee "never understate" while giving the filer nothing to check it against.

**Fix:** Add a Tier 1 item: `report --tax-year` (and the TUI tax panel) print the computed AMTI / exemption / TMT / AMT = $0 line, with the goldens to hold it. This is also the artifact a reviewer or the filer needs when the §3.3 reading is later revisited.

---

## [Nit] §3.1 — CORRECT

**Claim:** "After `fix/amt-screen-line2`, worksheet line 3 **is** AMTI for every input v1 accepts: AMTI = taxable_income_L15 + amt_worksheet_line2(...)" — headed "already exact".

**Authority:** Form 6251 (2024) line 1: "Enter the amount from Form 1040 or 1040-SR, line 15, if more than zero. If Form 1040 or 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from line 11 … and enter the result here. (If less than zero, enter as a negative amount.)" The plan's formula ignores that branch, but it is harmless: both reachable add-backs ($10,000 capped SALT, $29,200 standard deduction) are below the unphased exemption of $133,300 (line 5), and a zero-taxable-income return cannot reach the $1,218,700 phase-out start, so line 6 is $0 either way.

**Fix:** Record the one-line unreachability proof in `PART_III.md` rather than leaving "exact" to be rediscovered — a future input that raises the add-back above the exemption reopens it.

---

## [CONFIRMED_CORRECT] T8 / §1 table — CORRECT

**Claim:** "`Schedule2Lines.line2` = AMT, `line3` = Part I total → 1040 **L17**; L18 = L16 + L17."

**Authority:** 2024 Schedule 2 Part I: "1z Add lines 1a through 1y"; "2 Alternative minimum tax. Attach Form 6251"; "3 Add lines 1z and 2. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 17". Form 6251 (2024) line 11: "AMT. Subtract line 10 from line 9. If zero or less, enter -0-. Enter here and on Schedule 2 (Form 1040), line 2." Every line number in T8 is right.

**Fix:** None.

---

## [CONFIRMED_CORRECT] §3.3 — CORRECT

**Claim:** "Part III line 20 pulls 'the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet' — a figure from the **regular** computation — which indicates the bands are positioned by the regular bottom, making $75,812.50 the correct one."

**Authority:** Form 6251 (2024) line 20: "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet or the amount from line 14 of the Schedule D Tax Worksheet, whichever applies (as figured for the regular tax)…" and line 27 repeats the same source for the 20%-band position, while line 17 (= line 12 − line 16) keeps the 26/28% slice on the AMT base. The question T1 must answer is answerable from the form in one sitting; what is broken is the gate around it, not the reading.

**Fix:** None — but this is the sentence T1 should quote, and it makes the Tax-Calculator cross-check cheap rather than exploratory.

---

