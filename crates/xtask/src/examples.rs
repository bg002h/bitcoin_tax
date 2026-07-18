//! `xtask examples` — the CLI verbatim-I/O examples generator (SPEC §7, Artifact 1).
//!
//! Runs the freshly-built `btctax` binary against synthetic vaults under a pinned, deterministic
//! environment (SPEC §3.3) and captures `$ cmd` + verbatim stdout (+ exit code, + labelled stderr where
//! relevant) into a single whole-file Markdown golden. The golden is a pure function of
//! `(repo tree, binary, synthetic inputs)`; a `regen == committed` test (examples_golden.rs) gates it.
//!
//! The generator NEVER runs a stale binary: `built_btctax()` compiles `btctax` via a nested `cargo build`
//! (I3 — `CARGO_BIN_EXE_btctax` is not set for xtask), so freshness holds by construction.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// SPEC §3.3 pinned environment (minus `BTCTAX_NOW`/`HOME`, which are set per invocation).
const PINNED_ENV: &[(&str, &str)] = &[("TZ", "UTC"), ("LC_ALL", "C"), ("LANG", "C")];
/// The synthetic passphrase every captured journey uses (front-matter discloses the interactive prompt
/// a real user would see instead).
const PASSPHRASE: &str = "pw";

/// The workspace root (two levels up from `crates/xtask`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above crates/xtask")
        .to_path_buf()
}

/// Compile `btctax` and return the path to the fresh debug binary. Honors `CARGO_TARGET_DIR`.
pub fn built_btctax() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let ws = workspace_root();
    let status = Command::new(&cargo)
        .current_dir(&ws)
        .args(["build", "-p", "btctax-cli", "--bin", "btctax"])
        .status()
        .expect("spawn `cargo build -p btctax-cli`");
    assert!(status.success(), "cargo build -p btctax-cli --bin btctax failed");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| ws.join("target"));
    target.join("debug").join("btctax")
}

/// The `btctax-cli` crate version, read from its `Cargo.toml` (the binary has no `--version`; SPEC §7).
/// A release bump reds `regen == committed` until the golden is regenerated.
fn btctax_cli_version() -> String {
    let toml = std::fs::read_to_string(workspace_root().join("crates/btctax-cli/Cargo.toml"))
        .expect("read btctax-cli/Cargo.toml");
    for line in toml.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("version") {
            if let Some(v) = rest.split('"').nth(1) {
                return v.to_string();
            }
        }
    }
    panic!("no version in btctax-cli/Cargo.toml");
}

/// One captured command in a journey. `display` is what the reader sees after `$ `; `args` is what the
/// real binary runs (display is bare `btctax …`, execution is the resolved binary — display-faithful,
/// execution-pinned). `now` pins `BTCTAX_NOW`; `show_stderr` adds a labelled `stderr:` block.
struct Cmd<'a> {
    args: &'a [&'a str],
    now: Option<&'a str>,
    show_stderr: bool,
}

/// Run one command in `cwd` under the pinned environment; return `(stdout, stderr, exit_code)`.
fn capture(bin: &Path, cwd: &Path, cmd: &Cmd) -> (String, String, i32) {
    let mut c = Command::new(bin);
    c.current_dir(cwd)
        .env("BTCTAX_PASSPHRASE", PASSPHRASE)
        .env("BTCTAX_PRICE_CACHE", cwd.join("no-such-price-cache.csv"))
        .env("HOME", cwd);
    for (k, v) in PINNED_ENV {
        c.env(k, v);
    }
    if let Some(now) = cmd.now {
        c.env("BTCTAX_NOW", now);
    }
    c.args(cmd.args);
    let out = c.output().expect("spawn btctax");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// Shell-quote an argument for display so the captured command is copy-pasteable (event refs carry `|`,
/// `#`, `:`; donee/appraiser names carry spaces). Bare only for a conservative safe set.
fn shell_quote(arg: &str) -> String {
    let safe = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '=' | ',' | '-' | '@' | '+'));
    if safe {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('"', "\\\""))
    }
}

