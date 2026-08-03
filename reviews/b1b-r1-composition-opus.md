# §G-28/B1b review r1 — the COMPOSITION lens (Opus, 2026-08-02)

Scope: `9880385..21fa20e` (the five §199A commits). One question: *is there any remaining place where one
number has TWO authorities, or where a value flows into a filed return WITHOUT the guard that makes it
safe?* Independent context; read-only; told what was already settled (the one-oracle limit, the skippable
design, the two already-fixed Criticals, the four unfilled 8995-A sub-schedules).

**Persisted VERBATIM before folding**, per `STANDARD_WORKFLOW.md` §2. Everything below the rule is the
reviewer's own text.

---

No Critical findings.

---

SEVERITY: Important
FILE: /scratch/code/bitcoin_tax/crates/btctax-input-form/src/apply.rs:77-86 (with /scratch/code/bitcoin_tax/crates/btctax-input-form/src/spec/sections.rs:1072-1120)
CLAIM: `Edit::ClearField` on the two new §199A(b)(2) fields writes `Some($0)` instead of `None`, which is precisely the state `screen_absolute`'s `QbiAboveThreshold` refusal — the only guard in front of `qbi_w2_wages.unwrap_or(ZERO)` — tests for.
TRACE: `qbi_w2_wages`/`qbi_ubia` are the **only two `Option<Usd>` leaves in `ReturnInputs`** (return_inputs.rs:313, 318); every other `Option<Usd>`-shaped answered-ness in the codebase carries its answered-ness on a separate `Option<bool>` gate (`has_income_exclusion`, return_inputs.rs:697-712, which documents exactly why: *"`Option<Usd>` is a scalar the `_` rule permits"*). Both new `Field`s are declared `clear: None` (sections.rs:1075, 1099). `apply.rs`'s un-answer path routes a field with no dedicated `clear` through `set(empty_for_kind)`, and `empty_for_kind` for `FieldKind::Money` is `FieldValue::Money(Usd::ZERO)` (apply.rs:78). Their setters then execute `…qbi_w2_wages = Some(m)` with `m = 0`. `screen_absolute` (return_1040.rs:1938-1942) refuses only on `c.qbi_w2_wages.is_none() || c.qbi_ubia.is_none()`, so the laundered `Some(0)` passes, and `assemble_absolute`'s `unwrap_or(Usd::ZERO)` (return_1040.rs:1497-1506) — whose own comment says it "is safe ONLY because `screen_absolute` refuses an unanswered pair" — is reached with the guard not having fired.
FAILURE: A sole proprietor above the §199A(e)(2) threshold who pays $120k of W-2 wages, enters it, then un-answers it through the seam: Form 8995-A line 4 = 0 → line 10 = 0 → line 11 = 0 → line 16 = 0 → 1040 line 13 = 0 instead of the capped ≈$50k. Tax **overstated** — the exact direction return_1040.rs:1495 warns about. Reachability caveat: the shipped TUI binds `ClearField` only for `TriState` (tax_inputs.rs:627) and `parse_money("")` errors rather than clearing, so today this is reachable through the public `btctax-input-form::apply` seam, not through the TUI/CLI. `clearfield_kind_matrix_and_registry_unanswer` (apply.rs:617) covers Enum/TriState/Date/Secret/plain-Date and **has no Money-over-`Option<Usd>` case**, so nothing reds. What would settle the reachability half: whether any consumer (or a planned TUI clear key) emits `ClearField` for a `FieldKind::Money` field. The laundering itself is settled by reading apply.rs:74-86.

---

SEVERITY: Important
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs:1507 and :1708
CLAIM: "Taxable income before the qualified business income deduction" now has two authorities that print on **the same** Form 8995-A — line 20 (Part III) and line 33 (Part IV) — and this branch put the form choice and the refusal regime on the wrong one of the two.
TRACE: `printed_inputs.ti_before_qbi = agi − deduction − schedule_1a_additional` (return_1040.rs:1708), and its own doc comment states the 13b term is load-bearing: *"Omitting 13b would OVERSTATE this, inflating the §199A deduction and firing `qbi_over_threshold` too EARLY."* The new code passes the **un-adjusted** `agi − deduction` to `Qbi199aRegime::of` (:1462, :1509), to `uses_8995a` (:1473), and as `PartIToIiiInputs::ti_before_qbi` (:1507) → Part III line 20 (qbi_a.rs:327). `screen_absolute` does the same (`ar.agi − ar.deduction`, :1889). Meanwhile `assemble_printed_forms` passes `pi.ti_before_qbi` to `form_8995_lines` → Form 8995 line 11 → Part IV line 33 (packet.rs:600). The `compute_8995`/`form_8995_lines` split on this argument is pre-existing; what B1b added is a **second printed line of the same quantity on the same page**, plus the form-selection and refusal-regime decisions, all on the other side of the split.
FAILURE: Dormant today — `schedule_1a_additional` is a hardcoded `Usd::ZERO` (:1539). The moment Schedule 1-A B3 lands and line 13b is non-zero, a single filer with, say, $40k of Schedule 1-A deductions and TI-before-QBI of $230k (adjusted) / $270k (unadjusted): the regime classifies as `AboveThePhaseInRange` instead of `InPhaseInRange`, Part III is skipped entirely, line 12 stays blank, line 13 = line 11 (the hard cap) instead of the phased-in figure — deduction understated, tax **overstated** — and the emitted form prints line 20 = $270,000 against its own line 33 = $230,000, which is the same side-by-side inconsistency this branch already fixed twice. It also flips `uses_8995a`, so a filer at the boundary gets the wrong §199A form.

