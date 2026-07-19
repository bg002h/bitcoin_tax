---
title: BTCTAX-ARCHITECTURE
section: 7
header: Software Architecture
footer: btctax 0.7.0
---

# NAME

btctax software architecture — how the Bitcoin-tax application set is put together.

# OVERVIEW

**btctax** is an offline, deterministic, privacy-first engine that turns a person's
Bitcoin exchange history into a correct U.S. federal tax result and the filled IRS
paper that reports it. It is a **mechanical calculator, not authorised for filing**:
the repository `NOTICE` disclaims any authorisation to file while leaving the
MIT-OR-Unlicense grant over the *software* unrestricted — the disclaimer speaks to
the *output*, never the code.

Four properties shape every design decision in the codebase:

- **Deterministic and total.** The core projection is a pure function of
  `(events, prices, config)`: identical inputs yield byte-identical output, it never
  panics, and it performs no I/O. Money is exact `rust_decimal::Decimal` and integer
  satoshis end-to-end — there is no floating-point money anywhere.
- **Event-sourced.** Nothing is ever mutated. A user's activity is an append-only log
  of immutable events; corrections are *new* events (a re-import becomes an
  `ImportConflict`, a reconciliation mistake becomes a `VoidDecisionEvent`). The
  ledger is a *projection* of that log, recomputable from scratch at any time.
- **Fail-closed.** When the engine cannot compute a defensible number it **refuses**
  rather than present an authoritative-looking wrong one. Every open question is a
  typed *blocker*; every unmodelled full-return input is a typed *refusal*; a form
  never leaves the paper layer unless its geometry verifies on read-back.
- **Offline and private.** The entire tax pipeline links no network client (enforced
  in CI). The vault is a single encrypted file. Tests and goldens use synthetic data
  only; a CI job scans for real SSN/EIN-shaped tokens.

This document is a standing reference for how the twelve crates of the workspace
realise those properties. It is organised bottom-up: the workspace map and data
flow, then the foundation, domain, application, and tooling layers, then the
cross-cutting invariants, the build/test discipline, and finally the notable design
trade-offs and known risks.

# THE WORKSPACE AT A GLANCE

The workspace has twelve member crates, all versioned in lockstep at 0.7.0.

| Crate | Layer | Role |
|-------|-------|------|
| `btctax` | — | Name reservation on crates.io. Exposes no API; points installers at `btctax-cli`. Zero dependencies. |
| `btctax-core` | foundation | The domain: event model, the pure/total projection (`resolve` + `fold`), lot selection, the two tax engines, forms row-projection, optimize/what-if, and the event-persistence schema. |
| `btctax-store` | foundation | The encrypted local vault: OpenPGP-sealed, in-memory SQLite, atomic writes, single-instance lock, memory hygiene. Domain-blind. |
| `btctax-adapters` | domain | Exchange-export parsers (Coinbase / Gemini / River / Swan) to normalized events; the bundled daily-close price dataset; the bundled IRS tax tables. |
| `btctax-forms` | domain | Fills the *official* IRS fillable PDFs, offline and byte-deterministically, verifying every value's geometry on read-back. Never recomputes tax. |
| `btctax-input-form` | domain | A UI-agnostic form engine for authoring the full-return non-crypto inputs (`ReturnInputs`). "TUI now, web app later." |
| `btctax-cli` | app | The composition root: the `btctax` binary plus the application library that wires vault + ingest + core + forms into the command surface. |
| `btctax-tui` | app | A read-only ratatui vault viewer (six tabs + export). Also a library — its screens and unlock flow are reused by the editor. |
| `btctax-tui-edit` | app | The interactive reconciliation / authoring editor. Append-only writes behind payload-showing confirmations. |
| `btctax-update-prices` | tooling | The opt-in online price updater — the **only** network-linked crate in the workspace. |
| `btctax-oracle-harness` | tooling | A `publish = false` test binary: fills a scenario and reads it back off the paper, classifying divergences against two external tax oracles. |
| `xtask` | tooling | `publish = false` dev tooling: man-page/PDF generation, the worked-examples generator, subcommand-coverage, and the net-isolation check. |

