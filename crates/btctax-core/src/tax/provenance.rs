//! ★★★ **R10 — PROVENANCE IS STRUCTURAL** (`design/SPEC_interview.md` §3 R10, §5.6; build task T1).
//!
//! Four parts, all reachable from [`ReturnInputs`], and all of them the kind of fact that **cannot be
//! back-filled**: nobody can reconstruct in December which words a filer was shown in September, so the
//! schema has to exist before the interview does.
//!
//! 1. **Source by struct** — [`LEAF_SOURCE`] maps every money leaf's serde path prefix to the [`Source`]
//!    it comes from. There is deliberately **no per-leaf metadata**: a box on a declared document is
//!    testimony *because the row exists*, so the provenance of an amount is the struct it lives in.
//! 2. **Identity per document** — `payer_tin` / `transcribed_on` on the information-return structs
//!    (in [`super::return_inputs`], where the documents are).
//! 3. **Per answer** — [`AnswerKey`] → [`AnswerRecord`] in `ReturnInputs::answer_log`, written by the ONE
//!    writer [`record_answer`], superseded (never overwritten) into `answer_log_history`.
//! 4. **Year N+1** — [`super::return_inputs::CarryProvenance::ComputedFromPriorReturn`] (the opener
//!    itself is task T4b).
//!
//! ★★ **What the log deliberately does NOT hold** (`FIELD_PROVENANCE.md:400-403`): progress, position,
//! "what remains", superseded *values*, half-typed tokens. A record says *when* an answer was given and
//! *which words were asked* — never how far through the interview the filer got.

use crate::tax::questions::{QuestionId, SkippableId, FORM_QUESTIONS, SKIPPABLE_QUESTIONS};
use crate::tax::return_inputs::ReturnInputs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use time::Date;

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 1. Source by struct — LEAF_SOURCE
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A document type btctax transcribes. The provenance of every money leaf inside one of these
/// structs is *"box N of this document"*, which is why no leaf carries its own source tag.
///
/// ★ Only the kinds whose struct exists today. The full §5.1 census (1099-R, SSA-1099, K-1 …) is
/// task T3's `DocumentCensus`; a kind is added here when its struct lands, and adding a variant reds
/// every exhaustive match — which is the intended blast radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    W2,
    Form1099Int,
    Form1099Div,
    Form1099G,
    Form1099B,
}

/// Where one money leaf's figure comes from (R10 part 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A numbered box on a document the filer holds and transcribed.
    Document(DocumentKind),
    /// The filer's OWN records — a figure no information return reports (R5). The form asks for it by
    /// name and the filer reads it off their own books.
    FilerRecords,
    /// btctax's crypto ledger — a figure the tool computed from transactions, never typed by the filer.
    Ledger,
    /// An answer to a registry question (a figure that IS the answer, rather than a yes/no).
    Answer,
    /// Computed by btctax from other lines, or carried from a prior year's computed return.
    Computed,
}

/// ★★★ **THE PER-LEAF PROVENANCE TABLE** — every money (`Usd` / `Option<Usd>`) leaf of
/// [`ReturnInputs`], by serde path **prefix**, mapped to the [`Source`] it comes from.
///
/// The §8 guarantee it exists for: *"no `Money` field outside a document or filer's-records struct;
/// every `Usd` leaf has one source"*. Its kill runs **both directions** — every money leaf matches
/// exactly one prefix, and every prefix matches at least one money leaf (the stale-exemption
/// discipline `coverage.rs` already applies to its own lists).
///
/// ★ Longest-prefix wins is NOT used, and that is deliberate: an entry that is a prefix of another
/// entry would make two rows both "match", and the KAT reds on it. Every prefix here is disjoint, so
/// the table can be read as a partition rather than as a priority list.
pub const LEAF_SOURCE: &[(&str, Source)] = &[
    // ── Documents ────────────────────────────────────────────────────────────────────────────────
    ("w2s", Source::Document(DocumentKind::W2)),
    ("int_1099", Source::Document(DocumentKind::Form1099Int)),
    ("div_1099", Source::Document(DocumentKind::Form1099Div)),
    ("g_1099", Source::Document(DocumentKind::Form1099G)),
    ("b_1099", Source::Document(DocumentKind::Form1099B)),
    // ── The filer's own records ──────────────────────────────────────────────────────────────────
    // Schedule A: medical, SALT, interest, gifts — every one a figure the filer reads off their own
    // books or a statement btctax does not transcribe. (`mortgage_interest_1098` moves to a
    // `Form1098` document row in task T9; the prefix follows it then.)
    ("schedule_a", Source::FilerRecords),
    ("sch1", Source::FilerRecords),
    ("schedule_c", Source::FilerRecords),
    ("schedule_1a", Source::FilerRecords),
    ("payments", Source::FilerRecords),
    ("qbi", Source::FilerRecords),
    // ★ Form 8960 line 9b — i8960's *"any reasonable method"* allocation. Collected, never computed:
    //   the method is the FILER'S election, so the figure is theirs.
    ("form_8960_line9b", Source::FilerRecords),
    // §164(b)(7)(B)(iv) / Schedule 1-A Part I add-backs — each *"enter the amount from"* another form
    // the filer prepared (Form 2555 lines 45/50, Form 4563 line 15) or their Puerto Rico exclusion.
    ("excluded_puerto_rico_income", Source::FilerRecords),
    ("form_2555_line45", Source::FilerRecords),
    ("form_2555_line50", Source::FilerRecords),
    ("form_4563_line15", Source::FilerRecords),
    // ── Computed (by btctax, this year or a prior one) ───────────────────────────────────────────
    // The two carryovers in. `CarryProvenance` (the sibling scalar) says WHICH computation — this
    // year's write-back, the filer's own entry, or `ComputedFromPriorReturn` (T4b).
    ("capital_loss_carryforward_in", Source::Computed),
    ("charitable_carryover_in", Source::Computed),
];

/// Resolve one serde leaf path to its [`Source`], or `None` when no [`LEAF_SOURCE`] prefix claims it.
///
/// A prefix matches `p` exactly, or `p.` (a nested field), or `p[` (a row of a `Vec`) — the same
/// matcher `coverage.rs` uses for its exemption prefixes, so the two lists mean the same thing by
/// "prefix".
pub fn source_of_leaf(path: &str) -> Option<Source> {
    LEAF_SOURCE
        .iter()
        .find(|(p, _)| prefix_matches(p, path))
        .map(|(_, s)| *s)
}

