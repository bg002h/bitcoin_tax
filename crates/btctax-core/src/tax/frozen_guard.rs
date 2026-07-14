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
pub const FROZEN_TYPES_SHA256: &str =
    "0d51da823e5efcd23ef500a35c50009540c0f0c22ca20871d254bdf243c86b9c";
/// SHA-256 of `tax/compute.rs` (frozen).
pub const FROZEN_COMPUTE_SHA256: &str =
    "38e87b7d7954988c312d0dbdac103b52ad206aa014aad791594cb4f72ae1e62e";
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
