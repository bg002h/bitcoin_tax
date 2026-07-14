# IMPL-P1 — Fable independent code review r1 (full-return Phase 1)

**Scope:** Phase 1 diff `e6efb0f..HEAD` (commits 376594b, 25c7fa2, 0eceb13, 30437ab, 1950d59, 9c44d88) —
`btctax-core/src/tax/{return_inputs,return_refuse}.rs`, `btctax-cli/src/{return_inputs,resolve}.rs`,
`cmd/tax.rs`, `cli.rs`, `main.rs`, `lib.rs`, `tax/mod.rs`, `Cargo.toml`.
**Implements:** `SPEC_full_return.md` §4 (+§4.10/§4.12), `IMPLEMENTATION_PLAN_full_return.md` Phase 1 tasks 1–5.
**Reviewer:** Fable (independent — author was a different model).
**Verdict: NOT GREEN — 1 Critical / 6 Important / 6 Minor.**

---

## Verification actually run

- `cargo test -p btctax-core -p btctax-adapters -p btctax-cli --lib` → **294 pass** (58 + 111 + 125), 0 fail.
  `tax::frozen_guard::tests::frozen_engine_files_are_unchanged` **green**; `git diff e6efb0f..HEAD` touches
  **no** frozen file (`types.rs` / `compute.rs` / `se.rs` unchanged).
- `cargo clippy --workspace --lib --bins` → **0 warnings/errors**.
- `cargo clippy --workspace --all-targets` → **FAILS: 46 × E0061** across 8 integration-test binaries (→ C1).
- `cargo test -p btctax-cli --test pseudo_reconcile_cli` (in isolation) → 17/17 pass. But
  `--test tax_report` (which holds `pseudo_mode_injects_placeholder_profile_clearing_tax_profile_missing`,
  the behavior-preservation test for the resolver rewire) **does not compile** — so the rewire's key
  regression test cannot currently run.

---

## CRITICAL

### C1 — The workspace validation suite does not compile: `set_profile` signature change not propagated to integration tests

`cmd::tax::set_profile` gained a 5th `force: bool` parameter (`crates/btctax-cli/src/cmd/tax.rs:19-25`), but
**46 call sites in 8 integration-test binaries still pass 4 arguments** (E0061):
`crates/btctax-cli/tests/{tax_profile,tax_report,optimize_run,optimize_consult,optimize_accept,whatif_sell,whatif_harvest,export_irs_pdf}.rs`.

`cargo test -p btctax-cli` (all targets) and `cargo clippy --workspace --all-targets` fail to build. The
phase gate is "green = the full validation suite passes"; it does not. Consequences beyond the gate itself:

- The **pseudo-reconcile placeholder-injection test** (`tests/tax_report.rs:90-112`) — the exact test that
  proves the `resolve_profile` rewire preserved stored/pseudo/missing behavior — cannot run.
- The **D-4 guard has zero executable tests anywhere** (lib tests cover only TOML parsing; the vault-level
  test file that would exercise `set_profile` is one of the broken binaries). Task 3 is nominally TDD;
  there is no red→green test for the guard or for `--force`.

**Fix:** update all 46 call sites (`, false` — or `true` where a test intends override), then **add** the
missing D-4 tests: (a) `income import` → `set_profile` without `--force` ⇒ `CliError::Usage`; (b) with
`force = true` ⇒ stores; (c) re-run `pseudo_reconcile_cli` + `tax_report` + full workspace suite.

## IMPORTANT

### I1 — §402(g) excess-deferral screen sums household-wide, not per person → false refusal of ordinary MFJ dual-earner households

`crates/btctax-core/src/tax/return_refuse.rs:93-131`: `deferral_sum` accumulates D/E/F/G/S box-12 amounts
across **all** W-2s regardless of `w2.owner`. §402(g)(1) limits *an individual's* elective deferrals; on a
joint return each spouse independently gets $23,000 (TY2024). An MFJ household where the taxpayer defers
$15,000 and the spouse defers $15,000 (both legal, both inert) is refused — as is *every* dual-earner
household whose combined 401(k) deferrals exceed $23k, an extremely common case. The spec's "Σ … across
employers" (F3, §4.10) is the per-person cross-*employer* sum, not a cross-*spouse* sum.
**Fix:** accumulate per `w2.owner` (two accumulators or a small map); refuse iff **any** person's sum
exceeds `p.elective_deferral_limit`. Add an MFJ two-owner KAT ($15k + $15k ⇒ **no** refusal; $15k + $10k
same owner ⇒ refusal).

