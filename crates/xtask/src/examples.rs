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

// The synthetic corpora now live in `btctax_cli::testonly` (one source of truth, shared with the TUI
// screen-walkthrough emit tests). These bytes are unchanged by the move — the golden gate proves it.
use btctax_cli::testonly::{
    J1_CSV, J2_CSV, J3_CSV, J4_CSV, J5_CSV, J6_COINBASE_CSV, J6_FULLRETURN_TOML, J6_RIVER_CSV,
    J7_CSV, J8_COINBASE_CSV, J8_RIVER_CSV, J9_CSV,
};

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
        // `--locked` matches the outer `cargo run --locked` the CI job uses (N-1, hygiene). `Stdio::null()`
        // on stdout is belt-and-suspenders (M-3): the generator's own stdout IS the golden under the CI
        // `> docs/examples/examples.md` redirect, and this nested build inherits that stdout — cargo writes
        // progress to stderr, but nulling stdout makes a corrupt golden structurally impossible.
        .args(["build", "--locked", "-p", "btctax-cli", "--bin", "btctax"])
        .stdout(std::process::Stdio::null())
        .status()
        .expect("spawn `cargo build -p btctax-cli`");
    assert!(
        status.success(),
        "cargo build -p btctax-cli --bin btctax failed"
    );
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
    // Scan ONLY the `[package]` table (the first table) and require `version =` (with the `=`), so a
    // re-ordered manifest can't match a dependency's `version = "…"` first (N-3).
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') && t != "[package]" {
            break; // left the [package] table
        }
        if let Some(rest) = t.strip_prefix("version") {
            if let Some(rest) = rest.trim_start().strip_prefix('=') {
                if let Some(v) = rest.split('"').nth(1) {
                    return v.to_string();
                }
            }
        }
    }
    panic!("no [package] version in btctax-cli/Cargo.toml");
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
        .env("HOME", cwd)
        // Clear any ambient BTCTAX_NOW so an unpinned step reads none (a dev who exports BTCTAX_NOW in
        // their shell — this project *teaches* the variable — would otherwise leak its stderr banner into
        // `show_stderr` blocks and false-RED the golden). Re-set below only for pinned steps (M-1).
        .env_remove("BTCTAX_NOW");
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
///
/// LIMITATION (N-2): the quoted form only escapes `"`; an argument containing `$`, `` ` ``, `\`, or `!`
/// would display as a command a shell re-interprets (non-copy-pasteable). No current journey argument
/// contains one — a future journey that introduces one must extend this (or the golden will mislead).
fn shell_quote(arg: &str) -> String {
    let safe = !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '=' | ',' | '-' | '@' | '+')
        });
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
        cmd.args
            .iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ")
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
         than passing `BTCTAX_PASSPHRASE`.\n\n\
         Each block shows the verbatim command after `$ ` and its real stdout. A non-zero exit is shown\n\
         as a trailing `[exit N]` line (present only when it is non-zero). A command's **stderr** is\n\
         captured SELECTIVELY: where it carries substantive output — an advisory, the not-authorised\n\
         filing notice, a Form 8283 caveat — it is shown in a separately labelled `stderr:` block, never\n\
         merged into stdout. What is NOT shown is the fixed integrity banner that a pinned `BTCTAX_NOW`\n\
         prints to stderr on the clock-pinned steps: that banner is the seam's own reproducibility\n\
         notice, not part of a command's result,\n\
         and is deliberately elided (disclosed here so the omission is never silent).\n\n",
    );
}

/// Write a committed synthetic corpus into `cwd/name` (so a journey's `import` sees a real file).
fn write_corpus(cwd: &Path, name: &str, content: &str) {
    std::fs::write(cwd.join(name), content).expect("write corpus");
}

/// A no-`now` non-stderr command (the common case).
fn plain<'a>(args: &'a [&'a str]) -> Cmd<'a> {
    Cmd {
        args,
        now: None,
        show_stderr: false,
    }
}

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
    journey_j4(&mut md, bin);
    journey_j5(&mut md, bin);
    journey_j6(&mut md, bin);
    journey_j7(&mut md, bin);
    journey_j8(&mut md, bin);
    journey_j9(&mut md, bin);

    md
}

