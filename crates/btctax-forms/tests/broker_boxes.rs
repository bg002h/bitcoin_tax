//! spec 1099-DA T3 (the fail-closed half, landed with step C): until the per-(part, box) page-sets
//! exist, a row routed to a broker-reported box (G/H/J/K) REFUSES to print. The bundled map checks
//! one box per part, and printing such a row under the I/L checkbox would be a wrong box on a signed
//! return laundered as the not-reported default.

mod common;
use common::*;

use btctax_core::forms::Form8949Box;
use btctax_core::Form8949Part;
use btctax_forms::fill_form_8949;
use rust_decimal_macros::dec;

#[test]
fn a_broker_reported_box_refuses_to_print_until_its_page_set_exists() {
    for (b, part) in [
        (Form8949Box::G, Form8949Part::ShortTerm),
        (Form8949Box::H, Form8949Part::ShortTerm),
        (Form8949Box::J, Form8949Part::LongTerm),
        (Form8949Box::K, Form8949Part::LongTerm),
    ] {
        let mut r = row(part, "1.00000000 BTC", dec!(100), dec!(50), false);
        r.box_ = b;
        let err = fill_form_8949(&[r], 2025).expect_err("a G/H/J/K row must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("1099-DA T3") && msg.contains(&format!("{b:?}")),
            "the refusal names the box and the task that closes it: {msg}"
        );
    }
    // the same rows under the map's own not-reported boxes print
    let mut i = row(
        Form8949Part::ShortTerm,
        "1.00000000 BTC",
        dec!(100),
        dec!(50),
        false,
    );
    i.box_ = Form8949Box::I;
    fill_form_8949(&[i], 2025).expect("an I row prints");
}
