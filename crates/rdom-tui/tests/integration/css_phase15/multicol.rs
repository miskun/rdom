//! Multi-column layout (CSS Multi-column 1, CSS Fragmentation 3,
//! C15-COLUMNS): the column boxes' count and width (§3.4), the content
//! fragmented into them — balanced (§7.1) or filled in turn — with the
//! break rules (Fragmentation 3 §3–§4), a box split across columns drawn
//! as its fragments, and hit-testing the fragments.

use super::*;
use rdom_tui::{HitTestExt, TuiAccessors};

const PAGE: &str = "body { margin: 0 } ";

/// The `w` × `h` grid of `buf`: `#` where the background is red.
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

/// Rows `0 .. h` of `buf` as text, trailing spaces kept.
fn rows(buf: &Buffer, h: u16) -> Vec<String> {
    (0..h).map(|y| row(buf, y)).collect()
}

/// §3.4 (02) and §7.1: three columns share the width — `(20 − 2·1) / 3 =
/// 6` cells — and an `auto` height balances six lines two to a column.
#[test]
fn column_count_shares_the_width_and_balances_the_lines() {
    let mut dom = doc(
        r#"<body><div id="m">aaaaaa bbbbbb cccccc dddddd eeeeee ffffff</div><div id="n">next</div></body>"#,
    );
    let buf = paint(
        &mut dom,
        &format!("{PAGE} #m {{ column-count: 3; column-gap: 1 }}"),
        20,
        4,
    );
    assert_eq!(
        rows(&buf, 3),
        [
            "aaaaaa cccccc eeeeee",
            "bbbbbb dddddd ffffff",
            "next                ",
        ]
    );
    assert_eq!(rect(&dom, "m"), (0, 0, 20, 2));
}

/// §3.4 (03): as many columns of at least `column-width` as fit —
/// `⌊(26 + 1) / (8 + 1)⌋ = 3` — widened to fill: `27 / 3 − 1 = 8`.
/// `column-gap: normal` is one cell.
#[test]
fn column_width_decides_the_count() {
    let mut dom = doc(
        r#"<body><div id="m">aaaaaaaa bbbbbbbb cccccccc dddddddd eeeeeeee ffffffff</div></body>"#,
    );
    let buf = paint(&mut dom, &format!("{PAGE} #m {{ column-width: 8 }}"), 26, 3);
    assert_eq!(
        rows(&buf, 2),
        ["aaaaaaaa cccccccc eeeeeeee", "bbbbbbbb dddddddd ffffffff"]
    );
}

/// §7.1: `column-fill: auto` under a constrained height fills each column
/// in turn; `balance` (the initial value) evens them out; columns past the
/// count overflow in the inline direction (§8.2).
#[test]
fn column_fill_auto_fills_in_turn_and_extra_columns_overflow() {
    let blocks = |n: usize| {
        let items: String = (1..=n).map(|i| format!("<div>a{i}</div>")).collect();
        doc(&format!(r#"<body><div id="m">{items}</div></body>"#))
    };
    let css = |fill: &str, h: u16| {
        format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11; height: {h}; column-fill: {fill} }}"
        )
    };
    let mut dom = blocks(4);
    let buf = paint(&mut dom, &css("auto", 3), 20, 3);
    assert_eq!(
        rows(&buf, 3),
        [
            "a1    a4            ",
            "a2                  ",
            "a3                  "
        ]
    );
    let mut dom = blocks(4);
    let buf = paint(&mut dom, &css("balance", 3), 20, 3);
    assert_eq!(
        rows(&buf, 2),
        ["a1    a3            ", "a2    a4            "]
    );
    // Six blocks in two-row columns: a third column past the box's edge.
    let mut dom = blocks(6);
    let buf = paint(&mut dom, &css("auto", 2), 20, 2);
    assert_eq!(
        rows(&buf, 2),
        ["a1    a3    a5      ", "a2    a4    a6      "]
    );
}

/// Fragmentation 3 §5.4 (`slice`): a block split across two columns is
/// drawn as two fragments — its background on its rows in each column,
/// not across the gap — and reports both (`client_rects`), its bounding
/// box the `layout_rect`.
#[test]
fn a_split_box_draws_and_reports_its_fragments() {
    let mut dom = doc(r#"<body><div id="m"><p id="p">x1<br>x2<br>x3<br>x4</p></div></body>"#);
    let buf = paint(
        &mut dom,
        &format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 9 }}
             p {{ margin: 0; background-color: rgb(255, 0, 0) }}"
        ),
        10,
        3,
    );
    assert_eq!(
        red_map(&buf, 10, 3),
        ["####.####.", "####.####.", ".........."]
    );
    let p = by_id(&dom, "p");
    let frags: Vec<_> = dom
        .node(p)
        .client_rects()
        .iter()
        .map(|r| (r.x, r.y, r.width, r.height))
        .collect();
    assert_eq!(frags, [(0, 0, 4, 2), (5, 0, 4, 2)]);
    assert_eq!(rect(&dom, "p"), (0, 0, 9, 2));
}

