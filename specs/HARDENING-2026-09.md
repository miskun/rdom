# HARDENING-2026-09 — full-project review, divergence audit, debt paydown

**Status:** IN PROGRESS (started 2026-09-21). Batch 1 (rdom-core) done and gated; Batch 2 (rdom-style + rdom-css) done and gated; Batch 3 (rdom-tui) underway (R4 landed).

Origin: a full review of the workspace at `rdom-tui` 0.3.14 / `rdom-core` 0.3.5 (five grumpy-architect
passes, one per crate group, plus a docs/process pass), followed by an entry-by-entry audit of
[`DIVERGENCES.md`](DIVERGENCES.md) and [`TECH_DEBT.md`](TECH_DEBT.md) against the code. This document is
the program plan: what the review found, which divergences stay and which get replaced by the web
behavior, and the order the work lands in. Per-entry rewrites of the two contract docs happen in the
batch that changes the behavior, so those files stay truthful commit by commit.

Gate at review time: `fmt` clean, `clippy -D warnings` clean, 2759 tests passing. The problems below
are not caught by the suite — they are invariant breaks, spec inversions, and silent misparses.

## 1. Review findings (verified against source)

### Blocking — substrate invariants

| # | Finding | Where | Status |
|---|---|---|---|
| R1 | `NodeId` has no generation; freed slots are reissued and `contains()` says yes for a stale id. Router `hover_target` / `down_target`, `App.autoscroll_container`, `NodeList` snapshots all hold ids across detaches. `DIVERGENCES.md` claimed ids are never reused. | `rdom-core/src/dom.rs` alloc, `node_id.rs` | **fixed** (generational `NodeId`, per-slot generation checked on every lookup) |
| R2 | `compare_document_position` returned `CONTAINS \| PRECEDING` for a descendant argument; spec is `CONTAINED_BY \| FOLLOWING`. `selection_range` and paint were tuned to the inverted bits; ancestor positions ignored their offset. | `rdom-core/src/position.rs` | **fixed** (spec bits + `compare_boundary_points`, DOM §5.2) |
| R3 | A panicking `MutationObserver` left `is_observing = true` and dropped every observer; a panicking listener was silently unregistered. | `observer.rs`, `dispatch.rs` | **fixed** (`catch_unwind` → restore → `resume_unwind`) |
| R4 | Timer API reaches the scheduler through a raw-pointer thread-local installed only in `handle_event` / `tick`; any listener fired from a timer callback, `transitionend`, an injected closure, or the autoscroll synthetic drag panics on `set_timeout`. Installing the guard in the pump would alias `&mut Scheduler`. | `rdom-tui/src/runtime/timers.rs` | **fixed** (shared `Rc<RefCell<Scheduler>>` handle installed on every user-code path; per-call borrows, no raw pointer) |
| R5 | `record_scroll_content_size` measures element children only. Text-only scroll containers (`<textarea>`, `<pre style="overflow:auto">`) report zero content and clamp `scroll_y` to 0 every frame. | `render/layout_pass/mod.rs` | **fixed** (extent includes inline flow + anonymous boxes; one scrolled content rect for paint / hit-test / caret; caret reveal) |
| R6 | Flex shrink/grow apply min/max clamps in a single pass; no freeze-and-redistribute loop (Flexbox §9.7). | `render/layout_pass/flex.rs` | **fixed** (freeze-and-redistribute loop for grow and shrink) |
| R7 | `position: sticky` shifts `layout` / `content_layout` but not `anonymous_blocks[*].rect`; text stays at the pre-stick row. | `render/layout_pass/sticky.rs` | open |
| R8 | CSS decimals are reassembled from integer tokens: `0.05` → 0.5, `1.05s` → 1500ms. Oversized integers become 0 via `unwrap_or(0)`. | `rdom-style/src/parse/{values,token}.rs` | open — real `<number-token>` / `<dimension-token>` |
| R9 | At-rules are never recognized; `@import …;` and `@media {…}` swallow the following rule. The `UnsupportedAtRule` warning is never constructed. No brace-depth tracking, stray `}` corrupts the next selector. | `rdom-css/src/top_level.rs` | open — CSS Syntax 3 §5.4 consumer |
| R10 | HTML parser: `<` followed by a non-letter in text is a hard error; `<style>` / `<script>` are not raw text (yet rdom-tui consumes `<style>` bodies); `<textarea>` is not RCDATA. | `rdom-parser/src/parser.rs` | open |
| R11 | `enter_tui_mode` enables raw mode, then `?`-returns on a failed batched write with no guard constructed: the shell is left raw. | `render/backend_crossterm.rs` | **fixed** (escapes first, raw mode last, undo on failure) |

