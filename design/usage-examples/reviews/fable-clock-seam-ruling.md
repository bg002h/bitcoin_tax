# Fable architect ruling — `BTCTAX_NOW` determinism seam (persisted verbatim)

*Persisted 2026-07-16, verbatim before folding, per STANDARD_WORKFLOW §2. Author of ruling:
Fable-model architect (independent). Question posed: whether adding a `BTCTAX_NOW` clock-injection
seam as Phase 0 of the usage-examples cycle violates the standing rule "don't edit the engine to make
a doc pretty," and which of options A/B/C to adopt. This ruling closes brainstorm open-question #3's
one code-level blocker.*

---

# RULING — `BTCTAX_NOW` determinism seam vs. the "don't edit the engine for a doc" standing rule

## FACTS-VERIFIED (with corrections)

**Confirmed:**
1. `crates/btctax-cli/src/main.rs:66` — `let now = OffsetDateTime::now_utc();` is the **only** wall-clock read in the entire `btctax-cli` crate. It is threaded as a plain `now: OffsetDateTime` parameter into the cmd layer (verified signatures in `crates/btctax-cli/src/cmd/optimize.rs:35,149,174,283`). Because it is the single read, the what-if `--date` "defaults to today UTC" flags (`cli.rs:339,416,466`) derive from the same value — one seam covers everything in the CLI binary.
2. `crates/btctax-core/src/persistence.rs:238-262` — `append_decision` persists the caller's `utc_timestamp`; DDL `utc_timestamp TEXT NOT NULL` at persistence.rs:110. Confirmed.
3. Read-back leak confirmed: `render.rs:2258` prints `"  recorded {} effective {} -> {} [{}]"` from `recorded = tax_date(e.utc_timestamp, e.original_tz)` (render.rs:531); bulk previews build `date: tax_date(e.utc_timestamp, …)` at `session.rs:1134` and `session.rs:1183`, printed by `render_bulk_void_preview` at `main.rs:2005`.
4. No seam exists: the only env var the binary reads is `BTCTAX_PASSPHRASE` (main.rs:50). No `--as-of`/`--today`/`BTCTAX_NOW` anywhere.
5. Library-level test injection confirmed: `crates/btctax-cli/tests/reconcile.rs:16` — `datetime!(2026-02-01 12:00:00 UTC) // fixed decision clock (NFR4 deterministic tests)`. The binary genuinely lacks the seam its own test suite depends on one layer down.
6. Attestation logic confirmed at `crates/btctax-core/src/optimize.rs:469-484`: `persistability()` compares `selection_made` (= `tax_date(now)`) against `sale_date`; `made ≤ sale → ContemporaneousNow`, else `NeedsAttestation`. The consumer is `render.rs:1817-1831` (your ~1827 cite is accurate).
7. No RNG, no `CARGO_PKG_VERSION`, no elapsed-time output anywhere in CLI stdout. CLI `export-snapshot` writes to an explicit `--out` path (no timestamp in naming).

**Corrections:**
- **C1 — wrong crate on two cites.** `render.rs` and `session.rs` live in `crates/btctax-cli/src/`, **not** `btctax-core`. All line numbers were right; the paths were wrong. This correction *strengthens* the case for A: every file implicated in the leak, and the only file the fix touches, is in the CLI crate. Core is not involved at all.
- **C2 — core is clock-free, with one asterisk.** `btctax-core` does contain a `now_utc()` at persistence.rs:430, but it is inside a `#[cfg(test)]` module. Production core paths are fully parameterized. Claim holds.
- **C3 — the "one threaded-arg seam" claim is true for the CLI only.** The TUI side is materially bigger: `btctax-tui/src/lib.rs:247,256` (2 reads — and lib.rs:256 feeds `export_dir_for(&vault_path, export_now)` at `btctax-tui/src/export.rs:30`, i.e. a **timestamped directory path rendered on screen**), and `btctax-tui-edit/src/main.rs` + `edit/persist.rs` have **~28 reads**. The TUI-capture doc will hit a much larger seam problem that this ruling's Phase 0 must NOT be stretched to cover.

## RULING

### Q1 — Does option A violate the standing rule?

**No.** On three independent grounds, any one of which suffices:

**Ground 1 — object.** The rule forbids editing the *compute/fill engine*. The seam touches `main.rs:66` — the composition root of the CLI binary, the one line that *observes* the environment. The engine already receives `now` as a parameter everywhere downstream; the change alters the **provenance of an existing input**, not any computation, classification, rounding, wording, or persisted semantics. With the env var unset, the binary is behaviorally byte-identical. A rule against editing the engine cannot reach a change that leaves the engine's entire input-output relation untouched — the corrected file paths make this crisp: nothing in `btctax-core` changes.

**Ground 2 — purpose.** "To make a doc pretty" names an evil: shaping product *output* so the documentation reads better — the tail wagging the dog. The seam shapes no output. Every byte of every transcript is identical whether the clock is pinned or real; only *reproducibility across days* changes. Pinning an input is the categorical opposite of prettifying an output. And the binary's own test suite is the star witness: `reconcile.rs:16` already injects exactly this fixed clock, but only by bypassing the binary through the library API — meaning the shipped binary is not end-to-end deterministically testable today. That is a pre-existing testability gap with value independent of this docs cycle. The docs work *surfaced* it; it did not *create* the need.

**Ground 3 — the rule's own remedial path lands here anyway.** Suppose we insist on classifying the missing seam as a surfaced gap routed to FOLLOWUPS. FOLLOWUPS entries get an owning phase, and the repo's burndown rule says a phase-owned item is not deferrable past its owning phase. An item that blocks this cycle's co-equal, budgeted bug-hunt is owned by *this cycle*. So the FOLLOWUPS path, executed correctly, converges on option A (file it, own it in Phase 0, burn it down before goldens are recorded). Option C is the FOLLOWUPS path executed *incorrectly* — deferring a phase-owned item past its owning phase.

**The fence, so this ruling is not a loophole:** the exemption is narrow. A change qualifies as a determinism prerequisite (not an engine edit) iff **(i)** with the seam inactive the binary is behaviorally identical, **(ii)** it injects an *input*, never transforms an *output*, and **(iii)** tests pin the inactive-path equivalence. Anything failing this trichotomy — rewording a message the doc finds awkward, changing column widths, altering rounding, touching persisted schema — stays under the standing rule and goes to FOLLOWUPS with severity + owning phase. Write this trichotomy into the spec verbatim.

### Q2 — Which option?

**A.** B and C both amputate the decision read-back surface — reconcile, bulk previews, config set-forward-method, optimize accept, attestation flows — which is precisely the multi-step, cross-flag, affordance-rich surface the co-equal bug-hunt exists to drive, and where the user's locked "broad journey set (5+)" decision points. B contradicts two locked decisions outright. C has B's amputation *plus* double corpus churn (author a subset corpus now, re-golden and re-author decision journeys in a later cycle) *plus* the Ground-3 burndown violation. A costs roughly fifteen lines of production code and a test file.

### Q3 — Home, scope, tests, integrity

**Home: Phase 0 inside this cycle**, not an independent cycle. Ceremony scales down, never off: a standalone brainstorm→spec→plan cycle for one env-var read is ceremony scaled *up*. Phase 0 gets a spec section, TDD, and independent review to 0C/0I — the gates intact — while keeping the seam and the twice-run hygiene harness that consumes it inside one reviewable dependency graph. **Hard condition: Phase 0 closes green before the first golden is recorded.**

