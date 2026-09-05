# RECON — Where should USD-value validation live so a new ingestion path cannot skip it?

**Agent:** recon (read-only). **Branch:** `main` @ `edeb70b5`. **Date:** 2026-09-05.
**Scope:** design investigation for FOLLOWUPS.md FR-38. No tracked file modified, nothing committed.

---

## ANSWER IN ONE LINE

**Put the total, context-free half of the check (the sign refusal) at the persistence boundary —
`btctax_core::persistence::insert` (`crates/btctax-core/src/persistence.rs:131`), the sole
`INSERT INTO events` in the workspace (`:153`) — written as an exhaustive destructuring match with no
`..`, so a new `Usd` field on any payload is an `E0027` compile error; and leave the contextual half
(the `sats_as_dollars_advisory`, which needs sat + date + prices) at the surfaces, because the
persistence boundary provably does not have the right date for a decision payload.** Only the
persistence seam makes the omission *impossible*; the destructuring makes a *new field* compile-time
detectable; everything else is merely *currently covered*.

---

## 1. THE INVENTORY OF DOORS

Every path by which a USD value reaches a persisted event. Counted, not estimated: there is exactly
**one** `INSERT INTO events` (`persistence.rs:153`), reached from exactly **three** `insert(&tx, …)`
calls (`:200` import, `:226` conflict, `:259` decision), reached from exactly **two** public append
functions (`append_import_batch:172`, `append_decision:238`), which have **40 production call sites**
(non-test, counted by partitioning each file at its first `#[cfg(test)]`):

| file | production callers |
|---|---|
| `crates/btctax-tui-edit/src/edit/persist.rs` | 23 |
| `crates/btctax-cli/src/cmd/reconcile.rs` | 13 |
| `crates/btctax-cli/src/chokepoint/mod.rs` | 2 |
| `crates/btctax-cli/src/cmd/import.rs` | 1 |
| `crates/btctax-cli/src/cmd/optimize.rs` | 1 |

Of those 40, the ones that can carry a **filer- or file-supplied** USD figure are the doors below.

### D1 — CSV / XLSX import (the biggest door, and unvalidated)

`crates/btctax-cli/src/cmd/import.rs:15-17` is 20 lines end to end:

```rust
let batch = ingest_files_bundled(files)?; // adapters: detect→group→parse→normalize (FR2/FR3)
let mut session = Session::open(vault_path, pp)?;
let import = append_import_batch(session.conn(), &batch.events)?; // ATOMIC batch (FR1)
```

There is **no USD validation anywhere between the CSV cell and the vault row.** The money parser
`btctax_adapters::parse::parse_usd` (`crates/btctax-adapters/src/parse.rs:22-51`) *deliberately
preserves* negatives, including the accounting-parens form:

```rust
let (neg, body) = match t.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
    Some(inner) => (true, inner),
    None => (false, t),
};
…
if neg { d.set_sign_negative(true); }
```

Sat quantities are `.abs()`-ed by every adapter; **USD is not**, except in Gemini:

| adapter | USD → payload | sign-guarded? |
|---|---|---|
| Gemini | `usd_cost: usd_abs` `gemini.rs:147`, `usd_proceeds: usd_abs` `:157`, `fee` `:107` | **yes** — `.abs()`, with a written rationale ("I-2: abs-normalize … Type fixes the field's role … Applied only in this Gemini parser") |
| Coinbase | `usd_cost: subtotal` `coinbase.rs:153`, `usd_proceeds: subtotal` `:162`, `fees` `:141-144` | **no** |
| Swan | `usd_cost: cost` `swan.rs:194` and `:248`, `fee` | **no** |
| River | `usd_cost: cost` `river.rs:139`, `fee` `:130-133` | **no** |

★ This is the B3 shape again, inside one crate: **the fix already exists in the branch** (Gemini's
`.abs()`, reasoned out in a comment) and was never carried to the sibling three parsers, because no
reviewer held all four at once.
(River/Swan `Income` is safe by construction — its FMV comes from `resolve_fmv`/`fmv_of`, not the file.)

### D2 — CLI `reconcile classify-raw` (the FR-38 door)

`crates/btctax-cli/src/cmd/reconcile.rs:633-656`. A serde parse and a variant check, nothing else —
as the brief states. Dispatched from `main.rs:1254-1257`; the flag is `cli.rs:690-701`, whose help
text advertises `usd_cost` and `fee_usd` as free-form decimal strings.

### D3 — CLI flag decisions (the FR-37 surface — validated)

