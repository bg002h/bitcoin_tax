# R0 spec review — tui-edit chunk 4a (link-transfer + classify-raw), round 1

**Artifact:** `design/SPEC_tui_edit_chunk4a.md`
**Baseline:** `main` @ `755e47c` (HEAD verified = `755e47c`; all citations re-grounded against current source at review time).
**Reviewer:** independent adversarial architect (did NOT author this spec).
**Bar:** 0 Critical / 0 Important.

## Verdict: 0 Critical / 2 Important / 3 Minor / 1 Nit

The spine of the spec is sound. Every substrate anchor verifies: dispatch (`main.rs:109-262`, 8 modal layers → 8 flow arms → form → screen), `residue_latch_status` (`:409-427`), `on_persist_error` (`:434-452`, single `rollback_failed` site), free keys `l`/`u` (`:249-257` binds `p c o r f v s d a` only), the save-rollback persist shape (`persist.rs:75` `save_or_rollback`, `PersistError` `:33-44`), `events_by_id` (`:1878`), the `open_reclassify_outflow_flow` / `open_classify_inbound_flow` filter patterns, `is_revocable_payload` (`form.rs:840-854` — includes **both** `TransferLink` and `ClassifyRaw`), and the engine facts (`TransferTarget`/`TransferLink` `event.rs:93-102`; SelfTransfer projection `resolve.rs:201-216`; the three TransferLink `DecisionConflict` arms `resolve.rs:492-519`; ClassifyRaw dup `resolve.rs:410-419`; `is_imported` `event.rs:281-291`; `FIELD_CAP=64` `form.rs:18`; `BlockerKind::Unclassified` Hard `state.rs:29`/`fold.rs:1157`).

Two load-bearing correctness claims I pressure-tested and **confirmed true**:
- **Linking removes the out from the pending set.** `pending_reconciliation` is pushed ONLY in the `Op::PendingOut` arm (`fold.rs:729`). A TransferOut carrying a non-voided `TransferLink` projects to `Op::SelfTransfer` (`resolve.rs:201-216`) and is never pushed → after re-projection it disappears from `pending_reconciliation`; neither link-transfer nor reclassify-outflow re-offers it. No double-offer. ✓
- **save_or_rollback shape.** Both persist fns match the shipped single-append pattern (`snapshot → append_decision(conn, payload, now, UtcOffset::UTC, None) → save_or_rollback`); neither has a post-append fallible step like `persist_void`'s `optimize_attest::clear` (`persist.rs:283-287`), so neither needs the bespoke `rollback` helper or a batch latch. Spec is correct. ✓

The blocking findings are both in the **classify-raw scoped builder (D2)** and the **link-transfer wallet-target source (D1)** — the two places where the spec asserted field/enumeration facts without grounding them against the actual structs.

---

### [I1] IMPORTANT — D2 scoped builder: the specified sub-form fields don't match the imported payload structs
`design/SPEC_tui_edit_chunk4a.md:136-138` vs `event.rs:44-58`, `resolve.rs:187`, `resolve.rs:709-729`

D2 is called "the load-bearing decision," but its per-variant field lists were not grounded against the `Acquire`/`Income` structs, and the cited reuse (`validate_classify_inbound_income`) cannot produce the payloads.

**(a) Acquire — the severe part.** The spec says the Acquire sub-form collects **`(sat, usd_basis, acquired-at)`**. The actual struct is:
```rust
// event.rs:45-50
pub struct Acquire { pub sat: Sat, pub usd_cost: Usd, pub fee_usd: Usd, pub basis_source: BasisSource }
```
Three defects: (1) **`acquired-at` does not exist** on `Acquire` and is semantically inert — a `ClassifyRaw` replaces only the payload; the effective event keeps the *target's* `utc_timestamp`/`original_tz` (`resolve.rs:709-729`, `timeline.push(Eff{ utc: e.utc_timestamp, tz: e.original_tz, … })`), so the acquisition date is inherited from the original Unclassified row and any collected "acquired-at" is discarded. Collecting it would mislead the user into thinking they set a date that has no effect. (2) **`basis_source: BasisSource` is omitted entirely** — it is a required field with 8 variants (`event.rs:16-26`) and is tax-load-bearing (it labels how basis was derived; `ExchangeProvided` vs `ComputedFromCost` etc. flow into every downstream lot). The spec leaves the implementer to invent it. (3) `usd_basis` conflates the struct's two separate fields `usd_cost` + `fee_usd`. As written, an implementer cannot build a valid `Acquire` and must improvise a tax-load-bearing field.

