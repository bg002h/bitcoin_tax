//! ★ The UI-agnostic input-form engine (`design/SPEC_input_form.md`). A `FormSpec` tree over
//! `ReturnInputs`, a serde `Edit` seam, and `apply`/`parse`/`attribute` — rendered by the TUI now and a web
//! app later. Depends on `btctax-core` only; no vault, no terminal.
#![forbid(unsafe_code)]

// modules land in later tasks:
mod seam;      pub use seam::*;      // Task 2
mod spec;      pub use spec::*;      // Tasks 4-5
mod apply;     pub use apply::*;     // Task 7
// mod parse;     pub use parse::*;     // Task 8
// mod attribute; pub use attribute::*; // Task 9
