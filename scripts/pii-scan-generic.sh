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
# Every value here is a SYNTHETIC stand-in in test fixtures / worked-example inputs
# — never real filer PII (real fixtures mask SSNs to ***-**-XXXX before any output).
# If a new synthetic shape-matching value enters the test suite, it MUST be added
# here with a citation comment.
#   SSN-shaped (3-2-4):
#     000-00-0000  — all-zeros placeholder (scripts/oracle/ots_direct.py)
#     111-11-1111  — repeated-digit synthetic SSN (btctax-forms/tests/sp2.rs)
#     111-22-3333  — synthetic second-person SSN (btctax-cli/src/cmd/tax.rs,
#                    btctax-core/src/tax/testonly.rs, tests/fixtures/.../fullreturn_inputs.toml)
#     123-45-6789  — canonical sequential fake SSN, primary filer in the KATs
#                    (btctax-core/src/tax/testonly.rs, btctax-forms/tests/*)
#     222-22-2222  — synthetic SSN / appraiser TIN (btctax-forms/tests/sp2.rs,
#                    btctax-forms/tests/extract_lines.rs)
#     222-33-4444  — synthetic SSN (btctax-cli/tests/export_irs_pdf.rs,
#                    btctax-core/src/tax/testonly.rs)
#     987-65-4321  — SSA-reserved never-issued SSN (btctax-core/src/donation.rs, kat_forms.rs)
#   EIN-shaped (2-7):
#     11-1111111   — repeated-digit synthetic EIN, "Charity Alpha" (btctax-forms/tests/sp2.rs)
#     22-2222222   — repeated-digit synthetic EIN, "Charity Beta" (btctax-forms/tests/sp2.rs)
#     12-3456789   — sequential synthetic EIN / appraiser TIN (btctax-core/src/donation.rs,
#                    btctax-cli/src/cmd/reconcile.rs, .../donation_details.rs, .../render.rs, kat_forms.rs)
#     98-7654321   — synthetic donee EIN in the §170 donation worked examples
#                    (crates/xtask/src/examples.rs J2/J5; deliberately synthetic so the
#                    shipped docs carry no real organization identifier)
#     99-1234567   — second synthetic sequential EIN (btctax-cli/tests/tax_report.rs) [R0-I1]
# NOT excluded (cannot match the shapes; documented only): the bare 9-digit TIN
# and the alphanumeric PTIN used in the same fixtures.
ALLOWED='^(000-00-0000|111-11-1111|111-22-3333|123-45-6789|222-22-2222|222-33-4444|987-65-4321|11-1111111|22-2222222|12-3456789|98-7654321|99-1234567)$'

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
