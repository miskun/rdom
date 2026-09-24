# DIVERGENCES — where rdom departs from the web platform

The rdom default rule is: **track the web platform.** WHATWG DOM, CSS Working Group specs, and UI Events are the reference. If a behavior is not listed here, it should match the web platform within the supported subset.

This document collects every deliberate departure in the shipped crates (current line: `rdom-core` 0.4 / `rdom-tui` 0.4 under [`HARDENING-2026-09.md`](HARDENING-2026-09.md)). Departures fall into three groups:

1. **TUI medium constraints** — fixed by the fact that we render to a character grid.
2. **Simplifications** — places where rdom keeps a smaller model than the web platform.
3. **Not yet shipped** — common web features rdom still omits.

Roadmap for what's coming next: see [`DESIGN.md`](DESIGN.md#roadmap).

---

## 1. TUI medium constraints

These are intrinsic to terminals. They will not change.

- **Integer cells only.** No subpixel positioning, no fractional widths, no anti-aliasing. Coordinates are `u16` cells.
- **Monospaced advance.** Variable-width fonts are out of scope.
- **No images, no SVG, no pixel painting.** `<canvas>` is a cell-painting escape hatch via `RenderContext`, not a pixel-painting surface.
- **Length units.** Sizing accepts cells (unitless integers), the flex `fr` unit, and `%` (resolves against the parent's content-area dimension at layout time). `px`, `em`, `rem`, `ch`, and viewport units (`vh`/`vw`) are tokenized but produce a warning and are dropped — they depend on a pixel or font-size concept the terminal grid doesn't have. `%` is *relative* to parent dimensions (which the layout pass already knows), so it ships as a first-class unit.
- **Color.** `Color::Rgb` emits truecolor SGR sequences unconditionally; there is no `COLORTERM` runtime autodetection. A separate 256-color fallback exists as an explicit code path.
- **UA stylesheet glyphs assume BMP box-drawing support** (U+25xx, U+250x, U+256x). Terminals without these blocks are out of scope.
- **Tabs in `<pre>` render as a single space.** Full 8-column tab expansion is not implemented.
- **No bidirectional text (RTL), soft hyphens, `letter-spacing`, `word-spacing`.**
- **Synchronized output (DEC 2026 BSU/ESU) is always emitted** unless the `no-synchronized-output` feature flag is set.

## 2. Simplifications from the web platform

### Layout

- **`border-collapse` is a layout-only, non-inheriting opt-in that applies to any container's direct children, not only `<table>`s.** Three divergence axes from CSS:
  1. *Scope.* CSS restricts `border-collapse: collapse` to `<table>` boxes; rdom honors it on any container (flex or block). Terminal UIs lean on shared-border rendering for non-table chrome.
  2. *Inheritance.* CSS makes `border-collapse` inheritable. rdom makes it non-inheriting. A container that wants its direct children to participate must declare `border-collapse: collapse` itself — no spooky action across subtrees. The reset means demo / consumer subtrees never inherit a chrome's collapse decision implicitly.
  3. *Scope of effect within a subtree.* Within a single collapse container, the overlap-share affects only the container's **direct children**. The "transparent intermediate" recursive propagation present in earlier rdom builds is gone — to share borders with a more deeply nested element, every container in the chain declares collapse explicitly.
- **The 2×2 outcome grid for `gap` × `border-collapse`** is the canonical reference for what authors get:

  | `gap` on parent | `border-collapse` on parent | Outcome |
  |-----------------|------------------------------|---------|
  | `0`             | `separate` (default)         | Adjacent children, each owns its own border (`┐┌`). |
  | `>0`            | `separate`                   | Visible gap between bordered children. |
  | `0`             | `collapse`                   | Direct children overlap by 1 cell at shared edges; paint emits junction glyphs (`┬┴├┤┼`). |
  | `>0`            | `collapse`                   | Visible gap. `collapse` is a no-op because there is nothing adjacent to share — `gap` always wins where the two would conflict. |

- **`border-style: hidden` is honored as CSS Tables 3 §11.5's kill-switch on any rdom collapse subtree, not only tables.** Wherever `hidden` appears in a border conflict, that direction is suppressed regardless of any other contributor. Same divergence axis as the scope extension above — rdom adopts the table conflict-resolution algorithm wholesale and applies it to the non-table cases the substrate enables.
- **Border-style support is the full CSS keyword set, with terminal-faithful degradation:** `none`, `hidden`, `solid`, and `double` render with distinct glyphs (`│─┌┐└┘` / `║═╔╗╚╝`). `dashed`, `dotted`, `ridge`, `outset`, `groove`, `inset` parse and rank correctly in conflict resolution but render as `solid` (matching CSS's "render as best you can on this medium" principle — the data model is faithful even where the glyphs aren't yet distinguishable).
- **No margin collapsing between adjacent block boxes.** Margins are additive.
- **`min-width: auto` / `min-height: auto` content size suggestion uses intrinsic natural size**, not strict CSS min-content (longest-word width with wrap). The auto-min resolution itself follows CSS Flexbox §4.5 — `min(content_size_suggestion, specified_size_suggestion)`, dropped to 0 when overflow on the axis is non-visible. The approximation is in HOW `content_size_suggestion` is computed (natural width vs. longest-word). Tracked as `M5-MIN-CONTENT-2` in `TECH_DEBT.md`.
- **`flex: <N>` shorthand collapses `<basis>` to 0%.** rdom's parser stores both `flex: 1` and `flex: 1 1 auto` as `Size::Flex(1)`; the basis token is parsed-and-accepted but ignored. Effective behavior: any `flex: <N>` form is treated as `flex: <N> 1 0%`. Authors who need `flex-basis: auto` semantics (specified suggestion = the item's `width`) write `width: auto; flex-grow: 1` instead. Parser doc at `crates/rdom-style/src/parse/values.rs::parse_flex_shorthand` documents the ignore; this entry surfaces the user-visible consequence.
- **Percentage height resolves against a flex-sized ancestor only when that ancestor *flexes*.** Per CSS Flexbox §9.8 a flex item in a definite-size flex container has a definite post-flex size, so `height: <pct>%` on its content resolves against it — rdom honors this when the ancestor's height is `flex: <N>` (a growing/flexing item, including the chrome's `flex: 1` panes and the document root's viewport-sized children). The remaining gap: a flex item with a bare `height: auto` (e.g. a row flex item stretched on the cross axis, which CSS treats as definite) is still treated as **indefinite**, so a percentage-height child falls back to content size. Block-flow `auto` heights are indefinite exactly as CSS 2.1 §10.5 requires. Gate: `nearest_block_ancestor_height_is_definite` in `crates/rdom-tui/src/render/layout_pass/block.rs`.
- **`aspect-ratio` requires the explicit `<w>/<h>` form.** Bare numbers, decimals, and the `auto && <ratio>` fallback form are not parsed. Half-to-even (banker's) rounding is used when discretizing onto the cell grid.
- **Anonymous block boxes (CSS 2.1 §9.2.1.1), not anonymous inline boxes.** Mixed inline + block children inside a block-flow container produce **anonymous BLOCK boxes** wrapping each inline run (text + `display: inline` + atomic `display: inline-block`). Each anon box establishes its own IFC. This matches CSS for block containers; what rdom does NOT yet generate is anonymous *inline* boxes for the `<span>foo <span>bar</span> baz</span>` text-around-inline-around-text shape inside an existing IFC (inline-ancestor-breaking). Texts inside one inline element render as a single fragment; nested inline-ancestor-breaking is deferred.
- **No floats.** `float: left` / `float: right` are not parsed and not supported. CSS float layout (line-box exclusion, clearance, float resolution) is not in scope for rdom; flex / block / IFC cover the TUI use cases. Authors targeting browser-faithful float behavior should restructure with flex.
- **Inline backgrounds only.** Inline borders are not painted.
- **No `display: grid`.** Flexbox is the only multi-axis layout. `display: inline-block` ships for content-hugging chrome (buttons, badges, tags).
- **`text-align`, `vertical-align`, `text-decoration: line-through` are not implemented.** Left alignment only; single baseline.

### Positioning

- **Stacking contexts form from the root, positioned elements with a numeric `z-index`, and `opacity < 1` only.** Paint and hit-test follow CSS 2.1 Appendix E inside each context (negative `z-index` below in-flow content, positioned boxes above it, positive `z-index` on top). The other triggers — `transform`, `filter`, `isolation: isolate`, `will-change`, `mix-blend-mode`, `contain: paint` — do not exist.
- **Positioned `::before` / `::after` pseudo-elements paint in one flat pass above every stacking context**, ordered by the host's `z-index` and tree order; they are not part of their host's context.
- **Sticky containing block is the element's parent's content box**, not the CSS "nearest scroll container" for nested-scroller edge cases.
- **The static position inside a flex container ignores `justify-content` / `align-items`.** Flexbox §4.1 places an absolutely positioned child's hypothetical box as if it were the sole flex item, so `justify-content: center` would center it; rdom uses the content box's start corner (`flex-start`). In block and inline flow the static position follows CSS 2.1 §10.3.7 / §10.6.4.
- **No `transform`, `rotate`, `scale`, `matrix`, `isolation: isolate`, `will-change`.**
- **Positioned `::before` / `::after` pseudo-elements are not in the hit-test set.** Clicks on pseudo rects resolve to the underlying element.

### Values

- **`calc()` accepts both `5+5` and `5 + 5` inside the call.** CSS Values L3 requires whitespace around `+`/`-`; rdom's tokenizer doesn't preserve whitespace so the parser accepts either form. `*` and `/` don't need whitespace in CSS either, so those match.
- **`calc()` on `padding` / `margin` is constant-only.** Width/height/top/right/bottom/left/gap support full layout-time `calc()` with percentages. Padding/margin support `calc()` with constants only (`padding: calc(2 * 3)`); percent-bearing forms reject at parse time. The narrow gap is tracked as `CALC-PADMARG-1` in `TECH_DEBT.md`. A `calc()` / percent `gap` does not animate at all (cell ↔ cell gaps do); `Size` / `Length` calc values snap at the midpoint. A percent `gap` resolves against the container's content size on the gap's axis, and against 0 when that axis is indefinite — rdom takes `height: auto` on a column container as "indefinite" (CSS also treats `height: 50%` under an indefinite parent that way; rdom resolves it against the available height).
- **`calc()` does NOT support `min(...)` / `max(...)` / `clamp(...)`.** CSS Values L4 functions. Deferred.
- **CSS transitions don't smoothly tween between `calc()` values.** When either endpoint of a `transition` carries a `calc()` expression (Size or Length axis), the engine snaps at midpoint instead of interpolating. Smooth tweening would require resolving both endpoints to concrete cells using the current layout's parent dimensions at every animation tick — straightforward but unwired in M6.
- **`border-style: half-block` is rdom-specific.** Not a CSS-spec keyword. Each border cell fills the **quadrants that point inward** toward the bordered element's content — an edge fills a half (`▄ ▀ ▌ ▐`, U+2584/U+2580/U+258C/U+2590), a corner fills a single quadrant (`▗ ▖ ▝ ▘`, U+2596/U+2597/U+259D/U+2598). Pairs with a `background-color`-filled interior to produce a "pill"-style primary-CTA button that reads as ~2 cells tall on a 3-row layout (the half-blocks contribute half-cells of color each, joining the filled interior into a continuous accent region).
- **Half-block borders weld across elements (quadrant union).** When two half-block borders share a cell, the joiner **unions their inward quadrants** and emits the matching block glyph — so a "tab" box whose bottom edge overlaps a panel's top row welds into one tab-panel outline (`▟ █ ▌` at the junction), and any T-junction / cross resolves too. All 16 quadrant combinations have a Unicode block element (`▘▝▖▗ ▀▄▌▐ ▚▞ ▛▜▙▟ █`), so — unlike single-line borders, which have no rounded T-junctions — there are no gaps. This is the half-block analog of `border-collapse` welding and needs no opt-in (two adjacent half-block borders just join).
- **`border: half-block` clips `background-color` to the padding box implicitly.** The half-block paint phase resets the cell's bg to `Color::Reset` (terminal default) on every border cell after writing the glyph. Without this, the element's own `background-color` — which `fill_bg` paints across the entire outer rect including border cells — would cover the "empty" half of each half-block glyph, collapsing the silhouette back into a solid rectangle. Strict CSS would require an explicit `background-clip: padding-box`; rdom doesn't ship that property yet, so the half-block style hard-codes the equivalent behavior. Consequence: any parent `background-color` shows through the outer half of the half-block cells (intentional — it's what makes the pill visually "round"). To avoid the bg-clear, switch to `solid` or `rounded` border style.

