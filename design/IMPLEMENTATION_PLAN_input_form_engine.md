# Input-Form Engine — Implementation Plan (subsystem 1 of 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the headless, UI-agnostic `btctax-input-form` engine (`FormSpec` tree + `Edit`/`FieldValue` seam + `apply`/`parse`/`attribute`) and move the class-(B) skippable registry into core — the foundation both the TUI and a future web app render. Persistence (subsystem 2), the TUI (3), and docs (4) are separate plans.

**Architecture:** A new crate `btctax-input-form` depends on `btctax-core` only (vault-free, unit-testable). It exposes a `FormSpec` — a depth-≤3 tree of `Section`s over `ReturnInputs` — whose leaf `Field`s carry stable ids, a `FieldKind`, a `live` predicate, and monomorphic `get`/`set` accessors over `(&ReturnInputs, RowAddr)`. Declarations come from the existing `FORM_QUESTIONS`; skippables from a new `SKIPPABLE_QUESTIONS` registry in core (kept SEPARATE from `FORM_QUESTIONS` so `screen_inputs` never refuses on a `None`-legal skippable). The working model is `Option<ReturnInputs>` so "filing status chosen ≡ RI exists" holds by construction.

**Tech Stack:** Rust 2021, `rust_decimal` (Usd), `time` (Date), `serde` (the `Edit`/`FieldValue` wire types). Reuses `btctax_core::tax::{questions, return_inputs, return_refuse}` and `btctax_core::identity::{Ssn, IpPin}`.

**Spec:** `design/SPEC_input_form.md` (r5, GREEN 0C/0I). Section refs below are to that spec.

## Global Constraints

- Rust edition `2021`, `rust-version = "1.88"`, license `MIT OR Unlicense`, version `0.5.0` — all via `edition.workspace`/`license.workspace`/etc. (copy the `btctax-adapters/Cargo.toml` pattern).
- **Crate name is `btctax-input-form`, NOT `btctax-form`** (spec M-7 — collides with the published `btctax-forms`).
- **FROZEN — never edit:** `crates/btctax-core/src/tax/{types,compute,se}.rs`.
- **`btctax-input-form` depends on `btctax-core` ONLY** — no `btctax-cli`, no vault, no terminal.
- **No `serde_json::Value` path reflection anywhere** (spec §4 veto — reintroduces null-vs-absent laundering).
- **Skippables are a SEPARATE core registry from `FORM_QUESTIONS`** (spec §5.3 HARD RULE). Merging bricks `screen_inputs`.
- Gate per task: `make check` (~7s; the fast suite + clippy `-D warnings`), NOT `cargo test --workspace`. TDD: write the failing test, watch it fail, implement, watch it pass, commit. Mutation-check each guard (delete it → a named test fails → restore; use a `cp` backup + `touch` before re-run, never `git checkout` on uncommitted work).
- Fish shell: quote globs; use a heredoc for `git commit -F -`.

---

### Task 1: The `btctax-input-form` crate skeleton

**Files:**
- Create: `crates/btctax-input-form/Cargo.toml`
- Create: `crates/btctax-input-form/src/lib.rs`
- Modify: `Cargo.toml:3` (workspace `members`)

**Interfaces:**
- Produces: an empty compiling crate `btctax-input-form` depending on `btctax-core`, `rust_decimal`, `time`, `serde`.

- [ ] **Step 1: Add the crate to the workspace members.** In `Cargo.toml:3`, append `"crates/btctax-input-form"` to the `members` array (after `"crates/btctax-forms"`).

- [ ] **Step 2: Write `crates/btctax-input-form/Cargo.toml`**

```toml
[package]
name = "btctax-input-form"
version = "0.5.0"
edition.workspace = true
license.workspace = true
description = "UI-agnostic form engine (FormSpec + Edit seam) for authoring btctax ReturnInputs."
repository.workspace = true
homepage.workspace = true
keywords.workspace = true
categories = ["finance"]

[dependencies]
btctax-core = { path = "../btctax-core", version = "0.5.0" }
rust_decimal = { version = "1.36", default-features = false, features = ["std"] }
time = { version = "0.3", features = ["macros", "parsing", "formatting"] }
serde = { version = "1", features = ["derive"] }
```

- [ ] **Step 3: Write `crates/btctax-input-form/src/lib.rs`**

