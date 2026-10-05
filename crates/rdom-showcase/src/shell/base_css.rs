//! The shell's chrome stylesheet, [`BASE_CSS`] — split from `shell`
//! (C5-BOX-SIZING) so the chrome builder and its CSS stay apart.

/// Chrome stylesheet for the shell. Authored as CSS so it reads
/// like CSS — the showcase IS the dogfooding fixture, so when a
/// consumer browses the source they should see the same shape
/// they'd write themselves.
///
/// Borders use `border-collapse: collapse` declared explicitly on
/// `.app`, `.app-body`, and `.main` so adjacent inner borders
/// share cells instead of stacking into double rules at every
/// junction. Under BORDER-MODEL-1 the property is non-inheriting
/// — each container along the chain (panel → body → main) opts
/// in for its own direct children to participate. `.app-body`
/// additionally gets its own (visually invisible) border so it
/// can join `.app`'s collapse group at the header / footer rows.
///
/// **`.main` has NO padding by design.** The chrome's job is to
/// define the panel container — its borders, position, and
/// stacking relative to the sidebar. Choosing the content's
/// visual inset belongs to the demo, not the chrome. This lets
/// canvas-style or full-bleed demos paint to every cell of the
/// panel without fighting an injected panel margin. Text demos
/// that want content padding set their own `padding: 1` (or
/// other value) on their content root.
///
/// `.sidebar` keeps its `padding: 1` because the sidebar IS the
/// showcase's own nav UI — not a demo — and the chrome owns its
/// look.
///
/// **Fitting-pane chain.** `.app`, `.app-body`, and `.main` each
/// declare `min-width: 0` / `min-height: 0`. This is a web-faithful
/// app-shell pattern, not a workaround: by default flex items
/// can't shrink below their intrinsic content size (CSS Flexbox
/// §4.5), so a content-heavy demo would force its ancestor chain
/// to grow past the viewport. The `min-*: 0` opt-in says "I am a
/// fitting pane — clip me to the viewport, don't grow me to my
/// children's content." Real CSS authors use the same pattern in
/// browser app shells; the substrate fix (`M5-MIN-CONTENT-1`)
/// adopted the contract, the chrome opts into the override for
/// every container in the fitting chain. If a future structural
/// change adds another container between `.app` and `.main`, that
/// container needs the same opt-in or content overflow will
/// reappear.
pub(super) const BASE_CSS: &str = r#"
/* Border-box sizing for the shell's chrome, as most pages declare:
 * a panel's `width` / `height` include its border and padding. Scoped
 * to the chrome — the panels, the sidebar, the source disclosure, the
 * status bar — and not to the mounted demo inside `.view-content`, so
 * each demo lays out under its own CSS (most declare the same reset;
 * `ua_chrome` shows the UA defaults, `raf_progress` content-box). */
.app-shell, .app, .app-header, .app-header *, .app-body,
.sidebar, .sidebar *, .sidebar *::before, .sidebar *::after,
.main, .view-content,
.source-disclosure, .source-disclosure *,
.source-disclosure *::before, .source-disclosure *::after,
.status-bar, .status-bar *, .status-bar *::before, .status-bar *::after {
  box-sizing: border-box;
}
/* `.app-shell` is the outer flex column that holds the bordered
 * `.app` panel and the status bar below it. `.app` flexes to fill
 * remaining viewport height; the status bar holds its intrinsic
 * 1-row footprint. `min-*: 0` keeps the panel shrinkable past its
 * children's intrinsic content size (same fitting-pane pattern
 * used inside `.app`).
 */
