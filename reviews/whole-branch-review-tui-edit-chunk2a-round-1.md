# Whole-branch review — mutating-TUI chunk 2a (classify-inbound + reclassify-outflow) — round 1

**Branch:** `feat/tui-edit-chunk2a` @ `1db4d3b` (4 commits over base `096a07b`).
**Spec:** `design/SPEC_tui_edit_chunk2a.md` (R0 GREEN, 2 rounds).
**Reviewer stance:** independent; all implementer-report claims re-verified against source at
this HEAD (the Task-1 report's original green claim was false — see §7). Every targeted test
below was RUN by this reviewer at `1db4d3b`.

**Verdict: NOT READY TO MERGE — 0 Critical / 4 Important / 2 Minor / 7 Nit.**
All four Importants are small, localized fixes; none touches the write-safety core (pre-filters,
payloads, persist confinement, dispatch, and the strict-prefix/cancel/save-error KATs are sound).
Re-review required after the fold per §2 of `STANDARD_WORKFLOW.md`.

**Empirical baseline:** `cargo test -p btctax-tui-edit` at `1db4d3b` → **82 passed / 0 failed**
(includes KAT-G1, P2a/P2b, C2a/C2b, S2, S2-RO, all 8 E2E KATs, V-CI-1..9, V-RO-1..9, and the
full chunk-1 suite). Full workspace gate independently verified green by the controller.

---

## Findings

### Important

**WB-I1 — Both DecisionConflict statuses hand the user a malformed CLI remedy
(`void decision|decision|{seq}`).**
`derive_classify_inbound_status` (main.rs:1343–1349) and `derive_reclassify_outflow_status`
(main.rs:1412–1418) format:

```rust
"… clear with CLI: btctax reconcile void decision|{}", decision_id.canonical()
```

`EventId::Decision{seq}.canonical()` is already `"decision|{seq}"` (identity.rs:103), so the
rendered command is `btctax reconcile void decision|decision|N`. `cmd::reconcile::void` parses
its target with `parse_event_id`, which requires **exactly 2** `|`-parts for a decision ref
(eventref.rs:44–50) → the suggested command fails with `BadEventRef`. **Empirically confirmed**
via a temporary probe in KAT-S2 (reverted after):
`Some("Saved, but DecisionConflict fired on this decision — see Compliance; clear with CLI: btctax reconcile void decision|decision|2")`.
The spec's D2/D3 status strings mandate `void decision|{seq}`. The FmvMissing and
UnknownBasisInbound arms build the ref correctly from the raw `seq` (main.rs:1357–1364,
1376–1384) — only the two DecisionConflict arms are wrong. This is the one status whose entire
purpose is walking the user out of a Hard, tax-year-gating blocker (the [R0-I1] failed-save-retry
duplicate). Fix: drop the literal `decision|` prefix (use `canonical()` alone) or extract the seq
as the other arms do — in both functions.

**WB-I2 — KAT-S2 and KAT-S2-RO omit the spec-required post-retry status assertion (weakened
safety KAT).**
Spec §D5 KAT-S2 step 3, final bullet: "the post-persist `status` surfaces the conflict (the D4
step-2 check)"; repeated in the Task-3 cross-cuts ("KAT-S2 asserts … the status surfacing it").
Neither `kat_s2_save_error_path_classify_inbound_chmod` (main.rs:2729–2887) nor
`kat_s2_ro_save_error_path_reclassify_outflow_chmod` (main.rs:4158–4319) asserts `app.status`
after the successful retry — both assert only the `DecisionConflict` blocker in a fresh
re-projection and then drop the app. The status *does* functionally surface the conflict
(verified empirically, see WB-I1), but no test pins it — and this missing assertion is exactly
why WB-I1 shipped. Fix: assert `app.status` contains `"DecisionConflict"` **and** pin the
(corrected) remedy substring in both KATs.

