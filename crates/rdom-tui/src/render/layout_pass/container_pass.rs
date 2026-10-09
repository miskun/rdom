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
//! more level of nested containers; the passes are bounded by
//! [`MAX_PASSES`], past which the sizes are kept for the next layout. A
//! document that queried no container lays out once, as before.
//!
//! The re-cascades use the sheets the `App` publishes (and note the
//! restyled roots on its dirty tracker, so the next frame starts their
//! transitions), or those of the last `CascadeExt` cascade.

use rdom_core::Dom;

use crate::ext::TuiExt;
use crate::render::Rect;
use crate::style::cascade::{cascade_subtrees_all_with, container};

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
    if !container::any_queried(dom) {
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
    for _ in 0..MAX_PASSES {
        let stale = container::stale(dom);
        if stale.is_empty() {
            return;
        }
        #[cfg(test)]
        probe::PASSES.with(|c| c.set(c.get() + 1));
        let cascaded = cascade_subtrees_all_with(dom, &sheets, Some(registry.clone()), &stale);
        if let Some(tracker) = &tracker {
            tracker.note_flushed(&cascaded);
        }
        layout(dom, viewport);
    }
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
