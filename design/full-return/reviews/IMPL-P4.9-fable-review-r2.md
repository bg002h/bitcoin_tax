# Fable re-review r2 — full-return P4.9 (carryover write-back), fold commit `6eeda51`

*(Persisted verbatim per STANDARD_WORKFLOW §2.)*

## VERDICT: GREEN 0C/0I — Phase 4 fully closes.

All three Importants and all three Minors from r1 are correctly folded; the fold introduced no new Critical or Important defect. Full validation surface: **81 suites / 1554 passed / 0 failed** (`cargo test --workspace --locked`, exit 0; core lib 231, cli `tax_report` 26), **clippy stable `-D warnings` = 0 errors**, **`cargo fmt --all -- --check` clean**, **frozen files byte-identical** (`git diff 059ec2a..HEAD -- tax/{types,compute,se}.rs` = 0 bytes).

## Fold confirmation, finding by finding

**I1 — FIXED, and the fix is complete.** `crates/btctax-cli/src/cmd/tax.rs:402` now does `return_inputs::get(s.conn(), year + 1)?.ok_or_else(refuse)` — no `unwrap_or_default()`, no fabricated Y+1 row. Re-ran the full r1 binary reproduction against a fresh vault (2024 `income import`, 2025 planned via stored `tax-profile`):
- `report --tax-year 2024 --write-carryover` with no 2025 row → **refuses (exit 2)** with an actionable, honest message (names the exact `income import` command, and explains the shadowing tradeoff the r1 bug silently inflicted). No row created (`income show --year 2025` → none).
- After the refusal: `report --tax-year 2025` still computes from the stored profile (exit 0), and `tax-profile --year 2025` still accepts updates — the D-4 guard no longer fires on a row the user never made. **The brick is gone.**
- After `income import --year 2025`: write-back succeeds; the $10,000 cash60 carryover lands with `provenance: computed` (verified $50k AGI × 60% ceiling − $40k gift arithmetic). The subsequent 2025 fail-closed refusal is now the user's informed v1 tradeoff, and its prescribed recovery (`income clear --year 2025` → 2025 computes from the profile again) **works without losing anything the user didn't choose to lose** — verified end-to-end.
- No remaining fabrication path: the only production callers of `return_inputs::set` are `import_return_inputs` (explicit user action) and the write-back itself (now requires the row). No half-apply: every refusal precedes the in-memory `set` + `s.save()`; binary-verified that a refused write-back leaves the Y+1 row byte-identical.

**I2 — FIXED, conservatively.** `cmd/tax.rs:63-97`: a `Computed` carryover the incoming TOML does not supply is preserved (with a stderr note naming the replace paths); a TOML-supplied carryover wins as `User` (binary-verified: supplied $7,777 → `provenance: user`, then protected — write-back refuses without `--force`, `--force` overwrites). The preserve-on-empty direction is the *conservative* one for both fields (a kept QBI loss carryforward can only reduce the QBI deduction; a kept charitable carryover is exactly the value the tool computed and announced). The user can still fully clear: `income clear --year Y` deletes the row wholesale (verified), and a fresh import then preserves nothing. `ri.qbi = existing.qbi.clone()` is safe — `QbiInputs` is exactly the carryforward + provenance pair. KAT `import_preserves_a_computed_carryover` exercises the real CLI flow and passes.

**I3 — FIXED.** The two write-back KATs are struct literals now; `cargo clippy --workspace --all-targets --locked -- -D warnings` on stable exits 0.

**M1 — FIXED, semantics-preserving.** `cargo fmt --all -- --check` is clean. I mechanically verified the reformat: re-ran rustfmt on every touched file's pre-fold version and diffed — **18 of 25 files are formatting-only**; the 7 with real deltas contain only the intended fixes plus pure re-wraps. Frozen files untouched.

**M2 — FIXED** (the user-facing half). `main.rs:116` hoists one `passphrase(false)?`; both `report_tax_year` and `write_back_carryover` reuse it — one prompt per command.

**M3 — FIXED.** `legacy_carryover_blob_without_provenance_loads_as_user` pins that a provenance-less legacy blob loads as `User` *and* is protected from the write-back — the right pin, against JSON, which is the side-table's actual storage format.

## Recorded, non-gating

- **Minor (owning phase → P5):** `LIMITATIONS.md` (new in this fold) is the **Phase 5** deliverable landed early. Everything I spot-checked against current source is accurate (box-12 allowlist, §402(g)/kiddie/SALT/QBI/std-deduction/FTC figures, the refusal list, the write-carryover semantics), **but** the "Forms filled: … Schedules 1, 2, 3, A, B, C … 8959, 8960, 8995" line describes the **Phase 6 PDF fillers** — `btctax-forms` today fills only 1040/8949/8283/Sch D/Sch SE. True of the compute, ahead of the PDFs. The doc must get its own line-by-line pass at the P5 gate (it has *not* been certified by this review); the forms-filled wording must be reconciled there or in P6.
- **Nit:** `write_back_carryover` still opens a second `Session` (second decrypt + re-projection) after `report_tax_year` — the r1 M2 prompt pain is gone; the redundant decrypt remains. Perf only.
- **Nit (process):** the fold commit mixes in a P5 artifact and pre-existing fmt drift cleanup; folds are cleaner kept to the findings.
- `p4-9-capital-loss-writeback` remains a correctly-documented, non-fail-open, spec-complete deferral (SPEC R3-M6 names only charitable + QBI) with the frozen-type constraint and design sketch recorded in `design/full-return/FOLLOWUPS.md`.

**Phase 4 is CERTIFIED GREEN at `6eeda51`.**
