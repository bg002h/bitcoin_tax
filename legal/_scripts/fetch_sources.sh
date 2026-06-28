#!/usr/bin/env bash
# Download primary-authority tax documents with provenance for legal defense.
set -u
ROOT=/scratch/code/bitcoin_tax/legal/primary-sources
LOG=/tmp/claude-1000/-scratch-code-bitcoin-tax/71ab70cd-f674-4e66-86de-cbc9cc49e1a8/scratchpad/fetch_log.tsv
RETRIEVED="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo -e "status\tbytes\tsha256\tcontent_type\tpath\turl" > "$LOG"

dl () {
  # dl <url> <relpath>
  local url="$1" rel="$2" out="$ROOT/$2"
  local code ctype bytes sha
  code=$(curl -fsSL --retry 3 --retry-delay 2 --max-time 120 \
        -A "Mozilla/5.0 (legal-archive bot; bitcoin_tax)" \
        -w "%{http_code}\t%{content_type}" -o "$out" "$url" 2>/dev/null)
  local rc=$?
  if [ $rc -ne 0 ] || [ ! -s "$out" ]; then
    echo -e "ERR(rc=$rc)\t0\t-\t-\t$rel\t$url" | tee -a "$LOG"
    rm -f "$out"
    return
  fi
  ctype=$(echo "$code" | cut -f2)
  code=$(echo "$code" | cut -f1)
  bytes=$(stat -c%s "$out")
  sha=$(sha256sum "$out" | cut -d' ' -f1)
  echo -e "$code\t$bytes\t$sha\t$ctype\t$rel\t$url" | tee -a "$LOG"
}

# ---- IRS sub-regulatory guidance (the crypto-specific core) ----
dl https://www.irs.gov/pub/irs-drop/n-14-21.pdf  irs-guidance/Notice_2014-21.pdf
dl https://www.irs.gov/pub/irs-drop/n-23-34.pdf  irs-guidance/Notice_2023-34.pdf
dl https://www.irs.gov/pub/irs-drop/rr-19-24.pdf irs-guidance/RevRul_2019-24.pdf
dl https://www.irs.gov/pub/irs-drop/rr-23-14.pdf irs-guidance/RevRul_2023-14.pdf
dl https://www.irs.gov/pub/irs-drop/rp-24-28.pdf irs-guidance/RevProc_2024-28.pdf
dl https://www.irs.gov/pub/irs-drop/n-25-07.pdf  irs-guidance/Notice_2025-07.pdf
dl https://www.irs.gov/pub/irs-drop/n-24-56.pdf  irs-guidance/Notice_2024-56.pdf
dl https://www.irs.gov/pub/irs-drop/n-24-57.pdf  irs-guidance/Notice_2024-57.pdf

# ---- IRS Publications ----
dl https://www.irs.gov/pub/irs-pdf/p544.pdf irs-publications/Pub544_Sales_and_Other_Dispositions.pdf
dl https://www.irs.gov/pub/irs-pdf/p551.pdf irs-publications/Pub551_Basis_of_Assets.pdf
dl https://www.irs.gov/pub/irs-pdf/p525.pdf irs-publications/Pub525_Taxable_Nontaxable_Income.pdf
dl https://www.irs.gov/pub/irs-pdf/p550.pdf irs-publications/Pub550_Investment_Income_Expenses.pdf
dl https://www.irs.gov/pub/irs-pdf/p526.pdf irs-publications/Pub526_Charitable_Contributions.pdf
dl https://www.irs.gov/pub/irs-pdf/p561.pdf irs-publications/Pub561_Value_of_Donated_Property.pdf

# ---- IRS Forms & instructions ----
dl https://www.irs.gov/pub/irs-pdf/f8949.pdf   irs-forms/Form_8949.pdf
dl https://www.irs.gov/pub/irs-pdf/i8949.pdf   irs-forms/Instructions_8949.pdf
dl https://www.irs.gov/pub/irs-pdf/f1040sd.pdf irs-forms/Schedule_D_1040.pdf
dl https://www.irs.gov/pub/irs-pdf/i1040sd.pdf irs-forms/Instructions_Schedule_D.pdf
dl https://www.irs.gov/pub/irs-pdf/f1099da.pdf irs-forms/Form_1099-DA.pdf
dl https://www.irs.gov/pub/irs-pdf/i1099da.pdf irs-forms/Instructions_1099-DA.pdf
dl https://www.irs.gov/pub/irs-pdf/f8283.pdf   irs-forms/Form_8283_Noncash_Charitable.pdf

echo "RETRIEVED_UTC=$RETRIEVED"
echo "=== file types ==="
find "$ROOT" -type f \( -name '*.pdf' \) -exec file {} \;