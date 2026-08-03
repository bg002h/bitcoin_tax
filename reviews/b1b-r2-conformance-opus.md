# §G-28/B1b review r2 — the FORM-CONFORMANCE lens (Opus, 2026-08-02)

Scope: `9880385..21fa20e`. One question: *does the new Form 8995-A implementation faithfully TRANSCRIBE
the form and its instructions?* Independent context, run in parallel with r1 (composition) and told the
same settled facts. Read-only.

**Persisted VERBATIM before folding**, per `STANDARD_WORKFLOW.md` §2. Everything below the rule is the
reviewer's own text.

★ Its opening paragraph is the finding that matters most and is easy to skim past: the reviewer
**regenerated `pdftotext -layout` from the shipped PDF and compared it byte-for-byte to the committed
extract**, then machine-checked every quote against it. That is the check the transcription rule exists
to demand, and it passed.

---

No Criticals found. The transcription itself is clean: I regenerated `pdftotext -layout` from `crates/btctax-forms/forms/2024/f8995a.pdf` and it is **byte-identical** to `design/forms/extract/f8995a--2024.txt`, then machine-checked every `*"…"*` quote in `qbi_a.rs`, `form8995a.rs`, `return_inputs.rs`, `return_refuse.rs` and the map against that text plus `i8995a`/`f8995` — all faithful. Every arithmetic sentence (19=17−18, 22=20−21, 25=19×24, 26=17−25, 15=13−14, 10=greater of 5/9, 11=smaller of 3/10, 13=greater of 11/12, 30/35/37/39/40) does what its own line number says. The three regime boundaries are correct (strict at the bottom, inclusive at the top, matching Exception 1/2 and §199A(b)(3)(B)(i)/(d)(3)(A)). The printed form cross-foots. All 111 AcroForm fields are mapped or censused with true reasons (I verified the Part I column x-order, the Part II/III B/C column x-positions, and that lines 20–24 map to the `Ln` entry boxes and not the `_RO` mirrors).

---

