# IMPL-P3 Fable review r2 — full-return Phase 3 fold re-review (deductions: §63 std + Schedule A + §170(b) + std-vs-itemized)

- **Reviewer:** Fable (independent adversarial gate; author is a different model, Opus). Re-review **r2** after
  the r1 fold.
- **Scope:** the fold commit `695b1e6` (`git show 695b1e6`) plus the WHOLE P3 diff since P2-GREEN
  (`git diff 4b10e4c..HEAD`, 4b10e4c = P2 r4 CERTIFIED). Primary changed files this fold:
  `crates/btctax-core/src/tax/charitable.rs`, `crates/btctax-core/src/tax/return_1040.rs`,
  `crates/btctax-core/src/tax/return_refuse.rs`, `design/full-return/FOLLOWUPS.md`.
- **Read first:** my r1 review `design/full-return/reviews/IMPL-P3-fable-review-r1.md` (1C/2I/4M — defines what
  had to change); recon `design/full-return/recon/deep/04-schedule-a-and-std-deduction-engine.md` §1/§3;
  SPEC §4.6/§4.7/§6.
- **Verdict:** **GREEN — 0 Critical / 0 Important.** (3 new findings, all Minor/Nit, recorded below — none gate.)

---

## 0. Suite / lint / frozen-guard evidence (run locally at HEAD `695b1e6`)

`cargo test --workspace` (redirected to a file so nothing is buffered away) — **exit 0**;
aggregated across all 81 `test result:` lines: **1,488 passed / 0 failed / 1 ignored / 0 FAILED-or-panicked**.
Representative real result lines from the captured log:

```
test result: ok. 268 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 40.01s
test result: ok. 127 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.93s
test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.50s
test result: ok. 168 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
```

The r1-fold KATs all pass (verbatim from the log):

```
test tax::return_refuse::tests::non50org_cash_gift_refuses ... ok
test tax::return_refuse::tests::non50org_capgain_gift_refuses ... ok
test tax::return_refuse::tests::non50org_carryover_in_refuses ... ok
test tax::return_refuse::tests::dependent_spouse_flag_refuses ... ok
test tax::charitable::tests::negative_agi_clamped_to_zero_ceilings ... ok
test tax::charitable::tests::deep04_worked_example_cent_exact ... ok
test tax::charitable::tests::thirty_percent_class_capped_by_overall_fifty_room ... ok
test tax::charitable::tests::kat13_carryover_ages_across_a_std_year ... ok
test tax::return_1040::tests::derive_matches_deep02_example1_to_the_cent ... ok
test tax::frozen_guard::tests::frozen_engine_files_are_unchanged ... ok
```

`cargo clippy --workspace --all-targets` — **exit 0**, clean:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
```

FROZEN files: `git diff 059ec2a..HEAD -- crates/btctax-core/src/tax/types.rs
crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs` = **0 bytes** (byte-identical), and
`frozen_guard::tests::frozen_engine_files_are_unchanged` passes (above). The DELTA engine is untouched.

**Working tree:** `git status --porcelain` clean; no probe tests left behind (the r1 probes were deleted; I
verified structurally by grep + re-derivation rather than re-adding a probe, so nothing to clean this round).

---

## 1. Per-finding verification of the r1 fold (independent — I did not trust the commit message)

### C1 (was Critical) — non-50%-org charitable classes silently understated tax — **FOLD VERIFIED (fail-closed)**

The author chose fix-path (b): **refuse upstream** rather than model the Pub. 526 special-30%-limit ordering.
I verified each leg independently:

- **(a) The refuse fires for both r1 probe scenarios and for carryover-in.** `screen_inputs`
  (`return_refuse.rs:348-378`) defines `is_non50org` = `{Cash30, OrdinaryProp30, CapGainProp20}` and refuses
  `RefuseReason::NonPublicCharityContribution` when **either** a current `schedule_a.charitable` gift **or** a
  `charitable_carryover_in` item carries a non-50%-org class. KATs `non50org_cash_gift_refuses` (probe 1: AGI
  $100k, $50k Cash60 + $30k Cash30 — the exact input whose old engine allowed $80k vs law's $50k),
  `non50org_capgain_gift_refuses` (probe 2: $30k CGP30 + $20k CGP20 — old $50k vs law $30k), and
  `non50org_carryover_in_refuses` (OrdinaryProp30 arriving purely as carryover, no current gift) each assert the
  right reason **and** the negative control (dropping the non-50%-org item ⇒ `reason == None`). All pass.
- **(b) No path bypasses the screen.** The only production caller of `derive_tax_profile` is
  `btctax-cli/src/resolve.rs:105`, and it is guarded by `screen_inputs` at `resolve.rs:96` which `return`s the
  refusal before derivation (a non-`None` refusal blocks the profile). Every other `derive_tax_profile` /
  `apply_170b` reference in the workspace is a `#[cfg(test)]` call. So in production a non-50%-org class can
  never reach `apply_170b`. Defense-in-depth gap → new finding N1 (Minor) below, non-gating.
