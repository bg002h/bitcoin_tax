//! **Form 8283 Section A — the transcription, and the routing decision it forces.**
//!
//! FR-200(b). A $600 bag of used clothing given to a thrift store used to refuse the ENTIRE packet,
//! including the Form 8949 and Schedule D that are the reason this product exists, because btctax held
//! no property details for a noncash gift that did not come from the ledger and so could produce no
//! Section A rows. The refusal was right for what it saw — an incomplete required attachment is a
//! §170(f)(11) denial risk, and Regulations section 1.170A-16(c)(3)(v) now *requires* Section A to be
//! fully completed — so the fix is to **hold the property details**, never to loosen the gate.
//!
//! ## What this module is
//!
//! `CLAUDE.md`: *"one field per numbered line, named for the line, in the form's own numbering,
//! carrying the official instruction text verbatim as its doc comment."* [`SectionARow`] is that
//! struct for **Form 8283 line 1**, columns (a) through (i). The column headings below are verbatim
//! from the text layer of the bundled PDF (`design/forms/extract/f8283--2024.txt:29-44` and
//! `f8283--2025.txt:26-41`); the Section A column set is identical on the Rev. 12-2023 and
//! Rev. 12-2025 revisions, which is **asserted rather than assumed** — see
//! `btctax-forms/tests/f8283_section_a.rs`.
//!
//! [`NoncashGiftProperty`] is the other half: what a filer must be **asked** so that those columns can
//! be filled. *"If the form asks something our input surface cannot answer, collect it. That is
//! following instructions, not scope creep."*
//!
//! ## ★★ THE TWO $500 THRESHOLDS, which are not the same threshold
//!
//! They are modelled as two constants with two readers, because they answer two questions:
//!
//! | constant | question | measured over |
//! |---|---|---|
//! | [`crate::tax::printed::FORM_8283_THRESHOLD`] | is a Form 8283 filed **at all**? | the **TOTAL** deduction for all contributed property |
//! | [`COLUMNS_EFG_PER_ITEM_THRESHOLD`] | must columns **(e), (f), (g)** be completed? | **ONE item** (or group of similar items) |
//!
//! The filing one is the form's own header (`f8283--2024.txt:12-13`): *"Attach one or more Forms 8283
//! to your tax return if you claimed a total deduction of over $500 for all contributed property."*
//! The per-item one is the note printed between the two halves of line 1 (`f8283--2024.txt:39`):
//! *"Note: If the amount you claimed as a deduction for an item is $500 or less, you do not have to
//! complete columns (e), (f), and (g)."*
//!
//! A $600 bag of clothes is over the **per-item** line and needs (e), (f) and (g). Four $200 bags
//! totalling $800 must **file** the form and need not complete those columns. Both are KATs
//! (`the_six_hundred_dollar_bag_of_clothes_files_section_a_with_columns_efg`,
//! `four_two_hundred_dollar_bags_file_the_form_and_omit_columns_efg`), and together they are what
//! holds the two constants apart: no test that exercises only one of them can tell them apart,
//! because on today's form both read `$500`.
//!
//! ## ★ "Optional" is not "blank"
//!
//! When the per-item carve-out applies, columns (e)(f)(g) need not be *completed* — but per
//! `CLAUDE.md`'s *"blank is the normal case"*, a blank still needs a determinate **provenance**.
//! [`SectionAColumnsEfg`] is that provenance, in the type system: `NotRequired…` is a recorded
//! decision that names the form's own sentence, and it is the ONLY way to reach an empty (e)(f)(g).
//! "Nobody ever collected it" is not representable — it is
//! [`NoncashGiftRefusal::ColumnsEfgRequiredButNotCollected`], a refusal. Two blanks that look
//! identical on the printed page are therefore not identical here.
//!
//! ## What files, and what still refuses
//!
//! [`section_a_row`] is the single decision point. Every refusal it can return names its own
//! condition ([`NoncashGiftRefusal`]); no arm widens into silence.

use crate::conventions::{TaxDate, Usd};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// **The PER-ITEM carve-out threshold for columns (e), (f) and (g)** — `$500`.
///
/// Form 8283 line 1, verbatim (`design/forms/extract/f8283--2024.txt:39`, identical at
/// `f8283--2025.txt:38`):
///
/// > Note: If the amount you claimed as a deduction for an item is $500 or less, you do not have to
/// > complete columns (e), (f), and (g).
///
/// i8283's column (d) note repeats it per item (`i8283--2024.txt:909-912`): *"If the amount you
/// claimed as a deduction for the item is $500 or less, you do not have to complete columns (e), (f),
/// and (g)."* — and column (g) states the same rule from the other side (`i8283--2024.txt:926`):
/// *"For items over $500, enter your cost or adjusted basis."*
///
/// ★★ **This is NOT [`crate::tax::printed::FORM_8283_THRESHOLD`]**, which is measured over the total
/// for ALL contributed property and decides whether the form is filed at all. The module docs carry
/// the two-row table; the two KATs named there are what hold them apart.
pub const COLUMNS_EFG_PER_ITEM_THRESHOLD: Usd = dec!(500);

/// **The per-item Section B threshold** — `$5,000`, strict `>`.
///
/// Form 8283 Section A's own heading (`f8283--2024.txt:26-29`): *"List in this section only an item
/// (or a group of similar items) for which you claimed a deduction of $5,000 or less."* i8283
/// (`i8283--2024.txt:208-218`): *"Use Section A to report donations of property for which you claimed
/// a deduction of $5,000 or less per item or group of similar items"* … *"Use Section B to report
/// donations of property for which you claimed a deduction of more than $5,000 per item or group of
/// similar items."*
///
/// ★ It is a distinct NAME for [`crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD`], not a second
/// number: it is *defined as* that constant, so the two cannot drift while both claim to be
/// §170(f)(11)(C)'s *"more than $5,000"*.
pub const SECTION_B_PER_ITEM_THRESHOLD: Usd = crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD;

