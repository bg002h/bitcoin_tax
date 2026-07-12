# IMPL-P1 — Fable independent code review r3 (full-return Phase 1, post-r2-fold)

**Scope:** fold commit `4fef823` ("fold Fable code review r2 (3I/5M) — P1 r3"), i.e.
`git diff c942c14..4fef823` (8 files: `return_refuse.rs`, `return_inputs.rs`, `cmd/tax.rs` [tests],
`main.rs`, `resolve.rs`, `tests/tax_report.rs`, `FOLLOWUPS.md`, the persisted r2 review) + the tree at HEAD.
Verifies each r2 finding is genuinely closed and hunts for defects introduced by the fold.
**Prior:** `reviews/IMPL-P1-fable-review-r2.md` (0 Critical / 3 Important / 5 Minor), persisted verbatim
(191 lines) in the fold commit.
**Reviewer:** Fable (independent — author was a different model). Date: 2026-07-12.
**Verdict: GREEN — 0 Critical / 0 Important / 1 Minor.**

All three Importants and all five Minors are genuinely closed. The centerpiece — the up-front
`first_negative_amount` screen — was audited field-by-field against the full `ReturnInputs` model and is
**exhaustive: all 51 money paths reachable from `ReturnInputs` are covered** (inventory below). The removed
§402(g) `.max(ZERO)` clamp is safely redundant because the negative screen is the *first* statement of
`screen_inputs`, before any accumulator exists. The new KATs are mutation-strong (each would fail under
both the pre-fold code *and* a clamp-only fix). One new Minor (a future-drift hardening, not a present
hole) is recorded below.

---

## Verification actually run

- `cargo test --workspace` → **exit 0; 82 test targets, 1438 passed, 0 failed, 1 ignored (pre-existing)**.
  Full result-line sweep (`grep -E "^test result:|FAILED|^error"` over the complete output): every
  `test result:` line is `ok. … 0 failed`, zero `FAILED`/`error` lines. 1438 = r2's 1435 + the 3 new tests
  (2 `return_refuse` KATs + 1 `tax_report` vault-level test). Suite tail (last non-boilerplate lines are
  all doc-test `ok` lines):

  ```text
     Doc-tests btctax_store
  running 0 tests
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Doc-tests btctax_tui
  running 0 tests
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Doc-tests btctax_update_prices
  running 0 tests
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```

- `cargo clippy --workspace --all-targets` → `Finished 'dev' profile [unoptimized + debuginfo] target(s)`
  — **0 warnings / 0 errors**. A second pass with `-- -D warnings` also finished clean.
- Targeted re-runs:
  - `cargo test -p btctax-core --lib return_refuse` → **11 passed** (was 9 at r2; the two new KATs
    `negative_amount_refuses_before_any_threshold_offset` and
    `spouse_owned_item_on_non_joint_return_refuses` both run and pass).
  - `cargo test -p btctax-core --lib frozen_guard` →
    `test tax::frozen_guard::tests::frozen_engine_files_are_unchanged ... ok` (1 passed).
  - `cargo test -p btctax-cli --test tax_report report_tax_year_with_return_inputs` →
    `report_tax_year_with_return_inputs_refuses_pending_with_income_clear_hint ... ok` (1 passed).
- **FROZEN:** `git diff e6efb0f..4fef823 -- crates/btctax-core/src/tax/types.rs
  crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs` → **EMPTY (0 diff lines)**;
  content-pin guard test green at HEAD (above).

---

## R2-I1 exhaustiveness audit (the KEY risk)

`first_negative_amount` (`return_refuse.rs:94-166`) was checked line-by-line against the current
`ReturnInputs` model (`return_inputs.rs:328-368` + every child struct). Inventory of `Usd`-typed paths
reachable from `ReturnInputs` (there are no other `Decimal`-typed fields; `Usd = Decimal` is the only
money alias, `conventions.rs:8`):

| Container | money paths | covered at |
|---|---|---|
| `W2` (`Vec`) | 11 fields (box1/2/3/4/5/6/7/17/19/8/10) + `box12[].amount` = **12** | `:97-110` |
| `Form1099Int` (`Vec`) | box1/2/3/4/6/8/9 = **7** | `:113-119` |
| `Form1099Div` (`Vec`) | box1a/1b/2a/2b/2c/2d/4/5/7/12/13 = **11** | `:122-132` |
| `Form1099G` (`Vec`) | box1/box4 = **2** | `:135-136` |
| `ScheduleCInputs` (`Option`) | `expenses` = **1** (only `Usd` field) | `:139` |
| `ScheduleAInputs` (`Option`) | 7 scalars + `charitable[].amount` = **8** | `:142-151` |
| `CharitableCarryItem` (`Vec`) | `amount` = **1** (`origin_year` is `i32` metadata, not money) | `:154` |
| `Schedule1Inputs` | 3 = **3** | `:156-158` |
| `Payments` | 3 = **3** | `:159-161` |
| `QbiInputs` | `reit_ptp_carryforward_in` = **1** | `:162` |
| `Carryforward` | `short` + `long` = **2** | `:163-164` |
| `HouseholdHeader`/`Person`/`Dependent` | **0** money fields | n/a |

