# UX-P4-1 implementation review — r1 (Fable, independent)

**Scope.** The COMPLETE UX-P4-1 change on `feat/post-v070-product-cycle` — commits `dbea745`
(surface 1: CLI delta banner + suffix), `3cd735f` (surface 4: fail-closed `--write-carryover` gate),
`3285a55` (surfaces 2+3: dual-report suffixes + TUI Tax tab) — against the GREEN contract,
`SPEC_post_v070_product_cycle.md` §3.1 + the §1 invariant. Review only; no code edited.

**Method.** Read all three diffs in full; verified every anchor in current source
(`state.rs:282 pseudo_active`, `resolve.rs:120-128` placeholder rung, `cmd/tax.rs report_tax_year`
/ `write_back_carryover`, `render.rs`, `btctax-tui/src/tabs/tax.rs`,
`btctax-cli/src/session.rs:489-520 resolve_all_screened`, `btctax-tui/src/unlock.rs build_snapshot`,
`btctax-tui-edit/src/draw_edit.rs:176`, `main.rs` exit mapping); swept for bypass writers
(`apply_carryover_writeback`, `return_inputs::set`, `render_tax_outcome`/`render_dual_report`
callers); statically mutation-checked each KAT; ran the full validation gate
(`make check`: **1970/1970 passed**, incl. `examples_golden_matches_committed`).

---

## 1. Conformance to SPEC §3.1 — verified, all four surfaces

| Clause | Where | Verdict |
|---|---|---|
| Predicate `pseudo_active() OR PseudoPlaceholder` | `cmd/tax.rs:307-313` (`PseudoDisclosure` in `render.rs:1013-1046`) | **Conforms.** Computed in `report_tax_year` AFTER `compute_tax_year`, from the LIVE projection (`load_events_and_project` uses the STORED config → pseudo-ON view, `session.rs:568-576`), and from the same `resolve_and_screen` provenance the figure used. Not a pseudo-OFF view. |
| Precedence (Synthetic wins / channels exclusive) | `cmd/tax.rs:307-313` | **Conforms — and is PINNED**: the surface-1 KAT fixture (unknown-basis lot, no stored profile) co-occurs both states (count>0 AND provenance `PseudoPlaceholder`); `assert_eq!(…, Synthetic)` reds a flipped precedence. |
| Channel-aware banner text, true per channel | `render.rs:1035-1046` | **Conforms** — both texts verbatim from §3.1; the placeholder text makes no `[PSEUDO]`-rows / verify-advisory claim (pinned by `pseudo_disclosure_helper_text_is_channel_correct`, incl. the negative asserts). Synthetic claims are live: `pseudo_synthetic_count > 0 ⇔ PseudoReconcileActive` (`state.rs:275-284`). |
| Surface 1 — delta banner + ` [PSEUDO]` suffix | `render.rs:1063` (banner leads), `:1115-1119` (TOTAL suffix, leading space kept per [T-N2]) | **Conforms.** |
| Surface 2 — dual-report suffixes | `render.rs:1285-1291` (L24), `:1308-1314` (Absolute TOTAL TAX) | **Conforms** — both filer-transcribed lines; suffix-only is right (the surface-1 banner leads the combined stdout; the dual block is provenance-gated to `ReturnInputs`, so the placeholder disjunct is structurally inert there, exactly as §3.1 notes). |
| Surface 3 — TUI banner + suffix, shared entry | `tabs/tax.rs:70-79` (banner), `:105-110` (suffix); editor routes through the same `tabs::tax::render` (`draw_edit.rs:176`) | **Conforms**; one change covers both TUIs. Trip-wire comment present at the exact edit site. |
| Surface 4 — `--write-carryover` refuse | `cmd/tax.rs:512-542`, before `apply_carryover_writeback` (`:564`) | **Conforms.** (4a) `pseudo_active()` — the `PseudoPlaceholder` disjunct is inert here because the `provenance != ReturnInputs` refuse (`:505-510`) precedes, exactly the §3.1 argument. (4b) `NotComputable` delta (computed over the same live events/state/profile the report path uses) refuses with the blocker named. `CliError` → exit 2 (`main.rs:39-47`), nonzero. |
| Fail-closed persistence | `write_back_carryover` | **Structurally fail-closed**: the DB is in-memory; disk persist happens ONLY at the tail `s.save()` (`:566`) — every refusal (and even the earlier `coherence_clear_or_refuse` draft mutation) is discarded on refuse. Plus pinned byte-compares in KATs (d)/(e). |
| No bypass | repo sweep | The ONLY production caller of `apply_carryover_writeback` is the gated `write_back_carryover` (`cmd/tax.rs:564`); the only `write_back_carryover` caller is `main.rs:181`. Other `return_inputs::set` writers (`income import` `cmd/tax.rs:101`, `answer.rs:212`, input-form commit `input_form_store.rs:299`) persist USER-authored data, not derived figures — out of the laundering class. |

