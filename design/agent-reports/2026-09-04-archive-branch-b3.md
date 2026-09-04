# B3 whole-branch review — `chore/archive-reconciliation`, scoped `main..HEAD`

One agent, opus, no subagents. Scope: 8 commits (`dca6ef25..35ab4ab3`), 35 files,
+1132/−3544. Target: **interaction between commits**, not correctness inside any one.
No tracked file was modified; the one mutation planted is recorded and verified reverted
in **WHAT I VERIFIED AND HOW**.

---

## VERDICT

**0 Critical / 3 Important / 3 Minor / 3 Nit — DO NOT MERGE.**

I-1 alone blocks: the branch's 7 → 0 pin change removed the only assertion that reds when a
**documented command silently deletes 60 of the manifest's 102 entries**, and I reproduced
that end state with the whole xtask suite green and both gates printing OK. I-2 and I-3 are
the canonical B3 shape — a fix that already exists on this branch and was not carried across.

None of the three is a re-litigation of the retirement decision, which I did not review. The
decision is sound on the evidence I checked: every factual claim in the RESET LOG row and in
`c6c4a7dc`'s message that I could measure came back exact, including `905,833` bytes to the
byte and all five retired hashes matching their surviving notes.

---

## FINDINGS

### I-1 (Important) — at a pin of 0 the "two-sided" assertion is one-sided, and 7 → 0 disarmed the only tripwire on a manifest-destroying regen

`crates/xtask/src/authority_manifest.rs:144` (the pin), `:796` (the claim), `:807-820` (the assertion)

**The claim.** `:796` — *"The `assert_eq!` predates the change and already reds on a rise **and**
on a fall the pin has not tracked, so the guarantee was **never weakened at any pin**."*
`:807` — *"ONE two-sided assertion, deliberately … Both are failures."*

**Why it is wrong.** `dups.len()` is `usize`. At `DUPLICATE_SOURCE_GROUPS = 0` a fall is
unrepresentable, so the `else` arm at `:818` (*"Duplicates were RETIRED"*) is unreachable and
the assertion is one-sided. This is the **same `usize::MIN` vacuity** the branch had just
deleted from `assert!(dups.len() <= PIN)` — removed from the `assert!` and reintroduced in the
message of its replacement, then asserted as live in the doc comment. The sibling
`the_archive_count_may_only_shrink` (`archive_check.rs:501`) pins **3**, where both directions
really are reachable; that contrast is the proof.

**Why it matters, measured rather than argued.** The fall arm was not decorative at 7 — it was
the only thing standing between the repo and a silent manifest wipe:

- `regen` builds the entry list by walking the **filesystem** for binaries
  (`collect_sources`, `:294`). The 60 (A) documents are gitignored, so on any tree where they
  have not been fetched — a fresh clone, CI — they are simply not collected.
- The "trust the note" fallback that should rescue this (`:563-578`) parses `sha256:` or a bare
  64-hex line. **Zero of the 60 notes match either form** (they are `# sha256  <hash>`), so it
  has never worked; and it is unreachable anyway, because a dropped document never becomes an
  entry.
- `verify()`'s `Storage::Note` arm (`:252`) checks only that the note **exists**. Nothing ever
  compares a note-storage entry's `sha256` or `url` to the note that is supposed to be its
  source of truth.

I simulated a fresh clone (60 gitignored PDFs moved aside) and ran the documented
`cargo run -p xtask -- authority-manifest --regen`:

```
authority-manifest: regenerated 42 entries          # was 102 — all 60 (A) entries dropped
cargo test -p xtask   → 66 passed; 0 failed; 1 ignored
xtask authority-manifest → "OK — every entry resolves and every source is listed"   exit 0
xtask archive-check      → green                                                    exit 0
```

The legal-defense index for 60 documents is deleted and **every instrument says OK**. On
`main` the identical manifest **reds**: main pins 7, the regenerated manifest has 0 duplicate
groups, so `assert_eq!(0, 7)` fires. The branch converted a live tripwire into a vacuous one,
in the same commit that describes the replacement as *"a STRONGER guard"* (`:134`).

**Smallest fix** (not a new checker — a refusal in code that already exists): make `regen`
**error** instead of silently omitting, when a `<path>.txt` note exists whose binary is absent
and whose sha cannot be recovered. And correct `:796`/`:807` to say the fall arm is dormant at
0 rather than live.