**(b) Income — the secondary part.** The spec says the Income sub-form is `(sat, fmv, kind, business)` "reusing the classify-inbound income sub-form's field validation." But `validate_classify_inbound_income` returns `InboundClass::Income { kind, fmv, business }` (`form.rs:399-415`) — a *different* type with **no `sat` and no `fmv_status`**. To build `EventPayload::Income` (`event.rs:52-58`) the builder additionally needs `fmv_status: FmvStatus`, which the spec omits. This is load-bearing: `resolve.rs:187` computes `fmv = x.usd_fmv.filter(|_| x.fmv_status != FmvStatus::Missing)` — a wrong `fmv_status` **silently discards a supplied FMV** (or, harmlessly, keeps a `None` gated). The reuse claim covers the *field parsing* but not the payload construction.

**Why it gates:** D2 is the spec's explicitly flagged load-bearing call; its variant builders cannot be implemented as written for Acquire, and the Income reuse is insufficient. Both leave a tax-affecting field (`basis_source`, `fmv_status`) unspecified.

**Fix:** Re-derive both sub-forms from the actual structs:
- **Acquire:** collect `sat`, `usd_cost`, `fee_usd`; **drop `acquired-at`** (state that the date is inherited from the target event's timestamp); **specify `basis_source`** explicitly (e.g. fix it to `BasisSource::ExchangeProvided` or `ComputedFromCost` for a manually-classified raw acquire, and say which).
- **Income:** collect `sat`, `fmv`, `kind`, `business`, and **specify the `fmv_status` mapping** (`fmv=Some ⇒ ManualEntry`; `fmv=None ⇒ Missing`), noting `resolve.rs:187` is why it matters. Note the reuse is field-parsing only; a new validator returns `EventPayload::Income`.

---

### [I2] IMPORTANT — link-transfer wallet-target list is sourced from a set narrower than "any known wallet"
`design/SPEC_tui_edit_chunk4a.md:67-68` vs `fold.rs:1170-1187`

The spec's wallet-list = "keys of `snap.state.holdings_by_wallet` … Any known wallet is valid (no engine existence check)." The engine half is right (`resolve.rs:208`: `Wallet(w) ⇒ Some(w.clone())`, no existence check; CLI accepts any `--to-wallet`, `cli/main.rs:796-805`). But **`holdings_by_wallet` is not "known wallets" — it only contains wallets with a current positive balance:**
```rust
// fold.rs:1170-1187
for lot in pool { if lot.remaining_sat > 0 { *holdings.entry(lot.wallet.clone())… } }
st.holdings_by_wallet = holdings;
```
A wallet with zero current holdings — a fresh/never-funded cold wallet, or one fully drained — is **not a key**, so it is unofferable as a self-transfer destination. That is precisely the primary Wallet-target use case: reconciling a TransferOut that has *no* matching import by naming the destination wallet the coins moved to (which may hold nothing the engine knows about). There is no free-text fallback (the spec mandates "no free text"), so the TUI flow is strictly narrower than the CLI, and the spec's stated invariant ("Any known wallet is valid") is false for the offered set — corrupting the pre-filter reasoning and the review record.

**Why it gates:** a reachable, common reconciliation path (self-transfer into an empty/new wallet) cannot be completed in-TUI, and the spec justifies the source with a factually wrong completeness claim.

**Fix (pick one):** (a) source the wallet-list from a genuinely complete set — the union of `holdings_by_wallet` keys and all distinct `snap.events[].wallet` values (there is no dedicated wallet registry in core/tui; events carry `.wallet`, `event.rs:302`) — or (b) explicitly document the "positive-balance wallets only" limitation, correct the "any known wallet" claim, add a FOLLOWUP + a quit-first CLI-escape note for empty-destination wallets. (a) is the honest fix; (b) is the proportionate minimum.

---

### [M1] MINOR — in-list `wallet.is_some()` refers to the LedgerEvent's `.wallet`, not a TransferIn field
`design/SPEC_tui_edit_chunk4a.md:64-66`

"`snap.events` for `EventPayload::TransferIn` with `wallet.is_some()`" reads as a payload field, but `TransferIn` has no `wallet` (`event.rs:73-78`: `sat, src_addr, txid`). The engine's condition (c) checks the **event-level** wallet: `by_id.get(in_id).and_then(|e| e.wallet.as_ref()).is_none()` (`resolve.rs:509`). The intent is inferable from the `resolve.rs:509-519` citation and the `open_classify_inbound_flow` precedent (which reads `ev.wallet`, `main.rs:1960`), but state it as `LedgerEvent.wallet.is_some()` so an implementer doesn't hunt for a nonexistent payload field.

### [M2] MINOR — status arm-3 example blocker is unreachable for the scoped variants
`design/SPEC_tui_edit_chunk4a.md:148-150`

Arm 3 cites "e.g. `FmvMissing`/`UnknownBasisInbound` from the materialized payload." For the two scoped variants only `FmvMissing` is reachable (a materialized `Income` with `fmv_status==Missing`, `fold.rs:652`). `UnknownBasisInbound` fires only for inbound/`TransferIn` classification, never for a materialized `Income` or `Acquire` (`Acquire` always carries complete basis). Drop the `UnknownBasisInbound` example (or defer it to when the TransferIn variant is added).

### [M3] MINOR — the link-transfer `DecisionConflict` post-save arm is effectively unreachable, not "only reachable via failed-save race"
`design/SPEC_tui_edit_chunk4a.md:109-112`

The pre-filter prevents dup-on-`out_event` (a) and dup-on-`in_event` (b); the `wallet.is_some()` in-filter prevents (c); and the editor holds the vault's exclusive lock for its lifetime (`persist.rs:8-11` — "no concurrent-writer case"), so there is no "failed-save race." Keeping the defensive arm is fine (mirrors the other flows), but the justification is wrong — call it a defensive/should-be-unreachable arm rather than a race.

### [N1] NIT — `event.rs:79` citation for `EventPayload::Unclassified(Unclassified{raw})`
`design/SPEC_tui_edit_chunk4a.md:127`

The `Unclassified` struct is at `event.rs:80` (line 79 is its `#[derive]`); the `EventPayload::Unclassified` variant lives in the enum elsewhere. Cosmetic; re-anchor to `:80`.

---

## Pressure-test results (for the record)

1. **link-transfer pre-filter** — out-list source (`pending_reconciliation`, `.event`/`.principal_sat`, `state.rs:197-202`) matches `open_reclassify_outflow_flow` (`main.rs:2001-2024`) ✓; linking removes from the set ✓; in-list filter matches engine conditions (a)/(b)/(c) ✓ (see M1 wording); wallet-list source is **narrow** (I2). `TransferTarget`/`TransferLink` types ✓.
2. **classify-raw scope** — "Income+Acquire only, defer the rest" is defensible; `FIELD_CAP=64` real (`form.rs:18`) and a full imported-payload JSON is genuinely unusable in one field; Unclassified pre-filter correct (blocker→raw `Unclassified{raw}`, exclude non-voided `ClassifyRaw` dup) ✓; `ClassifyRaw.as_` requires `is_imported()` and Income/Acquire satisfy it ✓; materialized-blocker arm-3 real for Income. **But** the builder field lists are wrong/incomplete (I1).
3. **save_or_rollback shape** — both fns match the shipped pattern; no post-append fallible step; no bespoke latch needed ✓.
4. **key bindings / dispatch** — `l`/`u` free ✓; additive modal-layer + flow-arm extension is correct ✓.
5. **status / revocability** — `TransferLink` and `ClassifyRaw` both in `is_revocable_payload` (`form.rs:844,848`) → void works, no irrevocability warning ✓; both have `summarize_void_payload` arms (`main.rs:2394,2418`) ✓; status arms blocker/decision-id keyed ✓ (M2/M3 nits).
6. **KATs / completeness** — the KAT set (strict-prefix, cancel bytes-unchanged, `#[cfg(unix)]` save-error, validation, E2E incl. wallet+in-event targets, dup-link conflict, blocker-cleared, unsupported-variants-not-offered, KAT-G1 green) is sufficient. The under-specification is in the D2 builders (I1) and the wallet source (I2), not the test plan.

**Gate:** I1 and I2 are Important → **block implementation**. Fold both (correct the D2 field lists against the structs; correct/widen the wallet source and its claim), re-review to 0C/0I before Task 1 begins.
