# VERIFICATION ledger — spec-4868-1040v r2 review (2026-09-06-spec-4868-1040v-review-r2.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `264c0520`.

| finding | claim | check | verdict |
|---|---|---|---|
| N-I1 | the reader mis-joins the 4868's Part I: `f1_4` → "5", `f1_9`/`f1_10` → "9" | `xtask label-boxes f4868--2025`: `f1_4 5`, `f1_5 6`, `f1_6 8`, `f1_9 9`, `f1_10 9`, `f1_11 4` | **TRUE** — a `line1/2/3` map would red `every_mapped_line_lands_on_its_own_printed_label` |
| N-I2 | the row-band rule joins 0 boxes on the 1040-V; its labels are cell captions ~21pt above / 130pt left of their boxes | the reader's own refusal ("no numbered label column found", measured earlier); the reviewer's replay of the in-row predicate over the fixture (its numbers not re-run here — my fixture probe did not find the label words by text) | **accepted** |
| N-M1 | the out-of-country span breaks after "country" in the extract | `f4868--2025.txt:112-113` two-column interleave | **TRUE** |
| N-M2 | the recorded payment is a raw `Usd` that can carry cents | the field is `Usd` (C-1 ledger); the printed line 10 is `round_dollar` (`printed.rs:1543`) | **TRUE** |
| N-M3 | "+2 months" ≠ June 15 shifted by §7503; TY2017: 2018-04-17 + 2 months = 06-17 vs 06-15 | arithmetic; §1.6081-5 is NOT archived in `legal/` — the rule is taken from the form's own sentence ("by June 15, 2026") | **TRUE** (reg cite unverified in-tree) |
| N-M4 | `export_full_return` has three refusals | `admin.rs:984` "no full-return tables for {tax_year}…", `:990` "no return_inputs stored for {tax_year}", `:995` "the {tax_year} return is not computable […]" | **TRUE** |
| N-M5 | 26 mentions in 15 files; `return_1040.rs:1809` `pub extension_payment: Usd` on `PrintedInputs` | grep → 26 / 15; `:1809` verbatim | **TRUE** |
| N-M6 | `PrintedReturn` has `header` and `filing_status`; `ReturnHeader` has no `filing_status` | `packet.rs:472-473`; `:335-375` | **TRUE** |
| N-M7 | `promote_export_gate` runs on the packet path | `admin.rs:145`, `:621` | **TRUE** |
| N-N1 | `schedule_3_lines` returns `None` when lines 8 and 15 are zero | `printed.rs:1547-1549` | **TRUE** |

**Verdict: 10/10 (9 TRUE, 1 accepted), 0 refuted.**
