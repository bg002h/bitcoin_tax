//! `tax-profile` command helpers — set/show the per-year `TaxProfile` side-table entry.
//! `report_tax_year` (Task 9) provides the standalone "tax owed / what-if" calculator.
//! `report_tax_year` also runs the M4 carryforward-consistency advisory (Task 10).
use crate::{return_inputs, tax_profile, CliError, Session};
use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::tables::FullReturnTables;
use btctax_core::{
    carryforward_consistency, compute_se_tax, compute_tax_year, schedule_d, se_net_income,
    ScheduleDTotals, TaxOutcome, TaxProfile, TaxTables, Usd,
};
use btctax_store::Passphrase;
use std::path::Path;

/// Persist `p` as the tax profile for `year` in the vault at `vault`, then save.
///
/// **D-4 guard (SPEC §4.12):** when full-return `ReturnInputs` already exist for the year, a raw
/// `tax-profile` would be IGNORED (`resolve_profile` gives `ReturnInputs` precedence). Refuse rather than
/// silently store an unused figure — the two-sources-of-truth cardinal sin — unless `force` is set.
pub fn set_profile(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    p: TaxProfile,
    force: bool,
) -> Result<(), CliError> {
    let mut s = Session::open(vault, pp)?;
    if !force && return_inputs::exists(s.conn(), year)? {
        return Err(CliError::Usage(format!(
            "tax year {year} already has full-return inputs (`income import`); a raw tax-profile would be \
             ignored (full-return inputs take precedence). Re-run with --force to store it anyway."
        )));
    }
    tax_profile::set(s.conn(), year, &p)?;
    s.save()
}

/// Return the stored `TaxProfile` for `year` from the vault at `vault`, or `None`.
pub fn show_profile(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<Option<TaxProfile>, CliError> {
    tax_profile::get(Session::open(vault, pp)?.conn(), year)
}

/// `income import` — parse a full-return [`ReturnInputs`] from a TOML file (offline; key order in the file
/// is irrelevant to deserialization) and persist it in the `return_inputs` side-table for `year`.
///
/// ★★★ **T4 / `SPEC_interview.md` R11 — IT SCREENS BEFORE IT WRITES.** Until T4 this wrote a
///     committed, UNSCREENED row on every year with no `FullReturnParams`, so a TOML carrying
///     `documents.k1 = true` was stored and poisoned the year at `resolve.rs` precedence 1 (the
///     `SPEC_input_surface.md` §3.2 hazard) until `report` finally said so. Now
///     [`screen_param_free`] runs on the assembled row first and a refusal writes nothing. The
///     rules that read the year's package (§6413(c), §402(g), §904(j)) still wait for commit, and
///     the UNANSWERED tier is deliberately not run here — see `ScreenTier`.
///
/// `discard_draft` is `--discard-draft`: the T4 confirmation that a work-in-progress draft holding
/// an interview may be superseded by this write. It is a SEPARATE flag from `force`
/// (`--force`, the scrub-marker override), which documents itself as overriding that guard *"and
/// NOTHING else"*.
///
/// [`screen_param_free`]: btctax_core::tax::return_refuse::screen_param_free
pub fn import_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    file: &Path,
    force: bool,
    discard_draft: bool,
) -> Result<(), CliError> {
    let text = std::fs::read_to_string(file)?;
    // ★★★ §4.3 — THE MARKER GUARD, A PRE-PARSE SCAN OF THE FILE TEXT.
    //
    //     It lives HERE and not in `parse_return_inputs_toml`, which the round-trip test also calls:
    //     a guard in the parser would make the parser unable to read its own output. And it is a
    //     text scan rather than a key lookup because the marker is a COMMENT — TOML parsing discards
    //     comments, and an unknown KEY would be rejected before any guard could run.
    //
    //     What it stops: a scrubbed file is schema-identical to a real one and this is an
    //     unconfirmed whole-blob upsert, so importing one over a real vault destroys that vault's
    //     identity and IP PIN — unrestorable — and leaves a synthetic SSN well-formed enough to
    //     print on a filed 1040.
    if !force && text.contains(btctax_core::tax::scrub::SCRUB_PROVENANCE_MARKER) {
        return Err(CliError::ImportOfScrubbedFile {
            path: file.display().to_string(),
        });
    }
    let mut ri = parse_return_inputs_toml(&text)?;
    // ★★★ **`Computed` IS BTCTAX'S SIGNATURE, AND THE IMPORT SURFACE MUST NOT BE ABLE TO SIGN IT.**
    //
    // Every provenance field is `#[serde(default)]` on `ReturnInputs`, so until this line a
    // hand-written TOML could simply say `capital_loss_carryforward_in_provenance = "computed"` and
    // mint the stamp — reproduced end-to-end: a $99,000 carryover btctax never derived stored as
    // `Computed`, exit 0. That is not a cosmetic mislabel. `Computed` is read as *"btctax derived
    // this from a year it actually computed"*, and three surfaces act on it: `m4_authority` goes
    // SILENT rather than dispute a figure btctax wrote, `BenefitCarryoversNotStated` stops asking,
    // and the write-back's `--force` guard stops protecting it. A forged stamp buys silence from all
    // three at once, on a figure with no derivation behind it.
    //
    // ★ THE WHOLE CLASS, not the one field the review named. All four carryovers carry a provenance
    //   and every one of them is `#[serde(default)]`; fixing one would leave three doors open and
    //   read as covered. The per-ITEM charitable stamp is normalised too — the preservation block
    //   below filters `existing` items by exactly that field.
    //
    // ★★ NORMALISE, never REFUSE — the key is a legitimate part of the serialized shape, so
    //    rejecting it would make import unable to read a file btctax itself could emit. This also
    //    makes the block below's own comment TRUE — *"a carryover the TOML does supply is the user's
    //    and wins (as `User`)"* — which it was not while the file could say otherwise. Placed BEFORE
    //    the preservation block, which re-stamps `Computed` from the STORED row, where that is
    //    genuinely btctax's own authorship.
    //
    // ★ MEASURED, not assumed, against the one round trip that runs through this function
    //   (`the_scrubbed_toml_round_trips_back_through_import`): planting `Computed` in
    //   `maximal_sentinel` reds it identically WITH and WITHOUT this block, so the stamp was already
    //   being lost on that path before this change and nothing here made it worse. That pre-existing
    //   divergence is filed as FR-18; it is not this branch's to fix.
    {
        use btctax_core::tax::return_inputs::CarryProvenance;
        ri.capital_loss_carryforward_in_provenance = CarryProvenance::User;
        ri.charitable_carryover_in_provenance = CarryProvenance::User;
        ri.qbi.reit_ptp_carryforward_in_provenance = CarryProvenance::User;
        ri.qbi.qbi_carryforward_in_provenance = CarryProvenance::User;
        for item in &mut ri.charitable_carryover_in {
            item.provenance = CarryProvenance::User;
        }
    }
    // ★★★ **THE ANSWER LOG IS BTCTAX'S SIGNATURE ABOUT THE *ASKING*, AND THE IMPORT SURFACE MUST
    //     NOT BE ABLE TO SIGN IT EITHER.** (Seam review C1.)
    //
    // Exactly the same class as the `Computed` block above, one field-set over: `answer_log` and
    // `answer_log_history` are `#[serde(default)]` on `ReturnInputs`, and `income import` is a
    // whole-blob upsert, so until this line the TOML was a SECOND WRITER of the log that
    // `provenance.rs` documents as having exactly one. Both directions were reproduced end to end:
    //
    //   (a) FORGERY. A hand-written TOML mints an `AnswerRecord` btctax never observed — `state =
    //       "given"`, an `answered_on` of the author's choosing, and a `prompt_hash` that MATCHES
    //       the live registry wording, so `answer_status` reads `Given` and R10.3's mismatch
    //       refusal is satisfied by an act that never happened.
    //   (b) SILENT DESTRUCTION, which re-opens the same door from the other side. A routine
    //       re-import that simply omits `[answer_log]` wipes every genuine record. The answered
    //       VALUES survive (they are in the TOML), so the return still stands as testimony while
    //       every trace of when, and under which words, it was given is gone — and because *"an
    //       absent record is not a mismatch"* (`return_refuse.rs`, deliberate), the wording-change
    //       refusal is thereafter permanently disarmed for that return.
    //
    // ★★ NORMALISE, never REFUSE — same reasoning as the `Computed` block: the keys are a
    //    legitimate part of the serialized shape (`income scrub` emits them), so rejecting a file
    //    carrying them would make btctax unable to read a file it emits. Discarding here and
    //    re-attaching the STORED row's below is the only rule under which the log is written by
    //    `record_answer` alone.
    //
    // ★ AND IT SAYS SO. The `Computed` block beside it prints what it preserved; the review's
    //   complaint about the shipped behaviour was that the diligence file went *"with no note at
    //   all"*. A file that carried records is a file whose author expects them to arrive, so the
    //   discard is announced — and only when there was something to discard, so the ordinary import
    //   of a hand-written TOML stays silent.
    let carried = ri.answer_log.len() + ri.answer_log_history.len();
    if carried > 0 {
        eprintln!(
            "note: ignored {carried} answer record(s) in the file. When and in what words a question \
             was answered is recorded by btctax when it asks you — it cannot be imported, because a \
             record of an act this vault never observed would be provenance for something that did \
             not happen. Any records already on the {year} row are kept; run `btctax income answer \
             --year {year}` to answer the questions here."
        );
    }
    ri.answer_log.clear();
    ri.answer_log_history.clear();
    let mut s = Session::open(vault, pp)?;
    // ★ §6.2 (M-1): reconcile the crash-recovery draft BEFORE any committed-row read/write — refuse a
    // parked one (its sole copy, C-1) or a WIP one holding an interview (T4/R11) here, and CLEAR it
    // only on the path that actually writes, below the screen. Checking first and deleting last is
    // what makes "a refusal writes nothing" true of the draft as well as of the committed row.
    let coherence = crate::input_form_store::coherence_check(s.conn(), year, discard_draft)?;
    // §4 R3-M6 (Fable P4.9 r1 I2): `income import` is a whole-blob upsert, so a re-import would SILENTLY
    // DROP a carryover that `report --write-carryover` computed onto this row. For QBI that is a fail-OPEN
    // (losing the REIT/PTP loss carryforward OVERSTATES the QBI deduction ⇒ understates tax). So a
    // **Computed** carryover-in SURVIVES an import that does not itself supply one; a carryover the TOML
    // *does* supply is the user's and wins (as `User`, which the next write-back then refuses to clobber).
    if let Some(existing) = return_inputs::get(s.conn(), year)? {
        use btctax_core::tax::return_inputs::CarryProvenance;
        // ★★★ …AND THE OTHER HALF OF C1: re-attach the STORED row's log, which is the only copy
        //     `record_answer` ever wrote. On a fresh year there is no stored row and this block does
        //     not run, so the log stays empty — a return imported from a file has been ANSWERED by
        //     nobody, which is the truthful record of it. (Unlike the carryover arms below this is
        //     unconditional: there is no "the TOML supplied one" case, because the TOML may not.)
        ri.answer_log = existing.answer_log.clone();
        ri.answer_log_history = existing.answer_log_history.clone();
        let mut preserved: Vec<String> = Vec::new();
        if ri.charitable_carryover_in.is_empty() {
            let computed: Vec<_> = existing
                .charitable_carryover_in
                .iter()
                .filter(|c| c.provenance == CarryProvenance::Computed)
                .cloned()
                .collect();
            if !computed.is_empty() {
                preserved.push(format!("{} charitable carryover item(s)", computed.len()));
                ri.charitable_carryover_in = computed;
            }
            // ★ …and the LIST-LEVEL provenance with them. An empty vec has no per-item provenance,
            //   so this stamp is the only thing that distinguishes "no carryover" from "never
            //   asked"; it reverted to `User` on every re-import.
            if existing.charitable_carryover_in_provenance == CarryProvenance::Computed {
                ri.charitable_carryover_in_provenance = CarryProvenance::Computed;
            }
        }
        // ★★★ PER-FIELD, and the whole-struct version it replaces was a LIVE FAIL-OPEN.
        //
        // The old arm keyed entirely on `reit_ptp_carryforward_in > 0` and then restored the WHOLE
        // `qbi` struct. A filer with a Form 8995 line-3 business-loss carryforward and ZERO REIT/PTP
        // therefore lost the computed `qbi_carryforward_in` on re-import — and `qbi.rs`'s line 4 is
        // `(business_qbi − qbi_carryforward_in).max(0)`, so losing it INFLATES the §199A deduction
        // and UNDERSTATES the tax. That is the exact direction this block's own comment above says
        // it exists to prevent. Two carryforwards, two conditions.
        if ri.qbi.reit_ptp_carryforward_in.is_zero()
            && existing.qbi.reit_ptp_carryforward_in > rust_decimal::Decimal::ZERO
            && existing.qbi.reit_ptp_carryforward_in_provenance == CarryProvenance::Computed
        {
            preserved.push(format!(
                "QBI REIT/PTP carryforward ${:.2}",
                existing.qbi.reit_ptp_carryforward_in
            ));
            ri.qbi.reit_ptp_carryforward_in = existing.qbi.reit_ptp_carryforward_in;
            ri.qbi.reit_ptp_carryforward_in_provenance = CarryProvenance::Computed;
        }
        if ri.qbi.qbi_carryforward_in.is_zero()
            && existing.qbi.qbi_carryforward_in > rust_decimal::Decimal::ZERO
            && existing.qbi.qbi_carryforward_in_provenance == CarryProvenance::Computed
        {
            preserved.push(format!(
                "QBI business-loss carryforward ${:.2}",
                existing.qbi.qbi_carryforward_in
            ));
            ri.qbi.qbi_carryforward_in = existing.qbi.qbi_carryforward_in;
            ri.qbi.qbi_carryforward_in_provenance = CarryProvenance::Computed;
        }
        // ★★ THE FOURTH ARM — the §1212(b) capital-loss carryover, keyed on the PROVENANCE
        //    TRANSITION rather than on `is_zero()`. A computed ZERO is meaningful ("btctax worked
        //    this year out and there is no carryover"), so a value test could not tell it from "the
        //    TOML said nothing". What a fresh parse always yields is `User`; what the write-back
        //    leaves is `Computed`. That transition is the signal.
        if ri.capital_loss_carryforward_in == btctax_core::tax::types::Carryforward::default()
            && ri.capital_loss_carryforward_in_provenance == CarryProvenance::User
            && existing.capital_loss_carryforward_in_provenance == CarryProvenance::Computed
        {
            preserved.push(format!(
                "capital-loss carryover short ${:.2} / long ${:.2}",
                existing.capital_loss_carryforward_in.short,
                existing.capital_loss_carryforward_in.long
            ));
            ri.capital_loss_carryforward_in = existing.capital_loss_carryforward_in;
            ri.capital_loss_carryforward_in_provenance = CarryProvenance::Computed;
        }
        if !preserved.is_empty() {
            eprintln!(
                "note: kept the computed carryover already on the {year} row ({}) — your TOML did not \
                 supply one. To replace it, put the carryover in the TOML (it then counts as user-entered), \
                 or re-run `report --tax-year {} --write-carryover`.",
                preserved.join("; "),
                year - 1
            );
        }
    }
    // ★★★ T4/R11 — THE SCREEN, and it runs BEFORE anything is written, deleted, or ANNOUNCED. A
    //     refusal here leaves the committed row absent and the draft untouched: `s.save()` below is
    //     never reached, so not one byte of the in-memory session reaches disk.
    //
    // ★ It is placed ABOVE the FR-48 note deliberately. That note says *"these inputs are stored
    //   now"*, and printing it and then refusing would tell the filer the opposite of what happened.
    if let Some(refusal) = btctax_core::tax::return_refuse::screen_param_free(&ri) {
        return Err(CliError::Usage(format!(
            "the {year} inputs were NOT stored — [{:?}] {}",
            refusal.reason, refusal.detail
        )));
    }
    // ★ FR-48: a year whose full return cannot compute yet is STORED, with a note — not refused.
    //   `report --tax-year N-1 --write-carryover` legitimately writes onto year N before N's package
    //   exists, and the TUI keeps the same row as a draft; refusing here would break that chain. What
    //   was harmful was `report` then prescribing `income clear` — fixed in `uncomputable_sentence`.
    if let Some(note) = crate::year_readiness::import_note(year, !ri.broker_reporting.0.is_empty())
    {
        eprintln!("{note}");
    }
    crate::input_form_store::coherence_clear(s.conn(), year, &coherence)?;
    return_inputs::set(s.conn(), year, &ri)?;
    s.save()
}

