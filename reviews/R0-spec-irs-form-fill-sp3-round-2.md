# R0 review — SPEC_irs_form_fill_sp3.md — round 2

- **Artifact:** `design/SPEC_irs_form_fill_sp3.md` (R0 round-1 folded, 3C/5I/5M/2N) @ `feat/irs-form-fill-sp3` `383636c` (main == `55f5812`)
- **Reviewer:** independent architect (R0, round 2; model: Opus 4.8 [1M]). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical feature (filling OFFICIAL IRS PDFs for PRIOR years 2017 + 2024).
- **Method:** evidence-driven, same rig as round 1. Re-measured the actual official PDFs with pypdf (scratchpad venv):
  `f1040-{2017,2024,2025}.pdf`, `fSE-{2017,2024,2025}.pdf`, `schedD-{2017,2024,2025}.pdf`, `f8949-{2017,2024,2025}.pdf`,
  and the two 8283 revisions `f8283-rev{2014,2023}.pdf` — leaf-field dumps (`/FT`/`/Rect`/`/MaxLen`/pre-set `/V`),
  same-y `/Btn` clustering + on-states, subform-token enumeration, page-text. Verified every round-1 fold and every
  engine claim against current source: `btctax-forms/src/{verify,map,pdf,fill8949,schedule_d,schedule_se,form1040,form8283,overflow,lib}.rs`,
  `btctax-adapters/src/tax_tables.rs`, `btctax-core/src/tax/{tables,se}.rs`.

## VERDICT: **BLOCKED — 0 Critical / 3 Important / 5 Minor / 2 Nit**

The three round-1 Criticals are **soundly folded and re-verified TRUE against source**: (C1) the TY2017 `TaxTable` is a
clean data addition — the `TaxTable` struct holds the pre-TCJA 7-rate schedule and the 2017 §1(h) breakpoints, nothing
hardcodes a supported-year set, and the primary-source KAT bar is right (§C1 below); (C2) the DA-pair adjacency fix is
**measured correct on all three years** — 2024's filing-status pair is a 266.4 pt trap, the real DA pair is 36 pt, 2025's
DA pair is 36 pt and stays selected, 2017 has no {1,2} pair at all (§C2 below); (C3) the 8283 revision facts are
confirmed — Rev. 12-2014 has "j Other"/no DA box/5 Section-A rows, Rev. 12-2023 has "k Digital assets" (§C3 below).

