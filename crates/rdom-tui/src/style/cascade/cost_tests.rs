//! What the cascade does *not* do: work counters (`ladder::probe`) for
//! the paths that must stay free.
//!
//! The rollback states `revert` / `revert-layer` read (CSS Cascade 4
//! §7.3, Cascade 5 §7.4) are needed only when such a value is met, and
//! a pseudo-element no rule matches has no declarations to walk
//! (Cascade 4 §6: the cascade orders *declarations*; with none, the
//! box does not exist — CSS Pseudo-Elements 4 §4 for `::before` /
//! `::after` without `content`).

use super::ladder::probe::{LADDER_WALKS, ROLLBACK_ALLOCS, take};
use super::*;
use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::style::Stylesheet;

fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let mut sheet = Stylesheet::new();
    for rule in parsed.stylesheet.rules() {
        sheet
            .add_rule(&rule.source_text, rule.style.clone())
            .unwrap();
    }
    sheet
}

fn one_div() -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    dom
}

/// One element, one matched rule, no `revert`, no author pseudo-element
/// rules: two ladders — the element's and its `::selection` (the UA's
/// `*::selection` matches every element) — and no rollback memo. The
/// unmatched `::before`, `::after` and `::backdrop` walk nothing.
#[test]
fn no_revert_no_pseudo_match_walks_one_ladder() {
    let mut dom = one_div();
    let css = sheet("div { color: red }");
    take(&LADDER_WALKS);
    take(&ROLLBACK_ALLOCS);
    dom.cascade(&css);
    assert_eq!(take(&LADDER_WALKS), 2, "the element and UA ::selection");
    assert_eq!(take(&ROLLBACK_ALLOCS), 0, "no revert, no rollback memo");
}

/// A `revert` materialises the memo, for the box that meets it only.
#[test]
fn revert_in_a_pseudo_allocates_its_rollback_only() {
    let mut dom = one_div();
    let css = sheet("div { color: red } div::before { content: 'x'; color: revert }");
    take(&LADDER_WALKS);
    take(&ROLLBACK_ALLOCS);
    dom.cascade(&css);
    assert_eq!(take(&LADDER_WALKS), 3, "element, ::before, ::selection");
    assert_eq!(take(&ROLLBACK_ALLOCS), 1, "only ::before met a revert");
}

/// `C2G-STATELESS-REGISTRY` — the stateless `CascadeExt` forms keep the
/// sheet set's property registry (CSS Properties and Values 1 §2) on the
/// document: two cascades with the same, unchanged sheets build it once
/// and keep each element's match record (no reallocation); a changed
/// sheet builds it again.
#[test]
fn headless_cascades_with_the_same_sheets_build_one_registry() {
    use super::registry_probe::take_builds;
    let mut dom = one_div();
    let div = dom.node(dom.root()).first_child().unwrap().id();
    let mut css = sheet("div { color: red }");
    take_builds();
    dom.cascade(&css);
    let record = dom.node(div).ext().unwrap().matched.clone().unwrap();
    dom.cascade(&css);
    dom.cascade_subtrees(&css, &[div]);
    assert_eq!(take_builds(), 1, "one registry for one sheet set");
    let again = dom.node(div).ext().unwrap().matched.clone().unwrap();
    assert!(Rc::ptr_eq(&record, &again), "the match record is kept");

    css.add_rule("p", crate::style::TuiStyle::new()).unwrap();
    dom.cascade(&css);
    assert_eq!(take_builds(), 1, "a changed sheet is a new sheet set");
    let other = sheet("div { color: blue }");
    dom.cascade_all(&[&css, &other]);
    assert_eq!(take_builds(), 1, "so is another list");
}

/// `C5G-LOGICAL-COST` — an inline-axis flow-relative property maps by
/// the element's `direction` (CSS Logical 1 §4), but the block's two
/// physical forms are built once, with the sheet: cascading an element
/// against `margin-inline-start` (and a declaration after it, which
/// keeps its order) allocates no more than against `margin-left`.
#[test]
fn a_logical_block_cascades_at_the_cost_of_its_physical_twin() {
    use crate::test_alloc::allocations_in;
    let cost = |css: &str| {
        let mut dom = one_div();
        let css = sheet(css);
        // Warm-up: the first cascade builds the per-document caches.
        dom.cascade(&css);
        allocations_in(|| dom.cascade(&css))
    };
    let physical = cost("div { margin-left: 1; color: red }");
    let logical = cost("div { margin-inline-start: 1; color: red }");
    assert!(
        logical <= physical,
        "logical {logical} allocations, physical {physical}"
    );
}

/// An element whose own `direction` differs from the inherited one runs
/// the ladder at most twice when a matched block is directional (the
/// second run with its own direction; a flow-relative property cannot
/// change `direction`).
#[test]
fn a_direction_change_reruns_the_ladder_once() {
    let mut dom = one_div();
    let css = sheet("div { direction: rtl; margin-inline-start: 1 }");
    take(&LADDER_WALKS);
    dom.cascade(&css);
    assert_eq!(
        take(&LADDER_WALKS),
        3,
        "the element twice, ::selection once"
    );
    let div = dom.node(dom.root()).first_child().unwrap().id();
    let margin = &dom.node(div).computed().unwrap().margin;
    assert_eq!(
        (margin.left.clone(), margin.right.clone()),
        (
            crate::layout::MarginValue::Cells(0),
            crate::layout::MarginValue::Cells(1)
        ),
        "rtl: inline-start is the right margin"
    );
}

