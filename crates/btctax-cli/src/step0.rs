//! ★★★ **R9 / T6 — STEP 0 OF THE INTERVIEW: A STATUS PANEL, NEVER A QUESTION SET.**
//!
//! The ledger's questions are about EVENTS — *this outbound and that inbound are the same coins* —
//! durable forever, gated by [`BlockerKind`], and answered in `reconcile`. The return's questions are
//! about a TAX YEAR, `Durability::PerYear`, gated by `RefuseReason`. Different durability, different
//! scope, different store — so Step 0 of the interview does not ASK anything. It REPORTS, over the
//! held session, what the ledger still needs, and every row hands off to the `reconcile` command that
//! answers it.
//!
//! **What it lists** (R9): blockers by kind; imports per venue; venues with dispositions in the year
//! versus providers with a Form 1099-DA answer (J-4, J-7); venues with a custodial disposition and no
//! standing order in force before it — Notice 2026-20 §4.02(2) — with the consequence stated now
//! (J-9, J-10); and the owner actions with dates (J-9b).
//!
//! **It writes NOTHING**, and it reads the ledger only. `record_answer` stays the only writer of
//! `answer_log`; no `reconcile` question moves into a return registry (R15).
//!
//! ★★★ **Authoring proceeds in parallel with an unresolved ledger.** This panel does not gate: the
//!     Sep–Dec calendar needs a filer to answer their W-2 boxes while a transfer is still unpaired.
//!     Commit and export stay hard-gated by the gates that already exist —
//!     `resolve::resolve_full_return`'s `TaxYearNotComputable` and `screen_*` chain — and this module
//!     adds none of its own.
//!
//! ★ **Every derivation here is SHARED with the thing it reports on**, never re-implemented:
//!   - the standing-order row calls [`btctax_core::standing_order_in_force`], which calls the SAME
//!     `resolve_election` the fold uses to pick the filed basis;
//!   - the venue-vs-answer rows call [`btctax_core::forms::broker_key_census`] over
//!     [`btctax_core::form_8949`] — the same keys `screen_broker_reporting` demands answers for;
//!   - the blocker rows walk `state.blockers` and map the KIND through an exhaustive `match`, so a
//!     new [`BlockerKind`] is a compile error here rather than a row that silently loses its exit.

use btctax_core::forms::{broker_key_census, Cohort};
use btctax_core::state::{BlockerKind, LedgerState, Severity};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::{EventId, LedgerEvent, WalletId};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

/// One panel row: what was found, and the command that answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step0Row {
    /// The finding, in one sentence, naming the thing the filer can go and look at.
    pub what: String,
    /// The command that answers it. Empty only for a row that is purely informational (an import
    /// count), because *"a refusal with no exit is just a brick with better prose"* and a status row
    /// with no exit is the same shape one step earlier.
    pub handoff: String,
}

impl Step0Row {
    fn new(what: impl Into<String>, handoff: impl Into<String>) -> Self {
        Step0Row {
            what: what.into(),
            handoff: handoff.into(),
        }
    }
}

/// ★★★ The Step 0 panel: five lists, each derived from the ledger, none of them stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Step0Panel {
    /// The tax year the panel was computed for.
    pub year: i32,
    /// Ledger blockers, one row per [`BlockerKind`] present, with its count and severity. Unresolved
    /// import and decision CONFLICTS are two of these kinds — they are not a separate list, because a
    /// second derivation of the same rows is a second thing to keep in step.
    pub blockers: Vec<Step0Row>,
    /// Imports per venue: how many events each source contributed. Informational.
    pub imports: Vec<Step0Row>,
    /// Venues with a Form 8949 row in the year versus providers with a Form 1099-DA answer on file.
    /// A venue with rows and no answer is NAMED (J-4, J-7).
    pub venues: Vec<Step0Row>,
    /// Venues with a custodial disposition in the year and NO standing order in force before it —
    /// Notice 2026-20 §4.02(2) (J-9, J-10).
    pub standing_orders: Vec<Step0Row>,
    /// Owner actions with dates — today, the §170(f)(11) qualified appraisals a donation needs.
    pub owner_actions: Vec<Step0Row>,
}

