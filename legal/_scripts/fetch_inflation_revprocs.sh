#!/usr/bin/env bash
# ★★★ The annual INFLATION-ADJUSTMENT revenue procedures — the authority behind every bracket,
# breakpoint and threshold in `crates/btctax-adapters/src/tax_tables.rs`.
#
# Why this script exists: `shipped_tables_are_the_validated_tables.rs` found that the binary ships
# tax tables for four years while only ONE has a validated counterpart. Closing that gap means
# transcribing each year's numbers from its revenue procedure — and NONE of those procedures was in
# the repo. The `source` field of every shipped table cited authority nobody could open.
#
# ★ A table validated against a document we do not hold is not validated. Fetch, then transcribe.
# ★ NEVER close the gap by copying `tax_tables.rs` into the corpus — that is an echo, not a witness.
set -u
ROOT="$(cd "$(dirname "$0")/../primary-sources" && pwd)"
UA="Mozilla/5.0 (legal-archive bot; bitcoin_tax)"

dl () {
  local url="$1" rel="$2" out="$ROOT/$2" code bytes sha
  code=$(curl -fsSL --retry 3 --retry-delay 2 --max-time 120 -A "$UA" \
        -w "%{http_code}" -o "$out" "$url" 2>/dev/null) || { echo -e "ERR\t-\t$rel\t$url"; rm -f "$out"; return; }
  [ -s "$out" ] || { echo -e "EMPTY\t-\t$rel\t$url"; rm -f "$out"; return; }
  head -c4 "$out" | grep -q '%PDF' || { echo -e "NOTPDF\t-\t$rel\t$url"; rm -f "$out"; return; }
  bytes=$(stat -c%s "$out"); sha=$(sha256sum "$out" | cut -d' ' -f1)
  printf '%s\t%s\t%s\t%s\t%s\n' "$code" "$bytes" "${sha:0:16}" "$rel" "$url"
}

# One per shipped tax year. The mapping year -> procedure is a LOOKUP, not a formula: the procedure
# is published in the PRECEDING calendar year, and its number is not derivable from the tax year.
dl https://www.irs.gov/pub/irs-drop/rp-16-55.pdf irs-guidance/RevProc_2016-55.pdf   # TY2017
dl https://www.irs.gov/pub/irs-drop/rp-23-34.pdf irs-guidance/RevProc_2023-34.pdf   # TY2024
dl https://www.irs.gov/pub/irs-drop/rp-24-40.pdf irs-guidance/RevProc_2024-40.pdf   # TY2025
dl https://www.irs.gov/pub/irs-drop/rp-25-32.pdf irs-guidance/RevProc_2025-32.pdf   # TY2026

# ---- SSA OASDI contribution-and-benefit base (the SS wage base) ----
# ★ NOT in any revenue procedure. It is an SSA determination published in the Federal Register as
#   "Cost-of-Living Increase and Other Determinations for <year>". Fetched from govinfo because
#   ssa.gov returns 403 to an archival user-agent.
# ★★ `tax_tables.rs` cites dates ~10 days EARLIER than these (e.g. "SSA 2023-10-12" vs FR
#    2023-10-23). Those are the SSA PRESS RELEASE dates; the Federal Register notice is the
#    published determination. Both are real artifacts — the citation names a different one.
dl https://www.govinfo.gov/content/pkg/FR-2016-10-27/pdf/2016-26026.pdf federal-register/SSA_COLA_Determinations_2017.pdf
dl https://www.govinfo.gov/content/pkg/FR-2023-10-23/pdf/2023-23317.pdf federal-register/SSA_COLA_Determinations_2024.pdf
dl https://www.govinfo.gov/content/pkg/FR-2024-10-25/pdf/2024-24871.pdf federal-register/SSA_COLA_Determinations_2025.pdf
dl https://www.govinfo.gov/content/pkg/FR-2025-11-03/pdf/2025-19763.pdf federal-register/SSA_COLA_Determinations_2026.pdf
