//! ★ Task 6 — the coverage KAT (spec §5.6). A drift-proofing test: every IN-SCOPE `ReturnInputs`
//! leaf-path must be covered by exactly one `Field`'s `set` (observed, not declared), or be listed in the
//! explicit `EXEMPT` surface (the §5.8 deferred structs/leaves). A newly-added struct field on an in-scope
//! struct — not covered, not exempt — makes the two sets unequal, so the build goes red until someone gives
//! it a form `Field` (or exempts it deliberately). That standing bite is the whole point of the task.
//!
//! ★ **Mechanism — mutate-and-diff (drift-proof).** The covered-path set is DERIVED BY OBSERVATION, never
//! hand-declared: for each `Field` in `form_spec()`, clone a MAXIMALLY-POPULATED fixture, apply the Field's
//! `set` with a per-kind sentinel, and record which serde_json leaf path(s) actually changed. The union is
//! the covered set. A hand-written `FieldId → path` table could silently drift from what the accessors
//! really touch AND would not perturb when a new struct field appears; observation cannot. It also re-catches
//! a wrong-field mapping for free (a Field that writes the wrong leaf shows the wrong path here).
//!
//! ★ **Maximal fixture, not `default()`.** `ReturnInputs::default()` leaves `spouse`/`schedule_a` `None`,
//! `w2s`/`dependents`/`charitable`/`box12` empty, `ip_pin` `None` — so those leaf paths NEVER appear in the
//! serialized `Value`, and a KAT built on `default()` would give FALSE drift-protection for every W-2 /
//! spouse / Schedule-A / dependent field. This fixture forces every optional present and ≥1 element in every
//! in-scope Vec, so all 62 in-scope leaves are realized.
//!
//! ★ `serde_json::Value` walking is permitted HERE ONLY — the §4 veto is on get/set/production paths, not a
//! test. No accessor in this crate walks `Value`.

use super::form_spec;
use crate::seam::{Field, FieldId, FieldKind, FieldValue, RowAddr, SectionId};
use btctax_core::tax::return_inputs::{
    Box12Entry, CharitableClass, CharitableGift, Dependent, Person, ReturnInputs, ScheduleAInputs, W2,
};
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::date;

/// Flatten a JSON value into `leaf-path → leaf-value`. A leaf is any node that represents ONE logical struct
/// field: a scalar (string / number / bool / `null` — including an `Option::None`), OR an array whose every
/// element is itself scalar. That second case matters because `time::Date` serializes as `[year, ordinal]`
/// — a two-int array that is ONE struct leaf, not two. A real `Vec<Struct>` (its elements are objects)
/// instead recurses per element (`parent[i].field`), which is exactly the per-row granularity we want.
fn walk(v: &Value, prefix: &str, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                let p = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                walk(child, &p, out);
            }
        }
        // A real Vec<Struct> — at least one element is compound — recurses per element.
        Value::Array(arr) if arr.iter().any(|e| e.is_object() || e.is_array()) => {
            for (i, child) in arr.iter().enumerate() {
                walk(child, &format!("{prefix}[{i}]"), out);
            }
        }
        // A scalar, or an all-scalar array (notably a serialized `time::Date`) — one logical leaf.
        leaf => {
            out.insert(prefix.to_string(), leaf.clone());
        }
    }
}

/// The full leaf-path map of a `ReturnInputs`.
fn leaf_map(ri: &ReturnInputs) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    walk(&serde_json::to_value(ri).expect("ReturnInputs serializes"), "", &mut out);
    out
}

