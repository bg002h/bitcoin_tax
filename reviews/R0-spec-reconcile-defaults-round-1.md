# R0 — SPEC_reconcile_defaults.md — round 1 (independent architect)

**Artifact:** `design/SPEC_reconcile_defaults.md` (DRAFT). **Baseline:** branch `feat/reconcile-defaults`
@ `3136b89` (main == `b976621`). **Bar:** 0 Critical / 0 Important. **Mode:** read-only; no implementation.
**Method:** citations verified against current source; test blast radius measured EMPIRICALLY in an isolated
worktree (applied the intended flips, ran `cargo test --workspace`, captured the failing set).

## Verdict

**2 Critical / 4 Important / 3 Minor / 2 Nit**

The spec's *intent* is sound and the two code changes are each roughly one line. But the **Change-1 site
enumeration is materially wrong**: two of the four cited "default fallback" sites are **test-fixture literals**,
and the **two sites that actually determine the tax outcome are missing** — the forward-method default
(`fold.rs:41`) and the real CLI persisted default (`config.rs:26`). Implemented literally, the headline "FIFO→
HIFO globally" would NOT take effect for post-2025 disposals or for any real CLI vault. Separately, the
serde-default discussion is aimed at the wrong field and, if actioned as written, corrupts an *immutable*
safe-harbor record. These must be fixed before Plan.

---

## C1 [Critical] — Change-1 misses the two default sites that actually move tax numbers; two cited sites are TEST code

**The spec's four "flip these" sites (§Change 1):** `mod.rs:54`, `mod.rs:125`, `event.rs:487`, `persistence.rs:434`.

Verified against source:

- `crates/btctax-core/src/project/mod.rs:54` — `pre2025_method: LotMethod::Fifo` in `impl Default for
  ProjectionConfig`. **Real default**, but see C2: in the CLI path it is *shadowed* by `CliConfig::default()`.
- `crates/btctax-core/src/project/mod.rs:125` — `method: LotMethod::Fifo` in the `None` arm of
  `in_force_methods`. This is a **UI display helper** (`in_force_methods` feeds the tui-edit method-election
  screen), **NOT** the fold's tax-computation path. Flipping it changes what the screen *shows*, not what the
  gain *is*.
- `crates/btctax-core/src/event.rs:487` — `pre2025_method: LotMethod::Fifo` is inside
  `#[cfg(test)] mod tests` (module opens at `event.rs:360`), in `fn every_variant_serde_round_trips()`
  (`event.rs:381`). **It is a test-fixture value, not a default fallback.** Flipping it accomplishes nothing
  behavioral.
- `crates/btctax-core/src/persistence.rs:434` — `method: LotMethod::Fifo` is inside `#[cfg(test)] mod tests`
  (`persistence.rs:382`), in `fn load_all_ordered_returns_rows_in_ordinal_order()` (`persistence.rs:422`).
  **Also a test fixture, not a default.**

**The two production sites that actually change tax outcomes are ABSENT from the spec:**

1. **`crates/btctax-core/src/project/fold.rs:41`** — `.unwrap_or(LotMethod::Fifo)` inside `applicable_method`.
   Its own doc-comment (`fold.rs:30`) says: *"This is the ONLY method-resolution path in the fold."* For any
   **post-2025 disposal with no in-force election**, THIS is the default that selects lots and sets the gain.
   It is the single most important site for "default method FIFO→HIFO," and the spec does not list it. Grep
   confirms it is the *only* production `.unwrap_or(LotMethod::Fifo)` in the workspace.
2. **`crates/btctax-cli/src/config.rs:26`** — `pre2025_method: LotMethod::Fifo` in `CliConfig::default()`. The
   CLI builds its `ProjectionConfig` via `read_config()` → `CliConfig::default()` → `to_projection()`
   (`config.rs:82`, `35`), so `ProjectionConfig::default()` (mod.rs:54) is **never** used on the real CLI path.
   If `config.rs:26` is not flipped, **every real vault with no persisted `pre2025_method` key still defaults to
   FIFO** — the user-mandated change simply does not happen for actual users.

**Consequence if implemented as written:** flipping only the four cited sites leaves `fold.rs:41` = FIFO (all
post-2025 unelected disposals stay FIFO) and `config.rs:26` = FIFO (all real vaults stay FIFO), while the
tui-edit screen (mod.rs:125) misleadingly shows "HIFO." The feature would be simultaneously broken and
self-inconsistent.

