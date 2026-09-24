# STABILIZE-2026-09 — pay down every open debt item before 0.5.0

**Status:** IN PROGRESS (started 2026-09-23, the day after 0.4.0 shipped).

**Goal.** The next release, **0.5.0**, is the "stable and complete" line: it ships with an empty
`## Open` section in [`TECH_DEBT.md`](TECH_DEBT.md). Every item there is handled one of three ways,
and nothing is left as "later":

- **fix** — implement the web behavior (or the real refactor), TDD, in the crate that owns it;
- **divergence** — the behavior is a deliberate, permanent departure: it moves to
  [`DIVERGENCES.md`](DIVERGENCES.md) with its reason, and stops being called debt;
- **external** — the cause is outside rdom (a terminal emulator bug); it is documented under
  "Known limitations" in `DIVERGENCES.md` and the workaround is user-facing.

Routing (0.6.0) and async tasks (0.7.0) do not start before this program closes.

**Rules.** Same as [`HARDENING-2026-09.md`](HARDENING-2026-09.md): one item per commit, failing test
first, per-crate tests then the workspace gate, docs move with the code (`DIVERGENCES.md`,
`TECH_DEBT.md` row deleted in the same commit), grumpy architect + API gates at the end of each
phase, findings fixed or filed before the next phase starts. All five crates bump to 0.5.0 (the core
changes force it), so phases are ordered by dependency, not by publishability.

## 1. Triage (verified against source 2026-09-23)

Already resolved in 0.4.0 and deleted from the ledger in the program's first commit:

| Id | Why it is stale |
|---|---|
| `CSS-INHERIT-KEYWORD-1` | CSS-wide keywords ship since Batch 2 (`property_dispatch::css_wide_keyword`, `set_css_wide`). |
| `CSS-BG-SHORTHAND-1` | `background` is registered in the property table and dispatches to `bg` (color-only subset, documented in DIVERGENCES). |
| `BORDER-MODEL-1` | Landed in 0.2.0; `BorderStyle` enum, per-direction conflict resolution, no `border-model` branch exists. Its "closes" list was already deleted. |

Everything else is real. Disposition per item:

