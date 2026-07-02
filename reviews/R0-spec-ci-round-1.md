# R0 — SPEC_ci_infrastructure.md — round 1

**Artifact:** `design/SPEC_ci_infrastructure.md` (554 lines, read in full)
**Baseline verified against:** `main` @ `059f056` (current HEAD, confirmed)
**Reviewer:** R0 architect (independent; author ≠ reviewer)
**Date:** 2026-07-01

**Notation rule (this file is tracked content):** digit strings that match the spec's own
SSN/EIN shapes are written with a middle dot `·` in place of the hyphen (e.g. `99·1234567`
means the 2-digits, hyphen, 7-digits literal). This keeps this review from tripping the very
scan it reviews. The four already-excluded synthetic values are also written this way for
uniformity. The repository owner's legal name is written as `[owner name]`, never literally.

**Verdict: NOT green — 1 Critical, 6 Important. Blocking. The spec must be revised and
re-reviewed before any implementation begins.**

Everything below was verified empirically against the working tree at `059f056` (commands run,
not assumed). The single most important result: **two of the five CI jobs, as specified, fail
on their first run against the current repo** — the MSRV job can never pass (C1) and the
pii-scan job fails day one (I1).

---

## Critical

### C1 — The MSRV job is unimplementable as specified: cargo 1.74 cannot parse the committed lockfile, and the locked dependency graph itself requires ≥ edition-2024 toolchains

The spec's premise (D1 `msrv` job, lines 148–176) is `cargo check --workspace --locked` under
toolchain 1.74 against the committed `Cargo.lock`, with the only headroom risk identified as
"ratatui 0.29 MSRV exactly 1.74." Both layers of that premise are false, verified by running
the exact command locally (a 1.74 toolchain is installed on this machine):

1. **Lockfile format.** `Cargo.lock` is `version = 4` (line 3 of the file). Lockfile v4
   requires Cargo ≥ 1.78. Empirical result of `cargo +1.74 check --workspace --locked`:

   ```
   error: failed to parse lock file at: /scratch/code/bitcoin_tax/Cargo.lock
   Caused by:
     lock file version `4` was found, but this version of Cargo does not
     understand this lock file, perhaps Cargo needs to be updated?
   ```

   The job fails before resolving a single dependency. It can never go green.

2. **Locked dependency versions.** Even after flipping the lockfile header to `version = 3`
   (tested in a scratch copy; the pristine lockfile was restored afterward — `git status` clean),
   `cargo +1.74 metadata --locked` fails on **`zeroize v1.9.0`**: "this version of Cargo is
   older than the `2024` edition." Downgrading zeroize to 1.8.1 and retrying surfaces the next
   offender, **`time v0.3.51`**, with the identical edition-2024 error. The hole is at least two
   crates deep and plausibly deeper (iteration was stopped after two rounds — the point was
   proven). The committed lock's *actual* toolchain floor is ≥ 1.85 (edition 2024), not 1.74.
   The declared `rust-version = "1.74"` is currently aspirational, not a property of the build.

3. **Scope contradiction.** The spec's Out-of-scope section (line 551) states "no crate source,
   `Cargo.toml`, `Cargo.lock`, or test file is modified." Under that constraint the MSRV gate
   as designed is *provably* impossible: the untouchable lockfile is the thing that breaks it.

**Required fix — the spec must present this decision to the user (it cannot be silently
"resolved at implementation time"):**

- **Option A — make the lock 1.74-compatible.** Downgrade `Cargo.lock` to `version = 3` and
  walk offending deps down (`cargo +1.74 update -p zeroize --precise 1.8.1`, `time` → a
  pre-edition-2024 0.3.x, iterate to fixpoint) until `cargo +1.74 check --workspace --locked`
  passes locally. Costs: the spec's scope changes (Cargo.lock IS modified); security-relevant
  crates (`zeroize`, `time`) are pinned backward; every future `cargo update` risks re-breaking
  the floor (which is, to be fair, exactly what the gate is for).
- **Option B — raise the declared MSRV to the true floor.** Bump `workspace.package.rust-version`
  to match reality (empirically determine it; ≥ 1.85 given edition-2024 deps) and gate on that
  version. Costs: FOLLOWUPS M5 ("cargo +1.74 MSRV gate") must be consciously amended, and the
  "ratatui 0.29 zero-headroom" rationale re-derived against the new floor. For a single-user
  binary application (not a published library), MSRV is a self-imposed constraint — this is
  likely the cheaper honest answer, but it is the user's call, not the implementer's.
