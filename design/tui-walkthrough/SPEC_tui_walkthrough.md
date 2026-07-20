# SPEC — TUI screen-based walkthrough (visual tutorial PDF)

Status: DRAFT for review (brainstorm complete; awaiting spec review → plan → build).
Owner decisions captured (2026-07-19): purpose = user-facing visual tutorial;
CLI-only steps = hybrid (narrate setup, capture real TUI screens for the rest);
scope = all nine journeys J1–J9 (full parity with `docs/examples/examples.md`);
reproducibility = CI-gated goldens.

## 1. Goal

Produce a **visual, screen-by-screen tutorial PDF** — `btctax-tui-walkthrough.pdf` —
that mirrors the nine worked-example journeys (J1–J9), but instead of CLI
command+output blocks it shows the **actual terminal UI** doing the work: real,
deterministic screen captures of the `btctax-tui` viewer and `btctax-tui-edit` editor,
each with a caption explaining what the filer is doing and seeing.

It is a companion to `btctax-examples.pdf` (the CLI journeys), aimed at a newcomer who
wants to *watch it happen in the UI*.

## 2. Non-goals

- Not a replacement for `btctax-examples.pdf` (the CLI reference) — a companion.
- Not a live/interactive demo — it is a static captured-screens PDF.
- Not a redesign of the TUI — it captures the UI as it is today.
- The gated artifact is the **captured goldens** (reproducible); the PDF is a
  convenience render (like the existing TUI-screens PDF).

## 3. The core constraint (and how the hybrid resolves it)

The two TUI binaries cover only part of what a journey does:

- **CLI-only** (no TUI surface): `init`, `import`, and running `report`/`optimize`/
  `export` *from scratch*.
- **TUI-visible**: viewing computed results (viewer: Holdings / Disposals / Income /
  Tax / Forms / Compliance tabs), reconciling (editor: classify-inbound-income /
  -self-transfer / -gift, match-self-transfers, select-lots, void, reclassify, …), and
  authoring the full return (editor: the tax-inputs form). The viewer also has a
  what-if overlay and an export modal.

**Hybrid resolution:** each journey is rendered as a short **narrated CLI-setup
preamble** (the `init`/`import` steps, in prose — optionally echoing the CLI command)
followed by a sequence of **captioned TUI screen captures** for every step the TUI can
actually perform, ending with the viewer showing the journey's computed result. Where a
journey is CLI-heavy with little TUI surface (e.g. J1), the walkthrough is honestly
mostly a narrated setup plus the viewer's result screens — this is disclosed per
journey, never faked.

## 4. Mechanism — reuse the existing golden capture system

The TUI already captures screens deterministically and headlessly: `to_golden` is a
`pub` lib fn shared by both crates (`crates/btctax-tui/src/capture.rs:29`), and the
editor's `edit-classify-confirm-modal` golden is produced TODAY by exactly the driver
pattern this feature relies on — seed a temp vault, real unlock, scripted `KeyEvent`s
through the real `handle_key`, pin the clock + displayed vault path, render into
`TestBackend::new(120, 40)`, serialize with `to_golden`
(`crates/btctax-tui-edit/src/main.rs:14195-14242`). A frame is a pure function of
`(code, synthetic state)`. The walkthrough is therefore **mostly new capture *drivers*,
not new machinery** — with the four constraints below made explicit (r1 review).

### 4.1 Shared corpora — a real refactor this spec owns (r1 I-1)

"J-parity" requires the SAME corpora the examples journeys use, but those are private
`const`s (`J1_CSV … J9_CSV`) inside the bin-only `xtask` crate
(`crates/xtask/src/examples.rs:203-291`), which neither TUI crate can depend on. So the
first deliverable is a refactor: **hoist the eleven CRLF corpus consts** (J6 and J8 each
split into two per-exchange corpora) **plus the `J6_FULLRETURN_TOML` fixture** (which
moves SAME-crate, retiring the cross-crate `include_str!` examples.rs flags as its M-5
exception) **into a shared `testonly` module in `btctax-cli`** (which `xtask`,
`btctax-tui`, and `btctax-tui-edit` all already depend on; the `btctax-forms`
`pub mod testonly` precedent applies — a plain pub module). `xtask`
then imports them from there (its golden must stay byte-identical), and the walkthrough
drivers import the same consts — so parity is *structural* (one source of truth), not a
copy. A driver writes the corpus to a tempdir and runs the real adapter ingest via the
pub `btctax_cli::cmd::import::run` (`crates/btctax-cli/src/cmd/import.rs:10`).

