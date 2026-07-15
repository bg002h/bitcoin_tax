# P9 IMPLEMENTATION REVIEW — steps 6–10 (`4acb850..0f2a553`), independent adversarial pass

*Persisted VERBATIM (STANDARD_WORKFLOW §2 — persist before folding). Reviewer: Fable, independent (not the
author). Round: IMPL r3 (r1/r2 covered steps 1–5). Persisted 2026-07-14 at HEAD `0f2a553`.*

---

**VERDICT: 0 Critical / 0 Important** — with 3 Minors, 2 Nits, and 2 gate notes. Nothing blocking found. Steps 1–5 were re-verified only where steps 6–10 touch them; the fresh surface (6–10) was read line-by-line against the r9 spec.

**Reviewer:** Fable (independent — not the author). **Gate at HEAD:** `make check` → 1760 passed / 1 skipped, green. **FROZEN:** `git diff 4acb850..HEAD -- tax/{types,compute,se}.rs` is empty. **Signatures:** `screen_inputs` and `screen_compute_dependent` unchanged (§4 acceptance).

---

## Findings

### MINOR-1 — `SalesTaxElectionWithoutAmount` carries a second, hand-copied derivation of the income-tax-SALT set, and only one of its three legs is test-pinned

- **Where:** `crates/btctax-core/src/tax/return_refuse.rs:590-598` vs `crates/btctax-core/src/tax/return_1040.rs:113-124` (`salt_line_5a`'s else-branch).
- **What:** The guard's `income_tax_salt` (Σ W-2 box17+box19 + `salt_state_estimated_payments` + `salt_prior_year_balance_paid`) is an inline copy of exactly the set `salt_line_5a` deducts on the income-tax path. I verified the two sets are **identical today** — the guard is complete per §2.2. But it is a second copy of one predicate, the shape this codebase's own doctrine forbids ("the count and the amount must come from one derivation," quoted in §2.7). And the named test (`sales_tax_election_without_amount_refuses`, `return_refuse.rs:1615-1636`) exercises **only the estimated-payments leg** — the mutation "drop the W-2 Σ from `income_tax_salt`" survives the whole suite.
- **Failure scenario:** P8 adds a new income-tax SALT input (or a refactor drops the W-2 leg). `salt_line_5a` deducts it; the guard doesn't count it. A filer with election `Some(true)`, amount $0, and **only W-2 box-17 withholding** — the most common filer shape — then computes with 5a = $0: the silent SALT collapse §2.2's guard exists to prevent, with the suite green.
- **Fix direction:** Factor the income-tax leg into one function both sites call (e.g. `fn income_tax_salt(ri, a) -> Usd` next to `salt_line_5a`), and add a W-2-withholding-only case to the named test so each leg's mutation dies.

### MINOR-2 — `SalesTaxElectionWithoutAmount`'s refusal detail describes the exits but names no command

- **Where:** `crates/btctax-core/src/tax/return_refuse.rs:600-604`.
- **What:** The detail says "enter the sales-tax amount, or turn the election off to deduct income taxes" — correct exits, but neither `btctax income import` (the only way to enter the amount; `answer` cannot capture amounts) nor `btctax income answer` (which can flip the election off — verified the skippable prompt re-asks with the current value as default) is named. Every other P9 refusal follows the named-command discipline: `ScheduleBForeignCountryMissing` names `income import` (text-tested both directions, `return_refuse.rs:1642-1657`); the stale-row refusal names all three commands (text-tested). This is the one new refusal whose exit the filer must guess the command for.
- **Fix direction:** Name both commands in the detail; extend the existing named test to pin them, per the §3.5 text-test pattern.

### MINOR-3 — `SalesTaxElectionNotAsked`'s text tells a standard-deduction filer "your Schedule A used state and local INCOME taxes" — describing a form they did not file

- **Where:** `crates/btctax-core/src/tax/advisories.rs:196-202`.
- **What:** The advisory correctly fires on `None` ∧ `schedule_a.is_some()` regardless of the deduction taken (r5 Nit-3, honored). But its single text opens "your Schedule A used … INCOME taxes" — shown verbatim when the return took the **standard** deduction and no Schedule A printed. This is precisely the r5 M-1 shape the sibling mixed-use advisory was required to branch its text for (and does, `advisories.rs:166-188`, with a named text test). The `deduction_is_itemized` bool is already computed and passed into `advisories()` — the branch costs one `if`. (Lesser variant of the same looseness: a filer with $0 income-tax SALT is told their Schedule A "used" income taxes of $0.)
- **Not Important because:** the spec's own §3.4 defines this advisory without a text branch and went green r9 — so this is a truthfulness improvement inside the implementation's discretion, not a spec violation; and the second sentence ("can even flip a near-standard return into itemizing") is accurate for the standard filer.
- **Fix direction:** Branch the first clause on `deduction_is_itemized`, mirroring `MixedUseMortgageNotAllocated`.

### NIT-1 — The per-question property test implements only the "n" half of §3.5's "answer it (n **and** y)"

`return_refuse.rs:942-968` answers each question `false` and asserts the unanswered reason is gone; the `y` half is not in the loop. I verified all 8 questions do have a `Some(true)`-side named test somewhere (DependentTaxpayer `:891`, DependentSpouse `:1742`, MfsSpouseItemizes `return_1040.rs:2361`, ForeignAccounts `:1659` via country-supplied, ForeignTrust `:1590`, HsaActivity `:1580`, DualStatusAlien `:1605`, Mortgage `return_1040.rs:2229`) — so coverage exists today, but a 9th question's y-half will not be automatically covered the way the n-half is. One `(q.set)(&mut r, true)` + assert-ne inside the loop closes it (four questions then refuse for a *different* reason, which the assert-on-specific-reason already tolerates, per §3.5's own note).

