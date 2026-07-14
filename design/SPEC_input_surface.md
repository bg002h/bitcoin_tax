# SPEC — the full-return INPUT SURFACE (P8)

*Ceremony scaled down per `STANDARD_WORKFLOW.md` §8: one spec covering both cycles, no separate plan
document (the build order is §7 below). The gates are NOT scaled down — Fable reviews to 0C/0I.*

---

## 1. The problem

btctax fills a complete, filable 1040 packet (P0–P7 green, validated against two independent engines and
a paper round-trip). **A human still cannot drive it.**

The only way to enter the non-crypto side of a return is:

```
btctax income import --year 2024 --file inputs.toml
```

…where `inputs.toml` deserializes into `ReturnInputs` via serde. There is **no example, no template, no
schema documentation** in the repo. A user cannot discover — without reading Rust — that money fields
are *quoted strings* (`Decimal`), that `owner` is `"taxpayer"` but `filing_status` is `"Mfj"`, or which
fields are required.

**And `income import` never validates.** `screen_inputs` — the ~20-rule fail-closed refuse guard —
appears **zero times** in `cmd/tax.rs`. It runs only later, at `report` and `export-irs-pdf`. So a
malformed SSN, a spouse-owned W-2 on a Single return, or a Schedule C with no business name **parses and
persists silently**, and surfaces much later as a compute-path error with no pointer back to the file.

*The tax is never wrong* — the downstream gate is genuinely fail-closed. This is a usability defect, not
a correctness one. But it is the wrong place to learn you mistyped an SSN.

**And the PII story is incoherent.** The vault encrypts; `income show` masks; `RefuseReason` is careful
never to print SSN digits — and then the input path asks the user to write SSNs, an IP PIN, and
dependents' dates of birth into a **plaintext file on disk, beside the vault**.

## 2. Scope

**IN — Cycle 1 (discoverability + validation):**
- `btctax income template` — a fully commented TOML skeleton, to stdout. No vault, no PII, offline.
- A **drift alarm**: a KAT that fails the build when a new `ReturnInputs` field is not documented.
- **Import-time screening**: `income import` runs `screen_inputs`.
- Parse errors that name the file and point at `income template`.

**IN — Cycle 2 (close the plaintext-PII hole):**
- `btctax income set-pii --year N` — interactive, no-echo prompts for identity; merges into the vault
  row. Identity never touches disk.

**OUT (filed, with rationale):**
- A guided full-return TUI form. `btctax-tui-edit` has **no form engine** — it is per-flow state machines
  and one fixed `[FieldBuffer; 10]`; `ReturnInputs` is dominated by nested collections. That is a new form
  engine plus ~10 flows. Deferred; see §8.
- An external machine-readable schema (JSON Schema). **Declined** — see §4.

## 3. ★ The two facts that shape everything (r2 — the first version of this section was FALSE)

