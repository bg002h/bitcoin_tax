#!/usr/bin/env bash
# Track B remainder: IRC statute (govinfo USCODE-2024), Treasury Regs (eCFR),
# Federal Register TD 10000, and two late-surfaced IRS docs. Provenance appended to fetch_log.tsv.
set -u
ROOT=/scratch/code/bitcoin_tax/legal/primary-sources
LOG=/scratch/code/bitcoin_tax/legal/_provenance/fetch_log.tsv
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

echo "# --- Track B remainder fetched $(date -u +%Y-%m-%dT%H:%M:%SZ) ---" >> "$LOG"

# ---- IRC statute: govinfo USCODE-2024 title-26 per-section HTML granules ----
GB="https://www.govinfo.gov/content/pkg/USCODE-2024-title26/html/USCODE-2024-title26"
declare -A P=(
 [1]="subtitleA-chap1-subchapA-partI-sec1"
 [170]="subtitleA-chap1-subchapB-partVI-sec170"
 [1001]="subtitleA-chap1-subchapO-partI-sec1001"
 [1011]="subtitleA-chap1-subchapO-partII-sec1011"
 [1012]="subtitleA-chap1-subchapO-partII-sec1012"
 [1015]="subtitleA-chap1-subchapO-partII-sec1015"
 [1016]="subtitleA-chap1-subchapO-partII-sec1016"
 [1031]="subtitleA-chap1-subchapO-partIII-sec1031"
 [1091]="subtitleA-chap1-subchapO-partVII-sec1091"
 [1211]="subtitleA-chap1-subchapP-partII-sec1211"
 [1212]="subtitleA-chap1-subchapP-partII-sec1212"
 [1221]="subtitleA-chap1-subchapP-partIII-sec1221"
 [1222]="subtitleA-chap1-subchapP-partIII-sec1222"
 [1411]="subtitleA-chap2A-sec1411"
)
for s in 1 170 1001 1011 1012 1015 1016 1031 1091 1211 1212 1221 1222 1411; do
  dl "${GB}-${P[$s]}.htm" "statute-irc/26USC_s${s}.html"
done

# ---- Treasury Regulations: eCFR versioner API (snapshot 2025-12-01), Title 26 part 1 ----
EB="https://www.ecfr.gov/api/versioner/v1/full/2025-12-01/title-26.xml?part=1&section="
dl "${EB}1.1012-1"     "regulations-cfr/26CFR_1.1012-1_basis.xml"
dl "${EB}1.6045-1"     "regulations-cfr/26CFR_1.6045-1_broker_reporting.xml"
dl "${EB}1.1091-1"     "regulations-cfr/26CFR_1.1091-1_wash_sales.xml"
dl "${EB}1.1031(a)-1"  "regulations-cfr/26CFR_1.1031a-1_like_kind.xml"
dl "${EB}1.1015-1"     "regulations-cfr/26CFR_1.1015-1_gift_basis.xml"
dl "${EB}1.170A-13"    "regulations-cfr/26CFR_1.170A-13_charitable_records.xml"

# ---- Federal Register: TD 10000 (final digital-asset broker regs, 89 FR 56480) ----
dl "https://www.govinfo.gov/content/pkg/FR-2024-07-09/pdf/2024-14004.pdf" \
   "federal-register/TD_10000_89FR56480_broker_regs.pdf"

# ---- Two late-surfaced IRS guidance docs ----
dl "https://www.irs.gov/pub/irs-drop/n-26-20.pdf"   "irs-guidance/Notice_2026-20.pdf"
dl "https://www.irs.gov/pub/irs-wd/202124008.pdf"   "irs-guidance/CCA_202124008.pdf"

echo "=== verify file types ==="
for f in statute-irc/26USC_s1012.html regulations-cfr/26CFR_1.1012-1_basis.xml \
         regulations-cfr/26CFR_1.6045-1_broker_reporting.xml \
         federal-register/TD_10000_89FR56480_broker_regs.pdf \
         irs-guidance/Notice_2026-20.pdf irs-guidance/CCA_202124008.pdf; do
  [ -f "$ROOT/$f" ] && file "$ROOT/$f"
done