# UX-P4-4 — record-time value validation — independent implementation review r1 (Fable)

**Scope:** the whole UX-P4-4 diff (`074b90f..HEAD`, commits `4343543`, `674df3a`, `242a3d7`,
`6f2150e`, `fd40dc9`, `a9e41c6`, `64b49c6`), sub-parts (a)/(b)/(c)/(d), reviewed against
`design/usage-examples/SPEC_post_v070_product_cycle.md` §3.3 (lines 180–228) and the Step-1d plan
(`IMPLEMENTATION_PLAN_post_v070_product_cycle.md:95-109`). All findings verified against the CURRENT
tree, not just the diff hunks. CI surface reported green by the author (nextest 1989, clippy -D,
fmt, msrv, pii-scan, isolation); nothing below disputes that — the blocking findings are contract
gaps, not build breaks.

**What was verified correct (stated plainly, so the verdict is calibrated):**

- **(c) choke-point relocation is REAL and sound.** The spec's cited `set_donation_details`
  (`crates/btctax-cli/src/cmd/reconcile.rs:1259-1291`) does NOT cover the TUI: the TUI persists via
  `persist_donation_details` (`crates/btctax-tui-edit/src/edit/persist.rs:1055-1065`), which calls
  `btctax_cli::donation_details::set` directly (persist.rs:1061), bypassing reconcile.rs entirely.
  I swept every writer to the side-table: the ONLY production `INSERT` is inside
  `donation_details::set` (`crates/btctax-cli/src/donation_details.rs:144-148`), and the only two
  production callers of `set` are reconcile.rs:1287 (CLI) and persist.rs:1061 (TUI). No form
  writer, bulk path, or admin path writes donation details around `set` (bulk_estimated.rs,
  admin.rs, tax.rs, export.rs, forms.rs, packet.rs are all readers). Moving validation into `set`
  (donation_details.rs:139) is therefore a genuine single choke point and a correct improvement on
  the spec's cited location. Both surfaces fail closed: the CLI errors before `session.save()`
  (reconcile.rs:1287-1289), the TUI errors before `save_or_rollback` (persist.rs:1061-1062,
  surfaced non-destructively via `PersistError::NoChange` → `on_persist_error`,
  tui-edit/src/main.rs:635-643). One residual bypass exists by construction — a whole-vault
  snapshot restore copies the table wholesale — but that is a restore of previously-written data,
  not a record path; no action needed.
- **(c) shape helpers are panic-free and correct.** All predicates operate on `as_bytes()` with a
  length check before any index (`donation_details.rs:50-76`); byte-slice indexing has no char-
  boundary panics, and non-ASCII bytes simply fail `all_digits`. The `&e[..2]`/`&e[2..]` *str*
  slices in the normalize arm (donation_details.rs:122) are guarded by `is_bare9` (all-ASCII
  digits ⇒ every index is a char boundary) — no panic path. Masked `***-**-1234` is refused
  (fails `all_digits` in `is_ssn_shape`, wrong length for the other shapes; tested at
  donation_details.rs:276-281 and end-to-end on both surfaces). Lengths are exact (10/11/9/9) — no
  off-by-one.
- **(c) normalization honors the §1 invariant and is idempotent.** `donee_ein` is filed verbatim
  as free text on Form 8283 (`crates/btctax-forms/src/form8283.rs:435-437` → `push_free` →
  `FieldValue::Text`); it is never parsed or computed with, and the hyphenated form is the IRS
  canonical display. Re-running `set` on normalized data hits the `is_ein_shape` keep-as-is arm
  (donation_details.rs:119-120) — idempotent. No existing roundtrip test depends on normalization
  (`full_details()` uses an already-canonical donee EIN and a bare-9 *appraiser* TIN, which is not
  normalized). No golden changed in the diff.
- **(b) the guard core is correct on the CLI path.** Strict `acquired > receipt` with same-day
  allowed (reconcile.rs:73), receipt resolved as `tax_date(ev.utc_timestamp, ev.original_tz)`
  (reconcile.rs:72) — exactly the projection's dating convention (`Eff::date()`,
  `crates/btctax-core/src/project/resolve.rs:117-121`). The guard fires only when the ref resolves
  to a `TransferIn` (reconcile.rs:71), so unknown/wrong-target refs fall through to the existing
  append-then-`DecisionConflict` adjudication (resolve.rs:674-714) unchanged. Refusal is before
  `append_and_save` — fail-closed, and the KATs assert no lot / no decision persisted. `tz_label`
  (reconcile.rs:94-104) is correct by inspection for negative offsets and minutes: `as_hms()`
  returns sign-consistent components, so UTC−05:00 → `h=-5,m=0` → `-`/`05:00`, UTC+05:45 →
  `+05:45`, and the subtle UTC−00:30 case (`h==0, m<0`) is caught by the `m < 0` sign arm;
  `unsigned_abs()` on `i8` is safe for the whole legal offset range. (Test gap noted below.)
