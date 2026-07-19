# UX-P4-4 — record-time value validation — independent implementation re-review r2 (Fable)

**Scope:** the whole UX-P4-4 diff (`074b90f..HEAD`, now including the fold commits `f4f0dcd`
(persist r1), `13e1704` (I1+I2 fold), `3a7a3f0` (I3 fold), `002ee48` (M/N folds)), re-reviewed
against `design/usage-examples/SPEC_post_v070_product_cycle.md` §3.3 as amended and review r1
(`reviews/ux-p4-4-impl-fable-review-r1.md`, 0C/3I). Job: verify the three folded Importants are
actually closed, sweep for fold-introduced drift, and rule on green. All claims below were checked
against the CURRENT tree; the two load-bearing mutation claims were run empirically, not inferred.
Validation surface: `make check` green in this tree — 1999/1999 nextest + clippy (up from 1989
pre-fold; the 10 new tests are the fold KATs).

**Fold verification (stated plainly, so the verdict is calibrated):**

- **I1 (negative money on the TUI) — CLOSED.** All five TUI money validators now route every
  user-typed USD field through the new `parse_nonneg_usd`
  (`crates/btctax-tui-edit/src/edit/form.rs:669-675`): income `fmv` (form.rs:711),
  gift `fmv-at-gift` + `donor-basis` (form.rs:736, :740), self-transfer `basis` (form.rs:778),
  reclassify-outflow `amount` + `fee` (form.rs:931, :937), set-fmv `usd-fmv` (form.rs:1090).
  The helper is behavior-identical to the old parse except for the guard: same `Usd::from_str`
  (Decimal) semantics, same `bad USD {s:?}` refusal for unparseable text, callers still pass
  `buf.trim()`, and only `v < Usd::ZERO` is refused — zero basis (the app's conservative
  self-transfer default) stays allowed and is positively KAT'd
  (form.rs `ux_p4_4_self_transfer_negative_basis_refused_zero_allowed`). No legit value is newly
  refused.
  **Bypass sweep (the attack that would reopen I1): none found.** Every production construction of
  the three guarded payload classes was traced: `InboundClass` is built only inside the validators
  (form.rs:713/:755/:791), by the bulk self-transfer path with `basis: None, acquired_at: None`
  (`edit/persist.rs:663-666` — no user money), and by the bulk-income path with the *plan-resolved*
  per-row auto-FMV (`tui-edit/src/main.rs:7727-7732` — dataset price, not user-typed).
  `ReclassifyOutflow` is built only by `validate_reclassify_outflow` (form.rs:961) and the bulk
  path with plan-resolved FMV (persist.rs:846-851). `ManualFmv` is built only by `validate_set_fmv`
  (form.rs:1091) and the set-fmv confirm modal — which is fed *from* `validate_set_fmv`'s output
  (tui-edit/src/main.rs:3012-3030, destructuring the validated payload into the modal), then
  rebuilt verbatim at :2540. The classify-inbound confirm modal likewise rebuilds from the
  validated `m.as_` (tui-edit/src/main.rs:1541-1546), and its `as_` is set only from the three
  validators' `Ok` results (:1771, :1903, :2020). `persist_classify_inbound` has exactly one
  production caller (:1562), behind that modal. The TUI record surface for this payload class is
  validator-gated end-to-end.
- **I2 (acquired-after-receipt on the TUI) — CLOSED, with the correct receipt.** The new
  `check_acquired_not_after_receipt` (form.rs:680-695) is strict (`d > receipt` refused; same-day
  allowed) and applied to BOTH the self-transfer `acquired` (form.rs:789) and the gift
  `donor-acquired` (form.rs:753). The threaded receipt is genuine: `InboundListItem.date` is
  computed as `tax_date(ev.utc_timestamp, ev.original_tz)` on a row filtered to a raw `TransferIn`
  payload (`tui-edit/src/main.rs:3279-3292`) — byte-identical to the CLI guard's receipt
  (`cmd/reconcile.rs:74`), so the two surfaces refuse the same datum on the same calendar basis.
  Both confirm sites thread it (tui-edit/src/main.rs:1889-1893, :2007-2012; compile-enforced by
  the signature change). No bypass: the bulk self-transfer path appends `acquired_at: None`
  (persist.rs:663-666), the Income variant has no acquired field, and the classify-raw Acquire
  form deliberately has NO acquired-at field (form.rs:1818-1821, R0-I1). KATs cover
  refuse-strictly-after + allow-same-day on the self-transfer arm and refuse on the gift arm
  (form.rs `ux_p4_4_self_transfer_acquired_after_receipt_refused_same_day_ok`,
  `ux_p4_4_gift_donor_acquired_after_receipt_refused`).
