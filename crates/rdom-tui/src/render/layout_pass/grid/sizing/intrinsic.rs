//! §11.5 Resolve Intrinsic Track Sizes, and §11.5.1 Distributing Extra
//! Space: the base sizes and growth limits of the tracks with intrinsic
//! sizing functions, from the contributions of the items they hold —
//! single-span items first, then spanning items by increasing span, then
//! the items that cross a flexible track, together.

use super::{Contributions, Space, fill_equally};
use crate::render::layout_pass::grid::track::{MaxFn, MinFn, Span, Track, TrackGrid};
use crate::render::layout_pass::shares::Rolling;

/// Run §11.5 steps 2–5 over `grid`, whose tracks `items` span.
pub(super) fn resolve(
    grid: &mut TrackGrid,
    items: &[Span],
    c: &mut dyn Contributions,
    space: Space,
) {
    let flexible: Vec<bool> = items
        .iter()
        .map(|s| grid.tracks[s.tracks()].iter().any(|t| t.flex().is_some()))
        .collect();
    // Step 2: tracks holding single-span items, none of them flexible.
    let mut singles: Vec<Vec<usize>> = vec![Vec::new(); grid.tracks.len()];
    for (i, s) in items.iter().enumerate() {
        if s.len() == 1 && !flexible[i] {
            singles[s.start].push(i);
        }
    }
    for (t, members) in singles.iter().enumerate() {
        if !members.is_empty() {
            size_to_singles(grid, t, members, c, space);
        }
    }
    // Step 3: spanning items that cross no flexible track, by span.
    let widest = items
        .iter()
        .zip(&flexible)
        .filter(|(s, f)| !**f && s.len() > 1)
        .map(|(s, _)| s.len())
        .max()
        .unwrap_or(0);
    for len in 2..=widest {
        let group: Vec<usize> = (0..items.len())
            .filter(|&i| !flexible[i] && items[i].len() == len)
            .collect();
        if !group.is_empty() {
            accommodate(grid, items, &group, c, space, Share::Equal);
        }
    }
    // Step 4: every item crossing a flexible track, together, into the
    // flexible tracks only.
    let group: Vec<usize> = (0..items.len()).filter(|&i| flexible[i]).collect();
    if !group.is_empty() {
        accommodate(grid, items, &group, c, space, Share::ByFlex);
    }
    // Step 5: an infinite growth limit left is the base size.
    for t in &mut grid.tracks {
        if t.limit.is_none() {
            t.limit = Some(t.base);
        }
    }
}

/// §11.5 step 2 for track `t` and its single-span `members`.
fn size_to_singles(
    grid: &mut TrackGrid,
    t: usize,
    members: &[usize],
    c: &mut dyn Contributions,
    space: Space,
) {
    let track = grid.tracks[t];
    if !track.min.is_intrinsic() && !track.max.is_intrinsic() {
        return;
    }
    let span = Span::new(t, t + 1);
    let largest = |f: &mut dyn FnMut(usize) -> u32| members.iter().map(|&i| f(i)).max();
    let base = match track.min {
        MinFn::MinContent => largest(&mut |i| c.min_content(i)),
        MinFn::MaxContent => largest(&mut |i| c.max_content(i)),
        // Under a min- / max-content constraint, the limited min-content
        // contributions; otherwise the minimum contributions.
        MinFn::Auto if space.is_constrained() => largest(&mut |i| {
            let (min_content, minimum) = (c.min_content(i), c.minimum(i));
            limited(grid, span, min_content, minimum)
        }),
        MinFn::Auto => largest(&mut |i| c.minimum(i)),
        MinFn::Fixed(_) => None,
    };
    let limit = match track.max {
        MaxFn::MinContent => largest(&mut |i| c.min_content(i)),
        MaxFn::MaxContent | MaxFn::Auto => largest(&mut |i| c.max_content(i)),
        // "For fit-content() maximums, furthermore clamp this growth limit
        // by the fit-content() argument."
        MaxFn::FitContent(arg) => largest(&mut |i| c.max_content(i)).map(|m| m.min(arg)),
        MaxFn::Fixed(_) | MaxFn::Flex(_) => None,
    };
    let track = &mut grid.tracks[t];
    if let Some(b) = base {
        track.base = b;
    }
    if let Some(l) = limit {
        track.limit = Some(l);
    }
    if let Some(l) = track.limit
        && l < track.base
    {
        track.limit = Some(track.base);
    }
}

/// A limited contribution (§11.5): `contribution` capped at the sum of
/// the spanned tracks' fixed max track sizing functions (a length, or a
/// `fit-content()` argument) when every spanned track has one, and
/// floored by the item's `minimum` contribution.
fn limited(grid: &TrackGrid, span: Span, contribution: u32, minimum: u32) -> u32 {
    let caps: Option<u32> = grid.tracks[span.tracks()]
        .iter()
        .map(|t| t.max.fixed_limit())
        .sum::<Option<u32>>()
        .map(|sum| sum + grid.inner_gutters(span));
    caps.map_or(contribution, |cap| contribution.min(cap))
        .max(minimum)
}