**WB-I3 — KAT-E2E-RO weakened vs spec: the Disposal assertion dropped; CLI read-back steps
absent (E2E-RO and E2E-CI).**
Spec KAT-E2E-RO step 3: "a `Disposal` with `kind=Sell` and the specified proceeds appears in
`state.disposals`"; step 4: "CLI reads back via `cmd::inspect::report`."
`kat_e2e_ro_reclassify_outflow_sell` (main.rs:3720–3873) seeds a **lot-free** vault
(`seed_transfer_out_vault`), so the sell is uncovered and no Disposal can appear; the assertion
is explicitly waved off (`let _ = disposal; // present or not depends on lot coverage; not the
critical assertion`). The lot-seeded helper needed to honor the spec
(`seed_transfer_out_vault_with_lots`, main.rs:3496–3551) exists in the same file and is used by
the DONATE and GIFTOUT KATs. Net effect: **no test verifies the TUI sell/spend flow produces a
Disposal carrying the entered gross proceeds.** Additionally, neither E2E-RO's
`cmd::inspect::report` read-back nor E2E-CI's spec step 4 (`cmd::inspect::verify` read-back,
spec lines 884–885) is implemented (`cmd::` is test-region-legal; the tests already use
`btctax_cli::cmd::init::run`). Fix: re-seed E2E-RO on the lot-seeded helper (or add a second
covered-sell E2E) and assert the Disposal kind + proceeds; add the two CLI read-back steps.

**WB-I4 — Spec-mandated FOLLOWUPS records not written; the spec's own "Recorded in FOLLOWUPS"
citation is false at HEAD.**
Spec §Pre-filter (line ~147) states the [R0-M2] raw-vs-effective under-inclusion is "Recorded in
FOLLOWUPS", and §Plan Task 3 enumerates the chunk-2a records: chunk 2b (void flow retires the
CLI-void interim named in the D2/D3 statuses), chunk 3+ (link precedence check), [R0-M2] cheap
in-TUI fix, incomplete-gift rows, [R0-N3] negative-sign parity, [R0-N4] donee trim/cap
divergence, list-display polish. `FOLLOWUPS.md` at `1db4d3b` contains **none** of them (newest
entries are export-from-TUI / burndown-3, 2026-07-02); no Task-3 commit or report exists. Merging
now breaks the documented-limitation trail the spec's honesty story depends on (the D2/D3
statuses point users at a CLI interim path whose retirement is tracked only by these records).
Fix: write the FOLLOWUPS entries (include the WB-M1 wallet-None wording edge and the
superseded-TransferIn raw-`sat` display note, N7 below, while there).

### Minor

**WB-M1 — Wallet-less TransferIn: gift classification yields an Income-worded, wrong-remedy
status.**
fold.rs fires `FmvMissing` and returns early when `eff.wallet` is `None` in BOTH the
`Op::IncomeInbound` (fold.rs:829–839, "income inbound without wallet") and `Op::GiftReceived`
(fold.rs:891–901, "gift received without wallet") arms. A wallet-less TransferIn passes the
compound pre-filter (listed as "(no wallet)"); after a **GiftReceived** classification the
FmvMissing status arm renders `"Classified as Income(?) but FMV missing … re-classify with an
FMV"` (the `_ => "?"` kind fallback, main.rs:1365–1368) — wrong variant name, and the named
remedy (re-classify with an FMV) cannot clear a wallet-origin FmvMissing. Still blocker-derived
and truthfully alarming (no false success), so Minor, not Important. The Task-1 correction
record itself observed this rendering. Suggest a distinct wording for the gift case (or at least
a FOLLOWUPS record under WB-I4).

**WB-M2 — Browse footer keybindings omit the new `c`/`o` bindings.**
draw_edit.rs:142–144 still lists only `p: edit tax profile …`. The two new flows are
undiscoverable from the UI chrome; the chunk-1 precedent is that live bindings appear in the
footer. (The spec's D2/D3 define the bindings and D1 says the footer row "is shown throughout".)

### Nit

- **WB-N1** — KAT-P2a/P2b omit the literal `post[pre.len()].kind == "decision"` assertion from
  the spec formula. Equivalent coverage exists (`decision_seq` non-null + returned
  `EventId::Decision` equality; the schema sets `decision_seq` only on decision rows), so
  record-only.
- **WB-N2** — Stale `#[allow(dead_code)]` on `OutflowListItem` (form.rs:280) — fully used since
  Task 2.
- **WB-N3** — editor.rs:88–101 doc comments state a dispatch order (RO modal → RO flow → CI
  modal → CI flow) that differs from `handle_key`'s actual order (mutation modal → CI modal → RO
  modal → CI flow → RO flow, main.rs:94–122). Harmless under the at-most-one-`Some` invariant,
  but the citation should match the code.
