# SPEC — SE completion Chunk C: ReclassifyIncome decision (the business flip)

**Source baseline:** `main` @ `1b6dfe3` (post Chunk A). Queue item 2, chunk 2 of 3 (A shipped; B = expenses
advisory next).
**Goal:** A new **`ReclassifyIncome` decision** so the user can flip `business` (and optionally `kind`) on
an already-imported **Income** event — closing the River `business:false` immutability (River hard-codes
`business: false` on its income/interest rows; Income events are not `ClassifyRaw`-able, so a River
business-miner currently has NO path to SE treatment). Event-sourced, voidable, conflict-checked —
mirroring `ReclassifyOutflow`.

**SemVer:** new `EventPayload` variant + a CLI subcommand ⇒ **MINOR** (pre-1.0). Back-compat: ADDING an
enum variant is safe for reading OLD vaults (they contain none); old BINARIES cannot read a vault that
contains the new variant — the same accepted trade-off as every prior decision-type addition (document).

## Current-state (recon @ 33b7f26, re-verify @ 1b6dfe3)
- `river.rs:~145-180`: `"income"`→`Reward` and `"interest"`→`Interest`, BOTH `business: false` hard-coded,
  with comments explicitly flagging "IMMUTABLE post-ingest … no reconciliation path" — update those
  comments once this ships.
- `ClassifyRaw` (`event.rs:~188-191`, `resolve.rs:~400-410`) targets `Unclassified` rows by convention —
  not a sanctioned path for Income events.
- The `ReclassifyOutflow` pattern to mirror: payload struct + `EventPayload` variant (`event.rs`), resolve
  collection into a `BTreeMap<EventId, _>` with duplicate → `DecisionConflict` (`resolve.rs:~487-496`),
  `build_op` consult, `VoidDecisionEvent` revocation (generic, free).
- `build_op` `EventPayload::Income(x)` branch (`resolve.rs:~180-191`) → `Op::Income { …, business:
  x.business }`; the fold (`fold.rs:~642-697`) pushes `IncomeRecord { …, kind, business }` — no fold
  change needed (the override lands in build_op).
- Consumers of the flip: `se_net_income` (business && kind != Interest) — the point of the feature;
  `crypto_ord` (engine B) is kind/business-AGNOSTIC → a flip does NOT move any engine-B figure;
  NII: `interest_nii` filters on kind only → a business flip doesn't move NIIT; a KIND flip to/from
  `Interest` WOULD move NIIT + SE — that's correct behavior (the user is correcting the facts), KAT it.

## Design

### D1 — the decision payload
```rust
pub struct ReclassifyIncome {
    pub income_event: EventId,          // the target imported Income event
    pub business: bool,                 // the corrected flag
    #[serde(default)]
    pub kind: Option<IncomeKind>,       // optional kind correction (None = keep the original)
}
```
+ `EventPayload::ReclassifyIncome(ReclassifyIncome)`. Serde: new variant (old vaults read fine; document
the old-binary limitation in the variant doc-comment). `fingerprint()` → None (a decision, like
ReclassifyOutflow — no dedup fingerprint).

