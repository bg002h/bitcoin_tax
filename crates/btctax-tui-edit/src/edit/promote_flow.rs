//! The Promote flow (Task 9, Phase P-C): select a tranche row (DFW-D12 — one tranche at a time; no
//! bulk-promote) → **attest the BG-D5 acquisition provenance** (the filer SELECTS from the closed
//! `btctax_cli::ProvenanceKind` enumeration; the tool never answers it for them) → author the Form 8275
//! Part II narrative (an in-TUI **multiline** path, DFW-D12/M-2) → drive the shipped
//! `btctax_cli::plan_promote` chokepoint for the two-sided informed-consent screen (`render_consent`) →
//! a TypedWord acknowledgment gate (mirrors `PROMOTE_ACK_PHRASE`) → promote.
//! **C-3:** this module COLLECTS the provenance answer, the Part II narrative and the typed ack phrase,
//! and READS `btctax_cli::plan_promote`/`render_consent` — it never calls `btctax_cli::apply_promote`
//! directly; the WRITE goes through `edit::persist::persist_promote_tranche` (the ONLY caller of
//! `apply_promote` in this crate, mechanically enforced by `persist::tests::kat_g1_mechanized_source_gate`).

use crate::edit::form::FieldBuffer;
use btctax_core::price::PriceProvider;
use btctax_core::project::ProjectionConfig;
use btctax_core::{EventId, LedgerEvent};

/// Generous byte-length cap for the Form 8275 Part II narrative buffer (DFW-D12/M-2: "an in-TUI
/// multiline path") — larger than the donation free-text fields' `FREETEXT_CAP` (512), since a real
/// acquisition narrative may run to several sentences/paragraphs across multiple lines.
pub const PART_II_CAP: usize = 4096;

/// The prompt shown when the filer presses Enter on the BG-D5 provenance step WITHOUT having selected
/// anything. NOT a refusal text (no engine gate has run): it is the UI insisting the filer answer, which
/// is the whole point of the step (the answered-ness invariant — the tool must never silently answer a
/// filing attestation for the filer).
const PROVENANCE_UNANSWERED: &str =
    "select how you acquired these units (press 1-7) — this is YOUR \
     attestation about YOUR coins; the tool will not answer it for you.";

/// Defensive-only, provably unreachable: `plan_promote` refuses EVERY non-`Purchase` provenance (BG-D5)
/// immediately after resolve-live, so the `Ok` arm of the non-purchase probe below cannot be taken. If it
/// somehow were, fail CLOSED (stay on the step, discard the plan) rather than advance. This is an
/// internal-invariant message, NOT a re-implementation of any gate's refusal copy.
const PROVENANCE_ENGINE_DID_NOT_REFUSE: &str =
    "internal error: the engine did not refuse a non-purchase provenance. Nothing was recorded; this \
     promote is refused here. Please report this, and use the CLI's `btctax promote-tranche` if needed.";

/// Which step of the flow is active.
///
/// `Provenance` is the FIRST step (★ P-C gate tax I-2 / BG-D5): the filer must ACTIVELY choose their
/// acquisition provenance from the closed `btctax_cli::ProvenanceKind` enumeration, exactly as the CLI's
/// `--provenance` makes them choose. "No acquisition record" is NOT "purchased" — mined/forked/airdropped
/// coins carry a documented FMV-at-receipt or carryover basis (Notice 2014-21; Rev. Rul. 2019-24), which
/// is why the enumeration is closed and every non-`Purchase` value is refused. The refusal is
/// ENGINE-enforced (DFW-D1): a non-`Purchase` answer is driven through `btctax_cli::plan_promote`, whose
/// shipped `Refusal::Provenance` text is what the filer reads — nothing is re-implemented here.
///
/// `PartII` carries the LAST `plan_promote` refusal (if any), so a bounced-back filer sees WHY, not a
/// silent re-entry (mirrors the Declare flow's own refusal-surfacing philosophy, DFW-D5).
///
/// `Consent` carries the ALREADY-COMPUTED `PromotePlan` + its rendered consent text + a fresh TypedWord
/// ack buffer — computed ONCE at the PartII→Consent transition (`review`), never recomputed at the final
/// Enter. ★ arch-m-new-3 (chokepoint/mod.rs): a single synchronous CLI/TUI invocation cannot append
/// anything between `plan_promote` and `apply_promote` — and this flow is the ONLY mutation surface open
/// at a time (the editor's "at most one flow `Some`" invariant) — so reusing the SAME already-computed
/// plan all the way to `persist_promote_tranche` is behavior-preserving; it exactly mirrors the CLI thin
/// driver (`cmd::promote::promote_tranche`), which also computes the plan exactly once.
#[derive(Debug)]
pub enum PromoteFlowStep {
    /// The BG-D5 acquisition-provenance attestation. The filer's pick lives on
    /// `PromoteFlowState::provenance` (`None` until they choose); `error` carries either the
    /// "you must answer" prompt or the SHIPPED `Refusal::Provenance` text after a non-purchase answer.
    Provenance { error: Option<String> },
    /// Authoring the Part II narrative. `error` is `Some` after a bounced-back `review()` refusal.
    PartII { error: Option<String> },
    /// The consent screen + TypedWord ack gate.
    Consent {
        /// Boxed (clippy `large_enum_variant`): `PromotePlan` carries `Vec<ConsentTerm>` +
        /// `BTreeSet<i32>` + the full `EventPayload` — large enough that leaving it inline would bloat
        /// every `PromoteFlowStep`, incl. the far-smaller `PartII` variant, to match its size.
        plan: Box<btctax_cli::PromotePlan>,
        /// `btctax_cli::render_consent(&plan)` — the byte-identical (to the CLI) filer-visible text.
        rendered: String,
        /// The typed acknowledgment buffer (mirrors `SafeHarborAttestStep::TypedWord`'s own buffer).
        ack: FieldBuffer,
        /// `Some` after a wrong-phrase Enter (buffer PRESERVED — the filer corrects via Backspace).
        error: Option<String>,
    },
}

