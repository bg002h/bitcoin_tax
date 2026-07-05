# R0 review — SPEC_attest_export_gate.md — round 2

**Artifact:** `design/SPEC_attest_export_gate.md` (post round-1 fold — 1C/3I/1M/1N merged in-place).
**Baseline verified against:** branch `feat/attest-export-gate` @ `7ad6155` (`main` == `afb0807`, the sub-2 merge — `git log` confirms). Read-only architect pass, no implementation.
**Bar:** 0 Critical / 0 Important.
**Prior round:** `reviews/R0-spec-attest-export-gate-round-1.md` (1C / 3I / 1M / 1N — NOT GREEN).

## Verdict: **0 Critical / 0 Important / 0 Minor / 2 Nit — R0-GREEN**

Every round-1 finding is resolved and correct against current source. The central scope premise is now true: the spec gates **both** form-writing paths (CLI `export-snapshot` **and** the `btctax-tui` viewer `e` export), the helper is a pure exact-compare with the interaction pushed to the binary/modal, the wrong-phrase error is unambiguous, and the plan enumerates the compile-breaking sub-2 KAT + all three `PseudoActiveExport` refs. No other form/data-file writer is reachable from any binary. Two Nit-level cosmetics remain (crate qualification on one cite; an implicit `pub`-visibility detail) — neither blocks. **May proceed to implementation.**

---

## C1 (the Critical) — gate covers BOTH form-writing paths — RESOLVED + correct

The spec now closes the accidental-filing door on both writers, and every supporting claim verifies:

**1. Viewer path is real + ungated today.**
- `crates/btctax-tui/src/lib.rs:246` (`KeyCode::Char('e')`) opens the export modal → `:169–181` (modal `KeyCode::Enter` → `export::do_export(snap, &modal)` at `:173`) → `crates/btctax-tui/src/export.rs:143` `btctax_cli::render::write_form_csvs(...)` writes `form8949.csv` / `schedule_d.csv` / `form8283.csv` / `schedule_se.csv`.
- `do_export` (`export.rs:115–152`) contains **no** `pseudo_active` / attestation check anywhere — the modal is a plain Enter/Esc confirm (`lib.rs:167–192`). The `pseudo on → e → Enter → fictional 8949` bypass is genuine and exactly what sub-3 exists to close. ✓ Spec §[R0-C1] item 2 (lines 18–23) + Mechanism "TUI" bullet (lines 50–53) describe this accurately.

**2. Viewer projection exposes `pseudo_active()`.**
- `crates/btctax-tui/src/unlock.rs:173` `build_snapshot` → `session.load_events_and_project()` → `crates/btctax-cli/src/session.rs:458–466` (`self.config()?.to_projection()` at `:462`) → `crates/btctax-cli/src/config.rs:35–42` maps the persisted `pseudo_reconcile` key (`:40`), which `read_config` populates from the stored `"pseudo_reconcile"` value (`config.rs:125–126`). So `snap.state.pseudo_synthetic_count` is honoured in the viewer, and `snap.state.pseudo_active()` (`crates/btctax-core/src/state.rs:268–270` = `pseudo_synthetic_count > 0`) is available at both `e`-press (`snap` in hand at `lib.rs:247`) and inside `do_export`. ✓
- **Cite nuance (see Nit-1):** the spec (line 27) cites `pseudo_active()` as `state.rs:268` — the definition is in **`btctax-core`**`/src/state.rs:268`, not a `btctax-cli` file. Line number is correct; only the crate is unstated.

**3. `btctax-tui` depends on `btctax-cli`** → it can share `ATTEST_PHRASE` / `require_attestation`.
- `crates/btctax-tui/Cargo.toml:22` `btctax-cli = { path = "../btctax-cli", version = "0.1.0" }`. ✓ (Note: for the share to compile, both `ATTEST_PHRASE` and `require_attestation` must be `pub` in the `btctax-cli` lib — see Nit-2.)

