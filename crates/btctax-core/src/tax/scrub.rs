//! Replace a return's IDENTITY with synthetic stand-ins, preserving every figure and every field the
//! computation reads — so a real return can be handed to someone else to reproduce a defect.
//!
//! ## The guarantee, and the test that holds it
//!
//! **Scrubbing must not change a single computed number.** That is the whole value of the thing: a
//! scrubbed return that files differently from the original is worse than useless, because it sends
//! the recipient after a bug that is not there. The invariant is asserted directly —
//! `assemble_absolute(original) == assemble_absolute(scrubbed)`, over the whole `AbsoluteReturn`, not
//! a sampled figure — in `scrub_preserves_every_computed_figure`.
//!
//! ## ★★★ WHY THIS DESTRUCTURES EXHAUSTIVELY
//!
//! Every struct touched here is taken apart with **no `..`** — all TEN of them. A scrubber that
//! silently passes a NEW PII field through is worse than no scrubber at all: it carries the user's
//! authorisation ("this file is safe to share") onto a file that is not. So adding a field to
//! `ReturnInputs`, `Person`, `Dependent`, `HouseholdHeader`, `W2`, `ScheduleCInputs`, `Form1099Int`,
//! `Form1099Div`, `Form1099G` or `Form1099B` is *pattern does not mention field* here, and the author
//! must decide which side it falls on. The same argument the P9 classifier makes about answered-ness,
//! applied to disclosure.
//!
//! ★ **This sentence was an OVERCLAIM until 2026-08-09 and is the reason to distrust prose here.** It
//! said "every struct touched here", and it was true of four: the six it missed — the W-2 and the four
//! 1099s and Schedule C — are precisely the ones holding the free text a filer types in their own
//! words. A doc comment asserting a property nothing enforced, in the module whose whole subject is
//! that distinction.
//!
//! ★★★ THE TOP LEVEL WAS THE ONE THAT MATTERED, AND IT WAS THE ONE THAT WAS MISSING. The first
//! version of this module made the sentence above true of the three nested structs and false of
//! `ReturnInputs`, which was a `clone()` plus targeted mutation — so `business_description` (free
//! text, where a filer writes their business NAME) and `foreign_country_names` (which countries this
//! person banks in) rode straight through into a file the tool called shareable. New input types are
//! added at the TOP level, so that is precisely where the compile-time guard was worth having. Caught
//! by a security review, not by the suite.
//!
//! ## What is REPLACED, and what must survive
//!
//! | replaced — read by nothing but the printer | preserved — the computation reads it |
//! |---|---|
//! | names, SSNs, occupation, street/city/state/ZIP, IP PIN | filing status, every dollar figure |
//! | employer / payer / bank names | dates of birth and death, blindness |
//! | Schedule C `business_description` (free text) | `can_be_claimed_*`, the spouse flags, presidential-fund boxes |
//! | foreign account COUNTRY NAMES (count kept) | the NUMBER of dependents and each `relationship` |
//! | | Schedule C `naics_code` — a federal industry code, not a person |
//!
//! ★★ **EINs are replaced but keep their SAMENESS.** §6413(c)'s excess-social-security credit turns on
//! having *more than one employer*, decided by comparing canonicalised W-2 EINs
//! (`return_1040.rs`) — a real understatement bug in v0.15.0 was exactly a comparison that got this
//! wrong. So each DISTINCT real EIN maps to a distinct synthetic one, and two W-2s that shared an
//! employer still share one afterwards. Replacing them independently would silently turn one employer
//! into two and manufacture a credit; replacing them all with one constant would do the reverse.
//!
//! ★★ **SSNs — AND ONLY SSNs — ARE SAFE BY CONSTRUCTION.** [`synthetic_ssn`] emits a middle group of
//! `00`, which the SSA has never issued, and `scripts/pii-scan-generic.sh` has a matching STRUCTURAL
//! rule (`ALLOWED_SSN_IMPOSSIBLE`) that admits the whole shape.
//!
//! ★★★ **The same is NOT true of EINs, and the claim used to be written as though it were.** There is
//! no impossible EIN — the IRS has issued prefixes across nearly the whole 2-digit space — so the
//! scanner has no structural rule for them and says so in its own comments. A generator-keyed
//! allowlist would be `^9[0-9]-[0-9]{7}$`, which exempts every REAL EIN issued under 91/94/95/99 from
//! a scan whose entire purpose is stopping a real taxpayer identifier reaching a public repo.
//! Widening an exemption is never the safe edit, so (SPEC_income_scrub.md §7): **committing a scrubbed
//! return as a repo fixture is OUT OF SCOPE for v1.** If it is ever wanted it is its own decision, and
//! it must state the real-EIN intersection of whatever window it proposes in writing first.

use crate::state::LedgerState;
use crate::tax::return_inputs::{Dependent, HouseholdHeader, Person, ReturnInputs};
use std::collections::BTreeMap;

/// **THE OBLIGATION.** Scrub may emit only when the ledger contributes **nothing** to any figure,
/// refusal, gate, watermark or advisory btctax produces for `year`. This function owns that
/// sentence; `true` means the year is NOT scrubbable and the command must refuse.
///
/// ★★★ IT IS SCRUB-OWNED, AND IT IS **NOT** THE 1040 DIGITAL-ASSET BOX. The first draft reused
/// [`crate::tax::return_1040::digital_asset_activity`] alone, arguing the refusal was "exactly as
/// accurate as a box btctax already prints". Both halves of that were wrong, and a spec review
/// killed it (SPEC_income_scrub.md §2.2):
///
/// 1. **Year-scope is not artifact-scope.** [`LedgerState::pseudo_active`] and the Hard-blocker gate
///    are **projection-wide and year-blind**. A vault
///    whose only activity is an unclassified inbound has no in-year digital-asset activity at all,
///    yet the FILER's export is DRAFT-watermarked and attestation-gated while the recipient's copy
///    would be clean and ungated. A 2022 `ImportConflict` makes the filer's 2024 delta
///    `NotComputable` and the recipient's computable. **That is precisely the filer most likely to
///    run `income scrub`** — "btctax won't export my forms" — handed the one file on which the
///    problem vanishes.
/// 2. **It leaned on a don't-know.** `digital_asset_activity`'s own doc says `false` leaves the box
///    **unchecked, NOT answered "No"**. Reading that deliberate silence as "the ledger is empty" is
///    widening an exemption, which is never the safe edit.
///
/// Every disjunct names the mechanism that put it there:
///
/// - `digital_asset_activity(state, year)` — disposals / income / removals dated in-year.
/// - `digital_asset_activity(state, year - 1)` — ★ **DELIBERATE OVER-REFUSAL that secures no live
///   read.** Two rounds of naming a mechanism for it were both wrong: it is *not* the M4
///   carryforward-consistency advisory (which needs the prior year's stored *profile*, so it does
///   not reproduce from a one-year file whatever this decides), and it is *not* "the prior year's
///   ledger feeds this year's carryforward-in" ([`scrub_pii`] preserves
///   `capital_loss_carryforward_in` and its provenance verbatim, so those figures travel in the
///   TOML and reproduce with no ledger at all). It is kept because scrub declines to prove the
///   negative that no path reads the prior year's ledger live. The error direction is over-refusal,
///   which is the safe one. **Do not "fix" it by inventing a third mechanism.**
/// - [`LedgerState::pseudo_active`] — the DRAFT watermark and the attestation gate.
/// - An unresolved `Severity::Hard` blocker — `NotComputable`, projection-wide.
///
/// ★★★ **THE HARD-BLOCKER DISJUNCT DOES NOT CALL `compute::first_hard_blocker`, AND THAT IS A
/// CORRECTION TO THE SPEC.** SPEC_income_scrub.md §2.2 directed widening that helper to
/// `pub(crate)`. It cannot be: `tax/compute.rs` is a **frozen engine file**, content-pinned by
/// [`crate::tax::frozen_guard`] (SPEC_full_return §2), and even an edit preserving the public API
/// trips `frozen_engine_files_are_unchanged` — which is exactly what the pin is for. Its exception
/// process demands a separately-reviewed commit, and "a sharing feature wanted one more caller" is
/// not the exceedingly-rare case that process exists for.
///
/// So this uses the predicate every other non-frozen caller already uses —
/// `state.blockers.iter().any(|b| b.kind.severity() == Severity::Hard)`, as at `admin.rs:234`,
/// `:712` and `:972`. `first_hard_blocker` has no caller outside `compute.rs`; it returns the FIRST
/// such blocker for a deterministic `TaxYearNotComputable` detail, and this only needs existence.
/// ★ Five spec-review rounds mandated an edit to a frozen file and none of them noticed. It was a
/// `cargo nextest` away, and the build found it in one run.
///
/// ★ The caller must also **project the ledger** before asking, and **refuse if projection fails**
/// — never fall back to "assume empty".
#[must_use]
pub fn ledger_contributes(state: &LedgerState, year: i32) -> bool {
    ledger_contribution(state, year).is_some()
}

