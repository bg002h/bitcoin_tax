# Phase 2 ("Legibility") independent re-review r2 — UX-P4-7 / UX-P4-8 / UX-P4-9 fold verification

Reviewed the fold commit `579358d` on top of `34e9945`/`66b4bad`/`fa4badc`, full phase-2 diff verified against current source at HEAD. Independent validation: `make check` green — **2038 passed / 8 skipped**, clippy `-D warnings` clean (parallel job, no failure marker) — exactly r1's 2033 + the 5 new tests the fold claims (`missing_vault_maps_to_concise_no_vault_message`, `mkdir_out_collision_names_path_and_hint`, `export_irs_pdf_out_collision_names_path`, `export_full_return_out_collision_names_path`, `backup_key_out_collision_names_path`). The gating defect below was **proven by live repro** (temporary test, run, then deleted; tree left clean).

## CRITICAL

None.

## IMPORTANT

**I1(r2) — The M1 fold wrote a comment that inverts reality, and it exposes that the `write_csv_exports` enrichment wrap cannot fire for its advertised class: an `open_owner_only` collision under `--out` still surfaces PATHLESS.**

The fsperms helpers return `StoreError` (`crates/btctax-store/src/fsperms.rs:22/73`). Inside `write_csv_exports` (`crates/btctax-cli/src/render.rs:681-683` etc., fn returns `Result<(), CliError>`), `?` on those converts via `From<StoreError>` into `CliError::Store(StoreError::Io(..))` — **not** `CliError::Io`. `cli_io_with_path` (`crates/btctax-cli/src/lib.rs:169-178`) matches **only** `CliError::Io`, so every `mkdir_owner_only`/`open_owner_only` failure inside `write_csv_exports` passes through the wrap at `crates/btctax-cli/src/cmd/admin.rs:128` unenriched. The wrap's only *live* inputs are the raw-`io::Error` sites — `w.flush()?` and the Form 8283 `writeln!(file, …)?` — i.e. exactly the "mid-write" class. The fold's rewritten comment (`admin.rs:117-119`) claims the opposite split: "name the --out path when `write_csv_exports` fails to CREATE/OPEN a file under out_dir (its `mkdir_owner_only`/`open_owner_only` `io::Error`s)". Both clauses about mkdir/open are false; only the csv-passthrough clause is true.

Proven repro (I ran this): `btctax init`; create `out/` as a real directory containing a **directory** named `lots.csv`; run `export-snapshot --out out`. `Vault::export_snapshot` succeeds (tolerant mkdir + `snapshot.sqlite` writes fine), then `write_csv_exports`'s `open_owner_only(out/lots.csv)` fails. Observed:

```
variant = Store(Io(Os { code: 21, kind: IsADirectory, .. }))
display = io: Is a directory (os error 21)
```

— pathless, hintless: byte-for-byte the UX-P4-8 symptom class, at an export-`--out` surface. The r1 collision KAT passes only because the *out_dir-itself* collision is caught earlier by the store-side wrap at `admin.rs:87` (`Vault::export_snapshot` mkdirs first, vault.rs:265) — the `cli_io_with_path` wrap contributed nothing to that KAT. For the record, r1's own M1 text asserted the wrong mechanism ("only the mkdir/open `io::Error` paths get the path"), and the fold codified that mis-analysis into the comment; under the untested-guard standard this wrap is a guard whose advertised trigger is unreachable and whose actual trigger (flush/writeln io) has no test.

