# SPEC — Charitable/gift Chunk 3b: Form 8283 Section-B appraiser + structured-donee details

**Source baseline:** `origin/main` @ `114d6e0` (post Chunk 3a).
**Goal:** Let the user attach **Form 8283 Section-B** data (structured donee name/address/EIN + the
appraiser declaration) to a donation via a new `reconcile set-donation-details` command, stored in a
**side-table** keyed by the donation's `EventId`, and populate the previously-empty `Form8283Row`
appraiser/structured-donee fields (+ flip `needs_review` when details are present). Also resolves the
Chunk-1 `fmv_method` deferral via an optional user override.

FINAL piece of the charitable/gift completion cluster (Chunk 1 §170(f)(11)(F) + Chunk 2 donee/per-donee
709 + Chunk 3a §2505 all shipped). **Standalone** — pure Form 8283 metadata; does NOT feed
`compute_tax_year` / engine B / the projection.

**SemVer:** new side-table + CLI subcommand + additive `Form8283Row` fields ⇒ **MINOR** (pre-1.0). Additive;
the side-table DDL is `CREATE TABLE IF NOT EXISTS` (idempotent — old vaults gain it transparently).

## Legal grounding (R0 to spot-check)
- **§170(f)(11)(D) / Treas. Reg. §1.170A-16(d):** a contribution of property with a claimed deduction
  > $5,000 requires **Form 8283 Section B**: Part I (donated-property info), Part III (the qualified
  **appraiser's declaration**), Part IV (the **donee acknowledgment** — donee organization name/address/
  EIN). The appraiser declaration (Part III): appraiser name, address,
  identifying number (TIN), and (per §6695A) the appraiser's PTIN + a declaration of qualifications.
- **§170(f)(11)(E):** "qualified appraiser" — the qualifications description belongs on the form.
- **CCA 202302012:** crypto > $5,000 needs a qualified appraisal (no readily-valued exception) → Section B
  is the relevant regime for BTC donations that aggregate > $5k (§170(f)(11)(F), Chunk 1).
- This is FORM-COMPLETION data (metadata), not a tax computation — advisory/standalone.

## Current-state (recon @ 114d6e0)
- **Side-table precedents:** `tax_profile.rs` (keyed by year) and — the exact structural analog —
  `optimize_attest.rs` (`CREATE TABLE IF NOT EXISTS optimize_attestation (disposal_event TEXT PRIMARY KEY,
  …)` keyed by an `EventId::canonical()` string; idempotent DDL + defensive `init_table` guard on every
  access; initialized in `Session::from_fresh_vault` `session.rs:46`). `TaxProfile` the TYPE lives in
  `btctax-core`; its STORAGE (`tax_profile.rs`) lives in `btctax-cli` — the pattern to mirror.
- **`Form8283Row`** (`forms.rs:262-300`): `appraiser: String` always `""` (`forms.rs:402`); `needs_review:
  bool` always `true` (`forms.rs:403`); `donee: String` from `Removal.donee` label (Chunk 2); `fmv_method`
  section-derived (Chunk 1, Section A → `""`). `form_8283(state, year)` (`forms.rs:348`) iterates
  `state.removals` Donations in `year`.