- **(d) the warn math and posture are correct.** Threshold `amount > mv * 100` with
  `mv = fmv_of(prices, event_date, sats)` (reconcile.rs:127-135, price.rs:13-18) matches the spec
  formula `FMV > 100 × (sats/1e8) × close-at-the-outflow-date`, uses the event-date close (not a
  "recent" close), and is strict at the boundary (unit-tested both sides,
  reconcile.rs:1362-1379). Dust that rounds to $0 falls to the silent arm; no-price emits the NOTE
  the spec's "state explicitly — silent death of the guard is the failure mode" clause demands.
  Applying it to every outflow kind is sound: for a sell, gross proceeds >100× the event-date
  close is not a legitimate trade; for a gift/donation, FMV *is* the contribution-date market
  value (high *appreciation* is vs basis, not vs close — the spec's repudiation of a basis
  threshold is honored, so the common $0-basis long-held donation cannot false-warn). The advisory
  is stderr-only and non-fatal; `principal` enters the payload unchanged (reconcile.rs:139-145) —
  no persisted figure is touched. Wiring is mutation-proven by the real-binary KAT
  (`tests/reconcile.rs:183-238`).
- **(a) CLI wiring is complete against the spec table.** Every refuse-<0 row is guarded via
  `parse_nonneg_usd_arg` (`eventref.rs:88-97`) at main.rs:977 (`--fmv` income), :995
  (`--donor-basis`), :1001 (`--fmv-at-gift`), :1013 (`--basis`), :1042 (`--amount`), :1045
  (`--fee`), :1053 (set-fmv `--fmv`), :252 (`--proceeds`), :330/:409 (`--price` ×2), :358/:435
  (`--carryforward-in` ×2); both `--sell` sites use `parse_pos_sell_arg` (refuse ≤ 0,
  eventref.rs:100-110) at main.rs:230/:310. The guard-per-flag rule is honored (shared
  `parse_usd_arg` untouched); the `=`-form clap bypass is closed by guarding the parsed value.
  The ad-hoc `--income`/`--magi` staying unguarded matches the PLAN's recorded decision
  (PLAN:97-100 — negative AGI/MAGI is legitimate in NOL years; a blanket refuse would be a §1
  false-refuse) and the tax-profile posture (`--ordinary-taxable-income`/`--magi-excluding-crypto`
  are unguarded, main.rs:855-878). Zero stays allowed for `--basis`.
- **Tax authority checks (attack item 6).** 26 CFR 301.6109-1(a)(1)(i) is cited correctly — the
  reg's principal TIN types are SSN, ITIN, ATIN, EIN — and used soundly (an ATIN is SSN-formatted,
  so the SSN-shape arm covers it). §170(c) is used soundly: every §170(c) donee class is a
  governmental unit or organization, never an individual, so refusing an SSN-shaped `--donee-ein`
  is correct law. The §1.170A-1(c) *doctrine* (charitable FMV = value at the contribution date,
  price-based, not cost basis) is correct and correctly drives the (d) design; the pin-cite is one
  paragraph off (Minor M2 below). No misapplied authority → no Critical under item 6.

---

## Critical

None.

## Important

**I1. (a) Negative money is still accepted on the TUI surface — the spec's headline acceptance
criterion "negative basis refused on BOTH surfaces" is unmet.**
- Contract: SPEC §3.3 acceptance (SPEC:223) — "negative basis refused on **BOTH surfaces** incl.
  the CLI `=` form". "Both surfaces" is not ambiguous: the origin finding this spec line burns
  down says verbatim "*Negative basis accepted on BOTH surfaces — CLI `--basis=-5000.00` … and the
  TUI form (which rejects `abc` as 'bad USD' but not `-5000`) — and flows into gain math: basis
  -5000.00 → gain 26799.23 (> proceeds)*"
  (`design/usage-examples/reviews/tutorial-workaround-audit.md:23`; same text in
  `FOLLOWUPS.md:2206-2215`, owning phase = this cycle). The spec's own severity map names this
  exact class as the Important that gates: "UX-P4-4 (negative basis / bad TIN reach a filed form)"
  (SPEC:31).
