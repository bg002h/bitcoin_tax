# Worktree / branch / scratch-dir housekeeping audit — 2026-09-06

Read-only audit. No deletions performed. Timestamps below are `-07:00`.

## Timeline note (a live race, not an error)

At the FIRST measurement (`git worktree list --porcelain`, run first), exactly one non-main
worktree existed:

```
worktree /scratch/code/bitcoin_tax/.claude/worktrees/agent-a4d62830162e96c69
HEAD 961c265300835f74f43fc92f2f4a04ec2e29f5c2
detached
```

with a companion branch `worktree-agent-a4d62830162e96c69` (tip `2bd04d45`, an ancestor of
`961c2653`, i.e. same lineage, older commit). This matches the brief's description of the
in-flight reviewer exactly (HEAD `961c2653`, directory `agent-a4d62830162e96c69`, dated
`2026-09-06 14:03:02`, main's tip one minute ahead at `b43a2b53` 14:04:01) — this was the LIVE
worktree at audit start.

By the SECOND round of commands (~10 minutes later in wall-clock terms), that worktree directory
was gone (`No such file or directory`), the branch `worktree-agent-a4d62830162e96c69` no longer
appeared in `git branch --list`, `git worktree prune --dry-run` reported nothing (no stale
admin state), and `.git/worktrees/` had no subdirectories at all. Main had advanced to
`5f03b965` — commit message `review(spec 1099da R6 r3): persist the Opus re-review VERBATIM —
7/11 resolved, 4 partial; NEW 1C/4I/3M/1N, nothing folded yet`. This is exactly the
persist-the-report step the controller runs right after a review agent finishes, immediately
followed by `git worktree remove --force` + `git branch -D`.

**Conclusion: the one live worktree completed and was correctly cleaned up by its own
controller during this audit.** Its report was persisted to main (commit `5f03b965`) before
removal — no uncopied-report risk. There is nothing left to clean up for it, and nothing here
should be treated as abandoned garbage from it.

## Worktrees

| Path | Branch | HEAD | Dirty? | Newest mtime (excl. `target*`) | Uncopied `design/agent-reports/`? | Verdict |
|---|---|---|---|---|---|---|
| `/scratch/code/bitcoin_tax` | `main` | `5f03b965` (end of audit) | clean (`git status --porcelain` empty) | `2026-09-06 14:16` (`design/agent-reports/2026-09-06-spec-1099da-R6-review-r3.md`) | n/a — this *is* the main tree | **LIVE** — the repository itself, never touch |
| `.claude/worktrees/agent-a4d62830162e96c69` | (detached, was paired with branch `worktree-agent-a4d62830162e96c69`) | `961c2653` | unknown — directory vanished before status could be captured | last observed `2026-09-06 14:04` | No — content landed in main as `5f03b965` before removal | **Was LIVE at audit start; already self/controller-cleaned mid-audit. Confirmed gone. Nothing to do.** |

No other worktree directories or `git worktree list` entries exist.

## Orphan `worktree-agent-*` branches

| Branch | Tip | On `main`? | Verdict |
|---|---|---|---|
| *(none present at time of writing)* | — | — | — |

Historical: `worktree-agent-a4d62830162e96c69` (tip `2bd04d45`, an ancestor of the review
worktree's own `961c2653` but **not** an ancestor of `main`) existed at audit start and was
deleted by the controller's normal cleanup before this report was written. `git for-each-ref
refs/heads/worktree-agent-*` now returns nothing.

## Dangling directories under `.claude/worktrees/`

| Directory | Size | Contents | Registered as a git worktree? | mtime | Verdict |
|---|---|---|---|---|---|
| `.claude/worktrees/ordchk-scratch` | 0 bytes | two empty dirs: `ordchk-scratch/`, `ordchk-scratch/src/` | **No** — absent from `git worktree list`, no `.git` file/dir inside it | `2026-09-06 14:07` (today) | **ABANDONED** — plain (non-git) orphan scratch directory, unrelated to any tracked worktree or branch; safe to remove |

`git worktree prune --dry-run -v` produced no output (nothing stale to prune) and
`.git/worktrees/` has zero admin subdirectories — no interrupted-removal residue anywhere.
`git count-objects -vH`: `count: 87, size: 2.21 MiB, in-pack: 20880, size-pack: 54.95 MiB,
prune-packable: 0, garbage: 0` — clean object store, nothing loose to gc.

## Build / cache directories (report only — not proposed for deletion)

| Directory | Size | Note |
|---|---|---|
| `target/` | **521G** | main build dir |
| `target-clippy/` | 22G | clippy's own target dir |
| `target-review/` | 22G | deliberate, gitignored, shared reviewer-agent cargo cache — **not garbage** |

`target/` at 521G is unusually large and may be worth a separate look (stale incremental
artifacts across many toolchains/features), but that is outside this audit's scope per the
brief — reported, not touched.

## `/tmp/claude-1000/` — report only (size/age; contents not read)

| Item | Size | mtime | Age |
|---|---|---|---|
| `-scratch-code-bitcoin-tax` | 2.0G | 2026-09-06 10:10 | same day (this repo's active session scratch) |
| `-scratch-code-shibboleth-mnemonic-engrave` | 9.9M | 2026-09-06 03:52 | same day |
| `-scratch-code-bitcoinprojections` | 2.1M | 2026-09-06 13:18 | same day |
| `-home-bcg-Work` | 104K | 2026-09-02 19:29 | ~4 days |
| `btctax-table-claims` | 4.0K | 2026-09-05 20:15 | ~18 hours |
| `-scratch-code-DistroHopping` | 0 (empty) | 2026-09-02 17:05 | ~4 days |
| `-scratch-code-bitcoin-tax--claude-worktrees-agent-a1d841b37fe601cd2` | 0 (empty) | 2026-09-05 06:27 | ~32 hours; named after a *different*, already-gone agent worktree id (`a1d841b37fe601cd2`, not today's `a4d62830162e96c69`) |

Nothing here is proposed for cleanup — the brief scopes `/tmp` to report-only.

## Proposed cleanup commands

```
# The one worktree observed at audit start (.claude/worktrees/agent-a4d62830162e96c69, HEAD
# 961c2653, branch worktree-agent-a4d62830162e96c69) was LIVE and has ALREADY been removed by
# its own controller mid-audit (worktree + branch both gone; report persisted to main as
# commit 5f03b965). No worktree/branch removal command is needed — excluded from this list.

rm -rf /scratch/code/bitcoin_tax/.claude/worktrees/ordchk-scratch   # 0 bytes, non-git plain directory (no .git), absent from `git worktree list`, unrelated "ordchk" scratch leftover dated 2026-09-06 14:07 with no referencing branch or process
```
