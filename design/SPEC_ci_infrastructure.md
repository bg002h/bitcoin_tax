# SPEC — CI infrastructure (v0.3 — R0 rounds 1 + 2 folded)

**Source baseline:** `main` @ `059f056`. Sequencing note from FOLLOWUPS: "CI infrastructure (MSRV
gate + PII scan — BEFORE the new write surface/dep)" — placed immediately after the TY2024
tables backfill, before the export-from-TUI work that introduces a new write surface.

**Goal:** Add a `.github/workflows/ci.yml` file that gates every push and pull request on five
checks — `cargo test`, `cargo clippy`, `cargo fmt`, an MSRV gate at the workspace's TRUE toolchain
floor (empirically 1.88 — see C1 fold), and a generic-shape PII scan — plus a range-scanning
`scripts/pre-push` hook (checked into the repo, owner patterns supplied via a local untracked
file) that enforces the owner-specific PII scan at the push boundary, and a branch-protection
ruleset so a red check actually blocks. All CI jobs run on Linux (`ubuntu-latest`) only;
Windows/macOS runners are deferred (see Out of scope).

**SemVer:** Infrastructure + a `rust-version` metadata bump. No production code, no struct/trait/
API change ⇒ **PATCH (chore)**. The `workspace.package.rust-version` bump (1.74 → 1.88) is a
declared-support change, not a code change; see the C1 fold below.

**Notation rule (this file is tracked content):** any digit string that would match this spec's
own SSN/EIN shape regexes is written with a middle dot `·` in place of the hyphen (e.g.
`12·3456789` denotes the 2-digits-hyphen-7-digits literal). The implementer writes the real
hyphen in the one canonical committed exclusion list (`scripts/pii-scan-generic.sh` — safe there,
because those tokens are excluded by the very list that contains them) and NOWHERE else. All
tracked prose — specs, reviews, plans — uses the `·` notation. [R0-I1.3]

---

## Fold record (round 1 — `reviews/R0-spec-ci-round-1.md`)

| Finding | Resolution in this revision |
|---------|------------------------------|
| C1 (Critical) | MSRV redesigned around the TRUE floor: **Option B — raise the declared MSRV**, do NOT downgrade `Cargo.lock`/deps. Decision made by the USER directly via an in-session structured question on 2026-07-02 ("Raise MSRV to the true floor" selected) — coordinator-attested to this author; per the R0 round-2 process note, the user's own confirmation line is echoed into FOLLOWUPS at ship. Author re-verified empirically (see Current state): `cargo +1.85.0 check --workspace --locked` FAILS (`time 0.3.51`, `time-core 0.1.9`, `time-macros 0.2.30` and the `darling`/`darling_core`/`darling_macro` 0.23.0 family require rustc 1.88.0; `instability 0.3.12` requires 1.88; `icu_*` 2.2.0 / `idna_adapter 1.2.2` require 1.86); `cargo +1.88.0 check --workspace --locked` PASSES. Floor = **1.88**. Task 1 bumps `workspace.package.rust-version` to `"1.88"`, re-confirms the floor empirically, and runs the MSRV command locally (also cures I6 for this job). FOLLOWUPS M5 language amended at ship; repo-wide stale-"1.74" grep is Task 2 hygiene. |
| I1 | Fifth synthetic EIN `99·1234567` (`crates/btctax-cli/tests/tax_report.rs:786`) added to the exclusion list with citation. ALL KAT fixture literals assembled at runtime (`printf` segments) — no shape-matching literal in any tracked file, including this spec (`·` notation adopted). Task 1 acceptance gains "the generic-shape scan exits 0 against the full tree including all files this change adds." |
| I2 | Missing `scripts/.pii-patterns` ⇒ hook **exits 1 (fail-closed)** with remediation text; explicit `BTCTAX_PII_BYPASS=1` env override for bootstrap. KAT-H5 flipped; KAT-H5b added (bypass path). |
| I3 | Hook scans the **pushed range**, not the working tree: reads the pre-push stdin ref protocol, `git rev-list remote_sha..local_sha` per ref (all-zeros remote SHA → `--not --remotes`, corrected by I7), scans every revision with `git grep`. KATs reworked to drive the hook via stdin ranges; the intermediate-commit-PII KAT is mandatory. |
| I4 | Constraint 1 restated with the enumerated **`LICENSE` carve-out** — grounded in the user's own standing rule ("only the LICENSE author name allowed", the session-local scan's `':!LICENSE'`); echoed into FOLLOWUPS at ship alongside the C1 decision. Hook gains a path-allowlist mechanism with `LICENSE` as the only entry. No relicensing. KAT-H8 added. |
| I5 | Task 2 gains a branch-protection **ruleset step** (documented `gh api` call requiring the five checks on `main`) with operator confirmation, and an explicit documented-acceptance fallback if declined. |
| I6 | ALL locally-testable verification (MSRV command, pii-scan run, hook KATs) moved into Task 1 acceptance. Post-merge confirms only the GitHub-side run. |
| M1 | Token-level filtering: `grep -oE` extracts matched tokens; exclusions filter tokens, not whole lines. |
| M2 | ONE canonical committed script `scripts/pii-scan-generic.sh` holds the shapes + exclusion list; both the CI job and the hook call it. |
| M3 | Scanning is `git grep -I` against commit trees (tree-accurate, NUL-safe by construction, binaries skipped); no `ls-files | xargs` piping; grep exit statuses propagated (1 = clean, >1 = error). `legal/` PDF limitation documented: compressed streams are outside the regex gate's reach — control is provenance (public IRS documents only). |
| M4 | Top-level `permissions: contents: read` added to the workflow. |
| M5 | `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}` — bursts never cancel a `main` run. |
| M6 | Recon errata corrected (three crates, `crates/` path prefixes, root `.gitignore` stated as fact, `BTCTAX_PASSPHRASE` wording). |
| N1 | push + pull_request double-run on PR branches: accepted (cost-only, single-user repo); noted in D1. |
| N2 | Fabricated example SHAs removed from D4; placeholders only, with an explicit "never copy examples" warning. |
| N3 | bash ≥ 4 requirement (`mapfile`) noted in `README-pii-setup.md` for the future macOS leg. |
| N4 | KAT temp repos commit fixtures before any `git grep`-based scan; KATs drive the hook via the stdin ref protocol (subsumed by the I3 rework). |
| N5 | Commit authorship carries the owner's real email — outside the tracked-content invariant, mitigated by the private repo; recorded here as a **conscious exclusion**, not an oversight. |

