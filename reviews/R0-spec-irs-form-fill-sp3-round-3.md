# R0 review — SPEC_irs_form_fill_sp3.md — round 3 (delta / fold-verification)

- **Artifact:** `design/SPEC_irs_form_fill_sp3.md` @ `feat/irs-form-fill-sp3` `1c23802` (main == `55f5812`); the spec commit is
  `1c23802` "spec(irs-form-fill): fold SP3 R0 round 2 (0C/3I/5M/2N, Opus) — awaiting round 3".
- **Reviewer:** independent architect (R0, round 3; model: Opus 4.8 [1M]). Author ≠ reviewer.
- **Scope of THIS round:** a bounded delta check — confirm the round-2 folds (**I1/I2/I3 + M1–M4** + the two-stage-delivery rec)
  are captured correctly and introduced **no new contradiction**. The three round-1 Criticals and the C2/C3 facts were already
  re-verified TRUE in round 2; NOT re-litigated here.
- **Method:** read the folded spec end-to-end; re-measured the cheap facts against current source
  (`btctax-forms/src/{schedule_d,overflow,form8283,form1040,map,verify,lib}.rs`, `btctax-core/src/tax/tables.rs`) and the
  scratchpad PDFs (`f8283-rev{2014,2023}.pdf`, `schedD-{2017,2024}.pdf`, `f1040-{2017,2024}.pdf`) with the round-2 pypdf rig.

## VERDICT: **R0-GREEN — 0 Critical / 0 Important / 3 Minor / 1 Nit**

The 0C/0I bar is met. All six enumerated folds (I1/I2/I3, M1–M4) and the two-stage delivery are captured and coherent; the round-2
Importants are soundly closed. Three Minors + one Nit remain — two are **incomplete-fold** residue (the `SEC_B_CAP` twin of the
folded `SEC_A_CAP`; a stale hedge N2 asked to sweep) and one is a **fold-introduced** wording inconsistency (M1's KAT names a
`TaxTable` field that does not — and per the crate must not — exist). None blocks; **cleared to implement, starting with T0+T1.**
Fold the Minors opportunistically (all are one-line edits) — ideally before the T2/2017 branch, since two touch the 2017 8283 leg.

---

## Fold verification — each round-2 item (all captured unless noted)