impl Step0Panel {
    /// How many rows the panel would print. Zero ⇒ the ledger has nothing outstanding for this year.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.blockers.len()
            + self.imports.len()
            + self.venues.len()
            + self.standing_orders.len()
            + self.owner_actions.len()
    }
}

/// ★★★ **The `reconcile` command that answers a blocker of this kind.**
///
/// An EXHAUSTIVE `match` with no wildcard, deliberately: adding a [`BlockerKind`] is then a compile
/// error here, and the alternative — a lookup table with a fallback — is exactly the shape that ships
/// a blocker row whose exit silently becomes *"see the docs"*.
fn blocker_handoff(kind: BlockerKind) -> &'static str {
    use BlockerKind as B;
    match kind {
        B::FmvMissing => "btctax reconcile set-fmv <event> --fmv <usd>",
        B::UncoveredDisposal => {
            "btctax reconcile classify-raw <event>  (or declare the missing acquisition)"
        }
        B::ImportConflict => {
            "btctax reconcile accept-conflict <conflict> | reject-conflict <conflict>"
        }
        B::DecisionConflict => "btctax reconcile void <decision>  (two decisions target one event)",
        B::UnknownBasisInbound => {
            "btctax reconcile classify-inbound-income | classify-inbound-gift | \
             classify-inbound-self-transfer <event>"
        }
        B::Unclassified => "btctax reconcile classify-raw <event>",
        B::SafeHarborUnconservable => "btctax reconcile safe-harbor-allocate  (fix the allocation)",
        B::SafeHarborTimebar => "btctax reconcile safe-harbor-attest  (if timely in your books)",
        B::UnmatchedOutflows => {
            "btctax reconcile match-self-transfers  (preview), then link-transfer --in/--out"
        }
        B::Pre2025MethodNote => "btctax config --set-pre2025-method <m> --attest",
        B::MethodElectionBackdated => {
            "btctax reconcile void <decision>, then btctax config --set-forward-method <m> \
             (a standing order can never be back-dated)"
        }
        B::LotSelectionInvalid => "btctax reconcile select-lots <disposal>  (re-identify the lots)",
        B::LotSelectionPostHoc => {
            "btctax optimize accept --attest  (or accept the deemed acquisition order)"
        }
        B::IdentificationDefaulted => {
            "btctax config --set-forward-method <m> --exchange <venue>  (forward only; past sales \
             cannot be re-identified)"
        }
        B::Pre2025MethodConflictsAllocation => {
            "btctax config --set-pre2025-method <the method the allocation recorded>"
        }
        B::TaxYearNotComputable => "resolve the Hard blockers above; this one clears with them",
        B::TaxProfileMissing => "btctax tax-profile --year <y> …  (or `btctax income import`)",
        B::TaxTableMissing => "no action — this build bundles no table for that year",
        B::QualifiedAppraisalNote => {
            "btctax reconcile set-donation-details <event> --appraiser-name … --appraisal-date …"
        }
        B::SelfTransferInboundZeroBasis => {
            "btctax reconcile classify-inbound-self-transfer <event> --basis <usd>"
        }
        B::SelfTransferInboundDefaultedAcquired => {
            "btctax reconcile classify-inbound-self-transfer <event> --acquired <YYYY-MM-DD>"
        }
        B::PseudoReconcileActive => {
            "btctax reconcile pseudo approve  (or turn pseudo mode off) — a pseudo projection is \
             never filable"
        }
        B::SelfTransferDoubleBooked => {
            "btctax reconcile void <link>  (a link and an inbound book the same coins twice)"
        }
    }
}

/// ★★★ **Notice 2026-20 §4.02(2) — the consequence, stated NOW rather than at export** (J-9, J-10).
///
/// The Notice makes a standing order an adequate identification only if it is *"entered into the
/// taxpayer's books and records before the units covered by the order are sold"*. Without one, the
/// units sold are whatever the BROKER says they were, and §4.02's relief does not apply for
/// §1.6045-1 information reporting anyway — so box 1g will carry the broker's own default, the
/// filer's `basis_differs` answer will refuse, and the exit is a per-lot identification.
const STANDING_ORDER_CONSEQUENCE: &str =
    "expect box 1g to reflect the broker's default; a `basis_differs` answer refuses; \
     `btctax reconcile select-lots` / `import-selections` is the exit";

