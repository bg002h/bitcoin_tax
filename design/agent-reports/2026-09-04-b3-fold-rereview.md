# Re-review of the B3 fold `f6d91e59` — the last gate before merge

Reviewer: one Opus agent, no subagents. Scope: `f6d91e59` in the context of
`main..HEAD` (10 commits). Working tree left byte-identical to `54074e06`
(`git status` empty; `make check` 2766/12/0 re-run after every mutation was
reverted).

---

## VERDICT

**0 Critical / 3 Important / 2 Minor / 2 Nit — DO NOT MERGE.**

All three Importants are in the fold itself, and two of them are the shapes the
brief predicted: **a kill test that passes against the very breakage it holds**
(I-1, demonstrated by mutation, not argued), and **a refusal that does not
refuse for 41% of the manifest** (I-2, demonstrated by probe test). The third
is the fold failing to actually perform the correction its own commit message
says it performed (I-3).

None of the three is a regression against `main`. The branch is strictly better
than the pre-fold state, and every Q3 wording claim I checked is finally
accurate. The three Importants are all small, local fixes.

---

## FINDINGS

### I-1 (Important) — the B1 kill test PASSES while `regen` destroys the manifest

`crates/xtask/src/authority_manifest.rs:879-913` (the test),
`:588-620` (the guard), `:685` (the write)

The test asserts that `regen` returns `Err`. The guarantee is that the
**manifest survives**. Those are not the same assertion, and the gap is not
theoretical: moving the guard from the top of `regen` to just above
`Ok(entries.len())` — i.e. below `std::fs::write(manifest_path(root), …)` — is
an ordinary refactor, and under it `regen` **writes the destroyed manifest and
then refuses**, and

    regen_refuses_to_delete_a_document_whose_binary_is_missing ... PASS

along with the other 7 `authority_manifest` tests. Measured, not reasoned —
see WHAT I VERIFIED, mutation M1. A probe assertion added at the same point
printed `PROBE: manifest exists after refusal = true`.

This is the brief's named blocking class ("a test reporting a false PASS") and
it is the same shape as F4: the instrument was seen red on *one* planted defect
(the refusal neutered) and was never watched against the other obvious one.
B1's letter is satisfied; its point is not.

**Smallest fix.** Seed a manifest before the failing call and assert it is
unchanged. Minimal version, verified green on the pristine tree and red on M1:

```rust
let mp = root.join("design/forms/MANIFEST.json");
assert!(!mp.exists(), "regen must not write the manifest before refusing");
```

Better (asserts preservation rather than absence): write a sentinel
`MANIFEST.json` into the tempdir before `regen(root)`, and `assert_eq!` its
bytes afterwards.

### I-2 (Important) — the refusal is blind to 42 of the 102 entries: documents with no note