## Fold record (round 2 — `reviews/R0-spec-ci-round-1.md`, Round 2 section)

| Finding | Resolution in this revision |
|---------|------------------------------|
| I7 (Important) | The new-ref arm scanned NOTHING: `--not --all` excludes the very branch being pushed (its tip IS `local_sha`) → empty rev set on every new-branch push. Fixed in D3b (semantics item 1 + design shape): `git rev-list <local_sha> --not --remotes` — commits not reachable from any remote-tracking ref, i.e. not yet public. Author re-verified empirically (temp repo, 2 commits, new local branch): `--not --all` → 0 revs; `--not --remotes` → 2 revs; scoped `--not --remotes=origin` with no such remote → 2 revs (non-matching glob excludes nothing — over-scan, fails safe). The optional sharpening `--remotes="$1"` (the hook's first argument is the remote name) is documented with plain `--remotes` as the fallback. KAT-H4 unchanged — now satisfiable (temp repos have no remote-tracking refs). |
| M7 | D3a second-pass diagnostics loop had an argument-order bug (`-- "$tok" "$REV"` parses both as pathspecs → blank output). Fixed: `git grep -InF -e "$tok" "$REV" --`. KAT-G1 gains an assertion that the failure output contains a `file:line` location for the offending token. |
| M8 | KAT isolation mechanism now explicit in D3c: the harness COPIES `pre-push` + `pii-scan-generic.sh` into each temp workspace and runs the copies; the operator's real `scripts/.pii-patterns` is never read, written, or deleted by any KAT (in-place runs would break KAT-H5 on machines with a real patterns file and risk clobbering it in KAT-H6). |
| M9 | A present-but-EMPTY patterns file (zero non-comment lines) is treated the same as a missing one: fail-closed exit 1 + the same `BTCTAX_PII_BYPASS=1` path (closes the `touch`-half-setup fail-open shape). KAT-H5c added. |
| N6 | The 1.88-floor evidence list completed: `darling`/`darling_core`/`darling_macro` 0.23.0 also require rustc 1.88.0 (Current state + zero-headroom note — a future `time` upgrade alone would not lower the binding constraint). |

**Round-2 process note (blocking for SHIP, not for this fold):** the C1 and I4 decisions are
recorded above as user decisions per the coordinator's attestation (C1: in-session structured
question, "Raise MSRV to the true floor" selected, 2026-07-02; I4: the user's original standing
rule). Per the R0's process caveat, the user's own confirmation line is echoed into FOLLOWUPS at
ship; implementation prep may proceed against this revision at the author's risk before then.

---

## Current state (recon @ `059f056`, errata folded)

- **No `.github/` directory exists.** Confirmed by `ls`; no CI workflow runs on push or PR today.
- **No `rust-toolchain` or `rust-toolchain.toml` file exists.** Toolchain selection is implicit.
- **`workspace.package.rust-version = "1.74"`** in `Cargo.toml` (line 7) — **aspirational, not a
  property of the build** (see next bullet). Applied via `rust-version.workspace = true` in
  `crates/btctax-tui/Cargo.toml:5` and `crates/btctax-cli/Cargo.toml:6`. The other **three**
  crates (`btctax-store`, `btctax-core`, `btctax-adapters`) do not carry the field.
- **The TRUE toolchain floor of the committed lock is 1.88** [R0-C1, author-verified 2026-07-02]:
  - `Cargo.lock` is **`version = 4`** (line 3) — lockfile v4 requires Cargo ≥ 1.78; cargo 1.74
    cannot even parse it.
  - `cargo +1.85.0 check --workspace --locked` FAILS: `time v0.3.51`, `time-core v0.1.9`,
    `time-macros v0.2.30` AND `darling`/`darling_core`/`darling_macro` v0.23.0 [R0-N6] declare
    `rust-version = 1.88.0`; `instability v0.3.12` declares 1.88;
    `icu_properties_data`/`icu_provider`/`idna_adapter` v2.2.x/1.2.2 declare 1.86;
    `zeroize v1.9.0` is edition-2024.
  - `cargo +1.88.0 check --workspace --locked` PASSES (all five crates, `Finished dev profile`).
  - Floor = **1.88 exactly**, with **zero headroom**: BOTH the `time` 0.3.51 family and the
    `darling` 0.23.0 family require precisely 1.88.0 — upgrading `time` alone would not lower
    the binding constraint [R0-N6]. (The previous draft's "ratatui 0.29 zero-headroom at 1.74"
    rationale is superseded: ratatui 0.29's MSRV of 1.74 now has 14 minor versions of headroom.
    The gate's purpose is unchanged — catch any future lock change that raises the floor above
    the declared `rust-version`.)
- **`Cargo.lock` is committed** (v4 format). `ratatui 0.29.0` + `crossterm 0.28.1` pinned therein.
- **Test suite is hermetic.** `grep` over all crates confirms: no `reqwest`, `ureq`, `hyper`,
  `tokio`, `async-std`, or any other network/async dep in `[dependencies]` or `[dev-dependencies]`
  of any crate. `BTCTAX_PASSPHRASE` is the only env-var seam: five integration tests supply it
  via `.env("BTCTAX_PASSPHRASE", "pw")` on their `Command` builders, and one TUI unit test
  (`crates/btctax-tui/src/unlock.rs:434`) uses in-process `std::env::set_var` (mutex-serialized).
  Either way: no CI-level secret or env configuration is required.
