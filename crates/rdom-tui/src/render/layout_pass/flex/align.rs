//! Cross-axis self-alignment of flex items — `align-items` /
//! `align-self` (CSS Flexbox §8.3, §9.4 steps 8 and 11; CSS Box
//! Alignment 3 §6, §9): what each item of a line does on the cross axis,
//! and how far a line's baseline-aligned items reach.
//!
//! An item's alignment is its `align-self`, or its container's
//! `align-items` when that is `auto`. `normal` behaves as `stretch` for
//! a flex item (Box Alignment §6.1); `stretch` stretches only an item
//! with an `auto` cross size (§9.4 step 11 — any other is placed as
//! `flex-start`). `baseline` / `first baseline` and `last baseline` group
//! a row's items whose cross margins are not `auto`: their first (last)
//! baseline rows share a row, the group flush with the line's block-
//! start (block-end) edge — the fallback alignment's side, `self-start`
//! (`self-end`), which is a row's block axis whatever `wrap-reverse`
//! says. A column has no baseline on its cross (inline) axis: there the
//! baseline values fall back to `safe self-start` / `safe self-end`.

use rdom_core::Dom;

use super::cross::{CrossSpace, ResolvedMain, baseline_box};
use crate::ext::TuiExt;
use crate::layout::{Align, Alignment, Direction, OverflowAlign, TextDirection};
use crate::render::layout_pass::items::{BaselineBox, Item};
use crate::style::ComputedStyle;

/// Where an item goes on its line's cross axis, in the frame whose
/// origin is the line's cross-start edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CrossAlign {
    /// Fill the line (an `auto` cross size), else as `Start`.
    Stretch,
    Start,
    End,
    /// Centered, the leading space rounded down.
    Center,
    /// The border box at this offset from the line's cross-start edge
    /// (baseline alignment).
    At(i32),
}

/// An item's cross alignment and whether it is `safe` (§4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ItemAlign {
    pub(super) align: CrossAlign,
    pub(super) safe: bool,
}

/// `item`'s self-alignment in its flex container `container`: its
/// `align-self`, or `align-items` for `auto`.
fn self_alignment(container: &ComputedStyle, item: &ComputedStyle) -> Alignment {
    if item.align_self.keyword == Align::Auto {
        container.align_items
    } else {
        item.align_self
    }
}

/// The container facts the cross-axis keywords map through.
#[derive(Debug, Clone, Copy)]
pub(super) struct CrossFrame {
    pub(super) direction: Direction,
    /// The cross axis runs from its physical end (`AxisFlip::cross`).
    pub(super) flipped: bool,
    /// The container's `direction: rtl`.
    pub(super) rtl: bool,
}

impl CrossFrame {
    /// The frame placement of a box aligned to the physical start (top
    /// of a row's cross axis, left of a column's) or end.
    fn physical(self, at_start: bool) -> CrossAlign {
        if at_start != self.flipped {
            CrossAlign::Start
        } else {
            CrossAlign::End
        }
    }

    /// `start` / `end` of the axis by the container's writing mode (§4.2):
    /// a row's cross axis is its block axis (top first), a column's its
    /// inline axis (right first under `rtl`).
    fn writing_start(self, start: bool) -> CrossAlign {
        let left_or_top = match self.direction {
            Direction::Row => start,
            Direction::Column => start != self.rtl,
        };
        self.physical(left_or_top)
    }

    /// `self-start` / `self-end` by the item's own writing mode.
    fn self_start(self, item: &ComputedStyle, start: bool) -> CrossAlign {
        let left_or_top = match self.direction {
            Direction::Row => start,
            Direction::Column => start != (item.text_direction == TextDirection::Rtl),
        };
        self.physical(left_or_top)
    }
}

/// One item's plan before its line's cross size is known.
#[derive(Debug, Clone, Copy)]
enum Plan {
    Placed(ItemAlign),
    FirstBaseline(BaselineBox),
    LastBaseline(BaselineBox),
}

/// A line's cross-axis plan: each item's alignment, the baseline groups
/// waiting for the line's cross size.
pub(super) struct LinePlan {
    plans: Vec<Plan>,
    /// The first-baseline group's shared row from the line's top.
    first_above: i32,
    /// The last-baseline group's largest extent below its shared row.
    last_below: i32,
    /// How far the baseline groups reach (§9.4 step 8).
    extent: i32,
}

/// What `plan_line` needs to measure an item.
pub(super) struct PlanItem {
    pub(super) item: Item,
    pub(super) main: ResolvedMain,
    /// The item is a strut (`ChildMain::strut`): no alignment, no
    /// baseline — placed at the line's cross-start.
    pub(super) strut: Option<u16>,
}