- **WB-N4** — `draw_classify_inbound_form` defines `focus_style`/`normal_style` and silences them
  with `let _ =` (draw_edit.rs:428–431, 554–555) — dead styling vars.
- **WB-N5** — The spec'd C2a/C2b "complement: confirmed path writes" is not implemented as named
  complement tests; the E2E KATs' drop+reopen+`load_all_ordered` disk assertions are strictly
  stronger, so record-only.
- **WB-N6** — Report drift (record-only): the t2 report claims focus "skips 1→3 for
  Sell/Spend/Gift"; in code only Gift skips (Sell/Spend clamp at max focus 1,
  `max_focus_for_kind`). The t1 report's original false green claim is documented in its appended
  correction record and audited in §7 below.
- **WB-N7** — For the rare superseded-TransferIn (raw ≠ effective payload, same-type), the list
  and modal display the RAW `sat` (spec D1 prescribes raw-payload display, so spec-conformant);
  same family as the documented [R0-M2] limitation — name it in the FOLLOWUPS record (WB-I4).

---

## Verified clean (evidence per item)

### 1. Pre-filters — exact, and adversarially conflict-free

`open_classify_inbound_flow` (main.rs:1186–1269) implements the spec's compound filter exactly:
(1) `kind == UnknownBasisInbound`; (2) `Blocker.event` resolves to a raw
`EventPayload::TransferIn` in `snap.events`; (3) no non-voided `ClassifyInbound` targets it
(voided set = targets of all `VoidDecisionEvent`s, main.rs:1195–1220). Empty list → status, flow
not opened [R0-M8]. `open_reclassify_outflow_flow` (main.rs:1279–1325) sources
`pending_reconciliation` with no extra filter (Claim B).

**Adversarial result: no listed target in either flow can produce a `DecisionConflict` at
confirm.** Cases exhausted against resolve.rs at HEAD:

- **Voided-classify re-list:** resolve treats `VoidDecisionEvent` targets as non-revocable
  (resolve.rs:307–341 — a void-of-a-void fires `DecisionConflict` on the *second* void; the
  first void stays in force). Therefore, for `ClassifyInbound` targets (revocable class), the
  TUI's flat voided-set is EXTENSIONALLY IDENTICAL to resolve's voided-set — a TransferIn whose
  only classify is voided is re-listed, and the new classify inserts cleanly into
  `inbound_class` (resolve.rs:552–564; the voided one is skipped at :487). No conflict.
- **Already-classified (paths 2/3, incomplete gift):** filter 3 excludes; empirically pinned by
  KAT-E2E-GIFT-UNKNOWN's re-open assert (`c` → "No unclassified inbound transfers", flow not
  opened).
- **Non-TransferIn blocker sources (path 4, removal-consuming):** filter 2's payload check
  excludes (removal blockers point at outflow events).
- **Link-consumed TransferIn:** `Op::Skip` (resolve.rs:251–254) → blocker never fires → never
  listed. A FAILED link (in-event without wallet, resolve.rs:509–520) is not inserted into
  `links`/`consumed_ins`, so the TransferIn stays classifiable — the new classify passes
  type-validation and dedup (no prior map entry). No conflict.
- **Raw-vs-effective divergence:** effective-non-TransferIn targets never fire
  `Op::UnknownInbound` (build_op runs on the effective payload) → not listed; the reverse
  direction is the documented safe under-inclusion [R0-M2].
- **Outflows:** `pending_reconciliation` is populated only by the `Op::PendingOut` residual
  (build_op: links → `outflow_class` → PendingOut, resolve.rs:201–250), so a pending TransferOut
  by construction has no live `ReclassifyOutflow`/`TransferLink`; a voided reclassify is skipped
  at :487 and the new decision inserts first-wins-cleanly (resolve.rs:600–617). A pending
  target's effective payload is necessarily `TransferOut` (it reached the TransferOut build_op
  arm), so the type-validation arm cannot fire either.

