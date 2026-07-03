# R0 spec review — `design/SPEC_tui_edit_chunk5.md` (safe-harbor-allocate CREATION flow)

**Reviewer role:** independent, adversarial architect. I did NOT author this spec.
**Baseline:** branch `feat/tui-edit-chunk5` @ `f31c1d6` (confirmed = current HEAD; every citation ground against this tree).
**Gate:** 0 Critical / 0 Important to pass.

## Verdict: **0 Critical / 0 Important / 1 Minor / 2 Nit — PASSES the R0 gate.**

The design is sound. All three load-bearing gotchas (G1/G2/G3) are correct against source. The residue helper is byte-identical to the CLI subset it factors out, the eligibility guards are a strict superset of the CLI, the persist template and payload are right, key `A` is free, and the attest/#7 interactions hold. No blocking finding. The Minor and Nits below are robustness/accuracy polish and do not gate implementation.

---

## Gotcha pressure-tests (all verified against source)

**G1 — voidability tracks EFFECTIVENESS, not attestation — CORRECT.**
`resolve.rs:924-934`: the `allocation_voids` loop fires `DecisionConflict` on `v.void_id` **iff** the void target is in the `effective` set. An allocation enters `effective` only after passing BOTH the `unconservable` (`:883-890`) and `timebarred` (`:891-898`) `continue` guards; attestation (`timely_allocation_attested`) feeds ONLY the `timebarred` computation (`:865`). So an effective allocation — attested or not — makes its void a `DecisionConflict`; an inert allocation (timebarred OR unconservable) is not in `effective` → its void applies cleanly. "Voidable iff inert" is exact. The #7 pre-filter (`main.rs:2550-2560`) encodes precisely this (`effective_alloc = SafeHarborAllocation ∧ ¬SafeHarborTimebar ∧ ¬SafeHarborUnconservable`, filtered out; inert stays listed). Spec framing ("voidable while inert", D4/D6) is correct and not misleading.

**G2 — every fresh allocation is timebarred at the current date — CORRECT.**
`resolve.rs:865-866`: `timebarred = ¬attested ∧ (made > bar ∨ method==ProRata)`. For `ActualPosition`, `bar = min_opt(first_2025_disposition, Some(TY2025_RETURN_DUE))`; `min_opt(None, Some(due)) = Some(due)` (`:980-985`), and `TY2025_RETURN_DUE = 2026-04-15` (`conventions.rs:19`). Today is 2026-07-03, so a fresh made-date (`now`) satisfies `made > 2026-04-15 ≥ bar` unconditionally (any `first_2025_disposition` is a 2025 date, only lowering `bar`). ProRata is timebarred whenever unattested via the `∨ ProRata` term. Hence every freshly-created unattested allocation today → timebarred → inert → voidable → status arm 3. Arm 4 (immediately-effective) is genuinely unreachable at the current date yet correct to keep (reachable only under an injected past `now`; deleting it would be wrong). Spec is right on both counts.

**G3 — ProRata not implemented; preview identical across the toggle — CORRECT.**
`safe_harbor_residue` depends only on `config.pre2025_method` (a `LotMethod`), never on `AllocMethod`. The `method: AllocMethod` toggle feeds only the recorded tag; in the engine it changes ONLY the `bar`/timebar rule (`resolve.rs:859-866`), never the seed lots (both methods seed from the same per-wallet actuals — `reconcile.rs:245-248`, O4). So the displayed lot table is byte-identical whether `ActualPosition` or `ProRata` is selected. The spec's directive that the modal must NOT imply ProRata redistributes basis is correct and matches core.

---

## Also-verify items

**(4) `Session::safe_harbor_residue` correctness + KAT-G1 cleanliness — CORRECT.**
The helper's subset filter (`EventId::Import` → `tax_date < TRANSITION_DATE`; else `¬matches!(SafeHarborAllocation)`), `config().to_projection()`, `project()`, and `residue.lots.filter(remaining_sat>0) → AllocLot` are **byte-identical** to the CLI command (`reconcile.rs:282-305`). `AllocLot` field mapping (`event.rs:150-160`: wallet/sat=remaining_sat/usd_basis/acquired_at/dual_loss_basis/donor_acquired_at) is exact. Uses `self.conn()` — no second `Session::open` (matches the `optimize_proposal` precedent, `session.rs:158-180`). The DRY refactor is behavior-preserving and pinned by `reconcile.rs` tests `:570`/`:733`/`:828`. At the TUI call site (`session.safe_harbor_residue()?`, `session.config()?`) none of the real KAT-G1 `persist_only_tokens` (`conn(` / `save(` / `append_` / `restore(` / `tax_profile::set` / `donation_details::set` / `optimize_attest::set`, `persist.rs:1224-1232`) appear — `config(` does not contain `conn(`. KAT-G1 stays green. (See Nit N1 re the spec's token *list*.)

**(5) Eligibility guards — CORRECT and a strict superset of the CLI.**
`pre2025_method_attested` gate mirrors `reconcile.rs:264-279` / `config.rs:14`. The "no existing live allocation" guard mirrors the attest opener's live-set construction (`main.rs:4679-4699`: voided-set then non-voided `SafeHarborAllocation`), correctly excludes voided priors (so allocate→void→re-allocate is not blocked), and the CLI has no such guard — the TUI is stricter, missing no CLI case.

