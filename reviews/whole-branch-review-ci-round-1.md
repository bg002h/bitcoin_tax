# Whole-diff review — CI infrastructure branch (`feat/ci-infra`) — round 1

**Range:** `059f056..6561313` (3 commits: `a1d5e26` spec+R0, `b334412` infrastructure, `6561313`
render.rs corollary). Diff package `.superpowers/sdd/review-059f056..6561313.diff` verified
content-identical to a fresh `git diff 059f056..HEAD` (hunks differ only in context-window width).
Working tree clean at `6561313`; all checks below run at THIS HEAD.
**Spec:** `design/SPEC_ci_infrastructure.md` v0.3 (R0 GREEN after 3 rounds).
**Reviewer:** independent whole-diff reviewer (author ≠ reviewer). **Date:** 2026-07-02.

**Notation rule (this file is tracked content):** shape-matching digit strings are written with
`·` in place of the hyphen (e.g. `99·1234567`); the repository owner's name is written
`[owner name]`, never literally.

**Verdict: NOT ready to merge — 0 Critical / 1 Important / 3 Minor / 3 Nit.**
The one Important (I-1, the hook's executable bit) is a one-command, mode-only fix; everything
else verified green. Re-review of the fold required per §2.

---

## Empirical verification (re-run at this HEAD, per the R0 round-3 standing condition)

All commands run by this reviewer, not taken from the report.

### 1. Hook binding semantics — VERIFIED (with I-1 on the install boundary)

- **Range scan via pre-push stdin protocol:** `scripts/pre-push:32` reads
  `local_ref local_sha remote_ref remote_sha` per line; deletion (all-zeros local SHA) skipped
  (`:33`); existing-ref arm `git rev-list "$remote_sha..$local_sha"` (`:41`); new-ref arm
  `git rev-list "$local_sha" --not --remotes` (`:39`) — **NOT `--not --all`**.
- **New-ref arm re-tested in a temp repo** (2 commits, new local branch, no remote-tracking
  refs): `--not --all` → **0 revs** (the I7 hole, confirmed still real); `--not --remotes` →
  **2 revs** (correct); `--not --remotes=origin` with no such remote → 2 revs (non-matching glob
  excludes nothing — over-scan, fails safe); with a simulated `origin/main` at commit 1 →
  **1 rev** (exactly the unpushed commit). The shipped arm is correct and minimal.
- **Every rev in the range scanned:** the `for rev in $revs` loop (`:44`) runs BOTH scans per
  revision against commit trees (`git grep … "$rev"`), no working-tree dependence. KAT-H3
  (intermediate add-then-remove commit) re-run → exit 1. KAT-H4 (all-zeros remote SHA) re-run →
  exit 1.
- **LICENSE allowlist:** owner scan is `git grep -InE "$COMBINED" "$rev" -- ':(exclude)LICENSE'`
  (`:48`) — applied ONLY to the owner-specific scan; the generic scan has no carve-out. KAT-H8-1
  (match only in LICENSE → 0) and H8-2 (also elsewhere → 1) re-run, both pass.
- **Fail-closed [I2/M9]:** missing patterns file → exit 1 with remediation text (KAT-H5 pass);
  present-but-empty (comments/blanks only) → identical exit 1 (KAT-H5c pass). `mapfile` +
  non-comment filter (`:18`) makes empty ≡ missing.
- **Bypass scope:** `BTCTAX_PII_BYPASS=1` downgrades ONLY the missing/empty-patterns check
  (`:21–29`); the generic scan call (`:63`) is unconditional inside the rev loop. KAT-H5b-1
  (bypass + generic-shaped fixture → exit 1) and H5b-2 (bypass + clean → exit 0) re-run, pass.
- **Error propagation [M3]:** owner-scan grep status handled three-way (0 = finding, 1 = clean,
  >1 = abort with distinct message, `:51–57`); generic-scan status 1 = finding vs >1 = abort
  (`:66–71`); in `pii-scan-generic.sh`, `pipefail` (not disabled by the local `set +e`) carries
  git grep's status through `| sort -u`, and status >1 exits 2 (`:30`).
- **Diagnostics form [M7]:** `git grep -InF -e "$tok" "$REV" -- >&2`
  (`scripts/pii-scan-generic.sh:38`) — pattern via `-e` before the rev, pathspec separator last.
  KAT-G1's `file:line` output assertion re-run, passes.
- **Token-exactness:** verified in the temp repo that `git grep -IhoE … <rev>` with `-h`
  suppresses the `rev:path:` prefix (bare token out), so the anchored `ALLOWED` exclusion is
  token-exact; without `-h` the prefix would defeat it — the shipped flags are right.
- **KAT isolation [M8]:** full harness re-run: **18/18 PASS, exit 0**. Every KAT copies both
  scripts into a `mktemp -d` workspace and runs the copies from inside it; every temp
  `.pii-patterns` is written to `$TMPWS`. No real `scripts/.pii-patterns` exists on this machine
  before or after the run — nothing was created, read, or deleted.

### 2. Zero PII in tracked content — VERIFIED

- `bash scripts/pii-scan-generic.sh` (HEAD, full tree incl. every file this branch adds) →
  `pii-scan: clean (HEAD).`, **exit 0**.
- Owner-token sweep (name from `LICENSE:3`, and the owner's email localpart) over the entire
  3-commit diff: **0 hits**. Over the full tracked tree minus `LICENSE`: only the known
  coincidental compressed-stream bytes in `legal/…/CCA_202124008.pdf` (adjudicated non-finding in
  R0 round 1; `-I` skips it as binary anyway).
- KAT fixtures are runtime-assembled (`scripts/test-pii-hook.sh:67–71`, `printf` segments); no
  segment matches a shape; no composed literal exists in any tracked file.
- **Exclusion list vs live tree — exact match, all citations re-grepped at this HEAD:**
  - `987·65·4321` → `crates/btctax-core/src/donation.rs:94`,
    `crates/btctax-core/tests/kat_forms.rs:1102` ✔
  - `12·3456789` → `donation.rs:91`, `crates/btctax-cli/src/cmd/reconcile.rs:666`,
    `crates/btctax-cli/src/donation_details.rs:110`, `crates/btctax-cli/src/render.rs:2926,2970`,
    `kat_forms.rs:1099` ✔ (render.rs line numbers unaffected by the 1-for-1 corollary edit)
  - `99·1234567` → `crates/btctax-cli/tests/tax_report.rs:786` ✔
  - Bare 9-digit TIN and alphanumeric PTIN re-confirmed non-matching against the shipped shapes
    (piped through the exact `SHAPES` ERE → 0 matches); documented-only, no exclusion entry ✔
  - One additional tracked-prose site found — see M-2 (pre-existing, scans clean).

### 3. Workflow — VERIFIED

- **Five jobs** `test` / `clippy` / `fmt` / `msrv` / `pii-scan`, all `ubuntu-latest`, names
  matching the spec's ruleset contexts. `--locked` on every compiling invocation (test, clippy,
  msrv `cargo check`); `fmt` correctly takes none and has no cache step. Top-level
  `permissions: contents: read`. Concurrency exactly per spec:
  `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`. `toolchain: "1.88"` is quoted
  (YAML float pitfall avoided) and coherent with `key: msrv-1.88` and the manifest.
- **SHA pins independently resolved by this reviewer** via `git ls-remote` against the live
  repos (method independent of the report's `gh api`):
  - `actions/checkout` `refs/tags/v4` → `34e114876b0b11c390a56381ad16ebd13914f8d5` — matches pin ✔
  - `dtolnay/rust-toolchain` `refs/tags/v1` → `e97e2d8cc328f1b50210efc529dca0028893a2d9` — matches ✔
  - `Swatinem/rust-cache` `refs/tags/v2` = annotated tag `42dc69e1…`, peeled (`v2^{}`) →
    `e18b497796c12c097a38f9edb9d0641fb99eee32` — matches; the pin correctly targets the
    **dereferenced commit**, not the tag object ✔
  All three actions are within the vetted set (Constraint 4); no others used; no `secrets.`
  reference anywhere in the workflow; no `pull_request_target`.
- `actionlint .github/workflows/ci.yml` → **exit 0** (which also proves YAML-parse clean).

### 4. MSRV bump — VERIFIED

- `Cargo.toml`: `rust-version = "1.74"` → `"1.88"` is the sole manifest change (diff hunk is
  exactly that line).
- **`Cargo.lock` untouched:** `git diff 059f056..HEAD -- Cargo.lock` is empty.
- Floor re-bisected at this HEAD: `cargo +1.88.0 check --workspace --locked` → **exit 0**
  (`Finished dev profile`); `cargo +1.85.0 check --workspace --locked` → **fails (exit 101)**
  with `darling`/`darling_core`/`darling_macro 0.23.0` requiring rustc 1.88.0 and `icu_* 2.2.0`
  requiring 1.86 — consistent with the spec's two-binding-family, zero-headroom analysis.
- **Stale-1.74 sweep:** the spec assigns the repo-wide sweep to **Task 2** (fold-record C1:
  "repo-wide stale-'1.74' grep is Task 2 hygiene"; Task 2 item 9), NOT Task 1 — so the mentions
  remaining at this HEAD are in-plan, not diff defects. Live sites still to update at ship:
  `FOLLOWUPS.md:42` ("MSRV 1.74 gate"), `FOLLOWUPS.md:193/203`,
  `crates/btctax-cli/Cargo.toml:6` ("(1.74)" comment), `crates/btctax-store/src/lock.rs:20`
  (comment; its ≥1.64 claim stays true at 1.88). Historical specs/plans/reviews mentioning 1.74
  are archives and should be left as record. See Open ship obligations.

### 5. The render.rs corollary — RATIFIED

`crates/btctax-cli/src/render.rs:196`: `year.map_or(true, |f| f == y)` →
`year.is_none_or(|f| f == y)`.

- **Behavior-identical:** `Option::is_none_or(f)` returns `true` for `None`, else `f(v)` —
  exactly `map_or(true, f)`. Stabilized in Rust 1.82 < 1.88. The stale "1.74-compatible; not
  is_none_or" comment is replaced with "is_none_or stable since 1.82".
- **Causally forced by the spec's own D6 + clippy job — reviewer-verified mechanism:** clippy's
  `unnecessary_map_or` is MSRV-gated on the manifest's `rust-version`. Reproduced in a scratch
  crate on the same stable toolchain: with `rust-version = "1.74"` the lint is **silent**; with
  `"1.88"` it **fires**. So the D6 bump (mandatory) makes `clippy -D warnings` (the branch's own
  gate) red without this edit. The deviation from Constraint 9 ("crate source files NOT
  modified") is real but **justified, minimal (1 line, no logic change, test count unchanged),
  and correctly isolated in its own commit with an honest message**. Accepted as an MSRV-bump
  corollary. (The report's supporting claim that the lint was "pre-existing on `a1d5e26`" is
  wrong — see M-1 — but the corrected causality argues *more* strongly for the corollary, not
  less.)

### 6. Suite, determinism, install docs

- Test suite untouched by the infra commit; the corollary is behavior-identical; the report and
  the attested local gate record **692 passed / 0 failed**, clippy/fmt clean. Per the review
  charter the suite was not re-run here; mechanisms were.
- Scripts are deterministic: no network, no clock, no env dependence beyond the documented
  `BTCTAX_PII_BYPASS`; KATs use isolated `mktemp` repos.
- Install is documented, not automatic (`scripts/README-pii-setup.md`: symlink or
  `core.hooksPath`, patterns-file format with placeholder-only examples, bootstrap bypass,
  bash ≥ 4 note, LICENSE-allowlist note). `git check-ignore -v scripts/.pii-patterns` and
  `….bak` resolve via `scripts/.gitignore:2–3`; the root `.gitignore` carries the same two lines
  (D5 defense-in-depth) ✔. But see I-1: as committed, both documented installs produce a hook
  git will not run.

---

## Findings

### Important

#### I-1 — `scripts/pre-push` is committed NON-executable (mode 100644): both documented installs yield an ignored hook, and every push proceeds unguarded — the primary gate fails open at the install boundary

- **Fact:** `git ls-files -s scripts/` shows `100644` for `scripts/pre-push` (and
  `pii-scan-generic.sh`); only `test-pii-hook.sh` is `100755`. The diff records
  `new file mode 100644` for the hook. Spec D3b is explicit: "`scripts/pre-push` — the hook
  script (committed, **executable**)."
- **Consequence, empirically demonstrated by this reviewer** (temp origin + clone,
  `core.hooksPath` install, mode-644 hook that `exit 1`s): the push **succeeded** (exit 0) with
  only a stderr hint — `hint: The 'scripts/pre-push' hook was ignored because it's not set as
  executable.` After `chmod +x`, the same hook ran and blocked the push. Git requires the
  executable bit for BOTH documented install paths (the symlink resolves to the same non-exec
  file). On any fresh clone — precisely the new-machine scenario R0-I2 was about — the
  owner-specific gate silently does not run, and a hint on stderr "is not a gate" (R0-I2's own
  words). This is the exact fail-open shape the fail-closed redesign exists to prevent, one
  layer down.
- **Why 18/18 KATs didn't catch it:** the harness `chmod +x`es its copies
  (`test-pii-hook.sh:34`) and invokes them via `bash ./pre-push` (`:59`) — the tracked mode is
  never exercised. (No KAT change required for the fix, but see N-2 for an optional hardening.)
- **Fix (mode-only, no content change):** `chmod +x scripts/pre-push scripts/pii-scan-generic.sh`
  and commit (tracked modes → `100755`). The generic script's bit is not load-bearing (every
  caller uses `bash …`), but the spec's file table and shebang imply executability; flip both
  for consistency.
- **Severity rationale:** Important, not Critical — no PII exists in the diff, git does print a
  hint per push, CI's generic scan is unaffected, and the fix is one command. But it is blocking:
  the branch's headline deliverable ("fail-closed range-scanning pre-push PII hook") does not
  run at all when installed as documented.

### Minor

#### M-1 — Report's Constraint-9 justification misstates the clippy baseline ("pre-existing on `a1d5e26`")

`.superpowers/sdd/ci-infra-report.md` (untracked, gitignored) claims the `unnecessary_map_or`
lint "was pre-existing on `a1d5e26` (the base commit also fails cargo clippy … -D warnings)".
Reviewer-tested: the lint is MSRV-gated on the manifest's `rust-version` — at `a1d5e26`
(rust-version still `"1.74"`) it cannot fire; it fires only after the D6 bump. The corollary's
justification is therefore *stronger* than the report states (the bump directly creates the
red), but the record feeding the ship notes is factually wrong and should be corrected at ship
so the Constraint-9 deviation is logged with the true causality.

#### M-2 — Pre-existing tracked prose contradicts the spec's Notation rule (real-hyphen synthetic tokens outside the canonical list)

`reviews/whole-branch-review-gift-chunk3b-round-1.md:30` (an earlier cycle's review, NOT in this
diff) contains `987·65·4321` and `12·3456789` with **real hyphens**. The spec's Notation rule
asserts the real hyphen exists in `scripts/pii-scan-generic.sh` "and NOWHERE else" — false at
this HEAD for that one pre-existing file. **No gate consequence** (verified: the token-level
exclusion is file-agnostic, so the tree scans clean — which is itself worth knowing: excluded
synthetics are excluded *everywhere*, not just at the cited source lines). Resolve at ship in
the Task-2 hygiene pass: either dot-notate that old review or amend the spec's "NOWHERE else"
sentence to scope it to files from this spec forward. A decision, not silence.

#### M-3 — Commit `b334412` is transiently clippy-red (gate-bisection hygiene)

Because the lint fires the moment `rust-version = "1.88"` lands (M-1 mechanism) and the
corollary fix is the *next* commit, `b334412` itself fails the branch's own
`clippy -D warnings` job. HEAD is green and CI runs on pushed tips, so this only bites
`git bisect`/per-commit CI. Note for the record; squashing or reordering at merge time is the
author's call. (Borderline Nit; kept Minor because the branch ships the very gate the
intermediate commit fails.)

### Nit

#### N-1 — Hook prints matched owner-PII content to stderr on a hit
`git grep -InE "$COMBINED" … >&2` echoes the matching line (the PII) to the local terminal.
Local-only, arguably useful for remediation; recorded as accepted behavior.

#### N-2 — KAT gap: nothing asserts the tracked executable bit
An optional one-line KAT (or Task-1 acceptance line) asserting
`git ls-files -s scripts/pre-push` starts with `100755` would have caught I-1 red/green and
prevents regression. Recommended alongside the I-1 fix, not required.

#### N-3 — Spec D3b "what must not change" still omits "or effectively-empty" (R0-N7, carried)
Already recorded as N7 in R0 round 3; the implementation is correct (KAT-H5c). Wording-only.

---

## Open ship obligations (in-plan Task-2/ship items — NOT findings against this diff)

1. **FOLLOWUPS amendment at ship** (spec, fold-record C1 + R0 round-3 standing condition 2):
   amend `FOLLOWUPS.md:42/193/203` M5/TUI wording to the 1.88 floor AND echo the **user's own
   confirmation lines** for the C1 decision (MSRV → 1.88, lock not downgraded) and the I4
   LICENSE carve-out. The coordinator-attested relay is still the only record at this HEAD.
2. **Stale-1.74 comment sweep** (Task 2 item 9): `crates/btctax-cli/Cargo.toml:6`,
   `crates/btctax-store/src/lock.rs:20` (comment-only), plus the M-2 resolution.
3. **Branch-protection ruleset** on `main` requiring the five checks (contexts `test`, `clippy`,
   `fmt`, `msrv`, `pii-scan` — job names verified to match), or the documented detective-only
   fallback decision.
4. **Post-merge green run:** first push triggers `ci.yml`; record the run URL in FOLLOWUPS.
5. **Operator hook install** on the dev machine (none installed at review time — verified
   `.git/hooks/pre-push` absent, `core.hooksPath` unset) — do this AFTER the I-1 fix, then
   create the real `scripts/.pii-patterns`.

---

## Disposition

**0 Critical / 1 Important / 3 Minor / 3 Nit ⇒ gate closed; NOT ready to merge.**

Everything the spec's Task-2 items 1–11 demanded was re-verified empirically at this HEAD and
passed — range semantics (including the new-ref `--not --remotes` arm re-tested in a temp repo),
fail-closed + bypass scoping, diagnostics form, KAT isolation (18/18), full-tree scan clean,
zero owner tokens in the diff, all three SHA pins independently resolved and matching, actionlint
clean, MSRV floor re-bisected (+1.85 fails / +1.88 passes), `Cargo.lock` byte-identical, and the
render.rs corollary is **ratified** as a justified, minimal Constraint-9 deviation.

The single blocker is I-1: the committed hook is not executable, so the shipped primary gate
fails open on any documented install. The fix is a two-file mode flip (no content change). Fold
I-1 (and optionally N-2), then re-review per §2 — expected to be a fast confirmation round.

---
---

# Confirmation round — I-1 fold (`ad663b1`)

**HEAD reviewed:** `ad663b1` ("fix(scripts): mark pre-push + pii-scan-generic executable
(100755)"). **Date:** 2026-07-02. Same reviewer, same notation rule.

## I-1 fold — verified CLOSED

1. **Mode-only flip, content byte-identical.** `git show ad663b1 --raw`:
   `:100644 100755 350cef8 350cef8 M scripts/pii-scan-generic.sh` and
   `:100644 100755 36ea2c7 36ea2c7 M scripts/pre-push` — blob hashes UNCHANGED on both sides,
   0 insertions / 0 deletions. `git ls-files -s` at HEAD confirms `100755` for both scripts
   (and `test-pii-hook.sh` was already `100755`). No other file touched.
2. **Fresh-clone empirical re-check (the I-1 scenario, re-run end-to-end):** cloned this repo
   into a temp workspace (checked-out modes `-rwxr-xr-x` for both scripts), applied the
   documented install (`git config core.hooksPath scripts`), pushed to a temp bare remote:
   - **(a) No patterns file:** the hook EXECUTED and the push was **blocked** with the
     fail-closed remediation text ("missing or has no patterns … See
     scripts/README-pii-setup.md. Bypass once: BTCTAX_PII_BYPASS=1 git push") — the round-1
     fail-open (git's "hook was ignored" hint + successful push) is gone.
   - **(b) Bypass + planted violation** (runtime-assembled `999·00·1234` committed in the pushed
     range): push **blocked** by the generic scan, diagnostics included the
     `rev:planted.txt:1:` file:line location [M7 holds under real push conditions].
   - **(c) Bypass + clean tip:** push **succeeded** — no false positive.
3. **No new issue from the flip:** content identical ⇒ all round-1 empirical results carry over;
   spot re-confirmed `bash scripts/pii-scan-generic.sh` at `ad663b1` → `clean (HEAD).`, exit 0.
   `Cargo.lock`, workflow, manifests untouched by `ad663b1`.

## Non-blocking findings — disposition confirmed

M-1 (report's clippy-baseline misstatement — correct the ship record), M-2 (pre-existing
real-hyphen synthetics in the gift-chunk3b review vs the Notation rule — resolve in the Task-2
hygiene pass), M-3 (transiently clippy-red `b334412` — bisection-only), and N-1/N-2/N-3 remain
as rated: none blocks merge. Route M-1/M-2/N-2 into FOLLOWUPS alongside the open ship
obligations (§Open ship obligations above), which stand unchanged — including that the FOLLOWUPS
amendment must carry the **user's own** confirmation lines for the C1 and I4 decisions
(coordinator relay is not user confirmation; this remains a ship-time obligation, not a
diff defect).

## Final disposition

**0 Critical / 0 Important / 3 Minor / 3 Nit ⇒ whole-diff review GREEN — ready to merge**
(4 commits `a1d5e26`, `b334412`, `6561313`, `ad663b1`), with the Minors/Nits and the ship
obligations recorded for FOLLOWUPS at merge time. The render.rs corollary ratification (§5)
stands.
