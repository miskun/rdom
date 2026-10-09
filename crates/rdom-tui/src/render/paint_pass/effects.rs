//! The graphical effects a stacking context paints through a layer
//! (`group`): group `opacity` (OPACITY-1), `filter` (Filter Effects 1 §5,
//! C15-FILTER) and `backdrop-filter` (Filter Effects 2 §3).
//!
//! - **`backdrop-filter`** maps the colors of the cells behind the
//!   element's border box — the layer's copy of the backdrop, before the
//!   element paints — so the element's `opacity` fades the filtered
//!   backdrop as it fades the element. Only the backdrop root's image is
//!   filtered (Filter Effects 2 §3.1): inside a layer that tracks coverage
//!   (a filter's, an opacity group's), the cells painted in it.
//! - **`filter`** paints the element into a layer that tracks coverage,
//!   then maps the colors of the cells the element painted — a background
//!   it filled, a glyph it drew, a border it contributed — leaving the
//!   backdrop's: the color-matrix functions in order, in sRGB (§5), the
//!   terminal's default colors as the canvas of the color scheme.
//!   `drop-shadow()` shades, by `box-shadow`'s whole-cell rule, the cells
//!   at its offset from the painted ones that the element does not paint
//!   itself, in its color through the functions after it. `opacity()`
//!   multiplies the group opacity. `blur()` and `url()` draw nothing.
//!
//! A stacking context with none of them paints straight into its
//! parent's buffer: no layer, no coverage.

use rdom_core::{Dom, NodeId};

use super::layout_rect_to_grid;
use crate::ext::TuiExt;
use crate::layout::FilterList;
use crate::node::TuiNodeExt;
use crate::render::buffer::coverage::{ALL, BG, BORDER, GLYPH, SHADOW};
use crate::render::compose::{canvas_bg, canvas_fg};
use crate::render::{Buffer, Rect};
use crate::style::Color;

/// What a stacking context's layer applies.
pub(super) struct Effects<'a> {
    /// The group opacity: `opacity` times the `opacity()` filters.
    pub alpha: f32,
    /// A `filter` with something to draw.
    pub filter: Option<&'a FilterList<Color>>,
    /// A `backdrop-filter` with something to draw, and the cells behind
    /// the border box it maps.
    pub backdrop: Option<(&'a FilterList<Color>, Rect)>,
}

