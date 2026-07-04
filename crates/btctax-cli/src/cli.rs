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
    },
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
        /// Hypothetical sale amount in satoshis (required).
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
        in_ref: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        fmv: Option<String>,
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
    /// zero-basis advisory when omitted); `--acquired` defaults to the receipt date (short-term).
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
        event: String,
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
        /// `raw` (unclassified row placeholder), or `conflict` (import conflict accept-first). Omit = all.
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
}

#[derive(Copy, Clone, ValueEnum)]
pub enum SelfTransferActionArg {
    /// Same-wallet passthrough → SelfTransferPassthrough (both legs skipped, non-taxable).
    Drop,
    /// Cross-wallet transfer → TransferLink (relocate the lots to the destination wallet).
    Relocate,
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
