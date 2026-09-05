//! The "could these two legs be ONE movement?" predicate — the SINGLE shared definition.
//!
//! Two surfaces ask this question and they must not answer it differently:
//!
//!  1. `Session::self_transfer_match_plan` (`btctax-cli`) — PROPOSES pairs for the filer to confirm
//!     (`reconcile match-self-transfers`). Confirmation is always explicit: the owner's
//!     self-transfer policy is *matched pairs are confirmed, never auto*.
//!  2. The FR-31 double-booking guard in `resolve` — REFUSES when a `TransferLink{Wallet(w)}`
//!     (`--to-wallet`, which names a destination and no in-event) coexists with an un-consumed
//!     deposit at `w` that could be the other half of that same movement. Booking both relocates
//!     the real lot into `w` AND mints a fresh origin lot for the coins that just arrived —
//!     doubling the pool and the basis, which understates tax.
//!
//! Note the asymmetry, and that it is the whole design: the predicate is only ever allowed to
//! PROPOSE or to REFUSE. It never picks a pairing and applies it. That is why a heuristic is
//! admissible here at all — a false positive costs the filer a blocker that names both events, and
//! is cleared by naming the true pairing (`--to-event`) or by voiding the wrong decision. A false
//! positive can never move a number on a return.

use crate::conventions::{Sat, TaxDate};

/// Where the two legs sit relative to each other, which fixes the direction of the date window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairTopology {
    /// Deposit and withdrawal at the SAME wallet — an exchange passthrough. The deposit precedes
    /// the withdrawal.
    Passthrough,
    /// Withdrawal at one wallet, arrival at another — a relocation. The withdrawal precedes the
    /// arrival.
    Relocation,
}

/// The withdrawal half.
#[derive(Debug, Clone, Copy)]
pub struct OutLeg<'a> {
    /// Principal sats leaving (EXCLUDING the network fee).
    pub principal_sat: Sat,
    /// On-chain fee sats, when known — the deposit arrives short by at most this much.
    pub fee_sat: Option<Sat>,
    pub txid: Option<&'a str>,
    pub date: TaxDate,
}

/// The deposit half.
#[derive(Debug, Clone, Copy)]
pub struct InLeg<'a> {
    pub sat: Sat,
    pub txid: Option<&'a str>,
    pub date: TaxDate,
}

/// Amount slack allowed between the two legs: `max(fee_sat, ceil(0.005 × principal))`.
///
/// The fee term covers a deposit that arrived short by the network fee; the 0.5% floor covers an
/// exchange that reports a rounded or fee-inclusive figure. `ceil(p / 200) == (p + 199) / 200` for
/// `p >= 0`; a negative or zero principal contributes no floor.
pub fn pair_amount_tolerance(principal_sat: Sat, fee_sat: Option<Sat>) -> Sat {
    let slack = if principal_sat > 0 {
        (principal_sat + 199) / 200
    } else {
        0
    };
    fee_sat.unwrap_or(0).max(slack)
}

/// Could these two legs be the two halves of one movement?
///
/// A matching txid is conclusive on the amount (the chain says so) and bypasses the tolerance; the
/// date window still applies, since a txid can be reused across the same pair of legs only once.
pub fn legs_could_be_one_movement(
    in_leg: &InLeg<'_>,
    out_leg: &OutLeg<'_>,
    topology: PairTopology,
) -> bool {
    let txid_match = in_leg.txid.is_some() && in_leg.txid == out_leg.txid;
    let tol = pair_amount_tolerance(out_leg.principal_sat, out_leg.fee_sat);
    let amount_ok = txid_match || (in_leg.sat - out_leg.principal_sat).abs() <= tol;
    if !amount_ok {
        return false;
    }
    // ±2-day window, direction keyed to the topology (exchange timestamp drift tolerated).
    match topology {
        PairTopology::Passthrough => {
            let d = (out_leg.date - in_leg.date).whole_days();
            (0..=2).contains(&d)
        }
        PairTopology::Relocation => {
            let d = (in_leg.date - out_leg.date).whole_days();
            (0..=2).contains(&d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    const ONE_BTC: Sat = 100_000_000;

    fn out(sat: Sat, fee: Option<Sat>, d: TaxDate) -> OutLeg<'static> {
        OutLeg {
            principal_sat: sat,
            fee_sat: fee,
            txid: None,
            date: d,
        }
    }
    fn inn(sat: Sat, d: TaxDate) -> InLeg<'static> {
        InLeg {
            sat,
            txid: None,
            date: d,
        }
    }

    #[test]
    fn tolerance_is_the_larger_of_the_fee_and_half_a_percent() {
        // 0.5% of 1 BTC = 500_000 sat, which dominates a 10_000-sat fee.
        assert_eq!(pair_amount_tolerance(ONE_BTC, Some(10_000)), 500_000);
        // A fat fee on a small principal dominates instead.
        assert_eq!(pair_amount_tolerance(1_000, Some(10_000)), 10_000);
        // ceil, not floor: 201 sat → ceil(1.005) = 2.
        assert_eq!(pair_amount_tolerance(201, None), 2);
        assert_eq!(pair_amount_tolerance(0, None), 0);
    }

    #[test]
    fn an_exact_same_day_relocation_pairs() {
        assert!(legs_could_be_one_movement(
            &inn(ONE_BTC, date!(2025 - 03 - 01)),
            &out(ONE_BTC, None, date!(2025 - 03 - 01)),
            PairTopology::Relocation
        ));
    }

    #[test]
    fn a_tenth_of_the_amount_does_not_pair() {
        assert!(!legs_could_be_one_movement(
            &inn(ONE_BTC / 10, date!(2025 - 03 - 01)),
            &out(ONE_BTC, None, date!(2025 - 03 - 01)),
            PairTopology::Relocation
        ));
    }

    #[test]
    fn the_relocation_window_is_directional_and_two_days_wide() {
        let o = out(ONE_BTC, None, date!(2025 - 03 - 01));
        for (day, want) in [(1, true), (2, true), (3, true), (4, false)] {
            let d = date!(2025 - 03 - 01).replace_day(day).unwrap();
            assert_eq!(
                legs_could_be_one_movement(&inn(ONE_BTC, d), &o, PairTopology::Relocation),
                want,
                "arrival on 2025-03-{day:02}"
            );
        }
        // An arrival BEFORE the withdrawal is not a relocation.
        assert!(!legs_could_be_one_movement(
            &inn(ONE_BTC, date!(2025 - 02 - 28)),
            &o,
            PairTopology::Relocation
        ));
        // …but it IS the right direction for a passthrough.
        assert!(legs_could_be_one_movement(
            &inn(ONE_BTC, date!(2025 - 02 - 28)),
            &o,
            PairTopology::Passthrough
        ));
    }

    #[test]
    fn a_matching_txid_carries_an_amount_the_tolerance_would_reject() {
        let o = OutLeg {
            principal_sat: ONE_BTC,
            fee_sat: None,
            txid: Some("abc"),
            date: date!(2025 - 03 - 01),
        };
        let i = InLeg {
            sat: ONE_BTC / 2,
            txid: Some("abc"),
            date: date!(2025 - 03 - 01),
        };
        assert!(legs_could_be_one_movement(&i, &o, PairTopology::Relocation));
        // Two absent txids are NOT a match (`None == None` must not pair everything).
        let i_none = InLeg {
            sat: ONE_BTC / 2,
            txid: None,
            date: date!(2025 - 03 - 01),
        };
        let o_none = OutLeg { txid: None, ..o };
        assert!(!legs_could_be_one_movement(
            &i_none,
            &o_none,
            PairTopology::Relocation
        ));
    }
}