### Non-blocking, worth fixing

Runtime: Tab traversal reaches `display:none` (closed `<dialog>` buttons are focusable); `<dialog>` has no
focus trap / `inert` / top layer; `<form>` reset doesn't restore defaults; checkbox state flips *after* `click`
(web: before, revert on cancel); `<select>` treats any `size` attribute as listbox; Ctrl-C is swallowed
before `keydown`; `<select>` type-ahead lives in a `thread_local!` on wall-clock time; dblclick timing
reads `Instant::now()` instead of the scheduler clock; `event::poll` errors become an eternal tick spin.

Core: target-phase listeners run in registration order, interleaving capture and bubble (DOM §2.9 fires
capture listeners first at target); `stop_propagation` flags are not reset on re-dispatch;
`add/remove_mutation_observer` don't honor the re-entrancy contract they document; `indexes.rs` does O(n)
`contains` / `retain` per alloc/free; `set_class_name` emits three passes of records; `NodeRef::node_type`
panics on a freed id without a `# Panics` doc. (The review also flagged `bitflags_like!` as exported for one internal use; that was wrong — `rdom-style` consumes it, so the export stays.)

Style/CSS: custom properties silently dropped unless the selector is literally `:root`; `serialize_literal_color`
hand-matches 17 triples and emits non-CSS names (`lightred`); calc serialization drops parens; `calc(10 / 0)`
is 0 with no warning; `flex: 1 1 0%` rejected; `background` shorthand and the `inherit` keyword unsupported;
tokenizer lacks string escapes, `url(`, exponents, non-ASCII idents, fractional percentages.

Render performance (layout + paint run every frame): `ComputedStyle` cloned per node in layout and paint
and per fragment in inline paint; whole `InlineLayout` / `anonymous_blocks` cloned per element per frame;
one `String` per grapheme in the packer, text packed ≥2× per frame; `ComputedStyle::initial()` built twice
per matched rule; `DirtyTracker` marks every sibling on every `ChildListChanged` with a linear `contains`
(O(n²) for n appends); SGR emission allocates per changed cell.

Coupling: paint knows `<select>` / `<progress>` / password chrome and keys on `dialog` / `treeitem`
attributes; `layout_atomic_inline_blocks` duplicated between block and flex; the in-flow filter re-inlined
in five places; `collect_z_list` triplicated; `layout_differs` omits position/inset/z-index so
`is_layout_dirty()` lies; `LAYOUT_MASK` / `INHERITS_MASK` exported but unused.

Process: `rdom-tui` dev-depends on its own consumer `rdom-showcase` and the published tarball ships
`examples/` and `tests/` that import it (cannot build from crates.io) — add `exclude` or relocate; CI ran
without `--locked` (fixed 2026-09-21); the toolchain "pin" is `channel = "stable"`; `STATE.md` is a 700-line
append-only log; `TECH_DEBT.md` keeps 36 resolved rows against its own retire rule.

## 2. DIVERGENCES.md audit

115 bullets audited. Counts: **KEEP 58 · FOLLOW-WEB 37 · DELETE 20 · REWRITE 14**, plus 16 divergences
present in code but missing from the file. Verdict key: KEEP = terminal-medium constraint or a product
decision that still holds; FOLLOW-WEB = a v1 shortcut with no medium reason, implement the web behavior;
DELETE = entry describes web-matching behavior or a feature that has since shipped; REWRITE = real
divergence, wrong text.

### Factually wrong today (fix the text in the batch that touches the code, or now if no code changes)

- "No margin collapsing" — §8.3.1 collapsing shipped with BFC-1. DELETE.
- "`calc()` on padding/margin/gap constant-only" — only `gap` remains (`CALC-GAP-1`). REWRITE.
- "Not implemented: `calc()`" and "`calc()` scheduled for 0.2.0" — shipped 0.2.0. DELETE.
- "IDs never reused within a Dom" — LIFO free list reissues slots. REWRITE now; becomes true again with R1.
- "Pointer capture does not retarget `event.target`" — the router dispatches *to* the captured node (web-faithful). REWRITE to keep only "mouse-only, no `pointerId`".
- "Tab focus-nav runs before keydown" — keydown fires first and Tab is a `preventDefault`-gated default action (web-faithful). DELETE.
- "`Event.detail` is `Option<String>`", "removal via AbortSignal only", "classList methods only", "no microtask queue", "`beforeinput.detail` is a plain String" — all superseded by shipped code. REWRITE / DELETE.
- Intro and §3 still say "ships in 0.1.0" / "scheduled for 0.2.0". REWRITE as "as of 0.3.x".

