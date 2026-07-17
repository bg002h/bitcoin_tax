# Usage-examples docs — CONTINUITY (resume point)

*Written 2026-07-16. **This is a pre-brainstorm scaffold**, not a spec. Recon is done and persisted;
the next phase is BRAINSTORM. Safe to clear context here — everything needed is on disk.*

## The goal (user's ask, 2026-07-16)

Build usage-example documentation for the **btctax constellation**, modeled on what the sibling
"mnemonic constellation" already did, and use the authoring process to **discover workflow/usability
bugs** in btctax (that dual purpose is explicit and wanted):

- **Artifact 1 — a CLI verbatim-I/O examples doc**: real `btctax` command-line input **and** output,
  captured verbatim, as a distributed doc (PDF).
- **Artifact 2 — a SEPARATE TUI-capture doc**: `btctax-tui` / `btctax-tui-edit` screens. Kept in a
  **separate file** from the CLI examples on purpose (screenshots/captures make for a big file — the
  user was explicit about the split).

The mnemonic originals: `mnemonic-toolkit/docs/Examples.pdf` (CLI) and
`mnemonic-toolkit/docs/manual-gui/build/gui_example.pdf` (GUI screenshots).

## What's already done

- **3 Opus recon agents** reverse-engineered the mnemonic method. Their full reports are persisted
  verbatim in **`design/usage-examples/RECON.md`** (read it first — the digest below is only a map).
- **Key btctax facts spot-verified** against current source (2026-07-16): `TestBackend::new(120,40)`
  in `crates/btctax-tui/src/tabs/tests.rs`; groff/roff docs pipeline in `crates/xtask/src/docs.rs`
  (**no pandoc/xelatex anywhere in the repo**); synthetic `crates/btctax-cli/tests/fixtures.rs`;
  deterministic price source `crates/btctax-adapters/data/btc_usd_daily_close.csv`; `ratatui = "0.29"`.

## Digest of the method (full detail in RECON.md)