#[cfg(test)]
thread_local! {
    /// Layers made for a graphical effect (tests only: the cost pin).
    pub(crate) static EFFECT_LAYERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl<'a> Effects<'a> {
    /// The effects of the stacking context `root`, painting in `clip`;
    /// `None` when it has none — it paints with no layer.
    pub(super) fn of(dom: &'a Dom<TuiExt>, root: NodeId, clip: Rect) -> Option<Effects<'a>> {
        let node = dom.node(root);
        let ext = node.ext()?;
        let c = node.computed()?;
        let filter = crate::style::effects::filtered(c)
            .then_some(&c.effects.filter)
            .filter(|f| f.maps_colors() || f.drop_shadows().next().is_some());
        let alpha = c.opacity
            * if crate::style::effects::filtered(c) {
                c.effects.filter.opacity()
            } else {
                1.0
            };
        let backdrop = (crate::style::effects::backdrop_filtered(c)
            && c.effects.backdrop_filter.maps_colors())
        .then(|| layout_rect_to_grid(ext.layout, clip))
        .flatten()
        .map(|area| (&c.effects.backdrop_filter, area));
        (alpha < 1.0 || filter.is_some() || backdrop.is_some()).then_some(Effects {
            alpha,
            filter,
            backdrop,
        })
    }

    /// How many rows past the subtree's own a drop shadow can shade.
    pub(super) fn reach(&self) -> i64 {
        self.filter.map_or(0, |f| {
            f.drop_shadows()
                .map(|(_, s)| i64::from(s.offset_y.offset_cells().abs()))
                .max()
                .unwrap_or(0)
        })
    }

    /// Paint into `layer` — a copy of `parent`'s cells — through the
    /// effects: the backdrop filtered, the context painted (`paint`), its
    /// cells filtered and shaded. The group opacity is the caller's.
    pub(super) fn render(&self, parent: &Buffer, layer: &mut Buffer, paint: &impl Fn(&mut Buffer)) {
        let scheme = layer.color_scheme();
        if let Some((list, area)) = self.backdrop {
            // Filter Effects 2 §3.1: the backdrop root's image — inside a
            // layer that tracks coverage, the cells painted in it.
            let in_root = |x: u16, y: u16| {
                !parent.tracks_coverage()
                    || parent
                        .index_of(x, y)
                        .is_some_and(|i| parent.coverage_of(i) != 0)
            };
            layer.map_colors(
                area,
                |x, y, _| {
                    let inside = in_root(x, y);
                    (inside, inside, inside)
                },
                |c| filtered(list, 0, canvas_bg(c, scheme)),
                |c| filtered(list, 0, canvas_fg(c, scheme)),
            );
        }
        if self.filter.is_some() {
            layer.track_coverage();
        }
        paint(layer);
        if let Some(list) = self.filter {
            shade_drop_shadows(layer, list);
            let area = layer.area;
            layer.map_colors(
                area,
                |_, _, bits| {
                    let own = bits & ALL != 0;
                    (
                        own && bits & BG != 0,
                        own && bits & GLYPH != 0,
                        own && bits & BORDER != 0,
                    )
                },
                |c| filtered(list, 0, canvas_bg(c, scheme)),
                |c| filtered(list, 0, canvas_fg(c, scheme)),
            );
        }
    }
}

/// `c` (a definite color: the canvas resolved) through `list`'s
/// color-matrix functions from the `from`-th on; its alpha kept.
fn filtered(list: &FilterList<Color>, from: usize, c: Color) -> Color {
    let (r, g, b) = match c {
        Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => (r, g, b),
        Color::Indexed(n) => rdom_style::color::palette::xterm_rgb(n),
        // Resolved by the caller; nothing to map.
        Color::Reset => return c,
    };
    let (r, g, b) = list.filter_rgb_from(from, (r, g, b));
    let alpha = c.alpha();
    if alpha == u8::MAX {
        Color::Rgb(r, g, b)
    } else {
        Color::rgba(r, g, b, alpha)
    }
}

/// Filter Effects 1 §6 `drop-shadow()`: each shadow shades the cells at
/// its offset from the cells painted so far — the element's and the
/// earlier shadows' — that the element does not paint itself, in its
/// color through the functions after it, as `box-shadow` shades a cell.
fn shade_drop_shadows(layer: &mut Buffer, list: &FilterList<Color>) {
    let area = layer.area;
    for (k, s) in list.drop_shadows() {
        let color = filtered(list, k + 1, s.color);
        if !super::fills(color) {
            continue;
        }
        let (dx, dy) = (s.offset_x.offset_cells(), s.offset_y.offset_cells());
        let mut targets: Vec<(u16, u16)> = Vec::new();
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                let Some(i) = layer.index_of(x, y) else {
                    continue;
                };
                if layer.coverage_of(i) & (ALL | SHADOW) == 0 {
                    continue;
                }
                let (tx, ty) = (i32::from(x) + dx, i32::from(y) + dy);
                let (Ok(tx), Ok(ty)) = (u16::try_from(tx), u16::try_from(ty)) else {
                    continue;
                };
                if layer
                    .index_of(tx, ty)
                    .is_some_and(|t| layer.coverage_of(t) & ALL == 0)
                {
                    targets.push((tx, ty));
                }
            }
        }
        for (x, y) in targets {
            let Some(i) = layer.index_of(x, y) else {
                continue;
            };
            if layer.coverage_of(i) & SHADOW != 0 && layer.coverage_of(i) & ALL == 0 {
                // Shaded by an earlier shadow: the later one is beneath.
                continue;
            }
            super::shadow::shade_cell(layer, x, y, color);
            // The shade is no paint of the element's: no filter maps it.
            if let Some(c) = &mut layer.coverage {
                c[i] = SHADOW;
            }
        }
    }
}
