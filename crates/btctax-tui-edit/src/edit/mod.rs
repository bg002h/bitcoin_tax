//! Editor mutation modules.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! `persist` is the ONLY module permitted to name the mutation surface
//! (`conn()` / `save()` / `tax_profile::set` / `append_decision`).

pub mod form;
pub mod persist;