/// `PromoteFlow{target, part_ii, step}` — DFW-D12: one tranche at a time. This state ALWAYS targets
/// exactly one `DeclareTranche` decision; there is no bulk/multi-select promote (each promotion needs its
/// own consent figures, its own Part II narrative, and its own `Acknowledgment` — DFW-D12/SPEC.md).
///
/// **C-3:** this module COLLECTS input and READS `btctax_cli::plan_promote`/`render_consent` — it never
/// calls `btctax_cli::apply_promote` directly; the WRITE goes through
/// `edit::persist::persist_promote_tranche` (the ONLY caller of `apply_promote` in this crate,
/// mechanically enforced by `persist::tests::kat_g1_mechanized_source_gate`).
#[derive(Debug)]
pub struct PromoteFlowState {
    /// The `DeclareTranche` decision this flow promotes (the dashboard row's own `target`).
    pub target: EventId,
    /// ★ BG-D5 (P-C gate tax I-2): the filer's OWN acquisition-provenance answer. `None` until they
    /// actively select one on the `Provenance` step — the tool NEVER supplies a value here, so
    /// `plan_promote` always receives what the FILER attested, exactly like the CLI's `--provenance`.
    /// Lives OUTSIDE `step` so a later bounce back to `Provenance` shows their previous pick.
    pub provenance: Option<btctax_cli::ProvenanceKind>,
    /// The in-TUI multiline Part II narrative buffer. Lives OUTSIDE `step` (not nested inside `PartII`)
    /// so a Consent→PartII bounce (Esc, or a refusal) PRESERVES the filer's authored text for further
    /// editing rather than discarding it.
    pub part_ii: FieldBuffer,
    pub step: PromoteFlowStep,
}

impl PromoteFlowState {
    /// Open the flow for `target`, at the BG-D5 `Provenance` step, with NO provenance answered and an
    /// empty narrative buffer.
    pub fn new(target: EventId) -> Self {
        Self {
            target,
            provenance: None,
            part_ii: FieldBuffer::with_cap(PART_II_CAP),
            step: PromoteFlowStep::Provenance { error: None },
        }
    }

    /// Record the filer's provenance pick (a pure setter — it advances nothing and gates nothing; the
    /// filer confirms with Enter, which runs `attest_provenance`). Clears any stale step error.
    pub fn select_provenance(&mut self, kind: btctax_cli::ProvenanceKind) {
        self.provenance = Some(kind);
        self.step = PromoteFlowStep::Provenance { error: None };
    }

    /// Confirm the BG-D5 provenance answer (the `Provenance` step's Enter).
    ///
    /// - Nothing selected → the step insists (the answered-ness invariant: never answer for the filer).
    /// - `Purchase` → advance to Part II authoring.
    /// - Any other value → drive the FILER'S CHOSEN kind through the shipped `btctax_cli::plan_promote`
    ///   chokepoint and surface its `Refusal::Provenance` verbatim (via the shipped `Refusal → CliError`
    ///   mapping). DFW-D1: the gate stays ENGINE-enforced — this flow re-implements no refusal, and
    ///   `plan_promote` is a pure read, so a refused answer records nothing by construction.
    pub fn attest_provenance(
        &mut self,
        events: &[LedgerEvent],
        prices: &dyn PriceProvider,
        cfg: &ProjectionConfig,
        now: time::OffsetDateTime,
    ) {
        let Some(kind) = self.provenance else {
            self.step = PromoteFlowStep::Provenance {
                error: Some(PROVENANCE_UNANSWERED.to_string()),
            };
            return;
        };
        if kind == btctax_cli::ProvenanceKind::Purchase {
            self.step = PromoteFlowStep::PartII { error: None };
            return;
        }
        // BG-D5 runs BEFORE the Part II gate inside `plan_promote`, so the (possibly still empty)
        // narrative cannot mask the provenance refusal.
        let part_ii_text = self.part_ii.as_str().to_string();
        let error = match btctax_cli::plan_promote(
            events,
            prices,
            cfg,
            &self.target,
            kind,
            &part_ii_text,
            now,
        ) {
            Err(refusal) => {
                let err: btctax_cli::CliError = refusal.into();
                err.to_string()
            }
            Ok(_plan) => {
                debug_assert!(
                    false,
                    "plan_promote accepted a non-Purchase provenance ({kind:?}) — BG-D5 is broken"
                );
                PROVENANCE_ENGINE_DID_NOT_REFUSE.to_string()
            }
        };
        self.step = PromoteFlowStep::Provenance { error: Some(error) };
    }

