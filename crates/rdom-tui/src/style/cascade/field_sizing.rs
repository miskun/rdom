//! `field-sizing: content` (CSS UI 4 §7.2): a text field sized by its
//! content. A browser's text field has an intrinsic size from its `size`
//! (an `<input>`) or `cols` / `rows` (a `<textarea>`) that `content`
//! replaces with the content's; rdom's UA sheet gives the fields fixed
//! sizes instead (`input { width: 20 }`, `textarea { width: 20; height:
//! 4 }`), so under `content` a field whose `width` (`height`) no author or
//! inline declaration sets is sized to its content — `max-content` on the
//! inline axis, `auto` (its rows) on the block axis of a `<textarea>`.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{FieldSizing, IntrinsicSize, Size};
use crate::style::{ComputedStyle, Rule, RuleOrigin, TuiStyle};

/// Size `working` (the computed style of element `id`, cascaded from
/// `rules` and its `inline` style) to its content, when it is a text
/// field with `field-sizing: content`.
pub(super) fn finalize(
    working: &mut ComputedStyle,
    dom: &Dom<TuiExt>,
    id: NodeId,
    rules: &[&Rule],
    inline: Option<&TuiStyle>,
) {
    if working.ui.field_sizing != FieldSizing::Content {
        return;
    }
    let textarea = dom.node(id).tag_name() == Some("textarea");
    if !(textarea || crate::node::is_text_input(dom, id)) {
        return;
    }
    // Declared by the page, not the UA: an author rule or the `style`
    // attribute.
    let authored = |names: &[&str]| {
        rules
            .iter()
            .filter(|r| r.origin != RuleOrigin::UserAgent)
            .map(|r| &r.style)
            .chain(inline)
            .any(|s| declares(s, names))
    };
    if !authored(&["width", "inline-size"]) {
        working.width = Size::Intrinsic(IntrinsicSize::MaxContent);
    }
    if textarea && !authored(&["height", "block-size"]) {
        working.height = Size::Auto;
    }
}

/// Whether `style` declares one of `names` — a value set, or one kept
/// for substitution or for its flow-relative mapping.
fn declares(style: &TuiStyle, names: &[&str]) -> bool {
    names
        .iter()
        .any(|n| rdom_style::property_dispatch::serialize(n, style).is_some())
        || style
            .pending
            .iter()
            .any(|d| names.contains(&d.name.as_str()))
}