// ─────────────────────────────────────────────────────────────────────────────────────────────────
//  The column-provenance table — the conformance denominator (CLAUDE.md "blank is the normal case")
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// What holds one Section A column. *"Every line has a determinate PROVENANCE — collected from the
/// filer, computed from named lines, a constant the form prints, or a refusal."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnDisposition {
    /// Collected from the filer and always printed when a Section A row prints. The `&str` names the
    /// [`NoncashGiftProperty`] field that carries it.
    Collected(&'static str),
    /// Collected and printed **unless** the per-item carve-out applies — see
    /// [`COLUMNS_EFG_PER_ITEM_THRESHOLD`] and [`SectionAColumnsEfg`].
    CollectedUnlessCarveOut(&'static str),
    /// **Never printed, and nothing is laundered as a blank**: the property class this column
    /// describes REFUSES. The `&str` names the [`NoncashGiftRefusal`] variant.
    RefusedProperty(&'static str),
}

/// One row of the Section A column census: the form's own letter, the form's own heading verbatim,
/// and what holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionAColumn {
    /// The column letter the form prints: `'a'` … `'i'`.
    pub letter: char,
    /// The column heading, **verbatim from the text layer**, reassembled across the heading's wrapped
    /// lines within the column's own x-span. `btctax-forms/tests/f8283_section_a.rs` derives the same
    /// set from the extract and demands the two agree, in both directions.
    pub heading: &'static str,
    /// What holds the column.
    pub disposition: ColumnDisposition,
}

/// **Every column Form 8283 line 1 prints, and what holds each one.**
///
/// ★ This table is the *claim*; it is not the measurement. `btctax-forms`'s
/// `every_section_a_column_of_the_form_is_accounted_for` re-derives the `(letter, heading)` pairs from
/// `design/forms/extract/f8283--<year>.txt` for every supported year and asserts set equality both
/// ways — so a column added by a future revision reds, a column dropped from this table reds, and a
/// heading swapped between two letters reds.
///
/// ★ **What it does not cover, stated rather than implied** (`CLAUDE.md`): it checks the heading text
/// and the letter set, not that `disposition` is *true* of the emitter. That is held separately, by
/// this module's KATs and by `btctax-forms`'s fill tests.
pub const SECTION_A_COLUMNS: &[SectionAColumn] = &[
    SectionAColumn {
        letter: 'a',
        heading: "(a) Name and address of the donee organization",
        disposition: ColumnDisposition::Collected("donee_name_and_address"),
    },
    SectionAColumn {
        letter: 'b',
        heading:
            "(b) If donated property is a vehicle (see instructions), check the box. Also enter \
                  the vehicle identification number (unless Form 1098-C is attached).",
        // ★ The vehicle box and its VIN cells are NOT in `f8283.map.toml`'s `[section_a]`, so nothing
        //   can write them — and `verify::no_unmapped_filled` fails closed if anything tries. A
        //   vehicle therefore refuses rather than printing an unchecked box beside a vehicle
        //   description, which would be a Form 8283 contradicting its own row.
        disposition: ColumnDisposition::RefusedProperty("NoncashGiftRefusal::Vehicle"),
    },
    SectionAColumn {
        letter: 'c',
        heading:
            "(c) Description and condition of donated property (For a vehicle, enter the year, \
                  make, model, and mileage. For securities and other property, see instructions.)",
        disposition: ColumnDisposition::Collected("description_and_condition"),
    },
    SectionAColumn {
        letter: 'd',
        heading: "(d) Date of the contribution",
        disposition: ColumnDisposition::Collected("date_of_contribution"),
    },
    SectionAColumn {
        letter: 'e',
        heading: "(e) Date acquired by donor (mo., yr.)",
        disposition: ColumnDisposition::CollectedUnlessCarveOut("date_acquired_by_donor"),
    },
    SectionAColumn {
        letter: 'f',
        heading: "(f) How acquired by donor",
        disposition: ColumnDisposition::CollectedUnlessCarveOut("how_acquired_by_donor"),
    },
    SectionAColumn {
        letter: 'g',
        heading: "(g) Donor\u{2019}s cost or adjusted basis",
        disposition: ColumnDisposition::CollectedUnlessCarveOut("cost_or_adjusted_basis"),
    },
    SectionAColumn {
        letter: 'h',
        heading: "(h) Fair market value (see instructions)",
        disposition: ColumnDisposition::Collected("fair_market_value"),
    },
    SectionAColumn {
        letter: 'i',
        heading: "(i) Method used to determine the fair market value",
        disposition: ColumnDisposition::Collected("method_used_to_determine_fmv"),
    },
];

// ─────────────────────────────────────────────────────────────────────────────────────────────────
//  What the filer is asked
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// **Which kind of property was given** — the question that decides whether Section A may carry it.
///
/// ★★ This enumerates the classes i8283 *names*, with `OtherProperty` as the residue — and it is
/// deliberately NOT a bare "other, assume it is fine". Every class Section A cannot honestly carry has
/// its own variant and its own refusal, because *"widening an exemption is never the safe edit"*: a
/// filer whose property is one of the refusing classes must be told **which** class it is.
///
/// i8283 *"Section A. Include in Section A only the following items."* (`i8283--2024.txt:262-292`)
/// enumerates the over-$5,000 exceptions: securities with published quotations, a qualified vehicle
/// with a contemporaneous written acknowledgment, intellectual property, and inventory or property
/// held primarily for sale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoncashPropertyKind {
    /// Clothing or a household item.
    ///
    /// ★★★ **§170(f)(16) makes the CONDITION a gate, not a description.** i8283
    /// (`i8283--2024.txt:735-743`): *"Generally, you cannot claim a deduction for clothing or
    /// household items you donate unless the clothing or household items are in good used condition
    /// or better."* … *"However, you can claim a deduction for a contribution of an item of clothing
    /// or a household item that is not in good used condition or better if your claimed value is more
    /// than $500 and you substantiate that value with a qualified appraisal and Form 8283, Section
    /// B."* Pub. 561 says the same (`Pub561_Value_of_Donated_Property.txt:317-320`): *"You cannot take
    /// an income tax charitable contribution deduction for an item of clothing unless it is in good
    /// used condition or better."*
    ///
    /// So the answer routes three ways and only one of them files here — see [`section_a_row`].
    ClothingOrHouseholdItem {
        /// §170(f)(16)(A) / i8283: is the item *"in good used condition or better"*?
        good_used_condition_or_better: bool,
    },
    /// A publicly traded security — `i8283--2024.txt:265-276` items 2.a-2.d. Section A carries these
    /// *"even if the claimed value was more than $5,000 per item (or group of similar items)"*.
    ///
    /// ★★★ **AND IT STILL REFUSES HERE — a recorded boundary, not an omission** (`CLAUDE.md`: *"state,
    /// in the source, exactly what it covers and what it does not"*). Two of Section A's columns ask a
    /// security something btctax has never collected:
    ///
    /// * column **(g)**, i8283 (`i8283--2024.txt:926-929`): *"Do not complete this column for publicly
    ///   traded securities held more than 12 months, unless you elect to limit your deduction cost
    ///   basis. See section 170(b)(1)(C)(iii)."* Whether (g) prints at all therefore turns on a
    ///   §170(b)(1)(C)(iii) election btctax does not model, and printing a basis the form says not to
    ///   complete is the Section B column (i) defect again — an entry the form did not ask for, sworn
    ///   to under §6065.
    /// * column **(c)**, i8283 (`i8283--2024.txt:876-882`): for securities it wants the company name,
    ///   the number of shares, the kind of security, whether it is a mutual fund share and whether it
    ///   is regularly traded — five sub-fields, not one free-text line.
    /// * column **(e)**, i8283 (`i8283--2024.txt:920-921`): *"For publicly traded securities, enter
    ///   only if you held the securities for more than 12 months."*
    ///
    /// Admitting a security would therefore file a Section A row whose (c), (e) and (g) are each a
    /// guess. Closing that is a separate piece of work with its own questions to collect; until then
    /// this refuses and says so.
    PubliclyTradedSecurity,
    /// A vehicle, boat or airplane (`i8283--2024.txt:277-281`; §170(f)(12)). **Refuses** — see
    /// column (b)'s disposition in [`SECTION_A_COLUMNS`].
    Vehicle,
    /// Intellectual property (`i8283--2024.txt:282`; §170(e)(1)(B)(iii)). **Refuses.**
    IntellectualProperty,
    /// Inventory or property held primarily for sale to customers in the ordinary course of a trade or
    /// business (`i8283--2024.txt:283-285`; §1221(a)(1)). **Refuses.**
    InventoryOrHeldForSale,
    /// Any other property — the residue, and the class a Section A row files on when the claimed
    /// deduction is `$5,000` or less.
    OtherProperty,
}

/// **Column (e) — *"Date acquired by donor (mo., yr.)"***, which the form lets a filer answer with a
/// word instead of a date.
///
/// i8283 column (e) (`i8283--2024.txt:916-919`): *"If you are donating a group of similar items and
/// you acquired the items on various dates (but have held all the items for at least 12 months), you
/// can enter"* `Various`. A bag of used clothing bought over years is exactly that case, so the form's
/// own answer is modelled rather than forcing the filer to invent one date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateAcquiredByDonor {
    /// An approximate acquisition date. Printed `MM/YYYY` — the column's own *"(mo., yr.)"* format.
    On(TaxDate),
    /// The form's own word for a group of similar items acquired on various dates, all held ≥ 12
    /// months.
    Various,
}

impl DateAcquiredByDonor {
    /// The word the form prints for [`Self::Various`]. i8283 column (e) names it in quotes; a
    /// capital-V `Various` is the form's own spelling.
    pub const VARIOUS_WORD: &'static str = "Various";
}

