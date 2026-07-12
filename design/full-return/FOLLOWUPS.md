# FOLLOWUPS — full-return v1

Non-blocking items deferred from the spec/plan review loop. Fold at plan time or later; none gates.

## From Fable spec review r4 (5 Minors — spec is GREEN 0C/0I with these open)

- **fr-schedc-27a** — Schedule C fill: the single `expenses` scalar should land on **Part V line 48 → line
  27a** (so the form cross-foots) rather than 27a with Part V blank. Mechanical map detail; resolve when the
  Schedule C map is extracted (SPEC §7.3, phase 6).
- **fr-se-sscap-clamp** — SPEC §5 stage 7 Sch SE L10 paraphrase omits the SS-cap `max(0, SS_base − wages)`
  clamp. Frozen `se.rs` and the existing `schedule_se.rs` filler both already clamp, so this is a spec-text
  paraphrase gap (fail-loud, not file-wrong). Tighten the wording; no code impact.
- **fr-schb-user-forced** — SPEC §7.1 Schedule B trigger lists "or user-forced" but no `force_schedule_b`
  input is named. Either add the input or drop the clause; harmless (always-file is valid).
- **fr-8962-taxonomy** — SPEC §9.2: excess-APTC / Form 8962 (Sch 2 L2) is listed under REFUSALS but is really
  "unrepresentable / would-refuse-if-captured" (no input exists), and is absent from §1.2's out-of-scope list.
  Move to list (iii) and add to §1.2.
- **fr-profile-diagram-nit** — SPEC §2 diagram labels `TaxProfile` "(2 scalars)"; it is the ~9-field struct
  (deep/02 §1.3). Pure diagram nit; the "2 scalars" is the load-bearing *objective*, not the field count.

## From Fable PLAN review r2 (4 Minors — plan is GREEN 0C/0I with these open)

- **pm-r2-m1** — plan KAT-ownership line mislabels the single-employer excess-SS refuse row as compute-
  dependent; it is input-screenable (P1). One-word fix in the ownership block.
- **pm-r2-m2** — the "KAT 9 → P0 (arithmetic + round-mode)" annotation re-blurs the P0 task-1 (mode) vs task-6
  (cross-foot) split; drop "round-mode" from the KAT-9 label (mode is task 1's discriminating cells).
- **pm-r2-m3** — P1 task-3's parenthetical "(no vault can hold ReturnInputs yet)" is false at phase end; the
  stub is fail-closed regardless — reword to "stub is fail-closed."
- **pm-r2-m4** — P0 task 0 FROZEN pin: make explicit that what-if / pseudo-reconcile / existing-crypto-test
  files are "never alter" (would break loudly) but are not content-pinned (only the 3 delta-path files are).

## From Fable IMPL-P0 code review r1 (folded into P0 r2; 2 recorded/deferred here)

- **p0-taxtable-deviation** (RECORDED — no action) — SPEC §8 / plan-task-5 said to add the standard
  deduction to the per-year `TaxTable`; the impl instead put the full-return indexed params in a NEW
  `FullReturnParams` / `BundledFullReturnTables`. Justified on true grounds: `TaxTable` is a published-crate
  API read by the crypto-delta path (which never needs these fields), and v1 bundles TY2024 only, so a
  separate fail-closed-gated table has the smallest blast radius. (The original code comment claiming a
  frozen `se.rs` struct-literal blocked it was WRONG — `se.rs` only calls the unfrozen `synthetic_table` —
  and has been corrected.) Recorded for traceability.
- **p0-cc0-crosscheck** (DEFERRED → Phase 7) — the P0 acceptance "CI cross-check vs a vendored CC0 PSL
  Tax-Calculator param slice" is not yet implemented. Deferred to P7 (where the independent oracles —
  tenforty / PolicyEngine / IRS ATS — live). Justification: P0's numeric values are already
  **primary-source-verified** (Fable re-fetched Rev. Proc. 2023-34; the 5 QDCGT fixtures are cent-exact
  against the official 2024 worksheet), so the CC0 diff is an *additional independent layer*, not a P0
  correctness blocker. Vendor the TY2024 slice + the diff test in P7.

## From Fable IMPL-P1 code review r1 (C1/I1–I5 + M1/M4/M5/M6 FOLDED into P1 r2; deferred items here)

