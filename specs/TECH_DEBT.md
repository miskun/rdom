# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md). This file is for the *temporary* simplifications and known follow-ups.

## Open

### Layout & cascade

- **`D-M2-2` — Static-position resolution simplified.** When `top: auto; bottom: auto` (or `left: auto; right: auto`), CSS uses the "hypothetical in-flow position" the element would have had. rdom simplifies to "containing block top-left edge." Real CSS resolution would require phase-1 to track hypothetical positions for absolute children — substantial layout work. Lift if real apps trip on it.
- **`D-M2-3` — Flat root-level z-sort instead of nested stacking contexts.** Correct for tooltip / dropdown / modal patterns; doesn't match CSS Appendix-E for nested stacking. Apps wanting truly local z-order can wrap in `position: relative; z-index: 0`.
- **`D-M2-4` — Negative z-index paints in flat sort order.** CSS 2.1 paints negative-z positioned elements *before* non-positioned content. rdom paints in pure `(z, doc_order)` ascending order — so a negative-z positioned element paints behind other *positioned* elements but ABOVE unpositioned content.
- **`D-M5N-8` — Positioned-pseudo content does NOT prefix-shift on left-edge viewport clip.** When a pseudo's layout rect starts at a negative viewport x, `layout_rect_to_grid` clips the rect's start to `clip.x` and `paint_text` writes the text's prefix at the clipped origin — the FRONT of the content shows where the back should have. Right-edge clip is correct (suffix dropped). Real CSS would shift the text rendering by the clipped-off cell count. Fix when prefix-shift becomes visible in real UI.
- **`M5-MIN-CONTENT-1` — `min-width: auto` resolves to intrinsic natural size, not strict CSS min-content.** Avoids a separate min-content measurement pass; diverges from CSS for multi-word content (rdom holds the full text width, CSS allows shrinking to longest-word width with line wrap). Pay down with a min-content-specific measurement walker if a real consumer hits the difference.
- **`M5-MIN-AUTO-1` — `overflow: hidden → min-width: auto = 0` CSS exception not implemented.** Flex items with `overflow: hidden` still hold their intrinsic width — a tighter clip than CSS would produce. One extra branch in the resolver.
- **`M5-MARGIN-1` — Margin collapsing between adjacent block boxes not implemented.** Deferred with documented divergence (flex children never collapse in CSS anyway, so the divergence is narrow — only adjacent block boxes, a rare TUI shape).
- **`M5-COLLAPSE-1` — `border-collapse` style conflict resolution simplified to "outermost wins".** CSS table-cell rules cascade through "hidden > double > solid > dashed > dotted > inset > outset > ridge > groove > none", then by width, then by color. Pay down when a real consumer hits mixed-style edges.
- **`M5-STICKY-1` — `position: sticky` v1 honors `top` and `left` only.** `right` / `bottom` insets and nested-scroller containing-block edge cases not implemented.

### Cascade misses

- **`D-M1-3` — `text-decoration: line-through` is a no-op.** Parser accepts the keyword, paint writes nothing. Substrate fix: `strikethrough` field on `TuiStyle` + `Modifier::STRIKETHROUGH` bit + paint pass support.
- **`D-M1-4` — Inline-style caching is one-shot.** `seed_inline_styles` runs once before `App::new`. Mutating the `style="…"` attribute later does not re-trigger parsing. Apps either call `seed_inline_styles` again or use the typed `set_inline_style(TuiStyle)` API.

### Animations

- **`D-M3-2` — Timing function parser is named-keyword only.** `cubic-bezier(a, b, c, d)` and `steps(n, position)` are not parsed. The bezier evaluator inside `TimingFunction::ease()` is parameterized — only the parser needs the follow-up.
- **`D-M3-3` — Pseudo-element transitions deferred.** The cascade produces `computed_before` / `computed_after` / `computed_backdrop` / `computed_selection` on `TuiExt`, but `diff_and_register` only inspects the main `computed` slot. Apps can't transition pseudo-element styles.
- **`D-M3-5` — Microtask integration runs three drains per tick.** Could batch into one. Profile-driven if it becomes hot.
- **`D-M3-6` — Discrete properties under `transition: all` are not midpoint-toggled.** CSS L1 says discrete properties (display, position, content, …) under `transition: all` switch at midpoint; rdom's diff loop only registers animations for properties in the animatable enum. Apps explicitly transitioning a discrete property via `transition-property: display` get a warning.

### Paint pipeline

