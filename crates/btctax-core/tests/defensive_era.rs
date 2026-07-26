//! KATs for the Defensive Filing Wizard's era-preset table (Task 8, DFW-D9): `era::era_window` maps
//! EVERY preset to a concrete `[start, end]` window (KAT a). PRIVACY: no real data — pure date table.

use btctax_core::defensive::era::{era_window, EraPreset, ALL_PRESETS};
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
    // The OWNER-ratified post-cutover bucket: starts EXACTLY at the pooling cutover, ends at the last
    // day of the newest tax year this app can file (a concrete bound — `era_window` is clock-free).
    assert_eq!(
        era_window(EraPreset::Y2025Onward),
        (date!(2025 - 01 - 01), date!(2025 - 12 - 31))
    );
    assert_eq!(
        era_window(EraPreset::Y2025Onward).0,
        btctax_core::conventions::TRANSITION_DATE,
        "the post-cutover bucket must START at TRANSITION_DATE itself, so nothing straddles it"
    );
}

#[test]
fn era_window_is_a_pure_total_function_over_every_variant() {
    // A grep/enumeration guard: every ALL_PRESETS entry must produce SOME concrete window (era_window
    // is total — no variant panics or silently falls through).
    for &p in ALL_PRESETS {
        let (s, e) = era_window(p);
        assert!(
            s <= e,
            "{p:?} must produce a well-formed [start,end] window"
        );
    }
    assert_eq!(
        ALL_PRESETS.len(),
        6,
        "sanity: the six OWNER-ratified calendar buckets (2009-2011 .. 2025-onward)"
    );
}

/// ★ REPLACES `all_presets_end_strictly_before_the_pre2025_pooling_cutover`, which asserted every
/// preset ends `< TRANSITION_DATE`. The OWNER-ratified `Y2025Onward` bucket deliberately violates that.
///
/// ★ CORRECTED (whole-branch r2 tax I-1): this doc used to justify the bucket with "a pre-2025 tranche
/// cannot cover a post-2025 disposal in the same wallet (`pools::pool_key` puts them in different
/// pools)". That is FALSE under the default Path A — `project::transition::seed_transition` drains every
/// Universal residue lot into `PoolKey::Wallet(lot.wallet)` at the cutover and explicitly preserves
/// `BasisSource::EstimatedConservative` doing it, which
/// `kat_tranche.rs::tranche_tag_survives_2025_path_a_seed_and_reaches_a_2025_disposal_leg` pins green.
/// The bucket's real justifications (truthful 2025+ attestation without ~150 nudges; not forfeiting Rev.
/// Proc. 2024-28 eligibility via `pre2025_tranche_exists`) are in `era::EraPreset::Y2025Onward`'s doc.
/// Only the RATIONALE was wrong — the bucket, and this guard, are unchanged.
///
/// The property the old guard was really protecting is preserved here in its correct, stronger form:
/// no window may STRADDLE the cutover. Every bucket lies entirely before `TRANSITION_DATE` or entirely
/// on/after it, so a declared tranche's lot always lands in exactly ONE pooling era (Universal before,
/// per-wallet from it on) rather than spanning the split.
#[test]
fn no_preset_window_straddles_the_pooling_cutover() {
    let cutover = btctax_core::conventions::TRANSITION_DATE;
    for &p in ALL_PRESETS {
        let (s, e) = era_window(p);
        assert!(
            e < cutover || s >= cutover,
            "{p:?} ({s}..{e}) STRADDLES the {cutover} pooling cutover — its lot would span the \
             Universal/per-wallet pool split"
        );
    }
    // …and both sides of the split are actually reachable from the table (the invariant above is
    // vacuously satisfiable by an all-pre-2025 table, which is exactly the gap the owner closed).
    assert!(
        ALL_PRESETS.iter().any(|&p| era_window(p).1 < cutover),
        "at least one bucket must lie entirely BEFORE the cutover"
    );
    assert!(
        ALL_PRESETS
            .iter()
            .any(|&p| era_window(p).0 >= cutover),
        "at least one bucket must lie entirely ON/AFTER the cutover — otherwise a post-2025 shortfall \
         has no applicable preset at all"
    );
}

// ★ whole-branch arch M-2: `next_preset_cycles_oldest_to_newest_then_wraps` was DELETED here alongside
// the `era::next_preset` accessor it was the only remaining caller of (with its unit-test twin in
// `era.rs`). The picker takes an explicit numbered pick; nothing in production cycles.