- **p1-per-field-subcommands** (DEFERRED → follow-on) — v1 ships only the TOML bulk-import (`income import`)
  + `income show` (JSON) + `income clear`. Incremental per-field editors (`income add-w2`, `add-1099-div`,
  …) are a usability follow-on, not a v1 gate — the offline TOML round-trip is the complete input surface.
- **p1-show-as-json-not-toml** (DEFERRED → follow-on) — `income show` emits pretty JSON, not TOML, because
  serde-toml requires scalar keys before nested tables and the nested `ReturnInputs` model violates that
  ordering. A faithful TOML round-trip-out (custom serializer or field reorder) is a follow-on; import
  accepts TOML today, which is the load-bearing direction.
- **p1-consumer-sweep-P2** (SCHEDULED → P2, MANDATORY) — the `resolve_profile` single-source resolver + its
  `Provenance` are wired into `report --tax-year` in P1, but `optimize` / `what-if` / `export` / the TUI still
  read `tax_profile::get` directly. P2 MUST route every profile consumer through `resolve_profile` and print
  the provenance, or a year with `ReturnInputs` silently gives those paths a stale/absent profile. Tracked as
  a hard P2 task, not opportunistic.
- **p1-carryover-writeback-P3P4** (SCHEDULED → P3/P4) — charitable + capital-loss carryovers are read from
  `ReturnInputs` but not yet written back (next-year carryforward_out). The write-back lands with the
  Schedule A / Schedule D compute stages in P3/P4; P1 only stores the declared inputs.
- **p1-task4-row-reclassification** (DEFERRED → task-4 follow-on) — reclassifying an imported inbound row
  (e.g. income ↔ self-transfer) from inside the full-return flow is out of P1; the existing ledger
  reclassification commands remain the path. Revisit when task-4 row editing is specced.
- **p1-r1-m2-excess-aptc** (NOTE — already tracked) — the impl leaves Sch 2 L1a (excess-APTC) with no input,
  consistent with `fr-8962-taxonomy` above (unrepresentable / would-refuse-if-captured). No new action; this
  cross-links the two so the P3 Schedule-2 filler doesn't treat L1a as a live zero.
- **p1-r1-m3-dob-option-pin** (NOTE) — `Person.dob` is `Option<NaiveDate>`; the age-65/blind and kiddie-tax
  paths must treat `None` as "not established" (fail-loud / no silent age-0), never as a birthdate. Pin this
  contract in the P2 `derive_tax_profile` doc + a KAT when the age-dependent standard deduction lands.

## From the whole-design Fable audit (Minors — C1/I1/I2/I3 were FOLDED into spec r5/r6; these Minors remain)

- **audit-minors** — the audit's Minors M2–M8, M10, M11 are recorded in
  `reviews/DESIGN-fable-audit-final.md` (the confirmation review noted they weren't transcribed here). Named
  examples: derived-profile `pref>TI` clamp mirror in `derive_tax_profile`; a couple of taxonomy nits. All
  ranked Minor by two independent Fable passes; fold opportunistically during the relevant phase. (spec §8
  KAT-3 mod-25 + the Sch 2 L1a/L2 structure are now FOLDED into spec r6, not open.)

## Spec errata surfaced by the plan review (fix spec text; do not re-open the GREEN gate for these)

- **spec-s8-kat3-mod25** — SPEC §8 / §10 KAT-3 says "no bracket edge < $100k inside a $50 bin". That's a
  **TY2024-only** fact (deep/01:59). TY2017 (9,325) and TY2025 (11,925 / 48,475) have edges at bin **midpoints**
  (≡ 25 mod 50), which are harmless (IRS taxes at the midpoint; TCW continuous). Correct the spec wording to
  "every edge < $100k ≡ 0 (mod $25)". The **plan already implements the corrected assertion** (P0 task 4).
- **spec-s48-l36** — SPEC §5 stage 9 carries "− L36 applied-to-2025" but §4.8 `Payments` has no L36 input. v1
  pins L36 = 0/blank (plan P4 task 6); add the input in a follow-on or note L36-always-0 in §4.8.

## From earlier reviews (folded, recorded for traceability)

- (r1–r3 findings were all folded into the spec; see `reviews/SPEC-fable-review-r{1,2,3}.md`.)