- **(c) `apply_170b` no longer over-deducts.** The three non-50%-org allocations and their contributions to
  `allowed_cash`/`allowed_noncash`/`allowed_carryover`/`carryover_out` are **deleted**
  (`charitable.rs:147-158`); only `cash60`/`ord50`/`cgp30` are summed. The 50%-org spine (Worksheet-2 order with
  the R2-I1 total-of-earlier-tiers subtraction) is **unchanged** — I re-derived the deep/04 §3 worked example to
  the cent against the shipping code (§2.1) and it matches $65,000 / carryover $10,000. `deep04_worked_example`
  and `thirty_percent_class_capped_by_overall_fifty_room` both still pass. **Verified.**

### I1 (was Important) — unconsumed `can_be_claimed_as_dependent_spouse` → full MFJ std — **FOLD VERIFIED (fail-closed)**

`screen_inputs` (`return_refuse.rs:371-378`) now refuses `RefuseReason::DependentSpouseUnsupported` when the
flag is set; KAT `dependent_spouse_flag_refuses` asserts the refuse + the negative control. I grepped the whole
workspace for `can_be_claimed_as_dependent_spouse`: the **only** consumers are the definition
(`return_inputs.rs:167`), this refuse guard, and its test — there is **no other silent consumer** that could now
conflict (the §63(c)(5) floor in `standard_deduction` keys solely on the *taxpayer* flag, unchanged). The prior
fail-open (MFJ std overstated up to ~$27,900) is closed. Spec/recon erratum recorded
(`spec-recon-dependent-spouse-checkbox` + `p3-i1-dependent-spouse-refuse`). **Verified.**

### I2 (was Important) — `p2-pref-over-ti-clamp` P3-scheduled item vanished without a record — **FOLD VERIFIED (traceability restored)**

