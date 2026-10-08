//! `TuiExt` defaults, size tripwire and equality.

use super::*;
use crate::layout::{Padding, Size};
use crate::style::Color;

#[test]
fn defaults_are_sensible() {
    let ext = TuiExt::new();
    // Geometry now lives in `inline_style` (empty by default) — the
    // raw `ext` geometry fields were removed in EXT-LAYOUT-SETTERS-1.
    assert!(ext.inline_style.is_none());
    assert_eq!(ext.scroll_x, 0);
    assert_eq!(ext.scroll_y, 0);
    assert!(ext.inline_style.is_none());
    assert!(ext.before_content.is_none());
    assert!(ext.after_content.is_none());
    assert_eq!(ext.layout, LayoutRect::default());
    // Cascade cache starts empty; cascade populates it on first pass.
    assert!(ext.computed.is_none());
    assert!(ext.computed_before.is_none());
    assert!(ext.computed_after.is_none());
    // Dirty flags default false — a brand-new `TuiExt` has no cascade
    // work yet; the subtree root gets marked dirty when first attached.
    assert!(!ext.style_dirty);
    assert!(!ext.layout_dirty);
}

/// `P7G-CLONE-RESET-1`: cloning is the element's cloning steps —
/// the author inputs and the captured defaults are copied, the
/// per-activation runtime state starts fresh.
#[test]
fn clone_copies_author_inputs_and_resets_runtime_state() {
    let mut dom: crate::TuiDom = crate::TuiDom::new();
    let opener = dom.create_element("button");
    let mut ext = TuiExt {
        inline_style: Some(Box::new(
            TuiStyle::new()
                .fg(Color::Rgb(255, 0, 0))
                .width(Size::Fixed(80))
                .padding(Padding::all(2)),
        )),
        before_content: Some("▾ ".into()),
        after_content: Some(" ←".into()),
        default_value: Some("hi".into()),
        default_checked: Some(true),
        default_selected: Some(false),
        scroll_x: 3,
        scroll_y: 9,
        scroll_state: Some(Box::default()),
        caret_reveal_pending: true,
        caret_blink_off: true,
        style_dirty: true,
        dialog_return_focus: Some(opener),
        ..Default::default()
    };
    {
        let form = ext.form_state.get_mut();
        form.custom_validity = "taken".into();
        form.value_user_edited = true;
        form.firing_submission_events = true;
    }
    let cloned = ext.clone();
    assert_eq!(cloned.inline_style, ext.inline_style);
    assert_eq!(cloned.before_content, ext.before_content);
    assert_eq!(cloned.after_content, ext.after_content);
    assert_eq!(cloned.default_value, ext.default_value);
    assert_eq!(cloned.default_checked, ext.default_checked);
    assert_eq!(cloned.default_selected, ext.default_selected);
    let fresh = TuiExt {
        inline_style: ext.inline_style.clone(),
        before_content: ext.before_content.clone(),
        after_content: ext.after_content.clone(),
        default_value: ext.default_value.clone(),
        default_checked: ext.default_checked,
        default_selected: ext.default_selected,
        ..Default::default()
    };
    assert_eq!(cloned, fresh, "everything else is at its default");
    assert!(cloned.form_state.get().is_none(), "no form state carried");
    assert!(cloned.scroll_state.is_none(), "no scroll in flight");
}

/// Every element pays for `TuiExt`, so growth should be a decision:
/// state only some elements use (form controls, scroll containers,
/// editing hosts, selects) lives behind a lazily created box
/// (`P7G-FORM-STATE-BOX-1`: 4496 → 4344 bytes on 64-bit targets),
/// and so do the pseudo-element styles (`Rc`), the transition
/// overrides and the inline style (`PERF-TUIEXT-SIZE-1`: 4344 →
/// 432). Raise the bound deliberately, with the reason in the commit:
/// 440 for the recorded matches a vars-only restyle reuses
/// (`C1G-PROPERTY-RESTYLE`, one `Rc`); 448 for the floated `::before` /
/// `::after` boxes a box's formatting context run places
/// (`C8G-PSEUDO-ATOMS`, one thin `Box`); 456 for a list item's `::marker`
/// style (`C10-LIST-ITEM`, one `Rc`); 464 for a `<details>` element's
/// `::details-content` style (`C10-DETAILS-CONTENT`, one `Rc` — the
/// positioned pseudo-elements' boxes, `::first-line` / `::first-letter`
/// and the highlight styles of C10-PSEUDO-UNIFY, -FIRST and -HIGHLIGHT
/// fit in what the two `PseudoLayout`s freed).
#[test]
fn tui_ext_size_tripwire() {
    const MAX: usize = 464;
    let size = std::mem::size_of::<TuiExt>();
    let computed = std::mem::size_of::<ComputedStyle>();
    let inline = std::mem::size_of::<TuiStyle>();
    let presentation = std::mem::size_of::<PresentationStyle>();
    eprintln!(
        "TuiExt {size} B; ComputedStyle {computed} B, TuiStyle {inline} B, \
         PresentationStyle {presentation} B"
    );
    assert!(size <= MAX, "size_of::<TuiExt>() = {size}, bound {MAX}");
}

#[test]
fn partial_eq_works() {
    let a = TuiExt {
        inline_style: Some(Box::new(TuiStyle::new().width(Size::Fixed(10)))),
        ..Default::default()
    };
    let b = TuiExt {
        inline_style: Some(Box::new(TuiStyle::new().width(Size::Fixed(10)))),
        ..Default::default()
    };
    let c = TuiExt {
        inline_style: Some(Box::new(TuiStyle::new().width(Size::Fixed(11)))),
        ..Default::default()
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}
