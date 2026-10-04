//! The custom properties a sheet registers (CSS Properties and Values
//! API 1 §3 `CSS.registerProperty`, §4 `@property`): the data model is
//! [`PropertyRegistration`](crate::PropertyRegistration); a backend's
//! cascade reads the registrations of all its sheets, a later one of a
//! name winning.

use super::Stylesheet;

impl Stylesheet {
    /// The custom properties this sheet registers (`@property`,
    /// Properties and Values 1 §3), in source order; for a name
    /// registered twice the cascade uses the last.
    pub fn registered_properties(&self) -> &[crate::PropertyRegistration] {
        &self.registrations
    }

    /// Register a custom property (`@property`, or a Rust-built sheet's
    /// `CSS.registerProperty`).
    pub fn register_property(&mut self, registration: crate::PropertyRegistration) {
        self.touch();
        self.registrations.push(registration);
    }
}
