//! The `@keyframes` rules a sheet defines (CSS Animations 1 §3). A
//! backend reads the rules of all its sheets and lets the last of a name
//! win, in cascade layer order (CSS Cascade 5 §6.4.3: unlayered last),
//! then sheet order, then source order — rules of one name never merge.

use super::Stylesheet;
use crate::keyframes::KeyframesRule;

impl Stylesheet {
    /// The `@keyframes` rules this sheet defines, in source order.
    pub fn keyframes(&self) -> &[KeyframesRule] {
        &self.keyframes
    }

    /// Define a `@keyframes` rule (parsed, or a Rust-built sheet's).
    pub fn define_keyframes(&mut self, rule: KeyframesRule) {
        self.touch();
        self.keyframes.push(rule);
    }
}
