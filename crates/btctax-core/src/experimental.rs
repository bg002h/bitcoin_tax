//! The Approach-B ("Defensive Filing" — declare/promote tranche, Form 8275, estimated basis)
//! experimental notice: a single, presentation-neutral source of truth for telling a filer that this
//! part of btctax is newer and less proven than the rest of the tool, was developed with heavy AI
//! assistance, and has already shipped two defects that affected FILED output (both since fixed).
//!
//! ★ THE HARD CONSTRAINT: this notice is an INTERFACE-ONLY disclosure. It must **never** reach anything
//! the export machinery produces — not a Form 8275 field, not the Part II narrative, not
//! `form_8275.txt`, not `basis_methodology.txt`, not any PDF, not even as a separate sibling file in an
//! export directory. The export directory is what a filer mails or hands to a preparer; a file in it
//! saying the feature is AI-developed and has shipped defects is the SAME hazard as printing it on the
//! 8275 itself — it would undermine the very disclosure the packet exists to make credible. It appears
//! on exactly three surfaces: CLI stderr (never stdout — stdout is parsed/piped), the TUI banner rows
//! (`btctax-tui`, `btctax-tui-edit`), and the repo-root `NOTICE` (a project document, not a filed one).
//! Nothing in `crates/btctax-forms/` (`Printed8275`/`Disclosure8275`) and no export/write path anywhere
//! may ever reference this module's text.
//!
//! [`ExperimentalNotice`] is presentation-neutral and structured (title/summary/defects/action) so a
//! future web front-end can render it in its own idiom without string-munging; [`ExperimentalNotice::plain_text`]
//! is the terminal rendering the CLI and TUI front-ends share. No ANSI escapes, no assumed line width,
//! no terminal-specific formatting live in the data — only in each front-end's OWN rendering.
//!
//! [`uses_approach_b`] is the single gate every surface consults: true iff a live (non-voided)
//! `DeclareTranche` or `PromoteTranche` decision is on file. It reuses [`crate::tranche_guard::void_targets`]
//! — the same shared "which decision ids are voided" scan `tranche_guard::pre2025_tranche_exists` /
//! `in_force_allocation_exists` already use — rather than re-deriving liveness. A ledger where every
//! `DeclareTranche`/`PromoteTranche` has been voided returns `false`: showing the notice to a filer who
//! voided everything would be exactly the wrong answer.

use crate::event::EventPayload;
use crate::LedgerEvent;

/// A presentation-neutral, structured disclosure. No ANSI escapes, no assumed line width, no
/// terminal-specific formatting — CLI, TUI, and a future web UI all render this in their own idiom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentalNotice {
    /// One-line identification of the feature this notice is about.
    pub title: &'static str,
    /// The maturity + provenance disclosure: newer/less-proven, heavy AI assistance, both shipped
    /// defects fixed.
    pub summary: &'static str,
    /// The two shipped defects that affected FILED output, each a standalone sentence fragment.
    pub defects: &'static [&'static str],
    /// The filer-facing call to action.
    pub action: &'static str,
}

/// The single source of truth for the Approach-B experimental notice. Every fact in the design text is
/// preserved: newer/less-proven, heavy AI assistance, the two shipped defects (the Form 8275 Part II
/// narrative silently truncated to ~137 characters; the editor's residue-latch guarantee bypassable),
/// both now fixed, and the "check every figure" action.
pub const NOTICE: ExperimentalNotice = ExperimentalNotice {
    title: "EXPERIMENTAL — DEFENSIVE FILING (declare/promote tranche, Form 8275, estimated basis)",
    summary: "This feature is newer and less proven than the rest of btctax, and it was developed with \
        heavy AI assistance. Two defects that affect what gets FILED shipped and were found only by \
        later review. Both are fixed.",
    defects: &[
        "the Form 8275 disclosure was silently truncated to its first ~137 characters",
        "an editor guarantee (\"no in-editor action will save until you quit\") could be bypassed",
    ],
    action: "Their existence is the point: check every figure and every disclosure this feature \
        produces against your own records before you file.",
};

