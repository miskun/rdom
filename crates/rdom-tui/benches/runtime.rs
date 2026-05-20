//! rdom-tui runtime benchmarks.
//!
//! Core workloads:
//!
//! - **Hit test on a 10k-node tree** — cold hit-test throughput.
//! - **Dispatch depth-50** — capture → bubble walk cost on a deep
//!   chain of listeners.
//! - **Full-frame cascade + layout + paint + diff** — end-to-end
//!   steady-state at 80×24 and 200×60.
//! - **Range serialization on a 10k-cell selection** — clipboard
//!   serialize cost, relevant for long selections in a log view.
//!
//! Plus two comparative workloads:
//!
//! - **Scrollable-list mutation** — 1k / 10k rows, scroll by one
//!   row per iteration; proxies steady-state redraw cost.
//! - **Unicode paragraph wrap** — 10k-grapheme wrapped paragraph;
//!   exercises the inline formatting context.
//!
//! Run a single bench:
//!
//! ```text
//! cargo bench -p rdom-tui --bench runtime -- hit_test
//! ```
//!
//! All benches target the production code paths: same `cascade`,
//! `layout_dom`, `paint_dom` that `App::run` uses each frame. The
//! sink is a `Buffer` owned by the bench — no terminal I/O.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use rdom_core::{Event, ListenerOptions, Position, Range};
use rdom_tui::prelude::*;
use rdom_tui::runtime::selection::clipboard::serialize_selection;

// ── Shared fixtures ─────────────────────────────────────────────────

/// Build a tree with `n_rows` rows under a single list container.
/// Each row has a nested inline span — gives the layout pass
/// something to walk through.
fn build_list_dom(n_rows: usize) -> (TuiDom, Stylesheet) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("list");

    for i in 0..n_rows {
        let row = dom.create_element("row");
        let t = dom.create_text_node(&format!("row {i:05}: content"));
        dom.append_child(row, t).unwrap();
        let span = dom.create_element("span");
        dom.append_child(row, span).unwrap();
        dom.append_child(list, row).unwrap();
    }
    dom.append_child(root, list).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "list",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .rule_unchecked("row", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    (dom, sheet)
}

/// Build a chain `depth` deep: `root → n1 → n2 → ... → nd`. Attach
/// a no-op listener on every node so dispatch walks the full chain.
/// Returns `(dom, leaf)`.
fn build_dispatch_chain(depth: usize) -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mut cursor = root;
    for _ in 0..depth {
        let el = dom.create_element("node");
        dom.append_child(cursor, el).unwrap();
        dom.add_event_listener(el, "ping", ListenerOptions::default(), |_| {})
            .unwrap();
        cursor = el;
    }
    (dom, cursor)
}

// ── 1. Hit test on a 10k-node tree ──────────────────────────────────

fn bench_hit_test_10k(c: &mut Criterion) {
    let (mut dom, sheet) = build_list_dom(10_000);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    // Hit somewhere in the middle of the visible viewport.
    c.bench_function("hit_test_10k_nodes", |b| {
        b.iter(|| {
            black_box(dom.hit_test(black_box(20), black_box(12)));
        })
    });
}

// ── 2. Dispatch depth-50 ────────────────────────────────────────────

fn bench_dispatch_depth_50(c: &mut Criterion) {
    let (mut dom, leaf) = build_dispatch_chain(50);
    c.bench_function("dispatch_capture_bubble_depth_50", |b| {
        b.iter(|| {
            let mut ev = Event::new("ping");
            let _ = dom.dispatch_event(black_box(leaf), &mut ev);
            black_box(&ev);
        })
    });
}

// ── 3. Full-frame render + diff ────────────────────────────────────

