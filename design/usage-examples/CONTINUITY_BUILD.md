# Usage-examples BUILD — CONTINUITY (implementation resume point)

*Written 2026-07-17. This is the **implementation-phase** resume point (the brainstorm-era `CONTINUITY.md`
is historical). The autonomous build is mid-P1. Everything is committed on branch `feat/usage-examples`
and pushed to `origin`. Safe to clear context here.*

## ▶ KICK-OFF — paste into a FRESH session in `/scratch/code/bitcoin_tax`

> Resume the autonomous usage-examples build on branch `feat/usage-examples`. Read
> `design/usage-examples/CONTINUITY_BUILD.md` then `design/usage-examples/IMPLEMENTATION_PLAN_usage_examples.md`.
> P0 + 5 of 6 journeys are done and green; **next is J6** (the full TY2024 return = all 14 census forms),
> then Task 1.4–1.6, P2, P3, P4, and the v0.7.0 release. Proceed straight through, keeping the standing
> discipline: each phase gets an independent **Fable** review to 0 Critical / 0 Important (persist verbatim
> under `reviews/` before folding; re-review after every fold). The crates.io token is held (release step
> does not pause). Bugs the authoring surfaces → FOLLOWUPS (don't inline-edit the engine).

## The mandate

User: **"proceed autonomously through merge, tag and release"** + **"straight through"** (don't pause for
per-gate sign-off) + **"you hold a crates token, not expired"** (release step publishes to crates.io
without pausing). Follows `STANDARD_WORKFLOW.md`; reviews use **Fable**.

## State (branch `feat/usage-examples`, pushed to origin; `main` untouched)

**DONE + green:**
- **SPEC** (`SPEC_usage_examples.md`) + **PLAN** (`IMPLEMENTATION_PLAN_usage_examples.md`) — both Fable
  0C/0I. Reviews in `reviews/` (spec-r0, spec-r1-rereview, plan-r0, plan-r1-rereview, plan-r2-rereview,
  p0, + the fable-clock-seam-ruling).
- **P0 — the `BTCTAX_NOW` seam** — `e5a182f` (Task 0.1 seam), `27b43f7` (Task 0.2 integrity KAT + man
  ENVIRONMENT), `ad2b9b3` (review-fold). Fable P0 review GREEN 0C/0I. UX-P0-1 resolved.
- **Pre-existing bug fixed** — `909ded7` regenerated a stale `btctax-update-prices.1` (v0.6.1 release
  skipped man-page regen → `gen_docs_is_deterministic` was RED on main). Filed UX-P0-3.
- **P1 generator** (`crates/xtask/src/examples.rs`, wired in `main.rs`, `f9f1c71`) + **5 of 6 journeys**,
  each byte-deterministic, `make check` green (1941 tests):
  - J1 single-buyer happy path (`2ff3c92` + `8c45c81` CRLF-const fix)
  - J2 §170(e) donation → Form 8283 (`8de120c`, + shell-quote fix)
  - J3 self-transfer reconcile — hard blocker → resolved (`4c97e1d`)
  - J5 optimize + attestation + what-if (`e26001b`)
  - J4 staking income → Schedule SE (`2b05006`)

**Bug-hunt findings filed** (FOLLOWUPS): UX-P0-3 (release man-page drift), UX-P1-2 (a man page
contradicting current `export-irs-pdf` behavior), UX-P1-3 (`reconcile reclassify-outflow --amount` is the
USD FMV but undocumented; passing sats silently yields a $100M deduction — a footgun, not an engine bug).

## ★ Journey-authoring learnings (do NOT relearn)

- **Corpora = EMBEDDED Rust consts with `\r\n`** in examples.rs (NOT committed `.csv` — `.gitattributes`
  `* text=auto eol=lf` force-LF's committed CSVs and breaks the Coinbase parser; follow `fixtures.rs`).
- Generator emits **shell-quoted** commands (`shell_quote()`) so event refs (`|#:`) + spaced names
  copy-paste. The determinism test (`generate()×2 ==`) is `#[cfg(unix)]` (I4 — export paths use `.display()`,
  which differs on Windows).
- **`report --tax-year Y` needs a `tax-profile` first** (else "NOT COMPUTABLE [TaxProfileMissing]"). Canonical:
  `tax-profile --year 2025 --filing-status single --ordinary-taxable-income 100000 --magi-excluding-crypto
  100000 --qualified-dividends 0`.
- **Form year must be in `SUPPORTED_YEARS` = [2017, 2024, 2025]** (the 2026 date in fixtures.rs won't emit forms).
- **Donations:** `reconcile reclassify-outflow <ref> --as-kind donate --amount <USD-FMV>` (--amount is the
  **USD** proceeds/FMV, NOT sats) + `set-donation-details <ref> --donee-name … --appraiser-name …` (both
  required). >$5,000 fires a rich `[QualifiedAppraisalNote]`.
- **Income (missing FMV / business):** income comes from the **River** adapter (Coinbase has no income
  type). River income CSV header: `Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee
  Amount,Tag` with `Tag=income`. FMV resolves from the bundled dataset (dense through 2026-06); off-dataset
  → Missing (but that's an unsupported year). `reconcile reclassify-income <ref> --business true --kind
  mining` → Schedule SE. Income refs embed the ms-timestamp of the received date (deterministic).
- **Optimize:** the no-election default is **HIFO** (already tax-optimal → nothing to propose). To get a
  changed-row proposal you MUST first set a **FIFO baseline**: `config --set-forward-method fifo
  --effective-from 2025-01-01`. `optimize accept` also needs a **tax-profile**. `optimize accept
  --tax-year 2025` recomputes internally (no `--disposal`/prior `optimize run` needed). Backdated
  `BTCTAX_NOW` (≤ sale) ⇒ persisted `[Contemporaneous]`; postdated (> sale) ⇒ skipped "already executed".
- **what-if sell** needs `--wallet exchange:coinbase:default` (mandatory post-2025) + `--sell 0.5` (a `.`
  = BTC, bare int = sats).
- **stderr:** `export-irs-pdf` prints the NOT-AUTHORISED notice + 1099-DA caveat on **stderr** — capture
  via the generator's `show_stderr: true` (labelled block).
- **Determinism harness (SPEC §3.3):** `BTCTAX_PASSPHRASE=pw`, `BTCTAX_PRICE_CACHE`→nonexistent, `HOME=cwd`,
  `TZ=UTC LC_ALL=C LANG=C`, relative `--vault v.pgp`/`--out`. `BTCTAX_NOW` (RFC3339) pins decision dates;
  its stderr banner is discarded unless `show_stderr`.

## NEXT — J6 (the hardest; gates the P2 forms-census)

Goal: a TY2024 return emitting **all 14 census forms** (`f1040, f1040s1, f1040s2, f1040s3, f1040sa,
f1040sb, f1040sc, schedule_d, f8949, schedule_se, f8995, f8959, f8960, f8283`). Recipe (SPEC §4.2/§6.1):
1. `income import --year 2024 --file inputs.toml` — the non-crypto ReturnInputs (wages, interest→SchB,
   high income→8959/8960, business→SchC/SE/8995), sourced from
   `btctax_core::tax::testonly::kitchen_sink_household().0`. **Commit the TOML as a file** (LF is fine for
   TOML) so the **I6/M7 oracle-equality test** (a btctax-cli integration test) can parse it and assert
   `toml::from_str::<ReturnInputs>(committed) == kitchen_sink_household().0` (do NOT compare the PII-masked
   `income show` JSON — M8).
2. A crypto **disposition** (Schedule D / 8949) + a **donation leg** so Schedule A line 12 noncash > $500
   emits **f8283** (kitchen_sink alone = 13/14; the donation supplies the 14th — SPEC §6.1 table).
3. `export-irs-pdf --out irs --tax-year 2024` (full-return path — writes `{seq}_{name}.pdf` stems; the
   census keys off THOSE, J6 only).
4. Verify the emitted set == the 14 (Task 2.1 census assertion). Empirically confirm each form emits
   before committing (like the earlier journeys).

## Remaining after J6

- **Task 1.4** — commit the golden `docs/examples/examples.md` (`cargo run -p xtask -- examples > …`) +
  the `regen==committed` test + determinism proofs (double-regen, cross-HOME, price-cache-present/absent)
  in `crates/xtask/tests/examples_golden.rs`, all `#[cfg(unix)]`; golden + test in the SAME commit (I6).
- **Task 1.5** — groff `make examples` target (wrap verbatim blocks in roff `.nf/.fi`; `groff -k -man -T
  pdf`; PDF not byte-gated).
- **Task 1.6** — reconcile FOLLOWUPS (UX-P1-1/-2/-3) + independent **Fable P1 review** → 0C/0I.
- **P2** — CI `examples` job (`git diff --exit-code docs/examples`) + forms-census (scan J6 packet
  manifest ONLY, exact `{name}` match; enumerate 14 via an all-arms `PrintedReturn` fixture asserting
  count==14) + subcommand-coverage report (soft); born-green + perturb-one-byte→RED proof; Fable review.
- **P3** — TUI: a shared clock helper over the ~24 `now_utc()` sites (`btctax-tui`/`btctax-tui-edit`,
  incl. `lib.rs:247,256`, `export.rs:30`, `tui-edit/main.rs:2609`), then style-aware `TestBackend` capture
  (glyphs + per-cell fg/bg/modifier) → `docs/examples-tui/` goldens; Fable review. (UX-P3-1.)
- **P4** — the adversarial workaround-audit (`reviews/tutorial-workaround-audit.md`) → file findings; then
  the **whole-branch Fable review** → 0C/0I.
- **Merge → tag v0.7.0 → release** — bump all 10 crates to 0.7.0, **regen both goldens AND the man pages
  in the bump commit** (they embed `CARGO_PKG_VERSION` — UX-P0-3), `gh release` + attach the example PDFs,
  `cargo publish` (token held; per [[crate-publishing-state]]: `--workspace` can internal-error at the tail
  → resume `-p <crate>`; verify index with `grep -c`; remind user to revoke the token after).

## Validation / commands

- Fast gate: `make check` (nextest + clippy, ~8-16s). Regenerate examples: `cargo run -p xtask -- examples`.
  Examples test: `cargo test -p xtask examples`. The btctax binary the generator runs is built by a nested
  `cargo build -p btctax-cli` (`CARGO_BIN_EXE` isn't set for xtask).
- Everything is committed + pushed. `git log --oneline main..HEAD` shows the 15 build commits.
