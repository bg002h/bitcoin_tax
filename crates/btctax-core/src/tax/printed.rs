//! Full-return v1 **printed line chains** for the 1040's numbered schedules (P6 / SPEC §3.1).
//!
//! Every form btctax files needs a *printed* chain distinct from the exact-cents computation:
//!
//! - each printed line is `round_dollar`ed **at the line**, and
//! - each printed **total sums the already-rounded lines above it**, so the filed form cross-foots.
//!
//! That is deliberately NOT `round_dollar(exact_total)` — with two `.50` components the two differ by
//! a dollar, and the cross-footing one is what a human re-adding the column gets (SPEC §10 KAT-9).
//!
//! **The chains compose.** A schedule that carries a figure from another form takes that form's
//! **printed** line, never the exact-cents computation behind it — Schedule 2 line 11 is Form 8959's
//! printed line 18, not `round_dollar(additional_medicare.additional_medicare_tax)`. Otherwise the
//! attached form and the schedule that references it would disagree by a dollar, and the return would
//! not tie out. This is why the builders below take the upstream `*Lines` structs as arguments.
//!
//! **`btctax-forms` does no tax arithmetic**: it transcribes these structs cell-for-cell. A second,
//! independent derivation in the filler is exactly how a filed PDF comes to disagree with the tax it
//! reports, and no core KAT would catch it.

use crate::conventions::{round_dollar, Usd};
use crate::tax::other_taxes::{Form8959Lines, Form8960Lines};
use crate::tax::return_1040::{AbsoluteReturn, MEDICAL_FLOOR_RATE};
use crate::tax::types::FilingStatus;

// ── Form 8949 — the printed rows (SPEC §3.1 / ARCH-P6.3a D2) ────────────────────────────────────

/// One PRINTED Form 8949 row: columns (d), (e) and (h) as they appear on the filed page.
///
/// ★ **Column (h) is DERIVED, never independently rounded.** The form's own column-(h) header says
/// "Subtract column (e) from column (d) and combine the result with column (g)" — an instruction about
/// the PRINTED cells. Rounding the exact gain independently produces rows that visibly contradict their
/// own subtraction (proceeds 100.49 → 100, basis 0.50 → 1, exact gain 99.99 → 100, but 100 − 1 = 99),
/// and it drifts Σh away from Σd − Σe, which would then break Schedule D's Part I, whose columns carry
/// the same subtract-and-combine header. Deriving h makes both identities hold exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8949Row {
    /// (a) description — the exact BTC amount, 8dp (never rounded: it is a quantity, not money).
    pub description: String,
    /// (b) date acquired.
    pub date_acquired: crate::conventions::TaxDate,
    /// (c) date sold.
    pub date_sold: crate::conventions::TaxDate,
    /// (d) proceeds — `round_dollar` at the cell.
    pub proceeds_d: Usd,
    /// (e) cost basis — `round_dollar` at the cell.
    pub cost_e: Usd,
    /// (h) gain/loss = **printed (d) − printed (e)** (column (g) is always blank in v1).
    pub gain_h: Usd,
}

/// A part's totals row — the sum of the PRINTED rows above it (never a re-rounding of the exact sum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Printed8949Totals {
    pub proceeds_d: Usd,
    pub cost_e: Usd,
    pub gain_h: Usd,
}

/// The printed Form 8949: Part I (short-term) and Part II (long-term), each with its rows and its
/// totals row. The conservative "not reported" box is year-aware (C/F pre-TY2025; the digital-asset
/// I/L from TY2025). These totals ARE Schedule D lines 3 and 10 — the schedule's own text defines
/// them as "Totals for all transactions reported on Form(s) 8949 with Box C/F checked" pre-2025, and
/// "with Box C or Box I checked" (line 3) / "Box F or Box L checked" (line 10) on the 2025 revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8949 {
    pub short_term: Vec<Printed8949Row>,
    pub long_term: Vec<Printed8949Row>,
    pub st_totals: Printed8949Totals,
    pub lt_totals: Printed8949Totals,
}

/// Derive the printed Form 8949 from the ledger's rows. `None` when the year has no disposals — a
/// carryover/distribution-only Schedule D files with lines 3/10 blank and NO 8949 attached.
pub fn form_8949_printed(rows: &[crate::forms::Form8949Row]) -> Option<Printed8949> {
    use crate::forms::Form8949Part;
    if rows.is_empty() {
        return None;
    }
    let printed = |r: &crate::forms::Form8949Row| {
        let proceeds_d = round_dollar(r.proceeds);
        let cost_e = round_dollar(r.cost_basis);
        Printed8949Row {
            description: r.description.clone(),
            date_acquired: r.date_acquired,
            date_sold: r.date_sold,
            proceeds_d,
            cost_e,
            // ★ derived from the PRINTED cells, not from `r.gain`
            gain_h: proceeds_d - cost_e,
        }
    };
    let total = |rs: &[Printed8949Row]| Printed8949Totals {
        proceeds_d: rs.iter().map(|r| r.proceeds_d).sum(),
        cost_e: rs.iter().map(|r| r.cost_e).sum(),
        gain_h: rs.iter().map(|r| r.gain_h).sum(),
    };

    let short_term: Vec<_> = rows
        .iter()
        .filter(|r| r.part == Form8949Part::ShortTerm)
        .map(printed)
        .collect();
    let long_term: Vec<_> = rows
        .iter()
        .filter(|r| r.part == Form8949Part::LongTerm)
        .map(printed)
        .collect();

    Some(Printed8949 {
        st_totals: total(&short_term),
        lt_totals: total(&long_term),
        short_term,
        long_term,
    })
}

// ── Form 8283 — the printed rows (ARCH-P6.3a D7) ────────────────────────────────────────────────

/// The PRINTED Form 8283 rows: the same rows the crypto slice emits, with the three money columns
/// (`cost_basis`, `fmv`, `claimed_deduction`) rounded to whole dollars for the filed packet.
///
/// A **newtype**, not a bare `Vec<Form8283Row>`, on purpose: the slice's rows carry CENTS, and the two
/// must not be interchangeable. Handing a cents row to the full-return filler would put an 8283 under a
/// different rounding regime from the Schedule A it is attached to — which is the whole defect this
/// phase exists to close.
///
/// **Unlike Schedule D ← 8949, Schedule A line 12 does NOT re-derive from this.** The rule is: a line
/// whose form text CITES another form as its source composes on that form's printed line; a line that
/// merely REQUIRES an attachment does not. Schedule A L12's text is "…You must attach Form 8283 if over
/// $500" — an attachment requirement. Form 8283 has no grand-total line for L12 to equal, and the
/// §170(b) ceilings legitimately make L12 *smaller* than the sum of the 8283's per-donation amounts (the
/// excess becomes carryover). Forcing them equal would be wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8283Rows(Vec<crate::forms::Form8283Row>);

impl Printed8283Rows {
    /// The rows, whole-dollar.
    pub fn rows(&self) -> &[crate::forms::Form8283Row] {
        &self.0
    }
}

/// Round the ledger's Form 8283 rows for the filed packet. `None` when there are no donation rows.
///
/// The PRESENCE rule (does an 8283 get attached at all?) is the packet's, not this function's: the form
/// is required when the return itemizes AND the printed Schedule A line 12 exceeds $500.
pub fn form_8283_printed(rows: &[crate::forms::Form8283Row]) -> Option<Printed8283Rows> {
    if rows.is_empty() {
        return None;
    }
    Some(Printed8283Rows(
        rows.iter()
            .map(|r| crate::forms::Form8283Row {
                cost_basis: round_dollar(r.cost_basis),
                fmv: round_dollar(r.fmv),
                claimed_deduction: r.claimed_deduction.map(round_dollar),
                ..r.clone()
            })
            .collect(),
    ))
}

/// Form 8283 is REQUIRED when the return itemizes and its printed noncash gifts exceed $500 — the
/// threshold is printed on Schedule A line 12 itself ("You must attach Form 8283 if over $500").
pub const FORM_8283_THRESHOLD: Usd = rust_decimal_macros::dec!(500);

// ── Schedule SE — the printed chain (ARCH-P6.3a D5) ─────────────────────────────────────────────

/// The printable **Schedule SE** line chain.
///
/// ★ Schedule SE is a **filed form**, not a worksheet. The SE *engine* (`se.rs`) stays a FROZEN
/// exact-cents worksheet; this chain rounds its values AT THE LINE and derives the additions from the
/// printed operands, exactly like every other filed form under the §3.1 election. (SPEC §3.1 listed "SE"
/// among the unprinted worksheets — a contradiction with §7.1's fill set, amended alongside this work.)
///
/// The citations bind it to its neighbours, and the form states each one:
/// - **line 2** ← Schedule C's printed **line 31** ("Net profit or (loss) from Schedule C, line 31")
/// - **line 12** → Schedule 2 **line 4** ("Enter here and on Schedule 2 (Form 1040), line 4")
/// - **line 13** → Schedule 1 **line 15** ("Enter here and on Schedule 1 (Form 1040), line 15")
/// - **line 6** ← cited BY Form 8959 line 8 ("from Schedule SE, Part I, line 6")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleSeLines {
    /// L2 — net profit from Schedule C line 31.
    pub line2: Usd,
    /// L3 — combine 1a, 1b and 2 (1a/1b blank) ⇒ `= line2`.
    pub line3: Usd,
    /// L4a — line 3 × 92.35% (the §1402(a) base).
    pub line4a: Usd,
    /// L4c — add 4a and 4b (4b blank) ⇒ `= line4a`.
    pub line4c: Usd,
    /// L6 — add 4c and 5b (5b blank) ⇒ `= line4c`. **Form 8959 line 8 cites THIS line.**
    pub line6: Usd,
    /// L8a — the earner's own W-2 Social Security wages (boxes 3 + 7).
    pub line8a: Usd,
    /// L8d — add 8a, 8b, 8c (8b/8c blank) ⇒ `= line8a`.
    pub line8d: Usd,
    /// L9 — subtract PRINTED 8d from line 7 (the wage base), floored at 0.
    pub line9: Usd,
    /// L10 — 12.4% × the smaller of line 6 or line 9 (the §1402(b)(1) cap).
    pub line10: Usd,
    /// L11 — 2.9% × line 6.
    pub line11: Usd,
    /// L12 — **self-employment tax = add the PRINTED 10 and 11** → Schedule 2 line 4.
    pub line12: Usd,
    /// L13 — the §164(f) one-half deduction → Schedule 1 line 15.
    pub line13: Usd,
}

/// Derive the printed Schedule SE chain. `None` when there is no SE tax — the §6017 $400 floor already
/// dropped `ar.se` to `None` upstream, so no Schedule SE is filed and Schedule 2 line 4 is zero.
///
/// `line13` takes the ENGINE's `deductible_half` rounded at the line, rather than recomputing 50% of the
/// printed line 12. The two can differ by a dollar (ss = 10.50, medicare = 11.50 ⇒ printed L12 = 23,
/// half = 12, but `round_dollar(11.00)` = 11); taking the engine value keeps §164(f) faithful to the
/// statute and makes the Schedule 1 line-15 tie-out hold by construction. A human recomputing "50% of
/// line 12" by hand may therefore land a dollar away — the same accepted residual class as Form 8959's
/// shipped line 7 (ARCH-P6.3a D5, decision recorded with its alternative).
pub fn schedule_se_lines(ar: &AbsoluteReturn, sch_c: &ScheduleCLines) -> Option<ScheduleSeLines> {
    let se = ar.se.as_ref()?;
    let pi = &ar.printed_inputs;

    let line2 = sch_c.line31; // ★ the PRINTED Schedule C line 31 — one figure, two destinations
    let line3 = line2;
    let line4a = round_dollar(se.base);
    let line4c = line4a;
    let line6 = line4c;

    let line8a = round_dollar(pi.se_w2_ss_wages);
    let line8d = line8a;
    let line9 = (round_dollar(pi.ss_wage_base) - line8d).max(Usd::ZERO);

    let line10 = round_dollar(se.ss);
    let line11 = round_dollar(se.medicare);
    let line12 = line10 + line11; // ★ "Add lines 10 and 11" — over the PRINTED lines
    let line13 = round_dollar(se.deductible_half);

    Some(ScheduleSeLines {
        line2,
        line3,
        line4a,
        line4c,
        line6,
        line8a,
        line8d,
        line9,
        line10,
        line11,
        line12,
        line13,
    })
}

/// The printable **Schedule 2 (Additional Taxes)** line chain.
///
/// **Part I is entirely BLANK in v1**, and that is a load-bearing fact rather than an omission:
/// line 1a (excess advance premium tax credit) has no input and would REFUSE if it did (repaying it
/// *increases* tax, so omitting it would understate), and line 2 (AMT) is $0 by construction — the
/// return is refused outright if the official "Should You Fill In Form 6251" worksheet trips. So
/// 1040 line 17 is zero, and nothing in Part I is printed.
///
/// Part II carries the three taxes v1 does compute. Note **line 4 excludes the 0.9% Additional
/// Medicare Tax**: that is a Form 8959 item routed to line 11, and bundling it into line 4 would
/// double-count it (deep/02 C5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule2Lines {
    /// L4 — self-employment tax (Schedule SE): §1401(a) Social Security + §1401(b)(1) regular
    /// Medicare ONLY.
    pub line4: Usd,
    /// L11 — Additional Medicare Tax: **Form 8959's printed line 18**.
    pub line11: Usd,
    /// L12 — net investment income tax: **Form 8960's printed line 17**. Zero when no NIIT is owed.
    pub line12: Usd,
    /// L21 — total other taxes = add 4, 7 through 16, 18 and 19 ⇒ `4 + 11 + 12` here → 1040 **L23**.
    pub line21: Usd,
}

/// Derive the printed Schedule 2 chain. Takes the **printed** 8959/8960 chains, not the computed
/// figures, so the schedule and its attachments agree to the dollar.
///
/// Returns `None` when there is nothing to report — no SE tax, no Additional Medicare Tax, no NIIT —
/// in which case Schedule 2 is not filed at all and 1040 line 23 is zero.
pub fn schedule_2_lines(
    sch_se: Option<&ScheduleSeLines>,
    f8959: &Form8959Lines,
    f8960: Option<&Form8960Lines>,
) -> Option<Schedule2Lines> {
    // ★ L4 IS Schedule SE's PRINTED line 12 — the form says so ("Enter here and on Schedule 2, line 4").
    // A re-rounding of the exact SE tax would put this schedule a dollar away from the Schedule SE
    // attached to it, which is the same defect the L11 ← 8959 rule already forbids (ARCH-P6.3a D3).
    let line4 = sch_se.map_or(Usd::ZERO, |s| s.line12);
    let line11 = f8959.line18; // already a printed whole dollar
    let line12 = f8960.map_or(Usd::ZERO, |f| f.line17); // ditto
    let line21 = line4 + line11 + line12; // ★ sums the PRINTED lines

    if line21 <= Usd::ZERO {
        return None;
    }
    Some(Schedule2Lines {
        line4,
        line11,
        line12,
        line21,
    })
}

