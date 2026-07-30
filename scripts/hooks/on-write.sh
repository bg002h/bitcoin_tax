#!/usr/bin/env bash
# btctax-harness — PreToolUse(Write) — A3 (deny) + A4 (ask) of design/HARNESS.md.
#
# ★ SCOPE — btctax only. Wired from THIS repo's .claude/settings.json, never ~/.claude/settings.json.
#   Paths outside this repository are ignored outright.
#
#   A3  DENY  a primary-source-shaped file written outside every accounted-for tree.
#             Catches F1: `design/forms/` was built as an archive while `legal/primary-sources/`
#             already held the same material. The authoritative decision lives in Rust
#             (`xtask classify-path`) so the rule cannot drift from the test that enforces it.
#
#   A4  ASK   once, on the first write into a directory that does not exist yet.
#             Cannot verify a search happened; CAN force the question while it is still answerable.
#             ★ One-shot BY DESIGN: retrying the same write proceeds. A wall here would just be
#               routed around with `mkdir -p`, and teaching that gates are routable is worse than
#               the omission it would prevent.
#
# Exit 0 = allow. Exit 2 = block, stderr fed back into the model's loop at the decision point.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"
[ -z "$ROOT" ] && exit 0

INPUT="$(cat)"
FILE="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // empty')"
[ -z "$FILE" ] && exit 0

# Absolute-ise, then ignore anything outside this repo (scratchpad, /tmp, other projects).
case "$FILE" in /*) ABS="$FILE" ;; *) ABS="$ROOT/$FILE" ;; esac
case "$ABS" in "$ROOT"/*) ;; *) exit 0 ;; esac
REL="${ABS#"$ROOT"/}"
BASE="$(basename "$REL")"

# ── A3 ──────────────────────────────────────────────────────────────────────
# Cheap, deliberately OVER-BROAD prefilter: it only decides whether the authoritative check is worth
# running. Over-broad is the safe direction — a false positive costs 5ms, a false negative is a silent
# second archive. The real classification is `xtask classify-path`, the same code the test uses.
if printf '%s' "$BASE" | grep -qiE '\.(pdf|txt|html?|xml)$' \
   && printf '%s' "$BASE" | grep -qiE '^([fip][0-9]{3}|.*(usc|cfr).*[0-9]|form_|instructions_|schedule_|notice_|revrul_|revproc_|cca_|td_|pub)'
then
  XTASK="$ROOT/target/debug/xtask"
  [ -x "$XTASK" ] || XTASK="$ROOT/target/release/xtask"
  if [ -x "$XTASK" ]; then
    # `cargo run` is NOT used here: a cold build inside a Write hook would take a minute, and a hook
    # that hangs is a hook that gets disabled.
    if ! OUT="$("$XTASK" classify-path "$REL" 2>&1)"; then
      printf '%s\n' "$OUT" >&2
      cat >&2 <<'EOF'

  ★ This is A3 of design/HARNESS.md. On 2026-07-30 `design/forms/` was built from scratch as a
    primary-source archive while `legal/primary-sources/` already held the same material — hours
    after writing down "before deriving or building, grep for what already exists".

    A walk of the tree then found FOUR such trees, not two. Do not start a fifth.
EOF
      exit 2
    fi
  else
    cat >&2 <<EOF
BLOCKED: \`$REL\` looks like an IRS/USC/CFR primary source, but the authoritative check
(\`xtask classify-path\`) could not run — no built xtask binary at target/{debug,release}/xtask.

  Run \`make check\` (or \`cargo build -p xtask\`) and retry.

  ★ Blocking rather than waving it through is deliberate: A3 exists because a primary-source archive
    was once created without looking, and "the checker could not run" must never be the quiet path
    to the same outcome.
EOF
    exit 2
  fi
fi

# ── A4 ──────────────────────────────────────────────────────────────────────
DIR="$(dirname "$ABS")"
[ -d "$DIR" ] && exit 0                       # existing directory — nothing to ask

STATE="$ROOT/.git/btctax-harness"
mkdir -p "$STATE"
ACKED="$STATE/acked-dirs"
touch "$ACKED"
NEWREL="${DIR#"$ROOT"/}"

if grep -qxF "$NEWREL" "$ACKED" 2>/dev/null; then
  exit 0                                       # already asked once — proceed
fi
printf '%s\n' "$NEWREL" >> "$ACKED"

cat >&2 <<EOF
PAUSE — this Write creates a NEW DIRECTORY: $NEWREL/

  Before creating it, the question that F1 failed to ask:

    "before deriving or building, grep for what already exists —
     I conclude from not having looked"

  On 2026-07-30 that memory was written, and hours later \`design/forms/\` was built from scratch as
  a primary-source archive while \`legal/primary-sources/\` already held the same material. A walk of
  the tree later found FOUR overlapping archives. The rule was known. It was not applied.

  So: has a search for an existing home been run this session?
    - Yes, and nothing exists  → retry the write; it will proceed.
    - No                       → search first. rg/grep/ls cost seconds; a duplicate tree costs a
                                 reconciliation project.

  ★ Asked ONCE per directory. This is not a wall — retrying proceeds.
EOF
exit 2
