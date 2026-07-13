# Fable review — full-return P4.9 (carryover write-back), commit `d57990d`

*(Persisted verbatim before folding, per STANDARD_WORKFLOW §2.)*

## VERDICT: NOT GREEN — 0 Critical / 3 Important

The R3-M6 precedence logic itself is **correct** (I could not break it). The blockers are all in what the write *does to year Y+1* and one red CI gate.

---

## Important

### I1 — The write-back bricks year Y+1: the row it creates shadows the stored `TaxProfile` in the §4.12 ladder, and Y+1 can never have full-return tables in v1
`crates/btctax-cli/src/cmd/tax.rs:355` — `let next = ...get(s.conn(), year + 1)?.unwrap_or_default();` then `set(...)` at :357.

`unwrap_or_default()` **manufactures a `ReturnInputs` row for Y+1 that the user never imported.** `resolve_core` (`crates/btctax-cli/src/resolve.rs:85-95`) puts `ReturnInputs` at the top of the precedence ladder; if the year has no `FullReturnParams` it fails **closed** → `Uncomputable`. v1 bundles full-return tables for **TY2024 only** (`btctax-adapters/src/tax_tables.rs:101`, and its own KAT at :757 asserts `full_return_for(2025).is_none()`). Since Y is always 2024 in v1, **Y+1 is always 2025 — a year that can never compute.** This is not an edge case; it is the only case.

Reproduced end-to-end with the built binary:

```
# before: 2025 planned via a stored tax-profile (the documented v1 escape hatch)
$ btctax report --tax-year 2025                     → computes fine (exit 0)
$ btctax report --tax-year 2024 --write-carryover   → "carryover written back to 2025"
# after:
$ btctax report --tax-year 2025
error: usage: tax year 2025 has full-return inputs, but full-return computation is not
supported for 2025 in this version (v1 supports TY2024); run `income clear --year 2025`
to remove them and use a raw `tax-profile`                                      exit=2
$ btctax tax-profile --year 2025 --filing-status single ...
error: usage: tax year 2025 already has full-return inputs (`income import`); a raw
tax-profile would be ignored ...                                                exit=2
```

Three separate harms: (a) `report --tax-year 2025` / `optimize` / `what-if` / the TUI 2025 tab — all previously working from the stored profile — now hard-error; (b) the D-4 guard (`cmd/tax.rs:28`) refuses `tax-profile --year 2025` citing an `income import` **the user never ran**; (c) the recovery the error message itself prescribes, `income clear --year 2025`, **deletes the carryover the feature just wrote** — the user cannot both keep the carryover and compute 2025. Pseudo-reconcile mode is hit the same way (step 1 of the ladder precedes the placeholder arm).

Fix (also discharges I2): require the Y+1 row to **already exist**. SPEC §4 says the carryover is written back "as year (Y+1)'s `*_carryover_in` **on that row**" — so `get(year + 1)?` returning `None` should refuse with "import your Y+1 inputs first, then re-run `--write-carryover`", not fabricate a default row. (Teaching the resolver to see through a carryover-only row would weaken the fail-closed arm — not recommended.)

### I2 — `income import` for Y+1 silently discards a written-back `Computed` carryover; nothing detects it
`crates/btctax-cli/src/cmd/tax.rs:49` (`import_return_inputs` → `return_inputs::set`, a whole-blob upsert).

The R3-M6 precedence model is enforced in exactly **one** direction. The natural user order — finish Y, write back, then later prepare Y+1 — destroys the persisted value. Reproduced:

```
2025 carryover-in after write-back:  [{class: cash60, amount: "10000.00", origin_year: 2024,
                                      provenance: "computed"}]
$ btctax income import --year 2025 --file 2025.toml     → "Imported full-return inputs for 2025."
2025 carryover-in after the import:  []          ← gone. No warning.
```

No advisory catches it: `carryforward_consistency` (`btctax-core/src/tax/compute.rs:436`) only compares the **frozen capital-loss `Carryforward`** (short/long) — nothing watches charitable or QBI. Failure directions: charitable → a lost deduction (overpay, conservative); **QBI REIT/PTP → the loss carryforward that *reduces* the QBI deduction vanishes → QBI deduction overstated → tax understated.** The tool tells the user "carryover written back to 2025", so reliance on it is reasonable.