impl LinePlan {
    /// Plan `items` — one line — in `parent`'s `frame`; `space` and
    /// `cb_width` are what their baseline boxes measure against.
    pub(super) fn new(
        dom: &Dom<TuiExt>,
        parent: &ComputedStyle,
        frame: CrossFrame,
        items: &[PlanItem],
        space: CrossSpace,
        cb_width: u16,
    ) -> Self {
        let mut plans = Vec::with_capacity(items.len());
        for item in items {
            if item.strut.is_some() {
                plans.push(Plan::Placed(ItemAlign {
                    align: CrossAlign::Start,
                    safe: false,
                }));
                continue;
            }
            let computed = item.item.computed(dom);
            let value = self_alignment(parent, &computed);
            let auto_margins = match frame.direction {
                Direction::Row => computed.margin.top.is_auto() || computed.margin.bottom.is_auto(),
                Direction::Column => {
                    computed.margin.left.is_auto() || computed.margin.right.is_auto()
                }
            };
            let baseline = matches!(value.keyword, Align::Baseline | Align::LastBaseline);
            plans.push(
                if baseline && frame.direction == Direction::Row && !auto_margins {
                    let b = baseline_box(dom, &item.item, cb_width, space, item.main);
                    if value.keyword == Align::Baseline {
                        Plan::FirstBaseline(b)
                    } else {
                        Plan::LastBaseline(b)
                    }
                } else {
                    Plan::Placed(placed(value, &computed, frame))
                },
            );
        }
        let first: Vec<&BaselineBox> = plans
            .iter()
            .filter_map(|p| match p {
                Plan::FirstBaseline(b) => Some(b),
                _ => None,
            })
            .collect();
        let last: Vec<&BaselineBox> = plans
            .iter()
            .filter_map(|p| match p {
                Plan::LastBaseline(b) => Some(b),
                _ => None,
            })
            .collect();
        let first_above = first.iter().map(|b| b.above_first()).max().unwrap_or(0);
        let first_extent = first
            .iter()
            .map(|b| first_above - b.above_first() + b.outer())
            .max()
            .unwrap_or(0);
        let last_above = last.iter().map(|b| b.above_last()).max().unwrap_or(0);
        let last_below = last
            .iter()
            .map(|b| b.outer() - b.above_last())
            .max()
            .unwrap_or(0);
        let last_extent = if last.is_empty() {
            0
        } else {
            last_above + last_below
        };
        Self {
            plans,
            first_above,
            last_below,
            extent: first_extent.max(last_extent),
        }
    }

    /// How far the line's baseline groups reach on the cross axis: a
    /// multi-line container's line is at least this large (§9.4 step 8).
    pub(super) fn baseline_extent(&self) -> u16 {
        self.extent.clamp(0, i32::from(u16::MAX)) as u16
    }

    /// Each item's alignment once the line is `line` cells across, in
    /// the frame (`flipped`: measured from the physical bottom).
    pub(super) fn resolve(&self, line: u16, flipped: bool) -> Vec<ItemAlign> {
        let line = i32::from(line);
        let shared_last = line - self.last_below;
        self.plans
            .iter()
            .map(|p| {
                let (top, b) = match p {
                    Plan::Placed(a) => return *a,
                    // The border box's top row, from the line's top.
                    Plan::FirstBaseline(b) => (self.first_above - i32::from(b.first), b),
                    Plan::LastBaseline(b) => (shared_last - i32::from(b.last), b),
                };
                let at = if flipped {
                    line - top - i32::from(b.height)
                } else {
                    top
                };
                ItemAlign {
                    align: CrossAlign::At(at),
                    safe: false,
                }
            })
            .collect()
    }
}

/// A non-baseline self-alignment's placement (§6.1, §4.2).
fn placed(value: Alignment, item: &ComputedStyle, frame: CrossFrame) -> ItemAlign {
    let safe = value.overflow == OverflowAlign::Safe;
    let align = match value.keyword {
        Align::Normal | Align::Stretch | Align::Auto => CrossAlign::Stretch,
        Align::FlexStart => CrossAlign::Start,
        Align::FlexEnd => CrossAlign::End,
        // CSS Anchor Positioning 1 §3.4: `center` for a box that is not
        // absolutely positioned.
        Align::Center | Align::AnchorCenter => CrossAlign::Center,
        Align::Start => frame.writing_start(true),
        Align::End => frame.writing_start(false),
        Align::SelfStart => frame.self_start(item, true),
        Align::SelfEnd => frame.self_start(item, false),
        // §9.3: no baseline on the axis — `safe self-start` / `self-end`.
        Align::Baseline => {
            return ItemAlign {
                align: frame.self_start(item, true),
                safe: true,
            };
        }
        Align::LastBaseline => {
            return ItemAlign {
                align: frame.self_start(item, false),
                safe: true,
            };
        }
        // Not in the self-alignment grammars (the distributions; `left`
        // / `right` are inline-axis only).
        Align::Left
        | Align::Right
        | Align::SpaceBetween
        | Align::SpaceAround
        | Align::SpaceEvenly => CrossAlign::Start,
        // `Align` is non-exhaustive (DESIGN): a keyword added to it must be
        // mapped here — the workspace's tests catch one that is not.
        _ => {
            debug_assert!(false, "unmapped `Align` keyword {:?}", value.keyword);
            CrossAlign::Start
        }
    };
    ItemAlign { align, safe }
}