/// The printable **Schedule 1 (Additional Income and Adjustments to Income)** line chain.
///
/// **Unmodeled lines are BLANK, not zero**: line 2a/2b (alimony), 4 (Form 4797), 5 (Schedule E —
/// unrepresentable in v1), 6 (Schedule F), most of the 8a–8z write-ins, and in Part II lines 11–14,
/// 16, 17, 19, 20, 23 and all of 24a–24z. **Line 22 is the IRS's own "Reserved for future use"** —
/// a live ReadOnly widget that must never be written.
///
/// v1's crypto ordinary income lands on **line 8v** ("Digital assets received as ordinary income not
/// reported elsewhere") when it is NOT a trade or business; business crypto goes to line 3 via
/// Schedule C instead. The two are mutually exclusive by construction, which is why both can be
/// printed without double-counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule1Lines {
    /// L1 — taxable state/local income-tax refund.
    pub line1: Usd,
    /// L3 — business income (the crypto Schedule C net).
    pub line3: Usd,
    /// L7 — unemployment compensation.
    pub line7: Usd,
    /// L8v — digital assets received as ordinary income (non-business).
    pub line8v: Usd,
    /// L9 — total other income = add **printed** 8a through 8z ⇒ `= line8v` here.
    pub line9: Usd,
    /// L10 — combine **printed** 1 through 7 and 9 → 1040 **L8**.
    pub line10: Usd,
    /// L15 — the §164(f) deductible part of self-employment tax.
    pub line15: Usd,
    /// L18 — penalty on early withdrawal of savings.
    pub line18: Usd,
    /// L21 — the §221 student-loan interest deduction (post-phase-out).
    pub line21: Usd,
    /// L26 — add **printed** 11 through 23 and 25 ⇒ `15 + 18 + 21` here → 1040 **L10**.
    pub line26: Usd,
}

/// Derive the printed Schedule 1 chain. Returns `None` when there is neither additional income nor an
/// adjustment — the schedule is then not filed, and 1040 lines 8 and 10 are zero.
pub fn schedule_1_lines(ar: &AbsoluteReturn) -> Option<Schedule1Lines> {
    let p = &ar.schedule_1;

    // Part I — additional income.
    let line1 = round_dollar(p.state_refund_1);
    let line3 = round_dollar(p.schedule_c_net_3);
    let line7 = round_dollar(p.unemployment_7);
    let line8v = round_dollar(p.crypto_ordinary_8v);
    let line9 = line8v; // 8a-8u and 8w-8z are blank
    let line10 = line1 + line3 + line7 + line9; // ★ sums the PRINTED lines

    // Part II — adjustments to income.
    let line15 = round_dollar(p.half_se_15);
    let line18 = round_dollar(p.early_withdrawal_18);
    let line21 = round_dollar(p.student_loan_21);
    let line26 = line15 + line18 + line21; // ★ sums the PRINTED lines

    if line10 <= Usd::ZERO && line26 <= Usd::ZERO {
        return None;
    }
    Some(Schedule1Lines {
        line1,
        line3,
        line7,
        line8v,
        line9,
        line10,
        line15,
        line18,
        line21,
        line26,
    })
}

/// The printable **Form 1040** line chain — the return itself.
///
/// **★ Every line that comes from a schedule takes that schedule's PRINTED figure**, never a
/// re-rounding of the exact-cents computation: line 2b is Schedule B's printed line 4, line 8 is
/// Schedule 1's printed line 10, line 12 is Schedule A's printed line 17 (when itemizing), line 13 is
/// Form 8995's printed line 15, line 20 is Schedule 3's printed line 8, line 23 is Schedule 2's
/// printed line 21, line 25c is Form 8959's printed line 24. Take the exact figure instead and the
/// 1040 disagrees with its own attachments by a dollar, and the filed return does not tie out. This
/// is why the builder takes every upstream chain as an argument rather than reaching into
/// `AbsoluteReturn` for the totals.
///
/// **Line 7 is the one signed cell**, and it is signed with a LEADING MINUS, not parentheses (SPEC
/// §3.2) — unlike Schedule D's own lines 6/14/21, which are parenthesized boxes carrying magnitudes.
/// On a net-loss year it carries `−(Schedule D line 21)`, the §1211(b)-limited amount, not the full
/// loss.
///
/// **Conservative omissions print as absent, not zero, where the form allows** — but line 19 (the
/// CTC/ODC) is a computed credit line the form expects, so it prints `0` with the `CtcOdcOmitted`
/// advisory carrying the news that the filer may be owed more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form1040Lines {
    /// L1a / L1z — wages (Σ W-2 box 1). v1 has no other line-1 component, so 1z = 1a.
    pub line1z: Usd,
    /// L1a — "Total amount from Form(s) W-2, box 1". ★ Its absence left the filed 1z sitting above an
    /// EMPTY operand column: the form's own "Add lines 1a through 1h" sums blanks to 0 ≠ 1z, on the very
    /// line the Service document-matches against your W-2s (Fable P6 r1 I1).
    pub line1a: Usd,
    /// L2a — **tax-exempt interest** (Σ 1099-INT box 8 + 1099-DIV box 12). Changes no tax; the IRS
    /// document-matches it, and a return that omits it misstates itself (ARCH-P6.3a Q7 item 2).
    pub line2a: Usd,
    /// L2b — taxable interest: **Schedule B's printed line 4** when Schedule B files, else the sum.
    pub line2b: Usd,
    /// L3a — qualified dividends (Σ 1099-DIV box 1b; the preferential slice).
    pub line3a: Usd,
    /// L3b — ordinary dividends: **Schedule B's printed line 6** when Schedule B files, else the sum.
    /// This is the FULL box-1a amount, which INCLUDES the qualified subset on 3a.
    pub line3b: Usd,
    /// L7 — capital gain or (loss): **Schedule D's printed line 16**, or on a net-loss year the
    /// §1211(b)-limited `−(Schedule D line 21)`. Signed with a **leading minus**.
    pub line7: Usd,
    /// L8 — **Schedule 1's printed line 10** (additional income).
    pub line8: Usd,
    /// L9 — total income = add **printed** 1z, 2b, 3b, 7 and 8.
    pub line9: Usd,
    /// L10 — **Schedule 1's printed line 26** (adjustments).
    pub line10: Usd,
    /// L11 — **AGI** = printed 9 − printed 10.
    pub line11: Usd,
    /// L12 — the deduction actually claimed: **Schedule A's printed line 17** when itemizing, else the
    /// §63 standard deduction.
    pub line12: Usd,
    /// L13 — **Form 8995's printed line 15** (the §199A QBI deduction).
    pub line13: Usd,
    /// L14 — add **printed** 12 and 13.
    pub line14: Usd,
    /// L15 — **taxable income** = printed 11 − printed 14, floored at 0.
    pub line15: Usd,
    /// L16 — the regular tax (Tax Table / TCW / QDCGT, per `method.rs`).
    pub line16: Usd,
    /// L17 — **Schedule 2's printed line 3** (Part I). Always 0 in v1 — see [`Schedule2Lines`].
    pub line17: Usd,
    /// L18 — add **printed** 16 and 17.
    pub line18: Usd,
    /// L19 — CTC / credit for other dependents. **Always 0** (a §3.4 conservative omission; the
    /// `CtcOdcOmitted` advisory tells the filer their tax is overstated).
    pub line19: Usd,
    /// L20 — **Schedule 3's printed line 8** (nonrefundable credits: the FTC).
    pub line20: Usd,
    /// L21 — add **printed** 19 and 20.
    pub line21: Usd,
    /// L22 — printed 18 − printed 21, floored at 0.
    pub line22: Usd,
    /// L23 — **Schedule 2's printed line 21** (other taxes).
    pub line23: Usd,
    /// L24 — **TOTAL TAX** = add **printed** 22 and 23.
    pub line24: Usd,
    /// L25a — federal income tax withheld from Form(s) W-2 (Σ box 2).
    pub line25a: Usd,
    /// L25b — withheld from Form(s) 1099 (Σ box 4).
    pub line25b: Usd,
    /// L25c — withheld from other forms: **Form 8959's printed line 24** (the Additional-Medicare
    /// over-withholding credit) plus any other declared withholding.
    pub line25c: Usd,
    /// L25d — add **printed** 25a, 25b and 25c.
    pub line25d: Usd,
    /// L26 — estimated tax payments.
    pub line26: Usd,
    /// L31 — **Schedule 3's printed line 15** (other payments: extension + excess Social Security).
    pub line31: Usd,
    /// L32 — total other payments and refundable credits ⇒ `= line31` (lines 27–30 are blank: the EIC
    /// is a §3.4 conservative omission, and the rest are unrepresentable).
    pub line32: Usd,
    /// L33 — **TOTAL PAYMENTS** = add **printed** 25d, 26 and 32.
    pub line33: Usd,
    /// L34 / L35a — the overpayment refunded. Zero when the return owes. v1 never fills the
    /// direct-deposit block (35b–35d), so a refund arrives as a paper check — the `RefundByPaperCheck`
    /// advisory says so.
    pub line34: Usd,
    /// L37 — the amount owed. Zero when the return is due a refund.
    pub line37: Usd,
    /// The **Digital Asset question**. `true` for any crypto disposal, income, gift or donation.
    /// btctax never answers "No" — a "No" it cannot vouch for is worse than leaving the question to
    /// the filer, so this is `true` or the question is left for them.
    pub digital_asset_yes: bool,
}

/// The 1040's **income block** (lines 1a–11), printed.
///
/// Extracted from [`form_1040_lines`] because **Schedule A line 2 cites it**: "Enter amount from Form
/// 1040 or 1040-SR, **line 11**". Under the §3.1 citation-composition rule that is a SOURCE citation, so
/// Schedule A must take the PRINTED L11 — not a re-rounding of the exact AGI, which can differ by
/// dollars and, on a negative-AGI itemizer, prints 0 beside a negative 1040 L11 (Fable P6 r1 I5).
///
/// There is no cycle: L11 = L9 − L10 depends on Schedules B/1/D, never on Schedule A (which lands at
/// L12). So the income block is derived first, Schedule A composes on its L11, and the rest of the 1040
/// consumes both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form1040Income {
    /// The QDCGT worksheet's **line 3** operand: `min(printed Sch D L15, printed Sch D L16)`, floored at
    /// 0 — the §1(h) preferential net capital gain **as a human reads it off the FILED Schedule D**
    /// (`qdcgt_line16`'s own doc: "`net_ltcg` = `min(Sch D 15,16)`, ≥0; L3").
    ///
    /// ★ It is NOT `round_dollar(exact net_ltcg)` (Fable P6 r2 NEW-I1). The printed Sch D L15/L16 are sums
    /// of printed operands and drift from the exact figure — that is what the 302≠301 8949 KAT is about.
    /// A $1 drift here shifts the worksheet's ordinary remainder by $1, and THAT feeds the same $50-tread
    /// Tax Table step function line 16 was already gated on: across a bin edge the filed L16 disagrees
    /// with the worksheet a filer (or the Service) computes from the filed Schedule D, by a whole bin.
    pub qdcgt_net_capital_gain: Usd,
    pub line1a: Usd,
    pub line1z: Usd,
    pub line2a: Usd,
    pub line2b: Usd,
    pub line3a: Usd,
    pub line3b: Usd,
    pub line7: Usd,
    pub line8: Usd,
    pub line9: Usd,
    pub line10: Usd,
    pub line11: Usd,
}

/// Derive the 1040's printed income block (lines 1a–11).
pub fn form_1040_income_lines(
    ar: &AbsoluteReturn,
    sch_b: Option<&ScheduleBLines>,
    sch_1: Option<&Schedule1Lines>,
    sch_d: &ScheduleDLines,
) -> Form1040Income {
    // L1a is the Σ of W-2 box 1, and L1z is "Add lines 1a through 1h" — v1 has no 1b–1h, so they are
    // the same figure, and BOTH cells print. A filled 1z above a blank 1a does not add up.
    let line1a = round_dollar(ar.wages);
    let line1z = line1a;
    let line2a = round_dollar(ar.printed_inputs.tax_exempt_interest);
    let line2b = sch_b.map_or_else(|| round_dollar(ar.taxable_interest), |b| b.line4);
    let line3a = round_dollar(ar.qualified_dividends);
    let line3b = sch_b.map_or_else(|| round_dollar(ar.ordinary_dividends), |b| b.line6);

    // L7 — signed, with a LEADING MINUS (not parentheses; SPEC §3.2). On a net-loss year it is the
    // §1211(b)-LIMITED amount from Schedule D line 21, not the full loss.
    let line7 = match sch_d.routing {
        ScheduleDRouting::NetLoss { line21, .. } => -line21,
        _ => sch_d.line16,
    };
    let line8 = sch_1.map_or(Usd::ZERO, |s| s.line10);
    let line9 = line1z + line2b + line3b + line7 + line8; // ★ sums the PRINTED lines
    let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
    let line11 = line9 - line10;

    Form1040Income {
        // The worksheet reads these OFF THE FILED SCHEDULE D — printed lines 15 and 16, smaller of, ≥ 0.
        qdcgt_net_capital_gain: sch_d.line15.min(sch_d.line16).max(Usd::ZERO),
        line1a,
        line1z,
        line2a,
        line2b,
        line3a,
        line3b,
        line7,
        line8,
        line9,
        line10,
        line11,
    }
}