### Cascade & selectors

Supported selector grammar: type, class, ID, attribute, descendant, child (`>`), adjacent sibling (`+`), general sibling (`~`), comma list. Supported pseudo-classes: `:hover`, `:focus`, `:focus-within`, `:checked`, `:indeterminate`, `:open`, `:first-child`, `:last-child`, `:only-child`, `:empty`, `:root`, `:not(<list>)`, `:where(<list>)`, `:placeholder-shown`.

- **`:where(<list>)`** matches like `:is()` (any complex selector in its list) but contributes **zero specificity** (Selectors L4) — the mechanism a component library uses to ship default styles that any author rule overrides. `:is()` (specificity = most-specific argument) is *not* yet implemented.
- **Not implemented:** `:nth-child(an+b)`, `:nth-of-type`, `:has()`, `:is()`, `:focus-visible`, `:disabled`, `:enabled`, `:read-only`, `:read-write`, `:required`, `:invalid`, `:valid`, `:modal`.
- **Not implemented as author-styleable pseudo-elements:** `::marker`, `::placeholder`, `::caret`, `::first-line`, `::first-letter`. List markers use `::before` content; placeholder text uses `:placeholder-shown::before { content: attr(placeholder) }`.
- **`::scrollbar`, `::scrollbar-thumb`, `::scrollbar-thumb:vertical` / `:horizontal` are rdom pseudo-elements** modeled on WebKit's `::-webkit-scrollbar` / `::-webkit-scrollbar-thumb` / `:vertical` / `:horizontal`; there is no standard equivalent (CSS Scrollbars 1 has only `scrollbar-color` / `scrollbar-width`, not shipped). They style the gutter cells that `scrollbar-gutter` reserves; `content` is the cell glyph.
- **`var()` is consumed in color positions and in `content` only.** Custom properties themselves follow CSS Variables 1: any selector, per-element scope, inherited, `!important` honored, inline `style="--x: …"` included. But `padding: var(--gap)` and every other non-color, non-`content` property do not substitute; the declaration is invalid at parse time and warns. Reason: rdom's property values are typed at parse time, and a general substitution pass (parse-time tokens → computed-time re-parse) is not built.
- **`unset` is resolved at parse time** from the inherited-property table — an implementation detail with the same observable result as the web; see `DESIGN.md`.
- **`z-index` accepts the `i16` range** (±32 767); larger integers are invalid and the declaration is dropped, where browsers accept any `<integer>`. Insets (`top` / `left` / …) take `i32`.
- **Not implemented:** `min()`, `max()`, `clamp()`, `currentColor`, CSS Nesting (`&`). (`calc()` shipped in 0.2.0; see the Values section above.)
- **`rgba()` alpha is dropped at parse time.** Translucency is handled by the dedicated `opacity` property, which composes alpha at paint time.
- **No at-rule is evaluated.** Every at-rule (`@import`, `@charset`, `@media`, `@supports`, `@keyframes`, `@font-face`, vendor-prefixed ones) is consumed whole per CSS Syntax 3 §5.4.2 — statement form through `;`, block form through its `{…}` — and reported as `WarningKind::UnsupportedAtRule(name)`; the rules around it are unaffected. `@keyframes` is on the roadmap; `@media` has no meaningful queries in a cell grid beyond viewport size, which rdom does not expose to CSS.
- **`background` is color-only.** The shorthand accepts a single `<color>` (it sets `background-color`); images, positions, repeat, and attachment have no cell-grid meaning, so any other value is dropped as invalid rather than partially applied.
- **Times resolve to whole milliseconds.** `1.5ms` rounds to 2ms: the animation clock ticks in milliseconds and a terminal frame is ~16ms, so sub-millisecond precision has no observable effect. (Percentages keep their fraction — `Size::Percent(f32)` — and round once, onto the cell grid, at layout.)
- **Per-side longhands share the shorthand's storage.** `padding-top: inherit` (or `margin-*`, `border-*`) marks the whole `padding` as inherited, and a later `padding-left: 2` on the same element replaces it entirely. The web keeps four independent longhands.
- **Flex factors are whole numbers.** `flex-grow` / `flex-shrink` / `flex: <n>` accept integers only (`flex: 1.5` is dropped as invalid); a fractional *shrink* factor in the 3-value shorthand is accepted and ignored like the rest of the shrink value. Cells are integers, so fractional grow rarely changes a result.
- **`flex: inherit` inherits the parent's main-axis size**, because `flex` maps onto `width` / `height` (see the flex entry above), not onto separate grow / shrink / basis longhands.