/// Which disjunct of [`ledger_contributes`] fires, so a refusal can name its own mechanism.
///
/// ★★ The CAUSE is the primitive and the bool is derived from it — `ledger_contributes` is literally
/// `self.is_some()`. The alternative (a `bool` here plus a separate cause-finder for the message, or
/// the CLI re-deriving the cause from public state) would be **two sources of truth for one
/// obligation**, and they would drift on the first new disjunct. The refusal message is not
/// decoration: a filer who cannot see which mechanism stopped them cannot act on it.
///
/// Order is the disjunction's own and matters only to the message, never to the verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerContribution {
    /// Disposals / recognized income / removals dated in the scrub year itself.
    InYearActivity,
    /// The same, in `year - 1`. ★ Deliberate over-refusal — see [`ledger_contributes`]; this
    /// disjunct secures no live read and must not be re-justified with a fresh mechanism.
    PriorYearActivity,
    /// A synthetic default contributes to the projection: the filer's export is DRAFT-watermarked
    /// and attestation-gated, and the recipient's copy would be neither. Projection-wide.
    PseudoActive,
    /// An unresolved `Severity::Hard` blocker anywhere in the projection makes the filer's year
    /// `NotComputable` while the recipient's computes. Projection-wide and year-blind.
    HardBlocker,
}

impl LedgerContribution {
    /// A short phrase naming the mechanism, for the refusal message.
    #[must_use]
    pub fn cause(self) -> &'static str {
        match self {
            Self::InYearActivity => "the ledger records digital-asset activity in that year",
            Self::PriorYearActivity => {
                "the ledger records digital-asset activity in the PRIOR year, which this command \
                 refuses on deliberately rather than prove that nothing reads it"
            }
            Self::PseudoActive => {
                "the ledger is pseudo-reconciled — a synthetic default contributes to the \
                 projection, so your own export carries a DRAFT watermark and an attestation gate \
                 that a scrubbed copy would not reproduce"
            }
            Self::HardBlocker => {
                "the projection carries an unresolved hard blocker, so your year is not computable \
                 while a scrubbed copy of these inputs would compute cleanly"
            }
        }
    }
}

/// See [`LedgerContribution`]. Returns the first firing disjunct, or `None` when the year is
/// scrubbable.
#[must_use]
pub fn ledger_contribution(state: &LedgerState, year: i32) -> Option<LedgerContribution> {
    use crate::tax::return_1040::digital_asset_activity;

    if digital_asset_activity(state, year) {
        Some(LedgerContribution::InYearActivity)
    } else if digital_asset_activity(state, year - 1) {
        Some(LedgerContribution::PriorYearActivity)
    } else if state.pseudo_active() {
        Some(LedgerContribution::PseudoActive)
    } else if state
        .blockers
        .iter()
        .any(|b| b.kind.severity() == crate::state::Severity::Hard)
    {
        Some(LedgerContribution::HardBlocker)
    } else {
        None
    }
}

/// ★★★ **THE PROVENANCE MARKER** (§4.2). A leading COMMENT LINE on the emitted raw text — never a
/// TOML key, because `parse_return_inputs_toml` rejects unknown keys *before* any guard could run.
///
/// **Why it is not cosmetic.** A scrubbed file is schema-identical to a real one and `income import`
/// is an unconfirmed whole-blob upsert, so re-importing one over a real vault destroys that vault's
/// identity and IP PIN — **unrestorable** — and leaves a synthetic SSN well-formed enough to print on
/// a filed 1040. This repo's rubric grades data loss Critical.
///
/// ★ The token is deliberately long and unpunctuated so it cannot arise by accident in a real file,
/// and it is matched as a SUBSTRING of the file text, so a filer who adds their own comments above it
/// does not defeat the guard.
pub const SCRUB_PROVENANCE_MARKER: &str =
    "# btctax-scrubbed-return: identity replaced, figures intact. NOT your real return.";

/// ★★★ **THE SCRUB CONSTANTS, MOVED RATHER THAN THE FIXTURES** (§3.4).
///
/// The stand-in address used to be `Springfield / IL / 62704` — **the same address the repo's own
/// fixtures carry** (`testonly.rs:260-262`, `:419-421`). A disclosure test asserting "the address
/// changed" was therefore VACUOUS on every existing household, and the natural fix — editing the
/// fixtures — reds three committed byte-pinned artifacts.
///
/// Moving the SCRUB constants instead is one file, zero golden churn, and it immunises **every future
/// fixture** rather than the three that exist. ★ The values are deliberately not plausible: a
/// recipient must never mistake a scrubbed address for a real one, so `XX` is not a state and `00000`
/// is not a ZIP.
pub(crate) const SCRUB_STREET: &str = "1 Example St";
pub(crate) const SCRUB_CITY: &str = "Exampleton";
pub(crate) const SCRUB_STATE: &str = "XX";
pub(crate) const SCRUB_ZIP: &str = "00000";

/// A synthetic SSN that the SSA can never have issued (middle group `00`), distinct per `n`.
fn synthetic_ssn(n: usize) -> String {
    format!("1{:02}-00-{:04}", n % 100, (n % 9999) + 1)
}

/// A synthetic EIN, distinct per `n`. There is no "impossible EIN" — the IRS has issued prefixes
/// across nearly the whole 2-digit space — so this is merely synthetic, not provably unissued.
fn synthetic_ein(n: usize) -> String {
    format!("9{}-{:07}", n % 10, n + 1)
}

/// A synthetic EIN that **`canonical_ein` REJECTS** — eight digits, one short — and distinct per `n`
/// for every `n` a return can reach.
///
/// ★ Distinctness is PERIODIC, not total: the `% 1_000_000` below (which is what makes the class
/// guarantee hold for every `usize`) gives period 10^6, so `f(0) == f(1_000_000)`. `EinMap` passes
/// `n = self.0.len()`, so reaching it needs a million DISTINCT EINs on one return. It is also benign
/// if reached: merging two MALFORMED stand-ins cannot move a §6413(c) figure, because both stay
/// unreadable and `over_cap_needs_ein` still refuses. Stated rather than left to be rediscovered —
/// the previous version of this line said "distinct per `n`" flatly, which stopped being true when
/// the modulo was added.
///
/// ★★★ THE VALIDITY CLASS OF AN EIN IS LOAD-BEARING, AND MISSING THIS SHIPPED r1's CRITICAL TWICE.
/// `screen_inputs` refuses `ExcessSsEmployerUnknown` when a filer is over the §3101(a) cap and any of
/// their W-2s has an EIN `canonical_ein` cannot read (`return_refuse.rs`'s `over_cap_needs_ein`):
/// whether the over-withholding is creditable turns on "more than one employer", and a malformed EIN
/// makes that **undecidable**.
///
/// Upgrading it to a well-formed stand-in ANSWERS a question the filer's own data cannot answer. The
/// scrubbed copy then stops refusing and claims a §6413(c) credit — $1,546.80 on the vector in
/// `a_malformed_ein_stays_malformed_so_the_copy_refuses_where_the_original_did` — that the original
/// does not have. The filer is sending the file precisely to reproduce that refusal.
fn synthetic_malformed_ein(n: usize) -> String {
    // ★ `{:06}` PADS, it does not truncate, so at n = 999_999 this would emit nine digits and
    //   silently canonicalize — inverting the very class it exists to preserve. Unreachable (it needs
    //   a million distinct EINs on one return) but the assert makes it structural rather than
    //   incidental, which is the difference between a guarantee and a coincidence.
    let s = format!("9{}-{:06}", n % 10, (n + 1) % 1_000_000);
    debug_assert!(
        crate::tax::return_1040::canonical_ein(&s).is_none(),
        "a malformed-EIN stand-in that canonicalizes defeats its own purpose: {s}"
    );
    s
}

