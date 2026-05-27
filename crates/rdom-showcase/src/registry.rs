//! The hardcoded demo registry. Explicit, boring, grep-able — no
//! build.rs scanning, no inventory crate, no macros. Adding a demo
//! is a single line here plus a new module under `crate::demos`.

use crate::Demo;
use crate::demos::border_collapse::BorderCollapse;
use crate::demos::counter_button::CounterButton;
use crate::demos::dom_api::DomApi;
use crate::demos::flex_row::FlexRow;
use crate::demos::headings::Headings;
use crate::demos::hello::HelloWorld;
use crate::demos::hover::Hover;
use crate::demos::inline_formatting::InlineFormatting;
use crate::demos::interval_counter::IntervalCounter;
use crate::demos::mutation_observer::MutationObserverDemo;
use crate::demos::parse_and_render::ParseAndRender;
use crate::demos::raf_progress::RafProgress;
use crate::demos::scrollable_list::ScrollableList;
use crate::demos::selectable_text::SelectableText;
use crate::demos::sticky::Sticky;
use crate::demos::tab_form::TabForm;
use crate::demos::transition_box::TransitionBox;
use crate::demos::ua_chrome::UaChrome;
use crate::demos::whitespace_modes::WhitespaceModes;

/// Every demo the showcase knows about, in stable order. Order
/// here determines display order in the sidebar (within each
/// category — the shell groups by `Category` for display).
pub const DEMOS: &[&dyn Demo] = &[
    &HelloWorld,
    &FlexRow,
    &BorderCollapse,
    &ScrollableList,
    &Hover,
    &CounterButton,
    &MutationObserverDemo,
    &Sticky,
    &SelectableText,
    &TabForm,
    &ParseAndRender,
    &DomApi,
    &UaChrome,
    &TransitionBox,
    &IntervalCounter,
    &RafProgress,
    &InlineFormatting,
    &Headings,
    &WhitespaceModes,
];

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

    #[test]
    fn every_demo_stylesheet_uses_only_class_scoped_selectors() {
        // M3 architectural pin: per-demo stylesheets are pre-pushed
        // onto the App at startup. Every demo's sheet stays loaded
        // even when its subtree isn't mounted. This works only as
        // long as each demo's CSS uses class-scoped selectors —
        // if a demo ships a bare `div { ... }` rule, that rule
        // bleeds onto every OTHER demo's subtree.
        //
        // The convention is enforced here, not in the cascade: a
        // selector contains at least one `.` class selector
        // somewhere in its text. Combinator forms like `.x > .y`,
        // `.x .y`, `.x.y`, `.x:hover` all pass; bare `div`,
        // `:root`, `body` all fail.
        //
        // If a future demo legitimately needs to restyle a chrome
        // rule (e.g. override `<main>`'s padding), the right fix
        // is a substrate push/pop API for event handlers — see
        // SHOWCASE-EVT-1 in TECH_DEBT.md — not weakening this
        // invariant.
        // Parse the demo's OWN css text via `rdom_css::parse` —
        // bypasses the UA-merge that `Demo::stylesheet()` performs
        // via `rdom_css::from_css`. We only want to check the
        // author rules, not the UA defaults bundled in.
        for demo in DEMOS {
            let css = demo.source().css;
            let parsed = rdom_css::parse(css);
            for rule in parsed.stylesheet.rules() {
                assert!(
                    rule.source_text.contains('.'),
                    "demo {:?} stylesheet rule {:?} has no class selector — \
                     every per-demo rule must be class-scoped (see \
                     SHOWCASE-EVT-1 in TECH_DEBT.md). If you need to \
                     restyle a chrome element, file a substrate gap, \
                     don't ship an unscoped selector.",
                    demo.slug(),
                    rule.source_text,
                );
            }
        }
    }
}