**But the fold's "engine changes SP3 owns" enumeration — which the spec presents as authoritative, correcting the DRAFT's
false "no engine change" claim — is incomplete in three concrete, evidence-backed ways, each of which breaks or misrenders
a 2017 leg:** (I1) the `MoneyPair` scope is stated as "2017 SE + 1040 only … 2024/2025 stay single-field," but the **2017
Form 8283 (Rev. 12-2014) has 26 cent fields** — its money columns are dollars/cents pairs too, and it **overflows via
`merge_copies`**, so this is exactly the "MoneyPair × overflow" interaction the charter asked to hunt for (the spec even
says at line 47 "dollars+cents pairs likely (verify at extraction)" — I verified it: they exist; this is the same
"verify-at-extraction" hand-wave round-1 C3 rejected). (I2) the **Schedule D read-back token is per-year too** — 2017's
Part I grid subform is `TablePartI` (no underscore) vs 2024/2025's `Table_PartI`; `SCHED_D_TABLE_TOKEN` is a hardcoded
const the spec never mentions (the spec's "per-year table token" fold covers only the *8949* token), so a correct 2017
Schedule D map **fails closed at read-back**. (I3) the 2017 Form 1040 requires making `Form1040Map`'s DA fields optional
**and skipping the `topmost_yes_no_pair` guard** when the map has no DA field — a schema+engine change exactly parallel to
the enumerated per-year QOF-optional item, but omitted; without it `fill_form_1040_capgains` errors at `form1040.rs:118`
on the 2017 form. All three fixes are small and mechanical, but each is a required fold before implementation under the
0C/0I bar. None kills the sub-project.

---

## Round-1 fold verification (all PASS — evidence)

**C1 [TY2017 tax table] — FOLDED, sound.** Spec §"R0-C1" adds a TY2017 `TaxTable` to btctax-adapters (Rev. Proc. 2016-55
brackets/LTCG + $127,200 wage base) with a primary-source KAT.
- *Addable shape:* `TaxTable` (`btctax-core/src/tax/tables.rs:53-82`) is `{year, source, ordinary:
  BTreeMap<FilingStatus,OrdinarySchedule>, ltcg: BTreeMap<…,LtcgBreakpoints>, gift_annual_exclusion, ss_wage_base,
  gift_lifetime_exclusion}`. `OrdinarySchedule.brackets: Vec<OrdinaryBracket{lower, rate}>` is count-agnostic and
  rate-agnostic, so the pre-TCJA **7-rate** structure (10/15/25/28/33/35/**39.6%** = `dec!(0.396)`) fits with no schema
  change — the 2024/2025/2026 tables already carry 7 brackets each. The 2017 §1(h) breakpoints (0%/15%/20% keyed to the
  ordinary edges: `max_zero` = top of the 15% ordinary bracket, `max_fifteen` = the 39.6% threshold) are expressible as
  the two dollar amounts `LtcgBreakpoints{max_zero, max_fifteen}` — but these must be **pinned to the Rev. Proc. 2016-55
  dollar values, not computed from the schedule** (see M1).
- *Clean data addition:* `BundledTaxTables::load()` (`tax_tables.rs:72-78`) is `insert(2024,…); insert(2025,…);
  insert(2026,…)`; `table_for` is `by_year.get(&year)` (`tax_tables.rs:82-84`). Adding `insert(2017, ty2017())` is
  purely additive — **nothing hardcodes a supported-year set** (a missing year returns `None` →
  `NotComputable(TaxTableMissing)`). Confirmed.
- *Right bar:* "primary-source-verified by KAT" is correct — this un-gates the *SE wage base* (load-bearing for the
  packet's SE leg) AND, as a bonus, `report --tax-year 2017`, whose LTCG/ordinary math is tax-critical. The existing
  precedent is a full-schedule equality lock (`ty2024_full_schedule_equality_all_28_edges_and_ltcg`, `tax_tables.rs:1119`)
  precisely to close the "delta-cancellation hole" documented in that test's comment — the 2017 KAT should match that
  rigor, not just spot-check "known figures" (M1). Free cross-check: the blank 2017 SE preprints `Line7Dollars = '127,200'`
  (verified `/V`), which must equal `TaxTable.ss_wage_base` — the spec already names this KAT (round-1 I2). **Fold sound.**
- *Note on the charter framing:* the charter lists "NIIT threshold, std deduction" among "fields the struct needs for
  2017." It does **not** need them — `TaxTable` carries neither; NIIT threshold is statutory/year-independent
  (`tables.rs:190` `niit_threshold`), and the crate consumes post-deduction income so no std-deduction field exists. The
  spec is **correct** (and more precise than the charter) to list only ordinary brackets + LTCG + `ss_wage_base` + SE
  rates. See N1.

**C2 [DA-pair adjacency] — FOLDED, measured correct on all three years.** `topmost_yes_no_pair` exists at
`verify.rs:406-455` (charter cited ~406 — exact). Measured page-0 same-y `/Btn` {1,2} pairs:

| form | pairs (y desc) | dx | selected by `dx ≤ ~60` |
|---|---|---|---|
| **f1040-2017** | *(none)* | — | *(no DA question — correct; map has no DA field)* |
| **f1040-2024** | filing-status `c1_3` x=[106.8, 373.2] @y588; **DA `c1_5[0]/c1_5[1]` x=[508, 544] @y487** | 266.4 / **36.0** | **DA pair** ✓ (topmost-y currently wrongly picks filing-status) |
| **f1040-2025** | **DA `c1_10[0]/c1_10[1]` x=[522.4, 558.4] @y501** | **36.0** | **DA pair** ✓ (only {1,2} pair; no regression) |

The `dx ≤ ~60 pt` guard cleanly separates DA (36) from the filing-status trap (266.4) on 2024, and **preserves 2025**
(the sole {1,2} pair, dx 36, still selected). The spec's "re-verify 2025, no regression" claim (line 28) is confirmed.
**No form in scope has two *adjacent* {1,2} pairs**, so the selection is unique on all three years (see M5 for the
defensive tiebreak). **Fold sound.**

**C3 [8283 revisions] — FOLDED, confirmed.** Measured `f8283-rev2014.pdf` (TY2017): 154 leaves, **"digital asset" absent**
from text, **"Other" present** (the "j Other" box), appraiser + donee sections present, Section A rows =
**Line1A…Line1E (5 rows)** (2025 has 4). `f8283-rev2023.pdf` (TY2024): 117 leaves, **"digital asset" present**, Section-B
row tokens `Line3B/Line3C` matching 2025's 117-leaf structure. Filing-year→revision map (2017→12-2014, 2024→12-2023)
correct. **Facts sound** — except the 8283-2017 money columns are ¢-pairs (I1) and the 5-row Section-A capacity is
per-revision data the config must carry (I1).

**The 5 enumerated engine changes:**
1. **DA-pair adjacency** — sound (C2 above).
2. **`MoneyPair{dollars,cents}`** — the *mechanism* is sound (dollars field carries the column-x cluster + descent-y;
   cents field joins the `allowed`/no-unmapped set; a `FlatPlacement::free`-style entry suffices). Verified 2017-only for
   SE + 1040 (fSE-2017 = 29 ¢-fields; f1040-2017 = 79 ¢-fields; fSE-2024/2025 + f1040-2024 = 0). **BUT the scope is
   wrong** — it omits the 2017 Form 8283 (I1).
3. **Per-year QOF-optional** — `ScheduleDMap.qof_yes/qof_no` are non-optional (`map.rs:269-272`); `schedule_d.rs:111-119`
   always writes "No." Making them `Option` per year is correct and coherent. Sound.
4. **Per-year 8949 table token** — confirmed: `F8949_TABLE_TOKEN="Table_Line1_Part"` (`verify.rs:53`) matches 2025's
   `Table_Line1_Part1`/`Part2` but **not** 2017/2024's `Table_Line1`. The per-year-token fold is right — **but note
   `verify_8949` reads the const directly at `verify.rs:161`, so its signature must gain a `table_token` param** (small
   engine delta the fold should name; M3). Sound in intent.
5. **Per-year pre-filled-exempt set** — confirmed: fSE-2017 carries factory `/V` on exactly 4 text fields
   (`Line7Dollars='127,200'`, `Line7Cents='00'`, `Line14Dollars='5,200'`, `Line14Cents='00'` — full-width, so the SP2
   spacer exclusion misses them), which `assert_only_filled` (`verify.rs:236-252`) would reject. The exempt set is
   correct. *Checked the sibling risk:* the 42 pre-`/V` fields on f1040-2017 and the pre-`/V` fields on schedD-2017 are
   all `/Off` checkboxes (`/AS`-absent → `checkbox_on` returns `None` → not "filled"), so they need no exemption. The
   spec correctly scopes the exempt set to the SE text constants. Sound.

**Importants/Minors from round 1:** the 2017 SE §B long-form mapping (spec lines 66-67, strictly-descending chain, line
12 = 10+11, $400 floor) — captured; the 2017 no-DA produce/skip rule (income-only ⇒ skip) — captured in prose (line 63,
but see I3 for the missing engine mechanism); Box C/F `/3` + 14 rows — captured (14×8 grid re-measured on both years).

---

## Important

### I1 — The `MoneyPair` scope is wrong: the 2017 Form 8283 (Rev. 12-2014) ALSO splits money into dollars+cents pairs (26 ¢-fields), and it overflows via `merge_copies`. The spec scopes MoneyPair to "SE + 1040 only … 2024/2025 stay single-field," contradicting its own line-47 "¢-pairs likely (verify at extraction)". This is the MoneyPair × overflow interaction the fold left open.

**Evidence:**
- `f8283-rev2014.pdf`: **26 `/Tx` fields with `/MaxLen == 3`** (the cents columns), e.g. `p1-t17[0]` [353,420,367,432],
  `p1-t19[0]` [439,420,454,432] — the (g) cost and (h) FMV cents twins. `f8283-rev2023.pdf` (TY2024) and `f8283-2025.pdf`:
  **0** ¢-fields. So 8283 ¢-pairs are a **2017-only** trait — exactly like SE/1040 — and the spec's "single-field" claim
  is right for 2024/2025 but the "SE + 1040 only" claim omits the 2017 8283.
- `form8283.rs` writes every money value as a single `fmt_money(...)` cell via `push_cell`: `m.cost`
  (`form8283.rs:179`, `216`), `m.fmv` (`:180`, `199`), `m.deduction` (`:218`) — each a column-checked + descent
  `FlatPlacement::cell`. On the 2017 form these map to the DOLLARS field only, so a value like `"12345.67"` lands in the
  whole-dollars box with the ¢-box left blank — a **misrendered amount on an official, filed tax form** (the identical
  failure round-1 I1 called out for SE/1040).
- **Overflow interaction (the charter's hunt target):** `fill_form_8283` overflows Section A / B via
  `overflow::merge_copies` (`form8283.rs:87-99`). `merge_copies` renames only each copy's **root** `/T`
  (`overflow.rs:43-46`), so a MoneyPair's `dollars_fqn`+`cents_fqn` — sharing the root prefix — inherit the SAME new
  prefix and stay paired. The interaction is therefore benign **provided** the MoneyPair is modeled inside `fill_one`'s
  placements and read back by `verify_flat` per copy (which it is) — but **the spec must say so**; today it neither scopes
  MoneyPair to the 8283 nor addresses the overflow. (The 8949 grid — the other `merge_copies` user — has **0** ¢-fields
  on both years, verified, so it is untouched; the 8283 is the sole MoneyPair × overflow site.)
- **Section-A capacity is per-revision:** `SEC_A_CAP = 4` is hardcoded (`form8283.rs:48`), but Rev. 12-2014 has **5**
  Section-A rows (Line1A–1E). Left at 4, a 5-lot donation mis-overflows to two copies. `SEC_A_CAP` must become per-year
  map data (like `rows_per_page` for 8949).

**Fix:** fold the 8283 into the MoneyPair engine scope (verified: 2017 has 26 ¢-fields; 2024/2025 single-field), state
that `fill_one`'s money cells become MoneyPairs on 2017 and that `merge_copies` is compatible (root-rename keeps twins
paired), and add `SEC_A_CAP`/`SEC_B_CAP` to the per-year config. KATs: `ty2017_8283_money_is_dollars_cents_pairs` +
`ty2017_8283_five_section_a_rows` + a MoneyPair-across-overflow-copy fault-inject; `ty2024_8283_single_field_money`
(regression). Replace line 47's "likely (verify at extraction)" with the verified fact.

### I2 — The Schedule D read-back token is per-year too: 2017's Part I grid is `TablePartI` (no underscore), not `Table_PartI`. `SCHED_D_TABLE_TOKEN` is hardcoded and the spec never mentions it, so a CORRECT 2017 Schedule D map fails closed. The "per-year table token" fold covers only the 8949 token.

**Evidence:**
- Measured Part I grid subform tokens: `schedD-2017.pdf` → **`TablePartI`** (leaves like `…Line1[0].f1_003[0]`);
  `schedD-2024.pdf` → **`Table_PartI`**; `schedD-2025.pdf` → **`Table_PartI`**. The 2017 name differs from 2025 by a
  **missing underscore** (a sneaky one, not a `_Part1/_Part2` split like the 8949).
- `schedule_d.rs:18` — `const SCHED_D_TABLE_TOKEN: &str = "Table_PartI";`. `verify_schedule_d` always calls
  `column_x_bands(fields, 0, SCHED_D_TABLE_TOKEN)` (`schedule_d.rs:159`) to re-derive the d/e/g/h amount-column bands from
  the Part I grid. On 2017, `"TablePartI".contains("Table_PartI")` is **false** for every field → `derive_bands`
  (`verify.rs:83-87`) errors `"no data-grid widgets found"` → **every 2017 Schedule D fill fails closed at read-back**,
  despite a correct map. (2024 is fine — `Table_PartI` matches.)
- The spec's engine-change item 4 and gotchas only ever cite the **8949** token (`F8949_TABLE_TOKEN`,
  `Table_Line1_Part → Table_Line1`); `SCHED_D_TABLE_TOKEN` is a distinct const in a distinct module and appears nowhere
  in the spec. Round-1 I5 was likewise 8949-only. This is a genuinely un-owned sibling of I5, same failure class.

**Fix:** make the Schedule D Part I token per-year map config (`2017 = "TablePartI"`, `2024/2025 = "Table_PartI"`) exactly
as item 4 does for the 8949, and thread it into `verify_schedule_d`. KAT: `ty2017_schedule_d_fills_and_reads_back`
(would fail closed pre-fix) + `map_2017_schedule_d_grid_token`.

### I3 — The 2017 Form 1040 needs `Form1040Map`'s DA fields made optional AND the `topmost_yes_no_pair` guard skipped when the map has no DA field — a schema+engine change parallel to the enumerated per-year QOF-optional item, but omitted. Without it the 2017 1040 fill errors at `form1040.rs:118`.

**Evidence:**
- `Form1040Map` (`map.rs:98-109`) has **non-optional** `da_yes: CheckChoice, da_no: CheckChoice`. A 2017 map — which has
  no DA question (verified: **no {1,2} pair on f1040-2017 page 0**) — cannot be expressed against the current schema.
- `fill_form_1040_capgains` **unconditionally** calls `topmost_yes_no_pair(&check, &fields, 0)` (`form1040.rs:118`) and
  errors if it doesn't match the map's `da_yes/da_no`. On the 2017 form `topmost_yes_no_pair` finds **no** {1,2} pair and
  returns `Err("no same-y {/1,/2} /Btn pair found")` (`verify.rs:444-448`) → the 2017 1040 fill errors even on a correct
  no-DA map.
- The spec captures the 2017 1040 *behavior* in prose (line 63: "the map has no DA field"; "income-only 2017 ⇒ skip"),
  and the produce/skip semantics (round-1 I4) — but the **enumerated engine changes** (which the fold presents as the
  authoritative, complete list correcting the DRAFT) do not include this schema/guard change. It is the direct analogue of
  the enumerated per-year QOF-optional item (item 3): both make a required checkbox pair `Option` and write/guard it only
  when present. The produce/skip rule also needs the 2017 branch to skip on `!schedule_d_active` (line-13 value absent),
  not on `!da_yes` — `da_yes` no longer doubles as the form's existence condition for 2017.

**Fix:** enumerate the 6th engine change: `Form1040Map.da_yes/da_no → Option<CheckChoice>`; `fill_form_1040_capgains`
runs the `topmost_yes_no_pair` guard **only when the map declares a DA field**, and the 2017 produce/skip gate keys off
"line 13 will receive a value" (Schedule D active ∧ line 16 ≥ 0), not `da_yes`. KATs:
`ty2017_1040_no_da_field_skips_guard`, `ty2017_1040_skipped_when_schedule_d_inactive`, `ty2025_da_guard_still_runs`.

---

## Minor

### M1 — The TY2017 tax-table KAT should be a FULL-SCHEDULE equality lock, not a "pin known figures" spot-check.
The spec (line 70) says "pin the brackets/LTCG breakpoints + the $127,200 wage base to known 2017 values." The existing
tables use a full-schedule equality lock (`ty2024_full_schedule_equality_all_28_edges_and_ltcg`, `tax_tables.rs:1119-1223`)
**specifically to close the delta-cancellation hole** its own comment documents (marginal-delta KATs can cancel a
lower-edge transposition). The 2017 KAT should assert all 4 statuses × 7 edges + all 4 §1(h) pairs verbatim from Rev.
Proc. 2016-55. Note the 2017 §1(h) breakpoints are the ordinary-bracket-derived amounts (`max_zero` = top of the 15%
ordinary bracket, e.g. Single $37,950; `max_fifteen` = the 39.6% threshold, e.g. Single $418,400) — **pin the Rev. Proc.
dollar values, do not re-derive them from the schedule.**

### M2 — `MoneyPair` render format is under-specified; "`fmt_money` splits" is not literally true.
`fmt_money` (`lib.rs:61-63`) = `d.to_string()` — the raw `Decimal` Display, **native scale, no commas, no forced 2 dp**.
So `fmt_money(dec!(92350))` = `"92350"` (no `.00` to split) and `fmt_money(dec!(1234.5))` = `"1234.5"`. MoneyPair needs a
**distinct** formatter that normalizes to 2 dp then splits: whole dollars + **zero-padded 2-digit** cents (round-1 I1
said "comma-grouped dollars, zero-padded 2-digit cents" — the fold dropped both details). Decide explicitly whether the
dollars part is **comma-grouped** to match the form's own preprint (`Line7Dollars='127,200'`, `Line14Dollars='5,200'`) —
the rest of the packet uses no commas, so mixing btctax's no-comma amounts next to the comma preprint is a cosmetic
inconsistency worth a deliberate call. KAT should assert the exact split strings (e.g. `92350`/`00`, not `92350.0`/`0`).

### M3 — Name the geometry-critical members of the per-year config (round-1 I1(b) named them; the fold compressed them out) and the `verify_8949` signature change.
The T0 config list (line 93-94) enumerates box/rows/table-token/QOF/pre-filled/logical-sequence but omits the per-year
**x-clusters**, **page-indexes**, **PDF-bytes selection**, and **SE-chain page** that round-1 I1(b) measured as differing:
2017 SE clusters mid[355,452]/amount[477,576] vs 2025's `SE_CLUSTERS=[(410,482),(504,576)]` (`schedule_se.rs:29`); the
2017 SE §B long form is on **page index 1** while `schedule_se.rs:86` pins page 0; f1040-2017 line-13 dollars column
[482,554] vs `F1040_CLUSTERS=[(504,576)]` (`form1040.rs:27`); every `fill_*` hardcodes `*Map::ty2025()` + a `_2025` PDF
const + `SUPPORTED_YEAR=2025` (`lib.rs:41`). These are "per-year config driving `fill_*`" in spirit, but left off the
enumeration they read as 2025 hardcodes an implementer must rediscover. Also: `verify_8949` (`verify.rs:161`) reads
`F8949_TABLE_TOKEN` directly, so making the token per-year changes `verify_8949`'s signature.

### M4 — The facts table mis-attributes the `f1-_51[0]` IRS glitch to the wrong year/line.
Spec line 55 says 2024 line 7 = `f1-_51[0]` "IRS-glitched name." **Measured:** the hyphen-glitch `f1-_51[0]` is on
**f1040-2017**, and it is the **line-13 DOLLARS** field ([482,336,554,348], twinned with cents `f1_52[0]`
[554,336,576,348]) — matching round-1 M5 exactly. **f1040-2024 has NO hyphen-glitch field at all**; its line-7 amount is
`Line4a-11_ReadOrder[0].f1_52[0]` ([504,162,576,174]). Round-1 M5 flagged `f1-_51[0]` as a **booby-trap to pin verbatim**
(so nobody "fixes" it to `f1_51`); mis-filing it under 2024 defeats that warning and would send a 2024 extractor hunting
for a field that isn't there. Move it to the 2017 row (line-13 dollars; note it is the MoneyPair dollars field).

### M5 — The adjacency predicate should keep a topmost-among-adjacent tiebreak.
No in-scope year has two *adjacent* {1,2} pairs (verified 2017/2024/2025), so the `dx ≤ ~60` selection is unique today.
Defensively, spec the predicate as "the **top-most** {1,2} pair with `dx ≤ ~60`" so a future form with two adjacent pairs
degrades gracefully rather than picking arbitrarily.

---

## Nit

### N1 — Clarify (in the tax-table task) that the TY2017 table adds NO std-deduction / NIIT-threshold field.
The `TaxTable` struct carries neither (NIIT threshold is statutory year-independent, `tables.rs:190`; std deduction is
not a table field — the crate takes post-deduction income). The spec is already correct to list only ordinary + LTCG +
`ss_wage_base` + SE rates; a one-line note prevents a reviewer/implementer from "completing" the 2017 table with fields
2024/2025/2026 also don't have.

### N2 — Carry the measured 8283-2017 numbers now that they're verified.
Rev. 12-2014: 154 AcroForm leaves, 26 `/MaxLen==3` cent fields, Section A = Line1A–1E (5 rows), "digital asset" absent,
"j Other" present. Replace line 47's "dollars+cents pairs likely (verify at extraction); 5 Section-A rows" hedge with
these measured facts (the "verify at extraction" phrasing is the pattern round-1 C3 rejected).

---

## Answers to the charter's direct questions

1. **C1 — TY2017 table addable?** Yes, cleanly. The `TaxTable` shape holds the pre-TCJA 7-rate schedule and the
   ordinary-edge-derived §1(h) breakpoints with no schema change; `load()` is a pure additive `insert(2017, …)` and
   nothing hardcodes a supported-year set. The struct needs no std-deduction/NIIT field (those aren't table fields).
   "Primary-source KAT" is the right bar — make it a full-schedule equality lock (M1). Only `ss_wage_base` is load-bearing
   for the *packet* (SE leg); the brackets/LTCG un-gate `report 2017` (bonus, still tax-critical). **Sound.**
2. **C2 — adjacency sound + preserves 2025 + holes?** Sound and measured on all three years: 2024 DA dx=36 vs
   filing-status dx=266.4 (the trap); 2025 DA dx=36 stays selected (no regression); 2017 has no {1,2} pair. `verify.rs:406`
   `topmost_yes_no_pair` exists as cited. No form in scope has two adjacent {1,2} pairs (unique selection); keep a
   topmost-among-adjacent tiebreak defensively (M5). **Sound.**
3. **Engine changes coherent + fit the schema without breaking 2025?** MoneyPair *mechanism* is sound (dollars carries
   column+descent; cents joins `allowed`) — but its *scope* omits the 2017 8283 (**I1**, incl. the MoneyPair × overflow
   interaction on `merge_copies`). Per-year QOF-optional, 8949 table token, pre-filled-exempt: sound and 2025-safe.
   `MoneyPair` + `fmt_money`-split + geometric-oracle-on-the-pair: sound, but "`fmt_money` splits" is not literal — needs
   a distinct 2-dp/zero-pad(/comma?) formatter (M2).
4. **C3 — 8283 revisions coherent + filing-year→revision map right?** Yes: Rev. 12-2014 (2017: j Other, no DA box, 5
   Section-A rows, parts II/III/IV) and Rev. 12-2023 (2024: k Digital assets) confirmed; the 2017 revision also carries
   ¢-pairs (folds into I1).
5. **Importants/Minors captured?** 2017 SE §B long-form mapping — captured. 2017 no-DA produce/skip — captured in prose,
   but missing its engine mechanism (**I3**). Box C/F `/3`, 14 rows — captured. The `f1-_51[0]` glitch is captured but
   **mis-attributed to 2024** (**M4**; it is 2017 line-13 dollars).
6. **Self-consistency / NEW gaps / scope.** Residual internal contradiction: line 33 ("2017-only … single-field") vs line
   47 ("8283 ¢-pairs likely") — resolved by I1. NEW gaps found, exactly in the charter's predicted spots: **MoneyPair ×
   merge_copies on the 8283** (I1) and a second **per-year token** on Schedule D (I2), plus the **Form1040 DA-optional**
   omission (I3). The pre-filled-exempt × `no_unmapped_filled` interaction is sound (only the 4 SE text constants need
   exempting; the 2017 1040/schedD pre-`/V` are all `/Off` checkboxes, not "filled"). **Split recommendation:** keep the
   T0(engine+table) → T1(2024) → T2(2017) structure — but this round confirms 2017 carries *even more* engine weight than
   round 1 knew (MoneyPair across **three** forms + the overflow interaction, **two** per-year grid tokens, DA-optional,
   per-year clusters/pages/PDF, and the TY2017 table), while 2024 is genuinely near-2025 (single-field, `Table_PartI`, DA
   + QOF present). Given that concentration, **promote T2 (2017) to a pre-emptive SP3b** rather than relying on the
   conditional "STOP if T2 balloons" escape (plan line 100) — 2017 is where essentially all the engine-touching risk
   lives.
7. **Oracle generalization.** After I1/I2/I3 fold, the oracle re-derives per year: 8949 grid (14×8, token per-year),
   Schedule D column bands (token per-year — new), SE/1040/8283 flat oracle (clusters/pages per-year, MoneyPair on 2017),
   DA-pair by adjacency (2024/2025), DA guard skipped where absent (2017). The 2025 suite + goldens guard regressions.

**Round-2 disposition:** BLOCKED at 0C/**3I**. All three Importants are the same class round 1 already established
(per-year engine deltas the "data-only" framing hid) — surfaced here against the real PDFs + current source, each with a
concrete evidence-backed fix. Fold I1/I2/I3 (and the Minors), then re-review. Nothing here threatens the sub-project;
2024 is close, 2017 is heavier than the fold admits.
