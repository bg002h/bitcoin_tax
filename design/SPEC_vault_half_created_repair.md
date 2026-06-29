# SPEC + PLAN — `vault-half-created-autorepair` (Slug 4)

**Source baseline:** `origin/main` @ `71dc513` (cycle-prep recon `428f457`, CLEAN — all citations accurate).
**Goal:** Turn the confusing "interrupted-init" dead-end into a clear, recoverable state — **detect + clear error + explicit `init --repair` opt-in**. Non-destructive by default; the orphan key is cleared ONLY on explicit consent, and NEVER when a real or recoverable vault is present.
**SemVer:** additive `--repair` flag + additive `StoreError` variant ⇒ **PATCH** (pre-1.0). No breaking change. GUI/manual locksteps: N/A.

## Problem (verified against current source)

A crash between the `vault.key` write (`vault.rs:56`) and the first `vault.pgp` rename (`save()` → `atomic_write`, `atomic.rs:24`) leaves the **half-created signature**:
`vault.key` present · `vault.pgp` absent · `vault.pgp.bak` absent.
(The first `save()`'s `atomic_write` only copies `target → .bak` `if target.exists()` — on the first create the target never existed, so there is **no** `.bak` to recover from.)

Today: `create` → `AlreadyExists` (`vault.rs:36` `kp.exists()`), `open` → `Io(NotFound)` (`vault.rs:90` reads an absent `vault.pgp`). The user is stuck unless they manually delete `vault.key`.

## Design

### Safety invariants (load-bearing)
- **Never delete `vault.key` when `vault.pgp` OR `vault.pgp.bak` exists** — those mean a healthy or `recover_target`-recoverable vault; the key is required to decrypt it. `--repair` MUST refuse (`AlreadyExists`) in that case.
- The ONLY repair-eligible state is the exact signature: `kp.exists() && !vault.exists() && !bak_of(vault).exists()`.
- Detection is read-only; the only mutation (orphan-key removal) is gated behind explicit `--repair`.

### Store — `crates/btctax-store/src/lib.rs`
Add one `StoreError` variant (after `AlreadyExists`):
```rust
#[error("vault initialization was interrupted: key '{0}' exists but the encrypted store was never written — \
rerun `init --repair` to clear it and start fresh, or delete that .key file manually")]
HalfCreatedVault(std::path::PathBuf),
```

### Store — `crates/btctax-store/src/vault.rs`
Refactor `create` into a private inner that takes a `repair` flag; expose `create` and `repair`:
```rust
pub fn create(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
    Self::create_inner(vault, pp, false)
}
/// Like `create`, but first clears a known **orphan key** from an interrupted init
/// (the half-created signature). NEVER clears a key when a real/recoverable vault is present.
pub fn repair(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
    Self::create_inner(vault, pp, true)
}
fn create_inner(vault: &Path, pp: &Passphrase, repair: bool) -> Result<Vault, StoreError> {
    // … existing .key-extension guard + mkdir_owner_only(parent) …
    let lock = VaultLock::acquire(vault)?;       // lock FIRST (unchanged)
    let kp = paths::suffixed_key(vault);
    // Refuse to clobber a real OR recoverable vault — even under --repair.
    if vault.exists() || paths::bak_of(vault).exists() {
        return Err(StoreError::AlreadyExists);
    }
    if kp.exists() {
        if repair {
            // half-created: clear ONLY the orphan key + any stray tmp sidecars, then build fresh.
            let _ = std::fs::remove_file(&kp);
            let _ = std::fs::remove_file(paths::tmp_of(&kp));
            let _ = std::fs::remove_file(paths::tmp_of(vault));
        } else {
            return Err(StoreError::HalfCreatedVault(kp));
        }
    }
    // … unchanged: cleanup closure, build cert, atomic_write(&kp), save() …
}
```
And in `open`, AFTER the existing `recover_target`/`reap_tmp` loop (so a `.bak`-recoverable vault is restored first and proceeds normally), BEFORE reading the cert:
```rust
if !vault.exists() && kp.exists() {
    return Err(StoreError::HalfCreatedVault(kp));   // clear error instead of Io(NotFound)
}
```

### CLI — `crates/btctax-cli/src/session.rs`
Factor the post-`Vault` DDL/config/save out of `create`, then add `repair`:
```rust
pub fn create(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
    Self::from_fresh_vault(Vault::create(vault_path, pp)?)
}
pub fn repair(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
    Self::from_fresh_vault(Vault::repair(vault_path, pp)?)
}
fn from_fresh_vault(mut vault: Vault) -> Result<Session, CliError> {
    // … existing init_schema + cli_config init + re-save body, verbatim …
}
```

### CLI — `crates/btctax-cli/src/cmd/init.rs`
**[R0-I1 fold]** KEEP the existing 3-arg `run(...)` signature so the ~24 existing callers (the call at `cmd/init.rs:25` + 23 test sites across `tests/{end_to_end,verify_report,reconcile,export,init_import,fr9_exit_code}.rs`) compile UNCHANGED; add a 4-arg `run_with_repair` that `run` delegates to, and have `main.rs` call the 4-arg form:
```rust
pub fn run(vault_path: &Path, pp: &Passphrase, key_backup_path: &Path) -> Result<(), CliError> {
    run_with_repair(vault_path, pp, key_backup_path, false)
}
pub fn run_with_repair(vault_path: &Path, pp: &Passphrase, key_backup_path: &Path, repair: bool)
    -> Result<(), CliError> {
    let session = if repair { Session::repair(vault_path, pp)? } else { Session::create(vault_path, pp)? };
    session.vault().backup_key(key_backup_path)?;
    Ok(())
}
```

### CLI — `crates/btctax-cli/src/main.rs`
Add `--repair` to the `Init` command and thread it:
```rust
Init {
    #[arg(long)] key_backup: PathBuf,
    /// Clear an interrupted/half-created init (orphan `vault.key`, no encrypted store) and start fresh.
    #[arg(long, default_value_t = false)] repair: bool,
},
// dispatch:
Command::Init { key_backup, repair } => {
    cmd::init::run_with_repair(vault, &passphrase(true)?, &key_backup, repair)?;
    println!("{} vault {} (key backed up to {})",
        if repair { "Repaired + initialized" } else { "Initialized" },
        vault.display(), key_backup.display());
}
```

## Plan (TDD, one reviewable change)

### Task A — store: error variant + `create`/`repair`/`open` (TDD)
- **A1 (test, store `vault.rs` tests):**
  - `create_on_half_created_returns_half_created_error`: write an orphan `vault.key` (e.g. `fs::write(suffixed_key, b"x")`) with no `vault.pgp`/`.bak`; `Vault::create` → `Err(HalfCreatedVault(_))`.
  - `open_on_half_created_returns_half_created_error`: same orphan; `Vault::open` → `Err(HalfCreatedVault(_))` (NOT `Io`).
  - `repair_clears_orphan_key_and_creates_fresh`: orphan key present; `Vault::repair(pp)` → `Ok`; then `Vault::open(pp)` round-trips (write/read a row).
  - `repair_refuses_to_clobber_healthy_vault`: `Vault::create` a real vault; `Vault::repair` → `Err(AlreadyExists)`; assert `vault.key` STILL exists and `Vault::open` still works.
  - `repair_refuses_when_bak_present`: real vault, then `fs::rename(vault.pgp, bak_of(vault))` (pgp absent, bak present, key present); `Vault::repair` → `Err(AlreadyExists)`; key untouched; `Vault::open` recovers from `.bak` and works.
  - **[R0-M1] `repair_on_clean_path_behaves_as_create`:** empty dir (no key/pgp/bak); `Vault::repair(pp)` → `Ok`; `Vault::open` round-trips (repair on a clean slate == plain create).
  - **[R0-M2] `repair_clears_orphan_tmp_sidecars`:** orphan `vault.key` + an orphan `tmp_of(vault)` (and/or `tmp_of(kp)`) present, no pgp/bak; `Vault::repair(pp)` → `Ok`; assert the orphan `.tmp` files are gone and `Vault::open` round-trips.
- **A2 (impl):** add `HalfCreatedVault` to `StoreError`; refactor `create`→`create_inner(repair)`; add `repair`; add the `open` half-created guard. Run the new tests + the existing store suite green.

### Task B — CLI: `Session::repair` + `init --repair` (TDD)
- **B1 (test):**
  - cli `cmd/init` test `init_repair_recovers_a_half_created_vault`: `init` a temp vault, then delete `vault.pgp` (+ `vault.pgp.bak` if any) leaving `vault.key` → `run_with_repair(.., true)` succeeds; `Session::open` round-trips.
  - `init_without_repair_on_half_created_errors`: same half-created state → 3-arg `run(..)` (repair=false) → `Err(Store(HalfCreatedVault(_)))`.
  - The existing `init_refuses_to_clobber_an_existing_vault` + `init_creates_vault_key_and_forced_backup` stay UNCHANGED (the 3-arg `run` wrapper is preserved — [R0-I1]).
- **B2 (impl):** factor `Session::from_fresh_vault`; add `Session::repair`; add `run_with_repair` and KEEP the 3-arg `run` as a `..,false` wrapper (no churn to the ~24 existing callers — [R0-I1]); add the `--repair` flag + repair-aware message in `main.rs`. No binary-level test required (the lib tests cover the path; main wiring is trivial).

## Validation gate
`cargo test -p btctax-store -p btctax-cli`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --check`; `cargo build --release --bin btctax`. Then the full `cargo test --workspace`.

## R0 nits (folded / acknowledged)
- **N1:** adding `HalfCreatedVault` to the non-`#[non_exhaustive]` `StoreError` is technically a SemVer-breaking enum change, but immaterial here — no external/exhaustive consumer exists (the CLI wraps via `#[from]`; all in-repo matches are `matches!`/`#[from]`). Left as a plain variant add; `#[non_exhaustive]` not introduced (out of this slug's scope).
- **N2/N3:** the `.tmp` removal in the repair path is defensive only (`open_owner_only` truncates, not `O_EXCL`, so a stray `.tmp` is never load-bearing); the `VaultLock` acquired first is the TOCTOU protection. Note this in a code comment.

## Out of scope
- The totally-absent-vault `open` (no key, no pgp) keeps returning `Io(NotFound)` — that is "no vault here", a different condition.
- No interactive confirmation prompt: passing `--repair` IS the explicit consent, and `repair` provably never deletes a real/recoverable key (the `AlreadyExists` guard fires first).
