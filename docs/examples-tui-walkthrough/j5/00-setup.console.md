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
  in 200000000 = disposed 100000000 + removed 0 + held 100000000 + fee-sats 0 + pending 0
2025 transition: Path A (actual per-wallet reconstruction; default, no election)
Pending reconciliation: 0 transfer(s); unknown-basis inbounds: 0
Hard blockers (gate tax computation): 0
Advisory blockers: 1
  [IdentificationDefaulted] import|coinbase|trade|opt-sell :: no specific identification was made for this disposition — no LotSelection covers it and no MethodElection is in force for this wallet — so btctax applied its DEFAULT method HIFO, consuming $80000.00 of lot basis. §1.1012-1(j)(1) / (j)(3)(i) treat units with no adequate identification as sold in order of acquisition (earliest first), which on this pool draws $30000.00: the default takes $50000.00 MORE basis than the deemed order, so this return reports $50000.00 LESS GAIN than §1.1012-1(j) gives. A standing order IS an adequate identification (§1.1012-1(j)(3)(ii)): record one with `btctax config --set-forward-method <fifo|lifo|hifo>` so later sales rest on your identification rather than this default. It cannot be back-dated, so for THIS disposition only a contemporaneous identification already in your own books supports HIFO
Pre-2025 method (attested historical fact): HIFO (attested: false)
Standing orders (MethodElection): 0
Lot selections recorded: 0
Per-disposal compliance (post-2025): 1
  import|coinbase|trade|opt-sell @ 2025-06-01 :: non_compliant
Promote-basis drift advisories: 0
```
```console
$ btctax --vault v.pgp tax-profile --year 2025 --filing-status single --ordinary-taxable-income 100000 --magi-excluding-crypto 100000 --qualified-dividends 0
Tax profile for 2025 saved.
```
```console
$ btctax --vault v.pgp config --set-forward-method fifo --effective-from 2025-01-01
Recorded standing order (MethodElection) decision|1
fee_treatment: non-taxable, basis carries (TP8 c)
pre2025_method: HIFO (attested: false)
forward_method: FIFO (vault-wide, in force as of 2025-01-01)
  (1 standing order(s) recorded — `btctax verify` lists them with effective dates + status)
```
