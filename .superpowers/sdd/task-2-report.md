# Task 2 Report — `cli_config` side-table (`CliConfig` ⇄ `ProjectionConfig`, TP8 (c)/(b))

**Status:** DONE
**Branch:** feat/btctax-cli

---

## Files Touched

- `crates/btctax-cli/src/config.rs` — full rewrite per brief (was a partial skeleton missing `set_fee_treatment`, using wrong field name, and wrong DB key)
- `crates/btctax-cli/src/lib.rs` — added `CliError::BadConfigValue { key, value }` variant (M1)

---

## Test Command and Output

```
cargo test -p btctax-cli
```

```
running 7 tests
test config::tests::default_is_treatment_c_user_mandated ... ok
test config::tests::to_projection_carries_treatment_and_fifo ... ok
test config::tests::bad_stored_value_is_an_error_not_silent_default ... ok
test config::tests::read_config_on_table_less_vault_returns_default ... ok
test config::tests::set_then_read_b_opt_in_round_trips ... ok
test session::tests::create_then_open_round_trips_over_a_temp_vault ... ok
test session::tests::wrong_passphrase_is_surfaced_not_a_panic ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.32s
```

`cargo clippy -p btctax-cli --all-targets -- -D warnings` — clean (0 warnings)
`cargo fmt --check -p btctax-cli` — clean

---

## M1 Handling (unrecognized stored config value)

Added `CliError::BadConfigValue { key: String, value: String }` to `lib.rs`. In `read_config`, the `match v.as_str()` arm for `fee_treatment` returns `Err(CliError::BadConfigValue { key: "fee_treatment".into(), value: v })` for any tag that is neither `"c"` nor `"b"`. Test `bad_stored_value_is_an_error_not_silent_default` inserts `'z'` directly and asserts the correct variant is returned.

---

## M2 Handling (vault created without the table)

`init_config_table` uses `CREATE TABLE IF NOT EXISTS` (idempotent DDL). Additionally, `read_config` now calls `init_config_table(conn)?` as its first statement (ensure-table-then-read pattern). Test `read_config_on_table_less_vault_returns_default` opens a bare in-memory connection without calling `init_config_table` first and asserts that `read_config` returns the (c) default cleanly.

---

## API Delta vs. Prior Skeleton

The pre-existing `config.rs` skeleton had the following diffs from the brief spec that were corrected:

| Item | Old (skeleton) | New (brief-correct) |
|------|---------------|---------------------|
| `CliConfig` field name | `self_transfer_fee` | `fee_treatment` |
| `CliConfig` derives | `Debug, Clone` | `Debug, Clone, Copy, PartialEq, Eq` |
| `to_projection` receiver | `&self` | `self` (consuming; `Copy` makes callers unchanged) |
| DB key | `'self_transfer_fee'` | `'fee_treatment'` |
| `set_fee_treatment` | missing | added |
| Silent fallback on unknown value | yes | no (M1 error) |
| Ensure-table in `read_config` | no | yes (M2) |

`session.rs` required no changes — it only calls `to_projection()` on the result of `?` and doesn't access `CliConfig` fields directly.

---

## Concerns

None.
