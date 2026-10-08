//! C11-LINK-LANG — `:link` / `:any-link` / `:visited` (Selectors 4
//! §8.1–§8.2), `:lang()` (§7.2) and `:dir()` (§7.1) through a sheet, the
//! HTML rendering section's `dir` rules written with `:dir()`, and the
//! restyle when what they read changes.

use rdom_tui::layout::TextDirection;
use rdom_tui::{NodeId, TuiDom, TuiNodeExt};

use super::{BLUE, RED, UNSTYLED, app, app_fg, cascade, el, fg};

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) -> NodeId {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
    t
}

/// Selectors 4 §8.2: `a:link, a:visited` keeps its `:link` half (an
/// unknown `:visited` would drop the rule); `:visited` matches nothing.
#[test]
fn link_rules_style_hyperlinks_only() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a", "");
    dom.set_attribute(a, "href", "#top").unwrap();
    let bare = el(&mut dom, root, "a", "");
    cascade(
        &mut dom,
        "a:link, a:visited { color: red } :visited { color: blue } :any-link { background-color: blue }",
    );
    assert_eq!(fg(&dom, a), RED);
    assert_eq!(fg(&dom, bare), UNSTYLED);
    assert_eq!(dom.node(a).computed().unwrap().bg, BLUE);
}

/// Selectors 4 §7.2: `:lang()` reads the inherited language, and a
/// change of an ancestor's `lang` restyles the descendants.
#[test]
fn lang_rules_follow_the_inherited_language() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = el(&mut dom, root, "div", "");
    dom.set_attribute(outer, "lang", "de-CH").unwrap();
    let inner = el(&mut dom, outer, "p", "");
    let mut app = app(dom, ":lang(de) { color: red } :lang(fr) { color: blue }");
    assert_eq!(app_fg(&app, inner), RED);
    app.dom_mut().set_attribute(outer, "lang", "fr").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, inner), BLUE);
}

/// HTML §15.3.5 (Bidirectional text) writes the `dir` rules with
/// `:dir()`: `[dir]:dir(rtl), bdi:dir(rtl) { direction: rtl }` — so
/// `dir=auto` and `<bdi>` take the direction of their first strong
/// character.
#[test]
fn dir_auto_sets_direction_from_its_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let auto = el(&mut dom, root, "p", "");
    dom.set_attribute(auto, "dir", "auto").unwrap();
    text(&mut dom, auto, "שלום abc");
    let bdi = el(&mut dom, root, "bdi", "");
    text(&mut dom, bdi, "مرحبا");
    let latin = el(&mut dom, root, "p", "");
    dom.set_attribute(latin, "dir", "auto").unwrap();
    text(&mut dom, latin, "abc שלום");
    cascade(&mut dom, "");
    let dir = |id| dom.node(id).computed().unwrap().text_direction;
    assert_eq!(dir(auto), TextDirection::Rtl);
    assert_eq!(dir(bdi), TextDirection::Rtl);
    assert_eq!(dir(latin), TextDirection::Ltr);
}

/// Selectors 4 §7.1: `:dir()` matches the document's directionality, not
/// the CSS `direction` property.
#[test]
fn dir_does_not_follow_the_direction_property() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let styled = el(&mut dom, root, "div", "rtl");
    let child = el(&mut dom, styled, "p", "");
    cascade(
        &mut dom,
        ".rtl { direction: rtl } :dir(rtl) { color: red } p:dir(ltr) { background-color: blue }",
    );
    assert_eq!(fg(&dom, styled), UNSTYLED);
    assert_eq!(fg(&dom, child), UNSTYLED);
    assert_eq!(dom.node(child).computed().unwrap().bg, BLUE);
    assert_eq!(
        dom.node(child).computed().unwrap().text_direction,
        TextDirection::Rtl,
        "`direction` inherits all the same"
    );
}

/// HTML §3.2.6.4: an edit of the text under `dir=auto` can change its
/// first strong character — the next frame restyles the `auto` element.
#[test]
fn editing_text_under_dir_auto_restyles_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let auto = el(&mut dom, root, "p", "");
    dom.set_attribute(auto, "dir", "auto").unwrap();
    let span = el(&mut dom, auto, "span", "");
    let t = text(&mut dom, span, "abc");
    let mut app = app(dom, "p:dir(rtl) { color: red }");
    assert_eq!(app_fg(&app, auto), UNSTYLED);
    app.dom_mut().node_mut(t).set_node_value("שלום").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, auto), RED);
    // A new first child with a strong left-to-right character wins.
    let first = app.dom_mut().create_text_node("x");
    app.dom_mut()
        .insert_before(auto, first, Some(span))
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, auto), UNSTYLED);
}