/// C6G-RERUN-BOUND: the cascade runs an element's ladder again with its
/// own `direction` when a flow-relative inline property met an inherited
/// one that differs (CSS Logical 1 §4). That re-run settles it — a
/// flow-relative property cannot set `direction` — and the bound is the
/// loop's, not an assertion's: a ladder that kept changing direction
/// still runs twice, then stops with the second run.
#[test]
fn the_direction_rerun_runs_at_most_twice() {
    use crate::layout::TextDirection;
    let mut runs = 0;
    let flip = |d: TextDirection| match d {
        TextDirection::Ltr => TextDirection::Rtl,
        _ => TextDirection::Ltr,
    };
    let (last, settled) = super::element::settle_direction(TextDirection::Ltr, |d| {
        runs += 1;
        (d, flip(d))
    });
    assert_eq!((runs, last), (2, TextDirection::Rtl));
    assert!(!settled);

    let mut runs = 0;
    let (_, settled) = super::element::settle_direction(TextDirection::Ltr, |d| {
        runs += 1;
        (d, d)
    });
    assert_eq!(runs, 1);
    assert!(settled);
}

/// The cascade recurses once per tree level, so each level's frame is
/// paid as many times as the tree is deep: it keeps only the element's
/// styles behind `Rc`s, the ones it computes living in frames that are
/// gone before the children start (`walk::style_element` /
/// `finish_element`). A 400-deep chain — every level a `::before` and a
/// scroll container's pseudo-elements, the most an element computes —
/// cascades on a 1 MiB stack.
#[test]
fn a_deep_tree_cascades_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let mut dom: TuiDom = TuiDom::new();
            let mut parent = dom.root();
            for _ in 0..400 {
                let div = dom.create_element("div");
                dom.append_child(parent, div).unwrap();
                parent = div;
            }
            let css = sheet("div { color: red; overflow: auto } div::before { content: 'x' }");
            dom.cascade(&css);
            assert!(dom.node(parent).computed().is_some());
        })
        .unwrap()
        .join()
        .expect("the cascade fits the stack");
}

/// `C7G-INITIAL-ALLOC` — every element's cascade starts from
/// `ComputedStyle::initial()` (CSS Cascade 4 §7.1), so the initial
/// values must cost nothing: `grid-auto-columns` / `grid-auto-rows`'
/// initial `auto` (CSS Grid 2 §7.6) is a shared static list, not a
/// fresh `Vec` per element. Building the initial style allocates only
/// what it did before grid — the custom-property map's `Rc` (CSS
/// Variables 1 §2: an empty map, shared by everything that inherits it)
/// — and cloning it allocates nothing. A plain element pays for grid only
/// when it is one.
#[test]
fn the_initial_style_allocates_nothing_for_grid() {
    use crate::test_alloc::allocations_in;
    let mut initial = None;
    let built = allocations_in(|| initial = Some(ComputedStyle::initial()));
    assert_eq!(
        built, 1,
        "ComputedStyle::initial() allocations (`vars` only)"
    );
    let initial = initial.unwrap();
    let cloned = allocations_in(|| drop(std::hint::black_box(initial.clone())));
    assert_eq!(cloned, 0, "a clone of the initial style allocations");
}

/// C7G-MINOR — a restyle (`Mode::Restyle`) that leaves a `display:
/// contents` element's style unchanged keeps its subtree, as it does any
/// element's, unless its children's parent box (CSS Display 3 §2.5: the
/// one above it) changed whether it lays out flex or grid items — what
/// their blockification reads (§2.7, `apply_tests::
/// a_restyle_unblockifies_the_children_of_a_contents_item`). Here it did
/// not: the restyle visits the `contents` element alone, not its 100
/// children.
#[test]
fn an_unchanged_contents_element_keeps_its_subtree_in_a_restyle() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("div");
    dom.set_attribute(c, "class", "c").unwrap();
    dom.append_child(root, c).unwrap();
    for _ in 0..100 {
        let span = dom.create_element("span");
        dom.append_child(c, span).unwrap();
    }
    let css = sheet(".c { display: contents }");
    dom.cascade(&css);
    let sheets = [&css];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    super::walk::probe::take();
    restyle_vars(&mut dom, &sheets, registry, &[c]);
    let visits = super::walk::probe::take();
    assert!(visits <= 2, "visited {visits} nodes");
}

