# R0 — SPEC_binary_documentation.md — round 1 (independent architect)

**Artifact:** `design/SPEC_binary_documentation.md`
**Branch:** `feat/binary-docs` @ `1dfaca2` · main @ `13b3b17`
**Scope:** read-only spec review vs CURRENT source. No implementation, no branch switch.
**Bar:** 0 Critical / 0 Important.

## Verdict: **1 Critical / 2 Important / 4 Minor / 2 Nit — BLOCKED (fold before Plan)**

The headline: the two questions that matter split. **#2 (file-list completeness) is GREEN** — the
enumerated set of file/structured-format args is complete. **#1b (clap_mangen renders the long-help into
the man page) is FALSE as specified** — this is the Critical, and it invalidates the load-bearing
single-source claim for the man-page half.

---

### [C1] CRITICAL — clap_mangen's single top-level render does NOT emit subcommand-arg long-help; the file-format examples never reach `btctax.1`

**Spec location:** "Architecture — single source of truth" (lines 16-25); the `btctax.1 [HYBRID]` bullet —
"a new workspace generator … builds the clap `Command` (`Cli::command()`), renders it via **`clap_mangen`**
(all ~33 subcommands + the file-format long-help)"; Gotchas (lines 104-107): "file-format docs live in the
clap long-help ONLY; the man page inherits them via clap_mangen." KAT `file_format_examples_present_in_manpage`
(line 88-89).

**Source evidence:** every file-format arg the spec targets lives on a **subcommand** (or a *grand*child of
`reconcile`), not on the root `Cli`:
- `init --key-backup` — `Command::Init.key_backup` (`crates/btctax-cli/src/main.rs:28-30`)
- `backup-key --out` — `Command::BackupKey.out` (`main.rs:86-88`)
- `export-snapshot --out` — `Command::ExportSnapshot.out` (`main.rs:77-79`)
- `reconcile import-selections <csv>` — `Reconcile::ImportSelections.csv` (`main.rs:296-297`)
- `reconcile classify-raw --payload-json` — `Reconcile::ClassifyRaw.payload_json` (`main.rs:271-276`)
- `reconcile select-lots --from` — `Reconcile::SelectLots.from` (`main.rs:290-295`)