/// Emit one `$ btctax …` block (command + verbatim stdout + exit-code marker, + labelled stderr when
/// `show_stderr`) into `md`.
fn emit(md: &mut String, bin: &Path, cwd: &Path, cmd: &Cmd) {
    let shown = format!(
        "btctax {}",
        cmd.args.iter().map(|a| shell_quote(a)).collect::<Vec<_>>().join(" ")
    );
    let (stdout, stderr, code) = capture(bin, cwd, cmd);
    md.push_str("```console\n");
    let _ = writeln!(md, "$ {shown}");
    md.push_str(&stdout);
    if !stdout.ends_with('\n') && !stdout.is_empty() {
        md.push('\n');
    }
    if code != 0 {
        let _ = writeln!(md, "[exit {code}]");
    }
    md.push_str("```\n");
    if cmd.show_stderr && !stderr.is_empty() {
        md.push_str("\nstderr:\n```console\n");
        md.push_str(&stderr);
        if !stderr.ends_with('\n') {
            md.push('\n');
        }
        md.push_str("```\n");
    }
}

/// The front matter: the pinned-env convention + the honest passphrase sentence + the version pin.
fn front_matter(md: &mut String) {
    let _ = writeln!(md, "---");
    let _ = writeln!(md, "title: btctax — worked examples");
    let _ = writeln!(md, "btctax-version: {}", btctax_cli_version());
    let _ = writeln!(md, "---");
    md.push('\n');
    md.push_str(
        "<!-- GENERATED by `cargo run -p xtask -- examples`; do NOT edit by hand. A `regen == committed`\n\
         test gates this file; CI re-diffs it. Every block below is the real `btctax` binary run against\n\
         synthetic vaults. -->\n\n\
         All examples run under a pinned, deterministic environment: `BTCTAX_PASSPHRASE=pw`, `TZ=UTC`,\n\
         `LC_ALL=C`, a nonexistent `BTCTAX_PRICE_CACHE`, a scrubbed `HOME`, and (where a decision is\n\
         recorded) a fixed `BTCTAX_NOW`. A real user is prompted for the passphrase interactively rather\n\
         than passing `BTCTAX_PASSPHRASE`.\n\n",
    );
}

/// Write a committed synthetic corpus into `cwd/name` (so a journey's `import` sees a real file).
fn write_corpus(cwd: &Path, name: &str, content: &str) {
    std::fs::write(cwd.join(name), content).expect("write corpus");
}

