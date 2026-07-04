# btctax man pages

Roff man pages for the three binaries — the CLI `btctax(1)` (plus one page per
subcommand, git-style) and the interactive `btctax-tui(1)` / `btctax-tui-edit(1)`.

## Layout

- `btctax.1` — CLI root: NAME / SYNOPSIS / DESCRIPTION / OPTIONS / SUBCOMMANDS / FILES / EXAMPLES.
- `btctax-<sub>[-<subsub>].1` — one page per clap subcommand (e.g.
  `btctax-reconcile-import-selections.1`). The file/structured-argument FORMATS (key
  backup, export CSV set, import-selections CSV, classify-raw JSON, select-lots picks)
  are documented on their owning subcommand page.
- `btctax-tui.1`, `btctax-tui-edit.1` — the interactive viewer / editor (hand-authored;
  the apps have no clap surface). The editor page mirrors the in-app `?` keymap overlay.

## Regenerate

The generated CLI pages come from the clap command tree via `crates/xtask`. The single
source of truth for the inline file-format docs is the clap doc-comments in
`crates/btctax-cli/src/cli.rs` — they flow to BOTH `--help` and these man pages, so
there is no drift. The two TUI pages are hand-authored roff and edited directly.

- Man pages only (no external tools): `cargo run -p xtask -- docs`  (or `make docs-man`)
- Man pages **and** PDFs: `cargo run -p xtask -- docs --pdf`         (or `make docs`)

Generation is deterministic (no embedded dates, no `#[command(version)]`).
`cargo test -p xtask` fails if the committed `.1` pages are stale (regenerate and commit).

## View

```
man -l docs/man/btctax.1
man -l docs/man/btctax-reconcile-import-selections.1
man -l docs/man/btctax-tui-edit.1
```

To install system-wide, copy the `.1` files into a `man1` directory on your `MANPATH`
(e.g. `~/.local/share/man/man1/`) and run `mandb`.

## PDFs

`docs/pdf/*.pdf` are produced by `groff -k -man -T pdf` from each `.1`. They are **build
artifacts** (git-ignored): gropdf embeds a creation timestamp, so they are not
byte-reproducible. Regenerate them with `make docs`.
