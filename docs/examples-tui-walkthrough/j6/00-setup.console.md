```console
$ btctax --vault v.pgp init --key-backup key-backup.asc
Initialized vault v.pgp (key backed up to key-backup.asc)
```
```console
$ btctax --vault v.pgp import coinbase.csv river.csv
Import:
  coinbase [coinbase.csv]: parsed 3 rows -> 3 BTC events (0 dropped no-BTC, 0 unclassified)
  river [river.csv]: parsed 1 rows -> 1 BTC events (0 dropped no-BTC, 0 unclassified)
  appended 4 | duplicates 0 | NEW import-conflicts 0
```
```console
$ btctax --vault v.pgp reconcile reclassify-income "import|river|in|1710504000000|income|5000000#0" --business true --kind mining
Recorded decision decision|1
```
```console
$ btctax --vault v.pgp reconcile reclassify-outflow "import|coinbase|out|cb-donate" --as-kind donate --amount 6000.00 --donee "Habitat for Humanity"
Recorded decision decision|2
```
```console
$ btctax --vault v.pgp reconcile set-donation-details "import|coinbase|out|cb-donate" --donee-name "Habitat for Humanity" --donee-ein 98-7654321 --appraiser-name "Jane Appraiser" --appraiser-tin 12-3456789 --appraiser-qualifications "ASA-accredited digital-asset appraiser, 8 yrs" --appraisal-date 2024-09-15
Donation details saved for import|coinbase|out|cb-donate.
```
```console
$ btctax --vault v.pgp verify
Conservation (FR9): BALANCED
  in 35000000 = disposed 5000000 + removed 10000000 + held 20000000 + fee-sats 0 + pending 0
2025 transition: Path A (actual per-wallet reconstruction; default, no election)
Pending reconciliation: 0 transfer(s); unknown-basis inbounds: 0
Hard blockers (gate tax computation): 0
Advisory blockers: 2
  [Pre2025MethodNote] import|coinbase|trade|cb-sell :: pre-2025 lots reconstructed under HIFO (FIFO is the §7.4 legal default); you have NOT declared your filed pre-2025 lot method — if your filed pre-2025 returns used a different method your carryforward basis may differ. Declare it: config --set-pre2025-method <m> --attest-pre2025-method
  [QualifiedAppraisalNote] import|coinbase|out|cb-donate :: Claimed deduction $6000.00 exceeds the §170(f)(11)(C) $5,000 threshold. Qualified appraisal likely required: CCA 202302012 — a crypto donation with a claimed deduction >$5,000 requires a qualified appraisal; the exchange-price/readily-valued exception does NOT apply to crypto. This is the exact §170(e) deduction for a non-dealer individual investor donating a capital asset (LT→FMV; ST→min(FMV,basis)). Caveat (a) dealer/inventory: crypto held as inventory/for sale in a trade or business (§1221(a)(1)) or other ordinary-income property deducts at basis under §170(e) REGARDLESS of holding period — this figure assumes capital-asset (investor) status and would OVER-STATE for a dealer; verify. Caveat (b) donee type: LT→FMV assumes a public charity (50%-limit org); a non-operating private foundation reduces appreciated LT crypto to basis (§170(e)(1)(B)(ii); crypto is not qualified appreciated stock) — donee type is not modeled; would OVER-STATE for a private-foundation gift; verify. §170(f)(11)(F) aggregation: this flags a single donation; the $5,000 test also aggregates similar donated items across the tax year — cross-donation aggregation is not considered here.
Pre-2025 method (attested historical fact): HIFO (attested: false)
Standing orders (MethodElection): 0
Lot selections recorded: 0
Per-disposal compliance (post-2025): 0
Promote-basis drift advisories: 0
```
```console
$ btctax --vault v.pgp income import --year 2024 --file fullreturn.toml
Imported full-return inputs for tax year 2024.
```
