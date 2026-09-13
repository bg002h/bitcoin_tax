//! `xtask toml-schema` — the PUBLISHED schema for the TOML file `btctax income import` consumes
//! (`docs/income-import-schema.md`), and the staleness gate that keeps it true.
//!
//! ## Why this is generated and not written
//!
//! FR-203: nothing in `docs/` published the shape of that file, so every *"use `income import`
//! instead"* remedy in every refusal message was unactionable — the operator who drove it by hand had
//! to reconstruct the TOML from `btctax_cli::testonly::J10_FULLRETURN_TOML`.
//!
//! The obvious fix — type the field list into a Markdown document — is the single defect shape this
//! repo finds most often (`CLAUDE.md`, *"Derive the list, or make the compiler hold it"*): a list
//! correct on the day it is written, beside a struct that grows underneath it. So the key set is
//! **derived from the type**, by the same mechanism the import guard itself uses:
//!
//! > `ReturnInputs` is serialized, and every path in the resulting `toml::Value` is a key
//! > `income import` honours. Nothing else is one — `parse_return_inputs_toml` runs `serde_ignored`
//! > over the same serde shape and refuses every path it does not recognise, by name.
//!
//! Two properties make that derivation total rather than merely plausible:
//!
//! 1. the fixture is `maximal_sentinel` — *"every `Option` is `Some`, every `Vec` holds two
//!    elements, every nested struct is present"* — so no key can be missing because the fixture
//!    never instantiated it (the exact blind spot `scrub_axis`'s module header was written about);
//! 2. that fixture is an **exhaustive struct literal with no `..`**, so a field added anywhere under
//!    `ReturnInputs` is a **compile error** in `scrub_axis.rs` before it can be a silently
//!    unpublished key here.
//!
//! ## What it does NOT cover, stated rather than implied
//!
//! - **Semantics.** The inventory says a key exists and what TOML type it takes. It does not say what
//!   the figure means; the doc comment on the field does, and the worked example shows one in use.
//! - **Whether the return is COMPLETE.** [`required_paths`] measures which keys the *parser* needs,
//!   which is a much smaller set than the one a filable return needs. What makes a return complete is
//!   `screen_inputs`, whose refusals quote the form; that is a different instrument, it reports at a
//!   different moment, and this document points at it rather than duplicating it.
//! - **Open-keyed tables.** Two paths are maps whose KEYS are filer data rather than schema
//!   ([`OPEN_KEYED`]); their key segment is published as a placeholder, and each declaration is
//!   asserted against the emitted value so a rename reds.

use btctax_core::tax::scrub_axis::maximal_sentinel;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
#[cfg(test)]
use std::path::{Path, PathBuf};

/// The workspace root (two levels up from `crates/xtask`). Only the staleness gate needs it — the
/// generator writes to stdout, exactly like `xtask examples`.
#[cfg(test)]
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above crates/xtask")
        .to_path_buf()
}

/// The committed document this generator owns.
pub const DOC_PATH: &str = "docs/income-import-schema.md";

/// The command that regenerates it, quoted in every failure message.
pub const REGEN: &str = "cargo run -p xtask -- toml-schema > docs/income-import-schema.md";

/// ★★★ **The paths whose child KEYS are filer data, not schema** — a map, not a struct.
///
/// `(path, placeholder, what the key is)`. Published with the placeholder in place of the sentinel
/// keys the fixture happens to carry, because `broker_reporting.SENTINEL_venue_one.covered` is a
/// *value* masquerading as a schema key.
///
/// ★ **DECLARED, and then CHECKED.** [`assert_open_keyed_paths_exist`] runs on every generation and
/// panics if a declared path is absent from the emitted value or is not a table — so renaming
/// `broker_reporting` cannot leave a stale entry here quietly pruning nothing.
const OPEN_KEYED: &[(&str, &str, &str)] = &[
    (
        "broker_reporting",
        "<provider>",
        "the exchange/provider key, exactly as `report --tax-year` lists it",
    ),
    (
        "answer_log",
        "<answer-key>",
        "btctax's own record of asking — NOT importable, see below",
    ),
];

/// The TOML type name for one value, in the vocabulary the TOML spec uses.
fn kind_of(v: &toml::Value) -> &'static str {
    match v {
        toml::Value::String(_) => "string",
        toml::Value::Integer(_) => "integer",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "boolean",
        toml::Value::Datetime(_) => "datetime",
        // ★ A HETEROGENEOUS array is just "array". Naming it after its FIRST element is how
        //   `answer_log_history` — a `Vec<(AnswerKey, AnswerRecord)>`, i.e. a list of
        //   `[string, table]` pairs — was first published as an "array of strings", which is a
        //   false claim in a document about exactly the kind of thing this repo does not ship.
        toml::Value::Array(a) => {
            let first = a.first().map(std::mem::discriminant);
            let uniform = first.is_some_and(|d| a.iter().all(|e| std::mem::discriminant(e) == d));
            match a.first() {
                Some(toml::Value::Table(_)) if uniform => "array of tables",
                Some(toml::Value::String(_)) if uniform => "array of strings",
                Some(toml::Value::Integer(_)) if uniform => "array of integers",
                _ => "array",
            }
        }
        toml::Value::Table(_) => "table",
    }
}

