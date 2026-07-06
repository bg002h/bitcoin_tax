//! `btctax-update-prices` binary — the opt-in online price-cache updater (#41 Part C). Thin shell over
//! `btctax_update_prices::run`; the ONLY btctax executable that links an HTTP client.
use btctax_update_prices::{run, Cli, RunOutcome};
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    let today = time::OffsetDateTime::now_utc().date();
    match run(&cli, today) {
        Ok(RunOutcome::UpToDate) => {
            println!(
                "Price cache is up to date (within the {}-day settling window). Nothing to do.",
                cli.lag
            );
        }
        Ok(RunOutcome::Updated {
            start,
            end,
            summary,
        }) => {
            println!(
                "Fetched daily closes for {start} … {end} (source: {:?}).",
                cli.source
            );
            if summary.dry_run {
                println!(
                    "DRY RUN — would append {} new close(s); {} already present. No file written.",
                    summary.appended, summary.skipped_present
                );
            } else {
                println!(
                    "Appended {} new close(s); skipped {} already present.",
                    summary.appended, summary.skipped_present
                );
            }
            println!("Cache: {}", summary.cache_path.display());
        }
        Err(e) => {
            eprintln!("btctax-update-prices: {e}");
            std::process::exit(1);
        }
    }
}