fn bench_full_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_frame");

    // Terminal sizes: a typical small (80×24) and a modern large
    // (200×60). Both benchmark the full cascade + layout + paint
    // pipeline on a representative list.
    for (w, h) in [(80u16, 24u16), (200u16, 60u16)] {
        let viewport = Rect::new(0, 0, w, h);
        // A list sized to roughly fill the viewport's height.
        let (mut dom, sheet) = build_list_dom((h as usize) * 2);
        // Prime: run one frame so `layout_dirty` is cleared; then
        // each iteration re-runs the pipeline on a steady state.
        dom.cascade(&sheet);
        dom.layout_dom(viewport);
        let mut buf = Buffer::empty(viewport);
        dom.paint_dom(&mut buf, viewport);

        group.bench_with_input(
            BenchmarkId::new("cascade_layout_paint", format!("{w}x{h}")),
            &(viewport,),
            |b, &(vp,)| {
                b.iter(|| {
                    dom.cascade(&sheet);
                    dom.layout_dom(vp);
                    let mut buf = Buffer::empty(vp);
                    dom.paint_dom(&mut buf, vp);
                    black_box(buf);
                })
            },
        );
    }
    group.finish();
}

// ── 4. Range serialization on 10k-cell selection ────────────────────

fn bench_serialize_10k_cells(c: &mut Criterion) {
    // Build a single text node with ~10k ASCII bytes inside an IFC
    // paragraph. Select the full range; serialize.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let text: String = "The quick brown fox jumps over the lazy dog. ".repeat(230); // ~10_350 bytes
    let t = dom.create_text_node(&text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(200)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 220, 60));

    let range = Range::ordered_unchecked(Position::new(t, 0), Position::new(t, text.len()));

    c.bench_function("serialize_selection_10k_cells", |b| {
        b.iter(|| {
            black_box(serialize_selection(&dom, black_box(&range)));
        })
    });
}

// ── 5. Scrollable-list steady-state mutation ────────────────────────

fn bench_scroll_list_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("scroll_list");

    for n_rows in [1_000usize, 10_000usize] {
        let (mut dom, sheet) = build_list_dom(n_rows);
        let viewport = Rect::new(0, 0, 80, 24);
        dom.cascade(&sheet);
        dom.layout_dom(viewport);

        group.bench_with_input(
            BenchmarkId::new("mutate_first_row_and_repaint", n_rows),
            &n_rows,
            |b, _| {
                // Each iteration: mutate row 0's text node, re-cascade
                // (subtree), layout, paint. Represents "one tick of
                // steady-state UI updates."
                let row0_text: NodeId = first_row_text(&dom);
                let mut counter = 0u64;
                b.iter(|| {
                    counter = counter.wrapping_add(1);
                    let _ = dom
                        .node_mut(row0_text)
                        .set_node_value(&format!("row 00000: tick {counter}"));
                    dom.cascade(&sheet);
                    dom.layout_dom(viewport);
                    let mut buf = Buffer::empty(viewport);
                    dom.paint_dom(&mut buf, viewport);
                    black_box(buf);
                })
            },
        );
    }
    group.finish();
}

/// Walk to the first row's text node.
fn first_row_text(dom: &TuiDom) -> NodeId {
    let root = dom.root();
    let list = dom.node(root).first_child().unwrap().id();
    let row0 = dom.node(list).first_child().unwrap().id();
    dom.node(row0).first_child().unwrap().id()
}

// ── 6. Unicode paragraph wrapping ──────────────────────────────────

fn bench_unicode_paragraph_wrap(c: &mut Criterion) {
    // A paragraph with mixed CJK + ASCII + emoji — many grapheme
    // clusters, lots of cell-width decisions in the inline packer.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let mut text = String::new();
    for _ in 0..500 {
        text.push_str("Hello 世界 🦀 ");
    }
    let t = dom.create_text_node(&text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(80)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    c.bench_function("unicode_paragraph_wrap_10k_graphemes", |b| {
        b.iter(|| {
            dom.cascade(&sheet);
            dom.layout_dom(Rect::new(0, 0, 80, 200));
            black_box(());
        })
    });
}

criterion_group!(
    benches,
    bench_hit_test_10k,
    bench_dispatch_depth_50,
    bench_full_frame,
    bench_serialize_10k_cells,
    bench_scroll_list_mutation,
    bench_unicode_paragraph_wrap,
);
criterion_main!(benches);