/// J4 — crypto income, and reclassifying it as a trade or business (Schedule SE self-employment tax).
fn journey_j4(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J4 — mining/staking income and self-employment tax\n\n\
         Erin receives staking rewards on River. Imported, they are ordinary income at fair market\n\
         value on the day received (btctax reads the FMV from its bundled daily-close dataset; an\n\
         off-dataset day would instead flag a *missing-FMV* blocker to resolve by hand). Set a profile\n\
         and see the ordinary income:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "river.csv", J4_CSV);
    // deterministic income refs (the id embeds the ms-timestamp of the received date, not wall-clock)
    let r1 = "import|river|in|1744718400000|income|5000000#0";
    let r2 = "import|river|in|1747742400000|income|3000000#0";
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "river.csv"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "tax-profile",
            "--year",
            "2025",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "100000",
            "--magi-excluding-crypto",
            "100000",
            "--qualified-dividends",
            "0",
        ]),
    );
    md.push_str(
        "\nAdapters import income as *not* a business by default. If mining/staking is a trade or\n\
         business, reclassify each receipt — that moves it onto Schedule SE (self-employment tax):\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "reclassify-income",
            r1,
            "--business",
            "true",
            "--kind",
            "staking",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "reclassify-income",
            r2,
            "--business",
            "true",
            "--kind",
            "staking",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "report", "--tax-year", "2025"]),
    );
}

/// J5 — lot-selection optimization + attestation, and a what-if planning query. Showcases the
/// `made ≤ sale → Contemporaneous` lever that the `BTCTAX_NOW` seam pins; the `made > sale → attest`
/// branch is described in prose (demonstrating it needs a first-time post-sale accept on a separate
/// disposal — a re-accept here just reports "already optimal"; recorded in SPEC r2, descope (b)).
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
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "tax-profile",
            "--year",
            "2025",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "100000",
            "--magi-excluding-crypto",
            "100000",
            "--qualified-dividends",
            "0",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "config",
                "--set-forward-method",
                "fifo",
                "--effective-from",
                "2025-01-01",
            ],
            now: Some(now),
            show_stderr: false,
        },
    );
    md.push_str("\n`optimize run` is read-only — it proposes, files nothing:\n\n");
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &["--vault", "v.pgp", "optimize", "run", "--tax-year", "2025"],
            now: Some(now),
            show_stderr: false,
        },
    );
    md.push_str(
        "\nAccept it. Because the identification is made *before* the sale date, it is persisted as\n\
         **Contemporaneous** (an identification made after the sale would instead require an\n\
         attestation — this is exactly what the pinned clock governs, Treas. Reg. §1.1012-1(j)):\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "optimize",
                "accept",
                "--tax-year",
                "2025",
            ],
            now: Some(now),
            show_stderr: false,
        },
    );
    md.push_str(
        "\nAnd a forward-looking what-if — the marginal tax of a hypothetical future sale:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "what-if",
            "sell",
            "--sell",
            "0.5",
            "--wallet",
            "exchange:coinbase:default",
            "--at",
            "2025-07-01",
        ]),
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
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"])); // exits 1: the hard blocker
    md.push_str(
        "\nClassify it as your own coins returning — non-taxable, carrying the original basis and\n\
         acquisition date (for the holding period). The blocker clears and the ledger balances:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "reconcile",
                "classify-inbound-self-transfer",
                inbound,
                "--basis",
                "19000.00",
                "--acquired",
                "2024-11-01",
            ],
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
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "reclassify-outflow",
            donation,
            "--as-kind",
            "donate",
            "--amount",
            "217992.34",
            "--donee",
            "Habitat for Humanity",
        ]),
    );
    md.push_str("\nRecord the Form 8283 Section-B appraiser + donee details:\n\n");
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "set-donation-details",
            donation,
            "--donee-name",
            "Habitat for Humanity",
            "--donee-ein",
            "98-7654321",
            "--appraiser-name",
            "Jane Appraiser",
            "--appraiser-tin",
            "12-3456789",
            "--appraiser-qualifications",
            "ASA-accredited digital-asset appraiser, 8 yrs",
            "--appraisal-date",
            "2025-09-15",
        ]),
    );
    md.push_str(
        "\n`verify` recomputes the §170(e) deduction (long-term lot → FMV; short-term lot →\n\
         min(FMV, basis)) and flags the qualified-appraisal requirement for a >$5,000 crypto gift:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &["--vault", "v.pgp", "verify"],
            now: None,
            show_stderr: false,
        },
    );
    md.push_str(
        "\nFill Form 8283. Because this gift spans two lots, the Section B form carries two property rows;\n\
         btctax fills the appraiser + donee declaration on the first and flags the second for you to\n\
         complete on the paper form — that is the `needs REVIEW` note on stderr below (the appraiser\n\
         details ARE recorded; the flag is about the extra property row, not your input). A single-lot\n\
         gift — see J6 — clears with no such note:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "export-irs-pdf",
                "--out",
                "irs",
                "--tax-year",
                "2025",
                "--forms",
                "form8283",
            ],
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
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
    md.push_str(
        "\nThe year's tax is *not computable* until a tax profile is set (btctax refuses to guess your\n\
         bracket). Set one, then the report computes:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "tax-profile",
            "--year",
            "2025",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "100000",
            "--magi-excluding-crypto",
            "100000",
            "--qualified-dividends",
            "0",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "report", "--tax-year", "2025"]),
    );
    md.push_str(
        "\nExport the reconciled snapshot (CSVs + a decrypted SQLite) and fill the IRS forms:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "export-snapshot",
            "--out",
            "snapshot",
            "--tax-year",
            "2025",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "export-irs-pdf",
                "--out",
                "irs",
                "--tax-year",
                "2025",
                "--forms",
                "f8949,schedule-d",
            ],
            now: None,
            show_stderr: true, // the NOT-AUTHORISED notice + 1099-DA caveat are on stderr and matter
        },
    );
}