/// ★★★ **What `income import` does with this key BEYOND storing it** — derived from the path, not
/// listed beside it, so a new `*_provenance` field or a new `answer_log*` leaf is annotated the day
/// it appears.
///
/// Both mechanisms are in `cmd::tax::import_return_inputs`, which NORMALISES rather than refuses (the
/// keys are a legitimate part of the shape `income scrub` emits, so refusing them would make btctax
/// unable to read a file it writes).
fn note_for(path: &str) -> &'static str {
    if path.starts_with("answer_log") {
        "**discarded** on import"
    } else if path.ends_with("_provenance") || path.ends_with(".provenance") {
        "forced to `user`"
    } else {
        ""
    }
}

/// Walk one `toml::Value` and record `(key path, TOML type)` for every path it reaches.
///
/// ★ Array elements collapse to a single `[]` segment, so the inventory is **field-shaped, not
///   index-shaped** — `w2s[0].ein` and `w2s[1].ein` are one key, `w2s[].ein`. Same vocabulary
///   `scrub_axis::diff_paths` uses, and the reason the maximal fixture's two-element `Vec`s do not
///   double the inventory.
///
/// ★ An [`OPEN_KEYED`] table's child keys are replaced by its placeholder, so two sentinel provider
///   names collapse to one published key.
fn walk(v: &toml::Value, path: &str, out: &mut BTreeMap<String, &'static str>) {
    out.insert(path.to_string(), kind_of(v));
    match v {
        toml::Value::Table(t) => {
            let open = OPEN_KEYED.iter().find(|(p, _, _)| *p == path);
            for (k, child) in t {
                let seg: &str = open.map_or(k.as_str(), |(_, placeholder, _)| placeholder);
                walk(child, &format!("{path}.{seg}"), out);
            }
        }
        toml::Value::Array(a) => {
            for e in a {
                walk(e, &format!("{path}[]"), out);
            }
        }
        _ => {}
    }
}

/// ★★★ **B1's other half for [`OPEN_KEYED`]** — a declaration that prunes nothing is a lie.
///
/// Panics if a declared path is absent from the emitted value, or is present but not a table. Either
/// means the declaration has gone stale (a rename, a type change), and the inventory would then
/// publish sentinel VALUES as if they were schema keys with nothing reporting it.
fn assert_open_keyed_paths_exist(root: &toml::Value) {
    for (path, _, _) in OPEN_KEYED {
        let mut cur = root;
        for seg in path.split('.') {
            cur = cur.get(seg).unwrap_or_else(|| {
                panic!(
                    "toml-schema: OPEN_KEYED declares {path:?}, which is NOT a path of the serialized \
                     ReturnInputs. It was renamed or removed — fix OPEN_KEYED in \
                     crates/xtask/src/toml_schema.rs, or the inventory will publish filer VALUES as \
                     schema keys."
                )
            });
        }
        assert!(
            cur.as_table().is_some(),
            "toml-schema: OPEN_KEYED declares {path:?} as an open-keyed TABLE, but the serialized \
             value is a {}. The placeholder would prune nothing.",
            kind_of(cur)
        );
    }
}

/// ★★★ **THE ONE PLACE `maximal_sentinel` IS DELIBERATELY NOT MAXIMAL, AND IT COST KEYS.**
///
/// `scrub_axis.rs:620` sets `broker_reporting: Default::default(), // nothing answered (spec 1099-DA
/// T1)` — correct for the scrub axis, which is about fields scrub REPLACES, and wrong for a key
/// inventory: an empty `BTreeMap` instantiates no child, so the first generation published
/// `broker_reporting` as a bare table and **omitted `broker_reporting.<provider>.covered` and
/// `.noncovered` entirely** — two keys `income import` honours, that the clap help for `income import`
/// devotes a paragraph to, and that a TY2026 return refuses without.
///
/// So the fixture is augmented here rather than in `scrub_axis` (changing the maximal sentinel moves
/// the derived scrub axis, which is a different instrument's business). `CohortAnswers` carries
/// `skip_serializing_if = "Option::is_none"` — the only such attribute under `ReturnInputs` — so BOTH
/// slots must be `Some` or the key vanishes again.
///
/// ★ And the augmentation is not trusted: [`assert_no_empty_containers`] fails the generation on any
///   empty table or array anywhere in the emitted value, so the NEXT container a fixture leaves empty
///   is a loud panic naming the path rather than a silently shorter document.
fn augmented_sentinel() -> btctax_core::tax::return_inputs::ReturnInputs {
    use btctax_core::forms::{BrokerReported, CohortAnswers};
    let mut ri = maximal_sentinel();
    ri.broker_reporting.0.insert(
        "SCHEMA_PLACEHOLDER_PROVIDER".to_string(),
        CohortAnswers {
            covered: Some(BrokerReported::BasisMatches),
            noncovered: Some(BrokerReported::NotReported),
        },
    );
    ri
}

