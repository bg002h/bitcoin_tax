# SPEC review — `design/SPEC_input_form.md` (r2), independent Fable pass r1

*Persisted VERBATIM (STANDARD_WORKFLOW §2 — persist before folding). Reviewer: Fable, independent (fresh —
not the design architect). Persisted 2026-07-14 against HEAD `435affc`.*

---

# Independent spec review — `design/SPEC_input_form.md` (DRAFT r2)

**VERDICT: 1 Critical / 11 Important** (plus 7 Minor, 4 Nit). Verification was done against current source; every code citation in the spec was checked and the results are listed at the end.

---

## CRITICAL

**C-1. §6.2 + §6.3 + §9 — the parked full return inherits WIP-grade destruction policies; the "non-destructive toggle" guarantee (§13) is unmet as written.**
§9 parks the committed row by stashing it into `return_inputs_draft` and deleting the committed row, and the spec itself states the stakes: *"a failed stash must never delete the row, because those SSNs (D-6) exist nowhere else."* But once parked, the sole copy of that committed return lives in a table governed by two WIP-grade rules:

- **§6.2 RULE:** `income import` / `write-carryover` / `income clear` **delete that year's draft** with only a parenthetical *"(warn if discarding a non-trivial draft)"* — a warn, not a confirm. Park 2024 → run `income import --year 2024` (or `income clear`, the intuitive "reset" command) → the parked return, SSNs and all, is destroyed in one non-gated step. The RULE's "one exception" for the toggle cannot save this: the §6.1 DDL (`year, inputs_json, schema_version`) has **no parked flag**, so the coherence-clear physically cannot distinguish a parked return from stale WIP.
- **§6.3:** a stale-`schema_version` draft is *silently DISCARDED*. The committed row refuses on staleness precisely because *"it may hold irreplaceable carryover"* — and the parked blob **is** a former committed row. Park → upgrade the app (SCHEMA_VERSION bump, `return_inputs.rs:24` is already at 2) → toggle-back finds nothing; the return evaporates "with a note."

Fix: give parked blobs committed-row semantics — a `parked` flag column (coherence-clear confirms or refuses on parked rows; stale parked rows refuse-and-reimport like `StaleReturnInputs`), or park into a separate slot that no other writer clears.

---

## IMPORTANT

**I-1. §5.8 — `ip_pin` is missing from the inventory (a coverage hole in the section that claims completeness).**
`HouseholdHeader.ip_pin: Option<String>` (`return_inputs.rs:186`) is in scope per §2 ("header + PII (SSN, **IP PIN**)") and §5.5 ("Secret fields (SSN ×N, IP PIN)"), yet no §5.8 section lists it as a Field and the exemption list does not exempt it. The table "IS the coverage-KAT target"; as written the KAT target itself violates its own rule. Every other leaf of all eight in-scope structs is correctly placed (see confirmations below). Fix: add `ip_pin` **S** to the Taxpayer section.

**I-2. §4/§5.7/§10 — the secret seam cannot carry digits inbound: `set` is unimplementable as typed.**
§4 declares get/set asymmetry ("`set` takes the value"), but the only secret-shaped `FieldValue` is `Secret(SecretView)` and `SecretView::{Empty, Set{masked}}` "NEVER carries digits (§4)". `Edit::SetField { value: FieldValue }` and `set: fn(…, FieldValue)` therefore have **no variant that can transport an SSN or IP PIN to be written**. The asymmetry is asserted in prose but unexpressed in the types. Corollary: the §10 round-trip KAT ("every Field get→FieldValue→set round-trips") is incoherent for Secret fields — `get` returns the mask; setting that back would corrupt the SSN. Fix: an inbound-only value (e.g. `FieldValue::SecretEntry(String)` accepted by `set`/rejected by `get`, with masked `Debug`), and an explicit Secret carve-out in the round-trip KAT.

**I-3. §7 — `PrivateActivityBondAmt` is mis-attributed to `Section(W2s)`; there is no such W-2 box.**
The refusal fires only from `int_1099[i].box9_private_activity_bond_amt` and `div_1099[i].box13_private_activity_amt` (`return_refuse.rs:734-773`) — 1099 forms, both deferred to TOML in v1. The spec's gloss "(box 12 / box 8 / box 10 / **box-9 AMT**)" confuses 1099-INT box 9 with a W-2 box. As written, the form would jump focus to the W-2 section for a defect in a TOML-imported 1099. Fix: move it to the `NotInForm` bucket.

