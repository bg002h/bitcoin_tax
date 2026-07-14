# INDEPENDENT REVIEW — SPEC_input_surface.md r3 (P8), review r3

*Persisted VERBATIM before folding. Author = Opus; reviewer = Fable.*

**VERDICT: 1 Critical / 4 Important / 4 Minor / 1 Nit**

**Direct answers first:** (1) Back-compat is *worse* than "free" — `false` does deserialize to `Some(false)`, which is precisely the problem (C1). (2) The ask-the-user list is complete under the right criterion — but the spec never states the criterion, and `Dependent.relationship` is the near-miss. (3) The exemption assertion relocates the problem; the criterion is what closes it. (4) Refusing on *duplicates* is right; refusing on *renames* strands the user (I2). (5) Fifth fold-defect: yes — three.

---

### [CRITICAL] D-8's "back-compat is free" LAUNDERS the guess — the fix reaches not one already-affected row

**Where:** §5 D-8 ("a stored blob serialized `false` deserializes to `Some(false)`"), §9 bullet 3; `crates/btctax-cli/src/return_inputs.rs:44` (`serde_json::to_string(ri)`), `return_inputs.rs:164`.
**What:** Verified — the store writes the whole struct with no `skip_serializing_if`, so **every** stored row already on disk carries `"can_be_claimed_as_dependent_taxpayer": false` (the bare `bool` is always serialized, whether or not the user ever saw the field). After the migration that `false` becomes `Some(false)` — an *answered* "No". The new UNANSWERED refusal therefore **never fires for any pre-existing row**. The guess is not fixed; it is promoted to an answer and made permanent.
**Why:** D-8 is billed as fixing "a live understatement of tax in released software," and §9 promises "a claimable-as-dependent filer cannot silently receive the full standard deduction." For the entire population that already has the bug — every user with a stored `return_inputs` row, the only population that *can* have it today — both statements stay false after P8 ships, and nothing tells them. A guarantee the artifact claims and does not deliver.
**Evidence:** `return_inputs.rs:32/44` (JSON in, JSON out, no schema version); `false` → `Some(false)` is standard serde. There is no marker distinguishing a user-supplied `false` from a never-asked `false`.
**Fix:** Decide the migration **in the spec**. The sound form: add a `#[serde(default)] schema_version` to `ReturnInputs` (absent ⇒ 0, pre-P8); on load, a version-0 row's claimed-flags map to `None`, so the year refuses UNANSWERED with "answer `can_be_claimed_as_dependent_taxpayer` and re-import." That takes previously-computing years down **in the safe direction only**, one TOML line recovers, and it is the same fail-loud doctrine D-8 invokes. If instead the launder is accepted, say so explicitly and add a one-time advisory — but "back-compat is free" cannot stand as written.

### [IMPORTANT] D-8's refusal is unscoped — a Single filer is force-asked a question about a spouse who does not exist, and the default journey refuses

**Where:** §5 D-8 ("both claimed-flags → `Option<bool>`; `None` fires UNANSWERED"), D-2 (both ship **commented**); `return_refuse.rs:570`.
**What:** `can_be_claimed_as_dependent_spouse` today only refuses when **true** (`DependentSpouseUnsupported` — out of scope). Making `None` refuse *unconditionally* means every Single/HoH/QSS filer must answer a spouse question with no referent — and since D-2 ships the flag commented, the out-of-the-box path (`income template` → fill money → `import`) **refuses for every filer**, naming a key about a nonexistent spouse. That breaks §9's first acceptance bullet on the majority filing status.
**Why:** The project's own precedent scopes exactly this: `MfsSpouseItemizeUnknown` fires only on MFS (`return_refuse.rs:531`). D-8 copied the tri-state without copying the scoping.
**Fix:** Taxpayer flag: `None` refuses unconditionally (the 1040 asks everyone). Spouse flag: `None` refuses **only when the return has a spouse** (MFJ, or `header.spouse.is_some()`); otherwise `None` is inert and unconsumed. Name the new refusal variant in §3.5.

### [IMPORTANT] Fold-introduced: D-5 rule 5's "refuse on **renamed** names" makes a dependent's name typo uncorrectable without destroying the secrets

