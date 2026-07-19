//! Task 2.1 — the forms-coverage census (HARD gate; SPEC §6.1/§6.2).
//!
//! Two guarantees, kept in the crate that owns `PrintedReturn`/`fill_full_return` (N2 — no xtask→core dep):
//!
//! 1. **`census_key_set_is_exactly_14`** — pushing an ALL-ARMS `PrintedReturn` through `fill_full_return`
//!    emits EXACTLY the §6.1 literal 14 form-name keys. A new form (packet.rs destructures `PrintedForms`
//!    with no `..`, so a new arm without a filler is already a compile error) or a renamed stem reds here.
//!    SPEC §6.2 forbids reading a *household's* packet as the authority — kitchen_sink emits 13/14, which
//!    would silently under-gate — so this fixture injects the 14th (`f8283`) arm explicitly.
//!
//! 2. **`every_census_form_demonstrated_in_j6`** — every one of those 14 keys appears in the committed
//!    golden's **J6 full-return packet manifest ONLY** (`{seq}_{name}.pdf`, exact `{name}` match). Never a
//!    corpus-wide scan: three crypto-slice stems (`f8949`/`schedule_d`/`schedule_se`) are byte-identical to
//!    census keys and would re-attribute slice output to the census (admin.rs seq-prefixes the packet
//!    precisely because they collided).

use btctax_core::forms::{Form8283HowAcquired, Form8283Row, Form8283Section};
use btctax_core::tax::packet::{assemble_printed_return, PrintedReturn};
use btctax_core::tax::printed::form_8283_printed;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};
use btctax_forms::fill_full_return;
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::date;

/// The §6.1 census key set — the 14 forms `fill_full_return` can emit — in ONE place. Schedule D/SE use
/// bare `schedule_d`/`schedule_se`; the numbered schedules use `f1040s{1,2,3,a,b,c}`.
const CENSUS_KEYS: [&str; 14] = [
    "f1040",
    "f1040s1",
    "f1040s2",
    "f1040s3",
    "f1040sa",
    "f1040sb",
    "f1040sc",
    "schedule_d",
    "f8949",
    "schedule_se",
    "f8995",
    "f8959",
    "f8960",
    "f8283",
];

/// A minimal, well-formed Section-A Form 8283 row. The census only needs the `f8283` filler to return
/// `Some` (the double-gate at packet.rs:155); a Section-A gift (≤ $5,000, no appraiser declaration
/// required) fills cleanly with no `DonationDetails`.
fn injected_8283_row() -> Form8283Row {
    Form8283Row {
        section: Some(Form8283Section::A),
        description: "0.05000000 BTC".to_string(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2020 - 01 - 01),
        date_contributed: date!(2024 - 09 - 01),
        cost_basis: dec!(400),
        fmv: dec!(600),
        claimed_deduction: Some(dec!(600)),
        fmv_method: String::new(),
        donee: "Test Charity".to_string(),
        appraiser: String::new(),
        needs_review: true,
        details: None,
    }
}

/// An ALL-ARMS `PrintedReturn`: every optional arm populated AND the three non-`Option` gates satisfied
/// (`sch_d.must_file`, `f8959` internal `must_file`, `f8283` filler → `Some`). kitchen_sink assembles
/// 13/14 (everything but `f8283` — its charity is cash, not a noncash ledger gift, and there is no CLI
/// path to a donating `LedgerState`), so inject the 14th arm.
fn all_arms_return() -> PrintedReturn {
    let (ri, state) = kitchen_sink_household();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &table, 2024);
    let details: BTreeMap<_, _> = BTreeMap::new();
    let mut pr = assemble_printed_return(&ri, &state, &details, &ar, &table, 2024)
        .expect("kitchen_sink assembles");
    assert!(
        pr.forms.f8283.is_none(),
        "premise: kitchen_sink is 13/14 (no f8283) — if this fires the fixture assumption changed"
    );
    pr.forms.f8283 = form_8283_printed(&[injected_8283_row()]);
    assert!(
        pr.forms.f8283.is_some(),
        "the injected f8283 arm must be Some"
    );
    pr
}

#[test]
fn census_key_set_is_exactly_14() {
    let pr = all_arms_return();
    let forms = fill_full_return(&pr, 2024).expect("the all-arms packet must fill");
    let emitted: BTreeSet<&str> = forms.iter().map(|f| f.name.as_str()).collect();
    let expected: BTreeSet<&str> = CENSUS_KEYS.iter().copied().collect();
    assert_eq!(
        forms.len(),
        14,
        "fill_full_return must emit EXACTLY 14 forms; got {} ({emitted:?})",
        forms.len()
    );
    assert_eq!(
        emitted, expected,
        "the emitted form-name set must equal the §6.1 census keys exactly — a difference is a new or \
         renamed form the census does not yet track"
    );
}

/// Read the committed golden (`docs/examples/examples.md`), two levels up from this crate.
fn golden() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/examples/examples.md"
    );
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("read {path}: {e} — regenerate with `cargo run -p xtask -- examples > docs/examples/examples.md`")
    })
}

/// The `{name}` stems of the sequence-prefixed `{seq}_{name}.pdf` files in **J6's full-return packet block
/// only** (SPEC §6.2). Bounded to the J6 section, then to the `Full-return packet —` block; a stem counts
/// only when the pre-`_` component starts with a digit (the IRS Attachment Sequence No.), which the bare
/// crypto-slice stems (`f8949.pdf`, …) do not have.
fn j6_packet_names(golden: &str) -> BTreeSet<String> {
    let j6_start = golden.find("## J6").expect("golden has a J6 section");
    let j6_body = &golden[j6_start..];
    // Bound to the J6 section (defensive against a future J7) — stop at the next "\n## " header.
    let end = j6_body[2..]
        .find("\n## ")
        .map(|i| i + 2)
        .unwrap_or(j6_body.len());
    let j6 = &j6_body[..end];
    let pkt_start = j6
        .find("Full-return packet —")
        .expect("J6 shows the `Full-return packet —` block");
    let mut names = BTreeSet::new();
    for line in j6[pkt_start..].lines() {
        let base = line.trim().rsplit('/').next().unwrap_or("");
        if let Some(stem) = base.strip_suffix(".pdf") {
            if let Some((seq, name)) = stem.split_once('_') {
                if seq.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names
}

#[test]
fn every_census_form_demonstrated_in_j6() {
    let names = j6_packet_names(&golden());
    let expected: BTreeSet<String> = CENSUS_KEYS.iter().map(|s| s.to_string()).collect();
    let missing: Vec<&String> = expected.difference(&names).collect();
    assert!(
        missing.is_empty(),
        "census forms undemonstrated in J6's packet: {missing:?} (present: {names:?})"
    );
    assert_eq!(
        names, expected,
        "J6's full-return packet stems must be EXACTLY the 14 census keys — got {names:?}"
    );
}
