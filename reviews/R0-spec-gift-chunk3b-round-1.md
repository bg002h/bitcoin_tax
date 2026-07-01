# R0 architect review — SPEC_gift_chunk3b_appraiser (round 1)

- **Artifact:** `design/SPEC_gift_chunk3b_appraiser.md`
- **Baseline verified against:** HEAD `114d6e0` (`Merge charitable/gift Chunk 3a: §2505 advisory-level lifetime exemption`)
- **Reviewer role:** independent architect (author ≠ reviewer)
- **Gate:** 0 Critical / 0 Important required before implementation.
- **Verdict:** **NOT green — 0 Critical, 1 Important (I1).** One blocking finding; exact fix below. All three Critical-class risks named in the mandate (side-table back-compat, engine-B leak, TUI read-only) **PASS**.

---

## Recon citation verification (all confirmed against HEAD `114d6e0`)

| Spec claim | Source | Status |
|---|---|---|
| `optimize_attest.rs` — `CREATE TABLE IF NOT EXISTS optimize_attestation (disposal_event TEXT PRIMARY KEY, …)`; `init_table` per-access + `set/get/all` keyed by `disposal.canonical()`; `all()` reconstructs via `parse_event_id` | `optimize_attest.rs:15-107` | ✅ exact analog; tableless-vault tests present (`get_on_tableless_vault_returns_none`, `all_on_tableless_vault_returns_empty`, `table_created_on_existing_conn_without_explicit_init`) |
| `tax_profile.rs` — type in core, JSON storage in cli, idempotent DDL, defensive guard, `all()` sorted | `tax_profile.rs:16-81` | ✅ |
| `TaxProfile` TYPE lives in core | `crates/btctax-core/src/tax/types.rs:31` | ✅ (spec's READ list said `tax/tables.rs+types.rs` — it is `types.rs`) |
| `optimize_attest::init_table` in `from_fresh_vault` | `session.rs:46` (tax_profile at :45) | ✅ |
| `all_tax_profiles(&self)` read accessor | `session.rs:86-90` | ✅ |
| `Form8283Row` struct | `forms.rs:261-300` | ✅ (spec says `262-300`; decl is line **261** — Nit N1) |
| `appraiser: String::new()` always | `forms.rs:402` | ✅ exact |
| `needs_review: true` always | `forms.rs:403` | ✅ exact |
| `donee` from `Removal.donee` on carrier | `forms.rs:397-401` | ✅ |
| `form_8283(state, year)` | `forms.rs:348` | ✅ |
| Standalone (no engine B) comment | `forms.rs:190-196` | ✅ verbatim |
| `write_form8283_csv` | `render.rs:806-870` (owner-only handle at `render.rs:812`) | ✅ |
| `parse_event_id` | `eventref.rs:24` | ✅ |
| TUI Forms tab calls `form_8283(&snap.state, year)` | `tabs/forms.rs:126` | ✅ |
| TUI `Snapshot` (`profiles: BTreeMap<i32,TaxProfile>` via `all_tax_profiles`) | `app.rs:103-109`, `unlock.rs:112-128` | ✅ |
| `TaxDate` = `time::Date`; serde enabled | `conventions.rs:10`; `btctax-core/Cargo.toml:12` (`features=["serde-well-known",…]`); serde structs already hold `TaxDate` (`event.rs:155,159,207`) | ✅ so `appraisal_date: Option<TaxDate>` serializes cleanly |

**No material recon drift.** Only Nit N1 (off-by-one line ref) and Nit N2 (Form-8283 Part labels).

---

## Mandate answers

### (a) Side-table back-compat on OLD vaults — **PASS (no Critical)**
An existing vault created before `donation_details` existed opens and reads cleanly:
- DDL is `CREATE TABLE IF NOT EXISTS donation_details (donation_event TEXT PRIMARY KEY, details_json TEXT NOT NULL)` — **idempotent**, exact mirror of `optimize_attestation`.
- `init_table` runs in `from_fresh_vault` **and** defensively at the top of every `get/set/all` — so an old vault missing the table gets it created **in-memory** on first access; `all()` returns an empty `BTreeMap`.
- The `optimize_attest` precedent proves the pattern with three passing tableless-vault tests; the spec's Task-1 test list reproduces them ("old vault with no `donation_details` table opens + returns an empty map"; "`init_table` idempotent on a pre-existing vault").
- End-to-end: with an empty map, `details.get(&r.event)` is `None` on every row ⇒ `form_8283` falls back to *exactly* today's Chunk-3a behavior (donee-from-label, `appraiser==""`, `needs_review==true`, section-derived `fmv_method`). **Zero behavior change for old vaults.**
- Keying: `EventId::canonical()` matches `optimize_attest` on write; the read side must reconstruct via `parse_event_id` (see M3 — spec should name it explicitly to lock the round-trip).

**No path by which an existing vault fails to open or a read errors.**

### (b) btctax-tui read-only guarantee — **PRESERVED (no Critical)**
- `attempt_open` binds `let session` (immutable — `unlock.rs:94-95`, `[R0-I1]` comment: "`let mut session` would make `save()` callable"). `Session::save()` takes `&mut self`, so it is **compile-uncallable** on the immutable binding.
- The spec's new `Session::donation_details(&self)` accessor takes `&self` (a read) and mirrors `all_tax_profiles(&self)` exactly; `build_snapshot` uses only typed read methods, never `session.conn()` directly (`unlock.rs:111-127`). Adding it does **not** weaken the immutable-binding guarantee.
- `donation_details::all` issues `CREATE TABLE IF NOT EXISTS` on the **in-memory** connection for an old vault, but `build_snapshot` never calls `save()` — so the encrypted vault **file bytes are unchanged**. This is identical to how `all_tax_profiles → tax_profile::all → init_table` already behaves, and it is guarded by the existing regression test `vault_file_bytes_unchanged_after_open_build_snapshot_drop` (`unlock.rs:465`), which will transparently cover the new accessor (see Nit N3).

**The compile-enforced read-only guarantee is intact: no `mut` Session, no `save`/`append`, no file write.**

---

## Findings

### CRITICAL — none.

### IMPORTANT

**I1 — `needs_review = details.is_none()` overstates completeness on a skeletal Section-B entry.**
The type makes every appraiser field optional (`appraiser_tin`, `appraiser_ptin`, `appraiser_qualifications`, `appraisal_date`, `donee_ein` are all `#[serde(default)] Option<…>`); only `donee_name` + `appraiser_name` are required. So `reconcile set-donation-details --donee-name X --appraiser-name Y` (the two required fields alone) produces a **Section-B carrier row with `needs_review == false`** — i.e. a positive "this form data is filled, no manual completion needed" signal — while the row is legally **incomplete** for Section B (§170(f)(11)(D) + §6695A require the appraiser's identifying number, appraisal date, and qualifications declaration; the donee acknowledgment needs the EIN).

This **reverses the codebase's explicit, repeated conservative invariant** — `forms.rs:294-299`, `Form8949Row.box_needs_review`, and the module docs all state `needs_review`/`box_needs_review` exist precisely to flag honest gaps ("honest gaps, never fabricated"). The spec's own justification — *"details present ⇒ the Section-B form data is filled"* — is unsound: **presence of the struct does not imply the Section-B-required fields are populated.** For a tax-compliance tool, a false "complete" is a soundness defect, not cosmetic.

**Fix (minimal, in scope):** make the flip section-aware completeness, not mere presence. E.g. add a predicate on the type:
```rust
impl DonationDetails {
    /// Section B is "review-complete" only when the §170(f)(11)(D)/§6695A fields a qualified
    /// appraisal requires are present; Section A needs no appraisal, so presence ⇒ complete.
    fn is_review_complete(&self, section: Form8283Section) -> bool {
        match section {
            Form8283Section::A => true,
            Form8283Section::B =>
                self.appraiser_tin.is_some()
                && self.appraisal_date.is_some()
                && self.appraiser_qualifications.is_some()
                && self.donee_ein.is_some(),
        }
    }
}
```
then `needs_review = d.map_or(true, |d| !d.is_review_complete(section))`. Add KATs: Section-B *skeletal* details ⇒ `needs_review == true`; Section-B *full* details ⇒ `false`; Section-A present details ⇒ `false`. (Keep the WITHOUT-details KAT asserting `needs_review == true` unchanged.)

### MINOR

- **M1 — DonationDetails module + derives underspecified.** D1 says only "`crates/btctax-core/src/…`". Pin it to `forms.rs` (co-located with `Form8283Row`) and add `DonationDetails` to the `pub use forms::{…}` re-export (`lib.rs:17-19`). Also state it must derive **`Clone, Debug, PartialEq, Eq`** in addition to `Serialize, Deserialize` — the spec embeds `details: Option<DonationDetails>` on `Form8283Row`, which derives `Clone, Debug, PartialEq, Eq`; those bounds propagate. (`TaxDate`/`String`/`Option` all satisfy `Eq` — fine — but the spec should say so.)
- **M2 — `set-donation-details` validation source.** "Validate the event is a Donation removal" should validate against the **projected `state.removals` (`kind == Donation`)**, not the raw `ReclassifyOutflow` event log — otherwise a *voided/superseded* donation would pass and leave an orphan side-table row that never surfaces in `form_8283` (which keys on `state.removals`). Low harm (metadata only), but pin the source explicitly. The `void` command (`reconcile.rs:104-143`) shows the load pattern; use the projection instead.
- **M3 — Read-side key reconstruction not stated.** Spec should say `donation_details::all` reconstructs the `EventId` via `parse_event_id` on the stored `canonical()` string (mirror `optimize_attest.rs:76`), and add an **import-keyed** round-trip KAT (cf. `optimize_attest` `eid_import` / `attested_set_reflects_all_stored_disposals`) so the `canonical()→parse` round-trip on real `import|…|out|TXID` refs is locked — the donation event id is an import id, not a `decision(seq)`.
- **M4 — `fmv_method_override` on Section B.** The override replaces the section-derived `"qualified appraisal"` default for Section B, not only the Section-A `""`. This is defensible (user may state a more precise method), but the spec frames it purely as "resolves the Section-A `""` deferral". State that a Section-B override *replaces* `"qualified appraisal"` so a later reviewer does not read it as erasing the Section-B signal; consider whether an empty `--fmv-method` should be rejected for Section B.

### NIT

- **N1 —** `Form8283Row` cited as `forms.rs:262-300`; the struct decl is line **261**. Trivial.
- **N2 — Form 8283 Part labels.** On Form 8283 (Rev. Dec 2023) **Section B**, the donee acknowledgment (name/address/EIN) is **Part IV**, and the appraiser declaration is **Part III** (Part II is the donor statement for ≤$500 items). The spec's legal grounding writes donee as "Part II/IV" and the flatten note as "Part II/III". Tighten the comments/column docs to Part IV (donee) / Part III (appraiser). Data model is correct and complete — this is labeling only.
- **N3 —** Name `vault_file_bytes_unchanged_after_open_build_snapshot_drop` (`unlock.rs:465`) in the Task-3 cross-cut as the standing read-only guard-rail for the new `donation_details` accessor (it already covers the in-memory `CREATE TABLE`, no file write).
- **N4 —** The Task-3 sweep item "tighten KAT-B's weak `contains("0.00")`" did not surface in `kat_forms.rs`/CLI tests via grep; verify the exact target exists at implementation time so the sweep is not phantom. (The other sweep item **is** grounded — see below.)

---

## Item-by-item (mandate 1-8)

1. **Side-table back-compat — PASS.** See (a). Idempotent DDL, defensive per-access `init_table`, `from_fresh_vault` init, `canonical()` keying, empty-map fallback = today's behavior. No open/read failure path.
2. **DonationDetails placement — sound.** Type in core, storage in cli mirrors `TaxProfile`. `form_8283` taking `&BTreeMap<EventId, DonationDetails>` is **no dependency inversion** — `EventId` + `DonationDetails` are both core types; cli builds the map from its side-table and passes it down (same clean direction as engine B taking `&TaxProfile`). Pin the module + derives (M1).
3. **TUI read-only ripple — PRESERVED.** See (b). Immutable-binding guarantee compile-enforced; `donation_details(&self)` is a read; no save/append; in-memory `CREATE TABLE` does not dirty the file.
4. **Section B fields — complete + correctly optional.** Donee name/address/EIN + appraiser name/address/TIN/PTIN/qualifications/appraisal-date is the right, complete set (PTIN captured for §6695A). Only `donee_name` + `appraiser_name` required. Part labels are loose (N2), not the data model.
5. **needs_review flip + fmv_method override.** Flip is **unsound as specified (I1)** — fix to section-aware completeness. `fmv_method_override` is user input (honest, not fabrication) and in scope; clarify Section-B replacement semantics (M4).
6. **Standalone / no engine-B leak — CONFIRMED.** `form_8283` is standalone (`forms.rs:190-196`); the new param touches neither `compute_tax_year`, the projection, nor the fold. **Signature-change ripple enumerated & complete:** (i) `crates/btctax-core/tests/kat_forms.rs` — **19** `form_8283(` call sites (pass a literal `&BTreeMap`); (ii) `render.rs:847` `write_form8283_csv` (load `session.donation_details()`, pass, flatten); (iii) `tabs/forms.rs:126` TUI (from Snapshot). Plus `lib.rs:17` re-export gains `DonationDetails` (a `pub use`, not a call). `Form8283Row` is constructed literally in **one** place only (`forms.rs:383`) — no test constructs it literally, so the additive `details` field does not ripple into KAT constructors. No other callers.
7. **PII — export path unchanged.** Details hold user-entered third-party PII (donee/appraiser names/addresses); vault is PGP-encrypted; `form8283.csv` is written via the same `fsperms::open_owner_only` 0o600 handle (`render.rs:812`) — **no new export path**, new columns land in the same owner-only file. Spec mandates synthetic-only test data; enforce at review time (no real names/addresses/EIN/PTIN).
8. **Scope / TDD / Chunk-3a sweep.** Right-sized as the final chunk; out-of-scope list is clear. TDD list covers back-compat-old-vault, set/get round-trip, needs_review flip, fmv override, standalone-golden-unmoved KATs — but the needs_review KATs must encode the **I1 fix** (skeletal vs full Section B). Chunk-3a sweep item **verified grounded**: `--prior-taxable-gifts` negative validation currently sits *inside* `if let Some(y) = tax_year` (`main.rs:402-406`), so a negative value is silently ignored when `--tax-year` is absent (the `else` at `main.rs:429`) — the sweep correctly moves it out. (Verify the `contains("0.00")` target per N4.)

---

## Required before green (re-review after fold)
- **I1** — section-aware `needs_review` completeness (+ KATs). *(blocking)*
- M1-M4, N1-N4 — fold as spec edits; re-review the folded spec (including the last fold) per §2.

*One Important open ⇒ gate not met. Fix I1, address the Minors/Nits, re-review to 0C/0I before any implementation.*

---

# Round 2 — re-review (post-fold)

- **Artifact (revised):** `design/SPEC_gift_chunk3b_appraiser.md`
- **Baseline re-verified against:** HEAD `114d6e0` (unchanged since round 1). Ripple counts re-checked live: **19** `form_8283(` call sites in `kat_forms.rs` + `render.rs:847` (`write_form8283_csv`) + `tabs/forms.rs:126` (TUI) — enumeration accurate.
- **Scope:** confirm the round-1 folds (I1 blocking + M1-M4/N1-N4). Per mandate, side-table back-compat (PASS) and TUI read-only (PASS, compile-enforced) were settled in round 1 and are **not** re-litigated.
- **Verdict:** **I1 CLOSED. 0 Critical / 0 new Important.** Gate met → **R0 GREEN.** Residuals are all Minor/Nit (below); two of them (R2-m1, R2-m2) touch the I1 mechanism itself and should be folded as spec-text edits before the `needs_review` predicate is coded — trivial, non-blocking.

## I1 — CLOSED (highest priority)

The fold replaces presence-completeness with **section-aware** completeness in D3 (spec lines 85-93):
`needs_review = d.map_or(true, |d| !d.is_review_complete(section))`, where Section B requires
`appraiser_name` + (`appraiser_tin` OR `appraiser_ptin`) + `appraisal_date` + `appraiser_qualifications`
+ `donee_ein`, and Section A is complete-on-presence.

- **Substance is sound.** The required set maps 1:1 onto §170(f)(11)(D)/§6695A: the appraiser's *identifying
  number* (the `TIN OR PTIN` disjunction is a legally-correct refinement of my round-1 `appraiser_tin`-only
  sketch — Form 8283 Part III accepts either as the "Identifying number"), the appraisal date, the
  qualifications declaration, and the Part-IV donee EIN. Section A (≤ $5k, no appraisal) correctly needs no
  appraiser fields.
- **The honest-gap invariant is restored on the signal-bearing row.** A skeletal
  `--donee-name X --appraiser-name Y` on a Section-B (aggregate > $5k) donation now yields
  `needs_review == true`. `is_review_complete` operating on the TYPE (re-asserting the two required fields
  belt-and-suspenders) is robust against deserialized data — good.
- **KATs lock it (Task 2):** FULL-Section-B → `false`; **SKELETAL-Section-B → `true`** (the honest-gap lock,
  fails-if-unconditional); Section-A-present → `false`; no-details → `true`. This is exactly the round-1
  prescription. The SKELETAL KAT is the regression backstop that makes an accidental `is_none()`
  re-introduction a test failure.

**The completeness overstatement is fully cured on the carrier row (where appraiser/donee/details live and
where the "form is complete" signal is read). "Honest gaps, never fabricated" holds.** I1 does not reopen.

## Confirmed folds (round-1 items)

| R1 item | Fold | Status |
|---|---|---|
| **M1** — placement + derives | Author chose a **new `btctax-core` module** (`donation.rs`) rather than co-locating in `forms.rs` — a legitimate author call; both satisfy the concern. `#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]` present (covers the `Option<DonationDetails>` embed's `Clone/Debug/PartialEq/Eq` propagation); re-exported from `lib.rs`; explicitly kept out of `LedgerState`/the fold. | ✅ |
| **M2** — validation source | D2 (lines 70-72) now validates the ref against the **projected `state.removals`** (`Removal{kind == Donation}`), NOT the raw event log — orphan-row path closed. | ✅ |
| **N2** — Form 8283 Part labels | Legal grounding (lines 20-21) corrected to Part I (property) / Part III (appraiser) / Part IV (donee); Task-2 (line 145) says "Part III/IV columns". **Partial** — see R2-n1: the D3 flatten note (line 95) still reads "Part II/III". | ⚠ partial |

## Residual findings (all Minor/Nit — non-blocking)

- **R2-m1 (Minor) — stale, now-contradictory Decisions bullet.** The Decisions section still carries
  `**needs_review = details.is_none()** — present ⇒ complete` (spec line 117) — the *exact reversed rule I1
  eliminated*. It directly contradicts the corrected D3 body (lines 85-93). The authoritative design body +
  the SKELETAL KAT are correct and backstop it, but a future reader consulting "Decisions" for rationale
  would find the unsound rule. This is the fingerprint of an incompletely-swept I1 fold. **Fix:** replace the
  bullet with the section-aware rule (or delete it). Highest-priority residual.
- **R2-m2 (Minor) — `is_review_complete` signature/None-arm under-specified.** The method is written
  `is_review_complete(section: Option<Section>)` (line 86), but the call passes the function-local
  `section` which is a non-optional `Form8283Section` (`forms.rs:354`); the row's Option-typed `section`
  field is `None` on non-carrier legs (`forms.rs:384`). The spec defines the A and B arms but **not the
  `None` arm**, and `needs_review` is computed per-leg. The coherent intent is `None → false` so continuation
  legs stay `needs_review == true` (today's behavior); state it (or keep the non-optional signature from my
  round-1 sketch and recompute only the carrier row). Either resolves it — but pin one before coding, so the
  I1-critical predicate isn't guessed. Carrier-row soundness is unaffected either way (hence Minor, not
  Important).
- **R2-n1 (Nit) — flatten-note Part label stale + now internally inconsistent.** D3 line 95 still says
  "Part II/III fields" for EIN/address (Part IV) + TIN/PTIN/quals/date (Part III); Task-2 line 145 correctly
  says "Part III/IV". Same columns, two labels → internal inconsistency. Tighten line 95 to "Part III/IV".
- **R2-n2 (Nit) — M4 not folded.** `fmv_method_override` is still framed purely as resolving the Section-A
  `""` deferral (lines 83-84, 116); the Section-B *replacement* of `"qualified appraisal"` (and whether an
  empty `--fmv-method` should be rejected for Section B) is not stated. Documentation-only.
- **R2-n3 (Nit) — M3 partial.** The `parse_event_id` read-side reconstruction of the `canonical()` key and
  an explicit *import-keyed* round-trip KAT aren't named; the `set-donation-details`/`show-donation-details`
  round-trip on a real donation event (line 130) covers it functionally. Low.
- **R2-n4 (Nit) — N1/N3/N4 not folded.** N1: line 34 still cites `forms.rs:262-300` (decl is 261). N3:
  `vault_file_bytes_unchanged_after_open_build_snapshot_drop` not named as the read-only guard-rail in
  Task-3. N4: the `contains("0.00")` sweep target still lacks the "verify-exists-at-implementation" caveat.
  All trivial.

## Item-by-item (mandate)

1. **I1 CLOSED** — section-aware completeness cures the overstatement on the carrier row; SKELETAL-Section-B
   KAT locks the honest-gap invariant. ✅ (residual R2-m1/R2-m2 are text/pinning hygiene, not a reopen).
2. **Minors/Nits:** M1 ✅ (new module, derives, re-export, out of `LedgerState`), M2 ✅ (projected
   `state.removals`), N2 ⚠ **partial** (legal grounding + Task-2 fixed; D3 line 95 stale → R2-n1).
3. **No new Critical/Important.** Spec is right-sized, standalone (no engine B — lines 11-12, 118), TDD-
   complete; side-table back-compat + TUI read-only intact (round-1 PASS); form_8283 ripple (19 KATs +
   `write_form8283_csv` + TUI) accurately enumerated. Only residual internal inconsistencies are the two
   stale-fold artifacts (R2-m1, R2-n1) + the None-arm gap (R2-m2), all Minor/Nit.

## Verdict

**I1 closed. 0 Critical / 0 new Important ⇒ the spec is R0 GREEN and may proceed to implementation.** Fold
the residual Minors/Nits as spec-text edits — do **R2-m1** (delete/replace the reversed `is_none()` Decisions
bullet) and **R2-m2** (pin the `is_review_complete` signature + `None` arm) before writing the `needs_review`
predicate, since both touch the I1 mechanism. None hold the gate; no further re-review round is required
beyond confirming these edits land.
