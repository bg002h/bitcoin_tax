# R0 review — SPEC_irs_form_fill_sp2.md — round 3 (delta check on the R0-round-2 fold)

- **Artifact:** `design/SPEC_irs_form_fill_sp2.md` @ `feat/irs-form-fill-sp2` `c8b3813` (main == `99f26ca`).
  Spec's own last commit is `c8b3813` ("fold SP2 R0 round 2 (0C/1I/3M/2N, Opus) — awaiting round 3 (changed lines)").
- **Reviewer:** independent architect (R0, round 3; model: Opus 4.8 [1m]). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical (filling 3 OFFICIAL IRS PDFs).
- **Charge:** delta check ONLY — verify each round-2 fold (I★1, M1, M2, M3, N1, N2, the T1 "add SE
  computation" note) is captured correctly and introduced no new contradiction. NOT a re-derivation.
  Round 2 already confirmed all 4 Criticals + the C3 oracle correct/sound.
- **Method:** diffed `80ba633..c8b3813` on the spec (the exact changed lines); re-verified the load-bearing
  source facts the I★1/M3 folds rest on against this commit — `schedule_d.rs:24-27,101-110` (`active()` gate),
  `forms.rs:156-172` (`schedule_d()` non-`Option`, all-zero default), `lib.rs:56-58` (`fmt_money`),
  `admin.rs:80-98` (`ss_wage_base` via `BundledTaxTables`/`table_for` + `p.w2_ss_wages`). No PDF re-dumps
  (round 2 already reconfirmed the leaf counts + geometry; no round-2 edit touched a PDF fact).

## VERDICT: **0 Critical / 0 Important / 2 Minor / 2 Nit → R0-GREEN (cleared to implement)**

Every round-2 fold is captured **correctly** and each is source-verified. The one Important from round 2
(**I★1**, 7a in the DA-yes/no-Schedule-D-activity year) is folded soundly into the authoritative body, with
all four line-7a cases now unambiguous and a pinning KAT named. The round-2 edit introduced **no new blocking
contradiction**. It did leave two derivative-summary lines (Gotchas `[C2]`, Scope) un-synced with the refined
body — real but non-blocking (the authoritative body + the named KATs govern the implementation). Two nits on
the `-0-` glyph and the `$176,100` literal. **The folds are clean.**

---

## Per-fold verification (delta only)

### I★1 — 1040 line 7a: fill iff Schedule D ACTIVE ∧ line 16 ≥ 0; inactive ⇒ BLANK; `-0-` reserved for active-netted-zero — ✅ CORRECT, source-verified, all four cases covered
- **Body captured** (spec 80-92): "Fill 7a ONLY when Schedule D is **ACTIVE** (there are capital disposals)
  **AND line 16 ≥ 0**"; "If Schedule D is **INACTIVE** … **leave 7a BLANK even when DA = YES**"; "Reserve
  `-0-` for the active-netted-to-zero case only"; "On a NET LOSS, leave 7a BLANK + notice". Matches the
  round-2 fix sentence verbatim in intent.
- **Both premises verified in source** (this commit):
  - `fill_schedule_d_totals` writes line 16 **only** under `if st_active || lt_active` (`schedule_d.rs:101-110`)
    → on an income-only / donation-only year the attached Schedule D line 16 is **BLANK**. ✓
  - `schedule_d(state, year)` is **non-`Option`**, `ScheduleDTotals::default()` = all-zero (`forms.rs:156-172`)
    → a naïve 1040 fill would compute `line 16 = 0` and (pre-fold) stamp `-0-`. The fold's ACTIVE conjunction
    is exactly what suppresses that. ✓
- **Self-consistent with the attached Schedule D?** YES, on the substantive point: 7a is now filled **iff**
  Schedule D line 16 is filled (both gated on `st_active || lt_active`), so btctax never stamps a
  zero-capital-gains claim on 1040 while its own Schedule D line 16 is blank. The I★1 defect is closed.
- **All four cases unambiguous** (the charter's explicit check):
  1. **active-gain** (`st_active||lt_active`, line 16 > 0) → 7a = line 16 amount (= filled Schedule D 16). ✓
  2. **active-zero** (active, line 16 == 0 — e.g. ST +100 / LT −100, or proceeds=cost) → 7a = `-0-`; Schedule D
     line 16 is filled (= `fmt_money(0)`). Reachable and distinct from case 4. ✓
  3. **active-loss** (active, line 16 < 0) → 7a BLANK **+ §1211 loss notice**. A loss makes a part active
     (`active()` is true on nonzero `gain`, incl. negative), so this is a real, separately-handled case. ✓
  4. **inactive** (no disposals) → 7a BLANK, **no** loss notice. ✓
  Cases 3 and 4 both blank 7a but are disambiguated by the presence/absence of the loss notice; cases 2 and 4
  are disambiguated by the ACTIVE gate (the whole point of the fold). No overlap, no gap.
- **KAT present:** `form_1040_7a_blank_when_schedule_d_inactive` (spec 91-92: "income-only year, DA=YES, 7a
  empty"). This is the pinning test the charter asked for. ✓ (Body also keeps
  `form_1040_line7a_gain_equals_schedule_d_line16`, `…_loss_is_blank_with_notice`, `form_1040_7b_checkboxes_untouched`.)
- **Residual (non-blocking):** the ACTIVE flag isn't returned by `schedule_d()` (totals only) — the 1040 fill
  must recompute activeness (mirror `active(st) || active(lt)`, or read `form_8949` rows). Derivable from the
  same data; an impl note, not a spec hole. (See also Minor M1 below on the stale Gotchas summary, and Nit N1
  on the `-0-` glyph.)

### M1 — 8283 ordinal-y descent is PER-COLUMN — ✅ CORRECT
Spec 27-28: "**[R0-M1] 8283 ordinal-y must be PER-COLUMN** — Section A rows A–D split across two y-bands / two
tables, so the descent is asserted within each column's own field set, not globally." Captures the round-2 M1
exactly (the ColsA-C @ y≈432–528 vs ColsD-I @ y≈348–396 split). ✓

### M2 — fault-inject KAT covers BOTH a cross-column and a same-column swap — ✅ CORRECT
Spec 34-36: "the fault-inject KAT must cover BOTH a cross-column swap (caught by column-x, e.g. SE 12↔13) AND
a same-column swap (caught by ordinal-y, e.g. SE 10↔11) — one of each proves both oracle legs bite." Both
oracle limbs are now pinned by test. ✓

### M3 — SE line 9 threads `ss_wage_base`; signature `fill_schedule_se(se_result, w2_ss_wages, ss_wage_base, year)` — ✅ CORRECT + reachable
- Body (spec 67-68): "9(= line 7 − 8d, where line 7 is the `ss_wage_base` $176,100 constant — **[R0-M3] thread
  `ss_wage_base` in too**)". Plan T1 signature (spec 122-123): `fill_schedule_se(se_result, w2_ss_wages,
  ss_wage_base, year)` — exactly the charter's expected signature. ✓
- **Reachability verified:** at the export site the year's `TaxTable t` (holding `ss_wage_base`) is loaded via
  `BundledTaxTables::load()` + `tables.table_for(y)` (`admin.rs:85-87`, comment at :80 names "the year's
  `ss_wage_base`"), and `p.w2_ss_wages` from the profile (`admin.rs:93`). So both new fill inputs are
  supply-able where SP2 adds the SE computation. ✓
- The retained `year` param alongside `ss_wage_base` is mild belt-and-suspenders (year still drives
  `require_year` / map-edition selection); not a contradiction. (See Nit N2 on the `$176,100` prose literal.)

### N1 — DA-YES treats ANY `income_recognized` as qualifying — ✅ CORRECT
Spec 96-98: "**[R0-N1]** treat ANY income_recognized as qualifying (mining/staking/airdrop/interest/reward all
= clause-(a) receipt) — NOT a narrow kind whitelist." The four kinds are now explicitly examples, not a
filter. The `form_8949 ∨ ANY income_recognized ∨ Gift/Donate removal` predicate reads correctly, and the
gift signal is correctly sourced from `state.removals` ("`form_8283()` emits no gift rows; r2-confirmed"). ✓

### N2 — T2 pins Part IV/V positions by `/Rect` at map-extraction — ✅ CORRECT
Spec 127-129: "**[R0-N2] verify Part IV/V field positions at map-extraction** — the round-1 appendix's
tentative Part V donee cells actually fall in the Part IV appraiser block; pin each by its own `/Rect`, do not
trust the appendix names." Captures the round-2 heads-up. ✓

### T1 note — `export_irs_pdf` must ADD the SE computation — ✅ CORRECT + reachable
Spec 124-125: "**NOTE: `export_irs_pdf` today computes only 8949+SchedD — T1 must ADD the SE computation**
(`compute_se_tax` via `session.tax_profile(y)`; `w2_ss_wages` is reachable there, admin.rs:86)." The pattern
it points at is live in `export_snapshot` (`admin.rs:83-101`: `tax_profile(y)` + `table_for(y)` +
`compute_se_tax(... p.w2_ss_wages ...)`) — directly transplantable into the PDF export path. ✓

---

## New residue introduced/left by the round-2 edit

## Minor

### M1 — the Gotchas `[C2]` cheat-sheet line ("fill only on gain/zero") was NOT synced with the I★1-refined body, so read alone it still sanctions `-0-` on ANY zero — the exact defect I★1 removed.
The round-2 edit refined the **body** (7a filled iff Schedule D **ACTIVE** ∧ line 16 ≥ 0) but left the
derivative summary at spec 136-137 unchanged: "**[C2] 1040 line 7a (not 7); fill only on gain/zero;** loss ⇒
blank + notice; 7b unchecked." An implementer keying off the Gotchas alone would fill `7a = -0-` on an
income-only DA-yes year — precisely the I★1 defect. **Not blocking:** the authoritative body is unambiguous
*and* the body names the pinning KAT `form_1040_7a_blank_when_schedule_d_inactive`, so the specified TDD forces
blank-on-inactive regardless of the summary; the Gotchas line is explicitly labelled `[C2]` (a pointer to the
governing clause). Fix in one phrase, e.g. "fill only on gain / active-netted-zero (`-0-`); **BLANK when
Schedule D is inactive (income/donation-only) or a net loss**." (Round 3 flags it because the charter asks
"does any round-2 edit contradict an unchanged part?" — this is the one place it does; severity is Minor, not
Important, because the body + KAT govern the build.)

### M2 — Scope line 113 lists only `w2_ss_wages` as the CLI-threaded SE input; M3 added `ss_wage_base`, which is not reflected there.
Spec 113: "btctax-cli (`export-irs-pdf` extension; **thread `w2_ss_wages` into the SE fill**)." After M3 the
CLI must also supply `ss_wage_base` (from the bundled `TaxTable`, `admin.rs:85`). Incompleteness, not a
contradiction — but sync the Scope threading list to "`w2_ss_wages` **and `ss_wage_base`**" so the SemVer/scope
line matches the Plan-T1 signature.

## Nit

### N1 — the `-0-` for the active-netted-zero case does not *literally* mirror the filled Schedule D line 16, which uses `fmt_money` → `"0"` (native scale), not `"-0-"`.
Spec 82/86 justifies `-0-` as "mirroring the *filled* Schedule D line 16", but `fmt_money(0)` =
`Usd(0).to_string()` = `"0"`/`"0.00"` (`lib.rs:56-58`) — Schedule D line 16 shows `0`, not `-0-`. Cosmetic:
both are zero and `-0-` is the standard IRS glyph, and the substantive self-consistency (7a filled iff line 16
filled) is intact. Either drop the "mirroring the filled Schedule D line 16" clause or fill 7a with
`fmt_money(0)` to make the two glyphs identical. Trivial.

### N2 — Body line 69's `$176,100` skip literal should compare against the now-threaded `ss_wage_base` param, not a hard constant.
Spec 69: "If 8a≥$176,100, follow the form ('skip 8b–10')". With `ss_wage_base` now a parameter (M3), the impl
should test `8a ≥ ss_wage_base`; `176,100` is merely the 2025 value. Prose-only heads-up for T1; no defect.

---

## Answers to the round-3 charter
- **I★1:** ✅ folded correctly; both source premises re-verified (`active()` gate blanks line 16; `schedule_d()`
  non-`Option` all-zero); **self-consistent with the attached Schedule D**; all four cases
  (active-gain / active-zero / active-loss / inactive) covered **unambiguously**; KAT
  `form_1040_7a_blank_when_schedule_d_inactive` present. One residual hole is only in the **Gotchas summary**
  (Minor M1), not the governing body.
- **M1 / M2 / M3:** ✅ per-column ordinal-y; both-limb fault-inject KAT; `ss_wage_base` threaded with the exact
  `fill_schedule_se(se_result, w2_ss_wages, ss_wage_base, year)` signature (reachable at `admin.rs:85`).
- **N1 / N2:** ✅ ANY `income_recognized` (examples not whitelist); Part IV/V pinned by `/Rect` at extraction.
- **T1 note:** ✅ "`export_irs_pdf` must ADD the SE computation" — captured; the transplant source is live in
  `export_snapshot`.
- **Self-consistency:** the round-2 edit introduced **no new blocking contradiction**. Two derivative lines
  (Gotchas `[C2]`, Scope) trail the refined body → Minor doc-sync; the body + named KATs govern. The Plan
  (T0 oracle → T1 SE → T2 8283 → T3 1040) remains implementable with **0 open blocking questions** — T0 is
  unit-testable against the blank PDFs before any fill; the T1–T3 fault-inject KATs depend on T0.

## Bottom line
`c8b3813` is a faithful, source-verified fold of the round-2 0C/1I/3M/2N. The one round-2 Important (I★1) is
closed correctly in the authoritative body and the four line-7a cases are now unambiguous with a pinning KAT.
The only residue is two un-synced derivative summaries (Minor) and two nits — none blocking. **Verdict:
0 Critical / 0 Important / 2 Minor / 2 Nit → R0-GREEN (cleared to implement).** Optionally sync the Gotchas
`[C2]` line and Scope threading list on the way into T1/T3.
