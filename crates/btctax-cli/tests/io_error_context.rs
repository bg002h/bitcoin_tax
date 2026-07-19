//! UX-P4-8 — I/O failures at a user-named path (`--vault`, `--out`) must NAME the path and offer a
//! one-clause remedy hint, instead of the bare `io: No such file or directory (os error 2)` /
//! `io: File exists (os error 17)` the raw `io::Error` produces. Mirrors the adapters' path-bearing
//! `AdapterError::Io { path, source }`.
mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_store::Passphrase;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// A missing (or wrong) `--vault` names the path the user gave AND hints how to recover — check the
/// `--vault` flag, or run `btctax init` to create one — not a bare pathless `io` error.
#[test]
fn open_missing_vault_names_path_and_hint() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("does_not_exist.pgp");
    let err = Session::open(&vault, &pp()).expect_err("a missing vault must error");
    let msg = err.to_string();
    assert!(
        msg.contains(&vault.display().to_string()),
        "names the vault path: {msg}"
    );
    assert!(msg.contains("--vault"), "hints the --vault flag: {msg}");
    assert!(msg.contains("init"), "hints `btctax init`: {msg}");
    assert!(
        !msg.contains("No such file or directory (os error 2)")
            || msg.contains(&vault.display().to_string()),
        "the raw errno may remain as the source, but the path must be present: {msg}"
    );
}

/// An `--out` that collides with an existing FILE (so the export directory cannot be created) names
/// the offending out path, not a bare `io: File exists`.
#[test]
fn export_out_collision_names_path() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // The chosen --out already exists as a plain file → `mkdir_owner_only` cannot create the dir.
    let out = dir.path().join("collide");
    std::fs::write(&out, b"i am a file, not a directory").unwrap();

    let err = cmd::admin::export_snapshot(&vault, &pp(), &out, None, None)
        .expect_err("an --out that collides with a file must error");
    let msg = err.to_string();
    assert!(
        msg.contains(&out.display().to_string()),
        "names the --out path: {msg}"
    );
}
