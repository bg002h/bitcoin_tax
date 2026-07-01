# FINAL whole-slug review — pre-2025 filed-method reconciliation (round 1)

- **Slug diff:** `c70922d..75d21db` (3 commits: `8f075ce` spec R0-GREEN, `fd6b559` Tasks 1+2, `75d21db` Tasks 0/3/4).
- **Contract:** `design/SPEC_pre2025_method_reconciliation.md` (D1/D2/D3 + Decisions/I2 + R0 fold records).
- **Reviewer hat:** independent whole-diff reviewer over the cross-cutting net (all tasks), focused on the
  §1.1012-1(j) / §7.4 compliance boundary. Source re-read at HEAD `75d21db`; per-task green bar (441 tests,
  clippy `-D warnings`, fmt, release, PII) NOT re-run — code/diff reviewed directly.
- **Verdict: READY TO MERGE — 0 Critical / 0 Important / 1 Minor / 2 Nit.** One non-blocking artifact
  deliverable (the FOLLOWUPS deferral entry) is part of THIS task and must land before ship; it moves no number.

---

## Scope of change (confirmed exhaustively)

The only **source** files touched are:
- `crates/btctax-core/src/project/mod.rs` — additive `ProjectionConfig.pre2025_method_attested: bool` (default `false`).
- `crates/btctax-core/src/project/fold.rs` — `note_pre2025_once` gains an `attested` param + branched detail text;
  threaded at exactly the 3 pre-2025 removal arms (Dispose 565, GiftOut 951, Donate 1024).
- `crates/btctax-cli/src/config.rs` — `to_projection` maps the flag through.
- `crates/btctax-cli/src/cmd/reconcile.rs` — the D3 attestation gate in `safe_harbor_allocate`.

