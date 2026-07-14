# Fable — independent review of P8a (D-8), round 1

*Persisted VERBATIM before folding. Author = Opus (`2b53717` + `aaf3f07`); reviewer = Fable.*

**Verdict: 0 Critical / 3 Important. NOT GREEN.**

---

**Verified directly:** FROZEN files (`types.rs`, `compute.rs`, `se.rs`) unchanged over `05d9085..HEAD`. `make check` green at HEAD: 1729/1729, exit 0, run twice (before and after my mutation experiments; tree restored to pristine). Every mutant the commit claims is caught, I ran myself — results below.

**Verdict: 0 Critical, 3 Important. Not green yet.**

---

## Important

### I1 — The spouse-flag refusal predicate is narrower than the spec, and the gap reaches a printed form. `return_refuse.rs:589`

The spec (`design/SPEC_dependent_flag.md` §3 fix (2)) states the scope as: refuse "**only when the return has a spouse (MFJ, or `header.spouse.is_some()`)**". The implementation dropped the MFJ disjunct:

```rust
if ri.header.spouse.is_some() && ri.header.can_be_claimed_as_dependent_spouse.is_none() {
```

I verified empirically (temporary probe test, since removed): an MFJ return with **no `header.spouse` Person captured** — e.g. `income import` of `filing_status = "Mfj"` with no `[header.spouse]` table, which nothing anywhere refuses — **passes `screen_inputs` with the spouse question unanswered**, and `ReturnHeader::build` (packet.rs:322) then succeeds and sets `claimed_as_dependent_spouse: false`. `export_full_return` (admin.rs:420–484) has no other gate — `first_malformed_ssn` maps an absent spouse to `""` (return_refuse.rs:163), which by policy blocks nothing, and `ReturnHeader::build` only canonicalizes the spouse SSN when the Person exists. The filler (form1040_full.rs:346–351) transcribes unconditionally. Result: a filed 1040-MFJ printing "Someone can claim: your spouse as a dependent" **unchecked, never asked** — the exact D-8 false-statement shape, surviving the fix on the spouse's box. It also silently disarms `DependentSpouseUnsupported` for this population — the guard that exists because a truly claimable spouse limits the joint standard deduction (an understatement channel).

Mitigation on severity: that return is facially incomplete in other visible ways (no spouse name/SSN), the missing-spouse-identity export gap is pre-existing, and there are no users — so Important, not Critical. But this deviation is **undisclosed** (the commit flags two deviations; this is a third) and it contradicts the phase's own acceptance ("no path takes a year down silently, or in the understating direction").

**Fix:** predicate `(ri.filing_status == FilingStatus::Mfj || ri.header.spouse.is_some())` at return_refuse.rs:589, and widen `has_spouse` for `Question::DependentSpouse` in `live_questions` (answer.rs:140–141) identically so the ask-scope tracks the refusal-scope — **but not for `DateOfBirthSpouse`**, because `set_date` (answer.rs:196–200) silently discards a spouse DOB when `header.spouse` is `None`; asking it on a spouse-less MFJ would swallow a typed answer. (The larger alternative — refuse MFJ without spouse identity outright — is real but new scope; the predicate fix is the P8a-owned piece.)

### I2 — Untested guard: the spouse-flag unlaundering. `crates/btctax-cli/src/return_inputs.rs:63`

I deleted `unlaunder(&mut ri.header.can_be_claimed_as_dependent_spouse);` and ran the full gate: **1729/1729 PASS.** The mutant survives because `v0_blob` (return_inputs.rs:249–252) rewrites only the **taxpayer** key — no migration test ever loads a v0 row carrying a spouse `false`. This falsifies the commit's claim as it reads ("Every new guard here is now mutation-checked") — the enumerated 7 mutants skip this line — and it is precisely the vacuous-fixture shape the commit says was already caught once. If this line regressed: every pre-P8a MFJ/MFS row's spouse `false` is ratified as an answered "No", `DependentSpouseStatusUnanswered` never fires for exactly the population that has the bug, and the spouse box prints unchecked unaffirmed. The code is correct today; nothing holds it.

**Fix:** make `v0_blob` rewrite both keys, and add a named test asserting a v0 spouse `false` loads as `None` (and a v0 spouse `true` is preserved).

For contrast, I ran every other claimed mutant and each is killed by a named test:
- whole fixup deleted → 3 tests fail (`a_version_0_rows_unanswered_false_loads_as_none`, `all_migrates_identically_to_get`, `answering_false_on_a_version_0_row_sticks`)
- DO-UPDATE stamp dropped → `answering_false_on_a_version_0_row_sticks` fails
- `standard_deduction` flipped to `== Some(true)` → `the_unanswered_refusal_is_what_guards_the_63c5_floor` fails
- kiddie screen flipped to `== Some(true)` → `kiddie_tax_refuses_dependent_over_threshold` fails
- taxpayer refusal disarmed → 2 tests fail; spouse refusal disarmed → 1 test fails
- INSERT-branch version stamp is killed transitively by the updated import-then-report tests.

### I3 — The print boundary trusts a distant screen, with no local guard and no test. `packet.rs:321–322`

I mutated `== Some(true)` to `!= Some(false)` at the packet projection and ran the **entire** suite: it survives, 1729/1729. So at the one consumer where a wrong value becomes a false statement on a filed PDF, the invariant "`None` never reaches here" is enforced solely by a screen two crates away, pinned by nothing — while the two compute consumers, where the same unreachable `None` could only *overstate*, received both defense-in-depth and mutation-checked tests. That is the asymmetry inverted: depth was added where it was free and omitted where the failure is worst. I did verify the invariant holds today — `assemble_printed_return`'s sole production caller is `export_full_return` (admin.rs:453 runs `screen_inputs` first, refusal writes no bytes), and the report path renders the header-less `assemble_printed_forms` — so this is not currently reachable, which is why it is Important and not Critical.

