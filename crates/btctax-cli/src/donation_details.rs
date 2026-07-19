//! `donation_details` side-table — persists Form 8283 Section-B appraiser + structured-donee
//! metadata for each donation event. Keyed by the donation's `EventId::canonical()` string.
//! Modeled on `optimize_attest.rs` discipline (idempotent DDL, defensive `init_table` guard on
//! every accessor, JSON storage via serde_json — same as `tax_profile.rs`). The data is
//! POST-HOC form-completion metadata; it does NOT enter the fold or projection.
use crate::CliError;
use btctax_core::{DonationDetails, EventId};
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

/// Create the `donation_details` side-table if it does not exist (idempotent).
///
/// `CREATE TABLE IF NOT EXISTS` makes this safe to call on any vault — new, old, or restored
/// from snapshot — without errors. Called by `Session::from_fresh_vault`; also called at the
/// top of every `get`/`set`/`all` as a defensive ensure-table-then-read guard (robust to older
/// vaults that predate Chunk 3b, mirroring the `optimize_attest` back-compat pattern).
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS donation_details \
         (donation_event TEXT PRIMARY KEY, details_json TEXT NOT NULL);",
    )?;
    Ok(())
}

/// Return the stored `DonationDetails` for `event`, or `None` if none has been set.
///
/// Robust to older vaults (calls `init_table` first so "no such table" is never returned).
/// Returns `CliError::BadConfigValue` if the stored JSON is malformed.
pub fn get(conn: &Connection, event: &EventId) -> Result<Option<DonationDetails>, CliError> {
    init_table(conn)?;
    let json: Option<String> = conn
        .query_row(
            "SELECT details_json FROM donation_details WHERE donation_event=?1",
            [event.canonical()],
            |r| r.get(0),
        )
        .optional()?;
    match json {
        None => Ok(None),
        Some(j) => Ok(Some(serde_json::from_str(&j).map_err(|e| {
            CliError::BadConfigValue {
                key: format!("donation_details[{}]", event.canonical()),
                value: format!("invalid JSON: {e}"),
            }
        })?)),
    }
}

/// True iff every byte of `b` is an ASCII digit (and `b` is non-empty).
fn all_digits(b: &[u8]) -> bool {
    !b.is_empty() && b.iter().all(u8::is_ascii_digit)
}
/// EIN shape `##-#######` (2 digits, hyphen, 7 digits).
fn is_ein_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[2] == b'-' && all_digits(&b[..2]) && all_digits(&b[3..])
}
/// SSN/ITIN shape `###-##-####` (3-2-4). An ITIN (`9##-##-####`) is a subset — accepted here.
fn is_ssn_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 11
        && b[3] == b'-'
        && b[6] == b'-'
        && all_digits(&b[..3])
        && all_digits(&b[4..6])
        && all_digits(&b[7..])
}
/// Exactly 9 digits, unhyphenated.
fn is_bare9(s: &str) -> bool {
    s.len() == 9 && all_digits(s.as_bytes())
}
/// PTIN shape `P########` (a `P` then exactly 8 digits).
fn is_ptin_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 9 && b[0] == b'P' && all_digits(&b[1..])
}

