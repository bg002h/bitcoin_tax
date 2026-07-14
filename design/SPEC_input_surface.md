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

⚠️ **This cycle TOUCHES SHIPPED COMPUTE.** D-8 changes the tax path (`Option<bool>` claimed-flags + a new
refusal + a stored-row migration) to close a live understatement. It is not additive tooling.

**IN — Cycle 1 (discoverability + validation):**
- `btctax income template` — a fully commented TOML skeleton, to stdout. No vault, no PII, offline.
- A **drift alarm**: a KAT that fails the build when a new `ReturnInputs` field is not documented.
- **Import-time screening**: `income import` runs `screen_inputs`.
- Parse errors that name the file and point at `income template`.
- **★ D-8** — the claimed-as-dependent flags (a CRITICAL in shipped code) + `income answer`.

**IN — Cycle 2 (close the plaintext-PII hole):**
- `btctax income set-pii --year N` — interactive, no-echo prompts for the secrets; merges into the vault
  row. Secrets never touch disk. `--clear-ip-pin`.
- `income clear` — confirmation + `--keep-identity` (which leaves the year fail-closed).

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

The input-screenable refusals, classified. (`KiddieTax` is **not** among them — it is compute-dependent, D-4. D-8 adds a new UNANSWERED variant; `DependentSpouseUnsupported` stays SEPARATE, since one `RefuseReason` cannot carry two classes.)

| class | meaning | examples | store? |
|---|---|---|---|
| **INVALID** | the data cannot be true | `SsnMalformed` (non-empty only) · `NegativeAmount` · `SpouseOwnerWithoutJointReturn` · `InconsistentDividendSubset` · `SaltSalesTaxWithoutElection` | **never** |
| **UNANSWERED** | a required question is `None` | `ScheduleBPart3Unanswered` · `MfsSpouseItemizeUnknown` · `ScheduleCNoBusinessDescription` | never (§3.2) |
| **UNSUPPORTED** | the data is **TRUE**; btctax's scope is short | `HsaPresent` · `IraDeductionClaimed` · `AllocatedTips` · `ForeignTrust` · `ExcessElectiveDeferral` · `NonPublicCharityContribution`… | never (§3.2) |

*(`kind()` spans the whole enum, but import can only ever see the input-screenable variants — so its
"nothing was stored" phrasing must never leak into report-time text. `SingleEmployerExcessSs` straddles
INVALID/UNSUPPORTED and is filed as **UNSUPPORTED**.)*

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

### D-2 — ★ The ASK-THE-USER field class. (r2's "every field present" created a CRITICAL.)

r2 said "every field present" and §6's KAT C enforces completeness by parsing the template as
`toml::Value` — **where comments are invisible.** Those two together **force the fail-loud tri-states to
ship uncommented, with values.** A user who skims imports `foreign_accounts = false`, the
`ScheduleBPart3Unanswered` guard **never fires**, and the filed Schedule B Part III prints **"No"**
(`printed.rs:936` — `unwrap_or(false)`): **a foreign-account disclosure answer the user never gave**, with
FBAR-grade stakes.

★ **My completeness machinery structurally revoked the codebase's fail-loud guarantee, for exactly the
fields it was built for. Visibility with a pre-filled answer is a guess wearing documentation's clothes.**

**The ASK-THE-USER class** — fields where btctax must never supply the answer:

| field | why it must not be pre-filled |
|---|---|
| `foreign_accounts`, `foreign_trust` | Schedule B Part III — a disclosure. `unwrap_or(false)` PRINTS the answer. |
| `mfs_spouse_itemizes` | §63(c)(6) couples the spouses' choice |
| `can_be_claimed_as_dependent_{taxpayer,spouse}` | see D-8 — silently understates tax |
| `date_of_birth` | a *recent* dummy suppresses the §63(f) forfeit advisory; an *old* dummy GRANTS the aged add-on (understatement). `Option<Date>` has no refusable-but-parseable placeholder. |
| `ip_pin` | a crown jewel (D-6). Ships as a **commented** `# ip_pin = "..."` doc line — never a live value. |

★ **THE MEMBERSHIP CRITERION** — state the rule, not just the list. A *list* can always have a line added
to it under a red build; a *criterion* is what a reviewer holds an implementer to.