Fix: I1's fix makes import-then-write-back the only possible order, which eliminates this. Otherwise, warn in `import_return_inputs` when the incoming blob would drop a `Computed` carryover-in.

### I3 — Red suite: the CI clippy gate fails on P4.9's own new tests
`crates/btctax-core/src/tax/return_1040.rs:2861, 2883, 2892`.

Running CI's exact command (`.github/workflows/ci.yml:43`) on **stable**:

```
$ cargo +stable clippy --workspace --all-targets --locked -- -D warnings
error: could not compile `btctax-core` (lib test) due to 3 previous errors
```

All three are `clippy::field_reassign_with_default` (`let mut prior = ReturnInputs::default(); prior.charitable_carryover_in = vec![...]`), introduced by the new `writeback_overwrites_computed_silently` / `writeback_refuses_user_without_force` KATs. Per the standing rule, a red lint gate is itself a blocking finding. Fix: struct-literal + `..Default::default()`.

---

## Minor

- **M1 (pre-existing, NOT P4.9)** — `cargo fmt --all -- --check` (`ci.yml:54`) is red on this branch, but the diffs span files P4.9 never touches (`tax/amt.rs`, `tax/qbi.rs`, `tax/method.rs`, `adapters/tax_tables.rs`, `conventions.rs`, `btctax-tui-edit/*`). Not attributable to this increment, but the branch cannot ship with a red CI — worth its own cleanup commit.
- **M2 — double passphrase prompt + double vault decrypt.** `main.rs:151` calls `passphrase(false)?` a *second* time for `write_back_carryover`, which opens a second `Session` and re-projects the whole ledger. With `BTCTAX_PASSPHRASE` unset, one command prompts the user twice.
- **M3 — the back-compat claim has no KAT.** No test deserializes a legacy blob lacking the `provenance` key. I verified the behavior manually (a TOML carryover with no `provenance` loads as `"user"` and the write-back correctly refuses it) — the behavior is right, the pin is missing.

---

## Focus areas that PASS

1. **R3-M6 precedence — correct.** `return_1040.rs:989-1026`: (a) a `Computed` or empty carryover-in is overwritten silently ✓; (b) a `User` one refuses without `--force`, and `--force` overwrites ✓; (c) **both** conflicts are checked in the `if !force` block (`:994`) *before* either field is written (`:1016`, `:1023`) — no half-applied write on a QBI conflict ✓; (d) both written fields are stamped `Computed` (`:1018`, `:1024`) ✓. I found no fail-open path and no wrongly-refused computed overwrite. CLI-level atomicity also holds: `Session` is an in-memory DB, and the `Err` returns before `return_inputs::set` + `s.save()`, so a refusal persists nothing.
2. **Provenance model + serde back-compat — sound.** `#[serde(default)] User` is the right default (verified: a no-provenance item loads as `user` and *is* protected). `apply_170b`'s provenance is genuinely inert — `charitable_carryover_out` has exactly one non-test consumer (`apply_carryover_writeback`), which restamps everything `Computed`. Preserving `item.provenance` on a surviving item is harmless and semantically right. Aging/vintage logic untouched; all charitable KATs pass.
3. **CLI wiring — fail-closed.** `write_back_carryover` requires `ReturnInputs` provenance + both refuse screens (via `resolve_and_screen`) + `screen_absolute` passing before it writes. `--write-carryover` without `--tax-year` errors (verified). Opt-in/default-read-only is a sound, non-fail-open refinement of the SPEC's "at report time". (The *write* is where I1/I2 bite, not the gating.)
4. **Capital-loss deferral — LEGITIMATE, not a gate.** SPEC §4 (lines 177-181) names only charitable + QBI in R3-M6, and `design/full-return/reviews/DESIGN-fable-audit-final.md:171` (M3) explicitly accepted the exclusion. Non-fail-open. `p4-9-capital-loss-writeback` is a correctly-scoped follow-up.
5. **Frozen invariant — intact.** `git diff 059ec2a..HEAD -- crates/btctax-core/src/tax/{types,compute,se}.rs` = **0 bytes** ✓. Tests all green: `btctax-core --lib` 230 passed; `btctax-cli --test tax_report` 25 passed; full workspace green. The no-`..` destructure updates in `return_refuse.rs:286,302` are correct. The only regression from the model change is the clippy one (I3).

**To close Phase 4:** fix I1 (the fix subsumes I2) and I3, then re-review.
