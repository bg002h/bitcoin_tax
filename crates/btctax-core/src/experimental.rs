//! The Approach-B ("Defensive Filing" — declare/promote tranche, Form 8275, estimated basis)
//! experimental notice: a single, presentation-neutral source of truth for telling a filer that this
//! part of btctax is newer and less proven than the rest of the tool, was developed with heavy AI
//! assistance, and has shipped filing-affecting defects found only by later review (all known ones
//! fixed).
//!
//! ★ `defects` is EXEMPLARY, not exhaustive — do not phrase `summary`/callers around a specific count.
//! Three are listed at the time of writing (the Form 8275 truncation; the editor residue-latch bypass;
//! a filed basis derivable from a stale in-editor snapshot, `design/stale-snapshot-latch/DESIGN.md`
//! §0.1 — that one shipped in v0.10.0 too, and its own design doc records that the "both confirm tails
//! re-run a fresh plan" justification for NOT treating it as filing-affecting was "**False**" — the
//! exact predicate this notice exists to state as a standing fact, not a closed history).
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
//! future web front-end can render it in its own idiom without string-munging — no ANSI escapes, no
//! assumed line width, no terminal-specific formatting live in the data, only in each front-end's OWN
//! rendering (mechanically pinned by the `notice_fields_are_presentation_neutral` test below). Two
//! renderings are shared so no front-end hand-copies/paraphrases the facts (the exact defect class the
//! shipped `STALE_FACT_1`/`fact2_loss_sentence` precedent at `btctax-tui-edit/src/draw_edit.rs:7673`
//! already fixed once, for a different independently-drifting pair of copies):
//! [`ExperimentalNotice::plain_text`] (CLI stderr, multi-line) and [`ExperimentalNotice::one_line`] (a
//! TUI `Constraint::Length(1)` banner row, single-line).
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
///
/// `#[non_exhaustive]`: every field is `pub`, so adding one later (a link, a severity, a schema
/// version — the stated consumer is a future web UI) would otherwise be a breaking change for any
/// downstream crate that constructs or exhaustively destructures this type. `btctax-core` publishes to
/// crates.io; this costs nothing today and avoids a forced major bump later. `Serialize` for the same
/// reason: a web UI is the stated consumer, and it will want this as JSON, not re-parsed plain text.
/// `Deserialize` is deliberately NOT derived — `&'static str` cannot be deserialized into without an
/// arena, and there is exactly one instance of this type (`NOTICE`); nothing needs to build one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct ExperimentalNotice {
    /// One-line identification of the feature this notice is about.
    pub title: &'static str,
    /// The maturity + provenance disclosure: newer/less-proven, heavy AI assistance, and that
    /// filing-affecting defects have shipped and were found only by later review (all known ones fixed).
    pub summary: &'static str,
    /// EXEMPLARY, not exhaustive (see module docs) — known defects that affected FILED output, each a
    /// standalone sentence fragment. Do not treat this list's length as a count to cite elsewhere.
    pub defects: &'static [&'static str],
    /// The filer-facing call to action — concrete, PERFORMABLE checks against what this feature
    /// actually produces (never "against your own records": the feature's whole premise is
    /// undocumented BTC, so there are no records to check against).
    pub action: &'static str,
}

/// The single source of truth for the Approach-B experimental notice. Every fact in the design text is
/// preserved: newer/less-proven, heavy AI assistance, that filing-affecting defects have shipped and
/// were found only by later review (all known ones fixed), three exemplary defects, and a performable
/// action tied to what this feature actually produces.
pub const NOTICE: ExperimentalNotice = ExperimentalNotice {
    title: "EXPERIMENTAL — DEFENSIVE FILING (declare/promote tranche, Form 8275, estimated basis)",
    summary: "This feature is newer and less proven than the rest of btctax, and it was developed with \
        heavy AI assistance. Defects that affect what gets FILED have shipped and were found only by \
        later review; all known ones are fixed.",
    defects: &[
        "the Form 8275 disclosure was silently truncated to its first ~137 characters",
        "a change the editor said had been rolled back could still be written to your vault, silently \
            changing the figures on your forms",
        "a filed basis figure could be derived from a stale in-editor ledger image after a failed \
            re-projection",
    ],
    action: "Check what this feature actually produced: open the Form 8275 PDF and confirm the Part II \
        narrative renders whole, continuing onto Part IV on page 2, rather than stopping mid-sentence \
        after about one line; confirm the basis in Form 8949 column (e) for each promoted lot equals \
        the floor you consented to at promote time; and confirm the tranche quantity and acquisition \
        window on the 8275 match what you declared.",
};

impl ExperimentalNotice {
    /// Render as plain text for a terminal front-end (CLI stderr). No ANSI, no line wrapping — callers
    /// wrap/style in their own idiom. Ends with a single trailing newline (matches the shipped
    /// `Disclosure8275::render()` / `basis_methodology` convention of a write-don't-writeln body).
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

    /// A single-line rendering — `title`, `summary`, and `action` concatenated, in that order — for a
    /// fixed-height banner row (a TUI `Constraint::Length(1)`). Deliberately excludes `defects` (a
    /// list, not a sentence; `summary` already states that defects have shipped and are fixed).
    ///
    /// Every front-end that shows a compact banner MUST call this rather than hand-paraphrasing
    /// `summary`/`action` — two independent hand-copies of the same facts can be reworded differently
    /// and then drift silently apart from each other AND from this struct, with nothing red (the exact
    /// defect class already fixed once at `btctax-tui-edit/src/draw_edit.rs:7673`, for a different pair
    /// of independently-drifting copies — see that comment). No wrapping is applied here; a narrow
    /// terminal clips the tail visually, which is a rendering choice for the CALLER, not this fn.
    pub fn one_line(&self) -> String {
        format!("{} — {} {}", self.title, self.summary, self.action)
    }
}