**I1 — MoneyPair scope now covers the 2017 8283; overflow interaction; per-year cap — CAPTURED (one half of the cap fold dropped: see M-a).**
- Engine change #2 (spec lines 29–38) now states the 8283 explicitly: "**26 on the 2017 8283 Rev.12-2014 — NOT just SE+1040**"
  (`[R0-r2-I1]`), and "the DRAFT's '2024/2025 stay single-field' is right but 'SE+1040 only' was wrong." **Re-measured TRUE:**
  `f8283-rev2014.pdf` = 154 leaves, **26** `/Tx` `/MaxLen==3` (cents) fields; `f8283-rev2023.pdf` (2024) & `f8283-2025.pdf` = **0**.
  The residual line-33-vs-line-47 contradiction round 2 called out is resolved: line 38 "(All 2017-only; 2024/2025 stay single-field)"
  is now correct for the expanded scope (2024's rev2023 has 0 cent fields — verified).
- **Overflow survival — coherent, and the spec's phrasing fits `merge_copies`.** Spec: "the per-copy rename must rewrite BOTH the
  dollars and cents field names as a unit." `overflow.rs:43–46` renames **only the copy's root `/T`** (`btctaxcopy{k}`); every
  descendant's fully-qualified name inherits the new prefix (the code comment says exactly this). So a MoneyPair's dollars-fqn +
  cents-fqn — sharing that root prefix — move together automatically; they stay paired "as a unit." The spec's assertion is right;
  the only imprecision is that nothing new needs to "rewrite" the leaves — the existing root-rename already carries both. Benign.
- **`SEC_A_CAP` per-year — CAPTURED and TRUE.** Spec line 38: "`SEC_A_CAP` becomes per-year (5 for 2017, not the hardcoded 4)."
  `form8283.rs:48` = `const SEC_A_CAP: usize = 4`; `f8283-rev2014.pdf` Section A = `Line1A…Line1E` = **5** rows. Correct.
  **But see M-a:** the `SEC_B_CAP` twin (round-2 I1's fix said "add `SEC_A_CAP`/`SEC_B_CAP`") was NOT folded, and 2017 Section B = 4.

**I2 — BOTH grid tokens per-year — CAPTURED, and `schedule_d.rs:18` confirmed hardcoded.**
- Engine change #4 (lines 41–44) now names both: the 8949 `F8949_TABLE_TOKEN="Table_Line1_Part"` AND
  "`SCHED_D_TABLE_TOKEN` (schedule_d.rs:18, used :159) is hardcoded `Table_PartI` but 2017's is `TablePartI` (no underscore)" →
  "**Make BOTH grid tokens per-year map config**." **Confirmed:** `schedule_d.rs:18` = `const SCHED_D_TABLE_TOKEN: &str = "Table_PartI";`.
  Re-measured grid subform tokens: `schedD-2017.pdf` → **`TablePartI`** (32 leaves); `schedD-2024.pdf` → **`Table_PartI`** (32).
  On 2017, `"TablePartI".contains("Table_PartI")` is false → band derivation would fail closed on a correct map. Fold sound.

**I3 — 2017 1040 DA fields `Option` + guard skipped when absent — CAPTURED, coherent with the C2 adjacency logic.**
- Engine change #6 (lines 48–50) now reads: "make `Form1040Map`'s DA fields `Option` (parallel to the QOF-optional item) AND skip
  the `topmost_yes_no_pair`/adjacency DA guard when the DA field is absent — else `fill_form_1040_capgains` errors at
  form1040.rs:118 on the no-DA 2017 form." **Confirmed against source:** `Form1040Map.da_yes/da_no` are non-optional
  (`map.rs:106–108`); `form1040.rs:118` calls `topmost_yes_no_pair(&check, &fields, 0)?` unconditionally and returns
  `Err("no same-y {/1,/2} /Btn pair found")` when absent (`verify.rs:444–448`). `f1040-2017.pdf` page 0 has **no** {/1,/2} pair
  (only one `/2`-on-state widget total — no same-y two-widget {1,2} group). The "parallel to QOF-optional" claim is exact:
  `ScheduleDMap.qof_yes/qof_no` are likewise non-optional (`map.rs:270–272`) and #3 makes them `Option` — same pattern.
  Phrasing "skip the `topmost_yes_no_pair`/adjacency DA guard" correctly spans both the current (topmost) and the C2-fixed
  (adjacency) form of the guard. Coherent. The produce/skip rule (lines 71–72: "reportable capital activity only (no DA gate)")
  captures round-2 I3's "gate off line-13-will-receive-a-value, not `da_yes`." Sound.

**M1 — full-schedule equality lock — CAPTURED, but the fold added a spurious field (see M-b).** Line 80–82 now specifies
"a FULL-SCHEDULE equality lock (every ordinary bracket edge + rate, every §1(h) LTCG breakpoint, the $127,200 wage base, **the NIIT
threshold**), not a few spot-pins." The full-schedule intent is right (matches `ty2024_full_schedule_equality_all_28_edges_and_ltcg`).
**But "the NIIT threshold" does not belong** — the `TaxTable` struct carries no such field, and the crate forbids it (M-b).

**M2 — real 2-dp/zero-pad formatter — CAPTURED (comma decision left open, acceptable).** Line 34: "MoneyPair needs a REAL
2-decimal/zero-pad formatter (`fmt_money` is raw `Decimal::to_string` — no cents padding)." Confirmed: `lib.rs:61–63`
`fmt_money(d) = d.to_string()` (native scale, no padding, no commas). The 2-dp/zero-pad essence is captured. The round-2-M2
sub-question — whether the dollars part is **comma-grouped** to match the form's own preprint (`Line7Dollars='127,200'`) — is left
implicit; that is a cosmetic implementation call, not a blocker. (Noted, not re-raised.)

**M3 — per-year config enumeration — CAPTURED, comprehensive.** Gotchas lines 124–127 now list the full set: "box C/F + `/3`,
14 rows, **BOTH** grid tokens (8949 + Schedule D), QOF-optional, DA-optional, pre-filled-exempt, `SEC_A_CAP`, MoneyPair-vs-single,
the column-x clusters + page indexes, and the bundled PDF bytes — all map/config DATA keyed by year, not `if year==` ladders."
This absorbs round-2 M3's named members (x-clusters, page indexes, PDF-bytes). Only `SEC_A_CAP` is named among caps → M-a.

**M4 — `f1-_51[0]` glitch relocated to 2017 line 13 — CAPTURED and re-measured TRUE.** Facts table line 64 (2017 row):
"line 13 (¢-pair; the IRS-glitched field name `f1-_51[0]` is HERE — [R0-r2-M4])"; line 65 (2024 row): "line 7 (**no glitch fields**)."
**Confirmed:** `f1-_51[0]` appears only in `dump_f1040-2017.txt`, at `[482,336,554,348]` = line-13 dollars (`grep -c f1-_51`
on the 2024 dump = 0). Correctly moved off 2024 and flagged as the MoneyPair dollars field. The round-2 M4 booby-trap warning is preserved.

**Two-stage delivery (T0+T1 → SP3a; T2/2017 → SP3b) — coherent with the Plan, leaves one seam (see M-c).** Lines 113–116 rec is
consistent with the Plan's T0(engine+table)/T1(2024)/T2(2017) phasing; "the spec stays one coherent design" holds — the T2 work
(2017 maps, tax table, MoneyPair×overflow, no-DA 1040, §B SE) is fully specced, merely delivered on a second branch. Shipping the
TY2017 `TaxTable` in T0/SP3a ahead of the 2017 forms is harmless (a standalone data addition that also un-gates `report 2017`).
One seam: the man-page/README update is Plan-slotted in T2 (line 111) but 2024 support ships in SP3a (M-c).

---

## Minor

### M-a — The `SEC_B_CAP` per-year fold was dropped; the 2017 8283 Section B holds 4 rows, not 3 — and Section B is the BTC-donation path. (Incomplete I1 fold.)
Round-2 I1's fix said "add `SEC_A_CAP`/`SEC_B_CAP` to the per-year config." The spec folded only `SEC_A_CAP` (line 38: "5 for 2017,
not the hardcoded 4"; M3 config list names only `SEC_A_CAP`). **Measured:** `f8283-rev2014.pdf` Section B property rows = `Line5A…Line5D`
= **4 rows**; `f8283-rev2023.pdf`/`f8283-2025.pdf` Section B = `Line3A…Line3C` = **3**. `form8283.rs:49` = `const SEC_B_CAP: usize = 3`,
used at `:85` for `Form8283Section::B`. Since a qualified BTC donation (> $5,000 appreciated property) files in **Section B**, this is the
more load-bearing cap for the actual use case. Consequence is **benign** (a 4-lot 2017 Section-B donation mis-paginates to 2 copies —
3+1 — instead of 1; each copy stays within the form's 4-row capacity, so no fail-closed and no mis-render), which is why this is Minor
and not Important — but it is the exact twin of the `SEC_A_CAP` capacity issue the fold *did* capture, and it should be folded for
consistency. **Fix:** make `SEC_B_CAP` per-year too (4 for 2017, 3 for 2024/2025); add it to the M3 config list beside `SEC_A_CAP`.
KAT: `ty2017_8283_section_b_four_rows` / a 4-lot-fits-one-copy overflow assertion.

### M-b — M1's KAT description lists "the NIIT threshold" as part of the TY2017 full-schedule equality lock, but `TaxTable` carries no NIIT field — and the crate forbids putting one there. (Fold-introduced contradiction; re-opens the exact N1 trap.)
Line 80–82's equality-lock member list ends "…the $127,200 wage base, **the NIIT threshold**." **Source contradicts this:** the
`TaxTable` struct (`tables.rs:53–82`) is `{year, source, ordinary, ltcg, gift_annual_exclusion, ss_wage_base, gift_lifetime_exclusion}`
— **no NIIT field**; its doc-comments say "INDEXED to the year's Rev. Proc. — **never NIIT/loss-limit**," and the §1211 comment says
"**Must never be placed in a `TaxTable`**." `niit_threshold` is a **year-independent statutory function** (`tables.rs:190`,
`pub fn niit_threshold(status)`, fixed §1411(b) amounts). So a TY2017-table equality lock cannot (and must not) assert a NIIT threshold.
This also contradicts the spec's own C1 field list (lines 17–18: "ordinary brackets + LTCG breakpoints … SS wage base $127,200 …
SE rates" — no NIIT) and round-2 **N1**, which was raised *specifically to stop* an implementer from "completing" the 2017 table with a
NIIT field. Naming it in the lock re-opens that trap. **Fix:** delete "the NIIT threshold" from the line-80–82 member list (the lock is
ordinary edges+rates × 4 statuses, §1(h) breakpoints × 4 statuses, and `ss_wage_base`); optionally add N1's one-line "no NIIT/std-deduction
field" note the spec never captured. Source guardrails make the bad outcome unlikely, hence Minor — but it is a real internal contradiction.

### M-c — Two-stage seam: the man-page/README update is Plan-slotted in T2/SP3b, but 2024 becomes a supported year at SP3a (T0+T1) — SP3a would ship an undocumented supported year.
The Plan lists "man page + README" only in T2 (line 111), written when T2 was the final phase. The two-stage rec now merges T0+T1 as
SP3a (adding `export-irs-pdf --tax-year 2024`) *before* T2. As written, SP3a ships 2024 form-fill while `--help`/the man page still say
2025-only (scope line 99 lists "supported years 2017/2024/2025" as the *end-state*). The per-stage whole-diff review (line 116) would
likely catch this, so it is Minor. **Fix:** one clarifying sentence — each stage updates its own docs (SP3a: man page + README reflect
2024; SP3b: add 2017), rather than deferring all docs to SP3b.

---

## Nit

### N-a — Line 57 still hedges "dollars+cents pairs likely (verify at extraction)" for the 2017 8283, contradicting the now-definitive "26 cent fields" in engine change #2. (Round-2 N2 not swept.)
Round-2 N2 asked to replace this hedge with the measured fact. The fold added the definitive statement in the engine section (line 30)
but left line 57's "likely (verify at extraction)" — the very "verify-at-extraction" phrasing round-1 C3 rejected — so the spec now
states the same fact two ways (definitive vs hedged). Cosmetic; the engine section is authoritative. **Fix:** replace line 57's hedge with
"dollars+cents pairs (26 ¢-fields, measured); 5 Section-A rows."

---

## Answers to the charter's direct questions

1. **I1 captured + coherent; "rename both as a unit" fit `merge_copies`?** Captured (26 ¢-fields on the 2017 8283 stated definitively;
   overflow addressed; `SEC_A_CAP` per-year). The "as a unit" phrasing fits: `overflow.rs:43–46` renames only the copy root `/T`, and
   both the dollars- and cents-fqn inherit that prefix automatically, staying paired. **Gap:** the `SEC_B_CAP` half of I1's fix was
   dropped (M-a); 2017 Section B = 4 rows and is the BTC path.
2. **I2 both tokens per-year; `schedule_d.rs:18` hardcoded?** Yes — captured for both the 8949 and Schedule D tokens; `schedule_d.rs:18`
   is confirmed `const SCHED_D_TABLE_TOKEN = "Table_PartI"`, and 2017's grid is `TablePartI` (measured). Sound.
3. **I3 DA-`Option` + guard-skip; consistent with the C4/adjacency fill logic?** Yes — parallel to QOF-optional (both `CheckChoice`
   pairs made `Option`), the guard is skipped only when the DA field is absent, and the phrasing spans both the topmost and the
   C2-fixed adjacency form of the guard. Confirmed against `form1040.rs:118` / `verify.rs:444` / `map.rs`. Sound.
4. **M1–M4 captured?** M1 (full-schedule lock) captured but names a non-existent `TaxTable` NIIT field (M-b). M2 (real 2-dp/zero-pad
   formatter) captured; comma-grouping left open (acceptable). M3 (full per-year config set) captured comprehensively; only names
   `SEC_A_CAP` among caps → M-a. M4 (`f1-_51[0]` → 2017 line 13; 2024 has none) captured and re-measured TRUE.
5. **Two-stage delivery coherent with the Plan + single coherent design?** Yes — consistent with T0/T1/T2 phasing; the 2017 work is
   fully specced and merely deferred to SP3b; shipping the TY2017 table early is harmless. One seam: docs slotted in T2 but 2024
   ships in SP3a (M-c).
6. **Residual contradiction / NEW gaps / Plan implementable with 0 open blocking questions?** The round-1/round-2 contradiction
   (line 33 "single-field" vs line 47 "8283 ¢-pairs likely") is **resolved** by the expanded MoneyPair scope + the "All 2017-only;
   2024/2025 stay single-field" clause (verified: 2024/2025 8283 have 0 ¢-fields). Residuals found: a fold-introduced one (M-b, NIIT
   in the KAT lock), an incomplete-fold one (M-a, `SEC_B_CAP`), a two-stage seam (M-c), and a stale hedge (N-a) — all Minor/Nit, none
   blocking. **The Plan is implementable with 0 open blocking questions.**

**Round-3 disposition:** **R0-GREEN — 0C / 0I / 3 Minor / 1 Nit.** The three round-2 Importants are soundly closed and the folds are
coherent against current source + the real PDFs. Cleared to implement, **starting with the T0+T1 stage (2024 + engine)**. Fold the three
Minors + one Nit opportunistically — all are one-line edits, and two (M-a, N-a) touch the 2017 8283 leg, so ideally sweep them before
the T2/SP3b branch.