The re-schedule is truthfully recorded in **both** required places: the strip-site comment
(`return_1040.rs:414-422`, now "RE-SCHEDULED to P4 (review I2)" with the channel-rewiring justification) **and**
the FOLLOWUPS entry (`p2-pref-over-ti-clamp` header changed from "SCHEDULED → P3" to "RE-SCHEDULED P3 → P4 at
the P3 review, review I2", with the "Why P4, not P3" paragraph). The underlying number is unchanged and remains
**conservative-only**: reconstructed TI ≥ true TI ⇒ the delta can only OVERSTATE, and the fold explicitly notes
the larger P3 Schedule A deductions make the `TI < qd + cap_gain_distr` region *more* reachable "but never flip
the conservative sign." The P4 landing site (dual-report `absolute_with − absolute_without ≠ delta` KAT) is
named. Trace intact. **Verified.**

### M1 (was Minor) — negative AGI → negative ceilings + inflated carryover — **FOLD VERIFIED (clamp correct + complete)**

- `apply_170b` clamps `let agi = agi.max(Usd::ZERO)` at the top (`charitable.rs:110`) **before** any `pct(...)`,
  so every ceiling is 0 and the whole gift carries forward — I re-derived the −$10k/$5k-Cash60 case (§2.2):
  `allowed = 0`, `carryover_out = [(Cash60, 5000, 2024)]` — the gift, never more. KAT
  `negative_agi_clamped_to_zero_ceilings` pins exactly this. The r1 corruption ($−9,000 allowed, $11,000
  carryover) is gone.
- `schedule_a_deduction` clamps `let agi = agi.max(Usd::ZERO)` (`return_1040.rs:137`) before the 7.5% medical
  floor, so a negative AGI can no longer *inflate* the medical deduction (subtracting a negative). Direction is
  conservative (floor never helps the taxpayer).
- **Completeness check (adversarial):** I traced every AGI-sensitive line. The two AGI-sensitive lines are the
  medical floor and the charitable ceilings — both now clamped. SALT (`salt_5d`/`salt_5e`) and mortgage are
  AGI-independent. The student-loan phase-out uses `agi_before_student_loan` which cannot be driven negative in
  the same way (it is an income sum less early-withdrawal). No other negative-AGI channel remains in P3.
  **Verified complete.**

### M2 / M3 / M4 (record-only) — **RECORDED accurately with correct ownership**

- **M2** (P4 must hoist `apply_170b` out of the `schedule_a.map_or` guard for G8 std-year aging): folded into
  the `p3-carryover-writeback-P4` entry as rider (ii), owning phase **P4**. Accurate — I confirmed
  `return_1040.rs:406` still gates the engine call behind `ri.schedule_a.as_ref().map_or(...)`, so a filer with
  `charitable_carryover_in` but no Schedule A block still skips aging (harmless in P3, carryover_out discarded).
- **M3** (G21 dependent-floor earned income completes with P4's ½-SE): recorded as
  `p3-m3-dependent-floor-earned-income-G21`, owning phase **P4**. Accurate — `standard_deduction(ri, params,
  year, wages)` at `return_1040.rs:405` still passes wages-only (conservative interim).
- **M4** (P5 advisory for None-DOB forfeited §63(f)): recorded as `p3-m4-none-dob-forfeited-63f-advisory`,
  owning phase **P5**. Accurate.

All three land on a *later* phase than P3, consistent with the per-phase ownership rule — none is overdue.

---

## 2. Independent cent-exact re-derivations against the SHIPPING code

### 2.1 deep/04 §3 charitable worked example (50%-org spine unchanged by the fold)

MFJ, AGI $200,000; $5,000 `Cash60` + $70,000 `CapGainProp30`; no carryover-in; year 2024. Traced through the
post-fold `apply_170b`:

| Step | Class | Ceiling | Allowed | Carryover-out |
|---|---|---|---|---|
| 1 | Cash60 | 0.60·200,000 = **120,000.00** | min(5,000, 120,000) = **5,000.00** | — |
| 2 | OrdinaryProp50 | max(0, 100,000 − 5,000) = 95,000.00 | 0 | — |
| 3 | CapGainProp30 | min(0.30·200,000 = **60,000**, max(0, 100,000 − 5,000 − 0) = 95,000) = **60,000.00** | min(70,000, 60,000) = **60,000.00** | **10,000.00** @2024 |

`allowed_cash = 5,000` + `allowed_noncash = 60,000` + `allowed_carryover = 0` = **$65,000.00**;
`carryover_out = [(CapGainProp30, 10,000.00, 2024)]`. Matches recon §3 line 14 ($65,000) and the carryover to
the cent. The non-50%-org deletion did not perturb the spine. **Verified.**

### 2.2 Negative-AGI case

`apply_170b(-10,000, [Cash60 $5,000], [], 2024)`: `agi` clamps to 0 ⇒ every `pct(...) = 0` ⇒ `cash60` ceiling 0
⇒ `current_allowed = 0`, `carry_out = [(Cash60, 5,000, 2024)]`. Result: `allowed = 0`, `carryover_out` = the
$5,000 gift exactly. `schedule_a_deduction` at AGI −$10k clamps to 0 ⇒ `medical = max(0, medical − 0) = medical`
(no negative-floor inflation). Both match the shipping code and the KAT. **Verified.**

---

## 3. New findings introduced/left by the fold (adversarial scan)

None reach Critical or Important. Three Minor/Nit, recorded so the trace is a grep:

### N1 (Minor) — `apply_170b` silently drops non-50%-org gifts if its upstream `screen_inputs` invariant is ever bypassed

The C1 fix moves the safety to `screen_inputs` and makes `apply_170b` **assume** it never sees a non-50%-org
class — but `apply_170b` is `pub` and enforces nothing itself. If P4's absolute-Schedule-A assembly calls
`apply_170b` on a with-crypto path **without** re-running `screen_inputs` (or a future caller forgets the
guard), a `Cash30`/`OrdinaryProp30`/`CapGainProp20` gift is silently summed into **nothing** — `current(...)`
never matches it, and it is absent from `carryover_out`, so the gift *vanishes*. Direction is **conservative**
(a dropped deduction overstates tax, never a wrong refund), which is why this is Minor, not a gate — but it is a
silent drop with no `debug_assert!`. **Recommendation (P4-owned):** add
`debug_assert!(gifts.iter().chain(carryover markers).all(|c| is_50pct_org(c.class)))` at the top of
`apply_170b`, or have `apply_170b` itself return an error on a non-50%-org class, so the invariant is enforced
at the function boundary and cannot regress when P4 adds a second caller. File under `p3-carryover-writeback-P4`
(same entry that already warns P4 about the classes).

### N2 (Nit) — the r1 §3.3 ruling on `p3-crypto-donation-delta-integration` is not reflected in the FOLLOWUP entry text

My r1 review §3.3 *ruled* the open design-Q "derive-side crypto-donation exclusion is CORRECT (conservative),
keep open for P4 with a **medical-floor-fixture** KAT rider." The fold persisted the r1 review verbatim (so the
ruling is traceable in-repo), but the `p3-crypto-donation-delta-integration` FOLLOWUP entry itself is unchanged
and still reads "unresolved … decide at P3 review / P4 start." A P4 reader grepping FOLLOWUPS (not the review
file) will not see that the derive-side direction is settled or that the medical-floor fixture is required.
**Recommendation:** add one line to the entry pointing at r1 §3.3's ruling + the medical-floor-KAT rider.
Non-blocking (the ruling is committed in the r1 review file).

### N3 (Nit) — the `schedule_a_deduction` medical-floor clamp has no direct KAT

`negative_agi_clamped_to_zero_ceilings` pins the clamp inside `apply_170b`, but the *second* M1 clamp
(`return_1040.rs:137`, the medical floor) is exercised by no test — it is correct by inspection and can only be
reached in P3 via a negative non-crypto AGI (harmless there), but a future refactor could delete it silently.
**Recommendation:** add a one-line KAT asserting `schedule_a_deduction` at a negative AGI returns the
un-inflated medical deduction (or fold it into the M1 note so P4 pins it when the medical floor becomes
absolute-return-live). Nit.

---

## 4. Deferral-acceptance re-check (r1 acceptances still hold after the fold)

- **`p3-carryover-writeback-P4`** — still ACCEPTABLE. The fold added exactly the two r1 riders I demanded: (i)
  non-50%-org now refused + negative AGI clamped, so `carryover_out` is trustworthy over the in-scope input
  space, and (ii) P4 must hoist the engine out of the `schedule_a` guard (M2). N1 above tightens rider (i) to a
  `debug_assert`. Nothing persists in P3, so no fail-open.
- **`p3-l16-absolute-P4`** — unchanged, still ACCEPTABLE (L16 is an absolute-return line; the frozen-DELTA path
  P3 ships is complete without it; no code path prints a stubbed L16).
- **`p3-non50org-charitable-special-limit`** — the deferral is now **GUARDED by a fail-loud refuse** and the
  false "conservative" claim is corrected in the entry text; the cross-terms are recorded as the path to
  support the classes later. This is the correct posture.
- **`p2-pref-over-ti-clamp`** — re-scheduled P3→P4 with justification; conservative-only; accepted (I2 above).

---

## 5. Plan-acceptance cross-check (Phase 3) — unchanged from r1 except the C1/I1 rows now fail-closed

| Plan acceptance item | Status |
|---|---|
| Charitable worked example (deep/04 §3) to the cent | **PASS** (§2.1; `deep04_worked_example_cent_exact`) |
| Non-50%-org classes | **REFUSED** (fail-closed, C1 fold) — 3 KATs |
| Dependent-spouse flag | **REFUSED** (fail-closed, I1 fold) — KAT |
| Negative AGI | **CLAMPED** (M1 fold) — KAT |
| KAT-13 std-year carryover | Engine-level PASS; derive-level wiring → M2 (P4) |
| Carryover write-back / L16 | DEFERRED → P4 — accepted §4 |
| deep/02 Ex.1 unchanged cent-exact | **PASS** (`derive_matches_deep02_example1_to_the_cent`) |
| FROZEN guard green | **PASS** (0-byte diff vs `059ec2a` + guard test) |

---

## 6. Verdict

**GREEN — 0 Critical / 0 Important.** All three r1 blockers are folded fail-closed and independently verified:
C1 refuses non-50%-org classes (gift + carryover) with no bypass path and an unperturbed 50%-org spine; I1
refuses the dependent-spouse flag with no conflicting consumer; I2's re-schedule is truthfully recorded in both
required sites and stays conservative-only. M1's clamp is correct and complete in both AGI-sensitive lines, does
not leak into `magi_excluding_crypto` or SALT/mortgage, and M2/M3/M4 are recorded on their correct later phases.
Suite 1,488 pass / 0 fail, clippy clean, frozen engine 0-byte diff. The three new items (N1 Minor, N2/N3 Nit)
are all non-gating and conservative-direction; fold N1 into `p3-carryover-writeback-P4` and N2/N3 opportunistically. **Phase 3 is GREEN and may proceed to P4.**
