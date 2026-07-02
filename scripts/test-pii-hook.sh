#!/usr/bin/env bash
# test-pii-hook.sh — Shell KATs for pii-scan-generic.sh and pre-push hook.
# Isolation: COPIES of scripts run in temp workspaces (never in-place) [R0-M8].
# Fixtures:  ALL shape-matching strings assembled at runtime; NEVER a literal in
#            this file or any tracked file.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GENERIC="$SCRIPT_DIR/pii-scan-generic.sh"
HOOK="$SCRIPT_DIR/pre-push"

PASS=0
FAIL=0
TMPWS=""
TIP=""
BASE=""

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# ── Temp workspace helper [R0-M8] ────────────────────────────────────────────
# Creates a fresh temp git repo and copies the scripts into it.
# The hook resolves PATTERNS_FILE and calls pii-scan-generic.sh relative to its
# own location ($SCRIPT_DIR). git grep uses the process CWD for repo detection,
# so ALL script invocations must run from inside $TMPWS (via cd subshells).
# This also means the operator's real scripts/.pii-patterns is never touched.
new_ws() {
  TMPWS=$(mktemp -d)
  git -C "$TMPWS" init -q
  git -C "$TMPWS" config user.email "test@example.com"
  git -C "$TMPWS" config user.name "Test"
  cp "$GENERIC" "$TMPWS/pii-scan-generic.sh"
  cp "$HOOK"    "$TMPWS/pre-push"
  chmod +x "$TMPWS/pii-scan-generic.sh" "$TMPWS/pre-push"
}

cleanup_ws() {
  [ -n "${TMPWS:-}" ] && rm -rf "$TMPWS"
  TMPWS=""
}

# Commit a file in $TMPWS; sets TIP to the new HEAD.
commit_file() {
  local fname="$1" content="$2" msg="${3:-test}"
  printf '%s' "$content" > "$TMPWS/$fname"
  git -C "$TMPWS" add "$fname"
  git -C "$TMPWS" commit -q -m "$msg"
  TIP=$(git -C "$TMPWS" rev-parse HEAD)
}

# Run the generic scan from inside TMPWS (so git grep finds the right repo).
run_scan() {
  (cd "$TMPWS" && bash ./pii-scan-generic.sh "$@")
}

# Run the hook from inside TMPWS via stdin ref protocol.
# Usage: run_hook [env prefix] <<< "local_ref local_sha remote_ref remote_sha"
run_hook() {
  (cd "$TMPWS" && bash ./pre-push "$@")
}

ZERO=0000000000000000000000000000000000000000

# ── Runtime fixture assembly ─────────────────────────────────────────────────
# ALL shape-matching literals assembled here at runtime via printf segments.
# No segment individually matches the scanner shapes; the composed value does.
SSN_HIT=$(printf '%s-%s-%s' 999 00 1234)   # SSN-like, non-excluded
EIN_HIT=$(printf '%s-%s' 55 5678901)        # EIN-like, non-excluded
EXC_SSN=$(printf '%s-%s-%s' 987 65 4321)   # excluded synthetic SSN
EXC_EIN1=$(printf '%s-%s' 12 3456789)      # excluded synthetic EIN 1
EXC_EIN2=$(printf '%s-%s' 99 1234567)      # excluded synthetic EIN 2

# ═══════════════════════════════════════════════════════════════════════════════
# G-series: pii-scan-generic.sh unit KATs
# ═══════════════════════════════════════════════════════════════════════════════

echo "=== G-series: pii-scan-generic.sh ==="

# KAT-G1: SSN-shaped non-excluded token → exits 1; output has file:line [R0-M7]
{
  new_ws
  commit_file "secret.txt" "value: $SSN_HIT" "add SSN hit"
  out=$(run_scan "$TIP" 2>&1) && rc=0 || rc=$?
  # git grep -InF -e "$tok" "$REV" -- prefixes output as "<rev>:<file>:<line>:<content>"
  # The spec requires a file:line location in the output [R0-M7]; match that substring.
  if [ "$rc" -eq 1 ] && echo "$out" | grep -qE 'secret\.txt:[0-9]+:'; then
    pass "G1 (SSN-shaped hit + file:line diagnostics)"
  else
    fail "G1: expected rc=1 + file:line; got rc=$rc, output: $out"
  fi
  cleanup_ws
}

# KAT-G2: EIN-shaped non-excluded token → exits 1
{
  new_ws
  commit_file "data.txt" "ein: $EIN_HIT" "add EIN hit"
  run_scan "$TIP" >/dev/null 2>&1 && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "G2 (EIN-shaped hit → exit 1)" || fail "G2: expected rc=1; got $rc"
  cleanup_ws
}

