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
#     44-4444444   — repeated-digit synthetic EIN, the SECOND employer in the §6413(c) per-employer
#                    probes (btctax-core/src/tax/return_1040.rs, reviews/branch-r8-ink-opus.md). The
#                    "silent second employer who withheld nothing" case needs an EIN distinct from
#                    every other, which is the whole point of that probe.
#     33-3333333   — repeated-digit synthetic EIN, the SPOUSE's employer in the §6413(c)
#                    excess-SS test (btctax-core/src/tax/return_1040.rs). Three distinct EINs
#                    are needed there: the credit turns on "more than one employer", so the
#                    test cannot state the rule with fewer.
#     12-3456789   — sequential synthetic EIN / appraiser TIN (btctax-core/src/donation.rs,
#                    btctax-cli/src/cmd/reconcile.rs, .../donation_details.rs, .../render.rs, kat_forms.rs)
#     98-7654321   — synthetic donee EIN in the §170 donation worked examples
#                    (crates/xtask/src/examples.rs J2/J5; deliberately synthetic so the
#                    shipped docs carry no real organization identifier)
#     99-1234567   — second synthetic sequential EIN (btctax-cli/tests/tax_report.rs) [R0-I1]
# NOT excluded (cannot match the shapes; documented only): the bare 9-digit TIN
# and the alphanumeric PTIN used in the same fixtures.
# ── SSN exclusion, part 1: a STRUCTURAL rule, not an enumeration ─────────────
# An SSN the SSA can NEVER have issued cannot be any real person's, so it is safe
# by construction and needs no per-token approval. This is the repo's standing
# rule (CLAUDE.md, the oracle excuse lists): state the mechanism, let it decide,
# never enumerate the outcomes you happened to see. A test author needing a fresh
# synthetic SSN now picks one from this space and edits nothing here.
#
#   area 000 or 666      — never issued, and never an ITIN prefix
#   group 00             — never issued (no valid SSN or ITIN has it)
#   serial 0000          — never issued
#   987-65-432x          — the SSA's own reserved advertising block
#
# ★★★ THE ITIN TRAP, and why the never-issued area range 900-999 is NOT here.
#     Area 9xx is never an SSN — but it IS the ITIN space (9NN-NN-NNNN, groups
#     70-88/90-92/94-99), and an ITIN is a REAL taxpayer identifier belonging to a
#     real person. Allowing 9xx wholesale would allowlist every real ITIN, in a tax
#     application, which is precisely the leak this scan exists to stop. So the
#     group/serial rules are anchored to a NON-9 area ([0-8][0-9]{2}) and 9xx is
#     admitted nowhere. Widening an exemption is never the safe edit.
ALLOWED_SSN_IMPOSSIBLE='^(000|666)-[0-9]{2}-[0-9]{4}$|^[0-8][0-9]{2}-00-[0-9]{4}$|^[0-8][0-9]{2}-[0-9]{2}-0000$|^987-65-432[0-9]$'

# ── SSN exclusion, part 2: the LEGACY tokens, and why this list is permanent ──
# These five are valid-SHAPED SSNs (they break no SSA rule), so the structural
# rule above cannot admit them and must not be bent to. They predate it.
#
# ★★ They can never be retired, and the reason is worth stating so nobody spends
#    an afternoon trying. Each appears not only in live code but in PERSISTED
#    REVIEW ARTIFACTS — design/**/reviews/*.md, reviews/branch-r9-b2-opus.md,
#    design/no-testimony/CONSULT-architect-fable.md — which the workflow requires
#    be kept VERBATIM. Migrating every call site would still leave the tokens in
#    the tree, and editing a reviewer's persisted output to clear a scan would
#    falsify the record to make a gate green. So: closed set, never extended.
#    A NEW synthetic SSN goes in the structural space, not here.
ALLOWED_SSN_LEGACY='^(111-11-1111|111-22-3333|123-45-6789|222-22-2222|222-33-4444)$'

# ── SSN exclusion, part 3: tokens that survive ONLY IN UNPUSHED HISTORY ──────
# ★★★ NONE OF THESE APPEARS AT HEAD. Grep the working tree and you will not find
#     them — which is exactly why they need a comment rather than just a listing.
#     `pre-push` scans EVERY COMMIT in the pushed range, not the tip, so a token
#     that was introduced and later removed still blocks the push from the
#     intermediate commit that carried it. Fixing HEAD does not fix history.
#
#     All three are synthetic; the alternative was rewriting the commits, and the
#     owner chose the allowlist (2026-08-03).
#
#   111-22-0004  — synthetic dependent SSN in a rendered Form 1040 dependents
#   111-22-0009    table, reviews/branch-r9-b2-opus.md. Since removed from that
#                  file; the review commit that introduced it is in the range.
#   333-44-5555  — synthetic SSN in the §G-6 AMT fixture (btctax-core
#                  testonly.rs) and quoted in a FOLLOWUPS entry. Both replaced
#                  at HEAD — the fixture now uses an allowed number and the
#                  entry names the class instead of the token — but commits
#                  942120e/3d53ecd still carry it.
#
# ★ This bucket is CLOSED and should shrink to nothing the moment these commits
#   become public (once pushed, they leave the pushed range and are never
#   rescanned). It is scaffolding for one push, not a permanent exemption. A new
#   synthetic SSN still belongs in the structural space above, never here.
ALLOWED_SSN_UNPUSHED_HISTORY='^(111-22-0004|111-22-0009|333-44-5555)$'

# ── EIN exclusion (2-7 shape) ────────────────────────────────────────────────
# No structural rule is available: the IRS has issued prefixes across nearly the
# whole 2-digit space, so there is no "impossible EIN" to build a mechanism from.
# Token-exact, with a citation comment above, remains correct here.
#   56-1234567   — synthetic shared employer EIN in the four-W-2 §6413(c) worked
#                  example, design/direction/FILING-TRIAL-2026-08-02.md. Same
#                  unpushed-history case as the SSN bucket above: absent at HEAD,
#                  present in the commit that introduced the trial write-up.
# ── EIN exclusion, part 2: synthetic EINs QUOTED IN PERSISTED REVIEW ARTIFACTS ──
# ★★★ These are minted by `btctax_core::tax::scrub::synthetic_ein` and appear inside review documents
#     that the workflow requires be kept VERBATIM (reviews/scrub-r1-workflow.md,
#     reviews/scrub-spec-r1-review.md). Editing a reviewer's persisted output to clear a scan would
#     falsify the record to make a gate green — the same refusal recorded for the SSN legacy list.
#
#   90-0000001  — the EIN-splitting reproduction in the scrub code review: one employer's two W-2
#   91-0000002    spellings became TWO synthetic employers, manufacturing an excess-SS credit
#   55-5555555  — a worked EIN example inside the scrub SPEC review
#
# ★ THIS BUCKET IS RESIDUE, NOT A PATTERN. `design/SPEC_income_scrub.md` §7 moves `synthetic_ein`
#   into a documented STRUCTURAL window so future output needs no entry here at all. These three
#   predate that and are permanent only because the documents quoting them are.
ALLOWED_EIN_REVIEW_ARTIFACT='^(90-0000001|91-0000002|55-5555555)$'

ALLOWED_EIN='^(11-1111111|22-2222222|33-3333333|44-4444444|12-3456789|98-7654321|99-1234567|56-1234567)$'

ALLOWED="$ALLOWED_SSN_IMPOSSIBLE|$ALLOWED_SSN_LEGACY|$ALLOWED_SSN_UNPUSHED_HISTORY|$ALLOWED_EIN|$ALLOWED_EIN_REVIEW_ARTIFACT"

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