*r1 of this spec claimed "every `screen_inputs` refusal means the data is WRONG, not merely UNFINISHED."
**That is false** — `ScheduleBPart3Unanswered` and `MfsSpouseItemizeUnknown` fire on `None`, i.e. on data
that is true as far as it goes. The conclusion (refuse, don't store) survives; the reasoning did not.*

### 3.1 ABSENT ≠ PARTIAL

**A field is either ABSENT or VALID. It is never partially valid.**

- **ABSENT** is a legitimate *state*. An empty SSN yields a working report and refuses only the packet —
  `RefuseReason::SsnMalformed`'s own doc: *"the tax math never reads an SSN, so a household that has
  entered no PII still gets a report."*
- **PARTIAL** is not a state; it is an error wearing a state's clothes. `"123"` is not an SSN on its way
  to being right. It is wrong, permanently, and storing it puts garbage in the vault the user has been
  told to trust.

The code already draws this line (`Ssn::canonical`: empty ⇒ `Missing`; non-empty invalid ⇒
`NotDigits`/length). **"Unfinished" must never become a licence to store.**

### 3.2 ★ A stored-but-refused row POISONS THE WHOLE YEAR

This is the fact that settles D-3, and neither the author nor the architect had it.

`resolve.rs` is a **precedence ladder**. A stored `ReturnInputs` row is **precedence 1**, and when
`screen_inputs` refuses it, the resolver returns `profile: None` **and never falls through** to
precedence 2 (the raw `tax-profile` escape hatch) — verified at `resolve.rs:85–110`, which precede
`:112`.

**So a refused row does not merely fail to help. It takes the year down** — the crypto-only report and
the hand-entered profile both stop working. And the resulting "uncomputable" message then points the
user at `income clear`… which, after Cycle 2, destroys vault-only PII (D-7).

**Storing what the engine has already judged unusable is not a kindness. It is a trap.**

## 3.5 The three classes of refusal (a compiler-forced classification)

Twenty-one input-screenable refusals, classified:

| class | meaning | examples | store? |
|---|---|---|---|
| **INVALID** | the data cannot be true | `SsnMalformed` (non-empty only) · `NegativeAmount` · `SpouseOwnerWithoutJointReturn` · `InconsistentDividendSubset` · `SaltSalesTaxWithoutElection` | **never** |
| **UNANSWERED** | a required question is `None` | `ScheduleBPart3Unanswered` · `MfsSpouseItemizeUnknown` · `ScheduleCNoBusinessDescription` | never (§3.2) |
| **UNSUPPORTED** | the data is **TRUE**; btctax's scope is short | `HsaPresent` · `IraDeductionClaimed` · `AllocatedTips` · `ForeignTrust` · `ExcessElectiveDeferral` · `KiddieTax`… | never (§3.2) |

**All three refuse to store** — because of §3.2, not because the data is bad. For UNSUPPORTED, the user's
data is *correct* and refusing to store is the **kind** option: storing a truthful HSA would silently
break their working crypto report and then steer them to a PII-destroying `clear`.

**The classification lives on `RefuseReason` as an exhaustive `match`** (`fn kind(&self) -> RefusalKind`),
so a new variant **cannot be added without someone deciding which kind it is.** What differs by class is
only the **message** (D-3).

## 4. Rejected: an external machine-readable schema

I proposed one; the architect declined it and is right.

**The Rust structs already ARE the schema, and the money-bearing ones are compiler-audited.**
`first_negative_amount` destructures every *money-bearing* struct with **no `..`**, so adding a field
there breaks the build until someone classifies it.

⚠️ **But NOT the header.** The destructure wildcards it — `header: _, // PII only — no money`
(`return_refuse.rs:196`) — so `HouseholdHeader` / `Person` / `Dependent` get **no compiler forcing**, and
those are *precisely* the structs Cycle 2 writes. (That comment is also the source of the false
dichotomy this spec inherited — see D-6.) The header's drift protection therefore comes from the §6 KAT,
**not** from the compiler, and §6 must populate it exhaustively.

An external schema would be a **third representation** to keep in sync, and it cannot express the
validation that actually matters: "the sum of box-12 codes D/E/F/G/S across this owner's W-2s must not
exceed the §402(g) limit." JSON Schema can say *"this is a string."* `screen_inputs` says the truth.

**The real shared validation source already exists. The only missing act is CALLING it at entry time.**

*(A JSON Schema for third-party editor completion — taplo, Even-Better-TOML — is genuinely nice and
cheap via `schemars`. Filed as an optional follow-on. Editor sugar, not architecture.)*

## 5. Design decisions

### D-1 — `income template` emits, it does not write
Prints to **stdout**. No `--out`, no vault access. `btctax income template > inputs.toml` is the idiom,
and piping keeps the command trivially safe: it cannot clobber a file or touch the encrypted store.

### D-2 — The template carries EXAMPLE VALUES, and its PII placeholders must be values the packet REFUSES
Every field present. Money demonstrated as a quoted string. Enum variants spelled inline at the field
(note the trap: `filing_status` is CamelCase `"Mfj"`, `owner` is snake_case `"taxpayer"`). Refusal
footguns annotated **at the field that trips them**.

⚠️ **No plausible dummy PII.** A syntactically valid example SSN (`"123-45-6789"`) canonicalizes, passes
the screen, and **prints on the filed 1040** if a bulk-uncommenting user never runs `set-pii`. Placeholders
must be values the packet **refuses** — empty strings (`SsnError::Missing` ⇒ fail-closed). A template that
can produce a filed return bearing a fake SSN is a defect, not a convenience.

### D-3 — ★ Import REFUSES on a screen failure. SETTLED. (`--force` stores anyway.)
**Settled by §3.2, not by whose data is "wrong":** a stored-but-refused row **takes the whole year down**,
shadowing the crypto-only report and the `tax-profile` escape hatch beneath it. The architect's
warn-and-store is **retracted by the architect**. Refusal loses no work — the TOML file is the source of
truth and persists regardless.

Refusal output is **class-aware** (§3.5):
- **INVALID** → "fix the file" — names the field.
- **UNANSWERED** → "set `<key>`" — names the key to add.
- **UNSUPPORTED** → "this is TRUE but out of btctax's scope. Nothing was stored; **your current report
  still works.** Options: `tax-profile` for the crypto-only report, or another preparer."

`--force` stores anyway and **must state the consequence**: *the year becomes uncomputable until it
screens clean.* ⚠️ The `tax-profile set --force` precedent is **inverted** — that one stores a harmlessly
*shadowed* value; this one **arms the poison**. Its one legitimate use: staged entry with plaintext
hygiene (store the confidential money data, delete the TOML, finish later, accepting an uncomputable year
meanwhile).

### D-4 — Screening is ADVISORY about what it cannot see
There are **three** screens, not two: `screen_inputs` (input-only), `screen_compute_dependent` (needs the
ledger — `ScheduleCLoss`, `KiddieTax`, `BusinessIncomeWithoutScheduleC`, `NonCryptoNoncashGift`,
`BusinessInterestIncome`) and `screen_absolute` (needs the assembled return — `QbiAboveThreshold`,
`AmtScreenTriggered`, `TaxableIncomeNonPositiveWithCarryforward`). Import can run only the first.

The success message names **the classes of checks not run**, never a hardcoded list — r1 of this spec
hardcoded one and it was already wrong.

### D-5 — ★ FIELD-LEVEL merge for the secrets. (r1's rule was CRITICALLY wrong.)
r1 said "preserve the vault header when the file supplies **none**." **That is the narrowest aperture of
the defect and the KAT would have passed while the bug lived.**

`Person.first_name`/`last_name`/`ssn` carry **no `#[serde(default)]`** (`return_inputs.rs:124–135`), so
**any** uncommented header block *must* carry an `ssn` key — and the template's placeholder is `ssn = ""`.
So **every re-import supplies a header**, and the whole-blob upsert (`tax.rs:98`) wipes the vault's
`set-pii`-entered SSNs and IP PIN — data that, **by Cycle 2's own design, exists nowhere else.**
Unrecoverable.

**Rules:**
1. Add `#[serde(default)]` to `Person.{first_name,last_name,ssn}` so a header block can legitimately
   **omit** the secret.
2. **Field-level merge on the secret leaves** (SSNs, IP PIN): *absent or empty in the file* ⇒ **preserve
   the vault value**. *Non-empty in the file* ⇒ the file wins (the user chose TOML PII).
3. An intentional clear goes through `set-pii` or `income clear` — **never** through an empty TOML string.

**KATs (all four):** file supplies no header · file supplies a partial header with `ssn = ""` (the
template-placeholder state — **this is the one r1 would have missed**) · a non-empty file SSN wins ·
**both orderings**, `set-pii`→`import` and `import`→`set-pii`.

### D-6 — ★ NEW. The header is NOT pure PII. Split by SECRECY, not by struct.
r1's identity/money dichotomy was **false and dangerous**, and it inherited the falsehood from the
codebase's own comment (`header: _, // PII only — no money`).

`HouseholdHeader` carries **tax-changing** facts:
- `Person.date_of_birth` → the §63(f) age-65 standard-deduction add-on
- `Person.blind` → the blind add-on
- `can_be_claimed_as_dependent_taxpayer` → the §63(c)(5) dependent standard-deduction **floor**, and the
  Form 8615 kiddie-tax screen

★ **Under r1's design this silently UNDERSTATES tax.** A claimable-as-dependent filer whose flag is never
entered — because the whole header was presented as skippable "PII" — receives the **full** standard
deduction. That is the one direction this project's entire refuse architecture exists to prevent, and r1
created it.

**The line is SECRECY, not struct membership:**

| | lives in | why |
|---|---|---|
| **The crown jewels** — SSNs (taxpayer, spouse, dependents), IP PIN | **vault only**, via `set-pii` | enable identity theft and fraudulent filing. Never on disk. |
| **Everything else** — names, address, DOB, blind, claimed-flags, occupation, dependent names/relationships | **the money TOML** | tax-changing and/or needed by the packet. **Must be visible in the template**, where the user is forced to see them. |

`set-pii` therefore prompts for **SSNs and the IP PIN only** — a small, sharp, secret-bearing surface.

### D-7 — The store-gate is ENTRY-PATH-AGNOSTIC, and `income clear` must not eat the secrets
- **No entry path may persist a blob that `screen_inputs` refuses.** D-3 gated only the TOML door;
  `set-pii`'s no-echo prompt is the *likeliest* place a malformed SSN is typed (the user cannot see the
  typo). `set-pii` validates **at the prompt** (`Ssn::canonical`, `IpPin::canonical`) *and* screens the
  **merged** blob before storing.
