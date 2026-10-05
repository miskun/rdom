//! Smooth scrolling and a live `<style>` element.
//!
//! The log has `scroll-behavior: smooth`, so paging it from the
//! keyboard and the Top / Bottom buttons animate (CSSOM View §4.1).
//! Top calls `scrollIntoView()` on the first entry; Bottom calls the
//! legacy `scrollIntoView(false)` on the last, aligning it to the
//! bottom edge. Theme rewrites the text of the demo's own `<style>`
//! element, and the entries restyle on the next frame (HTML §4.2.6:
//! the style block is updated when its child text changes).

use std::io;

use rdom_parser::parse_into;
use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiAccessorsMut, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="scroll-style">
  <style>.scroll-style .log p { color: rgb(120, 200, 255); }</style>
  <p class="hint">Page the log (smooth)  •  Top / Bottom: scrollIntoView  •  Theme: edit the style</p>
  <div class="log" tabindex="0">
    <p>entry 01: boot</p>
    <p>entry 02: config loaded</p>
    <p>entry 03: listening</p>
    <p>entry 04: request /</p>
    <p>entry 05: request /about</p>
    <p>entry 06: cache miss</p>
    <p>entry 07: cache fill</p>
    <p>entry 08: request /</p>
    <p>entry 09: cache hit</p>
    <p>entry 10: tick</p>
    <p>entry 11: tick</p>
    <p>entry 12: request /api</p>
    <p>entry 13: slow query</p>
    <p>entry 14: retry</p>
    <p>entry 15: ok</p>
    <p>entry 16: tick</p>
    <p>entry 17: request /</p>
    <p>entry 18: cache hit</p>
    <p>entry 19: shutdown requested</p>
    <p>entry 20: bye</p>
  </div>
  <div class="controls"><button class="top">Top</button><button class="bottom">Bottom</button><button class="theme">Theme</button></div>
  <p class="theme-name">theme: cool</p>
</div>"#;

pub const CSS: &str = r#"
.scroll-style, .scroll-style *, .scroll-style *::before, .scroll-style *::after {
  box-sizing: border-box;
}
.scroll-style {
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.scroll-style .log {
  height: 6;
  width: 40;
  overflow-y: auto;
  scroll-behavior: smooth;
  border: rounded;
}
.scroll-style .controls {
  display: flex;
  flex-direction: row;
  gap: 1;
}
"#;

/// The two themes Theme alternates between: the whole text of the
/// demo's `<style>` element.
pub const THEMES: [(&str, &str); 2] = [
    (
        "cool",
        ".scroll-style .log p { color: rgb(120, 200, 255); }",
    ),
    (
        "warm",
        ".scroll-style .log p { color: rgb(255, 180, 120); }",
    ),
];

/// The demo's elements a driver (or a test) needs.
#[derive(Debug, Clone, Copy)]
pub struct Parts {
    pub root: NodeId,
    pub log: NodeId,
    pub first: NodeId,
    pub last: NodeId,
    pub top: NodeId,
    pub bottom: NodeId,
    pub theme: NodeId,
    pub theme_name: NodeId,
}

pub fn build(dom: &mut TuiDom) -> NodeId {
    build_parts(dom).root
}

pub fn build_parts(dom: &mut TuiDom) -> Parts {
    let host = dom.create_element("div");
    parse_into(dom, MARKUP, host).expect("template parses");
    let root = dom.node(host).first_element_child().unwrap().id();
    let one = |dom: &TuiDom, sel: &str| {
        dom.node(root)
            .query_selector(sel)
            .unwrap_or_else(|| panic!("{sel} is in the markup"))
            .id()
    };
    let log = one(dom, ".log");
    let parts = Parts {
        root: host,
        log,
        first: dom.node(log).first_element_child().unwrap().id(),
        last: dom.node(log).last_element_child().unwrap().id(),
        top: one(dom, ".top"),
        bottom: one(dom, ".bottom"),
        theme: one(dom, ".theme"),
        theme_name: one(dom, ".theme-name"),
    };
    let style = one(dom, "style");
    wire(dom, parts, style);
    parts
}

fn wire(dom: &mut TuiDom, parts: Parts, style: NodeId) {
    let click = ListenerOptions::default();
    let first = parts.first;
    dom.add_event_listener(parts.top, "click", click.clone(), move |ctx| {
        // `scrollIntoView()`: block start, inline nearest.
        // The log entries are never removed, so the call cannot fail.
        ctx.dom
            .node_mut(first)
            .scroll_into_view()
            .expect("the first entry is a live element");
    })
    .unwrap();
    let last = parts.last;
    dom.add_event_listener(parts.bottom, "click", click.clone(), move |ctx| {
        // `scrollIntoView(false)`: block end, inline nearest.
        ctx.dom
            .node_mut(last)
            .scroll_into_view_with(false.into())
            .expect("the last entry is a live element");
    })
    .unwrap();
    let theme_name = parts.theme_name;
    dom.add_event_listener(parts.theme, "click", click, move |ctx| {
        let current = ctx.dom.node(style).text_content();
        let next = THEMES
            .iter()
            .find(|(_, css)| *css != current)
            .expect("two themes");
        ctx.dom
            .node_mut(style)
            .set_text_content(next.1)
            .expect("a style element takes text");
        ctx.dom
            .node_mut(theme_name)
            .set_text_content(&format!("theme: {}", next.0))
            .expect("the theme line takes text");
    })
    .unwrap();
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct ScrollLiveStyle;

impl Demo for ScrollLiveStyle {
    fn slug(&self) -> &'static str {
        "animations/scroll-live-style"
    }

    fn title(&self) -> &'static str {
        "Smooth scroll + live style"
    }

    fn category(&self) -> Category {
        Category::Animations
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        stylesheet()
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