fn prefix_matches(prefix: &str, path: &str) -> bool {
    path == prefix
        || path.starts_with(&format!("{prefix}."))
        || path.starts_with(&format!("{prefix}["))
}

/// ★★★ **THE LEAF WALK — test scaffolding, in the library on purpose.**
///
/// [`money_leaves`] is the type-driven money detector the [`LEAF_SOURCE`] KAT is built on, and
/// **T4b's kill needs the same walk from another crate**: *"no `Usd` leaf of the year-N+1 seed is
/// non-zero except the carryforwards"* is exactly this question asked of a different fixture, and
/// the whole point of R10.4's kill is that it reads no hand-list of fields. A second copy in
/// `btctax-cli` would be a second thing to keep true — so this lives here, `#[doc(hidden)]`, for
/// the same reason [`crate::tax::testonly`] does: one walk, one detector, two callers.
///
/// It contains no tax logic and nothing production reads it.
#[doc(hidden)]
pub mod leaf_walk {
    use super::ReturnInputs;
    use crate::conventions::Usd;
    use serde_json::Value;
    use std::collections::{BTreeMap, BTreeSet};

    // ─────────────────────────────────────────────────────────────────────────────────────────────
    // The leaf walk — the sibling of `btctax-input-form`'s coverage walk, with the same rules: a
    // leaf is a scalar, or an all-scalar array (a serialized `time::Date` is ONE leaf, not two).
    // ─────────────────────────────────────────────────────────────────────────────────────────────
    pub fn walk(v: &Value, prefix: &str, out: &mut Vec<String>) {
        match v {
            Value::Object(map) => {
                for (k, child) in map {
                    let p = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    walk(child, &p, out);
                }
            }
            Value::Array(arr) if arr.iter().any(|e| e.is_object() || e.is_array()) => {
                for (i, child) in arr.iter().enumerate() {
                    walk(child, &format!("{prefix}[{i}]"), out);
                }
            }
            _ => out.push(prefix.to_string()),
        }
    }

    /// Replace the value at one walked leaf path. Returns `false` if the path does not resolve.
    pub fn set_at(v: &mut Value, path: &str, new: Value) -> bool {
        let mut cur = v;
        let mut rest = path;
        loop {
            // A segment is `name` (after an optional leading '.') or `[i]`.
            if let Some(after) = rest.strip_prefix('[') {
                let Some((idx, tail)) = after.split_once(']') else {
                    return false;
                };
                let Ok(i) = idx.parse::<usize>() else {
                    return false;
                };
                let Some(next) = cur.get_mut(i) else {
                    return false;
                };
                if tail.is_empty() {
                    *next = new;
                    return true;
                }
                cur = next;
                rest = tail.strip_prefix('.').unwrap_or(tail);
                continue;
            }
            let end = rest.find(['.', '[']).unwrap_or(rest.len());
            let (name, tail) = rest.split_at(end);
            let Some(next) = cur.get_mut(name) else {
                return false;
            };
            if tail.is_empty() {
                *next = new;
                return true;
            }
            cur = next;
            rest = tail.strip_prefix('.').unwrap_or(tail);
        }
    }

    /// ★★★ **MONEY LEAVES, DETECTED BY TYPE — never by a hand-list and never by a value.**
    ///
    /// For each leaf: write a decimal-shaped string, then a non-numeric one, and deserialize the whole
    /// blob back into [`ReturnInputs`] each time. A leaf that ACCEPTS `"1234.56"` and REJECTS `"zzz"`
    /// is a `Usd` / `Option<Usd>`; a `String` accepts both, a `bool` / `Date` / enum rejects both.
    ///
    /// ★ That is what makes this drift-proof in the direction that matters: a newly added money field
    /// on any reachable struct is detected the moment the fixture realizes it, with nobody having
    /// remembered to list it. It is `Decimal`'s own deserializer doing the classifying.
    ///
    /// ★★ **Its honest limits, stated rather than hidden — and MEASURED, because a wrong stated limit
    /// is worse than none** (seam review M4). It can only classify a leaf that appears in the
    /// serialized JSON at all, so there are exactly two blind spots, neither of them the one this
    /// comment used to name:
    ///
    /// 1. **A field that is not serialized.** `#[serde(skip_serializing_if = "Option::is_none")]`
    ///    (used in `forms.rs:293-295`) removes the key entirely when it is `None`, so [`walk`] never
    ///    emits a leaf for it and no probe is ever written. Nothing here can see a leaf that is not
    ///    in the document.
    /// 2. **The elements of an EMPTY `Vec`.** `walk` descends an array only when some element is
    ///    itself an object or array, so `[]` is pushed as one leaf at the vec's own path — and that
    ///    leaf rejects both probes (a string is not a `Vec`). Every money box on the element type is
    ///    therefore unwalked. This is why [`maximal_sentinel`] realizes two rows of every `Vec`.
    ///
    /// ★ A new `Option<Usd>` left `None` is **NOT** a blind spot, contrary to what this comment said
    ///   before it was checked: it serializes as `null`, `walk` emits it, `set_at` replaces the whole
    ///   value with the probe string, and the `Option<Usd>` deserializer then classifies it correctly.
    ///   Planted and observed — the KAT reds on such a field.
    ///
    /// ★ The second net either way is the classifier, which forbids `_` on an `Option<Usd>` leaf
    ///   (`no_option_money_leaf_is_bound_with_underscore`), so a money leaf cannot be added without a
    ///   human naming it.
    pub fn money_leaves(ri: &ReturnInputs) -> BTreeSet<String> {
        let base = serde_json::to_value(ri).expect("ReturnInputs serializes");
        serde_json::from_value::<ReturnInputs>(base.clone())
            .expect("the fixture must round-trip before any leaf is probed");
        let mut leaves = Vec::new();
        walk(&base, "", &mut leaves);
        let accepts = |path: &str, probe: &str| {
            let mut v = base.clone();
            assert!(
                set_at(&mut v, path, Value::String(probe.into())),
                "unreachable leaf {path}"
            );
            serde_json::from_value::<ReturnInputs>(v).is_ok()
        };
        leaves
            .into_iter()
            .filter(|p| accepts(p, "1234.56") && !accepts(p, "zzz"))
            .collect()
    }

