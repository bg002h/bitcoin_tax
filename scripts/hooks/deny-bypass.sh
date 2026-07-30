#!/usr/bin/env bash
# btctax-harness — PreToolUse(Bash) deny: close the route-around on the commit gate.
#
# A2 of design/HARNESS.md. The pre-commit hook makes "commit with the gate red" inexpressible; this
# makes UNDOING that a deliberate act rather than a reflex. Doctrine (design/HARNESS.md, "What this
# deliberately does NOT attempt"): gate FACTS, never judgement — "did this command ask git to skip its
# hooks?" is a fact with a yes/no answer.
#
# ★ SCOPE — btctax only. Wired from THIS repo's .claude/settings.json, never ~/.claude/settings.json,
#   so it cannot fire in another project.
#
# ★★ AND IT ONLY BINDS THE ASSISTANT. This governs Claude Code's Bash tool. The human owner running
#   `git commit --no-verify` in their own terminal is untouched, which is the correct division: the
#   observed failure (F3) was the assistant's reflex, and the owner does not need a machine's
#   permission to override their own repo.
#
# Exit 0 = allow. Exit 2 = block, with stderr fed back into the model's loop at the decision point.
set -euo pipefail

INPUT="$(cat)"
COMMAND="$(printf '%s' "$INPUT" | jq -r '.tool_input.command // empty')"
[ -z "$COMMAND" ] && exit 0

# Tokenise with shell semantics so a quoted MESSAGE containing "--no-verify" is not a false positive.
# A hook that cries wolf gets muted, and a muted hook is worse than no hook.
VERDICT="$(printf '%s' "$COMMAND" | python3 -c '
import shlex, sys

cmd = sys.stdin.read()
try:
    toks = shlex.split(cmd)
except ValueError:              # unbalanced quotes — not our business, let it through
    sys.exit(0)

SEPS = {"&&", "||", ";", "|", "&"}
GATED = {"commit", "push", "merge", "rebase"}

def bypasses(flag: str) -> bool:
    if flag == "--no-verify":
        return True
    # short-flag cluster containing n, e.g. -n, -nm  (but NOT -m, --amend, or a value like -n5)
    return (
        len(flag) > 1
        and flag[0] == "-"
        and flag[1] != "-"
        and "n" in flag[1:]
        and flag[1:].isalpha()
    )

i = 0
while i < len(toks):
    if toks[i] != "git":
        i += 1
        continue
    j = i + 1
    verb = None
    while j < len(toks) and toks[j] not in SEPS:
        t = toks[j]
        if verb is None and not t.startswith("-"):
            verb = t
        elif verb in GATED and bypasses(t):
            print(f"{verb}:{t}")
            sys.exit(0)
        j += 1
    i = j

# Disabling the hooks path is the other way to make the gate stop existing.
for k, t in enumerate(toks):
    if t == "core.hooksPath" and k > 0 and "config" in toks[max(0, k - 3):k]:
        if "--unset" in toks[max(0, k - 3):k + 1]:
            print("config:--unset core.hooksPath")
            sys.exit(0)
' || true)"

# ── A4 (Bash half) — the new-directory ask, for directories made OUTSIDE the Write tool ────────
#
# ★★ ADDED 2026-07-30 FROM AN OBSERVED FAILURE, not an anticipated one. A4 was built watching only
#    Write/Edit. Within the hour its own author created `design/forms/2026/` with `mkdir -p` in Bash
#    and the ask never fired — `.git/btctax-harness/acked-dirs` was still absent, which is how the
#    hole was found. design/HARNESS.md had predicted this exact route-around in writing and the
#    prediction changed nothing, because a documented hole is still a hole.
#
# ★ Deliberately narrow, since a noisy hook gets muted: only `mkdir`, only paths INSIDE the repo,
#   only directories that do not already exist, and never anything git ignores (target/, .venv/, …).
#   It shares on-write.sh's ack file, so a directory is asked about ONCE across both routes.
if [ -z "$VERDICT" ]; then
  ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"
  if [ -n "$ROOT" ]; then
    NEWDIRS="$(printf '%s' "$COMMAND" | python3 -c '
import shlex, sys
cmd = sys.stdin.read()