> **A field may be exempt from KAT C's completeness check ONLY IF it satisfies one of:**
> **(a) its absence FAILS LOUD; or**
> **(b) its absence is CONSERVATIVE *and* ADVISED; or**
> **(c) it is a SECRET (D-6) — it must never appear as a live value in a plaintext file.**

**Leg (c) is why `ip_pin` is exempt.** Its absence neither fails loud nor is conservative-and-advised — it
is simply *optional and secret*. Without (c) the criterion would be **false of one of its own members**.
*(r5 reported this leg folded and it was NOT in the artifact — the edit's anchor never matched. Verify
folds land.)*

**Everything NOT exempt ships VISIBLE.** Checked: `presidential_fund_*` (unchecked *is* the true
no-election state; no tax effect), `itemize_election` (= Auto, §63(e) larger-of), `salt_use_sales_tax`
(= false deducts real withholding; forgoing a larger sales-tax deduction **overstates** tax),
`accounting_method` (= Cash is *correct*, not merely conservative — the engine derives Schedule C gross
from ledger income realized on receipt).

⚠️ **`blind` would FAIL leg (b)** — there is no advisory anywhere for an unclaimed blind box (only the
missing-DOB forfeit is advised). Harmless today because `blind` is **not** exempt. Recorded so nobody
exempts it later on a false reading.

**Rules:**
1. Ask-the-user fields ship **COMMENTED** — a `# key = <example>` line, which **is** the doc line (it
   satisfies both "documented" and "never a live secret/answer in the file"). Absent ⇒ the engine's own
   fail-loud fires, which is the whole point.
2. **KAT C carries an EXPLICIT EXEMPTION LIST, asserted inside the KAT** — so the exemption set is itself
   tested and cannot silently grow. A separate raw-text grep KAT requires each exempted field's commented
   doc line to be present, so "commented" never becomes "missing".
3. **Every MONEY placeholder is `"0"`.** KAT B forces each `Vec` to carry an exemplar row, so the template
   ships a W-2, a gift, a carryover. A non-zero example (`amount = "2500"`) left unedited by the same
   skimming user imports as a **phantom deduction** — an understatement no screen can see. `"0"` is inert
   and still demonstrates the quoted-string format. Block headers instruct deleting inapplicable
   `[[…]]` blocks.
4. **No plausible dummy PII.** A valid-looking SSN canonicalizes, screens clean, and **prints on the filed
   1040**. Placeholders must be values the packet **refuses** (empty ⇒ `SsnError::Missing`).

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
4. ★ **STRUCTURE FOLLOWS THE FILE — but dropping a SECRET-BEARING person must be CONFIRMED.**
   Leaf-preservation applies only *within* structure the file supplies: a removed `[header.spouse]` does
   not resurrect the vault's spouse SSN. **But r3 made that silent**, so deleting one block from the
   *money* TOML would permanently destroy a vault-only SSN with no warning — cutting a new hole in the
   very invariant D-7 states. ⇒ Import compares the merged header against the stored one; if the merge
   **drops a person who carries a vault secret**, it **warns, names them, and requires confirmation**
   (or `--force`).
5. ★ **Collections merge by IDENTITY, never by INDEX.** Index-merging silently binds dependent A's vault
   SSN to dependent B after a reorder or an inserted newborn: **a wrong SSN on a filed 1040**, which no
   screen can detect. Merge dependents **by `name`**.
   - **Within-file DUPLICATE names ⇒ refuse.** Genuinely ambiguous.
   - ★ **An UNMATCHED file name is NOT ambiguous — it is a NEW or RENAMED person. Import SUCCEEDS**, that
     dependent's SSN is empty, and the user is told *"no stored SSN for 'John' — run `income set-pii`."*
     r3 refused here, which **trapped the user**: fix a name typo ⇒ no match ⇒ refuse ⇒ nothing stored ⇒
     the vault keeps the typo ⇒ `set-pii` cannot edit names ⇒ the only exit destroys the SSNs. **No exit
     is never an acceptable state.** An empty SSN is already safe: the screen skips it, the packet
     fail-closes at export.

