# R0 — SPEC review: Form 8283 multi-donee fix (round 2 — delta / fold verification)

- **Artifact:** `design/SPEC_irs_8283_multi_donee.md`
- **Baseline:** branch `fix/irs-8283-multi-donee` @ `e99ca34`; main == `d43d294`.
- **Reviewer role:** independent architect, R0 round 2 (author ≠ reviewer). Read-only; no implementation.
- **Bar:** 0 Critical / 0 Important.
- **Scope:** delta check — confirm the round-1 folds (1C/4I/4M/2N) are captured correctly + introduced no
  new contradiction. Sources re-read at this SHA: the spec; `crates/btctax-forms/src/form8283.rs`
  (`fill_form_8283`/`fill_one`, in full); `crates/btctax-core/src/forms.rs` (`form_8283()`, lines 350-438);
  spot-checked `crates/btctax-forms/tests/sp2.rs:722-733` (byte-golden) and
  `crates/btctax-forms/src/overflow.rs:23-73` (`merge_copies`).

## Verdict — **0 Critical / 0 Important / 3 Minor / 1 Nit → R0-GREEN (cleared to implement)**

Every round-1 finding is folded correctly and each fold is corroborated by the current source. No fold
introduced a contradiction; the three output paths (total-1 direct / Section-A count-overflow / Section-B
multi-group) are mutually consistent and each is implementable with no open blocking question. Three Minors
and one Nit remain, all at author discretion — none blocks implementation.

---

## Fold-by-fold verification

### C1 — carrier signal now `row.section.is_some()` — **CONFIRMED CORRECT**
Spec §Fix step 1 (lines 24-28) + gotcha (line 86) now partition donations at `row.section.is_some()`,
explicitly rejecting `details.is_some()`. Verified against source:
- `forms.rs:401` — `section: is_first.then_some(section)` — set **unconditionally** on every carrier
  (gated only on `is_first`, never on `d`/details); leg rows are `is_first == false` ⇒ `section: None`.
- `forms.rs:395-399` + `forms.rs:431` — `details: if is_first { d.cloned() } else { None }` where
  `d = details.get(&r.event)` may be `None` ⇒ a **no-details carrier has `section: Some(_)` but
  `details: None`**. So `section.is_some()` opens a donation for that carrier; `details.is_some()` would
  have swallowed it onto the previous donee (the C1 mis-attribution).
- `form8283.rs:106-109` (`find_map(|r| r.section)`) confirms `section` is already the module's canonical
  carrier probe — the fix is now consistent with existing code.

A `details: None` carrier is therefore correctly partitioned into its own donation, and (step 2, line 34)
keyed on `row.donee` — empty ⇒ singleton, non-empty ⇒ groups by name with blank identity + `needs_review`.
Sound.

### I2 — grouping scoped to Section B only — **CONFIRMED COHERENT, no accidental Section-A change**
Spec §Fix (lines 18-22) + Implementation (lines 49-52) + gotcha (line 88) now scope grouping to Section B;
Section A keeps count-overflow over flat `rows`. Verified: Section A has a per-row donee **column**
(`form8283.rs:187`) and **no** Part IV/V identity block (that block is only in the `Section::B` arm,
`form8283.rs:264-289`), so passing `details` into the Section-A `fill_one` is a genuine no-op (Section A
never reads it). The false round-1 "only observable for Section B" claim is gone; the KAT
`form_8283_section_a_multi_donee_stays_one_form` (line 71) locks the no-split behavior. The Implementation
section reflects Section-A-unchanged with no residual behavior change.

### I3 — key = donee AND appraiser, split-on-difference, empty-key singleton — **CONFIRMED SOUND** (one residual precision Minor, m1)
Spec step 2 (lines 29-34) keys on donee (`donee_name`+`donee_ein`) **and** appraiser
(`appraiser_name`+`appraiser_tin`/`ptin`), split-on-difference, `details: None` ⇒ `row.donee` key ⇒
empty singleton never merged. Verified against the identity block (`form8283.rs:265-289`): it prints
donee_name (282), donee_ein (283-285), donee_address (286-288), appraiser_name (268-270),
appraiser_address (271-273), appraiser_tin-else-ptin (275-281) — all from one `details`. Adding appraiser
to the key closes the Part IV hole (same donee / different appraiser now splits onto correct forms), and
split-on-difference is the safe posture (over-split = extra valid form; over-merge = wrong named party). The
round-1 Important is genuinely retired: **who** is named on each form (donee by name+EIN, appraiser by
name+TIN) is now correct by construction. See m1 for the one residual (non-keyed address / "which details").

### I4 — total==1 returns `fill_one` directly; `merge_copies` only when total ≥ 2 — **CONFIRMED, count is TOTAL across groups**
Spec (lines 40-43) + Implementation (line 54) + gotcha (line 91). Verified:
- Byte-golden `sp2.rs:722-733` (`form_8283_is_byte_deterministic`, `GOLDEN_8283_SHA256`) uses a **single**
  `b_row` — single donee, ≤ cap ⇒ one group, one copy. Under the fold that is total==1 ⇒ direct `fill_one`
  ⇒ byte-identical to today's `n_copies == 1` fast path (`form8283.rs:119-122`). Preserved.
