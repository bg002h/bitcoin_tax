# IMPL-P1 — Fable independent code review r2 (full-return Phase 1, post-r1-fold)

**Scope:** fold commit `c942c14` ("fold Fable code review r1 (1C/6I/6M) — P1 r2") + the tree at HEAD;
verifies each r1 finding is genuinely closed and scans the fold for regressions/new defects.
**Prior:** `reviews/IMPL-P1-fable-review-r1.md` (1 Critical / 6 Important / 6 Minor), persisted verbatim
(212 lines) in the fold commit.
**Reviewer:** Fable (independent — author was a different model). Date: 2026-07-12.
**Verdict: NOT GREEN — 0 Critical / 3 Important / 5 Minor.**

The fold is high quality: C1 and all six Importants are functionally landed, the new tests are real
(none vacuous), and the full workspace suite is green for the first time this phase. What keeps r2 from
GREEN is (a) two **empirically demonstrated fail-open paths in `screen_inputs`** — one a residue of
r1-M4's general point, one *introduced by the I1 per-owner fix* — and (b) one r1-I6 FOLLOWUPS entry that
records the wrong item, leaving two refuse-guard rows with no owning phase. All three are cheap to fix.

---

## Verification actually run

- `cargo test --workspace` → **exit 0; 81 test targets, 1435 tests passed, 0 failed** (result-line
  sweep: every `test result:` line is `ok`, no `error`/`FAILED` lines). The 8 integration-test binaries
  that did not compile in r1 (46 × E0061) all build and pass — C1's gate failure is gone.
- Targeted re-runs (post-fix proof):
  - `cargo test -p btctax-core --lib frozen_guard` →
    `test tax::frozen_guard::tests::frozen_engine_files_are_unchanged ... ok` (1 passed).
  - `cargo test -p btctax-core --lib return_refuse` → **9 passed** (incl. the new
    `excess_402g_deferral_is_per_person` and the QSS-$301 arm of `foreign_tax_over_ceiling_refuses`).
  - `cargo test -p btctax-cli --test tax_profile` → **7 passed**, incl. the two NEW vault-level tests
    `set_profile_is_refused_while_return_inputs_exist_unless_forced` and
    `income_import_then_show_redacts_pii_at_the_vault_level`.
  - `cargo test -p btctax-cli --test tax_report pseudo` →
    `pseudo_mode_injects_placeholder_profile_clearing_tax_profile_missing ... ok` — the resolver-rewire
    regression test r1 flagged as un-runnable now runs and passes.
- `cargo clippy --workspace --all-targets` → `Finished 'dev' profile` — **0 warnings / 0 errors**
  (r1: failed with 46 × E0061). A `-D warnings` pass was also run to force strictness.
- **FROZEN:** `git diff e6efb0f..c942c14 -- crates/btctax-core/src/tax/types.rs
  crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs` → **EMPTY** (verified; zero
  stat lines). Content-pin guard test green at HEAD (above).
- **Fail-open PoC actually executed** (scratch crate against `btctax-core` at HEAD; drives
  `screen_inputs` with the real bundled TY2024 `TaxTable` + the TY2024 `FullReturnParams` values):

  ```text
  PoC-A single, DIV box7=+500, INT box6=-250 -> screen_inputs = None
  PoC-A control (+500 only)                  -> screen_inputs = Some(ForeignTaxOverCeiling)
  PoC-B Single, 15k D (taxpayer) + 15k D ('spouse') -> screen_inputs = None
  PoC-B control (both taxpayer)                     -> screen_inputs = Some(ExcessElectiveDeferral)
  ```

  Both `None` lines are refusals that a wrong-but-representable input suppresses → R2-I1 / R2-I2 below.

---

## r1 findings — closed / not closed

