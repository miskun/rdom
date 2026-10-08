//! The animation properties — the `animation-*` longhands and the
//! `animation` shorthand (CSS Animations 1 §4, CSS Animations 2 §3):
//! their `set` and `serialize` arms.

use super::value_serializers::{join_csv, serialize_timing_function, specified};
use crate::keyframes::{
    AnimationComposition, AnimationDuration, AnimationName, AnimationTimeline, IterationCount,
};
use crate::parse::token::Token;
use crate::parse::values::{
    AnimationPiece, parse_animation_duration_list, parse_animation_name_list,
    parse_animation_shorthand, parse_composition_list, parse_direction_list, parse_fill_mode_list,
    parse_iteration_count_list, parse_play_state_list, parse_time_list, parse_timeline_list,
    parse_timing_function_list,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    fn put<T>(field: &mut Option<Value<Vec<T>>>, list: Option<Vec<T>>) -> Option<()> {
        *field = Some(Value::Specified(list?));
        Some(())
    }
    Some(match name {
        "animation-name" => put(&mut style.animation_name, parse_animation_name_list(value)),
        "animation-duration" => put(
            &mut style.animation_duration,
            parse_animation_duration_list(value),
        ),
        "animation-timing-function" => put(
            &mut style.animation_timing_function,
            parse_timing_function_list(value),
        ),
        "animation-delay" => put(&mut style.animation_delay, parse_time_list(value)),
        "animation-iteration-count" => put(
            &mut style.animation_iteration_count,
            parse_iteration_count_list(value),
        ),
        "animation-direction" => put(&mut style.animation_direction, parse_direction_list(value)),
        "animation-fill-mode" => put(&mut style.animation_fill_mode, parse_fill_mode_list(value)),
        "animation-play-state" => put(
            &mut style.animation_play_state,
            parse_play_state_list(value),
        ),
        "animation-composition" => put(
            &mut style.animation_composition,
            parse_composition_list(value),
        ),
        "animation-timeline" => put(&mut style.animation_timeline, parse_timeline_list(value)),
        "animation" => parse_animation_shorthand(value).map(|pieces| set_shorthand(&pieces, style)),
        _ => return None,
    })
}

/// Write the shorthand's pieces: each longhand gets one entry per piece,
/// an unset component its initial value; the reset-only longhands go
/// back to their initial values (CSS Animations 2 §3.9).
fn set_shorthand(pieces: &[AnimationPiece], style: &mut TuiStyle) {
    fn each<T>(
        pieces: &[AnimationPiece],
        f: impl Fn(&AnimationPiece) -> T,
    ) -> Option<Value<Vec<T>>> {
        Some(Value::Specified(pieces.iter().map(f).collect()))
    }
    style.animation_name = each(pieces, |p| p.name.clone().unwrap_or(AnimationName::None));
    style.animation_duration = each(pieces, |p| p.duration.unwrap_or_default());
    style.animation_timing_function = each(pieces, |p| {
        p.timing
            .clone()
            .unwrap_or(crate::transition::TimingFunction::Ease)
    });
    style.animation_delay = each(pieces, |p| p.delay.unwrap_or(0));
    style.animation_iteration_count = each(pieces, |p| p.iterations.unwrap_or_default());
    style.animation_direction = each(pieces, |p| p.direction.unwrap_or_default());
    style.animation_fill_mode = each(pieces, |p| p.fill.unwrap_or_default());
    style.animation_play_state = each(pieces, |p| p.play_state.unwrap_or_default());
    style.animation_composition = Some(Value::Specified(vec![AnimationComposition::Replace]));
    style.animation_timeline = Some(Value::Specified(vec![AnimationTimeline::Auto]));
    let normal = || {
        Some(Value::Specified(vec![
            crate::keyframes::RangeBoundary::Normal,
        ]))
    };
    style.animation_range_start = normal();
    style.animation_range_end = normal();
}

/// A `<keyframes-name>` as CSS text: an identifier when it is one as
/// written — no escapes needed, not a reserved word — else a string
/// (`"a b"`, `"none"`), as browsers serialize it.
pub(crate) fn serialize_keyframes_name(name: &str) -> String {
    let ident = rdom_core::css_syntax::serialize_identifier(name);
    let plain = ident == name
        && crate::parse::values::keyframes_name(&Token::Ident(name.to_string())).is_some();
    if plain {
        ident
    } else {
        rdom_core::css_syntax::serialize_string(name)
    }
}

