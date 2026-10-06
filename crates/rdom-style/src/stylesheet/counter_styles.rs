//! The counter styles a sheet defines (`@counter-style`, CSS Counter
//! Styles 3 §3). A backend's cascade reads the definitions of all its
//! sheets and lets the last of a name win, in cascade layer order
//! (CSS Cascade 5 §6.4.3: unlayered last), then sheet order, then source
//! order.

use super::Stylesheet;
use crate::counters::CounterStyleDefinition;

impl Stylesheet {
    /// The counter styles this sheet defines, in source order.
    pub fn counter_styles(&self) -> &[CounterStyleDefinition] {
        &self.counter_styles
    }

    /// Define a counter style (`@counter-style`, or a Rust-built sheet's).
    /// The caller checks the rule ([`crate::counters::check_rule`]).
    pub fn define_counter_style(&mut self, definition: CounterStyleDefinition) {
        self.touch();
        self.counter_styles.push(definition);
    }
}
