//! Regression: when the source disclosure is OPEN and the active
//! demo's intrinsic content is taller than its allotted
//! `.view-content` slot, demo descendants must NOT paint into the
//! source disclosure's rect.
//!
//! Reproduced visually as:
//!
//! ```text
//! │ ▾ Source                                                 ┃ │
//! │ Markup 符也可以选择。CJK graphemes snap to full width.   ┃ │  <- BUG
//! │ <div class="selectable-text-demo">                       ┃ │
//! ```
//!
//! The `<h3>Markup</h3>` only paints its 6 leading cells; the
//! remaining cells on that row retain the demo's CJK paragraph
//! because `.view-content` defaults to `overflow: visible` and lets
//! the demo's overflowing children paint outside its layout box.
//!
//! Contract: the source disclosure's rect must be free of demo
//! glyphs — specifically the CJK character `符` only appears in
//! `selectable_text`'s CJK paragraph, not in MARKUP / CSS source.

use rdom_showcase::demos::selectable_text::SelectableText;
use rdom_showcase::{Demo, build_shell, shell::base_stylesheet};
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, PaintExt, TuiDom, TuiNodeExt};

#[test]
fn open_source_does_not_show_demo_overflow_glyphs() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    // Pick the selectable-text demo: its intrinsic content (h1 +
    // chrome + multi-line prose + code + CJK + gaps + padding) is
    // taller than .view-content at this viewport, guaranteeing
    // overflow when source is open.
    let demo = SelectableText;
    let demo_root = demo.build(&mut dom);
    dom.append_child(handles.main, demo_root).unwrap();

    // Open the source disclosure programmatically — mirrors what
    // the UA's click handler does.
    dom.set_attribute(handles.source_disclosure, "open", "")
        .unwrap();

    let base = base_stylesheet();
    let demo_sheet = demo.stylesheet();
    let sheets: Vec<&_> = vec![&base, &demo_sheet];

    // 89 × 28 matches the user-reported repro viewport.
    let viewport = Rect::new(0, 0, 89, 28);
    dom.cascade_all(&sheets);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    let disclosure_layout = dom
        .node(handles.source_disclosure)
        .layout_rect()
        .expect("source-disclosure must have a layout after layout_dom");

    // Scan every cell inside the source disclosure's rect for the
    // CJK glyph `符` — present only in the demo's CJK paragraph,
    // never in the MARKUP / CSS source strings.
    let bleed_char = "符";
    let x0 = disclosure_layout.x.max(0) as u16;
    let y0 = disclosure_layout.y.max(0) as u16;
    let x1 = (x0 + disclosure_layout.width).min(viewport.width);
    let y1 = (y0 + disclosure_layout.height).min(viewport.height);

    let mut bleed_at: Vec<(u16, u16)> = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(cell) = buf.cell(x, y)
                && cell.symbol() == bleed_char
            {
                bleed_at.push((x, y));
            }
        }
    }

    if !bleed_at.is_empty() {
        // Dump the disclosure region for diagnostic purposes.
        eprintln!(
            "source disclosure rect = ({}, {}, w={}, h={})",
            disclosure_layout.x,
            disclosure_layout.y,
            disclosure_layout.width,
            disclosure_layout.height
        );
        for y in y0..y1 {
            let mut line = String::new();
            for x in x0..x1 {
                if let Some(cell) = buf.cell(x, y) {
                    if cell.is_spacer() {
                        continue;
                    }
                    line.push_str(cell.symbol());
                } else {
                    line.push('?');
                }
            }
            eprintln!("y={y:>2}: {line}");
        }
        panic!(
            "demo content overflowed into source disclosure region: \
             '{bleed_char}' appears at {bleed_at:?} — \
             .view-content must clip its descendants"
        );
    }
}
