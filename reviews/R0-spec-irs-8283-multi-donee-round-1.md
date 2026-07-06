# R0 — SPEC review: Form 8283 multi-donee fix (round 1)

- **Artifact:** `design/SPEC_irs_8283_multi_donee.md` (DRAFT)
- **Baseline:** branch `fix/irs-8283-multi-donee` @ `5f2e2ef`; main == `d43d294`.
- **Reviewer role:** independent architect, R0 (author ≠ reviewer). Read-only; no implementation.
- **Bar:** 0 Critical / 0 Important.
- **Sources read in full:** the spec; `crates/btctax-forms/src/form8283.rs`
  (`fill_form_8283`/`fill_one`); `crates/btctax-core/src/forms.rs` (`form_8283()`, `Form8283Row`);
  `crates/btctax-core/src/donation.rs` (`DonationDetails`); `crates/btctax-forms/src/overflow.rs`
  (`merge_copies`); the existing 8283 KATs (`crates/btctax-forms/tests/sp2.rs`).

## Verdict — **1 Critical / 4 Important / 4 Minor / 2 Nit → NOT GREEN (BLOCKED)**

The **core idea is right** — group rows into donations, group donations by donee, count-overflow within
each group, pass identity explicitly per copy. It correctly resolves the reported Part V donee-loss for the
common case, and it correctly fixes a *second* latent bug (identity missing on page 2 of an overflowing
single donee — see below). But the DRAFT has one Critical (the carrier-boundary signal is wrong for a real
Section-B input class and can **mis-attribute** one donee's property to another's form) and four Important
gaps (a false "Section A unaffected" claim that is actually a behavior change; a donee-only key that drops the
Part IV appraiser identity and is undefined when a group holds differing details; a byte-identity hazard from
routing the 1-copy case through `merge_copies`; and load-bearing missing KATs). Fold before implementation.

---

## 1. Critical

### C1 — Carrier-boundary signal `details.is_some()` is WRONG; it must be `section.is_some()`. A Section-B donation with no captured `DonationDetails` is invisible to the partitioner, and its property gets absorbed onto the PREVIOUS donee's form (a wrong official 8283 — worse than the bug being fixed).

**Spec text (step 1, lines 18-20):** "Partition `rows` into donations at carrier-row boundaries — a new
donation starts at a row with `details.is_some()` (the carrier/first leg …)."

**Why this is wrong — evidence from `form_8283()`:** `details` is populated on the carrier row **only when a
`DonationDetails` entry exists for that event**, and is `None` otherwise:

- `forms.rs:395-399` — `let d = if is_first { details.get(&r.event) } else { None };` → `d` is `None` when the
  event has no stored details.
- `forms.rs:431` — `details: if is_first { d.cloned() } else { None },` → so the **carrier** row of a
  no-details donation has `details: None`, exactly like its own leg rows.

By contrast, the **`section`** field is set on the carrier **unconditionally** (independent of details):

- `forms.rs:401` — `section: is_first.then_some(section),` → `row.section.is_some()` iff the row is the
  carrier/first leg, for *every* donation.

The existing code already relies on this: it detects the year's section via `rows.iter().find_map(|r| r.section)`
(`form8283.rs:106-109`). The spec's own carrier signal is inconsistent with the module's established one.

**Is a no-details Section-B donation a real input?** Yes. Section B is chosen by the **year-aggregate**
> $5,000 (`forms.rs:363-370`), which is independent of whether the user captured `DonationDetails` for any
given donation. A Section-B donation with no details is representable and produces a carrier row with
`needs_review == true` (`forms.rs:426-427`, `d.is_none_or(...)`), a not-filing-ready form the CLI escalates
(module doc `form8283.rs:9-12`). The form is still generated (fill is unconditional on non-empty rows,
`form8283.rs:101-103`).

**Concrete failure (2-donee Section-B year; donee A has details, donee B does not):**

| row | source | `section` | `details` |
|-----|--------|-----------|-----------|
| R0 | D1(A) carrier | `Some(B)` | `Some(A-details)` |
| R1 | D2(B) carrier | `Some(B)` | `None` |

Partition at `details.is_some()`: only R0 opens a donation; R1 (`details.is_none()`) attaches to D1 → **one**
group (donee A) containing **both** donations' property → donee B's BTC is filled on donee A's form under
donee A's Part V acknowledgment. This is the identical wrong-official-form class the fix exists to eliminate,
and it is arguably **worse** than today's bug: today the identity block simply goes blank for a no-details
donation (`find_map` returns `None`, `form8283.rs:265`), whereas the spec's grouping actively **mis-attributes**
one donee's property to a *named* different donee.