### Entries describing web-matching behavior (dilute the contract; DELETE, move any useful note to DESIGN.md)

Border conflict resolution §11.5; content occludes border; `user-select: all` / `contain`; readonly
`beforeinput`; Shift+Up/Down sticky-x; click at common ancestor; capture routing/release; focus events
synchronous; scroll containers focusable; scroll keys on nearest ancestor.

### FOLLOW-WEB, by consumer impact

1. `pointer-events: none` (overlays and backdrops need it).
2. Wheel scroll chaining to the nearest scrollable ancestor at the rail end.
3. Selectors: `:disabled` / `:enabled` / `:required` (attribute-backed, small), `:is()` (alias of `:where()` with specificity), `:nth-child()`.
4. `<style>` text re-parse on `CharacterData` mutation; scoped custom properties (any selector, not just `:root`); generic `var()` substitution beyond colors and `content`.
5. Tab skips non-rendered elements; checkbox/radio pre-activation flip with revert on cancel; `<form>` reset restores defaults; `change` on text inputs at blur; `<select size>` semantics.
6. Target-phase capture-before-bubble ordering; `stopPropagation` flags reset on re-dispatch; duplicate ids resolve in document order; `:indeterminate` for checkboxes and orphan radios.
7. Layout: flex cross-axis margins (currently ignored entirely) and negative main-axis margins; absolute elements with `auto` size get intrinsic size instead of 0×0; sticky `bottom` / `right` and sticky containing block; static position for absolutes; `text-align`; anonymous inline boxes; nested stacking contexts; opacity nesting multiplies.
8. Values: `min()` / `max()` / `clamp()`; `currentColor`; `cubic-bezier()` / `steps()`; transitions to calc endpoints resolve per tick; transition to `auto` snaps immediately (currently at midpoint); `aspect-ratio` decimals; `<pre>` tab stops.
9. Selection relocates boundary points on detach instead of collapsing to `None`; cross-node edits undoable as a compound entry; undo fires `beforeinput`.
10. Form validation (`required`, `pattern`, `:invalid`); `@keyframes` (roadmap).

### KEEP (medium or product) — representative

Integer cells, monospace, no images, cell length units, truecolor with indexed passthrough, box-drawing
and half-block border extensions, no RTL / letter-spacing, DEC 2026 sync, `border-collapse` on any
container and non-inheriting, `hidden` kill-switch, dashed/dotted render solid, rgba alpha dropped,
at-rules skipped until `@keyframes`, no floats / grid / transform / Shadow DOM / namespaces / Window /
Document / prototype chain, byte offsets, single selection range, `scroll` bubbles, `keyup` needs kitty,
Ctrl-C swallowed (terminal convention — must be *listed*), `:checked` reflects the attribute (single-source
design — must be *listed*), invalid selector strings return empty instead of throwing (must be *listed*),
strict template parser (except `<style>` raw text, which is FOLLOW-WEB), ARIA tree built-in, clipboard via
arboard + OSC 52, caret paint and `caret-text-color`, disabled → `user-select: none`, tick precision.

Open product questions (owner decides; default is the recommendation): `flex: <N>` basis → `0%` is the
CSS-specified expansion, so the entry stays but should say so; `%`-height only under a flexing ancestor is
a real simplification worth following (block definiteness for stretched items).

## 3. TECH_DEBT.md audit

80 rows audited. Counts: **DELETE 36 · ACCEPT 20 · FIX-LATER 13 · FIX-NOW 11**, plus 18 items of debt
found in code with no entry (8 FIX-NOW). Marked-resolved entries that are wrong: `BFC1-CODE-COLLAPSE-INSETS-1`
(helpers never moved out of `flex.rs`), `CALC-PADMARG-1` (padding percent basis is the element's own outer
width, not the containing block), `M5-MIN-AUTO-1` (resolved but never marked), `ARENA-RECLAIM-1` (resolution
introduced slot reuse with no generation), the Process section ("no current process debt" is false).

