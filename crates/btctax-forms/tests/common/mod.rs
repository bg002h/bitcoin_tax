//! Shared synthetic fixtures for the btctax-forms KATs. No real user file is ever read (NFR);
//! everything is built inline from invented values.

use btctax_core::{
    Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals, WalletId,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::macros::date;

/// Build a synthetic Form 8949 row (gain = proceeds − cost; no adjustment).
pub fn row(
    part: Form8949Part,
    desc: &str,
    proceeds: Decimal,
    cost: Decimal,
    exchange: bool,
) -> Form8949Row {
    Form8949Row {
        part,
        // The core taxonomy is C/F; the forms crate must map to Box I/L regardless.
        box_: if part == Form8949Part::ShortTerm {
            Form8949Box::C
        } else {
            Form8949Box::F
        },
        box_needs_review: exchange,
        description: desc.to_string(),
        date_acquired: date!(2024 - 02 - 03),
        date_sold: date!(2025 - 06 - 15),
        proceeds,
        cost_basis: cost,
        adjustment_code: String::new(),
        adjustment_amount: dec!(0),
        gain: proceeds - cost,
        wallet: if exchange {
            WalletId::Exchange {
                provider: "acme".into(),
                account: "a1".into(),
            }
        } else {
            WalletId::SelfCustody {
                label: "cold".into(),
            }
        },
        disposition_kind: btctax_core::DisposeKind::Sell,
    }
}

/// A canonical mixed fixture: 2 short-term + 1 long-term row.
pub fn mixed_rows() -> Vec<Form8949Row> {
    vec![
        row(
            Form8949Part::ShortTerm,
            "0.53000000 BTC",
            dec!(30000.50),
            dec!(25000),
            false,
        ),
        row(
            Form8949Part::ShortTerm,
            "0.10000000 BTC",
            dec!(6000),
            dec!(5500),
            true,
        ),
        row(
            Form8949Part::LongTerm,
            "1.00000000 BTC",
            dec!(60000),
            dec!(20000),
            false,
        ),
    ]
}

/// Sum a slice of rows of one part into a Schedule D part total (mirrors `btctax_core::schedule_d`).
pub fn sum_part(rows: &[Form8949Row], part: Form8949Part) -> ScheduleDPart {
    let mut p = ScheduleDPart::default();
    for r in rows.iter().filter(|r| r.part == part) {
        p.proceeds += r.proceeds;
        p.cost_basis += r.cost_basis;
        p.gain += r.gain;
    }
    p
}

/// The Schedule D totals consistent with a set of Form 8949 rows.
pub fn totals_for(rows: &[Form8949Row]) -> ScheduleDTotals {
    ScheduleDTotals {
        st: sum_part(rows, Form8949Part::ShortTerm),
        lt: sum_part(rows, Form8949Part::LongTerm),
    }
}
