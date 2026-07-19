# DESIGN PROPOSAL — TUI screen-based walkthrough PDF

**Status: PROPOSAL — pending user review/approval. Nothing is implemented; this
document exists so the design can be approved (or adjusted) before any build, per the
brainstorming gate.**

Date: 2026-07-19. Owner decision needed before implementation begins.

## 1. What this is

A second worked-examples document that tells the same stories as `btctax-examples.pdf`
(the J1–J9 journeys) but as a **visual, screen-by-screen walkthrough of the TUI** —
what a newcomer actually *sees and presses* in the `btctax-tui` viewer and
`btctax-tui-edit` editor — instead of CLI command + text-output blocks.

Deliverable: `docs/pdf/btctax-tui-walkthrough.pdf` (colorized, landscape), plus its
committed source goldens.

## 2. Decisions locked in brainstorming

- **Purpose:** a user-facing **visual tutorial** (bar: a newcomer can follow along).
- **CLI/TUI split (hybrid):** the TUI does not `init`, `import` a CSV, or run
  `report`/`export-irs-pdf` from scratch — those are CLI-only. So the walkthrough is
  **hybrid**: it *narrates* the CLI setup steps in prose and *shows TUI screen
  captures* for everything the TUI genuinely does (reconciling, authoring a return,
  viewing the computed tabs, exporting CSVs).
- **Scope:** **all nine journeys** (J1–J9), each adapted to a short screen sequence.
  Journeys that are view-only in the TUI (e.g. J1) lean more on narration; the
  reconcile/author-heavy journeys (J3/J6/J7/J8/J9) are where the screens carry the
  story.

## 3. Proposed approach (defaults — adjust on review)

### 3.1 Capture mechanism (reuse the existing golden machinery)

The TUI already renders any screen *headlessly, from a plain state value*: build an
`App`/`EditorApp` with a synthetic or seeded-vault `Snapshot` and a **pinned clock**,
optionally drive it into a target state by feeding synthetic `KeyEvent`s through the
**real `handle_key`**, render once into `TestBackend::new(120, 40)`, and serialize with
`capture::to_golden` (glyph grid + style-run overlay). This is exactly how the four
existing `docs/examples-tui/*.txt` goldens are made.

Proposal: a new **`xtask` generator** (`cargo run -p xtask -- tui-walkthrough`) that,
per journey, seeds a temp vault from the same synthetic corpora the examples generator
uses, drives the documented key sequence for each step, and captures a frame per
"beat" into a committed golden per screen. Determinism recipe is the proven one:
pinned `BTCTAX_NOW`, fixed 120×40 geometry, fixed displayed vault path, synthetic data
only.

### 3.2 Gating (consistent with the project's determinism discipline)

The captured frames are **committed, CI-gated goldens** (a `regen == committed` test,
like `examples_golden_matches_committed`), so the tutorial cannot silently rot when the
UI changes. This mirrors the existing `*_goldens_match_committed` discipline.

*Lighter alternative if effort must be bounded:* generate the frames at build time
without byte-gating (a convenience artifact). Not recommended — it breaks the project's
"everything deterministic and gated" norm — but noted as the dial-back option.

### 3.3 Output format

