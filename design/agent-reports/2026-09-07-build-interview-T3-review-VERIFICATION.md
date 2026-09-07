# Verification ledger — interview T3 seam review (`2026-09-07-build-interview-T3-review.md`, 0C/5I/5M/1N)

Controller machine-checks from the main tree at `4260080c`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| I1 | two TOML keys (`direction` → `NoDollar` on the 2024 f1040 `Income` block; line 1b's cover → `Advisory::UnmodeledDeductionsOmitted`) leave `census-join` green | re-planted both, ran `xtask census-join`, reverted | *"298 unmodeled entries across 13 maps, every one placed … and covered"* — **green** | true; **FOLD** — the direction VALUE is the one un-anchored field |
| I1 | every `NoDollar` block is rangeless and every ranged block is `Understates`/`Overstates`, the sole rangeless non-`NoDollar` block being Schedule 1's 1099-K line | `tomllib` over all maps | exactly one exception, `f1040s1` *"For 2024, enter the amount reported to you on Form(s) 1099-K"* — `Overstates`, no range (placed by `part`) | true; the structural rule holds on the committed tree |
| I2 | `form_1099b_gains` is added into the ledger's capital net, so the refusal's named route double-counts | `grep -n form_1099b_gains return_1040.rs` | `:665` definition, used at `:691` and `:1333` | true |
| I2 | the 1099-B's own text makes the summary optional | `grep 'leave this line blank and go'` | `return_inputs.rs:165` quotes *"…report all these transactions on Form 8949, leave this line blank and go to line 1b"* | true; **FOLD** (declaration without a transcription demand) |
| I3 | `Form1099G` has no box 2 | `awk` over the struct | `payer, payer_tin, transcribed_on, box1_unemployment, box4_fed_withheld` — no box 2 | true; **FOLD** (declare without demanding; box 2 is T5's) |
| I4 | Schedule C carries 88 `unmodeled` entries per year with no `covered_by`, line 6 among them, no direction table, and nothing in core names Form 4136 | `grep -c` per map; `grep -rn '4136\|fuel tax' crates/btctax-core/src` | 2024: 88 / 0 covered / line 6 present / 0 direction; 2025 identical; core: 0 hits | true; **FOLD** the two-line cover now; the join's scope → FR-66 |
| I5 | the new advisory tells a filer the §6013(g)/(h) election does not change their tax and to mark it by hand | `sed -n 518,529p advisories.rs` | contains *6013* and *"mark the form by hand"* | true; **FOLD** (split the sentence); an NRA-spouse gate → FR-67 |
| M1 | stale doc comments in `document_census.rs` state the pre-fold rule | exact-phrase grep 0 (wrapped text) | not decisive by grep; the fold reads the three passages by line | fold inline |
| M2 | `form_1098`/`form_1098e` unconditionally non-live, undocumented as a deviation | the controller's own T3 brief ruled it | ruled by the controller (non-live until T9 / T5 replace the scalars); it was omitted from the D-list | record as D16 with owners; no behaviour change |
| M3 | dead-sum lines' covers name one addend's refusal | read | true; a limitation the join cannot express | record at the site |
| M4 | the D11 flip misses the PAREN convention (Sch 1 8a/8d/8s) | read | true and conservative (stricter, never laxer) | FR-68, owning task T5 |
| M5 | the census rows are asked after the residual attestation | enum order in `questions.rs` (`OtherOutOfScopeIncome` :98, `DocW2` :123) | true | **fold**: sort `live_questions`' output document-first; the registry array untouched |
| N1 | `census_tristate!`'s `clear` skips the liveness guard | read | true | fold inline |

## Disposition

- **I1 — FOLD, two parts.** (a) Structural: a block with a line range may not be `NoDollar`
  (`census-join` reds). (b) The direction VALUE leaves the maps: `direction` is deleted from every
  `[[direction]]` block, and the join derives it from the block's verbatim caption through ONE code-side
  reading `DIRECTION_OF_CAPTION: &[(&str, Direction, &str /* the form's own total sentence as
  evidence */)]` in `census_join.rs`, asserted both ways (every committed caption has a reading; every
  reading names a caption some map carries). A map can then state no direction at all, so plants 1–3
  are unrepresentable, and the reading lives in one reviewed place beside the rule that consumes it,
  each row citing the sentence that justifies it (*"Combine lines 1 through 7 and 9"* into Schedule 1
  line 10 → Form 1040 line 8 → income). Spec §8's line 1220 is amended by the controller to say what the
  guarantee delivers: the caption is read off the form, and its sign is a reading pinned once in code
  with the form's total sentence as evidence — never a per-map key.
- **I2 / I3 — FOLD, one mechanism.** Split the rows invariant into two predicates: `declared_rows(kind)`
  (`Some(len)` for every `Vec`-bearing kind — the CONTRADICTION rule `Some(false)` over rows keeps
  working for all five) and `requires_transcription(kind)` (`true` for `w2`, `int_1099`, `div_1099`;
  `false` for `b_1099` — the form's own sentence makes the summary optional, and the ledger is the
  crypto filer's 1099-B — and `false` for `g_1099` until T5 adds `box2_state_refund`). `entry_route
  (B1099)` stops naming a route that double-counts; the 1099-G row's detail names the scalar
  `sch1.state_refund_taxable` as where box 2 reaches the return today and T5 as the screen.
- **I4 — FOLD** the two-line cover (attestation limb in the form's words + `covered_by` with `names`
  on both years' Schedule C line 6); **FR-66** files the join's scope (Schedule C's 88 entries, Schedule
  1-A, SE, 8949 …) with owning task T5.
- **I5 — FOLD** the sentence split; **FR-67** files the NRA-spouse election gate (owning task T8, with
  the filing-status gates).
- **M1, M3, N1 — fold inline. M2 — D16 recorded. M4 — FR-68 (T5). M5 — fold** (document-first order in
  `live_questions`' output).

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T3-review.md`.