/// Derive the printed Form 1040 chain. Every schedule figure is taken from that schedule's **printed**
/// chain, so the return ties out against its own attachments.
///
/// `standard_or_itemized` is the deduction actually claimed (line 12): Schedule A's printed line 17 if
/// the return itemizes, else `round_dollar` of the §63 standard deduction.
#[allow(clippy::too_many_arguments)]
pub fn form_1040_lines(
    ar: &AbsoluteReturn,
    income: &Form1040Income,
    sch_a: Option<&ScheduleALines>,
    sch_2: Option<&Schedule2Lines>,
    sch_3: Option<&Schedule3Lines>,
    f8959: &Form8959Lines,
    f8995: Option<&crate::tax::qbi::Form8995Lines>,
    table: &crate::tax::tables::TaxTable,
    status: FilingStatus,
    other_withholding: Usd,
    estimated_payments: Usd,
    digital_asset_yes: bool,
) -> Form1040Lines {
    // ── Income — from the printed block (Schedule A already composed on its L11). ───────────────
    let Form1040Income {
        qdcgt_net_capital_gain,
        line1a,
        line1z,
        line2a,
        line2b,
        line3a,
        line3b,
        line7,
        line8,
        line9,
        line10,
        line11,
    } = *income;

    // ── Deductions → taxable income ─────────────────────────────────────────────────────────────
    let line12 = match sch_a {
        Some(a) => a.line17, // itemizing — Schedule A's PRINTED total
        None => round_dollar(ar.deduction),
    };
    let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
    let line14 = line12 + line13;
    let line15 = (line11 - line14).max(Usd::ZERO);

    // ── Tax → total tax ─────────────────────────────────────────────────────────────────────────
    // ★ L16 is the Tax Table / QDCGT worksheet applied to the PRINTED line 15 — NOT a re-rounding of the
    // tax computed on the exact-cents taxable income (Fable P6 r1 I2). The Tax Table is a STEP function
    // with $50 treads: when the exact TI and the printed L15 straddle a bin edge, the two differ by a
    // whole bin step (up to ~$18.50 at the top rate), not by the $1 rounding residual §3.1 tolerates —
    // and "L16 vs Table(L15)" is the single most-recomputed arithmetic on a transcribed return. The
    // exact-cents `ar.regular_tax` remains the COMPUTED liability; only the FILED cell changes.
    let line16 = crate::tax::method::qdcgt_line16(
        table.ordinary_for(status),
        table.ltcg_for(status),
        line15,
        line3a,
        qdcgt_net_capital_gain, // ★ the PRINTED Schedule D figure, not round(exact) — r2 NEW-I1
    );
    let line17 = Usd::ZERO; // Schedule 2 Part I is blank in v1 (see Schedule2Lines)
    let line18 = line16 + line17;
    let line19 = Usd::ZERO; // ★ CTC/ODC — a §3.4 conservative omission (advisory fires)
    let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
    let line21 = line19 + line20;
    let line22 = (line18 - line21).max(Usd::ZERO);
    let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
    let line24 = line22 + line23; // ★ TOTAL TAX, from the PRINTED lines

    // ── Payments ────────────────────────────────────────────────────────────────────────────────
    let line25a = round_dollar(ar.withholding_25a);
    let line25b = round_dollar(ar.withholding_25b);
    // 25c carries Form 8959's PRINTED line 24 — the Additional-Medicare over-withholding credit.
    let line25c = f8959.line24 + round_dollar(other_withholding);
    let line25d = line25a + line25b + line25c;
    let line26 = round_dollar(estimated_payments);
    let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
    let line32 = line31; // 27-30 blank (EIC omitted conservatively; rest unrepresentable)
    let line33 = line25d + line26 + line32; // ★ TOTAL PAYMENTS, from the PRINTED lines

    // ── Refund or owed — from the PRINTED total tax and the PRINTED total payments, so the bottom
    //    line of the filed form is the one a reader re-adding the column arrives at.
    let line34 = (line33 - line24).max(Usd::ZERO);
    let line37 = (line24 - line33).max(Usd::ZERO);

    Form1040Lines {
        line1a,
        line1z,
        line2a,
        line2b,
        line3a,
        line3b,
        line7,
        line8,
        line9,
        line10,
        line11,
        line12,
        line13,
        line14,
        line15,
        line16,
        line17,
        line18,
        line19,
        line20,
        line21,
        line22,
        line23,
        line24,
        line25a,
        line25b,
        line25c,
        line25d,
        line26,
        line31,
        line32,
        line33,
        line34,
        line37,
        digital_asset_yes,
    }
}

/// The **Schedule D Part III routing** (SPEC §7.2) — which of lines 17–22 get answered, and how.
///
/// The form's Part III is a decision tree, not a column of numbers, and the four branches are
/// mutually exclusive and exhaustive. Modelling it as an enum rather than a bag of `Option`s means an
/// impossible combination (say, line 17 = "Yes" together with a line-21 loss) cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleDRouting {
    /// **L16 > 0 and L15 > 0** — gains in both characters. L17 = **Yes**; L18 = L19 = 0 (the 28%-rate
    /// and unrecaptured-§1250 amounts, both refused upstream if they could ever be nonzero); L20 =
    /// **Yes** → the Qualified Dividends and Capital Gain Tax Worksheet. Lines 21 and 22 are NOT
    /// completed — the form says so in terms.
    BothGains,
    /// **L16 > 0 but L15 ≤ 0** — a short-term gain with a long-term loss. The common crypto year.
    /// L17 = **No** → skip 18 through 21 → L22. Tax routes to QDCGT iff there are qualified dividends.
    ShortGainLongLoss { line22_yes: bool },
    /// **L16 < 0** — a net capital loss. Skip 17–20; L21 carries the §1211(b) allowed offset (a
    /// POSITIVE MAGNITUDE — the form prints the parentheses); L22 is still answered.
    NetLoss {
        /// L21 — the allowed §1211(b) offset, ≤ $3,000 ($1,500 MFS). Positive magnitude.
        line21: Usd,
        line22_yes: bool,
    },
    /// **L16 = 0** — 1040 line 7 is `-0-`. Skip 17–21; L22 is still answered.
    Zero { line22_yes: bool },
}

/// The printable **Schedule D (Capital Gains and Losses)** line chain — the FULL return's Schedule D.
///
/// This is not the crypto-slice Schedule D that `export-irs-pdf` has always produced. That one fills
/// only lines 3/7/10/15/16 from the ledger totals — it has **no line 13** (1099-DIV box-2a
/// capital-gain distributions) and **no lines 6/14** (capital-loss carryovers), which is exactly why
/// the crypto-slice export REFUSES for a full-return year (P5-C1): those omissions make a
/// complete-looking Schedule D that understates income.
///
/// **★ Lines 6, 14 and 21 are PARENTHESIZED boxes — positive magnitudes only.** The form pre-prints
/// the parentheses, so they ARE the minus sign. A negative written there renders as a POSITIVE number
/// on a filed return.
///
/// Column (g) (adjustments from Form 8949) is left blank throughout: v1 models no basis adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleDLines {
    /// L3 (d) — short-term proceeds (Form 8949 Box C or **Box I**).
    pub line3_d: Usd,
    /// L3 (e) — short-term cost basis.
    pub line3_e: Usd,
    /// L3 (h) — short-term gain or loss (signed).
    pub line3_h: Usd,
    /// L6 — prior-year short-term capital loss carryover. **Positive magnitude** (paren box).
    pub line6: Usd,
    /// L7 — net short-term gain or loss (signed).
    pub line7: Usd,
    /// L10 (d) — long-term proceeds (Form 8949 Box F or **Box L**).
    pub line10_d: Usd,
    /// L10 (e) — long-term cost basis.
    pub line10_e: Usd,
    /// L10 (h) — long-term gain or loss (signed).
    pub line10_h: Usd,
    /// L13 — capital gain distributions (Σ 1099-DIV box 2a).
    pub line13: Usd,
    /// L14 — prior-year long-term capital loss carryover. **Positive magnitude** (paren box).
    pub line14: Usd,
    /// L15 — net long-term gain or loss (signed).
    pub line15: Usd,
    /// L16 — combine **printed** 7 and 15 (signed) → 1040 **L7** (as a signed figure; a loss is
    /// limited to line 21's amount).
    pub line16: Usd,
    /// Part III's routing — which of lines 17–22 are answered (SPEC §7.2).
    pub routing: ScheduleDRouting,
}

impl ScheduleDLines {
    /// Whether Schedule D must actually be FILED.
    ///
    /// The 1040's line 7 reads this chain either way (it is simply zero when there is no capital
    /// activity), but a return with **no** disposals, **no** capital-gain distributions and **no**
    /// carryover has no Schedule D to attach — and a blank one stapled to a W-2-only return is a form
    /// the filer did not need. Like `Form8959Lines::must_file`, the decision is a CORE fact, so the
    /// packet's KATs can see it (ARCH-P6 Q2).
    pub fn must_file(&self) -> bool {
        [
            self.line3_d,
            self.line3_e,
            self.line3_h,
            self.line6,
            self.line10_d,
            self.line10_e,
            self.line10_h,
            self.line13,
            self.line14,
            self.line16,
        ]
        .iter()
        .any(|v| *v != Usd::ZERO)
    }
}

/// Derive the printed Schedule D chain, including SPEC §7.2's exhaustive Part III routing.
pub fn schedule_d_lines(ar: &AbsoluteReturn, f8949: Option<&Printed8949>) -> ScheduleDLines {
    let p = &ar.schedule_d;

    // ★ Lines 3 and 10 ARE the attached Form 8949's printed column totals — the schedule's own text
    // defines them as "Totals for all transactions reported on Form(s) 8949 with Box C / Box F checked"
    // (ARCH-P6.3a D3). Re-rounding the exact aggregate here would put Schedule D a dollar away from the
    // 8949 stapled behind it: Σ round(row) ≠ round(Σ row). Zero when no 8949 is attached (a
    // carryover/distribution-only Schedule D has no transactions to total).
    let st = f8949.map(|f| f.st_totals).unwrap_or_default();
    let lt = f8949.map(|f| f.lt_totals).unwrap_or_default();

    let line3_d = st.proceeds_d;
    let line3_e = st.cost_e;
    let line3_h = st.gain_h;
    let line6 = round_dollar(p.st_carryover_6); // magnitude (paren box)
                                                // ★ "Combine lines 1a through 6 in column (h)" — an addition chain over the PRINTED cells.
    let line7 = line3_h - line6;

    let line10_d = lt.proceeds_d;
    let line10_e = lt.cost_e;
    let line10_h = lt.gain_h;
    let line13 = round_dollar(p.cap_gain_distr_13);
    let line14 = round_dollar(p.lt_carryover_14); // magnitude (paren box)
                                                  // ★ "Combine lines 8a through 14 in column (h)" — again over the PRINTED cells.
    let line15 = line10_h + line13 - line14;

    let line16 = line7 + line15; // ★ combines the PRINTED lines
    let has_qd = round_dollar(p.qualified_dividends) > Usd::ZERO;

    // ★ SPEC §7.2 — exhaustive, and the four branches are mutually exclusive by construction.
    let routing = if line16 > Usd::ZERO && line15 > Usd::ZERO {
        ScheduleDRouting::BothGains
    } else if line16 > Usd::ZERO {
        // …and line 15 ≤ 0: a short-term gain against a long-term loss.
        ScheduleDRouting::ShortGainLongLoss { line22_yes: has_qd }
    } else if line16 < Usd::ZERO {
        ScheduleDRouting::NetLoss {
            // §1211(b): the smaller of the loss and the ceiling — on the PRINTED line 16 (magnitude,
            // paren box), so the filed form's own "smaller of" holds.
            line21: line16.abs().min(ar.printed_inputs.capital_loss_limit),
            line22_yes: has_qd,
        }
    } else {
        ScheduleDRouting::Zero { line22_yes: has_qd }
    };

    ScheduleDLines {
        line3_d,
        line3_e,
        line3_h,
        line6,
        line7,
        line10_d,
        line10_e,
        line10_h,
        line13,
        line14,
        line15,
        line16,
        routing,
    }
}

/// One listed payer row on Schedule B — the payer's name and the amount they paid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleBRow {
    /// The payer's name, as entered (Schedule B is a *list*: the IRS wants to see who paid).
    pub payer: String,
    /// The amount, rounded at the line.
    pub amount: Usd,
}

/// The printable **Schedule B (Interest and Ordinary Dividends)** line chain.
///
/// Schedule B is a **listing** schedule: Part I names every interest payer and Part II every dividend
/// payer. The totals sum the PRINTED row amounts, so the form cross-foots against its own list.
///
/// **Part III is TRANSCRIBED, never decided.** Lines 7a (a financial interest in a foreign account)
/// and 8 (a distribution from a foreign trust) carry the filer's OWN declared answers — `screen_inputs`
/// REFUSES the return if they were left unanswered, precisely so that btctax never has to guess. The
/// Line **7b's country list** is the filer's own too — it IS captured (`ReturnInputs
/// .foreign_country_names`), and the claim that "v1 has no input for it" was simply FALSE: the input
/// existed, was screened, and was then dropped on the floor (ARCH-P6.3a Q7 item 7). It now prints.
///
/// Only the unnumbered FBAR sub-question under 7a is left BLANK — for that one there genuinely is no
/// input, and the `FbarFinCen` advisory tells the filer in terms that they must decide it themselves.
/// An incomplete Part III is the honest output there; a guessed one would not be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleBLines {
    /// L1 — the listed interest payers (Part I).
    pub part1_rows: Vec<ScheduleBRow>,
    /// L2 — add the amounts on **printed** line 1.
    pub line2: Usd,
    /// L4 — line 2 − line 3 (Form 8815 excludable savings-bond interest, unmodeled ⇒ blank)
    /// ⇒ `= line2` → 1040 **L2b**.
    pub line4: Usd,
    /// L5 — the listed dividend payers (Part II).
    pub part2_rows: Vec<ScheduleBRow>,
    /// L6 — add the amounts on **printed** line 5 → 1040 **L3b**.
    pub line6: Usd,
    /// L7a — "did you have a financial interest in… a foreign country?" — the filer's own answer.
    pub foreign_accounts_7a: bool,
    /// L8 — "did you receive a distribution from… a foreign trust?" — the filer's own answer.
    pub foreign_trust_8: bool,
    /// L7b — the foreign-country list. The filer's own words, printed verbatim when 7a is "Yes".
    pub line7b_countries: String,
}

/// Derive the printed Schedule B chain from the filer's 1099s. Returns `None` when Schedule B is not
/// required ([`crate::tax::return_1040::schedule_b_files`] — interest or dividends over $1,500, or a
/// declared foreign account).
///
/// A payer with nothing to report is not listed: a zero row would name someone on a federal form for
/// no reason.
///
/// # Panics
/// Never. Part III's answers are `Option<bool>` on the inputs, but `screen_inputs` refuses the return
/// when either is unanswered, so by the time a return is assembled they are both known; `unwrap_or`
/// defaults defensively to `false` rather than panicking on a caller that skipped the screen.
pub fn schedule_b_lines(ri: &crate::tax::return_inputs::ReturnInputs) -> Option<ScheduleBLines> {
    if !crate::tax::return_1040::schedule_b_files(ri) {
        return None;
    }

    // Part I — every 1099-INT that paid taxable interest (box 1 + box 3 treasury).
    let part1_rows: Vec<ScheduleBRow> = ri
        .int_1099
        .iter()
        .map(|i| ScheduleBRow {
            payer: i.payer.clone(),
            amount: round_dollar(i.box1_interest + i.box3_treasury_interest),
        })
        .filter(|r| r.amount > Usd::ZERO)
        .collect();
    let line2 = part1_rows.iter().map(|r| r.amount).sum(); // ★ sums the PRINTED rows
    let line4 = line2; // − line 3 (Form 8815), unmodeled ⇒ blank

    // Part II — every 1099-DIV that paid ordinary dividends (box 1a, which INCLUDES the qualified 1b).
    let part2_rows: Vec<ScheduleBRow> = ri
        .div_1099
        .iter()
        .map(|d| ScheduleBRow {
            payer: d.payer.clone(),
            amount: round_dollar(d.box1a_ordinary),
        })
        .filter(|r| r.amount > Usd::ZERO)
        .collect();
    let line6 = part2_rows.iter().map(|r| r.amount).sum(); // ★ sums the PRINTED rows

    Some(ScheduleBLines {
        part1_rows,
        line2,
        line4,
        part2_rows,
        line6,
        foreign_accounts_7a: ri.foreign_accounts.unwrap_or(false),
        // Printed only when 7a is "Yes" — a country list beside a "No" would contradict the answer.
        line7b_countries: if ri.foreign_accounts == Some(true) {
            ri.foreign_country_names.clone()
        } else {
            String::new()
        },
        foreign_trust_8: ri.foreign_trust.unwrap_or(false),
    })
}

