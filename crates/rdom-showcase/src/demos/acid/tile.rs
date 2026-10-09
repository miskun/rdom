//! [`Tile`] — one labelled square of the acid page.

use rdom_tui::{NodeId, TuiDom};

/// One acid tile: a feature *interaction* rendered into a fixed rectangle
/// of one acid page (`specs/ACID.md` ground rule 3).
///
/// The tile's box is the crop the reference describes: an absolutely
/// positioned `div.acid-tile.<class>` at `(x, y)` on its page, `w` × `h`
/// cells, clipping its content (`overflow: clip`), its own stacking context
/// (`z-index: 0`) and the containing block of its positioned descendants.
/// The label sits on the row above it and is not part of the crop.
///
/// The reference — what the spec says the crop shows — lives with the
/// tests (`crates/rdom-showcase/tests/integration/acid/`), never here:
/// it is derived from the spec, not from this markup's output.
#[derive(Copy, Clone, Debug)]
pub struct Tile {
    /// `ACID.md`'s tile number (`"1"`, `"9a"`).
    pub id: &'static str,
    /// The tile's name, printed in its label.
    pub title: &'static str,
    /// The tile's class, which scopes every rule of [`Tile::css`]
    /// (`acid-t1`).
    pub class: &'static str,
    /// The acid page the tile is on, from 1.
    pub page: u8,
    /// Column of the tile's left edge on its page.
    pub x: u16,
    /// Row of the tile's top edge on its page (its label is the row above).
    pub y: u16,
    /// Width in cells.
    pub w: u16,
    /// Height in rows.
    pub h: u16,
    /// The tile's content, parsed into its box.
    pub markup: &'static str,
    /// Rules for the page's main author sheet — the sheet the `App` is
    /// built with.
    pub css: &'static str,
    /// Rules for a second author sheet pushed after the main one
    /// (`App::push_stylesheet`), for the cascade-order contests that need
    /// a later sheet; empty for most tiles.
    pub late_css: &'static str,
    /// Script the tile runs on its box after the page is built — what the
    /// static markup cannot say (a custom highlight's ranges, built from
    /// Rust as a page's script builds them); `None` for most tiles.
    pub setup: Option<fn(&mut TuiDom, NodeId)>,
    /// Script the tile runs on its box once the page has laid out and
    /// painted its first frame — a page's load handler, for what needs
    /// layout (a scroll offset is clamped to the laid-out range); the page
    /// is painted again after it. `None` for most tiles.
    pub script: Option<fn(&mut TuiDom, NodeId)>,
}
