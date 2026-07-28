# r1 adjudication (Fable)

All figures and reachability claims are now independently verified — the form and instruction text from the fetched 2024 PDFs, the vector arithmetic recomputed by hand, and the code facts (state refund, MFS, FTC, harness, goldens) checked against source. Here is the ruling.

---

# 1. VERDICT

**NOT build-ready: 5 Critical and 12 Important findings block it.** The plan's structure (two tiers, §3.2 exemption arithmetic, §3.4 no-8801 argument, Schedule 2 plumbing, four of six vectors) is sound; its Part III answer, two of its oracle numbers, its AMTI formula, and its gate design are not.

# 2. THE PART III ANSWER

**The bands are positioned by the regular return, AND the preferential slice is capped at the AMT taxable excess. The plan's §3.3 question is a false dichotomy — both candidate figures ($75,812.50 and $55,897.50) are wrong, each half-right.** Verified against f6251--2024.pdf directly:

- **Position — regular side.** Line 20: *"Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet or the amount from line 14 of the Schedule D Tax Worksheet, whichever applies **(as figured for the regular tax)**."* Line 27 repeats it for the 20% band. Contrast line 13's *"(as refigured for the AMT, if necessary)"*: the gain **amount** is AMT-side; the band **positions** are regular-side.
- **Cap — the excess.** Line 16: *"Enter the **smaller** of line 12 or line 15"*; line 22: *"Enter the **smaller** of line 12 or line 13"* — only min(taxable excess, net capital gain) flows through the 0/15/20 bands, and line 17 (= L12 − L16) floors the 26/28% slice at 0. When line 32 = line 12: *"skip lines 33 through 37 and go to line 38"* — the 20% tranche never engages.
- **Final min.** Line 40: *"Enter the **smaller** of line 38 or line 39."*

The correct rule for T1/PART_III.md: `TMT = min( tax26_28(L12 − min(L12, gain)) + preferential_tax(regular-side bottoms, min(L12, gain)), tax26_28(L12) )`.

**Exact figures, computed line-by-line (Reviewer 1's arithmetic is right; Reviewer 5's ~110,090/~116,255 are wrong):**

- **§8 V2 as printed** (wages 1,000,000 / LTCG 500,000 / donation 750,000, TI 750,000): AMTI 750,000; exemption 133,300; L12 = 616,700; L16 = 500,000; L17 = 116,700 → L18 = 30,342.00; L21 = 0; L30 = 333,750 → L31 = 50,062.50; L33 = 166,250 → L34 = 33,250.00; L38 = 113,654.50 < L39 = 168,024.00. **TMT = $113,654.50. AMT = $0** (regular 129,397.50). Fill the blank with this.
- **The §3.3 prose vector is a *different* return** — its "$1M wages / $10M gain / $1M donation" label is wrong. The figures $75,812.50/$55,897.50 arise only from wages 1,000,000 / **LTCG 500,000** / donation 1,000,000 → capped at 900,000 (60% × AGI 1,500,000), TI 600,000: L12 = 466,700 < gain 500,000, so L16 caps at 466,700, L17 = 0, all 466,700 lands in the 15% band from regular bottom 100,000 → L31 = 70,005.00, L32 = L12 → 20% skipped. **TMT = $70,005.00** ($75,812.50 ignores the cap; $55,897.50 misplaces the bands). Pin this vector as its own KAT — it is the only excess-<-gain / L32-skip exemplar.
- The **literal** $10M-gain taxpayer has ordinary TI $0 in both systems, so both readings coincide: TMT = regular tax = $1,956,705.00, AMT $0. There is no "$19,915 spread on one return."

# 3. BLOCKING FINDINGS

**C-1 (§3.3 / §8 V2) — Part III rule wrong; three vectors conflated.** As above. Encoding $75,812.50 ships a rule that overstates TMT whenever taxable excess < net capital gain (missing the L16/L22 caps and the L32 skip). Replace §3.3's resolution paragraph with the rule + figures in section 2; fill V2's TMT = 113,654.50; add the TI-600,000 vector (reg 87,918.50 / TMT 70,005.00 / AMT 0) as a KAT.

**C-2 (§8 V5, §1) — "≈$28,000" is the wrong-reading figure; exact is $26,271.00.** Recomputed: regular 420,929.50; AMTI 2,250,000; exemption 0; L17 = 250,000 → 65,348.00; L31 = 54,442.50; L34 = 327,410.00; TMT 447,200.50; **AMT = 26,271.00**. The AMT-slice misreading gives exactly 27,731.00 ≈ 28,000 — the plan's own number reproduces the stacking it disfavors, and §8 says "use verbatim as KATs." Replace both occurrences with 26,271.00.