| Id | Disposition | Phase |
|---|---|---|
| `PROC-TOOLCHAIN-PIN-1` | fix — pin `channel = "1.95.0"` locally and in CI; bump deliberately | 1 |
| `PROC-TUI-DEV-DEP-1` | fix — self-contained examples in `rdom-tui`; showcase shims + snapshot tests move to `rdom-showcase` | 1 |
| `CORE-DOCPOS-ALLOC-1` | fix — allocation-free depth-walk comparison | 2 |
| `CORE-GEN-COLOCATE-1` | fix — `(generation, Option<Node>)` in one slot | 2 |
| `CORE-DROP-PANIC-LEAK-1` | fix — free under a drop guard | 2 |
| `PARSER-VOID-TAGS-1` | fix — `rdom_core::markup::VOID_TAGS` becomes the exported single source; `vr` decided | 2 |
| `PARSER-ENTITIES-1` | fix — full WHATWG named-reference table (generated, sorted static) incl. the legacy no-semicolon set | 2 |
| `STYLE-TRANSITION-VALUE-1` | fix — `Value<Vec<…>>` for the four transition longhands | 3 |
| `STYLE-PERCENT-FRACTION-1` | fix — `Size::Percent` carries the fraction; rounding happens once, at resolution | 3 |
| `CSS-WARNING-POSITION-1` | fix — token spans; declaration warnings point at the declaration | 3 |
| `STYLE-PROPERTY-TABLES-1` | fix — one `fields_of(name)` table, four derived functions, coverage test | 3 |
| `STYLE-INHERITS-TWO-SOURCES-1` | fix — `INHERITS_MASK` derived from `property_dispatch::inherits` | 3 |
| `D-M3-2` | fix — `cubic-bezier()` and `steps()` parsed | 3 |
| `CALC-GAP-1` | fix — `GapValue::{Cells, Calc}` resolved at layout | 3 |
| `SUB-4` | fix — `Length::Cells(i32)` | 3 |
| `SUB-3` | fix — UA `style { display: none }` in its section | 3 |
| `UA-CHECKBOX-INLINE-1` | fix — `inline-block`, matching `<button>` | 3 |
| `UA-SB-1` | fix — `::scrollbar-thumb:vertical` / `:horizontal` | 3 |
| `UA-OL-1` | fix — CSS counters (`counter-reset` / `counter-increment` / `counter()`), `list-item` auto counter, `ol > li::before { content: counter(list-item) ". " }` | 3 |
| `CSS-VARS-SCOPE-1` | fix — per-element custom properties across style / css / cascade | 3 + 4 |
| `CASCADE-INITIAL-ALLOC-1` | fix — hoisted initial constants + rightmost-selector rule index | 4 |
| `D-M3-3` | fix — pseudo-element transitions | 4 |
| `D-M3-5` | fix — one microtask drain per tick | 4 |
| `D-M3-6` | fix — discrete properties toggle at midpoint under `transition: all` | 4 |
| `D-M2-2` | fix — hypothetical static position for `auto` insets | 5 |
| `D-M2-3` / `D-M2-4` | fix — nested stacking contexts per CSS 2.1 Appendix E (negative z-index below in-flow content) | 5 |
| `DRY-1` / `DRY-2` | fix — one in-flow filter, one positioned-collection helper for paint and hit-test | 5 |
| `D-M5N-8` | fix — prefix shift on left-edge clip | 5 |
| `M5-MIN-CONTENT-2` | fix — strict longest-unbreakable-run min-content | 5 |
| `M5-STICKY-1` | fix — `right` / `bottom` sticky, nested scroller containing block | 5 |
| `CALC-PADMARG-1` / `BFC1-MARGIN-PERCENT-CHAIN-1` | fix — containing-block width threaded through padding / margin resolution and the collapse walkers | 5 |
| `BFC1-PERF-MARGIN-CHAIN-1` | fix — single pre-order margin pass or per-pass memo | 5 |
| `BFC1-PERF-INLINE-FLOW-LOOKUP-1` | fix — `text_node → flow` index | 5 |
| `BFC1-CODE-BLOCK-SPLIT-1` | fix — `block/{mod,margin_collapse,width,height}.rs` | 5 |
| `SCROLLBAR-AUTO-TWO-PASS-1` | fix — pass 2 re-resolves auto height and the scroll carry-back | 5 |
| `BFC1-AUTO-HEIGHT-ORDERING-1` | fix — measure-then-place for `auto`-height parents with content-dependent descendants | 5 |
| `FLEX-BLOCK-MAIN-INTRINSIC-1` | fix — block items resolve flex base size via `max-content` | 5 |
| `FLEX-ITEM-MARGIN-MAIN-INTRINSIC-1` | fix — main-axis margins count toward the container's hypothetical main size | 5 |
| `FLEX-ITEM-NEGATIVE-MARGIN-1` | fix — `i32` placement, negative margins pull outward | 5 |
| `SCROLL-CROSS-AXIS-1` | fix — cross-axis scroll offsets and overflow cross size; horizontal autoscroll band | 5 |
| `TABLE-COLSPAN-1` | fix — `colspan` / `rowspan` width and height spreading in the column-sync pass | 5 |
| `TABLE-TFC-1` | **divergence** — tables are element-driven (`<table>` family), not `display: table` on arbitrary elements; anonymous table fixup is not implemented. Reason: no consumer needs it and it would import a second layout algorithm. Documented under Layout. | 5 |
| `FOCUS-THUMB-NEAREST-1` | fix — post-layout nearest-scroll-container marker | 5 |
| `OPACITY-1` | fix — subtree group rendering into an off-screen buffer | 6 |
| `TREE-BFC-PSEUDO-1` | fix — pseudo emitted into the first / last anonymous box | 6 |
| `PACKER-STRING-ALLOC-1` | fix — byte ranges into the source text; per-frame `(node, width)` cache | 6 |
| `PAINT-INLINE-LAYOUT-CLONE-1` | fix — borrow / `Rc` the inline layout | 6 |
| `SGR-ALLOC-1` | fix — allocation-free SGR emission | 6 |
| `CARET-REVEAL-STALE-LAYOUT-1` | fix — pending-reveal flag serviced after `layout_dom` | 6 |
| `POINTER-EVENTS-IFC-1` | fix — resolve `pointer-events` through the fragment owner chain | 6 |
| `FLEX-RS-SPLIT-1`, `SCROLLBAR-SPLIT-1`, `HIT-TEST-SPLIT-1`, `SELECT-SPLIT-1`, `APP-MOD-SPLIT-1`, `INLINE-PAINT-SPLIT-1` (+ `cssom/declaration.rs`, `render/buffer.rs`, `virtual_screen` behind `cfg(test)` / a `test-util` feature) | fix — split by concern, no behavior change; chrome substitution behind a runtime hook | 6 |
| `SHOWCASE-EVT-1` | fix — `AppHandle` gains stylesheet intents drained after the current event | 6 |
| `FORM-DEFAULTS-1` | fix — `defaultValue` / `defaultChecked` separate from the dirty value; reset restores | 6 |
| `EDIT-2` | fix — `user-select: contain` clamps to the nearest in-host fragment | 6 |
| `ITERM2-MOUSE-MOTION-1` | **external** — moves to DIVERGENCES "Known limitations" with the click-to-wake workaround; rdom-tui README notes it | 6 |

