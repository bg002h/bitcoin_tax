//! ★★★ **FR-200(b) — the SPELLING a filer actually types for a Form 8283 Section A gift.**
//!
//! `docs/income-import-schema.md` publishes the keys, and `income import`'s deserializer is what has to
//! accept them — including the **ISO date** spelling, which a JSON `TaxDate` does not take (it wants
//! `time`'s compact `[year, ordinal]`). A schema document asserting a spelling the import does not
//! accept is the *"correct derived document describing behaviour the code had not implemented"* shape
//! this repo has been bitten by once already, on `CarryProvenance`.
//!
//! ★ The end-to-end half — that a $600 bag of clothes no longer refuses the whole packet — lives in
//! `export_irs_pdf.rs`, beside the refusal it is the mirror of, because it needs that file's vault
//! harness.

/// The TOML a filer writes for a thrift-store donation, in the spelling
/// `docs/income-import-schema.md` publishes.
const GIFT_TOML: &str = r#"
class = "cap_gain_prop30"
amount = "600"

[noncash]
donee_name_and_address = "Goodwill Industries, 1 Main St, Springfield IL 62701"
description_and_condition = "Bag of used adult clothing (12 shirts, 4 pairs of trousers), good used condition"
date_of_contribution = "2024-11-30"
date_acquired_by_donor = "various"
how_acquired_by_donor = "Purchase"
cost_or_adjusted_basis = "2400"
fair_market_value = "600"
method_used_to_determine_fmv = "Thrift shop value"

[noncash.kind.clothing_or_household_item]
good_used_condition_or_better = true
"#;

type Gift = btctax_core::tax::return_inputs::CharitableGift;

/// ★★★ **The published spelling parses, ISO date and all.**
///
/// **Planted-defect check (B1), observed RED:** rename any key in `GIFT_TOML` to the name it would have
/// had under a different field spelling — e.g. `fmv_method` for `method_used_to_determine_fmv` — and
/// this reds with serde's *"missing field"*. That is the honest kill for "the document publishes a key
/// the code does not read".
#[test]
fn the_published_toml_spelling_parses_including_the_iso_date() {
    use btctax_core::tax::form8283_section_a::{DateAcquiredByDonor, NoncashPropertyKind};
    let gift: Gift = toml::from_str(GIFT_TOML).expect("the published spelling parses");
    let p = gift.noncash.as_ref().expect("the noncash block is present");
    assert_eq!(
        p.kind,
        NoncashPropertyKind::ClothingOrHouseholdItem {
            good_used_condition_or_better: true
        }
    );
    assert_eq!(
        p.date_of_contribution,
        time::macros::date!(2024 - 11 - 30),
        "the ISO date spelling a filer types — `docs/income-import-schema.md`'s \"a DATE takes EITHER \
         spelling\" has to be true of the TOML deserializer, not only of `time`'s compact form"
    );
    assert_eq!(
        p.date_acquired_by_donor,
        Some(DateAcquiredByDonor::Various),
        "column (e) takes i8283's own word for a group of similar items"
    );
    assert_eq!(p.method_used_to_determine_fmv, "Thrift shop value");
    assert_eq!(
        p.cost_or_adjusted_basis,
        Some(rust_decimal_macros::dec!(2400))
    );
}

/// …and the whole block round-trips, so a scrubbed or re-exported file reads back.
#[test]
fn the_published_toml_spelling_round_trips() {
    let gift: Gift = toml::from_str(GIFT_TOML).expect("parses");
    let p = gift.noncash.clone().expect("present");
    let back: btctax_core::tax::form8283_section_a::NoncashGiftProperty =
        toml::from_str(&toml::to_string(&p).expect("serializes")).expect("round-trips");
    assert_eq!(back, p);
}

/// ★ Every key the schema document publishes under the noncash block is one this deserializer reads.
/// Derived from the committed document, so a key published and never read reds — the FR-196a shape.
#[test]
fn every_published_noncash_key_is_read_by_the_deserializer() {
    let doc = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/income-import-schema.md"),
    )
    .expect("the committed schema document");
    let prefix = "schedule_a.charitable[].noncash";
    // The document's table rows are `| `path` | type | note |`.
    let published: Vec<String> = doc
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .filter(|p| p.starts_with(prefix))
        .map(String::from)
        .collect();
    assert!(
        published.len() >= 9,
        "the document publishes only {} keys under {prefix} — it has gone stale, which would make \
         this assertion vacuous: {published:?}",
        published.len()
    );

    // A leaf key the deserializer reads is one the TOML above sets, or one under `kind`/`date_acquired_
    // by_donor` (enum-tagged sub-tables), or the block itself.
    let toml_value: toml::Value = toml::from_str(GIFT_TOML).expect("parses");
    let set_keys: Vec<String> = toml_value["noncash"]
        .as_table()
        .expect("the noncash table")
        .keys()
        .map(|k| format!("{prefix}.{k}"))
        .collect();
    for key in &published {
        let covered = key == prefix
            || set_keys.iter().any(|s| s == key)
            || set_keys.iter().any(|s| key.starts_with(&format!("{s}.")));
        assert!(
            covered,
            "`{key}` is published in docs/income-import-schema.md but the worked TOML in this test \
             does not exercise it — either the document publishes a key nothing reads, or this test \
             has stopped covering the surface it claims to. Keys exercised: {set_keys:?}"
        );
    }
}
