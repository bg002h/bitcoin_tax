# Brief — fold the interview T3 seam review (I1–I5, M1/M3/M5, N1; D16 recorded)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create (revert a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E
'<filter>'` (never `cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and
a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D
warnings` before finishing. Every guarantee lands with a kill seen red once; quote the red. Every
pinned number moved: old → new with cause. `/tmp` is a 32 GB tmpfs — build in the repo's target dirs.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T3-review.md` (read whole) and the
controller's ledger `2026-09-07-build-interview-T3-review-VERIFICATION.md` — every claim there is
machine-verified; its **Disposition** is the fold list and fixes the shape. The build you are
folding: `design/agent-reports/2026-09-07-build-interview-T3-implementation.md` (+ its appendix).
Spec: `design/SPEC_interview.md` r2 R2.2, R3, R12, §5.1, §8 (line 1220 is being amended by the
controller to match part (b) below — do not edit the spec).

## The fold, in order
1. **I1(a) — structural.** In `census_join.rs`: a `[[direction]]` block carrying `first_line` or
   `last_line` may not be `NoDollar`; a `NoDollar` block carries no range. Kill: the reviewer's plant 1
   (the 2024 f1040 `Income` block → `NoDollar`) reds with a message naming the block.
2. **I1(b) — the direction value leaves the maps.** Delete `direction = …` from every `[[direction]]`
   block in the 11 tables (`DirectionBlock` loses the field; `deny_unknown_fields` then reds any map
   that still carries one — that is a kill, keep it). The join derives each block's direction from its
   verbatim `caption` through ONE reading in `census_join.rs`:
   `DIRECTION_OF_CAPTION: &[(&'static str /* caption */, Direction, &'static str /* evidence */)]`,
   where evidence is the form's own sentence that justifies the sign (the total the block's lines
   are combined into and where that total goes — e.g. `Additional Income` ⇒ *"Combine lines 1 through
   7 and 9 … Schedule 1 line 10"* → Form 1040 line 8 ⇒ `Understates`; `Nonrefundable Credits` ⇒ *"Add
   lines 1 through 5 and 7"* → Form 1040 line 20 ⇒ `Overstates`; a `NoDollar` caption cites the cell
   that shows it carries no amount). A KAT asserts the join both ways: every committed block's caption
   has exactly one reading, and every reading names a caption at least one map carries (so a dead
   reading reds). Kills: the reviewer's plants 2 and 3 become unrepresentable (there is no key to edit)
   — demonstrate by planting a `direction` key back into one block → the parser reds; delete one
   reading → the join reds naming the orphaned caption; add a reading for a caption no map carries →
   red. Then re-run K10–K13 and the D11 kills: all still red-then-green.
3. **I2 / I3 — split the rows invariant into two predicates** in `document_census.rs`:
   `declared_rows(ri, kind) -> Option<usize>` (`Some(len)` for every `Vec`-bearing kind — the
   CONTRADICTION rule `Some(false)` over rows, and `apply`'s guard, keep working for all five) and
   `requires_transcription(kind) -> bool` (`true` for `W2`, `Int1099`, `Div1099`; `false` for `B1099`
   — the form's own printed option makes the summary rows optional and the ledger is the crypto
   filer's 1099-B; `false` for `G1099` until T5 adds `box2_state_refund`, with the reason at the site
   naming T5). `DocumentDeclaredNotTranscribed` fires only where `requires_transcription`. Rewrite
   `entry_route(B1099)` so it never names a route that double-counts a ledger return (say instead that
   the ledger's dispositions print on Form 8949 / Schedule D and a `[[b_1099]]` summary row is for
   the form's line 1a/8a option only); the `G1099` route names `sch1.state_refund_taxable` as where
   box 2 reaches the return today and T5 as the screen. Kills: the reviewer's probes P1
   (`b_1099 = true`, zero rows, ledger disposals) → passes; P2 (`g_1099 = true`, box 2 only) → passes;
   `b_1099 = false` with one `[[b_1099]]` row → still refuses; `w2 = true` with zero rows → still
   refuses; a `requires_transcription` flipped to `true` for `B1099` → P1 reds again (quote it).
4. **I4 — cover Schedule C line 6 now.** Add one limb to the residual attestation in the form's words
   (*"other income on a Schedule C, including a federal or state gasoline or fuel tax credit or refund
   (Form 4136)"*) and `covered_by = "QuestionId::OtherOutOfScopeIncome", names = "fuel tax"` on both
   years' `line = "6"` entries of `f1040sc.map.toml`. Because Schedule C is outside the join's 13
   maps, add a narrow KAT that asserts those two entries' cover and reach directly (the prompt contains
   the `names` keyword) so the cover cannot rot silently; the join's scope widening is FR-66, not
   yours.
5. **I5 — the advisory's false clause.** In `advisories.rs` (`UnmodeledReturnOptionsOmitted`), keep
   *"none of these changes your tax"* for the administrative cells only, and give the §6013(g)/(h)
   election its own clause: a substantive election that subjects the nonresident-alien spouse's
   worldwide income to U.S. tax, which btctax has neither asked about nor computed — a preparer's
   return, not a box to hand-check. Snapshot kill: the old sentence's presence reds.
6. **M1** — delete the three stale passages in `document_census.rs` (`:22-26`, `:129-137`, `:468-482`
   per the review) so no comment states the pre-fold rule. **M3** — record at the site of the
   dead-sum covers (Sch 2 1z / 7 / 18) that the join cannot express a conjunction and what would rot.
   **N1** — `census_tristate!`'s `clear` takes the same liveness guard as `get`/`set`. **M5** —
   `live_questions` orders its output document-first: the census rows before every other declaration
   (the registry array untouched; `decl_tristate!`'s index coupling unchanged); the no-brick test
   and the `income answer` goldens/snapshots move accordingly (list each moved line's cause).
7. **D16** — add to the build report's deviation table (append, do not rewrite): `form_1098` /
   `form_1098e` non-live until T9 / T5 replace the scalars, ruled by the controller in
   `BRIEF-build-interview-T3.md`.

## Constraints
- `record_answer` stays the only writer of `answer_log`; no new prompt beyond the attestation limb and
  the advisory clause, both in the form's or the statute's words.
- `line-coverage` counts unmoved; `census-join` still 298 entries / 13 maps; the cover split may move
  only by the Schedule C line 6 addition (it is outside the join's count — say so).
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T3-fold.md`: per numbered item what landed, the
full `DIRECTION_OF_CAPTION` table as committed (caption → direction → evidence sentence), the two
predicates' truth table over the seven supported kinds, every kill with its red text, every pinned
number moved, suite lines per crate. Return only a 4-line summary plus the path.