Partition at `section.is_some()` instead: R0 and R1 both open donations → grouped by donee → donee A form
(D1) + donee B form (D2, blank identity but correct property). Correct.

**Fix:** change the carrier signal from `details.is_some()` to `row.section.is_some()` in step 1 and in the
implementation sketch (line 33-34). `section` is the only field guaranteed present on every carrier
(`forms.rs:401`) and is already the module's canonical carrier probe (`form8283.rs:107`). Keep passing the
carrier's `details` (which may legitimately be `None` → blank identity, preserving today's behavior +
`needs_review` escalation for that donation).

---

## 2. Important

### I2 — "Section A really unaffected?" — NO. Applying per-donee grouping to Section A changes its pagination: a multi-donee Section-A year that today packs onto ONE form is split into one-form-per-donee. The spec's claim "it is only observable for Section B" (line 30) is FALSE, and the change is unblessed + untested.

Section A is *designed* to carry multiple donees on one physical form — it has a per-row donee **column**
(`form8283.rs:187`, `push_cell(..., &m.donee, row.donee.clone(), 0, ord)`), and the Section-A arm
(`form8283.rs:183-218`) has **no** Part IV/V identity block (that block lives only in the `Section::B` arm,
`form8283.rs:265-289`). So the reviewer sub-question #4 is confirmed *for the identity block*: grouping is a
no-op for Section-A **output content**.

**But grouping is NOT a no-op for Section-A pagination.** Today Section A count-chunks across all rows
(`form8283.rs:117`, `rows.len().div_ceil(cap)`), packing up to `cap` rows — of *different* donees — onto one
copy. The spec (lines 29-30) says "apply the grouping to BOTH sections for a single code path" — that forces
one-copy-per-donee-group even for Section A. A 2-donee, 1-row-each Section-A year that today emits **one**
form (two rows, each naming its donee) would now emit **two** forms. That is a real change to an official
tax form's output for a real input class (multi-donee ≤ $5,000 year), it directly contradicts the reviewer
sub-question #6 expectation "Section A multi-donee stays one form," and no KAT covers it (the only Section-A
KAT, `sp2.rs:651-719`, is single-row/single-donee).

**Fix (pick one, state it in the spec):**
(a) **Preferred — scope grouping to Section B only.** Section A keeps today's pure count-chunk path
(`form8283.rs:117`); Section B gets the donee-group path. This makes Section A **fully** byte-identical (not
just single-donee), removes the appraiser edge (Section A has no appraiser block), and is simpler to reason
about. The "single code path" aspiration is not worth a silent behavior change to a filed form.
(b) If a single code path is kept, make the Section-A group key a constant (all rows → one group) so A's
pagination is preserved, **and** add a KAT asserting a 2-donee Section-A year = one form. Either way, delete
the false "only observable for Section B" claim.

### I3 — Donee-only grouping key drops the Part IV **appraiser** identity, and "pass the group's `details`" is undefined when a donee group contains donations with DIFFERENT details.

The identity block reads donee **and** appraiser from a **single** `details` (`form8283.rs:265-289`:
`donee_name`/`donee_ein`/`donee_address` **and** `appraiser_name`/`appraiser_address`/`appraiser_tin` all come
from the one `details` found). The spec's key is `donee_name` + `donee_ein` only (line 22). So two Section-B
donations to the **same** donee but with **different appraisers** (separate qualified appraisals) land in one
group → one form → only the first appraiser's Part IV identity is shown. That is the same wrong-official-form
class as the donee bug, merely relocated from Part V to Part IV, and the spec produces it silently.

Relatedly, the spec says "pass the group's `details` EXPLICITLY" (line 24-26) and "pass the group's `details`
to each copy" (line 36) without defining **which** `details` when the group has multiple carriers whose
details differ (different appraiser/EIN/address). This selection is undefined.