`classify_inbound` (`reconcile.rs:68`), `reclassify_outflow` (`:207`), `set_fmv` (`:309`).
Sign-refused at parse via `eventref::parse_nonneg_usd_arg` (`crates/btctax-cli/src/eventref.rs:88-96`,
wired at `main.rs:1174/1192/1198/1210/1239/1242/1250`) **and** advised via
`sats_as_dollars_advisory` (`reconcile.rs:287-305`) at its six call sites (`:177`, `:243`, `:341`,
plus the unit-test reference `:1590`). This is the only fully-covered door.

### D4 — TUI classify-inbound / reclassify-outflow / set-fmv (sign only, never advised)

`crates/btctax-tui-edit/src/edit/form.rs:691` `parse_nonneg_usd`, used at `:733`, `:760`, `:765`,
`:801`, `:953`, `:959`, `:1112`. **`sats_as_dollars_advisory` has zero TUI call sites** — a grep for
its name or for the message text `"did you enter the sats amount as dollars"` returns exactly one
file, `reconcile.rs`. So the entire TUI is an unadvised surface for the same five FR-37 fields.

### D5 — TUI classify-raw (★ NEW: not sign-checked either — wider than FR-38 says)

`form.rs:1859` `validate_classify_raw_acquire` and `:1893` `validate_classify_raw_income` build the
payload with the **raw** parser, not the guarded sibling that lives 1,168 lines above them in the
same file:

```rust
let usd_cost = Usd::from_str(uc).map_err(|_| format!("bad USD {uc:?}"))?;   // form.rs:1870
…
let fee_usd = … Usd::from_str(t) …                                          // form.rs:1876
…
let v = Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?;            // form.rs:1904 (income fmv)
```

FR-38 is written as a CLI-only finding. It is not: **the TUI classify-raw path admits a negative
`usd_cost` / `fee_usd` / `usd_fmv` too**, through a function whose name is `validate_…`. Same file,
same crate, same review-window blindness as the adapters above.

★ Also unguarded on both classify-raw doors: **`sat`**. `form.rs:1846` `parse_required_sat` is
`t.parse::<i64>()` with no positivity check, and the CLI JSON door has none either. `Sat` is `i64`
(`conventions.rs:6`) and nothing in `btctax-core` refuses a negative `sat` on a payload (the only
`sat > 0` predicates in core are on `remaining_sat` in the pool/fold layer).

### D6 — `accept-conflict` (a door with no code of its own)

`reconcile.rs:657` appends only `SupersedeImport { conflict_event }` — but its *effect* is to promote
the USD-bearing `ImportConflict.new_payload` (stored at `persistence.rs:222-230`) into force. There is
no place in this verb to validate anything; the value must have been checked when the
`ImportConflict` row was written. **Only a persistence-boundary check covers this door at all.**

### D7 — the promote/declare chokepoint (computed, and legitimately signed)

`chokepoint/mod.rs:475` (`apply_promote`) / `:603` (`apply_declare`). `PromoteTranche.filed_basis` is
computed at record time, not typed. Its `acknowledgment.shown_terms` carry **legitimately negative**
USD — see §4.

### D8 — bulk paths (safe by construction, worth preserving)

`apply_bulk_classify_inbound_income` (`reconcile.rs:799`) and `apply_bulk_reclassify_outflow` (`:874`)
re-derive every USD from `price::fmv_of` and `continue` when there is no price. No typed USD can enter.

### D9 — `import-selections` CSV (`reconcile.rs:1228`)

A second CSV door, but it carries `sat` only (header asserted `disposal_ref,origin_event_id,split_sequence,sat`).
No USD. Note it does `sat_str.trim().parse::<i64>()` with no sign guard, same as D5.

### Adjacent, out of scope

`return_inputs` / `return_inputs_draft` / `tax_profile` / `donation_details` / `optimize_attestation`
are side tables, not events (their own `INSERT INTO`s, `donation_details.rs:145`, `tax_profile.rs:57`,
`return_inputs.rs:127`, `input_form_store.rs:79`). `what-if` / `consult` persist nothing.

---

## 2. THE COMPLETE USD FIELD SET ON `EventPayload`

Enumerated from `crates/btctax-core/src/event.rs` (`grep -n Usd`), **20 direct fields plus 2 recursive
carriers**. `Usd` is a **type alias**, not a newtype: `pub type Usd = Decimal;` (`conventions.rs:8`).

