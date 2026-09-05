//! Approach-B / Task 13 — Form 8275 (Disclosure Statement) content: Part I (auto, one item per
//! promoted Form 8949 disposal leg filed in `year`) + Part II (the filer's own stored narrative,
//! BG-D7) + the BG-D10 penalty-risk copy `render()` always appends.
//!
//! **Disposal-scoped, not tag-scoped (BG-D11).** A promoted REMOVAL (gift/donation) leg files
//! documented-only (`conservative_promote::clamped_leg_basis` with `net_proceeds_share = $0` —
//! `forms.rs`'s §170(e) emitters + the 8283 basis column never print the estimate floor for a
//! removal) — so a removal never takes an estimated POSITION on the return, and there is nothing
//! for an 8275 to disclose about it. Only a promoted 8949 DISPOSAL leg files the estimate (Cohan)
//! floor as its col (e) basis, so `disclosure_8275` is scoped to disposals only.
//!
//! **The Part I amount is the AS-FILED col (e), never the pre-clamp floor (tax r1 I-6).** `leg.basis`
//! IS Form 8949 col (e) (`forms.rs::form_8949` copies it verbatim) — so reading it here can never
//! diverge from what the attached 8949 actually prints, even when BG-D4's clamp bound (a below-floor
//! sale: `leg.basis == leg.proceeds`, the same heuristic `conservative::basis_methodology` uses).
//! Disclosing the pre-clamp floor while filing less would recreate the exact examiner mismatch an
//! 8275 exists to prevent.
use crate::conventions::Usd;
use crate::event::{EventPayload, LedgerEvent};
use crate::identity::EventId;
use crate::state::{LedgerState, Term};
use std::collections::BTreeSet;

/// The exact Part-I description for a promoted disposal leg's estimated basis (BG-D7 copy, pinned by
/// the review loop). Cohan v. Commissioner is the estimate's own authority; "the bearing-heavily
/// minimum" names WHY the window-low close was chosen — the most conservative (taxpayer-adverse)
/// number the coverage supports, never a favorable one.
const PART_I_DESCRIPTION: &str = "basis estimated at the minimum daily closing price over the \
     attested acquisition window (Cohan; the bearing-heavily minimum)";

/// Appended to a promoted leg's description when BG-D4's loss clamp bit (`leg.basis >= leg.proceeds`,
/// i.e. gain <= 0): the estimate was limited so as not to report a loss the estimate itself
/// manufactured. Exact substring
/// `a_clamped_leg_disclosure_adds_the_no_loss_sentence_and_files_the_clamped_amount` pins.
const NO_LOSS_SUFFIX: &str = "; limited so as not to report a loss from the estimate";

/// The BG-D10 penalty-risk paragraph `render()` always appends. EXACT copy, pinned by the review
/// loop: the penalty base is the RESULTING ADDITIONAL TAX (never "the disallowed basis"), 20%
/// ordinary / 40% §6662(h) worst-case, plus interest; disclosure + good-faith methodology MITIGATE,
/// they do not ELIMINATE; adequate disclosure does NOT defeat the §6662(e)/(h) valuation-misstatement
/// penalty (Woods v. Commissioner — disclosure is a §6662(d) understatement-penalty defense, not a
/// §6662(e)/(h) one); and for charitable-deduction property §6664(c)(2) removes the reasonable-cause
/// defense outright. NEVER "safe harbor" — a promoted floor is a disclosed estimate, not a harbor.
const RISK_PARAGRAPH: &str = "Penalty exposure — if an exam determines a different basis, the penalty \
     is 20% ordinary / 40% worst-case of the resulting additional tax (the underpayment attributable \
     to the misstatement), plus interest; the Form 8275 disclosure and the good-faith window-low-close \
     methodology mitigate, they do not eliminate, that exposure; adequate disclosure does NOT protect \
     against the \u{00a7}6662(e)/(h) valuation-misstatement penalty (Woods v. Commissioner); for \
     charitable-deduction property, \u{00a7}6664(c)(2) removes the reasonable-cause defense.";

