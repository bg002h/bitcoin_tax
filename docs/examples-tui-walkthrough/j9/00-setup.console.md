```console
$ btctax --vault v.pgp init --key-backup key-backup.asc
Initialized vault v.pgp (key backed up to key-backup.asc)
```
```console
$ btctax --vault v.pgp import coinbase.csv
Import:
  coinbase [coinbase.csv]: parsed 3 rows -> 3 BTC events (0 dropped no-BTC, 0 unclassified)
  appended 3 | duplicates 0 | NEW import-conflicts 0
```
```console
$ btctax --vault v.pgp verify
Conservation (FR9): BALANCED
  in 100000000 = disposed 50000000 + removed 0 + held 50000000 + fee-sats 0 + pending 0
2025 transition: Path A (actual per-wallet reconstruction; default, no election)
Pending reconciliation: 0 transfer(s); unknown-basis inbounds: 0
Hard blockers (gate tax computation): 0
Advisory blockers: 0
Pre-2025 method (attested historical fact): HIFO (attested: false)
Standing orders (MethodElection): 0
Lot selections recorded: 0
Per-disposal compliance (post-2025): 1
  import|coinbase|trade|sale @ 2025-06-01 :: non_compliant
```
