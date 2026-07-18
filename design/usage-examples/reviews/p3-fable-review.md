# P3 Fable review — TUI style-aware capture + TUI clock seam + goldens/gate (SPEC §3.4, §8)

**Reviewer:** Fable (independent; author ≠ reviewer). **Scope:** `git diff 5183602..HEAD`
(f30b1e8 clock seam, 28672f4 capture serializer, ccfedc5 goldens/guard/PDF), verified against live
source. **Date:** 2026-07-18.

## Verdict

The engineering that IS here is sound and was verified live: all 25 production `now_utc()` reads in
both TUI crates are routed through the injected `Clock` (only the 2 test-module sites and
`Clock::Wall`'s own read remain); `Wall` preserves real-session semantics (a fresh read per call, not
a frozen startup time); `from_env` textually mirrors the CLI `resolve_now` contract exactly
(unset→Wall, RFC3339→Pinned, malformed/empty/non-UTF-8→Err, identical banner string, exit 2 before
raw mode); the capture serializer is deterministic and its trailing-space trim provably hides no
styled cell (a non-default trailing cell still emits a coordinates-bearing style run); the three
goldens are deterministic (fixed vault paths, pinned clock, bundled prices, sorted collections); the
source-gate change fixes only a false positive (the everywhere/vault-mutation tokens still scan
`tests.rs`, which is genuinely `#[cfg(test)]`-declared); the N-2 CI resolution is equivalent-or-
stronger than SPEC §9's letter (staleness gated in-process on BOTH unix `test` legs — the goldens
cannot rot silently while windows skips); and `make check` is green (1956/1956, 8 skipped). **But the
phase does not discharge its mandate.** The delivered golden surface is one viewer tab of six, one
modal, and an *empty* editor Browse screen — SPEC §8's and PLAN Task 3.3's named centerpiece, a
captured **btctax-tui-edit reconcile flow** ("the primary bug-hunt surface", which §10's P4 audit
consumes by name), was silently re-scoped away with no recorded rescope. And the guard story over the
seam is materially overclaimed: I ran the mutations, and 22 of the 23 editor clock sites (plus the
what-if viewer site) can be reverted to the wall clock with the entire suite staying green, while the
guard's doc comment and both commit messages claim the opposite. The env half of the seam
(`from_env` + the exit-2/banner wiring) has zero test coverage — §3.1(iii) requires tests to pin it.
**Gate: RED — 0 Critical / 3 Important.**

## Validation evidence (run by this reviewer)

- `make check` → **GREEN**: 1956 passed, 8 skipped (incl. both `#[ignore]` regen helpers), ~13s.
- `cargo test -p btctax-tui --lib export_modal_dir_name_uses_the_injected_clock` → pass.
- `cargo test -p btctax-tui-edit persisted_decision_made_date_is_the_injected_clock` → pass.
- Both `*_goldens_match_committed` tests → pass (fresh capture == committed bytes).
- `make examples-tui` → `docs/pdf/btctax-tui-screens.pdf` written, `%PDF` magic verified, path
  git-ignored (`docs/pdf/` in .gitignore:53), no collision with `bundles` (its merge excludes
  `btctax-tui*`, Makefile:47).
- **Mutation A** (guard works for its one site): reverted `main.rs:1551` (classify-inbound) to
  `now_utc()` → `persisted_decision_made_date_is_the_injected_clock` **FAILED**. Restored.
- **Mutation B** (the gap is real): reverted `main.rs:8811` (`handle_match_self_transfers_modal_key`)
  to `now_utc()` → the **full btctax-tui-edit suite stayed GREEN** (329 passed, 0 failed). Restored;
  tree verified byte-identical to HEAD and the guard re-run green.

## Findings

### Important

