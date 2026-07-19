# Phase 2 ("Legibility") independent re-review r3 — r2-I1 fold verification

Reviewed fold commit `99043a3` on top of `579358d`; full phase-2 diff cross-checked against current source at HEAD. Independent validation: `make check` exit 0 — **2039 passed / 8 skipped** (exactly r2's 2038 + the 1 new KAT), clippy `-D warnings` clean (no warning/error markers anywhere in the captured log). The mutation claim was **independently re-proven live** (arm reverted via scripted edit, suite run, file restored from cp-backup; `git status` clean at HEAD afterward).

## Fold-claim verification (all confirmed against source)

- **Or-pattern correctness** — `crates/btctax-cli/src/lib.rs:173-184`: `CliError::Io(source) | CliError::Store(btctax_store::StoreError::Io(source))` is valid Rust and binds consistently: both variants wrap `std::io::Error` directly (`CliError::Io(#[from] std::io::Error)`, cli `lib.rs:44-45`; `StoreError::Io(#[from] std::io::Error)`, store `lib.rs:19-20`), so `source: std::io::Error` in both arms; compiles clean under `-D warnings`. `other => other` still passes `Csv` and every remaining variant through.
- **No over-enrichment** — `cli_io_with_path` has exactly ONE call site (grep-swept: `admin.rs:130`, the `write_csv_exports` wrap). I enumerated every error that can escape `write_csv_exports` and its callees (`render.rs:681/683/708/749/800`, `write_form8949_csv:1041`, `write_schedule_d_csv:1085`, `write_form8283_csv`, `write_schedule_se_csv`): the only `StoreError` sources are `fsperms::mkdir_owner_only(out_dir)` / `open_owner_only(out_dir/…)` — all genuinely out-dir write failures (fsperms can produce nothing but `StoreError::Io`); the only raw-`Io` sources are `w.flush()` and the 8283 `writeln!` — also out-dir writes; `write_record` → `Csv`, passed through. No `Store(Io)` with non-path meaning can reach the wrap. `store_io_with_path` is byte-unchanged (still enriches only `StoreError::Io`, `other → Store(other)`; all four call sites intact: `admin.rs:87/420/430`, `session.rs:393`).
- **KAT premise is real, not vacuous** — `export_out_subpath_collision_names_path` (`io_error_context.rs:67-98`): `Vault::export_snapshot` (`vault.rs:263-272`) uses tolerant recursive `mkdir_owner_only` + writes `snapshot.sqlite` (unobstructed), so the store-side wrap at `admin.rs:87` never fires; `write_csv_exports`' tolerant mkdir passes; `open_owner_only(out/lots.csv)` (create-truncate open on a directory) hits `EISDIR`. Proof it reaches the wrap: with the `Store(Io)` arm reverted, the KAT fails showing the unenriched pass-through — `io: Is a directory (os error 21)`, byte-for-byte the r2 symptom — while the other 3 KATs stay green. **Mutation-proven.**
- **Comment fix** — `admin.rs:117-121` now states the true split (mkdir/open → `Store(Io)` via `From<StoreError>`, flush/writeln → `Io`, both enriched; csv passes through). Matches the source trace above.
- **r2-N2 fix** — the `mkdir_out` unit test (`admin.rs:~600-608`) now asserts the literal `"does not already exist as a file"` (a real substring of `EXPORT_OUT_HINT`) instead of the self-referential const; emptying the const now reds it. The path assert (`contains("collide")`) is retained.
- **FOLLOWUPS entry** — accurate: r2-I1 + r2-N2 marked folded; r2-N1/N3 filed with later-cycle ownership, restated correctly.

## CRITICAL

None.

## IMPORTANT

None.

## MINOR

**M1(r3) — The subpath-collision class the fold just fixed for `export-snapshot` remains PATHLESS at the two PDF `--out` exporters.** `write_bytes_owner_only` (`admin.rs:404-411`) does `fsperms::open_owner_only(path)?` → `From<StoreError>` → `CliError::Store(StoreError::Io)`, and none of its call sites is wrapped by any path enricher — only their `mkdir_out(out_dir)` is: `export_irs_pdf`'s `f8949.pdf`/`schedule_d.pdf`/schedule-SE writes (~`admin.rs:268/278`), and `export_full_return`'s per-form loop + `manifest.txt` (~`admin.rs:530/540`). Exact input: `export-irs-pdf --tax-year Y --out out` with `out` an existing directory containing a **directory** named `f8949.pdf` → the store-side mkdir tolerates, then the identical pathless `io: Is a directory (os error 21)` escapes (same chain shape my mutation run displayed live). Rated Minor, not Important, deliberately: the SPEC's UX-P4-8 class term (§4 + §9.5) cites exactly vault-open + `admin.rs:82` + `render.rs:586-618` — the PDF subpath writes were never in the mandated site list; unlike r2-I1 there is NO wrap or comment advertising coverage that doesn't fire (r2-I1's gating essence was the advertised-but-dead guard + inverted comment); and r2-N3 set the precedent of filing same-shape out-of-class gaps as residue. Disposition required (Minors don't gate but must be recorded): either the one-line inline fix — enrich inside `write_bytes_owner_only` itself (`.map_err(|e| crate::cli_io_with_path(e, path, crate::EXPORT_OUT_HINT))`, which names the exact colliding file) plus a KAT — or a FOLLOWUPS entry owned by the later legibility-polish cycle that already owns r2-N1/N3.

## NIT

**N1(r3)** — The corrected comments call `csv::Error` "serialization, not a path problem", but `Csv(csv::Error)` can also carry an underlying `io::Error`: csv's `Writer` writes through its internal buffer mid-`write_record`, so a mid-write `ENOSPC`/`EIO` on a large CSV arrives as pathless `csv: <io>`, not `CliError::Io`. The passthrough was deliberately blessed in r1 and r2; noting only that the parenthetical is imprecise and this residual pathless class exists (exotic trigger). Sweep with the polish residue if ever.

**N2(r3)** — `cli_io_with_path`'s new doc comment says what it enriches but not the call-site precondition that makes the `Store(Io)` arm safe: every `Io`/`Store(Io)` reaching the wrap must genuinely be a write-at-`path` failure (true today at the single call site). A future caller wrapping a broader expression that can surface a vault-side `Store(Io)` would mislabel it with the wrong path/hint. One sentence ("wrap ONLY expressions whose every Io is a write under `path`") closes the latent hazard.

**N3(r3)** — Process hygiene: the r2 review was persisted in the SAME commit as the fold (`99043a3`), so git history cannot prove persist-before-fold precedence (earlier rounds used a separate persist commit, e.g. `158b040`). The artifact is present and internally consistent; non-gating.

## Standing invariants (re-checked)

- **§1 dollar-invariant holds** — the fold touches only error mapping, comments, tests, FOLLOWUPS, and the persisted review; golden-packet byte-reproducibility, oracle smoke, and examples goldens all passed in my independent run.
- **Fail-closed holds** — no over-masking (single call site, escaping-error enumeration above); `store_io_with_path` unchanged; curated store meanings (`WrongPassphrase`/`Locked`/`Corrupt`/…) unreachable by the new arm (fsperms emits only `Io`).
- **No regression** — 2039/2039 + clippy clean; test delta exactly the one claimed KAT; fold commit's file footprint matches its message.
- **Untested-guard honesty** — the `Store(Io)` arm is genuinely mutation-proven (independently re-executed). One remaining unenriched subpath class exists at the PDF exporters — recorded as M1, outside the phase's specced site list.

## STATUS

- **r2-I1 — RESOLVED.** The wrap now fires for its advertised `Store(Io)` class; pinned by a non-vacuous, mutation-proven KAT of the exact proven repro shape; the comment now matches reality.
- **r2-N2 — RESOLVED.** The unit test pins the hint's literal content; the self-referential assert is gone.

## VERDICT

**GREEN — 0 Critical / 0 Important.** M1(r3) must be recorded (one-line inline fix + KAT, or a FOLLOWUPS entry owned by the later legibility-polish cycle alongside r2-N1/N3); N1–N3 at the author's discretion. Phase 2 (UX-P4-7/8/9) passes the gate.
