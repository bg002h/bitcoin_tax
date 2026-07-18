# P3 recon — TUI deterministic style-aware goldens (digested 2026-07-18)

*Load-bearing facts for P3 (Task 3.1 shared clock seam + Task 3.2 style-aware capture). Full recon in the
session; this is the durable digest so P3 can resume without re-reconning.*

## Versions
- `ratatui 0.29.0`, default features incl. `underline-color` (so `Cell.underline_color` is present).
- `crossterm 0.28.1`. Clock type is `time::OffsetDateTime` (NO chrono in the TUI crates).

## The clock sites — 25 production reads to pin (all `OffsetDateTime::now_utc()`)
No spinners/elapsed/animation reads exist — every clock read is production and must be seam-injected.

**`btctax-tui` — 2, both in `fn handle_key` (`crates/btctax-tui/src/lib.rs`):**
- `lib.rs:247` (`'w'` arm) → `WhatIfPanel::new(snap, year, now)` (what-if as-of date + LT/ST on screen).
- `lib.rs:256` (`'e'` arm) → `export::export_dir_for(&vault, export_now)` — the timestamp becomes the export
  DIR NAME (`btctax-export-YYYYMMDD-HHMMSSZ`), rendered in the modal (`draw.rs:209,257`) AND written to disk.
- `export.rs:30` / `whatif_panel.rs:80` take the timestamp as a PARAM — do NOT read the clock. **The seam
  for the whole viewer is those 2 `handle_key` call sites** (inject a clock into `App`).

**`btctax-tui-edit` — 23, all `crates/btctax-tui-edit/src/main.rs`:**
- 21 modal-confirm handlers capture `now` at Enter-press → `persist_*`/`session.*` decision made-date
  (persisted to the vault). Lines: 1551, 2126, 2474, 2546, 2742, 3085, 3936, 5233, 5802, 6123, 6383, 6868,
  7239, 7312, 7765, 8049, 8280, 8623, 8811, 9112, 9453. (Pattern comment at :1551: "Capture now at
  Enter-press … for determinism.")
- 2 flow-open reads feeding a rendered recompute: `main.rs:2609` `open_method_election_flow` (`today` →
  `exchange_method_election_rows`), `main.rs:9218` `open_optimize_accept_flow` (`now` → `optimize_proposal`).
- Test-only (skip): `main.rs:21677`, `edit/persist.rs:1176`.

## Existing seam to mirror — `resolve_now()` (`btctax-cli/src/main.rs:70`, PRIVATE to the binary)
- `std::env::var_os("BTCTAX_NOW")`; `None` → `OffsetDateTime::now_utc()`; non-UTF-8 → `CliError::Usage`;
  parse `OffsetDateTime::parse(s, &Rfc3339)`, failure → `CliError::Usage` (→ exit 2 via `main`).
- Active path prints an unconditional **stderr** banner "BTCTAX_NOW override active …". KATs:
  `btctax-cli/tests/btctax_now_seam.rs`.
- **No shared btctax-core clock util** (core is deliberately clock-free; each binary reads its own clock).
  P3 must REPLICATE this (a shared helper — promote into a lib the TUI crates can call, or a small per-crate
  copy). NOTE: the CLI's stderr banner can't surface in a raw-mode alt-screen TUI — the TUI variant needs
  its own disclosure surface (status line) IF that property is required (decide in P3 brainstorm/spec).

## The render seam (already decoupled from the event loop — key enabler)
- Viewer top entry: `btctax_tui::draw::draw(frame: &mut Frame, app: &mut App)` (`draw.rs:19`; Viewer →
  `draw_viewer` :107). Per-tab: `tabs::{holdings,disposals,income,forms}::draw(f, area, &mut app)`,
  `tabs::{tax,compliance}::draw(f, area, &app)`.
- Editor top entry: `draw_edit::draw(frame, app: &mut EditorApp)` (`draw_edit.rs:51`).
- Event loops (`lib.rs:637`, `main.rs:9740`) are the ONLY things tied to crossterm raw-mode; goldens bypass
  them and call `draw::draw`/`draw_edit::draw` on a `TestBackend`.

## TestBackend pattern (proven — `btctax-tui/src/tabs/tests.rs`)
```rust
let backend = TestBackend::new(120, 40);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|f| crate::draw::draw(f, &mut app)).unwrap();
let buf: ratatui::buffer::Buffer = terminal.backend().buffer().clone();
```
App for tests (`make_app`, `make_snapshot`): `App::new(fixed_path)`, set `screen=Screen::Viewer`,
`selected_year`, `snapshot=Some(make_snapshot(state))` — NO vault/Session (STRICTLY READ-ONLY). Editor
tests drive `draw_edit::draw` headlessly (e.g. `main.rs:10207,11712,11794,12296`), sizes 80x10..160x44.

## Per-cell style (ratatui 0.29 `Cell`)
`c.symbol()` (fn), `c.fg`, `c.bg`, `c.underline_color`, `c.modifier`, `c.skip` (all pub); `c.style()` returns
combined `Style`. Buffer: `buf.area()`, `buf.cell((x,y)) -> Option<&Cell>`. `Color`/`Modifier` impl Debug.
Serialize per (x,y): glyph + fg/bg/modifier (+ underline_color/skip — decide per §14 gap 7).

## Other non-determinism to pin (besides clock)
1. **Vault path** rendered in titles (`draw.rs:55,125`; `unlock.rs:246,249`; `draw_edit.rs:119`) — pin by
   constructing `App`/`EditorApp` with a fixed synthetic `vault_path`.
2. Export dir name = clock + vault-parent (pin both).
3. No version strings on screen. Ordering is BTreeMap/sorted (deterministic). Prices from bundled data
   (pin `BTCTAX_PRICE_CACHE` as the CLI docs pipeline does).

## Minimal P3 shape (no new production render mode strictly required)
Two injections don't exist yet: (a) a clock seam so the 25 sites take an injected `OffsetDateTime`
(thread a clock into `App`/`EditorApp`, default = `resolve_now()`-style env read); (b) a pinned synthetic
`vault_path`. Then a test/xtask harness builds synthetic `App`/`EditorApp`, renders named screens via
`draw::draw`/`draw_edit::draw` on a `TestBackend`, and serializes the style-aware buffer into
`docs/examples-tui/` goldens (owning FOLLOWUP UX-P3-1). Optionally a real `--dump-screen <name>` subcommand
(slots into `fn main` before `enable_raw_mode()`) instead of a harness — decide in the P3 brainstorm.