/// ★★★ **THE STEP 0 PANEL**, computed over the held session. Reads; never writes.
///
/// `ri` is the year's working return when one exists — `None` on a year the filer has not started,
/// which is a normal state at Step 0 and not an error: the venue-vs-answer list then names every
/// venue with rows, because none of them has an answer yet.
#[must_use]
pub fn step0_panel(
    state: &LedgerState,
    events: &[LedgerEvent],
    ri: Option<&ReturnInputs>,
    year: i32,
) -> Step0Panel {
    let mut panel = Step0Panel {
        year,
        ..Default::default()
    };

    // ── Blockers, by KIND. One row per kind present, with its count and severity. ────────────────
    let mut by_kind: BTreeMap<BlockerKind, usize> = BTreeMap::new();
    for b in &state.blockers {
        *by_kind.entry(b.kind).or_insert(0) += 1;
    }
    for (kind, n) in by_kind {
        let sev = match kind.severity() {
            Severity::Hard => "HARD — commit and export refuse while it stands",
            Severity::Advisory => "advisory — it never gates, and it is still worth clearing",
        };
        panel.blockers.push(Step0Row::new(
            format!("{n} × {kind:?} ({sev})"),
            blocker_handoff(kind),
        ));
    }

    // ── Imports per venue. Informational: it is how a filer notices a missing export. ────────────
    //
    // ★ DERIVED FROM THE EVENTS, never from a typed list of venues: a source that stops being
    //   importable, or one that is added, needs no edit here.
    let mut per_venue: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in events {
        if let EventId::Import { source, .. } = &e.id {
            *per_venue.entry(source.tag()).or_insert(0) += 1;
        }
    }
    for (venue, n) in per_venue {
        panel.imports.push(Step0Row::new(
            format!("{venue}: {n} imported event(s)"),
            String::new(),
        ));
    }

    // ── Venues with dispositions in the year vs providers with a Form 1099-DA answer (J-4, J-7). ─
    //
    // ★★★ THE SAME KEYS `screen_broker_reporting` DEMANDS. `broker_key_census` over `form_8949` is
    //     the one derivation of *"which (provider, cohort) keys does this year have rows under"*, so
    //     the panel can never name a venue the screen would not ask about, or stay quiet about one
    //     it would.
    let rows = btctax_core::form_8949(state, year);
    let census = broker_key_census(&rows);
    let mut unanswered: BTreeMap<String, Vec<(Cohort, usize)>> = BTreeMap::new();
    let mut answered: BTreeSet<String> = BTreeSet::new();
    for ((provider, cohort), n) in &census {
        let has = ri
            .and_then(|r| r.broker_reporting.answer(provider, *cohort))
            .is_some();
        if has {
            answered.insert(provider.clone());
        } else {
            unanswered
                .entry(provider.clone())
                .or_default()
                .push((*cohort, *n));
        }
    }
    for (provider, cohorts) in &unanswered {
        let detail: Vec<String> = cohorts
            .iter()
            .map(|(c, n)| format!("{n} {} row(s)", c.slot_name()))
            .collect();
        panel.venues.push(Step0Row::new(
            format!(
                "{provider} has {} on this year's Form 8949 and NO Form 1099-DA answer on file — \
                 a venue you disposed on is not accounted for",
                detail.join(" and ")
            ),
            format!("btctax income answer --year {year}  (the Form 1099-DA block)"),
        ));
    }
    for provider in &answered {
        if !unanswered.contains_key(provider) {
            panel.venues.push(Step0Row::new(
                format!("{provider}: every cohort with rows this year is answered"),
                String::new(),
            ));
        }
    }

    // ── Standing orders — Notice 2026-20 §4.02(2) (J-9, J-10). ──────────────────────────────────
    //
    // ★★★ ONE ROW PER VENUE, keyed on that venue's FIRST custodial disposition of the year, because
    //     that is the moment §4.02(2) asks about: was a standing order recorded BEFORE the units
    //     were sold? A later election does not rescue the earlier sale, and an election can never be
    //     back-dated, so the date the filer needs to see is the earliest one.
    let mut first_custodial: BTreeMap<String, (btctax_core::TaxDate, WalletId)> = BTreeMap::new();
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in &d.legs {
            if !matches!(leg.wallet, WalletId::Exchange { .. }) {
                continue;
            }
            let key = leg.wallet.label();
            match first_custodial.get(&key) {
                Some((seen, _)) if *seen <= d.disposed_at => {}
                _ => {
                    first_custodial.insert(key, (d.disposed_at, leg.wallet.clone()));
                }
            }
        }
    }
    for (label, (date, wallet)) in &first_custodial {
        if btctax_core::standing_order_in_force(events, wallet, *date).is_none() {
            panel.standing_orders.push(Step0Row::new(
                format!(
                    "{label}: your first custodial disposition of {year} is dated {date}, and no \
                     standing order (a dated method election) was in force for it — \
                     {STANDING_ORDER_CONSEQUENCE}"
                ),
                format!(
                    "btctax config --set-forward-method <hifo|fifo|lifo> --exchange {label} \
                     (forward only — §1.1012-1(j) allows no post-hoc identification, so this \
                     cannot cover the {date} sale)"
                ),
            ));
        }
    }

    // ── Owner actions with dates (J-9b). ────────────────────────────────────────────────────────
    //
    // ★ §170(f)(11)(C)/(D): a donation over $5,000 of claimed deduction needs a QUALIFIED APPRAISAL,
    //   and CCA 202302012 removes the readily-valued exception for digital assets. The action is the
    //   OWNER'S — btctax cannot appraise — so it is listed with the removal's own date, which is what
    //   the appraisal has to be contemporaneous with.
    for r in state
        .removals
        .iter()
        .filter(|r| r.removed_at.year() == year && r.appraisal_required)
    {
        panel.owner_actions.push(Step0Row::new(
            format!(
                "a {} dated {} needs a QUALIFIED APPRAISAL (§170(f)(11)(C); CCA 202302012 — no \
                 readily-valued exception for digital assets). btctax cannot make it for you",
                match r.kind {
                    btctax_core::RemovalKind::Donation => "donation",
                    btctax_core::RemovalKind::Gift => "gift",
                },
                r.removed_at
            ),
            format!(
                "btctax reconcile set-donation-details {} --appraiser-name … --appraisal-date …",
                r.event.canonical()
            ),
        ));
    }

    panel
}

