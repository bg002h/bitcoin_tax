# INDEPENDENT REVIEW — SPEC_input_surface.md (P8), commit `2a82d89`, review r1

*Persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2. Author = Opus; reviewer = Fable
(the same reviewer who gave the ARCH opinion this spec builds on — including the D-3 position the author
dissented from). Nothing below has been edited, softened, or reordered.*

**VERDICT: 1 Critical / 7 Important / 4 Minor / 2 Nit**

---

Every factual claim below was checked against source at HEAD; the suite is green at review time (`make check`: 1710/1710 passed, 7.3s).

## Verification of the four mandated claims

1. **"`income import` never validates" — TRUE.** `screen_inputs` appears 0 times in `crates/btctax-cli/src/cmd/tax.rs`; `import_return_inputs` (tax.rs:49–100) is parse → carryover-merge → store, no screen. CLI call sites are exactly `resolve.rs:96` and `cmd/admin.rs:453` (the export path).
2. **"Empty SSN is deliberately not a refusal" — TRUE in the narrow form; FALSE in the §3 generalization.** `first_malformed_ssn` skips empty (`return_refuse.rs:161–165`), and the §3 quote matches `return_refuse.rs:45–47` verbatim. But "every refusal means WRONG, not UNFINISHED" is false — see Finding 2, which settles D-3.
3. **Drift alarm — HAS A HOLE**, confirmed empirically. See Finding 3.
4. **"`first_negative_amount` destructures every input struct with no `..`" — FALSE for the header.** True for `ReturnInputs`, `W2`, `Box12Entry`, `Form1099Int/Div/G`, `ScheduleCInputs`, `ScheduleAInputs`, `CharitableGift`, `CharitableCarryItem`, `Schedule1Inputs`, `Payments`, `QbiInputs`, `Carryforward` (return_refuse.rs:191–474). But `header: _` (return_refuse.rs:196) — `HouseholdHeader`, `Person`, `Dependent` get **no** compiler forcing. See Finding 4.

Also verified: the D-5 carryover-preservation precedent exists exactly as cited (tax.rs:58–97); the `--force` precedent exists (tax.rs:20–33); `IncomeCmd` today is Import/Show/Clear only (cli.rs:357–379), no name collisions; no user-facing example/template TOML exists in the repo.

---

### [CRITICAL] D-5's merge rule fails the partial-header case — the exact data loss it vows to prevent, on a spec-sanctioned path

**Where:** §5 D-5; §7 step 3 (the merge-not-clobber KAT).

**What:** D-5's rule is scoped to a re-import "when the file supplies **none**" (no header). But D-2 explicitly sanctions mixed mode ("A user who wants TOML-only PII can uncomment it"), and mixed mode is the *natural* workflow: names and address in TOML (the packet needs them; they are not identity-theft-grade), SSNs via `set-pii`. Serde structure makes the clobber case the norm, not the edge: `HouseholdHeader.taxpayer` has no `#[serde(default)]` and `Person` requires the `first_name`/`last_name`/`ssn` **keys** (`return_inputs.rs:124–135, 150–151`) — so any uncommented header block *must* carry an `ssn` key, and the template's placeholder will be `ssn = ""`. Every subsequent re-import of that file supplies a header → the whole-blob upsert (tax.rs:98) overwrites the vault SSNs/IP PIN/DOBs with empties. The KAT as specified (file-supplies-none) **passes while the defect lives**.

**Why it matters:** This is data loss of exactly the data Cycle 2 exists to protect, and — by Cycle 2's own design ("identity never touches disk") — it is **unrecoverable**: the set-pii-entered values exist nowhere but the vault row being clobbered. The spec calls D-5 "the highest-risk defect in the cycle" and then mitigates only its narrowest aperture.

**Evidence:** tax.rs:58 ("`income import` is a whole-blob upsert"); the carryover special-case (tax.rs:63–97) shows the precedent is *field-level* preservation, which D-5 does not extend to the header; `return_inputs.rs` serde attributes as cited.

