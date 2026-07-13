//! Full-return v1 **§170(b) charitable-ceiling engine** (Phase 3 task 2). Applies the AGI-percentage
//! ceilings to a year's classified charitable gifts in Pub. 526 Worksheet-2 statutory order, and computes
//! the §170(d)(1) carryover-out (excess over ceiling, tagged by class/vintage, 5-year expiry).
//!
//! **Classes (deep/04 §2d, corrected):** 6 `CharitableClass` buckets. Crypto donations flow from the
//! ledger's §170(e)-reduced `Removal.claimed_deduction` — LT legs → `CapGainProp30` (FMV), ST legs →
//! `OrdinaryProp50` (basis) — never re-typed here (§4.6).
//!
//! **Ordering (Pub. 526 Worksheet 2):** 60% cash → 50% ordinary-income property → 30% capital-gain
//! property (to 50%-orgs) → then the non-50%-org classes. The 30%-cap-gain class is capped at the LESSER
//! of 30%·AGI OR (50%·AGI − allowed cash − allowed ordinary) — the overall-50%-room interaction (R2-I1).
//!
//! **Stage A (this file, first increment):** current-year gifts + ceilings + current-year carryover-out.
//! Carryover-IN consumption (current-first, oldest-vintage-first, expiry, std-year aging G8) lands next.
use crate::conventions::Usd;
use crate::tax::return_inputs::{CharitableCarryItem, CharitableClass, CharitableGift};
use rust_decimal_macros::dec;

/// The result of applying the §170(b) ceilings for one year (Schedule A lines 11/12/14 + the carryover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharitableResult {
    /// Schedule A **line 11** — cash contributions allowed this year (Cash-class gifts).
    pub allowed_cash: Usd,
    /// Schedule A **line 12** — noncash (property) contributions allowed this year, incl. crypto.
    pub allowed_noncash: Usd,
    /// Schedule A **line 14** — total charitable allowed this year (`11 + 12`; +line 13 carryover in Stage B).
    pub allowed: Usd,
    /// §170(d)(1) carryover to next year — the per-class excess over each ceiling, tagged with `year` as
    /// the origin vintage (oldest-first / expiry handled when carryover-IN lands).
    pub carryover_out: Vec<CharitableCarryItem>,
}