/// ★★★ **THE DERIVATION'S BLIND SPOT, MADE LOUD.** An empty table or array instantiates no child, so
/// every key beneath it goes unpublished — and the document still looks complete, which is the worst
/// possible failure for a schema. `scrub_axis`'s own module header is about exactly this shape:
/// *"the hazard is that a replaced field produces no differing path when the fixture never
/// instantiates it"*.
///
/// This is the check that turns that hazard into a build failure. It is what makes
/// [`augmented_sentinel`] a fix rather than a patch: the next `BTreeMap` or `Vec` a fixture leaves
/// empty panics here, by path, instead of shortening the inventory.
fn assert_no_empty_containers(v: &toml::Value, path: &str) {
    match v {
        toml::Value::Table(t) => {
            assert!(
                !t.is_empty(),
                "toml-schema: `{path}` serialized as an EMPTY table, so every key beneath it is \
                 UNPUBLISHED and this document would look complete anyway. Give the fixture a value \
                 for it — see `augmented_sentinel`, which does exactly that for `broker_reporting`."
            );
            for (k, child) in t {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                assert_no_empty_containers(child, &p);
            }
        }
        toml::Value::Array(a) => {
            assert!(
                !a.is_empty(),
                "toml-schema: `{path}` serialized as an EMPTY array, so every key inside one of its \
                 rows is UNPUBLISHED. Give the fixture at least one row — see `augmented_sentinel`."
            );
            for e in a {
                assert_no_empty_containers(e, &format!("{path}[]"));
            }
        }
        _ => {}
    }
}

/// Split one field-shaped inventory path into tokens: `w2s[].box12[].code` → `["w2s[]", "box12[]",
/// "code"]`.
fn tokens(path: &str) -> Vec<&str> {
    path.split('.').collect()
}

/// Remove one field-shaped path from a `toml::Value` in place; `true` if anything was removed.
///
/// A `name[]` token means *descend into every element of the array at `name`* — the same
/// field-shaped-not-index-shaped vocabulary [`walk`] emits, so `w2s[].ein` removes the key from BOTH
/// rows the maximal fixture carries and the answer is about the FIELD rather than about row 0.
fn remove_at(v: &mut toml::Value, toks: &[&str]) -> bool {
    let Some((head, rest)) = toks.split_first() else {
        return false;
    };
    let (name, is_arr) = head
        .strip_suffix("[]")
        .map_or((*head, false), |n| (n, true));
    let Some(table) = v.as_table_mut() else {
        return false;
    };
    if rest.is_empty() && !is_arr {
        return table.remove(name).is_some();
    }
    let Some(child) = table.get_mut(name) else {
        return false;
    };
    if !is_arr {
        return remove_at(child, rest);
    }
    // `name[]` with nothing after it denotes the ELEMENT type, not a key a filer can omit.
    let Some(arr) = child.as_array_mut() else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let mut any = false;
    for e in arr.iter_mut() {
        any |= remove_at(e, rest);
    }
    any
}

/// Replace the value at one field-shaped path; `true` if anything was replaced. The sibling of
/// [`remove_at`], with the same `name[]`-descends-into-every-row rule.
fn replace_at(v: &mut toml::Value, toks: &[&str], new: &toml::Value) -> bool {
    let Some((head, rest)) = toks.split_first() else {
        return false;
    };
    let (name, is_arr) = head
        .strip_suffix("[]")
        .map_or((*head, false), |n| (n, true));
    let Some(table) = v.as_table_mut() else {
        return false;
    };
    if rest.is_empty() && !is_arr {
        return match table.get_mut(name) {
            Some(slot) => {
                *slot = new.clone();
                true
            }
            None => false,
        };
    }
    let Some(child) = table.get_mut(name) else {
        return false;
    };
    if !is_arr {
        return replace_at(child, rest, new);
    }
    let Some(arr) = child.as_array_mut() else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let mut any = false;
    for e in arr.iter_mut() {
        any |= replace_at(e, rest, new);
    }
    any
}

/// ★★★ **WHICH LEAVES ARE DATES — ASKED, NOT INFERRED FROM THE TYPE NAME.**
///
/// A `time::Date` SERIALIZES here as the compact `[year, ordinal]` tuple, so the raw inventory labels
/// every date `array of integers` — which would tell a filer to write `[2012, 106]` and is not what
/// the deserializer wants.
///
/// The first draft of this module went further and published the claim that the `"YYYY-MM-DD"`
/// spelling is *refused*. **It is not** — `either_date_spelling_parses_and_the_serialized_form_is_the_tuple`
/// reported that, which is the only reason the document does not say so today. The asymmetry is
/// `toml`'s: its serializer is not human-readable and its deserializer is, so a `Date` goes out as a
/// tuple and comes back from either spelling.
///
/// So which leaves are dates is **measured**: substitute an ISO date string at the path and re-parse.
/// A leaf that accepts one is a date; one that does not keeps its literal TOML type, and the
/// document's paragraph is scoped to exactly the set this returns.
fn date_paths(base: &toml::Value, paths: &BTreeMap<String, &'static str>) -> BTreeSet<String> {
    let iso = toml::Value::String("1970-01-01".to_string());
    let candidates: Vec<&String> = paths
        .iter()
        .filter(|(_, kind)| **kind == "array of integers")
        .map(|(p, _)| p)
        .collect();
    let found: BTreeSet<String> = candidates
        .iter()
        .filter(|p| {
            let mut probe = base.clone();
            replace_at(&mut probe, &tokens(p), &iso)
                && probe
                    .try_into::<btctax_core::tax::return_inputs::ReturnInputs>()
                    .is_ok()
        })
        .map(|p| (*p).clone())
        .collect();

    // ★★★ **THE PROBE'S OWN BLIND SPOT, MADE LOUD.** Two shapes it cannot address: a path under an
    //     [`OPEN_KEYED`] table (its `<placeholder>` segment is not a literal key) and a nested
    //     array-of-arrays (`answer_log_history[][]`, which `replace_at` deliberately does not descend).
    //     Both live under `answer_log*` today, and the import **discards** those keys anyway — so
    //     leaving them typed as `array of integers` costs a filer nothing.
    //
    // ★ That is a boundary, so it is CHECKED rather than assumed. If a date leaf ever appears outside
    //   the discarded subtree and the probe cannot reach it, the document would quietly tell a filer to
    //   write `[year, ordinal]` at a key that wants an ISO string — so the generation fails instead.
    for p in candidates {
        assert!(
            found.contains(p) || p.starts_with("answer_log"),
            "toml-schema: `{p}` serializes as an `array of integers` but the ISO-date probe could not \
             confirm it is a date, and it is NOT under `answer_log*` (the subtree the import discards, \
             where the probe is known to be blind). The document would publish `array of integers` for \
             a key the deserializer may want as a string. Extend `replace_at`, or state the new \
             boundary here."
        );
    }
    found
}

