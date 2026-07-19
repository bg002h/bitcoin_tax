# UX-P4-4 — record-time value validation — independent implementation re-review r3 (Fable, closing gate)

**Scope:** the whole UX-P4-4 diff (`074b90f..HEAD`, HEAD = `9647c7e`), re-reviewed after the r2 fold
(commits `1e0f39c` persist-r2, `9647c7e` fold-r2) against review r2
(`reviews/ux-p4-4-impl-fable-review-r2.md`, 0C/1I) and SPEC §3.3 as amended. Job: verify the one
blocking r2 finding (r2-I1) is closed, sweep the fold for new drift, sanity-pass the previously
verified closures, and rule on green. All load-bearing claims below were checked empirically against
the CURRENT tree — three mutations were run and restored (cp-backup/restore; tree verified clean
against HEAD afterward), and the validation surface was re-run in this tree, not taken from the
author's report.

**Fold verification (stated plainly, so the verdict is calibrated):**

- **r2-I1 (the two `what-if harvest` guard sites unwitnessed) — CLOSED, empirically.** The two new
  rows exist (`crates/btctax-cli/tests/value_guard_wiring.rs:226-239` `what-if harvest … --price=-1`;
  `:240-257` `… --carryforward-in=-1`), both `=`-form, both asserting flag + rule (`">= 0"`). I re-ran
  the exact mutations r2 used to prove the hole, plus the sibling:
  - `main.rs:409` (`--price`) reverted to `parse_usd_arg` → the sweep
    (`record_time_value_guards_are_wired_across_the_dispatch_surface`) REDS on precisely the harvest
    `--price` row. The mutated stderr is `TaxProfileMissing` — i.e. the unguarded binary sails past
    clap, `--target` parse (main.rs:390), `--wallet` parse (:392), and the price parse into the
    command proper. That simultaneously proves reachability: in the unmutated tree the row's refusal
    (`--price must be >= 0`) can only originate at the :409 guard (the sell arm at :330 is not
    dispatched for `what-if harvest`, and the message text exists only in `parse_nonneg_usd_arg`,
    `eventref.rs:87-95`).
  - `main.rs:435` (`--carryforward-in`) reverted → the sweep REDS on precisely the harvest
    carryforward row, this time at the `assert_ne!(code, 0)` (value_guard_wiring.rs:68): the mutated
    binary ACCEPTS the negative carryforward and exits 0 (the ad-hoc branch is genuinely entered —
    `--filing-status single --income 50000` satisfy main.rs:418-431 — and the ad-hoc profile makes
    the year computable). So the row demonstrably reaches :435 and the guard is its only refusal
    source. Attack items 1 and 2 both die here: the rows witness their guards, for the right reason.
  - Unmutated baseline: both tests PASS (2/2).
