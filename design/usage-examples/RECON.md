# Usage-examples docs — RECON (mnemonic-constellation method)

*Written 2026-07-16. Three Opus recon agents fanned out over the sibling "mnemonic
constellation" (`/scratch/code/shibboleth/mnemonic-*`) to reverse-engineer how they built
(a) a verbatim CLI-I/O examples PDF and (b) a GUI-screenshot tutorial, both of which doubled
as usability/workflow **bug-discovery** instruments. Goal: replicate for the btctax
constellation — a CLI verbatim-I/O examples doc + a **separate** TUI-capture doc.*

**The three reports below are persisted verbatim** (HTML transport-entities decoded to `<`/`>`/`&`).
Read all three before the brainstorm. The tl;dr digest + open questions live in `CONTINUITY.md`.

---

## Report 1 — the "Examples PDF" method in `mnemonic-toolkit` (CLI verbatim-I/O)

**Scope note / current-state flag:** the SPEC and PLAN describe the original modernization (pin `0.55.3 → 0.75.0`, shipped as tag `examples-v1.0.0`, 2026-07-05). The **live in-tree `gen.sh` has since been re-pinned to `mnemonic 0.90.0`** (`gen.sh:3,44,109`; golden subtitle `Examples.md:3`). That drift is itself the strongest evidence the gate works: the "re-pin + regen every release or CI goes red" ritual has fired through at least two later releases (0.75.0 → 0.90.0). All line numbers below are against the **current** files unless I say "baseline."

Key files (all absolute):
- Generator: `/scratch/code/shibboleth/mnemonic-toolkit/.examples-build/gen.sh`
- Committed golden (gated): `/scratch/code/shibboleth/mnemonic-toolkit/.examples-build/Examples.md`
- Tracked input assets: `.examples-build/{degrade2.desc, tr2.desc, tr4.desc, degrade2-spec.json}` + helper `.py`/`.sh` (see `git ls-files .examples-build`)
- CI gate: `/scratch/code/shibboleth/mnemonic-toolkit/.github/workflows/examples.yml`
- Deliverable: `/scratch/code/shibboleth/mnemonic-toolkit/docs/Examples.pdf`
- Design: `design/SPEC_examples_pdf_modernize_and_gate.md`, `design/IMPLEMENTATION_PLAN_examples_pdf_modernize_and_gate.md`, `design/agent-reports/examples-pdf-*.md`
- The "deferred gate B" reusable transcript model (sibling books): `docs/manual/tests/verify-examples.sh`, `docs/manual/pandoc/filters/include-transcript.lua`, `docs/manual/transcripts/*.{cmd,out,err}`, `docs/manual/Makefile`

### 1. End-to-end pipeline

The whole thing is **one hand-written bash script that executes the real CLI at build time and prints Markdown to stdout**, then a single pandoc/xelatex render. No typst, weasyprint, or wkhtmltopdf; no custom harness beyond the shell script.

**Source format → capture → render:**

