# SPEC — btctax-tui: export-from-TUI (form CSVs, read-only viewer extension)

**Source baseline:** `main` @ `4125db3`.
**Review status:** R0 round 1 (`reviews/R0-spec-tui-export-round-1.md`) FOLDED — findings tagged
`[R0-…]` inline; re-review required per §2.
**Goal:** A user-triggered export keybinding (`e`) in the read-only `btctax-tui` Viewer that writes
the four year-scoped form CSVs (`form8949.csv` / `schedule_d.csv` / `form8283.csv` /
`schedule_se.csv`) and **NOTHING ELSE** — to a timestamped owner-only output subdirectory under the
vault's parent dir, after an explicit confirmation modal. Additionally folds in the
**TUI-disclosure-lines** gap (Chunk-A N-1 family): the TUI's condensed SE block is replaced with a
call to the already-`pub` `render_schedule_se`, restoring six missing disclosure/advisory lines.

**SemVer:** additive to existing crates (`btctax-cli` gains one new `pub fn`; `btctax-tui` gains
one new module + new `App` field + keybinding) ⇒ **MINOR** (pre-1.0).

## Hard constraints (load-bearing)

**Re-stated read-only guarantee** (supersedes the original viewer guarantee for this scope):
> "never writes the vault OR any decrypted image of it; writes only named form CSVs on explicit
> user confirmation."

**[R0-N2]** Reading note: the export necessarily also creates ONE containing directory (the
timestamped export subdir shown in the modal). "Writes only named form CSVs" is to be read as
"the export directory and the four named form CSVs, nothing else"; the guarantee sentence above
is kept verbatim as mandated.

- **Never `export_snapshot` from `btctax-tui`.** `cmd::admin::export_snapshot` writes
  `snapshot.sqlite` — a full decrypted vault image. It is CLI-only **forever**. The gate blocks
  `export_snapshot` (and `cmd::`) anywhere in the `btctax-tui` source tree. Structural backstop
  (recorded from R0): the TUI retains neither the `Session` nor the `Passphrase` after unlock
  (`attempt_open` drops both, unlock.rs:93–107) — `export_snapshot` needs a live `Session` and is
  therefore *structurally unreachable* from the export path, independent of the gate.
- **Never `write_csv_exports` from `btctax-tui`.** The existing full-dump writer writes
  `lots.csv`/`disposals.csv`/`removals.csv`/`income.csv` in addition to form CSVs — the TUI writes
  **only** the four form CSVs via the new `write_form_csvs`. The gate blocks
  `write_csv_exports` in `btctax-tui`.
- **0o600 files, 0o700 dir — EXCLUSIVE dir creation [R0-I1].** Every form CSV is opened via
  `fsperms::open_owner_only` (0o600 on Unix). The output directory is created by `export.rs` via a
  **new** `fsperms::mkdir_owner_only_exclusive` (0o700, `recursive(false)`, **fails with
  `AlreadyExists` on a pre-existing dir** — see D2). The tolerant `mkdir_owner_only` is mkdir-p
  (`recursive(true)`, fsperms.rs:73–80): it silently succeeds on an existing dir and never touches
  its permissions, and `open_owner_only` is `O_CREAT|O_WRONLY|O_TRUNC` without `O_EXCL`/`O_NOFOLLOW`
  (fsperms.rs:22–30) — so a pre-created dir (the timestamp is predictable) containing a symlink
  named e.g. `form8949.csv` could truncate ANY user-writable file. Exclusive creation guarantees a
  fresh, empty, user-owned 0o700 dir, closing both the symlink edge and the same-second-clobber
  case. These fsperms calls are permitted **only** inside `export.rs` (the designated write
  module — see D2).
- **PII posture.** The four form CSVs contain the user's tax data (disposal gains/losses, SE
  figures, donation deductions). Owner-only permissions are mandatory on Unix; the confirmation
  modal and any future documentation must state this explicitly.
- **No CSV header comments.** The `write_form_csvs` writer surface is frozen. The §6017 $400 SE
  filing floor (a parallel burndown lane) is text-report-only; it must NOT add CSV header comments
  or new columns. Coordination note for that lane: add any §6017 advisory exclusively to
  `render_schedule_se` (text rendering) — do NOT touch `write_form_csvs` or any CSV writer
  function.
- **Mechanized source gate (inherited + extended + AUTOMATED) [R0-I4].** The existing review-level
  gate (`save(` / `append_` / `cmd::` / `conn(`) is extended with write-class I/O restrictions for
  `btctax-tui` AND mechanized as an in-crate `#[test]` (KAT-E10) that runs on every
  `cargo test`/CI — the crate is changing from provably-write-free to write-capable, and the
  only-in-`export.rs` isolation has no compile-level backstop (`write_form_csvs` is `pub` and
  callable from any module). The whole-diff review grep remains as the independent second layer.
  See D5.

## Current state (recon @ `4125db3`)

- **`crates/btctax-cli/src/render.rs:567`** — `pub fn write_csv_exports(out_dir, state,
  tax_year: Option<i32>, se_result, donation_details)`: the all-years dump + optional form CSVs.
  Calls four **private** form writers:
  - `write_form8949_csv` (`render.rs:898`)
  - `write_schedule_d_csv` (`render.rs:942`)
  - `write_form8283_csv` (`render.rs:812`)
  - `write_schedule_se_csv` (`render.rs:736`) — called only when `se_result` is `Some`.
  There is NO `pub` function that writes only the four form CSVs. **This gap is D1.**
