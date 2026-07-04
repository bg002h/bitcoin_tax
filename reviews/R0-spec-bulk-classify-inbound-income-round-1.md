# R0 — Spec review: bulk-classify-inbound-income (Cycle 4), round 1

**Artifact:** `design/SPEC_bulk_classify_inbound_income.md` (DRAFT)
**Branch/base:** `feat/bulk-classify-inbound-income` @ `69742b9` (main == `c643ddd`)
**Reviewer role:** independent architect, read-only vs CURRENT source. Bar: 0 Critical / 0 Important.

## Verdict: **0 Critical / 1 Important / 4 Minor / 2 Nit** — NOT R0-GREEN

One Important architectural blocker (I1): the spec prescribes an **unbuildable** CLI persist path
(`btctax-cli` cannot delegate to `persist_bulk_decisions`, which lives in `btctax-tui-edit`). The core
tax-safety thesis (#a) is otherwise **verified sound and, if anything, understated** — see the adjudication
below. Fold I1 (and the Minors) and re-review.

---

## ★ The #a tax-safety exclusion — ADJUDICATED SOUND (spec understates it)

The spec's central claim — a persisted `InboundClass::Income { fmv: None }` re-fires a **Hard `FmvMissing`**
that gates the tax year, so bulk-income MUST exclude every `fmv_of == None` row — is **CONFIRMED** on all
four sub-points:

- **(a) `Income{fmv:None}` → Hard `FmvMissing`.** `InboundClass::Income{kind,fmv,business}` projects via
  `build_op` → `Op::IncomeInbound{fmv:*fmv,…}` (`crates/btctax-core/src/project/resolve.rs:273-282`).
  In the fold, `Op::IncomeInbound` with `fmv == None` raises `BlockerKind::FmvMissing`
  (`crates/btctax-core/src/project/fold.rs:853-860`, detail "income inbound FMV missing"). **Confirmed.**
- **(b) `FmvMissing` is Hard and gates the year.** `severity()` maps `FmvMissing → Severity::Hard`
  (`crates/btctax-core/src/state.rs:71-83`). `compute_tax_year` refuses on *any* Hard blocker anywhere →
  `TaxYearNotComputable` (`crates/btctax-core/src/tax/compute.rs:242-256`). **Confirmed — the year is gated.**
- **(c) `fmv_of` returns `Option<Usd>`.** `crates/btctax-core/src/price.rs:13-18` — `None` on missing daily
  close *or* checked-arithmetic overflow, never a panic. **Confirmed** (path is `price.rs`, not the spec's
  stale `btctax-core/src/price.rs:13` root — same content, correct crate is `crates/btctax-core/…`).
- **(d) bulk-sti INCLUDES missing-price rows (so this is the one real difference).**
  `bulk_self_transfer_in_plan` pushes every row with `usd_fmv: Option<Usd>` (`= fmv_of(...)`) into
  `included`, then merely *counts* the `None`s as `missing_price_count` — it does not drop them
  (`crates/btctax-cli/src/session.rs:598-611`). **Confirmed.** The `fmv_of==None` EXCLUSION is therefore
  the single behavioral delta, exactly as claimed.

**Adversarial note — the spec understates the blocker's stickiness (see M1):** for the *inbound* income
path, `Op::IncomeInbound` (`resolve.rs:277`) does **not** consult `manual_fmv`, and a `ManualFmv` decision
pointed at a classified TransferIn is itself rejected as a Hard `DecisionConflict` (pass-1d validates the
target's *effective* payload is `Income`; a classified TransferIn's effective payload is still `TransferIn`
— `resolve.rs:476-495`). So a persisted `Income{fmv:None}` inbound **cannot be cleared by `ManualFmv`** at
all — only by voiding the `ClassifyInbound` and re-classifying with a real `fmv`. The exclusion is even more
load-bearing than the spec says; there is no cheap remedy after the fact. **Exclude, never emit.**

Other Hard-blocker vectors on this path, all handled by the mirrored exclusions:
- **wallet-less** included row → `Op::IncomeInbound` with `wallet == None` *also* raises Hard `FmvMissing`
  (`fold.rs:832-838`, detail "income inbound without wallet"). Covered by the wallet-less exclusion (M5).
- **double-classify** → duplicate `ClassifyInbound` on one target raises Hard `DecisionConflict`
  (`resolve.rs:582-592`, first-wins). Covered by the already-classified exclusion.
A valid included row (wallet + price + not-already-classified) yields `Op::IncomeInbound{fmv:Some}` → income
lot + `IncomeRecord`, clears `UnknownBasisInbound`, adds **no** new Hard blocker. The #a design is complete.

---

## Findings

### [I1] IMPORTANT — CLI apply cannot delegate to `persist_bulk_decisions` (wrong crate; unbuildable as specified)
**Spec:** §Persist (L50-55) "it uses the shipped `persist_bulk_decisions(...)` directly … No new persist
fn"; §CLI (L68) "`apply_bulk_classify_inbound_income(...)` … delegates to `persist_bulk_decisions`"; §Plan
Task 1 (L99); §Gotchas (L110) "No bespoke persist — reuse `persist_bulk_decisions`".

**Source evidence:** `persist_bulk_decisions` is defined in **`crates/btctax-tui-edit/src/edit/persist.rs:394`**.
`btctax-cli`'s dependencies are `btctax-core`, `btctax-store`, `btctax-adapters` **only**
(`crates/btctax-cli/Cargo.toml`) — it does **not** (and must not) depend on `btctax-tui-edit`, which itself
depends on `btctax-cli` (`crates/btctax-tui-edit/Cargo.toml`). A CLI→tui-edit call is a circular dependency;
`btctax-cli` has zero references to `persist_bulk_decisions` (grep). The **shipped** CLI apply proves the
real pattern: `apply_bulk_self_transfer_in` (`crates/btctax-cli/src/cmd/reconcile.rs:273-294`) uses its
**own** N-append + single-`save` loop with bare-`?` atomicity — it does NOT call `persist_bulk_decisions`.
Only the TUI wrapper `persist_bulk_self_transfer_in` (`.../edit/persist.rs:452-469`) delegates to it. The
spec's own §CLI header even says "mirror the shipped bulk commands" — which contradicts "delegates to
`persist_bulk_decisions`". The claim is internally inconsistent and unbuildable for the CLI path.

**Concrete fix:** mirror the shipped split exactly.
- **CLI:** `cmd::reconcile::apply_bulk_classify_inbound_income` in `btctax-cli` uses its **own** append loop +
  single `save` (clone of `apply_bulk_self_transfer_in`, reconcile.rs:273). It does NOT use
  `persist_bulk_decisions`. Pass it the resolved FMV per row (see below), e.g.
  `apply_bulk_classify_inbound_income(vault, pp, rows: Vec<(EventId, Usd)>, kind, business, now)`, building
  `ClassifyInbound{ Income{ kind, fmv: Some(usd), business } }` per row.
- **TUI:** add a thin `persist_bulk_classify_income` in `btctax-tui-edit/src/edit/persist.rs` (mirror
  `persist_bulk_self_transfer_in`) that builds `Vec<EventPayload::ClassifyInbound{Income{…}}>` from the
  plan's `included` rows and delegates to `persist_bulk_decisions`. ("No new persist fn" is false — you need
  this wrapper, or inline payload-building at the modal call site.)
