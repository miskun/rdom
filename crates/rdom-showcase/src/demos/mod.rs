//! Demo implementations. Each demo is a module that defines a
//! single unit struct implementing [`crate::Demo`]. The registry
//! ([`crate::registry::DEMOS`]) holds `'static` references to
//! const instances of those structs.
//!
//! Adding a demo:
//! 1. Add a new module file under this directory.
//! 2. Define a unit struct + impl `Demo` for it.
//! 3. Register the struct in [`crate::registry::DEMOS`].

pub mod flex_row;
pub mod hello;
pub mod hover;
