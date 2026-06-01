//! ARIA tree navigation with a fake lazy-loaded branch.
//!
//! A `<ul role=tree>` rendered with guide lines (`│ ├ └`), chevrons,
//! and keyboard navigation. The "Nodes" branch loads its children
//! 2 seconds after you first expand it (Right arrow or click the
//! twisty) — showing the `aria-busy` loading affordance in between.
//!
//! Controls: Tab to focus the tree, then ↑/↓ to move the cursor,
//! →/← to expand/collapse or descend/ascend, Home/End to jump,
//! Enter/Space to activate. Ctrl-C to quit.
//!
//! Run: `cargo run --example tree_nav -p rdom-tui`
//!
//! The DOM-building + CSS lives in `rdom-showcase::demos::tree_nav`
//! so it runs both standalone (this binary) and in the showcase
//! under "Built-ins → ARIA tree (lazy load)". Single source of truth.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::tree_nav::run_standalone()
}
