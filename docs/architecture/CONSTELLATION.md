---
title: BTCTAX-CONSTELLATION
section: 7
header: Code Map
footer: btctax 0.7.0
---

# NAME

btctax constellation code map — a curated guide to the load-bearing modules and their
key functions across the twelve-crate workspace.

# HOW TO USE THIS

This is a **navigation map**, not an exhaustive API dump. For each crate it lists the
load-bearing modules (skipping boilerplate and pure test files) with a one-line purpose
and the key public functions/types, each with a terse "what + why". Line numbers are
current as of 0.7.0 and are navigation hints, not contracts.

- **Human**: browse by layer and crate to learn where a responsibility lives.
- **LLM / tooling**: grep a concept (e.g. `would_conflict`, `to_golden`, `resolve_election`)
  to jump to the owning file, then open it. The companion `ARCHITECTURE.md` explains the
  *why* of the whole system; this map tells you *where*.

Layers, bottom-up: **Foundation** (core, store) → **Domain** (adapters, forms,
input-form) → **App** (cli, tui, tui-edit) → **Tooling** (update-prices, oracle-harness,
xtask). The `btctax` crate is a name reservation only (`crates/btctax/src/lib.rs`, ~14
lines, no API).

# FOUNDATION

## btctax-core — domain model + the two tax engines

The pure, total, event-sourced domain. Its only I/O is `persistence.rs`.

