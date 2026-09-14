//! ★ Design r2 §6 / port report D7 — **`YearReadiness`: what a year DECLARES versus what this build
//! actually CARRIES, as one type.** The forms crate holds the declaration (`YearRecord`) and the
//! glob; the adapters crate holds the tables, the full-return params and the price dataset; only this
//! crate can see all of them — so this is where "is TY2026 ready?" gets one answer, rendered on every
//! number-bearing surface and used to build every refusal string, instead of five literals that drift
//! (`selected_year: 2025`, "2017, 2024 and 2025 only", …). The default year is the NEWEST bundled
//! record, which since spec 1099-DA T0 is a `preparing` TY2026 with zero forms — deliberately.
//!
//! The compute gate stays `full_return_for(year)`; this type never makes a year computable. What it
//! adds is the declared/actual COMPARISON, with kills: a year declared `filable` whose params are not
//! bundled, or whose price dataset stops before `prices_through`, reds.
use btctax_adapters::price::BundledPrices;
// ★★★ FR-225 — ONE verdict type, defined beside the walk that also consumes it
// (`btctax_core::tax::interview_state`). A second enum here would be a second thing to keep in step,
// and the whole finding is two surfaces disagreeing about one return.
pub use btctax_core::tax::interview_state::ReturnVerdict;
use btctax_core::tax::tables::{FullReturnTables, TaxTables};
use btctax_forms::bundled::{bundled_years, BUNDLED};
use btctax_forms::year_record::{YearRecord, YearStatus};

/// One year, declared versus actual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearReadiness {
    /// The tax year asked about.
    pub year: i32,
    /// The declaration, when this build bundles the year at all.
    pub declared: Option<YearRecord>,
    /// A `TaxTable` is bundled (the crypto slice and `report` compute).
    pub table: bool,
    /// `FullReturnParams` are bundled (the full return computes) — the compute gate.
    pub params: bool,
    /// How many forms the build binds for the year (0 = no year directory).
    pub forms_bundled: usize,
    /// The newest close in the bundled price dataset.
    pub prices_max_date: Option<btctax_core::conventions::TaxDate>,
}

impl YearReadiness {
    /// Assemble from the live registries.
    pub fn for_year(
        year: i32,
        tables: &dyn TaxTables,
        full: &dyn FullReturnTables,
        prices: &BundledPrices,
    ) -> Self {
        Self {
            year,
            declared: YearRecord::for_year(year),
            table: tables.table_for(year).is_some(),
            params: full.full_return_for(year).is_some(),
            forms_bundled: BUNDLED.iter().filter(|(_, y)| *y == year).count(),
            prices_max_date: prices.max_date(),
        }
    }

    /// The declared/actual disagreements — empty when the declaration is honest. Each string is a
    /// complete sentence a refusal can print.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Some(d) = &self.declared else {
            if self.forms_bundled > 0 || self.table || self.params {
                out.push(format!(
                    "TY{}: the build carries assets for this year but forms/{}/YEAR.toml declares nothing",
                    self.year, self.year
                ));
            }
            return out;
        };
        match d.status {
            YearStatus::Filable => {
                if !self.params {
                    out.push(format!(
                        "TY{}: declared filable but no FullReturnParams are bundled — the full return cannot compute",
                        self.year
                    ));
                }
                if !self.table {
                    out.push(format!(
                        "TY{}: declared filable but no TaxTable is bundled",
                        self.year
                    ));
                }
                match self.prices_max_date {
                    Some(max) if max >= d.prices_through => {}
                    Some(max) => out.push(format!(
                        "TY{}: declared filable but the bundled price dataset ends {max}, before prices_through {}",
                        self.year, d.prices_through
                    )),
                    None => out.push(format!("TY{}: declared filable with an empty price dataset", self.year)),
                }
            }
            YearStatus::Slice | YearStatus::Preparing => {
                if d.status == YearStatus::Slice && !self.table {
                    out.push(format!(
                        "TY{}: declared a crypto-slice year but no TaxTable is bundled — the slice cannot compute",
                        self.year
                    ));
                }
                if self.params {
                    out.push(format!(
                        "TY{}: FullReturnParams are bundled but the year declares itself {:?} — declare it filable or unbundle them",
                        self.year, d.status
                    ));
                }
            }
        }
        if d.forms_expected.len() != self.forms_bundled {
            out.push(format!(
                "TY{}: declares {} expected forms but the build binds {}",
                self.year,
                d.forms_expected.len(),
                self.forms_bundled
            ));
        }
        out
    }

    /// One line for a status surface: `"TY2025 — preparing (15 forms; TaxTable yes; full-return params no)"`.
    pub fn sentence(&self) -> String {
        match &self.declared {
            None => format!(
                "TY{} — not bundled (this build bundles {})",
                self.year,
                btctax_forms::bundled::years_sentence()
            ),
            Some(d) => format!(
                "TY{} — {} ({} forms; TaxTable {}; full-return params {}; 1099-DA {})",
                self.year,
                match d.status {
                    YearStatus::Preparing => "preparing",
                    YearStatus::Slice => "crypto slice only",
                    YearStatus::Filable => "filable",
                },
                self.forms_bundled,
                if self.table { "yes" } else { "no" },
                if self.params { "yes" } else { "no" },
                // spec 1099-DA T6 — the year's information-return regime, from its record
                match (
                    d.information_returns.f1099da.proceeds,
                    d.information_returns.f1099da.basis,
                ) {
                    (false, _) => "none",
                    (true, false) => "proceeds",
                    (true, true) => "proceeds+basis",
                },
            ),
        }
    }
}

impl YearReadiness {
    /// Assemble from the BUNDLED registries (what this binary ships). The price dataset is optional:
    /// a build whose dataset fails to load still answers the table/params questions honestly, with
    /// `prices_max_date = None`.
    pub fn bundled(year: i32) -> Self {
        use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};
        let tables = BundledTaxTables::load();
        let full = BundledFullReturnTables::load();
        Self {
            year,
            declared: YearRecord::for_year(year),
            table: tables.table_for(year).is_some(),
            params: full.full_return_for(year).is_some(),
            forms_bundled: BUNDLED.iter().filter(|(_, y)| *y == year).count(),
            prices_max_date: BundledPrices::load().ok().and_then(|p| p.max_date()),
        }
    }
}