**Minimal correct scope:**
- **CLI-only.** One read of `BTCTAX_NOW` at main.rs:66, strict RFC3339 (any offset, used as parsed — everything downstream is `OffsetDateTime`), fallback to `now_utc()` when **unset**. Do NOT touch `btctax-tui`/`btctax-tui-edit` (~30 sites; owned by the TUI-doc's own spec, which should consider a shared clock helper and must handle the timestamped export-dir path at `export.rs:30`). Leave `update-prices` alone (network tool, out of golden scope).
- **Env var, no flag.** Matches the `BTCTAX_PASSPHRASE` precedent at main.rs:50; keeps it a scripting seam out of every subcommand's `--help`; no precedence question arises because no flag exists or is added.
- **Malformed or empty ⇒ hard usage error** (exit 2, message naming `BTCTAX_NOW` and the expected format). Never silent fallback: a typo must not silently yield wall-clock nondeterminism in docs CI, nor a wrong made-date in real use.
- **Unconditional stderr notice when active** — one line, e.g. `warning: BTCTAX_NOW override active — decision timestamps are simulated`. On stderr so stdout goldens stay clean; unconditional (not TTY-gated) because the scripted case is exactly the misuse case.

**TDD must pin:** (1) unset → wall-clock path, behavior unchanged; (2) set → persisted decision `utc_timestamp` round-trips exactly through the *binary* (`reconcile … && verify` read-back — closing the very gap the library-level tests dodge); (3) malformed/empty → exit 2, named error; (4) warning present on stderr when set, absent when unset, never on stdout; (5) twice-run **byte-identical stdout AND exit code** for a representative decision-record + read-back journey under pinned `BTCTAX_NOW` + `BTCTAX_PASSPHRASE`; (6) an **integrity KAT**: with `BTCTAX_NOW` backdated to ≤ sale date, `persistability` yields `ContemporaneousNow` — the test *is* the disclosure that the property exists, so it can never be silently forgotten.

**Integrity risk — take it seriously, and honestly.** Yes: backdating `BTCTAX_NOW` flips `NeedsAttestation → ContemporaneousNow` at optimize.rs:479, bypassing the attestation gate and its audit line (render.rs:1887). Three-part fence: **(1) No pretense.** The user already owns the clock (`faketime`, `date -s`); the vault's `utc_timestamp` was never cryptographic evidence — it is self-reported. The seam does not create the capability; it removes the pretense that it didn't exist. Per the project's authority hierarchy: §1.1012-1(j) demands contemporaneity *in fact*; the vault row is evidence of it, not the fact itself. **(2) The unconditional stderr banner**, pinned by test. **(3) Man-page language** stating the variable exists for reproducible testing/documentation and that backdating a decision record does not make an identification contemporaneous under the reg. **Rejected alternative, deliberately:** persisting an "override was active" marker in the vault. That would be a core schema change (a real engine edit — the exact thing the standing rule forbids for a docs cycle) and security theater besides: a motivated forger uses `faketime` and leaves no marker. Also: do **not** entangle the seam with pseudo-mode/DRAFT machinery (no auto-flagging vaults as pseudo under override) — that would change engine semantics and defeat the docs' purpose of capturing *real* flows; DRAFT/attest policy stays as user-mandated.

### Q4 — Adjacent findings the recon should absorb

1. **The TUI seam is the next blocker and it is not small** (~30 sites; plus the on-screen timestamped export path, `btctax-tui/src/export.rs:30`). Budget it in the TUI-doc spec explicitly; do not let anyone claim this ruling covers it.
2. **Paths in stdout**: `init`/`import` print `vault.display()` and key-backup paths — goldens need a fixed cwd + relative-path invocation convention (harness discipline, no code change).
3. **Capture convention needs a spec line**: with the stderr warning unconditional, "verbatim I/O" docs should capture stdout (+ exit code) and state the pinned-env convention (`BTCTAX_NOW`, `BTCTAX_PASSPHRASE`) in front-matter — otherwise every example carries the warning line or the doc silently strips stderr. Also note the doc-fidelity wrinkle that captures use `BTCTAX_PASSPHRASE` while a real user sees an interactive prompt — worth one honest sentence in the doc.
4. **Exit codes are output**: `verify` returns 1 on hard blockers (main.rs:89-91) — the hygiene test and the goldens should assert them, not just stdout.
5. Nothing else nondeterministic found: no RNG, no version strings in output, no timers, `decision_seq`/ordinal deterministic given identical command sequences. The vault file itself is nondeterministic (encryption nonces) — goldens must never hash it; regenerate-in-CI (already locked) is the right posture.

## RECOMMENDATION

**Option A**, as **Phase 0 of this cycle**, CLI-only, scoped exactly as above, logged in FOLLOWUPS as the cycle's first surfaced finding with owning phase = Phase 0.

## RISKS / CONDITIONS

- **Gate:** Phase 0 green (spec'd, TDD'd, independently reviewed to 0C/0I) *before* any golden is recorded.
- **Fence:** the (i)/(ii)/(iii) trichotomy goes into the spec verbatim; every future "just a tiny tweak for the docs" is tested against it, and failures go to FOLLOWUPS.
- **Integrity:** stderr banner + integrity KAT + man-page misuse language are Phase 0 deliverables, not follow-ups — shipping the seam without its disclosure would be shipping a correct fix with no test holding it, the project's own named failure pattern.
- **Correction to carry forward:** fix the two crate-path citations (`btctax-cli`, not `btctax-core`, for render.rs/session.rs) in the brainstorm doc before it becomes the spec's substrate — citations decay, and this one changes the argument's strength in your favor.
