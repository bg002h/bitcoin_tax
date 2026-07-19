//! The clap-4 command surface for `btctax`, extracted into the library so tooling
//! (the `xtask` man-page generator) can obtain the `Command` via `Cli::command()`
//! (`clap::CommandFactory`). The binary (`main.rs`) is a thin dispatch over these types.
//!
//! FILE-FORMAT DOCS — SINGLE SOURCE OF TRUTH: the long-help (`///` doc-comments with
//! `#[arg(verbatim_doc_comment)]`) on the file/format-taking args below is rendered BOTH
//! into `--help` (clap) AND into the per-subcommand man page (clap_mangen), zero drift.
//! Formats were read from the writers, never from stale comments: export CSVs from
//! `render.rs`, the classify-raw serde shape from `btctax-core::EventPayload`, the key
//! armor from `btctax-store::Vault::backup_key`, the selections header from
//! `cmd::reconcile::import_selections`, the lot pick from `eventref::parse_lot_pick`.
use btctax_core::{FilingStatus, LotMethod};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "btctax", about = "Offline US Bitcoin tax ledger (Phase 1)")]
pub struct Cli {
    /// Path to the encrypted vault (vault.pgp).
    #[arg(long, global = true, default_value = "vault.pgp")]
    pub vault: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create the encrypted vault + force a key backup.
    Init {
        /// File to write the forced key backup to: an ASCII-armored, passphrase(S2K)-encrypted
        /// private key, owner-only (mode 0600). Identical format to `backup-key --out`. Store it
        /// offline — it is the only way to recover the vault if you lose `vault.key`.
        ///
        /// FORMAT (structure — NOT a real key):
        ///   -----BEGIN PGP PRIVATE KEY BLOCK-----
        ///   ... base64 armor of the S2K-encrypted secret key ...
        ///   -----END PGP PRIVATE KEY BLOCK-----
        #[arg(long, verbatim_doc_comment)]
        key_backup: PathBuf,
        /// Clear an interrupted/half-created init (orphan `vault.key`, no encrypted store) and start fresh.
        #[arg(long, default_value_t = false)]
        repair: bool,
    },
    /// Import one or more export files (auto-groups Swan).
    Import { files: Vec<PathBuf> },
    /// FR9 integrity check (non-zero exit on hard blockers).
    Verify,
    /// Show holdings + realized disposals/removals/income. With --tax-year: standalone TaxResult.
    ///
    /// Exit code with --tax-year (UX-P4-10): 0 = a filing-ready number was rendered; 1 = ran but the
    /// year is NOT COMPUTABLE (no filing-ready number — a missing tax profile/table or a hard blocker;
    /// mirrors `verify`); 2 = the command failed (any error). Scripts should key on NON-ZERO. A
    /// pseudo-active report still exits 0 — the on-screen banner is the signal, not the exit code.
    #[command(alias = "show")]
    Report {
        /// Filter realized disposals/removals/income to a specific calendar year (display path).
        #[arg(long)]
        year: Option<i32>,
        /// Compute the crypto-attributable federal tax for the given tax year (B.5 / Task 9).
        /// Requires a stored tax profile (`tax-profile --year Y ...`) and the bundled TY table.
        /// Independent of --year; the two flags are not aliased.
        #[arg(long)]
        tax_year: Option<i32>,
        /// Cumulative prior-year TAXABLE gifts (post-annual-exclusion Form 709 amounts), not
        /// gross gifts. Used for the §2505 lifetime-exclusion consumption advisory. Defaults to
        /// $0 when omitted (the advisory discloses this assumption). Must not be negative.
        #[arg(long)]
        prior_taxable_gifts: Option<String>,
        /// §4 R3-M6: persist this year's computed charitable + QBI carryover-OUT as next year's
        /// carryover-IN (a full-return `--tax-year` ReturnInputs year only). `report` is otherwise
        /// read-only; this flag opts into the vault write.
        #[arg(long, default_value_t = false)]
        write_carryover: bool,
        /// With `--write-carryover`: overwrite a next-year carryover-in that was user-entered
        /// (`income import`). Without it, a user-entered value is left untouched and the write refuses.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Discover the reconciliation event references (`ref`s) you pass to the `reconcile` verbs.
    #[command(subcommand)]
    Events(Events),
    /// Print the LIMITATIONS & supported-forms document: what a v1 full return covers, the credits it
    /// omits conservatively (your tax is overstated, never understated), what it refuses outright, and
    /// what it cannot represent. Read this before you file.
    Limitations,
    /// Emit a reconciliation decision event.
    #[command(subcommand)]
    Reconcile(Reconcile),
    /// Show or set projection config (TP8 fee treatment / pre-2025 lot method / forward method).
    Config {
        #[arg(long, value_enum)]
        set_fee_treatment: Option<FeeArg>,
        #[arg(long, value_enum)]
        set_pre2025_method: Option<MethodLotArg>,
        #[arg(long, default_value_t = false)]
        attest_pre2025_method: bool,
        /// §A.5(a): append a MethodElection decision (the forward standing order). Not a flag
        /// mutation — this is an event in the ledger. Use --effective-from to set the date
        /// (default: today / the decision's made-date).
        #[arg(long, value_enum)]
        set_forward_method: Option<MethodLotArg>,
        /// §A.5(a) per-ACCOUNT scope for --set-forward-method (IRS 2025+ per-account rule):
        /// exchange:PROVIDER:ACCOUNT (the canonical wallet grammar). Omit for a GLOBAL election
        /// (the existing behavior). Only exchange accounts are electable (a method election is a
        /// brokerage-account concept; self:LABEL is rejected). The account MUST already exist in the
        /// vault — an unknown/typo'd account is rejected LOUDLY so it can't create a dead election.
        #[arg(long)]
        exchange: Option<String>,
        /// Effective-from date for --set-forward-method (YYYY-MM-DD). Defaults to made-date.
        #[arg(long)]
        effective_from: Option<String>,
    },
    /// FR10: export decrypted SQLite + CSV (the NFR2 plaintext exception).
    ///
    /// WARNS (does not refuse) on unresolved Hard blockers: any Hard blocker makes every affected
    /// tax year NOT COMPUTABLE, so the exported Form 8949 / Schedule D / figures are INFORMATIONAL,
    /// not final. A warning is printed to stderr and the export still succeeds (exit 0). Automation
    /// that must GATE on unresolved blockers should check `btctax verify` (which exits non-zero),
    /// since export-snapshot itself stays exit 0.
    ExportSnapshot {
        /// Output DIRECTORY receiving the decrypted SQLite DB (snapshot.sqlite) + projection CSVs
        /// (the NFR2 plaintext exception; created owner-only). ALWAYS writes: lots.csv,
        /// disposals.csv, removals.csv, income.csv. With --tax-year it ALSO writes form8949.csv,
        /// schedule_d.csv, form8283.csv, and schedule_se.csv (schedule_se only when there is
        /// business self-employment income). The `event` column in disposals.csv / removals.csv /
        /// income.csv is the event-ref that reconcile commands consume (select-lots,
        /// set-donation-details, reclassify-income, …).
        ///
        /// FORMAT (removals.csv header + one sample donation row):
        ///   event,kind,removed_at,lot,sat,basis,fmv_at_transfer,term,acquired_at,claimed_deduction,donee
        ///   import|coinbase|X,donation,2025-03-01,import|coinbase|X#0,25000,120.00,150.00,long,2023-01-05,150.00,Charity Y
        #[arg(long, verbatim_doc_comment)]
        out: PathBuf,
        /// Also emit the per-tax-year Form 8949 + Schedule D CSVs (form8949.csv / schedule_d.csv),
        /// scoped to this calendar year. Omit to write only the all-years projection CSVs.
        #[arg(long)]
        tax_year: Option<i32>,
        /// Attestation phrase required to export while the ledger is PSEUDO-RECONCILED (a synthetic
        /// default contributes to the projection). Pass the exact phrase `I attest this is true`
        /// (trimmed, case-sensitive) to export the fictional draft ON PURPOSE. Omit on a fully-real
        /// ledger (never gated). Omit on an interactive terminal to be prompted; omit when piped
        /// (non-TTY) while pseudo-active and the export is refused.
        #[arg(long)]
        attest: Option<String>,
    },
    /// Fill the OFFICIAL IRS fillable PDFs for a tax year (a whole packet).
    ///
    /// Writes (owner-only) into --out, populated from btctax's already-computed projection — no
    /// capital-gains figure is recomputed:
    ///   - f8949.pdf + schedule_d.pdf — ALWAYS. On the 2025 (1099-DA) revision Bitcoin is filed under
    ///     Box I (short-term) / Box L (long-term) — the digital-asset boxes; on the pre-1099-DA 2024
    ///     and 2017 revisions it is Box C / Box F ("not reported on a 1099-B"). Never the wrong pair
    ///     for the year. More rows than a part's grid holds (11 in 2025, 14 in 2024/2017) paginate
    ///     onto multiple copies, each with its own totals.
    ///   - schedule_se.pdf — when there is business self-employment income and net earnings are ≥ the
    ///     $400 floor. Line 12 (SE tax) = Social Security + regular Medicare ONLY; the 0.9% Additional
    ///     Medicare Tax is a Form 8959 item (flagged on stderr, not put on Schedule SE). Requires a
    ///     stored `tax-profile` for the year (filing status); missing profile ⇒ a NOTE, not a form.
    ///   - form_8283.pdf — when there are BTC donations. Fills the donee/appraiser IDENTITY + per-
    ///     donation property rows (Section A ≤ $5,000 or Section B > $5,000). The property-type box is
    ///     "k Digital assets" on the Rev. 12-2023/2025 forms (2024/2025); the 2017 Rev. 12-2014 form
    ///     has no such box, so BTC uses "j Other" + a printed note. Leaves every OTHER party's
    ///     declaration/signature BLANK — a Section B 8283 is NOT filing-ready without those signed.
    ///     Overflows onto additional copies.
    ///   - form_1040_capgains.pdf — when there is reportable capital/digital-asset activity. Fills the
    ///     capital-gain line (line 7a in 2025 / line 7 in 2024 / line 13 in 2017, when Schedule D is
    ///     active and line 16 ≥ 0; active-and-zero → "-0-"; a net loss leaves it blank — the §1211
    ///     line-21 cap is yours) and, on 2024/2025, the Digital-Asset question (YES iff any disposal,
    ///     income, gift, or donation; never a "No"). The 2017 form has no Digital-Asset question, so an
    ///     income-only 2017 year produces no 1040. 7b checkboxes are untouched.
    ///
    /// Every written value is read back GEOMETRICALLY against the blank PDF's own field coordinates and
    /// the fill FAILS CLOSED on any mis-placement — a wrong tax form is never written. The engine drops
    /// the forms' XFA layer (else Acrobat opens them blank) and sets NeedAppearances so a viewer
    /// regenerates the visible values. Schedule D lines 17-22 (28%-rate / unrecaptured-§1250 / QDI
    /// worksheet, incl. the line-21 loss limit) are OUT OF SCOPE. Rows on an exchange that MAY carry
    /// 1099-DA broker reporting are flagged on stderr (btctax files them all under Box I/L and says so).
    ///
    /// A tax year that has FULL-RETURN inputs (`income import`) DISPATCHES to the complete return packet —
    /// the 1040 and every schedule/attachment it cites, in Attachment-Sequence order, plus a manifest
    /// (`--forms` is ignored on that path). A crypto-only year (no `income import`) instead fills the
    /// crypto SLICE: Schedule D carries only the ledger's crypto totals — it has no line 13 (1099-DIV
    /// box-2a capital-gain distributions) and no lines 6/14 (capital-loss carryovers), and the 1040 fill
    /// covers only the capital-gain cluster. For a crypto-only year those slice forms are complete and
    /// correct; a full-return year needs the full packet, which is why the two paths are dispatched
    /// separately (and write non-overlapping filenames). See `btctax limitations`.
    ///
    /// PSEUDO-RECONCILED ledgers: the same attestation gate as export-snapshot applies, AND every
    /// page is stamped with a diagonal `DRAFT — ESTIMATE, NOT FOR FILING` watermark.
    ExportIrsPdf {
        /// Output DIRECTORY receiving the filled official PDFs (created owner-only): f8949.pdf,
        /// schedule_d.pdf, and — when applicable — schedule_se.pdf, form_8283.pdf,
        /// form_1040_capgains.pdf. These contain your unencrypted tax data — write --out OUTSIDE any
        /// git repo.
        #[arg(long, verbatim_doc_comment)]
        out: PathBuf,
        /// The tax year to fill (this build bundles TY2017, TY2024 and TY2025; other years are
        /// refused). TY2024/TY2017 are pre-1099-DA revisions: Bitcoin is filed under Box C/F (not Box
        /// I/L). TY2017 additionally uses the OLD forms — the §B long Schedule SE, Form 8283 Rev.
        /// 12-2014 ("j Other", no digital-asset box), the 1040 capital gain on line 13, and NO
        /// Digital-Asset question.
        #[arg(long)]
        tax_year: i32,
        /// Restrict the crypto-slice packet to specific forms (repeat or comma-separate). Values:
        /// `f8949`, `schedule-d`, `schedule-se`, `form8283`, `form1040`. Default = every applicable form
        /// (f8949 + schedule-d always; schedule-se when SE income ≥ the $400 floor; form8283 when there
        /// are donations; form1040 when there is reportable digital-asset activity). A named form is still
        /// skipped when it does not apply. Ignored on a full-return year (that path fills the whole packet).
        #[arg(long, value_enum, value_delimiter = ',')]
        forms: Vec<FormArg>,
        /// Attestation phrase required to export while the ledger is PSEUDO-RECONCILED (a synthetic
        /// default contributes to the projection). Pass the exact phrase `I attest this is true`
        /// (trimmed, case-sensitive) to fill the DRAFT-watermarked forms ON PURPOSE. Omit on a
        /// fully-real ledger (never gated). Omit on an interactive terminal to be prompted; omit when
        /// piped (non-TTY) while pseudo-active and the export is refused.
        #[arg(long)]
        attest: Option<String>,
    },
    /// Export the passphrase-protected key.
    BackupKey {
        /// File to write the exported key to: an ASCII-armored, passphrase(S2K)-encrypted private
        /// key, owner-only (mode 0600). Identical format to `init --key-backup`.
        ///
        /// FORMAT (structure — NOT a real key):
        ///   -----BEGIN PGP PRIVATE KEY BLOCK-----
        ///   ... base64 armor of the S2K-encrypted secret key ...
        ///   -----END PGP PRIVATE KEY BLOCK-----
        #[arg(long, verbatim_doc_comment)]
        out: PathBuf,
    },
    /// Lot-specific-identification optimizer (§C — read-only proposal or gated persistence).
    #[command(subcommand)]
    Optimize(Optimize),
    /// Read-only what-if tax planning (task #43): posit a HYPOTHETICAL, NON-persisted transaction and
    /// see its MARGINAL federal-tax effect on the current-year position. Routes through the same audited
    /// tax engine as `report --tax-year`; invents no tax authority. Writes NOTHING — no event, no
    /// side-table row, no vault mutation. Tax decision-support (consequences), not buy/sell/hold advice.
    #[command(subcommand)]
    WhatIf(WhatIf),
    /// Full-return (v1) input surface: import, show, or clear the per-year full-return inputs (W-2s,
    /// 1099s, deductions, household). Offline; stored in the encrypted vault.
    #[command(subcommand)]
    Income(IncomeCmd),
    /// Set or show the per-tax-year tax profile (filing status, income, MAGI, etc.).
    TaxProfile {
        /// The tax year (e.g. 2025).
        #[arg(long)]
        year: i32,
        /// IRS filing status.
        #[arg(long, value_enum)]
        filing_status: Option<FilingStatusArg>,
        /// Ordinary taxable income EXCLUDING all app-computed crypto items (net ST gains,
        /// mining/staking ordinary income). The engine adds the crypto items on top (B.1 / I5).
        #[arg(long)]
        ordinary_taxable_income: Option<String>,
        /// Modified AGI excluding crypto items, for the §1411 NIIT threshold comparison.
        ///
        /// IMPORTANT (§1411 contract): this value MUST already include the taxpayer's qualified
        /// dividends and non-crypto net capital gains (and any other MAGI add-backs from
        /// §1411(d)). The engine adds ONLY the crypto AGI delta on top (ambiguity #5 in the
        /// design). Omitting QD or non-crypto cap gains from this figure understates NIIT.
        #[arg(
            long,
            long_help = "Modified AGI excluding crypto items, for the §1411 NIIT \
            threshold comparison.\n\nIMPORTANT (§1411 contract): this value MUST already \
            include the taxpayer's qualified dividends and non-crypto net capital gains (and \
            any other MAGI add-backs from §1411(d)). The engine adds ONLY the crypto AGI \
            delta on top (ambiguity #5 in the design). Omitting QD or non-crypto cap gains \
            from this figure understates NIIT."
        )]
        magi_excluding_crypto: Option<String>,
        /// Qualified dividends + other preferential-rate income sharing the §1(h) 0/15/20 LTCG
        /// rate stack. Required when setting a profile.
        #[arg(long)]
        qualified_dividends: Option<String>,
        /// Non-crypto net LT-character capital gain already in the profile (optional; defaults
        /// to 0 when omitted).
        #[arg(long)]
        other_net_capital_gain: Option<String>,
        /// §1212(b) short-term capital loss carryforward into this year (optional; defaults to 0).
        #[arg(long)]
        carryforward_short: Option<String>,
        /// §1212(b) long-term capital loss carryforward into this year (optional; defaults to 0).
        #[arg(long)]
        carryforward_long: Option<String>,
        /// Form W-2 Social Security wages (Box 3 + Box 7 tips; Schedule SE line 8a).
        /// Reduces the §1401(a) SS cap: ss_cap = max(0, wage_base − w2_ss_wages). Optional;
        /// defaults to $0. Must not be negative.
        #[arg(long)]
        w2_ss_wages: Option<String>,
        /// Medicare wages (Box 5; Form 8959 line 1).
        /// Reduces the Additional-Medicare threshold: addl_threshold = max(0, threshold − w2_medicare_wages)
        /// (§1401(b)(2)(B)/Form 8959 Part II). Optional; defaults to $0. Must not be negative.
        #[arg(long)]
        w2_medicare_wages: Option<String>,
        /// Schedule C deductible business expenses for the year — reduces net SE earnings;
        /// the income-tax stack above is NOT adjusted (see the advisory).
        /// Optional; defaults to $0. Must not be negative.
        #[arg(long)]
        schedule_c_expenses: Option<String>,
        /// Show the stored profile for `--year` instead of setting it.
        #[arg(long, default_value_t = false)]
        show: bool,
        /// Store the raw profile even when full-return inputs (`income import`) already exist for the year
        /// (they take precedence, so the raw profile would otherwise be ignored — D-4 guard).
        #[arg(long, default_value_t = false)]
        force: bool,
    },
}

