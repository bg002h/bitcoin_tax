```console
$ btctax --vault v.pgp init --key-backup key-backup.asc
Initialized vault v.pgp (key backed up to key-backup.asc)
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
  in 0 = disposed 0 + removed 0 + held 0 + fee-sats 0 + pending 0
2025 transition: Path A (actual per-wallet reconstruction; default, no election)
Pending reconciliation: 0 transfer(s); unknown-basis inbounds: 1
Hard blockers (gate tax computation): 1
  [UnknownBasisInbound] import|coinbase|in|cb-recv :: unclassified TransferIn — basis unknown
Advisory blockers: 0
Pre-2025 method (attested historical fact): HIFO (attested: false)
Standing orders (MethodElection): 0
Lot selections recorded: 0
Per-disposal compliance (post-2025): 0
Promote-basis drift advisories: 0
[exit 1]
```
```console
$ btctax --vault v.pgp tax-profile --year 2024 --filing-status single --ordinary-taxable-income 100000 --magi-excluding-crypto 100000 --qualified-dividends 0
Tax profile for 2024 saved.
```
