//! Full-return v1 **§170(b) charitable-ceiling engine** (Phase 3 task 2). Applies the AGI-percentage
//! ceilings to a year's classified charitable gifts + prior carryover in Pub. 526 Worksheet-2 statutory
//! order, and computes the §170(d)(1) carryover-out (excess over ceiling, tagged by class/vintage, 5-year
//! expiry, oldest-vintage-first consumption).
//!
//! **Classes (deep/04 §2d, corrected):** 6 `CharitableClass` buckets. Crypto donations flow from the
//! ledger's §170(e)-reduced `Removal.claimed_deduction` — LT legs → `CapGainProp30` (FMV), ST legs →
//! `OrdinaryProp50` (basis) — never re-typed here (§4.6).
//!
//! **Ordering (Pub. 526 Worksheet 2):** 60% cash → 50% ordinary-income property → 30% capital-gain
//! property (to 50%-orgs) → then the non-50%-org classes. The 30%-cap-gain class is capped at the LESSER
//! of 30%·AGI OR (50%·AGI − allowed cash − allowed ordinary) — the overall-50%-room interaction (R2-I1).
//!
//! **Carryover:** within each class's ceiling, CURRENT-year gifts are allowed first, then carryover-in
//! oldest-vintage-first (Pub. 526). Current excess → a fresh carryover (this year's vintage); unused
//! carryover-in survives with its original vintage; carryover older than 5 years is EXPIRED (dropped).
//! The engine runs even in a standard-deduction year so the carryover ages / is reduced (Reg.
//! §1.170A-10(a)(2), G8) — the caller uses `allowed` only when itemizing but always writes `carryover_out`.
use crate::conventions::Usd;
use crate::tax::return_inputs::{CharitableCarryItem, CharitableClass, CharitableGift};
use rust_decimal_macros::dec;

/// The result of applying the §170(b) ceilings for one year (Schedule A lines 11/12/13/14 + the carryover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharitableResult {
    /// Schedule A **line 11** — CURRENT-year cash contributions allowed (Cash-class gifts).
    pub allowed_cash: Usd,
    /// Schedule A **line 12** — CURRENT-year noncash (property) contributions allowed, incl. crypto.
    pub allowed_noncash: Usd,
    /// Schedule A **line 13** — prior-year CARRYOVER allowed this year (any class).
    pub allowed_carryover: Usd,
    /// Schedule A **line 14** — total charitable allowed this year (`11 + 12 + 13`).
    pub allowed: Usd,
    /// §170(d)(1) carryover to next year: the current-year excess (tagged with THIS `year`) plus any
    /// unused carryover-in (its original vintage). Expired (> 5-year) carryover-in is dropped, not carried.
    pub carryover_out: Vec<CharitableCarryItem>,
}

/// §170(d)(1): a gift's excess carries forward up to 5 succeeding years. A carryover from `origin` is
/// EXPIRED in tax `year` once `year − origin > 5` (dropped — never used, never carried further).
fn is_expired(origin: i32, year: i32) -> bool {
    year - origin > 5
}

struct ClassAlloc {
    current_allowed: Usd,
    carryover_allowed: Usd,
    carry_out: Vec<CharitableCarryItem>,
}

/// Allocate one class's `ceiling`: CURRENT-year gifts first, then carryover-in OLDEST-vintage-first
/// (Pub. 526). Current excess → a new carryover item (this `year`); unused carryover-in survives with its
/// original vintage. `carryover` is this class only, already non-expired and sorted oldest-first.
fn allocate_class(
    class: CharitableClass,
    current_total: Usd,
    carryover: &[CharitableCarryItem],
    ceiling: Usd,
    year: i32,
) -> ClassAlloc {
    let current_allowed = current_total.min(ceiling);
    let mut room = ceiling - current_allowed;
    let mut carryover_allowed = Usd::ZERO;
    let mut carry_out = Vec::new();

    let current_excess = current_total - current_allowed;
    if current_excess > Usd::ZERO {
        carry_out.push(CharitableCarryItem {
            class,
            amount: current_excess,
            origin_year: year,
        });
    }
    for item in carryover {
        let used = item.amount.min(room);
        carryover_allowed += used;
        room -= used;
        let remaining = item.amount - used;
        if remaining > Usd::ZERO {
            carry_out.push(CharitableCarryItem {
                class,
                amount: remaining,
                origin_year: item.origin_year,
            });
        }
    }
    ClassAlloc {
        current_allowed,
        carryover_allowed,
        carry_out,
    }
}

