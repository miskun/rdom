//! The track sizing algorithm (CSS Grid 2 §11.3–§11.8) over hand-made
//! contributions: the spanning-item and flexible-track distributions
//! that auto-placed single-span items cannot reach.

use super::*;
use crate::render::layout_pass::grid::track::{MaxFn, MinFn, Span, Track, TrackGrid};

/// Each item's (min-content, max-content, minimum) contribution.
struct Fixed(Vec<(u32, u32, u32)>);

impl Contributions for Fixed {
    fn min_content(&mut self, i: usize) -> u32 {
        self.0[i].0
    }
    fn max_content(&mut self, i: usize) -> u32 {
        self.0[i].1
    }
    fn minimum(&mut self, i: usize) -> u32 {
        self.0[i].2
    }
}

fn auto() -> Track {
    Track::with(MinFn::Auto, MaxFn::Auto)
}

fn fr(f: f32) -> Track {
    Track::with(MinFn::Auto, MaxFn::Flex(f))
}

fn sized(tracks: Vec<Track>, items: &[Span], c: Vec<(u32, u32, u32)>, space: Space) -> Vec<u32> {
    let n = tracks.len();
    let mut grid = TrackGrid::new(tracks, &vec![false; n], 0);
    size_tracks(
        &mut grid,
        items,
        &mut Fixed(c),
        Frame {
            space,
            stretch: false,
            min: None,
            max: None,
        },
    );
    grid.tracks.iter().map(|t| t.base).collect()
}

/// §11.5.1: whole cells — an equal share each, the remainder to the
/// first tracks with room — each track freezing at its room.
#[test]
fn fill_equally_freezes_and_hands_the_remainder_to_the_first() {
    assert_eq!(fill_equally(7, &[Some(1), None, None]), (vec![1, 3, 3], 0));
    assert_eq!(fill_equally(3, &[None, None]), (vec![2, 1], 0));
    assert_eq!(fill_equally(5, &[Some(0), Some(2)]), (vec![0, 2], 3));
}

/// §11.5 step 3.1: an item spanning two `auto` tracks shares the space
/// its minimum contribution needs equally between them (the odd cell to
/// the first); its min- / max-content contributions then set the growth
/// limits (steps 3.5 / 3.6), which hold no more.
#[test]
fn a_spanning_item_shares_its_space_among_auto_tracks() {
    let items = [Span::new(0, 2)];
    assert_eq!(
        sized(
            vec![auto(), auto()],
            &items,
            vec![(7, 7, 7)],
            Space::Definite(7)
        ),
        [4, 3]
    );
}

/// §11.5 step 3.1: only the tracks with an intrinsic min track sizing
/// function grow — a fixed track keeps its size.
#[test]
fn a_spanning_item_grows_only_its_intrinsic_tracks() {
    let items = [Span::new(0, 2)];
    let fixed = Track::with(MinFn::Fixed(2), MaxFn::Fixed(2));
    assert_eq!(
        sized(
            vec![fixed, auto()],
            &items,
            vec![(10, 10, 10)],
            Space::Definite(10)
        ),
        [2, 8]
    );
}

/// §11.5.1 step 2.3: once every affected track is at its limit, the
/// space left goes to those with an intrinsic max track sizing function
/// — none here, so to all of them, equally.
#[test]
fn space_beyond_the_limits_goes_to_every_track_when_none_is_intrinsic() {
    let items = [Span::new(0, 2)];
    let three = Track::with(MinFn::Auto, MaxFn::Fixed(3));
    let two = Track::with(MinFn::Auto, MaxFn::Fixed(2));
    assert_eq!(
        sized(
            vec![three, two],
            &items,
            vec![(10, 10, 10)],
            Space::Definite(10)
        ),
        [6, 4]
    );
}

/// §11.5 step 4: an item crossing flexible tracks grows only them, by
/// their flex factors (8 cells over 1fr / 3fr: 2 + 6); §11.7 then
/// shares a definite leftover by fr (20 over 4 fr: 5 + 15).
#[test]
fn an_item_crossing_flexible_tracks_grows_them_by_factor() {
    let items = [Span::new(0, 2)];
    let c = || vec![(8, 8, 8)];
    assert_eq!(
        sized(vec![fr(1.0), fr(3.0)], &items, c(), Space::Definite(8)),
        [2, 6]
    );
    assert_eq!(
        sized(vec![fr(1.0), fr(3.0)], &items, c(), Space::Definite(20)),
        [5, 15]
    );
}