**Tax posture (reviewer sub-question #3 — split vs merge):** merging is unsafe when identities differ.
Over-*merging* two distinct identities onto one form names the wrong party for the other's property (a wrong
form); over-*splitting* one donee into two forms produces an extra but still-valid 8283 ("Attach one or more
Forms 8283" sanctions it, `form8283.rs:14-15`). So **split-on-difference is the safer posture**.

**Fix:** either (a) include the appraiser identity in the grouping key (group by `(donee identity, appraiser
identity)`), so differing-appraiser donations split onto separate correct forms; or (b) explicitly document a
"one appraiser per donee" assumption **and** escalate/`needs_review` when a donee group holds differing
appraiser details, and pin "the group's details = the first-seen carrier's details." Option (a) matches the
safer split-on-difference posture and closes the Part IV hole.

### I4 — Byte-identity hazard: routing the single-total-copy case through `merge_copies` will (silently) change the single-donee golden. The spec's "merge_copies over the flattened per-group copies (unchanged)" (line 27/37) does not preserve the existing `n_copies == 1` no-merge fast path.

Today, a single copy is returned **without** `merge_copies` (`form8283.rs:119-122`):
```
if n_copies == 1 { let chunk = rows.iter().collect(); return Ok(Some(fill_one(&chunk, section, map)?)); }
```
`merge_copies` re-`load`s copy 0 and re-`save`s it through lopdf (`overflow.rs:24-26,71-72`); the current code
deliberately avoids that second round-trip for the 1-copy case. The single-donee byte-golden
(`GOLDEN_8283_SHA256`, `sp2.rs:722-733`) is exact and is the promised regression guard (spec gotcha, lines
58-59). If the refactor "flattens all per-group copies and always calls `merge_copies`," a single-donee year
(one group, one copy) takes a second load/save round-trip and the golden hash may change — either breaking
the build or, worse, prompting a well-meaning golden-hash "update" that masks a real byte change to a filed
form.

**Fix:** the spec must state that when the **total** flattened copy count is 1, the lone `fill_one` bytes are
returned directly (bypass `merge_copies`), exactly as today. One sentence; but it guards the lead guarantee.

### I5 — KAT set is insufficient to lock the fix. Two load-bearing cases are missing.

The proposed KATs (lines 40-47) cover 2-donee-one-copy-each, single-donee byte-identical, single-donee
overflow-both-pages, and fail-closed. Good, but two cases that the design's own mechanics make failure-prone
are absent:

1. **Interleaved same-donee (proves group-by-key, not adjacency-run).** `form_8283()` sorts rows by
   `(removed_at, event, lot_id)` (`forms.rs:436`) — donations to the *same* donee on different dates are
   **non-adjacent** when a different donee's donation falls between them (e.g. Jan→A, Feb→B, Mar→A yields
   rows A,B,A). The spec says "group donations by donee identity (first-seen order preserved)" (line 21),
   which is a global group-by — but a naive adjacency/run-length implementation would split donee A across two
   forms or fold A's second donation into B. A KAT with an A,B,A interleave (donee A must produce **one** form
   with both A donations; donee B one form) is the highest-value missing test; it is the case the ordering
   makes real and the one most likely to expose a mis-implementation.
2. **No-details distinct second donee (locks C1).** donee A (with details) + donee B (`details: None`, distinct
   donee) → B's property must NOT appear on A's form. This directly guards the Critical above; without it, the
   `details.is_some()` vs `section.is_some()` regression is untested.

**Fix:** add both KATs. (Also-nice, Minor: a 3-donee case and a multi-group overflow-merge case — see m4.)

---

## 3. Minor

### m1 — Empty donee-key collision. When `details` is `None` **and** `Removal.donee` is `None`, the carrier's
`donee` is `""` (`forms.rs:415-420`) and `donee_ein` is `None` → key is empty. Two such distinct donations
would group together (one form, blank identity). These are `needs_review` / not-filing-ready anyway, but the
spec should state that empty-keyed donations are **not** merged (keep each as its own group) — consistent with
the split-on-difference posture in I3.

