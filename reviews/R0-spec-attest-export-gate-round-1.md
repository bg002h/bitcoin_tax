# R0 review — SPEC_attest_export_gate.md — round 1

**Artifact:** `design/SPEC_attest_export_gate.md` (DRAFT)
**Baseline verified against:** branch `feat/attest-export-gate` @ `fee11a3` (spec claims `main`==`afb0807`; `git log` confirms `afb0807` is the sub-2 merge parent). Read-only architect pass.
**Bar:** 0 Critical / 0 Important.

## Verdict: **1 Critical / 3 Important / 1 Minor / 1 Nit — NOT GREEN**

The gate is well-shaped for the CLI path, but the spec's central scope premise ("`export-snapshot` is the SOLE path that writes data/form FILES", "btctax-cli only … no GUI crate") is **factually false**: the shipped read-only viewer `btctax-tui` writes the exact same Form 8949 / Schedule D / 8283 / SE CSVs via a keybinding, with no gate, and its projection honors the vault's pseudo flag. That is the accidental-filing door this whole sub-project exists to close. Fix C1 before Plan. Three Important design/KAT inconsistencies compound it.

---

### [C1] CRITICAL — the gate misses the `btctax-tui` viewer's form-export path (the door it exists to close)

The spec asserts (goal + scope + SemVer):
- line 21–22: "**Scope = `export-snapshot` only** — it is the sole path that writes data/form FILES."
- line 63–66: "btctax-cli only … **No core change** … **No GUI schema_mirror (no GUI crate)**."
- goal line 11: it gates "the tax-year **Form 8949 / Schedule D / 8283 / Schedule SE forms**."

All three are contradicted by current source. There is a **second, ungated IRS-form writer**:

- `crates/btctax-tui/src/export.rs:115` `do_export()` → `crates/btctax-tui/src/export.rs:143` `btctax_cli::render::write_form_csvs(...)` writes `form8949.csv`, `schedule_d.csv`, `form8283.csv`, `schedule_se.csv` — the identical forms the spec enumerates.
- Reached from the viewer: `crates/btctax-tui/src/lib.rs:246` (`KeyCode::Char('e')` opens the modal) → `crates/btctax-tui/src/lib.rs:169–181` (Enter → `export::do_export`). The modal is a plain Enter/Esc confirm — **no attestation, no pseudo check** anywhere in `do_export` (export.rs:115–152).
- The viewer's projection **honors the stored pseudo flag**: `crates/btctax-tui/src/unlock.rs:173` `session.load_events_and_project()` → `crates/btctax-cli/src/session.rs:461–464` reads `self.config()?.to_projection()` → `crates/btctax-cli/src/config.rs:125–126,40` maps the persisted `pseudo_reconcile` key into `ProjectionConfig`. So `snap.state.pseudo_synthetic_count > 0` whenever the vault flag is on.

Fully reproducible bypass of the entire gate:
1. `btctax reconcile pseudo on` (persists the vault flag)
2. open the same vault in the `btctax` viewer → press `e` → `Enter`
3. fictional `form8949.csv` / `schedule_d.csv` / `schedule_se.csv` land on disk — one keypress, **no attestation**.