/// ★★★ **WHICH KEYS THE PARSER REQUIRES — MEASURED, one deletion at a time.**
///
/// The first draft of the generated document said *"every key is optional to the parser: `ReturnInputs`
/// carries `#[serde(default)]` on every field"*. **That is false** — `filing_status` has no default,
/// and neither do `Person`'s name and SSN fields — and it was written from reading a handful of
/// attributes rather than from asking the deserializer. A schema that tells a filer a required key is
/// optional sends them to a parse error with no way to see why.
///
/// So the answer is derived by the only thing that actually knows: for each inventory path, delete it
/// from the complete value and hand the rest to `serde`. If the file no longer deserializes, the key
/// is **required when its parent table is present** — which is the right qualifier, because deleting
/// `w2s` entirely is fine while deleting `w2s[].box1_wages` from a row that exists is not.
///
/// ★ The predicate is *"omitting it breaks the parse"*, not *"the error message says `missing
///   field`"*: a schema reader cares that the file will not load, not which of serde's messages says
///   so.
fn required_paths(base: &toml::Value, paths: &BTreeMap<String, &'static str>) -> BTreeSet<String> {
    paths
        .keys()
        .filter(|p| {
            let mut probe = base.clone();
            if !remove_at(&mut probe, &tokens(p)) {
                return false; // nothing to remove (a placeholder segment, or an element type)
            }
            probe
                .try_into::<btctax_core::tax::return_inputs::ReturnInputs>()
                .is_err()
        })
        .cloned()
        .collect()
}

/// The serialized fixture every derivation in this module reads, with both guards already run.
fn serialized() -> toml::Value {
    let ri = augmented_sentinel();
    // ★ `toml::Value::try_from` THEN render, never `toml::to_string(&ri)`: the latter fails
    //   `ValueAfterTable` on this struct (the same note `tests/fullreturn_oracle.rs:72` carries).
    let value = toml::Value::try_from(&ri)
        .expect("ReturnInputs → toml::Value (income scrub serializes the same type)");
    assert_open_keyed_paths_exist(&value);
    assert_no_empty_containers(&value, "");
    value
}

/// The complete key inventory, derived from the type: `(key path, TOML type)`, sorted.
fn inventory_of(value: &toml::Value) -> BTreeMap<String, &'static str> {
    let mut out = BTreeMap::new();
    // The root table itself is not a key.
    let toml::Value::Table(t) = value else {
        panic!("a serialized ReturnInputs is a TOML table");
    };
    for (k, child) in t {
        walk(child, k, &mut out);
    }
    out
}

/// [`inventory_of`] over [`serialized`] — the one-call form the tests use.
#[cfg(test)]
fn inventory() -> BTreeMap<String, &'static str> {
    inventory_of(&serialized())
}