**Total: 51 paths, 51 covered — no missed field.** Additional correctness points:

- **Carryforward convention:** the frozen `types.rs:24-25` declares `short`/`long` "(≥ 0)" magnitudes
  (§1212(b) character carryforwards), so refusing negatives here is convention-consistent — no legitimate
  input is rejected. Same for every 1099/W-2 box (form-box magnitudes by definition; Schedule D's
  "enter as a negative" convention is a *presentation* rule that P4 owns, not an input rule).
- **$0 is not rejected:** the predicate is strict (`v < Usd::ZERO`, `:95`); `rust_decimal` compares
  `-0 == 0`, so even a pathological `-0.00` blob neither refuses nor offsets (it is zero).
- **Ordering:** the negative screen is the **first statement** of `screen_inputs` (`:170-178`) — no
  accumulator (`deferral_tp/sp` `:210-211`, `foreign_tax` `:255`) exists before it returns, so removing the
  per-entry `.max(ZERO)` clamp (`:239-244`, comment updated at `:208-209`) opens no path where a negative
  reaches accumulation. `screen_inputs` is the only function containing the accumulators, and it has no
  other entry point (repo grep: no production callers yet — P2/P4 wire it at the boundary per SPEC §4.10,
  covered by `p1-consumer-sweep-P2`; in P1 no code consumes `ReturnInputs` numerically, so there is no
  exposure window).
- **KAT is mutation-strong** (`:465-494`): the box-12 case (`+30000, −10000`) asserts
  `NegativeAmount("W-2 box 12 amount")` — under the pre-fold clamp it would get `ExcessElectiveDeferral`,
  and with no screen at all it would get `None` (net 20000 ≤ 23000); either mutation fails the assert. The
  PoC-A case (`box7=+500, box6=−250`) likewise distinguishes screen (NegativeAmount) / clamp
  (ForeignTaxOverCeiling) / nothing (None). Not vacuous.

---

## r2 findings — closed / not closed

| r2 | Claimed fix | Verified at | Status |
|---|---|---|---|
| **I1** FTC refusal suppressible by negative foreign tax (fail-open) | up-front `first_negative_amount` screen; clamp removed as redundant | `return_refuse.rs:94-166` (exhaustive — audit above), gate first at `:171-178`; KAT `:465-494` reproduces PoC-A and asserts refusal. `RefuseReason::NegativeAmount(String)` `:34-37` | **CLOSED** |
| **I2** spouse-owner split evades §402(g) on non-joint returns (fail-open) | `SpouseOwnerWithoutJointReturn` before accumulation | `return_refuse.rs:188-203` — refuses any `Owner::Spouse` W-2 **or** Schedule C when `filing_status != Mfj`, placed before the deferral loop (`:210+`); `Owner` has exactly two consumers (W2.owner, ScheduleCInputs.owner) — both covered. KAT `:496-524` reproduces PoC-B (Single 15k+15k ⇒ refuse), covers HoH+spouse-Sch-C ⇒ refuse, and asserts the MFJ clone of the same split passes; the pre-existing MFJ dual-earner KAT (`:404-417`) stays green. QSS/MFS refusal is correct-conservative (no spouse wages on either return; year-of-death files MFJ) | **CLOSED** |
| **I3** FOLLOWUPS entry recorded the wrong item; 2 refuse rows unowned | rewritten + sibling entry | `FOLLOWUPS.md:65-72` `p1-se-earners-and-business-interest-rows` (SCHEDULED → **P2, MANDATORY**, same class as `p1-consumer-sweep-P2`) — claims verified against `IMPLEMENTATION_PLAN_full_return.md:94-97` (both rows listed in P1 task 4) and SPEC §4.10 (normative rows "≥2 self-employment/business crypto earners" + "business-flagged crypto Interest income"); `:73-76` keeps the old entry accurately rescoped to *ledger*-row reclassification with a "Distinct from" cross-reference | **CLOSED** |
| **M1** `AccountingMethod` split the Sch-C doc from its struct | moved | `return_inputs.rs:176-183` (enum + own one-line doc above), `:185-188` (`ScheduleCInputs` rustdoc restored directly on the struct) | **CLOSED** |
| **M2** pending-derivation `report` branch untested | vault-level test | `tests/tax_report.rs:533-559` — drives `import` → `report_tax_year` ⇒ `CliError::Usage` asserting both the "full-return" explanation and the literal `income clear --year 2025` hint, then `clear` → re-report ⇒ `NotComputable` (recovery proven, and proves the pending error is *gone* after clear). Branch itself unchanged (`cmd/tax.rs:162-169`). Not vacuous | **CLOSED** |
| **M3** false "SSN stored normalized" doc; masking test gaps | doc fixed + FOLLOWUPS + test extended | `return_inputs.rs:120-122` ("stored AS ENTERED … canonicalization deferred to P6"); `FOLLOWUPS.md:77-80` `p1-ssn-normalization-P6` (owner: P6, where the PDF filler needs one format); `cmd/tax.rs` mask test now asserts spouse (`***-**-4321`) + dependent (`***-**-3333`) redaction and originals-untouched | **CLOSED** |
| **M4** stale `resolve.rs` "no vault can hold ReturnInputs" claim | reworded | `resolve.rs:7-12` — states the subcommands shipped, the stub is fail-closed regardless, and names the `income clear` recovery | **CLOSED** |
| **M5** `income show` `print!` without newline | `println!` | `main.rs:217` | **CLOSED** |

