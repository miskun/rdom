//! User-agent stylesheet — the default rules baked into every
//! `Stylesheet::new()`.
//!
//! Authors override any rule by writing their own at equal-or-higher
//! specificity — the standard browser model. The UA exposes no
//! theming API and no `--rdom-*` custom properties; consumers
//! re-theme by writing override rules in their own stylesheet.
//!
//! This is the single source of truth for the UA. The
//! `ua_total_rule_count` test (`tests.rs`) pins the count — any accidental
//! rule addition or deletion breaks the test and requires a
//! deliberate update.
//!
//! ## Organization
//!
//! Rules are grouped by element family for grep-ability:
//!
//! 1. Form / interaction state (`:disabled`, `[hidden]`)
//! 2. Inline typography (emphasis, weight, semantic inline, code, edits, highlight, abbreviation, etc.)
//! 3. Links (`<a>` / `<a href>` / `:hover`) — inside inline typography
//! 4. Block typography (paragraphs, headings, pre, blockquote, hr, figures)
//! 5. Block structural / sectioning
//! 6. Block interactive (`<details>`, `<summary>`, `<dialog>`, `<form>`, `<fieldset>`, `<legend>`)
//! 7. Tree (`[role=tree]` guides, chevrons, cursor)
//! 8. Form fields (`<input>`, `<textarea>`)
//! 9. Buttons
//! 10. Focus indicator (the FOCUS-VOCAB-1 allowlist + scrollbar accent)
//! 11. Toggle widgets (`<input type=checkbox/radio>` + state glyphs)
//! 12. Select widget (`<select>`, `<option>`, `<optgroup>`)
//! 13. Canvas
//! 14. Tables
//! 15. Gauge widgets (`<progress>`, `<meter>`)
//! 16. Range slider (`<input type=range>`)
//! 17. Lists
//! 18. Scrollbars (`::scrollbar`, `::scrollbar-thumb`)
//! 19. Selection (`::selection`)
//! 20. Document metadata (`<style>`)
//!
//! The groups live in five files, concatenated in this order: `text`
//! (1–5), `interactive` (6–7), `controls` (8–11), `widgets` (12–16)
//! and `decorations` (17–20). The palette they paint with is
//! `color::system`.

mod controls;
mod decorations;
mod interactive;
mod text;
mod widgets;

use crate::TuiStyle;

/// `style` with the CSS declaration `name: value` — for a UA value only
/// CSS spells, an `attr()` substitution (CSS Values 5 §8.7), which the
/// cascade makes per element as for an author rule.
fn css(mut style: TuiStyle, name: &str, value: &str) -> TuiStyle {
    crate::property_dispatch::set(name, value, &mut style).expect("UA declaration parses");
    style
}

/// Return the slice of `(selector, style)` pairs that `Stylesheet::new()`
/// installs as UA defaults. Single source of truth for the user-agent
/// stylesheet.
///
/// Consumed by `Stylesheet::new()` in `stylesheet.rs`; not exposed in
/// `lib.rs` because it's a build detail of the public `Stylesheet::new()`
/// API rather than a separate user-facing entry point.
pub(crate) fn user_agent_defaults() -> Vec<(&'static str, TuiStyle)> {
    [
        text::rules(),
        interactive::rules(),
        controls::rules(),
        widgets::rules(),
        decorations::rules(),
    ]
    .concat()
}

#[cfg(test)]
mod tests;