## The dependency graph

Every dependency edge points strictly downward — the workspace is a **clean DAG**
(no cycles). External crate versions are pinned per-crate; there is deliberately no
`[workspace.dependencies]` table (alignment is kept by comment convention).

```
  FOUNDATION (no btctax-* deps; mutually independent):
     btctax-core     pure domain + the two tax engines
     btctax-store    encrypted persistence

  DOMAIN (-> core only):
     adapters -> core    forms -> core    input-form -> core

  APP (a reuse chain):
     btctax-cli       -> core, store, adapters, forms    (composition root)
     btctax-tui       -> cli, store, core, adapters
     btctax-tui-edit  -> tui, cli, input-form, store, core, adapters

  TOOLING:
     xtask            -> cli, update-prices, forms
     oracle-harness   -> core, forms
     update-prices    -> adapters    (the only net-linked crate)
```

The two foundation crates never depend on each other. `btctax-core`'s persistence
module operates over a borrowed `rusqlite::Connection`; `btctax-store` produces that
decrypted, in-memory connection. They meet only in `btctax-cli`. This keeps the
domain storage-format-agnostic and the store domain-agnostic — either could be
replaced without touching the other.

The application layer is a **reuse chain**: the read-only viewer links the CLI
*library* for its session and rendering logic; the editor links the viewer for its
screens and unlock flow. Both TUIs therefore link "the CLI" as a library.

# END-TO-END DATA FLOW

A user's data travels one path, from an exchange CSV to a filled IRS PDF. The crate
responsible for each hop:

```
  exchange CSV / XLSX
    |
    v  [btctax-adapters]  detect -> parse -> normalize; stamp ingest FMV
  LedgerEvents  (Acquire / Income / Dispose / Transfer{In,Out} / Unclass.)
    |
    v  [btctax-cli import] -> [btctax-store]
  encrypted vault          decrypt -> in-memory SQLite -> append-only rows
    |
    v  [btctax-core]  project = resolve + fold  (pure, total, no I/O)
  LedgerState  (lots, disposals, income, removals, blockers, conservation)
    |
    +--> [btctax-cli report]    terminal report + CSV export
    +--> [btctax-tui / -edit]   six-tab viewer / mutating editor
    +--> [btctax-core tax/]     ReturnInputs + classifier
             |                    -> delta engine (crypto-attributable)
             |                    -> absolute engine (full Form 1040)
             v                    -> printed line chains
         [btctax-forms]   fill IRS PDFs; verify geometry on read-back
             |
             v  [btctax-cli]    btctax export-irs-pdf

  sidecar: [btctax-update-prices] -> price-cache CSV -> read by adapters
```

Reconciliation decisions (classify an inbound, void a decision, elect a lot method,
select specific lots) are themselves appended events; re-projecting the log applies
them. The tax engines and every UI surface all go through the *single* `project()`
contract, so no surface can show a number a different surface would not.

# FOUNDATION LAYER

## btctax-core — domain model and tax engine

`btctax-core` is the heart of the system: an event-sourced projection plus two
federal tax engines. Its charter is strict — the projection is *total* (pure,
deterministic, never panics), and the crate's **only** I/O is `persistence.rs`,
which reads and appends canonical event rows over a caller-supplied connection.

### The event model

A `LedgerEvent` is `{ id, utc_timestamp, original_tz, wallet, payload }` — immutable
and append-only. The `EventPayload` sum type has three families:

- **Imported** (adapter-emitted): `Acquire`, `Income`, `Dispose`, `TransferOut`,
  `TransferIn`, `Unclassified`.
- **System**: `ImportConflict` (the same source row re-imported with changed content).
- **Decisions** (the thirteen user reconciliation verbs): `TransferLink`,
  `ReclassifyOutflow`, `ClassifyInbound`, `ManualFmv`, `SafeHarborAllocation`,
  `SupersedeImport`, `RejectImport`, `VoidDecisionEvent`, `ClassifyRaw`,
  `MethodElection`, `LotSelection`, `ReclassifyIncome`, `SelfTransferPassthrough`.

