# BRIEF — Fable deep pass on `btctax income scrub` (POST-PUBLISH)

## The ONE question

Nine review passes ran on this feature. **Every one of them found an instrument that was green
because it had never been watched discriminating** — not a wrong answer, a checker that could not
give one. Each fix was then itself found blind by the next pass.

> **What is still green and blind now?**
>
> Name the **instrument**, not the defect. For each: say what it appears to guarantee, what it
> actually asserts, and **what would have to be true for it to discriminate** — i.e. the mutation
> that ought to kill it and would not.

A finding here is *"instrument X cannot fail on class Y"*. A finding is **not** "here is a bug",
though if you find one, say it — it just is not what this pass is for.

## Why this is worth paying for after the fact

`income scrub` shipped in **v0.17.0**, live on crates.io, ten crates. **Its product is an
authorization**: it tells a filer *"this file is safe to hand to a stranger."* A wrong figure is a bug
they can audit; a wrong safety claim is one nobody can audit afterwards, and it is now in every
installed copy — crates.io yanks, never deletes. The remedy for a finding is a 0.17.1 and a yank:
worse than catching it, better than never knowing.

## ★★★ The record — nine passes, one shape

| pass | the instrument that was green and blind |
|---|---|
| code review (r1) | — (found the defects; the instruments came later) |
| spec r2 | §3.3's field axis and §5's divergence set: **hand-lists** dressed as mechanisms |
| spec r3 | r2's *replacements* were hand-lists one level down; §5.1 missed the IP PIN, which is replaced, printed, and invisible to both §3.3 tests |
| spec r4 | r3's derivation named the **wrong blind spot** (value collision, not structural absence) and authorised its exemptions by the **wrong discriminator** (the type, which does not decide malformed-ness) |
| r5 fold check | maximality defined over *presence*, so an empty `String` collapsed exactly as an absent `Option` |
| whole-branch | **a CRITICAL**: a malformed EIN upgraded to a well-formed synthetic ⇒ the copy files where the original refused and claims \$1,546.80. §3.2 named the EIN; `EinMap`'s doc *delegated* to §3.2; §3.2 had no EIN leg. **A pointer to another section is not an implementation.** |
| …and fixing it | **§3.3's matrix refusal assertion had been comparing `Some(ForeignTrust)` to ITSELF on every row.** `screen_inputs` returns the FIRST refusal and the fixture refused unconditionally. The entire assertion asserted nothing. Four further masks sat behind it, each hidden by the one before. |
| fold check | the `--out` mode test was blind to the write-then-narrow window it existed for; the round trip could not fail on a lossy emit; a scan list had 3 entries naming tokens that no longer existed |
| nit fold check | the inode kill-test could flake red on ext4; a false citation inside the commit that closed a false citation |
| CI (twice) | the A3 write-hook test depended on an ambient `target/debug/xtask`; **the Windows PII test ran the WSL launcher and had never executed a shell** — reporting *"the PII exclusion rule misclassified 15 of 23 vectors"*, an accusation against a security control, for months |

## ★★ ALREADY MACHINE-CHECKED — do not re-derive, and do not re-run

- **2666 tests pass; CI green on ubuntu, macos AND windows** (first fully green run this repo has had).
  `fmt`, `clippy -D warnings`, `msrv`, `check-isolation`, `pii-scan` all green.
- **~23 mutations were planted and killed**, each observed red before the fix: the four refusal
  disjuncts (one test each), the raw-string EIN key (2 tests), dependent/spouse/taxpayer SSN
  unconditional, `business_description` unconditional, broker payer unconditional, `ip_pin Some("")→None`,
  the `NotDigits` payload leak, spouse pass-through, `address_city` kept, dependents dropped, the
  committed-row read, bare `fs::write`, no marker, no import guard, the EIN class defect (2 tests),
  `remove_file` deleted, `payments` dropped from the emitter.
- **The 22 instruments** guarding scrub are listed at the end of this brief.

**That list is exactly what makes this pass answerable: every one of those mutations is a class
someone thought of. The question is which class nobody thought of.**

## Read these — by path

| path | what |
|---|---|
| `design/SPEC_income_scrub.md` | the spec (r6) the build followed |
| `reviews/scrub-*.md` | **twelve persisted reviews.** Four carry `CLEAN` sections recording what could not be broken |
| `crates/btctax-core/src/tax/scrub.rs` | the scrubber |
| `crates/btctax-core/src/tax/scrub_axis.rs` | the derived axis, the maximal sentinel, §3.3's matrix |
| `crates/btctax-cli/tests/scrub_refusal.rs` | the CLI instruments |
| `crates/btctax-cli/src/cmd/tax.rs`, `src/main.rs` | the refusal, marker, import guard, `--out` write |

