//! DFW-D9: era presets — confirm/edit STARTING POINTS for the Declare flow's window, never
//! authoritative windows. `era_window` maps each preset to a concrete `[start, end]` calendar window;
//! the Declare flow always applies the DFW-D5 before-the-short-op-date clamp ON TOP of whatever a
//! preset seeds ("the DFW-D5 before-the-short-op prefill governs over a preset's `window_end` where
//! they conflict" — SPEC.md DFW-D9).
//!
//! ★ **PROVISIONAL TABLE — flagged for the P-C gate, not silently final.** SPEC DFW-D9 requires this
//! table get "the same review rigor as copy", and — confirmed by an exhaustive grep across every
//! `SPEC.md`/`DESIGN.md`/`BRAINSTORM.md`/`IMPLEMENTATION_PLAN.md` and every `reviews/*.md` round in
//! `design/defensive-filing-wizard/` — **no reviewed era→window table exists anywhere in the design
//! corpus**. The architecture review itself says so explicitly: "No era table exists anywhere in the
//! tree (grep: nothing) — this is NEW product-authored reference data" (`brainstorm-architecture-
//! fable-review-r1.md`), confirmed unchanged at r2. The five buckets below are deliberately **round,
//! non-narrative calendar-year spans** (no named-exchange/historical-event claims, e.g. no "Silk Road
//! era" framing) spanning Bitcoin's genesis block (2009-01-03) through the pre-2025 pooling cutover —
//! chosen ONLY so the MECHANISM (confirm/edit, prefill-precedence, live readout, era-cycling) can be
//! built and KAT-proven end-to-end. **This table is NOT a product-approved artifact.** A P-C-owned
//! follow-up (filed in `FOLLOWUPS.md`) tracks the outstanding copy/date-boundary review; per the
//! standing workflow's "phase-owned follow-up" rule it must be burned down before the P-C gate closes.

use crate::conventions::TaxDate;
use time::macros::date;

/// A named starting-point window for the Declare flow (DFW-D9). Presets are confirm/edit starting
/// points, NEVER authoritative windows — see the module doc's provisional-table note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EraPreset {
    /// Bitcoin's genesis block (2009-01-03) through 2011.
    Y2009To2011,
    Y2012To2014,
    Y2015To2017,
    Y2018To2020,
    Y2021To2024,
}

/// Every preset, OLDEST first — the Declare flow's cycle order (`next_preset`) and KAT (a)'s
/// enumeration.
pub const ALL_PRESETS: [EraPreset; 5] = [
    EraPreset::Y2009To2011,
    EraPreset::Y2012To2014,
    EraPreset::Y2015To2017,
    EraPreset::Y2018To2020,
    EraPreset::Y2021To2024,
];

/// Map a preset to its concrete `[start, end]` calendar window (inclusive both ends).
pub fn era_window(preset: EraPreset) -> (TaxDate, TaxDate) {
    match preset {
        EraPreset::Y2009To2011 => (date!(2009 - 01 - 03), date!(2011 - 12 - 31)),
        EraPreset::Y2012To2014 => (date!(2012 - 01 - 01), date!(2014 - 12 - 31)),
        EraPreset::Y2015To2017 => (date!(2015 - 01 - 01), date!(2017 - 12 - 31)),
        EraPreset::Y2018To2020 => (date!(2018 - 01 - 01), date!(2020 - 12 - 31)),
        EraPreset::Y2021To2024 => (date!(2021 - 01 - 01), date!(2024 - 12 - 31)),
    }
}

/// The next preset after `p` in `ALL_PRESETS`' cycle order, wrapping to the first after the last —
/// the Declare flow's "cycle era preset" key action.
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
        assert_eq!(next_preset(EraPreset::Y2021To2024), EraPreset::Y2009To2011);
    }
}
