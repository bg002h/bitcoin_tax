# BRIEF — check the fold that answered the fold check

```
git show 87a605e9            # the fold check (all-closed, then 2 Minor + 6 Nit in the fold itself)
git diff 87a605e9..9be921e2  # THE FOLD — the only text under examination
```

**Not a fresh review.** Seven passes have run on this branch. This checks one diff, and it is the
only text on the branch nobody has read.

## The ONE question

For each of the eight findings: **did the fold close it, and did the response introduce a new defect?**

## ★★★ PRESS HARDEST HERE — three places where the fix could be worse than the finding

1. **The false-citation fix asserts a NEW claim. Is it true?** The old comment claimed box-12
   KEPT-ness was "asserted directly by value" in the survival test — false, and that was the finding.
   The replacement claims the **derived axis** holds it: *"if scrub ever began replacing the code,
   `replaced_paths` would gain `w2s[].box12[].code` and the matrix's 'every derived path has a row'
   check reds."* **Verify that end to end.** If it is wrong, I have replaced a false citation with a
   more confident false citation, which is strictly worse.

2. **The inode kill-test.** It asserts `ino_before != ino_after` as evidence the file was created
   fresh rather than written through. Is that sound? Consider whether a filesystem can reuse a
   just-freed inode, whether tmpfs behaves differently from the CI filesystem, and whether the test
   can therefore flake — a flaky guard on a security fix is worse than the unheld guard it replaced.

3. **`synthetic_malformed_ein` gained `% 1_000_000` and a `debug_assert!`.** The modulo was added to
   stop the pad-not-truncate overflow. Does it now break **distinctness**, which is the property
   §3.1's CRITICAL turns on? `EinMap` passes `n = self.0.len()`, so two distinct real EINs must never
   map to the same stand-in. Also: `debug_assert!` is compiled out in release — does that matter for
   what it claims to guarantee?

## Also check

- `CliError::ScrubOutput` — is it used consistently, does anything still reach `Usage`/`BadConfigValue`
  wrongly on this path, and does the new variant's doc claim anything false?
- The two "collapsed string" fixes — read the EMITTED literal, not the source. Are they actually fixed,
  and did the edit introduce a new one?
- Did any edit strand a statement behind it, the way the previous fold did twice?

## Already machine-checked — do not re-verify

2666 tests pass; `cargo fmt --all --check` and `clippy -D warnings` clean at `9be921e2`. Deleting the
`remove_file` line reds the mode test (mutation-verified after the fix).

## FORBIDDEN

Re-auditing anything outside this diff. Re-reporting settled items. Style. Manufacturing a finding —
**"all closed, nothing new" is the expected and useful result.**

## OUTPUT FORMAT

```
VERDICT: <all-closed | needs-changes>

Minor-1 (false citation) / Minor-2 (inode kill-test) / the six Nits: <closed | partially | reopened |
new-defect> — one line of evidence each

IS THE NEW BOX-12 CLAIM TRUE? <yes/no, with the mechanism traced end to end>
IS THE INODE ASSERTION SOUND? <yes/no, and can it flake>
DOES synthetic_malformed_ein STILL PRODUCE DISTINCT VALUES? <yes/no>

NEW DEFECTS: <none, or list with severity, where, failure vector>

WHAT WOULD MAKE THIS CHECK WRONG: <one sentence>
```