`pools.rs`, `transition.rs` (src), `resolve.rs`, `render.rs`, `state.rs`, `event.rs` are **untouched** (verified
against the diff's file list and by grep). The basis-reconstruction / conservation / seed math therefore cannot
have moved (dimension 2 — see below).

---

## Dimension-by-dimension findings

### 1. The mechanism end-to-end — COMPLETE and CONSISTENT ✔
Traced the full chain against source:
`config --set-pre2025-method <m> --attest-pre2025-method`
→ `main.rs:471` guards `--attest` requires `--set` (no silent no-op)
→ `main.rs:498` → `cmd::admin::set_pre2025_method` → `config::set_pre2025_method` persists BOTH
  `pre2025_method` and `pre2025_method_attested` (`config.rs:123-135`)
→ `read_config` parses the flag with a fail-closed `BadConfigValue` on a corrupt value (`config.rs:107-118`)
→ `CliConfig` → `to_projection()` carries it (`config.rs:34`) → `ProjectionConfig.pre2025_method_attested`
→ `ctx.config.pre2025_method_attested` reaches `note_pre2025_once` at all 3 arms (`fold.rs:570/956/1029`)
→ the D3 gate reads the RAW `session.config()?.pre2025_method_attested` (`reconcile.rs:257-258`).

The gate genuinely prevents the irrevocable append of an UNDECLARED method: it returns `Err` at
`reconcile.rs:264` **before** any residue projection, payload construction, `append_and_save`, or `session.save()`.
A declared+attested method — including an **explicit FIFO** — allocates normally (Task-3 KAT (c),
`reconcile.rs` tests). The remedy string it prints (`config --set-pre2025-method fifo --attest-pre2025-method`)
is a VALID command: `MethodLotArg{Fifo,Lifo,Hifo}` is a clap `ValueEnum` (`main.rs:289-294`) which renders
lowercase, matching the `{m}` the gate emits.

**No bypass path.** The only two other `SafeHarborAllocation` constructors in the tree are both `#[cfg(test)]`
(`event.rs:369` inside `every_variant_serde_round_trips`; `optimize.rs:1563` inside `mod tests`). The only
user-facing persistence paths are `safe_harbor_allocate` (D3-gated) and `safe_harbor_attest`, which re-appends
via `..prior` (`reconcile.rs:543-546`) and thus copies an already-gated `pre2025_method` — it introduces no new
undeclared method, correctly staying outside D3.

### 2. No unintended behavior change beyond the two intended ones ✔
The tax math is provably unchanged: no basis/conservation/seed source file is in the diff, and the two core
changes are inert w.r.t. figures — the new `ProjectionConfig` field is read ONLY by `note_pre2025_once`, and
that function only ever `add_blocker`s an **Advisory** `Pre2025MethodNote` (kind + severity unchanged:
`state.rs:73` still maps it to `Severity::Advisory`, and `state.rs` is untouched). The advisory NEVER gates
`compute_tax_year` — proven for BOTH attested and unattested configs by
`pre2025_advisory_note_does_not_gate_compute_tax_year` (method_election.rs). The two intended changes
(louder/attestation-aware advisory; allocate now requires attestation) are the only observable deltas.

### 3. Gate correctness ✔
- **Appends nothing on refusal** — early return before all append/save calls; Task-3 KAT (a) asserts the event
  log contains no `SafeHarborAllocation` after a refused allocate. No partial/irrevocable state.
- **Reads the right flag** — `pre2025_method_attested` (config), NOT the `timely_allocation_attested` (§5.02(4))
  function parameter. No conflation; the code comment (`reconcile.rs:254-255`) and the updated test comments
  keep the two attestations explicitly distinct.
- **Composes cleanly** — the gate is first (before empty-lots at `reconcile.rs:298` and conservation), so no
  interaction with those refusals. No conflict/double-fire with the `Pre2025MethodConflictsAllocation` Hard
  blocker: D3 gates *creation* at command time; that blocker detects *post-creation* live-vs-recorded drift at
  projection time — different times, and if D3 fires there is no allocation for the blocker to evaluate.

### 4. Backward-compat ✔
Existing vaults default `attested=false` → (a) the advisory becomes the "have NOT declared" actionable warning
(Task-4 unattested KAT); (b) `safe_harbor_allocate` now requires an attest step. These are the only two
behavior changes. **Task 0 is complete**: every pre-existing CLI `safe_harbor_allocate` call site now attests
FIFO first — verified by reading all callers in `crates/btctax-cli/tests/{reconcile.rs,verify_report.rs}`
(reconcile.rs @ 501, 558, 567, 607, 694; verify_report.rs @ 115, 128, 221, 232). The ONLY un-attested allocate
remaining is the deliberate Task-3 refusal KAT. No orphan un-attested call survives.

### 5. I2 deferral honesty ("Path A commits nothing irrevocable") — TRUE ✔
Verified there is no path where an unattested/undeclared **Path-A** method is irrevocably committed or produces
a wrong number without attestation:
- Path A has no `SafeHarborAllocation`; the carryforward re-derives from events under the current
  `config.pre2025_method` on every projection (`fold.rs::applicable_method` → Universal pool), so a later
  `--set-pre2025-method` simply re-bases it — nothing is locked.
- The single irrevocable step (the Path-B allocation append) is exactly what D3 gates.
- Pre-2025 disposal figures do compute under the FIFO default when unattested, but that is (i) the §7.4 legal
  default, (ii) **pre-existing** behavior this slug did not change, and (iii) now accompanied by the louder
  actionable advisory — intended per the spec's "tax computation is NOT hard-gated" decision, not a silent
  wrong number.
The deferral of the durable Path-A `Pre2025MethodDeclaration` event is therefore correctly characterized as an
audit-trail gap, not a correctness gap. (But the deliverable to RECORD it is missing — see Minor M-1.)

### 6. NFR/privacy/drift/dead-code ✔ (one drift item → M-1)
- **Privacy:** all new/updated tests use synthetic fixtures (`cb-*` refs, `bc1q…` placeholder addresses,
  static prices, SYNTHETIC tax table). No real PII.
- **NFR4/NFR5:** determinism preserved — the gate is a pure config read; no ordering/nondeterminism introduced.
- **Dead code:** none — the gate's `m` binding is consumed in the error string; the new field is read.
- **Spec drift:** implementation matches D1/D2/D3 and the KATs match Task 2/3/4. The one drift is the missing
  FOLLOWUPS entry (M-1).

---

## Findings

### M-1 (Minor) — Task-5 required deliverable missing: the deferred Path-A declaration event is NOT in FOLLOWUPS.md
The spec mandates, in BOTH "Task 5" and "Out of scope", that the deferred durable Path-A
`Pre2025MethodDeclaration` ledger event (R0-I2) be recorded in `FOLLOWUPS.md` **at ship**, with its rationale
("append-only/supersede-tracked auditable record for the no-allocation majority case; changes no number for Path
A — audit-trail enhancement, not correctness"). `FOLLOWUPS.md` was NOT touched by this slug, and no such entry
exists (the existing pre-2025 entries @ FOLLOWUPS.md:151-154 and :335 predate this slug and describe the
Phase-1 advisory / the mechanism-at-large, not this specific deferred event). This changes no tax figure and is
not a compliance-boundary defect, so it does not bar the 0C/0I verdict — but it is a concrete deliverable of the
very task this review closes and MUST land before the slug is considered shipped.
**Fix:** add the R0-I2 deferral to `FOLLOWUPS.md` with the audit-trail-not-correctness rationale.

### N-1 (Nit) — double config read in `safe_harbor_allocate`
`session.config()?` is read twice: once for the gate (`reconcile.rs:257`) and again for `.to_projection()`
(`reconcile.rs:281`). Both read the same in-memory `cli_config` within one session, so there is no correctness
risk, but they can be collapsed to a single `let cli_cfg = session.config()?;` reused for the gate and
`cli_cfg.to_projection()`. Pure cleanup.

### N-2 (Nit, the open triage item) — no separate non-FIFO attested-allocate success KAT → **DEFER**
The only attested-allocate success KAT records FIFO (Task-3 KAT (c)); there is no LIFO/HIFO attested-allocate
success KAT through the CLI command. **Triage: DEFER (non-blocking).** Rationale: the gate is method-agnostic —
its only branch is `if !attested { refuse }`; once attested, the method value merely flows into the recorded
payload, which KAT (c) already verifies by asserting `alloc.pre2025_method == Fifo`. The non-FIFO
residue-projection + conservation path is pre-existing behavior already covered by the core tests that build
`SafeHarborAllocation` payloads directly (`safe_harbor_method.rs` / `transition.rs`) and by
`pre2025_method_note_renders_declared_method` (HIFO). Marginal incremental coverage; optionally note in
FOLLOWUPS but do not block.

---

## Answer to the merge question

**Yes — the slug is ready to merge (0 Critical / 0 Important).** The declaration now has genuine teeth: the
attestation flag reaches both the advisory and the irrevocable allocate gate through a complete, consistent
chain; the gate fails closed and appends nothing on refusal; the tax/conservation math is untouched (no figure
moves); backward-compat is handled with every existing allocate call site updated (no orphan); and the I2
deferral is honest (Path A commits nothing irrevocable). The single Minor (M-1) is a FOLLOWUPS deliverable
belonging to this task and must be written before ship; the two Nits are optional. Re-review is not required to
reach green once M-1 is folded (it touches only FOLLOWUPS.md, no code/tests).
