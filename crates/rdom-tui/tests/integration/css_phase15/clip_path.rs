//! `clip-path` (CSS Masking 1 §5, CSS Shapes 1 §3.1, C15-CLIP-PATH): a cell
//! of the element or a descendant paints, and is hit, only when its centre
//! is inside the shape; the clip makes a stacking context. `path()` and
//! `url()` clip nothing; the `mask*` properties draw nothing.

use super::*;
use rdom_tui::HitTestExt;

const PAGE: &str = "body { background-color: rgb(0, 0, 255); margin: 0 } ";

/// The `w` × `h` grid of `buf`: `#` where the background is red, `.`
/// elsewhere.
fn red_map(buf: &Buffer, w: u16, h: u16) -> Vec<String> {
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    if buf.cell(x, y).unwrap().bg == RED {
                        '#'
                    } else {
                        '.'
                    }
                })
                .collect()
        })
        .collect()
}

/// Paint a red `w` × `h` box clipped by `clip`, with a red child filling
/// it, on a blue page.
fn clipped(clip: &str, w: u16, h: u16) -> Vec<String> {
    let mut dom = doc(r#"<body><div id="c"><div id="k"></div></div></body>"#);
    let buf = paint(
        &mut dom,
        &format!(
            "{PAGE} #c {{ width: {w}; height: {h}; background-color: rgb(255, 0, 0); clip-path: {clip} }}
             #k {{ height: {h}; background-color: rgb(255, 0, 0) }}"
        ),
        w + 1,
        h + 1,
    );
    red_map(&buf, w, h)
}

/// Shapes 1 §3.1.1 `inset()`: the insets in cells, `round` corner radii
/// sampled at the cell centres.
#[test]
fn inset_clips_to_the_inset_rectangle_with_rounded_corners() {
    assert_eq!(clipped("inset(1)", 5, 3), [".....", ".###.", "....."]);
    assert_eq!(clipped("inset(0 1 0 2)", 5, 2), ["..##.", "..##."]);
    assert_eq!(
        clipped("inset(0 round 2)", 6, 4),
        [".####.", "######", "######", ".####."]
    );
}

/// §3.1.1 `circle()` / `ellipse()` / `polygon()`: a cell is in when its
/// centre is — in cells, so a circle is round in cells.
#[test]
fn circles_ellipses_and_polygons_sample_cell_centres() {
    assert_eq!(
        clipped("circle(2)", 5, 5),
        ["..#..", ".###.", "#####", ".###.", "..#.."]
    );
    assert_eq!(
        clipped("ellipse(2 1)", 6, 3),
        ["......", ".####.", "......"]
    );
    assert_eq!(
        clipped("polygon(0 0, 100% 0, 0 100%)", 4, 4),
        // A centre on the hypotenuse is outside.
        ["###.", "##..", "#...", "...."]
    );
    assert_eq!(
        clipped("circle(1 at left top)", 4, 3),
        ["#...", "....", "...."]
    );
}

/// Masking 1 §5.1: a `<geometry-box>` alone clips to that box; with a
/// shape it is the shape's reference box.
#[test]
fn a_geometry_box_is_the_clip_or_the_reference() {
    let mut dom = doc(r#"<body><div id="c"></div></body>"#);
    let buf = paint(
        &mut dom,
        &format!(
            "{PAGE} #c {{ width: 3; height: 1; padding: 1; background-color: rgb(255, 0, 0); clip-path: content-box }}"
        ),
        6,
        4,
    );
    assert_eq!(red_map(&buf, 5, 3), [".....", ".###.", "....."]);
}

/// `path()` (an SVG path rdom does not draw) and `url()` (no SVG to
/// reference) clip nothing.
#[test]
fn path_and_url_clip_nothing() {
    for clip in ["path('M 0 0 L 1 1')", "url(#x)"] {
        assert_eq!(clipped(clip, 3, 2), ["###", "###"], "{clip}");
    }
}

/// Masking 1 §5: the clipped-out area does not receive pointer events —
/// the element's nor its descendants'.
#[test]
fn the_clipped_out_area_is_not_hit() {
    let mut dom = doc(r#"<body><div id="c"><div id="k">x</div></div></body>"#);
    styled(
        &mut dom,
        &format!("{PAGE} #c {{ width: 5; height: 3; clip-path: inset(1) }} #k {{ height: 3 }}"),
        6,
        4,
    );
    let k = by_id(&dom, "k");
    assert_eq!(dom.hit_test(2, 1), Some(k));
    assert_ne!(dom.hit_test(0, 0), Some(k));
    assert_ne!(dom.hit_test(0, 0), Some(by_id(&dom, "c")));
}

/// Masking 1 §5.1: a `clip-path` other than `none` — `url()` too — makes a
/// stacking context.
#[test]
fn a_clip_path_makes_a_stacking_context() {
    for clip in ["inset(0)", "url(#x)"] {
        let mut dom = doc(r#"<div id="t"><div id="k">k</div></div>"#);
        let buf = paint(
            &mut dom,
            &format!(
                "#t {{ clip-path: {clip}; background-color: rgb(0, 0, 255); height: 2 }}
                 #k {{ position: relative; z-index: -1; background-color: rgb(255, 0, 0); width: 1; height: 1 }}"
            ),
            6,
            3,
        );
        assert_eq!(buf.cell(0, 0).unwrap().bg, RED, "{clip}");
    }
}

/// Masking 1 §6–§7: a cell has no alpha to mask by — `mask*` draws
/// nothing (the declarations are kept, DIVERGENCES).
#[test]
fn masks_draw_nothing() {
    assert_eq!(
        {
            let mut dom = doc(r#"<body><div id="c"></div></body>"#);
            let buf = paint(
                &mut dom,
                &format!(
                    "{PAGE} #c {{ width: 3; height: 1; background-color: rgb(255, 0, 0); mask: url(m.svg) no-repeat; mask-border: url(b.svg) 30 }}"
                ),
                4,
                2,
            );
            red_map(&buf, 3, 1)
        },
        ["###"]
    );
}

/// CSS 2.1 §11.1.2: `clip` clips an absolutely positioned box to the
/// rectangle its offsets give from the border box's top-left corner (an
/// `auto` edge the border box's own); on any other box it does nothing.
/// The Tailwind v3 / Bootstrap visually-hidden pattern (`width: 1px` is
/// dropped as geometry) must not show its text (C15G-LEGACY-CLIP).
#[test]
fn legacy_clip_hides_the_visually_hidden_pattern() {
    let mut dom = doc(r#"<body><p id="p">ab<span id="sr">secret</span>cd</p></body>"#);
    let css = "body { margin: 0 } p { margin: 0; position: relative }
        #sr { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
              overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border-width: 0 }";
    let sheet = rdom_css::parse(css);
    dom.set_viewport(Viewport::new(20, 2));
    dom.cascade(&sheet.stylesheet);
    dom.layout_dom(Rect::new(0, 0, 20, 2));
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    let shown = format!("{}{}", row(&buf, 0), row(&buf, 1));
    assert!(!shown.contains('s') && !shown.contains('t'), "{shown:?}");
    assert!(shown.starts_with("abcd"), "{shown:?}");
    let sr = by_id(&dom, "sr");
    let r = dom.node(sr).layout_rect().unwrap();
    assert_ne!(
        dom.hit_test(r.x as u16 + 1, r.y as u16),
        Some(sr),
        "not hit"
    );
}

/// A partial `rect()` keeps the cells inside it: `rect(0, 3, 1, 1)` on a
/// 4 × 2 absolute box keeps columns 1–2 of row 0; `auto` edges are the
/// border box's; and a box that is not absolutely positioned is not
/// clipped.
#[test]
fn legacy_clip_keeps_the_rectangle_on_absolute_boxes_only() {
    for (position, want) in [
        ("absolute", vec![".##.....", "........"]),
        ("relative", vec!["####....", "####...."]),
    ] {
        let mut dom = doc(r#"<body><div id="c"></div></body>"#);
        let buf = paint(
            &mut dom,
            &format!(
                "{PAGE} #c {{ position: {position}; top: 0; left: 0; width: 4; height: 2;
                   background-color: rgb(255, 0, 0); clip: rect(0, 3, 1, 1) }}"
            ),
            8,
            2,
        );
        assert_eq!(red_map(&buf, 8, 2), want, "{position}");
    }
    let mut dom = doc(r#"<body><div id="c"></div></body>"#);
    let buf = paint(
        &mut dom,
        &format!(
            "{PAGE} #c {{ position: absolute; top: 0; left: 0; width: 4; height: 2;
               background-color: rgb(255, 0, 0); clip: rect(auto auto 1 2) }}"
        ),
        8,
        2,
    );
    assert_eq!(red_map(&buf, 8, 2), vec!["..##....", "........"]);
}