.app-shell {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.app {
  /* BFC-1 Phase 4.3: with the block/flex dispatch wired,
   * containers that want flex distribution (flex-grow, flex-
   * direction) MUST declare `display: flex`. Default `<div>` is
   * Block flow per UA + CSS3 Display Module. */
  display: flex;
  flex: 1;
  flex-direction: column;
  border: solid;
  border-color: rgb(45, 47, 49);
  border-collapse: collapse;
  /* App shell fits the viewport — opt out of intrinsic-min on
   * both axes so the children's content can scroll rather than
   * the shell ballooning to fit them. Mirrors `.app-body` and
   * `.main`. */
  min-width: 0;
  min-height: 0;
}

.app-header {
  height: 3;
  border: solid;
  border-color: rgb(45, 47, 49);
  padding: 0 1;
}
.app-header h1 {
  color: rgb(200, 220, 255);
  font-weight: bold;
}

.app-body {
  display: flex;
  flex: 1;
  flex-direction: row;
  /* BORDER-MODEL-1 (non-inheriting `border-collapse`): a container
   * that wants its direct children to share borders must declare
   * collapse itself. `.app-body`'s direct children are `.sidebar`
   * and `.main`; declaring collapse here makes their adjacent
   * vertical borders overlap (the `┬`/`┴` junctions between
   * sidebar and main on the header / footer rows).
   *
   * `.app-body` also gives itself a border so it can participate
   * in `.app`'s collapse — its outer top shares the row with
   * `.app-header`'s bottom, and its outer left/right share the
   * columns with `.app`'s left/right borders. Without a real
   * border, body would be "transparent" and the direct-children
   * rule would put it on a row below header rather than coincident
   * with header's bottom row.
   *
   * Visually the body's border ring is invisible: every cell it
   * paints to coincides with an `.app` or `.app-header` border
   * cell, so paint-time mask-OR produces the same junction glyph
   * either way. */
  border: solid;
  border-color: rgb(45, 47, 49);
  border-collapse: collapse;
  /* Body is an app-shell pane — it should track the available
   * height, not balloon to the sum of its children's intrinsic
   * heights. Web-faithful: real CSS authors use `min-height: 0`
   * on flex items they want shrinkable past their content. */
  min-height: 0;
}

.sidebar {
  width: 28;
  height: 100%;
  border: solid;
  border-color: rgb(45, 47, 49);
  /* No sidebar padding — the 1-cell left inset that keeps item text
   * off the border lives on `.sidebar-tree` instead (below), so the
   * tree's selected/cursor row highlight fills that cell too (the
   * highlight spans the tree's padding box). No right padding either:
   * the rightmost content column is the scrollbar gutter
   * (`overflow-y: auto`); padding there would wedge a blank cell
   * between the scrollbar `┃` and the right border. */
  padding: 0;
  /* The scroll lives on the `.sidebar-tree` (the `<ul role=tree>`),
   * NOT on this wrapper — so the FOCUSED element (the tree) owns its
   * scrollbar and gets the `:focus::scrollbar-thumb` accent cue.
   * `.sidebar-tree` opts into `overflow-y: auto` below; this `.sidebar`
   * just bounds the height. (Overflow is a layout choice the consumer
   * makes per element — rdom's `<ul role=tree>` is `overflow: visible`
   * by default, like any `<ul>`.) */
}
/* The `<nav>` between `.sidebar` and the tree must carry the definite
 * height through, or the tree's `height: 100%` resolves against an
 * auto-height containing block and over-sizes (guides bleed past the
 * panel). */
.sidebar nav {
  height: 100%;
}
/* The 1-cell left inset rides on the tree, not the sidebar, so the
 * row-highlight bg (which fills the tree's padding box) extends to
 * the panel border — a full-width selection bar — while the content
 * stays exactly where the sidebar padding put it. Chrome-owned class;
 * see `build_shell` for why it must NOT be the demo's `nav-tree`.
 *
 * `overflow-y: auto` makes the tree its OWN scroll container, and
 * `height: 100%` bounds it to the (definite-height) sidebar so the
 * overflow actually clips + scrolls — the focused tree then owns its
 * scrollbar, whose thumb turns accent while focused. */
.sidebar-tree {
  height: 100%;
  overflow-y: auto;
  padding: 0 0 0 1;
}

.main {
  display: flex;
  flex: 1;
  flex-direction: column;
  border: solid;
  border-color: rgb(45, 47, 49);
  /* BORDER-MODEL-1 (non-inheriting `border-collapse`): declare
   * collapse on `.main` so its direct children (`.view-content`
   * and `.source-disclosure`) share borders — the
   * source-disclosure's `border-top` sits on `.main`'s outer
   * frame and shares the cell with the panel's right and left
   * borders to form the `├`/`┤` T-junctions at row 20. */
  border-collapse: collapse;
  /* Opt into responsive shrink past intrinsic content size — the
   * source disclosure can hold lines wider than the terminal, but
   * `<main>` should still fit the row alongside the sidebar.
   * `min-height: 0` lets `<main>` fit the available height of
   * `.app-body` so the demo can fill the viewport vertically.
   * Web-faithful: real CSS authors use `min-width: 0` /
   * `min-height: 0` on flex items they want shrinkable past
   * their content. */
  min-width: 0;
  min-height: 0;
}

/* `.view-content` is the "Page": the single-slot scroll viewport that
 * hosts the active demo. It's a flex ITEM of `<main>` (`flex: 1`,
 * `min-height: 0`) so it fills the height left over after the Source
 * tray — and shrinks when Source expands — but its OWN formatting is
 * `display: block` with `overflow-y: auto`. That gives demos the
 * web "page scrolls" default: a demo at natural height taller than
 * the pane overflows it and the Page scrolls. (A flex-column pane
 * would instead force every `flex: 1` demo to exactly pane height,
 * so tall content squished rather than scrolled.)
 *
 * Two demo idioms, by design:
 * - CONTENT demo (the default): no special CSS — a stray `flex: 1`
 *   is a harmless no-op in a block parent, so the demo lays out at
 *   its natural height and the Page scrolls when it's tall.
 * - FILL demo (claims the viewport + wraps its own scroller, e.g.
 *   `scrollable_list`, `mutation_observer`): sets `height: 100%` on
 *   its root. That now resolves against this flex-sized pane (CSS
 *   Flexbox §9.8 definite size — see DIVERGENCES.md), bounding an
 *   inner `overflow: auto` so it scrolls internally and the Page
 *   does NOT.
 *
 * `overflow-y: auto` also keeps a tall demo from bleeding into the
 * Source tray below (the old `overflow: hidden` clipped but couldn't
 * scroll); horizontal stays visible. `min-height: 0` keeps the Page
 * bounded to its slot so it scrolls instead of ballooning `<main>`.
 */
.main .view-content {
  display: block;
  flex: 1;
  min-width: 0;
  min-height: 0;
  /* Scroll vertically (the Page), clip horizontally. The old
   * `overflow: hidden` clipped both axes to stop a tall demo bleeding
   * into the Source tray; `overflow-y: auto` keeps that vertical clip
   * AND scrolls, while `overflow-x: hidden` preserves the horizontal
   * no-bleed guarantee (horizontal page-scroll isn't a goal). */
  overflow-x: hidden;
  overflow-y: auto;
}

/* Source disclosure. Two states:
 *
 * - CLOSED (default) — intrinsic height = 1 row summary +
 *   1 row `border-top: solid` = 2 outer rows. The demo gets all
 *   the remaining vertical space inside `<main>`.
 * - OPEN — fixed 12 outer rows = 1 border-top + 11-row content
 *   area, with `overflow: auto` so long source scrolls inside the
 *   panel rather than pushing chrome around. Predictable split
 *   regardless of demo source length.
 *
 * `[open]` attribute selector targets only the open state, leaving
 * the closed state at its intrinsic. The substrate's UA rule
 * `details:not([open]) > *:not(summary) { display: none }` hides
 * the body when closed, so there's nothing to scroll when closed.
 */
/* Horizontal padding lives on the container, NOT on each child.
 * Web-idiom: container provides the inset, every block child
 * (<summary>, <h3>, <pre>, plus anything added later) inherits the
 * same content-box left edge automatically. Per-child padding on
 * <summary> + <pre> with nothing on <h3> previously left the
 * "Markup" / "CSS" labels flush against the panel border. */
.main .source-disclosure {
  border-top: solid;
  border-color: rgb(45, 47, 49);
  padding: 0 1;
}
.main .source-disclosure[open] {
  height: 12;
  /* `overflow: auto` reserves a scrollbar gutter only when the
   * scrollbar actually shows (CSS-default `scrollbar-gutter: auto`
   * post-`CHROME-SCROLL-GUTTER-DEFAULT-1`), so short source doesn't
   * lose a content column. */
  overflow: auto;
}
.main .source-disclosure summary {
  color: rgb(180, 200, 230);
  font-weight: bold;
  /* The "Source" label is an interactive disclosure control, not prose —
   * a drag-select spilling out of the demo above shouldn't highlight it.
   * The source `<pre>` below stays selectable so its code can be copied. */
  user-select: none;
}
.main .source-disclosure h3 {
  color: rgb(180, 200, 230);
}
.main .source-disclosure pre {
  color: rgb(200, 210, 230);
}

/* Status bar — sibling of `.app` under `.app-shell`. Lives outside
 * the bordered panel so its row doesn't fight `.app`'s
 * `border-collapse`. Empty by default; the scroll listener writes
 * scroll-position info here when a descendant of view-content
 * scrolls. Phase 1b will seed default keyboard-shortcut hints.
 *
 * `min-height: 1` keeps the bar visible when empty — same reason
 * as the prior `.scroll-indicator` rule (substrate's auto-min
 * floor would otherwise let it collapse to zero under tight
 * viewport pressure).
 */
.status-bar {
  display: flex;
  flex-direction: row;
  height: 1;
  min-height: 1;
  padding: 0 1;
  color: rgb(150, 170, 200);
}
/* Hints slot grows to absorb the remaining row space, leaving
 * the mouse-pos slot pushed flush against the right edge. Two
 * spans in a flex row with `flex: 1` on the first one gives the
 * canonical "left content + right content" layout.
 */
.status-bar .status-bar-hints {
  flex: 1;
}
.status-bar .status-bar-mouse-pos {
  color: rgb(140, 160, 190);
}
/* Keyboard-hint spans inside the status bar. Two-tone styling
 * (key vs label) mirrors the conventional terminal-status-line
 * pattern: the *key* is what the user has to press, so it's bright
 * + bold; the *label* describes what pressing it does, so it's
 * muted to read as supporting prose.
 */
.status-bar .key {
  color: rgb(220, 230, 255);
  font-weight: bold;
}
.status-bar .label {
  color: rgb(140, 160, 190);
}
.status-bar .sep {
  color: rgb(80, 90, 110);
}
"#;