/// A MAXIMALLY-POPULATED fixture. Every optional is `Some`, every IN-SCOPE Vec has ≥1 element, so every
/// in-scope leaf path is realized. Leaf VALUES are the empty/zero/`None` defaults, so every sentinel below
/// is guaranteed to differ from what's here — the diff can never be a false "not covered". Exempt Vecs
/// (`int_1099`/…) stay empty and `schedule_c` stays `None`: they are exempt by struct-prefix regardless.
fn maximal_fixture() -> ReturnInputs {
    let mut ri = ReturnInputs { filing_status: FilingStatus::Mfj, ..Default::default() };
    ri.header.spouse = Some(Person::default());
    ri.header.ip_pin = Some("000000".to_string());
    ri.header.dependents = vec![Dependent::default()];
    ri.w2s = vec![W2 {
        // `Box12Entry` has no `Default`; a blank code + zero dollars is the empty new row (per sections.rs).
        box12: vec![Box12Entry { code: String::new(), amount: dec!(0) }],
        ..Default::default()
    }];
    ri.schedule_a = Some(ScheduleAInputs {
        // `CharitableGift` has no `Default`; Cash60/zero is the sections.rs `add` starting point.
        charitable: vec![CharitableGift { class: CharitableClass::Cash60, amount: dec!(0) }],
        ..Default::default()
    });
    ri
}

/// A per-`FieldKind` sentinel guaranteed to differ from the fixture's default leaf value, so a `set` that
/// truly writes ALWAYS produces a diff. Enum sentinels are chosen per `FieldId` to be a REAL variant that
/// differs from the fixture's default variant; a new Enum `Field` panics here until a sentinel is added.
fn sentinel(f: &Field) -> FieldValue {
    match f.kind {
        FieldKind::Money => FieldValue::Money(dec!(4242)),
        FieldKind::Text => FieldValue::Text("SENTINEL".to_string()),
        FieldKind::Bool => FieldValue::Bool(true),
        FieldKind::TriState => FieldValue::TriState(Some(true)),
        FieldKind::Date => FieldValue::Date(Some(date!(1990 - 01 - 02))),
        FieldKind::Secret => FieldValue::SecretEntry("123456789".to_string()),
        FieldKind::Enum(_) => {
            let choice = match f.id {
                FieldId::FilingStatus => "Single",      // fixture is Mfj
                FieldId::ItemizeElection => "ForceItemize", // fixture is Auto
                FieldId::W2Owner => "Spouse",           // fixture default is Taxpayer
                FieldId::CharClass => "OrdinaryProp50", // fixture is Cash60
                other => panic!("no Enum sentinel for {other:?} — add a distinct real choice"),
            };
            FieldValue::Choice(choice.to_string())
        }
    }
}

/// The `RowAddr` at which a section's `set` addresses row 0 (nested sections need a deeper path). A wrong
/// addr makes `set` return `Err`, or panics on an out-of-bounds index — it can NEVER yield a false PASS, so
/// this scaffolding map is not part of the coverage source of truth (the fixture already has row 0 present).
fn addr_for(id: SectionId) -> RowAddr {
    match id {
        SectionId::W2Box12 => RowAddr(vec![0, 0]),
        SectionId::Dependents | SectionId::W2s | SectionId::ScheduleACharitable => RowAddr(vec![0]),
        _ => RowAddr::default(),
    }
}

