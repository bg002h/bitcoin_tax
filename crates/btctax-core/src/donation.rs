//! `DonationDetails` — Form 8283 Section-B appraiser + structured-donee metadata for a
//! donation event. Pure type + completeness logic (no fold/projection coupling — this never
//! enters `LedgerState`). Storage lives in `btctax-cli::donation_details` (side-table,
//! mirrors the `TaxProfile` type-in-core / storage-in-cli pattern).
use crate::conventions::TaxDate;
use crate::forms::Form8283Section;
use serde::{Deserialize, Serialize};

/// Structured Form 8283 Section-B data attached to a donation event via the
/// `reconcile set-donation-details` command. Stored in the `donation_details` side-table
/// (keyed by the donation's `EventId::canonical()`) — never enters the fold or projection.
///
/// `donee_name` and `appraiser_name` are the only REQUIRED fields (enforced by the CLI).
/// All other fields are optional (`#[serde(default)]`) for forward compatibility — a future
/// vault written with extra fields round-trips cleanly on an older binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DonationDetails {
    /// Donee organization name (Part IV; required).
    pub donee_name: String,
    /// Donee organization mailing address (Part IV; optional).
    #[serde(default)]
    pub donee_address: Option<String>,
    /// Donee EIN (Part IV; required for Section-B completeness).
    #[serde(default)]
    pub donee_ein: Option<String>,
    /// Qualified appraiser name (Part III; required).
    pub appraiser_name: String,
    /// Appraiser mailing address (Part III; optional).
    #[serde(default)]
    pub appraiser_address: Option<String>,
    /// Appraiser TIN (SSN or EIN; Part III §6695A; required-or-PTIN for Section-B completeness).
    #[serde(default)]
    pub appraiser_tin: Option<String>,
    /// Appraiser PTIN (Part III §6695A; satisfies the TIN-or-PTIN requirement).
    #[serde(default)]
    pub appraiser_ptin: Option<String>,
    /// Appraiser qualifications declaration (§170(f)(11)(E); required for Section-B completeness).
    #[serde(default)]
    pub appraiser_qualifications: Option<String>,
    /// Date the qualified appraisal was made (Part III; required for Section-B completeness).
    #[serde(default)]
    pub appraisal_date: Option<TaxDate>,
    /// Optional user-supplied FMV determination method override.
    /// When `Some`, overrides the section-derived default (`"qualified appraisal"` / `""`) on
    /// the carrier row — resolves the Chunk-1 Section-A `fmv_method` deferral.
    #[serde(default)]
    pub fmv_method_override: Option<String>,
}

impl DonationDetails {
    /// Whether the stored details are sufficient to mark the Form 8283 carrier row as
    /// **not** needing review (`needs_review == false`), per the section-aware completeness rules:
    ///
    /// **Section B** (year-aggregate > $5,000 — qualified appraisal required under
    /// §170(f)(11)(D)/§6695A): all of the following must be present —
    /// - `appraiser_name` non-empty (already required by the command)
    /// - `appraiser_tin` OR `appraiser_ptin` (§6695A appraiser identifier)
    /// - `appraisal_date`
    /// - `appraiser_qualifications` (§170(f)(11)(E))
    /// - `donee_ein` (Part IV)
    ///
    /// A skeletal entry (only `donee_name` + `appraiser_name`) on a Section-B donation leaves
    /// `needs_review == true` — the appraiser declaration is incomplete. This upholds the
    /// "honest gaps, never fabricated" invariant.
    ///
    /// **Section A** (year-aggregate ≤ $5,000 — no qualified appraiser required): complete on
    /// presence — details present ⇒ complete (`true`).
    pub fn is_review_complete(&self, section: Form8283Section) -> bool {
        match section {
            Form8283Section::B => {
                !self.appraiser_name.is_empty()
                    && (self.appraiser_tin.is_some() || self.appraiser_ptin.is_some())
                    && self.appraisal_date.is_some()
                    && self.appraiser_qualifications.is_some()
                    && self.donee_ein.is_some()
            }
            Form8283Section::A => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    fn full_section_b() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: Some("123 Main St, Anytown USA".into()),
            donee_ein: Some("12-3456789".into()),
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: Some("456 Appraiser Ave".into()),
            appraiser_tin: Some("987-65-4321".into()),
            appraiser_ptin: Some("P01234567".into()),
            appraiser_qualifications: Some("Certified bitcoin appraiser, 10 yrs exp".into()),
            appraisal_date: Some(date!(2025 - 06 - 01)),
            fmv_method_override: None,
        }
    }

    fn skeletal() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: None,
            donee_ein: None,
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: None,
            appraiser_ptin: None,
            appraiser_qualifications: None,
            appraisal_date: None,
            fmv_method_override: None,
        }
    }

    /// Full Section-B details (all §6695A fields present) → complete.
    #[test]
    fn section_b_full_details_is_complete() {
        assert!(full_section_b().is_review_complete(Form8283Section::B));
    }

    /// Skeletal Section-B (only donee_name + appraiser_name) → NOT complete.
    /// This is the [R0-I1] honest-gap lock: partial Section-B must NOT flip needs_review to false.
    #[test]
    fn section_b_skeletal_is_not_complete() {
        assert!(!skeletal().is_review_complete(Form8283Section::B));
    }

    /// Missing BOTH appraiser_tin AND appraiser_ptin → not complete (§6695A requires one).
    #[test]
    fn section_b_missing_tin_and_ptin_is_not_complete() {
        let mut d = full_section_b();
        d.appraiser_tin = None;
        d.appraiser_ptin = None;
        assert!(!d.is_review_complete(Form8283Section::B));
    }

    /// PTIN alone (no TIN) satisfies the §6695A TIN-or-PTIN requirement.
    #[test]
    fn section_b_ptin_alone_satisfies_tin_or_ptin() {
        let mut d = full_section_b();
        d.appraiser_tin = None; // PTIN only
        assert!(d.is_review_complete(Form8283Section::B));
    }

    /// TIN alone (no PTIN) satisfies the §6695A TIN-or-PTIN requirement.
    #[test]
    fn section_b_tin_alone_satisfies_tin_or_ptin() {
        let mut d = full_section_b();
        d.appraiser_ptin = None; // TIN only
        assert!(d.is_review_complete(Form8283Section::B));
    }

    /// Missing donee_ein → not complete (Part IV requires it for Section B).
    #[test]
    fn section_b_missing_donee_ein_is_not_complete() {
        let mut d = full_section_b();
        d.donee_ein = None;
        assert!(!d.is_review_complete(Form8283Section::B));
    }

    /// Missing appraisal_date → not complete.
    #[test]
    fn section_b_missing_appraisal_date_is_not_complete() {
        let mut d = full_section_b();
        d.appraisal_date = None;
        assert!(!d.is_review_complete(Form8283Section::B));
    }

    /// Missing appraiser_qualifications → not complete (§170(f)(11)(E)).
    #[test]
    fn section_b_missing_qualifications_is_not_complete() {
        let mut d = full_section_b();
        d.appraiser_qualifications = None;
        assert!(!d.is_review_complete(Form8283Section::B));
    }

    /// Section A: complete on presence — no appraiser required.
    #[test]
    fn section_a_skeletal_is_complete() {
        assert!(skeletal().is_review_complete(Form8283Section::A));
    }

    /// Section A: full Section-B-grade details are also complete (superset).
    #[test]
    fn section_a_full_details_is_complete() {
        assert!(full_section_b().is_review_complete(Form8283Section::A));
    }
}