/// How a spanning item's extra space is shared among its tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Share {
    /// Equally, up to the tracks' limits then beyond (step 3).
    Equal,
    /// To the flexible tracks only, by their flex factors (step 4).
    ByFlex,
}

/// Which size a distribution grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Base,
    Limit,
}

/// Which affected tracks take the space left once every track is at its
/// limit (§11.5.1 step 2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Beyond {
    /// Accommodating minimum or min-content contributions: the tracks
    /// with an intrinsic max track sizing function.
    IntrinsicMax,
    /// Accommodating max-content contributions: the tracks with a
    /// max-content max track sizing function.
    MaxContentMax,
    /// A growth limit: every affected track.
    All,
}

/// §11.5 step 3's sub-steps (step 4's with [`Share::ByFlex`]) for one
/// group of items.
fn accommodate(
    grid: &mut TrackGrid,
    items: &[Span],
    group: &[usize],
    c: &mut dyn Contributions,
    space: Space,
    share: Share,
) {
    let flex_only = share == Share::ByFlex;
    let only =
        move |t: &Track, rule: fn(&Track) -> bool| (!flex_only || t.flex().is_some()) && rule(t);
    // 1. Intrinsic minimums: the minimum contributions — limited
    //    min-content ones under a min- / max-content constraint.
    let pass = Pass {
        target: Target::Base,
        beyond: Beyond::IntrinsicMax,
        share,
        mark_growable: false,
    };
    distribute(
        grid,
        items,
        group,
        pass,
        &|t| only(t, |t| t.min.is_intrinsic()),
        &mut |g, i| {
            if space.is_constrained() {
                let (min_content, minimum) = (c.min_content(i), c.minimum(i));
                limited(g, items[i], min_content, minimum)
            } else {
                c.minimum(i)
            }
        },
    );
    // 2. Content-based minimums: the min-content contributions.
    distribute(
        grid,
        items,
        group,
        pass,
        &|t| {
            only(t, |t| {
                matches!(t.min, MinFn::MinContent | MinFn::MaxContent)
            })
        },
        &mut |_, i| c.min_content(i),
    );
    // 3. Max-content minimums: under a max-content constraint the limited
    //    max-content contributions to `auto` / `max-content` minimums,
    //    and in all cases the max-content contributions to `max-content`
    //    minimums.
    let pass = Pass {
        beyond: Beyond::MaxContentMax,
        ..pass
    };
    if space == Space::MaxContent {
        distribute(
            grid,
            items,
            group,
            pass,
            &|t| only(t, |t| matches!(t.min, MinFn::Auto | MinFn::MaxContent)),
            &mut |g, i| {
                let (max_content, minimum) = (c.max_content(i), c.minimum(i));
                limited(g, items[i], max_content, minimum)
            },
        );
    }
    distribute(
        grid,
        items,
        group,
        pass,
        &|t| only(t, |t| t.min == MinFn::MaxContent),
        &mut |_, i| c.max_content(i),
    );
    // 4. No growth limit below its base size.
    for t in &mut grid.tracks {
        if let Some(l) = t.limit
            && l < t.base
        {
            t.limit = Some(t.base);
        }
    }
    if flex_only {
        // A flexible track's maximum is no intrinsic function.
        return;
    }
    // 5. Intrinsic maximums: the min-content contributions, marking the
    //    limits that become finite infinitely growable for step 6.
    let pass = Pass {
        target: Target::Limit,
        beyond: Beyond::All,
        share,
        mark_growable: true,
    };
    distribute(
        grid,
        items,
        group,
        pass,
        &|t| t.max.is_intrinsic(),
        &mut |_, i| c.min_content(i),
    );
    // 6. Max-content maximums: the max-content contributions, a
    //    `fit-content()` track growing no further than its argument.
    distribute(
        grid,
        items,
        group,
        Pass {
            mark_growable: false,
            ..pass
        },
        &|t| t.max.is_max_content(),
        &mut |_, i| c.max_content(i),
    );
    for t in &mut grid.tracks {
        t.growable = false;
    }
}

/// One distribution's settings.
#[derive(Debug, Clone, Copy)]
struct Pass {
    target: Target,
    beyond: Beyond,
    share: Share,
    /// Mark the tracks whose growth limit becomes finite infinitely
    /// growable (step 3.5).
    mark_growable: bool,
}