**This is the fourth gap the brief asked for, and FR-25 is mis-scoped.** FR-25 is filed in the
wrong direction — as *staleness* (the committed manifest drifting behind a regen), when the
live hazard is the opposite: **a fresh regen destroys the committed manifest**. Its proposed
fix, *"regen into a temp dir, assert byte-equality with the committed file,"* is non-hermetic:
it reds on every machine that has not fetched all 60 gitignored PDFs, i.e. CI and every fresh
clone, and the obvious way to make it pass is to commit the 42-entry manifest. That is exactly
the *"a golden cannot validate its own regeneration"* trap FR-25 itself cites. FR-23 and FR-24
are correctly scoped.

### I-2 (Important) — the standing B1 seen-red record names the wrong path, and reproducing it as written destroys a tracked note

`crates/xtask/src/authority_manifest.rs:800-802`

The doc comment: *"The same document was planted under a second path
(`design/forms/2024/f8949--2024.pdf` copied from the 2025 note) and **this test failed** with
`duplicate archived documents: 1, pinned 0`."*

`c6c4a7dc`'s message records something different — the kill was at
`design/forms/2026/f8949--2026.pdf`, quoted with its output, and a **first attempt**
*"collided with a real tracked note and destroyed it plus its gitignored PDF … restored — the
note from git, the PDF re-fetched and hash-verified (dcd2d7ff…, 129683 bytes)."*

Measured: `design/forms/2024/f8949--2024.pdf.txt` is a real tracked note recording
`sha256 dcd2d7ff6833485038aa34946d9a91b7d5adf639677116b57f143677a85a3b51`, `bytes 129683`.
So the doc comment names **the path of the attempt that PASSED and proved nothing**, and
reports it as the run that reddened. B1's whole reviewable question is *"which test reds?"* —
its standing record must be exact, and this branch itself elevated *"this harness is only worth
its cost if its own record of what went wrong is accurate"* to a stated principle one commit
later. A maintainer reproducing the kill as written re-destroys that note and its gitignored
PDF, and gets a green test.

The record also covers only the rise; the commit message records a fall kill too (pin raised to
1 with no duplicates present), which is the direction I-1 shows is now dormant.

**Smallest fix:** `2024/f8949--2024.pdf` → `2026/f8949--2026.pdf` in the doc comment; add the
fall line.

### I-3 (Important) — the `human-readable-form` shape's witness is a file this branch deleted

`crates/xtask/src/archive_check.rs:116` (the witness), `:36` (its contract), `:83` (the doc's examples)

`Shape.witness`'s contract at `:36` is explicit: *"A real filename from one of the archives, so
the shape is anchored to observed data and the test below can prove the shape still fires."*
`witness: "Form_8949.pdf"` is no longer a filename from any archive — `c6c4a7dc` deleted it.
Measured, the only surviving filenames matching `human_readable_form` anywhere in the repo are
`Form_1099-DA.pdf` and `Instructions_1099-DA.pdf`, and **FR-23 schedules both for removal**, at
which point the shape has no observed subject at all. `every_shape_fires_on_its_witness`
(`:392`), whose failure message is *"the detector has gone blind"*, then proves only that a
string literal matches a string matcher. The doc at `:83` compounds it: all three filenames it
gives as the live examples of (B)'s naming convention (`Form_8949.pdf`,
`Instructions_Schedule_D.pdf`, `Schedule_D_1040.pdf`) are deleted.

★ **The fix already exists on this branch.** `36c6d12b` fixed the identical defect one file
over — `kind_is_derived_from_the_path` swapped `Instructions_8949.pdf` →
`Instructions_1099-DA.pdf` with a comment saying *"Must be a path that still EXISTS."* Nobody
carried it to `archive_check.rs`, because the reviewer's window was `c6c4a7dc` and the fold was
never re-reviewed.

**Judgment call, stated openly:** the prior review rated its instance a **Nit**. I rate this
Important because the two are not the same — that one was an example list inside a test, this
one is a **declared anchor** whose field contract ("a real filename from one of the archives")
is now false, in the instrument that exists to detect blindness.

**Smallest fix:** `witness: "Form_1099-DA.pdf"`; update the three examples at `:83`.