- `merge_copies` unconditionally re-`load`s copy 0 (`overflow.rs:24`) and re-`save`s (`overflow.rs:72`), so
  routing the 1-copy case through it would re-serialize and could shift bytes — the bypass is justified.
- The count is correctly specified as **TOTAL across all groups** (lines 42-43 "across all groups", line 54
  "TOTAL copy count == 1"), **not** per-group and **not** `rows.len().div_ceil(cap)`. This matters: a
  2-donee, 1-row-each Section-B year is `rows.len()==2, cap==3` ⇒ old `div_ceil == 1` but new total == 2
  copies. The spec's "total across groups" is the correct discriminant; no contradiction.

### I5 + minors — KAT set — **CONFIRMED SUFFICIENT** (one nice-to-have Minor, m2)
All 7 cited KATs are present and each locks a distinct mechanism: `..._one_copy_per_donee` (core),
`..._interleaved_same_donee_groups_globally` (I5a — proves global group-by over the `forms.rs:436`
date-sort, not adjacency-run), `..._second_donee_without_details_still_separate` (I5b/C1 — the exact
`section` vs `details` regression), `..._same_donee_different_appraiser_splits` (I3 Part IV), `..._single_
donee_unchanged` (byte-golden / total-1 path), `..._one_donee_overflow_carries_identity_on_both_pages`
(the second latent page-2-blank bug), `..._section_a_multi_donee_stays_one_form` (I2). Plus the per-copy
geometric read-back fail-closed oracle and the 2024/2025 regression goldens. The two round-1 load-bearing
gaps (interleaved + no-details-2nd-donee) are both covered. No required case is left unlocked.

---

## Self-consistency / new-gap sweep

- **Three paths cleanly specified.** (a) total-1 direct, (b) Section-A count-overflow (unchanged), (c)
  Section-B partition→group→per-group chunk→collect. The total-1 check (line 54, "Both:") is applied after
  either section arm produces its copies; there is no path where Section-B grouping and the total-1 bypass
  disagree. No leftover round-1 language contradicts the folds.
- **The second latent bug** (overflowing single donee blank on page 2) is still correctly cured: the
  page-2 chunk is all leg rows (`details: None`), so today's in-chunk `find_map` (`form8283.rs:265`)
  returns `None`; passing the group's `details` explicitly stamps both pages. KAT #6 locks it.
- **No new contradiction** introduced by any fold.

## Minor / Nit (author discretion — non-blocking)

### m1 — "which `details` a group carries" is not explicitly pinned; "identical within a group by construction" is slightly overstated.
The key (line 30) omits the two non-keyed fields the identity block still prints — `donee_address`
(`form8283.rs:286-288`) and `appraiser_address` (271-273). Two donations sharing the key
(donee_name+EIN+appraiser_name+TIN) but differing in address group together, so **which** carrier's
address prints is undefined by the spec text. Impact is genuinely Minor — the **named parties** are correct
regardless (EIN/TIN match), only a cosmetic address on a data-inconsistency input can differ, and the
natural implementation (group's details = the first donation that opened the group, in first-seen order)
is deterministic (NFR4). Recommend one sentence: "group.details = the first-seen carrier's `details`."
This closes the round-1 I3 tail ("pin which details = first-seen carrier") and round-1 n2.

### m2 — multi-group-overflow merge KAT still absent (round-1 m4, nice-to-have).
The existing KATs exercise cross-group merge at total==2 (`..._one_copy_per_donee`, 2 groups × 1 copy) and
single-group overflow at total==2 (`..._overflow...both_pages`, 1 group × 2 copies), but not a group that
**itself** overflows alongside another group (e.g. donee A > cap ⇒ 2 copies, donee B 1 copy ⇒ 3 total).
That case is the one where the per-copy rename index must be the **global** flattened index across groups
(`overflow.rs:43-46`, `btctaxcopy{k}`) to keep FQNs unique — worth one assertion. Nice-to-have.

### m3 — degenerate pre-carrier / carrier-less Section-B input under grouping is unspecified (round-1 m2 residual).
`form_8283()` always emits a carrier per donation, so only hand-constructed inputs hit this. A fully
carrier-less input routes to Section A anyway (`form8283.rs:106-109` `find_map(...).unwrap_or(A)`), so the
only unspecified case is Section-B rows that precede the first carrier. One line ("leading pre-carrier rows
attach to the first group / fall back to count-chunk") would remove the implementer guess. Minor.

### n1 — citation precision.
Spec line 25 / 86 cite `form8283.rs:107` for the canonical carrier probe; the `find_map(|r| r.section)`
is on **line 108** (107 is `.iter()`). The `forms.rs:401` (section), `395-431` (details `None`),
`265-289` (identity block), `436` (sort), `187` (Section-A donee column), and `sp2.rs:722` (byte-golden)
citations all verify exactly. Trivial.

## Bottom line
The 1C/4I fold is complete and faithful to source; no fold created a new contradiction; the plan is
implementable with 0 open blocking questions. **R0-GREEN — cleared to implement.** m1 (one sentence pinning
"which details = first-seen carrier") is the single highest-value optional tightening; m2/m3/n1 at author
discretion.