- **`set-pii` on a year with no stored row REFUSES** (pointing at `income template` / `income import`).
  Creating a default row would put an all-zero, screen-clean `ReturnInputs` at precedence 1 — silently
  flipping a user's report from their stored `tax-profile` to one derived from **zeros**. That is the
  "two liabilities, silently different number" sin `resolve.rs` documents itself against.
- **`income clear` warns and requires confirmation when the header carries secrets.** It is the tool's own
  advertised recovery from an uncomputable year (`resolve.rs:216–224`) — and after Cycle 2 it destroys
  SSNs that exist nowhere else. A `--keep-identity` variant clears the money and preserves the header.

## 6. The drift alarm — THREE assertions, because value-equality has a hole

r1 specced "parse the template, assert it `==` a fixture built without `..Default::default()`."
**That does not bite.** `ReturnInputs` carries **85** `#[serde(default)]` attributes (and a missing
`Option` key parses to `None` even without one), so a template that **omits** a field still parses — and
value-equality passes whenever the fixture's value for that field *happens to equal its default*. Nothing
forced the fixture's values to be non-default.

And my proposed fix — compare key-sets by re-serializing the fixture to TOML — **reopens the same hole**:
`toml::to_string` **silently drops `None`-valued keys**. `serde_json` does not; it renders them as
`null`. So the completeness check must run through a **null-visible representation**.