`EventId` is *structured and injective* — `Import{source, source_ref}`,
`Conflict{…, fingerprint}`, `Decision{seq}` — never a hash. A SHA-256 `Fingerprint`
is used **only** to detect import conflicts, never for identity. Decision variants
are forward-only: a vault written by a newer binary containing a new decision variant
fails loudly (rather than silently mis-reads) on an older binary.

Money is `Usd = Decimal` and `Sat = i64`, with **two deliberately distinct rounding
regimes**: half-even to cents inside the engine, and IRS half-up to whole dollars on
filed forms. Conflating them mis-prints real IRS Tax-Table cells, so the two live in
separate functions.

### The projection: resolve then fold

`project(events, prices, config) -> LedgerState` is the single contract every surface
uses. It runs in two passes:

**Pass 1 — `resolve`** adjudicates decisions against the imported events, in stages:
collect voids and classify revocability; resolve import conflicts (latest decision
sequence wins); apply `ClassifyRaw` / `ManualFmv`; apply the classification decisions
(each validated against the *effective* target payload; duplicates become a
first-wins `DecisionConflict`); synthesize pseudo-mode defaults if enabled; build the
*effective timeline* mapping every payload to an `Op`; collect method elections
(back-dated ones become a hard blocker) and lot selections; and decide the 2025
transition mode. A cross-type guard ensures a self-transfer "skip" can never erase a
taxable event.

**Pass 2 — `fold`** replays the effective timeline in canonical order into a
`PoolSet`: acquires create lots; disposals consume the pool building pro-rata
`DisposalLeg`s (with the four-zone §1015 dual-basis logic for gifted lots);
gifts/donations build zero-gain `Removal`s carrying the §170(e) claimed deduction;
self-transfers relocate lot fragments carrying basis, holding period, and any pseudo
taint; on-chain fee satoshis are consumed with the configured fee treatment.

The output `LedgerState` carries lots, holdings-by-wallet, disposals, removals,
recognized income, `pending_reconciliation`, typed `blockers`, and the FR9
conservation accumulators. Crucially, one dispatcher — `fold_event` — is shared by
the real fold, the transition's conservation pre-fold, and the truncated re-folds the
optimizer and what-if use, so those read models cannot diverge from reality.

### Blockers: how open questions are represented

Every unresolved question is a typed `Blocker` with a `severity()` of **Hard** or
**Advisory**. Hard blockers (missing FMV, an uncovered disposal, an import conflict, a
decision conflict, an unknown-basis inbound, an unclassified event, a back-dated
method election, an invalid lot selection, a missing tax table/profile) **gate the
tax computation** — projection-wide, not merely for the year in question. Advisories
(an unmatched outflow, a qualified-appraisal note, a zero-basis self-transfer default,
pseudo-mode-active) never gate; they inform.

### Lot selection and the 2025 transition

Before 2025, all lots live in one universal pool; on and after 1 Jan 2025 they live in
per-wallet pools (the Rev. Proc. 2024-28 wallet-by-wallet regime). The consumption
order is a total order over lots — `Fifo`, `Lifo`, or `Hifo` — computed without
division or floats. Method resolution uses one shared two-tier resolver: the latest
in-force election *scoped to the wallet*, else the latest *global* election, else the
HIFO default. **Specific identification** (`LotSelection`) lets a filer name lots per
disposal; the picks must sum to the disposal principal and cannot cross wallets
(§1.1012-1(j)) — any violation degrades to method order so conservation always holds.

The 2025 transition is either Path A (default: relocate the pre-2025 residue per
holding wallet) or Path B (seed lots from a timely, conservation-checked safe-harbor
allocation). An effective allocation is irrevocable, and its totals must equal the
pre-2025 residue computed under the allocation's own recorded method — verified by
re-running the very same `fold_event`.

### The two tax engines

The crate holds two engines, deliberately separated:

- **The crypto-delta engine** (frozen behind SHA-256 content pins) computes the
  *incremental* federal tax attributable to crypto: `tax(with crypto) − tax(without)`.
  It implements §1222 short/long netting, the §1211(b) capital-loss limit, §1212(b)
  character-preserving carryforward, exact-bracket §1 ordinary tax, §1(h) preferential
  stacking, and §1411 NIIT. It has a pinned identity: `total = ordinary_delta +
  ltcg_tax + niit`.
