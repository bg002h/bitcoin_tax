//! **FROZEN-ENGINE CONTENT GUARD** (full-return v1, Phase 0 task 0).
//!
//! The full-return build is strictly ADDITIVE: it wraps the crypto-**delta** engine without editing it
//! (SPEC_full_return §2). This module pins the exact **content** (SHA-256) of the frozen delta-path files
//! so any edit — even one that preserves the public API — trips a test. The invariant is "never *edit*",
//! which a public-surface check would miss (plan-review I3).
//!
//! **Frozen set** (enumerated): `tax/types.rs` (`TaxProfile`, `TaxResult`), `tax/compute.rs`
//! (`compute_tax_year`, `net_1222`, `ordinary_tax_on`, `preferential_tax`, the NIIT closure), and
//! `tax/se.rs` (`compute_se_tax`, `addl`). The delta-only helpers all live inside these three files, so the
//! three-file pin covers them (confirmed plan re-review r2). `what-if` / pseudo-reconcile / the existing
//! crypto tests are "never alter" but not content-pinned — they consume the frozen contract and would break
//! loudly (FOLLOWUP pm-r2-m4).
//!
//! **Exception process:** a legitimate change to a frozen file (should be exceedingly rare in v1) is its own
//! separately-reviewed commit that ALSO updates the pin below — never a silent pin bump folded into other
//! work.

/// SHA-256 of `tax/types.rs` (frozen). Update only via the documented exception process.
///
/// ★ **EXCEPTION 1, 2026-07-30 — the all-in LTCG marginal rate.** `MarginalRates` gained
/// `niit_at_margin` and the `ltcg_all_in()` accessor, so `report --tax-year` can headline the rate a
/// filer sizing a sale must actually reserve against (§1(h) 20% + §1411 3.8% = **23.8%**, not 0.20).
/// **Strictly additive and display-only**: no existing field changed, no arithmetic moved, and the
/// TY2024 golden matrix is byte-unchanged (`golden_returns.rs`). Recorded here rather than folded
/// silently, per the module's exception process.
pub const FROZEN_TYPES_SHA256: &str =
    "51b912cce4b7cd3606e85de133430d6bb2361885b0e27d8f77caad0f2c148f57";
/// SHA-256 of `tax/compute.rs` (frozen).
///
/// ★ **EXCEPTION 1, 2026-07-30** — the same change: `compute_tax_year` populates the new
/// `niit_at_margin` flag (`magi_with > thr && nii_with >= 0`). It is read by nothing but the
/// renderers; every tax figure the function returns is bit-identical.
pub const FROZEN_COMPUTE_SHA256: &str =
    "97b5cd914de53365b2394488c29f377a0373f9c4abfd1a1c16c648766f89f9c2";
/// SHA-256 of `tax/se.rs` (frozen).
pub const FROZEN_SE_SHA256: &str =
    "3aba83c20bee7816d6d7ec716867bcfb5fef8f360f1cc5c4aa00559f51795889";

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn hash(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The frozen delta-path files must be byte-identical to their pinned fingerprints. A failure here
    /// means the additive-only invariant (SPEC §2) was violated — revert the edit, or, if the change is
    /// genuinely intended, update the pin in its own reviewed commit (see module docs).
    #[test]
    fn frozen_engine_files_are_unchanged() {
        // include_bytes! embeds the sibling files at compile time — hermetic, no runtime file IO.
        assert_eq!(
            hash(include_bytes!("types.rs")),
            FROZEN_TYPES_SHA256,
            "tax/types.rs was edited — the delta engine is FROZEN (SPEC_full_return §2)"
        );
        assert_eq!(
            hash(include_bytes!("compute.rs")),
            FROZEN_COMPUTE_SHA256,
            "tax/compute.rs was edited — the delta engine is FROZEN (SPEC_full_return §2)"
        );
        assert_eq!(
            hash(include_bytes!("se.rs")),
            FROZEN_SE_SHA256,
            "tax/se.rs was edited — the delta engine is FROZEN (SPEC_full_return §2)"
        );
    }
}
