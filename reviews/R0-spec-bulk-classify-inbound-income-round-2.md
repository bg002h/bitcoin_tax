# R0 — Spec review: bulk-classify-inbound-income (Cycle 4), round 2

**Artifact:** `design/SPEC_bulk_classify_inbound_income.md` (folded from round 1)
**Branch/base:** `feat/bulk-classify-inbound-income` @ `75913c4` (main == `c643ddd`)
**Reviewer role:** independent architect, read-only vs CURRENT source. Bar: 0 Critical / 0 Important.
**Round-1:** 0C / 1I / 4M / 2N — BLOCKED on I1. All findings folded by the author.

## Verdict: **0 Critical / 0 Important / 1 Minor / 1 Nit** — **R0-GREEN**

Every round-1 finding (the I1 blocker + M1–M4) is resolved against current source with no new drift.
Two residuals remain, both **non-blocking** (one Minor spec-hygiene contradiction, one carried-over Nit).
Neither touches the bar. **R0-GREEN — clear to plan/implement.**

---

## I1 (BLOCKER) — RESOLVED. CLI own-loop + TUI wrapper split; structural no-`None`.

The spec now prescribes the buildable split (§Persist L58-75, §CLI L83-91, §Plan Task 1 L122-125): CLI
`apply_bulk_classify_inbound_income` gets its OWN append-loop + single `save` (mirroring shipped
`apply_bulk_self_transfer_in`), NOT `persist_bulk_decisions`; a thin TUI `persist_bulk_classify_income`
wrapper delegates to `persist_bulk_decisions`; `plan.included` carries a RESOLVED `fmv: Usd` (non-Option)
and both builders emit `InboundClass::Income { fmv: Some(row.fmv), … }`. All three CONFIRM points hold:

- **(a) CLI cannot reach `persist_bulk_decisions` (original delegation truly unbuildable).** `btctax-cli`'s
  `[dependencies]` are `btctax-core`, `btctax-store`, `btctax-adapters` ONLY — no `btctax-tui-edit`
  (`crates/btctax-cli/Cargo.toml:16-27`). `persist_bulk_decisions` is defined in
  `crates/btctax-tui-edit/src/edit/persist.rs:394`. Grep of `crates/btctax-cli/` finds ZERO calls — the sole
  hit is a comment (`reconcile.rs:327`). CLI→tui-edit would be a dependency cycle. **Confirmed unbuildable;
  the fold is mandatory, not stylistic.**
- **(b) shipped `apply_bulk_self_transfer_in` is own-loop + single save.**
  `crates/btctax-cli/src/cmd/reconcile.rs:273-294`: `for in_event in &in_events { … append_decision(…)? }`
  then a single `session.save()?`. Bare `?` before `save` = a mid-batch failure returns with the in-memory
  session dropped, nothing on disk (CLI atomicity). It does NOT call `persist_bulk_decisions`. **Confirmed —
  the spec mirrors this exactly.**
- **(c) `Income{fmv:None}` structurally unrepresentable / `Some(row.fmv)` typechecks.**
  `InboundClass::Income.fmv` is `Option<Usd>` (`crates/btctax-core/src/event.rs:130`), and
  `fmv_of(…) -> Option<Usd>` (`crates/btctax-core/src/price.rs:13`). A plan that unwraps `Some` once at
  filter time into `fmv: Usd` and re-wraps `Some(row.fmv)` at both builders typechecks against
  `fmv: Option<Usd>`, and there is no second `fmv_of` call-site to reintroduce a `None`. **Confirmed — the #a
  exclusion cannot be defeated by a later construction bug.**

The TUI wrapper convention is real: `persist_bulk_self_transfer_in` (persist.rs:452) delegates to
`persist_bulk_decisions` (persist.rs:469), exactly the shape the spec's `persist_bulk_classify_income`
mirrors. No internal contradiction remains in §Persist/§CLI/§Plan. **I1 fully resolved.**

## M1 — RESOLVED. Raise-site re-cited to `fold.rs:853-860`; remedy is void+reclassify.

§Tax-safety L38-42 now cites the Hard `FmvMissing` raise at `fold.rs:853-860`
(`crates/btctax-core/src/project/fold.rs:854-858`, detail `"income inbound FMV missing"` in the `Op::IncomeInbound`
`fmv == None` arm) — **confirmed**; the stale `resolve.rs:167` doc-comment cite is gone. It correctly states
the remedy is void + reclassify (NOT `ManualFmv`): a `ManualFmv` aimed at a classified inbound is itself
rejected as Hard `DecisionConflict` because the target's effective payload is `TransferIn`, not `Income`
(`crates/btctax-core/src/project/resolve.rs:481-493`, whose detail literally reads "for a TransferIn
classified as income, set the FMV via classify-inbound-income … void this decision"). **Both confirmed.**

## M2 — RESOLVED. Seed is `state.blockers` where `kind==UnknownBasisInbound`; double-classify is Hard.

§Candidate L19-23 now seeds from "`state.blockers` with `kind == UnknownBasisInbound`
(`session.rs:569-573`, the bulk-sti seed)" — **confirmed** at
`crates/btctax-cli/src/session.rs:569-573` (`for b in &state.blockers { if b.kind != UnknownBasisInbound
{ continue } … }`); the stale `self_transfer_match_plan` reference is gone. The "second `ClassifyInbound`
→ Hard `DecisionConflict`" claim is confirmed at `resolve.rs:582-592` (duplicate `ClassifyInbound` on a
`TransferIn` → `DecisionConflict`, first-wins, second excluded). **Both confirmed.**