/// A no-`now` non-stderr command (the common case).
fn plain<'a>(args: &'a [&'a str]) -> Cmd<'a> {
    Cmd { args, now: None, show_stderr: false }
}

// ── Synthetic corpora (embedded as consts with explicit CRLF — committed .csv files are force-LF'd by
//    .gitattributes and would break the Coinbase parser, so we follow the fixtures.rs pattern) ─────
const J1_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.02000000,USD,67500.00,1350.00,1340.00,10.00,,,\r\n";

/// J2 corpus: an LT lot (2023) + an ST lot (2025) + a 2025 Send of 2 BTC donated to charity.
const J2_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy-lt,2023-06-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,5000.00,5000.00,5000.00,0.00,,,\r\n\
cb-buy-st,2025-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,2000.00,2000.00,2000.00,0.00,,,\r\n\
cb-donate,2025-09-01 12:00:00 UTC,Send,BTC,2.00000000,USD,108996.17,,,,,,bc1qcharity\r\n";

/// J3 corpus: a Buy + a Receive (an inbound transfer with unknown basis → a hard blocker until classified).
const J3_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-02-01 12:00:00 UTC,Buy,BTC,0.50000000,USD,95000.00,47500.00,47550.00,50.00,,,\r\n\
cb-recv,2025-08-01 12:00:00 UTC,Receive,BTC,0.20000000,USD,110000.00,,,,,,\r\n";

/// J5 corpus: an LT lot + a higher-basis ST lot + a 2025 sell — a genuine changed-selection scenario
/// (HIFO ≠ FIFO) so the optimizer has a tax-saving pick to propose.
const J5_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
opt-buy-lt,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
opt-buy-st,2025-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,80000.00,80000.00,80000.00,0.00,,,\r\n\
opt-sell,2025-06-01 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n";

/// Generate the whole-file golden by running `bin` across every journey. Pure function of
/// `(repo tree, binary, synthetic inputs)`.
pub fn generate(bin: &Path) -> String {
    let mut md = String::new();
    front_matter(&mut md);

    md.push_str("## btctax at a glance\n\nThe top-level command surface:\n\n");
    let dir = tempfile::tempdir().expect("tempdir");
    emit(&mut md, bin, dir.path(), &plain(&["--help"]));

    journey_j1(&mut md, bin);
    journey_j2(&mut md, bin);
    journey_j3(&mut md, bin);
    journey_j5(&mut md, bin);

    md
}

/// J5 — lot-selection optimization + attestation, and a what-if planning query. Showcases the
/// `made ≤ sale → Contemporaneous` lever that the `BTCTAX_NOW` seam pins.
fn journey_j5(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J5 — optimizing lot selection (and the contemporaneity clock)\n\n\
         Dana holds two lots (a cheap long-term one and an expensive short-term one) and has a standing\n\
         FIFO election. After a sale, `optimize` finds the lot identification that minimizes tax — here\n\
         picking the short-term lot to realize a loss. Set the profile + the FIFO baseline first:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J5_CSV);
    let now = "2025-01-01T00:00:00Z"; // before the 2025-06-01 sale → a contemporaneous identification
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]));
    emit(
        md, bin, cwd,
        &plain(&[
            "--vault", "v.pgp", "tax-profile", "--year", "2025", "--filing-status", "single",
            "--ordinary-taxable-income", "100000", "--magi-excluding-crypto", "100000",
            "--qualified-dividends", "0",
        ]),
    );
    emit(
        md, bin, cwd,
        &Cmd { args: &["--vault", "v.pgp", "config", "--set-forward-method", "fifo", "--effective-from", "2025-01-01"], now: Some(now), show_stderr: false },
    );
    md.push_str("\n`optimize run` is read-only — it proposes, files nothing:\n\n");
    emit(md, bin, cwd, &Cmd { args: &["--vault", "v.pgp", "optimize", "run", "--tax-year", "2025"], now: Some(now), show_stderr: false });
    md.push_str(
        "\nAccept it. Because the identification is made *before* the sale date, it is persisted as\n\
         **Contemporaneous** (an identification made after the sale would instead require an\n\
         attestation — this is exactly what the pinned clock governs, Treas. Reg. §1.1012-1(j)):\n\n",
    );
    emit(md, bin, cwd, &Cmd { args: &["--vault", "v.pgp", "optimize", "accept", "--tax-year", "2025"], now: Some(now), show_stderr: false });
    md.push_str("\nAnd a forward-looking what-if — the marginal tax of a hypothetical future sale:\n\n");
    emit(
        md, bin, cwd,
        &plain(&["--vault", "v.pgp", "what-if", "sell", "--sell", "0.5", "--wallet", "exchange:coinbase:default", "--at", "2025-07-01"]),
    );
}

/// J3 — an inbound self-transfer: an unknown-basis deposit is a hard blocker until you classify it.
fn journey_j3(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J3 — reconciling a self-transfer (unknown-basis inbound)\n\n\
         Carol moves 0.2 BTC into her exchange from her own cold storage. btctax will not guess its\n\
         basis — an unclassified inbound transfer is a **hard blocker** that gates the tax computation:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J3_CSV);
    let inbound = "import|coinbase|in|cb-recv";
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"])); // exits 1: the hard blocker
    md.push_str(
        "\nClassify it as your own coins returning — non-taxable, carrying the original basis and\n\
         acquisition date (for the holding period). The blocker clears and the ledger balances:\n\n",
    );
    emit(
        md, bin, cwd,
        &Cmd {
            args: &["--vault", "v.pgp", "reconcile", "classify-inbound-self-transfer", inbound, "--basis", "19000.00", "--acquired", "2024-11-01"],
            now: Some("2025-08-02T00:00:00Z"), // decision made-date pinned (banner → stderr, not captured)
            show_stderr: false,
        },
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
}

