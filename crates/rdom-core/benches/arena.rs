//! rdom-core arena benchmarks. Phase 1 scaffolding — more benchmarks
//! added as the API grows (getElementById, query_selector, etc.).

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rdom_core::Dom;

fn bench_append_child_10k(c: &mut Criterion) {
    c.bench_function("append_child x10k siblings", |b| {
        b.iter(|| {
            let mut dom: Dom = Dom::new();
            let root = dom.root();
            for _ in 0..10_000 {
                let el = dom.create_element("div");
                dom.append_child(root, el).unwrap();
            }
            black_box(dom);
        });
    });
}

fn bench_parent_node_deep(c: &mut Criterion) {
    // Build a chain 500 deep; measure walking up to the root.
    let mut dom: Dom = Dom::new();
    let mut cursor = dom.root();
    for _ in 0..500 {
        let el = dom.create_element("div");
        dom.append_child(cursor, el).unwrap();
        cursor = el;
    }
    let leaf = cursor;

    c.bench_function("parent_node walk depth 500", |b| {
        b.iter(|| {
            let mut cur = dom.node(leaf);
            let mut steps = 0;
            while let Some(p) = cur.parent_node() {
                cur = p;
                steps += 1;
            }
            black_box(steps)
        });
    });
}

fn bench_child_iter(c: &mut Criterion) {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    for _ in 0..10_000 {
        let el = dom.create_element("div");
        dom.append_child(root, el).unwrap();
    }

    c.bench_function("child_nodes iterate 10k", |b| {
        b.iter(|| {
            let n = dom.node(root).child_nodes().count();
            black_box(n)
        });
    });
}

fn bench_get_element_by_id_vs_dfs(c: &mut Criterion) {
    // Build 10k elements, assign ids to every 100th.
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let mut target = None;
    for i in 0..10_000 {
        let el = dom.create_element("div");
        dom.append_child(root, el).unwrap();
        if i % 100 == 0 {
            dom.set_attribute(el, "id", &format!("n{i}")).unwrap();
        }
        if i == 9_900 {
            target = Some(el);
        }
    }
    let _ = target;

    c.bench_function("get_element_by_id indexed (10k tree)", |b| {
        b.iter(|| {
            let found = dom.get_element_by_id("n9900");
            black_box(found)
        });
    });

    c.bench_function("get_element_by_id_within DFS (10k tree)", |b| {
        b.iter(|| {
            let found = dom.get_element_by_id_within(root, "n9900");
            black_box(found)
        });
    });
}

fn bench_get_elements_by_tag_name(c: &mut Criterion) {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    for _ in 0..5_000 {
        let el = dom.create_element("div");
        dom.append_child(root, el).unwrap();
    }
    for _ in 0..5_000 {
        let el = dom.create_element("span");
        dom.append_child(root, el).unwrap();
    }

    c.bench_function("get_elements_by_tag_name_all indexed (10k tree)", |b| {
        b.iter(|| {
            let v = dom.get_elements_by_tag_name_all("div");
            black_box(v.len())
        });
    });
    c.bench_function("get_elements_by_tag_name DFS (10k tree)", |b| {
        b.iter(|| {
            let v = dom.get_elements_by_tag_name(root, "div");
            black_box(v.len())
        });
    });
}

criterion_group!(
    benches,
    bench_append_child_10k,
    bench_parent_node_deep,
    bench_child_iter,
    bench_get_element_by_id_vs_dfs,
    bench_get_elements_by_tag_name,
);
criterion_main!(benches);