### m2 — Degenerate all-non-carrier input under grouping. Today `fill_form_8283` tolerates a hand-constructed
all-non-carrier input via `find_map(...).unwrap_or(A)` + count-chunk (`form8283.rs:106-109`). Under
`section.is_some()`-boundary grouping, an input whose leading rows precede the first carrier (or has no carrier
at all) needs defined handling (a leading orphan group, or fall back to today's count-chunk). Only
hand-constructed inputs hit this (`form_8283()` always emits a carrier per donation), but the spec should say
what the grouping does with pre-carrier / carrier-less rows so it isn't left to implementer guess.

### m3 — Key normalization for EIN-present vs EIN-absent on the SAME donee. Key = `(donee_name, donee_ein)`
means the same donee appearing once with `donee_ein: Some(..)` and once with `None` splits into two groups (two
forms for one donee). Harmless-but-suboptimal (valid extra form). Consider normalizing (e.g. key on name, or
merge when EIN is present on either) — a judgment call the spec should make explicit rather than leave to the
tuple's default equality.

### m4 — Additional KATs (nice-to-have, beyond the two required in I5): a **3-donee** year (proves grouping
generalizes past 2, per reviewer sub-question #6) and a **multi-group overflow merge** (donee A > cap rows →
2 copies, donee B 1 row → 1 copy; assert 3 copies, correct per-group identities, and unique FQNs after the
per-copy rename). The rename is per-flattened-copy-index (`overflow.rs:43-46`, `btctaxcopy{k}`), so k must be
the **global** index across groups for uniqueness — worth asserting once.

## 4. Nit

### n1 — Citation precision. Spec line 20 cites `forms.rs:254` for "`form_8283()` sets `donee`+`details` on the
first leg only." Line 253-256 is the *doc-comment*; the actual assignments are at `forms.rs:401` (section),
`415-420` (donee), `421-425` (appraiser), `431` (details). Per CLAUDE.md ("verify citations against current
source"), cite the assignment lines. (This nit is also *why* C1 matters: the doc-comment says "donee+details
on the first leg," but the code sets `details` on the first leg **only when captured** — the comment glosses
the `None` case the Critical turns on.)

### n2 — The spec should pin "the group's details" = the first-seen carrier's `details` in first-seen row
order (deterministic, NFR4), even after I3 is resolved. State it explicitly so the identity source is
unambiguous.

---

## What the fix gets RIGHT (verified, for the re-review record)

- **Core multi-donee resolution** (reviewer #1): once the carrier signal is corrected (C1), grouping donations
  by donee and count-overflowing within each group does resolve the Part V donee-loss, including the
  non-adjacent/interleaved case (global group-by over the `forms.rs:436` order), and keeps the common
  single-donee case one group.
- **Single-donee byte-identical** (reviewer #1, #6): one donee ⇒ one group ⇒ `div_ceil(n/cap)` copies == today
  (`form8283.rs:117`); the existing byte-golden (`sp2.rs:722-733`) is a single-donee single-copy case, so it
  holds — **provided I4 (bypass `merge_copies` at total-1-copy) is honored.**
- **Second latent bug fixed** (reviewer #5): today an overflowing **single** donee gets **no** identity on
  page 2 — the 2nd chunk is `rows.skip(cap)` (`form8283.rs:125`), all leg rows with `details: None`, so
  `find_map` (`form8283.rs:265`) returns `None`. Passing the group's `details` explicitly to each copy
  correctly stamps identity on **both** pages. The existing overflow KAT only asserts page count
  (`sp2.rs:608-619`) and there is **no** overflow byte-golden, so this correct change breaks nothing — the
  proposed `form_8283_one_donee_overflow_carries_identity_on_both_pages` KAT is the right lock.
- **Section A identity block** (reviewer #4): genuinely absent from the Section-A arm; grouping does not add
  any identity content to Section A. (The pagination change is the separate I2 problem.)
- **`merge_copies` over N copies from M groups** (reviewer #5): mechanically fine — it renames only the root
  `/T` of copies 1.. (`overflow.rs:43-46`), copy 0 stays base; unique as long as the flattened index is global
  (m4). The per-copy geometric read-back (`verify_flat`, `form8283.rs:303-306`) is untouched by adding a
  `details` parameter to `fill_one`.

## Required to reach GREEN (round 2)
1. **C1** — carrier boundary = `row.section.is_some()`, not `details.is_some()`.
2. **I2** — resolve Section A: scope grouping to Section B (preferred) or bless+KAT the Section-A split; delete
   the false "only observable for Section B" claim.
3. **I3** — key includes appraiser identity (split-on-difference) OR document single-appraiser assumption +
   escalate on divergence; pin "which details" = first-seen carrier.
4. **I4** — spec states total-1-copy bypasses `merge_copies` (byte-golden guard).
5. **I5** — add the interleaved-same-donee KAT and the no-details-distinct-second-donee KAT.
6. Minors/Nits at author discretion, but m1/m2 (empty-key + degenerate input) should get a one-line policy.
