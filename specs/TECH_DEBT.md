# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. The [`STABILIZE-2026-09`](STABILIZE-2026-09.md) program is paying every open item down before 0.5.0; each row's disposition is in that file. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md). This file is for the *temporary* simplifications and known follow-ups.

## Open

### Style crate — from the STABILIZE-2026-09 Phase 3+4 gates

- **`STYLE-DISPATCH-SPLIT-1` — `property_dispatch.rs` (~2 300 lines) mixes the field table, `set`, `serialize` and their tests.** Split into `property_dispatch/{table,set,serialize}.rs` (tests alongside); no behavior change.
- **`STYLE-VALUES-SPLIT-1` — `parse/values.rs` (~1 800 lines) holds every value parser.** Split per value type (`color`, `length`, `calc` entry, `transition`, `content`); no behavior change. Also over the few-hundred-line bar: `layout.rs`, `ua.rs`, `tui_style.rs`, `stylesheet.rs`, and in `rdom-tui` `cascade/apply.rs` and `cascade/tests.rs`.

### Deferred from HARDENING-2026-09 Batch 3

- **`CARET-REVEAL-STALE-LAYOUT-1` — `reveal_caret` runs at edit time against the previous frame's layout.** A wrap-induced new line is revealed one keystroke late, and a `scroll` listener can observe an over-maximum offset for the frame before layout clamps it (`ClampTo::NextLayout`). Root fix: a "pending caret reveal" flag serviced by the App after `layout_dom`.
- **`POINTER-EVENTS-IFC-1` — `pointer-events: none` on an IFC block hides its `auto` inline descendants from hit-testing** (`hit_test::descend` recurses into element children, which have no box rects inside an inline formatting context) and skips the block's scrollport clip check. Resolve through `hit_fragment` → owner → nearest ancestor's `pointer_events`; keep the clip test for transparent elements. Only the positioned-children case is tested.
- **`FLEX-RS-SPLIT-1`** (`layout_pass/flex.rs` ~950 lines: main-axis resolution, freeze loops, placement, cross sizing), **`SCROLLBAR-SPLIT-1`** (`runtime/scrollbar/mod.rs` ~750: hit / geometry / drag / autoscroll / reveal), **`HIT-TEST-SPLIT-1`** (`runtime/hit_test/mod.rs` ~750: descend / nearest / fragment), **`SELECT-SPLIT-1`** (`builtins/select/mod.rs` ~750: click / keyboard / type-ahead / dropdown) — over the few-hundred-line bar; split by concern, no behavior change. Also untouched but over the bar: `cssom/declaration.rs`, `render/buffer.rs`, `render/virtual_screen.rs` (test-only VT emulator that should be `cfg(test)` / a `test-util` feature rather than a public re-export).

- **`APP-MOD-SPLIT-1` — `runtime/app/mod.rs` (~1350 lines, 19 fields) mixes the loop, the stylesheet stack, the autoscroll session, and the clipboard / undo / editable key defaults.** Split into `app/{keyboard_defaults,autoscroll,stylesheets}.rs`; no behavior change.
- **`INLINE-PAINT-SPLIT-1` — `paint_pass/inline_paint.rs` (~950 lines) mixes text paint, the selection overlay, the caret, and `<select>` / password / gauge chrome substitution.** Paint knowing builtin internals is the coupling the module doc forbids; move chrome substitution behind a runtime-registered hook and the caret / selection overlay into their own files.
- **`PACKER-STRING-ALLOC-1` — the line packer allocates one `String` per grapheme** (`render/inline/packer.rs` `PendingGrapheme { text }` + `g.to_string()`), and text is packed at least twice per frame (intrinsic measure + layout). Store byte ranges into the source text and cache the packed layout per `(node, width)` within a frame.
- **`PAINT-INLINE-LAYOUT-CLONE-1` — paint clones each element's `InlineLayout` / `anonymous_blocks`** (every fragment `String`) per frame (`inline_paint.rs` `paint_ifc`, anonymous-box paint). Borrow through the ext or put the layout behind an `Rc` like `computed`.
- **`SGR-ALLOC-1` — SGR emission allocates a `Vec` per changed cell** and formats each code with `write!` (`render/sgr.rs`). Minor, but it is the byte hot path.

### Paint pipeline

