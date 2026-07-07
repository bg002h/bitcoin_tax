# R0 — SPEC review, round 1: what-if TUI overlay (task #43, phase P3)

**Artifact:** `design/SPEC_whatif_tui.md` (DRAFT).
**Baseline:** branch `feat/whatif-tui` @ `181e345` (main == `21f05ac`).
**Reviewer role:** independent architect (author ≠ reviewer). Read-only; no implementation.
**Scope of risk (per the spec's own framing):** a UI slice reusing the ALREADY-VERIFIED tax core
(`btctax-core::whatif::{sell,harvest}`). The risk is read-only/persistence + TUI integration, NOT tax math.
Every claim below is grounded in current source with `file:line` citations verified at write time.

---

## VERDICT

**0 Critical / 3 Important / 6 Minor / 2 Nit — NOT R0-GREEN.**

The spec's core judgement is right: the read-only / non-persistence invariant is **airtight** (Finding 1 —
SOUND), and the one data gap it identifies (prices) is the correct gap. But the **stated mechanism for the
prices fix is not implementable as written** (I1), the **regression scope mis-states the editor-crate impact**
(I2), and the panel **lacks an `at: TaxDate` input that `whatif` structurally requires** (I3). All three are
fixable at the spec level; none is a correctness/safety hazard. Re-review required after the fold.

---

## Per-question findings (grounded)

### 1. ★ Read-only / non-persistence invariant — **SOUND** (no finding)

The design is airtight. Evidence:

- **App holds no `Session`.** `attempt_open` drops the session immediately on success
  (`crates/btctax-tui/src/unlock.rs:156-159`, `drop(session)`), documented as the viewer's structural
  property "the viewer App never stores a Session" (`unlock.rs:91-92`, `unlock.rs:141`). `App` (`app.rs:125-159`)
  has no `Session` field. ✓
- **`whatif::{sell,harvest}` are clone-fold-discard.** `sell` (`whatif.rs:208-350`) and `harvest`
  (`whatif.rs:533-782`) call ONLY pure readers — `project`, `fold_as_of`, `synthetic_state`,
  `evaluate_disposal`, `compute_tax_year` — over `&[LedgerEvent]` + `&dyn PriceProvider` +
  `&ProjectionConfig`, and return owned reports. No `conn()`, no `save()`, no writer anywhere. The module
  header states the guarantee (`whatif.rs:1-16`) and the core carries a `whatif_never_persists` KAT. ✓
- **`handle_key` mutates only UI fields** (`app.rs:120-121` doc). Adding `App.whatif: Option<WhatIfPanel>`
  to the allow-list is **correct** and directly mirrors the existing `export_modal: Option<ExportConfirmState>`
  precedent (`app.rs:155`; modal-first dispatch at `lib.rs:171-174`). A `WhatIfPanel` holds only owned input
  state + an owned `Result<SellReport|HarvestReport, WhatIfError>` (all report structs derive `Clone`,
  `whatif.rs:90`, `435`) — it never borrows the snapshot, exactly like `ExportConfirmState`. ✓
- **No keystroke → writer path exists.** Recompute calls `whatif::{sell,harvest}` with borrows of the
  read-only `Snapshot` + owned locals; there is no `Session`/`conn()` in scope in the viewer at all. ✓
- **Byte-identical KAT is the right behavioral gate** — and it is not the only gate. The **existing
  mechanized source-gate KAT-E10** (`export.rs:715-945`) scans every non-test region of `btctax-tui/src/`
  and FAILS on `conn(`, `save(`, `write_form_csvs`, `fs::write`, `File::create`, … The new panel module will
  be scanned automatically, so it is *structurally* forbidden from containing any write-class token
  (see Minor M6 for the one caveat: keep it OUT of `export.rs`, the only exempt file). This is a **stronger**
  guarantee than the spec credits.

**Conclusion:** the invariant is sound; the byte-identical KAT + KAT-E10 + the App-has-no-Session structure
form a triple lock. No change required here other than the doc-comment allow-list extension (already planned)
and M6.

### 2. The prices gap + fix — **the gap is real; the stated FIX mechanism is not implementable → see I1**

- The `Snapshot` (`app.rs:104-116`) genuinely carries `events, state, cli_config, profiles, tables,
  donation_details, bulk_estimated` and **no** `PriceProvider`. Confirmed. ✓ (the diagnosis is correct)