- Reality: every TUI money validator still parses with bare `Usd::from_str` and no sign check —
  `validate_classify_inbound_self_transfer` (basis,
  `crates/btctax-tui-edit/src/edit/form.rs:740-758`), `validate_classify_inbound_gift`
  (donor-basis + fmv-at-gift, form.rs:697-728), `validate_classify_inbound_income` (fmv,
  form.rs:670-686), `validate_reclassify_outflow` (amount + fee, form.rs:886-931),
  `validate_set_fmv` (form.rs:1053-1066). A TUI user can still record `basis = -5000`, which rides
  into gain math (gain > proceeds) and onto a filed form — the exact defect the feature exists to
  refuse, on a surface the acceptance criterion names.
- Rationale: half of a phase-owned Important item is undone at its own gate; the (c) sub-part of
  this same diff proves the intended pattern (validate at the shared point both surfaces converge
  on). Fix direction: either guard in the TUI validators (mirroring the per-flag table) or find a
  shared record-time point; either way, TUI-side KATs (`-5000` refused, `0` allowed for basis)
  must red under mutation.

**I2. (b) The acquired-after-receipt guard is bypassable from the TUI edit form.**
- The guard lives only in `cmd::reconcile::classify_inbound` (reconcile.rs:59-86). The TUI
  classify-inbound flow collects `acquired_at` / `donor_acquired_at` in its own form
  (form.rs:740-758 / :697-728) and appends via `persist_classify_inbound`
  (persist.rs:285-300), which calls `append_decision` directly — the (b) guard never runs. A TUI
  user can still record the "factually impossible" (FOLLOWUPS.md:2211-2213) acquired-after-receipt
  date, which then silently corrupts the holding-period math and (per the origin finding) makes
  the lot invisible to earlier what-if sales.
- Contract note, stated honestly: SPEC §3.3(b) (SPEC:200-204) names the CLI flags
  (`--acquired`/`--donor-acquired`), so a literal reading is CLI-only. But the impossibility
  rationale is surface-independent, the sibling (a) criterion is explicitly both-surfaces, and (c)
  in this same diff was deliberately relocated to cover the TUI. Leaving the same impossible datum
  enterable from the adjacent record surface is a missing case (Important), not a scoping choice —
  unless the author obtains explicit spec cover for deferral (per the workflow, a deferral needs a
  mandating-section citation, and none blesses this hole). Fix direction: validate in the TUI
  date-field validators (the receipt date is available on the flow's list item) or at a shared
  record-time point; KAT + mutation red.

**I3. Spec/plan-mandated acceptance KATs are missing; the sign-policy *wiring* is
mutation-unprotected everywhere except `--basis`.**
- SPEC:223-224 mandates: "**`--sell=-1`** refused with a message assert `[R3-nit]` (the `=` form —
  the space form `--sell -1` is clap-rejected pre-fix, so it cannot witness the guard under
  mutation)". No such KAT exists anywhere (swept `crates/btctax-cli/tests/` incl. whatif_sell.rs,
  whatif_harvest.rs, optimize_consult.rs, optimize_run.rs). The only `--sell` test is the unit
  test on the helper (`eventref.rs:565-570`), which has no message assert and — being a direct
  helper call — cannot witness the *wiring*: reverting main.rs:230 or :310 to `parse_sell_arg`
  leaves the whole suite green.
- PLAN:100-102 (the delegated "Decide in the PLAN" decision) mandates: "KAT:
  `--carryforward-in=-1` refused; `--income=-5000` / `--magi=-5000` accepted (and flow into the
  marginal computation unchanged)". None of the three exist. The accept-side KATs are not
  optional garnish — they are the §1 false-refuse guard for the deliberate carve-out (a future
  "harden all the flags" sweep would silently break NOL-year planning with nothing going red).
- Systemically: SPEC:228 requires "Mutation reds each", and the commit message for `674df3a`
  itself concedes only "the `--basis` integration KAT proves the record-time wiring". Reverting
  any *other* call site — `--fmv` (main.rs:977), `--donor-basis` (:995), `--fmv-at-gift` (:1001),
  `--amount` (:1042), `--fee` (:1045), set-fmv `--fmv` (:1053), `--proceeds` (:252), either
  `--price` (:330/:409), either `--carryforward-in` (:358/:435), either `--sell` (:230/:310) — to
  the unguarded parser keeps the suite green. This is the repo's documented recurring failure
  mode (the untested-guard pattern: a correct fix with no test holding it). Fix: one table-driven
  binary-level KAT (flag, `=`-form value, expected message fragment) over the dispatch surface
  would close all of these at once; plus the three ad-hoc trio KATs.

## Minor