/// J6 — a COMPLETE Form 1040: crypto activity (mining income, a sale, a donation) combined with a full
/// non-crypto household imported from a TOML, exporting all fourteen forms of the return in one packet.
fn journey_j6(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J6 — a complete return (the full 1040 packet)\n\n\
         Frank has a full tax life, not just crypto: wages, interest, dividends, a mortgage, and a\n\
         dependent — plus Bitcoin mining income, a sale, and a charitable gift of appreciated coin. The\n\
         non-crypto figures live in an offline TOML (see `income import`); btctax merges them with the\n\
         reconciled ledger and fills the **entire** federal return. This is the TY2024 full-return path.\n\n\
         First the crypto side. Import the River mining export and the Coinbase export (a 2020 lot, a\n\
         2024 sale, and a 2024 donation), then make the ledger filing-ready:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "river.csv", J6_RIVER_CSV);
    write_corpus(cwd, "coinbase.csv", J6_COINBASE_CSV);
    write_corpus(cwd, "fullreturn.toml", J6_FULLRETURN_TOML);
    // Deterministic refs: the income id embeds the ms-timestamp of 2024-03-15T12:00:00Z; the donation is
    // the Coinbase Send `cb-donate`.
    let income = "import|river|in|1710504000000|income|5000000#0";
    let donation = "import|coinbase|out|cb-donate";
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv", "river.csv"]),
    );
    md.push_str(
        "\nThe mining income is a trade or business (moves it onto Schedule C ⇒ Schedule SE):\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "reclassify-income",
            income,
            "--business",
            "true",
            "--kind",
            "mining",
        ]),
    );
    md.push_str("\nThe outbound 0.1 BTC is a §170(e) charitable donation (⇒ Form 8283):\n\n");
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "reclassify-outflow",
            donation,
            "--as-kind",
            "donate",
            "--amount",
            "6000.00",
            "--donee",
            "Habitat for Humanity",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "reconcile",
            "set-donation-details",
            donation,
            "--donee-name",
            "Habitat for Humanity",
            "--donee-ein",
            "98-7654321",
            "--appraiser-name",
            "Jane Appraiser",
            "--appraiser-tin",
            "12-3456789",
            "--appraiser-qualifications",
            "ASA-accredited digital-asset appraiser, 8 yrs",
            "--appraisal-date",
            "2024-09-15",
        ]),
    );
    md.push_str("\nCheck the ledger balances and the §170(e) deduction is computed:\n\n");
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
    md.push_str(
        "\nNow the non-crypto side. `income import` reads the offline TOML — wages, interest (Schedule B),\n\
         dividends, the itemized deductions (Schedule A), and the fail-loud yes/no questions the return\n\
         requires. Unknown keys are rejected, never silently dropped:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "income",
            "import",
            "--year",
            "2024",
            "--file",
            "fullreturn.toml",
        ]),
    );
    md.push_str("\n`income show` echoes the stored inputs with every SSN and IP-PIN redacted (they never reach a pipe or your scrollback):\n\n");
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "income", "show", "--year", "2024"]),
    );
    md.push_str(
        "\nExport the whole return. With full-return inputs present btctax fills the entire packet — the\n\
         1040 and every schedule and attachment it cites, in IRS Attachment-Sequence stapling order,\n\
         plus a `manifest.txt`:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "export-irs-pdf",
                "--out",
                "irs",
                "--tax-year",
                "2024",
            ],
            now: None,
            show_stderr: true, // the NOT-AUTHORISED notice + the Form 8283 Section-B signature caveat
        },
    );
}

