//! Editor mutation modules.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! `persist` is the ONLY module permitted to name the mutation surface
//! (`conn()` / `save()` / `tax_profile::set` / `append_decision` / `apply_declare(` — ★ Task 8/C-3:
//! the Defensive Filing Wizard's DECLARE chokepoint write, confined to `persist::persist_declare_tranche`
//! / `apply_promote(` — ★ Task 9/C-3: the Defensive Filing Wizard's PROMOTE chokepoint write, confined to
//! `persist::persist_promote_tranche` — and mechanically enforced by
//! `persist::tests::kat_g1_mechanized_source_gate`).

pub mod declare_flow;
pub mod form;
pub mod persist;
pub mod promote_flow;
pub mod tax_inputs;
