# SPEC — cleaner reconciliation defaults: HIFO global default + long-term self-transfer-in acquisition

**Source baseline:** `main` @ `b976621` (branch `feat/reconcile-defaults`). **Review status: DRAFT — awaiting
R0.** User-mandated tax-policy default change (2026-07-05), updating [[self-transfer-completion-policy]]. Two
small code changes, tax-critical, with a LARGE test blast radius. btctax-core (+ the config default surface).

## Goal & rationale (user-mandated)
The auto-reconcile estimate was needlessly punitive: every unknown-basis deposit defaulted to **short-term**
(acquired = receipt date) under **FIFO**. But the overwhelming majority of received BTC is a **long-held
cold-storage deposit** (almost nobody is paid in BTC, and almost all such coins are held ≥ 1 year), and **HIFO
is the most common elected method**. So make both defaults realistic:
1. **Default method FIFO → HIFO** (GLOBAL — real + auto-reconcile; the fallback when no per-exchange/global
   election is on file).
2. **Self-transfer-in acquisition default = 1 year + 1 day before the receipt date** (→ always long-term), for
   BOTH the pseudo default AND the manual `classify-inbound-self-transfer` (when no explicit acquired date is
   supplied).

**Tax direction (state honestly):** both changes REDUCE the estimate (HIFO minimizes gain; long-term lowers the
rate) — the estimate moves from "maximally conservative" to "realistic for a long-term hodler." The basis stays
$0 (still conservative on the gain AMOUNT). This REVISES the prior conservative short-term policy — intended.

## Change 1 — HIFO as the global default method
- Flip the config/profile FIFO fallbacks to `LotMethod::Hifo`: `project/mod.rs:54` (config `pre2025_method`
  default), `mod.rs:125` (forward-method default when no election), `event.rs:487` (default TaxProfile/config),
  `persistence.rs:434`. Both the pre-2025 method AND the forward method default to HIFO.
- **Do NOT touch the deliberate-FIFO MECHANIC** `pools.rs:63` (`consume(.., Fifo, None)` — acquisition-date
  order for a specific relocation/removal mechanic, NOT the electable tax method) or the `LotMethod::Fifo`
  *variant* itself.
- **[serde back-compat — R0 assess]** `event.rs:187`: `#[serde(default)] -> Fifo for pre-A.7 records`. If the
  serde default flips to HIFO, a stored config/profile serialized WITHOUT an explicit method deserializes as
  HIFO — silently changing an existing vault's method on load. That is the user's intent (HIFO is now the
  default), but R0 must confirm whether any real stored config relies on the serde default vs always writing an
  explicit method, and whether the flip should be at the serde-default layer, the projection-default layer, or
  both. Surface it as a deliberate, documented behavior change either way.
- **[compliance note]** HIFO on a return requires specific-ID + adequate records; the default stays
  `attested: false` (config already surfaces this) so the user is prompted to affirm it per exchange. Keep that.

## Change 2 — long-term self-transfer-in acquisition (one line)
- **`fold.rs:1019`** `let acq = acquired_at.unwrap_or(date);` → default to **`date − (1 year + 1 day)`** when
  `acquired_at` is `None`. This single site is the common default for BOTH the pseudo `SelfTransferMine{$0}`
  synthetic AND the manual `classify-inbound-self-transfer` (both resolve to `Op::SelfTransferInbound` with
  `acquired_at: None`, resolve.rs:380-386). An explicitly-supplied `acquired_at` (Some) is unchanged.
- **Compute:** `date.replace_year(date.year()-1)` then `− 1 day` (handle Feb-29 → Feb-28), or `date −
  Duration::days(366)` — exact length is IMMATERIAL (the user: "whether it's 1 or 2 years doesn't matter"), the
  only requirement is **guaranteed long-term**: since any sale is on/after `date`, holding ≥ 366 days ⇒ always
  long-term. Use a saturating/checked sub (BTC dates are 2009+, no real underflow).
- **[ordering interaction — R0/impl note]** `acquired_at` is the FIFO sort key AND the HIFO tie-break
  (pools.rs:254-284). Backdating self-transfer-in lots by ~1 year reorders them relative to real-acquired lots
  (a backdated 2017 deposit now sorts as a 2016 lot). Under HIFO (all $0-basis pseudo lots tie on basis), the
  acquired-date tie-break then drives selection. This is a real behavior change; KAT the resulting order.

## Test blast radius (the plan OWNS this — R0 to ENUMERATE exhaustively)
Both changes shift tax outcomes that many tests pin:
- **HIFO default:** every test that projects WITHOUT an explicit method election and asserts a gain/lot-selection
  now under HIFO instead of FIFO (grep `LotMethod`, disposal-gain assertions, `pre2025_method`, the KATs in
  btctax-core/tests + btctax-cli/tests + the adapters rate-engine KATs). Tests that specifically want FIFO must
  set an explicit FIFO election (not rely on the default).
- **Long-term acquired:** every test asserting a self-transfer-in lot's `acquired_at == receipt` or a `Term`
  (short↔long) or a holding-period on such lots.
R0 must list the affected test files (like the price-fmv C1 enumeration) so T1 migrates them; expect this to be
the bulk of the work.

## KATs
- `default_method_is_hifo` (no election → HIFO; config shows `Hifo (attested: false)`); an explicit FIFO
  election still yields FIFO. `pools.rs:63` mechanic stays FIFO (unchanged behavior for its op).
- `self_transfer_in_defaults_to_long_term` (unknown-basis inbound, no acquired date → lot acquired 1yr+1day
  before receipt, term = long on any later sale); `explicit_acquired_date_supersedes_the_default`.
- `manual_classify_inbound_self_transfer_also_long_term` (the CLI path, no `--acquired` → long-term).
- **★ fault-inject:** revert `fold.rs:1019` to `unwrap_or(date)` ⇒ `self_transfer_in_defaults_to_long_term`
  RED; revert the method default to Fifo ⇒ `default_method_is_hifo` RED.

## Scope / SemVer / lockstep
btctax-core (the 4 method-default sites + fold.rs:1019) + the migrated tests. No new public API. **Behavior
change** to tax outcomes for default (unelected/undated) inputs → a notable MINOR; a CHANGELOG/README note +
update [[self-transfer-completion-policy]] memory. No CLI-flag change (config already prints the method).

## Plan (TDD)
- **T1** — flip the 4 method defaults + `fold.rs:1019`; the KATs + the ★ fault-injects; **migrate the enumerated
  FIFO-default / acquired-date / term tests** (set explicit FIFO elections where a test needs FIFO; update
  expected gains/terms for the now-HIFO-long-term defaults). Whole-diff + full suite + CHANGELOG + FOLLOWUPS +
  memory.

## Gotchas
- **HIFO reduces gain; long-term lowers rate — both cut the estimate** (user-mandated realistic default; not a bug).
- **The mechanic FIFO (`pools.rs:63`) stays FIFO** — only the electable-method DEFAULT flips.
- **serde-default flip silently shifts unset stored configs to HIFO** — intended, but document it (R0).
- **Backdated acquired_at reorders lots** for FIFO/HIFO selection — KAT the order.
- **Guarantee long-term** (holding ≥ 1yr+1day); saturating sub; Feb-29 edge.
- **HIFO stays `attested: false`** until the user affirms it (specific-ID/records) — keep the surface.
- **Large test blast radius** — the plan owns the migration; R0 enumerates.
