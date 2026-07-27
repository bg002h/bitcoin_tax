# Form 8275 Part II narrative overflow — design note

**Scope boundary (the one sentence this cycle is measured against):** *This cycle fixes Form 8275 Part II
narrative overflow only — wrapping across the mapped Part II and Part IV continuation lines with a
fail-closed over-capacity error; Form 8949 VARIOUS and Form 8275 Part I row pagination are explicitly
out of scope and remain open follow-ups.*

**Base:** `main` @ `aa969ca`. Branch `fix/f8275-part-ii-overflow`.
**Lineage:** displaces Approach-B **sub-project 3**, which is closed below.

## 1. The defect (shipped in v0.9.0 and v0.10.0)

`crates/btctax-forms/src/form8275.rs:120-123` writes the filer's **entire** Part II narrative into a
single free-text field:

```rust
// Part II — the filer's combined narrative, written whole to the one free-text field (no per-line
// splitting; mirrors form8283's whole-address identity writes).
push_free(&mut w, &mut p, &map.part_ii_narrative, &printed.part_ii);
```

`part_ii_narrative` maps to exactly one single-line 8pt AcroForm field
(`forms/2024/f8275.map.toml:10` → `topmostSubform[0].Page1[0].p1-t80[0]`). A narrative longer than the
field silently renders about its first ~137 characters. **No error, no refusal, no truncation marker.**

Three facts make this Critical rather than cosmetic — all verified against source, not inferred:

1. **It is the disclosure itself.** Part II is the §1.6662-4(f) adequate-disclosure text: the artifact
   that makes a >$0 estimated basis defensible under §6662, and the good-faith record under §6664(c).
   The whole of Approach B rests on it. A fifth of a disclosure is not a disclosure.
2. **The engine already refuses the empty case.** `cmd/admin.rs:399` (KAT-E12) hard-refuses an *empty*
   Part II narrative — then silently prints a fraction of a full one. The guarantee is asserted at one
   end and abandoned at the other.
3. **Nothing could have caught it.** The map has 29 fields; `p1-t81..t85` are present in the bundled PDF
   and **unmapped** (verified: 0 hits). Every Part II test fixture uses the same short narrative — there
   is no Part II string over 200 characters anywhere in the test suite (verified: 0). And `verify_flat`
   is structurally blind: it compares `/V` and passes on a truncated *render* by construction, because
   free placements skip both the geometry and `/MaxLen` checks.

## 2. The fix

Wrap the narrative across continuation lines that already exist in the bundled Rev. 10-2024 asset and
are 100% unmapped — `p1-t81..t85` plus page-2 Part IV `p2-t1..t27`, 32 lines in total. **No new PDF
asset is required.** If the narrative still will not fit, **fail closed** with a named error, mirroring
the shipped Part I >6-row refusal — never truncate.

- Character coverage is asserted at the **fill** layer ("every character of `part_ii` lands in some
  mapped field"), *not* by retrofitting `verify_flat`. The read-back oracle passes on a truncated render
  by design; do not teach it otherwise.
- Text measurement (Helvetica 8pt) is the one genuinely new piece — no width table exists in the
  workspace today.

**Test-first is mandatory here, not stylistic:** the first commit adds a >1500-character narrative
fixture and *watches it fail*. Shipping the fix without seeing the red is this repo's recorded
`untested-guard-pattern`, and the absence of exactly such a fixture is why the defect shipped twice.

## 3. Explicitly out of scope

- **Form 8949 VARIOUS** — see §4.
- **Part I >6-row pagination.** Fails **closed** today (`form8275.rs:95-101`) with a clear message and a
  stated workaround, so it is a UX improvement, not a correctness fix. Follow-up.
- **Part I column-(c) clipping.** `PART_I_DESCRIPTION` (124 chars, 178 with `NO_LOSS_SUFFIX`) also clips
  in its 2-line cell. Shortening it is review-pinned tax copy and would reopen a tax-lens round for an
  unrelated reason. Separate follow-up.

## 4. Sub-project 3 (VARIOUS multi-date 8949 rows) — CLOSED as unnecessary

Recorded so it is not re-opened by default:

- The 2025 i8949 grants "VARIOUS" in column (b) as a **permission** to combine lots acquired on different
  dates into one row. It is **never a requirement**.
- Our shipped column (b) — the tranche's window-end date (`forms.rs:78`, fed from `resolve.rs:1310`) —
  was already adjudicated compliant for a single-row tranche (`design/conservative-filing/SPEC.md:78,259`),
  and can only err toward **short-term**, i.e. *higher* tax. It cannot understate.
- The mandatory `basis_methodology.txt` already labels it as the conservative window-end date
  (`conservative.rs:175-183`).
- Building it would require a `window_start` / `acquired_various` field on **both** `Lot` and
  `DisposalLeg` — neither carries one today, so the emitter cannot currently distinguish a 1-day window
  from a 5-year one — i.e. a taxonomy change on a hot type, which this repo's own
  `whole-surface-sweep-on-taxonomy-change` experience prices at four review rounds.
- The only benefit is **row compaction** when one sale draws across multiple tranches. Nothing a filer or
  an examiner would call wrong.

**Reopen only if** a filer needs multi-tranche row compaction for a real return, or the IRS changes
column (b) from permissive to mandatory.

## 5. Gate

`make check` **and** `cargo fmt --all -- --check`, both, from the first commit. One two-lens review
(tax + architecture) before merge — this writes a filed disclosure. No per-task review rounds; this is
one bounded fix.

## 6. SemVer

PATCH. `btctax-forms` internals; no public API change, no new asset.
