# PRE-PUBLISH BRIEF — btctax v0.15.0

**Tree under review: `main` at `b27e6b9`.** This is the exact tree that would be published.

## Why you, and why now

crates.io versions are **immutable**. A published crate cannot be unpublished, replaced, or
corrected — only yanked, which leaves the source tarball permanently downloadable. This is the first
irreversible action in this release, and the house rule reserves a single escalated review for
exactly this moment. It has not been spent elsewhere on this tree.

**"Do not publish" is a successful outcome.** So is "publish". Say which, and why, in one line.

## The ONE question

**Would publishing this tree, as-is, to a permanent public registry be a mistake?**

Three ways it could be, in descending order of how much they should worry you:

1. **A broken or wrong artifact ships and cannot be withdrawn.** The tarball does not build, is
   missing a file it `include_str!`s, or resolves against the wrong version of a sibling crate.
2. **Something private becomes permanently public** — in the tarball, which is the part that cannot
   be deleted.
3. **The release misrepresents itself** — the notes claim a fix that isn't there, or fail to disclose
   one that filers who already used v0.14.0 need to know about.

## ★ THE TWO TRAPS THIS PROJECT HAS ALREADY BEEN BITTEN BY

Both are invisible to `make check` **and** to CI. Both were found only after a publish. Check them
first and explicitly:

- **An escaping `include_str!` ships a broken tarball with EXIT 0.** If a crate `include_str!`s (or
  `include_bytes!`s) a path outside its own directory, `cargo package` happily builds it in the
  workspace — where the path resolves — and the published tarball is broken for everyone else.
  Enumerate every `include_str!`/`include_bytes!` in the ten published crates and confirm each target
  is inside that crate's package.
- **A missed inter-crate version pin does NOT fail the publish.** It silently ships a crate that
  resolves against the PREVIOUS release. All ten crates were bumped `0.14.0 → 0.15.0` in `75f27f9`,
  along with every `version =` pin; verify independently rather than trusting that commit.

## What ships, and what does not

Ten published crates under `crates/`. `btctax-oracle-harness` and `xtask` are `publish = false`.
No crate declares `include`/`exclude`, so each tarball is its git-tracked directory contents.
`design/`, `legal/`, `reviews/`, `docs/` and `scripts/` are **outside every crate directory** and do
not ship — this was verified with `cargo package --list` across all ten, but re-check it if any part
of your reasoning depends on it.

Publish order matters (dependency-first, not `--workspace`): core and store first, then adapters,
forms, input-form, then cli, then tui, tui-edit, update-prices, then the `btctax` binary crate.

## Settled — do NOT re-derive or re-litigate

1. **The tax logic has converged.** Six independent review rounds ran on this branch, the last three
   scoped to what the earlier ones could not see; they found 9 blocking defects, all fixed and
   mutation-verified. The final round asked "what did the fixes leak?" and answered *no leak*.
   Reviewer outputs are persisted verbatim in `reviews/`. **Do not re-audit tax correctness.** If you
   find yourself reading `return_1040.rs` for a wrong figure, you are in the wrong document.
2. Open and deliberately unfixed, each filed in `FOLLOWUPS.md`: **§G-19a** (§1411 display — owner
   decision), **§G-12** (no Form 8275-R — blocked on an unobtainable asset), **§G-22** (other
   carryforward families still import-only), **§G-20b**, **§G-23** (`CarryProvenance` cannot express
   "the filer stated zero"). Not findings.
3. Neither tax oracle validates a value it is HANDED (Form 8995 line 3). Known standing condition.
4. `scripts/.pii-patterns` does not exist and gates **push**, not publish. Out of scope.
5. A PII sweep ran at `b27e6b9`: no real SSNs, amounts, account numbers, wallet data or keys
   anywhere; the author's home paths and a private sibling project's identity were redacted from
   `design/` in that commit. None of it was ever in a tarball. Re-check the **tarball** surface if you
   wish; do not re-run the `design/` sweep.
6. All five gates pass at HEAD: `make check` (2542 tests), `cargo fmt --all --check`,
   `cargo +1.88 check --workspace --locked`, `xtask check-isolation`, `scripts/pii-scan-generic.sh`.
   TY2024 golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47`.

## Specifically worth your judgment

- **`design/release-0.15.0-notes.md`** — read it against `git diff v0.14.0..HEAD`. Does it claim
  anything that isn't there? Does it omit anything a filer needs? It leads with Form 8995 line 3
  because that defect was **live in v0.14.0 and understated tax**. Is that disclosure adequate and
  correctly scoped — i.e. would a filer who used v0.14.0 know from these notes whether they are
  affected and what to do?
- **Licensing / NOTICE posture.** `MIT OR Unlicense`, and the NOTICE disclaims authorisation and
  warranty for filing. It must stay UNRESTRICTED — a NOTICE that becomes a use restriction is a
  defect. Confirm nothing in this release turns it into one.
- **README / LIMITATIONS accuracy at the version being published.** A limitation that was fixed but
  is still documented as present, or vice versa, is a real problem for a tax tool.
- **Anything a first-time installer would hit.** `cargo install btctax` on a clean machine: does the
  binary crate's dependency set, MSRV (1.88) and feature wiring actually work from the registry
  rather than from the workspace?

## Output — follow exactly

First line: `VERDICT: publish` or `VERDICT: do-not-publish: <the one thing>`

Then findings, most severe first, each fenced:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: what a user or the registry actually experiences.
EVIDENCE: quote the code, manifest, or file listing.
```

**Critical** = a broken or wrong artifact ships irreversibly, or something private becomes permanent.
**Important** = the release misrepresents itself, or a first-run path is broken.
Do not inflate; do not pad. A short clean report is the expected result.

End with:

`WHAT I VERIFIED BY EXECUTION:` — say what you actually ran, versus what you reasoned about.
Prefer running things: `cargo package --list`, `cargo package`, grep, `git diff v0.14.0..HEAD`.

`WHAT WOULD MAKE THIS REVIEW WRONG:` — the assumption you did not verify.

**Constraints:** READ-ONLY on tracked files. Do not commit, tag, push, or publish. Do not run
`cargo publish` even with `--dry-run` against the network. `cargo package --list --allow-dirty
--offline` is fine and encouraged. Do not spawn subagents.
