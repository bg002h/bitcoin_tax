# SPEC — pre-2025 filed-method reconciliation mechanism (Slug 3, Phase-2)

**Source baseline:** `origin/main` @ `c70922d` (post A→B→C + burndown-2; recon refreshed against this HEAD).
**Goal:** Give the **pre-2025 filed-method declaration** engine *teeth*, so a taxpayer's declared+attested
pre-2025 lot method (which already drives carryforward-basis reconstruction) is an explicit, attested
choice — never a silent default — and so an **irrevocable safe-harbor allocation can no longer lock in an
undeclared method**. North-star: the carryforward basis crossing into 2025 reflects the method the
taxpayer *actually filed under*, declared as an explicit attested choice (durably recorded in the ledger
for Path-B allocations; a durable Path-A declaration event is a deferred follow-up — see Decisions/I2).

**SemVer:** additive `ProjectionConfig` field + a command-time refusal precondition in
`safe-harbor-allocate` ⇒ **MINOR** (pre-1.0). No new `BlockerKind` variant (the allocate gate is a CLI
refusal, not a projection blocker). Backward-compatible: existing vaults default to
`pre2025_method=Fifo, attested=false` (which now makes the advisory louder and requires an explicit
attest before a safe-harbor allocation — see Backward-compat in Task 5).

## What already exists (recon @ c70922d — do NOT rebuild)

The basis-adjustment mechanism is **already complete and method-aware end-to-end**:
- `ProjectionConfig.pre2025_method: LotMethod{Fifo,Lifo,Hifo}` (`project/mod.rs:25-42`); set via
  `config --set-pre2025-method <m> --attest-pre2025-method` (`main.rs:59-61` → `cmd/admin.rs:29-39` →
  `config.rs:122-134`, persisted in `cli_config` as `pre2025_method` + `pre2025_method_attested`).
- Pre-2025 disposals consume the single `PoolKey::Universal` pool (`pools.rs:15-21`) under
  `applicable_method(date<TRANSITION → config.pre2025_method)` (`fold.rs:30-45`) → `consume_ordered` /
  `method_order` (`pools.rs:179-268`). So setting LIFO/HIFO **already changes which lots survive to
  2025-01-01 and their basis/HP** (the Universal residue is the complement of pre-2025 disposals).
- The 2025 boundary seed (`transition.rs:75-103`) carries that residue forward (Path A relocate /
  Path B `SafeHarborAllocation` direct declaration). The allocation captures the method immutably
  (`event.rs:162-168`, `reconcile.rs:290`); conservation uses the recorded method
  (`resolve.rs:638-675`); `Pre2025MethodConflictsAllocation` (Hard) enforces live-vs-recorded sync
  post-allocation (`resolve.rs:736-751`).
- Advisory `Pre2025MethodNote` (Advisory, `state.rs:33,73`) emitted once on the first pre-2025
  Dispose/GiftOut/Donate (`fold.rs:80-101`), surfaced in `verify` (`render.rs:982-989`).

## The gap (what this slug builds)

The **declaration has no behavioral teeth** (recon §6):
1. `pre2025_method_attested` is stored + displayed but **read by nothing in the engine** — it gates
   nothing, suppresses nothing.
2. `note_pre2025_once` is **attestation-blind** (`fold.rs` only receives `ProjectionConfig`, which lacks
   the attested flag) — it always says "verify against those filings" even after the user has declared
   and attested their filed method.
3. `safe-harbor-allocate` reads the **live** `cfg.pre2025_method` and records it permanently
   (`reconcile.rs:259,290`) **without checking attestation** — so a user who never declared silently
   commits the default FIFO into an irrevocable allocation.

## Design

### D1 — plumb the attested flag into the engine
Add `pub pre2025_method_attested: bool` to `ProjectionConfig` (`project/mod.rs`), default `false`.
`CliConfig::to_projection` (`config.rs:30-35`) maps the already-read `CliConfig.pre2025_method_attested`
into it. (No serde on `ProjectionConfig` is required — it is built per-projection from `CliConfig`;
confirm at impl. If any `ProjectionConfig` literal exists in tests, default the new field to `false`.)
`fold.rs`'s context (`ctx.config`) now carries it, reaching `note_pre2025_once`.

### D2 — attestation-aware advisory (`note_pre2025_once`, `fold.rs:80-101`)
`note_pre2025_once` currently takes `method: LotMethod` (not the whole `ctx`). Add an `attested: bool`
parameter, threaded from `ctx.config.pre2025_method_attested` at all THREE call sites (the pre-2025
Dispose / GiftOut / Donate arms — `fold.rs:551, 931, 998`). Keep the single-fire `Pre2025MethodNote`
(Advisory). Make its `detail` attestation-aware:
- **Unattested** (`pre2025_method_attested == false`): the current warning, made actionable —
  `"pre-2025 lots reconstructed under {m} (FIFO is the §7.4 legal default); you have NOT declared your
  filed pre-2025 lot method — if your filed pre-2025 returns used a different method your carryforward
  basis may differ. Declare it: config --set-pre2025-method <m> --attest-pre2025-method"`.