This is exactly "making an accidental filing impossible" (goal line 14) being **not met**, and it matches the design-of-record, which does NOT scope sub-3 to the CLI: brainstorm line 15 — "Producing `export-snapshot` **or any IRS-form output** requires the user to TYPE …", and line 84–88 — "Producing `export-snapshot` / **any IRS-form output** when the ledger is pseudo-active … requires typing." The TUI form export is squarely "any IRS-form output." (Note: sub-2's interim [I3] guard was also CLI-only, so this hole is inherited — but brainstorm line 81 explicitly says the taint/accidental-filing risk is "enforced by the attest gate (**sub-3**)". Sub-3 is the place to close it.)

**Fix:** extend the gate to `btctax-tui`. Concretely: in `export::do_export` (or at the `e`→modal open in `lib.rs:246`), when `snap.state.pseudo_active()`, require the same typed attestation before `write_form_csvs` (a typed-phrase modal, since the viewer has no `--attest`/stdin line — mirror the safe-harbor typed-word modal already used in `btctax-tui-edit`). Add a KAT proving a pseudo-active `do_export` writes NO form file without attestation, and correct the spec's scope/SemVer/lockstep: it is **not** btctax-cli-only, there **is** a GUI crate, and `export-snapshot` is **not** the sole form writer. If the team consciously defers the TUI to a follow-up, that must be an explicit, user-approved decision that **documents the open hole** — the current spec instead denies the hole exists, which is the dangerous part. (`btctax-tui-edit` is clean: its `edit/persist.rs:1708–1757` source-gate forbids `export_snapshot`/`write_csv_exports`/`write_form_csvs`, so it has no form-export path — only `btctax-tui` does.)

---

### [I1] IMPORTANT — mechanism vs KAT vs error-variant: wrong non-interactive `--attest` yields the wrong error

The three-branch `require_attestation` (spec line 33–37) routes a **provided-but-wrong** `--attest` (non-TTY) to the else-arm:
> line 37: "else (non-interactive, **no/wrong `--attest`**) → `Err(CliError::AttestationRequired)`"

But the KAT and the variant semantics both demand `AttestationFailed` for a wrong phrase:
- KAT (line 52–53): "`export_pseudo_active_wrong_phrase_refused` — `--attest "i attest this is true"` / … → **`AttestationFailed`** (exact, trimmed, case-sensitive)."
- variant doc (line 38–39): "`AttestationRequired` (non-interactive, phrase **missing**) + `AttestationFailed` (phrase **typed but wrong**)."

A wrong `--attest` **is** "phrase typed but wrong," but the mechanism only ever produces `AttestationFailed` from the TTY branch. As written, the wrong-phrase KAT (which runs non-interactively) would get `AttestationRequired` and **fail**. The ★ fault-inject target is also undermined: it keys off the wrong-phrase KAT.

**Fix:** branch on `Some`/`None` before TTY, so "typed but wrong" is TTY-independent:
```
Some(s) if s.trim()==ATTEST_PHRASE  -> Ok
Some(_)                              -> Err(AttestationFailed)   // typed but wrong, any env
None (missing)                        -> [main.rs decides: TTY prompt | AttestationRequired]  // see I2
```

---

### [I2] IMPORTANT — TTY read inside the library function makes the KATs non-deterministic and breaks the I/O-explicit invariant

The spec puts the TTY detection + `read_line` **inside `cmd::admin::export_snapshot`** (via `require_attestation`, spec line 28–37). Two problems:

1. **Non-deterministic tests (NFR4).** The KATs call `cmd::admin::export_snapshot(...)` directly (today: `crates/btctax-cli/tests/pseudo_reconcile_cli.rs:144`). Under `cargo test` in an interactive shell, `std::io::stdin().is_terminal()` is **true**, so `export_pseudo_active_missing_attest_refused_out_dir_untouched` (attest `None`, expecting `AttestationRequired`) would instead take the prompt branch and **block on `read_line`** (hang), while CI (stdin not a tty) passes — an environment-dependent flake in a tax-liability gate.
2. **Layering.** The library is documented I/O-explicit: `crates/btctax-cli/src/lib.rs:3` ("The library is I/O-explicit and deterministic"), and `crates/btctax-cli/src/session.rs:2` ("passphrase is ALWAYS a parameter — production resolves it in `main` (prompt/env); tests inject"). Every existing interactive prompt lives in the binary and follows plan→prompt→apply, e.g. `crates/btctax-cli/src/main.rs:692–701` and `:764–772` (print → `stdout().flush()` → `stdin().read_line` → `apply_*`). Reading a TTY inside `admin.rs` departs from that established pattern.

**Fix:** keep the library pure and put the interaction in `main.rs`, mirroring the existing pattern. `require_attestation(attest: Option<&str>)` does exact-compare only (never touches stdin; `None`→`AttestationRequired`, folding I1). The `ExportSnapshot` arm (`crates/btctax-cli/src/main.rs:291–294`) resolves the interactive path: cheaply probe pseudo-active first (e.g. a `session.project()` / a small `is_pseudo_active` helper — the plan→confirm→apply shape already used for bulk ops), and if pseudo-active + `--attest` absent + `stdin().is_terminal()` → prompt, read one line, pass `Some(line)` down; otherwise pass the `--attest` value (or `None`) and let the library return `AttestationRequired`. This makes the KATs deterministic (they pass explicit `Some`/`None`, never a TTY) and keeps the fault-inject on a pure compare.

---

### [I3] IMPORTANT — plan omits updating the existing sub-2 KAT that asserts the old refusal (will not compile)

Replacing `PseudoActiveExport(usize)` (spec line 38) removes the variant. Current users of it:
- production: `crates/btctax-cli/src/cmd/admin.rs:57` (the block being replaced) + definition `crates/btctax-cli/src/lib.rs:61`.
- **test that will break:** `crates/btctax-cli/tests/pseudo_reconcile_cli.rs:138–162` `export_snapshot_refused_while_pseudo_active`, whose assertion `matches!(err, btctax_cli::CliError::PseudoActiveExport(n) if n > 0)` (lines 145–148) references the removed variant → **compile error for the whole `btctax-cli` test target**, and its "OFF ⇒ export proceeds" second half (lines 154–161) still passes but its "ON ⇒ refused unconditionally" premise is now wrong behavior.

The spec's plan adds a NEW `attest_gate_supersedes_interim_i3_refusal` KAT (line 56–57) but **nowhere says to update/replace the existing `export_snapshot_refused_while_pseudo_active`**. The task explicitly asked this be enumerated so the plan covers it.

**Fix:** the plan (T1/T2) must call out rewriting `crates/btctax-cli/tests/pseudo_reconcile_cli.rs:138–162` — either delete it (superseded) or convert it to the new gate semantics (pseudo-active + no attest → `AttestationRequired`, out-dir untouched; pseudo-active + correct attest → writes). Also confirm no other reference to `PseudoActiveExport` remains (grep shows only admin.rs:57, lib.rs:61, and this test).

---

### [M1] MINOR — prompt/error strings duplicate the phrase literal instead of referencing `ATTEST_PHRASE`

The spec has both the prompt (line 34–36) and the error variants (line 38–40) "name the exact phrase." If those user-facing strings hard-code `"I attest this is true"` separately from `const ATTEST_PHRASE`, a future edit to the const silently desyncs the guidance from the accepted value — the user would be told to type a phrase the compare no longer accepts.

**Fix:** build the prompt/error text from `ATTEST_PHRASE` (format), and add a tiny KAT asserting the `AttestationRequired`/`AttestationFailed` `Display` (and the prompt string) **contain** `ATTEST_PHRASE`, so the two can't drift.

---

### [N1] NIT — "byte-identical to today" is a by-construction claim the KAT can't literally assert

Spec line 55 / 12 says a not-pseudo-active export is "byte-identical to today," but `export_not_pseudo_active_needs_no_attest` (line 54–55) can only assert "exports with no `--attest`, succeeds" — `snapshot.sqlite` embeds timestamps, so true byte-identity isn't testable. It IS byte-identical by construction (the only change on the not-pseudo path is the skipped guard; the `attest` arg is unused there). Fine to keep, but reword the KAT intent to "the not-pseudo path ignores `--attest` and produces the same file set as before" (the sub-2 export KATs already pin output shape) rather than implying a byte-diff assertion.

---

## Verified clean (no finding)

- **Guard placement / order.** `admin.rs:56–57` is the sole CLI pseudo guard and runs at `admin.rs:55–58`, **before** `session.vault().export_snapshot` (`:59`) and `write_csv_exports` (`:84`) — a refused export leaves `out_dir` untouched. The replacement `if state.pseudo_active() { require_attestation(attest)?; }` preserves that order. Good. (See C1/I2 for where the interactive part must live.)
- **Trigger correctness.** `pseudo_active()` = `pseudo_synthetic_count > 0` (`state.rs:268–270`), set by the fold (`fold.rs:391`); it counts contributing synthetics, so it also catches the C1 basis-taint case (a real Sell on a pseudo $0 lot flags via its upstream synthetic — sub-2 fixture `pseudo_reconcile_cli.rs:80–96`). A real ledger with the mode flag on but 0 unresolved events → 0 → not gated → prompt-free — matches the settled decision (brainstorm line 15; spec line 17,75). Complete and correct signal.
- **`report --tax-year` is not a file writer.** `Report` (`cli.rs:50–75`) has no `--out`; it renders to stdout. The only production `write_csv_exports`/`write_form_csvs` callers are `admin.rs:84` (CLI) and `btctax-tui/src/export.rs:143` (the C1 path). Correctly not gating `report`.
- **`backup-key` correctly excluded.** `admin.rs:95–100` exports the passphrase-protected key armor, not fictional tax numbers — not an accidental-filing surface. Right call to leave it ungated.
- **Exit mapping.** Spec's "main() maps to exit 2, stderr" (line 40) matches `main.rs:42–43` (`Err(e) => eprintln!("error: {e}"); ExitCode::from(2)`).
- **`--attest` flag is non-colliding + threadable.** `ExportSnapshot` (`cli.rs:93–111`) has only `out` + `tax_year` today; adding `attest: Option<String>` is additive, and the unrelated `attest` on the disposal subcommand (`cli.rs:213`) is a different arm. MINOR bump (new capability + flag) is the right SemVer for the CLI change.

## Recommendation
Fold C1 (extend the gate to `btctax-tui` or make deferral an explicit, documented, user-approved decision — not a false "sole path" claim) and I1/I2/I3 before the Plan gate. I1+I2 collapse into one clean helper shape (pure `require_attestation` in the library; TTY prompt + non-TTY refuse resolved in `main.rs`, matching the codebase's plan→prompt→apply layering). Re-run R0 after the fold.
