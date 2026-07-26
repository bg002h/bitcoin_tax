//! DFW-D9: era presets — confirm/edit STARTING POINTS for the Declare flow's window, never
//! authoritative windows. `era_window` maps each preset to a concrete `[start, end]` calendar window;
//! the Declare flow always applies the DFW-D5 before-the-short-op-date clamp ON TOP of whatever a
//! preset seeds ("the DFW-D5 before-the-short-op prefill governs over a preset's `window_end` where
//! they conflict" — SPEC.md DFW-D9).
//!
//! **What these buckets are (OWNER-RATIFIED).** Deliberate, CLAIM-FREE **round calendar spans** —
//! plain multi-year calendar blocks running from Bitcoin's genesis block (2009-01-03) to the newest tax
//! year this app can file. They make **no** historical, exchange, price, protocol or event claim (no
//! "Silk Road era", no named venue, no halving framing), which is exactly the point: there is nothing
//! in the table that can be factually WRONG. A bucket is one keystroke that seeds two dates the filer
//! then confirms or edits. This is a **UI convenience seeding a FILER ATTESTATION** — it is not
//! IRS-approved, not authoritative, and asserts nothing about when anyone actually acquired anything.
//!
//! **The filer's own window governs, and is validated downstream.** No preset value reaches a filed
//! number: the Declare flow requires an **EXPLICIT pick** (nothing is pre-selected — the tool never
//! answers "when did you acquire these coins?" for the filer, and `window_end` IS the lot's acquisition
//! date, `resolve.rs:~1310`, so the answer also sets short/long-term character); the DFW-D5
//! before-the-short-op clamp always governs `window_end`; `btctax_cli::plan_declare` re-validates
//! whatever window the filer confirms; and a later promotion's filed floor comes from
//! `conservative_promote::filed_basis_for`, which requires `Coverage::Full` over real price data.
//!
//! **No bucket STRADDLES the pooling cutover** (`conventions::TRANSITION_DATE`, 2025-01-01): every
//! window lies entirely before it or entirely on/after it, so a declared tranche's lot always lands in
//! exactly ONE pooling era (`project::pools::pool_key` — a single Universal pool before the cutover,
//! per-wallet from it on). Pinned by
//! `defensive_era.rs::no_preset_window_straddles_the_pooling_cutover`.

use crate::conventions::{TaxDate, TRANSITION_DATE};
use time::macros::date;

/// A named starting-point window for the Declare flow (DFW-D9). Presets are confirm/edit starting
/// points, NEVER authoritative windows — see the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EraPreset {
    /// Bitcoin's genesis block (2009-01-03) through 2011.
    Y2009To2011,
    Y2012To2014,
    Y2015To2017,
    Y2018To2020,
    Y2021To2024,
    /// 2025-01-01 (`conventions::TRANSITION_DATE`) onward — the post-cutover, wallet-partitioned
    /// pooling era.
    ///
    /// **Why it exists:** `pools::pool_key` puts a pre-2025 lot in the Universal pool and a 2025+ lot in
    /// its wallet's own pool, so **a pre-2025 tranche cannot cover a post-2025 disposal in the same
    /// wallet**. Without this bucket a 2025-or-later shortfall was reachable only by ±1-day nudging away
    /// from a pre-cutover preset.
    ///
    /// **Why the concrete end is 2025-12-31:** `era_window` is pure and CLOCK-FREE, so "onward" needs a
    /// concrete upper bound. 2025-12-31 is the last day of the newest tax year this app can file
    /// (`conventions::TY2025_RETURN_DUE` is TY2025's return due date; `btctax_forms::SUPPORTED_YEARS`
    /// likewise tops out at 2025) — the same round calendar-year end every other bucket uses, and never
    /// a seeded window running past a year the filer could actually be filing. A filer whose coins are
    /// newer nudges `window_end` forward; the REAL upper bound is enforced downstream regardless (the
    /// DFW-D5 before-the-short-op clamp, and `filed_basis_for`'s `Coverage::Full` requirement over
    /// actual price data), so this bound can never produce a wrong filed number.
    Y2025Onward,
}

