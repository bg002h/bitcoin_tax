# REVERIFY — the step-5 provenance fold (`99468b5d`)

**Verifier:** one sonnet re-verifier, isolated worktree `e181802a`
(`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a90ad6361edc34994`). Nothing committed, no
subagents; two targeted builds under `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
(not the full suite — the controller's `make gate` run stands, per the brief) plus four of my own
plant/restore cycles against `xtask -p forge_reach_check`, all via `cp` backups, all restored and
diffed clean before this report was written.

**Counts: 0 Critical / 0 Important / 0 Minor / 2 Nit.**

---

## Verdict

The fold closes the Important it was built to close. `Form6251Line1::Y2025` now carries
`SeniorDeductionSubtotal` (`crates/btctax-core/src/tax/form6251.rs:82`) instead of the bare
`schedule_1a_line: u32` the prior round found forgeable, and the emitter's join
(`crates/btctax-forms/src/form6251.rs:326`) reads the line number off that vouched type rather than
off a caller-settable field. I independently re-derived every claim in Hunt 1 (no `Deserialize`,
`Default`, `From`, `Into`, `FromStr`, `Arbitrary`, or builder anywhere in the
`SeniorDeductionSubtotal` → `Form6251Line1` → `Form6251` chain) rather than trusting the fold's
grep, and it holds: exactly two construction sites of `SeniorDeductionSubtotal {` in the whole
workspace (`schedule_1a.rs:1394` the accessor, `:1492` inside `testonly`, both same-module-tree by
Rust's own privacy rule), and exactly five external constructors of `Form6251Line1::Y2025 {` outside
`btctax-core/src` (`f6251_obbba.rs:781,989,1153`, `f6251_fill.rs:190`,
`line_coverage_check.rs:1538`) — matching the fold's corrected count, not the ledger's stale four.

The new `crates/xtask/src/forge_reach_check.rs` is not merely claimed to discriminate — I planted
both directions myself, independently of the fold's own plants (different call site, different
mismatched value), and watched it red both times, then restored and re-ran green. I also drove the
one blind spot the module's header states — a forge call from a `#[cfg(test)] mod tests` inside a
`src/` file — and confirmed both halves of the claim exactly: it is *not* reported as reaching
production, and it is *also* not counted toward the anti-vacuity floor, so removing the real
`tests/`-directory caller while adding this one reds the floor rather than passing silently. That is
the honest failure direction the header names, and it is what actually happens.

Nothing here rests on an unverified premise. Two Nits recorded below (neither gates); no Minor,
because neither rises to a real defect — one is a stale number in a doc comment, the other is a
theoretical extension of a blind spot the guard's own tests already exercise under a different name.

---

## Findings

None at Critical or Important. Two Nits:

**Nit 1 — `FORGE_head`'s file-floor doc comment cites a stale measurement.**
`crates/xtask/src/forge_reach_check.rs:58`: `"crates/**/*.rs measured 358 on 2026-09-11"`. My own
count on this tree, today (2026-09-11), is **355** (`find crates -name "*.rs" -not -path
"*/target/*" | wc -l`). Both numbers clear `FILE_FLOOR = 300` by a wide margin, and the floor is
deliberately loose (per its own comment, "a check that scans nothing passes by finding nothing," not
a pinned count) — so this has no effect on what the guard catches. Recorded because a stale
self-measurement in the same commit that made the measurement is exactly the kind of thing `CLAUDE.md`
asks reviewers to catch, even when it doesn't move the outcome.

**Nit 2 — the header states one blind spot; a second, narrower one exists but is provably inert.**
`production_source` (`crates/xtask/src/r15_stop_list.rs:75`) strips every line starting with `//`
(which includes `///`) before the `#[cfg(test)]` skip logic ever runs. That means a forge call typed
inside a ```rust fence in a doc comment — a doc-test — is invisible to `forge_reaches_production` for
a *different* reason than the stated one (comment-stripping, not the test-item skip), and is also not
counted by `forge_call_sites` (a doc comment isn't a file under `/tests/`). This is not a new
mechanism, though: it is the identical stripping behavior the module's own
`a_production_call_to_the_forge_reds_and_its_near_misses_do_not` test already exercises as "near miss
2" (a `///` comment *naming* the forge is correctly not a call) — doc-test *code* is swallowed by the
same rule as doc-test *prose*. It cannot smuggle an actually-executing production call (doc-test code
is never compiled into the shipped library or binary — only into a separate binary under `cargo test
--doc`), and if someone tried to satisfy the anti-vacuity floor with a doc-test instead of a real
kill, the floor would still correctly red, exactly as it does for the `#[cfg(test)] mod tests` case I
drove directly. Recorded as a boundary the header could name explicitly, not as something the
guarantee depends on.

---

## Refuted premises

None. Every row in the brief's "already settled" table reproduced under my own re-derivation
(independent plants, independent greps, independent builds), and no claim in the FOLD report or the
ledger/brief it worked from was found false on inspection.

---

## Checked clean — one line per hunt

1. **Every route to a forged subtotal, derived.** Grepped the whole workspace for `derive(` next to
   `SeniorDeductionSubtotal`, `Form6251Line1`, and `Form6251`: `SeniorDeductionSubtotal` derives only
   `Debug, Clone, Copy, PartialEq, Eq` (`schedule_1a.rs:1432`); `Form6251Line1` derives
   `Debug, Clone, PartialEq, Eq` plus a hand-written `impl Default` that returns `Y2024` (never
   touches the `Y2025` variant or `SeniorDeductionSubtotal`, `form6251.rs:86-91`); `Form6251` derives
   `Debug, Clone, PartialEq, Eq, Default` (its `Default` composes `Form6251Line1::default()`, same
   `Y2024` path). Grepped for `Deserialize`, `Serialize`, `From<`, `Arbitrary`, `FromStr` anywhere
   touching any of the three types across `crates/`: zero hits. Confirmed the only two construction
   sites of `SeniorDeductionSubtotal {` in the whole workspace are `schedule_1a.rs:1394` (the
   accessor) and `:1492` (the forge, inside the `testonly` child module — which Rust's own privacy
   rule permits, since a private field is visible to its defining module *and descendants*, exactly
   why the forge needed to live there rather than being blocked by the type itself). Confirmed the
   `.map.toml` files that co-occur with "Form6251" + "toml" in a grep are `Form6251Map`/
   `Form6251ObbbaMap` (PDF field-position config, `map.rs`), an unrelated type — not a serialization
   path for `Form6251Line1`. No route exists.
2. **The guard's own partition, and its stated blind spot.** Read `production_source`
   (`r15_stop_list.rs:75-114`) in full: it skips lines from a `#[cfg(test)]` line's start-of-line match
   through the balanced end of that block or declaration, and resumes scanning after — the specific
   defect this file's own header records being fixed once already (truncating at the first
   `#[cfg(test)]`). I did not take the module's honesty claim on the ledger's word: I built the
   workspace twice with `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` and (a) planted a
   real forge call in `crates/btctax-forms/src/form6251.rs` (a different production seam than either
   of the fold's own two plants) — reds, quoting the exact line; (b) neutralized the sole `tests/`
   caller — reds with the anti-vacuity message; (c) restored both to a byte-identical `git diff`-clean
   state, confirmed by `diff` against `cp` backups; (d) added a forge call inside the *existing*
   `#[cfg(test)] mod tests` block of `crates/btctax-forms/src/f6251_revision.rs` (a `src/` file) while
   simultaneously neutralizing the real kill's forge call, and confirmed the result is a **floor
   failure naming "no kill test calls the forge any more"** — not silence, and not a false "reaches
   production" — exactly the header's claimed direction to fail in; restored, clean. `build.rs`
   (`crates/btctax-forms/build.rs`) and one `tests/fixtures/examples` directory exist under `crates/`;
   both are swept by the walk and treated correctly by mechanism (build.rs is production-scanned since
   it isn't under `/tests/`; the fixtures directory's path contains `/tests/` and is treated as
   lawful) — neither is a gap. See Nit 2 for the one boundary not named in the header but proven
   inert.
3. **Did each of the five construction sites get the right treatment?** Read all five in the current
   diff: `line_coverage_check.rs:1538` and `f6251_fill.rs:190` and `f6251_obbba.rs:781,989` all call
   the new `vouched_senior_subtotal()` helpers or `Schedule1A::senior_deduction_subtotal(None)` — a
   real accessor call, never the forge. `f6251_obbba.rs:1153` (the kill) is the sole forge call in the
   workspace, and its mismatched line (`let other = if printed == 37 { 43 } else { 37 };`,
   `f6251_obbba.rs:1150`) is derived from the revision's own printed line, never a typed literal —
   confirmed by reading the surrounding test body directly, not by re-quoting the fold's report. Ran
   `the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite` and the
   `f6251_revision::tests` unit tests on the untouched tree: all pass.
4. **The emitter's destructure.** Read every site that pattern-matches `Form6251Line1::Y2025` in the
   current diff (`btctax-forms/src/form6251.rs:296-300`, `btctax-core/src/tax/form6251.rs:338-342` in
   `printed()`, `:509` in `compute_6251`'s match, `:1108` in a test, `line_coverage.rs:707`,
   `return_1040.rs:11911,12055`): every one names all three fields (`line1a`, `line1b`,
   `senior_deduction`) and none uses `..`. A field added to the `Y2025` variant tomorrow is `E0027` at
   every one of these — the property the fold's report claims.
5. **The three corrected doc comments.** Read all three against the diff and the primary mechanism,
   not the report's paraphrase: `schedule_1a.rs`'s `senior_deduction_subtotal` doc now says "only
   lawful way" and names the forge as the one other route — true, since the forge is a real second
   constructor and the comment says so rather than hiding it. `SeniorDeductionSubtotal`'s struct doc
   adds "and that now holds inside this crate too... Every module but this one... gets E0451" — true,
   reproduced independently in Hunt 2's plant (b) above landing an `E0451`-shaped refusal via the
   guard, and the fold's own citation of a direct `return_1040.rs` plant is consistent with the
   privacy rule I traced in Hunt 1. `f6251_revision.rs`'s module doc adds "That sentence only became
   true on 2026-09-11" — consistent with the git history (`27d3351a` ledger predates `99468b5d`
   same-day) and with the mechanism itself (the sentence was false while the variant carried a bare
   `u32`, which is exactly what this fold changed).
6. **FR-123's deferral reasoning.** Read `f6251_revision.rs:176-179`
   (`schedule_1a_line_agreeing_with(&self, read_by_the_computation: u32)`) and its only production
   caller (`form6251.rs:326`, passing `senior_deduction.schedule_1a_line()` — a value read off the
   vouched type, not typed by the caller). Its unit tests (`f6251_revision.rs`'s own
   `#[cfg(test)] mod tests`, confirmed via `cargo test -p btctax-forms --lib f6251_revision::` — 5
   passed) call it with bare integers including a deliberate 37/43 mismatch, and the only way to
   supply a genuinely mismatched *vouched* value for that pure-parsing test today would be the forge —
   from inside a `src/` file's `#[cfg(test)] mod tests`, which Hunt 2's plant (d) proved is exactly the
   location the guard cannot count as a lawful caller. So typing the parameter today would add a forge
   use the guard cannot attribute either way, rather than merely moving the existing one — FR-123's
   stated reason holds, and the Minor/TY2026-owning-phase severity call is consistent with the "no live
   path today" standard the prior round used for the analogous case.
7. **Scope creep.** Re-read `git show 99468b5d --stat` against the fold report's own file list: 13
   files, matches exactly (11 modified, 1 new source file, 1 new report). `FOLLOWUPS.md`'s +21 lines
   are exactly one FR-123 entry (`grep -c "^- \*\*FR-123"` → 1; no duplicate FR numbering in the
   120-123 range). No `2026` arm, no `FullReturnParams`, no fail-closed gate, no touch to any of the
   previous fold's six items appears anywhere in the diff.

---

## Out of scope

The rest of the branch, `d8d023af` and earlier folds (re-verified clean in prior rounds), any TY2026
wiring, `FullReturnParams`, and re-running `make gate` in full (the controller's run stands; every
claim above is either a source read, a grep, or one of my own isolated `cargo test -p xtask
forge_reach_check` / `cargo test -p btctax-forms --lib` / `--test f6251_obbba <name>` runs, each
restored to a `git diff`-clean state before the next).