/// ★★★ **FR-225 / FR-63 — a refusal's VARIANT NAME, without its `Debug` payload.**
///
/// `btctax report` prints `NOT COMPUTABLE [{reason:?}]` on a surface that scrolls, so it can afford the
/// whole payload. [`EntryStates::lines`] cannot: its lines are drawn into a fixed 118-column pane that
/// does not wrap, and `DigitalAssetAnswerContradictsLedger { date: …, venue: …, kind: … }` measured
/// **164 columns** — it would have been clipped mid-payload, which is FR-63.
///
/// ★★ **So the two surfaces agree on the refusal's IDENTITY rather than on its rendering**, which is
/// the thing a filer matches between them and the thing
/// `tests::the_printed_claim_agrees_with_the_return_verdict` asserts. The payload is not dropped from
/// the product — it rides the `detail`, which the answer panel and `report` both print in full.
///
/// ★ Taken from `Debug` rather than from a hand-written `RefuseReason → &str` map, and that is the
///   FR-99 rule: a map would be a list of 126 names beside an enum that grows, and it would go stale
///   silently. `Debug`'s first token IS the variant name, for every shape a derive can produce.
fn refuse_reason_name(r: &btctax_core::tax::return_refuse::RefuseReason) -> String {
    let d = format!("{r:?}");
    d.split([' ', '{', '('])
        .next()
        .unwrap_or(d.as_str())
        .to_string()
}

/// ★★★ **T4 / `SPEC_interview.md` R11 — THE STATES A YEAR IS IN AT ENTRY.**
///
/// They are independent, and conflating them is the defect R11 exists to fix. A filer opening
/// TY2026 in September 2026 can finish the whole interview and still not be able to file, because
/// the IRS has not published the year's numbers — and a screen that says only *"not ready"* tells
/// them their work is pointless, while one that says only *"complete"* promises a return they
/// cannot get.
///
/// - **interview-complete** — [`btctax_core::tax::interview_state::interview_state`] (T3/R12) has
///   nothing blocking. Derivable with no package at all: it is a fact about the ANSWERS.
/// - **package-computes** — [`YearReadiness::params`]: this build bundles the year's
///   `FullReturnParams`. A fact about the BUILD.
/// - **the return's verdict** — [`ReturnVerdict`]: did the refusal chain, run on THIS return, refuse?
///   A fact about the RETURN, and the only one of the three that licenses the word *computable*.
///
/// ★★★ **FR-225 — THE THIRD STATE IS NEW, AND ITS ABSENCE WAS A LIVE DEFECT ON TY2024.** R11 named
/// two states and the second was rendered as `return: computable` — but `YearReadiness::params` says
/// *this build can compute the year*, which is not *this return computes*. A charitable gift whose
/// §170(f)(8)(A) acknowledgment is unresolved refuses at `screen_absolute`, so `btctax income answer`
/// printed **`interview: complete · return: computable`** and exited 0 while `btctax report` printed
/// **`NOT COMPUTABLE [CharitableCwaUnresolved]`** and `export-irs-pdf` wrote nothing. Two instruments
/// in one binary, over one stored return, disagreeing about whether it can be filed.
///
/// ★★ **The two questions are now answered by two fields, and the word `computable` is derived from
/// the verdict alone** — so the claim and the return's own answer cannot diverge by construction, not
/// merely by care. [`Self::claims_computable`] exposes the claim for the agreement kill.
///
/// ★ `interview_complete` is `None` when there is no return to ask about (a year the filer has not
///   started), so the sentence says *"not started"* rather than asserting either state of a return
///   that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryStates {
    pub year: i32,
    /// `Some(true)` = nothing blocking; `None` = no working return for the year yet.
    pub interview_complete: Option<bool>,
    /// How many class-(A) items still block, when there is a return. Named so the sentence can say
    /// what is left instead of only that something is.
    pub blocking: Option<usize>,
    /// ★★★ FR-225 — [`YearReadiness::params`]: does THIS BUILD bundle the year's `FullReturnParams`?
    ///
    /// **Renamed from `return_computable`, which is what the defect was.** This is a fact about the
    /// build, it is the compute gate everywhere else, and it says nothing whatever about whether one
    /// particular return computes — for that, and for the printed word, see [`Self::verdict`].
    pub package_computes: bool,
    /// ★★★ FR-225 — the RETURN'S own verdict, where the caller ran the chain
    /// ([`Self::with_return_verdict`]). [`ReturnVerdict::NotRun`] by default: a surface that has not
    /// computed the return says so rather than claiming it computes.
    pub verdict: ReturnVerdict,
    /// The readiness line for the year, for the second clause.
    readiness: String,
}

impl EntryStates {
    /// Both states for `year`, over the working return `ri` (the draft or the committed row —
    /// whichever the caller resolved; `None` when the year has neither).
    #[must_use]
    pub fn for_year(year: i32, ri: Option<&btctax_core::tax::return_inputs::ReturnInputs>) -> Self {
        Self::package_only(year).with_interview(ri)
    }

    /// The PACKAGE half only — the half that cannot change while a form is open.
    ///
    /// ★ Split out because [`YearReadiness::bundled`] reads the bundled price dataset, and a TUI
    ///   redraw happens on every keystroke: the build fact is cached once at open and the interview
    ///   half — a registry walk, cheap — is recomputed per frame by [`Self::with_interview`], so the
    ///   line the filer reads tracks the answer they just gave.
    #[must_use]
    pub fn package_only(year: i32) -> Self {
        let r = YearReadiness::bundled(year);
        Self {
            year,
            interview_complete: None,
            blocking: None,
            package_computes: r.params,
            verdict: ReturnVerdict::NotRun,
            readiness: r.sentence(),
        }
    }

    /// ★★★ **FR-225 — hand in the RETURN'S OWN VERDICT, from a caller that computed it.**
    ///
    /// The chain is `screen_inputs` → `screen_compute_dependent` → `assemble_absolute` +
    /// `screen_absolute`, exactly as `btctax report` composes it (`cmd/tax.rs`), and only a caller
    /// holding the year's `TaxTable`, the ledger `LedgerState` and the year's information-return regime
    /// can run it. `btctax income answer` does ([`crate::cmd::answer::return_verdict`]); the TUI's
    /// tax-inputs pane does not — it holds no ledger — so it stays on [`ReturnVerdict::NotRun`] and its
    /// line says so instead of over-claiming.
    #[must_use]
    pub fn with_return_verdict(mut self, verdict: ReturnVerdict) -> Self {
        self.verdict = verdict;
        self
    }