### NIT-2 — The HSA prompt says "In this tax year" where §2.4's mandated text is "In {year}", and the answer session never displays the year

`questions.rs:146`. The registry prompt is a `&'static str` so it cannot interpolate; but `answer_return_inputs` (`cmd/answer.rs:216-303`) could print a one-line year banner. A user running `income answer --year 2024` in 2026 must hold the year in their head across eight questions about "this tax year."

---

## Gate notes (not findings against the reviewed range)

1. **Steps 11–12 are in-flight, uncommitted.** The reviewed range ends at step 10. `FOLLOWUPS.md` (the two owned items, correctly phased: (a) → P8, (b) → release gate) and `crates/btctax-cli/LIMITATIONS.md` (+41 lines covering QOF/8949/G-H-I-J/stale-row/`income show`/answer-can't-capture-strings on inspection of the working diff) exist **only in the working tree**. The phase cannot close green until they are committed and the whole-diff review covers them. The classifier's `TrackedFollowup` citation for `accounting_method` ("filed → P8") is accurate — the item lives in `design/SPEC_input_surface.md:414`.
2. **The class-(B) prompt relocation (step 4 → step 7) landed as the continuity doc directed** — prompts and advisories shipped together in `9fc80c8`; no interim window opened (class B never refuses). Flagged here as the continuity doc instructed.

---

## What I verified (attacked, held)

- **Classifier soundness (step 10), field-by-field.** All 19 `ReturnInputs` fields and all 15 reachable structs (`HouseholdHeader`, `Person`, `Dependent`, `W2`, `Box12Entry`, `Form1099Int/Div/G`, `ScheduleCInputs`, `ScheduleAInputs`, `CharitableGift`, `Schedule1Inputs`, `Payments`, `Carryforward`, `CharitableCarryItem`, `QbiInputs`) destructured with **no `..`** (the only `..` in the file is the test fixture's `..Default::default()`); no classifiable leaf (bool / `Option<bool>` / defaulted enum) bound to `_`; no `_`-prefixed bindings; `#![deny(unused_variables)]` present, and it has real teeth — dropping an `exempt(...)` call leaves its binding unused, a hard error. Every `SerdeRequired` claim checked against the actual serde attributes: `filing_status`, `W2.owner`, `ScheduleCInputs.owner` all genuinely lack `#[serde(default)]`. The reachable set is a strict superset of `first_negative_amount`'s (which now cites the classifier as the closure of its `header: _` waiver — the follow-up folded as promised). Class assignments match §2/§2.8 exactly; the census test pins declarations ⇔ `QuestionId::ALL` exactly-once on a Schedule-A fixture, so a `declaration` mis-wire (the one thing the compiler can't catch) fails a named test. The FROZEN `Carryforward` is read, not modified.
- **Advisory correctness (steps 6–7).** `persons` implements the §3.4 formula verbatim (`taxpayer.blind.is_none()` + `Mfj ∧ (spouse absent ∨ spouse.blind.is_none())`); MFS-with-spouse-Person = 1 and absent-MFJ-spouse = 2 are both test-pinned with real dollar rates; `per_box` married/unmarried split (incl. QSS → married) matches §63(f)(3) and the shipped aged advisory. Blind and SALT fire on `None` only — `Some(false)` silence is test-pinned (no D-8 shape anywhere). The mixed-use text genuinely branches on the deduction **actually taken** (distinct itemized/standard texts, both named-tested; "up to" ceiling in both), and the standard filer's Schedule A never prints (`schedule_a_lines` returns `None` unless `deduction_is_itemized`).
- **Value-refusals (step 8).** All three conditions exact and disjoint from the `None` loop by construction (`None` vs `Some(true)`). `SalesTaxElectionWithoutAmount`'s set is complete today (MINOR-1 is about drift, not a present gap), with the no-SALT-to-lose negative case tested. `ScheduleBForeignCountryMissing` names `income import` and its test asserts `income answer` is **absent**; whitespace-only 7b covered. `DualStatusAlienUnsupported` has its named test and its mutation dies (delete the arm → `:1605` fails) — r5 I-3's ghost is laid.
- **Mixed-use one-derivation (step 6).** `mixed_use_mortgage_forgone` is the single source: 8a-zeroing, `ScheduleAParts.mortgage_mixed_use_box`, `PrintedScheduleA.line8_mixed_use_box`, the forms `check_8_mixed_use` fill, and the advisory all key on it, and the **delta path shares it** (`derive_tax_profile` → `schedule_a_deduction` → `schedule_a_parts`) — no second copy anywhere. The box/advisory are scoped to the live predicate (`Some(false)` ∧ interest > 0); bare `Some(false)` with $0 interest is tested box-unchecked and advisory-silent (r6 Nit-3). **No brick**: the truthful mixed-use filer computes under both `Auto` (standard wins) and `ForceItemize`, named-tested end to end including the printed PDF box (real-AcroForm fill + read-back).
- **serde_ignored (step 9).** The `toml::Value` two-pass honors every `#[serde(default)]` (the sparse-TOML parse test omits most fields and passes); it binds **only** the CLI TOML import — the stored-JSON path (`return_inputs::get`, serde_json) is untouched; the rejection names `hsa_present` and `box13_retirement_plan` per the named test, and the message explains the rename and the deletions. `serde_ignored` is a CLI-crate-only dependency. The deleted `box13` line was also removed from `first_negative_amount`.
- **Untested guards.** Hunted specifically for mutations that would survive: every new guard in steps 6–10 has a named test whose deletion-mutation dies, except the leg-level residue in MINOR-1. The two r1 Importants' fixes were spot-checked still in place (`income_answer_asks_every_live_declaration` is registry-derived; the stale-row remedy chain has all three behavioral tests including full-chain carryover restoration).
- **Print boundary.** `assemble_printed_return` runs `ReturnHeader::build` before any form chain; the surviving `unwrap_or(false)` at `printed.rs` (Sch B 7a/8 projection) is unreachable with `None` on both real paths (report: screen passed inside the resolver; export: build refuses) — as adjudicated in IMPL r1(f).
- **No re-opened owner decisions:** the hard-refuse migration, the re-ask-to-`None` rule, and the class-(B) prompt relocation are all implemented as settled.

The two Minors worth acting on before the phase gate are MINOR-1 and MINOR-2 (both small, test-plus-refactor work in `return_refuse.rs`); MINOR-3 is a one-`if` text branch. None blocks under the §2 severity ladder, and I found no defect that changes a computed number, prints an unaffirmed state, bricks a valid return, or lets an unanswered input reach compute or print.