---

SEVERITY: Important
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/line_coverage.rs:2240-2243
CLAIM: The 25 new printed money lines of Form 8995-A Parts II and III are outside §G-11 line coverage, so neither the compile-time destructure nor the xtask instruction-text check ever sees them — and no test reds.
TRACE: `all()` registers `cover_form8995lines` and `cover_form8995apartiv` (:2242-2243) from a hand-written list. `Form8995APartIi` (lines 2-16) and `Form8995APartIii` (lines 17-26) are new structs with no `cover_*` function and no entry in `all()`; the module's guarantee is "a newly added money field is a *pattern does not mention field* COMPILE ERROR", which only holds for structs already destructured somewhere in the file — a new *struct* produces no error at all. `PrintedForms::f8995a` also changed type from `Form8995APartIv` to `Form8995A` (packet.rs:511) and `cover_form8995apartiv(&Form8995APartIv::default())` still compiles unchanged, so that type change reded nothing either. The xtask text half checks only `instruction` strings present in this table, so the doc comments on Part II lines 2-16 and Part III lines 17-26 were never verified against `design/forms/extract/f8995a--2024.txt`.
FAILURE: Not a known-wrong figure — a missing guarantee. This is the instrument that would catch a Form 6251-line-33-class transcription slip (`"Subtract line 32 from line 12"` vs `line 22`); with Parts II/III outside it, e.g. a line 6 doc comment reading 25% while `pct(line4, 25, 100)` is correct, or a line 9 that added lines 5 and 8 instead of 6 and 8, is caught by nothing. The module's own doc already names this exact class as "THE HONEST LIMIT" for nested types; it does not appear to have been re-filed for the two new structs.

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs:1486-1490 vs :1717
CLAIM: The business name has two authorities in one packet — Form 8995-A Part I column (a) gets the raw string, Schedule C line A and Form 8995 row 1i(a) get a trimmed one.
TRACE: `PartIToIiiInputs::business_name` is `c.business_description.clone()` (:1489), untrimmed. `printed_inputs.schedule_c_header.business_description` is `c.business_description.trim().to_string()` (:1717), with the comment *"Trimmed ONCE, here, so Schedule C line A and Form 8995 row 1i(a) carry the same canonical string (Fable P7 r3, Minor)."* That trimmed value flows to Schedule C line A (printed.rs:1142) and to `form_8995_lines` (packet.rs:592); the untrimmed one flows to `Form8995ARowA::col_a_name` (qbi_a.rs:370) and straight into the AcroForm cell (form8995a.rs:129-130). Core only rejects an all-whitespace name (`return_refuse.rs:831` uses `.trim().is_empty()`), so `"  Bitcoin mining  "` reaches both paths.
FAILURE: A filer whose stored `business_description` has surrounding whitespace files a packet whose Schedule C line A reads `Bitcoin mining` and whose Form 8995-A line 1(a) reads `  Bitcoin mining  `. No dollar figure moves; it is the same canonicalization regression a prior review already closed for Form 8995.

---

SEVERITY: Minor
FILE: /scratch/code/bitcoin_tax/crates/btctax-forms/src/form8995a.rs:126-136
CLAIM: The Form 8995-A emitter lacks the fail-closed "non-zero QBI with an unnamed business" guard that the Form 8995 emitter carries, on the identical decision.
TRACE: `form8995.rs:166-175` refuses to produce a form when `lines.line2 > 0` and `business_name.trim().is_empty()`, with reasoning explicitly framed as a backstop for a state core already refuses (*"this fails closed if one ever reaches here anyway"*). `form8995a.rs` writes `a.col_a_name` unconditionally whenever `parts_i_to_iii` is `Some` — i.e. whenever `business_qbi > 0` (qbi_a.rs:296) — with no equivalent check, so an empty name would produce a filed Form 8995-A Part II line 2 of $N over a nameless row A.
FAILURE: Unreachable today (`ScheduleCNoBusinessDescription` refuses first), so no wrong number ships. Reported because it is the same field-of-view failure the branch doctrine names: the fix exists nine files away, for the same decision, and was not carried across.