The only conflict path remains the documented failed-save-retry duplicate [R0-I1] — asserted by
KAT-S2/S2-RO (run, pass).

### 2. Payloads + persist

`persist_classify_inbound` / `persist_reclassify_outflow` (edit/persist.rs:60–113) are exact:
`append_decision(conn, payload, now, UtcOffset::UTC, None)` then `session.save()`, returning the
`EventId` — mirroring `cmd::reconcile` with the held session. `now` is injected: exactly two
`now_utc()` sites exist in non-test code, both at modal Enter-press (main.rs:387, 842) — none
inside the persist fns. The payload builders map form state to the exact event.rs shapes
(`InboundClass::Income{kind,fmv,business}` / `GiftReceived{donor_basis,donor_acquired_at,
fmv_at_gift}`; `ReclassifyOutflow{transfer_out_event, as_, principal_proceeds_or_fmv, fee_usd,
donee}` with the four-way kind mapping, form.rs:381–618); KAT-P2a/P2b round-trip them from disk.
KAT-G1 (run, pass) confines the surface; the `persist_only_tokens` allowlist
(`conn(`/`save(`/`tax_profile::set`/`append_`) is **byte-identical** to base (diffed). The sole
non-test call sites of the two persist fns are the two modal Enter arms (main.rs:397, 852).

### 3. Dispatch order + quit containment

`handle_key` (main.rs:89–182): mutation modal → CI modal → RO modal → CI flow (Option-guarded)
→ RO flow (Option-guarded) → profile form → screen. Every step handler swallows unmatched keys;
`q` is swallowed at every flow step and both new modals (text-focus caveat: `q` inserts into the
focused buffer, per [R2-N1], and the C2a/C2b KATs backspace it out before submit). Esc steps back
exactly one step per step handler (form → picker → list → close). KAT-C2a and KAT-C2b (run,
pass) drive the full Esc-walk with per-step `!should_quit` + flow-still-open asserts and the
byte-identical vault check. No fall-through path to the Browse quit arm was found: the Browse
`match` is unreachable while any flow/modal is `Some`, and the flow guard is the Option, not the
step [R0-I2].

### 4. Statuses — re-projection-derived

`derive_classify_inbound_status` / `derive_reclassify_outflow_status` (main.rs:1335–1436) key
exclusively on the NEW `snap.state.blockers` (order: decision-id-attributed DecisionConflict →
target-attributed FmvMissing/UnknownBasisInbound (inbound) or UncoveredDisposal (outflow) →
clean). Run and passing at HEAD: KAT-E2E-FMV-MISSING (asserts "FmvMissing"+"void", asserts NO
"set-fmv" [R0-I4], asserts `basis_pending=true` lot); KAT-E2E-GIFT-UNKNOWN (re-fired UBI +
void remedy + re-list exclusion); KAT-E2E-GIFT-PRICE-GAP (1990-01-01 donor date → same UBI
status — proves blocker-derived, not shape-keyed [R0-I5]); KAT-E2E-UNCOVERED (**pre-state
UncoveredDisposal asserted first** [R0-M6], post-reclassify status contains "UncoveredDisposal",
pending entry gone). `DecisionConflict` is Hard by kind (state.rs `severity()`). The remedy
wording is void-then-re-classify only — except the WB-I1 malformed ref in the DecisionConflict
arms.

### 5. Modals — full payload

Classify-inbound modal (draw_edit.rs:559–628): target canonical + date + sat; Income: kind, fmv
(with "(empty = FmvMissing will fire)"), business; Gift: fmv_at_gift (REQUIRED tag), donor_basis,
donor_acquired_at, **both-donor-None WARNING** (draw_edit.rs:585–589; also a pre-write NOTE in
the gift form itself, :515–519). Reclassify-outflow modal (draw_edit.rs:835–923): target
canonical + date + principal_sat; sell/spend sections labeled **gross_proceeds** [R0-I3]; gift:
fmv + fee + **donee**; donate: fmv + fee + appraisal_required + **donee** [R0-I7]. Empirically
pinned: KAT-E2E-DONATE (appraisal true + donee rendered), KAT-E2E-GIFTOUT-DONEE (donee rendered
AND `appraisal_required` absent), KAT-E2E-CI (canonical + "staking" + FMV rendered), KAT-E2E-RO
("gross proceeds" label + canonical + amount rendered). All run, pass.

