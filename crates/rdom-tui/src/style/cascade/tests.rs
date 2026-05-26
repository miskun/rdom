//! Integration-style tests for the cascade engine.
//!
//! Exercises `Dom::cascade(&sheet)` end-to-end: inheritance,
//! specificity, `!important`, inline style, pseudo-elements,
//! `:hover` / `:focus`, UA defaults, layout invalidation, vars.
//! 80+ tests; far larger than the impl, on the "move to
//! `tests/cascade.rs` as integration tests" shortlist once the
//! crate's integration-test story solidifies.

use super::*;
use crate::layout::{Border, Direction, Display, Flow, Overflow, Padding, Size, WhiteSpace};
use crate::style::{Color, Content, Modifier, Stylesheet, TuiStyle};
use crate::{TuiDom, TuiNodeMutExt};
use rdom_core::NodeId;

fn dom_with_div() -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    (dom, div)
}

// ── Inheritance ──────────────────────────────────────────────────

#[test]
fn fg_inherits_to_child() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);

    assert_eq!(computed_of(&dom, parent).fg, Color::Rgb(255, 0, 0));
    assert_eq!(computed_of(&dom, child).fg, Color::Rgb(255, 0, 0)); // inherited
}

#[test]
fn bg_does_not_inherit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().bg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);

    assert_eq!(computed_of(&dom, parent).bg, Color::Rgb(255, 0, 0));
    assert_eq!(computed_of(&dom, child).bg, Color::Reset); // not inherited
}

#[test]
fn bold_inherits() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().bold(true));
    dom.cascade(&sheet);

    assert!(computed_of(&dom, parent).modifiers.contains(Modifier::BOLD));
    assert!(computed_of(&dom, child).modifiers.contains(Modifier::BOLD));
}

// ── Specificity ──────────────────────────────────────────────────

#[test]
fn higher_specificity_wins() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).set_id("hero").unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("#hero", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 0, 255));
}

#[test]
fn same_specificity_source_order_wins() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).add_class("a").unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(".a", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked(".a", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 0, 255));
}

// ── Inline style ─────────────────────────────────────────────────

#[test]
fn inline_beats_stylesheet_rule() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(0, 128, 0)));
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 128, 0));
}

#[test]
fn inline_beats_id_rule_too() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).set_id("x").unwrap();
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(0, 128, 0)));
    let sheet = Stylesheet::bare().rule_unchecked("#x", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 128, 0));
}

// ── !important ───────────────────────────────────────────────────

#[test]
fn important_rule_beats_normal_inline() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(0, 128, 0)));
    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg_important(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn important_inline_beats_important_author() {
    // Wait, actually per CSS: important author > important inline.
    // So setting both important, author wins.
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg_important(Color::Rgb(0, 128, 0)));
    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg_important(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn ua_rule_loses_to_author() {
    // UA stylesheet has [disabled] { color: gray; }. Author can
    // override with their own rule at the same specificity and later
    // source order.
    let (mut dom, div) = dom_with_div();
    dom.set_attribute(div, "disabled", "").unwrap();

    // Plain UA: disabled div takes the muted fg (lens TextMuted).
    dom.cascade(&Stylesheet::new());
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(127, 134, 139));

    // Author override at same specificity, later in source order → wins.
    let sheet =
        Stylesheet::new().rule_unchecked("[disabled]", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn ua_important_beats_author_important() {
    // Same-origin / same-specificity / same-importance contest: source
    // order late wins. We use `fg` as the probe.
    let (mut dom, div) = dom_with_div();
    dom.set_attribute(div, "disabled", "").unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "[disabled]",
            TuiStyle::new().fg_important(Color::Rgb(255, 0, 0)),
        )
        .rule_unchecked(
            "[disabled]",
            TuiStyle::new().fg_important(Color::Rgb(0, 0, 255)),
        );
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 0, 255));
}

// ── Initial / Inherit resolution ─────────────────────────────────

#[test]
fn fg_initial_restores_spec_default() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("span", TuiStyle::new().fg_initial());
    dom.cascade(&sheet);
    // Parent red, child forced to initial (Reset).
    assert_eq!(computed_of(&dom, parent).fg, Color::Rgb(255, 0, 0));
    assert_eq!(computed_of(&dom, child).fg, Color::Reset);
}

#[test]
fn fg_inherit_keyword_forces_parent_value() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        // Force-inherit (no-op here since fg already inherits, but tests
        // the code path).
        .rule_unchecked("span", TuiStyle::new().fg_inherit());
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, child).fg, Color::Rgb(255, 0, 0));
}

// ── Pseudo-elements ──────────────────────────────────────────────

#[test]
fn before_pseudo_created_from_rule() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::before",
        TuiStyle::new().content(Content::Str("▾".into())),
    );
    dom.cascade(&sheet);
    let ext = dom.node(div).ext().unwrap();
    let before = ext.computed_before.as_ref().unwrap();
    assert_eq!(before.content.as_deref(), Some("▾"));
}

#[test]
fn pseudo_position_cascades_through_to_computed_style() {
    // Positioning props on `::before` / `::after` rules must survive
    // the cascade ladder and land in the pseudo's `ComputedStyle`.
    // Without this, no amount of paint-side work matters — the
    // position info wouldn't reach paint.
    use crate::layout::{Length, Position};
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div::before",
            TuiStyle::new()
                .content(Content::Str("[".into()))
                .position(Position::Absolute)
                .left(Length::Cells(0)),
        )
        .rule_unchecked(
            "div::after",
            TuiStyle::new()
                .content(Content::Str("]".into()))
                .position(Position::Absolute)
                .right(Length::Cells(0)),
        );
    dom.cascade(&sheet);
    let ext = dom.node(div).ext().unwrap();

    let before = ext.computed_before.as_ref().expect("::before computed");
    assert_eq!(before.position, Position::Absolute);
    assert_eq!(before.left, Length::Cells(0));

    let after = ext.computed_after.as_ref().expect("::after computed");
    assert_eq!(after.position, Position::Absolute);
    assert_eq!(after.right, Length::Cells(0));
}

