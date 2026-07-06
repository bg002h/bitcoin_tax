//! Resolve the local price-cache path (#41 Part C). `dirs` lives HERE (and in `btctax-update-prices`),
//! NOT in btctax-adapters [R0-M2/r2 M-A] — the adapter's `LayeredPrices` takes an ALREADY-resolved
//! `cache_path` and stays a pure format/provider crate (no `dirs`, no network). The cache is a documented
//! LOCAL INPUT the offline projection layers OVER the bundled dataset; it is populated ONLY by the
//! separate `btctax-update-prices` binary, so btctax-cli itself carries NO network dependency.
use std::path::PathBuf;

/// The env var that overrides the default cache location (tests / reproducibility / custom data dirs).
pub const PRICE_CACHE_ENV: &str = "BTCTAX_PRICE_CACHE";

/// The pointer surfaced next to a "no price for {date}" condition — a STRING only (no dep, no shell-out):
/// the tax binaries never fetch; the user runs the separate updater explicitly.
pub const UPDATE_PRICES_HINT: &str =
    "no local price for this date — run `btctax-update-prices` to fetch newer daily closes into the cache";

/// The default local price-cache path: `$BTCTAX_PRICE_CACHE` if set, else
/// `<data_dir>/btctax/price_cache.csv` (`dirs::data_dir()`), else `None` (no platform data dir ⇒
/// bundled-only — still fully functional, just no cache overlay).
pub fn default_cache_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(PRICE_CACHE_ENV) {
        return Some(PathBuf::from(p));
    }
    dirs::data_dir().map(|d| d.join("btctax").join("price_cache.csv"))
}