### 6. Safety KATs — run and verified

- **KAT-P2a/P2b** (edit/persist.rs:244–504): `post.len()==pre.len()+1`; full-`RawEventRow`
  prefix equality; `decision_seq == MAX(pre)+1` (the [R0-I6] MAX formula, asserted against a
  mixed import+decision seed where `pre` is non-empty); returned-id == `Decision{seq}`; payload
  round-trip; drop+reopen disk equality. Pass.
- **KAT-C2a/C2b**: full cancel walk, per-step q-swallow, byte-identical vault. Pass.
- **KAT-S2/S2-RO**: root-skip probe [R0-N5]; failure leg (modal closed, form intact, "Save
  error" status, bytes unchanged); retry leg (pre+2 decision rows on disk, both payloads
  identical, Hard `DecisionConflict` attributed to the RETRY id in the re-projection —
  first-wins). Pass. Gap: the post-retry status assert (WB-I2).
- No dedup/rollback "fix" crept into the persist fns (read line-by-line; doc comments state the
  [R0-I1] semantics).

### 7. Fix commit 1db4d3b — audited hunk-by-hunk

Exactly one hunk, entirely inside `mod tests`: `seed_transfer_in_vault` gains
`wallet: Some(WalletId::Exchange{River/main})` + comment (main.rs:2505–2516). **No flow,
persist, draw, or dispatch code touched; no assertion weakened** (the diff adds a fixture field
and 4 comment lines; nothing else changes). The commit-message rationale was verified against
fold.rs at HEAD and is accurate: both `Op::IncomeInbound` (fold.rs:829–839) and
`Op::GiftReceived` (fold.rs:891–901) fire `FmvMissing` and return early when `eff.wallet` is
`None`, so the original wallet-less fixture could never produce the lot/IncomeRecord the five
E2E classify-inbound KATs assert. The fix parallels the pre-existing documented requirement in
`seed_transfer_out_vault`. The t1 report carries an appended correction record. Legitimate
test-side fix; the trust-note incident is fully accounted for.

### 8. Substrate, scope, determinism, PII

- **Chunk-1 substrate byte-identical** (function-level diff vs `096a07b`): `persist_tax_profile`,
  KAT-G1 (scanner + all token lists), `handle_modal_key`, `handle_form_key`, `open_profile_form`,
  `validate`, `draw_mutation_modal` — all IDENTICAL.
- **Scope:** branch touches only `crates/btctax-tui-edit/**`, `design/`, `reviews/`, and a
  dev-dependency (`serde_json`, dev-only — no new workspace member, no new `[lib]` target;
  SemVer story intact). Viewer, core, and CLI crates untouched.
- **Determinism:** both lists sorted by date (stable sort over deterministic projection order);
  `now` injected at Enter; no other time/randomness sources in non-test code.
- **Synthetic-only:** all fixtures use synthetic identifiers ("River"/"test-ti-1"/"Alice"/
  "Community Foundation"); PII scans green per the controller's first-hand gate run.

---

## Required before merge (the fold list)

1. **WB-I1:** fix both DecisionConflict remedy strings (use `canonical()` bare or raw seq).
2. **WB-I2:** add the post-retry `app.status` asserts to KAT-S2 and KAT-S2-RO, pinning
   "DecisionConflict" and the corrected remedy substring.
3. **WB-I3:** re-seed KAT-E2E-RO on `seed_transfer_out_vault_with_lots` (or add a covered-sell
   E2E) asserting `Disposal{kind: Sell, proceeds}`; add the `cmd::inspect::report` /
   `cmd::inspect::verify` read-back steps to E2E-RO / E2E-CI.
4. **WB-I4:** write the chunk-2a FOLLOWUPS records (spec Task-3 list + WB-M1 + WB-N7).
5. Minors/Nits at author's discretion (WB-M2 footer bindings recommended alongside).

Then re-run the targeted KATs + full gate and submit for round 2.

---

# Round 2 — confirmation of the WB-I1/I2/I3 fold (commit `6bc053b`)