- **Attested**: informational acknowledgment (still Advisory severity, but non-warning tone) —
  `"pre-2025 lots reconstructed under your DECLARED + ATTESTED filed method {m} (§7.4); carryforward
  basis into 2025 reflects that method"`.

The blocker KIND stays `Pre2025MethodNote` (Advisory — it must never gate `compute_tax_year`; the
default-FIFO flow stays computable). Only the `detail` text differs. A machine-readable attested signal
already exists for consumers (`VerifyReport.pre2025_method_attested`, `render.rs:488`), so a text-only
`detail` change is sufficient — do NOT add a payload field to the variant.

### D3 — gate `safe-harbor-allocate` on attestation (the load-bearing fix)
A `SafeHarborAllocation` permanently records `pre2025_method` and is irrevocable. **Refuse to create one
while the pre-2025 method is unattested.** Implement as a **command-time refusal** in
`cmd/reconcile.rs::safe_harbor_allocate` (mirrors existing allocate preconditions): if
`session.config()?.pre2025_method_attested == false`, return a clear `CliError` —
`"refusing to record a safe-harbor allocation under an UNDECLARED pre-2025 method ({m}); a safe-harbor
allocation permanently records the method used to reconstruct your pre-2025 basis. Declare your filed
method first: config --set-pre2025-method <m> --attest-pre2025-method"`. This prevents the silent-FIFO
commitment (recon gap #3) at the only irrevocable step. (FIFO filers simply attest FIFO once — an
explicit confirmation that their filed returns used FIFO, not a silent inheritance of the default.)

### Decisions (made autonomously; flag for spec-review)
- **Tax computation is NOT hard-gated on attestation.** FIFO is the §7.4 legal default; an undeclared
  user still gets a computable result under FIFO + the (now louder, actionable) advisory. Only the
  *irrevocable* allocate step is gated. Rationale: don't break the basic flow; gate only the permanent
  commitment.
- **No new ledger "declaration" event in THIS slug — with an honest scope boundary (R0-I2).** The
  durable, immutable record of the declared method exists **only for Path B** (the `SafeHarborAllocation`
  records `pre2025_method` in the append-only ledger). For **Path A** (the legal default, no allocation),
  the attested method lives solely in mutable `cli_config`, which is explicitly NOT the source of truth
  (NFR6, `config.rs:1-4`) — so there is no durable/auditable record of the taxpayer's representation for
  the majority case. This slug therefore does NOT claim to durably record the Path-A declaration; it
  delivers the declaration's *teeth* (attestation-aware advisory + the irrevocable-allocate gate, which
  is exactly where a wrong record is permanent). A durable **append-only `Pre2025MethodDeclaration`
  event** for Path A (so the attestation is auditable + supersede-tracked like other decisions) is a
  real follow-up — **deferred to FOLLOWUPS** (see Task 5), not built here, to keep the slug right-sized.
  Rationale for deferral: for Path A nothing is *irrevocably committed* (the basis recomputes from
  events under whatever method is set, and the advisory updates with it), so the absence of a durable
  record changes no number — it is an audit-trail gap, not a correctness gap.
- **The basis-adjustment itself is unchanged** — it already works (verified). This slug adds declaration
  integrity only; it does not touch the reconstruction/conservation math.

### Legal grounding
§1.1012-1(j) / §7.4: FIFO is the default lot-identification method; a taxpayer may have used a different
method on filed pre-2025 returns (a specific-identification / standing-order choice). The reconstructed
2025 carryforward must reflect the method actually filed. Attestation is the taxpayer's representation of
that historical fact; requiring it before the irrevocable safe-harbor allocation ensures the recorded
method is a deliberate, attested choice — consistent with the standing-order / adequate-identification
posture this app enforces for post-2025 (§1.1012-1(j)(3)).

## Plan (TDD)