| # | field | site | polarity |
|---|---|---|---|
| 1 | `Acquire.usd_cost` | `event.rs:58` | ≥ 0 |
| 2 | `Acquire.fee_usd` | `:59` | ≥ 0 |
| 3 | `Income.usd_fmv: Option` | `:65` | ≥ 0 |
| 4 | `Dispose.usd_proceeds` | `:73` | ≥ 0 |
| 5 | `Dispose.fee_usd` | `:74` | ≥ 0 |
| 6 | `ReclassifyOutflow.principal_proceeds_or_fmv` | `:125` | ≥ 0 |
| 7 | `ReclassifyOutflow.fee_usd: Option` | `:126` | ≥ 0 |
| 8 | `InboundClass::Income.fmv: Option` | `:137` | ≥ 0 |
| 9 | `InboundClass::GiftReceived.donor_basis: Option` | `:141` | ≥ 0 |
| 10 | `InboundClass::GiftReceived.fmv_at_gift` | `:143` | ≥ 0 |
| 11 | `InboundClass::SelfTransferMine.basis: Option` | `:152` | ≥ 0 |
| 12 | `ManualFmv.usd_fmv` | `:166` | ≥ 0 |
| 13 | `AllocLot.usd_basis` | `:177` | ≥ 0 |
| 14 | `AllocLot.dual_loss_basis: Option` | `:180` | ≥ 0 |
| 15 | `ConsentTerm::ComputedTax.delta_usd` | `:355` | **SIGNED** |
| 16 | `ConsentTerm::ComputedTax.deduction_delta_usd: Option` | `:356` | **SIGNED** |
| 17 | `ConsentTerm::Uncomputable.gain_delta_usd` | `:361` | **SIGNED** |
| 18 | `ConsentTerm::Uncomputable.deduction_delta_usd` | `:362` | **SIGNED** |
| 19 | `ConsentTerm::Unrealized.hypothetical_reduction: Option` | `:367` | ≥ 0 |
| 20 | `PromoteTranche.filed_basis` | `:405` | ≥ 0 |
| R1 | `ClassifyRaw.as_: Box<EventPayload>` | `:212` | recursive — carries 1–5 |
| R2 | `ImportConflict.new_payload: Box<EventPayload>` | `:96` | recursive — carries 1–5 |

**Rows 15–18 are the finding that constrains the whole design.** They are differences, and
`conservative_promote.rs:367` computes one as `let delta_usd = t_without - t_with;` — a promote that
*raises* tax yields a negative. So **"every `Usd` on a payload is ≥ 0" is false**, and any blanket
type-level or serde-level non-negativity is wrong. The policy must be **per field**, which is exactly
the ruling already on the books: *"Guard **per-flag** … never in the shared parser"*
(`SPEC_post_v070_product_cycle.md:182-184`, `[G-I5, T-M1]`), echoed in `eventref.rs:81-87`.

---

## 3. CANDIDATE SEAMS, RANKED

The ranking axis the brief asked for: does the seam make the omission **impossible**, **compile-time
detectable**, or merely **currently covered**?

### (a) Another call site in `classify_raw` — *currently covered*

Cost: ~10 lines. Closes D2 only. Leaves D1 (four adapters), D5 (TUI classify-raw), D6 entirely open,
and leaves the 41st call site free to skip it. This is the pattern that produced the bug; FR-38 says
so itself. **Reject as the answer**, keep as a possible stopgap.

### (b) `serde(deserialize_with)` on the USD fields — *worse than nothing; reject*

Three independent defeaters, in ascending order:

1. **It misses most doors.** D1, D4, D5 and every bulk path construct `Acquire {…}` in Rust. Serde
   never runs. Only D2 (the JSON string) is covered.
2. **It would brick existing vaults.** `load_all` reads every row back through the *same* impl:
   `let payload: EventPayload = serde_json::from_str(&payload_json)?;` (`persistence.rs:290`). A
   deserializer that rejects a negative turns any already-stored negative into an
   **unloadable vault**, not a refused entry.
3. It cannot host the advisory (no sat, no date, no prices).

### (c) A newtype that cannot be constructed without validation — *impossible-by-construction for the sign, but expensive and half-blind*

`Usd` is an alias (`conventions.rs:8`) with **3,306 references across 124 files**, so `Usd` itself
cannot become the newtype. A separate `NonNegUsd(Decimal)` applied to fields 1–14/19/20 would be
genuinely unconstructible-without-validation, and `#[serde(transparent)]` keeps the wire format. But:

