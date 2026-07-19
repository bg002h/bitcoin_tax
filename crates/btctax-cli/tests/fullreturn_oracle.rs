//! I6 / M7 — the full-return oracle.
//!
//! J6 (the worked example that emits all 14 census forms) imports its non-crypto figures from a COMMITTED
//! TOML fixture, `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`. That fixture MUST be the
//! `btctax_core::tax::testonly::kitchen_sink_household()` oracle vector, verbatim — otherwise the doc
//! quietly grows a second, drifting source of truth for the same numbers.
//!
//! This test binds the two: the committed TOML must `toml::from_str` to exactly `kitchen_sink_household().0`.
//! A hand-edit that drifts the fixture reds HERE (the oracle crate owns the numbers), not silently in the
//! golden. `ReturnInputs` derives `Deserialize + PartialEq + Eq`, so the comparison is exact.
//!
//! The fixture is generated, never hand-authored — `kitchen_sink_household()` ends with
//! `answer_all_live_declarations`, which sets fields beyond the literal constructor, so the only reliable
//! source is the value itself. Regenerate with the ignored `emit_fullreturn_fixture` helper below.

use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::testonly::kitchen_sink_household;

/// The committed fixture J6 imports (`income import --year 2024 --file …`), read at COMPILE time. It lives
/// here in `btctax-cli/tests/` (not xtask) so the PUBLISHED crate's test is self-contained — the xtask
/// generator holds the cross-crate `include_str!` instead, since xtask is `publish = false` (M-5). The
/// equality assertion lives here (not xtask) because xtask must not gain a btctax-core dep (Task 1.2, N2).
const FIXTURE: &str = include_str!("fixtures/examples/fullreturn_inputs.toml");

#[test]
fn fullreturn_fixture_is_the_kitchen_sink_oracle() {
    let parsed: ReturnInputs = toml::from_str(FIXTURE)
        .expect("the committed fullreturn_inputs.toml parses as ReturnInputs");
    assert_eq!(
        parsed,
        kitchen_sink_household().0,
        "the committed J6 fixture must equal the kitchen_sink_household() oracle vector, verbatim — \
         regenerate it with `cargo test -p btctax-cli --test fullreturn_oracle -- --ignored \
         emit_fullreturn_fixture` if the oracle changed"
    );
}

/// Regeneration helper (not a gate — `#[ignore]`). Serializes the oracle vector to the committed fixture.
///
/// Via `toml::Value::try_from` THEN string, NOT `toml::to_string(&ri)`: the latter fails `ValueAfterTable`
/// (a scalar field declared after a table field is unrepresentable in streaming TOML). Building the
/// `Value` tree first lets the Value serializer emit tables last.
/// The generated-file banner prepended to the fixture. Kept in the emitter (not hand-added) so a regen
/// reproduces the committed bytes exactly — the fixture is idempotent under this helper.
const FIXTURE_BANNER: &str = "\
# GENERATED — do not hand-edit. This is `btctax_core::tax::testonly::kitchen_sink_household()`'s
# ReturnInputs (`.0`) serialized to TOML; the J6 worked example imports it via `btctax income import`.
# The oracle test `fullreturn_oracle::fullreturn_fixture_is_the_kitchen_sink_oracle` pins it == the
# oracle vector. Regenerate with:
#   cargo test -p btctax-cli --test fullreturn_oracle -- --ignored emit_fullreturn_fixture
# (keys sort alphabetically — that is the `toml::Value` table order, not a meaningful layout).

";

#[test]
#[ignore = "regeneration helper: rewrites the committed fullreturn_inputs.toml from the oracle"]
fn emit_fullreturn_fixture() {
    let ri = kitchen_sink_household().0;
    let value = toml::Value::try_from(&ri).expect("ReturnInputs → toml::Value");
    // `to_string` (not `to_string_pretty`): compact inline arrays (`date_of_birth = [2012, 106]`,
    // `box12 = []`) instead of the pretty serializer's multi-line arrays — less diff noise, same bytes
    // on every regen. NOT `toml::to_string(&ri)` (that fails ValueAfterTable) — serialize the Value.
    let text = format!(
        "{FIXTURE_BANNER}{}",
        toml::to_string(&value).expect("toml::Value → TOML text")
    );
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/examples/fullreturn_inputs.toml"
    );
    std::fs::write(path, text).expect("write the committed fixture");
    eprintln!("wrote {path}");
}