/// One Part I line item: a position taken on a filed form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part1Item {
    /// **Column (a) — "Rev. rul., rev. proc., etc."** `i8275--2024.txt:352-354`: *"If you are
    /// disclosing a position contrary to a rule (such as a statutory provision or IRS revenue ruling),
    /// you must identify the rule in column (a)."*
    ///
    /// ★ `None` for a promoted-basis leg, and that is not an omission: a Cohan estimate is a
    /// *valuation* position, contrary to no named rule. It is `Some("IRC section 1(g)")` for the
    /// FR-29 no-path position, which IS contrary to a statutory provision.
    pub rule: Option<String>,
    /// **Column (b) — "Item or group of items"** (`i8275--2024.txt:355`: *"Identify the item by
    /// name."*). `None` for a promoted-basis leg, whose identity is column (c)'s description plus the
    /// form/line in (d)/(e).
    pub item_name: Option<String>,
    /// The filed form this position is taken on — always `"8949"` here (BG-D11: a removal leg never
    /// contributes a Part I item; see module doc).
    pub form: String,
    /// The column/line the position occupies (e.g. `"Part I \u{2014} column (e)"` for a short-term
    /// leg, `"Part II \u{2014} column (e)"` for long-term — Form 8949's own Part I/II split).
    pub line: String,
    /// The Cohan-estimate explanation, `NO_LOSS_SUFFIX`-appended when BG-D4's clamp bound.
    pub description: String,
    /// The AS-FILED Form 8949 col (e) basis for this leg (`leg.basis` — the clamped amount where the
    /// clamp bound, NEVER the pre-clamp floor).
    pub amount: Usd,
}

/// Form 8275 content: Part I (auto) + Part II (the filer's own narrative) + the incompleteness flag
/// T14's export-refusal gate keys on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disclosure8275 {
    /// One item per promoted Form 8949 disposal leg filed in `year` (BG-D11: never a removal leg).
    pub part_i: Vec<Part1Item>,
    /// The promote's stored `part_ii_narrative` (BG-D7) — present-by-construction at record time
    /// (`cmd/promote.rs`'s empty/whitespace refusal), but a raw-vault write can still bypass that CLI
    /// gate, so this is read back from the event rather than assumed non-empty.
    pub part_ii: String,
    /// `true` iff `part_ii.trim()` is empty — the raw-vault-bypass condition T14's export-refusal
    /// gate keys on (present-by-construction is a CLI-layer guarantee, not a type-level one).
    pub incomplete: bool,
}

/// True iff `target` is named by any (non-voided-relevant here — see below) live `PromoteTranche`
/// whose id itself has NOT been voided. A `PromoteTranche` decision is itself voidable (BG-D9); a
/// voided promote attempt must never donate its narrative to a later, actually-live promote on the
/// same target.
fn part_ii_narrative_for(events: &[LedgerEvent], target: &EventId) -> Option<String> {
    let voided: BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect();
    events.iter().find_map(|e| match &e.payload {
        EventPayload::PromoteTranche(p) if p.target == *target && !voided.contains(&e.id) => {
            Some(p.part_ii_narrative.clone())
        }
        _ => None,
    })
}

