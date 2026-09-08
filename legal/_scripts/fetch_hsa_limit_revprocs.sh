#!/usr/bin/env bash
# ★ The §223(b)(2) HSA annual contribution limitation — `FullReturnParams::hsa`'s self-only and
# family figures — is NOT in the annual inflation revenue procedure that carries the brackets and the
# standard deduction (Rev. Proc. 2023-34 / 2025-32). §223(g) requires the HSA amounts to be published
# by June 1 of the PRECEDING year, so they arrive in their own Revenue Procedure each spring, a year
# and a half ahead of the return. Fetched 2026-09-07 for interview T16 (Form 8889) so the TY2024
# citation (Rev. Proc. 2023-23) and the TY2026 one (Rev. Proc. 2025-19) each resolve to a HELD
# document. Rev. Proc. 2024-25 is the TY2025 figures, fetched with them so the year the packet is
# PAUSED on has its authority in hand rather than pending. Same `dl` helper and provenance log as
# fetch_retirement_limit_notices.sh.
set -u
ROOT="$(cd "$(dirname "$0")/../primary-sources" && pwd)"
LOG="$(cd "$(dirname "$0")/../_provenance" && pwd)/fetch_log.tsv"
UA="Mozilla/5.0 (legal-archive bot; bitcoin_tax)"

dl () { # dl <url> <relpath>
  local url="$1" rel="$2" out="$ROOT/$2" hdr code ctype bytes sha
  hdr=$(curl -fsSL --retry 3 --retry-delay 2 --max-time 180 -A "$UA" \
        -w "%{http_code}\t%{content_type}" -o "$out" "$url" 2>/dev/null)
  local rc=$?
  if [ $rc -ne 0 ] || [ ! -s "$out" ]; then
    echo -e "ERR(rc=$rc)\t0\t-\t-\t$rel\t$url" | tee -a "$LOG"; rm -f "$out"; return
  fi
  ctype=$(echo "$hdr" | cut -f2); code=$(echo "$hdr" | cut -f1)
  bytes=$(stat -c%s "$out"); sha=$(sha256sum "$out" | cut -d' ' -f1)
  echo -e "$code\t$bytes\t$sha\t$ctype\t$rel\t$url" | tee -a "$LOG"
}

echo "# --- §223(b)(2) HSA limit revenue procedures fetched $(date -u +%Y-%m-%dT%H:%M:%SZ) ---" >> "$LOG"
dl https://www.irs.gov/pub/irs-drop/rp-23-23.pdf irs-guidance/RevProc_2023-23.pdf   # TY2024 limits
dl https://www.irs.gov/pub/irs-drop/rp-24-25.pdf irs-guidance/RevProc_2024-25.pdf   # TY2025 limits
dl https://www.irs.gov/pub/irs-drop/rp-25-19.pdf irs-guidance/RevProc_2025-19.pdf   # TY2026 limits