- **I3 (mandated KATs + wiring mutation-protection) — MOSTLY closed; one proven residue (gates,
  below).** The spec-mandated `--sell=-1` `=`-form KAT now exists at BOTH `--sell` sites with
  flag+rule message asserts (`tests/value_guard_wiring.rs:170-181`: `what-if sell` and
  `optimize consult`). The PLAN Step-1d trio exists: `--carryforward-in=-1` refused
  (value_guard_wiring.rs:207-221) and negative `--income`/`--magi` ACCEPTED end-to-end — exit 0
  against a real 2-BTC pool where the ad-hoc profile is mandatory, so the computation genuinely
  runs (value_guard_wiring.rs:230-283). The table-driven binary KAT covers 14 of the 16 guarded
  dispatch sites (13 wiring rows + the `--basis` `=`-form KAT in
  `tests/classify_inbound_self_transfer_cli.rs:157-180`), and I verified its sensitivity
  empirically: mutating the covered `what-if sell --price` site (main.rs:330) back to
  `parse_usd_arg` REDS `record_time_value_guards_are_wired_across_the_dispatch_surface`. The two
  `what-if harvest` sites are the exception — see I1(r2).
- **Minor/Nit folds — all landed as claimed.** M1: SPEC §3.3(c) amended (SPEC:208-219) to the
  as-built contract — choke point = `donation_details::set` with the r1 correction note, and the
  bare-9 `--appraiser-tin` widening recorded with its anti-hardening rationale; matches the code
  (donation_details.rs:105 `is_bare9` arm; validation inside `set` at :139). M2: the pin-cite is
  now `1.170A-1(c)(1)` in every user-facing location — `cli.rs:543` (→ `--help`),
  `cmd/reconcile.rs:152`, SPEC:229, and the regenerated man page
  (`docs/man/btctax-reconcile-reclassify-outflow.1:28`) — and (c)(1) is the correct timing cite
  ("at the time of the contribution"). M3: `tz_label` now has the exact three-offset unit test
  including the subtle `h==0, m<0` arm (cmd/reconcile.rs `tz_label_renders_utc_and_signed_offsets`:
  UTC, −05:00, +05:45, −00:30). N3: the warn line says "USD proceeds/FMV"
  (cmd/reconcile.rs:160-161). M4/N1/N2/N4 are filed in FOLLOWUPS.md:2312-2331 with recorded
  ownership (ownerless post-release residue — correct per the workflow, none is phase-gating).
- **No fold-introduced drift.** The gift/self-transfer validator signature change is threaded to
  both TUI callers (compile-enforced; no caller mis-wired). The pre-existing form KATs were updated
  by parameter-threading only: `any_receipt()` = 2025-01-01 postdates every acquired-date fixture
  in them (2015-01-02, 2022-04-01), so no pre-existing case changes behavior, and no assertion was
  weakened (diff-verified hunk by hunk). The `tests/tax_report.rs` and
  `tests/pseudo_reconcile_cli.rs` hunks are rustfmt reflow only — assertions byte-identical. No new
  dead code (`parse_nonneg_usd` has 7 field uses across 5 validators;
  `check_acquired_not_after_receipt` has 2). The TUI refusal messages use form-field labels
  (`basis`, `donor-acquired` — no `--` prefix), which also avoids extending the M4
  audience-mismatch to the new guards.
- **§1 invariant intact across the whole diff.** The folds add only refusals of value-classes with
  no legitimate instance (negative money on the table's record flags — §1012/§1016; acquisition
  strictly after receipt — factually impossible, same-day allowed) plus tests and doc/spec text. No
  computed tax figure for a correctly-specified return is touched; the (d) advisory remains
  stderr-only and non-fatal; no golden changed.

---

## Critical

None.

## Important

**I1 (r2). The wiring KAT misses the two `what-if harvest` guarded sites — their sign guards are
still mutation-unprotected, the exact residual class of r1-I3 (empirically proven).**
- The 16 guarded dispatch sites include `what-if harvest --price`
  (`crates/btctax-cli/src/main.rs:409`) and `what-if harvest --carryforward-in` (main.rs:435).
  Neither has a row in `crates/btctax-cli/tests/value_guard_wiring.rs` (its only `--price=-1` and
  `--carryforward-in=-1` rows drive `what-if sell`, :182-193 and :207-221), and
  `tests/whatif_harvest.rs` has no negative-value case.
- Proven, not inferred: with main.rs:409 reverted to `parse_usd_arg`, the FULL `btctax-cli` test
  set (412 tests, including the wiring KAT) stays green; the identical mutation at the covered
  sell site (main.rs:330) reds the wiring row. So the suite is blind to exactly these two
  reversions — the untested-guard pattern r1-I3 named.
