//! C7-GRID-ALIGN — Box Alignment in grid layout (CSS Grid 2 §10, CSS
//! Box Alignment 3): each item aligned in its grid area by `justify-self`
//! / `align-self` (defaulted by `justify-items` / `align-items`), `auto`
//! margins (Grid §10.2), `safe` / `unsafe` (Box Alignment §4.4), and the
//! tracks distributed in the container by `justify-content` /
//! `align-content` (Grid §10.5).

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// A grid `.g` holding one `div.<class>` per entry (text `ab`), laid out
/// with `css` at 20 × 12. Returns each item's border box `(x, y, width,
/// height)`.
fn place(css: &str, classes: &[&str]) -> Vec<(i32, i32, u16, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids: Vec<NodeId> = classes
        .iter()
        .map(|c| {
            let s = el(&mut dom, g, "div", c);
            let t = dom.create_text_node("ab");
            dom.append_child(s, t).unwrap();
            s
        })
        .collect();
    lay_out(&mut dom, css, 20, 12);
    ids.iter()
        .map(|&id| {
            let r = rect(&dom, id);
            (r.x, r.y, r.width, r.height)
        })
        .collect()
}

/// One item in a 10 × 5 area (`.g`'s only track on each axis), aligned
/// by `item` (its declarations) inside a container styled by `container`.
fn one(container: &str, item: &str) -> (i32, i32, u16, u16) {
    place(
        &format!(
            ".g {{ display: grid; grid-template-columns: 10; grid-template-rows: 5; \
             {container} }} .i {{ {item} }}"
        ),
        &["i"],
    )[0]
}

/// Box Alignment §6.1 / Grid §10.3: `justify-self` places the item in
/// its area's inline axis — an `auto` width sized `fit-content` (CSS
/// Grid 2 §6.2) unless the value stretches; `normal` behaves as
/// `stretch`; `left` / `right` are physical; `auto` takes the
/// container's `justify-items`.
#[test]
fn justify_self_places_an_item_on_the_inline_axis() {
    let x = |item: &str| {
        let (x, _, w, _) = one("", item);
        (x, w)
    };
    assert_eq!(x("justify-self: normal"), (0, 10));
    assert_eq!(x("justify-self: stretch"), (0, 10));
    assert_eq!(x("justify-self: start"), (0, 2));
    assert_eq!(x("justify-self: end"), (8, 2));
    assert_eq!(x("justify-self: center"), (4, 2));
    assert_eq!(x("justify-self: right"), (8, 2));
    assert_eq!(x("justify-self: flex-end"), (8, 2));
    let (x, _, w, _) = one("justify-items: center", "");
    assert_eq!((x, w), (4, 2));
    let (x, _, w, _) = one("justify-items: legacy right", "");
    assert_eq!((x, w), (8, 2));
}

/// §4.2 with Writing Modes 4 §2.1: under `direction: rtl` the inline
/// axis starts at the right — the column is the container's right half,
/// `start` its right edge, `left` stays physical — and `self-start`
/// reads the item's own direction.
#[test]
fn justify_self_follows_the_writing_mode() {
    let x = |item: &str| one("direction: rtl", item).0;
    assert_eq!(x("justify-self: start"), 18);
    assert_eq!(x("justify-self: end"), 10);
    assert_eq!(x("justify-self: left"), 10);
    assert_eq!(x("justify-self: self-start; direction: ltr"), 10);
}

/// Box Alignment §6.2 / Grid §10.4: `align-self` places the item in its
/// area's block axis — an `auto` height its content's unless the value
/// stretches; `auto` takes `align-items`.
#[test]
fn align_self_places_an_item_on_the_block_axis() {
    let y = |item: &str| {
        let (_, y, _, h) = one("", item);
        (y, h)
    };
    assert_eq!(y("align-self: normal"), (0, 5));
    assert_eq!(y("align-self: stretch"), (0, 5));
    assert_eq!(y("align-self: start"), (0, 1));
    assert_eq!(y("align-self: end"), (4, 1));
    assert_eq!(y("align-self: center"), (2, 1));
    assert_eq!(y("align-self: self-end"), (4, 1));
    let (_, y, _, h) = one("align-items: end", "");
    assert_eq!((y, h), (4, 1));
    // `place-self: <align> <justify>` sets both.
    let (x, y, w, h) = one("", "place-self: end center");
    assert_eq!((x, y, w, h), (4, 4, 2, 1));
}