/// **Test scaffolding**, not `#[cfg(test)]`: the leak-guard tests that assert this notice never reaches
/// a filed artifact or an export directory live in DOWNSTREAM crates (`btctax-cli`, `btctax-tui`,
/// `btctax-tui-edit`), whose test binaries cannot see a `#[cfg(test)]` item of a dependency. Mirrors the
/// established `crate::tax::testonly` / `btctax_forms::testonly` convention.
#[doc(hidden)]
pub mod testonly {
    use super::NOTICE;

    /// The distinctive phrases a leak-guard test scans for. Derived from [`NOTICE`] itself — never
    /// hand-copied — so it can never drift out of sync with the actual text: three independent
    /// hand-typed needle lists existed before this helper, and two of them were missing a phrase the
    /// third had.
    pub fn leak_guard_needles() -> Vec<&'static str> {
        let mut v = vec![NOTICE.title, NOTICE.summary, NOTICE.action];
        v.extend(NOTICE.defects.iter().copied());
        v
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

    /// `plain_text()` preserves every load-bearing fact: newer/less-proven, AI-assisted, that
    /// filing-affecting defects have shipped and were found only by later review (all known ones
    /// fixed), all three exemplary defects (the 8275 truncation, the editor latch bypass, and the
    /// stale-snapshot basis), and the performable action — and ends with a trailing newline (the
    /// `write!`-not-`writeln!` file-writer convention).
    #[test]
    fn plain_text_preserves_every_fact() {
        let t = NOTICE.plain_text();
        assert!(t.contains("newer"), "{t}");
        assert!(t.contains("less proven"), "{t}");
        assert!(t.contains("AI assistance"), "{t}");
        assert!(t.contains("found only by later review"), "{t}");
        assert!(t.contains("137 characters"), "{t}");
        assert!(
            t.contains("could still be written to your vault, silently changing the figures"),
            "{t}"
        );
        assert!(
            t.contains("a stale in-editor ledger image after a failed re-projection"),
            "{t}"
        );
        assert!(t.contains("all known ones are fixed"), "{t}");
        assert!(t.contains("Form 8949 column (e)"), "{t}");
        assert!(t.contains("Part IV on page 2"), "{t}");
        assert!(t.ends_with('\n'), "{t:?}");
        assert!(
            !t.contains('\u{1b}'),
            "plain_text must carry no ANSI escapes: {t:?}"
        );
    }

    /// `one_line()` is DERIVED — `title`, `summary`, `action`, in that order, joined with " — "/" " —
    /// not an independent hand-typed sentence. Pins the exact formula so a future edit that hardcodes a
    /// literal inside `one_line()` itself (rather than reading `self.title`/`self.summary`/`self.action`)
    /// is caught here, even though this specific test cannot see how the TUI crates call it.
    #[test]
    fn one_line_is_derived_from_title_summary_and_action() {
        assert_eq!(
            NOTICE.one_line(),
            format!("{} — {} {}", NOTICE.title, NOTICE.summary, NOTICE.action)
        );
        let l = NOTICE.one_line();
        assert!(l.contains(NOTICE.title), "{l}");
        assert!(l.contains(NOTICE.summary), "{l}");
        assert!(l.contains(NOTICE.action), "{l}");
    }

    /// ★ THE PRESENTATION-NEUTRALITY CONTRACT (module docs): no ANSI, no assumed width, no terminal
    /// formatting lives in the DATA — and it holds only because every literal uses `\`-continuations
    /// rather than embedded newlines. Pins that mechanically over `title`, `summary`, `action`, and
    /// every `defects` element: no `\n`, `\r`, ESC (`\u{1b}`), tab, or a run of two-or-more spaces (a
    /// tell that someone hard-wrapped/indented the text for a specific terminal width, defeating a
    /// front-end's own wrapping).
    ///
    /// Mutation: change one `\`-continuation in `NOTICE` to an embedded `\n` → this reds (verified,
    /// reverted via `cp`; see `design/approach-b-experimental-notice/BUILD-REPORT.md`).
    #[test]
    fn notice_fields_are_presentation_neutral() {
        let mut fields: Vec<&str> = vec![NOTICE.title, NOTICE.summary, NOTICE.action];
        fields.extend(NOTICE.defects.iter().copied());
        for f in fields {
            assert!(!f.contains('\n'), "no embedded newline: {f:?}");
            assert!(!f.contains('\r'), "no embedded CR: {f:?}");
            assert!(!f.contains('\u{1b}'), "no ANSI escape: {f:?}");
            assert!(!f.contains('\t'), "no tab: {f:?}");
            assert!(
                !f.contains("  "),
                "no run of 2+ spaces (a hard-wrap/indent tell): {f:?}"
            );
        }
    }

    /// [`testonly::leak_guard_needles`] is genuinely derived — it must contain every field verbatim,
    /// including every `defects` element, so a leak-guard test built on it can never silently omit a
    /// phrase the way the three independent hand-typed needle lists it replaces once could.
    #[test]
    fn leak_guard_needles_covers_every_field() {
        let needles = testonly::leak_guard_needles();
        assert!(needles.contains(&NOTICE.title));
        assert!(needles.contains(&NOTICE.summary));
        assert!(needles.contains(&NOTICE.action));
        for d in NOTICE.defects {
            assert!(needles.contains(d), "missing defect: {d:?}");
        }
    }
}