- **Option C — separate MSRV lockfile** (e.g. a committed `Cargo.lock.msrv` swapped in by the
  job). Nonstandard, drift-prone; not recommended; listed only for completeness.

Whichever option is chosen, Task 1's acceptance criteria MUST add: "`cargo +<MSRV> check
--workspace --locked` passes **locally** before merge" — the toolchain is installed and the
check is fully local-testable; deferring it to the post-merge green-run (as the spec currently
does) is an avoidable verification hole and is how this Critical went unnoticed.

---

## Important

### I1 — The pii-scan job fails on day one: the exclusion list is incomplete, and the spec's own KAT design plants two more failures

The spec asserts (lines 39–49, 480–482) that the four documented synthetic values are the
complete set, "confirmed by grep against the current source." Running the spec's exact CI
pipeline (D1 `pii-scan`, the `git ls-files | xargs grep -E … | grep -Fv …×4` chain, verbatim)
against `059f056` produces a **residual hit**:

```
crates/btctax-cli/tests/tax_report.rs:786:  donee_ein: Some("99·1234567".into())
```

That is a fifth synthetic EIN-shaped value (`99·1234567`, hyphen where the dot is), absent from
both the CI exclusion list and the hook's (which carries only two exclusions — see M2). The
first CI run fails; the first hook-guarded push fails. Fail-closed, so not a leak — but the
spec's completeness claim is factually wrong, and the gate arrives broken.

Two more day-one failures are *planted by the spec itself*:

- **KAT fixture literals.** KAT-H1 and KAT-H3 (lines 360–362) specify fixture strings
  `999·00·1234` (SSN-shaped) and `11·1234567` (EIN-shaped) — deliberately non-excluded so the
  KATs exercise the "hit" path. Per Task 1, `scripts/test-pii-hook.sh` is **committed** and will
  contain those literals → the pii-scan job flags the hook's own test script.
- **The spec document itself.** `design/` is tracked (12+ design docs at HEAD); when
  `SPEC_ci_infrastructure.md` is committed per the standard workflow, its lines 361–362 contain
  the same two literals → the scan flags the spec.

**Fixes (all three required):**
1. Add `99·1234567` (with hyphen) to the exclusion list, with a citation comment
   (`tax_report.rs:786`, synthetic sequential EIN). Alternative — change the test to reuse the
   canonical `12·3456789` — is cleaner long-term but violates the spec's "no test file modified"
   scope; if chosen, amend the scope statement explicitly.
2. `scripts/test-pii-hook.sh` must **assemble** fixture strings at runtime (e.g.
   `printf '%s-%s-%s' 999 00 1234`) so no shape-matching literal exists in tracked content.
   State this as a hard requirement in D3.
3. The spec document (and all future tracked prose, including reviews) must use a non-matching
   notation for shape examples — adopt the `·` convention used by this review, and say so in D2's
   INVARIANT block.
4. Add to Task 1 acceptance criteria: "the pii-scan script, run locally against the full tracked
   tree including all files this change adds, exits 0." That criterion is currently absent and
   would have caught all three failures.

### I2 — Missing patterns file = fail-open on the primary gate

D3 (lines 316–320) and KAT-H5 (line 364): when `scripts/.pii-patterns` is absent, the hook
prints a warning to stderr and **proceeds** (exit 0 if the generic scan is clean). The
owner-specific scan is the *primary* gate by the spec's own architecture (D1: "the hook is the
primary gate"; CI deliberately carries no owner-specific patterns). A warning is not a gate:

- On non-TTY pushes (IDE integrations, scripts), stderr scrolls past unseen or is swallowed.
- The exact scenario the spec worries about — "pushes from a new machine" (line 264) — is
  precisely when the patterns file is missing, and the hook's answer is to wave the push through.

Git hooks are not auto-installed, so the hook's mere presence already implies a deliberate
per-machine setup step; requiring the patterns file at the same moment costs nothing extra.

**Fix:** absent patterns file ⇒ **exit 1** with remediation text pointing at
`scripts/README-pii-setup.md`, plus an explicit, deliberate bypass for bootstrap
(e.g. `BTCTAX_PII_ACK_NO_PATTERNS=1 git push`). Flip KAT-H5 to assert exit 1; add KAT-H5b
asserting the bypass path exits 0 and still runs the generic scan. Installation documentation
(symlink one-liner + README) is otherwise adequate; consider also documenting
`git config core.hooksPath scripts` as the lower-friction install (the script is already named
exactly `pre-push`).

### I3 — The hook scans the working tree, not the commits being pushed

Both scans in D3 use `git ls-files | xargs grep` — i.e. the **current working-tree contents**.
A pre-push hook's job is to vet the *ref range being pushed*. Concrete leak path: commit 3 of 5
introduces PII, commit 4 removes it, operator pushes all 5 — working tree is clean, hook passes,
CI (which checks out only the tip) passes, and the PII is **permanently in remote history**.
Secondary defects of the same root cause: pushing a branch that is not checked out scans an
unrelated tree, and uncommitted working-tree edits are scanned even though they are not being
pushed (false positives/negatives both possible).

**Fix:** consume the standard pre-push stdin protocol
(`<local_ref> <local_sha> <remote_ref> <remote_sha>` per line); for each pushed ref, scan every
revision in the range — `git rev-list remote_sha..local_sha` (for new branches / deleted refs,
handle the all-zeros SHA via `--not --all` or equivalent) — using `git grep -E <pattern> <rev>`
(tree-accurate, no working-tree dependence, binary-safe via git's own attributes). At absolute
minimum, scan each pushed *tip* with `git grep … <local_sha>` instead of the working tree; the
full-range scan is strongly preferred given that history, once pushed, cannot be unpushed.
Rework KAT-H1–H7 to drive the hook via stdin ranges (temp repo, two branches, PII in an
intermediate commit as its own KAT — that case is the one that matters).

### I4 — Constraint 1 is already false at HEAD: the owner's legal name is in `LICENSE`, and any sane `.pii-patterns` will therefore fail every push

Constraint 1 (lines 59–61): "No owner-identifying literal (name, …) may appear in any committed
file." Verified sweep of all tracked files for owner tokens: **`LICENSE` line 3 — the MIT
copyright line — contains `[owner name]` literally.** (MIT requires a copyright holder; the
workspace declares `license = "MIT OR Unlicense"`. A second grep hit inside
`legal/…/CCA_202124008.pdf` was verified to be coincidental bytes in a compressed stream of a
public IRS document — non-finding.)

Consequence for the design: the first pattern any operator will put in `.pii-patterns` is the
owner's name. The hook (which scans all tracked files) then fails on `LICENSE` on **every push,
forever**. The operator's realistic responses are (a) delete the name pattern — silently
weakening the primary gate, or (b) habitual `git push --no-verify` — destroying the gate
entirely. Both are worse than the problem.

**Fix (both parts):**
1. Restate Constraint 1 with an explicit, enumerated carve-out: the copyright holder line in
   `LICENSE` is a deliberate, user-accepted exception (or, if the user prefers, relicense to
   Unlicense-only and drop the name — user decision; do not make it silently).
2. Give the hook a path-exception mechanism — either a hardcoded documented allowlist
   (`:!LICENSE` pathspec on the owner-specific `git grep`) or patterns-file syntax for per-pattern
   path exceptions. Document it in `README-pii-setup.md`. Add a KAT: owner pattern matching only
   the allowlisted path ⇒ exit 0; matching anywhere else ⇒ exit 1.

### I5 — Nothing makes a red CI run actually block anything: branch protection is neither specified nor consciously deferred

The Goal (line 7) says the workflow "gates every push and pull request," and the review prompt
for this repo class treats "a CI gate that silently doesn't gate" as blocking. As specified,
nothing blocks: this repo's demonstrated workflow is local merges pushed directly to `main`
(HEAD itself is such a merge commit). A failing job on a `main` push is *detective* — the commit
is already on the remote — and GitHub enforces nothing unless a branch protection rule / ruleset
requires the checks. The spec never mentions branch protection, so the five "gates" are, at
GitHub level, advisory lights.

**Fix:** add an explicit operator step (Task 2 / Definition of Done): create a ruleset on `main`
requiring the five named checks (and decide: require PRs? for a single-user repo maybe not) —
**or** explicitly document that detective-only posture is accepted for a single-user repo and
why, so the deferral is a decision rather than an omission. Either resolution is acceptable;
silence is not.

### I6 — The verification plan defers locally-testable gates to post-merge

Task 1's acceptance criteria (lines 447–458) omit the two checks that are both fully
local-testable *and* the ones this review found broken:
- the MSRV command (a 1.74 toolchain is installed on the dev machine — C1 was found by running
  it in under a minute), and
- the pii-scan pipeline (pure shell over the tracked tree — I1 was found the same way).

The spec's TDD claim ("local-testable first", line 429) is right in spirit and unapplied to 2 of
5 jobs. The post-merge green-run (lines 504–515) is a fine *confirmation* but the spec currently
uses it as the *first* execution of these two jobs.

**Fix:** Task 1 acceptance criteria gain two lines: (1) `cargo +<MSRV> check --workspace
--locked` passes locally; (2) the exact pii-scan script exits 0 locally against the tree
*including all files added by this change*. (Overlaps C1/I1 remedies; listed separately because
it is a plan-structure defect that will otherwise recur in future CI specs.)

---

## Minor

### M1 — `grep -Fv` excludes whole lines, not matched values
A line containing both a synthetic excluded value and a *real* SSN-shaped value would be
whitelisted wholesale. Filter on the matched token instead:
`git ls-files -z | xargs -0 grep -IHnoE '<shapes>' | grep -vE '(987·65·4321|12·3456789|99·1234567)$'`
(hyphens in place of dots; `-o` emits only the match, so the trailing anchor is exact).

### M2 — Exclusion lists are inconsistent across the spec's own snippets
The CI snippet carries four `-Fv` lines; the prose (lines 219–227) correctly proves
`987654321` and `P01234567` cannot match the shapes and "need no exclusion line" — yet the
snippet includes them anyway; the hook's generic scan carries only two. Three artifacts, three
lists, drift guaranteed. **Fix:** one canonical committed script (e.g.
`scripts/pii-scan-generic.sh`) invoked by both the hook and the CI job; the exclusion list
exists in exactly one place.

### M3 — `xargs`/binary robustness of the scan pipeline
`git ls-files | xargs grep` breaks on filenames with spaces (none tracked today — verified —
but the `legal/` archive grows), and errors are masked by `2>/dev/null || true`, i.e. unscanned
files fail *silently*. Tracked binary PDFs under `legal/` are grepped as bytes: today none match
(verified), but a future PDF whose compressed stream matches a shape yields
`Binary file … matches` — a hit no `-Fv` line can exclude. **Fix:** `git ls-files -z | xargs -0
grep -I …` (NUL-safe, skip binaries), and note in D1 that PDF content is out of the scan's reach
by design (compressed streams), so binary documents rely on provenance (public IRS documents
only) rather than the regex gate.

### M4 — No least-privilege `permissions:` block in the workflow
Depending on repo settings the default `GITHUB_TOKEN` may be read/write. Add top-level
`permissions: contents: read` to `ci.yml`. Costless hardening consistent with the spec's own
supply-chain posture.

### M5 — `cancel-in-progress: true` also cancels runs on `main`
A burst of pushes to `main` can cancel the very run the Definition of Done needs recorded.
**Fix:** `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`.

### M6 — Recon errata (correct before the spec is archived)
- Line 26–28: "The other two crates" — three are listed (`btctax-store`, `btctax-core`,
  `btctax-adapters`). Verified: exactly those three lack `rust-version`; the count word is wrong.
- Crate manifest citations lack the `crates/` path prefix (actual:
  `crates/btctax-tui/Cargo.toml:5`, `crates/btctax-cli/Cargo.toml:6` — line numbers verified).
- D5 "it likely exists": the root `.gitignore` exists at HEAD and already opens with a strong
  PII banner — state it as fact and note the new lines join an existing PII-focused file.
- `BTCTAX_PASSPHRASE` recon: 5 `.env(…)` Command-builder uses verified, but
  `crates/btctax-tui/src/unlock.rs:434` additionally uses in-process `std::env::set_var` in a
  test — hermeticity conclusion (no CI-level secret) stands; the "inside each integration-test
  Command builder" wording does not.

---

## Nit

- **N1:** `push` + `pull_request` triggers double-run every PR branch; harmless (cost only) in a
  single-user repo.
- **N2:** D4's example SHAs/comments are illustrative (the `dtolnay/rust-toolchain` line cites a
  fabricated "stable 2025-x"); the spec says resolve at impl time — add "the D4 examples are NOT
  real pins; never copy them" to prevent exactly that.
- **N3:** `mapfile` requires bash ≥ 4 — fine for the Linux-only scope; note it in
  `README-pii-setup.md` for the future macOS leg (ships bash 3.2).
- **N4:** KAT temp repos must `git add` fixtures before any `git ls-files`/`git grep`-based scan
  sees them; after the I3 rework, KATs drive the hook via the stdin ref protocol.
- **N5:** Commit authorship on every push carries the owner's real email (author name is
  initials only — verified). Outside the tracked-content invariant and mitigated by the private
  repo; recorded here so the exclusion is a decision, not an oversight.

---

## Answers to the standing gate questions

1. **PII mechanism.** (a) The spec text and its embedded snippets contain no owner-identifying
   literal — placeholders and synthetic values only — **pass** (but see I1.2/I1.3: two synthetic
   fixture literals in the spec/KAT design would themselves trip the scan once committed).
   (b) Hook install is documented (symlink + README) — adequate; missing-patterns behavior is
   fail-open — **I2**; working-tree-vs-pushed-range — **I3**; LICENSE collision — **I4**.
   (c) Generic-shape scan run verbatim against HEAD: raw shapes hit 10 lines across 6 source
   files + 1 review file; after the four documented exclusions, **one residual hit remains**
   (`tax_report.rs:786`, `99·1234567`) — **I1**. Verified: the SSN shape does not match the
   undelimited 9-digit TIN, the EIN shape does not match it either, and `P01234567` is
   alphanumeric — the spec's no-exclusion-needed analysis for those two values is correct.
2. **Gates gate.** `--locked` on every compiling invocation — correct as specified. MSRV job —
   **C1** (cannot pass: lockfile v4 + edition-2024 deps; the "lockfile must be MSRV-compatible"
   subtlety is not handled — it is the whole ballgame). Blocking mechanism — **I5**.
3. **Supply chain.** SHA-pinning resolved at impl time with tag-comment convention — acceptable
   (with N2). Minimal action set (checkout / rust-toolchain / rust-cache) — each justified; no
   unnecessary action; no secrets exist, none echoed, no `pull_request_target` — pass, with M4
   as costless hardening.
4. **Verification plan.** Hook KATs against synthetic fixtures = genuine TDD — good; but 2 of 5
   CI jobs are locally testable and untested pre-merge — **I6**. Post-merge green-run as
   *confirmation* — fine.
5. **Scope.** Linux-only, cargo-audit/deny deferred, no rust-toolchain.toml — right-sized and
   consistent with FOLLOWUPS; the one scope error is the "Cargo.lock is not modified" clause
   colliding with the MSRV gate (**C1**).

## Disposition

**1 Critical, 6 Important, 6 Minor, 5 Nit ⇒ gate closed.** C1 and I4 require user decisions
(MSRV floor vs. lockfile downgrade; LICENSE name carve-out vs. relicense) and must not be
resolved unilaterally by the implementer. Re-review (round 2) required after the spec is
revised.

---
---

# Round 2 — re-review of v0.2 (round-1 folds)

**Artifact:** `design/SPEC_ci_infrastructure.md` v0.2 (695 lines, re-read in full)
**Date:** 2026-07-02. Same notation rule as round 1 (`·` = hyphen in shape-matching digit
strings; `[owner name]` never literal).

**Verdict: NOT green — 1 NEW Important (I7), 3 new Minor, 1 new Nit. All 18 round-1 findings
are correctly folded (verified below, empirically where testable). The I7 fix is one token;
a round-3 confirmation pass is required after the fold, per the workflow.**

## Round-1 folds — verification results

| Finding | Status | Evidence |
|---------|--------|----------|
| C1 | **CLOSED** | Re-ran the bisection myself: `cargo +1.85.0 check --workspace --locked` FAILS with rust-version errors (`time 0.3.51` / `time-core 0.1.9` / `time-macros 0.2.30` require rustc 1.88.0; `instability 0.3.12` requires 1.88; `icu_*` 2.2.0 / `idna_adapter 1.2.2` require 1.86 — matching the spec's list, plus one omission, see N6). `cargo +1.88.0 check --workspace --locked` PASSES (`Finished dev profile`, exit 0). Floor = 1.88 confirmed (the 1.88.0-exact requirements make 1.86/1.87 moot). Sole manifest change is `rust-version`; `Cargo.lock` untouched; workflow `toolchain: "1.88"` + `key: msrv-1.88` coherent; the "empirical result wins" re-verification clause and the local MSRV acceptance item are present (Task 1 order step 1 + acceptance bullet 3). |
| I1 | **CLOSED** | Shape regexes vs the spec file: **0 matches** (·-notation holds throughout, including the fold record and D3a comments). Vs the round-1 review: **0 matches**. Token-level scan of the full `HEAD` tree with the three exclusions: **0 residual tokens**. `99·1234567` excluded with citation (D3a + Current state). KAT fixtures runtime-assembled (D3c, binding). "Scan exits 0 against the full tree including every added file" is Task 1 acceptance bullet 2. |
| I2 | **CLOSED** | D3b semantics 2 + design shape: missing patterns file ⇒ exit 1 with remediation text; `BTCTAX_PII_BYPASS=1` downgrades ONLY the missing-file check (generic scan still runs — confirmed in the design shape's control flow). KAT-H5 flipped, KAT-H5b added and asserts the generic scan stays active under bypass. Residual: M9 below (empty-but-present patterns file). |
| I3 | **CLOSED** (with new defect I7 in the rework) | Stdin ref protocol consumed; per-ref ranges via `rev-list`; every rev scanned via `git grep` against commit trees; deletion (all-zeros local) skipped; KAT-H3 is the add-then-remove intermediate-commit case and KAT-H9 covers deletions. The new-ref (all-zeros remote) arm is WRONG — see I7. |
| I4 | **CLOSED** | Constraint 1 restated with the enumerated `LICENSE` carve-out; `':(exclude)LICENSE'` applied only to the owner-specific scan; allowlist hardcoded, `LICENSE` the only entry, growth declared a spec-level change; KAT-H8 covers both directions (match only in LICENSE ⇒ 0; same content elsewhere ⇒ 1). |
| I5 | **CLOSED** | Task 2 ruleset step with concrete `gh api` call requiring the five checks, operator confirmation, and an explicit documented-acceptance fallback ("a decision, not an omission"). The spec honestly notes required checks on `main` force a PR-based flow — the fallback handles the user declining. |
| I6 | **CLOSED** | All locally-testable verification (KATs, full-tree scan, MSRV command + floor confirmation, check-ignore, actionlint-or-review, test count 692) is in Task 1 acceptance; post-merge verifies GitHub-side wiring only. |
| M1–M6 | **CLOSED** | M1: token-level `-o` extraction + token-anchored exclusion filter, KAT-G4 locks it in. M2: one canonical `scripts/pii-scan-generic.sh`; CI job and hook both call it; the workflow contains no regexes/digits. M3: `git grep -I` against revs (tree-accurate, binaries skipped), grep statuses propagated — verified the D3a subtlety: `pipefail` (set at top, NOT disabled by the local `set +e`) makes `gs` reflect git grep's status through the `| sort -u` pipe, so the >1 error check genuinely fires. M4: `permissions: contents: read` (Constraint 8 + D1). M5: `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`. M6: all four errata corrected — including the `unlock.rs:434` set_var wording; the "mutex-serialized" claim verified against source (the `_env_guard` comment at `crates/btctax-tui/src/unlock.rs:432–434`). |
| N1–N5 | **CLOSED** | N1 accepted in D1; N2 fabricated SHAs removed, explicit "never copy examples" warning in D4; N3 bash ≥ 4 note routed to the README; N4 subsumed by the I3 stdin/commit-tree rework; N5 recorded as a conscious exclusion in FOLLOWUPS. |

## New findings (round 2)

### I7 (Important) — The new-ref arm of the range scan scans NOTHING: `--not --all` excludes the very branch being pushed

D3b Semantics item 1 (binding) and the design shape both specify, for an all-zeros remote SHA:
`revs = git rev-list <local_sha> --not --all`. `--all` includes `refs/heads/<the branch being
pushed>`, whose tip IS `local_sha` — so every commit reachable from `local_sha` is excluded and
the result is the **empty set**. Empirically demonstrated in a temp repo (2 commits, tip on a
local branch): `git rev-list <tip> --not --all` → **0 revs**; `git rev-list <tip> --not
--remotes` → 2 revs. Consequence as specified: every push of a **new branch** — the most common
way brand-new work (and brand-new mistakes) first leaves the machine — is scanned zero times by
the primary gate. The spec is also internally contradictory: KAT-H4 asserts the new-ref path
*finds* a hit, which the binding semantics make impossible; at implementation one of the two
must silently win, and if the semantics win the gate has a hole.

**Fix (one token, two places — D3b Semantics item 1 and the design-shape `rev-list` line):**
`--not --all` → `--not --remotes` (commits not reachable from any remote-tracking ref — i.e.
not yet known to be public). Optionally sharpen to `--not --remotes="$1"` (the hook's first
argument is the remote name) with plain `--remotes` as the documented fallback — if `$1` is a
URL rather than a named remote, a non-matching glob excludes nothing and the hook over-scans,
which fails safe. KAT-H4 needs no change (temp repos have no remote-tracking refs, so
`--not --remotes` scans the full history there — consistent with its exit-1 assertion).

### M7 (Minor) — D3a second-pass location loop has an argument-order bug that blanks the diagnostics

`git grep -InF -- "$tok" "$REV"` puts BOTH `$tok` and `$REV` after `--`, so git parses them as
pathspecs with no pattern → status 2, swallowed by `|| true`. Exit 1 still fires (the gate
gates — `bad` is non-empty), but the "actionable file:line locations" the loop exists to print
are empty. Fix: `git grep -InF -e "$tok" "$REV" --`. Add a KAT assertion that the failure
output contains a `file:line` for the offending token (locks the diagnostics in red/green).

### M8 (Minor) — KAT isolation must copy the scripts; running them in place breaks H5 and can clobber the operator's real patterns file

The hook resolves `PATTERNS_FILE` relative to its own location (`BASH_SOURCE`). If the KATs run
`scripts/pre-push` from the real repo in place: KAT-H5 (missing patterns file) fails on any
machine where the operator HAS a real `scripts/.pii-patterns`, and KAT-H6 (create a temp
patterns file) would have to write into the real `scripts/` directory — risking overwrite of
the operator's actual owner-pattern file. D3c says "isolated from the real repo" but never
states the mechanism. Fix: D3c explicitly requires the harness to COPY `pre-push` +
`pii-scan-generic.sh` into each temp workspace and run the copies; the real
`scripts/.pii-patterns` is never read, written, or deleted by any KAT.

### M9 (Minor) — Present-but-empty patterns file silently degrades the primary gate to generic-only

If `scripts/.pii-patterns` exists but contains zero non-comment lines, `COMBINED` stays empty
and the owner-specific scan is skipped with no message — the exact fail-open shape I2 was
about, one notch removed (and `touch scripts/.pii-patterns` is a plausible fresh-clone
half-setup). Fix: treat an effectively-empty patterns file the same as a missing one
(fail-closed + the same bypass), or at minimum warn loudly; add KAT-H5c either way.

### N6 (Nit) — The 1.88-floor dep list omits the `darling` family

`cargo +1.85.0` also reports `darling@0.23.0` / `darling_core@0.23.0` / `darling_macro@0.23.0`
requiring rustc **1.88.0** — absent from the fold record and the Current-state bullet, and the
zero-headroom note credits only the `time` family. Conclusion (floor = 1.88, zero headroom)
unchanged; the list should be complete so a future `time` upgrade isn't mistaken for the only
binding constraint.

## Process caveat (not a technical finding — blocking for SHIP, not for the next fold)

The two user decisions (C1 Option B; I4 LICENSE carve-out) reached this review **relayed via
the coordinator**, and the spec itself flags them "recorded for round-2 confirmation." A relay
is not user confirmation. R0 verifies the *technical content* of both resolutions as sound and
correctly folded; the workflow still requires the user's own recorded confirmation (one line
from the user in the main session, echoed into FOLLOWUPS at ship) before this spec is treated
as user-approved. Implementation prep may proceed against v0.2 + the I7 fold at the author's
risk; shipping may not.

## Round-2 disposition

**0 Critical / 1 Important / 3 Minor / 1 Nit ⇒ gate still closed — one fold away from green.**
Required: fold I7 (one-token fix, no KAT change), M7–M9 (small, mechanical), N6 (list
completion); then round 3 confirms the fold. All round-1 findings stay closed as verified
above; nothing in v0.2 regressed a round-1 resolution.

---
---

# Round 3 — confirmation of v0.3 (round-2 folds)

**Artifact:** `design/SPEC_ci_infrastructure.md` v0.3 (743 lines, re-read in full)
**Date:** 2026-07-02. Same notation rule (`·` = hyphen; `[owner name]` never literal).

## Round-2 folds — verified

- **I7 CLOSED.** D3b Semantics item 1 and the design shape both now use
  `git rev-list <local_sha> --not --remotes` for the new-ref arm, with the "NEVER `--not
  --all`" rationale inline, the author's empirical re-verification recorded (0 vs 2 revs; the
  scoped non-matching glob over-scans → fails safe), and the optional `--remotes="$1"`
  sharpening documented with plain `--remotes` as fallback. Grep-verified: the only remaining
  `--not --all` mentions in the spec are the three explanatory/prohibitive references (fold
  record, D3b rationale, Task 2 item 6's "NEVER" check); both operative `rev-list` lines carry
  `--not --remotes`. KAT-H4's exit-1 assertion is now satisfiable and its text explains why
  (temp repos have no remote-tracking refs → full history scanned).
- **M7 CLOSED.** D3a's diagnostics loop is `git grep -InF -e "$tok" "$REV" -- >&2 || true`
  (pattern via `-e` before the rev, pathspec separator last) with the why-comment; KAT-G1 gains
  the file:line output assertion; Task 2 item 10 locks both in review.
- **M8 CLOSED.** D3c states the isolation mechanism explicitly: the harness COPIES `pre-push` +
  `pii-scan-generic.sh` into each temp workspace and runs the copies; the operator's real
  `scripts/.pii-patterns` is never read, written, or deleted by any KAT; the stdin example
  invokes `<copied>/pre-push`. Task 2 item 11 added.
- **M9 CLOSED.** Present-but-empty (zero non-comment, non-blank lines) is treated identically
  to missing: fail-closed exit 1, same `BTCTAX_PII_BYPASS=1` path, bypass still leaves the
  generic scan running (confirmed in the restructured design-shape control flow: the empty
  `COMBINED` check covers both cases; the per-rev generic call is unconditional). KAT-H5c
  added; Task 2 item 7 updated.
- **N6 CLOSED.** The `darling`/`darling_core`/`darling_macro` 0.23.0 family (rustc 1.88.0) now
  appears in the fold record, the Current-state floor evidence, and the zero-headroom note —
  which correctly observes there are now two independent binding families, so upgrading `time`
  alone would not lower the floor.

## Fresh sweep (no regressions, no new findings of consequence)

Shape regexes vs v0.3: **0 matches** (the runtime-assembly example `printf '%s-%s-%s' 999 00
1234` does not form a contiguous shape — verified). Owner tokens vs v0.3: **0**. Tracked tree:
unmodified. Internal consistency spot-checks: KAT roster references (G1–G4, H1–H9 incl.
H5b/H5c) consistent across D3c, Task 1, and Task 2; round-1 fold-record I3 line updated to
reflect the I7 correction; Task 2 items 6/7/10/11 mirror the new binding requirements.

**N7 (Nit, non-blocking, recorded only):** D3b's closing "what must not change" list still says
"fail-closed on a missing patterns file" without "or effectively-empty". Zero enforcement
consequence — the binding Semantics item 2 and KAT-H5c govern — but the implementer may align
the wording in passing.

## Process note (unchanged in substance, now internalized by the spec)

The C1 and I4 decision attributions remain coordinator-attested; R0 cannot itself verify user
consent from a relay, and does not. What round 2 required — that the spec carry the obligation
to echo the user's own confirmation line into FOLLOWUPS at ship, before the spec counts as
user-approved — is now written into the spec (fold-record C1 entry and the round-2 process
note). Verification of that echo belongs to the whole-diff review at ship. On that basis the
process caveat no longer blocks implementation.

## Round-3 disposition

**0 Critical / 0 Important / 0 Minor / 1 Nit (N7, non-blocking) ⇒ R0 GREEN.** The spec is
ready to implement against v0.3. Standing conditions carried into Task 2 / ship: (1) the
whole-diff review re-runs the empirical checks (floor bisection at implementation HEAD, KATs,
full-tree scan) per Task 2 items 1–11; (2) the FOLLOWUPS amendment at ship must include the
user's own confirmation of the C1 (MSRV → 1.88) and I4 (LICENSE carve-out) decisions.
