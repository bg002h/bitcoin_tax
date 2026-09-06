//! ★ Design r2 §5 — **the publishing trap, as a gate.** `btctax-forms`'s `build.rs` binds every
//! file under `forms/<year>/` with `include_*!`; if the published tarball omitted one, `cargo build`
//! of the crate from crates.io would fail — and `cargo publish` itself would not say so
//! (`crate-publishing-state`: an escaping `include_str!` once shipped a broken tarball with exit 0).
//! So this asks cargo what the tarball WOULD contain and holds it to the glob on disk, plus
//! `build.rs` itself.
//!
//! Read-only: `cargo package --list` builds nothing and, with `--offline`, touches no network.

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf()
    }

    fn forms_files_on_disk(forms_crate: &Path) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let root = forms_crate.join("forms");
        for y in std::fs::read_dir(&root).unwrap().flatten() {
            if !y.path().is_dir() {
                continue;
            }
            for f in std::fs::read_dir(y.path()).unwrap().flatten() {
                let rel = f
                    .path()
                    .strip_prefix(forms_crate)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel);
            }
        }
        out
    }

    fn package_list(forms_crate: &Path) -> BTreeSet<String> {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let out = Command::new(cargo)
            .args([
                "package",
                "--list",
                "--allow-dirty",
                "--offline",
                "-p",
                "btctax-forms",
            ])
            .current_dir(forms_crate)
            .output()
            .expect("run cargo package --list");
        assert!(
            out.status.success(),
            "cargo package --list failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// ★ Every globbed file is in the tarball, and so is the script that binds them.
    #[test]
    fn every_bundled_form_file_and_the_build_script_are_in_the_published_tarball() {
        let forms_crate = workspace_root().join("crates/btctax-forms");
        let disk = forms_files_on_disk(&forms_crate);
        assert!(
            disk.len() >= 74,
            "walk found {} files under forms/",
            disk.len()
        );
        let listed = package_list(&forms_crate);
        let missing: Vec<&String> = disk.iter().filter(|f| !listed.contains(*f)).collect();
        assert!(
            missing.is_empty(),
            "files under forms/ that `cargo package` would NOT ship — a build.rs include_*! of any of \
             these fails on crates.io with the publish reporting success: {missing:?}"
        );
        assert!(
            listed.contains("build.rs"),
            "build.rs is not in the tarball; the published crate would have no bindings"
        );
    }

    /// B1 — the gate discriminates: a file on disk that the tarball does not list is reported.
    #[test]
    fn a_file_the_tarball_does_not_list_is_caught() {
        let forms_crate = workspace_root().join("crates/btctax-forms");
        let mut disk = forms_files_on_disk(&forms_crate);
        disk.insert("forms/2026/f9999.pdf".to_string()); // planted: on "disk", never packaged
        let listed = package_list(&forms_crate);
        let missing: Vec<&String> = disk.iter().filter(|f| !listed.contains(*f)).collect();
        assert_eq!(missing, vec![&"forms/2026/f9999.pdf".to_string()]);
    }
}
