# Whole-Branch (Final Whole-Diff) Review — `btctax-tui` read-only viewer — Round 1

**Artifact:** the full 8-commit branch `30570e0..93a73eb` (diff at
`.superpowers/sdd/review-30570e0..93a73eb.diff`).
**Contract:** `design/SPEC_tui_readonly_viewer.md` + R0 review
`reviews/R0-spec-tui-readonly-round-1.md` (R0 reached 0C/0I in its round 2).
**Reviewer role:** independent final whole-diff reviewer (cross-cutting net over all 4 tasks).
**Scope of concern:** read-only guarantee, passphrase security, terminal safety, offline,
figure parity/no-float, additivity, determinism. A hole in any of the first four, or a
displayed tax figure diverging from the engine, is **Critical**.

## Verdict

**READY TO MERGE — 0 Critical / 0 Important.** 1 Minor, 5 Nit (all non-blocking).

The crate is genuinely read-only, offline, terminal-safe, and passphrase-hygienic, and its
displayed figures are parity-by-construction with the CLI. Every load-bearing guarantee in the
spec is implemented as designed and is either compile-enforced or backed by a behavioral test.
The residual findings are display-string accuracy and dead-code cleanup — none touch a
security-sensitive invariant, and none block merge.

---

## Cross-cutting verification (the 7 dimensions)

### 1. Read-only guarantee (highest priority) — AIRTIGHT ✅
- **Grep of the whole crate for `save(` / `append_` / `cmd::` / `conn(`:** the ONLY hits are
  (a) doc-comments restating the prohibition, and (b) `btctax_cli::cmd::init::run(...)` inside
  `#[cfg(test)]` vault-setup code in `unlock.rs` (7 sites, all tests). **No production hit.**
  No `Session::save()`, no `persistence::append_*`, no `Session::conn()`, no mutating
  `cmd::*` anywhere in shipping code.
- **Compile-level lever:** `unlock.rs:95` binds `let session = Session::open(...)` — an
  **immutable** binding. `save()` is `&mut self`, so it is *compile-impossible* to call. The
  binding is never re-bound `mut`, and the `Session` is dropped (not stored) after `build_snapshot`.
- **`Snapshot` is immutable data:** `App` holds `Option<Snapshot>`; `handle_key` mutates ONLY
  UI-nav fields (`screen`, `tab`, `should_quit`, `selected_year`, `unlock`, `*_state`) — never
  ledger data. Confirmed by inspection of every `handle_key`/scroll helper.
- **Never reads `~/Documents/BitcoinTax/ReadOnly`:** the only path opened is the vault via
  `Session::open(vault_path, ...)`. The default path is `~/Documents/BitcoinTax/vault.pgp`
  (`main.rs:88-91`), NOT the ReadOnly source dir. No CSV/XLSX ingest path is reached.
- **Behavioral proof:** `vault_file_bytes_unchanged_after_open_build_snapshot_drop`
  (`unlock.rs:468-495`) asserts the vault file is **byte-identical** before vs after an
  open→build-Snapshot→drop cycle. Genuinely proves no write to vault data.
- **No mutation escape found.**

### 2. Passphrase security — SOUND ✅
- Buffer is `String::with_capacity(PASSPHRASE_CAP=128)` (`unlock.rs:26,44`); `push_char`
  rejects any char that would push byte-length past the cap (`unlock.rs:53-57`), so the
  `String` **never reallocates** — no partial-passphrase fragments scattered across freed heap.
  Tests `input_past_cap_is_silently_ignored` + `buffer_never_reallocates_within_cap` confirm
  capacity stays 128.
- Handed to the vault by **MOVE**: `Passphrase::new(std::mem::take(&mut self.unlock.buffer))`
  (`app.rs:163`); the store's zeroizing `Passphrase` owns the only copy. `attempt_open` even
  `drop(pp)`s eagerly once `Session::open` succeeds (`unlock.rs:101`).
