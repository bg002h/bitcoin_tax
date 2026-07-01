//! Viewer tab modules: Holdings, Disposals, Income, Tax, Forms, Compliance.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

pub mod compliance;
pub mod disposals;
pub mod forms;
pub mod holdings;
pub mod income;
mod tags;
pub mod tax;
mod utils;

#[cfg(test)]
mod tests;