**C-3 (§3.1, T2) — AMTI formula omits the line 2b state-refund subtraction, and the input is live.** Form 6251 line 2b: *"Tax refund from Schedule 1 (Form 1040), line 1 or line 8z"* (negative); i6251 p.5: *"Enter the total as a negative amount."* `ri.sch1.state_refund_taxable` (return_inputs.rs:317) is passed to the screen at return_1040.rs:1426, and amt.rs's own `line5 = line3 − state_refund_and_8z` already subtracts it — the plan's formula reproduces worksheet lines 1–3 and drops lines 4–5. Tier 2 would file an overstated Form 6251/Sch 2 L2; Tier 1 over-refuses. §3.1 must read: `AMTI = taxable_income_L15 + amt_worksheet_line2(...) − state_refund_taxable`, and compute_6251 takes the refund. Add one refund-bearing vector to §8 (all six have refund = 0). Note: line 2b is outside Who-Must-File condition 4's "lines 2c through 3," so it never by itself forces an attach.

**C-4 (§3.1/§3.2, T2) — the MFS line-4 AMTI kicker is missing.** i6251 p.9, Line 4, verified verbatim: *"If your filing status is married filing separately and line 4 is more than $875,950, you must include an additional amount on line 4. If line 4 is $1,142,550 or more, include an additional $66,650. Otherwise, include 25% of the excess of the amount on line 4 over $875,950."* (§55(d)(3).) MFS is a live status (types.rs:13; refused only while `MfsSpouseItemizeUnknown`), and T2's signature takes `status`. Omission understates AMTI by up to $66,650 → TMT by up to ~$18,662 → **understated filed tax**. Add the kicker with both constants in `AmtParams`, MFS boundary KATs at $875,950/$1,142,550, and (fold-time) check the existing screen against the same rule — it lacks it too.

**C-5 (T1 + §8 + §9 rows 1 and 5) — the gate guarding the plan's one acknowledged unknown cannot detect the wrong answer.** V1/V3/V4/V6 have regular ordinary bottoms of 915,000/970,800/670,800/750,000 — all above L25's $583,750 — so both readings yield identical TMTs; V3 is *mathematically insensitive* and cannot be §9 row 1's canary. V2's AMT is $0 under both readings (only its TMT field discriminates), and V5 is recorded only as "≈". Meanwhile §9 row 5 ("oracle can't validate AMT") is false in this tree: `scripts/oracle/gen_goldens.py:215–223` already runs Tax-Calculator and reads `c09600` — an independent Form 6251 including Part III. T1 must: assert `tentative_minimum_tax` (not `amt`) on V2; pin V5 and the TI-600,000 vector to the cent from independently derived figures recorded *before* T1 chooses; strike V3 from row 1; cross-check §8 against `c09600` inside T1.

