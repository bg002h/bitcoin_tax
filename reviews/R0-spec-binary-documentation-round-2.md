# R0 — SPEC_binary_documentation.md — round 2 (independent architect)

**Artifact:** `design/SPEC_binary_documentation.md`
**Branch:** `feat/binary-docs` @ `7387b17` · main @ `13b3b17`
**Scope:** read-only round-2 verification that round-1's 1C/2I/4M/2N folds are resolved. No implementation, no branch switch.
**Bar:** 0 Critical / 0 Important.

## Verdict: **0 Critical / 0 Important / 4 Minor / 2 Nit — R0-GREEN**

The blocker (**C1**) and both Important findings (**I1**, **I2**) are substantively resolved, and the
load-bearing mechanism (per-subcommand `clap_mangen::Man::new(subcmd)` rendering the subcommand's own
args' long-help) is technically correct. All four Minors and both Nits are folded at the level of intent;
the residuals below are non-blocking wording/propagation tidy-ups the author may address at Plan time.
Cleared to proceed to Plan.

---

## Per-finding verification

### [C1] — per-subcommand man pages — **RESOLVED**

The fix is correct and complete at the mechanism level.

- **Layout folded (git-style, per-subcommand).** Lines 24–33 now specify: `clap_mangen::Man::new(cmd).render()`
  renders only ONE command's own args + a subcommand *name→about* list, NOT subcommand ARGS from the root;
  therefore generate one page per command by RECURSING `Command::get_subcommands()` — `docs/man/btctax.1`
  (root) + `docs/man/btctax-<sub>[-<subsub>].1` per node (e.g. `btctax-reconcile-import-selections.1`). The
  Gotcha (132–135) and clap_mangen-integration (84–89) restate it consistently. **A single-`btctax.1`
  design is explicitly called wrong (line 135).**
- **Mechanism confirmed correct.** `clap_mangen::Man::new(subcmd)` renders the OPTIONS block for that
  command's *own* args, including each arg's `get_long_help()`, and does not recurse into sub-subcommand
  args — which is exactly why one `Man` per node is required and why the long-help lands on ITS page. The
  6 target args all sit on subcommands: `init`/`backup-key`/`export-snapshot` are direct children of the
  root `Command` enum (`main.rs:28/86/77`), and `import-selections`/`classify-raw`/`select-lots` are
  grandchildren under `reconcile` (`Reconcile::ClassifyRaw` main.rs:272, `SelectLots` 291, `ImportSelections
  { csv }` 297). So per-subcommand rendering places each file-format long-help on the right leaf page.
- **KATs now coherent with the layout:**
  - `file_format_examples_present_in_manpage` (105–107) checks the **subcommand** page (e.g.
    `btctax-reconcile-import-selections.1` contains the CSV header; `btctax-init.1` the armor block) — the
    exact gap C1 flagged. ✓
  - `manpage_covers_every_subcommand` (102–104) recurses `get_subcommands()` and asserts each subcommand
    has a committed page. ✓ (No longer masks C1, because `file_format_examples_present` now targets the
    subcommand page rather than a root render.)
  - `help_documents_*` (98–101) navigate to the SUBCOMMAND `Command` via `find_subcommand(...)` recursing
    into `reconcile`, render ITS long-help — and the spec explicitly notes the root long-help does NOT carry
    subcommand args (the C1 lesson). ✓
- **Residual single-`btctax.1` assumptions:** the *architecture* (line 28) is clean. Two *propagation*
  leftovers remain in the summary bullets → **Minor 1** and **Minor 2** below.

### [I1] — `verbatim_doc_comment` — **RESOLVED** (one citation imprecision → Nit 1)

- The spec now REQUIRES `#[arg(verbatim_doc_comment)]` (or `#[command(verbatim_doc_comment)]`) on the 6
  file-format args (lines 51–54, 118–119, Gotcha 137) so the armor block / 2-line CSV / JSON are not
  reflowed. The attribute is a real clap-4 derive attribute with exactly this effect, and the spec notes the
  verbatim text flows to clap_mangen too. Mechanism correct. ✓
- **Citation imprecision (Nit 1):** the spec says "the codebase already uses this workaround at
  main.rs:111-119" / "Mirror the existing use at main.rs:111-119." But 111–119 is the `long_help = "…\n\n…"`
  *string* variant (the ALTERNATIVE reflow-control mechanism), **not** `verbatim_doc_comment` — `grep -rn
  verbatim_doc_comment crates/btctax-cli/src` returns zero current uses. Because the spec offers both
  mechanisms and both are valid (an implementer who literally "mirrors 111-119" would write a correct
  `long_help` string), the substance is unaffected; only the "already uses *this* workaround" phrasing
  conflates the two. Reword to "mirror the reflow-control approach at 111-119 (which uses the `long_help`
  string variant of the same fix)."
