//! The interleaved cascade and layout of query containers (CSS
//! Conditional 5 §6.4–§6.6): a container query and a container-relative
//! unit read the container's content box, which only layout gives, and
//! the styles they decide change the layout inside the container.
//!
//! Browsers style, lay out up to a container, style its subtree with its
//! size known, and lay that out. rdom's layout is a whole-tree pass, so it
//! does the same in passes: lay out; measure every container the cascade
//! queried (`style::cascade::container`); re-cascade the subtree of each
//! whose size moved from the one its descendants read; lay out again —
//! until no size moves. A container's own size never depends on its
//! subtree on a queried axis (size containment), so each pass settles one
//! more level of nested containers — but for the scrollbar gutter, which
//! comes out of a scroll container's content box: a query that shows a
//! bar can take itself back, and a container that returns to a size its
//! readers were already cascaded at is frozen there ([`cycles`]). The
//! passes are bounded by [`MAX_PASSES`], past which the sizes are kept for
//! the next layout. A container nobody reads any more is forgotten by the
//! cascade that stopped reading it (`style::cascade::container`). A
//! document that queried no container lays out once, as before.
//!
//! The re-cascades use the sheets the `App` publishes (and, once the
//! sizes settle, note the restyled roots on its dirty tracker, so the next
//! frame starts their transitions), or those of the last `CascadeExt`
//! cascade.
//!
//! The same passes settle `content-visibility: auto` (CSS Containment 2
//! §4): after each layout the elements whose relevance changed start or
//! stop skipping their contents, and are re-cascaded and laid out again.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::Rect;
use crate::style::cascade::cascade_subtrees_all_with;
use crate::style::cascade::container::{self, AxisSizes};
use crate::style::content_visibility;

/// The most re-cascades one layout makes: one per level of nested query
/// containers.
pub(crate) const MAX_PASSES: usize = 8;

/// Lay out with `layout`, then re-cascade and lay out again while a
/// queried container's size moves (module doc).
pub(super) fn lay_out(
    dom: &mut Dom<TuiExt>,
    viewport: Rect,
    layout: impl Fn(&mut Dom<TuiExt>, Rect),
) {
    layout(dom, viewport);
    if !container::any_queried(dom) && !content_visibility::any_auto(dom) {
        // Only the sizes the elements remember can move
        // (`contain-intrinsic-size: auto`).
        content_visibility::after_layout(dom, layout_viewport(viewport));
        return;
    }
    let Some(crate::runtime::style_flush::CascadeInputs {
        sheets,
        registry,
        tracker,
    }) = container::inputs(dom)
    else {
        return;
    };
    let sheets: Vec<&crate::style::Stylesheet> = sheets.iter().map(|s| &**s).collect();
    // The sizes each container's readers were cascaded at in this layout.
    let mut cascaded_at: HashMap<NodeId, Vec<Option<AxisSizes>>> = HashMap::new();
    let mut restyled = Vec::new();
    let mut settled = false;
    for _ in 0..MAX_PASSES {
        // The queried containers whose size moved, but for those cycling,
        // and the `content-visibility: auto` elements that started or
        // stopped skipping their contents (`style::content_visibility`).
        let mut stale = container::stale(dom);
        stale.retain(|&c| !cycles(dom, c, &mut cascaded_at));
        stale.extend(content_visibility::after_layout(
            dom,
            layout_viewport(viewport),
        ));
        if stale.is_empty() {
            settled = true;
            break;
        }
        #[cfg(test)]
        probe::PASSES.with(|c| c.set(c.get() + 1));
        restyled.extend(cascade_subtrees_all_with(
            dom,
            &sheets,
            Some(registry.clone()),
            &stale,
        ));
        layout(dom, viewport);
    }
    if settled {
        // The roots' transitions start at the next frame, which lays out
        // once more and finds nothing stale.
        if let Some(tracker) = &tracker {
            tracker.note_flushed(&restyled);
        }
    } else {
        // At the cap the sizes are kept for the next layout, and no frame
        // is asked for: what reached the cap is a nesting deeper than
        // `MAX_PASSES`, which the next layout carries on. The last
        // layout's sizes are still remembered.
        content_visibility::remember_sizes(dom);
    }
}

