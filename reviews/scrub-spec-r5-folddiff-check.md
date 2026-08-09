# r5 fold-diff check — `design/SPEC_income_scrub.md`

_Scoped independent check (Sonnet, read-only) of `git diff 27f9a633..178b941e` against the r4 report.
Dispatched per `reviews/BRIEF-scrub-spec-r5-folddiff.md`. NOT a whole-document review — r4 recommended
against a fifth round. Persisted VERBATIM before any fold._

---

VERDICT: needs-changes

I-1: partially — The fold correctly relocates the blind spot to structural absence and fixes it for `Option`/`Vec`/nested-struct fields (clause 1) plus locks the fixture to an exhaustive literal (clause 2), and it adds a hard backstop — the derived set "must contain `w2s[].ein`, `header.ip_pin`, `foreign_country_names`, `b_1099[].payer`" (§3.3, §8 step 4) — which does genuinely force those four fields to a real fixture value (verified against `scrub.rs:261` filter, `scrub.rs:164` unconditional `None`, `scrub.rs:175-177`, `testonly.rs:498`). But the "maximal" definition constrains *presence* only (Option→Some, Vec≥2, struct present) — it has no clause requiring a plain `String` field to be non-empty, and the removed pre-r4 clause that would have done that ("every replaceable string holds a distinct, recognisable value") does not reappear anywhere in the current text (grepped: 0 hits for "recognisable"/"distinct value" in the whole spec). `schedule_c.business_description` is a plain `String` (`return_inputs.rs:327`) that this very spec (§3.2, §8 step 4: *"business_description is the one that matters"*) mandates become conditionally replaced — trim-empty in ⇒ `""` out, exactly the ein/foreign_country_names mechanism. If the maximal fixture's `business_description` is `""`, it silently drops out of the derived set (same collapse as the original four), and neither the three maximality clauses nor the four-name backstop catches it — the "one per derived field" `assert_ne!` can't help either, since a field never enters the derived set in the first place. This is the identical circularity the fold's own text names ("you would have to already know the replaced set to populate it"), now recurring on a fifth field the document itself flags as important, one row above the fix in §5.1's own table (line ~501, class `printed`).

I-2: closed — The split (`absent` ⇐ type; `malformed` ⇐ "does a predicate read a validity class off the field") is coherent and I could not find a fourth reader: grepping `packet.rs`/`return_1040.rs`/`return_refuse.rs` for other validity-class checks over any field `scrub_pii` replaces (address, occupation, first/last name, business_description, payer names) turned up nothing besides the three named (`Ssn::canonical` at `packet.rs:208`,`:425`; `IpPin::canonical`, def at `packet.rs:167` — the fold's cited `:169` is 2 lines into the body, not the signature, immaterial); `canonical_ein`, used at `return_refuse.rs:936` for the excess-SS screen. The dependent/spouse-SSN live vector is preserved intact and correctly attributes the exemption to "no predicate reads it," not "the type." No unauthorized or double-authorized cell found — `absent` and `malformed` answer disjoint questions on the same field with no conflict.

M-1: closed — §5.1 row now keyed `w2s[].ein` → "the excess-SS advisory (`advisories.rs:706`), via `NonCreditableSs.ein`"; verified `NonCreditableSs.ein: String` (`return_1040.rs:696`) is populated from the canonicalized W-2 EIN and read at `advisories.rs:706`. No duplicate `w2s[].ein` row in the table.

M-2: closed — new row for `header.dependents[].date_of_birth`, citing `packet.rs:418-428`; verified that range is the `DependentRow` *construction* site inside `ReturnHeader::build` (struct def itself is at `:225-229`) — a different but equally valid citation for the same claim (only `name`/`ssn`/`relationship` set), carried over unchanged from r4's own verified citation.

M-3: closed — `serde_json::to_value` now named explicitly, `Cargo.toml:19` spot-checked and is exactly `serde_json = {...}`; the "presence is a difference" clause is stated in the mechanism paragraph (§3.3).

N: closed — step 1 now reads "r5 is this document"; step 9 now reads "r1–r4 covered ... all four are persisted in `reviews/`" — confirmed 4 files exist (`scrub-spec-r1-review.md` … `r4-review.md`).

NEW DEFECTS INTRODUCED BY THE FOLD: none beyond the I-1 residue above (that is a completeness gap in the new fix, not a fresh unrelated defect — see I-1 for full evidence and citations).

WHAT WOULD MAKE THIS CHECK WRONG: if an implementer, on building the actual §8-step-4 fixture, naturally populates every scalar `String` field (including `business_description`) with realistic non-empty content as a matter of course — never choosing `""` for a field the "maximal" wording doesn't explicitly forbid — the I-1 residue never manifests and I-1 should read `closed`.