`crates/xtask/src/authority_manifest.rs:540-560` (`notes_without_binaries`),
`:600` (the guard's predicate)

The brief asked directly whether "a document with no note at all" can still be
destroyed. **It can, and there are 42 of them** — the whole of
`legal/primary-sources/`, which by the (B) convention has no `.pdf.txt` notes
at all. Measured: of the 102 manifest entries, 60 have a sibling note and 42 do
not, and the 42 with no note are exactly the `committed` ones.

Demonstrated end to end with a probe test (M2): a tempdir holding one
note-backed form and one committed `26USC_s1.html` with no note regenerates to
2 entries; delete the `.html`; `notes_without_binaries` returns `[]`; `regen`
returns **`Ok(1)`** and the statute is gone from the manifest without a word.
That is the identical failure the guard exists to prevent, one storage class
over.

The guard is keyed on the wrong invariant. What the manifest needs is *"no path
currently in `MANIFEST.json` may disappear from a regen"*, which is a handful
of lines, needs no second tree walker, and strictly subsumes the note case.
What was built is *"every note has a binary"*, which happens to cover the 60
because notes and gitignoring coincide there.

**Honest reachability caveat.** The 60 are the ambient hazard — absent on every
fresh clone and in CI — and they *are* now covered. The 42 are committed, so
exposure needs a local deletion first, and
`every_manifest_entry_resolves_and_hashes_true` reds on that deletion *before*
any regen. So this is a narrower hazard than I-1. It is Important rather than
Minor because (a) the brief asked exactly this question, (b) the claim as
written is broader than the code — `regen`'s own comment says "REFUSE rather
than silently delete", and `FOLLOWUPS.md` FR-25 says "That half is now FIXED"
of the destruction hazard, then records the residue as *only* the note-sha
parse gap, and (c) the correct formulation is smaller than the one shipped.

**Smallest fix.** Before the walk, load the committed manifest (if it exists)
and refuse on `listed − collect_sources`, naming the dropped paths; keep the
note-specific "fetch them first" hint for the subset that has notes. Or, if the
owner prefers to keep the current scope, narrow the claims in `regen`'s comment
and FR-25 to "note-backed documents" and file the committed half.

### I-3 (Important) — the fold's own I-2 was not folded: the incorrect B1 seen-red record is still in the file

`crates/xtask/src/authority_manifest.rs:948-956` (the correction) and
`:957-959` (the uncorrected original, still present)

The commit message says the seen-red record was *"Corrected to
`2026/f8949--2026.pdf`"*. It was **appended to, not corrected**. `grep -c "Seen
red 2026-09-04 (B1)"` returns **2**, and the two paragraphs contradict each
other three lines apart:

- `:948` — red observed at `design/forms/2026/f8949--2026.pdf`; the 2024
  attempt *"collided with a real tracked note and destroyed it … **Do not
  reproduce it at that path.**"*
- `:957` — *"The same document was planted under a second path
  (`design/forms/2024/f8949--2024.pdf` …) and this test failed … Then reverted,
  and green."*

So the standing B1 provenance record still asserts, unqualified, that the red
was observed at the path the paragraph above it says destroyed a tracked file
(`c6c4a7dc`'s recorded incident), and still reads as an instruction to
reproduce it there. This is both a false record inside the harness whose value
the fold itself says "depends on its own defect record being accurate", and a
live hazard for the next person who tries to re-observe the kill.

**Smallest fix.** Delete `:957-959`.

### M-1 (Minor) — `regen`'s doc comment was captured by the new function

`crates/xtask/src/authority_manifest.rs:529-539` vs `:540`, `:588`

The fold inserted `notes_without_binaries` **between `regen`'s doc block and
`regen`**. `notes_without_binaries` now carries, as its rustdoc, `` /// `cargo
run -p xtask -- authority-manifest --regen` `` and *"Derived from the trees,
never hand-listed … Storage is read from `git check-ignore`"* — three
statements that are about `regen` and false about the function they now
document. `pub fn regen`, the destructive entry point, is left with **no doc
comment at all** (only inner `//` comments).

**Smallest fix.** Move the three new `///` lines (the `★★★ Every note whose
binary is absent …` paragraph) above `notes_without_binaries` and leave the
original block on `regen`.

### M-2 (Minor) — the refusal tells the maintainer to do something with no actuator, and the docs don't mention the new precondition

`crates/xtask/src/authority_manifest.rs:601-619`, `CONTINUITY.md:803`

The message says *"The (A) PDFs are gitignored — fetch them first (each note's
line 1 is the URL, and its sha256 is the check)"*. There is no tool that does
that: `legal/_scripts/` fetches the **(B)** tree only, there is no `make`
target, and `grep -rn design/forms --include=*.sh --include=Makefile
--include=*.py` finds nothing. On a fresh clone the refusal names 10 documents
and *"… and 50 more"*, and the maintainer's next move is 60 hand-issued
`curl`s. `CONTINUITY.md:803` still documents `--regen` as *"derives it from the
trees"* with no mention that it now refuses on any tree lacking the 60 PDFs.

Fail-closed is the right default and I am not asking for a `--force`; what is
missing is the actuator and one sentence of documentation. This is Minor, not
Important: the wrong outcome (a blocked regen) is far better than telling the
user nothing.

**Smallest fix.** One `legal/_scripts/`-style loop over
`design/forms/**/*.pdf.txt` (URL is line 1, sha256 is line `# sha256`), named in
the refusal message; plus a clause at `CONTINUITY.md:803`.

### N-1 (Nit) — `irs_stem`'s doc examples include two files the repo does not hold

`crates/xtask/src/archive_check.rs:48` — the examples are `f1040--2024.pdf`,
`i1040gi--2024.pdf.txt`, `f8949.pdf`, `p550.pdf`; the last two are not in the
tree (`f8949--2025.pdf` and `Pub550_Investment_Income_Expenses.pdf` are). I-3
established the standard *"these examples must be documents the repo actually
HOLDS"* and applied it to `human_readable_form` one function below; the sibling
shape was not swept. Nit rather than a repeat of I-3 because `irs_stem`'s prose
illustrates the IRS **naming convention** rather than asserting holdings, and
its witness (`f1040--2024.pdf`) *is* held.

### N-2 (Nit) — the two walkers disagree about what a note's stem is

`crates/xtask/src/authority_manifest.rs:546` uses
`note_rel.trim_end_matches(".txt")`, which strips **every** trailing `.txt`;
`collect_notes` at `:568` uses `strip_suffix(".txt")`, which strips one. For a
hypothetical `x.pdf.txt.txt` the two disagree about which path must exist.
Unreachable today (no such file exists). Use `strip_suffix` in both.

---

## THE THREE QUESTIONS, ANSWERED

### 1. Is the new refusal correct, and correctly scoped?

**Correct for what it checks; scoped too narrowly, and its ordering guarantee
is untested.**

- **Correct?** Yes, in isolation. `notes_without_binaries` mirrors
  `collect_sources` on the two things that matter (the `__pycache__`/`reviews`
  directory skips and the `EXTRACT_TREES` skip), and I confirmed it against the
  real tree with a line-by-line transcription of both walkers: 102 sources / 60
  notes / **0 orphans**, and the source set is exactly the 102 manifest paths
  in both directions. `harvested_urls` and `extract_for` both take `root`, so
  `regen` is hermetic and no helper falls back to `repo_root()`.
- **Too narrow — yes, measurably.** 42 of 102 entries have no note and are
  invisible to the guard (**I-2**), and the ordering of the guard relative to
  the write is not held by any test (**I-1**).
- **Too broad?** No. It cannot fire on the working tree (0 orphans, verified),
  and the positive half of the kill proves it does not block a legitimate
  regen. It *does* make `--regen` unusable on a fresh clone until all 60 PDFs
  are fetched, which is the correct fail-closed default; the message names the
  documents, names the mechanism (gitignored), names the remedy (line 1 is the
  URL, the sha256 is the check) and names the safe read-only alternative. What
  it does not have is a tool to carry out the remedy (**M-2**).
- **No fourth call site, and CI cannot reach it.** Confirmed independently:
  `regen(` resolves to `main.rs:88` plus the fold's two `tempfile` test calls,
  and `grep -rn "authority.manifest\|authority_manifest" .github/ Makefile
  scripts/` is **empty**. The pre-commit path is `make check` (nextest +
  clippy), which never regenerates.

### 2. Is the kill a real kill?

**Its positive control is real; its negative half is not strong enough, and I
broke it.**

- The positive control is genuine, not incidental: with the binary written, the
  same tree returns `Ok(1)`, so the guard cannot degenerate into blocking every
  regen. The test is hermetic — the tempdir is not a git repo (`git
  check-ignore` prints *"Stopping at filesystem boundary"*, so storage falls
  back to `Committed`), it never touches the real root, and `design/forms/`
  in the real tree was byte-identical before and after every run.
- It reds on the obvious mutations: neutering `notes_without_binaries`, or
  removing the `if !orphans.is_empty()` block, both fail at the first
  `assert_eq!` or the `expect_err`. The `err.contains("f8949--2025.pdf")` half
  genuinely holds the "name what you are protecting" requirement.
- **But it passes against the breakage that matters most** — the guard below
  the write (**I-1**). The test's subject is the return value; the guarantee's
  subject is the file. One assertion closes it.

### 3. Are the corrected claims now true?

**Yes — every one I could check. The third attempt is accurate.** This is the
one part of the fold I could not break.

| claim | verdict | how |
|---|---|---|
| `authority_manifest.rs:965-969` — "At the CURRENT pin of 0 only the rise arm is reachable (`usize`), so the `else` branch below is dormant" | **TRUE** | `DUPLICATE_SOURCE_GROUPS = 0`; the message closure runs only when `assert_eq!` fires, at which point `dups.len() != 0` and therefore `> 0`. The `else` is unreachable. |
| `:936` — "The sibling `the_archive_count_may_only_shrink` pins 3, where both directions really are live" | **TRUE** | `archive_check.rs:523-544` is `assert!(len <= 3)` **plus** `assert_eq!(len, 3)`; at 3 a fall is representable, so the first passes and the second reds with its own message. Not vacuous. |
| `:930-934` — "the fall arm was not decorative at 7 … replaced deliberately" | **TRUE and consistent** | At pin 7 a manifest wiped to the 42 committed entries yields 0 duplicate groups, so `assert_eq!(0, 7)` reds via the fall direction; at 0 it cannot. (The I-1 reproduction itself is out of scope per the brief.) |
| `archive_check.rs:167-173` (N-2) — "the difference is the PIN, not taste" | **TRUE** | Same evidence as above. |
| `HARNESS.md` (M-1) — "the tickle fired twice and reset TWICE" | **TRUE** | The RESET LOG holds exactly two rows: 2026-08-13→08-28 (decided 08-20) and 2026-08-28→09-11 (decided 09-04). |
| `design/forms/README.md` (M-2) — "**60** documents, all in `design/forms/`; `design/amt-form6251/` holds **no** notes" | **TRUE** | 60 `*.pdf.txt` notes under `design/forms/`; `find design/amt-form6251 -name '*.pdf' -o -name '*.pdf.txt'` → **0**. |
| `CONTINUITY.md` (M-3) — branch in flight, `main` still `945d1ac2` | **TRUE** | `git branch --show-current` → `chore/archive-reconciliation`; `git rev-parse --short main` → `945d1ac2`. |
| `CONTINUITY.md:429` (N-1) — "See ⬜ WHAT IS OPEN NOW row 5, and §0 step ③" | **TRUE** | Row 5 (`CONTINUITY.md:487`) is the archive review-by item; §0 step ③ is at `:527`. Both resolve. |
| `archive_check.rs:211-213` (N-3) — "ROW 2 IS DISCHARGED … future tense because it was written before the work" | **TRUE** | Row 2 is future-tense; the work landed 2026-09-04. |
| `archive_check.rs:88-93` / `:123-125` (I-3) — the `human-readable-form` witness and examples | **TRUE** | `Form_1099-DA.pdf` and `Instructions_1099-DA.pdf` both exist in `legal/primary-sources/irs-forms/`. All four `SHAPES` witnesses resolve to real files. |
| **FR-25 re-scoping** — `verify()`'s `Storage::Note` arm "checks only that the note EXISTS" | **TRUE** | `authority_manifest.rs:251-257`: the `Note` arm's only test is `note.is_file()`. |
| **FR-25 re-scoping** — the note-sha fallback "has never once fired" | **TRUE** | Measured over all 60 notes: **0** contain `sha256:` and **0** contain a bare 64-hex line. Every real note writes `# sha256  <hash>`, which matches neither branch. |
| **FR-25 re-scoping** — the originally-proposed fix is non-hermetic | **TRUE, and now doubly so** | A temp-regen-and-compare test on a tree lacking the PDFs would not merely differ, it would now hit the refusal and `Err`. |

**FR-25's re-scoping is correct** as far as it goes. Its one omission is that
the residue it records is only the note-sha/`verify()` gap; the committed-tree
blind spot (**I-2**) is not recorded anywhere, and its "That half is now FIXED"
reads as covering the whole destruction hazard.

The one wording I looked at hardest for a fourth error — the untouched RETIRED
block at `archive_check.rs:185-191`, which still calls the pin-0 guard
*"strictly stronger"* than the date it replaced — is **not** contradicted by
I-1. It claims only that the pin "reds on any duplicate the instant one
appears", which is the rise direction, and that is live.

---

## WHAT I VERIFIED AND HOW

Every number below is a command's output, not a recollection.

**Baseline, after all mutations were reverted** (proves the restore was
byte-exact and the tree ships green):

```
$ make check                 → Summary [19.771s] 2766 tests run: 2766 passed, 12 skipped
$ cargo fmt --all --check    → fmt clean
$ cargo run -q -p xtask -- archive-check
    archive-check: no primary source outside the 5 accounted-for tree(s)
    archive-check: 3 accounted-for archive(s) — hybrid, decided 2026-07-30 …
$ cargo run -q -p xtask -- authority-manifest
    authority-manifest: 102 entries — 42 committed, 60 note-only
    authority-manifest: 0 document(s) archived under more than one path (pinned 0 …)
    authority-manifest: OK — every entry resolves and every source is listed
$ git status --short          → (empty)
$ sha256sum crates/xtask/src/authority_manifest.rs
    b097597a4092fb5154cb827d674be6686a02c1fb28c2f418d0f97c952d8e557d   (== the pre-mutation backup)
$ sha256sum design/forms/MANIFEST.json
    c0e5b2e551deccdb17e998cdfcfd37ef0c7fee16197a4deef992b636566cb62d   (unchanged throughout)
```

**Mutation M1 — the guard moved below the manifest write (the I-1 evidence).**
Backed up with `cp` to the scratchpad, edited in place, restored with `cp`,
never `git checkout --`.

```
$ cargo nextest run -p xtask -E 'test(authority_manifest)'
    PASS (6/8) authority_manifest::tests::regen_refuses_to_delete_a_document_whose_binary_is_missing
    Summary 8 tests run: 8 passed, 60 skipped
```

With a temporary probe assertion at the same point, on the *same* mutation:

```
    PROBE: manifest exists after refusal = true
    panicked at …:908: PROBE: regen WROTE the manifest before refusing
    FAIL regen_refuses_to_delete_a_document_whose_binary_is_missing
```

and the identical probe on the **pristine** guard:

```
    test authority_manifest::tests::regen_refuses… ok
```

— so `assert!(!mp.exists())` is a valid, minimal strengthening: green on
correct code, red on the defect.

**Probe M2 — the no-note blind spot (the I-2 evidence).** A temporary test
built a tempdir with `design/forms/2025/f8949--2025.pdf{,.txt}` and
`legal/primary-sources/statute-irc/26USC_s1.html` (no note):

```
initial regen                       → Ok(2)
rm 26USC_s1.html
notes_without_binaries(root)        → []          (the guard sees nothing)
regen(root)                         → Ok(1)       (no refusal; the statute is gone)
```

**Real-tree measurements** (line-by-line transcription of `collect_sources`,
`collect_notes`, `classify` and all four `SHAPES`, run over the repo; validated
by reproducing the shipped numbers exactly before being trusted):

```
collect_sources                                → 102
collect_notes                                  →  60
notes_without_binaries                         →   0
manifest entries                               → 102
in manifest but not collected                  → []
collected but not in manifest                  → []
manifest entries with NO sibling note          →  42   ← the I-2 blind spot
manifest storage counter                       → {'note': 60, 'committed': 42}
notes containing "sha256:"                     →   0
notes containing a bare 64-hex line            →   0
```

**Structural checks**: `grep -c "Seen red 2026-09-04 (B1)"` → **2** (and → 1 at
the parent `1ab79e2e`, at line 800); `regen(` → `main.rs:88` + two test call
sites; `grep -rn authority.manifest .github/ Makefile scripts/` → empty;
`legal/SHA256SUMS` is 42 lines and is wired into **nothing** (no Makefile, CI,
script or test reference); all four `SHAPES` witnesses and all of
`human_readable_form`/`usc_or_cfr`/`irs_guidance`'s doc examples resolve to real
files, while `irs_stem`'s `f8949.pdf` and `p550.pdf` do not.

---

## WHAT I COULD NOT CHECK

- **The I-1 regression reproduction itself** (102 → 42 with the 60 PDFs absent,
  and the pin-7 red). The brief declares it settled and out of scope, and I did
  not re-run it — reproducing it means either deleting 60 gitignored PDFs from
  the working tree or building a 20 GB copy of the repo, and the first is the
  destructive act this whole branch exists to prevent. Everything I *did*
  measure is consistent with it.
- **`regen`'s behaviour on the real repo root.** I never ran `cargo run -p
  xtask -- authority-manifest --regen`; it writes the real `MANIFEST.json` and
  the brief forbids modifying tracked files. All `regen` evidence above is from
  `tempfile` roots, which is how the shipped test exercises it too.
- **Whether the 60 IRS URLs still resolve.** No network. So I cannot say
  whether the remedy the refusal prescribes ("fetch them first") actually
  succeeds today, only that no tool in the repo attempts it.
- **The `orphans.len() > 10` truncation branch** of the refusal message is not
  covered by any test; I read it and it is arithmetically right (`… and {n-10}
  more`), but it has never been executed.
- **Rustdoc rendering** of the M-1 misattribution — I read the source
  positions rather than building `cargo doc`.
