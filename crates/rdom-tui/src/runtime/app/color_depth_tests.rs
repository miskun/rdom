//! C16G-COLOR-DEPTH (Phase 16 gate decision 1, API B1): an `App` emits
//! each color at its color depth — the nearest the terminal shows — and
//! the media features `color` / `color-index` / `monochrome` (Media
//! Queries 4 §6.1–§6.3) read the same depth. What a terminal of each
//! depth shows is read back through `VirtualScreen`.

use crate::render::{Terminal, TestBackend, VirtualScreen};
use crate::runtime::app::App;
use crate::{Color, ColorDepth, TuiDom};

/// `<p>x</p>` styled `css`, drawn at `depth`: the cell's colors as a
/// terminal shows them, and the bytes.
fn drawn(depth: ColorDepth, css: &str) -> (Color, Color, String) {
    let mut dom = TuiDom::new();
    let p = dom.create_element("p");
    let t = dom.create_text_node("x");
    dom.append_child(p, t).unwrap();
    let root = dom.root();
    dom.append_child(root, p).unwrap();
    let terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
    let mut app = App::with_backend(dom, rdom_css::from_css(css), terminal)
        .unwrap()
        .with_color_depth(depth);
    app.advance(0).unwrap();
    let bytes = app.terminal().backend().bytes().to_vec();
    let mut screen = VirtualScreen::new(8, 2);
    screen.apply(&bytes);
    let cell = screen.cell(0, 0).expect("on screen");
    (
        cell.fg,
        cell.bg,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

const ORANGE_ON_NAVY: &str = "p { color: rgb(255, 135, 0); background-color: rgb(0, 0, 95) }";

#[test]
fn truecolor_emits_each_color_as_written() {
    let (fg, bg, bytes) = drawn(ColorDepth::TrueColor, ORANGE_ON_NAVY);
    assert_eq!((fg, bg), (Color::Rgb(255, 135, 0), Color::Rgb(0, 0, 95)));
    assert!(bytes.contains("\x1b[38;2;255;135;0m"), "{bytes:?}");
}

/// The xterm palette's cube: orange is entry 208, navy 17.
#[test]
fn a_256_color_terminal_gets_palette_indices() {
    let (fg, bg, bytes) = drawn(ColorDepth::Ansi256, ORANGE_ON_NAVY);
    assert_eq!((fg, bg), (Color::Indexed(208), Color::Indexed(17)));
    assert!(
        !bytes.contains("38;2;") && !bytes.contains("48;2;"),
        "{bytes:?}"
    );
}

/// The 16 ANSI colors, as SGR 30–37 / 90–97 and 40–47 / 100–107.
#[test]
fn a_16_color_terminal_gets_the_ansi_codes() {
    let (fg, bg, bytes) = drawn(
        ColorDepth::Ansi16,
        "p { color: rgb(250, 10, 10); background-color: rgb(0, 0, 230) }",
    );
    assert_eq!((fg, bg), (Color::Indexed(9), Color::Indexed(4)));
    assert!(
        bytes.contains("\x1b[91m") && bytes.contains("\x1b[44m"),
        "{bytes:?}"
    );
}

/// `NO_COLOR`: no color at all, the attributes kept.
#[test]
fn no_color_emits_no_color() {
    let (fg, bg, bytes) = drawn(
        ColorDepth::NoColor,
        "p { color: rgb(250, 10, 10); background-color: navy; font-weight: bold }",
    );
    assert_eq!((fg, bg), (Color::Reset, Color::Reset));
    assert!(
        !bytes.contains("38;") && !bytes.contains("48;"),
        "{bytes:?}"
    );
    assert!(bytes.contains("\x1b[1m"), "bold stays: {bytes:?}");
}

/// Media Queries 4 §6.1–§6.3: `color`, `color-index` and `monochrome`
/// read the app's depth.
#[test]
fn the_color_media_features_follow_the_depth() {
    let css = "p { color: rgb(0, 0, 95) } \
               @media (color-index: 256) { p { color: rgb(255, 135, 0) } } \
               @media (color: 8) { p { color: rgb(0, 95, 0) } } \
               @media (monochrome) { p { font-weight: bold } }";
    let (fg, _, _) = drawn(ColorDepth::Ansi256, css);
    assert_eq!(fg, Color::Indexed(208), "color-index: 256 at 256 colors");
    let (fg, _, _) = drawn(ColorDepth::TrueColor, css);
    assert_eq!(fg, Color::Rgb(0, 95, 0), "color: 8 at 24 bits");
    let (_, _, bytes) = drawn(ColorDepth::NoColor, css);
    assert!(
        bytes.contains("\x1b[1m"),
        "monochrome with no color: {bytes:?}"
    );
}
