# VERIFICATION ledger — the R6 r4 review (`2026-09-06-spec-1099da-R6-review-r4.md`, 12/13 resolved; NEW 0C/2I/4M/4N)

Controller's machine-check at `76d8eb9a`, before the fold.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | I-14 | `screen_broker_reporting` takes `&ReturnInputs`, so arm (2) needs the working return, not a bare `BrokerReporting` | `return_refuse.rs:888-893` | HOLD |
| 2 | I-15 | the viewer Snapshot's answers and `export --csv` read `return_inputs::get` (committed only), so a corrected draft answer would not reach them | `session.rs:598-599`; `admin.rs:209` | HOLD |
| 3 | I-15 | the T9 accessor compiles as a thin wrapper over `load`, `parked` observable from `Loaded::Draft{..}` | reviewer's compile check, accepted (the build proves it) | accepted |
| 4 | M-12..M-15, N-4..N-7 | wording claims | read against the spec | HOLD |

Disposition (per the continuity rule written before r4): fold everything now and END the spec loop —
four rounds, each finding one blocking defect in the controller's own folds; the build (which executes)
is the next gate, with its seam review. The two Importants are one-clause and folded verbatim.
