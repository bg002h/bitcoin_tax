//! Task 2.1 — the forms-coverage census (HARD gate; SPEC §6.1/§6.2). Task 16 bumped it 14→15 with
//! Form 8275 (Disclosure Statement, Attachment Sequence No. 92).
//!
//! Two guarantees, kept in the crate that owns `PrintedReturn`/`fill_full_return` (N2 — no xtask→core dep):
//!
//! 1. **`census_is_exactly_15_forms_including_8275_when_a_promote_is_present`** — pushing an ALL-ARMS
//!    `PrintedReturn` through `fill_full_return` emits EXACTLY the §6.1 literal 15 form-name keys. A new
//!    form (packet.rs destructures `PrintedForms` with no `..`, so a new arm without a filler is already a
//!    compile error) or a renamed stem reds here. SPEC §6.2 forbids reading a *household's* packet as the
//!    authority — kitchen_sink emits 13/15 (no `f8283`, no promoted `f8275`), which would silently
//!    under-gate — so this fixture injects the 14th (`f8283`) AND 15th (`f8275`) arms explicitly.
//!
//! 2. **`every_census_form_demonstrated_in_j6`** — every one of those 15 keys appears in the committed
//!    golden's **J6 full-return packet manifest ONLY** (`{seq}_{name}.pdf`, exact `{name}` match). Never a
//!    corpus-wide scan: three crypto-slice stems (`f8949`/`schedule_d`/`schedule_se`) are byte-identical to
//!    census keys and would re-attribute slice output to the census (admin.rs seq-prefixes the packet
//!    precisely because they collided).

use btctax_core::conservative::Coverage;
use btctax_core::event::{Acknowledgment, BasisSource, DisposeKind, FloorMethod, PromoteTranche};
use btctax_core::forms::{Form8283HowAcquired, Form8283Row, Form8283Section};
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, Term};
use btctax_core::tax::form8275::Part1Item;
use btctax_core::tax::packet::{assemble_printed_return, PrintedReturn};
use btctax_core::tax::printed::{form_8283_printed, Printed8275};
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table, w2_only_household};
use btctax_core::EventPayload;
use btctax_forms::fill_full_return;
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::date;

/// The §6.1 census key set — the 15 forms `fill_full_return` can emit — in ONE place. Schedule D/SE use
/// bare `schedule_d`/`schedule_se`; the numbered schedules use `f1040s{1,2,3,a,b,c}`.
const CENSUS_KEYS: [&str; 15] = [
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
    "f8275",
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

/// A minimal, well-formed Form 8275 disclosure: one Part I item (a promoted disposal leg's estimated
/// basis) + a non-empty Part II narrative. The census only needs the `f8275` filler to return `Some`
/// (`printed.part_i` non-empty) — the exact figures are not otherwise asserted here.
fn injected_8275() -> Printed8275 {
    Printed8275 {
        part_i: vec![Part1Item {
            form: "8949".to_string(),
            line: "Part I \u{2014} column (e)".to_string(),
            description: "basis estimated at the minimum daily closing price over the attested \
                acquisition window (Cohan; the bearing-heavily minimum)"
                .to_string(),
            amount: dec!(300),
        }],
        part_ii: "cash P2P purchase, no records; window bounded on-chain".to_string(),
    }
}

/// An ALL-ARMS `PrintedReturn`: every optional arm populated AND the three non-`Option` gates satisfied
/// (`sch_d.must_file`, `f8959` internal `must_file`, `f8283` filler → `Some`, `f8275` non-empty Part I).
/// kitchen_sink assembles 13/15 (everything but `f8283` — its charity is cash, not a noncash ledger
/// gift, and there is no CLI path to a donating `LedgerState`; and `f8275` — its one disposal leg is not
/// a promoted origin), so inject the 14th and 15th arms.
fn all_arms_return() -> PrintedReturn {
    let (ri, state) = kitchen_sink_household();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &table, 2024);
    let details: BTreeMap<_, _> = BTreeMap::new();
    let mut pr = assemble_printed_return(&ri, &state, &details, &ar, &table, 2024, &[])
        .expect("kitchen_sink assembles");
    assert!(
        pr.forms.f8283.is_none(),
        "premise: kitchen_sink has no f8283 — if this fires the fixture assumption changed"
    );
    assert!(
        pr.forms.f8275.is_none(),
        "premise: kitchen_sink has no promoted disposal leg (no f8275) — if this fires the fixture \
         assumption changed"
    );
    pr.forms.f8283 = form_8283_printed(&[injected_8283_row()]);
    pr.forms.f8275 = Some(injected_8275());
    assert!(
        pr.forms.f8283.is_some(),
        "the injected f8283 arm must be Some"
    );
    assert!(
        pr.forms.f8275.is_some(),
        "the injected f8275 arm must be Some"
    );
    pr
}