**I-1 (T2/T3/T7/T9, §3) — lines 8–10 unmodeled; attach boundary is line 7 vs line 10, not AMT > 0.** i6251 p.1, Who Must File, verified: *"Attach Form 6251 to your return if any of the following statements are true. 1. Form 6251, line 7, is greater than line 10."* i6251 p.10, Line 8: *"your AMTFTC is the same as the foreign tax credit on Schedule 3 (Form 1040), line 1"* (§904(j) elector), and *"If the amount on line 10 is greater than or equal to the amount on line 7 … Leave line 8 blank and enter -0- on line 11."* v1 accepts an FTC up to $600 (`box6_foreign_tax`, return_inputs.rs:75). Net AMT is invariant (line 8 and line 10's Sch-3-L1 subtraction cancel), but: (a) printed lines 8/9/10 are each wrong by the FTC if `amt = max(0, TMT − regular_tax)` is filled as-is; (b) in the window `1040 L16 − FTC < line 7 ≤ 1040 L16`, AMT = $0 yet condition 1 requires attaching — T3's proceed-with-no-form and T9's skip-when-$0 are wrong there; (c) `regular_tax` is undefined — passing 1040 L24 would overstate AMT by NIIT + Additional Medicare ($25,750 on V1 alone). Define `regular_tax` = Form 6251 line 10, fill lines 8/9/10 explicitly, make the Tier-1/Tier-2 boundary the Who-Must-File conditions (only condition 1 is reachable in v1), and add one FTC-bearing §8 vector.

**I-2 (§3.3 last paragraph) — the upper-bound rejection has a false rationale.** The regular-position stack on the full gain IS an unconditional upper bound on line 40 (regular-side positions per L20/L27; preferential tax monotone in the amount, and L16/L22 only shrink it; L40's min only lowers further) — and at V3, excess ≥ gain makes it exact (2,275,348.00), so it does not "fail exactly where the margin is thinnest." Keep the rejection; restate the reason: Tier 2 must fill lines 12–40 exactly anyway, T3's message names the exact dollar, and a second approximate path adds risk for nothing. Delete the "only valid while the add-back is smaller than the exemption" sentence — that misconception is what produced $75,812.50.

**I-3 (§1) — the exposure narrative's figures are wrong and internally inconsistent.** The $24,615 "peak" is the zero-add-back value at 384,000 (past the kink); the zero-add-back peak is 24,619.00 at ordinary TI 383,900, and the peak grows by 0.28 × add-back: **32,795.00 for a standard-deduction filer** (V5's 26,271 already exceeds the claimed peak). The $769,139 crossover is the zero-add-back case only (≈800,250 SALT-capped; ≈859,983 standard — V3/V4 straddle *that*, not 769,139). State both as add-back-dependent or a wrong sanity bound gets encoded.

**I-4 (§8 V1) — balance due $83,225.50 is ambiguous by exactly $7,200.** It is correct only if payments = box-2 $300,000 **plus** mandatory Additional-Medicare withholding of $7,200 (0.9% × $800,000, Form 8959 line 24 → 1040 line 25c). State the withholding composition in the KAT.

**I-5 (§2/§3.1) — the imported capital-loss carryforward can carry a divergent AMT twin.** i6251 Line 2k covers *"Capital gain or loss (including any carryover that is different for the AMT)"*. `capital_loss_carryforward_in` is a declared, externally-originated input — neither "refused upstream" nor "never captured," so §2's dichotomy is false for it (Reviewer 3's no-interaction proof covers only btctax-tracked history). State the equal-for-AMT assumption explicitly and back it with a declaration/attestation or a refusal; add it to §9's guard list. Direction when wrong: understates.

**I-6 (§2) — mortgage interest on a non-qualified dwelling is a missing §56(b)(1)(C) add-back.** i6251 p.8, Line 3: interest on a dwelling that *"isn't a principal residence … or qualified dwelling for AMT"* (houseboats, RVs, transient use) is added back; the 8a input has no dwelling question. Add a qualified-dwelling declaration (None ⇒ refuse, matching the mixed-use pattern) or document precisely why 8a-only excludes the case; add to §9's guards. Direction when hit: understates.

**I-7 (T5, §4, §9 row 5) — the harness edit is wrong and the real AMT exclusion lives elsewhere.** main.rs:704–706 is a single combined `screen_absolute` check covering three refusal classes; deleting it admits QBI-over-threshold and TI≤0 returns to the sweep, and it is a no-op for zero-AMT returns after T3 anyway. The binding exclusions are `gen_goldens.py:257–260` (rejects any `c09600 != 0`) and `corpus.py`'s domain caps — neither in §4 or any task. Narrow the check to the AMT reason only, add both scripts to scope, and replace §9 row 5's mitigation with the c09600 cross-check (in T1).

**I-8 (T2/T7) — five scalars cannot fill ~30 printed boxes.** Part III alone is lines 12–40, each a printed box, and §5 requires per-line rounding; §10 also ships `Amt6251` as public API in Tier 1, forcing a Tier-2 breaking change. T2 must emit the full line vector (or `#[non_exhaustive]`, stated). Also state the L38-vs-L39 min — §3.3's summary omits it.

**I-9 (T9 vs §5/§9 row 3) — "Remove `RefuseReason::AmtOwed`" deletes the fail-closed path both sections require**, and removing a public variant contradicts §10's "Tier 2: MINOR." Keep the variant, narrow its trigger.

**I-10 (T4/T9) — both packet assertions are vacuous.** "No Form 6251 in the packet" is trivially true before the emitter exists, and no Tier-2 task re-asserts the skip; every bundled journey is deliberately sized under the screen (testonly.rs:48–51, 58–59), so regeneration proves nothing. Add a screen-tripping zero-AMT journey to T4 and a Tier-2 skip KAT with T3-style mutation discipline to T9.