    /// Fill the interview half over the working return (`None` = the year has no return yet).
    #[must_use]
    pub fn with_interview(
        mut self,
        ri: Option<&btctax_core::tax::return_inputs::ReturnInputs>,
    ) -> Self {
        let st = ri.map(btctax_core::tax::interview_state::interview_state);
        self.interview_complete = st.as_ref().map(|s| s.blocking.is_empty());
        self.blocking = st.as_ref().map(|s| s.blocking.len());
        self
    }

    /// The entry states as DISPLAY LINES — one when the year computes, three when it does not.
    ///
    /// ★★ Lines rather than one string because the TUI's status pane is a fixed 118 columns and
    ///    R11's sentence alone is 78 of them: a single line would be silently clipped, which is the
    ///    FR-63 defect (*"a fact that will not fit is added BESIDE it rather than by lengthening
    ///    it"*). [`Self::sentence`] joins them for surfaces that scroll.
    ///
    /// ★★ On a year with no package line 2 is R11's own words — *"authoring and saving work;
    ///    computing and committing wait for the TY{year} package"* — because that is the fact a
    ///    filer needs in September and no other surface says it, and line 3 is the readiness
    ///    sentence, which says WHAT is missing.
    ///
    /// ★ It deliberately does NOT name a month the package is expected in. `YearReadiness` exists
    ///   precisely to replace year literals that drift, and no bundled record declares a package
    ///   date (`YearRecord` carries `return_due` and `prices_through`, neither of which is it), so a
    ///   hand-typed *"expected Jan 2027"* would be the one unsourced figure on the screen.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let interview = match (self.interview_complete, self.blocking) {
            (None, _) => "interview: not started".to_string(),
            (Some(true), _) => "interview: complete".to_string(),
            (Some(false), Some(n)) => format!("interview: {n} question(s) still to answer"),
            (Some(false), None) => "interview: incomplete".to_string(),
        };
        // ★★★ **FR-225 — THE PACKAGE HALF FIRST, because it is a different sentence.** A year with no
        //     `FullReturnParams` cannot compute for ANY return, so there is no return-level verdict to
        //     report and R11's own three lines stand unchanged (byte-identical: this is the dominant
        //     TY2026 case, and its wording was adjudicated at T4).
        if !self.package_computes {
            return vec![
                format!("{interview} · return: NOT computable"),
                format!(
                    "authoring and saving work; computing and committing wait for the TY{} package",
                    self.year
                ),
                self.readiness.clone(),
            ];
        }
        // ★★★ **FR-225 — AND THEN THE RETURN'S OWN VERDICT, which is the only thing that may print the
        //     word `computable`.** Derived from [`ReturnVerdict`] and from nothing else, so the claim
        //     cannot drift from the answer `btctax report` gives over the same stored return — the
        //     divergence FR-225 recorded, live, on the year that files.
        match &self.verdict {
            ReturnVerdict::Computes => vec![format!("{interview} · return: computable")],
            // ★★ The refusal is NAMED — the same reason code `btctax report` prints, so a filer can
            //    match the two surfaces — and pointed at the surfaces that carry its own exit
            //    sentence verbatim.
            //
            // ★ **The detail is deliberately NOT inlined here, and that is FR-63's rule rather than
            //   brevity.** These are DISPLAY LINES for a fixed 118-column pane that does not wrap
            //   (`draw_edit.rs` draws each one as a single `Line`), and the §170(f)(8)(A) detail is
            //   ~1,400 characters — it would be silently clipped, which is exactly the *"a fact that
            //   will not fit is added BESIDE it rather than by lengthening it"* defect. The panel
            //   (`refusing_lines`) and `report` both print it in full, on surfaces that scroll.
            ReturnVerdict::Refuses { reason, detail: _ } => vec![
                format!(
                    "{interview} · return: NOT computable [{}]",
                    refuse_reason_name(reason)
                ),
                format!(
                    "the answer panel and `btctax report --tax-year {year}` print it in full, \
                     with its cure",
                    year = self.year
                ),
            ],
            // ★★★ **THE HONEST BOUNDARY, and it is the third rule of "derive the list or state what it
            //     covers".** The TUI's tax-inputs pane holds no `LedgerState`, so it cannot run
            //     `screen_compute_dependent` or `screen_absolute` and genuinely does not know whether
            //     this return computes. Before FR-225 it said `return: computable` anyway. It now says
            //     that it has not looked, and names the command that has — a weaker claim, and a true
            //     one. (That the BUILD can compute the year is already implied: the `!package_computes`
            //     branch above owns the other case, in R11's own three lines.)
            //
            // ★★ **ONE line, and the WIDTH is the reason.** A two-line version of this shipped for
            //    exactly one test run: `docs/examples-tui-walkthrough/j6/01` came back with the second
            //    line CLIPPED at the pane's 118th column — *"…, which run"* — and one section row
            //    pushed off the list to make room. That is FR-63 verbatim, committed on the very
            //    function whose own doc cites it, and the GOLDEN is what caught it. See
            //    [`tests::the_entry_lines_fit_the_fixed_pane`], which now holds the budget for every
            //    arm so the next edit here cannot reintroduce it silently.
            ReturnVerdict::NotRun => vec![format!(
                "{interview} · return: not checked here (`btctax report --tax-year {year}` \
                 decides it)",
                year = self.year
            )],
        }
    }

    /// ★★★ **FR-225 — DOES THIS SURFACE CLAIM THE RETURN COMPUTES?** Read off the RENDERED lines, not
    /// off the predicate, because the defect was that a filer read the word.
    ///
    /// ★★ It exists so the agreement can be ASSERTED rather than reasoned about:
    /// `claims_computable() == verdict.computes()` must hold for every combination of package state,
    /// interview state and verdict — which is the invariant
    /// `the_printed_claim_agrees_with_the_return_verdict` pins, and it reds wherever a future edit
    /// re-derives the word from anything but the verdict.
    #[must_use]
    pub fn claims_computable(&self) -> bool {
        self.lines()
            .iter()
            .any(|l| l.contains("return: computable"))
    }

    /// [`Self::lines`] as one sentence, for a surface that is not column-bound.
    #[must_use]
    pub fn sentence(&self) -> String {
        self.lines().join(" — ")
    }
}