**Fix — the correct production default-site set (flip Fifo→Hifo):**
- `crates/btctax-cli/src/config.rs:26` (CLI persisted default — the real user-facing default)
- `crates/btctax-core/src/project/fold.rs:41` (forward/post-2025 computation default)
- `crates/btctax-core/src/project/mod.rs:54` (core `ProjectionConfig::default`)
- `crates/btctax-core/src/project/mod.rs:125` (`in_force_methods` UI — for consistency with the above)
- **Do NOT** touch `event.rs:487` / `persistence.rs:434` (test fixtures — leave, or note they are expected-value
  updates, not "default flips"). **Do NOT** touch the enum `#[default]` — see C2. **Do NOT** touch
  `pools.rs:63` (verified deliberate FIFO mechanic — §check-2 below).

---

## C2 [Critical] — serde-default: the spec targets the wrong field; flipping it silently rewrites an IMMUTABLE safe-harbor record

The spec (§Change 1, "[serde back-compat]") cites `event.rs:187` and frames the risk as *"a stored
config/profile serialized WITHOUT an explicit method deserializes as HIFO."* Two problems, both material:

1. **There is no serde-default on the config path.** The CLI config is a key-value SQLite table
   (`cli_config`), read by `read_config()` with explicit `"fifo"|"lifo"|"hifo"` string matching and a
   `CliConfig::default()` fallback (`config.rs:82-111`) — **not** serde. `ProjectionConfig` derives nothing
   from serde for its method field either. So the "stored config deserializes as HIFO" scenario does not exist.
2. **The one `#[serde(default)] pre2025_method` is on `SafeHarborAllocation` (`event.rs:188`)** — and it is
   plain `#[serde(default)]`, i.e. it resolves to `LotMethod::default()` = the enum `#[default]` variant
   (`mod.rs:27` = `Fifo`). There is no `#[serde(default = "...")]` override (confirmed: no method-specific
   `serde(default=` in event.rs). This field is documented **IMMUTABLE**: *"the historical lot-consumption
   method … captured at attestation time and IMMUTABLE thereafter"* (`event.rs:183-187`); `universal_snapshot`
   conserves under THIS recorded value, not live config, and a mismatch fires `Pre2025MethodConflictsAllocation`
   (`transition.rs:29-32`).

**Consequence:** if the plan flips the enum `#[default]` from `Fifo` to `Hifo` to "move the serde-default layer"
(as the spec floats), then every pre-A.7 `SafeHarborAllocation` event that was persisted *relying on the serde
default* would, on next load, deserialize with `pre2025_method = Hifo` — **silently changing the method of an
already-attested, legally-irrevocable allocation**, flipping conservation and potentially firing a spurious
`Pre2025MethodConflictsAllocation`. That is a correctness/immutability violation, not a benign default change.

**Fix:** **Do NOT flip the enum `#[default]` (`mod.rs:27`); keep it `Fifo`.** All live defaults (C1's four
sites) are explicit `LotMethod::Fifo` literals, so flipping them to `Hifo` achieves the user's intent with
**zero** effect on the serde-default. Old safe-harbor records keep deserializing as FIFO (correct — their
method was captured when FIFO was the default). The spec's "should the flip be at the serde-default layer, the
projection-default layer, or both" question has a definite answer: **projection/default-literal layer only;
never the serde layer.** State this explicitly and delete the "[serde back-compat]" flip suggestion.

---

## I1 [Important] — Change-2: the `days(366)` formula offered by the spec does NOT guarantee long-term across a leap boundary

`§Change 2` offers two computations and says exact length is immaterial "the only requirement is guaranteed
long-term": (a) `replace_year(-1)` then `−1 day`, or (b) **`date − Duration::days(366)`**.

`is_long_term(acq, disposed) = disposed > one_year_after(acq)` (`conventions.rs:65-67`); `one_year_after` adds
one calendar year (Feb-29 → Feb-28) (`conventions.rs`). Long-term therefore requires
`one_year_after(acq) < date`, i.e. the disposal (which is `≥ date`) must be strictly after the 1-year
anniversary of `acq`.

**Counter-example for (b):** `date = 2020-03-01`. `date − days(366) = 2019-03-01` (the 2019→2020 window spans
the leap day, so it is exactly 366 days = one calendar year). `one_year_after(2019-03-01) = 2020-03-01 = date`.
Then `is_long_term(2019-03-01, 2020-03-01) = (2020-03-01 > 2020-03-01) = false` → **SHORT-TERM.** The formula
silently defeats the entire guarantee whenever the [acq, date] window contains a Feb-29.