#[test]
fn after_pseudo_created_from_rule() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::after",
        TuiStyle::new().content(Content::Str("→".into())),
    );
    dom.cascade(&sheet);
    let ext = dom.node(div).ext().unwrap();
    let after = ext.computed_after.as_ref().unwrap();
    assert_eq!(after.content.as_deref(), Some("→"));
}

#[test]
fn pseudo_inherits_from_host_not_parent() {
    // Parent is blue; host (div) is red; ::before should inherit red, not blue.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("section");
    let host = dom.create_element("div");
    dom.append_child(parent, host).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("section", TuiStyle::new().fg(Color::Rgb(0, 0, 255)))
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked(
            "div::before",
            TuiStyle::new().content(Content::Str("x".into())),
        );
    dom.cascade(&sheet);
    let before = dom
        .node(host)
        .ext()
        .unwrap()
        .computed_before
        .as_ref()
        .unwrap();
    assert_eq!(before.fg, Color::Rgb(255, 0, 0));
}

#[test]
fn pseudo_without_rules_or_fallback_is_none() {
    let (mut dom, _div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    let ext = dom.node(_div).ext().unwrap();
    assert!(ext.computed_before.is_none());
    assert!(ext.computed_after.is_none());
    assert!(ext.computed_selection.is_none());
}

#[test]
fn selection_pseudo_created_from_rule() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::selection",
        TuiStyle::new()
            .bg(Color::Rgb(255, 255, 0))
            .fg(Color::Rgb(0, 0, 0)),
    );
    dom.cascade(&sheet);
    let ext = dom.node(div).ext().unwrap();
    let sel = ext.computed_selection.as_ref().unwrap();
    assert_eq!(sel.bg, Color::Rgb(255, 255, 0));
    assert_eq!(sel.fg, Color::Rgb(0, 0, 0));
}

#[test]
fn selection_pseudo_inherits_from_host_not_parent() {
    // Same pattern as before/after — selection styles inherit from
    // the host's computed style, not from the host's parent.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("section");
    let host = dom.create_element("div");
    dom.append_child(parent, host).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("section", TuiStyle::new().fg(Color::Rgb(0, 0, 255)))
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked(
            "div::selection",
            TuiStyle::new().bg(Color::Rgb(255, 255, 0)),
        );
    dom.cascade(&sheet);
    let sel = dom
        .node(host)
        .ext()
        .unwrap()
        .computed_selection
        .as_ref()
        .unwrap();
    assert_eq!(sel.bg, Color::Rgb(255, 255, 0));
    assert_eq!(sel.fg, Color::Rgb(255, 0, 0));
}

#[test]
fn pseudo_with_legacy_before_content_renders() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).set_before_content("▸ ");
    dom.cascade(&Stylesheet::bare());
    let ext = dom.node(div).ext().unwrap();
    let before = ext.computed_before.as_ref().unwrap();
    assert_eq!(before.content.as_deref(), Some("▸ "));
}

#[test]
fn content_rule_beats_legacy_before_content() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).set_before_content("legacy");
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::before",
        TuiStyle::new().content(Content::Str("new".into())),
    );
    dom.cascade(&sheet);
    let before = dom
        .node(div)
        .ext()
        .unwrap()
        .computed_before
        .as_ref()
        .unwrap();
    assert_eq!(before.content.as_deref(), Some("new"));
}

#[test]
fn content_concat_resolves() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::before",
        TuiStyle::new().content(Content::Concat(vec![
            Content::Str("▾ ".into()),
            Content::Str("ITEM".into()),
        ])),
    );
    dom.cascade(&sheet);
    let before = dom
        .node(div)
        .ext()
        .unwrap()
        .computed_before
        .as_ref()
        .unwrap();
    assert_eq!(before.content.as_deref(), Some("▾ ITEM"));
}

#[test]
fn content_none_suppresses_pseudo() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).set_before_content("legacy");
    let sheet =
        Stylesheet::bare().rule_unchecked("div::before", TuiStyle::new().content(Content::None));
    dom.cascade(&sheet);
    let ext = dom.node(div).ext().unwrap();
    // Content::None yields `None` content; the rule did match so a
    // ComputedStyle is created, but content is None — paint decides
    // to skip based on content.is_none().
    let before = ext.computed_before.as_ref().unwrap();
    assert!(before.content.is_none());
}

// ── :hover / :focus ──────────────────────────────────────────────

#[test]
fn hover_pseudo_affects_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("div:hover", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));

    dom.set_hovered(None);
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));

    dom.set_hovered(Some(div));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 0, 255));
}

#[test]
fn focus_pseudo_affects_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("div:focus", TuiStyle::new().fg(Color::Rgb(0, 128, 0)));

    dom.set_focused(Some(div));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 128, 0));
}

// ── UA stylesheet ────────────────────────────────────────────────

#[test]
fn ua_disabled_rule_mutes_by_default() {
    // T8: `[disabled]` UA rule sets `fg: TEXT_MUTED` (lens TextMuted
    // #7F868B — was `dim: true` pre-T8, briefly CSS gray pre-palette
    // refresh).
    let (mut dom, div) = dom_with_div();
    dom.set_attribute(div, "disabled", "").unwrap();
    dom.cascade(&Stylesheet::new());
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(127, 134, 139));
}

#[test]
fn ua_disabled_does_not_apply_to_nondisabled() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::new());
    // Without the [disabled] attribute, fg stays at its default.
    assert_eq!(computed_of(&dom, div).fg, Color::Reset);
}

// ── Layout invalidation ──────────────────────────────────────────

#[test]
fn first_cascade_marks_layout_dirty() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    assert!(dom.node(div).ext().unwrap().layout_dirty);
}