/// Box Alignment §6.1: `stretch` stretches only an `auto` size; an item
/// with a definite size "behaves as `flex-start`", the area's start.
#[test]
fn a_sized_item_is_aligned_not_stretched() {
    assert_eq!(one("", "width: 4; height: 2"), (0, 0, 4, 2));
    assert_eq!(
        one(
            "",
            "width: 4; height: 2; justify-self: end; align-self: center"
        ),
        (6, 1, 4, 2)
    );
}

/// Grid §10.2: "auto margins absorb positive free space prior to
/// alignment via the box alignment properties" — one `auto` margin
/// pushes the item to the far side, two center it, whatever
/// `justify-self` / `align-self` say; with no free space they are 0.
#[test]
fn auto_margins_absorb_the_free_space() {
    assert_eq!(
        one(
            "",
            "margin-left: auto; justify-self: start; align-self: start"
        ),
        (8, 0, 2, 1)
    );
    assert_eq!(
        one("", "margin: auto; justify-self: end; align-self: end"),
        (4, 2, 2, 1)
    );
    assert_eq!(one("", "margin-top: auto"), (0, 4, 10, 1));
    assert_eq!(one("", "margin-left: auto; width: 14"), (0, 0, 14, 5));
}

/// Box Alignment §4.4: an item larger than its area overflows as its
/// alignment says (`end` past the start edge, `center` on both sides),
/// unless the value is `safe`, which aligns it as `start` then.
#[test]
fn safe_alignment_keeps_an_overflowing_item_at_the_start() {
    let x = |item: &str| one("", &format!("width: 14; {item}")).0;
    assert_eq!(x("justify-self: end"), -4);
    assert_eq!(x("justify-self: unsafe end"), -4);
    assert_eq!(x("justify-self: safe end"), 0);
    assert_eq!(x("justify-self: center"), -2);
    assert_eq!(x("justify-self: safe center"), 0);
    let y = |item: &str| one("", &format!("height: 7; {item}")).1;
    assert_eq!(y("align-self: end"), -2);
    assert_eq!(y("align-self: safe end"), 0);
}

/// CSS Grid 2 §6.2: under `normal`, an item with a preferred aspect
/// ratio is not stretched — it is "sized consistent with the size
/// calculation rules for block-level elements": its `auto` width fills
/// the area, its `auto` height follows the ratio, at the area's start.
/// `stretch` stretches it on both axes.
#[test]
fn an_item_with_an_aspect_ratio_is_sized_as_a_block() {
    let ratio = |item: &str| {
        place(
            &format!(
                ".g {{ display: grid; grid-template-columns: 10; grid-template-rows: 8 }} \
                 .i {{ aspect-ratio: 2; {item} }}"
            ),
            &["i"],
        )[0]
    };
    assert_eq!(ratio(""), (0, 0, 10, 5));
    assert_eq!(ratio("align-self: stretch"), (0, 0, 10, 8));
    assert_eq!(ratio("align-self: end"), (0, 3, 10, 5));
}

/// Grid §9.1 with Box Alignment §6.1 / §6.2 (C6G-DOCS's abspos
/// self-alignment): an absolutely positioned child whose containing block
/// is a grid area aligns in that area — `justify-self: center` with an
/// `auto` width is a centered fit-content box, `align-self: end` sits on
/// the area's bottom.
#[test]
fn an_absolutely_positioned_box_aligns_in_its_grid_area() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let abs = el(&mut dom, g, "div", "abs");
    let t = dom.create_text_node("ab");
    dom.append_child(abs, t).unwrap();
    lay_out(
        &mut dom,
        ".g { display: grid; position: relative; grid-template-columns: 2 6 4; \
         grid-template-rows: 1 4 } \
         .abs { position: absolute; inset: 0; grid-area: 2 / 2 / 3 / 3; \
         justify-self: center; align-self: end }",
        20,
        6,
    );
    let r = rect(&dom, abs);
    assert_eq!((r.x, r.y, r.width, r.height), (4, 4, 2, 1));
}

/// The `x` of each of `.g`'s items — one per column of `columns` —
/// with `justify-content: value`, in a 20-wide container.
fn columns_at(columns: &str, value: &str) -> Vec<i32> {
    let n = columns.split_whitespace().count();
    let classes = vec![""; n];
    place(
        &format!(
            ".g {{ display: grid; grid-template-columns: {columns}; justify-content: {value} }}"
        ),
        &classes,
    )
    .iter()
    .map(|r| r.0)
    .collect()
}