/// J7 — manual income FMV (UX-P1-7). An inbound deposit of coins you earned as staking rewards is an
/// unknown-basis TransferIn: a hard blocker until classified as income. The single-event
/// `classify-inbound-income` command does NO auto-valuation — you supply the fair-market value at receipt
/// with `--fmv` (the bulk command is the one that reads the bundled daily close).
fn journey_j7(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J7 — income received off-exchange, valued by hand (`--fmv`)\n\n\
         Frank earns staking rewards on a platform btctax has no price feed for and moves them to\n\
         Coinbase. Imported, the deposit is an unknown-basis inbound — a **hard blocker** until you say\n\
         what it is:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J7_CSV);
    let inbound = "import|coinbase|in|cb-recv";
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"])); // exits 1: the hard blocker
    md.push_str(
        "\nClassify it as staking income. On this single-event command there is no auto-valuation —\n\
         supply the FMV at receipt (from your own records) with `--fmv`; omitting it would record a\n\
         missing-FMV blocker instead. The blocker clears:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "reconcile",
                "classify-inbound-income",
                inbound,
                "--kind",
                "staking",
                "--fmv",
                "3300.00",
            ],
            now: Some("2024-07-01T00:00:00Z"), // decision made-date pinned (banner → stderr, not captured)
            show_stderr: false,
        },
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
    md.push_str(
        "\nWith a profile set, the hand-entered FMV is the ordinary income the report attributes to\n\
         crypto:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&[
            "--vault",
            "v.pgp",
            "tax-profile",
            "--year",
            "2024",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "90000",
            "--magi-excluding-crypto",
            "90000",
            "--qualified-dividends",
            "0",
        ]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "report", "--tax-year", "2024"]),
    );
}

/// J8 — a self-transfer across two exchanges (UX-P1-8). Coins leave River (a Withdrawal → TransferOut) and
/// land at Coinbase (a Receive → TransferIn). Unreconciled, that is a hard blocker. `match-self-transfers`
/// with no arguments PREVIEWS the proposed pairs (read-only); confirming one with `--in`/`--out` records a
/// cross-wallet RELOCATE (never automatic — you confirm the pairing).
fn journey_j8(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J8 — matching a self-transfer across two exchanges\n\n\
         Grace withdraws 0.10 BTC from River and deposits it at Coinbase. Imported, the two legs are\n\
         **unreconciled transfers** — a hard blocker until btctax knows they are the same coins moving,\n\
         not a disposal and a mystery deposit:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "river.csv", J8_RIVER_CSV);
    write_corpus(cwd, "coinbase.csv", J8_COINBASE_CSV);
    let out_leg = "import|river|out|1741608000000|withdrawal|10000000#0";
    let in_leg = "import|coinbase|in|cb-recv";
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "river.csv"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"])); // exits 1: unreconciled transfers
    md.push_str(
        "\nWith no arguments, `match-self-transfers` is read-only — it PREVIEWS the pairs it can see (an\n\
         out-leg and an in-leg of equal size across your wallets), proposing a **RELOCATE** because the\n\
         coins land in a different wallet:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "reconcile", "match-self-transfers"]),
    );
    md.push_str(
        "\nConfirm that pair by naming both legs. The relocation carries the original basis and holding\n\
         period to Coinbase, and the ledger balances:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "reconcile",
                "match-self-transfers",
                "--in",
                in_leg,
                "--out",
                out_leg,
            ],
            now: Some("2025-04-01T00:00:00Z"), // decision made-date pinned (banner → stderr, not captured)
            show_stderr: false,
        },
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
}