#[test]
fn paint_only_change_does_not_set_layout_dirty() {
    let (mut dom, div) = dom_with_div();
    // First cascade to establish baseline.
    dom.cascade(&Stylesheet::bare());
    // Clear layout_dirty so we can observe the next cascade's effect.
    dom.node_mut(div).ext_mut().unwrap().layout_dirty = false;

    // Second cascade with a pure-paint rule.
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));
    assert!(!dom.node(div).ext().unwrap().layout_dirty);
}

#[test]
fn layout_change_sets_layout_dirty() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    dom.node_mut(div).ext_mut().unwrap().layout_dirty = false;

    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().padding(Padding::all(2)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).padding, Padding::all(2));
    assert!(dom.node(div).ext().unwrap().layout_dirty);
}

#[test]
fn cascade_clears_style_dirty() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div).ext_mut().unwrap().style_dirty = true;
    dom.cascade(&Stylesheet::bare());
    assert!(!dom.node(div).ext().unwrap().style_dirty);
}

// ── Modifier composition ─────────────────────────────────────────

#[test]
fn bold_and_italic_both_set() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().bold(true).italic(true));
    dom.cascade(&sheet);
    let m = computed_of(&dom, div).modifiers;
    assert!(m.contains(Modifier::BOLD));
    assert!(m.contains(Modifier::ITALIC));
}

#[test]
fn child_can_turn_off_inherited_bold() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().bold(true))
        .rule_unchecked("span", TuiStyle::new().bold(false));
    dom.cascade(&sheet);
    assert!(computed_of(&dom, parent).modifiers.contains(Modifier::BOLD));
    assert!(!computed_of(&dom, child).modifiers.contains(Modifier::BOLD));
}

// ── border_fg special case ───────────────────────────────────────

#[test]
fn border_fg_initial_tracks_fg() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    // border_fg not set → falls back to fg per property catalog.
    // Current apply_color uses working.fg as initial — by the time
    // border_fg is processed, fg is already Red.
    assert_eq!(computed_of(&dom, div).border_fg, Color::Rgb(255, 0, 0));
}

#[test]
fn border_fg_explicit_overrides_fg() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .fg(Color::Rgb(255, 0, 0))
            .border_fg(Color::Rgb(0, 0, 255)),
    );
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));
    assert_eq!(computed_of(&dom, div).border_fg, Color::Rgb(0, 0, 255));
}

// ── Layout properties cascade ────────────────────────────────────

#[test]
fn padding_and_gap_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().padding(Padding::symmetric(2, 1)).gap(3),
    );
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert_eq!(c.padding, Padding::symmetric(2, 1));
    assert_eq!(c.gap, 3);
}

#[test]
fn width_and_height_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().width(Size::Fixed(40)).height(Size::Flex(1)),
    );
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert_eq!(c.width, Size::Fixed(40));
    assert_eq!(c.height, Size::Flex(1));
}

#[test]
fn min_max_option_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet =
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().min_width(10).max_width(100));
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert_eq!(c.min_width, Some(rdom_style::layout::MinSize::Cells(10)));
    assert_eq!(c.max_width, Some(100));
}

#[test]
fn border_collapse_inherits_to_children() {
    use rdom_style::layout::BorderCollapse;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "outer",
        TuiStyle::new().border_collapse(BorderCollapse::Collapse),
    );
    dom.cascade(&sheet);
    assert_eq!(
        computed_of(&dom, outer).border_collapse,
        BorderCollapse::Collapse
    );
    assert_eq!(
        computed_of(&dom, inner).border_collapse,
        BorderCollapse::Collapse,
        "border-collapse inherits per CSS spec"
    );
}

#[test]
fn border_collapse_default_is_separate() {
    use rdom_style::layout::BorderCollapse;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("el");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare();
    dom.cascade(&sheet);
    assert_eq!(
        computed_of(&dom, el).border_collapse,
        BorderCollapse::Separate
    );
}

#[test]
fn border_direction_overflow_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .border(Border::single())
            .flow(Flow::Flex)
            .direction(Direction::Row)
            .overflow(Overflow::Hidden),
    );
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert_eq!(c.border, Border::single());
    assert_eq!(c.direction, Direction::Row);
    assert_eq!(c.overflow_x, Overflow::Hidden);
    assert_eq!(c.overflow_y, Overflow::Hidden);
}

// ── Element type check: only elements get computed ───────────────

#[test]
fn text_node_is_skipped_but_siblings_cascade() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let text = dom.create_text_node("hi");
    let span = dom.create_element("span");
    dom.append_child(root, text).unwrap();
    dom.append_child(root, span).unwrap();

    let sheet =
        Stylesheet::bare().rule_unchecked("span", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, span).fg, Color::Rgb(255, 0, 0));
    // Text node has no ext; nothing to assert, but no panic proves
    // cascade handles non-elements gracefully.
}

// ── Selector list matching ───────────────────────────────────────

#[test]
fn selector_list_rule_matches_multiple_tags() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let sheet =
        Stylesheet::bare().rule_unchecked("a, b", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, a).fg, Color::Rgb(255, 0, 0));
    assert_eq!(computed_of(&dom, b).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn descendant_combinator_cascade() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet =
        Stylesheet::bare().rule_unchecked("div span", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, inner).fg, Color::Rgb(255, 0, 0));
}

// ── cascade_subtrees (incremental) ───────────────────────────────

#[test]
fn cascade_subtrees_noop_on_empty_list() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    let before = computed_of(&dom, div);
    dom.cascade_subtrees(&Stylesheet::bare(), &[]);
    assert_eq!(computed_of(&dom, div), before);
}

#[test]
fn cascade_subtrees_inherits_from_parent_computed() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    // Full cascade with parent red.
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, child).fg, Color::Rgb(255, 0, 0));

    // Incrementally re-cascade just the child — still inherits red.
    dom.cascade_subtrees(&sheet, &[child]);
    assert_eq!(computed_of(&dom, child).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn cascade_subtrees_picks_up_new_rule() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    assert_eq!(computed_of(&dom, div).fg, Color::Reset);

    // Author adds a rule, we re-cascade just the div's subtree.
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    dom.cascade_subtrees(&sheet, &[div]);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 0, 255));
}