/// Apply the §170(b) AGI ceilings for `year` to the current-year classified `gifts` at `agi`
/// (Stage A — no carryover-in yet). Returns the allowed cash/noncash + the current-year carryover-out.
pub fn apply_170b(agi: Usd, gifts: &[CharitableGift], year: i32) -> CharitableResult {
    let sum = |class: CharitableClass| -> Usd {
        gifts
            .iter()
            .filter(|g| g.class == class)
            .map(|g| g.amount)
            .sum()
    };
    let cash60 = sum(CharitableClass::Cash60);
    let ord50 = sum(CharitableClass::OrdinaryProp50);
    let cgp30 = sum(CharitableClass::CapGainProp30);
    let cash30 = sum(CharitableClass::Cash30);
    let ord30 = sum(CharitableClass::OrdinaryProp30);
    let cgp20 = sum(CharitableClass::CapGainProp20);

    let pct = |p: Usd| p * agi;
    let nonneg = |v: Usd| v.max(Usd::ZERO);

    // ── 50%-organization classes, in Worksheet-2 order ───────────────────────────────────────────
    // (2) 60% — cash to a public charity.
    let allow_cash60 = cash60.min(pct(dec!(0.60)));
    // (3) 50% — ordinary-income (basis) property: limited by the 50%-of-AGI room the cash leaves.
    let allow_ord50 = ord50.min(nonneg(pct(dec!(0.50)) - allow_cash60));
    // (4) 30% — capital-gain property: LESSER of 30%·AGI or (50%·AGI − allowed cash − allowed ordinary).
    let cgp30_ceiling = pct(dec!(0.30)).min(nonneg(pct(dec!(0.50)) - allow_cash60 - allow_ord50));
    let allow_cgp30 = cgp30.min(cgp30_ceiling);

    // ── non-50%-organization classes (rare — spec "capture-only v1") ─────────────────────────────
    // Conservative own-% ceilings under a shared 30%-of-AGI room; the precise Pub. 526 "special 30%
    // limit" interaction is a documented follow-on (these classes never come from the crypto ledger).
    let allow_cash30 = cash30.min(pct(dec!(0.30)));
    let allow_ord30 = ord30.min(nonneg(pct(dec!(0.30)) - allow_cash30));
    let allow_cgp20 = cgp20.min(pct(dec!(0.20)).min(nonneg(pct(dec!(0.30)) - allow_cash30 - allow_ord30)));

    // Carryover-out: the per-class excess over its allowed amount, tagged with THIS year's vintage.
    let carryover_out = [
        (CharitableClass::Cash60, cash60, allow_cash60),
        (CharitableClass::OrdinaryProp50, ord50, allow_ord50),
        (CharitableClass::CapGainProp30, cgp30, allow_cgp30),
        (CharitableClass::Cash30, cash30, allow_cash30),
        (CharitableClass::OrdinaryProp30, ord30, allow_ord30),
        (CharitableClass::CapGainProp20, cgp20, allow_cgp20),
    ]
    .into_iter()
    .filter_map(|(class, total, allowed)| {
        let excess = total - allowed;
        (excess > Usd::ZERO).then_some(CharitableCarryItem {
            class,
            amount: excess,
            origin_year: year,
        })
    })
    .collect();

    let allowed_cash = allow_cash60 + allow_cash30;
    let allowed_noncash = allow_ord50 + allow_cgp30 + allow_ord30 + allow_cgp20;
    CharitableResult {
        allowed_cash,
        allowed_noncash,
        allowed: allowed_cash + allowed_noncash,
        carryover_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gift(class: CharitableClass, amount: Usd) -> CharitableGift {
        CharitableGift { class, amount }
    }

    /// deep/04 §3 worked example — MFJ, AGI $200,000, $5,000 cash + $70,000 LT crypto (30% class).
    /// Allowed: cash $5,000 (line 11) + crypto $60,000 (line 12) = $65,000 (line 14); carryover $10,000
    /// tagged 30%-capital-gain-property.
    #[test]
    fn deep04_worked_example_cent_exact() {
        let gifts = [
            gift(CharitableClass::Cash60, dec!(5000)),
            gift(CharitableClass::CapGainProp30, dec!(70000)),
        ];
        let r = apply_170b(dec!(200000), &gifts, 2024);
        assert_eq!(r.allowed_cash, dec!(5000)); // line 11
        assert_eq!(r.allowed_noncash, dec!(60000)); // line 12
        assert_eq!(r.allowed, dec!(65000)); // line 14
        assert_eq!(
            r.carryover_out,
            vec![CharitableCarryItem {
                class: CharitableClass::CapGainProp30,
                amount: dec!(10000),
                origin_year: 2024,
            }]
        );
    }

    /// The 30%-class two-term cap (R2-I1): the 30% capital-gain class must also fit under the overall
    /// 50%-of-AGI room AFTER cash + ordinary. AGI $100k; $40k cash (60% class) + $50k LT crypto: the crypto
    /// ceiling = min(30%·100k=$30k, 50%·100k − $40k cash = $10k) = $10k, NOT $30k.
    #[test]
    fn thirty_percent_class_capped_by_overall_fifty_room() {
        let gifts = [
            gift(CharitableClass::Cash60, dec!(40000)),
            gift(CharitableClass::CapGainProp30, dec!(50000)),
        ];
        let r = apply_170b(dec!(100000), &gifts, 2024);
        assert_eq!(r.allowed_cash, dec!(40000));
        assert_eq!(r.allowed_noncash, dec!(10000)); // crypto capped at $10k, not $30k
        assert_eq!(r.carryover_out[0].amount, dec!(40000)); // $50k − $10k
    }

    /// KAT-17: same-year ST + LT crypto donation. ST → OrdinaryProp50 (basis, 50% class), LT →
    /// CapGainProp30 (FMV, 30% class). AGI $100k; ST $20k + LT $40k: ordinary allowed $20k (≤ 50%·100k),
    /// crypto 30% ceiling = min($30k, 50%·100k − $0 cash − $20k ord = $30k) = $30k → $30k allowed, $10k carry.
    #[test]
    fn kat17_same_year_short_and_long_crypto() {
        let gifts = [
            gift(CharitableClass::OrdinaryProp50, dec!(20000)),
            gift(CharitableClass::CapGainProp30, dec!(40000)),
        ];
        let r = apply_170b(dec!(100000), &gifts, 2024);
        assert_eq!(r.allowed_noncash, dec!(50000)); // $20k ordinary + $30k capgain
        assert_eq!(r.allowed, dec!(50000));
        // Only the 30% class overflows ($40k − $30k = $10k).
        assert_eq!(
            r.carryover_out,
            vec![CharitableCarryItem {
                class: CharitableClass::CapGainProp30,
                amount: dec!(10000),
                origin_year: 2024,
            }]
        );
    }

    /// Under-ceiling gifts are fully allowed with no carryover.
    #[test]
    fn under_ceiling_fully_allowed_no_carryover() {
        let gifts = [gift(CharitableClass::Cash60, dec!(1000))];
        let r = apply_170b(dec!(100000), &gifts, 2024);
        assert_eq!(r.allowed, dec!(1000));
        assert!(r.carryover_out.is_empty());
    }
}
