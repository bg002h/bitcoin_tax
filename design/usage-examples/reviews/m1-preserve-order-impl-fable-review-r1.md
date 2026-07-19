# Independent review — Phase 5 (Display), M-1: `serde_json` `preserve_order` workspace-wide

Reviewed against HEAD `7284d6f`. Re-audited the workspace source directly.

## Central risk (persisted/fingerprinted bytes) — CONFIRMED SAFE
- Fingerprints (`persistence.rs:25`): hand-rolled 0x1e-separated normalized Decimals, no JSON.
- Persisted event bytes (`persistence.rs:164-165`): typed `WalletId`/`EventPayload` — field-ordered regardless of preserve_order. No struct in core/store/adapters/cli holds a `serde_json::Value`/`Map` field; no `#[serde(flatten)]` on any persisted type.
- Side tables typed (tax_profile.rs:52, return_inputs.rs:92, input_form_store.rs:74, donation_details.rs:140).
- classify-raw typed end-to-end (reconcile.rs:460 from_str::<EventPayload>; stores the typed enum).
- Production Value-serialization sites: (1) income show display cmd/tax.rs:195-197 (never parsed); (2) oracle-harness json!→stdout (main.rs:156/179/317/321/468/668 via :99 — displayed/parsed, not stored/hashed); (3) input-form coverage to_value (coverage.rs:71 — cfg(test)); (4) update-prices parse-only (lib.rs:163/203 — cache is CSV). btctax-forms/xtask serde_json-free.

## Other verifications
- Consistency: all 6 serde_json crates flipped; isolated + workspace builds agree.
- Golden diff: one hunk, J6 income-show block only; −/+ multiset compare = pure permutation, identical values; matches ReturnInputs declared field order. §1 holds.
- KATs non-vacuous (flip byte-asserts insertion order; income-show pin distinguishes struct vs alphabetical via filing_status<capital_loss_carryforward_in).
- indexmap pre-existing transitive (2.14.0); no network; msrv 1.88 OK.

## FINDINGS
CRITICAL — none.
IMPORTANT
- I1 — The spec/plan-mandated blast-radius enumeration KAT was not delivered ("Pin that enumeration in the KAT"). The two delivered tests prove only that the feature is on + one surface's order; nothing pins the audited set of Value-serialization sites, so the safety claim lives only in a code comment — a future Value-serialization site feeding persisted bytes has no tripwire. Compounding: spec §4/§9.6's enumeration OMITS the oracle-harness json!→stdout sites (real Value serializations in a flipped crate). Fold: add a scan-style KAT (precedent: no_direct_now_utc_in_production) asserting the known production Value-serialization sites, with the corrected four-entry enumeration (income-show display / oracle-harness stdout / input-form coverage cfg(test) / update-prices parse-only); amend spec §9.6.
MINOR — none.
NIT
- N1 — "workspace_wide" KAT stays green if any one crate keeps the flag (feature unification); doc "Removing the feature from the serde_json deps reds this" accurate only as "from all deps". Harmless; the I1 scan is the right place for a stronger guard.

VERDICT: 1 Important (I1) to fold — deliver the mandated enumeration-pinning KAT + reconcile spec §9.6. 0 Critical; central risk clean (no persisted/fingerprint drift, golden is pure permutation, flip consistent).