- **`crates/btctax-cli/src/render.rs:1118`** — `pub fn render_schedule_se(year, result,
  gross_se, table_present, schedule_c_expenses, w2_ss_wages, w2_medicare_wages) -> Option<String>`:
  already `pub`; the canonical source of the full SE text block with three-way `None` split.
- **`crates/btctax-cli/src/cmd/admin.rs:45`** — `export_snapshot`: writes `snapshot.sqlite` via
  `session.vault().export_snapshot(out_dir)` then calls `write_csv_exports`. CLI-only forever;
  never called from `btctax-tui`.
- **`crates/btctax-cli/src/cmd/tax.rs:79–106`** — `report_tax_year`: shows the canonical SE-input
  assembly pattern: **the whole assembly is wrapped in `match profile.as_ref() { Some(p) => …,
  None => None }` — no profile ⇒ no SE section at all [R0-I2]**; inside the `Some(p)` arm:
  `gross_se = se_net_income(&state, year)`, `table_opt = tables.table_for(year)`,
  `table_present = table_opt.is_some()`, `se_result = table_opt.and_then(|t| compute_se_tax(...p))`,
  then `render_schedule_se(year, se_result.as_ref(), gross_se, table_present, p.schedule_c_expenses,
  p.w2_ss_wages, p.w2_medicare_wages)`. The SE assembly sits **outside** the income-tax outcome
  match — the CLI shows the SE section even for a `NotComputable` year [R0-I2]. **The TUI must
  mirror this assembly exactly, including the profile gate and the outcome-independent placement.**
