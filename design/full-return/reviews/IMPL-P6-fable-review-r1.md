# P6 GATE REVIEW — full-return PDF fillers + packet (`b40bdec..HEAD`) — Fable r1

Persisted verbatim before folding, per STANDARD_WORKFLOW §2. Requested at HEAD `eef5e5f`
(P6.1–P6.5 complete; 1676 passing / 0 failed).

**VERDICT: 0 Critical / 9 Important / 8 Minor / 5 Nit** — the gate is OPEN. Fold, then re-review.

---

**Reviewer:** Fable (independent; previously architect on this phase — reviewed against my own prior rulings adversarially, and overruled or tightened them where warranted).
**Gates re-verified by me, not taken on faith:** `cargo test --workspace --locked` = 1676 passed / 0 failed (exit 0), `cargo clippy --workspace --all-targets --locked -- -D warnings` = 0 (exit 0), `cargo fmt --all -- --check` clean, FROZEN files (`tax/{types,compute,se}.rs`) = 0 bytes changed since `059ec2a`. The suite is green; every finding below is a code/spec defect, not a red gate.

**Summary judgment:** the core printed-chain architecture is genuinely good — the composition rule is implemented where it was specified, the discriminating KATs are real (I fault-checked the KAT-9-class fixtures: `.50/.50` Sch 2 L4, the 302≠301 8949 totals, the cross-PDF Schedule D↔8949 cell-text oracle — none is vacuous), the SSN compute/packet split is sound, and the extension-payment Critical is correctly fixed and pinned. But the citation audit I prescribed — and which the implementer did not run to completion — finds the packet is **not yet a return whose every form is internally and mutually consistent**: the 1040's own line 1a is blank under a filled 1z, the filed L16 can contradict the Tax Table applied to the filed L15, the packet's Form 8949 is unnamed, the full Schedule D dropped the QOF answer the slice has always given, and Schedule A line 2 violates the citation-composition rule on its face. The dispatch is logically airtight but its non-overlapping-filenames guarantee — the condition I attached to deleting the P5-C1 refusal — is false as claimed in three places.

---

## Findings

### Critical — none

### Important

**I1 — 1040 line 1a is never filled; wages appear only on line 1z.**
`crates/btctax-forms/src/form1040_full.rs:108–121` fills `map.line1z` only; `crates/btctax-forms/forms/2024/f1040.map.toml:44` maps `line1z = f1_41` and has **no key for f1_32 (line 1a)** — I dumped the blank PDF and correlated: f1_32 is line 1a ("Total amount from Form(s) W-2, box 1"), f1_41 is 1z ("Add lines 1a through 1h").
Why wrong: the filed 1z prints the household's wages above an **empty operand column** — the form's own instruction "Add lines 1a through 1h" sums blanks to 0 ≠ 1z. This breaks §3.1's cross-foot guarantee *on the 1040 itself*, on the very line the Service document-matches against W-2s. SPEC §5 stage 1 says "1a=Σ box1" in terms. Same class as the aged/blind gap (a form contradicting its own arithmetic), which this phase treated as gating.
Fix: map `line1a = f1_32`, write the same figure to both 1a and 1z (`Form1040Lines` can keep one field; the filler writes two cells), and extend the 1040 read-back KAT to assert 1a non-empty.

