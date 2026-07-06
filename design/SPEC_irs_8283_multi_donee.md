# SPEC — Form 8283 multi-donee fix (fill one 8283 per donee)

**Source baseline:** `main` @ `d43d294` (branch `fix/irs-8283-multi-donee`). **Review status: R0 round 1 folded
(1C/4I/4M/2N — Opus; merged IN-PLACE). Awaiting R0 round 2.** Review:
`reviews/R0-spec-irs-8283-multi-donee-round-1.md`. Resolves the `irs-8283-multi-donee-identity` FOLLOWUP.
(R0 also surfaced a SECOND latent bug the fix cures: an overflowing single donee gets NO identity on page 2
today — `find_map` over leg rows that all carry `details: None`.)

## The bug (Section B only)
`fill_form_8283` (form8283.rs:97) paginates rows purely by COUNT (`n_copies = rows.len().div_ceil(cap)`), and
the **Section B** Part V donee identity + Part IV appraiser identity are read from the first `details`-bearing
row in each count-chunk (form8283.rs:265, `rows.iter().find_map(|r| r.details.as_ref())`). So a tax year with
donations to **multiple DISTINCT donees** whose rows fall in one count-chunk fills only the FIRST donee's
identity block — a wrong official 8283 (the Part V donee acknowledgment must name the actual donee of that
form's property). Section A is UNAFFECTED (it has a per-row donee COLUMN, form8283.rs:187 — each row names its
own donee; Section A has no Part IV/V identity block).

## Fix — SECTION B ONLY: group by identity, then overflow within each group
Form 8283 Section B is **one donee's donation of similar property per form** ("Attach one or more Forms 8283"
sanctions multiple). **[R0-I2] Apply the grouping to SECTION B ONLY** — Section A keeps today's count-overflow
(it has a per-row donee COLUMN, no Part IV/V identity block, so grouping there would only change PAGINATION —
unblessed). So, for Section B:
1. **[★ R0-C1] Partition `rows` into donations** at carrier boundaries — a new donation starts at a row with
   **`row.section.is_some()`** (set UNCONDITIONALLY on every carrier by `form_8283()`, forms.rs:401; already the
   module's canonical carrier probe, form8283.rs:107). **NOT `details.is_some()`** — `details` is `None` on the
   carrier of any donation with no captured `DonationDetails` (forms.rs:395-431), so `details.is_some()` would
   absorb a no-details second donee onto the PREVIOUS donee's form under a named Part V (worse than today).
   Leg rows (`section: None`) attach to their carrier.
2. **[★ R0-I3] Group donations by the full IDENTITY** (first-seen order preserved), key = the carrier's
   `details` identity block **= donee (`donee_name`+`donee_ein`) AND appraiser (`appraiser_name`+`appraiser_tin`/
   `ptin`)** — because the Part V donee AND the Part IV appraiser are both read from one `details`
   (form8283.rs:265-289). **Split-on-difference** (same donee, different appraiser ⇒ separate forms — a shared
   form would print a wrong Part IV). A carrier with `details: None` keys on `row.donee` (empty ⇒ its own
   singleton group — [R0-M] never merged with another empty-key donation). Single-donee year ⇒ ONE group.
3. **For each group:** count-overflow its rows (`div_ceil(cap)`), passing the group's `details` EXPLICITLY into
   each copy's identity block (not `find_map` within the chunk) so a donee whose legs overflow to a 2nd page
   carries the identity on BOTH pages (also cures the R0-noted page-2-blank bug).
4. **`merge_copies`** all copies across all groups.

**[★ R0-I4] Byte-identity:** the single-physical-copy case (total copies == 1, incl. every single-donee year)
MUST return `fill_one(...)` DIRECTLY — do NOT route it through `merge_copies` (which re-loads/saves and would
break the existing byte-golden, sp2.rs:722). Only build+`merge_copies` when the total copy count across all
groups is ≥ 2.

## Implementation (form8283.rs)
- `fill_one` takes `details: Option<&DonationDetails>` and uses it for the Section B identity block (replacing
  the in-chunk `find_map` at form8283.rs:265). Property-table rows + the "k/j" property box unchanged.
- `fill_form_8283`:
  - **Section A:** unchanged — count-overflow the flat `rows`, `details` passed as the first row's (today's
    behavior); Section A has no identity block so this is a no-op change.
  - **Section B:** partition into donations at `row.section.is_some()` [C1]; group donations by the identity
    key [I3, split-on-difference]; per group, chunk by `cap` → `fill_one(chunk, B, map, group.details)`; collect
    all copies across groups.
  - **Both:** if the TOTAL copy count == 1, return that single `fill_one` result directly; else
    `merge_copies(&all_copies)` [I4 byte-identity].

## KATs (btctax-forms)
- **★ `form_8283_multi_donee_one_copy_per_donee`** — 2 Section-B donations to donee A and donee B (each ≤ cap
  rows) ⇒ TWO 8283 copies, each with its OWN Part V donee identity (A on copy 1, B on copy 2) — the fix's core.
- **★ [R0-I5a] `form_8283_interleaved_same_donee_groups_globally`** — donations ordered A,B,A (`form_8283()`
  sorts by date, forms.rs:436, so same-donee donations are NON-adjacent) ⇒ TWO copies: A's copy has BOTH A
  donations, B's has B's — proves GLOBAL group-by-identity, not an adjacency run.
- **★ [R0-I5b/C1] `form_8283_second_donee_without_details_still_separate`** — donee B's carrier has
  `details: None` (only `row.section` set) ⇒ still TWO copies, B NOT absorbed onto A's named form.
- **[R0-I3] `form_8283_same_donee_different_appraiser_splits`** — same donee, different appraiser ⇒ 2 copies
  (correct Part IV on each).
- **`form_8283_single_donee_unchanged`** — a single-donee (multi-lot) donation ⇒ **byte-identical** to the
  pre-fix golden (the total-1-copy direct path; the common case must not change).
- **`form_8283_one_donee_overflow_carries_identity_on_both_pages`** — one donee, > cap rows ⇒ 2 copies, the
  donee identity on BOTH.
- **[R0-I2] `form_8283_section_a_multi_donee_stays_one_form`** — Section A ≤$5k with 2 donees ⇒ still ONE form
  (per-row donee column; pagination unchanged).
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
- **[★ C1] carrier signal = `row.section.is_some()`**, NOT `details.is_some()` (a Section-B donation can have
  `details: None`).
- **[★ I2] Section B ONLY** — Section A keeps count-overflow (grouping there changes pagination).
- **[★ I3] identity key = donee AND appraiser** (split-on-difference); empty-key (`details: None`, empty donee)
  ⇒ its own singleton group, never merged with another empty-key donation.
- **[★ I4] single-physical-copy returns `fill_one` directly** — only `merge_copies` when total copies ≥ 2
  (byte-golden for the common case).
- **[overflow identity on every page]** pass the group's `details` explicitly; don't rely on a carrier in each
  chunk (also cures the page-2-blank bug R0 found).
- **[fail-closed unchanged]** the per-copy geometric read-back is not modified.