| r1 | Claimed fix | Verified at | Status |
|---|---|---|---|
| **C1** `set_profile` signature not propagated (suite doesn't compile) | `force: bool` param + 46 call sites + D-4 tests | `cmd/tax.rs:19-35`; all 8 test binaries compile; workspace 1435 pass; D-4 test `tests/tax_profile.rs:131-170` covers import → refuse (`Usage`, nothing stored) → per-year isolation → `--force` stores → `clear` → un-forced allowed | **CLOSED** |
| **I1** §402(g) summed household-wide | per-owner accumulators, clamp ≥0 | `return_refuse.rs:95-135`; MFJ 15k+15k dual-earner KAT passes (`:292-305`), same-owner 15k+10k still refuses | **CLOSED** — but the owner split opens R2-I2 (new) |
| **I2** FTC ceiling doubled for QSS | `Mfj` only | `return_refuse.rs:72-80` — `match status { FilingStatus::Mfj => ×2, _ => ceiling }`; QSS-$301 ⇒ refuse KAT (`:346-349`) passes. Arithmetic: QSS $301 > $300 → refuse ✓; MFJ $301 ≤ $600 → pass ✓ | **CLOSED** |
| **I3** report dead end; message recommends a non-working fix | message → `income clear --year N`; subcommand shipped | `cmd/tax.rs:162-169` (message names the real flag syntax, verified vs `cli.rs:352-356`); `clear_return_inputs` `cmd/tax.rs:98-103`; store-layer `delete` `cli/return_inputs.rs:69-73` + test; vault-level clear→set exercised in the D-4 test | **CLOSED** functionally — the `report` pending-refusal branch itself has no test (→ R2-M2) |
| **I4** `accounting_method` missing | enum + field added | `return_inputs.rs:179-197` — `AccountingMethod { #[default] Cash, Accrual }`, snake_case serde, `#[serde(default)]`, manual `Default` mirrored (`:203-212`) | **CLOSED** — placement regression → R2-M1 |
| **I5** `income show` prints cleartext SSN/IP-PIN | `mask_pii` display copy | `cmd/tax.rs:66-94` (`mask_ssn` → `***-**-NNNN`, digits-only extraction, short input → `***-**-****`; ip_pin → `***`); unit test `:301-313` incl. stored-value-untouched; vault-level test asserts redacted present AND cleartext absent | **CLOSED** — normalization half not done (→ R2-M3) |
| **I6** deferrals unrecorded in FOLLOWUPS | 7 entries added | `FOLLOWUPS.md:48-73`: per-field subcommands ✓, show-as-JSON ✓, consumer-sweep → **P2 MANDATORY incl. provenance printing** ✓ (matches r1 ruling 2's condition), carryover-writeback → P3/P4 ✓, 8962 cross-link ✓, DOB-Option pin ✓ | **PARTIAL — NOT CLOSED** for I6.5 → R2-I3 |
| **M1** `IncomeCmd` between WhatIf doc and enum | moved | `cli.rs:203-212` (WhatIf doc restored on `WhatIf`), `cli.rs:332-357` (`IncomeCmd` has its own doc) | **CLOSED** — but the same mistake class recurs in the I4 fix (R2-M1) |
| **M2** module doc omits 8962 taxonomy | FOLLOWUPS cross-link | `FOLLOWUPS.md:68-70` (`p1-r1-m2-excess-aptc`) records it against `fr-8962-taxonomy` and warns the P3 Sch-2 filler off a live-zero L1a | **CLOSED (by acceptable alternative)** — recorded in FOLLOWUPS rather than the `return_refuse.rs` module doc |
| **M3** DOB `Option` pin | FOLLOWUPS note | `FOLLOWUPS.md:71-73` (`p1-r1-m3-dob-option-pin`): `None` = "not established", fail-loud, P2 doc + KAT | **CLOSED** |
| **M4** negative amounts can offset §402(g) | per-entry `.max(ZERO)` clamp | `return_refuse.rs:127-131` — clamp applied per entry before accumulation; a −$5k code-D can no longer offset | **CLOSED for §402(g) only** — the identical offset in the FTC accumulator was not addressed (→ R2-I1; r1-M4's text asked for a general ≥0 screen) |
| **M5** `exists()` deserializes the blob | `SELECT 1` | `cli/return_inputs.rs:56-66` | **CLOSED** (note: a corrupt blob now passes `exists` and the D-4 guard *refuses* rather than errors — still fail-closed, direction unchanged) |
| **M6** no vault-level import→show test | added | `tests/tax_profile.rs:172-198` — asserts `***-**-6789` present, `123-45-6789` absent, non-PII (`82000`) verbatim; `show` on unset year `None`. Not vacuous | **CLOSED** |

---

## NEW findings

### IMPORTANT

#### R2-I1 — §904(j) FTC refusal is suppressible by a negative foreign-tax amount (fail-open; r1-M4 residue)

`crates/btctax-core/src/tax/return_refuse.rs:143-170`: `foreign_tax += int.box6_foreign_tax` /
`+= div.box7_foreign_tax` with **no ≥0 clamp and no import-time validation** — the exact accumulator-offset
pattern the fold just fixed for §402(g) (M4), forty lines above. r1-M4's text was general ("No non-negative
validation on imported money amounts anywhere … Add a ≥0 screen at import or in `screen_inputs`"); the fold
implemented only the box-12 instance.

**Demonstrated** (PoC-A above): Single filer, one 1099-DIV `box7_foreign_tax = "500"` (over the $300
ceiling — must refuse) plus one 1099-INT `box6_foreign_tax = "-250"` (sign typo / bad import) →
`foreign_tax = 250 ≤ 300` → `screen_inputs = None`. In P4 this return claims the Sch 3 L1 credit without
Form 1116 on $500 of actual foreign tax — an invalid §904(j) election, i.e. a **wrong return**, the one
direction the posture forbids. A negative box-6/box-7 is nonsense on a real form; fail-closed says refuse it.

**Fix (either):** clamp each addend `.max(Usd::ZERO)` (mirrors the M4 fix; PoC-A then refuses at $500), or
— better, and what r1-M4 originally asked — reject any negative money amount at `income import` (one
walk over the deserialized struct; also future-proofs every P2 consumer: a negative `box2_fed_withheld`,
`box1_wages`, etc. currently imports silently and P2 would sum it). KAT the chosen path.

#### R2-I2 — per-owner §402(g) split trusts `w2.owner` with no owner/filing-status consistency: mislabeled Spouse W-2 on a non-joint return evades the refusal (fail-open **introduced by the I1 fix**)

`crates/btctax-core/src/tax/return_refuse.rs:126-135`: deferrals are routed into `deferral_tp` /
`deferral_sp` purely by the `owner` tag. Nothing anywhere (import, screen, or model) validates that
`Owner::Spouse` items are coherent with the filing status — on a **Single/HoH/MFS return there is no
spouse on the return**, yet a spouse-owned W-2 imports and screens cleanly (repo-wide grep: the only
`Owner::Spouse` consumers are these two accumulator lines).

**Demonstrated** (PoC-B above): Single filer, two employers, $15k code-D each, second W-2 mislabeled
`owner = "spouse"` (copy-paste of a `[[w2s]]` block is the realistic vector) → each bucket ≤ $23k →
`screen_inputs = None`, though one real person deferred $30k > $23,000 → unreported 1040-1h taxable excess
→ wrong return. **Pre-fold this could not happen** — the household-wide sum always over-refused, never
under-refused; the per-owner split created the hole. The same inconsistent input will also poison P2
derivation (phantom-spouse wages summed into a Single return's line 1a).

**Fix:** input-screenable consistency row — refuse any `Owner::Spouse`-tagged item (W-2, `schedule_c.owner`)
when `filing_status` is not `Mfj` (an MFS spouse files separately; Single/HoH/QSS-current-year have no
spouse with wages on the return). Optionally also require `header.spouse.is_some()` for spouse-owned items
on MFJ. Two KATs: Single+spouse-W2 ⇒ refuse; the existing MFJ dual-earner KAT stays green.

#### R2-I3 — r1-I6.5 not closed: the FOLLOWUPS entry records the wrong item; the ≥2-SE-earner and business-Interest refuse rows have no owning phase

`design/full-return/FOLLOWUPS.md:65-67` (`p1-task4-row-reclassification`) describes "reclassifying an
imported inbound row (e.g. income ↔ self-transfer) from inside the full-return flow" — that is ledger-row
reclassification (the reconcile system), **not** what r1-I6.5 flagged. I6.5 was: plan P1 task 4
(`IMPLEMENTATION_PLAN_full_return.md:95-97`) lists **business-Interest (R3-I3)** and **≥2 SE earners**
among the P1 input-screenable one-KAT-per-row items; the impl (defensibly) reclassified both as
ledger-dependent → P2/3, documented **only** in the `return_refuse.rs:6-9` module doc.

Verified: no FOLLOWUPS entry mentions either row (grep "SE earner|business.*Interest" → no hits), and no
plan P2/P3 task text lists them either (P2 task 2 owns Sch-C-loss + kiddie + business-income-without-Sch-C,
not these two). Both are normative spec §4.10 refuse rows (SPEC:315-316); if P2/3 doesn't pick them up, a
two-SE-earner household or business-flagged crypto interest **silently computes a wrong return** — and the
deferral audit trail r1 conditioned I6's closure on is broken for exactly these rows, worse than absent
because the mis-named entry *looks* like closure.

**Fix:** rewrite `p1-task4-row-reclassification` (or add a sibling) to record the actual deviation:
"≥2-SE-earners + business-flagged-Interest refuse rows moved input-screenable(P1) → ledger-dependent,
**SCHEDULED → P2 (income assembly), MANDATORY**" — same shape as `p1-consumer-sweep-P2`. If the
income↔self-transfer note is worth keeping, keep it under an accurate name.

### MINOR

- **R2-M1** `crates/btctax-core/src/tax/return_inputs.rs:175-187` — the I4 fix inserted `AccountingMethod`
  **between** `ScheduleCInputs`' three-line rustdoc and the struct: the Sch-C doc (+ the one-line enum doc)
  now heads `AccountingMethod`, and `pub struct ScheduleCInputs` (`:187`) has no rustdoc — the exact
  mistake class r1-M1 flagged (and the fold fixed) in `cli.rs`. Move the enum above the Sch-C doc block.
- **R2-M2** The pending-derivation refusal in `report_tax_year` (`cmd/tax.rs:162-169`) — the actual I3
  dead-end fix — has no test. r1-I3's fix line said "Test the chosen path"; the resolver arm
  (`resolve.rs` unit) and `income clear` (D-4 test) are covered separately, but no test drives
  `report_tax_year` with `ReturnInputs` present and asserts the `Usage` error + the `income clear --year`
  recovery text. One vault-level test closes it.
- **R2-M3** SSN **normalization** is not implemented: `income import` stores the SSN exactly as typed
  (`cmd/tax.rs:48-59` — parse + `set`, no normalization pass), yet `Person`'s doc
  (`return_inputs.rs:120-121`) claims "SSN stored normalized" and SPEC §4.2 says "SSNs normalized/masked".
  Masking (the security-load-bearing half) shipped; the false doc claim should be fixed now and
  normalization either done at import (digits-only or NNN-NN-NNNN canonical form — P6's PDF filler needs
  one) or recorded in FOLLOWUPS with P6 as owner. Nit within the same area: the `mask_pii` unit test
  covers taxpayer-SSN + ip_pin only; spouse/dependent redaction is code-covered but untested.
- **R2-M4** `crates/btctax-cli/src/resolve.rs:10-11` — the module doc still claims "No vault can hold
  `ReturnInputs` until the `income …` subcommands ship" — false at HEAD (they shipped in this phase; the
  D-4 test stores one). Same stale claim the plan review corrected as `pm-r2-m3`. Reword to "the stub is
  fail-closed regardless" when P2 touches this file.
- **R2-M5** `main.rs:217` — `income show` emits the JSON via `print!` with no trailing newline; the
  closing `}` glues to the next shell prompt and `income show | jq` works but a bare terminal read is
  ugly. `println!` it.

---

## Checked CLEAN (r2 pass)

- **Frozen engine:** byte-identical across the whole phase diff (empty git diff on all 3 pinned files);
  SHA-256 content-pin test green at HEAD.
- **Fold blast radius:** the 46 call-site edits are purely mechanical `, false` appends (diff-verified;
  no test logic changed); man pages regenerated (`xtask` docs tests green incl.
  `manpage_covers_every_subcommand` — the four new `income` pages exist); `lib.rs`/`mod.rs` additions are
  module registrations only.
- **New tests are non-vacuous:** the D-4 test asserts the *negative* space (profile NOT stored after
  refusal) and per-year isolation; the PII test asserts cleartext-absent, not just mask-present; the
  §402(g) KAT covers both the refuse and the no-refuse direction; the QSS KAT pins the un-doubled ceiling.
- **I2 arithmetic re-derived:** §904(j)(3)(A)(ii) doubles only for a joint return; TY2024 $300/$600
  matches `ftc_ceiling: 300` + `×2` for `Mfj` alone. §402(g) $23,000 per person, employee OASDI 6.2%,
  kiddie $2,600, std deduction 14,600/29,200/21,900 — all still the correct TY2024 primary-source figures.
- **D-4 guard semantics:** refuse iff `!force && exists` (`cmd/tax.rs:26-32`); corrupt-blob behavior after
  the M5 `SELECT 1` change stays fail-closed (guard refuses instead of erroring).
- **Money discipline:** all new/changed code paths remain `Decimal` (`Usd`) — no floats introduced.
- **`income clear` recovery loop** works end to end at the vault level (import → D-4 refuse → clear →
  raw profile allowed), test-proven.

## Required to reach GREEN

Fix R2-I1 and R2-I2 in `screen_inputs` (clamp-or-refuse negatives; owner/filing-status consistency row)
each with a KAT; rewrite the R2-I3 FOLLOWUPS entry so the two reclassified refuse rows have a recorded
owning phase (P2, mandatory). Minors at the author's discretion. Then re-run the full suite + re-review
the (small) fold.
