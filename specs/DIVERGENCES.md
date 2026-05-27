# DIVERGENCES — where rdom departs from the web platform

The rdom default rule is: **track the web platform.** WHATWG DOM, CSS Working Group specs, and UI Events are the reference. If a behavior is not listed here, it should match the web platform within the supported subset.

This document collects every deliberate departure that ships in 0.1.0. Departures fall into three groups:

1. **TUI medium constraints** — fixed by the fact that we render to a character grid.
2. **Simplifications** — places where rdom keeps a smaller model than the web platform.
3. **Not yet shipped** — common web features that 0.1.0 omits.

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
- **Border conflict resolution follows CSS Tables 3 §11.5 in priority order:** `hidden` wins absolutely → `none` loses absolutely → border-style ranking (`double > solid > dashed > dotted > ridge > outset > groove > inset`) → child wins over ancestor (more-nested element outranks less-nested) → earlier DOM order wins among same-depth siblings (CSS rule 6: leftmost / topmost in LTR geometric order maps to earliest in DOM). The winning border contributes BOTH glyph and color; paint never picks one from the winner and one from a loser.
- **Border-style support is the full CSS keyword set, with terminal-faithful degradation:** `none`, `hidden`, `solid`, and `double` render with distinct glyphs (`│─┌┐└┘` / `║═╔╗╚╝`). `dashed`, `dotted`, `ridge`, `outset`, `groove`, `inset` parse and rank correctly in conflict resolution but render as `solid` (matching CSS's "render as best you can on this medium" principle — the data model is faithful even where the glyphs aren't yet distinguishable).
- **No margin collapsing between adjacent block boxes.** Margins are additive.
- **`min-width: auto` / `min-height: auto` content size suggestion uses intrinsic natural size**, not strict CSS min-content (longest-word width with wrap). The auto-min resolution itself follows CSS Flexbox §4.5 — `min(content_size_suggestion, specified_size_suggestion)`, dropped to 0 when overflow on the axis is non-visible. The approximation is in HOW `content_size_suggestion` is computed (natural width vs. longest-word). Tracked as `M5-MIN-CONTENT-2` in `TECH_DEBT.md`.
- **`flex: <N>` shorthand collapses `<basis>` to 0%.** rdom's parser stores both `flex: 1` and `flex: 1 1 auto` as `Size::Flex(1)`; the basis token is parsed-and-accepted but ignored. Effective behavior: any `flex: <N>` form is treated as `flex: <N> 1 0%`. Authors who need `flex-basis: auto` semantics (specified suggestion = the item's `width`) write `width: auto; flex-grow: 1` instead. Parser doc at `crates/rdom-style/src/parse/values.rs::parse_flex_shorthand` documents the ignore; this entry surfaces the user-visible consequence.
- **`aspect-ratio` requires the explicit `<w>/<h>` form.** Bare numbers, decimals, and the `auto && <ratio>` fallback form are not parsed. Half-to-even (banker's) rounding is used when discretizing onto the cell grid.
- **Anonymous block boxes (CSS 2.1 §9.2.1.1), not anonymous inline boxes.** Mixed inline + block children inside a block-flow container produce **anonymous BLOCK boxes** wrapping each inline run (text + `display: inline` + atomic `display: inline-block`). Each anon box establishes its own IFC. This matches CSS for block containers; what rdom does NOT yet generate is anonymous *inline* boxes for the `<span>foo <span>bar</span> baz</span>` text-around-inline-around-text shape inside an existing IFC (inline-ancestor-breaking). Texts inside one inline element render as a single fragment; nested inline-ancestor-breaking is deferred.
- **No floats.** `float: left` / `float: right` are not parsed and not supported. CSS float layout (line-box exclusion, clearance, float resolution) is not in scope for rdom; flex / block / IFC cover the TUI use cases. Authors targeting browser-faithful float behavior should restructure with flex.
- **Inline backgrounds only.** Inline borders are not painted.
- **No `display: grid`.** Flexbox is the only multi-axis layout. `display: inline-block` ships for content-hugging chrome (buttons, badges, tags).
- **`text-align`, `vertical-align`, `text-decoration: line-through` are not implemented.** Left alignment only; single baseline.

### Positioning

- **Stacking is flat at the document root.** `z-index` is parsed; paint order is document order with z-sort at the root only. No nested stacking contexts.
- **`position: sticky` v1 honors `top` and `left` only.** `right` / `bottom` insets are not implemented.
- **Sticky containing block is the element's parent's content box**, not the CSS "nearest scroll container" for nested-scroller edge cases.
- **Static position for absolutely-positioned elements with both edges `auto`** resolves to the containing block's top-left edge (not the CSS "hypothetical in-flow position").
- **No `transform`, `rotate`, `scale`, `matrix`, `isolation: isolate`, `will-change`.**
- **Positioned `::before` / `::after` pseudo-elements are not in the hit-test set.** Clicks on pseudo rects resolve to the underlying element.

### Values

- **`calc()` accepts both `5+5` and `5 + 5` inside the call.** CSS Values L3 requires whitespace around `+`/`-`; rdom's tokenizer doesn't preserve whitespace so the parser accepts either form. `*` and `/` don't need whitespace in CSS either, so those match.
- **`calc()` on `padding` / `margin` / `gap` is constant-only.** Width/height/top/right/bottom/left support full layout-time `calc()` with percentages. Padding/margin/gap support `calc()` with constants only (`padding: calc(2 * 3)`); percent-bearing forms reject at parse time. The narrow gap is tracked as `CALC-PADMARG-1` in `TECH_DEBT.md`.
- **`calc()` does NOT support `min(...)` / `max(...)` / `clamp(...)`.** CSS Values L4 functions. Deferred.
- **CSS transitions don't smoothly tween between `calc()` values.** When either endpoint of a `transition` carries a `calc()` expression (Size or Length axis), the engine snaps at midpoint instead of interpolating. Smooth tweening would require resolving both endpoints to concrete cells using the current layout's parent dimensions at every animation tick — straightforward but unwired in M6.

### Cascade & selectors

Supported selector grammar: type, class, ID, attribute, descendant, child (`>`), adjacent sibling (`+`), general sibling (`~`), comma list. Supported pseudo-classes: `:hover`, `:focus`, `:active`, `:checked`, `:indeterminate`, `:open`, `:first-child`, `:last-child`, `:not(<simple>)`, `:placeholder-shown`.

- **`:not()` accepts simple selectors only.** Complex `:not(X Y)` is not parsed.
- **Not implemented:** `:nth-child(an+b)`, `:nth-of-type`, `:only-child`, `:has()`, `:is()`, `:where()`, `:focus-within`, `:focus-visible`, `:disabled`, `:enabled`, `:read-only`, `:read-write`, `:required`, `:invalid`, `:valid`, `:modal`.
- **Not implemented as author-styleable pseudo-elements:** `::marker`, `::placeholder`, `::caret`, `::first-line`, `::first-letter`, `::scrollbar*`. List bullets use `::before` content; placeholder text uses `:placeholder-shown::before { content: attr(placeholder) }`.
- **`var()` is supported in color positions only.** Other property types do not consume custom properties at parse time.
- **Not implemented:** `calc()`, `min()`, `max()`, `currentColor`, CSS Nesting (`&`).
- **`rgba()` alpha is dropped at parse time.** Translucency is handled by the dedicated `opacity` property, which composes alpha at paint time.
- **At-rules tokenized but bodies skipped** with `WarningKind::UnsupportedAtRule`: `@media`, `@keyframes`, `@supports`, `@import`, `@font-face`, vendor prefixes.

### DOM API shape

The DOM API is Rust-shaped rather than JS-shaped. The semantics match WHATWG DOM; the surface differs in idiomatic ways.

- **Handles are arena IDs (`NodeId`), not object references.** IDs are arena-scoped and never reused within a `Dom`. Comparing IDs across separate `Dom` instances is meaningless.
- **No `Node.prototype` / `Element.prototype` inheritance.** Behaviors attach via Rust trait impls on `NodeRef` / `NodeMut`.
- **Tag and attribute names are case-sensitive.** HTML's ASCII-case-insensitive matching is not applied.
- **Snapshots replace live collections.** `child_ids`, `query_selector_all`, attribute iterators, etc. return `Vec<NodeId>` snapshots or iterators — there is no live `NodeList`, `HTMLCollection`, `NamedNodeMap`, or `DOMTokenList`.
- **`classList` is methods only** (`add_class` / `remove_class` / `toggle_class` / `has_class`); no `DOMTokenList` wrapper.
- **Attributes are a flat `BTreeMap<String, String>`** exposed as iterator pairs. The `Attr` interface is not implemented.
- **No `Window` vs `Document` split.** The TUI `App` plays both roles.
- **`Dom::root()` is a Fragment, not `<html>`.** `<body>` and `<head>` are not auto-inserted.
- **`textContent.len()` returns bytes**, not UTF-16 code units.
- **No XML namespaces.** `namespaceURI`, `prefix`, `*NS` method variants are not applicable.
- **No Shadow DOM, no custom elements registry, no `<template>` cloning semantics.**

### Events

- **`Event.detail` is `Option<String>`**, not a typed event-payload object. There is no `InputEvent`, `MouseEvent`, `KeyboardEvent`, `SubmitEvent`, `TransitionEvent` payload class hierarchy. Transition events encode `"<property>|<elapsed_seconds>"` in `detail`.
- **`isTrusted` is inverted as `is_synthetic`.**
- **Listener removal is via `AbortSignal`**, not function-identity equality on a per-listener `removeEventListener(fn)` call.
- **Not implemented:** `composedPath()`, `initEvent()`, `passive: true`, `CustomEvent` ≠ `Event` distinction.
- **`keyup` requires kitty-keyboard-protocol terminal support.** rdom-tui enables `KeyboardEnhancementFlags::REPORT_EVENT_TYPES` on startup, but the host terminal has to honor it. Supporting terminals (kitty, foot, WezTerm, alacritty 0.13+, recent xterm) fire `keyup` events for every key release. Non-supporting terminals (legacy xterm, basic VT100, most macOS Terminal.app builds) only ever send `KeyEventKind::Press` — `keyup` listeners on those terminals will never fire. No emulation; if your demo needs cross-terminal key-release behavior, derive it from `keydown` + a timer.
- **No `Window` object — events that target `Window` in HTML target `dom.root()` here.** rdom has no chrome around the document grid; the terminal IS the document. Web-spec events that fire on `Window` (today: `resize`; future candidates that follow the same pattern: hypothetical `beforeunload`, `popstate`, etc.) collapse onto the document root. Authors write `dom.root().add_event_listener("resize", …)` where browsers would use `window.addEventListener`. Bubbling, cancelable flags, and dispatch timing match the HTML spec for the corresponding `Window` event; only the target identity differs.
- **`scroll` events bubble even on non-Document targets.** CSSOM View Module §6: `scroll` on `Document` bubbles, but `scroll` on element targets does NOT bubble. rdom fires every `scroll` event with `bubbles = true` regardless of target — a deliberate divergence that lets consumers install a single listener at `dom.root()` and observe all scroll activity (the showcase's scroll-position indicator works this way; per-scrollable installation would require re-wiring on every subtree swap). Suppress propagation explicitly in handlers if needed.
- **`scroll` events are NOT coalesced per rendering step.** HTML5 says multiple scroll-offset writes between rendering steps collapse to one `scroll` event. rdom fires one `scroll` event per mutation-site call: wheel ticks, scrollbar drag deltas, and each programmatic `set_scroll_top` / `scroll_to` write each fire a separate event. For typical terminal-app patterns (one offset write per crossterm event) this matches HTML behavior; bulk programmatic scrolling (e.g., a loop that mutates `scroll_top` ten times) sees ten events where browsers would deliver one. Pay down with a "dirty scroll" set drained at end-of-event-tick if a consumer hits the difference.
- **Not implemented:** `dragstart` / `drag` / `drop` / `DataTransfer`, touch events, IME composition events, pointer events beyond mouse, `wheel` event chaining beyond the first scrollable ancestor.
- **`change` fires on number-step and checkbox/radio toggle**, not on text-input blur.
- **Event dispatch is synchronous and FIFO.** No microtask queue, no `setImmediate`.

### Selection & editing

- **Selection is single-range only.** Multi-range selection (Ctrl-click in browsers) is not supported.
- **Selection collapses to `None` when an endpoint's node is detached**, rather than relocating to *(parent, index_where_node_was)* per WHATWG DOM removing steps §4.2.5. Affects both `anchor` and `focus` endpoints — if either is inside a detached subtree, the entire selection is cleared. A `Mutation::SelectionChanged { prev, next: None }` fires; consumers wanting "the selection now lives just where the removed thing was" must reconstruct that semantic themselves. Pay down with a proper boundary-point relocation walk when a consumer trips on it.
- **Cross-text-node edits inside contenteditable** apply as a single `beforeinput` / `input` pair but are **not recorded on the undo stack** in 0.1.0 — multi-node mutations can't be captured by the per-node `EditEntry` shape cleanly. Tracked for v0.2.0 (compound edit entries).
- **`beforeinput.detail` is a plain `String`**, not a structured `inputType` enum.
- **Undo/redo fires `input` only.** `beforeinput` does not fire on history transitions.
- **Caret paint composes `caret-color` (cell background) and the rdom-extension `caret-text-color` (glyph foreground).** When either is `auto`, the painter uses the inverse of the cascaded fg/bg at the caret cell — matching browser visual semantics without the legacy SGR-7 REVERSED modifier. There is no `::caret` pseudo-element, no blink animation, and the terminal's hardware cursor stays hidden.
- **`caret-text-color` is an rdom extension**, not a CSS Working Group property. Browsers have no glyph-color knob for the caret (the underlying cell is the user's font). Terminals paint full cells, so rdom exposes the glyph color separately to let authors tune contrast against `caret-color`.
- **`user-select: all` selects the host's entire text on click; drag-extend is suppressed.** Matches Chromium behavior.
- **`user-select: contain` clamps drag-extend focus to the host's subtree.** Above the host clamps to start; below clamps to end; same-row past-right clamps to end.
- **Disabled form controls have `user-select: none`** in the UA stylesheet, blocking drag-selection inside them. Matches Firefox; Chromium permits selection inside disabled controls.
- **`readonly` form controls fire `beforeinput` (cancelable) then the UA cancels the edit by default.** Listeners can observe the attempted edit (analytics, validation feedback). No `input` event fires for cancelled edits. Matches UI Events / Input Events Level 2 §5.
- **Shift+Up / Shift+Down extend the selection vertically and share the editor's sticky-x state** with bare Up/Down, so the original column is preserved across clamped short lines. Shift+Home / Shift+End extend to line edges.
- **Vertical caret motion (Up/Down) uses sticky-x**: an `EditorState.sticky_x` field remembers the column the caret started from so traversing short lines doesn't shrink the column permanently. Browsers do the same but call it "preferred caret position."

### Runtime & focus

- **Click is synthesized at the nearest common ancestor** of the `mousedown` and `mouseup` targets, not the `mouseup` hit-test target alone. (This matches the HTML spec and is called out because it's non-obvious.)
- **Pointer capture:** while captured, `mousemove` / `mouseup` route to the captured element regardless of hit; capture releases on `mouseup`.
- **Wheel scrolling auto-scrolls the nearest scrollable ancestor** (first-scrollable-wins). Scroll chaining beyond the first ancestor is not implemented.
- **Focus events dispatch synchronously**, not deferred.
- **`Tab` / `Shift-Tab` route through focus navigation first**, then dispatch as `keydown` if not consumed.
- **`pointer-events` property is not implemented.** Every painted element is hittable.

### Clipboard

- **System clipboard via `arboard` with OSC 52 fallback** for SSH/tmux. Read/write text only.
- **No `ClipboardItem`, no MIME types, no clipboard permissions model.**
- **Copy serializes selected text preserving visual line breaks** per `white-space: normal` collapse rules.

### Timers & animations

- **CSS transitions** support named timing keywords (`linear`, `ease`, `ease-in`, `ease-out`, `ease-in-out`). `cubic-bezier(...)` and `steps(...)` are not implemented.
- **Sub-tick precision is the tick rate** (~16ms while animating, ~50ms idle). `setTimeout(fn, 10)` fires at the next tick, not at exactly 10ms. The HTML 4ms nested-timeout minimum clamp does not apply.
- **Transitioning to or from `auto` width/height is declined** per CSS L1.
- **Not implemented:** `@keyframes`, `animation-*` properties, the Web Animations API, `requestIdleCallback`, `cancelIdleCallback`, `setImmediate`, scroll-linked animations.

## 3. Not yet shipped

Common web-platform surface that 0.1.0 omits entirely. Schedule lives in [`DESIGN.md`](DESIGN.md#roadmap).

- **Form validation:** `pattern`, `required`, `minlength`/`maxlength`, `ValidityState`, `checkValidity()`, the `:valid` / `:invalid` / `:required` pseudo-classes, constraint-validation API.
- **Smooth scrolling.** `scroll-behavior: smooth` is parsed but the animation is not implemented.
- **Live style sheets.** `<style>` block bodies and inline `style="…"` are snapshotted at app start / element creation; live re-parse on subsequent attribute mutation is not implemented.
- **`calc()` value system.** Scheduled for 0.2.0.
- **Event surface:** `dblclick`, `contextmenu`, `keyup`, `mousemove`, `scroll`, `resize`. Scheduled for 0.2.0.

## 4. Known limitations within shipped features

- **Opacity nesting is flat.** A child with `opacity: 0.5` under a parent with `opacity: 0.5` renders at `0.5`, not the CSS-correct `0.25`. CSS group rendering would require an off-screen buffer; not implemented.
<!-- The two `border-collapse` simplifications (corner-glyph last-paint and "outermost wins" conflict resolution) were retired by the BORDER-MODEL-1 initiative. The current contract — CSS Tables 3 §11.5 algorithm, hidden kill-switch, glyph-and-color from the same winner — lives under "Layout" above and supersedes both prior entries. -->