- **No `scripts/` directory exists.** No pre-push hook is installed (`.git/hooks/` has samples only).
- **Root `.gitignore` exists** and already opens with a strong PII banner ("NEVER commit
  personal/tax data or secrets"), ignoring `*.pgp`/`vault*`/etc. The new ignore lines (D5) join an
  existing PII-focused file.
- **Synthetic PII-shaped values in the test suite** (`·` = hyphen; all confirmed by grep, and the
  set below was verified COMPLETE by running the generic-shape pipeline against the full tree
  [R0-I1]):
  - `987·65·4321` — SSA-reserved never-issued SSN: `crates/btctax-core/src/donation.rs:94`,
    `crates/btctax-core/tests/kat_forms.rs:1102`.
  - `12·3456789` — sequential synthetic EIN: `crates/btctax-core/src/donation.rs:91`,
    `crates/btctax-cli/src/cmd/reconcile.rs:666`, `crates/btctax-cli/src/donation_details.rs:110`,
    `crates/btctax-cli/src/render.rs:2926,2970`, `crates/btctax-core/tests/kat_forms.rs:1099`.
  - `99·1234567` — second synthetic sequential EIN: `crates/btctax-cli/tests/tax_report.rs:786`
    [R0-I1 residual hit — now excluded with this citation].
  - `987654321` (bare 9 digits) and `P01234567` (alphanumeric PTIN) — synthetic, but they match
    NEITHER shape regex (no hyphens / alphanumeric), so they need **no exclusion entry**; listed
    here for documentation only.
- **`LICENSE` line 3 (MIT copyright line) contains the owner's legal name** [R0-I4]. This is the
  one deliberate, user-accepted exception to the no-PII-in-tracked-content constraint (MIT
  requires a copyright holder; the workspace declares `license = "MIT OR Unlicense"`). The
  session-local PII scan has always run with `':!LICENSE'`; the hook mirrors that.
- **FOLLOWUPS references:** the TUI shipped entry defers "CI infra (no `.github/workflows`
  exists — add one, incl. the MSRV gate [M5] and the PII scan)". Its "cargo +1.74" wording is
  superseded by the C1 floor finding; FOLLOWUPS is amended at ship. Store FOLLOWUPS M-3 defers
  "verify under Windows CI that the written files are not world-readable" — confirming Windows CI
  is a future item, not this spec.

---

## Constraints

1. **No PII in tracked content — with ONE enumerated carve-out.** No owner-identifying literal
   (name, account number, wallet address fragment, or any regex that would match owner-specific
   data) may appear in any committed file — including the workflow YAML, the hook script, the
   shared scan script, and all tracked prose (this spec included; see the Notation rule).
   **Carve-out (user standing rule):** the copyright-holder line in `LICENSE` is a deliberate,
   accepted exception; the hook's owner-specific scan excludes exactly that path and nothing else.
2. **No network in tests — confirmed.** The suite is hermetic (no network deps); CI must not add
   any step that requires outbound access beyond `crates.io` resolution (constrained by `--locked`).
3. **Pinned actions by full commit SHA.** Supply-chain posture for a security-sensitive repo; tag
   aliases (`@v4`) are NOT acceptable (tags are mutable). Each action is pinned to a full
   40-hex-character commit SHA resolved at implementation time.
4. **No third-party actions beyond the minimal vetted set.** Only `actions/checkout`,
   `dtolnay/rust-toolchain`, and `Swatinem/rust-cache` are permitted. No cargo-audit/deny or any
   phone-home scanner (deferred to FOLLOWUPS).
5. **`--locked` on every compiling `cargo` invocation.** The committed `Cargo.lock` is the
   authoritative resolution. (`cargo fmt` does not compile and does not take `--locked`.)
6. **Linux-only for this spec.** All jobs run on `ubuntu-latest`. Windows/macOS deferred.
7. **No `rust-toolchain.toml` added by this spec.** The workflow installs toolchains explicitly.
8. **Least privilege:** the workflow declares top-level `permissions: contents: read` [R0-M4].
9. **Scope of file changes:** `Cargo.lock` and all crate source/test files are NOT modified. The
   workspace `Cargo.toml` IS modified in exactly one field (`rust-version` 1.74 → 1.88, the C1
   fold). FOLLOWUPS M5 wording is amended at ship.

---

## Design

### D1 — Workflow file: `.github/workflows/ci.yml`

**Triggers:** `push` on all branches + `pull_request`. A PR branch push runs twice (push + PR
event) — accepted, cost-only in a single-user repo [R0-N1].

**Top-level hardening:** `permissions: contents: read` [R0-M4].

**Concurrency:**
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```
Bursts to feature branches cancel superseded runs; pushes to `main` never cancel the run the
Definition of Done needs recorded [R0-M5].

**Five jobs, all on `ubuntu-latest`:**

---

**Job `test` — `cargo test --workspace --locked`**

```yaml
steps:
  - uses: actions/checkout@<40-hex-SHA>        # pin resolved at implementation time
  - uses: dtolnay/rust-toolchain@<40-hex-SHA>
    with:
      toolchain: stable
  - uses: Swatinem/rust-cache@<40-hex-SHA>
  - run: cargo test --workspace --locked
```

No special env vars: `BTCTAX_PASSPHRASE` is supplied inside the tests themselves (Command
builders / in-process set_var), never at CI level.

---

**Job `clippy` — `cargo clippy --workspace --all-targets --locked -- -D warnings`**

```yaml
steps:
  - uses: actions/checkout@<40-hex-SHA>
  - uses: dtolnay/rust-toolchain@<40-hex-SHA>
    with:
      toolchain: stable
      components: clippy
  - uses: Swatinem/rust-cache@<40-hex-SHA>
  - run: cargo clippy --workspace --all-targets --locked -- -D warnings
```

`--all-targets` covers lib + bins + tests + benches + examples; `-D warnings` promotes all
Clippy warnings to errors; `components: clippy` is explicit in case the cached toolchain lacks it.

---

**Job `fmt` — `cargo fmt --all -- --check`**

```yaml
steps:
  - uses: actions/checkout@<40-hex-SHA>
  - uses: dtolnay/rust-toolchain@<40-hex-SHA>
    with:
      toolchain: stable
      components: rustfmt
  - run: cargo fmt --all -- --check