Reuse the **colorized landscape PDF** pipeline built for `btctax-tui-screens.pdf`
(`tui-wrap.awk` + the landscape groff geometry). Each screen is preceded by a short
prose lead-in ("Carol presses `c` to classify the deposit…") and followed by a one-line
takeaway ("the blocker clears; the ledger balances"). CLI setup steps appear as brief
narration between screen groups. One journey per section (## J1 … ## J9), each opening
with a one-paragraph scenario recap.

### 3.4 Annotation style

Per screen: **lead-in** (what the user does) → **the screen** (colorized capture) →
**takeaway** (what changed / what to notice). Keep prose tight; the screens carry the
weight.

## 4. Per-journey adaptation (the sketch to approve)

Legend: *[narrate]* = prose only (CLI step); *[screen]* = TUI capture.

- **J1 — single buyer, start to finish.** *[narrate]* init + import a Coinbase CSV →
  *[screen]* editor authors a tax profile (`p`) → *[screen]* viewer **Holdings** tab →
  *[screen]* viewer **Tax** tab (the attributable-tax report) → *[screen]* viewer
  **export** modal (the four CSVs).
- **J2 — donating appreciated BTC (§170(e) / 8283).** *[narrate]* init + import →
  *[screen]* editor **set-donation-details** (`d`) confirm → *[screen]* viewer **Forms**
  tab (Form 8283 rows + the qualified-appraisal note) → *[screen]* viewer **Tax** tab.
- **J3 — reconciling a self-transfer (unknown-basis inbound).** *[narrate]* init +
  import → *[screen]* viewer **Compliance** tab showing the Hard `UnknownBasisInbound`
  blocker → *[screen]* editor **classify-inbound-self-transfer** (`c`) payload-confirm
  modal → *[screen]* viewer **Compliance** (blocker cleared, conservation BALANCED).
- **J4 — mining/staking income + SE tax.** *[narrate]* init + import River income →
  *[screen]* editor **reclassify-income** to a trade/business (`r`) → *[screen]* viewer
  **Income** tab → *[screen]* viewer **Tax** tab (Schedule SE self-employment tax).
- **J5 — optimizing lot selection.** *[narrate]* init + import + author profile +
  FIFO election → *[screen]* editor **optimize-accept** (`z`) proposal/confirm →
  *[screen]* viewer **Disposals**/**Tax** (the tax-minimizing identification, marked
  contemporaneous).
- **J6 — a complete return (the full 1040 packet).** *[narrate]* init + import →
  *[screen]* editor **tax-inputs authoring** (`T`) — the ReturnInputs form, section
  navigation, answered-vs-unanswered glyphs (**the showcase screen**) → *[screen]*
  editor commit → *[screen]* viewer **Forms** tab (the packet) → *[narrate]* CLI
  `export-irs-pdf` produces the filled PDFs.
- **J7 — income valued by hand (`--fmv`).** *[narrate]* init + import an off-exchange
  deposit → *[screen]* viewer **Compliance** (the FMV-missing/unknown-basis blocker) →
  *[screen]* editor **classify-inbound-income** with a hand-entered FMV (`c`) →
  *[screen]* viewer **Tax** tab (the ordinary income).
- **J8 — matching a self-transfer across two exchanges.** *[narrate]* init + import
  both exchanges → *[screen]* viewer **Compliance** (the two unreconciled legs) →
  *[screen]* editor **match-self-transfers** (`m`) preview → *[screen]* the RELOCATE
  confirm → *[screen]* viewer **Compliance** (BALANCED).
- **J9 — identifying specific lots (`select-lots`).** *[narrate]* init + import →
  *[screen]* editor **select-lots** (`S`) picking specific lots for a disposal →
  *[screen]* viewer **Disposals** tab (the disposal now drawing from the chosen lot,
  recorded as per-disposal compliance).

Roughly 4 screens × 9 journeys ≈ **~35 committed frame goldens**. (This count is the
main effort driver; a dial-back would trim to the reconcile-heavy journeys.)

## 5. Proposed build phasing (TDD, gated, reviewed — once approved)

1. **Generator skeleton + one journey (J3).** The `xtask tui-walkthrough` driver + the
   seed/drive/capture harness + J3's frames + the `regen == committed` gate + the PDF
   render. Prove the whole pipeline end-to-end on one journey. Review to green.
2. **Remaining journeys** in 2–3 batches (J1/J2/J4; J5/J7/J9; J6/J8), each adding its
   corpora, key-sequence driver, frame goldens, and prose. Review per batch.
3. **PDF polish + docs.** Landscape colorized render, page-fit verification (rasterize
   every page), a short intro page, and a `make tui-walkthrough` target. Whole-artifact
   review to green.

Each step follows the standard workflow: TDD (the golden gate reds without the frame),
independent Fable review to 0C/0I, deterministic + CI-gated.

## 6. Open questions / defaults flagged for review

1. **Gating vs convenience** (§3.2): recommend committed CI-gated goldens; confirm the
   ~35-frame maintenance cost is acceptable, or dial back to the reconcile-heavy subset.
2. **One combined PDF vs per-journey?** Recommend one combined PDF (like the examples
   doc), sections J1–J9.
3. **Relationship to `btctax-examples.pdf`:** keep both (CLI doc + TUI doc) as siblings,
   or eventually converge? Recommend siblings; they serve different learning styles.
4. **Seeded vs synthetic-`Snapshot` vaults:** recommend seeding real temp vaults from
   the examples corpora (so the frames reflect the real projection), matching how the
   `classify-confirm-modal` golden is already produced.

## 7. Risks

- **Golden maintenance surface.** ~35 frames pinned at 120×40 will red on any
  intentional UI restyle; that is the point (regression safety) but is real upkeep.
  Mitigated by the one-command regenerate (`#[ignore]` emit test) already used for the
  existing goldens.
- **Screen-sequence drift.** The key sequences that drive each journey encode UI
  navigation; a keymap change reds them. Same mitigation.
- **Hybrid honesty.** The narration must be clear that init/import/export-irs-pdf are
  CLI, so a reader never hunts for a TUI screen that does not exist. The lead-in/takeaway
  framing carries this.
- **Scope creep vs the examples doc.** Keep the tutorial's prose thin — it complements,
  not duplicates, `btctax-examples.pdf`.

## 8. Not doing (YAGNI)

- No new TUI *features* — the walkthrough only captures existing screens/flows.
- No animation/asciinema — static frames only (matches the man-page/PDF doc family).
- No re-theming of the TUI for the tutorial — capture what ships.