/// §11.5.1 for one sub-step: each item of `group` asks its `affected`
/// tracks for the space its `contribution` needs beyond their sizes;
/// each track's planned increase is the largest any item incurs, and is
/// added once every item is considered. An item with no affected track
/// is skipped, and its contribution never measured.
fn distribute(
    grid: &mut TrackGrid,
    items: &[Span],
    group: &[usize],
    pass: Pass,
    affected: &dyn Fn(&Track) -> bool,
    contribution: &mut dyn FnMut(&TrackGrid, usize) -> u32,
) {
    let n = grid.tracks.len();
    let mut planned = vec![0u32; n];
    let mut touched = vec![false; n];
    let size = |t: &Track| match pass.target {
        Target::Base => t.base,
        Target::Limit => t.limit_or_base(),
    };
    for &i in group {
        let span = items[i];
        let tracks: Vec<usize> = span
            .tracks()
            .filter(|&t| affected(&grid.tracks[t]))
            .collect();
        if tracks.is_empty() {
            continue;
        }
        for &t in &tracks {
            touched[t] = true;
        }
        let space = contribution(grid, i).saturating_sub(grid.span_size(span, size));
        if space == 0 {
            continue;
        }
        let incurred = match pass.share {
            Share::ByFlex => by_flex(grid, &tracks, space),
            Share::Equal => equally(grid, &tracks, space, pass),
        };
        for (&t, inc) in tracks.iter().zip(incurred) {
            planned[t] = planned[t].max(inc);
        }
    }
    for (t, track) in grid.tracks.iter_mut().enumerate() {
        if !touched[t] {
            continue;
        }
        match pass.target {
            Target::Base => track.base += planned[t],
            Target::Limit => {
                let was_infinite = track.limit.is_none();
                track.limit = Some(track.limit_or_base() + planned[t]);
                if pass.mark_growable && was_infinite {
                    track.growable = true;
                }
            }
        }
    }
}

/// §11.5.1 steps 2.2–2.3: `space` shared equally among `tracks`, first
/// up to each one's limit — for a base size its growth limit (capped by
/// a `fit-content()` argument), for a growth limit infinity when the
/// track is infinitely growable (or its limit still infinite), else no
/// growth — then what is left among the tracks `pass.beyond` picks (all
/// of them when it picks none), a `fit-content()` track stopping at its
/// argument while another can take the rest.
fn equally(grid: &TrackGrid, tracks: &[usize], space: u32, pass: Pass) -> Vec<u32> {
    let size = |t: &Track| match pass.target {
        Target::Base => t.base,
        Target::Limit => t.limit_or_base(),
    };
    let fit = |t: &Track| match t.max {
        MaxFn::FitContent(arg) => Some(arg),
        _ => None,
    };
    let headroom: Vec<Option<u32>> = tracks
        .iter()
        .map(|&k| {
            let t = &grid.tracks[k];
            let cap = match pass.target {
                Target::Base => match (t.limit, fit(t)) {
                    (Some(l), Some(a)) => Some(l.min(a)),
                    (l, a) => l.or(a),
                },
                Target::Limit if t.growable || t.limit.is_none() => fit(t),
                Target::Limit => Some(size(t)),
            };
            cap.map(|c| c.saturating_sub(size(t)))
        })
        .collect();
    let (mut increase, mut left) = fill_equally(space, &headroom);
    if left == 0 {
        return increase;
    }
    let picks = |k: usize, inc: u32| {
        let t = &grid.tracks[k];
        match pass.beyond {
            Beyond::IntrinsicMax => t.max.is_intrinsic(),
            // `fit-content()` is a max-content maximum until it reaches
            // its argument.
            Beyond::MaxContentMax => match t.max {
                MaxFn::FitContent(arg) => size(t) + inc < arg,
                m => m.is_max_content(),
            },
            Beyond::All => true,
        }
    };
    let mut chosen: Vec<usize> = (0..tracks.len())
        .filter(|&j| picks(tracks[j], increase[j]))
        .collect();
    if chosen.is_empty() {
        chosen = (0..tracks.len()).collect();
    }
    let capped: Vec<Option<u32>> = chosen
        .iter()
        .map(|&j| {
            let t = &grid.tracks[tracks[j]];
            fit(t).map(|a| a.saturating_sub(size(t) + increase[j]))
        })
        .collect();
    // A `fit-content()` track stops at its argument while an uncapped one
    // can take the rest.
    let headroom = if capped.iter().all(Option::is_some) {
        vec![None; chosen.len()]
    } else {
        capped
    };
    let (more, rest) = fill_equally(left, &headroom);
    for (&j, m) in chosen.iter().zip(more) {
        increase[j] += m;
    }
    left = rest;
    debug_assert_eq!(left, 0, "an uncapped track takes every cell left");
    increase
}

/// Step 4: `space` shared among the flexible `tracks` by their flex
/// factors — if those sum to less than one, that fraction of the space
/// by factor and the rest equally — rolling in whole cells.
fn by_flex(grid: &TrackGrid, tracks: &[usize], space: u32) -> Vec<u32> {
    let factors: Vec<f64> = tracks
        .iter()
        .map(|&k| f64::from(grid.tracks[k].flex().unwrap_or(0.0)))
        .collect();
    let sum: f64 = factors.iter().sum();
    let even = (1.0 - sum).max(0.0) / tracks.len() as f64;
    let weights: Vec<f64> = factors.iter().map(|f| f + even).collect();
    let mut shares = Rolling::new(f64::from(space), weights.iter().sum());
    weights.iter().map(|&w| shares.share(w)).collect()
}