- **`OPACITY-1` — Group rendering for proper CSS opacity composition.** The current 0.1.0 implementation composites per-paint-op (cell-level), which works correctly for the common cases (element with bg + opacity, z-stacked translucent elements). Two divergence pockets remain:
  1. **Pseudo + own bg + opacity** — pseudo elements carry `bg` in the glyph style (they have no upstream `fill_bg`). Under `opacity < 1.0`, the compose pipeline blends pseudo's bg against the cell's already-blended element bg — a small color shift.
  2. **No opacity multiplication / no CSS group rendering** — a parent with `opacity: 0.5` containing a child with `opacity: 0.5` renders the child at 0.5, not the CSS-correct 0.25.

  Proper fix is subtree group rendering: each element with `opacity < 1.0` renders its entire subtree into a temporary `Buffer` at full opacity, then composites that buffer at the element's opacity. Eliminates both pockets. Defer until a real consumer hits a case the approximation produces wrong output for.

- **`TREE-BFC-PSEUDO-1` — the `::before` / `::after` prefix on a true mixed-content block is dropped.** The duplicate-text class of this bug is fixed (own text no longer double-paints on pseudo- or mixed-content blocks); what remains is that a block with both a pseudo `content` and mixed inline + block children paints the pseudo nowhere. Emit the pseudo into the first / last anonymous box's line.

### UA stylesheet

### Substrate gaps

### Events

- **`SHOWCASE-EVT-1` — Event handlers cannot mutate the App's stylesheet stack.** Listeners registered via `dom.add_event_listener` receive an `EventCtx` with mutable access to the `Dom` but NOT the `App`. They can mutate the tree freely (`append_child`, `clear_children`, `set_attribute`) but cannot call `App::push_stylesheet` / `App::remove_stylesheet` because the App owns the cascade pipeline and event dispatch happens inside its outer loop. The showcase works around this by pre-pushing every demo's stylesheet at startup and relying on each demo's class-scoped selectors (`.flex-row-demo`, `.hover-demo`, etc.) for isolation — an unenforced-in-substrate convention pinned by a registry-level test in `rdom-showcase`. The workaround scales linearly: at N demos, every cascade pass walks N× the rules per element regardless of which demo is mounted. **Pay down with a substrate hook** — either (a) lifecycle observers the App installs that can mutate sheets in response to mutation records, or (b) a deferred-queue API on `EventCtx` for "App-level intents" the dispatch loop drains after the current event completes. Option (b) is the smaller change; option (a) is the more general one and aligns with how M5's implicit-event work needs to thread through the App anyway. **Defer to M5 or M7** — M5 already touches the App / observer plumbing for `EVT-DETACH-1`; do it then.

### Forms

- **`FORM-DEFAULTS-1` — no `defaultValue` / `defaultChecked`, so `<form>` reset cannot restore anything.** The IDL value and the content attribute are conflated: `builtins/input::mirror_to_attribute` writes typed text into `value` and the toggle builtin flips the `checked` attribute itself. HTML keeps the attribute as the default and a separate dirty value. Fix = a per-control default snapshot (`TuiExt` or a `data-rdom-default-*` attribute captured on first edit) that reset restores, then stop mirroring — a consumer-visible change (`DIVERGENCES.md` documents the current behavior), so it ships with a major-line bump and a CHANGELOG migration note.

### Editing

- **`EDIT-2` — `user-select: contain` clamps only via the host's outer layout rect.** The clamp uses the mouse coordinates against the host's `layout_rect()`; it doesn't consult per-line content extents. Good enough for single-paragraph contain hosts; multi-paragraph contain hosts with internal gaps may clamp to the wrong end if the mouse lands in inter-paragraph whitespace. Pay down by clamping to the nearest in-host inline fragment instead.

## Accepted simplifications (forever-state)

These won't be paid down — they reflect deliberate architectural choices.

- **`D-M1-1` — `from_css` is a free function in `rdom-css`, not `impl Stylesheet`.** Original draft proposed inherent methods on `Stylesheet`. Couldn't ship — `rdom-tui` already depends on `rdom-css` for the inline-style cascade rung; making the inverse import work would require either a cycle or splitting `Stylesheet` to a third crate.
- **`D-M1-2` — `<style>` extraction + inline-style seeding are free functions called explicitly**, not auto-runs in `App::build`. Same cycle constraint as `D-M1-1`. Apps call `extend_from_style_tags(&dom, &mut sheet)` and `seed_inline_styles(&mut dom)` between `parse_into` and `App::new`.

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