- **`crates/btctax-store/src/fsperms.rs:73–80`** — `mkdir_owner_only` is
  `DirBuilder::new().recursive(true)` (**mkdir-p semantics: silently succeeds on an existing dir;
  never touches an existing dir's permissions**); `open_owner_only` (fsperms.rs:22–30) is
  `OpenOptions::create(true).write(true).truncate(true)` — create-or-**truncate**, no
  `O_EXCL`/`O_NOFOLLOW` (follows an existing symlink). **There is no exclusive-create dir helper;
  D2 adds one.** [R0-I1]
- **`crates/btctax-tui/src/unlock.rs:112–120`** — `build_snapshot`: the `Snapshot` already holds
  `events`, `state`, `cli_config`, `profiles` (BTreeMap<i32,TaxProfile>), `tables`
  (BundledTaxTables), `donation_details` (BTreeMap<EventId,DonationDetails>). No new Snapshot
  fields are needed.
- **`crates/btctax-tui/src/tabs/tax.rs:89–126`** — the current SE block: `compute_se_tax` called
  inline via `snap.tables.table_for(year).and_then(...)` (no three-way split; `table_present` is
  never computed). **It DEFAULTS `FilingStatus::Single` / $0 wages / $0 expenses when NO profile
  exists (tax.rs:90–95) — diverging from the CLI's profile gate — and it sits INSIDE the
  `TaxOutcome::Computed` arm (tax.rs:53–126), so NotComputable years show no SE section [R0-I2].**
  The block renders six hand-coded lines. **Six disclosure/advisory elements are absent vs
  `render_schedule_se`** — see D3.
- **`crates/btctax-tui/src/main.rs:145–165`** — `handle_key` Viewer arm: `q`/`Esc` quit
  (**`Esc` on Viewer currently QUITS — main.rs:146; the modal dispatch must therefore precede the
  Viewer arm [R0-M4]**), `Tab`/`BackTab` tab cycle, `↑/↓/j/k` scroll, `PgUp/PgDn`, `g/G`
  top/bottom, `←/→` year change. `e` is currently unbound. No modal state exists in `App`.
- **`crates/btctax-tui/src/app.rs:116–135`** — `App` struct: no export-modal field, no export
  status field.
- **`crates/btctax-tui/Cargo.toml`** — `time = "0.3"` is currently under `[dev-dependencies]`
  ONLY; D2 uses `time::OffsetDateTime` in production types → promote to `[dependencies]` [R0-M1].
- **Stale doc-comments [R0-M3]:** `main.rs:10–11`, `app.rs:3–4`, `unlock.rs:11–13`, `tabs/tax.rs:3`
  all state the original absolute "STRICTLY READ-ONLY … MUST NOT" guarantee, which becomes false
  as written once the binary writes. Task 1 re-scopes them to the new guarantee wording.
- **No CI grep gate exists** (`.github/workflows/ci.yml` has no grep step); the inherited gate is
  review-time-only [R0-I4].

## Design

### D1 — `pub fn write_form_csvs` in `render.rs`

Add one new `pub` function in `crates/btctax-cli/src/render.rs` that writes **only** the four
form-CSV artifacts for a single tax year:

```rust
/// Write the four year-scoped form CSVs for `year` into `out_dir`.
///
/// Creates `out_dir` owner-only (0o700) if absent (tolerant `mkdir_owner_only`, mkdir-p);
/// each CSV is written via `fsperms::open_owner_only` (0o600).  Only `form8949.csv`,
/// `schedule_d.csv`, `form8283.csv`, and — when `se_result` is `Some` — `schedule_se.csv`
/// are written. The all-years dump CSVs (`lots.csv`, `disposals.csv`, `removals.csv`,
/// `income.csv`) are NOT written; `export_snapshot` / `snapshot.sqlite` are NEVER called
/// or written.
///
/// Path containment is the CALLER's job (matching `write_csv_exports` / `export_snapshot`
/// / `backup_key`): callers must pass a freshly-created or trusted directory — this
/// function truncates the four fixed filenames in `out_dir` (`open_owner_only` is
/// create-or-truncate and follows symlinks). The TUI's `export.rs` satisfies this via
/// `mkdir_owner_only_exclusive` (D2).
pub fn write_form_csvs(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
    se_result: Option<&SeTaxResult>,
    donation_details: &BTreeMap<EventId, DonationDetails>,
) -> Result<(), crate::CliError>
```

Implementation: call `fsperms::mkdir_owner_only(out_dir)?` (tolerant — kept for future CLI-style
reuse; the exclusive-create precondition is the TUI caller's responsibility, per the doc-comment
[R0-I1]), then call the four private form writers (`write_form8949_csv`, `write_schedule_d_csv`,
`write_form8283_csv`, and — iff `se_result.is_some()` — `write_schedule_se_csv`). The four private
writers are unchanged.

The function is analogous to the year-scoped block inside `write_csv_exports` (lines 716–728),
extracted as a named `pub` entry point with no all-years side-effects.

**Misuse exposure (R0-rated Low, recorded):** strictly narrower than the already-`pub`
`write_csv_exports` (same caller-supplied `out_dir`, subset of files, no `snapshot.sqlite` path);
fixed four filenames — it cannot target the vault by name; the year param is `i32` (not
`Option`), forcing year-scoping.

### D1b — `fsperms::mkdir_owner_only_exclusive` in `btctax-store` [R0-I1]

Add to `crates/btctax-store/src/fsperms.rs`:

```rust
/// Create `path` with owner-only permissions (0o700 on Unix), FAILING if it already
/// exists (`ErrorKind::AlreadyExists`). NON-recursive: the parent must exist — callers
/// pass a child of an existing directory (the TUI export passes a child of the vault's
/// parent, which always exists). Guarantees the caller receives a FRESH, EMPTY,
/// caller-owned 0o700 directory — the precondition `write_form_csvs` documents.
pub fn mkdir_owner_only_exclusive(path: &Path) -> Result<(), StoreError>
```

Unix: `DirBuilder::new().recursive(false).mode(0o700).create(path)`; non-Unix:
`DirBuilder::new().recursive(false).create(path)` (ACL-inherited). `recursive(false)` is what
makes the create genuinely exclusive — `recursive(true)` (the existing `mkdir_owner_only`)
succeeds silently on an existing dir. This closes: (1) same-second re-export silently truncating
the previous export's files, (2) writing into a pre-created dir the user doesn't own / whose mode
was never forced to 0o700, (3) the pre-created-dir symlink truncation edge (`form8949.csv` →
any user-writable file).

### D2 — `export.rs` module in `btctax-tui`

Add `crates/btctax-tui/src/export.rs`. This is the **only** module in `btctax-tui` permitted to
name `write_form_csvs`, `fsperms`, `open_owner_only`, `mkdir_owner_only`, or
`mkdir_owner_only_exclusive`. All other `btctax-tui` source files remain write-class-I/O-free
(see D5 for the gate; mechanized by KAT-E10 [R0-I4]).

**Export dir computation.** No file-picker exists; the export location is derived deterministically:

```
export_dir = vault_parent / "btctax-export-{YYYYMMDD}-{HHMMSS}Z"
```

where `vault_parent = vault_path.parent().unwrap_or(Path::new("."))` and the timestamp comes from
an injected `export_now: time::OffsetDateTime` parameter (UTC; production callers pass
`OffsetDateTime::now_utc()`; tests pass a fixed value for determinism — matching the existing
`cmd/optimize.rs` + `cmd/reconcile.rs` convention of injecting `now: OffsetDateTime`).
**[R0-N1]** Note on the fallback: for a bare relative filename (`btctax-tui vault.pgp`),
`Path::parent()` returns `Some("")` — not `None` — so the `unwrap_or(".")` arm is nearly dead
code and the effective result is a **cwd-relative** export dir (`"".join(name)` ≡ `./name`).
Behaviourally fine; stated so the implementation doesn't "fix" it into something else.

Timestamp format: `time` crate format string `[year][month][day]-[hour][minute][second]Z`
(e.g. `20251024-143022Z`). The resulting directory name is filesystem-safe, human-readable, and
monotone within a session. **[R0-M1]** `time` must be promoted from `[dev-dependencies]` to
`[dependencies]` in `crates/btctax-tui/Cargo.toml` (production use in `ExportConfirmState` +
the `e` keybinding).

**No-clobber = EXCLUSIVE create [R0-I1].** `export.rs` calls
`fsperms::mkdir_owner_only_exclusive(out_dir)` (D1b) **before** `write_form_csvs`. A pre-existing
dir — same-second re-press OR a pre-created dir at the predictable path — fails with
`AlreadyExists` and **nothing is written** (surfaced via `export_status` as an error; the user
retries a second later). `write_form_csvs`' internal tolerant `mkdir_owner_only` is then a no-op
on the just-created fresh dir. The prior draft claimed `mkdir_owner_only` itself fails on an
existing dir — that was drift (it is mkdir-p); the exclusive helper makes the no-clobber contract
actually true.

**SE-input assembly — PROFILE-GATED [R0-I2].** `export.rs` assembles SE inputs from the
`Snapshot` mirroring `cmd/tax.rs:79–106` exactly, **including the `match profile` wrapper**:

```
profile     = snap.profiles.get(&year)
se_result   = match profile {
    Some(p) => {
        let table_opt = snap.tables.table_for(year);
        table_opt.and_then(|t| compute_se_tax(
            &snap.state, year,
            p.filing_status, t,
            p.w2_ss_wages, p.w2_medicare_wages,
            p.schedule_c_expenses,
        ))
    }
    None => None,   // no profile ⇒ no SE figure ⇒ no schedule_se.csv
}
```

(`gross_se = btctax_core::se_net_income(&snap.state, year)` and
`table_present = table_opt.is_some()` are additionally computed where the D3 rendering needs
them.) No profile ⇒ `se_result = None` ⇒ the export omits `schedule_se.csv` AND the Tax tab
shows no SE section (D3) — tab, export, and CLI report all agree.

**`ExportConfirmState`** (held in `App`):
```rust
pub struct ExportConfirmState {
    pub year: i32,
    pub out_dir: PathBuf,
    /// Files that will be written (derived before the modal opens).
    pub files: Vec<&'static str>,   // e.g. ["form8949.csv", "schedule_d.csv", ...]
    pub export_now: time::OffsetDateTime,   // frozen at modal-open time
}
```

**`pub fn do_export(snap: &Snapshot, state: &ExportConfirmState) -> Result<PathBuf, CliError>`**
calls `fsperms::mkdir_owner_only_exclusive(out_dir)` [R0-I1], assembles the SE result
(profile-gated, above), and calls `btctax_cli::render::write_form_csvs`. Returns the written dir
path on success; on `AlreadyExists` returns the error with nothing written.

**[R0-N3]** Recorded limitation: `ExportConfirmState` is freely constructible, so "the
confirmation modal gates the ONLY call site of `do_export`" is a **procedural** guarantee
(enforced by the KATs + KAT-E10 + whole-diff review), not a type-level proof. Acceptable for
this scope; a future reviewer must not over-read the type as enforcement.

### D3 — Disclosure lines: replace hand-rolled SE block in `tabs/tax.rs`

The existing hand-coded SE block in `render_tax_content` (`tabs/tax.rs:89–126`) is replaced with
a call to the already-`pub` `btctax_cli::render::render_schedule_se`, mirroring the
`cmd/tax.rs:79–106` assembly. This is the single source of truth; replicating logic in the TUI
was the root of the gap.

**Enumerated missing elements** (all absent from the current TUI block, present in
`render_schedule_se`):

1. **Gross breakout line** (`render.rs:1138–1145`, when `schedule_c_expenses > 0`): `"gross
   business income X − Schedule C expenses Y = net SE earnings Z"` — the TUI currently shows
   only `Net SE income: {net_se:.2}` with no expense context.

2. **Chunk-B I3-mechanism advisory** (`render.rs:1146–1156`, when `schedule_c_expenses > 0`):
   `"(Schedule C advisory) Schedule C expenses also reduce your ORDINARY taxable income, but the
   income-tax total above uses GROSS crypto income — to first order it OVERSTATES your tax by your
   marginal ordinary rate applied to ${expenses}. The tax profile cannot express this; coordinate
   on your actual return."` — entirely absent from TUI.

3. **No-expenses note** (`render.rs:1163–1166`, when `schedule_c_expenses == 0`): `"(Schedule C)
   no Schedule C expenses supplied (--schedule-c-expenses)"` — absent from TUI; the TUI shows
   neither the breakout nor this note.

4. **§164(f) advisory** (`render.rs:1199–1211`): `"(§164(f) advisory) The §164(f) deduction (X)
   is NOT auto-coordinated into the income-tax total above — to first order, that total overstates
   your combined tax by your marginal ordinary rate applied to X. The tax profile cannot express
   this deduction directly — coordinate it on your actual return."` — absent from TUI.

5. **W-2 coordination disclosure** (`render.rs:1212–1229`): either `"(W-2 coordination applied)
   SS cap = max(0, wage base − {w2_ss}) (Box 3+7); Additional-Medicare threshold reduced (not
   below 0) by {w2_medicare} (Box 5, §1401(b)(2)(B)/Form 8959 Part II)."` or `"(W-2) assumes
   $0 W-2 wages (set --w2-ss-wages/--w2-medicare-wages on the tax profile if you had a wage
   job)."` — absent from TUI.

6. **Fully-expensed line** (`render.rs:1257–1271`, the `None`-case when `gross_se > 0 &&
   table_present && result == None`): `"fully expensed: gross X − Schedule C expenses Y ≤ $0 →
   no §1401 SE tax for {year}."` — the current TUI simply renders nothing for this case (the
   `if let Some(se) = compute_se_tax(...)` block silently produces no output when
   `compute_se_tax` returns `None`, with no three-way split).

**Replacement design — SE gating PINNED [R0-I2].** In `render_tax_content` (`tabs/tax.rs`),
remove the hand-rolled SE block from inside the `Computed` arm and render the SE section
**after / outside the outcome match** (before the charitable-deduction and advisory-blockers
sections), so `NotComputable` years still show it — matching `report_tax_year`, where the SE
assembly sits outside the outcome match:

```
// AFTER the match on `outcome` (Computed | NotComputable both fall through to here):
se_text = match snap.profiles.get(&year) {
    Some(p) => {
        let gross_se    = se_net_income(&snap.state, year);
        let table_opt   = snap.tables.table_for(year);
        let table_prsnt = table_opt.is_some();
        let se_result   = table_opt.and_then(|t| compute_se_tax(
                              &snap.state, year, p.filing_status, t,
                              p.w2_ss_wages, p.w2_medicare_wages, p.schedule_c_expenses));
        btctax_cli::render::render_schedule_se(
            year, se_result.as_ref(), gross_se, table_prsnt,
            p.schedule_c_expenses, p.w2_ss_wages, p.w2_medicare_wages)
    }
    None => None,   // PROFILE-GATED: no profile ⇒ no SE section (mirrors cmd/tax.rs:79–106)
};
if let Some(text) = se_text { write into s }
```

**Two intentional behaviour changes from today's TUI (both convergences to the CLI report):**
1. **Profile gate.** Today's TUI defaults `FilingStatus::Single` / $0 wages / $0 expenses when
   no profile exists (tax.rs:90–95) and can show a full SE figure for a no-profile year. After
   D3: no profile ⇒ NO SE section — same as the CLI, and consistent with the export (which
   omits `schedule_se.csv` for a no-profile year, D2). Without this pin the tab would show an
   SE figure the export doesn't write — an internal tab-vs-export inconsistency.
2. **Outcome-independent placement.** Today's TUI renders SE only inside `TaxOutcome::Computed`;
   after D3 a `NotComputable` year with a profile + business income still shows the SE section
   (or the "wage base unavailable" note), matching the CLI report.

The condensed one-line formatting (SS / Medicare / Addl on a single line) disappears; the TUI now
shows the same semantic content as the CLI report, wrapped by the `Paragraph { wrap: Wrap { trim:
false } }` already in place. This is acceptable — the Tax tab is a Paragraph with wrap, not a
fixed-column table.

### D4 — Confirmation modal

**`App` additions** (`app.rs`):
- `pub export_modal: Option<ExportConfirmState>` — `Some` when the modal is open.
- `pub export_status: Option<String>` — shown in the footer after a completed export (or error);
  cleared on the next key press.

**Keybinding.** On `Screen::Viewer`, `KeyCode::Char('e')` (currently unbound) opens the modal:
1. If `app.snapshot.is_none()` → no-op (no data to export).
2. Compute `export_now = OffsetDateTime::now_utc()`, derive `out_dir`, compute `files` list
   (always includes `form8949.csv`, `schedule_d.csv`, `form8283.csv`; includes `schedule_se.csv`
   iff `se_result.is_some()` for `selected_year`).
3. Set `app.export_modal = Some(ExportConfirmState { year, out_dir, files, export_now })`.

**Modal rendering** (drawn over the current tab by `draw.rs`, checked before the normal tab
dispatch):
```
╔═ Export form CSVs for {year} ══════════════════════════╗
║  Output directory:                                      ║
║    {out_dir}                                            ║
║                                                         ║
║  Files to write:                                        ║
║    form8949.csv                                         ║
║    schedule_d.csv                                       ║
║    form8283.csv                                         ║
║    schedule_se.csv   ← only when SE result present      ║
║                                                         ║
║  The vault is never written.                            ║
║  Exported files contain your tax data and are           ║
║  owner-only (0o600 on Unix).                            ║
║                                                         ║
║  [Enter] Confirm     [Esc] Cancel — writes nothing      ║
╚═════════════════════════════════════════════════════════╝
```

**Modal keybindings** (while `export_modal.is_some()`, modal keys take priority —
**[R0-M4] the modal dispatch MUST precede the Viewer arm in `handle_key`**: `Esc` on Viewer
currently quits (main.rs:146), so without the ordering pin, Esc-on-modal would quit the app
instead of cancelling the export):
- `Enter` → call `do_export`; on `Ok(dir)` set `app.export_status = Some(format!("Exported to
  {dir}"))`, clear modal; on `Err(e)` set `app.export_status = Some(format!("Export error: {e}"))`,
  clear modal (this includes the `AlreadyExists` exclusive-create failure [R0-I1]).
- `Esc` → clear modal only. **Writes nothing. Does NOT quit** (`should_quit` stays false).
- Any other key (including `q`) → ignored (modal is blocking; `q` does not quit while the modal
  is open).

**Footer.** When `export_status.is_some()`, show the status string in the footer row; on the next
non-modal key press, clear `export_status`.

### D5 — Source gate: extended table + MECHANIZED [R0-I4, R0-M2]

The existing whole-diff review gate (`save(` / `append_` / `cmd::` / `conn(`) is extended. The
full normative gate for `btctax-tui` after this spec:

| Pattern | Rule |
|---|---|
| `save(` | FORBIDDEN everywhere in `btctax-tui` |
| `append_` | FORBIDDEN everywhere in `btctax-tui` |
| `cmd::` | FORBIDDEN in non-test code; `btctax_cli::cmd::init::run` in **test code** is the sole permitted exception (documented exception 1 — fixture setup, already present) |
| `conn(` | FORBIDDEN everywhere in `btctax-tui` |
| `export_snapshot` | FORBIDDEN everywhere in `btctax-tui` |
| `write_csv_exports` | FORBIDDEN everywhere in `btctax-tui` |
| `write_form_csvs` | FORBIDDEN outside `export.rs` (documented exception 2 — the designated writer call site) |
| `open_owner_only` / `mkdir_owner_only` / `mkdir_owner_only_exclusive` / `fsperms` | FORBIDDEN outside `export.rs` |
| `File::create` / `File::options` | FORBIDDEN outside `export.rs` in non-test code |
| `OpenOptions` [R0-M2] | FORBIDDEN outside `export.rs` in non-test code (the obvious `open_owner_only` bypass spelling) |
| `fs::write` / `write_owner_only` [R0-M2] | FORBIDDEN outside `export.rs` in non-test code |
| `create_dir` / `create_dir_all` / `DirBuilder` [R0-M2] | FORBIDDEN outside `export.rs` in non-test code |
| `set_permissions` / `fs::copy` / `fs::rename` / `fs::remove_` [R0-M2] | FORBIDDEN outside `export.rs` in non-test code |

Read-class I/O (`std::fs::read` in the unlock.rs bytes test) remains permitted. Test code
(`#[cfg(test)]` blocks) may use write-class verbs for fixture setup (temp dirs, pre-creating the
collision dir in KAT-E11) — never targeting a vault path.

**Mechanization — KAT-E10 [R0-I4].** The table above is enforced by an in-crate `#[test]`
(~30 lines) in `btctax-tui` that walks `crates/btctax-tui/src/`, scans each file's **non-test
region** (the portion before the file's `#[cfg(test)]` marker — house convention places the tests
module at the end of the file) for the forbidden tokens, applies the two documented exceptions
(`export.rs` for the write-class/`write_form_csvs` rows; test-region code for `cmd::init::run`
and fixture write verbs), and fails with the offending `file:line` on any other hit. Runs on
every `cargo test` and in CI — the isolation invariant is enforced between whole-diff reviews,
not just at them. The whole-diff review grep (Task 2) remains as the independent second layer.

## Plan (TDD)

### Task 1 — `write_form_csvs` + disclosure lines + export module + modal + keybinding

**Files:**
- `crates/btctax-store/src/fsperms.rs` — add `pub fn mkdir_owner_only_exclusive` (D1b) [R0-I1]
- `crates/btctax-cli/src/render.rs` — add `pub fn write_form_csvs` (D1)
- `crates/btctax-tui/Cargo.toml` — promote `time = "0.3"` from `[dev-dependencies]` to
  `[dependencies]` [R0-M1]
- `crates/btctax-tui/src/tabs/tax.rs` — replace hand-rolled SE block with the profile-gated,
  outcome-independent `render_schedule_se` call (D3) + re-scope the module doc-comment [R0-M3]
- `crates/btctax-tui/src/export.rs` — new module: `ExportConfirmState`, `do_export`,
  export-dir computation, exclusive dir creation (D2)
- `crates/btctax-tui/src/app.rs` — add `export_modal: Option<ExportConfirmState>` +
  `export_status: Option<String>` to `App` + re-scope the doc-comment [R0-M3]
- `crates/btctax-tui/src/main.rs` — add `e` keybinding + modal Enter/Esc dispatch (BEFORE the
  Viewer arm [R0-M4]) + `export_status` footer clearing (D4) + re-scope the doc-comment [R0-M3]
- `crates/btctax-tui/src/unlock.rs` — re-scope the doc-comment [R0-M3]
- `crates/btctax-tui/src/draw.rs` — modal rendering (D4)

**[R0-M3] Doc-comment re-scope (load-bearing safety text must not drift):** the absolute
"STRICTLY READ-ONLY … MUST NOT" statements at `main.rs:10–11`, `app.rs:3–4`, `unlock.rs:11–13`,
and `tabs/tax.rs:3` become false as written once the binary writes. Replace with the re-scoped
guarantee wording: "never writes the vault or any decrypted image of it; writes only the four
form CSVs via `export.rs` on explicit user confirmation" (modules other than `export.rs`
additionally keep "this module performs no writes").

