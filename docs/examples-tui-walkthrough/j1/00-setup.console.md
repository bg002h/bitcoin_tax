```console
$ btctax --vault v.pgp init --key-backup key-backup.asc
Initialized vault v.pgp (key backed up to key-backup.asc)
```
```console
$ btctax --vault v.pgp import coinbase.csv
Import:
  coinbase [coinbase.csv]: parsed 2 rows -> 2 BTC events (0 dropped no-BTC, 0 unclassified)
  appended 2 | duplicates 0 | NEW import-conflicts 0
```
```console
$ btctax --vault v.pgp verify
Conservation (FR9): BALANCED
  in 10000000 = disposed 2000000 + removed 0 + held 8000000 + fee-sats 0 + pending 0
2025 transition: Path A (actual per-wallet reconstruction; default, no election)
Pending reconciliation: 0 transfer(s); unknown-basis inbounds: 0
Hard blockers (gate tax computation): 0
Advisory blockers: 0
Pre-2025 method (attested historical fact): HIFO (attested: false)
Standing orders (MethodElection): 0
Lot selections recorded: 0
Per-disposal compliance (post-2025): 1
  import|coinbase|trade|cb-sell @ 2025-06-15 :: non_compliant
Promote-basis drift advisories: 0
```
```console
$ btctax --vault v.pgp tax-profile --year 2025 --filing-status single --ordinary-taxable-income 100000 --magi-excluding-crypto 100000 --qualified-dividends 0
Tax profile for 2025 saved.
```