FIX-NOW: flex margins (cross-axis ignored, main negatives clamped) + the intrinsic pair
`FLEX-BLOCK-MAIN-INTRINSIC-1` / `FLEX-ITEM-MARGIN-MAIN-INTRINSIC-1`; `UA-CHECKBOX-INLINE-1`;
`CSS-BG-SHORTHAND-1`; `CSS-INHERIT-KEYWORD-1`; `EDIT-1` (cross-node edits skip the undo history);
`rdom-tui` tarball `exclude`; plus R1–R11 above.

FIX-LATER (refactors, low risk): `BFC1-CODE-BLOCK-SPLIT-1` (block.rs now 1211 lines),
`BFC1-CODE-ATOMIC-IB-DUP-1`, `BFC1-CODE-COLLAPSE-INSETS-1` (reopen), `BFC1-MARGIN-PERCENT-CHAIN-1`,
`BFC1-PERF-*`, `DRY-1` / `DRY-2` (three copies of `collect_z_list`), `M5-STICKY-1`, `D-M3-2/3/6`,
`SCROLL-CROSS-AXIS-1` (absorb horizontal autoscroll), `SHOWCASE-EVT-1`, `EDIT-2`, `TREE-BFC-PSEUDO-1` prefix case.

ACCEPT (stay, with honest text): `D-M2-2/3/4`, `M5-MIN-CONTENT-2`, `CALC-GAP-1`, `SCROLLBAR-AUTO-TWO-PASS-1`,
`FOCUS-THUMB-NEAREST-1`, `BFC1-AUTO-HEIGHT-ORDERING-1`, `D-M3-5`, `OPACITY-1`, `UA-OL-1`, `UA-SB-1`, `SUB-3/4`,
`TABLE-COLSPAN-1`, `TABLE-TFC-1`, `ITERM2-MOUSE-MOTION-1`, `D-M1-1/2`.

## 4. Batches (grouped so version bumps batch)

Every behavior change lands test-first. Each batch ends with the grumpy-architect + grumpy-API passes,
then a release (divergent bumps as before; a `rdom-core` change forces a `rdom-tui` bump).

- **Batch 1 — rdom-core** (→ `rdom-core` 0.4.0, `rdom-tui` follows): R2 ✔, R3 ✔, R1 generational `NodeId`,
  target-phase ordering + flag reset ✔, observer re-entrancy contract ✔, index structure ✔, duplicate-id document
  order ✔, `set_class_name` single pass ✔, `# Panics` docs ✔, accessor split. Rewrite the
  DOM-API section of DIVERGENCES.md.
- **Batch 2 — rdom-style + rdom-css** (→ 0.4.0, `rdom-tui` follows): R8 number tokens, R9 at-rules and
  brace recovery, scoped custom properties, `background` shorthand, `inherit` / `initial` / `unset`,
  `flex: 1 1 0%`, color reverse lookup, calc paren serialization and `/ 0` warning, tokenizer gaps.
- **Batch 3 — rdom-tui** (→ 0.4.0): R4 scheduler handle, R11 raw-mode guard, R5 scroll extent, R6 flex
  freeze loop, R7 sticky anon blocks, flex margins, Tab visibility, checkbox pre-flip, dialog / form / select
  fidelity, `pointer-events`, wheel chaining, `Cargo.toml` `exclude`, `EDIT-1`, type-ahead and dblclick on the
  scheduler clock, `layout_differs` + masks, per-element custom-property scope in the cascade (lifts the
  `:root`-only divergence), then the performance items and the file splits.
- **Batch 4 — rdom-parser** (→ 0.4.0): R10 text `<`, RAWTEXT / RCDATA, entity table, an "HTML parsing"
  section in DIVERGENCES.md.
- **Housekeeping (each batch, doc-only commits allowed):** delete resolved TECH_DEBT rows, fix the four wrong
  resolutions, replace the Process section; delete the ten web-matching DIVERGENCES entries; correct the
  crate-count / version-policy text in `CLAUDE.md`, `DESIGN.md`, `publish.md`; cut `STATE.md` to a ledger.

## 5. Progress log

