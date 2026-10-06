//! An item's block-axis geometry for baseline alignment (CSS Box
//! Alignment 3 §9), shared by flex lines (CSS Flexbox §8.3) and grid rows
//! (CSS Grid 2 §10.4): its margins, its border-box height and its first
//! and last baseline rows.

use rdom_core::Dom;

use super::Item;
use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// An item's physical top and bottom margins, its (not stretched)
/// border-box height, and its first and last baseline rows from its
/// border-box top — the text rows of its first and last line boxes
/// (`layout_pass::baselines`), or, with none, a baseline synthesized at
/// its border box's bottom row (§9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) struct BaselineBox {
    pub(in crate::render::layout_pass) margin_top: i32,
    pub(in crate::render::layout_pass) height: u16,
    pub(in crate::render::layout_pass) margin_bottom: i32,
    pub(in crate::render::layout_pass) first: u16,
    pub(in crate::render::layout_pass) last: u16,
}

impl BaselineBox {
    /// Measure `item` (styled `computed`), laid out `width` × `height`
    /// in a containing block `cb_width` wide, with these margins.
    pub(in crate::render::layout_pass) fn measure(
        dom: &Dom<TuiExt>,
        item: &Item,
        computed: &ComputedStyle,
        (width, height): (u16, u16),
        (margin_top, margin_bottom): (i32, i32),
        cb_width: u16,
    ) -> Self {
        let synthesized = height.saturating_sub(1);
        let rows = match item {
            Item::Element(id) => crate::render::layout_pass::baselines::content_rows(
                dom, *id, computed, width, cb_width,
            ),
            Item::Anonymous(anon) => anon.content_rows(dom, width, cb_width),
        };
        let (first, last) = rows.unwrap_or((synthesized, synthesized));
        // §9.1: "for legacy reasons" a scroll container's last baselines
        // are its block-end margin edge — its scrollbar gutter and
        // clipped content aside (as an inline block's in its line,
        // `inline::vertical::atom_rows`).
        let scrolls = computed.is_scroll_container();
        let last = if scrolls {
            (i32::from(height) + margin_bottom - 1).clamp(0, i32::from(u16::MAX)) as u16
        } else {
            last
        };
        Self {
            margin_top,
            height,
            margin_bottom,
            first,
            last,
        }
    }

    /// Rows from its margin-box top to its first baseline row.
    pub(in crate::render::layout_pass) fn above_first(&self) -> i32 {
        self.margin_top + i32::from(self.first)
    }

    /// Rows from its margin-box top to its last baseline row.
    pub(in crate::render::layout_pass) fn above_last(&self) -> i32 {
        self.margin_top + i32::from(self.last)
    }

    /// Rows from its last baseline row to its margin-box bottom.
    pub(in crate::render::layout_pass) fn below_last(&self) -> i32 {
        self.outer() - self.above_last()
    }

    /// Its margin box's height.
    pub(in crate::render::layout_pass) fn outer(&self) -> i32 {
        self.margin_top + i32::from(self.height) + self.margin_bottom
    }
}