impl ExperimentalNotice {
    /// Render as plain text for a terminal front-end (CLI stderr, the TUI export-directory file). No
    /// ANSI, no line wrapping — callers wrap/style in their own idiom. Ends with a single trailing
    /// newline (matches the shipped `Disclosure8275::render()` / `basis_methodology` convention of a
    /// write-don't-writeln body).
    pub fn plain_text(&self) -> String {
        let mut s = String::new();
        s.push_str(self.title);
        s.push_str("\n\n");
        s.push_str(self.summary);
        s.push('\n');
        for d in self.defects {
            s.push_str("  - ");
            s.push_str(d);
            s.push('\n');
        }
        s.push('\n');
        s.push_str(self.action);
        s.push('\n');
        s
    }
}

/// True iff a live (non-voided) `DeclareTranche` or `PromoteTranche` decision exists in `events` — the
/// gate every Approach-B experimental-notice surface (CLI stderr, the TUI banner rows) consults.
///
/// "Live" = the event's OWN id is not targeted by any `VoidDecisionEvent` — the same record-time scan
/// `tranche_guard::pre2025_tranche_exists`/`in_force_allocation_exists` already use (deliberately NOT the
/// fuller `conservative.rs::live_declare_ids`, which additionally re-admits a `DeclareTranche` whose void
/// the engine held inert via `state.promoted_origins` — unneeded here: a `DeclareTranche` held in force by
/// exactly that mechanism always has a live, non-voided `PromoteTranche` targeting it, and THAT event
/// alone already satisfies this predicate via the `PromoteTranche` arm below).
///
/// A ledger where every `DeclareTranche`/`PromoteTranche` has been voided returns `false` — getting this
/// wrong is how the notice would show to a filer who voided everything.
pub fn uses_approach_b(events: &[LedgerEvent]) -> bool {
    let voided = crate::tranche_guard::void_targets(events);
    events.iter().any(|e| {
        !voided.contains(&e.id)
            && matches!(
                e.payload,
                EventPayload::DeclareTranche(_) | EventPayload::PromoteTranche(_)
            )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        Acknowledgment, DeclareTranche, FloorMethod, PromoteTranche, VoidDecisionEvent,
    };
    use crate::identity::{EventId, WalletId};
    use rust_decimal_macros::dec;
    use time::macros::{date, datetime, offset};

    fn wallet() -> WalletId {
        WalletId::SelfCustody {
            label: "cold".into(),
        }
    }

    fn dec_ev(seq: u64, payload: EventPayload) -> LedgerEvent {
        LedgerEvent {
            id: EventId::decision(seq),
            utc_timestamp: datetime!(2026-01-01 00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: None,
            payload,
        }
    }

    fn tranche_ev(seq: u64) -> LedgerEvent {
        dec_ev(
            seq,
            EventPayload::DeclareTranche(DeclareTranche {
                sat: 100_000_000,
                wallet: wallet(),
                window_start: date!(2018 - 01 - 01),
                window_end: date!(2018 - 12 - 31),
            }),
        )
    }

    fn promote_ev(seq: u64, target: EventId) -> LedgerEvent {
        dec_ev(
            seq,
            EventPayload::PromoteTranche(PromoteTranche {
                target,
                method: FloorMethod::WindowLowClose,
                filed_basis: dec!(1000),
                coverage: crate::conservative::Coverage::Full,
                provenance_attested: true,
                acknowledgment: Acknowledgment {
                    phrase: "ack".into(),
                    shown_terms: vec![],
                    provenance_text: "provenance".into(),
                    provenance_version: "v1".into(),
                },
                part_ii_narrative: "narrative".into(),
            }),
        )
    }

    fn void_ev(seq: u64, target: EventId) -> LedgerEvent {
        dec_ev(
            seq,
            EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                target_event_id: target,
            }),
        )
    }

    /// No tranche/promote event at all ⇒ Approach-B is not in use.
    #[test]
    fn no_tranche_is_false() {
        let evs: Vec<LedgerEvent> = vec![];
        assert!(!uses_approach_b(&evs));
    }

    /// A live (non-voided) `DeclareTranche` ⇒ true.
    #[test]
    fn live_declare_tranche_is_true() {
        let evs = vec![tranche_ev(1)];
        assert!(uses_approach_b(&evs));
    }

    /// A live (non-voided) `PromoteTranche` ⇒ true (even scanning promote alone, independent of its
    /// target's own void status — the OR over both payload types is deliberate, see module docs).
    #[test]
    fn live_promote_tranche_is_true() {
        let target = EventId::decision(1);
        let evs = vec![tranche_ev(1), promote_ev(2, target)];
        assert!(uses_approach_b(&evs));
    }

    /// A `DeclareTranche` voided (and never promoted) ⇒ false. The load-bearing "don't show it to
    /// someone who voided everything" case.
    #[test]
    fn voided_only_tranche_is_false() {
        let tranche_id = EventId::decision(1);
        let evs = vec![tranche_ev(1), void_ev(2, tranche_id)];
        assert!(!uses_approach_b(&evs));
    }

    /// A `DeclareTranche` voided AND its `PromoteTranche` also voided ⇒ false — the whole Approach-B
    /// position was unwound.
    #[test]
    fn voided_declare_and_voided_promote_is_false() {
        let tranche_id = EventId::decision(1);
        let promote_id = EventId::decision(2);
        let evs = vec![
            tranche_ev(1),
            promote_ev(2, tranche_id.clone()),
            void_ev(3, tranche_id),
            void_ev(4, promote_id),
        ];
        assert!(!uses_approach_b(&evs));
    }

    /// A `DeclareTranche` voided but its `PromoteTranche` is STILL LIVE (the BG-D9 "void of a
    /// promoted-target tranche is inert" corner, `void.rs::promoted_target`) ⇒ still true — the promote
    /// event itself is unvoided and satisfies the predicate on its own, so this never depends on
    /// `state.promoted_origins`.
    #[test]
    fn declare_voided_but_promote_still_live_is_true() {
        let tranche_id = EventId::decision(1);
        let evs = vec![
            tranche_ev(1),
            promote_ev(2, tranche_id.clone()),
            void_ev(3, tranche_id), // engine treats this as inert (a live promote holds it), but the
                                    // record-time scan here doesn't need to know that — the promote
                                    // event alone already answers true.
        ];
        assert!(uses_approach_b(&evs));
    }

    /// Non-Approach-B decision/import events never trip the predicate.
    #[test]
    fn unrelated_events_are_false() {
        use crate::event::Acquire;
        use crate::identity::{Source, SourceRef};
        use crate::BasisSource;
        let evs = vec![LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("x")),
            utc_timestamp: datetime!(2026-01-01 00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(wallet()),
            payload: EventPayload::Acquire(Acquire {
                sat: 1,
                usd_cost: dec!(1),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        }];
        assert!(!uses_approach_b(&evs));
    }

    /// `plain_text()` preserves every load-bearing fact: newer/less-proven, AI-assisted, BOTH shipped
    /// defects (the 8275 truncation figure and the editor latch bypass), fixed, and the check-every-
    /// figure action — and ends with a trailing newline (the `write!`-not-`writeln!` file-writer
    /// convention).
    #[test]
    fn plain_text_preserves_every_fact() {
        let t = NOTICE.plain_text();
        assert!(t.contains("newer"), "{t}");
        assert!(t.contains("less proven"), "{t}");
        assert!(t.contains("AI assistance"), "{t}");
        assert!(t.contains("137 characters"), "{t}");
        assert!(
            t.contains("no in-editor action will save until you quit"),
            "{t}"
        );
        assert!(t.contains("fixed"), "{t}");
        assert!(t.contains("check every figure"), "{t}");
        assert!(t.ends_with('\n'), "{t:?}");
        assert!(
            !t.contains('\u{1b}'),
            "plain_text must carry no ANSI escapes: {t:?}"
        );
    }
}
