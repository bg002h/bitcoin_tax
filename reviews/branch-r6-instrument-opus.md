# r6 — INSTRUMENT LENS (Opus)

**Date:** 2026-08-02 · **Range:** `fc96703..HEAD` · **Brief:** [`BRIEF-r6.md`](./BRIEF-r6.md)

**Result: 0 Critical / 6 Important / 2 Minor / 2 Nit.** Every claim verified by mutation, every
mutation restored.

★★★ **I-1 IS THE KEYSTONE, AND IT IS THE EXACT FAILURE HARNESS RULE B1 EXISTS TO PREVENT.** The test
I named *"B1 — the planted defects"* **never calls `run()`**. It asserts on the FIXTURES — it constructs
a `LineCoverage` row and then asserts the row has the fields it just set. **The entire checker can be
deleted and the suite stays green**, verified: replacing `run()`'s body with `return Ok(..)` gives
*"55 passed; 0 failed"*. B1's own reviewable question is *"which test reds when this checker is
removed?"* and the answer is **none**. I ran real kills by hand against the committed table and then
committed a test that plants nothing into the checker.

★★★ **I-5 is the canonical defect class, and rule (2) cannot see it.** The quote is verified to exist
*somewhere in the form's file* — never bound to the line number. A row can name line 4, quote line 9,
and pass; demonstrated by giving f8995 line 5 the verbatim text of line 9. **The committed table
already carries one instance** (`addl` carrying line 12's sentence). This is `CLAUDE.md`'s standing
root cause — Form 6251 line 33 transcribed as *"Subtract line 32 from line 12"* where the form says
line 22 — and *"No review would have caught it."* Rule (2) would not have either.

★★ **I-2/I-3 are the same shape I congratulated the instrument for catching.** The "derived, never
hand-listed" completeness scan is rooted at a **hand-chosen directory**, so `Form8283Row`,
`Form8949Row` and `ScheduleDPart` — in `crates/btctax-core/src/forms.rs`, one directory out — are
invisible; the crypto-slice 8949's **column (g) `adjustment_amount` is covered nowhere at all**. And
`mentions_ident` has a realized false negative: Form 8275's `Part1Item.amount`, a §6662 disclosure on a
filed return, is unseen because the emitter holds a `Form8275Content` and never types the name.
**`ScheduleBRow` — the module's own headline example — would not be demanded by (4b) today either.**

★★ **I-4: `_` on money is not forbidden, only the doc says so.** `line4: _` compiles, the row vanishes,
the count drops to 188, and nothing asserts on the count.

★ **I-6: the table contradicts itself across the two Schedule SE shapes** — line 10 is `Exception` in
one function and `Scaled` in the other; line 4c's Exception reason says truncating the quote *would
hide a real branch*, and the other row **does truncate and passes**. Rule (5) keys on the field name, so
it cannot see it — and a line can be re-filed under a second field name to make `MAX_EXCEPTIONS` go
DOWN. The ratchet's monotonicity is real; its meaning is not.

**Verbatim below.**

---

VERDICT: 0 Critical / 6 Important

Instrument lens, r6. Every claim below was verified by mutation and every mutation was restored (`cp` backup/restore, never `git checkout --`). Final `git status --short` shows only `?? reviews/BRIEF-r6.md`, which was untracked at the start of this review; `git diff --stat` is empty; `make check` is green at 2548 passed / 12 skipped.

---

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:496-565
CLAIM: The "B1 — the planted defects" test never calls run(), so no rule of the checker has ever
       been observed red, and the entire checker can be deleted with the suite staying green.
FAILURE: I replaced the whole body of run() with `return Ok("MUTATED: every rule removed".into())`
       and ran `cargo test -p xtask`: "55 passed; 0 failed; 1 ignored". Twelve rules, both ratchets
       and the completeness scan can all vanish and nothing reds.
EVIDENCE: The test's five parts assert on the FIXTURES, not on the verdict. (a) asserts
       `!text.contains(&normalize(paraphrase))` — a property of the extract file. (c)/(d) assert on
       string literals. (e) is the clearest:
           c.0.push(LineCoverage { production: Exception, reason: None, .. });
           assert!(matches!(c.0[0].production, Production::Exception) && c.0[0].reason.is_none(),
                   "★ the shape rule (1) rejects: an Exception with no reason");
       It constructs a row and asserts the row has the fields it just set. Rule (1) is never invoked.
       Contrast the model CLAUDE.md cites, cite_check.rs:796, which DOES call the checker:
           assert_eq!(unverified_quotations(paraphrase, ...).len(), 1);
       B1: "no checker exists until it has been observed RED on a planted defect" — and its own
       reviewable question, "which test reds when this checker is removed?", answers: none.
```

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:304-312 (the (4b) completeness scan root)
CLAIM: The "derived, never hand-listed" completeness scan is rooted at a HAND-CHOSEN directory, and
       Form 8283 — a form btctax emits, with three money columns — is entirely outside it, with zero
       coverage rows and no complaint.
FAILURE: `crates/btctax-forms/forms/2024/f8283.map.toml` and `.../2025/f8283.map.toml` exist, so
       btctax fills Form 8283. `form8283.rs` writes `cost`, `fmv` and `deduction` cells from
       `btctax_core::forms::Form8283Row { cost_basis: Usd, fmv: Usd, claimed_deduction: Option<Usd> }`.
       `line_coverage::all()` has no f8283 rows and the checker prints OK. Same for the CRYPTO-SLICE
       Form 8949: `fill8949.rs:36 row_cells(r: &Form8949Row)` writes proceeds, cost_basis,
       adjustment_amount and gain — and `adjustment_amount` (column (g)) has no counterpart on the
       covered `Printed8949Row` at all, so it is covered nowhere.
EVIDENCE: `let tax_dir = root.join("crates/btctax-core/src/tax");` … `read_dir(&tax_dir)`. All three
       types (`Form8949Row`, `Form8283Row`, `ScheduleDPart`) live in `crates/btctax-core/src/forms.rs`.
       Re-running the checker's own predicate with the scan rooted at `crates/btctax-core/src`
       reports exactly those three as missing a `cover_*()`. This is the SAME shape the MAX_EXCEPTIONS
       comment celebrates catching — "the CRYPTO-SLICE Schedule SE … a form shape the derived scope
       predicate found and no hand-list contained" — except the 8949/8283 crypto-slice twins sit one
       directory outside the scan, so the predicate could not have found them either.
```

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:335, 467-481 (mentions_ident as the scope predicate)
CLAIM: "A type is IN SCOPE iff the emitter crate names it in real code" has a false-negative
       direction that is realized today: a money type reached only through a parent's field is
       "not printed", and Form 8275's Part I amount is exactly that.
FAILURE: `crates/btctax-forms/src/form8275.rs:240` does
           push_free(&mut w, &mut p, &row_map.amount, &fmt_money(item.amount));
       filling `topmostSubform[0].Page1[0].Table_Part1[0].LineN[0].p1-tNN[0]` from
       `btctax_core::tax::form8275::Part1Item.amount: Usd` — the as-filed §6662 disclosure amount on a
       filed return. `Part1Item` IS inside the scanned directory, but btctax-forms never names the
       identifier (it holds a `Form8275Content`), so `mentions_ident` returns false, rule (4b) does
       `continue`, and there is no f8275 row and no complaint.
EVIDENCE: Replaying the predicate over `tax/*.rs` puts `ScheduleBRow`, `Printed8949Totals` and
       `Form1040Income` in the SAME position — out of scope by the predicate, covered only because a
       human chose to. So the module's own headline example is still undetectable: the doc says
       "`ScheduleBRow.amount` reached paper while nothing in the table mentioned it" is "the failure
       (4b) exists to close", and (4b) would not demand `cover_schedulebrow` today. The predicate
       decides printedness by whether a NAME is typed in the emitter, and money reaches paper through
       field access, which types no name.
```

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/line_coverage.rs:17-20 (the "_ is FORBIDDEN on money" claim)
CLAIM: The compile-time guarantee is that a money field must be NAMED, not that it must be
       CLASSIFIED. `field: _` satisfies the compiler and the checker, and drops the line silently.
FAILURE: I changed `line4,` to `line4: _,` in `cover_schedule2lines` and deleted its `c.line(...)`.
       It compiles, and `cargo run -p xtask -- line-coverage` prints
         "line-coverage OK: 188 money lines across 14 form(s) [… f1040s2:3 …]"
       Schedule 2 line 4 — Self-employment tax, a real printed money line — is gone, and the only
       visible difference is a row count nothing asserts on.
EVIDENCE: The doc comment says "Here `_` is **FORBIDDEN** on money: a `Usd` must be passed to
       [`Coverage::line`], which consumes it." Nothing forbids it — `#![deny(unused_variables)]` is
       satisfied by `_`, there is no lint, and there is no floor ratchet on row count nor any rule
       that the row set for a form matches the form's lines. So the ROW set is a hand-written list,
       against CLAUDE.md's standing rule that a conformance KAT must "enumerate the expected line set
       **from the form's extracted text**, never from a range or a hand-written list". The compile
       error a new field raises says "name me"; the minimum edit that silences it is `field: _` plus
       `field: Usd::ZERO` in the `zero_*` builder — two mechanical edits, zero coverage.
```

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:212 (rule (2))
CLAIM: Rule (2) verifies the quote exists SOMEWHERE in the form's text; it never binds the quote to
       the line number, nor the value to either. A row can name line 4, quote line 9, and pass line 3.
FAILURE: I gave Form 8995 line 5 the verbatim text of line 9 ("REIT and PTP component. Multiply line
       8 by 20% (0.20)" in place of "Qualified business income component. Multiply line 4 by 20%
       (0.20)"). The checker printed OK, 189 lines, 14 forms, unchanged.
EVIDENCE: `if !text.contains(&normalize(e.instruction))` — a whole-file substring test; `e.line` is
       used only for error messages and rule (5)'s key. `Coverage::line(_value, form, line, field,
       production, instruction)` takes six independent parameters and ignores `_value` entirely, so
       the value↔line↔quote triple is unbound in all three directions. This is precisely the class
       CLAUDE.md names as the standing root cause — Form 6251 line 33 transcribed as "Subtract line
       32 from line 12" where the form says line 22, "No review would have caught it." Rule (2) would
       not have caught it either. The committed table already carries one instance:
         c.exception(*addl, f, "(none)", "addl", "Self-employment tax. Add lines 10 and 11.", …)
       — line 12's sentence attached to a field the reason itself says is "NOT a Schedule SE line".
       The quote is verbatim; it is verbatim from the wrong line, which is what rule (2) cannot see.
```

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/line_coverage.rs:742 vs :2053, and :719 vs :2039
CLAIM: The table gives CONTRADICTORY dispositions to the same (form, line) across the two shapes of
       Schedule SE, rule (5) cannot see it because it keys on the field name, and the MAX_EXCEPTIONS
       ratchet — the number two full review rounds were spent estimating — is understated as a result.
FAILURE: f1040sse line 10, identical instruction, two verdicts:
         ScheduleSeLines: c.exception(*line10, f, "10", "line10",
             "Multiply the smaller of line 6 or line 9 by 12.4% (0.124)",
             "A composition the grammar cannot express — the operand is Bounded … and the result is
              Scaled …, so neither production alone states the line or its blank-ness rule.")
         SeTaxResult:     c.line(*ss, f, "10", "ss", Production::Scaled,
             "Multiply the smaller of line 6 or line 9 by 12.4% (0.124)")
       Same form, same line, same sentence: Exception in one function, Scaled in the other. By the
       table's own written reason, `Scaled` is wrong; by `Scaled`, the Exception is wrong. One of the
       two must be, and no rule can say which.
EVIDENCE: f1040sse line 4c is the sharper case, because the second row does exactly what the first
       row's reason forbids in writing:
         ScheduleSeLines: c.exception(*line4c, …, "Combine lines 4a and 4b. If less than $400, stop; …
              Exception: If less than $400 and you had church employee income, enter -0- and continue",
              "… Combine is rejected by rule (4) and TRUNCATING THE QUOTE TO PASS IT WOULD HIDE A REAL
               BRANCH.")
         SeTaxResult:     c.line(*base, f, "4c", "base", Production::Combine, "Combine lines 4a and 4b")
       — the truncated quote, classified Combine, passing rule (4) because the truncation removed the
       "-0-". Rule (5) keys `(form, line, field)` and the fields differ ("line4c" vs "base"), so the
       contradiction is invisible. Consequence for the ratchet: a line filed as an Exception under one
       field name can be re-filed under a second field name with a truncated quote and a production,
       and MAX_EXCEPTIONS goes DOWN. The ratchet's monotonicity is real; its MEANING is not.
       (f1040sse line 2 is a third, milder instance: Carry vs Collected on the same line.)
```

---

**Minor / Nit (recorded, not gating)**

- **M-1 — `money_bearing_types` blind shapes, confirmed by planted probes.** I added five probe types to `tax/printed.rs`, each named by `btctax-forms` so the scope predicate admits them. Detected: `pub amount: Usd` (control) and `pub amount : Usd` — the brief's spacing guess is wrong, `": Usd"` is a substring of `" : Usd"`. **Not detected:** `pub amounts: Vec<Usd>`; `pub enum E { Amount(Usd) }`; `pub struct S(pub Usd)` (explicitly skipped by the `;`-before-brace test); `pub amount:Usd` with no space after the colon. Also, `read_dir(&tax_dir)` is **non-recursive** and filters on a `.rs` extension, so a directory module `tax/schedule_1a/mod.rs` is invisible — which matters now, because the comment justifying the widening says its purpose was to see "a new `schedule_1a.rs`", and Schedule 1-A is the next build. (`crates/btctax-core/src/tax/fixtures/` already establishes that subdirectories occur here.)
- **M-2 — `shipped_tables_are_the_validated_tables.rs` skips two money fields of `TaxTable`.** `FullReturnParams` is complete: all 15 fields are compared. `TaxTable` has seven; `year`, `ordinary`, `ltcg` and `ss_wage_base` are compared, `source` is metadata, and **`gift_annual_exclusion` (§2503(b)) and `gift_lifetime_exclusion` (§2010(c)(3)) are not compared at all** — unmentioned in the file, in the `MAX_UNVALIDATED` ratchet, and in §G-25. They agree today ($18,000 / $13,610,000 on both sides), so this is a missing edge rather than a divergence, and `tax_tables.rs`'s own KATs pin the shipped side. But nothing makes a NEW field of either struct appear in this file — there is no exhaustive destructure — so "field by field" holds only for the fields somebody listed, which is the same shape as the header's own indictment. `STATUSES` is likewise a hand-list of 5 rather than derived from `FilingStatus`.
- **N-1** — `rustc-ice-2026-08-02T04_37_20-3125185.txt`, a compiler ICE dump, was committed to the repo root in this range.
- **N-2** — the "is this form emitted?" fallback keys on `forms/{year}/{form}.map.toml`, but two of the fourteen covered stems do not follow that convention (`f1040sd` → `schedule_d.map.toml`, `f1040sse` → `schedule_se.map.toml`). Harmless today because both extracts exist; if either extract were lost, the checker would mis-diagnose it as "the form name is wrong" instead of counting it unverifiable. Fail-closed, but it names the wrong cause.

---

**ALSO CHECKED, SOUND:**

- `Coverage::exception` / rule (1) shape logic itself is correct in both directions (a reason on a non-Exception is an error too) — it is only never *observed* (I-1).
- `normalize()` genuinely fixes the `pdftotext -layout` wrapping class; f8995 line 8's wrapped clause matches.
- `FLOOR_IDIOMS` / `CEIL_IDIOMS` are fail-closed as advertised: a `Clamped` row matching no idiom is an error, not a silent pass, and the `"-0-"` half of rule (3) is what correctly forces f1040:34 into the Exception bucket rather than blessing a clamp the form does not state.
- Rule (5)'s widening to `(form, line, field)` is right for its stated purpose (`ScheduleBRow.amount` legitimately serves Schedule B lines 1 and 5); the contradiction in I-6 is a consequence, not a reason to revert it.
- `zero_scheduleblines()` seeding one row in each `Vec` really does prevent the nested coverage from being vacuous — an empty `Vec` would have emitted zero rows silently.
- `MAX_UNVERIFIABLE` went 10 → 0 for the stated reason: `design/forms/extract/f1040s1--2024.txt` is committed, `legal/_provenance/fetch_log.tsv` records the fetch, and all ten Schedule 1 quotes verify.
- The eleven exceptions are, individually, honest misfits and not nearest-fit dodges hiding a wrong figure — f1040:12 (larger-of), f1040:16 (Tax Table), f1040sse:4a (two-branch), f1040sse:10 and QDCGT:3 (compositions), f1040s1:21 and f1040s3:11 (worksheet-produced) each read correctly against the extracted text. My I-6 finding is about their *count* and their *consistency across the two Schedule SE shapes*, not about any one of them being a wrong disposition.
- `shipped_tables_are_the_validated_tables.rs::a_single_moved_bracket_is_detected` is a real B1 kill: it calls the comparison, moves one threshold by a dollar, asserts inequality, AND asserts every other bracket still matches so the comparison is not always-unequal. This is the standard I-1 fails.
- The MFS/HoH ratchet is symmetric (neither side is required to carry every status) and correctly labels QSS's absence as lawful under §1(a)/§2(a) rather than a gap.
- `Cargo.toml` version bumps and `xtask/src/main.rs` wiring are inert. The `line-coverage` subcommand is not a separate CI step, but the xtask test that calls `run()` is in the workspace suite CI runs, so the table IS checked on every commit — that half of the claim holds.

**WHAT WOULD MAKE THIS REVIEW WRONG:**

1. If `Production` is intended purely as documentation and nothing downstream will ever consume it, then I-6 is a doc inconsistency rather than a defect — but the productions each state a *blank-ness rule* ("blank iff every operand is blank"), and §G-11 P0b/T3a plan to make `LineEntry` honour them, at which point f1040sse:10 and 4c would be *built* two different ways from one form line. Note also that the shipped writer already contradicts the table: f1040:37 is declared `Combine` yet `form1040_full.rs` now gates it on `line24 > line33`. That gate is right for the return; it means the declared production is not the thing the emitter obeys, and nothing checks that it is.
2. If Form 8283 / Form 8275 / the crypto-slice Form 8949 are considered out of §G-11's scope by an explicit decision I did not find, I-2 and I-3 collapse to "the doc should say so". I searched `FOLLOWUPS.md` §G-11 and the module docs and found the opposite: the module's stated limit is *nested money*, now closed, with no carve-out for crypto-slice forms — and the MAX_EXCEPTIONS comment treats finding the crypto-slice Schedule SE twin as the mechanism working. Under that reading the 8949/8283 twins are the same class, unfound.
3. If a `_`-bound money field is expected to be caught by human review of the diff rather than by the compiler, I-4 is a doc-wording fix (delete "FORBIDDEN") rather than a defect. I read it as a defect because the module's entire thesis is that review is the wrong instrument for this question.
4. I did not attempt to construct a false POSITIVE for `mentions_ident` (a type named only in a string literal, a `#[cfg]` block, a `/* */` comment, or a `#[cfg(test)] mod tests` inside btctax-forms — all of which the emitter-code filter admits, since it strips only lines beginning `//`). That direction over-demands coverage rather than under-demanding it, so I judged it not worth budget; if over-demand ever pushes someone to invent a row for an inert type, that judgement was wrong.