```rust
//! ★ The UI-agnostic input-form engine (`design/SPEC_input_form.md`). A `FormSpec` tree over
//! `ReturnInputs`, a serde `Edit` seam, and `apply`/`parse`/`attribute` — rendered by the TUI now and a web
//! app later. Depends on `btctax-core` only; no vault, no terminal.
#![forbid(unsafe_code)]

// modules land in later tasks:
// mod seam;      pub use seam::*;      // Task 2
// mod spec;      pub use spec::*;      // Tasks 4-5
// mod apply;     pub use apply::*;     // Task 7
// mod parse;     pub use parse::*;     // Task 8
// mod attribute; pub use attribute::*; // Task 9
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p btctax-input-form`
Expected: `Finished` (an empty lib).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/btctax-input-form
git commit -F - <<'EOF'
feat(input-form): btctax-input-form crate skeleton (plan 1 task 1)

New UI-agnostic form-engine crate (depends on btctax-core only), added to the
workspace. Empty lib; modules land in the following tasks.
EOF
```

---

### Task 2: The seam types

**Files:**
- Create: `crates/btctax-input-form/src/seam.rs`
- Modify: `crates/btctax-input-form/src/lib.rs` (uncomment `mod seam`)

**Interfaces:**
- Produces: `RowAddr`, `SectionId`, `FieldId`, `SectionKind`, `FieldKind`, `FieldValue`, `SecretView`, `Field`, `Section`, `Edit`, `Anchor`, `SetError`, `ParseError`, `ApplyError` — the data seam (spec §5.7, §4). `FieldId`/`SectionId`/`Edit`/`FieldValue` derive `Serialize`/`Deserialize` (the web wire, spec M-5).

- [ ] **Step 1: Write the failing test** (append to `seam.rs` a `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    /// The Edit/FieldValue seam serializes losslessly (the web wire, spec §4/M-5).
    #[test]
    fn edit_roundtrips_through_json() {
        let e = Edit::SetField {
            id: FieldId::Box1Wages,
            addr: RowAddr(vec![0]),
            value: FieldValue::Money(rust_decimal_macros::dec!(50000)),
        };
        let j = serde_json::to_string(&e).unwrap();
        let back: Edit = serde_json::from_str(&j).unwrap();
        assert_eq!(e, back);
    }

    /// A SecretView never carries digits; SecretEntry is inbound-only and masks its Debug.
    #[test]
    fn secret_view_is_presence_only_and_entry_masks_debug() {
        assert_eq!(
            SecretView::Set { masked: "***-**-6789".into() },
            SecretView::Set { masked: "***-**-6789".into() }
        );
        let entry = FieldValue::SecretEntry("123456789".into());
        assert!(!format!("{entry:?}").contains("123456789"), "SecretEntry Debug must not leak digits");
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test -p btctax-input-form --lib seam` → FAIL (types not defined). *(serde_json is a dev-dependency; add `serde_json = "1"` under `[dev-dependencies]` in the crate Cargo.toml first, then re-run.)*

- [ ] **Step 3: Write `crates/btctax-input-form/src/seam.rs`** (above the test module)

```rust
//! The data seam (spec §4, §5.7): a render model + an Edit stream both renderers consume.
use btctax_core::conventions::Usd;
use btctax_core::tax::return_inputs::ReturnInputs;
use serde::{Deserialize, Serialize};
use std::fmt;
use time::Date;

/// A path of indices to a row; empty for singletons; ≤ 2 today (`[w2_i, box12_i]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RowAddr(pub Vec<usize>);

/// Stable section identity — the wire contract; NEVER a Vec index (spec §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectionId {
    ReturnOptions, Taxpayer, Spouse, Address, Dependents,
    W2s, W2Box12, ScheduleA, ScheduleACharitable, Payments,
    Declarations, Skippables,
}

/// Stable field identity. One per leaf across the v1 sections (spec §5.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldId {
    // ReturnOptions
    FilingStatus, ItemizeElection,
    // Taxpayer / Spouse (Person + header.ip_pin)
    TpFirstName, TpLastName, TpSsn, TpOccupation, TpPresidentialFund, IpPin,
    SpFirstName, SpLastName, SpSsn, SpOccupation, SpPresidentialFund,
    // Address
    AddrStreet, AddrCity, AddrState, AddrZip,
    // Dependents (per row)
    DepName, DepSsn, DepRelationship, DepDob,
    // W2 (per row)
    W2Owner, W2Employer, Box1Wages, Box2FedWh, Box3SsWages, Box4SsWh,
    Box5MedWages, Box6MedWh, Box7SsTips, Box17StateWh, Box19LocalTax,
    Box8AllocTips, Box10DepCare,
    // W2 box 12 (per row)
    Box12Code, Box12Amount,
    // Schedule A
    SaMedical, SaSaltRealEstate, SaSaltPersonalProp, SaSaltStateEst,
    SaSaltPriorYear, SaSaltSalesTaxAmt, SaMortgage1098,
    SaSaltUseSalesTax, SaMortgageAllUsed,
    // Schedule A charitable (per row)
    CharClass, CharAmount,
    // Payments
    PayEstimated, PayExtension, PayOtherWh,
    // Declarations (from FORM_QUESTIONS) + the 7b country text
    DeclDependentTaxpayer, DeclDependentSpouse, DeclMfsSpouseItemizes,
    DeclForeignAccounts, DeclForeignTrust, DeclHsaActivity, DeclDualStatusAlien,
    ForeignCountryNames,
    // Skippables (from SKIPPABLE_QUESTIONS)
    BlindTaxpayer, BlindSpouse, SalesTaxElection, DobTaxpayer, DobSpouse,
}