/// The printable **Schedule C (Profit or Loss From Business)** line chain — the crypto trade or
/// business.
///
/// **Part II is NOT itemized.** The filer supplies a flat expense total, so the individual expense
/// lines (8 through 27a) are BLANK and only the **line 28 total** is printed. Printing a 0 into each
/// of those twenty lines would assert we considered and found no advertising, no insurance, no
/// legal fees — which is a different claim from "the filer gave us one number".
///
/// Also blank, and deliberately: line 2 (returns and allowances), line 4 and Part III (cost of goods
/// sold — mining and staking have no inventory), line 6 (other income), line 30 (the §280A home-office
/// deduction, out of scope), and Part IV (vehicle information).
///
/// **A Schedule C LOSS is refused upstream** (§465 at-risk substantiation is out of scope), so line 31
/// is always ≥ 0 and the at-risk checkboxes on lines 32a/32b are never needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleCLines {
    /// Line **A** — "Principal business or profession". Captured (`ScheduleCInputs.business_description`)
    /// expressly for this cell; a Schedule C with a blank line A is incomplete on its face (Q7 item 6).
    pub line_a_business: String,
    /// Line **B** — the NAICS code.
    pub line_b_naics: String,
    /// Line **F** — the accounting method (a checkbox: Cash or Accrual).
    pub line_f_accrual: bool,
    /// L1 — gross receipts or sales.
    pub line1: Usd,
    /// L3 — subtract line 2 (returns/allowances, blank) from line 1 ⇒ `= line1`.
    pub line3: Usd,
    /// L5 — gross profit = line 3 − line 4 (cost of goods sold, blank) ⇒ `= line3`.
    pub line5: Usd,
    /// L7 — gross income = line 5 + line 6 (other income, blank) ⇒ `= line5`.
    pub line7: Usd,
    /// L28 — total expenses (the flat total; Part II's individual lines are blank).
    pub line28: Usd,
    /// L29 — tentative profit = **printed** line 7 − **printed** line 28.
    pub line29: Usd,
    /// L31 — net profit = line 29 − line 30 (home office, blank) ⇒ `= line29`. Flows to **both**
    /// Schedule 1 line 3 and Schedule SE — one figure, two destinations.
    pub line31: Usd,
}

/// Derive the printed Schedule C chain. Returns `None` when the filer has no crypto trade or business.
pub fn schedule_c_lines(ar: &AbsoluteReturn) -> Option<ScheduleCLines> {
    let p = ar.schedule_c.as_ref()?;
    let h = &ar.printed_inputs.schedule_c_header;

    let line1 = round_dollar(p.gross_receipts_1);
    let line3 = line1; // − line 2 (returns and allowances), blank
    let line5 = line3; // − line 4 (cost of goods sold), blank — no inventory
    let line7 = line5; // + line 6 (other income), blank
    let line28 = round_dollar(p.total_expenses_28);
    let line29 = (line7 - line28).max(Usd::ZERO); // a loss refuses upstream
    let line31 = line29; // − line 30 (home office), blank

    Some(ScheduleCLines {
        line_a_business: h.business_description.clone(),
        line_b_naics: h.naics_code.clone(),
        line_f_accrual: h.accrual,
        line1,
        line3,
        line5,
        line7,
        line28,
        line29,
        line31,
    })
}

/// The printable **Schedule A (Itemized Deductions)** line chain.
///
/// **Every derived line is computed from the PRINTED line above it**, not from the exact-cents
/// components: line 3 is 7.5% of the *printed* line 2, line 4 subtracts the *printed* line 3, line 5e
/// caps the *printed* line 5d, and line 17 sums the *printed* subtotals. That is what a human filling
/// the paper form does, and it is why the form cross-foots.
///
/// **Unmodeled lines are BLANK, not zero** (no field here at all): line 6 (other taxes), line 8b
/// (mortgage interest not on a Form 1098) and 8c (points), line 9 (investment interest), line 15
/// (casualty and theft losses) and line 16 (other itemized deductions). Line 8d is the IRS's own
/// "Reserved for future use" — a ReadOnly widget that must never be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleALines {
    /// L5a's **checkbox** — §164(b)(5): "If you elect to include general sales taxes instead of income
    /// taxes, check this box". The election is already in the arithmetic; without the box the filed form
    /// claims income taxes the filer did not use (ARCH-P6.3a Q7 item 3).
    pub line5a_is_sales_tax: bool,
    /// L18's **checkbox** — §63(e): "If you elect to itemize deductions even though they are less than
    /// your standard deduction, check this box". Without it the Service's math-error unit may "correct"
    /// the return back to the standard deduction (Q7 item 4).
    pub line18_elects_smaller: bool,
    /// ★ §2.7 — L8's **checkbox**: "If you didn't use all of your home mortgage loan(s) to buy, build, or
    /// improve your home, check this box." Set from [`ScheduleAParts::mortgage_mixed_use_box`]; when set,
    /// line 8a above prints $0 (v1 cannot do the Pub. 936 allocation — the `MixedUseMortgageNotAllocated`
    /// advisory names the forgone interest).
    pub line8_mixed_use_box: bool,
    /// L1 — medical and dental expenses.
    pub line1: Usd,
    /// L2 — AGI (the floor's base).
    pub line2: Usd,
    /// L3 — the §213(a) floor: 7.5% × **printed** line 2.
    pub line3: Usd,
    /// L4 — medical allowed = max(0, printed 1 − printed 3).
    pub line4: Usd,
    /// L5a — state/local income taxes, or general sales taxes under the §164(b)(5) election.
    pub line5a: Usd,
    /// L5b — state/local real-estate taxes.
    pub line5b: Usd,
    /// L5c — state/local personal-property taxes.
    pub line5c: Usd,
    /// L5d — add **printed** 5a, 5b and 5c.
    pub line5d: Usd,
    /// L5e — the §164(b) cap: min(printed 5d, $10,000 / $5,000 MFS).
    pub line5e: Usd,
    /// L7 — add 5e and 6 (6 blank) ⇒ `= line5e`.
    pub line7: Usd,
    /// L8a — home-mortgage interest reported on Form 1098.
    pub line8a: Usd,
    /// L8e — add 8a through 8c (8b/8c blank) ⇒ `= line8a`.
    pub line8e: Usd,
    /// L10 — add 8e and 9 (9 blank) ⇒ `= line8e`.
    pub line10: Usd,
    /// L11 — gifts by cash or check.
    pub line11: Usd,
    /// L12 — gifts other than by cash or check (includes crypto donations; Form 8283 over $500).
    pub line12: Usd,
    /// L13 — carryover from a prior year.
    pub line13: Usd,
    /// L14 — add **printed** 11, 12 and 13.
    pub line14: Usd,
    /// L17 — total itemized deductions = printed 4 + 7 + 10 + 14 (15 and 16 blank) → 1040 **L12**.
    pub line17: Usd,
}

/// Derive the printed Schedule A chain.
///
/// Returns `None` unless the return actually **itemizes** — Schedule A is computed even when the
/// standard deduction wins (that is how the `max()` is taken), but it is only *filed* when it is the
/// deduction actually claimed.
///
/// Note the printed line 17 can differ by a dollar from `round_dollar(itemized_deduction)`: it sums
/// the printed subtotals, each already rounded at its own line. That is the SPEC §3.1 election, and
/// the printed figure is the one that appears on 1040 line 12.
pub fn schedule_a_lines(ar: &AbsoluteReturn, line11_1040: Usd) -> Option<ScheduleALines> {
    if !ar.deduction_is_itemized {
        return None;
    }
    let p = ar.schedule_a.as_ref()?;

    // ★ L2's own text is "Enter amount from Form 1040 or 1040-SR, **line 11**" — a SOURCE citation, so it
    // takes the PRINTED L11 (SPEC §3.1's citation-composition rule). A re-rounding of the exact AGI can
    // differ by dollars, and on a negative-AGI itemizer it printed 0 beside a NEGATIVE 1040 L11: two
    // cells visibly disagreeing, with the divergence propagating into the 7.5% floor and on to L17 →
    // 1040 L12 (Fable P6 r1 I5).
    let line1 = round_dollar(p.medical_expenses);
    let line2 = line11_1040;
    // ★ The floor is CLAMPED at zero (Fable P6 r2 NEW-I2). L2 carries the true printed 1040 L11 — which
    // can be NEGATIVE — but a negative 7.5% floor would INFLATE the deduction: line 4 would print
    // $13,750 of allowed medical on $10,000 actually paid. §213(a) allows the expenses "to the extent
    // that [they] exceed 7.5 percent" of AGI — the deduction can never exceed the expense. The exact
    // engine has always clamped here (`schedule_a_parts`: "so the floor never helps the taxpayer"); the
    // printed chain must too, or the FILED form claims more than the return computed.
    let line3 = round_dollar(MEDICAL_FLOOR_RATE * line2).max(Usd::ZERO);
    let line4 = (line1 - line3).max(Usd::ZERO);

    // SALT — the cap binds the PRINTED 5d.
    let line5a = round_dollar(p.salt_5a);
    let line5b = round_dollar(p.salt_5b);
    let line5c = round_dollar(p.salt_5c);
    let line5d = line5a + line5b + line5c;
    let line5e = line5d.min(p.salt_cap);
    let line7 = line5e; // + line 6 (other taxes), unmodeled ⇒ blank

    // Interest.
    let line8a = round_dollar(p.mortgage_8a);
    let line8e = line8a; // + 8b/8c, unmodeled ⇒ blank
    let line10 = line8e; // + line 9 (investment interest), unmodeled ⇒ blank

    // Charitable — the §170(b)-limited classes are already Schedule A's own lines 11/12/13.
    let line11 = round_dollar(p.charitable_cash_11);
    let line12 = round_dollar(p.charitable_noncash_12);
    let line13 = round_dollar(p.charitable_carryover_13);
    let line14 = line11 + line12 + line13;

    // ★ The total sums the PRINTED subtotals (15 and 16 are blank).
    let line17 = line4 + line7 + line10 + line14;

    Some(ScheduleALines {
        line5a_is_sales_tax: p.salt_is_sales_tax,
        // §63(e) is only an ELECTION when the itemized total is actually SMALLER than the standard
        // deduction — otherwise itemizing simply won, and the box would misstate what the filer did.
        line18_elects_smaller: ar
            .itemized_deduction
            .is_some_and(|it| it < ar.standard_deduction),
        line8_mixed_use_box: p.mortgage_mixed_use_box,
        line1,
        line2,
        line3,
        line4,
        line5a,
        line5b,
        line5c,
        line5d,
        line5e,
        line7,
        line8a,
        line8e,
        line10,
        line11,
        line12,
        line13,
        line14,
        line17,
    })
}

/// The printable **Schedule 3 (Additional Credits and Payments)** line chain.
///
/// Part I carries only the **foreign tax credit** (line 1). Every other nonrefundable credit on the
/// schedule — education, dependent-care, saver's, residential-energy, adoption — is a §3.4
/// *conservative omission*: v1 does not compute it, which can only OVERSTATE tax, and the report
/// fires a loud advisory ([`crate::tax::advisories::Advisory::OtherCreditsOmitted`]). They are left
/// BLANK, never a misleading 0.
///
/// Part II carries only the **§6413(c) excess Social Security** credit (line 11), computed per person
/// and never pooled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule3Lines {
    /// L1 — foreign tax credit (the §904(j) de-minimis election; above the ceiling the return refuses).
    pub line1: Usd,
    /// L8 — total nonrefundable credits = add 1 through 4, 5a, 5b and 7 ⇒ `= line1` → 1040 **L20**.
    pub line8: Usd,
    /// L10 — "Amount paid with request for extension to file". ★ Dropping this line told a filer who had
    /// ALREADY paid with their extension to pay it again (Fable ARCH-P6.3a D1): it is in the exact
    /// `total_payments`, so omitting it from the printed chain lowers L31/L33 and RAISES L37 "amount you
    /// owe" by exactly the amount already paid.
    pub line10: Usd,
    /// L11 — §6413(c) excess Social Security and tier-1 RRTA tax withheld.
    pub line11: Usd,
    /// L15 — total other payments and refundable credits = "Add lines 9 **through 12** and 14" ⇒
    /// `line10 + line11` here (9, 12 and 14 are blank) → 1040 **L31**. Sums the PRINTED lines (§3.1).
    pub line15: Usd,
}