/// Build the Form 8275 disclosure for `year`, or `None` when no promoted DISPOSAL leg files in it.
///
/// ★ `None` for a promoted REMOVAL-only year (BG-D11): a promoted gift/donation leg files
/// documented-only (the estimate evaporates — `conservative_promote::clamped_leg_basis` with
/// `net_proceeds_share = $0`), so it takes no estimated position on the return to disclose. Only a
/// promoted 8949 DISPOSAL leg does, so `disclosure_8275` scans `state.disposals` exclusively.
pub fn disclosure_8275(
    events: &[LedgerEvent],
    state: &LedgerState,
    year: i32,
    section_1g: Option<&Section1gPosition>,
) -> Option<Disclosure8275> {
    let mut part_i: Vec<Part1Item> = Vec::new();
    let mut targets: BTreeSet<EventId> = BTreeSet::new();
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in d
            .legs
            .iter()
            .filter(|l| state.promoted_origins.contains(&l.lot_id.origin_event_id))
        {
            targets.insert(leg.lot_id.origin_event_id.clone());
            let mut description = PART_I_DESCRIPTION.to_string();
            // BG-D4: `leg.basis >= leg.proceeds` (i.e. gain <= 0) is the clamp-bound heuristic
            // `conservative::basis_methodology` uses. For a PROMOTED leg (this loop is
            // `promoted_origins`-scoped), gain <= 0 means the estimate was limited so as not to
            // manufacture a loss off it: the pure below-floor clamp files `basis == proceeds` (gain 0),
            // and a below-floor sale that ALSO carries a documented TP8(c) fee (re-homed AFTER the
            // clamp) files `basis == proceeds + documented_fee > proceeds` (a small documented loss) —
            // the `==`-only test used to miss that corner (whole-branch tax review M1). (A promoted leg
            // sold ABOVE its floor files `basis = floor < proceeds`, gain > 0, so this stays false.)
            if leg.basis >= leg.proceeds {
                description.push_str(NO_LOSS_SUFFIX);
            }
            let line = match leg.term {
                Term::ShortTerm => "Part I \u{2014} column (e)",
                Term::LongTerm => "Part II \u{2014} column (e)",
            }
            .to_string();
            part_i.push(Part1Item {
                // ★ A Cohan basis estimate is contrary to no NAMED rule, so columns (a) and (b) stay
                //   blank — the position is identified by (c)/(d)/(e). See `Part1Item::rule`.
                rule: None,
                item_name: None,
                form: "8949".to_string(),
                line,
                description,
                amount: leg.basis, // ★ AS FILED — never the pre-clamp floor.
            });
        }
    }
    let mut part_ii = targets
        .iter()
        .filter_map(|t| part_ii_narrative_for(events, t))
        .collect::<Vec<_>>()
        .join("\n\n");
    // ★★★ FR-29 / SPEC §6.3.3 — THE SECOND PART I ITEM SOURCE. A return that computes on the no-path
    //     certification takes a position contrary to a STATUTE, and it must not file silently: an
    //     undisclosed position here would be strictly worse than the refusal it replaced.
    //
    //     ★ Note what this changes about the `None` below: `disclosure_8275` used to return `None`
    //       whenever `part_i` was empty, i.e. whenever no promoted disposal leg filed. A §1(g)-only
    //       year has no promoted leg at all, so leaving that early return in place would have dropped
    //       the disclosure entirely. It is now keyed on the WHOLE of Part I.
    if let Some(pos) = section_1g {
        part_i.push(section_1g_part_i(pos));
        if !part_ii.trim().is_empty() {
            part_ii.push_str("\n\n");
        }
        part_ii.push_str(&section_1g_part_ii(pos));
    }
    if part_i.is_empty() {
        return None;
    }
    let incomplete = part_ii.trim().is_empty();
    Some(Disclosure8275 {
        part_i,
        part_ii,
        incomplete,
    })
}

/// SPEC §6.3.3 — the facts a §1(g) no-path disclosure states. Built by
/// [`crate::tax::return_1040::assemble_absolute`], which is the one place that holds both the §1(g)
/// threshold and the ledger, so the disclosure and the gate cannot disagree about whether the position
/// was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section1gPosition {
    /// The filer's unearned income for the year (SPEC §5.2's component sum).
    pub unearned: Usd,
    /// The §1(g) threshold that year's params carry.
    pub threshold: Usd,
    /// **Form 1040 line 16 AS FILED** — the tax computed WITHOUT regard to §1(g). Part I column (f).
    pub tax_line16: Usd,
}

/// SPEC §6.3.3 — Part I, one row, mapped to Form 8275's own columns
/// (`design/forms/extract/i8275--2024.txt:352-374`).
pub fn section_1g_part_i(pos: &Section1gPosition) -> Part1Item {
    Part1Item {
        rule: Some("IRC section 1(g)".to_string()),
        item_name: Some("Tax on unearned income of a child — Form 8615 not filed".to_string()),
        form: "1040".to_string(),
        line: "16".to_string(),
        description: "The taxpayer's tax was figured without regard to section 1(g). The taxpayer \
             cannot identify either parent and therefore cannot supply the parent's name, SSN or \
             filing status required by Form 8615 lines A, B and C, cannot obtain the parent's taxable \
             income required by section 1(g)(3), and cannot make the request described in the \
             Instructions for Form 8615 under \"Parent's return information unavailable\", which \
             requires the parent's name and address."
            .to_string(),
        // ★ Column (f) is the amount AS FILED — the tax on Form 1040 line 16 computed WITHOUT §1(g),
        //   which is exactly the position being disclosed. Never a hypothetical §1(g) figure: btctax
        //   cannot compute one, and disclosing a number the return does not carry would recreate the
        //   examiner mismatch an 8275 exists to prevent.
        amount: pos.tax_line16,
    }
}