/// Parse a `ReturnInputs` from TOML text (split out for testing).
///
/// ★ P9 §2.3 — REJECTS unknown keys, via `serde_ignored` rather than a hand-written key list (which would
/// be the exact drift-prone hand-wiring P9 abolishes). `serde_ignored` reports every ignored path DURING
/// the same deserialization, so the key set is DERIVED from the type: no list to forget, and `[[w2s]]`
/// arrays, nested tables and comments all work for free. This binds ONLY the CLI's TOML import — the
/// stored-JSON path (`return_inputs::get`) keeps its documented forward-compat and is untouched. Without
/// this, a faithfully-transcribed `box13_retirement_plan` (a deleted field) or a `hsa_present` (the §2.4
/// rename) would import CLEAN and silently vanish — no error, no trace even in `income show`.
fn parse_return_inputs_toml(text: &str) -> Result<ReturnInputs, CliError> {
    // Parse to the TOML tree FIRST (toml's streaming deserializer + serde_ignored mishandles arrays of
    // tables), then run `serde_ignored` over the in-memory `Value` to collect every unknown path.
    let value: toml::Value = toml::from_str(text)
        .map_err(|e| CliError::Usage(format!("invalid ReturnInputs TOML: {e}")))?;
    let mut ignored: Vec<String> = Vec::new();
    let ri: ReturnInputs = serde_ignored::deserialize(value, |path| ignored.push(path.to_string()))
        .map_err(|e| CliError::Usage(format!("invalid ReturnInputs TOML: {e}")))?;
    if !ignored.is_empty() {
        return Err(CliError::Usage(format!(
            "unknown key(s) in the ReturnInputs TOML: {}. btctax does not honor these — likely a typo or a \
             field removed in this version (e.g. `hsa_present` was RENAMED to `sch1.hsa_activity`; \
             `box13_retirement_plan` and `ssn_valid_for_employment` were REMOVED). Fix or delete them, then \
             re-run `btctax income import` — a silently-ignored key would drop data you meant to enter.",
            ignored.join(", ")
        )));
    }
    Ok(ri)
}

/// Redact an SSN/ITIN to `***-**-NNNN` (last 4 digits), or empty/`***-**-****` when too short (review I5).
fn mask_ssn(ssn: &str) -> String {
    if ssn.is_empty() {
        return String::new();
    }
    let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 {
        format!("***-**-{}", &digits[digits.len() - 4..])
    } else {
        "***-**-****".to_string()
    }
}

/// A DISPLAY copy of `ReturnInputs` with all SSNs and the IP-PIN redacted (the stored value is never
/// mutated). Used by `income show` so cleartext PII never reaches stdout/scrollback/pipes (SPEC §4.2).
fn mask_pii(ri: &ReturnInputs) -> ReturnInputs {
    let mut m = ri.clone();
    m.header.taxpayer.ssn = mask_ssn(&m.header.taxpayer.ssn);
    if let Some(sp) = m.header.spouse.as_mut() {
        sp.ssn = mask_ssn(&sp.ssn);
    }
    for d in &mut m.header.dependents {
        d.ssn = mask_ssn(&d.ssn);
    }
    if m.header.ip_pin.is_some() {
        m.header.ip_pin = Some("***".to_string());
    }
    m
}