/// Every preset, OLDEST first — the Declare flow's picker order (`1..=ALL_PRESETS.len()`), the
/// `next_preset` cycle order, and KAT (a)'s enumeration.
pub const ALL_PRESETS: [EraPreset; 6] = [
    EraPreset::Y2009To2011,
    EraPreset::Y2012To2014,
    EraPreset::Y2015To2017,
    EraPreset::Y2018To2020,
    EraPreset::Y2021To2024,
    EraPreset::Y2025Onward,
];

/// Map a preset to its concrete `[start, end]` calendar window (inclusive both ends).
pub fn era_window(preset: EraPreset) -> (TaxDate, TaxDate) {
    match preset {
        EraPreset::Y2009To2011 => (date!(2009 - 01 - 03), date!(2011 - 12 - 31)),
        EraPreset::Y2012To2014 => (date!(2012 - 01 - 01), date!(2014 - 12 - 31)),
        EraPreset::Y2015To2017 => (date!(2015 - 01 - 01), date!(2017 - 12 - 31)),
        EraPreset::Y2018To2020 => (date!(2018 - 01 - 01), date!(2020 - 12 - 31)),
        EraPreset::Y2021To2024 => (date!(2021 - 01 - 01), date!(2024 - 12 - 31)),
        // Starts EXACTLY at the pooling cutover, so no window straddles it (module doc).
        EraPreset::Y2025Onward => (TRANSITION_DATE, date!(2025 - 12 - 31)),
    }
}

/// The next preset after `p` in `ALL_PRESETS`' order, wrapping to the first after the last.
///
/// ★ It has **no production caller** since the owner's explicit-pick decision replaced the Declare
/// flow's Tab-cycling with a numbered picker (a "next" key would have made the first bucket a single
/// keystroke away from being the de-facto default again). Retained as a pure, total, already-published
/// accessor of the table's own ordering — it holds no state, gates nothing, and cannot drift from
/// `ALL_PRESETS` because it is derived from it. Filed as a follow-up rather than deleted silently.
pub fn next_preset(p: EraPreset) -> EraPreset {
    let idx = ALL_PRESETS.iter().position(|&e| e == p).unwrap_or(0);
    ALL_PRESETS[(idx + 1) % ALL_PRESETS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_window_is_non_empty_and_ordered() {
        for &p in &ALL_PRESETS {
            let (start, end) = era_window(p);
            assert!(start <= end, "{p:?}: start {start} must be <= end {end}");
        }
    }

    #[test]
    fn presets_are_non_overlapping_and_increasing_in_all_presets_order() {
        let mut prev_end = None;
        for &p in &ALL_PRESETS {
            let (start, _end) = era_window(p);
            if let Some(pe) = prev_end {
                assert!(
                    start > pe,
                    "{p:?} must start strictly after the previous preset's end"
                );
            }
            prev_end = Some(era_window(p).1);
        }
    }

    #[test]
    fn next_preset_cycles_and_wraps_to_first() {
        assert_eq!(next_preset(EraPreset::Y2009To2011), EraPreset::Y2012To2014);
        assert_eq!(next_preset(EraPreset::Y2021To2024), EraPreset::Y2025Onward);
        assert_eq!(next_preset(EraPreset::Y2025Onward), EraPreset::Y2009To2011);
    }

    /// The property the REPLACED `all_presets_end_strictly_before_the_pre2025_pooling_cutover` guard
    /// was REALLY protecting, restated so it survives a post-cutover bucket: no window may STRADDLE
    /// `TRANSITION_DATE`, so a declared tranche's lot lands in exactly one pooling era. (The
    /// integration KAT `defensive_era.rs::no_preset_window_straddles_the_pooling_cutover` pins the same
    /// invariant from outside the crate; this unit copy keeps it adjacent to the table it constrains.)
    #[test]
    fn no_window_straddles_the_pooling_cutover() {
        for &p in &ALL_PRESETS {
            let (start, end) = era_window(p);
            let cutover = crate::conventions::TRANSITION_DATE;
            assert!(
                end < cutover || start >= cutover,
                "{p:?} straddles the {cutover} pooling cutover ({start}..{end}) — its lot would span \
                 the Universal/per-wallet pool split"
            );
        }
    }
}
