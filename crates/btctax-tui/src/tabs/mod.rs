//! Viewer tab modules: Holdings, Disposals, Income.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

pub mod disposals;
pub mod holdings;
pub mod income;
mod tags;

#[cfg(test)]
mod tests;