/// The widest a Step 0 line may be before it is wrapped for the TUI's status block.
///
/// ★★★ **The block does NOT wrap, it CLIPS.** `draw_tax_inputs_status` writes each line as one
/// `Line` and the terminal cuts it at the pane edge — so an unwrapped row named a venue and then
/// silently dropped the consequence, which is the failure this panel exists to prevent one level up.
/// 112 leaves room for the border and the two-space indent inside a 120-column pane, which is the
/// width the committed walkthrough goldens render at.
const TUI_WIDTH: usize = 112;

/// The most named rows the status block shows before it summarises the rest. A cap that hid rows
/// silently would be the truncation this codebase treats as a defect; the overflow line states the
/// count and where the full list is, so nothing disappears.
const TUI_MAX_NAMED: usize = 3;

/// Hard-wrap on whitespace at [`TUI_WIDTH`], continuation lines indented. A single word longer than
/// the width is emitted whole rather than cut — a truncated venue label is worse than a long line.
fn wrap(text: &str, indent: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let prefix = if out.is_empty() { "" } else { indent };
        if !line.is_empty() && prefix.len() + line.len() + 1 + word.len() > TUI_WIDTH {
            out.push(format!("{prefix}{line}"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        let prefix = if out.is_empty() { "" } else { indent };
        out.push(format!("{prefix}{line}"));
    }
    out
}

impl Step0Panel {
    /// ★★★ **The panel as the TUI's entry screen shows it** (R9's *"rendered as a TUI pane"*).
    ///
    /// The status block is a handful of rows and the full panel is unbounded, so this is a SUMMARY
    /// LINE plus the rows that NAME something the filer must act on — a venue with dispositions and
    /// no Form 1099-DA answer, and a venue whose first custodial sale had no standing order. Those
    /// two are R9's own *"is NAMED"* lists.
    ///
    /// ★★ **Nothing is hidden, and the lines say so.** The blocker rows are counted here and printed
    ///    in full by `btctax verify` and by `income answer`'s Step 0 print, and the summary names
    ///    both routes; a named list longer than [`TUI_MAX_NAMED`] ends with an explicit *"…and N
    ///    more"*. A summary that silently dropped rows would be the truncation this codebase treats
    ///    as a defect (harness B2), and a count with no route to the detail is the same thing
    ///    wearing a number.
    #[must_use]
    pub fn tui_lines(&self) -> Vec<String> {
        if self.rows() == 0 {
            return vec![format!(
                "Step 0 ({}): nothing outstanding on the ledger.",
                self.year
            )];
        }
        let named: Vec<&Step0Row> = self
            .venues
            .iter()
            .filter(|v| !v.handoff.is_empty())
            .chain(self.standing_orders.iter())
            .collect();
        let mut out = wrap(
            &format!(
                "Step 0 ({}): {} blocker kind(s) · {} venue(s) needing a Form 1099-DA answer · {} \
                 venue(s) with no standing order · {} owner action(s). `btctax verify` and `btctax \
                 income answer` print them in full.",
                self.year,
                self.blockers.len(),
                self.venues.iter().filter(|v| !v.handoff.is_empty()).count(),
                self.standing_orders.len(),
                self.owner_actions.len()
            ),
            "    ",
        );
        for row in named.iter().take(TUI_MAX_NAMED) {
            out.extend(wrap(&format!("• {}", row.what), "      "));
        }
        if named.len() > TUI_MAX_NAMED {
            out.push(format!(
                "  …and {} more — `btctax income answer` lists every one.",
                named.len() - TUI_MAX_NAMED
            ));
        }
        out
    }
}

/// Print the panel. `income answer` calls this BEFORE the census — the panel is STATUS, not a
/// question, so document-first order is untouched.
///
/// ★ No progress bar and no "N of M" (R15): it lists ITEMS.
pub fn write_step0(out: &mut impl Write, p: &Step0Panel) -> std::io::Result<()> {
    writeln!(out, "\n── Step 0: your ledger, for {} ──", p.year)?;
    if p.rows() == 0 {
        writeln!(
            out,
            "  nothing outstanding: no blockers, no imports, no venue without an answer."
        )?;
        return Ok(());
    }
    let section = |title: &str, rows: &[Step0Row], out: &mut dyn Write| -> std::io::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        writeln!(out, "  {title} ({}):", rows.len())?;
        for r in rows {
            writeln!(out, "    • {}", r.what)?;
            if !r.handoff.is_empty() {
                writeln!(out, "      → {}", r.handoff)?;
            }
        }
        Ok(())
    };
    section("LEDGER BLOCKERS", &p.blockers, out)?;
    section("IMPORTS BY VENUE", &p.imports, out)?;
    section("VENUES vs FORM 1099-DA ANSWERS", &p.venues, out)?;
    section(
        "STANDING ORDERS (Notice 2026-20 §4.02(2))",
        &p.standing_orders,
        out,
    )?;
    section("OWNER ACTIONS", &p.owner_actions, out)?;
    writeln!(
        out,
        "  (Step 0 is STATUS, not a question — authoring works with an unresolved ledger; \
         committing and exporting do not.)"
    )?;
    Ok(())
}

/// ★★★ **R9 — VENUE / ACCOUNT GRANULARITY IS DOCUMENTED, NOT ASKED.**
///
/// btctax models one account per provider: `normalize.rs` hardcodes the account segment to
/// `default` (`crates/btctax-adapters/src/normalize.rs:63-69`), so every `WalletId::Exchange` this
/// product can build is `exchange:<provider>:default`. Asking *"how many accounts do you hold at
/// Coinbase?"* would therefore collect an answer nothing reads — a stored value with no reader,
/// which is the shape this codebase treats as a defect rather than a feature.
///
/// It matters for one rule and the panel says so where it matters: §1012(c)(1) applies the basis
/// conventions ACCOUNT BY ACCOUNT, so a filer with two accounts at one broker has two standing-order
/// scopes and btctax has one. The honest statement is this sentence, printed beside the standing-order
/// list; the field arrives when the adapters can tell the accounts apart.
pub const VENUE_GRANULARITY_NOTE: &str =
    "btctax models ONE account per venue (`exchange:<venue>:default`). §1012(c)(1) applies the \
     basis conventions account by account, so if you hold more than one account at a venue, the \
     standing order above is recorded for the venue and not per account — check that your broker \
     applied the same method to each.";