/// Generate `docs/income-import-schema.md`.
#[must_use]
pub fn generate() -> String {
    let value = serialized();
    let inv = inventory_of(&value);
    let required = required_paths(&value, &inv);
    let dates = date_paths(&value, &inv);

    // ★★★ **THE ROWS ARE BUILT FIRST, AND EVERY COUNT IN THE PROSE IS COUNTED OFF THEM.**
    //
    // The first draft wrote the header count from `inv.len()` and then rendered a table that SKIPS a
    // date's `[]` element row — so the document announced "379 paths" above a table of 365. A
    // published schema whose own two halves disagree is the *"a tool claims something it did not do"*
    // class, in the one artifact a filer has no other way to check.
    let rows: Vec<(&String, &'static str, String)> = inv
        .iter()
        .filter(|(path, _)| {
            // A date's own `[]` element row (`…date_of_birth[]` | integer) is an artefact of the
            // compact serialized form, not a key. Once the parent is published as a `date` it is
            // noise, so it is dropped from the table AND from every count.
            !path
                .strip_suffix("[]")
                .is_some_and(|parent| dates.contains(parent))
        })
        .map(|(path, kind)| {
            let kind = if dates.contains(path) { "date" } else { *kind };
            let mut notes: Vec<&str> = Vec::new();
            if required.contains(path) {
                notes.push("**required**");
            }
            let n = note_for(path);
            if !n.is_empty() {
                notes.push(n);
            }
            (path, kind, notes.join("; "))
        })
        .collect();
    let leaf_keys = rows.iter().filter(|(_, kind, _)| *kind != "table").count();
    let mut d = String::new();

    writeln!(d, "# The `btctax income import` TOML schema").unwrap();
    writeln!(d).unwrap();
    writeln!(
        d,
        "**GENERATED — do not edit `{DOC_PATH}`.** Regenerate it with:\n\n```\n{REGEN}\n```\n\n\
         `xtask::toml_schema::tests::the_committed_schema_matches_a_fresh_generation` reds when this \
         file is stale, so a field added to `ReturnInputs` cannot reach a release unpublished."
    )
    .unwrap();
    writeln!(d).unwrap();

    // ── What the file is ────────────────────────────────────────────────────────────────────────
    writeln!(d, "## What this file is").unwrap();
    writeln!(d).unwrap();
    writeln!(
        d,
        "`btctax income import --year <Y> --file <f.toml>` reads one TOML file describing a whole \
         federal return — the household, every document transcribed off paper, the schedules, and the \
         payments. It is the authoring surface for anything the interview (`btctax income answer`) \
         cannot express, and the surface every *\"use `income import` instead\"* refusal points at.\n\n\
         Start from [the worked example](#a-complete-worked-example) below and delete what does not \
         apply to you.\n\n\
         **Almost every key is optional to the PARSER** — `ReturnInputs` carries `#[serde(default)]` \
         on nearly every field — so a short file parses. {required_sentence}\n\
         A key marked **required** in the table below cannot be omitted while its parent table is \
         present; leaving it out is a TOML parse error, not a refusal, so it is reported differently \
         and earlier — and note how many of them are `[[table]]` row fields, which is what makes \
         *\"required when its parent is present\"* the load-bearing qualifier: a return with no W-2 \
         needs none of the `w2s[]` keys.\n\n\
         What decides whether the return is COMPLETE is a separate gate, which refuses at import time \
         with a named reason and the form's own words — see \
         [Refusals you should expect](#refusals-you-should-expect). Parsing and completeness are two \
         different instruments and this document does not conflate them.",
        required_sentence = if required.is_empty() {
            "No key is required by the parser today.".to_string()
        } else {
            format!(
                "**{} are not**, and they are measured rather than remembered — each one was deleted \
                 from a complete file and handed back to the deserializer:\n\n{}\n",
                required.len(),
                required
                    .iter()
                    .map(|p| format!("- `{p}`"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    )
    .unwrap();
    writeln!(d).unwrap();

    // ── The traps ───────────────────────────────────────────────────────────────────────────────
    writeln!(d, "## Three things that will cost you time").unwrap();
    writeln!(d).unwrap();
    writeln!(
        d,
        "**1. A bare key written after a `[[table]]` header belongs to THAT TABLE.** This is TOML's \
         own rule, not a btctax quirk, and it is the easiest way to lose an answer:\n\n\
         ```toml\n\
         [[form_1098]]\n\
         lender = \"Example Bank\"\n\
         box1_interest = \"12400\"\n\
         \n\
         # WRONG — this is now `form_1098.0.claiming_mortgage_interest_credit`,\n\
         # a key that does not exist, and the import is REFUSED by name.\n\
         claiming_mortgage_interest_credit = false\n\
         ```\n\n\
         A return-level or section-level key must appear **before** the first `[table]`/`[[table]]` \
         header that follows it, or under the table it actually belongs to. btctax catches this one \
         rather than silently dropping it (trap 3), but only once you have run the import.\n\n\
         **2. The document census must agree with the rows you supplied.** `[documents]` is one \
         answer per document type, and it is not a derivation: `w2s` being empty means either *no \
         W-2* or *nobody asked*, and those are the same blank on a printed page. So `int_1099 = \
         false` beside a `[[int_1099]]` row is refused as a **contradiction** — btctax cannot know \
         which of the two is wrong, so it refuses rather than choose. Answer every row of \
         `[documents]` explicitly, including the `false`s: a recorded \"none\" is the honest record.\n\n\
         **3. An unknown key is REFUSED, by name — it is never ignored.** `income import` runs \
         `serde_ignored` over the same serde shape this document is generated from, so a typo, or a \
         field renamed or removed in a later version, stops the import and is printed:\n\n\
         ```\n\
         unknown key(s) in the ReturnInputs TOML: form_1098.0.claiming_mortgage_interest_credit. \
         btctax does not honor these — likely a typo or a field removed in this version … a \
         silently-ignored key would drop data you meant to enter.\n\
         ```\n\n\
         That is the behaviour to rely on: if the import does not complain about a key, every key in \
         your file was read."
    )
    .unwrap();
    writeln!(d).unwrap();

    // ── Refusals ────────────────────────────────────────────────────────────────────────────────
    writeln!(d, "## Refusals you should expect").unwrap();
    writeln!(d).unwrap();
    writeln!(
        d,
        "An import that stops has written nothing. Each refusal names its reason and quotes the \
         form:\n\n\
         - a **census contradiction** (trap 2 above);\n\
         - an **unanswered declaration** — a yes/no box with no safe default. Mortgage interest with \
         `claiming_mortgage_interest_credit` left out is refused with Schedule A's own Line 8a Caution \
         quoted and cited; guessing `false` would print an unsubtracted line 8a the filer never \
         affirmed. (Note the path: it is a **return-level** key, not a `schedule_a` one — the document \
         arrives whether or not you itemize — so writing it after `[[form_1098]]` or `[schedule_a]` \
         hits trap 1.) Answer these in the TOML, or run `btctax income answer --year <Y>`;\n\
         - a **year whose return cannot compute yet** is NOT refused — it is stored with a note, so \
         `report --write-carryover` can write onto it before that year's package exists.\n\n\
         Two key groups are read and then **normalised away**, with a note on stderr rather than a \
         refusal, because `income scrub` emits them and btctax must be able to read a file it \
         writes:\n\n\
         - `answer_log` / `answer_log_history` — btctax's record of *when, and in what words, it \
         asked you*. A hand-written record would be provenance for an act this vault never observed, \
         so the import discards what the file carried (announcing how many) and keeps whatever is \
         already on the row. Answer the questions with `income answer` instead.\n\
         - every `*_provenance` key — forced to `user`. A carryover the file supplies is the filer's, \
         never btctax's own `computed` authorship."
    )
    .unwrap();
    writeln!(d).unwrap();

    // ── The inventory ───────────────────────────────────────────────────────────────────────────
    writeln!(d, "## Every key `income import` honours").unwrap();
    writeln!(d).unwrap();
    writeln!(
        d,
        "**{} paths, {} of them leaves that take a value.** Derived from the serialized shape of \
         `ReturnInputs` over `btctax_core::tax::scrub_axis::maximal_sentinel()` — the fixture whose \
         every `Option` is `Some`, every `Vec` non-empty and every nested struct present, written as \
         an exhaustive `..`-free struct literal so a new field is a compile error before it can be an \
         unpublished key.\n\n\
         **Reading the paths.** `a.b` is the key `b` under `[a]`. `a[]` is a repeated table, written \
         `[[a]]` once per row, and `a[].b` is a key inside one of those rows. A `<placeholder>` \
         segment is a key YOU choose, not a literal:\n",
        rows.len(),
        leaf_keys
    )
    .unwrap();
    for (path, ph, what) in OPEN_KEYED {
        writeln!(d, "- `{path}.{ph}` — {what}.").unwrap();
    }
    writeln!(d).unwrap();
    writeln!(
        d,
        "**Money is a string.** Every dollar figure is a decimal serialized as a quoted string — \
         `\"12400\"`, `\"1234.56\"` — never a bare number, because a TOML float cannot carry a cent \
         exactly.\n\n\
         **A DATE takes EITHER spelling, and the {n_dates} `date` leaves below were found by trying \
         one.** Write `date_of_birth = \"2012-04-15\"` — that is what the deserializer accepts and what \
         you should type. btctax's own `income scrub` writes the same value as `time`'s compact \
         `[2012, 106]` (year, ordinal day), which also reads back, so a scrubbed file round-trips \
         without editing. Each of those leaves was identified by substituting an ISO date string at \
         the path and re-parsing the whole file. A leaf still shown below as `array of integers` was \
         NOT confirmed that way — every one of them is under `answer_log*`, which the import discards \
         anyway, and the generator fails if such a leaf ever turns up anywhere else.\n\n\
         **The last column** is `required` when omitting the key breaks the parse (measured, see \
         above), plus what the import does with it BEYOND storing it (derived from the path). Blank \
         means optional and stored as given.",
        n_dates = dates.len()
    )
    .unwrap();
    writeln!(d).unwrap();

    writeln!(d, "| key | TOML type | notes |").unwrap();
    writeln!(d, "|---|---|---|").unwrap();
    for (path, kind, notes) in &rows {
        writeln!(d, "| `{path}` | {kind} | {notes} |").unwrap();
    }
    writeln!(d).unwrap();

    // ── The worked example ──────────────────────────────────────────────────────────────────────
    writeln!(d, "## A complete worked example").unwrap();
    writeln!(d).unwrap();
    writeln!(
        d,
        "A married-filing-jointly household with wages, interest, dividends, an itemized Schedule A \
         and Bitcoin dispositions. This is journey J10's fixture \
         (`btctax_cli::testonly::J10_FULLRETURN_TOML`), reproduced VERBATIM — the same bytes \
         `docs/examples/examples.md` drives through a real `income import`, so it is a file known to \
         work rather than one that reads as if it should.\n\n\
         ★ Note that it answers **every** row of `[documents]`, including the `false`s.\n\n\
         ```toml"
    )
    .unwrap();
    d.push_str(btctax_cli::testonly::J10_FULLRETURN_TOML.trim_end());
    writeln!(d).unwrap();
    writeln!(d, "```").unwrap();

    d
}

/// Print the document to stdout (the generator's only operator-facing mode).
pub fn run() {
    print!("{}", generate());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **THE STALENESS GATE.** The committed document matches a fresh generation, byte for byte.
    ///
    /// Same shape as `examples::tests::examples_golden_matches_committed` and the CI `git diff
    /// --exit-code` step beside it: adding a field to `ReturnInputs` reds this test, and the only way
    /// to green it is to regenerate — which publishes the key.
    #[test]
    fn the_committed_schema_matches_a_fresh_generation() {
        let path = workspace_root().join(DOC_PATH);
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "committed {} missing ({e}); regenerate: {REGEN}",
                path.display()
            )
        });
        assert_eq!(
            generate(),
            committed,
            "{DOC_PATH} is STALE; regenerate with `{REGEN}`"
        );
    }

    /// The inventory reaches the keys FR-203 was filed about — the payment figures no document
    /// reports, the Schedule A declaration whose omission is refused with the Line 8a Caution, and
    /// the census rows that must agree with the transcribed rows.
    ///
    /// ★ A spot check on the DERIVATION, not a second copy of it: a walk that silently stopped
    ///   descending (an early `return`, a pruned branch) would leave the inventory looking plausible,
    ///   and this is what notices.
    #[test]
    fn the_inventory_publishes_the_keys_the_refusals_point_at() {
        let inv = inventory();
        for key in [
            "payments.estimated_tax_payments",
            "payments.extension_payment",
            "payments.other_withholding",
            // ★ RETURN-LEVEL, not a `schedule_a` leaf. This test is how that was found: the prose
            //   beside it asserted `schedule_a.claiming_mortgage_interest_credit`, from reading the
            //   field's doc comment instead of the derived inventory.
            "claiming_mortgage_interest_credit",
            "documents.int_1099",
            "form_1098[].box1_interest",
            "w2s[].box1_wages",
            // ★★ The two keys the FIRST generation silently omitted, because the maximal fixture
            //    leaves `broker_reporting` empty on purpose. See `augmented_sentinel`.
            "broker_reporting.<provider>.covered",
            "broker_reporting.<provider>.noncovered",
        ] {
            assert!(
                inv.contains_key(key),
                "the derived inventory is missing {key:?} — {} keys published:\n{:#?}",
                inv.len(),
                inv.keys().collect::<Vec<_>>()
            );
        }
    }

    /// ★★★ **B1 — the pruning declaration is observed DISCRIMINATING.** A sentinel provider name is a
    /// filer VALUE; publishing it as a schema key is the defect [`OPEN_KEYED`] exists to stop. This
    /// asserts the placeholder actually replaced it, so an entry that prunes nothing cannot pass.
    #[test]
    fn an_open_keyed_tables_sentinel_key_is_not_published_as_schema() {
        let inv = inventory();
        let leaked: Vec<&String> = inv
            .keys()
            .filter(|k| {
                k.contains("SENTINEL") || k.contains("question:") || k.contains("skippable:")
            })
            .collect();
        assert!(
            leaked.is_empty(),
            "the inventory published {} FILER VALUE(S) as schema keys: {leaked:#?} — an OPEN_KEYED \
             entry is missing or stale",
            leaked.len()
        );
    }

    /// ★★★ **The document's claim about DATES, machine-checked rather than remembered.**
    ///
    /// `time::Date`'s serde form here is the compact `[year, ordinal]` tuple — the crate's
    /// `serde-human-readable` feature is not enabled — so a filer hand-writing a TOML must write
    /// `[2012, 106]`, and the `"YYYY-MM-DD"` spelling everyone reaches for first is a parse error.
    /// Both spellings are driven through the real deserializer here, so the paragraph in the
    /// generated document cannot quietly become false if that feature is ever switched on.
    #[test]
    fn either_date_spelling_parses_and_the_serialized_form_is_the_tuple() {
        use btctax_core::tax::return_inputs::ReturnInputs;
        // ★ The minimum that isolates the DATE is `filing_status` plus a whole person — and those are
        //   exactly the keys `required_paths` derives, so this fixture is the derivation's own answer
        //   rather than a guess. (The first draft wrote `[header.taxpayer]` alone and this test is
        //   what reported the generated document's claim that *every* key was optional.)
        let person = |dob: &str| {
            format!(
                "filing_status = \"Single\"\n\
                 [header.taxpayer]\nfirst_name = \"A\"\nlast_name = \"B\"\nssn = \"000-11-1111\"\n\
                 date_of_birth = {dob}\n"
            )
        };
        let ordinal: ReturnInputs = toml::from_str(&person("[1970, 1]"))
            .expect("`[year, ordinal]` is the accepted date spelling");
        assert_eq!(
            ordinal
                .header
                .taxpayer
                .date_of_birth
                .map(|d| d.to_string())
                .as_deref(),
            Some("1970-01-01"),
            "`[1970, 1]` is 1 January 1970"
        );
        let iso: ReturnInputs = toml::from_str(&person("\"1970-01-01\""))
            .expect("the ISO spelling is the one the document tells a filer to write");
        assert_eq!(
            iso.header.taxpayer.date_of_birth, ordinal.header.taxpayer.date_of_birth,
            "both spellings must name the same day, or the document's \"either spelling\" \
             paragraph is wrong"
        );
        // …and the SERIALIZED form is the tuple, which is why the raw inventory types a date as an
        // `array of integers` and why `date_paths` has to ask rather than read the type name.
        let back = toml::Value::try_from(&ordinal).expect("serializes");
        assert_eq!(
            back["header"]["taxpayer"]["date_of_birth"].type_str(),
            "array",
            "if `time` ever serializes a human-readable date here, `date_paths` and the document's \
             `income scrub` sentence both need re-reading"
        );
    }

    /// ★★★ **B1 — EVERY KEY NAMED IN THE PROSE IS A KEY THE INVENTORY PUBLISHES.**
    ///
    /// The inventory is derived and therefore cannot be wrong; the **prose around it** is hand-written
    /// and was wrong twice in one sitting, both times by describing a field from its Rust doc comment
    /// instead of from the derivation:
    ///
    /// - `schedule_a.claiming_mortgage_interest_credit` — the key is **return-level**;
    /// - `form_1098[].box1_mortgage_interest` — the key is `box1_interest`.
    ///
    /// A schema document whose explanatory text names keys that do not exist is worse than no
    /// document, because the reader has no way to tell which half to trust. So the prose is checked
    /// against the derivation. Fenced code blocks are excluded on purpose: the trap-1 example is
    /// *deliberately* a misplaced key, and the worked example is already driven through a real `income
    /// import` by `docs/examples/examples.md`.
    #[test]
    fn every_key_named_in_the_prose_exists_in_the_derived_inventory() {
        /// Backticked spans that look like keys but are not: the `a.b` path-reading illustration, the
        /// enum spellings a value takes, the column words, and two crate names.
        const ALLOWED: &[&str] = &[
            "a.b",
            "a[]",
            "a[].b",
            "b",
            "computed",
            "user",
            "date",
            "false",
            "required",
            "serde_ignored",
            "time",
        ];
        let inv = inventory();
        let doc = generate();

        // Prose only: drop ``` fences and the inventory table's own rows.
        let mut prose = String::new();
        let mut in_fence = false;
        for line in doc.lines() {
            if line.starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence || line.starts_with("| ") {
                continue;
            }
            prose.push_str(line);
            prose.push('\n');
        }

        // Every backticked span, plus every `token = ` spelling (the date paragraph writes one).
        let mut candidates: BTreeSet<String> = prose
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect();
        for (i, _) in prose.match_indices(" = ") {
            let head = &prose[..i];
            let word = head
                .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default();
            if !word.is_empty() {
                candidates.insert(word.to_string());
            }
        }

        let key_shaped = |t: &str| {
            !t.is_empty()
                && t.starts_with(|c: char| c.is_ascii_lowercase())
                && t.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.[]<>".contains(c))
        };
        let leaf_names: BTreeSet<&str> = inv
            .keys()
            .filter_map(|k| k.rsplit('.').next())
            .filter(|s| !s.ends_with("[]"))
            .collect();

        let bad: Vec<&String> = candidates
            .iter()
            .filter(|t| key_shaped(t) && !ALLOWED.contains(&t.as_str()))
            .filter(|t| {
                if inv.contains_key(t.as_str()) {
                    return false;
                }
                // A bare single-segment name in prose (`date_of_birth`) is legitimate when it is the
                // LEAF of some published path; a typo of one is not.
                let single = !t.contains('.') && !t.contains("[]");
                !(single && leaf_names.contains(t.as_str()))
            })
            .collect();

        assert!(
            bad.is_empty(),
            "the generated document's PROSE names {} key(s) the derived inventory does not publish: \
             {bad:#?}\n★ Either the key was renamed, or the prose was written from a doc comment \
             instead of from the derivation — which is how `box1_mortgage_interest` and \
             `schedule_a.claiming_mortgage_interest_credit` both got written. Fix the prose in \
             `crates/xtask/src/toml_schema.rs`, or add a genuinely non-key word to ALLOWED.",
            bad.len()
        );
    }

    /// ★★★ **The COUNT the prose announces equals the ROWS the table prints.**
    ///
    /// It did not. The header read `inv.len()` while the table skipped each date's `[]` element row, so
    /// the document said *"379 paths"* above a table of 365 — a schema disagreeing with itself, in the
    /// one artifact a filer has no other way to check. Both now come off the same `rows` vector; this is
    /// what reds if they are ever computed from two places again.
    #[test]
    fn the_announced_path_count_is_the_number_of_rows_printed() {
        let doc = generate();
        let rows = doc.lines().filter(|l| l.starts_with("| `")).count();
        let announced: usize = doc
            .split("**")
            .find_map(|seg| {
                seg.strip_suffix(" paths, ")
                    .or_else(|| seg.split_once(" paths, ").map(|(n, _)| n))
            })
            .and_then(|n| n.trim().parse().ok())
            .expect("the inventory heading announces a path count");
        assert_eq!(
            announced, rows,
            "the document announces {announced} paths and prints {rows} rows — one of the two counts \
             is computed from something other than what is published"
        );
    }

    /// The document carries the worked example verbatim, not a re-typed copy of it.
    #[test]
    fn the_worked_example_is_the_fixture_byte_for_byte() {
        let doc = generate();
        assert!(
            doc.contains(btctax_cli::testonly::J10_FULLRETURN_TOML.trim_end()),
            "the schema document does not carry J10_FULLRETURN_TOML verbatim"
        );
    }
}