- **The absolute full-return engine** (additive-only; never edits the frozen engine)
  assembles a complete Form 1040 from `ReturnInputs` plus the ledger: Schedules 1/2/3
  /A/B/C/D/SE, Forms 8959/8960/8995, standard-vs-itemized, charitable ceilings with
  five-year carryover, the AMT screen, and Line 16 via the IRS Tax Table / worksheet.
  It derives the delta engine's tax profile from *non-crypto lines only*, so it is
  structurally incapable of double-counting crypto.

Planning layers — the lot-selection optimizer and the non-persisted what-if — route
through the same engine; there is no independent tax authority anywhere.

## btctax-store — persistence and security

`btctax-store` is the encrypted, atomic, single-user vault. It is deliberately
domain-blind: it stores one opaque SQLite image plus a keypair and hands back a live
in-memory connection. All schema lives above it (the canonical `events` table in
core; typed side-tables in the CLI).

### The vault model

The vault is **OpenPGP**, via `sequoia-openpgp` with the pure-Rust crypto backend
(chosen for cross-platform parity). On disk: `vault.pgp` (the encrypted SQLite image)
and a sidecar `vault.key` (the passphrase-encrypted private key), plus
`.bak`/`.tmp`/`.lock` families. Opening decrypts the image, decodes a four-byte schema
version, migrates, and deserializes into an in-memory SQLite database; a wrong
passphrase is distinguished from a corrupt ciphertext by whether the secret key
actually decrypted.

### On-disk format and atomicity

The working database is entirely in memory, round-tripped with SQLite's
serialize/deserialize. Every `save()` serializes, encrypts, and **atomically
replaces** the whole file: write a `.tmp` (owner-only) and fsync it, copy the live
target to `.bak` and fsync, rename `.tmp` over the target, fsync the parent directory.
Each save thus preserves the prior generation as a `.bak`, and a bounded, *classified*
recovery restores from it only on genuine corruption — never on a wrong passphrase or
a newer-than-supported schema (which would be a silent downgrade).

### Security posture

At rest, the image is encrypted to a storage subkey whose private half is itself
S2K-encrypted under the passphrase; `.tmp`/`.bak` are ciphertext, never plaintext.
File permissions are 0o600 files / 0o700 directories on Unix, set at `.tmp`-creation
so the renamed target inherits them. In memory, the passphrase is zeroized on drop,
key material is page-locked (`mlock`/`VirtualLock`) where possible, and a single-
instance exclusive lock (via `fs2`) prevents concurrent writers. The honest documented
bound: the live SQLite connection holds plaintext in its own heap for the session's
duration.

### Drafts and the finalize guard

A `return_inputs_draft` side-table holds work-in-progress full-return authoring that
is **invisible to the resolver** — so a filer can work on a year all year long without
poisoning the engine-visible state. On load, precedence is Draft ⇒ Committed ⇒ Fresh.
The finalize guard (I-11) refuses to write a *committed* full-return row for a year
whose tax tables do not exist yet — per-year, not per-call — so a table-less year can
never be poisoned at resolve time. A stale work-in-progress draft is discarded (with a
returned note); a stale *parked* draft — which may be the only copy of carryover data
— refuses rather than discard.

# DOMAIN SERVICES LAYER

## btctax-adapters — ingestion

`btctax-adapters` is the boundary that turns exchange exports into normalized
`LedgerEvent`s, and it bundles the two reference datasets the rest of the system reads
(prices and tax tables). It produces events only; persistence is the CLI's job.

Each source implements an `Adapter` trait — `detect -> group -> parse -> normalize`.
Recognition is content-based header-token sniffing (except Gemini, which is
extension-only and runs last so it claims only files the others declined). Swan merges
three role-specific files (trades / transfers / withdrawals) into one ingest batch,
routed by header signature.

