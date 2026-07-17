# Fable independent re-review — IMPLEMENTATION_PLAN r1 (persisted verbatim)

*Persisted 2026-07-16 verbatim before folding, per STANDARD_WORKFLOW §2. Reviewer: Fable (independent).
Verdict: NOT GREEN — 0C / 1 Important (I9) / 1 new Minor (M7). I9+M7 folded → r2 → re-review.*

---

## VERDICT: 0 Critical / 1 Important / 1 new Minor — NOT GREEN. One more (small, mechanical) fold round.

### RESOLVED: I1, I3, I4, I5, I6, I7, I8 — and I2's original landmine (the stub) is gone

- **I1** — Task 0.1 Files lists only `main.rs` + the new test; the explicit note routes all man text to Task 0.2. Verified: `render_root` is at `crates/xtask/src/docs.rs:144-159` and stitches the hand-authored `ROOT_DESCRIPTION`/`ROOT_FILES`/`ROOT_EXAMPLES` consts (docs.rs:153-157, consts at :161/:186/:206); no ENVIRONMENT const exists yet, so "add a `ROOT_ENVIRONMENT` wired into `render_root`" is the correct instruction; `gen_docs_is_deterministic` at docs.rs:353; regen in the same commit. RESOLVED.
- **I2 (stub half)** — T-P0.6 now has a real body: `assert_ne!(back, post)` cannot pass empty, Step 2 honestly says "Run → PASS (born-passing)", Step 3 is the invert→RED→revert mutation-check. The surface is real: `Optimize::Accept { tax_year, disposal: Option, attest: Option }` (cli.rs:315-326) — `optimize accept --tax-year 2025` parses with no `--disposal`, and `accept_with_tables` **recomputes internally** via `optimize_year` (cmd/optimize.rs:198-208), so no prior `optimize run` is needed and the discovery hedge is moot for args. Import shape `import <path>` (positional `Vec<PathBuf>`, cli.rs:45) correct; no tax profile needed (`resolve_screened_profile` → `Ready{profile: None}` → `optimize_year(profile: Option<&TaxProfile>)`). **But the fold introduced a new defect in the same test — I9 below.**
- **I3** — nested `cargo build -p btctax-cli --bin btctax` via `env!("CARGO")` + `{CARGO_TARGET_DIR or ./target}/debug/btctax`. Verified `.cargo/config.toml` sets only lld rustflags (no `target-dir`), and the Makefile's `CARGO_TARGET_DIR=target-clippy` is clippy-only. Sound. RESOLVED.
- **I4** — `#[cfg(unix)]` on Task 1.4's regen/determinism tests + Task 1.1's spawning unit test, with the same conditional carried to Task 3.3. The basis holds: `main.rs:609-620` and `:640-648` print paths via `.display()`. RESOLVED.
- **I5** — `run_with_stderr(cmd, label)` labelled-fence mode + front-matter (env convention + honest passphrase sentence + Cargo.toml-sourced version). NO-AUTHORISATION `eprintln!` confirmed at main.rs:~622-634. Matches SPEC §3.3 verbatim. RESOLVED.
- **I6** — Task 1.2 equality test `ReturnInputs == kitchen_sink_household().0`. Verified: `kitchen_sink_household() -> (ReturnInputs, LedgerState)` at `testonly.rs:165`, and `testonly` is a plain `pub mod` (tax/mod.rs:29, not `#[cfg(test)]`) so cross-crate tests reach it (btctax-forms tests already do). RESOLVED (placement nit below).
- **I7** — Tasks 2.1/2.2 end "Stage (no commit — the P2 atom commits in Task 2.3)"; 2.3 is the one-commit atom with the perturb-one-byte proof. Matches SPEC §9. RESOLVED.
- **I8** — `gh release upload v0.7.0` of both PDFs in the release tail. RESOLVED.

