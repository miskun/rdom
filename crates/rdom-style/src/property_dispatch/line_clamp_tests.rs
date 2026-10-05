//! Dispatch tests for `line-clamp` and its longhands (C8-LINE-CLAMP,
//! CSS Overflow 4 §4), and the legacy `-webkit-line-clamp`,
//! `-webkit-box-orient` and `display: -webkit-box`.

use super::*;
use crate::layout::{BlockEllipsis, BoxOrient, Continue, Display, Flow};
use crate::{TuiStyle, Value};

fn spec<T: Clone>(v: &Option<Value<T>>) -> Option<T> {
    match v {
        Some(Value::Specified(t)) => Some(t.clone()),
        _ => None,
    }
}

/// `(max-lines, block-ellipsis, continue)` as specified.
type Longhands = (Option<Option<u32>>, Option<BlockEllipsis>, Option<Continue>);

/// The [`Longhands`] after `name: css`.
fn longhands(name: &str, css: &str) -> Option<Longhands> {
    let mut style = TuiStyle::new();
    set(name, css, &mut style).ok()?;
    Some((
        spec(&style.max_lines),
        spec(&style.block_ellipsis),
        spec(&style.continue_),
    ))
}

/// §4.1: `line-clamp: none | [ <integer [1,∞]> || <'block-ellipsis'> ]
/// -webkit-legacy?` — `none` is `max-lines: none; continue: auto;
/// block-ellipsis: no-ellipsis`; an integer is `max-lines` with
/// `block-ellipsis: auto` unless given and `continue: collapse`, or
/// `-webkit-legacy` when written.
#[test]
fn line_clamp_sets_its_three_longhands() {
    use BlockEllipsis::*;
    assert_eq!(
        longhands("line-clamp", "none"),
        Some((Some(None), Some(NoEllipsis), Some(Continue::Auto)))
    );
    assert_eq!(
        longhands("line-clamp", "3"),
        Some((Some(Some(3)), Some(Auto), Some(Continue::Collapse)))
    );
    assert_eq!(
        longhands("line-clamp", "'+' 2"),
        Some((
            Some(Some(2)),
            Some(Str("+".into())),
            Some(Continue::Collapse)
        ))
    );
    assert_eq!(
        longhands("line-clamp", "2 no-ellipsis -webkit-legacy"),
        Some((
            Some(Some(2)),
            Some(NoEllipsis),
            Some(Continue::WebkitLegacy)
        ))
    );
    for bad in ["0", "-1", "2 3", "auto", "none 2", "1.5", ""] {
        assert_eq!(longhands("line-clamp", bad), None, "{bad:?}");
    }
    let mut style = TuiStyle::new();
    set("line-clamp", "2 '>'", &mut style).unwrap();
    assert_eq!(serialize("line-clamp", &style).as_deref(), Some("2 \">\""));
    set("line-clamp", "4", &mut style).unwrap();
    assert_eq!(serialize("line-clamp", &style).as_deref(), Some("4"));
}

/// §4.2–§4.4: the longhands on their own; `block-ellipsis` inherits.
#[test]
fn the_longhands_parse_and_serialize() {
    let one = |name: &str, css: &str| {
        let mut style = TuiStyle::new();
        set(name, css, &mut style).ok()?;
        serialize(name, &style)
    };
    assert_eq!(one("max-lines", "2").as_deref(), Some("2"));
    assert_eq!(one("max-lines", "none").as_deref(), Some("none"));
    assert_eq!(one("max-lines", "0"), None);
    assert_eq!(one("block-ellipsis", "auto").as_deref(), Some("auto"));
    assert_eq!(one("block-ellipsis", "'x'").as_deref(), Some("\"x\""));
    assert_eq!(
        one("block-ellipsis", "NO-ELLIPSIS").as_deref(),
        Some("no-ellipsis")
    );
    for kw in ["auto", "discard", "collapse", "-webkit-legacy"] {
        assert_eq!(one("continue", kw).as_deref(), Some(kw));
    }
    assert!(inherits("block-ellipsis"));
    assert!(!inherits("max-lines") && !inherits("continue"));
}

/// The legacy forms: `-webkit-line-clamp: <integer>` is `max-lines`
/// with `block-ellipsis: auto` and `continue: -webkit-legacy`; `none`
/// is `max-lines: none; continue: auto`, `block-ellipsis: auto`.
/// `display: -webkit-box` / `-webkit-inline-box` are `flex` /
/// `inline-flex` (Compat Standard §5), and `-webkit-box-orient` its
/// four keywords.
#[test]
fn the_webkit_forms() {
    use BlockEllipsis::*;
    assert_eq!(
        longhands("-webkit-line-clamp", "2"),
        Some((Some(Some(2)), Some(Auto), Some(Continue::WebkitLegacy)))
    );
    assert_eq!(
        longhands("-webkit-line-clamp", "none"),
        Some((Some(None), Some(Auto), Some(Continue::Auto)))
    );
    let mut style = TuiStyle::new();
    set("display", "-webkit-box", &mut style).unwrap();
    assert_eq!(
        (spec(&style.display), spec(&style.flow)),
        (Some(Display::Block), Some(Flow::Flex))
    );
    set("display", "-webkit-inline-box", &mut style).unwrap();
    assert_eq!(
        (spec(&style.display), spec(&style.flow)),
        (Some(Display::Inline), Some(Flow::Flex))
    );
    set("-webkit-box-orient", "vertical", &mut style).unwrap();
    assert_eq!(spec(&style.webkit_box_orient), Some(BoxOrient::Vertical));
    assert_eq!(
        serialize("-webkit-box-orient", &style).as_deref(),
        Some("vertical")
    );
    set("-webkit-line-clamp", "3", &mut style).unwrap();
    assert_eq!(
        serialize("-webkit-line-clamp", &style).as_deref(),
        Some("3")
    );
}