- **All guarded dispatch sites are now witnessed — enumerated, not assumed.** The complete set of
  record-time guard call sites in `crates/btctax-cli/src/main.rs` (swept the whole crate;
  `parse_nonneg_usd_arg` / `parse_pos_sell_arg` appear nowhere else in `src/`) is **14**: 12 ×
  `parse_nonneg_usd_arg` — :252 `--proceeds`, :330 + :409 `--price`, :358 + :435 `--carryforward-in`,
  :977 `--fmv` (income), :995 `--donor-basis`, :1001 `--fmv-at-gift`, :1013 `--basis`, :1042
  `--amount`, :1045 `--fee`, :1053 `--fmv` (set-fmv) — and 2 × `parse_pos_sell_arg` — :230 + :310
  `--sell`. This matches the SPEC §3.3(a) table exactly (1+3+1+2+3+2+2 = 14 refuse sites,
  SPEC:188-194). The wiring sweep now has **14 rows** (value_guard_wiring.rs:84-257), one per site,
  each on the distinct dispatch arm that owns its site (the flag-name collisions — `--fmv` ×2,
  `--price` ×2, `--carryforward-in` ×2, `--sell` ×2 — are disambiguated by subcommand, so no row can
  be satisfied by a sibling arm's guard); `--basis` (:1013) is double-covered by the `=`-form KAT in
  `tests/classify_inbound_self_transfer_cli.rs:157-180`. Coverage is 14/14. Mutation sensitivity is
  now empirically proven at four sites across r2+r3 (:330 by r2; :409, :435, and the M2 income-drop
  by r3), and holds structurally for the rest (every row's message assert names a flag+rule emitted
  only by the guard helper at that row's arm). The r1-I3 / r2-I1 untested-guard class is closed.
  (The "16 sites" figure in the fold's commit message and FOLLOWUPS is arithmetic drift — Nit N1(r3)
  below; the substance, exactly which two sites were missing and that all are now covered, is
  unaffected.)
- **M2(r2) strengthening — REAL, and proven against the exact mutation it names.** The trio accept
  KAT (`adhoc_negative_income_and_magi_are_accepted`, value_guard_wiring.rs:264-338) now runs the
  same 0.1-BTC LT sale at `--income=-5000` and `--income=400000` (the `sell` closure, :271-288, uses
  the `=`-form via `format!("--income={income}")` — the space form would trip clap's `-`-prefix
  detection) and asserts `out_neg != out_hi` (:306-310). I ran the named mutation — `income: inc` →
  parse-then-default-to-0 in the `what-if sell` ad-hoc construction (main.rs:363) — and the test
  REDS on exactly that assert, with the two dumped outputs **byte-identical**. That one run settles
  all three sub-questions of attack item 3: (i) the mutation genuinely reds; (ii) the output is
  deterministic for identical inputs (explicit `--at 2025-06-01`, no clock/random content — so the
  `assert_ne` cannot pass spuriously and the unmutated pass cannot flake); (iii) the witness is on
  the computation, not an echo — the rendered plan contains bracket/room/marginal-tax figures
  derived from income (0% LTCG room 41846.10 at income 0), not the income value itself. In the
  unmutated tree the test passes (the two plans differ), and the negative-`--magi` accept case is
  unchanged (:313-338).
- **No fold-introduced drift.** The post-r2 delta (`002ee48..HEAD`) touches exactly four files —
  `value_guard_wiring.rs`, `FOLLOWUPS.md`, `CONTINUITY_post_v070.md`, and the persisted r2 review —
  and **zero production source**. The `run` helper's signature change to `(i32, String, String)`
  (value_guard_wiring.rs:50-63) is threaded to every caller: `assert_refused` destructures
  `(code, _stdout, stderr)` (:67), the `sell` closure's return type matches (:271), and the trio
  test binds `(code, out_neg, stderr)` / `(code_hi, out_hi, _)` / `(code, _out, stderr)` — no dead
  binding, no warning (clippy `-D warnings` green). Diff-verified hunk by hunk: no pre-existing row
  or assertion was weakened; the negative-income case moved into the closure with byte-identical
  arguments.
- **Docs folds landed.** N1(r2): `CONTINUITY_post_v070.md:65` now reads `1.170A-1(c)(1)` (the
  correct timing cite). M1(r2)/N2(r2) are filed in `FOLLOWUPS.md:2333-2348` with recorded
  disposition — M1(r2) with an owning phase (post-release UX) and the both-surfaces-symmetric fix
  note; N2(r2) recorded-only, as a Nit warrants.
- **Sanity pass over the r2-verified closures — nothing reopened.** Since the post-r2 delta touches
  no production code, r2's substantive verifications stand by construction; spot-checked anyway:
  the five TUI money validators still route through `parse_nonneg_usd`
  (`btctax-tui-edit/src/edit/form.rs:669`, uses at :711/:738/:743/:779/:931/:937/:1090) and the
  acquired-guard call sites are untouched. Nothing r2 missed surfaced in this pass.
- **Validation surface re-run in this tree:** `make check` green — nextest **1999/1999** (8 skipped)
  + clippy `--all-targets --all-features -D warnings` in parallel, exit 0; `cargo fmt --check`
  clean; `scripts/pii-scan-generic.sh HEAD` clean (the fold adds no TIN-shaped literal). The msrv
  and net-isolation CI jobs were not run locally (per the known make-check caveat); the fold is
  test-only with no new syntax class or dependency and no network use, so nothing in it can
  plausibly move those jobs — CI on push remains the confirming authority.
- **§1 invariant intact (attack item 5).** The r2 fold is two test rows, a test strengthening, a
  pin-cite correction in an internal doc, and two FOLLOWUPS filings — no production behavior change
  of any kind (verified by the delta file list, not just the commit message). Across the whole
  UX-P4-4 diff the invariant holds as established in r1/r2: only no-legitimate-instance values are
  refused, the (d) advisory is stderr-only and non-fatal, no golden changed, and no computed tax
  figure for a correctly-specified return is touched.

---

## Critical

None.

## Important

None. r2-I1 is closed: both `what-if harvest` guard sites (main.rs:409, :435) now have wiring rows
(value_guard_wiring.rs:226-257), and reverting either site to the unguarded parser was empirically
shown to red the sweep — the exact demonstration whose absence blocked r2.

## Minor

None new. (M1(r2) is filed with an owning phase in FOLLOWUPS.md:2339-2347 — correct disposition for
an out-of-contract sibling-path defect; M2(r2) is folded and mutation-proven above.)

## Nit

**N1 (r3). The guarded-site count "16" in the fold's records is wrong — the true count is 14.**
`FOLLOWUPS.md:2335` ("now covers all 16 guarded dispatch sites") and the commit message of `9647c7e`
("14/16") carry forward r2's own arithmetic slip (r2 wrote "13 wiring rows + the `--basis` KAT = 14
of the 16"; the actuals were 12 rows, 12/14 covered, and are now 14 rows, 14/14). The full
enumeration is in the preamble above and matches the SPEC §3.3(a) table. Substance unaffected — r2
named exactly the right two missing sites, and every guarded site is now witnessed. Fix the one word
in FOLLOWUPS.md on next touch; the persisted r2 review stays verbatim (this r3 corrects the record).

## Verdict

**GREEN — 0 Critical / 0 Important / 0 Minor / 1 Nit.**

The single r2 blocker is closed and was verified the hard way: reverting main.rs:409 and :435 each
reds the wiring sweep on exactly the new harvest row (previously both stayed green — r2's proven
hole), and the mutated-run behavior (TaxProfileMissing / exit-0 acceptance) doubles as proof that
both rows genuinely reach their guards rather than failing for a wrong reason. The complete guard
surface — 14 dispatch sites, enumerated against both the source and the SPEC table — is now
one-row-per-site witnessed, closing the untested-guard class that r1-I3 opened and r2-I1 narrowed.
The M2(r2) strengthening does what it claims (the parse-then-drop mutation reds on the new
`assert_ne`, and the byte-identical mutated outputs prove the witness is deterministic and
computation-grounded, not an echo). The fold introduced no drift — zero production code in the
post-r2 delta, every test-helper caller threaded, no assertion weakened — and the validation surface
re-ran green in this tree (1999/1999 + clippy + fmt + pii-scan). The §1 invariant holds across the
whole diff. The one Nit (a miscount in FOLLOWUPS prose) does not gate. **UX-P4-4 passes its closing
gate.**