- **But the stated mechanism — "built once in `build_snapshot` … via `session.prices()`" — cannot work.**
  `Session::prices()` returns `&dyn PriceProvider` (`session.rs:397-399`), a *borrow* tied to a `Session`
  the viewer *drops* (`unlock.rs:158`). `Snapshot` is an owned, `Box`-ed, moved value with no lifetime
  parameter (`app.rs:104`, `unlock.rs:77`), so it cannot store that borrow. And `PriceProvider`
  (`price.rs:5-8`) is a **single-method, non-`Clone`, non-enumerable** trait (`usd_per_btc(date)` only).
  From `&dyn PriceProvider` you therefore can neither (a) clone it into an owned `Box<dyn PriceProvider>`,
  nor (b) "materialize the daily-close map" (no iterator over dates exists). The spec's own two suggested
  routes are both blocked by the trait surface. **This is I1** — full fix below.

### 3. The whatif call wiring — mostly sound; two issues (I3 + M1)

- **`cli_config.to_projection()` exists and works from a `&Snapshot`.** `CliConfig::to_projection(self)`
  (`config.rs:38-46`) consumes by value, but `CliConfig` derives `Copy` (`config.rs:10`), so
  `snap.cli_config.to_projection()` copies out and compiles. ✓ Yields the exact `ProjectionConfig` whatif
  needs (`whatif.rs:161`, `config.rs:38-45`). ✓
- **`snap.profiles[selected_year]` is the right SOURCE but the wrong ACCESSOR.** `build_snapshot` loads
  `profiles` via `session.all_tax_profiles()` (`unlock.rs:174`) — the same data the CLI reads via
  `Session::tax_profile(year)` (`cmd/whatif.rs:83`), so the source is consistent. ✓ But literal
  `BTreeMap` indexing panics on a missing key; the common viewer case (no stored profile) would panic.
  Use `.get(&year)`. **Minor M1.**
- **The placeholder-with-caveat fallback is correct AND necessary.** With `profile: None`,
  `compute_tax_year` refuses: `TaxProfileMissing` (`compute.rs:269-275`), surfaced by `whatif` as
  `WhatIfError::YearNotComputable` (`whatif.rs:146`). So to render a computed report when no profile is
  stored, the panel MUST inject a placeholder `TaxProfile` — exactly the CLI's pattern
  (`cmd/tax.rs:16,90` `placeholder_tax_profile`; `cmd/whatif.rs:37-52` `adhoc_profile`). The spec's design
  is consistent (answers Finding 6). Note the CLI's `placeholder_tax_profile` is *private*, so the panel
  builds its own inline (trivial; see M-note in I3).
- **Does the panel have everything to call sell/harvest?** events ✓ (`snap.events`), prices ✗→I1,
  config ✓, profile ✓ (get-or-placeholder), tables ✓ (`snap.tables`), wallet — a picker over the pool ✓,
  price ✓ (optional). **year — NO: `whatif` needs a full `at: TaxDate`, not an `i32` year → I3.**

### 4. TUI integration — `w` is free; dispatch pattern is right; harvest recompute cost is understated (M2/M3)

- **`w` is free.** The Viewer arm (`lib.rs:207-256`) binds
  `q, Esc, Tab, BackTab, Up/k, Down/j, PageUp, PageDown, g, G, Left/h, Right/l, s, [, ], e`. `w` appears
  nowhere here, nor on Unlock/Locked. ✓ The open should mirror `e`'s guard "no-op if no snapshot"
  (`lib.rs:235-236`) — see M5.
- **The in-panel keys don't collide** because the panel must dispatch FIRST and *consume* the key while
  open — exactly the export-modal priority pattern (`lib.rs:166-174`). The spec says "mirrors
  `export_modal`," which is correct. `Tab/s/h` as mode-toggles are safe as long as the input sub-fields are
  numeric (BTC decimal) / pickers (not free text), so `s`/`h` never occur as data. See **M3** for the
  focus-model hardening.
- **Recompute-per-keystroke: fine for `sell`, optimistic for `harvest`.** `sell` = 2 engine folds
  (baseline + withhyp, `whatif.rs:167-178`) — cheap. `harvest` is a **segment walk**: one full-pool fold
  for edges + one probe per lot edge + a bisection loop, each probe a full
  `evaluate_disposal`+`synthetic_state`+`compute_tax_year` (`whatif.rs:599-745`). On a K-lot pool that is
  O(K + log(segment/τ)) full folds per recompute, on the same thread as the 100 ms draw loop
  (`lib.rs:552-559`). "A fold is fast" undersells harvest. **Minor M2.**