**4. A typed-word modal mirroring tui-edit's safe-harbor-attest is implementable.**
- The pattern is live in the sibling crate: `crates/btctax-tui-edit/src/edit/form.rs:1244` `SafeHarborAttestStep::TypedWord { buf, error, .. }`; validation at `crates/btctax-tui-edit/src/main.rs:5296–5318` (trim the typed buffer, compare to the required word `"ATTEST"` at `:5310`; wrong → set error + **preserve** buffer; Esc → step back). The viewer export modal can adopt the identical shape, comparing to `ATTEST_PHRASE` instead. ✓ (The tui-edit ceremony word is the single token `"ATTEST"`; sub-3's phrase is the full `"I attest this is true"` — a deliberate per-flow divergence; the spec correctly says "mirror the *pattern*," not reuse the word.)

**5. CLI path present + gated first.**
- `crates/btctax-cli/src/cmd/admin.rs:56–57` is the current interim `PseudoActiveExport` return, sitting after `session.project()` (`:55`) and **before** `session.vault().export_snapshot` (`:59`) / `write_csv_exports` (`:84`) — so a refusal leaves `out_dir` untouched. The spec's replacement `if state.pseudo_active() { require_attestation(attest)?; }` (line 44–47) preserves that order. ✓

**6. `btctax-tui-edit` genuinely out of scope.**
- Its mechanized source-gate `crates/btctax-tui-edit/src/edit/persist.rs:1757` forbids `["export_snapshot", "write_csv_exports", "write_form_csvs"]` in non-test source — the editor has **no** form-export path. Its writes are vault-event mutations + `fs::write(&probe, b"x")` permission probes (e.g. `persist.rs:2493`), neither of which produces form/data files. ✓ Spec line 24 is correct.

C1 fully folded — the "sole path" / "no GUI crate" falsehoods from round-1 are gone; scope/SemVer/gotchas all now say **cli + tui**.

---

## I1 — wrong non-interactive `--attest` → `AttestationFailed` — RESOLVED

Mechanism lines 37–41 now decide `Some`/`None` in the **pure helper**, before any TTY consideration:
- `attest.map(str::trim) == Some(ATTEST_PHRASE)` → `Ok(())`;
- `Some(_)` non-matching → `Err(AttestationFailed)` ("a wrong phrase is FAILED regardless of env");
- `None` → `Err(AttestationRequired)`.

This matches the KAT `export_pseudo_active_wrong_phrase_refused` (line 66 — `Some("i attest…")` / trailing-junk → `AttestationFailed`, which runs non-interactively) and the Gotcha (line 94). Unambiguous. ✓ The round-1 contradiction (wrong `--attest` routed to `AttestationRequired`, failing the wrong-phrase KAT and undermining the ★ fault-inject) is gone.

---

## I2 — no TTY read in the library path (KAT-deterministic) — RESOLVED

Mechanism lines 37–43 make `require_attestation(attest: Option<&str>) -> Result<(), CliError>` **exact-compare only, no TTY read**; the prompt lives in the `ExportSnapshot` **main.rs arm** (line 46–49, cites `main.rs:291–294`) and the TUI modal. Verified:
- `crates/btctax-cli/src/main.rs:291–294` **is** the `Command::ExportSnapshot { out, tax_year }` arm calling `cmd::admin::export_snapshot(...)` — the correct home for a `--attest`-absent + TTY prompt, mirroring the established plan→prompt→apply shape at `main.rs:692–701` (`print!` → `stdout().flush()` → `stdin().read_line`). ✓
- Keeping the compare out of the library preserves the documented I/O-explicit invariant (`crates/btctax-cli/src/lib.rs:3`) and makes the KATs (which pass explicit `Some`/`None`) deterministic — no env-dependent `is_terminal()` hang. ✓

---

## I3 — plan enumerates the compile-breaking sub-2 KAT + all `PseudoActiveExport` refs — RESOLVED / complete

Removing `PseudoActiveExport(usize)` breaks exactly three references — grep (`grep -rn PseudoActiveExport crates/`) returns precisely:
- `crates/btctax-cli/src/cmd/admin.rs:57` (the block being replaced),
- `crates/btctax-cli/src/lib.rs:61` (the variant definition, doc lines 55–60),
- `crates/btctax-cli/tests/pseudo_reconcile_cli.rs:146–147` inside `export_snapshot_refused_while_pseudo_active` (test body `:138–162`; assertion `matches!(err, …PseudoActiveExport(n) if n > 0)` at `:146`).

Spec §[R0-I3] (lines 57–61) names the test at `pseudo_reconcile_cli.rs:138–162`, states the only refs are `admin.rs:57` / `lib.rs:61` / that test, and Plan T1 (lines 83–85) explicitly calls out **rewriting** it; the KAT list (line 69) renames it `attest_gate_supersedes_interim_i3_refusal`. All three covered. ✓ Complete.

---

## M1 / N1 — strings built from `ATTEST_PHRASE`; not-pseudo KAT reworded — RESOLVED

- **M1:** line 31 — "the prompt + error strings are BUILT from `ATTEST_PHRASE` (a KAT asserts they contain it — no drift)"; KAT `attest_strings_contain_phrase` (line 70). This retires the current hard-coded literal in `lib.rs:58–61` (the `PseudoActiveExport` message text) by constructing the new `AttestationRequired`/`AttestationFailed` copy from the const. ✓
- **N1:** KAT `export_not_pseudo_active_ignores_attest` (lines 68–69) is reworded to "fully-real ledger exports with no `--attest`; **same file SET, not byte-identical** — sqlite embeds timestamps." The unassertable byte-identity claim is gone. ✓

---

## Self-consistency + new-gap sweep

- **Scope consistent cli+tui throughout.** Goal (form CSVs + snapshot SQLite + projection CSVs), §[R0-C1] (both paths), Mechanism (CLI + TUI bullets), Errors, Scope/SemVer (line 76 "btctax-cli … + btctax-tui"), Plan (T1/T2 CLI, T3 viewer modal), Gotchas ([C1] TWO paths). No residual "sole path" / "no GUI crate" contradiction. The trigger (`pseudo_active()` = synthetic-count > 0), check-first ordering, and output-stays-clean guard are stated identically in every place they appear. ✓
- **No other form/data-file writer missed.** Production callers of the two form writers are exhaustively: `write_csv_exports` → `crates/btctax-cli/src/cmd/admin.rs:84` (CLI, gated); `write_form_csvs` → `crates/btctax-tui/src/export.rs:143` (TUI, gated). No other production caller exists (remaining hits are `render.rs` defs, tests, the tui/tui-edit source-gate token tables, and doc comments). `report --tax-year` renders to stdout (no `--out`), `backup-key` exports the key armor (not tax numbers) — both correctly ungated, unchanged from round-1's verified-clean list. ✓
- **Read-only-viewer-now-prompts** — noted, not a finding. `btctax-tui` bills itself "read-only" (`Cargo.toml:7`) yet already writes the four form CSVs on `e` (an existing, module-documented write surface — `export.rs:1–10`). Adding a typed-word branch on the pseudo-active path changes the *export ceremony*, never the read-only-**vault** posture (still never writes the vault). The spec already describes the branch (plain Enter/Esc when not pseudo-active; typed-word when pseudo-active), so the UX shift is captured, not hidden. No action required.

---

## Nits (non-blocking; fold at author's discretion during Plan/Implement)

- **Nit-1 — crate qualification on the `pseudo_active` cite.** Spec line 27 cites `state.rs:268`; the definition is `crates/btctax-core/src/state.rs:268–270`, whereas every other file cite in the spec (`admin.rs`, `lib.rs`, `main.rs`, `config.rs`, `session.rs`) is `btctax-cli`. Line number is correct; adding "(btctax-core)" removes a reader's ambiguity. Cosmetic.
- **Nit-2 — implicit `pub` visibility for the shared symbols.** Line 53 says the viewer "uses the shared `ATTEST_PHRASE`/exact-compare (btctax-tui depends on btctax-cli)." For that to compile, both `ATTEST_PHRASE` and `require_attestation` must be `pub` at the `btctax-cli` crate root (not `pub(crate)`). Strongly implied by "shared" + "depends on," but the spec never states the visibility. A one-word Plan note ("export both `pub`") would pre-empt a compile stumble. Implementation-detail level.

---

## Verified clean (carried forward + re-checked this round)

- Guard order (`admin.rs:55–59/84`) — refusal before any bytes; the replacement preserves it. ✓
- TUI gate-before-write — `do_export` does `mkdir_owner_only_exclusive` first (`export.rs:121`); gating at the modal's Enter (before `do_export` is called at all) leaves the export dir uncreated on a wrong/absent phrase. Consistent with "out_dir untouched." ✓
- Trigger completeness — `pseudo_active()` catches the C1 basis-taint case (real Sell on a pseudo $0 lot); a real ledger with the mode flag on but 0 synthetics → not gated. ✓
- Exit mapping — `main.rs:38–46` (`Err(e) => eprintln!("error: {e}"); ExitCode::from(2)`) matches spec line 55. ✓
- Output-cleanliness KAT re-run — `pseudo_marker_on_screen_but_absent_from_every_export_file` (`pseudo_reconcile_cli.rs:80–123`) already asserts the writers omit the marker (it deliberately bypasses the command-level refusal, anticipating sub-3's attest-gated writes) — still green under the new gate. ✓

## Recommendation

**R0-GREEN.** All of C1/I1/I2/I3/M1/N1 are resolved and confirmed against source at `7ad6155`; no new Critical or Important gaps. The two Nits are optional polish. The spec may proceed to the implementation plan (T1 → T2 → T3 as written).