SEVERITY: Important
FILE: /scratch/code/bitcoin_tax/crates/btctax-input-form/src/attribute.rs:176 (and /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs:1998)
CLAIM: This commit repurposed `RefuseReason::QbiAboveThreshold` from "unmodeled, nothing can fix it" to "Form 8995-A lines 4 and 7 are unanswered" and added the form fields that fix it — but both remediation pointers still describe the old meaning, so the filer B1b exists for is refused and sent to two places that cannot collect the answer.
EVIDENCE: The refusal text now reads `"…(Form 8995-A lines 4 and 7). btctax will not guess either one… If your business has no employees and no property, answer zero and the return files — run \`btctax income answer\`"`. But `answer.rs::live_questions` collects only `FORM_QUESTIONS` (declarations) and `SKIPPABLE_QUESTIONS`; `qbi_w2_wages`/`qbi_ubia` are neither — `classifier.rs` classifies them `Class::NoTaxDirection` exempt, and they live as `FieldKind::Money` fields in `sections.rs::QBI_LIMITATION_FIELDS`. Meanwhile the attribution map still says `R::QbiAboveThreshold => vec![Anchor::NotInForm { note: "the §199A QBI-over-threshold screen is computed at \`report\`, not a v1 form field" }]`, which is now false — `FieldId::QbiW2Wages` and `FieldId::QbiUbia` in `SectionId::QbiLimitation` are exactly the v1 form fields that resolve it. The falsehood is *pinned* by a green test: `attribute.rs:387` lists `RefuseReason::QbiAboveThreshold` in `deferred_and_defensive_refusals_are_not_in_form`.
FAILURE: Single filer, Schedule C mining net $240,000, TI-before-QBI $290,000, `qbi_w2_wages`/`qbi_ubia` never set. The return refuses. `btctax income answer` runs to completion without ever asking for wages or UBIA, so the return still refuses; the input-form/TUI attribution says the refusal has no anchor, so nothing highlights the "§199A limitation" section. Recoverable only by finding that section unaided in the TUI edit form, or by TOML import. No wrong number reaches a filed return — this is a dead-end remediation path plus an attribution map that asserts a falsehood under test.

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-forms/tests/f8995a_map.rs:243
CLAIM: `every_quoted_instruction_is_verbatim_on_the_form` does not check what its name and doc claim — Part I's five column quotes are silently unparsed, and the `checked == 39` guard cannot see their absence.
EVIDENCE: The parser accepts a comment only if its first token is all digits and ≤2 chars: `if !(num.len() <= 2 && num.chars().all(|c| c.is_ascii_digit())) || !tail.starts_with('"') { continue; }`. The Part I comments are `# 1(a) "Trade, business, or aggregation name"`, `# 1(b) "Check if specified service"`, `# 1(c) …`, `# 1(d) …`, `# 1(e) …` — `num` is `1(a)`, so all five `continue`. The count guard is `assert_eq!(checked, 39, …)`, and 39 is exactly Parts II+III+IV (15+10+14), so it is satisfied with zero Part I quotes checked. (Separately: `"Check if specified service"` is *not* contiguous in the text layer — the extract wraps the column header as `(b) Check if … specified service` — so extending the parser to Part I would red on a faithful quote. That is a text-layer artifact, not a paraphrase, but it means the gap cannot be closed by parser widening alone.)
FAILURE: A future edit paraphrasing `1(b)` as e.g. "Check if service business" would ship with a green suite. Today's five quotes are in fact faithful — I checked them by hand against the extract — so no defect is live.

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-forms/forms/2024/f8995a.map.toml:34-38
CLAIM: The map's own scope banner is false, and false about the same file it heads.
EVIDENCE: `"★ SCOPE — PART IV ONLY, deliberately. … Parts I–III (the W-2-wage and UBIA limitations, the SSTB phase-in) are B1b and need three new filer inputs plus the phase-in range width, which does not exist in \`FullReturnParams\` at all. Mapping them here with nothing to write would be an instrument nobody has watched discriminate."` — but this same file maps `[part1_row_a]`, `[part2_col_a]` and `[part3_col_a]` at lines 90–187, and `qbi_phase_in_range_unmarried` exists at `tables.rs:474`. The census header 45 lines below (`"★★★ B1b FILLS PARTS I-IV"`) contradicts it directly.
FAILURE: None on a filed return. A reader auditing the map's coverage is told Parts I–III are deliberately unmapped and may not check them.

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs:1486-1490
CLAIM: Form 8995-A Part I column (a) uses the untrimmed business description while Schedule C line A and Form 8995 row 1i(a) use the trimmed canonical one — re-introducing the exact divergence the trim comment exists to prevent.
EVIDENCE: `business_name: ri.schedule_c.as_ref().map(|c| c.business_description.clone()).unwrap_or_default()` (no `.trim()`), versus `return_1040.rs:1714-1717`: `// Trimmed ONCE, here, so Schedule C line A and Form 8995 row 1i(a) carry the // same canonical string (Fable P7 r3, Minor).` / `business_description: c.business_description.trim().to_string(),`.
FAILURE: A filer whose imported TOML has `business_description = "  Bitcoin mining "` files a Schedule C line A reading `Bitcoin mining` and a Form 8995-A row A reading `  Bitcoin mining ` — the same business named two ways in one packet. An all-whitespace name refuses upstream (`ScheduleCNoBusinessDescription`), so the blank case cannot occur.

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi.rs:249-250 (and :199, :319)
CLAIM: `Form8995Lines`'s line-5 doc and the struct's cross-footing promise are now false on the 8995-A path, where line 5 silently holds a different form's line.
EVIDENCE: The field doc still reads `/// L5 — QBI component = 20% × line 4.` while the code is `let line5 = qbi_component_8995a.unwrap_or_else(|| round_dollar(QBI_RATE * line4));` — on the 8995-A path that is Form 8995-A line 16, i.e. 20% of QBI *after* the §199A(b)(2) cap and the Part III phase-in. The struct doc at :199 promises `"The printable **Form 8995 line chain** — whole dollars, cross-footing (SPEC §3.1)"`, and on that path `line5 ≠ round(0.20 × line4)`.
FAILURE: No wrong number today, because `packet.rs:631` (`let f8995 = if f8995a.is_some() { None } else { f8995 };`) never prints the struct as Form 8995 when the override is in play. If that null-out is ever reordered or removed, a Form 8995 files whose line 5 does not re-derive from its own line 4 — a form that does not cross-foot. Held by one assertion (`pr.forms.f8995.is_none()` in `the_1040_deduction_equals_the_attached_8995a_line_39`).

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi_a.rs:206-207, :365
CLAIM: The line-15 transcription carries the FORM's sentence but drops the INSTRUCTIONS' clamp on the same line.
EVIDENCE: i8995a (`design/forms/extract/i8995a--2024.txt`, line 851): `"Line 15. Subtract the patron reduction on line 14 from the amount on line 13. If zero or less, enter zero."` The doc comment is `/// L15 — *"Qualified business income component. Subtract line 14 from line 13"*.` and the code is `let line15 = line13 - line14.unwrap_or(Usd::ZERO); // "Subtract line 14 from line 13"` — no clamp, and the "if zero or less, enter zero" sentence appears nowhere in the module.
FAILURE: Unreachable today, and I verified why rather than assuming: `compute` returns `None` when `business_qbi <= 0`, so `line3 ≥ 0`; `line10 = max(line5, line9) ≥ 0` (negative wages/UBIA refuse in `return_refuse.rs::first_negative_amount`); `line11 = min(line3, line10) ≥ 0`; `line12 = line26 = line17 − line25` with `line24_ratio ≤ 1` so `line26 ≥ line18 ≥ 0`. Hence `line13 ≥ 0` and `line15 ≥ 0` always. The omission is the "dropped term becomes invisible" class rather than a live defect — it becomes live the moment Schedule C (Form 8995-A) loss netting or a patron reduction is ever modelled.