- Blast radius, counted: the ten field names total **1,410 references** (`usd_cost` 224, `fee_usd`
  353, `usd_proceeds` 75, `usd_fmv` 190, `fmv_at_gift` 68, `donor_basis` 70,
  `principal_proceeds_or_fmv` 72, `usd_basis` 178, `dual_loss_basis` 81, `filed_basis` 99). Every
  arithmetic read needs an unwrap or a `Deref`, and a `Deref` re-opens the hole on the write side.
- **★★ It structurally cannot carry the advisory.** The brief's hard constraint bites hardest here: a
  bare `NonNegUsd` has no satoshi quantity and no date, so the 100×-market-value check cannot live in
  its constructor. A newtype buys the *cheap* half of the check at the *highest* price.
- It does not cover `sat`.

### (d) `EventPayload::validate()` called by every ingestion path — *currently covered, dressed as structural*

One predicate, N call sites — the FR-37 shape. The brief asks for "a test that proves every path
invokes it". That test cannot be written honestly: proving "every ingestion path calls `validate`"
requires enumerating ingestion paths, and *the enumeration is the thing that goes stale* — which is
precisely how FR-38 happened (six call sites wired, the seventh unimagined). A grep-based "every
`append_*` caller also calls `validate`" lint is a text lint over the 40 sites and cannot see a 41st
written tomorrow. **This is (a) with more ceremony.**

### (e) Validating at the persistence boundary — *impossible to omit, and new fields are compile-time detectable*

`persistence.rs:131 fn insert(...)` is the **only** `INSERT INTO events` in the workspace, and every
door above — including D6, which has no code of its own — necessarily passes through it. A payload
that has not been validated cannot become a row.

Context available at that seam, honestly:

| the check needs | available at `insert`? |
|---|---|
| the USD values | **yes** — `ev.payload` |
| the sat quantity | **yes** — same payload, incl. through `ClassifyRaw.as_` and `ImportConflict.new_payload` |
| a date | **partly**: `ev.utc_timestamp` + `ev.original_tz` are the *event's* date for an import, but for a **decision** they are the decision's creation time (`append_decision(…, now, …)`), **not** the target event's date — the exact anchor FR-37 established as the requirement |
| a price provider | **no** — `PriceProvider` is a core trait (`price.rs:5-8`), but neither append fn takes one; prices live on the CLI `Session::prices()` (`session.rs:433`) |

So the seam supports the **refusal** completely and the **advisory** only by (i) threading
`&dyn PriceProvider` through `append_decision`/`append_import_batch` — which reds all 40 production
call sites plus ~70 test sites, and (ii) doing a `load_all` target lookup inside core to recover a
decision's anchor date. That is a real design change, not a fix. **Split the two halves.**

**The compile-time-detection property, machine-verified** (not asserted — `rustc --edition 2021` on a
minimal reproduction, baseline compiled, then a `Usd`-shaped field planted):

```
error[E0027]: pattern does not mention field `new_usd_field`
 --> …/e0027.rs:3:9
  |
3 |     let Acquire { sat, usd_cost, fee_usd, basis_source } = a;
  |         ^^^ missing field `new_usd_field`
```

So if the validator destructures every payload struct with **named fields and no `..`**, adding a USD
field to `Acquire` (or any other) **does not compile** until someone classifies it — which is the
doctrine's "prefer designs in which an omission does not compile", achieved. Two honest caveats:
rustc's own help text offers `..` as the escape hatch, so "no `..` in this file" remains a convention —
but it is a **one-file, greppable** convention rather than a 40-call-site one; and a new *variant* of
`EventPayload` is caught separately by writing the outer `match` with no `_` arm.

**In-repo precedent for exactly this seam choice.** `SPEC_post_v070_product_cycle.md:207-212` chose
the write chokepoint for the TIN/EIN checks — *"Validate at the **shared side-table write choke
point** `donation_details::set` — the single point BOTH the CLI … and the TUI-edit form … converge
on"* — carrying an as-built correction that the earlier CLI-only cite *"is CLI-only — the TUI persists
via `persist_donation_details` → `donation_details::set`, bypassing it"*. `donation_details.rs:136-139`
implements it: `set()` calls `validate_and_normalize(details)?` before the `INSERT`. This report
recommends applying the ruling the repo already made, to the event log.

### Ranking

