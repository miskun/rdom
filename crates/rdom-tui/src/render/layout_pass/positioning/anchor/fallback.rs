//! Position fallback (CSS Anchor Positioning 1 §4): a box's position
//! options — its base style, then each `position-try-fallbacks` entry: a
//! `@position-try` rule's declarations over the base, a `position-area`,
//! each with its try tactics applied — ordered by `position-try-order`
//! (§4.2); the first whose box does not overflow its inset-modified
//! containing block is used, else the first (§4.3).

use rdom_style::calc::{AnchorFunction, AnchorSide, AnchorSize, CalcExpr};

use crate::layout::{
    Align, Alignment, Length, MarginValue, MaxSize, MinSize, PositionTryOrder, Size, TryFallback,
    TryTactic,
};
use crate::style::{ComputedStyle, TuiStyle, Value};

/// One position option: its style, and the tactics its `position-area`
/// tracks flip by.
pub(super) struct PositionOption {
    pub(super) style: ComputedStyle,
    pub(super) tactics: Vec<TryTactic>,
}

/// `base`'s position options, unordered: the base first, then each valid
/// fallback (one naming a `@position-try` rule no sheet defines is none,
/// §4.1).
pub(super) fn options(base: &ComputedStyle) -> Vec<PositionOption> {
    let mut out = vec![PositionOption {
        style: base.clone(),
        tactics: Vec::new(),
    }];
    for f in &base.anchor.position_try_fallbacks {
        if let Some(option) = option(base, f) {
            out.push(option);
        }
    }
    out
}

fn option(base: &ComputedStyle, f: &TryFallback) -> Option<PositionOption> {
    let mut style = base.clone();
    if let Some(area) = f.area {
        style.anchor.position_area = Some(area);
        return Some(PositionOption {
            style,
            tactics: Vec::new(),
        });
    }
    if f.name.is_some() {
        apply_rule(&mut style, f.declarations()?);
    }
    for &t in &f.tactics {
        flip(&mut style, t);
    }
    Some(PositionOption {
        style,
        tactics: f.tactics.clone(),
    })
}

/// A `@position-try` rule's declarations over `s` (§4.1: they take the
/// place of the box's own values of those properties). `initial` takes the
/// initial value; another CSS-wide keyword keeps the box's.
fn apply_rule(s: &mut ComputedStyle, d: &TuiStyle) {
    let initial = ComputedStyle::initial();
    macro_rules! take {
        ($($($f:ident).+),* $(,)?) => {$(
            match &d.$($f).+ {
                Some(Value::Specified(v)) => s.$($f).+ = v.clone(),
                Some(Value::Initial) => s.$($f).+ = initial.$($f).+.clone(),
                _ => {}
            }
        )*};
    }
    take!(
        top,
        right,
        bottom,
        left,
        width,
        height,
        min_width,
        min_height,
        max_width,
        max_height,
        margin.top,
        margin.right,
        margin.bottom,
        margin.left,
        justify_self,
        align_self,
        anchor.position_anchor,
        anchor.position_area,
    );
}

/// `e` with each anchor function mapped by `f`.
fn map_anchors(e: &CalcExpr, f: &dyn Fn(&AnchorFunction) -> AnchorFunction) -> CalcExpr {
    match e {
        CalcExpr::Anchor(a) => CalcExpr::Anchor(Box::new(f(a))),
        CalcExpr::Binary { op, lhs, rhs } => {
            CalcExpr::binary(*op, map_anchors(lhs, f), map_anchors(rhs, f))
        }
        CalcExpr::Function { func, args } => {
            CalcExpr::function(*func, args.iter().map(|a| map_anchors(a, f)).collect())
        }
        other => other.clone(),
    }
}

/// An anchor function with its side mapped by `side` and its size by
/// `size`.
fn map_function(
    a: &AnchorFunction,
    side: fn(AnchorSide) -> AnchorSide,
    size: fn(AnchorSize) -> AnchorSize,
) -> AnchorFunction {
    let mut a = a.clone();
    match &mut a {
        AnchorFunction::Edge { side: s, .. } => *s = side(*s),
        AnchorFunction::Size { size: Some(z), .. } => *z = size(*z),
        _ => {}
    }
    a
}

macro_rules! map_values {
    ($s:expr, $side:expr, $size:expr) => {{
        let f = |a: &AnchorFunction| map_function(a, $side, $size);
        for l in [&mut $s.top, &mut $s.right, &mut $s.bottom, &mut $s.left] {
            if let Length::Calc(e) = l
                && e.contains_anchor()
            {
                *l = Length::calc(map_anchors(e, &f));
            }
        }
        for z in [&mut $s.width, &mut $s.height] {
            if let Size::Calc(e) = z
                && e.contains_anchor()
            {
                *z = Size::calc(map_anchors(e, &f));
            }
        }
        for z in [&mut $s.min_width, &mut $s.min_height] {
            if let MinSize::Calc(e) = z
                && e.contains_anchor()
            {
                *z = MinSize::calc(map_anchors(e, &f));
            }
        }
        for z in [&mut $s.max_width, &mut $s.max_height] {
            if let MaxSize::Calc(e) = z
                && e.contains_anchor()
            {
                *z = MaxSize::calc(map_anchors(e, &f));
            }
        }
        let m = &mut $s.margin;
        for z in [&mut m.top, &mut m.right, &mut m.bottom, &mut m.left] {
            if let MarginValue::Calc(e) = z
                && e.contains_anchor()
            {
                *z = MarginValue::calc(map_anchors(e, &f));
            }
        }
    }};
}

