# SPEC — btctax-tui: ratatui read-only viewer (GUI sub-project 1)

**Source baseline:** `origin/main` @ `30570e0`.
**Goal:** A new `btctax-tui` crate — a **ratatui terminal UI** that unlocks the PGP vault and lets the
user browse their already-ingested data **read-only**: Holdings, Disposals, Income, Tax Report, Forms,
Compliance. First GUI sub-project; strictly a viewer (no mutation, no writes, offline).

**SemVer:** new workspace member + binary (`btctax-tui`), no change to existing crates ⇒ **MINOR**
(pre-1.0). Additive only.

## Hard constraints (load-bearing)
- **Offline, local, single-user.** No network, no telemetry/CDN/analytics. Only local reads.
- **Read-only — enforced at compile AND review level [R0-I1].** NEVER call `Session::save()`,
  `persistence::append_*`, any `btctax_cli::cmd::*` mutating command, or `Session::conn()`.
  - **Compile-level:** hold the `Session` in an IMMUTABLE binding (`let session = Session::open(...)?;`
    — never `let mut`). `save()` takes `&mut self`, so an immutable `Session` makes `save()` a compile
    error — a real guarantee, not just discipline. Keep it immutable everywhere it's stored.
  - **`Session::conn() -> &Connection` is FORBIDDEN in btctax-tui** — rusqlite offers interior
    mutability, so a `&Connection` could execute writes without `&mut`/`save()`. Use only the typed
    read methods (`load_events_and_project`, `all_tax_profiles`, `config`).
  - **Review-level:** the whole-diff greps the crate for `save(`, `append_`, `cmd::`, AND `conn(` — any
    hit is a finding.
- **Never read `~/Documents/BitcoinTax/ReadOnly`.** The TUI opens ONLY the encrypted **vault** the user
  points it at (e.g. `~/Documents/BitcoinTax/vault.pgp` via `Session::open`); it never touches the
  read-only source CSV/XLSX. Synthetic fixtures + temp vaults only in tests.
- **Passphrase hygiene [R0-I2/M7].** Masked input into a `String` buffer **pre-allocated
  `String::with_capacity(128)` with a length CAP (reject/ignore input past the cap) so it NEVER
  reallocates** — a growing String would scatter partial-passphrase fragments across freed heap that
  `mem::take` can't wipe. Hand it to the vault by MOVING it —
  `Passphrase::new(std::mem::take(&mut buffer))` — so the store's zeroizing `Passphrase` owns the only
  copy and wipes it on drop (NO new `zeroize` dep needed; the buffer is left empty). NEVER `clone()` the
  buffer or the passphrase (a clone would leave an un-wiped copy in freed heap). Never log/render it.
  Honor `BTCTAX_PASSPHRASE` env (non-interactive) — that path passes the env `String` straight into
  `Passphrase::new` (no persistent buffer).
- **Terminal safety.** Enter raw mode + alternate screen on start; ALWAYS restore — on normal exit, on the
  run-loop's `Err` return path **[R0-M4]**, AND on panic (install a panic hook that restores the terminal
  before printing). A TUI crash/error must never leave the user's terminal in raw/alt-screen state.
- **Single-instance vault lock.** `Vault::open` holds a `VaultLock`; if the CLI (or another TUI) has the
  vault open, `Session::open` returns `StoreError::Locked` → show a clear "vault is in use by another
  process" screen, don't crash.

## Read-only API (recon @ 30570e0 — all verified pure/read-only)
- `btctax-cli` is a LIBRARY (`[lib] btctax_cli`): `Session` + `CliError` are `pub`. Depend on it.
- `Session::open(&Path, &btctax_store::Passphrase) -> Result<Session, CliError>` (no save);
  `load_events_and_project() -> (Vec<LedgerEvent>, LedgerState, ProjectionConfig)`;
  `all_tax_profiles() -> BTreeMap<i32,TaxProfile>`; `optimize_attested_set() -> BTreeSet<EventId>`;
  `config() -> CliConfig`. `save()` is the ONLY mutator — never call it.