**KATs:** no header · partial header with `ssn = ""` · non-empty file SSN wins · both orderings ·
**spouse block removed (⇒ confirm)** · **dependents reordered / inserted / removed / RENAMED (⇒ succeeds,
empty SSN)** · **within-file duplicate names (⇒ refuse)**.

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
- ★ **ONLY `income import` may CREATE a row.** `set-pii`, **`answer`**, and `clear --keep-identity` all
  **REFUSE** on a missing row (pointing at `income template` / `income import`). Stated as a PRINCIPLE, not
  per-command — r5 wrote the rule for `set-pii` and then added a **new door** (`answer`) that reopened it:
  an all-default row with the claimed-flags answered `Some(false)` is **screen-clean**, so it would land at
  precedence 1 and compute the user's return **from ZEROS**, shadowing their stored `tax-profile`.
  Creating a default row would put an all-zero, screen-clean `ReturnInputs` at precedence 1 — silently
  flipping a user's report from their stored `tax-profile` to one derived from **zeros**. That is the
  "two liabilities, silently different number" sin `resolve.rs` documents itself against.
- **`income clear` warns and requires confirmation when the header carries secrets.** It is the tool's own
  advertised recovery from an uncomputable year (`resolve.rs:216–224`) — and after Cycle 2 it destroys
  SSNs that exist nowhere else.
- ★ **`--keep-identity` must leave the year FAIL-CLOSED.** r2 said it "clears the money and preserves the
  header" — which leaves a **screen-clean, all-zero row at precedence 1**, so the year silently computes
  from **zeros**, shadowing the stored `tax-profile` beneath it. That is verbatim the "two liabilities,
  silently different number" sin this very decision refuses for `set-pii`, **reintroduced in the same
  breath**. The kept row is therefore marked **incomplete** and REFUSES at resolve —
  *"re-import your money TOML (`income template`)"* — until a real import replaces it.
  **Silently-wrong is worse than down.**

### D-8 — ★★ CRITICAL, IN SHIPPED CODE. The claimed-as-dependent flags must become `Option<bool>`.

**This is not a spec defect. It is a live understatement of tax in released software**, surfaced by the
input-surface work and therefore owned here.

`can_be_claimed_as_dependent_taxpayer` is a bare `bool` with `#[serde(default)]`
(`return_inputs.rs:164`). **It silently guesses `false` — "not claimable" — the taxpayer-favourable
direction.** And it gates **two** understatement guards:

1. **§63(c)(5)** — the dependent standard-deduction FLOOR (`return_1040.rs:78`). Guessed `false` ⇒ the
   filer receives the **full** $14,600 basic standard deduction instead of `max($1,300, earned + $450)`.
2. **§1(g) / Form 8615** — the KIDDIE-TAX refusal (`return_1040.rs:618`) is keyed on the **same flag**.
   Guessed `false` ⇒ **the entire screen is disarmed** and the return files at the child's rate.

★ The irony is exact, and it is the tell. The comment guarding the kiddie block reasons with great care
about staying conservative — *"an under-count would let a real kiddie return slip through at the child's
rate (an understatement)"* — **inside an `if` gated on a flag that is silently guessed.** A meticulously
fail-closed guard, wrapped in a guess.

**Who this hits:** a student or young adult with crypto gains, claimed on a parent's return. That is close
to the *archetypal* btctax user. They get the full standard deduction **and** child-rate tax, and their
1040 prints **"Someone can claim: ☐ You as a dependent" UNCHECKED** — an affirmative false statement on a
signed return.

**Nothing backstops it.** No advisory (zero hits in `advisories.rs`). No refusal. The user is never asked.

**And it contradicts the project's own doctrine and its own idiom.** SPEC §3.4: conservative omissions
*"only ever OVERSTATE tax, never understate."* Every other unguessable question is an `Option<bool>` that
**fails loud**: `foreign_accounts`, `foreign_trust`, `mfs_spouse_itemizes`. The 1040 asks this one of
every filer.

**Fix — three parts. The first one is the one I got wrong.**

**(1) ★ THE MIGRATION. "Back-compat is free" was the OPPOSITE of free — it LAUNDERS the bug.**
The store serializes the whole struct (`cli/return_inputs.rs:44`) and there is **no `skip_serializing_if`
anywhere**, so a bare `bool` is **always written**. **Every stored row already on disk carries
`"can_be_claimed_as_dependent_taxpayer": false`** — even though the user was never asked. Migrating
naively turns that into `Some(false)` = **an answered "No"**, so the new refusal **never fires for a
single pre-existing row.** The fix would repair the *future* and **silently ratify the past — for exactly
the population that has the bug.**