**I-4. §7 — `SingleEmployerExcessSs` is mis-attributed to `NotInForm`; it is screened from an in-form v1 field.**
`screen_inputs` refuses per-W-2 on `w2.box4_ss_withheld > excess_ss_max` (`return_refuse.rs:702-707`). `box4_ss_withheld` is in the §5.8 W2s inventory, so a filer who typos box 4 **in the form** triggers this at commit — and the form would then display the `NotInForm` note ("entered via TOML import / computed at report"), which is false and points away from the exact field they typed. Fix: `Section(W2s)` (box 4).

**I-5. §7 — "attribution is exact via `RefuseReason ↔ QuestionId`" is false: `ScheduleBPart3Unanswered` is shared by two registry entries.**
Both `ForeignAccounts` and `ForeignTrust` carry `unanswered: RefuseReason::ScheduleBPart3Unanswered` (`questions.rs:120,135`). The correspondence is not injective, the `Refusal` payload cannot disambiguate 7a from 8, and §7 itself forbids parsing the prose `detail`. The map's first row ("the corresponding Declaration field") has no unique referent for this variant. Fix: define the rule — anchor both fields (Vec<Anchor> allows it) and focus the first live-unanswered one.

**I-6. §6.2 — the RULE omits `income answer`, one of the five writers the same section just enumerated.**
The hazard list names `income answer` (`answer.rs:309`) as a committed-row writer, but the RULE clears drafts only on "**import, `write-carryover`, `income clear`, and `set-pii`**", and §10's coherence test list repeats the omission. Consequence: form-edit → autosave draft → quit → `income answer --year N` (writes declarations to the committed row, `answer.rs:309` + `s.save()`) → reopen form → `load` prefers the stale draft, hides the answers, and commit clobbers them — the exact silent-loss scenario §6.2 exists to close. Fix: add `income answer` to the RULE and the test list.

**I-7. §9A — "a terminal crash mid-entry loses nothing" is not deliverable by the specified API.**
The vault is decrypted into an **in-memory** SQLite; nothing reaches disk until `Vault::save()` serializes, encrypts, and atomically persists (`vault.rs:231-245`); every existing writer follows `set → s.save()`. `save_draft(conn, year, ri)` (§5.7) takes a `conn` and therefore mutates only the in-memory DB — an autosave built from it alone loses **everything** on a crash, silently (no §10 test covers crash persistence). Fix: the spec must state that autosave = `save_draft` + a vault save, and decide the cost policy (per-`apply` full re-encrypt of the vault vs. debounced/on-section-exit saves).

**I-8. §5.7/§5.8 — no `Bool` field kind exists, but two v1 leaves are `bool`; "Tri→bool" is undefined.**
`presidential_fund_taxpayer`/`_spouse` are `bool` on `HouseholdHeader` (`return_inputs.rs:182-184`), yet `FieldKind`/`FieldValue` offer only `TriState(Option<bool>)`. For a bool leaf: `get` can never return `TriState(None)`, `set(TriState(None))` has no defined meaning (SetError? no-op? coerce-to-false — a mini-laundering?), the §5.4 three-way render contract cannot apply, and the §10 round-trip KAT's semantics are unclear. Fix: add `FieldKind::Bool` (a plain checkbox — correct here, since an unchecked presidential-fund box is the lawful form default) or pin the coercion semantics.

**I-9. §6/§5.7 — `load`'s `RI::default()` silently answers the filing status "Single" — an answered-ness laundering the spec's own house invariant names.**
`filing_status` is serde-REQUIRED, so today's TOML path **forces** an explicit choice (import fails without it). The form regresses this: a fresh year loads `ReturnInputs::default()` = `FilingStatus::Single`, the Enum field renders pre-answered, and a filer who never touches it commits a screen-clean wrong-status return (an MFJ household with taxpayer-only W-2s trips no refusal — `SpouseOwnerWithoutJointReturn` needs a spouse-owned item). `screen_inputs` cannot catch it: the type has no `None` state. Fix: renderer-held "not yet chosen" state for `filing_status` (must be touched before commit), or at minimum the §6.1 payload-confirm must prominently name the filing status (its example — "2 W-2s, Schedule A, 1 dependent…" — currently doesn't).

