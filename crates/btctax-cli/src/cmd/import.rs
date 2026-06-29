//! `import <files…>` (FR1/FR2) — detect+parse via adapters, append the batch atomically into the
//! vault, and save. Idempotency + `ImportConflict` detection are core's job (`append_import_batch`);
//! the CLI surfaces the per-source FR2 counts (dropped/unclassified) and the append/dup/conflict tally.
use crate::{CliError, Session};
use btctax_adapters::{ingest_files_bundled, FileReport};
use btctax_core::persistence::{append_import_batch, ImportReport};
use btctax_store::Passphrase;
use std::path::{Path, PathBuf};

pub fn run(
    vault_path: &Path,
    pp: &Passphrase,
    files: &[PathBuf],
) -> Result<(Vec<FileReport>, ImportReport), CliError> {
    let batch = ingest_files_bundled(files)?; // adapters: detect→group→parse→normalize (FR2/FR3)
    let mut session = Session::open(vault_path, pp)?;
    let import = append_import_batch(session.conn(), &batch.events)?; // ATOMIC batch (FR1)
    session.save()?; // encrypted, atomic (NFR2/NFR3)
    Ok((batch.reports, import))
}
