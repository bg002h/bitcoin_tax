//! The Promote flow (Task 9, Phase P-C): select a tranche row (DFW-D12 — one tranche at a time; no
//! bulk-promote) → author the Form 8275 Part II narrative (an in-TUI **multiline** path, DFW-D12/M-2) →
//! drive the shipped `btctax_cli::plan_promote` chokepoint for the two-sided informed-consent screen
//! (`render_consent`) → a TypedWord acknowledgment gate (mirrors `PROMOTE_ACK_PHRASE`) → promote.
//! **C-3:** this module COLLECTS the Part II narrative + the typed ack phrase and READS
//! `btctax_cli::plan_promote`/`render_consent` — it never calls `btctax_cli::apply_promote` directly; the
//! WRITE goes through `edit::persist::persist_promote_tranche` (the ONLY caller of `apply_promote` in
//! this crate, mechanically enforced by `persist::tests::kat_g1_mechanized_source_gate`).

use crate::edit::form::FieldBuffer;
use btctax_core::price::PriceProvider;
use btctax_core::project::ProjectionConfig;
use btctax_core::{EventId, LedgerEvent};

/// Generous byte-length cap for the Form 8275 Part II narrative buffer (DFW-D12/M-2: "an in-TUI
/// multiline path") — larger than the donation free-text fields' `FREETEXT_CAP` (512), since a real
/// acquisition narrative may run to several sentences/paragraphs across multiple lines.
pub const PART_II_CAP: usize = 4096;

/// Which step of the flow is active.
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
    /// The in-TUI multiline Part II narrative buffer. Lives OUTSIDE `step` (not nested inside `PartII`)
    /// so a Consent→PartII bounce (Esc, or a refusal) PRESERVES the filer's authored text for further
    /// editing rather than discarding it.
    pub part_ii: FieldBuffer,
    pub step: PromoteFlowStep,
}

impl PromoteFlowState {
    /// Open the flow for `target`, at the PartII authoring step, with an empty narrative buffer.
    pub fn new(target: EventId) -> Self {
        Self {
            target,
            part_ii: FieldBuffer::with_cap(PART_II_CAP),
            step: PromoteFlowStep::PartII { error: None },
        }
    }

    /// Attempt to move from Part II authoring to the consent screen: runs `btctax_cli::plan_promote`
    /// FRESH over the caller's `events`/`prices`/`cfg` — provenance FIXED to `Purchase` (BG-D5 still
    /// runs, unmodified, as defense-in-depth: this flow only ever targets a DFW-D8 "$0, no acquisition
    /// record" declared tranche, which by construction has no OTHER real acquisition provenance to
    /// attest to — the SPEC's own "Other resolutions" section (M-1/M-2) names no provenance picker for
    /// this UX). `Ok` transitions to `Consent` (`render_consent(&plan)` + a fresh ack buffer); `Err`
    /// surfaces the refusal INLINE on the PartII step (BG-D5/BG-D3/BG-D7/`Refusal::Target`) — the
    /// filer's authored narrative is preserved on `self.part_ii`, never discarded, so they can revise and
    /// retry (mirrors the Declare flow's own "a refusal with a reason, not a silent append").
    pub fn review(
        &mut self,
        events: &[LedgerEvent],
        prices: &dyn PriceProvider,
        cfg: &ProjectionConfig,
        now: time::OffsetDateTime,
    ) {
        let part_ii_text = self.part_ii.as_str().to_string();
        match btctax_cli::plan_promote(
            events,
            prices,
            cfg,
            &self.target,
            btctax_cli::ProvenanceKind::Purchase,
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

/// The full Promote flow render — a pure derived text render (mirrors `render_declare_flow`'s own "pure
/// String builder" shape).
pub fn render_promote_flow(state: &PromoteFlowState) -> Vec<String> {
    let mut lines = vec![
        format!("Promote — tranche {:?}", state.target),
        String::new(),
    ];

    match &state.step {
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

    // ── constructor / render sanity ────────────────────────────────────────────────────────────────

    #[test]
    fn new_opens_at_part_ii_step_with_an_empty_buffer() {
        let state = PromoteFlowState::new(EventId::decision(1));
        assert!(matches!(
            state.step,
            PromoteFlowStep::PartII { error: None }
        ));
        assert!(state.part_ii.is_empty());
    }

    #[test]
    fn part_ii_step_renders_the_provenance_attestation_and_key_hints() {
        let state = PromoteFlowState::new(EventId::decision(1));
        let rendered = render_promote_flow(&state).join("\n");
        assert!(rendered.contains(btctax_cli::PROVENANCE_TEXT));
        assert!(
            rendered.contains("Tab"),
            "must hint the review key: {rendered}"
        );
    }

    // ── (b): an empty/whitespace Part II is refused (BG-D7) ────────────────────────────────────────

    #[test]
    fn empty_part_ii_is_refused_and_preserves_the_step() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
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
            PromoteFlowStep::Consent { .. } => {
                panic!("must NOT advance to Consent on an empty Part II narrative")
            }
        }
    }

    #[test]
    fn whitespace_only_part_ii_is_refused_bg_d7() {
        let (id, events) = tranche_events(date!(2020 - 01 - 01), date!(2020 - 01 - 10), 40_000_000);
        let prices = full_price_coverage(date!(2020 - 01 - 01), date!(2020 - 01 - 10));
        let mut state = PromoteFlowState::new(id);
        type_str(&mut state.part_ii, "   \n   ");
        state.review(&events, &prices, &cfg(), now());
        match &state.step {
            PromoteFlowStep::PartII { error } => {
                assert!(
                    error.is_some(),
                    "a whitespace-only (incl. multiline whitespace) narrative must refuse (BG-D7)"
                );
            }
            PromoteFlowStep::Consent { .. } => {
                panic!("must NOT advance to Consent on a whitespace-only Part II narrative")
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
            PromoteFlowStep::PartII { error } => {
                panic!("a valid narrative over a fully-covered window must advance to Consent: {error:?}")
            }
        }
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
        type_str(&mut state.part_ii, part_ii_text);
        state.review(&events_b, session_b.prices(), &cfg_b, now());
        let (plan_b, rendered_b) = match state.step {
            PromoteFlowStep::Consent { plan, rendered, .. } => (plan, rendered),
            PromoteFlowStep::PartII { error } => {
                panic!("this fixture's review() must reach Consent: {error:?}")
            }
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
            PromoteFlowStep::Consent { .. } => panic!("an unknown target must never reach Consent"),
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
        type_str(&mut state.part_ii, "cash P2P purchase, no records");
        state.review(&events, session.prices(), &cfg, now());
        let plan = match state.step {
            PromoteFlowStep::Consent { plan, .. } => plan,
            PromoteFlowStep::PartII { error } => {
                panic!("this fixture's review() must reach Consent: {error:?}")
            }
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