- Per-tab structured data (all pure reads over the projected `LedgerState` / pure builders — no I/O, no
  mutation): Holdings `state.lots` + `state.holdings_by_wallet`; Disposals `state.disposals[].legs`;
  Income `state.income_recognized`; Tax `btctax_core::compute_tax_year(events,&state,year,profile,&tables)
  -> TaxOutcome` + `compute_se_tax(&state,year,status,&table) -> Option<SeTaxResult>` +
  `BundledTaxTables::load()`; Forms `btctax_core::forms::{form_8949,schedule_d,form_8283}(&state,year)`;
  Compliance `disposal_compliance(&events,&state)` + `state.blockers` (partition by
  `kind.severity()` Hard/Advisory) + `btctax_cli::render::build_verify(&state,&events,&cli) -> VerifyReport`.
- Reusable `pub` display helpers: `btctax_cli::render::{wallet_label, filing_status_tag}`. The small
  private tag fns (`term_tag`/`basis_source_tag`/`income_kind_tag`/`dispose_kind_tag`/
  `compliance_status_tag`) are 3–5-line matches — re-implement locally in `btctax-tui` (do NOT widen the
  CLI's API surface just for this).

## Design

### Crate + wiring
Add `crates/btctax-tui` to workspace `members`. `Cargo.toml`: `edition.workspace = true` +
`rust-version.workspace = true` (these `[workspace.package]` keys exist; MSRV 1.74). bin `btctax-tui`.
Deps: `btctax-cli`/`btctax-store`/`btctax-core`/`btctax-adapters` (path deps) + **new** `ratatui = "0.29"`
+ `crossterm = "0.28"`. **[R0-M1] There is NO `[workspace.dependencies]` table — pin EXPLICIT versions
for every external dep** (do NOT use `.workspace = true` for deps); match the versions the other crates
already use (`rust_decimal`, `time`, `thiserror`). No other GUI deps exist; add only ratatui + crossterm.
**[R0-M5] MSRV headroom:** ratatui 0.29 declares MSRV exactly 1.74 (zero headroom; 0.30 jumps to 1.86) —
so **commit `Cargo.lock`** and add a `cargo +1.74 check` CI gate to catch transitive MSRV drift; do NOT
bump ratatui past 0.29 without a deliberate workspace-MSRV decision. CLI arg: the vault path (positional
or `--vault`).

### App architecture (standard ratatui loop)
- `App` state: `screen: Screen{Unlock, Locked, Viewer}`; `Snapshot` (loaded once at unlock: events,
  LedgerState, ProjectionConfig, profiles map, **`CliConfig` [R0-M2] — `build_verify` needs it (not
  ProjectionConfig)**, `BundledTaxTables`). **[R0-M3] do NOT store `optimize_attested_set`** — the
  read-only viewer's tabs (`disposal_compliance`, `build_verify`) don't consume it; omit it. `tab: Tab`;
  `selected_year: i32` (default: the latest year present in disposals/income, else current-ish); per-tab
  table/scroll state (`ratatui::widgets::TableState`).
- Main loop: setup terminal (raw + alt screen + panic-hook restore) → loop { draw(app) ; handle key
  event } → teardown. `crossterm` event read.
- **`draw`** dispatches on `screen`/`tab`; **`handle_key`** maps keys → state transitions. Pure-ish: draw
  reads `App`, handle_key mutates `App` (UI state only — never the ledger).

### Screens
1. **Unlock:** title + masked passphrase field + "offline · read-only" note; on Enter → `Session::open`.
   Errors mapped to clear messages: `WrongPassphrase`, `Locked` (→ the Locked screen), `HalfCreatedVault`,
   `Io(NotFound)` ("no vault at <path>"). `BTCTAX_PASSPHRASE` set → skip the prompt, open directly.
2. **Locked:** "vault is in use by another process (the CLI or another viewer). Close it and retry."
   (r retry / q quit).
3. **Viewer:** top tab bar `Holdings | Disposals | Income | Tax | Forms | Compliance`; the selected tab's
   content pane (bordered); a footer with context keybindings. The header shows the vault path + the
   selected year.

### Tabs (structured → widgets)
- **Holdings:** a `Table` of `state.lots` (wallet via `wallet_label`, acquired_at, BTC = sat/1e8 exact
  Decimal 8dp, usd_basis, basis_source tag, `basis_pending` flag) + a TOTAL row (Σ remaining_sat, Σ
  basis). Year-independent (current holdings). Selectable rows + scroll.
- **Disposals:** a `Table` over `state.disposals[].legs` filtered to `selected_year` (disposed_at.year):
  disposed_at, acquired_at, BTC, proceeds, basis, gain (signed), term tag, wallet. TOTAL row.
- **Income:** a `Table` over `state.income_recognized` filtered to `selected_year`: recognized_at, kind
  tag, business flag, BTC, usd_fmv. TOTAL row.
- **Tax:** render `compute_tax_year(...selected_year...)`: if `Computed`, show ST/LT net, ordinary-rate/
  LTCG/NIIT components + TOTAL (the reconciliation), loss deduction, carryforward, marginal rates; then
  the SE-tax block from `compute_se_tax` (with its standalone note) + the charitable-deduction total;
  if `NotComputable(blocker)`, show the blocker reason + no numbers. Show advisory blockers.
- **Forms:** `form_8949` rows (Part I/II, box, description, dates, proceeds/basis/gain) + `schedule_d`
  ST/LT totals + `form_8283` rows (section, description, deduction, needs_review) — year-scoped tables
  with the standing caveats (aggregation, box-review) shown as footnotes.
- **Compliance:** `build_verify(...)` → conservation status, Hard vs Advisory blockers (partitioned),
  pending-basis, safe-harbor status, pre-2025 method + attestation, per-disposal `disposal_compliance`
  statuses. Read-only snapshot of `verify`.

### Keybindings
`Tab`/`Shift-Tab` switch tabs; `←/→` change `selected_year` (affects Disposals/Income/Tax/Forms);
`↑/↓` (or `j/k`) select row; `PgUp/PgDn` scroll; `g/G` top/bottom; `r` re-project from the open session
(refresh); `q`/`Esc` quit. A `?` help overlay listing keys.

### Decisions
- **Read-only display only — export DEFERRED.** The MVP does NOT write files (no CSV export from the TUI
  yet — that's the CLI's `export`/`write_csv_exports`, which writes 0o600 files); the viewer purely
  displays. Export-from-TUI is a follow-up. This keeps the MVP's never-writes guarantee absolute.
- **Depend on `btctax-cli` (lib) for `Session`** — reuse the encapsulated open+project+side-table reads;
  don't replicate. The mutating surface is simply never called.
- **Snapshot loaded once at unlock; `r` re-projects** from the still-open session (cheap; the vault
  content can't change while we hold the lock, but `r` gives a clean re-read).

## Plan (TDD — ratatui `TestBackend` renders a buffer to assert cells; synthetic fixtures + temp vaults only)

### Task 1 — crate skeleton + terminal lifecycle + App loop + quit
- Add the workspace member + `Cargo.toml` + `main.rs`. Terminal setup/teardown (raw mode, alternate
  screen) with a **panic hook that restores the terminal**; the `App`/`Screen`/`Tab` enums; the
  draw/handle_key skeleton (empty tabs); `q`/`Esc` quits. A `Snapshot` type (read-only fields).
- Tests: the panic-hook restore is installed (unit-testable via the restore fn); `handle_key(q)` sets
  quit; `Tab`/`Shift-Tab` cycles tabs. (Terminal I/O itself isn't unit-tested; the restore fn is.)

### Task 2 — unlock flow + Session::open + error screens
- **Files:** `btctax-tui/src/{unlock.rs,app.rs}`.
- Masked passphrase input (`String` buffer, rendered as `●`), `BTCTAX_PASSPHRASE` fast-path;
  **[R0-I2] `Passphrase::new(std::mem::take(&mut buffer))`** (move — never clone — so the store's Drop
  wipes the only copy); hold the returned `Session` in an **immutable** binding **[R0-I1]**.
  `Session::open` → build `Snapshot` (`load_events_and_project` + `all_tax_profiles` + `config` (CliConfig,
  M2) + `BundledTaxTables::load`; NOT optimize_attested_set, M3). Map `CliError`/`StoreError` → the Unlock
  error line / the Locked screen.
- Tests (temp synthetic vault, like the CLI tests; `BTCTAX_PASSPHRASE` to avoid a prompt): a correct
  passphrase → `Screen::Viewer` with a populated Snapshot; a wrong passphrase → an error message, stays
  on Unlock; a `Locked` vault → the Locked screen. **[R0-M6] Read-only is enforced by construction** (the
  immutable `Session` binding makes `save()` a compile error; no `conn()`/`append_`/`cmd::` used) — there
  is no runtime "was save called" seam; instead a behavioral test asserts the **vault file bytes on disk
  are byte-identical** before vs after an open→build-Snapshot→drop cycle.

### Task 3 — Holdings / Disposals / Income tabs
- **Files:** `btctax-tui/src/tabs/{holdings.rs,disposals.rs,income.rs}` + local tag helpers.
- Render each as a `Table` from the Snapshot (with the year filter for Disposals/Income), selectable +
  scrollable, with a TOTAL row; exact-Decimal BTC + USD formatting (no float).
- Tests (`TestBackend`, fixture `LedgerState`): each tab renders the expected header + a known row's
  cells + the TOTAL; the year filter excludes other-year rows; empty state renders a placeholder.

### Task 4 — Tax / Forms / Compliance tabs
- **Files:** `btctax-tui/src/tabs/{tax.rs,forms.rs,compliance.rs}`.
- Tax: `compute_tax_year` + `compute_se_tax` → the report layout (Computed vs NotComputable; SE standalone
  note; advisories). Forms: `form_8949`/`schedule_d`/`form_8283` → tables + caveat footnotes.
  Compliance: `build_verify` + `disposal_compliance` + blocker partition + safe-harbor/pre-2025 status.
  `←/→` re-derives the year-scoped tabs.
- Tests (`TestBackend`, fixture state + a synthetic `TaxProfile`): Tax tab shows the ST/LT/NIIT/SE lines
  for a Computed year and the blocker reason (no numbers) for a NotComputable year; Forms tab shows a
  known 8949 row + the Schedule D totals; Compliance shows Hard vs Advisory partition + the pre-2025/
  safe-harbor status. Year change updates the figures.

### Task 5 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: **read-only guarantee** — grep the crate for `save(` / `append_` / `cmd::` / **`conn(`**
  (I1); confirm the `Session` is held in an IMMUTABLE binding (so `save()` is compile-impossible) and
  `Session::conn()` is never used; never reads `ReadOnly`; passphrase MOVED via `mem::take` (never cloned)
  + never rendered (I2); terminal restored on normal exit + `Err` path + panic (M4); `Locked` handled;
  offline (no network dep/call); `Cargo.lock` committed + MSRV-1.74 (M5); exact Decimal / no float in all
  formatting; determinism; synthetic-only tests. Confirm the figures match the CLI (`report --tax-year`,
  the form builders) for the same fixture (the TUI reuses the same core builders → parity by construction;
  spot-check one).
- FOLLOWUPS: export-from-TUI (CSV/snapshot); the MUTATING flows (import, reconcile/classify, config,
  tax-profile set, optimize run/accept/consult, safe-harbor attest) — a future interactive-TUI or the
  egui GUI; charts/visualizations; mouse support; a read-only concurrent-open mode (vs the exclusive
  VaultLock) if the store later supports it.

## Out of scope
- Any mutation (import/reconcile/config/optimize-accept/attest) — read-only viewer only; export-to-file;
  charts/graphs; mouse interaction; multi-vault; the egui/graphical GUI; concurrent read-only vault
  sharing (VaultLock is exclusive); 2026/2027 tables.
