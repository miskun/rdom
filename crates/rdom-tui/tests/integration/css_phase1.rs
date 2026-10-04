//! `C1G-INTEGRATION` — CSS-COMPLETE Phase 1 end to end: one sheet parsed
//! by `rdom_css`, cascaded by `rdom-tui`, combining cascade layers,
//! custom properties and `var()`, nesting, `@scope`, `@import` through a
//! closure loader, `revert-layer`, `all: revert` and an escaped class
//! selector. Each assertion names the spec text that fixes the value.

use rdom_tui::style::cascade::computed_of;
use rdom_tui::{CascadeExt, Color, ComputedStyle, Display, Modifier, NodeId, TuiDom};

const RED: Color = Color::Rgb(255, 0, 0);

/// The main sheet, through `from_css_strict` (no warning allowed).
const MAIN: &str = r"
@layer base, theme;
@layer theme { :root { --p: 1 2; --c: red } }
@layer base { :root { --c: blue } }
.card {
  padding: var(--p);
  & > p { color: var(--c) }
  @layer base { border-color: var(--c) }
}
@layer theme { .card.x { border-color: yellow } }
.card.x { border-color: revert-layer }
.card.y { border-color: green }
@scope (.card) to (.slot) { p { font-weight: bold } }
.card p.r { all: revert }
.\31 0 { width: 7 }
";

/// A second sheet whose `@import` the loader below resolves.
const WITH_IMPORT: &str = "@import 'tokens.css' layer(theme); .imported { width: var(--w) }";

struct Doc {
    dom: TuiDom,
    card: NodeId,
    x: NodeId,
    y: NodeId,
    child_p: NodeId,
    inner_p: NodeId,
    reverted_p: NodeId,
    ten: NodeId,
    imported: NodeId,
}

fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let id = dom.create_element(tag);
    if !class.is_empty() {
        dom.set_attribute(id, "class", class).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// ```html
/// <div class="card">
///   <p></p>
///   <div class="slot"><p></p></div>
///   <p class="r"></p>
///   <span class="10"></span>
///   <div class="imported"></div>
/// </div>
/// <div class="card x"></div>
/// <div class="card y"></div>
/// ```
fn doc() -> Doc {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let card = el(&mut dom, root, "div", "card");
    let child_p = el(&mut dom, card, "p", "");
    let slot = el(&mut dom, card, "div", "slot");
    let inner_p = el(&mut dom, slot, "p", "");
    let reverted_p = el(&mut dom, card, "p", "r");
    let ten = el(&mut dom, card, "span", "10");
    let imported = el(&mut dom, card, "div", "imported");
    let x = el(&mut dom, root, "div", "card x");
    let y = el(&mut dom, root, "div", "card y");
    Doc {
        dom,
        card,
        x,
        y,
        child_p,
        inner_p,
        reverted_p,
        ten,
        imported,
    }
}

/// The computed style of the only `div` under `css` (UA sheet included).
fn literal(css: &str) -> ComputedStyle {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = el(&mut dom, root, "div", "");
    dom.cascade(&rdom_css::from_css_strict(css).unwrap());
    computed_of(&dom, div)
}

#[test]
fn phase_one_features_cascade_together() {
    let main = rdom_css::from_css_strict(MAIN).expect("the main sheet parses cleanly");
    let loader = |url: &str| match url {
        "tokens.css" => Ok(":root { --w: 4 }".to_string()),
        other => Err(format!("no {other}")),
    };
    let imported = rdom_css::parse_with_loader(WITH_IMPORT, &loader);
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    assert_eq!(imported.stylesheet.imports().len(), 1);

    // The expected values are distinct, so no assertion below can pass
    // by a property keeping its initial value.
    let initial = literal("");
    assert_ne!(literal("div { padding: 1 2 }").padding, initial.padding);
    assert_ne!(literal("div { width: 7 }").width, initial.width);
    assert_ne!(literal("div { width: 4 }").width, initial.width);
    let yellow = literal("div { border-color: yellow }").border_fg;
    let green = literal("div { border-color: green }").border_fg;
    assert!(yellow != green && yellow != RED && green != RED);

    let mut d = doc();
    d.dom.cascade_all(&[&main, &imported.stylesheet]);
    let get = |id| computed_of(&d.dom, id);

    // CSS Variables 1 §3: `var(--p)` substitutes the token sequence
    // `1 2` and the `padding` shorthand parses it.
    assert_eq!(get(d.card).padding, literal("div { padding: 1 2 }").padding);

    // Cascade 5 §6.4: `theme` is declared after `base`, so its `:root`
    // `--c: red` beats `base`'s `blue`; Nesting 1 §2: `& > p` is
    // `.card > p`.
    assert_eq!(get(d.child_p).fg, RED);

    // Nesting 1 §3.2: declarations directly in a nested `@layer base`
    // apply to `.card` in that layer, reading the same `--c`.
    assert_eq!(get(d.card).border_fg, RED);

    // Cascade 5 §7.4: `revert-layer` in an unlayered rule rolls back to
    // the cascade of the layers alone, where `theme`'s yellow beats
    // `base`'s red.
    assert_eq!(
        get(d.x).border_fg,
        literal("div { border-color: yellow }").border_fg
    );
    // Unlayered normal declarations beat every layer (§6.4).
    assert_eq!(
        get(d.y).border_fg,
        literal("div { border-color: green }").border_fg
    );

    // Cascade 6 §2.5: `p` inside `.card` is in scope; the one under the
    // `.slot` limit is not.
    assert!(get(d.child_p).modifiers.contains(Modifier::BOLD));
    assert!(!get(d.inner_p).modifiers.contains(Modifier::BOLD));

    // Cascade 4 §3.2 / §7.3: `all: revert` in an author rule rolls every
    // property back to the user-agent origin — `p` is `display: block`
    // there, has no color (so it inherits `.card`'s) and is not bold.
    let reverted = get(d.reverted_p);
    assert_eq!(reverted.display, Display::Block);
    assert_eq!(reverted.fg, get(d.card).fg);
    assert!(!reverted.modifiers.contains(Modifier::BOLD));

    // CSS Syntax 3 §4.3.7: `.\31 0` is the class `10`.
    assert_eq!(get(d.ten).width, literal("div { width: 7 }").width);

    // Cascade 5 §3: the imported `:root` reaches `var(--w)`.
    assert_eq!(get(d.imported).width, literal("div { width: 4 }").width);
}