- **I-1 — The mandated reconcile-flow golden was not delivered, and no rescope is recorded.**
  PLAN Task 3.3 (IMPLEMENTATION_PLAN_usage_examples.md:438-445): "Drive **the tabs** + a
  `btctax-tui-edit` **reconcile flow** … (esp. the edit reconcile flow — **the primary bug-hunt
  surface**)". SPEC §8: the capture harness is "for `btctax-tui` tabs and `btctax-tui-edit`
  **reconcile flows** … drives real events … snapshots each `Buffer`", and §10 defines the P4 audit
  as driving "esp. **the P3 edit reconcile flows**" — P4's named input. Delivered
  (`docs/examples-tui/`): ONE tab of six (Holdings, `tabs/tests.rs:827`), the export-confirm modal,
  and the editor's **empty Browse chrome** (`main.rs:13907`, built from
  `browse_app_with_empty_snapshot` — no session, no events, no flow). Zero reconcile-flow frames are
  captured anywhere; the clock guard drives a classify flow but snapshots nothing. Concrete failure
  scenario: a rendering regression in any reconcile modal (e.g. the classify-inbound confirm showing
  the wrong FMV/basis line, or a leaked internal-state string — the exact class §8 cites as "the
  btctax analogue of the mnemonic `(none)`/reveal-toggle discoveries") leaves every committed golden
  green, and P4's audit surface doesn't exist. Neither commit ccfedc5 nor FOLLOWUPS records a
  rescope — the commit message simply re-describes the deliverable as "Browse chrome". Fix: capture
  a reconcile-flow frame sequence (the guard test already drives classify-inbound headlessly —
  snapshot the confirm-modal frame under the pinned clock) as committed goldens, or take an explicit
  reviewed SPEC §8/PLAN amendment. "It's covered by the clock guard" covers determinism, not the
  bug-hunt surface.

- **I-2 — The env half of the seam is entirely untested; §3.1(iii) is unmet for the TUI seam.**
  No test in either TUI crate references `BTCTAX_NOW` (grep: production sources only), and
  `clock.rs`'s own test (`clock.rs:78-86`) deliberately never calls `from_env` — it tests raw
  RFC3339 parsing. So `from_env`'s None→Wall link, the non-UTF-8 arm, the exit-2 wiring
  (`lib.rs:678-683`, `main.rs:9778-9783`), and the `OVERRIDE_BANNER` prints (`lib.rs:686`,
  `main.rs:9786`) are all held by convention. §3.1's fence demands "(iii) **tests pin** the
  inactive-path equivalence", and the CLI's identical contract got binary-level KATs
  (`btctax-cli/tests/btctax_now_seam.rs`) at P0's gate. Concrete failure scenario: `from_env` is
  edited to fall back to `Wall` on a malformed value (or the exit-2 arm is dropped in a refactor);
  the suite stays green because every golden injects `Pinned` directly; a user with a typo'd
  `BTCTAX_NOW` silently runs on the wall clock with no banner — the "hard error on malformed"
  guarantee and the disclosure guarantee both evaporate unobserved. Cheap fix: a
  `std::process::Command` KAT running the `btctax-tui-edit` binary with `BTCTAX_NOW=garbage`
  asserting exit code 2 + the stderr message — the error path exits BEFORE `enable_raw_mode`, so no
  TTY is needed (mirror the CLI KAT); plus a spawn-based check of the banner/None→Wall arms.
  (The pre-raw-mode stderr banner itself is a defensible §3.4 disclosure: it prints on the primary
  screen before the alt-screen switch and persists in scrollback after exit — accepted.)

- **I-3 — 22 of the 23 editor clock sites (and the what-if viewer site) can silently revert to the
  wall clock, and the delivered artifacts claim otherwise.** Proven by Mutation B above: reverting
  `main.rs:8811` to `now_utc()` leaves all 329 btctax-tui-edit tests green. Yet the guard's doc
  comment (`main.rs:13852-13853`) claims "reverting **any one** back to `now_utc()` reds this" —
  empirically false for 22 of 23 — and the two commit messages cross-claim coverage that neither
  delivers: f30b1e8 says "(The 23 editor made-date reads are guarded by the Task 3.3 goldens)" (the
  Task 3.3 goldens guard zero editor made-date reads — Browse-empty renders no decision timestamps),
  while ccfedc5 claims revert-proof for "the classify handler" only. PLAN Task 3.1 Step 1 also
  specified a pinned what-if panel render test; none exists (`whatif_panel.rs`'s tests pass a fixed
  `now` param — the `lib.rs:248` routing site is unguarded). Concrete failure scenario: a future
  "just a tweak" adds or reverts a `let now = time::OffsetDateTime::now_utc();` in a modal handler;
  compile + full suite green; the §3.4 invariant ("route EVERY production read") is silently broken,
  and the first symptom is a flaking future golden or a real timestamp persisted under a simulated-
  clock docs run. The repo already owns the fix pattern for exactly this invariant class: the
  export.rs e10 token-scan gate. Fix: a source-scan test asserting no `now_utc(` outside
  `clock.rs`/test regions across both crates (reuse the e10 machinery), or a clippy
  `disallowed-methods` entry for `time::OffsetDateTime::now_utc` in the two TUI crates — either
  makes the invariant structural — and correct the false comment at `main.rs:13852-13853`.
  (Per-site behavior today is verified correct; this is the guard, not the routing.)

### Minor

- **M-1 — The §14 gap 7 decision (`underline_color`/`skip`) was made implicitly but recorded
  nowhere.** `capture.rs` captures `(symbol, fg, bg, modifier)` and drops `underline_color` and
  `skip`; neither capture.rs's module docs, commit 28672f4, FOLLOWUPS, nor the SPEC records the
  decision the gap explicitly assigned to P3 ("decide whether to capture them"). The drop is
  currently lossless — verified: no `underline_color` use in either crate (sort.rs's cursor
  underline is `Modifier::UNDERLINED`, which IS captured), and `skip` never varies in a
  `TestBackend` buffer — but an unrecorded drop is indistinguishable from an oversight, and if a
  future style adopts `underline_color`, its regressions become invisible to the goldens with no
  breadcrumb saying that was chosen. Fix: one recorded paragraph (capture.rs docs + tick gap 7)
  stating the decision, the rationale, and the re-open trigger.

- **M-2 — SPEC §8 mandates "groff render **with color** from the style map"; the delivered PDF is
  monochrome, and the deferral (UX-P3-2) self-grades the deviation as a Nit without amending the
  SPEC.** The FOLLOWUPS entry is honest (it quotes §8 accurately, notes the gated `.txt` artifact
  carries full style, and the PDF is a git-ignored convenience render with no consumers), so this is
  not the false-citation anti-pattern — but a follow-up is a scheduling tool, not spec cover: as
  long as §8's text stands unamended, the SPEC claims a color render that does not exist. Fix in
  this phase: either amend §8 (one line, re-reviewed) to make the color render an explicitly
  deferred enhancement, or implement it; and let the reviewer, not the author, grade the deviation.

### Nit

- **N-1 — The `tests.rs` write-class exemption (`export.rs:872,889`) is filename-based, not
  cfg-verified.** Today it is sound (`tabs/mod.rs:14-15` declares `#[cfg(test)] mod tests;`, and the
  everywhere/vault-mutation tokens still scan the whole file), but a hypothetical future production
  module named `tests.rs` would inherit the write-class exemption. Hardening: have the gate assert
  the parent declares the module under `#[cfg(test)]`.

- **N-2 — `design/usage-examples/RECON_P3_TUI.md` is untracked.** The P3 load-bearing recon digest
  (which this review verified the site inventory against) exists only in the working tree — commit
  it with the P3 record per the artifact discipline.

- **N-3 — `color_str` (`capture.rs:63-65`) depends on ratatui `Color`'s derived `Debug`
  formatting.** Stable under the locked ratatui 0.29, and the failure mode is loud (a format change
  reds every golden at regen), never silent — acceptable; worth a one-line note in capture.rs so the
  regen-time red is diagnosable.

## Explicit gate

**RED — 0 Critical / 3 Important / 2 Minor / 3 Nit.** The suite itself is green (1956/1956 + clippy
via `make check`), so the reds are the findings, not the build. I-1 (deliver a reconcile-flow golden
or a reviewed rescope), I-2 (test the env seam), and I-3 (make the 25-site routing invariant
structural + correct the false claims) block P3 close; re-review after the fold.