/// ★★★ **TRIM-EMPTINESS IS A VALIDITY CLASS, AND EVERY REPLACEMENT PRESERVES IT** (§3.2).
///
/// Replace `s` with `stand_in` — **unless `s` is trim-empty, in which case emit `""`.**
///
/// The class every screen induces is `trim().is_empty()`, **not** `is_empty()`: `return_refuse.rs`
/// reads it at `:672` (broker payer), `:798` (country names) and `:901` (business description), and
/// the repo pins that exact substitution at `:1484` — *"Whitespace is not a name. This pins the
/// `trim()`, which a naive `is_empty()` would miss."* A rule keyed to `""` alone leaves `"   "` in
/// the replaced set and the class still flips.
///
/// **Why it matters most on `business_description`:** it is `#[serde(default)]`, so an imported TOML
/// that omits the key yields `""`, and `screen_inputs` then refuses `ScheduleCNoBusinessDescription`.
/// A filer whose description is blank hits that refusal, runs `income scrub` **because of it**, and
/// under an unconditional replacement is handed a file on which the refusal does not exist. That is
/// §3.2's governing harm — the scrubbed copy files where the original refused.
///
/// ★ Emitting `""` for a trim-empty original leaks nothing: an empty or whitespace-only string
/// carries no identity, so this opens no disclosure channel in exchange for closing a class flip.
fn replace_preserving_emptiness(s: &str, stand_in: String) -> String {
    if s.trim().is_empty() {
        String::new()
    } else {
        stand_in
    }
}

/// Remaps EINs so that DISTINCTNESS is preserved and nothing else is.
///
/// ★★★ **KEYED ON `canonical_ein`, NOT ON THE RAW SPELLING — this is r1's CRITICAL.** §3.1 of
/// SPEC_income_scrub.md: where code compares identifiers, scrub must preserve the partition **that
/// comparison** induces, never string equality. §6413(c)'s excess-Social-Security credit turns on
/// having *more than one employer*, and `excess_social_security` decides that by comparing
/// **canonicalized** EINs — so `"11-1111111"` and `"111111111"` are ONE employer.
///
/// Keying the raw string split them into two synthetic EINs that canonicalize apart, manufacturing a
/// **$1,546.80** credit on the repo's own `two_spellings` vector, raising `total_payments` and the
/// refund by the same. An understatement on a §6065 return, inside a file the command called safe to
/// share. Held by `two_spellings_of_one_employer_survive_as_one_employer`.
///
/// ★ A value `canonical_ein` REJECTS (letters, wrong digit count) has no canonical form, so it falls
/// back to the raw string as its own key: two malformed EINs that differ as text stay distinct, and
/// nothing is merged on the strength of a comparison the program never makes.
///
/// ★★★ **AND ITS VALIDITY CLASS IS PRESERVED HERE, IN THIS MAP.** This sentence used to end
/// "preserving the malformed *class* into the output is §3.2's job, not this map's" — and §3.2's
/// implementation had an SSN leg, an IP PIN leg and nothing for the EIN, so the delegation pointed
/// at an empty room. A malformed EIN was upgraded to a well-formed synthetic, the scrubbed copy
/// stopped refusing `ExcessSsEmployerUnknown`, and it claimed a $1,546.80 §6413(c) credit the
/// original does not have. **A pointer to another section is not an implementation**, and a whole-
/// branch review is what finally noticed.
#[derive(Default)]
struct EinMap(BTreeMap<String, String>);

impl EinMap {
    fn map(&mut self, real: &str) -> String {
        let canonical = crate::tax::return_1040::canonical_ein(real);
        // ★★★ THE VALIDITY CLASS SURVIVES TOO (§3.2), not just the partition (§3.1). A value
        //     `canonical_ein` rejects keys on its own raw spelling — two malformed EINs that differ
        //     as text stay distinct — AND maps to a stand-in that is *also* unreadable, so the
        //     undecidability the screen depends on is preserved rather than resolved.
        //
        //     ★ This is the leg the build originally omitted. `EinMap`'s doc used to forward the
        //       obligation to §3.2, and §3.2's implementation had an SSN leg and an IP PIN leg and
        //       nothing for the EIN — so the delegation pointed at an empty room. It is handled here
        //       now, where the mapping happens, rather than delegated anywhere.
        let key = canonical.clone().unwrap_or_else(|| real.to_string());
        let next = self.0.len();
        self.0
            .entry(key)
            .or_insert_with(|| {
                if canonical.is_some() {
                    synthetic_ein(next)
                } else {
                    synthetic_malformed_ein(next)
                }
            })
            .clone()
    }
}

/// ★★★ **THE VALIDITY CLASS SURVIVES, AND NO ORIGINAL BYTE DOES** (§3.2).
///
/// `absent → absent`, `malformed → SYNTHETIC-malformed`, `valid → synthetic-valid`. The preserved
/// thing is the **error variant**, never the bytes, because the whole point is that the recipient's
/// copy refuses exactly where the filer's did — the filer is sending the file *to reproduce a
/// refusal*. Normalizing every class into "valid" makes identity-shaped fail-closed screens vanish
/// and the scrubbed copy **files where the original refused**.
///
/// ★★ **`NotDigits` is the one deliberate COARSENING.** `SsnError::NotDigits(c)` carries a character
/// of the filer's REAL entry (`packet.rs:66-67`), the only `SsnError` payload that is *content*
/// rather than *shape* — `WrongLength(n)` is a digit count, which is what makes it both safe to
/// preserve and diagnostically load-bearing ("my SSN has four digits"). So a malformed original maps
/// to a stand-in whose first non-digit is a constant `'x'`, and `Ssn::canonical` checks `NotDigits`
/// **before** length, so the variant holds at any length and `'x'` cannot collapse into `Ok`.
fn class_preserving_stand_in(
    class: Result<(), crate::tax::packet::SsnError>,
    valid: String,
) -> String {
    use crate::tax::packet::SsnError;
    match class {
        Ok(()) => valid,
        // `absent → absent`: nothing was given, and inventing a value would file where the original
        // refused.
        Err(SsnError::Missing) => String::new(),
        // The payload is a real character of the filer's entry. Coarsen it; keep the variant.
        Err(SsnError::NotDigits(_)) => "x".to_string(),
        // A digit count is a shape, not content — preserve it exactly.
        Err(SsnError::WrongLength(n)) => "1".repeat(n),
    }
}

/// The SSN leg of [`class_preserving_stand_in`].
fn synthetic_ssn_like(real: &str, valid: String) -> String {
    class_preserving_stand_in(crate::tax::packet::Ssn::canonical(real).map(|_| ()), valid)
}