    /// The money leaves of `ri` that are NOT zero, path → amount.
    ///
    /// ★ Built on [`money_leaves`], so it inherits the type-driven detector rather than naming any
    ///   field: a money box added tomorrow is examined the day the fixture realizes it. The value is
    ///   read back out of the same serialized document the detector classified, never off a struct
    ///   field, so a leaf cannot be examined under one name and reported under another.
    pub fn nonzero_money_leaves(ri: &ReturnInputs) -> BTreeMap<String, Usd> {
        let doc = serde_json::to_value(ri).expect("ReturnInputs serializes");
        money_leaves(ri)
            .into_iter()
            .filter_map(|path| {
                let v = at(&doc, &path)?;
                // `Usd` serializes as a decimal STRING (`serde-str`); an `Option<Usd>` left `None`
                // is `null`, which is not a figure and cannot be non-zero.
                let amount: Usd = v.as_str()?.parse().ok()?;
                (amount != Usd::ZERO).then_some((path, amount))
            })
            .collect()
    }

    /// Read the value at one walked leaf path — the read counterpart of [`set_at`].
    pub fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
        let mut cur = v;
        let mut rest = path;
        loop {
            if let Some(after) = rest.strip_prefix('[') {
                let (idx, tail) = after.split_once(']')?;
                cur = cur.get(idx.parse::<usize>().ok()?)?;
                if tail.is_empty() {
                    return Some(cur);
                }
                rest = tail.strip_prefix('.').unwrap_or(tail);
                continue;
            }
            let end = rest.find(['.', '[']).unwrap_or(rest.len());
            let (name, tail) = rest.split_at(end);
            cur = cur.get(name)?;
            if tail.is_empty() {
                return Some(cur);
            }
            rest = tail.strip_prefix('.').unwrap_or(tail);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 3. Per answer — the answer log
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ **The dependent gate identity** — §5.3's sixteen §152 gates, the four row-(5)/(6) facts, and
/// the row's date of birth (which R6 makes REQUIRED, so its absence blocks like a gate).
///
/// The gates themselves are task T7's fields on `Dependent`; the **identity** is T1's, because the
/// diligence key is exactly the thing that cannot be back-filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependentGate {
    QcRelationship,
    ProvidedOverHalfOwnSupport,
    FilingJointReturn,
    JointReturnOnlyToClaimRefund,
    QualifyingChildOfAnotherPerson,
    CitizenNationalResidentOrCanadaMexico,
    Married,
    TinIssuedByDueDate,
    CitizenNationalOrResidentAlien,
    SsnsValidForEmploymentIssuedByDueDate,
    QrRelationshipOrMemberOfHousehold,
    QualifyingChildOfAnyTaxpayer,
    GrossIncomeUnderLimit,
    YouProvidedOverHalfSupport,
    DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
    YoungerThanYouOrSpouse,
    LivedWithYouOverHalfYear,
    LivedWithYouInUs,
    FullTimeStudent,
    PermanentlyAndTotallyDisabled,
    DateOfBirth,
}

impl DependentGate {
    /// Every gate identity. The exhaustive `match` in the completeness test makes a new variant a
    /// COMPILE ERROR until it is listed here.
    pub const ALL: &'static [DependentGate] = &[
        DependentGate::QcRelationship,
        DependentGate::ProvidedOverHalfOwnSupport,
        DependentGate::FilingJointReturn,
        DependentGate::JointReturnOnlyToClaimRefund,
        DependentGate::QualifyingChildOfAnotherPerson,
        DependentGate::CitizenNationalResidentOrCanadaMexico,
        DependentGate::Married,
        DependentGate::TinIssuedByDueDate,
        DependentGate::CitizenNationalOrResidentAlien,
        DependentGate::SsnsValidForEmploymentIssuedByDueDate,
        DependentGate::QrRelationshipOrMemberOfHousehold,
        DependentGate::QualifyingChildOfAnyTaxpayer,
        DependentGate::GrossIncomeUnderLimit,
        DependentGate::YouProvidedOverHalfSupport,
        DependentGate::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
        DependentGate::YoungerThanYouOrSpouse,
        DependentGate::LivedWithYouOverHalfYear,
        DependentGate::LivedWithYouInUs,
        DependentGate::FullTimeStudent,
        DependentGate::PermanentlyAndTotallyDisabled,
        DependentGate::DateOfBirth,
    ];
}

/// ★★★ **THE KEY OF ONE ANSWER — an IDENTITY, never a position** (R10.3 / fold I10).
///
/// `Dependent` rows are a `Vec` with `add` and `remove` through the form seam, so a
/// `DependentGate { row, gate }` key would move one child's `answered_on` and `prompt_hash` onto
/// another the moment row 0 is deleted — and *"a diligence record that lies is worse than none"*.
/// The key is therefore the row's `ssn_hash`, never the digits and never the index.
///
/// (The *refusal* `DependentGateUnanswered { row, gate }` keeps the row index — it points the filer at
/// a position on screen and is not stored.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnswerKey {
    Question(QuestionId),
    Skippable(SkippableId),
    DependentGate {
        ssn_hash: String,
        gate: DependentGate,
    },
}

/// The serde wire form of an [`AnswerKey`] — a stable string, because a `BTreeMap` key must be one.
impl fmt::Display for AnswerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnswerKey::Question(id) => write!(f, "question:{id:?}"),
            AnswerKey::Skippable(id) => write!(f, "skippable:{id:?}"),
            AnswerKey::DependentGate { ssn_hash, gate } => {
                write!(f, "dependent:{ssn_hash}:{gate:?}")
            }
        }
    }
}

/// Why a stored key could not be read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerKeyParseError(pub String);

impl fmt::Display for AnswerKeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a recognised answer-log key: {:?}", self.0)
    }
}