**Fix:** Specify **field-level merge semantics for PII leaves**: an empty/absent PII leaf in the file preserves the vault value; a non-empty file value wins. Specify the intentional-clear path (via `set-pii` or `income clear`, never via empty TOML strings). KATs must cover: (a) file supplies no header; (b) file supplies a partial header with `ssn = ""` (the template-placeholder state); (c) a non-empty file SSN wins over the vault; (d) both orderings, `set-pii`→`import` and `import`→`set-pii`.

---

### [IMPORTANT] §3 is false as stated; D-3 is SETTLED for the author's REFUSE default — but on a ground neither side argued

**Where:** §3 (the "load-bearing fact"), §5 D-3.

**What — the falsification (confirming the author's own retraction, with the full classification requested):** the claim "every `screen_inputs` refusal means the data is WRONG, not merely UNFINISHED" does not survive enumeration. Classifying all 21 input-screenable refusals (verified against `screen_inputs`'s body, return_refuse.rs:479–726):

- **INVALID** (cannot be true): `NegativeAmount`, `SsnMalformed` (non-empty only — a partial SSN is wrong, not on-its-way; the code already draws the ABSENT≠PARTIAL line at return_refuse.rs:161–165), `SpouseOwnerWithoutJointReturn`, `InconsistentDividendSubset`, `SaltSalesTaxWithoutElection` (internal inconsistency: amount set, election off).
- **UNANSWERED** (required question is `None`/empty; data entered so far is true): `ScheduleBPart3Unanswered` (:509), `MfsSpouseItemizeUnknown` (:531), `ScheduleCNoBusinessDescription` (:598). These directly falsify §3's corollary "staged entry is already expressible without tripping the screen" — an MFS filer who hasn't yet asked their spouse about itemizing trips the screen with wholly true data.
- **UNSUPPORTED** (data is TRUE; btctax's scope is short): `ForeignTrust`, `NonPublicCharityContribution`, `DependentSpouseUnsupported`, `AllocatedTips`, `DependentCareBenefit`, `UnsupportedBox12Code`, `PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`, `ForeignTaxOverCeiling`, `HsaPresent`, `IraDeductionClaimed`. I also **reclassify two of the principal's provisional placements**: `ExcessElectiveDeferral` is UNSUPPORTED, not INVALID — the real-world case is a mid-year job change with two 401(k)s, true data whose 1040-1h treatment the enum doc itself calls "unmodeled in v1" (return_refuse.rs:654); `SingleEmployerExcessSs` straddles (usually a typo, occasionally a true employer error whose remedy is off-return — either way uncomputable).

**The settlement.** My architecture position (warn-and-store) is **retracted**. The decisive fact, which neither the author's dissent nor my advice used, is the resolve precedence ladder: a stored `ReturnInputs` blob is precedence-1 and, when refused, renders the **entire year** `Uncomputable` — including the crypto-only report and the raw `tax-profile` escape hatch, both of which sit *below* it on the ladder (resolve.rs:85–110 precede :112). Warn-and-store therefore does not "let the user iterate": it converts a working year into a broken one, and the recovery message (resolve.rs:213–226) then directs the user to `income clear` — which, post-Cycle-2, destroys unrecoverable PII (see Finding 8). My "iterate with `income show`" argument was wrong on its own terms: the iteration medium is the TOML file, which persists regardless of import outcome, so refusal loses no work. And the third class does not flip the verdict: for UNSUPPORTED data, refusing to *store* is the kind option — storing a truthful HSA blob silently breaks the user's crypto report and then steers them to the PII-destroying clear. Refuse-with-explanation beats store-and-poison for all three classes.

**Why it still blocks:** the spec's stated rationale is unsound, and D-3 as written would ship refusal messages that tell an UNSUPPORTED user to "fix the import" when nothing is fixable.

**Fix (amend, don't flip):** (1) Rewrite §3 around the true narrow fact (empty SSN ≠ refusal; ABSENT ≠ PARTIAL) plus the ladder-poisoning ground above. (2) Adopt the three-class taxonomy and make import's refusal output **class-aware**: INVALID → "fix the file"; UNANSWERED → "set `<key>`"; UNSUPPORTED → "true but out of scope — options: `tax-profile` for the crypto-only report, or another preparer; nothing was stored, your current report still works." (3) `--force` messaging must state the consequence — the year becomes uncomputable until it screens clean — because the cited `tax-profile set --force` precedent is *inverted*: that one stores a harmlessly-shadowed value; this one arms the poison. (4) Record `--force`'s legitimate use: staged entry with plaintext hygiene (store confidential money data in the vault, delete the TOML, finish later, accepting an uncomputable year meanwhile).

---

### [IMPORTANT] §6's drift alarm does not bite as specified — and the author's proposed key-set fix reopens the hole unless built in a null-visible representation

**Where:** §6; consequentially §4 (the alarm is the entire justification for declining the schema).

**What:** Confirmed: `return_inputs.rs` carries exactly **85** `#[serde(default)]` attributes, and (empirically verified, toml 0.8 — the repo's version, btctax-cli/Cargo.toml:33) a missing `Option` key parses to `None` even *without* `serde(default)`. So a template that omits a field still parses, and value-equality passes whenever the fixture's value for that field equals its default. Nothing in §6 forces non-default fixture values, so "equality then forces the template to carry every field" is false. **Answer to the author's direct question:** yes, `toml::to_string` silently drops `None`-valued fields — probe output on toml 0.8:

```
req = "x"
empty_vec = []
defaulted = ""        # opt_none: Option<bool> = None — key GONE
```

so a key-set built by re-serializing the fixture *through TOML* is not airtight either. `serde_json::to_value` of the same struct shows `"opt_none": null` — None is visible there.

**Fix — the airtight construction (three assertions):**
- **KAT A (example correctness):** uncommented template parses via `parse_return_inputs_toml` (tax.rs:103) and `==` the typed fixture.
- **KAT B (fixture completeness, mechanical):** walk `serde_json::to_value(&fixture)` and assert **no `null` and no empty array, recursively**. This forces every `Option` to `Some` and every `Vec` to carry an exemplar — and it catches even a fixture that used `..Default::default()` on a nested struct when a new `Option` field lands (the new field surfaces as `null`).
- **KAT C (template completeness):** compare the recursive key-paths of the fixture's JSON value against the key-paths present in the raw template parsed as `toml::Value`.

Residual risk to note in the spec: a future `#[serde(skip_serializing_if)]` would evade B/C — none exist in `return_inputs.rs` today; ban it there by convention (a one-line grep KAT if desired). **With this amendment the §4 schema-decline stands** — the mechanism becomes strictly stronger than the spec's current version, so my original decline advice survives, but only after §6 is rewritten.

---

### [IMPORTANT] §4's "compiler-audited" claim is overbroad — the header structs are exactly the uncovered ones

**Where:** §4, sentence "`first_negative_amount` destructures every input struct with no `..`".

**What:** False for `HouseholdHeader`/`Person`/`Dependent`: the destructure wildcards them (`header: _, // PII only — no money`, return_refuse.rs:196). A field added to `Person` (e.g. a future prior-year-AGI for e-file identity) breaks nothing at compile time.

**Why it matters:** These are precisely the structs Cycle 2 writes, and §4 offers this claim as a pillar of the schema-decline. Left uncorrected, an implementer may trust `header: _` coverage and build the §6 fixture without exhaustive header population — shipping the exact silent drift §6 promises to prevent.

**Fix:** Correct the sentence ("every *money-bearing* struct; the header structs are covered by the drift KAT, not the compiler") and require in §6 that the fixture populate the header exhaustively — `spouse: Some(..)`, `dependents` ≥ 1, DOBs `Some` — which amended KAT B then enforces mechanically.

---

### [IMPORTANT] The header is not pure PII — tax-changing facts live in the "never needs TOML" block, and `set-pii`'s field coverage is unspecified

**Where:** §3 ("identity… never needs to" live in TOML), §5 D-2 (the commented-out PII block), §2 Cycle 2, §9 acceptance.

**What:** `HouseholdHeader` carries **compute-relevant** facts: `Person.date_of_birth` drives the §63(f) age-65 std-deduction add-on ("DOB drives §63(f) age-65 (F3)", return_inputs.rs:120); `Person.blind` drives the blind add-on; `can_be_claimed_as_dependent_taxpayer` drives the dependent std floor and the kiddie-tax screen (return_1040.rs keys the 8615 screen on it); `can_be_claimed_as_dependent_spouse` is itself a `screen_inputs` refusal trigger (return_refuse.rs:570). The spec's identity/money dichotomy has a third category it never names. On the default path (money TOML + `set-pii`), a claimable-as-dependent filer whose flag never gets entered — because the whole header is presented as skippable PII — receives the **full** standard deduction: a silent **understatement**, the one direction this project's entire refuse-guard architecture exists to prevent. A 65+ filer silently loses the add-on (overstatement). And the spec never says what `set-pii` actually prompts for: acceptance names only "SSNs, the IP PIN and dependents' DOBs" — not taxpayer/spouse DOB, blind, the claimed-flags, address (the packet needs it), or **creating dependent rows** (a default-path family has no rows for set-pii to "merge into").

**Fix:** Enumerate `set-pii`'s coverage explicitly — the full `HouseholdHeader`, including creating dependents — and either label the tax-changing fields inside D-2's commented block as *not-optional-for-correctness*, or move the non-secret tax facts (blind, claimed-flags) out of the PII block into the money TOML.

---

### [IMPORTANT] `set-pii` on a year with no stored row is unspecified — the obvious implementation silently changes the liability

**Where:** §2 Cycle 2 ("merges into the vault row"), §5 D-5, §7 step 3.

**What:** If `set-pii` creates a default row to merge into, that row is screen-clean (an all-default `ReturnInputs` trips nothing in `screen_inputs`) and sits at precedence-1 — so a user with a stored `tax-profile` who runs `set-pii` "just to enter my SSN for the export" silently flips their report from the stored profile to one derived from **all-zero** non-crypto inputs (resolve.rs:85–110 beat :112). That is the "two liabilities / silently different number" cardinal sin the resolver module documents itself against (resolve.rs:3–5).

**Fix:** Specify: `set-pii` on a year with no `return_inputs` row **refuses** with a pointer to `income template`/`income import` (or requires an explicit create flag). Add the ordering KATs from Finding 1(d).

---

### [IMPORTANT] The store-gate must be entry-path-agnostic — as specified, `set-pii` can store what `income import` refuses

**Where:** §5 D-3 (import-only), §7 step 3.

**What:** D-3 gates only the TOML door. `set-pii`'s no-echo prompts are the *most likely* place a malformed SSN is typed (the user cannot see the typo), and a stored 8-digit SSN is `SsnMalformed` at the next resolve — the same poisoned-year defect D-3 exists to close, through the other door.

**Fix:** One spec sentence: no entry path may persist a blob that `screen_inputs` refuses, under the same default/`--force` semantics; `set-pii` additionally validates at the prompt (`Ssn::canonical`, `IpPin::canonical`, packet.rs:55/121) so the user is told before the prompt sequence ends, and screens the **merged** blob before store.

---

### [IMPORTANT] `income clear` is the tool's own advertised recovery — and after Cycle 2 it destroys PII that exists nowhere else

**Where:** Missing from scope (§2/§5); interacts with D-5 and Cycle 2's "identity never touches disk".

**What:** Every uncomputable-year message directs the user to `income clear` (resolve.rs:216–224) — including compute-dependent refusals discovered *after* a successful import + `set-pii` (e.g. `ScheduleCLoss` at report time). Following the tool's own advice then deletes vault-only SSNs/IP PIN/DOBs, unrecoverable **by the cycle's own design**. The spec introduces the unrecoverable data without addressing its advertised destruction path.

**Fix:** Specify one of: `income clear` warns/confirms when the header is populated; a money-only clear variant that preserves the header; or amend the `uncomputable_detail` guidance to state the PII consequence. Any of the three closes it; pick in the spec, not in the implementation.

---

### [MINOR] D-4's compute-dependent enumeration is wrong, and the third screen is missing entirely

D-4 lists "(`ScheduleCLoss`, `KiddieTax`, `QbiAboveThreshold`, `AmtScreenTriggered`)". Actual: `screen_compute_dependent` (return_1040.rs:540) fires `NonCryptoNoncashGift` (aggregate with the ledger — note it fires here, not in `screen_inputs`, despite sitting in the enum's input-screenable section), `BusinessInterestIncome`, `BusinessIncomeWithoutScheduleC`, `ScheduleCLoss`, `KiddieTax`; and there is a **third** screen, `screen_absolute` (return_1040.rs:1349), firing `QbiAboveThreshold`, `AmtScreenTriggered`, `TaxableIncomeNonPositiveWithCarryforward` — the export path runs all three (admin.rs:453–465). Fix: D-4's import success message should name the *classes of checks not run* (compute-dependent + absolute), never a hardcoded list that is already wrong.

### [MINOR] §8's "OVERDUE" claim on `p1-ssn-normalization-P6` is stale — the ledger marks it DONE in P6.1

`design/full-return/FOLLOWUPS.md:617` records it "✅ DONE in P6.1" (canonicalization wired into `screen_inputs` as `SsnMalformed`; KAT `an_uncaptured_ssn_does_not_block_the_report`), with the empty-vs-malformed split recorded as an accepted declared deviation (:622–630). The spec's proposed disposition ("confirm the packet-time canonicalization is the single source and close") *is* the recorded resolution. The residue question — should import additionally canonicalize at capture — is legitimate new P8 scope, but it is not "overdue". Also clean the stale duplicate at FOLLOWUPS.md:718 ("carried; unchanged") so reconciliation-by-grep yields one answer.

### [MINOR] Screen the blob AS IT WILL BE STORED, not the file's parse

Import order must be parse → carryover-merge (tax.rs:63–97) → (post-D-5) header-merge → **screen** → store. §7 step 2 doesn't pin this; screening the raw parse would let stored bytes differ from screened bytes.

### [MINOR] D-2 must forbid plausible dummy PII in the template's commented block

A syntactically valid example SSN ("123-45-6789") survives `Ssn::canonical`, passes the screen, and **prints on the filed 1040** if a bulk-uncommenting user never runs `set-pii`. Placeholders must be values the packet refuses (empty strings → `SsnError::Missing` fail-closed). The §6 KATs are unaffected (KAT B tolerates empty strings; only nulls/empty arrays are banned).

### [NIT] §2: the TUI buffer is `[FieldBuffer; 10]`, not 9

draw_edit.rs:2303. The deferral argument (no form engine; per-flow state machines) is otherwise verified and stands.

### [NIT] Pre-existing code-comment drift found during review (file to FOLLOWUPS, not this spec)

`ScheduleCNoBusinessDescription` sits under the enum's "Compute-dependent rows" banner (return_refuse.rs:103 vs :112) but fires in `screen_inputs` (:598).

---

## The journey walk (§9 acceptance)

`init` → crypto `import` → `reconcile` → `income template` → fill → `income import` (now screened) → `set-pii` → `report` → `export-irs-pdf`: with Findings 1, 5, 6, 8 fixed, the walk completes for the Common W-2 household — the remaining walls found are all filed above (set-pii coverage/creation, partial-header merge, clear-destroys-PII). One non-finding worth recording: MFJ-with-no-spouse-record is deliberately un-screened (advisories.rs:218) and refuses at the packet like an empty SSN — consistent with the ABSENT-refuses-at-the-packet design, no action.

## Bottom line

The architecture is right and the scope is right: template + import screening + `set-pii`, schema declined, TUI deferred — all survive adversarial checking, and on D-3 **the author's dissent from my advice was correct** (though for a reason neither of us gave; my warn-and-store is retracted on the precedence-ladder evidence). What blocks is the artifact's load-bearing reasoning (§3 false as stated, §6's alarm toothless as specified, §4 overbroad) and the PII-lifecycle cases the cycle itself creates (partial-header clobber, set-pii ordering and coverage, clear-as-data-loss). All fixes are spec amendments; none reverses a design decision.

**VERDICT: 1 Critical / 7 Important / 4 Minor / 2 Nit**
