# VERIFICATION ledger — spec-1099da r1 review (2026-09-06-spec-1099da-review.md)

Controller: Claude Fable 5.1, 2026-09-06. Every measurable claim in the report re-run against the
tree at `2aa4ea98` before anything was folded. Method: `sed -n` on the cited lines, `grep -n` for
symbols, python over the CFR XML (ugrep's regex limit refused the `[^<]{0,n}` patterns).

| finding | claim | check | verdict |
|---|---|---|---|
| S1 | the form's own Note: (e) = basis as reported, (g) = the correction | `f8949--2025.txt:54-55` and `f8949--2026-DRAFT.txt:92-93` print it verbatim; `i8949--2025.txt:1018-1027` says the same; `:1000` "For most transactions, you don't need to complete columns (f) and (g) and can leave them blank" | **TRUE** |
| S2 | the crypto-slice arm fills 8949 with no `ReturnInputs` | `admin.rs:573-600`: `export_irs_pdf_from_session` branches on `return_inputs::exists`; the else-arm calls `btctax_core::form_8949(state, tax_year)` (`:631`) and `fill_form_8949` (`:716`) | **TRUE** |
| S3 | 1099-DA basis box is 1g, not 1e | `Instructions_1099-DA.txt:453` "Box 1e. Date Sold or Disposed"; `:498` "Box 1g. Cost or Other Basis"; `i8949--2025.txt:1052-1055` "box 1e on Form 1099-B or box 1g on Form 1099-DA"; "box 1e" occurs at spec `:89`, `:133`, `FOLLOWUPS.md:6164`, strategy review `:100` | **TRUE** (the strategy review is a persisted verbatim report and is NOT edited; FOLLOWUPS is) |
| S4 | covered = acquired in that account at that broker on/after 2026-01-01; transferred-in covered only with a §1.6045A-1 statement; one broker issues both kinds | CFR XML `(a)(15)(i)(J)` "acquired in a customer's account by a broker providing custodial services … on or after January 1, 2026, in exchange for cash …"; `(a)(15)(i)(G)` "transferred to an account if the broker … receives a transfer statement (as described in § 1.6045A-1)"; `(a)(16)` "any specified security that is not a covered security"; `Instructions_1099-DA.txt:597-600` (box 9 ⇒ 1g/box 2 optional), `:548-551` (voluntary basis on noncovered) | **TRUE** |
| I1 | `FormQuestion` is boolean/singular; `testonly` auto-answers with `neutral`; `RefuseReason` has no `Copy` | `questions.rs`: `get: fn(&ReturnInputs) -> Option<bool>`, `set: fn(&mut ReturnInputs, bool)`, `neutral: bool`, `unanswered: RefuseReason`; `classifier.rs:72` `fn declaration(&mut self, _leaf: &Option<bool>, id)`; `testonly.rs:34-40` `(q.set)(ri, q.neutral)`; `return_refuse.rs:35` `#[derive(Debug, Clone, PartialEq, Eq)]` | **TRUE** |
| I2 | no `forms/2026/`; `for_year` globs with `_ => None` | `ls crates/btctax-forms/forms/` → 2017 2024 2025; `build.rs` emits `_ => None`; test `the_information_return_regime_is_declared_per_year` at `tests/year_record.rs:96` | **TRUE** |
| I3 | 2025 regime `proceeds = true` | `forms/2025/YEAR.toml:39-41` | **TRUE** |
| M1 | C/I operative sentence at `:416-417`; F/L at `:462-466` | printed | **TRUE** |
| M2 | `broker_reporting_advisory` :467; `exchange_wallet` :64; `box_field`/`box_on` :18-19/:38-39 | `grep -n` | **TRUE** |
| M3 | `ScheduleDLines` has 18 `lineNN` fields, none of 1b/2/8b/9 | `printed.rs:935-972` field list: 1a_{d,e,h} 3_{d,e,h} 6 7 8a_{d,e,h} 10_{d,e,h} 13 14 15 16 | **TRUE** |
| M4 | `admin.rs:471-475` names A/B/D/E for pre-2025 | printed | **TRUE** |
| M5 | Schedule D prints "with Box A or Box G checked" | `f1040sd--2025.txt:32-33` | **TRUE** |
| M6 | `n_pages = max(⌈st/cap⌉, ⌈lt/cap⌉)`, chunk k paired with chunk k | `lib.rs:119-137` | **TRUE** |
| M7 | TD 10000 text is column-mangled; the CFR XML is clean | the CFR paragraphs above read as prose | **TRUE** |
| M8 | `exchange_wallet` sets `account = "default"`; multi-account is "future — FOLLOWUPS" | `normalize.rs:63-64` | **TRUE** |

**Verdict: 15/15 claims TRUE, 0 refuted.** Two facts the fold ADDS that the report did not check:
`btctax-forms` depends on `btctax-core` (`crates/btctax-forms/Cargo.toml:16`), so core cannot read
`YEAR.toml` — the regime must reach `form_8949` as a value; and `default_year()` is the newest
bundled year (`year_readiness.rs:203-208`), so bundling `forms/2026/YEAR.toml` (I2's T0) moves the
default year to 2026.