/// Fragmentation 3 §4.1: a monolithic box (here a scroll container) has no
/// break inside — it moves whole to the next column; §3.1 `break-before:
/// column` forces a break.
#[test]
fn monolithic_boxes_move_whole_and_forced_breaks_break() {
    let mut dom = doc(
        r#"<body><div id="m"><div>t1<br>t2</div><div id="box">m1<br>m2<br>m3</div></div></body>"#,
    );
    paint(
        &mut dom,
        &format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11; height: 4; column-fill: auto }}
             #box {{ overflow: hidden }}"
        ),
        12,
        4,
    );
    assert_eq!(rect(&dom, "box"), (6, 0, 5, 3));
    // Balanced, the three would sit two and one; forced, one and two.
    let mut dom =
        doc(r#"<body><div id="m"><div>u1</div><div id="b">u2</div><div>u3</div></div></body>"#);
    paint(
        &mut dom,
        &format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11 }}
             #b {{ break-before: column }}"
        ),
        12,
        2,
    );
    assert_eq!(rect(&dom, "b"), (6, 0, 5, 1));
}

/// Fragmentation 3 §3.3, §4.4 rule 3: a break leaves no fewer than
/// `widows` lines after it — four lines in three-row columns break after
/// the second, not the third.
#[test]
fn widows_move_the_break_up() {
    let html = r#"<body><div id="m"><p id="p">w1<br>w2<br>w3<br>w4</p></div></body>"#;
    let css = |extra: &str| {
        format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11; height: 3; column-fill: auto }}
             p {{ margin: 0 }} {extra}"
        )
    };
    let mut dom = doc(html);
    let buf = paint(&mut dom, &css(""), 12, 3);
    assert_eq!(rows(&buf, 2), ["w1    w3    ", "w2    w4    "]);
    let mut dom = doc(html);
    let buf = paint(&mut dom, &css("p { widows: 1 }"), 12, 3);
    assert_eq!(
        rows(&buf, 3),
        ["w1    w4    ", "w2          ", "w3          "]
    );
}

/// A point on a fragment hits the split box (its text, in its column); a
/// point in its bounding box but on no fragment hits the multi-column
/// container.
#[test]
fn hit_testing_follows_the_fragments() {
    let mut dom = doc(
        r#"<body><div id="m"><div id="i">intro</div><p id="p">h1<br>h2<br>h3<br>h4</p></div></body>"#,
    );
    paint(
        &mut dom,
        &format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 13; height: 3; column-fill: auto }}
             p {{ margin: 0 }}"
        ),
        14,
        3,
    );
    let (m, p) = (by_id(&dom, "m"), by_id(&dom, "p"));
    assert_eq!(dom.hit_test(0, 1), Some(p));
    assert_eq!(dom.hit_test(7, 0), Some(p));
    assert_eq!(dom.hit_test(7, 2), Some(m));
    assert_eq!(dom.hit_test(0, 0), Some(by_id(&dom, "i")));
}

/// §2: under `direction: rtl` the first column is the rightmost.
#[test]
fn rtl_columns_run_from_the_right() {
    let mut dom = doc(r#"<body><div id="m">r1<br>r2</div></body>"#);
    let buf = paint(
        &mut dom,
        &format!("{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11; direction: rtl }}"),
        11,
        1,
    );
    assert_eq!(row(&buf, 0), "   r2    r1");
}

/// A multi-column container sized by its content outside block flow (a
/// flex item): its block size is its content shared out over its columns
/// — four lines in two columns, two rows.
#[test]
fn a_multicol_flex_item_is_as_tall_as_its_columns() {
    let mut dom = doc(r#"<body><div id="f"><div id="m">f1<br>f2<br>f3<br>f4</div></div></body>"#);
    let buf = paint(
        &mut dom,
        &format!(
            "{PAGE} #f {{ display: flex; flex-direction: column }}
             #m {{ column-count: 2; column-gap: 1; width: 11 }}"
        ),
        11,
        3,
    );
    assert_eq!(rect(&dom, "m").3, 2);
    assert_eq!(rows(&buf, 2), ["f1    f3   ", "f2    f4   "]);
}