## 2. Phases

| Phase | Crates | Items | Status |
|---|---|---|---|
| 0 | docs | this plan; stale rows deleted | done 2026-09-23 |
| 1 | process | toolchain pin, dev-dep inversion | done 2026-09-23 |
| 2 | rdom-core, rdom-parser | 3 core + 2 parser | done 2026-09-23 |
| 3 | rdom-style, rdom-css | 12 style / css / UA items incl. counters and custom-property storage | |
| 4 | rdom-tui cascade + animation | custom-property cascade, inherits mask, initial hoist + rule index, three animation items | |
| 5 | rdom-tui layout | 22 layout items incl. stacking contexts, static position, cross-axis scroll, spans | |
| 6 | rdom-tui paint + runtime + forms | 18 items incl. group opacity, splits, form defaults, app intents | |
| 7 | completeness | scope confirmation with Miska: form validation, `:focus-visible`, `::placeholder` / `:placeholder-shown`, undo coalescing, blinking caret, clipboard whitespace, `scroll-behavior`, live `<style>` — the README's "open polish" list and DIVERGENCES §3 "Not yet shipped" | |
| 8 | release | 0.5.0 across all five crates, migration notes | |

Each phase ends with the two review gates; each commit carries the item id.

## 3. Log

- 2026-09-23 — Phase 2: `drop_subtree` frees under an observer panic (`catch_unwind` → free →
  `resume_unwind`); document position / boundary points / common ancestor are a depth walk with an
  oracle test over every pair of a 40-node tree; the slot generation is colocated with the node
  (`Slot { generation, node }`, `InvariantViolation::GenerationTableMismatch` removed); HTML §13.3's
  void list is exported from `rdom-core` and imported by the parser (`vr` dropped); the parser
  decodes the full WHATWG entity table (generated `entities.rs` + `tools/gen_entities.py`, legacy
  no-`;` names, attribute caveat). Phase 1 and 2 are gated together.
- 2026-09-23 — Phase 1+2 API gate: two blockers on the entity work fixed before commit — the C1
  0x80–0x9F → Windows-1252 remap (`&#146;` is `’`) and digit-only numeric scanning (`&#65abc;` is
  `Aabc;`); DIVERGENCES "in full" claim now earned. Also: `rust-version` 1.85 → 1.88 (let-chains),
  stale `rdom-tui/Cargo.toml` tarball comment removed, examples use HTML elements and no `let _ =`
  swallows, publish checklist notes the `rdom-tui` → `rdom-parser` dev-dep pin. Slip: the entity
  generator script landed in the Phase 1 commit instead of Phase 2's.
