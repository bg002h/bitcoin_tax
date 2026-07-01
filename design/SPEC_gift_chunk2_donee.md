# SPEC — Charitable/gift Chunk 2: donee identifier + per-donee Form 709 advisory

**Source baseline:** `origin/main` @ `3a405f0` (post Chunk 1).
**Goal:** Add a **donee identifier** to Gift/Donation events (the foundational event-schema change), populate
Form 8283's donee column from it, and refactor the Form 709 gift advisory to apply the §2503(b) annual
exclusion **PER DONEE** (not against the aggregate) with a Form 709 filing-required trigger.

Second of three chunks in the charitable/gift completion cluster (Chunk 1 = §170(f)(11)(F) aggregation
shipped; Chunk 3 = §2505 advisory + Section-B appraiser struct). **Standalone** — the donee is data; none
of this feeds `compute_tax_year` / engine B (the tax math is unchanged).

**SemVer:** additive event-payload field (`#[serde(default)]`, back-compat) + a new CLI flag + a refactored
advisory ⇒ **MINOR** (pre-1.0). No breaking change; existing vaults deserialize (donee → `None`).

## Legal grounding (R0 to web-verify)
- **§2503(b):** the gift-tax annual exclusion is **PER DONEE** — each recipient has their own exclusion
  ($18,000 TY2024, $19,000 TY2025, inflation-indexed per Rev. Proc. 2024-40 §2.43). Aggregating across
  donees is WRONG (the old advisory's flaw): two donees at $15k each is $0 taxable (each < $19k), even
  though the $30k aggregate exceeds one exclusion.
- **Form 709 filing trigger:** a return is required if the donor's gifts to **any single donee** of
  present-interest property exceed the annual exclusion for the year (also required for any future-interest
  gift, and for gift-splitting elections — those two the model can't detect; disclose).
- **§2513 gift-splitting (MFJ):** spouses may elect to treat gifts as made half-by-each, effectively
  doubling the exclusion. OUT OF SCOPE (single-filer model) — disclose as a caveat.

## Current-state (recon @ 3a405f0)
- **No donee anywhere.** `RemovalLeg`/`Removal` (`state.rs:149-176`) have no recipient field. Gift/Donation
  are created via the `reclassify-outflow` decision: `EventPayload::ReclassifyOutflow{ as_:
  OutflowClass::GiftOut | OutflowClass::Donate{appraisal_required}, … }` (`event.rs:104-116`) → CLI
  `reconcile reclassify-outflow --as-kind gift|donate --amount <fmv> [--appraisal]` (`main.rs:218-228`,
  `cmd/reconcile.rs:54-72`) → `resolve.rs:211-217` maps to `Op::GiftOut`/`Op::Donate` (`resolve.rs:72-86`)
  → `fold.rs` GiftOut (~958-1026) / Donate (~1028-1147) push `Removal`.
- **Precedent = `Income.business: bool`** (`event.rs:57`): a scalar in the decision-payload variant
  (`InboundClass::Income{business}`) → `Op::IncomeInbound{business}` → `IncomeRecord.business` → surfaced
  in income.csv. Mirror this for `donee`.
- **`render_gift_advisory`** (`render.rs:~1189` post-Chunk-1): sums `Σ
  fmv_at_transfer` over ALL `Removal{Gift}` legs in the year → a single AGGREGATE vs `gift_annual_exclusion`.
  No per-donee grouping. Explicitly says "donee identity is not modeled". `gift_annual_exclusion` TaxTable
  field exists (TY2025 $19k).
- `Form8283Row.donee: String` (`forms.rs:~288`) always `String::new()` (Chunk 1 populated `fmv_method`, not
  donee). `form8283.csv` already has the donee column (empty).
- Persistence: `EventPayload` serde round-trips through the vault; new fields need `#[serde(default)]` for
  existing-vault back-compat.

## Design

### D1 — donee identifier on Gift/Donation (event-schema)
Add `donee: Option<String>` (free-form label; a structured name/address/EIN is Chunk 3).
**[R0-C1] Put `donee` on the `ReclassifyOutflow` STRUCT, NOT on the `OutflowClass` variants.** Rationale:
`OutflowClass` is externally tagged and `GiftOut` is a UNIT variant (serialized as the bare string
`"GiftOut"`); converting it to a struct variant would fail to deserialize legacy `"GiftOut"` records
(unit-vs-struct-variant mismatch → the whole vault fails to open, since `load_all` propagates the first
`from_str` error). A field ADD to the `ReclassifyOutflow` struct is the proven back-compat pattern (JSON is
the format — `persistence.rs:165`/`290`; live precedents: `AllocLot.dual_loss_basis`,
`SafeHarborAllocation.pre2025_method`, and `ReclassifyOutflow`'s own `principal_proceeds_or_fmv`/`fee_usd`).
It also shrinks the diff (all `OutflowClass::GiftOut` sites stay byte-identical).
- `event.rs`: `ReclassifyOutflow { transfer_out_event, as_, principal_proceeds_or_fmv, fee_usd,
  #[serde(default)] pub donee: Option<String> }` — the new field on the STRUCT. `OutflowClass::GiftOut`
  (unit) and `OutflowClass::Donate { appraisal_required }` are UNCHANGED.
- `resolve.rs`: at the `OutflowClass::GiftOut → Op::GiftOut` / `OutflowClass::Donate → Op::Donate`
  mapping (~211-217), pass `ro.donee.clone()` into `Op::GiftOut { …, donee }` / `Op::Donate { …, donee }`
  (add `donee` to both `Op` variants, `resolve.rs:72-86`).
- `state.rs`: `Removal { …, pub donee: Option<String> }`.
- `fold.rs`: set `donee: op.donee` at both `st.removals.push(Removal{…})` sites (GiftOut + Donate).
- CLI: `reconcile reclassify-outflow --donee <LABEL>` (`main.rs`: `#[arg(long)] donee: Option<String>`;
  `cmd/reconcile.rs`: set `ReclassifyOutflow.donee`).
- `render.rs`: add a `donee` column to removals.csv (header + record).
NO change to tax math / engine B.

### D2 — Form 8283 donee column
`forms.rs` `form_8283`: populate `Form8283Row.donee` from `removal.donee` (carrier row;
`r.donee.clone().unwrap_or_default()`) instead of `String::new()`. Update the now-inaccurate doc/comment.
(Section-B donee name/address/EIN + appraiser remain Chunk 3.)

### D3 — per-donee Form 709 advisory
Refactor `render_gift_advisory(state, year, tables)`:
- Group `Removal{Gift}` legs by `donee` (the `Option<String>` label), summing `fmv_at_transfer` per donee
  for the year (labeled donees keyed by label; all `None`-donee gifts grouped as one "unlabeled" bucket).
- Per LABELED donee: `taxable_to_donee = max(0, donee_total − gift_annual_exclusion)`. If `donee_total >
  gift_annual_exclusion` → the donee triggers Form 709 (present-interest). Report each donee's total,
  applied exclusion, and taxable amount.
- **Filing-required trigger:** if ANY labeled donee's total > exclusion → "Form 709 filing required
  (donee(s): …)". Report total taxable gifts across donees (informational; the §2505 lifetime-exemption
  consumption is Chunk 3).
- **Unlabeled bucket:** if any `None`-donee gifts exist → a caveat: "N gift(s) totalling $X have no donee
  label — the §2503(b) annual exclusion is PER DONEE and cannot be applied without one; label them via
  `reconcile reclassify-outflow --donee`. Shown as a single conservative aggregate." (Keep the old
  aggregate-vs-single-exclusion signal for the unlabeled bucket so nothing is silently dropped.)
- Caveats: §2513 gift-splitting (MFJ) not modeled; future-interest gifts (which require filing regardless)
  not detectable; §2505 lifetime exemption is Chunk 3.
- **[R0-I2] Preserve the existing `render_gift_advisory` safety behaviors** when refactoring
  (`render.rs:~1189`): the `[R0-m6]` "gifts present but no bundled table → emit a note, NOT `None`" branch
  (~1210-1215) and the `any_gift`/no-gifts guard (~1194-1201) must survive the per-donee rewrite. Revise
  the now-stale `gift_advisory_tests` assertions (the `"donee identity is not modeled"` string at
  `render.rs:~1632` goes away — replace with the per-donee assertions).
- Still STANDALONE (render-time; does NOT enter `state.advisory`/engine B), matching the Chunk 1 advisory.

### Decisions
- **donee = free-form `Option<String>` label** (structured name/address/EIN = Chunk 3). `#[serde(default)]`
  for vault back-compat (existing gift/donate events → `None`).
- **Per-donee exclusion** (§2503(b)) — the correctness core; the old aggregate signal is retained ONLY for
  the unlabeled bucket (with a caveat), never silently dropped.
- **Donations** (charitable, §170) do NOT get the Form 709 treatment (that's Gifts, §2503); but Donations
  DO carry the donee (for Form 8283). The Form 709 advisory groups **Gifts only**.

## Plan (TDD)

### Task 1 — donee event-schema + CLI + removals.csv + back-compat
- **Files:** `crates/btctax-core/src/{event.rs,state.rs,project/resolve.rs,project/fold.rs}`,
  `crates/btctax-cli/src/{main.rs,cmd/reconcile.rs,render.rs}`.
- Implement D1. Tests: a `reclassify-outflow --as-kind gift --donee "Alice"` round-trips to
  `Removal.donee == Some("Alice")`; a donate with `--donee` too; NO `--donee` → `None`; **[R0-I1] back-compat: pin explicit LEGACY
  `ReclassifyOutflow` JSON strings that predate the donee field — one with `"as_":"GiftOut"` (the bare
  UNIT-variant string) and one with a legacy `Donate` map — neither containing `donee`; assert both
  `serde_json::from_str::<EventPayload>(...)` SUCCEED with `donee: None`** (the `#[serde(default)]`
  struct-field lock; this is the "existing vault opens" guarantee — the GiftOut unit string must still
  parse). removals.csv has the donee column populated/empty. Engine B / tax math unchanged (assert an
  existing tax golden unmoved).

### Task 2 — Form 8283 donee column
- **Files:** `crates/btctax-core/src/forms.rs` (+ the doc comment), `crates/btctax-cli/src/render.rs`
  (form8283.csv already has the column).
- Implement D2. Tests: a donation with `donee == Some("Charity X")` → `Form8283Row.donee == "Charity X"`
  (carrier row); `None` → empty; form8283.csv populated.

### Task 3 — per-donee Form 709 advisory
- **Files:** `crates/btctax-cli/src/render.rs` (`render_gift_advisory`).
- Implement D3. Hand-verified KATs (synthetic fixture; TY2025 exclusion $19k):
  - **Per-donee under exclusion (the key lock vs the old aggregate):** two donees "Alice" $15,000 +
    "Bob" $15,000 (aggregate $30,000 > $19k, BUT each < $19k) → NO filing required, $0 taxable (old
    aggregate rule wrongly flagged this).
  - **One donee over:** "Alice" $25,000 → filing required, taxable $6,000 (25,000 − 19,000) for Alice.
  - **Unlabeled bucket:** a `None`-donee gift $30,000 → the unlabeled caveat + the conservative aggregate
    signal (can't apply per-donee).
  - **Mixed:** "Alice" $25,000 (over) + unlabeled $5,000 → filing required for Alice + the unlabeled caveat.
  - **Donations excluded:** a `Removal{Donation}` does NOT appear in the Form 709 advisory (Gifts only).

### Task 4 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: the donee flows end-to-end (CLI→event→Op→Removal→8283/709/csv); **serde back-compat**
  (donee-less vaults still load → `None`); per-donee §2503(b) (NOT aggregate) is correct + the unlabeled
  bucket isn't silently dropped; filing trigger correct; STANDALONE (engine B/tax identity untouched;
  assert a tax golden unmoved); Gifts-only for 709, donee-on-both for 8283; exact Decimal; determinism.
- FOLLOWUPS: Chunk 3 (§2505 lifetime exemption + Section-B appraiser/structured-donee). Note §2513
  splitting + future-interest gifts remain unmodeled (disclosed).

## Out of scope
- §2505 lifetime exemption / unified credit (Chunk 3); structured donee name/address/EIN + Section-B
  appraiser (Chunk 3); §2513 gift-splitting; future-interest-gift detection; feeding any of this into
  engine B / `compute_tax_year`; real per-row FMV-method (needs FMV provenance on RemovalLeg — Chunk 3).