/// The value shape of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind { Money, Text, Bool, TriState, Date, Enum(&'static [&'static str]), Secret }

/// A field value crossing the seam (spec §4/§5.7). Owned (serde), so it is the web wire.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Money(Usd),
    Text(String),
    Bool(bool),
    TriState(Option<bool>),
    Date(Option<Date>),
    Choice(String),          // an Enum choice by its stable name
    Secret(SecretView),      // OUTBOUND only (get) — presence, never digits
    SecretEntry(String),     // INBOUND only (set) — masked Debug; get never returns it
}

/// A secret's presence, never its digits (spec §4/§5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretView { Empty, Set { masked: String } }

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::SecretEntry(_) => write!(f, "SecretEntry(***)"),   // never leak digits
            FieldValue::Money(v) => write!(f, "Money({v})"),
            FieldValue::Text(s) => write!(f, "Text({s:?})"),
            FieldValue::Bool(b) => write!(f, "Bool({b})"),
            FieldValue::TriState(t) => write!(f, "TriState({t:?})"),
            FieldValue::Date(d) => write!(f, "Date({d:?})"),
            FieldValue::Choice(c) => write!(f, "Choice({c:?})"),
            FieldValue::Secret(s) => write!(f, "Secret({s:?})"),
        }
    }
}

/// A leaf field (spec §5.2). Accessors are monomorphic over `(&ReturnInputs, RowAddr)` — the row type never
/// appears (spec §4). Secret `get` returns presence; `set` accepts only `SecretEntry`.
pub struct Field {
    pub id: FieldId,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: FieldKind,
    pub live: fn(&ReturnInputs) -> bool,
    pub get: fn(&ReturnInputs, &RowAddr) -> Option<FieldValue>,
    pub set: fn(&mut ReturnInputs, &RowAddr, FieldValue) -> Result<(), SetError>,
}

/// A section: a singleton, an optional-singleton (create/delete), or a repeating group (spec §5.1).
pub struct Section {
    pub id: SectionId,
    pub title: &'static str,
    pub kind: SectionKind,
    pub fields: &'static [Field],
}

pub enum SectionKind {
    Singleton,
    OptionalSingleton {
        present: fn(&ReturnInputs) -> bool,
        create: fn(&mut ReturnInputs),
        delete: fn(&mut ReturnInputs),
    },
    Repeating {
        len: fn(&ReturnInputs, &RowAddr) -> usize,
        add: fn(&mut ReturnInputs, &RowAddr),
        remove: fn(&mut ReturnInputs, &RowAddr),
    },
}

/// An edit from a renderer (spec §5.7). Serde-serializable — the web wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edit {
    SetField { id: FieldId, addr: RowAddr, value: FieldValue },
    ClearField { id: FieldId, addr: RowAddr },
    AddRow { section: SectionId, parent: RowAddr },
    RemoveRow { section: SectionId, addr: RowAddr },
    CreateSection { section: SectionId },
    DeleteSection { section: SectionId },
}