1. **`gen.sh` is the source.** It interleaves prose (`cat <<'MD' … MD` heredocs) with two emit helpers (`gen.sh:88-103`):
   - `run() { printf '\n```\n$ %s\n' "$1"; printf '%s\n```\n' "$(eval "$1" 2>&1)"; }` (`gen.sh:100`) — prints `$ <cmd>` **verbatim**, then **executes** it via `eval … 2>&1` (combined stdout+stderr), capturing with `$( )` and re-emitting inside a fenced code block. The command is single-quoted at the call site so display == what executed, with `$VAR`/`$( )` expansion deferred to run time. Output is captured (not streamed) and re-emitted with an explicit trailing newline so the closing ``` always lands on its own line (a real bug they hit — a `cat` of a no-final-newline descriptor ran into the fence and pandoc dropped it; see the comment at `gen.sh:93-99`).
   - `show()` (`gen.sh:103`) — prints command(s) in a fence **without executing** (for the `curl|sh` installer and bitcoin-cli steps needing a live node).
2. **Run:** `bash .examples-build/gen.sh > .examples-build/Examples.md`. There is no `make`/mdBook pre-build; the only ordering constraint is "binary must exist first."
3. **Render:** `gen.sh` writes its own LaTeX preamble (`gen.sh:67-86` → `preamble.tex`, deterministically) then the header comment (`gen.sh:8-11`) gives the exact render line:
   `pandoc Examples.md --include-in-header=preamble.tex --listings --pdf-engine=xelatex -f markdown-smart -o Examples.pdf`
   The preamble configures the `listings` package for line-wrapping (`breaklines=true`), a grey hook-arrow continuation marker (`postbreak=\mbox{\textcolor{gray}{$\hookrightarrow$}…}`), and a `literate` map that forces output unicode (Δ ± × → … —) through the DejaVu Sans Mono code font, plus an active-char hack for U+2265/U+2264 that otherwise leak out of listings into the roman body font.
4. **Tooling:** bash + the `mnemonic` binary + `jq` + `python3` + `sed` at capture time (all preflight-checked, `gen.sh:37-40`); `pandoc` + `xelatex` (texlive) + `fonts-dejavu` at render time.

### 2. Deterministic capture — the crux

**Yes, commands are actually executed at build time** (`run`'s `eval`). The design goal is: **gen.sh output is a pure function of `(repo tree, mnemonic binary)`** — nothing else. The techniques (all in `gen.sh:13-44`, spec §2 D1.2–D1.5, plan P1.b):

- **Binary pin via FATAL version check** (`gen.sh:43-44`): `VER=$(mnemonic --version); [ "$VER" = "mnemonic 0.90.0" ] || { echo FATAL…; exit 1; }`. This is the enforcement point — regen against any other binary aborts the build.
- **Binary sourcing override** (`gen.sh:23`): `export PATH="${EXAMPLES_BIN_DIR:+$EXAMPLES_BIN_DIR:}$HOME/.cargo/bin:$PATH"`. CI sets `EXAMPLES_BIN_DIR="$GITHUB_WORKSPACE/target/debug"` so the source-built binary wins over any stale cargo-installed one. Displayed commands stay bare `mnemonic` (display-faithful, execution-pinned).
- **Environment scrub** (`gen.sh:30-32`), done *after* PATH resolves off the real `$HOME`: `unset XDG_DATA_HOME CARGO_INSTALL_ROOT; export HOME=/home/user; export LC_ALL=C LANG=C TZ=UTC`. This pins every path the captured `install.sh --dry-run`/`--list` prints (they derive from exactly those three vars) to `/home/user/…` on any machine, and locale-pins the `python3`/`jq`/`sed` output.
- **Script-relative repo root** (`gen.sh:17`): `REPO="${REPO:-$(cd "$(dirname "$0")/.." && pwd)}"` — no hardcoded `/scratch/code/…` leaking into captured `install.sh` output. The two installer `run` calls are single-quoted (`gen.sh:202-203`) so the *display* shows the literal `sh "$REPO/scripts/install.sh" …` while `eval` still expands `$REPO` at runtime.
- **Static date literal, never `$(date)`** (`gen.sh:110`: `date: "2026-07-05"`).
- **Strict preflight** (`gen.sh:37-40`): FATAL up-front if `jq`/`python3`/`sed` are missing — otherwise a missing tool's `command not found` would be silently captured *into the document body* by `run`'s `eval … 2>&1`.
- **Deterministic inputs:** all keys/xpubs/addresses derive **live** from three hard-coded public BIP-39 test vectors (`gen.sh:49-51`) via `mnemonic convert`; the descriptor `.desc`/`.json` fixtures are fixed tracked files; no RNG, no network.
- **TTY-invariance:** `run` always captures via `$( … 2>&1)`, never a TTY, so any `is_terminal`-gated output is identical local vs CI.

**What cannot be pinned is frozen as a labelled STATIC CAPTURE, not executed** — the decisive tactic. Two cases:
- **§6.6 Bitcoin Core cross-check** (`gen.sh:679-686`): the `bitcoin-cli deriveaddresses` line + `["bc1p550…"]` result are a literal heredoc labelled *"(STATIC CAPTURE — recorded from Bitcoin Core v27.0 … NOT regenerated by gen.sh … needs no node in CI.)"* The address is a deterministic function of the fixed descriptor and is *also* re-derived live every run by the kept `mnemonic restore` (so a real change still reds the gate via the live line — "doubly-safe").
- **Appendix B experimental depth-≥2 reconstruction** (`gen.sh:764-790`): the output of the never-shipped `mnemonic-depth2` binary, frozen with an explicit *"recorded 2026-06-15 … not reproducible with a released binary or in CI"* label.

Because these are literal text, they're byte-covered by the whole-file diff like any prose and the CI differ needs **zero skip/carve-out logic** — the exemption is structural, not a diff filter.

Determinism was *proven*, not assumed (plan P1 gate; post-impl review §1): double-regen byte-identical; regen under two different real `$HOME` values byte-identical; regen with `mnemonic-depth2` present-vs-absent byte-identical.

### 3. The gate (`examples.yml`)

One job `examples` on `ubuntu-latest`. **What fails the build** and how:

- **Triggers** (`examples.yml:23-36`): `push` to `master`/`main` with a **wide** `paths` filter (`.examples-build/**`, `docs/Examples.pdf`, `scripts/install.sh`, `crates/**`, `Cargo.lock`, `Cargo.toml`, the workflow itself); `tags: examples-v*`; and `pull_request` **with no `paths` filter at all**.
- **Why no PR paths filter:** the check is a **required** branch-protection status check, and a path-filtered required check wedges forever at "Expected — waiting for status." So the job always runs+reports on every PR, and a fail-safe internal **guard** (`examples.yml:51-78`) computes `relevant=true/false` from `git diff --name-only FETCH_HEAD HEAD`; the 8 heavy steps carry `if: github.event_name != 'pull_request' || steps.guard.outputs.relevant == 'true'`. The guard is fail-safe by construction: the two-dot diff over-includes (never misses a PR-side edit to a gated path), and a fetch/diff error hard-fails under `bash -eo pipefail` — it can only ever err toward *running* the gate, never toward a silent false-green.
- **The load-bearing steps** (`examples.yml:80-143`): apt install pandoc+texlive+fonts-dejavu+jq → `dtolnay/rust-toolchain@1.85.0` + rust-cache → `cargo build --bin mnemonic` (**debug** binary is sufficient — `manual.yml` precedent) → **regen** `EXAMPLES_BIN_DIR="$GITHUB_WORKSPACE/target/debug" bash .examples-build/gen.sh > .examples-build/Examples.md || exit 1` → **the gate**: `git diff --exit-code -- .examples-build/Examples.md` → build the PDF (proves buildability) → upload artifacts → on `examples-v*` tag, `gh release upload docs/Examples.pdf --clobber`.
- **Three ways to red:** (1) `gen.sh` FATAL — a **crate version bump not accompanied by a re-pin+regen** (the `gen.sh:44` check), or a missing preflight tool; (2) **any drift** between the committed golden and the fresh regen — a new/removed subcommand in `--help`, changed output format, an `install.sh` self-pin advance, a refusal-message change, or a hand-edit to the golden that regen doesn't reproduce — printed verbatim in the CI log; (3) the PDF failing to build.
- **The PDF is NOT byte-gated** (`examples.yml:132-143` comment): xelatex output isn't reproducible across texlive versions, so the golden `.md` is the gated artifact and CI only re-proves the PDF *builds*. The committed `docs/Examples.pdf` is **convention-synced** (a regen that updates the golden but not the PDF still passes — an accepted honesty gap, mitigated by the release ritual and by `docs/Examples.pdf` being in the trigger paths).
- **Governance:** `examples` is a **required** context on `master` (`strict:false`, `enforce_admins:false` so admin direct-FF release pushes still work — the GUI `snapshots` precedent). The accepted consequence, stated repeatedly (spec §8, plan locked-decision 2): **every future release must re-pin `gen.sh` + regen the golden in the same PR or go red** — converting a memory-enforced ritual into a gate-enforced one.
- **Born-green rollout:** the `.gitignore` flip (untrack→track `Examples.md`) + `git add` of the golden + `examples.yml` land in one atomic commit; the negative proof (perturb one golden byte → observe RED at the diff step → drop) was mandatory before ship (plan P2 gate).

### 4. Artifact structure — and how the several PDFs differ

**`Examples.pdf` is a flat, monolithic "worked-examples" tutorial**, not a manual. Its `.md` (~1431 lines → ~214 KB PDF, text-only, no figures) is numbered sections emitted top-to-bottom by `gen.sh`: About/conventions → §1 Install → §2 single-sig → §3 multisig (+ §3.4 one-machine) → §4 Bitcoin Core import → §5 degrading-miniscript "pathological" wallet → §6 taproot (+ §6.5 cost comparison, §6.6 Core cross-check) → Appendix A (test seeds) → Appendix B (experimental depth-2). **Every block is `$ cmd` followed by full verbatim combined output** — no ellipses, no elided keys. YAML front-matter sets `toc-depth: 2`, `monofont: DejaVu Sans Mono`, `fontsize: 10pt`.

This is a *different machine* from the four "books," which is the axis that distinguishes all the PDFs:

| PDF | Source dir / workflow | Model |
|---|---|---|
| **`docs/Examples.pdf`** | `.examples-build/` / `examples.yml` | **Monolithic gen.sh: run the CLI at build time, capture verbatim I/O, whole-file `.md` golden diff.** The target of this recon. |
| `docs/m-format-manual.pdf` | `docs/manual/` / `manual.yml` | Full reference manual — many `src/*.md` concatenated + pandoc/xelatex, **with** mermaid figures, index, anchors. Uses the transcript-pair include machinery (§6 below). Authoritative per-flag reference. |
| `docs/m-format-quickstart.pdf` | `docs/quickstart/` / `quickstart.yml` | Condensed getting-started; its `pandoc/filters` + `transcripts` dirs are **symlinks** to the manual's single-source copies (62 shared + 6 own gated transcripts). |
| `docs/technical-manual/build/m-format-technical-manual.pdf` | `docs/technical-manual/` / `technical-manual.yml` | API/codec manual; additionally has a real **cargo-example crate** (`docs/technical-manual/examples/`) whose `*-api-roundtrip.rs` run via `cargo run --manifest-path …` inside the transcript harness. |
| `docs/manual-gui/build/gui_example.pdf` | `docs/manual-gui/` / `manual-gui.yml` | GUI walkthrough with a ~32 MiB screenshot corpus; **release-attach-only** (too big to commit). |
| `docs/m-format-ultraquickstart.pdf` | — | A condensed one-pager referenced only by `changelog-check.yml` + a manual FOLLOWUP; **not** built by a dedicated workflow or the transcript machinery — apparently hand-maintained / outside the gated pipeline. |

The four books gate output at **per-command granularity** (`.cmd`/`.out` goldens materialized into fences by a pandoc Lua filter — "prose == output by construction"); `Examples.pdf` gates at **whole-document granularity** (regenerate the entire `.md`, diff it). The `Examples.pdf` project explicitly names the book model as its deferred **"gate B"** (spec §7a, plan Honesty §a).

### 5. Docs-as-bug-discovery — concrete evidence

- **The `Examples.pdf` cycle was *born* from doc-rot discovery.** Inspecting the committed PDF revealed a **"three-era patchwork"** (SPEC §0, `SPEC_…md:24-28`): bundle cards printed **twice** (a 0.55.3-era display artifact), a stale `--help` capture **missing the `gen-man` and `word-card` subcommands** that had since shipped, and an `install.sh --list` capture pinning `v0.73.3` — i.e. the doc had silently drifted from the CLI across three release eras. Plus a **narration time-bomb**: `gen.sh:209` said *"Each card is printed twice…"* — flatly **false** at ≥ v0.56.0, where the CLI changed to print once. The gate exists precisely so that (spec §7) *"the three-era patchwork could not have survived this gate."*
- **The transcript machinery caught a fictional CLI diagnostic.** `FOLLOWUPS.md:185`: the manual's recovery-paths section claimed `mnemonic convert --from ms1=…` emits `position 11: invalid character 'Q' (expected 'q')` — a per-character BCH diagnostic. Capturing the real golden proved **the binary emits no such thing** — it rejects on **length** first (`error: ms1 string length 49 not in v0.1 set [50,56,…]`, exit 1). Writing+gating the example corrected the prose *and* spawned a follow-up for the aspirational feature the fictional prose had implied.
- **Writing the GUI walkthrough surfaced two missing UX affordances** that became their own fix cycle `tutorial_surfaced_fixes_batch` (`FOLLOWUPS.md:243,250,260`): `gui-secret-reveal-toggle` (a 👁 reveal control) and `restore-template-none-affordance` (a `(none)` template option). The `examples-pdf-un-ci-gated` FOLLOWUP itself was surfaced by that same tutorial recon (E5).
- **Standing behavioral assertion baked into the doc:** the depth-≥2 taproot **refusal** is deliberately kept *live and gated* (`gen.sh:757`, spec §5): *"if a future miniscript bump silently lifts the cap, examples.yml goes red and forces a doc decision."* The example doubles as a regression detector.
- **Adjacent corroboration of the "execute the real thing = surface drift" thesis** (found by running the CLI, not writing prose, but same principle): stress Cycle A found on the *first* proptest run that `sortedmulti`-in-combinator is accepted by build/bundle/export but refused by restore (`FOLLOWUPS.md:365`); and a CRITICAL funds-safety bug where `restore --md1` silently reconstructed a *different* wallet with the timelock dropped, exit 0, false "verified" banner (`FOLLOWUPS.md:373`).

### 6. Reusable vs mnemonic-specific (for a clap CLI like btctax)

**Transfers directly** (btctax is also a clap CLI emitting deterministic text, already has `xtask` + `make docs` from clap doc-comments):

- **The whole `gen.sh` shape**: one bash generator with `run()`/`show()` helpers emitting `$ cmd` + verbatim combined output into fences, interleaved with `cat <<'MD'` prose; `bash gen.sh > X.md`; pandoc+xelatex with a `listings` preamble (line-wrap + hook-arrow marker). Copy `gen.sh:88-103` + `gen.sh:8-11,67-86` nearly verbatim.
- **The determinism discipline (the crux)** — copy the pattern wholesale: FATAL version-pin check against `btctax --version`; env scrub (`HOME`, `LC_ALL/LANG/TZ`, unset locale/XDG); script-relative paths, no hardcoded cwd; static date literal (never `$(date)`); strict preflight FATAL on any tool the examples shell out to; capture via `$(…)` not a TTY.
- **The gate**: whole-file golden `.md` + `git diff --exit-code`, born-green atomic commit, and the **required-check-with-fail-safe-PR-guard** trick (`examples.yml:36,51-78`) so a required check never wedges. `EXAMPLES_BIN_DIR`-style binary override so CI's source-built binary wins.
- **The "freeze un-pinnable output as a labelled STATIC CAPTURE" tactic** — for anything btctax can't reproduce in CI (wall-clock, an external oracle, e-file endpoints, a feature behind an unshipped dep).
- **The two-tier honesty framing**: start with **Gate A** (whole-file regen-drift; cheap; catches output drift but *not* whether prose tells the truth about adjacent output), then optionally adopt **Gate B** (the sibling-book `.cmd`/`.out` transcript pairs + `include-transcript.lua` fence-materialization + `verify-examples.sh`). Both Gate-B files are essentially **generic and directly reusable** — `verify-examples.sh` takes `$MNEMONIC_BIN=`-style binary-path substitutions and per-cmd `mktemp -d` cwds; `include-transcript.lua` is a fail-closed pandoc filter parameterized only by `TRANSCRIPTS_DIR`. Rename the substitution vars to `$BTCTAX_BIN` and they work.

**Needs rework / mnemonic-specific:**

- **Domain content is 100% mnemonic**: BIP-39 seeds, descriptors, taproot/miniscript, the `.desc`/`.json` fixtures, "secret material on argv" warnings, Appendix B. btctax's fixtures are synthetic taxpayer scenarios / sample returns — the existing household corpus (oracle-sweep work) is the natural source, **but flag privacy**: use synthetic filers, never real taxpayer data, in a distributable PDF.
- **The invoked side-tools** (`jq`, `python3`, `sed`, frozen `bitcoin-cli`/Bitcoin Core) are mnemonic-specific — btctax's generator preflights/invokes only what its examples need (likely just the `btctax` binary, maybe `jq`). Drop the pinned-Core precedent unless btctax has an external oracle to cross-check against.
- **The LaTeX `literate` unicode map + the U+2265/2264 active-char hack** (`gen.sh:67-86`) are tuned to mnemonic's exact output glyphs; re-derive for btctax's output (tax output likely needs `$ % × ≤ ≥` and en/em dashes — partial overlap, but verify).
- **Version-pin coupling**: mnemonic ties the FATAL check to the crate version *and* to `install.sh`'s self-pin (which advances every release). btctax's analog is just `btctax --version`; the "every release must re-pin+regen or go red" ritual transfers, but the exact pin sites are btctax's.
- **Constellation infra is irrelevant**: `.docbins` tiers, sibling CLIs (md/ms/mk), `schema_mirror`/GUI locksteps, the g6/mlock byte-identity invariants.
- **Inherited honesty gaps to plan for**: (a) the committed PDF is *not* byte-gated (xelatex non-reproducible) — a stale committed PDF passes CI; mitigate with the release ritual + PDF-in-triggers. (b) Gate A doesn't close the "narration lies about unchanged output" hole — only Gate B does. (c) A live illustration of residual comment-rot: the current `.examples-build/.gitignore:5` still says *"Run with mnemonic 0.75.0"* while `gen.sh` is pinned to 0.90.0 — because that comment isn't in the gated golden. Lesson for btctax: keep the version pin in exactly **one gated place**.

---

## Report 2 — the TUI/GUI screenshot method in `mnemonic-gui`

### Bottom line up front

`mnemonic-gui` is a **graphical desktop GUI (egui/eframe on wgpu/winit)**, *not* a terminal app. Its screenshots come in **three distinct tiers**, and the interesting one — the deterministic, CI-gated corpus that feeds a distributed PDF — is captured **headlessly via `egui_kittest`'s offscreen wgpu renderer on a software rasterizer (lavapipe/llvmpipe)**. That specific capture mechanism is **egui-only and does not transfer** to btctax's ratatui TUI. However, the surrounding **architecture** (headless deterministic render → committed PNG corpus → byte-identical CI gate → pandoc/xelatex-assembled *separate* PDF book) transfers cleanly, and btctax **already has the equivalent render seam** (`ratatui::TestBackend`).

### 1. What mnemonic-gui is, and how screenshots are captured

**Framework (definitive):** egui + eframe, graphics stack wgpu → winit, with wayland/x11/accesskit. Evidence: `/scratch/code/shibboleth/mnemonic-gui/Cargo.toml` lines 13–14 (`eframe = { version = "0.31", ... features = ["wgpu", ..., "wayland", "x11", "accesskit"] }`, `egui = "0.31"`). README line 4: "Built with egui; single statically-linked binary per platform." It is a native windowed desktop app that subprocess-runs the four sibling CLIs.

There are **three tiers of images**, captured three different ways:

**Tier A — 3 hand-captured release screenshots (MANUAL).**
- `/scratch/code/shibboleth/mnemonic-gui/screenshots/01-mnemonic-bundle.png`, `02-mnemonic-export-wallet.png`, `03-mnemonic-convert.png`.
- 1873×1612 RGBA, 152–217 KB each, all dated within 3 consecutive minutes (May 12 15:11–15:13) — real OS window grabs of a human running the app with real multisig data filled in.
- Referenced **only** in `README.md` (lines 79/83/87, `![...](screenshots/...)`). An agent report explicitly classifies them as "release UI screenshots, not kittest artifacts" (`design/agent-reports/v0_2-phase-A3-kittest-scaffold-r2.md:123`). **These are manual and un-gated.**

**Tier B — 61 headless form snapshots (SCRIPTED / DETERMINISTIC / GATED).**
- `/scratch/code/shibboleth/mnemonic-gui/tests/snapshots/forms/<tab>-<sub>.png`, 61 files (mnemonic 32 + md 10 + ms 10 + mk 9), 857×912.
- Generated by `tests/gui_form_snapshots.rs` (`gui_form_snapshots_all_61`) using `egui_kittest::Harness` — an **offscreen wgpu render** of each subcommand form over a canonical blank fixture, at `pixels_per_point = 2.0`, viewport `fit_contents()`-sized, compared against the committed PNG at kittest's **default dify threshold 0.6**.
- `egui_kittest = { version = "0.31", features = ["wgpu", "snapshot"] }` is a **dev-dependency only** (Cargo.toml:112) — never in the shipped binary graph.

**Tier C — 50 headless tutorial snapshots + transcripts (SCRIPTED / DETERMINISTIC / GATED).**
- `/scratch/code/shibboleth/mnemonic-gui/tests/snapshots/tutorial/` — 50 PNGs + `.stdout/.stderr/.exit.txt` transcripts, organized as 5 "journeys" (J1–J5).
- Generated by `tests/gui_tutorial_snapshots.rs` — this drives the **REAL whole-window app** (`app_window::MnemonicGuiApp`) at 920×720 @ ppp 2.0 through a scripted manifest (`tests/tutorial/manifest.rs`, `Step`/`Drive` interpreter), filling each form, capturing the filled-form / secret-modal / populated-run-pane shots, and byte-persisting the CLI transcripts. Authority doc: `mnemonic-toolkit/docs/manual-gui/design/SPEC_gui_example_tutorial.md`.

**Determinism knobs (both B and C):** fixed `pixels_per_point`, fixed window size, canonical blank fixtures, and a hard **software-rasterizer provenance guard** — the test asserts the selected wgpu adapter is `device_type == Cpu` unconditionally, so GPU-driver pixel drift can't poison the corpus (`tests/gui_form_snapshots.rs`, the "A1 adapter guard").

### 2. Screenshot doc assembly — how images become a distributed artifact

The distributed artifact is **a PDF book (and an HTML twin)**, and critically it is **assembled in the *sibling toolkit repo*, not in the GUI repo**: `/scratch/code/shibboleth/mnemonic-toolkit/docs/manual-gui/`.

There are actually **two books** built there, both via the same `Makefile`:

1. **Reference manual** — `build/m-format-gui-manual.pdf` + `.html` (4.0 MB HTML). Embeds the **61 Tier-B form screenshots** as a "GUI-Forms gallery" (`src/75-gui-forms/…`, images referenced `../../figures/gui/<stem>.png`).
2. **Tutorial book** — `build/gui_example.pdf` (**11 MB**) + `.html`. Embeds the **50 Tier-C tutorial screenshots + transcripts** across the 5 worked journeys (`tutorial/*.md`, images `../figures/tutorial/<name>.png`, transcripts via a pandoc Lua filter `include-transcript.lua`).

**Toolchain (`docs/manual-gui/Makefile`):**
- **PDF:** `pandoc --to latex` (custom template `pandoc/templates/manual.latex` + `pandoc/preamble.tex` + `preamble-tutorial.tex`) → `xelatex ×3` + `makeindex`. Lua filters: `include-transcript.lua`, `primer-box.lua`, `wrap-long-code.lua`, mermaid-cache.
- **HTML:** `pandoc --to html5 --standalone --self-contained` — images embedded as **data-URIs** (gate is a positive census: `grep -c 'src="data:image/png'` == committed PNG count).
- **Reproducible build:** `make pdf-docker` runs the whole thing inside `Dockerfile.build` (pinned).
- **Release posture:** `gui_example.pdf` is **RELEASE-ATTACH-ONLY, never committed** (SPEC §3.2c amended 2026-07-05; corpus budget ceiling 32 MiB enforced in the test). The reference manual PDF/HTML are attached on `manual-gui-v*` tags via `make release-attach` (`gh release upload`).

**File sizes:** Tier-A 152–217 KB each; Tier-B 4–140 KB each (61 files); Tier-C 50 PNGs + 98 transcript files; assembled `gui_example.pdf` = 11 MB, reference HTML = 4 MB.

### 3. Organization & naming conventions

- **Tier A (manual):** `screenshots/NN-<command-slug>.png` — 2-digit ordinal + kebab command name. Flat dir, README-only.
- **Tier B (form corpus):** `tests/snapshots/forms/<tab>-<subcommand>.png`, e.g. `mnemonic-bundle.png`, `md-decode.png`. One per subcommand; the stem is the census key.
- **Tier C (tutorial corpus):** `tut-<journey>-<NN>-<slug>-<pane>.png` where pane ∈ `form|modal|run`, plus `tut-…-<slug>.{stdout,stderr,exit}.txt`. Example: `tut-j1-01-bundle-single-sig-form.png`, `…-modal.png`, `…-run.png`. The authoritative list lives in `tests/tutorial/manifest-stems.txt` (regenerated + diffed by a census gate).
- **In the toolkit:** copies land at `figures/gui/<stem>.png`, `figures/tutorial/<name>.png`, `transcripts/tutorial/<name>.{cmd,out,err}`.

### 4. Determinism / reproducibility — is the set regenerated or hand-curated?

**Tiers B and C are fully regenerated and hard-gated** (not hand-curated). Tier A is hand-curated and un-gated.

- **Regeneration command (documented in-source):** `GUI_SNAPSHOTS=1 WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 UPDATE_SNAPSHOTS=1 cargo test --test gui_form_snapshots` (tutorial: `UPDATE_TUTORIAL_SNAPSHOTS`/`GUI_TUTORIAL_SNAPSHOTS=1`).
- **CI gate in the GUI repo** (`/scratch/code/shibboleth/mnemonic-gui/.github/workflows/build.yml`): two jobs, `snapshots` and `tutorial-snapshots`, each installs `mesa-vulkan-drivers` (lavapipe **software** Vulkan), sets `WGPU_BACKEND=vulkan`, runs the suite at the 0.6 dify threshold, plus a "ran-at-all census" (counts `*.new.png` == expected) and uploads `*.diff.png` on failure. Env-gated so a plain `cargo test` on a dev box with no rasterizer skips them loudly.
- **Cross-repo byte gate** (`/scratch/code/shibboleth/mnemonic-toolkit/docs/manual-gui/tests/lint.sh`, phases 9–11): `verify-figures-gui`, `verify-tutorial-figures`, `verify-tutorial-transcripts` **byte-compare** every committed toolkit figure against the pinned GUI checkout's `tests/snapshots/`, census **both directions** (orphan baseline fails, coverage gap fails, any byte drift fails — fail-closed). So the PDF's images are provably identical to the GUI's gated corpus.
- **Secret hygiene:** fixtures are blank; a `secret_flags_never_carry_a_default_value` test guarantees no secret material can enter a PNG.

Net: the screenshot pipeline is **fully deterministic and machine-regenerated**, with a software-rasterizer provenance lock so it's byte-stable across machines.

### 5. Transfer to a terminal TUI (ratatui) — what applies, what doesn't

**What does NOT transfer (the capture engine):** `egui_kittest` renders *egui widget trees* through *wgpu* to pixels. It is intrinsically tied to the egui/eframe framework. btctax's TUI is `ratatui = "0.29"` (`crates/btctax-tui`, `crates/btctax-tui-edit`), which paints a **character cell grid**, not a pixel scene. None of the wgpu/lavapipe/dify machinery applies. Tier A (manual OS window grabs) also doesn't fit a terminal well.

**What DOES transfer (everything around the engine), and btctax is already half-way there:**

- **The render seam already exists.** `crates/btctax-tui/src/tabs/tests.rs` already renders whole tabs to a `ratatui::backend::TestBackend` at a fixed **120×40** and inspects the resulting `Buffer` (`render_holdings/disposals/income/viewer`, etc.). That `TestBackend` buffer is the exact deterministic, headless analog of the egui_kittest harness — it just needs a **buffer → image (or SVG/ANSI)** step. This is the single missing piece.
- **The corpus + byte-gate + pandoc-assembly architecture transfers 1:1:** commit a PNG/SVG corpus, gate it byte-identical in CI, and assemble a **separate** PDF/HTML book with pandoc→xelatex (btctax can reuse the exact toolkit `manual-gui` Makefile pattern). The "keep screenshots in a separate file from CLI examples" instinct you have is precisely what mnemonic-gui does (`gui_example.pdf` is a distinct book from the reference manual, release-attach-only to keep it out of git).

**Concrete terminal-TUI capture options, in rough order of fit:**

1. **`ratatui` `TestBackend` `Buffer` → SVG/PNG (recommended; lowest friction).** You already produce the `Buffer` deterministically. Render each cell (glyph + fg/bg/modifier) to an SVG grid (trivial, dependency-light, crisp, tiny files, diff-friendly) or rasterize to PNG. This is the closest structural match to the egui_kittest approach: same "drive the app headlessly to a fixed-size surface, snapshot, byte-gate" discipline, fully deterministic, no terminal emulator or GPU needed. It reuses infra btctax already wrote.
2. **VHS (charmbracelet/vhs).** Scripted `.tape` files drive a headless `ttyd`+`ffmpeg` terminal and emit PNG/GIF. Deterministic-ish, great-looking, real terminal fonts/colors — but adds heavy external deps (ttyd, ffmpeg, a headless browser) and is less byte-stable than a `Buffer` render. Best if you want polished marketing frames rather than a byte-gated corpus.
3. **`termsvg` / `asciinema` → frame/SVG.** Record a real session, export a frame as SVG. More manual, weaker determinism; better for a quick README hero shot than a gated corpus.
4. **`insta` snapshot testing of the `Buffer` text.** Not an image, but worth pairing: gate the textual buffer with `insta` (cheap, no rasterizer) and separately render the committed images for the doc — mirrors mnemonic-gui's split between always-run text gates and the env-gated pixel corpus.

**Recommended shape for btctax:** extend the existing `TestBackend` tests into a `--test tui_snapshots` corpus that renders each tab/screen `Buffer` to committed SVG (or PNG) under e.g. `crates/btctax-tui/tests/snapshots/`, gate it byte-identical in CI (the analog of the `snapshots` job), then assemble a **separate** `tui_gallery` book with the same pandoc→xelatex Makefile pattern mnemonic-gui's toolkit uses — kept out of the CLI-examples doc exactly as you intended.

### Key file references (all absolute)

- Framework proof: `/scratch/code/shibboleth/mnemonic-gui/Cargo.toml` (eframe/egui/wgpu; `egui_kittest` dev-dep L112)
- Manual screenshots: `/scratch/code/shibboleth/mnemonic-gui/screenshots/0{1,2,3}-*.png`; README §Screenshots (`README.md:75-88`)
- Form snapshot generator + determinism doc: `/scratch/code/shibboleth/mnemonic-gui/tests/gui_form_snapshots.rs`; corpus `/scratch/code/shibboleth/mnemonic-gui/tests/snapshots/forms/` (61)
- Tutorial capture harness: `/scratch/code/shibboleth/mnemonic-gui/tests/gui_tutorial_snapshots.rs`; corpus `/scratch/code/shibboleth/mnemonic-gui/tests/snapshots/tutorial/` (50 + transcripts); manifest `/scratch/code/shibboleth/mnemonic-gui/tests/tutorial/manifest{.rs,-stems.txt}`
- Headless structural (text, not pixels) render binary: `/scratch/code/shibboleth/mnemonic-gui/src/bin/gui_render.rs` + `src/form/render_emit.rs`
- CI gates: `/scratch/code/shibboleth/mnemonic-gui/.github/workflows/build.yml` (`snapshots`, `tutorial-snapshots` jobs)
- **Doc assembly (the distributed artifact):** `/scratch/code/shibboleth/mnemonic-toolkit/docs/manual-gui/` — `Makefile`, `AUTHORING.md`, `pandoc/`, `tests/lint.sh` (phases 9–11 byte gates), `Dockerfile.build`; built PDFs in `build/gui_example.pdf` (11 MB) + `build/m-format-gui-manual.html` (4 MB)
- Tutorial spec: `/scratch/code/shibboleth/mnemonic-toolkit/docs/manual-gui/design/SPEC_gui_example_tutorial.md`
- **btctax's existing render seam (reuse this):** `/scratch/code/bitcoin_tax/crates/btctax-tui/src/tabs/tests.rs` (`TestBackend::new(120, 40)` → `Buffer`); TUI crates `crates/btctax-tui`, `crates/btctax-tui-edit` (`ratatui = "0.29"`). No screenshot/VHS/asciinema infra exists in btctax today.

---

## Report 3 — method philosophy, bug-discovery payoff, and btctax adaptation sketch

**Scope note.** Read-only. Focused on the method's *philosophy, process, bug-discovery payoff, and btctax adaptation*. Reports 1 & 2 own the CLI-PDF and GUI-screenshot mechanics.

A crucial framing correction up front: what the constellation built is **not one thing but three coupled cycles**, and the bug-discovery payoff came overwhelmingly from the *GUI-tutorial* leg, not the CLI-PDF leg:

1. **`Examples.pdf`** — a pre-existing CLI verbatim-I/O tutorial (`.examples-build/gen.sh` real-binary capture → pandoc/xelatex PDF). The recent cycle `examples-pdf-un-ci-gated` *modernized + CI-gated* it. This leg surfaced **rot/consistency** bugs.
2. **`gui_example.pdf`** — a *new* GUI whole-window-screenshot tutorial mirroring the same wallet journeys (cycle `gui_example_tutorial`), built with egui_kittest. This leg is where **writing the worked journeys surfaced real UX/workflow defects**.
3. **`tutorial_surfaced_fixes_batch`** — a follow-on cycle that *fixed the defects the tutorial exposed*. This is the concrete proof that docs-authoring is a bug-discovery instrument.

### 1. The process they used to BUILD these docs

**Yes — full standard-workflow spine, no shortcuts, even though "it's just docs."** This is the single most transferable finding: they treated a docs artifact with the exact same gate discipline as a funds-critical code change.

Evidence, for the `examples-pdf-un-ci-gated` cycle (paths under `/scratch/code/shibboleth/mnemonic-toolkit/`):

- **Banked cycle-prep recon** → `cycle-prep-recon-examples-pdf-modernize-and-gate.md` (22 KB): a read-only P0 strict-gate recon that verified every `gen.sh:` line citation against live source and the live binary *before* any spec was written. It even distinguished "the doc is a three-era patchwork" as an *empirical* finding, not a hypothesis.
- **SPEC** → `design/SPEC_examples_pdf_modernize_and_gate.md`, authored by the Fable model, carried to **R0-GREEN (0C/0I)**. Review persisted at `design/agent-reports/examples-pdf-spec-r0-round-1.md`.
- **PLAN** → `design/IMPLEMENTATION_PLAN_examples_pdf_modernize_and_gate.md`, opus R0-GREEN (`examples-pdf-plan-r0-round-1.md`). A separate architect **ruling** was persisted for one thorny sub-decision: `examples-pdf-branch-protection-ruling.md` (24 KB, on the required-check/wedge trap).
- **Phased, test-first implementation** — P1 (gen.sh modernization), P2 (the CI gate + golden), P3 (regen + ship). Each phase had its own opus R0 (`examples-pdf-p1-r0-round-1.md`, `examples-pdf-p2-r0-round-1.md`), and the "tests" for a docs cycle were **determinism proofs** (double-regen byte-identity; regen under varied `$HOME`; diff ⊆ enumerated change classes).
- **Mandatory post-implementation whole-diff review** → `examples-pdf-postimpl.md`, in which the reviewer *independently re-ran everything*: rebuilt the binary, regenerated the golden, re-proved determinism, extracted **both** PDFs and diffed the entire funds surface (xpubs/addresses/checksums/fingerprints — 30/5/15/3 tokens, all "identical set"), perturbed the gate to observe red, and dry-ran the branch↔master merge. Verdict GREEN 0C/0I.

The `gui_example_tutorial` and `tutorial_surfaced_fixes_batch` cycles followed the identical structure (see their plan docs' "REVIEW CADENCE" tables — every gate names a persisted `design/agent-reports/*.md` artifact, all reviews opus, all persisted verbatim *before* the fold, folds re-enter the loop).

**Review/gating posture worth stealing:**
- A **STOP ledger** in both spec and plan: explicit user-decision tripwires ("S1: any address/xpub/descriptor/checksum change ⇒ halt, escalate; the funds surface is expected byte-stable"). This is how they made a *presentational* docs cycle safe: they pre-declared the funds surface immutable and gated on it.
- An **Honesty section** ("what the gate catches and what it does NOT") in both docs — they were explicit that gate A catches regen-drift but *not* narration-truth, and filed the deeper fix (gate B) as a tracked follow-up rather than pretending it was solved.
- **Determinism was treated as the load-bearing risk**, not an afterthought — the plan's GOTCHAS lead with "do not add `$(date)`, `$USER`, `hostname`, `$PWD`, or an unpinned locale anywhere."

### 2. Docs-as-bug-discovery, concretely

Two distinct discovery mechanisms showed up, and they are different in kind.

#### 2a. The CLI-PDF leg surfaced *rot / internal-inconsistency* bugs
From `cycle-prep-recon-examples-pdf-modernize-and-gate.md` and the postimpl review:
- **The narration went false.** The doc said "*Each card is printed twice: once unbroken, once grouped*" — a behavior that changed at v0.56.0 (now printed once). The prose became a "time-bomb": flatly false, undetectable except by regenerating and reading. This is the archetype: **prose that describes output rots silently when the output changes.**
- **A three-era patchwork.** The committed PDF simultaneously contained 0.55.3-era bundle blocks, a stale `--help` capture *missing two shipped subcommands* (`gen-man`, `word-card`), and `v0.73.3` install pins — one document, three binary/repo states. Nobody noticed until someone tried to regenerate it.
- **Environment leakage into "canonical" output** — author-machine paths (`/scratch/code/shibboleth/...`, `/home/bcg/.cargo/bin`) were baked into the committed examples, which no user would ever see.

#### 2b. The GUI-tutorial leg surfaced *real UX/workflow defects* — the high-value payoff
The systematic catalog is `docs/manual-gui/design/agent-reports/tutorial-workaround-audit.md` — an audit of every place the tutorial author had to "route around" the product to make a journey work. Classified findings:

- **F1 (FIXED): export-wallet `--template` trap → the descriptor arm was UNREACHABLE from the GUI.** The form always materialized `--template=bip44`, which is mutually exclusive with `--descriptor`, so `--descriptor` was permanently disabled. A whole documented capability could not be reached through the UI. Fix: a `(none)` sentinel row on the template dropdown (`EXPORT_WALLET_TEMPLATES`).
- **R1 (drove a new cycle): restore `--template` refuses in `--md1` mode (exit 2).** Six tutorial restore steps had to dodge this by selecting a *multisig* template (inert in md1 mode). Worse — a **doc-integrity finding** — the prose *disguised* the workaround as "for consistency" rather than disclosing it was dodging a refusal. This directly produced `SPEC_restore_template_none_affordance.md` and the `(none)` affordance fix.
- **The secret-reveal (👁) eye toggle was born from a documentation need.** To *show a reader what to type* into a secret field, the tutorial needed a screenshot with the plaintext demo phrase visible — but every secret field was unconditionally masked, so there was **no way to depict the input**. That gap produced an entire feature: a hold-to-reveal / bounded-latch eye toggle (`SPEC_gui_secret_reveal_toggle.md`), display-only, masked everywhere else. Documenting the workflow *demanded* a UI affordance that didn't exist.
- **B1 (filed LOW/UX): Path flags have no file picker** — `--descriptor-file`, `--spec`, `--output` etc. are bare text inputs; a user must type an exact filesystem path. Surfaced because the tutorial had to drive those fields.
- **C2 (latent-UX note): no in-app output→input chaining** — a real user must manually copy md1 chunks from the output pane into restore's rows.

Both fixes (reveal + restore `(none)`) were then shipped together in `IMPLEMENTATION_PLAN_tutorial_surfaced_fixes_batch.md`, and the FOLLOWUPS entry `examples-pdf-un-ci-gated` (`design/FOLLOWUPS.md:248`) records the whole resolution.

#### Why verbatim-I/O examples expose what unit tests don't
The recon/spec/audit make the argument concretely; distilled:

1. **They exercise the *assembled* surface, in sequence, as a user experiences it.** A unit test asserts one function's output in isolation. A worked journey composes real commands end-to-end — so it hits **cross-flag interactions** (the `--template`/`--descriptor` mutex that makes an arm unreachable), **mode-dependent refusals** (single-sig template in md1 mode), and **affordance gaps** (no way to reveal, no file picker, no output→input chaining) that no isolated unit ever probes. Unit tests confirm the arm *works if you can reach it*; the tutorial asks *can a user reach it at all?*
2. **They put prose next to output, so narration-truth becomes checkable.** "Each card is printed twice" is a claim about adjacent output. No unit test asserts prose; regenerating the doc and reading it does.
3. **The author is forced to be the user.** Every "route-around" the author performs is a UX papercut made visible — the audit method is literally "catalog every workaround and classify it (bug / harness-artifact / intentional)."
4. **Determinism forcing-functions expose hidden nondeterminism/environment-coupling** (path leakage, locale, `$HOME`) that only matters when you try to make output reproducible — which unit tests, run on one machine, never force.

### 3. Constellation doc layout

The constellation is a set of sibling crates under `/scratch/code/shibboleth/`: `mnemonic-toolkit` (umbrella + the four CLIs `md`/`ms`/`mk` routed through one `mnemonic` binary), `mnemonic-gui`, `mnemonic-key`, `mnemonic-secret`, `mnemonic-engrave`. The docs strategy is **hybrid, and deliberately so**:

- **The CLI examples doc is a single central artifact.** `Examples.pdf` (committed under `mnemonic-toolkit/docs/`) is *one* document covering the whole umbrella CLI — because everything routes through the one `mnemonic` binary, so there is one command surface to document. Its generator (`.examples-build/gen.sh`) invokes only `mnemonic`; it never calls the sibling CLIs directly. The gate (`examples.yml`) is correspondingly one job.
- **A parallel family of "books" is per-audience, not per-crate.** `mnemonic-toolkit/docs/` holds `manual/`, `technical-manual/`, `quickstart/`, and `manual-gui/` — each its own pandoc-built book with its own `tests/verify-examples.sh` transcript-gate and `pandoc/filters/include-transcript.lua` ("prose == .out by construction"). These are the **stricter gate model** (gate B) the Examples.pdf cycle deferred toward.
- **The GUI tutorial lives as a *sibling book inside* `manual-gui/`, split across two repos.** `mnemonic-gui` owns the *rendering* (the egui_kittest harness `tests/gui_tutorial_snapshots.rs`, the `tests/tutorial/` manifest+fixtures, the pixel threshold), and `mnemonic-toolkit/docs/manual-gui/` owns *byte-copied intake* (`figures/tutorial/*.png`, `transcripts/tutorial/*`) + the pandoc build → `gui_example.pdf`. The GUI repo tags a release; the toolkit book pins that tag (`pinned-upstream.toml`) and byte-censuses the copied artifacts. This "GUI repo renders, docs repo censuses, no rasterizer in docs CI" split is a deliberate pattern (`SPEC_gui_example_tutorial.md` §3).
- **Committed vs release-attach is decided by weight.** `Examples.pdf` (215 KB, pure text) stays **committed** *and* release-attached. `gui_example.pdf` (~50 whole-window screenshots, ~32 MiB corpus) is **release-attach only** — the screenshots are not committed (`README.md:89`).

**Lessons for btctax's multi-crate workspace:**
- btctax is *one product with one binary family* (`btctax` CLI + `btctax-tui` + `btctax-tui-edit`), all sharing one vault/engine — structurally like the mnemonic umbrella, not like a collection of independent libraries. So the right shape is **one combined "btctax examples" CLI doc**, not per-crate docs. This matches btctax's existing choice: `xtask/src/docs.rs` already emits *one* man-page tree + a single merged `btctax-manual.pdf` (`make bundles`).
- Follow the mnemonic weight rule: a **text CLI examples doc → committed**; a **screenshot/terminal-capture TUI doc → its heavier assets stay uncommitted / release-attached** (or, better for btctax, captured as *text* — see §4, this sidesteps the weight problem entirely).
- Keep the CLI doc and the TUI doc **separate artifacts** (as mnemonic keeps `Examples.pdf` vs `gui_example.pdf`) — they have different determinism stories and different capture tooling.

### 4. Concrete btctax adaptation sketch

btctax's existing infra and constraints (grounded, all under `/scratch/code/bitcoin_tax/`):

- **Docs pipeline is groff/roff, not pandoc.** `crates/xtask/src/docs.rs` renders one `clap_mangen` roff page per subcommand (55 pages: ~54 CLI + updater, plus **two hand-authored** roff pages `btctax-tui.1` / `btctax-tui-edit.1`), committed under `docs/man/`, and `make docs`/`make bundles` shell them through `groff -Tpdf` + `gs`. Determinism is already a first-class concern here: `gen_docs_is_deterministic` asserts committed pages match a fresh render (fails on stale docs); `manpage_covers_every_subcommand` fails if a new subcommand ships without a page. **There is no pandoc/xelatex/LaTeX in this repo.**
- **Synthetic-data infra already exists.** `crates/btctax-cli/tests/fixtures.rs` has named builders (`coinbase_buy_sell_send`, `coinbase_two_lot_donation`, `income_fmv_missing_batch`, …) that construct vaults from synthetic exchange CSVs. Integration tests build a tempdir vault with a throwaway passphrase (`Passphrase::new("pw")`), run commands, and read back CSVs.
- **★ The TUI is a deterministic TEXT grid, and there is already a harness for it.** `crates/btctax-tui/src/tabs/tests.rs` renders tabs to a `ratatui::backend::TestBackend` (120×40) and inspects the `Buffer`. **This is a decisive advantage over the mnemonic-gui pixel track:** btctax's TUI can be captured as *verbatim text*, not screenshots — no rasterizer, no lavapipe, no dify-0.6 threshold, no cross-GPU-backend drift. The hardest, most fragile part of the mnemonic method (whole-window pixel determinism, the P0 spike, the 32-MiB budget) **simply does not apply to a text TUI**.
- **PII posture is already hostile to real data.** `.gitignore` ignores `vault*`, `*-snapshot.*`, `snapshot.*`, and all decrypted/exported data; there is a `scripts/pre-push` PII hook + `scripts/pii-scan-generic.sh`. So "synthetic-only in committed docs" is already the house rule, not a new constraint.
- **CI is a single lightweight `ci.yml`** (vs mnemonic's 16 workflows).

#### Proposed artifacts

**Artifact 1 — `btctax` CLI verbatim-I/O examples doc** (the `Examples.pdf` analogue).
A generator (call it `xtask docs examples` or a `.examples-build/gen.sh`-style script) that runs a **scripted end-to-end journey** against a freshly-built `btctax` binary on a **synthetic vault** and captures verbatim `$ cmd` + output blocks: `init` → `import <synthetic coinbase/river CSVs>` → `report` / `report --tax-year 2025` → a `reconcile` decision (e.g. `select-lots`, `classify-inbound-*`) → `export-snapshot` → `export-irs-pdf` → `what-if sell`/`harvest` → `optimize run`. This is the natural btctax journey and it maps directly onto the recon/spec/audit method: each command in sequence, real deterministic output.

- **Fit to existing infra:** two viable levels, mirroring the mnemonic "gate A vs gate B" choice:
  - *Level A (recommended floor):* a generator script + a **committed golden text file** (`examples.md` or `examples.txt`) + a CI step that regenerates and `git diff --exit-code`s against the golden. Cheapest possible rot-stopper; catches the entire "narration/output drift" class with one diff. Render to PDF via the **existing groff pipeline** (wrap the captured text in a roff `.nf/.fi` verbatim block and feed `xtask` PDF rendering) — *no new pandoc/xelatex dependency*.
  - *Level B (stronger, more work):* per-command `.cmd`/`.out` golden pairs the way `docs/manual/tests/verify-examples.sh` does, so stable prose includes gated output blocks. Higher fidelity, larger build-out. Defer unless the doc grows.
- **Determinism levers btctax must pin** (the mnemonic determinism contract, adapted): fixed passphrase via `BTCTAX_PASSPHRASE`; `TZ=UTC`, `LC_ALL=C`; **a fixed price cache / `--as-of` clock** (tax numbers depend on FMV/prices — the analogue of mnemonic's "fixed BIP-39 vectors"; btctax already has `btctax-adapters/data/btc_usd_daily_close.csv` as a deterministic price source); scrubbed `$HOME`/paths; and a **fixed `EventId` scheme** (event references appear in output — confirm they're deterministic, see risks). A committed **fixed synthetic vault or fixed synthetic import CSVs** is the "hard-coded test vector" equivalent.

**Artifact 2 — `btctax-tui` / `btctax-tui-edit` terminal-capture doc** (the `gui_example.pdf` analogue, but *far* simpler).
Drive the two TUIs through the same journey and capture each `TestBackend` `Buffer` as **verbatim text frames** (optionally rendered as ANSI/box-drawing in a monospace PDF page, or committed as `.txt` goldens). Because it's text, this doc can be **committed and CI-gated identically to Artifact 1** — the mnemonic "screenshots are heavy → release-attach-only" tradeoff evaporates.

- **Fit:** extend the existing `TestBackend` KAT harness (`btctax-tui/src/tabs/tests.rs`) into a "tutorial driver" that steps through tabs (Holdings/Disposals/Income/Tax/Forms/Compliance) and, for `btctax-tui-edit`, through a reconcile flow, snapshotting each frame. Gate = `UPDATE`/golden-diff, same as the CLI leg.
- This is also where **btctax's own UX bugs will surface** — the whole point. Driving the editor through a real reconcile journey is exactly the exercise that found the mnemonic `(none)` affordance and the reveal-toggle gaps.

#### Rough phase breakdown (per btctax's STANDARD_WORKFLOW)
- **P0 — cycle-prep recon + brainstorm:** inventory which journeys/commands to cover; decide committed-vs-attach, text-vs-PDF, determinism strategy (esp. the price/clock/EventId pinning), and whether TUI capture is text-golden or PDF. Resolve the open questions in §5.
- **P1 — CLI examples generator + golden + determinism proofs** (double-run byte-identity; cross-`$HOME`/host). Wire into `xtask` + `make`. The "tests" are the determinism proofs, exactly as the mnemonic P1 gate.
- **P2 — CI gate:** extend `ci.yml` with a regenerate-and-`git diff --exit-code` job (btctax has one workflow, so this is a small addition, not a new 16-workflow governance problem). Born-green + a negative (perturb one byte → red) proof.
- **P3 — TUI text-capture doc** (its own golden + gate), driven through both binaries.
- **P4 — regen + ship + a `tutorial-workaround-audit`-style sweep:** deliberately catalog every workaround the journey author had to perform and classify each (bug-to-file / harness-artifact / intentional). This is the deliverable that pays for the whole exercise — file the discovered UX bugs into `FOLLOWUPS.md`.

#### btctax-CLI vs btctax-TUI, distinguished
- **CLI (`btctax`, clap):** deterministic-output-able today. Output is text; the only nondeterminism is prices/clock/EventIds/paths — all pinnable. Closest analogue to `Examples.pdf`. Reuse `fixtures.rs` synthetic vaults.
- **TUI (`btctax-tui`, `btctax-tui-edit`, ratatui):** interactive, but **captured as deterministic TEXT via `TestBackend`** — dramatically easier than mnemonic's egui pixel capture. No rasterizer risk. The interactivity (keystrokes, tab switches, edit flows) is driven programmatically through the existing harness. Tax numbers and any PII in captures **must be synthetic** (already enforced by `.gitignore` + PII hook); use the `fixtures.rs` world-known synthetic data as the "public test vectors" analogue.

### 5. Open questions for the brainstorm (the human decisions)

1. **Scope / which journeys.** One canonical end-to-end journey (init→import→reconcile→report→export→what-if→optimize), or several (single-buyer, multi-lot donation §170(e), self-transfer reconcile, income-with-missing-FMV)? The mnemonic doc ran 5 wallet journeys (J1–J5); more journeys = more surface exercised = more bugs found, but more to keep deterministic and gated.
2. **Which of the ~54 subcommands get worked examples vs just a man page.** The man pages already cover the full surface; the examples doc is editorial about *which* commands earn a narrated worked example. (mnemonic's honesty §d: the gate never *demands* an example per subcommand — that stays editorial.)
3. **Determinism strategy — the load-bearing decision.** Prices, the "current date"/tax-year clock, and **EventId generation** all feed captured output. Are EventIds deterministic for a fixed synthetic vault today, or do they embed timestamps/randomness? (Flagged uncertainty — I did not verify EventId construction; this must be checked in recon, because if EventIds are nondeterministic the whole golden-diff approach needs an EventId-pinning shim first.) Also: fixed price source (`--as-of` + the committed daily-close CSV), `TZ`/`LC_ALL`, scrubbed paths.
4. **Real vs synthetic data/PII.** Confirm synthetic-only (the house rule) and pick the fixed synthetic corpus to serve as btctax's "public test vectors." No real exchange exports, no real vault, ever committed. Decide whether to commit a **fixed synthetic vault** or **fixed synthetic import CSVs** (regenerate the vault in CI — cleaner, since vaults are encrypted/gitignored).
5. **CI-gate or not, and how hard.** mnemonic went WIDE + REQUIRED (fires on `crates/**` so any output-changing PR reds *in that PR*) and hit a required-check/wedge trap that needed a dedicated architect ruling. btctax has one `ci.yml` and simpler governance — decide: advisory vs required; narrow (`docs/**`) vs wide (`crates/**`, leading indicator) triggers. Recommendation: start with a **regenerate-and-`git diff` gate at level A**, advisory first, promote to required once born-green.
6. **Tooling: reuse groff, or add pandoc?** btctax has *no* pandoc/xelatex. Reusing the existing groff/roff → PDF pipeline (`xtask` + `make bundles`) avoids a new heavyweight toolchain and keeps determinism guarantees the repo already has. Adding pandoc buys nicer typography + the `include-transcript.lua` "prose==output" model but is a real dependency. Decide up front.
7. **TUI capture format: committed text goldens, or rendered PDF?** Text goldens are simplest and gate like code; a PDF is a nicer deliverable. Because `TestBackend` yields text, btctax can have *both* cheaply (text golden is the gated source; PDF is a render of it) — unlike mnemonic, which was forced into uncommitted heavy PNGs.
8. **Is the primary goal the doc, or the bug-hunt?** Framing matters for scope. If the payoff you want is the `tutorial-workaround-audit` (finding btctax's UX papercuts — the analogue of the reveal-toggle and `(none)` affordance), then a *deliberately adversarial journey author + a workaround-classification sweep* (P4 above) should be an explicit, budgeted deliverable, not a side effect.

**Single most important adaptation insight:** btctax's TUI is a *text* interface with an *existing* `TestBackend` harness, so the expensive/fragile half of the mnemonic method (whole-window pixel determinism) is free here — the entire method collapses to "run journeys against synthetic data, capture verbatim text, commit as goldens, diff in CI, and catalog every workaround." The determinism work shifts entirely onto **prices/clock/EventIds**, which is where recon should concentrate.
