//! The scroll-driven animation properties (Scroll-driven Animations 1):
//! `scroll-timeline*`, `view-timeline*`, `timeline-scope` and
//! `animation-range*` — their `set` and `serialize` arms
//! (`animation-timeline` is `animation`'s).

use super::value_serializers::{join_csv, serialize_length, specified};
use crate::keyframes::{
    AnimationTimeline, RangeBoundary, TimelineAxis, TimelineInset, TimelineName, TimelineScope,
    TimelineScroller,
};
use crate::layout::Length;
use crate::parse::token::Token;
use crate::parse::values::{
    parse_animation_range, parse_range_end_list, parse_range_start_list, parse_scroll_timeline,
    parse_timeline_axis_list, parse_timeline_inset_list, parse_timeline_name_list,
    parse_timeline_scope, parse_view_timeline,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    fn put<T>(field: &mut Option<Value<T>>, v: Option<T>) -> Option<()> {
        *field = Some(Value::Specified(v?));
        Some(())
    }
    Some(match name {
        "scroll-timeline-name" => put(
            &mut style.motion.scroll_timeline_name,
            parse_timeline_name_list(value),
        ),
        "scroll-timeline-axis" => put(
            &mut style.motion.scroll_timeline_axis,
            parse_timeline_axis_list(value),
        ),
        "scroll-timeline" => parse_scroll_timeline(value).map(|pieces| {
            let (names, axes) = pieces.into_iter().unzip();
            style.motion.scroll_timeline_name = Some(Value::Specified(names));
            style.motion.scroll_timeline_axis = Some(Value::Specified(axes));
        }),
        "view-timeline-name" => put(
            &mut style.motion.view_timeline_name,
            parse_timeline_name_list(value),
        ),
        "view-timeline-axis" => put(
            &mut style.motion.view_timeline_axis,
            parse_timeline_axis_list(value),
        ),
        "view-timeline-inset" => put(
            &mut style.motion.view_timeline_inset,
            parse_timeline_inset_list(value),
        ),
        "view-timeline" => parse_view_timeline(value).map(|pieces| {
            let mut names = Vec::new();
            let mut axes = Vec::new();
            let mut insets = Vec::new();
            for (n, a, i) in pieces {
                names.push(n);
                axes.push(a);
                insets.push(i);
            }
            style.motion.view_timeline_name = Some(Value::Specified(names));
            style.motion.view_timeline_axis = Some(Value::Specified(axes));
            style.motion.view_timeline_inset = Some(Value::Specified(insets));
        }),
        "timeline-scope" => put(
            &mut style.motion.timeline_scope,
            parse_timeline_scope(value),
        ),
        "animation-range-start" => put(
            &mut style.motion.animation_range_start,
            parse_range_start_list(value),
        ),
        "animation-range-end" => put(
            &mut style.motion.animation_range_end,
            parse_range_end_list(value),
        ),
        "animation-range" => parse_animation_range(value).map(|pieces| {
            let (starts, ends) = pieces.into_iter().unzip();
            style.motion.animation_range_start = Some(Value::Specified(starts));
            style.motion.animation_range_end = Some(Value::Specified(ends));
        }),
        _ => return None,
    })
}

fn name_text(n: &TimelineName) -> String {
    match n {
        TimelineName::None => "none".to_string(),
        TimelineName::Named(n) => rdom_core::css_syntax::serialize_identifier(n),
    }
}

/// An inset pair as written: one value when both sides agree.
pub(crate) fn inset_text(i: &TimelineInset) -> String {
    if i.start == i.end {
        serialize_length(&i.start)
    } else {
        format!(
            "{} {}",
            serialize_length(&i.start),
            serialize_length(&i.end)
        )
    }
}

/// A range boundary: `normal`, an offset, or a name and its offset.
pub(crate) fn boundary_text(b: &RangeBoundary) -> String {
    match b {
        RangeBoundary::Normal => "normal".to_string(),
        RangeBoundary::Offset { name: None, offset } => serialize_length(offset),
        RangeBoundary::Offset {
            name: Some(n),
            offset,
        } => format!("{} {}", n.keyword(), serialize_length(offset)),
    }
}