/// ★★★ spec 1099-DA R6 fold (I-1) — can `export-irs-pdf --tax-year {year}` print the crypto slice
/// AT ALL, template-wise?
///
/// `full_return_for(year).is_none()` is what makes the slice the *right* artifact for a year; it is
/// not what makes the slice *printable*. Printing needs the year's own form templates bundled, which
/// `slice_map_gate` and the `SUPPORTED_YEARS` refusal enforce at export time. **TY2026 is exactly
/// the year where the two diverge** — `forms/2026/` holds only `YEAR.toml`, so no full return
/// computes (the slice clause fired) while `Form8949Map::for_year(2026)` fails (the export refused).
/// The one year the whole feature exists for was being promised an artifact it could not get.
///
/// Form 8949 and Schedule D are the two forms the slice ALWAYS writes (`wants()` defaults to every
/// applicable form), so they are the right two to probe: if either map is missing, nothing prints.
pub fn slice_can_print(year: i32) -> bool {
    btctax_forms::Form8949Map::for_year(year).is_ok()
        && btctax_forms::ScheduleDMap::for_year(year).is_ok()
}

/// ★★★ spec 1099-DA R6 fold (I-1 + M-4) — **THE predicate for "tell this filer about the crypto
/// slice", shared by every surface that says so**, so no two of them can disagree about whether the
/// artifact exists: the answers are stored AND the year's own Form 8949 / Schedule D templates are
/// bundled.
///
/// Both terms are load-bearing and each was once missing. Without `answers_stored` (M-4) the
/// sentence asserted "from the stored answers" on a year holding none; without [`slice_can_print`]
/// (I-1) it promised an artifact `export-irs-pdf` would refuse — TY2026 is exactly that year.
///
/// ★ It deliberately does NOT re-check that the year's full-return parameters are absent. Every
/// caller is already inside a params-less branch by construction — [`uncomputable_sentence`] runs
/// only when the return did not compute, [`import_note`] returns early when `r.params`, and the TUI
/// input form's `CommitOutcome::NoTables` arm IS the no-parameters outcome. Re-deriving it here
/// would be a second answer to a question the caller has already answered.
pub fn slice_prints_from_answers(year: i32, answers_stored: bool) -> bool {
    answers_stored && slice_can_print(year)
}

/// ★ spec 1099-DA R6 fold (I-1 + M-4) — the slice clause the two readiness sentences carry, under
/// [`slice_prints_from_answers`]. Empty when the predicate is false — saying nothing beats promising
/// an artifact that refuses.
fn slice_clause(year: i32, answers_stored: bool) -> String {
    if slice_prints_from_answers(year, answers_stored) {
        format!(
            "`export-irs-pdf --tax-year {year}` still prints the crypto slice from the stored answers. "
        )
    } else {
        String::new()
    }
}

/// ★ FR-48 / port report §2.5 — the `report --tax-year` sentence for a year that HOLDS full-return
/// inputs but cannot compute them, built from readiness instead of a year literal. It says what is
/// missing, that the inputs are KEPT (a computed carryover written onto a not-yet-ready year is a
/// legitimate row), and names `income clear` only as the way to fall back to a raw tax-profile —
/// with its cost — never as "the" remedy. The previous text said "v1 supports TY2024" and prescribed
/// `income clear` outright, which deletes the filer's W-2s.
///
/// `answers_stored`: does the year's working `ReturnInputs` carry a non-empty `broker_reporting`?
/// The slice clause is CONDITIONAL on it (M-4) and on the year's templates (I-1) — see
/// [`slice_clause`]. Both terms were missing: the sentence asserted "from the stored answers"
/// unconditionally, on years holding none and on years that cannot print.
pub fn uncomputable_sentence(year: i32, answers_stored: bool) -> String {
    let r = YearReadiness::bundled(year);
    format!(
        "tax year {year} has full-return inputs, but full-return computation is not available for it \
         in this build — {}. The inputs are KEPT and will compute when the year's package is bundled. \
         {}To fall back to a raw `tax-profile` for {year} instead, run `income clear --year {year}` \
         (this DISCARDS the stored inputs, including any computed carryover on them).",
        r.sentence(),
        slice_clause(year, answers_stored)
    )
}

/// ★ FR-48 — the note `income import` prints (stderr) when it stores inputs for a year whose full
/// return cannot compute yet. It stores rather than refuses: `report --tax-year N-1 --write-carryover`
/// legitimately writes onto year N before N's package exists, and the TUI keeps the same thing as a
/// draft. `None` when the year is ready (nothing to say).
///
/// `answers_stored`: does the `ReturnInputs` being imported carry a non-empty `broker_reporting`?
/// Same conditional slice clause as [`uncomputable_sentence`] (R6 fold I-1 + M-4).
pub fn import_note(year: i32, answers_stored: bool) -> Option<String> {
    let r = YearReadiness::bundled(year);
    if r.params {
        return None;
    }
    Some(format!(
        "note: {} — these inputs are stored now; `report --tax-year {year}` computes the crypto delta \
         on a stored `tax-profile` and reports the full return as NOT COMPUTABLE (keeping the inputs) \
         until full-return parameters for {year} are bundled. {}",
        r.sentence(),
        slice_clause(year, answers_stored).trim_end()
    ))
}

/// ★ FR-48 — the `export-snapshot --tax-year` STAMP, written into the export directory so a
/// `form8949.csv` can no longer be byte-identical across years with nothing saying which year it is.
///
/// Deliberately NOT a gate (port report D7 asked for "gate it and stamp it"): `export-snapshot` is a
/// DATA export — lots, disposals, the 8949 rows, the promoted-tranche disclosure — and it is valid
/// for any year the ledger holds events for, including a filed-tranche year like TY2020 that this
/// build bundles no table for (`declare_tranche_cli::filed_tranche_year_exports_clean`). What the
/// stamp adds is the readiness sentence beside the data: whether this build could COMPUTE the year.
pub fn export_stamp(year: i32) -> String {
    YearReadiness::bundled(year).sentence()
}