/// ★ THE COVERAGE KAT. Every in-scope leaf of the maximal fixture is covered by exactly one `Field`, or is
/// listed EXEMPT; and nothing is both. A new in-scope struct field bites here.
#[test]
fn every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt() {
    let fixture = maximal_fixture();
    let before = leaf_map(&fixture);

    // ── 1. The COVERED set — derived by OBSERVATION (mutate-and-diff), never hand-declared. ──
    let mut covered: BTreeMap<String, FieldId> = BTreeMap::new();
    let mut seen_kinds: BTreeSet<&'static str> = BTreeSet::new();
    for section in form_spec() {
        let addr = addr_for(section.id);
        for field in section.fields {
            seen_kinds.insert(match field.kind {
                FieldKind::Money => "Money",
                FieldKind::Text => "Text",
                FieldKind::Bool => "Bool",
                FieldKind::TriState => "TriState",
                FieldKind::Date => "Date",
                FieldKind::Enum(_) => "Enum",
                FieldKind::Secret => "Secret",
            });

            let mut ri = fixture.clone();
            (field.set)(&mut ri, &addr, sentinel(field))
                .unwrap_or_else(|e| panic!("set failed for {:?} in {:?}: {e:?}", field.id, section.id));

            let after = leaf_map(&ri);
            let all_keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
            let changed: Vec<String> =
                all_keys.into_iter().filter(|k| before.get(*k) != after.get(*k)).cloned().collect();
            assert_eq!(
                changed.len(),
                1,
                "{:?} ({:?}) must touch EXACTLY ONE leaf; it touched {:?}",
                field.id,
                section.id,
                changed
            );
            let path = changed.into_iter().next().unwrap();
            if let Some(prev) = covered.insert(path.clone(), field.id) {
                panic!("leaf `{path}` is double-covered — by {prev:?} and {:?}", field.id);
            }
        }
    }

    // ── 2. The EXEMPT surface (spec §5.8) — a LITERAL in the test, so a new in-scope field still bites. ──
    // Struct-prefix for wholly out-of-scope top-level structs; explicit leaf paths for the deferred leaves
    // of the IN-SCOPE `sch1` (its `hsa_activity` IS in scope, so `sch1` cannot be exempted wholesale). The
    // "provenance leaves" (`qbi.*_provenance`, `charitable_carryover_in[].provenance`) fall under their
    // struct-prefix exemptions.
    const EXEMPT_PREFIXES: &[&str] = &[
        "int_1099",
        "div_1099",
        "g_1099",
        "schedule_c",
        "capital_loss_carryforward_in",
        "charitable_carryover_in",
        "qbi",
    ];
    const EXEMPT_LEAVES: &[&str] = &[
        "sch1.state_refund_taxable",
        "sch1.student_loan_interest_paid",
        "sch1.ira_deduction_claimed",
    ];
    let is_exempt = |path: &str| {
        EXEMPT_LEAVES.contains(&path)
            || EXEMPT_PREFIXES.iter().any(|p| {
                path == *p || path.starts_with(&format!("{p}.")) || path.starts_with(&format!("{p}["))
            })
    };

    // ── 3. THE ASSERTION: {all in-scope leaves} == {covered} ∪ {exempt}, and nothing is both. ──
    let uncovered: Vec<&String> =
        before.keys().filter(|p| !covered.contains_key(*p) && !is_exempt(p)).collect();
    assert!(
        uncovered.is_empty(),
        "these IN-SCOPE leaves are covered by NO Field and are NOT in EXEMPT — add a Field (or exempt it \
         deliberately in §5.8): {uncovered:#?}"
    );
    let covered_and_exempt: Vec<&String> =
        before.keys().filter(|p| covered.contains_key(*p) && is_exempt(p)).collect();
    assert!(
        covered_and_exempt.is_empty(),
        "these leaves are BOTH covered by a Field AND listed EXEMPT — resolve the contradiction: \
         {covered_and_exempt:#?}"
    );

    // A covered path that is NOT a fixture leaf means a Field wrote somewhere the maximal fixture never
    // realized — a fixture/accessor mismatch. (Belt-and-suspenders; must be empty.)
    let phantom: Vec<&String> = covered.keys().filter(|p| !before.contains_key(*p)).collect();
    assert!(phantom.is_empty(), "Fields touched non-fixture (phantom) leaves: {phantom:#?}");

    // Every FieldKind must have been exercised (requirement 4) — including Bool, Date, and Secret.
    for k in ["Money", "Text", "Bool", "TriState", "Date", "Enum", "Secret"] {
        assert!(seen_kinds.contains(k), "FieldKind {k} was never exercised by the KAT");
    }

    // Count tripwires — pin the 62-leaf / 62-Field identity so a silent drop is loud even if some other
    // change happened to keep the sets balanced.
    let field_count: usize = form_spec().iter().map(|s| s.fields.len()).sum();
    assert_eq!(field_count, 62, "expected 62 Fields (one per §5.8 in-scope leaf)");
    assert_eq!(covered.len(), 62, "expected 62 distinctly-covered in-scope leaves");
}