/// An `animation-timeline` entry in its shortest form (defaults left
/// out: `scroll(nearest block)` is `scroll()`).
pub(crate) fn animation_timeline_text(t: &AnimationTimeline) -> String {
    match t {
        AnimationTimeline::Auto => "auto".to_string(),
        AnimationTimeline::None => "none".to_string(),
        AnimationTimeline::Named(n) => rdom_core::css_syntax::serialize_identifier(n),
        AnimationTimeline::Scroll { scroller, axis } => {
            let mut parts = Vec::new();
            if *scroller != TimelineScroller::Nearest {
                parts.push(scroller.keyword());
            }
            if *axis != TimelineAxis::Block {
                parts.push(axis.keyword());
            }
            format!("scroll({})", parts.join(" "))
        }
        AnimationTimeline::View { axis, inset } => {
            let mut parts = Vec::new();
            if *axis != TimelineAxis::Block {
                parts.push(axis.keyword().to_string());
            }
            if !(inset.start == Length::Auto && inset.end == Length::Auto) {
                parts.push(inset_text(inset));
            }
            format!("view({})", parts.join(" "))
        }
    }
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    fn csv<T>(field: &Option<Value<Vec<T>>>, f: impl Fn(&T) -> String) -> Option<String> {
        field
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), f))
    }
    fn pair<A, B>(
        a: &Option<Value<Vec<A>>>,
        b: &Option<Value<Vec<B>>>,
        f: impl Fn(&A, &B) -> String,
    ) -> Option<String> {
        let (a, b) = (
            a.as_ref().and_then(specified)?,
            b.as_ref().and_then(specified)?,
        );
        (a.len() == b.len()).then(|| join_csv(a.iter().zip(b), |(a, b)| f(a, b)))
    }
    Some(match name {
        "scroll-timeline-name" => csv(&style.motion.scroll_timeline_name, name_text),
        "scroll-timeline-axis" => csv(&style.motion.scroll_timeline_axis, |a| {
            a.keyword().to_string()
        }),
        "scroll-timeline" => pair(
            &style.motion.scroll_timeline_name,
            &style.motion.scroll_timeline_axis,
            |n, a| match a {
                TimelineAxis::Block => name_text(n),
                a => format!("{} {}", name_text(n), a.keyword()),
            },
        ),
        "view-timeline-name" => csv(&style.motion.view_timeline_name, name_text),
        "view-timeline-axis" => csv(&style.motion.view_timeline_axis, |a| {
            a.keyword().to_string()
        }),
        "view-timeline-inset" => csv(&style.motion.view_timeline_inset, inset_text),
        "view-timeline" => {
            let names = style
                .motion
                .view_timeline_name
                .as_ref()
                .and_then(specified)?;
            let axes = style
                .motion
                .view_timeline_axis
                .as_ref()
                .and_then(specified)?;
            let insets = style
                .motion
                .view_timeline_inset
                .as_ref()
                .and_then(specified)?;
            (names.len() == axes.len() && names.len() == insets.len()).then(|| {
                join_csv(0..names.len(), |i| {
                    format!(
                        "{} {} {}",
                        name_text(&names[i]),
                        axes[i].keyword(),
                        inset_text(&insets[i])
                    )
                })
            })
        }
        "timeline-scope" => style
            .motion
            .timeline_scope
            .as_ref()
            .and_then(specified)
            .map(|s| match s {
                TimelineScope::None => "none".to_string(),
                TimelineScope::All => "all".to_string(),
                TimelineScope::Names(names) => join_csv(names.iter(), |n| {
                    rdom_core::css_syntax::serialize_identifier(n)
                }),
            }),
        "animation-range-start" => csv(&style.motion.animation_range_start, boundary_text),
        "animation-range-end" => csv(&style.motion.animation_range_end, boundary_text),
        "animation-range" => pair(
            &style.motion.animation_range_start,
            &style.motion.animation_range_end,
            |s, e| format!("{} {}", boundary_text(s), boundary_text(e)),
        ),
        _ => return None,
    })
}
