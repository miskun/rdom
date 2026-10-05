//! The UA sheet's rule count and coverage pins.

use crate::color::named;
use crate::color::system::TEXT_MUTED;
use crate::stylesheet::{Rule, RuleOrigin, Stylesheet};
use crate::{TuiColor, Value};

/// Pin the UA rule count. Any accidental addition or deletion
/// breaks this test and requires a deliberate update.
#[test]
fn ua_total_rule_count() {
    let s = Stylesheet::new();
    let ua: Vec<_> = s
        .rules()
        .iter()
        .filter(|r| r.origin == RuleOrigin::UserAgent)
        .collect();
    // Tripwire — any accidental rule addition or deletion breaks
    // this test and requires a deliberate update. Comma-list
    // selectors expand to one Rule per selector at insertion, so
    // the count can exceed the number of tuples in `ua_defaults`.
    // 142: the focus-affordance vocabulary (FOCUS-VOCAB-1) replaced the
    // generic `:focus` tint + its `canvas:focus` / `[role=tree]:focus`
    // opt-out hacks (net -3) with a 4-selector control allowlist
    // (`button/summary/a/area:focus`, +4) and the `:focus::scrollbar-thumb`
    // accent (+1) — alongside the retained 3-selector
    // `input/textarea/select:focus` !important tint. P7-FOCUS-VISIBLE-1
    // re-keyed both control lists on `:focus-visible` (net 0).
    // 144: the button-family `<input>` label (P6G-INPUT-BUTTON-LABEL-1)
    // adds the value-less `[type=submit]` / `[type=reset]` default
    // labels (+2); `button::before` split from the input `::before`
    // list is net 0.
    // 146: `::placeholder` (P7-PLACEHOLDER-PSEUDO-1) takes the
    // placeholder color in a 2-selector rule (+2).
    // 150: `<input type=image>` (P7G-INPUT-IMAGE-1) joins the button
    // family's box and `::after` lists (+2) and gets its `alt` /
    // default `::before` labels (+2).
    // 161: `box-sizing: border-box` for the HTML rendering section's
    // form controls plus `meter` / `progress` (C5-BOX-SIZING), an
    // 11-selector rule (+11).
    assert_eq!(ua.len(), 161);
    let disabled = ua
        .iter()
        .find(|r| r.source_text == ":disabled")
        .expect(":disabled rule must exist");
    assert_eq!(
        disabled.style.fg,
        Some(Value::Specified(TuiColor::Literal(TEXT_MUTED))),
    );
}

/// ARIA tree pattern (`<ul role=tree>` / `<li role=treeitem>` /
/// `<ul role=group>`) UA defaults. Structural display + indent,
/// disclosure chevron, collapse, and row-highlight rules.
#[test]
fn ua_tree_aria_rules() {
    use crate::layout::Display;

    let s = Stylesheet::new();
    let ua: std::collections::HashMap<String, &Rule> = s
        .rules()
        .iter()
        .filter(|r| r.origin == RuleOrigin::UserAgent)
        .map(|r| (r.source_text.clone(), r))
        .collect();

    for sel in [
        "[role=tree]",
        "[role=group]",
        "[role=treeitem]",
        "[role=treeitem]::before",
        "[role=treeitem][aria-expanded=false] > [role=group]",
        // `[role=tree]:focus` reset removed by FOCUS-VOCAB-1 — the tint is
        // now scoped to controls, so the tree container never gets a fill.
        "[role=tree]:focus [data-rdom-active]",
        "[role=treeitem][aria-selected=true]",
    ] {
        assert!(ua.contains_key(sel), "missing UA rule for `{sel}`");
    }

    // Container + group + item are block-level.
    for sel in ["[role=tree]", "[role=group]", "[role=treeitem]"] {
        assert_eq!(
            ua[sel].style.display,
            Some(Value::Specified(Display::Block)),
            "`{sel}` should be display:block",
        );
    }

    // Group reserves the connector gutter; treeitem reserves the
    // chevron cell — both via left padding.
    for sel in ["[role=group]", "[role=treeitem]"] {
        assert!(
            ua[sel].style.padding.is_some(),
            "`{sel}` must declare indent/chevron padding",
        );
    }

    // Collapsed branch hides its child group.
    assert_eq!(
        ua["[role=treeitem][aria-expanded=false] > [role=group]"]
            .style
            .display,
        Some(Value::Specified(Display::None)),
    );

    // Guide color is sourced from the treeitem's border color.
    assert!(
        ua["[role=treeitem]"].style.border_color.left.is_some(),
        "treeitem must declare a guide (border) color",
    );
}