The governing doctrine is **conservative — never guess**. Any ambiguous row type
(Coinbase internal Pro moves, a Gemini non-USD pair, a Swan fee event, an unknown
vocabulary word) becomes an `Unclassified` event that is *kept and counted*, never a
guessed acquire or disposal. An inbound on-chain transfer becomes a basis-less
`TransferIn`; basis and acquisition date are re-supplied later by reconciliation.

FMV at ingest follows a ladder: the export's own USD, else the bundled daily-close
dataset, else a hard "FMV missing" blocker. The price layer is two-level —
`BundledPrices` (a compiled-in daily-close CSV, exact-date lookup only, no gap-fill)
with an optional local cache CSV layered over it (`LayeredPrices`, cache wins). The
crate carries no network and no path logic; the online refresh lives only in
`btctax-update-prices`. Tax tables are compiled-in, transcribed verbatim from each
Revenue Procedure and pinned by tests (crypto slice: 2017/2024/2025/2026; full return:
TY2024 only, fail-closed).

Event references are deterministic: a native id where the export has a stable one
(direction-scoped), else a semantic ref built from the UTC millisecond timestamp, the
type, the satoshi amount, and a file-order occurrence index. The whole projection is
pure — identical files and prices yield identical events, with no clock, network, or
randomness anywhere in the crate.

## btctax-forms — the paper layer

`btctax-forms` fills the **official IRS fillable PDFs** from already-computed tax data,
offline and byte-deterministically, and never recomputes anything — every cell is
transcribed from core's printed line chains. Its defining idea, stated in the crate
header: *the map is what we distrust; the PDF's geometry is the oracle.*

The mechanism (via `lopdf`): the bundled IRS PDFs are static XFA hybrids, so a fill
removes the `/XFA` layer and sets `/NeedAppearances`, walks the AcroForm field tree
collecting each leaf's fully-qualified name and `/Rect`, applies values (writes that
*fail closed* on a missing field), and pins byte-determinism by dropping `/Info` dates
and the trailer `/ID`. Then it **verifies on read-back**, on the serialized bytes,
with two map-independent oracles: a *grid* oracle that re-derives column/row bands from
the blank PDF's own widget rectangles and asserts every written value lands in the band
its logical cell demands, and a *flat* oracle for non-grid forms (page membership,
column clusters, descending-y ordering, and `/MaxLen` enforcement in characters so an
11-character SSN can never be silently truncated by a viewer). A complementary inverse
transcriber confirms the right *value* is in the box, not just that *a* value landed
there. On any doubt — a geometry mismatch, an unmapped write, an overflow, a negative
in a parenthesized box — **zero bytes** are returned.

Field maps are *data*: adding a tax year is a `forms/<year>/` directory (PDF plus
per-form TOML maps), never a code change. The full-return **packet** fills every form
all-or-nothing and orders them by IRS Attachment Sequence Number, so the filer gets
their stapling order for free. Form 6251 (AMT) is deliberately *not* filled — the AMT
is a refuse-screen in core, so Schedule 2 line 2 is $0 by construction. The DRAFT
watermark and attestation gate are keyed to *pseudo-reconciliation* (fictional
figures), not to the full return, which exports clean per user policy; pseudo figures
are watermarked regardless and that gate dominates.

## btctax-input-form — the authoring engine

`btctax-input-form` is a UI-agnostic form engine for authoring the non-crypto 1040
inputs (`ReturnInputs`) — "rendered by the TUI now and a web app later; depends on
core only, no vault, no terminal." It is a pure model/controller: a `FormSpec` tree of
sections and fields, a serde `Edit` seam (stable `SectionId`/`FieldId` enums — "the
web wire"), and three validation tiers (`parse` for syntax, `apply` for structure,
`attribute` for mapping a refusal to the exact field or section that caused it).

Its most important property is **anti-laundering**: the working value is
`Option<ReturnInputs>`, and the *only* accepted first edit is choosing a filing status
— so "a return exists" is a type-level fact and a commit can never see a laundered
`default()`. Secrets are inbound-only with masked debug output. The form's structure
is generated from the single form-question registry in core (below), so no liveness
predicate or accessor is ever written twice.

# APPLICATION LAYER