impl FromStr for AnswerKey {
    type Err = AnswerKeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || AnswerKeyParseError(s.to_string());
        if let Some(rest) = s.strip_prefix("question:") {
            return QuestionId::ALL
                .iter()
                .find(|id| format!("{id:?}") == rest)
                .map(|id| AnswerKey::Question(*id))
                .ok_or_else(err);
        }
        if let Some(rest) = s.strip_prefix("skippable:") {
            return SkippableId::ALL
                .iter()
                .find(|id| format!("{id:?}") == rest)
                .map(|id| AnswerKey::Skippable(*id))
                .ok_or_else(err);
        }
        if let Some(rest) = s.strip_prefix("dependent:") {
            // `ssn_hash` is hex, so it never contains ':' — split on the LAST one.
            let (hash, gate) = rest.rsplit_once(':').ok_or_else(err)?;
            let gate = DependentGate::ALL
                .iter()
                .find(|g| format!("{g:?}") == gate)
                .ok_or_else(err)?;
            return Ok(AnswerKey::DependentGate {
                ssn_hash: hash.to_string(),
                gate: *gate,
            });
        }
        Err(err())
    }
}

impl Serialize for AnswerKey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AnswerKey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        AnswerKey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// ★★ **`Declined` is a STATE, never a default.** A skippable the filer deliberately passed over is
/// *asked and refused*; a leaf that is `None` with **no record at all** was never asked. Those two are
/// the same bytes on the page and must never be the same thing in the model — R12 keeps a `Declined`
/// benefit in the *forgoing* list precisely because declining is provenance, not absence.
///
/// ★ **It is WRITTEN here and READ by nothing in production yet** (seam review N3), and that is the
///   same *"a stored value with no reader is not a guarantee"* caveat this module states for
///   [`AnswerRecord::prompt_hash`]. `income answer` writes it; the only production reader of the
///   answer types today is `return_refuse`'s `== WordingChanged`, which treats `Declined`, `Given`
///   and `NeverAsked` alike. Its reader is **T3** — `interview_state()`, which lists a `Declined`
///   class-(B) item in `forgoing` marked *(declined)* and never in `blocking` — rendered by **T12**.
///   Expected at T1; it stops being acceptable if T3 slips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerState {
    Given,
    Declined,
}

/// One answer, as the record of an ACT: when it was given, and which words were on the screen.
///
/// ★ `prompt_hash` has exactly ONE reader — [`answer_status`], detecting that the words changed
/// (R10.3). A stored value with no reader is not a guarantee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerRecord {
    pub answered_on: Date,
    pub prompt_hash: String,
    pub state: AnswerState,
}

/// The append-only history: every record superseded by a `prompt_hash` mismatch or a changed `ssn`.
/// **Nothing reads it as an answer** (§5.6) — it exists so a superseded record is kept rather than
/// destroyed, and so `answer_log` only ever holds what stands today.
pub type AnswerLogHistory = Vec<(AnswerKey, AnswerRecord)>;

/// The hash of the words a filer was shown. Not a secret — a prompt is public text — so this is a
/// plain content fingerprint, and its only job is to be *different* when the sentence changes.
pub fn prompt_hash(prompt: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"btctax:prompt:v1\0");
    h.update(prompt.as_bytes());
    format!("{:x}", h.finalize())
}

/// ★★ **The dependent row's identity key — a salted hash of its SSN, never the digits.**
///
/// The salt is a fixed domain separator rather than a per-return random value, because the key must
/// be *stable* (the log is looked up by it on every read) and deterministic across the CLI and TUI
/// writers. It is **domain separation, not confidentiality**: the row's `ssn` is stored as entered in
/// the same encrypted blob, so anyone who can read the log can already read the SSN. What the hash
/// buys is that the *key* — the thing that ends up in a panel, a manifest line or a diagnostic — is
/// never the nine digits.
pub fn dependent_ssn_hash(ssn: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"btctax:dependent-ssn:v1\0");
    // Normalise the punctuation a filer may or may not type: "111-22-3333" and "111223333" are one
    // person, and an identity that changed when a dash was added would move records between children.
    let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
    h.update(digits.as_bytes());
    format!("{:x}", h.finalize())
}

/// ★★★ **THE ONE WRITER of `answer_log`** (R10.3).
///
/// Every surface reaches it: the form engine's `apply` (so the TUI records), and `income answer` (so
/// the CLI records). Two writers would drift, and the drift would be invisible — an answer given at
/// the keyboard and the same answer given in the editor must produce the **same record**, which is
/// what the T1 kill asserts byte for byte.
///
/// ★ **Deviation from R10.3's four-argument sketch, recorded deliberately:** the same rule requires
/// `Declined` to be recordable (*"a skippable skipped on purpose records `Declined`"*), which a
/// four-argument `record_answer(ri, key, prompt, now)` cannot express. `state` is therefore the fifth
/// parameter rather than a second writer function — one writer is the guarantee, four arguments were
/// only the sketch.
/// ★★★ **…AND THE ONE WRITER IS WHERE R10.3's HALF TWO LIVES** (seam review I1).
///
/// R10.3: *"**The old answer is kept as history and never as the current answer:** the superseded
/// `AnswerRecord` moves to `answer_log_history`, append-only, which nothing reads as an answer, **and
/// the re-answer writes a fresh record**."* The supersession is tied to the RE-ANSWER, and the
/// re-answer is exactly this function — so the move happens here, on the one path every surface
/// takes, rather than in a sweep somebody has to remember to call. (It was in a sweep, with zero
/// production callers: a function fully tested and never run.)
///
/// **Only a record whose `prompt_hash` DIFFERS is historised.** Re-answering the same question under
/// the same words is a correction, not a supersession — R10.3 names the changed wording as what
/// history holds (with `supersede_dependent_identity`'s changed `ssn`), and `forget_answer`'s doc
/// gives the same reason from the other side. So a filer who flips an answer back and forth under
/// one prompt cannot inflate the history, and the writer is idempotent in the way that matters.
///
/// ★ **Why NOT also sweep at the read boundary** (`return_inputs::row_to_inputs`), the other option
///   the review offered: it would *disarm the refusal it exists to serve.* R10.3's preceding sentence
///   is that `screen_inputs` refuses a class-(A) record whose hash disagrees, and R12's table lists
///   that record as **blocking** — both of which need the stale record still IN `answer_log` at read
///   time. Sweeping it into history on load makes `answer_status` report `NeverAsked`, and *"an
///   absent record is not a mismatch"* then lets a September answer stand silently under November's
///   words. Measured: with the sweep at the read boundary, `an_answer_hashed_against_earlier_words_
///   refuses_and_a_missing_record_does_not`'s end-to-end shape stops refusing. Supersession belongs
///   at the re-answer, which is where the spec puts it.
pub fn record_answer(
    ri: &mut ReturnInputs,
    key: AnswerKey,
    prompt: &str,
    now: Date,
    state: AnswerState,
) {
    let hash = prompt_hash(prompt);
    if let Some(superseded) = ri.answer_log.get(&key) {
        if superseded.prompt_hash != hash {
            let superseded = superseded.clone();
            ri.answer_log_history.push((key.clone(), superseded));
        }
    }
    ri.answer_log.insert(
        key,
        AnswerRecord {
            answered_on: now,
            prompt_hash: hash,
            state,
        },
    );
}

