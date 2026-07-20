# IMPLEMENTATION PLAN — TUI screen-based walkthrough

Derived from `SPEC_tui_walkthrough.md` (Fable-reviewed to GREEN, r1→r2). Phased,
test-first, each phase reviewed to 0 Critical / 0 Important. **The build is HELD for
owner approval** (SPEC §9): this plan is the artifact to approve; nothing here is built
until the owner says go. Citations verified against source at spec-r2 time; re-verify at
build time (they drift).

## Cross-cutting discipline (every phase)

- TDD: the golden emit test is written, run to produce the committed golden, then the
  byte-gate test reds if the golden is stale — that IS the test-first loop for captures.
- Determinism per SPEC §7: `Clock::Pinned`, fixed displayed vault path, `TestBackend
  ::new(120,40)`, and **`BTCTAX_PRICE_CACHE` pinned to a nonexistent file** in every
  emit/gate test.
- `#[cfg(unix)]` on the byte-exact golden gates (rendered paths carry a separator).
- After each phase: `make check` + fmt/pii/isolation/msrv green, independent Fable
  review to 0C/0I, push.

## Phase 0 — Shared fixtures home (the enabling refactor)

Owns SPEC §4.1. No new behavior — pure relocation, gated by an unchanged examples golden.

1. Create `crates/btctax-cli/src/testonly.rs` (`pub mod testonly;` in lib.rs, a plain pub
   module mirroring the `btctax-forms` precedent — not feature-gated).
2. Move the **eleven** corpus consts (`J1_CSV`…`J9_CSV` with J6/J8 split per-exchange:
   `J6_RIVER_CSV`/`J6_COINBASE_CSV`, `J8_RIVER_CSV`/`J8_COINBASE_CSV`) there as `pub`
   consts, byte-identical (keep the explicit-CRLF form; the `.gitattributes` LF hazard is
   avoided by keeping them as Rust string literals, never committed `.csv`).
3. Move `J6_FULLRETURN_TOML` there; since the fixture file already lives in btctax-cli
   (`tests/fixtures/examples/fullreturn_inputs.toml`), the `include_str!` becomes
   same-crate — retire the cross-crate include (the examples.rs M-5 exception).
4. Add a `pub struct JourneyFixture { corpus: &[(&str, &str)], decisions: Vec<Decision> }`
   (or similar) per journey: the corpus files + the ordered reconcile decisions, so both
   capture halves (Phase 1) share ONE source of truth.
