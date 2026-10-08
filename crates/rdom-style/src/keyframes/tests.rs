//! `KeyframesRule::resolve` (CSS Animations 1 §3): keyframes sorted by
//! offset, the blocks sharing an offset cascading in source order.

use super::*;

fn block(offsets: &[f32], color: crate::Color) -> Keyframe {
    Keyframe::new(
        offsets
            .iter()
            .map(|o| KeyframeSelector::at(*o).unwrap())
            .collect(),
        TuiStyle::new().fg(color),
    )
}

/// §3: "all of the values in the selectors are sorted in increasing
/// order by time" — a block listed under two selectors serves both.
#[test]
fn resolve_sorts_offsets_and_shares_blocks() {
    let rule = KeyframesRule::new("a")
        .with(block(&[1.0], crate::Color::Rgb(255, 0, 0)))
        .with(block(&[0.0, 0.5], crate::Color::Rgb(0, 0, 255)));
    let r = rule.resolve();
    let offsets: Vec<f32> = r.iter().map(|k| k.offset).collect();
    assert_eq!(offsets, [0.0, 0.5, 1.0]);
    assert!(std::ptr::eq(r[0].blocks[0], r[1].blocks[0]));
}

/// §3: "The rules within the @keyframes rule then cascade" — two blocks
/// at one offset both apply, in source order; a block naming the same
/// offset twice counts once.
#[test]
fn blocks_at_one_offset_cascade_in_source_order() {
    let rule = KeyframesRule::new("a")
        .with(block(&[0.5, 0.5], crate::Color::Rgb(255, 0, 0)))
        .with(block(&[0.5], crate::Color::Rgb(0, 0, 255)));
    let r = rule.resolve();
    assert_eq!(r.len(), 1);
    let colors: Vec<_> = r[0].blocks.iter().map(|b| b.style.fg.clone()).collect();
    assert_eq!(
        colors,
        [
            Some(crate::Value::Specified(crate::Color::Rgb(255, 0, 0).into())),
            Some(crate::Value::Specified(crate::Color::Rgb(0, 0, 255).into()))
        ]
    );
}

/// §3: a selector outside 0%–100% is invalid.
#[test]
fn selectors_stay_in_the_unit_interval() {
    assert!(KeyframeSelector::at(-0.01).is_none());
    assert!(KeyframeSelector::at(1.01).is_none());
    assert_eq!(KeyframeSelector::at(0.25).map(|s| s.offset()), Some(0.25));
}
