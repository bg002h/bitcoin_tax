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

[ -z "$VERDICT" ] && exit 0

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