- **Carry the resolved fmv (hardening the #a defense):** derive each row's `fmv: Usd` from `plan.included`
  (where it is guaranteed `Some` by the exclusion) and pass it as a **non-`Option`** into the builder, so
  `Income{fmv:None}` is **structurally impossible** and there is no second `fmv_of` call-site that could
  reintroduce the #a bug. This also fixes the §Uniform notation error (`fmv: Some(fmv_of(date,sat))` is a
  type error — `fmv_of` already returns `Option<Usd>`; write `fmv: fmv_of(...)`, or `fmv: Some(row.fmv)`).

### [M1] MINOR — `FmvMissing` raise-site mis-cited; "ManualFmv clears it" is wrong for the inbound path
**Spec:** §Tax-safety (L34-35) "the engine re-fires a **Hard `FmvMissing`** blocker (resolve.rs:167 —
`ManualFmv` is what clears it)"; L40-41 "the user can set FMV manually …".

**Source evidence:** `resolve.rs:167` is a **doc comment** ("ManualFmv on an Income replaces the FMV …"),
not the raise site, and it describes the **native** `Income` payload path (`build_op` `EventPayload::Income`
arm, `resolve.rs:199-216`, which *does* apply `manual_fmv`). The actual raise for the **inbound** path is
`fold.rs:854` (fmv missing) / `fold.rs:833` (wallet missing). The `Op::IncomeInbound` arm (`resolve.rs:277`)
does **not** read `manual_fmv`, and a `ManualFmv` aimed at a classified TransferIn is rejected as Hard
`DecisionConflict` (`resolve.rs:476-495`). So `ManualFmv` cannot clear this blocker — only void+re-classify
(or `classify-inbound-income --fmv`) can.