/// `optimize` subcommand tree.  Task 9 adds `Run`; Task 10 adds `Accept`; Task 11 adds `Consult`.
#[derive(Subcommand)]
pub enum Optimize {
    /// Mode-1 what-if: print the tax-saving lot-selection proposal. NOTHING is filed or bound.
    Run {
        /// The tax year to optimize (must be 2025 or later).
        #[arg(long)]
        tax_year: i32,
    },
    /// Mode-1 gated persistence: recompute the optimum and persist the proposed LotSelection(s),
    /// gated per disposal (§1.1012-1(j)). A genuinely-contemporaneous pick (made ≤ sale) persists
    /// freely; an already-executed disposal persists ONLY with a narrow per-disposal `--attest`
    /// scoped to one `--disposal`; a 2027+ broker-held pick is refused. Revoke via `reconcile void`.
    Accept {
        /// The tax year to accept (must be 2025 or later).
        #[arg(long)]
        tax_year: i32,
        /// Restrict to ONE disposal (required to carry `--attest`).
        #[arg(long)]
        disposal: Option<String>,
        /// Narrow contemporaneous-ID attestation for an already-executed disposal. Requires
        /// `--disposal` (no blanket attestation across all disposals).
        #[arg(long)]
        attest: Option<String>,
    },
    /// Mode-2 read-only pre-trade what-if (§C.3): tax-min lots + ST/LT split + federal tax + ST→LT
    /// timing. NOTHING is written — no event, no side-table row. Tax decision-support only;
    /// not buy/sell/hold advice.
    Consult {
        /// Hypothetical sale amount (required). Accepts a satoshi integer OR a BTC decimal, e.g.
        /// `0.05` or `5000000` (a value with a `.` is BTC; a bare integer is satoshis).
        #[arg(long)]
        sell: String,
        /// Wallet to sell from, e.g. `self:cold` or `exchange:coinbase:default` (required; per-wallet
        /// pool is mandatory post-2025).
        #[arg(long)]
        wallet: Option<String>,
        /// Sale date for the what-if (YYYY-MM-DD; defaults to today UTC if omitted).
        #[arg(long)]
        at: Option<String>,
        /// Explicit USD proceeds for the hypothetical sale. Required when `--at` is a future date
        /// with no bundled dataset price and `--fmv` is not used. Mutually exclusive with `--fmv`.
        #[arg(long, conflicts_with = "fmv")]
        proceeds: Option<String>,
        /// Use the bundled daily-close FMV for `--at` instead of an explicit proceeds amount.
        /// A future date with no dataset price will return a ProceedsRequired error. Mutually
        /// exclusive with `--proceeds`.
        #[arg(long, conflicts_with = "proceeds")]
        fmv: bool,
    },
}

