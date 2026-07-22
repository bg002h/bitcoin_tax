//! Defensive Filing Wizard: derived, read-only signals over the projected `LedgerState` — never new tax
//! logic, never a second source of truth. `discovery` is the DFW-D4 structured-shortfall/triage layer
//! (Task 5); Task 6 adds `journey_view` (the guided-dashboard read alongside it).
pub mod discovery;
