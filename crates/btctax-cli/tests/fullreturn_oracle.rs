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
///
/// ★★★ **THE THREE THINGS THE FIXTURE USED TO SAY ABOUT ITSELF, MOVED HERE** (seam review M2). They
/// were hand-added comment blocks inside a file whose own first line says *"GENERATED — do not
/// hand-edit"*, so the next reader to follow that instruction would have stripped them silently. A
/// generated file cannot hold its own annotations; the annotations belong beside the constant that
/// names it. Each is a live refusal, not a remark:
///
/// - `charitable_cwa_obtained = true` — §170(f)(8): this household holds a contemporaneous written
///   acknowledgment for every gift of $250 or more that this return deducts. **Deleting the line
///   refuses the return** (`CharitableCwaUnresolved`), and so does setting it `false`: the crypto
///   donation is far over $250 and the return itemizes.
/// - `filing_form_4952 = false` — Schedule D line 20 / Schedule A line 9: this household is not
///   filing Form 4952, so line 20 is checked "Yes" (the Qualified Dividends and Capital Gain Tax
///   Worksheet). **Deleting the line refuses the return** (`Form4952DeclarationUnanswered`); `true`
///   refuses it as `Form4952Required`.
/// - `schedule_a.mortgage_within_debt_limit = true` — §163(h)(3)(B): inside every i1040sca "Limits on
///   home mortgage interest" ceiling, so line 8a keeps the whole Form 1098 amount. **Deleting the
///   line refuses the return** (`MortgageDebtLimitUnanswered`), and `false` refuses it as
///   `MortgageOverDebtLimit`. That is the point.
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

/// The emitter, as a FUNCTION rather than a test body — so the gate below can run it in memory and
/// compare, instead of the header asserting a property nothing checks (seam review M2).
///
/// `toml::Value::try_from` THEN string, NOT `toml::to_string(&ri)`: the latter fails `ValueAfterTable`
/// (a scalar field declared after a table field is unrepresentable in streaming TOML). Building the
/// `Value` tree first lets the Value serializer emit tables last. And `to_string`, not
/// `to_string_pretty`: compact inline arrays (`date_of_birth = [2012, 106]`, `box12 = []`) instead of
/// the pretty serializer's multi-line ones — less diff noise, same bytes on every regen.
fn emit_fullreturn_fixture_text() -> String {
    let ri = kitchen_sink_household().0;
    let value = toml::Value::try_from(&ri).expect("ReturnInputs → toml::Value");
    format!(
        "{FIXTURE_BANNER}{}",
        toml::to_string(&value).expect("toml::Value → TOML text")
    )
}

/// ★★★ **THE FIXTURE MATCHES ITS EMITTER — the test the `GENERATED` banner was asserting on its
///     own** (seam review M2).
///
/// `fullreturn_fixture_is_the_kitchen_sink_oracle` above cannot see this drift: it compares the
/// PARSED fixture to the oracle value, and every `ReturnInputs` field is `#[serde(default)]`, so a key
/// the schema gained since the last regen is simply absent from the file, defaults on both sides, and
/// compares equal. It passed identically with the committed file and with a regenerated one — which
/// is the definition of an instrument that cannot discriminate.
///
/// This one compares BYTES, which is the only thing that makes *"do not hand-edit"* true. Its sibling
/// generated artefact (`docs/examples/examples.md`) has had exactly this test all along
/// (`xtask::examples::tests::examples_golden_matches_committed`).
#[test]
fn fullreturn_fixture_matches_its_emitter() {
    assert_eq!(
        FIXTURE,
        emit_fullreturn_fixture_text(),
        "the committed fullreturn_inputs.toml is not what its emitter produces — either it was \
         hand-edited (the banner forbids it; put the annotation beside `FIXTURE` in this file \
         instead) or the schema moved under it. Regenerate with `cargo test -p btctax-cli --test \
         fullreturn_oracle -- --ignored emit_fullreturn_fixture`."
    );
}

/// Regeneration helper (not a gate — `#[ignore]`). Writes what `emit_fullreturn_fixture_text` emits.
#[test]
#[ignore = "regeneration helper: rewrites the committed fullreturn_inputs.toml from the oracle"]
fn emit_fullreturn_fixture() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/examples/fullreturn_inputs.toml"
    );
    std::fs::write(path, emit_fullreturn_fixture_text()).expect("write the committed fixture");
    eprintln!("wrote {path}");
}