/// Grid §10.5 / Box Alignment §5.1: `justify-content` places the grid's
/// columns in the container's free space as one alignment subject per
/// track — packed to an edge or the center, or with the space between
/// them (`space-between`), around them (`space-around`: half at each
/// end) or evenly; a remainder cell goes to the earliest spaces
/// (DIVERGENCES §1). `normal` / `stretch` stretch `auto` tracks only
/// (§11.8), so fixed tracks stay at the start.
#[test]
fn justify_content_distributes_the_columns() {
    assert_eq!(columns_at("2 2", "normal"), [0, 2]);
    assert_eq!(columns_at("2 2", "stretch"), [0, 2]);
    assert_eq!(columns_at("2 2", "start"), [0, 2]);
    assert_eq!(columns_at("2 2", "end"), [16, 18]);
    assert_eq!(columns_at("2 2", "flex-end"), [16, 18]);
    assert_eq!(columns_at("2 2", "center"), [8, 10]);
    assert_eq!(columns_at("2 2", "space-between"), [0, 18]);
    assert_eq!(columns_at("2 2", "space-around"), [4, 14]);
    assert_eq!(columns_at("2 2", "space-evenly"), [6, 13]);
    // §5.3: one track falls back — `space-between` to `flex-start`,
    // `space-around` / `space-evenly` to `center`.
    assert_eq!(columns_at("2", "space-between"), [0]);
    assert_eq!(columns_at("2", "space-evenly"), [9]);
}

/// Grid §10.5: the distributed space widens the gutters — on top of the
/// gap, and inside the grid area of an item spanning the tracks it lies
/// between.
#[test]
fn distributed_space_widens_the_gutters_and_spanning_areas() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2 2 2; column-gap: 1; \
         justify-content: space-between } .s { grid-column: 1 / 3 }",
        &["", "", "", "s"],
    );
    // 20 − 6 − 2 gaps = 12 free: columns at 0, 2 + 1 + 6 = 9, 18.
    assert_eq!(
        at.iter().map(|r| (r.0, r.2)).collect::<Vec<_>>(),
        [(0, 2), (9, 2), (18, 2), (0, 11)]
    );
}

/// Box Alignment §4.2 / §4.4: under `direction: rtl` the columns pack to
/// the right for `start` and to the left for `end`, `left` / `right` are
/// physical; tracks wider than the container overflow as their
/// alignment says unless `safe`.
#[test]
fn justify_content_follows_the_writing_mode_and_safety() {
    let rtl = |value: &str| {
        place(
            &format!(
                ".g {{ display: grid; direction: rtl; grid-template-columns: 2 2; \
                 justify-content: {value} }}"
            ),
            &["", ""],
        )
        .iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
    };
    assert_eq!(rtl("start"), [18, 16]);
    assert_eq!(rtl("end"), [2, 0]);
    assert_eq!(rtl("left"), [2, 0]);
    assert_eq!(rtl("right"), [18, 16]);
    assert_eq!(columns_at("12 12", "center"), [-2, 10]);
    assert_eq!(columns_at("12 12", "safe center"), [0, 12]);
    assert_eq!(columns_at("12 12", "end"), [-4, 8]);
}

/// Grid §10.5: `align-content` distributes the rows in a container whose
/// height leaves free space (here a definite 10).
#[test]
fn align_content_distributes_the_rows() {
    let rows_at = |value: &str| {
        place(
            &format!(
                ".g {{ display: grid; height: 10; grid-template-rows: 2 2; \
                 align-content: {value} }}"
            ),
            &["", ""],
        )
        .iter()
        .map(|r| r.1)
        .collect::<Vec<_>>()
    };
    assert_eq!(rows_at("normal"), [0, 2]);
    assert_eq!(rows_at("end"), [6, 8]);
    assert_eq!(rows_at("center"), [3, 5]);
    assert_eq!(rows_at("space-between"), [0, 8]);
    assert_eq!(rows_at("space-around"), [2, 7]);
}

/// Grid §10.5 with §7.2.3.2: a collapsed `auto-fit` track is no alignment
/// subject — `space-between` spreads the two occupied columns to the
/// container's edges, not over the ten repetitions.
#[test]
fn collapsed_tracks_are_not_distributed() {
    assert_eq!(
        place(
            ".g { display: grid; grid-template-columns: repeat(auto-fit, 2); \
             justify-content: space-between }",
            &["", ""],
        )
        .iter()
        .map(|r| r.0)
        .collect::<Vec<_>>(),
        [0, 18]
    );
}