/// Where a `RefuseReason` points in the form (spec §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor { Field(FieldId), Section(SectionId), NotInForm { note: &'static str } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError { WrongKind, NoSuchRow, Immutable }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError { NotANumber, Negative, BadDate, BadSsn, BadIpPin, NotAChoice }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError { NotChosenYet, WrongFirstEdit, SetError(SetError), NoSuchSection }
```

- [ ] **Step 4: Uncomment `mod seam; pub use seam::*;` in `lib.rs`, run tests** — `cargo test -p btctax-input-form --lib seam` → PASS.

- [ ] **Step 5: Commit** — `git add crates/btctax-input-form && git commit -m "feat(input-form): the data seam types (plan 1 task 2)"`

---

### Task 3: Move `SKIPPABLE_QUESTIONS` into core; `income answer` consumes it

**Files:**
- Modify: `crates/btctax-core/src/tax/questions.rs` (add `SKIPPABLE_QUESTIONS` + `SkippableQuestion` + `SkippableKind`)
- Modify: `crates/btctax-cli/src/cmd/answer.rs` (delete the local skippable liveness; consume the core registry)

**Interfaces:**
- Consumes: the existing `FORM_QUESTIONS` (unchanged), `Person.blind`, `ScheduleAInputs.salt_use_sales_tax`, `date_of_birth` accessors.
- Produces: `btctax_core::tax::questions::{SkippableQuestion, SkippableKind, SKIPPABLE_QUESTIONS}` — a SEPARATE 5-entry registry (blind ×2, SALT, DOB ×2), same fn-pointer shape, each with `live`/`kind` + typed get/set. **NOT merged into `FORM_QUESTIONS`.**

- [ ] **Step 1: Write the failing test** (in `questions.rs` tests)

```rust
#[test]
fn skippable_registry_is_separate_and_has_five_entries_with_correct_liveness() {
    use crate::tax::types::FilingStatus;
    assert_eq!(SKIPPABLE_QUESTIONS.len(), 5, "blind ×2, SALT, DOB ×2");
    // SALT is live iff a schedule_a exists; spouse-blind iff a spouse Person exists.
    let salt = SKIPPABLE_QUESTIONS.iter().find(|s| s.id == SkippableId::SalesTaxElection).unwrap();
    let mut ri = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
    assert!(!(salt.live)(&ri));
    ri.schedule_a = Some(Default::default());
    assert!((salt.live)(&ri));
    // The skippables are NOT in FORM_QUESTIONS (merging would brick screen_inputs on a None-legal skippable).
    for s in SKIPPABLE_QUESTIONS {
        assert!(!FORM_QUESTIONS.iter().any(|q| format!("{:?}", q.id) == format!("{:?}", s.id)),
                "a skippable must not also be a mandatory FORM_QUESTIONS declaration");
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test -p btctax-core --lib questions::` → FAIL (types missing).

- [ ] **Step 3: Implement `SKIPPABLE_QUESTIONS` in `questions.rs`.** Add (mirroring the `answer.rs::Skippable` model, which is being deleted): a `SkippableId` enum `{ BlindTaxpayer, BlindSpouse, SalesTaxElection, DobTaxpayer, DobSpouse }`; a `SkippableKind { YesNo, Date }`; a `SkippableQuestion { id: SkippableId, prompt: &'static str, help: &'static str, kind: SkippableKind, live: fn(&ReturnInputs)->bool, get_bool: fn(&ReturnInputs)->Option<bool>, set_bool: fn(&mut ReturnInputs,bool), get_date: fn(&ReturnInputs)->Option<Date>, set_date: fn(&mut ReturnInputs,Date) }` (the non-applicable accessors return `None`/no-op, matching the existing `Skippable::current_bool` catch-all pattern); and a `SKIPPABLE_QUESTIONS: &[SkippableQuestion]` with the 5 entries. Copy the exact liveness gates from `answer.rs::live_questions` (`spouse.is_some()`, `schedule_a.is_some()`) and the prompts from `answer.rs::Skippable::prompt`.

- [ ] **Step 4: Refactor `answer.rs` to consume the core registry.** Delete the local `Skippable` enum + its `prompt`/`current_bool`/`set_bool`/`current_date`/`set_date`/`kind` impls and the hard-coded liveness in `live_questions`; make `Ask::Skippable` wrap `&'static SkippableQuestion`; build the skippable `Ask`s by iterating `SKIPPABLE_QUESTIONS.filter(|s| (s.live)(ri))`. **Preserve `every_live_question_can_actually_be_answered_and_clears_the_screen` unchanged** (the no-brick property — sacred). Move `income_answer_asks_the_class_b_skippables_when_live` and `only_the_skippables_are_skippable` to assert over the core registry.

- [ ] **Step 5: Run to verify green** — `make check` → all pass (the answer.rs integration tests still pass; the skippable prompts still appear when live).

- [ ] **Step 6: Mutation-check** — `cp questions.rs questions.rs.bak; ` change the SALT `live` to `|_| true`; `cargo test -p btctax-core --lib questions::skippable` → the liveness test FAILS; `mv questions.rs.bak questions.rs; touch questions.rs`.

- [ ] **Step 7: Commit** — `git commit -m "refactor(P9): SKIPPABLE_QUESTIONS moves to core; income answer consumes it (plan 1 task 3)"`

---

### Task 4: The Declarations + Skippables `FormSpec` sections

**Files:**
- Create: `crates/btctax-input-form/src/spec/registries.rs`
- Create: `crates/btctax-input-form/src/spec/mod.rs` (with `pub fn form_spec() -> &'static [Section]` growing over tasks 4–5)

**Interfaces:**
- Consumes: `FORM_QUESTIONS`, `SKIPPABLE_QUESTIONS` (Task 3), the seam types (Task 2).
- Produces: two `Section`s (`Declarations`, `Skippables`) whose `Field`s adapt the core registries — each `Field.get`/`set` delegates to the registry entry's typed accessors; `Field.live` = the registry entry's `live`. A `FieldId ↔ QuestionId`/`SkippableId` mapping (a `match`, both directions) used by attribution (Task 9).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn declarations_section_covers_all_eight_form_questions_and_reads_them() {
    use btctax_core::tax::questions::QuestionId;
    let decls = section(SectionId::Declarations);
    // Every FORM_QUESTIONS entry appears as exactly one live-delegating Field.
    for q in btctax_core::tax::questions::FORM_QUESTIONS {
        assert_eq!(
            decls.fields.iter().filter(|f| field_to_question(f.id) == Some(q.id)).count(), 1,
            "declaration {:?} must map to exactly one Field", q.id);
    }
    // A Field's get reflects the registry's value.
    let mut ri = fresh_single();  // helper: a materialized Single RI
    ri.foreign_accounts = Some(true);
    let fa = decls.fields.iter().find(|f| f.id == FieldId::DeclForeignAccounts).unwrap();
    assert_eq!((fa.get)(&ri, &RowAddr::default()), Some(FieldValue::TriState(Some(true))));
}
```

- [ ] **Step 2: Run to verify it fails**, then **Step 3: implement** `registries.rs` — for each `FORM_QUESTIONS` entry build a `Field { kind: TriState, live: q.live, get: |ri,_| Some(FieldValue::TriState((q.get)(ri))), set: |ri,_,v| { let FieldValue::TriState(Some(b)) = v else {..}; (q.set)(ri,b); Ok(()) } , .. }` (a small generator; the `FieldId`↔`QuestionId` map is the one hand-written `match` both directions). Do the same for `SKIPPABLE_QUESTIONS` (YesNo→TriState, Date→Date). Add `foreign_country_names` as a `Text` field with `live: |ri| ri.foreign_accounts == Some(true)`.

- [ ] **Step 4: Run green** (`cargo test -p btctax-input-form --lib`), **Step 5: commit.**

---

### Task 5: The v1 section tree — singleton / optional / repeating accessors

**Files:**
- Create: `crates/btctax-input-form/src/spec/sections.rs`
- Modify: `crates/btctax-input-form/src/spec/mod.rs` (assemble all sections into `form_spec()`)

**Interfaces:**
- Consumes: the seam types, `btctax_core::tax::return_inputs::{ReturnInputs, W2, Box12Entry, ScheduleAInputs, CharitableGift, Person, Dependent, Payments, Owner, ItemizeElection, CharitableClass}`, `FilingStatus`.
- Produces: the remaining nine `Section`s of `form_spec()`, one `Field` per §5.8 leaf. **The §5.8 inventory table is the exact field list — implement every row; Task 6's coverage KAT enforces completeness.**

This is repetitive by kind. Implement each SectionKind by the pattern below, then fill every §5.8 field.

- [ ] **Step 1: Write the failing coverage-shaped test** (a spot check per kind — the exhaustive KAT is Task 6)

```rust
#[test]
fn w2_repeating_with_nested_box12_reads_and_writes() {
    let mut ri = fresh_single();
    let w2s = section(SectionId::W2s);
    let SectionKind::Repeating { add, len, .. } = w2s.kind else { panic!() };
    (add)(&mut ri, &RowAddr::default());
    assert_eq!((len)(&ri, &RowAddr::default()), 1);
    let box1 = w2s.fields.iter().find(|f| f.id == FieldId::Box1Wages).unwrap();
    (box1.set)(&mut ri, &RowAddr(vec![0]), FieldValue::Money(dec!(50000))).unwrap();
    assert_eq!(ri.w2s[0].box1_wages, dec!(50000));
    // nested box12
    let b12 = section(SectionId::W2Box12);
    let SectionKind::Repeating { add: add12, .. } = b12.kind else { panic!() };
    (add12)(&mut ri, &RowAddr(vec![0]));               // parent = w2 index 0
    assert_eq!(ri.w2s[0].box12.len(), 1);
    let amt = b12.fields.iter().find(|f| f.id == FieldId::Box12Amount).unwrap();
    (amt.set)(&mut ri, &RowAddr(vec![0, 0]), FieldValue::Money(dec!(23000))).unwrap();
    assert_eq!(ri.w2s[0].box12[0].amount, dec!(23000));
}

#[test]
fn schedule_a_optional_singleton_create_delete_resets_itemize_election() {
    let mut ri = fresh_single();
    ri.itemize_election = ItemizeElection::ForceItemize;
    let sa = section(SectionId::ScheduleA);
    let SectionKind::OptionalSingleton { create, delete, present } = sa.kind else { panic!() };
    (create)(&mut ri);
    assert!((present)(&ri));
    (delete)(&mut ri);
    assert!(!(present)(&ri));
    assert_eq!(ri.itemize_election, ItemizeElection::Auto, "I-10: delete resets ForceItemize");
}
```

- [ ] **Step 2: Run fail. Step 3: implement `sections.rs` by these patterns:**

**Money leaf** (repeating-row example — `Box1Wages`):
```rust
Field { id: FieldId::Box1Wages, label: "Box 1 — wages", help: "W-2 box 1 (wages, tips, other comp.)",
    kind: FieldKind::Money, live: |_| true,
    get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Money(w.box1_wages)),
    set: |ri, a, v| { let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
        ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.box1_wages = m; Ok(()) } },
```
**W2s repeating kind:**
```rust
SectionKind::Repeating {
    len: |ri, _| ri.w2s.len(),
    add: |ri, _| ri.w2s.push(W2::default()),
    remove: |ri, a| { if a.0[0] < ri.w2s.len() { ri.w2s.remove(a.0[0]); } },
}
```
**W2Box12 nested repeating** (`parent = [w2_i]`): `len: |ri,a| ri.w2s.get(a.0[0]).map_or(0,|w| w.box12.len())`, `add: |ri,a| { if let Some(w)=ri.w2s.get_mut(a.0[0]) { w.box12.push(Box12Entry::default()); } }`, `remove: |ri,a| { if let Some(w)=ri.w2s.get_mut(a.0[0]) { if a.0[1] < w.box12.len() { w.box12.remove(a.0[1]); } } }`.
**ScheduleA optional-singleton** (I-10 reset in `delete`):
```rust
SectionKind::OptionalSingleton {
    present: |ri| ri.schedule_a.is_some(),
    create: |ri| { if ri.schedule_a.is_none() { ri.schedule_a = Some(ScheduleAInputs::default()); } },
    delete: |ri| { ri.schedule_a = None; ri.itemize_election = ItemizeElection::Auto; }, // ★ I-10
}
```
**Spouse optional-singleton:** `present: |ri| ri.header.spouse.is_some()`, `create: |ri| { if ri.header.spouse.is_none() { ri.header.spouse = Some(Person::default()); } }`, `delete: |ri| ri.header.spouse = None`.
**Enum leaf** (`FilingStatus` — the materialization trigger; ReturnOptions singleton):
```rust
Field { id: FieldId::FilingStatus, label: "Filing status", help: "Single / MFJ / MFS / HoH / QSS",
    kind: FieldKind::Enum(&["Single","Mfj","Mfs","HoH","Qss"]), live: |_| true,
    get: |ri, _| Some(FieldValue::Choice(format!("{:?}", ri.filing_status))),
    set: |ri, _, v| { let FieldValue::Choice(c) = v else { return Err(SetError::WrongKind) };
        ri.filing_status = match c.as_str() { "Single"=>FilingStatus::Single, "Mfj"=>FilingStatus::Mfj,
            "Mfs"=>FilingStatus::Mfs, "HoH"=>FilingStatus::HoH, "Qss"=>FilingStatus::Qss,
            _ => return Err(SetError::WrongKind) }; Ok(()) } },
```
**Secret leaf** (`TpSsn`): `kind: Secret`, `get: |ri,_| Some(FieldValue::Secret(mask(&ri.header.taxpayer.ssn)))` where `mask("")=Empty`, else `Set{masked}` via a local masker (reuse the `***-**-NNNN` shape); `set: |ri,_,v| { let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) }; ri.header.taxpayer.ssn = s; Ok(()) }`. *(Canonical validation happens in `parse` — Task 8 — before a `SecretEntry` is built.)*
**Bool leaf** (`TpPresidentialFund`): `kind: Bool`, get/set `FieldValue::Bool` ↔ `ri.header.presidential_fund_taxpayer`.
**Enum with reject-non-50%** (`CharClass`): options = the six `CharitableClass` names; get/set map name↔variant.

Fill **every** §5.8 leaf: ReturnOptions (2), Taxpayer (6 incl. ip_pin), Spouse (5), Address (4), Dependents (4), W2s (13), W2Box12 (2), ScheduleA (9 incl. the 2 registry-driven co-located from Task 4), ScheduleACharitable (2), Payments (3).

- [ ] **Step 4: Run green. Step 5: commit** — `git commit -m "feat(input-form): the v1 section tree + accessors (plan 1 task 5)"`

---

### Task 6: The coverage KAT (drift-proofing)

**Files:**
- Create: `crates/btctax-input-form/src/spec/coverage.rs` (a `#[test]`)