**Concrete fix:** cite `fold.rs:853-860` (and `:832-838` for wallet-less) as the raise site; state the
remedy is "void the `ClassifyInbound` and re-classify with a real `fmv` (or set it via
`classify-inbound-income`'s own `fmv` field)", **not** `ManualFmv`. This strengthens, not weakens, the #a
rationale (the blocker is unfixable-in-place).

### [M2] MINOR — candidate-seed cite is wrong (`self_transfer_match_plan` enumerates outflows, not the income seed)
**Spec:** §Candidate set (L18) "the EXACT bulk-sti candidate set … `TransferIn` events (as
`self_transfer_match_plan` enumerates)"; KAT `bulk_income_plan_lists_pending_inbounds` (L83) "candidate =
TransferIn − already-classified − wallet-less".

**Source evidence:** `bulk_self_transfer_in_plan` seeds from **`state.blockers` where
`kind == UnknownBasisInbound`**, then joins to the raw `TransferIn` via the index
(`crates/btctax-cli/src/session.rs:569-573`). `self_transfer_match_plan` (session.rs:708) is a *different*
helper that enumerates pending **TransferOut**s for match proposals — it does not enumerate the income
candidate set. The blocker-seed is load-bearing: it naturally excludes link-consumed inbounds (`Op::Skip`,
no blocker) and successfully-classified inbounds, and it *re-includes* gift-basis-unknown rows (which
re-emit `UnknownBasisInbound` — hence the separate already-classified filter is still required).

**Concrete fix:** replace the parenthetical with "TransferIns still carrying a live `UnknownBasisInbound`
blocker (bulk-sti iterates `state.blockers`, `session.rs:569-573`)"; update the KAT prose to name the
blocker seed so an implementer does not seed from *all* TransferIns.

### [M3] MINOR — `fmv_status` is a phantom field on the inbound-income path
**Spec:** §Uniform (L48) "`fmv_status` = the auto/ingest status the single classify-income arm already
assigns (reuse it — do not invent a new status)."

**Source evidence:** `InboundClass::Income` carries only `{ kind, fmv, business }`
(`crates/btctax-core/src/event.rs:127-132`) — there is **no** `fmv_status` field. `fmv_status` exists only
on the native `Income` payload struct (`event.rs:59`). The single classify-inbound-income arm sets no
`fmv_status` (`crates/btctax-cli/src/main.rs:926-938`: `InboundClass::Income{ kind, fmv, business }`).

