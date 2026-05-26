//! BFC-1 Phase 9 — block layout perf characterization.
//!
//! Confirms block layout cost is in the same league as flex on
//! equivalent content. Two shapes:
//!
//! 1. **100-paragraph document**: a `<div>` with 100 `<p>` children,
//!    each containing fixed text. Pure block-flow normal flow.
//! 2. **100-item flex column**: the same 100 children laid out under
//!    `flex-direction: column` for comparison.
//!
//! Run with: `cargo bench -p rdom-tui --bench block_layout`. The
//! relative numbers matter more than the absolute timings.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use rdom_tui::layout::Size;
use rdom_tui::prelude::*;

fn build_block_paragraphs(n: usize) -> (TuiDom, Stylesheet) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("container");
    dom.set_attribute(container, "class", "doc").unwrap();
    for _ in 0..n {
        let p = dom.create_element("para");
        let t = dom.create_text_node("Lorem ipsum dolor sit amet, consectetur adipiscing elit.");
        dom.append_child(p, t).unwrap();
        dom.append_child(container, p).unwrap();
    }
    dom.append_child(root, container).unwrap();
    // No `display: flex` → defaults to block flow.
    let sheet = Stylesheet::new().rule_unchecked(
        "para",
        TuiStyle::new()
            .display(Display::Block)
            .height(Size::Fixed(1)),
    );
    (dom, sheet)
}

fn build_flex_column_paragraphs(n: usize) -> (TuiDom, Stylesheet) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("container");
    dom.set_attribute(container, "class", "doc").unwrap();
    for _ in 0..n {
        let p = dom.create_element("para");
        let t = dom.create_text_node("Lorem ipsum dolor sit amet, consectetur adipiscing elit.");
        dom.append_child(p, t).unwrap();
        dom.append_child(container, p).unwrap();
    }
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "container",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column),
        )
        .rule_unchecked("para", TuiStyle::new().height(Size::Fixed(1)));
    (dom, sheet)
}

fn bench_block_layout_100_paragraphs(c: &mut Criterion) {
    let (mut dom, sheet) = build_block_paragraphs(100);
    dom.cascade(&sheet);
    let viewport = Rect::new(0, 0, 80, 60);

    c.bench_function("block_layout_100_paragraphs", |b| {
        b.iter(|| {
            // Re-layout from a clean state every iteration —
            // measures the full block pass.
            dom.layout_dom(black_box(viewport));
        })
    });
}

fn bench_flex_layout_100_paragraphs(c: &mut Criterion) {
    let (mut dom, sheet) = build_flex_column_paragraphs(100);
    dom.cascade(&sheet);
    let viewport = Rect::new(0, 0, 80, 60);

    c.bench_function("flex_column_layout_100_paragraphs", |b| {
        b.iter(|| {
            dom.layout_dom(black_box(viewport));
        })
    });
}

/// Stress the margin-collapse accumulator + outer-margin walkers.
/// 50 block siblings, each with non-zero `margin-top` and
/// `margin-bottom`. Margins collapse pairwise.
fn bench_block_margin_collapse_50_siblings(c: &mut Criterion) {
    use rdom_tui::layout::{Margin, MarginValue};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("container");
    dom.set_attribute(container, "class", "stack").unwrap();
    for _ in 0..50 {
        let p = dom.create_element("para");
        dom.append_child(container, p).unwrap();
    }
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::new().rule_unchecked(
        "para",
        TuiStyle::new()
            .display(Display::Block)
            .height(Size::Fixed(1))
            .margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(2),
                MarginValue::Cells(0),
            )),
    );
    dom.cascade(&sheet);
    let viewport = Rect::new(0, 0, 80, 200);

    c.bench_function("block_margin_collapse_50_siblings", |b| {
        b.iter(|| {
            dom.layout_dom(black_box(viewport));
        })
    });
}

/// Stress the parent-first-child upward propagation chain.
/// 20-level nest of collapse-eligible blocks; each level has a
/// `margin-top`. Exercises `accumulate_outer_top_margin`'s
/// recursive walk on every layout pass.
fn bench_block_deep_collapse_chain(c: &mut Criterion) {
    use rdom_tui::layout::{Margin, MarginValue};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut parent = root;
    for _ in 0..20 {
        let lvl = dom.create_element("lvl");
        dom.append_child(parent, lvl).unwrap();
        parent = lvl;
    }
    let leaf = dom.create_element("leaf");
    dom.append_child(parent, leaf).unwrap();
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "lvl",
            TuiStyle::new().display(Display::Block).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(2),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "leaf",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        );
    dom.cascade(&sheet);
    let viewport = Rect::new(0, 0, 80, 200);

    c.bench_function("block_deep_collapse_chain_20_levels", |b| {
        b.iter(|| {
            dom.layout_dom(black_box(viewport));
        })
    });
}

criterion_group!(
    benches,
    bench_block_layout_100_paragraphs,
    bench_flex_layout_100_paragraphs,
    bench_block_margin_collapse_50_siblings,
    bench_block_deep_collapse_chain,
);
criterion_main!(benches);