**Verdict: 0 Critical / 0 Important / 2 Minor / 7 Nit — READY TO MERGE**, with WB-I4 (the
chunk-2a FOLLOWUPS records) carried as an explicit **ship obligation owned by the controller at
merge** (excluded from this verdict per the coordinator's instruction; it remains mandatory —
the spec's "Recorded in FOLLOWUPS" citation is false until it lands).

All verification below performed first-hand at `6bc053b`; full crate suite re-run:
**82 passed / 0 failed** (includes KAT-G1).

**1. WB-I1 — FIXED.** Both DecisionConflict arms (main.rs:1346–1351, 1417–1422) now format
`"… btctax reconcile void {}", decision_id.canonical()` → exactly one prefix
(`void decision|N`), which `parse_event_id` accepts. Crate-wide grep: the only remaining
`decision|`-prefixed remedy strings are the FmvMissing/UnknownBasisInbound arms (raw `{seq}`,
correct single prefix, unchanged) and a persist.rs doc-comment notation; the only `decision|{}`
matches are the fix's own explanatory comments. No double-prefix pattern remains.

**2. WB-I2 — FIXED, and the new asserts are demonstrably load-bearing.** KAT-S2 and KAT-S2-RO
now capture `app.status` after the successful retry (before `drop(app)`) and assert it contains
`"DecisionConflict"` AND `format!("void {}", retry_id.canonical())`. The RED-against-pre-fix
reasoning is sound — the pre-fix string `"…void decision|decision|N"` does not contain
`"void decision|N"` as a substring (the prefix `"void decision|"` is followed by `d`, not the
seq digit, and `void` occurs exactly once) — and was **confirmed by mutation test**: this
reviewer temporarily restored the pre-fix `"void decision|{}"` format in both derive fns and
re-ran both KATs → both FAILED on exactly the new single-prefix assertion (failure output showed
`"…void decision|decision|2"`); reverted to `6bc053b` and both KATs pass. Run at HEAD: both pass.

**3. WB-I3 — FIXED.** `kat_e2e_ro_reclassify_outflow_sell` is re-seeded on
`seed_transfer_out_vault_with_lots` (covering Acquire, 500_000 sat) and now asserts spec step 3:
`Disposal` present for the target with `kind == DisposeKind::Sell` and summed leg proceeds
`== dec!(640.00)` (the entered gross; no fee ⇒ net == gross); the round-1 waving comment and
`let _ = disposal;` are gone — replaced by strict asserts (strengthening, no weakening). Spec
step 4 added: `cmd::inspect::report` read-back asserts the Sell Disposal from a fresh CLI
projection. KAT-E2E-CI gained its spec step 4: `cmd::inspect::verify` read-back asserts
`UnknownBasisInbound` absent from the hard-blocker list for the classified event. Both sessions
are dropped before the CLI calls (VaultLock released). Run at HEAD: both pass.

**4. Scope/weakening audit of `6bc053b` — CLEAN.** Single-file diff (main.rs, +102/−7). The
ONLY non-test changes are the two WB-I1 format strings (+ comments) inside the two derive fns;
every other hunk is inside `mod tests` (lines ≥ 2839). No assertion weakened anywhere — the only
deletions are the E2E-RO waiver comment/`let _ =` (replaced by stricter asserts) and the two
fixed format literals. KAT-G1 unaffected and green: the new `btctax_cli::cmd::inspect::*` calls
are test-region-only (the `cmd::` everywhere-rule scans non-test regions), consistent with the
pre-existing test-region use of `cmd::init::run`; the persist-only token allowlist is untouched.

**5. WB-I4 — SHIP OBLIGATION (controller, at merge):** write the chunk-2a FOLLOWUPS records
(spec Task-3 list: chunk 2b void-retires-CLI-interim, chunk 3+ link-precedence check, [R0-M2]
raw-vs-effective under-inclusion + cheap fix, incomplete-gift rows, [R0-N3] negative-sign
parity, [R0-N4] donee trim/cap divergence, list polish) plus the round-1 WB-M1 wallet-None
wording edge and WB-N7 raw-`sat` display note. Minors WB-M1/M2 and Nits WB-N1–N7 remain open at
author's discretion (WB-M2 footer bindings recommended).