5. Repoint `xtask/src/examples.rs` to import the consts from `btctax_cli::testonly`.
   **Gate**: `examples_golden_matches_committed` stays byte-identical (the consts are pure
   data — re-importing identical bytes can't change `generate()`).
6. **Review to green.** This phase is independently valuable (de-duplicates the corpora)
   and low-risk; it can land even if the walkthrough is later deferred.

## Phase 1 — Proof-of-concept: ONE journey end-to-end (proposed J8)

Exercises the WHOLE architecture on the hardest-realistic journey (cross-exchange
match-self-transfers: editor mutation + viewer BALANCED result). Validates §4.2, §5, §7.

1. **Editor emit test** (in `btctax-tui-edit`'s existing `#[cfg(test)]` module, beside
   `emit_btctax_tui_edit_goldens`): seed the J8 fixture's corpus, unlock, pin
   clock/path/`BTCTAX_PRICE_CACHE`, drive the editor via scripted `KeyEvent`s (Browse
   showing the unreconciled blocker → `m` match-self-transfers preview → confirm modal →
   post-relocate Browse), capturing a frame at each step via `capture_edit_frame` →
   `to_golden`. Commit the frames to `docs/examples-tui-walkthrough/j8/NN-*.txt`; add the
   `*_goldens_match_committed` byte-gate + the `#[ignore]` emit.
2. **Viewer emit test** (in `btctax-tui`'s `#[cfg(test)]` module, beside
   `emit_btctax_tui_goldens`): seed the SAME J8 fixture corpus, **replay its decisions via
   btctax-cli** (`self_transfer_match_plan` + `apply_self_transfer_passthrough`), then
   `build_snapshot` + `render_viewer` to capture the Holdings "BALANCED" result frame(s).
   Same golden dir, gate, emit.
3. **xtask `tui-walkthrough` subcommand**: emit the J8 slice of the manifest
   `docs/examples-tui-walkthrough/walkthrough.md` — narrated CLI-setup prose + per-frame
   captions (Rust literals in the assembler) + frame-file references, in order. Add the
   `regen == committed` gate (precedent `examples_golden_matches_committed`).
4. **`make tui-walkthrough`** + the interleave glue: walk the manifest, run `man-wrap.awk`
   on prose/caption chunks and `tui-wrap.awk` on referenced frames, concatenate in order
   → `docs/pdf/btctax-tui-walkthrough.pdf` (geometry `-dpaper=letterl -P-pletterl
   -rLL=10i -rPO=0.4i`, 9pt, `\m[` guard). Verify it renders + fits (rasterize).
5. **CI**: the per-crate golden gates run in the `test` job automatically; add the
   `make tui-walkthrough` `%PDF` proof to the advisory `examples` job.
6. **Review the PoC to green**, push.
7. **HARD GATE — owner approval of the PoC** before Phase 2. The owner sees a concrete
   one-journey PDF + the machinery, and can redirect format/caption/frame-granularity
   cheaply before the 8× rollout.

### As-built deviations (recorded at the PoC review, folding r1)

The PoC was built and reviewed; three deliberate deviations from the steps above are
recorded here so the owner's gate sees choices, not omissions (PoC review M-3, C-1, and the
reviewer's non-finding note):

- **Step 1 — three editor frames, not four.** The "post-relocate Browse" frame was dropped:
  the confirmed result is shown once, authoritatively, by the **viewer** Holdings frame
  (step 4), and the editor-side post-confirm write is already held by the e2e
  `kat_e2e_match_self_transfers_relocate_lands_coins_in_dest`. A second post-confirm editor
  screen would narrate the same beat twice. (SPEC §10 grants frame-granularity latitude.)
- **Step 2 — RELOCATE replayed via `link_transfer`, not `apply_self_transfer_passthrough`.**
  The plan's cited fn is the DROP action; `apply_self_transfer_passthrough` would not land the
  coins at the destination. The impl uses `btctax_cli::testonly::seed_j8_relocated` →
  `cmd::reconcile::link_transfer` (out→`InEvent(in)`), the byte-same payload the editor's
  confirm writes (per G-RELOCATE-REUSE). The deviation is toward correctness.
- **Steps 3–4 — hand-authored per-journey manifest + bijection gate, not an xtask-emitted
  `walkthrough.md` with Rust-literal captions + a `regen == committed` gate.** See the SPEC §5
  "As-built" note: prose+captions live in `docs/examples-tui-walkthrough/<journey>/manifest.txt`
  (still a single owner; a wording edit never touches a TUI crate). Because nothing regenerates
  a hand-authored file, the integrity gate is instead xtask's
  `walkthrough_manifests_valid_and_complete` — grammar + a FRAME⇄golden **bijection** that reds
  on a dangling reference *and* on a silently dropped `FRAME` line (a shrunk walkthrough). The
  prose is already roff, so the assembler emits `.PP` directly rather than routing through
  `man-wrap.awk`. The CI `%PDF` proof (step 5) is unchanged.

## Phase 2 — Rollout: the remaining eight journeys

Only after PoC approval. Mechanical repeat of Phase 1's two-emit-test pattern per journey,
driven by each journey's `JourneyFixture`. Suggested order by TUI richness: J3, J9, J7,
J6 (rich) → J4, J5, J2 (medium) → J1 (viewer-only "read your results" chapter). Each
journey: editor frames + viewer frames + manifest slice + captions; `make check` green;
the whole walkthrough PDF regenerated. Batch the review by a few journeys at a time.

## Phase 3 — Close

- The single **`make regen-walkthrough`** target running both `#[ignore]` emit tests + the
  xtask manifest regen in order (keeps the "one-command refresh" true, SPEC §11).
- Whole-feature Fable review to 0C/0I; regen all goldens; confirm the walkthrough PDF.
- FOLLOWUPS: record any residue with an owning phase.
- Push; verify CI green.

## Risks & sequencing notes

- Phase 0 is a prerequisite for Phase 1 (the shared fixture home). It is also the only
  phase touching a published crate's API (`btctax-cli::testonly`) — keep it a plain pub
  module, exactly like `btctax-forms::testonly`.
- The editor emit test grows `btctax-tui-edit`'s TEST module (as today's goldens do), not
  production; drivers must not leak into production code.
- Frame count grows to ~40–70 goldens across two crates — the `make regen-walkthrough`
  target (Phase 3) is what keeps regen a one-command operation.
- Citations in this plan and the spec drift; re-verify at build time (verify-citations
  rule).