fn same_size(z: AnchorSize) -> AnchorSize {
    z
}

/// A try tactic on `s` (§4.1.1): its insets, margins and self-alignment
/// swapped across the axis — and the anchor sides they name — or, for
/// `flip-start`, the two axes swapped (sizes too).
fn flip(s: &mut ComputedStyle, tactic: TryTactic) {
    match tactic {
        TryTactic::FlipBlock | TryTactic::FlipY => {
            std::mem::swap(&mut s.top, &mut s.bottom);
            std::mem::swap(&mut s.margin.top, &mut s.margin.bottom);
            s.align_self = flip_alignment(s.align_self);
            map_values!(
                s,
                |side| match side {
                    AnchorSide::Top => AnchorSide::Bottom,
                    AnchorSide::Bottom => AnchorSide::Top,
                    other => flip_logical(other),
                },
                same_size
            );
        }
        TryTactic::FlipInline | TryTactic::FlipX => {
            std::mem::swap(&mut s.left, &mut s.right);
            std::mem::swap(&mut s.margin.left, &mut s.margin.right);
            s.justify_self = flip_alignment(s.justify_self);
            map_values!(
                s,
                |side| match side {
                    AnchorSide::Left => AnchorSide::Right,
                    AnchorSide::Right => AnchorSide::Left,
                    other => flip_logical(other),
                },
                same_size
            );
        }
        TryTactic::FlipStart => {
            std::mem::swap(&mut s.top, &mut s.left);
            std::mem::swap(&mut s.bottom, &mut s.right);
            std::mem::swap(&mut s.width, &mut s.height);
            std::mem::swap(&mut s.min_width, &mut s.min_height);
            std::mem::swap(&mut s.max_width, &mut s.max_height);
            std::mem::swap(&mut s.margin.top, &mut s.margin.left);
            std::mem::swap(&mut s.margin.bottom, &mut s.margin.right);
            let (justify, align) = (s.justify_self, s.align_self);
            s.justify_self = to_inline(align);
            s.align_self = to_block(justify);
            map_values!(
                s,
                |side| match side {
                    AnchorSide::Top => AnchorSide::Left,
                    AnchorSide::Left => AnchorSide::Top,
                    AnchorSide::Bottom => AnchorSide::Right,
                    AnchorSide::Right => AnchorSide::Bottom,
                    other => other,
                },
                |size| match size {
                    AnchorSize::Width => AnchorSize::Height,
                    AnchorSize::Height => AnchorSize::Width,
                    AnchorSize::Block => AnchorSize::Inline,
                    AnchorSize::Inline => AnchorSize::Block,
                    AnchorSize::SelfBlock => AnchorSize::SelfInline,
                    AnchorSize::SelfInline => AnchorSize::SelfBlock,
                }
            );
        }
    }
}

/// The logical sides an axis flip mirrors: `start` ↔ `end`, `self-start`
/// ↔ `self-end`, a percentage from the other end.
fn flip_logical(side: AnchorSide) -> AnchorSide {
    match side {
        AnchorSide::Start => AnchorSide::End,
        AnchorSide::End => AnchorSide::Start,
        AnchorSide::SelfStart => AnchorSide::SelfEnd,
        AnchorSide::SelfEnd => AnchorSide::SelfStart,
        AnchorSide::Percent(p) => AnchorSide::Percent(100.0 - p),
        other => other,
    }
}

/// A self-alignment mirrored across its axis.
fn flip_alignment(a: Alignment) -> Alignment {
    let keyword = match a.keyword {
        Align::Start => Align::End,
        Align::End => Align::Start,
        Align::SelfStart => Align::SelfEnd,
        Align::SelfEnd => Align::SelfStart,
        Align::FlexStart => Align::FlexEnd,
        Align::FlexEnd => Align::FlexStart,
        Align::Left => Align::Right,
        Align::Right => Align::Left,
        other => other,
    };
    Alignment { keyword, ..a }
}

/// A block-axis alignment moved to the inline axis (`flip-start`).
fn to_inline(a: Alignment) -> Alignment {
    a
}

/// An inline-axis alignment moved to the block axis: `left` / `right`,
/// inline-only, become `start` / `end` (`flip-start`).
fn to_block(a: Alignment) -> Alignment {
    let keyword = match a.keyword {
        Align::Left => Align::Start,
        Align::Right => Align::End,
        other => other,
    };
    Alignment { keyword, ..a }
}

/// §4.2: the options ordered by `order` — by their inset-modified
/// containing blocks' size on its axis, largest first, stably;
/// `sizes[i]` is option `i`'s `(width, height)`.
pub(super) fn order(order: PositionTryOrder, sizes: &[(u16, u16)]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..sizes.len()).collect();
    let key = |i: &usize| match order {
        PositionTryOrder::Normal => 0,
        PositionTryOrder::MostWidth | PositionTryOrder::MostInlineSize => sizes[*i].0,
        PositionTryOrder::MostHeight | PositionTryOrder::MostBlockSize => sizes[*i].1,
    };
    idx.sort_by_key(|i| std::cmp::Reverse(key(i)));
    idx
}
