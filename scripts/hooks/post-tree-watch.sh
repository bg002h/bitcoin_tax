#!/usr/bin/env bash
# btctax-harness — PostToolUse(Bash): A3's coverage, made MECHANISM-INDEPENDENT.
#
# ★★★ WHY THIS EXISTS, AND WHY IT IS BETTER THAN WHAT IT SUPPLEMENTS.
#
#   A3's PreToolUse half watches the `Write` tool. A4's Bash half watches `mkdir`. Both enumerate
#   *ways a file can appear* — and that is the excuse-list mistake this whole harness warns against:
#   `cp`, `mv`, `tar -x`, `curl -o`, `>` redirection, `unzip`, `rsync`, `install`, a script three
#   levels down… the list is unbounded and every entry is one someone happened to think of.
#
#   This hook watches the TREE instead of the COMMAND. It cannot be routed around by choosing a
#   different tool, because it does not care which tool ran — only what is now on disk. Same rule the
#   oracle excuse lists follow: state the mechanism, let it decide, never enumerate the outcomes you
#   happened to see.
#
# ★ COST — measured, because a slow PostToolUse hook is one that gets disabled: a full walk of this
#   repo (1833 files, excluding .git/ and build dirs) plus sha256 is ~21ms. `git status` would be
#   3ms but is BLIND to a new file that matches a gitignore rule, and a detector whose correctness
#   depends on .gitignore is not a detector.
#
# ★ SCOPE — btctax only, wired from THIS repo's .claude/settings.json. It reports; it cannot undo,
#   since the tool has already run. That still converts "caught at the next `make check`" into
#   "caught within one tool call", which is the difference between a fact and an archaeology project.
#
# ★ It deliberately does NOT report new directories — only primary sources in the wrong place. A
#   new-directory report after the fact is noise, and noise gets the hook muted.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"
[ -z "$ROOT" ] && exit 0
cd "$ROOT"

STATE="${BTCTAX_HARNESS_STATE:-$ROOT/.git/btctax-harness}"
mkdir -p "$STATE"
CACHE="$STATE/tree-listing"
NEW="$STATE/tree-listing.new"

# ★★★ `.claude` is pruned because it holds git WORKTREES — second checkouts of the same commit, where
# isolated reviewers run. Every file in one is already accounted for at its canonical path, so walking
# it reports the entire archive as new (130+ paths on the first isolated review). An alarm that fires
# on nothing is worse than no alarm: it teaches the reader to skip it, and this one exists precisely
# because a real overlap went unnoticed.
find . -path ./.git -prune -o -path ./.claude -prune -o -path ./target -prune \
       -o -path ./target-clippy -prune \
       -o -path ./.venv -prune -o -path ./node_modules -prune -o -name __pycache__ -prune \
       -o -type f -print 2>/dev/null | sed 's|^\./||' | sort > "$NEW"

# Cold start: nothing to diff against. Seed the cache and stay silent — `make check`'s test half is
# the backstop for anything that landed before the watch began.
if [ ! -f "$CACHE" ]; then
  mv -f "$NEW" "$CACHE"
  exit 0
fi

ADDED="$(comm -13 "$CACHE" "$NEW" || true)"
mv -f "$NEW" "$CACHE"           # always advance, so the same file is reported once, not every call
[ -z "$ADDED" ] && exit 0

# ★ BTCTAX_XTASK lets the kill test run this against a THROWAWAY repo. Testing it against the real
#   tree would mean creating a stray primary source there, which races archive_check's live gate —
#   a test that can red an unrelated test is not a test.
XTASK="${BTCTAX_XTASK:-$ROOT/target/debug/xtask}"
[ -x "$XTASK" ] || XTASK="$ROOT/target/release/xtask"
[ -x "$XTASK" ] || exit 0        # no built binary — the test half still gates at `make check`

STRAYS=""
while IFS= read -r p; do
  [ -z "$p" ] && continue
  b="$(basename "$p")"
  # Same deliberately over-broad prefilter as on-write.sh: it only decides whether the authoritative
  # check is worth a 5ms call. Over-broad is the safe direction.
  printf '%s' "$b" | grep -qiE '\.(pdf|txt|html?|xml)$' || continue
  printf '%s' "$b" | grep -qiE '^([fip][0-9]{3}|.*(usc|cfr).*[0-9]|form_|instructions_|schedule_|notice_|revrul_|revproc_|cca_|td_|pub)' || continue
  if ! OUT="$("$XTASK" classify-path "$p" 2>&1)"; then
    STRAYS="${STRAYS}
  ${p}"
  fi
done <<< "$ADDED"

[ -z "$STRAYS" ] && exit 0

cat >&2 <<EOF
A PRIMARY SOURCE JUST APPEARED OUTSIDE EVERY ACCOUNTED-FOR TREE:
${STRAYS}

  This was detected by watching the TREE, not the command — so it fires for cp, mv, tar, curl,
  redirection, or anything else, not just the handful of commands someone thought to enumerate.

  The file already exists (this runs AFTER the tool). Move it into an accounted-for tree, or extend
  KNOWN_ARCHIVES in crates/xtask/src/archive_check.rs deliberately — \`make check\` will red until
  one of those is true.

  ★ A3 exists because on 2026-07-30 \`design/forms/\` was built from scratch while
    \`legal/primary-sources/\` already held the same material, and a walk then found FOUR overlapping
    archives where the record said two. Do not start another one.
EOF
exit 2