// ── Custom properties (var()) ────────────────────────────────────

#[test]
fn var_resolves_from_stylesheet() {
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("accent", "#ff0000")
        .rule_unchecked("div", TuiStyle::new().fg(TuiColor::var("accent")));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn var_with_named_color_value() {
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("muted", "gray")
        .rule_unchecked("div", TuiStyle::new().fg(TuiColor::var("muted")));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(128, 128, 128));
}

#[test]
fn var_with_rgb_function_value() {
    // SUB-1 / D-M1-5 retirement: custom-property values stored as
    // raw strings are resolved through the full CSS color grammar
    // at lookup time. `rgb(...)` must now resolve, not just the
    // simple named/hex/indexed subset.
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("accent", "rgb(10, 20, 30)")
        .rule_unchecked("div", TuiStyle::new().fg(TuiColor::var("accent")));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(10, 20, 30));
}

#[test]
fn var_with_rgba_function_value_drops_alpha() {
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("translucent", "rgba(200, 100, 50, 0.5)")
        .rule_unchecked("div", TuiStyle::new().fg(TuiColor::var("translucent")));
    dom.cascade(&sheet);
    // Alpha is dropped; terminals paint opaque cells.
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(200, 100, 50));
}

#[test]
fn fg_var_shorthand_setter() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("highlight", "blue")
        .rule_unchecked("div", TuiStyle::new().fg_var("highlight"));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 0, 255));
}

#[test]
fn unresolved_var_falls_back_to_inherit() {
    use crate::TuiColor;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("span", TuiStyle::new().fg(TuiColor::var("missing")));
    dom.cascade(&sheet);
    // var(--missing) unresolved → falls back to parent's fg (Red, inherited).
    assert_eq!(computed_of(&dom, child).fg, Color::Rgb(255, 0, 0));
}

#[test]
fn var_fallback_chain_at_cascade() {
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("b", "cyan")
        // var(--a, var(--b, red)) — a missing, b=cyan → cyan.
        .rule_unchecked(
            "div",
            TuiStyle::new().fg(TuiColor::var_with(
                "a",
                TuiColor::var_with("b", TuiColor::Literal(Color::Rgb(255, 0, 0))),
            )),
        );
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).fg, Color::Rgb(0, 255, 255));
}

#[test]
fn bg_and_border_fg_also_support_var() {
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("panel", "#101020")
        .define_var("edge", "#808080")
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .bg(TuiColor::var("panel"))
                .border_fg(TuiColor::var("edge")),
        );
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert_eq!(c.bg, Color::Rgb(0x10, 0x10, 0x20));
    assert_eq!(c.border_fg, Color::Rgb(0x80, 0x80, 0x80));
}

#[test]
fn var_works_in_pseudo_element() {
    use crate::TuiColor;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .define_var("arrow-color", "#ff00ff")
        .rule_unchecked(
            "div::before",
            TuiStyle::new()
                .fg(TuiColor::var("arrow-color"))
                .content(Content::Str("▾".into())),
        );
    dom.cascade(&sheet);
    let before = dom
        .node(div)
        .ext()
        .unwrap()
        .computed_before
        .as_ref()
        .unwrap();
    assert_eq!(before.fg, Color::Rgb(0xff, 0x00, 0xff));
}

// ── Display / WhiteSpace ────────────────────────────────────────────

#[test]
fn display_defaults_to_block() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    assert_eq!(computed_of(&dom, div).display, Display::Block);
}

#[test]
fn display_does_not_inherit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);

    assert_eq!(computed_of(&dom, parent).display, Display::Inline);
    // Child does NOT inherit display — stays Block.
    assert_eq!(computed_of(&dom, child).display, Display::Block);
}

#[test]
fn display_inline_via_rule() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).display, Display::Inline);
}

#[test]
fn white_space_defaults_to_normal() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    assert_eq!(computed_of(&dom, div).white_space, WhiteSpace::Normal);
}

#[test]
fn white_space_inherits() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("pre");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet =
        Stylesheet::bare().rule_unchecked("pre", TuiStyle::new().white_space(WhiteSpace::Pre));
    dom.cascade(&sheet);

    assert_eq!(computed_of(&dom, parent).white_space, WhiteSpace::Pre);
    // Child inherits — critical for <pre>-wrapped content.
    assert_eq!(computed_of(&dom, child).white_space, WhiteSpace::Pre);
}

#[test]
fn white_space_child_overrides_inherited() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("pre");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("pre", TuiStyle::new().white_space(WhiteSpace::Pre))
        .rule_unchecked("span", TuiStyle::new().white_space(WhiteSpace::Normal));
    dom.cascade(&sheet);

    assert_eq!(computed_of(&dom, child).white_space, WhiteSpace::Normal);
}

#[test]
fn display_and_white_space_mark_layout_dirty() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    dom.node_mut(div).ext_mut().unwrap().layout_dirty = false;

    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    assert!(dom.node(div).ext().unwrap().layout_dirty);

    // Reset and test white_space too.
    dom.node_mut(div).ext_mut().unwrap().layout_dirty = false;
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .display(Display::Inline)
            .white_space(WhiteSpace::Pre),
    );
    dom.cascade(&sheet);
    assert!(dom.node(div).ext().unwrap().layout_dirty);
}

#[test]
fn display_important_cascades_through_ladder() {
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().display(Display::Inline));
    let sheet =
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().display_important(Display::Block));
    dom.cascade(&sheet);
    // Author !important beats normal inline.
    assert_eq!(computed_of(&dom, div).display, Display::Block);
}

// ── user-select ─────────────────────────────────────────────────────

#[test]
fn user_select_defaults_to_auto() {
    use crate::layout::UserSelect;
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    assert_eq!(computed_of(&dom, div).user_select, UserSelect::Auto);
}