/// **The Form 8283 Section A property details for one non-crypto noncash gift** — one
/// [`crate::tax::return_inputs::CharitableGift`] entry is one item **or one group of similar items**,
/// which is exactly what one Section A row is (`i8283--2024.txt:314-323`, *Similar Items of
/// Property*).
///
/// Every field is named for the column it fills. Absent (`None`) is *"never collected"*, which is a
/// refusal and never a silent blank — [`section_a_row`] is the only thing that turns these into a
/// printable row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoncashGiftProperty {
    /// Which kind of property — the routing answer. See [`NoncashPropertyKind`].
    pub kind: NoncashPropertyKind,
    /// **Column (a)** — *"Name and address of the donee organization"*.
    pub donee_name_and_address: String,
    /// **Column (c)** — *"Description and condition of donated property (For a vehicle, enter the
    /// year, make, model, and mileage. For securities and other property, see instructions.)"*
    ///
    /// i8283 (`i8283--2024.txt:861-864`): *"Describe the property in sufficient detail. The greater
    /// the value of the property, the more detail you must provide."*
    pub description_and_condition: String,
    /// **Column (d)** — *"Date of the contribution"*. i8283 (`i8283--2024.txt:905-907`): *"Enter the
    /// date you contributed the property. If you made contributions on various dates, enter each
    /// contribution and its date on a separate row."*
    pub date_of_contribution: TaxDate,
    /// **Column (e)** — *"Date acquired by donor (mo., yr.)"*. `None` ⇒ not collected: permitted only
    /// under the per-item carve-out, refused otherwise.
    #[serde(default)]
    pub date_acquired_by_donor: Option<DateAcquiredByDonor>,
    /// **Column (f)** — *"How acquired by donor"*. i8283 (`i8283--2024.txt:923-925`): *"State how you
    /// acquired the property. This could be by purchase, gift, inheritance, or exchange."* Free text,
    /// because the form's is. `None` ⇒ not collected.
    #[serde(default)]
    pub how_acquired_by_donor: Option<String>,
    /// **Column (g)** — *"Donor's cost or adjusted basis"*. `None` ⇒ not collected.
    #[serde(default)]
    pub cost_or_adjusted_basis: Option<Usd>,
    /// **Column (h)** — *"Fair market value (see instructions)"*. i8283 (`i8283--2024.txt:936-939`):
    /// *"Enter the FMV of the property on the date you donated it. You must attach a statement if you
    /// were required to reduce the FMV to figure the amount of your deduction."*
    ///
    /// ★★ This is the FMV, **not** the claimed deduction on the gift's `amount`. The two part company
    /// exactly where the form demands an attached statement btctax cannot produce, which is why a
    /// mismatch REFUSES ([`NoncashGiftRefusal::FmvReducedNeedsAttachedStatement`]) instead of quietly
    /// printing one number in the place of the other.
    pub fair_market_value: Usd,
    /// **Column (i)** — *"Method used to determine the fair market value"*.
    ///
    /// i8283 (`i8283--2024.txt:944-950`): *"Enter the method(s) you used to determine the FMV."* …
    /// *"Examples of entries to make include"* `Appraisal`, `Thrift shop value` *"(for clothing or
    /// household items)"*, `Catalog` *"(for stamp or coin collections)"*, or `Comparable sales`
    /// *"(for real estate and other kinds of assets). See Pub. 561."* Pub. 561 on used clothing
    /// (`Pub561_Value_of_Donated_Property.txt:310-318`): *"The price that buyers of used items
    /// actually pay in used clothing stores, such as consignment or thrift shops, is an indication of
    /// the value."*
    ///
    /// Free text, and REQUIRED: an empty (i) refuses. Regulations section 1.170A-16(c)(3)(v) requires
    /// Section A to be *fully* completed, and i8283's *How To Complete* (`i8283--2024.txt:172-175`)
    /// forbids the alternative outright: *"You may not indicate that the information is"* `available
    /// upon request`.
    pub method_used_to_determine_fmv: String,
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
//  The printed row
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// **Columns (e), (f) and (g) — completed, or not required.** The provenance of an (e)(f)(g) blank,
/// in the type system.
///
/// There is no third variant, and that is the point: *"nobody ever collected it"* cannot be
/// represented here, because [`section_a_row`] turns it into
/// [`NoncashGiftRefusal::ColumnsEfgRequiredButNotCollected`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionAColumnsEfg {
    /// The three columns are completed.
    Completed {
        /// **(e)** *"Date acquired by donor (mo., yr.)"*.
        col_e_date_acquired_by_donor: DateAcquiredByDonor,
        /// **(f)** *"How acquired by donor"*.
        col_f_how_acquired_by_donor: String,
        /// **(g)** *"Donor's cost or adjusted basis"*.
        col_g_cost_or_adjusted_basis: Usd,
    },
    /// **Blank, by the form's own carve-out.** *"Note: If the amount you claimed as a deduction for an
    /// item is $500 or less, you do not have to complete columns (e), (f), and (g)."*
    NotRequiredDeductionAtOrUnderFiveHundred,
}

/// **Form 8283, line 1 — one row (A, B, C or D) of Section A.** One field per column, in the form's
/// own lettering, with the form's own heading as the doc comment.
///
/// Column **(b)** has no field: it is the vehicle box, and a vehicle refuses ([`SECTION_A_COLUMNS`],
/// letter `'b'`). That is a recorded decision, not an omission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionARow {
    /// **(a)** *"Name and address of the donee organization"*.
    pub col_a_donee_name_and_address: String,
    /// **(c)** *"Description and condition of donated property (For a vehicle, enter the year, make,
    /// model, and mileage. For securities and other property, see instructions.)"*
    pub col_c_description_and_condition: String,
    /// **(d)** *"Date of the contribution"*.
    pub col_d_date_of_contribution: TaxDate,
    /// **(e)/(f)/(g)** — completed, or blank by the per-item carve-out. See [`SectionAColumnsEfg`].
    pub cols_efg: SectionAColumnsEfg,
    /// **(h)** *"Fair market value (see instructions)"*.
    pub col_h_fair_market_value: Usd,
    /// **(i)** *"Method used to determine the fair market value"*.
    pub col_i_method_used_to_determine_fmv: String,
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
//  The refusals
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// **Why a non-crypto noncash gift still cannot be filed** — one variant per condition, so no refusal
/// is anonymous. Carried by [`crate::tax::return_refuse::RefuseReason::NonCryptoNoncashGift`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoncashGiftRefusal {
    /// No property details were collected for this gift at all — the original FR-200(b) defect's
    /// state. btctax holds an amount and a §170(b) class and nothing the form's columns ask for, so a
    /// Form 8283 it attached would UNDER-REPORT its own property list.
    DetailsNotCollected,
    /// Column **(a)** is empty. Regulations section 1.170A-16(c)(3)(v) requires Section A to be fully
    /// completed.
    DoneeNameAndAddressEmpty,
    /// Column **(c)** is empty.
    DescriptionAndConditionEmpty,
    /// Column **(i)** is empty.
    FmvMethodEmpty,
    /// The claimed deduction is over `$500`, so columns (e), (f) and (g) must be completed — and at
    /// least one of them was never collected. i8283 column (g): *"For items over $500, enter your cost
    /// or adjusted basis."*
    ///
    /// ★ The form does offer an out — *"If you must complete columns (e), (f), and (g) but have
    /// reasonable cause for not providing the information required, attach an explanation"*
    /// (`i8283--2024.txt:932-934`) — but btctax produces no such attachment, and a *"reasonable
    /// cause"* determination is the filer's, not software's. So it refuses and says what is missing.
    ColumnsEfgRequiredButNotCollected,
    /// Column (h)'s FMV is **larger** than the deduction claimed for the gift, so the FMV was reduced
    /// to figure the claim. i8283 column (h): *"You must attach a statement if you were required to
    /// reduce the FMV to figure the amount of your deduction."* btctax attaches no such statement —
    /// and §170(e)(1) reductions are exactly where a noncash claim goes wrong.
    FmvReducedNeedsAttachedStatement,
    /// Column (h)'s FMV is **smaller** than the deduction claimed for the gift — a claim larger than
    /// what the property was worth, which §170(a) does not allow in either direction the form can
    /// express. The §170(b) percentage ceilings only ever reduce a claim, and they apply to the return's
    /// total rather than to one gift, so they cannot explain it either. A corrupt or mistyped input.
    ClaimExceedsFairMarketValue,
    /// A publicly traded security. Columns (c), (e) and (g) each ask a security something btctax never
    /// collected — see [`NoncashPropertyKind::PubliclyTradedSecurity`], which carries the three
    /// instruction passages.
    PubliclyTradedSecurity,
    /// A vehicle. §170(f)(12) territory: Form 1098-C or another contemporaneous written
    /// acknowledgment must be attached, column (b)'s box and VIN cells are not in btctax's field map,
    /// and the deduction is generally limited to the donee's gross sale proceeds.
    Vehicle,
    /// Intellectual property (§170(e)(1)(B)(iii)) — the donee reports its net income from the property
    /// on Form 8899 and the deduction accretes over later years. btctax models neither.
    IntellectualProperty,
    /// Inventory or property held primarily for sale to customers (§1221(a)(1)) — the deduction is
    /// basis-limited under §170(e)(1)(A) and interacts with cost of goods sold.
    InventoryOrHeldForSale,
    /// The claimed deduction is over `$5,000` and the property is not one of the classes Section A
    /// carries above that line, so it belongs in **Section B** — which requires a written qualified
    /// appraisal by a qualified appraiser (§170(f)(11)(D), §6695A). btctax holds no appraiser identity
    /// for a gift that did not come from the ledger.
    OverFiveThousandNeedsSectionB,
    /// A single article of clothing or a household item **not in good used condition or better**, with
    /// a claimed deduction over `$500`. i8283: *"You must include with your return a qualified
    /// appraisal of any single item of clothing or any household item that is not in good used
    /// condition or better for which you are claiming a deduction of more than $500. Attach the
    /// appraisal and Section B to your return."*
    NotGoodUsedConditionNeedsSectionBAppraisal,
    /// A clothing or household item **not in good used condition or better** with a claimed deduction
    /// of `$500` or less — for which §170(f)(16)(A) allows **no deduction at all**. The over-$500
    /// qualified-appraisal exception is the only route to deductibility, so this is not a
    /// missing-attachment problem: the amount on Schedule A line 12 is not allowable.
    NotGoodUsedConditionNotDeductible,
}

