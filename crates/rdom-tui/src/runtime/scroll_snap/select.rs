//! Choosing a snap position (CSS Scroll Snap 1 §6.2 / §5.1) on one axis:
//! given the snap positions, where a scroll is headed and how it got
//! there, the position it comes to rest at — or none, under `proximity`
//! with no position near enough. Pure: positions are offsets on the
//! axis (`scrollTop` / `scrollLeft`), already clamped to the scroll
//! range.

/// How a scroll reached its destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    /// A scroll to a destination with no direction to honor — `scrollTo`,
    /// `scrollIntoView`, a scrollbar drag released, Home / End: the
    /// position nearest the destination.
    Nearest,
    /// A scroll by a delta from `from` — a wheel tick, an arrow or page
    /// key, `scrollBy`, a track click: a position in that direction.
    Directional { from: i32 },
}

/// rdom's `proximity` threshold (decided; CSS Scroll Snap 1 §5.1 leaves
/// it to the UA): a scroll snaps when a position is at most this many
/// cells from its destination.
pub(crate) const PROXIMITY_CELLS: i32 = 2;

/// One snap position: its offset, and whether its box has
/// `scroll-snap-stop: always` (§6.2: a scroll may not pass it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Position {
    pub(crate) offset: i32,
    pub(crate) stop: bool,
}

/// The index in `positions` of the one a scroll to `dest` comes to rest
/// at, `None` for none.
///
/// - [`Intent::Nearest`]: the position nearest `dest`.
/// - [`Intent::Directional`]: a position past `from` in the scroll's
///   direction, the one nearest `dest` — when none lies that way, the one
///   nearest `from` (the scroll stays) under `mandatory`, none under
///   `proximity`.
/// - Either way a `scroll-snap-stop: always` position between the start
///   and the chosen one (or `dest`) is not passed: the first of them is
///   chosen instead (§6.2).
/// - `mandatory` always rests at a position; `proximity` only when the
///   chosen one is within [`PROXIMITY_CELLS`] of `dest` (§5.1), a stop
///   that was passed always.
pub(crate) fn choose(
    positions: &[Position],
    dest: i32,
    intent: Intent,
    mandatory: bool,
) -> Option<usize> {
    if positions.is_empty() {
        return None;
    }
    let nearest_to = |target: i32, among: &mut dyn Iterator<Item = usize>| {
        among.min_by_key(|&i| ((positions[i].offset - target).abs(), positions[i].offset))
    };
    let (from, dir) = match intent {
        Intent::Directional { from } if dest != from => (Some(from), (dest - from).signum()),
        _ => (None, 0),
    };
    let ahead = |i: &usize| from.is_none_or(|f| (positions[*i].offset - f) * dir > 0);
    // A stop the scroll would pass on its way to `dest`.
    if let Some(f) = from {
        let passed = (0..positions.len())
            .filter(|i| positions[*i].stop && ahead(i))
            .filter(|&i| (dest - positions[i].offset) * dir >= 0);
        if let Some(i) = nearest_to(f, &mut passed.into_iter()) {
            return Some(i);
        }
    }
    let best = match nearest_to(dest, &mut (0..positions.len()).filter(ahead)) {
        Some(i) => i,
        None if mandatory => return nearest_to(from.unwrap_or(dest), &mut (0..positions.len())),
        None => return None,
    };
    // A stop between the start and the chosen position, beyond `dest`.
    if let Some(f) = from {
        let between = (0..positions.len())
            .filter(|i| positions[*i].stop && ahead(i))
            .filter(|&i| (positions[best].offset - positions[i].offset) * dir > 0);
        if let Some(i) = nearest_to(f, &mut between.into_iter()) {
            return Some(i);
        }
    }
    (mandatory || (positions[best].offset - dest).abs() <= PROXIMITY_CELLS).then_some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(offsets: &[i32]) -> Vec<Position> {
        offsets
            .iter()
            .map(|&offset| Position {
                offset,
                stop: false,
            })
            .collect()
    }

    fn offset(p: &[Position], i: Option<usize>) -> Option<i32> {
        i.map(|i| p[i].offset)
    }

    /// Non-directional: the nearest position; `proximity` only within the
    /// threshold.
    #[test]
    fn nearest() {
        let p = at(&[0, 3, 6, 9]);
        assert_eq!(offset(&p, choose(&p, 5, Intent::Nearest, true)), Some(6));
        assert_eq!(offset(&p, choose(&p, 4, Intent::Nearest, false)), Some(3));
        let far = at(&[0, 10]);
        assert_eq!(choose(&far, 5, Intent::Nearest, false), None);
        assert_eq!(
            offset(&far, choose(&far, 5, Intent::Nearest, true)),
            Some(0),
            "a tie takes the lower"
        );
    }

    /// Directional: past the start in the scroll's direction, the one
    /// nearest the destination — even when a position behind is nearer.
    #[test]
    fn directional() {
        let p = at(&[0, 3, 6, 9]);
        let down = Intent::Directional { from: 3 };
        assert_eq!(offset(&p, choose(&p, 4, down, true)), Some(6));
        assert_eq!(offset(&p, choose(&p, 8, down, true)), Some(9));
        let up = Intent::Directional { from: 6 };
        assert_eq!(offset(&p, choose(&p, 5, up, true)), Some(3));
        // Nothing ahead: `mandatory` stays, `proximity` does not snap.
        let last = Intent::Directional { from: 9 };
        assert_eq!(offset(&p, choose(&p, 10, last, true)), Some(9));
        assert_eq!(choose(&p, 10, last, false), None);
    }

    /// §6.2: a `scroll-snap-stop: always` position is not passed, by the
    /// destination or by the position chosen beyond it.
    #[test]
    fn a_stop_is_not_passed() {
        let mut p = at(&[0, 3, 6, 9]);
        p[1].stop = true;
        let from0 = Intent::Directional { from: 0 };
        assert_eq!(offset(&p, choose(&p, 8, from0, true)), Some(3));
        assert_eq!(
            offset(&p, choose(&p, 2, from0, true)),
            Some(3),
            "chosen beyond dest"
        );
        assert_eq!(
            offset(&p, choose(&p, 8, from0, false)),
            Some(3),
            "proximity too"
        );
        let from3 = Intent::Directional { from: 3 };
        assert_eq!(
            offset(&p, choose(&p, 8, from3, true)),
            Some(9),
            "past it once there"
        );
    }
}