/// Spot-check a handful of the Tier 1 UA rules to catch
/// accidental regressions (e.g. if a future refactor drops
/// entries from the defaults vec).
#[test]
fn ua_rules_cover_tier_1_semantics() {
    use crate::layout::Display;

    let s = Stylesheet::new();
    let ua: std::collections::HashMap<String, &Rule> = s
        .rules()
        .iter()
        .filter(|r| r.origin == RuleOrigin::UserAgent)
        .map(|r| (r.source_text.clone(), r))
        .collect();

    // A few coverage checks across each category.
    for tag in [
        "section",
        "article",
        "aside",
        "header",
        "footer",
        "main",
        "nav",
        "blockquote",
        "figure",
        "figcaption",
        "hr",
        "ul",
        "ol",
        "li",
        "dl",
        "dt",
        "dd",
        "details",
        "summary",
        "dialog",
        "form",
        "abbr",
        "cite",
        "mark",
        "kbd",
        "samp",
        "var",
        "dfn",
        "del",
        "ins",
        "sub",
        "sup",
        "small",
        "s",
        "q",
        "h4",
        "h5",
        "h6",
    ] {
        assert!(ua.contains_key(tag), "missing UA rule for <{tag}>");
    }

    // mark: yellow bg + black fg.
    let mark = ua["mark"];
    assert_eq!(
        mark.style.bg,
        Some(Value::Specified(TuiColor::Literal(named::YELLOW)))
    );
    assert_eq!(
        mark.style.fg,
        Some(Value::Specified(TuiColor::Literal(named::BLACK)))
    );

    // Headings h1-h6 all bold + block.
    for h in ["h1", "h2", "h3", "h4", "h5", "h6"] {
        let r = ua[h];
        assert_eq!(r.style.bold, Some(Value::Specified(true)));
        assert_eq!(r.style.display, Some(Value::Specified(Display::Block)));
    }

    // Inline + italic for semantic emphasis tags.
    for t in ["em", "i", "cite", "dfn", "var"] {
        let r = ua[t];
        assert_eq!(
            r.style.display,
            Some(Value::Specified(Display::Inline)),
            "<{t}> must be inline"
        );
        assert_eq!(
            r.style.italic,
            Some(Value::Specified(true)),
            "<{t}> must be italic"
        );
    }

    // Muted text: small / abbr render with the UA muted-text
    // color (`TEXT_MUTED` — #7F868B).
    for t in ["small", "abbr"] {
        let r = ua[t];
        assert_eq!(
            r.style.fg,
            Some(Value::Specified(TuiColor::Literal(TEXT_MUTED))),
            "<{t}> must use TEXT_MUTED fg"
        );
    }
    // <del> and <s> render with line-through, not dim.
    for t in ["del", "s"] {
        let r = ua[t];
        assert_eq!(
            r.style.text_decoration,
            Some(Value::Specified(crate::layout::TextDecoration::LineThrough)),
            "<{t}> must use text-decoration: line-through"
        );
    }
}

/// Buttons are not text — their label is a click affordance, not
/// selectable prose. The base button rules (`<button>` and the
/// three button-family input types) must declare
/// `user-select: none` so drag-selection skips them (a deliberate
/// UA declaration — see DIVERGENCES). The `:disabled` rule already
/// covers the disabled case; this pins the enabled case.
#[test]
fn ua_buttons_are_unselectable() {
    use crate::layout::UserSelect;

    let s = Stylesheet::new();
    // A selector can head several UA rules (the `box-sizing` rule lists
    // the buttons too), so look for the one that declares it.
    for sel in [
        "button",
        "input[type=button]",
        "input[type=submit]",
        "input[type=image]",
        "input[type=reset]",
    ] {
        let declared = s
            .rules()
            .iter()
            .filter(|r| r.origin == RuleOrigin::UserAgent && r.source_text == sel)
            .any(|r| r.style.user_select == Some(Value::Specified(UserSelect::None)));
        assert!(
            declared,
            "`{sel}` must declare user-select: none: a button label is not prose"
        );
    }
}

/// Toggles and range declare no `user-select` (P6G-TOGGLE-USER-SELECT-REVERT-1):
/// browsers' UA sheets do not, and a click on them reaches them without it
/// because the text-selection drag takes no pointer capture.
#[test]
fn ua_toggles_and_range_leave_user_select_alone() {
    let s = Stylesheet::new();
    let mut seen = std::collections::HashSet::new();
    for r in s
        .rules()
        .iter()
        .filter(|r| r.origin == RuleOrigin::UserAgent)
        .filter(|r| {
            [
                "input[type=checkbox]",
                "input[type=radio]",
                "input[type=range]",
            ]
            .contains(&r.source_text.as_str())
        })
    {
        seen.insert(r.source_text.as_str());
        assert_eq!(
            r.style.user_select, None,
            "`{}` must not declare user-select",
            r.source_text
        );
    }
    assert_eq!(seen.len(), 3, "all three UA rules present");
}

/// CSS Color 4 §6.2: the system colors are the UA sheet's own colors —
/// a link is `LinkText`, a mark `Mark` on `MarkText`, the selection
/// `Highlight` / `HighlightText`, a selected option `SelectedItem`.
#[test]
fn system_colors_match_the_ua_chrome() {
    use crate::TuiStyle;
    use crate::color::system::SystemColor;
    let ua: std::collections::HashMap<&str, TuiStyle> =
        super::user_agent_defaults().into_iter().collect();
    let fg = |sel: &str| ua[sel].fg.clone();
    let bg = |sel: &str| ua[sel].bg.clone();
    let lit = |s: SystemColor| Some(Value::Specified(TuiColor::Literal(s.color())));
    assert_eq!(fg("a[href]"), lit(SystemColor::LinkText));
    assert_eq!(fg("button"), lit(SystemColor::ButtonText));
    assert_eq!(bg("mark"), lit(SystemColor::Mark));
    assert_eq!(fg("mark"), lit(SystemColor::MarkText));
    assert_eq!(bg("*::selection"), lit(SystemColor::Highlight));
    assert_eq!(fg("*::selection"), lit(SystemColor::HighlightText));
    assert_eq!(bg("option[selected]"), lit(SystemColor::SelectedItem));
    assert_eq!(fg("option[selected]"), lit(SystemColor::SelectedItemText));
    assert_eq!(fg(":disabled"), lit(SystemColor::GrayText));
}