```

No cache step (fmt does not compile).

---

**Job `msrv` — MSRV gate at the true floor (1.88)** [R0-C1]

```yaml
steps:
  - uses: actions/checkout@<40-hex-SHA>
  - uses: dtolnay/rust-toolchain@<40-hex-SHA>
    with:
      toolchain: "1.88"
  - uses: Swatinem/rust-cache@<40-hex-SHA>
    with:
      key: msrv-1.88
  - run: cargo check --workspace --locked
```

`dtolnay/rust-toolchain` installs 1.88.x and sets it as the job default, so no `+1.88` override
is needed on the invocation. The `key: msrv-1.88` cache suffix separates the 1.88 artifacts from
the stable cache. `--locked` is the point of the job: the committed `Cargo.lock` must resolve AND
type-check under the declared `rust-version`.

**Zero-headroom note (re-derived at the new floor):** the committed lock's `time 0.3.51` /
`time-core 0.1.9` / `time-macros 0.2.30` AND `darling`/`darling_core`/`darling_macro 0.23.0`
[R0-N6] require rustc **exactly 1.88.0** — zero headroom, and two independent binding families
(upgrading one alone does not lower the floor). Any future `cargo update` that pulls a dep
requiring > 1.88 fails this job, which is the intended gate behavior: the failure forces a
conscious decision (bump `rust-version` + this job's toolchain + cache key in lockstep, or pin
the dep back).

`cargo check` (not `build`) is used for speed; it verifies the dependency tree and source
type-check under 1.88 without producing binaries.

If Task 1's re-verification finds a different floor than 1.88 (e.g. a dep changed between spec
and implementation), the empirically-confirmed floor wins: `rust-version`, the job's `toolchain:`,
and the cache key are all set to it, and this spec's figure is corrected in the same commit.

---

**Job `pii-scan` — generic-shape PII scan (calls the ONE canonical script)** [R0-M2]

```yaml
steps:
  - uses: actions/checkout@<40-hex-SHA>
  - name: PII generic-shape scan
    run: bash scripts/pii-scan-generic.sh
```

The workflow itself contains **no digit literals and no regexes** — the shapes and the exclusion
list live in exactly one committed place, `scripts/pii-scan-generic.sh` (D3a). The script scans
the checked-out tip (`HEAD`); the default shallow checkout suffices.

**Scope note:** the CI scan covers the tip tree only. Intermediate-commit history is the hook's
job (D3b) — CI cannot retroactively vet commits already pushed; the hook runs before they leave
the machine.

**What is NOT scanned by CI:** owner-specific PII (name, exchange account IDs, wallet address
fragments) — the most dangerous patterns for this repo. They are handled exclusively by the local
pre-push hook (D3b). CI's generic-shape scan is a defense-in-depth layer; the hook is the primary
gate.

---

### D2 — PII mechanism recommendation and rationale

**Recommended: Option (c) — the local pre-push hook AND a CI generic-shape scan.**

Three options were evaluated:

**(a) GitHub Actions SECRET (`PII_PATTERNS`):** the workflow greps with the secret when set and
skips-with-warning when absent (fork/PR contexts have no secrets). Weakness for a single-user
private repo: if the owner forgets to (re)configure the secret, CI silently skips the
owner-specific scan forever — a gate that quietly isn't one.

**(b) Local hook only + CI generic shapes:** the hook enforces owner-specific patterns at push
time (now fail-closed and range-scanning — I2/I3 folds); CI adds independent generic-shape
defense-in-depth.

**(c) Both = (b) now, (a) as an optional future enhancement.**

**Recommendation: (c), implemented as (b), with (a) documented in FOLLOWUPS.** The repo is
private and single-user; the owner is the only pusher. The fail-closed hook is the tightest gate
(it now refuses to run without its patterns file — I2). CI's generic scan catches SSN/EIN-shaped
digits even when the hook is bypassed. Option (a) adds GitHub-side configuration and rotation
burden for marginal benefit; it remains available later without re-architecture.

**INVARIANT (maintained at implementation and at every future change):** no literal PII
pattern — no owner name, no account number, no wallet address, no exchange-account ID, no regex
designed to match those values — ever appears in any file tracked by git, **including this spec
and all reviews/plans** (shape-matching digit examples use the `·` notation; KAT fixtures are
assembled at runtime). The single enumerated exception is the `LICENSE` copyright-holder line
(Constraint 1 carve-out). This applies to:
- `.github/workflows/ci.yml` — contains no regexes or digit literals at all (calls the script).
- `scripts/pii-scan-generic.sh` — generic shapes + the documented synthetic-token exclusion list
  only; nothing owner-specific.
- `scripts/pre-push` — hook framework with ZERO owner-specific patterns; reads patterns from the
  local untracked `scripts/.pii-patterns`.
- `scripts/README-pii-setup.md` — format documentation; no patterns.
- Any future workflow or script file — same rule, unconditionally.

---

### D3a — Canonical generic-shape scan: `scripts/pii-scan-generic.sh` [R0-M1/M2/M3]

ONE committed script owns the shape regexes and the exclusion list; the CI job and the hook both
call it. Interface: `pii-scan-generic.sh [<rev>]` — scans the tree of `<rev>` (default `HEAD`)
via `git grep`; exits 0 = clean, 1 = hit(s) found (locations printed to stderr), 2 = scan error.

**Design shape** (implementer refines mechanics; the invariants are binding). NOTE: per the
Notation rule, the `·` below denotes a real hyphen in the committed script:

```sh
#!/usr/bin/env bash
# Generic-shape PII scan — SSN-like (3·2·4 digits) and EIN-like (2·7 digits) tokens.
# The ONLY place the shapes and the exclusion list exist (CI job + pre-push hook both call this).
set -euo pipefail
REV="${1:-HEAD}"

# ── Shapes (ERE; hyphen-delimited digit groups) ──────────────────────────────
SHAPES='\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b|\b[0-9]{2}-[0-9]{7}\b'

# ── Exclusion list: documented synthetic test stand-ins (token-exact) ───────
# If a new synthetic shape-matching value enters the test suite, it MUST be added
# here with a citation comment.
#   987·65·4321  — SSA-reserved never-issued SSN (donation.rs:94, kat_forms.rs:1102)
#   12·3456789   — sequential synthetic EIN (donation.rs:91, reconcile.rs:666,
#                  donation_details.rs:110, render.rs:2926/2970, kat_forms.rs:1099)
#   99·1234567   — second synthetic sequential EIN (tests/tax_report.rs:786)
# NOT excluded (cannot match the shapes; documented only): the bare 9-digit TIN
# and the alphanumeric PTIN used in the same fixtures.
ALLOWED='^(987·65·4321|12·3456789|99·1234567)$'     # ← real hyphens in the script