### I2 — §904(j) FTC ceiling doubled for QSS: fail-open beyond the statutory election

`crates/btctax-core/src/tax/return_refuse.rs:73-78`: `ftc_ceiling_for` doubles the ceiling for
`FilingStatus::Mfj | FilingStatus::Qss`. §904(j)(3)(A)(ii) (and the Schedule 3 line-1 instructions) allow
$600 only "in the case of a **joint return**"; a Qualifying-Surviving-Spouse return uses MFJ *rate
schedules* but is not a joint return — its ceiling is $300. A QSS filer with $301–$600 of foreign tax
passes the screen today and would (P4) claim the credit on Sch 3 L1 without Form 1116 — an invalid
election, i.e. a **wrong return**, the one direction the fail-closed posture forbids. Spec §4.7a says
"$300 ($600 MFJ)" — MFJ only.
**Fix:** double for `Mfj` only; add a QSS-$301 ⇒ refuse KAT.

### I3 — After `income import`, `report --tax-year` is a dead end and the error message recommends a fix that cannot work

`crates/btctax-cli/src/cmd/tax.rs:124-131`: the pending-derivation refusal says "use `tax-profile set` for
a raw profile in the meantime." But `resolve_profile` (`resolve.rs:69-75`) checks `return_inputs::exists`
**first, unconditionally** — a stored raw profile is never reached while `ReturnInputs` exist. Following
the instruction, the user hits the D-4 guard, re-runs with `--force`, stores the profile … and `report`
still errors with the same message. There is **no `income clear`/`delete` subcommand**, and the vault is
encrypted, so the year's report is unrecoverable from the CLI until Phase 2 ships. Surfacing the pending
state (instead of silently treating it as missing) is correct and implemented; the recovery story is not.
**Fix (either):** (a) while derivation is pending, fall back to the stored profile **with a loud printed
warning** ("full-return inputs present; using raw profile until derivation ships") — the P1-only interim;
or (b) keep the hard refusal but correct the message and ship an `income clear --year` subcommand. Test
the chosen path.

### I4 — `ScheduleCInputs` is missing the spec-normative `accounting_method` field

`crates/btctax-core/src/tax/return_inputs.rs:178-201` vs SPEC §4.4a:
`ScheduleCInputs { owner, business_description, naics_code (default "999999"), accounting_method: Method
(default Cash, line F), expenses }`. The implemented struct omits `accounting_method` entirely. The §4
data model is Phase 1's core deliverable; Schedule C line F (P6 filler) needs it, and an accrual-method
filer is currently unrepresentable (silently, not fail-closed). Adding it later is serde-compatible, but
its omission is undocumented (no FOLLOWUPS entry).
**Fix:** add `AccountingMethod { #[default] Cash, Accrual }` (snake_case serde) with
`#[serde(default)]`, mirrored in the manual `Default`.

### I5 — `income show` renders SSNs and full PII unmasked; the P1 SSN-masking acceptance item shipped a render path without the masking

`crates/btctax-cli/src/cmd/tax.rs:69-83` (`show_return_inputs`) pretty-prints the entire `ReturnInputs`
JSON — including `header.taxpayer.ssn`, spouse and dependent SSNs, and `ip_pin` — to stdout in cleartext
(scrollback, pipes, logs). SPEC §4.2: "SSNs normalized/masked" (security-review item); plan P1 task 5:
"SSN `--stdin` entry + masked rendering", acceptance "SSN masking verified." Deferring `--stdin` entry is
tolerable (TOML entry exists); deferring *masked rendering* is not, because the render path itself ships
in this phase.
**Fix:** redact `ssn`/`ip_pin` fields at render time in `show_return_inputs` (e.g. `***-**-1234`), with a
test. Store normalized as specced.

### I6 — Multiple plan-task deferrals are undocumented: no FOLLOWUPS entries recorded

Phase 1 deviates from plan tasks 2/3/5 in several places, none recorded in
`design/full-return/FOLLOWUPS.md` (the workflow requires deviations be recorded, as was done for
`p0-taxtable-deviation`):

1. **Task 2** per-field subcommands (`income add-w2/add-1099-int/-div/-g`, `schedule-c set`,
   `deductions set`, `dependents add`, `household set`, `payments set`) — only `import`/`show` shipped
   (noted only in a cli.rs doc comment).