/// Un-answer: drop the current record, returning the question to **never asked**.
///
/// ★ It does NOT write history, and it does NOT write `Declined`. §5.6 says exactly what history
/// holds — records superseded by a changed prompt or a changed `ssn` — and a clear is neither. And
/// `Declined` is the answer *"I was asked and chose to pass"*, which is not what un-answering a
/// class-(A) declaration means (a declaration has no lawful decline at all).
pub fn forget_answer(ri: &mut ReturnInputs, key: &AnswerKey) {
    ri.answer_log.remove(key);
}

/// The answered-ness of one question **as the log sees it** (R10.3 / R12's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerStatus {
    /// No record: never asked. (The leaf's own value still decides whether that blocks.)
    NeverAsked,
    /// Answered under the words currently on the screen.
    Given,
    /// Asked and deliberately passed over — provenance, and still a forgone benefit (R12).
    Declined,
    /// ★★★ Answered under **earlier words**. Treated as UNANSWERED everywhere: blocking for class (A),
    /// forgoing for class (B), and refused by `screen_inputs`.
    WordingChanged,
}

/// The reason string R12's panel prints beside a re-asked question, and `screen_inputs`' refusal
/// detail names. One constant so the panel, the refusal and the test cannot drift apart.
pub const WORDING_CHANGED_REASON: &str = "the wording of this question changed since you answered";

/// Read one key's [`AnswerStatus`] against the words currently asked.
pub fn answer_status(ri: &ReturnInputs, key: &AnswerKey, current_prompt: &str) -> AnswerStatus {
    match ri.answer_log.get(key) {
        None => AnswerStatus::NeverAsked,
        Some(r) if r.prompt_hash != prompt_hash(current_prompt) => AnswerStatus::WordingChanged,
        Some(r) => match r.state {
            AnswerState::Given => AnswerStatus::Given,
            AnswerState::Declined => AnswerStatus::Declined,
        },
    }
}

/// The words currently asked for `key`, from the registry that owns it.
///
/// ★ `None` for a [`AnswerKey::DependentGate`]: the per-gate prompts live in the `DEPENDENT_GATES`
/// registry, which is task T7. Until it exists a dependent record has no current prompt to compare
/// against, so nothing outside [`record_answer`] can decide a dependent record is stale — and
/// `record_answer` never has to, because it hashes the prompt the caller actually showed.
pub fn current_prompt(key: &AnswerKey, ri: &ReturnInputs) -> Option<Cow<'static, str>> {
    match key {
        // ★★ R10.4 — `prompt_text`, not `prompt`: a question whose subject is a value ON the return
        //    (today, the carried filing status) is asked in words that quote it, and those are the
        //    words that must be hashed. Reading the static `prompt` here would hand every surface a
        //    hash that never changes when the value does, which is the re-ask rule silently disabled.
        AnswerKey::Question(id) => FORM_QUESTIONS
            .iter()
            .find(|q| q.id == *id)
            .map(|q| q.prompt_text(ri)),
        AnswerKey::Skippable(id) => SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == *id)
            .map(|s| Cow::Borrowed(s.prompt)),
        AnswerKey::DependentGate { .. } => None,
    }
}

/// Every `answer_log` key belonging to one dependent identity.
fn keys_for_identity(log: &BTreeMap<AnswerKey, AnswerRecord>, hash: &str) -> Vec<AnswerKey> {
    log.keys()
        .filter(|k| matches!(k, AnswerKey::DependentGate { ssn_hash, .. } if ssn_hash == hash))
        .cloned()
        .collect()
}

/// ★★ **The dependent row was REMOVED** — delete that identity's records (R10.3: *"`remove` on the
/// Dependents section deletes that identity's entries"*).
///
/// Deleted, not historied: the filer withdrew the row, so there is no superseded *answer* to keep —
/// there is no longer a person the record could be about. Returns how many entries went.
///
/// ★ **A row with a BLANK `ssn` has no identity**, so every blank row shares one bucket and removing
/// one clears it. That is the fail-closed direction — the alternative leaves records a *new* blank row
/// would silently inherit — but it is a degenerate state, not a designed one: R6 makes a dependent's
/// identity fields mandatory, and the per-gate registry that would let a blank row be answered at all
/// is task T7. Recorded here so T7 decides it rather than meets it.
pub fn retire_dependent_identity(ri: &mut ReturnInputs, ssn: &str) -> usize {
    let hash = dependent_ssn_hash(ssn);
    let keys = keys_for_identity(&ri.answer_log, &hash);
    for k in &keys {
        ri.answer_log.remove(k);
    }
    keys.len()
}

/// ★★ **The dependent row's `ssn` CHANGED** — a different person, so *"the old entries move to the
/// history"* and the new identity starts fresh (R10.3).
///
/// A no-op when the two SSNs hash the same (a filer adding the dashes to `111223333` is not a
/// different child). Returns how many records were superseded.
pub fn supersede_dependent_identity(ri: &mut ReturnInputs, old_ssn: &str, new_ssn: &str) -> usize {
    let old = dependent_ssn_hash(old_ssn);
    if old == dependent_ssn_hash(new_ssn) {
        return 0;
    }
    let keys = keys_for_identity(&ri.answer_log, &old);
    for k in &keys {
        if let Some(rec) = ri.answer_log.remove(k) {
            ri.answer_log_history.push((k.clone(), rec));
        }
    }
    keys.len()
}

#[cfg(test)]
mod tests {
    use super::leaf_walk::{money_leaves, walk};
    use super::*;
    use crate::tax::return_inputs::CarryProvenance;
    use crate::tax::scrub_axis::maximal_sentinel;
    use std::collections::BTreeSet;
    use time::macros::date;

