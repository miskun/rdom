//! The exclusion-area API the inline packer consumes (CSS 2.1 §9.5):
//! line boxes beside floats are shortened, a line with no room for its
//! first word moves down past them, and a float met in the inline
//! content is placed on the current line when it fits beside what is
//! already there, else at the top of the next (§9.5.1 rule 6: never above
//! a line holding earlier content). Rows and columns are the inline
//! formatting context's: rows from its top, columns from its content
//! box's left edge.

use rdom_core::Dom;

use super::area::ExclusionArea;
use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::box_tree::BoxItem;

/// What the line packer asks of the floats beside its lines.
pub(crate) trait LineExclusions {
    /// The columns a line box whose top is row `top` may use: its start
    /// (from the content box's left edge) and width.
    fn band(&self, top: u16) -> (i32, u16);
    /// The first row below `top` where a float beside it ends — where a
    /// line too narrow for its content can move down to.
    fn next_change(&self, top: u16) -> Option<u16>;
    /// Place the float `item`, met in the inline content on the line whose
    /// top is row `top` and which already holds `used` cells (`None`: an
    /// empty line). `false` when it does not fit beside them: the packer
    /// then places it at the next line's top.
    fn place_float(&mut self, item: BoxItem, top: u16, used: Option<u16>) -> bool;
}

/// [`LineExclusions`] over a block formatting context's area, for an
/// inline formatting context whose content box starts at `(x, y)` and is
/// `width` cells wide, in a block container whose content box's top is
/// row `content_top`. Collects the floats it places, with their border
/// boxes, for the layout pass to lay out and settle
/// ([`InlineFloats::into_placed`], `float::lay_out`).
pub(crate) struct InlineFloats<'a> {
    dom: &'a Dom<TuiExt>,
    area: &'a mut ExclusionArea,
    x: i32,
    y: i32,
    width: u16,
    content_top: i32,
    placed: Vec<super::PlacedFloat>,
}

impl<'a> InlineFloats<'a> {
    pub(crate) fn new(
        dom: &'a Dom<TuiExt>,
        area: &'a mut ExclusionArea,
        content: LayoutRect,
        content_top: i32,
    ) -> Self {
        Self {
            dom,
            area,
            x: content.x,
            y: content.y,
            width: content.width,
            content_top,
            placed: Vec::new(),
        }
    }

    /// The floats placed, in document order, with their border boxes.
    pub(crate) fn into_placed(self) -> Vec<super::PlacedFloat> {
        self.placed
    }

    fn row(&self, top: u16) -> i32 {
        self.y + i32::from(top)
    }
}

impl LineExclusions for InlineFloats<'_> {
    fn band(&self, top: u16) -> (i32, u16) {
        let band = self
            .area
            .band(self.x, self.x + i32::from(self.width), self.row(top), 1);
        (
            band.start - self.x,
            band.width().clamp(0, i32::from(u16::MAX)) as u16,
        )
    }

    fn next_change(&self, top: u16) -> Option<u16> {
        let next = self.area.next_bottom(self.row(top), 1)?;
        u16::try_from(next - self.y).ok()
    }

    fn place_float(&mut self, item: BoxItem, top: u16, used: Option<u16>) -> bool {
        let y = self.row(top);
        let fb = super::size::FloatBox::of(self.dom, item, self.width);
        if let Some(used) = used {
            // Beside the line's content only if the line's band has room
            // for it after that content, and no `clear` takes it lower.
            let room = self.band(top).1.saturating_sub(used);
            if room < fb.outer_width() || super::clearance_floor(self.dom, self.area, item, y) > y {
                return false;
            }
        }
        let at = super::Placement {
            y,
            x0: self.x,
            cb_width: self.width,
            content_top: self.content_top,
        };
        let placed = super::place_box(self.dom, self.area, item, &fb, at);
        self.placed.push(placed);
        true
    }
}
