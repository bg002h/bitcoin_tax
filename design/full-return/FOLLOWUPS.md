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

## Spec errata surfaced by the plan review (fix spec text; do not re-open the GREEN gate for these)

- **spec-s8-kat3-mod25** — SPEC §8 / §10 KAT-3 says "no bracket edge < $100k inside a $50 bin". That's a
  **TY2024-only** fact (deep/01:59). TY2017 (9,325) and TY2025 (11,925 / 48,475) have edges at bin **midpoints**
  (≡ 25 mod 50), which are harmless (IRS taxes at the midpoint; TCW continuous). Correct the spec wording to
  "every edge < $100k ≡ 0 (mod $25)". The **plan already implements the corrected assertion** (P0 task 4).
- **spec-s48-l36** — SPEC §5 stage 9 carries "− L36 applied-to-2025" but §4.8 `Payments` has no L36 input. v1
  pins L36 = 0/blank (plan P4 task 6); add the input in a follow-on or note L36-always-0 in §4.8.

## From earlier reviews (folded, recorded for traceability)

- (r1–r3 findings were all folded into the spec; see `reviews/SPEC-fable-review-r{1,2,3}.md`.)