**M1. (c) bare-9 `--appraiser-tin` acceptance is an out-of-literal-spec widening — defensible,
but record it.** SPEC:206-207 says appraiser-tin accepts "EIN-shape OR SSN-shape"; the code also
accepts a bare 9-digit (donation_details.rs:105). The reading is sound — 26 CFR
301.6109-1(a)(1)(i) makes the underlying nine digits the TIN regardless of punctuation; refusing
the unformatted form would false-refuse real TINs, contradicting the spec's own `[T2-N2]`
anti-hardening principle (SPEC:210-213); and the pre-existing fixtures already use bare-9
appraiser TINs (`cmd/reconcile.rs:1404` "987654321", `tests/tax_report.rs:1052` "111223333"),
which the literal rule would have broken. It accepts strictly more, refuses nothing the spec
accepts, and touches no computed figure. But it is a deviation from the written contract: amend
the SPEC (c) bullet (one clause) or file the deviation, so spec and code agree.

**M2. Pin-cite precision: the contribution-date timing rule is 26 CFR 1.170A-1(c)(1), not
(c)(2).** The code and user-facing docs cite "(c)(2)" for "FMV at the contribution date"
(`cli.rs:541-544` → `--help`, `docs/man/btctax-reconcile-reclassify-outflow.1:28`,
reconcile.rs:151-152). §1.170A-1(c)(1) carries the timing ("the amount of the contribution is the
fair market value of the property **at the time of the contribution**"); (c)(2) is the
willing-buyer/willing-seller *definition* of FMV. The doctrine as applied is correct (no
misapplication → not Critical), and the SPEC itself carries the same imprecise cite (SPEC:217) —
fix both to "(c)(1)" or "(c)(1)–(2)". User-facing text should not teach a subtly wrong pin-cite.

**M3. `tz_label` non-UTC branches are untested.** Both (b) KATs use UTC fixtures, so the negative-
offset, minutes, and especially the subtle `h==0 && m<0` (UTC−00:30) sign arm
(reconcile.rs:98-102) have no witness; a sign-handling regression would ship silently. Logic
verified correct by inspection — this is purely a test gap. A three-line unit test
(−05:00, +05:45, −00:30) closes it.

**M4. The TUI refusal wording names CLI flags.** A TUI user who typed into the `appraiser_tin`
form field gets "Save error: --appraiser-tin must be …" (choke-point message; the TUI KAT at
persist.rs:3198-3239 even asserts the flag spelling). Recoverable — the `[M1]` flow keeps the
FieldForm open with buffers intact — but the message audience is mismatched. Consider passing a
field-label context to `validate_and_normalize`, or accept the mismatch deliberately (the flag
name at least identifies the field).

## Nit

**N1. "§6695A PTIN" shorthand.** donation_details.rs:84 and the test comment at :315 label the
PTIN with §6695A; §6695A is the appraiser *penalty* section (referenced by the Form 8283
appraiser declaration) — the PTIN's authority is §6109(a)(4)/Reg. §1.6109-2. Pre-existing repo
shorthand (donation.rs:57); harmless in comments, but don't let it migrate into user-facing text.

**N2. `is_ptin_shape` refuses a lowercase `p` (donation_details.rs:75).** Spec-literal (`P\d{8}`),
so in-contract; uppercasing before the check would be friendlier.

**N3. The (d) warn line says "--amount is the USD FMV" for every kind (reconcile.rs:131-133); for
a Sell/Spend it is gross proceeds.** The cli.rs doc-comment draws the distinction correctly; the
warn line could say "USD FMV / proceeds".

**N4. A seconds-only offset (e.g. +00:00:30) renders as "UTC" (reconcile.rs:95-97).** Documented
minute-resolution behavior; pathological input; message-only.

## Verdict

**NOT GREEN — 0 Critical / 3 Important (I1, I2, I3) / 4 Minor / 4 Nit.**

Sub-part (c) is genuinely done: the choke-point relocation is verified real (the spec's cited
location would have missed the TUI; `donation_details::set` provably covers both surfaces, with
no bypassing writer), fail-closed, mutation-proven on both surfaces, and legally sound. Sub-part
(d) is done: correct price-based math, correct no-price NOTE, stderr-only, mutation-proven,
§1-clean. Sub-parts (a) and (b) are correct **on the CLI** but incomplete: the TUI record surface
still accepts negative money (I1 — an explicitly both-surfaces acceptance criterion, per the
origin finding this item burns down) and acquired-after-receipt dates (I2), and the acceptance-KAT
set the spec/plan mandate is missing, leaving all sign-policy wiring except `--basis`
mutation-unprotected (I3). What must change to reach green: TUI-side sign guards + KATs (I1),
TUI-side acquired guard or explicit spec cover for a CLI-only scope (I2), and the missing
`--sell=-1` / ad-hoc-trio / per-flag wiring KATs (I3). None of the blocking findings implicate a
computed tax figure for a correctly-specified return — the §1 invariant is intact throughout the
diff as shipped.