`clap_mangen::Man::new(cmd).render(..)` renders **one** command: its NAME/SYNOPSIS/DESCRIPTION, its **own**
args' help, and a SUBCOMMANDS section that is a *name → short-about* definition list. It does **not** recurse
into subcommands to render their args, and it renders **neither** the short nor the long help of a
subcommand's args in the parent page. So `Man::new(Cli::command()).render()` → a single `docs/man/btctax.1`
contains the subcommand *names* but **zero** of the six file-format examples. The armor block, the CSV
header, the `{"Acquire":…}` JSON, and the `origin#split:sat` spec are absent from the man page. The
requirement (lineage, lines 6-8: examples "inline in the man page AND in the binary's `--help`") is not met,
and `file_format_examples_present_in_manpage` fails by construction. The `--help` half is fine (clap shows a
subcommand's arg long-help under `btctax reconcile import-selections --help`); it is the **man-page half of
the single-source claim that is broken**.

Note this is *not* "clap_mangen omits long-help" (#1b's stated failure mode) — clap_mangen *does* render
`get_long_help()` for the args of the command it is handed. The break is that a single root render never
reaches subcommand args at all. The drift-guard KAT `manpage_covers_every_subcommand` (line 82-83) passes
*trivially* on subcommand names and therefore **masks** this gap — only `file_format_examples_present` would
catch it, and Task 2 (line 96-98) asserts that KAT passes off a plain root render, which is self-contradictory.

**Concrete fix (pick one, and re-plan Task 2 + the KATs + the committed-file set around it):**
1. **Per-subcommand pages via `clap_mangen::generate_to(Cli::command(), out_dir)`** — recursively emits one
   `.1` per (sub)command (`btctax-init.1`, `btctax-reconcile-import-selections.1`, …). The file-format
   long-help then lands in each leaf page and `man btctax-reconcile-import-selections` shows it. Cost: the
   committed artifact is now a *tree* of `.1` files (not a single `docs/man/btctax.1`); the FILES/EXAMPLES
   stitching and every man-page KAT must target the right per-command page. Update "Locations" (line 31-33)
   and `manpage_covers_every_subcommand` accordingly.
2. **Keep one `btctax.1` but recurse manually** — the generator walks `Cli::command().get_subcommands()`
   (recursively, including the nested `reconcile` tree) and for each renders its own `Man`/OPTIONS block,
   appending into the single roff. This is real generation logic *beyond* `Man::render` on the root; the spec
   must say so rather than implying clap_mangen produces the whole page.

Either way, the sentence "renders it via clap_mangen (all ~33 subcommands + the file-format long-help)" and
"one placement, both outputs, zero drift" overstate what a root `Man::render` does and must be corrected.

---

### [I1] IMPORTANT — multi-line examples in `///` doc-comments are reflowed unless `verbatim_doc_comment` is set (mangles the armor block / CSV / JSON in both `--help` and the man page)

**Spec location:** "The inline file-format docs" (lines 35-62) — the PGP armor 3-line block, the 2-line
import-selections CSV example, the JSON, all placed in the clap `///` long-help. Gotchas mention determinism
(108-110) but never `verbatim_doc_comment`.

**Source evidence:** clap's derive, by default, treats a doc-comment as prose — it strips per-line leading
whitespace and hard-wraps to terminal width, collapsing a pre-formatted block. Tellingly, the codebase
**already hit this**: `TaxProfile.magi_excluding_crypto` abandons the `///` for an explicit
`#[arg(long_help = "…\n\n…")]` string precisely to control paragraphing
(`crates/btctax-cli/src/main.rs:111-119`). clap_mangen renders the *processed* `get_long_help()`, so the
mangling propagates to the man page too.
- Single-token examples survive (no interior whitespace to wrap): `disposal_ref,origin_event_id,split_sequence,sat`
  and `origin#split:sat` — so `help_documents_import_selections` / `..select_lots` (lines 77-80) likely pass.
- The **PGP armor block** (`-----BEGIN PGP PRIVATE KEY BLOCK-----` … 3 lines) and the two-line CSV example
  reflow/collapse; `file_format_examples_present_in_manpage` and the readability of `--help` degrade or fail
  depending on tokenization.

**Concrete fix:** on each arg carrying a multi-line example, set `#[arg(verbatim_doc_comment)]` (or use an
explicit `long_help = "…\n…"` string as `magi_excluding_crypto` does), and put a blank `///` line between the
one-line summary and the FORMAT/EXAMPLE block so `-h` stays terse while `--help` shows the block. Verify
clap_mangen emits the block with roff line breaks (`.br`/no-fill) and add this to the Determinism/Gotchas
list. Pin a KAT that asserts a *multi-line* example (the armor BEGIN and END lines both present on their own
lines) survives into `btctax.1`.

---

### [I2] IMPORTANT — the `export-snapshot` format is stated WRONG: the file is `income.csv` (not `income_recognized.csv`) and the enumerated CSV set omits `lots.csv`, `form8283.csv`, `schedule_se.csv`

**Spec location:** lines 44-48 — "projection CSVs (`disposals.csv`, `removals.csv`, `income_recognized.csv`;
with `--tax-year`: `form8949.csv`, `schedule_d.csv`)". The `[R0/impl: read the exact headers …]` hedge
covers *headers*, not the file set/names.

**Source evidence** — `write_csv_exports` (the writer `export_snapshot` actually calls,
`crates/btctax-cli/src/cmd/admin.rs:45-85` → `crates/btctax-cli/src/render.rs:568-732`) writes:
- `lots.csv` (`render.rs:577`) — **omitted by the spec**
- `disposals.csv` (`render.rs:602`)
- `removals.csv` (`render.rs:643`)
- `income.csv` (`render.rs:694`) — the spec's `income_recognized.csv` is the **wrong filename**
- when `--tax-year`: `form8949.csv` (`render.rs:718`), `schedule_d.csv` (`render.rs:719`),
  `form8283.csv` (`render.rs:720`, always), and `schedule_se.csv` (`render.rs:727-729`, when an
  `SeTaxResult` exists) — **`form8283.csv` and `schedule_se.csv` omitted by the spec**
- plus `snapshot.sqlite` (`admin.rs:52`)

Caution for impl: the code's *own* doc-comments also misname it — `main.rs:307` says
"`income_recognized.csv`". So "read the real formats from source" must mean the **writer** at
`render.rs:694`, not the doc-comments (`state.income_recognized` is the in-memory `Vec` name, not the file).

**Concrete fix:** correct the FILES/example enumeration to `lots.csv, disposals.csv, removals.csv,
income.csv` (all-years) + `form8949.csv, schedule_d.csv, form8283.csv, schedule_se.csv` (with `--tax-year`;
last two conditional) + `snapshot.sqlite`. Keep the "read exact headers from the writer" hedge for the
column rows.

---

### [M1] MINOR — the TUI binaries have NO clap surface; "thin clap surface / clap_mangen stub" rationale is factually wrong (conclusion still holds)

**Spec location:** lines 26-29 — "interactive apps with a thin clap surface (`--vault`, passphrase prompt) —
clap_mangen gives only a stub, so these are hand-authored."

**Source evidence:** neither TUI crate depends on clap (`grep clap crates/btctax-tui/Cargo.toml
crates/btctax-tui-edit/Cargo.toml` → none); both parse args by hand via `std::env::args()` —
`parse_vault_path()` at `crates/btctax-tui-edit/src/main.rs:69-88`, and `btctax-tui/src/main.rs` has a bare
`fn main() -> std::io::Result<()>`. There is no `Command` for clap_mangen to render, not even a stub. The
*decision* (hand-author) is correct; the *reason* is wrong.

**Fix:** reword to "the TUI binaries do not use clap (manual `env::args` parsing), so there is no `Command`
to feed clap_mangen — the man pages are fully hand-authored roff." No plan change.

---

### [M2] MINOR — the editor keymap is styled `ratatui` `Line`s, not a reusable text constant; "reuse verbatim — single source with the overlay" is really a hand-copy (drift risk)

**Spec location:** line 29 — "The editor's keymap is the `?`-overlay content already in `draw_help_overlay`
(reuse verbatim — single source with the overlay)."

**Source evidence:** `draw_help_overlay` (`crates/btctax-tui-edit/src/draw_edit.rs:1697-1738`) constructs a
`vec![Line::from(Span::styled(...)), …]` — styled ratatui widgets, not a plain-text block. It cannot be
`include!`-ed or rendered into roff verbatim; the man page will be a **copy**, and the two can drift.
`manpages_have_required_sections` only asserts `?`, `V`, `O` appear (line 87) — it would not catch drift in
the other ~15 keybindings.

**Fix:** either (a) extract the keymap rows to a shared `const KEYMAP: &[(&str,&str)]` / `&str` that *both*
the overlay and the man-page generator consume (true single source), or (b) drop the "single source" wording,
acknowledge it is a maintained copy, and strengthen the KAT to assert *every* editor action key from a shared
list appears in `btctax-tui-edit.1` (mirror of the help-overlay-completeness pattern the spec cites).

---

### [M3] MINOR — a `[[bin]]` generator cannot use a "build/dev dependency"; clap_mangen would be a crate-wide dep of `btctax-cli`

**Spec location:** lines 22, 64-67, 114 — "a `[[bin]]` `gen-docs` in `btctax-cli`, or `crates/xtask`" with
"clap_mangen … as a **build/dev dependency** of the generator only — NOT a runtime dep."

**Technical issue:** cargo `dev-dependencies` are available only to tests/examples/benches, **not** to
`[[bin]]` targets; `build-dependencies` are for `build.rs` only. So a `gen-docs` **`[[bin]]`** in `btctax-cli`
must take clap_mangen as a **normal** (`[dependencies]`) entry, which is then compiled for *every*
`cargo build -p btctax-cli`. (It is never *linked* into the shipped `btctax` binary regardless — cargo
doesn't link unreferenced deps — so the "lean binary" goal holds automatically; the wording is what's wrong.)

**Fix:** choose one and state it precisely — (a) a separate `crates/xtask` crate with clap_mangen as a normal
dep (cleanest isolation; nothing added to `btctax-cli`), or (b) an **optional** dep + `#[cfg(feature =
"gen-docs")]` bin (`clap_mangen = { version = "0.2", optional = true }`, built only under `--features
gen-docs`). Drop "build/dev dependency of the `[[bin]]`."

---

### [M4] MINOR — the `help_documents_*` KATs must render each **subcommand's** `Command` long-help, not the root's

**Spec location:** KATs, lines 77-80 — "`Cli::command()`'s rendered long-help for each command CONTAINS its
format token."

**Source evidence:** the format args are on subcommands (see C1). `Cli::command()` renders the root; the
KAT must navigate, e.g. `Cli::command().find_subcommand("reconcile").unwrap()
.find_subcommand("import-selections").unwrap()` and render *that* command's help (or the arg's
`get_long_help()`). `Cli::command()` itself is available — `#[derive(Parser)]` supplies `CommandFactory`
(confirmed usable: `Cli::try_parse_from` in `main.rs:1991`), so #1c is GREEN — but the KAT wording implies a
root render and needs the traversal spelled out.