# KAT-G3: Only the three excluded synthetics → exits 0
{
  new_ws
  commit_file "synth.txt" "$EXC_SSN $EXC_EIN1 $EXC_EIN2" "excluded synthetics only"
  run_scan "$TIP" >/dev/null 2>&1 && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "G3 (excluded tokens only → clean exit 0)" || fail "G3: expected rc=0; got $rc"
  cleanup_ws
}

# KAT-G4: Mixed line — excluded synthetic + non-excluded shaped token [R0-M1]
# Token-level filtering: a line containing both must still be flagged.
{
  new_ws
  commit_file "mixed.txt" "$EXC_SSN and $SSN_HIT on the same line" "mixed line"
  run_scan "$TIP" >/dev/null 2>&1 && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "G4 (mixed line, token-level filter → exit 1)" || fail "G4: expected rc=1; got $rc"
  cleanup_ws
}

# ═══════════════════════════════════════════════════════════════════════════════
# H-series: pre-push hook KATs
# Hook is driven via its stdin ref protocol; run_hook cd's into TMPWS so that
# git commands inside the hook and pii-scan-generic.sh resolve the right repo.
# ═══════════════════════════════════════════════════════════════════════════════

echo "=== H-series: pre-push hook ==="

# KAT-H1: hook — hit at tip; range base..tip → exits 1
{
  new_ws
  commit_file "readme.txt" "clean content" "base"
  BASE="$TIP"
  commit_file "secret.txt" "value: $SSN_HIT" "add PII at tip"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "H1 (hook — hit at tip → exit 1)" || fail "H1: expected rc=1; got $rc"
  cleanup_ws
}

# KAT-H2: hook — clean range → exits 0
{
  new_ws
  commit_file "readme.txt" "clean content" "base"
  BASE="$TIP"
  commit_file "notes.txt" "no PII here at all" "clean commit"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "H2 (hook — clean range → exit 0)" || fail "H2: expected rc=0; got $rc"
  cleanup_ws
}

# KAT-H3: hook — INTERMEDIATE commit [R0-I3]
# Commit A adds PII; commit B removes it. Working tree is clean.
# The hook must still detect PII because history cannot be unpushed.
{
  new_ws
  commit_file "readme.txt" "clean content" "base"
  BASE="$TIP"
  commit_file "secret.txt" "value: $SSN_HIT" "commit A: add PII"
  commit_file "secret.txt" "value: REDACTED" "commit B: remove PII"
  TIP_B="$TIP"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP_B" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "H3 (hook — intermediate commit PII detected → exit 1)" || fail "H3: expected rc=1; got $rc"
  cleanup_ws
}

# KAT-H4: hook — new ref, all-zeros remote SHA; --not --remotes path [R0-I7]
# Temp repos have no remote-tracking refs so --not --remotes scans full history.
{
  new_ws
  commit_file "readme.txt" "clean content" "init"
  commit_file "secret.txt" "value: $SSN_HIT" "add PII on new branch"
  TIP_NEW="$TIP"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  # Remote SHA = all zeros (new ref never pushed before)
  printf 'refs/heads/feat %s refs/heads/feat %s\n' "$TIP_NEW" "$ZERO" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "H4 (hook — new ref --not --remotes → exit 1)" || fail "H4: expected rc=1; got $rc"
  cleanup_ws
}

# KAT-H5: hook — missing patterns file → fail-closed [R0-I2]
{
  new_ws
  commit_file "readme.txt" "clean" "init"
  BASE="$TIP"
  commit_file "notes.txt" "more clean" "add"
  # No .pii-patterns created
  out=$(printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>&1) && rc=0 || rc=$?
  if [ "$rc" -eq 1 ] && echo "$out" | grep -qi 'missing\|no patterns\|README'; then
    pass "H5 (hook — missing patterns → fail-closed with remediation text)"
  else
    fail "H5: expected rc=1 + remediation; got rc=$rc, output: $out"
  fi
  cleanup_ws
}