- **No `.clone()` on the buffer or `Passphrase` anywhere** — the crate's `.clone()` hits are all
  on unrelated values (a date string, `PathBuf` in tests, a form description, `TestBackend`
  buffers). Verified by grep.
- **Never rendered as plaintext:** `draw_unlock` renders `"●".repeat(buffer.chars().count())` —
  it reads only the char *count*, never the content (`draw.rs:32-33`). No error/log line embeds
  the passphrase (`map_open_error` never references `pp`).
- **`BTCTAX_PASSPHRASE` path moves the env `String` straight into `Passphrase::new`**
  (`app.rs:191-195`) — no persistent buffer, no clone.

### 3. Terminal safety — SOUND ✅
- `restore_terminal()` is idempotent (ignores errors from `disable_raw_mode`/`LeaveAlternateScreen`)
  and factored so both the panic hook and the normal/error exit paths call it (`main.rs:35-39`).
- `TerminalGuard` (`Drop → restore_terminal`) is created **immediately after** `enable_raw_mode()`
  (`main.rs:343-347`), so every subsequent failure point — `EnterAlternateScreen`,
  `Terminal::new`, `run()` — is covered by the guard's Drop on scope exit (setup `?`, run `Err`,
  normal return, and panic unwind).
- The panic hook is installed **before** raw mode (`main.rs:339` vs `343`) and chains the default
  hook, restoring the terminal *before* the backtrace prints (`main.rs:58-64`).
- `run()`'s result is captured, then `restore_terminal()` is called explicitly, then the guard
  drops (a third idempotent restore) — belt-and-suspenders. **No path can leave the terminal
  raw/alt-screen.**

### 4. Offline — CONFIRMED ✅
- Only new deps are `ratatui = "0.29"` + `crossterm = "0.28"` (terminal-only).
- **No network crate anywhere in `Cargo.lock`** — grep for `reqwest|hyper|tokio|ureq|curl|isahc|
  attohttpc|rustls|native-tls|openssl|http|surf|awc` returns nothing.
- No `std::net`/`TcpStream`/`connect(`/URL literals in the crate source.
- `BundledTaxTables::load()` / `BundledPrices` are `const`/`include_str!` compiled-in (per R0).

### 5. Figure parity + no-float — CONFIRMED ✅
- Tax tab calls `compute_tax_year` + `compute_se_tax`; Forms calls `form_8949`/`schedule_d`/
  `form_8283`; Compliance calls `build_verify` — the **same core builders the CLI uses**.
- **Spot-check (Tax reconciliation):** the TUI derives
  `ord_attr = r.total_federal_tax_attributable - r.ltcg_tax - r.niit` (`tax.rs:55`); the CLI
  computes the *identical* `ordinary_rate_attributable = r.total_federal_tax_attributable -
  r.ltcg_tax - r.niit` (`cli/render.rs:930`). Identical identity → identical figure.
- **Spot-check (SE tax):** CLI uses `p.filing_status` + `tables.table_for(year)`
  (`cli/cmd/tax.rs:76`); the TUI uses the same (`tax.rs:90-94`). Because the SE block runs only
  inside the `Computed` arm, and `compute_tax_year` returns `NotComputable(TaxProfileMissing)`
  when the profile is `None`, the `profile` is always `Some` there — the `unwrap_or(Single)`
  fallback is unreachable, so the filing status always equals the CLI's. (See Nit N-4.)
- **Spot-check (verify):** `build_verify(&state, &events, &cli_config)` matches the CLI's
  `inspect.rs:33` call exactly.
- **`NotComputable` emits NO dollar figures** (`tax.rs:50-52` writes only the blocker kind +
  detail); test `tax_tab_not_computable_no_profile_shows_blocker_no_numbers` asserts the LT
  figure `10000.00` does NOT appear.