**Fix:** state the tree-navigation in each KAT; note that `manpage_covers_every_subcommand` (name-level)
does **not** substitute for `file_format_examples_present` (arg-level) — keep both.

---

### [N1] NIT — do not add `#[command(version)]`, or the man page embeds the crate version and the committed `.1` churns per release

`Cli` currently has no `version` attribute (`crates/btctax-cli/src/main.rs:16`), so clap_mangen emits no
version and `gen_docs_is_deterministic` (line 84-85) is safe run-to-run. But the KAT's *stale-check* half
("committed `.1` match a fresh generation") would then fail on every unrelated version bump if a `version`
is later added. Recommend pinning `Man::date(..)` to an empty/fixed string and explicitly noting "no
`#[command(version)]`" in the Determinism gotcha.

### [N2] NIT (verified-correct, no change needed) — the `classify-raw` JSON example is accurate

`classify_raw` deserializes into `EventPayload` (`crates/btctax-cli/src/cmd/reconcile.rs:174`); the enum is
default externally-tagged (`event.rs:287-320`), so `{"Acquire":{…}}` is right; `Acquire { sat: Sat,
usd_cost: Usd, fee_usd: Usd, basis_source: BasisSource }` (`event.rs:49-54`) matches the field set; `Sat =
i64` (`conventions.rs:6`) → `"sat":2000000` as a number; `BasisSource::ExchangeProvided` → `"ExchangeProvided"`
(`event.rs:17-18`). `Usd = Decimal` (`conventions.rs:8`) string-encoding is the only open point and is
already covered by the spec's "[impl: verify the exact serde shape.]" hedge — acceptable.