### Task 0 — update existing CLI tests broken by the D3 allocate gate (R0-I1)
The D3 gate makes `safe_harbor_allocate` refuse when `pre2025_method_attested == false`. Six existing
tests call `safe_harbor_allocate(...).unwrap()` WITHOUT attesting first and will panic at runtime
(compiler won't catch it): `crates/btctax-cli/tests/reconcile.rs` (calls @ ~498, 552, 561, 599, 684)
and `crates/btctax-cli/tests/verify_report.rs` (calls @ ~112, 125, 216, 227). **Sequencing:** make this
the FIRST implementation step interleaved with Task 3 (the test fix and the gate must land together so
the suite is never RED). For each enumerated call site, attest FIFO before allocating — e.g.
`config::set_pre2025_method(conn, LotMethod::Fifo, /*attested=*/true)` (or the CLI helper the test
already uses) — preserving each test's intent (they test allocation behavior, not the new precondition).
Re-verify the exact line numbers at impl time (they drift); grep `safe_harbor_allocate` across
`crates/btctax-cli/tests/` to find ALL call sites, not only the nine enumerated. Core-crate tests that
build `SafeHarborAllocation` payloads directly are unaffected (the gate is CLI-command-only). Validate:
the full `cargo test --workspace` is green after Task 3 + Task 0 land together.

### Task 1 — `ProjectionConfig.pre2025_method_attested` + plumb to fold context
- **Files:** `crates/btctax-core/src/project/mod.rs`; `crates/btctax-cli/src/config.rs`
  (`to_projection`); any `ProjectionConfig` test literals.
- Add the field (default `false`); map it in `to_projection`; ensure `ctx.config` carries it into
  `fold.rs`. KAT: `to_projection` round-trips the attested flag both ways; default is `false`.

### Task 2 — attestation-aware `note_pre2025_once`
- **Files:** `crates/btctax-core/src/project/fold.rs` (`note_pre2025_once` + its 3 call sites).
- Add the `attested: bool` param (D2); thread `ctx.config.pre2025_method_attested` at the 3 call sites;
  branch the `detail` per D2. KAT fixtures MUST include a real pre-2025 disposal (Dispose/GiftOut/Donate
  with `disposed_at.year() < 2025`) — the note only fires on the first such event (`fold.rs:551/931/998`);
  a buy-only ledger never triggers it. KATs: (a) unattested → advisory detail contains "have NOT
  declared" + the `config --set-pre2025-method` guidance; (b) attested → detail contains "DECLARED +
  ATTESTED"; (c) BOTH remain `Severity::Advisory` and do NOT gate `compute_tax_year` (a year with only
  this note still yields `Computed`). Fire-once preserved.

### Task 3 — gate `safe-harbor-allocate` on attestation
- **Files:** `crates/btctax-cli/src/cmd/reconcile.rs` (`safe_harbor_allocate`).
- Refuse (clear `CliError`) when `pre2025_method_attested == false` per D3; otherwise proceed
  unchanged (still records `cfg.pre2025_method`). KATs (temp vaults): (a) unattested → allocate
  refused, NO `SafeHarborAllocation` appended (event log unchanged), error names the
  `config --set-pre2025-method … --attest-pre2025-method` remedy; (b) attested → allocate succeeds and
  records the attested method; (c) attested FIFO → succeeds (explicit FIFO confirmation works).

### Task 4 — `verify` surfacing consistency (likely verification-only)
- **Files:** `crates/btctax-cli/tests/verify_report.rs` (KAT); `crates/btctax-cli/src/render.rs` ONLY if
  an actual contradiction is found.
- The advisory text comes from the blocker `detail` (Task 2) and `render_verify` already prints advisory
  blockers + the separate "Pre-2025 method (attested historical fact): {m} (attested: {bool})" line
  (`render.rs:991-995`). So this is expected to need NO render.rs change — just a KAT confirming the two
  lines are CONSISTENT: an attested vault's `verify` shows the informational advisory + "attested: true"
  with NO "have NOT declared" warning; an unattested vault shows the warning + "attested: false". Only
  edit `render.rs` if the KAT exposes a genuine contradiction.

### Task 5 — whole-diff review (Phase E gate) + FOLLOWUPS
- Cross-cutting: the attested flag is read consistently; tax computation still computable when
  unattested (advisory never gates); the allocate refusal is the ONLY new hard gate and it blocks the
  append (no partial state); backward-compat (existing vaults: attested=false default → louder advisory,
  and allocate now requires an attest step — call this out as the one behavior change); NFR4/NFR5;
  privacy (synthetic-only).
- **Record in FOLLOWUPS:** the deferred durable Path-A `Pre2025MethodDeclaration` ledger event (R0-I2)
  with its rationale (audit-trail, not correctness).

## Out of scope
- **Durable Path-A `Pre2025MethodDeclaration` ledger event (R0-I2 deferral).** Add to FOLLOWUPS as a real
  follow-up: an append-only, supersede-tracked declaration event so a Path-A (no-allocation) taxpayer's
  attested method is auditable in the ledger rather than only in mutable `cli_config`. Deferred because it
  changes no number for Path A (basis recomputes from events under the set method) — audit-trail
  enhancement, not correctness. Record the rationale in FOLLOWUPS at ship.
- **Fully-unconstrained import of an as-filed ending carryforward.** Path-B safe-harbor already provides a
  direct per-wallet lot/basis DECLARATION at the boundary, but it is conservation-checked against the
  reconstructed Universal residue (`resolve.rs:638-675`); importing an arbitrary carryforward that does
  NOT reconcile to the reconstructed history is intentionally not supported (it would defeat conservation).
- `safe_harbor_attest` (the command that attests an already-created `SafeHarborAllocation`) is
  intentionally NOT gated by D3 — the allocation it operates on already passed the D3 attestation gate at
  creation time, so re-gating attest would be redundant. (Confirm at impl that `safe_harbor_attest` only
  ever runs post-allocation.)
- A pre-2025 *per-disposal* method or election (pre-2025 has a single config method by design;
  post-2025 uses forward `MethodElection`).
- Gating tax computation on attestation; §170/forms; 2026/2027 tax tables (not available; unrelated).