/// `income scrub` — the stored [`ReturnInputs`] for `year` as SHAREABLE TOML, or `None` if unset.
///
/// ★ TOML, not the JSON `income show` emits, because the whole point is that `income import` can take
/// it straight back — a scrubbed copy nobody can load is a screenshot with extra steps. (`show`'s
/// "serde-toml needs scalars before tables" note is stale: the round trip works.)
pub fn scrub_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<Option<String>, CliError> {
    let s = Session::open(vault, pp)?;

    // ★★★ §7 — READ THE ROW EVERY OTHER READER READS. `return_inputs::get` sees only the COMMITTED
    //     row, but §6.1's precedence is that a version-current DRAFT shadows it. A filer who edited
    //     their return in the input form and has not committed is looking at the draft; scrubbing the
    //     committed row would hand a stranger a DIFFERENT return from the one on their screen — and
    //     the defect they are writing in to report lives in the one on their screen.
    //
    //     ★ A PARKED draft is the sole copy of a screened return, so it must be scrubbed for the same
    //       reason, and more so: there is no committed row behind it to fall back to.
    // ★ T4 fold, M-3: `income scrub` writes nothing (it never calls `s.save()`), so it resolves
    //   through the READ-ONLY seam — an unreadable stale draft is skipped and KEPT, and the note
    //   below (already worded "skipped … Nothing was deleted") is what says so.
    let (loaded, stale) = crate::input_form_store::load_for_read(s.conn(), year)?;
    // ★ Every other caller of `load` surfaces this; scrub was the only one discarding it. A filer
    //   whose WIP draft was schema-stale gets the COMMITTED row scrubbed, and without this they are
    //   not told the draft they were editing was skipped — the soft form of "emits a return the
    //   filer is not looking at", which is the whole reason scrub reads through `load` at all.
    if let Some(note) = stale {
        // ★ Reworded rather than printed verbatim: `StaleNote`'s Display says "DISCARDED a stale
        //   draft", which is true of the writing callers but not of this one — scrub never calls
        //   `s.save()`, so nothing is persistently discarded here. The load-bearing half is that the
        //   draft the filer was editing was SKIPPED, and the committed row is what travels.
        eprintln!(
            "note: your {}-schema draft for {} could not be read by this build (expected v{}), so              the last COMMITTED return is what was scrubbed. Nothing was deleted.",
            note.found, note.year, note.expected
        );
    }
    let ri = match loaded {
        crate::input_form_store::Loaded::Draft { ri, .. } => Some(ri),
        crate::input_form_store::Loaded::Committed(ri) => Some(ri),
        crate::input_form_store::Loaded::Fresh => None,
    };

    // ★★★ SPEC §2.2 — the SCRUB-OWNED refusal, checked before anything is emitted.
    //
    //     The ledger must be PROJECTED to ask the question at all; this path never did, and
    //     assuming an unprojected ledger is empty is the widening the predicate exists to stop. A
    //     projection failure propagates as an error — it is never a fallback to "assume empty".
    //
    //     ★ Checked even when `ri` is None. The refusal is a fact about the YEAR, not about whether
    //       a return happens to be stored, and reporting "no inputs set" to a filer whose ledger
    //       would have blocked the scrub anyway tells them the wrong thing about their vault.
    let (state, _cfg) = s.project()?;
    if let Some(c) = btctax_core::tax::scrub::ledger_contribution(&state, year) {
        return Err(CliError::ScrubLedgerContributes {
            year,
            cause: c.cause().to_string(),
        });
    }

    ri.map(|ri| {
        let scrubbed = btctax_core::tax::scrub::scrub_pii(&ri);
        // ★ NOT `BadConfigValue`, which is documented as "a `cli_config` row held an unrecognized
        //   value (corrupt DB)" — its natural remedy is to clear the row and re-import, which is
        //   DESTRUCTIVE and is the wrong advice for what is really a serializer limitation on
        //   btctax's side. r1's sweep filed this; it reached neither the spec nor the build.
        let body = toml::to_string_pretty(&scrubbed).map_err(|e| {
            CliError::ScrubOutput(format!(
                "could not render the {year} return as TOML: {e}. This is a btctax limitation, not \
                 a problem with your vault — your stored return is untouched. Please report it."
            ))
        })?;
        // ★★ §4.2 — the marker rides on the EMITTED STRING, not on the `--out` path, so stdout and
        //    the file carry it identically. A filer who pipes this to a file must not get an
        //    unmarked one.
        Ok(format!(
            "{}\n{}",
            btctax_core::tax::scrub::SCRUB_PROVENANCE_MARKER,
            body
        ))
    })
    .transpose()
}

/// `income clear` — remove the stored full-return inputs for `year` (recovery path so a year with
/// `ReturnInputs` isn't a dead end while derivation is pending — review I3). Returns whether a row existed.
pub fn clear_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    discard_draft: bool,
) -> Result<bool, CliError> {
    let mut s = Session::open(vault, pp)?;
    // ★ §6.2 (M-1): a parked draft is the sole copy of a screened return — refuse rather than let this
    // clear leave it silently orphaned; a WIP draft holding an interview needs `--discard-draft`
    // (T4/R11); any other WIP draft is cleared alongside the committed-row delete.
    crate::input_form_store::coherence_clear_or_refuse(s.conn(), year, discard_draft)?;
    let removed = return_inputs::delete(s.conn(), year)?;
    s.save()?;
    Ok(removed)
}

/// `income show` — the stored [`ReturnInputs`] for `year` as pretty JSON with PII redacted, or `None`.
/// (JSON, not TOML: serde-toml requires scalar keys before nested tables, which the nested model violates;
/// a TOML round-trip-out is a follow-on. Import accepts TOML.)
pub fn show_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<Option<String>, CliError> {
    let ri = return_inputs::get(Session::open(vault, pp)?.conn(), year)?;
    ri.map(|ri| {
        let mkerr = |e: serde_json::Error| CliError::BadConfigValue {
            key: format!("return_inputs[{year}]"),
            value: e.to_string(),
        };
        // M-1 (DONE, post-v0.7.0): `serde_json` `preserve_order` is enabled workspace-wide, so routing
        // through `to_value` to host the DOB transform now preserves the ReturnInputs struct's declared
        // field order (curated) instead of sorting keys alphabetically. `income show` is display-only and
        // never parsed (M8); typed serde (the STORED serialization) is field-ordered regardless, so the
        // persisted bytes + fingerprints are unaffected by the flip.
        let mut val = serde_json::to_value(mask_pii(&ri)).map_err(mkerr)?;
        format_dobs_readable(&mut val); // UX-P1-5: render date_of_birth as MM/DD/YYYY, not raw [year, ordinal]
        serde_json::to_string_pretty(&val).map_err(mkerr)
    })
    .transpose()
}

/// UX-P1-5: `income show`'s JSON serializes each `time::Date` as a raw `[year, ordinal-day]` array (e.g.
/// `[2012, 106]`), which no filer reads as a calendar date. Rewrite every `date_of_birth` value in the
/// DISPLAY tree to a human `MM/DD/YYYY` string. Display-only — the STORED serialization is untouched
/// (`income show` is for viewing, never parsed back — M8).
fn format_dobs_readable(v: &mut serde_json::Value) {
    use time::macros::format_description;
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map.iter_mut() {
                if k == "date_of_birth" {
                    // Extract MM/DD/YYYY (the closure's immutable borrow of `val` ends before the write).
                    let readable = val.as_array().filter(|a| a.len() == 2).and_then(|a| {
                        let y = a[0].as_i64()? as i32;
                        let o = a[1].as_u64()? as u16;
                        let d = time::Date::from_ordinal_date(y, o).ok()?;
                        d.format(&format_description!("[month]/[day]/[year]")).ok()
                    });
                    if let Some(s) = readable {
                        *val = serde_json::Value::String(s);
                        continue;
                    }
                }
                format_dobs_readable(val);
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(format_dobs_readable),
        _ => {}
    }
}

/// The full `report --tax-year` bundle, in print order. A NAMED STRUCT (was a 7-tuple) so a new field can
/// never silently transpose with an existing one at a call site (Fable IMPL-P4 r1 N1, `p4-r1-n1`).
#[derive(Debug)]
pub struct TaxYearReport {
    /// The frozen crypto-DELTA engine's outcome for the year.
    pub outcome: TaxOutcome,
    /// M4 carryforward-consistency advisory (non-gating).
    pub advisory: Option<String>,
    /// RAW pre-netting Schedule D part totals.
    pub schedule_d: ScheduleDTotals,
    /// Standalone Form 709 gift advisory.
    pub gift_advisory: Option<String>,
    /// Standalone Schedule SE §1401 section.
    pub schedule_se: Option<String>,
    /// §170(f)(11)(F) year-aggregate donation appraisal advisory.
    pub donation_appraisal: Option<String>,
    /// Conservative-filing (D-9) advisory: per-disposal tranche dip lines + per-wallet method-inversion
    /// warnings. Provenance-neutral; non-gating (never affects the outcome or exit code).
    pub tranche_advisory: Option<String>,
    /// The §6 dual-report block (absolute filed return + crypto delta + the P5 advisories). `Some` only
    /// for a `ReturnInputs`-provenance year; `None` on the delta-only path.
    pub dual_report: Option<String>,
    /// UX-P4-1: the pseudo-disclosure channel for this year's figures — the full §3.1 predicate
    /// (`pseudo_active() OR PseudoPlaceholder`, Synthetic-wins). Drives the banner + `[PSEUDO]` suffix on
    /// every number-bearing surface (delta report, dual-report absolute totals, TUI Tax tab) and the
    /// fail-closed `--write-carryover` gate; `None` when the figures are not pseudo-contributed.
    pub pseudo_contributed: crate::render::PseudoDisclosure,
    /// ★ spec 1099-DA T6 — the (provider, cohort) keys the year's Form 8949 rows carry, their row
    /// counts, and the answers on file with the box each chooses. `Some` only when the question is
    /// live (a basis regime and ≥1 keyed row) or an unread answer is stored; printed after the
    /// readiness line, before the figures, so the filer sees what is asked before what was computed.
    pub broker_answers: Option<String>,
    /// ★ spec 1099-DA R6 (M-15) — `export-irs-pdf --tax-year {year}` still prints the crypto slice
    /// from the stored Form 1099-DA answers: the T9 accessor returned answers AND the year's
    /// full-return parameters are not bundled. Passed to `render_tax_outcome`, which appends the
    /// clause to the NOT-COMPUTABLE line so a filer is not told the year is dead.
    pub slice_prints_from_answers: bool,
}