### M-1 (Minor) — `design/HARNESS.md:169` under-reports the reset count

*"the tickle fired twice, then discharged. It **reset once** (2026-08-13 → 2026-08-28)."*
The RESET LOG records **two** resets: row 2 moved 2026-08-28 → 2026-09-11 in `dca6ef25`, four
commits before HARNESS.md was written. Pure seam — `dca6ef25` is outside the only review
window, so nobody held both commits. It matters because the log's own standard is *"Two entries
here is a decision; five is the gate being routed around"*: under-counting resets is the one
direction that standard cannot tolerate. **Fix:** "reset twice", and note the second firing was
answered first by a date and then by the work.

### M-2 (Minor) — `design/forms/README.md:54` carries the stale count the branch fixed elsewhere

*"**archived** — done: 66 documents recorded (57 here + the 9 in the older
`design/amt-form6251/`)."* Measured: `design/forms/*/*.pdf.txt` = **60**;
`design/amt-form6251/` holds **0** notes (retired 2026-07-30). ★ Seam: `0a6532d8` corrected the
identical stale `57` in `CONTINUITY.md` on this branch — calling it out as *"ALREADY WRONG
BEFORE THIS BRANCH"* — and did not sweep `design/forms/README.md`, which `36c6d12b` had edited
two commits earlier. **Fix:** "60 documents recorded, all in `design/forms/`".

### M-3 (Minor) — `CONTINUITY.md`'s RESUME POINT is false the moment this merges

`CONTINUITY.md:3` *"Last updated: **2026-08-30**"*, `:7` *"RESUME POINT — … **Nothing is in
flight**"*, `:11` *"**`main` is `945d1ac2`, local and remote IN SYNC**"*. All three are wrong
after the merge, and the same file already carries 2026-09-04 content at `:424`, `:483`, `:522`
and `:832` — so it disagrees with itself, which is precisely the Minor the prior review filed
against this file and which `36c6d12b` folded. This is the repo's designated "read this file
first" artifact, so it is the first thing the next session sees. **Fix:** the date, plus one
line in the RESUME POINT.

### N-1 (Nit) — `CONTINUITY.md:424` cross-reference points at the wrong step

*"See RESUME POINT §0 step 5."* §0's step ⑤ is **the label reader**; the archive item is §0 step
③. The intended target appears to be row 5 of "⬜ WHAT IS OPEN NOW", which is not §0 and not in
the RESUME POINT.

### N-2 (Nit) — the file now documents two opposite assertion conventions

`archive_check.rs:501-516` keeps `assert!(len <= 3)` **plus** `assert_eq!(len, 3)` — the shape
`authority_manifest.rs:807` replaces with *"ONE two-sided assertion, deliberately"*, three
hunks away in the same branch. Not a defect (the pair routes two different messages, and at a
pin of 3 neither half is vacuous), but a reader now finds both conventions with rationales that
contradict each other.

### N-3 (Nit) — RESET LOG row 2 is future-tense in what is now a permanent record

`archive_check.rs:198`: *"Both residual groups resolve …"*, *"when it lands … are deleted"*,
*"There is no #3."* Everything it promises did happen (see Q1 below), so it is accurate, but it
reads as a prediction in the block that is explicitly *"the only at-a-glance record"*. The
adjacent past-tense line at `:201` keeps a reader from being misled, which is why this is a Nit
and not a Minor.

---

## THE SEAM — what a single-commit review could not have found

**One pattern, three times: the fix existed on this branch and nobody carried it across.**
That is the exact B3 precedent this repo learned from, reproduced inside one week.

| # | fixed here | identical defect left standing | why the window missed it |
|---|---|---|---|
| I-3 | `36c6d12b` swapped a deleted path for a surviving one in `authority_manifest.rs:775`, with a comment saying why | `archive_check.rs:116` still names the deleted `Form_8949.pdf` as a shape's anchor | the fold is **after** the only review; the reviewer saw the pre-fold file |
| M-2 | `0a6532d8` corrected the stale `57` in `CONTINUITY.md`, flagging it as long-wrong | `design/forms/README.md:54` still says `57 here` | `README.md` was last edited in `36c6d12b`, one commit earlier, and never re-read |
| I-1 | the branch deleted a vacuous `assert!` for being vacuous at 0 | its replacement's `else` arm is vacuous at 0 for the same reason | the correction landed in `36c6d12b`, **after** the review closed |