**Fix:** `ReturnHeader::build` already returns `Result` — make it fail on either flag being `None` (the truly fail-closed print behavior; see direction note below), or at minimum add a named test constructing `None` directly against the build boundary so the projection cannot silently change meaning.

---

## Answers to the two deliberate deviations

**Deviation 1 (`!= Some(false)` at the compute consumers): the reasoning is correct, and I found no case where it is wrong or worse.** For an `Option<bool>`, `== Some(true)` ≡ `unwrap_or(false)` — the spec's rule was indeed unenforceable by any test. `!= Some(false)` ≡ `unwrap_or(true)`: at `standard_deduction` (return_1040.rs:84), `None` takes the §63(c)(5) branch, which is `basic.min(...)` — structurally never larger than basic, so only overstates; at `screen_compute_dependent` (return_1040.rs:626), `None` runs the kiddie screen, which only over-refuses. Both directions are now facts the suite checks — my `== Some(true)` mutants at those two sites die, which is exactly the enforcement the spec's rule could not provide. The deviation is an improvement.

**The packet inconsistency is not a bug, but it is not resolved either.** At print there is no conservative direction: if `None` reached, `== Some(true)` prints the D-8 understating unchecked box; `!= Some(false)` would print a checked box the filer equally never affirmed. Refusal is the only fail-closed print behavior — which is the substance of I3.

**Deviation 2 (`#[serde(default)]` on `HouseholdHeader::taxpayer`): weakens nothing.** `ReturnInputs.header` already carries `#[serde(default)]` (core return_inputs.rs:365), so a TOML omitting `[header]` entirely has *always* produced a default `Person` with an empty SSN. The prior "required" status of `taxpayer` bound only the accidental case of a present-but-partial `[header]` table. Identity was never guaranteed at parse; it is enforced at the packet (`SsnError::Missing`), and `a_full_return_without_an_ssn_refuses_and_writes_no_bytes` still holds that gate.

---

## Everything else I attacked and found sound

- **Migration:** `false ⇒ None` / `true` preserved is right and implemented; `row_to_inputs` is genuinely the only read boundary (`get` and `all` both call it; grep confirms no other deserializer of the table anywhere — the only raw `INSERT` outside the module is a corrupt-JSON test); `set` is the only writer and stamps `SCHEMA_VERSION` on both branches; `ALTER ... DEFAULT 0` backfills existing rows with 0 in SQLite, so old rows migrate; the duplicate-column tolerance is exact (`contains("duplicate column name")` — SQLite does not localize), any other error propagates; open-twice KAT exists and passes.
- **Refusal scope, taxpayer:** unconditional, correctly placed at top level of `screen_inputs` (which ends at line 751 with no earlier success return), fires before compute on every path (`resolve.rs:96`, `admin.rs:453`), and the refusal text names the remedy (`btctax income answer`), asserted by the acceptance test.
- **`income answer` completeness:** `live_questions` asks exactly the refusable set under the *implemented* predicates — `DependentTaxpayer` always; spouse questions on `spouse.is_some()`; Schedule B 7a/8 on `schedule_b_files(ri)`, which is the same predicate `schedule_b_part3_unanswered` (return_1040.rs:1462–1475) gates the refusal on; `MfsSpouseItemizes` on MFS, matching `MfsSpouseItemizeUnknown`. `every_live_question_can_actually_be_answered_and_clears_the_screen` pins the no-brick property. Refuses a year with no row before asking anything (tested). EOF mid-dialog stores nothing. Bare Enter never invents an answer (`parse_yes_no("", None)` → re-ask; tested). No secret is ever prompted.
- **Acceptance journey:** the end-to-end test (tax_report.rs `a_pre_d8_vault_refuses_until_answered_and_income_answer_is_the_way_out`) covers refuse → answer → compute → persistence as `Some(false)`. Note it exercises a *new-code* import that leaves `None`, not a literal v0 blob — the v0 path is covered at unit level, which is acceptable, but is where I2's gap lives.
- **`v0_blob` fixture hygiene:** the `assert_ne!` guard means a future `skip_serializing_if` would fail the fixture loudly rather than vacuously.

## Minor / Nit

- **M1** (`testonly.rs:434–446`): `not_a_dependent` was inserted between `build_golden_household`'s doc block and its `fn`, so the golden-household paragraph ("Build the SAME household in btctax's own input model…") now documents `not_a_dependent`, and `build_golden_household` lost its doc. Move the blank line / reorder.
- **M2** (answer.rs): a spouse-less return whose TOML typed `can_be_claimed_as_dependent_spouse = true` refuses `DependentSpouseUnsupported`, and `answer` will never ask the question that clears it. Not a brick (`income import`/`income clear` recover), and any "yes" re-refuses by design — but the refusal text says nothing about the way out.
- **M3** (answer.rs:301): `answer` stores unscreened; the spec bullet "stores through the screen-before-store gate (D-7)" is unimplementable today because D-7 is explicitly OUT of this phase. Harmless now (writes are strictly improving), but file a follow-up with an owning phase so `answer` adopts the gate when D-7 lands — otherwise the one-pass design's stated rationale silently decays.

---

**Bottom line:** the core fix is real — the flag is a question, the migration reaches the laundered rows without ratifying them, the recovery command exists and clears exactly what the screen refuses, and the two spec deviations the author disclosed are both sound (the first is an improvement the suite can and does enforce). What blocks green: the spouse side of the same defect escapes through the narrowed MFJ predicate (I1), the spouse half of the migration guard is held by no test (I2), and the highest-stakes consumer of the flag has neither guard nor test (I3). All three are small, targeted fixes; none undermines the design.