    /// Attempt to move from Part II authoring to the consent screen: runs `btctax_cli::plan_promote`
    /// FRESH over the caller's `events`/`prices`/`cfg`, passing the provenance THE FILER ATTESTED on the
    /// `Provenance` step (★ BG-D5 — never a value this flow supplies). `Ok` transitions to `Consent`
    /// (`render_consent(&plan)` + a fresh ack buffer); `Err` surfaces the refusal INLINE on the PartII
    /// step (BG-D3/BG-D7/`Refusal::Target`) — the filer's authored narrative is preserved on
    /// `self.part_ii`, never discarded, so they can revise and retry (mirrors the Declare flow's own
    /// "a refusal with a reason, not a silent append"). With no provenance answered this bounces to the
    /// `Provenance` step: unreachable through the step machine (PartII is only reachable via an attested
    /// `Purchase`), and fail-closed rather than substituting an answer the filer never gave.
    pub fn review(
        &mut self,
        events: &[LedgerEvent],
        prices: &dyn PriceProvider,
        cfg: &ProjectionConfig,
        now: time::OffsetDateTime,
    ) {
        let Some(provenance) = self.provenance else {
            self.step = PromoteFlowStep::Provenance {
                error: Some(PROVENANCE_UNANSWERED.to_string()),
            };
            return;
        };
        let part_ii_text = self.part_ii.as_str().to_string();
        match btctax_cli::plan_promote(
            events,
            prices,
            cfg,
            &self.target,
            provenance,
            &part_ii_text,
            now,
        ) {
            Ok(plan) => {
                let rendered = btctax_cli::render_consent(&plan);
                self.step = PromoteFlowStep::Consent {
                    plan: Box::new(plan),
                    rendered,
                    ack: FieldBuffer::new(),
                    error: None,
                };
            }
            Err(refusal) => {
                let err: btctax_cli::CliError = refusal.into();
                self.step = PromoteFlowStep::PartII {
                    error: Some(err.to_string()),
                };
            }
        }
    }
}

// ── Render (pure; no ratatui dependency here — draw_edit.rs wraps these lines in a Paragraph) ─────────

/// The Provenance step's "why only a purchase" line (★ tax Nit 1 / arch M-r2-2, P-C gate r2).
///
/// The enumeration of non-`Purchase` kinds is built from `ProvenanceKind::ALL` — the SAME closed list
/// the picker below and `refuse_non_purchase`'s refusal already derive from — so a new/renamed variant
/// is named here automatically instead of leaving a second, hand-maintained copy to drift stale (this
/// project's known taxonomy-drift failure class).
///
/// The basis MECHANISM is stated per regime rather than the previous blended "FMV at receipt, or
/// donor/decedent carryover", which paired the wrong mechanism with the wrong kind: a **gift** keeps
/// the donor's §1015 carryover basis; an **inheritance** steps up to fair market value at the date of
/// death, §1014 — never a carryover; everything else (mining/staking/earning/airdrop/fork) uses fair
/// market value AT RECEIPT (Notice 2014-21; Rev. Rul. 2019-24; Rev. Rul. 2023-14).
fn non_purchase_basis_note() -> String {
    let non_purchase: Vec<&str> = btctax_cli::ProvenanceKind::ALL
        .iter()
        .filter(|k| **k != btctax_cli::ProvenanceKind::Purchase)
        .map(|k| k.label())
        .collect();
    let (last, rest) = non_purchase
        .split_last()
        .expect("ALL always carries at least one non-Purchase kind");
    let enumeration = format!("{}, or {last}", rest.join(", "));
    format!(
        "Only a PURCHASE can be promoted to a >$0 estimated-basis floor: units acquired by \
         {enumeration} already have a documented, real basis of their own — a gift carries the \
         donor's carryover basis, an inheritance steps up to fair market value at the date of \
         death, and the rest carry a fair-market-value-at-receipt basis — model that real \
         acquisition instead."
    )
}