`days(366)` is off-by-one in the unsafe direction. The safe day-count is **`days(367)`** (a calendar year is at
most 366 days, so `acq = date − 367` gives `one_year_after(acq) ≤ date − 1 < date` unconditionally — verified
for the same 2020-03-01 case: `acq = 2019-02-28`, `one_year_after = 2020-02-28 < date` → long-term). The
calendar `replace_year(-1) − 1 day` form is also correct (it is exactly "1 year + 1 day"), but only if the
Feb-29 `replace_year` failure is handled (e.g. `date = 2020-02-29` → `replace_year(2019)` errors).

**Fix:** drop `days(366)` from the spec. Mandate **`days(367)`** (simplest, leap-safe, no Feb-29 branch) OR the
calendar `replace_year(y-1)`-then-`−1 day` with an explicit Feb-29→Feb-28 fallback. Add a KAT with a
leap-crossing date (e.g. receipt 2020-03-01) that would go RED under `days(366)`.

---

## I2 [Important] — Change-2 decouples the long-term backdating from the only disclosure of it → a silent long-term estimate

The self-transfer-in advisory fires **only when `basis.is_none()`** (`fold.rs:1020`), but the `acquired_at`
backdating fires whenever `acquired_at.is_none()` (`fold.rs:1019`) — **independent** conditions. The CLI exposes
`--basis` and `--acquired` as **independent optional flags** (`cli.rs:290`, `cli.rs:292`). So
`classify-inbound-self-transfer --basis 500` (no `--acquired`) produces `SelfTransferMine { basis: Some(500),
acquired_at: None }` → basis advisory does **not** fire, yet the lot is **silently backdated ~1 year to
long-term** with no disclosure anywhere. (The pseudo path always carries `basis: None` (`resolve.rs:973`) so it
is covered by the advisory + `[PSEUDO]` taint; this gap is specific to the manual basis-supplied/date-omitted
path.)

A long-term-vs-short-term assumption directly changes the tax *rate* on a later sale; defaulting it invisibly is
exactly the kind of unflagged estimate the app's design (loud advisories, `[PSEUDO]` taint) exists to prevent.

**Fix:** the plan must disclose the backdated-acquisition estimate **independently of the basis advisory** —
e.g. fire a `SelfTransfer…` advisory (or extend the existing one) whenever `acquired_at` was defaulted,
regardless of whether basis was supplied. Add a KAT for the `(Some basis, None acquired)` case asserting the
disclosure is present.

---

## I3 [Important] — user-facing "short-term" text becomes factually wrong; spec does not schedule the updates

Change-2 inverts the holding-period default, but several **user-facing strings still say "short-term"** and are
not listed for update:

- **`fold.rs:1025-1026`** — the `SelfTransferInboundZeroBasis` advisory body: *"holding period also defaults to
  the receipt date = short-term"*. After Change-2 this is **the opposite of what the code does** and would
  mislead the taxpayer about the term of their own lot.
- **`cli.rs:285-286`** — the `--help` doc for `classify-inbound-self-transfer`: *"`--acquired` defaults to the
  receipt date (short-term)."* Also user-facing, also now wrong.
- Stale code comments (Minor, but fix together): `fold.rs:1019` "conservative receipt-date default";
  `resolve.rs:974` "defaulted receipt date (short-term)"; the `kat_tax.rs:2969-2970` "Invariant 3a … Short-Term"
  doc.

**Fix:** enumerate these in the plan's migration and correct them in lockstep with the code change (the two
user-facing ones are Important; the comments are cleanup).

---

## I4 [Important] — Test blast radius: EMPIRICAL enumeration (the make-or-break deliverable)

*(measured: isolated worktree at 3136b89, applied the C1-correct flips (config.rs:26, fold.rs:41, mod.rs:54,
mod.rs:125 → Hifo; enum `#[default]` left Fifo) + Change-2 (`fold.rs:1019` → `date.saturating_sub(days(367))`),
then `cargo test --workspace --no-fail-fast`.)*

**Result: 42 tests fail across 14 test binaries. Adapters: ZERO failures (the spec's "adapters rate-engine
KATs" guess is wrong). The dominant cluster is the OPTIMIZER suite (~20 tests) — which the spec does not
mention at all — because the optimizer's baseline is the default method (FIFO→HIFO shifts every
"beats-FIFO-baseline"/"proposed ≠ current" oracle). The tui / tui-edit suites (5 tests) are also unmentioned by
the spec.** Counts below are under a representative implementation (C1-correct default flips + Change-2 =
`date.saturating_sub(days(367))`); the exact total will shift as T1 chooses "add explicit FIFO election" vs
"update expected value" per test, but the affected SET is reliable.