/// The IP PIN leg, which is §3.2's one deliberate exception and has **three** outcomes, not two.
///
/// ★ **It MUST validate with `IpPin::canonical`, not `Ssn::canonical`** — an IP PIN is SIX digits and
/// an SSN is nine, so the SSN validator reads a perfectly valid PIN as `WrongLength(6)` and mints a
/// stand-in for it. That inverts the carve-out precisely: the *valid* case, which must be DROPPED,
/// would instead be replaced. Caught by `the_identity_does_not_survive`.
///
/// ★★ **`Some("")` IS NOT `None`.** `ReturnHeader::build` canonicalizes a present PIN, so an empty
/// one errs `Missing` while an absent one is `Ok`. Collapsing `Some("")` to `None` therefore changes
/// the packet-boundary class — the scrubbed copy builds a header where the filer's refused. It is
/// preserved as `Some("")`, which leaks nothing and still errs `Missing`.
fn scrub_ip_pin(real: &str) -> Option<String> {
    use crate::tax::packet::{IpPin, SsnError};
    match IpPin::canonical(real) {
        // ★★★ VALID → DROPPED, never synthesised. A well-formed stand-in inside a file stamped
        //     shareable would FABRICATE a live IRS anti-fraud credential. Nothing computes from it,
        //     and `build` is `Ok` for both `Some(valid)` and `None`, so dropping moves no figure and
        //     changes no class.
        Ok(_) => None,
        // Present but empty: still errs `Missing`, so the stand-in must too.
        Err(SsnError::Missing) => Some(String::new()),
        // A character of the filer's real entry — coarsen it, keep the variant (§3.2).
        Err(SsnError::NotDigits(_)) => Some("x".to_string()),
        // n can never be 6 here (that is the `Ok` arm), so this cannot canonicalize into a
        // well-formed credential.
        Err(SsnError::WrongLength(n)) => Some("1".repeat(n)),
    }
}

fn scrub_person(p: &Person, who: &str, n: usize) -> Person {
    // ★ No `..` — a new `Person` field must be classified here before this compiles.
    let Person {
        first_name,
        last_name,
        ssn,
        date_of_birth,
        date_of_death,
        blind,
        occupation,
    } = p;
    Person {
        first_name: replace_preserving_emptiness(first_name, who.to_string()),
        last_name: replace_preserving_emptiness(last_name, "Example".into()),
        ssn: synthetic_ssn_like(ssn, synthetic_ssn(n)),
        // ★★ KEPT. §63(f)'s age-65 and blindness additions read these, and i1040gi's carve-out for
        //    someone who died in-year before reaching 65 reads the death date. Scrubbing them would
        //    move the deduction and break the invariant this module exists to guarantee.
        date_of_birth: *date_of_birth,
        date_of_death: *date_of_death,
        blind: *blind,
        occupation: replace_preserving_emptiness(occupation, "Occupation".into()),
    }
}

fn scrub_dependent(d: &Dependent, n: usize) -> Dependent {
    let Dependent {
        name,
        ssn,
        relationship,
        // ★ Bound and discarded rather than omitted: the `..`-free destructure is the guard, so this
        //   must stay named even though §6 drops the value. Removing it would silently re-open the
        //   hole for the NEXT field added to `Dependent`.
        date_of_birth: _,
    } = d;
    Dependent {
        name: replace_preserving_emptiness(name, format!("Dependent{n}")),
        // ★ A dependent's SSN is read by `Ssn::canonical` too (`packet.rs:425`), so its validity class
        //   is as load-bearing as the taxpayer's: an eight-digit typo refuses on the original, and an
        //   unconditional stand-in would let the scrubbed copy EXPORT where the filer could not.
        ssn: synthetic_ssn_like(ssn, synthetic_ssn(100 + n)),
        // ★ KEPT: `relationship` decides child-vs-other-dependent, and it IS read.
        relationship: relationship.clone(),
        // ★★★ DROPPED (§6) — and the comment this replaces was FALSE. It said "the DOB decides
        //     qualifying-child age. BOTH are read", which is not true of a DEPENDENT's DOB: btctax
        //     does not compute the CTC, and every non-test reader of `date_of_birth` is the
        //     taxpayer's or the spouse's (`questions.rs:804,820`; `packet.rs:306,314`).
        //     `DependentRow` carries `name`/`ssn`/`relationship` only (`packet.rs:418-428`).
        //
        //     So retaining it is RETENTION, not computation — a child's date of birth kept in a file
        //     stamped shareable for no reason anything reads. Dropping it cannot move a figure,
        //     which is exactly why it must be dropped rather than quantized: quantizing would be
        //     inventing a value to stand in for one nothing consumes.
        date_of_birth: None,
    }
}

fn scrub_header(h: &HouseholdHeader) -> HouseholdHeader {
    let HouseholdHeader {
        taxpayer,
        spouse,
        address_street,
        address_city,
        address_state,
        address_zip,
        dependents,
        can_be_claimed_as_dependent_taxpayer,
        can_be_claimed_as_dependent_spouse,
        spouse_had_no_income,
        spouse_not_filing_a_return,
        presidential_fund_taxpayer,
        presidential_fund_spouse,
        taxpayer_died_during_year,
        spouse_died_during_year,
        ip_pin,
        form8615_condition3_age_support,
        form8615_condition4_parent_alive,
        form8615_parent_identity_unobtainable,
    } = h;
    HouseholdHeader {
        taxpayer: scrub_person(taxpayer, "Taxpayer", 1),
        spouse: spouse.as_ref().map(|s| scrub_person(s, "Spouse", 2)),
        address_street: replace_preserving_emptiness(address_street, SCRUB_STREET.into()),
        address_city: replace_preserving_emptiness(address_city, SCRUB_CITY.into()),
        // ★ The STATE is replaced too. btctax computes no state tax, so nothing reads it — but a
        //   state plus a filing status plus an income is a long way toward identifying a household.
        address_state: replace_preserving_emptiness(address_state, SCRUB_STATE.into()),
        address_zip: replace_preserving_emptiness(address_zip, SCRUB_ZIP.into()),
        dependents: dependents
            .iter()
            .enumerate()
            .map(|(i, d)| scrub_dependent(d, i + 1))
            .collect(),
        // ★★ All KEPT: every one of these is a fail-loud declaration that moves the return.
        can_be_claimed_as_dependent_taxpayer: *can_be_claimed_as_dependent_taxpayer,
        can_be_claimed_as_dependent_spouse: *can_be_claimed_as_dependent_spouse,
        spouse_had_no_income: *spouse_had_no_income,
        spouse_not_filing_a_return: *spouse_not_filing_a_return,
        presidential_fund_taxpayer: *presidential_fund_taxpayer,
        presidential_fund_spouse: *presidential_fund_spouse,
        taxpayer_died_during_year: *taxpayer_died_during_year,
        spouse_died_during_year: *spouse_died_during_year,
        // ★★★ THE CARVE-OUT (§3.2). The IP PIN is a live IRS anti-fraud CREDENTIAL, so minting a
        //     well-formed one inside a file stamped shareable would fabricate a credential — the one
        //     thing worse than leaking the real value. So it is NEVER synthesised:
        //
        //       absent or valid → None   (keep dropping it; nothing computes from it, so no figure
        //                                 moves, and neither §3.3 test can see the drop)
        //       malformed       → a malformed NON-credential stand-in
        //
        //     ★ The malformed leg is not decoration: it is the only leg `ReturnHeader::build` reads.
        //       `IpPin::canonical` errs only on malformed, so `absent`/`valid` are both invisible to
        //       its `Result` and collapsing them to `None` changes no behaviour — while a malformed
        //       original must still refuse on the recipient's copy, or the scrubbed file EXPORTS
        //       where the filer's could not. The stand-in preserves `WrongLength(n)` for n ≠ 6, so it
        //       can never canonicalize into a well-formed credential.
        //
        //     ★★ This is a DELIBERATE EXCEPTION to §3.2's blanket "malformed → SYNTHETIC-malformed,
        //        valid → synthetic-valid", and it is the only one.
        ip_pin: ip_pin.as_deref().and_then(scrub_ip_pin),
        // ★★ FR-29 — Form 8615's three leaves are KEPT, on the same ground as the declarations
        //    above: each is a fail-loud answer that moves the return (SPEC §6's ladder refuses on
        //    every `None`), and none is identifying. Condition 3 and condition 4 are facts about the
        //    filer's own age, support and parents' survival; the dead-end fact records what the
        //    filer can obtain. Dropping any of them would make a scrubbed file COMPUTE where the
        //    filer's own refuses — the direction §3.3 forbids.
        form8615_condition3_age_support: *form8615_condition3_age_support,
        form8615_condition4_parent_alive: *form8615_condition4_parent_alive,
        form8615_parent_identity_unobtainable: *form8615_parent_identity_unobtainable,
    }
}

