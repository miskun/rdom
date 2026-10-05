//! Dispatch tests for the flexbox properties of Phase 6 (C6-ORDER, …).

use super::*;
use crate::{ImportantMask, TuiStyle, Value};

/// CSS Flexbox §5.4: `order: <integer>`, initial 0, not inherited; a
/// math function rounds to an integer (CSS Values 4 §10.9) and a value
/// past `i32` clamps to it (§5.1); a non-integer is invalid.
#[test]
fn order_takes_an_integer() {
    for (css, n, out) in [
        ("0", 0, "0"),
        ("-3", -3, "-3"),
        ("7", 7, "7"),
        ("calc(2 * 3)", 6, "6"),
        ("99999999999", i32::MAX, "2147483647"),
    ] {
        let mut style = TuiStyle::new();
        set("order", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
        assert_eq!(style.order, Some(Value::Specified(n)), "{css}");
        assert_eq!(serialize("order", &style).as_deref(), Some(out), "{css}");
    }
    for bad in ["1.5", "auto", "1px", "", "1 2"] {
        assert_eq!(
            set("order", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("order"));
    assert_eq!(property_mask("order"), Some(ImportantMask::ORDER));
}