**Interfaces:**
- Consumes: `form_spec()`, `ReturnInputs::default()` serialized to `serde_json::Value`.
- Produces: a test asserting every in-scope struct leaf-path maps to exactly one `Field` (or a listed exemption). **A new field on `W2` etc. fails this until covered.** (spec §5.6.)

- [ ] **Step 1: Write the test** — serialize `serde_json::to_value(&ReturnInputs::default())`; walk leaf key-paths of the in-scope structs (`header.*`, `w2s.*`, `schedule_a.*`, `payments.*`, top-level `filing_status`/`itemize_election`/`foreign_*`/`dual_status_alien`/`mfs_spouse_itemizes`/`sch1.hsa_activity`); build the set of paths each `Field`/registry entry covers; assert equality against `expected ∪ EXEMPT`, where `EXEMPT` is the explicit §5.8 deferred list (`int_1099`, `div_1099`, `g_1099`, `schedule_c`, `qbi`, `capital_loss_carryforward_in`, `charitable_carryover_in`, `sch1.{state_refund_taxable,student_loan_interest_paid,ira_deduction_claimed}`, provenance leaves). **Assert `EXEMPT` inside the test** (so an in-scope struct's new field still bites — spec §5.6). *(This walks `Value` for the KAT ONLY — never for get/set, per the §4 veto.)*

- [ ] **Step 2: Run green** (it passes for the complete Task 5). To prove it bites: temporarily comment out the `PayOtherWh` field → the KAT FAILS naming `payments.other_withholding`; restore.

- [ ] **Step 3: Commit** — `git commit -m "test(input-form): coverage KAT — a new in-scope field must be covered (plan 1 task 6)"`

---

### Task 7: `apply(&mut Working, Edit)` + the NI-2 materialization invariant

**Files:**
- Create: `crates/btctax-input-form/src/apply.rs`

**Interfaces:**
- Consumes: `form_spec()`, the seam types.
- Produces: `pub type Working = Option<ReturnInputs>;` and `pub fn apply(w: &mut Working, e: Edit) -> Result<(), ApplyError>`. On `None`, only `SetField{FilingStatus}` is accepted (materializes `Some(RI{filing_status, ..default})`); any other Edit on `None` → `ApplyError::{NotChosenYet|WrongFirstEdit}`. On `Some`, dispatch the Edit to the section/field accessors.

- [ ] **Step 1: Write the failing tests** (pin the NI-2 guard — spec §10 M-3)

```rust
#[test]
fn fresh_working_only_accepts_filing_status_first_then_materializes() {
    let mut w: Working = None;
    // a non-filing-status edit is rejected, leaving None
    let bad = apply(&mut w, Edit::SetField { id: FieldId::Box1Wages, addr: RowAddr(vec![0]),
        value: FieldValue::Money(dec!(1)) });
    assert_eq!(bad, Err(ApplyError::WrongFirstEdit));
    assert!(w.is_none());
    // choosing filing status materializes exactly that, all else default
    apply(&mut w, Edit::SetField { id: FieldId::FilingStatus, addr: RowAddr::default(),
        value: FieldValue::Choice("Mfj".into()) }).unwrap();
    let ri = w.as_ref().unwrap();
    assert_eq!(ri.filing_status, FilingStatus::Mfj);
    assert_eq!(ri.w2s.len(), 0);
    // filing_status can never be cleared (Enum, no empty state)
    assert_eq!(apply(&mut w, Edit::ClearField { id: FieldId::FilingStatus, addr: RowAddr::default() }),
        Err(ApplyError::SetError(SetError::Immutable)));
}
```

- [ ] **Step 2: Run fail. Step 3: implement `apply`** — match on `w`: `None` → accept only `SetField{FilingStatus, ..}` (materialize `Some(RI)`, then set it), else `Err(WrongFirstEdit)`; `Some(ri)` → look up the `Section`/`Field` by id and dispatch (`SetField`→`field.set`; `ClearField`→ a per-kind clear, Enum→`Err(Immutable)`; `AddRow`/`RemoveRow`→ the section's `add`/`remove`; `Create`/`DeleteSection`→ the optional-singleton `create`/`delete`).

- [ ] **Step 4: Run green. Step 5: mutation-check** the guard — make the `None` arm accept any Edit → the test fails; restore + touch. **Step 6: commit.**

---

### Task 8: `parse(kind, &str)` — the field-parse tier

**Files:**
- Create: `crates/btctax-input-form/src/parse.rs`

**Interfaces:**
- Consumes: `FieldKind`, `btctax_core::identity::{Ssn, IpPin}`, `rust_decimal::Decimal`.
- Produces: `pub fn parse(kind: FieldKind, raw: &str) -> Result<FieldValue, ParseError>` — Money = `Decimal ≥ 0` (else `Negative`), Date = `YYYY-MM-DD`, Secret = `Ssn::canonical`/`IpPin::canonical` (produces `SecretEntry`), TriState from `y/n/""`, Enum validated against the kind's options.

- [ ] **Step 1: Write the failing tests** — `parse(Money, "-5")` → `Err(Negative)`; `parse(Money, "50000")` → `Money(dec!(50000))`; `parse(Date, "1980-01-02")` → `Date(Some(..))`; `parse(Secret, "abc")` (as an SSN kind) → `Err(BadSsn)`; `parse(Enum(&["Single","Mfj"]), "Mfj")` → `Choice("Mfj")`, `parse(Enum(..), "Xx")` → `Err(NotAChoice)`.

- [ ] **Step 2: fail → Step 3: implement** (reuse `Ssn::canonical`/`IpPin::canonical`; note Secret parse needs to know SSN-vs-IP-PIN — pass it via distinct `FieldKind` or a `SecretKind` param; simplest: two parse entry points `parse_ssn`/`parse_ip_pin`, or thread the `FieldId`). → **Step 4: green → Step 5: commit.**

---

### Task 9: `attribute(&RefuseReason) -> Vec<Anchor>` — the exhaustive map

**Files:**
- Create: `crates/btctax-input-form/src/attribute.rs`

**Interfaces:**
- Consumes: `btctax_core::tax::return_refuse::RefuseReason`, the `FieldId ↔ QuestionId` map (Task 4).
- Produces: `pub fn attribute(r: &RefuseReason) -> Vec<Anchor>` — an **exhaustive** `match` (a new variant is a compile error) implementing the spec §7 table (all 37 variants).

- [ ] **Step 1: Write the failing tests** — the load-bearing rows from §7: `attribute(&ScheduleBPart3Unanswered)` → `[Field(DeclForeignAccounts), Field(DeclForeignTrust)]`; `attribute(&SingleEmployerExcessSs)` → `[Section(W2s)]`; `attribute(&PrivateActivityBondAmt)` → one `NotInForm`; `attribute(&NonPublicCharityContribution)` → contains `Section(ScheduleACharitable)` and a `NotInForm`; `attribute(&NonCryptoNoncashGift)` → `[Section(ScheduleACharitable)]`.

- [ ] **Step 2: fail → Step 3: implement the exhaustive `match`** exactly per the §7 table. **Step 4: green** (incl. a `match` with no `_ =>` arm — a new `RefuseReason` must not compile until placed). **Step 5: commit.**

---

## Self-Review

- **Spec coverage (§4, §5.1–5.8, §7 attribution, §5.6 KAT, the registry move §5.3, the NI-2 working model):** Task 2 = seam (§4/§5.7); Task 3 = registry move (§5.3); Tasks 4–5 = the FormSpec tree + all §5.8 leaves; Task 6 = coverage KAT (§5.6); Task 7 = `apply` + NI-2 (§5.7/§10 M-3); Task 8 = parse tier-1 (§7); Task 9 = attribution tier-3 (§7). **Deferred to later plans (correctly out of this plan):** the draft table / `load`/`save_draft`/`commit`/`park` (§6, §9 — subsystem 2); the TUI renderer (§9A — subsystem 3); docs (§9 build-order phase 9 — subsystem 4). Tier-2 (`screen_inputs` on commit) belongs to subsystem 2 (it runs at commit).
- **Placeholder scan:** the one intentional pattern-expansion is Task 5's field enumeration — mitigated by giving the complete accessor pattern per SectionKind, citing §5.8 as the exact list, and Task 6's KAT failing until every field is present. No "TBD"/"handle errors"/"similar to".
- **Type consistency:** `FieldId`/`SectionId`/`FieldValue`/`Edit`/`RowAddr`/`Anchor`/`Working` are defined in Task 2 and used unchanged in 4–9; `SKIPPABLE_QUESTIONS`/`SkippableId` defined in Task 3, consumed in Task 4; `form_spec()`/`section(id)` defined in Tasks 4–5, consumed in 6–9. The `FieldId` enum lists exactly the §5.8 leaves used by Task 5.