## 2. §1 tax-figure invariant — holds

Render-only + refuse-only, confirmed at the diff level: the three commits touch format strings
(added `{}` suffix slots + a leading banner) and add refusal branches; no computation, rounding,
or persisted dollar value is touched. The surface-4 change adds one extra `compute_tax_year`
call (pure) on the gate path; the success path persists byte-identically to before. Live proof:
`examples_golden_matches_committed` (byte-compare of the whole J1–J6 corpus, zero `pseudo`
occurrences in `docs/examples/examples.md`) passes — SPEC KAT (a)(i) discharged.

## 3. Findings

| ID | Sev | Finding |
|---|---|---|
| I1 | **Important** | **SPEC acceptance KAT (b) is missing — the `PseudoPlaceholder` OR-disjunct is mutation-survivable.** §3.1 mandates KAT (b): "pseudo on, `count==0`, no stored profile → banner fires with the placeholder-variant wording (the false-negative + correct-text KAT)". Only the correct-text half exists (`pseudo_disclosure_helper_text_is_channel_correct` tests the enum in isolation). No test drives `report_tax_year` on the placeholder vault shape and observes `pseudo_contributed == Placeholder` or the rendered placeholder banner. Deleting the `else if provenance == PseudoPlaceholder` arm (`cmd/tax.rs:309-310`) — i.e., re-opening exactly the [G-I1] false-negative this predicate exists to close — leaves the whole 1970-test suite green: the surface-1 KAT is Synthetic-channel, the helper test never touches the wiring, and `tax_report.rs:127 pseudo_mode_injects_placeholder_profile_clearing_tax_profile_missing` has the EXACT vault shape but asserts only `Computed`. This violates the §3.1 mutation clause ("removing any surface's emit … reds its KAT" — surface 1's placeholder emit channel has no KAT). **Fix is test-only** (the code is correct): extend the existing `tax_report.rs:127` test (or add one beside the surface-1 KAT) with `assert_eq!(report.pseudo_contributed, PseudoDisclosure::Placeholder)` + a rendered-output assert on "synthetic $0 placeholder profile". |
| I2 | **Important** | **KAT (f) does not pin the enumeration invariant that licenses the TUI count-only signal.** §3.1 licenses `snap.state.pseudo_active()` alone on surface 3 ONLY by the invariant that `resolve_all_screened` enumerates `tax_profile::years ∪ return_inputs::years` (verified true today, `session.rs:497-498`; a placeholder profile can never reach `snap.profiles`), and says "KAT (f) pins it". The implemented `e7_no_profile_year_renders_not_computable_never_a_pseudo_number` hand-builds a `Snapshot` with an empty `profiles` map — it pins the render-layer consequence GIVEN the invariant, not the invariant: it never goes through `build_snapshot`/`resolve_all_screened`, and the specced "pseudo on" is not represented in the fixture at all. Consequence: if the enumeration ever regresses (e.g., `resolve_all_screened` grows CLI-parity enumeration of bare years, without anyone touching `tabs/tax.rs`), a `PseudoPlaceholder` profile lands in `snap.profiles`, the viewer renders an unflagged $0-placeholder number with `count==0` ⇒ no banner — the C2 Critical channel reborn — and **no test reds**; the guard is a comment. **Fix is test-only**: an integration KAT that opens a real pseudo-ON vault with nothing stored for the year, builds the snapshot via `btctax_tui::unlock::build_snapshot` (it is `pub`), and asserts the year is absent from `snap.profiles` / renders NOT COMPUTABLE with no `[PSEUDO]` figure. |
| M1 | Minor | SPEC KAT (a)(ii) not implemented as written: the pseudo-active fixture KATs assert banner/suffix presence but never pin the DOLLAR figures on surfaces 1/2/3 (no pinned value / "only banner+suffix lines inserted" assert). §1 is held structurally (render-only threading) and by (a)(i)'s byte-golden, so this is belt-and-suspenders — but the acceptance list said it. Pin the fixture's TOTAL values in the two pseudo KATs. |
| M2 | Minor | KAT (c)'s "no suffix, on all surfaces" is unpinned for surface 2 and for surface 3's Computed arm. `examples.md` contains no dual report (`TOTAL TAX (L24)` absent), and every pre-existing dual-report assert is `.contains(...)` (`tax_report.rs:1494/1507`), so a stuck-ON surface-2 suffix survives the suite; likewise a hardcoded TUI TOTAL-line suffix survives (`e7_no_profile…` catches only the banner via its NotComputable rendering; the mode-off Computed-tab suffix-absence is unasserted). Safe direction (over-flagging), hence Minor. Surface 1's (c)-direction IS pinned (examples byte-golden, `report --tax-year` present, zero `pseudo`); surface 4's false-refuse direction IS pinned (`the_full_remedy_chain_restores_a_computed_carryover`). |
| N1 | Nit | The TUI banner is a condensed variant of the §3.1 pinned synthetic text (drops the `'[PSEUDO]' rows in 'btctax report'` pointer; keeps ESTIMATE / not-filing-ready / a live `btctax verify` pointer). Every clause is true on its channel, and the §3.1 surface-3 clause mandates only threading the signal, not the verbatim text — recording the deviation for the whole-diff review. |
| N2 | Nit | Both CLI and TUI banners print above a NOT COMPUTABLE outcome (banner emitted before the outcome match), so a pseudo-on/hard-blocked year can read "figures shown are an ESTIMATE" over no figures. Over-disclosure — the safe direction; cosmetic. |

## 4. Answers to the four review questions

1. **Conformance:** yes on all four surfaces (table above); predicate live-state; banner texts
   channel-true; precedence correct and pinned.
2. **§1 invariant:** holds — render-only + refuse-only; no dollar touched; examples golden
   byte-identical (live-verified).
3. **KAT genuineness:** the KATs that exist are genuine (each names its fault-inject target and
   statically reds under it; the surface-4 pair also pins persist-nothing byte-identity, and the
   4a/4b refuse messages are uniquely matched). But two spec-enumerated KATs are missing or
   weaker than specced, leaving two guards mutation-survivable: the surface-1 **placeholder
   channel** (I1) and the **enumeration invariant** behind surface 3 (I2).
4. **Gaps/soundness:** TUI count-only signal is sound TODAY (invariant verified in source) but
   test-unpinned (I2). The write-carryover gate closes the only derived-figure persist path; no
   bypass found; fail-closed is structural (save-at-tail) plus pinned. No false-positive /
   false-negative channel found in the shipped code itself.

## 5. Verdict

**NOT GREEN — 0 Critical / 2 Important (I1, I2).** Both Importants are validation-surface gaps,
not code defects: the shipped implementation conforms to SPEC §3.1 on all four surfaces and the
§1 invariant holds. Discharge = two test additions (no product-code change expected); re-review
after the fold per STANDARD_WORKFLOW §2.