/// J2 — a §170(e) charitable donation of appreciated BTC (Form 8283).
fn journey_j2(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J2 — donating appreciated Bitcoin (§170(e) / Form 8283)\n\n\
         Bob donates 2 BTC (a long-term lot + a short-term lot) to a public charity. Import, then\n\
         reclassify the outbound transfer as a donation. `--amount` is the **USD fair market value** of\n\
         the gift (here 2 × the $108,996.17 close = $217,992.34):\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J2_CSV);
    let donation = "import|coinbase|out|cb-donate";
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]));
    emit(
        md, bin, cwd,
        &plain(&["--vault", "v.pgp", "reconcile", "reclassify-outflow", donation, "--as-kind", "donate", "--amount", "217992.34", "--donee", "Habitat for Humanity"]),
    );
    md.push_str("\nRecord the Form 8283 Section-B appraiser + donee details:\n\n");
    emit(
        md, bin, cwd,
        &plain(&[
            "--vault", "v.pgp", "reconcile", "set-donation-details", donation,
            "--donee-name", "Habitat for Humanity", "--donee-ein", "53-0242739",
            "--appraiser-name", "Jane Appraiser", "--appraiser-tin", "12-3456789",
            "--appraisal-date", "2025-09-15",
        ]),
    );
    md.push_str(
        "\n`verify` recomputes the §170(e) deduction (long-term lot → FMV; short-term lot →\n\
         min(FMV, basis)) and flags the qualified-appraisal requirement for a >$5,000 crypto gift:\n\n",
    );
    emit(md, bin, cwd, &Cmd { args: &["--vault", "v.pgp", "verify"], now: None, show_stderr: false });
    md.push_str("\nFill Form 8283:\n\n");
    emit(
        md, bin, cwd,
        &Cmd {
            args: &["--vault", "v.pgp", "export-irs-pdf", "--out", "irs", "--tax-year", "2025", "--forms", "form8283"],
            now: None,
            show_stderr: true,
        },
    );
}

/// J1 — single-buyer happy path: init → import → verify → set a tax profile → report → export.
fn journey_j1(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J1 — a single buyer, start to finish\n\n\
         Alice buys 0.1 BTC, sells 0.02, and wants her 2025 numbers. Create an encrypted vault (a key\n\
         backup is mandatory), import the exchange CSV, and check the ledger balances:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J1_CSV);
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
    md.push_str(
        "\nThe year's tax is *not computable* until a tax profile is set (btctax refuses to guess your\n\
         bracket). Set one, then the report computes:\n\n",
    );
    emit(
        md, bin, cwd,
        &plain(&[
            "--vault", "v.pgp", "tax-profile", "--year", "2025", "--filing-status", "single",
            "--ordinary-taxable-income", "100000", "--magi-excluding-crypto", "100000",
            "--qualified-dividends", "0",
        ]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "report", "--tax-year", "2025"]));
    md.push_str("\nExport the reconciled snapshot (CSVs + a decrypted SQLite) and fill the IRS forms:\n\n");
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "export-snapshot", "--out", "snapshot", "--tax-year", "2025"]));
    emit(
        md, bin, cwd,
        &Cmd {
            args: &["--vault", "v.pgp", "export-irs-pdf", "--out", "irs", "--tax-year", "2025", "--forms", "f8949,schedule-d"],
            now: None,
            show_stderr: true, // the NOT-AUTHORISED notice + 1099-DA caveat are on stderr and matter
        },
    );
}

/// Regenerate the committed golden to stdout (`cargo run -p xtask -- examples`).
pub fn run() {
    let bin = built_btctax();
    print!("{}", generate(&bin));
}

#[cfg(test)]
#[cfg(unix)] // journey stdout can embed joined paths; byte-exact goldens are gated on unix (I4)
mod tests {
    use super::*;

    #[test]
    fn generate_is_deterministic_and_captures_help() {
        let bin = built_btctax();
        let a = generate(&bin);
        let b = generate(&bin);
        assert_eq!(a, b, "generate() must be byte-deterministic");
        assert!(a.contains("$ btctax --help"), "must show the verbatim command");
        assert!(a.contains("Usage: btctax"), "must capture the real help output");
        assert!(a.contains(&format!("btctax-version: {}", btctax_cli_version())), "front-matter version");
    }
}