/// The year a fresh interactive surface opens on: the NEWEST bundled year — derived from the glob,
/// never a literal. (The TUIs edit any bundled year; "newest" is where a new return starts.)
/// ★ The Form 1099-DA regime for a year, JOINED from the year record (spec 1099-DA T0). `None` for a
/// year with no bundled record — the caller refuses, never assumes.
pub fn regime_for(year: i32) -> Option<btctax_core::InformationReturnRegime> {
    let r = YearRecord::for_year(year)?;
    Some(btctax_core::InformationReturnRegime {
        proceeds: r.information_returns.f1099da.proceeds,
        basis: r.information_returns.f1099da.basis,
    })
}

/// [`regime_for`], or the refusal a command gives when the year has no record — the regime is a fact
/// about the year and a command must never guess it (spec 1099-DA R1).
pub fn regime_or_refuse(
    year: i32,
) -> Result<btctax_core::InformationReturnRegime, crate::CliError> {
    // a year with no record is exactly an UNSUPPORTED year — the same typed refusal the export arms
    // raise before any byte, so the two gates cannot disagree about what "unsupported" means
    regime_for(year)
        .ok_or_else(|| crate::CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(year)))
}

/// [`regime_for`], with the ONE case a missing record decides on its own: a year BEFORE the
/// digital-asset box revision (`DIGITAL_ASSET_8949_FIRST_YEAR`) had no Form 1099-DA at all, so its
/// regime is `NONE` whether or not a record exists — the same fact the constant encodes, and what
/// lets `export-snapshot` write a filed-tranche TY2020's CSVs. A later year with no record is the
/// typed `UnsupportedYear` refusal: its regime is unknown and a CSV box must not guess it.
pub fn regime_or_pre_regime(
    year: i32,
) -> Result<btctax_core::InformationReturnRegime, crate::CliError> {
    match regime_for(year) {
        Some(r) => Ok(r),
        None if year < btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR => {
            Ok(btctax_core::InformationReturnRegime::NONE)
        }
        None => Err(crate::CliError::FormFill(
            btctax_forms::FormsError::UnsupportedYear(year),
        )),
    }
}

/// ★ spec 1099-DA R6 (M-5/M-7) — the EXPORT-TIME price-coverage check: does the bundled daily-close
/// dataset actually reach the end of the year being filed?
///
/// [`YearReadiness::problems`] asks the same question, but it is a STATIC readiness report and it
/// asks it only of a year declared `filable`. This is the gate: it runs in BOTH `export-irs-pdf`
/// arms before any byte, on whatever year the filer named, because a packet whose prices stop in
/// June is a return computed from a partial year — a wrong number on a signed form, not a
/// readiness nuance. The readiness surfaces (`sentence()`, the unlock screen, `year_record` tests)
/// gain NOTHING from this: no new `problems()` row, no new sentence clause.
///
/// A year with no bundled record has no `prices_through` to compare, so there is nothing to say —
/// the export's own year gates (`regime_or_refuse`, `SUPPORTED_YEARS`) answer that year.
pub fn price_coverage_or_refuse(year: i32) -> Result<(), crate::CliError> {
    let r = YearReadiness::bundled(year);
    match price_coverage_problem(
        year,
        r.declared.as_ref().map(|d| d.prices_through),
        r.prices_max_date,
    ) {
        Some(msg) => Err(crate::CliError::Usage(msg)),
        None => Ok(()),
    }
}

/// The pure half of [`price_coverage_or_refuse`] — `Some(sentence)` when the dataset stops before
/// the year's declared `prices_through`. Split out so the kill can plant a short dataset without a
/// bundled year that has one.
pub(crate) fn price_coverage_problem(
    year: i32,
    prices_through: Option<btctax_core::conventions::TaxDate>,
    prices_max_date: Option<btctax_core::conventions::TaxDate>,
) -> Option<String> {
    let through = prices_through?;
    let tail = "No forms were written. Update the bundled daily-close dataset (`scripts/` — the \
                price updater appends closes) and re-export.";
    match prices_max_date {
        Some(max) if max >= through => None,
        Some(max) => Some(format!(
            "cannot export TY{year}: the bundled price dataset ends {max}, before TY{year}'s \
             prices_through {through} — every figure on the packet would be computed from a PARTIAL \
             year. {tail}"
        )),
        None => Some(format!(
            "cannot export TY{year}: this build carries an EMPTY price dataset, so no disposition in \
             {year} can be valued. {tail}"
        )),
    }
}