**Key-acceptance tests (all must be red pre-implementation, green post):**

**KAT-E1 — Confirmation flow (unit, temp vault).**
Open a synthetic vault with business SE income **and a `TaxProfile` for the year** (the SE
figure is profile-gated [R0-I2]); trigger `handle_key(e)` on Viewer →
`app.export_modal.is_some()` and `modal.files` includes `form8949.csv` and `schedule_se.csv`;
trigger Enter → `app.export_modal.is_none()` and `app.export_status` contains "Exported to";
the output dir exists with the four form CSVs present.

**KAT-E2 — Esc-cancel writes nothing (+ modal-priority asserts [R0-M4]).**
Open a synthetic vault; trigger `handle_key(e)` → modal opens; trigger `Esc` →
`app.export_modal.is_none()`, `app.export_status.is_none()`, **`!app.should_quit`** (Esc closed
the modal — it did NOT fire the Viewer quit arm), **and the output dir does NOT exist** (no
partial writes, no empty dir). Additional case: `q` while the modal is open → ignored
(`!app.should_quit`, modal state per design, nothing written).

**KAT-E3 — Vault bytes byte-identical after an export.**
Extend the existing `vault_file_bytes_unchanged_after_open_build_snapshot_drop` test: after
calling `do_export` (with a real vault + synthetic data), re-read the vault file bytes and assert
they are **byte-identical** to the pre-export bytes. This subsumes the existing open→drop test.