/// Replace each name in a delimited free-text list, preserving how many there are.
///
/// ★ Schedule B line 7b prints the COUNTRIES a filer holds foreign accounts in — which is free text
/// and is identifying (it is a fact about a specific person's offshore banking). The count and the
/// delimiter shape are kept because line 7b prints them and a refusal fires when the list is empty
/// while `foreign_accounts` is `Some(true)`; the names themselves carry nothing a reproducer needs.
fn scrub_name_list(s: &str) -> String {
    if s.trim().is_empty() {
        return String::new();
    }
    s.split(',')
        .enumerate()
        .map(|(i, _)| format!("Country{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Scrub the identity out of `ri`, preserving every figure and every computation-bearing field.
///
/// See the module docs for the guarantee and for why EIN sameness is preserved.
#[must_use]
pub fn scrub_pii(ri: &ReturnInputs) -> ReturnInputs {
    // ★★★ THE COMPLETENESS WITNESS. No `..`, so a new `ReturnInputs` field is *pattern does not
    //     mention field* HERE and someone must decide whether it can carry identity before this
    //     compiles.
    //
    //     This was missing in the first version, and a background security review caught it. The
    //     module doc claimed "every struct touched here is taken apart with no `..`" — true of
    //     `Person`, `Dependent` and `HouseholdHeader`, and FALSE of the top level, which was a
    //     `clone()` plus targeted mutation. So `schedule_c.business_description` (free text: a real
    //     filer writes their business NAME there) and `foreign_country_names` (which countries this
    //     person banks in) both rode straight through into a file the tool told the user was safe to
    //     share. The guarantee is worthless exactly where new input types get added, which is the
    //     top level. Same lesson as `a-figure-with-no-reader`: the doc asserted a property nothing
    //     enforced.
    //
    //     `_` = carries no identity: a figure, a flag, a provenance tag, or an enum.
    let ReturnInputs {
        tax_year: _,
        filing_status: _,
        header,
        w2s: _,
        int_1099: _,
        div_1099: _,
        g_1099: _,
        b_1099: _,
        schedule_c,    // ★ business_description is FREE TEXT — scrubbed below
        schedule_a: _, // money only
        itemize_election: _,
        mfs_spouse_itemizes: _,
        sch1: _, // money only
        payments: _,
        capital_loss_carryforward_in: _,
        capital_loss_carryforward_in_provenance: _,
        carryover_includes_spouses_joint_loss: _,
        excluded_canceled_debt: _,
        amt_carryover_same_as_regular: _,
        amt_depreciation_same_as_regular: _,
        charitable_carryover_in: _, // money + year only; no donee identity here
        charitable_carryover_in_provenance: _,
        qbi: _, // money only
        foreign_accounts: _,
        foreign_trust: _,
        foreign_country_names, // ★ FREE TEXT and identifying — scrubbed below
        fbar_filing_required: _,
        donations_had_restrictions: _,
        charitable_cwa_obtained: _,
        filing_form_4952: _,
        form_8960_line9b: _, // a money amount — carries no identity
        dual_status_alien: _,
        has_income_exclusion: _,
        other_out_of_scope_income: _,
        excluded_puerto_rico_income: _,
        form_2555_line45: _,
        form_2555_line50: _,
        form_4563_line15: _,
    } = ri;

    let mut out = ri.clone();
    out.header = scrub_header(header);
    out.foreign_country_names = scrub_name_list(foreign_country_names);

    if let (Some(sc_out), Some(sc)) = (out.schedule_c.as_mut(), schedule_c.as_ref()) {
        // ★ The `..`-free guard, extended to the tenth struct (§7).
        let crate::tax::return_inputs::ScheduleCInputs {
            owner: _,
            business_description: _,
            naics_code: _,
            accounting_method: _,
            expenses: _,
            other_gross_receipts: _,
            payments_requiring_1099: _,
            will_file_required_1099: _,
            qbi_w2_wages: _,
            qbi_ubia: _,
            is_sstb: _,
            is_cooperative_patron: _,
        } = sc;
        // ★★ FREE TEXT, and the one field here a filer fills in their own words. "Smith Family
        //    Dental" is a perfectly ordinary answer.
        //
        //    ★★★ NON-EMPTY IN, STAND-IN OUT; TRIM-EMPTY IN, `""` OUT. The earlier comment here
        //        ordered it to "stay NON-EMPTY" unconditionally, which is sound only when the
        //        original was non-empty. `business_description` is `#[serde(default)]`, so an
        //        imported TOML that omits the key yields `""` and `screen_inputs` refuses
        //        `ScheduleCNoBusinessDescription` (`return_refuse.rs:897-905`; the three-space case
        //        is pinned at `:1487-1495`). A filer whose description is blank hits that refusal,
        //        runs `income scrub` BECAUSE of it, and an unconditional stand-in hands them a file
        //        on which the refusal does not exist — the scrubbed copy files where the original
        //        refused. Both directions are now preserved.
        sc_out.business_description =
            replace_preserving_emptiness(&sc.business_description, "Example business".into());
        // ★ `naics_code` is KEPT: a six-digit federal industry taxonomy shared by thousands of
        //   businesses is not a personal identifier, and it is printed on Schedule C line B, so a
        //   reproducer wants the real one. Recorded as a decision rather than an oversight.
    }

    // ★★ W-2 employers: the NAME is free to replace, the EIN is not — only its sameness matters.
    //
    // ★ Every loop below preserves trim-emptiness (§3.2). The broker one is not hypothetical:
    //   `screen_inputs` reads `b.payer.trim().is_empty()` to choose between "(unnamed broker)" and
    //   the payer's name (`return_refuse.rs:672-676`), so an unconditional stand-in flips what the
    //   recipient's copy says. The others are held to the same rule because the next reader of one
    //   of these fields is not required to announce itself.
    //
    // ★★★ EACH LOOP OPENS WITH AN EXHAUSTIVE `..`-FREE DESTRUCTURE (§7). The guard used to cover 4
    //     of 10 structs — `ReturnInputs`, `Person`, `Dependent`, `HouseholdHeader` — and the six it
    //     missed are exactly the ones holding free text a filer types: employer and payer names, and
    //     the business description. These destructures bind nothing and are compiled away; their
    //     entire job is to make *pattern does not mention field* fire when someone adds a field to
    //     one of these types, so the author must decide which side it falls on BEFORE it can ride
    //     into a file the tool calls shareable.
    let mut eins = EinMap::default();
    for (i, w) in out.w2s.iter_mut().enumerate() {
        let crate::tax::return_inputs::W2 {
            owner: _,
            employer: _,
            ein: _,
            box1_wages: _,
            box2_fed_withheld: _,
            box3_ss_wages: _,
            box4_ss_withheld: _,
            box5_medicare_wages: _,
            box6_medicare_withheld: _,
            box7_ss_tips: _,
            box17_state_tax_withheld: _,
            box19_local_tax: _,
            // ★ `box12[].code` is KEPT and this is the record of that decision (§7): a box-12 code is
            //   a TAXONOMY ("D" = 401(k) elective deferral), not a person. Replacing it would destroy
            //   a reproducer's fidelity for no disclosure gain.
            box12: _,
            box8_allocated_tips: _,
            box10_dependent_care: _,
        } = w;
        w.employer = replace_preserving_emptiness(&w.employer, format!("Employer{}", i + 1));
        if let Some(e) = w.ein.as_ref().filter(|e| !e.trim().is_empty()) {
            w.ein = Some(eins.map(e));
        }
    }
    for (i, f) in out.int_1099.iter_mut().enumerate() {
        let crate::tax::return_inputs::Form1099Int {
            payer: _,
            box1_interest: _,
            box2_early_withdrawal_penalty: _,
            box3_treasury_interest: _,
            box4_fed_withheld: _,
            box6_foreign_tax: _,
            box8_tax_exempt_interest: _,
            box9_private_activity_bond_amt: _,
        } = f;
        f.payer = replace_preserving_emptiness(&f.payer, format!("Payer{}", i + 1));
    }
    for (i, f) in out.div_1099.iter_mut().enumerate() {
        let crate::tax::return_inputs::Form1099Div {
            payer: _,
            box1a_ordinary: _,
            box1b_qualified: _,
            box2a_capgain_distr: _,
            box2b_unrecap_1250: _,
            box2c_section_1202: _,
            box2d_collectibles_28: _,
            box4_fed_withheld: _,
            box5_section_199a: _,
            box7_foreign_tax: _,
            box12_exempt_interest_dividends: _,
            box13_private_activity_amt: _,
        } = f;
        f.payer = replace_preserving_emptiness(&f.payer, format!("Payer{}", i + 1));
    }
    for (i, f) in out.b_1099.iter_mut().enumerate() {
        let crate::tax::return_inputs::Form1099B {
            payer: _,
            short_term_proceeds: _,
            short_term_basis: _,
            long_term_proceeds: _,
            long_term_basis: _,
            basis_reported_and_no_adjustments: _,
        } = f;
        f.payer = replace_preserving_emptiness(&f.payer, format!("Broker{}", i + 1));
    }
    for (i, f) in out.g_1099.iter_mut().enumerate() {
        let crate::tax::return_inputs::Form1099G {
            payer: _,
            box1_unemployment: _,
            box4_fed_withheld: _,
        } = f;
        f.payer = replace_preserving_emptiness(&f.payer, format!("Agency{}", i + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_1040::assemble_absolute;
    use crate::tax::testonly::{
        kitchen_sink_household, ty2024_params, ty2024_table, w2_only_household,
    };

    /// ★★★ r1's CRITICAL, PINNED: **one employer spelled two ways must stay ONE employer.**
    ///
    /// §6413(c)'s excess-social-security credit turns on having *more than one employer*, and
    /// `excess_social_security` decides identity by comparing **canonicalized** EINs
    /// (`return_1040.rs::canonical_ein`). A W-2 and its W-2c — box b typed off the paper form on one
    /// and off a payroll-portal export on the other — are `"11-1111111"` and `"111111111"`: the SAME
    /// employer, and the repo pins that at `return_1040.rs`'s `two_spellings` vector, where the
    /// correct credit is **$0**.
    ///
    /// The held scrubber keyed `EinMap` on the RAW STRING, so the two spellings became two distinct
    /// synthetic EINs, which canonicalize apart — **two employers** — and manufacture a
    /// **$1,546.80** credit the filer is not entitled to. `total_payments` rises by it and so does
    /// the refund. That is an UNDERSTATEMENT on a §6065 return, reached through a file the command
    /// told the filer was safe to share, and it is the exact failure the module's own EIN paragraph
    /// says it exists to prevent.
    ///
    /// ★ Neither existing test could see it: the three fixtures in
    /// `scrub_preserves_every_computed_figure` carry no EIN at all, and
    /// `ein_distinctness_is_preserved_exactly` uses two IDENTICAL spellings.
    /// One employer, two standard spellings of its EIN, each W-2 withholding over half the §3101(a)
    /// cap. The correct §6413(c) credit is **$0** and the over-withholding is *stranded* — it appears
    /// in `AbsoluteReturn::excess_ss_not_creditable`, which is what makes this vector exercise BOTH
    /// halves of §8 step 3.
    /// ★★★ THE MALFORMED-EIN VECTOR — the half of r1's CRITICAL the build missed.
    ///
    /// One digit dropped from an EIN (`"11111111"`, eight digits — the ordinary hand-typing typo),
    /// on a filer whose W-2s withhold over the §3101(a) cap. `canonical_ein` returns `None`, so
    /// `over_cap_needs_ein` is true and `screen_inputs` REFUSES `ExcessSsEmployerUnknown`: whether
    /// any of the over-withholding is creditable is genuinely undecidable without a real EIN.
    ///
    /// A scrubber that upgrades it to a well-formed synthetic makes the answer decidable — and wrong.
    fn malformed_ein_over_cap_household() -> (ReturnInputs, crate::state::LedgerState) {
        use crate::tax::return_inputs::W2;
        let (base, state) = w2_only_household();
        let w = base.w2s[0].clone();
        let ri = ReturnInputs {
            w2s: vec![
                W2 {
                    ein: Some("111111111".into()),
                    box3_ss_wages: rust_decimal_macros::dec!(6000),
                    box4_ss_withheld: rust_decimal_macros::dec!(6000),
                    ..w.clone()
                },
                W2 {
                    ein: Some("11111111".into()), // EIGHT digits — undecidable, not a second employer
                    box3_ss_wages: rust_decimal_macros::dec!(6000),
                    box4_ss_withheld: rust_decimal_macros::dec!(6000),
                    ..w
                },
            ],
            ..base
        };
        (ri, state)
    }

    /// ★★★ r1's CRITICAL named this as the adjacent instance its fix **must cover**, SPEC §3.2
    /// mandated it by name — *"Applies to SSN, **EIN**, `business_description`…"* — and the build
    /// shipped an SSN leg and an IP PIN leg and no EIN leg at all.
    ///
    /// A malformed EIN is **undecidable**, and that undecidability is load-bearing: it is what makes
    /// the original refuse. Replacing it with a well-formed synthetic answers a question the filer's
    /// own data cannot answer, so the scrubbed copy FILES where the original refused **and** claims a
    /// §6413(c) credit of $1,546.80 it is not entitled to. The filer is sending this file precisely
    /// to reproduce that refusal.
    #[test]
    fn a_malformed_ein_stays_malformed_so_the_copy_refuses_where_the_original_did() {
        use crate::tax::return_refuse::screen_inputs;
        use crate::tax::testonly::{ty2024_params, ty2024_table};

        let (ri, state) = malformed_ein_over_cap_household();
        let scrubbed = scrub_pii(&ri);

        // The class itself: still undecidable after scrubbing.
        let canon = |r: &ReturnInputs, i: usize| {
            r.w2s[i]
                .ein
                .as_deref()
                .and_then(crate::tax::return_1040::canonical_ein)
        };
        assert!(
            canon(&ri, 1).is_none(),
            "precondition: the original is malformed"
        );
        assert!(
            canon(&scrubbed, 1).is_none(),
            "a MALFORMED EIN must stay malformed — upgrading it decides §6413(c) on a fact the \
             filer's data does not contain"
        );
        assert_ne!(
            ri.w2s[1].ein, scrubbed.w2s[1].ein,
            "…and the real one must not survive"
        );

        // The refusal the filer is trying to reproduce.
        let tbl = ty2024_table();
        let p = ty2024_params();
        assert_eq!(
            screen_inputs(&ri, &tbl, &p).map(|r| r.reason),
            screen_inputs(&scrubbed, &tbl, &p).map(|r| r.reason),
            "the scrubbed copy must refuse exactly as the original does"
        );

        // …and the figure, which is what would actually be filed.
        let a = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
        let b = assemble_absolute(&scrubbed, &state, &ty2024_params(), &ty2024_table(), 2024);
        assert_eq!(
            a.excess_social_security, b.excess_social_security,
            "scrubbing manufactured a §6413(c) credit"
        );
    }

    fn two_spellings_household() -> (ReturnInputs, crate::state::LedgerState) {
        use crate::tax::return_inputs::W2;
        let (base, state) = w2_only_household();
        let w = base.w2s[0].clone();
        let ri = ReturnInputs {
            w2s: vec![
                W2 {
                    ein: Some("11-1111111".into()),
                    box3_ss_wages: rust_decimal_macros::dec!(6000),
                    box4_ss_withheld: rust_decimal_macros::dec!(6000),
                    ..w.clone()
                },
                W2 {
                    ein: Some("111111111".into()), // the SAME employer, the other standard spelling
                    box3_ss_wages: rust_decimal_macros::dec!(6000),
                    box4_ss_withheld: rust_decimal_macros::dec!(6000),
                    ..w
                },
            ],
            ..base
        };
        (ri, state)
    }

    #[test]
    fn two_spellings_of_one_employer_survive_as_one_employer() {
        let (ri, state) = two_spellings_household();
        let scrubbed = scrub_pii(&ri);

        // The partition the PROGRAM'S OWN comparison induces must survive — never string equality.
        let canon = |r: &ReturnInputs| -> Vec<Option<String>> {
            r.w2s
                .iter()
                .map(|w| {
                    w.ein
                        .as_deref()
                        .and_then(crate::tax::return_1040::canonical_ein)
                })
                .collect()
        };
        let c = canon(&scrubbed);
        assert_eq!(
            c[0], c[1],
            "two spellings of ONE employer must canonicalize to one employer after scrubbing, or \
             §6413(c) manufactures an excess-Social-Security credit"
        );
        for (o, n) in ri.w2s.iter().zip(&scrubbed.w2s) {
            assert_ne!(o.ein, n.ein, "the real EIN must not survive");
        }

        // …and the figure itself, which is what the filer would actually be handed.
        let a = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
        let b = assemble_absolute(&scrubbed, &state, &ty2024_params(), &ty2024_table(), 2024);
        assert_eq!(
            a.excess_social_security, b.excess_social_security,
            "scrubbing moved the §6413(c) credit"
        );
        assert_eq!(
            a.total_payments, b.total_payments,
            "scrubbing moved total payments"
        );
    }

    /// ★★★ **THE COARSENING'S WHOLE PURPOSE, AND NO OTHER TEST CAN SEE IT.**
    ///
    /// §3.3's matrix compares `NotDigits` by DISCRIMINANT — it has to, because §3.2 mandates the
    /// payload change — so the matrix is structurally blind to whether the payload leaks. That makes
    /// this the only instrument standing between the filer's real keystroke and a file the command
    /// stamps shareable.
    ///
    /// `SsnError::NotDigits(c)` carries the **first non-digit character of the filer's own entry**
    /// (`packet.rs:66-67`). Preserving it verbatim — which is what value equality would demand, and
    /// what this repo's existing idiom does at `return_refuse.rs:1527` — emits an original identity
    /// byte, contradicting §3.2's governing sentence: *no original identity value is emitted in any
    /// class.*
    #[test]
    fn a_malformed_ssn_keeps_its_variant_without_leaking_the_filers_character() {
        use crate::tax::packet::{Ssn, SsnError};

        let (base, _) = w2_only_household();
        // 'Z' is deliberately distinctive: if it appears anywhere in the output, it came from here.
        let mut ri = base;
        ri.header.taxpayer.ssn = "123-45-678Z".into();
        let scrubbed = scrub_pii(&ri);

        assert!(
            matches!(
                Ssn::canonical(&scrubbed.header.taxpayer.ssn),
                Err(SsnError::NotDigits(_))
            ),
            "the ERROR VARIANT must survive, or the scrubbed copy files where the original refused: \
             {:?}",
            Ssn::canonical(&scrubbed.header.taxpayer.ssn)
        );
        assert!(
            !serde_json::to_string(&scrubbed).unwrap().contains('Z'),
            "the filer's own offending character LEAKED into a file stamped safe to share"
        );
    }

    /// ★★★ THE GUARANTEE: scrubbing changes NO computed figure, on any household.
    ///
    /// Asserted over the whole `AbsoluteReturn` rather than a sampled line, because the failure this
    /// guards is a field nobody thought to check. A scrubbed return that computes differently sends
    /// its recipient after a bug that is not there — worse than not sharing it at all.
    #[test]
    fn scrub_preserves_every_computed_figure() {
        for (name, (ri, state)) in [
            ("kitchen sink", kitchen_sink_household()),
            ("W-2 only", w2_only_household()),
            ("AMT-owing", crate::tax::testonly::amt_owing_household()),
            // ★ §8 step 3: the ONLY vector here carrying an EIN at all, and the only one that
            //   populates `excess_ss_not_creditable` — so it is what makes the `ein` normalization
            //   below load-bearing instead of speculative.
            ("two spellings, one employer", two_spellings_household()),
        ] {
            let scrubbed = scrub_pii(&ri);
            let mut a = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
            let mut b =
                assemble_absolute(&scrubbed, &state, &ty2024_params(), &ty2024_table(), 2024);

            // ★★★ THREE normalisations, each named rather than blanket. They are the identity
            //     strings that survive into `AbsoluteReturn`; every other field there is a figure, a
            //     flag or an enum. Scrubbing them is the POINT — a filer writes their business name
            //     in one — so each is blanked on BOTH sides and everything else must still be equal.
            //
            //     ★ This comment said "ONE normalisation" and called `business_description` "the
            //       ONLY identity-bearing string" long after the second and third members were added
            //       below it. r1 filed that sentence; the fold went around it instead of through it.
            //
            //     ★★ It lands in TWO places, which is why this is a helper and not one line: Schedule
            //        C line A, and Form 8995-A line 1(a)'s trade-or-business name. An identity string
            //        PROPAGATES, so the set of places to normalise grows as the forms grow — and the
            //        growth is safe precisely because forgetting one REDS THIS TEST rather than
            //        silently weakening it.
            //
            //     ★ This is deliberately not a `..`-style exclusion list. If a future field makes the
            //       two sides differ, this test reds and the author must decide whether that field is
            //       identity (blank it here, and scrub it) or a figure (a real bug). The opposite
            //       direction — a string that should have been scrubbed but was not — is held by
            //       `the_identity_does_not_survive`.
            //     ★★★ THE THIRD MEMBER IS `excess_ss_not_creditable[].ein`, and without it this test
            //         REDS ON A CORRECT SCRUB (§3.3). That advisory names the employer to go and ask,
            //         canonicalized — so it carries a scrubbed identity string into `AbsoluteReturn`,
            //         and the two sides differ by `"111111111"` vs `"900000001"` while every figure
            //         matches. The **re-sort by `(owner, amount)`** goes with it: once the eins are
            //         blanked, two entries that differed only by employer are otherwise equal, and
            //         `non_creditable_ss` buckets by canonical EIN — so their ORDER is a function of
            //         identity strings that scrubbing legitimately changes. Comparing unsorted would
            //         make this test fail on employer count, not on any figure.
            //
            //     ★ The normalization set is stated HERE, in full, and never inherited: Schedule C
            //       line A, Form 8995-A line 1(a), and this. Three members, each with a reason.
            let blank_identity = |r: &mut crate::tax::return_1040::AbsoluteReturn| {
                r.printed_inputs.schedule_c_header.business_description = String::new();
                if let Some(p) = r.f8995a_parts_i_to_iii.as_mut() {
                    p.part_i.col_a_name = String::new();
                }
                for nc in &mut r.excess_ss_not_creditable {
                    nc.ein = String::new();
                }
                // ★ Keyed on the Debug rendering of `owner` rather than deriving `Ord` on it: this is
                //   a test-local normalization and it must not reach back into a production type to
                //   get one.
                r.excess_ss_not_creditable.sort_by(|x, y| {
                    (format!("{:?}", x.owner), x.amount).cmp(&(format!("{:?}", y.owner), y.amount))
                });
            };
            blank_identity(&mut a);
            blank_identity(&mut b);
            assert_eq!(a, b, "{name}: scrubbing moved a computed figure");
        }
    }

    /// ★★★ EIN SAMENESS SURVIVES — the §6413(c) excess-SS credit turns on "more than one employer",
    /// so collapsing two employers into one (or splitting one into two) manufactures or destroys a
    /// credit. This is the property the invariant above cannot see on a single-W-2 fixture.
    #[test]
    fn ein_distinctness_is_preserved_exactly() {
        let (mut ri, _) = kitchen_sink_household();
        let w = ri.w2s[0].clone();
        ri.w2s = vec![
            crate::tax::return_inputs::W2 {
                ein: Some("11-1111111".into()),
                ..w.clone()
            },
            crate::tax::return_inputs::W2 {
                ein: Some("11-1111111".into()), // SAME employer
                ..w.clone()
            },
            crate::tax::return_inputs::W2 {
                ein: Some("22-2222222".into()), // a DIFFERENT one
                ..w
            },
        ];
        let s = scrub_pii(&ri);
        let e: Vec<Option<String>> = s.w2s.iter().map(|w| w.ein.clone()).collect();
        assert_eq!(
            e[0], e[1],
            "two W-2s from ONE employer must stay one employer"
        );
        assert_ne!(e[1], e[2], "a second employer must stay a second employer");
        for (orig, new) in ri.w2s.iter().zip(&s.w2s) {
            assert_ne!(orig.ein, new.ein, "the real EIN must not survive");
        }
    }

    /// ★★ The identity is actually gone — a scrubber that quietly kept a field would still pass the
    /// invariant above, because names move no figure. Both halves are needed.
    #[test]
    fn the_identity_does_not_survive() {
        let (mut ri, _) = kitchen_sink_household();
        ri.header.ip_pin = Some("123456".into());

        // ★★★ EXHAUSTIVE, NOT A HAND-LIST (r1: this named 6 of 16 fields and left the ENTIRE SPOUSE
        //     untested). The taxpayer and the spouse are the same `Person` type, so a scrubber
        //     correct on one and broken on the other is exactly the asymmetry a per-field list
        //     misses — and `kitchen_sink_household` has no spouse, so the loop below would silently
        //     cover nothing. One is planted here.
        ri.header.spouse = Some(crate::tax::return_inputs::Person {
            first_name: "Spousefirst".into(),
            last_name: "Spouselast".into(),
            ssn: "000-77-7777".into(),
            date_of_birth: Some(time::macros::date!(1972 - 02 - 02)),
            date_of_death: None,
            blind: Some(false),
            occupation: "Spouseoccupation".into(),
        });

        let s = scrub_pii(&ri);

        // Every identity field of BOTH people, driven off one closure so neither can be forgotten.
        let check_person = |o: &crate::tax::return_inputs::Person,
                            n: &crate::tax::return_inputs::Person,
                            who: &str| {
            assert_ne!(o.first_name, n.first_name, "{who}: first name survived");
            assert_ne!(o.last_name, n.last_name, "{who}: last name survived");
            assert_ne!(o.ssn, n.ssn, "{who}: SSN survived");
            assert_ne!(o.occupation, n.occupation, "{who}: occupation survived");
            // ★★ KEPT, and asserted in the same breath: §63(f)'s age-65 and blindness additions read
            //    these, so scrubbing them would move the deduction.
            assert_eq!(
                o.date_of_birth, n.date_of_birth,
                "{who}: DOB is computational"
            );
            assert_eq!(
                o.date_of_death, n.date_of_death,
                "{who}: DOD is computational"
            );
            assert_eq!(o.blind, n.blind, "{who}: blindness is computational");
        };
        check_person(&ri.header.taxpayer, &s.header.taxpayer, "taxpayer");
        check_person(
            ri.header.spouse.as_ref().unwrap(),
            s.header
                .spouse
                .as_ref()
                .expect("the spouse must survive AS a spouse"),
            "spouse",
        );

        assert_ne!(s.header.address_street, ri.header.address_street);
        assert_ne!(s.header.address_city, ri.header.address_city);
        assert_ne!(s.header.address_state, ri.header.address_state);
        assert_ne!(s.header.address_zip, ri.header.address_zip);
        // ★ §3.4's precondition, so the four assertions above cannot pass VACUOUSLY. They used to:
        //   the stand-in address WAS `Springfield / IL / 62704`, which is what the fixtures carry.
        assert_ne!(ri.header.address_city, SCRUB_CITY, "§3.4 precondition");
        assert_ne!(ri.header.address_zip, SCRUB_ZIP, "§3.4 precondition");

        assert_eq!(
            s.header.ip_pin, None,
            "an IP PIN is a live credential: DROPPED"
        );

        // ★★★ LENGTH FIRST. `zip` silently truncates to the shorter side, so a scrubber that DROPPED
        //     every dependent would satisfy the loop below by iterating zero times.
        assert_eq!(
            ri.header.dependents.len(),
            s.header.dependents.len(),
            "the NUMBER of dependents is computational and must survive"
        );
        assert!(
            !ri.header.dependents.is_empty(),
            "the fixture must actually have dependents, or the loop below asserts nothing"
        );
        for (o, n) in ri.header.dependents.iter().zip(&s.header.dependents) {
            assert_ne!(o.ssn, n.ssn);
            assert_ne!(o.name, n.name, "a dependent's NAME is identity");
            assert_eq!(
                o.relationship, n.relationship,
                "relationship is computational"
            );
            // ★ §6: DROPPED, not preserved. Nothing reads a dependent's DOB — btctax does not
            //   compute the CTC — so keeping it is retention with no computational warrant.
            assert_eq!(
                n.date_of_birth, None,
                "a dependent's DOB is DROPPED: nothing reads it, so retaining a child's date of \
                 birth in a file stamped shareable buys nothing"
            );
        }
        // ★★ …and the two fields a security review found riding through the FIRST version of this
        //    module: free text where a filer writes a business name, and the countries they bank in.
        let mut ri2 = ri.clone();
        if let Some(c) = ri2.schedule_c.as_mut() {
            c.business_description = "Smith Family Dental".into();
        }
        ri2.foreign_accounts = Some(true);
        ri2.foreign_country_names = "Switzerland, Liechtenstein".into();
        let s2 = scrub_pii(&ri2);
        assert_eq!(
            s2.schedule_c
                .as_ref()
                .map(|c| c.business_description.as_str()),
            Some("Example business"),
            "a business NAME is free text and must not survive"
        );
        assert!(
            !s2.foreign_country_names.contains("Switzerland")
                && !s2.foreign_country_names.contains("Liechtenstein"),
            "which countries someone banks in is identifying: {}",
            s2.foreign_country_names
        );
        assert_eq!(
            s2.foreign_country_names.split(',').count(),
            2,
            "the COUNT is kept — Schedule B line 7b prints the list and an empty one refuses"
        );

        // ★ …and the SSNs it emits are structurally impossible, so the output is safe to commit.
        assert!(
            s.header.taxpayer.ssn.contains("-00-"),
            "middle group 00 is never issued: {}",
            s.header.taxpayer.ssn
        );
    }
}