pub fn default_year() -> i32 {
    *bundled_years()
        .iter()
        .max()
        .expect("the build bundles at least one year")
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};

    /// ★★★ spec 1099-DA R6 (M-5/M-7) KILL — the EXPORT-TIME price-coverage gate.
    ///
    /// Two halves, and both are the test. The pure half plants a dataset that stops mid-year, which
    /// no bundled year has; the live half calls the real gate on real years — TY2024 and TY2025 pass
    /// (the bundled dataset reaches into 2026), TY2026 does NOT, because its `prices_through` is
    /// 2026-12-31 and the dataset stops before it. Without the passing half a gate that refused
    /// every year would satisfy the refusal.
    #[test]
    fn the_export_time_price_gate_refuses_a_dataset_that_stops_short() {
        use time::macros::date;
        // planted: the dataset ends in June, the year runs to December.
        let msg = price_coverage_problem(
            2026,
            Some(date!(2026 - 12 - 31)),
            Some(date!(2026 - 06 - 03)),
        )
        .expect("a short dataset must refuse");
        assert!(
            msg.contains("ends 2026-06-03")
                && msg.contains("prices_through 2026-12-31")
                && msg.contains("PARTIAL"),
            "the refusal names both dates and the harm: {msg}"
        );
        // an EMPTY dataset is its own sentence
        assert!(
            price_coverage_problem(2026, Some(date!(2026 - 12 - 31)), None)
                .is_some_and(|m| m.contains("EMPTY price dataset"))
        );
        // covered → silent; a year with no record has no `prices_through` to compare → silent
        assert!(price_coverage_problem(
            2024,
            Some(date!(2024 - 12 - 31)),
            Some(date!(2026 - 06 - 03))
        )
        .is_none());
        assert!(price_coverage_problem(2099, None, Some(date!(2026 - 06 - 03))).is_none());

        // …and the LIVE gate, on the real record + the real bundled dataset.
        price_coverage_or_refuse(2024).expect("TY2024's prices are complete in this build");
        price_coverage_or_refuse(2025).expect("TY2025's prices are complete in this build");
        assert!(
            price_coverage_or_refuse(2026).is_err(),
            "TY2026 runs past the bundled dataset — an export of it may not print"
        );
    }

    /// ★ …and the READINESS SURFACES gain nothing from it (R6: `problems`, `sentence`, the unlock
    /// screen and the `year_record` tests are UNCHANGED). TY2025's dataset is complete, so this
    /// pins the shape rather than the value: no `problems()` row mentions the export-time gate.
    #[test]
    fn the_price_gate_adds_no_readiness_problem() {
        for year in btctax_forms::bundled::bundled_years() {
            let r = YearReadiness::bundled(*year);
            assert!(
                !r.problems().iter().any(|p| p.contains("PARTIAL year")),
                "TY{year}: the export-time gate must not leak into the readiness report"
            );
            assert!(
                !r.sentence().contains("PARTIAL"),
                "TY{year}: {}",
                r.sentence()
            );
        }
    }

    /// ★ THE KILL: every bundled year's declaration agrees with what the build carries. A year
    /// declared `filable` without params, or with a price dataset that stops short, reds here.
    #[test]
    fn every_bundled_years_declaration_agrees_with_the_build() {
        let tables = BundledTaxTables::load();
        let full = BundledFullReturnTables::load();
        let prices = BundledPrices::load().expect("the bundled price dataset loads");
        for &year in bundled_years() {
            let r = YearReadiness::for_year(year, &tables, &full, &prices);
            assert!(
                r.problems().is_empty(),
                "TY{year}: {:#?}\n{}",
                r.problems(),
                r.sentence()
            );
        }
        // and a year the build does not bundle says so, naming the years it does
        let r = YearReadiness::for_year(2031, &tables, &full, &prices);
        assert!(r.declared.is_none() && r.problems().is_empty());
        assert!(r.sentence().contains("not bundled"));
    }

    /// B1 — the declared/actual kill observed red: a `filable` declaration with no params.
    #[test]
    fn a_filable_declaration_without_params_is_reported() {
        let tables = BundledTaxTables::load();
        let full = BundledFullReturnTables::load();
        let prices = BundledPrices::load().expect("the bundled price dataset loads");
        let mut r = YearReadiness::for_year(2024, &tables, &full, &prices);
        assert!(r.problems().is_empty(), "the reference year is clean");
        r.params = false; // plant: the params vanish
        let p = r.problems();
        assert!(
            p.iter()
                .any(|m| m.contains("declared filable but no FullReturnParams")),
            "{p:?}"
        );
        // …and the price-dataset kill: a filable year whose dataset stops before prices_through.
        let mut r = YearReadiness::for_year(2024, &tables, &full, &prices);
        r.prices_max_date = Some(time::macros::date!(2024 - 06 - 01));
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("before prices_through")));
        // …a `slice` year with no table (R11). ★ Re-pointed 2026-09-06: TY2017 was the only
        // bundled `slice` year and S9 dropped its package, so the declaration is CONSTRUCTED here
        // from TY2025's committed record rather than deleted with the year that happened to carry
        // it — the branch is live code and a future partial year is exactly what it is for.
        let slice_record = btctax_forms::year_record::YearRecord::parse(
            &btctax_forms::bundled::year_record_text(2025)
                .expect("TY2025 has a committed record")
                .replace(
                    "status         = \"preparing\"",
                    "status         = \"slice\"",
                ),
        )
        .expect("the re-declared record parses");
        assert_eq!(
            slice_record.status,
            btctax_forms::year_record::YearStatus::Slice,
            "the plant must actually change the declared status, or this kill proves nothing"
        );
        let mut r = YearReadiness::for_year(2025, &tables, &full, &prices);
        r.declared = Some(slice_record);
        assert!(
            r.problems().is_empty(),
            "a slice year WITH its table is clean: {:?}",
            r.problems()
        );
        r.table = false;
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("crypto-slice year but no TaxTable")));
        // …and the reverse: params bundled for a year that says `preparing`.
        let mut r = YearReadiness::for_year(2025, &tables, &full, &prices);
        r.params = true;
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("declare it filable or unbundle them")));
    }

    /// FR-48: the report sentence names readiness, keeps the inputs, and names `income clear` only
    /// as the fallback with its cost — no year literal.
    #[test]
    fn the_uncomputable_sentence_is_built_from_readiness_and_keeps_the_inputs() {
        let s = uncomputable_sentence(2025, true);
        assert!(s.contains("full-return"), "{s}");
        assert!(s.contains("preparing"), "{s}");
        assert!(s.contains("inputs are KEPT"), "{s}");
        assert!(s.contains("income clear --year 2025"), "{s}");
        assert!(
            !s.contains("v1 supports"),
            "the stale literal must be gone: {s}"
        );
        let s = uncomputable_sentence(2031, true);
        assert!(s.contains("not bundled"), "{s}");
    }

    /// FR-48: import warns for a not-ready year and says nothing for a ready one.
    #[test]
    fn the_import_note_fires_only_for_a_year_without_params() {
        assert!(import_note(2024, true).is_none());
        let n = import_note(2025, true).unwrap();
        assert!(
            n.contains("stored now") && n.contains("report --tax-year 2025"),
            "{n}"
        );
        assert!(import_note(2031, true).unwrap().contains("not bundled"));
    }

    /// FR-48: export-snapshot stamps the year and its readiness; it does not refuse (a data export
    /// for a filed-tranche year with no table is legitimate).
    #[test]
    fn the_export_stamp_names_the_year_and_its_readiness_for_any_year() {
        assert!(export_stamp(2024).starts_with("TY2024 — filable"));
        assert!(export_stamp(2025).starts_with("TY2025 — preparing"));
        let s = export_stamp(2020);
        assert!(s.starts_with("TY2020 — not bundled"), "{s}");
    }

    /// ★ spec 1099-DA T0 — the join, per bundled year, and the value type's two flags.
    #[test]
    fn the_regime_is_joined_from_the_year_record() {
        use btctax_core::InformationReturnRegime;
        let r = |y| regime_for(y).unwrap();
        assert_eq!(
            r(2024),
            InformationReturnRegime {
                proceeds: false,
                basis: false
            }
        );
        assert_eq!(
            r(2025),
            InformationReturnRegime {
                proceeds: true,
                basis: false
            }
        );
        assert_eq!(
            r(2026),
            InformationReturnRegime {
                proceeds: true,
                basis: true
            }
        );
        assert!(
            regime_for(2023).is_none(),
            "a year with no record has no regime — refuse, never assume"
        );
    }

    /// A year before the box revision resolves to NONE without a record; a later one without a record
    /// is the typed refusal; a recorded year is its record.
    #[test]
    fn the_pre_regime_resolver_guesses_nothing_after_the_box_revision() {
        use btctax_core::InformationReturnRegime;
        assert_eq!(
            regime_or_pre_regime(2020).unwrap(),
            InformationReturnRegime::NONE
        );
        assert_eq!(
            regime_or_pre_regime(2025).unwrap(),
            InformationReturnRegime::PROCEEDS_ONLY
        );
        assert!(matches!(
            regime_or_pre_regime(2027),
            Err(crate::CliError::FormFill(
                btctax_forms::FormsError::UnsupportedYear(2027)
            ))
        ));
    }

    /// spec 1099-DA T6 — the readiness sentence names the year's Form 1099-DA regime, so the
    /// `report` header and the export stamp say which information-return world the year is in.
    #[test]
    fn the_sentence_names_the_1099da_regime() {
        let s24 = YearReadiness::bundled(2024).sentence();
        let s25 = YearReadiness::bundled(2025).sentence();
        let s26 = YearReadiness::bundled(2026).sentence();
        assert!(s24.contains("1099-DA none)"), "{s24}");
        assert!(s25.contains("1099-DA proceeds)"), "{s25}");
        assert!(s26.contains("1099-DA proceeds+basis)"), "{s26}");
    }

    /// ★ build review r2 NEW-3 — `report`'s prior-year regime join (`regime_or_refuse(year - 1)`)
    /// is unreachable-as-Err only because every year that HAS full-return tables also has a
    /// YEAR.toml record. Pin that, so the `?` cannot start refusing a year the tables serve.
    #[test]
    fn every_year_with_full_return_tables_is_a_bundled_year() {
        let t = BundledFullReturnTables::load();
        let served: Vec<i32> = (2000..=2100)
            .filter(|&y| t.full_return_for(y).is_some())
            .collect();
        assert!(!served.is_empty(), "the tables serve at least one year");
        for y in served {
            assert!(
                bundled_years().contains(&y),
                "TY{y} has full-return tables but no forms/{y}/YEAR.toml record"
            );
        }
    }

    /// ★ spec 1099-DA T0 — the box-revision constant and the record cannot drift: proceeds reporting
    /// begins exactly with the digital-asset box revision, for every bundled year.
    #[test]
    fn the_constant_and_the_regime_agree_on_every_bundled_year() {
        for &y in bundled_years() {
            let r = regime_for(y).expect("every bundled year has a record");
            assert_eq!(
                r.proceeds,
                y >= btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR,
                "TY{y}: YEAR.toml says proceeds={} but DIGITAL_ASSET_8949_FIRST_YEAR says {}",
                r.proceeds,
                y >= btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR
            );
            assert!(
                !r.basis || r.proceeds,
                "TY{y}: basis reporting implies proceeds reporting"
            );
        }
    }

    #[test]
    fn the_default_year_is_the_newest_bundled_one() {
        assert_eq!(default_year(), 2026); // TY2026's record is bundled (preparing) — the year being filed
        assert_eq!(default_year(), *bundled_years().last().unwrap());
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // FR-225 — the printed claim IS the return's verdict, over the whole matrix.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// ★★★ **FR-225 KILL — THE CLAIM AND THE VERDICT AGREE, ON EVERY COMBINATION.**
    ///
    /// **The defect.** `lines()` printed `return: computable` from [`YearReadiness::params`] — *does
    /// this BUILD bundle the year* — while `btctax report`, over the same stored return, printed
    /// `NOT COMPUTABLE [CharitableCwaUnresolved]`. Two instruments in one binary disagreeing about
    /// whether one return can be filed, with the filer-facing one wrong.
    ///
    /// ★★ **This asserts the AGREEMENT, not a string.** The brief's requirement: *"do not settle for
    /// asserting the string — assert that the claim and the return's actual verdict agree, so a future
    /// divergence reds wherever it appears."* So the property is
    /// `claims_computable() == verdict.computes()`, quantified over every package state × interview
    /// state × verdict — 24 cells, enumerated from the sets rather than sampled. Any future edit that
    /// re-derives the printed word from anything other than the verdict reds here, whatever it derives
    /// it from.
    ///
    /// **Plants that red it:**
    /// - restore `if self.package_computes { return vec![… "return: computable"] }` — every
    ///   `NotRun`/`Refuses` cell on a bundled year fails;
    /// - make `ReturnVerdict::computes()` true for `NotRun` — the `NotRun` cells fail;
    /// - drop the `[{reason:?}]` from the refusing arm — the last two assertions fail.
    #[test]
    fn the_printed_claim_agrees_with_the_return_verdict() {
        use btctax_core::tax::return_refuse::RefuseReason;
        // A year WITH its package and a year WITHOUT, both read off the build rather than typed: the
        // FR-99 shape is a year literal beside a bundle that moves.
        let with_package = *bundled_years()
            .iter()
            .find(|y| YearReadiness::bundled(**y).params)
            .expect("this build bundles at least one filable year's params");
        let without_package = *bundled_years()
            .iter()
            .find(|y| !YearReadiness::bundled(**y).params)
            .expect("this build bundles at least one year with no params (TY2026 is preparing)");

        let refuses = ReturnVerdict::Refuses {
            reason: RefuseReason::CharitableCwaUnresolved,
            detail: "the §170(f)(8)(A) acknowledgment is unresolved".to_string(),
        };
        let verdicts = [
            ReturnVerdict::NotRun,
            ReturnVerdict::Computes,
            refuses.clone(),
        ];
        // The four interview states `lines()` can render, as (complete, blocking) pairs.
        let interviews = [
            (None, None),
            (Some(true), Some(0)),
            (Some(false), Some(3)),
            (Some(false), None),
        ];

        let mut cells = 0usize;
        let mut claimed = 0usize;
        for year in [with_package, without_package] {
            for (complete, blocking) in interviews {
                for verdict in &verdicts {
                    let st = EntryStates {
                        interview_complete: complete,
                        blocking,
                        ..EntryStates::package_only(year)
                    }
                    .with_return_verdict(verdict.clone());
                    let lines = st.lines().join(" · ");
                    cells += 1;
                    // ★ THE INVARIANT. The printed word is the RETURN's verdict and nothing else —
                    //   not the package, not the interview.
                    assert_eq!(
                        st.claims_computable(),
                        verdict.computes() && st.package_computes,
                        "TY{year} / interview {complete:?} / {verdict:?} — the printed claim and the \
                         return's verdict must agree: {lines}"
                    );
                    if st.claims_computable() {
                        claimed += 1;
                    }
                    // ★ …and the interview half is still stated independently, which is R11's rule.
                    assert!(
                        lines.contains("interview: "),
                        "both states are always stated: {lines}"
                    );
                }
            }
        }
        assert_eq!(cells, 24, "every cell of the matrix was visited");
        assert_eq!(
            claimed, 4,
            "exactly the four `Computes` cells on the bundled year may print `computable`"
        );

        // ★★ And the refusing line NAMES the refusal — the same VARIANT `report` names — and points
        //    at the surfaces that carry its exit sentence in full. It must inline neither the detail
        //    nor the reason's `Debug` payload: these are display lines for a pane that does not wrap
        //    (FR-63), and a payload-carrying reason measured 164 columns.
        let refusing_st = EntryStates::package_only(with_package).with_return_verdict(refuses);
        let refusing = refusing_st.sentence();
        assert!(
            refusing.contains("return: NOT computable [CharitableCwaUnresolved]"),
            "the refusal is named: {refusing}"
        );
        assert!(
            !refusing.contains("the §170(f)(8)(A) acknowledgment is unresolved"),
            "…and the detail is NOT inlined into a non-wrapping display line (FR-63): {refusing}"
        );
        assert!(
            refusing.contains("print it in full"),
            "…but the filer is sent somewhere that DOES print it: {refusing}"
        );
        assert!(
            refusing_st
                .verdict
                .refusal()
                .is_some_and(|(_, d)| d.contains("the §170(f)(8)(A) acknowledgment is unresolved")),
            "and the detail is still CARRIED, for the surfaces that can show it"
        );
    }

    /// ★★★ **FR-225 — A SURFACE THAT CANNOT COMPUTE THE RETURN SAYS SO, AND NAMES WHO CAN.**
    ///
    /// The TUI's tax-inputs pane holds no `LedgerState`, so it cannot run `screen_compute_dependent`
    /// or `screen_absolute` and genuinely does not know this return's verdict. Before FR-225 it
    /// printed `return: computable` regardless. The replacement must be a weaker TRUE claim with an
    /// exit, not merely a removal — an unexplained gap is the other half of the same defect.
    ///
    /// **Plant:** drop the command from the `NotRun` line — the "names the command" assertion reds.
    #[test]
    fn a_surface_with_no_verdict_states_the_boundary_and_names_the_command() {
        let year = *bundled_years()
            .iter()
            .find(|y| YearReadiness::bundled(**y).params)
            .expect("a bundled filable year");
        let st = EntryStates::package_only(year);
        assert_eq!(
            st.verdict,
            ReturnVerdict::NotRun,
            "no verdict was handed in"
        );
        let lines = st.lines();
        assert!(!st.claims_computable(), "so it claims nothing: {lines:?}");
        assert!(
            lines[0].contains("return: not checked here"),
            "it says which question it has not asked: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains(&format!("btctax report --tax-year {year}"))),
            "…and names the command that decides it: {lines:?}"
        );
    }

    /// ★★★ **FR-225 / FR-63 — EVERY LINE FITS THE FIXED PANE, ON EVERY VERDICT.**
    ///
    /// [`EntryStates::lines`] returns *lines* rather than one sentence precisely because the TUI's
    /// status pane is a fixed 118 columns and does not wrap (`draw_edit.rs` draws each as one
    /// `Line`). FR-63: *"a fact that will not fit is added BESIDE it rather than by lengthening it."*
    ///
    /// **And it is written from a measurement, not a worry.** FR-225's first `NotRun` wording was two
    /// lines, and `docs/examples-tui-walkthrough/j6/01` came back with the second one clipped at the
    /// 118th column — *"…, which run"* — plus a section row pushed off the list to make room. The
    /// golden caught it; nothing in this module could. This holds the budget for every arm so that the
    /// next edit here cannot reintroduce it silently.
    ///
    /// **Plant:** lengthen any arm's text past the budget — this reds, before a golden has to.
    #[test]
    fn the_entry_lines_fit_the_fixed_pane() {
        use btctax_core::tax::return_refuse::RefuseReason;
        // The pane is 118 columns of border-to-border content and `draw_edit.rs` prefixes each line
        // with two spaces, so this is what one line may occupy.
        const BUDGET: usize = 118 - 2;
        let verdicts = [
            ReturnVerdict::NotRun,
            ReturnVerdict::Computes,
            // ★ A LONG reason name and an enormous detail: the detail must not reach the line at all,
            //   and the reason must fit beside the longest interview clause.
            ReturnVerdict::Refuses {
                reason: RefuseReason::DigitalAssetAnswerContradictsLedger {
                    date: "2024-05-01".into(),
                    venue: "exchange:river:default".into(),
                    kind: "a disposition",
                },
                detail: "x".repeat(2000),
            },
        ];
        for year in bundled_years() {
            for (complete, blocking) in [
                (None, None),
                (Some(true), Some(0)),
                (Some(false), Some(999)),
            ] {
                for verdict in &verdicts {
                    let st = EntryStates {
                        interview_complete: complete,
                        blocking,
                        ..EntryStates::package_only(*year)
                    }
                    .with_return_verdict(verdict.clone());
                    for l in st.lines() {
                        assert!(
                            l.chars().count() <= BUDGET,
                            "TY{year} / {verdict:?}: {} columns exceeds the pane's {BUDGET}, so it \
                             would be CLIPPED (FR-63): {l}",
                            l.chars().count()
                        );
                    }
                }
            }
        }
    }
}