- Year nav via `[`/`]` is coherent as a *year selector*, but the year alone is insufficient input (I3).

### 5. KAT sufficiency + scope — good coverage; strengthen two KATs (M4/M5); scope holds ONLY under the I1 fix; **editor breakage is real (I2)**

- The seven listed KATs cover the risk surface well (★never-persists, sell/harvest render, btc-parse,
  error-renders-refusal, toggle, handle-key-invariant, build_snapshot_populates_prices). Recommended
  additions: **M4** (make `build_snapshot_populates_prices` a *parity* assertion), **M5** (`w`-no-snapshot
  no-op + no-profile placeholder+caveat).
- **Scope "btctax-tui only; core/cli UNCHANGED" is achievable — but ONLY via the I1 fix route** (re-materialize
  through existing *public* adapter + cli API). The spec's stated `session.prices()` route would force a
  new cli/core API and break that scope claim. Tie the scope claim to the corrected mechanism.
- **"the editor uses `open_session`/`render`, not `App`" is TRUE for `App` but MISLEADING for `Snapshot`.**
  Adding a mandatory field to the public `Snapshot` (all-pub fields, `app.rs:104-116`) breaks **every
  literal constructor** of `btctax_tui::app::Snapshot`, and the editor crate has ~10 of them:
  `crates/btctax-tui-edit/src/draw_edit.rs:5306`; `crates/btctax-tui-edit/src/main.rs:9169, 9217, 9299,
  9421, 9578, 9919, 13566, 13591` — plus btctax-tui's own test builders (`export.rs:200`, `tabs/tests.rs:44,
  861, 1480, 1662`, `lib.rs:1061, 1169, 1263`). The editor crate **will not compile** until all are updated.
  **This is I2** (the (cli/core) `Snapshot {` hits at `cli.rs:99`/`main.rs:448` are `ExportSnapshot`, and
  `transition.rs:19` is `UniversalSnapshot` — unrelated types; disambiguated).

### 6. Gaps — the `at` date (I3); no-profile consistency (answered, SOUND); wallet-across-pool (M3-adjacent)

- **No-profile consistency (SOUND):** without a stored profile, `whatif` returns
  `YearNotComputable(TaxProfileMissing)` (`compute.rs:269-275` → `whatif.rs:146`); the placeholder clears it —
  consistent with the CLI ad-hoc/placeholder profile (`cmd/whatif.rs:37-52`, `cmd/tax.rs:16,90`). The
  spec's "loud caveat when absent" is the right call; the numbers are then explicitly assumption-based. ✓
- **The `at`/price interaction (I3):** the spec says "default = bundled FMV **for the year**", but FMV is
  strictly per-DATE (`price.rs:7`, `usd_per_btc(date)`; `fmv_of(prices, at, sat)`), and `whatif` keys the
  as-of pool, the ST/LT boundary, and proceeds on `req.at` (`whatif.rs:216-243`, `541-595`). There is no
  "FMV for the year." **I3.**

---

## FINDINGS

### [I1 — Important] The prices fix names an unimplementable mechanism; use the adapter re-materialization route

**Evidence.** `Session::prices()` → `&dyn PriceProvider` (`session.rs:397-399`), a borrow of a Session the
viewer drops (`unlock.rs:158`). `Snapshot` is owned/no-lifetime (`app.rs:104`, `unlock.rs:77`).
`PriceProvider` has one method, no `Clone`, no enumeration (`price.rs:5-8`). Therefore "turn `session.prices()`
into an owned `Snapshot.prices`" and "materialize the daily-close map" are both impossible from the erased
borrow. The spec's diagnosis is right; the mechanism is wrong.

**Concrete fix (keeps btctax-core/cli source UNCHANGED — verified reachable public API):**
- Type the new field `pub prices: btctax_adapters::LayeredPrices` (owned; `#[derive(Debug, Clone)]` at
  `crates/btctax-adapters/src/price.rs:69`; `impl PriceProvider` at `price.rs:99`). btctax-tui already
  depends on btctax-adapters (`crates/btctax-tui/Cargo.toml:25`).