**I2 — Printed 1040 line 16 is derived from the EXACT taxable income, not the printed L15; the Tax Table's $50 bins make the filed L16 contradict the filed L15.** *(Borders Critical; blocks regardless.)*
`crates/btctax-core/src/tax/printed.rs:530` — `line16 = round_dollar(ar.regular_tax)`, where `ar.regular_tax = qdcgt_line16(exact-cents TI, …)`. `method.rs:21–27` bins the input TI into $50 bins; its own doc says the bin rule is defined "for a whole-dollar taxable income". The printed L15 = (printed L11 − printed L14) can legitimately differ from the exact TI by a few dollars, and when the two straddle a bin edge the filed L16 differs from `TaxTable(filed L15)` by a full bin step — up to $50 × 37% ≈ **$18.50**, not the $1 rounding residual §3.1 accepts. Concrete: wages $61,749.80, Single, std deduction → exact TI 47,149.80 (bin [47,100–47,150)), printed L15 = 61,750 − 14,600 = **47,150** (next bin). Filed L16 = Table(47,149.80); the Service recomputes Table(47,150) and issues a math-error notice for the $11. Line 16 vs Table(line 15) is the single most-recomputed arithmetic on a transcribed return.
Why this is not covered by my ARCH-P6.3a acceptance: I accepted **$1-bounded multiply-line residuals** (SE L13, 8959 L7). A step function with $50 treads is a different animal — the residual is unbounded past tolerance and asymmetric to no one's benefit.
Fix: the printed chain must compute L16 **from the printed operands** — `printed::form_1040_lines` calls `method::qdcgt_line16(printed L15, printed 3a, printed Sch D net LTCG figure)` (the QDCGT worksheet's own inputs are the printed 1040/Sch D lines a human copies into it). The exact-cents `ar.regular_tax` remains the *computed* liability; only the filed cell changes. Discriminating KAT: the 61,749.80 fixture above, asserting printed L16 == Table(printed L15) ≠ round(exact tax).

**I3 — The packet's Form 8949 carries no name and no SSN.**
`crates/btctax-forms/src/packet.rs:130` — `crate::fill_8949_full(p, year)?` takes no header; `fill8949_full.rs:74–81` and `lib.rs:277–283` confirm no identity write. The 2024 8949 has "Name(s) shown on return" + SSN cells on **both pages** (f1_1/f1_2, f2_1/f2_2, `/MaxLen 11` — I dumped them). ARCH-P6.3a Q2 required, in terms: "Identity: `Option<IdentityCells>` on `Form8949Map` (name_line + taxpayer SSN), fail-closed on the full path." FOLLOWUPS claims `p6-form-identity-header` CLOSED ("All nine schedules + the full Schedule D + the 1040 now print their identity") — the 8949 is the form that was missed, and the test named `every_schedule_carries_the_name_and_ssn_header` (full_return_forms.rs:1539) tests exactly one form (the 8959), which is how the gap survived. An unnamed multi-page detail attachment is the phase's own definition of not-filable ("an unnamed Schedule C is not a return").
Fix: `Option<IdentityCells>` on `Form8949Map` (both pages), thread `&ReturnHeader` through `fill_8949_full`, fail closed on `None`, read-back KAT asserting both pages' name/SSN non-empty.

**I4 — The full-return Schedule D leaves the QOF question unanswered; the crypto slice answers it "No".**
`crates/btctax-forms/src/schedule_d_full.rs` contains no write to `map.qof_yes/qof_no` (the map has them, `schedule_d.map.toml:14–15`; the slice answers No at `schedule_d.rs:109`). This is precisely the "composition change to shipped code breaking a case the old code got right" I was asked to hunt: on the full path a mandatory header question ("Did you dispose of any investment(s) in a qualified opportunity fund…?") is blank, on identical ledger knowledge (bitcoin-only, no QOF).
Fix: answer No on the full path, same rationale as the slice; KAT the checkbox.

**I5 — Schedule A line 2 violates the citation-composition rule on its own printed text, and the spec's closed list is missing the citation.**
The 2024 Schedule A line 2 reads: **"Enter amount from Form 1040 or 1040-SR, line 11"** — an on-face SOURCE citation under the amended §3.1 rule's own definition. `printed.rs:1000` prints `round_dollar(p.agi)` (exact AGI, clamped at 0), which can differ from the printed 1040 L11 (= printed L9 − printed L10) by several dollars — and on a negative-AGI itemizer, Sch A L2 prints **0** while 1040 L11 prints a negative figure: the two cells visibly disagree. The divergence then propagates into L3 (the 7.5% floor) and L17 → 1040 L12. The spec's closed list (`design/SPEC_full_return.md:136–140`) omits this citation, so the implementation is spec-conformant — but the closed list contradicts the rule's own criterion, and this review's citation audit is exactly the mechanism that was supposed to catch that.
Fix: amend §3.1's closed list to add "Sch A L2 ← 1040's printed L11"; compute printed L11 before `schedule_a_lines` (no cycle — L11 does not depend on L12) and thread it in; keep the §213(a) floor's max(0,·) on line 3/4 (the form's own "if line 3 is more than line 1, enter -0-" handles the negative case correctly once L2 carries the true figure).

**I6 — The non-crypto-noncash refuse guard is keyed on the wrong sum; the mixed case ships an 8283 that under-reports its own property list.**
`crates/btctax-core/src/tax/return_refuse.rs:563–578` refuses when Σ(non-crypto noncash gifts) > $500. My ARCH-P6.3a Q6 ruling — and the §4.10 row's own rationale ("Over the $500 threshold **printed on L12 itself**…") — keys on: non-crypto noncash **present** ∧ **L12 > $500**. The gap: user-entered noncash $300 (≤ $500, passes the screen) + crypto donations $400 from the ledger → printed Sch A L12 = $700 > $500 → `assemble_printed_forms` (core `packet.rs:465–468`) attaches an 8283 listing **only the crypto rows** — an incomplete required attachment (i8283: the 8283 must list *all* noncash property when the aggregate exceeds $500), i.e. exactly the §170(f)(11) denial risk the amendment exists to close.
Fix: refuse when any non-crypto noncash gift is present AND total noncash (or L12) exceeds $500. This is compute-dependent in the L12 formulation; the conservative input-screen form "non-crypto noncash present ∧ (Σ noncash user gifts + Σ ledger crypto donations) > $500" also works. KAT the mixed case.

**I7 — The "non-overlapping filenames" guarantee is false; three filenames collide across the two pipelines.**
The slice writes `f8949.pdf` (admin.rs:256), `schedule_d.pdf` (:264), `schedule_se.pdf` (:308); the full packet writes `f8949.pdf`, `schedule_d.pdf`, `schedule_se.pdf` (packet names at `crates/btctax-forms/src/packet.rs:123–139` → `{name}.pdf` at admin.rs:494). Yet the code comment (admin.rs:216–218), the dispatch KAT's doc (export_irs_pdf.rs:426–427, which only asserts the 1040 case), and shipped **LIMITATIONS.md:69–70** ("The two write different filenames on purpose, so two runs' artifacts can never be shuffled together") all claim disjointness. This was the explicit condition I attached to deleting the P5-C1 refusal: two runs into one directory (a 2024 full return + a 2025 slice) silently overwrite/interleave — a cents Schedule SE beside a whole-dollar f1040.pdf is the chimera return the mitigation existed to prevent. A false safety claim in the user-facing doc is itself a defect.
Fix: rename the full packet's colliding members (any scheme, as long as the KAT asserts set-disjointness of the two paths' filename sets), and pin with a KAT that computes both name sets and asserts empty intersection.

