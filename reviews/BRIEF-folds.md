# Review brief — THE TWO FOLDS

**Range: `5ab1258..HEAD` — exactly two commits.** `git log --oneline 5ab1258..HEAD`,
`git diff 5ab1258..HEAD`. This is deliberately narrow. Do not widen it.

## The ONE question

**What did these two fixes LEAK?**

Not "are they correct in isolation" — they were each written against a specific defect and each is
pinned by mutation-verified tests. The question is what *else* consumed the value, path, or invariant
the fix changed.

★★★ **This question has now paid twice in a row on this branch, and that is why you exist:**

- r3 fixed the §G-21 gate's *predicate* (it refused standard-deduction filers who claim no §170
  deduction — a false block). Correct about the **year**. But `apply_170b` runs unconditionally so
  the carryover still aged at full FMV, and `--write-carryover` laundered it into next year stamped
  `Computed`, where nothing can catch it. **The year stopped being wrong and the carryover became
  wrong instead.**
- r3 fixed `Mfs63fSpouseBoxesForgone`'s *firing condition*. Correct. But the **message text** still
  asserted the pre-fix world, and told the filer to hand-claim boxes btctax had already claimed.

Both times: the fix was right about the thing it aimed at, and something adjacent inherited the
defect. Find the third one, or establish there isn't one.

## What is in the range

`5ab1258` (the r3 fold) and `5ebd3cc` (the pre-merge fold). Between them they:

1. **Moved** the §G-21 refusal `screen_compute_dependent` → `screen_absolute`, re-keyed it from a
   ledger aggregate to `ar.deduction_is_itemized`, and added `state`/`year` to `screen_absolute`.
2. **Split** `spouse_63f_boxes_count` into two predicates — `spouse_63f_boxes_count` (record AND
   status → decides the DEDUCTION) and `spouse_63f_status_permits` (status only → drives the
   ADVISORIES, because an absent spouse record is itself a forgone box).
3. **Narrowed** `Mfs63fSpouseBoxesForgone` to stay silent when a condition is answered adversely, and
   rewrote its message.
4. **Added** a vouch-for gate in `apply_carryover_writeback` (in core; `state`/`year`/`ri` added to
   its signature) refusing to persist a charitable carryover when a restriction is declared, or due
   and unanswered. **`force` does not bypass it.**
5. **Deleted** the `capital_loss_carryforward_in_provenance = Computed` stamp.
6. **Added** a `section == Form8283Section::B` guard to the 8283 5a/5b/5c writer.
7. Added three export-path refusal tests, and swept ~20 stale doc/census-reason sites.

## Specific leak hypotheses to test (not an exhaustive list — find your own)

- **The vouch-for gate refuses the WHOLE write-back.** A year with a restricted donation now cannot
  persist its **QBI/REIT** carryovers either — those have nothing to do with donations. Is that a new
  false block? What is the filer's exit? Is there a partial-write or half-applied state?
- **`screen_absolute` runs later than `screen_compute_dependent` did.** Anything that assembles a
  return, or reads `AbsoluteReturn`, or emits *anything*, between where the old gate fired and where
  the new one does, now sees a state the old gate would have refused. Enumerate those consumers.
- **`spouse_63f_status_permits` ignores the spouse record.** Every consumer that then dereferences the
  spouse must handle `None`. Is there a path where "status permits" is used to *grant* rather than to
  *advise*?
- **The 8283 Section-B guard.** Can a return that SHOULD print 5a/5b/5c now miss them?
- **The narrowed advisory.** Is there a filer who genuinely forgoes a recoverable box and now hears
  nothing? (§3.4 permits a conservative omission only if the filer is TOLD.)
- **The deleted provenance stamp.** Anything that read `Computed` on that field and now behaves
  differently.

## Settled facts — do NOT re-derive, re-file, or relitigate

1. The three findings folded in `5ebd3cc` are FIXED and mutation-verified. Do not re-file them.
2. **§G-19a** (§1411 display) — open owner decision. **§G-12** (no 8275-R) — blocked on an
   unobtainable asset. **§G-22** — knowingly partial. **§G-20b**, **§G-23** — filed, direction safe.
3. `.pii-patterns` is absent; push and publish are BLOCKED. Out of scope.
4. Neither oracle validates a value they are HANDED (Form 8995 line 3). Known standing condition.
5. All five gates pass at HEAD: `make check` (2541 tests), `cargo fmt --all --check`,
   `cargo +1.88 check --workspace --locked`, `xtask check-isolation`, `scripts/pii-scan-generic.sh`.
   TY2024 golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47`.
6. **Prior coverage — do not redo.** The whole branch was reviewed by five lenses immediately before
   this range; `reviews/branch-premerge-workflow.md` has a COVERAGE section listing what was verified
   sound. Read it and skip those. Findings that section already cleared are not new findings.

## The authority

`/scratch/code/bitcoin_tax/CLAUDE.md` — transcribe-don't-paraphrase; blank-is-the-normal-case (assert
PROVENANCE, never non-blankness); an entry is TESTIMONY. Extracted form text is in
`design/forms/extract/`. Quote the form when you claim btctax contradicts it.

## Output format

First line: `VERDICT: <clean | fix-before-merge: X>`

Then findings, most severe first, each fenced:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: concrete inputs/state → wrong figure on a signed return, or wrongly-blocked filer.
EVIDENCE: quote the code AND the authority.
```

**Critical** = wrong tax figure / data loss / unmet guarantee. **Important** = real defect, missing
case, unsound assumption. Do not inflate. **A clean result is the expected outcome** — this range is
two commits of reviewed-and-mutation-verified fixes, and "these leaked nothing" is a useful finding.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.