## btctax-cli — the composition root

`btctax-cli` is both the `btctax` binary and the application library the two TUIs
reuse. It is the **only** place the vault, adapters, core, and forms are wired
together. Its own charter: the library is I/O-explicit and deterministic; the binary
(`main.rs`) is a thin clap dispatch with no business logic.

The command surface (clap-4 derive) covers `init`, `import`, `verify`, `report`,
`config`, `events list`, `tax-profile`, `income`, `optimize`, `what-if`,
`export-snapshot`, `export-irs-pdf`, `backup-key`, `limitations`, and the large
`reconcile` family (single-item classify/void/select-lots/match-self-transfers/… plus
the bulk verbs) and `pseudo` mode. Documentation is single-sourced: the clap
doc-comments render into *both* `--help` and the per-subcommand man pages.

Every command follows the same shape: parse (exact `Decimal`, dates, event refs —
never floats) → open a **`Session`** (the single seam wrapping one vault plus a price
provider) → call one library function → render. Bulk verbs are two-phase: the library
computes a read-only *plan*, `main.rs` renders a preview and confirms, then `apply`
derives targets *from plan rows, never raw refs*, so exclusions cannot be bypassed.

The error model is a typed `CliError` (transparent core/store/adapter errors plus
path-enriched I/O, form-fill read-back failure, stale-schema refusals, and attestation
errors). Exit codes are meaningful: **0** success (including a pseudo-active report —
the banner is the signal), **1** ran-but-not-filing-ready (`verify` with Hard
blockers; `report --tax-year` when not computable), **2** any error or a worker panic.

Reconciliation records exactly one decision per verb and appends it (monotonic
sequence), append-only and re-projectable. A record-time conflict guard runs the *real*
projection before appending, so a refusal at record time equals the resolver's
adjudication by construction. Non-decision data (tax profiles, donation details,
optimizer attestations, return inputs, drafts, config, the pseudo flag) lives in typed
SQLite side-tables inside the vault connection.

## btctax-tui and btctax-tui-edit — the terminal UIs

Two ratatui applications share one codebase through the viewer's library surface.

**The viewer (`btctax-tui`)** is strictly read-only: it opens the vault, builds an
immutable `Snapshot`, and **drops the session immediately** — so a write is
structurally unreachable (enforced by a byte-identical-vault test and a no-write-token
source gate). It renders six tabs (Holdings, Disposals, Income, Tax, Forms,
Compliance), plus an export modal and a what-if overlay.

**The editor (`btctax-tui-edit`)** is the mutating reconcile/authoring UI. It **holds
the live session** (and thus the exclusive lock) for its lifetime, so there is no
concurrent-writer case. It hosts Browse (the six viewer tabs), a full tax-inputs
authoring flow driven by `btctax-input-form`, and roughly two dozen classify/reconcile
flows. Every mutation is List → form/picker steps → a **payload-showing confirmation
modal** → exactly one call into the single persist choke-point → re-project. Failed
saves roll back from an in-memory snapshot; an unrecoverable failure sets a quit-first
latch that freezes all mutating openers.

Both render from one `Snapshot` built with the *same* screened resolver the CLI's
`report` uses — so the viewer can never show a different liability, and a refused year
renders "NOT COMPUTABLE (reason)", never a $0 placeholder. Tab renderers and
class-description strings are the CLI's own shared helpers, so wording cannot drift
between CLI and TUI.

### The golden capture system

Both crates share `capture::to_golden(&Buffer) -> String`, which serialises a rendered
frame into two views: a **glyph grid** (one line per row) and a **style overlay**
(per-row runs of `start..end fg=… bg=… mod=…`). A screen is captured *headlessly, from
a plain state value*:

1. build an `App`/`EditorApp` with a synthetic or seeded-vault snapshot and a **pinned
   clock**;
2. optionally drive it into a target state by feeding synthetic key events through the
   *real* `handle_key`;
3. render once into a `TestBackend::new(120, 40)` and serialise;
4. a `*_goldens_match_committed` test asserts the fresh capture equals the committed
   `docs/examples-tui/*.txt` file byte-for-byte.