impl NoncashGiftRefusal {
    /// The filer-facing sentence, naming the condition and the exit. One per variant — the `match` is
    /// `_`-free, so a new condition cannot ship without one.
    #[must_use]
    pub fn detail(self) -> &'static str {
        match self {
            Self::DetailsNotCollected => {
                "a non-crypto NONCASH charitable gift pushes total noncash gifts over $500, which \
                 requires a Form 8283 listing ALL of the contributed property — and no property \
                 details were recorded for this gift. Add the Form 8283 Section A details to the \
                 gift (`schedule_a.charitable[].noncash` in your `income import` file: the donee's \
                 name and address, a description and condition, the contribution date, the fair \
                 market value and how you determined it), or remove the gift"
            }
            Self::DoneeNameAndAddressEmpty => {
                "a noncash gift's Form 8283 Section A column (a) \"Name and address of the donee \
                 organization\" is empty. Regulations section 1.170A-16(c)(3)(v) requires Section A \
                 to be fully completed; set `noncash.donee_name_and_address`"
            }
            Self::DescriptionAndConditionEmpty => {
                "a noncash gift's Form 8283 Section A column (c) \"Description and condition of \
                 donated property\" is empty. Describe the property in sufficient detail (i8283: \
                 \"The greater the value of the property, the more detail you must provide\"); set \
                 `noncash.description_and_condition`"
            }
            Self::FmvMethodEmpty => {
                "a noncash gift's Form 8283 Section A column (i) \"Method used to determine the fair \
                 market value\" is empty. i8283 gives the examples — \"Appraisal\", \"Thrift shop \
                 value\" (for clothing or household items), \"Catalog\", \"Comparable sales\" — and \
                 forbids leaving it open: \"You may not indicate that the information is available \
                 upon request.\" Set `noncash.method_used_to_determine_fmv`"
            }
            Self::ColumnsEfgRequiredButNotCollected => {
                "a noncash gift claims a deduction of MORE than $500, so Form 8283 Section A columns \
                 (e) date acquired, (f) how acquired and (g) donor's cost or adjusted basis must all \
                 be completed — and at least one is missing. Set `noncash.date_acquired_by_donor` \
                 (a date, or \"various\" for a group of similar items all held over 12 months), \
                 `noncash.how_acquired_by_donor` and `noncash.cost_or_adjusted_basis`. At $500 or \
                 less the form does not ask for these three at all"
            }
            Self::FmvReducedNeedsAttachedStatement => {
                "a noncash gift's Form 8283 column (h) fair market value is LARGER than the deduction \
                 claimed for it, so the FMV was reduced to figure the claim. i8283 column (h): \"You \
                 must attach a statement if you were required to reduce the FMV to figure the amount \
                 of your deduction\" — btctax attaches no such statement. Either claim the full fair \
                 market value, or complete Form 8283 by hand"
            }
            Self::ClaimExceedsFairMarketValue => {
                "a noncash gift claims MORE than the Form 8283 column (h) fair market value you \
                 entered for the same property. §170(a) allows no more than the value contributed, \
                 and the §170(b) percentage ceilings only ever reduce a claim — and they apply to the \
                 return's total, not to one gift. Check `amount` and `noncash.fair_market_value`: one \
                 of the two is a typo"
            }
            Self::PubliclyTradedSecurity => {
                "a donated PUBLICLY TRADED SECURITY needs three things on Form 8283 Section A that \
                 btctax does not collect: column (c) wants the company name, number of shares, kind \
                 of security, whether it is a mutual fund share and whether it is regularly traded; \
                 column (e) is entered \"only if you held the securities for more than 12 months\"; \
                 and column (g) says \"Do not complete this column for publicly traded securities \
                 held more than 12 months, unless you elect to limit your deduction cost basis\" \
                 (§170(b)(1)(C)(iii)) — an election btctax does not model. Complete Form 8283 by \
                 hand, or remove the gift"
            }
            Self::Vehicle => {
                "a donated VEHICLE (car, boat or airplane) is Form 1098-C territory: §170(f)(12) \
                 requires a contemporaneous written acknowledgment attached to the return, generally \
                 limits the deduction to the donee's gross proceeds from selling it, and Form 8283 \
                 Section A column (b) wants the box checked and the VIN — none of which btctax \
                 produces. Complete Form 8283 by hand, or remove the gift"
            }
            Self::IntellectualProperty => {
                "donated INTELLECTUAL PROPERTY (§170(e)(1)(B)(iii)) is deducted over later years as \
                 the donee earns income from it, reported to you on Form 8899 — btctax models \
                 neither. Complete Form 8283 by hand, or remove the gift"
            }
            Self::InventoryOrHeldForSale => {
                "donated INVENTORY, or property held primarily for sale to customers (§1221(a)(1)), \
                 is basis-limited under §170(e)(1)(A) and interacts with cost of goods sold — btctax \
                 models neither. Complete Form 8283 by hand, or remove the gift"
            }
            Self::OverFiveThousandNeedsSectionB => {
                "a noncash gift claims a deduction of MORE than $5,000 for one item or group of \
                 similar items, which Form 8283 reports in SECTION B — and Section B requires a \
                 written qualified appraisal by a qualified appraiser (§170(f)(11)(D), §6695A) whose \
                 name, address and TIN btctax does not hold for property that did not come from your \
                 ledger. Complete Form 8283 Section B by hand, or remove the gift."
            }
            Self::NotGoodUsedConditionNeedsSectionBAppraisal => {
                "a single article of clothing or a household item that is NOT in good used condition \
                 or better, with a claimed deduction over $500, is deductible only with a qualified \
                 appraisal attached to the return along with Form 8283 SECTION B (§170(f)(16)(A); \
                 i8283 \"Clothing and household items not in good used condition\") — btctax holds \
                 no appraisal. Complete Form 8283 Section B by hand, or remove the gift"
            }
            Self::NotGoodUsedConditionNotDeductible => {
                "a clothing or household item that is NOT in good used condition or better is not \
                 deductible at all unless the claimed value is MORE than $500 and a qualified \
                 appraisal is attached (§170(f)(16)(A); Pub. 561: \"You cannot take an income tax \
                 charitable contribution deduction for an item of clothing unless it is in good used \
                 condition or better\"). This gift claims $500 or less, so the amount is not \
                 allowable on Schedule A line 12 — remove the gift, or answer that the item IS in \
                 good used condition or better"
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
//  The single decision
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// **Must columns (e), (f) and (g) be completed for an item claiming `claimed`?**
///
/// *"Note: If the amount you claimed as a deduction for an item is $500 or less, you do not have to
/// complete columns (e), (f), and (g)."* — so the answer is `claimed > $500`, strict, measured over
/// **one item or group of similar items** and never over the year's total.
#[must_use]
pub fn columns_efg_required(claimed: Usd) -> bool {
    claimed > COLUMNS_EFG_PER_ITEM_THRESHOLD
}

/// **§170(f)(16)(A) ALLOWABILITY — is a clothing or household item deductible at all?**
///
/// ★★★ Its own function because it is read **twice, for different reasons**, and the two must never
/// disagree: [`section_a_row`] asks it while building a row, and [`screen_noncash_gifts`] asks it on a
/// year where Form 8283 is **not** filed at all. Allowability does not depend on whether a form is
/// attached — a $50 shirt in poor condition is non-deductible with or without a Form 8283 — so the
/// under-threshold year must still be able to ask. One definition, two readers.
///
/// `None` for every class but clothing-or-household-in-poor-condition, which is the only class
/// §170(f)(16)(A) disallows.
#[must_use]
pub fn clothing_condition_refusal(
    claimed: Usd,
    kind: NoncashPropertyKind,
) -> Option<NoncashGiftRefusal> {
    match kind {
        NoncashPropertyKind::ClothingOrHouseholdItem {
            good_used_condition_or_better: false,
        } => Some(if claimed > COLUMNS_EFG_PER_ITEM_THRESHOLD {
            // Deductible, but only on a Section B with an attached qualified appraisal.
            NoncashGiftRefusal::NotGoodUsedConditionNeedsSectionBAppraisal
        } else {
            // §170(f)(16)(A) allows no deduction at all.
            NoncashGiftRefusal::NotGoodUsedConditionNotDeductible
        }),
        NoncashPropertyKind::ClothingOrHouseholdItem {
            good_used_condition_or_better: true,
        }
        | NoncashPropertyKind::PubliclyTradedSecurity
        | NoncashPropertyKind::Vehicle
        | NoncashPropertyKind::IntellectualProperty
        | NoncashPropertyKind::InventoryOrHeldForSale
        | NoncashPropertyKind::OtherProperty => None,
    }
}

/// **Whether Form 8283 is filed for the year** — which decides *how much* of the form's completeness
/// this module may demand. See [`screen_noncash_gifts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoncashGate {
    /// The year's TOTAL noncash deduction is over `$500`, so one or more Forms 8283 are attached and
    /// every column the form asks for must be answerable.
    FormIsFiled,
    /// No Form 8283 is attached. Only §170(f)(16)(A) **allowability** is checked — demanding the
    /// completeness of a form that will not be filed would be a refusal with no purpose, the
    /// `is_sstb`-below-the-threshold shape.
    AllowabilityOnly,
}

/// **The ONE walk over a return's non-crypto charitable gifts** — used by the refusal screen and by the
/// packet, so the decision to refuse and the decision about what to print cannot diverge.
///
/// Returns every admitted Section A row in input order, or the **first** refusal with the index of the
/// gift that caused it. Cash gifts are skipped ([`crate::tax::return_inputs::CharitableClass::is_cash`]
/// — Form 8283 says nothing about them).
///
/// ★ On [`NoncashGate::AllowabilityOnly`] the returned row list is empty by construction: no form is
/// filed, so there is nothing to print, and only the allowability refusal can fire.
pub fn screen_noncash_gifts(
    gifts: &[crate::tax::return_inputs::CharitableGift],
    gate: NoncashGate,
) -> Result<Vec<SectionARow>, (usize, NoncashGiftRefusal)> {
    let mut rows = Vec::new();
    for (i, g) in gifts.iter().enumerate() {
        if g.class.is_cash() {
            continue;
        }
        match (gate, g.noncash.as_ref()) {
            // Details present: allowability is checked on both gates; the full column set only when a
            // form is actually filed.
            (NoncashGate::AllowabilityOnly, Some(p)) => {
                if let Some(r) = clothing_condition_refusal(g.amount, p.kind) {
                    return Err((i, r));
                }
            }
            (NoncashGate::FormIsFiled, Some(p)) => {
                rows.push(section_a_row(g.amount, Some(p)).map_err(|r| (i, r))?);
            }
            // No details, and a form IS filed ⇒ the FR-200(b) defect's own state: btctax would attach
            // a Form 8283 that under-reports its own property list.
            (NoncashGate::FormIsFiled, None) => {
                return Err((i, NoncashGiftRefusal::DetailsNotCollected))
            }
            // No details and no form: nothing asks, nothing is answered, nothing refuses.
            (NoncashGate::AllowabilityOnly, None) => {}
        }
    }
    Ok(rows)
}

/// **The ONE place a non-crypto noncash gift becomes a Section A row, or a named refusal.**
///
/// `claimed` is the deduction claimed for this item or group of similar items — the
/// [`crate::tax::return_inputs::CharitableGift::amount`] that reaches Schedule A line 12. `details`
/// is what the filer was asked; `None` is the FR-200(b) defect's state, and it refuses.
///
/// The checks run in the order the form asks them: *which property is this* (does Section A carry it at
/// all), then *how much* (which section, and does the carve-out apply), then *is every required column
/// there*. Nothing falls through — the `match` on [`NoncashPropertyKind`] is `_`-free, so a new
/// property class cannot ship without a decision.
pub fn section_a_row(
    claimed: Usd,
    details: Option<&NoncashGiftProperty>,
) -> Result<SectionARow, NoncashGiftRefusal> {
    let d = details.ok_or(NoncashGiftRefusal::DetailsNotCollected)?;

    // 1a. §170(f)(16)(A) allowability, through the SHARED predicate — the same one the
    //     under-threshold year reads. See [`clothing_condition_refusal`].
    if let Some(r) = clothing_condition_refusal(claimed, d.kind) {
        return Err(r);
    }

    // 1b. The property class. Section A carries only the classes i8283 lists; every other class names
    //     its own refusal rather than being quietly admitted.
    match d.kind {
        NoncashPropertyKind::Vehicle => return Err(NoncashGiftRefusal::Vehicle),
        NoncashPropertyKind::IntellectualProperty => {
            return Err(NoncashGiftRefusal::IntellectualProperty)
        }
        NoncashPropertyKind::InventoryOrHeldForSale => {
            return Err(NoncashGiftRefusal::InventoryOrHeldForSale)
        }
        NoncashPropertyKind::PubliclyTradedSecurity => {
            return Err(NoncashGiftRefusal::PubliclyTradedSecurity)
        }
        // Clothing in poor condition already returned at 1a; a `true` here reaches Section A.
        NoncashPropertyKind::ClothingOrHouseholdItem { .. }
        | NoncashPropertyKind::OtherProperty => {}
    }

    // 2. Section A or Section B. Every class Section A carries above $5,000 — publicly traded
    //    securities, a qualified vehicle, intellectual property, inventory — has already refused above
    //    for its own reason, so what reaches here is subject to the plain rule with no exception.
    if claimed > SECTION_B_PER_ITEM_THRESHOLD {
        return Err(NoncashGiftRefusal::OverFiveThousandNeedsSectionB);
    }

    // 3. Column (h) is the FMV, and it has to agree with the claim — in BOTH directions, for two
    //    different reasons. Larger ⇒ the FMV was reduced and i8283 wants a statement btctax cannot
    //    attach; smaller ⇒ the claim exceeds what the property was worth, which is a corrupt input.
    //    ★ Two named refusals rather than one `!=`, because a filer fixing the first edits a different
    //      field from a filer fixing the second.
    if d.fair_market_value > claimed {
        return Err(NoncashGiftRefusal::FmvReducedNeedsAttachedStatement);
    }
    if d.fair_market_value < claimed {
        return Err(NoncashGiftRefusal::ClaimExceedsFairMarketValue);
    }

    // 4. The always-required text columns.
    if d.donee_name_and_address.trim().is_empty() {
        return Err(NoncashGiftRefusal::DoneeNameAndAddressEmpty);
    }
    if d.description_and_condition.trim().is_empty() {
        return Err(NoncashGiftRefusal::DescriptionAndConditionEmpty);
    }
    if d.method_used_to_determine_fmv.trim().is_empty() {
        return Err(NoncashGiftRefusal::FmvMethodEmpty);
    }

    // 5. Columns (e)(f)(g) — required, or blank with the form's own reason recorded.
    let cols_efg = if columns_efg_required(claimed) {
        match (
            d.date_acquired_by_donor,
            d.how_acquired_by_donor.as_deref().map(str::trim),
            d.cost_or_adjusted_basis,
        ) {
            (Some(e), Some(f), Some(g)) if !f.is_empty() => SectionAColumnsEfg::Completed {
                col_e_date_acquired_by_donor: e,
                col_f_how_acquired_by_donor: f.to_string(),
                col_g_cost_or_adjusted_basis: g,
            },
            _ => return Err(NoncashGiftRefusal::ColumnsEfgRequiredButNotCollected),
        }
    } else {
        SectionAColumnsEfg::NotRequiredDeductionAtOrUnderFiveHundred
    };

    Ok(SectionARow {
        col_a_donee_name_and_address: d.donee_name_and_address.clone(),
        col_c_description_and_condition: d.description_and_condition.clone(),
        col_d_date_of_contribution: d.date_of_contribution,
        cols_efg,
        col_h_fair_market_value: d.fair_market_value,
        col_i_method_used_to_determine_fmv: d.method_used_to_determine_fmv.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::{CharitableClass, CharitableGift};
    use time::macros::date;

    /// **The brief's own scenario, as i8283 describes it.** A $600 bag of used clothing in good used
    /// condition, given to a thrift store, valued at thrift-shop value.
    ///
    /// Every expected value is quoted from a document, never computed by this implementation:
    /// * `$600 > $500` ⇒ columns (e)(f)(g) REQUIRED — Form 8283's own note (`f8283--2024.txt:39`) and
    ///   i8283 column (g) (`i8283--2024.txt:926`): *"For items over $500, enter your cost or adjusted
    ///   basis."*
    /// * column (i) = `Thrift shop value` — i8283's own example for this exact property class
    ///   (`i8283--2024.txt:946-947`), and Pub. 561 (`:310-318`): *"The price that buyers of used items
    ///   actually pay in used clothing stores, such as consignment or thrift shops, is an indication of
    ///   the value."*
    /// * `$600 ≤ $5,000` ⇒ **Section A**, not B (`i8283--2024.txt:208-211`).
    fn the_six_hundred_dollar_bag() -> NoncashGiftProperty {
        NoncashGiftProperty {
            kind: NoncashPropertyKind::ClothingOrHouseholdItem {
                good_used_condition_or_better: true,
            },
            donee_name_and_address: "Goodwill Industries, 1 Main St, Springfield IL 62701".into(),
            description_and_condition: "Bag of used adult clothing (12 shirts, 4 pairs of \
                                        trousers), good used condition"
                .into(),
            date_of_contribution: date!(2024 - 11 - 30),
            date_acquired_by_donor: Some(DateAcquiredByDonor::Various),
            how_acquired_by_donor: Some("Purchase".into()),
            cost_or_adjusted_basis: Some(dec!(2400)),
            fair_market_value: dec!(600),
            method_used_to_determine_fmv: "Thrift shop value".into(),
        }
    }

    fn gift(amount: Usd, p: Option<NoncashGiftProperty>) -> CharitableGift {
        CharitableGift {
            class: CharitableClass::CapGainProp30,
            amount,
            noncash: p,
        }
    }

    // ══════════════════════════════════════════════════════════════════════════════════════════════
    //  THE TWO $500 THRESHOLDS — the pair of KATs that hold them apart
    // ══════════════════════════════════════════════════════════════════════════════════════════════

    /// ★★★ **THE DEFECT, CLOSED.** The bag files a Section A row, with columns (e)(f)(g) completed
    /// because `$600` is over the PER-ITEM line.
    #[test]
    fn the_six_hundred_dollar_bag_of_clothes_files_section_a_with_columns_efg() {
        let row = section_a_row(dec!(600), Some(&the_six_hundred_dollar_bag()))
            .expect("a $600 bag of clothes in good used condition is a Section A row");
        assert_eq!(
            row.col_i_method_used_to_determine_fmv, "Thrift shop value",
            "i8283 column (i) names this exact entry for clothing and household items"
        );
        assert_eq!(row.col_h_fair_market_value, dec!(600));
        assert_eq!(
            row.cols_efg,
            SectionAColumnsEfg::Completed {
                col_e_date_acquired_by_donor: DateAcquiredByDonor::Various,
                col_f_how_acquired_by_donor: "Purchase".into(),
                col_g_cost_or_adjusted_basis: dec!(2400),
            },
            "$600 is MORE than $500, so the form's carve-out does not apply and all three columns \
             must be completed: \"For items over $500, enter your cost or adjusted basis.\""
        );
    }

    /// ★★★ **THE OTHER $500.** Four $200 bags total $800, so a Form 8283 IS filed — and not one of
    /// them needs columns (e), (f) or (g), because the carve-out is measured **per item**.
    ///
    /// This is the vector that distinguishes the two constants. On today's form both read `$500`, so a
    /// test exercising only the per-item side (or only the filing side) cannot tell them apart.
    #[test]
    fn four_two_hundred_dollar_bags_file_the_form_and_omit_columns_efg() {
        let bag = || NoncashGiftProperty {
            // Nothing collected for (e), (f) or (g) — and that is CORRECT for a $200 item, not a gap.
            date_acquired_by_donor: None,
            how_acquired_by_donor: None,
            cost_or_adjusted_basis: None,
            fair_market_value: dec!(200),
            ..the_six_hundred_dollar_bag()
        };
        let gifts: Vec<CharitableGift> = (0..4).map(|_| gift(dec!(200), Some(bag()))).collect();

        let total: Usd = gifts.iter().map(|g| g.amount).sum();
        assert_eq!(
            total,
            dec!(800),
            "premise: the four bags total $800, over the FILING threshold"
        );
        assert!(
            total > crate::tax::printed::FORM_8283_THRESHOLD,
            "the form's own header: \"Attach one or more Forms 8283 to your tax return if you claimed \
             a total deduction of over $500 for all contributed property.\""
        );

        let rows = screen_noncash_gifts(&gifts, NoncashGate::FormIsFiled)
            .expect("four $200 bags are four Section A rows");
        assert_eq!(rows.len(), 4, "four gifts, four rows — none dropped");
        for (i, r) in rows.iter().enumerate() {
            assert_eq!(
                r.cols_efg,
                SectionAColumnsEfg::NotRequiredDeductionAtOrUnderFiveHundred,
                "row {i}: $200 is $500 or less, so the form's note applies — and the BLANK carries \
                 its reason in the type rather than looking like a gap"
            );
        }
    }

    /// ★★ **The two thresholds are read by two different functions over two different quantities.**
    ///
    /// A structural statement, not a value comparison: the per-item decision is a pure function of ONE
    /// item's claim, so no total can move it.
    #[test]
    fn the_per_item_carve_out_cannot_be_moved_by_the_years_total() {
        assert!(!columns_efg_required(dec!(400)), "$400 is not over $500");
        assert!(!columns_efg_required(dec!(500)), "exactly $500 is NOT over");
        assert!(columns_efg_required(dec!(500.01)), "strictly greater");
        assert!(columns_efg_required(dec!(600)));
        // And the filing threshold is a different number read over a different quantity.
        assert_eq!(COLUMNS_EFG_PER_ITEM_THRESHOLD, dec!(500));
        assert_eq!(crate::tax::printed::FORM_8283_THRESHOLD, dec!(500));
        assert_eq!(
            SECTION_B_PER_ITEM_THRESHOLD,
            dec!(5000),
            "§170(f)(11)(C) \"more than $5,000\""
        );
    }

    /// The $5,000 boundary is strict `>`: Section A's heading says *"$5,000 or less"*.
    #[test]
    fn exactly_five_thousand_is_section_a_and_a_cent_more_is_section_b() {
        let p = |v: Usd| NoncashGiftProperty {
            fair_market_value: v,
            ..the_six_hundred_dollar_bag()
        };
        assert!(
            section_a_row(dec!(5000), Some(&p(dec!(5000)))).is_ok(),
            "\"an item (or a group of similar items) for which you claimed a deduction of $5,000 or \
             less\""
        );
        assert_eq!(
            section_a_row(dec!(5000.01), Some(&p(dec!(5000.01)))),
            Err(NoncashGiftRefusal::OverFiveThousandNeedsSectionB)
        );
    }

    // ══════════════════════════════════════════════════════════════════════════════════════════════
    //  WHAT STILL REFUSES — one vector per condition, each naming its own
    // ══════════════════════════════════════════════════════════════════════════════════════════════

    /// ★★★ **§170(f)(16)(A) — the clothing-condition answer routes THREE ways, and only one files.**
    ///
    /// i8283: *"Generally, you cannot claim a deduction for clothing or household items you donate
    /// unless the clothing or household items are in good used condition or better."* … *"However, you
    /// can claim a deduction for a contribution of an item of clothing or a household item that is not
    /// in good used condition or better if your claimed value is more than $500 and you substantiate
    /// that value with a qualified appraisal and Form 8283, Section B."*
    #[test]
    fn clothing_not_in_good_used_condition_routes_three_ways() {
        let poor = |v: Usd| NoncashGiftProperty {
            kind: NoncashPropertyKind::ClothingOrHouseholdItem {
                good_used_condition_or_better: false,
            },
            fair_market_value: v,
            ..the_six_hundred_dollar_bag()
        };
        // (1) good used condition ⇒ files (the test above).
        // (2) NOT good used condition, over $500 ⇒ Section B + an attached qualified appraisal.
        assert_eq!(
            section_a_row(dec!(600), Some(&poor(dec!(600)))),
            Err(NoncashGiftRefusal::NotGoodUsedConditionNeedsSectionBAppraisal)
        );
        // (3) NOT good used condition, $500 or less ⇒ NO DEDUCTION AT ALL. The over-$500
        //     qualified-appraisal exception is the only route to deductibility.
        assert_eq!(
            section_a_row(dec!(50), Some(&poor(dec!(50)))),
            Err(NoncashGiftRefusal::NotGoodUsedConditionNotDeductible)
        );
    }

    /// ★★ …and (3) refuses even on a year that files NO Form 8283, because allowability is not a
    /// question about an attachment. `$50` of noncash is nowhere near the $500 filing threshold, and the
    /// deduction is still not allowable.
    #[test]
    fn a_poor_condition_item_under_the_filing_threshold_still_refuses() {
        let poor = NoncashGiftProperty {
            kind: NoncashPropertyKind::ClothingOrHouseholdItem {
                good_used_condition_or_better: false,
            },
            fair_market_value: dec!(50),
            ..the_six_hundred_dollar_bag()
        };
        assert_eq!(
            screen_noncash_gifts(&[gift(dec!(50), Some(poor))], NoncashGate::AllowabilityOnly),
            Err((0, NoncashGiftRefusal::NotGoodUsedConditionNotDeductible)),
            "§170(f)(16)(A) disallows the deduction whether or not a Form 8283 is attached"
        );
    }

    /// …while a year that files no Form 8283 is NOT interrogated about the form's completeness. A $300
    /// gift with an empty column (i) would refuse on a filing year and must not on this one — that is
    /// the `is_sstb`-below-the-threshold shape: a refusal with no purpose.
    #[test]
    fn below_the_filing_threshold_the_forms_own_completeness_is_not_demanded() {
        let bare = NoncashGiftProperty {
            method_used_to_determine_fmv: String::new(),
            fair_market_value: dec!(300),
            ..the_six_hundred_dollar_bag()
        };
        let gifts = [gift(dec!(300), Some(bare))];
        assert_eq!(
            screen_noncash_gifts(&gifts, NoncashGate::AllowabilityOnly),
            Ok(Vec::new()),
            "no form is filed, so no column of it is demanded — and no row is produced either"
        );
        assert_eq!(
            screen_noncash_gifts(&gifts, NoncashGate::FormIsFiled),
            Err((0, NoncashGiftRefusal::FmvMethodEmpty)),
            "…and on a year that DOES file, the same gift refuses for the empty column (i)"
        );
    }

    /// ★★★ **EVERY property class Section A cannot carry refuses, and NAMES ITSELF** — the expected
    /// verdict comes from an `_`-free `match` over the kind enum, so a new class is a COMPILE ERROR
    /// here rather than a silently untested one.
    #[test]
    fn every_refusing_property_class_names_its_own_condition() {
        for kind in [
            NoncashPropertyKind::Vehicle,
            NoncashPropertyKind::IntellectualProperty,
            NoncashPropertyKind::InventoryOrHeldForSale,
            NoncashPropertyKind::PubliclyTradedSecurity,
            NoncashPropertyKind::OtherProperty,
            NoncashPropertyKind::ClothingOrHouseholdItem {
                good_used_condition_or_better: true,
            },
            NoncashPropertyKind::ClothingOrHouseholdItem {
                good_used_condition_or_better: false,
            },
        ] {
            let expected: Result<(), NoncashGiftRefusal> = match kind {
                NoncashPropertyKind::Vehicle => Err(NoncashGiftRefusal::Vehicle),
                NoncashPropertyKind::IntellectualProperty => {
                    Err(NoncashGiftRefusal::IntellectualProperty)
                }
                NoncashPropertyKind::InventoryOrHeldForSale => {
                    Err(NoncashGiftRefusal::InventoryOrHeldForSale)
                }
                NoncashPropertyKind::PubliclyTradedSecurity => {
                    Err(NoncashGiftRefusal::PubliclyTradedSecurity)
                }
                NoncashPropertyKind::ClothingOrHouseholdItem {
                    good_used_condition_or_better: false,
                } => Err(NoncashGiftRefusal::NotGoodUsedConditionNeedsSectionBAppraisal),
                NoncashPropertyKind::ClothingOrHouseholdItem {
                    good_used_condition_or_better: true,
                }
                | NoncashPropertyKind::OtherProperty => Ok(()),
            };
            let got = section_a_row(
                dec!(600),
                Some(&NoncashGiftProperty {
                    kind,
                    ..the_six_hundred_dollar_bag()
                }),
            );
            match (expected, got) {
                (Ok(()), Ok(_)) => {}
                (Err(want), Err(g)) => assert_eq!(want, g, "{kind:?}"),
                (want, g) => panic!("{kind:?}: expected {want:?}, got {g:?}"),
            }
        }
    }

    /// **The FR-200(b) defect's own input** — an amount, a §170 class, and nothing the form's columns
    /// ask for. It still refuses, and now says exactly what to add.
    #[test]
    fn a_gift_with_no_property_details_still_refuses_and_names_the_reason() {
        assert_eq!(
            section_a_row(dec!(600), None),
            Err(NoncashGiftRefusal::DetailsNotCollected)
        );
        assert!(
            NoncashGiftRefusal::DetailsNotCollected
                .detail()
                .contains("schedule_a.charitable[].noncash"),
            "the message has to name the key the filer sets, or it is not actionable"
        );
    }

    /// Over $500 with (e), (f) or (g) missing refuses — one vector per column, so no single missing
    /// column can hide behind the other two.
    #[test]
    fn over_five_hundred_each_missing_efg_column_refuses_on_its_own() {
        let base = the_six_hundred_dollar_bag();
        let mutations: [fn(&mut NoncashGiftProperty); 4] = [
            |p| p.date_acquired_by_donor = None,
            |p| p.how_acquired_by_donor = None,
            // ★ An empty-string (f) is a blank column, not an answer — the same trim rule the rest of
            //   this file uses. Without the `!f.is_empty()` guard this vector passes.
            |p| p.how_acquired_by_donor = Some("   ".into()),
            |p| p.cost_or_adjusted_basis = None,
        ];
        for mutate in mutations {
            let mut p = base.clone();
            mutate(&mut p);
            assert_eq!(
                section_a_row(dec!(600), Some(&p)),
                Err(NoncashGiftRefusal::ColumnsEfgRequiredButNotCollected),
                "every one of (e), (f) and (g) is required over $500"
            );
        }
    }

    /// Each always-required TEXT column refuses on its own when blank — and whitespace is blank.
    /// Regulations section 1.170A-16(c)(3)(v) requires Section A to be fully completed.
    #[test]
    fn each_required_text_column_refuses_when_blank() {
        /// One planted blank, and the refusal it must produce.
        type BlankCase = (fn(&mut NoncashGiftProperty), NoncashGiftRefusal);
        let cases: [BlankCase; 3] = [
            (
                |p| p.donee_name_and_address = "  ".into(),
                NoncashGiftRefusal::DoneeNameAndAddressEmpty,
            ),
            (
                |p| p.description_and_condition = String::new(),
                NoncashGiftRefusal::DescriptionAndConditionEmpty,
            ),
            (
                |p| p.method_used_to_determine_fmv = "\t".into(),
                NoncashGiftRefusal::FmvMethodEmpty,
            ),
        ];
        for (mutate, want) in cases {
            let mut p = the_six_hundred_dollar_bag();
            mutate(&mut p);
            assert_eq!(section_a_row(dec!(600), Some(&p)), Err(want));
        }
    }

    /// Column (h) must equal the claim, and the two directions are different defects.
    #[test]
    fn column_h_disagreeing_with_the_claim_refuses_in_both_directions() {
        let p = |fmv: Usd| NoncashGiftProperty {
            fair_market_value: fmv,
            ..the_six_hundred_dollar_bag()
        };
        assert_eq!(
            section_a_row(dec!(600), Some(&p(dec!(900)))),
            Err(NoncashGiftRefusal::FmvReducedNeedsAttachedStatement),
            "FMV reduced to figure the claim ⇒ i8283 wants an attached statement"
        );
        assert_eq!(
            section_a_row(dec!(600), Some(&p(dec!(400)))),
            Err(NoncashGiftRefusal::ClaimExceedsFairMarketValue),
            "a claim larger than the value contributed"
        );
    }

    /// A CASH gift is skipped entirely — Form 8283 says nothing about cash, so a cash gift with no
    /// property block never refuses, however large.
    #[test]
    fn a_cash_gift_is_never_asked_for_form_8283_details() {
        let gifts = [CharitableGift {
            class: CharitableClass::Cash60,
            amount: dec!(50_000),
            noncash: None,
        }];
        assert_eq!(
            screen_noncash_gifts(&gifts, NoncashGate::FormIsFiled),
            Ok(Vec::new()),
            "i8283 Purpose of Form: \"do not use Form 8283 to report … amounts you gave by check or \
             credit card\""
        );
        assert!(
            CharitableClass::Cash60.is_cash() && CharitableClass::Cash30.is_cash(),
            "and the predicate is the one an `_`-free match holds"
        );
    }

    /// Every non-cash §170(b) class IS asked. Derived from the `_`-free `match` in `is_cash`, so a
    /// seventh class must decide rather than inherit.
    #[test]
    fn every_noncash_class_is_asked_for_form_8283_details() {
        for class in [
            CharitableClass::CapGainProp30,
            CharitableClass::CapGainProp20,
            CharitableClass::OrdinaryProp50,
            CharitableClass::OrdinaryProp30,
        ] {
            assert!(!class.is_cash(), "{class:?}");
            assert_eq!(
                screen_noncash_gifts(
                    &[CharitableGift {
                        class,
                        amount: dec!(600),
                        noncash: None,
                    }],
                    NoncashGate::FormIsFiled
                ),
                Err((0, NoncashGiftRefusal::DetailsNotCollected)),
                "{class:?} is noncash, so the form's property list must account for it"
            );
        }
    }

    /// The walk reports the INDEX of the offending gift, so a filer with several knows which to fix.
    #[test]
    fn the_refusal_names_which_gift_it_is() {
        let gifts = [
            gift(dec!(600), Some(the_six_hundred_dollar_bag())),
            gift(dec!(600), Some(the_six_hundred_dollar_bag())),
            gift(dec!(600), None),
        ];
        assert_eq!(
            screen_noncash_gifts(&gifts, NoncashGate::FormIsFiled),
            Err((2, NoncashGiftRefusal::DetailsNotCollected))
        );
    }

    /// ★★★ **B1 — the FIFTH GIFT MUST NOT DISAPPEAR**, at the row-building layer. Five admitted gifts
    /// produce five rows; the four-row page limit belongs to the emitter, and this is the half that
    /// guarantees it is handed all five. (`btctax-forms/tests/f8283_section_a.rs` holds the other
    /// half: that all five reach paper.)
    #[test]
    fn five_gifts_produce_five_rows_and_none_is_dropped() {
        let gifts: Vec<CharitableGift> = (1..=5)
            .map(|i| {
                gift(
                    dec!(600),
                    Some(NoncashGiftProperty {
                        description_and_condition: format!("bag {i}"),
                        ..the_six_hundred_dollar_bag()
                    }),
                )
            })
            .collect();
        let rows =
            screen_noncash_gifts(&gifts, NoncashGate::FormIsFiled).expect("all five admitted");
        let descriptions: Vec<&str> = rows
            .iter()
            .map(|r| r.col_c_description_and_condition.as_str())
            .collect();
        assert_eq!(
            descriptions,
            ["bag 1", "bag 2", "bag 3", "bag 4", "bag 5"],
            "five gifts in, five rows out, in input order"
        );
    }

    /// Every [`NoncashGiftRefusal`] carries a distinct, non-empty message.
    #[test]
    fn every_refusal_has_its_own_message() {
        use std::collections::BTreeSet;
        let all = [
            NoncashGiftRefusal::DetailsNotCollected,
            NoncashGiftRefusal::DoneeNameAndAddressEmpty,
            NoncashGiftRefusal::DescriptionAndConditionEmpty,
            NoncashGiftRefusal::FmvMethodEmpty,
            NoncashGiftRefusal::ColumnsEfgRequiredButNotCollected,
            NoncashGiftRefusal::FmvReducedNeedsAttachedStatement,
            NoncashGiftRefusal::ClaimExceedsFairMarketValue,
            NoncashGiftRefusal::PubliclyTradedSecurity,
            NoncashGiftRefusal::Vehicle,
            NoncashGiftRefusal::IntellectualProperty,
            NoncashGiftRefusal::InventoryOrHeldForSale,
            NoncashGiftRefusal::OverFiveThousandNeedsSectionB,
            NoncashGiftRefusal::NotGoodUsedConditionNeedsSectionBAppraisal,
            NoncashGiftRefusal::NotGoodUsedConditionNotDeductible,
        ];
        let msgs: BTreeSet<&str> = all.iter().map(|r| r.detail()).collect();
        assert_eq!(
            msgs.len(),
            all.len(),
            "two conditions sharing a message tells the filer to fix the wrong thing"
        );
        assert!(all.iter().all(|r| !r.detail().trim().is_empty()));
    }

    /// The property block round-trips through serde with the shape `docs/income-import-schema.md`
    /// publishes — including the form's own word for column (e). `serde_json` rather than `toml`
    /// because `btctax-core` does not depend on `toml`; the SHAPE is the same, and the published schema
    /// is derived from exactly this serialization.
    #[test]
    fn the_property_block_round_trips_through_serde() {
        let p = NoncashGiftProperty {
            date_acquired_by_donor: Some(DateAcquiredByDonor::Various),
            ..the_six_hundred_dollar_bag()
        };
        let s = serde_json::to_string(&p).expect("serializes");
        assert!(
            s.contains("\"various\""),
            "the carve-out word is a snake_case unit variant: {s}"
        );
        assert!(
            s.contains("clothing_or_household_item") && s.contains("good_used_condition_or_better"),
            "the §170(f)(16) condition flag is a published key, not an implicit default: {s}"
        );
        let back: NoncashGiftProperty = serde_json::from_str(&s).expect("round-trips");
        assert_eq!(back, p);
    }

    /// `kind` deserializes from the spelling the schema publishes, and a DATE for column (e) takes the
    /// `on` form the externally-tagged enum produces.
    ///
    /// ★ The dates are `time`'s compact `[year, ordinal]` form because that is what a **JSON**
    /// [`TaxDate`] is; `docs/income-import-schema.md`'s *"a DATE takes EITHER spelling"* is about the
    /// TOML deserializer, and the ISO spelling a filer actually types is exercised against the real
    /// `income import` in `btctax-cli/tests/f8283_section_a_import.rs`.
    #[test]
    fn the_property_kind_deserializes_from_its_published_spelling() {
        let p: NoncashGiftProperty = serde_json::from_str(
            r#"{
              "kind": { "clothing_or_household_item": { "good_used_condition_or_better": true } },
              "donee_name_and_address": "Goodwill, 1 Main St",
              "description_and_condition": "used clothing, good used condition",
              "date_of_contribution": [2024, 335],
              "date_acquired_by_donor": { "on": [2019, 124] },
              "how_acquired_by_donor": "Purchase",
              "cost_or_adjusted_basis": "2400",
              "fair_market_value": "600",
              "method_used_to_determine_fmv": "Thrift shop value"
            }"#,
        )
        .expect("the published spelling parses");
        assert_eq!(
            p.kind,
            NoncashPropertyKind::ClothingOrHouseholdItem {
                good_used_condition_or_better: true
            }
        );
        assert_eq!(p.date_of_contribution, date!(2024 - 11 - 30));
        assert_eq!(
            p.date_acquired_by_donor,
            Some(DateAcquiredByDonor::On(date!(2019 - 05 - 04)))
        );
        assert!(section_a_row(dec!(600), Some(&p)).is_ok());
    }
}