**Where:** §5 D-5 rule 5 ("duplicate **or renamed** names ⇒ refuse … re-run `set-pii`").
**What:** Trace it. Vault row holds dependent "Jonh" + SSN. User fixes the typo to "John" and re-imports ⇒ no identity match ⇒ **refuse** ⇒ nothing is stored ⇒ the vault keeps "Jonh". Now: `set-pii` prompts for **SSNs and the IP PIN only** (D-6) — it cannot edit a name. `income clear` destroys the SSNs. `--keep-identity` keeps the old name and fail-closes. **There is no exit.** The only escape is exactly the secret-destruction D-7 exists to prevent.
**Why:** rule 5 conflates two different events. A *duplicate* name inside one file is genuinely ambiguous (refuse — and this doesn't strand: `set-pii` walks the vec positionally). An *unmatched* name is not ambiguous at all — it is a new or renamed person, and the merge simply has no secret to inherit.
**Fix:** Unmatched file dependent ⇒ treated as **new**: import succeeds, its SSN is empty, and the user is told *"no stored SSN for 'John' — run `income set-pii`."* Only **within-file duplicate names** refuse. (The empty SSN is already a safe state: `first_malformed_ssn` skips it, and the packet fail-closes at export.)

### [IMPORTANT] Fold-introduced: D-5 rule 4 opens a SILENT secret-destruction path that D-7 closes everywhere else

**Where:** §5 D-5 rule 4 ("structure follows the file"), vs §5 D-7 bullet 3 (`income clear` "warns and requires confirmation when the header carries secrets").
**What:** Deleting `[header.spouse]` — or a `[[header.dependents]]` block — from the **money** TOML now permanently deletes that person's vault-only SSN on the next `income import`, with no warning, no confirmation, no `--force`. The spec asks for a confirmation when `income clear` does this, and grants none when a one-block edit to a file the user thinks contains no secrets does the same thing on the routine path.
**Why:** rule 4 is correct as *merge semantics* (a removed block must not resurrect a secret) and wrong as *policy*: r2's fix for one hole cut a new one in the same protection D-7 states as an invariant ("`income clear` must not eat the secrets").
**Fix:** Import compares the merged header against the stored one; if the merge **drops a person who carries a vault secret**, it warns and requires confirmation (or `--force`), naming them. Add the KAT.

### [IMPORTANT] Fold-introduced: §6's KAT triad and D-2 are now mutually unsatisfiable — `ip_pin` cannot exist and must exist

**Where:** §6 KAT A / KAT B; §5 D-2 (ask-the-user row: `ip_pin` "must not appear in a plaintext template **at all**") vs D-2 rule 2 (the grep KAT "requires each exempted field's commented doc line to be present").
**What:** Three-way contradiction, in one decision. (a) D-2 forbids `ip_pin` from the template *and* the grep KAT mandates its commented doc line. (b) **KAT B** bans `null` in the fixture ⇒ the fixture must carry `ip_pin: Some(..)`, `date_of_birth: Some(..)`, `foreign_accounts: Some(..)`. (c) **KAT A** says the "uncommented template" parses and `== fixture` — but the ask-the-user fields are *commented*, and `ip_pin` has no line to uncomment. KAT A cannot pass as written; an implementer under a red build will weaken KAT B, which is the assertion that catches every future `Option` field.
**Why:** §6 is the entire justification for §4's schema-decline. Its three assertions must be buildable, and post-D-2 they are not.
**Fix:** Pin the resolution in the spec: (1) "commented" means a `# key = <example>` line — that *is* the doc line, and it satisfies "never a live secret in the file"; drop the "at all". (2) KAT A operates on the template with the ask-the-user lines **mechanically uncommented** (a named transformation, tested). (3) KAT B's null-ban and KAT C's key-path check share the **same asserted exemption literal** — a *new* `Option` field is not in that set, so it is still caught, and the alarm keeps its teeth.

### [MINOR] §7 — the build order (which IS the plan; there is no plan doc) is stale w.r.t. r3

No step for D-8 (the `Option<bool>` migration + the new refusal + the C1 migration decision — the largest change in the spec, and the only one that touches shipped compute), none for the D-2 exemption/grep KATs, none for `--clear-ip-pin`, none for the `export-irs-pdf` message. Step 1 still lists only the `Person` `serde(default)`. An implementer executing §7 literally ships none of it.

### [MINOR] Mandate `Some(true)`-testing at the consumers — `unwrap_or(false)` would reintroduce the exact bug

`printed.rs:936` (`ri.foreign_accounts.unwrap_or(false)`) is the shipped idiom, and it is the *shape* of the defect D-8 fixes: a `None` silently becoming a taxpayer-favourable "No". When the flags become `Option<bool>`, `return_1040.rs:78` and `:618` must test `== Some(true)` (or match), never `unwrap_or(false)`. `None` is unreachable post-`screen_inputs` — say so, and forbid the re-guess.

### [MINOR] State the ask-the-user **membership criterion**, not just the list (this is what makes the exemption safe)

The list is complete — I checked every silently-defaulted field: `presidential_fund_*` (unchecked = the true no-election state; no tax effect), `blind` / `itemize_election` / `salt_use_sales_tax` / `accounting_method` (all default in the **overstate** direction or neutral, and all ship *visible*). The governing rule they satisfy — and which the spec never writes down — is: **a field may be exempt from KAT C only if its absence either fails loud or is conservative *and* advised.** With that stated, "asserted inside the KAT" stops merely relocating the problem (an implementer can still add a line to the exemption list under a red build; a *criterion* is what a reviewer can hold them to). Near-miss to record: `Dependent.relationship` defaults `""` and **prints blank on the filed 1040** (`form1040_full.rs:376`) with no screen — the same facial-incompleteness class as `ScheduleCNoBusinessDescription`, which the code *does* refuse.

### [MINOR] Fold residue in the spec text

D-5 now carries **two** KAT paragraphs — the new six-item list (lines 230–231) and r2's stale "**KATs (all four)**" block (233–235) — which contradict each other; delete the latter. §3.5 still says "**Twenty-one** input-screenable refusals" and its table omits D-8's new UNANSWERED variant (and `DependentSpouseUnsupported` must stay a *separate* variant, since one `RefuseReason` cannot carry two classes).

### [NIT] `ssn_valid_for_employment` is entirely unconsumed

Zero references outside its definition (`return_inputs.rs:129/143`). KAT C will force it into the template, where it asks the user a question that changes nothing on any form or number. Either wire it or note it as reserved.

---

## Bottom line

r3's two Critical folds are directionally right — D-8 correctly names a live understatement in shipped code, and D-2's ask-the-user class correctly un-does the force-answered disclosure. But D-8's remedy stops at the door of the population it was written for: `false` → `Some(false)` means the fix repairs the *future* and silently ratifies the *past*, while §9 keeps claiming otherwise. Around it, the fifth round produced its fifth crop of fold-defects — an unscoped spouse refusal that walls Single filers out of the default journey, a merge rule that traps a name typo, a structure rule that eats secrets silently, and a KAT triad that no longer builds. All fixes are spec amendments; no design decision reverses.

**VERDICT: 1 Critical / 4 Important / 4 Minor / 1 Nit**