- 2026-09-23 — Phase 1+2 architect gate: blocker — the drop-path guard covered only the
  `ChildListChanged` record; the focus / selection purge inside `detach_from_parent` fires before it.
  Fixed by snapshotting the subtree first and running detach + fire under one guard, freeing only if
  the root ended up detached (a `PreDetach` panic leaves an attached, intact subtree); the two
  `*_dropping` wrappers got the same guard. Four tests. Also: legacy prefix loop bounded by
  `LONGEST_LEGACY_NAME` (6 bytes, generated), `RefContext` enum instead of bool flags, numeric /
  named scanner edge tests, `#[rustfmt::skip]` emitted by the generator, CI channel read hardened
  against CRLF (`tr -d '\r'`, `*.toml eol=lf`), stale CI comment fixed. Decisions recorded: the full
  entity table ships unconditionally (66 KB of source, no feature flag — correctness over size); the
  `render` / `buffer_to_snapshot` test helpers are duplicated between the `rdom-tui` and
  `rdom-showcase` suites on purpose (two test targets, no shared test crate).
- 2026-09-23 — Phase 3 start: `STYLE-MASK-COLLISION-1` (FLOW → bit 41, uniqueness test);
  `STYLE-INHERITS-TWO-SOURCES-1` closed by deleting the dead `PropMask` / `INHERITS_MASK` /
  `LAYOUT_MASK` exports and pinning `inherit_inheritable_from` to `property_dispatch::inherits`
  with a test that probes all 41 properties (every inherited name must be probed).
- 2026-09-23 — `STYLE-TRANSITION-VALUE-1`: the four transition longhands carry `Value`; the
  css-wide coverage test no longer skips them; `apply_transition_lists` takes the parent.
- 2026-09-24 — `STYLE-PERCENT-FRACTION-1`: `Size::Percent(f32)` + `Size::percent_of` as the one
  rounding site (six layout call sites); whole-millisecond times stay as a narrowed divergence.
- 2026-09-24 — `STYLE-PROPERTY-TABLES-1`: `define_fields!` + `fields_of(name)`; four functions
  collapse to folds; found and fixed the stale-`flow`-after-`removeProperty("display")` bug on the
  way.
- 2026-09-24 — `CSS-WARNING-POSITION-1` (`tokenize_at` with per-token positions; declaration warnings
  and in-block tokenizer errors are absolute) and `D-M3-2` (`cubic-bezier()` / `steps()` / step
  keywords parsed, evaluated per CSS Easing 1, serialized).
- 2026-09-24 — `SUB-4` (`Length::Cells(i32)`), `CALC-GAP-1` (`GapValue`, `resolve_gap` in layout with
  the indefinite-axis rule, percent-gap layout test), `UA-CHECKBOX-INLINE-1` + `SUB-3` (inline-block
  toggles; UA section list matches the code).
- 2026-09-24 — `CSS-VARS-SCOPE-1`: `TuiStyle::custom_properties`, any-selector parsing (warning
  variant removed), cascade pre-pass folding declarations into a copy-on-write inherited map (this
  also fixes the inheritance overwrite found during mapping), CSSOM `--x` round-trip.
- Found while mapping Phase 3 (not on the ledger): `ImportantMask::FLOW` and `POINTER_EVENTS` share
  bit 39 (`tui_style.rs`), custom-property inheritance in the cascade is overwritten by the merged
  root map (`walk.rs`), and tokenizer errors inside a block are body-relative (`declarations.rs`).
  All three are Phase 3 items.

- 2026-09-23 — Phase 1: `rust-toolchain.toml` pins `1.95.0` and CI reads the channel from it
  (`PROC-TOOLCHAIN-PIN-1`). The ten showcase-backed example shims, the twelve demo snapshot tests and
  the sixteen goldens moved into `rdom-showcase/{examples,tests}/`; `rdom-tui/examples/` has three
  self-contained programs, the `rdom-showcase` dev-dep and the tarball `exclude` are gone
  (`PROC-TUI-DEV-DEP-1`).

- 2026-09-23 — Program opened. Triage: 3 stale rows deleted (`CSS-INHERIT-KEYWORD-1`,
  `CSS-BG-SHORTHAND-1`, `BORDER-MODEL-1`), 52 remain: 50 fix, 1 divergence (`TABLE-TFC-1`),
  1 external (`ITERM2-MOUSE-MOTION-1`).
