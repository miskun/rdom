//! CSS Anchor Positioning 1 (C15-ANCHOR): the anchor names and the
//! default anchor (§2), `anchor()` / `anchor-size()` (§5), `position-area`
//! (§3.1), `anchor-center` (§3.4), the position fallbacks (§4) and
//! `position-visibility` (§5) — read from the computed style, the laid-out
//! rects and the painted cells.

use super::*;
use rdom_tui::Length;

const PAGE: &str = "body { margin: 0 } ";

fn computed(dom: &TuiDom, id: &str) -> rdom_tui::ComputedStyle {
    dom.node(by_id(dom, id))
        .computed()
        .cloned()
        .expect("cascaded")
}

/// §5.1, §5.2: an anchor function in a box that is not absolutely
/// positioned is invalid at computed-value time — its fallback stands in,
/// or the property takes its initial value.
#[test]
fn anchor_functions_outside_abspos_compute_to_their_fallbacks() {
    let mut dom = doc(r#"<body><div id="r"></div><div id="a"></div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} #r {{ position: relative; top: anchor(bottom); left: anchor(right, 3); width: anchor-size(width) }}
             #a {{ position: absolute; top: anchor(bottom) }}"
        ),
        20,
        5,
    );
    let r = computed(&dom, "r");
    assert_eq!(r.top, Length::Auto);
    assert_eq!(r.left.cells(0), Some(3));
    assert_eq!(r.width, rdom_tui::Size::Auto);
    let a = computed(&dom, "a");
    assert!(matches!(&a.top, Length::Calc(e) if e.contains_anchor()));
}

/// §4.1: a fallback naming a `@position-try` rule carries that rule's
/// declarations — the last of the name, by layer order.
#[test]
fn fallbacks_carry_their_position_try_rules() {
    let mut dom = doc(r#"<body><div id="a"></div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} @position-try --x {{ top: 1 }}
             @layer l {{ @position-try --x {{ top: 2 }} }}
             #a {{ position: absolute; position-try-fallbacks: --x, flip-block, --missing }}"
        ),
        20,
        5,
    );
    let a = computed(&dom, "a");
    let f = &a.anchor.position_try_fallbacks;
    assert_eq!(f.len(), 3);
    assert_eq!(
        f[0].declarations().and_then(|d| d.top.clone()),
        Some(rdom_tui::Value::Specified(Length::Cells(1))),
        "unlayered beats layered"
    );
    assert!(f[1].declarations().is_none() && f[2].declarations().is_none());
}