# ★★★ TWO FALSE POSITIVES IN TWO REAL USES taught this parser its limits, and both were the same
#     lesson: statically parsing arbitrary shell to predict filesystem effects is the wrong
#     instrument. It is kept only for the PRE-hoc ask (the decision point, which no after-the-fact
#     hook can provide) and is now deliberately timid. post-tree-watch.sh is the robust half: it
#     observes the resulting TREE and needs no parsing at all.
#
#       1. `mkdir -p $S`   -> shlex does not expand variables, so the literal "$S" was read as a
#          repo-relative path and asked about a directory actually in /tmp.
#       2. `... "$S"; run() { ... }` -> shlex.split leaves ";" ATTACHED to the preceding word, so
#          the separator was never seen, the scan ran on, and `run()` from a shell function
#          definition was read as a directory name.
#
#     Hence: punctuation_chars so separators really are separators, and a strict path filter. An
#     unresolvable or implausible target fails OPEN. A hook that cries wolf gets muted, and a muted
#     hook protects nothing.
lex = shlex.shlex(cmd, posix=True, punctuation_chars=True)
lex.whitespace_split = True
try:
    toks = list(lex)
except ValueError:
    sys.exit(0)

SEPS = {"&&", "||", ";", "|", "&", "(", ")", "{", "}", ";;", "|&"}

def plausible_path(t: str) -> bool:
    if not t or t.startswith("-") or t.startswith("~"):
        return False
    # Unexpanded variables/substitutions: we cannot know where they point.
    if any(c in t for c in "$`*?[]<>=!"):
        return False
    # Shell syntax that is not a path (function definitions, groupings, redirections).
    if any(c in t for c in "(){}\";|&"):
        return False
    return True

out, i = [], 0
while i < len(toks):
    if toks[i] != "mkdir":
        i += 1
        continue
    j = i + 1
    while j < len(toks) and toks[j] not in SEPS:
        t = toks[j]
        if t.startswith("-"):
            j += 1
            continue
        if not plausible_path(t):
            break          # ★ stop at the first thing that is not clearly a path -- do not guess
        out.append(t)
        j += 1
    i = j if j > i else i + 1
print("\n".join(out))
' 2>/dev/null || true)"
    ASK=""
    while IFS= read -r d; do
      [ -z "$d" ] && continue
      case "$d" in /*) ABS="$d" ;; *) ABS="$ROOT/$d" ;; esac
      case "$ABS" in "$ROOT"/*) ;; *) continue ;; esac      # outside the repo — not our business
      [ -d "$ABS" ] && continue                              # already exists — nothing to ask
      REL="${ABS#"$ROOT"/}"
      git -C "$ROOT" check-ignore -q "$REL" 2>/dev/null && continue   # build dirs, .venv, etc.
      ASK="$REL"
      break
    done <<< "$NEWDIRS"

    if [ -n "$ASK" ]; then
      # ★ Overridable so tests get an isolated, empty ack dir: the ask is ONE-SHOT, so a
      #   persistent file would make "does it ask?" pass once and fail forever after, and two
      #   tests sharing it would race.
      STATE="${BTCTAX_HARNESS_STATE:-$ROOT/.git/btctax-harness}"; mkdir -p "$STATE"
      ACKED="$STATE/acked-dirs"; touch "$ACKED"
      if ! grep -qxF "$ASK" "$ACKED" 2>/dev/null; then
        printf '%s\n' "$ASK" >> "$ACKED"
        cat >&2 <<EOF
PAUSE — this command creates a NEW DIRECTORY in the repo: $ASK/

  Before creating it, the question that F1 failed to ask:

    "before deriving or building, grep for what already exists —
     I conclude from not having looked"

  On 2026-07-30 that memory was written, and hours later \`design/forms/\` was built from scratch as
  a primary-source archive while \`legal/primary-sources/\` already held the same material. A walk of
  the tree later found FOUR overlapping archives. The rule was known. It was not applied.

  So: has a search for an existing home been run this session?
    - Yes, and nothing exists  → retry the command; it will proceed.
    - No                       → search first.

  ★ Asked ONCE per directory, shared with the Write hook. This is not a wall — retrying proceeds.
EOF
        exit 2
      fi
    fi
  fi
  exit 0
fi

VERB="${VERDICT%%:*}"
FLAG="${VERDICT#*:}"

cat >&2 <<EOF
BLOCKED by the btctax harness: \`git $VERB\` with \`$FLAG\` skips this repo's gates.

  What you are about to skip:
    pre-commit — make check (nextest + clippy) + cargo fmt --all --check
    pre-push   — the PII scan over the pushed range

  Why this is denied rather than discouraged: on 2026-07-30 a commit landed in this
  repo with \`make check\` RED — the gate had been run and its output never read.
  An instruction not to do that is exactly what had already failed; this is a fact-gate.

  btctax emits a US federal 1040 signed under 26 USC §6065 penalties of perjury.

  If the gate is red: FIX IT, or say plainly that it is red and let the owner decide.
  Do not re-run this command with the flag removed if the gate is genuinely failing —
  that is the same bypass wearing a different costume.

  ★ This binds the assistant's Bash tool only. The repo owner can run it directly.
EOF
exit 2