**Minors/Nits spot-checked, all addressed:** M1 (`.gitignore:53` ignores only `docs/pdf/`; verify-not-ignored step present), M2 (`recorded 2025-05-01` matches `"  recorded {} effective {}"` at render.rs:2258; `TaxDate = time::Date`, ISO Display), M3 (census parses the committed golden's J6 packet stdout block; main.rs:640-648 prints the `{seq}_{name}.pdf` lines + manifest), M4 (price-cache present-vs-absent proof in Task 1.4), M5 (bump regenerates both goldens in the bump commit), M6 (HOME scrubbed in `run_in`), N1 (TOML path written separately), N2 (`tempfile` xtask dev-dep; census in btctax-forms avoids the core dep), N3 (`fill_full_return(&PrintedReturn, year)` — verified at `btctax-forms/src/packet.rs:36`).

### NEW Important

**I9 — T-P0.6's concrete fixture defeats its own assertion: `coinbase_buy_sell_send` is single-lot, so `optimize accept` skips as "already optimal" identically in BOTH runs — the KAT is born-RED, and the plan's escape hatch names the wrong failure mode.**
*Task 0.2 Step 1 (plan lines ~222-258); `crates/btctax-cli/src/cmd/optimize.rs:221-227`; `crates/btctax-cli/tests/optimize_run.rs:100-104, 347-370`.*
The fixture has ONE acquisition (0.10 BTC, 2025-03-01) before the 2025-06-15 sell, so `proposed_selection == current_selection` by construction — the codebase's own R2-M1 test documents exactly this shape ("single lot → proposed == current → no-change row", optimize_run.rs:347-370). In `accept_with_tables` the no-change skip (`"already optimal under current identification"`) fires **before** the persistability gate, so backdated and postdated runs print byte-identical `AcceptOutcome`s → `assert_ne!(back, post)` fails, and the mutation-check inverts to a false PASS — the exact inverse of what Step 2 ("Run → PASS, born-passing") and Step 3 promise. The Step 3 hedge only covers `--disposal`/prior-`optimize run` discovery, which is not the problem.
**Fix (mechanical):** swap the fixture for a two-lot changed-row CSV — `write_tax_saving_csv` (optimize_run.rs:85-98: LT lot 2023-01-01 + ST lot 2025-01-02 + 1 BTC sell 2025-06-01; HIFO proposes the ST lot ≠ FIFO baseline) lifted into `tests/fixtures.rs`. Backdated pin ≤ 2025-06-01 (the existing `2025-01-01T00:00:00Z` works); postdated > it. This also removes the pending-Send wrinkle. Simultaneously fix the comment: the Coinbase wallet **is** a broker (`is_broker` = `WalletId::Exchange`, optimize.rs:451-453) — the KAT satisfies SPEC §3.2's "**and/or a pre-2027 sale date**" arm, not the "non-broker wallet" arm the comment (inherited from my r0 wording) claims.

### NEW Minor (record; non-gating)

**M7 — Task 1.2's oracle-equality test has no named home crate.** It needs btctax-core (`testonly`, `ReturnInputs`) + a TOML parse; the natural parser (`parse_return_inputs_toml`, `cmd/tax.rs:114`) is private to btctax-cli. Placing the test xtask-side quietly adds the btctax-core dep Task 1.1 says isn't needed "here". Name the crate — e.g. a btctax-cli integration test reading the fixture via a relative path, or drive it through the binary (`income import` + `income show` JSON).

### Regression scan — clean otherwise

Task 1.1's `generate(bin: &Path)` ↔ Task 1.4's `generate(&built_btctax())` consistent; the btctax-forms census (N2/N3 relocation) still integrates with Task 2.3's CI job and is Windows-safe (it parses committed LF text, spawns nothing, and lands in P2 after the P1 golden exists); no fold made any citation stale (all r1-cited lines re-verified: docs.rs:144-159/261/353, optimize.rs:469-484, main.rs:609-648, testonly.rs:165, packet.rs:36, cli.rs:45/315).

---

**The single most important remaining thing:** replace T-P0.6's single-lot fixture with the two-lot `write_tax_saving_csv` shape (and correct the "non-broker" comment) — as folded, the plan's flagship KAT fails on its own specified inputs, in the phase that gates every golden.
