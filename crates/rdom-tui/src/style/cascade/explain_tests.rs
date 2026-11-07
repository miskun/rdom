//! The explanation is the cascade (CAPTURE phase 3 review B1): on a page
//! that exercises every input of an element's ladder — rules of several
//! specificities, `!important`, cascade layers, inline styles, `revert`,
//! presentational hints, `var()` from an ancestor and from a `<details>`
//! element's `::details-content` slot, an `rtl` element with a
//! flow-relative `var()` declaration — `explain_element` reports, for every
//! element, exactly the declaration blocks the cascade's settled ladder
//! run applied, in order, at the same importance.

use std::rc::Rc;

use rdom_core::NodeType;

use super::explain::explain_element;
use super::ladder::record;
use super::registered::PropertyRegistry;
use crate::style::CascadeExt;
use crate::{Stylesheet, TuiDom};

const MARKUP: &str = r#"<div class="page" style="--gap: 1">
  <p class="a" id="x" style="color: green">text</p>
  <ol start="3"><li class="item">one</li><li value="7">two</li></ol>
  <details open class="d"><summary>s</summary><p class="slotted">in</p></details>
  <div class="rtl" dir="rtl"><span class="logical">r</span></div>
  <p class="reverted">back</p>
</div>"#;

const CSS: &str = "
@layer base { p { color: red; padding-left: 3; } }
p { color: blue; }
#x { color: purple !important; }
.a { font-weight: bold; }
details::details-content { --slot: 2; }
.slotted { padding-left: var(--slot); margin-top: var(--gap); }
.rtl { direction: rtl; --side: 2; }
.logical { margin-inline-start: var(--side); padding-inline-end: 1; }
.reverted { color: orange; }
.reverted { color: revert; }
";

#[test]
fn the_explanation_reports_exactly_what_the_cascade_applied() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    rdom_parser::parse_into(&mut dom, MARKUP, host).unwrap();
    crate::seed_inline_styles(&mut dom);
    let sheet = rdom_css::from_css(CSS);
    record::enable();
    dom.cascade(&sheet);
    let applied = record::take();
    let sheets: Vec<&Stylesheet> = vec![&sheet];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    let mut checked = 0;
    for id in dom.descendants(dom.root()) {
        if dom.node(id).node_type() != NodeType::Element {
            continue;
        }
        let Some(cascade) = applied.get(&id) else {
            continue;
        };
        let explained =
            explain_element(&dom, &sheets, registry.clone(), id).expect("a styled element");
        let got: Vec<(bool, &crate::style::TuiStyle)> =
            explained.iter().map(|a| (a.important, &a.style)).collect();
        let want: Vec<(bool, &crate::style::TuiStyle)> =
            cascade.iter().map(|(i, s)| (*i, s)).collect();
        assert_eq!(
            got,
            want,
            "element {id:?} <{}>: the explanation differs from the cascade",
            dom.node(id).tag_name().unwrap_or_default()
        );
        checked += 1;
    }
    assert!(checked >= 10, "only {checked} elements checked");
}