⇒ ★ **The marker goes on the TABLE, not on `ReturnInputs`.** *(r4 put it on the struct — which
deserializes from **two** sources: the stored JSON **and the user's TOML**. A user's TOML never carries a
version ⇒ `serde(default)` ⇒ 0 ⇒ their explicitly-typed `false` would be mapped to `None` ⇒ refuse ⇒ **the
primary journey could never complete.** The distinguishing fact is **when the ROW was written** — a
property of storage, not of the user's document.)*

**The DDL.** ⚠️ There is **no idiom to inherit** — grep finds **zero `ALTER TABLE` in the whole repo**, and
SQLite has **no `ADD COLUMN IF NOT EXISTS`**. And `init_table` runs on **every** `get`/`set`/`exists`/… so
a bare ALTER would error `duplicate column name` on **every command after the first**. ⇒ Put the column in
the `CREATE TABLE IF NOT EXISTS` (fresh vaults) **and** guard the ALTER for old ones (`PRAGMA
table_info(return_inputs)`). **KAT: open an old-schema vault twice.**

**★ ONE read boundary, and it must cover BOTH deserializers.** `fn row_to_inputs(json, version) ->
ReturnInputs` applies the fixup, and **every** read calls it — `get` **and `all()`**, which today does a
raw `serde_json::from_str` and would return the **un-migrated `Some(false)`**. The migration's whole
correctness rests on **no reader ever seeing the laundered flag**.

**★ ONE write boundary, and it must stamp BOTH branches.** The existing upsert is `ON CONFLICT(year) DO
UPDATE SET inputs_json=excluded.inputs_json` — **it names one column.** Stamp `schema_version = 1` in the
**DO-UPDATE branch too.** Miss it and a user who answers `false` on a pre-existing row keeps version 0 ⇒
the fixup **re-fires on the very next read** ⇒ **their answer is silently re-laundered to `None` and never
sticks**, with **no error ever shown.** The bug would reconstitute itself out of its own fix.

**KATs:** answer `false` on a version-0 row → reload → still `Some(false)`, row is version 1 ·
`all()` migrates identically to `get()` · old-schema vault opens twice.

★ **Map only `false` ⇒ `None`. NEVER `true`.** A stored `false` is indistinguishable from "never asked" —
that *is* the bug. A stored `true` is not: **nothing defaults to `true`**, so it can only have been typed.
Blanket-mapping both would refuse the one user who got it right and discard the only trustworthy value the
field can hold.

This **takes previously-computing years down — LOUDLY, RECOVERABLY, and in the SAFE direction only.**

**(2) SCOPE the spouse flag.** `None` on the **taxpayer** flag refuses unconditionally (the 1040 asks
every filer). `None` on the **spouse** flag refuses **only when the return has a spouse** (MFJ, or
`header.spouse.is_some()`); otherwise it is inert. r3 refused unconditionally — which, since D-2 ships the
flag commented, would have made **the default journey refuse for every Single/HoH filer**, naming a key
about a spouse who does not exist. The project's own precedent scopes exactly this
(`MfsSpouseItemizeUnknown` fires only on MFS); r3 copied the tri-state without copying the scoping.

**(3) FORBID the re-guess at the consumers.** `return_1040.rs:78` and `:618` must test `== Some(true)`,
**never `unwrap_or(false)`** — which is the shipped idiom at `printed.rs:936` and is *the very shape of
this defect*: a `None` silently becoming a taxpayer-favourable "No". `None` is unreachable past
`screen_inputs`; say so, and do not re-guess it.

**(4) ★ GIVE THEM A WAY TO ANSWER.** The migration's recovery story was *"one TOML line recovers"* — which
assumes the user still has the file. **The spec itself tells them to delete it** (D-3's `--force`
plaintext-hygiene path), `income show` emits masked JSON and cannot regenerate it (§8), and `set-pii`
prompts for secrets only (D-6). So a TOML-less user would face a refusing year with **no in-app path to
answer one boolean** — a wall, landing on people who did exactly what the spec told them to.

⇒ **`btctax income answer --year N`** — interactive, covering precisely the ASK-THE-USER fields (D-2),
which are exactly the fields a TOML-less user can otherwise never reach. Stores through the
screen-before-store gate (D-7), and **refuses on a year with no row** (only `import` creates).

⚠️ **It must prompt the WHOLE class in ONE pass.** D-7 forbids storing a blob `screen_inputs` refuses — so
a *partial* answer (`--taxpayer=no` while `foreign_accounts` is still `None` on a Schedule-B year) **cannot
be stored**: the strictly-improving edit is rejected by the gate, and the user can never answer question 1
until question 2 is answered. **A deadlock.** One pass, whole class.

**Only then is §9's promise true — for the users who already have the bug, not merely for new ones.**

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

- **KAT A — the example is correct.** Take the template, apply a **named, tested transformation** that
  mechanically uncomments the ask-the-user lines to their example answers, parse it via the *real*
  `parse_return_inputs_toml`, and assert `==` the typed fixture. *(r3 said "the uncommented template",
  which was unsatisfiable: the ask-the-user fields are commented and `ip_pin` had no line at all.)*
- **KAT B — the FIXTURE is complete (mechanical).** Walk `serde_json::to_value(&fixture)` and assert
  **no `null` and no empty array, recursively.** This forces every `Option` to `Some` and every `Vec` to
  carry an exemplar — and it catches a new `Option` field even on a nested struct built with
  `..Default::default()`, because the new field surfaces as `null`. **This is what replaces the
  compiler for the header structs**, which `first_negative_amount` wildcards (§4).
- **KAT C — the TEMPLATE is complete.** Compare the recursive key-**paths** of the fixture's JSON value
  against the key-paths in the raw template parsed as `toml::Value`.

Add a field to `ReturnInputs` ⇒ B or C goes red until the template documents it. **No schema file, no
codegen, no third representation.**

⚠️ **Residual risks, recorded:**
- A future `serde(skip…)` on an input struct would evade B and C. None exists in `return_inputs.rs` today.
  Banned by convention; the grep KAT matches **`serde(skip`** generally, not just `skip_serializing_if`.
- **Enum-VARIANT drift is un-alarmed.** A new `CharitableClass`, or a new allowlisted box-12 code, moves no
  key-path — B and C see nothing. Field drift is caught; variant drift is not. Recorded, not solved.

⚠️ **KAT B and KAT C share ONE asserted exemption literal**, and it is governed by D-2's **criterion**, not
by taste. The ask-the-user fields are exempt from key-path completeness *because they must ship commented*.
A **new** `Option` field is not in that set — so B still catches it as a `null`, and the alarm keeps its
teeth. *(r3 left KAT B banning `null` while D-2 forbade `ip_pin` from the template entirely — the three
assertions were mutually unsatisfiable and the spec could not have been implemented.)*

## 7. Build order (TDD; each step red → green)

0. **`RefusalKind`** — `fn kind(&self) -> RefusalKind` on `RefuseReason`, an exhaustive `match` (§3.5).
1. **★ D-8 — the shipped-code Critical.** Largest change; the only one touching shipped compute; goes first.
   a. **The STORE migration** (NOT on `ReturnInputs` — see D-8(1)): the `schema_version` column in
      `CREATE TABLE IF NOT EXISTS` + a **guarded** ALTER for old vaults (`PRAGMA table_info`); one
      `row_to_inputs(json, version)` read boundary called by **`get` AND `all`**; the write stamping
      `schema_version = 1` in **both the INSERT and the DO-UPDATE branch**.
   b. Both claimed-flags → `Option<bool>`; version-0 rows map **`false` ⇒ `None`** (`true` preserved).
   c. The new **UNANSWERED** refusal — taxpayer unconditional; spouse **only when a spouse exists**.
   d. Consumers test `== Some(true)`, **never `unwrap_or(false)`**.
   e. **`income answer`** (whole ask-the-user class, one pass; refuses on a missing row).
2. **`#[serde(default)]` on `Person.{first_name,last_name,ssn}`** (D-5) — a header block must be able to
   OMIT the secret.
3. **`income template`** + the drift KATs (§6): A (uncomment-transform → parse → `== fixture`), B (no
   `null`, no empty array in `serde_json::to_value(&fixture)`), C (key-paths vs the template, sharing B's
   exemption literal), the grep KAT for the commented doc lines, and the `serde(skip` ban.
4. **`screen_inputs` at import** — order: parse → carryover-merge → **header-merge (D-5)** → **screen the
   blob AS IT WILL BE STORED** → store. Class-aware messages; `--force` states its consequence; the
   drop-a-secret-bearing-person confirmation.
5. **`income set-pii`** — no-echo, validates at the prompt, screens the merged blob, **refuses on a year
   with no row** (D-7), `--clear-ip-pin`. All the D-5 merge KATs, both orderings.
6. **`income clear`** — confirm when the header carries secrets; `--keep-identity` leaves the year
   **fail-closed** (D-7).
7. `export-irs-pdf`'s `SsnError::Missing` refusal names `income set-pii`. Man pages (`make docs`).

## 8. Follow-ups this phase OWNS (burn down here, per the in-phase rule)

- **`p1-per-field-subcommands`** — **PARTLY DELIVERED, not merely deferred.** This cycle ships
  `income answer` (a per-field editor for the ask-the-user class) and `set-pii` (for the secrets). What
  remains deferred is a general per-field editor for the money surface, and the TUI form — with the sizing
  recorded (no form engine; ~10 new flows).
- **`p1-show-as-json-not-toml`** — **DECIDED, not punted.** `income show` keeps emitting masked JSON; a
  TOML round-trip is **deferred**. ⚠️ If it ever ships, its secrets must emit as **empty strings, not
  masks**: today's `***-**-6789` is *non-empty*, so D-5's "non-empty file value wins" would store the mask
  and `SsnMalformed` would refuse every re-import — copy-forward would poison the year it exists to serve.
- **`export-irs-pdf`'s `SsnError::Missing` refusal must name `income set-pii`.** It is the last wall on the
  default path, and the first place a user learns identity is needed.
- **`income set-pii --clear-ip-pin`** — an IP PIN entered in error is otherwise inescapable: `set-pii`
  validates every prompt (`IpPin::canonical`: empty ⇒ `Missing`), so re-prompting cannot clear it, and the
  only exits destroy or keep it.
- **`p1-ssn-normalization-P6`** — **NOT overdue** (r1 said it was; the ledger records it ✅ DONE in P6.1,
  with the empty-vs-malformed split an accepted declared deviation). The live residue is new P8 scope:
  should **import** canonicalize at capture? D-7 answers it — `set-pii` validates at the prompt, and every
  path screens before storing. Close it, and delete the stale duplicate at `FOLLOWUPS.md:718` so
  reconciliation-by-grep yields one answer.
- **[NEW, pre-existing] `accounting_method = "accrual"` is UNREFUSED and UNMODELED.** Cash is *correct*,
  not merely conservative — the engine derives Schedule C gross from ledger income realized on receipt
  (cash-basis by construction), and Cash prints truthfully on line F. But **Accrual flips the printed
  line-F checkbox on a return whose income was computed cash-basis** — a facial misstatement, with no
  screen (`return_refuse.rs:373` wildcards the field). Today nobody can find the field. **After
  `income template`, every user will see it.** Refuse Accrual as UNSUPPORTED, or document the field as
  cash-only. **Do this IN this cycle** — the template is what makes it reachable.
- **[NEW, pre-existing] `Dependent.relationship`** defaults `""` and **prints BLANK on the filed 1040**
  (`form1040_full.rs:376`) with no screen — the same facial-incompleteness class as
  `ScheduleCNoBusinessDescription`, which the code *does* refuse. Fix here or file it.
- **[NIT, pre-existing] `ssn_valid_for_employment` is entirely unconsumed** (zero references outside its
  definition). KAT C would force it into the template, asking the user a question that changes nothing.
  Wire it or mark it reserved.
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
- **No path takes a year down SILENTLY, and no path takes it down in the UNDERSTATING direction.** ★ The
  D-8 migration DOES take previously-computing years down — **loudly, recoverably (`income answer`), and in
  the safe direction only, by design.** *(r4's bullet said "no path takes a working year down", which
  contradicted D-8 verbatim — and an implementer holding that line would have resolved it the cheap way, by
  softening the migration back into the `Some(false)` laundering it exists to kill.)*
- **The fix reaches the rows that ALREADY have the bug**, not merely new ones.
- Adding a field to `ReturnInputs` **breaks the build** until the template documents it.
- `make check` green; 0 Critical / 0 Important from the independent review.
