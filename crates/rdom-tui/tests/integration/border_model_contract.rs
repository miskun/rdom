//! BORDER-MODEL-1 contract pins.
//!
//! Every test in this file is a row of the `border-collapse` × `gap`
//! truth table or a clause of CSS Tables 3 §11.5 conflict resolution.
//! Tests use minimal synthetic trees (custom element names like
//! `<row>` / `<cell>`) so the contract reads decoupled from the
//! showcase chrome.
//!
//! Tests carry `#[ignore = "BORDER-MODEL-1 Mn"]` annotations where
//! the milestone making them pass is still upstream. As each
//! milestone lands, the corresponding `#[ignore]` lines are
//! removed and the tests join the regular suite. By M9 every test
//! here runs as part of the workspace gate.

use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, PaintExt, TuiDom, TuiNodeExt};

fn pipeline(dom: &mut TuiDom, css: &str, viewport: Rect) -> Buffer {
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

fn symbol_at(buf: &Buffer, x: u16, y: u16) -> &str {
    buf.cell(x, y).map(|c| c.symbol()).unwrap_or("?")
}

// ─────────────────────────────────────────────────────────────────
// The 2×2 outcome grid (gap × collapse)
// ─────────────────────────────────────────────────────────────────

#[test]
fn gap_zero_separate_produces_adjacent_borders() {
    // Default `border-collapse: separate`, gap: 0 → bordered children
    // sit next to each other. A's right edge (col 4) and B's left
    // edge (col 5) are at DIFFERENT cells — no merge, two visible
    // verticals next to each other. This is the "tight tile" layout.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell");
    let b = dom.create_element("cell");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row  { display: flex; flex-direction: row; width: 10; height: 3; gap: 0; }
        cell { width: 5; height: 3; border: solid; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    // Top row: ┌───┐┌───┐  — A's `┐` at col 4, B's `┌` at col 5.
    assert_eq!(symbol_at(&buf, 0, 0), "┌", "A top-left");
    assert_eq!(symbol_at(&buf, 4, 0), "┐", "A top-right");
    assert_eq!(
        symbol_at(&buf, 5, 0),
        "┌",
        "B top-left adjacent (NOT merged)"
    );
    assert_eq!(symbol_at(&buf, 9, 0), "┐", "B top-right");
}

#[test]
fn gap_positive_separate_leaves_visible_gap() {
    // `gap: 1` → one empty cell between bordered children.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell");
    let b = dom.create_element("cell");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row  { display: flex; flex-direction: row; width: 11; height: 3; gap: 1; }
        cell { width: 5; height: 3; border: solid; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    // ┌───┐ ┌───┐
    assert_eq!(symbol_at(&buf, 4, 0), "┐", "A top-right at col 4");
    assert_eq!(symbol_at(&buf, 5, 0), " ", "gap cell at col 5 — visible");
    assert_eq!(symbol_at(&buf, 6, 0), "┌", "B top-left at col 6");
    assert_eq!(symbol_at(&buf, 10, 0), "┐", "B top-right at col 10");
}

#[test]
fn gap_zero_collapse_overlaps_one_cell_and_paints_junction() {
    // `border-collapse: collapse` on parent + `gap: 0` + both
    // children bordered → outer rects overlap by 1 cell at the
    // shared edge. Paint emits `┬` / `┴` / `│` at the shared column.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell");
    let b = dom.create_element("cell");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row  { display: flex; flex-direction: row; width: 9; height: 3;
               gap: 0; border-collapse: collapse; }
        cell { width: 5; height: 3; border: solid; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    // ┌───┬───┐  — shared column at col 4.
    assert_eq!(symbol_at(&buf, 0, 0), "┌", "A top-left");
    assert_eq!(symbol_at(&buf, 4, 0), "┬", "shared top junction");
    assert_eq!(symbol_at(&buf, 8, 0), "┐", "B top-right");
    assert_eq!(symbol_at(&buf, 4, 1), "│", "shared vertical");
    assert_eq!(symbol_at(&buf, 4, 2), "┴", "shared bottom junction");
}

#[test]
#[ignore = "BORDER-MODEL-1 M5 — gap must be honored under collapse"]
fn gap_positive_collapse_has_no_overlap_collapse_is_noop() {
    // `gap: 1` is sacred — collapse can't merge across a gap cell.
    // Outcome identical to `gap_positive_separate_leaves_visible_gap`.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell");
    let b = dom.create_element("cell");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row  { display: flex; flex-direction: row; width: 11; height: 3;
               gap: 1; border-collapse: collapse; }
        cell { width: 5; height: 3; border: solid; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    assert_eq!(symbol_at(&buf, 4, 0), "┐", "A top-right at col 4");
    assert_eq!(
        symbol_at(&buf, 5, 0),
        " ",
        "gap cell at col 5 — visible under collapse too"
    );
    assert_eq!(symbol_at(&buf, 6, 0), "┌", "B top-left at col 6");
    assert_eq!(symbol_at(&buf, 10, 0), "┐", "B top-right at col 10");
}

// ─────────────────────────────────────────────────────────────────
// Non-inheritance — `border-collapse` does NOT inherit
// ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "BORDER-MODEL-1 M4 — border-collapse must stop inheriting"]
fn border_collapse_does_not_inherit() {
    // Parent declares `collapse`; intermediate borderless container
    // is declaration-less; intermediate's bordered children must NOT
    // overlap (no inherited collapse).
    let mut dom: TuiDom = TuiDom::new();
    let outer = dom.create_element("outer");
    let mid = dom.create_element("mid");
    let a = dom.create_element("cell");
    let b = dom.create_element("cell");
    dom.append_child(dom.root(), outer).unwrap();
    dom.append_child(outer, mid).unwrap();
    dom.append_child(mid, a).unwrap();
    dom.append_child(mid, b).unwrap();

    let css = r#"
        outer { display: flex; flex-direction: column; width: 12; height: 5;
                border-collapse: collapse; }
        mid   { display: flex; flex-direction: row; flex: 1; }
        cell  { width: 5; height: 3; border: solid; }
    "#;
    let _buf = pipeline(&mut dom, css, Rect::new(0, 0, 14, 6));

    // mid has no `border-collapse: collapse` declaration. Even
    // though outer has it, mid does not inherit. So cells inside
    // mid don't overlap — A's right at col 4, B's left at col 5.
    let a_rect = dom.node(a).layout_rect().expect("A laid out");
    let b_rect = dom.node(b).layout_rect().expect("B laid out");
    assert_eq!(a_rect.x, 0);
    assert_eq!(a_rect.width, 5);
    assert_eq!(
        b_rect.x, 5,
        "B should sit adjacent to A (no overlap) because mid is not collapse; got {b_rect:?}"
    );
}

// ─────────────────────────────────────────────────────────────────
// CSS Tables 3 §11.5 rule 1 — `border-style: hidden` kill-switch
// ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "BORDER-MODEL-1 M3+M7 — needs BorderStyle::Hidden + paint conflict resolution"]
fn border_style_hidden_on_one_participant_suppresses_merged_edge() {
    // Two bordered siblings under collapse. A says `solid`, B says
    // `hidden` on its left edge. The shared cell renders WITHOUT a
    // vertical — hidden wins absolutely.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell_a");
    let b = dom.create_element("cell_b");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row    { display: flex; flex-direction: row; width: 9; height: 3;
                 gap: 0; border-collapse: collapse; }
        cell_a { width: 5; height: 3; border: solid; }
        cell_b { width: 5; height: 3; border: solid; border-left-style: hidden; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    // At col 4 (the shared column), A says vertical; B says hidden.
    // Hidden wins → no vertical drawn at the merged direction.
    // The cell renders `─` (or empty in the body row), NOT `│`/`┬`/`┴`.
    assert_ne!(
        symbol_at(&buf, 4, 1),
        "│",
        "hidden suppresses the shared vertical"
    );
}

#[test]
#[ignore = "BORDER-MODEL-1 M3+M7"]
fn border_style_hidden_at_any_participant_wins_over_solid_and_double() {
    // Three-way pile-on: hidden beats solid AND double.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell_a");
    let b = dom.create_element("cell_b");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row    { display: flex; flex-direction: row; width: 9; height: 3;
                 gap: 0; border-collapse: collapse; }
        cell_a { width: 5; height: 3; border: double; }
        cell_b { width: 5; height: 3; border: solid; border-left-style: hidden; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    // Even though A says `double` (a higher-rank style), B's `hidden`
    // on the same shared edge kills it.
    assert_ne!(symbol_at(&buf, 4, 1), "║", "hidden beats double");
    assert_ne!(symbol_at(&buf, 4, 1), "│", "hidden beats solid");
}

// ─────────────────────────────────────────────────────────────────
// CSS Tables 3 §11.5 rule 4 — style ranking on width tie
// ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "BORDER-MODEL-1 M3+M7 — needs BorderStyle::Double + ranking"]
fn double_beats_solid_at_shared_cell() {
    // A says `double`, B says `solid`. Shared cell renders with
    // the double-line glyph (`║` for the vertical, `╗` / `╔` etc.
    // for corners). Style rank: double > solid.
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell_a");
    let b = dom.create_element("cell_b");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row    { display: flex; flex-direction: row; width: 9; height: 3;
                 gap: 0; border-collapse: collapse; }
        cell_a { width: 5; height: 3; border: double; }
        cell_b { width: 5; height: 3; border: solid; }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    // The shared vertical is `║` (double wins).
    assert_eq!(
        symbol_at(&buf, 4, 1),
        "║",
        "double beats solid on the shared vertical"
    );
}

// ─────────────────────────────────────────────────────────────────
// CSS Tables 3 §11.5 rules 5–6 — element ranking + DOM order
// ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "BORDER-MODEL-1 M7 — needs per-direction priority tracking"]
fn child_border_wins_over_ancestor_for_color() {
    // Ancestor declares red border around the shared edge; child
    // declares blue. They land on the same cell. Child wins color
    // (CSS Tables 3 §11.5 rule 5: closer-to-cell wins).
    use rdom_tui::style::Color;
    let mut dom: TuiDom = TuiDom::new();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    dom.append_child(dom.root(), outer).unwrap();
    dom.append_child(outer, inner).unwrap();

    let css = r#"
        outer { display: flex; flex-direction: column; width: 7; height: 3;
                border: solid; border-color: rgb(200, 80, 80);
                border-collapse: collapse; }
        inner { width: 7; height: 3; border: solid;
                border-color: rgb(80, 80, 200); }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 8, 4));

    // At the shared top-left corner, both contribute. Child (inner)
    // wins — color is blue.
    assert_eq!(
        buf.cell(0, 0).unwrap().fg,
        Color::Rgb(80, 80, 200),
        "child color wins over ancestor at shared corner"
    );
}

#[test]
#[ignore = "BORDER-MODEL-1 M7"]
fn later_sibling_wins_at_shared_cell_when_both_solid() {
    // Two siblings, both solid, different colors. Their right/left
    // borders share a cell under collapse. Per CSS Tables 3 §11.5
    // rule 6 (geometric position → adapted to "later in DOM order
    // wins" for non-table elements), B wins.
    use rdom_tui::style::Color;
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell_a");
    let b = dom.create_element("cell_b");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row    { display: flex; flex-direction: row; width: 9; height: 3;
                 gap: 0; border-collapse: collapse; }
        cell_a { width: 5; height: 3; border: solid; border-color: rgb(200, 80, 80); }
        cell_b { width: 5; height: 3; border: solid; border-color: rgb(80, 80, 200); }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    assert_eq!(
        buf.cell(4, 1).unwrap().fg,
        Color::Rgb(80, 80, 200),
        "later sibling (B) wins the shared vertical's color"
    );
}

// ─────────────────────────────────────────────────────────────────
// Color provenance — winner picks BOTH glyph and color
// ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "BORDER-MODEL-1 M7"]
fn winning_border_determines_glyph_color_not_last_paint() {
    // Pin that the winner of the conflict resolution contributes
    // BOTH the glyph (style) AND the color. The substrate must not
    // pick glyph from the winner and color from last-paint
    // separately. A says double in red; B says solid in blue;
    // they share a cell. Double wins style AND red.
    use rdom_tui::style::Color;
    let mut dom: TuiDom = TuiDom::new();
    let row = dom.create_element("row");
    let a = dom.create_element("cell_a");
    let b = dom.create_element("cell_b");
    dom.append_child(dom.root(), row).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(row, b).unwrap();

    let css = r#"
        row    { display: flex; flex-direction: row; width: 9; height: 3;
                 gap: 0; border-collapse: collapse; }
        cell_a { width: 5; height: 3; border: double;
                 border-color: rgb(200, 80, 80); }
        cell_b { width: 5; height: 3; border: solid;
                 border-color: rgb(80, 80, 200); }
    "#;
    let buf = pipeline(&mut dom, css, Rect::new(0, 0, 12, 4));

    let cell = buf.cell(4, 1).expect("shared vertical cell");
    assert_eq!(cell.symbol(), "║", "double's glyph wins");
    assert_eq!(
        cell.fg,
        Color::Rgb(200, 80, 80),
        "double's color wins (NOT B's blue)"
    );
}