- **No float:** grep for `as f64`/`as f32`/`1e8`/`f64`/`f32` returns NONE. `sat_to_btc` is exact
  `Decimal::from(sat) / Decimal::from(100_000_000)`; all money is `{:.2}` on `Decimal`, all BTC
  `{:.8}` on `Decimal`.

### 6. Additivity / back-compat — CONFIRMED ✅
- Branch diff touches ONLY: `crates/btctax-tui/**`, root `Cargo.toml` (adds the workspace
  member), `Cargo.lock`, and two docs (`design/SPEC_*`, `reviews/R0-*`). **No change to
  btctax-core / btctax-cli / btctax-store / btctax-adapters source.** New crate = MINOR bump,
  additive only, as specified.
- MSRV `rust-version = "1.74"` (workspace-inherited). `Cargo.lock` is committed and pins
  `ratatui 0.29.0` + `crossterm 0.28.1` (the versions with MSRV exactly 1.74 / 1.63).

### 7. Determinism / synthetic tests / Locked / the q-fix — CONFIRMED ✅
- **Determinism:** display iterates `Vec` (lots/disposals/income) and `BTreeMap` (profiles); no
  `HashMap` iteration order, no wall-clock in rendered output.
- **Synthetic-only tests:** temp vaults (`tempfile`) + synthetic `LedgerState` fixtures; no real
  user data.
- **VaultLock `Locked` handled:** `StoreError::Locked → OpenOutcome::Locked → Screen::Locked`
  (`unlock.rs:97`, `app.rs:177`); tested at both `attempt_open` and `App` level.
- **The `q`-typeable-in-passphrase fix:** `handle_key` dispatches on `screen` FIRST; on
  `Screen::Unlock`, `Char('q')` (and every printable) appends to the buffer — only `Esc` quits
  (`main.rs:118-135`). Regression tests `q_on_unlock_screen_appends_to_buffer_not_quit` +
  `char_input_on_unlock_screen_appends_various_chars_including_q` lock this in. (But the on-screen
  hint contradicts it — see Minor M-1.)

---

## Findings

### MINOR