/// UX-P4-4(c): validate + normalize the Form 8283 Section-B taxpayer identifiers, returning a
/// normalized copy. Called from `set` — the single side-table writer that BOTH the CLI
/// (`reconcile::set_donation_details`) and the TUI (`persist_donation_details`) converge on — so a
/// bad identifier can never reach a filed Form 8283 from either surface. Purely a shape check: it
/// cannot verify a TIN is real, only that it is well-formed enough to belong on the form.
///
/// - `appraiser_ptin`: `P########` (§6695A PTIN) or refuse.
/// - `appraiser_tin`: an EIN, an SSN/ITIN, or the same 9 digits unformatted (a TIN is any of
///   SSN/ITIN/ATIN/**EIN**, 26 CFR 301.6109-1(a)(1)(i)). A masked/short value is refused.
/// - `donee_ein`: an organization EIN — a bare 9-digit is normalized into the hyphenated form; an
///   individual **SSN-shape is refused** (an individual is not a §170(c) donee). The bare-9 case
///   necessarily also admits an unhyphenated SSN — inherent and ACCEPTED (refusing it would refuse
///   real unformatted EINs); this is deliberately not "hardened" into a false refuse.
pub(crate) fn validate_and_normalize(
    details: &DonationDetails,
) -> Result<DonationDetails, CliError> {
    let mut d = details.clone();

    if let Some(p) = d.appraiser_ptin.as_deref() {
        if !is_ptin_shape(p) {
            return Err(CliError::Usage(format!(
                "--appraiser-ptin must be a PTIN of the form P######## (P then 8 digits); got {p:?}"
            )));
        }
    }

    if let Some(t) = d.appraiser_tin.as_deref() {
        if !(is_ein_shape(t) || is_ssn_shape(t) || is_bare9(t)) {
            return Err(CliError::Usage(format!(
                "--appraiser-tin must be an EIN (12-3456789), an SSN/ITIN (123-45-6789), or 9 \
                 digits; got {t:?}"
            )));
        }
    }

    if let Some(e) = d.donee_ein.as_deref() {
        if is_ssn_shape(e) {
            return Err(CliError::Usage(format!(
                "--donee-ein must be an organization EIN (12-3456789), not an individual SSN \
                 ({e:?}); omit --donee-ein if the donee has none"
            )));
        } else if is_ein_shape(e) {
            // already canonical — keep as-is
        } else if is_bare9(e) {
            d.donee_ein = Some(format!("{}-{}", &e[..2], &e[2..]));
        } else {
            return Err(CliError::Usage(format!(
                "--donee-ein must be an EIN (12-3456789) or 9 digits; got {e:?}; omit --donee-ein \
                 if the donee has none"
            )));
        }
    }

    Ok(d)
}

/// Persist `details` for `event` (upsert — replaces any prior value; last-write-wins).
///
/// Idempotent DDL guard first. JSON via serde_json (mirrors `tax_profile::set`).
pub fn set(conn: &Connection, event: &EventId, details: &DonationDetails) -> Result<(), CliError> {
    init_table(conn)?;
    let details = validate_and_normalize(details)?;
    let j = serde_json::to_string(&details).map_err(|e| CliError::BadConfigValue {
        key: format!("donation_details[{}]", event.canonical()),
        value: e.to_string(),
    })?;
    conn.execute(
        "INSERT INTO donation_details(donation_event,details_json) VALUES(?1,?2) \
         ON CONFLICT(donation_event) DO UPDATE SET details_json=excluded.details_json",
        rusqlite::params![event.canonical(), j],
    )?;
    Ok(())
}