2. **Task 2** `income show --toml` — shipped as JSON (doc-comment justification only).
3. **Task 3** "wire the single resolver into report/TUI/optimize/what-if/export" — only `report` is wired.
   Still reading `tax_profile` directly: `cmd/optimize.rs:43,110,178`, `cmd/whatif.rs:81,139`,
   `session.rs:548` (`optimize_proposal`, the TUI path), `cmd/admin.rs:90,246`. **Provenance is printed on
   no output** (SPEC §4.12 "printed on every output"; `report_tax_year` discards `resolved.provenance`).
4. **Task 5** carryover write-back plumbing + computed-vs-user provenance (`CharitableCarryItem` has no
   provenance field) — absent.
5. **Task 4** row reassignment: plan lists business-Interest and ≥2-SE-earners as P1 input-screenable; the
   impl (correctly) reclassifies them ledger-dependent → P2/3, but only in a module doc-comment.

**Fix:** add a FOLLOWUPS entry per deferral with its owning phase (and fold I3/I5 outcomes into them), or
implement. See the deferral rulings below for which are substantively acceptable.

## MINOR

- **M1** `crates/btctax-cli/src/cli.rs:332-336` — `IncomeCmd` was inserted **between** the `WhatIf` enum's
  doc comment and `pub enum WhatIf`: the what-if tree's doc now heads `IncomeCmd`'s rustdoc and `WhatIf`
  has none. Move the new block below `WhatIf` (or above the what-if doc).
- **M2** `return_refuse.rs` module doc enumerates the deferred §4.10 rows but omits **excess-APTC/Form
  8962** entirely. Per the recorded `fr-8962-taxonomy` FOLLOWUP it is "unrepresentable — no input exists";
  cite that there so the one-KAT-per-row audit trail stays closed (and P5's LIMITATIONS list (iii) picks it
  up). Not a code bug — no input exists to screen.
- **M3** `return_inputs.rs:129,144` — `Person`/`Dependent` `date_of_birth: Option<Date>` vs spec §4.2
  `Date`. Acceptable capture-side relaxation **iff** P3's §63(f)/dependent-floor computation fail-louds on
  `None` (a silent "not 65" would overstate tax without the §3.4 advisory). Pin that expectation now.
- **M4** No non-negative validation on imported money amounts anywhere in `import_return_inputs`/
  `screen_inputs`; a negative box-12 `amount` can also *offset* the §402(g) sum. Add a ≥0 screen at import
  or in `screen_inputs`.
- **M5** `crates/btctax-cli/src/return_inputs.rs:57-59` — `exists()` = `get()?.is_some()` deserializes the
  whole blob; a `SELECT 1` suffices. (Behavioral note: a corrupt blob makes the D-4 guard *error* rather
  than pass — fail-closed, acceptable.)
- **M6** No vault-level integration test for `income import` → `income show` (unit TOML-parse + in-memory
  side-table tests exist). Add one when the C1 fix restores the integration-test build.

---

## Deferral rulings (as requested)

1. **Per-field subcommands deferred, TOML import ships — ACCEPTED for P1.** TOML import is a complete,
   tested capture path for every §4 field; the subcommands are UX sugar. Condition: FOLLOWUPS entry (I6.1).
2. **Consumer sweep (only `report` wired) — ACCEPTED for P1, must complete in P2.** While derivation is
   pending, `ReturnInputs` can produce **no number**, so the two-liabilities G4 divergence cannot yet occur;
   the D-4 guard fences new raw profiles and `report` surfaces pending. Residual hole: a pre-import stored
   profile keeps silently feeding optimize/what-if/TUI/export while `report` refuses — bounded, pre-existing
   behavior, but it means the fence is *posture-inconsistent*, and I3 shows the report side is a dead end.
   Conditions: FOLLOWUPS entry pinning the **full** sweep (all consumers + provenance printing) to P2
   alongside plan P2 task 5, and the I3 fix now.
3. **JSON `income show` instead of TOML — ACCEPTED.** The stated justification is real: `toml`'s serializer
   errors (`ValueAfterTable`) on direct struct serialization when scalars follow tables. Note for the
   follow-on that a `toml::Value::try_from(&ri)` round-trip reorders keys and would work — record it.