/// Derive the printed Schedule 3 chain. Returns `None` when there is neither a foreign tax credit nor
/// an excess-Social-Security credit — the schedule is then not filed.
pub fn schedule_3_lines(ar: &AbsoluteReturn) -> Option<Schedule3Lines> {
    let line1 = round_dollar(ar.foreign_tax_credit);
    let line8 = line1; // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
    let line10 = round_dollar(ar.printed_inputs.extension_payment);
    let line11 = round_dollar(ar.excess_social_security);
    let line15 = line10 + line11; // "Add lines 9 through 12 and 14"; 9, 12 and 14 are blank

    if line8 <= Usd::ZERO && line15 <= Usd::ZERO {
        return None;
    }
    Some(Schedule3Lines {
        line1,
        line8,
        line10,
        line11,
        line15,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::{Form8949Box, Form8949Part, Form8949Row};
    use crate::tax::other_taxes::form_8959_lines;
    use crate::tax::se::SeTaxResult;
    use crate::tax::types::FilingStatus;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn se_300k_single() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(300000),
            base: dec!(277050.00),
            ss: dec!(21836.40),
            medicare: dec!(8034.45),
            addl: dec!(693.45),
            total: dec!(30564.30),
            deductible_half: dec!(14935.42),
        }
    }

    /// An `AbsoluteReturn` carrying only what the Schedule 2/3 chains read; everything else zero.
    ///
    /// Spelled out in full rather than `..Default::default()` on purpose — `AbsoluteReturn`
    /// deliberately has no `Default`, because a silently-zeroed field on a real tax return is
    /// exactly the class of bug this codebase fails closed against.
    fn ar_with(se: Option<SeTaxResult>, ftc: Usd, excess_ss: Usd) -> AbsoluteReturn {
        use crate::tax::other_taxes::{Form8959, Form8960};
        let z = Usd::ZERO;
        AbsoluteReturn {
            wages: z,
            taxable_interest: z,
            ordinary_dividends: z,
            qualified_dividends: z,
            capital_gain: z,
            schedule_1_income: z,
            total_income: z,
            adjustments: z,
            half_se_deduction: z,
            agi: z,
            se,
            schedule_1: crate::tax::return_1040::Schedule1Parts {
                state_refund_1: z,
                schedule_c_net_3: z,
                unemployment_7: z,
                crypto_ordinary_8v: z,
                half_se_15: z,
                early_withdrawal_18: z,
                student_loan_21: z,
            },
            schedule_c: None,
            schedule_d: crate::tax::return_1040::ScheduleDParts {
                st_proceeds_3d: z,
                st_cost_3e: z,
                st_gain_3h: z,
                st_carryover_6: z,
                st_net_7: z,
                lt_proceeds_10d: z,
                lt_cost_10e: z,
                lt_gain_10h: z,
                cap_gain_distr_13: z,
                lt_carryover_14: z,
                lt_net_15: z,
                total_16: z,
                loss_deduction_21: z,
                qualified_dividends: z,
            },
            standard_deduction: z,
            itemized_deduction: None,
            schedule_a: None,
            deduction: z,
            deduction_is_itemized: false,
            qbi_deduction: z,
            total_deductions: z,
            taxable_income: z,
            net_ltcg: z,
            charitable_carryover_out: Vec::new(),
            qbi_reit_ptp_carryforward_out: z,
            regular_tax: z,
            se_tax_sch2_l4: z,
            schedule_2_other_taxes: z,
            additional_medicare: Form8959 {
                part1_wages: z,
                part2_se: z,
                additional_medicare_tax: z,
                part5_withholding: z,
            },
            niit: Form8960 {
                nii: z,
                magi: z,
                tax: z,
            },
            foreign_tax_credit: ftc,
            ctc_odc_credit: z,
            tax_after_credits: z,
            total_tax: z,
            excess_social_security: excess_ss,
            withholding_25a: z,
            withholding_25b: z,
            total_withholding: z,
            total_payments: z,
            overpayment_refund: z,
            amount_owed: z,
            // Spelled out in full — `PrintedInputs` has no `Default` on purpose (a zeroed
            // `capital_loss_limit` would silently disable the §1211(b) deduction).
            printed_inputs: crate::tax::return_1040::PrintedInputs {
                medicare_wages: z,
                medicare_withheld: z,
                schedule_c_header: crate::tax::return_1040::ScheduleCHeader::default(),
                business_qbi: z,
                tax_exempt_interest: z,
                crypto_lending_interest: z,
                reit_dividends: z,
                reit_ptp_carryforward_in: z,
                ti_before_qbi: z,
                qbi_net_capital_gain: z,
                se_w2_ss_wages: z,
                ss_wage_base: dec!(168600),     // TY2024 §230 wage base
                capital_loss_limit: dec!(3000), // §1211(b) — the non-MFS ceiling
                extension_payment: z,
                digital_asset_activity: false,
            },
        }
    }

    /// The TY2024 table the printed chain needs for line 16 (the Tax Table / QDCGT worksheet).
    fn tt() -> crate::tax::tables::TaxTable {
        crate::tax::testonly::ty2024_table()
    }

    // ── The ARCH-P6.3a Q7 sweep: captured inputs that reached the ARITHMETIC but never the FORM ──

    /// A Schedule A whose SALT line 5a is the §164(b)(5) SALES-TAX election.
    fn sched_a_parts_sales_tax() -> crate::tax::return_1040::ScheduleAParts {
        crate::tax::return_1040::ScheduleAParts {
            salt_is_sales_tax: true,
            medical_expenses: Usd::ZERO,
            agi: dec!(50000),
            medical_floor: dec!(3750),
            medical_allowed: Usd::ZERO,
            salt_5a: dec!(4000),
            salt_5b: Usd::ZERO,
            salt_5c: Usd::ZERO,
            salt_5d: dec!(4000),
            salt_cap: dec!(10000),
            salt_5e: dec!(4000),
            mortgage_8a: dec!(5000),
            mortgage_mixed_use_box: false,
            charitable_cash_11: Usd::ZERO,
            charitable_noncash_12: Usd::ZERO,
            charitable_carryover_13: Usd::ZERO,
            charitable_14: Usd::ZERO,
            total_17: dec!(9000),
        }
    }

    /// 1040 **line 2a** — tax-exempt interest (Σ 1099-INT box 8 + 1099-DIV box 12). It changes no tax,
    /// but the IRS document-matches box 8, and SPEC §5 stage 1 puts it on the return. It reached
    /// NOTHING: `Form1040Lines` had no line 2a (Q7 item 2).
    #[test]
    fn form_1040_line2a_carries_tax_exempt_interest() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.printed_inputs.tax_exempt_interest = dec!(1234.49);
        let sd = schedule_d_lines(&ar, None);
        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, None);

        let income = form_1040_income_lines(&ar, None, None, &sd);
        let l = form_1040_lines(
            &ar,
            &income,
            None,
            None,
            None,
            &f8959,
            None,
            &tt(),
            FilingStatus::Single,
            Usd::ZERO,
            Usd::ZERO,
            false,
        );
        assert_eq!(
            l.line2a,
            dec!(1234),
            "rounded at the line, like every printed cell"
        );
    }

    /// Schedule A **5a's checkbox** — "If you elect to include general sales taxes instead of income
    /// taxes, check this box". Core honours the §164(b)(5) election in the ARITHMETIC; without the box a
    /// sales-tax-electing return files claiming INCOME taxes (Q7 item 3).
    ///
    /// And Schedule A **line 18** — "If you elect to itemize deductions even though they are less than
    /// your standard deduction, check this box" (§63(e)). Without it the Service's math-error unit may
    /// "correct" the return back to the standard deduction (Q7 item 4).
    #[test]
    fn schedule_a_prints_the_sales_tax_and_force_itemize_elections_it_already_honours() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.deduction_is_itemized = true;
        ar.standard_deduction = dec!(14600);
        ar.itemized_deduction = Some(dec!(9000)); // SMALLER than the standard ⇒ §63(e) was elected
        ar.schedule_a = Some(sched_a_parts_sales_tax());

        let a = schedule_a_lines(&ar, round_dollar(ar.agi)).expect("the return itemizes");
        assert!(
            a.line5a_is_sales_tax,
            "the §164(b)(5) election must PRINT, not just compute"
        );
        assert!(
            a.line18_elects_smaller,
            "§63(e): itemizing BELOW the standard deduction must be declared on the form"
        );
    }

    /// …and neither box is checked on an ordinary return that simply itemizes because itemizing wins.
    #[test]
    fn schedule_a_leaves_both_election_boxes_unchecked_when_no_election_was_made() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.deduction_is_itemized = true;
        ar.standard_deduction = dec!(14600);
        ar.itemized_deduction = Some(dec!(20000)); // itemizing simply WINS — no §63(e) election
        let mut parts = sched_a_parts_sales_tax();
        parts.salt_is_sales_tax = false;
        ar.schedule_a = Some(parts);

        let a = schedule_a_lines(&ar, round_dollar(ar.agi)).unwrap();
        assert!(!a.line5a_is_sales_tax);
        assert!(!a.line18_elects_smaller);
    }

    /// ★ P9 §2.7 — the Schedule A **line-8 checkbox** ("didn't use all of your mortgage to buy/build/improve
    /// your home") propagates from the parts into the printed chain, and a mixed-use mortgage prints 8a as $0.
    /// (`schedule_a_lines` only returns Some on an itemizing return; a standard-wins mixed-use filer files no
    /// Schedule A — the advisory's "standard" branch, not this box, covers that case.)
    #[test]
    fn schedule_a_line8_mixed_use_box_prints_with_zeroed_8a() {
        let mut p = sched_a_parts_sales_tax();
        p.salt_is_sales_tax = false;
        // The filer declared a mixed-use mortgage: `schedule_a_parts` has already zeroed 8a and set the box.
        p.mortgage_8a = Usd::ZERO;
        p.mortgage_mixed_use_box = true;
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.deduction_is_itemized = true;
        ar.itemized_deduction = Some(dec!(20000));
        ar.schedule_a = Some(p);

        let a = schedule_a_lines(&ar, round_dollar(ar.agi)).unwrap();
        assert!(a.line8_mixed_use_box, "the line-8 mixed-use box must print");
        assert_eq!(a.line8a, Usd::ZERO, "8a printed as $0");
        assert_eq!(a.line8e, Usd::ZERO, "8e = 8a");
    }

    /// An acquisition-only mortgage leaves the line-8 box unchecked and prints full 8a.
    #[test]
    fn schedule_a_line8_box_off_for_acquisition_only_mortgage() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.deduction_is_itemized = true;
        ar.itemized_deduction = Some(dec!(20000));
        ar.schedule_a = Some(sched_a_parts_sales_tax()); // mortgage_8a = 5000, box false
        let a = schedule_a_lines(&ar, round_dollar(ar.agi)).unwrap();
        assert!(!a.line8_mixed_use_box);
        assert_eq!(a.line8a, dec!(5000));
    }

    // ── Schedule SE printed chain (ARCH-P6.3a D5) ───────────────────────────────────────────────

    fn se_result(ss: Usd, medicare: Usd, base: Usd, half: Usd) -> SeTaxResult {
        SeTaxResult {
            net_se: base,
            base,
            ss,
            medicare,
            addl: Usd::ZERO,
            total: ss + medicare,
            deductible_half: half,
        }
    }

    /// ★ Schedule SE line 12 is "Add lines 10 and 11" — it sums the PRINTED lines, so it can differ by a
    /// dollar from a re-rounding of the exact SE tax (KAT-9 class). And Schedule 2 line 4 IS that printed
    /// line 12: the form says so in terms ("Enter here and on Schedule 2 (Form 1040), line 4"), so a
    /// re-rounded Sch 2 L4 would disagree with the Schedule SE stapled behind it (ARCH-P6.3a D3).
    #[test]
    fn schedule_2_line4_takes_the_printed_se_line_12_not_the_rounded_total() {
        // ss = 10.50, medicare = 11.50 ⇒ printed 11 + 12 = 23, but round(exact 22.00) = 22.
        let se = se_result(dec!(10.50), dec!(11.50), dec!(1000), dec!(11.00));
        let mut ar = ar_with(Some(se), Usd::ZERO, Usd::ZERO);
        ar.schedule_c = Some(crate::tax::return_1040::ScheduleCParts {
            gross_receipts_1: dec!(1000),
            total_expenses_28: Usd::ZERO,
            net_profit_31: dec!(1000),
        });

        let sch_c = schedule_c_lines(&ar).expect("there is a business");
        let sse = schedule_se_lines(&ar, &sch_c).expect("there is SE tax");

        assert_eq!(sse.line10, dec!(11), "ss rounds at the line");
        assert_eq!(sse.line11, dec!(12), "medicare rounds at the line");
        assert_eq!(sse.line12, dec!(23), "L12 = add the PRINTED 10 and 11");
        assert_ne!(
            sse.line12,
            round_dollar(dec!(22.00)),
            "…and is deliberately NOT a re-rounding of the exact SE tax"
        );

        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, Some(&se));
        let s2 = schedule_2_lines(Some(&sse), &f8959, None).expect("SE tax ⇒ Schedule 2 files");
        assert_eq!(
            s2.line4, sse.line12,
            "★ Schedule 2 L4 IS Schedule SE's printed L12"
        );
    }

    /// The citation chain into and out of Schedule SE: its line 2 is Schedule C's printed line 31 ("Net
    /// profit or (loss) from Schedule C, line 31"), and its line 13 lands on Schedule 1 line 15
    /// ("Enter here and on Schedule 1 (Form 1040), line 15"). Form 8959 line 8 cites SE line 6.
    #[test]
    fn schedule_se_composes_with_schedule_c_schedule_1_and_form_8959() {
        let se = se_result(dec!(1240), dec!(290), dec!(9235), dec!(765));
        let mut ar = ar_with(Some(se), Usd::ZERO, Usd::ZERO);
        ar.schedule_c = Some(crate::tax::return_1040::ScheduleCParts {
            gross_receipts_1: dec!(11000),
            total_expenses_28: dec!(1000),
            net_profit_31: dec!(10000),
        });
        ar.half_se_deduction = dec!(765);
        ar.schedule_1.half_se_15 = dec!(765);

        let sch_c = schedule_c_lines(&ar).unwrap();
        let sse = schedule_se_lines(&ar, &sch_c).unwrap();
        assert_eq!(sse.line2, sch_c.line31, "SE L2 ← Schedule C's PRINTED L31");
        assert_eq!(sse.line3, sse.line2);
        assert_eq!(
            sse.line6,
            dec!(9235),
            "L6 = the §1402(a) base, rounded at the line"
        );

        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, Some(&se));
        assert_eq!(
            f8959.line8, sse.line6,
            "Form 8959 L8 cites Schedule SE Part I line 6"
        );

        let sch_1 = schedule_1_lines(&ar).unwrap();
        assert_eq!(
            sch_1.line15, sse.line13,
            "Sch 1 L15 ← Schedule SE's printed L13"
        );
    }

    /// §6017: no SE tax ⇒ NO Schedule SE is filed (and Schedule 2 line 4 is zero).
    #[test]
    fn no_se_tax_files_no_schedule_se() {
        let ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        let sch_c = ScheduleCLines {
            line_a_business: String::new(),
            line_b_naics: String::new(),
            line_f_accrual: false,
            line1: Usd::ZERO,
            line3: Usd::ZERO,
            line5: Usd::ZERO,
            line7: Usd::ZERO,
            line28: Usd::ZERO,
            line29: Usd::ZERO,
            line31: Usd::ZERO,
        };
        assert!(schedule_se_lines(&ar, &sch_c).is_none());
    }

    // ── Fable P6 gate review r1 — the folded findings ───────────────────────────────────────────

    /// ★ **I2 (borders Critical).** The filed line 16 is the Tax Table applied to the **printed line 15**,
    /// not a re-rounding of the tax computed on the exact-cents taxable income.
    ///
    /// The Tax Table is a STEP function with $50 treads. Fable's discriminating fixture: wages
    /// $61,749.80, Single, standard deduction. The exact TI is 47,149.80 — bin [47,100–47,150). The
    /// PRINTED L15 is 61,750 − 14,600 = **47,150**, which is the NEXT bin. A filer (or the Service)
    /// recomputing Table(printed L15) lands a whole bin step away from Table(exact TI) — up to ~$18.50 at
    /// the top rate, not the $1 residual §3.1 tolerates. "L16 vs Table(L15)" is the single most-recomputed
    /// arithmetic on a transcribed return, and a mismatch is a math-error notice.
    #[test]
    fn form_1040_line16_is_the_tax_on_the_printed_line15_not_on_the_exact_taxable_income() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.wages = dec!(61749.80);
        ar.agi = dec!(61749.80);
        ar.deduction = dec!(14600);
        ar.total_deductions = dec!(14600);
        ar.taxable_income = dec!(47149.80); // the EXACT TI — a different $50 bin from the printed L15
        ar.regular_tax = crate::tax::method::qdcgt_line16(
            tt().ordinary_for(FilingStatus::Single),
            tt().ltcg_for(FilingStatus::Single),
            dec!(47149.80),
            Usd::ZERO,
            Usd::ZERO,
        );

        let sd = schedule_d_lines(&ar, None);
        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, None);
        let income = form_1040_income_lines(&ar, None, None, &sd);
        let l = form_1040_lines(
            &ar,
            &income,
            None,
            None,
            None,
            &f8959,
            None,
            &tt(),
            FilingStatus::Single,
            Usd::ZERO,
            Usd::ZERO,
            false,
        );

        assert_eq!(l.line15, dec!(47150), "the PRINTED L15 (61,750 − 14,600)");
        let table_on_printed = crate::tax::method::qdcgt_line16(
            tt().ordinary_for(FilingStatus::Single),
            tt().ltcg_for(FilingStatus::Single),
            dec!(47150),
            Usd::ZERO,
            Usd::ZERO,
        );
        assert_eq!(
            l.line16, table_on_printed,
            "★ the filed L16 must be what a human gets by looking up the FILED L15"
        );
        assert_ne!(
            l.line16,
            round_dollar(ar.regular_tax),
            "…and it is NOT the tax on the exact-cents TI — the two bins differ"
        );
    }

    /// ★ **r2 NEW-I2 (a REGRESSION the r1 fold introduced).** Schedule A line 2 carries the true printed
    /// 1040 L11 — which can be NEGATIVE — but the 7.5% floor must still be clamped at zero.
    ///
    /// Unclamped, a negative AGI makes the floor negative and the deduction goes UP: line 4 would print
    /// $13,750 of allowed medical on $10,000 actually paid. §213(a) allows the expenses "to the extent
    /// that [they] exceed 7.5 percent" of AGI — the deduction can never exceed the expense. The exact
    /// engine has always clamped; the FILED form must agree with it.
    ///
    /// The r1 KAT missed this because it passed the FIXTURE's already-clamped AGI, not the production
    /// operand. This one passes a negative printed L11, exactly as `assemble_printed_forms` does.
    #[test]
    fn schedule_a_medical_floor_never_goes_negative_on_a_negative_agi() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.deduction_is_itemized = true;
        ar.itemized_deduction = Some(dec!(20000));
        ar.standard_deduction = dec!(14600);
        let mut parts = sched_a_parts_sales_tax();
        parts.medical_expenses = dec!(10000);
        ar.schedule_a = Some(parts);

        // The PRODUCTION operand: a negative printed 1040 line 11.
        let a = schedule_a_lines(&ar, dec!(-50000)).unwrap();

        assert_eq!(
            a.line2,
            dec!(-50000),
            "L2 still carries the true printed L11"
        );
        assert_eq!(
            a.line3,
            Usd::ZERO,
            "★ the 7.5% floor is CLAMPED — never negative"
        );
        assert_eq!(
            a.line4,
            dec!(10000),
            "…so the allowed medical is the expense PAID, never more"
        );
        assert!(
            a.line4 <= a.line1,
            "§213(a): the deduction can never exceed the expense"
        );
    }

    /// ★ **r2 NEW-I1, pinned END TO END (r3 NM7).** The QDCGT worksheet's line-3 operand is the figure a
    /// human reads off the FILED Schedule D — `min(printed L15, printed L16)`, ≥ 0 — not a re-rounding of
    /// the exact preferential gain. The printed Schedule D lines are sums of printed operands (their 8949
    /// column totals are `Σround`, not `roundΣ` — that is what the 302≠301 KAT is about), so they drift.
    ///
    /// This goes THROUGH `form_1040_lines`, not just the income block, because the r2 fold's first KAT
    /// pinned only the operand and a revert of the pass-through kept the suite green. The fixture is
    /// bin-straddling: printed Sch D = 5,003 vs exact 5,000, with printed L15 = 60,000, so the worksheet's
    /// ordinary remainder is 54,997 (bin [54,950–55,000)) on the printed operand and 55,000 (the NEXT bin)
    /// on the exact one — the $50-tread step function moves the filed L16 by a whole bin.
    #[test]
    fn form_1040_line16_takes_the_qdcgt_operand_off_the_printed_schedule_d() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.wages = dec!(60000);
        ar.deduction = dec!(5003);
        ar.net_ltcg = dec!(5000); // the EXACT preferential gain…

        // …but the PRINTED Schedule D says 5,003 (its lines sum the printed 8949 rows).
        let mut sd = schedule_d_lines(&ar, None);
        sd.line15 = dec!(5003);
        sd.line16 = dec!(5003);
        sd.routing = ScheduleDRouting::BothGains;

        let income = form_1040_income_lines(&ar, None, None, &sd);
        assert_eq!(
            income.qdcgt_net_capital_gain,
            dec!(5003),
            "★ the operand is min(printed Sch D L15, L16) — what the filer copies off the filed form"
        );
        assert_ne!(
            income.qdcgt_net_capital_gain,
            round_dollar(ar.net_ltcg),
            "…and NOT a re-rounding of the exact preferential gain"
        );

        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, None);
        let l = form_1040_lines(
            &ar,
            &income,
            None,
            None,
            None,
            &f8959,
            None,
            &tt(),
            FilingStatus::Single,
            Usd::ZERO,
            Usd::ZERO,
            false,
        );
        assert_eq!(
            l.line15,
            dec!(60000),
            "the fixture's printed taxable income"
        );

        let worksheet = |net_cap_gain: Usd| {
            crate::tax::method::qdcgt_line16(
                tt().ordinary_for(FilingStatus::Single),
                tt().ltcg_for(FilingStatus::Single),
                l.line15,
                l.line3a,
                net_cap_gain,
            )
        };
        // ★ The FILED L16 is the worksheet a human runs on the FILED forms…
        assert_eq!(
            l.line16,
            worksheet(income.qdcgt_net_capital_gain),
            "★ line 16 IS the worksheet on the PRINTED operands"
        );
        // …and it is NOT the worksheet on the exact-cents figure — the two land in different $50 bins.
        assert_ne!(
            l.line16,
            worksheet(round_dollar(ar.net_ltcg)),
            "a $1 drift in the operand crosses a Tax-Table bin edge and moves the filed L16"
        );
    }

    /// **I1.** 1040 line 1a ("Total amount from Form(s) W-2, box 1") must print, or the filed 1z sits
    /// above an EMPTY operand column and the form's own "Add lines 1a through 1h" sums blanks to 0 ≠ 1z.
    #[test]
    fn form_1040_line1a_carries_the_w2_wages_that_line1z_adds_up() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.wages = dec!(82000.49);
        let sd = schedule_d_lines(&ar, None);
        let income = form_1040_income_lines(&ar, None, None, &sd);

        assert_eq!(
            income.line1a,
            dec!(82000),
            "Σ W-2 box 1, rounded at the line"
        );
        assert_eq!(
            income.line1z, income.line1a,
            "L1z = 'Add lines 1a through 1h' — with no 1b–1h, it IS 1a, and both cells print"
        );
    }

    /// **I5.** Schedule A line 2's own text is "Enter amount from Form 1040 or 1040-SR, **line 11**" — a
    /// SOURCE citation, so it takes the PRINTED L11. Re-rounding the exact AGI could differ by dollars,
    /// and the divergence propagates into the 7.5% medical floor and on to L17 → 1040 L12.
    #[test]
    fn schedule_a_line2_is_the_printed_1040_line11() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.deduction_is_itemized = true;
        ar.itemized_deduction = Some(dec!(20000));
        ar.standard_deduction = dec!(14600);
        ar.schedule_a = Some(sched_a_parts_sales_tax());

        // A printed L11 that deliberately differs from the parts' exact AGI.
        let printed_l11 = dec!(50123);
        let a = schedule_a_lines(&ar, printed_l11).unwrap();
        assert_eq!(a.line2, printed_l11, "★ L2 IS the 1040's printed line 11");
        assert_eq!(
            a.line3,
            round_dollar(MEDICAL_FLOOR_RATE * printed_l11),
            "…and the 7.5% floor is taken on THAT figure, as the form says"
        );
    }

    // ── Form 8949 printed rows (ARCH-P6.3a D2/D3) ───────────────────────────────────────────────

    fn row8949(part: Form8949Part, proceeds: Usd, basis: Usd) -> Form8949Row {
        Form8949Row {
            part,
            box_: Form8949Box::C,
            box_needs_review: false,
            description: "1.00000000 BTC".into(),
            date_acquired: date!(2020 - 01 - 01),
            date_sold: date!(2024 - 05 - 01),
            proceeds,
            cost_basis: basis,
            adjustment_code: String::new(),
            adjustment_amount: Usd::ZERO,
            gain: proceeds - basis,
            wallet: crate::identity::WalletId::SelfCustody { label: "w".into() },
            disposition_kind: crate::event::DisposeKind::Sell,
        }
    }

    /// ★ Column (h) is DERIVED from the PRINTED cells — `h = d − e` — never rounded independently from
    /// the exact gain (ARCH-P6.3a D2). The discriminating row: proceeds $100.49, basis $0.50, exact gain
    /// $99.99. Independent rounding gives h = 100 while the printed d − e = 101 − 1 … no: d rounds to
    /// 100, e rounds to 1, so printed d − e = 99, and an independently-rounded h of 100 would VISIBLY
    /// violate the form's own column-(h) instruction ("Subtract column (e) from column (d)") by $1.
    #[test]
    fn form_8949_column_h_is_derived_from_the_printed_d_and_e_not_rounded_independently() {
        let rows = vec![row8949(Form8949Part::ShortTerm, dec!(100.49), dec!(0.50))];
        let p = form_8949_printed(&rows).expect("there are rows");

        let r = &p.short_term[0];
        assert_eq!(r.proceeds_d, dec!(100), "d rounds at the cell");
        assert_eq!(r.cost_e, dec!(1), "e rounds at the cell");
        assert_eq!(
            r.gain_h,
            dec!(99),
            "h = d − e on the PRINTED cells; an independently-rounded h would print 100 and the row \
             would contradict its own subtraction"
        );
        assert_eq!(r.gain_h, r.proceeds_d - r.cost_e, "the row self-checks");
        assert_ne!(
            r.gain_h,
            round_dollar(dec!(99.99)),
            "…and it is NOT round(exact gain)"
        );
    }

    /// The part totals sum the PRINTED rows, and Σh ≡ Σd − Σe by construction (an integer identity) —
    /// which is exactly what lets Schedule D's Part I satisfy its own "subtract (e) from (d)" header.
    #[test]
    fn form_8949_part_totals_sum_the_printed_rows_and_cross_foot() {
        let rows = vec![
            row8949(Form8949Part::ShortTerm, dec!(100.50), Usd::ZERO),
            row8949(Form8949Part::ShortTerm, dec!(200.50), Usd::ZERO),
            row8949(Form8949Part::LongTerm, dec!(50.49), dec!(10.50)),
        ];
        let p = form_8949_printed(&rows).unwrap();

        // ★ KAT-9, one level deeper: Σ round(row) = 101 + 201 = 302, while round(Σ) = round(301.00) = 301.
        assert_eq!(
            p.st_totals.proceeds_d,
            dec!(302),
            "the total sums the PRINTED rows"
        );
        assert_ne!(
            p.st_totals.proceeds_d,
            round_dollar(dec!(301.00)),
            "…and is deliberately NOT a re-rounding of the exact aggregate"
        );
        assert_eq!(
            p.st_totals.gain_h,
            p.st_totals.proceeds_d - p.st_totals.cost_e,
            "Σh ≡ Σd − Σe"
        );
        assert_eq!(
            p.lt_totals.gain_h,
            p.lt_totals.proceeds_d - p.lt_totals.cost_e
        );
    }

    /// ★ Schedule D lines 3(d)/(e)/(h) ARE the 8949's printed column totals — the form's own text says
    /// "Totals for all transactions reported on Form(s) 8949 with Box C checked". Re-rounding the exact
    /// aggregate instead would put Schedule D a dollar away from the 8949 stapled behind it.
    #[test]
    fn schedule_d_line3_takes_the_printed_8949_totals_not_a_re_rounded_aggregate() {
        let rows = vec![
            row8949(Form8949Part::ShortTerm, dec!(100.50), Usd::ZERO),
            row8949(Form8949Part::ShortTerm, dec!(200.50), Usd::ZERO),
        ];
        let f8949 = form_8949_printed(&rows).unwrap();

        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.schedule_d.st_proceeds_3d = dec!(301.00); // the EXACT aggregate
        ar.schedule_d.st_cost_3e = Usd::ZERO;
        ar.schedule_d.st_gain_3h = dec!(301.00);

        let sd = schedule_d_lines(&ar, Some(&f8949));
        assert_eq!(
            sd.line3_d,
            dec!(302),
            "= the 8949's printed column-(d) total"
        );
        assert_ne!(
            sd.line3_d,
            round_dollar(dec!(301.00)),
            "≠ round(exact aggregate)"
        );
        assert_eq!(
            sd.line3_h,
            sd.line3_d - sd.line3_e,
            "Schedule D Part I cross-foots too"
        );
    }

    /// ★ CRITICAL (`p6-extension-payment-dropped`, Fable ARCH-P6.3a D1). Schedule 3 line 10 is "Amount
    /// paid with request for extension to file", and line 15 is "Add lines 9 THROUGH 12 and 14" — so an
    /// extension payment belongs on the filed form. The exact arithmetic already credits it
    /// (`return_1040.rs` `total_payments`), but the printed chain had no line 10 and summed only line 11.
    ///
    /// A filer who paid $4,000 with their extension would therefore be told, ON THE FILED RETURN, to pay
    /// it a SECOND time: L31 drops the payment, so L33 falls and L37 "amount you owe" rises by exactly
    /// the amount already paid. The report and the PDF would have disagreed by the whole payment.
    #[test]
    fn schedule_3_line10_carries_the_extension_payment_and_line15_adds_it() {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.printed_inputs.extension_payment = dec!(4000);

        let s3 = schedule_3_lines(&ar).expect("an extension payment alone makes Schedule 3 file");
        assert_eq!(
            s3.line10,
            dec!(4000),
            "L10 — amount paid with the extension request"
        );
        assert_eq!(
            s3.line15,
            dec!(4000),
            "L15 = add 9 through 12 and 14 ⇒ includes L10"
        );
    }

    /// …and it sums with the excess-SS credit rather than replacing it (the old chain had `line15 =
    /// line11`, which silently made the two mutually exclusive).
    #[test]
    fn schedule_3_line15_adds_the_extension_payment_to_the_excess_ss_credit() {
        let mut ar = ar_with(None, Usd::ZERO, dec!(1200));
        ar.printed_inputs.extension_payment = dec!(4000);

        let s3 = schedule_3_lines(&ar).unwrap();
        assert_eq!(s3.line10, dec!(4000));
        assert_eq!(s3.line11, dec!(1200));
        assert_eq!(s3.line15, dec!(5200), "L15 sums the PRINTED lines above it");
    }

    /// ★ Schedule 2 line 4 EXCLUDES the 0.9% Additional Medicare Tax — that is a Form 8959 item, and
    /// it lands on line 11 instead. Bundling it into line 4 would double-count it against the 8959.
    #[test]
    fn schedule_2_line4_excludes_the_addl_medicare_which_lands_on_line_11() {
        let se = se_300k_single();
        let mut ar = ar_with(Some(se), Usd::ZERO, Usd::ZERO);
        ar.schedule_c = Some(crate::tax::return_1040::ScheduleCParts {
            gross_receipts_1: dec!(300000),
            total_expenses_28: Usd::ZERO,
            net_profit_31: dec!(300000),
        });
        let sch_c = schedule_c_lines(&ar).unwrap();
        let sse = schedule_se_lines(&ar, &sch_c).unwrap();
        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, Some(&se));
        let s2 = schedule_2_lines(Some(&sse), &f8959, None).unwrap();

        // ★ L4 IS Schedule SE's printed L12 = printed L10 + printed L11 = round(21,836.40) +
        // round(8,034.45) = 21,836 + 8,034 = 29,870 — and NOT round_dollar(29,870.85) = 29,871.
        // The dollar is the point: the filed Schedule 2 must agree with the Schedule SE behind it.
        assert_eq!(s2.line4, dec!(29870));
        assert_eq!(
            s2.line4, sse.line12,
            "★ Sch 2 L4 IS Schedule SE's printed line 12"
        );
        assert_ne!(
            s2.line4,
            round_dollar(dec!(29870.85)),
            "NOT a re-rounding of the exact ss + medicare — that would disagree with Schedule SE"
        );
        assert_ne!(
            s2.line4,
            round_dollar(se.total),
            "NOT the §1401 total (that folds in the 0.9%)"
        );
        // the 0.9% shows up HERE instead…
        assert_eq!(s2.line11, dec!(693)); // 8959 printed line 18
        assert_eq!(s2.line12, Usd::ZERO); // no NIIT
        assert_eq!(s2.line21, dec!(30563)); // 29,870 + 693 — the PRINTED lines
    }

    /// ★ **The chains COMPOSE on the PRINTED lines.** Schedule 2 line 11 must be Form 8959's printed
    /// line 18 — not `round_dollar` of the exact-cents figure. With the KAT-9 fixture (Part I of
    /// $274.50 and Part II of $499.50) those differ by a dollar: the printed 8959 says 775, while the
    /// exact total rounds to 774. If Schedule 2 took the latter, the schedule and its own attachment
    /// would disagree, and the filed return would not tie out.
    #[test]
    fn schedule_2_line11_takes_the_printed_8959_line_18_not_the_rounded_total() {
        let se = SeTaxResult {
            net_se: dec!(60097.46),
            base: dec!(55500.00),
            ss: dec!(0.00),
            medicare: dec!(1609.50),
            addl: dec!(499.50),
            total: dec!(2109.00),
            deductible_half: dec!(804.75),
        };
        let f8959 = form_8959_lines(FilingStatus::Mfj, dec!(280500), Usd::ZERO, Some(&se));

        assert_eq!(f8959.line18, dec!(775)); // the printed, cross-footing 8959 total
        let exact_total = dec!(274.50) + dec!(499.50); // what the engine carries, in cents
        assert_eq!(round_dollar(exact_total), dec!(774)); // …which rounds to something ELSE

        let s2 = schedule_2_lines(None, &f8959, None).unwrap();
        assert_eq!(
            s2.line11,
            dec!(775),
            "Schedule 2 must carry the 8959's PRINTED line 18"
        );
        assert_ne!(s2.line11, round_dollar(exact_total));
    }

    /// Nothing to report ⇒ no Schedule 2 at all (1040 line 23 is zero).
    #[test]
    fn schedule_2_absent_when_no_other_taxes() {
        let f8959 = form_8959_lines(FilingStatus::Single, dec!(50000), Usd::ZERO, None);
        assert_eq!(f8959.line18, Usd::ZERO);
        assert!(schedule_2_lines(None, &f8959, None).is_none());
    }

    /// Schedule 3 carries the FTC and the excess-SS credit, and cross-foots to 1040 L20 / L31.
    #[test]
    fn schedule_3_carries_ftc_and_excess_social_security() {
        let ar = ar_with(None, dec!(287.40), dec!(1234.56));
        let s3 = schedule_3_lines(&ar).unwrap();
        assert_eq!(s3.line1, dec!(287)); // FTC, rounded at the line
        assert_eq!(s3.line8, dec!(287)); // → 1040 L20 (every other credit is blank)
        assert_eq!(s3.line11, dec!(1235)); // excess SS, half-up
        assert_eq!(s3.line15, dec!(1235)); // → 1040 L31

        // Either one alone still files the schedule…
        assert!(schedule_3_lines(&ar_with(None, dec!(100), Usd::ZERO)).is_some());
        assert!(schedule_3_lines(&ar_with(None, Usd::ZERO, dec!(100))).is_some());
        // …but neither means no schedule.
        assert!(schedule_3_lines(&ar_with(None, Usd::ZERO, Usd::ZERO)).is_none());
    }

    /// A `ScheduleAParts` for the printed-chain tests.
    #[allow(clippy::too_many_arguments)]
    fn parts(
        medical: Usd,
        agi: Usd,
        salt_5a: Usd,
        salt_5b: Usd,
        salt_5c: Usd,
        salt_cap: Usd,
        mortgage: Usd,
        cash: Usd,
        noncash: Usd,
        carryover: Usd,
    ) -> crate::tax::return_1040::ScheduleAParts {
        use crate::tax::return_1040::{ScheduleAParts, MEDICAL_FLOOR_RATE};
        let agi = agi.max(Usd::ZERO);
        let floor = MEDICAL_FLOOR_RATE * agi;
        let salt_5d = salt_5a + salt_5b + salt_5c;
        let salt_5e = salt_5d.min(salt_cap);
        let medical_allowed = (medical - floor).max(Usd::ZERO);
        ScheduleAParts {
            salt_is_sales_tax: false,
            medical_expenses: medical,
            agi,
            medical_floor: floor,
            medical_allowed,
            salt_5a,
            salt_5b,
            salt_5c,
            salt_5d,
            salt_5e,
            salt_cap,
            mortgage_8a: mortgage,
            mortgage_mixed_use_box: false,
            charitable_cash_11: cash,
            charitable_noncash_12: noncash,
            charitable_carryover_13: carryover,
            charitable_14: cash + noncash + carryover,
            total_17: medical_allowed + salt_5e + mortgage + cash + noncash + carryover,
        }
    }

    fn ar_itemizing(p: crate::tax::return_1040::ScheduleAParts) -> AbsoluteReturn {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.schedule_a = Some(p);
        ar.deduction_is_itemized = true;
        ar.itemized_deduction = Some(p.total_17);
        ar
    }

    /// The printed Schedule A chain, end to end: the medical floor binds, the SALT cap binds, and the
    /// total sums the PRINTED subtotals.
    #[test]
    fn schedule_a_printed_chain_medical_floor_and_salt_cap() {
        // AGI 100,000 ⇒ 7.5% floor = 7,500. Medical 10,000 ⇒ 2,500 allowed.
        // SALT 8,000 + 4,000 + 500 = 12,500 ⇒ capped at 10,000.
        // Mortgage 12,000. Charitable: 1,000 cash + 2,000 noncash + 500 carryover = 3,500.
        let ar = ar_itemizing(parts(
            dec!(10000),
            dec!(100000),
            dec!(8000),
            dec!(4000),
            dec!(500),
            dec!(10000),
            dec!(12000),
            dec!(1000),
            dec!(2000),
            dec!(500),
        ));
        // The printed 1040 L11 for this fixture (the parts carry the AGI; `ar.agi` is unset here).
        let l = schedule_a_lines(&ar, round_dollar(ar.schedule_a.as_ref().unwrap().agi)).unwrap();

        assert_eq!(l.line1, dec!(10000));
        assert_eq!(l.line2, dec!(100000));
        assert_eq!(l.line3, dec!(7500)); // 7.5% of the PRINTED AGI
        assert_eq!(l.line4, dec!(2500));
        assert_eq!(l.line5d, dec!(12500));
        assert_eq!(l.line5e, dec!(10000)); // ★ the §164(b) cap binds
        assert_eq!(l.line7, dec!(10000));
        assert_eq!(l.line8a, dec!(12000));
        assert_eq!(l.line10, dec!(12000));
        assert_eq!(l.line11, dec!(1000));
        assert_eq!(l.line12, dec!(2000));
        assert_eq!(l.line13, dec!(500));
        assert_eq!(l.line14, dec!(3500));
        assert_eq!(l.line17, dec!(28000)); // 2,500 + 10,000 + 12,000 + 3,500
    }

    /// ★ Schedule A is COMPUTED even when the standard deduction wins (that is how the max() is
    /// taken) — but it is only FILED when it is the deduction actually claimed. Printing a Schedule A
    /// the filer did not use would be a form asserting a deduction they never took.
    #[test]
    fn schedule_a_not_filed_when_the_standard_deduction_wins() {
        let p = parts(
            Usd::ZERO,
            dec!(100000),
            dec!(1000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(10000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        );
        let mut ar = ar_itemizing(p);
        ar.deduction_is_itemized = false; // the standard deduction was larger
        assert!(
            schedule_a_lines(&ar, round_dollar(ar.agi)).is_none(),
            "a Schedule A the filer did not use must not be filed"
        );
    }

    /// The printed chain cross-foots, and every cell is a whole dollar — including when the inputs
    /// carry cents and a negative AGI clamps the floor to zero (so the floor can never HELP the filer).
    #[test]
    fn schedule_a_printed_lines_cross_foot() {
        for p in [
            parts(
                dec!(10000.49),
                dec!(100000.51),
                dec!(8000.50),
                dec!(4000),
                dec!(500),
                dec!(10000),
                dec!(12000),
                dec!(1000),
                dec!(2000),
                dec!(500),
            ),
            // Negative AGI: the clamp means floor = 0, so the FULL medical expense is allowed.
            parts(
                dec!(10000),
                dec!(-50000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                dec!(10000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
            ),
            // MFS: the cap is half.
            parts(
                Usd::ZERO,
                dec!(80000),
                dec!(9000),
                dec!(1000),
                Usd::ZERO,
                dec!(5000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
            ),
        ] {
            let l = {
                let a = ar_itemizing(p);
                schedule_a_lines(&a, round_dollar(a.schedule_a.as_ref().unwrap().agi))
            }
            .unwrap();
            assert_eq!(
                l.line3,
                round_dollar(MEDICAL_FLOOR_RATE * l.line2),
                "L3 = 7.5% × printed L2"
            );
            assert_eq!(
                l.line4,
                (l.line1 - l.line3).max(Usd::ZERO),
                "L4 = 1 − 3, floored"
            );
            assert_eq!(
                l.line5d,
                l.line5a + l.line5b + l.line5c,
                "L5d = 5a + 5b + 5c (printed)"
            );
            assert!(l.line5e <= l.line5d, "L5e never exceeds L5d");
            assert_eq!(l.line7, l.line5e, "L7 = 5e + 6 (6 blank)");
            assert_eq!(l.line10, l.line8a, "L10 = 8e + 9, 8e = 8a (rest blank)");
            assert_eq!(
                l.line14,
                l.line11 + l.line12 + l.line13,
                "L14 = 11 + 12 + 13 (printed)"
            );
            assert_eq!(
                l.line17,
                l.line4 + l.line7 + l.line10 + l.line14,
                "L17 sums the PRINTED subtotals"
            );
            for cell in [
                l.line1, l.line2, l.line3, l.line4, l.line5a, l.line5b, l.line5c, l.line5d,
                l.line5e, l.line7, l.line8a, l.line8e, l.line10, l.line11, l.line12, l.line13,
                l.line14, l.line17,
            ] {
                assert_eq!(
                    cell.fract(),
                    Usd::ZERO,
                    "printed cells are whole dollars: {cell}"
                );
            }
        }

        // The negative-AGI case, specifically: floor = 0 ⇒ the whole $10,000 medical is allowed.
        let neg = {
            let a = ar_itemizing(parts(
                dec!(10000),
                dec!(-50000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                dec!(10000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
            ));
            schedule_a_lines(&a, round_dollar(a.schedule_a.as_ref().unwrap().agi))
        }
        .unwrap();
        assert_eq!(
            neg.line2,
            Usd::ZERO,
            "a negative AGI is clamped — the floor must never HELP"
        );
        assert_eq!(neg.line3, Usd::ZERO);
        assert_eq!(neg.line4, dec!(10000));
    }

    /// Build an `AbsoluteReturn` whose Schedule D nets to the given (st, lt) with the given
    /// carryovers, distributions, §1211 offset and qualified dividends.
    #[allow(clippy::too_many_arguments)]
    /// The printed Form 8949 whose column totals ARE this fixture's Schedule D lines 3 and 10 — the
    /// composition the filed return requires (ARCH-P6.3a D3). The routing fixtures set the Schedule D
    /// parts directly; in production those cells come from the attached 8949, so the fixtures must
    /// supply one that agrees, exactly as a real return does.
    fn f8949_for(ar: &AbsoluteReturn) -> Printed8949 {
        let p = &ar.schedule_d;
        Printed8949 {
            short_term: Vec::new(),
            long_term: Vec::new(),
            st_totals: Printed8949Totals {
                proceeds_d: round_dollar(p.st_proceeds_3d),
                cost_e: round_dollar(p.st_cost_3e),
                gain_h: round_dollar(p.st_gain_3h),
            },
            lt_totals: Printed8949Totals {
                proceeds_d: round_dollar(p.lt_proceeds_10d),
                cost_e: round_dollar(p.lt_cost_10e),
                gain_h: round_dollar(p.lt_gain_10h),
            },
        }
    }

    /// `schedule_d_lines` with the fixture's own matching 8949 attached.
    fn sd_lines(ar: &AbsoluteReturn) -> ScheduleDLines {
        let f = f8949_for(ar);
        schedule_d_lines(ar, Some(&f))
    }

    fn ar_sched_d(
        st_net: Usd,
        lt_net: Usd,
        st_cf: Usd,
        lt_cf: Usd,
        distr: Usd,
        loss_ded: Usd,
        qd: Usd,
    ) -> AbsoluteReturn {
        use crate::tax::return_1040::ScheduleDParts;
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.schedule_d = ScheduleDParts {
            st_proceeds_3d: Usd::ZERO,
            st_cost_3e: Usd::ZERO,
            st_gain_3h: st_net + st_cf, // the raw crypto gain, before the line-6 carryover
            st_carryover_6: st_cf,
            st_net_7: st_net,
            lt_proceeds_10d: Usd::ZERO,
            lt_cost_10e: Usd::ZERO,
            lt_gain_10h: lt_net + lt_cf - distr,
            cap_gain_distr_13: distr,
            lt_carryover_14: lt_cf,
            lt_net_15: lt_net,
            total_16: st_net + lt_net,
            loss_deduction_21: loss_ded,
            qualified_dividends: qd,
        };
        ar
    }

    /// ★ **SPEC §7.2 path 1 — BOTH GAINS.** L16 > 0 and L15 > 0: line 17 = Yes, 18/19 = 0, line 20 =
    /// Yes → QDCGT. Lines 21 and 22 are NOT completed; the form says so in terms.
    #[test]
    fn schedule_d_routing_both_gains() {
        let l = sd_lines(&ar_sched_d(
            dec!(5000),
            dec!(20000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(1200),
        ));
        assert_eq!(l.line7, dec!(5000));
        assert_eq!(l.line15, dec!(20000));
        assert_eq!(l.line16, dec!(25000));
        assert_eq!(l.routing, ScheduleDRouting::BothGains);
    }

    /// ★ **SPEC §7.2 path 2 — SHORT-TERM GAIN / LONG-TERM LOSS.** The common crypto year. L16 > 0 but
    /// L15 ≤ 0 ⇒ line 17 = No ⇒ skip 18–21 ⇒ line 22, which routes to QDCGT iff there are qualified
    /// dividends.
    #[test]
    fn schedule_d_routing_short_gain_long_loss() {
        // With qualified dividends → line 22 = Yes.
        let l = sd_lines(&ar_sched_d(
            dec!(30000),
            dec!(-4000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(1200),
        ));
        assert_eq!(l.line16, dec!(26000));
        assert_eq!(
            l.routing,
            ScheduleDRouting::ShortGainLongLoss { line22_yes: true }
        );

        // …and WITHOUT them → line 22 = No (the Tax Table / TCW path, not QDCGT).
        let l = sd_lines(&ar_sched_d(
            dec!(30000),
            dec!(-4000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        ));
        assert_eq!(
            l.routing,
            ScheduleDRouting::ShortGainLongLoss { line22_yes: false }
        );
    }

    /// ★ **SPEC §7.2 path 3 — NET LOSS.** L16 < 0: skip 17–20; line 21 carries the §1211(b) allowed
    /// offset as a POSITIVE MAGNITUDE (the form pre-prints the parentheses, and a negative there would
    /// render as a positive number on a filed return); line 22 is still answered.
    #[test]
    fn schedule_d_routing_net_loss_line21_is_a_positive_magnitude() {
        // A $10,000 net loss, of which §1211(b) allows $3,000 this year.
        let l = sd_lines(&ar_sched_d(
            dec!(-10000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(3000),
            dec!(800),
        ));
        assert_eq!(l.line16, dec!(-10000));
        match l.routing {
            ScheduleDRouting::NetLoss { line21, line22_yes } => {
                assert_eq!(line21, dec!(3000), "the §1211(b) cap");
                assert!(
                    line21 > Usd::ZERO,
                    "★ line 21 is a MAGNITUDE — the form prints the ( )"
                );
                assert!(line22_yes);
            }
            other => panic!("expected NetLoss, got {other:?}"),
        }
    }

    /// ★ **SPEC §7.2 path 4 — ZERO.** L16 = 0: 1040 line 7 is `-0-`; skip 17–21; line 22 is still
    /// answered. The easiest branch to forget, and the one that silently routes the whole tax
    /// computation if it is wrong.
    #[test]
    fn schedule_d_routing_zero() {
        let l = sd_lines(&ar_sched_d(
            dec!(4000),
            dec!(-4000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        ));
        assert_eq!(l.line16, Usd::ZERO);
        assert_eq!(l.routing, ScheduleDRouting::Zero { line22_yes: false });
    }

    /// ★ The lines the CRYPTO-SLICE Schedule D omits — and which the P5-C1 refusal exists to cover.
    /// Line 13 (1099-DIV box-2a capital-gain distributions) and lines 6/14 (capital-loss carryovers)
    /// ARE part of the computed return, and the full-return Schedule D prints all three. Lines 6 and
    /// 14 are PAREN boxes ⇒ positive magnitudes.
    #[test]
    fn schedule_d_prints_the_lines_the_crypto_slice_omits() {
        let l = sd_lines(&ar_sched_d(
            dec!(1000),  // st_net (after the line-6 carryover)
            dec!(15000), // lt_net (after line 13 and the line-14 carryover)
            dec!(2000),  // line 6 — prior-year SHORT-term carryover
            dec!(500),   // line 14 — prior-year LONG-term carryover
            dec!(3000),  // line 13 — capital gain distributions
            Usd::ZERO,
            Usd::ZERO,
        ));
        assert_eq!(
            l.line6,
            dec!(2000),
            "line 6 is filled — the crypto slice has no line 6"
        );
        assert_eq!(
            l.line13,
            dec!(3000),
            "line 13 is filled — the crypto slice has no line 13"
        );
        assert_eq!(
            l.line14,
            dec!(500),
            "line 14 is filled — the crypto slice has no line 14"
        );
        for paren in [l.line6, l.line14] {
            assert!(paren >= Usd::ZERO, "paren cells are magnitudes: {paren}");
        }
        assert_eq!(l.line16, dec!(16000)); // 1,000 + 15,000, from the PRINTED lines
    }

    /// ★ **The composition rule, end to end.** Every 1040 line that comes from a schedule must carry
    /// that schedule's PRINTED figure, not a re-rounding of the exact-cents computation. This uses the
    /// KAT-9 fixture, where the two genuinely differ: Form 8959's printed line 18 is 775, while
    /// `round_dollar` of the exact total is 774. Schedule 2 line 21 must carry 775, and 1040 line 23
    /// must carry Schedule 2's 775 — so the dollar propagates correctly all the way to TOTAL TAX.
    /// Take the exact figure anywhere in that chain and the filed 1040 disagrees with its own
    /// attachments.
    #[test]
    fn form_1040_takes_the_printed_figures_from_its_schedules() {
        let se = SeTaxResult {
            net_se: dec!(60097.46),
            base: dec!(55500.00),
            ss: dec!(0.00),
            medicare: dec!(1609.50),
            addl: dec!(499.50),
            total: dec!(2109.00),
            deductible_half: dec!(804.75),
        };
        let mut ar = ar_with(Some(se), Usd::ZERO, Usd::ZERO);
        ar.wages = dec!(280500);
        ar.regular_tax = dec!(50000);
        ar.deduction = dec!(29200);

        let f8959 = form_8959_lines(FilingStatus::Mfj, dec!(280500), Usd::ZERO, Some(&se));
        assert_eq!(f8959.line18, dec!(775), "the printed 8959 total");
        assert_eq!(
            round_dollar(dec!(274.50) + dec!(499.50)),
            dec!(774),
            "…the exact one differs"
        );

        let s2 = schedule_2_lines(None, &f8959, None).unwrap();
        assert_eq!(
            s2.line11,
            dec!(775),
            "Schedule 2 L11 = the 8959's PRINTED L18"
        );

        let sd = schedule_d_lines(&ar, None);
        let l = {
            let income = form_1040_income_lines(&ar, None, None, &sd);
            form_1040_lines(
                &ar,
                &income,
                None,
                Some(&s2),
                None,
                &f8959,
                None,
                &tt(),
                FilingStatus::Single,
                Usd::ZERO,
                Usd::ZERO,
                false,
            )
        };
        assert_eq!(l.line23, s2.line21, "1040 L23 = Schedule 2's PRINTED L21");
        assert_eq!(
            l.line24,
            l.line22 + l.line23,
            "TOTAL TAX sums the PRINTED lines"
        );
        // …and 25c carries the 8959's printed line 24 (the over-withholding credit), not a re-derivation.
        assert_eq!(l.line25c, f8959.line24);
    }

    /// The printed 1040 cross-foots: every total re-derives from the OTHER printed lines, and every
    /// cell is a whole dollar.
    #[test]
    fn form_1040_printed_lines_cross_foot() {
        let mut ar = ar_with(None, dec!(287.40), dec!(1234.56));
        ar.wages = dec!(120000.49);
        ar.taxable_interest = dec!(2000.50);
        ar.ordinary_dividends = dec!(4000.50);
        ar.qualified_dividends = dec!(3000);
        ar.regular_tax = dec!(18000.50);
        ar.deduction = dec!(14600);
        ar.withholding_25a = dec!(15000.49);
        ar.withholding_25b = dec!(300.50);

        let f8959 = form_8959_lines(FilingStatus::Single, dec!(120000), dec!(1800), None);
        let s3 = schedule_3_lines(&ar).unwrap();
        let sd = schedule_d_lines(&ar, None);
        let l = {
            let income = form_1040_income_lines(&ar, None, None, &sd);
            form_1040_lines(
                &ar,
                &income,
                None,
                None,
                Some(&s3),
                &f8959,
                None,
                &tt(),
                FilingStatus::Single,
                Usd::ZERO,
                dec!(500),
                true,
            )
        };

        assert_eq!(
            l.line9,
            l.line1z + l.line2b + l.line3b + l.line7 + l.line8,
            "L9 = 1z+2b+3b+7+8"
        );
        assert_eq!(l.line11, l.line9 - l.line10, "AGI = 9 − 10");
        assert_eq!(l.line14, l.line12 + l.line13, "L14 = 12 + 13");
        assert_eq!(
            l.line15,
            (l.line11 - l.line14).max(Usd::ZERO),
            "TI = 11 − 14, floored"
        );
        assert_eq!(l.line18, l.line16 + l.line17, "L18 = 16 + 17");
        assert_eq!(l.line21, l.line19 + l.line20, "L21 = 19 + 20");
        assert_eq!(
            l.line22,
            (l.line18 - l.line21).max(Usd::ZERO),
            "L22 = 18 − 21, floored"
        );
        assert_eq!(l.line24, l.line22 + l.line23, "TOTAL TAX = 22 + 23");
        assert_eq!(
            l.line25d,
            l.line25a + l.line25b + l.line25c,
            "L25d = 25a+25b+25c"
        );
        assert_eq!(
            l.line33,
            l.line25d + l.line26 + l.line32,
            "TOTAL PAYMENTS = 25d + 26 + 32"
        );
        // At most one of refund / owed is nonzero, and they come from the PRINTED totals.
        assert_eq!(l.line34, (l.line33 - l.line24).max(Usd::ZERO));
        assert_eq!(l.line37, (l.line24 - l.line33).max(Usd::ZERO));
        assert!(
            l.line34.is_zero() || l.line37.is_zero(),
            "cannot both owe and be refunded"
        );

        // Schedule 3's printed L8/L15 land on 1040 L20/L31.
        assert_eq!(l.line20, s3.line8);
        assert_eq!(l.line31, s3.line15);
        // L19 is the CTC/ODC conservative omission — pinned to 0 (the advisory carries the news).
        assert_eq!(l.line19, Usd::ZERO);

        for cell in [
            l.line1z, l.line2b, l.line3a, l.line3b, l.line7, l.line8, l.line9, l.line10, l.line11,
            l.line12, l.line13, l.line14, l.line15, l.line16, l.line18, l.line21, l.line22,
            l.line24, l.line25a, l.line25b, l.line25c, l.line25d, l.line26, l.line33,
        ] {
            assert_eq!(
                cell.fract(),
                Usd::ZERO,
                "printed cells are whole dollars: {cell}"
            );
        }
    }

    /// ★ **1040 line 7 on a net-loss year.** It carries the §1211(b)-LIMITED amount — `−(Schedule D
    /// line 21)`, i.e. −3,000 — NOT the full $10,000 loss. And it is signed with a **leading minus**,
    /// not parentheses (SPEC §3.2), unlike Schedule D's own lines 6/14/21 which are paren boxes
    /// carrying magnitudes. Printing the full loss would overstate the deduction more than threefold.
    #[test]
    fn form_1040_line7_on_a_loss_year_is_the_limited_amount_with_a_leading_minus() {
        let mut ar = ar_sched_d(
            dec!(-10000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(3000), // the §1211(b) allowed offset
            Usd::ZERO,
        );
        ar.wages = dec!(80000);
        ar.deduction = dec!(14600);

        // A loss means there were disposals, so a real return has an 8949 behind Schedule D.
        let sd = sd_lines(&ar);
        assert_eq!(
            sd.line16,
            dec!(-10000),
            "Schedule D's own line 16 is the FULL loss"
        );
        assert!(
            matches!(sd.routing, ScheduleDRouting::NetLoss { line21, .. } if line21 == dec!(3000))
        );

        let f8959 = form_8959_lines(FilingStatus::Single, dec!(80000), Usd::ZERO, None);
        let l = {
            let income = form_1040_income_lines(&ar, None, None, &sd);
            form_1040_lines(
                &ar,
                &income,
                None,
                None,
                None,
                &f8959,
                None,
                &tt(),
                FilingStatus::Single,
                Usd::ZERO,
                Usd::ZERO,
                true,
            )
        };
        assert_eq!(
            l.line7,
            dec!(-3000),
            "★ the §1211-LIMITED loss, not the full −10,000"
        );
        assert!(
            l.line7 < Usd::ZERO,
            "a leading minus — 1040 L7 is not a paren box"
        );
    }
}