A frame is "a pure function of (code, synthetic state)" — the pinned clock, a fixed
displayed vault path, and the fixed 120×40 geometry are the whole determinism recipe.
This machinery is exactly what a future *screen-based walkthrough* would reuse: a
walkthrough is mostly new capture *drivers* (sequences of key events), not new
machinery. There are four committed goldens today; `make examples-tui` renders them
into a colorized PDF.

# TOOLING

- **`xtask`** generates the man pages and PDFs (single-sourced from the clap
  doc-comments), builds the worked-examples golden by running the *real* binary against
  synthetic vaults in a hermetic temp directory, reports SOFT subcommand coverage, and
  runs the net-isolation check.
- **`btctax-oracle-harness`** is a `publish = false` binary with a JSON contract:
  given a scenario it assembles, fills, and reads the return back off the paper; in
  `--check` mode it compares btctax's per-line figures against two independent external
  oracles (OpenTaxSolver and PSL Tax-Calculator) using the same Rust diff helpers the
  golden tests use — so the Python sweep drivers never re-implement btctax's arithmetic
  (Python's banker's rounding drifts on `.50`).
- **`btctax-update-prices`** is the one HTTP client in the workspace (Binance primary,
  CoinGecko fallback, an eight-day settling lag, forward-only append). Its cache is a
  documented local *input*, like the vault; the bundled-only projection remains the
  published-reproducible baseline.

# CROSS-CUTTING INVARIANTS

- **The clock seam.** Core takes "now" as a *parameter*; the edges resolve it. The CLI
  reads `BTCTAX_NOW` (RFC3339; malformed exits 2; when active an unconditional stderr
  banner discloses simulated timestamps and warns that backdating cannot make a lot
  identification contemporaneous). The TUIs inject a `Clock { Wall, Pinned }` mirroring
  the same contract. That every production wall-clock read routes through the seam is
  held *structurally* by a source-scanning test, not by per-site discipline.
- **The answered-ness invariant.** Historically btctax's one architectural defect —
  "anything that can silently answer for the filer" held only by convention — is now
  structural. A single form-question registry owns the *only* copy of each liveness
  predicate, and a classifier destructures every field reachable from `ReturnInputs`
  with no `..` under `deny(unused_variables)`, so a new boolean or defaulted field is a
  *compile error* until a human classifies it. The honest residual limit, documented
  in-file: the compiler forces "a human must edit," not "classified correctly."
- **Determinism.** Pure total projection, exact decimal money (no floats),
  byte-deterministic PDF fills, a hermetic examples generator, and `BTCTAX_NOW`-pinned
  goldens. The only sources of nondeterminism — the wall clock and the network — are
  fenced behind the two seams above.
- **Fail-closed posture.** Hard blockers gate the year; unmodelled full-return inputs
  refuse; the AMT worksheet is a refuse-trigger; a form never prints unless its geometry
  verifies. The engine would rather emit nothing than emit a wrong number that looks
  authoritative.

# SECURITY AND TRUST BOUNDARY

- **At rest**: a PGP-encrypted SQLite image plus an S2K-encrypted key file, owner-only,
  atomically replaced with a `.bak` safety net, passphrase zeroized, key pages locked,
  single-instance locked.
- **Network boundary**: the entire tax pipeline (all six tax crates) links no HTTP
  client — machine-verified in CI by a `cargo tree` gate that must find `ureq`/`rustls`
  *absent* from the tax crates and *present* in `btctax-update-prices` (a positive
  control so the gate can never go vacuous). Only the opt-in updater touches the
  network, writing only a plaintext local cache the tax binaries read as a file. No
  telemetry anywhere.
- **PII**: privacy-by-fixture — tests and goldens use synthetic data only; a CI job and
  a pre-push hook scan for SSN/EIN-shaped tokens.

# BUILD, TEST, AND CI

The local gate is **`make check`**: `cargo nextest run` run *concurrently* with clippy
in a separate target directory (their differing rustc flags would otherwise thrash one
shared cache), exit statuses individually waited and OR-ed. It is ~6 seconds warm
versus ~400 for `cargo test --workspace` — the suite is runtime-bound (integration
tests spawn the real binary), so the win comes from a raised dev opt-level plus `lld`
and nextest parallelism.