**I-10. §5.1/§5.8 — `DeleteSection(ScheduleA)` orphans a live-set `ForceItemize` into an invisible $0-deduction return.**
`itemize_election` lives on `ReturnInputs` (ReturnOptions section) with `live: schedule_a.is_some()`. Set `ForceItemize`, then delete Schedule A: the field goes non-live (**hidden** by the renderer per §9A), its value persists, commit screens clean — and `choose_deduction` returns the itemized arm even with no Schedule A (`return_1040.rs:391,397-398`: "`ForceItemize` ⇒ itemized always … even with a `None` Schedule A that makes it $0"). Result: a committed return with a $0 deduction and no in-form trace of why — worse than TOML, where the key at least sits visibly in the file. Fix: define `DeleteSection(ScheduleA)` to also reset `itemize_election` to `Auto` (or keep the field always-live), and generally state the spec's posture on non-live set values (core treats them as recorded over-asks; this is the one that changes the computed result).

**I-11. §6/§5.7 — `commit` for a year without full-return tables is undefined.**
`screen_inputs` requires `&TaxTable` + `&FullReturnParams`, and only TY2024 has full-return params (`resolve.rs:86-94` fails closed on other years, `refusal: None`). The vault holds many years; §12's year picker will surface them. For a non-2024 year the commit gate *cannot run*: does the form refuse to open the year, open read/draft-only, or commit unscreened (which would then brick the year at resolve — exactly the §6.1 poisoning the draft table exists to prevent)? The spec never says. Fix: gate the form (or at least commit) to table-bearing years, explicitly.

---

## MINOR

**M-1. §2 vs §5.8 vs §5.1 — scope statements disagree on `payments`/`sch1`.** §2's DEFERRED list omits `payments` and the `sch1` money leaves (both exempted in §5.8), and §5.1 even names "Payments" as a singleton-section example while §5.7's `SectionId` has no Payments. Also, the ★ SCOPE NOTE leaves an owner decision open inside a spec headed to a gate — resolve it before the plan.

**M-2. §7 — the SALT row's anchors are imprecise.** The `(+ Section(W2s) for the withholding leg)` applies only to `SalesTaxElectionWithoutAmount` (its trigger includes `income_tax_salt > 0`), not to `SaltSalesTaxWithoutElection` (which reads only the two Schedule A fields); and `income_tax_salt` also sums `salt_state_estimated_payments` + `salt_prior_year_balance_paid` — in-form fields the anchor doesn't name.

**M-3. §7 — two anchors ignore their deferred-struct trigger legs.** `SpouseOwnerWithoutJointReturn` also fires from `schedule_c.owner` (`return_refuse.rs:655-658`) and `NonPublicCharityContribution` also fires from `charitable_carryover_in` (`return_refuse.rs:624-627`) — both deferred to TOML, so the pure `Section(W2s)`/`Section(ScheduleACharitable)` anchor can point at an innocent section. Add the `NotInForm` leg to each.