/// §11.5 step 2 / §11.6: an `auto` track under a max-content constraint
/// grows to its growth limit, the item's max-content contribution; under
/// a min-content constraint it keeps its base size, the limited
/// min-content contribution.
#[test]
fn constraints_pick_the_contribution() {
    let items = [Span::new(0, 1)];
    let c = || vec![(2, 5, 2)];
    assert_eq!(sized(vec![auto()], &items, c(), Space::MaxContent), [5]);
    assert_eq!(sized(vec![auto()], &items, c(), Space::MinContent), [2]);
}

/// §7.2.2 / §11.5 step 2: `fit-content(4)` limits its growth limit to
/// 4 below the item's max-content contribution of 8.
#[test]
fn fit_content_caps_the_growth_limit() {
    let items = [Span::new(0, 1)];
    let track = Track::with(MinFn::Auto, MaxFn::FitContent(4));
    assert_eq!(
        sized(vec![track], &items, vec![(2, 8, 2)], Space::Definite(20)),
        [4]
    );
}

/// §11.7, indefinite free space: the flex fraction is the largest an
/// item crossing a flexible track needs — a max-content contribution of
/// 6 over `minmax(0, 1fr) minmax(0, 2fr)` is 2 per fr, so 2 + 4 — and a
/// min-content constraint makes it zero.
#[test]
fn an_indefinite_flex_fraction_fits_the_items() {
    let items = [Span::new(0, 2)];
    let c = || vec![(1, 6, 1)];
    let zero_fr = |f| Track::with(MinFn::Fixed(0), MaxFn::Flex(f));
    assert_eq!(
        sized(
            vec![zero_fr(1.0), zero_fr(2.0)],
            &items,
            c(),
            Space::MaxContent
        ),
        [2, 4]
    );
    assert_eq!(
        sized(
            vec![zero_fr(1.0), zero_fr(2.0)],
            &items,
            c(),
            Space::MinContent
        ),
        [0, 0]
    );
}

/// §11.5 steps 3.5 / 3.6: a growth limit that step 5 turns from infinite
/// to finite is infinitely growable in step 6, so a spanning item's
/// max-content space goes to it alone — not shared with a track whose
/// limit was already finite — and §11.6 grows its base there: 1 + 9.
#[test]
fn an_infinitely_growable_track_takes_the_max_content_space() {
    let items = [Span::new(0, 1), Span::new(0, 2)];
    assert_eq!(
        sized(
            vec![auto(), auto()],
            &items,
            vec![(1, 1, 1), (4, 10, 4)],
            Space::Definite(20),
        ),
        [1, 9]
    );
}

/// §11.5 step 3.3: under a max-content constraint an item spanning
/// `auto`-minimum tracks grows their base sizes to its limited
/// max-content contribution (12 over two tracks: 6 + 6), which their
/// `min-content` maxima then hold (§11.5 steps 3.5 / 5).
#[test]
fn a_max_content_constraint_grows_spanned_auto_minimums() {
    let items = [Span::new(0, 2)];
    let t = || Track::with(MinFn::Auto, MaxFn::MinContent);
    assert_eq!(
        sized(vec![t(), t()], &items, vec![(2, 12, 2)], Space::MaxContent),
        [6, 6]
    );
}

/// §11.5 step 2: under a min- or max-content constraint an `auto`
/// minimum takes the item's limited min-content contribution — capped by
/// a fixed max track sizing function (3), floored by its minimum
/// contribution (2) — not its whole min-content contribution (5).
#[test]
fn a_limited_contribution_stops_at_a_fixed_maximum() {
    let items = [Span::new(0, 1)];
    let t = Track::with(MinFn::Auto, MaxFn::Fixed(3));
    for space in [Space::MaxContent, Space::MinContent] {
        assert_eq!(
            sized(vec![t], &items, vec![(5, 8, 2)], space),
            [3],
            "{space:?}"
        );
    }
}