| seam | omission | new USD field | covers D1 | D2 | D5 | D6 |
|---|---|---|---|---|---|---|
| (a) call site | currently covered | silent | ✗ | ✓ | ✗ | ✗ |
| (b) serde | currently covered (JSON only) + **breaks vault load** | silent | ✗ | ✓ | ✗ | ✗ |
| (c) newtype | **impossible** (sign only) | `E0308` | ✓ | ✓ | ✓ | ✓ |
| (d) `validate()` + N calls | currently covered | silent | ✓* | ✓* | ✓* | ✗ |
| **(e) persistence** | **impossible** | **`E0027`** | ✓ | ✓ | ✓ | ✓ |

\* only for as long as the enumeration holds.

---

## 4. WHAT BREAKS

**On-disk format: unchanged. Digest: unchanged. This is not a migration.** Reasoning, checked:

- A `validate` that only **reads** `&EventPayload` and returns `Result` alters no `Serialize` impl, no
  column, no DDL (`persistence.rs:98-115`).
- The `fingerprint` (`persistence.rs:25-95`, `usd_cost` at `:60`) is computed from the same values and
  is part of the `Conflict` `EventId` (`:214`). It stays byte-identical **as long as validation
  refuses and never normalizes.** ★ **A `.abs()` or a rounding "fix" at this seam WOULD change
  fingerprints, hence conflict event identities — that is a migration.** So the rule is: refuse,
  never repair. (Which also means Gemini's existing `.abs()` at `gemini.rs:147/157` must be left
  alone, or its change of behaviour reasoned about separately.)
- Read-side is untouched: `load_all` (`persistence.rs:290`) keeps deserializing anything already
  stored, so no existing vault becomes unloadable. The invariant is forward-only, which is correct —
  it is a *record-time* gate, matching `guard_decision_conflict`'s posture (`reconcile.rs:37-45`).

**What must not be broken by the validator itself:** rows 15–18 of §2. A blanket rule reds
`ConsentTerm`. The four signed fields must be explicitly whitelisted **with the reason written down**,
per `[G-I5]`.

**Existing fixtures:** a grep for negative USD literals on payload fields
(`usd_cost: dec!(-`, `fee_usd: dec!(-`, `usd_proceeds: dec!(-`, `usd_fmv:`/`fmv: Some(dec!(-`,
`usd_basis: dec!(-`, `principal_proceeds_or_fmv: dec!(-`, `filed_basis: dec!(-`) returns **zero hits**
across `crates/`, and no `"usd_cost":-…` style value appears in any `.json` fixture. The 158 `dec!(-`
hits in the tree are all tax-computation values (`tax/compute.rs`, `capital_loss_carryover.rs`,
`other_taxes.rs`), none on an event payload. **A sign refusal at `insert` should red nothing.** This
is a grep-level claim, not a suite run — the build gate for the actual change must run `make check`.

**`CoreError`:** adding an `InvalidPayload { field, value, reason }` variant is free — a grep for
`CoreError::…` in a match arm returns **0** exhaustive matches; every consumer uses `#[from]`.

---

## 5. RECOMMENDATION

**Take (e). Split the check in two, and be explicit that only the first half is structural.**

1. **Refusal — `btctax_core::event::EventPayload::check_field_polarity()`, called from
   `persistence::insert` (`persistence.rs:131`), before the `INSERT`.** Total, pure, price-free,
   date-free. Outer `match` with **no `_` arm**; each arm destructures with **named fields and no
   `..`**; recurses into `ClassifyRaw.as_` and `ImportConflict.new_payload`; the four signed
   `ConsentTerm` fields bound and explicitly discarded with the `[G-I5]` reason in a comment. Covers
   D1, D2, D5, D6 and every future door in one place, and reds `E0027` when a USD field is added.
2. **Advisory — leave `sats_as_dollars_advisory` at the surfaces**, and close its two real gaps
   (D2 and D5) as *surface* work, because only the surface holds sat + date + prices. State plainly
   in the follow-up that the advisory is *covered*, not *structural*, and why: the anchor date FR-37
   requires is not reconstructible at the persistence boundary for a decision payload.

### The smallest first step that does not require the whole design

**One commit, no new architecture:** add `check_field_polarity` covering **only the six imported
payload variants** (fields 1–5 plus `sat > 0`) — the recursion targets — and call it from
`persistence::insert`. That alone closes D1's three unguarded adapters, D2, D5 and D6 simultaneously,
because all four funnel through the imported-payload path or through `ClassifyRaw`/`ImportConflict`
recursion. The decision-payload arms (fields 6–20) can land in a second commit without changing the
seam.