# Token-level extraction [M1]: -o emits only matched tokens; filter tokens, not lines.
# -I skips binaries [M3]; git grep is tree-accurate and NUL-safe by construction.
set +e
tokens=$(git grep -IhoE "$SHAPES" "$REV" -- | sort -u)
gs=$?
set -e
[ "$gs" -gt 1 ] && { echo "pii-scan: git grep failed (status $gs)" >&2; exit 2; }

bad=$(printf '%s\n' "$tokens" | grep -vE "$ALLOWED" | grep -v '^$' || true)
if [ -n "$bad" ]; then
  echo "pii-scan: non-excluded PII-shaped token(s) in $REV:" >&2
  # Second pass for actionable file:line locations of exactly the bad tokens.
  # [R0-M7] `-e "$tok"` BEFORE the rev, pathspec separator last — putting the token
  # after `--` would parse it as a pathspec and blank the diagnostics.
  while IFS= read -r tok; do git grep -InF -e "$tok" "$REV" -- >&2 || true; done <<<"$bad"
  exit 1
fi
echo "pii-scan: clean ($REV)."
```

**Binding invariants:** (1) token-level filtering — a line containing both an excluded synthetic
token and a real PII-shaped token is still flagged [M1]; (2) `git grep -I` against a rev — no
working-tree dependence, binaries skipped [M3]; (3) grep exit statuses propagated — status > 1 is
a scan error, never silently swallowed [M3]; (4) the exclusion list exists here and nowhere else
[M2].

**Word-boundary note:** `\b` support in `git grep -E` must be verified at implementation (it is a
GNU extension); if unreliable, switch to `git grep -P` (PCRE2 — standard in distro git builds;
Linux-only scope makes this safe). KAT-G1/G2 (below) catch a wrong choice red/green.

**Known limitation (documented, accepted):** tracked binary documents (the `legal/` PDF archive)
are compressed streams — regex scanning cannot see their decompressed content, and `-I` skips
them as binaries. The control for binary documents is **provenance** (public IRS/court documents
only), not the regex gate.

---

### D3b — Local pre-push hook: `scripts/pre-push` [R0-I2/I3/I4]

**Files created:**
- `scripts/pre-push` — the hook script (committed, executable). Contains ZERO patterns.
- `scripts/pii-scan-generic.sh` — the canonical generic scan (D3a).
- `scripts/test-pii-hook.sh` — shell KATs (fixtures assembled at runtime — never literal).
- `scripts/.gitignore` — ignores `scripts/.pii-patterns` and `scripts/.pii-patterns.bak`.
- `scripts/README-pii-setup.md` — install guide + patterns-file format; no patterns; notes the
  bash ≥ 4 requirement (`mapfile`) for the future macOS leg [R0-N3].

`scripts/.pii-patterns` is NEVER committed; it exists only on the operator's machine.

**Install** (documented in the README): `ln -s ../../scripts/pre-push .git/hooks/pre-push`, or
the lower-friction `git config core.hooksPath scripts` (the script is named exactly `pre-push`).

**Semantics (binding):**

1. **Range scan, not working tree [I3].** The hook consumes the standard pre-push stdin protocol
   (`<local_ref> <local_sha> <remote_ref> <remote_sha>` per line). For each pushed ref:
   - local SHA all-zeros (ref deletion) → nothing to scan, skip;
   - remote SHA all-zeros (new ref) → revs = `git rev-list <local_sha> --not --remotes`
     [R0-I7] — commits not reachable from any remote-tracking ref, i.e. not yet known to be
     public. (NOT `--not --all`: `--all` includes the very branch being pushed, whose tip IS
     `local_sha`, excluding everything — the new-ref arm would scan zero commits. Verified
     empirically: 0 revs vs the correct 2 in a two-commit temp repo.) Optional sharpening:
     `--not --remotes="$1"` scoped to the target remote (the hook's `$1` is the remote name),
     with plain `--remotes` as the documented fallback — a non-matching glob excludes nothing,
     so the hook over-scans, which fails safe;
   - otherwise → revs = `git rev-list <remote_sha>..<local_sha>`.
   **Every revision in the range is scanned** — a commit that adds PII and a later commit that
   removes it must still fail the push (history, once pushed, cannot be unpushed).
2. **Owner-specific scan (primary gate, fail-closed [I2/M9]).** If `scripts/.pii-patterns` is
   missing **or effectively empty (zero non-comment, non-blank lines)** [R0-M9 — a bare `touch`
   half-setup must not silently degrade the primary gate to generic-only]: print remediation
   text pointing at `scripts/README-pii-setup.md` and **exit 1** — unless `BTCTAX_PII_BYPASS=1`
   is set, which downgrades the missing/empty-file error to a warning (deliberate,
   per-invocation bootstrap escape: `BTCTAX_PII_BYPASS=1 git push`). The bypass affects ONLY the
   missing/empty-file check; the generic-shape scan still runs.
   If the file is present with patterns: build one combined ERE from its non-comment lines and
   run `git grep -InE "$COMBINED" <rev> -- ':(exclude)LICENSE'` for each rev in the range.
3. **Path allowlist [I4]:** hardcoded in the script, `LICENSE` the ONLY entry, applied ONLY to
   the owner-specific scan (the generic shapes don't match a name; no carve-out needed there).
   Documented in the README. Growing the allowlist is a spec-level change, not an edit-in-place.
4. **Generic-shape scan:** per rev in the range, call `scripts/pii-scan-generic.sh <rev>` (D3a).
   One list, no drift [M2].
5. Any hit in any rev ⇒ exit 1 with rev + file:line locations. All-clean ⇒ exit 0.

**Design shape:**

```sh
#!/usr/bin/env bash
# pre-push hook — PII gate (owner-specific + generic shape) over the PUSHED RANGE.
# Install: ln -s ../../scripts/pre-push .git/hooks/pre-push
#      or: git config core.hooksPath scripts
# Patterns: scripts/.pii-patterns (local, untracked, NEVER commit).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATTERNS_FILE="$SCRIPT_DIR/.pii-patterns"
ZERO=0000000000000000000000000000000000000000
EXIT=0