/// Full-return (v1) input subcommands (SPEC §4 / recon-04 §6). v1 ships the TOML bulk-import path +
/// a JSON show; incremental per-field subcommands (`add-w2`, …) are a follow-on.
#[derive(Subcommand)]
pub enum IncomeCmd {
    /// Import full-return inputs from an offline TOML file into the vault for a tax year.
    Import {
        /// The tax year (e.g. 2024).
        #[arg(long)]
        year: i32,
        /// Path to the TOML file describing the full-return inputs.
        #[arg(long)]
        file: std::path::PathBuf,
    },
    /// Show the stored full-return inputs for a tax year (JSON, PII redacted), or nothing if none set.
    Show {
        /// The tax year (e.g. 2024).
        #[arg(long)]
        year: i32,
    },
    /// Remove the stored full-return inputs for a tax year (fall back to a raw `tax-profile`).
    Clear {
        /// The tax year (e.g. 2024).
        #[arg(long)]
        year: i32,
    },
    /// Answer the return's fail-loud questions interactively — the yes/no boxes that have no safe
    /// default (can someone claim you as a dependent? Schedule B's foreign-account and foreign-trust
    /// lines) plus the optional dates of birth.
    ///
    /// These questions REFUSE the return until they are answered: guessing "no" on your behalf would
    /// understate your tax and print an unchecked box you never affirmed. This is the only way to answer
    /// them without editing a TOML file. It never asks for a secret — SSNs and the IP PIN belong to
    /// `set-pii`, which does not echo what you type.
    ///
    /// Requires an existing return for the year (create one with `income import`).
    Answer {
        /// The tax year (e.g. 2024).
        #[arg(long)]
        year: i32,
    },
}