**Findings that require holding two commits at once, which no reviewer on this branch did:**

- **I-1** needs `dca6ef25`'s pin-at-7 world beside `c6c4a7dc`'s pin-at-0 world. Inside
  `c6c4a7dc` the new assertion looks strictly better than what it replaced; only against the
  earlier state is it visible that a live tripwire went dormant.
- **I-2** needs `c6c4a7dc`'s **commit message** beside the doc comment it wrote in the same
  commit, plus the tracked note's own recorded hash to break the tie. The message and the code
  disagree, and only one of them is the reviewable artifact.
- **M-1** needs `dca6ef25` (the second reset) beside `c6c4a7dc` (HARNESS.md's "reset once").
  `dca6ef25` was never in any review window.
- **M-3** needs the branch's existence beside the file it did not update.

**And the shape of the miss is the one the harness names.** All three Importants are *green
instruments*: an assertion that cannot fire in one direction, a seen-red record that points at
a run which never went red, and a witness anchored to a document the repo no longer holds. Each
reports success; none is currently discriminating what it claims to discriminate.

---

## THE FIVE OPEN QUESTIONS, ANSWERED

**Q1 — does the branch contradict itself across commits?** No. RESET LOG row 2 is an accurate
historical record; I checked each of its claims and every one holds (see the verification
table). The only defect is tense — N-3. I also checked attribution, since the log's value is
its `who` column: `dca6ef25` (14:25:54) landed 40 seconds before the Fable verdict was
persisted (14:26:34), and the persist commit explains the brief could not be committed earlier
because the tickle was blocking every commit — so the reset was authored **after** the consult
ran, and "owner" is consistent with the owner-approved verdict. No finding.

**Q2 — did the fold correct its claims everywhere?** Yes, everywhere it can:
`git grep 'stopped discriminating'` returns exactly one hit, the corrected quotation at
`authority_manifest.rs:794`. Nothing downstream relies on the wrong claim. ★ **But the
correction is itself wrong in the other direction** — *"never weakened at any pin"* is false at
precisely pin 0, the pin this branch sets. That is I-1.

**Q3 — is anything orphaned by the deleted test?** Almost nothing. Every surviving reference to
`ARCHIVE_RECONCILIATION_REVIEW_BY`, `the_archive_reconciliation_is_not_past_its_review_by`,
`tickle`, `design/forms/periodic/` and the five deleted PDFs is either a deliberate retirement
record or a persisted report. `authority_conflicts::today()` correctly narrowed to private and
`AUTHORITY_CONFLICTS.md`'s review-by is genuinely its only caller. §G-12's unblock command was
repointed off the deleted directory. The real orphans are **I-3** (a witness whose subject was
deleted) and **M-1** (a count that describes the deleted mechanism wrongly).

**Q4 — counts.** Nine measured; eight exact, one stale (M-2). Table below.

**Q5 — is the replacement guard sufficient?** For what it covers, yes: `duplicates()` reads the
**committed manifest**, not the local tree, so the pin is environment-independent and reds on a
planted duplicate as recorded. What the pin cannot see, beyond FR-23/24/25, is **I-1**: nothing
binds a note to its manifest entry, so a note-storage document can lose its entry, its hash and
its URL with every gate green. FR-23 and FR-24 are correctly scoped; **FR-25 is mis-scoped** —
wrong direction, and its stated fix is non-hermetic (details under I-1). The retired tickle
would not have caught I-1 either, so this is not a regression *against the tickle* — it is a
regression against the pin's own previous value.

---

## WHAT I VERIFIED AND HOW

| claim | command | result |
|---|---|---|
| (B) binaries = 42 | `git ls-files legal/primary-sources \| wc -l` | **42** (16 html, 20 pdf, 6 xml) ✅ |
| (B) extracts = 20 | `git ls-files legal/text \| wc -l` | **20** ✅ |
| `SHA256SUMS` = 42 | `wc -l < legal/SHA256SUMS` | **42** ✅ |
| (A) extracts = 60 | `git ls-files design/forms/extract \| wc -l` | **60** ✅ |
| (A) notes = 60 | `git ls-files 'design/forms/*/*.pdf.txt' \| wc -l` | **60** ✅ |
| manifest = 102 | `json.load` | **102** (60 note + 42 committed) ✅ |
| deleted PDFs = 905,833 B | `git cat-file -s` ×5 on `main` | **905833** exact ✅ |
| `SOURCES.md` "~14 MB" | sum of tracked sizes | **14,357,756 B = 14.36 MB** ✅ |
| README "57 here + 9" | `git ls-files` ×2 | **60 + 0** ❌ → M-2 |
| five retired hashes preserved | note sha vs deleted `SHA256SUMS` line, ×5 | **5/5 MATCH** ✅ |
| five extracts survive | `test -f design/forms/extract/{f8949,i8949,f1040sd,i1040sd,f8283}--2025.txt` | 5/5 exist ✅ |
| repointed citations resolve | `grep -c` in the substituted extracts | `i1040sd--2025.txt` has "Schedule D Tax Worksheet" ×2; `f8283--2025.txt` has "Section B" ×12 ✅ |
| "only reader of `legal/text/irs-forms/` is MANIFEST.json" | `git grep 'text/irs-forms'` | only `MANIFEST.json:558,567` (the two surviving 1099-DA extracts) ✅ |
| manifest has 0 duplicate groups | hash-group count in python | **0** ✅ (main: 110 entries, **7** groups) |
| no empty sha/url/bytes in manifest | python scan | 0 / 0 / 0 ✅ |
| B1 kill path | `cat design/forms/2024/f8949--2024.pdf.txt` | `dcd2d7ff…`, `129683` — the destroyed-and-restored note ❌ → I-2 |
| regen is idempotent at HEAD | `--regen` then `git diff` | byte-identical ✅ |
| **the I-1 experiment** | 60 gitignored PDFs moved aside → `--regen` | **102 → 42 entries**; `cargo test -p xtask` 66 passed 0 failed; `authority-manifest` prints **"OK — every entry resolves and every source is listed"** exit 0; `archive-check` green exit 0 ❌ |
| no note is parseable by regen's sha fallback | grep for `sha256:` and bare-64-hex lines across all 60 notes | **0 of 60** for both forms ❌ |
| suite green at HEAD after revert | `cargo test -p xtask` | 66 passed / 0 failed / 1 ignored ✅ |

**The one mutation, and its revert.** To measure I-1 I moved 60 gitignored (untracked) PDFs to
the scratchpad and let `--regen` rewrite `MANIFEST.json`, having first `cp`-backed it up and
recorded `sha256 c0e5b2e551deccdb17e998cdfcfd37ef0c7fee16197a4deef992b636566cb62d`. Reverted by
`cp` from the backup — **never** `git checkout --` — and all 60 PDFs moved back. Post-revert:
`sha256sum` matches the pre-recorded value exactly, `git status --porcelain -uall` is **empty**,
60/60 PDFs present, a fresh `--regen` produces a byte-identical file, and the xtask suite and
both gates are green. Nothing tracked was modified, committed or pushed.

Not re-derived, per the brief: `make check`, `fmt`, `sha256sum -c`, and the author's own B1 kill.

---

## WHAT I COULD NOT CHECK

- **Network claims.** The three `irs-prior/` round-trips (HTTP 200 + hash-exact) and the
  8275-R `404`/`200` pair are asserted in the branch and were re-checked live by the author; I
  had no network and took them as given. They are load-bearing for Group A's retirement.
- **`main`'s CI history** — `dca6ef25`'s claim that the tickle reddened all three test platforms.
- **What the owner actually said.** RESET LOG row 2's `who = owner` is consistent with the
  commit ordering and the brief's statement that the Fable verdict was owner-approved, but the
  conversation itself is not in the repo.
- **The full `make check` surface.** I ran `cargo test -p xtask` (67 tests) repeatedly rather
  than the 2765-test suite, which the brief records as green at HEAD. My I-1 experiment
  therefore proves the **xtask** gates stay green on a gutted manifest; a test elsewhere in the
  workspace could in principle red on it, though nothing outside xtask reads `MANIFEST.json`.
- **Whether `--regen` is ever actually run on an unfetched tree in practice.** I proved the
  outcome, not the frequency. That is a judgment for the owner: the command is documented, the
  PDFs are gitignored, and FR-25 proposes automating exactly this call.
