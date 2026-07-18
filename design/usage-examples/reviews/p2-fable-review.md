# P2 review — forms-census (HARD) + subcommand report + CI `examples` job

**Reviewer:** Fable (independent — author ≠ reviewer). **Scope:** the one-commit P2 atom
`8a67ccc` (`crates/btctax-forms/tests/census.rs`, `crates/xtask/src/examples.rs` coverage
additions, `crates/xtask/src/main.rs` dispatch, `.github/workflows/ci.yml` `examples` job),
verified against SPEC §6.1/§6.2/§6.3/§9 and IMPLEMENTATION_PLAN Tasks 2.1–2.4, against live
source (not the commit message), with the validation suite and both negative proofs re-run
by the reviewer.

## Verdict

The atom is sound. The census is genuinely two-sided and loud in every failure direction I
could construct: `census_key_set_is_exactly_14` pushes a self-verifying all-arms fixture
(kitchen_sink's 13 arms + an explicitly injected `f8283`, with a premise assert that
kitchen_sink is still 13/14) through `fill_full_return` and asserts **both** `len == 14`
**and** exact set-equality against the §6.1 literal — a 15th form, a dropped arm, a renamed
stem, and a silently-unsatisfied non-Option gate all red (I verified all three non-Option
gates against `packet.rs`: `sch_d.must_file()` at :123, `fill_form_8959`'s internal
`Option` return at :149, and the `f8283` double-gate at :155–158; no push site can emit a
duplicate name, and `len==14` + set-equality would catch it even if one could).
`every_census_form_demonstrated_in_j6` scans **only** from `Full-return packet —` inside
the `## J6` section (the anchor is unique — line 181's "see J6" lacks `## `; no `^## `
line exists after the header; no `.pdf` line exists after the packet block), and the
digit-seq-prefix filter correctly admits `12A_f8949`/`155_f8283` while rejecting the two
bare/non-seq collision stems that actually occur in the golden (`irs/schedule_d.pdf` line
119, `irs/form_8283.pdf` line 186 — both also outside J6; double protection). I
independently re-observed both mandated negative proofs: appending one byte to the golden
makes the CI drift-gate command exit 1, and deleting the `155_f8283.pdf` line reds the
census naming exactly `["f8283"]`. The SOFT report's 17/46 split is genuine — I
cross-checked every one of the 17 covered leaves against a real `$ btctax …` line
(`what-if sell`, `income import`, `income show`, `optimize run`/`accept` all genuinely
demonstrated; `reconcile match-self-transfers` and `classify-inbound-income` correctly
listed uncovered) — and the tool exits 0 regardless of content. The CI job interpolates no
`github.event.*` (the only `${{ }}` uses are trusted contexts), pins the same
SHA-pinned actions and stable toolchain as the six sibling jobs, places `--locked`
correctly before `--`, installs full `groff` (ubuntu-latest ships only `groff-base`,
which lacks the pdf device; the Makefile fail-louds on a missing device via the `%PDF`
magic check and `mkdir -p`s the git-ignored `docs/pdf/`), and its hermeticity claim is
backed by the passing `examples_generate_is_hermetic_across_ambient_env`. Cross-machine
regen determinism is already exercised by `examples_golden_matches_committed` running in
the existing 3-OS `test` job's unix legs, and the golden contains no absolute/tempdir
paths. On mechanism choice: §6.2's ban is on using a *household's emitted packet as the
enumeration authority* (the silent-13 path); here the authority is the `CENSUS_KEYS`
literal and the household is only fixture *material*, with the `== 14` assert making any
fixture shortfall a red, not a silent under-gate — exactly the property §6.2's
fixture-authoring note demands. Option-1-compliant in substance. I also checked the one
platform the atom newly touches: `census.rs` is not `#[cfg(unix)]`-gated so it runs in the
Windows `test` leg — `str::lines()` strips `\r` (autocrlf-safe), the forward-slash
`CARGO_MANIFEST_DIR` path is Windows-valid, and the heavier `golden_packet` full-packet
fills already run green on Windows under libtest's 8 MiB test threads (the 1 MiB fix was
binary-only). `make check` green: **1947/1947 passed, 6 skipped**. Nothing in §6/§9 owed
to P2 was dropped: the mnemonic fail-safe guard and required-check promotion are owned by
Task 4.2 (P4), the TUI diff path by P3, and §6.4's year cross-check is a MAY.