- Contract: both sites are in the SPEC §3.3(a) table (the `what-if … --price` row cites both
  what-if sites, SPEC:192; the ad-hoc row cites `:421-427`, SPEC:194), and SPEC:234 mandates
  "Mutation reds each". r1-I3 explicitly listed ":330/**:409**" and ":358/**:435**" among the
  call sites whose reversion must red. The fold's own claim — "reverting ANY one call site reds
  its row" (value_guard_wiring.rs:4-8; same claim in commit `3a7a3f0`) — is false for these two,
  and no scope-narrowing was recorded anywhere.
- Fix: two more rows in the existing table-driven KAT (`what-if harvest <target> --price=-1` and
  `… --carryforward-in=-1`, message asserts naming the flag + rule), mirroring the sell rows.
  Production code is already correct at both sites — this is purely the missing witness.

## Minor

**M1 (r2). The TUI classify-raw forms still accept negative money into record-time payloads —
out of this feature's contract, but the defect class survives there; file it.**
`validate_classify_raw_acquire` (form.rs:1822-1848: `usd_cost`, `fee_usd`) and
`validate_classify_raw_income` (form.rs:1855-1880: `usd_fmv`) parse with bare `Usd::from_str` and
build `EventPayload::Acquire`/`Income` directly (deliberately NOT via `InboundClass`, R0-I1), so a
TUI user can still record a negative-basis Acquire via classify-raw. This is genuinely outside the
UX-P4-4 contract — the SPEC §3.3(a) table enumerates per-flag guards and classify-raw is on
neither surface's table (the CLI counterpart is a raw-JSON escape hatch with no money flags,
main.rs:1057-1060, equally unguarded — the surfaces are symmetric), the code is untouched by this
diff, and r1 did not scope it in. Not a fold defect and not a gate — but it is the same
"negative basis reaches a filed form" class on a sibling record path, and it should be a filed
FOLLOWUPS entry with an owning phase rather than tribal knowledge.

**M2 (r2). The trio accept-side KAT proves accepted-and-computed but does not pin the negative
value's effect.** `adhoc_negative_income_and_magi_are_accepted` (value_guard_wiring.rs:230-283)
asserts exit 0 against a real pool with the ad-hoc profile mandatory — the §1 false-refuse guard
the PLAN wanted is fully served (a future sign-hardening sweep reds it). But nothing asserts the
computed output reflects the negative income (e.g. the 0%-bracket result), so a value-dropping
mutation (`--income` parsed then defaulted to 0) stays green. The PLAN wording "flow into the
marginal computation **unchanged**" is therefore only partially witnessed. One stdout assert on
the marginal figure would close it; record or fix, does not gate.

## Nit

**N1 (r2).** `design/usage-examples/CONTINUITY_post_v070.md:65` still carries the stale
`1.170A-1(c)(2)` pin-cite the M2 fold corrected everywhere else. Internal continuity doc (and
excluded from this diff's scope), not user-facing — fix on next touch.

**N2 (r2).** The receipt threading at the two TUI confirm sites (tui-edit/src/main.rs:1889,
:2007) is compile-forced but not value-witnessed: the 7 new form KATs call the validators
directly, so a handler passing a wrong-but-well-typed date would not red. In practice `item.date`
is the only `TaxDate` in scope at both sites; recorded for completeness only.

## Verdict

**NOT GREEN — 0 Critical / 1 Important (I1 r2) / 2 Minor / 2 Nit.**

The r1 blocking findings are, in substance, closed — and verified, not taken on faith: I1's TUI
sign guards cover all five validators with no bypassing construction path anywhere in the TUI
record surface; I2's guard uses the genuinely-correct receipt (`tax_date(utc, original_tz)`,
identical to the CLI), strict-after/same-day-allowed, on both the self-transfer and gift arms;
I3's mandated `--sell=-1` KATs (both sites), ad-hoc trio, and a table-driven wiring KAT proven
mutation-sensitive at its covered rows all exist. The Minor/Nit folds (SPEC amendment, (c)(1)
pin-cite + man regen, tz_label test, warn wording, FOLLOWUPS filings) all landed, spec and code
now agree, no existing test was weakened, the suite is green (1999/1999 + clippy), and the §1
invariant holds across the whole diff.

Exactly one thing still blocks: the two `what-if harvest` guard sites (main.rs:409 `--price`,
:435 `--carryforward-in`) have no wiring rows, and I demonstrated by direct mutation that
reverting one leaves the entire suite green — a residual of r1-I3 against SPEC:234's "Mutation
reds each", with the fold's universal-coverage claim disproven. The fix is two rows in the
existing KAT table; on their landing (and their own red-under-mutation check), re-review should
be quick and this feature should be green.