/// `what-if` subcommand tree (task #43). READ-ONLY hypothetical-transaction tax planning: NOTHING is
/// filed, appended, or persisted. Mirrors the `optimize consult` shape, plus an ad-hoc `TaxProfile`
/// (so you can plan without `tax-profile set`).
#[derive(Subcommand)]
pub enum WhatIf {
    /// Posit a hypothetical, NON-persisted SALE and see its MARGINAL federal tax: the lots it would
    /// consume, the ST/LT split, which §1(h) LTCG bracket (0/15/20) it lands in + room to the next
    /// breakpoint, the exact marginal tax (with-hypothetical minus baseline — the sale's OWN effect,
    /// not the whole-year figure), the effective rate, the §1212(b) carryforward carried to next year,
    /// this year's ordinary offset, and the §1411 NIIT delta. A net loss surfaces the carryforward
    /// disclosure (its value is NOT this-year tax). Writes NOTHING.
    Sell {
        /// Hypothetical sale amount (required). Accepts a satoshi integer OR a BTC decimal, e.g.
        /// `0.05` or `5000000` (a value with a `.` is BTC; a bare integer is satoshis).
        #[arg(long)]
        sell: String,
        /// Wallet to sell from, e.g. `self:cold` or `exchange:coinbase:default` (required; the
        /// per-wallet pool is mandatory post-2025).
        #[arg(long)]
        wallet: Option<String>,
        /// Sale date for the what-if (YYYY-MM-DD; defaults to today UTC if omitted).
        #[arg(long)]
        at: Option<String>,
        /// USD price per WHOLE BTC for the hypothetical sale (proceeds = price × sat / 1e8). Omit to
        /// use the bundled daily-close FMV for `--at`; REQUIRED for a future/off-dataset `--at` with no
        /// bundled price (else the what-if returns a ProceedsRequired error).
        #[arg(long)]
        price: Option<String>,
        /// Lot-selection method for the hypothetical sale: fifo|lifo|hifo. Omit to consume by the
        /// STANDING method (the account's in-force election / the default), exactly as a real disposal
        /// on that date would.
        #[arg(long, value_enum)]
        method: Option<MethodLotArg>,
        /// AD-HOC filing status (single|mfj|mfs|hoh|qss). Supplying this (with `--income`) builds a
        /// NON-persisted profile for the plan instead of the stored `tax-profile`. Omit ALL ad-hoc
        /// flags to use the stored profile for the sale year.
        #[arg(long, value_enum)]
        filing_status: Option<FilingStatusArg>,
        /// AD-HOC ordinary taxable income EXCLUDING crypto (the base the crypto stacks on). Required
        /// when building an ad-hoc profile.
        #[arg(long)]
        income: Option<String>,
        /// AD-HOC modified AGI excluding crypto, for the §1411 NIIT threshold. DEFAULTS TO `--income`
        /// when omitted (never $0 — a $0 MAGI would silently suppress every NIIT disclosure); a printed
        /// caveat notes the assumption. Supply the true MAGI (incl. QD + non-crypto cap gains) to avoid
        /// understating NIIT.
        #[arg(long)]
        magi: Option<String>,
        /// AD-HOC §1212(b) LONG-TERM capital-loss carryforward INTO the sale year (optional; defaults
        /// to $0). The dominant BTC case; short-term carryforward-in is out of scope for the ad-hoc
        /// profile (set a stored `tax-profile` for that).
        #[arg(long)]
        carryforward_in: Option<String>,
    },
    /// Posit a hypothetical, NON-persisted HARVEST and find the MAX BTC to sell such that a target holds
    /// on the ENTIRE prefix [0, N]: `--target zero-ltcg` (sell all that fits in the §1(h) 0% bracket),
    /// `fifteen-ltcg` (stay at/under 15%), `gain=$X` (realize at most $X of gain WITH this sale), or
    /// `tax=$X` (add at most $X of marginal federal tax; `tax=$0` is the flagship "zero-tax harvest").
    /// Uses the STANDING lot method's consumption order (never re-optimized). Discloses the §1212(b)
    /// carryforward burn, the §1411 NIIT kink (a 0%/15% answer can still cost +3.8%), and the plateau
    /// notes. The answer is ALWAYS engine-verified. Writes NOTHING.
    Harvest {
        /// The harvest target: `zero-ltcg` | `fifteen-ltcg` | `gain=$X` | `tax=$X` (X >= 0). `$` and
        /// commas are optional (e.g. `gain=25000`, `tax=$0`, `gain=$1,000`).
        #[arg(long)]
        target: String,
        /// Wallet to harvest from, e.g. `self:cold` or `exchange:coinbase:default` (required; the
        /// per-wallet pool is mandatory post-2025).
        #[arg(long)]
        wallet: Option<String>,
        /// Harvest date for the what-if (YYYY-MM-DD; defaults to today UTC if omitted).
        #[arg(long)]
        at: Option<String>,
        /// USD price per WHOLE BTC. Omit to use the bundled daily-close FMV for `--at`; REQUIRED for a
        /// future/off-dataset `--at` with no bundled price.
        #[arg(long)]
        price: Option<String>,
        /// AD-HOC filing status (single|mfj|mfs|hoh|qss). Supplying this (with `--income`) builds a
        /// NON-persisted profile for the plan instead of the stored `tax-profile`. Omit ALL ad-hoc
        /// flags to use the stored profile for the harvest year.
        #[arg(long, value_enum)]
        filing_status: Option<FilingStatusArg>,
        /// AD-HOC ordinary taxable income EXCLUDING crypto (the base the crypto stacks on). Required
        /// when building an ad-hoc profile.
        #[arg(long)]
        income: Option<String>,
        /// AD-HOC modified AGI excluding crypto, for the §1411 NIIT threshold. DEFAULTS TO `--income`
        /// when omitted (never $0 — a $0 MAGI would silently suppress every NIIT disclosure); a printed
        /// caveat notes the assumption.
        #[arg(long)]
        magi: Option<String>,
        /// AD-HOC §1212(b) LONG-TERM capital-loss carryforward INTO the harvest year (optional; defaults
        /// to $0) — expands the harvestable-gain room (gains are absorbed before touching the pref stack).
        #[arg(long)]
        carryforward_in: Option<String>,
    },
}

