# Whole-diff review r2 — INSTRUMENT-INTEGRITY lens (Sonnet)

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `afa0ffe..HEAD`.

**Brief:** not tax correctness (the Opus reviewer had that lens) but **B1**: *for each guarantee this
diff claims, which test reds when the guarantee is removed — and is that test capable of reddening?*
Ten named guarantees. Required to MUTATE and run, not read; restore from cp backup, never
`git checkout --`. Pointed explicitly at the two changes that landed while the whole suite stayed
GREEN (Form 8283 page-2 header, 1040 line-7 box), and at two suspected false-green sites (the
`schedule_c` exemption split, the GAPS ratchet).

**Reviewer output reproduced VERBATIM below.**

---

VERDICT: all-guarantees-held

MUTATION LOG:

| guarantee | mutation applied | KILLED / SURVIVED | test that fired |
|---|---|---|---|
| 1. malformed SSN computes / packet refuses | (a) reintroduced compute-time SSN gate in `screen_inputs` | KILLED | `a_malformed_ssn_computes_but_the_packet_still_refuses_it` |
| 1. (cont.) | (b) `FiledPerson::build` silently substitutes a placeholder SSN instead of propagating `SsnError` | KILLED | same test, "packet refuses" side |
| 2. death gates no longer refuse | `is_aged`'s `(None,None)` arm flipped `false→true` (the named v0.14.0 regression) | KILLED | `the_death_gates_do_not_block_a_return` |
| 2. spouse gate is MFJ-only | dropped the `FilingStatus::Mfj` term from `SpouseDiedDuringYear`'s `live` closure | KILLED | `the_spouse_death_gate_is_asked_only_on_mfj` |
| 3. advisory fires only when DOB qualifies | (a) dropped `born_early_enough` check in `aged_box_forgone_for_unanswered_death` | KILLED | `the_forgone_aged_box_advises_only_when_the_skip_actually_costs_something` ("too young" case) |
| 3. (cont.) | (b) dropped the "gate actually unanswered" check (fires even when answered) | KILLED | same test, "answered ⇒ claimed" case |
| 4. `apply_writes` rejects undeclared on-state | disabled the `/AP /N` on-state guard (`if false && …`) | KILLED | `a_swapped_yes_no_map_fails_closed_instead_of_rendering_a_blank_box` |
| 5. BLIND box has no death carve-out | added a death carve-out to `taxpayer_blind` ("harmonising" it with the aged box) | KILLED | `the_blind_box_has_no_death_carve_out_but_the_aged_box_does` |
| 6. Form 8995 line 3/4/16 chain | (a) `compute_8995` ignores `qbi_carryforward_in` at line 4 | KILLED | `qbi_line3_carryforward_reduces_the_deduction`, `qbi_line3_carry_in_over_current_qbi_zeroes_line4_and_carries_the_remainder` |
| 6. (cont.) | (b) `qbi_carryforward_out` hardcoded to 0 in `compute_8995` | KILLED | same two tests |
| 6. (cont.) | (b′) deleted the exact write-back line CONTINUITY names as having survived the whole suite | KILLED (only by the new test — rest of `btctax-core --lib` stayed green, confirming CONTINUITY's claim) | `form_8995_line16_carries_into_next_years_line3` |
| 6. (cont.) | (c) removed the "line 3 is testimony, print only if nonzero" gate | KILLED (pre-existing test) | `f8995_fills_the_printed_chain_and_reads_back` ("L3 must be blank") |
| 6. (cont.) | (d) dropped `("3", lines.line3)` from the hand-maintained `assert_paren_magnitudes` array | KILLED | `a_negative_line3_is_rejected_like_its_paren_siblings` |
| 7. Form 8283 page-2 identity fails closed | made the writer silently skip when `identity_page2` is `None` instead of refusing | KILLED | `a_full_return_8283_map_without_page2_identity_fails_closed` |
| 8. Schedule C I/J print filer's answer only | `unwrap_or(false)` semantics on an unasked line (the exact pre-empted defect) | KILLED | `schedule_c_lines_i_and_j_print_the_filers_own_answer_and_nothing_when_unasked` |
| 9. 1040 line-7 box tracks `must_file` | inverted `line7_schedule_d_not_required` | KILLED (both directions, same mutation) | `the_packet_omits_every_form_that_is_not_required`, `a_return_that_attaches_schedule_d_does_not_check_the_line7_box` |
| 10. export-irs-pdf carries advisories | (a) full-return export always emits `Vec::new()` | KILLED | `export_dispatches_a_full_return_year_to_the_full_packet` |
| 10. (cont.) | (b) crypto slice emits a non-empty advisory it never computed | KILLED | `crypto_slice_1040_is_watermarked_as_a_worksheet` |
| extra: `coverage.rs` schedule_c leaf-exemption split | over-exempted a leaf (`schedule_c.payments_requiring_1099`) that is also covered by a Field | KILLED | `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` (double-coverage assertion) |
| extra: `field_census.rs` GAPS ratchet | removed the `#`-comment skip, exposing the commented-out `rule = "gap"` legend line in `schedule_se.map.toml` | KILLED (count went 6→7, wrongly) | `recorded_gaps_may_only_shrink` |

No vacuous assertions, no false-empty loops, and no SURVIVED mutations were found across the campaign. Every claimed guarantee has a test that actually reddens when the guarantee is removed, including the two changes that previously slipped through the whole suite unnoticed (Form 8283 page-2 header, 1040 line-7 box) — both are now genuinely covered, not decoratively.

One point worth flagging as an observation (not a finding, since it doesn't survive): the Schedule C I/J advisory-precision guarantee (advisories.rs `the_schedule_c_1099_pair_advises_only_on_the_skip`) and the PDF-writer guarantee (`schedule_c_lines_i_and_j_print_the_filers_own_answer_and_nothing_when_unasked`) are two independent tests over two independent code paths (advisory computation vs. PDF fill) that happen to share a name pattern — I verified each kills its own mutation independently, so this is deliberate defense-in-depth, not redundancy.

TREE CLEAN: yes (`git status --short` empty, full `cargo nextest run --workspace` 2524/2524 passed after every restore).

---

## Disposition (author, same day)

**Nothing to fold — 20 of 20 mutations killed, no survivors, no vacuous assertions.** Two results are
worth keeping for the record beyond the pass/fail:

1. **It independently reproduced the write-back finding.** Deleting
   `next_year.qbi.qbi_carryforward_in = ar.qbi_carryforward_out` left the rest of `btctax-core --lib`
   green; only the new test caught it. That was recorded in CONTINUITY on my own say-so, and is now
   corroborated by a reviewer who re-ran it.
2. **The two changes that had slipped through the whole suite are genuinely covered.** The reviewer was
   told to suspect the tests I wrote afterwards of being decorative and checked them specifically.
   They bite.

★ It also probed the two sites I flagged as likely false-greens and found neither: the `schedule_c`
exemption split reds on an over-exemption, and the GAPS ratchet reds if its `#`-comment skip is
removed (it would then wrongly count the commented legend line in `schedule_se.map.toml`).
