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

/// One Part I line item: a position taken on a filed form that rests on the estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part1Item {
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
                form: "8949".to_string(),
                line,
                description,
                amount: leg.basis, // ★ AS FILED — never the pre-clamp floor.
            });
        }
    }
    if part_i.is_empty() {
        return None;
    }
    let part_ii = targets
        .iter()
        .filter_map(|t| part_ii_narrative_for(events, t))
        .collect::<Vec<_>>()
        .join("\n\n");
    let incomplete = part_ii.trim().is_empty();
    Some(Disclosure8275 {
        part_i,
        part_ii,
        incomplete,
    })
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