/// `events` subcommand tree (UX-P4-11): read-only ref discoverability.
#[derive(Subcommand)]
pub enum Events {
    /// List every DECIDABLE event — the imported rows a `reconcile` verb can act on
    /// (transfer-in, transfer-out, unclassified, import-conflict, income) — with its
    /// reference, kind, date, amount, and decision status. Rows are in ledger (import)
    /// order, which is not necessarily by date. Read-only; writes nothing.
    ///
    /// Copy a listed `ref` verbatim into a reconcile verb, e.g.
    /// `btctax reconcile classify-inbound-self-transfer <ref>` or
    /// `btctax reconcile reclassify-outflow <ref> --as-kind sell --amount <usd>`.
    /// A row shown as `decided: decision|N` already carries a decision; to change it,
    /// `btctax reconcile void decision|N` first, then re-decide.
    List,
}

#[derive(Subcommand)]
pub enum Reconcile {
    /// Confirm a self-transfer (TransferLink).
    LinkTransfer {
        out: String,
        #[arg(long, conflicts_with = "to_wallet")]
        to_event: Option<String>,
        #[arg(long)]
        to_wallet: Option<String>,
    },
    /// Classify an inbound TransferIn as income.
    ClassifyInboundIncome {
        /// The inbound TransferIn event reference to classify (see `btctax events list`).
        in_ref: String,
        /// Income kind: one of `mining`, `staking`, `interest`, `airdrop`, `reward`.
        #[arg(long)]
        kind: String,
        /// Fair-market value of the received BTC at receipt — USD dollars, NOT sats. On this
        /// single-event command there is NO auto-valuation: omitting `--fmv` records a Hard
        /// "FMV missing" blocker. To supply it, `reconcile void <decision-ref>` then re-classify with
        /// `--fmv` (classify-inbound is first-wins — re-running without voiding first is refused). To
        /// value automatically from the bundled daily close, use `reconcile bulk-classify-inbound-income`.
        #[arg(long)]
        fmv: Option<String>,
        /// Mark this income as earned in a trade or business (routes to Schedule C / SE tax).
        #[arg(long)]
        business: bool,
    },
    /// Classify an inbound TransferIn as a received gift.
    ClassifyInboundGift {
        in_ref: String,
        #[arg(long)]
        fmv_at_gift: String,
        #[arg(long)]
        donor_basis: Option<String>,
        #[arg(long)]
        donor_acquired: Option<String>,
    },
    /// Classify an inbound TransferIn as an inbound self-transfer ("my own coins" returning) —
    /// non-taxable, creates a fresh lot. `--basis` defaults to $0 (conservative; fires the honest
    /// zero-basis advisory when omitted); `--acquired` defaults to 1 year + 1 day before receipt
    /// (assumed long-term for a cold-storage deposit; discloses an advisory so you can correct it).
    ClassifyInboundSelfTransfer {
        in_ref: String,
        #[arg(long)]
        basis: Option<String>,
        #[arg(long)]
        acquired: Option<String>,
    },
    /// Reclassify a pending TransferOut.
    ReclassifyOutflow {
        out: String,
        #[arg(long, value_enum)]
        as_kind: OutKindArg,
        /// USD fair-market value of the disposed BTC at the transfer date — dollars, NOT sats.
        /// For a sell/spend this is the gross proceeds; for a gift/donation it is the FMV at the
        /// contribution date (26 CFR 1.170A-1(c)(1)). Entering the sats amount here is a common
        /// error and draws a non-fatal warning when it exceeds 100x the market value.
        #[arg(long)]
        amount: String,
        #[arg(long)]
        fee: Option<String>,
        #[arg(long)]
        appraisal: bool,
        /// Free-form donee identifier (e.g. "Alice", "Charity X"). Carried through to
        /// removals.csv and Form 8283; does not affect tax math.
        #[arg(long)]
        donee: Option<String>,
    },
    /// Set a manual FMV on an event.
    SetFmv {
        /// The event reference to set the FMV on (see `btctax events list`).
        event: String,
        /// Fair-market value of the BTC at the event date — USD dollars, NOT sats.
        #[arg(long)]
        fmv: String,
    },
    /// Void a revocable decision.
    Void { target: String },
    /// Resolve an Unclassified row from a JSON imported payload.
    ClassifyRaw {
        target: String,
        /// A JSON-encoded imported EventPayload (serde externally-tagged: `{"Variant":{...}}`) to
        /// resolve the Unclassified target as. Must be an IMPORTED variant — Acquire, Income,
        /// Dispose, TransferOut, TransferIn, or Unclassified. USD fields (usd_cost, fee_usd, …) are
        /// decimal STRINGS; `sat` is an integer.
        ///
        /// FORMAT (Acquire example):
        ///   {"Acquire":{"sat":2000000,"usd_cost":"1680.00","fee_usd":"5.00","basis_source":"ExchangeProvided"}}
        #[arg(long, verbatim_doc_comment)]
        payload_json: String,
    },
    /// Accept an import conflict.
    AcceptConflict { conflict: String },
    /// Reject an import conflict.
    RejectConflict { conflict: String },
    /// Path-B safe-harbor allocate (from the actual pre-2025 position).
    SafeHarborAllocate {
        #[arg(long, value_enum, default_value_t = MethodArg::Actual)]
        method: MethodArg,
        #[arg(long)]
        attest: bool,
    },
    /// Attest an existing allocation as timely.
    SafeHarborAttest,
    /// §A.4 Specific-ID: pick the exact lots a disposal consumes.
    SelectLots {
        disposal: String,
        /// One lot pick per --from flag (repeatable). Each PICK is
        /// `<origin_event_id>#<split_sequence>:<sat>`. The origin_event_id + split come from the
        /// `lot` column of disposals.csv or the `origin_event`/`split` columns of lots.csv
        /// (export-snapshot). The total sat across the picks must equal the disposal's principal
        /// (validated in the fold).
        ///
        /// FORMAT (two picks):
        ///   --from import|coinbase|X#0:25000 --from import|river|Y#1:5000
        #[arg(long = "from", required = true, verbatim_doc_comment)]
        from: Vec<String>,
    },
    /// §A.4 Batch import LotSelections from a CSV (disposal_ref,origin_event_id,split_sequence,sat).
    ImportSelections {
        /// CSV of lot picks imported as LotSelection decisions (§A.4). The header is REQUIRED and
        /// validated loudly; rows sharing a disposal_ref are grouped into a single decision.
        /// disposal_ref is the disposal event's ref (disposals.csv `event` column); origin_event_id
        /// is the lot's origin (lots.csv `origin_event` column).
        ///
        /// FORMAT (header + one sample row):
        ///   disposal_ref,origin_event_id,split_sequence,sat
        ///   import|gemini|trade|T-2.O-2,import|coinbase|X,0,1000000
        #[arg(verbatim_doc_comment)]
        csv: PathBuf,
    },
    /// SE-completion Chunk C: flip `business` (and optionally `kind`) on an already-imported Income event.
    ///
    /// Corrects the `business: false` hard-code that River (and other adapters) emit at ingest time,
    /// enabling SE-tax treatment for professional miners / stakers. The engine validates that the target
    /// event exists and is an Income event — a missing or non-Income target fires a Hard DecisionConflict
    /// blocker (decision excluded). For TransferIn rows use `classify-inbound-income` instead.
    ///
    /// DecisionConflict is Hard — to re-decide, `void` the prior decision first, then re-issue.
    ReclassifyIncome {
        /// The Income event reference (from `report` or `income_recognized.csv` 'event' column).
        income_event: String,
        /// Whether this income is from a trade or business (true → SE-tax eligible).
        /// Must be supplied explicitly: `--business true` or `--business false`.
        #[arg(long, required = true, action = clap::ArgAction::Set)]
        business: bool,
        /// Optional income kind correction: mining|staking|interest|airdrop|reward.
        /// Omit to keep the original kind (only flip `business`).
        #[arg(long)]
        kind: Option<String>,
    },
    /// Store Form 8283 Section-B donation + appraiser details for a donation event.
    /// The event ref is the TransferOut EventId from the removals.csv 'event' column.
    SetDonationDetails {
        /// TransferOut event reference for the donation (from removals.csv 'event' column).
        out_event_ref: String,
        /// Donee organization name (Part IV; required).
        #[arg(long, required = true)]
        donee_name: String,
        /// Donee mailing address (Part IV; optional).
        #[arg(long)]
        donee_address: Option<String>,
        /// Donee EIN (Part IV; required for Section-B completeness).
        #[arg(long)]
        donee_ein: Option<String>,
        /// Qualified appraiser name (Part III; required).
        #[arg(long, required = true)]
        appraiser_name: String,
        /// Appraiser mailing address (Part III; optional).
        #[arg(long)]
        appraiser_address: Option<String>,
        /// Appraiser TIN/SSN/EIN (Part III §6695A; satisfies the TIN-or-PTIN requirement).
        #[arg(long)]
        appraiser_tin: Option<String>,
        /// Appraiser PTIN (Part III §6695A; satisfies the TIN-or-PTIN requirement).
        #[arg(long)]
        appraiser_ptin: Option<String>,
        /// Appraiser qualifications declaration (§170(f)(11)(E)).
        #[arg(long)]
        appraiser_qualifications: Option<String>,
        /// Date the qualified appraisal was made (YYYY-MM-DD).
        #[arg(long)]
        appraisal_date: Option<String>,
        /// FMV determination method override (overrides the section-derived default on the
        /// Form 8283 carrier row; resolves the Section-A fmv_method deferral when supplied).
        #[arg(long)]
        fmv_method: Option<String>,
    },
    /// Show stored Form 8283 donation details for a donation event.
    ShowDonationDetails {
        /// TransferOut event reference for the donation (from removals.csv 'event' column).
        out_event_ref: String,
    },
    /// Bulk-confirm self-transfers: link every PENDING outbound transfer in a time frame to one
    /// destination wallet (non-taxable). Shows a preview + requires --yes (or interactive y/N).
    BulkLinkTransfer {
        /// Destination wallet every selected outflow links to.
        #[arg(long)]
        to_wallet: String,
        /// Restrict to a single tax year (mutually exclusive with --from/--to).
        #[arg(long, conflicts_with_all = ["from", "to"])]
        year: Option<i32>,
        /// Range start (YYYY-MM-DD; requires --to).
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end (YYYY-MM-DD, inclusive; requires --from).
        #[arg(long, requires = "from")]
        to: Option<String>,
        /// Only outflows FROM this source wallet.
        #[arg(long)]
        from_wallet: Option<String>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-classify unknown-basis inbound deposits as self-transfer-ins ("my own coins"): apply
    /// Cycle A's `SelfTransferMine` ($0 conservative basis, non-taxable) to MANY pending inbounds in a
    /// time frame at once. Shows a preview surfacing the total USD given $0 basis (the over-tax
    /// exposure) + requires --yes (or interactive y/N). Each is a voidable decision; for a deposit
    /// whose real cost you can substantiate, classify it single-item with `classify-inbound-self-transfer --basis`.
    BulkClassifyInboundSelfTransfer {
        /// Restrict to a single tax year (mutually exclusive with --from/--to).
        #[arg(long, conflicts_with_all = ["from", "to"])]
        year: Option<i32>,
        /// Range start (YYYY-MM-DD; requires --to).
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end (YYYY-MM-DD, inclusive; requires --from).
        #[arg(long, requires = "from")]
        to: Option<String>,
        /// Only inbounds received INTO this wallet.
        #[arg(long)]
        wallet: Option<String>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-classify unknown-basis inbound deposits as INCOME (mining|staking|interest|airdrop|reward):
    /// recognize MANY pending inbounds as ordinary income at their auto-FMV (the daily-close market
    /// value at receipt) in one confirmed batch, with a UNIFORM `--kind` + `--business` flag. Shows a
    /// preview surfacing the total income recognized + the count of inbounds EXCLUDED because no price
    /// was available for their date (those stay pending — an income row with no FMV would year-gate).
    /// Each is a voidable decision; for a single deposit use `classify-inbound-income`.
    BulkClassifyInboundIncome {
        /// Income kind for the whole batch: mining|staking|interest|airdrop|reward.
        #[arg(long)]
        kind: String,
        /// Whether this income is from a trade or business (true → SE-tax eligible).
        #[arg(long)]
        business: bool,
        /// Restrict to a single tax year (mutually exclusive with --from/--to).
        #[arg(long, conflicts_with_all = ["from", "to"])]
        year: Option<i32>,
        /// Range start (YYYY-MM-DD; requires --to).
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end (YYYY-MM-DD, inclusive; requires --from).
        #[arg(long, requires = "from")]
        to: Option<String>,
        /// Only inbounds received INTO this wallet.
        #[arg(long)]
        wallet: Option<String>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-reclassify unknown pending OUTFLOWS as dispositions (Sell|Spend): reclassify MANY pending
    /// `TransferOut`s as a `Dispose` in one confirmed batch, with the daily-close market value at the
    /// outflow date as the ESTIMATED proceeds. Shows a preview surfacing the total ESTIMATED proceeds
    /// AND the total ESTIMATED gain (sum(fmv) - sum(basis)) + the count of outflows EXCLUDED because no price
    /// was available for their date (those stay pending — a Sell with fabricated proceeds would be a
    /// SILENT misreport). `--kind` is UNIFORM and accepts ONLY sell|spend (gift/donate are out of
    /// scope). Each is a voidable decision; for a single outflow use `reclassify-outflow`.
    BulkReclassifyOutflow {
        /// Disposition kind for the whole batch: sell|spend (gift/donate rejected — out of scope).
        #[arg(long)]
        kind: String,
        /// Restrict to a single tax year (mutually exclusive with --from/--to).
        #[arg(long, conflicts_with_all = ["from", "to"])]
        year: Option<i32>,
        /// Range start (YYYY-MM-DD; requires --to).
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end (YYYY-MM-DD, inclusive; requires --from).
        #[arg(long, requires = "from")]
        to: Option<String>,
        /// Only outflows from this SOURCE wallet.
        #[arg(long)]
        wallet: Option<String>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-resolve import conflicts: ACCEPT (adopt each new payload) or REJECT (keep each current
    /// payload) MANY flagged `ImportConflict`s in one confirmed batch. Shows a `current → new` preview,
    /// then requires --yes (or interactive y/N). Exactly one of --accept / --reject is required. Each
    /// resolution is NON-REVOCABLE (`SupersedeImport`/`RejectImport` cannot be voided); to resolve a
    /// conflict differently, exclude it and use single-item `accept-conflict`/`reject-conflict`.
    #[command(group(clap::ArgGroup::new("resolve_action").required(true).args(["accept", "reject"])))]
    BulkResolveConflict {
        /// Accept every listed conflict (adopt each new payload onto its target).
        #[arg(long)]
        accept: bool,
        /// Reject every listed conflict (keep each target's current payload).
        #[arg(long)]
        reject: bool,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-void MANY revocable reconcile decisions in one confirmed batch (bulk-void). Shows a preview
    /// of every voidable decision (the SHARED `voidable_decisions` predicate — effective safe-harbor
    /// allocations are OMITTED, #7), then requires --yes (or interactive y/N). Each void is
    /// NON-REVOCABLE (a `VoidDecisionEvent` cannot itself be voided — re-apply the original decision to
    /// restore). Voiding a `LotSelection` also re-exposes its disposal to the default method and clears
    /// its optimizer attestation.
    BulkVoid {
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Match unreconciled inbound + outbound legs as self-transfers (self-transfer-passthrough C3).
    /// With no --in/--out: PREVIEW the proposed pairs (read-only). With --in and --out: confirm ONE
    /// pair (DROP for a same-wallet passthrough, RELOCATE for a cross-wallet transfer). NEVER automatic.
    MatchSelfTransfers {
        /// Confirm this in-leg (TransferIn eventref); requires --out.
        #[arg(long = "in", requires = "out_ref")]
        in_ref: Option<String>,
        /// Confirm this out-leg (TransferOut eventref); requires --in.
        #[arg(long = "out", requires = "in_ref")]
        out_ref: Option<String>,
        /// Override the suggested action (else the proposal's topology-derived action is used).
        #[arg(long, value_enum)]
        action: Option<SelfTransferActionArg>,
        /// Print the preview and exit without writing (conflicts with --in/--out).
        #[arg(long, conflicts_with_all = ["in_ref", "out_ref"])]
        dry_run: bool,
    },
    /// Pseudo-reconcile MODE (sub-project 2): fill deliberately-fictional default decisions at
    /// projection time (NEVER persisted) to clear the Hard classification blockers — a loudly-flagged
    /// `[PSEUDO]` on-screen estimate you correct toward truth. `on`/`off` toggle the mode; `approve`
    /// promotes chosen defaults to real (attested) decisions.
    #[command(subcommand)]
    Pseudo(Pseudo),
}

/// `reconcile pseudo <action>` — the pseudo-reconcile mode sub-verbs (sub-project 2).
#[derive(Subcommand)]
pub enum Pseudo {
    /// Turn pseudo-reconcile mode ON. Projection now synthesizes non-persisted default decisions for
    /// unresolved unknown-basis inbounds (self-transfer $0), unclassified rows, and import conflicts
    /// (accept-first); every synthetic contribution is flagged `[PSEUDO]` on screen and BLOCKS export.
    On,
    /// Turn pseudo-reconcile mode OFF. Projection reverts to real-only instantly and totally (no
    /// fictional events were ever written). Already-approved decisions REMAIN (they are real now).
    Off,
    /// Promote pseudo default decisions to REAL (attested) decisions in bulk. Shows a preview + requires
    /// `--yes` (or `--dry-run` to preview only). Optional filters restrict which defaults are approved.
    Approve {
        /// Only approve defaults of this TYPE: `self-transfer` (unknown-basis inbound → $0 self-transfer),
        /// `raw` (unclassified row placeholder), `conflict` (import conflict accept-first), or `fmv`
        /// (native income FMV synthesized from the daily close). Omit = all.
        #[arg(long, value_enum)]
        kind: Option<PseudoKindArg>,
        /// Only approve defaults whose target event is in this wallet (e.g. `exchange:coinbase:main`).
        #[arg(long)]
        wallet: Option<String>,
        /// Only approve defaults whose target event falls in this tax year.
        #[arg(long)]
        year: Option<i32>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
}

/// The pseudo-default TYPE filter for `reconcile pseudo approve --kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum PseudoKindArg {
    /// Unknown-basis inbound defaulted to a $0-basis self-transfer-in.
    SelfTransfer,
    /// Unclassified row defaulted to a zero-value placeholder (ClassifyRaw).
    Raw,
    /// Import conflict defaulted to accept-first (SupersedeImport).
    Conflict,
    /// Native income with a missing FMV defaulted to the daily-close value (ManualFmv).
    Fmv,
}

#[derive(Copy, Clone, ValueEnum)]
pub enum SelfTransferActionArg {
    /// Same-wallet passthrough → SelfTransferPassthrough (both legs skipped, non-taxable).
    Drop,
    /// Cross-wallet transfer → TransferLink (relocate the lots to the destination wallet).
    Relocate,
}

/// One official form in the `export-irs-pdf` packet (the `--forms` opt-in filter).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FormArg {
    /// Form 8949 (per-disposition capital-gains rows).
    F8949,
    /// Schedule D (aggregated capital-gains totals).
    ScheduleD,
    /// Schedule SE (self-employment tax).
    ScheduleSe,
    /// Form 8283 (noncash charitable contributions).
    Form8283,
    /// Form 1040 (capital-gains cells + the digital-asset question).
    Form1040,
}

#[derive(Copy, Clone, ValueEnum)]
pub enum FilingStatusArg {
    Single,
    Mfj,
    Mfs,
    Hoh,
    Qss,
}

impl From<FilingStatusArg> for FilingStatus {
    fn from(a: FilingStatusArg) -> Self {
        match a {
            FilingStatusArg::Single => FilingStatus::Single,
            FilingStatusArg::Mfj => FilingStatus::Mfj,
            FilingStatusArg::Mfs => FilingStatus::Mfs,
            FilingStatusArg::Hoh => FilingStatus::HoH,
            FilingStatusArg::Qss => FilingStatus::Qss,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
pub enum FeeArg {
    C,
    B,
}

#[derive(Copy, Clone, ValueEnum)]
pub enum MethodLotArg {
    Fifo,
    Lifo,
    Hifo,
}

#[derive(Copy, Clone, ValueEnum)]
pub enum OutKindArg {
    Sell,
    Spend,
    Gift,
    Donate,
}

impl From<MethodLotArg> for LotMethod {
    fn from(a: MethodLotArg) -> Self {
        match a {
            MethodLotArg::Fifo => LotMethod::Fifo,
            MethodLotArg::Lifo => LotMethod::Lifo,
            MethodLotArg::Hifo => LotMethod::Hifo,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
pub enum MethodArg {
    Actual,
    ProRata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Render the LONG help (`--help`) of a subcommand identified by its path, recursing into
    /// nested subcommands (e.g. `["reconcile", "import-selections"]`). Mirrors what a user sees at
    /// `btctax <path...> --help`, which includes each argument's verbatim long-help.
    fn long_help_of(path: &[&str]) -> String {
        let mut cmd = Cli::command();
        for name in path {
            cmd = cmd
                .find_subcommand(name)
                .unwrap_or_else(|| panic!("subcommand {name:?} exists"))
                .clone();
        }
        cmd.render_long_help().to_string()
    }

    // Requirement 3, `--help` half: each file/format-taking arg's long-help carries its FORMAT +
    // a text EXAMPLE. Tokens are comma/brace-joined (no spaces) so help-wrapping can never break
    // them (verified against the real binary output). This is the single source of truth that
    // clap_mangen also renders into the per-subcommand man page (Task 2).

    #[test]
    fn help_documents_key_backup_format() {
        let h = long_help_of(&["init"]);
        assert!(
            h.contains("-----BEGIN PGP PRIVATE KEY BLOCK-----"),
            "init --key-backup help must document the ASCII-armored key format:\n{h}"
        );
    }

    #[test]
    fn help_documents_backup_key_format() {
        let h = long_help_of(&["backup-key"]);
        assert!(
            h.contains("-----BEGIN PGP PRIVATE KEY BLOCK-----"),
            "backup-key --out help must document the ASCII-armored key format:\n{h}"
        );
    }

    #[test]
    fn help_documents_export_snapshot_format() {
        let h = long_help_of(&["export-snapshot"]);
        // The exact removals.csv header read from the render.rs writer.
        assert!(
            h.contains("event,kind,removed_at,lot,sat,basis,fmv_at_transfer"),
            "export-snapshot --out help must document the projection CSV headers:\n{h}"
        );
    }

    #[test]
    fn help_documents_import_selections_format() {
        let h = long_help_of(&["reconcile", "import-selections"]);
        assert!(
            h.contains("disposal_ref,origin_event_id,split_sequence,sat"),
            "import-selections help must document the required CSV header:\n{h}"
        );
    }

    #[test]
    fn help_documents_classify_raw_format() {
        let h = long_help_of(&["reconcile", "classify-raw"]);
        // The exact externally-tagged serde shape (Usd = decimal string, sat = integer).
        assert!(
            h.contains(r#"{"Acquire":{"sat":2000000,"usd_cost":"1680.00","fee_usd":"5.00","basis_source":"ExchangeProvided"}}"#),
            "classify-raw --payload-json help must document the JSON payload shape:\n{h}"
        );
    }

    #[test]
    fn help_documents_select_lots_format() {
        let h = long_help_of(&["reconcile", "select-lots"]);
        assert!(
            h.contains("import|coinbase|X#0:25000"),
            "select-lots --from help must document the <event>#<split>:<sat> pick format:\n{h}"
        );
    }
}