#### M-1 (NEW) — Unlock-screen hint `Esc/q: quit` is inaccurate and contradicts the q-typeable fix
`draw.rs:56` renders `"Enter: unlock    Esc/q: quit"` on the **Unlock** screen. But by the
deliberate q-typeable fix, pressing `q` on Unlock **types `q` into the passphrase** — it does
NOT quit (only `Esc` does). So the hint actively misinforms the user on the security-sensitive
unlock screen: a user who presses `q` expecting to quit is silently appending a character to
their passphrase (they'd see an extra `●` and a failed unlock). No security leak and no
malfunction — purely a wrong help string — hence **Minor, not Important**. **Fix (one word):**
change the Unlock hint to `Esc: quit`. (The Locked screen at `draw.rs:73` correctly lists
`r retry   q quit` because `q` *does* quit there; only the Unlock hint is wrong.)

### NIT

#### N-1 — Viewer footer advertises `r: refresh` AND `?: help`, neither of which has a handler
`draw.rs:120` footer reads `... r: refresh   q/Esc: quit   ?: help`, but `Screen::Viewer` in
`handle_key` (`main.rs:145-164`) has **no** `Char('r')` and **no** `Char('?')` arm (the only
`Char('r')` handler is on `Screen::Locked`). Both keys are no-ops in the viewer — the footer
over-promises. This is the recorded Nit (a), and `?: help` is the same class. **Triage: DEFER
(non-blocking).** Cheapest correct fix = trim the footer to the keys actually implemented (drop
`r: refresh` and `?: help`); track the `r` re-project and `?` help overlay in `FOLLOWUPS.md`.
Implementing them is feature work and not required to merge. (Note: `PgUp/PgDn` are implemented
but not advertised — harmless, opposite direction.)

#### N-2 — `Snapshot.config: ProjectionConfig` is a dead field
`app.rs:107` stores `config: ProjectionConfig` (the third element of
`load_events_and_project()`), but **no tab ever reads it** — only `cli_config` (CliConfig) is
consumed (by `build_verify`); `events`/`state`/`profiles`/`tables` are all consumed. The
struct-level `#[allow(dead_code)]` (`app.rs:103`, comment "consumed in Task 4") is broader than
true — it masks this one genuinely-dead field. **Fix:** drop the field (a future `r` re-project
would call `load_events_and_project()` again and get a fresh config anyway), or narrow the
allow + document retention. Non-blocking.

#### N-3 — `form_8949` recomputed on scroll keypress (recorded Nit b)
`active_row_count` (`main.rs:216`) calls `form_8949(&state, year)` on each Forms-tab scroll
keypress, in addition to the per-frame call in `forms::draw` (`forms.rs:64`). Pure and
inconsequential (the builder is a cheap pure fold). **Triage: DEFER.**

#### N-4 — Dead helpers / unreachable fallback
(a) `dispose_kind_tag` (`tags.rs:40`, `#[allow(dead_code)]`) is unused — the Disposals tab has
no dispose-kind column (matches the spec's column list), so this is correctly dead; remove or
keep-with-allow. (b) The `unwrap_or(FilingStatus::Single)` in `tax.rs:92` is unreachable (the SE
block runs only when a profile exists). Neither affects output; both are cleanliness Nits.

#### N-5 — M5 `cargo +1.74` MSRV CI gate is absent (no CI infra in the repo)
The substantive MSRV protection is in place: `Cargo.lock` is committed and pins the exact
`ratatui 0.29.0` / `crossterm 0.28.1` (MSRV 1.74 floor). However, there is **no CI configuration
anywhere in the repo** (no `.github/workflows`, no `rust-toolchain`, no `*.yml`), so the
spec's `cargo +1.74 check` gate (R0-M5) has nothing to attach to and is not present. This is an
infra/process item, not crate code — **not a merge blocker for btctax-tui**. Track adding the
MSRV gate (and CI generally) in `FOLLOWUPS.md`; until then, do not bump ratatui past 0.29
(0.30 → MSRV 1.86) without a deliberate workspace decision.

---

## Nit triage (as requested by the brief)

- **(a) Footer `r: refresh` with no `Screen::Viewer` `r` handler → DEFER** (see N-1). Not a
  blocker; cheapest fix is to remove `r: refresh` (and `?: help`) from the footer and track the
  re-project/help-overlay features in FOLLOWUPS. No security/read-only/figure impact.
- **(b) `form_8949` computed twice (per frame + per scroll keypress) → DEFER** (see N-3). Pure,
  inconsequential.
- **(c) MSRV `cargo +1.74` CI gate (M5) → note only** (see N-5). Lock committed + pins correct;
  no CI infra exists in the repo to host the gate. Not a blocker; track in FOLLOWUPS.

---

## Bottom line

`btctax-tui` is **0 Critical / 0 Important → READY TO MERGE.** The read-only guarantee is
airtight (compile-enforced immutable `Session` + clean grep + byte-identical-vault behavioral
test), the passphrase is capped/moved/masked/never-cloned, the terminal is restored on every
exit path (guard + chained panic hook, all idempotent), the crate is fully offline (no network
crate in the lock, no net calls), figures are parity-by-construction with the CLI (verified on
the Tax reconciliation identity, the SE filing-status source, and the verify builder), there is
no float, and the diff is strictly additive (other crates untouched; `Cargo.lock` committed with
the MSRV-safe pins). The one Minor (a wrong `Esc/q: quit` hint on the unlock screen) and five
Nits (footer over-promise, one dead field, a recomputation, dead helpers, and the absent CI MSRV
gate) are all non-blocking — recommend folding M-1 and the footer trim (N-1) opportunistically
and tracking the rest in `FOLLOWUPS.md`.
