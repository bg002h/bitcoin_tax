# SPEC — the INPUT FORM (a swappable form UI for `ReturnInputs`)

*Status: **DRAFT r3** — r2 (impl-ready detail) passed an **independent Fable spec review**
(`design/input-form/reviews/SPEC-input-form-fable-r1.md`: 1C/11I/7M/4Nit), and r3 FOLDS all of it: the
Critical (C-1 — a `parked` flag so a parked return gets committed-row protection, not WIP-grade deletion)
and every Important (I-1 `ip_pin`; I-2 secret-entry asymmetry; I-3/I-4 attribution fixes; I-5 shared
`ScheduleBPart3Unanswered`; I-6 `income answer` in the coherence rule; I-7 autosave must `Vault::save`; I-8
`FieldKind::Bool`; I-9 no laundered `filing_status`; I-10 `DeleteSection(ScheduleA)` resets
`itemize_election`; I-11 TY2024-only commit gate) plus the Minors/Nits (M-1 Payments into v1; M-2/M-3 SALT &
deferred-leg anchors; M-4 `NonCryptoNoncashGift` honesty; M-5 owned `Edit`; M-6 `ClearField`; M-7 crate
renamed `btctax-input-form`; N-1..4). r1 validated in brainstorm + two Fable architect passes.*
*★ **r4** — the r3 fold was re-reviewed (0C/**2I**/4M: the fold's own new seams) and r4 folds those too:
NI-1 (a parked draft keeps `parked=1` through edits — `save_draft` preserves it, `Loaded::Draft{parked}`
round-trips it); NI-2 (the working model is `Working = Option<ReturnInputs>`, so "filing status chosen" ≡
"the RI exists" — held by construction, not a renderer bool; both renderers inherit it); M-a `FieldKind::Bool`
in §5.2; M-b Payments in the §9A order; M-c `NonCryptoNoncashGift` anchored; M-d the discard-parked path named.*
*★ **r5 — GREEN.** The r4 fold was re-reviewed **0C/0I** (`SPEC-input-form-fable-r3.md`; the `Working` model
change probed seam-free, the `RefuseReason` match confirmed 37/37); r5 folds its 4 residual non-gating
Minors (M-1 §3 box synced; M-2 the discard-parked key in §9A; M-3 the NI-2 materialization guard test; M-4
§10 wording). **Spec is green — ready for an implementation plan.***
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
`charitable`), dependents, **`Payments`** (estimated/extension/other-withholding — M-1), and the eight
class-(A) **declarations** and the class-(B) **skippables** (blindness ×2, the §164(b)(5) sales-tax
election, DOBs ×2).

**DEFERRED to TOML import (v1):** Schedule C, QBI, capital-loss / charitable / QBI carryforwards,
1099-INT/DIV/G, and the Schedule-1 money leaves (state refund, student-loan interest, IRA deduction). The §5 tree grammar already expresses these — deferral is "fewer `FieldSpec`s," not
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
               └──────────────► btctax-input-form ◄──────────────┘
┌─ btctax-input-form (NEW crate; depends on core ONLY; vault-free; unit-testable) ─┐
│  • FormSpec: a tree of Sections → Fields (stable ids, kind, help, live,     │
│    accessors)                                                               │
│  • apply(&mut Working, Edit) -> Result<(), ApplyError>   (Working = Option<RI>)│
│  • attribute(&RefuseReason) -> Vec<Anchor>   (the exhaustive 37/37 map)      │
│  • adapts the two CORE registries (declarations + skippables) into sections │
└──────────────┬─────────────────────────────────────────────────────────────┘
               │ reuses, unchanged:
   FORM_QUESTIONS + SKIPPABLE_QUESTIONS (core registries) · screen_inputs (gate) · ReturnInputs (type)