4. **SSN masking — NOT accepted as a deferral** (→ I5): the render path shipped, so the masking must ship
   with it. `--stdin` entry alone may defer (recorded).
5. **Carryover write-back plumbing — ACCEPTED.** Nothing computes a carryover-out before P3/P4; adding a
   provenance field later is `#[serde(default)]`-back-compatible. Condition: FOLLOWUPS entry (I6.4).

---

## Checked CLEAN

- **§4 data model** (except I4/M3): all §4 structs present — `W2` (owner, boxes 1/2/3/4/5/6/7/8/10/12/13,
  17/19), `Box12Entry`, `Person` (DOB + explicit `blind` + occupation), `Dependent`, `HouseholdHeader`
  (can-be-claimed ×2, presidential ×2, ip_pin), `Form1099Int/Div/G` (all specced boxes incl. INT 9,
  DIV 2b/2c/2d/5/13), 6-class `CharitableClass` with correct semantics (ST-crypto = `OrdinaryProp50`),
  `CharitableGift`/`CharitableCarryItem` (class + vintage), `ScheduleAInputs` (SALT either/or fields),
  enumerated-minimal `Schedule1Inputs` (L1 attest, L21, IRA-claimed, HSA), `Payments`, `QbiInputs`, **no
  `qbi_override`** (audit I3 honored), tri-state `Option<bool>` for `mfs_spouse_itemizes` /
  `foreign_accounts` / `foreign_trust`.
- **Serde discipline:** `#[serde(default)]` on every optional field; manual `Default` for `ReturnInputs`
  (Single + empty) and `ScheduleCInputs` (NAICS 999999) each consistent with their serde defaults; enum
  reprs (`Owner`/`CharitableClass`/`ItemizeElection` snake_case, `FilingStatus` variant-named) exercised by
  the round-trip + TOML tests; `time` `serde-well-known` ⇒ ISO-8601 dates in TOML/JSON; `rust_decimal`
  `serde-str` ⇒ money-as-string (matches the TOML KAT).
- **Refuse-guard rows verified correct:** box-12 **allowlist** exactly `{D,E,F,G,H,S,AA,BB,EE,DD}` with
  trim+uppercase normalization (K/R/T/W/A/B/M/N/Z all refuse; KAT-20 = code K present); box 8/10 `> 0`;
  INT box 9 / DIV box 13; DIV 2b/2c/2d; single-employer excess-SS at **employee OASDI 6.2%** ×
  `ss_wage_base` (not 12.4%; $10,453.20 on the real TY2024 base); HSA / IRA-with-deduction (box 13 alone
  does **not** refuse — I3 honored); `foreign_trust == Some(true)` only (`Some(false)`/`None` pass).
  First-refusal-wins is spec-compatible.
- **Compute/ledger-dependent rows loudly deferred, not dropped:** kiddie (P2), Sch C net<0 (P2),
  TI≤0-with-carryforward (P3), ≥2 SE earners + business-Interest (ledger, P2/3) — module doc states each.
- **Side-table** mirrors `tax_profile.rs`: idempotent DDL, defensive `init_table` in every read (old-vault
  robust — tested), typed `BadConfigValue` on bad JSON (tested), upsert, sorted `all()`.
- **Resolver:** §4.12 precedence order exact; `placeholder_tax_profile` moved verbatim from `cmd/tax.rs`
  (still injected post-projection ⇒ can never clear a Hard gate); `is_derivation_pending` explicit;
  `report_tax_year` surfaces pending rather than treating the year as missing; stored-beats-pseudo and
  ReturnInputs-beats-stored both KAT'd; pseudo lib + `pseudo_reconcile_cli` (17/17, in isolation) green.
- **D-4 guard logic** itself correct (`!force && exists ⇒ Usage error`), wired through `cli.rs`/`main.rs`
  (`--force` flag, default false) — but see C1 for the missing tests.
- **FROZEN:** no frozen file touched; content-pin guard test green.
- **TY2024 params** (P0 carry-in used here): $23,000 §402(g), $2,600 §1(g)(4), $300 §904(j), 6.2% OASDI —
  all correct primary-source figures.

## Required to reach GREEN

Fix C1 (+ its missing D-4/pseudo-regression tests), I1, I2, I3, I4, I5; record I6's FOLLOWUPS entries.
Then re-run: full workspace `cargo test` + `cargo clippy --workspace --all-targets` + re-review.