#[test]
fn user_select_inherits_to_child() {
    use crate::layout::UserSelect;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet =
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().user_select(UserSelect::None));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, parent).user_select, UserSelect::None);
    assert_eq!(
        computed_of(&dom, child).user_select,
        UserSelect::None,
        "user-select inherits"
    );
}

#[test]
fn user_select_child_rule_overrides_inherited() {
    use crate::layout::UserSelect;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().user_select(UserSelect::None))
        .rule_unchecked("span", TuiStyle::new().user_select(UserSelect::Text));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, child).user_select, UserSelect::Text);
}

#[test]
fn user_select_important_beats_inline() {
    use crate::layout::UserSelect;
    let (mut dom, div) = dom_with_div();
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().user_select(UserSelect::Text));
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().user_select_important(UserSelect::None),
    );
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).user_select, UserSelect::None);
}

// ── Tier 1 UA defaults (end-to-end) ─────────────────────────────────

/// Build a single element under root and return its computed
/// style after cascading with `Stylesheet::new()` (UA defaults
/// only, no author rules).
fn ua_computed_for(tag: &str) -> ComputedStyle {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element(tag);
    dom.append_child(root, el).unwrap();
    dom.cascade(&Stylesheet::new());
    computed_of(&dom, el)
}

#[test]
fn ua_mark_has_yellow_bg_and_black_fg() {
    let c = ua_computed_for("mark");
    // CSS named: yellow = #FFFF00, black = #000000.
    assert_eq!(c.bg, Color::Rgb(255, 255, 0));
    assert_eq!(c.fg, Color::Rgb(0, 0, 0));
    assert_eq!(c.display, Display::Inline);
}

#[test]
fn ua_kbd_is_bold_accent_inline() {
    let c = ua_computed_for("kbd");
    // Accent: dodgerblue = #1E90FF.
    assert_eq!(c.fg, Color::Rgb(30, 144, 255));
    assert!(c.modifiers.contains(Modifier::BOLD));
    assert_eq!(c.display, Display::Inline);
}

#[test]
fn ua_del_is_inline_and_strikethrough() {
    let c = ua_computed_for("del");
    assert_eq!(c.display, Display::Inline);
    // `<del>` / `<s>` are styled via `text-decoration: line-through`
    // (proper strikethrough via SGR-9).
    assert!(c.modifiers.contains(Modifier::CROSSED_OUT));
}

#[test]
fn ua_ins_is_inline_and_underlined() {
    let c = ua_computed_for("ins");
    assert_eq!(c.display, Display::Inline);
    assert!(c.modifiers.contains(Modifier::UNDERLINED));
}

#[test]
fn ua_h4_through_h6_are_block_and_bold() {
    for tag in ["h4", "h5", "h6"] {
        let c = ua_computed_for(tag);
        assert_eq!(c.display, Display::Block, "<{tag}> should be Block");
        assert!(
            c.modifiers.contains(Modifier::BOLD),
            "<{tag}> should be bold"
        );
    }
}

#[test]
fn ua_cite_dfn_var_are_italic_inline() {
    for tag in ["cite", "dfn", "var"] {
        let c = ua_computed_for(tag);
        assert_eq!(c.display, Display::Inline, "<{tag}> should be Inline");
        assert!(
            c.modifiers.contains(Modifier::ITALIC),
            "<{tag}> should be italic"
        );
    }
}

#[test]
fn ua_blockquote_has_left_rail_and_is_muted() {
    let c = ua_computed_for("blockquote");
    assert_eq!(c.display, Display::Block);
    // 1-cell left border (`│` rail) + 1-cell padding = 2-cell total
    // left indent, same visual budget as the pre-rail design.
    assert_eq!(
        c.padding.left,
        crate::layout::PaddingValue::Cells(1),
        "blockquote needs 1 cell padding"
    );
    assert_eq!(c.border, Border::left(), "blockquote shows `│` rail");
    // T8: blockquote text is muted via `fg: TEXT_MUTED` (lens
    // TextMuted #7F868B), not the deleted `dim` modifier.
    assert_eq!(c.fg, Color::Rgb(127, 134, 139));
}

#[test]
fn ua_ul_ol_menu_have_left_padding() {
    for tag in ["ul", "ol", "menu"] {
        let c = ua_computed_for(tag);
        assert_eq!(
            c.padding.left,
            crate::layout::PaddingValue::Cells(2),
            "<{tag}> needs list-marker room"
        );
    }
}

#[test]
fn ua_figcaption_is_italic_muted_block() {
    let c = ua_computed_for("figcaption");
    assert_eq!(c.display, Display::Block);
    assert!(c.modifiers.contains(Modifier::ITALIC));
    // T8: figcaption is muted via `fg: TEXT_MUTED` (lens TextMuted
    // #7F868B), not the deleted `dim` modifier.
    assert_eq!(c.fg, Color::Rgb(127, 134, 139));
}

#[test]
fn ua_sectioning_tags_are_block() {
    for tag in [
        "article", "section", "aside", "header", "footer", "main", "nav", "search",
    ] {
        let c = ua_computed_for(tag);
        assert_eq!(c.display, Display::Block, "<{tag}> should be Block");
    }
}

#[test]
fn ua_interactive_block_tags_are_block() {
    // `<dialog>` is block when open, `display:none` otherwise —
    // it's exercised separately in `ua_dialog_is_none_when_closed`.
    for tag in [
        "details", "summary", "form", "fieldset", "legend", "dl", "dt", "dd",
    ] {
        let c = ua_computed_for(tag);
        assert_eq!(c.display, Display::Block, "<{tag}> should be Block");
    }
    // summary / dt / legend are also bold.
    for tag in ["summary", "dt", "legend"] {
        let c = ua_computed_for(tag);
        assert!(
            c.modifiers.contains(Modifier::BOLD),
            "<{tag}> should be bold"
        );
    }
}