    /// What a [`LEAF_SOURCE`] audit found. Empty on every count = the guarantee holds.
    #[derive(Debug, Default, PartialEq, Eq)]
    struct Audit {
        /// Money leaves no prefix claims — *"a `Usd` field with no source"*.
        unsourced: Vec<String>,
        /// Prefixes that claim no money leaf — a stale entry, which over-claims silently.
        unmatched: Vec<String>,
        /// Money leaves claimed by more than one prefix — the table stops being a partition.
        multi: Vec<String>,
    }

    /// The audit, parameterised on the table so a KILL can hand it a MUTATED one and watch it red.
    fn audit(table: &[(&str, Source)], money: &BTreeSet<String>) -> Audit {
        let mut a = Audit::default();
        for leaf in money {
            let hits: Vec<&str> = table
                .iter()
                .filter(|(p, _)| prefix_matches(p, leaf))
                .map(|(p, _)| *p)
                .collect();
            match hits.len() {
                0 => a.unsourced.push(leaf.clone()),
                1 => {}
                _ => a.multi.push(leaf.clone()),
            }
        }
        for (p, _) in table {
            if !money.iter().any(|leaf| prefix_matches(p, leaf)) {
                a.unmatched.push((*p).to_string());
            }
        }
        a
    }

    /// ★★★ **THE LEAF_SOURCE KAT — BOTH DIRECTIONS** (§8: *"no `Money` field outside a document or
    /// filer's-records struct; every `Usd` leaf has one source"*).
    #[test]
    fn every_money_leaf_has_exactly_one_source_and_every_source_prefix_is_live() {
        let money = money_leaves(&maximal_sentinel());
        // ★ The floor is the MEASURED count, not a round number well below it (seam review N1). At
        //   `> 50` a fixture that had lost HALF its realized money leaves still passed the guard
        //   whose own message says *"it has stopped being maximal"* — a guard that cannot fire on the
        //   thing it names. 106 leaves measured at the time of writing; 100 leaves a little slack for
        //   a field legitimately retired without making the guard vacuous.
        assert!(
            money.len() >= 100,
            "the maximal fixture realized only {} money leaves — it has stopped being maximal, and a \
             shrunken fixture makes this KAT vacuous",
            money.len()
        );
        assert_eq!(audit(LEAF_SOURCE, &money), Audit::default());
    }

    /// ★ **The money DETECTOR, observed discriminating** (B1). A type-driven classifier can only be
    /// trusted once it has been watched saying yes to a `Usd`, yes to an `Option<Usd>`, and no to the
    /// `String` / `Date` / `bool` leaves sitting right beside them.
    #[test]
    fn the_money_detector_separates_usd_from_the_leaves_that_look_like_it() {
        let money = money_leaves(&maximal_sentinel());
        for yes in [
            "w2s[0].box1_wages",                  // a plain `Usd`
            "form_8960_line9b", // an `Option<Usd>` — realized because the fixture is maximal
            "schedule_a.medical", // nested
            "capital_loss_carryforward_in.short", // inside a frozen shared value type
        ] {
            assert!(
                money.contains(yes),
                "{yes} is a money leaf and was not detected"
            );
        }
        for no in [
            "w2s[0].employer",               // a String
            "int_1099[0].payer_tin",         // a String that CAN look numeric
            "header.taxpayer.ssn", // a String of digits — the trap a value-based detector falls in
            "header.taxpayer.date_of_birth", // an Option<Date>, serialized as an int array
            "foreign_accounts",    // an Option<bool>
            "itemize_election",    // a defaulted enum
        ] {
            assert!(
                !money.contains(no),
                "{no} is NOT a money leaf but was detected as one"
            );
        }
    }

    /// ★★★ **KILL — a money leaf with NO source reds** (B1: plant the exact defect).
    #[test]
    fn deleting_a_source_prefix_reds_the_kat() {
        let money = money_leaves(&maximal_sentinel());
        let gutted: Vec<(&str, Source)> = LEAF_SOURCE
            .iter()
            .copied()
            .filter(|(p, _)| *p != "w2s")
            .collect();
        let a = audit(&gutted, &money);
        assert!(
            a.unsourced.iter().any(|p| p.starts_with("w2s[")),
            "deleting the `w2s` prefix must leave every W-2 money box unsourced; got {a:?}"
        );
    }

    /// ★★★ **KILL — a STALE prefix reds** (the `coverage.rs` stale-exemption discipline). An entry
    /// that matches nothing silently over-claims: it looks like coverage and is not.
    #[test]
    fn a_prefix_that_matches_no_money_leaf_reds_the_kat() {
        let money = money_leaves(&maximal_sentinel());
        let mut padded: Vec<(&str, Source)> = LEAF_SOURCE.to_vec();
        padded.push(("a_leaf_that_does_not_exist", Source::Ledger));
        let a = audit(&padded, &money);
        assert_eq!(
            a.unmatched,
            vec!["a_leaf_that_does_not_exist".to_string()],
            "a prefix matching no money leaf must red"
        );
    }