# KAT-H5b: hook — bypass with BTCTAX_PII_BYPASS=1 [R0-I2]
#   sub-case 1: bypass + generic-shaped fixture in range → still exits 1 (generic scan runs)
#   sub-case 2: bypass + clean range → exits 0
{
  new_ws
  commit_file "readme.txt" "clean" "base"
  BASE="$TIP"
  commit_file "secret.txt" "value: $SSN_HIT" "add PII"
  # No .pii-patterns
  BTCTAX_PII_BYPASS=1 \
    bash -c "cd '$TMPWS' && printf 'refs/heads/main %s refs/heads/main %s\n' '$TIP' '$BASE' \
      | bash ./pre-push origin" 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "H5b-1 (bypass + generic hit still fails → exit 1)" || fail "H5b-1: expected rc=1; got $rc"
  cleanup_ws

  new_ws
  commit_file "readme.txt" "clean" "base"
  BASE="$TIP"
  commit_file "notes.txt" "no pii" "clean"
  # No .pii-patterns
  BTCTAX_PII_BYPASS=1 \
    bash -c "cd '$TMPWS' && printf 'refs/heads/main %s refs/heads/main %s\n' '$TIP' '$BASE' \
      | bash ./pre-push origin" 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "H5b-2 (bypass + clean range → exit 0)" || fail "H5b-2: expected rc=0; got $rc"
  cleanup_ws
}

# KAT-H5c: hook — present-but-empty patterns file → fail-closed [R0-M9]
{
  new_ws
  commit_file "readme.txt" "clean" "init"
  BASE="$TIP"
  commit_file "notes.txt" "clean" "add"
  printf '# comment only\n\n# another comment\n' > "$TMPWS/.pii-patterns"
  out=$(printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>&1) && rc=0 || rc=$?
  if [ "$rc" -eq 1 ] && echo "$out" | grep -qi 'missing\|no patterns\|README'; then
    pass "H5c (hook — empty patterns file → fail-closed)"
  else
    fail "H5c: expected rc=1 + remediation; got rc=$rc, output: $out"
  fi

  # Bypass also works with empty file → exit 0 on clean range
  BTCTAX_PII_BYPASS=1 \
    bash -c "cd '$TMPWS' && printf 'refs/heads/main %s refs/heads/main %s\n' '$TIP' '$BASE' \
      | bash ./pre-push origin" 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "H5c-bypass (empty + bypass + clean → exit 0)" || fail "H5c-bypass: expected rc=0; got $rc"
  cleanup_ws
}

# KAT-H6: hook — owner-specific hit in range → exits 1
{
  new_ws
  commit_file "readme.txt" "clean" "base"
  BASE="$TIP"
  commit_file "data.txt" "SYNTHETIC-OWNER-42 here" "add owner PII"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "H6 (owner-specific hit → exit 1)" || fail "H6: expected rc=1; got $rc"
  cleanup_ws
}

# KAT-H7: hook — owner-specific pattern present, no match in range → exit 0
{
  new_ws
  commit_file "readme.txt" "clean" "base"
  BASE="$TIP"
  commit_file "notes.txt" "nothing suspicious here" "clean"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "H7 (owner-specific — no match → exit 0)" || fail "H7: expected rc=0; got $rc"
  cleanup_ws
}

# KAT-H8: LICENSE carve-out [R0-I4]
#   sub-case 1: owner pattern matches ONLY in LICENSE → exit 0
#   sub-case 2: same content also in another file → exit 1
{
  new_ws
  commit_file "readme.txt" "clean" "base"
  BASE="$TIP"
  commit_file "LICENSE" "SYNTHETIC-OWNER-99 copyright holder" "add LICENSE"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "H8-1 (LICENSE carve-out — only in LICENSE → exit 0)" || fail "H8-1: expected rc=0; got $rc"
  cleanup_ws

  new_ws
  commit_file "readme.txt" "clean" "base"
  BASE="$TIP"
  commit_file "LICENSE" "SYNTHETIC-OWNER-99 copyright holder" "add LICENSE"
  commit_file "other.txt" "SYNTHETIC-OWNER-99 also here" "also in other file"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  printf 'refs/heads/main %s refs/heads/main %s\n' "$TIP" "$BASE" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 1 ] && pass "H8-2 (LICENSE carve-out — also in other file → exit 1)" || fail "H8-2: expected rc=1; got $rc"
  cleanup_ws
}

# KAT-H9: deleted ref — all-zeros local SHA → skipped, exit 0
{
  new_ws
  commit_file "readme.txt" "clean" "init"
  printf 'SYNTHETIC-OWNER-[0-9]+\n' > "$TMPWS/.pii-patterns"
  # Local SHA = zeros → deletion, must be skipped
  printf 'refs/heads/old %s refs/heads/old abc123def456\n' "$ZERO" \
    | run_hook origin 2>/dev/null && rc=0 || rc=$?
  [ "$rc" -eq 0 ] && pass "H9 (deleted ref → skipped, exit 0)" || fail "H9: expected rc=0; got $rc"
  cleanup_ws
}

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed."
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
