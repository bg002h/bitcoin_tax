//! Stable identity & canonical ordering (§6.2). EventId is a STRUCTURED (injective) function of its
//! components — no hashing needed for identity; only the content *fingerprint* (conflict detection) hashes.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The four supported venues. Fixed source priority for same-instant fold ties (§6.2): Swan>Coinbase>Gemini>River.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Source {
    Swan,
    Coinbase,
    Gemini,
    River,
}
impl Source {
    /// Lower = folds first at the same `utc_timestamp` (Swan=0 … River=3).
    pub fn priority(self) -> u8 {
        match self {
            Source::Swan => 0,
            Source::Coinbase => 1,
            Source::Gemini => 2,
            Source::River => 3,
        }
    }
    pub fn tag(self) -> &'static str {
        match self {
            Source::Swan => "swan",
            Source::Coinbase => "coinbase",
            Source::Gemini => "gemini",
            Source::River => "river",
        }
    }
}

/// Stable real-world-row identity scoped by (source, direction) (§6.2). Opaque string assigned by adapters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRef(pub String);
impl SourceRef {
    pub fn new(s: impl Into<String>) -> Self {
        SourceRef(s.into())
    }
}

/// SHA-256 hex of canonical content; used ONLY for conflict detection (§6.2/§9).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fingerprint(pub String);
impl Fingerprint {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(bytes);
        Fingerprint(format!("{:x}", h.finalize()))
    }
}

/// Universal reference target (§6.2). Equality is what matters; we also derive Ord/Hash for map keys + stable output.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventId {
    /// Imported events: f(source, source_ref) — survives cosmetic re-exports/corrections.
    Import {
        source: Source,
        source_ref: SourceRef,
    },
    /// System ImportConflict: f("conflict", source, source_ref, new_fingerprint) — distinct from its target.
    Conflict {
        source: Source,
        source_ref: SourceRef,
        fingerprint: Fingerprint,
    },
    /// App-generated decisions: f("decision", decision_seq).
    Decision { seq: u64 },
}
impl EventId {
    pub fn import(source: Source, source_ref: SourceRef) -> Self {
        EventId::Import { source, source_ref }
    }
    pub fn conflict(source: Source, source_ref: SourceRef, fingerprint: &Fingerprint) -> Self {
        EventId::Conflict {
            source,
            source_ref,
            fingerprint: fingerprint.clone(),
        }
    }
    pub fn decision(seq: u64) -> Self {
        EventId::Decision { seq }
    }
    /// Stable string form for the persistence `event_id` column (components are also stored separately).
    pub fn canonical(&self) -> String {
        match self {
            EventId::Import { source, source_ref } => {
                format!("import|{}|{}", source.tag(), source_ref.0)
            }
            EventId::Conflict {
                source,
                source_ref,
                fingerprint,
            } => {
                format!(
                    "conflict|{}|{}|{}",
                    source.tag(),
                    source_ref.0,
                    fingerprint.0
                )
            }
            EventId::Decision { seq } => format!("decision|{seq}"),
        }
    }
}

/// Basis pool identity (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WalletId {
    Exchange { provider: String, account: String },
    SelfCustody { label: String },
}

/// Lot identity (§6.2): origin event + a per-origin split sequence, assigned deterministically as lots split.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LotId {
    pub origin_event_id: EventId,
    pub split_sequence: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_priority_is_swan_first_river_last() {
        let mut v = [
            Source::River,
            Source::Coinbase,
            Source::Swan,
            Source::Gemini,
        ];
        v.sort_by_key(|s| s.priority());
        assert_eq!(
            v,
            [
                Source::Swan,
                Source::Coinbase,
                Source::Gemini,
                Source::River
            ]
        );
    }

    #[test]
    fn import_event_id_is_stable_function_of_source_and_ref() {
        let a = EventId::import(Source::Coinbase, SourceRef::new("ID-1"));
        let b = EventId::import(Source::Coinbase, SourceRef::new("ID-1"));
        assert_eq!(a, b);
        assert_eq!(a.canonical(), "import|coinbase|ID-1");
    }

    #[test]
    fn conflict_event_id_is_distinct_from_its_target() {
        let target = EventId::import(Source::Gemini, SourceRef::new("T1"));
        let fp = Fingerprint::of_bytes(b"new-content");
        let c1 = EventId::conflict(Source::Gemini, SourceRef::new("T1"), &fp);
        let c2 = EventId::conflict(Source::Gemini, SourceRef::new("T1"), &fp);
        assert_ne!(EventId::import(Source::Gemini, SourceRef::new("T1")), c1);
        assert_eq!(c1, c2); // re-importing the identical changed row reproduces the same conflict id (§6.2)
        let _ = target;
    }

    #[test]
    fn decision_event_id_is_function_of_seq() {
        assert_eq!(EventId::decision(7).canonical(), "decision|7");
        assert_ne!(EventId::decision(7), EventId::decision(8));
    }

    #[test]
    fn lot_id_is_origin_plus_split_sequence() {
        let origin = EventId::import(Source::Swan, SourceRef::new("TX9"));
        let l0 = LotId {
            origin_event_id: origin.clone(),
            split_sequence: 0,
        };
        let l1 = LotId {
            origin_event_id: origin,
            split_sequence: 1,
        };
        assert_ne!(l0, l1);
        assert!(l0 < l1); // deterministic ordering for stable output
    }
}
