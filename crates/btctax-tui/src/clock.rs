//! The TUI's deterministic clock seam (SPEC §3.4) — the TUI analogue of the CLI's `resolve_now`
//! (`btctax-cli/src/main.rs`). Every production wall-clock read in `btctax-tui` AND `btctax-tui-edit`
//! (which depends on this crate) routes through an injected [`Clock`], so a pinned `BTCTAX_NOW` makes the
//! style-aware TUI goldens deterministic.
//!
//! **§3.1 fence.** With `BTCTAX_NOW` unset, [`Clock::Wall`] returns `OffsetDateTime::now_utc()` on EVERY
//! call — byte-identical to the pre-seam behavior, AND preserving real-session semantics (each decision
//! is stamped with its own made-date, not a frozen startup time). A [`Clock::Pinned`] clock returns the
//! same instant on every call (simulated time). This injects an *input*; it never transforms an *output*,
//! and the equivalence is pinned by tests — a determinism prerequisite, not an engine edit.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// An injected clock. `Wall` reads the system clock per call (the production default); `Pinned` returns a
/// fixed instant on every call (a `BTCTAX_NOW` override, or a test / golden-capture harness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clock {
    /// Read `OffsetDateTime::now_utc()` on each call — the default when `BTCTAX_NOW` is unset.
    #[default]
    Wall,
    /// Return a fixed instant on every call (simulated time).
    Pinned(OffsetDateTime),
}

impl Clock {
    /// The current time under this clock: a fresh `now_utc()` for `Wall`, the fixed instant for `Pinned`.
    pub fn now(&self) -> OffsetDateTime {
        match self {
            Clock::Wall => OffsetDateTime::now_utc(),
            Clock::Pinned(t) => *t,
        }
    }
}

/// Resolve a [`Clock`] from the environment, mirroring the CLI `BTCTAX_NOW` contract EXACTLY: unset →
/// `Wall`; a valid RFC3339 timestamp → `Pinned`; non-UTF-8 or malformed (including empty) → `Err`. The
/// caller prints the message and exits 2 BEFORE entering raw mode.
pub fn from_env() -> Result<Clock, String> {
    match std::env::var_os("BTCTAX_NOW") {
        None => Ok(Clock::Wall),
        Some(os) => {
            let s = os
                .to_str()
                .ok_or_else(|| "BTCTAX_NOW is set but not valid UTF-8".to_string())?;
            let t = OffsetDateTime::parse(s, &Rfc3339).map_err(|e| {
                format!(
                    "BTCTAX_NOW is set but not a valid RFC3339 timestamp ({s:?}): {e}. \
                     Expected e.g. 2026-02-01T12:00:00Z."
                )
            })?;
            Ok(Clock::Pinned(t))
        }
    }
}

/// The disclosure a binary prints to stderr ONCE at startup — before `enable_raw_mode`, since the
/// alternate screen hides stderr during the session — when a `BTCTAX_NOW` override is active. The TUI
/// analogue of the CLI's unconditional stderr banner.
pub const OVERRIDE_BANNER: &str =
    "warning: BTCTAX_NOW override active — decision timestamps are simulated";

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn wall_is_the_default_and_reads_the_system_clock() {
        assert_eq!(Clock::default(), Clock::Wall);
        let a = Clock::Wall.now();
        let b = Clock::Wall.now();
        assert!(
            b >= a,
            "Wall advances monotonically (or is equal within a tick)"
        );
    }

    #[test]
    fn pinned_returns_the_same_instant_every_call() {
        let t = datetime!(2024 - 06 - 01 12:00:00 UTC);
        let c = Clock::Pinned(t);
        assert_eq!(c.now(), t);
        assert_eq!(c.now(), t);
    }

    /// Serializes the `from_env` tests (they mutate the process-global `BTCTAX_NOW`); save/restore around
    /// each so the mutation never leaks. `set_var` is safe on edition 2021.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn with_btctax_now<T>(val: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = std::env::var_os("BTCTAX_NOW");
        match val {
            Some(v) => std::env::set_var("BTCTAX_NOW", v),
            None => std::env::remove_var("BTCTAX_NOW"),
        }
        let r = f();
        match saved {
            Some(v) => std::env::set_var("BTCTAX_NOW", v),
            None => std::env::remove_var("BTCTAX_NOW"),
        }
        r
    }

    #[test]
    fn from_env_unset_is_wall() {
        assert_eq!(with_btctax_now(None, from_env).unwrap(), Clock::Wall);
    }

    #[test]
    fn from_env_valid_rfc3339_is_pinned() {
        let c = with_btctax_now(Some("2024-06-01T12:00:00Z"), from_env).unwrap();
        assert_eq!(c, Clock::Pinned(datetime!(2024 - 06 - 01 12:00:00 UTC)));
    }

    #[test]
    fn from_env_malformed_and_empty_are_err() {
        // The CLI `resolve_now` contract: a set-but-unparseable value is a hard error (the caller exits 2).
        assert!(with_btctax_now(Some("garbage"), from_env).is_err());
        assert!(with_btctax_now(Some("2024-06-01"), from_env).is_err()); // date only, not RFC3339
        assert!(with_btctax_now(Some(""), from_env).is_err());
    }
}