**`src/event.rs`** — the event taxonomy. `LedgerEvent { id, utc_timestamp, original_tz,
wallet, payload }` (immutable, append-only) and `EventPayload` — one sum type with three
families: 6 *imported* (`Acquire`, `Income`, `Dispose`, `TransferOut`, `TransferIn`,
`Unclassified`), 1 *system* (`ImportConflict`), and 13 *decision* variants (the user
reconciliation verbs). Decision variants are forward-only (an old binary fails loudly on
a newer vault's unknown variant, ~`:214`).

**`src/identity.rs`** — `EventId` (structured, injective: `Import{source, source_ref}` /
`Conflict{…, fingerprint}` / `Decision{seq}`), `WalletId` (`Exchange{provider,account}` /
`SelfCustody`), and `Fingerprint` (SHA-256 — used ONLY for conflict detection, never
identity). Source tie-break priority Swan>Coinbase>Gemini>River (~`:14`).

**`src/state.rs`** — the projection output. `LedgerState` (lots, holdings_by_wallet,
disposals, removals, income_recognized, pending_reconciliation, blockers, `FoldStats`),
`Lot`/`LotId`, `DisposalLeg`, `Removal`, and `Blocker`/`BlockerKind` with
`severity() -> Hard | Advisory` — every open question is a typed blocker.

**`src/conventions.rs`** — money & dates. `Usd = Decimal`, `Sat = i64`, `TaxDate`;
`round_cents` (half-even, engine) vs `round_dollar` (half-up, filed forms) — two
deliberately distinct regimes; `TRANSITION_DATE` (2025-01-01); `split_pro_rata`
(remainder-takes-rest, conserves Σbasis); `long_term_default_acquired` (leap-safe).

**`src/persistence.rs`** — the crate's ONLY I/O, over a borrowed `Connection`.
`append_import_batch` (~`:172`, atomic, fingerprint-idempotent, emits `ImportConflict`
on changed re-import), `append_decision` (~`:238`, `seq = MAX+1`), `load_all`.

**`src/project/mod.rs`** — the projection contract. `project(events, prices, config) ->
LedgerState` (~`:63`, = resolve then fold), `LotMethod`, `FeeTreatment` (default (c),
comment-fenced "DO NOT change" ~`:51`), `ProjectionConfig`, `would_conflict` (~`:107` —
record-time conflict prediction that runs the real projection twice and diffs; "this IS
the resolver", never a hand-rebuilt subset), `pseudo_plan`.

**`src/project/resolve.rs`** — PASS 1, decision resolution. `Op` (the effective-timeline
operation enum), `build_op` (~`:258`, imported payload → `Op`), `resolve_election`
(~`:175` — the SOLE two-tier method resolver: latest wallet-scoped election, else latest
global, else HIFO), the void/conflict/classification stages, pseudo synthesis
(`PseudoKind` ~`:219`), and the 2025 transition-effectiveness decision (~`:1181`).

**`src/project/fold.rs`** — PASS 2. `fold_event` (~`:554`, the ONE dispatcher shared by
the real fold, the transition pre-fold, `pools_before`, and `state_as_of` — eliminates a
class of divergence bugs), `consume_principal` (~`:52`, pro-rata legs + §1015 dual-basis),
`consume_fee` (~`:323`, TP8 treatments), `applicable_method` (~`:33`, HIFO default),
`finalize` (~`:1285`), and the read-model helpers `pools_before`/`state_as_of`.

**`src/project/pools.rs`** — the lot pool. `PoolSet`, `PoolKey` (Universal pre-2025,
per-`WalletId` after), `method_order`/`hifo_cmp` (~`:275`, gain-basis-per-sat via
cross-multiply, no float), `take_from` (conserves Σbasis), lot-selection validation
(existence / cross-wallet / over-draw → `LotSelectionInvalid`).

**`src/project/transition.rs`** — the 2025 Path A/B transition. `universal_snapshot`
(~`:32`, the pre-2025 residue computed under the allocation's own recorded method, via
the same `fold_event` so it provably matches — the conservation guard for safe-harbor).

**`src/project/conservation.rs`** — `conservation_report` (~`:37`, FR9:
Σin = Σdisposed + Σremoved + Σheld + Σfee_sats + Σpending). **`src/project/compliance.rs`** —
`disposal_compliance` (~`:92`, per-disposal contemporaneity; never launders post-hoc ID).
**`src/project/evaluate.rs`** — `evaluate_disposal` (~`:98`, clone→inject-synthetic→fold→
read→discard, for the optimizer/what-if).

**`src/tax/compute.rs`** — the crypto-**delta** engine (FROZEN behind SHA pins).
`compute_tax_year` (~`:232`, refuses on any Hard blocker projection-wide → then missing
table → then missing profile), `net_1222` (~`:137`, §1222/§1211/§1212), `ordinary_tax_on`
(exact marginal brackets), `preferential_tax` (§1(h) 0/15/20 stacking), NIIT (§1411).
Pinned identity `total = ord_delta + ltcg_tax + niit`.

**`src/tax/return_1040.rs`** — the **absolute** full-1040 engine (additive; never edits
the frozen engine). `assemble_absolute` (~`:1035`), `derive_tax_profile` (~`:724`, from
non-crypto lines only → structurally can't double-count crypto), `screen_absolute`
(~`:1395`, compute-dependent refuse screens).

**`src/tax/questions.rs`** — the `FORM_QUESTIONS` registry (~`:80`): each jurat
declaration owns its prompt, `RefuseReason`, refusal detail, **the ONLY copy of its
liveness predicate**, and get/set. The answered-ness invariant's single source of truth.

**`src/tax/classifier.rs`** — the compile-time answered-ness guard: destructures every
field reachable from `ReturnInputs` with **no `..`** under `deny(unused_variables)`
(~`:1`), so a new bool/`Option<bool>`/defaulted-enum is a compile error until classified.

**`src/tax/printed.rs`** / **`src/tax/packet.rs`** — the printed line chains (`round_dollar`
at each line, cross-footing over already-rounded lines) and `PrintedReturn`. **`src/tax/
forms.rs`** — pure year-scoped Form 8949 / Schedule D / 8283 row projections over
`state.disposals`/`removals`. **`src/tax/types.rs`** — `TaxProfile`, `TaxResult`,
`TaxOutcome::{Computed, NotComputable}`.

**`src/tax/{se,other_taxes,charitable,qbi,amt,method,tables,return_inputs,return_refuse}.rs`** —
the schedules & satellites: `se.rs` (§1401 SE tax, standalone), `other_taxes.rs`
(8959/8960 + the Additional-Medicare unbundle), `charitable.rs` (§170(b) ceilings +
5-year carryover), `qbi.rs` (Form 8995), `amt.rs` (the refuse-**screen**
`amt_should_file_6251` — 6251 is never *filled*, Sch 2 L2 is $0 by construction),
`method.rs` (L16 Tax Table / worksheet), `tables.rs` (indexed vs statutory constants),
`return_inputs.rs` (the non-crypto 1040 inputs), `return_refuse.rs` (typed `Refusal`s for
unmodelled inputs). **`src/tax/frozen_guard.rs`** — SHA-256 content pins on the frozen
delta engine.

**`src/optimize.rs`** — `optimize_year`/`consult_sale`/`score_assignment` (rate-aware lot
assignment minimizing attributable tax within §1.1012-1(j); notes §1091 wash-sale
inapplicable, monitored). **`src/whatif.rs`** — non-persisted `sell`/`harvest` marginal
planning (both full `compute_tax_year` runs). **`src/price.rs`** — the `PriceProvider`
trait + `fmv_of`. **`src/donation.rs`**, **`src/void.rs`** — donation details, void helpers.

## btctax-store — the encrypted vault

Domain-blind persistence: one opaque encrypted SQLite image + a keypair.

**`src/vault.rs`** — `Vault { path, cert, conn, _lock }`. `create`/`open`
(crash-recovery + classified `.bak` restore), `save` (serialize→encrypt→atomic write),
`snapshot`/`restore` (in-memory rollback primitive), `export_snapshot` (plaintext,
owner-only), `backup_key`. Distinguishes wrong-passphrase from corruption via a decrypt
`unlocked` flag.

**`src/crypto.rs`** — sequoia-openpgp glue; `Passphrase` (zeroize on drop); S2K
iterated-salted SHA-256 (no Argon2 in Sequoia 1.x). **`src/blob.rs`** — the `[u32 version
|| SQLite image]` envelope; `migrate` (identity-or-refuse at v1). **`src/sqlite_io.rs`** —
serialize/deserialize the in-memory DB; OOM remapped to `Io` so the corruption
classifier never treats OOM as corruption. **`src/atomic.rs`** — the atomic write
(`.tmp`→fsync→copy target to `.bak`→rename→fsync dir). **`src/fsperms.rs`** — the single
authority for 0o600/0o700 owner-only perms (`open_owner_only`). **`src/lock.rs`** — the
`fs2` single-instance exclusive lock (Windows codes 32/33). **`src/memlock.rs`** —
`SecretBuf` (mlock/VirtualLock + zeroize). **`src/paths.rs`** — the sidecar path family.

# DOMAIN

## btctax-adapters — ingestion + bundled datasets

Turns exchange exports into `LedgerEvent`s; bundles the price dataset + tax tables.

**`src/adapter.rs`** — the `Adapter` trait (`detect → group → parse → normalize`) +
`SourceFile`/`FileGroup`/`GroupOutput` (with the FR2 drop/unclassified counters).
**`src/ingest.rs`** — `ingest_files`/`ingest_files_bundled` (~`:27`); detection order
Swan→Coinbase→River→Gemini (Gemini extension-only, runs last); `UnknownSource` on no
match. **`src/read.rs`** — format-agnostic `RawRow`; CSV preamble scan; XLSX via calamine
(Excel serials, shortest-round-trip). **`src/parse.rs`** — `parse_usd` (exact, accounting-
negative), `parse_btc_to_sat` (half-even sub-sat quantities only), `parse_timestamp`(`_flex`).
**`src/normalize.rs`** — `resolve_fmv` (the FR3 ladder: export USD → dataset → Missing),
`SourceRefMint` (deterministic refs), `exchange_wallet`.

**`src/sources/{coinbase,gemini,river,swan}.rs`** — the four adapters. Doctrine:
**conservative, never guess** — ambiguous types → `Unclassified` (kept + counted).
Coinbase (Buy→Acquire, Sell→Dispose, Send/Receive→Transfer, else Unclassified); Gemini
(XLSX; BTCUSD-gated trades, Credit/Debit→Transfer, non-USD pairs→Unclassified); River
(Buy/Income/Interest/Withdrawal by `Tag`, else Unclassified; income auto-valued from the
dataset); Swan (three roles by header signature; sent-side BTC→Unclassified).

**`src/price.rs`** — `BundledPrices` (compiled-in daily-close CSV, exact-date lookup, no
gap-fill) + `LayeredPrices` (local cache layered over bundled, cache wins); no network,
no path logic (the caller resolves the cache path). **`src/tax_tables.rs`** —
`BundledTaxTables` (2017/2024/2025/2026, Rev-Proc-verbatim + pinned by tests) +
`BundledFullReturnTables` (TY2024 only, fail-closed).

## btctax-forms — the paper layer

Fills official IRS PDFs, offline, byte-deterministic, geometry-verified on read-back.
Never recomputes tax.

**`src/lib.rs`** — the fill entry points: `fill_form_8949`, `fill_schedule_d`,
`fill_schedule_se`, `fill_form_8283`, `fill_form_1040_capgains` (crypto slice) and the
`*_full` variants + `fill_form_1040_full`. Each conditional schedule's filing decision
is a **core** fact (`must_file`), never the filler's.

**`src/pdf.rs`** — the lopdf mechanism: strip `/XFA`, set `/NeedAppearances`, walk the
AcroForm leaf tree (FQN + `/Rect` + `/FT` + `/MaxLen`), apply `/V` (fail-closed on a
missing field), drop `/Info` dates + trailer `/ID` for byte-determinism.

**`src/verify.rs`** — the signature idea: *"the map is what we distrust; the PDF's
geometry is the oracle."* Two map-independent read-back oracles on the serialized bytes —
`verify_8949` (grid: re-derives column/row bands from the blank PDF's widget `/Rect`s) and
`verify_flat` (page membership + column clusters + descending-y + `/MaxLen`-in-characters,
plus a map-independent Digital-Asset Yes/No oracle). **`src/transcribe.rs`** —
`extract_lines` (the inverse line-keyed transcriber: right *value* in the right box).

**`src/map.rs`** — the committed per-(form,year) TOML field maps (logical cell → AcroForm
FQN); maps are DATA (a new year is a `forms/<year>/` dir, not code). **`src/packet.rs`** —
`fill_full_return` (all-or-nothing — any refusal → zero bytes; exhaustive no-`..`
destructure; ordered by IRS Attachment Sequence No.). **`src/watermark.rs`** —
`stamp_draft` (~`:21`, the DRAFT overlay, keyed to pseudo-reconciliation, not the full
return). **`src/cells.rs`** / **`src/overflow.rs`** — per-cell SSN/`MaxLen` rendering;
multi-copy pagination with root-field renaming. The per-form modules (`fill8949.rs`,
`fill8949_full.rs`, `form1040.rs`, `form1040_full.rs`, `form8283.rs`, `form8959.rs`,
`form8960.rs`, `form8995.rs`, `schedule_*.rs`) transcribe one form each. **No
`serde_json` anywhere** (money stays exact `Decimal`).

## btctax-input-form — the authoring engine

A UI-agnostic model/controller for authoring `ReturnInputs` ("TUI now, web app later";
depends on core only).

**`src/seam.rs`** — the stable wire: `SectionId`/`FieldId` enums (never Vec indices),
owned serde `FieldValue`/`Edit`; secrets are presence-only on read (`SecretView`,
5+-digit run rejected). **`src/apply.rs`** — the anti-laundering core: `Working =
Option<ReturnInputs>` and the ONLY accepted first edit is `SetField{FilingStatus}`
(~`:1`) — "a return exists" is a type-level fact, so commit can't see a laundered
`default()`. **`src/parse.rs`** / **`src/attribute.rs`** — the validation tiers (tier-1
syntax; tier-3 exhaustive `RefuseReason → Anchor` map — a new refuse reason fails to
compile until placed). **`src/spec/{mod,sections,registries,coverage}.rs`** — `form_spec()`
(twelve §9A-ordered sections; two synthetic registry-driven), the macro delegation to the
core registry (no predicate written twice, `registries.rs`), and the observation-based
coverage KAT (`coverage.rs` — a new struct field goes red until given a Field or exempted).

# APP

## btctax-cli — the composition root (`btctax` binary + library)

The only place vault + adapters + core + forms are wired together.

**`src/cli.rs`** — the clap-4 command surface (all verbs + the `reconcile`/`pseudo`
sub-verbs); doc-comments single-source `--help` + man pages. **`src/main.rs`** — thin
dispatch on a 64 MiB worker thread; the passphrase + `BTCTAX_NOW` seams (~`:64`); the
bulk-preview renderers + confirm loops; exit codes 0/1/2. **`src/lib.rs`** — `CliError`
(incl. `PathIo{path,hint,source}`, `FormFill` read-back failure, the stale-schema
refusals) + `require_attestation` (pure exact-compare) + `ATTEST_PHRASE`.

**`src/session.rs`** — `Session` (~`:331`, "the single seam every command opens": one
`Vault` + a `PriceProvider`); `project()`/`load_events_and_project()`; the read-only bulk
**plan** computers (append/persist nothing) that main.rs previews then `apply`s from plan
rows (exclusions can't be bypassed). **`src/resolve.rs`** — `resolve_and_screen`
(~the single screened resolver: `ReturnInputs` → stored `TaxProfile` → pseudo → missing;
fail-closed), reused by the CLI *and* both TUIs' snapshots.

**`src/render.rs`** — the shared formatters (3,984 lines): `render_report`,
`build_verify`/`render_verify`, the tax/schedule/optimizer/what-if/events renderers, the
CSV writers (`write_form_csvs`, `write_csv_exports`), and the helpers reused by the TUIs
(`describe_inbound_class`/`describe_outflow_class`, `wallet_label`, `lot_method_display`,
`fee_treatment_display`, `filing_status_tag`). `PseudoDisclosure` drives the `[PSEUDO]`
banner on every number-bearing surface.

**`src/cmd/{tax,reconcile,admin,inspect,answer,optimize,whatif,import,init}.rs`** — one
module per command family. `reconcile.rs` (~1,742 lines): each fn builds exactly ONE
decision variant, `append_and_save`, guarded by `guard_decision_conflict` (~`:46`, the
record-time `would_conflict` check = the resolver by construction); `void` co-clears the
attest/`[est]` markers atomically. `admin.rs`: `export-irs-pdf` (+ the unconditional
not-authorised stderr notice, DRAFT/attest gate), `mkdir_out`. `inspect.rs`:
`events list` (UX-P4-11 ref discoverability). `answer.rs`: `income answer` (Schedule-B
tri-states have no safe default). **`src/input_form_store.rs`** — the draft/committed
side-table: `commit` (the I-11 finalize guard — `NoTables` per-year, never poisons a
table-less year), draft-shadows-committed (§6.1), park/discard. **`src/return_inputs.rs`** —
the `return_inputs` side-table (refuse-and-reimport at the single read boundary).
**`src/eventref.rs`** — `EventId` ref parsing. **`src/{config,tax_profile,donation_details,
optimize_attest,bulk_estimated,price_cache}.rs`** — the typed side-tables.

## btctax-tui — the read-only viewer (`btctax-tui` binary + library)

Opens the vault, builds an immutable `Snapshot`, **drops the session** — a write is
structurally unreachable.

**`src/lib.rs`** — `run_viewer` + the event loop (`poll(100ms)` → `handle_key` → `draw`),
the exported reusable surface, the export-modal + what-if key handling, the `Clock` resolve
before raw mode. **`src/app.rs`** — `App`, `Screen`/`Tab`, and `Snapshot` (~`:104`: events,
`LedgerState`, config, per-year *resolved+screened* `profiles`/`refused`, tables, prices —
built with the SAME resolver as the CLI). **`src/unlock.rs`** — `open_session`/
`build_snapshot`; `attempt_open` drops the Session immediately (byte-identical-vault
test). **`src/tabs/{holdings,disposals,income,tax,forms,compliance}.rs`** — each an
App-free `pub fn render(frame, area, &Snapshot, year, …)` (the seam the editor composes);
Tax renders `refused` years as "NOT COMPUTABLE", never $0. **`src/export.rs`** — the four
owner-only form CSVs (+ the no-write-token source gate). **`src/whatif_panel.rs`** /
**`src/sort.rs`** — the read-only what-if overlay; display-only column sort.

**`src/clock.rs`** — `Clock { Wall, Pinned }` (~`:17`); `from_env()` mirrors the
`BTCTAX_NOW` contract. **`src/capture.rs`** — `to_golden(&Buffer) -> String` (~`:29`,
`pub` so both crates share it): the glyph grid + the style-run overlay (`cell_sig`; a
signature is `(symbol, fg, bg, modifier)`, deterministic modifier order). The heart of
the golden system.

## btctax-tui-edit — the editor (`btctax-tui-edit` binary)

The mutating reconcile/authoring UI; holds the live Session + lock; writes only via the
single choke-point.

**`src/main.rs`** — a ~26k-line monolith: `EditorApp` event loop, the strict priority
key-dispatch ladder (help → ~22 modal gates → ~23 flow gates → tax-inputs → profile →
screen), the debounced draft autosave on idle, the Browse keymap opening ~two-dozen
flows, the
tax-inputs commit outcomes (`Committed`/`Refused`/`NoTables`/`Err`), plus the
`no_direct_now_utc_in_production` structural scan + `production_now_utc_lines` (~`:14103`)
and the golden emit tests (`capture_edit_frame` ~`:14195`, `emit_btctax_tui_edit_goldens`).
**`src/editor.rs`** — `EditorApp` state; the "at most one flow Some" invariant; the
quit-first error latches (`attest_save_failed`, `rollback_failed`). **`src/draw_edit.rs`** —
the editor rendering (Browse `[EDITOR]` badge + red pseudo banner; the tax-inputs form's
answered/unanswered glyphs; the status NOTICE line). **`src/edit/persist.rs`** — the ONLY
module permitted to name the mutation surface (`conn()`/`save()`/`append_decision`);
`save_or_rollback` snapshots + reverts on save failure (KAT-G1 forbids vault constructors
crate-wide). **`src/edit/form.rs`** / **`src/edit/tax_inputs.rs`** — the form-flow model
(field kinds, RowAddr, validation) driving `btctax-input-form`.

# TOOLING

## btctax-update-prices — the online price updater

The **only** network-linked crate. **`src/lib.rs`** — the HTTP client (ureq/rustls;
Binance primary, CoinGecko fallback; 8-day settling lag; forward-only append into the
local cache CSV). **`src/main.rs`** — the thin binary. Depends only on `btctax-adapters`
(reuses `BundledPrices::max_date`/`contains` to compute the fetch window).

## btctax-oracle-harness — the double-oracle test harness (`publish = false`)

**`src/main.rs`** — a stdin/stdout JSON contract: DEFAULT mode assembles+fills+reads a
scenario back off the paper (`extract_lines`) → flattened line map; `--check` mode
compares btctax's printed figures against two external oracles (OpenTaxSolver + PSL
Tax-Calculator) via the same `oracle_diff` helpers the golden tests use — so the Python
sweep never re-implements btctax's arithmetic.

## xtask — dev tooling (`publish = false`)

**`src/docs.rs`** — man pages + PDFs from the clap doc-comments (`Cli::command()` +
clap_mangen). **`src/examples.rs`** — the worked-examples generator (the J1–J9 journey
functions + corpora consts; runs the real `btctax` binary in a hermetic temp dir; the
`is_demonstrated` SOFT coverage matcher) + the byte-gated golden. **`src/check_isolation.rs`** —
the net-isolation gate (`ureq`/`rustls` absent from the six tax crates, present in
update-prices). **`src/dump_fields.rs`** — the AcroForm field dumper (the filler's view).

# SEE ALSO

`ARCHITECTURE.md` (the *why* of the whole system), and the per-binary man pages under
`docs/man/`.