/// J9 — choosing which lots a sale draws from (UX-P1-10). With two lots and a sale smaller than either
/// combined holding, the default method picks the lots for you; `select-lots` lets you identify EXACTLY
/// which ones — the picks (`<origin>#<split>:<sat>`) come from the disposal's `lot` column in
/// export-snapshot, and their sats must sum to the disposal's size.
fn journey_j9(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J9 — identifying specific lots for a disposal (`select-lots`)\n\n\
         Heidi holds a cheap 2023 long-term lot (0.60 BTC) and a pricier 2024 lot (0.40 BTC), and sells\n\
         0.50 — less than her holdings, so *which* lots the sale draws from is a real choice.\n\
         `export-snapshot` writes a `disposals.csv` whose `lot` column shows the default split and the\n\
         `<origin>#<split>` refs you would name to choose differently (each origin is the lot's acquiring\n\
         trade, e.g. `import|coinbase|trade|lot-a`):\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J9_CSV);
    let disposal = "import|coinbase|trade|sale";
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]),
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "export-snapshot", "--out", "snapshot"]),
    );
    md.push_str(
        "\nThe default split draws from both lots. Instead, identify the whole 0.50 against the cheap\n\
         long-term lot `lot-a` — a deliberate per-disposal identification (the picks' sats must total the\n\
         0.50 BTC / 50 000 000 sat disposal):\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &Cmd {
            args: &[
                "--vault",
                "v.pgp",
                "reconcile",
                "select-lots",
                disposal,
                "--from",
                "import|coinbase|trade|lot-a#0:50000000",
            ],
            now: Some("2025-01-01T00:00:00Z"), // identification made before the sale → contemporaneous
            show_stderr: false,
        },
    );
    md.push_str(
        "\nRe-export: the disposal now draws entirely from `lot-a`, and the selection is recorded as\n\
         per-disposal compliance:\n\n",
    );
    emit(
        md,
        bin,
        cwd,
        &plain(&["--vault", "v.pgp", "export-snapshot", "--out", "snapshot2"]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
}

/// Regenerate the committed golden to stdout (`cargo run -p xtask -- examples`).
pub fn run() {
    let bin = built_btctax();
    print!("{}", generate(&bin));
}

/// Collect every LEAF subcommand path of the CLI (a command with no subcommands), skipping clap's `help`
/// pseudo-command. `["reconcile", "reclassify-outflow"]`, `["income", "import"]`, `["what-if", "sell"]`, …
fn leaf_subcommands() -> Vec<Vec<String>> {
    use btctax_cli::cli::Cli;
    use clap::CommandFactory;
    fn walk(cmd: &clap::Command, path: &[String], out: &mut Vec<Vec<String>>) {
        let subs: Vec<&clap::Command> = cmd
            .get_subcommands()
            .filter(|s| s.get_name() != "help")
            .collect();
        if subs.is_empty() {
            if !path.is_empty() {
                out.push(path.to_vec());
            }
            return;
        }
        for sub in subs {
            let mut p = path.to_vec();
            p.push(sub.get_name().to_string());
            walk(sub, &p, out);
        }
    }
    let mut out = Vec::new();
    walk(&Cli::command(), &[], &mut out);
    out.sort();
    out
}

/// Global options that appear BEFORE the subcommand on a `$ btctax …` line and CONSUME a value — the
/// `--vault v.pgp` pair every journey opens with. Anchoring skips both tokens so `path[0]` binds to the
/// actual subcommand, never the flag's value (UX-P2-1).
const GLOBAL_VALUE_OPTS: &[&str] = &["--vault"];

/// Whether some `$ btctax …` line in the golden runs the leaf `path`: after skipping leading global
/// options, `path[0]` must be EXACTLY the first subcommand token, and the remaining sub-verb tokens then
/// appear in order (UX-P2-1 — a name that shows up only as an argument does not count).
fn is_demonstrated(golden: &str, path: &[String]) -> bool {
    golden
        .lines()
        .filter_map(|l| l.trim().strip_prefix("$ btctax"))
        .any(|rest| {
            let toks: Vec<&str> = rest.split_whitespace().collect();
            // UX-P2-1: advance past leading global options (a `-`-prefixed flag, plus its value when it
            // takes one) to the FIRST subcommand token, so a global flag/value is never miscounted.
            let mut start = 0;
            while let Some(&t) = toks.get(start) {
                if t.starts_with('-') {
                    start += 1;
                    if GLOBAL_VALUE_OPTS.contains(&t) {
                        start += 1; // also skip the option's value (e.g. `v.pgp` after `--vault`)
                    }
                } else {
                    break;
                }
            }
            // `path[0]` MUST be exactly the first subcommand token (anchored — not a free subsequence), so a
            // subcommand named only as an ARGUMENT deeper in the line does not falsely count as demonstrated.
            let Some((first, rest_path)) = path.split_first() else {
                return true; // an empty path is trivially demonstrated by any invocation
            };
            if toks.get(start).map(|t| &**t) != Some(first.as_str()) {
                return false;
            }
            // The remaining path tokens (sub-subcommands) still subsequence-match after the anchor — a
            // sub-verb can be separated from its parent by nothing, and args never precede it.
            let mut i = start + 1;
            for p in rest_path {
                match toks.get(i..).and_then(|s| s.iter().position(|t| t == p)) {
                    Some(off) => i += off + 1,
                    None => return false,
                }
            }
            true
        })
}

/// The SOFT subcommand-coverage report (SPEC §6.3): which leaf subcommands have NO worked example in the
/// committed golden. Non-blocking — administrative/rare commands (`backup-key`, `init --repair`, …) need
/// no contrived example; this is a maintainer's map, printed/uploaded, never a gate.
pub fn subcommand_coverage_report() -> String {
    // Fail LOUD on a missing golden — a silent `.unwrap_or_default()` would print a confident "0/N …
    // demonstrated" (the golden scan finding nothing because there is no golden), a misleading map (M-1).
    let path = workspace_root().join("docs/examples/examples.md");
    let golden = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "subcommand-coverage: cannot read {} ({e}) — regenerate it first: \
             `cargo run -p xtask -- examples > docs/examples/examples.md`",
            path.display()
        )
    });
    let leaves = leaf_subcommands();
    let (mut covered, mut uncovered) = (0usize, Vec::new());
    for path in &leaves {
        if is_demonstrated(&golden, path) {
            covered += 1;
        } else {
            uncovered.push(path.join(" "));
        }
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Subcommand coverage (SOFT — SPEC §6.3): {covered}/{} leaf subcommands have a worked example.",
        leaves.len()
    );
    if uncovered.is_empty() {
        out.push_str("  every leaf subcommand is demonstrated.\n");
    } else {
        out.push_str(
            "  not demonstrated (no worked example — administrative/rare commands need none):\n",
        );
        for n in &uncovered {
            let _ = writeln!(out, "    - btctax {n}");
        }
    }
    out
}

