//! C12G-APP-CONFIG — `App`'s construction-time options are one family of
//! consuming `with_*` builders, chainable after either constructor; the
//! `&mut self` setters are for what an app changes while it runs.

use std::rc::Rc;
use std::time::Duration;

use super::{App, ControlFlow};
use crate::render::{Terminal, TestBackend};
use crate::runtime::selection::clipboard::MemoryClipboard;
use crate::runtime::url_opener::MemoryUrlOpener;
use crate::style::Stylesheet;
use crate::{ColorScheme, PointerShapes, SgrCapabilities, TuiDom};

fn app() -> App<TestBackend> {
    let terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    App::with_backend(TuiDom::new(), Stylesheet::new(), terminal).unwrap()
}

/// Every construction-time option chains as `with_*` on the constructed
/// App, and each is read back where the App exposes it.
#[test]
fn every_construction_option_is_a_with_builder() {
    let app = app()
        .with_tick_rate(Duration::from_millis(20))
        .with_tick_handler(|_| ControlFlow::Continue)
        .with_animation_frame_rate(30)
        .with_caret_blink(None)
        .with_sgr_capabilities(SgrCapabilities::EXTENDED)
        .with_pointer_shapes(PointerShapes::None)
        .with_color_scheme(ColorScheme::Light)
        .with_clipboard(Box::new(MemoryClipboard::new()))
        .with_url_opener(Rc::new(MemoryUrlOpener::new()))
        .with_import_loader(|_: &str| Ok(String::new()));
    assert_eq!(app.tick_rate, Duration::from_millis(20));
    assert!(app.on_tick.is_some());
    assert_eq!(app.animation_frame_ms, 33);
    assert_eq!(app.sgr_capabilities(), SgrCapabilities::EXTENDED);
    assert_eq!(app.color_scheme(), ColorScheme::Light);
}

/// `App::with_backend`'s defaults, as the `App` docs' table states them:
/// a 50 ms tick, ~60 fps frames, a steady caret, the backend's SGR set
/// (a `TestBackend`'s `BASIC`), no pointer shapes, the dark scheme.
#[test]
fn with_backend_defaults_match_the_table() {
    let app = app();
    assert_eq!(app.tick_rate, Duration::from_millis(50));
    assert_eq!(app.animation_frame_ms, 16);
    assert_eq!(app.sgr_capabilities(), SgrCapabilities::BASIC);
    assert_eq!(app.pointer_shapes(), PointerShapes::None);
    assert_eq!(app.color_scheme(), ColorScheme::Dark);
    assert!(app.on_tick.is_none());
}