/// Whether the stale `container` returned to a size its readers were
/// already cascaded at in this layout — a cycle: its query flips its own
/// size, which only a scrollbar can do, as size containment keeps the
/// container's size independent of its subtree on a queried axis but the
/// scrollbar gutter (CSS Overflow 3 §3.4) comes out of its content box.
/// Such a container is frozen: its readers keep the styles of their last
/// cascade, and its size counts as read (DIVERGENCES §2). Otherwise its
/// new size is noted as the one the pass cascades at.
fn cycles(
    dom: &Dom<TuiExt>,
    container: NodeId,
    cascaded_at: &mut HashMap<NodeId, Vec<Option<AxisSizes>>>,
) -> bool {
    let Some((read, measured)) = container::sizes(dom, container) else {
        return false;
    };
    let seen = cascaded_at.entry(container).or_insert_with(|| vec![read]);
    if seen.contains(&measured) {
        container::freeze(dom, container);
        return true;
    }
    seen.push(measured);
    false
}

/// The viewport as a layout rect (what `content-visibility` relevance is
/// measured against).
fn layout_viewport(r: Rect) -> crate::layout::LayoutRect {
    crate::layout::LayoutRect::new(i32::from(r.x), i32::from(r.y), r.width, r.height)
}

/// Test-only: the re-cascades the passes made.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        PASSES.with(|c| c.replace(0))
    }
}

#[cfg(test)]
mod tests {
    //! What the interleaving costs, counted: nothing without a query,
    //! one re-cascade per level of nested containers the first time, none
    //! once the sizes settle.

    use super::probe;
    use crate::render::{LayoutExt, Rect};
    use crate::style::CascadeExt;
    use crate::style::cascade::container::probe::EVALUATIONS;
    use crate::{TuiDom, Viewport};

    /// `<div id=outer><div id=card><p id=t>` styled by `css`, cascaded
    /// and laid out in 40 × 5; the counters taken before.
    fn run(css: &str) -> TuiDom {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        rdom_parser::parse_into(
            &mut dom,
            r#"<div id="outer"><div id="card"><p id="t">x</p></div></div>"#,
            root,
        )
        .unwrap();
        let sheet = rdom_css::parse(css).stylesheet;
        dom.set_viewport(Viewport::new(40, 5));
        probe::take();
        EVALUATIONS.with(|c| c.set(0));
        dom.cascade(&sheet);
        dom.layout_dom(Rect::new(0, 0, 40, 5));
        dom
    }

    /// No `@container` rule and no container unit: no query is evaluated,
    /// no container is recorded, layout runs once — a size container
    /// alone (`container-type`) changes none of it.
    #[test]
    fn no_query_costs_nothing() {
        let dom = run("#card { container-type: inline-size; width: 10 } #t { color: red }");
        assert_eq!(probe::take(), 0);
        assert_eq!(EVALUATIONS.with(|c| c.get()), 0);
        assert!(!crate::style::cascade::container::any_queried(&dom));
    }

    /// One container: its size is unknown at the first cascade, so the
    /// pass re-cascades its subtree once; a second layout with nothing
    /// moved re-cascades nothing.
    #[test]
    fn one_container_takes_one_pass_then_none() {
        let mut dom = run(
            "#card { container-type: inline-size; width: 10 } @container (width > 5) { #t { color: red } }",
        );
        assert_eq!(probe::take(), 1);
        dom.layout_dom(Rect::new(0, 0, 40, 5));
        assert_eq!(probe::take(), 0, "settled");
    }