/// `cargo run -p xtask -- subcommand-coverage` — print the SOFT coverage report (SPEC §6.3, Task 2.2).
pub fn run_coverage() {
    print!("{}", subcommand_coverage_report());
}

// UX-P2-1: `is_demonstrated` is pure string logic — its tests run on every platform (NOT unix-gated like
// the byte-exact golden tests below), so the coverage map cannot silently over-report on Windows/macOS.
#[cfg(test)]
mod matcher_tests {
    use super::is_demonstrated;

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    /// The real subcommand IS demonstrated even though a global `--vault v.pgp` option precedes it —
    /// anchoring must SKIP the leading global option (and its value), not stop at it.
    #[test]
    fn anchors_past_the_global_vault_option() {
        let golden = "$ btctax --vault v.pgp verify\n";
        assert!(is_demonstrated(golden, &path(&["verify"])));
        let golden2 = "$ btctax --vault v.pgp reconcile void abc123\n";
        assert!(is_demonstrated(golden2, &path(&["reconcile", "void"])));
    }

    /// ★ UX-P2-1 core: a subcommand name that appears ONLY as an argument (never as the invoked
    /// subcommand) must NOT be reported demonstrated. The old free-subsequence matcher counted
    /// `path[0]` anywhere in the line, so a `reconcile void import` line falsely "demonstrated" the
    /// top-level `import` leaf. Anchoring `path[0]` to the first non-`-` token kills that over-report.
    #[test]
    fn a_subcommand_named_only_as_an_argument_is_not_demonstrated() {
        let golden = "$ btctax --vault v.pgp reconcile void import\n";
        assert!(
            !is_demonstrated(golden, &path(&["import"])),
            "`import` as a bare argument to `reconcile void` must not count as demonstrating the \
             `import` subcommand"
        );
        // …but the command actually invoked on that line still is.
        assert!(is_demonstrated(golden, &path(&["reconcile", "void"])));
    }

    /// The `--vault` flag itself and its value token must never be mistaken for the subcommand.
    #[test]
    fn the_vault_flag_and_its_value_are_not_subcommands() {
        let golden = "$ btctax --vault v.pgp verify\n";
        assert!(!is_demonstrated(golden, &path(&["v.pgp"])));
        assert!(!is_demonstrated(golden, &path(&["--vault"])));
    }
}

