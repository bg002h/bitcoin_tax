# R0 — SPEC_pseudo_reconcile_mode.md — round 1

**Artifact:** `design/SPEC_pseudo_reconcile_mode.md` (sub-project 2 of auto-pseudo-reconcile).
**Baseline:** branch `feat/pseudo-reconcile-mode` @ `408fd44` (main `514875b`). Read-only architect review.
**Bar:** 0 Critical / 0 Important. **Design of record:** `design/BRAINSTORM_auto_pseudo_reconcile.md` (settled decisions not relitigated).

## Verdict: **2 Critical / 5 Important / 6 Minor / 2 Nit** — NOT GREEN (fix C1, C2, I1–I5 before round 2)

The reversible-mode shape is sound and the two structural safety pillars hold: (a) `project()` is a pure read
(`session.rs:446-452`, `458-466` — `load_all` → `project` → return; no `save`), so an in-memory injection **cannot**
persist; only `append_decision` + `session.save()` write (`reconcile.rs:33-34`). (b) The on-screen render path
(`render_verify`/`render_report`) and the file path (`write_csv_exports`, `export_snapshot`) are distinct functions,
and `snapshot.sqlite` is the decrypted **event** DB — since synthetics are never persisted it is automatically clean.
So ★2 (not-persisted) and the sqlite half of ★3 are structurally satisfied.

But three things break as written: the on-screen guard has a **taint-propagation hole** (a real Sell of pseudo-basis
coins renders an unflagged fictional gain — C1); the settled **accept-first-for-`DecisionConflict`** default is
unimplementable without violating the "real supersedes" invariant (C2); and the **"N blockers → 0"** contract is false
for several Hard kinds the defaults cannot clear (I2). Details below.

---

### [C1] CRITICAL — pseudo-basis taint must PROPAGATE to real disposals that consume pseudo lots; a per-event `pseudo_origin` flag leaves the most dangerous number unflagged

**Spec:** "each resulting disposal/lot/row carries a `pseudo_origin: bool` … so render can flag it" (SPEC §Mechanism,
lines 20-21); the guard promises `[PSEUDO]` flags "wherever pseudo defaults contribute" (line 40).

**Evidence.** The `SelfTransferMine{basis:$0}` default creates a $0-basis origin lot (`fold.rs:958-1012`,
`Op::SelfTransferInbound`; `usd_basis = basis.unwrap_or(Usd::ZERO)`, `:980`). Imported **Sells stay taxable** and are
NOT touched by pseudo (SPEC line 32; brainstorm line 35). When a real `Op::Dispose` later consumes that pseudo lot, the
leg gain is `proceeds − 0` (`fold.rs:595-640`, `make_disposal_legs` `:189-199`) — a **fictional max-gain** number on an
otherwise-**real** disposal. Basis also flows through relocation: a pseudo lot moved by a later real `SelfTransfer`
carries into a new lot (`fold.rs:766-813`). A `pseudo_origin: bool` set only on the directly-defaulted event/lot leaves
the consuming disposal (and the relocated lot, and the held-balance row) **unmarked**. `disposals.csv` already emits
`basis`/`gain` with no basis provenance (`render.rs:617-639`), and `render_report` renders those same figures on-screen.