**Its B1 kill-test** — the one-sentence factual answer to *"which test reds when this checker is
removed?"* — must be a **binary-level** test, not a unit test, and there must be **one per door**,
because a unit test on the predicate would pass on all four doors while three of them still bypass it:
`btctax reconcile classify-raw --payload-json '{"Acquire":{…,"usd_cost":"-1"}}'` refuses; `btctax
import` of a Coinbase CSV with `(1,234.56)` in *Subtotal* refuses; the TUI
`validate_classify_raw_acquire` path refuses. Each mutation-verified RED with the `insert` call site
deleted. (The FR-37 harness `crates/btctax-cli/tests/fr37_sats_in_basis_field_cli.rs` is the pattern.)

---

## 6. REFUSAL OR ADVISORY, PER DOOR

The distinction that decides this is not CLI-vs-TUI-vs-file. It is **"is the value structurally
impossible?"** versus **"is the value merely improbable?"** — the same line FR-37 drew.

**Sign → REFUSE at every door, no exceptions.** A negative cost basis does not exist
(`eventref.rs:84-86`: §1012; §1016 floors adjustments at zero; §301(c)(2)–(3)/§733 excess-of-basis is
*gain*). There is no filer for whom `-1` is the truthful answer, so a refusal can never destroy a
legitimate entry, and — per the *"an entry is testimony"* rule — silently recording an impossible
figure fabricates testimony. The CLI already refuses on the named flags (`parse_nonneg_usd_arg`); the
TUI already refuses on classify-inbound (`form.rs:691`). **The doors that do not refuse are
inconsistent with the doors that do, and consistency argues for levelling up, not down.**

**Magnitude (sats-as-dollars) → ADVISORY at every door, including the JSON one.** The tempting
argument is that a machine-supplied JSON payload deserves a refusal because no human is watching. It
is wrong, for a reason specific to this predicate: `sats_as_dollars_advisory` is a **100× heuristic
against a market price**, and its own doc comment concedes the anchor is approximate
(`reconcile.rs:275-283`). A filer with a genuinely enormous basis, or a date whose price is missing,
would be **refused from filing a true figure** — and the wrong outcome would be worse than telling
them nothing, which is the journey-walk test for whether a divergence earns a change. Keep it a
warning, and keep the "no price for this date" NOTE (`reconcile.rs:301-303`), which is what stops the
guard from dying silently.

**One asymmetry worth conceding to the machine-payload argument:** a JSON payload is typically
scripted, and stderr on a scripted run is frequently discarded. If that matters, the right lever is a
`--strict` flag that promotes advisories to refusals on `classify-raw` — an **opt-in**, not a default,
and out of scope for the first step.

**D1 (the file import) is the one door where the advisory should probably not fire per-row at all** —
a 10,000-row CSV would emit 10,000 warnings. Fold it into the existing per-source `FileReport`
counters (`adapter.rs`, surfaced by `cmd/import.rs`) as a count with the first few examples. The
refusal, by contrast, should fire per row and abort the batch: `append_import_batch` is already
atomic (`persistence.rs:176` `unchecked_transaction`), so a `?` inside the loop rolls the whole
import back, which is the correct all-or-nothing behaviour for a file that contains an impossible
figure.

---

## 7. FINDINGS BEYOND FR-38 (for the follow-up file)

1. **The TUI classify-raw path is unguarded too** — `form.rs:1859`/`:1893` use bare `Usd::from_str`
   while `parse_nonneg_usd` sits at `:691` in the same file. FR-38 as written would be closed by a
   CLI-only fix that leaves this open.
2. **Three of four adapters do not sign-guard USD** — Coinbase `:153`/`:162`, Swan `:194`/`:248`,
   River `:139`; Gemini does (`:147`/`:157`), with the rationale already written out. `parse_usd`
   actively produces negatives from accounting parens (`parse.rs:29-36`).
3. **`sat` is unguarded on both classify-raw doors** (`form.rs:1846`, and the CLI JSON), and on
   `import-selections` (`reconcile.rs:1272-1275`). Nothing in core refuses a negative `sat` on a
   payload. This travels with the same fix at the same seam and should not be split off.
4. **`sats_as_dollars_advisory` has zero TUI call sites.** FR-37 closed the CLI half of its own
   finding; the TUI half was never opened.
5. **Never `.abs()` or round at the persistence seam** — it would change `fingerprint`
   (`persistence.rs:60`) and therefore conflict `EventId`s (`:214`). Refuse, never repair.
