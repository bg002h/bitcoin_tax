# SPEC — btctax-tui-edit chunk 1: the mutating TUI editor, tax-profile set/edit

**Source baseline:** `main` @ `22cda75` (working tree re-verified file-by-file at write time; all
line citations below checked against the current source — drift from the recon is flagged inline
as **[DRIFT-n]**).
**Review status:** R0 round 1 (`reviews/R0-spec-tui-edit-chunk1-round-1.md`) FOLDED — findings
tagged `[R0-…]` inline; re-review required per §2 of `STANDARD_WORKFLOW.md`.
**Goal:** Chunk 1 of the mutating-TUI program: a **separate sibling crate `btctax-tui-edit`** — a
ratatui editor that unlocks the vault, HOLDS the live `Session` for the whole TUI session, lets the
user browse the viewer's six tabs read-only, and performs exactly **one** mutating flow: **set/edit
the per-year `TaxProfile`** via a 10-field form, an explicit payload-showing confirmation modal, a
typed side-table upsert, and an immediate atomic save + snapshot re-projection. The read-only
viewer crate `btctax-tui` is refactored into lib+bin (a pure visibility refactor) so the editor
reuses its unlock flow, `Snapshot`, and tab renderers — the viewer binary's behavior and its whole
test suite are **unchanged**, and its write-free guarantee (E10 gate) is untouched.

**SemVer:** new workspace member + binary (`btctax-tui-edit`); `btctax-tui` gains a `[lib]` target
(additive); `btctax-core` gains one `pub` read-only fn (`persistence::load_all_ordered`) plus its
row type ⇒ **MINOR** (pre-1.0). No behavior change to any existing binary.

## Hard constraints (load-bearing)

**The editor's guarantee statement** (the crate-level contract, stated in every module
doc-comment of `btctax-tui-edit` and enforced by D3's gate):

> "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`, each behind
> an explicit payload-showing confirmation; the vault file only via `Vault::save`'s atomic path"

Chunk-1 instantiation: the ONLY writer is `tax_profile::set` (a side-table **upsert**,
tax_profile.rs:50–62 — `INSERT … ON CONFLICT(year) DO UPDATE`) followed by `session.save()`.
No event is appended by this chunk (D5 states the exact consequence for the prefix test).
Append-only events arrive with chunk 2's decision flows; the guarantee wording covers both now
so it never needs rewording.

- **Crate boundary = guarantee boundary.** The editor is a SEPARATE crate. The viewer crate
  `btctax-tui` stays provably write-free: its E10 mechanized gate (export.rs:690–919) and its
  guarantee wording are **unchanged by this spec** — Task 1 is a pure visibility refactor of the
  viewer (no new tokens, no behavior change; the E10 scanner walks `crates/btctax-tui/src/` via
  `CARGO_MANIFEST_DIR` and continues to cover every file it covers today). Any finding that
  requires changing the viewer's guarantee is out of scope by construction.
- **The live-session architecture.** The editor holds `session: Option<Session>` — the live
  `Session` opened once at unlock and held (mutably reachable) for the whole TUI session. The
  editor must NOT call the `cmd::*` command fns for mutations: `cmd::tax::set_profile`
  (cmd/tax.rs:14–23) does `Session::open → tax_profile::set → s.save()` — it opens and drops its
  **own** session per call, which (a) cannot coexist with the editor's held lock
  (`StoreError::Locked`) and (b) is the wrong lifecycle for a live editor. The editor calls the
  **underlying typed setters** against its held session: `tax_profile::set(session.conn(), year,
  &p)?; session.save()?;` — mirroring exactly what `cmd::tax::set_profile` does internally, minus
  the open/drop.
- **Mutation confinement.** ALL of `conn()` / `save()` / `tax_profile::set` / `append_decision`
  are confined to ONE editor module, `src/edit/persist.rs` — enforced by the editor's OWN
  E10-analog mechanized gate (same scanner structure as btctax-tui's export.rs:690–919,
  including comment-stripping and the plant-a-token self-check; allowlist = `edit/persist.rs`).
  **[R0-I1]** The gate additionally bans the vault-CREATING constructors (`Session::create` /
  `Session::repair` / `Vault::create` / `Vault::repair`) crate-wide in non-test code — they
  create/overwrite a vault file outside `Vault::save`'s atomic path without tripping any other
  token, and the guarantee names `Vault::save` as the ONLY sanctioned vault-file write path.
  See D3.
- **VaultLock exclusivity (documented concurrency story).** `Session::open` acquires the store's
  single-instance lock (session.rs:53–58, NFR7); `Vault` holds it as `_lock` for its lifetime
  (vault.rs:137–142). While the editor runs, the CLI (or a viewer) CANNOT open the vault — it gets
  `StoreError::Locked`. Conversely the editor shows the Locked screen when something else holds
  it. There is no concurrent-writer case to reason about; this is stated in the editor's docs.