- **Partial residual (Minor 4):** round-1's I1 fix also asked to (a) verify clap_mangen emits roff line
  breaks (`.br`/no-fill) for the block and (b) pin a KAT that a *multi-line* example survives on separate
  lines. The spec requires verbatim (the primary fix, done) and asserts "flows to clap_mangen too," but adds
  no dedicated line-separation KAT — `file_format_examples_present_in_manpage` checks the block is *present*,
  not that BEGIN/END land on their own lines. Non-blocking (the mechanism is required; presence is guarded),
  but worth strengthening at Plan.

### [I2] — export CSV set corrected — **RESOLVED**

Spot-checked every filename against the WRITER `write_csv_exports` (`render.rs:568–732`):
- All-years (unconditional): `lots.csv` (577), `disposals.csv` (602), `removals.csv` (643),
  **`income.csv`** (694). The spec now lists `income.csv` — the round-1 `income_recognized.csv` bug is gone;
  the spec even correctly flags `state.income_recognized` (render.rs:703) as the in-memory Vec name, not the
  file. ✓
- With `--tax-year` (inside `if let Some(year)`, 717–730): `form8949.csv` (write_form8949_csv → 935),
  `schedule_d.csv` (→ 979), `form8283.csv` (→ 851; always within the block), and `schedule_se.csv` (→ 769;
  further gated on `Some(se_result)`). All four now enumerated (lines 63–68). ✓
- The spec says read EXACT headers from the writer `render.rs`, NOT the doc-comments (63, 67–68) — matching
  the round-1 caution that `main.rs:307` misnames it. ✓
- **Nit 2:** the `~577/694/720/727` line hints are approximate — `577`/`694` are exact header-write anchors,
  but `720`/`727` point at the *dispatch* call-sites (`write_form8283_csv(...)` at 720; the `if let Some(se)`
  guard at 727), whereas the actual header rows are inside the writers (`form8283.csv` at 851, `schedule_se.csv`
  at 769). The `~` and "read from the writer" instruction keep this from being misleading; optionally point at
  the writer functions.

### [M1] — TUI clap surface — **RESOLVED**

Line 39–40 now says the TUIs have "NO clap surface (they parse `env::args` manually, main.rs:69-88)."
Confirmed: neither `crates/btctax-tui/Cargo.toml` nor `crates/btctax-tui-edit/Cargo.toml` depends on clap;
`btctax-tui-edit` parses via `std::env::args()` in `parse_vault_path` (main.rs:73–89), and `btctax-tui`
has a bare `fn main() -> std::io::Result<()>`. The false "thin clap surface / stub" rationale is gone. ✓

### [M2] — editor keymap is a copy — **RESOLVED**