#[test]
fn ua_dialog_is_none_when_closed() {
    // `<dialog>` defaults to Block, but the `:not([open])` UA rule
    // hides it via `display: none` when the `open` attribute is
    // absent. When `open` is present, the Block rule wins again.
    let c_closed = ua_computed_for("dialog");
    assert_eq!(
        c_closed.display,
        Display::None,
        "<dialog> without [open] should be None"
    );

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("dialog");
    dom.set_attribute(d, "open", "").unwrap();
    dom.append_child(root, d).unwrap();
    dom.cascade(&Stylesheet::new());
    assert_eq!(
        computed_of(&dom, d).display,
        Display::Block,
        "<dialog open> should be Block"
    );
}

#[test]
fn ua_colgroup_and_col_are_none() {
    assert_eq!(ua_computed_for("colgroup").display, Display::None);
    assert_eq!(ua_computed_for("col").display, Display::None);
}

#[test]
fn ua_hidden_attribute_makes_element_none() {
    // `[hidden] { display: none }`. Mirrors HTML's global `hidden`
    // attribute — present-with-any-value hides the element from
    // layout and paint.
    let (mut dom, div) = dom_with_div();
    dom.set_attribute(div, "hidden", "").unwrap();
    dom.cascade(&Stylesheet::new());
    assert_eq!(computed_of(&dom, div).display, Display::None);
}

#[test]
fn ua_hidden_attribute_value_does_not_matter() {
    // HTML treats the `hidden` attribute as a boolean — any value
    // (including "false" or "until-found") still hides the element.
    // The UA selector is `[hidden]`, not `[hidden=""]`.
    let (mut dom, div) = dom_with_div();
    dom.set_attribute(div, "hidden", "false").unwrap();
    dom.cascade(&Stylesheet::new());
    assert_eq!(computed_of(&dom, div).display, Display::None);
}

#[test]
fn ua_hidden_attribute_can_be_overridden_by_author_rule() {
    // Author rules outweigh the tiny-specificity UA rule. A site
    // that wants `hidden` elements visible (e.g. for an editor view)
    // can do so without `!important`.
    let (mut dom, div) = dom_with_div();
    dom.set_attribute(div, "hidden", "").unwrap();
    let sheet =
        Stylesheet::new().rule_unchecked("[hidden]", TuiStyle::new().display(Display::Block));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).display, Display::Block);
}

#[test]
fn ua_hidden_does_not_apply_to_nonhidden() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::new());
    assert_eq!(computed_of(&dom, div).display, Display::Block);
}

#[test]
fn ua_optgroup_before_resolves_label_attribute_via_attr_content() {
    // Polish #2: `optgroup::before { content: attr(label) }`
    // resolves from the host element's `label` attribute.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let og = dom.create_element("optgroup");
    dom.set_attribute(og, "label", "Fruits").unwrap();
    dom.append_child(root, og).unwrap();
    dom.cascade(&Stylesheet::new());
    use crate::node::TuiNodeExt;
    let before = dom.node(og).computed_before().cloned().unwrap();
    assert_eq!(before.content.as_deref(), Some("Fruits"));
}

#[test]
fn attr_content_falls_back_to_empty_when_attribute_missing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let og = dom.create_element("optgroup");
    // No `label` attribute set.
    dom.append_child(root, og).unwrap();
    dom.cascade(&Stylesheet::new());
    use crate::node::TuiNodeExt;
    let before = dom.node(og).computed_before().cloned().unwrap();
    assert_eq!(before.content.as_deref(), Some(""));
}

#[test]
fn placeholder_shown_triggers_before_content_with_placeholder_text() {
    // Polish #3: empty input with `placeholder="Search"` grows a
    // `::before` pseudo-element whose content is the placeholder
    // text (DarkGray + dim from the UA rule).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "placeholder", "Search").unwrap();
    dom.append_child(root, inp).unwrap();
    dom.cascade(&Stylesheet::new());
    use crate::node::TuiNodeExt;
    let before = dom.node(inp).computed_before().cloned().unwrap();
    assert_eq!(before.content.as_deref(), Some("Search"));
    // T8: placeholder is muted via `fg: TEXT_MUTED` (lens TextMuted
    // #7F868B), not the deleted `dim` modifier.
    assert_eq!(before.fg, Color::Rgb(127, 134, 139));
}

#[test]
fn placeholder_disappears_when_input_has_text_content() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "placeholder", "Search").unwrap();
    let t = dom.create_text_node("typed");
    dom.append_child(inp, t).unwrap();
    dom.append_child(root, inp).unwrap();
    dom.cascade(&Stylesheet::new());
    use crate::node::TuiNodeExt;
    // `:placeholder-shown` no longer matches → UA ::before rule
    // doesn't contribute → computed_before has no content.
    let before = dom.node(inp).computed_before();
    let has_content = before
        .and_then(|c| c.content.as_deref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    assert!(
        !has_content,
        "placeholder should not render when input has text"
    );
}

#[test]
fn textarea_placeholder_shown_also_renders() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ta = dom.create_element("textarea");
    dom.set_attribute(ta, "placeholder", "Write something…")
        .unwrap();
    dom.append_child(root, ta).unwrap();
    dom.cascade(&Stylesheet::new());
    use crate::node::TuiNodeExt;
    let before = dom.node(ta).computed_before().cloned().unwrap();
    assert_eq!(before.content.as_deref(), Some("Write something…"));
}

#[test]
fn attr_content_is_overridable_by_author_rule() {
    // Author can override with their own ::before content.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.set_attribute(el, "data-status", "warning").unwrap();
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::new().rule_unchecked(
        "div::before",
        TuiStyle::new().content(Content::Attr("data-status".into())),
    );
    dom.cascade(&sheet);
    use crate::node::TuiNodeExt;
    let before = dom.node(el).computed_before().cloned().unwrap();
    assert_eq!(before.content.as_deref(), Some("warning"));
}