## FORBIDDEN — this is where a pass like this usually goes wrong

- **A fresh audit.** Nine passes covered correctness. Re-deriving a settled finding spends the budget
  that makes this pass worth running.
- **Re-opening the CLEAN sections.** Four reviews record what they could not break, with reasoning.
  If you believe one is wrong, say which and why in one line — but do not re-walk them.
- **Re-litigating decided things**: `absent → absent`; the ledger is never scrubbed (figures on a
  public chain *are* the identifier); no perturbing figures; the refusal is a `CliError` not a
  `RefuseReason`; **the `year - 1` disjunct is deliberate over-refusal securing no live read — two
  attempts to name a mechanism for it were both wrong and a third is not wanted**; the structural EIN
  window stays refused; the IP PIN is never synthesised when valid.
- **Style, prose, naming, doc-comment length.** Several docs are long on purpose.
- **"Add more tests"** without naming the specific class the new test catches and the mutation that
  would kill it.

## ★ Where I would look, offered so you can disagree

Not a checklist — if these are wrong, say so and go where the evidence leads.

1. **The derivation's own fixture.** `maximal_sentinel` is now the authority for the field axis, the
   matrix, and the round trip. It is one hand-written literal. What class of change to it silently
   shrinks what everything downstream can see? (`box12: vec![]` is one; are there others?)
2. **`screen_inputs` returns the FIRST refusal.** That property already destroyed one whole assertion
   surface. Where else does a first-wins/short-circuit read make a checker unable to distinguish?
3. **The emitted TOML is the artifact, but every instrument compares Rust values.** Nothing asserts a
   property of the *bytes a recipient receives* except the marker's presence and one round trip.
4. **`assert_emptiness_class_preserved` walks string leaves in parallel with `zip`.** Its safety was
   argued from "scrub changes no collection length" — an argument about today's code, not an invariant.

## OUTPUT FORMAT

```
VERDICT: <sound | needs-changes | wrong-shape>

STILL GREEN AND BLIND:
  For each instrument:
    INSTRUMENT: <name, file:line>
    APPEARS TO GUARANTEE: <what a reader would take from it>
    ACTUALLY ASSERTS: <what it can fail on>
    THE MUTATION IT WOULD SURVIVE: <concrete; the class nobody thought of>
    SEVERITY: <C | I | M | N>  — C only for a false safety authorization or a wrong figure

IF NOTHING IS BLIND: say so plainly, and name the two instruments you attacked hardest and how.

WHAT THIS FEATURE STILL CANNOT DO SAFELY: <at most three sentences; the honest scope limit>

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence naming the assumption it depends on>
```

**A verdict of `sound` is a real and useful outcome.** Do not manufacture findings to justify the
pass; equally, do not soften a Critical because nine rounds preceded you — nine rounds preceding you
is precisely why an unfound one would still be here.

---

### The 22 instruments (machine-listed, so you need not reconstruct them)

`a_ledger_free_vault_still_scrubs` · `a_malformed_ein_stays_malformed_so_the_copy_refuses_where_the_original_did` ·
`a_malformed_ssn_keeps_its_variant_without_leaking_the_filers_character` · `an_empty_string_drops_out_of_the_axis_too` ·
`a_none_option_drops_out_of_the_axis_which_is_why_the_fixture_must_be_maximal` · `an_out_of_year_hard_blocker_refuses_to_scrub` ·
`a_pseudo_active_vault_refuses_to_scrub_though_the_year_itself_is_ledger_quiet` · `ein_distinctness_is_preserved_exactly` ·
`every_replaced_field_preserves_its_class_in_every_representable_state` · `importing_a_scrubbed_file_is_refused_without_force_and_accepted_with_it` ·
`in_year_digital_asset_activity_refuses_to_scrub` · `no_fixture_value_collides_with_a_stand_in` ·
`prior_year_digital_asset_activity_refuses_to_scrub` · `scrub_preserves_every_computed_figure` ·
`scrub_reads_the_draft_that_shadows_the_committed_row` · `the_committed_example_tomls_still_import_without_force` ·
`the_derived_axis_reaches_every_field_the_review_passes_were_about` · `the_identity_does_not_survive` ·
`the_out_file_is_owner_only_even_when_it_already_exists` · `the_scrubbed_output_carries_the_provenance_marker_on_both_paths` ·
`the_scrubbed_toml_round_trips_back_through_import` · `the_surviving_sentinels_are_exactly_the_deliberately_kept_fields`