---

SEVERITY: Nit
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs:1899
CLAIM: The comment overstates the refusal coverage.
EVIDENCE: `"…what is left is precisely the four schedules attached to 8995-A — A (SSTB in the range), B (aggregation), C (loss carryforward) and D (patron) — each of which gets its own named refusal below."` The block below raises `CooperativePatronUnanswered`, `CooperativePatron`, `SstbUnanswered`, `SstbInPhaseInRange`, `QbiAboveThreshold` and `QbiCarryforwardNeedsSchedule8995AC` — there is no Schedule B refusal.
FAILURE: None. The behaviour is right: aggregation is elective over ≥2 businesses, btctax has one, and `col_c_aggregation` is hard-`false` with that reasoning recorded at `qbi_a.rs:141-145`. Only the count in the comment is wrong.

---

SEVERITY: Nit
FILE: /scratch/code/bitcoin_tax/crates/btctax-forms/src/form8995a.rs:271-278
CLAIM: Line 24 prints with Decimal-scale trailing zeros.
EVIDENCE: `iii.line24_ratio * rust_decimal::Decimal::from(100)` then `fmt_money(d) = d.to_string()` (`lib.rs:79-81`), with no `.normalize()`. I ran this against the workspace's own rust_decimal: `14031/50000 = 0.28062` (scale 5) `× 100 → "28.06200"`; `44757/100000 → "44.75700"`; `25000/50000 → "50.00"`; `50000/50000 → "100"`.
FAILURE: The filed form's line 24 reads `28.06200 %` instead of `28.062 %` or `28.06 %`. Arithmetically correct and it cross-foots with line 25; `verify_flat`'s `/MaxLen` read-back would fail closed if it ever overflowed the widget.

---

SEVERITY: Nit
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi_a.rs:79-85 vs :327
CLAIM: The regime is classified on exact cents while lines 20 and 33 print the rounded figure, so at a sub-dollar boundary the printed form contradicts its own Who-Must-File and Part III gate.
EVIDENCE: `Qbi199aRegime::of` compares `ti_before_qbi <= params.qbi_ti_threshold(status)` on the unrounded `agi - deduction` (`return_1040.rs:1509`), while `let line20 = round_dollar(i.ti_before_qbi);`. The form's header says `"Use this form if your taxable income, before your qualified business income deduction, is above $191,950"` and Part III says `"more than $191,950"`.
FAILURE: TI-before-QBI of $191,950.40 files Form 8995-A whose line 33 prints `191950` and whose line 20 prints `191950` against a line 21 of `191950` — both gates read as unsatisfied on the page. No figure changes (line 22 = 0 ⇒ line 24 = 0% ⇒ line 26 = line 17, so line 16 equals Form 8995's line 5). PLAUSIBLE rather than a defect: the statute's "taxable income" is the actual $191,950.40, so the *choice* is right and only the printed presentation is ambiguous. Settled by deciding whether §3.1 rounding is meant to be applied before or after the threshold test.

---

SEVERITY: Nit
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi_a.rs:429
CLAIM: A doc-comment quote uses an ASCII apostrophe where the form's text layer has a curly one, and nothing checks Rust doc quotes at all.
EVIDENCE: The doc reads `*"DPAD under section 199A(g) allocated from an agricultural or horticultural cooperative. Don't enter more than line 33 minus line 37"*`; the extract line 98 reads `Don’t enter more than line 33 minus line 37`. The map's copy (`f8995a.map.toml:70`) correctly uses `Don’t`, and only the map is under `every_quoted_instruction_is_verbatim_on_the_form`.
FAILURE: None. Noted only because it is the one place a *machine* could have caught a drift and there is no such machine over `.rs` doc comments for this form.