#[test]
fn census_is_exactly_15_forms_including_8275_when_a_promote_is_present() {
    let pr = all_arms_return();
    let forms = fill_full_return(&pr, 2024).expect("the all-arms packet must fill");
    let emitted: BTreeSet<&str> = forms.iter().map(|f| f.name.as_str()).collect();
    let expected: BTreeSet<&str> = CENSUS_KEYS.iter().copied().collect();
    assert_eq!(
        forms.len(),
        15,
        "fill_full_return must emit EXACTLY 15 forms; got {} ({emitted:?})",
        forms.len()
    );
    assert_eq!(
        emitted, expected,
        "the emitted form-name set must equal the §6.1 census keys exactly — a difference is a new or \
         renamed form the census does not yet track"
    );
    let f8275 = forms
        .iter()
        .find(|f| f.name == "f8275")
        .expect("f8275 is in the packet");
    assert_eq!(
        f8275.attachment_sequence,
        Some("92"),
        "Form 8275's IRS Attachment Sequence No. is 92"
    );
}

/// The direct, non-injected wiring: a `PrintedReturn` assembled from a REAL promoted disposal leg (via
/// `assemble_printed_return` → core's `disclosure_8275`) fills an `f8275` at Attachment Sequence 92;
/// the SAME household with no promoted leg fills no `f8275` at all — the "iff" both directions.
#[test]
fn full_return_packet_emits_8275_iff_a_promoted_leg_is_filed() {
    let (ri, mut state) = w2_only_household();
    let table = ty2024_table();
    let params = ty2024_params();

    // WITHOUT a promoted leg: a plain 2024 disposal, ordinary basis, origin NOT in `promoted_origins`.
    let origin = EventId::decision(1);
    state.disposals.push(Disposal {
        event: EventId::decision(2),
        kind: DisposeKind::Sell,
        disposed_at: date!(2024 - 06 - 01),
        legs: vec![DisposalLeg {
            lot_id: LotId {
                origin_event_id: origin.clone(),
                split_sequence: 0,
            },
            sat: 1_000_000,
            proceeds: dec!(500),
            basis: dec!(300),
            gain: dec!(200),
            term: Term::ShortTerm,
            basis_source: BasisSource::ExchangeProvided,
            gift_zone: None,
            acquired_at: date!(2024 - 01 - 01),
            wallet: WalletId::SelfCustody {
                label: "cold".into(),
            },
            pseudo: false,
        }],
        fee_mini_disposition: false,
    });

    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    let pr_unpromoted = assemble_printed_return(
        &ri,
        &state,
        &BTreeMap::new(),
        &ar,
        &table,
        2024,
        &[],
    )
    .expect("the fixture assembles");
    assert!(
        pr_unpromoted.forms.f8275.is_none(),
        "a plain (non-promoted) disposal leg discloses nothing"
    );
    let packet_unpromoted =
        fill_full_return(&pr_unpromoted, 2024).expect("the packet without a promote must fill");
    assert!(
        !packet_unpromoted.iter().any(|f| f.name == "f8275"),
        "no f8275 in the packet when no leg is promoted"
    );

    // WITH the SAME leg promoted (origin in `promoted_origins`) + a matching PromoteTranche event for
    // the Part II narrative.
    state.promoted_origins.insert(origin.clone());
    let events = vec![btctax_core::LedgerEvent {
        id: EventId::decision(3),
        utc_timestamp: time::OffsetDateTime::UNIX_EPOCH,
        original_tz: time::UtcOffset::UTC,
        wallet: None,
        payload: EventPayload::PromoteTranche(PromoteTranche {
            target: origin.clone(),
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(300),
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I understand and accept this estimated-basis risk".into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
    }];

    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    let pr_promoted =
        assemble_printed_return(&ri, &state, &BTreeMap::new(), &ar, &table, 2024, &events)
            .expect("the promoted fixture assembles");
    let f8275 = pr_promoted
        .forms
        .f8275
        .as_ref()
        .expect("a promoted disposal leg discloses via f8275");
    assert_eq!(f8275.part_i.len(), 1, "one promoted leg ⇒ one Part I item");
    assert_eq!(f8275.part_i[0].amount, dec!(300), "the AS-FILED col (e) basis");
    assert_eq!(
        f8275.part_ii,
        "cash P2P purchase, no records; window bounded on-chain"
    );

    let packet_promoted =
        fill_full_return(&pr_promoted, 2024).expect("the promoted packet must fill");
    let f8275_form = packet_promoted
        .iter()
        .find(|f| f.name == "f8275")
        .expect("f8275 is in the packet when a leg is promoted");
    assert_eq!(f8275_form.attachment_sequence, Some("92"));
    assert!(!f8275_form.bytes.is_empty());
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