**M-4. §7 D-4 honesty note under-enumerates: `NonCryptoNoncashGift` is commit-invisible but form-triggerable.** It is raised in `return_1040.rs:610` (compute-side), *not* in `screen_inputs` — yet a >$500 noncash gift is entered entirely through v1 form fields. The form will say "screens clean," and `report` will then refuse. It belongs in the named "what the form cannot see" list (it's the only such refusal reachable from v1 form data alone).

**M-5. §5.7 — `FieldValue::Choice(&'static str)` cannot implement `Deserialize`.** A `serde`-serializable `Edit` (the stated web wire contract, §4/§13) cannot round-trip a `&'static str`. Use `String` or a choice id. ("Illustrative, not final" softens this, but the serializability claim is load-bearing.)

**M-6. §5.7 — `ClearField` semantics are undefined per kind.** TriState→`None` is obvious; Money (→ $0?), Text (→ ""), Secret (→ empty), and especially Enum (`filing_status` and `owner` have no empty state) are not. The un-answer path is answered-ness-relevant; pin it.

**M-7. §3 — the new crate name `btctax-form` sits one letter from the existing, published `btctax-forms`** (the IRS-PDF filler, a current `btctax-cli` dependency). The spec never acknowledges the collision-adjacent name; with all crates published to crates.io this is a self-typo-squat. Pick a distincter name (`btctax-form-engine`, `btctax-input-form`).

---

## NIT

**N-1. §9 — "self-deadlocks on the exclusive `VaultLock`" is wrong in mechanism:** `VaultLock::acquire` uses `try_lock_exclusive` (non-blocking, `lock.rs:18`), so a nested `Session::open` **errors** with lock contention; it does not hang. The conclusion (in-session `return_inputs::delete` on the held conn) is unchanged and correct — `session.rs:611` documents the same pattern for `optimize::accept`.

**N-2. §5.7 — `commit(…, t: &TaxTables)`: no `TaxTables` type exists;** `screen_inputs` needs `&TaxTable` **and** `&FullReturnParams` (`return_refuse.rs:516`).

**N-3. §6.1 — "fresh year = 8 `None` declarations":** only ~5 are *live* on a fresh Single year (DependentSpouse, MfsSpouseItemizes, MortgageAllUsed aren't). The refused-mid-entry point stands.

**N-4. Citation drift:** `session.rs:389` → the `pub fn open` is at :390; "mirroring `return_inputs.rs` (~100 lines)" → the file is 328 lines (≈200 non-test). Cosmetic.

---

## Positive confirmations (verified against source)

- **§5.8 inventory:** complete and correct **except I-1 (ip_pin) and I-8 (Tri→bool)**. All leaves of `ReturnInputs`, `HouseholdHeader`, `Person`, `Dependent`, `W2` (all 14 fields incl. `box12`), `Box12Entry`, `ScheduleAInputs` (all 10), `CharitableGift` are each exactly one Field, registry entry, or listed exemption. **No deleted field is resurrected** (`ssn_valid_for_employment` ×2, `box13_retirement_plan` are absent, as they should be). Enum option lists are exact: `FilingStatus` {Single,Mfj,Mfs,HoH,Qss}, `Owner` {Taxpayer,Spouse}, `ItemizeElection` {Auto,ForceItemize}, `CharitableClass` (all 6, spelled right). Declaration liveness matches `FORM_QUESTIONS` verbatim (incl. `DependentSpouse` = `Mfj || spouse present` and the mortgage predicate `schedule_a ∧ 1098 > 0`); skippable set and liveness match `answer.rs` exactly ({DOB×2, blind×2, SALT}, gated at lines 158–170 as cited).
- **§7 exhaustiveness:** all **37** `RefuseReason` variants are placed — no variant is missing from the map. The defects are the two mis-attributions (I-3, I-4), the false exactness claim (I-5), and the minor anchor imprecision (M-2, M-3).
- **Code citations:** `resolve.rs:85` early-returns on a present RI row in all three sub-branches before `tax_profile::get` at :112 ✓; `screen_inputs → Option<Refusal>`, first-refusal only ✓; `FORM_QUESTIONS` is 8 (len-pinned test) ✓; committed-row writer list is complete (tax.rs:98, answer.rs:309, tax.rs:461, tax.rs:165 delete — nothing else in non-test code; `set-pii` is docs-only/unimplemented, worth marking "future" in §6.2) ✓; `return_inputs` DDL is mirrored exactly and `StaleReturnInputs` refuse semantics are as described ✓; the three named `answer.rs` tests exist (:397, :478, :509) ✓; `Ssn::canonical`/`IpPin::canonical`/`mask_ssn`/`IpPin(******)` all exist ✓; the §10 "form-commit preserves Computed carryovers" claim is sound (carryover structs are exempt from the form and ride the working copy; §6.2 clears a pre-write-carryover draft) ✓; the crate graph is cycle-free (`btctax-tui-edit → btctax-cli` already exists today) ✓; the §11 build order is executable as phased (phase 2's registry sections precede phase 3's coverage KAT, which needs them) ✓.
