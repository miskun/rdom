//! Dispatch tests for `visibility` (C6-VISIBILITY, CSS Display 3 §4).

use super::*;
use crate::layout::Visibility;
use crate::{ImportantMask, TuiStyle, Value};

/// CSS Display 3 §4: `visible | hidden | collapse`, ASCII
/// case-insensitive, inherited, serialized as the keyword; and an
/// animatable property (CSS Transitions §, CSS Display 3 §4 "Animation
/// type: discrete, with visible in between").
#[test]
fn visibility_takes_its_three_keywords_and_inherits() {
    for (css, kw) in [
        ("visible", Visibility::Visible),
        ("HIDDEN", Visibility::Hidden),
        ("collapse", Visibility::Collapse),
    ] {
        let mut style = TuiStyle::new();
        set("visibility", css, &mut style).unwrap();
        assert_eq!(style.visibility, Some(Value::Specified(kw)));
        assert_eq!(
            serialize("visibility", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["none", "hidden visible", "0", ""] {
        assert_eq!(
            set("visibility", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(inherits("visibility"));
    assert_eq!(property_mask("visibility"), Some(ImportantMask::VISIBILITY));
    let mut style = TuiStyle::new();
    set("transition-property", "visibility", &mut style).unwrap();
    assert_eq!(
        serialize("transition-property", &style).as_deref(),
        Some("visibility")
    );
}