/// The full Promote flow render — a pure derived text render (mirrors `render_declare_flow`'s own "pure
/// String builder" shape).
pub fn render_promote_flow(state: &PromoteFlowState) -> Vec<String> {
    let mut lines = vec![
        format!("Promote — tranche {:?}", state.target),
        String::new(),
    ];

    match &state.step {
        PromoteFlowStep::Provenance { error } => {
            lines.push(
                "How did you ACQUIRE these units? This is YOUR attestation about YOUR coins — the tool \
                 cannot and will not answer it for you."
                    .to_string(),
            );
            lines.push(non_purchase_basis_note());
            lines.push(String::new());
            for (i, kind) in btctax_cli::ProvenanceKind::ALL.iter().enumerate() {
                let marker = if state.provenance == Some(*kind) {
                    '>'
                } else {
                    ' '
                };
                lines.push(format!("{marker} [{}] {}", i + 1, kind.label()));
            }
            lines.push(String::new());
            lines.push(match state.provenance {
                Some(k) => format!("selected: {}", k.label()),
                None => "selected: (none yet — press 1-7)".to_string(),
            });
            if let Some(e) = error {
                lines.push(String::new());
                // The "you must answer" prompt is not a refusal (no engine gate ran); only the shipped
                // `Refusal::Provenance` (or a `Refusal::Target`) earns the REFUSED banner.
                if e == PROVENANCE_UNANSWERED {
                    lines.push(e.clone());
                } else {
                    lines.push(format!("REFUSED: {e}"));
                }
            }
            lines.push(String::new());
            lines.push("[1-7] choose   [Enter] attest & continue   [Esc] cancel".to_string());
        }
        PromoteFlowStep::PartII { error } => {
            lines.push(
                "Author the Form 8275 Part II narrative below: real, specific facts about how and when \
                 these coins were acquired (Reg. §1.6662-4(f), 'in sufficient detail'). An empty or \
                 whitespace-only narrative is refused."
                    .to_string(),
            );
            lines.push(format!(
                "By continuing, you attest: {}",
                btctax_cli::PROVENANCE_TEXT
            ));
            lines.push(String::new());
            for line in state.part_ii.as_str().split('\n') {
                lines.push(format!("  {line}"));
            }
            if let Some(e) = error {
                lines.push(String::new());
                lines.push(format!("REFUSED: {e}"));
            }
            lines.push(String::new());
            lines.push("[Enter] new line   [Tab] review consent screen   [Esc] cancel".to_string());
        }
        PromoteFlowStep::Consent {
            rendered,
            ack,
            error,
            ..
        } => {
            for line in rendered.split('\n') {
                lines.push(line.to_string());
            }
            lines.push(String::new());
            // T9-review Minor-1: echo the purchase attestation adjacent to the moment of consent —
            // `rendered` (`btctax_cli::render_consent(&plan)`) never includes `PROVENANCE_TEXT` (it's
            // built purely from `plan.advisory_lines`/`plan.terms`/`plan.gift_only_years`/
            // `plan.post_consent_note` — see `chokepoint::render_consent`), so without this line the
            // Part II screen is the ONLY place the filer sees what they're attesting. DISPLAY-only: this
            // is the flow's own render, not `render_consent`/`shown_terms` — the recorded `Acknowledgment`
            // is untouched.
            lines.push(format!("You attest: {}", btctax_cli::PROVENANCE_TEXT));
            lines.push(String::new());
            lines.push(format!(
                "Type the acknowledgment phrase EXACTLY to record this promote: {:?}",
                btctax_cli::PROMOTE_ACK_PHRASE
            ));
            lines.push(format!("> {}", ack.as_str()));
            if let Some(e) = error {
                lines.push(String::new());
                lines.push(format!("REFUSED: {e}"));
            }
            lines.push(String::new());
            lines.push("[Enter] submit   [Esc] back to Part II authoring".to_string());
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::event::{ConsentTerm, DeclareTranche, EventPayload};
    use btctax_core::price::StaticPrices;
    use btctax_core::{TaxDate, WalletId};
    use std::collections::BTreeMap;
    use time::macros::date;

    fn wallet() -> WalletId {
        WalletId::SelfCustody {
            label: "promote-flow-test".into(),
        }
    }

    fn now() -> time::OffsetDateTime {
        time::macros::datetime!(2026 - 01 - 01 0:00 UTC)
    }

    fn cfg() -> ProjectionConfig {
        ProjectionConfig::default()
    }

    /// A vault-free fixture: ONE live, unpromoted `DeclareTranche` over `[window_start, window_end]`.
    fn tranche_events(
        window_start: TaxDate,
        window_end: TaxDate,
        sat: i64,
    ) -> (EventId, Vec<LedgerEvent>) {
        let id = EventId::decision(1);
        let events = vec![LedgerEvent {
            id: id.clone(),
            utc_timestamp: now(),
            original_tz: time::UtcOffset::UTC,
            wallet: None,
            payload: EventPayload::DeclareTranche(DeclareTranche {
                sat,
                wallet: wallet(),
                window_start,
                window_end,
            }),
        }];
        (id, events)
    }

    /// FULL daily-close coverage over `[window_start, window_end]` (BG-D3's `Coverage::Full`).
    fn full_price_coverage(window_start: TaxDate, window_end: TaxDate) -> StaticPrices {
        let mut m = BTreeMap::new();
        let mut d = window_start;
        loop {
            m.insert(d, rust_decimal_macros::dec!(10_000));
            if d == window_end {
                break;
            }
            d = d.next_day().unwrap();
        }
        StaticPrices(m)
    }

    fn type_str(buf: &mut FieldBuffer, s: &str) {
        for c in s.chars() {
            buf.push_char(c);
        }
    }

    /// Drive the BG-D5 provenance step the way the filer does: pick `Purchase`, confirm. Every test
    /// below that needs to be AT the Part II step goes through this — there is no back door.
    fn attest_purchase(
        state: &mut PromoteFlowState,
        events: &[LedgerEvent],
        prices: &dyn PriceProvider,
    ) {
        state.select_provenance(btctax_cli::ProvenanceKind::Purchase);
        state.attest_provenance(events, prices, &cfg(), now());
    }

    // ── constructor / render sanity ────────────────────────────────────────────────────────────────

    #[test]
    fn new_opens_at_the_provenance_step_with_nothing_answered() {
        let state = PromoteFlowState::new(EventId::decision(1));
        assert!(matches!(
            state.step,
            PromoteFlowStep::Provenance { error: None }
        ));
        assert_eq!(
            state.provenance, None,
            "★ BG-D5: the flow must open with NO provenance answered — the tool never answers a filing \
             attestation for the filer"
        );
        assert!(state.part_ii.is_empty());
    }

    #[test]
    fn part_ii_step_renders_the_provenance_attestation_and_key_hints() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        attest_purchase(&mut state, &events, &prices);
        let rendered = render_promote_flow(&state).join("\n");
        assert!(rendered.contains(btctax_cli::PROVENANCE_TEXT));
        assert!(
            rendered.contains("Tab"),
            "must hint the review key: {rendered}"
        );
    }

    // ── ★ P-C gate tax I-2 (BG-D5): the provenance question is ASKED, and engine-enforced ───────────

    #[test]
    fn the_provenance_step_offers_the_whole_closed_enumeration_and_asserts_nothing_for_the_filer() {
        let state = PromoteFlowState::new(EventId::decision(1));
        let rendered = render_promote_flow(&state).join("\n");
        for kind in btctax_cli::ProvenanceKind::ALL {
            assert!(
                rendered.contains(kind.label()),
                "the closed BG-D5 enumeration must be OFFERED in full — {:?} ({}) missing: {rendered}",
                kind,
                kind.label()
            );
        }
        assert!(
            rendered.contains("none yet"),
            "nothing may be pre-selected FOR the filer: {rendered}"
        );
    }

    /// arch M-r2-2 (P-C gate r2): the "why only a purchase" PROSE — not just the picker rows below it —
    /// must name every non-`Purchase` label. Kept even though `non_purchase_basis_note` builds the
    /// enumeration from `ProvenanceKind::ALL` (so this can't drift stale), as documentation-grade
    /// regression insurance against a future hand-edit re-introducing a hardcoded copy.
    #[test]
    fn the_provenance_restriction_prose_names_every_non_purchase_label() {
        let note = non_purchase_basis_note();
        for kind in btctax_cli::ProvenanceKind::ALL {
            if kind == btctax_cli::ProvenanceKind::Purchase {
                continue;
            }
            assert!(
                note.contains(kind.label()),
                "the provenance-restriction PROSE must name every non-Purchase label, not just the \
                 picker rows below it — {:?} ({}) missing: {note}",
                kind,
                kind.label()
            );
        }
    }

    #[test]
    fn enter_without_answering_the_provenance_question_never_advances() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        state.attest_provenance(&events, &prices, &cfg(), now());
        match &state.step {
            PromoteFlowStep::Provenance { error } => {
                assert!(
                    error.is_some(),
                    "an unanswered attestation must insist, not advance"
                );
            }
            other => panic!("must NOT advance without an answer: {other:?}"),
        }
        assert_eq!(state.provenance, None);
    }

    /// (a) A non-Purchase answer surfaces the SHIPPED `Refusal::Provenance` text (byte-compared against
    /// the chokepoint's own mapping — nothing re-implemented in the TUI) and records NOTHING: the flow
    /// never leaves the Provenance step, so `persist_promote_tranche` is never reachable.
    #[test]
    fn a_non_purchase_answer_surfaces_the_shipped_refusal_and_never_advances() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));

        for kind in btctax_cli::ProvenanceKind::ALL {
            if kind == btctax_cli::ProvenanceKind::Purchase {
                continue;
            }
            let mut state = PromoteFlowState::new(id.clone());
            state.select_provenance(kind);
            state.attest_provenance(&events, &prices, &cfg(), now());

            // The SHIPPED text: exactly what `plan_promote` → `CliError` produces for this kind.
            let shipped: String = match btctax_cli::plan_promote(
                &events,
                &prices,
                &cfg(),
                &id,
                kind,
                "cash P2P purchase, no records",
                now(),
            ) {
                Err(refusal) => btctax_cli::CliError::from(refusal).to_string(),
                Ok(_) => panic!("BG-D5 must refuse {kind:?}"),
            };

            match &state.step {
                PromoteFlowStep::Provenance { error } => {
                    let e = error.as_ref().expect("a non-purchase answer must refuse");
                    assert_eq!(
                        e, &shipped,
                        "the filer must read the SHIPPED Refusal::Provenance text, never a TUI \
                         re-implementation ({kind:?})"
                    );
                    assert!(
                        e.contains(kind.label()),
                        "the refusal must name the filer's own answer ({kind:?}): {e}"
                    );
                }
                other => panic!("a non-purchase answer must NEVER advance ({kind:?}): {other:?}"),
            }
            let rendered = render_promote_flow(&state).join("\n");
            assert!(
                rendered.contains("REFUSED:"),
                "the refusal must be VISIBLE on the step: {rendered}"
            );
        }
    }

    /// The value reaching `plan_promote` is the FILER'S, not a constant this flow supplies: with a
    /// non-Purchase answer on the state, `review()` (the Part II → Consent transition) can never mint a
    /// `Purchase` and sail through.
    #[test]
    fn review_never_substitutes_a_provenance_the_filer_did_not_give() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));

        // Unanswered: bounces back to the attestation, never to Consent.
        let mut state = PromoteFlowState::new(id.clone());
        type_str(&mut state.part_ii, "cash P2P purchase, no records");
        state.review(&events, &prices, &cfg(), now());
        assert!(
            matches!(state.step, PromoteFlowStep::Provenance { error: Some(_) }),
            "review() with no attested provenance must bounce to the Provenance step: {:?}",
            state.step
        );

        // Answered non-Purchase: the engine refuses; still never Consent.
        let mut state = PromoteFlowState::new(id);
        state.provenance = Some(btctax_cli::ProvenanceKind::Mining);
        type_str(&mut state.part_ii, "mined on a laptop in 2010");
        state.review(&events, &prices, &cfg(), now());
        assert!(
            !matches!(state.step, PromoteFlowStep::Consent { .. }),
            "a mining provenance must never reach the consent screen: {:?}",
            state.step
        );
    }

    // ── (b): an empty/whitespace Part II is refused (BG-D7) ────────────────────────────────────────

    #[test]
    fn empty_part_ii_is_refused_and_preserves_the_step() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        attest_purchase(&mut state, &events, &prices);
        // Left empty — never advances.
        state.review(&events, &prices, &cfg(), now());
        match &state.step {
            PromoteFlowStep::PartII { error } => {
                let e = error
                    .as_ref()
                    .expect("an empty narrative must refuse (BG-D7)");
                assert!(
                    e.to_lowercase().contains("part ii") || e.to_lowercase().contains("narrative"),
                    "the refusal must name the Part II gate: {e}"
                );
            }
            other => panic!("must NOT advance past Part II on an empty narrative: {other:?}"),
        }
    }

    #[test]
    fn whitespace_only_part_ii_is_refused_bg_d7() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        attest_purchase(&mut state, &events, &prices);
        type_str(&mut state.part_ii, "   \n   ");
        state.review(&events, &prices, &cfg(), now());
        match &state.step {
            PromoteFlowStep::PartII { error } => {
                assert!(
                    error.is_some(),
                    "a whitespace-only (incl. multiline whitespace) narrative must refuse (BG-D7)"
                );
            }
            other => {
                panic!("must NOT advance past Part II on a whitespace-only narrative: {other:?}")
            }
        }
        // The filer's (whitespace) text is preserved, not discarded, on a bounce-back.
        assert_eq!(state.part_ii.as_str(), "   \n   ");
    }

    // ── (d): an undisposed tranche promotes and records the Unrealized term (DFW-D5.3) ─────────────

    #[test]
    fn undisposed_tranche_promotes_and_records_the_unrealized_term() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        attest_purchase(&mut state, &events, &prices);
        type_str(
            &mut state.part_ii,
            "cash P2P purchase, no records; on-chain window bounded",
        );
        state.review(&events, &prices, &cfg(), now());
        match &state.step {
            PromoteFlowStep::Consent { plan, rendered, .. } => {
                let EventPayload::PromoteTranche(p) = &plan.payload else {
                    panic!("plan.payload must be a PromoteTranche");
                };
                assert!(
                    p.acknowledgment
                        .shown_terms
                        .iter()
                        .any(|t| matches!(t, ConsentTerm::Unrealized { .. })),
                    "a fully-undisposed promote must record an Unrealized term, never a bare empty Vec: \
                     {:?}",
                    p.acknowledgment.shown_terms
                );
                assert!(
                    rendered.to_lowercase().contains("undisposed"),
                    "the rendered consent screen must surface it too: {rendered}"
                );
            }
            other => {
                panic!(
                    "a valid narrative over a fully-covered window must reach Consent: {other:?}"
                )
            }
        }
    }

    // ── T9-review Minor-1: the Consent step ECHOES the purchase attestation adjacent to the ack ──────

    #[test]
    fn consent_step_renders_the_purchase_attestation_echo_above_the_ack_prompt() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        attest_purchase(&mut state, &events, &prices);
        type_str(
            &mut state.part_ii,
            "cash P2P purchase, no records; on-chain window bounded",
        );
        state.review(&events, &prices, &cfg(), now());
        assert!(
            matches!(state.step, PromoteFlowStep::Consent { .. }),
            "this fixture's review() must reach Consent"
        );
        let rendered = render_promote_flow(&state).join("\n");
        let attest_line = format!("You attest: {}", btctax_cli::PROVENANCE_TEXT);
        assert!(
            rendered.contains(&attest_line),
            "the Consent step must echo the purchase attestation adjacent to the ack prompt: {rendered}"
        );
        // Adjacency: the echo must sit ABOVE the ack prompt (right before the moment of consent), not
        // buried after it.
        let attest_pos = rendered.find(&attest_line).expect("checked above");
        let ack_prompt_pos = rendered
            .find("Type the acknowledgment phrase EXACTLY")
            .expect("the ack prompt must render");
        assert!(
            attest_pos < ack_prompt_pos,
            "the attestation echo must render ABOVE the ack prompt: {rendered}"
        );
    }

    // ── (a) / T4 tie-in: the TUI promote path records an Acknowledgment Eq-identical to the CLI ─────
    //
    // Task 4's own harness (`crates/btctax-cli/tests/chokepoint_parity.rs`) spawns the real `btctax`
    // binary via `CARGO_BIN_EXE_btctax` — NOT available here (verified empirically: `env!` fails to
    // compile in this crate's tests; the var is only set for a package's OWN `[[bin]]` targets, not a
    // downstream crate's — `btctax-tui-edit` depends on `btctax-cli`, not the reverse). So "the CLI"
    // side here is the shipped IN-PROCESS driver fn `btctax_cli::cmd::promote::promote_tranche` — the
    // EXACT fn `main.rs`'s CLI dispatch calls — rather than a spawned binary; "the TUI" side drives
    // `PromoteFlowState::review` → `edit::persist::persist_promote_tranche`, the real production path.
    // Both sides build an IDENTICALLY-constructed vault, so identical decision-sequence numbers land on
    // both, and the recorded `Acknowledgment` (the §6664(c) good-faith artifact) must be `Eq`-identical.
    #[test]
    fn tui_promote_records_an_acknowledgment_eq_identical_to_the_cli_driver() {
        use btctax_core::persistence::load_all;

        fn build_vault(dir: &std::path::Path) -> (std::path::PathBuf, EventId) {
            let pp = btctax_store::Passphrase::new("pw".into());
            let vault = dir.join("vault.pgp");
            btctax_cli::cmd::init::run(&vault, &pp, &dir.join("k.asc")).unwrap();
            let target = btctax_cli::cmd::tranche::declare_tranche(
                &vault,
                &pp,
                40_000_000,
                wallet(),
                date!(2020 - 01 - 01),
                date!(2020 - 01 - 10),
                now(),
            )
            .unwrap();
            (vault, target)
        }

        fn only_promote(vault: &std::path::Path) -> btctax_core::event::PromoteTranche {
            let pp = btctax_store::Passphrase::new("pw".into());
            let s = btctax_cli::Session::open(vault, &pp).unwrap();
            load_all(s.conn())
                .unwrap()
                .into_iter()
                .find_map(|e| match e.payload {
                    EventPayload::PromoteTranche(p) => Some(p),
                    _ => None,
                })
                .expect("exactly one PromoteTranche recorded")
        }

        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let (vault_a, target_a) = build_vault(dir_a.path());
        let (vault_b, target_b) = build_vault(dir_b.path());
        assert_eq!(
            target_a, target_b,
            "identical construction on two fresh vaults must yield identical decision refs"
        );

        let part_ii_text = "cash P2P purchase, no records";

        // (a) the shipped CLI driver fn (in-process — the exact fn main.rs's dispatch calls).
        let pp_a = btctax_store::Passphrase::new("pw".into());
        btctax_cli::cmd::promote::promote_tranche(
            &vault_a,
            &pp_a,
            &target_a.canonical(),
            btctax_cli::ProvenanceKind::Purchase,
            part_ii_text.to_string(),
            Some(btctax_cli::PROMOTE_ACK_PHRASE),
            now(),
        )
        .unwrap();

        // (b) the TUI Promote flow's own production path.
        let pp_b = btctax_store::Passphrase::new("pw".into());
        let mut session_b = btctax_cli::Session::open(&vault_b, &pp_b).unwrap();
        let events_b = load_all(session_b.conn()).unwrap();
        let cfg_b = session_b.config().unwrap().to_projection();
        let mut state = PromoteFlowState::new(target_b);
        // ★ BG-D5: the TUI filer ATTESTS purchase on the Provenance step (the CLI side passes the same
        // `--provenance purchase` above) — the recorded artifact must still be Eq-identical.
        state.select_provenance(btctax_cli::ProvenanceKind::Purchase);
        state.attest_provenance(&events_b, session_b.prices(), &cfg_b, now());
        type_str(&mut state.part_ii, part_ii_text);
        state.review(&events_b, session_b.prices(), &cfg_b, now());
        let (plan_b, rendered_b) = match state.step {
            PromoteFlowStep::Consent { plan, rendered, .. } => (plan, rendered),
            other => panic!("this fixture's review() must reach Consent: {other:?}"),
        };
        crate::edit::persist::persist_promote_tranche(
            &mut session_b,
            *plan_b,
            Some(btctax_cli::PROMOTE_ACK_PHRASE),
            now(),
        )
        .unwrap();
        drop(session_b); // release the vault lock before re-opening vault_b below

        // ★ Eq-identical recorded Acknowledgment (the §6664(c) good-faith artifact).
        let promote_a = only_promote(&vault_a);
        let promote_b = only_promote(&vault_b);
        assert_eq!(
            promote_a.acknowledgment, promote_b.acknowledgment,
            "the recorded Acknowledgment (incl. shown_terms) must be Eq-identical across drivers"
        );
        assert!(
            !promote_a.acknowledgment.shown_terms.is_empty(),
            "sanity: this fixture's consent has real ConsentTerm rows, not a vacuous empty Vec"
        );
        assert_eq!(promote_a.filed_basis, promote_b.filed_basis);

        // ★ The TUI's own `rendered_b` (`btctax_cli::render_consent(&plan)`) is the SAME text the CLI
        // driver prints verbatim (I-1) — sanity-check it carries the shared consent-screen intro.
        assert!(
            rendered_b.contains("Promoting this tranche is a KNOWING choice"),
            "the TUI's rendered consent must be the real render_consent output, not a stub: {rendered_b}"
        );
    }

    // ── a Target refusal (unknown decision) bounces back with a reason, text preserved ──────────────

    #[test]
    fn unknown_target_refusal_bounces_back_with_a_reason() {
        let events: Vec<LedgerEvent> = vec![];
        let prices = StaticPrices::default();
        let mut state = PromoteFlowState::new(EventId::decision(999_999));
        attest_purchase(&mut state, &events, &prices);
        type_str(&mut state.part_ii, "cash P2P purchase, no records");
        state.review(&events, &prices, &cfg(), now());
        match &state.step {
            PromoteFlowStep::PartII { error } => {
                let e = error.as_ref().expect("an unknown target must refuse");
                assert!(
                    e.to_lowercase().contains("live")
                        || e.to_lowercase().contains("declaretranche"),
                    "the refusal should name the resolve-live gate: {e}"
                );
            }
            other => panic!("an unknown target must never advance past Part II: {other:?}"),
        }
        assert_eq!(
            state.part_ii.as_str(),
            "cash P2P purchase, no records",
            "the authored narrative must survive a Target-refusal bounce-back"
        );
    }

    // ── (c): a wrong ack phrase refuses, fail-closed, via persist_promote_tranche ────────────────────

    #[test]
    fn wrong_ack_phrase_refuses_fail_closed_and_records_nothing() {
        use btctax_core::persistence::load_all;

        let dir = tempfile::tempdir().unwrap();
        let pp = btctax_store::Passphrase::new("pw".into());
        let vault = dir.path().join("vault.pgp");
        btctax_cli::cmd::init::run(&vault, &pp, &dir.path().join("k.asc")).unwrap();
        let target = btctax_cli::cmd::tranche::declare_tranche(
            &vault,
            &pp,
            40_000_000,
            wallet(),
            date!(2020 - 01 - 01),
            date!(2020 - 01 - 10),
            now(),
        )
        .unwrap();

        let mut session = btctax_cli::Session::open(&vault, &pp).unwrap();
        let events = load_all(session.conn()).unwrap();
        let cfg = session.config().unwrap().to_projection();
        let mut state = PromoteFlowState::new(target);
        state.select_provenance(btctax_cli::ProvenanceKind::Purchase);
        state.attest_provenance(&events, session.prices(), &cfg, now());
        type_str(&mut state.part_ii, "cash P2P purchase, no records");
        state.review(&events, session.prices(), &cfg, now());
        let plan = match state.step {
            PromoteFlowStep::Consent { plan, .. } => plan,
            other => panic!("this fixture's review() must reach Consent: {other:?}"),
        };

        // A WRONG ack phrase (fail-closed — `apply_promote`'s own `require_promote_ack`, reached via
        // `persist_promote_tranche`, is the REAL gate; this is not a re-implemented compare).
        let err = crate::edit::persist::persist_promote_tranche(
            &mut session,
            *plan,
            Some("the wrong phrase"),
            now(),
        )
        .expect_err("a wrong ack phrase must refuse");
        match err {
            crate::edit::persist::PersistError::ResidueLive(_) => {
                panic!("a wrong ack phrase must never leave residue: {err:?}")
            }
            crate::edit::persist::PersistError::NoChange(_)
            | crate::edit::persist::PersistError::RolledBack(_) => {}
        }
        drop(session);

        let s2 = btctax_cli::Session::open(&vault, &pp).unwrap();
        let count = load_all(s2.conn())
            .unwrap()
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::PromoteTranche(_)))
            .count();
        assert_eq!(
            count, 0,
            "a wrong ack phrase must record NOTHING (fail-closed) — no PromoteTranche appended"
        );
    }

    // ── grep guard: this module never calls apply_promote directly (C-3) ───────────────────────────

    #[test]
    fn promote_flow_never_calls_apply_promote_directly() {
        // Token constructed at RUNTIME (mirrors KAT-G1's own self-check convention) so this assertion's
        // own source line does not itself contain the literal forbidden token.
        let forbidden = format!("{}(", "apply_promote");
        let src = include_str!("promote_flow.rs");
        assert!(
            !src.contains(&forbidden),
            "promote_flow.rs must COLLECT input + read plan_promote/render_consent only — the write \
             goes through edit::persist::persist_promote_tranche (C-3/KAT-G1)"
        );
    }
}