## M3 — RESOLVED. Phantom `fmv_status` removed; field list exact; no double-wrap.

§Uniform L53-56 now states `InboundClass::Income` has ONLY `{ kind, fmv, business }` and no `fmv_status`
field. Confirmed exact against `crates/btctax-core/src/event.rs:127-132`:
`Income { kind: IncomeKind, fmv: Option<Usd>, business: bool }` — three fields, no `fmv_status`. Because
`fmv` is `Option<Usd>` and `fmv_of` already returns `Option<Usd>`, the spec correctly writes
`fmv: Some(row.fmv)` (row-resolved `Usd`), never `Some(fmv_of(...))`. **Confirmed.**

## M4 — RESOLVED. `bulk_income_plan_excludes_wallet_less` KAT added.

§KATs L111-113 adds `bulk_income_plan_excludes_wallet_less`, framing wallet-less as a Hard-`FmvMissing`/
no-lot vector citing `fold.rs:833`. Confirmed: a wallet-less `Op::IncomeInbound` raises Hard `FmvMissing`
("income inbound without wallet") at `crates/btctax-core/src/project/fold.rs:832-838`, and a wallet-less
unclassified `TransferIn` still carries `UnknownBasisInbound` (`Op::UnknownInbound`, fold.rs:815-821, no
wallet gate) so it IS a live candidate that must be excluded. bulk-sti already excludes wallet-less at
`session.rs:585-587`. **KAT folded; the vector is real.**

## Spot-checks (no regression)

- **#a `fmv_of==None` exclusion intact.** §Candidate L25-27 + §Tax-safety + structural guarantee L70-75 all
  retain "MINUS every row where `fmv_of == None`, reported as `excluded_missing_price`, never dropped." The
  behavioral delta vs bulk-sti is genuine: bulk-sti INCLUDES `None` rows (pushes `usd_fmv: Option<Usd>` and
  merely counts `missing_price_count`, `session.rs:598-611`). **Intact.**
- **Dispatch derives from `plan.included` (non-bypassable).** §CLI L90-91 + §Gotchas L136 preserve this;
  it mirrors the shipped `plan.included.iter().map(|r| r.in_event.clone())` at
  `crates/btctax-cli/src/main.rs:1244-1245` (no `--ref` bypass exists). Combined with I1's resolved-fmv plan,
  the fmv-exclusion is unbypassable. **Confirmed.**
- **Confirm-tier = bulk-sti's revocable, non-typed tier.** §Confirm L77-81 reuses bulk-sti's tier and
  explicitly refuses Tier-B/typed-word. `handle_bulk_sti_modal_key` is "explicit confirm; NOT typed"
  (`crates/btctax-tui-edit/src/main.rs:6157`, Enter→persist / Esc→cancel). `ClassifyInbound{Income}` is
  voidable, so the revocable tier is correct. **Confirmed — reuse, no divergence.**

---

## Residuals (non-blocking)

### [M1-r2] MINOR — §Gotchas L135 still says "No bespoke persist — reuse `persist_bulk_decisions`"
This blanket gotcha contradicts the (now-correct) §Persist/§CLI/§Plan, which require the CLI to use its
OWN append-loop — a bespoke persist for the CLI path — precisely because it cannot reach
`persist_bulk_decisions` (the exact defect I1 fixed). The load-bearing prescriptive sections are all
correct and unambiguous, so this is a stale leftover, not a re-opening of I1; but an implementer citing
L135 in isolation could be misled. **Fix:** reword to scope it to the TUI, e.g. "TUI reuses
`persist_bulk_decisions` via a thin wrapper; the CLI must NOT (wrong crate — R0-I1) and uses its own
append-loop + single save." One-line hygiene edit; does not gate.

### [N1-r2] NIT — carried-over from round-1 N2: KAT `bulk_income_apply_sets_autofmv` names `Income.usd_fmv`
§KATs L110 asserts an included row's "persisted `Income.usd_fmv == fmv_of(date, sat)`". The persisted
field is `InboundClass::Income.fmv` (`Option<Usd>`, event.rs:130); `usd_fmv` is the PROJECTED lot basis /
`IncomeRecord` value (fold.rs:847). Reword to either "persisted `Income.fmv == Some(fmv_of(date,sat))`" or
"projected `IncomeRecord.usd_fmv == fmv_of(date,sat)`." Cosmetic; the KAT intent is clear.

## What is already correct (no action)
- The three-way exclusion (missing-price ∪ wallet-less ∪ already-classified) closes every Hard-blocker
  vector on the classify-income path — re-verified against `fold.rs` + `resolve.rs`.
- I1 split (CLI own-loop + TUI wrapper) matches the shipped bulk-sti pattern byte-for-byte in shape.
- Structural no-`None` guarantee (resolved `fmv: Usd` in `plan.included`, `Some(row.fmv)` at both builders).
- Revocable confirm tier, non-bypassable dispatch, zero btctax-core / serde change.

**R0-GREEN.** The two residuals are Minor/Nit spec-hygiene items the author may fold at will; they do not
gate the plan phase.