# ── Fail-closed patterns-file check [I2 + M9: empty == missing] ─────────────
COMBINED=""
if [ -f "$PATTERNS_FILE" ]; then
  mapfile -t PATTERNS < <(grep -vE '^\s*(#|$)' "$PATTERNS_FILE")
  [ ${#PATTERNS[@]} -gt 0 ] && COMBINED=$(IFS='|'; echo "${PATTERNS[*]}")
fi
if [ -z "$COMBINED" ]; then                                  # missing OR empty [M9]
  if [ "${BTCTAX_PII_BYPASS:-0}" = "1" ]; then
    echo "pre-push WARNING: $PATTERNS_FILE missing/empty; owner-specific scan SKIPPED (bypass)." >&2
  else
    echo "pre-push: $PATTERNS_FILE missing or has no patterns — owner-specific PII scan cannot run." >&2
    echo "See scripts/README-pii-setup.md.  Bypass once: BTCTAX_PII_BYPASS=1 git push" >&2
    exit 1
  fi
fi

# ── Pushed-range scan [I3] ──────────────────────────────────────────────────
while read -r local_ref local_sha remote_ref remote_sha; do
  [ "$local_sha" = "$ZERO" ] && continue                     # deletion — skip
  if [ "$remote_sha" = "$ZERO" ]; then
    revs=$(git rev-list "$local_sha" --not --remotes)        # new ref [I7 — never --all]
  else
    revs=$(git rev-list "$remote_sha..$local_sha")
  fi
  for rev in $revs; do
    if [ -n "$COMBINED" ]; then                              # owner-specific [I4 allowlist]
      if git grep -InE "$COMBINED" "$rev" -- ':(exclude)LICENSE' >&2; then
        echo "pre-push: OWNER-SPECIFIC PII pattern in $rev (see above)." >&2; EXIT=1
      fi
    fi
    bash "$SCRIPT_DIR/pii-scan-generic.sh" "$rev" || EXIT=1  # generic shapes [M2]
  done
done
exit $EXIT
```

The above is the DESIGN SHAPE; the implementer refines mechanics (git grep status handling — 0 =
match = finding, 1 = clean, >1 = error — must be propagated correctly, including inside the
`if git grep …` condition and the `|| EXIT=1` on the generic call, where a status-2 scan error
must abort with a distinct message rather than masquerade as a finding). What must not change:
no pattern in the script itself; fail-closed on a missing patterns file; every rev in the pushed
range scanned; `LICENSE` the only allowlisted path; exit 1 on any hit.

**Patterns file format (`scripts/.pii-patterns`, local only):**
```
# One ERE pattern per line.  Lines starting with # and blank lines are ignored.
# DO NOT put real values in any committed file — this file is untracked and local.
```
The README documents the format; it contains no example that could be a real pattern.

---

### D3c — Testable shell KATs: `scripts/test-pii-hook.sh` [R0-I1.2/I3/N4]

All KATs run in temp git repos isolated from the real repo. **Isolation mechanism [R0-M8]: the
harness COPIES `pre-push` and `pii-scan-generic.sh` into each temp workspace and runs the
copies — never the in-place scripts.** (The hook resolves `PATTERNS_FILE` relative to its own
location, so an in-place run would break KAT-H5/H5c on any machine where the operator has a real
`scripts/.pii-patterns`, and KAT-H6 would have to write into the real `scripts/` directory.)
The operator's real `scripts/.pii-patterns` is never read, written, or deleted by any KAT.
**Every shape-matching fixture string is assembled at runtime** — e.g.
`tok=$(printf '%s-%s-%s' 999 00 1234)` — so NO shape-matching literal exists in the committed
test script (or anywhere tracked). Fixtures are committed inside the temp repo before scanning
(git grep scans commit trees). The hook is driven via its stdin ref protocol
(`printf 'refs/heads/x %s refs/heads/x %s\n' "$tip" "$base" | bash <copied>/pre-push`), not by
mutating a working tree.

- **KAT-G1 (generic — SSN-shaped hit):** temp repo; commit a file containing the runtime-assembled
  3·2·4 token (non-excluded); `pii-scan-generic.sh <tip>` exits 1 **and the failure output
  contains a `file:line` location for the offending token** [R0-M7 — locks the second-pass
  diagnostics in red/green].
- **KAT-G2 (generic — EIN-shaped hit):** same with a runtime-assembled 2·7 token; exits 1.
- **KAT-G3 (generic — excluded tokens):** file contains only the three excluded synthetics
  (runtime-assembled); exits 0.
- **KAT-G4 (generic — mixed line) [M1]:** ONE line contains an excluded synthetic AND a
  non-excluded shaped token; exits 1 (token-level filtering, not line-level).
- **KAT-H1 (hook — hit at tip):** range `base..tip` where tip adds a shaped token; hook exits 1.
- **KAT-H2 (hook — clean range):** hook exits 0.
- **KAT-H3 (hook — INTERMEDIATE commit) [I3, the one that matters]:** commit A adds a shaped
  token, commit B removes it; push range includes both; working tree clean; hook exits 1.
- **KAT-H4 (hook — new ref, all-zeros remote SHA):** remote SHA = 40 zeros; the
  `--not --remotes` path is taken (temp repos have no remote-tracking refs, so the full history
  is scanned [I7]); hit found; exits 1.
- **KAT-H5 (hook — missing patterns file, fail-closed) [I2]:** no `.pii-patterns`; hook exits 1
  with the remediation message.
- **KAT-H5b (hook — bypass) [I2]:** no `.pii-patterns`, `BTCTAX_PII_BYPASS=1`; hook proceeds AND
  the generic scan still runs (a generic-shaped fixture in the range still fails with exit 1; a
  clean range exits 0).
- **KAT-H5c (hook — present-but-empty patterns file, fail-closed) [M9]:** a `.pii-patterns`
  containing only comments/blank lines; hook exits 1 with the same remediation message;
  `BTCTAX_PII_BYPASS=1` downgrades it identically to the missing-file case.
- **KAT-H6 (owner-specific — hit):** temp `.pii-patterns` with a synthetic pattern (e.g.
  `SYNTHETIC-OWNER-[0-9]+`); a range commit contains a matching string; exits 1.
- **KAT-H7 (owner-specific — miss):** same patterns; no match in range; exits 0.
- **KAT-H8 (LICENSE carve-out) [I4]:** owner pattern matching content that exists ONLY in the
  temp repo's `LICENSE` ⇒ exit 0; the same content also in another file ⇒ exit 1.
- **KAT-H9 (deleted ref):** stdin line with all-zeros local SHA; skipped; exit 0.

---

### D4 — Action version pinning [R0-N2]

At implementation time, resolve the CURRENT commit SHA for each of the three permitted actions
(GitHub UI or `gh api repos/<owner>/<repo>/git/ref/tags/<tag>`) and pin the full 40-hex SHA, with
a same-line comment naming the human-readable tag for auditing:

```yaml
- uses: actions/checkout@<40-hex-SHA>         # <tag resolved at implementation time>
```

**The placeholders above are NOT real pins. This spec deliberately contains no concrete SHAs —
never copy an example SHA from any document; resolve each pin fresh at implementation time.**
When an action is later updated, the SHA and the comment tag change in the same commit.

---

### D5 — `.gitignore` additions

The root `.gitignore` exists and already opens with a PII banner (fact, verified at `059f056`).
Append:

```
# CI/scripts: local PII-patterns file (never commit)
scripts/.pii-patterns
scripts/.pii-patterns.bak
```

Verify via `git check-ignore -v scripts/.pii-patterns`. `scripts/.gitignore` carries the same two
entries as defense-in-depth for path-relative tooling.

---

### D6 — Workspace MSRV bump [R0-C1]

`Cargo.toml` line 7: `rust-version = "1.74"` → `rust-version = "1.88"` (or the Task-1-confirmed
floor if it differs — the empirical result wins). This is the ONLY manifest change. `Cargo.lock`
is NOT modified; dependency versions are NOT downgraded (user decision, Option B). Crates
inheriting `rust-version.workspace = true` (`btctax-cli`, `btctax-tui`) pick the new floor up
automatically; the stale "(1.74)" comment in `crates/btctax-cli/Cargo.toml:6` and other prose
mentions are Task 2 hygiene (comment-only, no logic change).

---

## Plan (TDD / KAT-first)

### Task 1 — Implement + ALL locally-testable verification [R0-I6]

**Files created/changed (no crate source, no test file, no `Cargo.lock`):**

| File | Change |
|------|--------|
| `.github/workflows/ci.yml` | NEW — five-job workflow |
| `scripts/pii-scan-generic.sh` | NEW — canonical generic scan (shapes + exclusions, the only copy) |
| `scripts/pre-push` | NEW — range-scanning fail-closed hook (no patterns) |
| `scripts/test-pii-hook.sh` | NEW — shell KATs (runtime-assembled fixtures only) |
| `scripts/.gitignore` | NEW — ignores `.pii-patterns` |
| `scripts/README-pii-setup.md` | NEW — install + format guide (no patterns; bash ≥ 4 note) |
| `.gitignore` | append 2 ignore lines (D5) |
| `Cargo.toml` | `rust-version` `"1.74"` → `"1.88"` (D6, sole manifest change) |

**Implementation order (TDD — local-testable first):**

1. **Confirm the MSRV floor empirically** (re-verification of the C1 finding at implementation
   HEAD): `cargo +1.85.0 check --workspace --locked` must fail, `cargo +1.88.0 check --workspace
   --locked` must pass (toolchains are installed locally). If the floor moved, use the measured
   value everywhere "1.88" appears. Then apply the D6 `rust-version` bump.
2. **Write `scripts/test-pii-hook.sh` first**, then `scripts/pii-scan-generic.sh` and
   `scripts/pre-push` until all KATs (G1–G4, H1–H9 incl. H5b/H5c) pass: red → green.
3. **Create `scripts/.gitignore`, `README-pii-setup.md`, and the root `.gitignore` lines.**
4. **Create `.github/workflows/ci.yml`.** Run `actionlint` locally if installed; otherwise a
   careful line-by-line schema review. Resolve and pin the three action SHAs (D4).

**Task 1 acceptance criteria (ALL must pass locally before Task 2):**

- `bash scripts/test-pii-hook.sh` exits 0, all KATs PASS.
- **`bash scripts/pii-scan-generic.sh` exits 0 against the full tracked tree INCLUDING every file
  this change adds** (run against a local commit of the complete change) [R0-I1.4]. This
  includes this spec file and the round-1 review — both use the `·` notation and must scan clean.
- **`cargo +1.88 check --workspace --locked` passes locally** (the exact MSRV-job command)
  [R0-C1/I6] — and `cargo +1.85.0 check --workspace --locked` fails (floor confirmation).
- `git check-ignore -v scripts/.pii-patterns` confirms the ignore rule.
- `actionlint .github/workflows/ci.yml` exits 0 (if available) OR documented line-by-line review.
- Every `uses:` line carries a 40-hex SHA + tag comment; no tag aliases.
- Manual read-through of every added file confirms zero owner-specific content (Constraint 1).
- `cargo test --workspace --locked` passes; test count unchanged at 692 (no code touched).

---

### Task 2 — Whole-diff review (Phase E) + ruleset + post-merge verification

**Cross-cutting items the whole-diff R0 must verify:**

1. **MSRV coherence [C1]:** `Cargo.toml` `rust-version`, the workflow's `toolchain:`, and the
   cache key all name the same empirically-confirmed floor; the Task-1 pass/fail evidence
   (+1.85 fails / +1.88 passes) is recorded; `Cargo.lock` is byte-identical to `main`.
2. **Workflow semantics:** job names, `permissions: contents: read`, the conditional
   `cancel-in-progress`, no cache on `fmt`, `--locked` on every compiling invocation.
3. **SHA pinning:** all three `uses:` pins are real 40-hex SHAs resolved this cycle (not copied
   from any document), tag comments accurate.
4. **PII invariant:** full read of `ci.yml`, both scripts, the README, and this spec — zero
   owner-specific content; zero shape-matching literals outside the canonical exclusion list;
   KAT fixtures runtime-assembled; the `LICENSE` carve-out is the only path exception.
5. **Exclusion list vs current source:** the three excluded tokens match the live tree (re-grep
   at review time — the set can drift during the cycle); the bare-TIN/PTIN no-exclusion-needed
   analysis re-confirmed (R0 round 1 already verified it independently).
6. **Range-scan correctness [I3/I7]:** stdin protocol parsing, all-zeros handling both
   directions, `rev-list` bounds; the new-ref arm uses `--not --remotes` (NEVER `--not --all`);
   KAT-H3 (intermediate commit) and KAT-H4 (new ref) re-run by the reviewer.
7. **Fail-closed behavior [I2/M9]:** KAT-H5/H5b/H5c re-run (missing AND present-but-empty
   patterns file both fail closed); bypass leaves the generic scan active.
8. **Hermeticity:** no new `[dependencies]` anywhere in the diff.
9. **Stale-"1.74" hygiene sweep:** grep the repo for MSRV-1.74 mentions
   (`crates/btctax-cli/Cargo.toml:6` comment, FOLLOWUPS M5/TUI entry, the store fs2 note, the TUI
   spec) and update comment/prose sites to the new floor — doc-only, no logic change. FOLLOWUPS
   M5 is amended to record the C1 decision (floor raised; lock not downgraded).
10. **Scripts' error propagation [M3/M7]:** git grep status > 1 surfaces as a scan error
    (exit 2), never silently ignored or conflated with a finding; the diagnostics loop uses
    `-e "$tok" "$REV" --` (pattern before rev — KAT-G1's file:line assertion locks it).
11. **KAT isolation [M8]:** the harness runs COPIES of the scripts in temp workspaces; no KAT
    reads, writes, or deletes the operator's real `scripts/.pii-patterns`.

**Branch-protection ruleset step (operator-confirmed) [R0-I5]:**

After merge, create a ruleset on `main` requiring the five checks, e.g.:

```
gh api repos/{owner}/{repo}/rulesets -X POST --input - <<'JSON'
{ "name": "ci-required", "target": "branch", "enforcement": "active",
  "conditions": { "ref_name": { "include": ["refs/heads/main"], "exclude": [] } },
  "rules": [ { "type": "required_status_checks", "parameters": {
      "strict_required_status_checks_policy": false,
      "required_status_checks": [
        {"context": "test"}, {"context": "clippy"}, {"context": "fmt"},
        {"context": "msrv"}, {"context": "pii-scan"} ] } } ] }
JSON
```

(Exact JSON refined at implementation against the current API; check contexts must match the job
names.) The operator confirms the ruleset is active. **Fallback:** if the user declines
enforcement (single-user repo; direct pushes to `main` are the demonstrated workflow, and
required checks on `main` force a PR-based flow), record the detective-only posture explicitly in
FOLLOWUPS — the deferral must be a decision, not an omission.

**Post-merge green-run verification (GitHub-side ONLY — everything local already ran in Task 1):**

1. The first push after merge triggers `ci.yml`; all five jobs (`test`, `clippy`, `fmt`, `msrv`,
   `pii-scan`) show green in the Actions UI.
2. The ruleset (or the documented fallback decision) is in place.
3. Record the run URL in FOLLOWUPS ("first CI run: `<url>` — all 5 jobs green").

This is the spec's Definition of Done. Task 1's local runs are the primary verification; the
post-merge run confirms the GitHub-side wiring (triggers, runner env, action pins) only.

---

### FOLLOWUPS after ship (open)

- **Windows/macOS runners:** store FOLLOWUPS M-3 defers "verify under Windows CI that the written
  files are not world-readable". Add `windows-latest`/`macos-latest` matrix legs after the Linux
  baseline is stable. The store crate's `cfg(windows)` paths are compile-checked today, not
  CI-executed. (macOS leg: mind the bash-3.2 note in `README-pii-setup.md`.)
- **`cargo-audit` / `cargo-deny`:** supply-chain advisory scan; needs an advisory-DB update
  strategy and handling of expected findings (`allow-variable-time-crypto` in sequoia-openpgp).
- **Optional `PII_PATTERNS` GitHub Secret (D2 option (a)):** adds owner-specific coverage to CI
  (skip-with-warning when absent) without patterns in tracked content. The hook stays primary.
- **Commit-author email [R0-N5]:** commit metadata carries the owner's real email (author name is
  initials only). Outside the tracked-content invariant; mitigated by the private repo. Recorded
  as an accepted exposure — revisit only if the repo's visibility ever changes.

---

## Out of scope

- **Windows/macOS runners** — deferred per FOLLOWUPS (store M-3) and the TUI CI-deferral note.
- **`cargo-audit` / `cargo-deny`** — separate decision (advisory-DB cadence).
- **`rust-toolchain.toml`** — not added; the workflow installs toolchains explicitly; a repo-level
  toolchain pin affects local developer flow and must be scoped independently.
- **GitHub Secret PII approach (option (a))** — documented in D2 and FOLLOWUPS; not implemented.
- **Relicensing / removing the LICENSE copyright line** — explicitly NOT done; the carve-out is
  the user's standing rule (I4 resolution).
- **`Cargo.lock` changes / dependency downgrades** — explicitly NOT done (C1 Option B). The lock
  stays byte-identical; the MSRV declaration rises to meet it.
- **Crate source / test changes** — none. The `99·1234567` fixture stays in `tax_report.rs` and
  is handled by exclusion, not by editing the test (keeps this change infrastructure-only).
- **TUI MSRV check specifically** — the `msrv` job's `--workspace` already covers `btctax-tui`.
- **Retroactive history scanning** — the hook gates future pushes; auditing already-pushed
  history is a one-time operator action outside this spec (the repo is private; the standing
  session scan has kept the tree clean).