- **`OPACITY-1` — Group rendering for proper CSS opacity composition.** The current 0.1.0 implementation composites per-paint-op (cell-level), which works correctly for the common cases (element with bg + opacity, z-stacked translucent elements). Two divergence pockets remain:
  1. **Pseudo + own bg + opacity** — pseudo elements carry `bg` in the glyph style (they have no upstream `fill_bg`). Under `opacity < 1.0`, the compose pipeline blends pseudo's bg against the cell's already-blended element bg — a small color shift.
  2. **No opacity multiplication / no CSS group rendering** — a parent with `opacity: 0.5` containing a child with `opacity: 0.5` renders the child at 0.5, not the CSS-correct 0.25.

  Proper fix is subtree group rendering: each element with `opacity < 1.0` renders its entire subtree into a temporary `Buffer` at full opacity, then composites that buffer at the element's opacity. Eliminates both pockets. Defer until a real consumer hits a case the approximation produces wrong output for.

### UA stylesheet

- **`UA-OL-1` — `<ol>` UA marker is a bullet, not a counter.** Phase F shipped `ol > li::before { content: "• " }` as an honest fallback (a static `"1. "` marker would lie about ordering). When CSS counters land, upgrade to `content: counter(list-item) ". "`.
- **`UA-SB-1` — Scrollbar thumb `content` is single-pseudo, applies to both axes.** Authors who override `content` get the literal glyph on BOTH axes. WebKit's `::scrollbar-thumb:vertical` / `:horizontal` is the documented future migration target.

### Substrate gaps

- **`SUB-2` — IFC detection requires at least one inline-element child.** `is_ifc_block` returns `false` for a block whose only children are text nodes. CSS treats a text-only `<p>` as an IFC; rdom doesn't. Workaround: append an empty `<span>`.
- **`SUB-3` — UA `<style>` rule placed at end of defaults array.** The rule for `style { display: none }` was added at the end to avoid breaking source-order-indexed tests. Cosmetic. Pay down when the UA stylesheet is reorganized.
- **`SUB-4` — `i16` ceiling on `Length::Cells`.** Positioning offsets max out at ±32k cells. Probably forever-fine; flag if anyone reaches for a virtualized scroll surface beyond the 16-bit range.
- **`SUB-5` — Scrollbar gutter is always reserved, never conditional.** Elements with `overflow-x: auto` or `overflow-y: auto` reserve a 1-cell gutter on the matching axis even when content doesn't overflow (browser equivalent: `scrollbar-gutter: stable`). Cost: an unused 1-cell margin on the right (or bottom) of every auto-overflow element. Visible on `<textarea>` and any custom scrollable container. Pay down by either (a) reserving only when content actually overflows (re-layout cost on content changes), or (b) adding the `scrollbar-gutter` property so authors can choose between `stable` / `auto` / `both-edges`.

### DRY opportunities

- **`DRY-1` — "Skip out-of-flow children" filter in 4 places.** `flex::layout_children`, `layout_fragment_children`, `paint_pass::recurse_children`, and `hit_test::descend_children_reverse` all have nearly identical filters for `display: none` + `position: absolute | fixed | sticky`. Factor to a shared `is_in_flow(dom, id) -> bool` helper.
- **`DRY-2` — Paint-vs-hit-test z-list collection duplicated.** Both passes walk the tree and collect positioned elements in nearly identical loops; only the sort direction differs. Could share a `collect_positioned` helper. Profile first; cost has to justify the abstraction.

### Process

- **`OPS-4` — Snapshot-pin remaining six examples.** The paint-snapshot harness exists (`crates/rdom-tui/tests/common/mod.rs`) and `ua_chrome` + `app_shell` have goldens. The six older examples (`counter_button`, `tab_form`, `scrollable_list`, `selectable_text`, `parse_and_render`, `dom_api_demo`) have no goldens — cascade/layout/paint regressions there would only surface via a downstream consumer issue. Pattern is mechanical.

## Accepted simplifications (forever-state)

These won't be paid down — they reflect deliberate architectural choices.

- **`D-M1-1` — `from_css` is a free function in `rdom-css`, not `impl Stylesheet`.** Original draft proposed inherent methods on `Stylesheet`. Couldn't ship — `rdom-tui` already depends on `rdom-css` for the inline-style cascade rung; making the inverse import work would require either a cycle or splitting `Stylesheet` to a third crate.
- **`D-M1-2` — `<style>` extraction + inline-style seeding are free functions called explicitly**, not auto-runs in `App::build`. Same cycle constraint as `D-M1-1`. Apps call `extend_from_style_tags(&dom, &mut sheet)` and `seed_inline_styles(&mut dom)` between `parse_into` and `App::new`.

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