/// The allocations of cascading a `.p` parent styled `css` holding `n`
/// plain `div` children.
fn cascade_allocations(css: &str, n: usize) -> u64 {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("section");
    dom.set_attribute(p, "class", "p").unwrap();
    dom.append_child(root, p).unwrap();
    for _ in 0..n {
        let div = dom.create_element("div");
        dom.append_child(p, div).unwrap();
    }
    let css = sheet(css);
    // The thread's shared empty `var()` map, built on first use, is no
    // per-element cost.
    let _ = ComputedStyle::initial();
    crate::test_alloc::allocations_in(|| dom.cascade(&css))
}

/// C9G-PACKER-ALLOC. An inherited value that draws nothing costs nothing
/// per element: a `font-family` list on an ancestor — almost every sheet's
/// `body { font-family: system-ui, … }` — is shared by its descendants'
/// computed styles, not copied, so a plain element cascades with the same
/// allocations whether the list is there or not (it cost one per family
/// name per element).
#[test]
fn an_inherited_family_list_is_shared_not_copied() {
    let per_element = |css: &str| cascade_allocations(css, 40) - cascade_allocations(css, 20);
    let plain = per_element(".p { color: red }");
    let family = per_element(
        ".p { color: red; font-family: system-ui, -apple-system, \"Segoe UI\", Roboto, sans-serif }",
    );
    assert_eq!(
        family, plain,
        "20 elements: {family} allocations for {plain}"
    );
}

/// C9-CARRY-INDENT. `text-indent` is inherited (CSS Text 3 §8.1), so every
/// descendant's computed style carries its value: a `calc()` indent —
/// `calc(50% + 2)` keeps its percentage to the used value, resolved against
/// each block's width (CSS Values 4 §10.9) — is shared by them, not copied,
/// so a plain element cascades with the same allocations whether the
/// expression is there or not (it cloned the expression tree per element).
#[test]
fn an_inherited_calc_text_indent_is_shared_not_copied() {
    let per_element = |css: &str| cascade_allocations(css, 40) - cascade_allocations(css, 20);
    let plain = per_element(".p { color: red; text-indent: 2 }");
    let calc = per_element(".p { color: red; text-indent: calc(50% + 2) }");
    assert_eq!(calc, plain, "20 elements: {calc} allocations for {plain}");
}

/// C10G-HIGHLIGHT-COST. A `::highlight(name)` rule with no compound before
/// it (`*::highlight(search)`, CSS Custom Highlight API 1 §5.1) styles every
/// element, and an element's highlight style is most often its parent's:
/// it is shared, so a plain element cascades with the same allocations
/// whether a highlight is styled or not (it allocated a style and a list
/// per element per name).
#[test]
fn a_universal_highlight_rule_shares_its_style() {
    let per_element = |css: &str| cascade_allocations(css, 40) - cascade_allocations(css, 20);
    let plain = per_element(".p { color: red }");
    let styled = per_element(".p { color: red } ::highlight(search) { background-color: yellow }");
    assert_eq!(
        styled, plain,
        "20 elements: {styled} allocations for {plain}"
    );
}

/// C10G-HIGHLIGHT-COST. A restyle that recomputes an element (a
/// registered custom property's transition, `walk::Mode::Restyle`) reloads
/// its recorded `::highlight()` matches as it reloads its other boxes' —
/// no selector is matched again.
#[test]
fn a_restyle_reuses_the_highlight_matches() {
    let mut dom = one_div();
    let div = dom.node(dom.root()).first_child().unwrap().id();
    let css = sheet("div { color: red } ::highlight(search) { background-color: yellow }");
    let sheets = [&css];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    cascade_all_with(&mut dom, &sheets, Some(registry.clone()));
    // A stale style: the restyle computes the element afresh.
    dom.node_mut(div).ext_mut().unwrap().computed = Some(Rc::new(ComputedStyle::initial()));
    super::matching::probe::take();
    restyle_vars(&mut dom, &sheets, registry, &[div]);
    assert_eq!(super::matching::probe::take(), 0, "selectors matched");
    let ext = dom.node(div).ext().unwrap();
    assert!(ext.computed_highlight("search").is_some());
}

/// C10G-IDLE-SCANS. What a sheet set holds that every element's cascade
/// asks — whether it styles `::first-line` / `::first-letter`, the names
/// its `::highlight()` rules style, its `@counter-style` registry — is
/// found once per sheet set (its registry stamp), not rescanned by every
/// cascade run over the same sheets.
#[test]
fn a_sheet_set_is_scanned_once() {
    use super::sheets::cost::FACT_BUILDS;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ol = dom.create_element("ol");
    dom.append_child(root, ol).unwrap();
    let li = dom.create_element("li");
    dom.append_child(ol, li).unwrap();
    let css = sheet(
        "p::first-line { color: red } ::highlight(h) { color: blue } \
         @counter-style x { system: cyclic; symbols: '*' }",
    );
    let sheets = [&css];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    FACT_BUILDS.with(|c| c.set(0));
    cascade_all_with(&mut dom, &sheets, Some(registry.clone()));
    cascade_all_with(&mut dom, &sheets, Some(registry.clone()));
    cascade_subtrees_all_with(&mut dom, &sheets, Some(registry), &[li]);
    assert_eq!(FACT_BUILDS.with(|c| c.get()), 3, "one scan per fact");
}
