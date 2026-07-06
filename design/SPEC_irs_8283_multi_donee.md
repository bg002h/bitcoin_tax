# SPEC — Form 8283 multi-donee fix (fill one 8283 per donee)

**Source baseline:** `main` @ `d43d294` (branch `fix/irs-8283-multi-donee`). **Review status: DRAFT — awaiting
R0 (Opus).** Resolves the `irs-8283-multi-donee-identity` FOLLOWUP filed in the SP2 whole-diff.

## The bug (Section B only)
`fill_form_8283` (form8283.rs:97) paginates rows purely by COUNT (`n_copies = rows.len().div_ceil(cap)`), and
the **Section B** Part V donee identity + Part IV appraiser identity are read from the first `details`-bearing
row in each count-chunk (form8283.rs:265, `rows.iter().find_map(|r| r.details.as_ref())`). So a tax year with
donations to **multiple DISTINCT donees** whose rows fall in one count-chunk fills only the FIRST donee's
identity block — a wrong official 8283 (the Part V donee acknowledgment must name the actual donee of that
form's property). Section A is UNAFFECTED (it has a per-row donee COLUMN, form8283.rs:187 — each row names its
own donee; Section A has no Part IV/V identity block).

## Fix — group by donee, then overflow within each group
Form 8283 Section B is **one donee's donation of similar property per form** ("Attach one or more Forms 8283"
sanctions multiple). So:
1. **Partition `rows` into donations** at carrier-row boundaries — a new donation starts at a row with
   `details.is_some()` (the carrier/first leg; `form_8283()` sets `donee`+`details` on the first leg only,
   forms.rs:254). Leg rows attach to their carrier.
2. **Group donations by donee identity** (first-seen order preserved) — key = the carrier's
   `DonationDetails.donee_name` + `donee_ein` (fall back to `row.donee`). Single-donee year ⇒ ONE group ⇒
   byte-identical to today (no behavior change for the common case).
3. **For each donee group:** count-overflow its rows (`div_ceil(cap)`), and pass the group's `details`
   EXPLICITLY into each copy's identity block (not `find_map` within the chunk) so a donee whose legs overflow
   to a 2nd page still carries the identity on BOTH pages.
4. **`merge_copies`** all copies across all donee groups (unchanged).

Section A path unchanged (per-row donee column already correct); apply the grouping to BOTH sections for a
single code path, but it is only observable for Section B.

## Implementation (form8283.rs)
- Refactor `fill_form_8283`: build `Vec<DoneeGroup{ details: Option<&DonationDetails>, rows: Vec<&Form8283Row> }>`
  then, for each group, chunk by `cap` and call `fill_one(chunk, section, map, group.details)`.
- Change `fill_one` to take `details: Option<&DonationDetails>` and use it for the Section B identity block
  (replacing the in-chunk `find_map`). The property-table rows + the "k/j" property box are unchanged.
- `merge_copies` over the flattened per-group copies (unchanged).

## KATs (btctax-forms)
- **★ `form_8283_multi_donee_one_copy_per_donee`** — 2 Section-B donations to donee A and donee B (each ≤ cap
  rows) ⇒ TWO 8283 copies, each with its OWN Part V donee identity (A on copy 1, B on copy 2) — the fix's core.
- **`form_8283_single_donee_unchanged`** — a single-donee (multi-lot) donation ⇒ byte-identical to the
  pre-fix golden (regression guard; the common case must not change).
- **`form_8283_one_donee_overflow_carries_identity_on_both_pages`** — one donee, > cap rows ⇒ 2 copies, the
  donee identity on BOTH.
- **★ geometric read-back fail-closed** still holds per copy (swap two map entries ⇒ RED) — unchanged oracle.
- **regression:** the existing 2024/2025 8283 KATs + the full suite stay green.

## Scope / SemVer
`btctax-forms` (form8283.rs only; no map/PDF/engine change; no core change — `form_8283()` already carries
per-row `donee`+`details`). PATCH-level behavior fix (a new correct output for multi-donee; single-donee
byte-identical). No man-page/README change (the capability was already documented).

## Plan (TDD)
- **T1** — the multi-donee KAT (RED first), then the group-by-donee refactor of `fill_form_8283`/`fill_one`;
  the single-donee regression + overflow-identity KATs; full suite + whole-diff.

## Gotchas
- **[single-donee byte-identical]** the common path must not change — one group ⇒ today's exact output (golden).
- **[overflow identity on every page]** pass the group's `details` explicitly; do not rely on a carrier row
  being present in each chunk.
- **[Section A untouched in effect]** it has a per-row donee column + no identity block; the grouping is a
  no-op there.
- **[fail-closed unchanged]** the per-copy geometric read-back is not modified.