/// Apply the §170(b) AGI ceilings for `year` to the current-year `gifts` + prior `carryover_in` at `agi`.
///
/// v1 scope: only the three 50%-organization classes (Cash60 / OrdinaryProp50 / CapGainProp30) are
/// allocated; any non-50%-org gift or carryover-in is refused upstream by `screen_inputs`, so this function
/// never sees one. A negative `agi` is clamped to zero before any ceiling is computed.
pub fn apply_170b(
    agi: Usd,
    gifts: &[CharitableGift],
    carryover_in: &[CharitableCarryItem],
    year: i32,
) -> CharitableResult {
    use CharitableClass::{Cash60, CapGainProp30, OrdinaryProp50};

    // Ceilings are a fraction of AGI; a negative AGI would make every ceiling negative and (after the
    // `nonneg` clamps) zero out all allowances, but we clamp explicitly so the intent is unmistakable and
    // no downstream arithmetic ever multiplies a percentage by a negative base (review M1).
    let agi = agi.max(Usd::ZERO);

    let pct = |p: Usd| p * agi;
    let nonneg = |v: Usd| v.max(Usd::ZERO);
    let current = |class: CharitableClass| -> Usd {
        gifts
            .iter()
            .filter(|g| g.class == class)
            .map(|g| g.amount)
            .sum()
    };
    // Non-expired carryover-in for one class, OLDEST-vintage-first.
    let carry_for = |class: CharitableClass| -> Vec<CharitableCarryItem> {
        let mut v: Vec<CharitableCarryItem> = carryover_in
            .iter()
            .filter(|c| c.class == class && !is_expired(c.origin_year, year))
            .cloned()
            .collect();
        v.sort_by_key(|c| c.origin_year);
        v
    };

    // ── 50%-org classes in Worksheet-2 order; each ceiling uses the TOTAL (current + carryover) allowed
    //    of the earlier classes (R2-I1). ──────────────────────────────────────────────────────────────
    let cash60 = allocate_class(Cash60, current(Cash60), &carry_for(Cash60), pct(dec!(0.60)), year);
    let allowed_cash_tier = cash60.current_allowed + cash60.carryover_allowed;
    let ord50 = allocate_class(
        OrdinaryProp50,
        current(OrdinaryProp50),
        &carry_for(OrdinaryProp50),
        nonneg(pct(dec!(0.50)) - allowed_cash_tier),
        year,
    );
    let allowed_ord_tier = ord50.current_allowed + ord50.carryover_allowed;
    let cgp30_ceiling = pct(dec!(0.30)).min(nonneg(pct(dec!(0.50)) - allowed_cash_tier - allowed_ord_tier));
    let cgp30 = allocate_class(CapGainProp30, current(CapGainProp30), &carry_for(CapGainProp30), cgp30_ceiling, year);

    // Non-50%-organization classes (Cash30 / OrdinaryProp30 / CapGainProp20) are REFUSED upstream by
    // `screen_inputs` (review C1): their Pub. 526 "special 30% limit" ordering — which interleaves with the
    // 50%-org tiers rather than sitting under an independent 30% room — is unmodeled in v1, so only the three
    // 50%-org classes ever reach here. Keep the `CharitableClass` capture-only variants; do not allocate them.
    let allowed_cash = cash60.current_allowed;
    let allowed_noncash = ord50.current_allowed + cgp30.current_allowed;
    let allowed_carryover =
        cash60.carryover_allowed + ord50.carryover_allowed + cgp30.carryover_allowed;
    let carryover_out = [cash60, ord50, cgp30]
        .into_iter()
        .flat_map(|a| a.carry_out)
        .collect();

    CharitableResult {
        allowed_cash,
        allowed_noncash,
        allowed_carryover,
        allowed: allowed_cash + allowed_noncash + allowed_carryover,
        carryover_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gift(class: CharitableClass, amount: Usd) -> CharitableGift {
        CharitableGift { class, amount }
    }
    fn carry(class: CharitableClass, amount: Usd, origin_year: i32) -> CharitableCarryItem {
        CharitableCarryItem {
            class,
            amount,
            origin_year,
        }
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
        let r = apply_170b(dec!(200000), &gifts, &[], 2024);
        assert_eq!(r.allowed_cash, dec!(5000)); // line 11
        assert_eq!(r.allowed_noncash, dec!(60000)); // line 12
        assert_eq!(r.allowed_carryover, Usd::ZERO); // line 13
        assert_eq!(r.allowed, dec!(65000)); // line 14
        assert_eq!(
            r.carryover_out,
            vec![carry(CharitableClass::CapGainProp30, dec!(10000), 2024)]
        );
    }

    /// The 30%-class two-term cap (R2-I1): the 30% capital-gain class must fit under the overall 50%-of-AGI
    /// room AFTER cash. AGI $100k; $40k cash + $50k LT crypto → crypto ceiling = min($30k, $50k − $40k) =
    /// $10k, NOT $30k.
    #[test]
    fn thirty_percent_class_capped_by_overall_fifty_room() {
        let gifts = [
            gift(CharitableClass::Cash60, dec!(40000)),
            gift(CharitableClass::CapGainProp30, dec!(50000)),
        ];
        let r = apply_170b(dec!(100000), &gifts, &[], 2024);
        assert_eq!(r.allowed_cash, dec!(40000));
        assert_eq!(r.allowed_noncash, dec!(10000));
        assert_eq!(r.carryover_out[0].amount, dec!(40000));
    }

    /// KAT-17: same-year ST + LT crypto donation. ST → OrdinaryProp50 (50% class), LT → CapGainProp30 (30%).
    /// AGI $100k; ST $20k + LT $40k → ordinary $20k + crypto min($30k, $50k−$20k=$30k)=$30k → $50k allowed.
    #[test]
    fn kat17_same_year_short_and_long_crypto() {
        let gifts = [
            gift(CharitableClass::OrdinaryProp50, dec!(20000)),
            gift(CharitableClass::CapGainProp30, dec!(40000)),
        ];
        let r = apply_170b(dec!(100000), &gifts, &[], 2024);
        assert_eq!(r.allowed_noncash, dec!(50000));
        assert_eq!(r.allowed, dec!(50000));
        assert_eq!(
            r.carryover_out,
            vec![carry(CharitableClass::CapGainProp30, dec!(10000), 2024)]
        );
    }

    /// Carryover-in is consumed AFTER current-year gifts, within the class ceiling → Schedule A line 13.
    #[test]
    fn carryover_in_consumed_after_current_year() {
        // AGI $100k, 60%-cash ceiling $60k. Current cash $50k + a $20k cash carryover from 2022.
        let gifts = [gift(CharitableClass::Cash60, dec!(50000))];
        let cin = [carry(CharitableClass::Cash60, dec!(20000), 2022)];
        let r = apply_170b(dec!(100000), &gifts, &cin, 2024);
        assert_eq!(r.allowed_cash, dec!(50000)); // line 11 (current)
        assert_eq!(r.allowed_carryover, dec!(10000)); // line 13 ($60k ceiling − $50k current)
        assert_eq!(r.allowed, dec!(60000)); // line 14
        // The unused $10k of the 2022 carryover survives with its ORIGINAL vintage.
        assert_eq!(
            r.carryover_out,
            vec![carry(CharitableClass::Cash60, dec!(10000), 2022)]
        );
    }

    /// Carryover older than 5 years is EXPIRED — dropped, never used, never carried (§170(d)(1)).
    #[test]
    fn expired_carryover_is_dropped() {
        let cin = [
            carry(CharitableClass::Cash60, dec!(5000), 2018), // 2024 − 2018 = 6 > 5 → expired
            carry(CharitableClass::Cash60, dec!(3000), 2019), // 2024 − 2019 = 5 → still usable
        ];
        let r = apply_170b(dec!(100000), &[], &cin, 2024);
        assert_eq!(r.allowed_carryover, dec!(3000)); // only the 2019 item is usable
        assert!(r.carryover_out.is_empty()); // 2019 fully used; 2018 expired (dropped)
    }

    /// KAT-13: carryover survives an intervening standard-deduction year (the engine still runs so it ages
    /// but isn't fully consumed). A $30k 2022 carryover, in a low-ceiling year, is partly used + partly
    /// carried with its original 2022 vintage — so a later year can still use it (within the 5-year window).
    #[test]
    fn kat13_carryover_ages_across_a_std_year() {
        // AGI $40k → 60%-cash ceiling $24k. No current gifts; a $30k 2022 cash carryover.
        let cin = [carry(CharitableClass::Cash60, dec!(30000), 2022)];
        let r = apply_170b(dec!(40000), &[], &cin, 2024);
        assert_eq!(r.allowed_carryover, dec!(24000)); // used up to the ceiling
        // The unused $6k carries with its 2022 vintage (usable through 2027, still within 5 years).
        assert_eq!(
            r.carryover_out,
            vec![carry(CharitableClass::Cash60, dec!(6000), 2022)]
        );
    }

    /// Oldest-vintage-first: with limited room, the OLDER carryover is consumed before the newer one.
    #[test]
    fn oldest_vintage_consumed_first() {
        // AGI $20k → 60%-cash ceiling $12k. Carryovers: $10k from 2020 + $10k from 2023.
        let cin = [
            carry(CharitableClass::Cash60, dec!(10000), 2023),
            carry(CharitableClass::Cash60, dec!(10000), 2020),
        ];
        let r = apply_170b(dec!(20000), &[], &cin, 2024);
        assert_eq!(r.allowed_carryover, dec!(12000));
        // 2020 fully used ($10k) + $2k of 2023; the remaining $8k of 2023 survives.
        assert_eq!(
            r.carryover_out,
            vec![carry(CharitableClass::Cash60, dec!(8000), 2023)]
        );
    }

    /// Review M1: a negative AGI is clamped to zero, so every ceiling is zero — nothing is allowed and the
    /// ENTIRE gift carries forward (no negative "allowed", no carryover inflated beyond the gift).
    #[test]
    fn negative_agi_clamped_to_zero_ceilings() {
        let gifts = [gift(CharitableClass::Cash60, dec!(5000))];
        let r = apply_170b(dec!(-10000), &gifts, &[], 2024);
        assert_eq!(r.allowed_cash, Usd::ZERO);
        assert_eq!(r.allowed_noncash, Usd::ZERO);
        assert_eq!(r.allowed_carryover, Usd::ZERO);
        assert_eq!(r.allowed, Usd::ZERO);
        // The whole $5,000 gift carries forward — never more (the pre-fix bug inflated this past the gift).
        assert_eq!(
            r.carryover_out,
            vec![carry(CharitableClass::Cash60, dec!(5000), 2024)]
        );
    }
}