- 2026-09-21 — Review + audits complete. Lockfile synced, CI `--locked`, `rdom-tui-v0.3.14` tagged.
  Batch 1: R2 fixed (spec bits, `Dom::compare_boundary_points`, `selection_range` and paint selection
  membership use boundary-point order); R3 fixed (observer + listener panic restore). Runtime test that
  enshrined the old "panicking listener vanishes" behavior rewritten to assert retention. R1 fixed
  (generational `NodeId`). Two-pass target dispatch + flag reset; observers may add/remove observers
  during a callback; index buckets are `BTreeSet`s; `getElementById` duplicates resolve in document
  order; `set_class_name` single pass. Process note: the first two Batch-1 commits went in with one
  red runtime test (the rewritten panic test asserted a root listener count of 1, ignoring the App's
  builtin root listeners) because the background test runner's exit status was read from the wrong
  place — fixed in the following commit; the gate is now read from the log's own `test exit` line.
- 2026-09-24 — Batch 1 review gates run (architect + API). No blocking findings. Follow-ups landed: stale
  rustdoc (indexes / observer / dispatch / event), CHANGELOG "Breaking" section with migration notes,
  DIVERGENCES DOM-API/Events entries rewritten (classList, listener removal, dispatch order) plus new
  entries for arena-wide `getElementById` and the `compare_boundary_points` shape, DESIGN.md id-reuse and
  observer-delivery paragraphs corrected, `dispatch_event` rejects an in-flight event
  (`DomError::InvalidState`), `drop_subtree(root)` rejected, `validate()` checks the generation table,
  timing assertion removed from the index test, missing tests added (stale id → dispatch/add_listener,
  observer removing a later observer, boundary offset past child count). Deferred to TECH_DEBT:
  `CORE-DOCPOS-ALLOC-1`, `CORE-GEN-COLOCATE-1`, `CORE-DROP-PANIC-LEAK-1`. Release note: every crate
  pinning `rdom-core` must bump with it (not just `rdom-tui`).
- 2026-09-24 — Batch 2 (rdom-style + rdom-css) code complete, pending review gates: R8 fixed
  (`Token::Float`, `Percentage(f64)`, whole-literal number tokenization per CSS Syntax 3 §4.3.12;
  checked `u16` conversions instead of wrapping casts); R9 fixed (at-rules consumed whole with
  `UnsupportedAtRule`, stray `}` ignored, EOF closes a block, depth-aware bodies, strings in preludes,
  `MalformedDeclaration` warning); CSS-wide keywords `inherit` / `initial` / `unset` for every
  property (`property_dispatch::inherits` decides `unset`); `background` shorthand; `flex: 1 1 0%`;
  named-color reverse lookup; calc serialization parentheses; `calc(x / 0)` rejected; string hex
  escapes + non-ASCII identifiers; custom properties outside `:root` warn
  (`UnsupportedCustomPropertyScope`) and DIVERGENCES documents the `:root`-only scope. Deferred to
  TECH_DEBT: `STYLE-TRANSITION-VALUE-1`, `STYLE-INHERITS-TWO-SOURCES-1`. Not done in this batch:
  `url(` token, `rgb(255 0 0)` / `hsl()` syntaxes, fractional-percentage layout resolution beyond
  whole percent (parsed, truncated), per-element custom-property scope (Batch 3 cascade work).
- 2026-09-24 — Batch 2 review gates run (architect + API). Blocking items fixed: the cascade resolved
  `opacity` / `text-decoration` / `scrollbar-gutter: inherit` to the initial value (arms were dead
  before the parser produced the keywords); `css_wide_of` claimed `inherit` for a shorthand when only
  its first field was inherited. Also fixed: `aspect-ratio` wrapping cast, `1e400` → finite clamp,
  negative opacity clamps (CSS Color 4), inline `--x` warns like any non-root scope, stale comments.
  Docs: `Breaking — rdom-style` / `rdom-css` sections with migration notes; DIVERGENCES gains
  color-only `background`, whole-percent / whole-ms resolution, shared longhand storage, integer flex
  factors, `flex: inherit` semantics, transitions rejecting CSS-wide keywords, corrected at-rule
  wording, and drops the stale "calc() not implemented" line; `unset` note moved to DESIGN.md; a test
  pins `property_dispatch::inherits` to the cascade's `INHERITS_MASK`; READMEs swept (M5 / 0.2.0
  claims, at-rules, custom-property scope, timing functions). New TECH_DEBT: `STYLE-PERCENT-FRACTION-1`,
  `CSS-WARNING-POSITION-1`, `STYLE-PROPERTY-TABLES-1`. Batch 3 (rdom-tui) has begun with R4 landed.