## Findings

### Critical
None.

### Important
None.

### Minor

- **M-1 — silent 0/N on a missing golden, and the well-formedness test doesn't hold the
  line its comment claims.** `crates/xtask/src/examples.rs:573` reads the golden with
  `.unwrap_or_default()`, so a missing/unreadable `docs/examples/examples.md` yields a
  confident-looking "0/46 … have a worked example." instead of an error; and
  `subcommand_coverage_report_is_well_formed` (examples.rs:719–731) *says* "a 0/N report
  would mean the scan is broken" but asserts only the presence of the summary substring —
  it would pass on 0/N. Scenario: a future refactor moves the golden path; the SOFT CI
  step prints an absurd 0/46, nothing reds, and the maintainer's map is quietly garbage
  until a human reads the log. Advisory surface only (the HARD census reads the golden
  via its own loud panic path), so not gating. Owning phase: P4 residue.
- **M-2 — `is_demonstrated` counts a parent-level leaf covered via a nested command
  line.** examples.rs:558–575: the in-order-subsequence match allows arbitrary tokens
  *before* `path[0]`, so leaf `["import"]` is satisfied by the line
  `$ btctax income import --year 2024 …` (examples.md:445). Today every one of the 17
  covered leaves is independently, genuinely demonstrated (verified line-by-line), so the
  current count is honest; the failure scenario is future drift — J1 drops bare `import`
  while J4 keeps `income import`, and the report still claims top-level `import` covered.
  SOFT/non-blocking by spec, so Minor. A stricter matcher would require `path[0]` to be
  the first non-`-`-prefixed token. Owning phase: P4 residue.
- **M-3 — the CI drift-gate's stdout purity rests on cargo's stream contract, with a
  false-RED (not silent-corruption) failure mode.** ci.yml:107–110 redirects the whole
  `cargo run … -- examples` process tree to the golden, and `built_btctax()`
  (examples.rs:30–43) runs a nested `cargo build` via `.status()`, which *inherits* the
  redirected stdout. Cargo documents build progress/diagnostics on stderr (the in-file
  comment says so, accurately), and if that contract ever broke the result is a loud
  drift-gate RED in an advisory job — never a silently-committed corrupt golden (the
  committed file is produced locally via the in-process `regen == committed` test, which
  bypasses the shell redirect). Recorded as a Minor because the local test and the CI
  step regenerate through *different plumbing*; `.stdout(Stdio::null())` on the nested
  build would collapse the difference. Owning phase: P4 residue.

### Nit

- **N-1** — the nested `cargo build -p btctax-cli` (examples.rs:35) lacks `--locked`; the
  outer `cargo run --locked` has already validated the same workspace lockfile in the
  same job, so no drift is possible in-CI — hygiene only.
- **N-2** — ci.yml:109 diffs `-- docs/examples/examples.md` while SPEC §9 writes
  `git diff --exit-code docs/examples docs/examples-tui`; equivalent today (the generator
  writes exactly one file; `man-wrap.awk` is untouched), but P3 must remember to widen
  the diff to `docs/examples-tui` when the TUI golden lands. Owning phase: **P3**.
- **N-3** — plan Task 2.2 names `crates/xtask/src/examples/mod.rs`; the code lives at
  `crates/xtask/src/examples.rs` (P1's actual layout). No substance; noting so the plan's
  citation drift doesn't confuse a later reconciliation.

## Validation evidence (re-run by the reviewer, not taken from the commit message)

- `make check` — **1947 run: 1947 passed, 6 skipped** (green; includes both census tests
  and all three P1 determinism/hermeticity proofs).
- `cargo test -p btctax-forms --test census` — 2 passed.
- `cargo run -p xtask -- subcommand-coverage` — prints 17/46 + 29 named uncovered leaves;
  exit 0.
- Negative proof 1 — one byte appended to `docs/examples/examples.md` →
  `git diff --exit-code -- docs/examples/examples.md` exits **1**; restore → 0.
- Negative proof 2 — `155_f8283.pdf` line removed →
  `every_census_form_demonstrated_in_j6` **FAILED**: `census forms undemonstrated in
  J6's packet: ["f8283"]`; restore → clean tree.

## Gate

**0C / 0I — P2 GREEN.** (3 Minor, 3 Nit recorded above with owning phases; none holds the
gate. M-1..M-3 → P4 residue burndown; N-2 → P3.)