---

## Question-by-question résumé

- **#1a (clap `///` short/long split):** ACCURATE for clap 4.6.1 — but only once the *summary/blank-line/block*
  and `verbatim_doc_comment` details of **I1** are handled.
- **#1b (clap_mangen renders long-help into the man page):** **FALSE as specified → C1.** clap_mangen renders
  long-help for the command it's given, but a single root `Man::render` never touches subcommand args, so the
  file-format examples never reach `btctax.1`.
- **#1c (`Cli::command()` available):** GREEN — `CommandFactory` from `#[derive(Parser)]`.
- **#2 (file list COMPLETE):** **GREEN.** Walking `Command`, `Optimize`, `Reconcile` (main.rs:25-523): the
  only args taking a file/PathBuf or a structured (JSON/CSV/pick) format — excluding the global `--vault`
  (`main.rs:19-20`) and the exchange-import `Import { files }` (`main.rs:36`) — are exactly the spec's six.
  Everything else is scalars: `tax-profile`/`report` USD-decimal strings, `config` value-enums + a date,
  `set-donation-details` plain strings, `optimize accept --attest` free-text, event-ref/wallet strings,
  bulk-* date/kind strings. No omissions.
- **#3 (formats correct):** `backup_key` armored S2K owner-only ✓ (`vault.rs:176-190`); import-selections
  header `disposal_ref,origin_event_id,split_sequence,sat` ✓ (`reconcile.rs:756`); `parse_lot_pick` =
  `<event-id>#<split>:<sat>` ✓ (`eventref.rs:107-121`); classify-raw JSON ✓ (N2). **export-snapshot WRONG →
  I2.**
- **#4 (clap_mangen dep):** `clap_mangen 0.2` is compatible with clap 4.6.1 (both off clap's `Command`);
  determinism sound given no version/date (N1). Dep *placement* wording is wrong → M3.
- **#5 (TUI pages):** hand-authored is correct, but the "thin clap surface / stub" rationale is wrong (M1) and
  the overlay "single source" is a copy (M2).
- **#6 (PDF / KATs / SemVer):** `groff -man -T pdf` present (`/usr/bin/groff`). KAT set is reasonable but
  `file_format_examples_present` fails under the current mechanism (C1) and `manpage_covers_every_subcommand`
  masks it (C1); help KATs need tree-navigation (M4); add a multi-line-example KAT (I1). **SemVer PATCH claim
  is GREEN** — additive `///` long-help + a new `docs/` tree + a generator, no flag/enum-name change, no GUI
  `schema_mirror` impact, clap_mangen never linked into the shipped binaries.

## Required before Plan
Fold **C1** (choose per-subcommand `generate_to` or manual recursion; re-plan Task 2 + KATs + committed-file
layout), **I1** (`verbatim_doc_comment` + summary separator), and **I2** (correct the export CSV set/names).
Then re-review (round 2) to confirm 0C/0I.
