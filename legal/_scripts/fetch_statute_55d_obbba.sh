#!/usr/bin/env bash
# ★★★ 26 U.S.C. §55(d) as AMENDED by the One Big Beautiful Bill Act — the authority behind the TY2026
# AMT exemption phase-out RATE (0.50) and the permanent §55(d)(4) reset in `tax_tables.rs`.
#
# Why this script exists (2026-09-05, Fable plan review I4 → FR-47): the repo archived sixteen IRC
# sections in the govinfo USCODE-2024 edition and NOT §55 — the one section every AMT figure rests on.
# And the 2024 edition is PRE-OBBBA: Pub. L. 119-21 (2025-07-04) §70107(c) substitutes "50 percent"
# for "25 percent" in §55(d)(4)(A)(ii), effective for taxable years beginning after 2025-12-31. So
# three documents, not one:
#   1. §55, USCODE-2024 edition — the BASE text, same edition as its sixteen siblings (pre-amendment).
#   2. Pub. L. 119-21 itself — the AMENDING text; §70107 is at 139 Stat. 162-163.
#   3. §55, OLRC "prelim" edition — the CONSOLIDATED post-amendment text (reflects Pub. L. 119-21).
# ★ 1 + 2 are the law; 3 is the Office of the Law Revision Counsel's consolidation of 1 + 2, archived
#   so a reader can see the amended subsection in one place. Cite 2 for the rate; cite 3 for wording.
# ★ Rev. Proc. 2025-32 §2.10 (already archived) publishes the resulting dollar amounts; it does NOT
#   state the rate. The rate is statute.
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

echo "# --- §55(d) + OBBBA §70107 fetched $(date -u +%Y-%m-%dT%H:%M:%SZ) ---" >> "$LOG"

dl https://www.govinfo.gov/content/pkg/USCODE-2024-title26/html/USCODE-2024-title26-subtitleA-chap1-subchapA-partVI-sec55.htm statute-irc/26USC_s55.html
dl https://www.govinfo.gov/content/pkg/PLAW-119publ21/pdf/PLAW-119publ21.pdf statute-irc/PLAW-119publ21_OBBBA.pdf
dl "https://uscode.house.gov/view.xhtml?req=granuleid:USC-prelim-title26-section55&num=0&edition=prelim" statute-irc/26USC_s55_OLRC-prelim.html