---

## Arithmetic re-derived (r3 pass)

- **§904(j) FTC ceiling:** `ftc_ceiling_for` (`return_refuse.rs:83-88`) doubles for `Mfj` **only** —
  §904(j)(3)(A)(ii) doubles "in the case of a joint return"; QSS borrows MFJ rate schedules but is not a
  joint return. TY2024 $300 / $600 MFJ. KAT (`:449-462`): Single $301 > $300 ⇒ refuse; MFJ $301 ≤ $600 ⇒
  pass; QSS $301 ⇒ refuse. All three re-derived correct and passing.
- **§402(g):** $23,000 per *person* (TY2024). Same-owner $15k+$10k = $25k ⇒ refuse; MFJ $15k/$15k
  per-owner ⇒ pass — both KATs green; the buckets are now protected against owner-tag abuse by the I2 gate.
- **Negative screen vs legitimate $0:** strict `<` — a $0 box neither refuses nor offsets (see audit above).
- **Excess-SS MAX in tests:** the 176,100 wage base is the explicitly labeled `SYNTHETIC` test table
  (`tables.rs:304-316`, "Hand-chosen synthetic … real numbers come from BundledTaxTables"); the real
  TY2024 base 168,600 (⇒ MAX $10,453.20, SPEC §4.9) lives in `btctax-adapters/src/tax_tables.rs`.
  Consistent; production unaffected (pre-existing, out of fold scope).

---

## NEW findings

### MINOR

- **R3-M1 — `first_negative_amount` has no compile-time exhaustiveness enforcement (future-drift trap,
  not a present hole).** `return_refuse.rs:94-166` reaches its 51 money paths by hand-written field
  access, and `screen_inputs` now *relies* on that exhaustiveness ("Amounts are already guaranteed ≥ 0 by
  the negative screen above, so no per-entry clamp is needed", `:208-209`). Concrete failure scenario: P2
  adds a `Usd` field to `ReturnInputs` (or a child struct) — e.g. a new payments line or a 1099 box — and
  the workspace compiles clean with the new field silently outside the screen; the R2-I1 offset hole
  reopens for exactly that field with no test to catch it. Suggest making the walk use exhaustive struct
  destructuring (bind every field, **no `..` rest pattern**) so any added field is a compile error at the
  screen, or pin the money-field inventory with a unit test. Today the audit shows zero missed fields, so
  this is hardening, not a defect — Minor, author's discretion (record in FOLLOWUPS if deferred past P2's
  model changes).

*(Unranked nano-nit, pre-existing text outside this fold: `FOLLOWUPS.md:84` `p1-r1-m3-dob-option-pin` says
`Option<NaiveDate>`; the field is `Option<time::Date>` (`return_inputs.rs:130`). Intent is unambiguous.)*

---

## Checked CLEAN (r3 pass)

- **Ordering/masking:** the negative screen preempts every other refusal when both are present (e.g. a
  negative + foreign trust now reports `NegativeAmount`, previously `ForeignTrust` was checked first) —
  both directions refuse, the reported reason is accurate (data integrity first, documented at
  `:171-172`), and no existing KAT depended on the old order (all 11 pass). Same for the spouse-owner gate
  preempting box-12 refusals on the same W-2.
- **No accumulate-before-screen path:** the accumulators live only inside `screen_inputs`, and the screen
  is its first statement; no other entry point exists (grep: zero production callers at P1, by plan).
- **New tests are non-vacuous:** analyzed individually above; each distinguishes the shipped fix from both
  the pre-fold behavior and the weaker clamp-only alternative.
- **Fold blast radius:** `cmd/tax.rs` changes are test-only; `main.rs` is the one-token `println!`;
  `resolve.rs`/`return_inputs.rs` changes are doc/placement-only (the moved `AccountingMethod` is
  byte-identical semantics — serde `snake_case`, `#[default] Cash`, manual `Default` mirror intact);
  the r2 review was persisted verbatim in the fold commit (same practice as r1).
- **FROZEN engine:** byte-identical across the whole phase diff (empty git diff on all 3 pinned files);
  SHA-256 content-pin test green at HEAD.
- **Money discipline:** all fold code is `Usd`/`Decimal`; no floats introduced.
- **FOLLOWUPS integrity:** every entry has an owning phase; the two P2 entries are marked MANDATORY; no
  r2 deferral is missing or mis-scoped.

## Verdict: GREEN

0 Critical / 0 Important / 1 Minor (R3-M1, hardening — non-blocking). Phase 1 passes the whole-diff
review gate. R3-M1 may be folded opportunistically or recorded in FOLLOWUPS with P2 as owner (P2 is the
phase that will next touch the `ReturnInputs` model).