**I8 — The full-return CLI output is wrong or silent about the artifact it just produced.**
(a) main.rs:628–632 unconditionally prints *"Schedule D lines 17-22 … are OUT OF SCOPE and left blank — complete them by hand"* — on the full path this is **false** (schedule_d_full.rs fills all of Part III) and instructs the filer to hand-modify a correct filed form. (b) The 8283 escalations are dropped: `export_full_return` returns `form_8283_path: None`, `form_8283_needs_review: false` (admin.rs:522–524), so the shipped loud notices — "a Section B Form 8283 is NOT filing-ready without a signed Part IV/V" and the needs-review escalation (main.rs:686–712) — never fire on the one path that now announces a "clean, filable" packet, though the packet can contain a Section-B 8283 with an unsigned appraiser declaration. A silenced loud guard is a fail-open. (c) `full_return_paths`/`full_return_manifest` are printed by **no one** — grep confirms main.rs never references them — so the command's stdout lists zero files and never mentions `manifest.txt`.
Fix: branch the ExportIrsPdf output on `full_return_paths.is_empty()`; print the packet paths + manifest; suppress the slice-only scope note on the full path; carry `form_8283_needs_review`/section into `IrsPdfReport` from the packet path and keep the Section-B notice.

**I9 — The report still prints exact-cents figures under 1040-line labels; the ARCH-P6 Q3 decision is half-implemented and LIMITATIONS overclaims it.**
`render.rs::render_dual_report` prints printed figures for L13/L15/L16/L24/L33/L34/L37 — but **exact cents** for "Total income (1040 L9)" (`ar.total_income`), "Adjustments (L10)", "AGI (L11)", "Deduction (L12)", "SE tax (Sch 2 L4)" (`ar.se_tax_sch2_l4` — the very figure whose $1 divergence from the printed Sch 2 L4 has its own discriminating KAT), "Additional Medicare" (`ar.additional_medicare.additional_medicare_tax` — the KAT-9 fixture prints 775 on the form and 774.00 here), and NIIT (`ar.niit.tax`). The Q3 ruling was explicit: any figure labeled with a form-line citation has promised the form's figure; "do not print both." The block header even says "Absolute **filed return**". Meanwhile LIMITATIONS.md:73–77 claims "The report now prints those same whole-dollar figures." Also arithmetically incoherent within one screen: printed L15 shown beneath cents L11/L12 does not re-derive.
Fix: finish the conversion — every line in the absolute block labeled with a form line renders from `PrintedForms` (`f.line9/line10/line11/line12`, `sch_2.line4`, `f8959.line18`, `f8960.line17`); the delta block stays cents. Re-scope or close `p5-m1` honestly at the same time (see M7).