    /// `markup` styled by `css`, cascaded and laid out `w` × 5; the
    /// counters taken before.
    fn run_over(markup: &str, css: &str, w: u16) -> (TuiDom, crate::style::Stylesheet) {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        rdom_parser::parse_into(&mut dom, markup, root).unwrap();
        let sheet = rdom_css::parse(css).stylesheet;
        dom.set_viewport(Viewport::new(w, 5));
        probe::take();
        EVALUATIONS.with(|c| c.set(0));
        dom.cascade(&sheet);
        dom.layout_dom(Rect::new(0, 0, w, 5));
        (dom, sheet)
    }

    /// Architect B2a: once nothing reads a container — the class its query
    /// styled removed, the tree cascaded again — its size moving
    /// re-cascades nothing (it re-cascaded every pass, to the cap, every
    /// layout).
    #[test]
    fn a_container_nobody_reads_takes_no_pass() {
        let (mut dom, sheet) = run_over(
            r#"<div id="card"><p id="t" class="badge">x</p></div>"#,
            "#card { container-type: inline-size; width: 50% } \
             @container (width > 30) { .badge { color: red } }",
            80,
        );
        assert_eq!(probe::take(), 1);
        let t = dom.get_element_by_id("t").unwrap();
        dom.set_attribute(t, "class", "").unwrap();
        dom.cascade(&sheet);
        dom.layout_dom(Rect::new(0, 0, 80, 5));
        probe::take();
        dom.set_viewport(Viewport::new(70, 5));
        dom.layout_dom(Rect::new(0, 0, 70, 5));
        assert_eq!(probe::take(), 0);
    }

    /// Architect B2b: a container whose query shows a scrollbar that takes
    /// the query back oscillates between two sizes; the pass freezes it at
    /// the first size it returns to, well inside the bound, and the next
    /// layout re-cascades nothing.
    #[test]
    fn an_oscillating_container_is_frozen() {
        let (mut dom, _) = run_over(
            r#"<div id="c"><div id="list">x</div></div>"#,
            "#c { container-type: inline-size; overflow-y: auto; height: 3; width: 31 } \
             @container (width > 30) { #list { height: 10 } }",
            40,
        );
        let passes = probe::take();
        assert!(passes < super::MAX_PASSES, "{passes} passes");
        dom.layout_dom(Rect::new(0, 0, 40, 5));
        assert_eq!(probe::take(), 0, "frozen");
    }

    /// Architect N4: a container condition is evaluated for the elements
    /// its rule's selector matches, so `@container (…) { #t:hover {…} }`
    /// makes no element a reader while nothing is hovered (it made every
    /// candidate one, re-cascaded as the container moved).
    #[test]
    fn a_condition_is_evaluated_only_for_matching_elements() {
        let dom = run(
            "#card { container-type: inline-size; width: 10 } @container (width > 5) { #t:hover { color: red } }",
        );
        assert_eq!(EVALUATIONS.with(|c| c.get()), 0);
        assert!(!crate::style::cascade::container::any_queried(&dom));
        assert_eq!(probe::take(), 0);
    }

    /// Architect N6: "a size container exists" describes the styles in
    /// use — gone with the last `container-type`, so the cascade stops
    /// looking for containers for container-relative units.
    #[test]
    fn the_size_container_flag_follows_the_styles() {
        let mut dom = run("#card { container-type: inline-size; width: 10 }");
        assert!(crate::style::cascade::container::has_size_containers(&dom));
        dom.cascade(&rdom_css::parse("#card { width: 10 }").stylesheet);
        assert!(!crate::style::cascade::container::has_size_containers(&dom));
    }

    /// Two nested containers, the inner sized by the outer's query: one
    /// pass per level.
    #[test]
    fn nested_containers_take_a_pass_per_level() {
        run("#outer { container-type: inline-size; width: 30 }
             #card { container-type: inline-size; width: 5 }
             @container (width >= 30) { #card { width: 20 } }
             @container (width >= 20) { #t { color: red } }");
        assert_eq!(probe::take(), 2);
    }
}