- **Event keying:** a donation's `EventId` is the TransferOut import event id (e.g. `import|coinbase|out|
  TXID`); surfaced in `render_report` (`render.rs:274`) + removals.csv `event` column (`render.rs:671`);
  parsed via `parse_event_id` (`eventref.rs:24`) — the same ref the user already gives `reclassify-outflow`.
- **`write_form8283_csv`** (`render.rs:806-870`) builds + writes the rows.
- Standalone confirmed (`forms.rs:190-196`; does NOT feed engine B).

## Design

### D1 — `DonationDetails` type (core) + `donation_details` side-table (cli)
- **Type in `btctax-core`** (so `form_8283` can consume it — mirrors `TaxProfile`): `DonationDetails {
  donee_name: String, #[serde(default)] donee_address: Option<String>, #[serde(default)] donee_ein:
  Option<String>, appraiser_name: String, #[serde(default)] appraiser_address/appraiser_tin/appraiser_ptin/
  appraiser_qualifications: Option<String>, #[serde(default)] appraisal_date: Option<TaxDate>,
  #[serde(default)] fmv_method_override: Option<String> }` — `Serialize+Deserialize`, all optional fields
  `#[serde(default)]` (forward-compat). `donee_name` + `appraiser_name` are the only required fields.
  **[R0-M] Placement + derives:** a new `btctax-core` module (e.g. `donation.rs`) with
  `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`, re-exported from `lib.rs` (mirrors how
  `TaxProfile` is re-exported); + the `is_review_complete(section)` method (D3, I1). Keep it out of the
  fold/projection types (it never enters `LedgerState`).
- **Side-table in `btctax-cli`** (`donation_details.rs`, mirror `optimize_attest.rs`): `CREATE TABLE IF NOT
  EXISTS donation_details (donation_event TEXT PRIMARY KEY, details_json TEXT NOT NULL)`; `init_table`
  (idempotent, called in `from_fresh_vault` + defensively per access), `get(conn,&EventId)`,
  `set(conn,&EventId,&DonationDetails)`, `all(conn) -> BTreeMap<EventId,DonationDetails>`. JSON via
  serde_json (like tax_profile). Keyed by `EventId::canonical()`. Upsert (last-write-wins — appraisals get
  revised).
- `Session::donation_details(&self) -> Result<BTreeMap<EventId,DonationDetails>,CliError>` accessor
  (mirror `all_tax_profiles`); `init_table` added to `from_fresh_vault`.

### D2 — `set-donation-details` CLI command
New `reconcile set-donation-details <out_event_ref>` subcommand (`main.rs` + `cmd/reconcile.rs`):
`--donee-name` (required), `--donee-address`, `--donee-ein`, `--appraiser-name` (required),
`--appraiser-address`, `--appraiser-tin`, `--appraiser-ptin`, `--appraiser-qualifications`,
`--appraisal-date YYYY-MM-DD`, `--fmv-method`. Parse `<out_event_ref>` via `parse_event_id`; **[R0-M] validate against the PROJECTED `state.removals`** —
confirm the ref matches a `Removal{kind == Donation}` in the projected ledger (error "not a donation / not
found" otherwise), NOT by scanning the raw event log; build `DonationDetails`; `donation_details::set` +
`session.save()`. No `append_decision` / no `EventPayload` variant (side-table, not a decision). A
`show-donation-details <ref>` read variant. (Help text points users to removals.csv `event` column for the
ref.)

### D3 — `form_8283` consumes details + `needs_review` flip + fmv_method override
- `form_8283(state, year, details: &BTreeMap<EventId, DonationDetails>)` (add the param). On the carrier
  row, `let d = details.get(&r.event)`:
  - `donee` → `d.map(|d| d.donee_name.clone()).unwrap_or_else(|| r.donee.clone().unwrap_or_default())`
    (structured name preferred, else the Chunk-2 label).
  - `appraiser` → `d.map(|d| d.appraiser_name.clone()).unwrap_or_default()`.
  - `fmv_method` → `d.and_then(|d| d.fmv_method_override.clone()).unwrap_or(section_derived)` — **resolves
    the Chunk-1 Section-A `""` deferral** when the user supplies `--fmv-method`.
  - `needs_review` → **section-aware completeness [R0-I1]** — `d.map_or(true, |d|
    !d.is_review_complete(section))`. **[R0-m2]** `DonationDetails::is_review_complete(section:
    Form8283Section) -> bool` (NON-optional — the carrier row always carries a concrete Section; non-carrier
    legs never get a details lookup, so they keep `needs_review == true` by construction):
    **Section B** (the > $5k appraisal regime) requires the §170(f)(11)(D)/§6695A fields ALL present
    — `appraiser_name` + (`appraiser_tin` OR `appraiser_ptin`) + `appraisal_date` +
    `appraiser_qualifications` + `donee_ein` (donee_name/appraiser_name are already required by the
    command); **Section A** (≤ $5k, no appraiser required) is complete on presence (details present ⇒
    complete). So a SKELETAL `--donee-name X --appraiser-name Y` on a Section-B donation leaves
    `needs_review == true` (honest — the appraiser declaration is incomplete); do NOT flip to false on
    partial Section-B details. This upholds the "honest gaps, never fabricated" invariant.
  - Embed the full `details: Option<DonationDetails>` on the carrier row (for the CSV to flatten the extra
    Part III/IV fields — donee EIN/address (Part IV), appraiser TIN/PTIN/qualifications/appraisal_date
    (Part III) — without bloating the common `Form8283Row` fields).
- `write_form8283_csv` (cli): load `session.donation_details()`, pass into `form_8283`, and flatten the
  embedded details into new columns (donee_ein, donee_address, appraiser_tin, appraiser_ptin,
  appraiser_qualifications, appraisal_date) alongside the existing donee/appraiser/fmv_method columns.
  Header-named columns (stable ordering). All callers of `form_8283` updated to pass the map (an empty map
  where no CLI/side-table context — the core KATs pass a literal map).
- **Cross-crate ripple — `btctax-tui`:** the read-only viewer's Forms tab calls `form_8283(&snap.state,
  year)`. The new param breaks that call. Load the `donation_details` side-table into the TUI `Snapshot`
  (read-only — mirror `all_tax_profiles`, add `donation_details` to the Snapshot; the TUI is strictly
  read-only so `donation_details::all(conn)` via the immutable Session is fine) and pass it to `form_8283`,
  so the TUI Forms tab shows the appraiser/donee too (parity with the CSV). Keep the TUI read-only
  guarantee intact (no save/append/conn-write).

### Decisions
- **Side-table (NOT a decision)** — post-hoc pure-form metadata, zero projection effect; mirrors
  `optimize_attestation` (EventId-keyed, idempotent DDL, JSON). Trade-off vs a decision: last-write-wins
  upsert + no fold cost, at the cost of append-log audit history (acceptable — form completion is not a
  tax decision with rollback semantics).
- **`DonationDetails` type in core, storage in cli** — mirrors `TaxProfile`.
- **`fmv_method_override` in scope** — resolves the Chunk-1 Section-A deferral without any fold/schema
  change (user-supplied, honest).
- **`needs_review` is SECTION-AWARE [R0-I1]** — `!is_review_complete(section)`: Section B requires the
  full §6695A appraiser block + donee EIN; Section A is complete-on-presence. Partial Section-B details
  stay `needs_review == true` (honest gaps). (NOT the naive `details.is_none()`.)
- Standalone; no engine B.

## Plan (TDD)

### Task 1 — `DonationDetails` type + `donation_details` side-table + Session + `set-donation-details` CLI
- **Files:** `crates/btctax-core/src/…` (DonationDetails type + re-export), `crates/btctax-cli/src/
  donation_details.rs` (new — DDL/init/get/set/all), `crates/btctax-cli/src/session.rs` (accessor +
  init), `crates/btctax-cli/src/main.rs` + `cmd/reconcile.rs` (the command).
- Tests: `donation_details::set`/`get` round-trip (all fields incl. optionals); `init_table` idempotent on
  a pre-existing (old) vault (defensive-guard pattern — no error, table created); `all()` returns the map;
  `set-donation-details` on a real donation event stores it + `show-donation-details` reads it; targeting a
  NON-donation / missing event → a clear error; `--appraisal-date` parse (bad date → error);
  back-compat: an old vault with no `donation_details` table opens + returns an empty map.

### Task 2 — `form_8283` consumes details + `needs_review` flip + fmv_method override + CSV
- **Files:** `crates/btctax-core/src/forms.rs` (form_8283 param + carrier-row logic + embed), `crates/
  btctax-cli/src/render.rs` (write_form8283_csv loads + passes + flattens the columns).
- Tests (core KATs pass a literal `BTreeMap<EventId,DonationDetails>`):
  - **FULL Section-B details** (all §6695A fields) → `appraiser == appraiser_name`, `donee == donee_name`
    (structured), `fmv_method == override` (else section-derived), and **`needs_review == false`**.
  - **[R0-I1] SKELETAL Section-B details** (only `--donee-name` + `--appraiser-name`, a Section-B/aggregate
    > $5k donation) → `appraiser`/`donee` populated BUT **`needs_review == true`** (the appraiser
    declaration is incomplete — the honest-gap lock; MUST fail if the flip were unconditional).
  - **Section A with details** (≤ $5k) → `needs_review == false` on presence (no appraiser required).
  - **No details** → `appraiser == ""`, `needs_review == true`, `donee ==` the Chunk-2 label, `fmv_method
    ==` section-derived (Section A `""` unchanged).
  - The CSV carries the extra Part III/IV columns (donee EIN/address, appraiser TIN/PTIN/qualifications/
    appraisal_date) populated vs empty. Multi-leg: details on the carrier row only.

### Task 3 — whole-diff review (Phase E) + FOLLOWUPS + Chunk-3a nit sweep
- Cross-cutting: side-table mirrors optimize_attest (idempotent DDL, EventId key, back-compat on old
  vaults); the details are STANDALONE (engine B / compute_tax_year / the projection untouched — assert a
  tax golden unmoved); `needs_review` flip correct; fmv_method override resolves the Section-A deferral;
  the CSV columns stable/header-named; determinism (BTreeMap); privacy (the details contain donee/appraiser
  PII the USER enters — synthetic in tests; NEVER real names; the 0o600 + owner-only export still applies).
- **Sweep the Chunk-3a nits:** tighten KAT-B's weak `contains("0.00")` remaining-$0 assertion; move the
  `--prior-taxable-gifts` negative-validation so it isn't silently skipped outside `--tax-year`.
- FOLLOWUPS: the charitable/gift cluster is COMPLETE (all of Chunk 1/2/3a/3b shipped); a filled-PDF Form
  8283 (vs CSV); the append-log-audit alternative for donation details (deferred vs the side-table);
  donee-registry reuse across donations (a donee used repeatedly is re-entered — a future convenience).

## Out of scope
- A filled PDF Form 8283 (CSV only); an event-sourced (decision) donation-details variant (side-table
  chosen); a donee registry / reuse; validating EIN/PTIN formats (store as strings); the §2502 gift-tax
  liability; feeding any of this into engine B; 2026/2027 tables.
