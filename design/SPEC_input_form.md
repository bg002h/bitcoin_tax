# SPEC — the INPUT FORM (a swappable form UI for `ReturnInputs`)

*Status: **DRAFT r1** — validated in brainstorm and through **two independent Fable architect passes**
(verdict: "READY TO SPEC, no architectural blocker"). Not yet independently spec-reviewed to 0C/0I.*
*Provenance: brainstorm 2026-07-14; supersedes the deferred "guided full-return TUI form" follow-up in
`SPEC_input_surface.md` (P8) §8. Depends on P9 (`SPEC_form_questions.md`, shipped) for `FORM_QUESTIONS`,
the skippable registry, and `screen_inputs`.*

---

## 1. The problem

btctax fills a complete, filable 1040 packet, but **a human still cannot drive the non-crypto side.** The
only way to enter W-2s, interest/dividends, Schedule A, dependents, and PII is to hand-edit a fiddly
`inputs.toml` (`btctax income import`). P8's recon concluded the *authoring* fix was `income template` +
docs; the owner has now decided that **editing TOML in a text editor is not good enough** — we need an
**interactive interface** for creating and editing that data.

Constraints, fixed with the owner (2026-07-14):

- **Target user: technical (terminal) first, but a web app must be able to replace the TUI later.** The
  presentation layer must be swappable; the core must be UI-agnostic.
- **Storage is unchanged.** Validated `ReturnInputs` already persists as a JSON blob per tax year in the
  encrypted vault's SQLite `return_inputs` table, schema-versioned (P9 hardened this). Keep it. TOML drops
  to optional import/export. The form writes the whole blob via the existing `return_inputs::set`.
- **Scope v1 is a common subset** (header + PII, W-2s, Schedule A, dependents, declarations); the engine
  must GENERALIZE to the rest without rework.

## 2. Scope

**IN (v1):** the 1040 header + PII (SSN, IP PIN), W-2s (**including `box12`** — it carries the §402(g)
excess-deferral and unsupported-code refusals; cutting it is an under-ask), Schedule A (including
`charitable`), dependents, the eight class-(A) **declarations** and the class-(B) **skippables**
(blindness ×2, the §164(b)(5) sales-tax election, DOBs ×2).

**DEFERRED to TOML import (v1):** Schedule C, QBI, capital-loss / charitable / QBI carryforwards, and
1099-INT/DIV/G. The §5 tree grammar already expresses these — deferral is "fewer `FieldSpec`s," not
"a different engine." The coverage KAT (§5.6) exempts the deferred structs *explicitly*, so drift in an
in-scope struct still breaks the build.

**OUT (not this spec):** any change to the compute engine or the frozen files
(`tax/{types,compute,se}.rs`); a web front-end (but the seam that enables it is day-one — §4); collecting
all screen refusals at once (`screen_inputs_all`, §6, a later item).

⚠️ **This spec does not touch shipped compute.** It is an authoring/persistence layer over unchanged types
and the unchanged `screen_inputs` gate.

## 3. Architecture

Three layers, with the **swap boundary in the middle** so the TUI and a future web app are two renderers
of one core:

```
┌─ btctax-tui-edit ───────────────┐        ┌─ (future) web front-end ─┐
│  new "tax inputs" mode:         │        │  serves the render model  │
│  renders FormSpec, handles keys │        │  as JSON, POSTs Edits     │
└──────────────┬──────────────────┘        └────────────┬─────────────┘
               │        both consume the SAME data seam:  │
               │   FormSpec (render model) + Edit stream    │
               └──────────────► btctax-form ◄──────────────┘
┌─ btctax-form (NEW crate; depends on core ONLY; vault-free; unit-testable) ─┐
│  • FormSpec: a tree of Sections → Fields (stable ids, kind, help, live,     │
│    accessors)                                                               │
│  • apply(&mut ReturnInputs, Edit) -> Result<(), ApplyError>                 │
│  • validate(&ReturnInputs) -> field/section errors  (attributes screen_inputs)│
│  • the RefuseReason -> FieldId/SectionId attribution map                    │
│  • adapts the two CORE registries (declarations + skippables) into sections │
└──────────────┬─────────────────────────────────────────────────────────────┘
               │ reuses, unchanged:
   FORM_QUESTIONS + SKIPPABLE_QUESTIONS (core registries) · screen_inputs (gate) · ReturnInputs (type)

┌─ btctax-cli :: input_form_store (NEW module beside return_inputs.rs) ───────┐
│  • load(year) · save_draft(year, &RI) · commit(year, &RI) · toggle(...)     │
│  • the `return_inputs_draft` table (mirrors return_inputs, discard-on-stale)│
│  operates on ReturnInputs + screen_inputs (core); need NOT depend on btctax-form │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Crate graph (Fable-endorsed, no cycle):** `btctax-form → btctax-core`. `btctax-tui-edit → btctax-form`
(engine) **and** `btctax-cli` (persistence). The `input_form_store` persistence module lives in
`btctax-cli` because `return_inputs::get/set` needs `Session`/`CliError` and core has no vault access; it
depends only on core types, not on `btctax-form`. **Drafts store `ReturnInputs` JSON blobs, not `Edit`
logs** — so no `btctax-form` type ever enters the vault, and `Edit` stays a transient UI→engine message.

**The load-bearing rule:** the TUI never names a `ReturnInputs` field. It asks the `FormSpec` "what
sections/fields exist, what kind, current value, is it live, what's the help" and emits `Edit`s. That is
the identical trick `FORM_QUESTIONS` already uses for declarations (`fn(&ReturnInputs)` accessors),
generalized. A web UI later asks the same questions over the wire.

## 4. The data seam (why a web UI drops in for free)

The seam is **data, not closures.** The closures (accessors) never cross a wire; what a web front-end
serializes is a *render model* and an *edit stream*:

- **Stable `FieldId` / `SectionId`** — enums (or stable snake_case strings), **never "index into a Vec"**.
  IDs are the wire contract; indexes drift per build.
- **`Edit`** — a `serde`-serializable enum:
  `SetField { id, addr, value }`, `ClearField { id, addr }`, `AddRow { section }`,
  `RemoveRow { section, addr }`, `CreateSection { section }`, `DeleteSection { section }`.
  `RowAddr` is a short index path (≤ 2 today: e.g. `[w2_index, box12_index]`).
- **`FieldValue`** — a union mirroring `FieldKind`: `Money(Usd)`, `Text(String)`, `TriState(Option<bool>)`,
  `Date(Option<Date>)`, `Choice(&'static str)`, `Secret(SecretView)`.
- **Per-field `live: fn(&ReturnInputs) -> bool`**, re-evaluated after **every** `apply`. Liveness *is* the
  P9 program — without it the renderer hard-codes visibility and un-swaps the layer, and a `filing_status`
  edit that changes which fields exist would not propagate.
- **Accessors are monomorphic** `fn(&ReturnInputs, RowAddr) -> Option<FieldValue>` and
  `fn(&mut ReturnInputs, RowAddr, FieldValue) -> Result<(), SetError>`. The **row type never appears in the
  interface** (internally `ri.w2s.get(addr[0])?.box12.get(addr[1])?…`). A kind mismatch is a `SetError`,
  and one engine test that round-trips every field kills that class.
- **Secrets are write-only through the seam.** `FieldValue::Secret` is **get/set-asymmetric**: `get`
  returns a `SecretView` presence flag (`Set(masked: "***-**-1234")` / `Empty`), **never digits**; `set`
  takes the value. No accessor may return secret digits, and no renderer may echo them. (Mirrors the
  existing `mask_ssn` / `IpPin(******)` Debug discipline.)

⚠️ **Vetoed encodings (Fable):** no `FormRow` trait (needs object safety, reinvents `FieldValue` anyway);
no `serde_json::Value` path reflection (**it reintroduces the stringly-typed null-vs-absent laundering P9
exists to abolish** — this codebase's one named architectural sin, walking back in through the form); no
derive macro (that is the rejected Approach C — the human part of a field is the *metadata*, which no
derive can produce; see §7).

## 5. The FormSpec model

### 5.1 A tree of depth ≤ 3, three node kinds

The flat "singleton vs repeating" split fails *inside v1 scope* (`box12` in a W-2 row; optional Schedule A
with its own `charitable` list; optional spouse). The grammar is a tree:

- **Singleton** — fields over one struct. *(Header identity, Payments.)*
- **Optional-singleton** — an `Option<T>` with **`CreateSection` / `DeleteSection`** edits.
  *(Schedule A — delete ⇒ standard deduction; its existence feeds the mortgage declaration's liveness.
  Spouse `Option<Person>` — presence couples to `filing_status` liveness.)*
- **Repeating** — a `Vec<T>` with **`AddRow` / `RemoveRow`**. *(W-2s, dependents, and `box12` **inside** a
  W-2 row.)*

`RowAddr` is the path of indices to a row (≤ 2 today). This grammar also covers every deferred section
(Schedule C = optional-singleton; 1099s = repeating; carryforwards = flat singletons + a repeating
`charitable_carryover_in`), so nothing deferred needs a new node kind.

### 5.2 A Field

`{ id: FieldId, label, help: &'static str, kind: FieldKind, live: fn(&RI)->bool, get, set }`. `label`/`help`
are **single-sourced** — the help line *is* the doc (the P8 "comment = doc" idea), and it is phrased as the
form phrases it. `FieldKind ∈ { Money(Usd, ≥0), Text, TriState(Option<bool>), Date(Option<Date>),
Enum(&[&str]), Secret }`.

### 5.3 The two CORE registries — declarations and skippables, kept SEPARATE

- The **Declarations section is generated from `FORM_QUESTIONS`** (P9) — each entry already carries its
  prompt, help, `live`, `get`, `set`. Zero duplication; P9's completeness guarantees carry over.
- The class-(B) **skippables** (blind ×2, SALT election, DOBs ×2) **move out of `cmd/answer.rs` into a new
  `core::tax::questions::SKIPPABLE_QUESTIONS`** const (same fn-pointer shape, with `live` predicates and a
  `skippable: true` semantic — silence leaves `None` and the advisory fires). This **deletes the CLI-side
  second copy** of skippable liveness (`answer.rs::live_questions:158–170`'s hard-coded `spouse.is_some()`
  / `schedule_a.is_some()` gates) — the "ONLY copy in the codebase" discipline P9 built.

> ### ★ HARD RULE (Fable): the skippables are a SEPARATE registry from `FORM_QUESTIONS`, never merged.
> `screen_inputs` loops `FORM_QUESTIONS` and **refuses on any *live* entry that is `None`** — but a
> skippable is `None`-legal by design (blindness unanswered is lawful; the advisory fires). Merging the two
> would make `screen_inputs` **refuse on an unanswered blindness box — a brick.** The `Ask::{Declaration,
> Skippable}` split in `answer.rs` survives as a two-registry split in core.

Placement note: the **registries (fn-pointer data) live in CORE**; `btctax-form` *adapts* both into
`FormSpec` sections; `income answer` **consumes the core registries directly** (as it already consumes
`FORM_QUESTIONS`). Nothing `FormSpec`-shaped enters core. `income answer` becomes a **second renderer** over
the same fields — it stays valuable (scriptable; the documented recovery remedy named in shipped refusal
texts; needs no full-screen terminal). Two interactive *paths* are fine; two *registries* are not.

Test relocation: `answer.rs`'s skippable unit tests (`income_answer_asks_the_class_b_skippables_when_live`,
`only_the_skippables_are_skippable`, the `current_bool`/`set_bool` round-trips) move to core call-sites;
behavior preserved. **`every_live_question_can_actually_be_answered_and_clears_the_screen` (the no-brick
property) is sacred — preserve it wherever it lands.**

### 5.4 TriState never displays `None` as "No"

The renderer contract (inherited by TUI and web) states that `TriState` renders as a **three-way** control
(never asked / yes / no) and **no renderer may default-display `None` as "No"** — the OpenTaxSolver bug
(see `[[ots-is-not-a-model-for-answeredness]]`). This sentence lives in the FormSpec doc so both renderers
inherit it.

### 5.5 Secrets (v1 effectively ships the deferred `set-pii`)

`Secret` fields (SSN ×N, IP PIN) inherit `SPEC_input_surface.md` D-6's obligations verbatim: **masked
display** (`***-**-1234`), **no-echo / masked input** in the TUI (the `UnlockState` passphrase discipline
is the precedent), **write-only over any future wire**, and **prompt-time `Ssn::canonical` /
`IpPin::canonical` validation**. Because the vault is encrypted, storing secrets in `ReturnInputs` (and in
the draft — §8) is **no new plaintext-on-disk exposure**: D-6's "never on disk" is about the plaintext
*TOML*, which the form replaces. (One preempting sentence so a reviewer does not flag a secrets
regression.)

### 5.6 Drift-proofing: the coverage KAT (not a macro)

A KAT (the P8 §6 KAT-C technique, re-aimed) walks the `serde_json` leaf key-paths of the **in-scope**
structs and asserts every leaf maps to **exactly one** `Field` **or an explicit exemption**. Add a field to
`W2` ⇒ the KAT goes red until the form covers or exempts it. The **exemption list is asserted *inside* the
KAT** (P8 KAT-C discipline) so an in-scope struct's new field still bites. This is the house style
(`first_negative_amount`'s no-`..` destructures; `QuestionId::ALL`; the P9 classifier) and it is what makes
the hand-written accessors safe without a macro.

## 6. Data flow, and the draft table

```
load(year)   → working ReturnInputs:  draft if present, else the committed row, else EMPTY
apply(Edit)  → mutate working copy → re-eval liveness → field-parse (tier 1, §7)
save_draft   → return_inputs_draft table — ANYTIME, incl. mid-invalid; resolve.rs NEVER reads it
commit       → screen_inputs(working) → payload-confirm modal → return_inputs::set → DELETE draft
```

### 6.1 Why a draft table (the blocking reason)

"Save the whole blob live via `return_inputs::set`" **would brick the year on every pause.** `resolve.rs`
treats a stored `ReturnInputs` row as top precedence, and if `screen_inputs` refuses it, the resolver
returns `profile: None` and **never falls through** to the crypto-only report or the `tax-profile` escape
hatch (`resolve.rs:85–109`, verified). A form is *necessarily* refused mid-entry (fresh year = 8 `None`
declarations; half-typed W-2 = missing employer). So:

- **`return_inputs_draft`** — a sibling table in the **same encrypted vault**, one row per year
  (`year PRIMARY KEY`), mirroring `return_inputs.rs` (~100 lines by the existing pattern). **`resolve.rs`
  never reads it** ⇒ every P8/`resolve.rs` invariant holds by construction. Type-invalid text never enters
  the working `ReturnInputs` (raw buffers that do not parse are held in the renderer), so a draft is always
  type-valid, possibly screen-refused — exactly the right laxity.
- **`commit(year, &RI)`** (the `btctax-cli` store fn) runs `screen_inputs` (the real gate); if refused it
  **returns the `Refusal` and writes nothing**; if clean it `return_inputs::set`s and deletes the draft.
  The **payload-showing confirm** (editor house style: "replaces the stored 2024 row; 2 W-2s, Schedule A, 1
  dependent…") is the **caller's** (the TUI's) responsibility *before* calling `commit`; a returned refusal
  is surfaced via the §7 attribution map and the working copy stays uncommitted.

### 6.2 ★ Draft-vs-committed COHERENCE across sessions (Fable's near-blocking item)

A draft **persists across form sessions** (its crash-recovery purpose), and the other writers of the
committed row are ignorant of it: `income import` (`tax.rs:98`), `income answer` (`answer.rs:309`),
`report --write-carryover` (`tax.rs:461`), `income clear` (delete), `set-pii` (secret merge). The
`VaultLock` serializes them against the form (no concurrent access), but a *stale* draft is a silent-loss
hazard: edit → close → `income import` (writes the row, draft untouched) → reopen form → `load` prefers the
**stale draft**, hides the import, and committing clobbers it.

> ### RULE: an authoritative committed-row write CLEARS that year's draft.
> `return_inputs::set` / `delete` from **import, `write-carryover`, `income clear`, and `set-pii`** also
> delete that year's `return_inputs_draft` row (warn if discarding a non-trivial draft). A fresh
> committed write supersedes stale WIP. **The toggle's park/commit (§9) is the one exception** — it manages
> the draft explicitly.

### 6.3 Draft stale-version = DISCARD, not refuse

The committed row refuses on `schema_version` mismatch (`StaleReturnInputs`, refuse-and-reimport — it may
hold irreplaceable carryover). **A draft is regenerable WIP: a stale-version draft is silently DISCARDED
(discarded-with-note), never a hard refuse** — refusing would brick a resume for no benefit. This is the
one deliberate divergence from the mirrored `return_inputs` table.

## 7. Validation — three tiers

1. **Field parse — a shared `btctax-form` helper `parse(kind, &str) -> Result<FieldValue, ParseError>`,
   driven by the renderer's raw text buffer (new but thin — parsers, not tax law).** Money = `Decimal ≥ 0`
   (which makes `NegativeAmount` **unreachable from the form**), `Date` format, SSN via `Ssn::canonical`,
   IP PIN via `IpPin::canonical`. Live as you type; text that does not parse stays in the renderer's buffer
   and never enters the working `ReturnInputs`. Reuses the existing canonical validators — it does not
   restate rules, and both renderers share the one parse helper.
2. **`screen_inputs` — UNCHANGED — the commit gate.** Run on section-exit / commit attempt against the
   working copy. **First-refusal display for v1** (the compiler-with-one-error model; fix-one-see-next is
   fast in a live form). **Do NOT refactor `screen_inputs` to collect all refusals** — its early-return
   tiers are semantic (a later rule assumes earlier integrity; §402(g) accumulation after an un-refused
   negative would show garbage). A future `screen_inputs_all` (tiered collection; integrity refusals
   suppress downstream tiers; `screen_inputs` delegates to `.first()`) is a **P-later** item, not v1.
3. **Attribution — `btctax-form` — an EXHAUSTIVE `match RefuseReason -> Vec<FieldId | SectionId>`.**
   Exhaustive so a new `RefuseReason` variant is a **compile error** until attributed. **Never parse the
   prose `detail` strings** (the labels in `NegativeAmount(String)` are display prose, not identities). Some
   refusals attribute to a *section* or a *pair* of fields (`SaltSalesTaxWithoutElection` → two Schedule A
   fields; `ExcessElectiveDeferral` → the W-2 section). For the eight declarations, attribution is exact via
   `RefuseReason ↔ QuestionId`.

**Honesty carry-over (D-4):** the form's "screens clean" message must **name what it cannot see** — the
compute-dependent (`ScheduleCLoss`, `KiddieTax`) and absolute (`QbiAboveThreshold`, AMT) screens still run
at `report`/`export`.

## 8. PII / secrets in the draft

Covered in §5.5: the draft stores a full `ReturnInputs` JSON including SSNs / IP PIN, but it is inside the
**encrypted** vault (same posture as `return_inputs`, which already stores SSNs), so there is **no new
plaintext exposure**. The `FieldValue::Secret` get/set asymmetry (§4) guarantees no accessor or render model
ever carries digits.

## 9. Create-row + the tax-profile toggle (owner-approved)

**Shadowing is precedence, not deletion** (`resolve.rs:85` early-returns before ever reading `tax_profile`;
the two live in physically separate tables). So a form-commit that creates a `ReturnInputs` row makes the
full return the *active source* while leaving the `tax_profile` **saved and unused**, and toggling is just
the presence/absence of the RI row.

- **Amendment to "only `income import` creates a row."** A **screened** form-commit becomes a second lawful
  creation door. When a `tax_profile` exists for that year, commit **warns by name and requires
  confirmation**: *"this makes the full return the active source for {year}, computed from the numbers you
  entered; your tax-profile estimate stays saved and unused."* ⚠️ Name the **all-zero** consequence: a filer
  who answers the eight declarations and enters no income commits a *screen-clean* zero return (proven by
  `every_live_question_can_actually_be_answered_and_clears_the_screen`) that shadows the profile and computes
  ≈ $0 — the one-key toggle-back makes this recoverable, but the confirm must not hide it.
- **The form always shows which source is active** for the year.
- **One-key NON-DESTRUCTIVE toggle:**
  - *"use tax-profile"* — **stash the committed row into its draft, THEN delete the committed row**, via
    **in-session `return_inputs::delete` on the held `conn`** (⚠️ **NOT** the CLI `income clear` command —
    that re-opens `Session` and self-deadlocks on the exclusive `VaultLock`, `session.rs:389`). The
    `tax_profile` resumes automatically via precedence, untouched.
  - *"use full return"* — re-commit the stashed draft.
  - **Stash-before-clear is atomic:** the delete is conditional on a **confirmed successful stash** within
    one session — a failed stash must never delete the row, because those SSNs (D-6) exist nowhere else.
  - **Offer "use tax-profile" only from a clean/committed state** (no WIP divergent from the committed
    row), so the one-row-per-year draft slot cannot clobber unsaved edits. Once parked, the parked blob
    *becomes* the WIP and re-commits on toggle-back — one table suffices, and the collision moment is gated
    away.
- **The `tax_profile` is NEVER auto-deleted.** It is the fallback.

## 10. Testing (KATs)

**Engine (`btctax-form`, no terminal):**
- **Field round-trip:** every `Field` `get`→`FieldValue`→`set` round-trips; kind mismatch is a `SetError`
  (kills the type-mismatch class in one iterating test).
- **`apply` + liveness:** each `Edit` mutates the working copy and liveness is re-evaluated (a
  `filing_status` edit changes the live set).
- **Tree edits:** `AddRow`/`RemoveRow` (incl. `box12` at depth 2), `CreateSection`/`DeleteSection` (Schedule
  A, spouse).
- **Exhaustive attribution:** every `RefuseReason` maps to a field/section (compile-forced) — and a
  representative refusal per section attributes to the right place.
- **Coverage KAT (§5.6):** in-scope leaf paths ⇔ fields, exemptions asserted inside the KAT.
- **TriState:** no code path renders `None` as "No"; `Secret` `get` never returns digits.

**Persistence (`btctax-cli`):**
- **Draft is invisible to `resolve.rs`:** a screen-refused draft never poisons the year (the D-3/D-7
  property, re-pinned for the new table).
- **`commit`** screens → writes → deletes the draft; a screen-refused working copy does **not** commit.
- **Coherence rule (§6.2):** `income import` / `write-carryover` / `income clear` / `set-pii` clear that
  year's draft.
- **Draft stale-version DISCARDS** (does not refuse).
- **Toggle** is non-destructive (park → clear → profile resumes; re-commit → RI wins), stash-before-clear
  atomic, gated to a clean state.
- **★ form-commit preserves `Computed` carryovers** (the working copy starts from `get`, so it carries
  them — *better* than import's special-case merge; assert it).

**TUI:** snapshot tests in the existing `btctax-tui-edit` style; the renderer never names a `ReturnInputs`
field.

## 11. Build order (phased; each phase TDD, mutation-checked per the workflow)

1. **`btctax-form` crate skeleton + the seam types** — `FieldId`/`SectionId`, `Edit`, `FieldValue`,
   `FieldKind`, `RowAddr`, the `Field`/`Section` tree types. No accessors yet.
2. **The declarations + skippables sections** — move `SKIPPABLE_QUESTIONS` into core (keep it a separate
   registry; relocate the `answer.rs` tests; preserve the no-brick property), adapt both core registries
   into `FormSpec` sections. `income answer` re-consumes the core skippable registry.
3. **The v1 section tree + accessors** — header/PII, W-2s (incl. `box12`), Schedule A (incl. `charitable`),
   dependents. The coverage KAT goes red→green here.
4. **`apply` + per-field parse validation (tier 1)** + liveness re-eval.
5. **The `RefuseReason → FieldId/SectionId` attribution map (tier 3)** + the exhaustive match.
6. **`input_form_store` in `btctax-cli`** — the `return_inputs_draft` table (discard-on-stale), `load` /
   `save_draft` / `commit` (screen→confirm→set→delete-draft), and the §6.2 coherence rule wired into the
   existing committed-row writers.
7. **The toggle** — in-session stash/clear/re-commit, atomicity, clean-state gate, active-source state.
8. **The TUI "tax inputs" mode** — the renderer over `FormSpec`, key handling, the payload-confirm modal,
   secret no-echo input, snapshot tests.
9. **Docs** — man pages; `income template`/`income import` remain as import/export; `LIMITATIONS.md` note
   that the form is the primary authoring path and what it cannot see at entry.

## 12. Follow-ups this phase files (non-gating)

- **`screen_inputs_all`** — tiered all-refusals collection for a future "show every problem" form mode;
  `screen_inputs` delegates to `.first()`.
- **Parked-year visibility** — a year with data only in the draft (profile active) is invisible to
  `income show` / `return_inputs::years()`. The TUI year picker should union committed + draft years;
  `income show` should note a parked draft exists.
- **The deferred sections** — Schedule C, QBI, carryforwards, 1099-INT/DIV/G as additional `FieldSpec`s
  (the tree already expresses them).
- **The web front-end** — a second renderer over the same `FormSpec` render model + `Edit` stream (the
  serializable seam is day-one; the renderer is later).

## 13. Acceptance

- A technical user creates and edits a full v1-subset return **without hand-editing TOML**, with live
  per-field validation, and commits only a `screen_inputs`-clean blob.
- **No mid-entry save ever poisons the year** (drafts are invisible to `resolve.rs`).
- **The seam is data** — `FieldId`s + a serde `Edit` enum + `FieldValue` + per-field `live` — so a web
  renderer needs no core/engine change.
- **One registry per concept** — declarations from `FORM_QUESTIONS`, skippables from `SKIPPABLE_QUESTIONS`
  (separate), `income answer` and the form are two renderers of the same core; no third copy of any
  accessor/liveness.
- **The coverage KAT** makes a new in-scope field break the build until the form covers it.
- **Secrets** never reach plaintext disk and never surface digits through the seam.
- **The tax-profile toggle** is non-destructive and reversible; the create-row amendment warns on shadow.
- FROZEN (`tax/{types,compute,se}.rs`) unchanged; `screen_inputs` unchanged; `resolve.rs` precedence
  unchanged.
- `make check` green; independent review to **0 Critical / 0 Important**.