**KAT-E4 — Figure parity via HARD-CODED goldens [R0-I3] (choice: option (a), no gate impact).**
The test must NOT self-assemble its expectation (a mirrored assembly error — e.g. swapped
`w2_ss_wages`/`w2_medicare_wages` — would pass if the test mirrors `export.rs`'s assembly).
Instead: hard-coded, independently-derived golden figures for a pinned fixture in which **both
W-2 values BIND and differ**, so a parameter swap changes the answer.

Fixture: TY2025, Single, mining **$100,000 gross** with `schedule_c_expenses` **$60,000**
(→ net_se $40,000), `w2_ss_wages` **$150,000**, `w2_medicare_wages` **$170,000**
(w2_ss ≠ w2_medicare; SS wage base $176,100, Addl threshold Single $200,000 — both reduced
caps bind). Hand-verified goldens (assert EXACT against the `schedule_se.csv` Decimal strings):
- `net_se_earnings` = **40000** (the expensed net; the CSV column carries the expensed net per
  Chunk B R0-M3)
- base = 40,000 × 0.9235 = **$36,940.00**
- ss = 12.4% × min(36,940, 176,100 − 150,000 = 26,100) = **$3,236.40** (cap BINDS — cross-checked
  against the identical figure in SPEC_se_chunkB_expenses.md's combined golden)
- medicare = 2.9% × 36,940 = **$1,071.26**
- addl = 0.9% × max(0, 36,940 − (200,000 − 170,000 = 30,000)) = 0.9% × 6,940 = **$62.46**
  (threshold BINDS)
- `total_se_tax` = 3,236.40 + 1,071.26 + 62.46 = **$4,370.12**
- `deductible_half` = (3,236.40 + 1,071.26) / 2 = **$2,153.83** (excludes addl — the C1 rule)

Swap-catching check (documented in the test): swapping the W-2 params gives ss cap 6,100 →
ss $756.40 and addl threshold 50,000 → addl $0 — different figures, so the golden fails on a swap.

Parity for `form8949`/`schedule_d`/`form8283` is by construction (same private writers, same
`state`); the assembly-sensitive artifacts are the SE CSV (covered by the goldens) and the
`donation_details` passthrough — add one assertion that a fixture donation's known `donee` label
appears in the exported `form8283.csv`.

**KAT-E5 — 0o600 file permission asserts (Unix only, `#[cfg(unix)]`).**
After `do_export` on a fixture vault: for each written CSV file, assert
`fs::metadata(file).permissions().mode() & 0o777 == 0o600`. For the output dir: assert
`mode & 0o777 == 0o700`.

**KAT-E6 — Timestamped dir uniqueness / determinism.**
Call `export_dir_for(vault_path, export_now)` (a testable pure helper extracted from `export.rs`)
with a fixed `OffsetDateTime` (e.g., 2025-10-24 14:30:22 UTC) and assert the returned path ends
with `btctax-export-20251024-143022Z`. Call with a different fixed timestamp → different suffix.

**KAT-E7 — Disclosure-line KATs (Tax tab, `TestBackend`) + SE-gating KATs [R0-I2].**
Fixture: TY2025, mining $50,000, a synthetic `TaxProfile` in `snap.profiles` for 2025 with
`schedule_c_expenses = $5,000`, `w2_ss_wages = $30,000`. Render `render_tax_content(snap, 2025)`
and assert:
- (a) the gross breakout line is present: `"gross business income"` and `"Schedule C expenses"`
  and `"net SE earnings"` all appear.
- (b) the I3-mechanism advisory text is present: `"ORDINARY taxable income"` and `"OVERSTATES"`
  and `"coordinate"` all appear.
- (c) the §164(f) advisory is present: `"§164(f)"` and `"NOT auto-coordinated"` appear.
- (d) the W-2 disclosure is present: `"W-2 coordination applied"` and `"Box 3+7"` appear.
- Fixture with `schedule_c_expenses = $0`: the `"no Schedule C expenses supplied"` line is
  present; the breakout and I3 advisory are absent.
- Fixture with `gross_se = $10,000`, `schedule_c_expenses = $15,000` (fully expensed, `None`
  from `compute_se_tax`): `"fully expensed"` and `"no §1401 SE tax"` appear; `"SS wage base
  unavailable"` does NOT appear (negative assertion — three-way split is correct).
- **[R0-I2] Profile gate:** business income + table present + **NO profile** for the year →
  NO SE section in `render_tax_content` (no `"Schedule SE"` substring) — the intentional change
  from today's Single/$0 default.
- **[R0-I2] Outcome-independent placement:** a **NotComputable** year (fixture with a hard
  blocker) + profile + business income → the SE section IS present in the tab output (matching
  the CLI report).

**KAT-E8 — `e` on Viewer with no snapshot is a no-op.**
`handle_key(e)` when `app.snapshot.is_none()` → `app.export_modal.is_none()`.

**KAT-E9 — `schedule_se.csv` absent when SE result is absent [R0-I2 extended].**
(a) `do_export` on a vault with no business income for the selected year → the `files` list does
NOT include `schedule_se.csv` and `out_dir/schedule_se.csv` does not exist. (b) `do_export` on a
vault WITH business income but **no `TaxProfile`** for the year → same: no `schedule_se.csv`
(profile-gated, mirroring `cmd/tax.rs`/`cmd/admin.rs`) — and per the KAT-E7 profile-gate case the
tab shows no SE section either, so tab and export agree.

**KAT-E10 — Mechanized source gate [R0-I4].**
The `#[test]` specced in D5: walks `crates/btctax-tui/src/`, scans each file's non-test region
for every forbidden token in the D5 table, applies the two documented exceptions (write-class +
`write_form_csvs` tokens in `export.rs`; `cmd::init::run` + fixture write verbs in test regions),
and fails with `file:line` on any other hit. Self-check assertions: the test must FAIL if a
forbidden token is planted in a temp copy of a non-export module (test the tester once, in-line),
and must currently PASS on the real tree.

**KAT-E11 — Pre-created export dir → error, NOTHING written [R0-I1].**
Using the injected deterministic timestamp (KAT-E6), pre-create the exact
`btctax-export-{ts}` dir (with a sentinel file inside) before calling `do_export` →
`do_export` returns `Err` (`AlreadyExists` from `mkdir_owner_only_exclusive`); assert NO form
CSVs exist in the pre-created dir, and the sentinel file's contents are untouched (no truncation
— the symlink/pre-created-dir edge is closed).

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS

Cross-cutting checks:

- **Read-only guarantee (extended):** run the FULL D5 table as the whole-diff grep (independent
  second layer over KAT-E10): `save(` / `append_` / `conn(` / `export_snapshot` /
  `write_csv_exports` → zero hits outside allowed exceptions; the write-class rows
  (`open_owner_only` / `mkdir_owner_only*` / `fsperms` / `File::create` / `File::options` /
  `OpenOptions` / `fs::write` / `create_dir*` / `DirBuilder` / `set_permissions` / `fs::copy` /
  `fs::rename` / `fs::remove_`) appear ONLY in `export.rs` (non-test) [R0-M2].
- **KAT-E10 present and green:** the mechanized gate test exists, covers the full table, and
  passes [R0-I4].
- **`write_form_csvs` isolation:** grep `btctax-tui` for `write_form_csvs` → appears ONLY in
  `export.rs`.
- **`write_form_csvs` correctness:** does NOT call the all-years writers (`lots.csv` /
  `disposals.csv` / `removals.csv` / `income.csv`); does NOT call `export_snapshot`; carries the
  path-containment doc-comment (caller's job — D1).
- **Exclusive create [R0-I1]:** `do_export` calls `mkdir_owner_only_exclusive` (not the tolerant
  `mkdir_owner_only`) BEFORE `write_form_csvs`; `mkdir_owner_only_exclusive` is `recursive(false)`
  + 0o700; KAT-E11 passes.
- **SE assembly parity [R0-I2]:** the `export.rs` SE-input assembly is identical in logic to
  `cmd/tax.rs:79–106` **including the `match profile` wrapper**; the `tabs/tax.rs` assembly
  mirrors it AND renders outside the outcome match (NotComputable years show SE).
- **Three-way `None` split:** `render_tax_content` now correctly handles (1) no gross SE → no
  section; (2) gross SE > 0, no table → "SS wage base unavailable" note; (3) fully expensed →
  "fully expensed" line. Verify by inspection that `render_schedule_se` is called with the
  correct inputs.
- **Esc-cancel writes nothing + modal priority [R0-M4]:** KAT-E2 passes; modal dispatch precedes
  the Viewer arm; no dir creation on the cancel path; `Esc`/`q` on the modal do not quit.
- **Determinism:** `export_now` is injected everywhere; `OffsetDateTime::now_utc()` appears only
  in the production call site, not in test paths.
- **Golden parity [R0-I3]:** KAT-E4 asserts the hard-coded figures (not a self-assembled
  expectation); the fixture's W-2 values bind and differ.
- **0o600 / 0o700:** KAT-E5 passes.
- **`time` dependency [R0-M1]:** `time` is in `[dependencies]` (pinned explicit version — no
  workspace.dependencies table exists).
- **Doc-comments re-scoped [R0-M3]:** the four "STRICTLY READ-ONLY" sites carry the re-scoped
  guarantee wording; no stale absolute claim remains.
- **No CSV header comments:** `write_form_csvs` and the four private writers are unchanged in
  header/column content; NO NEW comment lines introduced (`form8283.csv`'s pre-existing `#`
  comment lines are unchanged — the freeze is on *additions*); no §6017 content anywhere in the
  CSV surface.
- **Vault bytes test:** KAT-E3 passes (vault bytes identical before vs after a full export cycle).
- **Regression:** all pre-existing tests pass; `write_csv_exports` is unchanged.

FOLLOWUPS after this spec:
- §6017 $400 SE filing floor advisory (text-report only; this spec's note in Hard Constraints
  pins the non-CSV scope).
- The mutating TUI flows (import / reconcile / classify / config / optimize-accept / attest) —
  a future interactive-TUI or egui GUI; explicitly out of scope here.
- PDF / FDF / XFDF form fill — the queue's 5a item.
- Friendlier same-second re-export UX (map the `AlreadyExists` error from
  `mkdir_owner_only_exclusive` to a "wait a second and retry" message rather than the raw error
  string) — the exclusive-create error [R0-I1] is correct and safe as-is; message polish only.
- Type-enforcing the modal→`do_export` gating (a sealed confirmation token) — currently
  procedural per [R0-N3]; revisit if the TUI grows more write actions.

## Out of scope

- `export_snapshot` from `btctax-tui` — never; this is a hard constraint, not a deferral.
- The all-years dump CSVs (`lots.csv` / `disposals.csv` / `removals.csv` / `income.csv`) from
  the TUI — this spec writes **only** the four form CSVs.
- PDF output, FDF/XFDF form fill.
- A file-picker or custom output-path prompt — the vault-parent default is sufficient and
  predictable for the MVP.
- The mutating TUI (import / reconcile / classify / config / optimize-accept / attest).
- Engine-B `crypto_ord` coordination (deferred; disclosed by the I3-mechanism advisory now
  visible in the TUI Tax tab after D3).
- §6017 $400 SE floor in the TUI or CSVs — text-report-only in a parallel burndown lane.
- Multi-vault / concurrent-open export.