- **Save-per-action atomicity (crash-safety statement).** Every confirmed mutation is immediately
  persisted: `Vault::save` = `db_to_bytes` → encrypt → `atomic_write` (vault.rs:147–151);
  `atomic_write` = write `.tmp` + fsync → copy live file to `.bak` + fsync → atomic `rename`
  (atomic.rs:6–31); `recover_target` runs at the next `Vault::open` (vault.rs:124; the fn at
  atomic.rs:35–44). **[DRIFT-1]** The recon cited "recover_target at next open — vault.rs:147–151";
  147–151 is `Vault::save` — the recover call site is vault.rs:124. Consequence, stated as the
  chunk's durability contract: a crash **between** actions loses nothing (every confirmed action
  is already on disk); a crash **during** a save leaves either the old or the new complete
  encrypted image (plus the fsync'd `.bak`) — never a torn vault.
- **Per-mutation confirmation.** No mutation executes without the confirmation modal that shows
  the EXACT payload (every field name + value + the target year). Enter confirms
  (persist + save + re-project); Esc cancels with **nothing written** (KAT-C1). The modal key
  dispatch precedes all other dispatch (the viewer's R0-M4 lesson, btctax-tui main.rs:124–153).
- **Unlock UX identical to the viewer.** Same masked-input `UnlockState` (pre-allocated
  never-reallocating buffer, unlock.rs:29–64), same `mem::take` → `Passphrase::new` move (never
  cloned), same `BTCTAX_PASSPHRASE` fast path, same error mapping. Passphrase hygiene is
  unchanged from the viewer spec [R0-I2/M7 there].
- **The user must know which binary they're in.** A clear "EDITOR" visual marker on every screen
  (D2). The viewer and editor are visually distinguishable at a glance.
- **Viewer compile-level read-only property is a viewer property.** The viewer holds its `Session`
  in an immutable binding (unlock.rs:94–95) making `save()` (`&mut self`, session.rs:66)
  compile-impossible. The editor **intentionally does not share that property** — its held session
  is mutable. The editor's substitute guarantee is the D3 confinement + gate + the D4 confirmation,
  plus the D5 behavioral tests. This contrast is stated so no reviewer mistakes the editor for a
  broken viewer.

## Current state (recon @ `22cda75` — every line re-verified)

- **`crates/btctax-tui/Cargo.toml:8–10`** — `[[bin]] name = "btctax-tui"`; there is **no `[lib]`
  target**. All modules are declared in `main.rs:13–17` (`mod app; mod draw; mod export; mod tabs;
  mod unlock;`) — nothing is externally reachable. **This gap is D1.**
- **`crates/btctax-tui/src/app.rs:104–111`** — `pub struct Snapshot { events, state, cli_config,
  profiles: BTreeMap<i32, TaxProfile>, tables: BundledTaxTables, donation_details }`. Everything
  the editor's browse tabs need; `pub` today only within the bin.
- **`crates/btctax-tui/src/app.rs:18–24, 27–36`** — `Screen { Unlock, Locked, Viewer }` and the
  six-variant `Tab` (derive lines included [R0-N1]; `ALL`/`title`/`index`/`next`/`prev` at
  app.rs:38–96).
- **`crates/btctax-tui/src/app.rs:117–142`** — `App` (holds `unlock`, `snapshot`,
  `selected_year`, four `TableState`s, `export_modal: Option<ExportConfirmState>`,
  `export_status`). To stay INTERNAL per the binding architecture.
- **`crates/btctax-tui/src/unlock.rs:29–64`** — `UnlockState` (the masked-input widget:
  `PASSPHRASE_CAP`-preallocated buffer, `push_char`/`pop_char` with the never-reallocate cap
  check) — the only text-input precedent in the codebase.
- **`crates/btctax-tui/src/unlock.rs:93–130`** — `pub fn attempt_open(vault_path, pp) ->
  OpenOutcome` (93–107) and private `fn build_snapshot(&Session) -> Result<(Snapshot, i32),
  CliError>` (112–130; takes `&Session` — already correct for post-mutation re-projection).
  **[DRIFT-2]** `attempt_open` DROPS the `Session` at scope exit (it returns only
  `Success(Box<Snapshot>, i32)`) — "attempt_open reuse" for a session-HOLDING editor therefore
  requires a new seam that returns the session; D1 adds `open_session` and re-implements
  `attempt_open` as a behavior-identical wrapper over it.
- **`crates/btctax-tui/src/unlock.rs:134–141`** — `pub fn latest_year(&LedgerState) -> i32`.
- **`crates/btctax-tui/src/unlock.rs:144–164`** — private `map_open_error` (the unlock-screen
  error strings: WrongPassphrase / Locked / HalfCreatedVault / NotFound). Single-sourced by D1's
  `open_session` so the editor's unlock messages are identical by construction.
- **`crates/btctax-tui/src/main.rs:37–41, 50–56, 60–66`** — `restore_terminal` /
  `TerminalGuard` / `setup_panic_hook` (terminal-lifecycle safety; currently bin-private).
- **`crates/btctax-tui/src/main.rs:119–223`** — `handle_key` with the modal-priority dispatch
  (124–153) — the structural template for the editor's modal handling.
- **`crates/btctax-tui/src/draw.rs:13–19`** — `pub fn draw(frame, app: &mut App)`;
  **`draw.rs:139–169`** — `draw_export_modal` (Clear + `centered_rect` + bordered `Paragraph`) —
  the structural template for D4's mutation-confirmation modal.
- **`crates/btctax-tui/src/tabs/*.rs`** — the six per-tab renderers: `holdings.rs:19`,
  `disposals.rs:18`, `income.rs:18`, `forms.rs:47` take `(frame, area, &mut App)`;
  `tax.rs:19`, `compliance.rs:28` take `(frame, area, &App)`. **[DRIFT-3]** The viewer's
  TestBackend suite calls the tab draw fns **directly with an `App`**
  (tabs/tests.rs:92, 104, 116, 827, 839, 851; tests.rs:128 additionally calls the full-frame
  `crate::draw::draw(f, app)` [R0-N1]) — so their signatures cannot change without changing the
  test suite, AND they cannot be externally `pub` while `App` stays internal (Rust E0446,
  private type in public interface). D1 resolves this with extracted App-free `pub`
  renderers + unchanged `pub(crate)` wrappers; the recon's "make `draw::draw` + `tabs::*::draw`
  pub" is adjusted accordingly (the one deliberate deviation, justified in D1).
- **`crates/btctax-tui/src/export.rs:690–919`** — the E10 mechanized-gate `#[test]`: walks
  `src/`, splits non-test/test regions at `#[cfg(test)]`, **strips `//` comments before
  matching** (export.rs:748–757), scans a forbidden-token table with an allowlist, and
  self-checks by planting runtime-constructed tokens in a temp file (export.rs:882–912). The
  editor's gate (D3) clones this structure.
- **`crates/btctax-tui/src/unlock.rs:470–526`** —
  `vault_file_bytes_unchanged_after_open_build_snapshot_drop`: read vault bytes → do the thing →
  re-read → assert byte-identical. The pattern for D5's cancel-path test.
- **`crates/btctax-cli/src/tax_profile.rs:16–22, 50–62`** — `init_table` (idempotent DDL) and
  `pub fn set(conn, year, &TaxProfile)` — the **side-table upsert** (`INSERT … ON CONFLICT(year)
  DO UPDATE`). `get` (28–47) and `all` (65–81) are the reads. This module is `pub` in the
  `btctax_cli` lib (lib.rs exports it; `cmd::tax` and `Session` already call it).
- **`crates/btctax-cli/src/session.rs:61–63, 66–69`** — `Session::conn(&self) -> &Connection`
  and `Session::save(&mut self)`. `session.rs:83–92` — the typed reads `tax_profile(year)` /
  `all_tax_profiles()`.
- **`crates/btctax-cli/src/cmd/tax.rs:14–23`** — `set_profile`: opens/drops its own `Session`
  per call. Recon-verified **wrong for a live editor** (see Hard constraints); the editor
  bypasses `cmd::*` entirely.
- **`crates/btctax-core/src/tax/types.rs:31–68`** — `TaxProfile`: 9 struct fields = **10 leaf
  fields** (the `Carryforward` field, types.rs:21–25, contributes `short` + `long`):
  `filing_status`, `ordinary_taxable_income`, `magi_excluding_crypto`,
  `qualified_dividends_and_other_pref_income`, `other_net_capital_gain`,
  `capital_loss_carryforward_in.short`, `capital_loss_carryforward_in.long`, `w2_ss_wages`,
  `w2_medicare_wages`, `schedule_c_expenses`. `FilingStatus` (types.rs:9–15): `Single | Mfj |
  Mfs | HoH | Qss` — 5 variants, ideal for Tab-cycling. This 10-leaf surface matches the CLI's
  10 value flags (main.rs:96–149) and the `--show` output's 10 lines (main.rs:663–685).
- **`crates/btctax-cli/src/main.rs:646–779`** — the `TaxProfile` command arm: the clap-side
  validation rules the editor must mirror EXACTLY (enumerated in D4): required-field checks at
  690–716, optional-default-0 at 718–760, negativity rejections at 738–740 / 746–750 / 756–760,
  all money parsing via `eventref::parse_usd_arg` = `Decimal::from_str(s.trim())`
  (eventref.rs:76–78). `--show` (661–687) prints the stored profile — the editor's equivalent
  is D4's form pre-population.
- **`crates/btctax-core/src/persistence.rs:101–119`** — the `events` table DDL: `ordinal INTEGER
  PRIMARY KEY AUTOINCREMENT` ("insertion order ONLY"), `event_id TEXT NOT NULL UNIQUE`,
  `payload_json TEXT NOT NULL`. There is `load_all` (264–328, unordered by design) but **no
  ordered read** — D5 adds `load_all_ordered` for the prefix test. `append_decision`
  (238–262) is the chunk-2 writer; named in the guarantee + gate now.
- **`crates/btctax-store/src/vault.rs:116–142, 147–151`** — `Vault::open` (runs
  `recover_target`/`reap_tmp` at 124–125, acquires the lock) and `Vault::save`.
  `atomic.rs:6–44` — `atomic_write` + `recover_target`.
- **Workspace `Cargo.toml:1–7`** — members list (add the new crate); `[workspace.package]` has
  `edition`/`license`/`rust-version = "1.88"`; there is **NO `[workspace.dependencies]` table** —
  external deps are pinned explicitly per crate (btctax-tui/Cargo.toml:17–24 pins
  `ratatui = "0.29"`, `crossterm = "0.28"`, `thiserror = "1"`, `rust_decimal = "1"`,
  `time = "0.3"`; the editor matches these versions). **[DRIFT-4]** The viewer spec's MSRV-1.74
  note is stale: workspace `rust-version` is now **1.88**.

## Design

### D1 — the `btctax-tui` lib split (pure visibility refactor; viewer byte-identical)

Add to `crates/btctax-tui/Cargo.toml`:

```toml
[lib]
name = "btctax_tui"
path = "src/lib.rs"
```

(the `[[bin]]` stays exactly as is). New `src/lib.rs` takes over the module declarations and the
bin-level items; `main.rs` shrinks to a thin entry point:

```rust
// main.rs — the whole binary:
fn main() -> std::io::Result<()> {
    btctax_tui::run_viewer()
}
```

`pub fn run_viewer() -> io::Result<()>` in the lib contains **verbatim** the old `main` body
(setup_panic_hook → parse_vault_path → enable_raw_mode → TerminalGuard → EnterAlternateScreen →
Terminal::new → run → restore_terminal). `handle_key`, the scroll helpers, `run`, and
`parse_vault_path` move (verbatim) from `main.rs` into the lib (lib.rs or a `viewer.rs` module —
implementer's choice; content unchanged). The `#[cfg(test)] mod tests` currently in `main.rs`
moves with them, **content unchanged** — the whole viewer test suite (main.rs tests, unlock.rs
tests, export.rs tests incl. E10, tabs/tests.rs) runs verbatim under the lib target. Rationale
for moving rather than double-compiling: a bin that re-declares the same module files would
compile and test everything twice and give the editor non-identical types.

**Externally-`pub` surface after Task 1** (the editor's entire view of the viewer crate):

| Item | Today | Change |
|---|---|---|
| `app::Snapshot` (app.rs:104–111) | `pub` in bin | `pub` in lib (module `pub`) |
| `app::Screen`, `app::Tab` (app.rs:19–36) | `pub` in bin | `pub` in lib |
| `unlock::UnlockState` + `PASSPHRASE_CAP` (unlock.rs:26, 29–64) | `pub` in bin | `pub` in lib |
| `unlock::attempt_open` + `OpenOutcome` (unlock.rs:72–107) | `pub` in bin | `pub` in lib; becomes a thin wrapper over `open_session` (below) — signature + behavior + tests unchanged |
| `unlock::open_session` — **NEW** | — | `pub fn open_session(vault_path: &Path, pp: Passphrase) -> SessionOpenOutcome` with `pub enum SessionOpenOutcome { Success { session: Session, snapshot: Box<Snapshot>, year: i32 }, Locked, Error(String) }`. Body = today's attempt_open logic (Session::open → error mapping via the existing private `map_open_error` → `build_snapshot`) except the session is RETURNED, not dropped. `attempt_open` = `match open_session(..) { Success { session, snapshot, year } => { drop(session); OpenOutcome::Success(snapshot, year) }, .. }` — the viewer's structural property "the viewer App never stores a Session" (relied on by SPEC_tui_export's backstop note) is **preserved**: the viewer still drops it at unlock [DRIFT-2 resolution]. **[R0-M5] Passphrase-drop ordering PINNED:** `open_session` zeroizes the passphrase (`drop(pp)`) immediately after `Session::open` succeeds and BEFORE `build_snapshot` — exactly today's ordering (unlock.rs:100–101). This is the one hygiene-relevant ordering in the re-cut seam and the wrapper-consistency KAT cannot detect its loss, so it is a stated requirement, verified by inspection at whole-diff. |
| `unlock::build_snapshot` (unlock.rs:112–130) | private | **`pub`** — the editor's re-projection call |
| `unlock::latest_year` (unlock.rs:134–141) | `pub` in bin | `pub` in lib |
| `restore_terminal` / `TerminalGuard` / `setup_panic_hook` (main.rs:37–66) | private in bin | **`pub`** in lib (TerminalGuard gets a `pub fn new()` or unit-struct pub construction) |
| `tabs::{holdings,disposals,income,tax,forms,compliance}::render` — **NEW** | — | App-free `pub` renderers extracted from each tab's `draw`: `render(frame, area, snap: &Snapshot, year: i32, table_state: &mut TableState)` for the four stateful tabs (holdings ignores `year`); `render(frame, area, snap: &Snapshot, year: i32)` for tax/compliance. Each existing `draw(frame, area, app)` becomes a `pub(crate)` thin wrapper: handle the `snapshot == None` placeholder exactly as today, then delegate. **Call sites (draw.rs:108–115) and the TestBackend tests (tabs/tests.rs:92–128, 827–851) keep calling the wrappers — unchanged** [DRIFT-3 resolution]. |
| `draw::draw_unlock_screen` — **NEW** | (private `draw_unlock(frame, &App)`, draw.rs:21–61) | extracted `pub fn draw_unlock_screen(frame, vault_path: &Path, unlock: &UnlockState, title: &str, note_line: &str)`; the private `draw_unlock` delegates with the viewer's exact current title/note strings — viewer rendering byte-identical; the editor passes its EDITOR-branded strings. |
| `app::App`, `export::ExportConfirmState`, `export::do_export`, `draw::draw`, `handle_key` | `pub`/`pub(crate)` in bin | **INTERNAL** (`pub(crate)` or private-module) — NOT reachable from the editor. `draw::draw` cannot be externally `pub` while `App` is internal (E0446); the editor composes the per-tab `render` fns instead [DRIFT-3]. |

**Viewer-behavior-identical acceptance (Task 1 gate):** the viewer's full test suite passes with
**zero test-content changes** (file moves only, for the main.rs tests); E10 passes unchanged;
`cargo run --bin btctax-tui` renders identically (same strings, same keys — nothing in any
render or dispatch path changed, only visibility and extraction seams).

### D2 — the `btctax-tui-edit` crate: EditorApp + lifecycle

New workspace member `crates/btctax-tui-edit` (add to workspace `members`,
Cargo.toml:3). `Cargo.toml`: `edition.workspace = true`, `rust-version.workspace = true`,
`license.workspace = true`; `[[bin]] name = "btctax-tui-edit"`; deps: `btctax-tui` (path — the
lib), `btctax-cli`, `btctax-core`, `btctax-store`, `btctax-adapters` (path), and explicit pinned
`ratatui = "0.29"`, `crossterm = "0.28"`, `rust_decimal = "1"`, `time = "0.3"` (matching the
viewer's pins; NO `[workspace.dependencies]` exists). `tempfile = "3"` +
`rust_decimal_macros = "1"` under `[dev-dependencies]` as needed.

Modules: `main.rs` (entry + `handle_key` + run loop), `editor.rs` (`EditorApp`,
`EditorScreen`), `edit/mod.rs` + `edit/persist.rs` (D3), `edit/form.rs` (D4 form state +
validation), `edit/modal.rs` or inline in a `draw_edit.rs` (D4 modal rendering), `draw_edit.rs`
(top-level draw: EDITOR chrome + delegate to the viewer's `tabs::*::render`).

```rust
pub struct EditorApp {
    pub vault_path: PathBuf,
    pub unlock: btctax_tui::unlock::UnlockState,     // identical unlock UX
    pub screen: EditorScreen,                        // Unlock | Locked | Browse
    pub tab: btctax_tui::app::Tab,
    pub should_quit: bool,
    /// The LIVE session — held for the whole TUI session (VaultLock exclusivity:
    /// the CLI cannot run concurrently). `Some` iff `snapshot` is `Some`.
    pub session: Option<btctax_cli::Session>,
    pub snapshot: Option<btctax_tui::app::Snapshot>,
    pub selected_year: i32,
    // per-tab TableStates (holdings/disposals/income/forms) — same as the viewer
    pub holdings_state: TableState, /* … disposals/income/forms … */
    /// The tax-profile form. `Some` while the form is open.
    pub profile_form: Option<ProfileFormState>,      // D4
    /// The per-mutation confirmation modal. `Some` while awaiting Enter/Esc.
    pub mutation_modal: Option<MutationModalState>,  // D4
    /// One-line status (saved / error), shown in the footer; cleared on the next
    /// NON-MODAL key press (mirrors the viewer's export_status semantics, app.rs:140
    /// [R0-N5] — the modal's own Enter/Esc handling must not instantly wipe the
    /// status it just set).
    pub status: Option<String>,
}
```

**Unlock flow.** `do_unlock` mirrors the viewer's (mem::take → `Passphrase::new`, never cloned)
but calls `btctax_tui::unlock::open_session` and on `Success { session, snapshot, year }` stores
**both**: `self.session = Some(session); self.snapshot = Some(*snapshot); self.selected_year =
year; self.screen = Browse`. `Locked` → the Locked screen (same wording as the viewer, retry with
`r`); `Error(msg)` → the unlock error line (identical strings — single-sourced in
`map_open_error`). `BTCTAX_PASSPHRASE` fast path mirrored. Terminal lifecycle: reuse
`btctax_tui::{setup_panic_hook, TerminalGuard, restore_terminal}` — same raw-mode/alt-screen
safety as the viewer.

**Browse.** The six viewer tabs, read-only, rendered via the D1 `tabs::*::render` fns with the
editor's own `Snapshot`/`selected_year`/TableStates. Keys mirror the viewer (Tab/BackTab, ←/→
year, ↑/↓ j/k, PgUp/PgDn, g/G, q/Esc quit) — plus **`p`** opens the tax-profile form for
`selected_year` (no-op when `snapshot.is_none()`). No `e` export binding in chunk 1 (out of
scope; the viewer exists for that).

**EDITOR visual marker** (the user must know which binary they're in):
- Unlock screen title: `" btctax-tui-edit — Unlock Vault [EDITOR] "`; the note line reads
  `"offline · local · EDITOR — writes on explicit confirmation only"` (the viewer says
  `"offline · local · read-only · PGP-encrypted"`, draw.rs:52 — visibly different).
- Browse tab-bar block title: `" btctax-tui-edit [EDITOR] — {vault_path} "`, styled distinctly
  (e.g. bold red/inverse `[EDITOR]` badge) vs the viewer's `" btctax-tui — {vault_path} "`
  (draw.rs:97).
- Footer includes `p: edit tax profile` and the `[EDITOR]` badge.

**Re-projection.** After every confirmed mutation: `persist` (D3) then
`build_snapshot(session)?` (pub from D1; takes `&Session`, unlock.rs:112 — already correct)
replaces `self.snapshot`; `selected_year` is preserved (no jump). On the (read-only,
near-impossible) re-projection `Err`: keep the previous snapshot, set an error `status` telling
the user to restart the editor — the SAVE already succeeded and is on disk. Recorded perf note
**[R0-N3]**: `build_snapshot` → `load_events_and_project` reloads `BundledTaxTables` and
`BundledPrices` on every mutation — an accepted correctness-first cost for chunk 1 (one
mutation per user confirmation, not a hot path); optimizing the reload is a FOLLOWUP only if it
ever matters, not a bug to be 'discovered' later.

### D3 — `edit/persist.rs` + the editor's mechanized gate

`edit/persist.rs` is the ONLY module in `btctax-tui-edit` permitted to name the mutation
surface. Chunk-1 contents:

```rust
//! The ONLY module in btctax-tui-edit that touches the mutation surface:
//! conn() / save() / tax_profile::set / append_decision live here and nowhere else.
//! Guarantee: "writes ONLY append-only events + typed side-table upserts via
//! edit/persist.rs, each behind an explicit payload-showing confirmation; the vault
//! file only via Vault::save's atomic path."

/// Upsert the tax profile for `year` and atomically save the vault.
/// Mirrors cmd::tax::set_profile (cmd/tax.rs:14–23) minus the open/drop —
/// the editor operates on its HELD session.
pub fn persist_tax_profile(
    session: &mut Session, year: i32, p: &TaxProfile,
) -> Result<(), CliError> {
    btctax_cli::tax_profile::set(session.conn(), year, p)?;  // typed side-table upsert
    session.save()?;                                          // encrypt + atomic_write
    Ok(())
}
```

(`append_decision` is named in the doc-comment and the gate now; its first `pub fn` here arrives
with chunk 2.)

**The editor's mechanized gate (KAT-G1)** — an in-crate `#[test]` cloning the E10 scanner
structure (export.rs:690–919: src-walk via `CARGO_MANIFEST_DIR`, non-test/test region split at
`#[cfg(test)]`, `//`-comment stripping before matching, `file:line` failure output, and the
plant-a-token self-check with runtime-constructed tokens). The editor's normative token table:

| Pattern | Rule in `btctax-tui-edit` |
|---|---|
| `conn(` | FORBIDDEN outside `edit/persist.rs` in non-test code. Test regions may use it **read-only** (e.g. `load_all_ordered(session.conn())` verification in KAT-P1). |
| `save(` | FORBIDDEN outside `edit/persist.rs` in non-test code. |
| `tax_profile::set` / `append_` | FORBIDDEN outside `edit/persist.rs` (non-test). Test regions may seed fixtures via `append_decision` (KAT-P1 setup). |
| `cmd::` | FORBIDDEN in non-test code (the cmd fns open/drop their own sessions — wrong for the live editor, and they'd deadlock on the held lock). `cmd::init::run` in test code is the sole exception (fixture setup — the same documented exception as the viewer's gate). |
| `Session::create` / `Session::repair` / `Vault::create` / `Vault::repair` | **[R0-I1]** FORBIDDEN everywhere in non-test code — four explicit tokens (NOT a broad `::create(`, which would collide with the `File::create`/`create_dir` rows). These constructors (session.rs:30–32, 37–39; the `Vault` pair in btctax-store) **create/overwrite a vault file on disk** without tripping `save(`/`conn(`/`cmd::` or any fs verb at the call site — exactly the class of vault-file writer the guarantee statement excludes ("the vault file only via `Vault::save`'s atomic path"). Test regions keep the sole sanctioned fixture-creation path, `cmd::init::run` (which lives outside the scanned crate). |
| `export_snapshot` / `write_csv_exports` / `write_form_csvs` | FORBIDDEN everywhere (chunk 1 has no export; the viewer owns form-CSV export). |
| `fsperms` / `open_owner_only` / `mkdir_owner_only` / `File::create` / `File::options` / `OpenOptions` / `fs::write` / `write_owner_only` / `create_dir` / `DirBuilder` / `set_permissions` / `fs::copy` / `fs::rename` / `fs::remove_` | FORBIDDEN everywhere in non-test code — the editor performs **no direct filesystem writes at all**; the vault file is written only inside `btctax-store` via `Vault::save`'s atomic path. Test regions may use fs verbs for fixtures/verification (temp dirs, byte reads). |

Self-check: plant `save(`, `conn(`, `tax_profile::set`, and **`Session::create`** [R0-I1]
(all runtime-constructed strings, e.g. `format!("Session::{}", "create")`, so no literal
forbidden token appears in the test source) in a temp file and assert the scanner reports each;
assert the real tree is clean.

**Inherited scanner conventions restated [R0-N2]** (accepted in the viewer's E10; the editor
holds them **by construction** rather than fixing the scanner): (a) the scanner sets `in_test`
at the FIRST `#[cfg(test)]` and never resets — therefore the `#[cfg(test)] mod tests` block is
the LAST item in every editor module (the existing house convention); (b) comment-stripping is
naive from the first `//` — therefore no non-test editor code places a load-bearing token after
a string literal containing `//` (in practice: don't put `//` inside non-test string literals).

**Recorded limitation (mirrors the viewer's R0-N3):** `persist_tax_profile` is a `pub fn` freely
callable; "the confirmation modal gates the ONLY call site" is a **procedural** guarantee
(enforced by KAT-G1's confinement of the *surface*, the KATs, and whole-diff review), not a
type-level proof. A sealed confirmation-token type is a FOLLOWUP if the editor grows more flows.

### D4 — the tax-profile form, validation, and confirmation modal

**`ProfileFormState`** (in `edit/form.rs`):

```rust
pub struct ProfileFormState {
    pub year: i32,                       // the target year (selected_year at open)
    pub filing_status: FilingStatus,     // cycled via Tab; default Single or existing
    pub fields: [FieldBuffer; 9],        // the 9 money leaves, fixed order (below)
    pub focus: usize,                    // 0 = filing_status, 1..=9 = the money fields
    pub error: Option<String>,           // validation error line
}
```

`FieldBuffer` follows the `UnlockState` push/pop pattern (unlock.rs:42–63 — the only text-input
precedent): `String::with_capacity(FIELD_CAP)` with `FIELD_CAP = 64`, `push_char` silently
ignoring input past the cap, `pop_char`. Rendered **plaintext** (these are not secrets — no
masking; the pattern reused is the buffer discipline, not the masking).

Field order (matches the `--show` output, main.rs:663–685): `filing_status`,
`ordinary_taxable_income`, `magi_excluding_crypto`, `qualified_dividends_and_other_pref_income`,
`other_net_capital_gain`, `carryforward_short`, `carryforward_long`, `w2_ss_wages`,
`w2_medicare_wages`, `schedule_c_expenses` — the 10 leaf fields of types.rs:31–68.

**Pre-population (the `--show` equivalent).** On `p`: if `snapshot.profiles.get(&year)` is
`Some(p)` (the snapshot already carries all profiles — app.rs:108, loaded via
`all_tax_profiles`), every buffer is pre-filled with the field's `Decimal` `Display` string and
`filing_status` is set from `p` — editing an existing year starts from its current values.
Otherwise: `filing_status = Single`, all buffers empty (required fields must be typed; optional
empties become $0 at validation, mirroring the CLI's omitted-flag defaults).

**Form keys** (form dispatch precedes Browse dispatch; modal dispatch precedes both):
`↑/↓` move focus; **`Tab` cycles `FilingStatus`** through the 5 variants (types.rs:9–15) when
any field is focused on row 0, and moves focus down otherwise (implementer may instead scope
Tab-cycling to the filing-status row only — pin: Tab NEVER inserts text); printable chars →
`push_char` on the focused buffer; `Backspace` → `pop_char`; `Enter` → validate; `Esc` → close
the form, **nothing written**, no state change beyond the form itself.

**Validation — RE-OWNED at submit** as a pure fn
`validate(form: &ProfileFormState) -> Result<TaxProfile, String>`, mirroring the CLI's
clap-side rules **exactly** (main.rs:688–760; parse = `Decimal::from_str(s.trim())`, the
`parse_usd_arg` semantics of eventref.rs:76–78). The complete rule enumeration:

1. `filing_status` — required; structurally always satisfied (the widget holds one of the 5
   `FilingStatus` variants at all times; mirrors main.rs:690–692's required check).
2. `ordinary_taxable_income` — **required**: empty buffer → error
   `"ordinary-taxable-income is required"` (main.rs:693–700); else `Decimal::from_str` on the
   trimmed buffer; parse failure → `"bad USD {input}"`.
3. `magi_excluding_crypto` — **required** (main.rs:701–708); same parse rule.
4. `qualified_dividends_and_other_pref_income` — **required** (main.rs:709–716); same parse rule.
5. `other_net_capital_gain` — optional: empty → `0` (main.rs:718–722); else parse.
6. `carryforward_short` — optional: empty → `0` (main.rs:723–727); else parse.
7. `carryforward_long` — optional: empty → `0` (main.rs:728–732); else parse.
8. `w2_ss_wages` — optional: empty → `0`; parsed value with `is_sign_negative()` →
   error `"w2-ss-wages must not be negative"` (main.rs:733–740).
9. `w2_medicare_wages` — optional: empty → `0`; negative → error (main.rs:741–750).
10. `schedule_c_expenses` — optional: empty → `0`; negative → error (main.rs:751–760).

**Parity pin:** the CLI negativity-checks ONLY fields 8–10. Fields 2–7 accept negative values at
the CLI today (the `Carryforward` doc-contract says magnitudes ≥ 0, types.rs:17–19, but
main.rs:723–732 does not enforce it). The editor mirrors this **exactly** — no invented rules,
no dropped rules; the CLI/editor asymmetry risk is zero by construction. (Tightening both
surfaces' carryforward validation is recorded as a FOLLOWUP, not smuggled in here.)

**Empty-buffer pin [R0-M4]:** "empty" = byte-length 0, tested **before** any trimming. A
whitespace-only buffer is NOT empty — it takes the parse path and fails
(`Decimal::from_str` on the trimmed-empty string errors), exactly matching the CLI: an
*absent* flag defaults to 0, but `--flag "  "` is a parse error inside `parse_usd_arg`.
For required fields the mapping is symmetric: len-0 → the "required" error; whitespace-only →
the parse error. KAT-V11 pins both cases.

**Error-string note [R0-N4]:** the editor's messages differ cosmetically from the CLI's
clap-facing strings (no `--` flag prefix, no "when setting a profile" suffix, `"bad USD
{input}"` vs the CLI's `bad USD {s:?}: {e}`). Parity is **behavioral** (accept/reject per the
10 rules above) — string parity is NOT promised, and no later review should claim it was.

Validation failure → `form.error = Some(msg)`, form stays open. Success → build the
`TaxProfile` (exactly the main.rs:762–775 construction) and open the modal.

**`MutationModalState`** (the per-mutation confirmation — a structural clone of
`draw_export_modal`, draw.rs:139–169: `Clear`, `centered_rect`, bordered wrapped `Paragraph`;
drawn over the form/tab content, checked before all other dispatch):

```rust
pub struct MutationModalState {
    pub year: i32,
    pub profile: TaxProfile,   // the VALIDATED payload — what will be persisted, verbatim
}
```

The modal shows the **EXACT payload** — the target year and every one of the 10 leaf field
names + values (the validated `Decimal`s, not the raw buffers; `filing_status` via
`render::filing_status_tag`), i.e. precisely the `--show` field list (main.rs:663–685):

```
╔═ Confirm: set tax profile for 2025 — WRITES THE VAULT ═══╗
║  year: 2025                                              ║
║  filing_status: single                                   ║
║  ordinary_taxable_income: 120000                         ║
║  magi_excluding_crypto: 130000                           ║
║  qualified_dividends_and_other_pref_income: 0            ║
║  other_net_capital_gain: 0                               ║
║  capital_loss_carryforward_in.short: 0                   ║
║  capital_loss_carryforward_in.long: 0                    ║
║  w2_ss_wages: 0                                          ║
║  w2_medicare_wages: 0                                    ║
║  schedule_c_expenses: 0                                  ║
║                                                          ║
║  Replaces any existing profile for this year (upsert).   ║
║  Saved immediately via the vault's atomic write path.    ║
║                                                          ║
║  [Enter] Confirm & save     [Esc] Cancel — writes nothing║
╚══════════════════════════════════════════════════════════╝
```

**Modal keys** (dispatch order pinned: modal → form → screen; the R0-M4 lesson —
`Esc` must never fall through to a quit arm):
- `Enter` → `persist_tax_profile(session, year, &profile)` (D3); on `Ok`: re-project
  (`build_snapshot`), close modal + form, `status = "Saved tax profile for {year}"`; on `Err(e)`:
  close modal, keep the form open (the user's input is not lost),
  `status = "Save error: {e}"` — **and the vault is unchanged on the disk-error path**
  (`atomic_write` fails before the rename or leaves the old image; the `.bak` discipline covers
  the torn case).

  **Failed-save semantics PINNED [R0-M1]:** when `tax_profile::set` succeeded but
  `session.save()` failed, the HELD in-memory session already carries the confirmed upsert while
  the on-disk vault remains the pre-action state (per the atomic path). This divergence is
  **intentional and safe — do NOT roll back the side-table**: everything in the session was
  explicitly confirmed by the user at some point, and the upsert is idempotent. Consequences,
  stated so no implementer "fixes" them and no later reviewer calls them a leak: (a) a retry
  (re-confirm) re-runs the idempotent upsert + save; (b) ANY later successful save — this
  action's retry or a different confirmed action — also persists the earlier confirmed upsert;
  (c) quitting without a successful save LOSES the unsaved mutation, which is exactly the
  save-per-action contract (nothing is durable until `Vault::save` returns `Ok`). The snapshot
  is NOT re-projected on the `Err` path (the UI keeps showing last-saved state). KAT-S1 (D5)
  tests this path.
- `Esc` → close the modal only (back to the form, buffers intact). **Writes nothing.** Does not
  quit.
- Any other key (including `q`) → swallowed (blocking modal).

### D5 — the two safety tests (+ the core read they need)

**`btctax_core::persistence::load_all_ordered` (new, read-only):**

```rust
/// One raw `events` row, ALL columns except `ordinal` itself [R0-M2].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEventRow {
    pub event_id: String,
    pub kind: String,
    pub source: Option<String>,
    pub source_ref: Option<String>,
    pub decision_seq: Option<i64>,
    pub utc_timestamp: String,
    pub tz_offset_sec: i32,
    pub wallet_json: Option<String>,
    pub payload_json: String,
    pub fingerprint: Option<String>,
}

/// The raw event log in INSERTION order — every persisted column of every row,
/// `SELECT event_id, kind, source, source_ref, decision_seq, utc_timestamp,
///  tz_offset_sec, wallet_json, payload_json, fingerprint
///  FROM events ORDER BY ordinal`.
/// Read-only; exists for append-only prefix verification in tests/tools.
/// (`ordinal` is AUTOINCREMENT insertion order, persistence.rs DDL — the projection
/// ignores it (NFR4); this fn is NOT a projection input.)
pub fn load_all_ordered(conn: &Connection) -> Result<Vec<RawEventRow>, CoreError>
```

**[R0-M2] Full-row width is deliberate:** projecting only `(event_id, payload_json)` would let
a bug that rewrites `utc_timestamp` / `wallet_json` / `tz_offset_sec` / `kind` / `source` /
`source_ref` / `decision_seq` / `fingerprint` on an EXISTING row pass the prefix test on the
confirmed path (KAT-C1's byte-check only covers the cancel path). "Nothing rewritten" is
enforced over the whole persisted row; the fn is new, so the wider SELECT costs nothing and
strengthens chunk-2's strict-prefix test for free.

**KAT-P1 — the append-only prefix test (side-table form, stated precisely).**
The program-level invariant (all chunks): for every editor mutation M,
`post = load_all_ordered(conn)` after M relates to `pre` captured before M by **prefix
equality** — `post.len() >= pre.len()` and `post[..pre.len()] == pre` (the log is append-only;
nothing rewritten, nothing reordered, nothing deleted). **Chunk-1 instantiation:** the
tax-profile set is a SIDE-TABLE **upsert** (tax_profile.rs:50–62), NOT an event append — so for
chunk 1 the prefix test asserts the degenerate strong form: the event log is **UNCHANGED**
(`post == pre`, which implies `post.len() == pre.len()`) by a profile set. The strictly-growing
form (`post.len() = pre.len() + k`, k ≥ 1) arrives with chunk-2's `append_decision` flows and
will reuse this same test skeleton and `load_all_ordered`.

Test body: temp vault via `cmd::init::run` (test-region exception); seed ≥ 2 decision events via
`persistence::append_decision` directly (fixture setup) + `save`; drop. Open the editor's
session; `pre = load_all_ordered(session.conn())`; `persist_tax_profile(&mut session, 2025,
&fixture_profile)`; assert in-memory `load_all_ordered == pre`; drop the session; **reopen** and
assert `load_all_ordered == pre` again (the persisted image agrees). Then the
mutation-actually-happened guard (so the test can't pass vacuously on a no-op):
`session.tax_profile(2025)` round-trips equal to `fixture_profile`; and a second
`persist_tax_profile` with different values still leaves the log `== pre` (upsert, not append)
while the read-back updates.

**KAT-C1 — the cancel-path vault-bytes-unchanged test** (the unlock.rs:470–526 pattern).
Temp vault via `cmd::init::run`; `bytes_before = fs::read(vault)`. Open the editor session
(`open_session`); drive the real dispatch (`handle_key`): `p` → form opens; type into buffers;
`Enter` → modal opens (assert `mutation_modal.is_some()`); **`Esc`** → modal closes (assert
form still open, `!should_quit`); `Esc` → form closes; `q` → quit path; drop the session.
`bytes_after = fs::read(vault)`; assert **byte-identical**. Sub-assertions along the way:
`q` while the modal is open is swallowed (`!should_quit`, modal stays); no `status` is set on
the cancel path. Complement (same test or sibling): after a **confirmed** mutation the vault
bytes DO differ from `bytes_before` (the save really writes) — guarding against a
trivially-green cancel test.

**KAT-S1 — the save-error path [R0-M1/M3]** (`#[cfg(unix)]`). Temp vault; open the editor
session; drive `p` → valid form → `Enter` → modal; make the vault's PARENT DIRECTORY read-only
(mode `0o500`) so `atomic_write`'s `.tmp` creation fails; press `Enter` (confirm) → assert the
D4 Err-arm claims: (1) modal closed, (2) form still open with buffers intact, (3) `status`
contains `"Save error"`, (4) the on-disk vault bytes are byte-identical to before. Then restore
the directory permissions and confirm again → the retry succeeds (the idempotent upsert re-runs
+ save), the profile round-trips, and the event log is still unchanged (side-table). Guard:
skip with an explicit message when the permission denial does not bite (e.g. running as root —
probe by attempting a write into the locked dir and skipping if it unexpectedly succeeds).
**Pre-recorded fallback [R0-M3]:** if this KAT proves flaky in CI, mark it `#[ignore]` AND move
the four Err-arm claims to documented-not-tested status in FOLLOWUPS — the claims must not be
silently dropped, and the downgrade must be explicit.

**Supporting KATs (Task 3, all TDD-red first):**
- **KAT-V1..V11** — one per validation rule in the D4 enumeration (empty-required errors for
  2–4; empty-optional → 0 for 5–7; negative-rejected for 8–10 and **negative-accepted for 2–7**
  — the exact-parity pin; parse-failure for a non-numeric buffer; filing-status structural
  validity by cycling Tab 5 times → back to start). **KAT-V11 [R0-M4]:** whitespace-only
  buffers — an optional field with `"  "` → parse ERROR (not the 0 default); a required field
  with `"  "` → parse error (not the "required" error); len-0 behaves per the empty-buffer pin.
- **KAT-F1** — pre-population: snapshot with an existing 2025 profile → `p` opens the form with
  every buffer equal to the stored field's `Display` string and the stored filing status
  (the `--show` equivalence).
- **KAT-F2** — modal payload exactness (TestBackend): the rendered modal buffer contains the
  year and all 10 leaf field names with the validated values.
- **KAT-F3** — confirm-flow end-to-end: `p` → type a full valid profile → `Enter` → `Enter` on
  the modal → `session.tax_profile(year)` round-trips; the re-projected
  `snapshot.profiles[&year]` equals it; `status` contains "Saved".
- **KAT-F4** — CLI parity: a profile persisted by the editor is read back identically by
  `cmd::tax::show_profile` (cross-binary agreement on the side-table).
- **KAT-G1** — the D3 mechanized gate + self-check.
- **KAT-U1** — unlock parity: wrong passphrase / Locked / success paths produce the same screens
  and messages as the viewer (shared `open_session` seam); on success the editor holds
  `session.is_some()` **and** `snapshot.is_some()`.

## Plan (TDD)

### Task 1 — the `btctax-tui` lib split (viewer-behavior-identical)

**Files:** `crates/btctax-tui/Cargo.toml` (add `[lib]`); NEW `src/lib.rs` (module decls +
`run_viewer` + the moved `main.rs` items, verbatim); `src/main.rs` (shrinks to the thin entry);
`src/app.rs` / `src/unlock.rs` / `src/draw.rs` / `src/tabs/*.rs` (visibility changes + the D1
extractions: `open_session`, `build_snapshot` pub, `tabs::*::render`, `draw_unlock_screen`;
`App`/`ExportConfirmState`/`draw::draw` pinned internal).

Acceptance: the **entire existing viewer test suite passes with zero content changes** (the
main.rs tests move file, verbatim); E10 green; a new thin KAT asserting `attempt_open` still
returns `OpenOutcome` variants identical to `open_session`'s (wrapper consistency). Whole-diff
sanity for the task: no new tokens from the E10 table anywhere in `btctax-tui`.

### Task 2 — the editor skeleton: crate + unlock (session-holding) + browse + gate

**Files:** workspace `Cargo.toml` (member); NEW `crates/btctax-tui-edit/{Cargo.toml,
src/main.rs, src/editor.rs, src/draw_edit.rs, src/edit/mod.rs, src/edit/persist.rs (doc-comment
+ empty surface)}`.

`EditorApp` + `EditorScreen`; unlock via `open_session` (KAT-U1); Locked screen; browse tabs via
`tabs::*::render` + year/scroll keys; the EDITOR markers (D2); terminal lifecycle via the lib's
`TerminalGuard`/`setup_panic_hook`; **KAT-G1** (the gate, green on the skeleton and
self-checked). `p` binding stubs to a no-op until Task 3 (or lands with Task 3 — implementer's
choice; the key must not exist half-wired).

### Task 3 — the profile flow: form + validation + modal + persist + re-projection + safety tests

**Files:** `crates/btctax-tui-edit/src/edit/{form.rs,persist.rs}`, `src/draw_edit.rs` (form +
modal rendering), `src/main.rs` (dispatch order: modal → form → screen);
`crates/btctax-core/src/persistence.rs` (add `load_all_ordered` + its unit test).

D4 in full: `ProfileFormState`/`FieldBuffer`, pre-population, the 10 validation rules (+ the
[R0-M4] empty-buffer pin), `MutationModalState`, `persist_tax_profile` (+ the [R0-M1]
failed-save semantics), re-projection. Tests: **KAT-P1** (the prefix test, side-table form,
full-row `RawEventRow` [R0-M2]), **KAT-C1** (cancel-path bytes-unchanged), **KAT-S1**
(save-error path [R0-M3]), KAT-V1..V11, KAT-F1..F4 — all red before implementation, green
after. KAT-G1 stays green (persist.rs is the only allowlisted module).

### Task 4 — whole-diff review (Phase E) + FOLLOWUPS

Cross-cutting checks:
- **Viewer untouched in behavior:** the viewer suite content-unchanged and green; E10 green;
  no viewer guarantee wording changed; `attempt_open` still drops the session; the viewer `App`
  still never stores a `Session`; `open_session` drops the passphrase immediately after
  `Session::open` succeeds, before `build_snapshot` [R0-M5] (inspection).
- **Editor guarantee:** grep the editor crate per the D3 table (independent second layer over
  KAT-G1): `conn(`/`save(`/`tax_profile::set`/`append_` only in `edit/persist.rs` (non-test);
  `cmd::` only in test code (`cmd::init::run`); zero direct fs-write verbs in non-test code;
  `export_snapshot`/`write_csv_exports`/`write_form_csvs` zero hits;
  **`Session::create`/`Session::repair`/`Vault::create`/`Vault::repair` zero hits in non-test
  code [R0-I1]** (and the KAT-G1 self-check plants one of them).
- **Modal gating:** `persist_tax_profile`'s sole non-test call site is the modal's Enter arm;
  dispatch order modal → form → screen verified; Esc paths write nothing (KAT-C1 green);
  the failed-save Err-arm implements the [R0-M1] pinned semantics (no side-table rollback;
  KAT-S1 green or explicitly downgraded per its pre-recorded fallback [R0-M3]).
- **Validation parity:** the D4 rule enumeration matches main.rs:688–760 one-for-one (including
  the fields-2–7-accept-negatives pin and the [R0-M4] empty-= len-0 pin); parse is
  `Decimal::from_str(trim)`.
- **Payload exactness:** the modal renders all 10 leaf fields + year from the VALIDATED profile.
- **Prefix test correctness:** KAT-P1 asserts log-unchanged (the side-table form) AND the
  mutation round-trip; `load_all_ordered` orders by `ordinal`, selects the FULL row
  (`RawEventRow`, all persisted columns [R0-M2]), and is read-only.
- **Atomicity statement:** persist = set → save; save is the vault's atomic path; no other
  write path exists in the editor.
- **Determinism / hygiene:** passphrase moved never cloned; no plaintext passphrase rendering;
  synthetic fixtures + temp vaults only; exact Decimal formatting (no float).

FOLLOWUPS to record:
- Chunk 2: the decision flows (`append_decision`-backed reconcile/classify/link/void/select-lots)
  — the strict-prefix form of KAT-P1 activates there.
- Sealed confirmation-token type for `persist_*` (the D3 recorded limitation).
- Carryforward negativity validation on BOTH surfaces (CLI + editor, together — parity-preserving).
- `Tab`-cycling scope polish on the form (row-0-only vs everywhere) per first real use.
- The viewer spec's stale MSRV-1.74 note (workspace is 1.88) — doc correction [DRIFT-4].
- Re-projection reload cost (`BundledTaxTables`/`BundledPrices` per mutation) — only if it ever
  matters [R0-N3].
- If KAT-S1 was downgraded per its fallback: the four save-error Err-arm claims are
  documented-not-tested and need a testing seam [R0-M3] (record only if the downgrade happened).

## Out of scope

- **Chunks 2+:** ALL decision flows (reconcile link-transfer / classify-inbound / reclassify /
  set-fmv / void / conflicts / safe-harbor / select-lots / reclassify-income /
  set-donation-details), import, config set, optimize run/accept/consult, safe-harbor attest,
  donation details — every `append_decision`- or other-side-table-backed flow beyond
  `tax_profile::set`.
- **Any viewer-guarantee change** — the viewer's E10 gate, guarantee wording, and write-free
  status are frozen; Task 1 is visibility-only.
- Export from the editor (the viewer's `e` flow is not duplicated here).
- Deleting a tax profile (the CLI has no delete either; parity).
- Undo/redo; multi-year bulk edit; a generic form framework.
- Concurrent CLI+editor operation (VaultLock is exclusive by design — documented, not changed).
- Type-level (sealed-token) enforcement of modal→persist gating — procedural in this chunk.
