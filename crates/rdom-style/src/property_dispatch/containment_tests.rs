//! Dispatch tests for the query-container properties (C14-CONTAINER):
//! `container-type`, `container-name` and the `container` shorthand (CSS
//! Conditional 5 §6.1–§6.3).

use super::*;
use crate::layout::{ContainerName, ContainerSize, ContainerType};
use crate::{TuiStyle, Value};

fn spec<T: Clone>(v: &Option<Value<T>>) -> Option<T> {
    match v {
        Some(Value::Specified(x)) => Some(x.clone()),
        _ => None,
    }
}

fn container_type(css: &str) -> Option<ContainerType> {
    let mut style = TuiStyle::new();
    set("container-type", css, &mut style).ok()?;
    spec(&style.container_type)
}

fn names(n: &ContainerName) -> Vec<&str> {
    n.names().iter().map(|s| &**s).collect()
}

/// §6.1: `normal | [ [ size | inline-size ] || scroll-state ]`.
#[test]
fn container_type_parses() {
    let t = |size, scroll_state| Some(ContainerType::new(size).with_scroll_state(scroll_state));
    assert_eq!(container_type("normal"), t(ContainerSize::Normal, false));
    assert_eq!(container_type("size"), t(ContainerSize::Size, false));
    assert_eq!(
        container_type("INLINE-SIZE"),
        t(ContainerSize::InlineSize, false)
    );
    assert_eq!(
        container_type("scroll-state"),
        t(ContainerSize::Normal, true)
    );
    assert_eq!(
        container_type("scroll-state inline-size"),
        t(ContainerSize::InlineSize, true)
    );
    for bad in [
        "size inline-size",
        "normal size",
        "size size",
        "auto",
        "",
        "1",
    ] {
        assert_eq!(container_type(bad), None, "{bad}");
    }
    let mut style = TuiStyle::new();
    set("container-type", "inline-size scroll-state", &mut style).unwrap();
    assert_eq!(
        serialize("container-type", &style).as_deref(),
        Some("inline-size scroll-state")
    );
}

/// §6.2: `none | <custom-ident>+`, excluding `none`, `and`, `not`, `or`
/// (and the CSS-wide keywords, `default`) as names; case-sensitive.
#[test]
fn container_name_parses() {
    let mut style = TuiStyle::new();
    set("container-name", "sidebar Card", &mut style).unwrap();
    assert_eq!(
        names(&spec(&style.container_name).unwrap()),
        ["sidebar", "Card"]
    );
    assert_eq!(
        serialize("container-name", &style).as_deref(),
        Some("sidebar Card")
    );
    set("container-name", "none", &mut style).unwrap();
    assert!(spec(&style.container_name).unwrap().is_none());
    for bad in [
        "none a", "a none", "and", "a or", "not", "default", "1", "'a'",
    ] {
        assert!(
            set("container-name", bad, &mut TuiStyle::new()).is_err(),
            "{bad}"
        );
    }
}

/// §6.3: `container: <'container-name'> [ / <'container-type'> ]?` — the
/// type resets to `normal` when omitted.
#[test]
fn container_shorthand_parses() {
    let mut style = TuiStyle::new();
    set("container", "card / inline-size", &mut style).unwrap();
    assert_eq!(names(&spec(&style.container_name).unwrap()), ["card"]);
    assert_eq!(
        spec(&style.container_type).unwrap().size,
        ContainerSize::InlineSize
    );
    assert_eq!(
        serialize("container", &style).as_deref(),
        Some("card / inline-size")
    );
    set("container", "a b", &mut style).unwrap();
    assert_eq!(spec(&style.container_type), Some(ContainerType::default()));
    assert_eq!(serialize("container", &style).as_deref(), Some("a b"));
    set("container", "none", &mut style).unwrap();
    assert!(spec(&style.container_name).unwrap().is_none());
    for bad in ["/ size", "a /", "a / b", "a / size /", ""] {
        assert!(
            set("container", bad, &mut TuiStyle::new()).is_err(),
            "{bad}"
        );
    }
}

/// Neither inherits (§6.1, §6.2).
#[test]
fn container_properties_do_not_inherit() {
    assert!(!inherits("container-type"));
    assert!(!inherits("container-name"));
}

/// CSS Containment 2 §2 / Containment 3: `contain`'s keywords and
/// combinations — `size` and `inline-size` exclusive, each type once.
#[test]
fn contain_parses() {
    use crate::layout::Contain;
    let parse = |css: &str| {
        let mut style = TuiStyle::new();
        set("contain", css, &mut style).ok()?;
        spec(&style.contain)
    };
    assert_eq!(parse("none"), Some(Contain::NONE));
    assert_eq!(parse("strict"), Some(Contain::STRICT));
    assert_eq!(parse("CONTENT"), Some(Contain::CONTENT));
    assert_eq!(
        parse("paint inline-size"),
        Some(Contain {
            inline_size: true,
            paint: true,
            ..Contain::NONE
        })
    );
    assert_eq!(parse("size layout style paint"), Some(Contain::STRICT));
    for bad in [
        "size inline-size",
        "layout layout",
        "none paint",
        "strict size",
        "auto",
        "",
    ] {
        assert_eq!(parse(bad), None, "{bad}");
    }
    let mut style = TuiStyle::new();
    set("contain", "layout style paint", &mut style).unwrap();
    assert_eq!(serialize("contain", &style).as_deref(), Some("content"));
    set("contain", "style layout", &mut style).unwrap();
    assert_eq!(
        serialize("contain", &style).as_deref(),
        Some("layout style")
    );
}

/// CSS Will Change 1 §2: `auto` or a list of `scroll-position`,
/// `contents` and custom idents — not `will-change`, `none`, `all`,
/// `auto` or a CSS-wide keyword; property names lowercased.
#[test]
fn will_change_parses() {
    let mut style = TuiStyle::new();
    set(
        "will-change",
        "Opacity, scroll-position, --mine",
        &mut style,
    )
    .unwrap();
    let w = spec(&style.will_change).unwrap();
    assert!(w.has("opacity"));
    assert!(w.has("scroll-position"));
    assert!(w.has("--mine"));
    assert_eq!(
        serialize("will-change", &style).as_deref(),
        Some("opacity, scroll-position, --mine")
    );
    set("will-change", "auto", &mut style).unwrap();
    assert!(spec(&style.will_change).unwrap().features().is_empty());
    for bad in [
        "none",
        "all",
        "opacity, auto",
        "will-change",
        "opacity transform",
        "1",
        "",
    ] {
        assert!(
            set("will-change", bad, &mut TuiStyle::new()).is_err(),
            "{bad}"
        );
    }
    assert!(!inherits("contain"));
    assert!(!inherits("will-change"));
}