### DOM API shape

The DOM API is Rust-shaped rather than JS-shaped. The semantics match WHATWG DOM; the surface differs in idiomatic ways.

- **Handles are arena IDs (`NodeId`), not object references.** A `NodeId` is a slot index plus a generation. Slots are recycled after `drop_subtree` / `remove_child_dropping`, but the generation changes on every recycle, so a handle to a dropped node is rejected by `contains`, `node_or_err`, and every mutation path rather than resolving to the slot's new occupant. There is no garbage collection: a cached id does not keep a node alive (the web's object reference would). Comparing IDs across separate `Dom` instances is meaningless.
- **No `Node.prototype` / `Element.prototype` inheritance.** Behaviors attach via Rust trait impls on `NodeRef` / `NodeMut`.
- **Tag and attribute names are case-sensitive.** HTML's ASCII-case-insensitive matching is not applied.
- **Snapshots replace live collections.** `child_ids`, `query_selector_all`, attribute iterators, etc. return `Vec<NodeId>` snapshots or iterators — there is no live `NodeList`, `HTMLCollection`, `NamedNodeMap`, or `DOMTokenList`.
- **`classList` is a snapshot `DomTokenList` / `DomTokenListMut`**, plus direct `add_class` / `remove_class` / `toggle_class` / `has_class`; not a live `DOMTokenList`.
- **`getElementById` is arena-wide.** The lookup is index-backed and also finds a *detached* element carrying the id when no connected element does. With duplicate ids the first connected element in document order wins, as on the web. The web only searches the document tree.
- **Boundary-point ordering is a `Dom` method**, `compare_boundary_points(a, b) -> Option<Ordering>` (DOM §5.2's internal algorithm), rather than `Range.compareBoundaryPoints(how, range)`; `None` means the two nodes are in different trees.
- **Attributes are a flat `BTreeMap<String, String>`** exposed as iterator pairs. The `Attr` interface is not implemented.
- **No `Window` vs `Document` split.** The TUI `App` plays both roles.
- **`Dom::root()` is a Fragment, not `<html>`.** `<body>` and `<head>` are not auto-inserted.
- **`textContent.len()` returns bytes**, not UTF-16 code units.
- **No XML namespaces.** `namespaceURI`, `prefix`, `*NS` method variants are not applicable.
- **No Shadow DOM, no custom elements registry, no `<template>` cloning semantics.**
- **Detached nodes are not garbage-collected.** `remove_child` / `clear_children` / `replace_child` / `replace_children` detach a node but leave it in the arena as a reusable orphan (so it can be reattached). The slot is reclaimed only by `drop_subtree`, or the `remove_child_dropping` / `clear_children_dropping` convenience variants. The web relies on GC to reclaim unreferenced detached nodes; rdom's arena requires **explicit** reclamation, so high-churn callers that won't reattach (e.g. a virtualized list/table re-materializing rows) must drop removed nodes or leak arena slots. (`ARENA-RECLAIM-1`.)

### Events

- **`Event.detail` is one typed enum** (`EventDetail::{Mouse, Key, Input, Transition, Submit, Toggle, …}` with typed accessors such as `as_keyboard()`), not a class hierarchy of `MouseEvent` / `KeyboardEvent` / `InputEvent` interfaces. The payloads match the web's fields; the shape is Rust's.
- **`isTrusted` is inverted as `is_synthetic`.**
- **Listener removal is by handle or `AbortSignal`** (`remove_event_listener(ListenerId)`, or `ListenerOptions::signal`), not function-identity equality on a `removeEventListener(fn)` call.
- **Not implemented:** `composedPath()`, `initEvent()`, `passive: true`, `CustomEvent` ≠ `Event` distinction.
- **`keyup` requires kitty-keyboard-protocol terminal support.** rdom-tui enables `KeyboardEnhancementFlags::REPORT_EVENT_TYPES` on startup, but the host terminal has to honor it. Supporting terminals (kitty, foot, WezTerm, alacritty 0.13+, recent xterm) fire `keyup` events for every key release. Non-supporting terminals (legacy xterm, basic VT100, most macOS Terminal.app builds) only ever send `KeyEventKind::Press` — `keyup` listeners on those terminals will never fire. No emulation; if your demo needs cross-terminal key-release behavior, derive it from `keydown` + a timer.
- **No `Window` object — events that target `Window` in HTML target `dom.root()` here.** rdom has no chrome around the document grid; the terminal IS the document. Web-spec events that fire on `Window` (today: `resize`; future candidates that follow the same pattern: hypothetical `beforeunload`, `popstate`, etc.) collapse onto the document root. Authors write `dom.root().add_event_listener("resize", …)` where browsers would use `window.addEventListener`. Bubbling, cancelable flags, and dispatch timing match the HTML spec for the corresponding `Window` event; only the target identity differs.
- **`scroll` events bubble even on non-Document targets.** CSSOM View Module §6: `scroll` on `Document` bubbles, but `scroll` on element targets does NOT bubble. rdom fires every `scroll` event with `bubbles = true` regardless of target — a deliberate divergence that lets consumers install a single listener at `dom.root()` and observe all scroll activity (the showcase's scroll-position indicator works this way; per-scrollable installation would require re-wiring on every subtree swap). Suppress propagation explicitly in handlers if needed.
- **`scroll` events are NOT coalesced per rendering step.** HTML5 says multiple scroll-offset writes between rendering steps collapse to one `scroll` event. rdom fires one `scroll` event per mutation-site call: wheel ticks, scrollbar drag deltas, and each programmatic `set_scroll_top` / `scroll_to` write each fire a separate event. For typical terminal-app patterns (one offset write per crossterm event) this matches HTML behavior; bulk programmatic scrolling (e.g., a loop that mutates `scroll_top` ten times) sees ten events where browsers would deliver one. Pay down with a "dirty scroll" set drained at end-of-event-tick if a consumer hits the difference.
- **Pointer capture is mouse-only.** `set_pointer_capture` / `release_pointer_capture` capture *the mouse* (rdom has no touch or pen pointers, so there is no `pointerId` and no `gotpointercapture` / `lostpointercapture`). While captured, moves and the `mouseup` are dispatched to the capture element as their target, as on the web; capture releases on `mouseup`.
- **Drag autoscroll is an explicit opt-in layered on capture, and vertical-only (v1).** `dom.set_drag_autoscroll(true)` on a captured drag makes the runtime scroll a scroll container while the pointer is near its edge, re-dispatching the drag at the held pointer each ~50ms tick so the selection follows (DRAG-AUTOSCROLL). The web autoscrolls *implicitly* during native text selection / DnD; rdom arms text selection internally (so that case matches) but a custom captured drag must opt in (stricter — avoids autoscrolling a slider that merely captures). **A drag owns one scroll container for its lifetime (sticky):** the container is resolved *once*, the first time the pointer reaches an edge zone, then never re-targeted or disarmed for the rest of the drag. So the captured node scrolling out of view, or the pointer overshooting *past* the container onto a sibling, both keep scrolling that same container — textarea-like, and deliberately unlike a browser, which re-targets the scroll to whatever container is under the cursor and **chains** to the viewport (rdom has no viewport-chaining: the scrollport *is* the surface; it autoscrolls the sticky container and stops at its limit). Edge zone is a cell-grained band (a couple of rows at each edge plus everything beyond) with speed ramping by distance and capped — not the browser's pixel-accelerated curve. **Horizontal autoscroll is not implemented** (pairs with the flex cross-axis-scroll gap).
- **Not implemented:** `dragstart` / `drag` / `drop` / `DataTransfer`, touch events, IME composition events, pointer events beyond mouse.
- **`<dialog>` modals have no top layer and no pointer `inert`.** Sequential focus navigation and Esc treat the document outside an open modal as inert (Tab cycles inside the dialog, Esc cancels it wherever focus sits, `showModal()` / `close()` move and restore focus per the HTML focusing steps), but the modal is not promoted to a top layer — authors position and z-order it — and clicks outside it still reach the underlying content. `::backdrop` paints.
- **Form controls have no separate default value: the `value` / `checked` attributes *are* the live state.** Typing into an `<input>` / `<textarea>` mirrors the text into the `value` attribute on every keystroke, and toggling a checkbox sets or removes `checked`, so `defaultValue` / `defaultChecked` (the attribute as originally authored) are not preserved and `<form>` reset fires the `reset` event without restoring anything. Consumers that read the attribute get the current value, which is the behavior downstream code relies on today. Tracked as `FORM-DEFAULTS-1`.
- **`change` fires on number-step and checkbox/radio toggle**, not on text-input blur.
- **Event dispatch is synchronous**, as on the web; there is no task queue, only the runtime's `queue_microtask` / timers. Listener order follows DOM §2.9 (capture pass, then bubble pass, with the target in both). Microtask checkpoints run after every timer / interval / rAF callback as HTML prescribes; the residual deviation is input: the runtime reads a burst of terminal events per tick and checkpoints once after the burst, not after each input event, and a bare `Dom::dispatch_event` outside an `App` never drains microtasks.

### Selection & editing

- **Selection is single-range only.** Multi-range selection (Ctrl-click in browsers) is not supported.
- **Selection collapses to `None` when an endpoint's node is detached**, rather than relocating to *(parent, index_where_node_was)* per WHATWG DOM removing steps §4.2.5. Affects both `anchor` and `focus` endpoints — if either is inside a detached subtree, the entire selection is cleared. A `Mutation::SelectionChanged { prev, next: None }` fires; consumers wanting "the selection now lives just where the removed thing was" must reconstruct that semantic themselves. Pay down with a proper boundary-point relocation walk when a consumer trips on it.
- **Undo/redo fires `input` only.** `beforeinput` does not fire on history transitions.
- **Caret paint composes `caret-color` (cell background) and the rdom-extension `caret-text-color` (glyph foreground).** When either is `auto`, the painter uses the inverse of the cascaded fg/bg at the caret cell — matching browser visual semantics without the legacy SGR-7 REVERSED modifier. There is no `::caret` pseudo-element, no blink animation, and the terminal's hardware cursor stays hidden.
- **`caret-text-color` is an rdom extension**, not a CSS Working Group property. Browsers have no glyph-color knob for the caret (the underlying cell is the user's font). Terminals paint full cells, so rdom exposes the glyph color separately to let authors tune contrast against `caret-color`.
- **Disabled form controls have `user-select: none`** in the UA stylesheet, blocking drag-selection inside them. Matches Firefox; Chromium permits selection inside disabled controls.

### Runtime & focus

- **The focus indicator is a typed vocabulary, not a universal outline** (`FOCUS-VOCAB-1`). The web shows focus with an *outline* (a non-destructive ring around the box) on **every** focusable element. A TUI can't draw a no-reflow ring — a border would consume a cell and reflow — so there's no single generic cue. Instead the UA stylesheet matches the affordance to the element kind:
  - **Atomic controls** (`button`, `input`, `textarea`, `select`, `summary`, `a`, `area`) → a `:focus { background }` tint. A fill reads as "focused" here because the box *is* the control. `input/textarea/select` keep `!important` only to beat their own field background; the rest are non-important and author-overridable.
  - **Scroll containers** (anything that clips and overflows) → the **scrollbar thumb glyph** of the scroll region the keyboard scrolls turns accent (foreground — a colored handle rather than a filled block); other thumbs are gray. That region is the nearest *overflowing* scroll ancestor of the focus, which no selector can express, so the runtime keeps the attribute `data-rdom-scroll-focus` on it (updated every frame from the previous layout) and the UA sheet styles `[data-rdom-scroll-focus]::scrollbar-thumb`. Authors may match the attribute too. Zero extra area — it reuses chrome the scroll already owns. Such regions are also keyboard-focusable when they're the focus target (see below) and keyboard-scrollable from wherever the focus is.
  - **Grid / tree / listbox** → the internal cursor (active cell/row) is the cue.
  - **Everything else** (a bare focusable `<div>`/`<table>` with no scrollbar) → **no default cue**; the consumer expresses focus in CSS (e.g. `:focus { border-color }`). A full-area background fill on a large container is destructive and unlike the web's outline, so the substrate does *not* apply one. This replaced the old generic `:focus` tint and its per-element opt-out hacks (`canvas:focus`, `[role=tree]:focus`).
- **`pointer-events` supports `auto` and `none` only.** The SVG-era values (`visiblePainted`, `stroke`, …) have no cell-grid meaning and are invalid. `none` falls through to what is beneath and `auto` descendants are hittable, as on the web.

### ARIA tree (no `<tree>` element on the web)

The web platform has no tree element — trees are built from `role="tree"` / `role="treeitem"` / `role="group"` on `<ul>` / `<li>`. rdom ships that pattern as a native built-in (`runtime::builtins::tree` + UA rules), which means it acts on `role` and `aria-*` where the web platform treats them as accessibility metadata only:

- **`role=tree/treeitem/group` drive built-in styling + behavior.** UA rules key box model, disclosure, and highlight off these roles; the runtime keys keyboard/pointer behavior off them. On the web these are ARIA semantics with no intrinsic styling/behavior.
- **Branch-ness keys on `aria-expanded` *presence*, not child count.** A branch with no loaded children (lazy/async) still renders as a branch with a chevron. `aria-expanded="true|false"` toggles open/closed; absent = leaf.
- **`aria-busy="true"` is the loading-state hook.** App-driven and app-styleable (`[role=treeitem][aria-busy=true] { … }`); rdom ships no built-in busy affordance.
- **The expand/collapse arrow (`▼`/`▶`) is painted into the arrow field**, not generated via `::before`. (Forced by the mixed-content pseudo gap — see `TREE-BFC-PSEUDO-1` in `TECH_DEBT.md`.) Layout is 2 cells per nesting level: `[ancestor trunks][connector ├ /└ ][arrow ▼ /▶  or blank][label]`, the connector aligning under the parent row's arrow column.
- **Tree guide lines (`│ ├ └`) have no web equivalent.** They're emitted as border-glyph contributions and reuse the border joiner. Their color comes from the treeitem's `border-color` (`[role=treeitem] { border-color: … }`) even though the item paints no actual CSS border.
- **A `[role=treeitem]`'s background is painted as a full-width row by the guide pass**, not by the normal box fill. On the web a `<li>`'s background covers its own (indented) box and would tint the nested child list; rdom instead fills the entire tree-width row — edge to edge, *including the guide gutter to the left of the indented box* — for exactly the cursor / `aria-selected` row, so the highlight reads as a full row and doesn't bleed onto the open subtree.
- **Active descendant, not roving DOM focus.** The `[role=tree]` container holds focus; the cursor row carries an internal `data-rdom-active` marker and highlights only while the container is focused. The container itself gets no background fill on focus — the focus tint is scoped to atomic controls (`FOCUS-VOCAB-1`), so a tree (or any container) is never flooded; its cue is the active row. A tree is `overflow: visible` by default (like any `<ul>`); a consumer that wants the tree to own its scroll (and thus show the accent `:focus-within::scrollbar-thumb` cue) sets `overflow-y: auto` + a bounded height on the tree itself — a layout choice, not a UA default.
- **Built-in keys are Arrows + Home/End + Enter/Space only** — no vi keys (`j`/`k`/`g`/`G`); apps add those. Every default action is `preventDefault`-overridable.

### Clipboard

- **System clipboard via `arboard` with OSC 52 fallback** for SSH/tmux. Read/write text only.
- **No `ClipboardItem`, no MIME types, no clipboard permissions model.**
- **Copy serializes selected text preserving visual line breaks** per `white-space: normal` collapse rules.

### Timers & animations

- **Sub-tick precision is the tick rate** (~16ms while animating, ~50ms idle). `setTimeout(fn, 10)` fires at the next tick, not at exactly 10ms. The HTML 4ms nested-timeout minimum clamp does not apply.
- **Transitioning to or from `auto` width/height is declined** per CSS L1.
- **Not implemented:** `@keyframes`, `animation-*` properties, the Web Animations API, `requestIdleCallback`, `cancelIdleCallback`, `setImmediate`, scroll-linked animations.

### HTML parsing (`rdom-parser`)

`rdom-parser` is a strict template parser, not a browser tree constructor. It follows the HTML tokenizer where that costs nothing (text `<`, RAWTEXT / RCDATA elements, character references, DOCTYPE skipping) and departs where recovery would hide author mistakes:

- **Malformed markup is an error, not repaired.** A missing or mismatched end tag (`<div><p>a</div>`), an unterminated attribute, or EOF inside a tag returns `ParseError` with line / column and a hint; HTML's tree-construction recovery (implied end tags, foster parenting, adoption agency) is not implemented.
- **`<tag/>` self-closes any element**, not just void and foreign elements (HTML ignores the `/` on a non-void HTML element).
- **Attribute names keep their case** (HTML lowercases them); the DOM's attribute lookup is exact-match. Tag names are lowercased as in HTML.
- **Duplicate attributes: the last one wins** (HTML keeps the first), except `class`, whose tokens are unioned into the class list (`<p class="a" class="b">` has both classes; HTML keeps only `a`).
- **Character references** follow HTML §13.2.5.72–80: the WHATWG table, longest-prefix matching, legacy no-`;` names, the attribute-value `=` / alphanumeric caveat, digit-only numeric scanning, the C1 → Windows-1252 remap, U+FFFD for U+0000 / surrogates / out-of-range. No departure; the tokenizer's recoverable parse errors (unknown name, missing semicolon, control / noncharacter code points) are simply not reported, as stated above.
- **`<!DOCTYPE …>` is dropped with no `DocumentType` node** (`Dom` has no such node type). Other `<!…>` and `<?…>` become Comment nodes, as in HTML's bogus-comment state.
- **Whitespace is preserved verbatim** in text nodes, including inter-element whitespace; collapsing happens in `rdom-tui`'s layout per `white-space`, as in the browser's rendering (not parsing) pipeline. The one tokenizer-level exception is honored: a newline right after `<textarea>` is dropped (HTML §13.2.6.4.7). The same rule for `<pre>` / `<listing>` is **not** implemented — `<pre>` keeps a leading newline.
- **`</` followed by a non-letter** ends the current element's children and is then an error, rather than becoming a bogus comment. At the top level any `</…` is an error ("unexpected closing tag at top level") instead of silently truncating the template.

- **Ordered lists count through UA rules, not `display: list-item`.** `ol { counter-reset: list-item } li { counter-increment: list-item } ol > li::before { content: counter(list-item) ". " }` ship in the UA stylesheet; `display: list-item`, `list-style-type`, `::marker`, `counters()` (the nested string form), `counter-set`, `<ol start>`, `<ol reversed>` and `<li value>` are not implemented. `ul`, `ol` and `menu` all reset `list-item` (HTML §15.3.8), so a nested bullet list does not advance the enclosing numbering. Authors override the marker with their own `ol > li::before` rule.

## 3. Not yet shipped

Common web-platform surface rdom omits entirely as of 0.4.x. Schedule lives in [`DESIGN.md`](DESIGN.md#roadmap).

- **`transition-behavior: allow-discrete`** (CSS Transitions 2): discrete properties (`display`, `position`, …) never transition; `transition-property: display` (or any other `<custom-ident>`) parses and is inert, exactly as Transitions Level 1 behaves without the Level 2 property.

- **Form validation:** `pattern`, `required`, `minlength`/`maxlength`, `ValidityState`, `checkValidity()`, the `:valid` / `:invalid` / `:required` pseudo-classes, constraint-validation API.
- **Smooth scrolling.** `scroll-behavior` is not parsed; every scroll is instant.
- **Live `<style>` sheets.** A `<style>` element's text is snapshotted when the stylesheet is built; editing its text later does not re-parse. (Inline `style="…"` *is* live — the CSSOM observer re-parses it on every attribute write.)

## 4. Known limitations within shipped features

- **Pseudo-element transitions cover paint properties only.** `::before` / `::after` transition `color`, `background-color` and `border-color`; a positioned pseudo-element's `width` / `height` / insets switch discretely, and `::backdrop`, `::selection` and the `::scrollbar*` pseudo-elements do not transition at all. Reason: pseudo geometry is resolved inside the host's layout pass, which has no per-frame override slot yet, and the other pseudo-elements have no presentation slot.

- **Opacity nesting is flat.** A child with `opacity: 0.5` under a parent with `opacity: 0.5` renders at `0.5`, not the CSS-correct `0.25`. CSS group rendering would require an off-screen buffer; not implemented.
<!-- The two `border-collapse` simplifications (corner-glyph last-paint and "outermost wins" conflict resolution) were retired by the BORDER-MODEL-1 initiative. The current contract — CSS Tables 3 §11.5 algorithm, hidden kill-switch, glyph-and-color from the same winner — lives under "Layout" above and supersedes both prior entries. -->