fn name_text(n: &AnimationName) -> String {
    match n {
        AnimationName::None => "none".to_string(),
        AnimationName::Named(n) => serialize_keyframes_name(n),
    }
}

fn duration_text(d: &AnimationDuration) -> String {
    match d {
        AnimationDuration::Auto => "auto".to_string(),
        AnimationDuration::Ms(ms) => format!("{ms}ms"),
    }
}

fn iterations_text(n: &IterationCount) -> String {
    match n {
        IterationCount::Infinite => "infinite".to_string(),
        IterationCount::Count(n) => n.to_string(),
    }
}

fn timeline_text(t: &AnimationTimeline) -> String {
    super::timeline::animation_timeline_text(t)
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    fn csv<T>(field: &Option<Value<Vec<T>>>, f: impl Fn(&T) -> String) -> Option<String> {
        field
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), f))
    }
    Some(match name {
        "animation-name" => csv(&style.animation_name, name_text),
        "animation-duration" => csv(&style.animation_duration, duration_text),
        "animation-timing-function" => {
            csv(&style.animation_timing_function, serialize_timing_function)
        }
        "animation-delay" => csv(&style.animation_delay, |ms| format!("{ms}ms")),
        "animation-iteration-count" => csv(&style.animation_iteration_count, iterations_text),
        "animation-direction" => csv(&style.animation_direction, |d| d.keyword().to_string()),
        "animation-fill-mode" => csv(&style.animation_fill_mode, |f| f.keyword().to_string()),
        "animation-play-state" => csv(&style.animation_play_state, |s| s.keyword().to_string()),
        "animation-composition" => csv(&style.animation_composition, |c| c.keyword().to_string()),
        "animation-timeline" => csv(&style.animation_timeline, timeline_text),
        "animation" => serialize_shorthand(style),
        _ => return None,
    })
}

/// `animation` serializes when its longhands' lists have one length and
/// the reset-only longhands hold their initial values (CSSOM §6.7.2):
/// each piece as duration, easing, delay, iteration count, direction,
/// fill mode, play state, name.
fn serialize_shorthand(style: &TuiStyle) -> Option<String> {
    fn of_len<T>(field: &Option<Value<Vec<T>>>, n: usize) -> Option<&Vec<T>> {
        field.as_ref().and_then(specified).filter(|l| l.len() == n)
    }
    let name = style.animation_name.as_ref().and_then(specified)?;
    let n = name.len();
    let duration = of_len(&style.animation_duration, n)?;
    let timing = of_len(&style.animation_timing_function, n)?;
    let delay = of_len(&style.animation_delay, n)?;
    let iterations = of_len(&style.animation_iteration_count, n)?;
    let direction = of_len(&style.animation_direction, n)?;
    let fill = of_len(&style.animation_fill_mode, n)?;
    let play = of_len(&style.animation_play_state, n)?;
    let initial_only = |c: &[AnimationComposition], t: &[AnimationTimeline]| {
        c.iter().all(|c| *c == AnimationComposition::Replace)
            && t.iter().all(|t| *t == AnimationTimeline::Auto)
    };
    let range_normal = |f: &Option<Value<Vec<crate::keyframes::RangeBoundary>>>| {
        f.as_ref().and_then(specified).is_none_or(|l| {
            l.iter()
                .all(|b| *b == crate::keyframes::RangeBoundary::Normal)
        })
    };
    if !range_normal(&style.animation_range_start) || !range_normal(&style.animation_range_end) {
        return None;
    }
    if !initial_only(
        style
            .animation_composition
            .as_ref()
            .and_then(specified)
            .map_or(&[][..], Vec::as_slice),
        style
            .animation_timeline
            .as_ref()
            .and_then(specified)
            .map_or(&[][..], Vec::as_slice),
    ) {
        return None;
    }
    Some(join_csv(0..n, |i| {
        format!(
            "{} {} {}ms {} {} {} {} {}",
            duration_text(&duration[i]),
            serialize_timing_function(&timing[i]),
            delay[i],
            iterations_text(&iterations[i]),
            direction[i].keyword(),
            fill[i].keyword(),
            play[i].keyword(),
            name_text(&name[i]),
        )
    }))
}