Line 42–43 now describes the keymap as "a hand-authored COPY of the `?`-overlay content (`draw_help_overlay`
is styled ratatui `Line`s, not a reusable const)" with "a KAT [that] pins the man page lists the current
action keys (same guard as the overlay's)." Confirmed: `draw_help_overlay` (`draw_edit.rs:1697–1738`) builds
a `vec![Line::from(Span::styled(...)), …]` — styled widgets, not an `include!`-able const; and an existing
completeness KAT (`help_lists_every_browse_action_key`) is the "same guard" referenced. The misleading
"single source with the overlay" claim is dropped and the drift is acknowledged. ✓ (The KATs-section
parenthetical "lists `?`, `V`, `O`" is only 3 illustrative keys, but the M2 prose "same guard as the
overlay's" commits to a completeness KAT — adequate.)

### [M3] — generator crate — **RESOLVED** (one stale reference → Minor 3)

Lines 34–38 and 84–89 now specify `crates/xtask` as a NEW workspace member (NOT a `[[bin]]` in
`btctax-cli`), with `clap_mangen = "0.2"` as its **normal** dep and `btctax-cli` as a dep for
`Cli::command()`; run via `cargo run -p xtask -- docs`. clap_mangen is explicitly NOT a runtime/build/dev
dep of the shipped `btctax`. The "build/dev dependency of the `[[bin]]`" error is corrected. ✓
- **Minor 3:** the "Locations" bullet (46–47) still refers to "the `gen-docs` bin" and "(or the bin
  itself) regenerates all" — stale wording from the pre-fold `[[bin]]` design. The authoritative resolution
  (crates/xtask, `cargo run -p xtask -- docs`) is at 34–38/84–89; the Locations bullet should be reconciled
  to it.

### [M4] — help KATs navigate to the subcommand — **RESOLVED**

Lines 98–101: `help_documents_<X>_format` NAVIGATE to the subcommand's `Command`
(`Cli::command().find_subcommand(...)`, recursing into `reconcile`) and render ITS long-help, with an
explicit note that the root `Cli::command()` long-help does not carry subcommand args. ✓

### [N1] — no `#[command(version)]` — **RESOLVED**

Lines 37–38, 89, and Gotcha 141–143 pin "No `#[command(version)]`" for determinism. Confirmed `Cli` at
`main.rs:16` (`#[command(name = "btctax", about = "…")]`) has no `version` attribute. ✓

### [N2] — classify-raw JSON — still correct (no change needed). ✓

---

## No-new-drift checks

- **File list is still the 6** (49–82; Task 1, 116–118): `init --key-backup`, `backup-key --out`,
  `export-snapshot --out`, `reconcile import-selections <csv>`, `reconcile classify-raw --payload-json`,
  `reconcile select-lots --from`. No additions/removals. ✓
- **groff PDF path unchanged:** `groff -man -T pdf` (44, 113, 128). ✓ — but see Minor 2 (the "all three"
  PDF task was not propagated to the per-subcommand page tree).
- **SemVer PATCH / docs-only holds** (91–94): only additive `///` long-help + the `verbatim_doc_comment`
  render attribute (not a flag/arg-name change), a new `docs/` tree + a generator crate, no new runtime dep
  on the shipped binaries, no GUI `schema_mirror` impact. ✓

---

## Minor findings (non-blocking)

- **[Minor 1]** *Locations bullet under-enumerates the `.1` set.* Line 45 lists generated
  `docs/man/{btctax,btctax-tui,btctax-tui-edit}.1` only — omitting the `btctax-<path>.1` per-subcommand tree
  that the C1 fix (line 28) and `manpage_covers_every_subcommand` (102–104) require. Reconcile the summary
  (e.g. `docs/man/btctax*.1` + "one `btctax-<path>.1` per subcommand").
- **[Minor 2]** *PDF task not propagated to per-subcommand pages.* Task 4 (128) says "`groff -T pdf` for all
  three → `docs/pdf/*.pdf`," but after C1 the file-format examples live in `btctax-<sub>.1`, so a CLI PDF
  built only from `btctax.1` would OMIT them. Requirement 3 (man page + `--help`) is still met by the `.1`
  files, so this is not Important — but the spec should state whether each subcommand `.1` also gets a PDF
  (or the CLI PDF concatenates the tree).
- **[Minor 3]** *Stale "gen-docs bin" wording.* Locations bullet (46–47) still says "the `gen-docs` bin" /
  "the bin itself," contradicting the M3 resolution (`crates/xtask`, `cargo run -p xtask -- docs`).
- **[Minor 4]** *No dedicated multi-line-survival KAT.* Round-1's I1 fix asked to pin a KAT asserting a
  multi-line example survives on separate lines (and verify clap_mangen's roff line breaks). The spec
  requires `verbatim_doc_comment` (primary fix) but leaves the line-separation guard to
  `file_format_examples_present_in_manpage`, which only checks presence.

## Nit findings

- **[Nit 1]** *`verbatim_doc_comment` citation imprecise* — "the codebase already uses this workaround at
  main.rs:111-119" points at the `long_help` *string* variant, not `verbatim_doc_comment` (zero current
  uses). Both are valid, so substance holds; reword to name the two mechanisms distinctly.
- **[Nit 2]** *`render.rs` line hints* — `~720/727` are dispatch call-sites; the actual `form8283.csv` /
  `schedule_se.csv` header writes are at 851 / 769. Approximate but not misleading given the "read from the
  writer" instruction.

---

## Bottom line

**R0-GREEN — 0 Critical / 0 Important.** C1/I1/I2 and all Minors/Nits from round 1 are folded; the
per-subcommand `clap_mangen` mechanism, the KAT coherence, the `verbatim_doc_comment` requirement, and the
corrected export-CSV set are each verified against current source at `7387b17`. The four Minor and two Nit
items above are optional tidy-ups (Locations/PDF/`gen-docs` wording, one KAT, two citations) and do not gate
the Plan phase.
