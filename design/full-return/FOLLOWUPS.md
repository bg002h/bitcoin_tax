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

## From earlier reviews (folded, recorded for traceability)

- (r1–r3 findings were all folded into the spec; see `reviews/SPEC-fable-review-r{1,2,3}.md`.)
