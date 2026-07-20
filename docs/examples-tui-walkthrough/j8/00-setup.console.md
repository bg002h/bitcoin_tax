```console
$ btctax --vault v.pgp init --key-backup key-backup.asc
Initialized vault v.pgp (key backed up to key-backup.asc)
```
```console
$ btctax --vault v.pgp import river.csv
Import:
  river [river.csv]: parsed 2 rows -> 2 BTC events (0 dropped no-BTC, 0 unclassified)
  appended 2 | duplicates 0 | NEW import-conflicts 0
```
```console
$ btctax --vault v.pgp import coinbase.csv
Import:
  coinbase [coinbase.csv]: parsed 1 rows -> 1 BTC events (0 dropped no-BTC, 0 unclassified)
  appended 1 | duplicates 0 | NEW import-conflicts 0
```
```console
$ btctax --vault v.pgp verify
Conservation (FR9): BALANCED
  in 10000000 = disposed 0 + removed 0 + held 0 + fee-sats 0 + pending 10000000
2025 transition: Path A (actual per-wallet reconstruction; default, no election)
Pending reconciliation: 1 transfer(s); unknown-basis inbounds: 1
Hard blockers (gate tax computation): 1
  [UnknownBasisInbound] import|coinbase|in|cb-recv :: unclassified TransferIn — basis unknown
Advisory blockers: 1
  [UnmatchedOutflows] import|river|out|1741608000000|withdrawal|10000000#0 :: unmatched transfer out
Pre-2025 method (attested historical fact): HIFO (attested: false)
Standing orders (MethodElection): 0
Lot selections recorded: 0
Per-disposal compliance (post-2025): 0
[exit 1]
```