┌─ btctax-cli :: input_form_store (NEW module beside return_inputs.rs) ───────┐
│  • load(sess,year) · save_draft(sess,year,&RI) · commit(sess,year,&RI,table,params) · park(sess,year)│
│  • the `return_inputs_draft` table (mirrors return_inputs + `parked` flag)   │
│  operates on ReturnInputs + screen_inputs (core); need NOT depend on btctax-input-form │
└─────────────────────────────────────────────────────────────────────────────┘
(§5.7 is the canonical signature list; this box is the overview.)
```

*(★ M-7 — the crate is `btctax-input-form`, NOT `btctax-form`: the latter is one letter from the existing,
published `btctax-forms` crate (the IRS-PDF filler, already a `btctax-cli` dependency) — a self-typo-squat to
avoid.)*

**Crate graph (Fable-endorsed, no cycle):** `btctax-input-form → btctax-core`. `btctax-tui-edit → btctax-input-form`
(engine) **and** `btctax-cli` (persistence). The `input_form_store` persistence module lives in
`btctax-cli` because `return_inputs::get/set` needs `Session`/`CliError` and core has no vault access; it
depends only on core types, not on `btctax-input-form`. **Drafts store `ReturnInputs` JSON blobs, not `Edit`
logs** — so no `btctax-input-form` type ever enters the vault, and `Edit` stays a transient UI→engine message.

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
  `Date(Option<Date>)`, `Bool(bool)`, `Choice(String)` (owned — serde, M-5), plus the secret pair
  `Secret(SecretView)` (out) / `SecretEntry(String)` (in — I-2).
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

**★ Non-live set values (I-10).** When an `apply` makes a previously-live field non-live (e.g. delete
Schedule A ⇒ the `itemize_election` and SALT fields go non-live and the renderer hides them, §9A), the
underlying `ReturnInputs` value **persists**. Core's posture is that a lingering non-live value is a
recorded over-ask that changes no result — **with one exception that DOES change the computed number:**
`itemize_election == ForceItemize` returns the itemized arm *even with no Schedule A* (a $0 deduction —
`return_1040.rs:397`). So **`DeleteSection(ScheduleA)` MUST also reset `itemize_election` to `Auto`** (in
the section's `delete` fn), so deleting Schedule A cannot silently commit an invisible $0-deduction return.
No other v1 non-live field changes a computed line; the coverage KAT's exemption note records this as the
single result-bearing case.

### 5.2 A Field

`{ id: FieldId, label, help: &'static str, kind: FieldKind, live: fn(&RI)->bool, get, set }`. `label`/`help`
are **single-sourced** — the help line *is* the doc (the P8 "comment = doc" idea), and it is phrased as the
form phrases it. `FieldKind ∈ { Money(Usd, ≥0), Text, Bool, TriState(Option<bool>), Date(Option<Date>),
Enum(&[&str]), Secret }` *(★ M-a — `Bool` is the plain-checkbox kind added by I-8; §5.7 is the canonical
list)*.

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

Placement note: the **registries (fn-pointer data) live in CORE**; `btctax-input-form` *adapts* both into
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

### 5.7 The concrete types (`btctax-input-form` — illustrative signatures, not final)

```rust
pub struct RowAddr(pub Vec<usize>);          // path of indices to a row; empty for singletons; ≤ 2 today

pub enum SectionId {                          // stable — the wire contract; NEVER a Vec index
    ReturnOptions, Taxpayer, Spouse, Address, Dependents,
    W2s, W2Box12, ScheduleA, ScheduleACharitable, Payments,
    Declarations, Skippables,
}
pub enum FieldId { /* one per leaf — see §5.8 */ }

pub enum SectionKind {
    Singleton,
    OptionalSingleton { present: fn(&ReturnInputs) -> bool,
                        create:  fn(&mut ReturnInputs),
                        delete:  fn(&mut ReturnInputs) },
    Repeating         { len:    fn(&ReturnInputs, parent: &RowAddr) -> usize,
                        add:    fn(&mut ReturnInputs, parent: &RowAddr),
                        remove: fn(&mut ReturnInputs, addr: &RowAddr) },
}

pub enum FieldKind { Money, Text, Bool, TriState, Date, Enum(&'static [&'static str]), Secret }
//                              ^^^^ I-8: a plain checkbox (presidential-fund boxes are bare `bool`)

pub enum FieldValue {                         // serde-serializable — the web wire (M-5: owned, not &'static)
    Money(Usd), Text(String), Bool(bool), TriState(Option<bool>), Date(Option<Date>),
    Choice(String),                           // an enum choice by its stable name
    Secret(SecretView),                       // OUTBOUND only (get) — presence, never digits
    SecretEntry(String),                      // ★ I-2: INBOUND only (set) — masked `Debug`; get never returns it
}
pub enum SecretView { Empty, Set { masked: String } }     // NEVER carries digits (§4)

pub struct Field {
    pub id: FieldId, pub label: &'static str, pub help: &'static str, pub kind: FieldKind,
    pub live: fn(&ReturnInputs) -> bool,
    pub get:  fn(&ReturnInputs, &RowAddr) -> Option<FieldValue>,      // Secret ⇒ SecretView presence only
    pub set:  fn(&mut ReturnInputs, &RowAddr, FieldValue) -> Result<(), SetError>,  // Secret ⇐ SecretEntry
}

pub enum Edit {                               // serde-serializable — the web boundary
    SetField   { id: FieldId, addr: RowAddr, value: FieldValue },
    ClearField { id: FieldId, addr: RowAddr },
    AddRow     { section: SectionId, parent: RowAddr },
    RemoveRow  { section: SectionId, addr: RowAddr },
    CreateSection { section: SectionId },
    DeleteSection { section: SectionId },
}

// ★ NI-2 — the WORKING MODEL is `Option<ReturnInputs>`: `None` until a filing status is chosen. This makes
// "filing_status was chosen" ≡ "the RI exists" — held BY CONSTRUCTION, not by a renderer bool (which would
// be answered-ness by convention, the project's named sin). Both renderers (TUI, web) inherit the guard.
pub type Working = Option<ReturnInputs>;
pub fn apply(w: &mut Working, e: Edit) -> Result<(), ApplyError>;              // then re-eval liveness
//   On `None` (Fresh), the ONLY accepted Edit is `SetField{ ReturnOptions.filing_status }`, which
//   MATERIALIZES `Some(ReturnInputs{ filing_status, ..default })`. Any other Edit on `None` is `ApplyError`.
//   `commit`/`save_draft` take `&ReturnInputs` (a materialized working copy) — so an unchosen return simply
//   has nothing to commit; no in-`commit` "is it chosen?" check exists (it structurally cannot, I-9/NI-2).
pub fn parse(kind: FieldKind, raw: &str) -> Result<FieldValue, ParseError>;    // reuses Ssn/IpPin/Decimal
pub fn attribute(r: &RefuseReason) -> Vec<Anchor>;                             // EXHAUSTIVE match, §7
pub enum Anchor { Field(FieldId), Section(SectionId), NotInForm { note: &'static str } }

// btctax-cli :: input_form_store   (needs Session; depends on core types, not on the engine crate)
pub fn load(sess, year) -> Result<Loaded, CliError>;                  // draft ⇒ committed ⇒ FRESH (I-9)
pub enum Loaded {
    Draft { ri: ReturnInputs, parked: bool },   // ★ NI-1: parked round-trips so editing can't strip it
    Committed(ReturnInputs),
    Fresh,                                       // Working = None; no filing_status yet — I-9/NI-2
}
pub fn save_draft(sess, year, ri: &ReturnInputs) -> Result<(), CliError>;
//   ★ NI-1: read-modify-write PRESERVES the existing row's `parked` flag; a parked return stays parked
//   across edits until a successful re-commit consumes the row. (An unchosen `Working=None` writes no draft.)
pub fn commit(sess, year, ri: &ReturnInputs,
              table: &TaxTable, params: &FullReturnParams) -> Result<CommitOutcome, CliError>;   // N-2
pub fn park_to_profile(sess, year) -> Result<(), CliError>;          // stash→draft(parked=1) THEN delete (C-1)
pub enum CommitOutcome { Committed, Refused(Refusal), NoTables /* year lacks full-return params — I-11 */ }
```

`SectionKind::Repeating`'s `add`/`remove` take a `parent` `RowAddr` so nesting works: `W2Box12`'s parent is
`[w2_index]`, and `RemoveRow` on it addresses `[w2_index, box12_index]`. **Secrets are asymmetric (I-2):**
`get` returns `SecretView` (presence), never digits; `set` accepts only `SecretEntry(String)` and validates
via `Ssn::canonical` / `IpPin::canonical`. The `OptionalSingleton` `create`/`delete` are the `Schedule A` /
`spouse` presence edits; **`DeleteSection(ScheduleA)` also resets `itemize_election` to `Auto`** (I-10 — see
§5.1). The store fns take the held `&mut Session` (not a bare `conn`) because `save_draft`/`commit`/`park`
must call `Vault::save` to reach disk (I-7).

**`ClearField` semantics per kind (M-6):** `TriState`→`None`, `Date`→`None`, `Money`→`$0`, `Text`→`""`,
`Bool`→`false`, `Secret`→empty. **Enum has no empty state** (`filing_status`/`owner`/`itemize_election` are
required), so `ClearField` on an Enum field is a `SetError` — the un-answer path exists only for kinds whose
type carries an absence. (This is answered-ness-relevant: only `TriState`/`Date` clear to a true "unasked".)

### 5.8 The v1 field inventory

Every leaf below is one `Field` (or a registry entry). This table IS the coverage-KAT target (§5.6): each
in-scope struct leaf appears exactly once, or is an explicit exemption. `M`=Money(≥0), `T`=Text,
`Tri`=TriState, `D`=Date, `E`=Enum, `S`=Secret.

**`ReturnOptions`** (singleton) — `filing_status` **E**{Single,Mfj,Mfs,HoH,Qss} *(serde-REQUIRED; always
live; drives most other liveness. ★ I-9: a FRESH year has NO filing status — the field renders "not yet
chosen" (a renderer state, since the type has no `None`), and **commit is blocked until it is explicitly
chosen**; the §6.1 confirm names it. This preserves the TOML path's forced-choice discipline that
`ReturnInputs::default() = Single` would otherwise launder.)*; `itemize_election` **E**{Auto,ForceItemize}
*(live: `schedule_a.is_some()`; reset to `Auto` when Schedule A is deleted — §5.1 I-10)*.

**`Taxpayer`** (singleton, `header.taxpayer: Person` + `header.ip_pin`) — `first_name` **T**, `last_name`
**T**, `ssn` **S**, `occupation` **T**, `presidential_fund_taxpayer` **Bool** *(a bare checkbox — §5.4/I-8;
§6096 fund designation, lawful unchecked)*, and `ip_pin` **S** *(★ I-1 — `header.ip_pin: Option<String>`,
one per return, co-located here; `IpPin::canonical` at entry)*. *(DOB + blindness are Skippables that write
this `Person`; the renderer co-locates them here.)*

**`Spouse`** (optional-singleton, `header.spouse: Option<Person>`; create/delete; offered on
`filing_status ∈ {Mfj,Mfs,Qss}`) — the same `Person` fields as Taxpayer (first/last/ssn/occupation) +
`presidential_fund_spouse` **Bool**.

**`Address`** (singleton, `header`) — `address_street`/`city`/`state`/`zip` **T** ×4.

**`Dependents`** (repeating, `header.dependents: Vec<Dependent>`) — per row: `name` **T**, `ssn` **S**,
`relationship` **T**, `date_of_birth` **D**.

**`W2s`** (repeating, `w2s: Vec<W2>`) — per row: `owner` **E**{Taxpayer,Spouse} *(Spouse offered only on a
joint-capable status — else `SpouseOwnerWithoutJointReturn` refuses)*, `employer` **T**, and **M**:
`box1_wages`, `box2_fed_withheld`, `box3_ss_wages`, `box4_ss_withheld`, `box5_medicare_wages`,
`box6_medicare_withheld`, `box7_ss_tips`, `box17_state_tax_withheld`, `box19_local_tax`,
`box8_allocated_tips`, `box10_dependent_care`.
  - **`W2Box12`** (repeating, `w2s[i].box12: Vec<Box12Entry>`, parent `[i]`) — per row: `code` **T**
    *(non-inert codes refuse `UnsupportedBox12Code`; D/E/F/G/S over §402(g) refuse `ExcessElectiveDeferral`)*,
    `amount` **M**.

**`ScheduleA`** (optional-singleton, `schedule_a: Option<ScheduleAInputs>`; delete ⇒ standard deduction) —
**M**: `medical`, `salt_real_estate`, `salt_personal_property`, `salt_state_estimated_payments`,
`salt_prior_year_balance_paid`, `salt_sales_tax_amount`, `mortgage_interest_1098`. Plus two registry-driven
fields co-located here: `salt_use_sales_tax` **Tri** *(Skippable)* and `mortgage_all_used_to_buy_build_improve`
**Tri** *(Declaration `MortgageAllUsed`; live: this section ∧ `mortgage_interest_1098 > 0`)*.
  - **`ScheduleACharitable`** (repeating, `schedule_a.charitable: Vec<CharitableGift>`) — per row: `class`
    **E**{Cash60, Cash30, CapGainProp30, CapGainProp20, OrdinaryProp50, OrdinaryProp30} *(non-50%-org:
    Cash30 / OrdinaryProp30 / CapGainProp20 refuse `NonPublicCharityContribution` at commit)*, `amount` **M**.

**`Declarations`** (synthetic, from `FORM_QUESTIONS` — all **Tri**): `DependentTaxpayer` (always),
`DependentSpouse` (`Mfj || spouse present`), `MfsSpouseItemizes` (`Mfs`), `ForeignAccounts` (always),
`ForeignTrust` (always), `HsaActivity` (always), `DualStatusAlien` (always), `MortgageAllUsed` (shown in
ScheduleA above). Plus `foreign_country_names` **T** *(live: `foreign_accounts == Some(true)`; the Schedule
B 7b field — MUST be in-form so a "Yes" 7a is answerable, else commit refuses `ScheduleBForeignCountryMissing`
with no in-form remedy)*.

**`Skippables`** (synthetic, from the new core `SKIPPABLE_QUESTIONS`): `BlindTaxpayer` **Tri** (always),
`BlindSpouse` **Tri** (spouse present), `SalesTaxElection` **Tri** (`schedule_a.is_some()`) *(the
`salt_use_sales_tax` leaf — one FieldId `SaSaltUseSalesTax`, **shown in ScheduleA above**; its backing
`ScheduleAInputs` is deleteable, so its FieldId is Schedule-A-owned, mirroring the `MortgageAllUsed` dedup.
The Skippables **section** shrinks to blind ×2 + DOB ×2; the coverage KAT asserts every `SkippableId` maps to
exactly one FieldId *somewhere in the form*, not that it appears in this section)*,
`DateOfBirthTaxpayer` **D** (always), `DateOfBirthSpouse` **D** (spouse present).

**`Payments`** (singleton, `payments: Payments`) — **M** ×3: `estimated_tax_payments`, `extension_payment`,
`other_withholding`. *(★ M-1 — pulled into v1: three money fields, common (estimated payments), and it
resolves the §5.1/§2 inconsistency that already named Payments a singleton section.)*

**Explicitly EXEMPT from v1 (coverage KAT records these), deferred to TOML:** `int_1099`, `div_1099`,
`g_1099`, `schedule_c`, `qbi`, `capital_loss_carryforward_in`, `charitable_carryover_in`,
`sch1.{state_refund_taxable, student_loan_interest_paid, ira_deduction_claimed}` *(Schedule 1 money is
deferred — the `hsa_activity` declaration is in via `FORM_QUESTIONS`)*, and the `CarryProvenance` provenance
leaves.

## 6. Data flow, and the draft table

```
load(year)   → Draft{ri,parked} | Committed(ri) | Fresh   (Fresh = Working None, no filing_status — I-9/NI-2; TY2024-only — I-11)
apply(Edit)  → Working(Option<RI>): first Edit on None MUST set filing_status (materializes RI) → re-eval liveness → parse
save_draft   → return_inputs_draft + Vault::save (I-7), preserving `parked` (NI-1) — ANYTIME; resolve.rs NEVER reads it
commit       → (only a materialized RI reaches here — NI-2) screen_inputs(RI) → [caller: payload-confirm] →
               return_inputs::set → DELETE draft
```

### 6.1 Why a draft table (the blocking reason)

"Save the whole blob live via `return_inputs::set`" **would brick the year on every pause.** `resolve.rs`
treats a stored `ReturnInputs` row as top precedence, and if `screen_inputs` refuses it, the resolver
returns `profile: None` and **never falls through** to the crypto-only report or the `tax-profile` escape
hatch (`resolve.rs:85–109`, verified). A form is *necessarily* refused mid-entry (a fresh Single year has
~5 *live* `None` declarations, N-3; a half-typed W-2 is missing its employer). So:

- **`return_inputs_draft`** — a sibling table in the **same encrypted vault**, one row per year
  (`year PRIMARY KEY`), beside `return_inputs.rs` (≈200 non-test lines by the existing pattern). **`resolve.rs`
  never reads it** ⇒ every P8/`resolve.rs` invariant holds by construction. Type-invalid text never enters
  the working `ReturnInputs` (raw buffers that do not parse are held in the renderer), so a draft is always
  type-valid, possibly screen-refused — exactly the right laxity.

  ```sql
  CREATE TABLE IF NOT EXISTS return_inputs_draft (
      year           INTEGER PRIMARY KEY,
      inputs_json    TEXT    NOT NULL,
      schema_version INTEGER NOT NULL DEFAULT 0,
      parked         INTEGER NOT NULL DEFAULT 0   -- ★ C-1: 0 = disposable WIP; 1 = a PARKED committed
                                                  -- return (its sole copy) — protected like a committed row
  );
  ```
  **★ C-1 — `parked` gives a parked return committed-row semantics.** `parked = 1` means "there is no
  committed row backing this draft; the draft IS the return" (set by the §9 park). Such a row is the only
  copy of a screened return, SSNs included, so it is **not** treated as disposable WIP: the §6.2 coherence-
  clear **confirms/refuses** on it (never a silent delete), and §6.3's stale-version handling **refuses-and-
  reimports** it (never discard). `parked = 0` rows are ordinary WIP.

- **Commit gate is TY2024-only (I-11).** `screen_inputs` needs `&TaxTable + &FullReturnParams`, which exist
  only for TY2024 in v1 (`resolve.rs:86–94` fails closed elsewhere). The form **opens only table-bearing
  years** (the §12 year picker greys the rest); `commit` on a year without params returns `NoTables` and
  writes nothing — it never commits unscreened (which would poison the year at resolve).
- **`commit(sess, year, &RI, table, params)`** (the `btctax-cli` store fn) takes a **materialized**
  `&ReturnInputs` — the `Working = Option<RI>` model (NI-2) means an unchosen return has no `RI` to pass, so
  "filing status was chosen" is enforced **by construction upstream**, not by an (impossible) in-`commit`
  check on a non-`Option` enum. `commit` runs `screen_inputs`; if refused it **returns the `Refusal` and
  writes nothing**; if clean it `return_inputs::set`s and deletes the draft. The **payload-showing confirm**
  — which **prominently names the filing status** (I-9) and the row it replaces ("commit as **MFJ**;
  replaces the stored 2024 row; 2 W-2s, Schedule A, 1 dependent…") — is the **caller's** (TUI's) responsibility
  *before* calling `commit`; a returned refusal is surfaced via the §7 attribution map.

### 6.2 ★ Draft-vs-committed COHERENCE across sessions (Fable's near-blocking item)

A draft **persists across form sessions** (its crash-recovery purpose), and the other writers of the
committed row are ignorant of it: `income import` (`tax.rs:98`), `income answer` (`answer.rs:309`),
`report --write-carryover` (`tax.rs:461`), `income clear` (delete), `set-pii` (secret merge). The
`VaultLock` serializes them against the form (no concurrent access), but a *stale* draft is a silent-loss
hazard: edit → close → `income import` (writes the row, draft untouched) → reopen form → `load` prefers the
**stale draft**, hides the import, and committing clobbers it.

> ### RULE: an authoritative committed-row write CLEARS that year's WIP draft — but NEVER a parked one.
> `return_inputs::set` / `delete` from **`income import`, `income answer`, `report --write-carryover`,
> `income clear`, and (future) `set-pii`** also delete that year's `return_inputs_draft` row **when
> `parked = 0`** (warn if discarding a non-trivial WIP). A fresh committed write supersedes stale WIP.
> **★ C-1: when `parked = 1`, the write instead REFUSES**, naming the two real exits (M-d): *"year {y} holds
> a parked full return — in the form, **'use full return'** to re-commit it, or **'discard parked draft'**
> (a confirmed delete) to drop it; then re-run this command."* The parked blob is the sole copy of a
> screened return and must not be silently destroyed. **The toggle's park/commit (§9) is the one path that
> manages the draft explicitly**, and the form's **'discard parked draft'** is the only path that deletes a
> `parked = 1` row (a payload-showing confirm — never silent).
> *(I-6: `income answer` is in this list — it was the omitted fifth writer. set-pii is not yet implemented;
> it inherits the rule when it lands.)*

### 6.3 Draft stale-version: DISCARD a WIP draft, REFUSE a parked one

The committed row refuses on `schema_version` mismatch (`StaleReturnInputs`, refuse-and-reimport — it may
hold irreplaceable carryover). A draft splits by `parked`:

- **`parked = 0` (WIP): silently DISCARD** a stale-version row (discarded-with-note) — it is regenerable, so
  refusing would brick a resume for no benefit. This is the deliberate divergence from the mirrored
  `return_inputs` table.
- **★ C-1: `parked = 1` (a parked committed return): REFUSE-and-reimport**, exactly like `StaleReturnInputs`
  — the parked blob *is* a former committed row and may hold the irreplaceable carryover the committed-row
  refusal exists to protect. Silently discarding it (app upgrade → `SCHEMA_VERSION` bump → toggle-back finds
  nothing) is the C-1 data-loss path; it must fail closed, not open.

## 7. Validation — three tiers

1. **Field parse — a shared `btctax-input-form` helper `parse(kind, &str) -> Result<FieldValue, ParseError>`,
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
3. **Attribution — `btctax-input-form` — an EXHAUSTIVE `match RefuseReason -> Vec<FieldId | SectionId>`.**
   Exhaustive so a new `RefuseReason` variant is a **compile error** until attributed. **Never parse the
   prose `detail` strings** (the labels in `NegativeAmount(String)` are display prose, not identities). Some
   refusals attribute to a *section* or a *pair* of fields (`SaltSalesTaxWithoutElection` → two Schedule A
   fields; `ExcessElectiveDeferral` → the W-2 section), and it returns **`Vec<Anchor>`**. For most
   declarations attribution is exact via `RefuseReason ↔ QuestionId`, **but NOT injectively (I-5):**
   `ScheduleBPart3Unanswered` is carried by BOTH `ForeignAccounts` and `ForeignTrust` (`questions.rs:120,135`),
   so it anchors both and focuses the first live-unanswered one.

   **The v1 attribution map** (the exhaustive `match`; input-screenable reasons a v1 form can surface):

   | RefuseReason | Anchor |
   |---|---|
   | `DependentStatusUnanswered` / `DependentSpouseStatusUnanswered` / `MfsSpouseItemizeUnknown` / `HsaActivityUnanswered` / `DualStatusAlienUnanswered` / `MixedUseMortgageUnanswered` | the corresponding **Declaration** field (via `QuestionId` — exact) |
   | `ScheduleBPart3Unanswered` | **`[Field(ForeignAccounts decl), Field(ForeignTrust decl)]`** — shared by both; focus the first live-unanswered (I-5) |
   | `HsaActivityUnsupported` / `DualStatusAlienUnsupported` / `ForeignTrust` / `DependentSpouseUnsupported` | the corresponding **Declaration** field (the `Some(true)` value-refusal) |
   | `ScheduleBForeignCountryMissing` | `Field(foreign_country_names)` |
   | `SaltSalesTaxWithoutElection` | `[Field(salt_sales_tax_amount), Field(salt_use_sales_tax)]` — Schedule A only (M-2) |
   | `SalesTaxElectionWithoutAmount` | `[Field(salt_use_sales_tax), Field(salt_sales_tax_amount), Field(salt_state_estimated_payments), Field(salt_prior_year_balance_paid), Section(W2s)]` — the income-tax-SALT set (M-2) |
   | `NonPublicCharityContribution` | `[Section(ScheduleACharitable), NotInForm]` — also fires from `charitable_carryover_in` (deferred; M-3) |
   | `UnsupportedBox12Code(_)` | `Section(W2Box12)` (the offending row's `code`) |
   | `ExcessElectiveDeferral` / `AllocatedTips` / `DependentCareBenefit` | `Section(W2s)` (box 12 D/E/F/G/S / box 8 / box 10) |
   | `SingleEmployerExcessSs` | `Section(W2s)` (`box4_ss_withheld` — an in-form field; I-4) |
   | `SpouseOwnerWithoutJointReturn` | `[Section(W2s), NotInForm]` — also fires from `schedule_c.owner` (deferred; M-3) |
   | `NonCryptoNoncashGift` | `Section(ScheduleACharitable)` — form-reachable (a `CapGainProp*`/`OrdinaryProp*` gift > $500), but *compute-side* (`return_1040.rs:598-609`), so it surfaces at `report`, not the commit-screen; the anchor lets a reopened form point at the gift (M-c/M-4) |
   | `NegativeAmount(_)` / `SsnMalformed(_)` | `NotInForm { note }` — the payload is display-prose (this section forbids parsing it), so no `FieldId` is recoverable at tier-3, and no single `Section` is honest (a negative can come from any money field, in-form or deferred); **defensive only** anyway — unreachable, since tier-1 parse rejects negatives and bad SSNs before they enter the working copy |
   | everything else (`BusinessInterestIncome`, `BusinessIncomeWithoutScheduleC`, `ScheduleCLoss`, `ScheduleCNoBusinessDescription`, `KiddieTax`, `QbiAboveThreshold`, `AmtScreenTriggered`, `TaxableIncomeNonPositiveWithCarryforward`, `ForeignTaxOverCeiling`, `IraDeductionClaimed`, `PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`, `InconsistentDividendSubset`) | `NotInForm { note }` — a **deferred section** (Schedule C, QBI, 1099s incl. `PrivateActivityBondAmt`'s 1099-INT box 9 / 1099-DIV box 13 — I-3, carryforwards) or a **compute/absolute** screen; the form says "entered via TOML import / computed at `report`" |

   The `NotInForm` sentinel keeps the `match` exhaustive *and* honest: a v1 form cannot fix a Schedule-C
   loss and must say so rather than point at a field that does not exist. A new `RefuseReason` is a compile
   error until placed in one of these buckets. *(All **37** `RefuseReason` variants are placed above.)*

**Honesty carry-over (D-4):** the form's "screens clean" message must **name what it cannot see** — the
compute-dependent (`ScheduleCLoss`, `KiddieTax`) and absolute (`QbiAboveThreshold`, AMT) screens still run
at `report`/`export`, **and (★ M-4 — reachable from v1 form data alone) a noncash charitable gift over $500
that `screen_inputs` does NOT raise (`NonCryptoNoncashGift` is a *compute-side* refusal,
`return_1040.rs:610`).** So a filer can commit a screen-clean return that `report` then refuses; the message
must name the noncash-gift and compute screens as running later.

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
  - *"use tax-profile"* — **stash the committed row into its draft with `parked = 1`, THEN delete the
    committed row**, via **in-session `return_inputs::delete` on the held session** (⚠️ **NOT** the CLI
    `income clear` command — that re-opens `Session`, and `VaultLock::acquire` uses `try_lock_exclusive`
    (non-blocking, `lock.rs:18`), so the nested open **errors with lock contention** against itself — N-1).
    The `tax_profile` resumes automatically via precedence, untouched.
  - *"use full return"* — re-commit the parked draft (which consumes the row and clears `parked`).
  - **Stash-before-clear is atomic:** the delete is conditional on a **confirmed successful stash** within
    one session — a failed stash must never delete the row, because those SSNs (D-6) exist nowhere else.
  - **Offer "use tax-profile" only from a clean/committed state** (no WIP divergent from the committed
    row), so the one-row-per-year draft slot cannot clobber unsaved edits.
  - **★ NI-1 — a parked draft STAYS `parked = 1` through edits until a successful re-commit consumes it.**
    Once parked, the blob becomes the editable working copy, but `save_draft` preserves its `parked` flag
    (read-modify-write) and `Loaded::Draft{parked}` round-trips it to the renderer — so the C-1 protection
    (coherence-writes refuse, stale refuses-and-reimports) holds for the whole edit window, not just a
    pristine parked row. Only a re-commit (`return_inputs::set` + delete draft) drops `parked`.
- **The `tax_profile` is NEVER auto-deleted.** It is the fallback.

## 9A. The TUI interaction model (`btctax-tui-edit` "tax inputs" mode)

A thin renderer over `FormSpec` — it holds a `Working` (`Option<ReturnInputs>`; `None` until a filing status
is chosen — NI-2), a raw text buffer for the field being edited, and the current `RowAddr`; it never names a
`ReturnInputs` field. On a `Fresh`/`None` working copy it presents **only** the filing-status choice; the
first `SetField(filing_status)` materializes the return and the rest of the sections appear.

**Layout** — three regions:
- **Left: section list.** The live sections in order (`ReturnOptions → Taxpayer → Spouse? → Address →
  Dependents → W-2s → Schedule A? → Payments → Declarations → Skippables` — Payments per M-b), each with a status glyph (`✓` all live
  fields set / `…` incomplete / `!` a screen refusal attributed here). Non-live sections (e.g. `Spouse` on
  a Single return) are hidden, recomputed after every `apply`.
- **Right: field pane** for the selected section — each live `Field` as `label  [value]  ‹inline error›`.
  Repeating sections show rows with an index and an `[+ add] / [− remove]` affordance; optional sections
  show `[create] / [delete]`.
- **Bottom: status line** — the **active source** (`full return` / `tax-profile`), the screen status
  (`screens clean, EXCEPT what report computes` / `1 issue: <refusal>`), and the key legend.

**Keys** (final bindings settle in implementation against the existing editor's scheme):
- `↑/↓` move field, `←/→` or `Tab` move section; `Enter` edits the focused field.
- Repeating: `a` add row, `d` remove row (payload-confirm); optional: `c` create, `x` delete section.
- `TriState` cycles never→yes→no→never; `Enum` cycles/selects; `Date` is `YYYY-MM-DD`; `Secret` is
  **no-echo, masked** entry (the `UnlockState` passphrase discipline), showing `***-**-1234` when set.
- `s` **commit**: run `screen_inputs`; if `Refused`, jump focus to the attributed anchor (§7) and show the
  refusal; if clean, a **payload-confirm modal** → write → clear draft.
- `t` **toggle source** (offered only from a clean/committed state, §9): use-tax-profile (park) /
  use-full-return (re-commit).
- `X` **discard parked draft** (M-2 — only shown when a `parked = 1` draft exists; a payload-showing
  confirm; the sole path that deletes a parked row, §6.2). Distinct from `x` (delete an *optional section*).
- `q` quit (warns on an unsaved-draft divergence, but the draft is already autosaved).

**Autosave (I-7 — must reach disk).** The vault is an in-memory SQLite; nothing is persisted until
`Vault::save` re-encrypts and atomically writes (`vault.rs:231–245`). So `save_draft` writes the draft row
**and calls `Vault::save`** — otherwise a crash loses everything. Because `Vault::save` re-encrypts the whole
vault, autosave is **debounced** (on section-exit and on a short idle, not per keystroke), a deliberate
cost/safety trade the spec fixes here. With it, a terminal crash loses at most the current field — the
fiddly-TOML pain this feature removes. Snapshot tests (existing `btctax-tui-edit` style) pin the
rendered buffer for representative states (empty year, a two-W-2 MFJ return, a screen-refused SALT state,
the commit modal, the toggle prompt).

## 10. Testing (KATs)

**Engine (the form crate, no terminal):**
- **Field round-trip:** every non-Secret `Field` `get`→`FieldValue`→`set` round-trips; kind mismatch is a
  `SetError`. **Secret carve-out (I-2):** `get` returns `SecretView` (presence, no digits) and `set` accepts
  only `SecretEntry(String)` — assert `get` never returns digits and `set(SecretView)` is a `SetError` (the
  asymmetry, not a symmetric round-trip).
- **`apply` + liveness:** each `Edit` mutates the working copy and liveness is re-evaluated (a
  `filing_status` edit changes the live set). **I-10:** `DeleteSection(ScheduleA)` resets `itemize_election`
  to `Auto` (assert a `ForceItemize` + delete-Schedule-A cannot leave a $0-deduction return).
- **★ NI-2 materialization guard (M-3 — the anti-laundering construction; pin it):** `apply(None, e)` where
  `e` is **not** `SetField{filing_status}` → `ApplyError` (nothing materializes); `apply(None,
  SetField{filing_status=X})` → `Some(RI)` with `filing_status == X` **and every other field at default**;
  and `filing_status` never returns to `None` (Enum has no clear — `ClearField{filing_status}` is a
  `SetError`).
- **Tree edits:** `AddRow`/`RemoveRow` (incl. `box12` at depth 2, `RemoveRow [w2,box12]`),
  `CreateSection`/`DeleteSection` (Schedule A, spouse).
- **Exhaustive attribution:** every one of the 37 `RefuseReason`s maps (compile-forced); a representative
  refusal per anchor attributes correctly, incl. the I-3/I-4 corrections (`PrivateActivityBondAmt` →
  `NotInForm`, `SingleEmployerExcessSs` → `W2s`) and the I-5 shared `ScheduleBPart3Unanswered` → both.
- **Coverage KAT (§5.6):** in-scope leaf paths ⇔ fields, exemptions asserted inside the KAT (incl. `ip_pin`
  present — I-1 — and the deferred exemptions).
- **Bool vs TriState (I-8):** `presidential_fund_*` are `Bool`; no code path renders `TriState None` as "No".

**Persistence (`btctax-cli`):**
- **Draft is invisible to `resolve.rs`:** a screen-refused draft never poisons the year (the D-3/D-7
  property, re-pinned for the new table).
- **`commit`** screens → writes → deletes the draft; a screen-refused working copy does **not** commit; a
  non-2024 year returns `NoTables` and writes nothing (I-11). *(An unchosen `filing_status` cannot reach
  `commit` at all — the `Working = None` model gives it no `&RI` to pass; enforced by construction, not an
  in-`commit` check — I-9/NI-2/M-4.)*
- **Coherence rule (§6.2):** `income import` / **`income answer`** (I-6) / `write-carryover` / `income clear`
  clear a **WIP** draft — and **REFUSE** when it is `parked` (C-1). *(set-pii inherits this when it lands.)*
- **Draft stale-version:** a WIP draft **DISCARDS**; a **parked** draft **REFUSES-and-reimports** (C-1).
- **★ Autosave persistence (I-7):** `save_draft` survives a fresh `Session::open` (proves it reached disk
  via `Vault::save`, not just the in-memory conn).
- **Toggle** is non-destructive (park → profile resumes; re-commit → RI wins); stash-before-clear is atomic
  (a failed stash never deletes the committed row — SSN safety); gated to a clean state (can't clobber a
  divergent WIP); **an external committed write over a parked year refuses, not clobbers** (C-1).
- **★ form-commit preserves `Computed` carryovers** (the working copy starts from `get`, so it carries
  them — *better* than import's special-case merge; assert it).

**TUI:** snapshot tests in the existing `btctax-tui-edit` style; the renderer never names a `ReturnInputs`
field.

## 11. Build order (phased; each phase TDD, mutation-checked per the workflow)

1. **`btctax-input-form` crate skeleton + the seam types** — `FieldId`/`SectionId`, `Edit`, `FieldValue`,
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