### Category A — Change-2 (long-term self-transfer-in): 2–3 tests, need expected-value + intent updates
- `btctax-core/tests/kat_tax.rs`
  - `self_transfer_in_hp_defaults_to_receipt_date_short_term` (kat_tax.rs:2972) — **INVERTED**: rename to
    `…_long_term`, flip `Term::ShortTerm`→`LongTerm` (line 2989) and `acquired_at == d`→`== d − (1yr+1day)`
    (line 2988); rewrite the "Invariant 3a" doc (2969-2970). (This is the spec's `self_transfer_in_defaults_to_long_term` KAT — but it must REPLACE an existing inverted one, not be added alongside.)
  - `pre_2025_self_transfer_in_conserves_through_universal_pool` (kat_tax.rs:3380) — asserts
    `lots[0].acquired_at == 2024-06-01` (line 3403); observed fail `left: 2023-05-31, right: 2024-06-01`. Update
    the expected acquired date (pool keying unaffected; still Universal + LongTerm).
- `btctax-core/tests/pseudo_reconcile.rs` — `tax_total_computes_when_pseudo_clears_all_hard_blockers` (pseudo
  `$0` self-transfer lot; term/gain shifts with the backdate).

### Category B — Change-1 (HIFO default): ~39 tests
**B1. Direct default assertion (config):** `btctax-cli` lib `config::tests::default_pre2025_method_is_fifo_unattested`
(config.rs:220 — `assert !matches!(cfg.pre2025_method, Hifo)`). Update to expect HIFO.

**B2. Method-resolution FALLBACK (no/rejected/voided/backdated election → default), multi-lot:**
- `btctax-core/tests/method_election.rs` (5): `election_applies_on_or_after_effective_from_else_fifo`,
  `backdated_election_is_rejected`, `pre_transition_election_is_rejected`, `voided_election_is_excluded`,
  `relocated_older_lot_consumed_first_under_acq_date_fifo_diverging_from_insertion_order`. These assert
  "fallback → FIFO → lot A → basis 50"; now HIFO → lot B. Decide per test: add an explicit FIFO election to
  preserve the FIFO-intent ones, else update expected basis/lot.
- `btctax-core/tests/method_election_scoped.rs` (4): `per_wallet_method_governs_only_that_wallet`,
  `scoped_election_backdating_blocks`, `two_accounts_same_provider_independent`, `voided_scoped_election_falls_back`.

**B3. OPTIMIZER baseline shift (baseline == default method; the LARGEST cluster — UNMENTIONED by the spec):**
- `btctax-cli/tests/optimize_accept.rs` (9): `accept_attested_persists_and_upgrades_to_attested_recording`,
  `accept_persists_contemporaneous_without_attestation`, `accept_refuses_2027_broker_contemporaneous_divergent_no_write`,
  `accept_refuses_2027_broker_held_even_with_attestation`, `accept_refuses_already_executed_without_attestation`,
  `accept_then_divergent_baseline_stays_noncompliant`, `bulk_void_clears_attestation_for_lotselection`,
  `void_clears_attestation_row_prevents_mislabel_as_attested_recording`, `void_revokes_a_persisted_selection`.
  (Observed: "the optimizer must propose the dearer lot (a change from FIFO)" now `left == right` — baseline is
  already the dearer HIFO pick, so the "divergent proposal" fixtures no longer diverge.)
- `btctax-cli/tests/optimize_run.rs` (3): `optimize_run_contemporaneous_when_now_before_sale`,
  `optimize_run_needs_attestation_when_now_after_sale`, `optimize_run_saves_tax_and_writes_nothing`.
- `btctax-cli` lib: `session::tests::optimize_proposal_recomputes_a_persistable_proposal_on_held_session`.
- `btctax-core/tests/optimize_mode1.rs` (3): `hifo_beats_fifo_matches_oracle`, `loss_harvest_within_3k_limit`,
  `per_wallet_constraint_respected`.
- `btctax-core/tests/optimize_score.rs` (1): `high_basis_pick_lowers_tax_below_fifo_baseline`.
- `btctax-core/tests/optimize_wash_sale.rs` (1): `loss_lot_freely_selectable_no_wash_sale_bar`.
- `btctax-core/tests/safe_harbor_method.rs` (1): `path_b_seed_in_non_acq_order_consumes_oldest_first_under_fifo`.
  ⚠ These optimizer fixtures are engineered around a FIFO baseline (names literally say `…_below_fifo_baseline`,
  `hifo_beats_fifo…`). Migrating them is more than an expected-value bump — several need their **baseline
  re-pinned to FIFO via an explicit election** to keep testing "optimizer beats baseline," or the fixture intent
  re-designed. The plan must budget real design time here, not a mechanical sed.

**B4. Multi-lot term/fee/ordering under HIFO (core):**
- `btctax-core/tests/kat_tax.rs`: `self_transfer_fee_c_cross_lot_normal_survivor_stays_non_dual` (fee
  cross-lot survivor changes under HIFO; observed basis 112 vs 70 — a self-transfer-FEE test that breaks from
  the HIFO reorder, NOT Change-2).
- `btctax-core/tests/transition.rs` (4): `path_a_mixed_vintages_post_2025_term_and_conservation`,
  `path_b_preserves_gift_dual_loss_basis`, `path_b_preserves_gift_tacking`,
  `reversed_offset_straddle_seeds_on_tax_date_not_utc_order`.

**B5. TUI method display / election flow (UNMENTIONED by the spec):**
- `btctax-tui` lib: `tabs::tests::compliance_tab_shows_hard_advisory_partition_and_status`.
- `btctax-tui-edit` bin (4): `method_election_flow_sets_and_attests_per_account` (seeds/asserts resolved
  default == FIFO, main.rs:22550), `kat_e2e_sl_select_lots_happy_path_discriminating_seed`,
  `kat_e2e_sl_void_lot_selection_re_appears_in_list`, `kat_e2e_oa_z_attested_persists_lotselection_and_attest_row`.

**Migration guidance for the plan (T1):** (1) The optimizer cluster (B3) is the real cost centre and needs
design, not sed — several fixtures are defined *relative to the FIFO baseline*. (2) For B2, prefer adding an
explicit FIFO `MethodElection` where a test's PURPOSE is FIFO mechanics (keeps it meaningful) over blindly
re-pinning expected bases. (3) The `kat_tax.rs:2972` inverted KAT must be REPLACED (not duplicated) by the
spec's `_long_term` KAT. (4) Add the fault-injects the spec lists PLUS a leap-crossing Change-2 KAT (I1) and a
`(Some basis, None acquired)` disclosure KAT (I2). (5) Note the spec's "adapters rate-engine KATs" prediction is
empirically empty — remove it; add the optimizer + tui/tui-edit clusters.

---

## Minor / Nit

- **M1 [Minor]** — Ordering interaction (spec §"ordering interaction"): confirmed real. Under HIFO all
  `$0`-basis lots sort LAST via `hifo_cmp` (`pools.rs:275-279`), tie-broken by `acquired_at` ASC then `lot_id`
  (`pools.rs:284-285`); backdating a self-transfer-in `acquired_at` by ~1 yr makes it "older" and reorders it
  *within* the `$0` group. The spec already flags "KAT the order" — good; just ensure the KAT covers the
  `$0`-group tie-break specifically (two `$0` self-transfer lots, one backdated), not only real-vs-pseudo.
- **M2 [Minor]** — Pool/transition orthogonality holds under Change-2: `applicable_method` keys on the
  **disposal** date (`fold.rs:36`) and pool assignment keys on the **receipt/event** date, not on `acquired_at`
  — so backdating `acquired_at` across `TRANSITION_DATE` (e.g. a 2025-04 receipt backdated to 2024) does NOT
  mis-key the pool or reroute method logic. Worth an explicit KAT note; no code risk.
- **M3 [Minor]** — `pools.rs:63` (`consume_fifo` → `consume(.., Fifo, None)`) verified a **deliberate mechanic**
  (fee/PendingOut acquisition-date consumption, C1), NOT the electable method. Spec correctly says keep it FIFO.
- **N1 [Nit]** — Spec §Change 1 says "the 4 method-default sites" throughout (Scope, Plan T1). Reword to the
  corrected set (C1) so T1 does not implement the literal-but-wrong four.
- **N2 [Nit]** — Tax-soundness (§#5): defaulting to HIFO-unattested + long-term is defensible as a *flagged
  estimate* given the user mandate and the retained `attested: false` surface (`config.rs:27`, HIFO-on-return
  compliance note). The one substantive soundness gap is I2 (silent long-term). The basis stays `$0`
  (conservative on gain amount), so the net direction is "realistic, still not under-stating basis" — sound to
  ship once I2's disclosure lands.

## Bottom line
Not R0-GREEN. **2 Critical (C1 wrong/incomplete site set; C2 serde immutability hazard) + 4 Important** must be
folded, then re-review. The intent is fine and the diff is small; the danger is entirely in *which lines* and
in the untracked test migration + disclosure/text updates.
