//! The hardcoded demo registry. Explicit, boring, grep-able — no
//! build.rs scanning, no inventory crate, no macros. Adding a demo
//! is a single line here plus a new module under `crate::demos`.

use crate::Demo;
use crate::demos::hello::HelloWorld;

/// Every demo the showcase knows about, in stable order. Order
/// here determines display order in the sidebar (within each
/// category — the shell groups by `Category` for display).
pub const DEMOS: &[&dyn Demo] = &[&HelloWorld];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_not_empty() {
        assert!(!DEMOS.is_empty(), "at least one demo must be registered");
    }

    #[test]
    fn slugs_are_unique() {
        let mut seen: Vec<&'static str> = Vec::new();
        for d in DEMOS {
            let s = d.slug();
            assert!(
                !seen.contains(&s),
                "duplicate slug {s:?} — slugs must be unique across the registry"
            );
            seen.push(s);
        }
    }

    #[test]
    fn first_demo_is_hello_world() {
        // Stability check: the M2 scaffold puts HelloWorld in slot
        // 0 so the binary mounts it on startup. M3 makes the
        // sidebar interactive and this stops mattering.
        let first = DEMOS[0];
        assert_eq!(first.slug(), "layout/hello-world");
        assert_eq!(first.title(), "Hello World");
    }
}
