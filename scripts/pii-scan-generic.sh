#!/usr/bin/env bash
# Generic-shape PII scan — SSN-like (3-2-4 digits) and EIN-like (2-7 digits) tokens.
# The ONLY place the shapes and the exclusion list exist (CI job + pre-push hook both call this).
# Interface: pii-scan-generic.sh [<rev>]
#   exits 0 = clean, 1 = hit(s) found (locations printed to stderr), 2 = scan error.
set -euo pipefail
REV="${1:-HEAD}"

# ── Shapes (ERE; hyphen-delimited digit groups) ──────────────────────────────
SHAPES='\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b|\b[0-9]{2}-[0-9]{7}\b'

# ── Exclusion list: documented synthetic test stand-ins (token-exact) ───────
# If a new synthetic shape-matching value enters the test suite, it MUST be added
# here with a citation comment.
#   987-65-4321  — SSA-reserved never-issued SSN (donation.rs:94, kat_forms.rs:1102)
#   12-3456789   — sequential synthetic EIN (donation.rs:91, reconcile.rs:666,
#                  donation_details.rs:110, render.rs:2926/2970, kat_forms.rs:1099)
#   99-1234567   — second synthetic sequential EIN (tests/tax_report.rs:786) [R0-I1]
# NOT excluded (cannot match the shapes; documented only): the bare 9-digit TIN
# and the alphanumeric PTIN used in the same fixtures.
ALLOWED='^(987-65-4321|12-3456789|99-1234567)$'

# Token-level extraction [R0-M1]: -o emits only matched tokens; exclusions filter
# tokens, not lines. -I skips binaries [R0-M3]; git grep is tree-accurate and
# NUL-safe by construction.
set +e
tokens=$(git grep -IhoE "$SHAPES" "$REV" -- | sort -u)
gs=$?
set -e
[ "$gs" -gt 1 ] && { echo "pii-scan: git grep failed (status $gs)" >&2; exit 2; }

bad=$(printf '%s\n' "$tokens" | grep -vE "$ALLOWED" | grep -v '^$' || true)
if [ -n "$bad" ]; then
  echo "pii-scan: non-excluded PII-shaped token(s) in $REV:" >&2
  # Second pass for actionable file:line locations of exactly the bad tokens.
  # [R0-M7] -e "$tok" BEFORE the rev, pathspec separator last — putting the token
  # after -- would parse it as a pathspec and blank the diagnostics.
  while IFS= read -r tok; do git grep -InF -e "$tok" "$REV" -- >&2 || true; done <<<"$bad"
  exit 1
fi
echo "pii-scan: clean ($REV)."
