#!/usr/bin/env bash
set -u
echo "=== eCFR date fix: try a few snapshot dates for Treas. Reg. 1.1012-1 ==="
for d in 2025-12-01 2025-06-01 2025-01-01; do
  code=$(curl -fsSL --max-time 60 "https://www.ecfr.gov/api/versioner/v1/full/$d/title-26.xml?part=1&section=1.1012-1" -o "/tmp/ecfr_$d.xml" -w "%{http_code}" 2>/dev/null)
  sz=$(stat -c%s "/tmp/ecfr_$d.xml" 2>/dev/null || echo 0)
  echo "date=$d http=$code bytes=$sz"
done
echo "--- sample of best eCFR xml ---"
head -c 300 /tmp/ecfr_2025-12-01.xml 2>/dev/null; echo

echo
echo "=== govinfo USCODE granule paths (HEAD) ==="
declare -A P=(
 [sec1]="subtitleA-chap1-subchapA-partI-sec1"
 [sec170]="subtitleA-chap1-subchapB-partVI-sec170"
 [sec1001]="subtitleA-chap1-subchapO-partI-sec1001"
 [sec1011]="subtitleA-chap1-subchapO-partII-sec1011"
 [sec1012]="subtitleA-chap1-subchapO-partII-sec1012"
 [sec1015]="subtitleA-chap1-subchapO-partII-sec1015"
 [sec1016]="subtitleA-chap1-subchapO-partII-sec1016"
 [sec1031]="subtitleA-chap1-subchapO-partIII-sec1031"
 [sec1091]="subtitleA-chap1-subchapO-partVII-sec1091"
 [sec1211]="subtitleA-chap1-subchapP-partII-sec1211"
 [sec1212]="subtitleA-chap1-subchapP-partII-sec1212"
 [sec1221]="subtitleA-chap1-subchapP-partIII-sec1221"
 [sec1222]="subtitleA-chap1-subchapP-partIII-sec1222"
 [sec1411]="subtitleA-chap2A-sec1411"
)
for yr in 2024 2023; do
  echo "-- USCODE-$yr --"
  for k in sec1 sec170 sec1001 sec1011 sec1012 sec1015 sec1016 sec1031 sec1091 sec1211 sec1212 sec1221 sec1222 sec1411; do
    u="https://www.govinfo.gov/content/pkg/USCODE-$yr-title26/html/USCODE-$yr-title26-${P[$k]}.htm"
    code=$(curl -fsIL --max-time 30 "$u" -o /dev/null -w "%{http_code}" 2>/dev/null)
    echo "$yr $k -> $code"
  done
done