**I-11 (§4/T3) — the file map misses the refusal's blast radius.** `btctax-input-form/src/attribute.rs:144` exhaustively anchors every `RefuseReason` (hand-enumerated test at :348), and clearing the Hard blocker un-gates report/harvest/what-if/conservative-promote/TUI blockers. Add attribute.rs to §4; T3 enumerates the flipped surfaces and their goldens.

**I-12 (§5/§9 row 2) — the source-scan guard and no-literal-constant rule appear in no task.** §3.1's exhaustiveness is load-bearing and guarded by nothing executable. Make both Tier-1 test items under T2.

# 4. CONFIRMED CORRECT

- **Part III positioning is regular-side** — the plan's directional reading of line 20 was right (it missed only the L16/L22 caps).
- **§3.4 stands in full**: §53(d)(1)(B)(ii)(I) specifies all of §56(b)(1) (taxes AND standard deduction) as exclusion items; Form 8801 Part I line 15 = the entire AMT, lines 18/21 = $0; per i8801 Who Should File the filer isn't even directed to complete the form; the §904(j) FTC cancels symmetrically (i8801 Line 12). Conditional only on I-5's attestation and I-12's guard. Scope the sentence to "no NEW obligation" (a pre-btctax 8801 carryforward is an already-unsupported input).
- **V1, V3, V4, V6 verify to the cent** (I recomputed all four): 364,675.50/327,965.00/0; 2,285,321.50/2,275,348.00/0 margin 9,973.50; V4 fills = reg 2,175,529.50, TMT 2,191,348.00, AMT 15,818.50; 2,203,625.50/2,205,348.00/1,722.50. V2's printed row (TI 750,000, reg 129,397.50, AMT 0) is also exact. V1's NIIT 19,000 and AddMed 6,750 check.
- **§3.2's exemption/phase-out arithmetic and every 2024 constant** (133,300 / 1,218,700 / 1,751,900 / 232,600 / 94,050 / 583,750) — verified against the form and the i6251 p.9 Exemption Worksheet note.
- **§199A stays subtracted** (§199A(f)(2); line 1 starts net of QBI, no add-back line).
- **Charitable can never diverge**: §57(a)(6) repealed; i6251 Related Adjustments (p.9) triggers only on 2c/2d/2h/2i/2k–2t/line-3 items and only for non-AGI-based limits; 60% ceilings and itemize-vs-standard are handled correctly in every vector.
- **Part I lines 2c–2t and 3 are all refused or input-less** in v1 (line-by-line walk confirmed); with C-3/C-4/I-1/I-5/I-6 folded, §3.1 is genuinely exhaustive.
- **T8's plumbing is exactly right**: 6251 L11 → *"Enter here and on Schedule 2 (Form 1040), line 2"*; Sch 2 L3 = L1z + L2 → 1040 L17.
- **The excess-<-gain case needs no special code** — L16/L17/L22 and the L32 skip handle it structurally (but it must gain a KAT: the TI-600,000 vector).
- **§1's crossover rule is a valid sufficiency** for the zero-add-back case, and the V6 donation-triggers-AMT mechanism is real.

# 5. NON-BLOCKING

- §3.2: line 6 needs `max(0, …)` and the enter-0-on-7/9/11 branch (form line 6 text).
- §3.1: record line 1's zero-TI negative-amount branch and its one-line unreachability proof.
- §8 lacks any vector with regular ordinary income < $94,050 — the 0% band (and T1's "a failing test per band") is unconstructible as written.
- T5's "record how many households" is not an assertion; make it a numeric floor.
- §1's "byte-identical to today" is uncheckable (today writes no packet); use T4's hand-built-packet phrasing.
- Add a Tier-1 `report`/TUI line showing AMTI/exemption/TMT/AMT so the filer can see the number that un-refused them.
- Carry R3's `debug_assert` invariant (every AMT adjustment is a §56(b)(1) exclusion item) plus an 8801-recompute KAT.
- T1 should quote lines 20/27 vs line 13's parentheticals in PART_III.md.

# 6. THE ONE THING MOST LIKELY TO PRODUCE A WRONG FILED NUMBER

Encoding the plan's own §3.3/§8 numbers as the oracle: $75,812.50 bakes in a Part III that ignores the line-16/22 excess caps, and V5's "≈28,000" is within rounding of exactly the wrong-stacking answer (27,731.00) — so the KAT suite would *certify* the misreading while every insensitive vector stays green. Fix C-1/C-2/C-5 together, pinning V2's TMT ($113,654.50), V5 ($26,271.00), and the TI-600,000 cap exemplar ($70,005.00) before T1 writes any code.