/// Return all stored `DonationDetails`, keyed by the donation `EventId` (NFR4-stable
/// `BTreeMap` — deterministic iteration order). Defensive `init_table` guard first.
pub fn all(conn: &Connection) -> Result<BTreeMap<EventId, DonationDetails>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare(
        "SELECT donation_event, details_json \
         FROM donation_details ORDER BY donation_event",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (ev_str, j) = row?;
        let eid = crate::eventref::parse_event_id(&ev_str)?;
        let details: DonationDetails =
            serde_json::from_str(&j).map_err(|e| CliError::BadConfigValue {
                key: format!("donation_details[{ev_str}]"),
                value: format!("invalid JSON: {e}"),
            })?;
        out.insert(eid, details);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{EventId, Source, SourceRef};
    use time::macros::date;

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_table(&c).unwrap();
        c
    }

    fn eid(label: &str) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(label.to_string()))
    }

    /// Full details — all optional fields populated. PRIVACY: no real PII.
    fn full_details() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: Some("123 Main St, Anytown USA".into()),
            donee_ein: Some("12-3456789".into()),
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: Some("456 Appraiser Ave".into()),
            appraiser_tin: Some("987654321".into()),
            appraiser_ptin: Some("P01234567".into()),
            appraiser_qualifications: Some("Certified bitcoin property appraiser".into()),
            appraisal_date: Some(date!(2025 - 06 - 01)),
            fmv_method_override: Some("qualified appraisal".into()),
        }
    }

    /// Minimal details — only required fields; all optionals are None.
    fn minimal_details() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: None,
            donee_ein: None,
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: None,
            appraiser_ptin: None,
            appraiser_qualifications: None,
            appraisal_date: None,
            fmv_method_override: None,
        }
    }

    /// Full round-trip: all optional fields populated (incl. appraisal_date) survive JSON.
    #[test]
    fn set_then_get_round_trips_full_details() {
        let c = mem();
        let e = eid("out|test-donation-full");
        set(&c, &e, &full_details()).unwrap();
        let stored = get(&c, &e).unwrap().unwrap();
        assert_eq!(stored, full_details());
    }

    /// Minimal round-trip: all optional fields are None (serde default).
    #[test]
    fn set_then_get_round_trips_minimal_details() {
        let c = mem();
        let e = eid("out|test-donation-minimal");
        set(&c, &e, &minimal_details()).unwrap();
        let stored = get(&c, &e).unwrap().unwrap();
        assert_eq!(stored, minimal_details());
    }

    /// UX-P4-4(c): a `DonationDetails` carrying only the three identifiers under test.
    fn details_with(
        donee_ein: Option<&str>,
        appraiser_tin: Option<&str>,
        appraiser_ptin: Option<&str>,
    ) -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: None,
            donee_ein: donee_ein.map(str::to_owned),
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: appraiser_tin.map(str::to_owned),
            appraiser_ptin: appraiser_ptin.map(str::to_owned),
            appraiser_qualifications: None,
            appraisal_date: None,
            fmv_method_override: None,
        }
    }

    /// A TIN is an EIN, an SSN/ITIN, or those 9 digits unformatted (26 CFR 301.6109-1(a)(1)(i));
    /// a masked or wrong-length value is refused.
    #[test]
    fn appraiser_tin_accepts_ein_ssn_bare9_and_refuses_masked() {
        // `987-65-4321` is a 9xx-prefixed ITIN-shape too — the ITIN case is SSN-shaped, so it is
        // covered here without adding a second PII-shaped literal (pii-scan allowlist).
        for good in ["12-3456789", "123-45-6789", "987-65-4321", "987654321"] {
            assert!(
                validate_and_normalize(&details_with(None, Some(good), None)).is_ok(),
                "appraiser-tin {good:?} must be accepted"
            );
        }
        // ITIN (9xx-xx-xxxx) is SSN-shaped — accepted above. Masked / wrong-length refused:
        let err =
            validate_and_normalize(&details_with(None, Some("***-**-1234"), None)).unwrap_err();
        assert!(
            format!("{err:?}").contains("--appraiser-tin"),
            "masked appraiser-tin refusal must name the flag: {err:?}"
        );
        assert!(validate_and_normalize(&details_with(None, Some("12345"), None)).is_err());
        assert!(validate_and_normalize(&details_with(None, Some("1234567890"), None)).is_err());
    }

    /// A donee EIN is an organization EIN (§170(c)) — NOT an individual SSN. A bare 9-digit is
    /// normalized into the hyphenated EIN form; an SSN-shape or garbage is refused.
    #[test]
    fn donee_ein_normalizes_bare9_refuses_ssn_shape_and_garbage() {
        let ok = validate_and_normalize(&details_with(Some("12-3456789"), None, None)).unwrap();
        assert_eq!(
            ok.donee_ein.as_deref(),
            Some("12-3456789"),
            "canonical EIN unchanged"
        );

        let norm = validate_and_normalize(&details_with(Some("123456789"), None, None)).unwrap();
        assert_eq!(
            norm.donee_ein.as_deref(),
            Some("12-3456789"),
            "a bare 9-digit donee EIN normalizes to hyphenated form"
        );

        let err =
            validate_and_normalize(&details_with(Some("123-45-6789"), None, None)).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("--donee-ein") && msg.contains("SSN") && msg.contains("omit"),
            "an SSN-shaped donee EIN refusal must name the flag, SSN, and the omit hint: {msg}"
        );

        assert!(validate_and_normalize(&details_with(Some("banana"), None, None)).is_err());
    }

    /// A PTIN is `P` followed by exactly 8 digits (Form 8283 Part III §6695A).
    #[test]
    fn appraiser_ptin_requires_p_plus_8_digits() {
        assert!(validate_and_normalize(&details_with(None, None, Some("P01234567"))).is_ok());
        assert!(validate_and_normalize(&details_with(None, None, Some("012345678"))).is_err());
        assert!(validate_and_normalize(&details_with(None, None, Some("P0123456"))).is_err());
        let err = validate_and_normalize(&details_with(None, None, Some("nope"))).unwrap_err();
        assert!(
            format!("{err:?}").contains("--appraiser-ptin"),
            "ptin refusal must name the flag: {err:?}"
        );
    }

    /// `set` IS the choke point: normalization flows through it, and a bad identifier is refused
    /// AT `set` — so BOTH the CLI (`reconcile::set_donation_details`) and the TUI
    /// (`persist_donation_details`) writers, which both call `set`, are covered. Fail-closed.
    #[test]
    fn set_choke_point_normalizes_and_refuses_fail_closed() {
        let c = mem();
        let e1 = eid("out|dd-normalize");
        set(
            &c,
            &e1,
            &details_with(Some("123456789"), Some("987654321"), None),
        )
        .unwrap();
        assert_eq!(
            get(&c, &e1).unwrap().unwrap().donee_ein.as_deref(),
            Some("12-3456789"),
            "bare-9 donee EIN must be stored normalized"
        );

        let e2 = eid("out|dd-bad");
        assert!(
            set(&c, &e2, &details_with(None, Some("***-**-1234"), None)).is_err(),
            "a masked appraiser-tin must be refused at set()"
        );
        assert!(
            get(&c, &e2).unwrap().is_none(),
            "a refused write must store nothing (fail-closed)"
        );
    }

    /// A missing key returns None (not an error).
    #[test]
    fn get_missing_returns_none() {
        let c = mem();
        assert_eq!(get(&c, &eid("out|no-such-event")).unwrap(), None);
    }

    /// Back-compat: a vault that has NO donation_details table (an "old" vault) → `get` returns
    /// None (the defensive `init_table` guard creates the table transparently on first access).
    #[test]
    fn get_on_tableless_vault_returns_none() {
        // No init_table call — simulates an old vault opened for the first time post-Chunk-3b.
        let c = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(get(&c, &eid("out|test")).unwrap(), None);
    }

    /// Back-compat: `all()` on a tableless vault returns an empty map (not an error).
    #[test]
    fn all_on_tableless_vault_returns_empty_map() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        assert!(all(&c).unwrap().is_empty());
    }

    /// `init_table` is idempotent — calling it multiple times must not error.
    #[test]
    fn init_table_is_idempotent() {
        let c = mem(); // already called init_table
        init_table(&c).unwrap();
        init_table(&c).unwrap();
        assert!(all(&c).unwrap().is_empty());
    }

    /// Upsert: a second `set` for the same key replaces the prior value.
    #[test]
    fn upsert_replaces_prior_details() {
        let c = mem();
        let e = eid("out|donation-upsert");
        set(&c, &e, &minimal_details()).unwrap();
        set(&c, &e, &full_details()).unwrap();
        let stored = get(&c, &e).unwrap().unwrap();
        assert_eq!(stored, full_details());
    }

    /// Defensive-guard path: no explicit `init_table` called, but `set` + `get` still work.
    #[test]
    fn defensive_guard_in_set_creates_table() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        let e = eid("out|guard-test");
        // No init_table — the defensive guard inside `set` must create the table.
        set(&c, &e, &minimal_details()).unwrap();
        assert_eq!(get(&c, &e).unwrap().unwrap(), minimal_details());
    }

    /// `all()` returns a `BTreeMap` ordered by EventId (NFR4 determinism).
    #[test]
    fn all_returns_btreemap_in_deterministic_order() {
        let c = mem();
        let e1 = eid("out|donation-alpha");
        let e2 = eid("out|donation-beta");
        // Insert in reverse order; BTreeMap must impose deterministic order.
        set(&c, &e2, &full_details()).unwrap();
        set(&c, &e1, &minimal_details()).unwrap();
        let m = all(&c).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&e1).unwrap(), &minimal_details());
        assert_eq!(m.get(&e2).unwrap(), &full_details());
    }
}
