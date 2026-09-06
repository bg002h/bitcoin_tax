#!/usr/bin/env bash
# ★ The §402(g)(1) elective-deferral limit — `FullReturnParams::elective_deferral_limit` — is NOT in the
# annual inflation revenue procedure. It is set by a separate IRS NOTICE each autumn ("<year> Amounts
# Relating to Retirement Plans and IRAs, as Adjusted for Changes in Cost-of-Living"). Fetched 2026-09-05
# so the TY2024 citation (Notice 2023-75) resolves to a held document and TY2026 (Notice 2025-67) can be
# transcribed. Same `dl` helper and provenance log as fetch_statute_55d_obbba.sh.
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

echo "# --- §402(g) retirement-limit notices fetched $(date -u +%Y-%m-%dT%H:%M:%SZ) ---" >> "$LOG"
dl https://www.irs.gov/pub/irs-drop/n-23-75.pdf irs-guidance/Notice_2023-75.pdf   # TY2024 limits
dl https://www.irs.gov/pub/irs-drop/n-25-67.pdf irs-guidance/Notice_2025-67.pdf   # TY2026 limits
