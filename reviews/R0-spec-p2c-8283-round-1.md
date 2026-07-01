# R0 architect review — SPEC_p2c_form8283_709 — round 1

**Artifact:** `design/SPEC_p2c_form8283_709.md`
**Baseline verified against:** HEAD `f7159c5` (confirmed via `git rev-parse HEAD`).
**Reviewer role:** independent R0 architect gate (author ≠ reviewer). Tax-filing artifact —
Section A/B split, annual-exclusion value, date_acquired↔term, and fabricated-vs-flagged
user-input fields are held to the Critical/Important bar.

## Verdict

**0 Critical / 1 Important.** Does **NOT** pass the gate. One blocking finding (**I1** —
§170(f)(11)(F) aggregation disclosure on the Section A/B split). Fix + re-review before
implementation. All other findings are Minor/Nit (non-blocking, fold at author's discretion).

The core design is sound: the Section split, the acquired_at↔term construction, the
indexed-table placement of the annual exclusion, the deferral of full Form 709, and the
honest-placeholder pattern for user-input fields are all correct and well-grounded. The single
blocking item is a missing *disclosure* (not a missing feature — do NOT implement aggregation now).

---

## Independent verification (web, authoritative)

- **Form 8283 Section A/B threshold — CONFIRMED.** IRS *Instructions for Form 8283 (Rev. Dec.
  2025)*: Section A = noncash contribution **> $500 but not more than $5,000**; Section B = **over
  $5,000**, which **requires a qualified appraisal** (property other than money or publicly-traded
  securities). The spec's `claimed_deduction > $5,000 → Section B, else A` (strict `>`) places
  exactly $5,000 in Section A — matches "not more than $5,000 = A". ✔
- **Crypto has no readily-valued exception — CONFIRMED.** CCA 202302012 (13 Jan 2023): crypto is
  not cash, a publicly-traded security, or other readily-valued property; a qualified appraisal is
  required for a claimed deduction **> $5,000**, and the reasonable-cause exception does **not**
  substitute. ✔
- **Driving off the *claimed deduction* is correct — CONFIRMED.** §170(f)(11)(C) keys the appraisal
  test on the amount "for which a deduction of more than $5,000 is claimed" — i.e. the deduction,
  not raw FMV. The engine's `Removal.claimed_deduction` is the right driver, and it correctly ties
  to the identical `QualifiedAppraisalNote` trigger (`fold.rs:1106`, same constant, same `>`). ✔
- **TY2025 §2503(b) annual exclusion = $19,000 — CONFIRMED.** IRS *Instructions for Form 709
  (2025)*: "The annual gift exclusion for 2025 is $19,000." ✔
- **TY2024 = $18,000; inflation-indexed under §2503(b)(2) — CONFIRMED.** Rev. Proc. 2023-34 §3.43
  set the 2024 exclusion at $18,000 (up from $17,000) as the §2503(b)(2) cost-of-living adjustment.
  Correctly an *indexed* value → belongs in the year-keyed `TaxTable`, NOT a `tables.rs` statutory
  constant (matches the codebase I4 "statutory-vs-indexed" doctrine, `tables.rs:1-9`). ✔

---

## Recon-citation drift check (spec claims vs current source @ f7159c5)

| Spec claim | Source | Verdict |
|---|---|---|
| `RemovalLeg{sat,basis,fmv_at_transfer,term,basis_source}`, no `acquired_at` | `state.rs:149-156` | ✔ exact |
| `Removal.claimed_deduction: Option<Usd>` | `state.rs:168` | ✔ |
| `make_removal_legs` uses `gain_hp_start` for `term`, doesn't store it | `fold.rs:246-253` (`term_for(c.gain_hp_start, removed)`) | ✔ |
| `make_removal_legs` has NO dual-basis loss-zone branch (unlike `make_disposal_legs`) | `fold.rs:219-256` vs `120-214` | ✔ **confirmed single HP-start** |
| `QUALIFIED_APPRAISAL_THRESHOLD = $5,000`, statutory constant | `tables.rs:115-119` | ✔ |
| Section-split threshold == `QualifiedAppraisalNote` trigger | `fold.rs:1106` | ✔ identical |
| `TaxTable` is the year-indexed struct; BundledTaxTables TY2025-only | `tables.rs:52-62`, `tax_tables.rs:48-54` | ✔ (no TY2024 table → advisory skip path is real) |
| `write_csv_exports` year-scoped block writes form8949/schedule_d | `render.rs:685-688` | ✔ |
| removals.csv first-leg-only claimed_deduction convention | `render.rs:636-643` | ✔ (P2-C mirrors it) |
| No donee modeled anywhere | `event.rs:105-108` (`Donate{appraisal_required:bool}`, `GiftOut` fieldless) | ✔ **confirmed** |
| RemovalLeg additive/migration-free (no serde) | `state.rs:148` (`derive(Debug,Clone,PartialEq,Eq)` only) | ✔ |
| RemovalLeg literals to update | exactly ONE (`fold.rs:246`) | ✔ minimal blast radius |

No drift. Recon is accurate.

---

## Findings

### Important

**I1 — Section A/B split does not disclose §170(f)(11)(F) similar-item aggregation (filing risk).**
The Section split is computed **per donation** off that donation's `claimed_deduction`. But the
$5,000 appraisal threshold **aggregates similar items of property donated during the year**
(§170(f)(11)(F); CCA 202302012 confirms it applies to crypto). Two individually-≤$5,000 crypto
donations (e.g. $3,000 + $3,000) aggregate to $6,000 → **Section B + a qualified appraisal is
actually required**, yet the spec stamps each row **Section A**. Worse: the existing
`QualifiedAppraisalNote` (`fold.rs:1106`) fires only when a *single* donation > $5,000, so in the
exact aggregation-relevant case (multiple sub-threshold donations) **no warning fires anywhere.**
Under-classifying to Section A → skipping the mandatory appraisal → CCA 202302012 warns the entire
deduction can be disallowed with **no reasonable-cause relief**. On an advisory note (P2-A) this was
tolerable; P2-C *materializes* the Section choice on a filing artifact, which raises the stakes.

`needs_review=true` on every row is about the blank donee/appraiser/FMV-method fields, not about
this aggregation risk, so it does not adequately cover it.

**Fix (disclosure + FOLLOWUP — do NOT implement aggregation now):**
1. Add a standing caveat surfaced on/near the Form 8283 output (and/or as an explicit needs_review
   reason string): "Section A/B is determined per-donation; the §170(f)(11)(C) $5,000 threshold
   AGGREGATES similar items donated across the tax year (§170(f)(11)(F)) — multiple crypto
   donations each ≤ $5,000 that total > $5,000 may require Section B + a qualified appraisal even
   though each row shows Section A. Verify aggregate similar-item totals (CCA 202302012)."
2. Add a FOLLOWUP: year-aggregation of the similar-items Section/appraisal threshold.

### Minor

**M1 — $500 form-requirement floor not applied.** Form 8283 is required only when total noncash
contributions > $500 (§170(f)(11); Form 8283 instructions). The spec emits a row for *every*
Donation regardless of amount. Over-inclusive, not incorrect (each row is needs_review), but the
legal-grounding section cites the > $500 requirement while the design never uses it. Either gate at
$500 or explicitly disposition it as out-of-scope (aggregation-adjacent; see I1).

**M2 — gift-received-then-donated `date_acquired` is the tacked donor date; document it.** For a lot
received as a gift and later donated, `gain_hp_start = donor_acquired_at` (`state.rs:106-108`), so
`acquired_at` = the *original* donor's acquisition date (tacked, §1223(2)). This is the correct
holding-period start and can never contradict `term` (both derive from the identical date — the
Critical bar is safe), but Form 8283's "Date acquired by donor" nominally wants the calendar date
the *filer* received the gift. It is flagged (how_acquired=gift + needs_review), which is adequate,
**provided** the spec explicitly documents in the `Form8283Row`/`date_acquired` doc-comment that
for gift-origin lots this is the tacked HP-start, not the receipt date — so a reviewer understands
why and does not "fix" it into a contradiction.

**M3 — Task 3 Files list omits the advisory call/print sites.** `render_gift_advisory` lives in
render.rs (pure), but it must be *invoked* where `state` + `tables` + `year` are in scope — that is
`crates/btctax-cli/src/cmd/tax.rs::report_tax_year` (`tax.rs:45-50`; add the advisory to its return
tuple) and printed in `crates/btctax-cli/src/main.rs:381-387` (alongside `render_schedule_d`).
Neither file is listed in Task 3. The prose ("wire into the tax report") covers intent; add the two
files so the plan's diff enumeration is complete and the advisory can't ship unwired.

**M4 — new required `TaxTable` field forces two edits the spec doesn't name.** Adding
`gift_annual_exclusion: Usd` (required — good call; forces every future bundled table to supply the
verified value) means the `#[cfg(test)] synthetic_table` literal (`tables.rs:187-193`) must set it
or the core crate won't compile. Also update `ty2025().source` to cite the gift-exclusion Rev. Proc.
section — the $19,000 comes from **Rev. Proc. 2024-40 §2.42**, not §2.01/§2.03 (`tax_tables.rs:179`).

**M5 — silent None when a year has gifts but no bundled table.** Consistent with
`TaxOutcome::NotComputable(TaxTableMissing)` on the tax side, so acceptable, but a one-line "gift
advisory unavailable: no bundled table for {year}" would be more transparent than a silent skip.
Optional.

### Nit

**N1 — `needs_review` is effectively unconditional; say so.** Because donee/appraiser/FMV-method are
always blank, `needs_review=true` on *every* row; the "Section B → always true" clause is a subset.
State this explicitly so an implementer doesn't make `needs_review` conditional and accidentally
drop the Section-B guarantee.

**N2 — how_acquired="income" isn't a literal Form 8283 category.** The form's "How acquired by
donor" options are Purchase/Gift/Inheritance/Exchange/Other; income-origin crypto (mining/staking)
maps to "Other". Descriptive label is fine; consider noting the mapping.

**N3 — 709 advisory omits future-interest + non-citizen-spouse cases.** Gifts of *future*
interests require Form 709 regardless of amount (no annual exclusion), and the non-citizen-spouse
exclusion is $190,000 (TY2025). Neither is modeled; the total-exposure advisory correctly doesn't
claim them, but a FOLLOWUP note would make the scope boundary explicit.

---

## Dimension-by-dimension assessment

1. **Section A/B split (correctness).** Mechanically correct: strict `>` on `claimed_deduction`
   against $5,000, tied to the identical `QualifiedAppraisalNote` trigger; driving off the
   engine-computed deduction (not the user `appraisal_required` bool) is the statutorily-correct
   choice (§170(f)(11)(C)). **Blocking gap = the aggregation disclosure (I1);** $500 floor = M1.
2. **`RemovalLeg.acquired_at` ↔ term (D1).** Verified: `make_removal_legs` has a **single**
   HP-start (`term_for(c.gain_hp_start, removed)`, `fold.rs:251`) — **no** loss-zone branch, because
   donations recognize no gain/loss (TP10). Setting `acquired_at = gain_hp_start` (the exact
   `term_for` argument) makes a date-vs-term contradiction structurally impossible. Correct.
   Tacked-donor-date subtlety adequately handled; strengthen the doc (M2).
3. **§2503(b) annual exclusion.** $19,000 TY2025 / $18,000 TY2024 confirmed; inflation-indexed →
   `TaxTable` (not `tables.rs`) — correct per I4. Required field is a good call. Skip-on-no-table
   (Option, no panic) is sound. Fix the source cite + synthetic_table literal (M4).
4. **Form 709 = thin advisory.** No donee modeled (verified `event.rs:105-108`) → full 709 rightly
   deferred. Advisory text is honest: conditional ("if any single donee..."), states only the
   computable total + per-donee exclusion, and explicitly disclaims a per-donee determination. No
   false-negative on the annual-exclusion test (total ≤ excl ⇒ each donee ≤ excl). Correct.
5. **Honest user-input gaps.** donee/appraiser/FMV-method emitted blank + needs_review, never
   fabricated; mirrors the established `Form8949Box` C/F honest-placeholder precedent
   (`forms.rs:26-37`). Deferral disclosed with FOLLOWUPS. Correct (N1 clarity).
6. **No engine-B / math change.** `acquired_at` additive, no serde, one literal (`fold.rs:246`);
   `gift_annual_exclusion` read only by the new advisory, never by `compute_tax_year`; first-leg
   deduction mirrors removals.csv (`render.rs:636-643`, no SUM double-count); form8283.csv 0o600 +
   year-scoped + deterministic (removed_at, event, lot_id). Correct. (M4 compile-forcing edits.)
7. **Scope/right-sizing + TDD.** 4 tasks, complete + testable. how_acquired mapping covers all **8**
   `BasisSource` variants soundly, with origin-lost sources (CarriedFromTransfer / SafeHarborAllocated
   / ReconstructedPerWallet) conservatively → "review" and a FOLLOWUP for origin recovery. KATs are
   genuine (Section split, first-leg-only, gift→no-8283-row, year-filter, no-table→skip, $19,000
   asserted vs bundled). Gaps: M3 (wiring files), plus the I1 disclosure belongs in Task 2/Task 4.

---

## Required before green (round 2)

- **I1** folded (disclosure + FOLLOWUP), re-reviewed.
- Recommended to also fold M1–M4 (cheap, plan-completeness) in the same pass.
- Persist this review verbatim (done) before folding; re-review after the fold (including the last).

---

# Round 2 — re-review

**Artifact re-read:** `design/SPEC_p2c_form8283_709.md` (revised).
**Reviewer role:** independent R0 architect gate, round 2. Scope per the round-2 charter: confirm
each fold landed and is adequate; do NOT re-litigate the round-1-validated core (Section split,
$5k A/B threshold, $19,000 TY2025 exclusion, acquired_at↔term structural safety, full-709 deferral,
honest-placeholder pattern). Recon @ f7159c5 was clean in round 1 and the folds touch no source
citations, so no drift re-scan.

## Verdict

**I1 CLOSED. 0 new Critical / 0 new Important. Spec is R0 GREEN — ready to implement**, with one
**Minor** residual (m6) recommended (not required) for reconciliation in the implement pass.

## Fold-by-fold confirmation

- **I1 — CLOSED (was the sole blocker).** The standing Section-A/B aggregation caveat is now
  specified on the form8283 output as a CSV header comment (+ the text/advisory path), D2 lines
  68–77, with the actionable content: per-donation split; the $5,000 threshold aggregates similar
  crypto items across the year; rows shown Section A may require Section B + appraisal; verify
  AGGREGATE similar-item totals. Surrounding prose cites §170(f)(11)(F) + CCA 202302012. A Task-2
  KAT asserts the caveat is present (line 119). The year-aggregation itself is correctly DEFERRED to
  FOLLOWUPS/Task 4 (lines 131–132), not implemented. **Adequate to prevent the silent
  under-classification:** the caveat travels in the CSV header of the very artifact the filer works
  from, and is test-locked. The disclosure-only right-sizing is the correct call. ✔
- **M1 — folded.** $500 form-filing floor note when the year's donation total ≤ $500: D2 lines
  78–80 + Task-2 KAT line 119. ✔
- **M2 — folded.** Gift-received `date_acquired` = tacked donor date (§1223), correct because it
  matches the leg `term`, documented in the field/CSV doc: D1 lines 49–51; Task-1 KAT asserts the
  gift-lot acquired_at↔term consistency (line 115). ✔
- **M3 — folded.** Task 3 Files now name `cmd/tax.rs::report_tax_year` + `main.rs` (~381–387) wiring
  (line 122). Advisory can no longer ship unwired. ✔
- **M4 — folded.** `TaxTable.gift_annual_exclusion` also updates `synthetic_table` ~187 (+ any other
  literal); source cite corrected to **Rev. Proc. 2024-40 §2.42** for $19,000 — D3 lines 85–86 +
  Task 3 line 122; KAT asserts $19,000 vs BundledTaxTables (line 123). ✔
- **M5 — folded, but internally inconsistent (see m6).** The "emit a note rather than nothing" prose
  is present (D3 lines 88–90), but the `render_gift_advisory` OMIT clause and the Task-3 KAT still
  encode the opposite (silent skip). Flagged below. ⚠
- **N2 — folded.** how_acquired FmvAtIncome→"Other" (not "income"); ambiguous→"Review": D2 lines
  65–67 + Task-2 KAT line 119. **N3 — folded.** future-interest / non-citizen-spouse → FOLLOWUPS
  (line 136). N1's concern (needs_review effectively unconditional + Section-B-always guarantee) is
  captured at D2 lines 63–64. ✔

## New findings from the folds

### Minor

**m6 — the M5 fold is self-contradictory across prose / spec / KAT.** Three statements address the
"year has gifts but no bundled table" case and disagree:
1. D3 lines 88–90 (M5 prose): emit a "gift annual-exclusion table unavailable…" note **rather than
   nothing**.
2. D3 lines 96–97 (`render_gift_advisory` contract): "OMIT the advisory (return None) if … the year
   has no bundled table."
3. Task 3 KAT (line 123): "a year with no bundled table → advisory **skipped** (no panic)."

Two of the three (the function contract and the test) lock in the silent skip — i.e. an implementer
doing TDD would satisfy the KAT with the exact behavior M5 was added to eliminate, and the prose
addition becomes dead. **Severity = Minor, non-blocking:** round 1 explicitly rated the silent-skip
behavior Minor/Optional/acceptable (mirrors `TaxOutcome::NotComputable(TaxTableMissing)`), and in the
shipped scope the only bundled year (TY2025) HAS a table, so the no-table branch is edge-of-edge in
practice. This bounds the worst case at a round-1-tolerated Minor. **Fix (implement-pass, either
direction):** (a) fully fold — change the OMIT clause to "no bundled table **AND no gifts**" and flip
the Task-3 KAT to assert the note IS emitted when gifts exist + no table; or (b) re-disposition M5
to the round-1-acceptable silent skip and drop the note prose. Do not ship the current three-way
split.

### Nit

**n4 — "text/advisory path" for the 8283 caveat/M1-note is asserted but not enumerated in the Plan.**
D2 says the caveat lands in "CSV header comment + the text/advisory path," and M1's note "appears"
somewhere, but Task 2 names only `write_form8283_csv` (no 8283 text renderer/wiring, unlike the M3
treatment for the gift advisory). This does not reopen I1 — the CSV header comment alone closes it
(the caveat is on the filed artifact and is KAT-locked). Treat as wording/plan-completeness polish:
either state that the CSV writer is the sole 8283 surface (caveat + notes as header comments) or add
the text-path wiring line the way M3 did for the advisory. Non-blocking.

**n5 — current-state prose still says "FmvAtIncome→income" (line 33).** This is the descriptive
basis_source *semantics* (income-origin), not the output label; D2 correctly maps it to "Other" per
N2. Harmless, but reusing the same arrow notation as the D2 output mapping invites a misread.
Optional one-word clarification. Non-blocking.

## Assessment against the round-2 charter (item 8)

- **No new Critical/Important** introduced by the folds. The only new items are m6 (Minor) and
  n4/n5 (Nits).
- **Internally consistent** except the m6 three-way split (bounded Minor).
- **Right-sized / standalone:** no engine-B, capital-gains, FMV, or basis math change; all folds are
  disclosure / notes / doc / wiring / a read-only indexed-table field. SemVer MINOR + "no tax-figure
  change" holds.
- **TDD-complete:** I1 caveat, M1 ≤$500 note, N2 mapping, M4 $19,000, gift-lot acquired_at↔term, and
  the advisory-emit/omit cases are all KAT-covered. The one test defect is the M5/no-table KAT
  encoding the wrong branch (m6).
- **Disclosure-only approach to aggregation is correct** — implementing §170(f)(11)(F) year
  aggregation now would be over-build; deferring it to Task 4 with a standing caveat is the right
  right-sizing.

## Required before green (round 2)

- **None.** I1 is closed and there are 0 Critical / 0 Important. **GREEN.**
- **Recommended (non-blocking):** reconcile m6 (align the OMIT clause + Task-3 KAT with the M5 note,
  or re-disposition to silent skip) and clear n4/n5 in the implement pass. Per the standard-workflow
  gate, Minors/Nits do not block; proceed to Plan/Implement.