**Why Critical.** This is a hole in the *headline* guard and the tax-critical mandate ("output must NEVER be mistakable
for a filing"): the single most misleading figure pseudo produces — a real Sell's gain computed on invented $0 basis —
would render **without** a `[PSEUDO]` flag. It directly violates the invariant on SPEC line 40/58.

**Fix.** Define taint **propagation**, not a per-event flag. Carry a `pseudo: bool` on `Lot` → thread it into `Consumed`
(`fold.rs` pool consume) → onto `DisposalLeg`/`RemovalLeg`/`PendingLeg` and the held-lot rows. Any row whose **existence
OR basis** traces to a synthetic default is `[PSEUDO]` on every on-screen surface (report + TUI). State the rule as a
tax-safety invariant with a fault-inject KAT: "a real Sell consuming a pseudo lot is flagged pseudo on-screen."

---

### [C2] CRITICAL — "accept-first" cannot clear `DecisionConflict`; that default is unimplementable without violating the "real decisions always supersede" invariant

**Spec:** `| DecisionConflict / ImportConflict | accept-first |` (SPEC line 34; brainstorm line 29).

**Evidence.** `ImportConflict` and `DecisionConflict` are fundamentally different:

- `ImportConflict` is a **system event awaiting a choice** — resolved by a real `SupersedeImport`/`RejectImport`
  (`resolve.rs:430-472`). Pseudo CAN synthesize that choice (a synthetic `Accept` into `conflict_res`, or a
  materializable `SupersedeImport{conflict_event}`, `event.rs:192-194`). Legitimate. ✓
- `DecisionConflict` is emitted **when two REAL decisions collide** — duplicate `ClassifyInbound` (`resolve.rs:630-640`),
  void-of-non-revocable (`:387-391`), bad-target `ReclassifyOutflow`/`ReclassifyIncome`/`ManualFmv`
  (`:665-708`, `:724-768`, `:511-543`), duplicate `LotSelection` (`:942-950`), multiple effective allocations
  (`:1113-1122`), etc. There is **no unresolved event awaiting a default** here — the event is *over*-specified.

The only ways to remove a `DecisionConflict` are (a) persist a `VoidDecisionEvent` on one side — violates the
not-persisted invariant (SPEC line 62, Gotcha line 83); or (b) in-memory suppress one of the two real decisions — which
means **a real decision is ignored**, the exact failure the spec fault-injects ("break the precedence → the real
decision is ignored → RED", SPEC line 56). So the settled default cannot be built as written.

**Fix.** Scope accept-first to **`ImportConflict` only** (materializable as a synthetic `SupersedeImport`). Remove
`DecisionConflict` from the defaults table and state explicitly: pre-existing `DecisionConflict`s (real-decision
collisions) are **not** auto-cleared by pseudo and remain surfaced; the user resolves them by voiding one side. This also
feeds the honest restatement in I2.

---

### [I1] IMPORTANT — the injection must NOT mint `EventId::Decision` values; "ids derived from the target event id" risks decision_seq collision and precedence corruption

**Spec:** "synthetic decisions get deterministic ids/order derived from the target event id" (SPEC line 25);
"synthesize an in-memory synthetic decision" (line 18).

**Evidence.** `EventId::Decision{seq: u64}` is keyed **solely by `seq`** (`identity.rs:69`, `:103`). Real decisions live
in this same seq space (`resolve.rs:420-427` collects `EventId::Decision{seq}`; `append_decision` assigns monotonic
seqs). A synthetic id "derived from the target event id" is an arbitrary `u64` that can **collide** with a real decision
seq → `by_id` overwrite (`resolve.rs:369`), spurious `voided` membership, or precedence misorder. Nothing else in the
pipeline needs a synthetic to be a `LedgerEvent`: `fold` consumes the `timeline` (`Vec<Eff>`) + elections + selections,
never raw decisions (`fold.rs:367-405`). "Fold consumes real + synthetic decisions identically" (line 19) is imprecise —
fold consumes **Ops**, which `build_op` derives from the resolve maps.

**Fix.** Inject at the **resolved-map layer**: after real decisions populate `inbound_class`/`outflow_class`/`links`/
`applied`/`conflict_res` (`resolve.rs:547-821`), for each still-unresolved imported event add the default entry to the
relevant map and record its `EventId` in a `pseudo_ids: BTreeSet<EventId>` threaded onto `Eff` (`resolve.rs:102-116`).
Reserve `EventId::Decision` minting for the **approve** path, where `append_decision` assigns real seqs. Revise line 25:
determinism comes from the target-event iteration order (sorted by `EventId: Ord`), not from fabricated ids.

---

### [I2] IMPORTANT — several Hard blockers the defaults CANNOT clear ⇒ "N blockers → 0" is false; define the target precisely and enumerate the exclusions

**Spec:** goal "from N blockers → 0" (line 10); "instant 0 classification-blockers" (brainstorm 19); T3 "0
classification-blockers end-to-end KAT" (line 76). The task explicitly asks: "any blocker kind the defaults can't clear?"

**Evidence (Hard kinds — `state.rs:67-90` — that survive pseudo):**

1. **`UncoveredDisposal`** (`fold.rs:597-603`, `:712-719`, dispose-without-wallet `:579-585`). A Sell/withdrawal short of
   coverage cannot be cleared: pseudo deliberately does **not** fabricate acquisitions, because a $0-basis synthetic buy
   would yield **max** gain, contradicting the "≈zero tax" intent (brainstorm line 39). Common in practice (a Sell whose
   acquisition is on an exchange not yet imported, or dated before its covering inbound → FIFO can't consume a future lot).
2. **`FmvMissing` on a native `Income`** (`fold.rs:672-678`; `EventPayload::Income` with `FmvStatus::Missing`,
   `resolve.rs:247-264`). Pseudo defaults **only `TransferIn` inbounds** (SPEC lines 30-33), not native `Income` rows, so
   an unpriced imported income row stays Hard, and its basis-pending lot re-raises `FmvMissing` on later disposal
   (`fold.rs:138-144`).
3. **Real-decision-defect Hard kinds**: `MethodElectionBackdated`, `LotSelectionInvalid`, `SafeHarborUnconservable`,
   `Pre2025MethodConflictsAllocation`, `TaxTableMissing` — all downstream of real decisions/data pseudo won't touch.

**Fix.** Restate the goal as **"0 Hard CLASSIFICATION blockers"** (`Unclassified` / `UnknownBasisInbound` /
`ImportConflict`), and add a "Hard blockers pseudo does NOT clear" list (the five above + `DecisionConflict` from C2). The
T3 KAT must assert the achievable set on a representative fixture, not literal 0. If clearing native-`Income`
`FmvMissing` is in scope, add a default (a synthetic `ManualFmv` at bundled-price FMV via the existing `fmv_of`, or $0
with an advisory) — otherwise scope it out explicitly.

---

### [I3] IMPORTANT — sub-2 ships the fiction generator BEFORE sub-3's gate: `export_snapshot` with pseudo-on emits clean-looking fictional filing CSVs with no guard

**Spec:** "NEVER in any output file (export CSVs / forms stay clean — sub-3 gates their production behind the typed
attestation)" (SPEC lines 41-42); the queryable pseudo-contribution signal is "for sub-3's gate" (line 44).

**Evidence.** `export_snapshot` (`admin.rs:45-85`) projects and calls `write_csv_exports` unconditionally — it emits
`disposals.csv`/`lots.csv`/`form8949.csv`/`schedule_d.csv` (`render.rs:568-732`) with **fictional numbers and no
marker**. Nothing in sub-2's scope gates this (the gate is deferred to sub-3, brainstorm 84-88). Sub-2's own KAT only
asserts *marker absence* (SPEC lines 58-60), not *production refusal*. Releasing sub-2 independently therefore opens a
window where a user runs `reconcile pseudo on` then `export-snapshot` and gets clean-looking fictional Form 8949/Schedule
D — the precise "mistaken for a filing" failure the mandate forbids.

**Fix.** Sub-2 must **consume its own signal**: make `export_snapshot` (and the year-scoped form writers) return a hard
error when the pseudo-contribution count is non-zero — an interim gate until sub-3 replaces it with the typed
attestation. Alternatively, the spec must state that sub-2 and sub-3 ship together and sub-2 is never released alone.
Either way, add a "pseudo-active export is refused/gated in sub-2" invariant + KAT.

---

### [I4] IMPORTANT — the `[PSEUDO]` marker channel must be SEPARATE from any field a CSV/form column serializes (esp. `BasisSource`), or the marker leaks into the export and the ★ grep-KAT fails

**Spec:** the ★ KAT greps export CSVs/forms for any `PSEUDO`/synthetic marker and asserts NONE (SPEC lines 58-60).

**Evidence.** `lots.csv` writes `basis_source_tag(l.basis_source)` (`render.rs:596`). If the pseudo marker is encoded as
a new `BasisSource` variant (e.g. `PseudoSelfTransfer`) — the "natural" place given `SelfTransferInbound` already exists
(`fold.rs:1004`) — that literal string lands in `lots.csv`, **failing the grep-KAT and the output-clean invariant**.
Same trap for encoding pseudo-ness into `Blocker.detail` (which no CSV emits today, but forms/reports do).

**Fix.** The marker is a **dedicated boolean / id-set** field that `write_csv_exports` and the form writers deliberately
omit (like the C1 `pseudo` bool). Add an explicit constraint: pseudo-ness is NEVER encoded in a field already serialized
by an export column (`basis_source`, `kind`, `term`, `detail`). The grep-KAT is realistic **only** under this rule.

---

### [I5] IMPORTANT — the `TransferOut → self-transfer` default has no destination and no materializable decision; `Op::SelfTransfer` requires a `dest` wallet an unmatched outflow lacks

**Spec:** `| TransferOut withdrawal (unmatched/unclassified) | non-taxable self-transfer (no Sell/Gift/Spend) |`
(SPEC line 32); bulk-approve "materialize the selected synthetic defaults as REAL decision events" (line 48).

**Evidence.** `Op::SelfTransfer` **requires** a `dest: WalletId` and relocates the consumed lots there
(`resolve.rs:265-280`; `fold.rs:742-813`) — the dest comes from a `TransferLink` target (`TransferTarget::InEvent` /
`Wallet`, `event.rs:98-101`). An **unmatched** TransferOut has no in-event; today it folds to `Op::PendingOut` →
**advisory** `UnmatchedOutflows` (`fold.rs:698-740`), which is *already non-taxable* (no disposal, no gain). So the
default is ambiguous: (a) leave it as PendingOut (non-taxable already, but the advisory persists and **there is no real
decision to `approve`**), or (b) synthesize `TransferLink{ Wallet(w) }` to a fabricated `SelfCustody{label}`
(`identity.rs:110-111`), relocating coins to a fictional wallet (materializable as a real `TransferLink` on approve).

**Fix.** Pick and specify the mechanism, the destination wallet it uses, and the **exact decision `approve` writes**. If
(a): state that pseudo does NOT clear `UnmatchedOutflows` (advisory) and materializes nothing for outbounds. If (b):
define the canonical synthetic `SelfCustody` label and confirm the fold's relocation is the intended semantics.

---

### [M1] MINOR — "≈zero tax null-hypothesis" is misleading when the ledger has real Sells

Brainstorm/SPEC call the result "all movement non-taxable, ~zero tax" (SPEC line 37; brainstorm 39). A Sell of coins that
arrived via a pseudo $0-basis self-transfer recognizes `proceeds − 0` = **max** gain (see C1), not zero. Reword: pseudo
yields a *conservative estimate* — all non-Sell movement non-taxable; imported Sells taxable at pseudo-derived (often $0)
basis — which can be far from zero. This also motivates why C1's on-screen flag is load-bearing.

### [M2] MINOR — the SelfTransferMine $0 default emits `SelfTransferInboundZeroBasis` advisories, and `PseudoReconcileActive` is always present ⇒ "verify shows 0 blockers" is never literally true in pseudo mode

Each defaulted inbound fires the advisory `SelfTransferInboundZeroBasis` on `basis == None` (`fold.rs:982-993`;
`state.rs:60-65`), and the spec adds an always-on `PseudoReconcileActive` advisory (SPEC line 40). Define the success
metric strictly around **Hard** blockers (advisories are intended and, here, are literally *produced by* the defaults).

### [M3] MINOR — "[PSEUDO] per-row markers in verify" is imprecise; verify shows blockers, not rows

`render_verify` renders conservation + Hard/Advisory blocker lists (`render.rs:1771-1822`), not per-row disposals/lots.
Per-row `[PSEUDO]` markers belong to `render_report` (`render.rs:195`) + the TUI. Verify surfaces the
`PseudoReconcileActive` advisory — which renders automatically via `{:?}` on the kind (`render.rs:1821`), so **no
match-arm is needed** to add the new advisory kind (a genuine ease-of-implementation point worth stating). Adjust the
guard wording on SPEC line 40.

### [M4] MINOR — CLI bulk-approve must reuse the CLI-side append loop, NOT `persist_bulk_decisions` (dependency-cycle trap)

SPEC line 48 says "reuse the bulk-reconcile append machinery"; SPEC line 68 scopes approve to both crates. The reusable
CLI machinery is the `apply_bulk_*` pattern (`reconcile.rs` — e.g. `apply_bulk_accept_conflicts:475-490`: `append_decision`
loop + single `session.save()`, `?`-before-save rollback), **not** tui-edit's `persist_bulk_decisions`
(`persist.rs:432`), which `btctax-cli` cannot reach (`btctax-tui-edit` depends on `btctax-cli`, `Cargo.toml:19` → cycle).
Name the per-crate reuse target: CLI approve → `apply_bulk_*` style; TUI approve → `persist_bulk_decisions`.

### [M5] MINOR — the `Unclassified` default needs a two-hop synthesis or an explicit scope note

`Op::Unclassified` comes from `EventPayload::Unclassified` (a raw row, `resolve.rs:353`; `fold.rs:1213-1219`), which may
be neither clearly inbound nor outbound. Turning it into a $0 self-transfer needs a synthetic `ClassifyRaw → TransferIn`
then `ClassifyInbound → SelfTransferMine` (two chained synthetics), and "acquire without wallet" `Unclassified`
(`fold.rs:543-548`) can't be classified at all. Specify the chain, or scope the default to `Unclassified` rows that
resolve to an inbound.

### [M6] MINOR — the placeholder tax-profile default is a CLI-layer injection, NOT a resolve decision, and must be applied on BOTH the report and export paths

`compute_tax_year` takes `profile: Option<&TaxProfile>` (`compute.rs:228-272`) and `TaxProfile` is a CLI side-table, not
a ledger event (`tax_profile.rs`). So the placeholder is expressible without persisting — pass `Some(&placeholder)` when
mode-on and `tax_profile(year)` is `None` — but the injection site is `report_tax_year` (`tax.rs:66-68`) **and**
`export_snapshot`'s own profile read (`admin.rs:58-76`), NOT `resolve`. The SPEC "Injection: in resolve" (line 18) does
not cover the profile; state the separate site, and ensure the export path (I3) applies the same placeholder logic (or
refuses).

### [N1] NIT — pin `ProjectionConfig`'s new flag default to `false`

`ProjectionConfig` is `Copy` (`mod.rs:31-40`); adding `pseudo_reconcile: bool` keeps it `Copy`. Its `Default` MUST be
`false` (mode-off = byte-identical, SPEC line 55) and `CliConfig::to_projection` must carry it (`config.rs:30-36`).
Follow the existing corrupt-value discipline in `read_config` (`config.rs:107-118`) for the new `cli_config` key.

### [N2] NIT — make the approve iteration order deterministic (NFR4)

`append_decision` assigns seqs by insertion order (`reconcile.rs:33`). Feed the synthetic-defaults-to-approve in a
deterministic order (sorted by target `EventId`) so two approves of the same selection assign identical seqs. Cosmetic,
but consistent with the NFR4 posture elsewhere.

---

## ★ Task-question summary

- **★ injection implementable + safe?** YES via resolve **map-injection** + an `Eff`-level `pseudo` flag; NOT via minting
  `EventId::Decision` (I1). Not-persisted safety holds structurally — `project()`/`resolve` never write (session.rs:446-466).
- **★ real-supersedes + not-persisted?** Real-supersedes is cleanly expressible for the map defaults (real decisions fill
  the maps first; inject only for still-unresolved events) — EXCEPT the `DecisionConflict` default, which cannot honor it
  (C2). Not-persisted: confirmed.
- **★ on-screen-yes / output-no feasible?** The render/export split is real and `snapshot.sqlite` is auto-clean; the
  grep-KAT is realistic **iff** the marker is a dedicated channel (I4) AND taint propagates so all pseudo-contributed
  rows are actually flagged on-screen (C1). Sub-2 also leaves the export ungated (I3).
- **Blockers the defaults can't clear:** `DecisionConflict` (C2), `UncoveredDisposal`, native-`Income` `FmvMissing`, and
  the real-decision-defect Hard kinds (I2). "N → 0" must be restated as "0 Hard classification blockers."

Re-review after fold (including the last fold) per §2.