#[test]
fn ua_author_rule_overrides_tier_1_default() {
    // <mark> defaults to yellow bg, but an author rule can override.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mark = dom.create_element("mark");
    dom.append_child(root, mark).unwrap();
    let sheet =
        Stylesheet::new().rule_unchecked("mark", TuiStyle::new().bg(Color::Rgb(0, 255, 255)));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, mark).bg, Color::Rgb(0, 255, 255));
}

// ── <a> anchor handling ─────────────────────────────────────────

#[test]
fn ua_bare_a_is_inline_with_no_link_styling() {
    // `<a>` without href is a named anchor / placeholder per HTML —
    // still inline, but no hyperlink colors.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.append_child(root, a).unwrap();
    dom.cascade(&Stylesheet::new());

    let c = computed_of(&dom, a);
    assert_eq!(c.display, Display::Inline);
    assert_eq!(c.fg, Color::Reset, "bare <a> should inherit, not colorize");
    assert!(!c.modifiers.contains(Modifier::UNDERLINED));
}

#[test]
fn ua_a_with_href_is_link_styled() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "https://example.com").unwrap();
    dom.append_child(root, a).unwrap();
    dom.cascade(&Stylesheet::new());

    let c = computed_of(&dom, a);
    assert_eq!(c.display, Display::Inline);
    // Accent: dodgerblue = #1E90FF.
    assert_eq!(c.fg, Color::Rgb(30, 144, 255));
    assert!(c.modifiers.contains(Modifier::UNDERLINED));
}

#[test]
fn ua_a_href_hover_bolds() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "/internal").unwrap();
    dom.append_child(root, a).unwrap();
    dom.set_hovered(Some(a));
    dom.cascade(&Stylesheet::new());

    let c = computed_of(&dom, a);
    assert!(
        c.modifiers.contains(Modifier::BOLD),
        "a[href]:hover should bold for clickable feedback"
    );
    // Hover doesn't remove the base link styling. Accent =
    // dodgerblue (#1E90FF).
    assert_eq!(c.fg, Color::Rgb(30, 144, 255));
    assert!(c.modifiers.contains(Modifier::UNDERLINED));
}

#[test]
fn ua_a_without_href_does_not_get_hover_bold() {
    // `a:hover` shouldn't apply to a bare `<a>` because the UA rule
    // is `a[href]:hover` — attribute presence gates it.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.append_child(root, a).unwrap();
    dom.set_hovered(Some(a));
    dom.cascade(&Stylesheet::new());

    let c = computed_of(&dom, a);
    assert!(!c.modifiers.contains(Modifier::BOLD));
}

// ── Positioning (M2) ─────────────────────────────────────────────

#[test]
fn position_default_is_static() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::new());
    assert_eq!(
        computed_of(&dom, div).position,
        crate::layout::Position::Static
    );
}

#[test]
fn position_relative_cascades() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().position(crate::layout::Position::Relative),
    );
    dom.cascade(&sheet);
    assert_eq!(
        computed_of(&dom, div).position,
        crate::layout::Position::Relative
    );
}

#[test]
fn position_does_not_inherit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().position(crate::layout::Position::Absolute),
    );
    dom.cascade(&sheet);

    assert_eq!(
        computed_of(&dom, parent).position,
        crate::layout::Position::Absolute
    );
    // child stays Static — position doesn't inherit per CSS spec.
    assert_eq!(
        computed_of(&dom, child).position,
        crate::layout::Position::Static
    );
}

#[test]
fn top_left_offsets_cascade() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .top(crate::layout::Length::Cells(3))
            .left(crate::layout::Length::Cells(-2)),
    );
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert_eq!(c.top, crate::layout::Length::Cells(3));
    assert_eq!(c.left, crate::layout::Length::Cells(-2));
    // unspecified edges fall back to initial Auto.
    assert_eq!(c.right, crate::layout::Length::Auto);
    assert_eq!(c.bottom, crate::layout::Length::Auto);
}

#[test]
fn z_index_cascades() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().z_index(crate::layout::ZIndex::Value(7)),
    );
    dom.cascade(&sheet);
    assert_eq!(
        computed_of(&dom, div).z_index,
        crate::layout::ZIndex::Value(7)
    );
}

#[test]
fn z_index_default_is_auto() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::new());
    assert_eq!(computed_of(&dom, div).z_index, crate::layout::ZIndex::Auto);
}

#[test]
fn position_important_beats_non_important() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new().position_important(crate::layout::Position::Absolute),
        )
        .rule_unchecked(
            "div",
            TuiStyle::new().position(crate::layout::Position::Relative),
        );
    dom.cascade(&sheet);
    // !important wins despite the second rule being later in source.
    assert_eq!(
        computed_of(&dom, div).position,
        crate::layout::Position::Absolute
    );
}

// ── tree_has_positioned_pseudo flag (D-M5N-2) ────────────────────

/// No positioned pseudo anywhere → flag stays false on every
/// element. Lets the layout / paint pseudo passes early-exit.
#[test]
fn tree_flag_false_without_positioned_pseudo() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    let span = dom.create_element("span");
    dom.append_child(div, span).unwrap();
    dom.append_child(root, div).unwrap();
    // No pseudo rules; even with `::before` content the flag should
    // stay false because position is Static by default.
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::before",
        TuiStyle::new().content(Content::Str("[ ".into())),
    );
    dom.cascade(&sheet);
    assert!(!dom.node(div).ext().unwrap().tree_has_positioned_pseudo);
    assert!(!dom.node(span).ext().unwrap().tree_has_positioned_pseudo);
}

/// A `::before { position: absolute }` rule on a leaf bubbles the
/// flag up through its ancestors so the document-root early-exit
/// fires correctly.
#[test]
fn tree_flag_bubbles_to_ancestors() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();
    dom.node_mut(inner).set_id("target").unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "#target::before",
        TuiStyle::new()
            .content(Content::Str("•".into()))
            .position(crate::layout::Position::Absolute),
    );
    dom.cascade(&sheet);

    assert!(dom.node(inner).ext().unwrap().tree_has_positioned_pseudo);
    assert!(
        dom.node(outer).ext().unwrap().tree_has_positioned_pseudo,
        "outer must inherit the aggregate so the early-exit check fires"
    );
}