### D2 — resolve
Collect non-voided `ReclassifyIncome` into `income_reclassify: BTreeMap<EventId, ReclassifyIncome>`
(mirroring `outflow_class`); a SECOND non-voided decision for the same `income_event` →
`DecisionConflict` (same pattern/message shape as ReclassifyOutflow's). In `build_op`'s
`EventPayload::Income(x)` branch: if an override exists → `business = o.business`, `kind =
o.kind.unwrap_or(x.kind)`; else unchanged.
**Bad-target validation [R0-I1 — CONCRETE, a deliberate divergence]:** ReclassifyOutflow's actual behavior
for a missing/mismatched target is SILENTLY INERT (blind insert at `resolve.rs:487-497`, consulted only in
the TransferOut branch) — that is NOT acceptable for an SE-relevant correction. Instead, validate at
pass-1e COLLECTION time against the EFFECTIVE payload (`applied.get(&target).unwrap_or(raw)` — so a
ClassifyRaw'd row that became Income stays reclassifiable, and a by_id miss counts as bad): target absent
OR its effective payload is not `Income` → Hard `BlockerKind::DecisionConflict` + the decision EXCLUDED
from the override map. Precedents: TransferLink's in-event check (`resolve.rs:456-466`), LotSelection
targeting (`resolve.rs:604-611`). Note the divergence from ReclassifyOutflow in a comment; add a FOLLOWUPS
item to backfill the same validation onto ReclassifyOutflow (out of scope here).

### D3 — CLI + River comments
`reconcile reclassify-income <event_ref> --business <true|false> [--kind mining|staking|interest|airdrop|
reward]` (parse via `eventref`; kind strings match the existing `eventref.rs` parser). Wire through
`cmd/reconcile.rs` (`append_and_save`, like the other decisions — this IS a mutating decision, unlike
Chunk 3b's side-table). `void` works via the existing `VoidDecisionEvent` (no new code). Update the two
river.rs "IMMUTABLE post-ingest" comments to point at `reconcile reclassify-income`.

### Decisions
- **Event-sourced decision (NOT a side-table)** — this changes PROJECTED STATE (IncomeRecord.business/kind
  → SE + potentially NIIT), unlike 3b's pure form metadata; auditability + void semantics required.
- `kind` override included (Option, default None) — River's `Reward` mislabel for actual mining is the
  known case.
- Engine-B invariance for a business-only flip (crypto_ord agnostic) is a REQUIRED KAT, not an accident.

## Plan (TDD)

### Task 1 — payload + resolve + CLI + KATs
- **Files:** `crates/btctax-core/src/{event.rs,project/resolve.rs}`, `crates/btctax-cli/src/{main.rs,
  cmd/reconcile.rs,eventref.rs (reuse)}`, `crates/btctax-adapters/src/sources/river.rs` (comments only).
- KATs (synthetic; assert EXACT):
  - **The headline flip:** an imported `Income{Reward, business:false}` (River-style) + `reclassify-income
    --business true --kind mining` → `IncomeRecord{kind: Mining, business: true}` → `se_net_income`
    includes it (compute_se_tax now Some with the P2-D math); before the decision → excluded/None.
  - **Business-only flip (no kind):** `kind` stays the original; business flips.
  - **Engine-B invariance:** the same fixture's `compute_tax_year` figures (ordinary/total) are IDENTICAL
    before vs after a business-only flip (crypto_ord is agnostic) — the tax goldens don't move.
  - **Kind flip moves NIIT correctly [R0-I2 — NON-VACUOUS]:** the fixture's profile MUST have MAGI above
    the §1411 threshold (else niit = 0 both sides and the KAT asserts nothing). Assert EXACT NONZERO niit
    deltas in BOTH directions: `Reward → Interest` puts the FMV into `interest_nii` (r.niit rises by the
    derived amount; SE still excludes it); `Interest → Mining` removes it (r.niit falls to the derived
    value; enters SE if business).
  - **Duplicate → DecisionConflict:** two non-voided reclassifies on the same income event; assert
    FIRST-WINS for the projected value [R0-Minor].
  - **Void reverts:** decision + `VoidDecisionEvent` → the original business/kind project again.
  - **Bad target [R0-I1]:** a ReclassifyIncome pointing at a MISSING event AND one pointing at a
    non-Income event → each yields the Hard `DecisionConflict` blocker and the decision is EXCLUDED
    (projected values unchanged) — NOT a panic and NOT silently inert.
  - **No-fingerprint KAT [R0-Minor]:** `ReclassifyIncome.fingerprint() == None` (repo convention); add the
    variant to the `every_variant_serde_round_trips` inventory + fix the `resolve.rs:~292-296`
    revocability doc.
  - **Back-compat:** an old-vault JSON event stream WITHOUT the new variant loads unchanged (trivially
    true — pin it); the serde round-trip of the new variant (with and without `kind`).
- Determinism (BTreeMap); exact Decimal; synthetic only.

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: the override lands ONLY via build_op (no fold change); DecisionConflict + void + bad-
  target semantics mirror ReclassifyOutflow exactly; engine-B invariance for business-only flips; the
  kind-flip NIIT/SE interactions correct; the river.rs comments updated; old-binary limitation documented;
  determinism. FOLLOWUPS: Chunk B (expenses advisory) next — the cluster's last piece.

## Out of scope
- Schedule C expenses (Chunk B); flipping FMV (ManualFmv exists) or amounts; reclassifying non-Income
  events; a migration for old binaries; batch flips; 2026/2027 tables.