### 4.2 Cross-crate capture architecture (r1 I-2 — decided)

The viewer's `App`/`handle_key`/full-frame `draw` are deliberately `pub(crate)`
(`crates/btctax-tui/src/app.rs:137`) and `btctax-tui-edit` is bin-only, so a single
driver cannot capture both an editor flow AND full viewer chrome. **Decision: keep the
encapsulation (no visibility relaxation).** Each journey's frames are captured in
whichever crate owns that screen:

- **Editor frames** (Browse, classify/reconcile/select-lots/tax-inputs flows + confirm
  modals) — captured in `btctax-tui-edit`'s `#[cfg(test)]` emit test, as today.
- **Viewer frames** (the six tabs' full chrome, the export modal, the what-if overlay) —
  captured in `btctax-tui`'s `#[cfg(test)]` emit test, as today.

The two halves of one journey **converge by construction** on a single shared
per-journey fixture (living with the hoisted corpora in `btctax-cli::testonly`): the
corpus + the ordered list of reconcile decisions. The editor half applies them via its
flows; the viewer half re-seeds the same corpus and **replays the same decisions via
`btctax-cli` calls** (`Session` + the `reconcile`/`import` fns `btctax-tui` already
depends on) before capturing its result tabs. One fixture, applied two ways — so the
viewer's "after" state provably matches the editor's mutations. The §8 "xtask-driven
capture harness" alternative is **struck**: xtask can *assemble* committed goldens, it
can never *capture* them (it depends on neither TUI crate).

### 4.3 Golden location & gating

Each captured frame is a **committed golden** at
`docs/examples-tui-walkthrough/<journey>/<NN>-<slug>.txt` (same glyph-grid + style-run
format as today; this path does not collide with the existing `docs/examples-tui/*.txt`
glob), byte-checked in CI by a `*_goldens_match_committed`-style test in its owning
crate (`#[cfg(unix)]`, as today). Regeneration is an `#[ignore]` emit test per crate,
mirroring `emit_btctax_tui_goldens` / `emit_btctax_tui_edit_goldens`.

## 5. Generation & rendering (r1 I-4, M-2)

The two existing awk pipelines are DISJOINT: `man-wrap.awk` renders Markdown *prose*
(examples.md → PDF); `tui-wrap.awk` renders one glyphs+style-runs `.txt` golden per
section, *no prose*. The walkthrough interleaves prose+captions with colorized frames,
which neither does alone — so it needs a thin new assembler + a manifest:

- An xtask subcommand `cargo run -p xtask -- tui-walkthrough` emits a **committed
  ordering MANIFEST** — `docs/examples-tui-walkthrough/walkthrough.md`, the byte-gated
  artifact — containing, in journey order: the narrated CLI-setup prose, each caption,
  and a **reference (path) to each frame golden `.txt`**. Captions + narration live ONLY
  here (M-2: a single owner), so a wording edit never touches a TUI crate; the drivers
  emit only the frames the manifest names.
- `make tui-walkthrough` walks the manifest: prose/caption chunks → `man-wrap.awk`;
  frame references → `tui-wrap.awk` on the referenced `.txt`; concatenated in order into
  ONE groff stream → the colorized landscape `docs/pdf/btctax-tui-walkthrough.pdf`
  (reusing the proven geometry `-dpaper=letterl -P-pletterl -rLL=10i -rPO=0.4i`, 9pt
  grid, and the `\m[` colorization guard).
- Gating: a `regen == committed` test on the manifest (precedent
  `examples_golden_matches_committed`, `xtask/src/examples.rs:1370`) + the per-frame
  golden gates in the TUI crates together pin the whole artifact; the advisory CI
  `examples` job proves `make tui-walkthrough` emits a `%PDF`.

**As-built (amended folding PoC review C-1).** The PoC implemented this contract with two
deliberate simplifications, kept because they preserve the property this section exists to
guarantee — *no screen silently drops out of, or dangles in, the walkthrough* — with less
machinery:

- **The manifest is hand-authored, per journey**, at
  `docs/examples-tui-walkthrough/<journey>/manifest.txt` (grammar: `PROSE <roff>` /
  `FRAME <file.txt> <caption>`), rather than a single xtask-emitted `walkthrough.md` built
  from Rust-literal captions. The single-owner property (M-2) is unchanged: prose + captions
  still live in exactly one place per journey, and a wording edit still never touches a TUI
  crate. Prose is authored as roff, so `make tui-walkthrough` (via
  `docs/examples-tui-walkthrough/assemble-walkthrough.sh`) emits `.PP` directly instead of
  routing prose through `man-wrap.awk`.
- **Because nothing regenerates a hand-authored file, the manifest gate is an INTEGRITY test,
  not a `regen == committed` test:** xtask's `walkthrough_manifests_valid_and_complete`
  (`crates/xtask/src/examples.rs`) validates the grammar AND asserts a **bijection** between
  each manifest's `FRAME` references and the frame goldens on disk. A reference with no golden
  (a typo/rename) reds it; a golden with no reference — the residue of a silently dropped
  `FRAME` line — also reds it. Combined with the per-frame `*_walkthrough_goldens_match_committed`
  gates (which pin each frame to the real TUI) and the CI `%PDF` proof, the whole artifact is
  pinned, as this section requires. No `cargo run -p xtask -- tui-walkthrough` subcommand is
  needed under this design and none was added.

## 6. Journey → screen mapping (all nine; hybrid)

For each journey: the narrated CLI setup, then the TUI screens captured. (Exact frame
list is refined during the plan; this is the intended shape.)

| J | Narrated setup | TUI screens (captured) | TUI surface |
|---|----------------|------------------------|-------------|
| J1 single buyer | init, import, buy/sell | viewer: Holdings → Disposals → Tax → export modal | thin (mostly viewer result) |
| J2 §170(e) donation | init, import | editor: set-donation-details flow (`d`); viewer: Forms (8283) → Tax | medium |
| J3 self-transfer (classify) | init, import, verify (blocker) | editor: Browse (blocker) → classify-inbound-self-transfer flow → confirm modal → cleared Browse; viewer: Holdings | **rich** |
| J4 income + SE | init, import | editor: reclassify-income flow → confirm; viewer: Income → Tax (SE) | medium |
| J5 optimize + clock | init, import | editor: profile form (`p`), method-election (`e`), optimize-accept flow (`z`, surfaced); viewer: what-if overlay | medium |
| J6 full 1040 | init, import | editor: tax-inputs authoring form (sections) → commit; viewer: Forms → Tax | **rich** |
| J7 manual income `--fmv` | init, import, verify (blocker) | editor: classify-inbound-income flow (kind + FMV) → confirm; viewer: Income → Tax | **rich** |
| J8 match-self-transfers | init, import both, verify | editor: match-self-transfers preview → confirm (RELOCATE); viewer: Holdings (BALANCED) | **rich** |
| J9 select-lots | init, import | editor: select-lots flow (pick lots) → confirm; viewer: Disposals (per-disposal) / Compliance | **rich** |

## 7. Determinism & reproducibility

- Pinned `Clock::Pinned` per driver; fixed displayed vault path; fixed 120×40 backend.
- **`BTCTAX_PRICE_CACHE` pinned to a nonexistent file inside each emit/gate test** (r1
  I-3) — otherwise the real unlock path layers the developer's LIVE local price cache
  (`crates/btctax-tui/src/unlock.rs:171` → `default_cache_path()`, `dirs::data_dir()/…`),
  which would perturb exactly the frames this feature adds (J4/J7 income/FMV, J5 what-if
  prices). The examples generator already pins this (`examples.rs:96`). Safe under
  nextest's process-per-test model (which `make check`/CI use); plain in-process `cargo
  test` threading is the documented caveat.
- Corpora imported from the single shared `btctax-cli::testonly` home (§4.1), so parity
  is structural (one source of truth), not a copy.
- `#[cfg(unix)]` on the byte-exact golden gates (rendered paths carry a separator).
- CI: the golden byte-checks (in the `test` job) + a `make tui-walkthrough` PDF-render
  proof (in the advisory `examples` job), matching the `examples-tui` pattern.

## 8. Deliverables

1. **Refactor (§4.1)**: hoist the eleven corpus consts + the `J6_FULLRETURN_TOML`
   fixture + a per-journey fixture (corpus + ordered decision list) into
   `btctax-cli::testonly`; repoint xtask's examples generator at it (its golden must
   stay byte-identical).