#[cfg(test)]
#[cfg(unix)] // journey stdout can embed joined paths; byte-exact goldens are gated on unix (I4)
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes the tests that (a) shell out to `cargo build` via [`built_btctax`] and/or (b) mutate
    /// process-global env (`HOME`, `BTCTAX_PRICE_CACHE`). Under nextest each test is its own process so
    /// this is uncontended; under threaded `cargo test` it prevents a HOME-mutating test from corrupting a
    /// sibling's concurrent `cargo build` (cargo reads `$HOME/.cargo` when `CARGO_HOME` is unset).
    static BUILD_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn generate_is_deterministic_and_captures_help() {
        let _guard = BUILD_ENV_LOCK.lock().unwrap();
        let bin = built_btctax();
        let a = generate(&bin);
        let b = generate(&bin);
        assert_eq!(a, b, "generate() must be byte-deterministic");
        assert!(
            a.contains("$ btctax --help"),
            "must show the verbatim command"
        );
        assert!(
            a.contains("Usage: btctax"),
            "must capture the real help output"
        );
        assert!(
            a.contains(&format!("btctax-version: {}", btctax_cli_version())),
            "front-matter version"
        );
    }

    /// The committed golden matches a fresh generation, byte-for-byte — reds when `docs/examples/examples.md`
    /// is STALE (a code/output change that wasn't regenerated). This is the in-tree half of the CI
    /// `git diff --exit-code docs/examples` gate (Task 2.3); it fires on every `make check`.
    #[test]
    fn examples_golden_matches_committed() {
        let _guard = BUILD_ENV_LOCK.lock().unwrap();
        let generated = generate(&built_btctax());
        let path = workspace_root().join("docs/examples/examples.md");
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "committed {} missing ({e}); regenerate with \
                 `cargo run -p xtask -- examples > docs/examples/examples.md`",
                path.display()
            )
        });
        assert_eq!(
            generated, committed,
            "docs/examples/examples.md is STALE; regenerate with \
             `cargo run -p xtask -- examples > docs/examples/examples.md`"
        );
    }

    /// TUI screen-walkthrough manifest gate (SPEC §5; folds PoC review C-1). Each
    /// `docs/examples-tui-walkthrough/<journey>/manifest.txt` is hand-authored (prose + captions live
    /// ONLY here — the single owner), so nothing REGENERATES it; instead this pins its INTEGRITY on every
    /// `make check`, the property the byte-gated `examples.md` gives that artifact:
    ///   (a) grammar — every non-blank, non-`#` line is `PROSE <text>` or `FRAME <file.txt> <caption>`,
    ///       both with non-empty payloads (mirrors, and slightly tightens, `assemble-walkthrough.sh`); and
    ///   (b) a BIJECTION between the manifest's `FRAME` references and the frame goldens on disk — every
    ///       reference resolves to a committed `.txt`, AND every committed `.txt` (bar `manifest.txt`) is
    ///       referenced. (a→b) alone catches a typo'd/renamed reference; (b→a) catches a SILENTLY DROPPED
    ///       `FRAME` line, which would otherwise leave an orphaned golden and quietly shrink the walkthrough
    ///       with the whole validation surface still green (the exact hole the review flagged).
    /// The frames themselves are byte-gated in the TUI crates (`*_walkthrough_goldens_match_committed`);
    /// this + those together pin the whole artifact. Generalizes for free to Phase 2's J1..J7,J9 dirs.
    #[test]
    fn walkthrough_manifests_valid_and_complete() {
        let root = workspace_root().join("docs/examples-tui-walkthrough");
        // NEW-N-1: assert (don't `return`) when the dir is absent, so this can never pass vacuously —
        // the `journeys_checked >= 1` floor below only bites if we actually reach it.
        assert!(
            root.is_dir(),
            "docs/examples-tui-walkthrough is missing — the PoC J8 manifest should be present"
        );
        let mut journeys_checked = 0;
        for entry in std::fs::read_dir(&root).expect("read docs/examples-tui-walkthrough") {
            let dir = entry.expect("dir entry").path();
            let manifest = dir.join("manifest.txt");
            if !manifest.is_file() {
                continue; // a non-journey subdir (none today) — skip
            }
            journeys_checked += 1;
            let rel = dir.file_name().unwrap().to_string_lossy().to_string();

            // (a) grammar — collect the FRAME references.
            let text = std::fs::read_to_string(&manifest).unwrap();
            let mut referenced: std::collections::BTreeSet<String> = Default::default();
            for (i, line) in text.lines().enumerate() {
                let ln = i + 1;
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(rest) = line.strip_prefix("PROSE ") {
                    assert!(
                        !rest.trim().is_empty(),
                        "{rel}/manifest.txt:{ln}: empty PROSE line"
                    );
                } else if let Some(rest) = line.strip_prefix("FRAME ") {
                    let (file, caption) = rest.split_once(' ').unwrap_or((rest, ""));
                    assert!(
                        !caption.trim().is_empty(),
                        "{rel}/manifest.txt:{ln}: FRAME `{file}` has no caption \
                         (the `.SH` would degrade to the filename)"
                    );
                    assert!(
                        file.ends_with(".txt") && file != "manifest.txt",
                        "{rel}/manifest.txt:{ln}: FRAME reference `{file}` is not a frame `.txt`"
                    );
                    assert!(
                        referenced.insert(file.to_string()),
                        "{rel}/manifest.txt:{ln}: FRAME `{file}` referenced twice"
                    );
                } else {
                    panic!("{rel}/manifest.txt:{ln}: not PROSE/FRAME/#comment/blank: {line:?}");
                }
            }

            // (b) bijection — the on-disk frame goldens (every `*.txt` except the manifest itself).
            let mut on_disk: std::collections::BTreeSet<String> = Default::default();
            for e in std::fs::read_dir(&dir).unwrap() {
                let name = e.unwrap().file_name().to_string_lossy().to_string();
                if name.ends_with(".txt") && name != "manifest.txt" {
                    on_disk.insert(name);
                }
            }
            assert_eq!(
                referenced, on_disk,
                "{rel}: the manifest's FRAME references and the committed frame goldens must be a \
                 bijection — a reference with no golden is a typo/rename; a golden with no reference is \
                 a silently dropped FRAME line (a shrunk walkthrough). referenced={referenced:?} \
                 on_disk={on_disk:?}"
            );
        }
        assert!(
            journeys_checked >= 1,
            "no walkthrough manifests found under docs/examples-tui-walkthrough — the PoC J8 manifest \
             should be present"
        );
    }

    /// Captures a set of env vars and restores them on Drop — so a panic inside `generate()` cannot leave
    /// the process env dirty for a sibling test under threaded `cargo test` (N-1). Held with BUILD_ENV_LOCK.
    struct EnvRestore(Vec<(&'static str, Option<std::ffi::OsString>)>);
    impl EnvRestore {
        fn capture(keys: &[&'static str]) -> Self {
            EnvRestore(keys.iter().map(|k| (*k, std::env::var_os(k))).collect())
        }
    }
    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (k, v) in &self.0 {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    /// Hermeticity (SPEC §7, M4): the generator pins `HOME`, `BTCTAX_PRICE_CACHE`, and `BTCTAX_NOW` per
    /// captured command, so ambient values — a real `HOME`, a PRESENT price cache, or a stray `BTCTAX_NOW`
    /// (M-1) — cannot bleed into the golden. Proven by regenerating under a junk `HOME` + a present dummy
    /// cache + a bogus `BTCTAX_NOW` and asserting byte-identity with the baseline. This is also the guard
    /// that would red if `capture()` stopped clearing an unpinned step's `BTCTAX_NOW`.
    #[test]
    fn examples_generate_is_hermetic_across_ambient_env() {
        let _guard = BUILD_ENV_LOCK.lock().unwrap();
        let bin = built_btctax(); // build BEFORE mutating HOME (cargo reads $HOME/.cargo)
        let baseline = generate(&bin);

        let tmp = tempfile::tempdir().expect("tempdir");
        let present_cache = tmp.path().join("present-cache.csv");
        std::fs::write(&present_cache, "date,usd\n2024-01-01,42000.00\n")
            .expect("write dummy cache");
        let junk_home = tmp.path().join("junk-home");
        std::fs::create_dir_all(&junk_home).expect("mkdir junk home");

        let perturbed = {
            let _restore = EnvRestore::capture(&["HOME", "BTCTAX_PRICE_CACHE", "BTCTAX_NOW"]);
            std::env::set_var("HOME", &junk_home);
            std::env::set_var("BTCTAX_PRICE_CACHE", &present_cache);
            std::env::set_var("BTCTAX_NOW", "2099-01-01T00:00:00Z"); // must NOT reach an unpinned step
                                                                     // `_restore` drops at the end of this block — after `generate()` returns and before the assert
                                                                     // (and on any panic inside `generate()`), so the process env is never left dirty for a sibling.
            generate(&bin)
        };

        assert_eq!(
            baseline, perturbed,
            "the golden must not depend on ambient HOME, a PRESENT price cache, or a stray BTCTAX_NOW \
             (SPEC §7 hermeticity)"
        );
    }

    /// The SOFT subcommand-coverage report (SPEC §6.3) walks the CLI + scans the golden without panicking
    /// and returns a well-formed summary. Non-gating content (the report is advisory), but the walk/scan
    /// must stay sound. No binary build — reads the committed golden + `Cli::command()`.
    #[test]
    fn subcommand_coverage_report_is_well_formed() {
        assert!(
            !leaf_subcommands().is_empty(),
            "the CLI must have leaf subcommands to report on"
        );
        let report = subcommand_coverage_report();
        assert!(
            report.starts_with("Subcommand coverage (SOFT"),
            "the report must open with its summary line; got: {report:?}"
        );
        // A 0/N split means the golden scan is broken (missing golden now panics upstream, but a matcher
        // regression could still zero the count) — enforce it, don't just claim it in a comment (M-1).
        assert!(
            !report.contains("): 0/"),
            "the covered count must be non-zero — 0/N ⇒ the golden scan is broken: {report}"
        );
    }

    /// UX-P1-7/8/10: the three new worked-example journeys actually EXERCISE their reconcile commands — a
    /// fresh generation must demonstrate `classify-inbound-income` (manual FMV, J7), `match-self-transfers`
    /// (two-exchange, J8), and `select-lots` (per-disposal, J9). This tests the CODE (a fresh generation),
    /// not the committed file, so it reds if a journey is dropped or stops invoking its command — a silent
    /// coverage regression the byte-golden alone would not catch (it would merely re-pin the weaker output).
    /// Uses the UX-P2-1-anchored matcher, so a command named only as an argument would NOT satisfy it.
    #[test]
    fn new_journeys_demonstrate_their_reconcile_commands() {
        let _guard = BUILD_ENV_LOCK.lock().unwrap();
        let golden = generate(&built_btctax());
        for leaf in [
            ["reconcile", "classify-inbound-income"].as_slice(),
            ["reconcile", "match-self-transfers"].as_slice(),
            ["reconcile", "select-lots"].as_slice(),
        ] {
            let path: Vec<String> = leaf.iter().map(|s| s.to_string()).collect();
            assert!(
                is_demonstrated(&golden, &path),
                "UX-P1-7/8/10: `btctax {}` must have a worked example in the generated golden",
                leaf.join(" ")
            );
        }
    }
}