### Minor

**M1** — `IpPin::canonical` reuses `SsnError` (`core/tax/packet.rs:120–131`): a 5-digit IP PIN refuses with "has 5 digits — an SSN has exactly 9", and an empty one with "no SSN was entered". Wrong noun, wrong count; give the PIN its own error or parameterize the message.
**M2** — An MFS return with no `spouse` captured files with the spouse-SSN cell and the "enter the name of your spouse" cell blank (`form1040_full.rs:294–307`); the 1040 requires them on MFS. No screen refuses the capture gap. Refuse or LIMITATIONS-note it.
**M3** — Schedule C face lines E (business address), G (material participation), H, I/J (1099 questions) are left blank with no LIMITATIONS entry. G in particular is a question the form expects answered; a mining/staking trade-or-business with SE tax is materially participated by construction. At minimum document; better, answer G = Yes on the same authority as the QOF "No".
**M4** — The full path silently ignores `--forms` (admin.rs:227–229 dispatches before the filter). Either reject the flag on a full-return year with a clear message or document that the packet is indivisible.
**M5** — `sch_d.must_file()` gating skips the 8949 branch entirely (`forms/src/packet.rs:123–132`): a real disposal whose printed cells all round to $0 files neither Schedule D nor 8949 while the DA question answers Yes; sub-dollar edge, but a disposal is reportable regardless of amount. Also `f8949 = Some` outside a `must_file` Schedule D is silently dropped. Tighten: file Schedule D whenever `f8949.is_some()`.
**M6** — Form 8960 L1/L2/L5a/L13 and Form 8995 L11/L12 are instruction-cited from 1040 lines ("see instructions" on-face) and print rounded exact values that can sit $1 from the printed 1040 2b/3b/7/11 cells the Service cross-matches. Spec-conformant today (the on-face-text criterion); consider extending the §3.1 closed list to instruction-level citations in a follow-up.
**M7** — `p5-m1` (report's interior schedule lines) is a P6-owned follow-up, neither delivered nor formally re-scoped (FOLLOWUPS.md still lists it open → P6); the Q3 fold was supposed to subsume it. Per the standing burndown rule an item whose owning phase is closing must not silently carry. Also `broker_reported_rows: 0` is hard-coded on the full path (admin.rs:512) with no recorded rationale (defensible for TY2024 — pre-1099-DA — but say so).
**M8** — `SsnError::Missing` surfaces at the packet boundary with no WHO ("the 2024 return cannot be printed: no SSN was entered", admin.rs:477–480 — which of four people?); and the ARCH note "line 2a blank when zero" is not honored (`push_money` writes "0" unconditionally). Both cosmetic-but-fixable.

### Nit

**N1** — `testonly::extract_lines` (the P7 inverse transcriber I asked to be built while the maps are fresh) was not built; the cross-PDF oracle was hand-rolled with `tv()`. P7-owned.
**N2** — The manifest does not carry the "attach your W-2 Copy B" line (ARCH-P6.3a verified-deliberate list asked for it).
**N3** — `every_schedule_carries_the_name_and_ssn_header` (full_return_forms.rs:1539) tests one form; the name promises all. Had it iterated the packet, I3 would have been caught mechanically. Also the 1040 L23 ↔ Sch 2 L21 *cell-text* leg of the cross-PDF oracle (my Q2 item 3) was implemented only for Sch D ↔ 8949.
**N4** — 1040 line 7 prints "0" where the form says "enter -0-" on the L16=0 routing; harmless, inconsistent with the form's idiom.
**N5** — The 1040's foreign-address row (f1_15–f1_17) is unmapped/unsupported and undocumented.

**Observation (no number):** all-or-nothing holds against filler refusals (verified: fill precedes any write, and the no-SSN CLI KAT asserts an empty out_dir), but an I/O error mid-write-loop (admin.rs:488–499) can still leave a partial packet on disk; LIMITATIONS' "you never find a half-packet on disk" is scoped to fill failures. Acceptable; noting for the record.

---

## Rulings on the three declared deviations

**1. SSN compute-vs-packet split — ACCEPTED.** The split is sound and opens no hole. The load-bearing chain: `Ssn`'s field is private and `canonical()` is its only constructor, so an identity cannot be fabricated; `fill_full_return` takes `&PrintedReturn`, which cannot exist without `ReturnHeader::build` succeeding; a malformed-but-captured SSN refuses at compute (`SsnMalformed`, KAT'd), an empty one refuses at the packet (`SsnError::Missing`, KAT'd end-to-end with the empty-out_dir assertion). The `PrintedForms`/`ReturnHeader` split (figures vs identity) is a better design than my original blanket compute-time refusal — the rationale ("the tax math never reads an SSN; refusing the computation blocks the report a filer uses to decide whether to file") is correct and the fail-closed property attaches to the artifact, where it matters. Residue: M1, M8.

**2. Schedule SE line 13 = `round_dollar(se.deductible_half)` — UPHELD, with a boundary drawn.** Re-examined against the form's "Multiply line 12 by 50%": the maximum divergence a human recomputation can see is $1 (23 × 50% = 11.50 vs printed 11), inside the Service's rounding tolerance for a multiply line, and the alternative (`round(line12/2)`) merely relocates the disagreement into the Sch 1 L15 tie-out plumbing while drifting L13 from the §164(f) figure the engine actually deducted. The 8959 L7 precedent covers it. **But this acceptance is expressly bounded to $1-class multiply residuals** — it does not extend to the Tax-Table step function, which is finding I2. Document the boundary in the §3.1 amendment when I2 is fixed.

**3. `ScheduleDLines::must_file()` — CORRECT and complete.** The predicate (`printed.rs:672–688`) enumerates lines 3d/e/h, 6, 10d/e/h, 13, 14, 16 — a carryover-only return (line 6/14 ≠ 0) and a distribution-only return (line 13 ≠ 0) both still file, verified by inspection of the arms; line 16 is redundant-but-harmless (identically 0 when all sources are 0). The one edge is M5 (all cells rounding to zero on a real disposal).

---

## Form-citation audit (the closed list, per the printed text of the bundled TY2024 PDFs)

Verdicts: ✅ satisfied (composed and/or attached, KAT'd) · 📄 documented (blank/refused per spec, LIMITATIONS/advisory) · ❌ finding.

| Form · line | Printed citation text (abridged) | Verdict |
|---|---|---|
| 1040 1z | "Add lines 1a through 1h" | ❌ **I1** — 1a blank, 1z filled |
| 1040 7 | "Attach Schedule D if required. If not required, check here" | ✅ Sch D files whenever L7 ≠ 0 (`must_file`); box never checked (N4 zero-idiom) |
| 1040 8 / 10 | "from Schedule 1, line 10 / line 26" | ✅ composed + packet KAT |
| 1040 9 | "Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8" | ✅ printed-operand sum; 4b/5b/6b unrepresentable → blank, 📄 LIMITATIONS |
| 1040 12 | "itemized deductions (from Schedule A)" | ✅ composed + KAT |
| 1040 13 | "from Form 8995 or 8995-A" | ✅ composed (8995); 8995-A refused 📄 |
| 1040 16 | "Tax (see instructions). Check if any from Form(s) 8814/4972" | ❌ **I2** (Table on printed L15); boxes correctly unchecked (8814 kiddie-refused) |
| 1040 17 | "Amount from Schedule 2, line 3" | ✅ 0; Sch 2 Part I blank by construction 📄 |
| 1040 19 | "Child tax credit … from Schedule 8812" | 📄 0 + `CtcOdcOmitted` advisory (conservative omission) |
| 1040 20 / 23 / 31 | "from Schedule 3, line 8" / "from Schedule 2, line 21" / "from Schedule 3, line 15" | ✅ composed + KATs (L23 cell-text leg N3) |
| 1040 25a/25b | "Form(s) W-2 / Form(s) 1099" | ✅ Σ box 2 / Σ box 4 (INT+DIV+G) |
| 1040 25c | "Other forms (see instructions)" | ✅ 8959 printed L24 + declared other withholding; 8959 attached iff `must_file` |
| 1040 27–30 / 35b–d / 36 / 38 | EIC etc. / direct deposit / applied / penalty | 📄 blank, advisories + LIMITATIONS |
| 1040 header | filing status, DA question, dependent boxes, aged/blind, MFS-itemizes, presidential, IP PIN, dependents table | ✅ all print (aged/blind + dependent-claim + MFS-itemize KAT'd); >4 dependents refuses; MFS-no-spouse M2 |
| Sch 1 3 | "Attach Schedule C" | ✅ Sch C files whenever L3 > 0 (`BusinessIncomeWithoutScheduleC` refuses the orphan) |
| Sch 1 9 / 10 / 26 | "Add lines 8a–8z" / "Enter … on Form 1040, line 8" / "…line 10" | ✅ printed sums, composed |
| Sch 1 15 | "Deductible part of self-employment tax. **Attach Schedule SE**" | ✅ composed (SE printed L13) + attached + tie-out KAT |
| Sch 2 3 | "Add lines 1z and 2 … on Form 1040, line 17" | ✅ blank Part I ↔ 1040 L17 = 0 📄 |
| Sch 2 4 | "Self-employment tax. **Attach Schedule SE** " | ✅ = SE printed L12, attached, discriminating KAT |
| Sch 2 11 / 12 | "Attach Form 8959" / "Attach Form 8960" | ✅ = printed L18 / L17, attached iff nonzero, KAT-9 |
| Sch 2 21 | "Add lines 4, 7–16, 18, 19 … on Form 1040 line 23" | ✅ printed-operand |
| Sch 3 1 | "Foreign tax credit. Attach Form 1116 **if required**" | ✅ §904(j) ⇒ not required; over-ceiling refuses 📄 |
| Sch 3 8 / 10 / 11 / 15 | "…on 1040 line 20" / extension payment / excess SS / "Add lines 9–12 and 14 … line 31" | ✅ incl. the D1 extension fix, KAT'd both alone and summed |
| Sch A 2 | "**Enter amount from Form 1040, line 11**" | ❌ **I5** — not composed on the printed L11 |
| Sch A 3 / 4 / 5d / 5e / 7 / 8e / 10 / 14 / 17 | multiply/subtract/add/smaller-of chains; "…on Form 1040, line 12" | ✅ printed-operand throughout, cross-foot KAT |
| Sch A 5a box / 18 box | sales-tax election / §63(e) election | ✅ print (Q7 items 3–4), KAT'd |
| Sch A 9 / 15 / 16 / 8b / 8c / 6 | Form 4952 / 4684 / other | 📄 unrepresentable, blank |
| Sch A 12 | "You must attach **Form 8283** if over $500" | ❌ **I6** mixed-case gap; crypto-only path ✅ (attach + no re-derive, per D7) |
| Sch B 2 / 4 / 6 | "Add the amounts on line 1" / "…on Form 1040, line 2b" / "…line 3b" | ✅ printed rows, composed, KAT |
| Sch B 3 | Form 8815 | 📄 blank (unmodeled) |
| Sch B Part III 7a / FinCEN / 7b / 8 | filer's own answers | ✅ transcribed (None refuses); FinCEN sub-question blank + `FbarFinCen` advisory 📄; 7b prints; 8 = Yes refuses (Form 3520) |
| Sch B capacity | 14 / 15 rows | 📄 refuses (`Overflow`), SPEC §7.4 as amended, packet-level KAT |
| Sch C A/B/F | description / NAICS / method boxes | ✅ print (incl. affirmative Cash box) |
| Sch C E/G/H/I/J | address / material participation / 1099 questions | ❌→M3 blank, undocumented |
| Sch C 31 | "enter on **both** Schedule 1, line 3, and Schedule SE, line 2" | ✅ both composed + KAT'd |
| Sch C 32a/b | loss boxes | ✅ unreachable (loss refuses) |
| Sch D header QOF | "Did you dispose of any investment(s) in a QOF…?" | ❌ **I4** — unanswered (slice answers No) |
| Sch D 1a/1b/2, 8a/8b/9 | 1099-B box rows | ✅ blank — all rows are Box C/F on the 8949 |
| Sch D 3 / 10 | "Totals for all transactions … 8949 with **Box C / Box F** checked" | ✅ composed on the 8949's printed totals; Box C/F checked by map; cross-PDF cell-text oracle |
| Sch D 6 / 14 | "from … your Capital Loss Carryover Worksheet" | ✅ user-entered magnitudes (worksheet unmodeled 📄; TI≤0-with-carryforward refuses); paren-guarded |
| Sch D 7 / 15 / 16 | "Combine … in column (h)" | ✅ printed-operand |
| Sch D 17 / 18 / 19 / 20 / 21 / 22 | Part III decision tree; "($3,000)/($1,500)" smaller-of; paren box | ✅ all four routings filled + KAT'd; 18/19 = 0 (2b/2c/2d refuse); 21 printed-operand magnitude, fail-closed on negative |
| 8949 header | "Name(s) shown on return" + SSN (both pages) | ❌ **I3** — never written |
| 8949 Box A–C/D–F | "You must check Box A, B, or C" | ✅ Box C / Box F via map on-state |
| 8949 col (h) / line 2 | "Subtract column (e) from column (d)" / "Add the amounts…" | ✅ h derived (D2), totals sum printed rows, page-partition identity KAT |
| Sch SE header | "Name of person **with self-employment income**" + SSN | ✅ proprietor (spouse-owner KAT) |
| Sch SE 2 | "Net profit … from **Schedule C, line 31**" | ✅ composed + KAT |
| Sch SE 3/4a/4c/6/8d/9/10/11 | combine/multiply chain; line 7 pre-printed | ✅ (multiply lines = engine-at-the-line, $1-bounded, D5/D2 ruling); 8b–10 skip rule matches the form |
| Sch SE 12 | "Add lines 10 and 11. Enter … Schedule 2, line 4" | ✅ printed-operand + composed |
| Sch SE 13 | "Multiply line 12 by 50%. Enter … Schedule 1, line 15" | ✅ per deviation ruling #2 (documented residual) |
| 8959 1 / 19 | "from Form W-2, box 5 / box 6" | ✅ |
| 8959 7 / 13 / 18 | "Multiply … by 0.9%" / "Add lines 7, 13, and 17 … Schedule 2, line 11" | ✅ printed-operand, KAT-9 |
| 8959 8 | "Self-employment income from **Schedule SE, Part I, line 6**" | ✅ equal by construction + tie-out KAT |
| 8959 24 | "**include** … on Form 1040, line 25c" | ✅ |
| 8960 1 / 2 / 3–6 / 13 | "(see instructions)" | ✅ conformant (M6 noted); unmodeled lines blank 📄 |
| 8960 8 / 12 / 15 / 16 / 17 | combine/subtract/smaller-of; "include on … your tax return" | ✅ printed-operand; → Sch 2 L12 composed; QSS $250k asymmetry KAT'd |
| 8995 2–10 / 13 / 14 / 15 | combine/multiply/smaller-of; "enter … on Form 1040, line 13" | ✅ printed-operand + composed; L11/12 M6 |
| 8995 16 / 17 | paren carryforward boxes | ✅ magnitudes, fail-closed filler guard |
| 8283 header | "Name(s) shown on your income tax return" + identifying number | ✅ full path writes them (KAT) |
| 8283 attach-trigger | "Attach … if you claimed a total deduction of over $500 for **all** contributed property" | ❌ **I6** (mixed case); carryover-year no-reattach ✅ (presence keys on current-year L12) |
| 8283 Part III–V declarations | taxpayer/appraiser/donee signatures | 📄 blank by necessity — but the loud not-filing-ready escalation is dropped on the full path → **I8(b)** |
| Attachment Sequence Nos. | 01,02,03,07,08,09,12,12A,17,55,71,72,155 | ✅ all verified against the printed forms; emission order ascending |

---

## The specific attack surfaces I was directed at

- **Composition changes to shipped code** (`schedule_2_lines` L4 ← SE L12; `schedule_d_lines` ← 8949 totals, printed-operand netting on 7/15/21): correct and well-pinned. The §7.2 routing is exhaustive across all four paths on printed operands; the §1211 cap `min(|printed L16|, limit)` with MFS $1,500 is right; the one regression found is the **QOF answer (I4)**.
- **Dispatch:** logically airtight — one `if` at the single entry point (`return_inputs::exists`), before any byte is written, pseudo-gate preserved on both branches, KATs in both directions, no other caller of the slice fillers. The **filename disjointness half of the mitigation is not delivered (I7)**.
- **All-or-nothing:** holds against filler refusals (fill precedes write; empty-out_dir KAT); I/O-mid-loop caveat noted.
- **PII:** clean. `Ssn`/`IpPin` Debug masked; refusals carry *who*, never digits; `Ssn` unfabricatable outside `canonical()`; no serde on either; packet/CLI error strings checked.
- **Test quality:** the discriminating KATs are genuinely discriminating (I verified the fixtures force `Σround ≠ roundΣ` and printed ≠ exact on every claimed axis); the cross-PDF byte oracle is real and reads serialized bytes; fault-injection swap tests exist per form. Gaps: N3 (one-form "every schedule" test; missing 1040↔Sch2 cell-text leg), and the kitchen-sink packet exercises no 8283 member.

## What must happen to close the gate

Fix and re-review I1–I9 (each has a small, local fix and a natural discriminating KAT; I5/I6 also take one-line spec amendments in the errata lane — fold them before the re-review, as before). The Minors should be burned down or filed with owning phases per the standing rule (M7 is itself a burndown-hygiene item). I am available for the re-review after the fold.

**VERDICT: 0 Critical / 9 Important / 8 Minor / 5 Nit**