- In `build_snapshot` (`unlock.rs:170-191`), build it WITHOUT re-opening the vault and WITHOUT
  `session.prices()`:
  `let prices = btctax_adapters::LayeredPrices::load_with_cache(
       btctax_cli::price_cache::default_cache_path().as_deref())?;`
  `default_cache_path()` is public (`price_cache.rs:19`; `pub mod price_cache` at `btctax-cli lib.rs:12`) and
  is the EXACT path `Session::default_prices()` uses (`session.rs:350-355`). `load_with_cache` is public and
  pure/no-network (`price.rs:80-97`). The result is therefore **byte-identical to the session's provider**
  (same bundled dataset + same cache file), so the panel's baseline matches the viewer's own Tax tab (which
  used the session's layered prices via `load_events_and_project`).
- Call site: `whatif::sell(&snap.events, &snap.prices, &snap.cli_config.to_projection(), profile, &snap.tables, &req)`
  — `&LayeredPrices` coerces to `&dyn PriceProvider`. ✓
- **Pitfall to forbid in the spec:** do NOT build with `None` (that drops the cache overlay →
  bundled-only → the panel would silently disagree with the viewer's cached-price tabs). Always pass
  `default_cache_path()`.
- If a future maintainer prefers a `Box<dyn PriceProvider>` field, note it is NOT `Default`/ergonomically
  cloneable, which worsens I2's constructor fanout; the concrete `LayeredPrices` is the better choice.

### [I2 — Important] Regression scope mis-states editor impact; enumerate the cross-crate `Snapshot` constructor fanout

**Evidence.** The public `Snapshot` (all-pub fields, `app.rs:104-116`) is constructed literally in the editor
crate at `btctax-tui-edit/src/draw_edit.rs:5306` and `btctax-tui-edit/src/main.rs:9169, 9217, 9299, 9421,
9578, 9919, 13566, 13591` (~10 sites), plus btctax-tui's own tests (`export.rs:200`; `tabs/tests.rs:44, 861,
1480, 1662`; `lib.rs:1061, 1169, 1263`). Adding a mandatory field breaks the build at all of them. The spec's
note "the editor uses `open_session`/`render`, not `App`" is true of `App` but does not insulate `Snapshot`.

**Concrete fix.** Amend the spec's regression bullet to: "adding `Snapshot.prices` is a breaking change to the
public `Snapshot` struct; every literal constructor across `btctax-tui` (tests) AND `btctax-tui-edit`
(≥10 sites) must be updated." In the plan, add the mechanical constructor sweep as an explicit P3 step. The
`LayeredPrices` field type (I1) makes each fix a one-liner — tests can use
`btctax_adapters::LayeredPrices::load_with_cache(None).unwrap()` (bundled-only is fine for a fixture) or a
`BundledPrices::from_csv_str(..)`-backed value where a specific FMV is asserted.

### [I3 — Important] The panel supplies a year (`i32`) but `whatif` requires a full `at: TaxDate`

**Evidence.** `App` carries only `selected_year: i32` (`app.rs:137`). But `whatif::sell`/`harvest` key on
`req.at: TaxDate` for (a) the as-of pool (`fold_as_of(events, prices, config, req.at)` + `pool_key(req.at, …)`,
`whatif.rs:223-228`, `570-576`), (b) the short/long-term boundary (`acquired_at <= req.at`), and (c) daily-close
FMV (`fmv_of(prices, req.at, sat)`, `whatif.rs:241`, `595`). "FMV **for the year**" (spec) is unresolvable —
`PriceProvider::usd_per_btc` is per-DATE (`price.rs:7`). The CLI takes an explicit `--at` date
(`cmd/whatif.rs:70`).

**Concrete fix.** Give the panel an explicit `at: TaxDate` input (editable), defaulting to a stated
convention within `selected_year` — recommend Dec-31 of `selected_year` (or "today" clamped into the year for
the current year). Reword the price bullet to "default = daily close at `at`, editable; a future/off-dataset
`at` with no price surfaces `ProceedsRequired`" (the spec already has the ProceedsRequired half right,
`whatif.rs:241-243`, `cmd/whatif.rs:189-192`). `[`/`]` then steps the year and the `at` default tracks it.

### [M1 — Minor] `snap.profiles[selected_year]` panics on a missing key

Use `snap.profiles.get(&year)` (BTreeMap `Index` panics; the no-profile case is common in the viewer). Design
intent ("when present; else placeholder") is correct — only the notation is unsafe.

### [M2 — Minor] Harvest recompute cost per keystroke is understated

`harvest` is a multi-fold segment walk (`whatif.rs:599-745`), unlike `sell`'s 2 folds. Per-keystroke on a large
pool, on the draw thread (`lib.rs:552-559`), can lag. Recommend: recompute `sell` live, but debounce or gate
`harvest` behind an explicit compute/Enter key (or state it as an accepted risk after a measurement).

### [M3 — Minor] Specify the panel focus/routing model + panel-first dispatch

The panel has multiple sub-fields (amount, wallet picker, target selector) plus `Tab/s/h` mode-toggles. This is
richer than the single-buffer attest modal (`lib.rs:275-326`). Define: (a) panel dispatch runs BEFORE screen
dispatch and swallows keys while open (mirror `lib.rs:171-174`), so viewer `s`/`h`/`[`/`]` don't leak; (b) an
explicit "focused sub-field" so printable routing is unambiguous and a future free-text field (e.g. a wallet
*name* rather than a picker) won't collide with the `s`/`h` toggles. Also note the pool may span several
wallets (the picker default) — the sale is per-wallet by construction (`whatif.rs:224-228`), so make the
wallet an explicit, always-visible choice, not a silent default.

### [M4 — Minor] Strengthen `build_snapshot_populates_prices` to a parity assertion

Assert not merely "field is set" but that `snap.prices.usd_per_btc(d)` equals the session's provider for a
known bundled date `d` — this pins the I1 consistency property (re-materialized == session provider) and would
catch a regression to `None`-cache or a diverging load path.

### [M5 — Minor] Add two KATs: `w`-no-snapshot no-op, and no-profile placeholder+caveat

- `w` pressed before unlock (`snapshot.is_none()`) must not open/panic — mirror KAT-E8 (`lib.rs:1031-1044`).
- With `profiles` empty, the panel renders a COMPUTED report via the placeholder AND shows the caveat line
  (not a `YearNotComputable` refusal) — pins I3-adjacent behavior + the placeholder wiring
  (`compute.rs:269-275`).

### [M6 — Minor] Keep the panel in its own module (NOT `export.rs`) so KAT-E10 covers it

KAT-E10 (`export.rs:715-945`) exempts exactly one file — `export.rs` — from the write-class token ban. Put the
panel in e.g. `whatif_panel.rs` so the strict no-`conn(`/no-`save(`/no-write-token rule applies to it
automatically. Also extend the `app.rs:120` allow-list doc to name `whatif`, and note in the spec that KAT-E10
is a second structural gate on the panel (complementing the byte-identical KAT).

### [N1 — Nit] Note the pub-struct field addition is itself the breaking change

`Snapshot` is `pub` with all-pub fields; adding `prices` is a breaking change to btctax-tui's public API
independent of the P0 break. Fine within the 0.4.0 breaking cycle — worth one line so the SemVer note is exact.

### [N2 — Nit] Reword "FMV for the year" → "daily close at `at`" throughout

Two spots (Inputs bullet; Output/price default) say "FMV for the year," which conflicts with the per-date
provider (`price.rs:7`). Align with I3's `at` wording.

---

## What is already SOUND (do not re-litigate)

- The ★ read-only / non-persistence invariant (Finding 1): App-has-no-Session (`unlock.rs:91-92,158`) +
  clone-fold-discard core (`whatif.rs:1-16,208-350,533-782`) + KAT-E10 source gate (`export.rs:715-945`) +
  the byte-identical behavioral KAT. Triple-locked.
- `cli_config.to_projection()` from a `&Snapshot` (Copy — `config.rs:10,38`).
- The profile source (`all_tax_profiles`, `unlock.rs:174`) and the placeholder-clears-refusal design
  (`compute.rs:269-275`; `cmd/whatif.rs:37-52`).
- `w` is unbound (`lib.rs:207-256`) and the export-modal dispatch pattern is the right model to copy.
- The refusal taxonomy the panel must render verbatim (`whatif.rs:128-141`, `map_whatif_err`
  `cmd/whatif.rs:177-196`).

---

## Required before R0-GREEN

Fold I1, I2, I3 (and, since ceremony scales down but is never removed, the Minors M1–M6 as spec edits or
explicit accept-risk notes), then **re-review** (round 2). The invariant work is done; the remaining work is
making the prices mechanism, the cross-crate scope, and the `at` input correct on paper before P3 starts.
