# S5 -- final re-verification of the FR-134..151 burndown, over the INTEGRATED tree

**Scope:** main..HEAD = 376d11419..3149c3936 (the plan commit through the B1a-widening commit),
the fully-integrated result of nine independently-worktreed agents (A1-A5, S1-S4) plus the owner's
Fable-consulted B1a amendment. **No file was edited except this report** -- every check below is either
a read, a build, a test run, or a CLI invocation of an already-committed binary; no plant was written to
a tracked file.

## Verdict

**0 Critical / 1 Important / 1 Minor / 0 Nit.** The nine patches integrate cleanly and the headline
numbers all reproduce independently, including a full, fresh 3646/3646 green run with 0 skips beyond
the documented 12. The one Important is a coordination gap the round's own text flagged and never
closed: two "pending integration" notes in a design doc were never reconciled once the agents they were
waiting on actually landed. The one Minor is a hand-miscounted number inside a follow-up entry that
exists specifically to warn against hand-miscounted numbers.

## Findings

### Important -- design/FORM_AUTHORITY_TABLE_DESIGN.md:235,238,251 -- S4's two "pending integration" notes were never reconciled, and are now false

S4 (FR-151 etc.) could not see A2's or A3's diffs when it wrote its report (neither existed yet in S4's
worktree), so it wrote two explicit forward-looking notes and closed with a request that the coordinator
re-read both notes against A2's and A3's actual merged diffs at integration time, and either confirm and
remove the note, or correct it if its candidate guess was wrong. Both A2 (commit 5cfd30be7) and A3
(commit 7f89baa3a) integrated before the S1-S4 batch (commit 7892c6991) landed, and the B1a commit
(3149c3936, the current HEAD) touches only design/HARNESS.md (confirmed via its own --stat) -- so nothing
after 7892c6991 could have done this reconciliation either. The notes are still present, verbatim, in
the current tree.

- Line 235 (inline pointer, part of the section-9 hand-lists table): "This is a DIFFERENT 'archived' from the
  manifest join... FORWARD-LOOKING NOTE pending agent A3's integration -- see below." This is now
  false. A3's own report and its integration commit message are explicit that the fix produced ONE
  notion of "archived" (the MANIFEST.json join); the old FORMS-row notion is no longer the ratchet's
  input at all. The row has not been rewritten to say so.
- Line 238: "FORWARD-LOOKING (pending integration -- not yet in this tree as of 2026-09-12)."
  Agent A3... I could not verify this against A3's diff: it is not present in this isolated worktree...
  Literally false in the current tree -- A3's diff has been in main since 7f89baa3a, three
  integration commits before HEAD.
- Line 251: the same construction for A2, ending with a most-likely candidate guess (section-10
  step 3) and a second candidate (the hand-lists table above has no row naming LineSet itself).
  Checked directly: section 9's table (lines 220-236) still has no row for LineSet's transcription pieces
  becoming build.rs-generated -- A2's second candidate is confirmed correct and still unaddressed.
  Section-10 step 3 (line 280) is a historical changelog entry about commit bc6dce35 and is arguably not the
  stale sentence at all; nobody adjudicated between S4's two candidates either way.

**Why this is Important, not Minor.** This is not a citation slip -- it is an explicit, self-issued
integration task, written into the artifact by name, that silently did not happen, and the artifact now
asserts things about the tree's own state ("not yet in this tree") that are false by three commits'
margin. A future agent reading this design doc (the next port-machine phase, which both A1's and A2's
reports explicitly hand work to) will be told the wrong thing about how cite_check.rs now decides
"archived," and will not find the LineSet-generation row FR-141/FR-146 created. FOLLOWUPS.md has no
entry for this at all (zero hits for FORM_AUTHORITY_TABLE_DESIGN in that file), so it is not even
tracked as known residue the way FR-152/153/155/156/157 are.

**Concrete failure scenario.** The next agent assigned FR-135's remainder (re-pointing the 12
Coverage::quoting("2024") literals -- "the port machine / step-24 widening," per the FR-135 ledger
entry) reads section 9's table looking for how LineSet was derived, to model its own per-year Coverage
rows the same way; finding no row, it either re-invents the pattern from scratch or misses that the same
technique (build.rs reading one header field) already exists one file over.

### Minor -- FOLLOWUPS.md:7818 (and integration commits 614152402, 428efdc61) -- FR-152's own numbers are a hand-miscount

FR-152 and both integration commit messages state: "41 such anchors exist across ten maps." Measured
directly against the committed tree with a tool, not by eye, using the pattern anchored to the real TOML
assignment (leading caret, "extract_line = ", against every map.toml file at the HEAD commit): 11 files
carry the field, summing to 30 total assignments. The same search without the leading-caret anchor (i.e.
counting every line that merely mentions the word, including each file's one doc-comment reference to
its own extract_line field) sums to 41 across the same 11 files -- i.e., the follow-up appears to have
conflated "occurrences of the word" with "anchors." A4's own report (its section 5) already said "11 map
files," correctly; the "ten" in FOLLOWUPS.md and both commit messages is the only place the wrong number
propagated.

**Not gating.** The underlying guarantee is functionally verified correct and unaffected: a fresh,
independent full-suite run (below) shows census_join::tests::the_committed_maps_are_covered_and_placed
-- the exact test that would red on a second uncorrected anchor -- passing against the real, fully
header-bearing extract corpus. This is a citation-accuracy defect inside a note, not a defect in the
code the note describes. Recorded because the note's own subject is "the hand-typed-number shape," and
it is itself one.

## Refuted premises