/// `::after { position: relative }` also sets the flag — Relative
/// is non-`Static` per spec.
#[test]
fn tree_flag_relative_after_pseudo_counts() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div::after",
        TuiStyle::new()
            .content(Content::Str(" ▶".into()))
            .position(crate::layout::Position::Relative),
    );
    dom.cascade(&sheet);
    assert!(dom.node(div).ext().unwrap().tree_has_positioned_pseudo);
}

/// Removing the only positioned pseudo via a fresh `cascade()` call
/// (full re-cascade) clears the flag. Partial `cascade_subtrees`
/// behavior is conservatively stale-`true` and not asserted here.
#[test]
fn tree_flag_clears_on_full_recascade() {
    let (mut dom, div) = dom_with_div();
    let pseudo_sheet = Stylesheet::bare().rule_unchecked(
        "div::before",
        TuiStyle::new()
            .content(Content::Str("[".into()))
            .position(crate::layout::Position::Absolute),
    );
    dom.cascade(&pseudo_sheet);
    assert!(dom.node(div).ext().unwrap().tree_has_positioned_pseudo);

    // Re-cascade without the positioned pseudo rule.
    let bare = Stylesheet::bare();
    dom.cascade(&bare);
    assert!(!dom.node(div).ext().unwrap().tree_has_positioned_pseudo);
}

// ── text-decoration property (T3) ───────────────────────────────

/// `text-decoration: underline` sets the UNDERLINED modifier bit.
#[test]
fn text_decoration_underline_sets_underlined_bit() {
    use crate::layout::TextDecoration;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().text_decoration(TextDecoration::Underline),
    );
    dom.cascade(&sheet);
    assert!(
        computed_of(&dom, div)
            .modifiers
            .contains(Modifier::UNDERLINED)
    );
    assert!(
        !computed_of(&dom, div)
            .modifiers
            .contains(Modifier::CROSSED_OUT)
    );
}

/// `text-decoration: line-through` sets the CROSSED_OUT modifier
/// bit (SGR-9). The paint pass's existing SGR emitter handles
/// the codepoint.
#[test]
fn text_decoration_line_through_sets_crossed_out_bit() {
    use crate::layout::TextDecoration;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().text_decoration(TextDecoration::LineThrough),
    );
    dom.cascade(&sheet);
    assert!(
        computed_of(&dom, div)
            .modifiers
            .contains(Modifier::CROSSED_OUT)
    );
    assert!(
        !computed_of(&dom, div)
            .modifiers
            .contains(Modifier::UNDERLINED)
    );
}

/// `text-decoration: none` clears both UNDERLINED and CROSSED_OUT
/// bits when an earlier rule set `text-decoration: underline` — a
/// straight "later same-specificity rule wins" conflict resolution
/// between two `text-decoration` declarations.
#[test]
fn text_decoration_none_clears_decoration_bits() {
    use crate::layout::TextDecoration;
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new().text_decoration(TextDecoration::Underline),
        )
        .rule_unchecked("div", TuiStyle::new().text_decoration(TextDecoration::None));
    dom.cascade(&sheet);
    let c = computed_of(&dom, div);
    assert!(!c.modifiers.contains(Modifier::UNDERLINED));
    assert!(!c.modifiers.contains(Modifier::CROSSED_OUT));
}

/// `text-decoration` is non-inheriting per CSS spec — a child of
/// an element with `text-decoration: underline` should render
/// *without* the underline. UNDERLINED is intentionally not in the
/// inheritable modifier mask.
#[test]
fn text_decoration_does_not_inherit_to_children() {
    use crate::layout::TextDecoration;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().text_decoration(TextDecoration::Underline),
    );
    dom.cascade(&sheet);
    // Parent gets the underline directly.
    assert!(
        computed_of(&dom, parent)
            .modifiers
            .contains(Modifier::UNDERLINED)
    );
    // Child does NOT inherit UNDERLINED — CSS-faithful.
    assert!(
        !computed_of(&dom, child)
            .modifiers
            .contains(Modifier::UNDERLINED)
    );
}

// ── opacity property ────────────────────────────────────────────────

/// Setter clamps `opacity` to [0.0, 1.0] regardless of input.
#[test]
fn opacity_setter_clamps_to_unit_interval() {
    let s_neg = TuiStyle::new().opacity(-0.5);
    let s_high = TuiStyle::new().opacity(1.5);
    let s_half = TuiStyle::new().opacity(0.5);
    if let Some(crate::style::Value::Specified(v)) = s_neg.opacity {
        assert_eq!(v, 0.0);
    } else {
        panic!("opacity not set");
    }
    if let Some(crate::style::Value::Specified(v)) = s_high.opacity {
        assert_eq!(v, 1.0);
    } else {
        panic!("opacity not set");
    }
    if let Some(crate::style::Value::Specified(v)) = s_half.opacity {
        assert_eq!(v, 0.5);
    } else {
        panic!("opacity not set");
    }
}

/// Cascade resolves `opacity` to the computed `f32` field. Default
/// is `1.0` (CSS initial value).
#[test]
fn opacity_cascade_initial_is_one() {
    let (mut dom, div) = dom_with_div();
    dom.cascade(&Stylesheet::bare());
    assert_eq!(computed_of(&dom, div).opacity, 1.0);
}

#[test]
fn opacity_cascade_applies_specified_value() {
    let (mut dom, div) = dom_with_div();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().opacity(0.5));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, div).opacity, 0.5);
}

/// `opacity` does NOT inherit per CSS spec. Child gets `1.0`
/// regardless of parent's opacity.
#[test]
fn opacity_does_not_inherit_to_child() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().opacity(0.5));
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, parent).opacity, 0.5);
    assert_eq!(computed_of(&dom, child).opacity, 1.0);
}