**Concrete fix:** delete the `fmv_status` sentence. The uniform payload is exactly `{ kind, fmv, business }`;
there is no status to reuse or invent. (Ties into I1's notation fix.)

### [M4] MINOR — no dedicated KAT for the wallet-less exclusion, which is *also* a Hard-`FmvMissing` vector
**Spec:** §KATs (L82-95) pins `bulk_income_plan_excludes_missing_price` and
`bulk_income_plan_excludes_already_classified`, but wallet-less is only mentioned inline in the candidate
KAT ("− wallet-less"). §Tax-safety frames wallet-less as merely "create no lot" (L24, L30-lineage).

**Source evidence:** a wallet-less `Op::IncomeInbound` raises Hard `FmvMissing` (`fold.rs:832-838`), and a
wallet-less unclassified TransferIn *does* carry `UnknownBasisInbound` (the `Op::UnknownInbound` arm has no
wallet gate — `fold.rs:815-821`), so it is a live candidate that must be excluded. This is the **same**
year-gating damage class as #a, yet it gets no dedicated test.

**Concrete fix:** add `bulk_income_plan_excludes_walletless` (a wallet-less inbound is not in `included`);
optionally fold it into the no-Hard-`FmvMissing` E2E. Correct the §Tax-safety wording: wallet-less income
inbound is a Hard-`FmvMissing` vector (`fold.rs:833`), not just "creates no lot".

### [N1] NIT — TUI "reuse `persist_bulk_decisions` directly, no wrapper" diverges from the shipped pattern
The shipped TUI flows each have a thin per-flow wrapper (`persist_bulk_self_transfer_in`,
`persist_bulk_link_transfer`) that delegates to `persist_bulk_decisions`. Building payloads inline at the
modal handler would work but breaks the established, unit-testable wrapper convention. Prefer a
`persist_bulk_classify_income` wrapper (see I1) for consistency and KAT reach.

### [N2] NIT — KAT names a non-existent field `Income.usd_fmv`
`bulk_income_apply_sets_autofmv` (L88-89) asserts "persisted `Income.usd_fmv == fmv_of(...)`". The persisted
field is `InboundClass::Income.fmv` (`event.rs:130`); `usd_fmv` is the projected lot basis / `IncomeRecord`
value (`fold.rs:847`). Reword to assert either the persisted `Income.fmv == Some(fmv_of(date,sat))` or the
projected `IncomeRecord.usd_fmv == fmv_of(date,sat)`.

---

## Adjudications requested by the task

- **Confirm-tier flag (§Confirm) — CONFIRMED / GREEN.** Reuse bulk-sti's tier; do NOT use Tier-B/typed-word.
  `handle_bulk_sti_modal_key` is an **explicit, non-typed Enter-confirm** modal ("explicit confirm; NOT
  typed" — `crates/btctax-tui-edit/src/main.rs:6157`, persist on `Enter`, cancel on `Esc`). This is the
  **revocable** tier, distinct from bulk-void / bulk-resolve-conflict, which are "Tier-B non-revocable"
  (main.rs:6440). `ClassifyInbound{Income}` is voidable (revocable), so it belongs in bulk-sti's revocable
  tier. The spec is correct — reuse, do not diverge.

- **Dispatch non-bypassable (§CLI L70, §Gotchas L111) — CONFIRMED.** The shipped CLI dispatch derives
  targets from `plan.included`, never raw refs: `plan = bulk_self_transfer_in_plan(...)` then
  `let in_events = plan.included.iter().map(|r| r.in_event.clone())`
  (`crates/btctax-cli/src/main.rs:1216,1244-1245`). There is no `--ref` argument to bypass with. Mirroring
  this for income means the fmv-exclusion (applied in the plan) cannot be bypassed. Good — and I1's
  "carry resolved fmv" fix makes it airtight (no second `fmv_of` site).

- **Second `ClassifyInbound` → Hard `DecisionConflict` (§Candidate, Q2) — CONFIRMED.** `resolve.rs:582-592`
  (duplicate `ClassifyInbound` on a TransferIn → `DecisionConflict`, first-wins, second excluded);
  `DecisionConflict` is Hard (`state.rs:74`). Excluding already-classified rows is mandatory, not cosmetic.

- **Over-reach / serde / lockstep (§Core, Q6) — CONFIRMED GREEN.** `InboundClass::Income` +
  `ClassifyInbound` already exist and derive `Serialize/Deserialize` (`event.rs:8,126-132,151-155`). The
  bulk flow only appends existing `ClassifyInbound` payloads. **No new `EventPayload` variant, no serde
  break, no btctax-core change.** The §6 zero-core-change claim holds.

## KAT coverage summary
Pins present and adequate for: the #a exclusion (`bulk_income_plan_excludes_missing_price` +
`bulk_income_apply_recognizes_income` E2E asserting *no* new Hard blocker), auto-FMV per row
(`bulk_income_apply_sets_autofmv`, reword per N2), already-classified exclusion, uniform kind/business,
empty-refuse, dry-run-writes-nothing, and the three TUI cases. **Gap:** the wallet-less exclusion has no
dedicated KAT despite being a Hard-`FmvMissing` vector (M4).

## What is already correct (no action)
- The three-way exclusion design (missing-price ∪ wallet-less ∪ already-classified) fully closes every
  Hard-blocker vector on the classify-income path — verified against `fold.rs` + `resolve.rs`.
- Candidate set otherwise mirrors `bulk_self_transfer_in_plan` faithfully (filter-3 + wallet-less).
- Payload shape, `IncomeKind` variants, and the revocable confirm tier are all correct.
- Zero btctax-core / serde change.

**Re-review required after folding I1 (+ Minors).** Not R0-GREEN this round.