CI runs the `test` suite on a three-OS matrix (Linux/macOS/Windows) so the store's
OS-specific primitives (locks, `mlock`/`VirtualLock`, atomic rename, owner-only perms)
are actually exercised. CI-only jobs that `make check` does **not** cover: `clippy`
(as a gate), `fmt`, `msrv` (pinned to Rust 1.88), the net-isolation check, a
`pii-scan`, and an advisory `examples` job that regenerates the worked-examples golden
and proves the PDFs render. Every compiling Linux job installs `lld` because the
committed cargo config forces it. The process discipline (spec/plan-gated development,
independent review to zero Critical/Important, reviews persisted verbatim, per-phase
follow-up burndown) is documented in `STANDARD_WORKFLOW.md`.

# DESIGN DECISIONS AND TRADE-OFFS

- **Event sourcing with structured identity.** Corrections are events, never
  mutations; identity is structured and injective, with hashing reserved for conflict
  detection. Classifications are first-wins (a stable answer), while `ManualFmv` is
  last-wins (a deliberate correction-flow exception).
- **One dispatcher, many read models.** `fold_event` is shared by the real fold, the
  transition's conservation check, and the optimizer/what-if re-folds — eliminating a
  whole class of divergence bugs. Each read-model helper's doc records the historical
  bug it fixed.
- **Two rounding regimes, two engines.** A cent-exact half-even delta engine (byte-
  frozen behind SHA pins) versus a whole-dollar half-up printed-forms path; the absolute
  full-return engine reuses the frozen engine's primitives *additively*, never editing
  them.
- **User-mandated policies, fenced in code.** Several conservative tax defaults are
  user decisions that reviews must not silently flip and are comment-fenced accordingly:
  the TP8 fee treatment (basis carries, non-taxable), the zero-basis / long-term-default
  inbound self-transfer completion policy, the pre-2025 HIFO default, and the
  clean-export full-return DRAFT-gate policy.
- **Forward-only vault compatibility.** A vault written with a newer decision variant
  fails loudly on an older binary rather than silently mis-reading it — an accepted
  trade for a single sealed artifact.

# KNOWN RISKS AND TENSIONS

For an architect picking this up, the honest tensions worth knowing:

1. **Answered-ness residue.** The classifier closes the defect at a compile-time seam,
   but its own header documents that grep-able evasions (`_`-prefixed bindings,
   `let _ =`) remain review-dependent — the invariant is structural *with an asterisk*,
   and the classifier file is now a load-bearing single point a careless edit could
   weaken.
2. **`make check` is not CI-green.** The fast local gate omits fmt/msrv/pii-scan/
   net-isolation/examples; this has already produced one false "green" report. Compounded
   by `main` not being branch-protected and the `examples` job being advisory.
3. **No `[workspace.dependencies]` table.** Version alignment across twelve crates is
   maintained by comment convention; drift is possible and caught only by review.
4. **The 120×40 terminal assumption.** All TUI draw tests and goldens pin an 80×24-plus
   fixed 120×40 backend; behavior at narrower real terminals is comparatively untested.
5. **App-layer coupling by reuse chain.** Because the session/render library lives in
   the crate named `-cli`, both TUIs link "the CLI" as a library; a CLI-library change
   ripples through all three apps, and the name/role mismatch is a readability cost.
6. **`btctax-tui-edit/src/main.rs` is a ~26k-line monolith** with two dozen
   hand-ordered modal/flow dispatch gates — the layering discipline is held by
   convention and tests, not types.
7. **Vault durability asymmetry.** The encrypted image is `.bak`-recoverable but the
   key file is not; losing or corrupting `vault.key` loses the vault. The mitigation is
   `init --key-backup` (and the `backup-key` command), but key-file loss is the real
   durability risk of the single-file design.

# SEE ALSO

`README.md`, `NOTICE`, `LIMITATIONS.md`, `STANDARD_WORKFLOW.md`, and the per-binary man
pages under `docs/man/`.