/// Task 9 (B.5) + Task 10 (M4) + P2-D Task 2 + Chunk-1 D2 + Chunk-3a: load events + project once,
/// read the year's `TaxProfile` + `BundledTaxTables`, call `compute_tax_year`, and assemble the
/// standalone Schedule D / Form 709 / Schedule SE artifacts + the M4 carryforward-consistency
/// advisory + the §170(f)(11)(F) year-aggregate donation appraisal advisory. See [`TaxYearReport`]
/// for the returned bundle. The advisory is `Some(msg)` iff BOTH the current-year and the prior-year
/// profiles exist AND the prior-year computes successfully AND the declared `carryforward_in` does
/// not match the prior year's `carryforward_out`. The advisory and the Schedule SE figure are
/// **never** hard blockers and do **not** change the exit code (non-gating).
///
/// `prior_taxable_gifts`: cumulative prior-year TAXABLE gifts (post-annual-exclusion Form 709
/// amounts), not gross gifts. Default $0 (caller passes $0 when the flag is not provided).
/// ★ spec 1099-DA R6 (M-15) — does `export-irs-pdf --tax-year {year}` print the crypto slice from
/// answers this vault already holds? Both halves: the T9 accessor returned a working return with a
/// NON-EMPTY `broker_reporting`, AND the year's full-return parameters are not bundled. The second
/// half matters — with parameters bundled the export files the FULL return, and telling the filer
/// about a slice would name an artifact they will not get.
fn stored_answers_reach_the_slice(
    s: &Session,
    year: i32,
    fr: &dyn btctax_core::tax::tables::FullReturnTables,
) -> Result<bool, CliError> {
    if fr.full_return_for(year).is_some() {
        return Ok(false);
    }
    // ★ spec 1099-DA R6 fold (I-1) — the THIRD term: the year's Form 8949 / Schedule D templates
    //   have to be bundled or the export refuses and writes nothing. TY2026 satisfies both of the
    //   other two and cannot print a page.
    if !crate::year_readiness::slice_can_print(year) {
        return Ok(false);
    }
    Ok(crate::input_form_store::broker_answers(s.conn(), year)?.is_some_and(|b| !b.0.is_empty()))
}