/// SPEC §6.3.3 — Part II, three paragraphs in this order.
///
/// `i8275--2024.txt:378-388` requires Part II to *"include information that can reasonably be expected
/// to apprise the IRS of the identity of the item, its amount, and the nature of the controversy or
/// potential controversy"*, and says it *"can include a description of the legal issues presented by
/// the facts"*.
///
/// ★★★ **The IMPOSSIBILITY argument LEADS**, per the owner ruling: the predicate cannot be established,
/// and the government's own remedy excludes the filer by its own required contents. That is materially
/// stronger than a constitutional one. **No constitutional argument appears anywhere in this
/// disclosure, and none may be added** — the ruling records that the controller is aware of NO authority
/// holding §1(g) invalid as applied here, and a disclosure that led with a constitutional theory would
/// be weaker, not stronger.
///
/// ★ Every sentence here is in BTCTAX's voice, never the filer's. The filer's own testimony is the two
/// facts recited in paragraph 1; the legal CONCLUSION is btctax's, because a return is signed under
/// §6065 and putting a conclusion in the filer's mouth would fabricate testimony.
pub fn section_1g_part_ii(pos: &Section1gPosition) -> String {
    let Section1gPosition {
        unearned,
        threshold,
        ..
    } = pos;
    format!(
        "Section 1(g) — tax on the unearned income of a child. The taxpayer states that they cannot \
         identify either parent, and that they cannot supply either parent's name and address. The \
         taxpayer's unearned income for the year was ${unearned}, above the section 1(g) threshold of \
         ${threshold}.\n\n\
         Form 8615 cannot be completed and section 1(g)(3) cannot be computed on these facts: lines A, \
         B and C of Form 8615 require the parent's name, social security number and filing status, and \
         the allocable parental tax is figured on the parent's taxable income. The sole administrative \
         remedy the Internal Revenue Service provides for a child who cannot obtain the parent's \
         information is a written request to the Service, and that remedy is unavailable to this \
         taxpayer because its own required contents include the parent's name and address — the very \
         facts the taxpayer lacks. The social security number and the filing status are expressly \
         qualified \"(if known)\"; the name and the address are not. Section 1(g) is therefore not \
         established on this return.\n\n\
         The return reports all of the taxpayer's income. The position is limited to the RATE at which \
         the unearned portion is taxed; no exclusion, deduction or credit is claimed on account of it. \
         The taxpayer will complete Form 8615 if the parent's information later becomes available."
    )
}

impl Disclosure8275 {
    /// Render the full disclosure text: Part I items, Part II narrative (or an incompleteness flag),
    /// then the BG-D10 risk paragraph — ALWAYS appended, regardless of `incomplete` (the risk is real
    /// whether or not the filer's own narrative is on record).
    pub fn render(&self) -> String {
        let mut out = String::from("Form 8275 \u{2014} Disclosure Statement\n\n");
        out.push_str("Part I \u{2014} Disclosure of Positions Taken\n\n");
        for item in &self.part_i {
            out.push_str(&format!(
                "  \u{2022} Form {form}, {line}: ${amount:.2} \u{2014} {desc}\n",
                form = item.form,
                line = item.line,
                amount = item.amount,
                desc = item.description,
            ));
        }
        out.push_str("\nPart II \u{2014} Detailed Explanation\n\n");
        if self.incomplete {
            out.push_str("[INCOMPLETE \u{2014} no Part II narrative on record]\n");
        } else {
            out.push_str(&self.part_ii);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(RISK_PARAGRAPH);
        out.push('\n');
        out
    }
}