**Three assertions, and all three are needed:**

- **KAT A — the example is correct.** The uncommented template parses via the *real*
  `parse_return_inputs_toml` and `==` the typed fixture.
- **KAT B — the FIXTURE is complete (mechanical).** Walk `serde_json::to_value(&fixture)` and assert
  **no `null` and no empty array, recursively.** This forces every `Option` to `Some` and every `Vec` to
  carry an exemplar — and it catches a new `Option` field even on a nested struct built with
  `..Default::default()`, because the new field surfaces as `null`. **This is what replaces the
  compiler for the header structs**, which `first_negative_amount` wildcards (§4).
- **KAT C — the TEMPLATE is complete.** Compare the recursive key-**paths** of the fixture's JSON value
  against the key-paths in the raw template parsed as `toml::Value`.

Add a field to `ReturnInputs` ⇒ B or C goes red until the template documents it. **No schema file, no
codegen, no third representation.**

⚠️ **Residual risk, recorded:** a future `#[serde(skip_serializing_if)]` on an input struct would evade B
and C. None exists in `return_inputs.rs` today. Banned there by convention, with a one-line grep KAT.

## 7. Build order (TDD; each step red → green)

0. **`RefusalKind`** — `fn kind(&self) -> RefusalKind` on `RefuseReason`, an exhaustive `match` (§3.5).
   A new variant cannot compile without a classification.
1. **`#[serde(default)]` on `Person.{first_name,last_name,ssn}`** (D-5 rule 1) — a header block must be
   able to omit the secret.
2. **`income template`** + the three drift KATs (§6).
3. **`screen_inputs` at import** — order: parse → carryover-merge → **header-merge (D-5)** → **screen the
   blob AS IT WILL BE STORED** → store. Screening the raw parse would let stored bytes differ from
   screened bytes. Class-aware messages + `--force` with its consequence stated.
4. **`income set-pii`** — no-echo, validates at the prompt, screens the merged blob, **refuses on a year
   with no row** (D-7). The four D-5 merge KATs, both orderings.
5. **`income clear`** — confirm/`--keep-identity` when the header carries secrets (D-7).
6. Man pages (`make docs`); `income show` disposition (§8).

## 8. Follow-ups this phase OWNS (burn down here, per the in-phase rule)

- **`p1-per-field-subcommands`** — disposition. `income template` + `set-pii` IS the answer for v1; the
  per-field editors and the TUI form are deferred with the sizing recorded.
- **`p1-show-as-json-not-toml`** — `income show` emits JSON, so it does not round-trip. With a template
  in hand, **copy-forward is the primary year-over-year workflow**, and it needs `show` to emit TOML.
  Resolve or re-defer with a reason.
- **`p1-ssn-normalization-P6`** — **NOT overdue** (r1 said it was; the ledger records it ✅ DONE in P6.1,
  with the empty-vs-malformed split an accepted declared deviation). The live residue is new P8 scope:
  should **import** canonicalize at capture? D-7 answers it — `set-pii` validates at the prompt, and every
  path screens before storing. Close it, and delete the stale duplicate at `FOLLOWUPS.md:718` so
  reconciliation-by-grep yields one answer.
- **[NEW, pre-existing]** `return_refuse.rs`'s `header: _, // PII only — no money` comment is **false**
  (DOB/blind/claimed-flags are tax-changing) and is what propagated the false dichotomy into r1. Fix the
  comment. And `ScheduleCNoBusinessDescription` sits under the enum's "Compute-dependent" banner but fires
  in `screen_inputs`.

## 9. Acceptance

- A user who has never read the source can run `btctax income template > inputs.toml`, fill it in, import
  it, and get told **at import** — naming the field — if it is wrong.
- **SSNs and the IP PIN** can be entered **without ever writing them to disk** — and a re-import of the
  money TOML **does not destroy them**.
- **No tax-changing fact is hidden behind an optional PII block.** A claimable-as-dependent filer cannot
  silently receive the full standard deduction (D-6).
- No entry path stores a blob the engine has already refused, and no path takes a working year down.
- Adding a field to `ReturnInputs` **breaks the build** until the template documents it.
- `make check` green; 0 Critical / 0 Important from the independent review.