**(6) persist + payload — CORRECT.**
`persist_safe_harbor_allocate` follows the `persist_reclassify_outflow` single-append template (`persist.rs:158-173`): `snapshot()` → `append_decision(conn, payload, now, UTC, None)` → `save_or_rollback`. A single append rolls back cleanly, so NO latch (unlike `persist_safe_harbor_attest`'s unrecoverable double-batch, `persist.rs:20-23`) is right. Payload fields (`as_of_date = TRANSITION_DATE`, `method` from toggle, `timely_allocation_attested = false`, `pre2025_method` from config) match `SafeHarborAllocation` (`event.rs:161-174`) and the CLI's own build (`reconcile.rs:311-321`, with `attested` fixed to `false` for creation). `derive_allocate_status` arms are keyed to `new_id`, mirroring `derive_attest_status` (`main.rs:4912-4946`); arm 3 (`SafeHarborTimebar`) is the expected outcome. (See Nit N2 on arm 2.)

**(7) Key `A` free; interactions — CORRECT.**
Browse dispatch (`main.rs:277-308`) binds only `G` among capitals; `A` is free (`a` = attest, `z` = optimize-accept). An `A`-created allocation is single/live/timebarred → it is exactly the attest opener's arm-6 input (`main.rs:4702-4754`) and is listed as voidable by #7 (inert). Modal/flow dispatch precedence (`handle_key`, `main.rs:120+`) and `close_all_mutation_surfaces` (`main.rs:506-532`) are the correct wiring sites; the spec explicitly commits to adding the two new fields there. The draw overlays model on `draw_safe_harbor_attest`/`draw_optimize_accept` in `src/draw_edit.rs` (present).

---

## Findings

### [M1] MINOR — DRY refactor splits the CLI's single `config()` read into two; recorded `pre2025_method` is not sourced from the read the residue used
`reconcile.rs:289` today reads `cfg` **once** and uses it for BOTH `project(&pre2025, …, &cfg)` and the payload's `pre2025_method: cfg.pre2025_method` — one read guarantees the recorded method equals the method the residue was computed under (the engine's conservation, `resolve.rs:872-882`, checks totals against `universal_snapshot` folded under `a.pre2025_method`). After the refactor, `safe_harbor_residue` reads config **internally** for the residue, while the command (and the TUI opener at D1 step 6) reads config **separately** to capture `pre2025_method`. In this codebase the two reads are provably equal (exclusive vault lock, single-threaded, no in-editor `pre2025_method` writer — `config.rs:123` is CLI-only, per G5), so it is behavior-preserving and cannot cause a conservation failure. But the coupling is now implicit, and the proposed KAT `safe_harbor_residue_matches_command_lots` pins only the LOTS — not that the recorded method matches the method those lots were derived under.
**Why it does not gate:** read-stability makes a divergence unreachable; no latent bug.
**Fix (robustness, either):** (a) have `safe_harbor_residue` also return (or accept) the `LotMethod` it projected under, making it the single source of truth for both lots and the recorded tag; or (b) add a one-line comment at both call sites tying the captured `pre2025_method` to the helper's config read, and/or extend the KAT to assert method-consistency.

### [N1] NIT — spec's KAT-G1 forbidden-token list is inaccurate (harmless over-statement)
D3/G6 describe the gate as forbidding "`conn(`/`load_all`/`project`/`append_`" in `btctax-tui-edit`. The actual `persist_only_tokens` (`persist.rs:1224-1232`) are `conn(`, `save(`, `tax_profile::set`, `append_`, `donation_details::set`, `optimize_attest::set`, `restore(` — `load_all` and `project` are **not** gated at all. The conclusion (the helper keeps the material tokens `conn(`/`append_` out of tui-edit) is correct; only the enumerated list is wrong. Correct the list so a future reader doesn't over-constrain.

### [N2] NIT — `derive_allocate_status` arm 2 reads the `event:None` "multiple effective" conflict, slightly outside the [R0-M10] "keyed only to new_id" discipline
D6 arm 2 checks `DecisionConflict` on `new_id` **OR** the `event:None` multiple-effective conflict (`resolve.rs:961-965`). `derive_attest_status` (`main.rs:4934`) keys DecisionConflict strictly to the new id. The broadening is defensive and cannot false-positive for the create flow (the step-5 guard forbids a second live allocation, and a fresh allocation is always inert, so `effective.len() ≥ 2` is unreachable), and because the target blocker is a *global* `event:None` conflict rather than a *stale per-allocation* blocker, the M10 stale-blocker hazard does not apply — so it is safe. Optional: note in the spec why this one deviation from "keyed only to new_id" is intentional and stale-free, to preempt a future reviewer flagging it.

---

## Citations spot-checked (accurate at `f31c1d6` unless noted)
`reconcile.rs` 26-34 / 240-241 / 245-248 / 250-323 / 264-279 / 282-305 / 306-310 / 311-321 ✓; tests 570/733/828 ✓ · `resolve.rs` 826-968 / 859-866 / 872-882 / 883-889 / 924-934 / 946-955 / 958-966 / 980-992 ✓ · `transition.rs` 32-72 / 75-103 ✓ · `event.rs` 145-174 ✓ · `conventions.rs` 17,19 ✓ · `session.rs` 90 / 137 / 158 ✓ · `config.rs` 14 / 123 ✓ · `main.rs` 277-308 / 415 (cycle_filing_status call site) / 506-532 / 2544-2560 / 4657 / 4712 / 4912 / 5259 / 5488 ✓ · `persist.rs` 158-173 / 1224-1232 ✓.
Only drift found: **`is_revocable_payload` is at `form.rs:841`, spec cites `:853`** (symbol correct, `SafeHarborAllocation` present — a 12-line drift, cosmetic).
