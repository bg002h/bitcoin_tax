//! KATs for the Defensive Filing Wizard's era-preset table (Task 8, DFW-D9): `era::era_window` maps
//! EVERY preset to a concrete `[start, end]` window (KAT a). PRIVACY: no real data — pure date table.

use btctax_core::defensive::era::{era_window, next_preset, EraPreset, ALL_PRESETS};
use time::macros::date;

#[test]
fn era_window_maps_every_preset_to_a_concrete_window() {
    assert_eq!(
        era_window(EraPreset::Y2009To2011),
        (date!(2009 - 01 - 03), date!(2011 - 12 - 31))
    );
    assert_eq!(
        era_window(EraPreset::Y2012To2014),
        (date!(2012 - 01 - 01), date!(2014 - 12 - 31))
    );
    assert_eq!(
        era_window(EraPreset::Y2015To2017),
        (date!(2015 - 01 - 01), date!(2017 - 12 - 31))
    );
    assert_eq!(
        era_window(EraPreset::Y2018To2020),
        (date!(2018 - 01 - 01), date!(2020 - 12 - 31))
    );
    assert_eq!(
        era_window(EraPreset::Y2021To2024),
        (date!(2021 - 01 - 01), date!(2024 - 12 - 31))
    );
}

#[test]
fn era_window_is_a_pure_total_function_over_every_variant() {
    // A grep/enumeration guard: every ALL_PRESETS entry must produce SOME concrete window (era_window
    // is total — no variant panics or silently falls through).
    for &p in &ALL_PRESETS {
        let (s, e) = era_window(p);
        assert!(
            s <= e,
            "{p:?} must produce a well-formed [start,end] window"
        );
    }
    assert_eq!(ALL_PRESETS.len(), 5, "sanity: five provisional presets");
}

#[test]
fn all_presets_end_strictly_before_the_pre2025_pooling_cutover() {
    // Every preset is a PRE-2025 era window (the safe-harbor / universal-pool boundary the Declare
    // flow's safe-harbor precheck cares about, DFW-D9) — none of the provisional buckets accidentally
    // reaches into the post-transition per-wallet pooling era.
    for &p in &ALL_PRESETS {
        let (_s, e) = era_window(p);
        assert!(
            e < btctax_core::conventions::TRANSITION_DATE,
            "{p:?} must end before the 2025-01-01 pooling cutover"
        );
    }
}

#[test]
fn next_preset_cycles_oldest_to_newest_then_wraps() {
    let mut p = ALL_PRESETS[0];
    for &expected in &ALL_PRESETS[1..] {
        p = next_preset(p);
        assert_eq!(p, expected);
    }
    // Wraps back to the first after the last.
    assert_eq!(next_preset(p), ALL_PRESETS[0]);
}