- **This task's own brief said the worktree is at 3149c393, the fully integrated tree.**
  False. HEAD on arrival was 376d114193b2db307292e8df6575016555fa943a -- the PLAN commit, seven
  commits (all six integrations plus the B1a widening) behind 3149c3936. The working directory
  additionally carried roughly fifteen uncommitted leftover directories/files (target-a1 through
  target-a5, target-s1 through target-s3, and all nine REPORT files plus the ADVICE file) from earlier
  wave-1/wave-2 agent dispatches that had reused this same physical worktree path without cleanup between
  sessions. Verified every stray file was byte-identical to what is already committed at 3149c3936
  (diffed each against the committed blob, clean), then corrected by moving this worktree's own local,
  unpushed branch pointer to 3149c3936 and syncing the working tree to match -- not a shared-tree
  operation, and no content was at risk since it was already proven identical to committed history.
  Recorded per this arc's own standing note: stop and report rather than build on a premise you can
  disprove.

## Checked clean

1. Cross-patch interaction. census_join's absolute-line-index anchors (FR-152's subject) --
   functionally green against the real, header-bearing extract corpus: the test named
   the_committed_maps_are_covered_and_placed and the test named the_join_reds_on_every_planted_defect
   both PASS in the full run below. A2's compiler-held coupling (the controller's explicitly-unreproduced
   claim -- "E0004 on an ADDED revision"): verified by direct source reading rather than by planting a
   file, since this task's mechanics permit editing no file but this report. The schema function in
   crates/btctax-forms/src/line_set.rs (lines 141-194) is a match over every LineSet variant with ZERO
   wildcard arms, over an enum generated with no non_exhaustive attribute anywhere in build.rs or
   line_set.rs. Rust's match-exhaustiveness check is a deterministic compiler guarantee, not a runtime
   property that could vary by input, so a new line_set value producing a LineSet variant with no schema
   arm is certain to fail to compile, independent of what A2 pasted. A1's caption checker against A4's
   110 new GENERATED extract headers: running the xtask line-coverage binary reproduces the exact
   committed numbers (377 money lines across 18 forms, 429 map captions across 38 bundled maps, floor
   429, 2 pinned, 12 bundled years quoted from another) verbatim. Confirmed no map.toml file besides
   forms/2024/f1040s1.map.toml was touched anywhere in the whole integration range (one file changed,
   plus 3 minus 3, over every map.toml file in the tree).

2. False greens among the new kills. Reset to the true integrated commit and ran the full suite from
   scratch in an independent, freshly-built worktree (target dir target-s5): the workspace nextest run
   gave 3646 tests run, 3640 passed, 6 failed, 12 skipped -- the 6 failures are exactly the documented
   form_delta PDF-absence set, nothing else. Per this task's brief, tried the forms-fetch restore
   subcommand (real network, no local mirror): 125 document(s) restored, 0 failed; re-ran the full suite:
   3646 tests run, 3646 passed, 0 failed, 12 skipped -- genuinely 0 red, not just the documented set
   explained away. This also independently confirms S3/FR-147's fix works under a REAL target-dir
   override (mine, not S3's or A4's self-report): the write-hook test passed on the first run, before any
   PDF restore, with the override active throughout. Clippy across the whole workspace with all targets
   and all features and warnings-as-errors gave exit 0. The formatting check gave exit 0. The forms
   extract check gave 125 of 126 unresolved before the restore (each failure naming the exact restore
   fix), 126 of 126 byte-for-byte after -- matches the coordinator's own main-tree number exactly. The
   cite-check command gave 37 of 38 archived, 1 excused, 0 unaccounted -- exact match.

3. Claims taken on report, not verified. A2's "E0004 on an added revision" -- addressed under item 1
   above via static analysis (deterministic compiler guarantee), the one gap the controller explicitly
   admitted not reproducing. No other "not reproduced" admission appears in any of the six integration
   commit messages (read in full: the A4, A1, A2, A3, A5, and S1-S4-batch integration commits, plus the
   B1a widening commit).
4. The unowned residue (FR-156). Confirmed both named files still carry exactly the described
   hardcoded counts and neither was touched by any of the nine diffs: the constant named
   BUNDLED_FORMS_PER_YEAR in crates/btctax-forms/tests/supported_years_cross_product.rs line 147, which
   reads as a slice of (2024, 20) and (2025, 18); and the literal year list [2023, 2025, 2026] at line
   200 of crates/btctax-forms/tests/map_pdf_conformance.rs, inside the test that checks both Form 6251
   and Form8995A refuse an unmapped year (the "8995-A refusal loop"). Did not attempt a full repo-wide
   sweep for unrelated hand-typed-list instances beyond what the nine reports/FOLLOWUPS already named --
   that is a pre-existing condition this round's patches did not create, and a fresh audit of it is out
   of this round's scope.

## Out of scope

- Resolving FR-152/153/155/156/157 themselves (each has an explicit owning phase -- the port machine, or
  ownerless residue -- and none is NOW-owned).
- A full repo-wide audit for every "list beside a growing set" pattern; scoped to what the nine agents'
  diffs and reports actually touched or surfaced.
- Executing A2's added-revision plant literally (would require editing a tracked map.toml/pdf pair,
  which this task's mechanics reserve to no file but this report); resolved instead by static analysis,
  noted explicitly above rather than silently substituted.
- Re-litigating FR-135's "half closed" status, FR-153/FR-155's deliberately-left residue, or FR-154's
  owner ruling -- all three are already correctly and honestly recorded in FOLLOWUPS.md and were not
  re-derived here.