    /// ★★★ **KILL — two prefixes claiming one leaf reds.** The table is a PARTITION, not a
    /// priority list: if it were read longest-prefix-first, a mis-scoped entry would silently
    /// re-attribute a figure's source and nothing would say so.
    #[test]
    fn two_prefixes_claiming_the_same_leaf_red_the_kat() {
        let money = money_leaves(&maximal_sentinel());
        let mut padded: Vec<(&str, Source)> = LEAF_SOURCE.to_vec();
        padded.push(("schedule_a.medical", Source::Ledger));
        let a = audit(&padded, &money);
        assert_eq!(
            a.multi,
            vec!["schedule_a.medical".to_string()],
            "a leaf claimed twice must red"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────────────────────
    // The answer log
    // ─────────────────────────────────────────────────────────────────────────────────────────────

    fn ri() -> ReturnInputs {
        ReturnInputs {
            tax_year: 2025,
            ..Default::default()
        }
    }
    const D1: Date = date!(2026 - 09 - 01);
    /// A LATER date, so a re-answer's record is distinguishable from the one it superseded.
    const D2: Date = date!(2026 - 11 - 14);

    /// The wire form round-trips for all three key shapes, and a stored blob deserializes back to the
    /// same map — the `BTreeMap` key contract.
    #[test]
    fn every_answer_key_shape_round_trips_through_its_string_wire_form() {
        let keys = [
            AnswerKey::Question(QuestionId::ForeignTrust),
            AnswerKey::Skippable(SkippableId::DobSpouse),
            AnswerKey::DependentGate {
                ssn_hash: dependent_ssn_hash("111-22-3333"),
                gate: DependentGate::GrossIncomeUnderLimit,
            },
        ];
        for k in &keys {
            assert_eq!(
                AnswerKey::from_str(&k.to_string()).as_ref(),
                Ok(k),
                "{k:?} did not round-trip through {:?}",
                k.to_string()
            );
        }
        let mut r = ri();
        for k in &keys {
            record_answer(&mut r, k.clone(), "a prompt", D1, AnswerState::Given);
        }
        let back: ReturnInputs = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.answer_log, r.answer_log);
    }

    /// ★★ Every gate identity is listed in `ALL` — the exhaustive `match` makes a new variant a
    /// compile error here, and the `ALL` scan is what the wire parser depends on.
    #[test]
    fn every_dependent_gate_is_in_all() {
        for (i, g) in DependentGate::ALL.iter().enumerate() {
            let idx = match g {
                DependentGate::QcRelationship => 0,
                DependentGate::ProvidedOverHalfOwnSupport => 1,
                DependentGate::FilingJointReturn => 2,
                DependentGate::JointReturnOnlyToClaimRefund => 3,
                DependentGate::QualifyingChildOfAnotherPerson => 4,
                DependentGate::CitizenNationalResidentOrCanadaMexico => 5,
                DependentGate::Married => 6,
                DependentGate::TinIssuedByDueDate => 7,
                DependentGate::CitizenNationalOrResidentAlien => 8,
                DependentGate::SsnsValidForEmploymentIssuedByDueDate => 9,
                DependentGate::QrRelationshipOrMemberOfHousehold => 10,
                DependentGate::QualifyingChildOfAnyTaxpayer => 11,
                DependentGate::GrossIncomeUnderLimit => 12,
                DependentGate::YouProvidedOverHalfSupport => 13,
                DependentGate::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies => 14,
                DependentGate::YoungerThanYouOrSpouse => 15,
                DependentGate::LivedWithYouOverHalfYear => 16,
                DependentGate::LivedWithYouInUs => 17,
                DependentGate::FullTimeStudent => 18,
                DependentGate::PermanentlyAndTotallyDisabled => 19,
                DependentGate::DateOfBirth => 20,
            };
            assert_eq!(idx, i, "DependentGate::ALL is out of order / missing {g:?}");
        }
        assert_eq!(
            DependentGate::ALL.len(),
            21,
            "§5.3's sixteen §152 gates + the four row-(5)/(6) facts + the required date of birth"
        );
    }

    /// ★★★ **`Declined` vs ABSENT.** The whole reason `AnswerState` exists: a skippable passed over
    /// on purpose is a record, and a question never put to the filer is no record at all. On the
    /// printed page both are the same blank.
    #[test]
    fn a_declined_skippable_is_a_record_and_an_untouched_one_is_not() {
        let mut r = ri();
        let declined = AnswerKey::Skippable(SkippableId::BlindTaxpayer);
        let untouched = AnswerKey::Skippable(SkippableId::BlindSpouse);
        record_answer(
            &mut r,
            declined.clone(),
            "prompt A",
            D1,
            AnswerState::Declined,
        );
        assert_eq!(
            answer_status(&r, &declined, "prompt A"),
            AnswerStatus::Declined
        );
        assert_eq!(
            answer_status(&r, &untouched, "prompt B"),
            AnswerStatus::NeverAsked
        );
        assert_ne!(
            answer_status(&r, &declined, "prompt A"),
            answer_status(&r, &untouched, "prompt B"),
            "declined and never-asked must not be the same state — they are the same BLANK"
        );
    }

    /// ★★★ **THE PROMPT-HASH MISMATCH RULE, in one test** (§8: *"an answer given under earlier words
    /// never stands under later ones"*). Change the words → unanswered; RE-ANSWER under the new words
    /// → the old record is in history and the fresh one is current; re-answer again under those same
    /// words → history does not grow.
    ///
    /// ★★★ **The second half runs through [`record_answer`] and NOTHING ELSE** (seam review I1). The
    ///     version of this kill that shipped called a `supersede_stale_prompts` sweep directly, and
    ///     the sweep had zero production callers — a function fully tested and never run, so R10.3's
    ///     *"the old answer is kept as history"* was unmet on every real path while this test was
    ///     green. Now the only thing touched here is the one writer both surfaces call, so a
    ///     regression cannot hide behind a test-only entry point.
    #[test]
    fn changing_the_words_re_asks_and_the_re_answer_itself_historises_the_old_record() {
        let mut r = ri();
        let k = AnswerKey::Question(QuestionId::ForeignTrust);
        record_answer(
            &mut r,
            k.clone(),
            "the ORIGINAL words",
            D1,
            AnswerState::Given,
        );
        assert!(
            r.answer_log_history.is_empty(),
            "a FIRST answer supersedes nothing"
        );

        // Same words: still an answer.
        assert_eq!(
            answer_status(&r, &k, "the ORIGINAL words"),
            AnswerStatus::Given
        );

        // Changed words: reported as unanswered, with the reason R12's panel prints — and the record
        // is still IN the log, which is what `screen_inputs` needs to refuse it as such.
        assert_eq!(
            answer_status(&r, &k, "the ORIGINAL words?"),
            AnswerStatus::WordingChanged,
            "one added character must be enough — {WORDING_CHANGED_REASON}"
        );
        assert!(
            r.answer_log.contains_key(&k),
            "a record refused for changed wording must STAY in the log until it is re-answered —              sweeping it out on load turns `WordingChanged` into `NeverAsked`, and an absent record              is not a mismatch"
        );

        // ── THE RE-ANSWER. No sweep is called; `record_answer` is the whole path.
        record_answer(
            &mut r,
            k.clone(),
            "the ORIGINAL words?",
            D2,
            AnswerState::Given,
        );
        assert_eq!(
            r.answer_log_history.len(),
            1,
            "the superseded record must be KEPT — history is where it goes, and nothing else wrote it"
        );
        assert_eq!(r.answer_log_history[0].0, k);
        assert_eq!(
            r.answer_log_history[0].1.prompt_hash,
            prompt_hash("the ORIGINAL words"),
            "history must hold the record that was superseded, not the one that replaced it"
        );
        assert_eq!(
            r.answer_log[&k].answered_on, D2,
            "…and the CURRENT record is the fresh one"
        );
        assert_eq!(
            answer_status(&r, &k, "the ORIGINAL words?"),
            AnswerStatus::Given
        );

        // ── Re-answering under the SAME words is a correction, not a supersession: history is flat.
        record_answer(
            &mut r,
            k.clone(),
            "the ORIGINAL words?",
            D2,
            AnswerState::Declined,
        );
        assert_eq!(
            r.answer_log_history.len(),
            1,
            "changing an answer under UNCHANGED wording must not grow the history — only the words \
             changing supersedes (R10.3), and a writer that appended every time would turn the \
             append-only file into a keystroke log"
        );
    }

    /// ★★★ **I10 — THE DILIGENCE KEY IS AN IDENTITY.** Answer gates on two dependent rows; delete
    /// row 0 and row 1's records are untouched while row 0's are gone; change row 1's `ssn` and its
    /// records move to history. *"A diligence record that lies is worse than none."*
    #[test]
    fn deleting_one_dependent_leaves_the_others_records_alone_and_a_new_ssn_starts_fresh() {
        let mut r = ri();
        let (ssn0, ssn1) = ("111-22-3333", "444-55-6666");
        let key = |ssn: &str, gate| AnswerKey::DependentGate {
            ssn_hash: dependent_ssn_hash(ssn),
            gate,
        };
        for ssn in [ssn0, ssn1] {
            for gate in [DependentGate::QcRelationship, DependentGate::Married] {
                record_answer(
                    &mut r,
                    key(ssn, gate),
                    "a gate prompt",
                    D1,
                    AnswerState::Given,
                );
            }
        }
        assert_eq!(r.answer_log.len(), 4);

        // Row 0 removed.
        assert_eq!(retire_dependent_identity(&mut r, ssn0), 2);
        assert_eq!(r.answer_log.len(), 2);
        for gate in [DependentGate::QcRelationship, DependentGate::Married] {
            assert!(
                r.answer_log.contains_key(&key(ssn1, gate)),
                "row 1's records must be untouched by row 0's removal"
            );
            assert!(!r.answer_log.contains_key(&key(ssn0, gate)));
        }
        assert!(
            r.answer_log_history.is_empty(),
            "a removed row is a WITHDRAWN row — there is no superseded answer to keep"
        );

        // Row 1's SSN corrected: a different person, so the old records become history.
        assert_eq!(supersede_dependent_identity(&mut r, ssn1, "777-88-9999"), 2);
        assert!(r.answer_log.is_empty());
        assert_eq!(r.answer_log_history.len(), 2);
        assert!(r.answer_log_history.iter().all(
            |(k, _)| matches!(k, AnswerKey::DependentGate { ssn_hash, .. }
                                   if *ssn_hash == dependent_ssn_hash(ssn1))
        ));
    }

    /// ★ Punctuation is not identity: `111-22-3333` and `111223333` are the same child, so re-typing
    /// the dashes must not move a single record.
    #[test]
    fn dashes_in_an_ssn_do_not_change_the_identity() {
        assert_eq!(
            dependent_ssn_hash("111-22-3333"),
            dependent_ssn_hash("111223333")
        );
        let mut r = ri();
        record_answer(
            &mut r,
            AnswerKey::DependentGate {
                ssn_hash: dependent_ssn_hash("111223333"),
                gate: DependentGate::Married,
            },
            "p",
            D1,
            AnswerState::Given,
        );
        assert_eq!(
            supersede_dependent_identity(&mut r, "111223333", "111-22-3333"),
            0
        );
        assert_eq!(r.answer_log.len(), 1);
        assert!(r.answer_log_history.is_empty());
    }

    /// ★ The hash never carries the digits — the *"never the digits"* half of R10.3.
    #[test]
    fn the_identity_key_never_contains_the_ssn() {
        let h = dependent_ssn_hash("111-22-3333");
        assert!(!h.contains("111"), "the hash leaked a digit run: {h}");
        assert!(!h.contains("223333"));
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// ★ Un-answering returns a question to NEVER ASKED, and writes no history: a clear is neither of
    /// the two things §5.6 says history holds.
    #[test]
    fn forgetting_an_answer_leaves_no_record_and_no_history() {
        let mut r = ri();
        let k = AnswerKey::Question(QuestionId::ForeignTrust);
        record_answer(&mut r, k.clone(), "p", D1, AnswerState::Given);
        forget_answer(&mut r, &k);
        assert_eq!(answer_status(&r, &k, "p"), AnswerStatus::NeverAsked);
        assert!(r.answer_log_history.is_empty());
    }

    /// ★★ **The FORBIDDEN shapes** (R10.3 / `FIELD_PROVENANCE.md:400-403`): no progress, no position,
    /// no "what remains" on `ReturnInputs`. A grep-KAT over the serialized leaf names, so it bites on
    /// a field added anywhere in the reachable tree rather than only at the top level.
    #[test]
    fn no_progress_or_position_field_exists_on_return_inputs() {
        let mut leaves = Vec::new();
        walk(
            &serde_json::to_value(maximal_sentinel()).unwrap(),
            "",
            &mut leaves,
        );
        for leaf in &leaves {
            let name = leaf.rsplit('.').next().unwrap_or(leaf);
            for banned in ["progress", "remaining", "position"] {
                assert!(
                    !name.contains(banned),
                    "`{leaf}` names a forbidden shape ({banned}): the answer log records WHAT WAS \
                     ASKED, never how far through the interview the filer is"
                );
            }
        }
    }

    /// ★ R10.4 — the year-N+1 carry provenance is a THIRD state, distinct from both existing ones,
    /// and it round-trips (task T4b writes it; T1 owns the schema, which cannot be back-filled).
    #[test]
    fn computed_from_prior_return_is_its_own_provenance_and_round_trips() {
        let p = CarryProvenance::ComputedFromPriorReturn { year: 2026 };
        assert_ne!(p, CarryProvenance::Computed);
        assert_ne!(p, CarryProvenance::User);
        assert_eq!(CarryProvenance::default(), CarryProvenance::User);
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<CarryProvenance>(&j).unwrap(), p);
    }
}