Fold (mechanical): teach the wrap to also map `CliError::Store(StoreError::Io)` (extend `cli_io_with_path`, or apply `store_io_with_path`-style mapping to `write_csv_exports`' error), pin it with a KAT of the proven `lots.csv`-as-directory shape, and reword the comment to match reality. Alternative: remove the wrap, write an honest comment, and file the subpath-write class as a follow-up with an owning phase — but the current comment+wrap pair cannot stand as written.

## MINOR

None (beyond what the Important subsumes).

## NIT

**N1(r2)** — `map_open_error` (`crates/btctax-tui/src/unlock.rs:243-250`) retains two now-dead NotFound arms (`CliError::Store(StoreError::Io)` and `CliError::Io`): `Session::open` maps every `Vault::open` Io to `PathIo`, and `default_prices()` errors are `AdapterError → CliError::Adapter` (`session.rs:352-357`), so neither variant can reach the function. Harmless defensive residue; they even keep the pin green if the `Session::open` enrichment were ever reverted. Optionally collapse or annotate.

**N2(r2)** — `mkdir_out_collision_names_path_and_hint` (`admin.rs:587-601`) pins the hint self-referentially (`assert_eq!(hint, crate::EXPORT_OUT_HINT)` and `contains(crate::EXPORT_OUT_HINT)` both stay green if the constant is emptied — `contains("")` is always true). The hint's *content* is pinned solely by the literal `"does not already exist as a file"` in `io_error_context.rs::export_out_collision_names_path`. Fine as-is; noting so nobody later deletes that KAT believing the unit test guards the hint.

**N3(r2)** — `init --key-backup` (`cli.rs:39`, `key_backup: PathBuf`) writes a key to a user-named path and fails as pathless `StoreError::Io` on collision — same UX shape, but outside UX-P4-8's class term ("vault-open and export-out call sites"; the flag is not an `--out`). Candidate for the later polish cycle alongside the filed N1 residue. Likewise the non-NotFound `PathIo` on the unlock screen (permission-denied vault → `_` arm → long unwrapped line + an inapt `btctax init` hint) is the already-filed N1 residue, not new.

## STATUS OF r1 FINDINGS

- **I1 — RESOLVED.** The `PathIo`/NotFound arm sits before `_` (`unlock.rs:255-257`), matches a distinct variant so it cannot shadow the `WrongPassphrase`/`Locked`/`HalfCreatedVault` arms (all verified byte-unchanged); the `Locked` short-circuit in `open_session` (unlock.rs:122) still fires since `store_io_with_path` passes `Locked` through. The pin exercises the real `Session::open` on a real missing path and `assert_eq`s the exact concise string — deleting or inverting the arm reds it (the fall-through produces the long path+hint string).
- **I2 — RESOLVED.** All three sites wired: `mkdir_out` at `admin.rs:260` (crypto slice) and `admin.rs:508` (full return), `backup_key` wrap at `admin.rs:428`. The KATs reach *distinct* call sites — the full-return dispatch (`return_inputs::exists`, admin.rs:237) precedes the crypto-slice `mkdir_out`, so the 2024-with-inputs KAT genuinely hits `export_full_return`'s own `mkdir_out` and the 2025 KAT hits line 260. `backup_key`'s attribution is honest (`Vault::backup_key` touches only `out_path`/its parent for Io; the `Crypto` armor error passes through unmasked; the inner `Session::open` keeps `VAULT_OPEN_HINT` for the vault path). Sweep confirms the path-typed `--out` surface is exactly these three commands (cli.rs:128/191/224; the reconcile `out: String` args are event refs, not paths). Mutation-honest: an unenriched failure displays as pathless `io: <errno>` (proven shape above), so each KAT's path-`contains` reds if its site's wrap is reverted.
- **M1 — NOT RESOLVED.** The rewritten comment's csv clause is now correct, but its main clause is false in a new way, and checking it exposed the ineffective wrap — see I1(r2).
- **M2 — RESOLVED.** `export_out_collision_names_path` pins the literal hint clause `"does not already exist as a file"`; with `PathIo`'s Display `io {path}: {source} ({hint})`, emptying `EXPORT_OUT_HINT` reds it.
- **N1 — RESOLVED** (as residue): filed in FOLLOWUPS with later-cycle ownership, accurately restated.
- **N2 — RESOLVED.** `whatif.rs:16-17` now reads "(empty OR merely insufficient) ⇒ `NoLots { available, requested, .. }`" — matches the variant and raise.
- **N3 — RESOLVED** (as residue): filed as its own follow-up, owning phase = later polish cycle, with the correct caveat that optimize's "no feasible selection" is not a mechanical reuse of `no_lots_message`.

## Standing invariants (re-checked, stated plainly)

- **§1 dollar-invariant holds.** The fold touches only error mapping, comments, tests, and docs — no computation. Golden-packet byte-reproducibility, oracle smoke, and examples goldens all passed in my independent `make check` run.
- **Fail-closed holds.** `store_io_with_path` enriches only `StoreError::Io` at all four of its call sites; `mkdir_out` passes any non-Io `StoreError` through; `backup_key` never masks `WrongPassphrase`/`Crypto`; the unlock curated messages are untouched and unshadowable.
- **No regression**: independently confirmed green (2038/2038 + clippy); test delta exactly matches the five claimed pins; the fold commit's file footprint matches its message (no unclaimed changes).
- **Untested-guard honesty**: the I1 arm and all four I2 sites are genuinely mutation-proven as claimed. The one remaining untested/ineffective guard is the `write_csv_exports` wrap — the Important above.

## VERDICT

**NOT GREEN — 0 Critical / 1 Important.** Must still be folded:

1. **I1(r2)**: make the `write_csv_exports` wrap actually catch its advertised `Store(Io)` class (or remove it), pin with the proven `out_dir/lots.csv`-as-directory KAT, and correct the `admin.rs:117-119` comment to match the real enrichment split. Re-review after the fold.