pub fn report_tax_year(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    prior_taxable_gifts: Usd,
) -> Result<TaxYearReport, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, state, cfg) = s.load_events_and_project()?;
    // Pseudo-reconcile (sub-project 2, [R0-M6]): when the mode is ON and the year has NO stored profile,
    // inject a CLI-layer PLACEHOLDER profile (single filer, $0 income/MAGI/qual-div) so the estimate can
    // proceed with zero setup. This clears `TaxProfileMissing` ONLY — it is injected AFTER the projection,
    // so it never touches `state.blockers` and thus can NEVER clear the Hard `TaxYearNotComputable` gate
    // (compute.rs checks Hard blockers BEFORE the profile branch). A real stored profile always wins.
    // Single resolver + BOTH refuse-guards, fail-closed (SPEC §4.12 / §4.10 / G4): ReturnInputs (derived,
    // input- AND compute-screened) → stored TaxProfile → pseudo → missing. `resolve_and_screen` is the one
    // entry point every computing consumer shares so the app never shows two liabilities for one year.
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (profile, provenance) = match crate::resolve::resolve_and_screen(
        s.conn(),
        &state,
        year,
        cfg.pseudo_reconcile,
        fr_tables.full_return_for(year),
        tables.table_for(year),
    )? {
        crate::resolve::ProfileOutcome::Uncomputable { detail } => {
            return Err(CliError::Usage(detail))
        }
        crate::resolve::ProfileOutcome::Ready {
            profile,
            provenance,
        } => (profile, provenance),
    };
    let outcome = compute_tax_year(&events, &state, year, profile.as_ref(), &tables);

    // UX-P4-1: the pseudo-disclosure channel for the figures below. `Synthetic` (a pseudo synthetic
    // lot/FMV feeds the number) wins over `Placeholder` (computed on the all-$0 placeholder profile) — the
    // two are mutually exclusive by precedence though the states can co-occur (SPEC §3.1). Read from the
    // LIVE pseudo-ON projected state + provenance (NOT a pseudo-OFF view — that would zero the count and
    // silence the banner, reinstating the answered-ness false-negative).
    let pseudo_contributed = if state.pseudo_active() {
        crate::render::PseudoDisclosure::Synthetic
    } else if provenance == crate::resolve::Provenance::PseudoPlaceholder {
        crate::render::PseudoDisclosure::Placeholder
    } else {
        crate::render::PseudoDisclosure::None
    };

    // §6 DUAL REPORT (SPEC §6 / §5 stages 1–9): the absolute filed return, side-by-side with the crypto
    // delta above. Only meaningful for a `ReturnInputs`-provenance year — the input-screen + compute-
    // dependent screen have already passed inside the resolver (else we returned `Uncomputable`), and
    // TY2024 is the only year with `FullReturnParams` (so both `Option`s are `Some` here). The absolute
    // path adds `screen_absolute` (the QBI-over-threshold rows), which — unlike the delta path — can
    // refuse the ABSOLUTE return while the delta still computes; render that as a note. (This comment
    // used to list AMT and TI≤0-with-carryforward too; §G-6 built the AMT emitter and widening (A)
    // lifted the carryforward refusal, so neither is a row any more.)
    // ★ spec 1099-DA T6 — the keys the ledger needs answered, beside the answers on file. Pure
    //   render over the census and the stored map; the regime is the year's record (None → no block
    //   unless an unread answer is stored, which the block then explains).
    let broker_answers = {
        // ★ r3 I-2 — the ROWS go in (the census is derived inside), so the block can ENUMERATE each
        //   key's rows with their column (e) — the per-row figure `basis_matches` swears to.
        let rows = btctax_core::form_8949(&state, year);
        // ★ spec 1099-DA T9 — the ONE resolution (§6.1: a draft shadows the committed row; a parked
        //   draft carries none). On a params-less year the TUI draft is the primary authoring
        //   surface, so `return_inputs::get` here would have shown the filer an EMPTY answers block
        //   for the answers `export-irs-pdf` is about to file from.
        let stored = crate::input_form_store::broker_answers(s.conn(), year)?;
        crate::render::render_broker_answers(
            year,
            crate::year_readiness::regime_for(year),
            &rows,
            stored.as_ref(),
        )
    };
    // ★ spec 1099-DA R6 (M-15) — state (2): answers are stored for a year whose full-return
    //   parameters are NOT bundled, so `export-irs-pdf` prints the crypto slice from them. The
    //   NOT-COMPUTABLE line says so, and BOTH halves are computed here rather than in the renderer
    //   (which sees neither the vault nor the bundled tables).
    let slice_prints_from_answers = stored_answers_reach_the_slice(&s, year, &fr_tables)?;
    let dual_report: Option<String> = if provenance == crate::resolve::Provenance::ReturnInputs {
        match (
            crate::return_inputs::get(s.conn(), year)?,
            fr_tables.full_return_for(year),
            tables.table_for(year),
        ) {
            (Some(ri), Some(params), Some(table)) => {
                let ar = btctax_core::assemble_absolute(&ri, &state, params, table, year);
                let regime = crate::year_readiness::regime_or_refuse(year)?;
                match btctax_core::screen_absolute(&ri, &ar, params, &state, year, regime) {
                    Some(refusal) => Some(format!(
                        "\n═══ Absolute filed return (Form 1040) — tax year {year} ═══\n  \
                         Profile source: {}\n  NOT COMPUTABLE [{:?}]: {}\n",
                        crate::render::provenance_label(provenance),
                        refusal.reason,
                        refusal.detail
                    )),
                    None => {
                        // P5: the full-return block carries the §3.4 conservative-omission advisories
                        // (CTC/ODC, EIC, forfeited §63(f) aged box) + the FBAR / charitable-donee
                        // disclosures. Non-gating: they never change a number or the exit code.
                        //
                        // ★ P6.3b: the block renders the PRINTED figures — exactly what the filed PDF
                        // carries. `assemble_printed_forms` is infallible and PII-free, so a household
                        // that has entered no identity yet still sees the real numbers (only the filable
                        // ARTIFACT needs a name and an SSN).
                        let details = s.donation_details()?;
                        let printed = btctax_core::tax::packet::assemble_printed_forms(
                            &ri, &state, &details, &ar, table, year, &events, regime,
                        );
                        let mut block = crate::render::render_dual_report(
                            year,
                            &ar,
                            &printed,
                            &outcome,
                            provenance,
                            pseudo_contributed,
                            // ★★★ FINAL-REVIEW FINDING 1 — §170(f)(8) behind the carryover, or not.
                            btctax_core::tax::return_1040::cwa_unvouched_carryover(
                                &ri, &ar, &state, year,
                            ),
                        );
                        let advs = btctax_core::tax::advisories::advisories_for(
                            &ri, &state, &ar, params, year,
                        );
                        block.push_str(&crate::render::render_advisories(&advs));
                        // ★★★ R4 — the checks the DOCUMENT guarantees, displayed and never written.
                        //     The year's table is present on this branch, so the box-3 wage-base
                        //     ceiling runs too.
                        block.push_str(&crate::render::render_transcription_warnings(
                            &btctax_core::tax::transcription_warnings::transcription_warnings(
                                &ri,
                                Some(table.ss_wage_base),
                                // ★ T9 / R8 — the year's package is present on this branch, so the
                                //   §163(h)(3)(B) aggregate ceiling check runs too.
                                Some(params.acquisition_debt_ceiling),
                            ),
                        ));
                        Some(block)
                    }
                }
            }
            _ => {
                // ReturnInputs provenance implies the inputs + TY2024 params/table are present (else the
                // resolver returned Uncomputable) — fail loud in debug if that invariant ever breaks.
                debug_assert!(
                    false,
                    "ReturnInputs provenance but missing inputs/params/table for year {year}"
                );
                None
            }
        }
    } else {
        None
    };
    // P2-B: the RAW pre-netting Schedule D part totals for the same year, from the same projection.
    let sched_d = schedule_d(&state, year);
    // P2-C Task 3 + Chunk-3a: standalone Form 709 gift advisory + §2505 lifetime-exclusion
    // consumption (does NOT feed engine B). prior_taxable_gifts comes from the CLI flag.
    let gift_advisory =
        crate::render::render_gift_advisory(&state, year, prior_taxable_gifts, &tables);
    // P2-D Task 2: standalone Schedule SE §1401 SE-tax figure (STANDALONE — does NOT feed engine B;
    // `total_federal_tax_attributable` is UNCHANGED by SE tax, D5). Requires the year's filing status
    // (from the profile). Business SE income present but no bundled table → the render emits a
    // "wage base unavailable" note (no silent drop); no business SE income → no Schedule SE section.
    let schedule_se = match profile.as_ref() {
        Some(p) => {
            let gross_se = se_net_income(&state, year);
            let table_opt = tables.table_for(year);
            let table_present = table_opt.is_some();
            let se_result = table_opt.and_then(|t| {
                compute_se_tax(
                    &state,
                    year,
                    p.filing_status,
                    t,
                    p.w2_ss_wages,
                    p.w2_medicare_wages,
                    p.schedule_c_expenses,
                )
            });
            crate::render::render_schedule_se(
                year,
                se_result.as_ref(),
                gross_se,
                table_present,
                p.schedule_c_expenses,
                p.w2_ss_wages,
                p.w2_medicare_wages,
            )
        }
        None => None,
    };
    // Chunk-1 D2: §170(f)(11)(F) year-aggregate donation appraisal advisory (STANDALONE — does NOT
    // enter state.advisory / the blocker set; render-time only, consistent with the standalone-forms
    // pattern). Non-gating; does not affect the exit code.
    let donation_appraisal_advisory =
        crate::render::render_donation_appraisal_advisory(&state, year);

    // Conservative-filing (P3 / D-9): tranche dip + method-inversion advisory. Non-gating; render-time
    // only, like the standalone-forms advisories above. The shared core assembler keeps the CLI + TUI
    // surfaces identical.
    let tranche_advisory = btctax_core::conservative::tranche_report_advisory(
        &state,
        &events,
        s.prices(),
        &cfg,
        year,
        profile.as_ref(),
        &tables,
    );

    // M4 carryforward consistency advisory (Task 10): only when both this year's profile AND
    // the prior year's profile exist AND the prior year is Computed.  Never a hard blocker.
    let advisory: Option<String> = if let Some(p) = &profile {
        // Prior-year profile through the same resolver (ReturnInputs-derived too); the M4 advisory is
        // non-gating, so an uncomputable/refused prior year just skips it rather than failing the report.
        let prior_profile = match s.resolve_screened(&state, year - 1, &tables)? {
            crate::resolve::ProfileOutcome::Ready { profile, .. } => profile,
            crate::resolve::ProfileOutcome::Uncomputable { .. } => None,
        };
        if let Some(prev_p) = prior_profile {
            let prior_out = compute_tax_year(&events, &state, year - 1, Some(&prev_p), &tables);
            if let TaxOutcome::Computed(prev) = prior_out {
                // ★★★ F1 (phase-1 seam review). WHICH figure is the authority on last year's
                // carryforward-out? Until N1 there was only one answer, so this compared against the
                // frozen delta engine's flat "loss − $3,000". N1 made the §1212(b)(2)(B) worksheet the
                // correct figure on a full-return year, and the two DIVERGE exactly on the floor
                // household the worksheet exists for: L4 carries $20,000, the flat rule says $17,000.
                //
                // The seam: N1's advisory tells the filer to enter $20,000, and M4 then told them
                // "verify your prior return" — an instrument DISPUTING the figure the product itself
                // instructed them to enter, phrased as an audit of it. A filer who obeys the audit
                // "corrects" to $17,000 and permanently forfeits $3,000 of capital loss. Neither lane
                // could see it: nobody touched M4, so the base tree was the second lane.
                //
                // ★ The fix is here, at the CALLER, and not only because `compute.rs` is frozen:
                // `carryforward_consistency` is a pure comparison, and CHOOSING the authority is the
                // caller's job. State the mechanism and let it decide — when last year has full-return
                // inputs and its absolute return computes, the worksheet figure IS last year's
                // carryforward-out; otherwise fall back to the flat one, which remains correct for a
                // crypto-slice year that never had a worksheet.
                let worksheet_out = match (
                    crate::return_inputs::get(s.conn(), year - 1)?,
                    fr_tables.full_return_for(year - 1),
                    tables.table_for(year - 1),
                ) {
                    (Some(ri_prev), Some(params), Some(table)) => {
                        let ar_prev = btctax_core::assemble_absolute(
                            &ri_prev,
                            &state,
                            params,
                            table,
                            year - 1,
                        );
                        // A prior year whose return REFUSES has no worksheet figure to be the
                        // authority — fall back rather than quoting a number off a refused return.
                        //
                        // ★ BOTH screens, not just the absolute one (phase-2 review, fold Minor).
                        // spec 1099-DA (build review M-3): the prior year's regime refuses TYPED when its record is
                        // missing — never a silently dropped carryforward
                        let regime_prev = crate::year_readiness::regime_or_refuse(year - 1)?;
                        //   Checking only `screen_absolute` let an INPUT-refused prior year still
                        //   supply "the authority" — e.g. an over-limit mortgage, whose overstated
                        //   line 8a understates taxable income and therefore OVERSTATES the
                        //   worksheet carryforward. That violated this fold's own stated rule.
                        (btctax_core::tax::return_refuse::screen_inputs(&ri_prev, table, params)
                            .is_none()
                            && btctax_core::screen_absolute(
                                &ri_prev,
                                &ar_prev,
                                params,
                                &state,
                                year - 1,
                                regime_prev,
                            )
                            .is_none())
                        .then_some(ar_prev.capital_loss_carryforward_out)
                    }
                    _ => None,
                };
                // ★★★ **M4 MUST NOT DISPUTE A FIGURE BTCTAX ITSELF WROTE AND STAMPED.**
                //
                // Two changes, and both exist because (B) made btctax an AUTHOR of this figure
                // rather than only a reader of it.
                //
                // (1) ROUND BOTH SIDES. `carryforward_consistency`'s comparison is EXACT
                //     (`compute.rs`, and it is frozen), while the persisted figure is now
                //     whole-dollar and the worksheet authority is not. A filer carrying btctax's own
                //     $42,872 against an authority of $42,871.66 would be audited by btctax for a
                //     rounding it performed itself.
                //
                // (2) STAY SILENT rather than fall back to the FLAT figure, when the worksheet
                //     authority is unavailable AND this year's carryover-in is btctax's own
                //     `Computed` stamp. Year Y−1 can stop screening clean without a single edit to
                //     it — a new ≥$250 donation Removal flips an itemizing year into
                //     `CharitableCwaUnresolved` — and the flat figure is the crypto slice, which
                //     on a floor household is up to the whole §1211(b) allowance smaller. Quoting
                //     it to dispute a value btctax wrote is the F1 defect arriving from the other
                //     side, and a filer who obeys the audit forfeits deductible loss permanently.
                //     A `User` carryover-in is different: btctax did not write it, so the flat
                //     figure remains the best cross-check it has.
                // The PROVENANCE lives on `ReturnInputs`, not on the derived `TaxProfile` — a
                // crypto-slice year has no row at all, and for it `User` (no btctax authorship) is
                // the right reading.
                let this_year_provenance = crate::return_inputs::get(s.conn(), year)?
                    .map(|r| r.capital_loss_carryforward_in_provenance)
                    .unwrap_or_default();
                m4_authority(worksheet_out, this_year_provenance, prev.carryforward_out).and_then(
                    |a| {
                        carryforward_consistency(
                            Some(&round_carryforward(a)),
                            &round_carryforward(p.capital_loss_carryforward_in),
                        )
                    },
                )
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(TaxYearReport {
        outcome,
        advisory,
        schedule_d: sched_d,
        gift_advisory,
        schedule_se,
        donation_appraisal: donation_appraisal_advisory,
        tranche_advisory,
        dual_report,
        pseudo_contributed,
        broker_answers,
        slice_prints_from_answers,
    })
}

/// Whole-dollar both limbs of a carryforward, for a comparison against a figure that is stored in
/// whole dollars.
///
/// `carryforward_consistency` compares EXACTLY (`compute.rs`, frozen), and since `--write-carryover`
/// began persisting a rounded figure the two sides stopped being on the same scale: a filer carrying
/// btctax's own $42,872 against an exact $42,871.66 authority would be audited by btctax for a
/// rounding btctax performed itself.
fn round_carryforward(
    c: btctax_core::tax::types::Carryforward,
) -> btctax_core::tax::types::Carryforward {
    btctax_core::tax::types::Carryforward {
        short: btctax_core::conventions::round_dollar(c.short),
        long: btctax_core::conventions::round_dollar(c.long),
    }
}

/// ★★★ WHICH figure is the authority on last year's capital-loss carryforward-out — or NONE.
///
/// Three cases, and the third is new with (B):
///   * a WORKSHEET figure exists (year Y−1 has full-return inputs, params and a table, and screens
///     clean) → it is the authority, always. This is the F1 fix.
///   * no worksheet figure, and this year's carryover-in is the FILER's (`User`) → the crypto-slice
///     flat figure is the best cross-check btctax has, and it fires.
///   * no worksheet figure, and this year's carryover-in is btctax's OWN `Computed` stamp → **SAY
///     NOTHING.** Quoting the flat figure to dispute a value btctax wrote and stamped is the F1
///     defect arriving from the other side. On a floor household the flat figure is smaller by up to
///     the whole §1211(b) allowance, and a filer who obeys the audit forfeits that permanently.
///     Year Y−1 can stop screening clean with no edit to it at all — a new ≥$250 donation Removal
///     flips an itemizing year into the §170(f)(8) gate — so this is not a hypothetical.
///
/// ★★ **EXTRACTED AS A FUNCTION BECAUSE THE THIRD CASE IS UNREACHABLE IN v1, AND AN UNTESTED GUARD
/// IS THE SHAPE THIS REPO KEEPS SHIPPING DEFECTS IN.** Reaching it end-to-end needs year Y to carry
/// a `ReturnInputs` row (so Y = 2024, the only full-return year v1 has) AND year Y−1 to have a tax
/// table (2023 has none), so no CLI-level fixture can produce it. It becomes reachable the moment
/// TY2025 full-return support lands — which is the same acceptance as "v1 cannot read the row it
/// writes". Pulled out here so the DECISION can be exercised directly and watched go red.
fn m4_authority(
    worksheet_out: Option<btctax_core::tax::types::Carryforward>,
    this_year_provenance: btctax_core::tax::return_inputs::CarryProvenance,
    flat: btctax_core::tax::types::Carryforward,
) -> Option<btctax_core::tax::types::Carryforward> {
    use btctax_core::tax::return_inputs::CarryProvenance;
    match (worksheet_out, this_year_provenance) {
        (Some(w), _) => Some(w),
        (None, CarryProvenance::Computed) => None,
        (None, _) => Some(flat),
    }
}

/// ★★★ **THE CARRYFORWARD CHAIN — year `year`'s COMPUTED carryover-OUTs, stamped onto a destination
/// row, with every gate that decides whether btctax may hand a figure across a year boundary.**
///
/// Extracted from [`write_back_carryover`] when T4b's year-N+1 opener needed the *same* chain: R10.4
/// says the opener's carryforwards are read *"from year N's committed **return** (the carryforward-out
/// chain), never from year N's inputs"*, and that is exactly this computation. Two callers, one
/// definition — a second copy would be a second place for the pseudo / not-computable / §170(f)(8) /
/// restriction gates to be forgotten, and each of those exists because it was forgotten once.
///
/// ★ `destination` is a CLOSURE, not a value, so the ORDER in which refusals are raised is
///   unchanged: `write_back_carryover` fetches year+1's committed row at the point it always did —
///   after `screen_absolute`, not before the pseudo gate — while the opener hands over a fresh seed.
///
/// Returns the destination row with the carryovers written, alongside year `year`'s frozen return and
/// its inputs, because the SUMMARY has to ask [`btctax_core::capital_loss_roll_is_grounded`] the same
/// question the write asked.
pub(crate) struct RolledCarryover {
    /// The destination row, with year `year`'s carryover-OUTs stamped on (provenance `Computed`).
    pub updated: btctax_core::tax::return_inputs::ReturnInputs,
    /// Year `year`'s FROZEN return — what every figure above was read off.
    pub ar: btctax_core::AbsoluteReturn,
    /// Year `year`'s committed inputs.
    pub ri: btctax_core::tax::return_inputs::ReturnInputs,
}

pub(crate) fn roll_carryover_onto(
    s: &Session,
    year: i32,
    destination: impl FnOnce(
        &Session,
    ) -> Result<btctax_core::tax::return_inputs::ReturnInputs, CliError>,
    force: bool,
) -> Result<RolledCarryover, CliError> {
    let (events, state, cfg) = s.load_events_and_project()?;
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (Some(params), Some(table)) = (fr_tables.full_return_for(year), tables.table_for(year))
    else {
        return Err(CliError::Usage(format!(
            "no full-return tables for {year} — carryover write-back needs a supported tax year (TY2024)"
        )));
    };
    // Must be a ReturnInputs-provenance year with both refuse screens passed (fail-closed).
    let (profile, provenance) = match crate::resolve::resolve_and_screen(
        s.conn(),
        &state,
        year,
        cfg.pseudo_reconcile,
        Some(params),
        Some(table),
    )? {
        crate::resolve::ProfileOutcome::Uncomputable { detail } => {
            return Err(CliError::Usage(detail))
        }
        crate::resolve::ProfileOutcome::Ready {
            profile,
            provenance,
        } => (profile, provenance),
    };
    if provenance != crate::resolve::Provenance::ReturnInputs {
        return Err(CliError::Usage(format!(
            "carryover write-back needs full-return inputs for {year} (`income import`); the resolved \
             profile source is {provenance:?}"
        )));
    }
    // UX-P4-1 surface 4 (SPEC §3.1 clause 4) [T-C1 + G2-NEW-4]: NEVER persist a carryover derived from a
    // pseudo-tainted OR hard-blocked ledger into year+1's stored inputs. Next year `pseudo_active()` is
    // false and the UX-P4-1 banner correctly does not fire — so an unflagged, deliberately-fictional (or
    // unanswerable) figure would ride into a real input. Fail-closed, consistent with the export gate.
    // (4a) At this gate the `PseudoPlaceholder` disjunct is structurally inert (provenance is ReturnInputs,
    // just checked), so `pseudo_active()` is the operative half of the §3.1 predicate.
    if state.pseudo_active() {
        return Err(CliError::Usage(format!(
            "carryover write-back REFUSED for {year}: pseudo-reconcile mode is contributing synthetic \
             default(s), so the derived carryover is an ESTIMATE — persisting it as {next}'s real input \
             would launder a deliberately-synthetic figure. Resolve the pseudo entries (or turn the mode \
             off) first.",
            next = year + 1
        )));
    }
    // (4b) A `NotComputable` crypto-delta means the ledger carries Hard blockers the engine refuses to
    // answer for; a carryover assembled over that state must not be persisted (the same laundering class
    // minus the pseudo mechanism).
    if let btctax_core::TaxOutcome::NotComputable(b) =
        compute_tax_year(&events, &state, year, profile.as_ref(), &tables)
    {
        return Err(CliError::Usage(format!(
            "carryover write-back REFUSED for {year}: the crypto-delta ledger is NOT COMPUTABLE [{:?}]: {} \
             — a carryover from an unanswerable ledger must not be written into {next}'s inputs.",
            b.kind,
            b.detail,
            next = year + 1
        )));
    }
    let ri = crate::return_inputs::get(s.conn(), year)?
        .ok_or_else(|| CliError::Usage(format!("no return_inputs stored for {year}")))?;
    let ar = btctax_core::assemble_absolute(&ri, &state, params, table, year);
    let regime = crate::year_readiness::regime_or_refuse(year)?;
    if let Some(refusal) = btctax_core::screen_absolute(&ri, &ar, params, &state, year, regime) {
        return Err(CliError::Usage(format!(
            "the {year} absolute return is not computable [{:?}]: {} — carryover not written",
            refusal.reason, refusal.detail
        )));
    }
    let next = destination(s)?;
    let updated = btctax_core::apply_carryover_writeback(&ar, &ri, &state, year, next, force)
        .map_err(CliError::Usage)?;
    Ok(RolledCarryover { updated, ar, ri })
}

/// §4 R3-M6 carryover write-back — persist year `year`'s computed charitable, QBI business-loss,
/// QBI-REIT/PTP and §1212(b) capital-loss carryover-OUTs
/// as year (`year+1`)'s carryover-IN in the side-table. Only for a `ReturnInputs`-provenance full-return
/// year (else there is no absolute return). Errors if the absolute return refuses (`screen_absolute`) or if
/// a user-entered next-year carryover would be overwritten without `force`. Returns a human summary.
pub fn write_back_carryover(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    force: bool,
    discard_draft: bool,
) -> Result<String, CliError> {
    let mut s = Session::open(vault, pp)?;
    // ★ §6.2 (M-1): write-back reads AND writes the year+1 committed row, so it reconciles the year+1
    // draft here — before the year+1 read below, which early-returns on an absent row (a parked year has
    // none) and would otherwise shadow the parked-refuse remedy.
    // ★ T4/R11: `discard_draft` is `--discard-draft`, and it is scoped to YEAR+1's draft — the year
    //   this command writes onto. It is separate from `force`, which overrides the write-back's own
    //   guard on a user-entered carryover and says nothing about a draft.
    crate::input_form_store::coherence_clear_or_refuse(s.conn(), year + 1, discard_draft)?;
    // ★★★ THE CHAIN, shared with T4b's year-N+1 opener — see [`roll_carryover_onto`]. The `next`
    //     row is fetched inside it, at the point it always was, by the closure below.
    let mut next_original = None;
    let rolled = roll_carryover_onto(
        &s,
        year,
        |s| {
            // SPEC §4 R3-M6 writes the carryover "as year (Y+1)'s `*_carryover_in` **on that row**" — the row must
            // ALREADY exist. Fabricating one would put a `ReturnInputs` row at the TOP of the §4.12 precedence
            // ladder for a year v1 has no full-return tables for (Y+1 is always 2025 in v1), which fails closed and
            // would make that year uncomputable — shadowing a stored `TaxProfile` the user was planning with, and
            // blocking `tax-profile --year Y+1` via the D-4 guard (Fable P4.9 r1 I1).
            let next = crate::return_inputs::get(s.conn(), year + 1)?.ok_or_else(|| {
            CliError::Usage(format!(
                "year {next} has no full-return inputs yet — the carryover is written onto that row, so import \
                 it first (`income import --year {next} --file <toml>`) and then re-run `--write-carryover`. \
                 (Creating the row here would shadow any stored tax-profile for {next} and make it uncomputable \
                 in this version, which supports full returns for TY2024 only.)",
                next = year + 1
            ))
        })?;
            next_original = Some(next.clone());
            Ok(next)
        },
        force,
    )?;
    let RolledCarryover { updated, ar, ri } = rolled;
    let next_original = next_original.expect("the destination closure ran, or the roll errored");
    crate::return_inputs::set(s.conn(), year + 1, &updated)?;
    s.save()?;

    // ★★★ **THE WRITE CAN MAKE NEXT YEAR UNFILABLE, AND IT MUST NOT DO SO IN SILENCE.**
    //
    // A nonzero capital-loss carryover-in makes THREE declarations live on year Y+1 that were not
    // live before it: Form 6251 line 2k, and the Capital Loss Carryover Worksheet's two header
    // conditions. All three are class-(A), so `None` REFUSES — and this is the only write path in
    // btctax that can leave a COMMITTED row in a state `input_form_store` would have refused to
    // create. That invariant held until `--write-carryover` learned to roll the capital loss.
    //
    // ★ WARN, never refuse. The row must EXIST for the answer to be givable at all: refusing here
    //   would leave the filer with a written row, a refusal, and no command that reaches it.
    //
    // ★★ SCREENED WITH YEAR Y'S PARAMS AND TABLE, deliberately, and the reason is that this is a
    //    DELTA. v1 has no full-return params for Y+1 (`full_return_for` is `None` by design), so
    //    there is no year-Y+1 authority to screen against — but both sides of the comparison use the
    //    same authority, so any difference between them is attributable to THE WRITE and to nothing
    //    else. A refusal the row already had is not reported; only one this command created is.
    // ★ The tables are re-loaded rather than carried out of [`roll_carryover_onto`]: `table_for`
    //   borrows from the loader, and the loader is that function's local. The `else` arm cannot be
    //   reached — the roll refuses year `year` outright without both — and it yields no note rather
    //   than a wrong one.
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (before, after) = match (fr_tables.full_return_for(year), tables.table_for(year)) {
        (Some(params), Some(table)) => (
            btctax_core::tax::return_refuse::screen_inputs(&next_original, table, params),
            btctax_core::tax::return_refuse::screen_inputs(&updated, table, params),
        ),
        _ => (None, None),
    };
    let newly_unfilable = match (&before, &after) {
        (None, Some(r)) => Some(format!(
            "\n★ NOTE: {next} now needs an answer it did not need before. Writing a capital-loss \
             carryover onto that row makes declarations live that were not — [{:?}] {} \
             \n  Run `btctax income answer --year {next}` before filing {next}.",
            r.reason,
            r.detail,
            next = year + 1
        )),
        _ => None,
    };

    // ★ THE SUMMARY IS DERIVED FROM WHAT WAS ASSIGNED, never from a hand-list. The previous one named
    //   two carryovers where the code wrote three, and the same drift bit `kat_attestation`'s
    //   enumeration. Each line below reads the field the write-back actually set.
    let mut wrote: Vec<String> = Vec::new();
    wrote.push(format!(
        "{} charitable carryover item(s)",
        updated.charitable_carryover_in.len()
    ));
    wrote.push(format!(
        "QBI REIT/PTP carryforward ${:.2}",
        updated.qbi.reit_ptp_carryforward_in
    ));
    wrote.push(format!(
        "QBI business-loss carryforward ${:.2}",
        updated.qbi.qbi_carryforward_in
    ));
    // ★★★ **AND "DERIVED" MEANS DERIVED FROM THE ASSIGNMENT, NOT FROM THE ROW FIELD.**
    //
    // The widening review's B-1: these lines read `updated.<field>`, so the capital-loss one printed
    // on the branch where `apply_carryover_writeback`'s `grounded` gate DELIBERATELY skipped the
    // write. The three above it are unconditional assignments, so field and assignment coincide;
    // the capital-loss one is the only gated write, and for it they do not.
    //
    // Two shapes, both reproduced before this was written:
    //   * a first roll from an ungrounded year printed *"short $0.00 / long $0.00"* as written back
    //     while nothing was stamped — and next year's `BenefitCarryoversNotStated` stayed live for
    //     exactly that carryover, so the filer held a success message the next advisory contradicts;
    //   * a RE-roll after the grounding was edited away printed the STALE PRIOR FIGURE as written
    //     back. (Observed: `long $34000.00`, unchanged, on a year that no longer has any loss.)
    //
    // ★ It reads `capital_loss_roll_is_grounded` rather than re-deriving the predicate here — one
    //   definition, the writer and the message it prints. Re-deriving would put the same defect one
    //   edit away.
    //
    // ★★ NAMED EITHER WAY, never merely omitted. T9's whole point is that the filer-facing surfaces
    //    account for all four carryovers; a silent gap is a worse answer than a truthful "not this
    //    one, and here is why". The stale case is called out by name because v1 cannot re-read the
    //    row it writes, so nothing downstream will mention it (FR-17).
    let capital_loss_note = if btctax_core::capital_loss_roll_is_grounded(&ar, &ri) {
        wrote.push(format!(
            "capital-loss carryover short ${:.2} / long ${:.2}",
            updated.capital_loss_carryforward_in.short, updated.capital_loss_carryforward_in.long
        ));
        String::new()
    } else {
        let stale = if updated.capital_loss_carryforward_in_provenance
            == btctax_core::tax::return_inputs::CarryProvenance::Computed
        {
            format!(
                " {next} still carries short ${:.2} / long ${:.2} stamped \"computed\" by an EARLIER \
                 roll, and this run did not re-derive it — check it before you file {next}.",
                updated.capital_loss_carryforward_in.short,
                updated.capital_loss_carryforward_in.long,
                next = year + 1
            )
        } else {
            String::new()
        };
        format!(
            "\n★ NOT WRITTEN: the capital-loss carryover. {year} was never asked about one and \
             produced none of its own, so btctax has no §1212(b) figure it can vouch for and stamps \
             nothing.{stale}"
        )
    };
    Ok(format!(
        "carryover written back to {}: {}{}{}",
        year + 1,
        wrote.join("; "),
        capital_loss_note,
        newly_unfilable.unwrap_or_default()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::return_inputs::CharitableClass;
    use btctax_core::FilingStatus;
    use rust_decimal_macros::dec;

    /// ★★★ **K13 — M4 never disputes a figure btctax itself wrote, and never audits its own
    /// rounding.**
    ///
    /// Exercised on [`m4_authority`] and [`round_carryforward`] directly, and the reason is recorded
    /// rather than glossed: **the silent case is UNREACHABLE end-to-end in v1.** It needs year Y to
    /// carry a `ReturnInputs` row — so Y = 2024, the only full-return year v1 has — AND year Y−1
    /// (2023) to have a tax table, which it does not. No CLI fixture can produce it. It becomes
    /// reachable the moment TY2025 full-return support lands, which is the same acceptance as
    /// "v1 cannot read the row it writes".
    ///
    /// ★ A guard that cannot be watched going red is exactly the shape this repo keeps shipping
    /// defects in (harness B1), which is why the decision was pulled out of the `if` chain into a
    /// function rather than left inline and untested. What is asserted is the DECISION; what the
    /// CLI-level `the_prior_year_worksheet_figure_is_the_m4_authority_for_a_floor_year` asserts is
    /// that the decision is wired to the report.
    ///
    /// Mutations that MUST red:
    ///   (a) restore the bare `worksheet_out.unwrap_or(flat)` in `m4_authority`;
    ///   (b) make `round_carryforward` the identity.
    #[test]
    fn m4_never_disputes_a_computed_figure_it_cannot_re_derive() {
        use btctax_core::tax::return_inputs::CarryProvenance;
        use btctax_core::tax::types::Carryforward;

        let worksheet = Carryforward {
            short: dec!(40000),
            long: dec!(60000),
        };
        let flat = Carryforward {
            short: dec!(37000),
            long: dec!(60000),
        };
        assert_ne!(
            worksheet, flat,
            "premise: the two engines must DISAGREE, or 'which is the authority' is not a question"
        );

        // The worksheet figure wins whenever it exists — under either provenance (F1).
        for prov in [CarryProvenance::User, CarryProvenance::Computed] {
            assert_eq!(
                m4_authority(Some(worksheet), prov, flat),
                Some(worksheet),
                "the prior year's WORKSHEET figure is the authority ({prov:?})"
            );
        }

        // No worksheet + the FILER's own figure ⇒ the flat one is the best cross-check there is.
        assert_eq!(
            m4_authority(None, CarryProvenance::User, flat),
            Some(flat),
            "a User carryover-in is not btctax's, so it is still cross-checked"
        );

        // ★★★ No worksheet + BTCTAX'S OWN figure ⇒ SILENCE.
        assert_eq!(
            m4_authority(None, CarryProvenance::Computed, flat),
            None,
            "★★★ M4 must NOT quote the crypto-slice flat figure to dispute a value btctax itself \
             wrote and stamped `Computed`. On a floor household the flat figure is smaller by up to \
             the whole §1211(b) allowance, and a filer who obeys that audit forfeits it permanently."
        );

        // ── The ROUNDING half: btctax must not audit its own rounding. ────────────────────────────
        let exact = Carryforward {
            short: Usd::ZERO,
            long: dec!(42871.66),
        };
        let stored = Carryforward {
            short: Usd::ZERO,
            long: dec!(42872),
        };
        assert_ne!(
            exact, stored,
            "premise: exact and stored differ, which is the whole reason both sides are rounded"
        );
        assert_eq!(
            round_carryforward(exact),
            round_carryforward(stored),
            "★★ a filer's whole-dollar $42,872 against an exact $42,871.66 authority must NOT fire — \
             btctax would be auditing a rounding it performed itself"
        );
        assert!(
            carryforward_consistency(
                Some(&round_carryforward(exact)),
                &round_carryforward(stored)
            )
            .is_none(),
            "…and the advisory it feeds must therefore stay silent"
        );
        // …while a REAL disagreement still fires.
        assert!(
            carryforward_consistency(Some(&round_carryforward(exact)), &round_carryforward(flat))
                .is_some(),
            "a genuine mismatch must still be reported — rounding both sides must not mute M4"
        );
    }

    /// Shared temp-vault fixture (mirrors `input_form_store.rs`'s helper, M-3): `create` + drop releases
    /// the store single-instance lock so a later `Session::open` (here, the one inside the command under
    /// test) can re-acquire it. The `TempDir` guard MUST be kept alive by the caller.
    fn tmp_vault() -> (tempfile::TempDir, std::path::PathBuf, Passphrase) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.pgp");
        {
            let _ = Session::create(&path, &Passphrase::new("test-pass".into())).unwrap();
        }
        (dir, path, Passphrase::new("test-pass".into()))
    }

    /// ★ §6.2 wiring — `income clear` REFUSES a year that holds a PARKED draft (the draft is the sole copy
    /// of a screened return, C-1), and never destroys it. `clear_return_inputs` needs no pre-existing
    /// committed row, so the coherence call is the cheapest reachable parked-refuse: this test pins the
    /// wiring into a real writer. Remove the `coherence_clear_or_refuse` call from `clear_return_inputs`
    /// and this goes red (mutation-check b).
    #[test]
    fn income_clear_refuses_a_parked_draft_and_preserves_it() {
        let (_dir, path, pp) = tmp_vault();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        {
            let mut s = Session::open(&path, &pp).unwrap();
            crate::input_form_store::set_draft_row(s.conn(), 2024, &ri, true).unwrap(); // parked
            s.save().unwrap();
        }
        let err = clear_return_inputs(&path, &pp, 2024, false).unwrap_err();
        assert!(
            matches!(err, CliError::ParkedDraftBlocksWrite { year: 2024 }),
            "income clear must refuse a parked-draft year, got {err:?}"
        );
        // the parked draft is STILL present — a committed-row write never silently destroys it.
        let s = Session::open(&path, &pp).unwrap();
        assert!(
            crate::input_form_store::draft_exists(s.conn(), 2024).unwrap(),
            "a refused clear must leave the parked draft intact"
        );
    }

    /// A representative `income import` TOML deserializes into `ReturnInputs` — exercises money-as-string
    /// (serde-str), the FilingStatus/Owner/CharitableClass enum reprs, and nested `[[w2s]]` / charitable
    /// arrays. This is the risky part of the import path (field-order in the file is irrelevant).
    #[test]
    fn return_inputs_toml_parses() {
        let text = r#"
            filing_status = "Mfj"

            [[w2s]]
            owner = "taxpayer"
            employer = "ACME"
            box1_wages = "82000"
            box2_fed_withheld = "9100"
            box5_medicare_wages = "82000"

            [[div_1099]]
            payer = "Vanguard"
            box1a_ordinary = "3400"
            box1b_qualified = "3100"

            [[form_1098]]
            lender = "Home Savings"
            box1_interest = "11200"
            other_borrower_paid_interest = false

            [schedule_a]
            salt_real_estate = "6800"

            [[schedule_a.charitable]]
            class = "cash60"
            amount = "2500"

            [payments]
            estimated_tax_payments = "6000"
        "#;
        let ri = parse_return_inputs_toml(text).unwrap();
        assert_eq!(ri.filing_status, FilingStatus::Mfj);
        assert_eq!(ri.w2s.len(), 1);
        assert_eq!(ri.w2s[0].box1_wages, dec!(82000));
        assert_eq!(ri.w2s[0].box5_medicare_wages, dec!(82000));
        assert_eq!(ri.div_1099[0].box1b_qualified, dec!(3100));
        // ★ T9 — the mortgage interest is a DOCUMENT now, not a Schedule A scalar.
        assert_eq!(ri.form_1098.len(), 1);
        assert_eq!(ri.form_1098[0].box1_interest, dec!(11200));
        let a = ri.schedule_a.as_ref().unwrap();
        assert_eq!(a.charitable[0].class, CharitableClass::Cash60);
        assert_eq!(a.charitable[0].amount, dec!(2500));
        assert_eq!(ri.payments.estimated_tax_payments, dec!(6000));
    }

    /// ★★★ **R4 / R5 / T5 — `income import` ACCEPTS THE NEW TABLES, AND NAMES A MISSPELT KEY IN
    ///     THEM.**
    ///
    /// Both halves, because either alone is satisfiable by the wrong thing: a parser that accepted
    /// everything would pass the first, and one that accepted nothing would pass the second.
    ///
    /// ★ The unknown-key half is the load-bearing one for a TRANSCRIPTION surface: a filer copying
    ///   `box11_bond_premum` off the paper must be told, because a silently-ignored key would drop a
    ///   figure that REFUSES the return, and they would file a smaller tax believing btctax had seen
    ///   it. `serde_ignored` derives the key set from the type, so this needs no list.
    #[test]
    fn income_import_takes_the_new_document_tables_and_names_a_misspelt_key_in_them() {
        let text = r#"
            filing_status = "Single"
            w2_wages_without_w2 = false
            interest_or_dividends_without_1099 = true
            state_refund_without_1099g = false
            itemized_prior_year = false

            [[int_1099]]
            payer = "First Bank"
            payer_tin = "11-1111111"
            transcribed_on = [2026, 34]
            box1_interest = "2000"
            box10_market_discount = "150"


            [[g_1099]]
            payer = "State of Example"
            box1_unemployment = "0"
            box2_state_refund = "900"
            box10_family_leave_benefits = "0"

            [[form_1098e]]
            lender = "Example Servicing"
            lender_tin = "99-9999999"
            box1_interest = "600"

            [[schedule_b_filer_records]]
            payer_name = "A neighbour"
            payer_ssn = "000-00-0001"
            payer_address = "1 Example St"
            amount = "1200"
            kind = "interest"

            [[w2s]]
            owner = "taxpayer"
            employer = "ACME"
            box1_wages = "50000"
            box2_fed_withheld = "5000"
            box13_statutory_employee = false
            box14b_treasury_tipped_occupation_codes = "102"
        "#;
        let ri = parse_return_inputs_toml(text).unwrap();
        assert_eq!(ri.int_1099[0].box10_market_discount, dec!(150));
        assert_eq!(
            ri.int_1099[0].transcribed_on,
            Some(time::macros::date!(2026 - 02 - 03))
        );
        assert_eq!(ri.g_1099[0].box2_state_refund, dec!(900));
        assert_eq!(ri.form_1098e[0].box1_interest, dec!(600));
        assert_eq!(ri.form_1098e[0].lender, "Example Servicing");
        assert_eq!(ri.schedule_b_filer_records[0].amount, dec!(1200));
        assert_eq!(
            ri.schedule_b_filer_records[0].kind,
            btctax_core::tax::return_inputs::ScheduleBRecordKind::Interest
        );
        assert_eq!(ri.w2s[0].box14b_treasury_tipped_occupation_codes, "102");
        assert!(!ri.w2s[0].box13_statutory_employee);
        assert_eq!(ri.itemized_prior_year, Some(false));
        assert_eq!(ri.interest_or_dividends_without_1099, Some(true));

        // ★ THE OTHER HALF: a typo in the very box whose non-zero value REFUSES.
        let bad = r#"
            filing_status = "Single"

            [[int_1099]]
            payer = "First Bank"
            box1_interest = "2000"
            box11_bond_premum = "40"
        "#;
        let err = parse_return_inputs_toml(bad).unwrap_err().to_string();
        assert!(
            err.contains("int_1099.0.box11_bond_premum"),
            "the refusal must name the exact path, or a dropped bond premium files a smaller tax \
             than the paper supports: {err}"
        );
    }

    /// `income show` redacts SSNs and the IP-PIN in a DISPLAY copy; the stored value is untouched (I5).
    #[test]
    fn mask_ssn_and_pii_redacts() {
        assert_eq!(mask_ssn("123-45-6789"), "***-**-6789");
        assert_eq!(mask_ssn("123456789"), "***-**-6789");
        assert_eq!(mask_ssn(""), "");
        assert_eq!(mask_ssn("12"), "***-**-****");
        let mut ri = ReturnInputs::default();
        ri.header.taxpayer.ssn = "123-45-6789".into();
        ri.header.ip_pin = Some("999999".into());
        ri.header.spouse = Some(btctax_core::tax::return_inputs::Person {
            ssn: "987-65-4321".into(),
            ..Default::default()
        });
        ri.header.dependents = vec![btctax_core::tax::return_inputs::Dependent {
            ssn: "111-22-3333".into(),
            ..Default::default()
        }];
        let masked = mask_pii(&ri);
        assert_eq!(masked.header.taxpayer.ssn, "***-**-6789");
        assert_eq!(masked.header.spouse.as_ref().unwrap().ssn, "***-**-4321");
        assert_eq!(masked.header.dependents[0].ssn, "***-**-3333");
        assert_eq!(masked.header.ip_pin.as_deref(), Some("***"));
        assert_eq!(ri.header.taxpayer.ssn, "123-45-6789"); // original untouched
        assert_eq!(ri.header.spouse.as_ref().unwrap().ssn, "987-65-4321"); // original untouched
    }

    /// Malformed TOML is a typed `Usage` error, never a panic.
    /// ★ spec 1099-DA T1/T6 — the TOML shape of the Form 1099-DA answers (a transparent newtype, so
    /// the path is `[broker_reporting.<provider>]`), and a misspelt slot is the unknown-key refusal
    /// naming the exact path — never a silently dropped answer.
    #[test]
    fn broker_reporting_toml_shape_parses_and_a_misspelt_slot_is_named() {
        use btctax_core::forms::{BrokerReported, Cohort};
        let text = r#"
            filing_status = "Single"

            [broker_reporting.coinbase]
            covered = "basis_matches"

            [broker_reporting.gemini]
            noncovered = "proceeds_only"
        "#;
        let ri = parse_return_inputs_toml(text).unwrap();
        assert_eq!(
            ri.broker_reporting.answer("coinbase", Cohort::Covered),
            Some(BrokerReported::BasisMatches)
        );
        assert_eq!(
            ri.broker_reporting.answer("coinbase", Cohort::Noncovered),
            None
        );
        assert_eq!(
            ri.broker_reporting.answer("gemini", Cohort::Noncovered),
            Some(BrokerReported::ProceedsOnly)
        );
        let bad = r#"
            filing_status = "Single"

            [broker_reporting.coinbase]
            covred = "basis_matches"
        "#;
        let err = parse_return_inputs_toml(bad).unwrap_err().to_string();
        assert!(
            err.contains("broker_reporting.coinbase.covred"),
            "the unknown-key refusal must name the exact path: {err}"
        );
        let unknown_answer = r#"
            filing_status = "Single"

            [broker_reporting.coinbase]
            covered = "reported"
        "#;
        assert!(
            parse_return_inputs_toml(unknown_answer).is_err(),
            "an unknown answer word is refused"
        );
    }

    #[test]
    fn bad_toml_is_typed_error() {
        assert!(matches!(
            parse_return_inputs_toml("not = = toml").unwrap_err(),
            CliError::Usage(_)
        ));
    }

    /// ★ P9 §2.3 / §3.5 (r7 I-2) — `income import` REJECTS unknown TOML keys via `serde_ignored`, not a
    /// hand-written key list. A TOML carrying `hsa_present` (the §2.4 rename) AND `box13_retirement_plan`
    /// (a deleted dead field — a real W-2 box 13 faithfully transcribed) must REFUSE naming BOTH, rather
    /// than import clean and silently vanish (the exact hole §2.3 exists to close). Mutation: revert to a
    /// bare `toml::from_str` ⇒ this fails.
    #[test]
    fn income_import_rejects_unknown_toml_keys_naming_each() {
        let text = r#"
            filing_status = "Single"
            hsa_present = false

            [[w2s]]
            owner = "taxpayer"
            employer = "ACME"
            box1_wages = "50000"
            box2_fed_withheld = "8000"
            box13_retirement_plan = true
        "#;
        let err = parse_return_inputs_toml(text).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("hsa_present"),
            "must name the renamed key: {msg}"
        );
        assert!(
            msg.contains("box13_retirement_plan"),
            "must name the deleted dead field so a transcribed W-2 box 13 can't silently vanish: {msg}"
        );
    }
}