2. **Editor emit test** (`btctax-tui-edit`) capturing that crate's frames per journey,
   and **viewer emit test** (`btctax-tui`) capturing viewer-chrome frames by replaying
   the same fixture's decisions via btctax-cli (§4.2). Both pin clock + vault path +
   `BTCTAX_PRICE_CACHE`.
3. Committed frame goldens under `docs/examples-tui-walkthrough/<journey>/`.
4. **The committed manifest + its integrity gate.** _(As-built, per §5's amendment — this
   supersedes the original "xtask `tui-walkthrough` subcommand emitting `walkthrough.md`
   + `regen == committed`" wording.)_ A hand-authored per-journey
   `docs/examples-tui-walkthrough/<journey>/manifest.txt`, pinned by xtask's
   `walkthrough_manifests_valid_and_complete` (grammar + a FRAME⇄golden bijection). No xtask
   subcommand.
5. **`make tui-walkthrough`** + the manifest-walking interleave glue
   (`assemble-walkthrough.sh`: `.PP` prose emitted directly, tui-wrap.awk on frames) →
   `btctax-tui-walkthrough.pdf`.
6. CI wiring (per-crate golden gates + the xtask manifest gate in `test`; PDF-render proof
   in the advisory `examples` job).
7. Captions + narrated preambles are **the single source of truth, hand-authored in each
   journey's `manifest.txt`** _(As-built — supersedes the original "Rust literals in the xtask
   assembler, emitted to `walkthrough.md`" wording; see §5)_. A wording edit still touches
   exactly one file per journey and never a TUI crate.

## 9. Build staging (per owner's autonomous plan)

A **one-journey proof-of-concept** first — a TUI-rich journey (proposed **J8**
match-self-transfers, or **J3** self-transfer) end-to-end, exercising the WHOLE
architecture so it validates the hard parts, not just the easy ones: the corpora hoist
(§4.1), BOTH the editor emit test and the viewer emit test converging on the one shared
fixture (§4.2), the hand-authored manifest + its xtask bijection gate + `make
tui-walkthrough` interleave glue (§5, As-built), and a one-journey PDF — reviewed to
green. The remaining eight journeys are then a mechanical rollout of the same pattern.

**Gate (brainstorming discipline):** the owner is travelling; this spec + the
implementation plan are the autonomous deliverable. The build (even the PoC) is **held
for owner approval** of the reviewed spec/plan — because §4.1 (a real cross-crate
refactor) and §4.2 (a capture-architecture decision) are choices worth a human nod
before code is written. One "go" unblocks the PoC; the PoC then unblocks the rollout.

## 10. Open questions (for owner review; do not block the PoC)

- **Caption placement/voice**: caption above each screen (recommended) vs a facing
  page; terse vs teaching-voice. PoC will pick one; easy to change.
- **Frame granularity**: how many intermediate frames per flow (e.g. show every form
  step, or just list → confirm → result). PoC will show a sensible default.
- **Location of drivers**: a dedicated harness vs extending the existing golden tests —
  decided in the plan to minimize the `main.rs` monolith's growth.
- **Colorized vs monochrome** for this PDF: colorized (reuse `tui-wrap.awk`) — matches
  the TUI-screens PDF.

## 11. Risks

- **Golden count & regen (r1 M-3).** ~4–8 frames × 9 journeys ≈ 40–70 goldens (10–20×
  today's four); any real UI change reds them (by design — that is the point). Regen spans
  TWO `#[ignore]` emit tests (editor + viewer crates); the manifest is hand-authored (edited,
  not regenerated) — so provide a SINGLE documented `make regen-walkthrough` target that runs
  both emit tests, keeping the "one-command refresh" claim true. (As-built, §5.)
- **Monolith growth.** The editor emit test lives in `btctax-tui-edit`'s existing
  `#[cfg(test)]` module (as today's goldens do) — it grows the TEST surface, not
  production; drivers must not leak into production code.
- **Thin journeys (softened by r1 M-1).** Only J1 is truly viewer-only (a legitimate
  "read your results" chapter: Holdings → Disposals → Tax → export). J2/J5 have real
  editor surfaces. Honesty per journey, not padding.
- **Cross-crate convergence (§4.2).** The editor-mutation and viewer-result halves must
  stay in sync via the ONE shared fixture; the per-journey fixture fn owned in
  `btctax-cli::testonly` is the single point that keeps them convergent.