**How mnemonic built the CLI examples doc (`Examples.pdf`):** one hand-written bash generator
(`.examples-build/gen.sh`) with `run()`/`show()` helpers that print `$ cmd` then **execute the real
binary at build time** and capture combined stdout+stderr into fenced Markdown, interleaved with prose
heredocs. `bash gen.sh > Examples.md`, then pandoc→xelatex to PDF. **Determinism is the crux**: FATAL
`--version` pin, env scrub (`HOME=/home/user`, `TZ=UTC`, `LC_ALL=C`), script-relative paths, static
date literal (never `$(date)`), fixed public test-vector inputs, capture via `$(…)` not a TTY;
anything un-reproducible is frozen as a labelled **STATIC CAPTURE**. **The gate** (`examples.yml`):
regenerate the golden `.md` and `git diff --exit-code` it (the PDF is only re-proven to *build* — not
byte-gated, xelatex isn't reproducible). Required check with a fail-safe PR guard to avoid the
"required + path-filtered = wedged" trap.

**How mnemonic built the screenshot doc (`gui_example.pdf`):** mnemonic-gui is a **graphical egui app**,
so it uses `egui_kittest` → wgpu → **software-rasterizer PNG** capture, byte-gated in CI, byte-copied
into the toolkit repo, assembled by pandoc→xelatex, **release-attach-only** (~32 MiB, too big to
commit). **This pixel machinery does NOT transfer to our terminal TUI** — but the architecture around
it does.

**★ The single most important adaptation insight:** btctax's TUI is a **text** grid with an **existing**
`TestBackend` render seam. So the expensive, fragile half of the mnemonic method (whole-window pixel
determinism, GPU/rasterizer drift, 32-MiB budget) **is free here** — the TUI can be captured as
**verbatim text**, committed and gated exactly like the CLI leg. The method collapses to: *run journeys
against synthetic data → capture verbatim text → commit as goldens → `git diff` in CI → catalog every
workaround the author had to perform.* The remaining determinism work is entirely about
**prices / clock / EventIds**.

**Why this finds bugs (the payoff):** worked journeys exercise the *assembled* surface in sequence, so
they hit cross-flag interactions, mode-dependent refusals, and affordance gaps that isolated unit tests
never probe. In mnemonic, authoring the tutorial *forced the author to be the user* and directly
produced real fixes (a `(none)` template affordance that unblocked an unreachable code path; a
secret-reveal toggle). btctax's analogue = driving `btctax-tui-edit` through a real reconcile flow. The
deliverable that pays for the exercise is a **workaround-audit** (`tutorial-workaround-audit.md` style):
catalog every route-around, classify each as bug-to-file / harness-artifact / intentional.

## Open questions to settle in the brainstorm (from RECON.md §5 of report 3)

1. **Scope / journeys** — one canonical end-to-end journey, or several (single-buyer; multi-lot §170(e)
   donation; self-transfer reconcile; income-with-missing-FMV)? More journeys = more surface = more bugs.
2. **Which subcommands earn a narrated example** (man pages already cover the full ~54-command surface;
   worked examples are editorial).
3. **Determinism strategy — load-bearing.** Prices (`--as-of` + committed daily-close CSV), the
   tax-year/"today" clock, `TZ`/`LC_ALL`, scrubbed paths, and **EventIds** — *are EventIds deterministic
   for a fixed synthetic vault, or do they embed timestamps/randomness?* MUST be checked early; if
   nondeterministic, an EventId-pinning shim precedes everything.
4. **Real vs synthetic data / PII** — synthetic-only is already the house rule (`.gitignore` + PII hook).
   Commit a fixed synthetic vault, or fixed synthetic import CSVs (regen the vault in CI)?
5. **CI gate posture** — advisory vs required; narrow (`docs/**`) vs wide (`crates/**`, leading
   indicator) triggers. Recommendation: level-A regen-and-`git diff`, advisory first, promote once
   born-green.
6. **Tooling — reuse groff or add pandoc?** btctax has groff/roff (`xtask` + `make bundles`) and **no**
   pandoc. Reusing groff keeps the repo's existing determinism guarantees and adds no heavy dep; pandoc
   buys nicer typography + the "prose==output" transcript model. Decide up front.
7. **TUI capture format** — committed text goldens vs rendered PDF. Because `TestBackend` yields text,
   btctax can cheaply have both (text golden = gated source; PDF = a render of it).
8. **Primary goal — the doc, or the bug-hunt?** If the bug-hunt is the point, make the adversarial
   workaround-audit an explicit, budgeted deliverable, not a side effect.

## Standing constraints

- **Follows `STANDARD_WORKFLOW.md`** — brainstorm → spec → plan → implement (phased, TDD) →
  whole-diff review → ship, each "→ green" an independent review loop to 0C/0I. **Reviews use Fable**
  (standing user directive). Treat "it's just docs" as exactly the rationalization the gates exist to
  override — the mnemonic cycle ran docs through the *full* spine, and that discipline is why the gate
  works. See [[standard-workflow]], [[binary-docs-infra]].
- **Synthetic data only** in any committed/distributed artifact — never real taxpayer data/PII.
- **Two separate artifacts** (CLI doc ≠ TUI doc) — user-mandated split.
- **Don't edit the compute/fill engine to make a doc pretty.** Bugs the authoring surfaces → FOLLOWUPS
  (severity + owning phase), same as the oracle-sweep discipline.
- The btctax workspace was just released **v0.6.1** (all 10 crates on crates.io); `main` is clean.

## ▶ Kick-off — paste into a FRESH session in this repo (`/scratch/code/bitcoin_tax`)

> We're building usage-example documentation for the btctax constellation (a CLI verbatim-I/O examples
> doc + a SEPARATE TUI-capture doc), modeled on the mnemonic constellation's method, and using the
> authoring to discover btctax UX/workflow bugs. Recon is done and persisted. Read
> `design/usage-examples/CONTINUITY.md` then `design/usage-examples/RECON.md`, then invoke
> `superpowers:brainstorming` to work through the 8 open questions with me before any spec. Reviews use
> Fable; follow STANDARD_WORKFLOW. Do NOT start writing a spec or code until the brainstorm converges.

*(If you prefer, the brainstorm can begin by verifying open question #3 — EventId determinism — since it
gates the whole golden-diff approach.)*
